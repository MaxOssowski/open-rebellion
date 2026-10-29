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
        }
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
                self.idle_seconds -= idle_length;
                self.enabled = true;
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
                self.queue_message(id as u32, player);
                return;
            }
        }
        self.enabled = false;
    }

    /// `FUN_0041d660` -> `FUN_0042b1d0` for each entry: the record comes
    /// from the briefing DLL with flag bit 1, else the sprite DLL; a and b
    /// replace its words 2 and 3 when not 0. Advice is on in a new game
    /// (`0x1000`, `FUN_004c27f0`), so bit-0 entries play too.
    fn queue_message(&self, id: u32, player: &mut ClipPlayer) {
        let Some(entries) = self.messages.get(&id) else {
            return;
        };
        for entry in entries {
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
    use BriefingAction::{Display, Mark};
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
