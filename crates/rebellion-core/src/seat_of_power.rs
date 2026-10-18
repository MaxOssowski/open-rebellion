//! The character manager's `SeatOfPower` (`+0x58` bit 1,
//! `ghidra/notes/story-message-classes.md`, row 16).
//!
//! `FUN_005617b0` sets it while the Emperor stands free at an Empire-held
//! Coruscant; a change notifies `0x230` (`FUN_0055f650` -> slot `+0x1d8`,
//! `FUN_00562450`), and the Empire reads Emperor Arrives at Coruscant when
//! it is set (`FUN_0048ce80`).
//!
//! port: the original runs the check from the character manager's
//! handlers (`FUN_00560d50`, `FUN_00561340`, `FUN_005615b0`,
//! `FUN_005616a0`, `FUN_005620e0`); the port runs it each day.

use serde::{Deserialize, Serialize};

use crate::world::GameWorld;

/// The Emperor's record (`0x34000280`, `FUN_005070d0`).
const EMPEROR: u32 = 0x280;
/// Coruscant (`0x90000109`, `FUN_005617b0`).
const CORUSCANT: u32 = 0x109;

/// The character manager's `SeatOfPower` bit (save v78).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatOfPowerState {
    /// Whether the first check ran.
    pub armed: bool,
    /// `SeatOfPower`, character manager `+0x58` bit 1.
    pub held: bool,
}

/// `FUN_005617b0`: the Emperor is located, not captured (`+0xac` bit 0),
/// and stands in Coruscant itself (his container is `0x90000109`), whose
/// side (`+0x24 & 0xc0`) is the Empire's.
///
/// port: "usable" (`+0x50` bit 0) has no port field.
#[must_use]
pub fn seat_of_power(world: &GameWorld) -> bool {
    let Some(emperor) = world
        .characters
        .values()
        .find(|c| c.dat_id.index() == EMPEROR && !c.is_killed)
    else {
        return false;
    };
    if emperor.is_captive || emperor.current_fleet.is_some() || emperor.current_abode.is_some() {
        return false;
    }
    emperor
        .current_system
        .and_then(|key| world.systems.get(key))
        .is_some_and(|sys| {
            sys.dat_id.index() == CORUSCANT
                && crate::uprising::holder(sys) == Some(crate::dat::Faction::Empire)
        })
}

/// Stateless system for `SeatOfPower`.
pub struct SeatOfPowerSystem;

impl SeatOfPowerSystem {
    /// The day's check (`FUN_005617b0` -> `FUN_0055f650`). Returns the new
    /// value when it changed: `Some(true)` is notification `0x230` with its
    /// argument set, Emperor Arrives at Coruscant.
    ///
    /// hyp: the first check, at the game's start, stores the value
    /// silently, as the original's setup places the Emperor at Coruscant
    /// (`FUN_0055f7b0`) before play.
    pub fn advance(state: &mut SeatOfPowerState, world: &GameWorld) -> Option<bool> {
        let held = seat_of_power(world);
        if !state.armed {
            state.armed = true;
            state.held = held;
            return None;
        }
        if held == state.held {
            return None;
        }
        state.held = held;
        Some(held)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::Faction;
    use crate::ids::DatId;
    use crate::world::{Character, ControlKind, System};

    fn world() -> (GameWorld, crate::ids::CharacterKey, crate::ids::SystemKey) {
        let mut world = GameWorld::default();
        let coruscant = world.systems.insert(System {
            dat_id: DatId::new(CORUSCANT),
            name: "Coruscant".into(),
            sector: crate::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.0,
            popularity_empire: 1.0,
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
            is_headquarters: true,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Empire),
        });
        let emperor = world.characters.insert(Character {
            dat_id: DatId::new(EMPEROR),
            name: "Emperor Palpatine".into(),
            is_empire: true,
            current_system: Some(coruscant),
            ..Character::default()
        });
        (world, emperor, coruscant)
    }

    #[test]
    fn the_seat_of_power_needs_a_free_emperor_in_an_empire_held_coruscant() {
        // FUN_005617b0: not captured, his container Coruscant 0x90000109
        // itself, its side the Empire's (0x80).
        let (mut world, emperor, coruscant) = world();
        assert!(seat_of_power(&world));
        let unseated: [fn(&mut GameWorld, crate::ids::CharacterKey, crate::ids::SystemKey); 6] = [
            |w, e, _| w.characters[e].is_captive = true,
            |w, e, _| w.characters[e].is_killed = true,
            |w, e, _| w.characters[e].current_fleet = Some(crate::ids::FleetKey::default()),
            |w, e, _| w.characters[e].current_system = None,
            |w, _, c| w.systems[c].control = ControlKind::Controlled(Faction::Alliance),
            |w, _, c| w.systems[c].dat_id = DatId::new(0x10a),
        ];
        for (i, edit) in unseated.into_iter().enumerate() {
            let mut w = world.clone();
            edit(&mut w, emperor, coruscant);
            assert!(!seat_of_power(&w), "case {i}");
        }
        world.characters[emperor].current_abode = Some(crate::ids::AbodeKey::default());
        assert!(!seat_of_power(&world));
    }

    #[test]
    fn only_a_change_after_the_first_check_notifies() {
        // FUN_0055f650 calls slot +0x1d8 (0x230) when the bit changes; the
        // first check stores it silently (hyp).
        let (mut world, emperor, _) = world();
        let mut state = SeatOfPowerState::default();
        assert_eq!(SeatOfPowerSystem::advance(&mut state, &world), None);
        assert!(state.armed && state.held);
        assert_eq!(SeatOfPowerSystem::advance(&mut state, &world), None);
        world.characters[emperor].is_captive = true;
        assert_eq!(SeatOfPowerSystem::advance(&mut state, &world), Some(false));
        assert_eq!(SeatOfPowerSystem::advance(&mut state, &world), None);
        world.characters[emperor].is_captive = false;
        assert_eq!(SeatOfPowerSystem::advance(&mut state, &world), Some(true));
        assert!(state.held);
    }
}
