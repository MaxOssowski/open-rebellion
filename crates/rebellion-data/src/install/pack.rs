//! The runtime pack (ORPK v3) built from an original installation's files,
//! the same entries `scripts/build-runtime-pack.py` packs from the staged
//! tree (`docs/plans/2026-11-24-bring-your-own-data.md`).
//!
//! port: the pack format and every key are the port's own; the resources
//! come from the player's DLLs as `tools/stage-ui-assets` stages them.

use std::collections::BTreeMap;
use std::fmt;

use super::pe::{self, ResId};

pub const KIND_GAME_DATA: u8 = 0;
pub const KIND_BITMAP: u8 = 1;
pub const KIND_AUDIO: u8 = 2;
pub const KIND_ADVISOR_FRAME: u8 = 3;

const MAGIC: &[u8; 4] = b"ORPK";
const VERSION: u16 = 3;
const RT_BITMAP: u32 = 2;
const RT_RCDATA: u32 = 10;
const RT_ADVISOR_FRAME: u32 = 302;

/// The files of an installation, found by name whatever their folder and
/// case (the GOG release keeps the DATs in `GData`, the music in `MDATA`,
/// the encyclopedia's pictures in `EData`).
#[derive(Debug, Default, Clone)]
pub struct InstallFiles {
    /// Upper-case file name -> (upper-case name of its folder, bytes).
    files: BTreeMap<String, (String, Vec<u8>)>,
}

impl InstallFiles {
    /// Add a file by its path; its name and the folder holding it count
    /// (the GOG release has an uninstaller's `UNINS000.DAT` beside the
    /// game, and the game's DATs in `GData`).
    pub fn insert(&mut self, path: &str, bytes: Vec<u8>) {
        let mut parts = path.rsplit(['/', '\\']);
        let name = parts.next().unwrap_or(path).to_ascii_uppercase();
        let folder = parts.next().unwrap_or("").to_ascii_uppercase();
        // A file in GData wins over one of the same name elsewhere.
        if self.files.get(&name).is_some_and(|(f, _)| f == "GDATA") && folder != "GDATA" {
            return;
        }
        self.files.insert(name, (folder, bytes));
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.files.get(&name.to_ascii_uppercase()).map(|(_, bytes)| bytes.as_slice())
    }

    /// The files whose upper-case folder and name `keep` accepts, in name
    /// order.
    pub fn matching<'a>(
        &'a self,
        keep: impl Fn(&str, &str) -> bool + 'a,
    ) -> impl Iterator<Item = (&'a str, &'a [u8])> + 'a {
        self.files
            .iter()
            .filter(move |(name, (folder, _))| keep(folder, name))
            .map(|(name, (_, bytes))| (name.as_str(), bytes.as_slice()))
    }
}

/// Why a pack could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    Missing(String),
    Pe { file: String, error: pe::PeError },
    Count { file: String, what: &'static str, found: usize, expected: usize },
    Duplicate(String),
    BadBitmap(String),
    UnknownNamedBitmap { file: String, name: String },
    BadTable(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "{name} is missing from the game folder"),
            Self::Pe { file, error } => write!(f, "{file}: {error}"),
            Self::Count { file, what, found, expected } => {
                write!(f, "{file}: found {found} {what}, expected {expected}")
            }
            Self::Duplicate(key) => write!(f, "duplicate pack key {key}"),
            Self::BadBitmap(key) => write!(f, "{key} is not a bitmap the pack can hold"),
            Self::UnknownNamedBitmap { file, name } => write!(f, "{file}: unknown named bitmap {name}"),
            Self::BadTable(why) => write!(f, "a text table does not decode: {why}"),
        }
    }
}

impl std::error::Error for BuildError {}

/// One DLL the pack takes resources from, with the counts the original
/// release has (`tools/stage-ui-assets/catalog.go`, `uiDLLTargets`).
struct Target {
    file: &'static str,
    /// Other names the GOG release ships it under.
    alias: Option<&'static str>,
    dir: &'static str,
    bitmaps: usize,
    frames: usize,
    records: usize,
    waves: usize,
    optional: bool,
}

