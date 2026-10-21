---
title: "The abstract battle: the battle object, its two side commanders, and the stalemate"
description: "How FUN_005ae9c0 steps an abstract battle: the side commander objects (vtables 0x66c428/0x66c3f8), their step FUN_005d14d0, strength FUN_005d0ce0/FUN_005b7a80, the 1200 stalemate that destroys the weaker side, and what the port lacks to run it"
category: ghidra
created: 2026-10-21
updated: 2026-10-21
tags: [combat, battle, tactical, auto-resolve, abstract]
---

# The abstract battle

Read 2026-10-21 from the typed corpus and the disassembly, for F-033.
It continues `battle-flow.md`, "Abstract mode in the tactical module".

## The battle object

- `FUN_00596f30` returns the battle singleton `DAT_006bc4a8`, built by
  `FUN_005af5f0` (0x274 bytes). Both modes use it, and the side code
  reaches it through `FUN_00596f30`.
- `FUN_005af5f0` builds two side commanders. Both are derived from
  `FUN_005c7cb0`, and both are 0xb4 bytes.
  - `+0x24c`: vtable `0x66c428`. Its lane fields are `+0x50` = 1.0 and
    `+0x54` = -1.0.
  - `+0x250`: vtable `0x66c3f8`. Its lane fields are `+0x50` = -1.0 and
    `+0x54` = -1.0.
  - Both commanders start with `+0xb0` = 0 and `+0x48` = `+0x4c` = 0.
- The setup `FUN_005ae460` ends by running each side's strength (slot
  `+0x14`, argument 0). It stores the two strengths as the starting lows
  at `+0x264` and `+0x268` (`0x5ae95d`).

## The side commander's slots

The commander vtable at `0x66c428` holds these slots, from `+0x00`
through `+0x2c`:

`FUN_005affd0`, `FUN_005a5b60` (the side id), `FUN_005d1a70`,
`FUN_005d0f50`, `FUN_005d1920`, `FUN_005d0ce0`, `FUN_005d1410`,
`FUN_005d1240`, `FUN_005d1080`, `FUN_005d14c0`, `FUN_005d14d0`,
`FUN_005d19a0`.

| Slot | Function | Role |
|---|---|---|
| `+0x04` | `FUN_005a5b60` / `FUN_005a66f0` | The side id (1 or 0) |
| `+0x0c` | `FUN_005d0f50` | Can it leave? True if a retreat is ordered (`+0x80` set, `+0x84` not 5) and some unit has a working hyperdrive (`FUN_005b1b70`) |
| `+0x14` | `FUN_005d0ce0` | Its strength, stored at `+0x3c` (see below) |
| `+0x28` | `FUN_005d14d0` | One combat step of time `dt`; returns 1 when the side is beaten |
| `+0x2c` | `FUN_005d19a0` | Should it retreat now? Only if a retreat is ordered, and its strength over the enemy's (`FUN_005afa30`) is below `+0x24` or zero |

## Strength (`FUN_005d0ce0`, `FUN_005b7a80`)

- A side's strength `+0x3c` is a sum over its units, the lists `+0x58`
  and `+0x64`.
- A unit already outmatched by an enemy group targeting it is left out.
  The test is `FUN_005d2190`, `FUN_005cb650` against the unit's slot
  `+0x64`.
- Each counted unit adds `FUN_005b7a80`: its weapon firepower in the arc
  it bears.
  - For each weapon family of that arc (`0x74`-byte battery records at
    `+0x17c`), it multiplies the count by the strength and the target
    modifier.
  - A capital ship scales the sum by hull (`FUN_005b1770`).
  - A destroyed unit gives `_DAT_0066c4f0`.
- With the Death Star present and operational (`FUN_005ba420`), side 1
  adds `_DAT_0066d0f0` = 20,000,000. Side 0 adds the same while it still
  has a Death Star attack in hand (`FUN_005afcc0`, order 6).

## One step of a side (`FUN_005d14d0`, slot `+0x28`)

In order:

1. The Death Star.
   - Side 1 charges the superlaser (`FUN_005ba7f0(dt)`). At full charge
     (100, `_DAT_0066d0f4`), it fires at the first capital ship in the
     object list (`FUN_005ba710`) and destroys it (`FUN_005af7a0`).
   - Side 0 may start the attack on the Death Star (`FUN_005aefc0`) if
     the counts compare (`FUN_005c5eb0`).
2. New arrivals in the battle's `+0x58` list become active: state
   `+0x170` = 2 (`FUN_005c5f20`).
3. Slot `+0x08`, then `FUN_005c9200(dt)`: the task-force and order
   management. This is what gives units their orders.
