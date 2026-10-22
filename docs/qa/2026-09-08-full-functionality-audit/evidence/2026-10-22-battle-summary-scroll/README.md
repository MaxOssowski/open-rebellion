# The Battle Summary's forces list scrolls with the wheel (F-006 / TAC-07)

Date: 2026-10-22. Notes: `ghidra/notes/battle-summary-window.md`.

## Change

- `tactical_view.rs`: the forces page's list (37, 136), 347 by 166, draws
  no scroll bar. As in the port's other lists, the wheel over it now
  scrolls all its columns together a row at a time
  (`result_list_scroll`). Two 70-high rows show, or three 50-high rows
  for Personnel. Another tab or category starts at the top
  (`select_resetting_scroll`, which the Battle Alert's tabs use too).
- Before this change, rows past the second (or third) could not be seen.

## Gates

- Tests: 1 new. Workspace 2640 passed, 0 failed.
- Mutants: 23 caught, 3 unviable, 7 missed:
  - the macroquad wrappers: the page's row count, and the click tests'
    `&&` in the alert and the results windows;
  - `TacticalState::begin_battle` and the results reset as a whole,
    which gained only the scroll reset. The browser gate goes through
    them;
  - `== Personnel` in `battle_result_list_length`, which is equivalent:
    the Captured column is always empty, so the longest column is the
    same either way.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build):
  tactical `production-battle-results-presentation` passes 4 of 4 cases.
  Its lists fit, so no scroll shows.
- Replay equivalence passed.
- Release WASM `4ace9cbc16ddde25ab65f2e7296aa71ac041aaecfe7648e3e32deade88509807`.
