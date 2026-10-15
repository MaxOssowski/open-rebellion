# The AI's module 3: defense balance, mines and construction yards (F-036)

Date: 2026-10-15.

Module 3 (`FUN_004d6b20`, step `FUN_004d6d30`) balances the side's
planetary defenses. It runs three cycles in turn: surplus (`0x3ed`),
then defenses and mines (`0x3ee`), then yards (`0x3ec`).
- The surplus cycle (`FUN_004d6da0`) scraps the shields and batteries a
  system holds beyond its record's goals.
- The defense and mine cycle (`FUN_004d6e70`) starts from one of its own
  sites (mark `0x2000000`) with an idle construction yard. It builds the
  strongest defense class where defenses fall short (Coruscant first), or
  scraps one when defenses cost more than 90% of 9% of the side's
  capacity. With nothing short, it builds a mine or a refinery. When no
  system is short, it raises the side's shield goal `+0x374` once
  (`FUN_00418240(4)`).
- The yard cycle (`FUN_004d7060`) borrows module 1's nearest idle yard
  with the yard token 1 and builds construction yards, from 10% of 9% of
  the capacity.

It is read in full in `ghidra/notes/ai-production-modules.md`, "Module 3",
with every query's arguments taken from the assembly. Eight of its
functions are module 1's or 2's with the fields moved (assembly compared
instruction by instruction). Module 2's site keeping and site scrap, and
module 1's class choice, became shared functions.

Points the assembly settled:
- States 3 and 7 of the surplus cycle are unreachable.
- A failed mine site still runs state 10, which leaves the next step in
  the yard cycle's state `0xb`, as in module 1.
- The class choice's second argument `0x2a` is the construction yard's
  build list, not a family: defenses come from it too, by flag
  (`0x20000`, `0x40000`, `0x80000`) and by key. Key `0x10000` is the
  class record's shields, `0x40000` its attack (`FUN_00473250`).
- Controller `+0x1dc` is the side's defense maintenance, the sector
  records' `+0x100` (`FUN_0041b7e0`).

## Port (save v43)

- `crates/rebellion-core/src/ai_module3.rs`: `Module3`, fields
  `+0x38..+0x74`, the step and the three cycles.
- `Controller::shield_goal` is `+0x374`; the system records read it
  (`SystemInputs::shield_goal`, formerly a constant).
- `Tallies::defenses` is `+0x1dc`.
- `ai.rs`: `AIState::module3`; the scheduler steps it and sends it
  refusals; the pass keeps the yard token with modules 1, 2 or 3.
- The production heuristic's defenses stood in for module 3 and are gone,
  since module 3 builds them. The test
  `each_production_order_belongs_to_one_module` changed its expectation
  for a defense from 3 to none for this reason.
- `the_round_robin_wraps_round_the_list_to_a_module_with_orders` used
  index 3 for a module with a decided order. Index 3 is module 3, which
  now runs its own cycle, so the example moved to index 4 (module 6),
  with the cursor expectations moved by one. The assertions are
  unchanged.

port:
- The other five yard-token modules are not ported.
- The site lists are sorted trees in the original; the port keeps them in
  insertion order.

## Campaign (`playtest-module3.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`:
- Module 3 scraps a surplus defense four times for the Empire (days 1 to
  4) and once for the Alliance (day 1). Once, in its yard cycle, it also
  scraps a facility at its target.
- Its defense cycle never starts. It has no site with a construction
  yard, and its yard budget (10% of 9% of a capacity of 650 to 1000, so
  5 to 9) stays below one construction yard's maintenance (10,
  MANFACSD). So it never builds or adopts one.
- The AI therefore builds no defenses in this campaign, where the
  heuristic used to build them.
- From day 373, modules 1 and 2 scrap a mine or refinery about every 21
  days at their yard targets with no free energy (`FUN_004db820`), as the
  original's code does. A temporary trace attributed every Scrap order to
  its module.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3748 | 0.5 ms | 0.7 ms | 1.8 ms |
| Fast | 60 s | 3746 | 0.4 ms | 0.8 ms | 49.0 ms |

No frame took over 50 ms. The AI's longest day at Fast took 2 ms
(`console.txt`), so the 49 ms frame is not the AI. Module 1's run had a
46.3 ms frame at the same point, the Bothawui battle window
(`2-fast.png`). The console has 32 `[ai]` lines and 0 errors.
`network.txt` lists 4 requests, all HTTP 200.

## Gates

- `cargo test --workspace --no-fail-fast`: 1938 passed, 0 failed.
  - 29 new tests in `ai_module3`, each citing its function.
  - `ai.rs`: `a_refused_module_three_order_ends_its_cycle_and_the_tick`.
- Seed-42 golden regenerated for save v43 and module 3's orders:
  - initial `v1:09ab7bb25d2c1611` -> `v1:76a53e974120e2d1`;
  - final `v1:b23375e49d5efe2d` -> `v1:faeeaa2d4a9ed165`.

  The replay equivalence check (`replay-equivalence.json`) passes with 0
  browser errors. WASM SHA-256
  `18a1a6c76c524ee234971adc0db771f587657b44b7be102da4fdbade7fa66ad9`.
- `cargo mutants --in-diff` (`ai_module3.rs` whole and the changed lines
  of `ai.rs`, `ai_module1.rs`, `ai_module2.rs`, `ai_records.rs`):
  - 300 mutants: 278 caught, 5 missed, 16 unviable, 1 timeout (a loop
    counter turned into an endless loop).
  - Three tests were added for four of the gaps: the defense cycle's
    end, the budgets at equality, and a yard class without
    maintenance. A retest of `after`, `defense_budget`, `plan_yards` and
    `yard_budget` caught all 51 mutants.
  - The survivor left is `& -> |` in `best_class`'s flag filter. It is
    equivalent: every class record in a construction yard's list
    carries exactly one family flag (`FUN_00473090`, `FUN_00473290`).
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in the changed files. rustfmt: the three module
  files are clean; no findings in the other changed lines.

Hashes are in `SHA256SUMS`.
