//! The Death Star's planet destruction and a destroyed system's cleanup.
//!
//! The Death Star is an ordinary capital ship (CAPSHPSD record 136, TEXTSTRA
//! 10120, research order 0), built by a shipyard like any other; a completed
//! one marks its fleet (`Fleet::has_death_star`). The original has no
//! separate construction timer.
//!
//! It destroys a planet only through the Fleet menu's Destroy System order
//! (`0x223`, `FUN_0058d990`; [`crate::bombardment`]), whose destroyed system
//! [`cleanup_destroyed_system`] carries out. Its superlaser in a tactical
//! battle is the tactical session's (`tactical-death-star-path.md`). The
//! system flag DeathStarNearby is [`crate::economy::death_star_nearby`]
//! (`FUN_005178b0`). A Death Star shield (DEFFACSD 4, family `0x25`) acts
//! only in a tactical battle, where it bars the fighters' Attack Death Star
//! (`FUN_0040bcd0`, `FUN_005ba450`); the strategic simulation keeps no
//! Death Star state.
//!
//! # Source
//! - `ghidra/notes/bombardment-order.md`
//! - `ghidra/notes/economy-cadence.md`, "Bit 11: DeathStarNearby"
//! - `ghidra/notes/tactical-death-star-path.md`

use serde::{Deserialize, Serialize};

use crate::ids::SystemKey;
use crate::world::GameWorld;

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// A Death Star's effect for the caller to apply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeathStarEvent {
    /// A Destroy System order's Death Star destroyed `system`
    /// (`FUN_0058d990`).
    ///
    /// Caller must set `world.systems[system].is_destroyed = true`.
    PlanetDestroyed { system: SystemKey, tick: u64 },
}

// ---------------------------------------------------------------------------
// Destroyed system cleanup
// ---------------------------------------------------------------------------

use crate::blockade::BlockadeState;
use crate::effects::GameEffect;
use crate::manufacturing::ManufacturingState;

/// Whether `character` stands in `system` itself: a direct child of the
/// system (`FUN_00513050`'s walk does not descend), not aboard a fleet, in
/// an abode or on a mission, and existing.
#[must_use]
pub fn stands_in(character: &crate::world::Character, system: SystemKey) -> bool {
    character.current_system == Some(system)
        && character.current_fleet.is_none()
        && character.current_abode.is_none()
        && !character.on_mission
        && !character.is_killed
}

