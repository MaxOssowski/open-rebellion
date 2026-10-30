# Tooltips, stage 8: the System Defenses and Missions windows (CMD-11-C020, C021)

Date: 2026-10-30. Inventory: `ghidra/notes/tooltips.md` §7c, §7d.

## The original

`FUN_004a8790` gives the System Defenses window's five tabs these TEXTSTRA
tips:

| Tab | String |
|---|---|
| Personnel | 0x1744 |
| Trooper Regiments | 0x1743 |
| Fighter Squadrons | 0x1742 |
| Planetary Shields | 0x1741 |
| Planetary Batteries | 0x1740 |

No Defenses tab is ever disabled. `FUN_0049f540` gives the Missions
window's two tabs 0x8500 "Agents" and 0x8501 "Decoys". The title-bar
buttons have no tip.

## A0

The A0 run is Coruscant's System Defenses window, opened by a double click
on the bottom-left quadrant icon in the Sesswenna sector window. Captures
are in `.artifacts/a0-vm/tooltip/` (hashes in `a0-captures.sha256`):
- 152 shows the window;
- 153–157 show the five tips at the pointer plus (0, 21), 19 high.

Coruscant had no mission, so the Missions window did not open in this save
(158). Its tabs have no A0 capture.

## The port

`DefensesPage::tooltip_string` and `MissionsTab::tooltip_string` hold the
strings. Each tab registers through `win_tooltip::response_tool`. This
also replaces an old port note in `defenses_window.rs` that said "the help
messages (`FUN_00600a40`) are not shown".

## Browser (Arch Chromium 155, run on a temporary Linux copy)

`sector-quadrants.mjs` passes all twelve cases for both sides
(`sector-quadrants-result.json`). On the open windows it rests the pointer
for a second on each tab and finds the tip at A0's geometry:
- the five Defenses tabs;
- the Missions window's Agents and Decoys tabs, with a mission chosen.

C021 (Missions) stays partial: it has no A0 capture of the tips.
