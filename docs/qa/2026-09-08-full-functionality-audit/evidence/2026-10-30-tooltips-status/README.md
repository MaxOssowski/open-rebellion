# Tooltips, stage 5: the Status window (CMD-11-C014)

Date: 2026-10-30. Inventory: `ghidra/notes/tooltips.md` §6a.

`FUN_00443020` adds the window's two buttons with `FUN_00606c90`:
- Encyclopedia (0x66), TEXTSTRA 0x1505 "Encyclopedia";
- Close (0x65), TEXTSTRA 0x1954 "Close".

## A0

The A0 run is a Manufacturing Status of Coruscant's Training Facility,
opened from the object menu's Status. Captures are in
`.artifacts/a0-vm/tooltip/` (hashes in `a0-captures.sha256`):
- `121-status.png` shows the window;
- `122-st-ency.png` shows "Encyclopedia" at (445, 333), with the pointer at
  (445, 312);
- `123-st-close.png` shows "Close" at (508, 333).

Both are the pointer plus (0, 21), 19 high.

## The port

`status_window.rs` registers both buttons with `win_tooltip::window_tool`.
The Encyclopedia button registers only while it is enabled
(`view.encyclopedia`); a disabled control shows no tip.

## Browser (Arch Chromium 155, run on a temporary Linux copy)

`status-window.mjs` passes all ten cases (`status-window-result.json`):
character, regiment, fleet, capital ship and production manager, for both
sides. After the pixel comparison, each case rests the pointer for one
second on each button. It then checks the tip's border and fill at A0's
geometry: the top-left border pixel at the pointer plus (0, 21), the bottom
border 18 rows lower, and the fill inside. Every case shows both tips.
