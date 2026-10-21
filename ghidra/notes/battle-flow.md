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
  (`FUN_0059a730`). For their meaning and the winner rule, see "The
  winner and the loser's withdrawal" below. (An earlier reading called
  words 1 and 3 each side's survivors. They are each side's "beaten"
  marks.)
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
    `DAT_0066c3e0` = 1200, the weaker side ends. If its slot `+0xc` says
    it can leave, it is marked `+0xb0`. Otherwise each of its units (lists
    `+0x58` and `+0x64`) is destroyed by `FUN_005af7a0`: state `0xb` and
    100,000 damage, not a withdrawal (`abstract-battle.md`);
  - with a side beaten, it fills the 7 result words (`+0x10`..`+0x28`).
    Among them are a side's flag when its units' state `+0x170` is 2 and
    one when a unit's state is 9, by the unit's side `+0x168`.
- So the abstract battle runs the tactical simulation's own combat step
  without the display, with a stalemate rule. It is not a separate
  formula.

## The winner and the loser's withdrawal (read 2026-10-21)

**The result words** (`FUN_005ae9c0`, `0x5aec52..0x5aee90`). The side
commander at `+0x24c` is the Alliance (side 0) and the one at `+0x250` is
the Empire (side 1). A unit's `+0x168` gives its side; state `+0x170` 2 is
active, 9 escaped and `0xb` destroyed.

| Word | Offset | Set when |
|---|---|---|
| 0 | `+0x10` | the Alliance's retreat ran (`+0x84` == 5) and one of its units escaped |
| 1 | `+0x14` | the Empire has no active unit (lists `+0x58`, `+0x64`), word 0 is clear, and no operational Death Star (`FUN_005ba420`) |
| 2 | `+0x18` | the Empire's retreat ran, and one of its units escaped or the Death Star's `+0x78` is set |
| 3 | `+0x1c` | the Alliance has no active unit, and word 2 is clear |
| 4..6 | `+0x20..+0x28` | 0 here (the tactical game fills them) |

When words 1 and 3 are both set, both are cleared: each side destroyed
the other.

**The winner** (`FUN_0040c780`, `local_10c`): 0 none, 1 Alliance, 2
Empire, 3 the system's.

1. From words 1 and 3: only word 1 gives 1, only word 3 gives 2. A side
   that retreats has no active unit left, so the side that stays wins.
2. If no winner yet, in the tactical mode (`+0xc` == 0) with word 6 set,
   the battle runs again (`*param_2` = 1; state `0x16` goes back to 8).
3. Otherwise the same test on words 0 and 2: only word 0 gives 1, only
   word 2 gives 2. This pass can only decide when the first one did not,
   which needs a side that retreated yet still has an active unit.
4. When both words of a pair are set, the winner comes from the system's
   `+0x78` bits 6..7: 3 gives 3, 1 gives 1, anything else gives 2.
5. `FUN_0050cfd0(1)`, `FUN_0050ceb0(winner)` store it on the battle.

The system's `+0x78` bits 6..7 hold the side of the last fleet (`0x08..
0x0f`) or squadron (`0x1c..0x1f`) to join the system. When a unit joins,
the system's slot `+0xd0` (`FUN_00508660`, `this` = the system) calls
`FUN_0050a1b0(unit +0x24 >> 6 & 3)` (`0x508a46`, `0x508b0d`).

**The loser's withdrawal** (`FUN_00516ef0`, state 9):

1. Clear Battle (`FUN_0050a580(0)`). Count each side's active fleets
   (`FUN_004ffef0(system, 1, side)`) and planet squadrons
   (`FUN_00503a50(system, 1, side)`).
2. The winner is the battle's (`FUN_00509ad0`). With none, and both sides
   still present, it comes from the system's `+0x78` bits 6..7, as above.
   With none and one side gone, nobody withdraws.
3. The loser is the other side: winner 1 gives 2, and winner 2 gives 1.
   Winner 3 gives 3, every side (the side mask of `FUN_004ffef0`).
4. When `FUN_00506e60` allows, each of the winner's planet squadrons goes
   to `FUN_00556710` (not read).
5. Each of the loser's fleets, then each of its planet squadrons, is set
   Autorouting (`FUN_004f78e0(1)`, `+0x50` bit `0x1000`).

**Autorouting a fleet** (event `0x307`, `FUN_00515480`):

- A destroyed object (`+0x50` & 8) is skipped. A character
  (`0x30..0x3f`) takes the character path (`captive-escape.md`).
- For a unit, with Autorouting set, from a system (`0x90..0x97`):
  - The autoroute search `FUN_005545e0` (`captive-escape.md`) finds a
    destination. In order: the current container, the `+0x70` object,
    then the nearest system of its side that is not blockaded, and so on.
  - A completed, standing unit then tests the destination
    (`FUN_00555410` -> `FUN_00555460`, not read). If that fails and the
    unit cannot move (slot `+0x34`, its speed, is 0), it is destroyed
    with reason 5 (slot `+0xac`).
  - The move itself is `FUN_00556390`, as for a player's move.
- Open: the branch with `local_10 != -1`, which calls slot `+0xac(2)`.

**In the port** (2026-10-21, W2j):

- `abstract_battle::AbstractBattle::finish` builds the four words and
  takes the winner as above, with words 0 and 2 both set from the
  system's last arrived side (W2l). The Death Star's `+0x78` in word 2 is
  not modelled.
- Since W2l (save v102) `System::last_arrival` keeps `+0x78` bits 6..7.
  `System::unit_joined` sets it where a fleet or squadron joins a system:
  - a fleet's departure: `FUN_00556390` puts it in its destination at
    once (`movement::begin_fleet_transit`);
  - a new fleet from a split;
  - a capital ship completed into a new fleet, or a squadron completed on
    the planet;
  - a squadron moved onto a planet.

  `port:` seeded systems start with no side. A battle sortie's squadrons
  landing back after the battle do not set it, because the sortie is the
  port's.
- `withdraw_loser` with no winner and both sides present sends away the
  side that did not arrive last.
- Since W2k the integrator's winner is the battle's, not the side left
  in orbit. A side that escaped is still in orbit until it is sent away.
- `abstract_battle::withdraw_loser` sends each loser fleet at the system
  to `missions::nearest_place`, leaving out the battle's system. It runs
  in the headless simulation, the app's AI battles, Auto Resolve, and
  after a tactical battle.
- `port:` a fleet with no destination, or with speed 0 or no hyperdrive,
  stays; the original destroys a unit that cannot reach another system.
  The loser's planet squadrons stay, and the winner's (`FUN_00556710`)
  are not read.

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
- How `FUN_0040c780`'s inner walk turns the result into damage per ship
  (`FUN_0040d260`, `FUN_0040d320`). The words are read above.
- State 6's answers are read (`abstract-battle.md`, "The battle
  choice").
