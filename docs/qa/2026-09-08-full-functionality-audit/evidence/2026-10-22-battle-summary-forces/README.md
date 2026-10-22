# The Battle Summary's forces page (F-006 / TAC-07)

Date: 2026-10-22. The reading is in `ghidra/notes/battle-summary-window.md`.

## Change

`tactical_view.rs`: the forces page follows `FUN_0046f360`,
`FUN_00471200`, `FUN_004714e0` and `FUN_004724a0`:

- the side's label in its colour;
- "Filters" and the bitmap category bar at (130, 59);
- the category's label;
- Operational / Destroyed (two-column art 10761), or Survivors / Captured
  / Killed for Personnel (10762);
- the original empty-column lines;
- pictures with names in 165 by 70 or 110 by 50 cells.

The port's invented "Damaged" column is gone. The page is built as
drawing steps (`battle_force_page`) that the tests check.

`tools/interface-parity/run.mjs`:

- the results probe now expects the two-column art and clicks the
  original bar, and checks the three-column art on Personnel;
- it no longer asserts the "[post_battle]" log of the invented
  post-battle bombardment. That bombardment was removed in `6c9220a5`
  (the original never bombards after a battle,
  `ghidra/notes/bombardment-order.md`). This assertion had failed since
  then.

## Gates

- Tests: 4 new, 1 changed by the page's own rule (the grid's cells, not
  the heads, place the empty-column lines). Workspace 2620 passed, 0
  failed.
- Mutants: 81 caught, 14 missed, 5 unviable. The misses:
  - 12 are in the macroquad wrapper and the click offsets. They need a
    GPU context. The browser probe covers them: the page art sits at the
    window offset, and a click on the Personnel button must bring up the
    three-column art.
  - 2 are the port's earlier regiment rule (capacity, strength).
- Browser, Arch Chromium 155.0.8059.84: tactical
  `production-battle-results-presentation` passes for both sides
  (`tactical-results-summary.json`, `*-category-*.png`).
- Replay equivalence passed.
- Release WASM `d46051f022af79e98d6875261e2de9c886d2b7216babfd22e1ecb274df72d546`.
