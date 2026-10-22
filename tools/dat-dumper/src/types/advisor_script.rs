//! GDATA `C3POACT.SPT` and `IMP22ACT.SPT` — the advisor's messages
//! (`ghidra/notes/advisor-briefing.md`).
//!
//! `FUN_004c2c70` (Alliance) and `FUN_004c0ba0` (Empire) read the file
//! with `FUN_0049c6f0`, each message through `FUN_004c52d0`. Little-endian:
//!
//! - `u32` message count;
//! - per message: `u32` id (`+0x18`), `u32` (`+0x1c`), `u32` entry count
//!   (`+0x20`), `u32` (`+0x24`);
//! - per entry, 14 bytes: `u16` RCDATA record, `u32` flags, `u32` a,
//!   `u32` b.
//!
//! `FUN_004c0a60` plays a due message's entries through `FUN_0042b1d0`:
//! the record comes from the advisor sprite DLL, or from the briefing DLL
//! when flag bit 1 is set; a and b replace the record's words 2 and 3
//! when not 0. The idle clip record (`[0x11, 0x18f]`) carries an action
//! that way, a kind and a code (`FUN_00439eb0`).

use crate::codec::{ByteReader, ByteWriter};
use crate::dat_record::DatRecord;
use serde::Serialize;

/// The Alliance briefing's message (C3POACT.SPT).
pub const ALLIANCE_BRIEFING: u32 = 0x29;
/// The Empire briefing's message (IMP22ACT.SPT).
pub const EMPIRE_BRIEFING: u32 = 0x4a;

/// One entry of a message: a record to play and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AdvisorEntry {
    /// The `RT_RCDATA` record.
    pub record: u16,
    /// Bit 0: played only with the advice flags `0x1000` without `0x4000`
    /// set (`DAT_006b28b0`); bit 1: the record is the briefing DLL's.
    pub flags: u32,
    /// The record's word 2 when not 0; an idle clip's action kind.
    pub a: u32,
    /// The record's word 3 when not 0; an idle clip's action code.
    pub b: u32,
}

impl AdvisorEntry {
    /// Flag bit 1: the record is read from the briefing DLL (slot `0xd`).
    #[must_use]
    pub const fn from_briefing(self) -> bool {
        self.flags & 2 != 0
    }
}

/// One advisor message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdvisorMessage {
    pub id: u32,
    /// `+0x1c`, 1 in every message of both files.
    pub field_1c: u32,
    /// `+0x24`, 0 in every message of both files.
    pub field_24: u32,
    pub entries: Vec<AdvisorEntry>,
}

/// A whole `.SPT` file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdvisorScriptFile {
    pub messages: Vec<AdvisorMessage>,
}

impl AdvisorScriptFile {
    /// The message with `id`.
    #[must_use]
    pub fn message(&self, id: u32) -> Option<&AdvisorMessage> {
        self.messages.iter().find(|message| message.id == id)
    }
}

impl DatRecord for AdvisorScriptFile {
    fn parse(r: &mut ByteReader) -> anyhow::Result<Self> {
        let count = r.read_u32()?;
        let mut messages = Vec::new();
        for _ in 0..count {
            let id = r.read_u32()?;
            let field_1c = r.read_u32()?;
            let entry_count = r.read_u32()?;
            let field_24 = r.read_u32()?;
            let mut entries = Vec::new();
            for _ in 0..entry_count {
                entries.push(AdvisorEntry {
                    record: r.read_u16()?,
                    flags: r.read_u32()?,
                    a: r.read_u32()?,
                    b: r.read_u32()?,
                });
            }
            messages.push(AdvisorMessage {
                id,
                field_1c,
                field_24,
                entries,
            });
        }
        Ok(Self { messages })
    }

    fn write_bytes(&self, w: &mut ByteWriter) {
        w.write_u32(u32::try_from(self.messages.len()).unwrap_or(u32::MAX));
        for message in &self.messages {
            w.write_u32(message.id);
            w.write_u32(message.field_1c);
            w.write_u32(u32::try_from(message.entries.len()).unwrap_or(u32::MAX));
            w.write_u32(message.field_24);
            for entry in &message.entries {
                w.write_u16(entry.record);
                w.write_u32(entry.flags);
                w.write_u32(entry.a);
                w.write_u32(entry.b);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le(words: &[(u32, usize)]) -> Vec<u8> {
        words
            .iter()
            .flat_map(|&(value, size)| value.to_le_bytes()[..size].to_vec())
            .collect()
    }

    #[test]
    fn a_message_list_decodes_its_entries_and_round_trips() {
        // FUN_004c52d0: id, +0x1c, entry count, +0x24, then each entry's
        // u16 record and three u32 (FUN_005f4dd0, FUN_005f4d90).
        let bytes = le(&[
            (2, 4),
            (0x29, 4),
            (1, 4),
            (2, 4),
            (0, 4),
            (0x2710, 2),
            (0, 4),
            (2, 4),
            (12, 4),
            (0x27ab, 2),
            (2, 4),
            (0, 4),
            (0, 4),
            (1, 4),
            (1, 4),
            (0, 4),
            (0, 4),
        ]);
        let mut reader = ByteReader::new(&bytes);
        let file = AdvisorScriptFile::parse(&mut reader).unwrap();
        reader.assert_exhausted("test").unwrap();

        let briefing = file.message(ALLIANCE_BRIEFING).unwrap();
        assert_eq!(
            briefing.entries,
            [
                AdvisorEntry {
                    record: 0x2710,
                    flags: 0,
                    a: 2,
                    b: 12
                },
                AdvisorEntry {
                    record: 0x27ab,
                    flags: 2,
                    a: 0,
                    b: 0
                },
            ]
        );
        assert!(!briefing.entries[0].from_briefing());
        assert!(briefing.entries[1].from_briefing());
        assert!(file.message(1).unwrap().entries.is_empty());
        assert!(file.message(0x4a).is_none());

        let mut writer = ByteWriter::new();
        file.write_bytes(&mut writer);
        assert_eq!(writer.into_bytes(), bytes);
    }

    #[test]
    fn a_truncated_entry_is_an_error() {
        let bytes = le(&[(1, 4), (1, 4), (1, 4), (1, 4), (0, 4), (0x2710, 2)]);
        assert!(AdvisorScriptFile::parse(&mut ByteReader::new(&bytes)).is_err());
    }
}
