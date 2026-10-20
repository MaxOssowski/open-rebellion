# Browser pass of Destroy System, both sides (F-017)

Date: 2026-10-20. F-017's last open clause, on the user's go-ahead.

## What the original does

- The Fleet menu's Planetary Bombardment lists Destroy System (`0x223`).
- `FUN_00582440` enables it only for a fleet holding a Death Star. A
  fleet without one is refused with `8`/`8`.
- `FUN_0058d990` destroys the system (`FUN_005080d0`). Its contents follow
  `FUN_004fb520`:
  - units and facilities are destroyed;
  - the fleet stays (`FUN_00524fb0`);
  - a character on no mission goes adrift (`FUN_004ef2e0`).
- The notes are in `bombardment-order.md`, "The destroyed system's
  contents".

## The gate

- A new fixture scenario, `FleetDestroySystem` (73, code 74), is
  `FleetBombardment` with two changes:
  - the fleet's ship is the side's Death Star (CAPSHPSD 136), which only
    the Empire has;
  - one of the other side's free characters stands on the planet.
- The fixture now reports:
  - in the submenu, `destroy_row` and `destroy_enabled`
    (`ObjectMenuState::submenu_row_enabled`);
  - in the observation, `primary_destroyed`, `fleet_present`,
    `adrift_characters` and `last_destroy_message`.
- A new `destroy-system` case in `tools/interface-parity/fleet-window.mjs`
  runs it for both sides.

| Side | Destroy System | After the click |
|---|---|---|
| Alliance | row 3, disabled | nothing destroyed, the fleet present (Escape closes the menu) |
| Empire | row 3, enabled | Akrit'tar destroyed; the fleet present; 2 characters adrift; 0 enemy regiments; "Akrit'tar DESTROYED by Death Star superlaser!" |

- Both runs had 0 page errors and 4 requests, all HTTP 200.
- Linux Chromium 153 ran it, through the documented temporary manifest
  copy.
- `destroy-system-browser.json` holds each run's checks.
- The screenshots stay in the run directory (they show original game art).
  Their SHA-256 sums are in `screenshots.sha256`.

## The full gates (`f11`)

- The Fleet window suite passes all 50 cases (48 + the two new ones).
- The Status window gate passes.
- The replay equivalence check passes with 0 browser errors (seed-42
  `v1:82332ef108ec7e21` -> `v1:49a3738dca7ca496`, unchanged; release WASM
  SHA-256
  `44e4c862396f42fda0f2c0b30efa59f0cb4bf0b9f41d076f43d5e3c6014b16ce`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.7 ms | 23.9 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.7 ms |

- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors; 4 requests, all HTTP 200.
- 24 dual-AI campaigns are identical to the stage before.

## Tests

- `the_destroy_system_scenario_gives_only_the_empire_s_fleet_a_death_star`
  (fixture).
- `a_fleets_bombardment_submenu_lists_the_four_targets_by_their_gates`
  now also checks `submenu_row_enabled`. The gate run had found its two
  mutants alive; a rerun of `cargo mutants --in-diff` catches both.
- `cargo test --workspace`: 2488 passed, 0 failed. The app's fixture
  tests: 81 passed.
- `provenance-scan check`: 53 uncited items, unchanged.

## Sector window

The sector window still draws the destroyed planet as before. The look of
a destroyed system is not part of this gate.

Hashes of the committed files are in `SHA256SUMS`.
