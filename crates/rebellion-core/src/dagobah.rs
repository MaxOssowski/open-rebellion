//! Luke's Dagobah mission: timer `0x392`, the Dagobah check and the end of
//! the training (`ghidra/notes/dagobah.md`).
//!
//! The mission manager's setup (`FUN_00549d60`) arms timer `0x392` with
//! GNPRTB 6146 (300) + rand(0..=6147 (100)) days. Its fire sets
//! `LukeDagobahRequired` (manager `+0x58` bit 0, `FUN_00543af0`), whose hook
//! disarms the timer (`FUN_0054a800`). The story check `FUN_00544030` then
//! pulses `LukeDagobah` (bit 2, notification `0x221`) once Luke is free and
//! Han is not held at Jabba's palace, and that hook (`FUN_0054a860`) creates
//! the Dagobah mission for Luke. The requirement clears when the mission
//! ends after phase 8 (`FUN_00548840`), which is also when the mission sets
//! Luke's `DagobahCompleted` (`FUN_005751d0`).

use serde::{Deserialize, Serialize};

use crate::ids::{AbodeKey, CharacterKey, SystemKey};
use crate::missions::{MissionEffect, MissionState};
use crate::tick::TickEvent;
use crate::world::GameWorld;

/// GNPRTB 6146 (`DAT_006bb560`) = 300: timer `0x392`'s minimum.
const GNPRTB_DAGOBAH_MIN: u16 = 6146;
/// GNPRTB 6147 (`DAT_006bb5bc`) = 100: its spread.
const GNPRTB_DAGOBAH_SPREAD: u16 = 6147;
/// GNPRTB 6174 (`DAT_006bb59c`) = 60: the Force percent a successful
/// Dagobah adds (`FUN_0055ca60`).
const GNPRTB_DAGOBAH_SUCCESS_SHARE: u16 = 6174;
/// The shipped GNPRTB 6174.
const SHIPPED_DAGOBAH_SUCCESS_SHARE: i64 = 60;
/// GNPRTB 6175 (`DAT_006bb514`) = 2: the days per Force percent of an
/// unsuccessful one (`FUN_0055ca90`).
const GNPRTB_DAGOBAH_DAYS_PER_PERCENT: u16 = 6175;
/// The shipped GNPRTB 6175.
const SHIPPED_DAGOBAH_DAYS_PER_PERCENT: i64 = 2;

/// Luke's record (`0x32000242`), which `FUN_005434b0` and `FUN_005751d0`
/// look up.
const LUKE: u32 = 0x242;
/// The Dagobah abode (`0x98000441`), `FUN_0054a860`'s target.
const DAGOBAH: u32 = 0x441;

/// The mission manager's Dagobah state (save v73).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DagobahState {
    /// Whether the setup armed timer `0x392` (the first advance does).
    pub armed: bool,
    /// The day timer `0x392` fires, while armed.
    pub timer_due: Option<u64>,
    /// `LukeDagobahRequired`, manager `+0x58` bit 0.
    pub required: bool,
}

/// A `LukeDagobah` pulse: Luke should leave for Dagobah from `from`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LukeDagobah {
    pub luke: CharacterKey,
    /// Luke's system, the mission's starting place.
    pub from: SystemKey,
    pub abode: AbodeKey,
    pub tick: u64,
}

fn luke(world: &GameWorld) -> Option<CharacterKey> {
    world
        .characters
        .iter()
        .find(|(_, c)| c.dat_id.index() == LUKE && !c.is_killed)
        .map(|(key, _)| key)
}

/// Stateless system for the Dagobah trigger.
pub struct DagobahSystem;

impl DagobahSystem {
    /// The rolls [`Self::advance`] draws: the timer's delay.
    ///
    /// port: the caller draws the rolls up front, as for the other timers.
    #[must_use]
    pub fn roll_budget() -> usize {
        1
    }

