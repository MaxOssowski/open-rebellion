# The AI's module 2: construction yard sites, mines and refineries (F-036)

Date: 2026-10-15.

Module 2 (`FUN_004e1e50`, step `FUN_004e2120`) keeps its own construction
yard sites (mark `0x1000000`). From a kept site with an idle yard it runs
the mine or refinery cycle (`FUN_004e25c0`), building one at a time.
Without one it runs the yard cycle (`FUN_004e2400`): it chooses a target,
borrows module 1's nearest idle yard with the yard token `0x40`, and
builds more construction yards there. It is read in full in
`ghidra/notes/ai-production-modules.md`, "Module 2", with every query's
arguments taken from the assembly.

Seven of its functions are module 1's, compiled again with the fields
moved. Their instructions match module 1's one for one, apart from the
offsets, the mark and the token. Module 1's steps became shared functions
taking the module's fields, and module 1's tests still pass through thin
adapters with their assertions unchanged.

Differences from module 1 that the port keeps:
- the mine and refinery cycle needs no yard token;
- state 6 (scrap a spare yard) is reachable when the side has more yard
  maintenance than it wants;
- a target found by query leaves `+0x5c` at 0, so its Build asks for 0
  yards, which the validator refuses (`FUN_004dd810`).

## Port (save v42)

- `crates/rebellion-core/src/ai_module2.rs`: `Module2`, fields
  `+0x38..+0x68`, the step and both cycles.
- `ai.rs`:
  - `AIState::module2`;
  - the scheduler steps module 2 and sends it refusals;
  - the pass hands the yard token on unless module 1 or 2 holds it.
- The production heuristic's manufacturing facilities stood in for module
  2. Module 2 builds only construction yards, mines and refineries, so
  they now stand in for module `0xa` (shipyard placement). The test
  `each_production_order_belongs_to_one_module` changed its expectation
  from 2 to `0xa` for this reason.

port:
- The site lists are sorted trees in the original (`FUN_005f4f10`); both
  ports keep them in insertion order.
- The other six yard-token modules are not ported.

## Campaign (`playtest-module2.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`:
- Day 1: module 2 runs before module 1 (`FUN_00485a50` order). The
  Empire's module 2 takes Bothawui as its site, so module 1 no longer
  orders yards there; the Alliance's module 1 still orders one at Hoth.
- Day 2: the Empire scraps one manufacturing facility and orders a mine
  at Xyquine from Coruscant's yard.
- From day 215: the Empire orders a mine or a refinery every 12 days
  from Yaga Minor's yard (27 Destination and 27 Build orders in all).
- The Alliance's module 2 orders nothing. A traced run of the first days
  shows it holding the token on day 2 with no idle module-1 yard to
  borrow (Hoth's is building), so it goes to state 10.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 2946 | 0.5 ms | 1.3 ms | 26.2 ms |
| Fast | 60 s | 3530 | 0.4 ms | 0.9 ms | 3.4 ms |

No frame took over 50 ms. Days 1 to 33 ran, up to the Bothawui battle
window (`2-fast.png`). The console (`console.txt`) has 32 `[ai]` lines and
0 errors. `network.txt` lists 4 requests, all HTTP 200.

## Gates

- `cargo test --workspace --no-fail-fast`: 1908 passed, 0 failed.
  - 25 new tests in `ai_module2`, each citing its function.
  - `ai.rs`: `a_refused_module_two_order_ends_its_cycle_and_the_tick`,
    shown failing with module 2's refusal arm removed (`(1004, 9)`
    against `(0, 0)`).
  - One expectation changed, as above:
    `each_production_order_belongs_to_one_module` (2 -> `0xa`). In
    `a_production_module_issues_only_its_own_kind`, module `0xa` joins
    the modules that order nothing in that world.
- Seed-42 golden regenerated for save v42 and module 2's orders:
  - initial `v1:ada51196c4d497ba` -> `v1:09ab7bb25d2c1611`;
  - final `v1:56a75db30c685a00` -> `v1:b23375e49d5efe2d`.

  The replay equivalence check (`replay-equivalence.json`) passes on
  nine checkpoints with 0 browser errors. WASM SHA-256
  `d5be580f206f02c31ff1726a8971ca8a1375d1582f864b8ca42252eb36a8c24d`.
- `cargo mutants` on `ai_module1.rs` and `ai_module2.rs`:
  - Full run: 373 mutants, 347 caught, 5 missed, 14 unviable, 7
    timeouts. The timeouts are loop counters turned into endless loops,
    which the tests hang on.
  - Three tests were added for the gaps, and a retest of `site_near`,
    `choose_target` and `plan_yards` caught all 37 mutants.
  - The one survivor left is module 1's `& -> |` in `yard_class`, which
    is equivalent (`evidence/2026-10-15-ai-module-1/`).
  - `--in-diff` on `ai.rs`: 17 mutants, 15 caught, 1 unviable, and 1
    missed. The missed one (module 2's refusal arm) is covered by the new
    `ai.rs` test above.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in the changed files. rustfmt: `ai_module1.rs` and
  `ai_module2.rs` are clean; no findings in the other changed lines.

Hashes are in `SHA256SUMS`.
