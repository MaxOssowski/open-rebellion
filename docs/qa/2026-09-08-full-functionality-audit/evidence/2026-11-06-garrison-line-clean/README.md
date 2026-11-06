# The garrison line against an unedited Alliance game (2026-11-06)

This covers OBJ-02-C006 (the troop tab), OBJ-02-C013 (a hostile window) and OBJ-10-C007 (the garrison requirement). It closes the open point of `../2026-11-02-defenses-pages-a0/`.

## What was open

`FUN_004a90d0` draws "Garrison Requirement: N" on the regiment page when the system's holder is the local player's side (`FUN_0041cdb0`). The port draws the line the same way.

The A0 captures of the time disagreed:
- the line showed on the Empire's Umgul and Coruscant;
- it did not show on the "Alliance player's" Mon Calamari and Pzob.

Those captures came from DS_flip_allian. That host-edited save builds its messages for the Empire behind an Alliance cockpit (see `../2026-11-06-occupation-and-finder-sides/`), so its local side was the Empire.

## A0 in an unedited game

`.artifacts/a0-vm/garrison-clean/test-day483-yavin-regiments.png` (sha256 `95003b7e9f8448a70596a3cc3943e0e29171f44912ccdf92928caf05474007a1`):
- the "test" save, the Alliance, day 483;
- the Troop Finder's Yavin row opens the Alliance's Yavin on its regiment page;
- the page reads "Trooper Regiments" over "Garrison Requirement: 0".

The line follows the player's own side, as the code reads and the port draws.

## Browser (`sector-quadrants.mjs` 'defenses', Linux, Alliance)

- The existing case already asserts "Garrison Requirement: N" on the player's own system and no line on the other side's ('defenses-other-side').
- It now also compares the line's ink box (window rows 66..79, above the regiment mini) with A0 Yavin's, window at (305, 86):
  - left and top within 2 pixels;
  - width within 15%.
- Relative to its window, the port's line spans x 46..185 and A0's 46..184; both start at row 66.
- The run passes on both sides: `.artifacts/interface-parity/sector-quadrants-2026-11-06T19-29-29-558Z-1640848`. Screenshot `alliance-defenses/regiments-label-screen.png`, sha256 `bbc3a8af9d273740e1a73d69b8cb6ac23290d57524127c3f737c1a98c4821f97`.
