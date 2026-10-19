# The sector planners' bits, and the heuristic covert missions removed, stage 7 (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "The bits and the end of the
heuristics".

## The original

- `FUN_00478000` adds each held mission planner's state to its sector
  planner's `+0x64`, reading the planner's `+0x20` as its last cycle left
  it:
  - `0x20000000` for any held planner;
  - `0x400` when bits 1 (team full) and 2 (decoys full) are not both set;
  - `0x80000000` with `0x100`, which state 4 (`FUN_0047b110`) sets when
    it keeps a member.
- The original has no heuristic covert missions. Every Sabotage and
  Assassination comes from a sector planner (`FUN_004b7150`, class
  `0x65a9b0` and `FUN_0047d380`).

## Port

- `sector_cycle` keeps bits 1, 2 and `0x100` in `MissionPlanner::pending`.
  `sync_sector_missions` reads them, and `SectorPlanner::missions` adds
  `mission_bits` to `requests`. The stage-1 port note that withheld them
  is gone.
- Removed:
  - `evaluate_espionage`, the heuristic Sabotage and Assassination;
  - `dispatch_planned`;
  - `adopt_mission_orders`;
  - `Planner::plan`;
  - `mission_planning::decoy_cap`, replaced by `decoy_limit`;
  - the constant `MAX_COVERT_OPS_PER_EVAL`;
  - the mission module's heuristic group.
- Kept for saves and saved configs:
  - `MissionEntry::Group`, for older saves;
  - `AiConfig::max_covert_ops_per_eval`, no longer read;
  - the cycle of planners without an owner that are not standing, which
    older saves may hold.
- `provenance-scan baseline`: 86 -> 85 uncited items. The removed constant
  was one of them.
- No save format change.

## Tests

- Updated, citing `FUN_00478000` reading `+0x20`:
  - `a_sector_planner_counts_each_held_kind_and_whether_each_is_complete`
    now sets the bits;
  - `a_sector_planner_s_mission_planner_orders_only_with_its_team_and_decoys_full`
    checks the bits each cycle leaves, and that it clears stale ones;
  - `a_sector_planner_asks_for_one_mission_kind_by_what_it_holds` checks
    `mission_bits` reaching `requests`.
- Selector tests that went through `Planner::plan` now use a test helper
  over `Planner::refill` and `Planner::sendable`. That is the pick and the
  order's filter (`FUN_0047b360`). Their assertions are unchanged, except:
  - `a_busy_best_candidate_still_takes_a_team_slot` now uses a fresh
    planner for its second plan. The removed `plan` freed a failed plan's
    picks, which the cycle does not do;
  - the Sabotage selector tests (`a_sabotage_team_needs_combat_and_espionage_of_51`,
    `an_alliance_commando_is_picked_for_sabotage`) ask `FUN_0047d520`
    directly.
- Removed with the code they tested:
  - `a_sabotage_goes_to_each_enemy_yard`;
  - `an_assassination_goes_where_a_free_enemy_major_stands_up_to_the_covert_cap`;
  - `covert_ops_capped_at_max_per_eval`;
  - `each_order_is_its_own_planner_unless_one_runs_for_it_and_held_members_stay_put`;
  - `a_plan_with_no_team_takes_no_decoys`, a `port:` behavior of the
    removed `plan`.
- `fail-without.txt`: without the bits, 3 tests fail.

## Campaigns (`playtest-sector-bits.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against stage 6.

| | Before | After |
|---|---|---|
| Missions dispatched | 19,149 | 18,345 |
| Sabotage (Alliance / Empire) | 1,133 / 1,547 | 342 / 1,190 |
| Assassination (all the Empire's) | 1,693 | 1,441 |
| Abduction (Alliance / Empire) | 499 / 309 | 757 / 339 |
| Reconnaissance | 6,450 | 7,493 |
| Successes | 21,794 | 22,207 |
| Failures | 9,203 | 9,220 |
| Systems at day 1500, summed (Alliance / Empire) | 612 / 549 | 636 / 539 |

## Gates

- `cargo test --workspace`: 2455 passed, 0 failed.
- Seed-42 golden regenerated for this cause (the bits and the removed
  heuristics):
  - the first change is at tick 10;
  - final `v1:ed40629cc5780d6d` -> `v1:1ddf726cecfd8477`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `b3be7aefd90154c2525cb9709735f4f0d2d4587282845d6ab3ca7da1563fc96c`.
- The Fleet window gate passes all 48 cases in 2 of 2 runs (interface
  WASM `c8335a5bc0bb059e54bb354821d49261d5a5e4f33d4dd0db97760cfae4ac6d79`).
  Stage 1 had withheld these bits after two failing runs, in
  `join-fleet` and `travel`. Those are the two cases `ddf17050` later
  found unstable without them, from the double click and the fixture's
  message race.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.8 ms | 24.4 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.5 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - 35 mutants: 28 caught, 7 missed;
  - the 5 on the bit clearing are now caught by the stale-bits case,
    checked by hand;
  - the 2 left (`|` to `^` in `TEAM_FULL | DECOYS_FULL`) are equivalent,
    because the bits are disjoint.

Hashes of the committed files are in `SHA256SUMS`.
