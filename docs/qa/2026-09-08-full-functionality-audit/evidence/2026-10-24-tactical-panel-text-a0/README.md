# The tactical panel texts follow the original (P24)

Date: 2026-10-24.

## Source

- `ghidra/notes/tactical-panel-text.md`:
  - the fighter panel's labels (`FUN_005e1fd0`) use table font 8, a 12 px
    cell;
  - the withdrawal confirmation (`FUN_005f1050`) uses font 10, a 14 px
    cell;
  - the face is Arial (`FUN_0060eed0`), the colour pure white;
  - the fighter portrait sits at panel (0x17, 0x3b), which is (505, 87) on
    screen (`FUN_005e2320`).
- The A0 captures TAC-03-C016, TAC-03-C009, TAC-02-C002 and TAC-06-C003
  give each text's first capital row and left edge:

| Text | x | Capitals from row |
|---|---|---|
| Ship name | 508 | 37 |
| Task-force caption "Task Force # 1" | 508 | 82 |
| Orders / tactics (ship) | 497 | 212 / 223 |
| Group / orders / tactics (fighters) | 508 / 503 / 502 | 39 / 50 / 61 |
| Withdraw Confirmation / two question lines | 502 / 496 | 59 / 123 / 137 |

- The capitals are 7 rows tall for font 8 and 8 rows for font 10. Across
  ten strings the widths fit Liberation Sans, which has Arial's metrics, at
  9.36 and 10.9 px (`port:`, measured, about 0.78 of the cell).

## Change

- The panel texts move from macroquad's built-in font to Liberation Sans
  (already in the repository, `theme::ARIAL_REGULAR`), at the measured
  sizes and positions, in pure white. The font atlas is prewarmed with the
  others.
- The caption reads "Task Force # N". It was "Task Force #N".
- The fighter panel shows the group, its orders and "Tactics: ...", as the
  original does. It used to show the craft name, the group and the orders.
  Its portrait moves to (505, 87).
- `run.mjs`:
  - the fighter portrait probe moves with the portrait;
  - the ship panel's ignored name rectangle also covers the caption;
  - the portrait matte probe moves to the portrait's bottom-right lime
    pixel, which no text covers.

## Gates

- Workspace: 2738 passed, 0 failed. Render tests with the ignored
  bitmap-mask tests: 908 passed.
- New test: `the_task_force_caption_spaces_its_number`.
- Mutants, with both packages' tests: 28 tested, 5 caught, 1 unviable,
  22 missed. All 22 missed mutants are in macroquad text or atlas code
  (`tactical_arial`, `draw_tactical_panel_text`, `tactical_text_font_size`,
  `prewarm_tactical_font_atlas`, `draw_tactical_withdraw_confirmation`,
  `draw_original_tactical_hud`), which needs a GPU context. The browser
  suite's exact-pixel panel probes and the frame-stability checks cover
  them.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `1e0652a10e5f04482759ee4e095666e2288670d62d2fabc7c62aaf2916d72514`.

## Still open

- The fighter count line, "X-wings: 1 12/12" at (505, 174). It is read in
  the note and is the next stage.
- The original's glyphs are GDI's unsmoothed Arial. The port draws smoothed
  Liberation Sans, so single pixels differ.
