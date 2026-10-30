# Tooltips, stage 7: the Fleet window's tabs (CMD-11-C019)

Date: 2026-10-30. Inventory: `ghidra/notes/tooltips.md` §7b.

## The original

`FUN_004a4b10` gives both tab strips the same four TEXTSTRA tips:
- 0x66: 0x3823 "Ships";
- 0x67: 0x3820 "Fighter Squadrons";
- 0x68: 0x3821 "Troop Regiments";
- 0x69: 0x3822 "Personnel".

`FUN_004a6980` disables a tab, which then has its own greyed art. A
disabled tab shows no tip.

## A0

Fleet 1 at Yavin (Alliance). It was opened from the Fleet Finder's
Display, then by a double click in Yavin's system window. Captures are in
`.artifacts/a0-vm/tooltip/` (hashes in `a0-captures.sha256`):
- 140 and 141 show the windows;
- 142–145 show the four tips at the pointer plus (0, 21).

## The port

`FleetWindowTab::tooltip_string` holds the strings. Each enabled tab
registers through `win_tooltip::response_tool`; a disabled one does not.

## Browser (Arch Chromium 155, interface WASM `f883c028…`)

`tooltip.mjs` loads the fleet-load fixture (code 48), opens the Fleet window
from the sector window's fleet icon and chooses the fleet. Results:
- the Ships tab, which is enabled, shows its tip at the pointer plus
  (0, 21), 19 high;
- the Fighters and Troops tabs are disabled for this fleet and show none.

The other tabs' strings are unit-tested, and every tab goes through the
same code.
