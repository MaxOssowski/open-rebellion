# The Battle Alert's title and summary (F-006 / TAC-07)

Date: 2026-10-22. The reading is in `ghidra/notes/battle-alert-window.md`.

## Change

- `tactical_view.rs`: `battle_alert_lines` builds the title from TEXTSTRA
  `0x7020` "Battle at {1:1}". It picks the summary as `FUN_0049c0d0`
  does, from the system's side and its last arrived side
  (`System::last_arrival`, `+0x78` bits 6..7):
  - on the viewer's system, `0x7022` (breaking the enemy's blockade) or
    `0x7021` (the enemy threatens it);
  - on a neutral system, `0x7025`;
  - on the enemy's system, `0x7024` (the enemy breaks the viewer's
    blockade) or `0x7023` (the viewer's fleet has entered).
- `battle_alert_sides` reads those two sides from the world.
- The summary now has the black shadow one pixel down and right
  (`FUN_0044f860`).
- Before this change the port always wrote "The {enemy} fleet has entered
  the {system} system." guide-055 shows the Empire's view of Deyer:
  "The Imperial fleet has entered the Deyer system. Alliance forces have
  been detected on an intercept course." A unit test now checks that
  exact text.

port: the window's positions and font sizes stay as before. The forces
and system tabs are still the port's.

## Gates

- Tests: 3 new. Workspace 2630 passed, 0 failed.
- Mutants: 27 caught, 4 unviable, 5 missed. The missed ones are in the
  macroquad wrapper `draw_battle_alert_text`: the whole function and the
  shadow's offsets. The screenshots show the drawn text.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build):
  tactical `production-battle-alert-entry` passes 4 of 4 cases. The
  fixture system is neutral, so both screenshots read `0x7025` ("Alliance
  and Imperial forces are about to engage in battle near the Akrit'tar
  system.").
- Replay equivalence passed.
- Release WASM `baea039d7fc270243483aeaa89202a52fd7de22d7d819097d688fadfaf5842e5`.
