# Battle status texts and the opening highlights follow the original (P24)

Date: 2026-10-24.

## Source

The A0 captures of the original `REBEXE.EXE`
(`../2026-10-24-tactical-right-panel-a0/`):

- TAC-01-C003 (Alliance) and TAC-01-C002 (Empire) are the first frame of a
  battle. Both side-highlight buttons show their off art (1034, 1036), and
  no ship carries a highlight bracket. Every later battle opened the same
  way: Coruscant twice, Umgul and Yavin.
- "Battle Paused" has no full stop. It is pure red (255,0,0) for the
  Alliance and pure green (0,255,0) for the Empire. Its capitals are 12
  rows tall from row 33, and it spans x 22-117.
- TAC-06-C005: while observing, "Observe Battle" appears in white at the
  same height, ending at x 451.

## Change

- `highlight_alliance` and `highlight_empire` start false and reset to
  false at each battle's start.
- The status texts use Liberation Sans at 16.5 px, narrowed to 0.93. That
  is `port:`, fitted to the caps height and width. "Battle Paused" sits at
  x 22 in the player's colour, and "Observe Battle" is right-aligned to
  x 451.
- `run.mjs` starts from the off art, and each first click now turns its
  highlight on. The wireframe-colour check compares the lit frame with the
  plain one.

## Gates

- Workspace: 2744 passed, 0 failed.
- New tests:
  - `a_battle_opens_with_both_side_highlights_off`
  - `the_pause_label_takes_the_players_colour`
- Mutants, with both packages' tests: 27 tested, 2 caught, 1 unviable,
  24 missed. They are in macroquad text code (`tactical_status_text_params`,
  `tactical_status_text_width`, `draw_tactical_status_text`, the atlas
  prewarm, `draw_tactical_view`) and in `TacticalState::begin_battle`,
  which needs a whole campaign world. The browser suite covers them: every
  case enters through `begin_battle`, and the initial-off probes fail if
  the reset is lost.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `2b2cc49abe0449a6df2d55f34bf469eb2d8d9918246df99d12c3f28899f7fc89`.
