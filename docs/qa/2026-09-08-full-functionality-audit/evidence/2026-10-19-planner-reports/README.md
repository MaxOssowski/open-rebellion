# Mission Reports and aborts of the standing planners (F-036, save v83)

Date: 2026-10-19. The user's grant of 2026-10-18 covers the save format
change for work until 06:00.

`ghidra/notes/ai-mission-planning.md`, "Mission Reports of the standing
planners".

## The original

- The Diplomacy, Incite and Subdue planners read their missions'
  reports through `FUN_0047c890`:
  - a failure counts against 5 (Diplomacy, Subdue) or 10 (Incite). Past
    the limit the planner aborts; otherwise it checks its target;
  - a success clears the count and checks the target.
- State 1 (`FUN_0047b250`) acts only while the mission runs:
  - a target that no longer qualifies turns the check into an abort;
  - the abort orders `0x250` on the mission (end code 2).

## Port

- `AISystem::mission_planner_reports` runs before the planners' cycles.
- `MissionPlanner::mission`, `report`, `failures` and `pending` (save
  v83).
- `bind_mission` and `note_mission_results` now reach the standing
  planners.

## Tests

`ai.rs`:

- `a_diplomacy_planner_aborts_its_mission_once_the_target_is_no_candidate_or_after_five_failures`;
- `incite_bears_ten_failures_and_subdue_checks_its_revolt`;
- `a_report_binds_to_its_standing_planner_and_a_failure_also_checks_the_target`.
- `fail-without.txt`: without the abort order, both of the first two
  tests fail.

## Campaigns (`playtest-planner-reports.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the Espionage commit:

| | Before | After |
|---|---|---|
| Abort orders | 216 | 871 |
| Diplomacy successes / failures | 3,572 / 1,829 | 2,471 / 1,338 |
| Incite successes / failures | 13 / 413 | 15 / 203 |
| Subdue orders | 307 | 437 |
| Systems at day 1500, summed (Alliance / Empire) | 415 / 412 | 399 / 416 |

## Gates

- `cargo test --workspace`: 2427 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v83, the reports):
  - initial `v1:cb9430c7635d6e7e` -> `v1:530e248f2599b293`;
  - final `v1:fe6f9d2ef366dfd9` -> `v1:45ce6a74e7ac0f51`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `f759f1e04813a3ff88760cdbaab21a6855d9911769e2ee159e2e207ef96ec421`.
- The Fleet window gate passes all 48 cases (interface WASM
  `46fe3dfcfb8639f8ccefd4c7d54901e9203eadc6f9f392b1e3db443e534349a9`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 22.9 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.3 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`):
  - first run: 52 mutants, 16 missed;
  - second run: 9 missed.
  - Closed by tests:
    - the standing guards (now one filter);
    - the wait for a running mission;
    - a failure's check;
    - both bits waiting at once.
  - Five that remain are equivalent, because the two bits are disjoint
    and any abort bit aborts:
    - `&`->`|` and `|`->`^` in the bit test at the loop's head;
    - `&`->`|` in the check test;
    - `!` deleted when the check becomes the abort;
    - `|`->`^` in the final clear.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
