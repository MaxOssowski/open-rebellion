//! When the AI players think: one tick per side per scheduler sub-tick.
//!
//! The frame loop `FUN_0041dff0` runs one sub-tick per pass of its phases 0
//! to 13. Phase 5 serves the due timers, so every timer due on a day fires
//! in the day's first sub-tick, ahead of that sub-tick's AI tick (phase 8,
//! `FUN_0041cf20` -> `FUN_004361a0`). Phase 8 ticks the player manager's
//! `+0x10` (the Alliance) and then its `+0x14` (the Empire). An order an AI
//! tick submits (`FUN_0048aa90` -> `FUN_0041cee0` -> `FUN_004360f0`) is
//! stored in its player's order table and reaches the scheduler in phase 3
//! of a later sub-tick, which runs it in phase 5. So both sides' orders of
//! sub-tick `k` run before sub-tick `k + 1`, and neither side's tick `k`
//! sees the other's orders of tick `k`. A day has the speed's rate plus one
//! sub-ticks (`FUN_0051dea0`; [`crate::ai::passes_per_day`]).
//! `ghidra/notes/ai-scheduling.md` has the trace.
//!
//! [`AiDay`] is the day's place in its passes; [`run_pass`] runs one pass
//! through an [`AiPassHost`], which owns the world and says how orders are
//! applied and what follows them. The headless runner runs a day's passes
//! in one burst; the interactive app runs them one sub-tick at a time.

use serde::{Deserialize, Serialize};

use crate::ai::{AIAction, AiFaction};
use crate::loyalty::LoyaltyWatch;

/// One of the two AI players a host can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiSlot {
    /// The runner's first AI (the playtest's `ai`, the app's opponent).
    Primary,
    /// The dual-AI mode's second AI.
    Secondary,
}

/// What a host does for the AI's passes.
pub trait AiPassHost {
    /// The side the AI in `slot` plays, or `None` when the host has none.
    fn faction(&self, slot: AiSlot) -> Option<AiFaction>;

    /// The AI tick (`FUN_00485b40`) of `slot` against the world as it
    /// stands, returning its orders without applying them.
    fn think(&mut self, slot: AiSlot, day: u64) -> Vec<AIAction>;

    /// Carry out `slot`'s orders (phase 5 of the next sub-tick).
    fn apply(&mut self, slot: AiSlot, actions: &[AIAction], day: u64);

    /// After a pass whose orders were applied: what the original's event
    /// handlers keep current before the next sub-tick. Returns whether the
    /// game is over, which holds the day's remaining passes
    /// (`FUN_0041e390` -> `FUN_0051de80`).
    fn settle(&mut self, day: u64) -> bool;
}

/// The slots `host` holds, in the order phase 8 ticks them: the Alliance's
/// player (`+0x10`) first, then the Empire's (`+0x14`) (`FUN_004361a0`).
#[must_use]
pub fn tick_order(host: &dyn AiPassHost) -> Vec<AiSlot> {
    let mut slots: Vec<(AiSlot, AiFaction)> = [AiSlot::Primary, AiSlot::Secondary]
        .into_iter()
        .filter_map(|slot| host.faction(slot).map(|side| (slot, side)))
        .collect();
    slots.sort_by_key(|&(_, side)| side != AiFaction::Alliance);
    slots.into_iter().map(|(slot, _)| slot).collect()
}

/// One sub-tick's AI pass: each side thinks against the same world, then
/// both sides' orders are applied, the Alliance's first, before the next
/// pass (`FUN_004361a0`; the order table's phase 3 and 5). Returns whether
/// the game ended.
pub fn run_pass(host: &mut dyn AiPassHost, day: u64) -> bool {
    let held: Vec<(AiSlot, Vec<AIAction>)> = tick_order(host)
        .into_iter()
        .map(|slot| (slot, host.think(slot, day)))
        .collect();
    let mut applied = false;
    for (slot, actions) in &held {
        host.apply(*slot, actions, day);
        applied |= !actions.is_empty();
    }
    applied && host.settle(day)
}

/// The current day's AI passes: how many it has, how many have run, and
/// what the end of the day still owes. It is saved, so a game saved within
/// a day resumes at its next pass (v132).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiDay {
    /// The day the passes belong to.
    pub day: u64,
    /// The passes the day has: its speed's sub-ticks
    /// ([`crate::ai::passes_per_day`]), fixed when the day begins.
    pub passes: u32,
    /// The passes already run.
    pub done: u32,
    /// The bombardment stream's state between passes
    /// ([`crate::bombardment::BombardEnv::rng_state`]): one seed a day.
    pub bombard_state: u64,
    /// The galaxy the end of the day's loyalty shifts compare against; set
    /// while the end of the day is owed.
    pub loyalty_before: Option<LoyaltyWatch>,
    /// The days the end of the day closes (a frame or a call can run more
    /// than one; the player's agent runs once for each).
    pub days: u32,
    /// port: the headless runner's place in its day's rolls, for the end of
    /// the day.
    pub roll_cursor: u32,
}

