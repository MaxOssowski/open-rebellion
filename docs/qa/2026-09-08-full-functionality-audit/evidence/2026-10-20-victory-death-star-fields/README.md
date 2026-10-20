# The victory state's Death Star fields go (F-017, save v97)

Date: 2026-10-20 (overnight run, "Gwiazda Śmierci").

## What was there

`VictoryState::death_star_active` and `death_star_location` had no
source, and nothing read them. Two places wrote the location: the
simulation's and the app's planet destruction. The flag was never set to
true. The original's victory conditions word (`FUN_005180e0`, ported in
v79) needs neither.

## Port

- Both fields and both writes are removed.
- `destroy_planet` no longer takes the victory state.
- `DeathStarEvent::PlanetDestroyed`'s doc no longer asks callers to set
  the location.

## Save v97

- `VictoryState` loses two fields, so `SAVE_VERSION` 96 -> 97.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:82332ef108ec7e21` -> `v1:ba71ff33556e6215`;
  - final `v1:49a3738dca7ca496` -> `v1:f7529f2440ec6c60`;
  - two recordings are identical, and only the fingerprints changed.

## Gates (`f15`)

- 24 campaigns are identical to the stage before.
- The replay equivalence check passes on the new golden with 0 browser
  errors (release WASM SHA-256
  `9d444f629741ea4fdafeec8af9bb977b3aa66923ed64108568c5df09c5921c1f`).
- The Fleet window suite passes 50 cases; the Status window gate passes.
- `cargo test --workspace`: 2490 passed, 0 failed.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- `cargo mutants --in-diff`: no mutants, since the change only removes
  code.
- `provenance-scan check`: 53 uncited items, unchanged.

## Note

- The working tree still holds the user's uncommitted native-window change
  in `crates/rebellion-app/src/main.rs`.
- This commit stages only its own lines of that file.
- The WASM was built with the change present.
