//! The Encyclopedia's source catalog and its manifest, decoded from the
//! player's ENCYTEXT.DLL (topic texts, RT_RCDATA) and ENCYBMAP.DLL (artwork
//! names, RT_STRING), byte for byte as `tools/stage-ui-assets`
//! (`encyclopedia.go`) writes them: the manifest's checksum covers the
//! catalog's exact bytes, which the runtime checks.
//!
//! port: the catalog format is the port's own; its content is the DLLs'.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use dat_dumper::types::textstra;
use sha2::{Digest, Sha256};

use super::pack::{BuildError, Entry, InstallFiles, KIND_GAME_DATA};
use super::pe::{self, ResId};

const SCHEMA_VERSION: u32 = 1;
const LANGUAGE_ID: u32 = 1033;
const RT_STRING: u32 = 6;
const RT_RCDATA: u32 = 10;
/// The English release's inventory (`encyclopediaExpectedTexts`/`Maps`).
const EXPECTED_TEXTS: usize = 348;
const EXPECTED_ARTWORK: usize = 191;

fn bad(why: String) -> BuildError {
    BuildError::BadTable(why)
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// A topic text: Windows-1252, ended by NULs, line ends as `\n`
/// (`decodeWindows1252Text`).
fn decode_text(id: u32, raw: &[u8]) -> Result<String, BuildError> {
    if raw.last() != Some(&0) {
        return Err(bad(format!("ENCYTEXT {id}: missing terminal NUL")));
    }
    let end = raw.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let body = &raw[..end];
    if body.contains(&0) {
        return Err(bad(format!("ENCYTEXT {id}: interior NUL")));
    }
    let mut text = String::with_capacity(body.len());
    for &byte in body {
        let c = windows_1252(byte).ok_or_else(|| bad(format!("ENCYTEXT {id}: undefined Windows-1252 byte {byte:#04x}")))?;
        text.push(c);
    }
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

fn windows_1252(byte: u8) -> Option<char> {
    const HIGH: [Option<char>; 32] = [
        Some('€'), None, Some('‚'), Some('ƒ'), Some('„'), Some('…'), Some('†'), Some('‡'),
        Some('ˆ'), Some('‰'), Some('Š'), Some('‹'), Some('Œ'), None, Some('Ž'), None,
        None, Some('‘'), Some('’'), Some('“'), Some('”'), Some('•'), Some('–'), Some('—'),
        Some('˜'), Some('™'), Some('š'), Some('›'), Some('œ'), None, Some('ž'), Some('Ÿ'),
    ];
    match byte {
        0x80..=0x9f => HIGH[usize::from(byte - 0x80)],
        _ => Some(char::from(byte)),
    }
}

/// A string as Go's `encoding/json` writes it: `<`, `>`, `&`, U+2028 and
/// U+2029 escaped as well.
fn go_string(s: &str) -> String {
    let json = serde_json::to_string(s).unwrap_or_default();
    json.replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

/// A map with number keys as Go writes it: keys sorted as strings.
fn go_map(out: &mut String, indent: &str, items: &BTreeMap<String, String>) {
    if items.is_empty() {
        out.push_str("{}");
        return;
    }
    out.push_str("{\n");
    for (i, (key, value)) in items.iter().enumerate() {
        let _ = write!(out, "{indent}  {}: {value}", go_string(key));
        out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
    }
    out.push_str(indent);
    out.push('}');
}

/// The catalog's and manifest's bytes.
fn catalog(texts: &[(u32, &[u8])], artwork_bundles: &[(u32, &[u8])], text_dll: &[u8], bitmap_dll: &[u8]) -> Result<(Vec<u8>, Vec<u8>), BuildError> {
    let mut bodies = BTreeMap::new();
    for &(id, raw) in texts {
        if id == 0 || id > 0xffff {
            return Err(bad(format!("ENCYTEXT has invalid resource id {id}")));
        }
        let body = decode_text(id, raw)?;
        if body.is_empty() {
            return Err(bad(format!("ENCYTEXT {id} is empty")));
        }
        let value = format!(
            "{{\n      \"body\": {},\n      \"body_sha256\": \"{}\"\n    }}",
            go_string(&body),
            hex(body.as_bytes())
        );
        if bodies.insert(id.to_string(), value).is_some() {
            return Err(bad(format!("duplicate ENCYTEXT {id}")));
        }
    }
    let mut names = HashMap::new();
    for &(bundle, raw) in artwork_bundles {
        if bundle == 0 || bundle > 4096 {
            return Err(bad(format!("ENCYBMAP bundle {bundle} outside the string range")));
        }
        textstra::parse_string_bundle(raw, (bundle - 1) * 16, &mut names).map_err(|e| bad(format!("ENCYBMAP: {e:#}")))?;
    }
    let mut artwork = BTreeMap::new();
    for (id, name) in names {
        let valid = name.strip_prefix("EDATA.").is_some_and(|n| n.len() == 3 && n.bytes().all(|b| b.is_ascii_digit()));
        if id == 0 || !valid {
            return Err(bad(format!("ENCYBMAP {id} has invalid artwork name {name:?}")));
        }
        artwork.insert(id.to_string(), go_string(&name));
    }
    if bodies.len() != EXPECTED_TEXTS || artwork.len() != EXPECTED_ARTWORK {
        return Err(bad(format!(
            "the Encyclopedia has {} topics and {} pictures, expected {EXPECTED_TEXTS} and {EXPECTED_ARTWORK}",
            bodies.len(),
            artwork.len()
        )));
    }
    let mut out = String::new();
    let _ = write!(
        out,
        "{{\n  \"schema_version\": {SCHEMA_VERSION},\n  \"language_id\": {LANGUAGE_ID},\n  \"encoding\": \"windows-1252\",\n  \"source_code_page\": 0,\n  \"texts\": "
    );
    go_map(&mut out, "  ", &bodies);
    out.push_str(",\n  \"artwork\": ");
    go_map(&mut out, "  ", &artwork);
    out.push_str("\n}\n");
    let manifest = format!(
        "{{\n  \"schema_version\": {SCHEMA_VERSION},\n  \"catalog_sha256\": \"{}\",\n  \"source_files\": {{\n    \"encytext_sha256\": \"{}\",\n    \"encybmap_sha256\": \"{}\"\n  }},\n  \"counts\": {{\n    \"texts\": {},\n    \"artwork_mappings\": {}\n  }}\n}}\n",
        hex(out.as_bytes()),
        hex(text_dll),
        hex(bitmap_dll),
        bodies.len(),
        artwork.len()
    );
    Ok((out.into_bytes(), manifest.into_bytes()))
}

/// Each numbered resource of `kind`, checked to be English with no code
/// page (`decodeEncyclopediaSource`).
fn english<'a>(file: &str, resources: &[pe::Resource<'a>], kind: u32) -> Result<Vec<(u32, &'a [u8])>, BuildError> {
    resources
        .iter()
        .filter(|r| r.kind == ResId::Id(kind))
        .map(|r| {
            let ResId::Id(id) = r.name else {
                return Err(bad(format!("{file} has a named resource")));
            };
            if r.language != LANGUAGE_ID {
                return Err(bad(format!("{file} resource {id} has language {}", r.language)));
            }
            Ok((id, r.data))
        })
        .collect()
}

/// The two Encyclopedia tables.
///
/// # Errors
/// A missing DLL, or resources other than the English release's.
pub fn encyclopedia_tables(install: &InstallFiles, out: &mut Vec<Entry>) -> Result<(), BuildError> {
    let read = |file: &str| install.get(file).ok_or_else(|| BuildError::Missing(file.to_owned()));
    let (text_dll, bitmap_dll) = (read("ENCYTEXT.DLL")?, read("ENCYBMAP.DLL")?);
    let parse = |file: &str, bytes| pe::resources(bytes).map_err(|error| BuildError::Pe { file: file.to_owned(), error });
    let texts = parse("ENCYTEXT.DLL", text_dll)?;
    let bitmaps = parse("ENCYBMAP.DLL", bitmap_dll)?;
    let (catalog, manifest) = catalog(
        &english("ENCYTEXT.DLL", &texts, RT_RCDATA)?,
        &english("ENCYBMAP.DLL", &bitmaps, RT_STRING)?,
        text_dll,
        bitmap_dll,
    )?;
    out.push(Entry { kind: KIND_GAME_DATA, key: "encyclopedia-source.json".into(), data: catalog });
    out.push(Entry { kind: KIND_GAME_DATA, key: "encyclopedia-source.manifest.json".into(), data: manifest });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_topic_text_keeps_windows_1252_and_unix_line_ends() {
        assert_eq!(decode_text(1, b"A\x92b\r\nc\rd\0\0").unwrap(), "A\u{2019}b\nc\nd");
        assert!(decode_text(1, b"no end").is_err());
        assert!(decode_text(1, b"a\0b\0").is_err());
        assert!(decode_text(1, b"\x81\0").is_err());
    }

    #[test]
    fn strings_escape_as_go_writes_them() {
        assert_eq!(go_string("a<b>&c"), "\"a\\u003cb\\u003e\\u0026c\"");
    }
}
