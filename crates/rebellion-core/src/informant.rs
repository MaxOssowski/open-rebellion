//! The informant incident: a held system's timer `0x38e`
//! (`ghidra/notes/informant-incident.md`).
//!
//! `FUN_0050c820` arms the timer on the system's record `+0x54`+`0x40`
//! while the system has a side (side bits not 3): the system's setup
//! (`FUN_0050e200`) and its side hook (`FUN_00510820`) call it. The record's
//! minimum is GNPRTB 7703 (30) and its spread 7704 (970), so it fires every
//! 30 to 1000 days, drawing a new delay each time.
//!
//! When it fires, `FUN_0050cbe0` rolls `FUN_00559f90` against the holder's
//! support (`FUN_00507270`): the incident happens when a percent draw is
//! not below it, so with `100 - support` percent. It then pulses system
//! `+0x88` bit 17 (`FUN_0050aac0`), whose hook (`FUN_00511860`) posts the
//! Informants message (notification `0x153`, TEXTSTRA 29008-29009) and
//! runs `FUN_0050d510`: INFORMTB looked up at GNPRTB 7709 (1) +
//! rand(0..=7710 (7)) (`FUN_00559db0`) chooses what of the system
//! `FUN_0050d5a0` passes to the other side.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::ids::SystemKey;
use crate::tick::TickEvent;
use crate::world::{GameWorld, MstbTable};

/// GNPRTB 7703 (`DAT_006bb3d0`) = 30: timer `0x38e`'s minimum
/// (`FUN_0050e200`, record `+0x54`+`0x40`).
const GNPRTB_INFORMANT_MIN: u16 = 7703;
/// GNPRTB 7704 (`DAT_006bb484`) = 970: its spread.
const GNPRTB_INFORMANT_SPREAD: u16 = 7704;
/// GNPRTB 7709 (`DAT_006bb3b0`) = 1: the INFORMTB draw's base
/// (`FUN_00559db0`).
const GNPRTB_INFORMTB_BASE: u16 = 7709;
/// GNPRTB 7710 (`DAT_006bb3ac`) = 7: the INFORMTB draw's spread.
const GNPRTB_INFORMTB_SPREAD: u16 = 7710;

/// Each held system's timer `0x38e`: the tick it next fires (save v66).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InformantState {
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub timers: HashMap<SystemKey, u64>,
}

/// An informant incident: the other side learns about `system`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformantReport {
    pub system: SystemKey,
    /// The side holding the system.
    pub holder: Faction,
    /// The side the informants tell (`FUN_0050d510`: side 1 tells 2, any
    /// other tells 1).
    pub recipient: Faction,
    /// The INFORMTB code: which of the system's objects and fields
    /// `FUN_0050d5a0` passes on. port: the port's fog knows systems, not
    /// their objects, so every code reveals the system.
    pub code: u32,
    pub tick: u64,
}

/// Caller-supplied `[0, 1)` rolls, consumed in order; 0 when exhausted.
struct Rolls<'a> {
    rolls: std::slice::Iter<'a, f64>,
}

impl Rolls<'_> {
    /// `FUN_0053e290(n)`: uniform over `0..=n`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap,
        reason = "the roll is in [0, 1), so the product is at most n"
    )]
    fn draw(&mut self, n: i32) -> i32 {
        let roll = self.rolls.next().copied().unwrap_or(0.0);
        let span = n.max(0) as u32;
        ((roll * f64::from(span + 1)) as u32).min(span) as i32
    }
}

/// Stateless system for timer `0x38e`.
pub struct InformantSystem;

impl InformantSystem {
    /// The most rolls [`Self::advance`] draws: one first delay per system,
    /// and three for each that fires (the support roll, the INFORMTB draw
    /// and the next delay).
    #[must_use]
    pub fn roll_budget(world: &GameWorld) -> usize {
        world.systems.len() * 4
    }

