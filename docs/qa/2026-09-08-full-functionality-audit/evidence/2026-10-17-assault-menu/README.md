# The player's Planetary Assault (F-006)

Date: 2026-10-17.

Stage 5f ported Planetary Assault (`0x234`) for the AI. The player's menu
item stayed disabled. This stage enables it. The reading is in
`ghidra/notes/planetary-assault.md` ("Ported (2026-10-17): the player's
order") and `ghidra/notes/object-popup-menu.md`.

## Port

- `rebellion-render/src/object_menu.rs`:
  - `OrderGates::assault` enables the Planetary Assault row (STRATEGY
    sort 220, TEXTSTRA 12318) on a fleet and on a sector window's fleet
    icon. These are the only selections whose class lists it
    (`FUN_004ff8e0`, `FUN_00507290`);
  - `ObjectMenuState::row_enabled` reports a row's state.
- `rebellion-render/src/panels/mod.rs`: `PanelAction::OrderAssault`.
- `rebellion-app/src/main.rs`:
  - the gate is `rebellion_core::assault::order_enabled` (the order's
    `+0x18`, `FUN_00581ae0`) for the fleet, or for the fleet icon's
    system's fleets of the player's side (`FUN_00512700`, kind `0x10`);
  - a click orders the assault at once. `FUN_00487cc0` asks no
    confirmation for `0x234`, which falls to its default;
  - the app checks the gate again, then carries the order out with the
    AI's `BombardEnv::assault` and writes the same summary line.
    Otherwise it writes "Planetary assault refused" (`FUN_00487c90`).
- `rebellion-app/src/interface_test_fixture.rs`: scenario `FleetAssault`
  (67, browser code 68), which is `FleetBombardment` with one of the
  player's regiments aboard the fleet.
- `tools/interface-parity/fleet-window.mjs`: the `assault` case.

port:
- The Assault Summary window is not ported. The message log carries the
  summary.

Save format: unchanged at v57. `rebellion-core` is not touched, so the
simulation and the AI are unchanged.

## Browser

The Fleet window gate (`fleet-window.mjs`) ran under Arch Chromium 153
through a temporary Linux manifest; the pinned macOS build is not
installed here. All 48 cases pass, both sides, including the new
`assault` case (`fleet-window-assault.json`). The interface WASM's
SHA-256 is
`c3ac76fac21ab39ce81053780c80fa334b4f80c0050f63e6823a4fc8896bf040`.
- `*-assault-menu.png`: the fleet icon's menu with Planetary Assault
  enabled.
- `*-assault-done.png`: after the click. The report's last line is
  "Planetary assault at Akrit'tar: 0 of 1 attacking and 1 of 2 defending
  regiment(s) lost, 0 landed". One defender is left, so nothing lands.

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast. Each
frame's work time was measured:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3600 | 0.5 ms | 0.7 ms | 24.7 ms |
| Fast | 60 s | 3601 | 0.5 ms | 1.5 ms | 3.4 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2258 passed, 0 failed.
  - `object_menu.rs`: the row and its gate on a fleet and a fleet icon,
    and no other selection offering it; `row_enabled`.
  - `interface_test_fixture.rs`: the scenario carries one of the player's
    regiments and enables the order. The bombardment scenario carries
    none and does not. The report keeps the last assault line.
- Neutralized changes: 7, and all fail.
  - Four fail a unit test: the row's gate, `row_enabled`, the scenario's
    regiment, and the report's line.
  - Three fail the browser case after a rebuild: the app's gate, the
    menu's dispatch, and the order's execution.
- `cargo mutants --in-diff` (render and app): 21 mutants, 13 caught, 4
  unviable, 4 missed. The missed ones are in the app loop, which has no
  headless test: `main` and `apply_panel_action` emptied, and the side test
  (`==` to `!=`) in each. The browser case covers these paths, as the
  neutralizations above show.
- The replay equivalence check passes with 0 browser errors. Golden
  `v1:a8343f6c6630924e`, final `v1:e523a780152b284e`, both unchanged.
  Release WASM SHA-256
  `cb28a3bf7cbac16f8471daf5a92524fb94a2add9cefa7be1b105d1ac8a6fc031`.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
