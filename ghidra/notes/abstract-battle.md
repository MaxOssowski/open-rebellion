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

## Retreat (read 2026-10-21, W2h)

- Slot `+0x0c` (`FUN_005d0f50`) says whether a side can leave. All of
  these must hold:
  - its commander's `+0x80` is set and `+0x84` is not 5 (the retreat has
    not run);
  - no arrival is pending (`FUN_005c7e40`: the battle's `+0x58` units with
    `+0x150`);
  - a fighter or capital ship has a working hyperdrive (`FUN_005b1b70`:
    `FUN_005b1bc0` = installed hyperdrives `+0x3ec` minus hits `+0x374`
    is above 0, and engine power, slot `+0x90`, is not 0); for side 1, an
    operational Death Star (`FUN_005ba420`, or its `+0x74`) also counts.
- Slot `+0x2c` (`FUN_005d19a0`) decides whether the side leaves now. If it
  can leave, it leaves when any of these holds:
  - its `+0xb0` is set;
  - its strength is 0;
  - its strength is under `+0x24` = 0.33 (`0x3ea8f5c3`, set by
    `FUN_005c7cb0`) of the enemy's (`FUN_005afa30`).
- The retreat itself (`FUN_005d14d0`, step 5):
  - fighters with a hyperdrive escape (`FUN_005af910`, state 9); those
    without are destroyed unless `FUN_005afe40` finds them recovered into
    their carrier;
  - capital ships with a hyperdrive escape; the others are destroyed;
  - `+0x84` becomes 5.
- At the stalemate (`FUN_005ae9c0`), a weaker side that can leave gets
  `+0xb0` = 1 instead of destruction. The Alliance's mark ends that step.
