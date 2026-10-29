//! The advisor's message schedule and the opening briefing
//! (`ghidra/notes/advisor-briefing.md`).
//!
//! The side's advisor (`Class_0065c4c8`, `FUN_004c27f0`, for the Alliance;
//! `Class_0065c4a0`, `FUN_004c0710`, for the Empire) keeps, per message
//! id, the last day it may play (`+0x168`) and its next free day
//! (`+0x16c`). A new advisor schedules its side's briefing for 40000 days:
//! message `0x29` (C3POACT.SPT) or `0x4a` (IMP22ACT.SPT).
//!
//! The agent droid's idle clip ends after 15 ticks (`FUN_0042adb0`,
//! `FUN_00472b20(0x12, .., 0xf)`) and calls `FUN_0041cea0`, which enables
//! the advisor (`DAT_006b28bc`, `FUN_00439ef0`). The advisor's update
//! (`FUN_004c0a60`) then plays the first due message: each entry's record
//! goes to the agent droid's player (`FUN_0041d660` -> `FUN_0042b1d0`).
//!
//! A left or right press, or Escape, while the briefing holds the input
//! (`FUN_0041d9d0`'s hooks) asks to skip (`0x2000`, `FUN_0043a200`): the
//! players are flushed (`FUN_0041da30` -> `FUN_0042d620`) and the short
//! form, `0x28` / `0x49`, is scheduled instead (`FUN_004c0d00`,
//! `FUN_004c2dd0`).

use std::collections::HashMap;

use egui_macroquad::egui;

use crate::advisor::AdvisorFaction;
use crate::advisor_clip::{
    ClipAction, ClipPlayer, BRIEFING_SLOT, CLIP_TICK_SECONDS, SPRITE_SLOT,
};
use crate::cockpit::GidMode;

/// The Alliance's opening briefing (C3POACT.SPT).
pub const ALLIANCE_BRIEFING: u32 = 0x29;
/// The Alliance briefing's short form, after a skip.
pub const ALLIANCE_SHORT_BRIEFING: u32 = 0x28;
/// The Empire's opening briefing (IMP22ACT.SPT).
pub const EMPIRE_BRIEFING: u32 = 0x4a;
/// The Empire briefing's short form, after a skip.
pub const EMPIRE_SHORT_BRIEFING: u32 = 0x49;

/// A schedule entry's "plays for 40000 days" (`FUN_004c27f0`).
const ALWAYS: u32 = 40_000;
/// A played message's next free day is 60 days on (`FUN_004c0a60`).
const REST_DAYS: u32 = 0x3c;
/// The idle clip's length in ticks (`FUN_00472b20(0x12, .., 0xf)`).
const IDLE_TICKS: u32 = 15;

/// One entry of a message (`FUN_004c52d0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptEntry {
    /// The RCDATA record: `[op, clip record, a, b]`.
    pub record: u16,
    /// Bit 0: played only with advice on; bit 1: the record is the
    /// briefing DLL's.
    pub flags: u32,
    /// The clip's action kind when not 0.
    pub a: u32,
    /// The clip's action code when not 0.
    pub b: u32,
}

/// The side's message count (`+0x164`): `0x58` for the Alliance, `0x56`
/// for the Empire.
const fn message_count(faction: AdvisorFaction) -> usize {
    match faction {
        AdvisorFaction::Alliance => 0x58,
        AdvisorFaction::Empire => 0x56,
    }
}

const fn briefings(faction: AdvisorFaction) -> (u32, u32) {
    match faction {
        AdvisorFaction::Alliance => (ALLIANCE_BRIEFING, ALLIANCE_SHORT_BRIEFING),
        AdvisorFaction::Empire => (EMPIRE_BRIEFING, EMPIRE_SHORT_BRIEFING),
    }
}

/// The side's advisor: its messages, its schedule and the briefing's
/// input hold.
#[derive(Debug)]
pub struct AdvisorSchedule {
    faction: AdvisorFaction,
    messages: HashMap<u32, Vec<ScriptEntry>>,
    last_day: Vec<u32>,
    next_free: Vec<u32>,
    /// `DAT_006b28bc`: the idle clip has ended since the last update.
    enabled: bool,
    /// Panel time the agent droid has idled, in ticks' seconds.
    idle_seconds: f32,
    /// `0x2000`: a skip was asked for.
    skip_requested: bool,
    /// `DAT_006b14b4`: the players were flushed by a skip.
    flushed: bool,
    /// `FUN_0041d9d0` .. `FUN_0041da80`: the briefing holds the input.
    holding_input: bool,
    /// `DAT_006b28b0` bit `0x1000`, Translate Counterpart: entries with flag
    /// bit 0 play only while it is set (`FUN_004c0a60`).
    translate: bool,
    /// The reaction bits of `DAT_006b28b0` not yet answered
    /// ([`Self::add_flags`]).
    flags: u32,
    /// `DAT_006b28b8`: the Agent menu's openings since the last answer.
    menu_openings: u32,
    /// `DAT_006b28d0`: the day the advisor last played a message.
    last_played: u32,
    /// The quip's roll (`FUN_005f5700(12)`). hyp: the original's generator
    /// is the interface's; a small xorshift stands in.
    roll: u32,
    /// `+0x174`/`+0x178`: the last refused order's status, and `+0x17c`
    /// whether it was refused twice running.
    last_refusal: Option<(u32, u32)>,
    refusal_repeated: bool,
}

/// The reaction bits of `DAT_006b28b0` the advisor answers
/// (`FUN_004c2dd0`, `FUN_004c0d00`).
pub mod reaction {
    /// `FUN_0043a2a0`: a ship move the agent ordered arrived.
    pub const MOVE_DONE: u32 = 0x1;
    /// `FUN_00439d60`: Manage Garrisons on, Production on, Garrisons off,
    /// Production off, and the toggle counter past 3.
    pub const GARRISONS_ON: u32 = 0x10;
    pub const PRODUCTION_ON: u32 = 0x20;
    pub const GARRISONS_OFF: u32 = 0x100;
    pub const PRODUCTION_OFF: u32 = 0x200;
    pub const TOO_MANY_TOGGLES: u32 = 0x100_0000;
    /// `FUN_00439ed0`: the Agent menu opened (`FUN_0041d160` →
    /// `FUN_00436640` → `FUN_00487c10`).
    pub const MENU: u32 = 0x200_0000;
    /// `FUN_00439ef0`: the idle clip ended after 50 quiet days.
    pub const QUIP: u32 = 0x8000_0000;
}

/// One side's answer to a reaction bit: the message and how many days it
/// stays due.
struct Reaction {
    bit: u32,
    message: u32,
    days: u32,
}