const TARGETS: &[Target] = &[
    Target { file: "COMMON.DLL", alias: None, dir: "common-dll", bitmaps: 321, frames: 0, records: 0, waves: 0, optional: false },
    Target { file: "GOKRES.DLL", alias: None, dir: "gokres-dll", bitmaps: 580, frames: 0, records: 0, waves: 0, optional: false },
    Target { file: "STRATEGY.DLL", alias: None, dir: "strategy-dll", bitmaps: 1042, frames: 0, records: 0, waves: 66, optional: false },
    Target { file: "TACTICAL.DLL", alias: None, dir: "tactical-dll", bitmaps: 288, frames: 0, records: 0, waves: 0, optional: false },
    Target { file: "ALSPRITE.DLL", alias: None, dir: "alsprite-dll", bitmaps: 38, frames: 1640, records: 752, waves: 213, optional: false },
    Target { file: "EMSPRITE.DLL", alias: None, dir: "emsprite-dll", bitmaps: 34, frames: 2348, records: 753, waves: 216, optional: false },
    Target { file: "ALBRIEF.DLL", alias: Some("ALBRIEF.DLL.OLD"), dir: "albrief-dll", bitmaps: 20, frames: 2684, records: 366, waves: 17, optional: true },
    Target { file: "EMBRIEF.DLL", alias: Some("EMBRIEF.DLL.OLD"), dir: "embrief-dll", bitmaps: 18, frames: 2738, records: 471, waves: 22, optional: true },
    Target { file: "REBDLOG.DLL", alias: None, dir: "rebdlog-dll", bitmaps: 23, frames: 0, records: 0, waves: 0, optional: false },
];

/// Named bitmaps REBEXE.EXE loads by number (`namedBitmapIDs`); one it
/// never loads is skipped (`unloadedNamedBitmaps`).
const NAMED_BITMAPS: &[(&str, u32)] = &[
    ("COCKPIT_BUTTON_GAMESCALE_HUGE_UP", 15856),
    ("COCKPIT_BUTTON_GAMESCALE_LARGE_UP", 15922),
    ("COCKPIT_BUTTON_GAMESCALE_STD_UP", 15990),
    ("DATA_BUTTON_UP_FIGHTERGROUP_RECOVER", 40720),
    ("DATA_BUTTON_DN_FIGHTERGROUP_RECOVER", 40792),
    ("DATA_BUTTON_UP_FIGHTERGROUP_TACTICS", 40864),
    ("DATA_BUTTON_DN_FIGHTERGROUP_TACTICS", 40936),
];
const UNLOADED_NAMED_BITMAPS: &[&str] = &["DLG_CORNER_GRAB_FRAME"];

/// The DLLs whose WAVEs ride under `advisor/` (`ADVISOR_VOICE_DIRS`).
const ADVISOR_VOICE_DIRS: &[&str] = &["albrief-dll", "embrief-dll", "alsprite-dll", "emsprite-dll", "strategy-dll"];

/// The score: MDATA.300 as the cockpit's main theme, 301..315 by number.
const MUSIC: &[(&str, &str)] = &[
    ("MDATA.300", "music/main_theme.wav"),
    ("MDATA.301", "music/301.wav"),
    ("MDATA.302", "music/302.wav"),
    ("MDATA.303", "music/303.wav"),
    ("MDATA.304", "music/304.wav"),
    ("MDATA.305", "music/305.wav"),
    ("MDATA.306", "music/306.wav"),
    ("MDATA.307", "music/307.wav"),
    ("MDATA.308", "music/308.wav"),
    ("MDATA.309", "music/309.wav"),
    ("MDATA.310", "music/310.wav"),
    ("MDATA.311", "music/311.wav"),
    ("MDATA.312", "music/312.wav"),
    ("MDATA.313", "music/313.wav"),
    ("MDATA.314", "music/314.wav"),
    ("MDATA.315", "music/315.wav"),
];

/// The menu's sound effects in COMMON.DLL (`dat-dumper --extract-menu-sfx`).
const MENU_SFX: &[(u32, &str)] = &[
    (8000, "menu_galaxy_size"),
    (8001, "menu_load_options"),
    (8002, "menu_quit"),
    (8004, "menu_select"),
];

