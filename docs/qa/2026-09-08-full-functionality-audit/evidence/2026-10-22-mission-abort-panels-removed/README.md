# Abort on the missions icon; the last replacement panels are removed (P09)

Date: 2026-10-22. Decided by the user: remove each replacement panel once
the original windows and menus cover it.

## Abort (`0x250`)

- The missions icon's object menu enables Abort when the player has a
  visible mission in the system (`quadrant_icons::own_visible_missions`;
  the team of `FUN_00512700` kind `0x40`, filtered to the side).
- The confirmation window asks "Are you sure you want to abort this
  mission?" (TEXTSTRA `0x7052`) with picture `0x40c`/`0x40d` (1036/1037),
  `FUN_0049a350` case `0x250`.
- The answer builds event `0x261` (`FUN_0054e090`). It checks each mission
  again through `FUN_00521600` (`MissionState::abort_allowed`: the side's,
  refused in phase 4 with `0x40` and in any phase but 8 with `0x41`), then
  fires `FUN_005233a0` -> `+0x1dc(2)`, ending it with code 2
  (`MissionState::abort`).

## Panels removed

| Panel (key) | Original |
|---|---|
| Missions (N): active missions, cancel | the Missions window and the missions icon's Abort, ported |
| Bombardment (B) | the fleet's Planetary Bombardment orders `0x220..0x223` from the object menu, ported |
| Death Star (D) | read-only since DS4; the Death Star's orders are the fleet's object menu |
| Loyalty (L): danger and uprising risk | not shown by the original; support appears in the system window and the GID filters |

`PanelAction::CancelMission` and the panels' keys, state and draw code are
deleted. `left_panel_open` stays `false`: no replacement side panel remains.

## Gates

- Workspace 2674 passed, 0 failed (the exclusive-panel toggle test went
  with its helper).
- Mutants on the touched core and render files: 29 tested, 27 caught,
  2 unviable, none missed.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- Replay equivalence passed.
- Release WASM `aaf255d2fb483c81de611afc804d0305db3054dad94b43e4385ad9de448ca8a2`.
