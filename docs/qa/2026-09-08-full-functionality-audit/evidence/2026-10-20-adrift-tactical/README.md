# The tactical battle's destroyed fleet sets its characters adrift too (F-040)

Date: 2026-10-20. Stage M9b-4 of the "Misje" area ("Powrót, automatyczna
trasa i dryf postaci"), on the user's go-ahead.

## The original

`FUN_00556710` calls a character's slot `+0xb4` (`FUN_004ef2e0`) when its
container is destroyed. The original has one destruction path for a fleet,
whatever battle removed it, so a character on no mission goes adrift
(`Role_SetAdrift` `FUN_00534790`) and is captured, if at all, by the Adrift
order's detection. The notes are in `mission-lifecycle.md`.

## What the port did before

Stage M9b-2 (`2026-10-20-adrift-path`) fixed the strategic battle path
(`integrator.rs`). The player's tactical battle (`tactical_flow.rs`,
`apply_results`) kept a copy of the unsourced rule: a losing, emptied
fleet's characters became the winner's prisoners at once.

## Port

- `missions::cast_adrift` puts a removed fleet's characters at its system
  and sets those on no mission adrift. Both battle paths call it.
- The port's other removals of a fleet leave no character to set adrift:
  - a merge moves the characters to the joined fleet;
  - scrap disbands only a fleet without characters;
  - the Death Star kills the characters of the system it destroys
    (`death_star.rs`, the Death Star area).

## Tests

- `missions.rs`: `a_destroyed_fleets_free_characters_go_adrift_where_it_stood`.
- `tactical_flow.rs`:
  `destroyed_losing_fleet_sets_its_officers_adrift_and_clears_fleet_assignment`
  replaces the test that asserted the capture. It fails without
  `cast_adrift` (`fail-without.txt`).
- `cargo test --workspace`: 2495 passed, 0 failed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all 24
are identical to the stage before. The headless campaigns fight no tactical
battle, and the AI puts no characters aboard its fleets yet.

## Gates

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `697a5ea86fc2ab0d1238d5020dda3fe63e98093b770ff12c0b82f261aee7e4e0`);
  the seed-42 fingerprints are unchanged (`v1:57cfc0d727199c15` ->
  `v1:4dee6ff02fdbe5bc`).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.5 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.5 ms |

- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors; 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 4 mutants, 4 caught.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