/// The message an event code schedules and for how many days: the cases
/// of `FUN_004c44b0` (Alliance) and `FUN_004c22f0` (Empire), each writing
/// `+0x168[message] = today + days`.
const fn event_message(faction: AdvisorFaction, code: u32) -> Option<(u32, u32)> {
    Some(match faction {
        AdvisorFaction::Alliance => match code {
            1 => (0x34, 10),
            2 => (0x35, 10),
            3 => (0x38, 5),
            4 => (0x3a, 10),
            5 => (0x3b, 10),
            6 => (0x3d, 10),
            8 => (0x3c, 10),
            9 => (0x3e, 10),
            0xc => (0x39, 10),
            0xd => (0x41, 10),
            0xe => (0x43, 10),
            0x14 => (0x2a, 10),
            0x15 => (0x42, 10),
            0x16 => (0x2d, ALWAYS),
            0x17 => (0x2b, ALWAYS),
            0x18 => (0x2c, ALWAYS),
            0x19 => (0x2e, ALWAYS),
            0x1a | 0x1b => (0x2a, ALWAYS),
            0x1c => (0x37, 10),
            0x1e => (0x1c, ALWAYS),
            0x1f => (0x1b, ALWAYS),
            0x20 => (0x1d, ALWAYS),
            0x21 => (0x1e, ALWAYS),
            0x22 => (0x20, ALWAYS),
            0x23 => (0x1f, ALWAYS),
            0x24 => (0x33, ALWAYS),
            0x25 => (0x30, ALWAYS),
            0x26 => (0x2f, ALWAYS),
            0x27 => (0x32, ALWAYS),
            0x28 => (0x31, ALWAYS),
            0x29 => (0x44, 0x32),
            0x2c => (0x4b, 10),
            0x2e => (0x19, 10),
            0x2f => (0x1a, 10),
            _ => return None,
        },
        AdvisorFaction::Empire => match code {
            1 => (0x30, 10),
            2 => (0x31, 10),
            3 => (0x34, 10),
            4 => (0x36, 10),
            5 => (0x37, 10),
            6 => (0x39, 10),
            8 => (0x38, 10),
            9 => (0x3a, 10),
            0xc => (0x35, 10),
            0xd => (0x3d, 10),
            0xe => (0x3e, 5),
            0x14 => (0x40, 10),
            0x15 => (0x3f, 10),
            0x16..=0x19 => (0x40, ALWAYS),
            0x1a => (0x22, ALWAYS),
            0x1b => (0x21, ALWAYS),
            0x1c => (0x33, ALWAYS),
            0x1e => (0x2a, ALWAYS),
            0x1f => (0x29, ALWAYS),
            0x20 => (0x2b, ALWAYS),
            0x21 => (0x2c, ALWAYS),
            0x22 => (0x2e, ALWAYS),
            0x23 => (0x2d, ALWAYS),
            0x24 => (0x2f, ALWAYS),
            0x25 => (0x26, ALWAYS),
            0x26 => (0x25, ALWAYS),
            0x27 => (0x28, ALWAYS),
            0x28 => (0x27, ALWAYS),
            0x29 => (0x41, 0x32),
            0x2d => (0x48, 10),
            0x2e => (0x23, 10),
            0x2f => (0x24, 10),
            _ => return None,
        },
    })
}

/// `FUN_004c2dd0` (Alliance, C3POACT) and `FUN_004c0d00` (Empire,
/// IMP22ACT), in their order: one bit answered a call. The menu's and the
/// quip's messages are picked apart ([`AdvisorSchedule::react`]).
const fn reactions(faction: AdvisorFaction) -> &'static [Reaction] {
    use reaction::{
        GARRISONS_OFF, GARRISONS_ON, MOVE_DONE, PRODUCTION_OFF, PRODUCTION_ON, TOO_MANY_TOGGLES,
    };
    match faction {
        AdvisorFaction::Alliance => &[
            Reaction { bit: MOVE_DONE, message: 0x23, days: 10 },
            Reaction { bit: GARRISONS_ON, message: 0x24, days: 10 },
            Reaction { bit: PRODUCTION_ON, message: 0x26, days: 10 },
            Reaction { bit: GARRISONS_OFF, message: 0x25, days: 0x32 },
            Reaction { bit: PRODUCTION_OFF, message: 0x27, days: 0x32 },
            Reaction { bit: TOO_MANY_TOGGLES, message: 0x4f, days: 5 },
        ],
        AdvisorFaction::Empire => &[
            Reaction { bit: MOVE_DONE, message: 0x1c, days: 10 },
            Reaction { bit: GARRISONS_ON, message: 0x1d, days: 10 },
            Reaction { bit: PRODUCTION_ON, message: 0x1f, days: 10 },
            Reaction { bit: GARRISONS_OFF, message: 0x1e, days: 0x32 },
            Reaction { bit: PRODUCTION_OFF, message: 0x20, days: 0x32 },
        ],
    }
}

impl AdvisorSchedule {
    /// A new game's advisor (`FUN_004c27f0`, `FUN_004c0710`): its side's
    /// briefing plays for 40000 days.
    #[must_use]
    pub fn new_game(faction: AdvisorFaction, messages: HashMap<u32, Vec<ScriptEntry>>) -> Self {
        let count = message_count(faction);
        let mut last_day = vec![0; count];
        let (briefing, _) = briefings(faction);
        last_day[briefing as usize] = ALWAYS;
        Self {
            faction,
            messages,
            last_day,
            next_free: vec![0; count],
            enabled: false,
            idle_seconds: 0.0,
            skip_requested: false,
            flushed: false,
            holding_input: false,
            translate: true,
            flags: 0,
            menu_openings: 0,
            last_played: 0,
            roll: 0x2545_f491,
            last_refusal: None,
            refusal_repeated: false,
        }
    }

    /// An order the side's validator refused (`FUN_00487c90` → slot `+0xc`,
    /// `FUN_004c2940` Alliance, `FUN_004c0870` Empire): `status` is its two
    /// words. The same status as the last one schedules the first of two
    /// messages, and once more the second, then forgets it
    /// (`FUN_004c43d0`, `FUN_004c21d0`). Otherwise a team left empty with
    /// decoys and no target (`0x40`/`0x91`, `decoys_without_target`) has its
    /// own message, and any other refusal one of two at random
    /// (`FUN_004c4390`, `FUN_004c2230`). Each stays due 10 days.
    pub fn refused(&mut self, status: (u32, u32), decoys_without_target: bool, today: u32) {
        let (same, decoy, generic) = match self.faction {
            AdvisorFaction::Alliance => ((0x0f, 0x10), 0x14, (0x11, 0x12)),
            AdvisorFaction::Empire => ((0x10, 0x11), 0x15, (0x12, 0x13)),
        };
        let message = if self.last_refusal == Some(status) {
            if self.refusal_repeated {
                self.refusal_repeated = false;
                self.last_refusal = None;
                same.1
            } else {
                self.refusal_repeated = true;
                same.0
            }
        } else {
            self.last_refusal = Some(status);
            self.refusal_repeated = false;
            if decoys_without_target && status == (0x40, 0x91) {
                decoy
            } else if self.next_roll() % 2 == 0 {
                generic.0
            } else {
                generic.1
            }
        };
        self.last_day[message as usize] = today + 10;
    }

    /// A delivered message's event code (`FUN_0048a060` → slot `+0x14`,
    /// `FUN_004c44b0` Alliance, `FUN_004c22f0` Empire), which schedules the
    /// side's message for it (`advisor-briefing.md`, "The events"). The
    /// advice (`0x2a`) picks one of four only while none of them waits
    /// (`FUN_004c4430`, `FUN_004c2270`); `0x2b` picks one of two
    /// (`FUN_004c4480`, `FUN_004c22c0`). Codes a side does not handle do
    /// nothing.
    pub fn event(&mut self, code: u32, today: u32) {
        let (first, count) = match (self.faction, code) {
            (AdvisorFaction::Alliance, 0x2a) => (0x45, 4),
            (AdvisorFaction::Empire, 0x2a) => (0x42, 4),
            (AdvisorFaction::Alliance, 0x2b) => (0x49, 2),
            (AdvisorFaction::Empire, 0x2b) => (0x46, 2),
            _ => {
                if let Some((message, days)) = event_message(self.faction, code) {
                    self.last_day[message as usize] = today + days;
                }
                return;
            }
        };
        let slots = first as usize..(first + count) as usize;
        if code == 0x2a && self.last_day[slots.clone()].iter().any(|&day| day != 0) {
            return;
        }
        let pick = self.next_roll() % count;
        self.last_day[slots.start + pick as usize] = today + 10;
    }

    /// The port's stand-in for the interface's generator (xorshift32).
    fn next_roll(&mut self) -> u32 {
        self.roll ^= self.roll << 13;
        self.roll ^= self.roll >> 17;
        self.roll ^= self.roll << 5;
        self.roll
    }