    /// Arm the timers of held systems, drop those of systems with no side,
    /// and fire the ones due by the last tick, in system order.
    pub fn advance(
        state: &mut InformantState,
        world: &GameWorld,
        tick_events: &[TickEvent],
        rolls: &[f64],
    ) -> Vec<InformantReport> {
        let Some(last) = tick_events.last() else {
            return Vec::new();
        };
        let tick = last.tick;
        let param = |id| world.gnprtb.value(id, world.difficulty_index);
        let mut rolls = Rolls {
            rolls: rolls.iter(),
        };
        let mut reports = Vec::new();
        let empty = MstbTable::new(Vec::new());
        let informtb = world.mission_tables.get("INFORMTB").unwrap_or(&empty);
        for (system, sys) in &world.systems {
            let Some(holder) = crate::uprising::holder(sys) else {
                // FUN_0050c820 arms with the new side bits 3: cancelled.
                state.timers.remove(&system);
                continue;
            };
            let delay = |rolls: &mut Rolls<'_>| {
                let days = param(GNPRTB_INFORMANT_MIN) + rolls.draw(param(GNPRTB_INFORMANT_SPREAD));
                u64::try_from(days.max(1)).unwrap_or(1)
            };
            let due = *state
                .timers
                .entry(system)
                .or_insert_with(|| tick + delay(&mut rolls));
            if due > tick {
                continue;
            }
            state.timers.insert(system, due + delay(&mut rolls));
            // FUN_00559f90: FUN_0053e2f0(support) fails, so draw >= support.
            let support = crate::uprising::support_points(sys, holder);
            if rolls.draw(99) < support {
                continue;
            }
            let at = param(GNPRTB_INFORMTB_BASE) + rolls.draw(param(GNPRTB_INFORMTB_SPREAD));
            reports.push(InformantReport {
                system,
                holder,
                recipient: if holder == Faction::Alliance {
                    Faction::Empire
                } else {
                    Faction::Alliance
                },
                code: informtb.step_lookup(at),
                tick,
            });
        }
        reports
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DatId;
    use crate::world::{ControlKind, GnprtbEntry, GnprtbParams, MstbEntry, System};

    /// One system held by `holder` with the holder's support at `support`
    /// percent, the shipped GNPRTB 7703/7704/7709/7710 and INFORMTB's rows.
    fn galaxy(holder: ControlKind, support: f32) -> (GameWorld, SystemKey) {
        let mut world = GameWorld {
            gnprtb: GnprtbParams::new(
                [(7703, 30), (7704, 970), (7709, 1), (7710, 7)]
                    .into_iter()
                    .map(|(parameter_id, value)| GnprtbEntry {
                        parameter_id,
                        development: value,
                        alliance_sp_easy: value,
                        alliance_sp_medium: value,
                        alliance_sp_hard: value,
                        empire_sp_easy: value,
                        empire_sp_medium: value,
                        empire_sp_hard: value,
                        multiplayer: value,
                    })
                    .collect(),
            ),
            ..GameWorld::default()
        };
        world.mission_tables.insert(
            "INFORMTB".into(),
            MstbTable::new(
                [
                    (1, 1),
                    (2, 2),
                    (3, 3),
                    (4, 4),
                    (5, 7),
                    (6, 8),
                    (7, 9),
                    (8, 10),
                ]
                .into_iter()
                .map(|(threshold, value)| MstbEntry { threshold, value })
                .collect(),
            ),
        );
        let alliance = holder == ControlKind::Controlled(Faction::Alliance);
        let system = world.systems.insert(System {
            dat_id: DatId::new(0x9000_0001),
            name: "Here".into(),
            sector: crate::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: if alliance { support } else { 1.0 - support },
            popularity_empire: if alliance { 1.0 - support } else { support },
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: vec![],
            is_headquarters: false,
            is_destroyed: false,
            control: holder,
        });
        (world, system)
    }

    fn day(
        state: &mut InformantState,
        world: &GameWorld,
        tick: u64,
        rolls: &[f64],
    ) -> Vec<InformantReport> {
        InformantSystem::advance(state, world, &[TickEvent { tick }], rolls)
    }

