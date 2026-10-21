//! The character manager's `SeatOfPower` (`+0x58` mask 1, bit 0,
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

use crate::world::{Character, GameWorld, Skill};

/// The Emperor's record (`0x34000280`, `FUN_005070d0`).
const EMPEROR: u32 = 0x280;
/// Coruscant (`0x90000109`, `FUN_005617b0`).
const CORUSCANT: u32 = 0x109;

/// The character manager's `SeatOfPower` bit (save v78).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatOfPowerState {
    /// Whether the first check ran.
    pub armed: bool,
    /// `SeatOfPower`, character manager `+0x58` mask 1 (`FUN_0055f650` ->
    /// `FUN_0053a640(1, ...)`).
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

/// GNPRTB 3076 (`Gnprtb_3076`): the seat of power's leadership bonus,
/// percent.
const GNPRTB_SEAT_BONUS: u16 = 3076;
/// GNPRTB 3076 as shipped (50 at every level).
const SHIPPED_SEAT_BONUS: i64 = 50;

/// `FUN_005618d0`, a character's `+0x92` (set through `FUN_004ee250`,
/// clamped to `0..=0x7fff`): GNPRTB 3076 while `SeatOfPower` holds and the
/// character's class is the Empire's alone (`FUN_004edc80` == 2: class
/// `+0x44` without `+0x40`), else 0. The manual's "Seat of Power" effect:
/// "all Imperial characters in the galaxy get a leadership bonus".
///
/// port: the original stores `+0x92` when `SeatOfPower` changes (slot
/// `+0x1d8`, `CharacterMgr_vt1d8`, over every character) and when a
/// character is placed (`FUN_00561470`); the port reads it from the world
/// as it stands, which is the same value without a save field. The class
/// sides are the port's `is_alliance` and `is_empire`, which a defection
/// rewrites where the class record would not.
#[must_use]
pub fn leadership_bonus(world: &GameWorld, character: &Character) -> u32 {
    if character.is_empire && !character.is_alliance && seat_of_power(world) {
        u32::try_from(crate::movement::gnprtb_or_shipped(
            world,
            GNPRTB_SEAT_BONUS,
            SHIPPED_SEAT_BONUS,
        ))
        .unwrap_or(0)
        .min(0x7fff)
    } else {
        0
    }
}

/// `FUN_004eede0`, a character's effective leadership `+0x88` (slot
/// `+0x1f4`): leadership plus leadership times `+0x92` over 100
/// (`FUN_0053e940`, `FUN_0053e190` truncating), clamped to `0..=0x7fff`
/// (`FUN_004edfb0`).
#[must_use]
pub fn effective_leadership(world: &GameWorld, character: &Character) -> u32 {
    let leadership = u64::from(character.effective_skill(Skill::Leadership));
    let bonus = u64::from(leadership_bonus(world, character));
    u32::try_from((leadership + leadership * bonus / 100).min(0x7fff)).unwrap_or(0x7fff)
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
            last_arrival: None,
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
    fn the_seat_of_power_adds_half_again_to_an_imperial_character_s_leadership() {
        // FUN_005618d0: +0x92 = G3076 (50) while SeatOfPower holds and the
        // class is the Empire's alone (FUN_004edc80 == 2), else 0;
        // FUN_004eede0: +0x88 = leadership + leadership * +0x92 / 100
        // (FUN_0053e940, truncating), at most 0x7fff (FUN_004edfb0). Manual,
        // "The Seat of Power Effect".
        let (mut world, emperor, _) = world();
        let character = |is_alliance, is_empire, leadership| Character {
            is_alliance,
            is_empire,
            leadership: crate::world::SkillPair {
                base: leadership,
                variance: 0,
            },
            ..Character::default()
        };
        let vader = character(false, true, 85);
        let ackbar = character(true, false, 85);
        let both = character(true, true, 85);
        assert_eq!(leadership_bonus(&world, &vader), 50);
        assert_eq!(effective_leadership(&world, &vader), 85 + 42);
        assert_eq!(effective_leadership(&world, &ackbar), 85);
        assert_eq!(effective_leadership(&world, &both), 85);
        assert_eq!(
            effective_leadership(&world, &character(false, true, 30_000)),
            0x7fff
        );
        // The shipped table's value is read when present.
        world.gnprtb = crate::world::GnprtbParams::uniform(&[(3076, 20)]);
        assert_eq!(effective_leadership(&world, &vader), 85 + 17);
        // Without the seat, no bonus.
        world.characters[emperor].is_captive = true;
        assert_eq!(leadership_bonus(&world, &vader), 0);
        assert_eq!(effective_leadership(&world, &vader), 85);
    }

    #[test]
    fn a_seat_of_power_general_s_bonus_reaches_the_missions_and_the_duel() {
        // Slot +0x1f4 (+0x88) is what the missions (missions::member_skill)
        // and the command readers (character_command::leadership) read.
        let (world, _, coruscant) = world();
        let mut world = world;
        let general = world.characters.insert(Character {
            is_empire: true,
            current_system: Some(coruscant),
            leadership: crate::world::SkillPair {
                base: 60,
                variance: 0,
            },
            ..Character::default()
        });
        assert_eq!(
            crate::missions::member_skill(
                &world,
                crate::missions::MissionMember::Character(general),
                Skill::Leadership
            ),
            Some(90)
        );
        assert_eq!(crate::character_command::leadership(&world, Some(general)), 90);
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
