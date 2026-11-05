# The Message Index's chosen row against A0 (2026-11-05)

This covers CMD-08-C007 ("mission"): a Mission list compared with A0.

## A0

The VM's Alliance game, day 633 (`messages-day633/index-tab3`, the index at (62, 49)). The Mission Messages list holds "Emperor Palpatine Captured", chosen.
- **The row** (at (25, 108) of the window). The bar `0x2aa2` sits at the row's origin: a white rim at x 0, red from x 16.
- **Under the icon.** In the icon's cell (x 1..15, y 1..16) the list's own background shows, stars on (11, 25, 99), not the bar's red.
- **The icon.** STRATEGY `0x2a9e` (10910, 15 by 16), keyed on palette index 0, at (1, 1). All 156 of its opaque pixels match.

## Before

- **The bar.** The port drew it under the icon too, red behind the icon's black.
- **The icon.** It was drawn into a 15 by 15 cell, which squeezed the 16-high icons (`0x2a9a..0x2a9e`) by a row.

## Now

- **`bar_around_icon`.** It draws the bar in pieces around the icon's cell, so the list's background shows there. This is a hyp: the item's selected image is the bar with the icon copied in whole (`FUN_00468ab0`: `FUN_005fd0f0` the bar, `FUN_005fcc30` the icon at (1, 1)), and the list blits it keyed on black (`FUN_006083c0`, `FUN_005fc710`). The list's draw is not read in full.
- **The icon.** It is drawn in its own size, 15 by 15 or 15 by 16.

## Tests

- **Unit test.** `the_chosen_bar_leaves_the_icons_cell_to_the_list` checks the pieces' areas and their texture coordinates, with a 15 by 16 and a 15 by 15 icon. Mutants: 41 caught, 7 unviable. One equivalent survives: `>` to `>=` on a piece's width, since no piece has zero width.
- **`message-display.mjs`** (Linux; the Alliance against A0). The Mission list is chosen and its first row, the capture, selected.
  - The icon's opaque pixels are exact in the port and in A0.
  - The row's head, its first 32 by 17 pixels (bar, icon and the background under it), matches A0 exactly, 0 differing.

## Hover changes no art (PRE-02-C003)

`main-menu-rest.mjs` (Linux) now rests the pointer on each still control of the main menu: the galaxy lever (242, 271), the three size lights and the game type button (305, 333). It compares each control's pixels with the pointer away. All are unchanged, 0 differing. The original has no hover art (`FUN_006028c0` has no `WM_MOUSEMOVE` case), and the gate already finds the menu at rest equal to A0's `tooltip/221..234`.
