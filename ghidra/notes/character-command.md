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
     (`FUN_004ee740`, `+0xa8`, the commanded object).

## Losing the command

`FUN_004eeed0` clears a command (`FUN_004ee350(0)`) unless all of these
hold (read again 2026-10-19 from the image, `0x004eeed0..0x004eef64`):
- the character is uninjured (`+0x94 == 0`);
- it is not captured (`+0xac` bit 0 clear);
- it exists (`+0x50` bit 6, GameObjExisting);
- its container `+0x68` is not a mission (family outside `0x50..0x7f`;
  `status-window.md`, `personnel-finder.md` "On Mission").

The first reading took the last test the other way round, and called
`+0x68` the commanding key. The commanding key is `+0xa8`
(`Character::SetCharacterCommandingKey`, `FUN_004ee740`).

The character's hooks run it:

| Slot | Function | On |
|---|---|---|
| `+0x32c` | `FUN_004f1720` | injury change (`character-injury.md`); also sets `Damaged` (`+0x50` bit `0x200`) |
| `+0x34c` | `FUN_004f1a80` | captured change (`captive-escape.md`) |
| `+0x11c` | `FUN_004f1340` | destruction: `FUN_004f7560` sets `+0x50` bit `0x8` and calls the slot; the base `Role_vt11c` also asks for removal from the mission |
| `+0x268` | `FUN_004f1490` | the mission change: the base `Role_vt268` sets the role's parent at mission completion (`SetRoleParentAtMissionCompletionKey`); then `FUN_004ef070`, `FUN_004eeed0`, `FUN_004ef0c0` |

`FUN_00560d50(object, from, to)`, from `FUN_00514a60` (an object's move;
`FUN_0057ba50` passes its event's `+0x3c`, `+0x4c`, `+0x50`), takes the
command away:
- a capital ship (`0x14..0x1b`) that moves clears the command of every
  character aboard (`FUN_004f25c0(ship, 3)`);
- a character (`0x30..0x3b`) holding a command that moves loses it, except
  from one capital ship to another of the same fleet (`FUN_00504dc0` of
  each, both of family `0x08..0x0f` and equal);
- a character of family `0x34` (the Emperor) also runs `FUN_005617b0`, the
  seat of power test (`CharacterMgr_SetSeatOfPower`: free, at Coruscant
  `0x90000109`, held by the Empire), not a command rule.
Ported in stage 5 (`Character::move_to`).

## Who reads it

- The name prefix (`FUN_004f17f0`).
- The decoy roll: a detector's backing officer rank, slot `+0x1bc`
  (`decoy-roll.md`).
- The bombardment multipliers for an admiral or general
  (`bombardment-order.md`).
- The Planetary Assault duel's two generals (`FUN_0058bec0`,
  `planetary-assault.md`).
- The AI's records: rank bits `0x4000000` / `0x1000000` / `0x2000000`
  for ranks 1, 2, 3 (`FUN_00402230`, `ai-mission-planning.md`). Most
  mission queries forbid `0x7000000`, keeping an officer off missions.

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
- `FUN_004eeed0`'s existence test is the port's "not killed"; its mission
  test reads the port's mission flags (stage 6);
- a capture clears the command at the day's pass, not in its hook;
- `Damaged` (`+0x50` bit `0x200`) is not kept: it equals "injury non-zero",
  and no character reader of it is known;
- `FUN_00560d50` is ported in stage 5.

Stage 3 (2026-10-19), the interface:
- The Command submenu (`object_menu.rs`) lists the STRATEGY records
  `0x260..0x263`: parent `0x160`, sort keys 402, 409, 406, 408, TEXTSTRA
  12354 None, 12356 Commander, 12358 Admiral, 12360 General, word 10
  bitmap 11902.
  - Each row is enabled by `order_allowed`. No row is checked (stage 7).
  - The choice runs `give` (`PanelAction::OrderCommand`).
- `order_allowed` adds the base check `FUN_004f9860`: the ordering side's,
  active (recruited), not destroyed; the en route test came in stage 7.
- The Fleet Status window's Admiral, General and Commander rows
  (`status_rows.rs`) name the fleet's side's holder aboard
  (`FUN_004fd790`).

Stage 3b (2026-10-19, save v86), the name prefix (`FUN_004f17f0`):
- A command renames the character through `FUN_004f6e60` to TEXTSTRA of
  its name id (class record `+0x30`) plus `0x6800`, `0x6c00` or `0x7000`.
  None renames it back to the plain name.
- The loader keeps the four names on the character
  (`Character::command_names`), and `Character::set_command` renames it.
  `give`, the injury and `release_unfit` all set the command through it.
- port: a character without a TEXTSTRA name for the kind keeps its name.

