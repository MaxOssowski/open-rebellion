# A covered control takes no click, against A0 (2026-11-03)

This covers surface ledger cell CMD-10 C020, "window overlap ordering".

## A0

The run used the Alliance save at days 304–305 (Windows 10 VM, original `REBEXE.EXE`). The captures are `.artifacts/a0-vm/personnel-states/36-*` (ignored); the hashes are in `a0-sha256.txt`.

- **`36-pzob-fleet-over-xyquine`.** Xyquine's System window, at (98, 62), is on its Mines page. Pzob's Fleet window lies over it at (147, 88), and its title bar covers Xyquine's tab row from x 147 on.
- **`36-covered-tab-clicked`.** A click at (267, 96) falls on Xyquine's Refineries tab, under Pzob's title bar. The screen is unchanged, identical to the capture before the click (no differing pixel).
- **`36-xyquine-still-mines`.** With Pzob's window minimized, Xyquine still shows Mines: the covered tab took no click.

## The port

`sector-quadrants.mjs`'s `two-defenses` case now runs on Linux for both sides:

1. It drags the second Defenses window onto one of the first's other tabs.
2. It clicks that covered point.
3. It asserts that the first window keeps its page.
4. It asserts that the top window takes the click: its own tab at that point switches it to Squadrons.

Run `sector-quadrants-2026-11-03T21-04-25-920Z-3616229`: pass.
