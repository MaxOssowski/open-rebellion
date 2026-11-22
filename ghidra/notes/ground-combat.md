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

## Entry Point

`FUN_00560d50` (232 lines) — called from system battle orchestrator (`FUN_00514a60`).

## Entity Classification

Ground combat classifies entities by DatId family byte (`id >> 24`):

> Corrected 2026-11-22: this table shifted the labels. From the shipped DAT
> files `0x14-0x1b` is capital ships, `0x30-0x3b` characters, `0x08-0x0f`
> fleets, `0x10-0x13` regiments (`entity-system.md`). The flow below was
> written with the old labels and is not re-traced.

| Family Range | Type (old label in brackets) |
|-------------|------|
| `0x14-0x1b` | Capital ships; `0x18` the Death Star (was: troops / special forces) |
| `0x30-0x3b` | Characters (was: capital ships) |
| `0x34` | A major character (was: Death Star handler `FUN_005617b0`) |
| `0x08-0x0f` | Fleets (was: characters) |

## Flow

1. Check if entity is a ship (family 0x30-0x3b) → delegate to space path
2. Check if entity is a troop (family 0x14-0x1b) → enter ground combat
3. Get parent object via `FUN_00504f70`
4. Validate parent is also a troop (family 0x14-0x1b)
5. If family == 0x34: call **Death Star handler** `FUN_005617b0`
6. Iterate all troops at system via `FUN_004f25c0` (type=3)
7. For each troop: check regiment strength at `offset +0x96` (short)
8. If strength > 0: call `FUN_004ee350` (per-unit combat resolution)
9. Validate both sides are valid characters (family 0x08-0x0f)
10. Check same-side (`param_2 == param_1` → skip, no friendly fire)

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
