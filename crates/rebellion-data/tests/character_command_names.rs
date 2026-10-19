use std::path::PathBuf;

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
fn a_character_loads_its_name_for_each_command() {
    // FUN_004f17f0: TEXTSTRA of the class name +0x6800, +0x6c00, +0x7000;
    // Ackbar's 10816 gives 37440 "Commander Ackbar", 38464 "Admiral Ackbar"
    // and 39488 "General Ackbar" (ghidra/notes/decoy-roll.md).
    let world = rebellion_data::load_game_data(&data_dir()).expect("load original game data");
    let ackbar = world
        .characters
        .values()
        .find(|c| c.name == "Ackbar")
        .expect("Ackbar is a minor character");
    assert_eq!(
        ackbar.command_names,
        [
            "Ackbar".to_owned(),
            "Commander Ackbar".to_owned(),
            "Admiral Ackbar".to_owned(),
            "General Ackbar".to_owned()
        ]
    );
    assert!(ackbar.can_be_admiral);
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn a_character_loads_its_class_flags_from_the_dat() {
    // MNCHARSD 832 (Ackbar, TEXTSTRA 10816): can_be_general 1,
    // is_unable_to_betray 1; MJCHARSD 576 (Mon Mothma): can_be_general 0.
    let world = rebellion_data::load_game_data(&data_dir()).expect("load original game data");
    let named = |name: &str| {
        world
            .characters
            .values()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("{name} is loaded"))
    };
    let ackbar = named("Ackbar");
    assert!(ackbar.can_be_general);
    assert!(ackbar.is_unable_to_betray);
    assert!(!named("Mon Mothma").can_be_general);
}
