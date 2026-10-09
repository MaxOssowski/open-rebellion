//! TEXTSTRA.DLL — Win32 PE `RT_STRING` resource extraction.
//!
//! `RT_STRING` resources store strings in bundles of 16. Bundle with resource ID N
//! holds string IDs (N-1)*16 through (N-1)*16+15. Each string entry is a u16
//! length (in UTF-16 code units) followed by that many little-endian UTF-16LE
//! code units. A length of 0 means the slot is empty.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use pelite::resources::{Entry, Name};
use pelite::{FileMap, PeFile};

/// `RT_STRING` resource type ID.
const RT_STRING_ID: u32 = 6;

/// Load all `RT_STRING` entries from a Win32 PE DLL into a `HashMap<string_id, name>`.
///
/// Works on both 32-bit and 64-bit PE images. Empty string slots (length == 0)
/// are omitted from the result.
///
/// # Errors
/// Returns an error if the DLL cannot be read, its PE resource tree is invalid,
/// or a string bundle is truncated or malformed.
pub fn load_strings(path: &Path) -> anyhow::Result<HashMap<u16, String>> {
    let map = FileMap::open(path).with_context(|| format!("opening {}", path.display()))?;

    let pe =
        PeFile::from_bytes(&map).with_context(|| format!("parsing {} as PE", path.display()))?;

    let resources = pe
        .resources()
        .with_context(|| format!("no resource section in {}", path.display()))?;

    let mut strings: HashMap<u16, String> = HashMap::new();

    // Root directory: entries are resource types. Find RT_STRING (id=6).
    let root = resources.root()?;
    for type_entry in root.entries() {
        let type_name = type_entry.name()?;
        if type_name != Name::Id(RT_STRING_ID) {
            continue;
        }

        // Type directory: entries are bundle IDs (1-based resource IDs).
        let type_dir = match type_entry.entry()? {
            Entry::Directory(d) => d,
            Entry::DataEntry(_) => continue,
        };

        for bundle_entry in type_dir.entries() {
            let bundle_id: u32 = match bundle_entry.name()? {
                Name::Id(id) => id,
                _ => continue,
            };

            // String IDs for this bundle: (bundle_id - 1) * 16  through  (bundle_id - 1) * 16 + 15
            if bundle_id == 0 {
                continue; // malformed
            }
            let base_id = (bundle_id - 1) * 16;

            // Language directory: one or more language entries. Use the first.
            let lang_dir = match bundle_entry.entry()? {
                Entry::Directory(d) => d,
                Entry::DataEntry(_) => continue,
            };

            let Some(lang_entry) = lang_dir.entries().next() else {
                continue;
            };

            let data = match lang_entry.entry()? {
                Entry::DataEntry(d) => d,
                Entry::Directory(_) => continue,
            };

            let raw = data.bytes()?;
            parse_string_bundle(raw, base_id, &mut strings)?;
        }

        break; // found RT_STRING; no need to continue
    }

    Ok(strings)
}

/// Parse one `RT_STRING` bundle (raw bytes) and insert decoded strings into `out`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "Preserve the existing fixed-width DAT/resource encoding and its low-bit conversions."
)]
fn parse_string_bundle(
    raw: &[u8],
    base_id: u32,
    out: &mut HashMap<u16, String>,
) -> anyhow::Result<()> {
    let mut pos = 0usize;

    for slot in 0u32..16 {
        if pos + 2 > raw.len() {
            break;
        }
        let len = u16::from_le_bytes([raw[pos], raw[pos + 1]]) as usize;
        pos += 2;

        if len == 0 {
            continue; // empty slot
        }

        let byte_len = len * 2;
        if pos + byte_len > raw.len() {
            anyhow::bail!(
                "RT_STRING bundle truncated at slot {} (need {} bytes, have {})",
                base_id + slot,
                byte_len,
                raw.len().saturating_sub(pos)
            );
        }

        let utf16_bytes = &raw[pos..pos + byte_len];
        pos += byte_len;

        let code_units: Vec<u16> = utf16_bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();

        let s = String::from_utf16(&code_units)
            .with_context(|| format!("invalid UTF-16 in string slot {}", base_id + slot))?;

        out.insert((base_id + slot) as u16, s);
    }

    Ok(())
}

/// `RT_RCDATA` resource type ID.
const RT_RCDATA_ID: u32 = 10;

