# Tooltips, stage 6: the system window's tabs (CMD-11-C022)

Date: 2026-10-30. Inventory: `ghidra/notes/tooltips.md` §7e.

## The original

`FUN_00455060` gives the six tabs TEXTSTRA tips:

| Tab | String |
|---|---|
| Manufacturing | 0x1835 |
| Shipyards | 0x1828 |
| Training Facilities | 0x1830 |
| Construction Yards | 0x1832 |
| Refineries | 0x1827 |
| Mines | 0x1826 |

The title-bar buttons pass tooltip 0, so they have no tip.

## A0

Coruscant's system window, Empire. Captures are in `.artifacts/a0-vm/tooltip/`
(hashes in `a0-captures.sha256`):
- 124–129 show the six tips at the pointer plus (0, 21). They are 86, 58,
  98, 108, 58 and 39 pixels wide.
- 130 and 131 show the title-bar minimize and close buttons with no tip.

## The port

`SystemWindowTab::tooltip_string` holds the strings. Each tab registers
through `win_tooltip::response_tool`. The title-bar buttons register
nothing.

## Browser (Arch Chromium 155, interface WASM `bca6d998…`)

`tooltip.mjs` opens the Build Selection fixture (code 55), which shows the
first system's window on its producer bands. It places the window from the
ships band at (55, 57) and rests the pointer on each tab. All six tips sit
at the pointer plus (0, 21) and are 19 high. They are 88, 63, 107, 116, 64
and 40 wide: the accepted text-width deviation, with the same texts.

The window stands elsewhere than in A0, so the probe checks A0's
placement rule, not the A0 file.
