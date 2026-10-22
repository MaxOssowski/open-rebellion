# The Retire order on a character, save v104 (P08)

Date: 2026-10-22. The reading is in `ghidra/notes/character-retire.md`.
The user approved the save format change.

## Original

- The character's menu lists Retire (`0x242`, TEXTSTRA 12336, sort 2002,
  last).
- Its rule is `FUN_004ed460`, reached through event `0x230`. The base
  check `FUN_004f9860` requires the player's side, recruited, not
  destroyed and not en route. On top of that the character must not be
  on a mission, holding a command or captured. An injured one may retire.
- `FUN_0049a350` asks first, in the move confirmation's window:
  - text 0x7051, "Are you sure you want to retire the following
    characters?", then each name on its own line (0x7054);
  - picture 1034 for the Alliance, 1035 for the Empire.
- The checkmark fires event `0x230`. `FUN_00534c00` destroys the
  character with reason `0x14`, which the Personnel Finder reads as
  "Retired" (0x8799).

## Change

- `rebellion_core::character_retire::{order_allowed, retire}`.
- `Character::is_retired`. **Save v104.** The seed-42 golden is
  regenerated for this cause: only the fingerprints change
  (`v1:0c1f7f5edba92404` -> `v1:efb4263c2b993f4f` initial, final
  `v1:02d192b13b2b3714`). Every checkpoint and the command profile are
  as before.
- The object menu's Retire row: `OrderGates::retire`.
- The confirmation: `ConfirmedOrder::Retire`,
  `MoveConfirmationAction::Retire`.
- The app's `PanelAction::RetireCharacter` validates again and retires.
- The Personnel Finder's "Retired".

Not ported: a special force's Retire, and any message the destruction
raises.

## Gates

- Tests:
  - new: the rule, the retirement, the menu row, the confirmation's
    text, picture and answer, and the Finder's "Retired";
  - the serde compatibility test lists the new field, as each save bump
    does.
  Workspace 2674 passed, 0 failed. The ignored `data/base` replay test
  passes with the regenerated golden.
- Mutants: 20 of 20 caught, 2 unviable.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- No browser probe walks the Retire menu and its confirmation yet. The
  unit tests cover the rule, the window's text and the answer.
- Replay equivalence passed.
- Release WASM `87970551d2aaf9a56ad3a3bdbe5e7d458f1b738287051367f77f88136cd9bdcb`.
