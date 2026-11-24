//! A small reader for the resources of a 32-bit Windows PE file (the
//! game's DLLs and REBEXE.EXE), dependency-free so it runs in the browser
//! build as well (`docs/plans/2026-11-24-bring-your-own-data.md`).
//!
//! It walks the three-level resource tree (type, name, language) of the
//! `.rsrc` data directory and hands out each leaf's bytes, borrowed from
//! the file. port: the format is Microsoft's PE/COFF specification; the
//! walk follows `tools/stage-ui-assets/pe_resources.go`.

use std::fmt;

const SUBDIRECTORY: u32 = 0x8000_0000;
/// The resource table's entry in the optional header's data directories.
const RESOURCE_DIRECTORY: usize = 2;

/// A resource type or name: a number, or a string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResId {
    Id(u32),
    Name(String),
}

impl ResId {
    /// The number, when it is one.
    #[must_use]
    pub fn id(&self) -> Option<u32> {
        match self {
            Self::Id(id) => Some(*id),
            Self::Name(_) => None,
        }
    }
}

/// One resource: its type, name and language, and its bytes in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource<'a> {
    pub kind: ResId,
    pub name: ResId,
    pub language: u32,
    pub data: &'a [u8],
}

/// Why a file's resources could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeError {
    NotPe,
    Truncated(&'static str),
    NoResources,
    BadDirectory(u32),
    BadName(u32),
    BadData { rva: u32, size: u32 },
}

impl fmt::Display for PeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPe => write!(f, "not a PE file"),
            Self::Truncated(what) => write!(f, "truncated {what}"),
            Self::NoResources => write!(f, "no resource table"),
            Self::BadDirectory(offset) => write!(f, "resource directory at {offset:#x} is out of bounds"),
            Self::BadName(offset) => write!(f, "resource name at {offset:#x} is out of bounds"),
            Self::BadData { rva, size } => write!(f, "resource data {rva:#x}+{size} is outside the file's sections"),
        }
    }
}

impl std::error::Error for PeError {}

fn u16_at(bytes: &[u8], at: usize, what: &'static str) -> Result<u16, PeError> {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or(PeError::Truncated(what))
}

fn u32_at(bytes: &[u8], at: usize, what: &'static str) -> Result<u32, PeError> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(PeError::Truncated(what))
}

/// A section: its virtual address and size, and where its raw data lies.
struct Section {
    virtual_address: u32,
    virtual_size: u32,
    raw_offset: u32,
    raw_size: u32,
}

/// A parsed PE file: its sections and its resource table's RVA.
struct Image<'a> {
    file: &'a [u8],
    sections: Vec<Section>,
    resource_rva: u32,
    resource_size: u32,
}

impl<'a> Image<'a> {
    fn parse(file: &'a [u8]) -> Result<Self, PeError> {
        if file.get(0..2) != Some(b"MZ") {
            return Err(PeError::NotPe);
        }
        let pe = u32_at(file, 0x3c, "DOS header")? as usize;
        if file.get(pe..pe + 4) != Some(b"PE\0\0") {
            return Err(PeError::NotPe);
        }
        let coff = pe + 4;
        let sections = u16_at(file, coff + 2, "COFF header")? as usize;
        let optional_size = u16_at(file, coff + 16, "COFF header")? as usize;
        let optional = coff + 20;
        let directories = match u16_at(file, optional, "optional header")? {
            0x10b => optional + 96,
            0x20b => optional + 112,
            _ => return Err(PeError::NotPe),
        };
        let entry = directories + RESOURCE_DIRECTORY * 8;
        let resource_rva = u32_at(file, entry, "data directories")?;
        let resource_size = u32_at(file, entry + 4, "data directories")?;
        let table = optional + optional_size;
        let sections = (0..sections)
            .map(|i| {
                let at = table + i * 40;
                Ok(Section {
                    virtual_size: u32_at(file, at + 8, "section table")?,
                    virtual_address: u32_at(file, at + 12, "section table")?,
                    raw_size: u32_at(file, at + 16, "section table")?,
                    raw_offset: u32_at(file, at + 20, "section table")?,
                })
            })
            .collect::<Result<Vec<_>, PeError>>()?;
        if resource_rva == 0 || resource_size == 0 {
            return Err(PeError::NoResources);
        }
        Ok(Self {
            file,
            sections,
            resource_rva,
            resource_size,
        })
    }

