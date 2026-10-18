# The AI's Recruitment planner (F-036)

Date: 2026-10-18.

`ghidra/notes/ai-mission-planning.md`, "The Recruitment planner".

## The original

- Row 1 of the strategy (`FUN_004e8290`) keeps two Recruitment planners
  while the side has MNCHARSD classes (`S+0xa4`, counted at setup).
- A planner's target (`FUN_0047dcc0`) is a system whose record has
  `+0x2c` bit 2: the side's, calm, while `S+0xa4 > 0`. It takes, in turn:
  - its kept target;
  - the recruiter's own system;
  - near its last gathering place, the nearest such system;
  - else the best supported one in the sector with the most of them.

## Port

- `AISystem::mission_strategy` and `keep_planners` (rows 1 and 5), and
  `recruitment_target`.
- Removed the heuristic Recruitment. It sent a recruiter to the most
  popular system for each character left in the pool.
- port: a standing Recruitment planner orders nothing while the pool is
  empty. Such a mission would end at once (code `0x10`).
- Fixed side `+0xa4` (`minor_classes`). It looked for family
  `0x38..0x3b`, but loaded characters carry no family byte, so a real
  game counted none. Then no system record had `+0x2c` bit 2.
- No save format change.

## Tests

`ai.rs`:

- rewritten for the planner, with the same assertions:
  - `a_recruiter_needs_leadership_of_80_and_someone_left_to_recruit`;
  - `the_strategy_keeps_two_recruiters_and_none_go_once_the_pool_is_empty`
    (it was `recruitment_sends_one_recruiter_per_character_left_to_recruit`,
    with the same counts, 2 and then 0);
- new:
  - `a_recruitment_target_is_the_recruiter_s_calm_system_else_by_count_and_support_else_by_distance`;
  - `the_port_s_incite_goes_where_the_enemy_is_most_popular`, which keeps
    the Incite heuristic covered until its row is ported;
- `a_side_s_minor_classes_are_its_minor_characters` now also counts a
  loaded minor character (no family byte);
- the Diplomacy fixtures place major characters, so their side has no
  Recruitment planner.
- `fail-without.txt`: the target's own-system step and the empty-pool
  rule removed fails 2 tests.

## Campaigns (`playtest-recruitment-planner.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the Diplomacy commit:

| | Before | After |
|---|---|---|
| Recruitment orders | 1,703 | 1,679 |
| Recruitment successes / failures | 1,117 / 345 | 1,224 / 409 |
| Ship / troop / facility research orders | 2,275 / 686 / 744 | 10,312 / 3,941 / 4,161 |
| Ship / troop / facility research successes | 3,041 / 276 / 420 | 4,922 / 1,044 / 1,677 |
| Missions dispatched | 11,366 | 26,519 |
| Systems at day 1500, summed (Alliance / Empire) | 531 / 340 | 487 / 363 |

The research order counts expose a fault that was already there. In 13
of 24 seeds before and 22 of 24 after, one designer is ordered to the
same site again every few days, 35 to 394 times. Each of those missions
ends without a result. This is the next stage.

## Gates

- `cargo test --workspace`: 2417 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the Recruitment planner and
  side `+0xa4`):
  - initial unchanged (`v1:6f750b483b72f71f`);
  - final `v1:bec91e71a20394cd` -> `v1:4a2572231a842e24`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `6a8e949bf9be4c193ef2ede5d360bd5deaf26942d741829970b24cdbcff8f7b6`.
- The Fleet window gate passes all 48 cases (interface WASM
  `3b40624337b20f77a6c8a2647d1799bc0ad39713c219756f9e93d3c4876e8ea5`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.1 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 3.4 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`):
  - 31 mutants: 29 caught, 1 timeout, 1 missed;
  - the miss was the Incite heuristic, which lost its only test with the
    old Recruitment tests;
  - `the_port_s_incite_goes_where_the_enemy_is_most_popular` closes it.
- `provenance-scan check`: 88 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
