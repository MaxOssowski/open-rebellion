# The Retire order on a character (read 2026-10-22)

## The order and its event

- Order `0x242`: `FUN_004f45b0` -> `FUN_004f4560`, vtable `0x0065d068`.
  - Its slot `+0x10` (`FUN_0040f340`) returns 1, so the character's menu
    lists it: the class's order list (`FUN_004f2400`) holds `0x242`, and
    STRATEGY record `0x242` gives TEXTSTRA 12336 "Retire", sort 2002, last
    on the menu.
  - Its `+0x18` and `+0x1c` (`FUN_0051fe20`, `FUN_005201c0`) build event
    `0x230` through slot `+0x48` (`FUN_004f4620` -> `FUN_00540c40`, vtable
    `0x00661af0`, `Event230`) and check it (`FUN_005535b0`).
- Event `0x230` resolves the member and asks the object itself:
  - `+0x18` (`FUN_00540da0`) calls slot `+0x1c0`;
  - `+0x1c` (`FUN_00540e70`) calls slot `+0x1c4`;
  - the fire `+0x20` (`FUN_00540f40`) calls slot `+0x200`.

## A character's slots (vtable `0x0065ca70`)

- `+0x1c0`, `FUN_004ed460`: `FUN_00533ba0` (the base check `FUN_004f9860`,
  then code 1 for a character on a mission, `+0x78` bit 7). Then code 1
  for one holding a command (`+0x96`) and code 2 for a captured one (`+0xac`
  bit 0). An injured character may retire.
- `FUN_004f9860`, the generic check:
  - 1: the object is not the ordering side's (`+0x24` bits 6..7);
  - 2: not completed (`+0x50` bit 2), except for orders `0x203`, `0x215`,
    `0x204`, `0x241`;
  - 3: destroyed (`+0x50` bit 3);
  - 4: en route (`+0x50` bit 4).
- `+0x1c4`, `FUN_00533c60`: calls `+0x1c0` again.
- `+0x200`, `FUN_00534b70`: `+0x1c4`, then `+0x204` (`FUN_00534c00`), which
  is slot `+0xac(0x14, ..)`: the character is destroyed with reason `0x14`.
  The Personnel Finder reads reasons `0x14..0x16` as "Retired"
  (`FUN_00465bb0`, TEXTSTRA `0x8799`).

## The confirmation

`FUN_00487740` validates the order and then `FUN_00487cc0` asks for kind
`0x242`. `FUN_0048a340` builds the confirmation (`FUN_0049a1b0`), and
`FUN_0049a350`'s case `0x242` fills it:
- the text: RCDATA 0x7051 "Are you sure you want to retire the following
  characters?", then 0x7054 ("\n{1:1}") with each member's name;
- the picture `+0x2e`: `0x40a` (1034) for side 1, `0x40b` (1035) otherwise;
- `+0x30`: `0x461` / `0x462`.

The window is the move confirmation's (`FUN_0044f060`, `move-order.md`).
Its checkmark resubmits with force 1 (`FUN_0041ce20`), which validates
again and fires.

## A special force

The special-force class (vtable `0x0065e160`, slot `+0x3c` `FUN_00503b50`)
lists the same unit orders as a character (`FUN_00503fa0` ->
`FUN_00536ca0` -> `FUN_00536bc0`: `0x201`, `0x202`, `0x242`, `0x240`,
`0x204`, `0x241`), without Command. It inherits the Role slots:
- `+0x1c0`, `FUN_00533ba0`: the base check, then code 1 for a unit on a
  mission;
- `+0x200`/`+0x204`: the same destruction with reason `0x14`.

## The AI

`FUN_004d0610` would give special forces a Retire, but no code sets the
record flag it needs (`ai-production-modules.md`).

## Ported (2026-10-22)

- `rebellion_core::character_retire::{order_allowed, retire}`. The
  retirement takes the character off its fleet's roster and out of its
  abode (`missions::kill_character`), as a destruction does.
- `special_force_allowed` and `retire_special_force`: the unit leaves its
  system or abode and the world.
- `Character::is_retired`, save v104.
- The object menu's Retire row (`OrderGates::retire`).
- The confirmation (`move_confirmation::ConfirmedOrder::Retire`).
- The Finder's "Retired".

port: the meaning of `+0x30` and any message the destruction raises are
not ported. A special force's confirmation line is its class name.
