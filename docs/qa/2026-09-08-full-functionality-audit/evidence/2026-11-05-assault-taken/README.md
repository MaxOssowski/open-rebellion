# The Assault Summary of a taken system (2026-11-05)

This covers EVT-03-C007 ("assault summary") and C008 ("assault success"). The reading is in `ghidra/notes/assault-summary-window.md`.

## A0

The VM's Alliance game, day 633: the Assault on Coruscant report from the Message Index, with the window at (62, 49). The hashes are in `a0-sha256.txt`.
- **Summary** (`assault-coruscant`). The picture is STRATEGY 11160 (`0x2b98`) at (12, 13), with the red title "Assault on Coruscant". The red body reads "Alliance troops have taken control of the Imperial system Coruscant" (`0x7090` / `0x7091`).
- **Alliance Forces** (`*-alliance-regiments`, `*-alliance-tab0..4`). The Trooper Regiments page lists the Alliance's regiments under Operational, with "No Casualties" under Destroyed. The other categories are captured too. The Personnel page has Survivors, Captured and Killed, and Veers stands under Captured.
- **System Forces** (`*-system-regiments`). "None" and "No Casualties".

## Before

- **The fixture.** Its assault always met two enemy regiments, so no browser case took a system.
- **The empty column.** It read "None" and "No Casualties" in a 10-pixel font, 4 below the list's top.

## Now

- **The fixture.** `Scenario::FleetAssaultTaken` (96, fixture code 97) is `FleetAssault` with no regiment of the other side's on the planet, so the assault takes the system.
- **The empty column.** `FUN_0044edb0(0, 0, text)` builds it as a text item in font 6 (`+0x38`), 10 below the item's top (`+0x34`), centred (`+0x50` 5). The port draws it that way (`EMPTY_LABEL_FONT`, `EMPTY_LABEL_Y`). Unit names keep font 10 (`+0x38` 10).

## Tests

- **Unit tests.**
  - `the_rail_shows_each_sides_forces_with_its_columns` now expects font 6, 146. Before, it held the port's own 10-pixel font at 140, which the trace contradicts.
  - `the_assault_scenario_lets_the_fleet_assault_with_a_regiment_aboard` checks the taken variant's planet.
- **`fleet-window.mjs` `assault-taken`** (Linux, both sides). The assault takes the system. The summary reads "Assault on {system}" and "{Alliance|Imperial} troops have taken control of the ...", over `0x2b98` / `0x2b99`. For the Alliance, against A0:
  - **The picture.** Between the title and the body (the bitmap's rows 25..205), it is exact in the port and in A0.
  - **The title.** Its rows match, and its centre is within 3 pixels (the names differ).
  - **The body.** Its first line's corner is within 2 pixels.
  - **The forces pages** (Attackers and System Forces, Trooper Regiments). The category bar is exact, 0 of 14268 pixels. The ink boxes of the subtitle, the category label, both heads, "No Casualties" and, on System Forces, "None" are within 2 pixels and 15% of width.

## Open

- **The list's scroll bar.** A0 draws one on the forces pages; the port has none (EVT-03-C010).
- **Captured personnel.** A0's Personnel page lists Veers as Captured after the assault, but the port's assault captures nobody (hyp in the note). Taking personnel on a captured system is not ported.
- **Rows and other categories.** The Operational rows' pictures and the other five categories' pages are not compared with A0.
