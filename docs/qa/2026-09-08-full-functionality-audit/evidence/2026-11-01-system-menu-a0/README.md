# A planet's right click in a sector window against A0 (2026-11-01)

## A0

- New capture (VM Alliance game, day 315): with the player's fleet icon at
  Kimanan selected (`menus/08-fleet-icon-menu.png`), a right click on the
  planet Joiol opens the fleet icon's menu (`menus/10-planet-right.png`).
  The press on a planet hits no overlay, so the window keeps its
  selection and the menu lists that selection's orders
  (`sector-icon-menus.md`: `FUN_004593e0` returns at once;
  `FUN_004ac5c0`, `FUN_0051d990`).
- `status/07-sys-menu.png` and `25-sys-menu.png`: Encyclopedia white and
  Status grey over a planet. Encyclopedia is enabled for one selected
  object; Status is always disabled for an icon, whose object is a system.

Hashes in `a0-sha256.txt`.

## The case

`sector-quadrants.mjs` gains `system-menu`:

1. With nothing selected, a right click on a planet lists Encyclopedia and
   Status, both grey (brightest pixel 128), because no object is selected.
2. A left press selects the primary system's mission icon; a right click
   on the headquarters' planet then lists that icon's Encyclopedia (white),
   Status (grey) and Abort, as A0 keeps the fleet icon.

Both sides pass, and the whole gate (16 cases) passes
(`gate-sector-quadrants.txt`; screenshots in `port-sha256.txt`).

The case's first version expected Encyclopedia white with nothing
selected, reading `07` and `25` as a planet's own menu. The new capture
shows they list whatever was selected before.
