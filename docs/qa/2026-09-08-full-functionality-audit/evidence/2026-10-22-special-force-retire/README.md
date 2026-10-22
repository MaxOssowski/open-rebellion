# Retire for special forces, and a retired character leaves its fleet (P08)

Date: 2026-10-22. The reading is in `ghidra/notes/character-retire.md`.

## Original

- The special-force class (vtable `0x0065e160`, slot `+0x3c`
  `FUN_00503b50` -> `FUN_00536bc0`) lists the unit orders a character
  has, Retire `0x242` among them, without Command.
- Its rule is the Role base `FUN_00533ba0`: the base check (the player's
  side, not en route), then a unit on a mission is refused.
- The fire destroys it with reason `0x14` (`FUN_00534c00`), as for a
  character.

## Change

- `character_retire::{special_force_allowed, retire_special_force}`. The
  unit leaves its system or abode and the world.
- The object menu enables a special force's Retire. The confirmation
  lists its class name. The app's `PanelAction::Retire(MissionMember)`
  serves both kinds of member.
- Fix to the previous stage: a retired character is now taken off its
  fleet's roster and out of its abode (`missions::kill_character`), as a
  destruction does. Before, only its own fields were cleared.

## Gates

- Tests:
  - new: a special force's Retire;
  - the character's retirement now checks the fleet roster. That
    assertion fails on the previous stage's code.
  - the menu test: a special force's Retire is enabled; a regiment
    lists none.
  Workspace 2675 passed, 0 failed.
- Mutants: 10 of 10 caught, 2 unviable.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- Replay equivalence passed.
- Release WASM `fb8adfe323a4eca739d801f9eec4e469253d561450013d896feecdff3ad58612`.