    /// Arm timer `0x392` on the first advance, fire it, keep
    /// `LukeDagobahRequired`, and run the Dagobah check `FUN_00544030`.
    ///
    /// port: the original runs the check from the mission manager's
    /// handlers (`FUN_00549910`, on many events); the port runs it each day.
    /// `HanCapturedAtPalace` (`+0x58` bit `0x40`) is derived each day
    /// ([`crate::jabba::han_captured_at_palace`]).
    pub fn advance(
        state: &mut DagobahState,
        world: &GameWorld,
        missions: &MissionState,
        tick_events: &[TickEvent],
        rolls: &[f64],
    ) -> Option<LukeDagobah> {
        let tick = tick_events.last()?.tick;
        let param = |id| world.gnprtb.value(id, world.difficulty_index);
        if !state.armed {
            state.armed = true;
            let mut rolls = crate::uprising::Rolls::new(rolls);
            state.timer_due = Some(
                tick + crate::uprising::timer_delay(
                    param(GNPRTB_DAGOBAH_MIN),
                    param(GNPRTB_DAGOBAH_SPREAD),
                    &mut rolls,
                ),
            );
        }
        if state.timer_due.is_some_and(|due| due <= tick) {
            state.required = true;
            state.timer_due = None;
        }
        let luke = luke(world)?;
        let c = &world.characters[luke];
        // FUN_00548840 clears the requirement where FUN_005751d0 sets
        // DagobahCompleted, at the mission's end after phase 8.
        if c.dagobah_completed {
            state.required = false;
        }
        if !state.required || crate::jabba::han_captured_at_palace(world, missions) {
            return None;
        }
        // FUN_005439b0: not captured, usable, not autorouting, not adrift,
        // not on a mission; FUN_005434b0 needs a location.
        let from = c
            .current_fleet
            .and_then(|fleet| world.fleets.get(fleet))
            .map(|fleet| fleet.location)
            .or(c.current_system)?;
        if c.is_captive || c.on_mission || c.on_mandatory_mission || missions.is_autorouting(luke) {
            return None;
        }
        let abode = world
            .abodes
            .iter()
            .find(|(_, a)| a.dat_id.index() == DAGOBAH)
            .map(|(key, _)| key)?;
        Some(LukeDagobah {
            luke,
            from,
            abode,
            tick,
        })
    }
}

/// `FUN_0054a860`: create the Dagobah mission (record `0x71000043`, target
/// the abode `0x98000441`, member Luke). Returns its id, or `None` when the
/// mission manager refuses it.
pub fn dispatch(
    missions: &mut MissionState,
    world: &mut GameWorld,
    pulse: &LukeDagobah,
) -> Option<u64> {
    let request = crate::missions::MissionRequest::single(
        crate::missions::MissionKind::Dagobah,
        crate::missions::MissionFaction::Alliance,
        pulse.luke,
        pulse.from,
        None,
        pulse.tick,
    );
    let id = missions.dispatch_guarded(request, world).ok()?;
    missions.aim_at_abode(id, pulse.abode);
    Some(id)
}