    /// The file bytes at `rva`..`rva + size`, from one section's raw data.
    fn range(&self, rva: u32, size: u32) -> Result<&'a [u8], PeError> {
        let bad = PeError::BadData { rva, size };
        let section = self
            .sections
            .iter()
            .find(|s| {
                let span = s.virtual_size.max(s.raw_size);
                rva >= s.virtual_address && u64::from(rva) < u64::from(s.virtual_address) + u64::from(span)
            })
            .ok_or_else(|| bad.clone())?;
        let within = u64::from(rva - section.virtual_address);
        if within + u64::from(size) > u64::from(section.raw_size) {
            return Err(bad);
        }
        let start = u64::from(section.raw_offset) + within;
        let end = start + u64::from(size);
        self.file
            .get(usize::try_from(start).map_err(|_| bad.clone())?..usize::try_from(end).map_err(|_| bad.clone())?)
            .ok_or(bad)
    }
}

/// The entries of the resource directory at `offset` in `table`:
/// `(name, target)` pairs.
fn directory(table: &[u8], offset: u32) -> Result<Vec<(u32, u32)>, PeError> {
    let at = offset as usize;
    let bad = PeError::BadDirectory(offset);
    let named = u16_at(table, at + 12, "resource directory").map_err(|_| bad.clone())?;
    let ids = u16_at(table, at + 14, "resource directory").map_err(|_| bad.clone())?;
    (0..usize::from(named) + usize::from(ids))
        .map(|i| {
            let entry = at + 16 + i * 8;
            Ok((
                u32_at(table, entry, "resource directory").map_err(|_| bad.clone())?,
                u32_at(table, entry + 4, "resource directory").map_err(|_| bad.clone())?,
            ))
        })
        .collect()
}

/// A directory entry's name: a number, or a UTF-16 string in the table.
fn res_id(table: &[u8], raw: u32) -> Result<ResId, PeError> {
    if raw & SUBDIRECTORY == 0 {
        return Ok(ResId::Id(raw));
    }
    let at = (raw & !SUBDIRECTORY) as usize;
    let len = usize::from(u16_at(table, at, "resource name").map_err(|_| PeError::BadName(raw))?);
    let units = table
        .get(at + 2..at + 2 + len * 2)
        .ok_or(PeError::BadName(raw))?
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect::<Vec<_>>();
    Ok(ResId::Name(String::from_utf16_lossy(&units)))
}

/// Every resource of a PE file, in the tree's own order (types, then
/// names, then languages, each as the file lists them).
///
/// # Errors
/// A file that is not a PE image, has no resource table, or whose tree or
/// data point outside the file.
pub fn resources(file: &[u8]) -> Result<Vec<Resource<'_>>, PeError> {
    let image = Image::parse(file)?;
    let table = image.range(image.resource_rva, image.resource_size)?;
    let mut out = Vec::new();
    for (kind_raw, kind_target) in directory(table, 0)? {
        if kind_target & SUBDIRECTORY == 0 {
            continue;
        }
        let kind = res_id(table, kind_raw)?;
        for (name_raw, name_target) in directory(table, kind_target & !SUBDIRECTORY)? {
            if name_target & SUBDIRECTORY == 0 {
                continue;
            }
            let name = res_id(table, name_raw)?;
            for (language, data_entry) in directory(table, name_target & !SUBDIRECTORY)? {
                if language & SUBDIRECTORY != 0 || data_entry & SUBDIRECTORY != 0 {
                    continue;
                }
                let at = data_entry as usize;
                let rva = u32_at(table, at, "resource data entry")?;
                let size = u32_at(table, at + 4, "resource data entry")?;
                out.push(Resource {
                    kind: kind.clone(),
                    name: name.clone(),
                    language,
                    data: image.range(rva, size)?,
                });
            }
        }
    }
    Ok(out)
}

/// A synthetic PE file for the tests of this module and its users.
#[cfg(test)]
pub(crate) mod fixture {
    const VA: u32 = 0x1000;
    /// Where the resource table starts in a PE32 fixture (DOS header, PE
    /// signature, COFF header, optional header, one section header).
    pub(crate) const TABLE: usize = 0x40 + 4 + 20 + 224 + 40;

    fn dir(named: u16, ids: u16) -> Vec<u8> {
        let mut d = vec![0u8; 16];
        d[12..14].copy_from_slice(&named.to_le_bytes());
        d[14..16].copy_from_slice(&ids.to_le_bytes());
        d
    }

