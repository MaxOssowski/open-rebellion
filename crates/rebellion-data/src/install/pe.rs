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

#[cfg(test)]
mod tests {
    use super::*;

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
