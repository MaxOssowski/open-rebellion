//! STRATEGY.DLL — the GID legends (`RT_RCDATA`).
//!
//! The galaxy view's legend window (`FUN_00452090`, painted by
//! `FUN_004522f0`) lays out one `RT_RCDATA` record per display through
//! `FUN_00452630`. A record is little-endian `u16` words:
//!
//! - the window's width and height, the module its bitmaps load from
//!   (`FUN_006037f0` index; 7 is STRATEGY.DLL) and the string module
//!   (the high word of each text's resource id);
//! - items, each opened by a kind word:
//!   - 1 or 2: a bitmap, `x, y, id`;
//!   - 3: a text, `x, y, string id, font, red, green, blue`; `y` 1 centres
//!     the text across the window's top;
//! - a 0 word ends the record.

#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use anyhow::bail;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::Context;
#[cfg(not(target_arch = "wasm32"))]
use pelite::resources::{Entry, Name};
#[cfg(not(target_arch = "wasm32"))]
use pelite::{FileMap, PeFile};
use serde::{Deserialize, Serialize};

/// `RT_RCDATA` resource type ID.
#[cfg(not(target_arch = "wasm32"))]
const RT_RCDATA_ID: u32 = 10;

/// The legend record of each display (`FUN_004522f0`'s switch), by the
/// display's command id. Popular Support has one per side, the Alliance's
/// first.
pub const LEGEND_RECORDS: [(u16, u16); 22] = [
    (0x11, 0x27c7),
    (0x11, 0x27c8),
    (0x12, 0x2d50),
    (0x21, 0x2d51),
    (0x22, 0x2d52),
    (0x43, 0x2d53),
    (0x44, 0x2d54),
    (0x51, 0x27cc),
    (0x52, 0x27cd),
    (0x53, 0x27ce),
    (0x54, 0x27cf),
    (0x62, 0x27d1),
    (0x63, 0x27d2),
    (0x64, 0x27d3),
    (0x65, 0x2d55),
    (0x66, 0x2d56),
    (0x67, 0x2d57),
    (0x71, 0x27d4),
    (0x72, 0x27d5),
    (0x73, 0x27d6),
    (0x74, 0x27d7),
    (0x75, 0x27d8),
];

/// One legend record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GidLegendRecord {
    pub width: u16,
    pub height: u16,
    pub bitmap_module: u16,
    pub string_module: u16,
    pub items: Vec<GidLegendItem>,
    /// Words after the end mark, kept for the round trip.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tail: Vec<u16>,
}

/// One item of a legend record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GidLegendItem {
    Bitmap {
        op: u16,
        x: u16,
        y: u16,
        id: u16,
    },
    Text {
        x: u16,
        y: u16,
        string: u16,
        font: u16,
        rgb: [u16; 3],
    },
}

/// Decode one legend record.
///
/// # Errors
/// Returns an error for a record shorter than its header or an item, or an
/// item kind other than 1..3.
pub fn decode(bytes: &[u8]) -> anyhow::Result<GidLegendRecord> {
    if bytes.len() % 2 != 0 {
        bail!("odd legend record length {}", bytes.len());
    }
    let words: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let [width, height, bitmap_module, string_module, ..] = words[..] else {
        bail!("legend record shorter than its header");
    };
    let mut items = Vec::new();
    let mut at = 4;
    loop {
        let Some(&op) = words.get(at) else {
            bail!("legend record without an end mark");
        };
        match op {
            0 => break,
            1 | 2 => {
                let Some(&[x, y, id]) = words.get(at + 1..at + 4) else {
                    bail!("truncated bitmap item at word {at}");
                };
                items.push(GidLegendItem::Bitmap { op, x, y, id });
                at += 4;
            }
            3 => {
                let Some(&[x, y, string, font, r, g, b]) = words.get(at + 1..at + 8) else {
                    bail!("truncated text item at word {at}");
                };
                items.push(GidLegendItem::Text {
                    x,
                    y,
                    string,
                    font,
                    rgb: [r, g, b],
                });
                at += 8;
            }
            other => bail!("unknown legend item kind {other} at word {at}"),
        }
    }
    Ok(GidLegendRecord {
        width,
        height,
        bitmap_module,
        string_module,
        items,
        tail: words[at + 1..].to_vec(),
    })
}