    /// A minimal PE image with one `.rsrc` section: `entries` are (type,
    /// name, language, data), a type `None` being the string "WAVE", in
    /// file order. The table: the root (16 + 8n bytes), then a 24-byte name
    /// directory per entry, a 24-byte language directory per entry, a
    /// 16-byte data entry per entry, the "WAVE" string and the data. The
    /// optional header is PE32 (0x10b) or PE32+ (0x20b).
    pub(crate) fn tiny_pe(entries: &[(Option<u32>, u32, u32, &[u8])], plus: bool) -> Vec<u8> {
        let n = entries.len();
        let names_at = 16 + 8 * n;
        let languages_at = names_at + 24 * n;
        let data_entries_at = languages_at + 24 * n;
        let string_at = data_entries_at + 16 * n;
        let mut wave = 4u16.to_le_bytes().to_vec();
        for unit in "WAVE".encode_utf16() {
            wave.extend_from_slice(&unit.to_le_bytes());
        }
        let mut data_at = string_at + wave.len();
        let named = entries.iter().filter(|e| e.0.is_none()).count() as u16;
        let mut table = dir(named, n as u16 - named);
        for (i, (kind, ..)) in entries.iter().enumerate() {
            let id = kind.unwrap_or(0x8000_0000 | string_at as u32);
            table.extend_from_slice(&id.to_le_bytes());
            table.extend_from_slice(&(0x8000_0000 | (names_at + 24 * i) as u32).to_le_bytes());
        }
        for (i, (_, name, ..)) in entries.iter().enumerate() {
            table.extend(dir(0, 1));
            table.extend_from_slice(&name.to_le_bytes());
            table.extend_from_slice(&(0x8000_0000 | (languages_at + 24 * i) as u32).to_le_bytes());
        }
        for (i, (_, _, language, _)) in entries.iter().enumerate() {
            table.extend(dir(0, 1));
            table.extend_from_slice(&language.to_le_bytes());
            table.extend_from_slice(&((data_entries_at + 16 * i) as u32).to_le_bytes());
        }
        for (.., data) in entries {
            table.extend_from_slice(&(VA + data_at as u32).to_le_bytes());
            table.extend_from_slice(&(data.len() as u32).to_le_bytes());
            table.extend_from_slice(&[0; 8]);
            data_at += data.len();
        }
        table.extend_from_slice(&wave);
        for (.., data) in entries {
            table.extend_from_slice(data);
        }
        let optional_size: u16 = if plus { 240 } else { 224 };
        let mut file = vec![0u8; 0x40];
        file[0..2].copy_from_slice(b"MZ");
        file[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        file.extend_from_slice(b"PE\0\0");
        let mut coff = vec![0u8; 20];
        coff[2..4].copy_from_slice(&1u16.to_le_bytes());
        coff[16..18].copy_from_slice(&optional_size.to_le_bytes());
        file.extend(coff);
        let mut optional = vec![0u8; usize::from(optional_size)];
        optional[0..2].copy_from_slice(&(if plus { 0x20bu16 } else { 0x10b }).to_le_bytes());
        let directories = if plus { 112 } else { 96 };
        optional[directories + 16..directories + 20].copy_from_slice(&VA.to_le_bytes());
        optional[directories + 20..directories + 24].copy_from_slice(&(table.len() as u32).to_le_bytes());
        file.extend(optional);
        let raw_offset = (file.len() + 40) as u32;
        let mut section = vec![0u8; 40];
        section[0..5].copy_from_slice(b".rsrc");
        section[8..12].copy_from_slice(&(table.len() as u32).to_le_bytes());
        section[12..16].copy_from_slice(&VA.to_le_bytes());
        section[16..20].copy_from_slice(&(table.len() as u32).to_le_bytes());
        section[20..24].copy_from_slice(&raw_offset.to_le_bytes());
        file.extend(section);
        file.extend(table);
        file
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::{tiny_pe, TABLE};
    use super::*;

    const BITMAP: Option<u32> = Some(2);
    const WAVE: Option<u32> = None;

    fn listed(file: &[u8]) -> Vec<(ResId, ResId, u32, Vec<u8>)> {
        resources(file)
            .unwrap()
            .into_iter()
            .map(|r| (r.kind, r.name, r.language, r.data.to_vec()))
            .collect()
    }

    #[test]
    fn a_small_file_lists_its_numbered_and_named_resources() {
        let file = tiny_pe(&[(WAVE, 9, 0, b"RIFF"), (BITMAP, 7, 1033, b"bits")], false);
        assert_eq!(
            listed(&file),
            vec![
                (ResId::Name("WAVE".into()), ResId::Id(9), 0, b"RIFF".to_vec()),
                (ResId::Id(2), ResId::Id(7), 1033, b"bits".to_vec()),
            ]
        );
        assert_eq!(ResId::Id(7).id(), Some(7));
        assert_eq!(ResId::Name("WAVE".into()).id(), None);
    }

    #[test]
    fn a_pe32_plus_file_reads_its_resources_too() {
        let file = tiny_pe(&[(BITMAP, 7, 1033, b"bits")], true);
        assert_eq!(listed(&file).len(), 1);
        // Any other optional-header magic is not a PE image.
        let mut odd = file.clone();
        odd[0x40 + 24] = 0x07;
        assert_eq!(resources(&odd).unwrap_err(), PeError::NotPe);
    }

    #[test]
    fn little_endian_words_are_read_at_their_offset() {
        assert_eq!(u16_at(&[1, 2, 3], 0, "x"), Ok(0x0201));
        assert_eq!(u16_at(&[1, 2, 3], 1, "x"), Ok(0x0302));
        assert_eq!(u32_at(&[1, 2, 3, 4], 0, "x"), Ok(0x0403_0201));
        assert_eq!(u16_at(&[1, 2], 1, "x"), Err(PeError::Truncated("x")));
    }

    #[test]
    fn a_file_without_a_resource_table_is_refused() {
        let file = tiny_pe(&[(BITMAP, 7, 1033, b"bits")], false);
        let directory = 0x40 + 24 + 96 + 16;
        for (at, len) in [(directory, 4), (directory + 4, 4)] {
            let mut none = file.clone();
            none[at..at + len].fill(0);
            assert_eq!(resources(&none).unwrap_err(), PeError::NoResources, "zeroed at {at:#x}");
        }
    }

    #[test]
    fn resource_data_must_lie_inside_its_section() {
        let mut file = tiny_pe(&[(BITMAP, 7, 1033, b"bits")], false);
        let table_size = u32::from_le_bytes(file[0x40 + 24 + 96 + 20..][..4].try_into().unwrap());
        let image = Image::parse(&file).unwrap();
        // The last byte of the section reads; one past it does not.
        assert_eq!(image.range(0x1000 + table_size - 1, 1).map(<[u8]>::len), Ok(1));
        assert!(image.range(0x1000 + table_size - 1, 2).is_err());
        assert!(image.range(0x1000 + table_size, 0).is_err());
        assert_eq!(image.range(0x1000, table_size).map(<[u8]>::len), Ok(table_size as usize));
        assert!(image.range(0x0fff, 1).is_err());
        // A data entry pointing past the section fails the whole read
        // (one entry: root 24, name and language directories 24 each).
        let entry = TABLE + 24 + 24 + 24;
        file[entry..entry + 4].copy_from_slice(&(0x1000 + table_size).to_le_bytes());
        assert!(matches!(resources(&file), Err(PeError::BadData { .. })));
    }

    #[test]
    fn leaves_where_directories_belong_are_skipped() {
        let file = tiny_pe(&[(BITMAP, 7, 1033, b"bits"), (Some(10), 8, 1033, b"rec")], false);
        assert_eq!(listed(&file).len(), 2);
        // The high byte of the first target at each level: the root's
        // entry, the name directory's, the language directory's.
        let (root, names, languages) = (TABLE + 16 + 7, TABLE + 32 + 16 + 7, TABLE + 32 + 48 + 16 + 7);
        let mut leaf_type = file.clone();
        leaf_type[root] &= 0x7f;
        assert_eq!(listed(&leaf_type).len(), 1, "a type entry that is a leaf");
        let mut leaf_name = file.clone();
        leaf_name[names] &= 0x7f;
        assert_eq!(listed(&leaf_name).len(), 1, "a name entry that is a leaf");
        let mut deep = file.clone();
        deep[languages] |= 0x80;
        assert_eq!(listed(&deep).len(), 1, "a language entry that is a directory");
        let mut named_language = file.clone();
        named_language[languages - 4] |= 0x80;
        assert_eq!(listed(&named_language).len(), 1, "a language given by name");
    }

    #[test]
    fn errors_say_what_failed() {
        assert_eq!(PeError::NotPe.to_string(), "not a PE file");
        assert_eq!(PeError::BadData { rva: 16, size: 2 }.to_string(), "resource data 0x10+2 is outside the file's sections");
    }

    #[test]
    fn a_file_that_is_not_pe_is_refused() {
        assert_eq!(resources(b"not a dll").unwrap_err(), PeError::NotPe);
        assert_eq!(resources(&[]).unwrap_err(), PeError::NotPe);
    }

    #[test]
    #[ignore = "requires an original installation in ~/Games/Heroic/Star Wars - Rebellion"]
    fn the_strategy_dll_lists_its_bitmaps_and_waves() {
        let path = std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/Heroic/Star Wars - Rebellion/STRATEGY.DLL");
        let file = std::fs::read(path).unwrap();
        let all = resources(&file).unwrap();
        // tools/stage-ui-assets/catalog.go: 1,042 bitmaps and 66 WAVEs.
        let bitmaps = all.iter().filter(|r| r.kind == ResId::Id(2)).count();
        let waves = all.iter().filter(|r| r.kind == ResId::Name("WAVE".into())).count();
        assert_eq!((bitmaps, waves), (1042, 66));
    }
}
