---
title: "Squadron strength: losses, repair and Squadron at Full Strength"
description: "How a fighter squadron loses craft in a battle (FUN_0040d260), regains one per repair fire of timer 0x380 (FUN_00503530, GNPRTB 4608/4609), and posts Squadron at Full Strength when its losses reach 0 (notification 0x106, FUN_00499de0)"
category: ghidra
created: 2026-11-16
updated: 2026-11-16
tags: [fighter, squadron, repair, timer, battle, messages, CMD-07-C011]
---

# Squadron strength (CMD-07-C011)

Read 2026-11-16 from the typed corpus (`decompiled-typed`), the vtable at
`0x0065def0` (read from `REBEXE.EXE`) and the disassembly. Ship repair is in
`ship-repair.md`; the message builder in `message-sounds.md`,
`message-pictures.md` and `story-message-classes.md`.

## The field

A `FightSquad` (families `0x1c..0x1f`, vtable `0x0065def0`) keeps its
**losses** at `+0x60`, the field Ghidra's type names `SquadSizeDamage` (the
same offset is a capital ship's `HullValueDamage`). It is a count of craft
lost, not of craft left (`cpp-class-hierarchy.md` called it
`squad_size_current`; that reading is wrong):

| Slot | Function | Value |
|---|---|---|
| `+0x244` | `0x500dd0` -> `FUN_00502f40` | the class's size, class `+0xc8` (FIGHTSD `squadron_size`) |
| `+0x248` | `0x500de0` -> `FUN_00502f60` | the class's bombardment value, class `+0xcc` |
| `+0x254` | `FUN_00503100` | `+0x60`, the losses |
| `+0x258` | `FUN_00503110` | `+0x248 x +0x254 / +0x244` (`FUN_0053e170` -> `FUN_0053e150`, truncating) |
| `+0x25c` | `FUN_00503340` | Damaged (`+0x50` bit `0x200`, `GameObj_SetDamaged`) = `+0x254 != 0` |
| `+0x260` | `FUN_005038b0` | on a change: `+0x25c` when the new value is within `+0x244` |
| `+0x264` | `FUN_005038e0` | `FightSquadSquadSizeDamageNotif`, notification `0x1a0` |
| `+0x1d8` | `FUN_00503140` | the repair period: GNPRTB 4608 with FastRepair, else 4609 |
| `+0x228` | `FUN_00503530` | a repair fire: when `+0x60 > 0`, `FUN_00503400(1)` |
| `+0xac` | `0x4f8660` | destroy |

- `FightSquad_SetSquadSizeDamage` (`FUN_005032c0`) checks the value is in
  `0..+0x244` (`FUN_0053fc90`, error `Invalid_SquadSizeDamage_value_`),
  writes `+0x60`, syncs both sides' copies (`FUN_005037f0`) and calls
  `+0x260`, which recomputes Damaged.
- `FUN_00503370(n, reason, ctx)` adds `n` losses, at most the size; when the
  losses reach the size (`+0x254 == +0x244`) it destroys the squadron
  (`+0xac` with the reason).
- `FUN_00503400(n, ctx)` removes `n` losses, at least 0.
- The current size is `+0x244 - +0x254` (`FUN_0040d730`).

## Losses in a battle

Both battle modes run the tactical module (`battle-flow.md`):

- **Build**: `FUN_0040be90` gives each squadron's tactical object
  (`FUN_005b9c60`) its current size (`FUN_0040d730`) and its size
  (`+0x244`). The object's hull is `current x 8.0` (`+0x3cc`) and its
  maximum `size x 8.0` (`+0x3c8`); the live craft count is the ceiling of
  hull / 8 (`tactical-fighter-combat.md`).
- **Result**: `FUN_0040c780` walks the system's fleets, their ships and
  each ship's squadrons (`FUN_005039d0`), then the planet's, and calls
  `FUN_0040d260` on each. That finds the squadron's result record
  (`FUN_004f6df0`, `FUN_005968e0`), whose word 6 is the craft that came back,
  and compares it with the current size: more adds back
  (`FUN_00503400(diff)`), fewer adds losses (`FUN_00503370(-diff, 0xc)`).
  A squadron with no craft left is destroyed with reason `0xc`.

So a squadron leaves a battle with exactly the craft its tactical object
kept, and enters the next one with them.

## Repair

A squadron's repair is the capital ship's timer `0x380` (`ship-repair.md`):

- UnderRepair = Damaged and Usable (`FUN_005578d0`, reached from slot
  `+0x134` `CombatUnit_vt134` when Damaged changes). Usable means existing,
  completed and not en route: in a fleet in orbit or on a planet, not while
  it travels.
