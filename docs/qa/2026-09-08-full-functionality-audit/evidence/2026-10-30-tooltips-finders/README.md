# Tooltips, stage 2: the four Finders (CMD-11-C008..C011)

Date: 2026-10-30. Inventory: `ghidra/notes/tooltips.md` §3a–3d. The first
stage (the shared tip) is `../2026-10-30-tooltips/`.

## The original

All four Finders add their tools with `FUN_00600a40` and `FUN_00606c90`.
The strings are TEXTSTRA:

| Finder | Tabs | Other controls |
|---|---|---|
| System (`FUN_00460270`) | 0x1867 All, 0x1868 Rebel, 0x1869 Imperial, 0x1870 Neutral, 0x1871 Unexplored Systems | |
| Fleet (`FUN_00461960`) | 0x1882..0x1884 All / Alliance / Imperial Fleets | 0x1885 Ship Finder, 0x1880 Fleet Finder |
| Personnel (`FUN_004637a0`) | 0x1893 Alliance Personnel, 0x1894 Imperial Personnel | 0x1899 Characters, 0x1898 SpecForces |
| Troop (`FUN_0046ce40`) | 0x1950 Alliance Troops, 0x1951 Imperial Troops | |

Every Finder also has 0x1954 "Close" and 0x1953 "Display".

Two Finder controls behave differently:
- **Fleet Finder side tabs.** In Ship mode `FUN_00462be0` renames them with
  `TTM_UPDATETIPTEXT` to 0x1887..0x1889 "All / Alliance / Imperial Ships".
- **Personnel Finder view tabs.** The command handler (`FUN_00464890` case
  0x6e) disables the chosen view tab with `EnableWindow(FALSE)` and draws
  it pressed. Win32 shows no tip for a disabled window, so only the other
  view tab has one. Fleet Finder's mode tabs are never disabled, so both
  show tips. See "Disabled controls show no tip" in the notes.

## A0 (Windows 10 VM, Empire, days 243–245)

Captures are in `.artifacts/a0-vm/tooltip/`, numbered 30–67; hashes are in
`a0-captures.sha256`. Each capture has a `*-base.png` taken right after the
pointer arrives, before the tip.

| Window | Captures | What they show |
|---|---|---|
| System Finder | 30–36 | five tabs, Close, Display |
| Fleet Finder | 41–47 | three tabs, Close, Display, both mode tabs |
| Fleet Finder, Ship mode | 48–50, 66, 67 | "All Ships", "Alliance Ships", "Imperial Ships"; both mode tabs keep their tips |
| Troop Finder | 51–54 | two tabs, Close, Display |
| Personnel Finder | 56–61 | two tabs, Close, Display, SpecForces |
| Personnel Finder, view tabs | 60, 62, 63 | no tip on the chosen Characters tab |
| Personnel Finder, after choosing SpecForces | 64, 65 | Characters has a tip; the chosen SpecForces has none |

## The port

`win_tooltip::window_tool(ui, rect, id, string, enabled)` registers a
window control. It needs all of these:
- the pointer is inside the control (`PtInRect`);
- the window's layer is the top one under the pointer;
- the control and its `Ui` are enabled.

Each Finder calls it for its tabs, Close and Display:
- `SystemTab::tooltip_string`;
- `fleet_finder::tab_tooltip_string(tab, mode)` and `mode_tooltip_string`;
- `PersonnelTab` / `PersonnelView::tooltip_string`;
- `TroopTab::tooltip_string`;
- `system_finder::CLOSE_TOOLTIP` / `DISPLAY_TOOLTIP`.

The Personnel Finder passes `enabled = false` for the chosen view tab.

## Browser (Arch Chromium 155, interface WASM `c91d1e64…`)

`tooltip.mjs --a0=.artifacts/a0-vm/tooltip` passes. On the Empire side it
opens each Finder (F2, F3, F4, F5) and rests the pointer on the spot of
every A0 capture above. Each check uses the tip's box:
- its left edge, top and height must equal A0's;
- where the wider tip (accepted text-width deviation) moves left, it must
  end at 639: Fleet Finder's mode tabs and Personnel's SpecForces;
- where A0 has no tip (the chosen Characters tab), the port must have none.

All 28 Finder checks pass, together with the first stage's checks.

## Tests

- Unit tests cover the tab strings of each Finder, the Fleet/Ship renaming,
  and `window_tool`. The `window_tool` test checks the inside spot and the
  `PtInRect` edges, and that a disabled control, a disabled `Ui` or a
  covering window shows no tip.
- `cargo mutants` covers `--re "tooltip|window_tool|pt_in_rect"` on the five
  files.
