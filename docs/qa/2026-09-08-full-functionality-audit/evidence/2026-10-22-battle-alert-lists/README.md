# The Battle Alert's list tabs (F-006 / TAC-07)

Date: 2026-10-22. The reading is in `ghidra/notes/battle-alert-window.md`,
"The list tabs".

## Change

- `tactical_view.rs`: tabs 2, 3 and 4 of the Battle Alert show the scene
  with the title only, and the list `FUN_00450770` fills
  (`battle_alert_rows`, drawn by `battle_alert_list`).
  - A side's forces: the heading "Alliance Forces" or "Imperial Forces",
    then each of the side's fleets at the system with its name. Under it
    come its capital ships (font 5, with the mini) and the units aboard
    (font 4, indented 20). Then come the system's name and the side's
    squadrons on the planet.
  - System Assets: the heading, then the facilities, regiments and
    personnel on the planet.
  - The list sits at (83, 40), 256 by 234, with rows 40 high and white
    text. A picture row puts its text at 70, or 90 when indented.
    Pictures are `FUN_0042c3b0(.., 0, 1)` minis over their backdrops,
    with the damage mark for a damaged ship or an injured character.
- Before this change the tabs printed the port's own counts ("Capital
  ships: N   Fighter groups: N").
- `draw_page_ops_at` draws page steps at any window origin.

port: a fleet keeps its squadrons, regiments and personnel, not each
ship, so they follow all of the fleet's ships. Facilities go by kind.
There is no scroll bar: the rows whose middle fits are drawn.

## Gates

- Tests: 7 new. Workspace 2635 passed, 0 failed.
- Mutants: 43 caught, 5 unviable, 17 missed.
  - 14 are in the macroquad wrapper `draw_page_ops_at` (its offsets, and
    the whole function as moved here) and 1 in `draw_battle_alert_text`.
    The screenshots show the drawing.
  - Two are equivalent at this geometry: `>` to `>=` and `/` to `%` in the
    list's cut-off. A row's middle (60 + 40k) never equals the bottom
    (274), and with `%` the same six rows still fit.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build):
  tactical `production-battle-alert-entry` passes 4 of 4 cases. The tab
  screenshots (`*-tab-2..4.png`) show each side's fleet ("Fleet"), its
  capital ship with its 66 by 25 mini, and the squadron aboard indented.
  The fixture system has no assets, so System Assets shows its heading
  only. A pixel diff against the summary tab confirms the minis are drawn.
- Replay equivalence passed.
- Release WASM `77af9a32cf310d7e8557296184a95a2a2a2008f22ab98767e9bb983739863955`.
