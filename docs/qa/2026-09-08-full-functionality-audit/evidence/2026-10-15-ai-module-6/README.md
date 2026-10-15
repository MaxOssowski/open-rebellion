# The AI's module 6: regiments and garrisons (F-036)

Date: 2026-10-15.

Module 6 (`FUN_004dd460`, step `FUN_004dd7a0`) balances the side's
regiments in three cycles that run in turn:
- **Moves** (`FUN_004dd840`): Move `0x201` of regiments from a system with
  a surplus to one short of garrison, the weakest (lowest class defense)
  first, up to the shortfall `v[21]` and at most the surplus `v[19]`.
- **Regiments** (`FUN_004dd8d0`): keep a training facility site (mark
  `0x400000`, shared with module 5), choose a destination by five queries
  in a rotating order (`FUN_004df440`), pick the regiment class each asks
  for (defensive by defense or maintenance, offensive by attack), train
  what 80% of 22% of the capacity pays for (less the regiments' upkeep on
  planets and aboard fleets), and send them there: Destination, then
  Build `0x212` on the troop manager.
- **Training facilities** (`FUN_004dda00`): module 5's yard cycle with the
  token `0x20` and 20% of 22% of the capacity.

It is read in full in `ghidra/notes/ai-production-modules.md`, "Module 6".

The regiment Move needed no new mechanic: the port already moves a
regiment on its own between systems with a transit
(`TroopTransportState::move_regiment`, `regiment-unload.md`). That note
still said the port refused it; it now says what the port does.

## Port (save v45)

- `crates/rebellion-core/src/ai_module6.rs`: `Module6`, fields
  `+0x38..+0x84`, the step and the three cycles.
- `AIAction::MoveRegiments { troops, to }`, carried out by the integrator
  and the interactive loop with `move_regiment` for each regiment.
  `TroopTransportState::check_move_regiment` runs the same checks without
  the move. `ai::accepts` uses it for each member, so a refused member
  refuses the order, as the move command's validation does
  (`FUN_00578d00`).
- `BlockadeSystem::blockaded_now` gives the blockade set the move needs,
  as `advance` would store it.
- `ai::accepts` checks a Destination against the manager of its area: a
  training facility for the troop manager.
- `Tallies` gains:
  - `regiment_upkeep` (`+0x1d8`, the sector records' `+0xfc`);
  - `cargo_upkeep` (`+0x250`, own fleet records' `values[15]`, the
    regiments' upkeep aboard);
  - `sector_short` (controller `+4` bit `0x20`).
- The heuristic's regiments are gone: module 6 trains them. The test
  `each_production_order_belongs_to_one_module` changed its expectation
  for a regiment from 6 to none for this reason.
- `the_round_robin_wraps_round_the_list_to_a_module_with_orders` used
  index 4 for a module with a decided order. Index 4 is module 6, which
  now runs its own cycle, so the example moved to index 5 (module 7),
  with the cursor expectations moved by one. The assertions are
  unchanged.

port:
- The first two moves (`FUN_004ddb70`, `FUN_004de130`) take their count
  from module 8's sector planners, which are not ported, so they move
  nothing.
- The raise and lowering of the side's garrison goals need the strategy
  module's controller `+4` bits `0x800000` and `0x400000`, which the port
  never sets.
- Controller `+0x14` and `+0x18` have no counterpart.

## Campaign (`playtest-module6.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`:
- The Empire moves regiments five times (day 1 one to Yaga Minor, day 4
  five to Drall, then Drall again from day 572). It trains 5 regiments
  from day 487, each sent on with a Destination.
- The Alliance moves regiments five times (Hoth on days 1 and 3, then
  Ghorman). It trains 10 regiments from day 3.
- Module 5 trains no special forces now. A temporary trace shows it
  keeping one or two sites, none with an idle training facility, while
  module 6, which runs first, trains regiments at the training facilities
  of the sites both mark.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3748 | 0.4 ms | 0.6 ms | 17.4 ms |
| Fast | 60 s | 3746 | 0.4 ms | 0.6 ms | 3.1 ms |

No frame took over 50 ms. The AI's longest day at Fast took 1 ms
(`console.txt`). The console has 32 `[ai]` lines and 0 errors.
`network.txt` lists 4 requests, all HTTP 200.

## Gates

- `cargo test --workspace --no-fail-fast`: 1981 passed, 0 failed.
  - 18 new tests in `ai_module6`, each citing its function.
  - `ai.rs`:
    - `a_refused_module_six_order_ends_its_cycle_and_the_tick`;
    - `a_regiment_move_needs_the_side_s_regiments_each_able_to_move`.
  - `troop_transport.rs`:
    `the_move_check_matches_the_move_and_changes_nothing`.
  - `blockade.rs`: `the_blockade_now_is_what_an_advance_would_store`.
  - Integrator:
    `module_six_s_regiment_move_sends_each_regiment_on_its_way`.
- Seed-42 golden regenerated for save v45 and module 6's orders:
  - initial `v1:ed967029485e168a` -> `v1:6d35df6768400d72`;
  - final `v1:6523821eb3c89a1a` -> `v1:8322498b801abe81`.

  The replay equivalence check (`replay-equivalence.json`) passes with 0
  browser errors. WASM SHA-256
  `a506fabf9f2343b63e8b0ba03bffad5402c44bb4b365bdf2f23c9bc1cf0d3729`.
- `cargo mutants --in-diff` (`ai_module6.rs` whole and the changed lines
  of `ai.rs`, `ai_module1.rs`, `blockade.rs`, `troop_transport.rs`):
  - 208 mutants: 191 caught, 10 missed, 7 unviable.
  - One survivor showed a redundant guard in the destination query (a
    system the query gives always has a record); the guard is gone.
  - Four tests were added for the other eight. A retest of the functions
    caught all 48 mutants.
  - The survivor left is `> -> >=` in `plan_regiments`. It is equivalent:
    at `m * count == w`, `w / m` is `count`.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in the changed files. rustfmt: `ai_module6.rs` is
  clean; no findings in the other changed lines.

Hashes are in `SHA256SUMS`.
