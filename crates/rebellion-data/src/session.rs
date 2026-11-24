//! The app's session as a replay: the state it started from and the
//! player's commands, each with its day and its place in the day
//! (`docs/plans/2026-11-24-player-orders-in-replay.md`, stage 2).
//!
//! The app gives an order in its frame step, between the day's AI passes
//! (sub-ticks), or between days; a speed change sets how many passes the
//! next day has. [`SessionLog::replay`] runs the same days with the app's
//! spread passes and puts each command back at its place, so the result
//! can be compared with the app's own state.
//!
//! port: the session log is the port's own tool; nothing in the original
//! keeps one.

use serde::{Deserialize, Serialize};

use crate::player_command::{CommandIdError, PlayerCommand};
use crate::replay::{CampaignDayRunner, ReplayActor, ReplayCommand};
use crate::save::SaveState;

/// Where in the game's time a command was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum OrderMoment {
    /// After the day closed, before the next begins.
    BetweenDays,
    /// During an open day, after this many of its AI passes.
    InDay { passes_done: u32 },
}

/// One recorded command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionEntry {
    pub tick: u64,
    pub moment: OrderMoment,
    pub actor: ReplayActor,
    pub command: ReplayCommand,
}

/// The session since the last new game or load.
#[derive(Debug, Clone, Default)]
pub struct SessionLog {
    start: Option<SaveState>,
    entries: Vec<SessionEntry>,
    unrecorded: u32,
}

impl SessionLog {
    /// Start over from `start` (a new game or a loaded save, between days).
    pub fn restart(&mut self, start: SaveState) {
        self.start = Some(start);
        self.entries.clear();
        self.unrecorded = 0;
    }

    /// Forget the session until the next [`Self::restart`]: orders given
    /// meanwhile are not kept.
    pub fn clear(&mut self) {
        self.start = None;
        self.entries.clear();
        self.unrecorded = 0;
    }

    /// Count an order the log cannot keep (one the replay has no command
    /// for yet, such as a bombardment or an assault).
    pub fn mark_unrecorded(&mut self) {
        if self.start.is_some() {
            self.unrecorded += 1;
        }
    }

    /// Record a player's order, or count it as unrecorded when one of its
    /// objects had no stable id.
    pub fn record_order(
        &mut self,
        tick: u64,
        moment: OrderMoment,
        is_alliance: bool,
        command: Result<PlayerCommand, CommandIdError>,
    ) {
        if self.start.is_none() {
            return;
        }
        match command {
            Ok(command) => self.entries.push(SessionEntry {
                tick,
                moment,
                actor: side(is_alliance),
                command: ReplayCommand::Player { command },
            }),
            Err(_) => self.unrecorded += 1,
        }
    }

    /// Record a game-speed change by the player.
    pub fn record_speed(
        &mut self,
        tick: u64,
        moment: OrderMoment,
        is_alliance: bool,
        speed: rebellion_core::tick::GameSpeed,
    ) {
        if self.start.is_none() {
            return;
        }
        self.entries.push(SessionEntry {
            tick,
            moment,
            actor: side(is_alliance),
            command: ReplayCommand::SetSpeed { speed },
        });
    }

    #[must_use]
    pub fn entries(&self) -> &[SessionEntry] {
        &self.entries
    }

    /// Orders given whose objects had no stable id, so the log misses them.
    #[must_use]
    pub fn unrecorded(&self) -> u32 {
        self.unrecorded
    }

    #[must_use]
    pub fn has_start(&self) -> bool {
        self.start.is_some()
    }

    /// Run the session again up to `end_tick` and `end_moment`: every day
    /// with the app's spread passes, each command at its place.
    ///
    /// # Errors
    /// No start, or a command that does not apply.
    pub fn replay(&self, end_tick: u64, end_moment: OrderMoment) -> anyhow::Result<CampaignDayRunner> {
        let start = self
            .start
            .clone()
            .ok_or_else(|| anyhow::anyhow!("the session has no start"))?;
        let mut runner = CampaignDayRunner::new(start);
        let between = |runner: &mut CampaignDayRunner, tick: u64| -> anyhow::Result<()> {
            for entry in self
                .entries
                .iter()
                .filter(|e| e.tick == tick && e.moment == OrderMoment::BetweenDays)
            {
                runner.apply_as(entry.actor, &entry.command)?;
            }
            Ok(())
        };
        let first = runner.tick();
        between(&mut runner, first)?;
        for day in first + 1..=end_tick {
            let orders: Vec<(u32, ReplayActor, ReplayCommand)> = self
                .entries
                .iter()
                .filter(|e| e.tick == day)
                .filter_map(|e| match e.moment {
                    OrderMoment::InDay { passes_done } => {
                        Some((passes_done, e.actor, e.command.clone()))
                    }
                    OrderMoment::BetweenDays => None,
                })
                .collect();
            let stop = match end_moment {
                OrderMoment::InDay { passes_done } if day == end_tick => Some(passes_done),
                _ => None,
            };
            runner.run_day_with_orders(&orders, stop)?;
            if stop.is_none() {
                between(&mut runner, day)?;
            }
        }
        Ok(runner)
    }
}

