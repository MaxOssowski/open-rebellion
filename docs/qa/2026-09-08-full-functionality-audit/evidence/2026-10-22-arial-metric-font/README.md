# The original's font: Arial, drawn with Liberation Sans

Date: 2026-10-22. The user approved adding the font.

## Reading

- `FUN_0060ee20(0x6ac7b8)` (called at `0x5ff83a` on the font table
  `0x6be4f8`) copies "Arial" to the table's `+0x38`. `FUN_0060eed0`
  creates every font of the table from that face.
- Each font has a positive `lfHeight`, the cell (ascent plus descent),
  and a weight:

| Index | Cell | Weight |
|---|---|---|
| 1, 5 | 16 | 700 |
| 2 | 14 | 700 |
| 3 | 8 | 400 |
| 6 | 18 | 400 |
| 7 | 18 | 700 |
| 8, 9 | 12 | 400 |
| 10 | 14 | 400 |
| 11 | 24 | 400 |
| 12 | 30 | 400 |
| 13 | 12 | 900 |
| others | 16 | 400 |

## Change

- Arial cannot be shipped. Liberation Sans 2.1.5 (SIL OFL 1.1) is its
  metric match: 2048 units, 1854 + 434. It was already in
  `assets/fonts/`, but it was read from disk, which the browser cannot
  do, so the browser used egui's own face.
- It is now built in with `include_bytes!`, regular and bold, so every
  platform and working directory gets it. The licence text is now
  beside it in `assets/fonts/LICENSE-OFL.txt`.
- `theme::original_font(index, scale)` gives the original's font as
  egui's: the bold face for weights 700 and 900, and the size
  `cell x 2048 / 2288`. epaint sizes by the em (it scales by the face's
  height over its em), while GDI's positive `lfHeight` is the cell; so
  16 gives 14.32.
- The GID legend draws its texts with it, in place of the guessed three
  quarters of the cell in egui's face.
- The browser build grows by 0.83 MB: 8,638,685 bytes, from 7,807,886.

Every proportional text of the port now uses this face in the browser
too. It already did natively when the game started from the repository.

## Gates

- Tests: font table, egui size and families (`theme`), replacing the
  legend's cell test. Workspace 2667 passed, 0 failed.
- Mutants: 26 of 28 caught. The 2 missed need an egui frame:
  `draw_gid_legend` (the browser probe covers it) and `load_fonts`
  (it hands `font_definitions`, which is tested, to egui; the
  screenshots show the face).
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build):
  - GID catalog: 152 of 152;
  - `status-window`, `fleet-window`, `message-index-shell`: pass.
  - `game-options-smoke` did not start: it needs Playwright's bundled
    headless shell, which this machine lacks. That is unrelated to this
    change.
- Replay equivalence passed.
- Release WASM `a62a534c4d3cef9c986cb21214e331c381bbe43642073083819567b935b486fc`.
- Native GUI acceptance (`cua-driver`) was not run.
