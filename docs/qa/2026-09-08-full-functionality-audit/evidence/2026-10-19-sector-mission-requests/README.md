# Sector planners' mission requests, stage 1 (F-036, save v84)

Date: 2026-10-19. The user approved the save format change for this
stage.

`ghidra/notes/ai-mission-planning.md`, "The sector planners' mission
planners".

## The original

- Each plan kind's slot `+0x18` (`FUN_004b7150`) asks for Recon,
  Espionage, Sabotage, or Abduction/Assassination of subtype 4. It works
  from the mission planners the sector planner holds (`+0xa0..+0xac`)
  and the sector record. It may also ask to cancel one.
- The mission module's states 2 and 3 (`FUN_004d2610`, `FUN_004d2690`)
  take the first pending request from the cursor. The module cancels as
  asked, builds the requested planner, and hands it to the sector
  planner (`FUN_00476ba0`), which counts it.

## Port

- `SectorPlanner::missions` is `FUN_004b7150` in full. The port had
  asked only for Recon or Espionage.
- `AISystem::take_sector_request`, `AISystem::sync_sector_missions`,
  `MissionPlanner::owner` and `subtype`, the cursor
  `AIState::mission_take`, and `mission_planning::decoy_cap` (save v84).
- port: the held planners do not run their cycle (`FUN_004bda60`) yet,
  so they order nothing. Their `+0x64` bits are computed but not yet
  added:
  - without the cycle, no held planner ever completes its team, and its
    `0x400` would mark every sector planner short for good;
  - with the bits added, the Fleet window gate failed in two runs.
- The heuristic Sabotage and Assassination still stand in.

## Tests

- `ai_planner.rs`: `a_sector_planner_asks_for_one_mission_kind_by_what_it_holds`.
- `ai.rs`:
  - `the_mission_module_takes_a_sector_planner_s_request_and_holds_its_planner`;
  - `a_sector_planner_counts_each_held_kind_and_whether_each_is_complete`;
  - `a_cancellation_drops_the_held_planner_with_the_weakest_team`;
  - `a_sector_planner_s_mission_planner_waits_in_the_cycle`.
- `mission_planning.rs`: `each_class_wants_its_own_decoy_count`.
- `fail-without.txt`: with no Sabotage request and no planner built, 3
  tests fail.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days. All
24 runs are unchanged event for event against the leader-bit commit. In
a seed-1 run the sector planners held up to 10 mission planners.

## Gates

- `cargo test --workspace`: 2443 passed, 0 failed.
- Seed-42 golden regenerated for this cause (save v84, the held
  planners):
  - initial `v1:530e248f2599b293` -> `v1:20ee82a2951ad9aa`;
  - final `v1:49ba43802d0d2840` -> `v1:14ec5ed9881bd79b`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `f9a38ad640298928be3f5c1fd9ca84bdd1c888a6d1f9ed46847e25163dedf785`.
- The Fleet window gate passes all 48 cases (interface WASM
  `96cf4dba02e09cd5b2c5ba050e532952ca269d1ba78e8acfadf5e7b8598abe64`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.0 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.7 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`, `ai_planner.rs`,
  `mission_planning.rs`):
  - first run: 123 mutants, 27 missed;
  - second run: 122 mutants, 4 missed;
  - three were then closed by hand-checked tests;
  - the last, `/ 2` to `* 2` in Sabotage's cancel key, is equivalent,
    because it keeps the order.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