4. The fire. Each task force in the commander's list `+0x70` with a
   target (`FUN_005d2190`) deals that target `FUN_005cb650 × dt` through
   `FUN_005b54d0`. A target left with hull 0 (`+0x3cc`) is destroyed
   (`FUN_005af7a0`). `FUN_005cb650` sums `FUN_005b7780` over the task
   force's members in state 2, 4 or 6. This is all the abstract battle's
   combat: no unit moves, and range plays no part.
5. Retreat: when slot `+0x2c` agrees, each unit escapes if it has a
   working hyperdrive (`FUN_005af910`, state 9). Otherwise it is
   destroyed (`FUN_005af7a0`), unless `FUN_005afe40` holds it.
   `+0x84` = 5 then marks the retreat as done.
6. Every unit of the side recharges its shields and weapon energy
   (`FUN_005b6320(dt)`). That function does nothing else.
7. The step counts the units in state 2. A side with none is beaten
   (return 1). The exception is side 1 while the Death Star stands.

## The abstract step (`FUN_005ae9c0`)

`dt` is `DAT_0066badc` = 1.0 (`FUN_0059a530`). It is one second.
Displayed play turns frame seconds into milliseconds (× 1000,
`DAT_0066bae0`), and its unit update hands `FUN_005b6320` those
milliseconds × 0.001 (`DAT_0066c26c`, `FUN_005a9de0`). The rates in
`FUN_005b6320` and `FUN_005b7780` are therefore per second. The 1200
stalemate is 1200 seconds of stepping.

1. Elapsed time `+0x00` += dt.
2. Both sides take their strength. Each side takes its step: side
   `+0x250` first, then `+0x24c`. Both strengths are taken again.
3. Stall clocks: a strength below its low stores the new low and resets
   the side's clock (`+0x26c` / `+0x270`). Otherwise the clock grows by
   dt.
4. Neither side beaten, and the smaller clock ≥ 1200 (`DAT_0066c3e0`):
   - the side with the lower strength, or both on a tie, ends;
   - an operational Death Star makes side `+0x24c`'s strength -1;
   - a side that can leave (slot `+0xc`) is marked `+0xb0` = 1;
   - otherwise each of its units is destroyed (`FUN_005af7a0`).
5. A side beaten: the step fills the seven result words (`+0x10` to
   `+0x28`) and returns 1.
   - Death Star flags come from `+0x84` == 5, `+0x78` and `FUN_005ba420`.
   - A side is marked as withdrawn when one of its units is in state 9
     (escaped), by that unit's side `+0x168`.
   - A side is marked as holding when none of its units is in state 2.

## A unit's damage rate (`FUN_005b7780`)

- Slot `+0x84` (`FUN_005b4c60` for both ships and fighters) picks the arc
  with the most firepower against the target's type. A tie keeps the
  first arc, and an arc with no weapons gives 4, meaning no fire.
- Slot `+0x20` gives the type: 0 for a capital ship (`FUN_006158b0`), 1
  for a fighter group (`FUN_0040f340`).
- Each weapon family's record (in the arc's `0x74` bytes) holds a
  strength, a modifier against fighters and a modifier against capital
  ships. Ion cannons give nothing against fighters.
- Laser and turbolaser fire at fighters is multiplied by the source's
  maneuverability over the target's (slot `+0xbc`), at least 0.1
  (`_DAT_0066c518`).
- A fighter group's terms and its weapon count are scaled by its hull
  (`FUN_005b1770`). The count is then truncated.
- The rate is the sum over the families, divided by the arc's weapon
  count and multiplied by the weapon recharge per second (slot `+0x60`:
  `FUN_005b0400` for a ship, `FUN_005b9c00` for a fighter group).
- A fighter group firing at a capital ship whose shields are down
  (`FUN_005b1650`) adds its torpedoes (`FUN_005b23a0`) before the
  division.
- `FUN_005b7a80` is the same rate against a capital ship, without the
  torpedoes. It is a unit's share of its side's strength.

## Targets (`FUN_005c9200`, `FUN_005ec740`)

The commander rebuilds its target list `+0x90`: the fighter groups move to
a second list, and the capital ships are re-keyed after them.

Each task force that has no target (`FUN_005c91e0`) joins one of two
lists:

- if its first member carries laser cannons in any arc (`FUN_005b7ce0`),
  the list that takes fighter targets;
- otherwise, the list that takes capital-ship targets.

Strategy `0x66cbe8` slot `+0x04` (`FUN_005ec740`) pairs each list's task
forces with targets. It scores each pair through the policy at `+0x2c`
(`0x66cbf8`, `FUN_005ed170`) with a limit of 180 (`DAT_0066cbac`), then
links the pair with `FUN_005a8fc0`.

Read further on 2026-10-21, for the next stage:

