# Betrayal: the invented defection removed (F-028, save v88)

Date: 2026-10-19. The user granted all permissions for this run, save
format changes included.

`ghidra/notes/betrayal-and-loyalty.md`. This is stage 2 of the tracker item
"Zdrada".

## Why

- The original's only betrayal is the mission phase `FUN_00589f10`. It is
  ported in `mission_detection::betray`.
- No function changes a character's side.
- The manual's "Traitors" says the same: a traitor betrays a mission.

The port's `betrayal.rs` had no source. It ran a 50-day check of
characters with base loyalty below 50, rolled UPRIS1TB, and switched them
to the other side.

## Removed

- `betrayal.rs`, `BetrayalState` (`SaveState`, `SimulationStates`,
  `LiveCampaign`).
- Step 10 in `simulation.rs` and in the app's day loop.
- `Integrator::apply_betrayal_events` and `GameEffect::CharacterBetrayed`.
- The telemetry `betrayal`, `betrayal_check` and `side_change`, and the
  betrayal setup of the ignored telemetry test.
- `agent_docs/systems/betrayal.md` and `jedi.md` now point to the notes.
- The provenance baseline: 56 -> 54.

## Save v88

- `SAVE_VERSION` 87 -> 88. A v87 save is refused, as every earlier version
  is.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:681087f1af0a0e6d` -> `v1:a2c72fbdc17546ff`;
  - final `v1:f2946c95622dd970` -> `v1:55c055d53d7ca5b8`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors (release
    WASM SHA-256
    `b4f280e7cdd8a17d37a787b92f65b67ba45140a99479b4a269fd2e3d164e6496`).

## Campaigns (named cause)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all 24
change.
- No defection had happened before: `side_change` was 0.
- The change comes from the shifted roll stream. Step 10 drew one roll per
  character a day.
- `campaigns.txt` has the counts.

## Gates

- `cargo test --workspace`: 2453 passed, 0 failed. `betrayal.rs`'s own
  tests went with it.
- The Fleet window gate passes all 48 cases (interface WASM
  `d26f00937fef399680b9b2d0c4be817c70174135dc236700bc2d0792f5d78a3a`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 27.2 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 7 mutants, 1 caught, 6 unviable.

Hashes of the committed files are in `SHA256SUMS`.
