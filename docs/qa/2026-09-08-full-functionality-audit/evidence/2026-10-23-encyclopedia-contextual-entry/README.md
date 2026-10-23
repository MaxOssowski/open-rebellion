# The Encyclopedia opens at an object's topic (P35)

Date: 2026-10-23.

## Source

`FUN_0045d400` resolves an initial game object and shows its topic, or
falls back to the index (`ghidra/notes/encyclopedia-window.md`).

## Change

- `encyclopedia::original_encyclopedia_object`: a menu object's
  Encyclopedia object, the catalog's compound id (the DAT id of the
  character, or of the class of the special force, regiment, capital
  ship, squadron or defense facility; a system icon gives its system,
  port). A fleet or production manager has none.
- `open_original_encyclopedia_at`: All Databases at that object's topic
  when the catalog lists it, else the index.
- The object menu's Encyclopedia uses them when the original source
  loaded.

## Gates

- Workspace 2705 passed, 0 failed
  (`a_menu_object_opens_the_encyclopedia_at_its_topic`).
- Mutants on the touched render code: 5 tested, 5 caught.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152 (no menu-entry journey yet).
- Replay equivalence passed.
- Release WASM `ce5d8c7f4593402aa723e78d607b3474d164f367c811275708d2e899ec9fc848`.