/// `FUN_005751d0` in phase `0xb` after phase 8: with a first training day,
/// Luke's Force becomes Force + Force x GNPRTB 6174 / 100 on a success
/// (`FUN_0055ca60`), else Force + Force x ((today - first day) / GNPRTB
/// 6175) / 100 (`FUN_0055ca90`); the rise goes to `ForceExperience` and
/// `DagobahCompleted` is set (`FUN_0056f730`).
#[must_use]
pub fn training_end(
    world: &GameWorld,
    first_training_day: Option<u64>,
    succeeded: bool,
    today: u64,
) -> Vec<MissionEffect> {
    let (Some(first), Some(luke)) = (first_training_day, luke(world)) else {
        return Vec::new();
    };
    let force = i64::from(world.characters[luke].force_level());
    let percent = if succeeded {
        crate::movement::gnprtb_or_shipped(
            world,
            GNPRTB_DAGOBAH_SUCCESS_SHARE,
            SHIPPED_DAGOBAH_SUCCESS_SHARE,
        )
    } else {
        let per = crate::movement::gnprtb_or_shipped(
            world,
            GNPRTB_DAGOBAH_DAYS_PER_PERCENT,
            SHIPPED_DAGOBAH_DAYS_PER_PERCENT,
        )
        .max(1);
        i64::try_from(today.saturating_sub(first)).unwrap_or(i64::MAX) / per
    };
    let gain = i32::try_from(force * percent / 100).unwrap_or(i32::MAX);
    vec![MissionEffect::DagobahCompleted {
        character: luke,
        force_gain: gain,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::Faction;
    use crate::ids::DatId;
    use crate::world::{Abode, Character, ControlKind, GnprtbEntry, GnprtbParams, System};

    /// The shipped GNPRTB 6146/6147, an Alliance system, Luke there and the
    /// Dagobah abode.
    fn setup() -> (GameWorld, CharacterKey, SystemKey) {
        let mut world = GameWorld {
            gnprtb: GnprtbParams::new(
                [(6146, 300), (6147, 100)]
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
        let home = world.systems.insert(System {
            dat_id: DatId::new(0x9000_0001),
            name: "Yavin".into(),
            sector: crate::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
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
            control: ControlKind::Controlled(Faction::Alliance),
        });
        let luke = world.characters.insert(Character {
            dat_id: DatId::new(LUKE),
            is_alliance: true,
            recruited: true,
            current_system: Some(home),
            ..Character::default()
        });
        world.abodes.insert(Abode {
            dat_id: DatId::new(DAGOBAH),
            name: "Dagobah".into(),
            side: Faction::Neutral,
            special_forces: Vec::new(),
        });
        (world, luke, home)
    }

    fn day(
        state: &mut DagobahState,
        world: &GameWorld,
        tick: u64,
        roll: f64,
    ) -> Option<LukeDagobah> {
        DagobahSystem::advance(
            state,
            world,
            &MissionState::new(),
            &[TickEvent { tick }],
            &[roll],
        )
    }

    #[test]
    fn timer_0x392_requires_dagobah_once_300_to_400_days_in() {
        // FUN_00549d60 arms 0x392 with GNPRTB 6146 (300) + rand(0..=6147
        // (100)); its fire sets LukeDagobahRequired and its hook disarms it.
        let (world, luke, home) = setup();
        let mut state = DagobahState::default();
        assert_eq!(day(&mut state, &world, 1, 0.5), None);
        assert_eq!(state.timer_due, Some(351));
        assert_eq!(day(&mut state, &world, 350, 0.0), None);
        assert!(!state.required);
        let pulse = day(&mut state, &world, 351, 0.0).expect("LukeDagobah");
        assert_eq!((pulse.luke, pulse.from, pulse.tick), (luke, home, 351));
        assert!(state.required && state.timer_due.is_none());
        // The draw spans 0..=100.
        let mut late = DagobahState::default();
        day(&mut late, &world, 1, 0.999);
        assert_eq!(late.timer_due, Some(401));
    }

    #[test]
    fn luke_leaves_only_when_free_and_until_his_training_is_done() {
        // FUN_005439b0: not captured, not on a mission, not autorouting;
        // FUN_005434b0: a location. FUN_00548840 clears the requirement when
        // the mission ends after phase 8, as DagobahCompleted is set.
        let (mut world, luke, _) = setup();
        let mut state = DagobahState {
            armed: true,
            timer_due: None,
            required: true,
        };
        let busy: [fn(&mut Character); 4] = [
            |c| c.is_captive = true,
            |c| c.on_mission = true,
            |c| c.on_mandatory_mission = true,
            |c| c.current_system = None,
        ];
        for edit in busy {
            let mut w = world.clone();
            edit(&mut w.characters[luke]);
            assert_eq!(day(&mut state, &w, 5, 0.0), None);
            assert!(state.required);
        }
        assert!(day(&mut state, &world, 5, 0.0).is_some());
        // Autorouting (+0x50 bit 0x1000) holds him back too.
        let mut routing = MissionState::new();
        routing.push_escapee(crate::missions::EscapeeTransit {
            character: luke,
            to: world.characters[luke].current_system.unwrap(),
            arrival: 9,
            abode: None,
        });
        assert_eq!(
            DagobahSystem::advance(
                &mut state,
                &world,
                &routing,
                &[TickEvent { tick: 5 }],
                &[0.0]
            ),
            None
        );
        // A killed Luke is no Luke (FUN_005434b0 finds none).
        let mut dead = world.clone();
        dead.characters[luke].is_killed = true;
        assert_eq!(day(&mut state, &dead, 5, 0.0), None);
        assert_eq!(DagobahSystem::roll_budget(), 1);
        world.characters[luke].dagobah_completed = true;
        assert_eq!(day(&mut state, &world, 6, 0.0), None);
        assert!(!state.required);
        // No Dagobah abode, no pulse.
        let (mut world, _, _) = setup();
        world.abodes.clear();
        let mut state = DagobahState {
            armed: true,
            timer_due: None,
            required: true,
        };
        assert_eq!(day(&mut state, &world, 5, 0.0), None);
    }

    #[test]
    fn luke_stays_while_han_is_held_at_jabba_s_palace() {
        // FUN_00544030 needs HanCapturedAtPalace (+0x58 bit 0x40) clear.
        let (mut world, luke, _) = setup();
        let mut state = DagobahState {
            armed: true,
            timer_due: None,
            required: true,
        };
        assert!(day(&mut state, &world, 5, 0.0).is_some());
        let palace = world.abodes.insert(crate::world::Abode {
            dat_id: crate::ids::DatId::new(0x481),
            name: "Jabba's Palace".into(),
            side: crate::dat::Faction::Empire,
            special_forces: Vec::new(),
        });
        let han = world.characters.insert(Character {
            dat_id: crate::ids::DatId::new(0x243),
            is_alliance: true,
            is_captive: true,
            current_abode: Some(palace),
            ..Character::default()
        });
        assert_eq!(day(&mut state, &world, 5, 0.0), None);
        assert!(state.required);
        world.characters[han].is_captive = false;
        assert_eq!(day(&mut state, &world, 5, 0.0).map(|p| p.luke), Some(luke));
    }
}
