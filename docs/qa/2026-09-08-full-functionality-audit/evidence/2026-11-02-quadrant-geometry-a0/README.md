# Quadrant items around a planet, against A0 (2026-11-02)

Surface ledger CMD-03 C015, "quadrant icon geometry".

## A0

The capture is `2026-10-28-nav/sector-a` (the Alliance, Moddell sector, day 8). It shows Vjun's planet with three quadrant items. Each item was located by its STRATEGY bitmap, every non-key pixel equal:

| Item | Bitmap | Corner |
|---|---|---|
| System | 10771 | (220, 132) |
| Defenses | 10773 | (220, 152) |
| Fleets | 10775 | (249, 132) |

Vjun's planet picture (SYSTEMSD picture 8, STRATEGY 10219, 37 by 37) is best placed at (230, 133). Part of it is covered by the items and the headquarters mark.

## The port

`sector_window::quadrant_rect` puts the items at the planet's (-10, -1), (-10, 19) and (19, -1), 28 by 19. For a planet at (230, 133) that is exactly A0's (220, 132), (220, 152) and (249, 132).

Test: `quadrant_items_stand_where_a0_shows_them_around_their_planet`.

`sector-quadrants.mjs` (Linux) already compares each item's art at these rectangles, 0 pixels differing.

## A selected planet (CMD-03 C011)

**A0.** `2026-10-28-nav/adega-selected` shows Adega after a click. There is no frame, and its name is still white: the selection does not change how the planet is drawn.

**The port.** `sector-windows.mjs` now clicks Lelmra in Churba's window (step `04-left`). Before the click, the planets and names there already matched A0. The gate then compares the planet's 37 by 41 cell and its name's box with the frame before the click. Both are unchanged: 0 pixels differ.

Run `sector-windows-2026-11-02T19-16-52-127Z`: pass.
