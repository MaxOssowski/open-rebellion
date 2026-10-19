# Character command (CommandKind, `+0x96`)

Read 2026-10-19 from the saved decompile corpus (`rebexe-decompile/decompiled`)
and REBEXE.EXE (vtables read from the image). F-038; the parity tracker's
item "Dowodzenie i jego utrata przy ranie".

## The field

Character short `+0x96` is `CommandKind` (the range check's message is
"Invalid CommandKind value", `FUN_004ee350`):

| Value | Command | Name prefix (`FUN_004f17f0`, slot `+0x330`) |
|---|---|---|
| 0 | none | — |
| 1 | Commander | TEXTSTRA `+0x6800` |
| 2 | Admiral | `+0x6c00` |
| 3 | General | `+0x7000` |

The prefixes follow `decoy-roll.md` ("Officer rank and detector slots"): 37440
"Commander Ackbar", 38464 "Admiral Ackbar", 39488 "General Ackbar".

`FUN_004ee350(kind, ctx)` is the setter. It checks the value is not negative
(`FUN_0053fc90`), stores it, notifies both sides (`FUN_004f04f0`), and calls
slot `+0x330` (`FUN_004f17f0`, the name with its prefix) with the old and new
values.

## The orders `0x260..0x263`

The character's Command submenu (`object-popup-menu.md`; parent `0x160`,
manual p. 99) holds four orders, each registered by `FUN_0051f4b0`:

| Order | Factory | Vtable | CommandKind (`FUN_0053ebb0`) | Menu item |
|---|---|---|---|---|
| `0x260` | `FUN_00580f30` | `0x00669cb8` | 0 | None (sort 402) |
| `0x261` | `FUN_00581010` | `0x00669d10` | 1 | Commander (409) |
| `0x262` | `FUN_005810d0` | `0x00669d68` | 2 | Admiral (406) |
| `0x263` | `FUN_00581190` | `0x00669dc0` | 3 | General (408) |

The four vtables share their slots except `+0x0c` (the kind getter). The
other shared slots:
- `+0x14` (`FUN_00580d80`) and `+0x18` (`FUN_00580db0`) refuse a selection
  of more than one object;
- `+0x18` asks the object's validation, the character's slot `+0x2d4`
  (`FUN_004ed950`), which runs slot `+0x2d0`, `FUN_004ed750`.

**`FUN_004ed750`: when the order is allowed.** On top of the base check
(`FUN_004f9860`), the order is refused when the character is:
- on a mission (`+0x78` bit 7), error `0x30`/1;
- injured (`+0x94 != 0`), error 3;
- captured (`+0xac` bit 0), error 2.

The kind must also be allowed, else error 4:
- None: only while the character has a command (`+0x96 != 0`);
- Commander: `FUN_004ed1e0`;
- Admiral: `FUN_004ed1c0`;
- General: `FUN_004ed200`.

These are the class capability bits that `FUN_00401d20` also reads (class
record `+0xa8`, `+0xac`, `+0xb0`). The port loads them as
`Character::can_be_admiral`, `can_be_commander` and `can_be_general`.

**`FUN_00561bb0`: the order's effect.** It has no caller by name; it is
presumably reached through the order's event (slot `+0x48`, `FUN_00580e70`,
not read). It acts only on a character (family `0x30..0x3b`):
1. The kind comes from the order (`FUN_0053ebb0`). Giving the character the
   kind it already holds clears it instead (`kind := 0`).
2. `FUN_0055ed30` finds where the character stands:
   - aboard a capital ship (`0x14..0x1b`), that ship's fleet (`0x08..0x0f`)
     and the fleet's system (`0x90..0x97`);
   - otherwise a system directly.
3. **One holder per kind.**
   - In a fleet, `FUN_004fd790` walks the fleet's ships (mode 3, the
     character's side) and their characters (`FUN_005006f0`) for one
     already holding the kind.
   - On a planet, `FUN_00509330` walks the system's characters (mode 3, the
     side) for one.
   - A holder other than this character loses it (`FUN_004ee350(0)`).
4. Slot `+0x2f4` (`FUN_004ef340`) re-validates (slot `+0x2d4`) and runs slot
   `+0x2f8` (`FUN_004ef400`):
   - the same kind again clears it;
   - otherwise it sets the kind and, with a kind, `SetCharacterCommandingKey`
     (`+0x68`, the commanded object).

## Losing the command

