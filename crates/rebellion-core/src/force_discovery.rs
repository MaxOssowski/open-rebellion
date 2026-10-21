//! Force discovery: each discovering trainer's timer `0x389`
//! (`ghidra/notes/force-awareness.md`).
//!
//! A character is a `DiscoveringForceUser` (`+0xac` bit `0x80`,
//! `FUN_004ef070`) while its class trains Jedi (`+0xb8`, `FUN_004ed220`), its
//! Force level is at least GNPRTB 2580 (80, `FUN_0053eb70`), it is usable
//! (`+0x50` bit 0), not on a mission (`+0x78` bit 7) and not captured
//! (`+0xac` bit 0). The flag's hook (slot `+0x364`, `FUN_004f1b70`) arms
//! timer `0x389` with it on record `+0x54`+`0x20`, whose minimum is GNPRTB
//! 2582 (1) and spread 2583 (0) (`FUN_004ef750`, disassembly
//! `0x4ef7aa..0x4ef7c8`): with the shipped values it fires every day.
//!
//! Each fire (`FUN_00560a30`) walks the characters of the trainer's side
//! held directly by its container (`FUN_004f6b50`, `FUN_004f2640(container,
//! 1, side)`). One that is not captured, not on a mission, not the trainer,
//! with `ForcePotential` and without `ForceAware` becomes aware
//! (`FUN_004ee950`) and records the trainer (`+0xa4`, `FUN_004ee680`). Leia
//! (`0x241`) is found only by Luke (`0x242`) once he knows his heritage
//! (`+0xb0` bit 0, `KnowsHeritage`, `FUN_0056f650`), and then learns hers
//! (`FUN_0056fe90`). Awareness's hook (slot `+0x358`, `FUN_004f1b30`) rolls
//! the new Force base (`FUN_004eee60`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ids::CharacterKey;
use crate::tick::TickEvent;
use crate::world::{Character, GameWorld};

/// GNPRTB 2580 (`DAT_006b9058`) = 80: the Force level a trainer needs to
/// discover others (`FUN_0053eb70`).
const GNPRTB_DISCOVERING_FORCE: u16 = 2580;
/// GNPRTB 2582 (`DAT_006b907c`) = 1: timer `0x389`'s minimum.
const GNPRTB_DISCOVERY_MIN: u16 = 2582;
/// GNPRTB 2583 (`DAT_006b90c0`) = 0: its spread.
const GNPRTB_DISCOVERY_SPREAD: u16 = 2583;

/// Leia's record (`0x31000241`, MJCHARSD 577), tested by `FUN_00560a30`.
const LEIA: u32 = 0x241;
/// Luke's record (`0x32000242`, MJCHARSD 578), tested by `FUN_00560a30`.
const LUKE: u32 = 0x242;

/// Each discovering trainer's timer `0x389`: the tick it next fires
/// (save v70).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ForceDiscoveryState {
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub timers: HashMap<CharacterKey, u64>,
}

/// A character whose Force a trainer discovered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceDiscovered {
    pub character: CharacterKey,
    pub discoverer: CharacterKey,
    /// The Force base the awareness hook rolls (`FUN_004eee60`), when one
    /// is due.
    pub force_base: Option<u16>,
    /// Leia learns her heritage (`FUN_0056fe90`).
    pub learns_heritage: bool,
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
        reason = "the roll is in [0, 1), so the product is at most n"
    )]
    fn draw(&mut self, n: u32) -> u32 {
        let roll = self.rolls.next().copied().unwrap_or(0.0);
        ((roll * f64::from(n.saturating_add(1))) as u32).min(n)
    }
}

/// `FUN_004ef070`: the `DiscoveringForceUser` flag.
///
/// port: usable (`+0x50` bit 0) is a living character with a location, a
/// fleet or a system; a character with neither is travelling. The port's
/// mandatory story mission counts as a mission.
#[must_use]
pub fn discovering_force_user(world: &GameWorld, c: &Character) -> bool {
    let threshold = world
        .gnprtb
        .value(GNPRTB_DISCOVERING_FORCE, world.difficulty_index);
    c.is_jedi_trainer
        && i64::from(c.force_level()) >= i64::from(threshold)
        && !c.is_killed
        && (c.current_fleet.is_some() || c.current_system.is_some())
        && !c.on_mission
        && !c.on_mandatory_mission
        && !c.is_captive
}

