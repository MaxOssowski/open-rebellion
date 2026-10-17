# The maintenance shortfall is the side's autoscrap timer (F-037)

Date: 2026-10-17.

The economy ran a 30-day counter of its own, labelled `0x304`. Every 30
days it posted a "maintenance shortfall" for each side holding a system
short of regiments. In the original, `0x304` is the notification
`GameObjDestroyedAutoscrap`, and the Maintenance Shortfall message
reports an object destroyed because the side's maintenance exceeds its
capacity.

## The original

- `FUN_0052f670` sets the side's over-limit flag `+0xa8` (`FUN_0052f3d0`)
  to whether the maintenance requirement `+0x70` or use `+0x74` exceeds the
  capacity `+0x58`. The flag's hook (side slot `+0x1f8`, `FUN_005329f0`,
  disassembly `0x5329f0`) arms timer `0x382` with the flag on record
  `+0xcc`. Raising the flag starts the timer; lowering it cancels it.
- `FUN_00532350` sets record `+0xcc`'s minimum to GNPRTB 7168 (10)
  (disassembly `0x53236c`). While the side stays over its limit, the timer
  fires every 10 days.
- `FUN_00530350` checks the limit again. It walks types `0x10..0x40`
  (`FUN_00505340`) and counts what `FUN_0052e530` accepts: Existing
  (`+0x50` bit `0x40`), the side's, completed, with a class maintenance
  cost. It draws `FUN_0053e290(count - 1)` and sets that object's
  `AutoscrapRequest` (`FUN_004f7950`).
- The request's hook (slot `+0x140` -> `+0xe0`, `FUN_004f8850`) destroys an
  Existing object at once with reason `0x15` (slot `+0xac`). The side
  refunds half the class's refined cost (`FUN_00530270`). The destruction,
  `0x304`, posts the Maintenance Shortfall message: TEXTSTRA 28800 "Maintenance
  Shortfall", 28801 "The following units have been destroyed at {system}
  due to a maintenance shortfall:", 28802 the units.

## Port

- `autoscrap.rs`: `AutoscrapState` (each side's timer, save v64),
  `AutoscrapSystem::advance` and `apply`. It runs daily after the production
  cycle, in the headless simulation (step 13c) and in the app.
- The candidates are the objects `maintenance_totals` charges, in type
  order: regiments (on a planet, aboard or travelling), capital ships,
  squadrons (aboard fleets, then on planets), defense, manufacturing and
  production facilities, and special forces. Each needs a class
  maintenance cost.
- `apply` removes the object (a fleet left empty disbands; a regiment
  leaves its cargo or journey, `TroopTransportState::forget_transit`) and
  refunds half its refined cost.
- The app posts the Maintenance Shortfall message with the original's
  wording. Telemetry `maintenance_shortfall` now records one autoscrapped
  object.
- Removed: `EconomyEvent::MaintenanceShortfall`, the economy's two
  cooldown counters and their test, and the app's old message.
- port: within a type the walk follows the world's key order, with a
  fleet's squadrons before the planets'. The flag is read once a day.

## Tests

- `autoscrap.rs`:
  - `a_side_over_its_limit_loses_one_object_every_ten_days`;
  - `a_side_back_within_its_limit_cancels_the_timer`;
  - `the_pick_runs_over_the_side_s_maintained_objects_by_type`, with the
    other side's and upkeep-free objects of each type left out, and rolls
    of 0.5, 0.999999 and 1;
  - `an_autoscrap_removes_the_object_and_refunds_half_its_refined_cost`;
  - `an_autoscrapped_ship_or_carried_regiment_leaves_its_fleet`.
- `troop_transport.rs`: `a_regiment_destroyed_on_its_way_never_arrives`
  and `the_carrier_is_the_fleet_holding_the_regiment`.
- `integrator.rs`:
  `an_autoscrap_removes_the_object_and_records_a_maintenance_shortfall`.
- `simulation.rs`:
  `mines_in_a_system_in_revolt_give_no_capacity_and_start_the_autoscrap_timer`.
- Removed: `k4_maintenance_shortfall_fires_on_30_tick_cooldown`, the test of
  the invented counter.
- Neutralized, each failing its tests:
  - firing at once;
  - keeping the timer when back within the limit;
  - counting upkeep-free objects;
  - no refund;
  - the integrator not applying the scrap;
  - the simulation's blocked test with `&&`.

## Campaigns (`playtest-autoscrap.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `0d041834`:
- The games are unchanged in all seven seeds: builds, control changes,
  uprisings and battles are equal.
- The old counter's 12 to 20 "maintenance shortfall" records per run are
  gone.
- No side stays over its limit for 10 days, so no object is autoscrapped.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`24f600f94bff74dcf05eccd1fcfa4adc9efee7175687692bed3e6d56e262d800`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.8 ms | 22.8 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 1.9 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2269 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the save layout and the
  autoscrap rolls): initial `v1:156fbf23b95e317e` ->
  `v1:3ce41483f255839d`, final `v1:99cb09fad4a57148` ->
  `v1:c23f38a8242bca8f`. Two recordings are identical, and only the
  fingerprints changed in the manifest. The replay equivalence check
  passes with 0 browser errors. Release WASM SHA-256
  `86106a44d986684c506aa45deab1d08ecbc1b9741482b91443ea93829ec616fd`.
- `cargo mutants --in-diff`: the first run missed 21 of 78, all in the
  candidate filters, `forget_transit`, `carrier`, the ship guard, the
  roll's bound and the simulation's blocked test. The tests above were
  added for them, and the second run missed 1, the simulation's `||`, now
  caught by the simulation test (tried).
- `provenance-scan check`: 89 uncited items, one fewer.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