- The target list `+0x90` is rebuilt by slot `+0x08` (`FUN_005d1a70`).
  - It holds the enemy commander's capital ships (`+0x58`) and fighters
    (`+0x64`). A unit counts if it is eligible (`FUN_005c91c0`) or in
    state 2.
  - The battle's arrivals in states 1 or 3 are added too.
  - Each is keyed by `-priority`, so the list runs from highest priority
    down.
- The priority is policy slot `+0x08` (`FUN_005ed8e0`):
  - 15 (`DAT_0066da7c`) × the unit's strength (`FUN_005b7a80`);
  - minus 0.01 (`DAT_0066daa0`) × each of `FUN_005b1710` and
    `FUN_005b1640`;
  - minus 0.01 (`DAT_0066da80`) in states 1 or 3;
  - plus 1.0 for each entry naming it in the game's lists `+0xe5c` and
    `+0xe50`, and 0.03 (`DAT_0066da88`) × `+0x28` for each matching entry
    of the game's list.
- `FUN_005c9200` then rebuilds the two lists:
  - the fighter list takes the fighters, then the capital ships;
  - `+0x90` keeps the capital ships, followed by that whole list.
  - Keys continue from each list's last key, in steps of 1
    (`DAT_0066cbdc` = -1, subtracted).
- The score (`FUN_005ed170`) of a target for a task force:
  - it takes the running rate `r`, which grows by the group's rate
    (`FUN_005cb650`), and the target's shield recharge `q` (slot `+0x64`);
  - the target's hit points are `hull × 0.65` (`DAT_0066da8c`) plus its
    shields, at least 1;
  - if `r - q > 0`, the score is `points / (r - q)`, plus 0.001
    (`DAT_0066da98`) × max(1, distance / speed of the task force's object
    `+0x54`, slot `+0x34`);
  - otherwise it is -1 (`DAT_0066da9c`).
- The pairing (`FUN_005ec740`), for each eligible target in list order:
  1. Score every unassigned task force alone. A score in 0..180 is its
     key. Otherwise the key is 100,000 (`DAT_0066da58`) minus its rate.
     The keys sort ascending: the head is the best (`FUN_005f5060`), the
     tail the worst (`FUN_005f5080`), in a tree with comparator
     `FUN_005ed0e0`.
  2. With one task force left, link it (`FUN_005a8fc0`) and stop.
  3. Otherwise start the running rate at the head's rate. Walk from the
     tail toward the head, adding each task force. When the combined
     score falls in 0..180, link the tail through the current one. The
     head itself is not linked here.
  4. Every task force still unassigned takes the first eligible target,
     or else the first in the list.

## `FUN_005af7a0` destroys; it does not withdraw

It sets the unit's state `+0x170` to `0xb` and deals 100,000 damage
(`0x47c35000`, `FUN_005b54d0`). It does the same to every unit the unit
carries (`+0x434`, `+0x438`). `FUN_005af910` is the escape (state 9).
`battle-flow.md` called the first one a withdrawal. That was wrong: a
stalled weaker side that cannot leave is destroyed.

## The port (2026-10-21)

`rebellion-core::abstract_battle` steps a `BattleSession` this way:

- the 1.0 step;
- per-task-force fire with `FUN_005b7780`;
- the recharge;
- the beaten test;
- strength with the out-damaged exclusion;
- the 1200-second stall that destroys the weaker side, or both on a tie.

It is not used by the campaign yet. Three pieces stay `hyp:`:

- Targeting: a task force keeps a live target, otherwise it takes the
  first enemy of the class its leader's lasers favour. `FUN_005ec740`
  scoring is next.
- A fighter group hit by `FUN_005b54d0`: shields first, then hull.
- The retreat decision and the Death Star's abstract actions. These are
  not ported.

The first probe over seeds 1–3 (67 battles) ended every battle with one
side destroyed, in 1 to 1008 steps (median 43), and none by stalemate.

## What the port lacked before (2026-10-21 probe)

The port's `BattleSession` (`rebellion-core::tactical`) has no side
commander. Its units move and attack only on a player's order, so no
equivalent of `FUN_005c9200` exists.

A probe ran it headless on every strategic battle of seeds 1–3: 67
battles, 1500 ticks, dual AI. None dealt damage in 40,000 steps. So the
port cannot yet resolve a battle this way. The missing pieces are:

1. the side commander's step (`FUN_005d14d0`) and its order management
   (`FUN_005c9200`, `FUN_005d1a70`, `FUN_005d1080`, `FUN_005d1240`);
2. the strength (`FUN_005d0ce0`, `FUN_005b7a80`) over the port's
   battery records;
3. this abstract step and its result words, read back as in
   `FUN_0040c780`.
