# The AI's module 1: construction yards, mines and refineries (F-036)

Date: 2026-10-15.

Module 1 (`FUN_004da950`, step `FUN_004dabe0`) keeps the side's
construction yards ("sites") and builds mines, refineries and more yards
with them. It is read in full in `ghidra/notes/ai-production-modules.md`,
"Module 1", with the uncertain decompiles checked in assembly. Assembly
settled three points:
- state 6 always runs state 7;
- state 4 never reaches state 5: its 5 is overwritten by 6 at
  `0x4dbe48`;
- the kept site goes to `+0x44`.

## Port (save v41)

- `crates/rebellion-core/src/ai_module1.rs` holds:
  - `Module1`: fields `+0x38..+0x60`, the step and every cycle state;
  - `Controller`: the yard token the modules share (`FUN_004199b0`,
    `FUN_004178f0`) and the no-site flag;
  - `Tallies`: the side's mines and refineries from its sector records,
    and its maintenance capacity and requirement.
- Orders: `AIAction::Scrap { team }` (the whole team or nothing,
  `scrap::scrap_team`), `SetDestination` (`0x214`) and `Build`
  (`0x210`, with a count).
- `ai::accepts` validates module 1's orders (slot `+0x1c`). A refused
  order runs `FUN_004dd810`.
- The scheduler (`FUN_00485b40`) now steps a module until it sets `+0x20`.
  A completed step with an order ends the tick. The heuristic modules
  complete every step, so their behaviour is unchanged.
- The record `+0x30` bits a refresh keeps (`0x35ff0000`) live in
  `AIState::record_kinds`; module 1's claim `0x10000000` is one of them.

port:
- **Construction manager:** one stands while the side has a
  construction yard in the system (hyp).
- **Yard token:** eight modules share it, and only module 1 is ported, so
  a pass hands on the yards that an unported module holds.
- **Coin:** the ranked lists' coin draws from the planner's generator.

## Campaign (`playtest-module1.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`:
- Day 1: the Empire orders 2 construction yards at Bothawui and the
  Alliance 1 at Hoth (Destination, then Build).
- After that, each site's yards stay busy with the heuristics' queues,
  and Bothawui is blockaded from about day 33. No site is idle, so the
  cycle waits. The original waits under the same conditions.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 40 s | 2500 | 0.5 ms | 0.7 ms | 2.1 ms |
| Fast | 20 s | 1246 | 0.4 ms | 1.2 ms | 46.3 ms |

No frame took over 50 ms. Days 1 to 33 ran; day 1 gave 4 orders. The
console (`console.txt`) has 33 `[ai]` lines and 0 errors. `network.txt`
lists 4 requests, all HTTP 200. The screenshots `1-veryslow.png` and
`2-fast.png` show the Bothawui battle window at day 33.

## Gates

- `cargo test --workspace --no-fail-fast`: 1882 passed, 0 failed.
  - New tests: 38 in `ai_module1`, plus the following.
    - `ai.rs`:
      - `a_module_that_has_not_completed_steps_again_in_the_same_pass`
      - `the_validator_checks_module_one_s_orders`
      - `a_refused_module_one_order_ends_its_cycle_and_the_tick`
      - `the_side_s_maintenance_leaves_out_suspended_systems`
    - `ai_records`:
      - `repeated_picks_give_the_best_then_the_next`
      - `a_module_s_bits_change_one_record_and_its_sector_is_found`
    - `scrap.rs`: `one_refused_member_keeps_the_whole_team`.
    - `mission_planning`: `the_planner_s_coin_is_a_draw_below_five_of_ten`.
    - Integrator: `module_one_s_destination_and_build_reach_the_manager`.
  - One fixture changed: `the_round_robin_wraps_round_the_list_to_a_module_with_orders`
    used index 2 for a module with a decided order. Index 2 is module 1,
    which now runs its own cycle, so the example moved to index 3 (module
    3), with the cursor expectations moved by one. The assertions are
    unchanged.
- Seed-42 golden regenerated for save v41 and module 1's orders:
  - initial `v1:e187411365b9f348` -> `v1:ada51196c4d497ba`;
  - final `v1:6685c6f382ede031` -> `v1:56a75db30c685a00`.

  The replay equivalence check (`replay-equivalence.json`) passes on
  nine checkpoints with 0 browser errors. WASM SHA-256
  `a023c709177f06c40bd0853c4571bb76a48d62dda9de6f6b5965dfe78f00645d`.
- `cargo mutants --in-diff` (`ai_module1.rs` whole, plus the changed
  lines):
  - First run: 323 mutants, 135 missed.
  - After the first batch of new tests: 324, 16 missed, 286 caught, 16
    unviable, 6 timeouts.
  - Retest of the functions with survivors: 170, 1 missed, 158 caught, 4
    unviable, 7 timeouts. The timeouts are loop counters turned into
    endless loops, which the tests hang on.
  - The one survivor is `& -> |` in `yard_class`'s flag filter. It is
    equivalent: a family `0x2a` class record carries only `0x10000`
    (`FUN_00473090`).
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in the changed files. rustfmt: `ai_module1.rs` and
  `scrap.rs` are clean; no findings in the other changed lines.

Hashes are in `SHA256SUMS`.
