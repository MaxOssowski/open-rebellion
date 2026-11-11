# The unreachable Load Game screen is removed (the legacy save panel stays)

Date: 2026-11-11. This follows the owner's rule of 2026-11-10: screens
that the original does not have are removed from the port.

## Reachability

`GameMode::LoadGame` could never be shown:
- **No assignment.** I searched `crates/` (the app, both interface and
  tactical test fixtures, the playtest crate and the dev commands),
  `tools/`, `web/` and `scripts/`. Nothing ever sets the game mode to
  `GameMode::LoadGame`.
- **Main menu.** Its Load button (`MainMenuAction::LoadGame`) opens the
  original Game Options window from the shuttle cockpit
  (`set_origin(ShuttleCockpit)`).

## The original

The original saves and loads only through the Game Options window:
- Its save path is `FUN_00406b80` (commands 0x6f..0x74) -> `FUN_00408000`
  -> `FUN_00415600`. No other function in the decompile corpus calls
  `FUN_00408000` or `FUN_00415600`.
- The manual's Command Center keyboard reference (pp. 64-65) and its "SAVE
  GAME/LOAD GAME" text (Game Options, six slots) name no other way to save
  or load.

## The change

`crates/rebellion-app/src/main.rs` loses the following:
- the `GameMode::LoadGame` variant;
- its Escape branch;
- its draw arm, which drew `draw_save_load` alone on a dark screen;
- the `CloseSaveLoadPanel` branch that returned from it to the main menu.

No test covered the mode, so no test changed.

## The save panel (removed, lead decision 2026-11-11)

The legacy egui save/load panel (`draw_save_load`, `SaveLoadPanelState`,
ten slots) was an invented replacement panel. It is gone together with
every key that opened it:
- `S` in the galaxy view (save);
- F8 or Ctrl+L in Game Options (load);
- F9 or Ctrl+S in Game Options (save).

Code that only it used is gone too:
- `PanelAction::OpenSaveLoad` and `CloseSaveLoadPanel`;
- `GameOptionsState::suspended` (the window paused under the panel);
- the panel-to-window request bridge in `main.rs`;
- `show_save_load`;
- `crates/rebellion-render/src/panels/save_load.rs`. `SaveSlotInfo` moved
  to `game_options.rs`.

What stays:
- **Saving and loading.** They go only through the window's own rows, as
  in the original (`FUN_00406b80` -> `FUN_00408000` -> `FUN_00415600`).
- **Persistence.** `rebellion_data::save` still has `MAX_SAVE_SLOTS` = 10,
  `list_saves` and `slot_occupied`, unchanged.
- **Save files.** None is deleted or migrated.
  - `read_save_slots` lists only slots 0..5 (`game_options::SAVE_ROWS` = 6).
  - `GameOptionsState` keeps six rows.
  - Saves in slots 6..9 stay on disk (native `<saves>/<slot>.reb`, browser
    `rebellion_save_v14_N`) untouched and are not listed.
  - `list_saves` reads fixed slot paths and never scans the directory, so
    stray files cannot reach it.

`tools/interface-parity/game-options-smoke.mjs` is rewritten but has not
been run.
- **Already covered.** Load and save already go through the window's
  buttons earlier in the gate: Save row 1 at (110, 181) and Load row 1 at
  (610, 181), with their questions.
- **Dropped steps:**
  - F8 with a scroll to slot 10, then selecting and loading it (asserted
    `load_state_fingerprint slot=9` / `restored slot=9`);
  - F9 and the `noDeleteLabel` red-pixel check on the panel (the helper and
    the `pngjs` import are gone);
  - F8 and the `legacy-load` screenshot.
- **Replacement.** With slot 10 seeded in `localStorage`, the gate presses
  F1, F8 and F9 and takes the `slot-ten-unlisted` screenshot. It asserts
  that slot 10's payload and metadata stay in storage and that slot 9 is
  never loaded. The window stays open for the following Restart click.
- **Results label.** 'slot 10 verified load/delete' became 'slot 10 kept
  and unlisted'.

Docs updated: `ghidra/notes/game-options.md` (Compatibility) and
`agent_docs/save-load.md` (ten slots in data, six in the UI).

## Tests

Save panel stage:
- **Removed** with the panel: `load_mode_rejects_empty_slots`,
  `load_mode_accepts_only_an_occupied_selection` and
  `save_mode_allows_empty_slot_after_a_name_is_entered`. They tested the
  invented panel.
- **Shortened:** `saves_require_valid_names_and_slots_and_overwrites_require_confirmation`
  loses only its `suspended` step, which tested the removed panel's pause.
- **Moved from slot 9 to slot 5**, the original's last row:
  - `occupied_legacy_slots_wait_for_confirmation_and_cancellation_preserves_them`
    (renamed `occupied_slots_wait_...`);
  - the overwrite loop in the same `saves_require...` test;
  - `unreadable_native_saves_remain_occupied_and_unchanged`.

  These check the same assertions on a reachable row. The same `saves_require...`
  test now also rejects a save to slot 6.
- **New tests:**
  - `game_options::a_save_beyond_the_six_rows_is_neither_listed_nor_loadable`;
  - `options_save_tests::saves_beyond_the_six_rows_stay_on_disk_and_are_not_listed`,
    which uses a slot-9 file plus a stray `notes.txt`; both stay
    byte-identical.

  Both fail with `SAVE_ROWS` set back to 10, the old listing. So does the
  slot-6 rejection. The files were restored afterwards (`cmp`).
- `cargo test -p rebellion-render --lib`: 1417 pass, 6 ignored.
- `cargo test -p rebellion-app`: 141 pass.
- `cargo check -p rebellion-app --all-targets`, and for wasm32 with
  `interface-test-fixtures`: no errors.

Load Game screen stage:

- `cargo check -p rebellion-app --all-targets`: no errors. It shows no new
  warnings; those near the edits are older.
- `cargo test -p rebellion-app`: 140 pass, 3 ignored, plus 5 ignored.

## game-options-smoke.mjs runs again (2026-11-11)

The smoke had not passed since the save format and the startup changed.
It now:
- skips the intro movies until the main menu is active, and the new
  game's movie until the campaign runs, as `main-menu-states.mjs` does;
- reads the save format's version from the keys it finds;
- answers the save's own "Selected Game Saved Successfully" box;
- checks the loaded fingerprint against the saved one without the
  `verified=` field, which 66227a8f removed from the log line;
- drops the version-13 metadata step: 66227a8f made one save format the
  only one, and `slot_occupied` reads the current keys.

Production build (`web/`, served on port 8769, `CHROMIUM_PATH=/usr/bin/chromium`):
both sides pass every remaining step. Results in
`.artifacts/interface-parity/game-options-smoke-2026-11-11T05-21-33Z` (`results.json` sha256
`521475614be0e417d434f599ebf07cac804d4666763d349831727f6808693be9`).

## Batch b186 (the night's check on main 535c2e14)

The batch script now runs 29 interface gates, adding `cutscenes`,
`main-menu-states`, `main-menu-rest`, `game-options`,
`encyclopedia-placement` and this smoke. All 29 pass on Linux; workspace
tests 3561 passed, fixture tests 149. Tactical matrix
`2026-11-11T06-19-56-822Z-3890398`: 148 of 148.
