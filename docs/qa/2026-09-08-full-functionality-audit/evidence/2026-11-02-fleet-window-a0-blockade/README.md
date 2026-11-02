# The Fleet window over a blockaded system against A0 (2026-11-02)

## A0 (VM, the `test` Alliance save, day 119)

The captures are under `.artifacts/a0-vm/fleet4/`; their hashes are in
`a0-sha256.txt`.

- `05` is the battle report. Fleet 8 (Corellian Corvette 1, and Medium
  Transport 1 with two regiments) reached Imperial Bortras and won the
  battle there. The report reads: "The Alliance fleet is victorious.
  Bortras is now under blockade by Alliance forces."
- `bortras-blockade-{tab}` show Bortras' Fleet window at (305, 86), with
  Fleet 8 selected, one capture per tab:
  - The window marks the blockade nowhere.
  - The corvette carries its damage flames, on its item and on the
    fleet's picture.
  - The title strip is red, as for the Alliance viewer's own systems.

## The port's copy

`FleetWindowBlockade` (code 87) builds this state:

- It moves `place_yavin`'s fleet, renamed "Fleet 8", to Bortras.
- Bortras is held by the Empire.
- The corvette is at half hull. A0 shows only the flames; its hull was
  not read.
- The window opens at A0's place, with Fleet 8 selected.

`fleet-window.mjs alliance/a0-blockade` compares each tab with A0. No
rendering code changed: the port already matched.

| Tab | Differing pixels | Ceiling |
|---|---|---|
| Ships | 1764 | 1950 |
| Fighters | 797 | 950 |
| Troops | 2164 | 2350 |
| Personnel | 699 | 850 |

What differs is text hinting, which is an accepted deviation. The blockade
messages (`0x70a0`, `0x7015`/`0x7016`) are covered in
`2026-10-21-blockade-browser` and `2026-10-27-blockade-report`.

## Tests

- `the_a0_blockade_scenario_puts_fleet_8_damaged_over_imperial_bortras`.

## Run

Gate run `fleet-window-2026-11-02T03-45-19-054Z-3194048` (Arch Chromium
155):

- 81 cases pass.
- Test WASM:
  `896b429b6deaf1b0463510be722783080a131e9eda42dceabc00497a19d8be8f`.
- The batch's other gates pass.
- The workspace passes 3314 tests and the fixtures 111.
