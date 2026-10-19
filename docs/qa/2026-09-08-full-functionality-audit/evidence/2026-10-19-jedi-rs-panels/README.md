# Replacing jedi.rs: the panels on the real fields (F-038, stage 3)

Date: 2026-10-19.

`ghidra/notes/jedi-rs-replacement.md`. This is stage 3 of the tracker item
"Zastąpienie wymyślonego jedi.rs".

## Port

**The J panel**, now "The Force" (`panels/jedi.rs`).
- It lists the player's living characters that have a Force value,
  `ForceAware` or `ForcePotential` (`has_force`), strongest first.
- For each one it shows:
  - the Force `+0x8c` (`Character::force_level`);
  - the Status window's ranking word (`FUN_004486f0`'s thresholds,
    `status_window::force_ranking`);
  - the two flags;
  - the trainer who discovered it (`+0xa4`).
- It gives no order. Training is the Jedi Training mission.
- It drops the old tiers, XP bars, flat detection risk and the Begin and
  Stop Training buttons.
- port: the original has no such panel; it shows the Force only in the
  Status window's "Force Ranking:" row.

**The Officers panel.**
- The detail view shows the same summary.
- The roster badge names the ranking from Force 10 (`ranking_badge`).
- The "JEDI" role badge had followed the class's `jedi_probability`, which
  marked Ackbar, Wedge and others with no Force. It is now "FORCE", for a
  character that has a Force (`has_force`).

**Removed:** `PanelAction::StartJediTraining` and `StopJediTraining`, with
their handlers and `apply_panel_action`'s `JediState` parameter.

## Tests

- `panels/jedi.rs`:
  - `the_force_panel_lists_the_side_s_force_values_strongest_first`;
  - `a_roster_badge_needs_a_force_and_names_a_ranking_from_ten`.
- These are new functions, so the tests could not pass before them.
- `cargo test --workspace`: 2473 passed, 0 failed.

## Browser (`agent-browser`, release build, a new game)

- J opens "The Force": Emperor Palpatine, Force 150, Jedi Master; Darth
  Vader, Force 120, Jedi Master; both "Force aware, Force potential".
- O opens the Officers panel. FORCE marks Luke, Leia, the Emperor, Vader
  and the minor characters with a rolled potential, and no longer marks
  Ackbar or Wedge.
- The console has 0 errors.
- The screenshots show original artwork and are not committed:
  - `target/acceptance/jedi-panel-2/`;
  - their SHA-256 are in `screenshots.sha256`.

## Simulation

This stage touches only the render crate and the app's panel actions, so
the campaigns were not re-run. The seed-42 golden is unchanged
(`v1:6bc0edeba372c686` -> `v1:f2b8571c7dec63a3`).

## Gates

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `5520ea6a9d2eea886bb6bb478d718302fca37c857d96be0ad29438d625c3f373`).
- The Fleet window gate passes all 48 cases (interface WASM
  `be33b4e6237ca66a40846e00ca7083dffab909fddfef72d9ce02835ec4d0a4a3`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.8 ms | 23.3 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.8 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-render`): 31 mutants, 18 caught,
  3 unviable, 10 missed (`mutants-missed.txt`).
  - The missed ones are in the egui drawing (`draw_jedi`,
    `draw_character_detail`, `draw_role_badges`) and in the port's label
    colour (`ranking_color`). No test harness reaches egui drawing; the
    browser screenshots above cover it.
  - The decisions those functions make are pure functions with tests:
    `force_summary`, `force_rows`, `has_force`, `ranking_badge`.
- `provenance-scan check`: 85 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
