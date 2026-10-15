# The AI's module `0xb`: capital ships (F-036)

Date: 2026-10-15.

Module `0xb` (`FUN_004c8820`, step `FUN_004c8b30`) keeps the side's
capital ships in three cycles that run in turn:
- **Fleets** (`FUN_004c8ba0`): damaged ships that can move go to the
  shipyard site nearest their fleet, else the nearest own system
  (`FUN_004c8e90`, Move `0x201`); a fleet with the Death Star goes to
  Coruscant while it is the side's (`FUN_004c94c0`).
- **Ships** (`FUN_004c8c20`): keep a shipyard site, choose a role
  (`FUN_004ca5e0`: troop carriers while the fleets' troop room is below
  the most regiments a system needs, the Death Star for the Empire with
  two sites, armed ships, ships with lasers, carriers, else none), and
  build one ship of the role's best class within 90% of 30% of the
  capacity less the ships' upkeep (`FUN_004ca910`, Build `0x211`); with no
  role any shipyard class qualifies, squadrons too. Over budget the
  cheapest ship of the cheapest fleet is scrapped (`FUN_004cab20`).
- **Shipyards** (`FUN_004c8d20`): module 7's yard cycle with the token 2
  and module 2's borrow, on 10% of 30% of the capacity.

It is read in full in `ghidra/notes/ai-production-modules.md`, "Module
`0xb`", with the queries' arguments taken from the assembly and the shared
functions compared instruction by instruction.

## Port (save v48)

- `crates/rebellion-core/src/ai_module_b.rs`: `ModuleB`, fields
  `+0x28`, `+0x38..+0x80`, the step and the three cycles.
- Ship records take Damaged (`ai_records::ship_damaged`, hull below the
  class's): `0x1000` with a speed, else `0x800000` (`FUN_00484630`,
  `FUN_00501c00`, `FUN_00500820`). A damaged ship keeps no bombardment
  figure and its fleet no longer counts it armed. The seed-42 fixtures
  that built ships below their class hull now build them at full hull.
- `AIAction::MoveShips`: the Move of some or all of a fleet's ships,
  carried out by `fleet_join::move_ships_to_system`; validated as the
  whole fleet's dispatch, so a move to the fleet's own system is refused.
- `ScrapTarget::Ship`: a capital ship takes Scrap (`FUN_005575f0`), half
  its refined cost back; a fleet left with nothing disbands; regiments
  beyond the room left are lost.
- `Tallies`: `ship_upkeep` `+0x24c`, `troop_room` `+0x260`, `armed`
  `+0x268`, `lasers` `+0x26c`, `hangars` `+0x27c` (`FUN_0041ba80`),
  `death_star` (`+4` bit 4), `troops_wanted` `+0x20c` (`FUN_0041b7e0`).
- Shared: `ai_module2::borrow_yard` (modules 2, 7, `0xb`),
  `ai_module3::tenth_less_yards` (modules 3, `0xb`).
- The production heuristic no longer builds ships or fighters.

port:
- `FUN_004c94c0`'s move to a base is left out: the bases (`+0x30`
  `0x40000`) are set by the fleet plan objects (`FUN_004b24b0`,
  `FUN_004b5b20`, `FUN_004b8070`), not ported, and without a base the
  fallback would send every idle fleet to another sector at each step.
- `FUN_004c9cb0` and the role requests need module 8's sector planners:
  no order, and no bombardment or interdictor role.
- A fleet of squadrons alone is passed over by the fleet queries.
- hyp: controller `+0x16c` has no writer found and reads as 0.

## Campaign (`playtest-module-b.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`:
- The Empire builds 7 troop carriers at Xyquine from day 148 (its fleets'
  troop room 4 below the 17 a system needs), then on day 408 a ship of
  another class; the Alliance 2 troop carriers at Vagran (days 107, 203)
  and scraps one ship over budget on day 232.
- With the roles met, the Empire's role is none, and its shipyard at
  Xyquine builds any shipyard class, squadrons mostly: 44 squadrons from
  module `0xb`, besides module 7's 35 at Yaga Minor. Module 7 then moves
  the surplus (48 moves) and scraps 29 squadrons. This follows the read:
  in the original module 8's planners would name roles.
- Freed of the heuristic's queues, module 7's shipyards are idle now and
  build squadrons every 12 days.
- No ship was damaged at an idle moment, so no repair Move was given.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3747 | 0.5 ms | 0.7 ms | 19.8 ms |
| Fast | 60 s | 3747 | 0.4 ms | 0.6 ms | 1.7 ms |

No frame took over 50 ms. The AI's longest day at Fast took 1 ms
(`console.txt`). The console has 32 `[ai]` lines and 0 errors.
`network.txt` lists 4 requests, all HTTP 200.

Native GUI acceptance was not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2064 passed, 0 failed.
  - 15 tests in `ai_module_b`, each citing its function; a damaged ship's
    record, the controller's fleet sums, a ship's Scrap, the ship Move's
    validation, the integrator's ship Move and Scrap, module `0xb`'s
    refusal and module `0xa`'s heuristic orders.
  - Each fails with its change neutralized.
- Seed-42 golden regenerated for save v48 and module `0xb`'s orders:
  initial `v1:da11bf56815cc333` -> `v1:bde18d3a1df8ed02`; final
  `v1:cf8eb895d3fad9e4` -> `v1:e407af1c444ac566`. The replay
  equivalence check (`replay-equivalence.json`) passes with 0 browser
  errors. WASM SHA-256
  `63aa98100fccfc170ce6fd4b15977813c4d714cef8dbd1e57004a86a87e58fe8`.
- `cargo mutants --in-diff` (`ai_module_b.rs` whole, the changed lines of
  `ai.rs`, `ai_module1.rs`, `ai_module2.rs`, `ai_module3.rs`,
  `ai_module7.rs`, `ai_records.rs`, `scrap.rs`, `integrator.rs`): 241
  mutants, 213 caught, 20 missed, 8 unviable. Tests were added for the
  gaps; a retest of those functions caught all 85 viable mutants. The one
  left is `> 0 -> >= 0` on the damaged ship's speed in `ship_record`; it is
  equivalent, as `capital_ship_speed` never gives 0.
- `provenance-scan check`: 103 uncited items, within the baseline.
- Clippy: no findings in the changed lines. rustfmt: `ai_module_b.rs` is
  clean; no findings in the other changed lines.

Hashes are in `SHA256SUMS`.
