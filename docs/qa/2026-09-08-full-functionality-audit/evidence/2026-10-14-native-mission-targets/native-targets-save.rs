//! Throwaway: write a native save holding the three mission targets.
use std::path::PathBuf;

use rebellion_core::dat::Faction;
use rebellion_core::world::{ControlKind, Fleet, ManufacturingFacilityInstance, ShipInstance};
use rebellion_data::replay_fixture::{seed42_initial_state, seed42_seed_options};
use rebellion_data::save::save_slot;

#[test]
#[ignore]
fn write_native_targets_save() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let world =
        rebellion_data::load_game_data_with_options(&root.join("data/base"), &seed42_seed_options())
            .unwrap();
    let mut state = seed42_initial_state(world).unwrap();
    let world = &mut state.world;
    let held = |world: &rebellion_core::world::GameWorld, key, side| {
        world.systems[key].control == ControlKind::Controlled(side) && !world.systems[key].is_destroyed
    };
    // An Empire system E on the map's right half and an Alliance system A
    // on its left half, so their windows open apart.
    let sector_x = |world: &rebellion_core::world::GameWorld, key| {
        world.sectors[world.systems[key].sector].x
    };
    let e = world
        .systems
        .keys()
        .filter(|&e| held(world, e, Faction::Empire) && sector_x(world, e) > 420)
        .min_by_key(|&e| world.systems[e].name.clone())
        .expect("an Empire system on the right");
    // A: the leftmost system of the Alliance-held system's sector on the
    // map's left half, held by the Alliance for this check, so its windows
    // open at the galaxy view's left edge.
    let anchor = world
        .systems
        .keys()
        .filter(|&a| held(world, a, Faction::Alliance) && sector_x(world, a) < 300)
        .min_by_key(|&a| world.systems[a].name.clone())
        .expect("an Alliance system on the left");
    let a = world
        .systems
        .keys()
        .filter(|&k| world.systems[k].sector == world.systems[anchor].sector)
        .min_by_key(|&k| world.systems[k].x)
        .unwrap();
    world.systems[a].control = ControlKind::Controlled(Faction::Alliance);
    world.systems[a].exploration_status = rebellion_core::dat::ExplorationStatus::Explored;
    let mut alliance = world
        .characters
        .iter()
        .filter(|(_, c)| c.is_alliance && !c.is_killed && c.recruited && !c.on_mission)
        .map(|(k, _)| k);
    let agent = alliance.next().unwrap();
    let prisoner = alliance.next().unwrap();
    for fleet in world.fleets.values_mut() {
        fleet.characters.retain(|&k| k != agent && k != prisoner);
    }
    for (key, c) in &mut world.characters {
        if key != prisoner && c.current_system == Some(e) {
            c.current_system = None;
        }
    }
    let c = &mut world.characters[agent];
    c.current_system = Some(a);
    c.current_fleet = None;
    let c = &mut world.characters[prisoner];
    c.current_system = Some(e);
    c.current_fleet = None;
    c.is_captive = true;
    c.captured_by = Some(Faction::Empire);
    let yard_class = world
        .manufacturing_facilities
        .values()
        .find(|f| f.is_shipyard)
        .map(|f| f.class_dat_id)
        .unwrap();
    let yard = world.manufacturing_facilities.insert(ManufacturingFacilityInstance {
        class_dat_id: yard_class,
        side: Faction::Empire,
        is_shipyard: true,
    });
    world.systems[e].manufacturing_facilities = vec![yard];
    let class = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| class.is_death_star())
        .map(|(k, _)| k)
        .unwrap();
    let fleet = world.insert_fleet(Fleet {
        location: e,
        capital_ships: vec![ShipInstance::new(class, 100, true)],
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: false,
        has_death_star: true,
    });
    world.systems[e].fleets.push(fleet);
    state.fog_alliance.reveal(e);
    state.fog_alliance.reveal(a);
    let names = (
        state.world.systems[a].name.clone(),
        state.world.systems[e].name.clone(),
        state.world.sectors[state.world.systems[a].sector].name.clone(),
        state.world.characters[agent].name.clone(),
        state.world.characters[prisoner].name.clone(),
    );
    let dir = PathBuf::from(std::env::var("SAVES").unwrap());
    let fingerprint = save_slot(&dir, 0, "Native mission targets", &state, &[]).unwrap();
    println!("A={} E={} sector={} agent={} prisoner={} fingerprint={fingerprint}", names.0, names.1, names.2, names.3, names.4);
    for key in [a, e] {
        let sys = &state.world.systems[key];
        let sector = &state.world.sectors[sys.sector];
        println!("{} at ({}, {}); sector {} at ({}, {})", sys.name, sys.x, sys.y, sector.name, sector.x, sector.y);
    }
}