- The period: GNPRTB 4608 (2) with FastRepair, else 4609 (30)
  (`FUN_00503140`). FastRepair (`FUN_0050c580`) needs the system's
  `+0x88` bit `0x400` (a usable, unsuspended yard, `FUN_0050bb70`) and the
  system's side equal to the squadron's. A change of FastRepair rebuilds the
  timer from that day (`FUN_00557c30`).
- Each fire (`FUN_00503530`): one craft back (`FUN_00503400(1)`).

There is no other replenish: no carrier or facility condition beyond the
FastRepair period. A squadron that lost `n` craft is whole after `n` fires:
`2n` days beside a working shipyard of its side, `30n` elsewhere.

## Squadron at Full Strength

When the losses reach 0, `+0x25c` turns Damaged off. `GameObj_SetDamaged`
(`FUN_004f7800`) syncs each side's copy (`FUN_00539fd0(1)`, `(2)`,
`GameObj_SyncDamaged` `FUN_004fa8f0`), whose slot `+0x19c`
(`GameObjDamagedNotif`, `FUN_004fc3a0`) posts `0x106` with the new value;
its own slot `+0x134` (`CombatUnit_vt134` `FUN_00557bd0`) posts `0x31d`
(`FUN_004fb9e0`) and recomputes UnderRepair (`FUN_005578d0`), which drops
the timer. The message class `0x65bb88` (`FUN_00499de0`) takes `0x106`
with the new value 0 for a unit of the reader's side:

- a capital ship (`0x14..0x1b`): Capital Ship Repaired, `+0x28` 8,
  TEXTSTRA `0x7041` / `0x7043`, sound `0x45e`;
- a squadron (`0x1c..0x1f`): Squadron at Full Strength, `+0x28` 9,
  `+0x30` `0x457 + (side != 1)`, TEXTSTRA `0x7040` "Squadron at Full
  Strength" / `0x7042` "The {1:1} Squadron attached to {2:1} has replaced
  all fighters lost in combat.": `{1}` the squadron's name, `{2}` its
  container's. Aboard a capital ship the container is the ship's fleet and
  `+0x34` is `0x80` (Fleet); otherwise (a planet) the system and `0x40`
  (Defense).
- The picture is the unit's own (`FUN_004c5000`, `message-pictures.md`).

## Port (2026-11-16)

- `FighterEntry::damage` (`SquadronDamage { lost, repair }`): a squadron
  with losses has an entry of its own (count 1). The battles report each
  surviving squadron's craft (`FighterLossEvent::craft_after`); the
  integrator and the tactical commit set the losses
  (`squadrons::apply_battle_craft`, `tactical_flow::commit_squadrons`).
- `BattleSession::new_for_fleets` builds each squadron with its current
  craft and its size's maximum hull.
- `RepairSystem::advance` runs the squadrons' timers
  (`RepairEvent::SquadronRepaired`); the app posts Squadron at Full Strength
  when one's losses reach 0 (`squadron_full_strength_message`).
- Moves, transits, sorties and fleet joins keep the losses; a transit drops
  the timer (not Usable).
- Save v126: `SaveState::squadron_damage`, the last field; older saves load
  every squadron whole.
- port: a squadron's name is its class's (the port's squadrons have none
  of their own); the planet's squadrons are named by class, so their
  Status window shows the first of the class.

## The AI's strength sums (2026-11-16)

The AI records weigh a squadron by its craft, not its class size:

- `FUN_00484630` (a fleet's ship record, `ai_records::ship_record`) and
  `FUN_00433e40` (a system's record, the planet's squadrons) pass
  `+0x244 - +0x254` (size less losses) to `FUN_00473610` / `620` / `630`.
- Those return the class record's `+0x30` / `+0x34` / `+0x38` x that
  count / the record's `+0x4c` (the squadron size, `values[7]`),
  truncating. So a squadron with one craft of two counts half.
- The fleet record sums them into `+0x90`, `+0x94`, `+0x84`, `+0x7c`,
  `+0x80`; the system record into `+0xb0`..`+0xb8`. The bombardment figure
  (`+0x40` into `+0x48`) and the counts are not scaled.
- `FUN_005031e0` (slot `+0x1f4`, the weakest-squadron pick in
  `ai_module7`) is the class size (`+0x244`) x the class's `+0xbc`
  (`FUN_005574f0`), not the craft left.

Port: `ai_records::at_strength` (tests
`a_squadron_below_full_strength_adds_its_craft_s_share_aboard`,
`a_planet_s_squadron_below_full_strength_counts_its_craft_s_share`).
