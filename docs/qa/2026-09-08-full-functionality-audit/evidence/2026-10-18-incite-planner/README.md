# The AI's Incite planner (F-036)

Date: 2026-10-18.

`ghidra/notes/ai-mission-planning.md`, "The Incite planner".

## The original

- Row 7 of the strategy (`FUN_004e8290`) keeps one Incite planner, and
  one more per six candidates (`S+0x218`). A candidate is an opponent's
  system where the side's support is above 50 (record `+0x2c` `0x100`).
- The target (`FUN_00480370`) is chosen in this order:
  - the kept target while it is still a candidate with no enemy fleet;
  - the candidate nearest the last gathering place;
  - the candidate with the side's highest support, in a sector ranked
    by its opponent systems and a draw.

## Port

- `incite_wanted` and `incite_target`; `mission_strategy` keeps row 7.
- Removed the heuristic Incite. It went to the enemy system where the
  enemy was most popular, and most of those missions failed. With it
  went `evaluate_officers` and the first mission group.
- No save format change.

## Tests

`ai.rs`:

- `the_strategy_keeps_one_incite_planner_and_one_more_per_six_candidates`
  also checks that the planner's cycle orders the Incite on its target;
- `an_incite_target_is_kept_else_the_nearest_else_the_best_supported`;
- removed: `the_port_s_incite_goes_where_the_enemy_is_most_popular`,
  which tested the removed heuristic.
- `fail-without.txt`: with `/ 5` for `/ 6` and no kept target, both
  tests fail.

## Campaigns (`playtest-incite-planner.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the research-order fix:

| | Before | After |
|---|---|---|
| Incite orders | 659 | 404 |
| Incite successes / failures | 300 / 3,691 | 56 / 436 |
| All successes / failures | 16,866 / 8,839 | 16,140 / 5,654 |
| Systems at day 1500, summed (Alliance / Empire) | 500 / 340 | 487 / 351 |

## Gates

- `cargo test --workspace`: 2419 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the Incite planner):
  - initial unchanged (`v1:6f750b483b72f71f`);
  - final `v1:4a2572231a842e24` -> `v1:1f09d2d3d2c77cf3`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `392429a4ab2286aa22fd10c53a2b85a3cdd61615da1e404a08006f3b5519f11e`.
- The Fleet window gate passes all 48 cases (interface WASM
  `7e0dbec34ba9151a77028d3bfd7fffe6df9918a6c3550080e94df2cebd20bc5e`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 22.7 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 3.3 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`):
  - 22 mutants: 21 caught, 1 missed;
  - the miss was the cycle's Incite arm, now covered by the strategy
    test's order check (verified by hand).
- `provenance-scan`: 87 uncited items. `baseline` was rerun to record
  the smaller set: `find_incite_target` is gone, and the counts of
  earlier stages shrank.

Hashes of the committed files are in `SHA256SUMS`.