/// The tactical events' sounds in TACTICAL.DLL, `tactical_event_{event}_{n}`
/// (`dat-dumper --extract-tactical-sfx`).
const TACTICAL_SFX: &[(u32, &str)] = &[
    (13_000, "00_0"), (13_001, "00_1"), (13_002, "00_2"), (13_003, "00_3"),
    (13_004, "01_0"), (13_005, "01_1"), (13_006, "01_2"), (13_007, "01_3"),
    (13_008, "02_0"), (13_009, "02_1"), (13_010, "02_2"), (13_011, "02_3"),
    (13_012, "03_0"), (13_013, "03_1"), (13_014, "04_0"), (13_015, "04_1"),
    (13_016, "05_0"), (13_017, "05_1"),
    (13_030, "0c_0"), (13_031, "0c_1"), (13_032, "0c_2"),
    (13_033, "0d_0"), (13_034, "0d_1"), (13_035, "0d_2"),
    (13_036, "0e_0"), (13_037, "0e_1"), (13_038, "0e_2"),
    (13_045, "11_0"), (13_046, "11_1"), (13_047, "11_2"),
    (13_048, "12_0"), (13_049, "12_1"), (13_050, "12_2"),
    (13_051, "13_0"), (13_052, "13_1"), (13_053, "13_2"),
    (13_054, "14_0"), (13_055, "15_0"), (13_056, "16_0"), (13_057, "17_0"),
    (13_058, "18_0"), (13_059, "19_0"), (13_061, "1b_0"), (13_062, "1c_0"),
    (13_063, "1d_0"), (13_064, "1e_0"), (13_065, "1f_0"),
];

/// The tactical voice banks: file, side folder, suffix, WAVE id ranges
/// (`dat-dumper --extract-tactical-voice`).
const VOICE_BANKS: &[(&str, &str, &str, &[(u32, u32)])] = &[
    ("VOICEFXA.DLL", "alliance", "voicefxa", &[(14_001, 14_122), (15_133, 15_163)]),
    ("VOICEFXE.DLL", "empire", "voicefxe", &[(15_001, 15_132)]),
];

/// A DLL's WAVE resources by number; each must be RIFF/WAVE audio.
fn waves<'a>(install: &'a InstallFiles, file: &str) -> Result<BTreeMap<u32, &'a [u8]>, BuildError> {
    let bytes = install.get(file).ok_or_else(|| BuildError::Missing(file.to_owned()))?;
    let resources = pe::resources(bytes).map_err(|error| BuildError::Pe { file: file.to_owned(), error })?;
    Ok(resources
        .into_iter()
        .filter(|r| r.kind == ResId::Name("WAVE".into()))
        .filter_map(|r| Some((r.name.id()?, r.data)))
        .collect())
}

/// The named WAVE ids of `file` as `prefix{name}.wav` entries.
fn wave_entries(
    install: &InstallFiles,
    file: &str,
    ids: impl IntoIterator<Item = (u32, String)>,
    out: &mut Vec<Entry>,
) -> Result<(), BuildError> {
    let waves = waves(install, file)?;
    for (id, key) in ids {
        let data = waves.get(&id).ok_or_else(|| BuildError::Missing(format!("{file} WAVE {id}")))?;
        if !data.starts_with(b"RIFF") || data.get(8..12) != Some(b"WAVE") {
            return Err(BuildError::BadBitmap(format!("{file} WAVE {id} is not RIFF/WAVE audio")));
        }
        out.push(Entry { kind: KIND_AUDIO, key, data: data.to_vec() });
    }
    Ok(())
}

/// A pack entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: u8,
    pub key: String,
    pub data: Vec<u8>,
}

/// A DIB resource as a BMP file: the file header the resource leaves out
/// (`tools/stage-ui-assets/bmp.go`, `dibToBMP`).
#[must_use]
pub fn dib_to_bmp(dib: &[u8]) -> Option<Vec<u8>> {
    let header = u32::from_le_bytes(dib.get(0..4)?.try_into().ok()?);
    if header < 40 || header as usize > dib.len() {
        return None;
    }
    let bits = u16::from_le_bytes(dib.get(14..16)?.try_into().ok()?);
    let mut colors = u32::from_le_bytes(dib.get(32..36)?.try_into().ok()?);
    if colors == 0 && bits <= 8 {
        colors = 1 << bits;
    }
    let pixels = 14u64 + u64::from(header) + u64::from(colors) * 4;
    let size = 14u64 + dib.len() as u64;
    if pixels > size || size > u64::from(u32::MAX) {
        return None;
    }
    let mut bmp = Vec::with_capacity(size as usize);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&(pixels as u32).to_le_bytes());
    bmp.extend_from_slice(dib);
    Some(bmp)
}

