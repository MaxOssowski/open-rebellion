use std::path::PathBuf;

use rebellion_core::dat::Faction;
use rebellion_core::headquarters_move::{move_headquarters, HeadquartersMove};
use rebellion_core::ids::DatId;
use rebellion_core::troop_transport::TroopTransportState;
use rebellion_core::victory::VictoryState;
use rebellion_core::world::{ControlKind, GameWorld, ManufacturingFacilityInstance};

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory")
        .parent()
        .expect("repository root")
        .join("data/base")
}

fn system(world: &GameWorld, name: &str) -> rebellion_core::ids::SystemKey {
    world
        .systems
        .iter()
        .find(|(_, value)| value.name == name)
        .map(|(key, _)| key)
        .unwrap_or_else(|| panic!("{name} in SYSTEMSD"))
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn the_headquarters_trip_from_jerijador_to_yavin_takes_87_days_as_a0() {
    // A0 2026-11-13-hq-move/02 and /04: the Alliance HQ ordered from
    // Jerijador to Yavin on day 346 reads "ETA Destination: Day 433"
    // (FUN_004fd2b0): the trip at the GNPRTB 1 default speed
    // (FUN_004f63f0, FUN_00555b30).
    let mut world = rebellion_data::load_game_data(&data_dir()).expect("load original game data");
    let (jerijador, yavin) = (system(&world, "Jerijador"), system(&world, "Yavin"));
    for value in world.systems.values_mut() {
        value.manufacturing_facilities.clear();
    }
    let headquarters = world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id: DatId::new(0x2000_0001),
            side: Faction::Alliance,
            is_shipyard: false,
        });
    world.systems[jerijador]
        .manufacturing_facilities
        .push(headquarters);
    for key in [jerijador, yavin] {
        world.systems[key].control = ControlKind::Controlled(Faction::Alliance);
    }
    let mut transport = TroopTransportState::default();
    let mut victory = VictoryState::new(jerijador, jerijador);
    let moved = move_headquarters(&mut world, &mut transport, &mut victory, true, yavin, 346)
        .expect("the Alliance may move its headquarters to its own Yavin");
    let HeadquartersMove::Departed(transit) = moved else {
        panic!("the headquarters stayed: {moved:?}");
    };
    assert_eq!((transit.origin, transit.destination), (jerijador, yavin));
    assert_eq!(transit.arrival_tick, 433);
}
