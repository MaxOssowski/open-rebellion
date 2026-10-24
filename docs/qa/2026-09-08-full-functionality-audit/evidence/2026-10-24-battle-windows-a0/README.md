# The Battle Alert and Battle Summary windows follow the A0 captures (P24)

Date: 2026-10-24.

## Source

The A0 captures of the original `REBEXE.EXE`
(`../2026-10-24-tactical-right-panel-a0/`): TAC-01-C010 (Alliance alert),
TAC-07-C001/C005/C013 (Alliance summary pages), TAC-07-C002 (Imperial
summary) and the Imperial Umgul alert. Located by exact pixel match:

- **Window origin.** Over the Alliance command center the alert's frame
  10710 and the summary's rail 10820 (at window x 412) put the window at
  (62, 49). Over the Empire's, frame 10711 and rail 10821 (537, 52) put it
  at (125, 52). The port had the alert at (65, 54) and the summary at
  (85, 74) for both sides.
- **Alert rail.** The alert's tab buttons sit on frame 10710's own rail:
  the rail columns match 10710 and not 10820. The port also drew 10820,
  with its empty fifth socket.
- **Texts.** `ghidra/notes/battle-alert-window.md` (`FUN_0044f860`): the
  title at window (0, 16), 400 wide, centred, font `0xb`; the summary at
  window (36, 216), 328 wide, font 6, over a shadow at (37, 217). In the
  capture the title is centred on window x 200 with capitals from row 20,
  and the summary starts at x 36 with capitals from row 219. The port had
  added the scene offset (12, 13) to both.
- **Summary page** (`FUN_0046f360`). The title capitals start at window
  row 22, the headline's at 229 (font `0xb`), and the rest at 258, at x 24
  with an 18-row step (font 6). On the Display page the lines start at 43
  and 133. The capitals are 15 rows tall for font `0xb` and 12 for font 6.
  Liberation Sans at 22 and 16.5 px, narrowed to 0.93, fits their widths
  (`port:`).
- **Colours.** Pure red (255,0,0) and green (0,255,0); the port had
  (255,45,40) and (40,255,65). No pure blue pixel shows in either window:
  the frames, tabs, buttons and destination art are blitted keyed on their
  blue.

## Change

- The window origin is per side, set by each window's draw from the
  player's faction.
- The alert no longer draws rail 10820.
- The window texts use Arial (Liberation Sans) at fonts `0xb` and 6, at the
  captured places. Lines step by the font's cell.
- The texts are pure red or green, and the window art keys out blue.
- Tests and `run.mjs` probes move by the window's offset per side. Each
  change cites its capture. The suite's alert-rail probe goes, since the
  rail is not drawn.

## Gates

- Workspace: 2749 passed, 0 failed. Render tests with the ignored
  bitmap-mask tests: 919 passed.
- New tests:
  - `the_battle_windows_open_over_each_sides_command_center`
  - `the_battle_windows_art_keys_out_blue`
- Changed expectations, on the captures above:
  - `the_battle_alert_scene_text_sits_where_the_original_paints_it`
  - `the_display_page_lays_out_its_buttons_like_the_original`
  - `the_summary_page_heads_the_outcome_over_its_shadowed_lines`
  - the two wheel-scroll tests, which follow the new window origin
- Mutants, with both packages' tests: 135 tested, 43 caught, 6 unviable,
  86 missed. Every missed mutant is in macroquad drawing or text measuring
  (`draw_original_battle_alert`, `draw_original_battle_results`,
  `draw_result_*`, `draw_page_ops`, `battle_window_text_*`). They entered
  the diff when the window constants became a per-side origin. The browser
  probes cover them: exact frame, tab, button and scene pixels at the new
  origins, for both factions.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `f6a64c8044786b127f8269751f3c4cbde7f6410bc8d04e7358ad27774d1c58b8`.
