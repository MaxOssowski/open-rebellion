# The AI's module 7: squadrons on planets and shipyards (F-036)

Date: 2026-10-15.

Module 7 (`FUN_004d2ef0`, step `FUN_004d3100`) balances the side's fighter
squadrons on planets in three cycles that run in turn:
- **Surplus** (`FUN_004d3170`): a system with more active squadrons than
  its goal (`v[53] > 0`) sends its weakest squadron to the side's fleet
  with hangar room nearest it, else to the nearest system short of
  squadrons, as Move `0x201`; with nowhere to go it scraps one (`0x200`).
- **Squadrons** (`FUN_004d3220`): keep a shipyard site (mark `0x200000`,
  an idle shipyard), choose the system short of squadrons (`v[38] < 0`)
  nearest it, and build what 80% of 17% of the capacity pays for, less the
  squadrons' upkeep on planets, at most the site's shipyards: Destination,
  then Build `0x211`. With no short system it raises the side's squadron
  goals (`+0x388..+0x390`); over 90% of the budget it moves a squadron to
  a fleet or scraps it (`FUN_004d4800`).
- **Shipyards** (`FUN_004d3350`): module 6's yard cycle for shipyards,
  with the token 8 and module 2's borrow.

It is read in full in `ghidra/notes/ai-production-modules.md`, "Module 7",
with the queries' arguments taken from the assembly and the shared
functions compared instruction by instruction.

## Port (save v47)

- `crates/rebellion-core/src/ai_module7.rs`: `Module7`, fields
  `+0x38..+0x74`, the step and the three cycles.
- Squadron travel: a Move to another system now sends the squadrons on
  their own at their FIGHTSD speed (`squadrons::SquadronTransits`, kept in
  `TroopTransportState`); they enter their planet or fleet on arrival, and
  are lost when the planet is destroyed or the fleet is gone. A fleet's room
  counts the squadrons on their way to it. The player's squadron Move uses
  the same path, and an arrival posts "Squadron arrived at ...".
- `ScrapTarget::PlanetFighter`: a squadron on a planet takes Scrap
  (`FUN_005575f0`), refunding half its refined cost.
- `Controller::squadron_goals`: side `+0x388`, `+0x38c`, `+0x390`, which
  the system records now read (`SystemInputs::squadron_goals`).
- `Tallies::squadron_upkeep`: controller `+0x1e0` (the sector records'
  `+0x104`, own systems' `v[32]`).
- `YardKind::SHIPYARD`; `scrap_at` now spares two families per kind
  (`FUN_004d5510` spares training facilities and construction yards, the
  others shipyards and their other family).
- `Galaxy::sector_position_of`, shared with `sectors_by_distance`.
- The heuristic's squadron loading (stage 1) is removed: the surplus cycle
  sends planet squadrons onto fleets. Its three tests are removed with it.
- `the_round_robin_wraps_round_the_list_to_a_module_with_orders` used
  index 5 for a module with a decided order. Index 5 is module 7, which
  now runs its own cycle, so the example moved to index 6 (module 4), with
  the cursor expectations moved by one. The assertions are unchanged.

port:
- No code sets fleet record bit 8, so the first fleet query (`0x100009`)
  finds none and the second (`0x100005`) decides.
- Module 8's sector planners (`FUN_00419e90`) are not ported: a squadron
  no fleet takes is scrapped.
- The fallbacks controller `+0x20` and `+0x24` have no counterpart.
- Every squadron is at full strength, so the weakest is the smallest
  class.
- The surplus loops take the weakest squadron again on each pass without
  leaving out the one taken (no `FUN_004f5940` check), so each order moves
  or scraps one squadron.

## Campaign (`playtest-module7.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`:
- The Empire builds a shipyard at Coruscant on day 43 and the Alliance one
  at Hoth on day 62 (module 7's yard cycle, `0x28000001`).
- The Empire builds one squadron at Yaga Minor on day 91, sent to
  Coruscant; the Alliance one at Ghorman on day 160.
- That is all in 600 days. A temporary trace shows why: the squadron cycle
  keeps a site only while its shipyard is idle (`+0x28` bit `0x200`), and
  the production heuristic that stands in for module `0xb` keeps the
  shipyard queues busy with up to three ships and fighters. Of 4,547
  squadron cycles, 2 kept a site. No surplus arose, so no squadron moved
  or was scrapped.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3746 | 0.5 ms | 0.6 ms | 2.1 ms |
| Fast | 60 s | 3745 | 0.4 ms | 0.7 ms | 3.2 ms |

No frame took over 50 ms. The AI's longest day at Fast took 1 ms
(`console.txt`). The console has 32 `[ai]` lines and 0 errors.
`network.txt` lists 4 requests, all HTTP 200.

The fleet-window interface family (44 cases, the squadron moves and the
squadron battle among them) passes on the new squadron move path, run on
the system Chromium 153 as in stage 1 (the pinned macOS build is not on
this machine).

Native GUI acceptance was not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2042 passed, 0 failed.
  - 22 tests in `ai_module7`, each citing its function.
  - `ai.rs`: `a_refused_module_seven_order_ends_its_cycle_and_the_tick`.
  - `squadrons.rs`: travel to a planet and to a fleet, the exact arrival
    day, a lost squadron.
  - `scrap.rs`: `a_squadron_on_a_planet_is_scrapped_one_at_a_time`.
- Seed-42 golden regenerated for save v47 and module 7's orders:
  - initial `v1:d33088556c4445a3` -> `v1:da11bf56815cc333`;
  - final `v1:9ff9c886bcd2e454` -> `v1:cf8eb895d3fad9e4`.

  The replay equivalence check (`replay-equivalence.json`) passes with 0
  browser errors. WASM SHA-256
  `9e6b5f9f160a889f478ce2a48e6ebb5e786257476ecaa760ce8bfcc6c34c37d5`.
- `cargo mutants --in-diff` (`ai_module7.rs` whole and the changed lines
  of `squadrons.rs`, `scrap.rs`, `ai.rs`, `ai_module1.rs`, `ai_records.rs`,
  `simulation.rs`, `integrator.rs`):
  - 272 mutants: 215 caught, 37 missed, 20 unviable.
  - Tests were added for the gaps (thresholds, an emptied entry, ties,
    another side's or a full fleet, a second sector, the budgets' edges,
    the yard state 9); the fleet query bits became plain constants, a
    serde default with no reader went, and the sector lookup moved into
    `Galaxy::sector_position_of`. Retests of those functions caught all
    but one.
  - The survivor left is `> -> >=` in `plan_squadrons`. It is equivalent:
    at `m * count == w`, `w / m` is `count`.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in the changed lines. rustfmt: `ai_module7.rs` and
  `squadrons.rs` are clean; no findings in the other changed lines.

Hashes are in `SHA256SUMS`.
