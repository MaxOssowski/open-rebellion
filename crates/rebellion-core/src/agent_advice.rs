//! The agent's advice (`ghidra/notes/agent-advice-list.md`).
//!
//! With Agent Advice on (`DAT_006b28b0` bit `0x8000` clear), the agent
//! posts the droid's advice to the Message Index's Advice category, one
//! message per advice (`FUN_0048a590` → `FUN_0048b2e0`). The advice comes
//! from TEXTSTRA (`FUN_0048b460`): each has a kind and a key that orders
//! the agent's list (`+0x154`). The list is built once (`FUN_00439f20`),
//! which posts every kind-7 advice at once. Opening a kind of window for
//! the first time posts the first advice of its kind (`FUN_0043a0b0`), and
//! every 300 sub-ticks the first advice whose window has been seen is
//! posted (`FUN_00439bc0`, `FUN_00439fb0`). A posted advice leaves the
//! list.
//!
//! Times are the scheduler's sub-ticks (`FUN_004fcee0`,
//! [`crate::tick::GameClock::sub_tick`]): a day has 600 at Very Slow, 60
//! Slow, 12 Medium and 4 Fast, so 300 are half a day to 75 days.

use serde::{Deserialize, Serialize};

/// One advice record from TEXTSTRA (`FUN_0048b460`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdviceRecord {
    /// The advice's number `n`: its title and body are TEXTSTRA message
    /// texts `base + 3n + 1` and `+ 2` (`FUN_0048b2e0`).
    pub number: u16,
    /// The low nibble of the record's first byte.
    pub kind: u8,
    /// The list key (`FUN_005f5600`).
    pub key: u16,
}

/// One advice on the agent's list: its number and kind (the node's
/// `+0x1c`, `number << 4 | kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdviceEntry {
    pub number: u16,
    pub kind: u8,
}

/// A window whose first opening posts advice (`FUN_0041d2d0`'s argument).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdviceWindow {
    /// A sector window (`FUN_00429ce0`).
    Sector,
    /// A system's Manufacturing window (`FUN_0045aac0` kind 4).
    System,
    /// A Fleet window (`FUN_0045aac0` kind `0x10`).
    Fleet,
    /// A System Defenses window (`FUN_0045aac0` kind 8).
    Defenses,
    /// A Missions window (`FUN_0045aac0` kind `0x40`).
    Missions,
}

impl AdviceWindow {
    /// The window's bit in `+0x150`, as its opener passes it to
    /// `FUN_0041d2d0`: `FUN_00429ce0` 1, `FUN_0045aac0` 2 (kind 4), 4 (kind
    /// `0x10`), 8 (kind 8) and `0x10` (kind `0x40`).
    #[must_use]
    pub const fn flag(self) -> u32 {
        match self {
            Self::Sector => 1,
            Self::System => 2,
            Self::Fleet => 4,
            Self::Defenses => 8,
            Self::Missions => 0x10,
        }
    }

    /// The advice kind its first opening posts (`FUN_0043a0b0`'s switch).
    #[must_use]
    pub const fn kind(self) -> u8 {
        match self {
            Self::Sector => 6,
            Self::System => 2,
            Self::Fleet => 3,
            Self::Defenses => 5,
            Self::Missions => 4,
        }
    }
}

/// `+0x150` bit `0x10000000`: the list is built (`FUN_00439f20`).
const LIST_BUILT: u32 = 0x1000_0000;

/// `FUN_00439bc0`: the next advice comes once `+0x148` plus 300 sub-ticks
/// have passed.
pub const ADVICE_PERIOD: u64 = 300;

/// The agent's advice state, all saved with the agent (`FUN_004397a0`:
/// `+0x148`, `+0x150`, `+0x154`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentAdvice {
    /// `+0x148`: the sub-tick of the last advice. `FUN_00439320` sets 0.
    last: u64,
    /// `+0x150`: the windows seen ([`AdviceWindow::flag`]) and
    /// [`LIST_BUILT`].
    flags: u32,
    /// `+0x154`: the advice not yet posted, in key order.
    entries: Vec<AdviceEntry>,
}

impl AgentAdvice {
    /// The advice not yet posted, in list order.
    #[must_use]
    pub fn entries(&self) -> &[AdviceEntry] {
        &self.entries
    }

