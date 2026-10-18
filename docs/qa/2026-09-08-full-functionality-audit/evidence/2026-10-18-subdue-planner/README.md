# The AI's Subdue planner (F-036)

Date: 2026-10-18.

`ghidra/notes/ai-mission-planning.md`, "The Subdue planner".

## The original

- Row 6 of the strategy (`FUN_004e8290`) keeps one Subdue planner per
  own uprising, plus one. It does so only while an own system is below
  70 support (controller bit `0x100`); otherwise it keeps none.
- The target (`FUN_0047d060`) is an own system in an uprising with no
  own fleet. It takes:
  - its kept target;
  - else, from the last gathering place, the nearest revolt;
  - else the best supported revolt in the sector with the most of them.

## Port

- `subdue_wanted`, `subdue_target`, and row 6 in `mission_strategy`. The
  port's AI sent no Subdue before.
- Removed `missions::record_admits_system` and its test. The removed
  heuristics were its only callers.
- No save format change.

## Tests

`ai.rs`:

- `the_strategy_keeps_a_subdue_planner_per_uprising_and_one_while_support_is_low`
  also checks that the planner gathers at the target, then orders the
  Subdue there;
- `a_subdue_target_is_kept_else_the_nearest_else_the_best_supported_in_the_busiest_sector`.
- `fail-without.txt`: without the `+ 1` and the kept target, both tests
  fail.

## Campaigns (`playtest-subdue-planner.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the Incite commit:

| | Before | After |
|---|---|---|
| Subdue orders (Empire / Alliance) | 0 | 330 (299 / 31) |
| Subdue successes / failures | 0 / 0 | 288 / 1,802 |
| Uprisings begun | 313 | 371 |
| Systems falling to no side | 184 | 27 |
| Systems at day 1500, summed (Alliance / Empire) | 487 / 351 | 376 / 462 |

Most uprisings now end without the system falling away. The Empire
keeps systems the Alliance used to gain.

## Gates

- `cargo test --workspace`: 2420 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the Subdue planner):
  - initial unchanged (`v1:6f750b483b72f71f`);
  - final `v1:1f09d2d3d2c77cf3` -> `v1:7ed462ebd95b759a`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `fa3b1cd8cd921bf8f0e7039148c8132a453e95191673d4cf9e1af1c11953c324`.
- The Fleet window gate passes all 48 cases (interface WASM
  `dff742c4b2a013a3696319fdadcc0f6f12c502d47530e28fa1d9d3cd4e6ad027`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.8 ms | 23.1 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 3.4 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`, `missions.rs`):
  - 16 mutants: 15 caught, 1 missed;
  - the miss was the cycle's Subdue arm, now covered by the strategy
    test's order check (verified by hand).
- `provenance-scan check`: 87 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
