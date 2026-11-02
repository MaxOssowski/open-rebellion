# Sabotage in Create Mission against A0 (2026-11-02)

Surface ledger OBJ-11 C014 "Sabotage".

## A0

The A0 run used the Windows 10 VM and the Empire's "resource turn 14" save, days 193..195. The captures are in `.artifacts/a0-vm/kinds2/`, with hashes in `a0-sha256.txt`.

1. Veers was at Coruscant. Coruscant's Defenses window was moved left. Enemy Yavin's Manufacturing window was opened on its Refineries page by double-clicking the facility icon (`40`).
2. Dragging Veers onto the refinery does nothing.
3. Veers' menu (`41`), then Mission, then a click on the refinery (`42`), opens Create Mission at (230, 40) (`43`, `44`):
   - Sabotage is the only kind (`45`, the drop-down);
   - "Target" is the refinery's picture, "Refinery".
4. The kind's 130 by 65 icon, GOKRES 7186 (TEXTSTRA 11282 & 0xfff, + 0x1000), sits at (300, 126), the window's (70, 86). Rows 0..63 match the resource at 0 of 8,320 pixels.

## The port

`mission-dialog.mjs` now runs scenario `SabotageTargeting` (58, code 59), on each side, as the case `sabotage`.

- **Setup.** An enemy shipyard is in the secondary system's window. The agent's Mission is released on it through the original entry.
- **Checks.**
  - The dialog offers `["Sabotage"]`.
  - Its chrome matches the composed STRATEGY bitmaps, as in the other cases.
  - The kind icon is found at the window's (70, 86), A0's place: 3090 for the Alliance and 7186 for the Empire, 0 pixels differing.

Run `mission-dialog-2026-11-02T09-56-32-342Z-3987181`, Linux Chromium 155, test WASM `07d33e9a…`: pass.
