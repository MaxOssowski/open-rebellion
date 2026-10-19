# The AI controller's leader bit (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "The controller's leader bit".

## The original

- `FUN_004182a0` sets controller `+4` bit 0 when the game's setting `+8`
  (`FUN_004fce50`) is 1.
- The Objectives lists (`FUN_0044b2d0`, `FUN_0044bb70`) read the same
  field as the game type, so the bit marks a Headquarters Only game.
- While it is set, `FUN_00401d20` marks no record as a leader. The
  selectors that forbid a leader may then take one.

## Port

- `AIState::set_game_type`, called each day by the simulation and the app
  from the campaign's setup.
- `LEADERS_UNMARKED`, `Planner::without_leaders`, and
  `AISystem::side_planner`.
- Before, the port read the bit as always clear.
- No save format change: the controller's flags are already saved.

## Tests

- `ai.rs`:
  - `the_game_type_sets_the_controller_s_leader_bit`;
  - `a_pass_s_records_mark_no_leader_in_a_headquarters_game`. It fails
    without the bit (`fail-without.txt`).
- `mission_planning.rs`: `in_a_headquarters_game_no_record_is_a_leader`.
  Without the leader bit, Luke may go on an Incite.

## Campaigns

The headless runs play the standard game. All 24 runs of 1500 days are
identical to the Diplomacy-trade commit's, event for event.

## Gates

- `cargo test --workspace`: 2437 passed, 0 failed.
- Replay equivalence passes with the unchanged golden
  (`v1:530e248f2599b293` -> `v1:49ba43802d0d2840`) and 0 browser errors.
  Release WASM SHA-256
  `2d505b3f6f1b246f45ef51c20e6f0dbad176e8acbf9afde83a28407c341501b5`.
- The Fleet window gate passes all 48 cases (interface WASM
  `e948f09067cd9931e7f91f54876c35486ef2e07a2528c084d946286c0e1a0358`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.3 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.4 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`, `mission_planning.rs`): 16 mutants,
  13 caught, 3 unviable.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
