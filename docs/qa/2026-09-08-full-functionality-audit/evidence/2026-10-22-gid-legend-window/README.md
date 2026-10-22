# The GID legend window (P07, stage B)

Date: 2026-10-22. The reading is in `ghidra/notes/gid-markers.md`.

## Change

- Choosing a display opens its legend window (`FUN_00426d00`), drawn from
  the display's STRATEGY.DLL layout (`FUN_004522f0`, loaded in stage A):
  - bitmaps and TEXTSTRA texts at their offsets; a text on row 1 is
    centred across the top;
  - a 2-pixel frame from tiles `0x2774..0x277b` (`FUN_00607740`);
  - the close button `0x277c` (pressed `0x277d`) at `width - 14 - 3`, 3;
  - the galaxy showing through the rest (`FUN_00452240`).
- The window starts at (150, 120) (`FUN_00421c70`), is moved in so 180 x
  240 fits, and keeps its place between displays.
- The menu reopens the legend even for the shown display (command
  `0x140`); the keys do so only for a new one (`FUN_00425d00`).
- The close button hides the legend and leaves the legend button,
  STRATEGY 10168 at (55, 50) or (113, 50); the button restores the
  legend and hides itself (`FUN_00422ce0` `0x405` case 3, command
  `0x134`). Display Off hides both.
- The window drags by its body, inside the command centre.
- The galaxy takes no click under the legend or its button.
- Before this change the port drew 10168 as a fixed picture and had no
  legend window.

port: the button opens the legend before any display has, since the port
starts on Popular Support where the original starts with Display Off.
hyp: the text is drawn at three quarters of the font's cell height
(16 for font 4, 14 for font 10) in egui's font; the original's face name
is not read. hyp: the drag is live and clamped to the command centre;
the original's drop clamp is not read.

## Gates

- Tests: 11 new in `gid_legend` (layout per display, opening and its
  clamp, close and restore, the area the galaxy gives up, drag, frame
  tiles, close button, fonts, centred title, screen mapping). Workspace
  2663 passed, 0 failed.
- Mutants: 86 caught, 16 unviable, 14 missed:
  - 11 in `draw_gid_legend`, which needs an egui frame; the browser
    probe below covers what it draws and does (frame pixels, close,
    restore, drag);
  - `frame_tiles` `>` to `>=` (two): equivalent, since an edge of length
    0 draws nothing;
  - `draw_cockpit_egui_layer -> None`: unchanged since before this stage.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152 cases. The run checks the legend
  window in every case that chooses a display:
  - at 640 x 480, the frame's grey corner and white inner line at
    (150, 120);
  - the close button hides the legend;
  - the legend button brings it back and hides itself;
  - a drag of 40, 20 moves the frame to (190, 140).
- The Popular Support case now compares its returns from Game Options
  with the command centre after the selection, legend open, not before
  it. Command `0x140` opens the legend, so the old baseline no longer
  held.
- Replay equivalence passed.
- Release WASM `3f8fbe201891f560260bb5e99c54f91fe1b88c4adbed4ad6e4641a31bb0584cc`.

Screenshots: the Alliance's Popular Support legend open, closed and
dragged; the Empire's Mines legend; the Alliance's Idle Fleets legend
letterboxed at 1280 x 800.
