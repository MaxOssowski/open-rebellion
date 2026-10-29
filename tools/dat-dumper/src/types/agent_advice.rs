//! TEXTSTRA.DLL — the agent's advice list (`RT_RCDATA`).
//!
//! `FUN_0048b460` builds the agent's advice list (`+0x154`) from TEXTSTRA:
//! record `0x6000` (Alliance) or `0x6800` (Empire) holds the count, a `u16`;
//! advice `n` (1-based) is record `base + 3n`, with its title and body the
//! message texts `base + 3n + 1` and `+ 2` (`FUN_0048b2e0`). An advice
//! record is four bytes:
//!
//! - byte 0: its kind in the low nibble (the list node's `+0x1c` keeps
//!   `n << 4 | kind`);
//! - byte 1: unused;
//! - bytes 2..4: a `u16`, the node's key (`FUN_005f5600`), which orders
//!   the list.

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use anyhow::Context;
use anyhow::bail;
#[cfg(not(target_arch = "wasm32"))]
use pelite::resources::{Entry, Name};
#[cfg(not(target_arch = "wasm32"))]
use pelite::{FileMap, PeFile};
use serde::{Deserialize, Serialize};

/// `RT_RCDATA` resource type ID.
#[cfg(not(target_arch = "wasm32"))]
const RT_RCDATA_ID: u32 = 10;

/// The count record of each side's list, Alliance then Empire
/// (`FUN_0048b460`: `0x6000`, plus `0x800` for a side other than 1).
pub const COUNT_RECORDS: [u16; 2] = [0x6000, 0x6800];

/// One advice record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentAdviceRecord {
    /// The advice's number `n`, 1-based: the record is `base + 3n`.
    pub number: u16,
    /// Byte 0; its low nibble is the advice's kind.
    pub kind_byte: u8,
    /// Byte 1, kept for the round trip.
    pub pad: u8,
    /// The list key.
    pub key: u16,
}

impl AgentAdviceRecord {
    /// The kind, byte 0's low nibble (`FUN_0048b460`: `*record & 0xf`).
    #[must_use]
    pub const fn kind(self) -> u8 {
        self.kind_byte & 0xf
    }
}

/// Both sides' advice records, in record order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentAdviceTables {
    pub alliance: Vec<AgentAdviceRecord>,
    pub empire: Vec<AgentAdviceRecord>,
}

/// Decode a count record.
///
/// # Errors
/// Returns an error for a record shorter than two bytes.
pub fn decode_count(bytes: &[u8]) -> anyhow::Result<u16> {
    let Some(&[low, high]) = bytes.get(..2) else {
        bail!("advice count record shorter than two bytes");
    };
    Ok(u16::from_le_bytes([low, high]))
}

/// Decode advice `number`'s record.
///
/// # Errors
/// Returns an error for a record that is not four bytes.
pub fn decode(number: u16, bytes: &[u8]) -> anyhow::Result<AgentAdviceRecord> {
    let &[kind_byte, pad, low, high] = bytes else {
        bail!("advice record {number} is {} bytes, not 4", bytes.len());
    };
    Ok(AgentAdviceRecord {
        number,
        kind_byte,
        pad,
        key: u16::from_le_bytes([low, high]),
    })
}

/// Encode an advice record back to its bytes.
#[must_use]
pub fn encode(record: AgentAdviceRecord) -> [u8; 4] {
    let [low, high] = record.key.to_le_bytes();
    [record.kind_byte, record.pad, low, high]
}

/// Decode one side's list from its records: the count at `base`, then
/// advice `n` at `base + 3n`. A missing record is left out, as
/// `FUN_0048b460` skips a resource that does not load.
///
/// # Errors
/// Returns an error for a count or advice record that does not decode.
pub fn decode_side<'a>(
    base: u16,
    record: impl Fn(u16) -> Option<&'a [u8]>,
) -> anyhow::Result<Vec<AgentAdviceRecord>> {
    let Some(count) = record(base) else {
        return Ok(Vec::new());
    };
    let count = decode_count(count)?;
    let mut records = Vec::new();
    for number in 1..=count {
        let Some(bytes) = base.checked_add(number.saturating_mul(3)).and_then(&record) else {
            continue;
        };
        records.push(decode(number, bytes)?);
    }
    Ok(records)
}

/// Load both sides' advice records from TEXTSTRA.DLL.
///
/// # Errors
/// Returns an error if the DLL cannot be read, its resource tree is
/// invalid, or a record does not decode.
#[cfg(not(target_arch = "wasm32"))]
pub fn load(path: &Path) -> anyhow::Result<AgentAdviceTables> {
    let map = FileMap::open(path).with_context(|| format!("opening {}", path.display()))?;
    let pe =
        PeFile::from_bytes(&map).with_context(|| format!("parsing {} as PE", path.display()))?;
    let resources = pe
        .resources()
        .with_context(|| format!("no resource section in {}", path.display()))?;
    let mut raw = std::collections::HashMap::new();
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
            let Some(id) = u16::try_from(id)
                .ok()
                .filter(|id| (0x6000..0x7000).contains(id))
            else {
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
            raw.insert(id, data.bytes()?);
        }
    }
    let side = |base| decode_side(base, |id| raw.get(&id).copied());
    Ok(AgentAdviceTables {
        alliance: side(COUNT_RECORDS[0])?,
        empire: side(COUNT_RECORDS[1])?,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn an_advice_record_decodes_its_kind_and_key_and_round_trips() {
        // TEXTSTRA RT_RCDATA 0x605d: 05 00 36 01, kind 5 with key 310
        // (FUN_0048b460: *record & 0xf, the u16 at +2).
        let raw = [0x05, 0x00, 0x36, 0x01];
        let record = decode(31, &raw).unwrap();

        assert_eq!(record.kind(), 5);
        assert_eq!(record.key, 310);
        assert_eq!(encode(record), raw);
        assert!(decode(1, &raw[..3]).is_err());
    }

    #[test]
    fn a_side_reads_its_count_then_every_third_record_skipping_a_missing_one() {
        // FUN_0048b460: the count at the base, advice n at base + 3n; a
        // resource that does not load is skipped.
        let records: HashMap<u16, Vec<u8>> = [
            (0x6000, vec![3, 0]),
            (0x6003, vec![7, 0, 10, 0]),
            (0x6009, vec![0x12, 0, 0x04, 0x01]),
        ]
        .into_iter()
        .collect();

        let side = decode_side(0x6000, |id| records.get(&id).map(Vec::as_slice)).unwrap();

        assert_eq!(
            side,
            [
                AgentAdviceRecord {
                    number: 1,
                    kind_byte: 7,
                    pad: 0,
                    key: 10
                },
                AgentAdviceRecord {
                    number: 3,
                    kind_byte: 0x12,
                    pad: 0,
                    key: 260
                },
            ]
        );
        assert_eq!(side[1].kind(), 2);
        assert!(
            decode_side(0x6800, |id| records.get(&id).map(Vec::as_slice))
                .unwrap()
                .is_empty()
        );
    }
}
