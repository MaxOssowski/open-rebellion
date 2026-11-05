use std::path::PathBuf;

use rebellion_core::build_selection::listed_classes;
use rebellion_core::manufacturing::{BuildableKind, ProductionArea};
use rebellion_core::research::ResearchState;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory")
        .parent()
        .expect("repository root")
        .join("data/base")
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn a_new_alliance_games_construction_yard_lists_the_facilities_as_a0() {
    // A0 2026-11-05 (build-selection3/l*, m*: Build Facilities on Sluis Van,
    // day 3 of a new Alliance game): Construction Yard, GenCore Level I,
    // KDY-150, LNR Series I, Mine, Orbital Shipyard, Refinery, Training
    // Facility, in that order: FUN_0052e580's classes, the dialog's sorted
    // list (FUN_00437880, FUN_005f59f0).
    let world = rebellion_data::load_game_data(&data_dir()).expect("load original game data");
    let listed = listed_classes(&world, &ResearchState::new(), ProductionArea::ConstructionYard, true);
    let names: Vec<String> = rebellion_render::build_selection::in_list_order(&world, listed)
    .into_iter()
    .map(|kind| match kind {
        BuildableKind::ManufacturingFacility(build)
        | BuildableKind::ProductionFacility(build)
        | BuildableKind::DefenseFacility(build) => world.buildable_classes[&build.class].name.clone(),
        other => format!("{other:?}"),
    })
    .collect();
    assert_eq!(
        names,
        [
            "Construction Yard",
            "GenCore Level I",
            "KDY-150",
            "LNR Series I",
            "Mine",
            "Orbital Shipyard",
            "Refinery",
            "Training Facility",
        ]
    );
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn a_new_alliance_games_shipyard_lists_ships_and_fighters_by_name_as_a0() {
    // A0 2026-11-05 (build-selection3/ships0, k*: Build Ships on Sluis Van,
    // day 5): Alliance Dreadnaught, Alliance Escort Carrier, Bulk Cruiser,
    // Bulk Transport, Corellian Corvette, Medium Transport, X-wing, Y-wing;
    // the fighters among the ships by name.
    let world = rebellion_data::load_game_data(&data_dir()).expect("load original game data");
    let listed = listed_classes(&world, &ResearchState::new(), ProductionArea::Shipyard, true);
    let names: Vec<String> = rebellion_render::build_selection::in_list_order(&world, listed)
        .into_iter()
        .map(|kind| rebellion_render::manufacturing_window::product(&world, kind).0)
        .collect();
    assert_eq!(
        names,
        [
            "Alliance Dreadnaught",
            "Alliance Escort Carrier",
            "Bulk Cruiser",
            "Bulk Transport",
            "Corellian Corvette",
            "Medium Transport",
            "X-wing",
            "Y-wing",
        ]
    );
}
