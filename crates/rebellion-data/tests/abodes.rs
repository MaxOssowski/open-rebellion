use std::path::PathBuf;

use rebellion_core::dat::Faction;

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
fn the_five_abodes_load_with_the_sides_the_galaxy_setup_gives_them() {
    // ABODESD holds five family-0x98 entries; FUN_00519910 creates them
    // with sides 1 (0x440), 3 (0x441, 0x442) and 2 (0x480, 0x481).
    let world = rebellion_data::load_game_data(&data_dir()).expect("load original game data");
    let abodes: Vec<(u32, &str, Faction)> = world
        .abodes
        .values()
        .map(|abode| (abode.dat_id.raw(), abode.name.as_str(), abode.side))
        .collect();
    assert_eq!(
        abodes,
        vec![
            (0x440, "Alliance Jail", Faction::Alliance),
            (0x441, "Dagobah", Faction::Neutral),
            (0x442, "Hermitage", Faction::Neutral),
            (0x480, "Empire Jail", Faction::Empire),
            (0x481, "Jabba's Palace", Faction::Empire),
        ]
    );
    // Nobody starts in an abode.
    assert!(world.characters.values().all(|c| c.current_abode.is_none()));
}
