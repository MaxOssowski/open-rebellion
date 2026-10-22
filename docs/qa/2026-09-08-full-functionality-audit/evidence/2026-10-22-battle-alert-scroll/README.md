# The Battle Alert's list scrolls with the wheel (F-006 / TAC-07)

Date: 2026-10-22. Notes: `ghidra/notes/battle-alert-window.md`, "The list
tabs".

## Change

- `tactical_view.rs`: the original list (`FUN_00607ea0`) has a scroll bar
  (`0x29fc`). The port draws none, as in its other lists, and now lets
  the wheel over the list scroll it a row at a time
  (`alert_list_scroll`, through `assault_summary::scrolled`). Six rows
  show; a new tab, or a new alert, starts at the top
  (`select_battle_alert_tab`).
- Before this change, rows past the sixth could not be seen.

## Gates

- Tests: 2 new. Workspace 2639 passed, 0 failed.
- Mutants: 14 caught, 2 unviable, 4 missed:
  - `draw_battle_alert_text`'s row count (0 or 1) and the tab click's
    `&&`, in the macroquad wrappers;
  - `TacticalState::begin_battle` as a whole, which gained only the
    scroll reset. The browser gate opens the alert through it.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build):
  tactical `production-battle-alert-entry` passes 4 of 4 cases. The
  fixture's lists are shorter than six rows, so no scroll shows.
  `alliance-tab-2.png` is the forces tab.
- Replay equivalence passed.
- Release WASM `2712ce6bed6c2b59f99e43fc9ea0d47d31f52908acb4c3658161a95eba5cd351`.
