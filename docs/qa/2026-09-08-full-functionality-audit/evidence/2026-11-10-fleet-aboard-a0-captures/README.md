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

## What is compared already

`fleet-window.mjs` compares the Alliance's tabs with A0:
- `a0-kimanan` (`menus/23-fleet1-*`), including two Alliance Fleet
  Regiments aboard;
- `a0-umgul`, with each ship selected (`fleet2/umgul-*-selected`). Those
  ships carry nothing.

So a ship that carries units, and the Empire's units aboard, have no
browser comparison yet. A case would need a fixture fleet of one
Imperial Star Destroyer with six TIE Fighter squadrons, three
Stormtrooper Regiments and Veers.
