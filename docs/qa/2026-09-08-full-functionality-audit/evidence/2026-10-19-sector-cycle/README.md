# The sector planners' mission planners run their cycle, stage 6 (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "The cycle".

## The original

`FUN_004bda60` runs states 3 to 9 after state 2 has set the target:

- 3, `FUN_004bdcd0`: the gathering place `+0x38` is the first of these
  that qualifies:
  - the previous place, while its record is the side's and not
    blockaded;
  - a team member's system in the sector `+0x74`;
  - a random such system of the sector;
  - in the nearest other sector, the side's system with no enemy fleet.
- 4, `FUN_0047b110`: keeps the members `FUN_0047ba40` passes.
- 5 and 6: the class's selectors fill the team and the decoys. The decoy
  selector lets the first decoy go past its cap (`FUN_0047b9e0`), and
  each selector sets its "full" bit (1 and 2).
- 7, `FUN_004be230`: Move `0x201` for every free member away from the
  gathering place. The planner is ready when no one is away or busy and
  both bits are set.
- 8, `FUN_0047b360`: the order.
- 9, `FUN_004be4c0`: without a target or place, releases the members
  unless its mission runs.
- The reports (slot `+0x30`: `FUN_0047f2c0`, `FUN_00480860`,
  `FUN_0047d4d0`):
  - Recon and Espionage successes mark the target seen;
  - failures count against slot `+0x24`, which is 10, or 5 for class
    `0x65a9b0`.

## Port

- `ai::sector_cycle`, `sector_staging`, `nearest_sector_random_system`,
  `sector_report`, and `mission_planning::team_size` and `decoy_limit`.
- `bind_mission` binds these planners' orders too.
- `adopt_mission_orders` no longer passes over these planners: they now
  order, so one aimed at a heuristic order's target stands for it. This
  reverses the stage-3 guard and its test case.
- port:
  - one Move per member;
  - the cycle runs once a day;
  - `0x40` is read as "outside the sector";
  - `+0x20` `0x80` is not kept.
- The `+0x64` bits are still not added, and the heuristic Sabotage and
  Assassination still run beside these planners (the next stage).
- No save format change.

## Tests

- `ai.rs`:
  - `a_sector_planner_s_mission_planner_gathers_in_its_sector_else_the_nearest`;
  - `a_sector_planner_s_mission_planner_gathers_its_full_team_then_orders`
    covers the moves, the order, the bound mission, the members staying
    while it runs, and the release;
  - `a_sector_planner_s_mission_planner_orders_only_with_its_team_and_decoys_full`;
  - `a_sector_planner_s_report_marks_its_target_seen_and_counts_failures`.
- `a_sector_planner_s_mission_planner_waits_in_the_cycle` is renamed
  `..._without_a_target_...`. Its port note is gone.
- `fail-without.txt`: with the cycle, the gathering place and the report
  stubbed out, 4 tests fail.

## Campaigns (`playtest-sector-cycle.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against stage 5.

| | Before | After |
|---|---|---|
| Missions dispatched | 11,976 | 19,149 |
| Reconnaissance | 1,682 | 6,450 |
| Assassination | 319 | 1,693 |
| Abduction | 579 | 808 |
| Sabotage | 2,268 | 2,680 |
| Two-member teams | 2,889 | 6,195 |
| Moves | 31,564 | 47,783 |
| Successes | 18,199 | 21,794 |
| Failures | 7,283 | 9,203 |
| Systems at day 1500, summed (Alliance / Empire) | 605 / 544 | 612 / 549 |

## Gates

- `cargo test --workspace`: 2460 passed, 0 failed.
- Seed-42 golden regenerated for this cause (the sector planners
  order):
  - the first change is at tick 10;
  - final `v1:dee3c02775ecfb47` -> `v1:ed40629cc5780d6d`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `8dd3c8f4e223d56db01402c4ff122efd39b00a57eff55d548050971922c92a7f`.
- The Fleet window gate passes all 48 cases (interface WASM
  `004b8592c14d91cef4fe5163dfa4513b8fbac69a86d1735222fa498bbcab804a`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 24.1 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.5 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - first run: 56 mutants, 10 missed;
  - second run: 56 mutants, 56 caught.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
