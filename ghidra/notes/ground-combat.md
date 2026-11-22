---
title: "Ground Combat Resolution"
description: "Decompiled ground combat entry point and troop iteration logic (FUN_00560d50)"
category: "ghidra"
created: 2026-03-13
updated: 2026-03-16
---

# Ground Combat Resolution

> Superseded 2026-10-16 by `planetary-assault.md`: ground battles happen
> only in the Planetary Assault event (`FUN_00563390`, `FUN_0058c120`), and
> this note's family ranges are wrong (`0x14..0x1b` are capital ships;
> regiments are `0x10..0x13`).

## FUN_00560d50 is not ground combat (re-traced 2026-11-22)

`FUN_00560d50(object, old, new, ctx)` is called only from `FUN_00514a60`
(`Event320_Handle`, the 0x320 event when an object changes container), after
`FUN_005457f0`. With the DAT family bytes (`entity-system.md`):

1. An object that is neither a character (`0x30..0x3b`) nor a capital ship
   (`0x14..0x1b`) returns 1 at once.
2. A capital ship: `FUN_00504f70` gives its object; when that is a capital
   ship, `FUN_004f25c0(ship, 3)` walks its contents and every item whose
   `+0x96` is not 0 goes through `FUN_004ee350(0, ctx)`.
3. A character: `FUN_00505190` gives its object. The Emperor (family `0x34`,
   MJCHARSD `0x34000280`) calls `FUN_005617b0`,
   `CharacterMgr_SetSeatOfPower`: the seat of power is on when he is not
   captive (`+0xac` bit 0 clear), `+0x50` bit 0 is set, and he is at
   `0x90000109` (Coruscant) while the Empire holds it (`+0x24 & 0xc0 ==
   0x80`); otherwise off. A character whose `+0x96` is not 0 then goes
   through `FUN_004ee350(0, ctx)`, except when the old and new containers
   are both capital ships (`0x14..0x1b`) of the same fleet (`0x08..0x0f`):
   a move between ships of one fleet changes nothing.

The earlier reading here (troops, a Death Star handler, "no friendly fire")
came from shifted family labels.

## Per-Unit Combat: FUN_004ee350 (30 lines)

```c
int combat_resolve(void* unit, int param_1, void* battle_context) {
    if (!is_valid(unit)) return 0;
    if (!validate(param_1, 0, param_1)) return 0;

    short old_strength = unit->strength;  // offset +0x96
    if (old_strength != param_1) {
        unit->strength = (short)param_1;  // update strength
        notify_side(unit, 1, battle_context);  // notify attacker side
        notify_side(unit, 2, battle_context);  // notify defender side
        vtable_dispatch(unit, old_strength, unit->strength, battle_context);  // +0x330
    }
    return result;
}
```

The vtable call at `+0x330` fires the damage notification (HullValueDamage, SquadSizeDamage, etc.).

## Regiment Strength (offset +0x96)

- Type: `short` (16-bit signed integer)
- Zero means the unit is destroyed/non-combat
- Changed by `FUN_004ee350` which sets the new value and notifies observers
- Matches the CapitalShipEntry/TroopEntry record layout

## Death Star Handler: FUN_005617b0 (68 lines)

Called when family_id == 0x34. Likely handles:
- Death Star planet destruction
- Special victory/defeat conditions
- Unique combat mechanics (superlaser)

## Decompiled Files

- `FUN_00560d50_ground_combat.c` — main ground combat (232 lines)
- `FUN_004ee350_unit_combat.c` — per-unit resolution (30 lines)
- `FUN_005617b0_death_star.c` — Death Star handler (68 lines)
