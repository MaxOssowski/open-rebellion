# The reference rail's twelve fixed slots against A0 (2026-11-03)

This covers surface ledger cell CMD-04 C013, "twelve-slot reference-rail capacity".

## The original

- `FUN_00428b40` minimizes a window into the first empty slot of the twelve.
- `FUN_00429020` empties a slot when its window is restored. The other slots keep their windows, so the restored window's slot stays empty until something is minimized into it.
- When all twelve slots are taken, the newest window replaces the oldest minimized one, in that window's own slot.

## A0

The run used the Alliance save at day 303 (Windows 10 VM, original `REBEXE.EXE`). The captures are in `.artifacts/a0-vm/personnel-states/`, which is ignored; the hashes are in `a0-sha256.txt`.

- **`32-rail-restored-slot-gap`.** Mon Calamari's Fleet window, restored from the fifth slot, is open. Its slot is empty, and Duros through Yavin stay where they were.
- **`32-rail-reminimized-same-slot`.** Minimized again, the window takes the fifth slot back.
- **`33-rail-full-thirteenth-open`.** All twelve slots are taken and Xyquine's System window is open.
- **`33-rail-thirteenth-in-oldest-slot`.** Xyquine, minimized as the thirteenth, takes the second slot, where Uvena (the oldest minimized) was. It does not go to the end, and the other eleven do not move.

## The port

Before this change, `SystemWindowState.rail` was a list. A restore removed the window's entry and shifted the later windows up, and when the rail was full the oldest window was dropped from the front and the new one appended at the end.

The rail is now twelve `Option` slots plus a minimize-age order:

- A restore empties its window's slot.
- A minimize fills the first empty slot.
- On a full rail, a minimize replaces the oldest window in its slot.
- A window that is already on the rail keeps its slot.

The unit test `a_full_rail_puts_the_thirteenth_in_the_oldest_slot_and_a_restore_leaves_a_gap` follows A0 32 and 33. It fails on the old list.

In the browser, the `sector-quadrants.mjs` `open-system` case now runs on Linux for both sides:

1. It minimizes the System window, then the same system's Defenses window, which takes the next slot.
2. It restores the System window and asserts that the Defenses window keeps its slot rectangle.
3. It minimizes the System window again and asserts that it is back in its first slot rectangle.

Run `sector-quadrants-2026-11-03T19-35-03-519Z-3458131`: pass.

The browser case does not fill all twelve slots. The thirteenth's replacement is covered by A0 33 and the unit test.

## The empty slot (CMD-04 C002)

`FUN_00428d70` draws only occupied slots, so an empty slot shows the cockpit shell beneath it.

- **A0.** In `32-rail-restored-slot-gap`, the empty fifth slot at (544, 149, 62, 18) matches STRATEGY 900 at the same place: 0 of 1116 pixels differ.
- **The port.** `open-system` now takes a screenshot at the gap and compares the empty slot with the shell (900 for the Alliance, 901 for the Empire) at the slot's rectangle. It passes for both sides.

Run `sector-quadrants-2026-11-03T19-38-54-198Z-3466068`: pass.

A0 shows only the Alliance's empty slot; the Empire's empty slot is checked against shell 901 alone.
