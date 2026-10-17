# The automatic landing retired (F-006, F-007E, F-036)

Date: 2026-10-17.

The port landed a fleet's carried regiments by itself whenever the fleet
stood over its side's planet. The unloading trace finds no such path in
the original: "Nothing in the original lands cargo without an order"
(`ghidra/notes/regiment-unload.md`, "What happens to the rest of the
cargo"). The roadmap had "Source or retire the port's automatic landing".
The user chose to retire it.

## Change

- `rebellion-data/src/simulation.rs`: step 3b only occupies. A planet with
  one side's regiments alone passes to that side (port). Carried
  regiments stay aboard.
- `rebellion-data/src/integrator.rs`: `apply_troop_landing` is removed.
- `rebellion-app/src/tactical_flow.rs`: `land_and_occupy` is now `occupy`,
  without the landing.
- `rebellion-core/src/ai.rs`: module `0xc`'s heuristic (port) used to send a
  regiment aboard a transport fleet and count on it landing on arrival.
  Now each weak system gets one donor regiment by the regiment's own Move
  (`AIAction::MoveRegiments`, Move `0x201`, as module 6 gives). Its loop
  over earlier fleet orders, which only the removed fleet heuristics
  filled, is gone, and so is the `movement` input no module reads now.
- The fleet's hold (`TroopTransportState::load`, `release`) only kept the
  landing off a fleet that loaded by order. It no longer changes anything,
  and it stays in the save.
- The notes are updated: `regiment-unload.md`, `planetary-assault.md`,
  `fleet-join-split.md`, `agent_docs/simulation.md`, `architecture.md`,
  `systems/combat.md`; the roadmap item is checked.

Save format: unchanged at v58.

## Tests that changed

- `simulation.rs`:
  - `a_regiment_loaded_by_order_stays_aboard_at_the_system_it_loaded_at`:
    its second case expected a regiment embarked for a departure to land.
    It now stays aboard;
  - `carried_regiments_land_only_on_their_side_s_standing_unblockaded_planet`
    is now `carried_regiments_stay_aboard_over_any_planet_without_an_order`,
    and the side's own planet joins the other cases.
- `tactical_flow.rs`:
  - `a_transport_lands_only_on_its_side_s_planet_and_a_held_fleet_keeps_its_cargo`
    is now `carried_regiments_stay_aboard_without_an_order`;
  - in `one_side_s_regiments_alone_take_the_planet_and_two_sides_stand_together`
    the two-sides case lands the Alliance's regiment by order first.
- `ai.rs`: `troop_deployment_prioritizes_frontline_systems` now expects the
  regiment's own Move where it expected a transport fleet's Move.
- `fleet-window.mjs`: `move-and-land` is now `move-and-carry`. The fleet
  arrives, its hold ends, and the regiment stays aboard, still 30 frames
  later. The fixture's report gains the fleet's system
  (`fleet_system_dat_id`).

## Campaigns (`playtest-landing.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, compared with
`522555db`:
- Seed 47 is identical. Seeds 1, 2, 3, 7 and 13 differ from day 1 or 2,
  when module `0xc`'s first regiment move goes, and seed 42 from day 105
  (`playtest-first-difference.txt`).
- Planets changing hands per seed (old to new): 1 8→10, 2 9→11, 3 11→19,
  7 10→13, 13 11→11, 42 6→6, 47 8→8.
- Most of the increase is one clash going back and forth. A regiment
  arrives at a system that turned neutral. The step 3b occupation (port)
  gives the system to its side, and the next day the economy's
  troop-based control (`FUN_0050b610`) makes it neutral again. In seed 3,
  Yavin goes this way three times and Xyquine twice. This clash was
  already open; it shows more now that regiments arrive on their own.
- `telemetry_coverage`, which failed for want of `repair` events, passes
  now (39 events).

## Browser

The Fleet window gate passes all 48 cases, both sides
(`fleet-window-carry.json`, interface WASM SHA-256
`2a40511093cb0e504add6daa2ae81d33720ddd11fcb72a01e2846339f742cdbf`).
`*-move-and-carry.png` shows the end of the new case.

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast. Each
frame's work time was measured:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3600 | 0.5 ms | 0.7 ms | 22.2 ms |
| Fast | 60 s | 3601 | 0.5 ms | 1.4 ms | 3.3 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2246 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Neutralized:
  - Putting the landing back into step 3b fails both simulation tests.
  - The app's `occupy` has no transport state, so it cannot land. The
    browser case `move-and-carry` covers the app's path.
- Seed-42 golden regenerated for the retired landing. The initial
  fingerprint `v1:51f96ff9faae705b` is unchanged; from the checkpoint after
  3 commands the fingerprints change, and the final goes
  `v1:d2e439b6e18aff1f` -> `v1:3bb9a6063ff07d84`. Two recordings are
  identical, and only the fingerprints changed in the manifest. The replay
  equivalence check passes with 0 browser errors. Release WASM SHA-256
  `ab286f56406c31d783708d19ec56b712e808a14dd2b1e0f5643a19e4ba471c38`.
- `cargo mutants --in-diff` (core, data, app): 13 mutants, 9 caught, 3
  unviable, 1 missed. The miss is `main` emptied in the app, which has no
  headless test.
- `provenance-scan check`: 100 uncited items, within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