impl AiDay {
    /// Begin `day` with `passes` passes. The end of the day is owed until
    /// [`Self::close`].
    #[must_use]
    pub const fn begin(
        day: u64,
        passes: u32,
        bombard_state: u64,
        loyalty_before: LoyaltyWatch,
        days: u32,
    ) -> Self {
        Self {
            day,
            passes,
            done: 0,
            bombard_state,
            loyalty_before: Some(loyalty_before),
            days,
            roll_cursor: 0,
        }
    }

    /// Whether every pass of the day has run (or a win held the rest).
    #[must_use]
    pub const fn passes_done(&self) -> bool {
        self.done >= self.passes
    }

    /// Whether the day's passes are done and its end is still owed.
    #[must_use]
    pub const fn owes_close(&self) -> bool {
        self.passes_done() && self.loyalty_before.is_some()
    }

    /// The passes due once `part` of the day (`0..1`) has elapsed: the
    /// first sub-tick's at once, then one as each later sub-tick begins.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "the day's part is in [0, 1), so the sub-tick fits the day's passes"
    )]
    pub fn due(&self, part: f32) -> u32 {
        let started = (part.clamp(0.0, 1.0) * self.passes as f32).floor() as u32;
        started.saturating_add(1).min(self.passes)
    }

    /// Run passes through `host` until `upto` have run (at most the day's).
    /// A pass that ends the game holds the rest. Returns whether the game
    /// ended.
    pub fn run_until(&mut self, host: &mut dyn AiPassHost, upto: u32) -> bool {
        while self.done < upto.min(self.passes) {
            self.done += 1;
            if run_pass(host, self.day) {
                self.done = self.passes;
                return true;
            }
        }
        false
    }

    /// Take what the end of the day needs and mark it paid: the loyalty
    /// watch and the days it closes. `None` when nothing is owed or passes
    /// are still to run.
    pub fn close(&mut self) -> Option<(LoyaltyWatch, u32)> {
        if !self.passes_done() {
            return None;
        }
        self.loyalty_before.take().map(|watch| (watch, self.days))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A host that records what it is asked, with two sides whose ticks
    /// return the orders queued for them.
    struct Recorder {
        sides: [Option<AiFaction>; 2],
        log: Vec<String>,
        orders: Vec<AIAction>,
        win_on_settle: bool,
    }

    impl Recorder {
        fn new(primary: Option<AiFaction>, secondary: Option<AiFaction>) -> Self {
            Self {
                sides: [primary, secondary],
                log: Vec::new(),
                orders: Vec::new(),
                win_on_settle: false,
            }
        }
    }

    fn index(slot: AiSlot) -> usize {
        match slot {
            AiSlot::Primary => 0,
            AiSlot::Secondary => 1,
        }
    }

    impl AiPassHost for Recorder {
        fn faction(&self, slot: AiSlot) -> Option<AiFaction> {
            self.sides[index(slot)]
        }
        fn think(&mut self, slot: AiSlot, day: u64) -> Vec<AIAction> {
            self.log.push(format!("think {slot:?} {day}"));
            self.orders.clone()
        }
        fn apply(&mut self, slot: AiSlot, actions: &[AIAction], _day: u64) {
            self.log.push(format!("apply {slot:?} {}", actions.len()));
        }
        fn settle(&mut self, _day: u64) -> bool {
            self.log.push("settle".into());
            self.win_on_settle
        }
    }

    fn an_order() -> AIAction {
        AIAction::AbortMission { mission_id: 1 }
    }

    // FUN_004361a0 ticks the manager's +0x10 (side 1, the Alliance;
    // FUN_004365d0) before its +0x14.
    #[test]
    fn the_alliance_ticks_before_the_empire_whichever_slot_holds_it() {
        let mut host = Recorder::new(Some(AiFaction::Empire), Some(AiFaction::Alliance));
        assert_eq!(tick_order(&host), [AiSlot::Secondary, AiSlot::Primary]);
        host.sides = [Some(AiFaction::Alliance), Some(AiFaction::Empire)];
        assert_eq!(tick_order(&host), [AiSlot::Primary, AiSlot::Secondary]);
        host.sides = [Some(AiFaction::Empire), None];
        assert_eq!(tick_order(&host), [AiSlot::Primary]);
    }

    // FUN_0048aa90 stores an order for phase 3/5 of a later sub-tick, so
    // both ticks of a pass run before either side's orders apply.
    #[test]
    fn both_sides_think_before_either_side_s_orders_apply() {
        let mut host = Recorder::new(Some(AiFaction::Empire), Some(AiFaction::Alliance));
        host.orders = vec![an_order()];
        assert!(!run_pass(&mut host, 7));
        assert_eq!(
            host.log,
            [
                "think Secondary 7",
                "think Primary 7",
                "apply Secondary 1",
                "apply Primary 1",
                "settle",
            ]
        );
    }

    #[test]
    fn a_pass_without_orders_settles_nothing() {
        let mut host = Recorder::new(Some(AiFaction::Empire), None);
        host.win_on_settle = true;
        assert!(!run_pass(&mut host, 1));
        assert_eq!(host.log, ["think Primary 1", "apply Primary 0"]);
    }

    fn watch() -> LoyaltyWatch {
        LoyaltyWatch::default()
    }

    // FUN_0051dea0: a day has rate + 1 sub-ticks, one AI tick each.
    #[test]
    fn a_day_runs_rate_plus_one_passes_one_per_sub_tick() {
        let passes = crate::ai::passes_per_day(crate::tick::GameSpeed::Medium);
        assert_eq!(passes, 13);
        let mut day = AiDay::begin(4, passes, 0, watch(), 1);
        let mut host = Recorder::new(Some(AiFaction::Empire), None);
        // The first sub-tick's pass is due at once.
        assert_eq!(day.due(0.0), 1);
        day.run_until(&mut host, day.due(0.0));
        assert_eq!(day.done, 1);
        // Halfway through, sub-tick 6 has begun: passes 0..=6.
        day.run_until(&mut host, day.due(0.5));
        assert_eq!(day.done, 7);
        assert!(!day.passes_done());
        assert_eq!(day.close(), None);
        // The last sub-tick begins at 12/13 of the day.
        day.run_until(&mut host, day.due(12.0 / 13.0 + 1e-4));
        assert_eq!(day.done, 13);
        assert_eq!(host.log.len(), 2 * 13);
        assert_eq!(day.close(), Some((watch(), 1)));
        assert_eq!(day.close(), None);
    }

    /// A host whose AI reads one value of its world.
    struct Watcher {
        world: u32,
        seen: Vec<u32>,
    }

    impl AiPassHost for Watcher {
        fn faction(&self, slot: AiSlot) -> Option<AiFaction> {
            (slot == AiSlot::Primary).then_some(AiFaction::Empire)
        }
        fn think(&mut self, _slot: AiSlot, _day: u64) -> Vec<AIAction> {
            self.seen.push(self.world);
            Vec::new()
        }
        fn apply(&mut self, _slot: AiSlot, _actions: &[AIAction], _day: u64) {}
        fn settle(&mut self, _day: u64) -> bool {
            false
        }
    }

    // FUN_0041dff0: the player's orders go in between sub-ticks (phases 3
    // and 5), so the next sub-tick's AI tick (phase 8) reads them.
    #[test]
    fn a_player_order_given_mid_day_is_seen_by_the_next_ai_pass() {
        let mut day = AiDay::begin(4, 13, 0, watch(), 1);
        let mut host = Watcher {
            world: 0,
            seen: Vec::new(),
        };
        day.run_until(&mut host, day.due(0.0));
        // The player's order, between sub-ticks 0 and 1.
        host.world = 1;
        day.run_until(&mut host, day.due(1.0 / 13.0 + 1e-4));
        assert_eq!(host.seen, [0, 1]);
    }

    #[test]
    fn pausing_mid_day_holds_the_remaining_passes() {
        let mut day = AiDay::begin(4, 61, 0, watch(), 1);
        let mut host = Recorder::new(Some(AiFaction::Empire), None);
        day.run_until(&mut host, day.due(0.25));
        let held = day.done;
        // A held clock's part of the day does not move.
        day.run_until(&mut host, day.due(0.25));
        assert_eq!(day.done, held);
        assert_eq!(held, 16);
    }

    // FUN_0041e390 -> FUN_0051de80: the game-over bit holds the clock, so
    // no later sub-tick ticks the AI.
    #[test]
    fn a_pass_that_ends_the_game_holds_the_day_s_remaining_passes() {
        let mut day = AiDay::begin(4, 5, 0, watch(), 1);
        let mut host = Recorder::new(Some(AiFaction::Empire), None);
        host.orders = vec![an_order()];
        host.win_on_settle = true;
        assert!(day.run_until(&mut host, 5));
        assert_eq!(
            host.log.iter().filter(|l| l.starts_with("think")).count(),
            1
        );
        assert!(day.passes_done());
        assert!(day.owes_close());
    }

    #[test]
    fn a_finished_day_owes_nothing_by_default() {
        let day = AiDay::default();
        assert!(day.passes_done());
        assert!(!day.owes_close());
    }
}
