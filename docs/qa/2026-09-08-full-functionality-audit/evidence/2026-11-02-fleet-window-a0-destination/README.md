# The Fleet window at a fleet's destination against A0 (2026-11-02)

## A0 (VM, the `test` Alliance save, days 23..97)

Captures under `.artifacts/a0-vm/fleet4/` (hashes in `a0-sha256.txt`):

| Capture | What it shows |
|---|---|
| `01` | Fleet 6, ordered from Yavin to Qat Chrystac. It leaves Yavin's window and its marker shows at Qat Chrystac. |
| `02`, `03`, `qat-fleet8-{tab}` | Qat Chrystac's Fleet window, opened at (305, 86). It lists the travelling fleet (A0 had renamed it Fleet 8) at its destination. The fleet's entry, picture and ships carry the engine glow, and the tabs show its two regiments. |
| `04` | Fleet 8 later bound for Bortras. Its Fleet Status reads "Status: Enroute" and "ETA Destination: Day 119"; the port already writes that row (`status_rows.rs`, `FUN_004fd2b0`). |

## The port's copy

`FleetWindowDestination` (code 86) reuses `place_yavin`'s fleet:

- It renames the fleet "Fleet 8" and orders it to Qat Chrystac.
- It opens Qat Chrystac's window at A0's place with the fleet selected.

The `fleet-window.mjs` case `alliance/a0-destination` compares each tab
with A0's capture. No rendering code changed; the port already matched.

| Tab | Differing pixels | Ceiling |
|---|---|---|
| Ships | 1967 | 2150 |
| Fighters | 1000 | 1150 |
| Troops | 2367 | 2550 |
| Personnel | 902 | 1050 |

The remaining differences are text hinting, which is an accepted
deviation.

## Tests and run

- Fixture test: `the_a0_destination_scenario_lists_fleet_8_en_route_at_qat_chrystac`.
- Fleet window gate run
  `fleet-window-2026-11-02T03-11-45-948Z-3118704` (Arch Chromium 155):
  80 cases pass. Test WASM
  `f5abf879f3fb6aa2af3901e5c08fbee1cbf6da49681ef1c429e6a17a310e85f0`.
- Workspace: 3313 tests pass. Fixtures: 110 pass. All other gates pass.
