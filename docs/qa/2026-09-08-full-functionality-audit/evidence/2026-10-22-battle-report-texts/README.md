# The battle report's sentences (F-006 / TAC-07)

Date: 2026-10-22. The reading is in `ghidra/notes/battle-summary-window.md`,
"The report's texts".

## Change

- `tactical_view.rs`: `battle_report_lines` composes the summary from
  TEXTSTRA RCDATA `0x7002..0x700f`, as `FUN_0049b0f0` picks a builder for
  the viewer:
  - no winner, `FUN_0049b150`: `0x7004`, `0x700e`, then `0x700f`, or
    `0x700c` and `0x700b` by the side that withdrew;
  - the viewer's win, `FUN_0049b330`: `0x7002`; `0x7005` defended,
    `0x7006` under blockade or `0x7007` cleared by the system's side; then
    the enemy's `0x700b` destroyed or `0x700c` withdrawn;
  - the viewer's loss, `FUN_0049b4d0`: `0x7003`; `0x7008`, `0x7009` or
    `0x700a`; then the viewer's `0x700b` or `0x700c`.
  
  The port used to write English of its own, always about the winner. A
  defeated viewer now reads "The Alliance fleet is defeated."
- `set_battle_texts`: the app hands the texts over once.
- `missions_window.rs`: an unused import (from this night's marks stage)
  moves into the tests.

port: the withdrawn fleet's destination (`+0x48`, `0x700d`) is decided
after the report, so it reads `0x700c`. The blockade-before side (`+0x5c`,
`0x7015`/`0x7016`) is not kept. Missing texts fall back to the port's
English.

## Gates

- Tests: 3 new. Workspace 2624 passed, 0 failed.
- Mutants: 23 caught, 1 missed (the macroquad wrapper; the browser probe
  draws the page), 1 unviable.
- Browser, Arch Chromium 155.0.8059.84: tactical
  `production-battle-results-presentation` passes. `alliance-summary.png`
  reads "The Alliance fleet is victorious." and "Akrit'tar has been
  cleared of Imperial forces. The Imperial fleet has been completely
  destroyed." from TEXTSTRA.
- Replay equivalence passed.
- Release WASM `6d20f409ed0a566d11cd8a969109549ca0ce75cff12e5c70c5adb3c7c7baa683`.
