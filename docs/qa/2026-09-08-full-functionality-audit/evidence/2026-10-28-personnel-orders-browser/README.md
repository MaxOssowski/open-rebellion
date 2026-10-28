# Personnel orders: a browser gate for Move, Retire and Abort (P08)

Date: 2026-10-28. Tracker item "Okno Personel": the last open point was a
browser test for Retire, Move and Abort.

## The gate

`tools/interface-parity/personnel-orders.mjs` drives each order through
its original entry, on both sides. It reads each result from the app's
`[interface]` console lines. It compares no artwork.

| Case | Fixture | Entry | Result |
|---|---|---|---|
| `character-move` | 70 `CharacterMove` | agent menu, Move, release on F'tral | `result=Ok(0)`, the Move mission `0x241` |
| `character-move-refused` | 69 `CharacterCommand` | the same, F'tral the enemy's | `Err("1/0x28")` (`FUN_004f4b60`) |
| `special-force-move-refused` | 63 `ReconTargeting` | unit menu, Move, release on F'tral | `Err("1/0x28")` |
| `character-retire` | 70 | agent menu, Retire, checkmark | `0x242` validates, then `Ok(())` (`FUN_004ed460`, `FUN_00534c00`) |
| `special-force-retire` | 63 | unit menu, Retire, checkmark | `Ok(())` (`FUN_00533ba0`) |
| `abort` | 50 `Quadrants` | mission icon menu, Abort, checkmark | see below |

The `abort` case:
- On day 0 the icon's Abort is disabled. The missions are only ordered,
  and `FUN_00521600` takes only a running mission (phase 8).
- The case runs the clock at Medium through the Game Speed menu
  (`FUN_0042d190`) for about two days, then pauses it.
- Abort is then enabled. The confirmation asks "Are you sure you want to
  abort this mission?".
- The checkmark ends the player's mission: `mission=0 result=true` (end
  code 2, `FUN_005233a0`).

## Fixture changes (`interface_test_fixture.rs`)

- The object-menu report gains `retire_row`/`retire_enabled` and
  `abort_row`/`abort_enabled`.
- The `Quadrants` scenario now reports its object menu, which has no
  target planet, and its speed menu.
- Its setup reports the day readout's centre (`day_readout_centre`, shared
  with the fleet-load setup).

## A defect the gate found

`macroquad::logging::info!` with only a format literal goes through
miniquad's `__private_api_log_lit`, which prints the text unformatted.
So the Retire and Abort lines logged `member={member:?} result={result:?}`
and `missions={missions:?}` literally (`fail-without.json`: 6 of 12
failed, every Move case passed).

The five such lines in `main.rs` now pass their arguments explicitly:
- Retire;
- the two Abort lines;
- the special force's Confirmed Move;
- the encyclopedia catalog error.

The orders themselves were unaffected.

## Acceptance (`acceptance.json`)

- 12 of 12 cases passed, both sides.
- Run `personnel-orders-2026-10-28T20-37-24-103Z-217333`.
- Interface WASM SHA-256 `c06bca154635134e704869430ebc49c51e9090a12b4738ece19004612c73cdc7`.
- Each run made 4 requests, all HTTP 200, with no console errors.
- The browser was Arch Chromium 155.0.8059.84, through a temporary Linux
  manifest, not the pinned macOS build.
- Screenshots are not committed (original artwork); their SHA-256 are in
  `acceptance.json`.
- The Retire and Abort confirmations were inspected: the dialog, the
  question and, for Retire, "Luke Skywalker".

## Gates

- Workspace: 3038 tests passed. The new test is
  `a_mission_icons_menu_reports_abort_without_a_target_planet`.
- `sector-quadrants.mjs`: 12 of 12 (run
  `sector-quadrants-2026-10-28T20-30-25-235Z-183106`). That scenario now
  also reports its menus.
- Mutants on the fixture diff (`--in-place`): the tree copy does not fit
  in `/tmp` beside the 58 GB `.artifacts`.
  - The menu guard survived until the new test.
  - Final run: 44 mutants, 38 caught, 4 unviable, 2 missed.
  - The 2 missed are `* scale` -> `/ scale` in `day_readout_centre`. They
    are equivalent: the fixture canvas is 640 by 480 at scale 1.
  - The `main.rs` changes are format arguments only.
- Provenance scan: 0 uncited items.
- rustfmt: only the file's 7 baseline differences remain.
- Release WASM SHA-256 `2059ab91a395cf455fb38bb5783dd9223cc8dd46db3e5b7f72aab19a5db979d3`.