- `+0x80` itself:
  - `FUN_00599c90` / `FUN_0059a0b0` set it per side through
    `FUN_005afb30`. Their stack arguments 7 and 8 are copies
    (`FUN_004fcd90`) of `FUN_00595d60`'s arguments 10 and 11, which
    `FUN_0040b0e0` takes from the battle object's `+0x10` and `+0x14`.
    The commanders are elsewhere: `+0x18` to `+0x24`
    (`FUN_00509460` / `FUN_00509530`).
  - `+0x10` and `+0x14` start at 0 (`FUN_0040afa0`, `FUN_0042dbe0`).
    `FUN_0041cfc0` -> `FUN_004362b0` fills them from the two sides'
    answers to the battle choice (their controllers' slot `+0x3c`).
  - What each answer means: see "The battle choice" below.

## The battle choice (read 2026-10-21)

`FUN_0040a050` state 6 asks both sides through `FUN_0041cfc0` ->
`FUN_004362b0`. The session object `DAT_006b14e4` (`FUN_00435bc0`) holds
the two side controllers at `+0x10` and `+0x14`. In a game against the
computer they are the player (vtable `0x65ad70`, `FUN_004861b0`) and the
AI player (vtable `0x65abf0`, `FUN_004857a0`). A network game has a
remote player instead (vtable `0x65aa70`, `FUN_00484e70`); its slot
`+0x3c` (`FUN_00485180`) reads the answer from the message queue.

- Each controller's slot `+0x3c` (`FUN_00488600`) returns its answer
  `+0x38`, and copies its retreat word `+0x3c` into the state machine's
  `+0x0c` / `+0x10` (`FUN_004fcd90`). States 7 and 8 copy those two words
  into the battle (`FUN_0040b0e0`), and from there they reach each side
  commander's `+0x80` (`FUN_005afb30`). `+0x80` is tested only for
  non-zero (`FUN_005d0f50`, `FUN_005c7e80`).
- Answer 0 means "not yet". Each answer is also sent to the other
  controller (slot `+0x48`).
- `FUN_004362b0` sets "decided" when both answers are non-zero. It sets
  "simulate" only when both are 1. State 6 then goes to 8 (simulate:
  the abstract battle) or 7 (the tactical battle).

**The player** answers through the Battle Alert dialog (class
`0x659d28`, built by `FUN_0044f860`). Its command handler
(`FUN_00450540`) calls `FUN_0041ce60` -> `FUN_00436090` ->
`FUN_00487830(a, b, …)`. With `a` = 0 that sets answer 2. With `a` = 1 it
sets answer 1, and with `b` also 1 it sets `+0x3c` = 10000.0
(`0x461c4000`). The dialog first clears `+0x3c` (`FUN_00486670`,
`FUN_004865b0`). Another window class (`FUN_004710f0` on Escape, and
`FUN_00470ea0`; window `+0x174`) gives answer 3 through `FUN_0041ce80` ->
`FUN_00487890`. Answer 3 is "decided, not simulate". Which window that is
remains open.

| Button | Id | Text (TEXTSTRA) | Bitmaps | Answer | `+0x3c` |
|---|---|---|---|---|---|
| Simulate Results | `0x65` | 6438 `0x1926` | `0x29dc..` / `0x29e2..` | 1 | 0 |
| Take Command | `0x66` | 6439 `0x1927` | `0x29df..` / `0x29e5..` | 2 | 0 |
| Retreat | `0x67` | 6474 `0x194a` | `0x2adb..` / `0x2ade..` | 1 | 10000.0 |

So Retreat runs a simulated (abstract) battle in which the player's side
may leave. In it, that side leaves under the usual test (`FUN_005d19a0`).
Retreat does not open the tactical battle.

**The AI player** answers in its slot `+0x44` (`FUN_00485d50`). Its
answer is always 1 (simulate). It clears `+0x3c`. Then, unless `+0x418`
is set, it asks its strategy module `0xd` (`FUN_005f5500(0xd)` on the
module map `+0x64`, `FUN_004bf300`). The current strategy object (`+0x60`,
one of seven from `FUN_004bf7c0`) writes `+0x3c` through its slot
`+0x10`. `+0x418` is zeroed by the constructor (`FUN_004856f0`) and
otherwise only loaded from a save; no code sets it.

The seven strategy slots are the same code with their own copies of the
constant 0.9: `FUN_004e6110`, `FUN_004e6a60`, `FUN_004e77f0`,
`FUN_004e87a0`, `FUN_004e94f0`, `FUN_004e9ec0`, `FUN_004eabb0`, with
constants at `0x65c8e4` .. `0x65ca04`. Each slot works as follows:

1. Start with 0.9 (`0x3f666666`).
2. Look up the AI's system record for the battle's system
   (`FUN_00403d30` on the controller's `+0x2c` map). Only when it is the
   AI's own system (`+0x30` bit 1) with active squadrons on the planet
   (`+0xac`, `v[25]`, above 0) does the rule go on. Otherwise the value
   stays 0.9.
3. At the AI's headquarters or system `0x109` (record `+0x28` bit
   `0x80000000`) the value is 0, so no retreat.
4. Otherwise take the game's system (`FUN_004f3220`, types `0x90..0x97`)
   and each fleet at it (`FUN_004ffe70(system, 3)`). Sum the fleet
   records' (`+0x78` map, `FUN_00419e40`) `+0x58` for own fleets (`+0x38`
   bit 1) and for the others. `+0x58` is the record's `values[2]`, the
   ships' attack (`ai_records::FleetRecord::add_ship`, `s[14]`).
5. The value is own / others (own alone when the others sum to 0). If it
   is above 0.9, it becomes 0.9.

So the AI orders a retreat everywhere except at its own system that has
active squadrons. There it retreats only when its fleets have some
attack, and never at its headquarters. A value of 0 means "no retreat".
The value's size does not matter, because the battle's leave threshold
is the side commander's `+0x24` = 0.33.

