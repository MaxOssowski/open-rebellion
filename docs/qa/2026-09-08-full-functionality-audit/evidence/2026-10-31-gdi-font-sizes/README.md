# GDI font sizes, and a full gate run (interface)

Date: 2026-10-31.

## Font sizes

GDI does not draw the game's fonts at the size the port assumed:
- The port took the em as `lfHeight × 2048 / 2288`: 12.53 for the
  14-pixel cell and 14.3 for the 16-pixel cell.
- GDI picks the ppem from Arial's VDMX table, which gives 11 and 13.

The method and the table are in `ghidra/notes/gdi-font-sizes.md`. Windows
10's Arial was copied out of the A0 VM; only the numbers are kept.

| A0 capture | Text | Size | Result |
|---|---|---|---|
| `sector-windows/04-left` | Churba's ten planet names (font 10) | 11 ppem | 6 match pixel for pixel; the other 4 differ in single pixels of "a", "k" and "A" |
| `sector-windows/04-left` | the title "Churba" (font 4) | 13 ppem | matches pixel for pixel |
| `tooltip/321` | "Resume Game Play?" (font 5) | bold 13 | 2 of 131 × 13 pixels differ |

All three were rasterised with FreeType's v35 interpreter, `hdmx`
advances and `kern` pairs.

`theme::original_font_ppem` now holds these sizes, with a test. The egui
sizes are unchanged in this stage, for this reason:
- A trial that moved every game font to egui cells matched to GDI's widths
  cut the mean width error per string from 6–10 pixels to 1–3.
- But it moved text that had been placed against A0 at the old sizes:
  - main menu label;
  - Game Options name box;
  - sector title.
- Text placement will move to GDI's metrics (tmAscent and tmHeight from
  VDMX) window by window, each against A0.

## Full gate run (Linux Chromium, test WASM at the egui sizes of HEAD)

**Pass:** advisor-events, agent-advice, agent-build, agent-objectives,
audit-batch, briefing, build-selection, command-events, cutscenes,
encyclopedia-placement, fleet-finder, fleet-window, game-options,
main-menu-rest, main-menu-states, message-display, mission-dialog,
personnel-orders, sector-quadrants, sector-windows, status-window,
system-finder, tooltip and `run.mjs --all` (152 of 152).

**Fixed in this stage:**
- `story-movie`. MDATA.102 is queued at the day's check. With the 0.25 s
  sub-tick (d0edd25b) it starts some time after 104 ends, so the gate now
  waits up to 4 s for a next movie.
- `fleet-registry` and `galaxy-map-visual-check`. They waited for the main
  menu while the release site played the intro (see
  `../2026-10-30-stale-gates/`), and now skip the movies with Escape.

**Still failing, as recorded before:**
- `message-index-shell`, 470 pixels: the window's last row. The gate
  expects DIB blue, the port shows what lies behind, and A0
  `modal2/17-msg` (row 382) shows black. The port is not right yet.
- `encyclopedia-index-shell`, 62 pixels: the buttons' first-pixel corners.
- `fleet-move`: the confirmed-move probe.

**Not run here:**
- `encyclopedia-art` needs `REBELLION_EDATA_DIR`.
- `game-options-smoke` needs a production build served on port 8769.
