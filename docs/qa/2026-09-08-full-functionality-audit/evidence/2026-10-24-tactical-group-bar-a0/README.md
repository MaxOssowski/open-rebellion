# The tactical group bar follows the original's captures (P24)

Date: 2026-10-24.

## Source

The A0 captures TAC-02-C002 (Alliance) and TAC-01-C002 (Empire) of the
original `REBEXE.EXE` (see `../2026-10-24-tactical-right-panel-a0/`). Each
bitmap was located by exact pixel match:

| Element | Bitmap | Original | Port before |
|---|---|---|---|
| Task-force header | 1001 / 1004 | (9, 1), match 1.000 / 0.988 | (10, 1) |
| Task-force slots 1-8 | 1005-1007 | x 60 + 29k (1007 at 89, 118, ... 263, 0.99) | 60 + 26k |
| Fighter header | 1008 / 1010 | (296, 1), 0.999 / 0.987 | (275, 1) |
| Squadron buttons 1-4 | 1012-1020 | x 347 + 29k (1016 at 347, 1013 at 376, 1020 at 434) | 330 + 26k |

## Change

- `TASK_FORCE_BUTTONS` and `FIGHTER_GROUP_BUTTONS` take the captured slots.
  The draw code takes its x positions from them, so drawing and hit tests
  agree.
- The headers move to x 9 and x 296.
- `run.mjs` moves its group-bar probes and clicks with their slots.
- The mask test moves its points by each slot's offset, and now also checks
  that a point between two slots hits nothing.

## Gates

- Workspace: 2737 passed, 0 failed. Render tests with the ignored
  bitmap-mask tests: 907 passed.
- New test: `group_bar_slots_are_29_pixels_apart`.
- Mutants, with both packages' tests: 1 tested, 1 missed. The missed one is
  `draw_original_tactical_hud` -> `()`, a draw function that the browser
  bitmap probes cover. The changed lines are constants, which mutants do
  not alter.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `0415aaff64df34ee7afa43d0bb014e59ca48c18ac5a9077e7a5e24e1a3eaa401`.
