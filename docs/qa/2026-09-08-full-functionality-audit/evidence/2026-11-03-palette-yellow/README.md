# The original's yellow, 0x200f0f0 (2026-11-03)

## A0

Three things are drawn in the COLORREF `0x200f0f0`: the sector window's title (`FUN_00459e30`), the reference rail's names (`FUN_00428d70`), and a Manufacturing band's progress bar (`FUN_004acec0`). A0 draws all three in (255, 255, 0), never in (240, 240, 0):

- `personnel-states/24-mon-calamari-building-band` (Alliance, day 208):
  - the "Corellian" title has 123 pixels of (255, 255, 0);
  - Mon Calamari's facilities band, building an Orbital Shipyard for Drall, has 352 pixels of (255, 255, 0) in its progress bar, x 370..457, rows 375..378.
- `rail/01..05`: the rail's names, 400 to 600 pixels of (255, 255, 0) per capture.

The high byte `0x02` makes the COLORREF a PALETTERGB. The game runs with its 8-bit palette, so (240, 240, 0) is drawn as the palette's nearest entry, (255, 255, 0).

## The port

`theme::PALETTE_YELLOW` is (255, 255, 0). The sector title, rail-name and progress-bar colours use it; before, all three were (240, 240, 0).

Test: the sector window's title test now asserts (255, 255, 0), with the A0 citation. It fails with the old colour.

## The progress bar's place

In window coordinates, the facilities band's bar row is the same in A0 and in the port:

- `build-selection2/07` (Coruscant, from the window's frame corner): black rows 289..293;
- the port's `build-selection.mjs` confirmed screen: the same rows;
- A0 `24`, from Mon Calamari's frame corner at y 86: the yellow at 289..292.

The port's bar is `PROGRESS_RECTS[2]` = (56, 289, 160, 4).

No browser case shows a band with progress above zero yet, so OBJ-09 C002 stays partial.
