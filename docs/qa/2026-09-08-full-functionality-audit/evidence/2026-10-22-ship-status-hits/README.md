# The Capital Ship Status rows after hits (F-032, D5)

Date: 2026-10-22. The reading is in `ghidra/notes/status-window.md`
("Capital Ship Status", Port) and `ghidra/notes/ship-subsystem-damage.md`.

## Change

- `rebellion-core/src/ship_condition.rs` (new): the ship object's damage
  terms over `ShipInstance::subsystem_hits` and the hull, with
  `DAT_006bb388` = GNPRTB 2048:
  - `recharge`: `rate - rate x hits / G` (`FUN_00500f60`, `FUN_00500fa0`);
  - `rating`: `value x (G - hits) / G x (hull - damage) / hull`
    (`FUN_00500e60` tractor, `FUN_005010c0` sub-light, `FUN_00501270`
    the arcs);
  - `bombardment`: `modifier - modifier x damage / hull` (`FUN_00500f00`);
  - `hyperdrive`: the primary less its hits, else the backup
    (`FUN_005011f0`, `FUN_00501230`).
- `status_rows.rs` (`FUN_00446fd0`): the Hyperdrive, Shield Recharge,
  Tractor Beam, Sub-Light, Weapon Recharge, Bombardment and arc rows read
  those terms. Ship Damaged (and the fleet's Damaged Ships) counts hit
  counters too (`FUN_00501c00`).

## Gates

- Tests: 5 new (core 4, render 1). Without the change the render test
  fails ("8:8" for "6:8"). Workspace 2604 passed, 0 failed.
- Mutants on the diff: 58 caught, 0 missed, 1 unviable.
- `provenance-scan` 0 uncited.
- Browser, Arch Chromium 155.0.8059.84: the Status window gate passes for
  both sides, 10 of 10 (`status-window.json`). Its fixture ship is
  undamaged, so the pixels are unchanged; the damaged values are covered
  by the unit tests.
- Replay equivalence passed (`v1:c117ed614a9dd901`).
- Release WASM `2426ed659e640742ea70ff6d6054d0a74a4f9d9dfe946d1ba957033624a75b7f`.
- The simulation does not read the new module, so campaigns are
  unchanged.