/// Load TEXTSTRA's `RT_RCDATA` message texts: the titles and bodies the
/// original's message classes fill (`FUN_0060b9d0`), keyed by resource id.
///
/// See [`decode_message_text`] for the stored form.
///
/// # Errors
/// Returns an error if the DLL cannot be read or its PE resource tree is
/// invalid.
pub fn load_message_texts(path: &Path) -> anyhow::Result<HashMap<u16, String>> {
    let map = FileMap::open(path).with_context(|| format!("opening {}", path.display()))?;
    let pe =
        PeFile::from_bytes(&map).with_context(|| format!("parsing {} as PE", path.display()))?;
    let resources = pe
        .resources()
        .with_context(|| format!("no resource section in {}", path.display()))?;

    let mut texts = HashMap::new();
    for type_entry in resources.root()?.entries() {
        if type_entry.name()? != Name::Id(RT_RCDATA_ID) {
            continue;
        }
        let Entry::Directory(type_dir) = type_entry.entry()? else {
            continue;
        };
        for record in type_dir.entries() {
            let Name::Id(id) = record.name()? else {
                continue;
            };
            let Ok(id) = u16::try_from(id) else {
                continue;
            };
            let Entry::Directory(languages) = record.entry()? else {
                continue;
            };
            let Some(language) = languages.entries().next() else {
                continue;
            };
            let Entry::DataEntry(data) = language.entry()? else {
                continue;
            };
            texts.insert(id, decode_message_text(data.bytes()?));
        }
    }
    Ok(texts)
}

/// Decode one message text record.
///
/// A record is Windows-1252 text ended by `0x01`. A substitution is `|`
/// followed by the parameter number (1-4), the value's kind (1 a name, 4 a
/// side's adjective, others rarer) and three zero bytes; it is kept as
/// `{parameter:kind}`. Trailing zero bytes before the end mark are dropped.
#[must_use]
pub fn decode_message_text(raw: &[u8]) -> String {
    let body = raw.strip_suffix(&[0x01]).unwrap_or(raw);
    let mut text = String::with_capacity(body.len());
    let mut pos = 0;
    while pos < body.len() {
        let byte = body[pos];
        if byte == b'|' && body.len() - pos >= 6 {
            let (parameter, kind) = (body[pos + 1], body[pos + 2]);
            text.push_str(&format!("{{{parameter}:{kind}}}"));
            pos += 6;
            continue;
        }
        text.push(windows_1252(byte));
        pos += 1;
    }
    text.trim_end_matches('\0').to_owned()
}

/// One Windows-1252 byte as a character. The 0x80-0x9f block differs from
/// Latin-1; unassigned bytes map to U+FFFD.
fn windows_1252(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{fffd}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{fffd}', 'Ž',
        '\u{fffd}', '\u{fffd}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ',
        '\u{fffd}', 'ž', 'Ÿ',
    ];
    match byte {
        0x80..=0x9f => HIGH[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

#[cfg(test)]
mod message_text_tests {
    use super::decode_message_text;

    #[test]
    fn a_message_record_keeps_its_substitutions_and_drops_its_end_mark() {
        // TEXTSTRA RT_RCDATA 0x7092 and 0x7180 shapes: `|` + parameter +
        // kind + three zero bytes, ended by 0x01.
        assert_eq!(
            decode_message_text(b"|\x01\x01\x00\x00\x00 Force Growth\x01"),
            "{1:1} Force Growth"
        );
        assert_eq!(
            decode_message_text(
                b"|\x02\x04\x00\x00\x00 Troops have defended |\x03\x01\x00\x00\x00.\x01"
            ),
            "{2:4} Troops have defended {3:1}."
        );
        assert_eq!(
            decode_message_text(b"Luke Goes to Dagobah\x01"),
            "Luke Goes to Dagobah"
        );
        assert_eq!(decode_message_text(b"Ends\x00\x01"), "Ends");
        // A substitution at the end keeps its own zero bytes (0x7156).
        assert_eq!(
            decode_message_text(b"Jabba Captures |\x01\x01\x00\x00\x00\x01"),
            "Jabba Captures {1:1}"
        );
        // A `|` too close to the end for a substitution stays text.
        assert_eq!(decode_message_text(b"50|50\x01"), "50|50");
        // Windows-1252 punctuation, not Latin-1 control codes.
        assert_eq!(decode_message_text(b"Jabba\x92s\x01"), "Jabba’s");
    }
}