/// `FUN_004f2640(container, 1, side)` with `FUN_00560a30`'s tests: the
/// characters of the trainer's side its container holds directly, free,
/// with the Force potential and not yet aware.
///
/// port: the container is the trainer's fleet, else its system; a system
/// holds the characters there that are in no fleet.
fn candidates(world: &GameWorld, key: CharacterKey, trainer: &Character) -> Vec<CharacterKey> {
    world
        .characters
        .iter()
        .filter(|&(other, c)| {
            other != key
                && c.is_alliance == trainer.is_alliance
                && c.is_empire == trainer.is_empire
                && !c.is_killed
                && match trainer.current_fleet {
                    Some(fleet) => c.current_fleet == Some(fleet),
                    None => c.current_fleet.is_none() && c.current_system == trainer.current_system,
                }
                && !c.is_captive
                && !c.on_mission
                && !c.on_mandatory_mission
                && c.force_potential
                && !c.force_aware
        })
        .map(|(other, _)| other)
        .collect()
}

fn is(c: &Character, record: u32) -> bool {
    c.dat_id.index() == record
}

/// Stateless system for timer `0x389`.
pub struct ForceDiscoverySystem;

impl ForceDiscoverySystem {
    /// The most rolls [`Self::advance`] draws: per trainer a delay to arm, a
    /// delay to rearm and a Force base for each character it finds.
    ///
    /// port: the caller draws the rolls up front, as for the other timers.
    #[must_use]
    pub fn roll_budget(world: &GameWorld) -> usize {
        let trainers = world
            .characters
            .values()
            .filter(|c| c.is_jedi_trainer)
            .count();
        trainers * (2 + world.characters.len())
    }

    /// Arm the timers of the discovering trainers, drop the others, and fire
    /// the ones due by the last tick, in character order.
    pub fn advance(
        state: &mut ForceDiscoveryState,
        world: &GameWorld,
        tick_events: &[TickEvent],
        rolls: &[f64],
    ) -> Vec<ForceDiscovered> {
        let Some(last) = tick_events.last() else {
            return Vec::new();
        };
        let tick = last.tick;
        let param = |id| world.gnprtb.value(id, world.difficulty_index);
        let min = u64::try_from(param(GNPRTB_DISCOVERY_MIN).max(0)).unwrap_or(0);
        let spread = u32::try_from(param(GNPRTB_DISCOVERY_SPREAD).max(0)).unwrap_or(0);
        let mut rolls = Rolls {
            rolls: rolls.iter(),
        };
        // FUN_00586130: min + rand(0..=spread), at least a day.
        let delay = |rolls: &mut Rolls<'_>| {
            let extra = if spread == 0 { 0 } else { rolls.draw(spread) };
            (min + u64::from(extra)).max(1)
        };
        let mut found = Vec::new();
        let mut aware: Vec<CharacterKey> = Vec::new();
        for (key, trainer) in &world.characters {
            if !discovering_force_user(world, trainer) {
                state.timers.remove(&key);
                continue;
            }
            let due = *state
                .timers
                .entry(key)
                .or_insert_with(|| tick + delay(&mut rolls));
            if due > tick {
                continue;
            }
            state.timers.insert(key, tick + delay(&mut rolls));
            for other in candidates(world, key, trainer) {
                if aware.contains(&other) {
                    continue;
                }
                let c = &world.characters[other];
                let learns_heritage = is(c, LEIA);
                if learns_heritage && !(is(trainer, LUKE) && trainer.heritage_known) {
                    continue;
                }
                let mut rolled = c.clone();
                rolled.force_aware = true;
                let force_base = rolled.force_base_due().then(|| {
                    rolled.roll_force_base(rolls.draw(c.jedi_level.variance));
                    rolled.force_base
                });
                aware.push(other);
                found.push(ForceDiscovered {
                    character: other,
                    discoverer: key,
                    force_base,
                    learns_heritage,
                    tick,
                });
            }
        }
        found
    }
}