- Open: the player controller's slot `+0x44` (`FUN_004865b0`) passes the
  dialog a flag from `FUN_004fcf50` / `FUN_0051dce0` (`FUN_00488d20`
  stores it at the window's `+0x50`). It is not read here.
- Open: the Retreat button's enabled state in the original dialog. The
  port enables it when the player's ships have a working hyperdrive.

### The port (W2k, 2026-10-21)

- `ai_strategy::orders_retreat` is the strategy slot's rule. It builds
  the facts from the world at the battle: the holder, a known squadron on
  an explored, populated planet, the headquarters or `0x109`, and the
  `values[2]` of each fleet record. `ai_strategy::retreat_orders` answers
  for each side an AI plays; an AI with no strategy yet (`Strategy::kind`
  0) gives no order.
- `abstract_battle::resolve_space_battle` takes the two answers as
  `retreat_allowed`. The headless simulation and the app's AI battles
  ask the AI players.
- The player's Battle Alert: Simulate Results gives no order, and Retreat
  (`TacticalAction::RetreatBeforeBattle`) gives one. Both run the abstract
  battle. Retreat no longer opens the tactical battle.
- `port:` Take Command (the tactical battle) does not yet pass the AI
  side's order to the tactical engine.

## The Death Star in the abstract battle (read 2026-10-21, W2i)

The side commander's step (`FUN_005d14d0`, `0x5d14d0..0x5d161c`) starts
with the Death Star (`FUN_005a66a0` on the battle). Its fields are `+0x68`
hull, `+0x6c` laser charge, `+0x70` shielded, `+0x74` destroyed and `+0x78`
action committed. `FUN_005ba420` (operational) needs `+0x74` clear,
`+0x68` != 0 and `+0x78` != 1. `FUN_005ba450` (attackable) also needs
`+0x70` clear.

**Side 1 (the Empire)**, with an operational Death Star:

1. If the charge is not 100 (`DAT_0066d0f4`), `FUN_005ba7f0(dt)` adds
   `dt x 1/3 (DAT_0066c708) x 0.001 (DAT_0066c718)`, capped at 100
   (`DAT_0066c71c`). It also needs hull > 0 (`DAT_0066c710`) and `+0x74`
   clear. Event `0x113` sounds on reaching full charge in display mode
   (`+0x9ec` == 1).
2. At 100 the commander refreshes its orders (slot `+0x08`) and walks its
   enemy list `+0x90` (`FUN_005f5060`; the priority list of "Targets"
   above). The first unit whose type (slot `+0x20`) is 0, a capital ship,
   is destroyed (`FUN_005af7a0`), and the charge returns to 0
   (`FUN_005ba710`).

The abstract step passes dt = 1.0 (`DAT_0066badc`), so the charge grows by
1/3000 a step and needs 300,000 steps to fill. The stalemate ends a battle
after 1200 steps without a strength fall (`DAT_0066c3e0`). **In the
abstract battle the superlaser in practice never fires.** Display mode
passes milliseconds, which fills the charge in 300 seconds (the port's
`ORIGINAL_DEATH_STAR_LASER_CHARGE_PER_SECOND`).

**Side 0 (the Alliance)**, with an attackable Death Star:

- If the Empire commander's fighter list (`+0x250` -> `+0x64`) is shorter
  than the Alliance's (`+0x64`, `FUN_005c5eb0` counts), the commander
  walks its task forces (`+0x70`, slot `+0x0c` then `+0x04`). The first
  whose leader's kind (slot `+0x04`) is 4, a fighter group, makes the
  trench run (`FUN_005aefc0(leader)`). Only one run happens per step.

**The trench run** (`FUN_005aefc0`, `0x5aefc0..0x5af41d`):

1. Take the task force's members (`FUN_005c6100` on the group `+0x1c`).
   Visit them in a cycle (`FUN_005f6590`, wrapping to the head), counting
   visits. After the 100th visit every member left is destroyed
   (`FUN_005af7a0`, `0x5af2d7`) and the run ends.
2. For the member visited: `m` = its maneuverability (slot `+0xbc`, cast to
   an integer). Roll `r` = 1..10 (`FUN_005af420(1, 10)`, the battle's
   stream `DAT_006bc4a8`).
   - If `r > m`, it is hit. `h` = its hull `+0x3cc` / 8.0 (`DAT_0066c3dc`),
     as an integer. With `h` <= 1 it is destroyed. Otherwise it takes
     `(1..h) x 8.0` damage (`FUN_005b54d0`) and is destroyed if its hull
     falls to 0 or below. A destroyed member leaves the list
     (`FUN_005c60d0`, its id `+0xec`); an empty list ends the run.
   - A member still with hull > 0 (`DAT_0066c3e8`), hit or not, tries the
     shot. `c` = `FUN_005afc00(0)` capped at 9.0 (`DAT_0066c3ec`).
     `s` = (`FUN_005b23a0(member)` + `c`) x 0.5 (`DAT_0066c3f0`). Roll
     1..100: at or under `s` (as an integer) the run succeeds.
3. Success (`0x5af30d`): `FUN_005ba8a0` zeroes the Death Star's charge and
   hull `+0x68`. Every unit in its lists `+0x38` and `+0x3c` is destroyed
   (`FUN_005af7a0`), and the run ends.
4. Otherwise go to the next member.

The terms:

- `FUN_005b23a0`: `FUN_005b1770(+0x134 x 0.53333336 (DAT_0066c4cc))`.
  That is the FIGHTSD torpedo rating `+0x134` times 0.5333, scaled by
  hull / full hull (`+0x3cc` / `+0x3c8`, `FUN_005b1770`). The port has the
  coefficient as `ORIGINAL_FIGHTER_TORPEDO_STRENGTH`.
- `FUN_005afc00(0)`: the battle's `+0x254` holds side 0's commanding
  character id. `FUN_005af440` registers a battle person (`FUN_00596c40`,
  `+0x44` side, `+0x18` id) there when its `+0x58` is set, and at `+0x25c`
  when `+0x54` is set. The value is that person's `+0x50` minus -1.0
  (`DAT_0066c3e4`), clamped to 1.0..9.0 (`DAT_0066c3f4`, `DAT_0066c3ec`);
  with no such person it is 1.0. `FUN_0040c200` builds each person from
  the character's slots `+0x1f0` and `+0x1f4`, through `FUN_0040d680` and
  `FUN_0040d670`.
  - Read 2026-10-21 from the pushes at `0x40c394..0x40c434` and
    `FUN_00596340` -> `FUN_00596c40`:
    - `+0x44` is the side (`FUN_0040d660(FUN_0040d690)`: 0 Alliance);
    - `+0x50` is combat (slot `+0x1f0`) / 20 (`fidivl DAT_006a80e8`);
    - `+0x4c` is slot `+0x1f4` / 10 (`DAT_006a80e4`);
    - `+0x54` is "is the battle's `+0x18`" (`+0x20` for the Empire);
    - `+0x58` is "is the battle's `+0x1c`" (`+0x24`);
    - `+0x48` is "a major character" (family `0x30..0x37`).
  - `FUN_0040b0e0` fills `+0x18` / `+0x20` with `FUN_00509460(system,
    side)`: the side's fleets' Admirals (`+0x96` = 2), best by slot
    `+0x1f4`. It fills `+0x1c` / `+0x24` with `FUN_00509530`:
    - first `FUN_00509330`, the Commander (`+0x96` = 1) of the system's
      own side standing there;
    - then the side's fleets' Commanders, replacing it on higher combat
      (slot `+0x1f0`).
  - So the trench run's term is the Alliance's best Commander's
    combat / 20 + 1, clamped to 1..9. It falls back to 1.0 when none
    exists or when the pick is the system's Imperial Commander.
  - Ported as `abstract_battle::trench_commander_term`. `hyp:` every
    character at the battle has a battle person.
- Open: the Death Star's lists `+0x38` and `+0x3c`, which are the units
  it carries.

In the port, the displayed battle's run (`BattleSession::
prepare_original_trench_run`, `FUN_005d04e0`) is a different function, with
its own success rating. Since W2i the abstract battle has both
(`AbstractBattle::death_star_step`, `trench_run`).
- `hyp:` the leader kind 4 is a fighter group.
- The commander term is `trench_commander_term` (see above).
- `port:` the Death Star's carried units are unknown and none falls with
  it. The port's Death Star capital ship falls with it, so the fleet loses
  the Death Star.

## The port (2026-10-21)

`rebellion-core::abstract_battle` steps a `BattleSession` this way:

- the 1.0 step;
- per-task-force fire with `FUN_005b7780`;
- the recharge;
- the beaten test;
- strength with the out-damaged exclusion;
- the 1200-second stall that destroys the weaker side, or both on a tie.

Since W2f it resolves every strategic space battle the port resolves on its
own (`resolve_space_battle`, all fleets at the system). Three pieces stay
`hyp:`:

- Targeting: ported in W2e, following the "Targets" section above.
  - `hyp:` the travel speed is the leader's engine power.
  - `hyp:` a fighter group's shield-generator ratio is its hull ratio.
  - The game-list terms are empty without display.
  - Over seeds 1-3, 55 of 67 battles end as with the earlier rule, with
    26 attacker and 41 defender wins (28 and 39 before).
- A fighter group hit by `FUN_005b54d0`: shields first, then hull.
- The retreat (W2h) is ported. Since W2k a side may leave when its
  answer to the battle choice orders a retreat (`retreat_allowed`). A
  fighter group leaves with a jumping carrier (`hyp:`). The Death Star's
  abstract actions are ported since W2i (see above).

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
