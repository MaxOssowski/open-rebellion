use std::path::PathBuf;

use rebellion_core::ids::DatId;

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
fn original_troop_classes_use_compound_dat_ids() {
    let world = rebellion_data::load_game_data(&data_dir()).expect("load original game data");

    assert_eq!(world.troop_classes.len(), 10);
    for troop in world.troops.values() {
        assert_eq!(troop.class_dat_id.family(), 0x10);
        assert!(
            world.troop_classes.contains_key(&troop.class_dat_id),
            "seeded troop {:?} must resolve to its TROOPSD class",
            troop.class_dat_id
        );
    }

    let alliance_army = world
        .troop_classes
        .get(&DatId::new(0x1000_0002))
        .expect("Alliance Army compound class id");
    assert!(alliance_army.attack_strength > 0);
    assert!(alliance_army.defense_strength > 0);
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn the_alliance_headquarters_has_its_record_s_bombardment_defense() {
    // ALLFACSD 1's last word is class +0x58, which FUN_0058cf80 reads as a
    // bombardment target's defense (ghidra/notes/bombardment-order.md).
    let world = rebellion_data::load_game_data(&data_dir()).expect("load original game data");
    let headquarters = world
        .buildable_classes
        .get(&DatId::new(0x2000_0001))
        .expect("ALLFACSD 1");
    assert_eq!(headquarters.stats.bombardment, 9);
}
