# The replacement Officers panel is removed (P08)

Date: 2026-10-22. Decided by the user: remove the replacement panels,
each once its original windows cover it.

## What the panel showed, and where the original shows it

| Panel content | Original window |
|---|---|
| The character list and where each one is | Personnel Finder (`personnel-finder.md`, F5 / `0x12f`), ported |
| Captured, injured, on a mission, en route | Character Status, "Status:" (`FUN_004486f0`), ported |
| Force ranking and the skills | Character Status rows (Force Ranking, Diplomacy, Espionage, Combat, Leadership, R&D) |
| Can be Admiral / General / Commander | The character's Command submenu offers only the allowed roles (`character-command.md`), ported |
| The portrait | Character Status picture (`FUN_0042c3b0`), ported |

The original has no counterpart for the panel's search field and its
"MAJOR / MINOR" and "MANDATORY" labels. They go with the panel.

## Change

- Deleted `rebellion-render/src/panels/officers.rs`, its exports, and
  `PanelAction::FocusCharacter`, which changed nothing in the game.
- Removed the app's panel state and the letter key O.
- The other replacement panels keep their keys: F, N, J, B, D, L.

## Gates

- Workspace 2667 passed, 0 failed. The panel had no tests.
- Mutants: no mutable code added; the change only removes code.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- Replay equivalence passed.
- Release WASM `f7312ce0366a361624ffa06d701ce3573d16f54dc83addef5c44057e3d7700f3`.
- Native GUI acceptance was not run.