    #[test]
    fn a_held_system_s_timer_waits_thirty_to_a_thousand_days() {
        // FUN_0050c820 arms 0x38e for side bits other than 3; record
        // +0x54+0x40: GNPRTB 7703 (30) + rand(0..=7704 (970)).
        let (world, system) = galaxy(ControlKind::Controlled(Faction::Empire), 0.5);
        let mut state = InformantState::default();
        assert_eq!(day(&mut state, &world, 1, &[0.0]), vec![]);
        assert_eq!(state.timers[&system], 31);
        let mut state = InformantState::default();
        day(&mut state, &world, 1, &[0.999_999]);
        assert_eq!(state.timers[&system], 1001);
        // A neutral system has none, and losing the side drops it.
        let (mut neutral, other) = galaxy(ControlKind::Uncontrolled, 0.5);
        let mut state = InformantState::default();
        day(&mut state, &neutral, 1, &[0.0]);
        assert!(!state.timers.contains_key(&other));
        neutral.systems[other].control = ControlKind::Controlled(Faction::Alliance);
        day(&mut state, &neutral, 2, &[0.0]);
        assert_eq!(state.timers[&other], 32);
        neutral.systems[other].control = ControlKind::Uncontrolled;
        day(&mut state, &neutral, 3, &[0.0]);
        assert!(!state.timers.contains_key(&other));
    }

    #[test]
    fn informants_tell_the_other_side_with_a_hundred_less_support_percent() {
        // FUN_0050cbe0: FUN_00559f90 succeeds when FUN_0053e2f0(support)
        // fails, a draw of 0..=99 not below the Empire's 60; FUN_00559db0
        // looks INFORMTB up at 1 + rand(0..=7); FUN_0050d510 tells side 1.
        let (world, system) = galaxy(ControlKind::Controlled(Faction::Empire), 0.6);
        let fire = |support_roll: f64, table_roll: f64| {
            let mut state = InformantState::default();
            state.timers.insert(system, 5);
            day(&mut state, &world, 5, &[0.0, support_roll, table_roll])
        };
        // A draw of 59 stays below 60: nothing.
        assert_eq!(fire(0.59, 0.0), vec![]);
        // 60: the incident, INFORMTB row 1 (code 1).
        assert_eq!(
            fire(0.6, 0.0),
            vec![InformantReport {
                system,
                holder: Faction::Empire,
                recipient: Faction::Alliance,
                code: 1,
                tick: 5,
            }]
        );
        // Row 8 gives code 10.
        assert_eq!(fire(0.6, 0.999_999)[0].code, 10);
    }

    #[test]
    fn a_day_draws_within_its_roll_budget() {
        // One first delay per system, and a fire's support roll, INFORMTB
        // draw and next delay.
        let (world, system) = galaxy(ControlKind::Controlled(Faction::Empire), 0.0);
        assert_eq!(InformantSystem::roll_budget(&world), 4);
        let mut state = InformantState::default();
        let mut rolls = vec![0.0; 4];
        rolls[2] = 0.999_999;
        state.timers.insert(system, 1);
        // The table draw is the third roll: row 8, code 10.
        let reports = day(&mut state, &world, 1, &rolls);
        assert_eq!(reports[0].code, 10);
    }

    #[test]
    fn the_timer_repeats_from_the_day_it_fires() {
        // FUN_005862a0 reschedules from the fire with a fresh delay.
        let (world, system) = galaxy(ControlKind::Controlled(Faction::Alliance), 1.0);
        let mut state = InformantState::default();
        state.timers.insert(system, 5);
        assert_eq!(day(&mut state, &world, 4, &[]), vec![]);
        assert_eq!(state.timers[&system], 5);
        // Support 100: no incident. The new delay is 30 + 485 (0.5 of
        // 0..=970).
        assert_eq!(day(&mut state, &world, 5, &[0.5, 0.999]), vec![]);
        assert_eq!(state.timers[&system], 520);
    }
}
