# The Personnel Finder's SpecForces view against A0 (2026-11-02)

Surface ledger OBJ-03 C018, "SpecForces Finder type-by-system grid".

## A0

The A0 run used the Windows 10 VM and the Alliance's "DS_flip_allian" save, days 165..170. The captures are in `.artifacts/a0-vm/specforces/`, with hashes in `a0-sha256.txt`.

- **`01-alliance-grid-day165`.** F5 opens the Personnel Finder at (62, 50). Its second view button opens the SpecForces view:
  - one row per system: Drall, Mon Calamari, Tralus, Yavin;
  - four type columns under icons;
  - a count in a red box, or "-".

  The boxes of units en route or on a mission (Mon Calamari, Tralus, Yavin) are dark red. Drall's is bright.
- **`03-mon-calamari-personnel`.** Display on Mon Calamari opens its Defenses window. The Longprobe Y-wing Recon Team is listed last. Its menu greys Mission because the team is en route.
- **`02-recon-mission-praesitlyn-day170`.** Display on Tralus opens the window of the team's running mission instead: "Mission at Praesitlyn", Reconnaissance.

## The port's view

`encyclopedia-placement.mjs --window=finders` (Alliance) now presses the SpecForces view button: the finder's (423, 147), 0x36 lower.

- **Header strip.** It compares the strip of the four type icons, the window's (256..370, 114..142), with A0 `01`: 0 of 3,192 pixels differ.
- **The rest of the view.** It matches A0 by eye in `personnel-finder-specforces.png`: the row and column places, the boxes, the dashes and the grey system names. The port's world has other systems (Ghorman, Ithor).
- **Not compared: dark red boxes.** No unit in the port's run was moving or on a mission, so the dark red state of en-route and on-mission units was not compared.

Run `encyclopedia-placement-2026-11-02T12-38-36-124Z` (Arch Chromium, release WASM `e1ad53f8…`): pass.

## The dark boxes (ported the same day)

`FUN_00465540` counts each column's units. A unit whose `+0x50` bit 0 is clear (en route or on a mission) sets the column's flag 4. Such a column is painted with the side's dark brush: (128, 0, 0) for the Alliance, (0, 128, 0) for the Empire. These are the brushes 3 and 4 of the Troop Finder's `FUN_0046d8d0`, which has the same test.

The port's `count_sf` now clears `all_usable` for a unit on a mission. A unit moves as a Move mission in the port, so en route is covered too. `sf_cell_colours` picks the dark brush for that column. Before this change, every box was bright.

Test: `a_column_with_a_unit_away_is_drawn_in_the_sides_dark_brush` (`personnel_finder.rs`).

The Troop Finder keeps its bright boxes. A0's Troop Finder (DS_flip_allian, day 164) shows Xyquine's three regiments dark, but why their bit 0 is clear is not known.
