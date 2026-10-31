# Reference rail: restore on a double click

Date: 2026-10-31. Source: `ghidra/notes/FUN_00422ce0.c` (the cockpit's
window procedure), lines 985-1036.

## Trace

- `FUN_00422ce0` handles `WM_LBUTTONDBLCLK` (`0x203`). When
  `FUN_004fcee0()` is below 2 it returns at once.
- With a modal child up (`param_1[0x30] == 2`) it passes the message to
  the child under the point (`ChildWindowFromPointEx`) and returns.
- Otherwise it walks the twelve slot rectangles from `param_1 + 0xb8`
  (`PtInRect`). The first slot holding the point calls
  `FUN_00429020(this, slot + 1)`, which restores that slot's window.
- No `WM_LBUTTONDOWN` (`0x201`) branch restores a slot. A single click on
  the rail does nothing.

Windows sends `WM_LBUTTONDBLCLK` on the second press when both presses
fall within the double-click time (`GetDoubleClickTime`, 500 ms by
default) and the double-click rectangle (`SM_CXDOUBLECLK` by
`SM_CYDOUBLECLK`, 4 by 4 by default) of the first press, in the same
window.

## A0

`.artifacts/a0-vm/system-windows` (Alliance Easy, days 11 to 13):
- **07.** A double click on Yavin's facility icon opens its System window
  at (315, 87).
- **08.** Its minimize box sends it to the rail.
- **10.** A single click on the rail slot leaves it there.
- **12.** A double click restores it to (315, 87), on the same page.

## Port

`system_window::draw_reference_rail` pairs the frame's presses in order.
A press on a slot that lies within 500 ms and 2 original pixels of the
last press on the rail restores that slot's window. Any press elsewhere
clears the pairing.

egui's own `double_clicked()` was not used, for two reasons:
- It counts a third click within 0.6 s as a triple, so a click on the
  minimize box followed by a double click on the slot restored nothing.
- It ignores where the first click fell.
