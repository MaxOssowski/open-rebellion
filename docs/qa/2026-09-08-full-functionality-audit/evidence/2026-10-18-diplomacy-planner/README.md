# The AI's Diplomacy planner (F-036, save v81)

Date: 2026-10-18. The user's grant of 2026-10-18 covers the save format
change for work until 06:00 on 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "The Diplomacy planner".

## The original

- Row 5 of the strategy's requests (`FUN_004e8290`) keeps as many
  Diplomacy planners (`S+0x34c`) as the side wants (`S+0x214`):
  - the want is the systems whose record has `+0x2c` bit `0x40` (own
    below 70 support, or neutral, with no fleet and no uprising), then
    `/ 5 + 1` when above 0 (`FUN_0041b7e0`, `FUN_00418500`);
  - a surplus cancels the planner whose team has the least diplomacy
    (`FUN_004e5710`, slot `+0x18` `FUN_00481500`).
- A Diplomacy planner lives until it is cancelled. State 5 (slot `+0x50`,
  `FUN_00481720`) picks and claims its target:
  - it keeps the target while that is still a candidate;
  - otherwise it tries five queries in turn: an own system in the
    sector of the lowest average own support below 70, then neutral
    systems near the last gathering place, then mixed rankings;
  - it claims the target on its record (`+0x2c` `0x20000000`, kept by
    the refresh), which keeps other planners off it.

## Port

- `AISystem::diplomacy_strategy`, `diplomacy_wanted`, `diplomacy_target`.
- `MissionPlanner::standing` and `claimed`, `AIState::record_claims`, and
  `ai_records::apply_claims` (save v81).
- Removed the port's heuristic Diplomacy orders, which went to every
  system below a popularity cap of 80%, and the uprising prevention,
  which sent two diplomats a pass to own systems below 40%.
- port:
  - the requests reach the want within one step;
  - the strategy's other rows are not yet ported;
  - the Diplomacy planner's Mission Reports and abort (slots `+0x30`,
    `+0x34`, `+0x3c`) are not ported.

## Tests

`ai.rs`:

- rewritten for the new planner, with the same assertions:
  - `a_diplomat_needs_diplomacy_of_69`;
  - `a_diplomat_is_sent_only_where_the_diplomacy_record_admits_the_target`;
- new:
  - `the_strategy_keeps_one_diplomacy_planner_and_one_more_per_five_candidates`;
  - `a_surplus_diplomacy_planner_with_the_weakest_team_is_cancelled_and_frees_its_claim`;
  - `a_diplomacy_planner_takes_an_own_system_below_70_before_a_neutral_one`;
  - `a_diplomacy_planner_keeps_its_claimed_target_and_another_takes_the_next`;
  - `a_diplomacy_planner_without_a_team_frees_its_claim`;
  - `the_own_sector_counts_by_its_average_support_below_70`;
  - `only_standing_diplomacy_planners_count_against_the_want_and_each_has_its_own_id`;
  - `the_refresh_keeps_the_planners_claims_and_a_sector_with_none_open_loses_0x4000000`;
  - `a_claim_is_seen_at_once_and_a_planner_left_with_no_candidate_holds_no_claim`;
  - `the_strategy_adds_and_cancels_only_standing_diplomacy_planners`;
- removed, because they asserted the removed heuristics:
  - `each_diplomat_goes_to_another_system_below_the_cap_least_popular_first`;
  - `uprising_prevention_sends_diplomats_to_our_systems_below_40_percent`;
  - `uprising_prevention_skips_a_system_at_exactly_40_percent`.
- `fail-without.txt`: 4 of the new tests fail when the claims, the
  own-sector step and the `/ 5 + 1` are taken out.

## Campaigns (`playtest-diplomacy-planner.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the F-005 commit:

| | Before | After |
|---|---|---|
| Diplomacy orders | 1,381 | 789 |
| Diplomacy successes / failures | 3,667 / 1,862 | 3,610 / 1,780 |
| Recruitment orders | 862 | 1,703 |
| Ship / troop / facility research orders | 1,247 / 44 / 134 | 2,275 / 686 / 744 |
| Missions dispatched | 8,605 | 11,366 |
| Systems at day 1500, summed (Alliance / Empire) | 557 / 347 | 531 / 340 |

A Diplomacy mission repeats, so it reports many times. Fewer diplomats
reach the same number of results, and the freed characters research and
recruit.

## Gates

- `cargo test --workspace`: 2415 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v81, the Diplomacy
  planner):
  - initial `v1:5ba66653ae1fb76d` -> `v1:6f750b483b72f71f`;
  - final `v1:f7fe1a11ab5f9ffd` -> `v1:bec91e71a20394cd`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `46897700e08cab08c2ae5a86b55898f0f7240e527fc7e9785533774fa75eb4bf`.
- The Fleet window gate passes all 48 cases (interface WASM
  `ce295bd3457e20d1b2e7071754f36ee06f0efd1c25dee426bcab60c5bb02c4f0`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.1 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 3.5 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`, `ai_records.rs`):
  - first run: 23 missed, mostly the claims;
  - second run: 8 missed;
  - third run: 89 mutants, 86 caught, 1 timeout, 2 missed;
  - the last two were closed by test changes and checked by hand: each
    now fails a test.
- `provenance-scan check`: 88 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
