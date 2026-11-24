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
