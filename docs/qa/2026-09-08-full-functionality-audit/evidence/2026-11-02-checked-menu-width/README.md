# A checked menu's width against A0 (2026-11-02)

Follows `2026-11-01-band-menus-a0`. That record left one question
open: the band menu with Reserved checked was 110 pixels wide in the
port and 107 in A0 (`menus3/32`, `33`).

## Source

`FUN_004abf60` measures an item's icon column in this order:

1. The item's own icon `+0x14`.
2. Else, for a checked item (`+0x5c`), its `+0x20` bitmap
   (`FUN_004ab840`).
3. Else 20 for a submenu parent.

The 20-wide check mark that the item draws comes from `+0x1c`, not
from `+0x20`.

`STRATEGY.DLL`'s bitmap headers give these sizes:

| Bitmap | Size |
|---|---|
| 11902, the check mark | 20 by 20 |
| 10117, the submenu arrow | 17 by 20 |

A0 sets Reserved's text 17 pixels in, so its column is 17 wide. hyp:
`+0x20` holds the arrow; which bitmap it is was not traced.

## Change

`game_menu::entry_icon_size` gives a checked item a 17-pixel column.
The unit test that asserted 20 pre-dates the A0 capture, so it now
expects 17.

`build-selection.mjs` asserts the A0 numbers on both sides: the
checked menu is 17 logical pixels wider than the unchecked one (90 to
107), at the same height (122).

## Run

- Gates passed on Arch Chromium 155. Build Selection run:
  `build-selection-2026-11-02T04-39-48-661Z-3322580`.
- The workspace and fixture tests pass.
- Mutants: 9 of 9 caught.
- The batch's other gates pass.
