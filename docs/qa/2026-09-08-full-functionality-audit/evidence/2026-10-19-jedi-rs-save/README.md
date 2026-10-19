# Replacing jedi.rs: the save (F-038, stage 4, save v87)

Date: 2026-10-19. The user approved save v87 for this stage ("Ok
dokończ").

`ghidra/notes/jedi-rs-replacement.md`. This is the last stage of the
tracker item "Zastąpienie wymyślonego jedi.rs".

## Removed (no source in the original)

- `rebellion-core/src/jedi.rs`.
- `ForceTier` and `Character::force_tier`, `force_experience` and
  `is_discovered_jedi`.
- `JediState`, in `SimulationStates`, `SaveState` and the app's
  `LiveCampaign`.
- The event engine's:
  - conditions `CharacterHasForceLevel`, `CharacterIsForceUser` and
    `CharacterForceExperience`;
  - actions `ModifyForceTier`, `StartJediTraining` and
    `AccumulateForceExperience`.
- `GameEffect::JediTierAdvanced` and `JediDiscovered`, which nothing
  produced.
- `EVT_LEIA_FORCE` (`0x363`, no original id), its telemetry name, and the
  interface fixture's message for it. The fixture still posts Leia's real
  `0x362` messages.
- The provenance baseline drops the 8 uncited items that went with them:
  85 -> 77.

What the original keeps instead is already ported: `ForcePotential`,
`ForceAware`, Force `+0x8c` = `+0x8e` + `+0x90`, the discoverer,
discovery, detection and growth.

Kept: the story class for `0x1e1` (Force Growth, TEXTSTRA
`0x7180`/`0x7181`). It is the original's id and texts, though nothing posts
it yet.

## Save v87

- `SAVE_VERSION` 86 -> 87.
- Like every earlier version, a v86 save is refused with "Please start a
  new game" (`agent_docs/save-load.md`, the no-migration rule).
- The seed-42 golden is regenerated for this cause (the state's layout):
  - initial `v1:6bc0edeba372c686` -> `v1:681087f1af0a0e6d`;
  - final `v1:f2b8571c7dec63a3` -> `v1:f2946c95622dd970`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors (release
    WASM SHA-256
    `3535e6fa3331642fde364aa5dba0ddd98df46665d73e1150c0102f92f4e5a8e3`).

## Tests

- Removed with the code:
  - `jedi.rs`'s own tests;
  - three `events.rs` tests of the removed conditions;
  - one `story_events.rs` test of `CharacterForceExperience`;
  - one `seeds.rs` assertion on `force_tier`.
- `story_messages.rs`: the class test now checks the Force Growth class
  (`0x1e1`) where it had used the removed `0x363`.
- New, ignored, needs the data:
  `character_command_names.rs`
  `a_character_loads_its_class_flags_from_the_dat` checks Ackbar's
  `can_be_general` and `is_unable_to_betray`, and Mon Mothma's
  `can_be_general`. It closes two mutants that the loader edit's diff
  reached.
- `cargo test --workspace`: 2459 passed, 0 failed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all
24 runs are unchanged event for event against stage 2. The removed fields
had no effect on the game.

## Gates

- The Fleet window gate passes all 48 cases (interface WASM
  `ed35e2e9899c67d234bbd2911ef4c1ed0d3e459bd692f552fa27748aa1792a5d`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.5 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 5.1 ms |

- The first performance run had read 2.6 and 3.7 ms at the 99th
  percentile. It overlapped a mutants run, and the run above was
  repeated alone.
- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - `rebellion-core`, `rebellion-render`, `rebellion-data`: 17 mutants,
    8 caught, 6 unviable, 3 missed;
  - the 3 missed are loader mutants (`convert_character`), caught by the
    ignored data tests (3 of 3).
- `provenance-scan check`: 77 uncited items, all within the shrunk
  baseline.

Hashes of the committed files are in `SHA256SUMS`.
