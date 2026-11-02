# The speed menu's hovered row against A0 (2026-11-02)

Surface ledger CMD-05 C008, "hover".

## A0

The A0 run used the Windows 10 VM and the Alliance's "DS_flip_allian" save, day 172. The captures are in `.artifacts/a0-vm/speed2/`, with hashes in `a0-sha256.txt`.

- **`01-menu`.** A right click on the day readout at (140, 22) opens the Game Speed menu, its corner at (140, 24). The rows are Pause, Very Slow, Slow, Medium and Fast, white, each with the side's red bars.
- **`02-hover-slow`.** With the pointer on Slow, only that row's text turns red (255, 0, 0). It is the side's colour, which `FUN_0042d190` passes as the highlight. The fill does not change.

## The port

- **`agent-objectives.mjs` (Alliance).** The gate opens the menu from the day readout at (144, 27), so its corner is at (144, 28). It moves the pointer onto Slow and measures the red ink relative to the menu's corner:
  - A0: (23, 46)..(49, 55), 71 pixels;
  - port: (23, 46)..(50, 55), 76 pixels.

  The 1 extra pixel is the text-width deviation.
- **`personnel-orders.mjs`.** It asserts the menu's five rows and its 86 by 102 frame, as A0 `status/08-speed` shows.

Run `agent-objectives-2026-11-02T13-20-31-909Z` (Arch Chromium): pass.

## Pressed, and a release beside the menu (CMD-05 C009)

- **`03-pressed-slow`.** A held left press on Slow looks as the hover does: the same red text at (23, 46)..(49, 55) from the corner. There is no pressed fill and no mark.
- **`04-released-outside`.** The press dragged off and released beside the menu leaves the menu open, with the highlight on the last row under the pointer.

`agent-objectives.mjs` holds the press on Slow and finds the same red ink as for the hover: (23, 46)..(50, 55). It then drags off and releases at (300, 300), and the menu is still drawn. The port dismisses the menu only on a new press outside it (`game_menu.rs`), as A0 does.

Run `agent-objectives-2026-11-02T13-23-02-900Z`: pass.
