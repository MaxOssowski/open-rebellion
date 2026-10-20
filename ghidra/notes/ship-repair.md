---
title: "Ship and squadron repair: timer 0x380"
description: "How a damaged capital ship or fighter squadron repairs: the UnderRepair and FastRepair flags, timer 0x380's period, and what each fire restores"
category: ghidra
created: 2026-10-20
updated: 2026-10-20
tags: [repair, timer, capital-ship, fighter, shipyard]
---

# Ship and squadron repair (timer `0x380`)

Read 2026-10-20 from the typed corpus (`decompiled-typed`) and the
disassembly, for F-035. Timer ids and records are in `timer-scheduler.md`.

## UnderRepair

- `CombatUnit_SetUnderRepair` (`FUN_005577f0`) writes `Flags5c` bit 1 and
  calls slot `+0x22c` (`FUN_00557c00`). That slot arms timer `0x380` with
  the flag on the unit's record `+0x54`+`0x18` (`FUN_0053fa60`). Turning
  the flag off cancels it: the record changes, and the pending timer finds
  a different counter.
- `FUN_005578d0` computes the flag: UnderRepair = Damaged (`+0x50`) and
  Usable (`+0x50`; existing, completed, not en route). `FUN_00557b60` and
  `FUN_00557bd0` run it when those bits change.
- There is no shipyard condition. A damaged ship repairs anywhere it is
  usable, so not while it travels.

## FastRepair and the period

- `CombatUnit_SetFastRepair` (`FUN_00557860`) writes `Flags5c` bit 2 and
  calls slot `+0x230` (`FUN_00557c30`). That slot sets the record's
  minimum delay to slot `+0x1d8` (`FUN_0053fab0`). If the delay changed
  while the timer runs, the timer is rebuilt and counts again from now.
- Slot `+0x1d8`:
  - a capital ship (`FUN_005010a0`): GNPRTB 2052 (2) with FastRepair,
    else 2053 (30);
  - a squadron (`FUN_00503140`): GNPRTB 4608 (2) with FastRepair, else
    4609 (30).
- A unit's FastRepair (`FUN_0050c580`) needs:
  - its system's bit `0x400` (`+0x88`, CombatUnitFastRepair);
  - the system's side (`+0x24` bits 6..7) equal to the unit's.
- The system's bit (`FUN_0050bb70`): its manager (`FUN_00509670`) has at
  least one usable yard that is not suspended (`FUN_00528040(0)`, yard
  `+0x60` bit 0 clear).
- The delay spread is not set (hyp: 0).

## A fire

Each fire calls slot `+0x228` (`FUN_0057c2c0`).

- A capital ship (`FUN_00501c50`), when `HullValueDamage` > 0:
  - it repairs a tenth of its class's maximum hull (slot `+0x248` =
    `FUN_00500db0`, a jump to `FUN_00500550`, the class hull; then
    `FUN_00501b70`: damage -= hull / 10, at least 0);
  - then, by a draw (`FUN_0053e290`), it lowers one of its damaged
    subsystem nibbles (`+0x64`: shield, weapon, ...) by one.
- A squadron (`FUN_00503530`), when `SquadSizeDamage` > 0: one craft
  (`FUN_00503400(1)`).

A damaged capital ship is whole after ten fires at most: 20 days beside a
working shipyard of its side, 300 days elsewhere.

## The port before

`repair.rs` repairs only at a system with any shipyard, every tick, by
the class `damage_control`. That rule has no source.
