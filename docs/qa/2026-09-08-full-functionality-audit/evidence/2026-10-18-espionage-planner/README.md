# The AI's Espionage planner (F-036)

Date: 2026-10-18.

`ghidra/notes/ai-mission-planning.md`, "The Espionage planner of subtype
6".

## The original

- Row 12 of the strategy (`FUN_004e8290`) keeps one Espionage planner
  (subtype 6) per sector holding an opponent's system, less the side's
  sector planners.
- The team (`FUN_00480f30`) is one special force with espionage 51 or
  more.
- The target (`FUN_00481110`) is the kept one while it is still the
  opponent's. Otherwise, in a sector near the last gathering place or
  drawn, it is the opponent's system seen longest ago.
- The planner claims the sector (record `+0x28` `0x20000000`).

## Port

- `espionage_target` and row 12 in `mission_strategy`.
- The Espionage selector is now subtype 6's: one member and no decoys.
- Removed the heuristic Espionage on unexplored systems, and with it
  `system_strength`, which had no caller.
- No save format change: the sector claims share v82's map.

## Tests

`ai.rs`:

- new:
  - `the_strategy_keeps_an_espionage_planner_per_enemy_sector_beyond_the_sector_planners`;
  - `an_espionage_planner_takes_the_enemy_system_seen_longest_ago_and_claims_its_sector`
    (also checks the order);
  - `an_unexplored_enemy_system_is_no_espionage_target`;
- removed, because they tested the removed heuristic:
  - `ai_dispatches_espionage_on_unexplored_system`;
  - `espionage_goes_to_unexplored_systems_up_to_the_covert_cap`;
- the Recon and Incite tests now count only their own kind, because an
  Espionage planner joins in their worlds.
- `fail-without.txt`: without the sector claim and the sector planners'
  share, 3 tests fail.

## Campaigns (`playtest-espionage-planner.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the Reconnaissance commit:

| | Before | After |
|---|---|---|
| Espionage orders | 1,296 | 1,840 |
| Espionage successes / failures | 2,460 / 602 | 1,697 / 687 |
| Systems at day 1500, summed (Alliance / Empire) | 483 / 476 | 415 / 412 |

## Gates

- `cargo test --workspace`: 2424 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the Espionage planner):
  - initial unchanged (`v1:cb9430c7635d6e7e`);
  - final `v1:3c9e4e1739d78f43` -> `v1:fe6f9d2ef366dfd9`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `83168088a42c9475deb7b9002117d42ccee7a00d684fc090f1e92b8fb1109794`.
- The Fleet window gate passes all 48 cases (interface WASM
  `da2332d962293c7eee2957a15d77e4d545520efed427f778a96a4ae7485e2cf7`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.3 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.3 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`, `ai_records.rs`,
  `mission_planning.rs`):
  - 29 mutants: 26 caught, 1 unviable, 2 missed;
  - both misses were the keep test's `0x200`, now closed by
    `an_unexplored_enemy_system_is_no_espionage_target` (verified by
    hand).
- `provenance-scan`: 86 uncited items. `baseline` was rerun to record
  the smaller set, without `system_strength`.

Hashes of the committed files are in `SHA256SUMS`.
