# Replacing jedi.rs: the simulation (F-038, stage 2)

Date: 2026-10-19.

`ghidra/notes/jedi-rs-replacement.md`. This is stage 2 of the tracker item
"Zastąpienie wymyślonego jedi.rs".

## Removed (no source in the original)

- The `JediSystem` day step:
  - `simulation.rs` step 13;
  - the app's copy in `main.rs`;
  - `Integrator::apply_jedi_events`;
  - the `StartJediTraining` extraction in both.
- The step's invented rules:
  - the tier advance on XP thresholds 50 and 150;
  - the 30-day detection at flat 5%, 10% and 20%;
  - the "reached ... tier" and "Force sensitivity discovered" lines.
- The telemetry `jedi_check`, `jedi_tier_advanced` and `jedi_discovered`.
- The three story events in `story_events.rs`:
  - "Force Awakening" (`0x1e1`) and "Force Training Milestone" (`0x1e5`).
    These are the original's `CharacterForceNotif` and
    `CharacterForceTrainingNotif` ids (`entity-system.md`), not messages;
  - "Leia Discovers the Force" (`0x363`, no original id). Leia's discovery
    is the trainer's `0x362`, ported in `force_discovery.rs`.
- `define_story_events` now defines no event.

What the original does in their place is already ported: the Force fields,
discovery, detection and growth (the note's first table).

## Kept until stage 4 (the save)

These remain because the save stores them:
- the event engine's tier conditions and actions (`EventState::events`
  keeps definitions);
- `Character::force_tier`, `force_experience` and `is_discovered_jedi`;
- `JediState`.

## Tests

- `story_events.rs`:
  - `the_invented_force_events_are_gone` replaces four tests of the
    invented events: `one_shot_story_events_do_not_refire`,
    `force_milestone_events_fire_at_correct_tiers`,
    `leia_force_discovery_fires_after_dagobah` and
    `leia_force_does_not_fire_before_dagobah`;
  - the one-shot rule itself stays covered in `events.rs`.
- `cargo test --workspace`: 2471 passed, 0 failed.

## Golden (named cause: the three invented story events removed)

- The seed-42 state held the three definitions:
  - initial `v1:094e77fcd3b4b666` -> `v1:6bc0edeba372c686`;
  - final `v1:beb63be8a01b2699` -> `v1:f2b8571c7dec63a3`.
- Two recordings are identical, and only the fingerprints changed.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `c544c595953407dd1d30bc0950f651fac28daa4836250e54c73882710acac00d`).

## Campaigns (named cause)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all 24
change.
- Step 13 drew one roll a day from the shared stream. Its removal shifts
  every later draw.
- In these campaigns no one started the invented training. Before the
  change, `jedi_tier_advanced` and `jedi_discovered` were both 0.
- `campaigns.txt` has the counts.

## Gates

- The Fleet window gate passes all 48 cases (interface WASM
  `e9b4a701d18b96eea9d526977a9f58a876f23fd3a4d291d300cabe0b98181057`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.5 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 1 mutant, missed. It is equivalent: it
  replaces `define_story_events` with `()`, and the function now does
  nothing.
- `provenance-scan check`: 85 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