Stage 4a (2026-10-19), the leadership readers:
- `fleet_holder` (`FUN_004fd790`): the fleet's side's holder aboard;
  `system_holder` (`FUN_00509330`): the system's side's holder on the
  planet, none for a neutral or contested system; `leadership`: the
  holder's slot `+0x1f4` (`+0x88`), else 0.
- Bombardment (`bombardment.rs`): the batteries' general and each fleet's
  admiral (`FUN_0055e930`, `FUN_0055e950`).
- Planetary Assault (`assault.rs`): the system's general and the
  attacking regiment's fleet's general (`FUN_0055eb00`; the push order at
  `0x0058c01c..0x0058c029` gives `(attack, fleet general, defense, system
  general)`).
- The Fleet Status rows use `fleet_holder`.

Stage 4b (2026-10-19), the detector's officer (`mission_detection.rs`):
- the rank by the detector's kind (slot `+0x1bc`): a capital ship's
  Admiral, a squadron's Commander, a regiment's General;
- the decoy roll and FOILTB subtract the officer's espionage times G3588
  or G3589 (35) over 100;
- an exposed member rolls RLEVADTB against the officer's effective combat
  and is captured by the officer's side.

Stage 4c (2026-10-19), the AI records (`mission_planning.rs`): the rank
bits follow `Character::command` (`OFFICER_RANKS`), replacing the `hyp:`
that gave every character in a fleet rank 1. The 24 dual-AI campaigns are
unchanged: in them the hyp's bit changed no pick.

The AI giving commands waits for module `0xc`; until then no AI character
holds one.

Stage 5 (2026-10-19), the move (`FUN_00560d50`): `Character::move_to`
drops the command when the character's fleet or system changes. The real
moves go through it: `missions::move_member` (missions and the AI's
character moves), the abode move, `fleet_join::merge_fleet_into` (each
moved ship takes the commands aboard), the fleet panel's assign and
remove, and the story's character transfer. port: characters ride the
fleet, not a ship, so a split moves none and a ship-to-ship move inside a
fleet does not occur. A capture is the day pass's `release_unfit`.

Stage 6 (2026-10-19), the hooks:
- `keeps` adds `FUN_004eeed0`'s mission test: a character on a mission
  (the port's `on_mission`, `on_hidden_mission`, `on_mandatory_mission`)
  loses its command at the day pass (`release_unfit`), as the mission
  hook (`+0x268`) does when it joins;
- `Character::mark_killed` clears the command, as the destruction hook
  (`+0x11c`) does.

Stage 7 (2026-10-19), closing:
- `FUN_004f9860` refuses an order for an object en route (`+0x50` bit 4,
  error 4, after the destroyed test). `order_allowed` and `give` take
  `en_route`, the port's `MissionState::is_en_route`
  (`Refusal::EnRoute`).
- No Command row is checked. `FUN_0051d990` builds a fresh order for each
  item: slot `+0x24` (`FUN_0051fb70`) puts the selection in the team at
  `+0x2c`, slot `+0x2c` the target, and the list at `+0x40` stays empty.
  Slot `+0x14` (`FUN_00580d80`, read from the image) returns
  `FUN_0051fd30`, 0 when there is more than one selected object.
  `FUN_0051fd30` (`0x0051fd30..0x0051fe14`; the decompile drops two
  blocks) walks that list with `FUN_00553720`: some item answers yes (`a`)
  and some no (`b`). With slot `+0x54` (`FUN_0040f340`, 1) it is checked
  when `a` and not `b`, so an empty list is never checked. This replaces
  the stage-3 `hyp:` that checked the held kind. The port's menu therefore
  has no checked row, and `CHECK_BITMAP` is gone.

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
     enabled per `FUN_004ed750` (none is checked, stage 7).
   - The name prefix (`FUN_004f17f0`) in the windows that show it (3b).
   - Browser acceptance.
4. **The readers.**
   - 4a: the bombardment multipliers and the assault's generals.
   - 4b: the decoy roll's backing rank, FOILTB and the exposure's captor.
   - 4c: the AI records' rank bits, replacing the `hyp:`. This also closes
     stage 5 of AI module 4.
5. **The move** (`FUN_00560d50`).
6. **The hooks.** `FUN_004eeed0`'s mission test in `keeps`, and the two
   other hooks that run it (`FUN_004f1340`, `Role_vt11c`; `FUN_004f1490`,
   `Role_vt268`).
7. **Closing.** `FUN_004f9860`'s en route test; the menu's checked row.

The AI giving commands waits for module `0xc`.

## Still open

- Which events reach `FUN_0057ba50` (the move event), and the order
  event's path to `FUN_00561bb0`. The port runs both rules where it makes
  the move or gives the order, so the paths change no behaviour.