    /// Reaction bits to answer ([`reaction`]): the agent's toggles, or the
    /// Agent menu's opening, which also counts (`FUN_00439ed0`).
    pub fn add_flags(&mut self, bits: u32) {
        if bits & reaction::MENU != 0 {
            self.menu_openings += 1;
        }
        self.flags |= bits;
    }

    /// The bits not yet answered.
    #[must_use]
    pub const fn pending_flags(&self) -> u32 {
        self.flags
    }

    /// `FUN_004c2dd0` / `FUN_004c0d00` with the agent active: the first bit
    /// in the side's order schedules its message, due for its days from
    /// `today`, and is cleared. The menu's answer counts on: the Alliance's
    /// `0x4a + n`, the Empire's `0x49 + n`, kept within three messages and
    /// starting over past them. The quip rolls one of twelve; six pick a
    /// message (`0x50..` for 40000 days, `0x4e..` for 5) and it re-arms
    /// the update.
    fn react(&mut self, today: u32) {
        if let Some(reaction) = reactions(self.faction)
            .iter()
            .find(|reaction| self.flags & reaction.bit != 0)
        {
            self.flags &= !reaction.bit;
            self.last_day[reaction.message as usize] = today + reaction.days;
            return;
        }
        if self.flags & reaction::MENU != 0 {
            self.flags &= !reaction::MENU;
            let (base, low, high) = match self.faction {
                AdvisorFaction::Alliance => (0x4a, 0x4c, 0x4e),
                AdvisorFaction::Empire => (0x49, 0x4b, 0x4d),
            };
            let mut message = base + self.menu_openings;
            if message < low {
                message = low;
            } else if message > high {
                message = high;
                self.menu_openings = 0;
            }
            self.last_day[message as usize] = today + 5;
            return;
        }
        if self.flags & reaction::QUIP != 0 {
            self.flags &= !reaction::QUIP;
            let pick = self.next_roll() % 12;
            if pick < 6 {
                let (first, days) = match self.faction {
                    AdvisorFaction::Alliance => (0x50, ALWAYS),
                    AdvisorFaction::Empire => (0x4e, 5),
                };
                self.last_day[(first + pick) as usize] = today + days;
            }
            self.enabled = true;
        }
    }

    /// A loaded game's advisor (`FUN_00439550`): its saved last days and
    /// update flag; the next free days load as 0. A table of another length
    /// is fitted to the side's message count.
    #[must_use]
    pub fn from_save(
        faction: AdvisorFaction,
        messages: HashMap<u32, Vec<ScriptEntry>>,
        last_day: &[u32],
        enabled: bool,
    ) -> Self {
        let count = message_count(faction);
        let mut table = last_day.to_vec();
        table.resize(count, 0);
        Self {
            last_day: table,
            enabled,
            ..Self::new_game(faction, messages)
        }
    }

    /// What the save keeps (`FUN_004397a0`): the last days and whether the
    /// update runs.
    #[must_use]
    pub fn saved(&self) -> (&[u32], bool) {
        (&self.last_day, self.enabled)
    }

    /// Translate Counterpart's bit, from the agent.
    pub fn set_translate(&mut self, translate: bool) {
        self.translate = translate;
    }

    #[must_use]
    pub const fn faction(&self) -> AdvisorFaction {
        self.faction
    }

    /// Whether the briefing holds the mouse buttons and Escape.
    #[must_use]
    pub const fn holds_input(&self) -> bool {
        self.holding_input
    }

    /// Whether the briefing still takes every key: until a skip flushes
    /// the players (`LAB_0041d8f0` passes keys on once `DAT_006b14b4` is
    /// set).
    #[must_use]
    pub const fn holds_keys(&self) -> bool {
        self.holding_input && !self.flushed
    }

    /// A left or right press, or Escape, while the input is held.
    pub fn request_skip(&mut self) {
        if self.holding_input {
            self.skip_requested = true;
        }
    }

    /// Run a clip's ended action on the schedule's side: codes `0xc` and
    /// `0xd` take and release the input (`FUN_004c0fc0`). The map actions
    /// are the caller's ([`briefing_action`]).
    pub fn clip_finished(&mut self, action: ClipAction) {
        if action.kind != 2 {
            return;
        }
        match action.code {
            0xc => {
                self.holding_input = true;
                self.flushed = false;
            }
            0xd => self.holding_input = false,
            _ => {}
        }
    }

    /// Advance by `dt` seconds of panel time on day `today`: the idle clip,
    /// a skip, and the advisor's update.
    pub fn update(&mut self, dt: f32, today: u32, player: &mut ClipPlayer) {
        if self.skip_requested {
            self.skip(today, player);
        }
        if player.is_busy() {
            self.idle_seconds = 0.0;
        } else {
            self.idle_seconds += dt;
            #[expect(
                clippy::cast_precision_loss,
                reason = "a tick count far below f32's exact range"
            )]
            let idle_length = IDLE_TICKS as f32 * CLIP_TICK_SECONDS;
            if self.idle_seconds >= idle_length {
                // FUN_00439ef0: the update runs again, and after 50 quiet
                // days the advisor has a quip.
                self.idle_seconds -= idle_length;
                self.enabled = true;
                if self.last_played + 0x32 < today {
                    self.flags |= reaction::QUIP;
                }
            }
        }
        if self.enabled {
            self.play_due(today, player);
        }
    }

    /// `FUN_004c0a60`: the first message still due whose rest is over
    /// plays; with none, the advisor waits for the next idle end
    /// (`FUN_004c0d00`).
    fn play_due(&mut self, today: u32, player: &mut ClipPlayer) {
        for id in 1..self.last_day.len() {
            let last = self.last_day[id];
            self.last_day[id] = 0;
            if last == 0 || last < today {
                continue;
            }
            if self.next_free[id] < today {
                self.next_free[id] = today + REST_DAYS;
                self.last_played = today;
                self.queue_message(id as u32, player);
                return;
            }
        }
        // FUN_004c0d00: nothing due; the update stops and a pending
        // reaction is scheduled.
        self.enabled = false;
        self.react(today);
    }

    /// `FUN_0041d660` -> `FUN_0042b1d0` for each entry: the record comes
    /// from the briefing DLL with flag bit 1, else the sprite DLL; a and b
    /// replace its words 2 and 3 when not 0. Entries with flag bit 0 play
    /// only while Translate Counterpart (`0x1000`) is on; a new game turns
    /// it on (`FUN_004c27f0`, `FUN_004c0710`).
    fn queue_message(&self, id: u32, player: &mut ClipPlayer) {
        let Some(entries) = self.messages.get(&id) else {
            return;
        };
        // FUN_004c0a60: an entry with flag bit 0 plays while 0x1000 is set
        // (hyp: 0x4000, which would block it, is never set here; its writer
        // FUN_00487f80 is not ported).
        for entry in entries.iter().filter(|entry| entry.flags & 1 == 0 || self.translate) {
            let slot = if entry.flags & 2 != 0 {
                BRIEFING_SLOT
            } else {
                SPRITE_SLOT
            };
            player.queue_entry(slot, entry.record, entry.a, entry.b);
        }
    }

    /// `FUN_004c0d00` with `0x2000`: the short form is due, and the first
    /// skip flushes the players (`FUN_0041da30`).
    fn skip(&mut self, today: u32, player: &mut ClipPlayer) {
        self.skip_requested = false;
        let (_, short) = briefings(self.faction);
        self.last_day[short as usize] = today + ALWAYS;
        if !self.flushed {
            player.clear();
            self.flushed = true;
        }
    }
}

