---
title: "Ship and squadron repair: timer 0x380"
description: "How a damaged capital ship or fighter squadron repairs: the UnderRepair and FastRepair flags, timer 0x380's period, and what each fire restores"
category: ghidra
created: 2026-10-20
updated: 2026-11-16
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

- A capital ship (`FUN_00501c50`, vtable `0x0065d650`, re-read 2026-11-16):
  - only when `HullValueDamage` > 0 it repairs a tenth of its class's
    maximum hull (slot `+0x248` = `FUN_00500db0`, a jump to
    `FUN_00500550`, the class hull; then `FUN_00501b70`: damage -= hull /
    10, at least 0). With no hull damage this step is skipped and counts
    as a success (`uVar6 = 1`);
  - the subsystem step runs either way: with `n` hits (`FUN_005006a0` > 0)
    a roll of 0..99 (`FUN_00558680` -> `FUN_0053e2f0` -> `FUN_0053e2e0`,
    `DAT_00661a88` = 100) must fall under slot `+0x260` less slot
    `+0x28c`. Slot `+0x260` is `0x500e10`, a jump to `FUN_005005d0`, the
    class's `+0xd8` (CAPSHPSD `damage_control`, DAT `0xb0`); slot
    `+0x28c` is `FUN_006158b0`, 0. On a pass it lowers one hit counter,
    each hit equally likely (`ship-subsystem-damage.md`). On a miss the
    fire mends nothing.
- A squadron (`FUN_00503530`), when `SquadSizeDamage` > 0: one craft
  (`FUN_00503400(1)`).

So a fire mends a ship's subsystems only with chance `damage_control`%,
hull damage or not. Hull damage is gone after ten fires at most; the hits
take a geometric number of fires.

## When FastRepair changes (re-read 2026-11-16)

Event-driven, not periodic:

- The system's bit `0x400` is set by `System_SetCombatUnitFastRepair`
  (`FUN_0050a820`), called only from `FUN_0050bb70`, which System slot
  `+0xc0` (`FUN_00508250`) and `FUN_00508660` run when they revalidate the
  system's derived bits (a facility added, removed or blocked). The
  manager is `FUN_00509670(0)` (`push 0` in the bytes: area 0, the
  shipyards); `FUN_00528040(0)` counts its yards with `+0x60` bit 0 clear
  (not blocked by uprising or blockade, `FUN_0050bbb0`).
- When the bit changes (`FUN_0053a640`), `FUN_0050a820` syncs both sides'
  copies and calls slot `+0x234` (`FUN_00511710`) -> `FUN_0050c350`,
  which walks the system's fleets, their ships and squadrons
  (`FUN_00502db0`, `FUN_005039d0`) and calls `FUN_0050c580` on each. Slot
  `+0xf0` (`FUN_00510820`) does the same after a side change.
- `FUN_0050c580` -> `CombatUnit_SetFastRepair` -> `FUN_00557c30`: a new
  period rebuilds the running timer from that day. So the 2-day period
  starts the day the yard is complete.
- A ship damaged before it was at the system: UnderRepair is set when
  Damaged or Usable changes (`FUN_00557b60`, `FUN_00557bd0` ->
  `FUN_005578d0`), and the timer is armed then; the timer is saved with
  the game, so a loaded game keeps its due day.

## A0: VD2 at Coruscant (OBJ-06-C012, 2026-11-16)

Original game, Empire slot 1 (day 508), `.artifacts/a0-vm/2026-11-16-empire-hunt`
(not committed; SHA256SUMS prefixes): h-3 `96f59062e745`, h-4
`eb95194af456`, h-28 `34d1e0218322`, h-29 `fd2acfbde37c`, h-79
`369951c9f24e`, h-80 `8d8cb6f009b8`, h-82 `f0d61d097189`, h-83
`4e6db844a752`, h-86 `c97fa087c2be`.

- VD2 (Victory Star Destroyer II, CAPSHPSD record 0x84: hull 1800,
  shield recharge 10, `damage_control` 20) shows Hull 1800:1800, Ship
  Damaged Yes, Shield Recharge Rate 8:10: one shield hit (10 - 10 x 1 /
  GNPRTB 2048 (4) = 8), no hull damage.
- Each fire mends it with chance 20%.
- No yard (2026-11-15 e2): 62 days at the 30-day period are two fires
  (three if the saved timer fell due by day 510): it stays damaged with
  0.8^2 = 64% (51%). Consistent.
- With the Orbital Shipyard (2-day period from its completion): five fires
  (10 days) on average. Run 9 repaired by day 591 and run 6 by day 634.
  Run 10: yard complete by day 584 (h-80; its Deployed row is already
  older than the newest in h-86), Damaged Yes through 603, No on 604:
  nine or ten missed fires, 0.8^9 = 13%. Consistent.
- Not settled: the draws themselves (the original's generator,
  `FUN_005f5700`) and the exact day the yard completed in run 10.

The port (`repair.rs`, D2/F-035) already does all of this: the subsystem
step without hull damage (`repair_hit`, test
`a_ship_with_only_subsystem_hits_is_repaired`), the roll under
`damage_control`, FastRepair re-read each day with the timer rebuilt on a
change (`a_new_period_rebuilds_the_timer_and_a_healthy_or_travelling_ship_has_none`).
No change was needed for the delay; only the draws differ (port:
`SplitMix64`).

## The port before

`repair.rs` repaired only at a system with any shipyard, every tick, by
the class `damage_control`. That rule had no source; F-035 replaced it
with timer `0x380` as above.
