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