/// Encode a legend record back to its bytes.
#[must_use]
pub fn encode(record: &GidLegendRecord) -> Vec<u8> {
    let mut words = vec![
        record.width,
        record.height,
        record.bitmap_module,
        record.string_module,
    ];
    for item in &record.items {
        match *item {
            GidLegendItem::Bitmap { op, x, y, id } => words.extend([op, x, y, id]),
            GidLegendItem::Text {
                x,
                y,
                string,
                font,
                rgb,
            } => words.extend([3, x, y, string, font, rgb[0], rgb[1], rgb[2]]),
        }
    }
    words.push(0);
    words.extend(&record.tail);
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

/// Load the legend records ([`LEGEND_RECORDS`]) from STRATEGY.DLL, keyed by
/// resource id.
///
/// # Errors
/// Returns an error if the DLL cannot be read, its resource tree is
/// invalid, or a legend record does not decode.
#[cfg(not(target_arch = "wasm32"))]
pub fn load(path: &Path) -> anyhow::Result<HashMap<u16, GidLegendRecord>> {
    let map = FileMap::open(path).with_context(|| format!("opening {}", path.display()))?;
    let pe =
        PeFile::from_bytes(&map).with_context(|| format!("parsing {} as PE", path.display()))?;
    let resources = pe
        .resources()
        .with_context(|| format!("no resource section in {}", path.display()))?;
    let wanted: Vec<u16> = LEGEND_RECORDS.iter().map(|&(_, id)| id).collect();
    let mut records = HashMap::new();
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
            let Some(id) = u16::try_from(id).ok().filter(|id| wanted.contains(id)) else {
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
            let decoded =
                decode(data.bytes()?).with_context(|| format!("legend record {id:#x}"))?;
            records.insert(id, decoded);
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    }

    #[test]
    fn a_legend_record_decodes_its_bitmaps_and_texts_and_round_trips() {
        // FUN_00452630: width, height, modules, then kind 1/2 bitmaps (x, y,
        // id) and kind 3 texts (x, y, string, font, r, g, b) up to a 0.
        let raw = bytes(&[
            180, 135, 7, 2, 3, 21, 1, 0x1655, 4, 255, 255, 255, 2, 7, 20, 10181, 1, 0, 100, 10182,
            0, 0,
        ]);
        let record = decode(&raw).unwrap();
        assert_eq!((record.width, record.height), (180, 135));
        assert_eq!((record.bitmap_module, record.string_module), (7, 2));
        assert_eq!(
            record.items,
            [
                GidLegendItem::Text {
                    x: 21,
                    y: 1,
                    string: 0x1655,
                    font: 4,
                    rgb: [255, 255, 255]
                },
                GidLegendItem::Bitmap {
                    op: 2,
                    x: 7,
                    y: 20,
                    id: 10181
                },
                GidLegendItem::Bitmap {
                    op: 1,
                    x: 0,
                    y: 100,
                    id: 10182
                },
            ]
        );
        assert_eq!(record.tail, [0]);
        assert_eq!(encode(&record), raw);
    }

    #[test]
    fn a_malformed_legend_record_is_refused() {
        assert!(decode(&[1]).is_err());
        assert!(decode(&bytes(&[180, 135, 7])).is_err());
        assert!(decode(&bytes(&[180, 135, 7, 2, 2, 7, 20])).is_err());
        assert!(decode(&bytes(&[180, 135, 7, 2, 3, 21, 1, 5, 4, 255])).is_err());
        assert!(decode(&bytes(&[180, 135, 7, 2, 9, 0])).is_err());
        assert!(decode(&bytes(&[180, 135, 7, 2, 2, 7, 20, 10181])).is_err());
    }
}