/// A briefing caption's text: TEXTSTRA `0x1622` "Yavin", `0x1623`
/// "Alliance Headquarters", `0x1624` "Coruscant", `0x1625` "Mon Mothma",
/// `0x1626` "Luke Skywalker", `0x1627` "Emperor Palpatine", `0x1628`
/// "Darth Vader".
#[must_use]
pub const fn caption_text(id: u16) -> Option<&'static str> {
    match id {
        0x1622 => Some("Yavin"),
        0x1623 => Some("Alliance Headquarters"),
        0x1624 => Some("Coruscant"),
        0x1625 => Some("Mon Mothma"),
        0x1626 => Some("Luke Skywalker"),
        0x1627 => Some("Emperor Palpatine"),
        0x1628 => Some("Darth Vader"),
        _ => None,
    }
}

/// While the briefing holds the input, an invisible layer over the whole
/// screen takes the pointer, as the mouse hook swallows the buttons
/// (`FUN_0041d950`).
pub fn draw_input_hold(ctx: &egui::Context) {
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new("briefing_input_hold"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.allocate_rect(screen, egui::Sense::click_and_drag());
        });
}

/// Where a marking action puts its mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkTarget {
    /// The system whose record index is this (`FUN_005f55d0`).
    System(u32),
    /// The system an object stands in (`FUN_004f2d10`, walked up to a
    /// system record, families `0x90..0x97`): a character, or the
    /// Alliance's headquarters `0x20000005`.
    ObjectSystem(u32),
}

/// The Alliance headquarters' record (`FUN_004c30c0`, case 3).
pub const ALLIANCE_HEADQUARTERS: u32 = 0x2000_0005;

/// What a briefing action does to the galaxy view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BriefingAction {
    /// Show a display (`FUN_0041d8c0`).
    Display(GidMode),
    /// Mark one system in a side's colour with a caption
    /// (`FUN_0041d890(0x92 | 0x93, .., caption)`).
    Mark {
        target: MarkTarget,
        mode: GidMode,
        caption: u16,
    },
}

