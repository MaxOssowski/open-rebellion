# Reference rail restores on a double click (CMD-04)

Date: 2026-10-31.

## A0

The VM's Alliance Easy game, days 11 to 13
(`.artifacts/a0-vm/system-windows`, hashes in `a0-captures.sha256`):

- **07-icon-dbl.** A double click on Yavin's facility icon in the Sumitra
  sector window opens the Yavin System window at (315, 87).
- **08-minimized.** The window's minimize box (515, 96) sends it to the
  right rail's first slot.
- **09-fleet, 11-fleet-closed.** The fleet icon opens the Fleet window,
  and its close box closes it.
- **10-restored.** A single click on the rail slot does nothing. The
  slot keeps Yavin.
- **12-rail-dbl.** A double click on the slot restores the System window
  to (315, 87), on the page it showed, and empties the slot.

## Finding and fix

A single click restored a slot's window in the port. The original
restores only on `WM_LBUTTONDBLCLK` (`FUN_00422ce0`, `0x203`; see
`ghidra/notes/reference-rail-restore.md`).

The rail now pairs presses as Windows does: a second press on a slot
within 500 ms and 2 original pixels of the first. egui's own double click
was not usable here. It counts the minimize press and the two slot presses
as a triple click, and it ignores where the first press fell.

New unit tests in `system_window.rs` cover:
- a single click;
- a press off the rail followed by one on a slot;
- two presses in one frame;
- the time limit;
- the scaled pixel rectangle;
- a window lying over the slot.

`cargo mutants` on `is_double_click` and `draw_reference_rail` caught 17
mutants; 4 did not build.

## Gates

Both gates now restore with a double click and pass on Linux Chromium
(test WASM `02bb3356…`):
- `fleet-window.mjs` (60 cases): `fleet-window-summary.json`.
- `sector-quadrants.mjs` (12 cases): `sector-quadrants-summary.json`.

Each gate checks that the window returns to its place.
