# The Battle Alert's title and summary positions (F-006 / TAC-07)

Date: 2026-10-22. The reading is in `ghidra/notes/battle-alert-window.md`.

## Change

- `tactical_view.rs`: `battle_alert_scene_text` puts the text where
  `FUN_0044f860` paints it into the scene at (12, 13). The title's top
  is at window y 29, 400 wide and centred. The summary sits at window
  (48, 229), 328 wide, over its black shadow at (49, 230). The port had
  the summary at about (38, 211), 340 wide, and the title 5 higher.
  guide-055 shows the summary starting about 48 from the window's left
  edge.
- The list tabs reuse it for the title alone.
- `run.mjs`: the scene probe's ignored band moves with the summary, from
  scene y 145..245 to 200..280.

port: font 6 keeps the port's earlier 14-point size.

## Gates

- Tests: 1 new. Workspace 2636 passed, 0 failed.
- Mutants: 3 caught, 1 unviable, 1 missed: the macroquad wrapper
  `draw_battle_alert_text` as a whole. The screenshots show it.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build):
  tactical `production-battle-alert-entry` passes 4 of 4 cases, with the
  scene compared pixel for pixel outside the text bands.
- Replay equivalence passed.
- Release WASM `094c48f333747d8c414129604f819ecbef6684e1fa6b88a01fd82b5084d888dd`.