fn count(file: &str, what: &'static str, found: usize, expected: usize) -> Result<(), BuildError> {
    if found == expected {
        Ok(())
    } else {
        Err(BuildError::Count { file: file.to_owned(), what, found, expected })
    }
}

/// The entries one DLL target gives: its bitmaps, and for the advisor's
/// DLLs its frames, clip records and WAVEs.
fn target_entries(target: &Target, file: &[u8], out: &mut Vec<Entry>) -> Result<(), BuildError> {
    let resources = pe::resources(file).map_err(|error| BuildError::Pe { file: target.file.to_owned(), error })?;
    let bitmap_kind = ResId::Id(RT_BITMAP);
    let mut bitmaps = 0;
    for resource in resources.iter().filter(|r| r.kind == bitmap_kind) {
        let id = match &resource.name {
            ResId::Id(id) => *id,
            ResId::Name(name) if UNLOADED_NAMED_BITMAPS.contains(&name.as_str()) => continue,
            ResId::Name(name) => NAMED_BITMAPS
                .iter()
                .find(|(n, _)| n == name)
                .map(|&(_, id)| id)
                .ok_or_else(|| BuildError::UnknownNamedBitmap { file: target.file.to_owned(), name: name.clone() })?,
        };
        let key = format!("{}/{id}", target.dir);
        let data = dib_to_bmp(resource.data).ok_or_else(|| BuildError::BadBitmap(key.clone()))?;
        out.push(Entry { kind: KIND_BITMAP, key, data });
        bitmaps += 1;
    }
    count(target.file, "bitmaps", bitmaps, target.bitmaps)?;
    let numbered = |kind: ResId, what: &'static str, expected: usize| -> Result<Vec<(u32, &[u8])>, BuildError> {
        let list: Vec<(u32, &[u8])> = resources
            .iter()
            .filter(|r| r.kind == kind)
            .filter_map(|r| Some((r.name.id()?, r.data)))
            .collect();
        if expected > 0 {
            count(target.file, what, list.len(), expected)?;
        }
        Ok(list)
    };
    if target.frames > 0 {
        for (id, data) in numbered(ResId::Id(RT_ADVISOR_FRAME), "type-302 frames", target.frames)? {
            out.push(Entry { kind: KIND_ADVISOR_FRAME, key: format!("{}/{id}", target.dir), data: data.to_vec() });
        }
    }
    if target.records > 0 {
        for (id, data) in numbered(ResId::Id(RT_RCDATA), "RCDATA records", target.records)? {
            out.push(Entry { kind: KIND_ADVISOR_FRAME, key: format!("{}/rcdata/{id}", target.dir), data: data.to_vec() });
        }
    }
    if target.waves > 0 && ADVISOR_VOICE_DIRS.contains(&target.dir) {
        for (id, data) in numbered(ResId::Name("WAVE".into()), "WAVE clips", target.waves)? {
            out.push(Entry { kind: KIND_AUDIO, key: format!("advisor/{}/{id}.wav", target.dir), data: data.to_vec() });
        }
    }
    Ok(())
}

