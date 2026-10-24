# The tactical lower HUD follows the original's captures (P24)

Date: 2026-10-24.

## Source

The lossless A0 capture TAC-04-C005 of the original `REBEXE.EXE` (Windows 10
guest, game-set 640x480; see `../2026-10-24-tactical-right-panel-a0/`). Each
TACTICAL.DLL bitmap was located by exact pixel match, leaving out its
transparent key colour (match 1.000 except where noted):

| Control | Bitmap | Original | Port before |
|---|---|---|---|
| Navigation sets 1-4 | 1026-1029 | x 485, 524, 563, 602; y 272 | 485, 521, 560, 601 |
| Pause | 1060/1061 | (561, 308) | (560, 307) |
| Alliance highlight | 1034 | (515, 305) | (517, 304) |
| Empire highlight | 1036 | (483, 305) | (482, 304) |
| Battle Options | 1038 | (608, 309) | (606, 308) |
| Zoom in / out | 1044 / 1046 | (491, 343) / (607, 343) | (486, 343) / (603, 343) |
| Camera left / right | 1048 / 1050 | (497, 381) / (582, 381) (0.76) | (511, 376) / (557, 376) |
| Camera up / down | 1052 / 1055 | (539, 338) / (540, 424) (0.75) | (537, 344) / (537, 412) |
| Camera target | 1058 | (549, 391) | (538, 379) |
| Recall (bottom left) | 1042 | (486, 444) | 1040 at (484, 430) |
| Memorize (bottom right) | 1040 | (594, 444) (0.97) | 1042 at (585, 430) |

- The tooltip at the bottom right reads "Remember Camera Position", and the
  bottom-left button restored the memorized view (TAC-04-C012/C013). So
  1040 is the memorize art and 1042 the recall art. The port had the two
  names swapped.
- The original shows the scene, not blue, through the blue key of 1036 and
  1040 (at (483, 330) and (632..636, 444..)).

## Change

- `TACTICAL_HUD_CONTROLS` and the HUD draw positions take the table's
  values.
- `BTN_CAMERA_MEMORIZE_*` = 1040/1041 and `BTN_CAMERA_RECALL_*` = 1042/1043.
- 1036-1041 key out blue like the other HUD controls.
- `run.mjs` clicks and bitmap probes move with their controls. Each new
  point was checked to land on an opaque pixel of its bitmap.
- The rect and mask tests move their points by each control's own offset.
  `FUN_005f0f40` only places the assign buttons, so the rect test now cites
  the A0 capture.

## Gates

- Workspace: 2736 passed, 0 failed. Render tests with the ignored
  bitmap-mask tests (`--include-ignored`, the original bitmaps are staged):
  906 passed.
- New tests:
  - `camera_memorize_art_is_on_the_right_and_recall_on_the_left`
  - `tactical_empire_highlight_and_memorize_art_key_out_blue`
- Mutants, with both packages' tests: 3 tested, 2 caught, 1 missed. The
  missed one is `draw_original_tactical_hud` -> `()`, a macroquad draw
  function that the browser suite's bitmap probes cover.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `96f14c00cdea345a38790b208f391f7af491abdad48392c0f61f5382c09eb4f1`.
