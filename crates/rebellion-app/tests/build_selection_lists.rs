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

#[test]
#[ignore = "requires original data/base DAT files"]
fn kessel_to_yavin_deployment_days_match_a0() {
    // A0 2026-11-09 (build-selection4/20..23: a new Alliance game, Kessel's
    // ships band with its Destination on Yavin): Best Time To Deployment
    // 76 Days for Alliance Dreadnaught, Alliance Escort Carrier and
    // Corellian Corvette, 57 Days for X-wing (FUN_00555f90 ->
    // FUN_00555b30: the product's transit from its yard to the destination).
    let world = rebellion_data::load_game_data(&data_dir()).expect("load original game data");
    let system = |name: &str| {
        world
            .systems
            .iter()
            .find(|(_, system)| system.name == name)
            .map(|(key, _)| key)
            .unwrap_or_else(|| panic!("no system {name}"))
    };
    let (kessel, yavin) = (system("Kessel"), system("Yavin"));
    let listed = listed_classes(&world, &ResearchState::new(), ProductionArea::Shipyard, true);
    let days: Vec<(String, u32)> = listed
        .into_iter()
        .map(|kind| {
            (
                rebellion_render::manufacturing_window::product(&world, kind).0,
                rebellion_core::delivery::transit_days(&world, kind, kessel, yavin),
            )
        })
        .filter(|(name, _)| {
            ["Alliance Dreadnaught", "Alliance Escort Carrier", "Corellian Corvette", "X-wing"]
                .contains(&name.as_str())
        })
        .collect();
    let mut days = days;
    days.sort();
    assert_eq!(
        days,
        [
            ("Alliance Dreadnaught".to_string(), 76),
            ("Alliance Escort Carrier".to_string(), 76),
            ("Corellian Corvette".to_string(), 76),
            ("X-wing".to_string(), 57),
        ]
    );
    // A0 build-selection4/24: a Mine ordered on Kessel's facilities band
    // with the Destination Yavin shows 96 Days (a facility moves at the
    // GNPRTB default speed, FUN_004f63f0).
    let listed = listed_classes(&world, &ResearchState::new(), ProductionArea::ConstructionYard, true);
    let mine = listed
        .into_iter()
        .find(|&kind| match kind {
            BuildableKind::ProductionFacility(build) => world.buildable_classes[&build.class].name == "Mine",
            _ => false,
        })
        .expect("a mine is listed");
    assert_eq!(rebellion_core::delivery::transit_days(&world, mine, kessel, yavin), 96);
}
