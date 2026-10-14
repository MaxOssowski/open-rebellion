# The AI's fourteen modules, stages 2 and 3 (F-036)

Date: 2026-10-14.

## Stage 2: the modules

`ghidra/notes/ai-mission-planning.md`, "The fourteen modules", identifies
each module that `FUN_00485a50` adds, in the order `0xd`, 2, 1, 3, 6, 7, 4,
5, 8, 9, `0xa`, `0xb`, `0xc`, `0xe`. For each it gives the constructor and
vtable (`FUN_004bea20`), the step (slot `+0x14`), the orders its call tree
builds (`FUN_004f5cd0`), and the records it reads or marks. The names are
first reads:

- `0xd` strategy;
- 1 mines and refineries;
- 2 facility placement;
- 3 facility balance;
- 4 missions;
- 5 troop production;
- 6 garrison balance;
- 7 ship balance;
- 8 sector fleet planners;
- 9 fleet planners;
- `0xa` shipyard placement;
- `0xb` ship production;
- `0xc` movement planners;
- `0xe` ship names.

The scheduler rules come from the decompiles:

- A step that builds an order completes its module and raises the
  module's priority `+0x34` while its cycle goes on (`FUN_004db2f0`,
  `FUN_004d0010`, `FUN_004c8ba0`, `FUN_004d2490` state 6, ...).
- `FUN_00485b40` zeroes the priority before each step and ends the tick on
  such a step, so the AI issues at most one order a sub-tick.
- `FUN_004bede0` takes the highest priority, scanning from a cursor
  `+0x14` that moves on one module each call; otherwise the round robin
  `+0x10` (`FUN_004be980`) applies. With no module ready, `+0x10` stays
  where the search began.

## Stage 3: the port (save v39)

- `MODULE_IDS` holds the order.
- `next_module` covers `FUN_004be980` and `FUN_004bede0`.
- `step_module` issues one order a step and raises the priority while the
  module's undecided orders (`module_pending`) remain.
- `mission_step` runs one entry of the mission module's list a step
  (`mission_next`, `FUN_004d2730`): the research planners, then five groups
  of mission heuristics.
- The research strategy runs as module `0xd`.
- Production splits by kind (`production_module`): 2 manufacturing
  facilities, 3 defense facilities, 5 regiments, `0xb` ships and fighters.
- Module 8 holds the Death Star escort and fleet deployment; `0xc` holds
  troop moves.
- Modules 1, 6, 7, 9, `0xa` and `0xe` have no port counterpart and order
  nothing.

## Headless campaigns

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai --speed S`;
`playtest-speeds.json` holds each run.

| Speed | Orders | Most orders by one side in a day | Days with orders |
|---|---|---|---|
| fast | 151 | 4 | 98 |
| medium | 128 | 10 | 75 |
| slow | 184 | 10 | 96 |
| very-slow | 160 | 10 | 82 |

At Fast a side gives at most 4 orders a day, one per sub-tick. From Medium
on, the 10 a day bind.

## Browser

The release site is run with `agent-browser` (headless Chromium, 640 by
480), with the `[ai]` console line per day:

- **Fast:** days 1 to 32 at `passes=4`, at most 4 orders a day; the
  longest frame was 48.2 ms.
- **Medium,** a new campaign: `passes=12`, 10 orders on day 1, then 6, 2
  and 0; the longest frame was 2.1 ms.

`1-medium-day-16.png` shows the campaign at day 16 and Medium. The console
(`console.txt`, both runs: 32 Fast and 16 Medium days) has 48 `[ai]` lines and
0 errors.
`network.txt` lists 8 requests, all HTTP 200. Those runs
used the build before the `production_module` refactor.

A rerun on the committed build (SHA-256 `6451f492...`), Fast for 12 s,
gave the same orders on days 1 to 6 (4, 4, 3, 0, 3, 4 at `passes=4`).
The longest frame was 2.2 ms, with 0 console errors and 4 requests, all
HTTP 200 (`console-final-build.txt`, `network-final-build.txt`,
`2-final-build-fast.png`).

## Gates

- `cargo test --workspace`: 1762 passed, 0 failed. New tests fail without
  the change:
  - `a_module_with_orders_left_raises_its_priority_and_runs_first`
  - `equal_priorities_go_to_the_first_from_a_scan_that_moves_on_each_call`

  Also new:
  - `a_pass_issues_one_order_and_the_next_goes_on_after_its_module`
  - `the_round_robin_wraps_round_the_list_to_a_module_with_orders`
  - `the_day_s_tenth_order_stops_the_ai_until_the_next_day`
  - `a_mission_step_runs_one_entry_and_the_next_step_the_one_after`
  - `the_modules_run_in_the_order_the_ai_player_adds_them`
  - `each_production_order_belongs_to_one_module`
  - `a_production_module_issues_only_its_own_kind`
  - the v39 save round trip.
- Seed-42 golden regenerated for save v39 and the module scheduler:
  `v1:eb82b46bf03b0753` -> `v1:e271a795cd483ebe` (initial),
  `v1:4225657ae5e05393` -> `v1:33233aedba63db5c` (final).
  `scripts/check-replay-equivalence.py --skip-build --json` passes on all
  nine checkpoints, 0 browser errors. WASM SHA-256
  `6451f492a66709e47137ef41938078612b431b797997e44ced354b9c690295fa`.
- `cargo mutants --in-diff`: 54 mutants, 50 caught, 2 unviable, 2 timeouts.
  The timeouts are `next_module` returning a fixed module, which loops
  forever and hangs the tests. None missed.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy and rustfmt: no findings in the changed lines.

Still open: the contents of the modules. The port's heuristics stand in
for them, and modules 1, 6, 7, 9, `0xa` and `0xe` are not ported.

Hashes are in `SHA256SUMS`.