    /// Build the list once and post every kind-7 advice (`FUN_00439f20`).
    /// The list is marked built only when TEXTSTRA held advice
    /// (`FUN_0048b460` returns 0 for a count of 0), so it is tried again.
    pub fn build(&mut self, records: &[AdviceRecord]) -> Vec<AdviceEntry> {
        if self.flags & LIST_BUILT != 0 {
            return Vec::new();
        }
        if !records.is_empty() {
            self.flags |= LIST_BUILT;
        }
        // FUN_005f4f10 keeps the list in key order. hyp: the list's
        // comparator (vtable +8) orders keys ascending.
        let mut sorted: Vec<&AdviceRecord> = records.iter().collect();
        sorted.sort_by_key(|record| record.key);
        self.entries
            .extend(sorted.into_iter().map(|record| AdviceEntry {
                number: record.number,
                kind: record.kind,
            }));
        let (posted, kept) = self.entries.iter().partition(|entry| entry.kind == 7);
        self.entries = kept;
        posted
    }

    /// A window opened for the first time (`FUN_0043a0b0`): build the list
    /// if needed, then post the first advice of the window's kind, and
    /// restart the period when one is posted.
    pub fn window_opened(
        &mut self,
        window: AdviceWindow,
        now: u64,
        records: &[AdviceRecord],
    ) -> Vec<AdviceEntry> {
        let mut posted = self.build(records);
        if self.flags & window.flag() != 0 {
            return posted;
        }
        self.flags |= window.flag();
        if let Some(at) = self
            .entries
            .iter()
            .position(|entry| entry.kind == window.kind())
        {
            posted.push(self.entries.remove(at));
            self.last = now;
        }
        posted
    }