`FUN_004eeed0` clears a command (`FUN_004ee350(0)`) unless all of these
hold:
- the character is uninjured (`+0x94 == 0`);
- it is not captured (`+0xac` bit 0 clear);
- `+0x50` bit 6 is set;
- its commanding key `+0x68` is of family `0x50..0x7f`.

The character's hooks run it:

| Slot | Function | On |
|---|---|---|
| `+0x32c` | `FUN_004f1720` | injury change (`character-injury.md`); also sets `Damaged` (`+0x50` bit `0x200`) |
| `+0x34c` | `FUN_004f1a80` | captured change (`captive-escape.md`) |
| `+0x11c` | `FUN_004f1340` | `Role_vt11c` (meaning not read) |
| `+0x268` | `FUN_004f1490` | `Role_vt268` (meaning not read) |

`FUN_00560d50`, from `FUN_00514a60`, clears the command of every character
aboard a capital ship (`FUN_004f25c0(ship, 3)`). When `FUN_00514a60` runs is
not read.

## Who reads it

- The name prefix (`FUN_004f17f0`).
- The decoy roll: a detector's backing officer rank, slot `+0x1bc`
  (`decoy-roll.md`).
- The bombardment multipliers for an admiral or general
  (`bombardment-order.md`).
- The AI's records: rank bits `0x4000000` / `0x1000000` / `0x2000000`
  (`FUN_00402230`, `ai-mission-planning.md`).
  - The port reads them as a `hyp:`: rank 1 for any character in a fleet,
    `mission_planning.rs`.

## The AI

Module `0xc`'s movement planners (`FUN_0042efe0`) answer slot `+0x1c` with a
command order:

| Type | Constructor | Vtable | Order |
|---|---|---|---|
| 3 | `FUN_0047a3c0` | `0x0065a460` | `0x262`, Admiral |
| 4 | `FUN_0047a000` | `0x0065a430` | `0x263`, General |
| 5 | `FUN_00479f70` | `0x0065a400` | `0x261`, Commander |

The AI's commands are therefore module `0xc`'s, a separate tracker item
("Moduł 0xc (ruchy flot)"), not yet ported.

## The port

Stage 2 (2026-10-19, save v85), `rebellion-core/src/character_command.rs`:
- `Character::command`;
- `order_allowed` (`FUN_004ed750`);
- `give` (`FUN_00561bb0`, `FUN_004ef400`);
- `keeps` (`FUN_004eeed0`).

`Character::injure` clears the command, as the injury hook does, and both
day loops call `release_unfit` for captures.

port:
- the base check `FUN_004f9860` is read as the character being alive;
- `FUN_004eeed0`'s `+0x50` bit 6 and commanding key tests are read as true
  while the character commands (`hyp:`, "Still open");
- a capture clears the command at the day's pass, not in its hook;
- `Damaged` (`+0x50` bit `0x200`) is not kept: it equals "injury non-zero",
  and no character reader of it is known;
- `FUN_00560d50` waits for `FUN_00514a60`'s trigger.

Not yet:
- the menu still shows "Command" without its children or a handler
  (`object_menu.rs`), so nobody gets a command yet;
- the readers use no rank: `bombardment-order.md` port note, and the AI
  records' `hyp:`.

## Plan (tracker item "Dowodzenie i jego utrata przy ranie")

1. **This note.**
2. **The field and the order.**
   - Add `CommandKind` (a save format change).
   - Add orders `0x260..0x263` with `FUN_004ed750`'s checks and
     `FUN_00561bb0`'s one holder per fleet or system.
   - Lose the command through `FUN_004eeed0` on injury and capture, and on
     the two other hooks once read. Set `Damaged` on injury.
   - Clear the commands aboard a ship on `FUN_00560d50`.
3. **The interface.**
   - The Command submenu with None, Admiral, General and Commander,
     enabled per `FUN_004ed750`, with the held kind checked.
   - The name prefix (`FUN_004f17f0`) in the windows that show it.
   - Browser acceptance.
4. **The readers.**
   - The decoy roll's backing rank.
   - The bombardment multipliers.
   - The AI records' rank bits, replacing the `hyp:`. This also closes stage
     5 of AI module 4.

The AI giving commands waits for module `0xc`.

## Still open

- What sets `+0x50` bit 6, and the family `0x50..0x7f` of the commanding key
  `+0x68`. `SetCharacterCommandingKey` is not read yet.
- `FUN_00514a60`'s trigger for `FUN_00560d50`.
- `Role_vt11c` and `Role_vt268`, the other two hooks that run
  `FUN_004eeed0`.
- The order event's path to `FUN_00561bb0`.
