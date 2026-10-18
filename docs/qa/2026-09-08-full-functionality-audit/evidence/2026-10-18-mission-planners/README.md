# Persistent AI mission planners (F-036, save v80)

Date: 2026-10-18.

The user approved the save format change and the AI architecture change
for this stage. The reading and the ported behaviour are in
`ghidra/notes/ai-mission-planning.md` ("Why the port's AI sends no
two-character team" and "Persistent mission planners").

## The original

- A mission planner lives across AI ticks (`FUN_004bced0`):
  - its selectors pick and hold the team and decoys (`FUN_0047b610`);
  - state 6 (`FUN_004bd0a0`) picks a gathering place: the target when it
    is the side's, else the side's system the nearness query finds, else
    the first member's system;
  - state 7 (`FUN_004bd570`) moves each member there;
  - state 8 (`FUN_0047b360`) orders the mission once all stand there.
- The port planned in one cycle and kept only the members standing with
  the first pick.

## Port

- `AIState::mission_planners`, `MissionPlanner` (save v80).
- `AISystem::adopt_mission_orders`: the heuristics' orders become
  planners. A planner already running for the same kind and target takes
  the new order's place, and members another planner holds are left out.
- `AISystem::mission_planner_cycle`: states 2, 6, 7 and 8, once a day per
  planner.
- `staging_point`: state 6.
- The evaluation holds every planner's members.
- `Planner::plan` keeps every pick, and `members_together` is removed.
- port:
  - the heuristics stand in for the strategy;
  - there is no refill;
  - a planner older than 60 days ends;
  - the nearness query is the side's unblockaded system nearest the
    target.

## Tests

- `ai.rs`:
  - `a_mission_planner_gathers_its_team_before_the_order`;
  - `a_mission_planner_gathers_at_the_side_s_system_nearest_an_enemy_target`;
  - `a_mission_planner_drops_members_it_cannot_keep_and_ends_when_empty_or_old`;
  - `each_order_is_its_own_planner_unless_one_runs_for_it_and_held_members_stay_put`.
- `mission_planning.rs`: `a_plan_keeps_both_picks_wherever_they_stand`
  replaces `an_order_keeps_only_the_members_standing_with_its_first`. That
  test asserted the port's own simplification, which the gathering now
  replaces.
- Shown failing without the change (`fail-without.txt`): with the
  gathering check loosened and the own-target staging removed, 2 planner
  tests fail.

## Campaigns (`playtest-mission-planners.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the previous commit:

| | Before | After |
|---|---|---|
| Missions dispatched | 41,485 | 13,590 |
| Two-member teams | 1,232 (3%) | 1,823 (13%) |
| Successes | 17,811 | 14,889 |
| Failures | 14,197 | 9,916 |
| Move orders | 320 | 23,522 |
| Systems at day 1500 (Alliance / Empire) | 26.3 / 19.8 | 26.6 / 17.0 |

- Teams now gather before they go. So fewer missions leave, a larger
  share are two-member teams, and fewer fail.
- No AI team holds two characters yet. The covert selectors find only
  special forces, and characters go on one-member missions. Learning a
  traitor (row 17) therefore still needs a player's team.
- During the stage, two faults were found and fixed before these runs:
  - a unit held by two planners shuttled between their targets;
  - Moves to an enemy target ended at once (code 8), until state 6's
    gathering place was ported.

## Gates

- `cargo test --workspace`: 2413 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v80, the planners):
  - initial `v1:e8ceaa386f41cc96` -> `v1:5ba66653ae1fb76d`;
  - final `v1:475c0c0f35968fba` -> `v1:7869f8168ab1782f`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `6797433be9adc84e3b0c3296cca5fb33a04ec032b0a994fabab1eccf70012299`.
- The Fleet window gate passes all 48 cases (interface WASM
  `896fe6f23b48cbcf2f8ae229dcf3a602bc5de72e2e2ed94b1454e7f2aa5ab7e9`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 29.3 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.3 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`, `mission_planning.rs`):
  - first run: 53 mutants, 13 missed (the staging distance and the
    adoption's matching);
  - second run: 1 missed, the held-decoy filter;
  - all three are closed by the tests above.
- `provenance-scan check`: 88 uncited items, unchanged.
- Native GUI acceptance was not run for this change.

Hashes of the committed files are in `SHA256SUMS`.