    /// The periodic check (`FUN_00439bc0`): once the period has passed,
    /// post the first advice whose window has been seen, or any advice of
    /// another kind (`FUN_00439fb0`), and restart the period. The list
    /// is read before it is built, so the call that builds it posts no
    /// more than the kind-7 advice.
    pub fn check(&mut self, now: u64, records: &[AdviceRecord]) -> Vec<AdviceEntry> {
        if self.last.saturating_add(ADVICE_PERIOD) > now {
            return Vec::new();
        }
        let was_built = self.flags & LIST_BUILT != 0;
        let mut posted = self.build(records);
        if was_built {
            let flags = self.flags;
            let seen = |entry: &AdviceEntry| {
                let window = match entry.kind {
                    2 => AdviceWindow::System,
                    3 => AdviceWindow::Fleet,
                    4 => AdviceWindow::Missions,
                    5 => AdviceWindow::Defenses,
                    6 => AdviceWindow::Sector,
                    _ => return true,
                };
                flags & window.flag() != 0
            };
            if let Some(at) = self.entries.iter().position(seen) {
                posted.push(self.entries.remove(at));
            }
        }
        self.last = now;
        posted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TEXTSTRA's Alliance list in record order (RT_RCDATA 0x6003..0x605d,
    /// kind and key), as FUN_0048b460 reads it.
    fn alliance_records() -> Vec<AdviceRecord> {
        let raw: [(u8, u16); 31] = [
            (7, 10),
            (7, 20),
            (7, 30),
            (7, 40),
            (7, 50),
            (7, 60),
            (7, 70),
            (7, 80),
            (7, 90),
            (1, 140),
            (1, 150),
            (1, 160),
            (1, 170),
            (1, 180),
            (1, 190),
            (6, 100),
            (6, 230),
            (3, 120),
            (3, 220),
            (3, 260),
            (3, 270),
            (3, 280),
            (2, 110),
            (2, 210),
            (2, 240),
            (2, 250),
            (5, 130),
            (5, 200),
            (5, 290),
            (5, 300),
            (5, 310),
        ];
        raw.iter()
            .zip(1..)
            .map(|(&(kind, key), number)| AdviceRecord { number, kind, key })
            .collect()
    }

    fn numbers(entries: &[AdviceEntry]) -> Vec<u16> {
        entries.iter().map(|entry| entry.number).collect()
    }

    #[test]
    fn building_the_list_posts_every_kind_seven_advice_once() {
        // FUN_00439f20: built once (+0x150 bit 0x10000000), then every
        // node of kind 7 is posted and removed.
        let records = alliance_records();
        let mut advice = AgentAdvice::default();

        assert_eq!(
            numbers(&advice.build(&records)),
            (1..=9).collect::<Vec<_>>()
        );
        assert_eq!(advice.entries().len(), 22);
        assert!(advice.build(&records).is_empty());
        // Key order: 100 (16), 110 (23), 120 (18), 130 (27), 140 (10).
        assert_eq!(numbers(&advice.entries()[..5]), [16, 23, 18, 27, 10]);
    }

    #[test]
    fn a_list_without_advice_is_not_marked_built() {
        // FUN_0048b460 returns 0 for a count of 0, so the bit stays clear.
        let mut advice = AgentAdvice::default();
        assert!(advice.build(&[]).is_empty());
        assert_eq!(numbers(&advice.build(&alliance_records())).len(), 9);
    }

    #[test]
    fn a_windows_first_opening_posts_the_first_advice_of_its_kind() {
        // FUN_0043a0b0: flag 4 (a Fleet window) maps to kind 3; the first
        // such node in key order is 18 (key 120). A second opening posts
        // nothing.
        let records = alliance_records();
        let mut advice = AgentAdvice::default();

        let posted = advice.window_opened(AdviceWindow::Fleet, 12, &records);

        assert_eq!(numbers(&posted), [1, 2, 3, 4, 5, 6, 7, 8, 9, 18]);
        assert_eq!(advice.last, 12);
        assert!(
            advice
                .window_opened(AdviceWindow::Fleet, 13, &records)
                .is_empty()
        );
        assert_eq!(advice.last, 12);
        // Sector (1) is kind 6: 16; System (2) kind 2: 23; Defenses (8)
        // kind 5: 27; Missions (0x10) kind 4: none.
        assert_eq!(
            numbers(&advice.window_opened(AdviceWindow::Sector, 14, &records)),
            [16]
        );
        assert_eq!(
            numbers(&advice.window_opened(AdviceWindow::System, 15, &records)),
            [23]
        );
        assert_eq!(
            numbers(&advice.window_opened(AdviceWindow::Defenses, 16, &records)),
            [27]
        );
        assert!(
            advice
                .window_opened(AdviceWindow::Missions, 17, &records)
                .is_empty()
        );
        assert_eq!(advice.last, 16);
    }

    #[test]
    fn every_three_hundred_sub_ticks_the_first_advice_of_a_seen_window_is_posted() {
        // FUN_00439bc0: +0x148 + 300 <= the sub-tick (FUN_004fcee0).
        // FUN_00439fb0 posts the first node whose kind is seen (2 System,
        // 3 Fleet, 4 Missions, 5 Defenses, 6 Sector) or of another kind,
        // then stores the sub-tick.
        let records = alliance_records();
        let mut advice = AgentAdvice::default();

        assert!(advice.check(299, &records).is_empty());
        // The first check builds the list but reads it before.
        assert_eq!(
            numbers(&advice.check(300, &records)),
            (1..=9).collect::<Vec<_>>()
        );
        assert_eq!(advice.last, 300);
        assert!(advice.check(599, &records).is_empty());
        // No window seen: 16 (kind 6), 23, 18, 27 are skipped for 10.
        assert_eq!(numbers(&advice.check(600, &records)), [10]);
        advice.window_opened(AdviceWindow::Sector, 650, &records);
        // The opening posted 16 and restarted the period at 650.
        assert!(advice.check(900, &records).is_empty());
        assert_eq!(numbers(&advice.check(950, &records)), [11]);
    }

    #[test]
    fn a_missions_window_makes_kind_four_advice_due() {
        // FUN_00439fb0 case 4: +0x150 bit 0x10 (FUN_0045aac0 kind 0x40).
        let records = [
            AdviceRecord {
                number: 1,
                kind: 4,
                key: 10,
            },
            AdviceRecord {
                number: 2,
                kind: 1,
                key: 20,
            },
        ];
        let mut advice = AgentAdvice::default();
        advice.check(300, &records);
        assert_eq!(numbers(&advice.check(600, &records)), [2]);
        let mut seen = AgentAdvice::default();
        assert_eq!(
            numbers(&seen.window_opened(AdviceWindow::Missions, 0, &records)),
            [1]
        );
        let mut later = AgentAdvice::default();
        later.build(&records);
        later.flags |= AdviceWindow::Missions.flag();
        assert_eq!(numbers(&later.check(300, &records)), [1]);
    }

    #[test]
    fn the_period_restarts_even_when_nothing_is_left() {
        let mut advice = AgentAdvice::default();
        assert!(advice.check(300, &[]).is_empty());
        assert_eq!(advice.last, 300);
    }
}
