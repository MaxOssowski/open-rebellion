# The seat of power's leadership bonus (`+0x92`)

Read 2026-10-19 from the saved decompile corpus (`decompiled-typed`) and
REBEXE.EXE. F-038; the parity tracker's item "Premia przywództwa
(pole +0x92)".

## The field

Character short `+0x92` is a percent bonus to leadership.

- The setter is `FUN_004ee250(value, ctx)`:
  - it clamps the value to `0..=0x7fff` (`FUN_0053e0f0`);
  - it stores it and syncs both sides' views (`FUN_004f03d0`);
  - it calls slot `+0x328` (`Character_vt328`, `FUN_004f1710`), which runs
    `FUN_004eede0`.
- `FUN_004eede0` recomputes the effective leadership `+0x88` (slot
  `+0x1f4`):
  - leadership `+0x64` plus leadership times `+0x92` over 100
    (`FUN_0053e940`, `FUN_0053e190` truncating);
  - the setter `FUN_004edfb0` clamps it to `0..=0x7fff`.
- Slot `+0x234` (`Character_vt234`, `FUN_004f1640`) and `FUN_004f11f0` also
  run `FUN_004eede0` when the base skills change.

## Who writes it

`FUN_004ee250` has one caller, `FUN_005618d0(character manager, character,
ctx)`. The value it writes is:
- GNPRTB 3076 (`Gnprtb_3076`; GNPRTB.DAT 50 at every level), when the
  character manager's `+0x58` mask 1 is set and `FUN_004edc80` returns 2;
- otherwise 0.

The conditions are:
- `+0x58` mask 1 is `SeatOfPower` (`CharacterMgr_SetSeatOfPower`,
  `FUN_0055f650`, which calls `FUN_0053a640(1, ...)`; `FUN_0053a610`
  takes a mask). Mask 2 is `PickupInProgress` (`FUN_0055f6c0`).
- `FUN_004edc80` reads the character's class record (`+0x2c`) side flags
  `+0x40` and `+0x44`:
  - 1 when only `+0x40` (the Alliance's) is set;
  - 2 when only `+0x44` (the Empire's) is set;
  - else 0.

`FUN_005618d0` runs:
- for every character from the character manager's slot `+0x1d8`
  (`CharacterMgr_vt1d8`, `FUN_00562400`), which `FUN_0055f650` calls when
  `SeatOfPower` changes;
- for a character the manager places (`FUN_00561470`, families
  `0x30..0x3b`).

## The manual

"The Seat of Power Effect": "If the Emperor is on Coruscant (not on a fleet
at Coruscant) and is not captured and Coruscant is under Empire control,
then all Imperial characters in the galaxy get a leadership bonus." The
trace agrees. The bonus is 50 percent.

## Who reads `+0x88`

Slot `+0x1f4` is read by:
- the missions (Recruitment, Incite and Subdue Uprising; `FUN_00520cd0`);
- the uprising terms;
- the AI's agent records;
- the bombardment's general and admiral and the assault's generals
  (`character-command.md`);
- the Status window's "Leadership Rating:" (`status-window.md`).

## The port (2026-10-19)

- `seat_of_power::leadership_bonus` is `FUN_005618d0`'s value, and
  `seat_of_power::effective_leadership` is `FUN_004eede0`.
- `missions::member_skill` (leadership) and `character_command::leadership`
  read it, and so does the Status window's row.
- The Status window had shown the base leadership `+0x64`.

port:
- The original stores `+0x92` when `SeatOfPower` changes or a character is
  placed. The port reads it from the world as it stands, with no save
  field. It reads the seat (`seat_of_power::seat_of_power`) when asked,
  where the stored bit (`SeatOfPowerState`) follows the day's check.
- The class sides are the port's `Character::is_alliance` and `is_empire`.
  A defection rewrites them, where the class record would not change.