/// The pack's entries this module builds from `install`, sorted by kind
/// and key as the pack keeps them.
///
/// # Errors
/// A required file missing, a DLL that does not read, or counts other than
/// the original release's.
pub fn build_entries(install: &InstallFiles) -> Result<Vec<Entry>, BuildError> {
    let mut out = Vec::new();
    // Kind 0: the DATs and the advisor's scripts, byte for byte.
    for (name, data) in install.matching(|folder, name| folder == "GDATA" && name.ends_with(".DAT")) {
        out.push(Entry { kind: KIND_GAME_DATA, key: name.to_owned(), data: data.to_vec() });
    }
    for name in ["C3POACT.SPT", "IMP22ACT.SPT"] {
        let data = install.get(name).ok_or_else(|| BuildError::Missing(name.to_owned()))?;
        out.push(Entry { kind: KIND_GAME_DATA, key: name.to_owned(), data: data.to_vec() });
    }
    // The encyclopedia's pictures (collect_edata_entries).
    for (name, data) in install.matching(|_, name| {
        name.strip_prefix("EDATA.").is_some_and(|n| n.len() == 3 && n.bytes().all(|b| b.is_ascii_digit()))
    }) {
        out.push(Entry { kind: KIND_GAME_DATA, key: format!("encyclopedia/assets/{name}"), data: data.to_vec() });
    }
    for target in TARGETS {
        let file = install.get(target.file).or_else(|| target.alias.and_then(|alias| install.get(alias)));
        match file {
            Some(file) => target_entries(target, file, &mut out)?,
            None if target.optional => {}
            None => return Err(BuildError::Missing(target.file.to_owned())),
        }
    }
    wave_entries(
        install,
        "COMMON.DLL",
        MENU_SFX.iter().map(|&(id, name)| (id, format!("sfx/{name}.wav"))),
        &mut out,
    )?;
    wave_entries(
        install,
        "TACTICAL.DLL",
        TACTICAL_SFX.iter().map(|&(id, event)| (id, format!("sfx/tactical_event_{event}.wav"))),
        &mut out,
    )?;
    for &(file, side, suffix, ranges) in VOICE_BANKS {
        let ids = ranges
            .iter()
            .flat_map(|&(first, last)| first..=last)
            .map(|id| (id, format!("voice/{side}/{id}-{suffix}.wav")));
        wave_entries(install, file, ids, &mut out)?;
    }
    super::strings::text_tables(install, &mut out)?;
    super::encyclopedia::encyclopedia_tables(install, &mut out)?;
    super::cursor::cursor_entries(install, &mut out)?;
    super::tactical::tactical_entries(install, &mut out)?;
    for &(file, key) in MUSIC {
        if let Some(data) = install.get(file) {
            out.push(Entry { kind: KIND_AUDIO, key: key.to_owned(), data: data.to_vec() });
        }
    }
    out.sort_by(|a, b| (a.kind, &a.key).cmp(&(b.kind, &b.key)));
    if let Some(pair) = out.windows(2).find(|w| w[0].kind == w[1].kind && w[0].key == w[1].key) {
        return Err(BuildError::Duplicate(pair[0].key.clone()));
    }
    Ok(out)
}