fn side(is_alliance: bool) -> ReplayActor {
    if is_alliance {
        ReplayActor::Alliance
    } else {
        ReplayActor::Empire
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_command::{after_order, dispatch_fleet, dispatch_fleet_command};
    use crate::simulation::{
        begin_simulation_day, close_simulation_day, run_ai_passes, NoPresentation, SliceRolls,
    };
    use rebellion_core::ids::{FleetKey, SystemKey};
    use rebellion_core::tick::GameSpeed;

    fn seed42() -> SaveState {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
        let world = crate::load_game_data_with_options(
            &data,
            &crate::replay_fixture::seed42_seed_options(),
        )
        .expect("load original campaign data");
        crate::replay_fixture::seed42_initial_state(world).unwrap()
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn the_log_keeps_orders_only_with_a_start() {
        let mut log = SessionLog::default();
        assert!(!log.has_start());
        log.record_speed(0, OrderMoment::BetweenDays, true, GameSpeed::Fast);
        log.record_order(0, OrderMoment::BetweenDays, true, Err(CommandIdError::Unnamed));
        log.mark_unrecorded();
        assert!(log.entries().is_empty());
        assert_eq!(log.unrecorded(), 0);
        assert!(log.replay(0, OrderMoment::BetweenDays).is_err(), "no start");

        log.restart(seed42());
        assert!(log.has_start());
        log.record_speed(0, OrderMoment::BetweenDays, false, GameSpeed::Fast);
        log.record_order(0, OrderMoment::BetweenDays, true, Err(CommandIdError::Unnamed));
        log.mark_unrecorded();
        log.mark_unrecorded();
        assert_eq!(
            log.entries(),
            &[SessionEntry {
                tick: 0,
                moment: OrderMoment::BetweenDays,
                actor: ReplayActor::Empire,
                command: ReplayCommand::SetSpeed { speed: GameSpeed::Fast },
            }]
        );
        assert_eq!(log.unrecorded(), 3);
        log.clear();
        assert!(!log.has_start());
        assert!(log.entries().is_empty());
        assert_eq!(log.unrecorded(), 0);
    }

    /// An "app" run without presentation: spread passes, a fleet move
    /// given after `pass` passes of day `day` (or after it closed when
    /// `pass` is `None`), logged with its moment.
    fn run_with_order(initial: &SaveState, days: u64, day: u64, pass: Option<u32>) -> (u64, SessionLog) {
        let mut log = SessionLog::default();
        log.restart(initial.clone());
        log.record_speed(initial.clock.tick, OrderMoment::BetweenDays, true, GameSpeed::Medium);
        let mut fleets: Vec<FleetKey> = initial
            .world
            .fleets
            .iter()
            .filter(|(_, f)| f.is_alliance && f.ship_count() > 0)
            .map(|(k, _)| k)
            .collect();
        fleets.sort_by_key(|&f| initial.world.fleet_serial(f));
        let mut systems: Vec<(u32, SystemKey)> =
            initial.world.systems.iter().map(|(k, s)| (s.dat_id.0, k)).collect();
        systems.sort_unstable();
        let mut today = 0u64;
        let fingerprint = crate::replay::drive_days(
            initial.clone(),
            GameSpeed::Medium,
            false,
            days,
            &mut |world, sim, ticks, rolls| {
                today += 1;
                let mut order = |world: &mut rebellion_core::world::GameWorld,
                                 sim: &mut crate::simulation::SimulationStates,
                                 moment| {
                    let (fleet, to) = fleets
                        .iter()
                        .flat_map(|&f| systems.iter().map(move |&(_, s)| (f, s)))
                        .find(|&(f, s)| {
                            rebellion_core::movement::validate_fleet_dispatch(&sim.movement, world, f, s, true)
                                .is_ok()
                        })
                        .unwrap();
                    let command = dispatch_fleet_command(world, fleet, to, &[]);
                    dispatch_fleet(world, &mut sim.movement, &mut sim.troop_transport, fleet, to, &[], true)
                        .unwrap();
                    after_order(world, &sim.manufacturing, &sim.troop_transport);
                    log.record_order(sim.clock.tick, moment, true, command);
                };
                let mut integrator = crate::integrator::PerceptionIntegrator::new(ticks[0].tick, 0);
                begin_simulation_day(world, sim, ticks, rolls, &mut integrator);
                if today == day && pass == Some(0) {
                    order(world, sim, OrderMoment::InDay { passes_done: 0 });
                }
                for upto in 1..=sim.ai_day.passes {
                    run_ai_passes(world, sim, &mut integrator, &mut NoPresentation, upto);
                    if today == day && pass == Some(upto) {
                        order(world, sim, OrderMoment::InDay { passes_done: upto });
                    }
                }
                let cursor = sim.ai_day.roll_cursor as usize;
                close_simulation_day(world, sim, &mut SliceRolls::at(rolls, cursor), &mut integrator, &mut NoPresentation);
                if today == day && pass.is_none() {
                    order(world, sim, OrderMoment::BetweenDays);
                }
            },
        )
        .unwrap();
        (fingerprint.value, log)
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_session_puts_orders_at_their_moment() {
        let initial = seed42();
        let days = 6;
        for pass in [Some(0), Some(2), None] {
            let (app, log) = run_with_order(&initial, days, 2, pass);
            assert_eq!(log.entries().len(), 2);
            let replayed = log.replay(days, OrderMoment::BetweenDays).unwrap();
            assert_eq!(replayed.fingerprint().unwrap().value, app, "order at {pass:?}");
            assert_eq!(replayed.tick(), days);
        }
        // Stopped in the open last day, after its second pass.
        let (_, log) = run_with_order(&initial, days, 2, Some(2));
        let open = log.replay(days, OrderMoment::InDay { passes_done: 2 }).unwrap();
        let closed = log.replay(days, OrderMoment::BetweenDays).unwrap();
        assert_eq!(open.tick(), days);
        // Only the last day stops: the days before close as in the full run.
        let mut by_hand = log.replay(days - 1, OrderMoment::BetweenDays).unwrap();
        by_hand.run_day_with_orders(&[], Some(2)).unwrap();
        assert_eq!(open.fingerprint().unwrap(), by_hand.fingerprint().unwrap());
        assert_ne!(open.fingerprint().unwrap(), closed.fingerprint().unwrap(), "the day stays open");
    }
}
