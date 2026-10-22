# The Battle Summary's summary page (F-006 / TAC-07)

Date: 2026-10-22. The reading is in `FUN_0046f360` (see
`ghidra/notes/battle-summary-window.md`); compared against the original
capture guide-059 (`docs/qa/2026-09-10-interface-parity-audit/
reference-captures/steam-guide/guide-059.png`).

## Change

`tactical_view.rs`: `battle_summary_page` builds the page as `FUN_0046f360`
paints it:

- the scene;
- the title at (12, 18), 400 wide, centred, font `0xb`;
- the outcome's first line ("The Imperial fleet is victorious.") at (12,
  225), 400 wide, centred, font `0xb`;
- the rest at (24, 255), 352 wide, word-wrapped, font 6, over a black
  shadow at (25, 256);
- all in the viewer's colour.

The port drew every line alike at (25, 230). One executor
(`draw_page_ops`) now draws both results pages.

## Gates

- Tests: 1 new. Workspace 2622 passed, 0 failed.
- Mutants: 5 caught, 7 missed, 1 unviable. The 7 are the macroquad
  executor and its two callers, which need a GPU context. The browser
  probe covers them: the summary scene and the force pages' art must sit
  at the window's offset.
- Browser, Arch Chromium 155.0.8059.84: tactical
  `production-battle-results-presentation` passes for both sides.
  `*-summary.png` matches guide-059's layout. The port's text strings
  and fonts are still its own.
- Replay equivalence passed.
- Release WASM `841c130905b626a063f424b68501f07446cf404a1cf412528ef1fde3db386a7c`.
