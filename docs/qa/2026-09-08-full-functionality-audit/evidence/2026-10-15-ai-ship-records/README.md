# The AI's ship records and Create Fleet: module 8, stage 3 (F-036)

Date: 2026-10-15.

The infrastructure the sector planners' plan objects need. The reading is
in `ghidra/notes/ai-sector-plans.md` ("The planner's ships and the
reports it gets"): a sector planner holds capital ships, marking their
records (`FUN_00478e00`, `FUN_004790d0`), splits ships off fleets with
Create Fleet `0x270`, and gets no order reports (Mission Reports reach
mission planners only, `FUN_00485f60` -> `FUN_0042f7f0`).

## Port (save v51)

- `ShipInstance::id` and `GameWorld::number_ships` (port): a capital ship
  gets an id, numbered fleet by fleet before the AI's passes in both
  drivers (`simulation.rs`, the app), at a new world and so at a load.
  The id goes with the ship between fleets. Chosen with the user over a
  field on the ship, which would have had the AI write to the world.
- `AIState::ship_holds`: what the AI keeps on a ship record
  (`+0x24 & 0x1b020000`, `+0x30`, `+0x34`; `FUN_00484230` constructs
  with `0x20000`, `FUN_00484630` keeps `0x1b0204ff`), applied at each
  refresh (`apply_ship_holds`); holds of ships gone are dropped.
- `FleetRecord::ships`: the fleet's ship records (`ShipRecord`: place,
  id, `+0x24` with `0x4000` for the side's ship and `0x20000`, holder,
  plan, values).
- `AIAction::CreateFleet`: Create Fleet `0x270` through
  `fleet_join::create_fleet`, validated by the new
  `fleet_join::check_create_fleet`, carried out by the integrator and
  the app.
- `Galaxy::fleets_near` / `fleets_in`: `FUN_0041a600`, module 7's query
  made shared, its results unchanged.

port: no module issues Create Fleet or takes a ship yet; the plan
objects come next.

## Campaign (`playtest-ship-records.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`: every
event count equals stage 2's (`identical: true`), as this stage changes
no decision.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3742 | 0.5 ms | 0.7 ms | 2.3 ms |
| Fast | 60 s | 3745 | 0.4 ms | 0.6 ms | 3.2 ms |

No frame took over 50 ms. The AI's longest day took 18 ms (day 1 at Very
Slow, 600 passes; `console.txt`). The console has 32 `[ai]` lines and 0
errors; `network.txt` lists 4 requests, all HTTP 200. Native GUI
acceptance was not run; the app's numbering and Create Fleet path have no
test of their own.

## Gates

- `cargo test --workspace --no-fail-fast`: 2115 passed, 0 failed. New
  tests: ship numbering (world, the simulation driver), a ship keeping
  its id through Create Fleet, the Create Fleet check, the fleet record's
  ship records, the kept ship record fields, the fleets near a sector,
  the AI's Create Fleet validation, the holds following the ships, the
  integrator's Create Fleet. Each fails with its change neutralized.
- Seed-42 golden regenerated for save v51 (the ship ids):
  initial `v1:6cc71d2663560368` -> `v1:1ec77e5da40c2f8f`, final
  `v1:951c79bcafd01fcf` -> `v1:73074bf4b25f5bf9`. The replay equivalence
  check (`replay-equivalence.json`) passes with 0 browser errors. WASM
  SHA-256 `f1c20cfc26417b9e346804478467195f9b43ad5ee5961229ef07bfb2f570c669`.
- `cargo mutants --in-diff`: core 33 mutants, 29 caught, 1 unviable, 3
  missed; data 7 mutants, 5 caught, 1 unviable, 1 missed. The core three
  are `|` -> `^` on disjoint bits (`apply_ship_holds`, the ship record's
  `0x4000 | 0x20000`), equivalent. The data one replaces
  `load_game_data_with_options` with an empty world; the library tests
  load no original data, and the data-backed replay test
  (`replay_manifest -- --ignored`), which checks the numbered initial
  state's fingerprint, catches it.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
