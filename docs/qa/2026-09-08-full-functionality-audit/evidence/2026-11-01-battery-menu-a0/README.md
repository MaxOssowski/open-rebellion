# A battery's pop-up menu against A0 (2026-11-01)

## The case

`sector-quadrants.mjs` gains `battery-menu`. It opens the player's
Defenses window on the primary system, turns to Planetary Batteries,
right-clicks the KDY-150 and reads the menu. Rows: Encyclopedia, Status,
Scrap; Scrap is enabled; no Move or Confirmed Move. The order list is
`FUN_00539c20`'s one 0x200 (`FUN_005399e0`), plus `FUN_0051d990`'s
Encyclopedia and Status. Both sides pass, and the whole gate passes
(`gate-sector-quadrants.txt`).

## Against A0 `status/22-battery-menu.png`

Measured from the frame's top-left corner (Empire screenshot,
`port-sha256.txt`; A0 hashes in `a0-sha256.txt`):

| | A0 | Port |
|---|---|---|
| Frame | 88 x 60 | 88 x 60 |
| Status ink | (6, 26), 36 wide | (6, 27), 36 wide |
| Scrap row ink | (3, 46), 82 wide | (3, 47), 82 wide |

A0's Encyclopedia row is under the cursor and cannot be measured; the
port's is at (6, 6), 77 wide, as in the regiment menu
(`2026-11-01-object-menu-text-a0`). The rows' tops sit a pixel lower by
this measure in the port: the round top of "S" is anti-aliased below the
threshold. Accepted deviations: anti-aliased glyphs and the fill's
palette step.

## Open

A0 `28-fac-menu.png` (a training facility in the System window) lists the
same three rows. The port's list is the same (`object_menu.rs` unit
tests), but no browser case opens a manufacturing facility's menu: the
Quadrants fixture has none. OBJ-14-C007 stays partial.
