---
title: "A battle from start to result: the strategy state machine and the tactical module"
description: "How the original resolves a strategic battle: FUN_0040a050's states, the tactical or abstract choice, the tactical module run in both cases, and FUN_0040c780 reading its result back"
category: ghidra
created: 2026-10-21
updated: 2026-10-21
tags: [combat, battle, tactical, auto-resolve, state-machine]
---

# A battle from start to result

Read 2026-10-21 from the typed corpus and the disassembly, for F-033
(tracker item "Wzory walki kosmicznej").

## Summary

The original has no separate strategic combat formula. A battle the
player fights in tactical mode and a battle resolved without him
("abstract") both run the tactical module. The strategy side reads the
module's result back into the galaxy. The port's `combat.rs`
auto-resolve is the port's own rule (`hyp:`), and its citations to a
"seven-phase space combat" are `MissionMgr` story steps (audit F-031).

## The state machine (`FUN_0040a050`)

The debug names are in `FUN_0040a700`'s table (`0x40a7b5`..), strings at
`0x6a7fe4`..`0x6a80c8`: `STRAT_ALL_BATTLE_START`, `STRAT_BATTLE_START`,
`STRAT_BATTLE_CHOOSE_TYPE`, `STRAT_BATTLE_WAIT_FOR_TYPE_CHOICE`,
`STRAT_BATTLE_CONVERT_TO_TACT`, `STRAT_BATTLE_CONVERT_TO_TACT_ABSTRACT`,
`STRAT_BATTLE_FINISH`, `STRAT_BATTLE_CHOOSE_DONE`,
`STRAT_BATTLE_WAIT_FOR_DONE_CHOICE`, `STRAT_ALL_BATTLE_FINISH`. The state is
`+8`.

| State | Action |
|---|---|
| 4 | a battle is pending (`FUN_0041e230`, `FUN_0041e1d0`); to 5, or `0xc` when none is left |
| 5 | notify the battle start for its location (`FUN_0041cfa0`) |
| 6 | ask the type: `FUN_0041cfc0` -> `FUN_004362b0`, each side's player object (`+0x10`, `+0x14`, slot `+0x3c`) chooses; then 7 (tactical) or 8 (abstract) by the answer |
| 7 | tactical: build the battle (`FUN_0040afa0`, `FUN_0040b0e0` with the two sides `+0xc`, `+0x10`), load and show it (`FUN_00409690` behind a wait cursor), switch to module 10 (`FUN_00596820` -> `FUN_00597530(10)`, `FUN_00599a70`); to `0x14` |
| 8 | abstract: the same build (`FUN_0040afa0`, `FUN_0040b0e0`), no display, module 10 (`FUN_00596820`); to `0x15` |
| `0x14` / `0x15` | run the module (`FUN_00597530(DAT_00658640)`, `FUN_0059a530`) until it reports done; to `0x16` / `0x17` |
| `0x16` / `0x17` | apply its result (`FUN_0040c730`, `FUN_0040c780` with `+0x14` = 0 or 1), close the module (`FUN_005975a0`), `FUN_0041e250`; to 9 (`0x16` may go to 8 again) |
| 9 | the battle's end at its location (`FUN_0041e270` -> `FUN_0051ddf0` -> `FUN_00516ef0`, `FUN_00517270`; `planet-squadrons.md`, "Battle") |
| `0xa`, `0xb` | the done choice (`FUN_0041cff0`, `FUN_0041d010`); back to 4 |
| `0xc` | all battles finished (`FUN_0041e320`, `FUN_0041db60`); to 2 |

## The result (`FUN_0040c780`)

- `FUN_005969d0` copies seven words from the tactical module
  (`FUN_0059a730`). Words 1 and 3 (and 0 and 2, a second pass) are each
  side's survivors.
- The winner (`local_10c`) is the only side with survivors. With both
  sides left, or neither, it is decided by the system's `+0x78` bits 6..7:
  3 is no winner (3), otherwise 1 or 2.
- `FUN_0050cfd0(1)` and `FUN_0050ceb0(winner)` store it on the battle
  object. `FUN_00516ef0` reads it (`FUN_00509ad0`, `+100`) to send the
  loser's units autorouting.
- It then walks the system's fleets (`FUN_004ffe70(system, 3)`), their
  capital ships (`FUN_00502db0`) and squadrons (`FUN_005039d0`), and
  applies the module's losses and damage to each.

## Abstract mode in the tactical module (read 2026-10-21, second pass)

- The mode flag travels as the battle object's `+0xc` (state 7 writes 0,
  state 8 writes 1; `0x40a4ca`).
- `FUN_0040b0e0` passes it as the last of the 17 arguments of
  `FUN_00595d60` (`0x40b30f`).
- That forwards it to the module's init, `FUN_00599c90` or
  `FUN_0059a0b0` by the first argument. The init stores it as the module
  object's `+0x668`.
- The module's run step `FUN_0059a530`:
  - with `+0x668 == 1` (abstract) it calls `FUN_005ae9c0` with a time
    step of `DAT_0066badc` = 1.0. When that reports the end, it copies
    the result (7 words at the abstract object's `+0x10`) into `+0x66c`;
  - otherwise (tactical) it runs the displayed game: frame time ×
    `DAT_0066bae0` (1000.0), `FUN_0059a6a0`, `FUN_005c1490` and
    `FUN_005c1470`. When the game reports done (`+0x8c4`), it copies the
    7 words at the game's `+0x9f0`.
  - `FUN_0059a730` returns `+0x66c` to `FUN_005969d0`.
- `FUN_005ae9c0`, one abstract step for the two sides (`+0x93`, `+0x94`):
  - each side runs its combat step (slot `+0x14`) and is asked whether it
    is beaten (slot `+0x28`);
  - each side's strength is its `+0x3c`. A side whose strength has not
    fallen accumulates time (`+0x9b`, `+0x9c`); a fall resets it and
    stores the new low (`+0x99`, `+0x9a`);
  - with neither side beaten and both stalled for at least
    `DAT_0066c3e0` = 1200, the weaker side withdraws. Each of its units
    (lists `+0x58` and `+0x64`) goes through `FUN_005af7a0`; a side whose
    slot `+0xc` refuses is marked `+0xb0` instead;
  - with a side beaten, it fills the 7 result words (`+0x10`..`+0x28`).
    Among them are a side's flag when its units' state `+0x170` is 2 and
    one when a unit's state is 9, by the unit's side `+0x168`.
- So the abstract battle runs the tactical simulation's own combat step
  without the display, with a stalemate rule. It is not a separate
  formula.

## For the port

- The port's strategic auto-resolve should be the tactical simulation run
  without display, stepped at 1.0 with the 1200 stalemate rule. The port
  has a tactical engine, and the tracker item "Walka taktyczna 3D" asks
  for its result on the same path.
- Since 2026-10-21 the engine's state and step are in `rebellion-core`
  (`tactical.rs`, `BattleSession::step`); the view stays in
  `rebellion-render` (`tactical_view.rs`).
- The four "slot handlers" `+0x1c4`..`+0x1d4` the audit named are the
  tactical module's, not a strategy formula.

## Open

- The sides' combat step (slot `+0x14` of the objects at `+0x93`,
  `+0x94`) and strength (`+0x3c`): the tactical engine's own rules, to
  check against the port's tactical code.
- The meaning of each of the 7 result words, and how `FUN_0040c780`'s
  inner walk turns them into damage per ship.
- What a computer player answers in state 6 (slot `+0x3c` of its player
  object).
