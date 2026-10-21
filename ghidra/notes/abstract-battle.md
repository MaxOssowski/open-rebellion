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
4. Field damage: a unit held by a field (`FUN_005d2190`) takes
   `FUN_005cb650 × dt` (`FUN_005b54d0`), and dies at hull 0.
5. Retreat: when slot `+0x2c` agrees, each unit escapes if it has a
   working hyperdrive (`FUN_005af910`, state 9). Otherwise it is
   destroyed (`FUN_005af7a0`), unless `FUN_005afe40` holds it.
   `+0x84` = 5 then marks the retreat as done.
6. Every unit of the side updates (`FUN_005b6320(dt)`): moving, firing
   and recharging.
7. The step counts the units in state 2. A side with none is beaten
   (return 1). The exception is side 1 while the Death Star stands.

## The abstract step (`FUN_005ae9c0`)

`dt` is `DAT_0066badc` = 1.0 (`FUN_0059a530`). Displayed play passes
frame milliseconds × 1000 (`DAT_0066bae0`) to `FUN_0059a6a0`, and the
game's own step gets × 0.4 (`DAT_0066c230`, `FUN_005aacb0`).

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

## `FUN_005af7a0` destroys; it does not withdraw

It sets the unit's state `+0x170` to `0xb` and deals 100,000 damage
(`0x47c35000`, `FUN_005b54d0`). It does the same to every unit the unit
carries (`+0x434`, `+0x438`). `FUN_005af910` is the escape (state 9).
`battle-flow.md` called the first one a withdrawal. That was wrong: a
stalled weaker side that cannot leave is destroyed.

## What the port lacks (2026-10-21 probe)

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
