# The AI's cadence, stage 1 (F-036)

Date: 2026-10-14.

## The original

- The frame loop `FUN_0041dff0` steps one phase a call, 0 to `0xd` and round
  again. Phase 2 (`FUN_0041e300`) holds until the scheduler's sub-tick
  count (`+0xc`) moves, so the loop turns once a sub-tick. Phase 8
  (`FUN_0041cf20` -> `FUN_004361a0`) ticks the two players in turn until
  both return 1. Phase `0xd` (`FUN_0041e500`) at a day boundary (scheduler
  `+0x24`) calls `FUN_0041d050` -> `FUN_00436530`, which clears each AI's
  order count through slot `+0x54` (`FUN_00485de0`; its window `+0x424`
  stays 0).
- A day has as many sub-ticks as the speed's rate (`FUN_0051dea0`, side
  `+0xc4`, speed order `0x280`): 600 Very Slow, 60 Slow, 12 Medium, 4 Fast.
- The AI player's tick (`FUN_00485b40`) refreshes its records (phase 4),
  readies every module (phase 5), and runs module steps (phase 3): the
  highest priority ready module (`+0x34`, `FUN_004bede0`), else the next
  ready one after the last (`FUN_004be980`). It counts every order but a
  Rename `0x203` against `+0x428` = 10 (`FUN_00485a50`), and ends the tick
  once a module completes with an order, none is ready, or the budget is
  spent.
- The mission module (`FUN_004d2490`) runs one planner per step
  (`FUN_004d2730`: the one `+0x4c` names, else the first; then the next).

This corrects the audit's earlier claim (F-036) that the original has no
AI tick.

## The port (save v38)

- `ai::passes_per_day(speed)` gives the rate. `run_simulation_tick` and
  the app run that many `AISystem::pass` a day, one side then the other,
  and apply each pass's orders before the next. `AIState::begin_day`
  clears `orders_today` (`ORDERS_PER_DAY` = 10).
- The port's ten heuristics stand in for the fourteen unidentified
  modules, in round robin (`module_cursor`), the research strategy and
  planners first. The research module runs one planner a step
  (`research_next`). Priorities and one-order steps are not ported. Each
  module finishes in one step, and orders past the budget are dropped.
- `AI_TICK_INTERVAL` and `GameConfig.ai.tick_interval` are removed.
- The officers module targets only systems the Diplomacy and Incite
  records admit (`missions::record_admits_system`: `FUN_00523450` rules 2,
  3, 4 and 7, `FUN_005868c0`). Without that check, the daily cadence
  ordered Leia Organa's Diplomacy at Coruscant, an Empire system the
  record refuses (end 8), on every day of the seed-42 run: 1017 Diplomacy
  dispatches in 600 days.
- `rebellion-playtest --speed very-slow|slow|medium|fast` sets the clock's
  speed (default fast).

## Headless campaigns

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai --speed S`
(release); `playtest-speeds.json` summarises each run.

| Speed | Orders | Most orders by one side in a day | Days with orders | Run time |
|---|---|---|---|---|
| fast | 144 | 10 | 65 | 0.2 s |
| medium | 139 | 10 | 60 | 0.4 s |
| slow | 133 | 10 | 60 | 1.9 s |
| very-slow | 133 | 10 | 60 | 17.6 s |

The old weekly cadence gave 350 orders in 87 days. Diplomacy missions now
run and repeat: 11 dispatches, 228 results at fast speed. At Very Slow the
AI costs about 28 ms of native release time per game-day. A game-day lasts
60 s there, and all its passes run at the day boundary.

Still open: repeated Sabotage and Recruitment orders that end at once
(also present under the weekly cadence), the module identities, and a
browser or native run of the app's game loop. The replay gate covers the
simulation path only.

## Gates

- `cargo test --workspace`: 1757 passed, 0 failed. New tests fail without
  the change:
  - `a_pass_ends_at_the_first_module_that_orders_and_the_next_goes_on_after_it`
  - `the_day_s_tenth_order_ends_the_ai_s_orders_until_the_next_day`
  - `a_pass_starts_after_the_module_last_run_and_wraps_round_to_it`
  - `a_research_step_runs_one_planner_and_the_next_step_the_one_after`
  - `a_diplomat_is_sent_only_where_the_diplomacy_record_admits_the_target`

  Also new: `a_day_has_as_many_ai_passes_as_the_speed_s_rate`,
  `a_record_admits_a_system_target_by_its_side_uprising_and_state` and the
  v38 save round trip. The four tests of the 7-day interval are removed with
  it. The module tests run every module once through a test helper.
- Seed-42 golden regenerated for save v38 and the cadence:
  `v1:a6ee248d7d0194dc` -> `v1:eb82b46bf03b0753` (initial),
  `v1:68580f6dd8f9f6b0` -> `v1:4225657ae5e05393` (final). The config
  fingerprint changed with `tick_interval`.
  `scripts/check-replay-equivalence.py --skip-build --json` passes: native
  and WASM agree on all nine checkpoints, 4 requests, 0 browser errors.
  WASM `open-rebellion.wasm` SHA-256
  `a967e8091da8891b76d60a9f46aa14fbea5e48482884e7ab9204fd7caf46689f`.
- `cargo mutants --in-diff`: 56 mutants, 52 caught, 2 unviable, 2 missed:
  `main` in the app and in `rebellion-playtest`, which no test runs.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
