# The AI's module 5: special forces and training facilities (F-036)

Date: 2026-10-15.

Module 5 (`FUN_004cfb20`, step `FUN_004cfed0`) alternates two cycles.
- The special force cycle (`FUN_004cff20`) keeps a training facility site
  (mark `0x400000`) with an idle facility. It takes the next of the four
  special force kinds short of units, and trains the dearest class of
  that kind its budget pays for: 70% of 4% of the capacity, less the
  special forces' upkeep.
- The yard cycle (`FUN_004d0010`) is module 3's for training facilities.
  It borrows module 1's yard with the token 4 and builds training
  facilities from 30% of 4% of the capacity.

It is read in full in `ghidra/notes/ai-production-modules.md`, "Module 5",
with the queries' arguments taken from the assembly and the yard cycle's
functions compared instruction by instruction with module 3's.

Two findings decide the special force cycle:
- The kind counts `+0x2ec..+0x2f8` need record `+0x34` bit `0x800000`,
  which no code sets: `FUN_00402720` sets bits 0 to 13, `FUN_00402e80`
  the kind bits, and a search of the assembly for writes found none. So
  every kind counts as short, and the cycle trains the kinds in turn.
- The Retire query (`FUN_004d0610`) needs record flag `0x2000`. No code
  sets it, and `FUN_00402230` masks it off at every refresh (`0x7080440e`).
  So module 5 never retires a unit, and the Retire order (`0x242`) is not
  needed for this port.

## Port (save v44)

- `crates/rebellion-core/src/ai_module5.rs`: `Module5`, fields
  `+0x38..+0x80`, the step and both cycles.
- The steps shared with modules 1 to 3 take a `YardKind`
  (`CONSTRUCTION`, `TRAINING`): the yards' family, record count, upkeep,
  class flag and idle bit, the family a target scrap spares, and the
  adoption query.
  - `keep_site` (module 2) and `choose_target` (module 3) became shared.
  - `best_class` takes the area and a `ClassKey`, so the special force
    class is chosen by maintenance.
  - `build` maps regiments and special forces to their kinds.
- `Tallies` gains:
  - `special_upkeep` (`+0x2c4`, the side's special forces' class
    maintenance; `FUN_0041bc10`, `FUN_004f2990`);
  - `special_goal` (`+0x2fc`, the most of the sector records' `+0x140`);
  - `special_ready` (always 0).
- `ai::accepts`: a Build needs the side's yard of its class's area (a
  training facility for special forces); it used to require a
  construction yard for every Build.
- The heuristic's regiments stood in for module 5 and now stand in for
  module 6 (garrison balance): module 5 trains only special forces. The
  test `each_production_order_belongs_to_one_module` changed its
  expectation for a regiment from 5 to 6 for this reason, and
  `a_production_module_issues_only_its_own_kind` checks module 6 too.

port:
- Controller `+0x14` (the fallback site) and `+0x18` (cleared before a
  Build) have no port counterpart; their writers were not found.
- The other four yard-token modules (6, 7, `0xa`, `0xb`) are not ported.

## Campaign (`playtest-module5.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`:
- The Alliance trains special forces at Vagran from day 75: 44 Build
  orders, the four basic kinds (`0x3c000001..4`) in turn.
- The Empire trains them at Yaga Minor from day 391: 21 Build orders.
- With more agents the Alliance's missions rise from 17 (module 3's run)
  to 192, the Empire's from 69 to 144.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 1924 | 0.7 ms | 2.3 ms | 22.8 ms |
| Fast | 60 s | 2899 | 0.5 ms | 2.1 ms | 7.9 ms |

No frame took over 50 ms. The AI's longest day at Fast took 2 ms
(`console.txt`). The run reached the Bothawui battle window
(`2-fast.png`). The console has 32 `[ai]` lines and 0 errors.
`network.txt` lists 4 requests, all HTTP 200.

## Gates

- `cargo test --workspace --no-fail-fast`: 1958 passed, 0 failed.
  - 16 new tests in `ai_module5`, each citing its function, and
    `a_build_names_the_kind_of_its_class_family` in `ai_module1`.
  - `ai.rs`:
    - `a_refused_module_five_order_ends_its_cycle_and_the_tick`;
    - `a_special_force_build_needs_the_side_s_training_facility`;
    - `the_special_force_upkeep_counts_the_side_s_own_units`.
- Seed-42 golden regenerated for save v44 and module 5's orders:
  - initial `v1:76a53e974120e2d1` -> `v1:ed967029485e168a`;
  - final `v1:faeeaa2d4a9ed165` -> `v1:6523821eb3c89a1a`.

  The replay equivalence check (`replay-equivalence.json`) passes with 0
  browser errors. WASM SHA-256
  `dbcbed31c72ce732d993cb72e0a98faaf8f254a90e5bf8897483caf51147322d`.
- `cargo mutants --in-diff` (`ai_module5.rs` whole and the changed lines
  of `ai.rs`, `ai_module1.rs`, `ai_module2.rs`, `ai_module3.rs`):
  - 276 mutants: 256 caught, 8 missed, 10 unviable, 2 timeouts (loop
    counters turned into endless loops).
  - Five tests were added for seven of the gaps: the budgets at zero, a
    kind past the fourth, a training class without maintenance, a class
    of no known kind, and the Build kinds. A retest of the functions
    caught all 67 viable mutants.
  - The survivor left is `> -> >=` in `plan_special_forces`. It is
    equivalent: at `m * count == w`, `w / m` is `count`.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in the changed files. rustfmt: the module files are
  clean; no findings in the other changed lines.

Hashes are in `SHA256SUMS`.
