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
#[ignore = "requires original data/base TEXTSTRA.DLL"]
fn the_message_texts_carry_the_side_names_from_the_string_table() {
    // FUN_0052d920 names a side by TEXTSTRA strings 0x4d50/0x4d51 (kind 1),
    // 0x4d60/0x4d61 (kind 2) and 0x4d70/0x4d71 (kind 4); the Uprising Begins
    // body 0x7033 (FUN_00499460) substitutes kind 4.
    let texts = rebellion_data::message_texts(&data_dir());
    assert_eq!(texts[&0x4d50], "Alliance");
    assert_eq!(texts[&0x4d61], "Galactic Empire");
    assert_eq!(texts[&0x4d70], "Alliance");
    assert_eq!(texts[&0x4d71], "Imperial");
    assert_eq!(
        texts[&0x7033],
        "An uprising has begun on the {1:4} controlled system {2:1}."
    );
}

#[test]
#[ignore = "requires original data/base TEXTSTRA.DLL"]
fn the_recruitment_repair_and_mission_end_messages_read_their_texts() {
    // FUN_0048ae40 0x7210 / 0x7211; FUN_00499de0 0x7040..0x7043;
    // FUN_00491500 codes 6..10, 0xc: 0x7138..0x713d, 0x7140..0x7145, and
    // the closing 0x70e6 / 0x70e7.
    let texts = rebellion_data::message_texts(&data_dir());
    let expected = [
        (0x7210, "Recruitment Done"),
        (0x7211, "We regret to report that there are no more candidates to be recruited."),
        (0x7040, "Squadron at Full Strength"),
        (0x7041, "Capital Ship Repaired"),
        (0x7042, "The {1:1} Squadron attached to {2:1} has replaced all fighters lost in combat."),
        (0x7043, "{1:1} attached to {2:1} has repaired all battle damage and is fully operational."),
        (0x7138, "{1:1} Mission Failed"),
        (0x7139, "My {2:1} Mission to {3:1} has failed because the target was not found at that location.  "),
        (0x713a, "{2:1} Mission Failed"),
        (0x713b, "The {2:1} Mission to {3:1} has failed because the target was not found at that location.  "),
        (0x713c, "My {2:1} Mission to {3:1} has failed because we have no research facilities at that location."),
        (0x713d, "The {2:1} Mission to {3:1} has failed because we have no research facilities at that location.  "),
        (0x7140, "{1:1} Mission Aborted"),
        (0x7141, "My {2:1} mission to {3:1} has been aborted because the system has joined the {4:1}.  "),
        (0x7142, "{2:1} Mission Aborted."),
        (0x7143, "The {2:1} mission to {3:1} has been aborted because the system has joined the {4:1}.  "),
        (0x7144, "My {2:1} mission to {3:1} has been aborted because the system has declared neutrality."),
        (0x7145, "The {2:1} mission to {3:1} has been aborted because the system has declared neutrality."),
        (0x70e6, "\nPersonnel are returning to {1:1}."),
        (0x70e7, "\nAll personnel assigned to the mission have failed to return."),
    ];
    for (id, text) in expected {
        assert_eq!(texts[&id], text, "{id:#x}");
    }
}
