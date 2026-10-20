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
