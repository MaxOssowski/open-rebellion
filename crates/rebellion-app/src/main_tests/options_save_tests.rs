use rebellion_data::save::WriteStatus;
use rebellion_render::game_options::{GameOptionsOrigin, GameOptionsState};

fn settle(status: WriteStatus) -> (GameOptionsState, Option<(usize, bool)>) {
    let root = std::env::temp_dir().join(format!(
        "options-settle-{:?}-{}",
        status,
        std::process::id()
    ));
    let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
    state.save_in_flight = Some(3);
    state.error = Some("stale".into());
    let mut slots = Vec::new();
    let settled = super::settle_game_options_save(&mut state, &mut slots, &root, status);
    (state, settled)
}

#[test]
fn a_pending_save_holds_the_window_without_a_message() {
    let (state, settled) = settle(WriteStatus::Pending);
    assert_eq!(settled, None);
    assert_eq!(state.save_in_flight, Some(3));
    assert_eq!(state.saved_notice, None);
    assert!(state.modal());
}

#[test]
fn a_committed_save_shows_the_saved_message() {
    // FUN_00408240: REBDLOG 0x1217 "Selected Game Saved Successfully",
    // and the visit's flag that lets Restart ask nothing.
    let (state, settled) = settle(WriteStatus::Committed);
    assert_eq!(settled, Some((3, true)));
    assert_eq!(state.save_in_flight, None);
    assert_eq!(state.saved_notice, Some(true));
    assert!(state.saved_this_visit);
    assert_eq!(state.error, None);
}

#[test]
fn a_save_over_the_quota_shows_only_the_failed_message() {
    // FUN_00408240: REBDLOG 0x1219 "Selected Game Failed To Save"; the
    // original has no disk-full string, so no other line is written.
    let (state, settled) = settle(WriteStatus::QuotaExceeded);
    assert_eq!(settled, Some((3, false)));
    assert_eq!(state.saved_notice, Some(false));
    assert!(!state.saved_this_visit);
    assert_eq!(state.error.as_deref(), Some("stale"));
}

#[test]
fn another_failed_save_shows_the_failed_message_only() {
    let (state, settled) = settle(WriteStatus::Failed);
    assert_eq!(settled, Some((3, false)));
    assert_eq!(state.saved_notice, Some(false));
    assert_eq!(state.error.as_deref(), Some("stale"));
}

#[test]
fn nothing_settles_without_a_save_in_flight() {
    let mut state = GameOptionsState::new(GameOptionsOrigin::CommandCenter);
    let mut slots = Vec::new();
    let root = std::env::temp_dir();
    assert_eq!(
        super::settle_game_options_save(&mut state, &mut slots, &root, WriteStatus::Committed),
        None
    );
    assert_eq!(state.saved_notice, None);
}

#[test]
fn unreadable_native_saves_show_an_empty_row_and_stay_unchanged() {
    // FUN_00415cd0 keeps FUN_00411730's empty header when the read
    // fails, so FUN_00407180 lists no name and disables Load
    // (+0x18 = 0), and FUN_00406b80 0x6f..0x74 asks no overwrite
    // question. The row's absence from the list is that empty row.
    let root = std::env::temp_dir().join(format!("options-corrupt-save-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = rebellion_data::save::slot_path(&root, 5);
    std::fs::write(&path, b"corrupt save fixture").unwrap();
    let empty = rebellion_data::save::slot_path(&root, 4);
    std::fs::write(&empty, b"").unwrap();
    let slots = super::read_save_slots(&root);
    assert!(slots.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), b"corrupt save fixture");
    assert!(std::fs::read(&empty).unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn saves_beyond_the_six_rows_stay_on_disk_and_are_not_listed() {
    // FUN_00407180 builds six save rows; slots 6..9 and stray files in
    // the saves directory are left alone.
    let root = std::env::temp_dir().join(format!("options-extra-saves-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let legacy = rebellion_data::save::slot_path(&root, 9);
    std::fs::write(&legacy, b"legacy slot fixture").unwrap();
    let stray = root.join("notes.txt");
    std::fs::write(&stray, b"not a save").unwrap();
    assert!(super::read_save_slots(&root).is_empty());
    assert_eq!(std::fs::read(&legacy).unwrap(), b"legacy slot fixture");
    assert_eq!(std::fs::read(&stray).unwrap(), b"not a save");
    std::fs::remove_dir_all(root).unwrap();
}