/// Carry out a Death Star's destruction of `system`.
///
/// The system's destruction (`FUN_0058ce70` -> slot `+0xac`, `FUN_005080d0`
/// -> `FUN_004f71d0`) sets its destroyed bit (`FUN_004f7560`), whose hook
/// (`FUN_00510950` -> `FUN_004fb520`) calls slot `+0xb4` on each existing
/// direct child:
/// - a ship, squadron, regiment or facility that is completed and not en
///   route is destroyed with the system's reason (`FUN_004f8710` ->
///   `FUN_004f7ed0`); so is a special force (`FUN_00503e20`);
/// - a fleet stays (`FUN_00524fb0` returns 1), with its ships and the
///   characters aboard, and one en route there keeps its trip;
/// - a character on no mission goes adrift (`FUN_004ef2e0`), as the
///   system's reason is not `0x14`.
///
/// Then each of `killed`, the minor characters the destruction hit
/// (`FUN_0058d990`, [`crate::bombardment`]), is killed, with one
/// `GameEffect::CharacterKilled` each in `effects` for the caller's
/// telemetry. A killed character stays in the arena so next-tick story
/// events can still resolve it.
///
/// port: a captive is not set adrift (where the original holds one is not
/// traced); the manufacturing queues and the blockade at the system are
/// cleared.
pub fn cleanup_destroyed_system(
    world: &mut GameWorld,
    system: SystemKey,
    killed: &[crate::ids::CharacterKey],
    manufacturing: &mut ManufacturingState,
    blockade: &mut BlockadeState,
    effects: &mut Vec<GameEffect>,
) {
    let Some(sys) = world.systems.get(system) else {
        return;
    };

    let troop_keys: Vec<_> = sys.ground_units.clone();
    let sf_keys: Vec<_> = sys.special_forces.clone();
    let def_keys: Vec<_> = sys.defense_facilities.clone();
    let mfg_keys: Vec<_> = sys.manufacturing_facilities.clone();
    let prod_keys: Vec<_> = sys.production_facilities.clone();

    for (_, c) in &mut world.characters {
        if stands_in(c, system) && !c.is_captive {
            c.adrift = true;
        }
    }
    for &ck in killed {
        if world.characters.get(ck).is_some_and(|c| !c.is_killed) {
            crate::missions::kill_character(world, ck);
            effects.push(GameEffect::CharacterKilled { character: ck });
        }
    }

    for &tk in &troop_keys {
        world.troops.remove(tk);
    }
    for &sk in &sf_keys {
        world.special_forces.remove(sk);
    }
    for &dk in &def_keys {
        world.defense_facilities.remove(dk);
    }
    for &mk in &mfg_keys {
        world.manufacturing_facilities.remove(mk);
    }
    for &pk in &prod_keys {
        world.production_facilities.remove(pk);
    }

    if let Some(sys) = world.systems.get_mut(system) {
        sys.ground_units.clear();
        sys.fighters.clear();
        sys.special_forces.clear();
        sys.defense_facilities.clear();
        sys.manufacturing_facilities.clear();
        sys.production_facilities.clear();
    }

    // Clear manufacturing queues at the destroyed system.
    manufacturing.clear_queue(system);

    // Lift any blockade at the destroyed system.
    blockade.clear_blockade(system);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{ExplorationStatus, Faction, SectorGroup};
    use crate::ids::{DatId, FleetKey};
    use crate::world::ControlKind;
    use crate::world::{Fleet, GameWorld, Sector, System};

    fn make_world() -> (GameWorld, SystemKey) {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(0x9200_0000),
            name: "Outer Rim".into(),
            group: SectorGroup::RimOuter,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let sys = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0000),
            name: "Alderaan".into(),
            sector,
            x: 100,
            y: 100,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.9,
            popularity_empire: 0.1,
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
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Alliance),
        });
        (world, sys)
    }

    fn add_ds_fleet(world: &mut GameWorld, sys: SystemKey) -> FleetKey {
        let fk = world.fleets.insert(Fleet {
            location: sys,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: true,
            special_forces: Vec::new(),
        });
        world.systems.get_mut(sys).unwrap().fleets.push(fk);
        fk
    }

    #[test]
    fn a_destroyed_planet_loses_its_squadrons_with_its_regiments() {
        // port: cleanup_destroyed_system removes every object at the system;
        // the planet's squadrons (FUN_005039d0) stand on it.
        let (mut world, sys) = make_world();
        let tie = world
            .fighter_classes
            .insert(crate::world::FighterClass::default());
        crate::squadrons::add(&mut world.systems[sys].fighters, tie, 2);
        let mut effects = Vec::new();
        cleanup_destroyed_system(
            &mut world,
            sys,
            &[],
            &mut ManufacturingState::new(),
            &mut BlockadeState::new(),
            &mut effects,
        );
        assert!(world.systems[sys].fighters.is_empty());
    }

    #[test]
    fn a_destroyed_systems_fleets_stay_and_the_characters_standing_there_go_adrift() {
        // The system's destroyed hook (FUN_00510950 -> FUN_004fb520) calls
        // slot +0xb4 on each direct child: a fleet's (FUN_00524fb0) does
        // nothing, a regiment's (FUN_004f8710 -> FUN_004f7ed0) destroys it,
        // a character's (FUN_004ef2e0) sets it adrift unless on a mission.
        // FUN_0058d990 then destroys the minor characters it hit.
        let (mut world, sys) = make_world();
        let fleet = add_ds_fleet(&mut world, sys);
        let mut person = |edit: &dyn Fn(&mut crate::world::Character)| {
            let mut c = crate::world::Character {
                current_system: Some(sys),
                ..crate::world::Character::default()
            };
            edit(&mut c);
            world.characters.insert(c)
        };
        let standing = person(&|_| {});
        let hit = person(&|_| {});
        let aboard = person(&|_| {});
        let on_mission = person(&|c| c.on_mission = true);
        let captive = person(&|c| c.is_captive = true);
        world.characters[aboard].current_fleet = Some(fleet);
        world.fleets[fleet].characters.push(aboard);
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance: true,
            regiment_strength: 1,
        });
        world.systems[sys].ground_units.push(troop);

        let mut effects = Vec::new();
        cleanup_destroyed_system(
            &mut world,
            sys,
            &[hit],
            &mut ManufacturingState::new(),
            &mut BlockadeState::new(),
            &mut effects,
        );

        assert_eq!(world.systems[sys].fleets, [fleet]);
        assert_eq!(world.fleets[fleet].characters, [aboard]);
        assert!(!world.characters[aboard].adrift && !world.characters[aboard].is_killed);
        assert!(world.characters[standing].adrift);
        assert!(!world.characters[standing].is_killed);
        assert!(!world.characters[on_mission].adrift);
        assert!(!world.characters[captive].adrift);
        assert!(world.characters[hit].is_killed);
        assert!(matches!(
            effects.as_slice(),
            [GameEffect::CharacterKilled { character }] if *character == hit
        ));
        assert!(!world.troops.contains_key(troop));
        assert!(world.systems[sys].ground_units.is_empty());
    }
}