/// Store a discovery: `ForceAware`, the discoverer (`+0xa4`), the rolled
/// Force base and Leia's heritage.
pub fn apply(world: &mut GameWorld, found: &ForceDiscovered) {
    let record = world.characters.get(found.discoverer).map(|c| c.dat_id);
    if let Some(c) = world.characters.get_mut(found.character) {
        c.force_aware = true;
        c.force_discoverer = record;
        if let Some(base) = found.force_base {
            c.force_base = base;
        }
        if found.learns_heritage {
            c.heritage_known = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{DatId, FleetKey, SystemKey};
    use crate::world::{ControlKind, Fleet, GnprtbEntry, GnprtbParams, SkillPair, System};

    /// The shipped GNPRTB 2580, 2582 and 2583, two systems, and an Alliance
    /// trainer at the first with Force `force`.
    fn setup(force: u16) -> (GameWorld, CharacterKey, SystemKey, SystemKey) {
        setup_with(force, 1, 0)
    }

    fn system(world: &mut GameWorld, name: &str) -> SystemKey {
        world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: name.into(),
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
            control: ControlKind::Uncontrolled,
        })
    }

    /// As [`setup`], with GNPRTB 2582 `min` and 2583 `spread`.
    fn setup_with(
        force: u16,
        min: i32,
        spread: i32,
    ) -> (GameWorld, CharacterKey, SystemKey, SystemKey) {
        let mut world = GameWorld {
            gnprtb: GnprtbParams::new(
                [(2580, 80), (2582, min), (2583, spread)]
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
        let here = system(&mut world, "Here");
        let there = system(&mut world, "There");
        let trainer = world.characters.insert(Character {
            dat_id: DatId::new(LUKE),
            is_alliance: true,
            is_jedi_trainer: true,
            force_potential: true,
            force_aware: true,
            force_base: force,
            current_system: Some(here),
            ..Character::default()
        });
        (world, trainer, here, there)
    }

    /// An Alliance character at `system` with the Force potential.
    fn latent(world: &mut GameWorld, system: SystemKey) -> CharacterKey {
        world.characters.insert(Character {
            dat_id: DatId::new(0x340),
            is_alliance: true,
            force_potential: true,
            jedi_level: SkillPair {
                base: 10,
                variance: 40,
            },
            current_system: Some(system),
            ..Character::default()
        })
    }

    fn day(
        state: &mut ForceDiscoveryState,
        world: &mut GameWorld,
        tick: u64,
        rolls: &[f64],
    ) -> Vec<ForceDiscovered> {
        let found = ForceDiscoverySystem::advance(state, world, &[TickEvent { tick }], rolls);
        for d in &found {
            apply(world, d);
        }
        found
    }

    #[test]
    fn a_trainer_with_force_eighty_finds_a_latent_character_beside_it_the_next_day() {
        // FUN_004ef070: trainer, Force >= GNPRTB 2580 (80); timer 0x389 on
        // record +0x20 waits GNPRTB 2582 (1) + rand(0..=2583 (0)) days;
        // FUN_00560a30 makes the character aware and FUN_004eee60 rolls its
        // base, class 10 + rand(0..=40).
        let (mut world, trainer, here, _) = setup(80);
        let found_one = latent(&mut world, here);
        let mut state = ForceDiscoveryState::default();
        assert_eq!(day(&mut state, &mut world, 1, &[0.5]), []);
        assert_eq!(state.timers[&trainer], 2);
        let found = day(&mut state, &mut world, 2, &[0.5]);
        assert_eq!(found.len(), 1);
        let c = &world.characters[found_one];
        assert!(c.force_aware);
        assert_eq!(c.force_base, 30);
        assert_eq!(c.force_discoverer, Some(DatId::new(LUKE)));
        // Already aware: nothing more to find.
        assert_eq!(day(&mut state, &mut world, 3, &[0.5]), []);
    }

    #[test]
    fn a_trainer_below_eighty_or_busy_discovers_nobody() {
        // FUN_004ef070: Force 79 fails GNPRTB 2580; a mission, captivity or
        // a missing trainer class clears the flag and the timer.
        let busy: [fn(&mut Character); 4] = [
            |c| c.force_base = 79,
            |c| c.on_mission = true,
            |c| c.is_captive = true,
            |c| c.is_jedi_trainer = false,
        ];
        for edit in busy {
            let (mut world, trainer, here, _) = setup(80);
            latent(&mut world, here);
            edit(&mut world.characters[trainer]);
            let mut state = ForceDiscoveryState::default();
            for tick in 1..=5 {
                assert_eq!(day(&mut state, &mut world, tick, &[]), []);
            }
            assert!(state.timers.is_empty());
        }
    }

    #[test]
    fn only_free_latent_unaware_characters_of_its_side_beside_it_are_found() {
        // FUN_00560a30 skips the captured (+0xac bit 0), those on a mission
        // (+0x78 bit 7), the aware (bit 0x10) and those without the
        // potential (bit 0x20); FUN_004f2640 keeps the trainer's side and
        // its container's own children.
        let (mut world, _, here, there) = setup(80);
        let found_one = latent(&mut world, here);
        let skipped: [fn(&mut Character); 6] = [
            |c| c.is_captive = true,
            |c| c.on_mission = true,
            |c| c.force_aware = true,
            |c| c.force_potential = false,
            |c| {
                c.is_alliance = false;
                c.is_empire = true;
            },
            |c| c.current_system = None,
        ];
        let mut others = Vec::new();
        for edit in skipped {
            let key = latent(&mut world, here);
            edit(&mut world.characters[key]);
            others.push(key);
        }
        let elsewhere = latent(&mut world, there);
        let mut state = ForceDiscoveryState::default();
        day(&mut state, &mut world, 1, &[]);
        let found = day(&mut state, &mut world, 2, &[]);
        assert_eq!(
            found.iter().map(|d| d.character).collect::<Vec<_>>(),
            vec![found_one]
        );
        assert!(!world.characters[elsewhere].force_aware);
    }

    #[test]
    fn a_trainer_in_a_fleet_finds_its_shipmates_only() {
        // FUN_004f6b50: the container is the fleet; the system's own
        // characters are not its children.
        let (mut world, trainer, here, _) = setup(80);
        let fleet: FleetKey = world.fleets.insert(Fleet {
            location: here,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![trainer],
            is_alliance: true,
            has_death_star: false,
        });
        world.characters[trainer].current_fleet = Some(fleet);
        let aboard = latent(&mut world, here);
        world.characters[aboard].current_fleet = Some(fleet);
        let ashore = latent(&mut world, here);
        let mut state = ForceDiscoveryState::default();
        day(&mut state, &mut world, 1, &[]);
        let found = day(&mut state, &mut world, 2, &[]);
        assert_eq!(
            found.iter().map(|d| d.character).collect::<Vec<_>>(),
            vec![aboard]
        );
        assert!(!world.characters[ashore].force_aware);
    }

    #[test]
    fn leia_is_found_only_by_luke_once_he_knows_his_heritage() {
        // FUN_00560a30: 0x31000241 only for 0x32000242 with +0xb0 bit 0
        // (KnowsHeritage, FUN_0056f650); she then learns hers
        // (FUN_0056fe90). MJCHARSD 577: Force 10, variance 0.
        let (mut world, trainer, here, _) = setup(80);
        let leia = world.characters.insert(Character {
            dat_id: DatId::new(LEIA),
            is_alliance: true,
            force_potential: true,
            jedi_level: SkillPair {
                base: 10,
                variance: 0,
            },
            current_system: Some(here),
            ..Character::default()
        });
        let mut state = ForceDiscoveryState::default();
        for tick in 1..=3 {
            assert_eq!(day(&mut state, &mut world, tick, &[]), []);
        }
        world.characters[trainer].heritage_known = true;
        let found = day(&mut state, &mut world, 4, &[]);
        assert_eq!(found.len(), 1);
        let c = &world.characters[leia];
        assert!(c.force_aware && c.heritage_known);
        assert_eq!(c.force_base, 10);
        // Another trainer, not Luke, never finds her.
        let (mut world, trainer, here, _) = setup(120);
        world.characters[trainer].dat_id = DatId::new(0x281);
        world.characters[trainer].heritage_known = true;
        let leia = world.characters.insert(Character {
            dat_id: DatId::new(LEIA),
            is_alliance: true,
            force_potential: true,
            current_system: Some(here),
            ..Character::default()
        });
        let mut state = ForceDiscoveryState::default();
        for tick in 1..=3 {
            day(&mut state, &mut world, tick, &[]);
        }
        assert!(!world.characters[leia].force_aware);
    }

    #[test]
    fn the_timer_waits_its_record_s_minimum_and_spread() {
        // FUN_00586130: min + rand(0..=spread); GNPRTB 2582 = 3 and 2583 = 4
        // with a draw of 0.5 (2 of 0..=4) wait five days.
        let (mut world, trainer, here, _) = setup_with(80, 3, 4);
        latent(&mut world, here);
        let mut state = ForceDiscoveryState::default();
        day(&mut state, &mut world, 1, &[0.5]);
        assert_eq!(state.timers[&trainer], 6);
        for tick in 2..=5 {
            assert_eq!(day(&mut state, &mut world, tick, &[]), []);
        }
        assert_eq!(day(&mut state, &mut world, 6, &[0.0, 0.0]).len(), 1);
        assert_eq!(state.timers[&trainer], 9);
    }

    #[test]
    fn the_roll_budget_covers_two_delays_and_every_character_per_trainer() {
        let (mut world, _, here, _) = setup(80);
        latent(&mut world, here);
        latent(&mut world, here);
        // One trainer, three characters: 2 + 3.
        assert_eq!(ForceDiscoverySystem::roll_budget(&world), 5);
    }
}
