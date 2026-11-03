# A double click on a sector window's icon opens at the pointer (2026-11-03)

Surface ledger CMD-03 C012, "double-click object".

## A0

The A0 run used the Alliance's DS_flip_allian save, day 301. The captures are in `.artifacts/a0-vm/personnel-states/`, with hashes in `a0-sha256.txt`.

- **`29-pzob-icons-before`.** Pzob's quadrant icons in the Moddell sector window. The Missions icon, bottom right, starts at about x 148.
- **`29-mission-at-pzob-double-click`.** A double click on that icon at (172, 286) opens "Mission at Pzob": Ship and Facility Design Research, Wedge Antilles. The window's corner is at (172, 86):
  - x is the pointer's;
  - y is clamped into the galaxy view, 390 − 304.

## The source

`FUN_004593e0` case `0x203` passes the pointer to `FUN_0045aac0`. A Finder's route (`FUN_0045c8e0`) passes the icon's top left instead; see `2026-11-03-finder-display-pages`.

## The port

`sector_window.rs` opens a double-clicked icon's window at the pointer (`open_point`).

The `defenses` case of `sector-quadrants.mjs` (Linux, both sides) double-clicks the primary system's Defenses icon 4 pixels in from its left. It now asserts that the window's x is the pointer's, clamped to the galaxy view's right edge less 235. The icon's corner would be 4 pixels off. Run `sector-quadrants-2026-11-03T15-00-22-150Z-3021856`: pass.