/// The ORPK v3 bytes of `entries` (sorted by kind and key).
#[must_use]
pub fn write_pack(entries: &[Entry]) -> Vec<u8> {
    let size = 12 + entries.iter().map(|e| 7 + e.key.len() + e.data.len()).sum::<usize>();
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for entry in entries {
        out.push(entry.kind);
        out.extend_from_slice(&(entry.key.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
        out.extend_from_slice(entry.key.as_bytes());
        out.extend_from_slice(&entry.data);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("Games/Heroic/Star Wars - Rebellion")
    }

    fn read_install() -> InstallFiles {
        let mut files = InstallFiles::default();
        for dir in ["", "GData", "MDATA", "EData"] {
            for entry in std::fs::read_dir(install_dir().join(dir)).unwrap() {
                let path = entry.unwrap().path();
                if path.is_file() {
                    files.insert(path.to_str().unwrap(), std::fs::read(&path).unwrap());
                }
            }
        }
        files
    }

    /// The offline pack's entries, by (kind, key).
    fn offline_pack() -> BTreeMap<(u8, String), Vec<u8>> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/data/runtime.orpk");
        let data = std::fs::read(path).unwrap();
        let count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
        let mut at = 12;
        let mut out = BTreeMap::new();
        for _ in 0..count {
            let kind = data[at];
            let klen = usize::from(u16::from_le_bytes(data[at + 1..at + 3].try_into().unwrap()));
            let dlen = u32::from_le_bytes(data[at + 3..at + 7].try_into().unwrap()) as usize;
            at += 7;
            let key = String::from_utf8(data[at..at + klen].to_vec()).unwrap();
            at += klen;
            out.insert((kind, key), data[at..at + dlen].to_vec());
            at += dlen;
        }
        out
    }

    #[test]
    fn a_file_in_gdata_wins_over_one_of_the_same_name_elsewhere() {
        // The GOG release has an uninstaller's UNINS000.DAT beside the game.
        for order in [["Game/GData/X.DAT", "Game/X.DAT"], ["Game/X.DAT", "Game/GData/X.DAT"]] {
            let mut files = InstallFiles::default();
            for path in order {
                files.insert(path, path.as_bytes().to_vec());
            }
            assert_eq!(files.get("x.dat"), Some(&b"Game/GData/X.DAT"[..]), "{order:?}");
        }
        let mut files = InstallFiles::default();
        files.insert("Game/A.DAT", b"one".to_vec());
        files.insert("Other/a.dat", b"two".to_vec());
        assert_eq!(files.get("A.DAT"), Some(&b"two"[..]), "elsewhere, the later file wins");
        let in_gdata: Vec<_> = files.matching(|folder, _| folder == "GDATA").collect();
        assert!(in_gdata.is_empty());
    }

    #[test]
    fn a_wave_resource_that_is_not_riff_audio_is_refused() {
        let riff = b"RIFF\x04\0\0\0WAVE";
        for (data, ok) in [(&riff[..], true), (&b"RIFF\x04\0\0\0AVI "[..], false), (&b"RIFX\x04\0\0\0WAVE"[..], false)] {
            let mut files = InstallFiles::default();
            files.insert("COMMON.DLL", super::super::pe::fixture::tiny_pe(&[(None, 8000, 1033, data)], false));
            let mut out = Vec::new();
            let result = wave_entries(&files, "COMMON.DLL", [(8000, "sfx/a.wav".to_owned())], &mut out);
            assert_eq!(result.is_ok(), ok, "{data:?}");
            assert_eq!(out.len(), usize::from(ok));
        }
    }

    #[test]
    fn build_errors_name_the_file_and_the_cause() {
        assert_eq!(BuildError::Missing("COMMON.DLL".into()).to_string(), "COMMON.DLL is missing from the game folder");
        assert_eq!(
            BuildError::Count { file: "X.DLL".into(), what: "bitmaps", found: 1, expected: 2 }.to_string(),
            "X.DLL: found 1 bitmaps, expected 2"
        );
    }

    #[test]
    fn a_dib_gains_the_file_header_its_resource_leaves_out() {
        // A 1x1 8-bit DIB with two palette entries used.
        let mut dib = vec![0u8; 40 + 2 * 4 + 4];
        dib[0..4].copy_from_slice(&40u32.to_le_bytes());
        dib[14..16].copy_from_slice(&8u16.to_le_bytes());
        dib[32..36].copy_from_slice(&2u32.to_le_bytes());
        let bmp = dib_to_bmp(&dib).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
        assert_eq!(u32::from_le_bytes(bmp[2..6].try_into().unwrap()) as usize, 14 + dib.len());
        assert_eq!(u32::from_le_bytes(bmp[10..14].try_into().unwrap()), 14 + 40 + 8);
        assert_eq!(&bmp[14..], &dib[..]);
        assert!(dib_to_bmp(&dib[..20]).is_none());
    }

    #[test]
    #[ignore = "requires an original installation and a staged web/data/runtime.orpk"]
    fn the_pack_built_from_the_install_matches_the_offline_pack() {
        let built = build_entries(&read_install()).unwrap();
        let offline = offline_pack();
        let mut per_kind: BTreeMap<u8, (usize, usize)> = BTreeMap::new();
        for entry in &built {
            let slot = per_kind.entry(entry.kind).or_default();
            slot.0 += 1;
            match offline.get(&(entry.kind, entry.key.clone())) {
                Some(data) if *data == entry.data => slot.1 += 1,
                Some(_) => panic!("{} {} differs from the offline pack", entry.kind, entry.key),
                None => panic!("{} {} is not in the offline pack", entry.kind, entry.key),
            }
        }
        let missing: Vec<_> = offline
            .keys()
            .filter(|(kind, key)| !built.iter().any(|e| e.kind == *kind && &e.key == key))
            .collect();
        eprintln!("built and equal per kind: {per_kind:?}; offline entries not built yet: {}", missing.len());
        // Every entry built equals the offline pack's (checked above), and
        // every offline entry is built.
        assert!(missing.is_empty(), "{missing:?}");
        assert_eq!(built.len(), offline.len());
        // And the written pack is the offline file, byte for byte.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/data/runtime.orpk");
        assert!(write_pack(&built) == std::fs::read(path).unwrap(), "the written pack differs from the offline file");
    }
}
