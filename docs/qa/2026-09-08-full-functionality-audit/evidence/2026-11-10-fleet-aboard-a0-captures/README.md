# A0 captures: units aboard an Empire fleet and ship

Date: 2026-11-10. These captures are not yet compared with the port.
They are recorded for OBJ-06-C010 (a ship's contents) and OBJ-10-C006
(units aboard a fleet).

## A0

Windows 10 VM, original REBEXE. A new Empire game (Standard), days
10..12. Coruscant's Fleet 1 was opened from the Fleet Finder
(Display), then expanded. The captures are in
`.artifacts/a0-vm/fleet-aboard/`, with their SHA-256 in `SHA256SUMS`.

| File | Shows |
| --- | --- |
| `00-coruscant-fleet1-ships` | Fleet 1: one Imperial Star Destroyer, tabs Ships, Fighters, Troops, Personnel |
| `01-coruscant-fleet1-troops` | the fleet's Troops tab: three Stormtrooper Regiments, the counts 3 and 3 |
| `02-coruscant-fleet1-personnel` | the fleet's Personnel tab: Veers |
| `10-isd1-fighters` | Imperial Star Destroyer 1 selected: its six TIE Fighter squadrons, the counts 6 and 6 |
| `11-isd1-troops` | the ship's Troops: the three regiments |
| `12-isd1-personnel` | the ship's Personnel: Veers |

The ship carries everything the fleet does, because the fleet has one
ship. When a ship is selected, its tabs are Fighters, Troops and
Personnel, with no Ships tab, and its name heads the right pane.

## The comparison (later the same day)

A new fixture scenario, `FleetWindowCoruscant` (code 100, Empire),
rebuilds A0's Fleet 1 at Coruscant:

- one Imperial Star Destroyer, named "Imperial Star Destroyer 1" and
  numbered;
- six TIE Fighter squadrons, three Stormtrooper Regiments embarked, and
  Veers seated aboard;
- the Fleet window at A0's (225, 92), the fleet selected and expanded.

The new case `a0-coruscant` in `fleet-window.mjs` (Empire, Linux
Chromium) compares the window (235 by 304) with A0, tab by tab:

| View | A0 | Differing pixels |
| --- | --- | --- |
| fleet, Ships | `00` | 2012 |
| fleet, Troops | `01` | 3452 |
| fleet, Personnel | `02` | 1501 |
| ship selected, Fighters | `10` | 4292 |
| ship selected, Troops | `11` | 4067 |
| ship selected, Personnel | `12` | 2116 |

Each check fails above a ceiling just over its measured value. Out of
71,440 pixels, what differs is:

- the text, the font deviation accepted on 2026-10-30;
- the ship pane's title, which A0 clips at "Destro" and the port draws a
  few pixels further;
- in the Fighters view, the right list's scroll bar, which A0 draws for
  six items and the port does not draw yet.

All pictures, tabs, frames, content marks and counts match.

## Two defects found and fixed

1. **A selected ship listed nothing.** `right_items` returned an empty
   list for a ship, so its tabs showed no units, and `tab_counts` gave no
   numbers. A0 lists the ship's own squadrons, regiments and Veers, and
   prints 6 and 6, then 3 and 3. The original reads the ship's own slots
   (`FUN_004a5c00`: `+0x23c`/`+0x26c`, `+0x240`/`+0x270`; `FUN_00536da0`
   on the ship). A selected ship now lists the part of the fleet's units
   the port seats on it (`ship_cargo`: squadrons and regiments fill ships
   in fleet order up to capacity, characters by seat). It prints that
   ship's own count and capacity.

   The unit test `a_selected_ship_lists_and_counts_what_it_carries` uses
   two ships of capacity 2, three squadrons and three regiments. It
   expects 2 and 1 squadrons, (2, 2) and (1, 2), and 2 and 1 regiments.
   It fails without the change, where the list was empty.

2. **The fixture's first version** did not number the ship, so Veers had
   no seat and the content mark 10456 was missing under the fleet and the
   ship. That was a fixture error, not a port defect, and the scenario
   now numbers its ships as the Vjun scenario does. The fixture unit test
   `the_a0_coruscant_scenario_loads_fleet_1_as_a0_shows_it` checks the
   seat.

## Still open

- The Fleet window's two lists have no scroll bars. The original builds
  one per list from STRATEGY RCDATA 10699: width 10, track 10657, arrows
  10692..10695, thumb 10696..10698 (`FUN_0060a490`,
  `ghidra/notes/fleet-window.md`).
- The selected ship's pane title is not clipped to the pane as A0's is.

## Batch b173

The first start was stopped when the scenario table's unit test failed.
`SCENARIO_COUNT` had not been raised to 100. Stopping it during the
in-place mutants left a mutant in `fleet_window.rs`, which was restored
before the rerun.

Rerun results:

- 23 of 23 interface gates on Linux, `fleet-window`
  (`fleet-window-2026-11-10T12-33-00-716Z-557001`) among them with
  `a0-coruscant`;
- workspace tests 3500 passed, fixture tests 140;
- tactical matrix `2026-11-10T13-23-16-295Z-665244`: 148 of 148.

The batch's mutants missed two arms of the Personnel filter. The unit
test then gained a seated character (listed under its own ship) and a
special force (under the first ship). A separate mutants run on the
file's diff caught 9, with 1 unviable.