/// The galaxy-view part of a kind-2 action `code` of `faction`'s advisor:
/// the Alliance's `FUN_004c30c0`, the Empire's `FUN_004c0fc0`. Codes `0xc`
/// and `0xd` hold and release the input
/// ([`AdvisorSchedule::clip_finished`]); `0xa` is in neither table.
#[must_use]
pub const fn briefing_action(faction: AdvisorFaction, code: u32) -> Option<BriefingAction> {
    use BriefingAction::Display;
    const fn mark(target: MarkTarget, mode: GidMode, caption: u16) -> BriefingAction {
        BriefingAction::Mark {
            target,
            mode,
            caption,
        }
    }
    let imperial = GidMode::MarkedImperialSystem;
    let rebel = GidMode::MarkedAllianceSystem;
    Some(match (faction, code) {
        (_, 1 | 0xb) => Display(GidMode::PopularSupport),
        (_, 5) => Display(GidMode::BriefingFleets),
        (_, 6) => Display(GidMode::AllDefenses),
        (_, 9) => Display(GidMode::DisplayOff),
        (_, 0xf) => Display(GidMode::EnemyLoyalSystems),
        (_, 0x11) => Display(GidMode::UnexploredSystems),
        (AdvisorFaction::Alliance, 2 | 0xe) => Display(GidMode::OwnLoyalSystems),
        (AdvisorFaction::Alliance, 3) => mark(
            MarkTarget::ObjectSystem(ALLIANCE_HEADQUARTERS),
            rebel,
            0x1623,
        ),
        (AdvisorFaction::Alliance, 4 | 8 | 0x14) => mark(MarkTarget::System(0x109), imperial, 0x1624),
        (AdvisorFaction::Alliance, 7) => mark(MarkTarget::ObjectSystem(0x3000_0240), rebel, 0x1625),
        (AdvisorFaction::Alliance, 0x10) => Display(GidMode::EnemyMilitaryControl),
        (AdvisorFaction::Alliance, 0x12) => mark(MarkTarget::System(0x121), rebel, 0x1622),
        (AdvisorFaction::Alliance, 0x13) => {
            mark(MarkTarget::ObjectSystem(0x3200_0242), rebel, 0x1626)
        }
        (AdvisorFaction::Empire, 2 | 0x12) => mark(MarkTarget::System(0x109), imperial, 0x1624),
        (AdvisorFaction::Empire, 3 | 8 | 0x14) => mark(MarkTarget::System(0x121), rebel, 0x1622),
        (AdvisorFaction::Empire, 4) => Display(GidMode::UnexploredSystems),
        (AdvisorFaction::Empire, 7) => mark(MarkTarget::ObjectSystem(0x3400_0280), imperial, 0x1627),
        (AdvisorFaction::Empire, 0xe) => Display(GidMode::OwnLoyalSystems),
        (AdvisorFaction::Empire, 0x10) => Display(GidMode::OwnMilitaryControl),
        (AdvisorFaction::Empire, 0x13) => {
            mark(MarkTarget::ObjectSystem(0x3500_0281), imperial, 0x1628)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisor_clip::{ClipEvent, OP_DEFINITION, OP_FRAME_LIST};
    use std::path::{Path, PathBuf};

    /// Message 0x29's opening (C3POACT.SPT): action 0xc, clip 0x27ab, action 1.
    fn briefing_messages() -> HashMap<u32, Vec<ScriptEntry>> {
        let action = |code| ScriptEntry {
            record: 0x2710,
            flags: 0,
            a: 2,
            b: code,
        };
        let clip = ScriptEntry {
            record: 0x27ab,
            flags: 2,
            a: 0,
            b: 0,
        };
        HashMap::from([
            (ALLIANCE_BRIEFING, vec![action(0xc), clip, action(1)]),
            (ALLIANCE_SHORT_BRIEFING, vec![action(0xb), action(0xd)]),
        ])
    }

    struct Root(PathBuf);

    impl Drop for Root {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The records the messages name: [0x11, 0x18f] and [4, 0x103b], the
    /// frame list [2001] and a one-frame definition of BMP 2101.
    fn staged_root(name: &str) -> Root {
        let root = std::env::temp_dir().join(format!("advisor-schedule-{}-{name}", std::process::id()));
        let write = |path: &str, bytes: &[u8]| {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        };
        let words = |words: &[u16]| -> Vec<u8> { words.iter().flat_map(|w| w.to_le_bytes()).collect() };
        let bmp = crate::advisor::tests::indexed_bmp_fixture();
        write("alsprite-dll/RCDATA/10000.bin", &words(&[OP_FRAME_LIST, 0x18f, 0, 0]));
        write("alsprite-dll/RCDATA/399.bin", &words(&[2001]));
        write("alsprite-dll/BMP/2001.bmp", &bmp);
        write("albrief-dll/RCDATA/10155.bin", &words(&[OP_DEFINITION, 0x103b, 0, 0]));
        write("albrief-dll/RCDATA/4155.bin", &words(&[0, 0, BRIEFING_SLOT, 1, 2101]));
        write("albrief-dll/BMP/2101.bmp", &bmp);
        Root(root)
    }

    fn run(
        schedule: &mut AdvisorSchedule,
        player: &mut ClipPlayer,
        seconds: f32,
        today: u32,
    ) -> Vec<ClipEvent> {
        let mut events = Vec::new();
        let mut left = seconds;
        while left > 0.0 {
            let dt = CLIP_TICK_SECONDS.min(left);
            schedule.update(dt, today, player);
            events.extend(player.update(dt));
            for event in &events {
                if let ClipEvent::Finished(action) = event {
                    schedule.clip_finished(*action);
                }
            }
            left -= dt;
        }
        events
    }

    fn player(root: &Path) -> ClipPlayer {
        ClipPlayer::new(AdvisorFaction::Alliance, root)
    }

    #[test]
    fn a_new_game_schedules_its_sides_briefing_for_40000_days() {
        // FUN_004c27f0 (+0xa4) and FUN_004c0710 (+0x128).
        let alliance = AdvisorSchedule::new_game(AdvisorFaction::Alliance, HashMap::new());
        assert_eq!(alliance.last_day.len(), 0x58);
        assert_eq!(alliance.last_day[0x29], 40_000);
        assert_eq!(alliance.last_day.iter().filter(|&&day| day != 0).count(), 1);
        let empire = AdvisorSchedule::new_game(AdvisorFaction::Empire, HashMap::new());
        assert_eq!(empire.last_day.len(), 0x56);
        assert_eq!(empire.last_day[0x4a], 40_000);
    }

    #[test]
    fn the_briefing_starts_once_the_idle_clip_has_run_its_fifteen_ticks() {
        let root = staged_root("start");
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, briefing_messages());
        let mut player = player(&root.0);

        run(&mut schedule, &mut player, 14.0 * CLIP_TICK_SECONDS, 1);
        assert!(!player.is_busy(), "the idle clip still runs");
        run(&mut schedule, &mut player, 1.5 * CLIP_TICK_SECONDS, 1);
        assert!(player.is_busy());
        // The briefing has played; the schedule holds it for 60 days.
        assert_eq!(schedule.last_day[ALLIANCE_BRIEFING as usize], 0);
        assert_eq!(schedule.next_free[ALLIANCE_BRIEFING as usize], 61);
    }

    #[test]
    fn the_briefings_entries_queue_their_clips_and_actions_in_order() {
        let root = staged_root("order");
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, briefing_messages());
        let mut player = player(&root.0);

        let events = run(&mut schedule, &mut player, 40.0 * CLIP_TICK_SECONDS, 1);
        let finished: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                ClipEvent::Finished(action) => Some(*action),
                ClipEvent::Voice { .. } => None,
            })
            .collect();
        // The frame lists carry kind 2 and the entry's code; the definition
        // keeps its record's (0, 0).
        assert_eq!(
            finished,
            [
                ClipAction { kind: 2, code: 0xc },
                ClipAction { kind: 0, code: 0 },
                ClipAction { kind: 2, code: 1 },
            ]
        );
        assert!(schedule.holds_input());
    }

    #[test]
    fn a_skip_flushes_the_clips_and_plays_the_short_form() {
        // FUN_0043a200 sets 0x2000; FUN_004c2dd0 schedules 0x28 and
        // FUN_0041da30 flushes both players once.
        let root = staged_root("skip");
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, briefing_messages());
        let mut player = player(&root.0);
        run(&mut schedule, &mut player, 18.0 * CLIP_TICK_SECONDS, 1);
        assert!(schedule.holds_input());
        assert!(schedule.holds_keys());

        schedule.request_skip();
        run(&mut schedule, &mut player, CLIP_TICK_SECONDS, 1);
        assert!(!schedule.holds_keys(), "the keys pass once the players are flushed");
        assert!(schedule.holds_input(), "the buttons stay held until 0xd");
        let events = run(&mut schedule, &mut player, 40.0 * CLIP_TICK_SECONDS, 1);
        let finished: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                ClipEvent::Finished(action) => Some(action.code),
                ClipEvent::Voice { .. } => None,
            })
            .collect();
        assert_eq!(finished, [0xb, 0xd]);
        assert!(!schedule.holds_input());
    }

    #[test]
    fn a_skip_schedules_the_short_form_until_today_plus_40000() {
        // FUN_004c2dd0: +0xa0 (message 0x28) = today + 40000.
        let root = staged_root("skip-day");
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, briefing_messages());
        let mut player = player(&root.0);
        run(&mut schedule, &mut player, 18.0 * CLIP_TICK_SECONDS, 3);
        schedule.request_skip();
        schedule.update(0.0, 3, &mut player);
        assert_eq!(schedule.last_day[ALLIANCE_SHORT_BRIEFING as usize], 40_003);
    }

    #[test]
    fn a_message_past_its_last_day_is_dropped_and_one_due_today_plays() {
        // FUN_004c0a60 zeroes a last day under today and moves on.
        let root = staged_root("last-day");
        let mut late = AdvisorSchedule::new_game(AdvisorFaction::Alliance, briefing_messages());
        late.last_day[ALLIANCE_BRIEFING as usize] = 5;
        let mut player = player(&root.0);
        run(&mut late, &mut player, 16.0 * CLIP_TICK_SECONDS, 6);
        assert!(!player.is_busy());
        assert_eq!(late.last_day[ALLIANCE_BRIEFING as usize], 0);

        let mut due = AdvisorSchedule::new_game(AdvisorFaction::Alliance, briefing_messages());
        due.last_day[ALLIANCE_BRIEFING as usize] = 6;
        run(&mut due, &mut player, 16.0 * CLIP_TICK_SECONDS, 6);
        assert!(player.is_busy());
    }

    #[test]
    fn with_nothing_due_the_advisor_waits_for_the_next_idle_end() {
        // FUN_004c0d00 clears DAT_006b28bc; FUN_00439ef0 sets it again
        // when the idle clip next ends, 15 ticks on.
        let root = staged_root("next-idle");
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, briefing_messages());
        let mut player = player(&root.0);
        schedule.last_day[ALLIANCE_BRIEFING as usize] = 0;
        run(&mut schedule, &mut player, 15.5 * CLIP_TICK_SECONDS, 1);
        assert!(!schedule.enabled);

        schedule.last_day[ALLIANCE_BRIEFING as usize] = 40_000;
        run(&mut schedule, &mut player, 2.0 * CLIP_TICK_SECONDS, 1);
        assert!(!player.is_busy(), "the briefing waits for the idle clip");
        run(&mut schedule, &mut player, 14.0 * CLIP_TICK_SECONDS, 1);
        assert!(player.is_busy());
    }

    #[test]
    fn without_translate_counterpart_the_bit_zero_entries_stay_silent() {
        // FUN_004c0a60: an entry with flag bit 0 plays only while
        // DAT_006b28b0 has 0x1000. The briefing's opening action carries
        // bit 0 here; turned off, only its clip and closing action play.
        let root = staged_root("translate");
        let mut messages = briefing_messages();
        messages.get_mut(&ALLIANCE_BRIEFING).unwrap()[0].flags |= 1;
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, messages);
        schedule.set_translate(false);
        let mut player = player(&root.0);
        let events = run(&mut schedule, &mut player, 40.0 * CLIP_TICK_SECONDS, 1);
        let codes: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                ClipEvent::Finished(action) => Some(action.code),
                ClipEvent::Voice { .. } => None,
            })
            .collect();
        assert_eq!(codes, [0, 1]);
    }

    #[test]
    fn a_loaded_advisor_keeps_its_last_days_and_flag_and_frees_its_days() {
        // FUN_004397a0 saves DAT_006b28bc and +0x168; FUN_00439550 loads
        // them and zeroes +0x16c.
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, briefing_messages());
        schedule.next_free[3] = 99;
        schedule.enabled = true;
        let (last_day, enabled) = schedule.saved();
        let loaded = AdvisorSchedule::from_save(
            AdvisorFaction::Alliance,
            briefing_messages(),
            last_day,
            enabled,
        );
        assert_eq!(loaded.saved(), (&schedule.last_day[..], true));
        assert_eq!(loaded.next_free, vec![0; 0x58]);
        // A short table is fitted to the side's count.
        let short = AdvisorSchedule::from_save(AdvisorFaction::Empire, HashMap::new(), &[5], false);
        assert_eq!(short.last_day.len(), 0x56);
        assert_eq!(short.last_day[0], 5);
        assert!(!short.saved().1);
    }

    #[test]
    fn each_sides_toggle_reactions_follow_its_table_one_per_call() {
        // FUN_004c2dd0 (C3POACT) and FUN_004c0d00 (IMP22ACT): one bit a
        // call, in order; the message stays due its days from today.
        let mut alliance = AdvisorSchedule::new_game(AdvisorFaction::Alliance, HashMap::new());
        alliance.add_flags(reaction::GARRISONS_OFF | reaction::GARRISONS_ON);
        alliance.react(5);
        assert_eq!(alliance.last_day[0x24], 15, "0x10 before 0x100");
        assert_eq!(alliance.last_day[0x25], 0);
        alliance.react(5);
        assert_eq!(alliance.last_day[0x25], 55);
        assert_eq!(alliance.pending_flags(), 0);
        for (bit, message, days) in [
            (reaction::MOVE_DONE, 0x23, 10),
            (reaction::PRODUCTION_ON, 0x26, 10),
            (reaction::PRODUCTION_OFF, 0x27, 50),
            (reaction::TOO_MANY_TOGGLES, 0x4f, 5),
        ] {
            alliance.add_flags(bit);
            alliance.react(7);
            assert_eq!(alliance.last_day[message], 7 + days, "{bit:#x}");
        }

        let mut empire = AdvisorSchedule::new_game(AdvisorFaction::Empire, HashMap::new());
        for (bit, message, days) in [
            (reaction::MOVE_DONE, 0x1c, 10),
            (reaction::GARRISONS_ON, 0x1d, 10),
            (reaction::PRODUCTION_ON, 0x1f, 10),
            (reaction::GARRISONS_OFF, 0x1e, 50),
            (reaction::PRODUCTION_OFF, 0x20, 50),
        ] {
            empire.add_flags(bit);
            empire.react(3);
            assert_eq!(empire.last_day[message], 3 + days, "{bit:#x}");
        }
        // FUN_004c0d00 has no answer for 0x1000000: it stays.
        empire.add_flags(reaction::TOO_MANY_TOGGLES);
        empire.react(3);
        assert_eq!(empire.pending_flags(), reaction::TOO_MANY_TOGGLES);
    }

    #[test]
    fn only_the_menus_bit_counts_an_opening_and_answered_bits_clear() {
        // FUN_00439ed0 counts the menu's openings alone; FUN_004c2dd0
        // clears each bit it answers.
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, HashMap::new());
        schedule.add_flags(reaction::GARRISONS_ON);
        assert_eq!(schedule.menu_openings, 0);
        schedule.add_flags(reaction::MENU | reaction::QUIP);
        assert_eq!(schedule.menu_openings, 1);
        schedule.react(1);
        schedule.react(1);
        assert_eq!(schedule.pending_flags(), reaction::QUIP);
        schedule.react(1);
        assert_eq!(schedule.pending_flags(), 0);
    }

    #[test]
    fn the_quips_draws_follow_the_ports_generator() {
        // port: a xorshift32 from 0x2545f491 stands in for FUN_005f5700(12);
        // its draws 6, 7, 4, 2, 3, 3, ... pick messages 0x50 + n below 6
        // (Alliance, 40000 days) and 0x4e + n (Empire, 5 days).
        for (faction, first, days) in [
            (AdvisorFaction::Alliance, 0x50, 40_000),
            (AdvisorFaction::Empire, 0x4e, 5),
        ] {
            let mut schedule = AdvisorSchedule::new_game(faction, HashMap::new());
            let mut picked = Vec::new();
            for _ in 0..6 {
                schedule.last_day.iter_mut().for_each(|day| *day = 0);
                schedule.add_flags(reaction::QUIP);
                schedule.react(10);
                picked.push(schedule.last_day.iter().position(|&day| day != 0).map(|index| {
                    assert_eq!(schedule.last_day[index], 10 + days);
                    index - first
                }));
            }
            assert_eq!(picked, [None, None, Some(4), Some(2), Some(3), Some(3)], "{faction:?}");
        }
    }

    #[test]
    fn the_agent_menus_answers_count_on_within_three_messages() {
        // FUN_00439ed0 counts DAT_006b28b8; the answer is 0x4a + n kept in
        // 0x4c..0x4e (Alliance), 0x49 + n in 0x4b..0x4d (Empire), and the
        // count starts over past the last.
        for (faction, expected) in [
            (AdvisorFaction::Alliance, [0x4c, 0x4c, 0x4d, 0x4e, 0x4e, 0x4c]),
            (AdvisorFaction::Empire, [0x4b, 0x4b, 0x4c, 0x4d, 0x4d, 0x4b]),
        ] {
            let mut schedule = AdvisorSchedule::new_game(faction, HashMap::new());
            for (opening, message) in expected.into_iter().enumerate() {
                schedule.add_flags(reaction::MENU);
                schedule.last_day.iter_mut().for_each(|day| *day = 0);
                schedule.react(9);
                assert_eq!(schedule.last_day[message], 14, "{faction:?} opening {}", opening + 1);
            }
        }
    }

    #[test]
    fn after_fifty_quiet_days_an_idle_end_brings_a_quip() {
        // FUN_00439ef0: DAT_006b28d0 + 0x32 < today sets 0x80000000; the
        // answer picks one of twelve, six with a message, and re-arms the
        // update (DAT_006b28bc = 1).
        let root = staged_root("quip");
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, HashMap::new());
        schedule.last_day.iter_mut().for_each(|day| *day = 0);
        let mut player = player(&root.0);
        run(&mut schedule, &mut player, 16.0 * CLIP_TICK_SECONDS, 50);
        assert_eq!(schedule.pending_flags() & reaction::QUIP, 0, "day 50 is not past 50");
        let mut picked = 0;
        for _ in 0..40 {
            schedule.add_flags(reaction::QUIP);
            schedule.enabled = false;
            schedule.react(60);
            assert!(schedule.enabled);
            if let Some(index) = (0x50..0x56).find(|&index| schedule.last_day[index] != 0) {
                assert_eq!(schedule.last_day[index], 60 + 40_000);
                schedule.last_day[index] = 0;
                picked += 1;
            }
        }
        assert!(picked > 0 && picked < 40, "{picked}");
        // From the idle end: some rolls play a quip on day 51, none on 50.
        let quips = |day| {
            (1..=12)
                .filter(|&seed| {
                    let mut quiet = AdvisorSchedule::new_game(
                        AdvisorFaction::Alliance,
                        HashMap::from([(0x50 + seed % 6, vec![])]),
                    );
                    quiet.last_day.iter_mut().for_each(|day| *day = 0);
                    quiet.roll = seed * 7919;
                    let mut clips = super::tests::player(&root.0);
                    run(&mut quiet, &mut clips, 17.0 * CLIP_TICK_SECONDS, day);
                    quiet.last_played == day
                })
                .count()
        };
        assert_eq!(quips(50), 0);
        assert!(quips(51) > 0);
    }

    #[test]
    fn a_toggles_reaction_plays_after_the_next_idle_end() {
        // FUN_004c0a60 finds nothing due, FUN_004c2dd0 schedules 0x24; the
        // next idle end plays it.
        let root = staged_root("toggle");
        let action = |code| ScriptEntry {
            record: 0x2710,
            flags: 0,
            a: 2,
            b: code,
        };
        let mut schedule = AdvisorSchedule::new_game(
            AdvisorFaction::Alliance,
            HashMap::from([(0x24, vec![action(0x77)])]),
        );
        schedule.last_day.iter_mut().for_each(|day| *day = 0);
        schedule.last_played = 1;
        schedule.add_flags(reaction::GARRISONS_ON);
        let mut player = player(&root.0);
        let events = run(&mut schedule, &mut player, 34.0 * CLIP_TICK_SECONDS, 2);
        assert!(
            events.iter().any(|event| matches!(event, ClipEvent::Finished(ClipAction { kind: 2, code: 0x77 }))),
            "{events:?}"
        );
        assert_eq!(schedule.last_played, 2);
    }

    #[test]
    fn a_refusal_schedules_its_reaction_and_a_repeat_the_two_after_it() {
        // FUN_004c2940 / FUN_004c0870: the same status twice running gives
        // the first of +0x3c/+0x40 (Alliance) or +0x40/+0x44 (Empire), then
        // the second and forgets it; an empty team with decoys and no target
        // +0x50 / +0x54; any other refusal one of +0x44/+0x48 or
        // +0x48/+0x4c. Ten days each.
        for (faction, same, decoy, generic) in [
            (AdvisorFaction::Alliance, (0x0f, 0x10), 0x14, [0x11, 0x12]),
            (AdvisorFaction::Empire, (0x10, 0x11), 0x15, [0x12, 0x13]),
        ] {
            let mut schedule = AdvisorSchedule::new_game(faction, HashMap::new());
            let due = |schedule: &AdvisorSchedule| {
                schedule.last_day.iter().position(|&day| day != 0 && day != ALWAYS)
            };
            let clear = |schedule: &mut AdvisorSchedule| {
                schedule.last_day.iter_mut().for_each(|day| *day = 0);
            };
            clear(&mut schedule);
            schedule.refused((0x40, 0x01), false, 4);
            let first = due(&schedule).unwrap();
            // The port's generator's first roll from its seed is even.
            assert_eq!(first, generic[0], "{faction:?} {first:#x}");
            assert_eq!(schedule.last_day[first], 14);
            clear(&mut schedule);
            schedule.refused((0x40, 0x01), false, 4);
            assert_eq!(due(&schedule), Some(same.0));
            clear(&mut schedule);
            schedule.refused((0x40, 0x01), false, 4);
            assert_eq!(due(&schedule), Some(same.1));
            // Forgotten: the next one is a fresh refusal again.
            clear(&mut schedule);
            schedule.refused((0x40, 0x01), false, 4);
            assert!(generic.contains(&due(&schedule).unwrap()));
            clear(&mut schedule);
            schedule.refused((0x40, 0x91), true, 4);
            assert_eq!(due(&schedule), Some(decoy));
            // Decoys matter only for 0x40/0x91.
            clear(&mut schedule);
            schedule.refused((0x40, 0x28), true, 4);
            assert!(generic.contains(&due(&schedule).unwrap()));
        }
        // Both generic messages come up.
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, HashMap::new());
        let mut seen = std::collections::BTreeSet::new();
        for code in 0..8 {
            schedule.last_day.iter_mut().for_each(|day| *day = 0);
            schedule.refused((0x40, code), false, 1);
            seen.insert(schedule.last_day.iter().position(|&day| day != 0).unwrap());
        }
        assert_eq!(seen.into_iter().collect::<Vec<_>>(), [0x11, 0x12]);
    }

    #[test]
    fn each_sides_events_schedule_their_messages() {
        // FUN_004c44b0 (Alliance) / FUN_004c22f0 (Empire): +0x168[offset / 4]
        // = today + days.
        // Every case, as the byte offsets the decompiles write.
        for (faction, code, message, days) in [
            (AdvisorFaction::Alliance, 0x1, 0xd0 / 4, 10),
            (AdvisorFaction::Alliance, 0x2, 0xd4 / 4, 10),
            (AdvisorFaction::Alliance, 0x3, 0xe0 / 4, 5),
            (AdvisorFaction::Alliance, 0x4, 0xe8 / 4, 10),
            (AdvisorFaction::Alliance, 0x5, 0xec / 4, 10),
            (AdvisorFaction::Alliance, 0x6, 0xf4 / 4, 10),
            (AdvisorFaction::Alliance, 0x8, 0xf0 / 4, 10),
            (AdvisorFaction::Alliance, 0x9, 0xf8 / 4, 10),
            (AdvisorFaction::Alliance, 0xc, 0xe4 / 4, 10),
            (AdvisorFaction::Alliance, 0xd, 0x104 / 4, 10),
            (AdvisorFaction::Alliance, 0xe, 0x10c / 4, 10),
            (AdvisorFaction::Alliance, 0x14, 0xa8 / 4, 10),
            (AdvisorFaction::Alliance, 0x15, 0x108 / 4, 10),
            (AdvisorFaction::Alliance, 0x16, 0xb4 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x17, 0xac / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x18, 0xb0 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x19, 0xb8 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x1a, 0xa8 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x1b, 0xa8 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x1c, 0xdc / 4, 10),
            (AdvisorFaction::Alliance, 0x1e, 0x70 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x1f, 0x6c / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x20, 0x74 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x21, 0x78 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x22, 0x80 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x23, 0x7c / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x24, 0xcc / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x25, 0xc0 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x26, 0xbc / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x27, 0xc8 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x28, 0xc4 / 4, ALWAYS),
            (AdvisorFaction::Alliance, 0x29, 0x110 / 4, 0x32),
            (AdvisorFaction::Alliance, 0x2c, 0x12c / 4, 10),
            (AdvisorFaction::Alliance, 0x2e, 0x64 / 4, 10),
            (AdvisorFaction::Alliance, 0x2f, 0x68 / 4, 10),
            (AdvisorFaction::Empire, 0x1, 0xc0 / 4, 10),
            (AdvisorFaction::Empire, 0x2, 0xc4 / 4, 10),
            (AdvisorFaction::Empire, 0x3, 0xd0 / 4, 10),
            (AdvisorFaction::Empire, 0x4, 0xd8 / 4, 10),
            (AdvisorFaction::Empire, 0x5, 0xdc / 4, 10),
            (AdvisorFaction::Empire, 0x6, 0xe4 / 4, 10),
            (AdvisorFaction::Empire, 0x8, 0xe0 / 4, 10),
            (AdvisorFaction::Empire, 0x9, 0xe8 / 4, 10),
            (AdvisorFaction::Empire, 0xc, 0xd4 / 4, 10),
            (AdvisorFaction::Empire, 0xd, 0xf4 / 4, 10),
            (AdvisorFaction::Empire, 0xe, 0xf8 / 4, 5),
            (AdvisorFaction::Empire, 0x14, 0x100 / 4, 10),
            (AdvisorFaction::Empire, 0x15, 0xfc / 4, 10),
            (AdvisorFaction::Empire, 0x16, 0x100 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x17, 0x100 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x18, 0x100 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x19, 0x100 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x1a, 0x88 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x1b, 0x84 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x1c, 0xcc / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x1e, 0xa8 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x1f, 0xa4 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x20, 0xac / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x21, 0xb0 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x22, 0xb8 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x23, 0xb4 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x24, 0xbc / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x25, 0x98 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x26, 0x94 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x27, 0xa0 / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x28, 0x9c / 4, ALWAYS),
            (AdvisorFaction::Empire, 0x29, 0x104 / 4, 0x32),
            (AdvisorFaction::Empire, 0x2d, 0x120 / 4, 10),
            (AdvisorFaction::Empire, 0x2e, 0x8c / 4, 10),
            (AdvisorFaction::Empire, 0x2f, 0x90 / 4, 10),
        ] {
            let mut schedule = AdvisorSchedule::new_game(faction, HashMap::new());
            schedule.last_day.iter_mut().for_each(|day| *day = 0);
            schedule.event(code, 7);
            let due: Vec<usize> =
                (0..schedule.last_day.len()).filter(|&id| schedule.last_day[id] != 0).collect();
            assert_eq!(due, [message as usize], "{faction:?} {code:#x}");
            assert_eq!(schedule.last_day[message as usize], 7 + days);
        }
        // Codes a side has no case for: 0x2d on the Alliance, 0x2c and 0x10
        // on the Empire.
        for (faction, code) in [
            (AdvisorFaction::Alliance, 0x2d),
            (AdvisorFaction::Empire, 0x2c),
            (AdvisorFaction::Empire, 0x10),
        ] {
            let mut schedule = AdvisorSchedule::new_game(faction, HashMap::new());
            schedule.last_day.iter_mut().for_each(|day| *day = 0);
            schedule.event(code, 7);
            assert!(schedule.last_day.iter().all(|&day| day == 0), "{faction:?} {code:#x}");
        }
    }

    #[test]
    fn the_advice_picks_one_of_four_only_while_none_waits() {
        // FUN_004c4430 / FUN_004c2270: none of +0x114..+0x120 (Alliance) or
        // +0x108..+0x114 (Empire) set, then FUN_0041cd80(4); 0x2b
        // (FUN_004c4480 / FUN_004c22c0) one of two, always.
        for (faction, advice, other) in [
            (AdvisorFaction::Alliance, 0x45_usize, 0x49_usize),
            (AdvisorFaction::Empire, 0x42, 0x46),
        ] {
            let mut schedule = AdvisorSchedule::new_game(faction, HashMap::new());
            schedule.last_day.iter_mut().for_each(|day| *day = 0);
            schedule.event(0x2a, 3);
            let set: Vec<usize> = (advice..advice + 4).filter(|&id| schedule.last_day[id] != 0).collect();
            assert_eq!(set.len(), 1, "{faction:?}");
            assert_eq!(schedule.last_day[set[0]], 13);
            schedule.event(0x2a, 3);
            assert_eq!(
                (advice..advice + 4).filter(|&id| schedule.last_day[id] != 0).count(),
                1,
                "{faction:?}: one still waits"
            );
            schedule.event(0x2b, 3);
            assert_eq!(
                (other..other + 2).filter(|&id| schedule.last_day[id] != 0).count(),
                1,
                "{faction:?}"
            );
            schedule.event(0x2b, 3);
            schedule.event(0x2b, 3);
            schedule.event(0x2b, 3);
            assert!((other..other + 2).all(|id| schedule.last_day[id] == 13), "{faction:?}");
        }
    }

    #[test]
    fn a_skip_outside_the_hold_is_ignored() {
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, HashMap::new());
        schedule.request_skip();
        assert!(!schedule.skip_requested);
    }

    #[test]
    fn the_briefing_waits_for_a_day_after_its_next_free_day() {
        // FUN_004c0a60 plays a message only when its next free day (0 in a
        // new schedule) is below today.
        let root = staged_root("day");
        let mut schedule = AdvisorSchedule::new_game(AdvisorFaction::Alliance, briefing_messages());
        let mut player = player(&root.0);
        run(&mut schedule, &mut player, 16.0 * CLIP_TICK_SECONDS, 0);
        assert!(!player.is_busy());
    }

    #[test]
    fn the_empires_briefing_actions_follow_fun_004c0fc0() {
        use AdvisorFaction::Empire;
        let display = |mode| Some(BriefingAction::Display(mode));
        assert_eq!(briefing_action(Empire, 1), display(GidMode::PopularSupport));
        assert_eq!(briefing_action(Empire, 0xb), display(GidMode::PopularSupport));
        assert_eq!(briefing_action(Empire, 0xe), display(GidMode::OwnLoyalSystems));
        assert_eq!(briefing_action(Empire, 0xf), display(GidMode::EnemyLoyalSystems));
        assert_eq!(briefing_action(Empire, 0x10), display(GidMode::OwnMilitaryControl));
        assert_eq!(briefing_action(Empire, 0x11), display(GidMode::UnexploredSystems));
        assert_eq!(briefing_action(Empire, 4), display(GidMode::UnexploredSystems));
        assert_eq!(briefing_action(Empire, 5), display(GidMode::BriefingFleets));
        assert_eq!(briefing_action(Empire, 6), display(GidMode::AllDefenses));
        assert_eq!(briefing_action(Empire, 9), display(GidMode::DisplayOff));
        let mark = |target, mode, caption| {
            Some(BriefingAction::Mark {
                target,
                mode,
                caption,
            })
        };
        let coruscant = mark(MarkTarget::System(0x109), GidMode::MarkedImperialSystem, 0x1624);
        let yavin = mark(MarkTarget::System(0x121), GidMode::MarkedAllianceSystem, 0x1622);
        for code in [2, 0x12] {
            assert_eq!(briefing_action(Empire, code), coruscant);
        }
        for code in [3, 8, 0x14] {
            assert_eq!(briefing_action(Empire, code), yavin);
        }
        assert_eq!(
            briefing_action(Empire, 7),
            mark(MarkTarget::ObjectSystem(0x3400_0280), GidMode::MarkedImperialSystem, 0x1627)
        );
        assert_eq!(
            briefing_action(Empire, 0x13),
            mark(MarkTarget::ObjectSystem(0x3500_0281), GidMode::MarkedImperialSystem, 0x1628)
        );
        for code in [0xa, 0xc, 0xd, 0x15] {
            assert_eq!(briefing_action(Empire, code), None);
        }
    }

    #[test]
    fn the_alliances_briefing_actions_follow_fun_004c30c0() {
        use AdvisorFaction::Alliance;
        let display = |mode| Some(BriefingAction::Display(mode));
        for code in [2, 0xe] {
            assert_eq!(briefing_action(Alliance, code), display(GidMode::OwnLoyalSystems));
        }
        assert_eq!(briefing_action(Alliance, 0xf), display(GidMode::EnemyLoyalSystems));
        assert_eq!(briefing_action(Alliance, 0x10), display(GidMode::EnemyMilitaryControl));
        assert_eq!(briefing_action(Alliance, 0x11), display(GidMode::UnexploredSystems));
        assert_eq!(briefing_action(Alliance, 1), display(GidMode::PopularSupport));
        assert_eq!(briefing_action(Alliance, 9), display(GidMode::DisplayOff));
        let mark = |target, mode, caption| {
            Some(BriefingAction::Mark {
                target,
                mode,
                caption,
            })
        };
        assert_eq!(
            briefing_action(Alliance, 3),
            mark(MarkTarget::ObjectSystem(0x2000_0005), GidMode::MarkedAllianceSystem, 0x1623)
        );
        for code in [4, 8, 0x14] {
            assert_eq!(
                briefing_action(Alliance, code),
                mark(MarkTarget::System(0x109), GidMode::MarkedImperialSystem, 0x1624)
            );
        }
        assert_eq!(
            briefing_action(Alliance, 7),
            mark(MarkTarget::ObjectSystem(0x3000_0240), GidMode::MarkedAllianceSystem, 0x1625)
        );
        assert_eq!(
            briefing_action(Alliance, 0x12),
            mark(MarkTarget::System(0x121), GidMode::MarkedAllianceSystem, 0x1622)
        );
        assert_eq!(
            briefing_action(Alliance, 0x13),
            mark(MarkTarget::ObjectSystem(0x3200_0242), GidMode::MarkedAllianceSystem, 0x1626)
        );
        for code in [0xa, 0xc, 0xd] {
            assert_eq!(briefing_action(Alliance, code), None);
        }
    }

    #[test]
    fn every_briefing_caption_has_its_textstra_text() {
        for (id, text) in [
            (0x1622, "Yavin"),
            (0x1623, "Alliance Headquarters"),
            (0x1624, "Coruscant"),
            (0x1625, "Mon Mothma"),
            (0x1626, "Luke Skywalker"),
            (0x1627, "Emperor Palpatine"),
            (0x1628, "Darth Vader"),
        ] {
            assert_eq!(caption_text(id), Some(text));
        }
        assert_eq!(caption_text(0x1620), None);
    }
}
