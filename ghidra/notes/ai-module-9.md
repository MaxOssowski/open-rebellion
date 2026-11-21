# The AI's module 9: the headquarters fleet planners

Read 2026-11-21 from the decompile (`~/Games/rebexe-decompile/decompiled-typed/`)
and checked in the REBEXE.EXE disassembly where the decompile drops `this`
or a stack argument (addresses below). Module 9 keeps one planner for each
of the side's headquarters systems that is not blockaded. A planner holds
capital ships and one fleet, gathers heavy ships to its system until its
fleet's attack covers the side's best heavy ship class (twice that in a
Headquarters Only game), gives back what is over, lands the regiments and
people its fleet carries, and brings the fleet home.

First look: `ai-production-modules.md`, "Modules 9 and `0xe`, looked at".

## The module (`FUN_004cf0a0`, vtable `0x65c718`)

`FUN_004e4fb0(9, ..)`; `+0x38..+0x44` = 0. Slots: `+0x14` the step
(`FUN_004cf290`: ready, `FUN_005f4960() == 1`, then `FUN_004cf2c0`),
`+0x18` the shared refusal `FUN_004dd810` (`+0x38`, `+0x3c` = 0).

| State `+0x3c` | Action |
|---|---|
| other | 2 |
| 2 | `FUN_004cf350`: the planner `+0x44` names (`FUN_005f5500`), else the list's head, into `+0x40`; `+0x44` takes the next one's id, 0 at the end (no wrap). Found: 3, not complete. None: 0, complete. |
| 3 | `FUN_004cf3b0`: the planner `+0x40` names steps once (`FUN_00474390`), its done flag preset to 1 (a vanished planner is done). An order completes the module; with the planner not done the priority `+0x34` rises and the state stays 3. Done: 0, complete. No order and not done: not complete. |

The planners live on controller `+0x11c` (`0x4e69d3`). The controller's
constructor (`FUN_004178f0`, `0x417aad`) points the list's `+0x10` at
the controller, which the planners keep at `+0x4c`.

## The strategy's state 5 (`FUN_004e69d0`)

With fewer planners than controller `+0x224` (`0x4e69e9`),
`FUN_0042ed50` adds one (`FUN_00474070`, `0x50` bytes, vtable
`0x65a3a0`, `FUN_005f5440` numbers it). With more, `FUN_0042edc0` looks at
the list's head only: when it has no system (`FUN_00474260`) it gives
everything back (`FUN_004742b0`) and leaves the list; else nothing.

Controller `+0x224` sums the sector records' `+0x1a0` (`FUN_0041b7e0`
reads `+0x130` of the values at `+0x70`), which `FUN_00432350` counts:
own systems with `+0x28` `0x80000000` (the headquarters, or Coruscant
`0x109`) and no blockade (`+0x28` bit 2).

## The planner (`FUN_00474070`)

| Field | Meaning |
|---|---|
| `+0x18` | id |
| `+0x1c` | state |
| `+0x20` | the side, copied into each order's `+0x20` |
| `+0x24` | the held ships' list (`FUN_004f5510`) |
| `+0x2c` | the held fleet (`0x08..0x0f`) |
| `+0x30` | the headquarters system (`0x90..0x97`) |
| `+0x34` | the balance: the fleet's attack less what it should have |
| `+0x38` | the lists want checking against the records |
| `+0x3c` | a merge list for the queries (`FUN_004355d0`) |
| `+0x4c` | the controller |

Save and load (`Class_0065a3a0_vt4`/`vt8`) keep `+0x1c`, `+0x20`, `+0x2c`,
`+0x34`, `+0x38`, the list and `+0x30`.

Marks it owns:
- ship record `+0x24` bit `0x10000000`: held by this kind of planner
  (`FUN_00475e70` sets it and clears `0x20000`, free; `+0x30`, `+0x34` =
  0);
- fleet record `+0x38` bit 8: its fleet (`FUN_00475fe0` sets it and clears
  bit 4, free);
- system record `+0x30` bit `0x80000`: its headquarters (`FUN_004744b0`).
  Only `FUN_004744b0` and `FUN_004742b0` touch it (`0x474376`,
  `0x474582`).

### The step (`FUN_00474390`)

Checked at `0x474390..0x47448b`. The step clears the caller's done flag,
runs the refresh `FUN_004747b0` when `+0x38` is set (and clears it), runs
one state, and sets `+0x38` again after an order.

| State | Action |
|---|---|
| other | 1, and `+0x38` = 1 |
| 1 | `FUN_004744b0` keeps or finds the system: 2; none: 8 |
| 2 | `FUN_004749b0`: Create Fleet of the damaged ships. 3 |
| 3 | `FUN_00474b90`: the balance. 4 |
| 4 | `FUN_00474c00`: take or give ships. 5 (always, below) |
| 5 | `FUN_00474ce0`: land the regiments. 6 |
| 6 | `FUN_00474ff0`: land the people. 7 |
| 7 | `FUN_00475330`: bring the fleet home. 0, done |
| 8 | `FUN_004742b0`: give everything back. 0, done |

State 4 always moves on: the step passes the address of its own
argument slot (`0x4743fd`), which still holds the caller's flag pointer,
and `FUN_00474c00` only ever writes 1 through it (`0x474cad`,
`0x474cc3`); the slot is never 0 (`0x47440f`).

### The refresh (`FUN_004747b0`, `0x4747d9..0x474986`)

- The fleet stays when its record is the side's (`+0x38` bit 1), still
  marked 8, and has a ship (`FUN_005f50e0` on record `+0x48`); else
  `FUN_00476080` gives it back (record bit 4 set, 8 cleared, `+0x30`,
  `+0x34` = 0) and clears `+0x2c`.
- Each held ship whose record is the side's (`0x4000`) stays when the
  planner's fleet is the ship's (record `+0x2c`); with no fleet it stays
  when the ship's fleet has a record, and a free one (bit 4) becomes the
  planner's (`FUN_00475fe0`, `0x474942`). Any other ship is given back
  (`FUN_00475f60`).

`FUN_00475f60` (give back): when the ship's record has no sector planner
(`+0x30` = 0) it is free again (`| 0x20000`, `& 0xecffffff`); the balance
loses its `+0x7c` (`values[14]`, attack); the id leaves the list.
`FUN_00475e70` (take): a ship id (`0x14..0x1b`) with a record loses
`0x20000`, gains `0x10000000`, `+0x30`/`+0x34` = 0; a `+0x7c` above 0
adds to the balance; the id joins the list.

### State 1 (`FUN_004744b0`)

- With a system: its record must be the side's (`+0x30` 1), a
  headquarters (`+0x28` `0x80000000`) and not blockaded (`+0x28` 2): it
  stays and its record gets `+0x30` `0x80000`. Else the planner forgets it
  and the record loses `0x80000` (`0x47457b`). With no record nothing
  changes and the state fails.
- With none: the sectors with `+0x24` `0x40000000` (an own system with
  `+0x28` `0x80000000`; `FUN_0041a1d0(0x40000000, 0, 0, 0, 0, 0, 1)`,
  keys drawn below 20, `0x4745c9`), in pick order; in each, the systems
  with `+0x28` `0x80000000` without 2, `+0x30` 1 without `0x80000`
  (`FUN_0041a460(sector, 0x80000000, 0, 1, 2, 0, 0x80000, 1)`, keys drawn
  below 10, `0x47465b`), the pick; the first found becomes the planner's
  and its record gets `0x80000` (`0x474715`).

### State 2 (`FUN_004749b0`, `0x4749b0..0x474b85`)

When the fleet's record has damaged ships (`+0x38` `0x3000`) and none of
`0xf0000802`: each ship of the record (`+0x48`) with `0x801000` and
without `0x800` goes into a list; Create Fleet `0x270` of them
(`0x474b1b`) with the fleet's system (record `+0x28`, copied at
`0x474a2f`) for its target. The ships are not given back here; the next
refresh does it once they are in another fleet.

### State 3 (`FUN_00474b90`, `0x474b90..0x474bf0`)

`+0x34` = the fleet record's `+0xb4` (`values[25]`, its attack with the
fighters), 0 without one, less controller `+0x164`, twice that when
controller `+4` has bit 1 (a Headquarters Only game, `FUN_004182a0`).

**Controller `+0x164`** is the class tree's (`+0x138`, `FUN_0042dc10`)
`+0x2c`. `FUN_0042e210` writes it (and `+0x160..+0x174`, `+0x150` the
level) at setup (`FUN_0042dd00`) and on each Shipyard research
notification (`FUN_004198a0`, `0x125`):
`FUN_0042e960(side, level, 1, 0x200, 1)`. That ranks the class records
of the side (`+0x28`, so capital ships only for flag 1) with flag 1
(`FUN_004739b0`: turbolasers or ion cannons, no gravity well) and a
research order at most the level, on key `0x200` (slot `+4`,
`FUN_00473930`: `values[0]`), order 1, picks the last (the largest), and
returns that class's key value. So `+0x164` is the best `values[0]` of the
side's researched heavy ship classes, 0 with none. The coin on equal keys
cannot change the value returned.

This refutes the earlier reading that controller `+0x164..+0x174` have no
writer (`ai-sector-plans.md`, kinds 1 to 6 take them as floors); all of
`+0x150..+0x174` are ported as `ai_records::ClassFloors`
(`ai-sector-plans.md`, "The class tree's floors").

### State 4 (`FUN_00474c00`, `0x474c00..0x474cd2`)

With the fleet en route (record `+0x38` 2) nothing. Else:
- balance below 0: `FUN_00475490`; with no order `FUN_00475890`;
- then, with no order and the balance above 0: `FUN_00475c70`.

`FUN_00475490` (with a system): the fleets nearest it
(`FUN_0041a8a0(system, 0x8005, 0xa, 2)`, `0x475529`: own, free, a ship
not damaged; not en route, not marked 8; squared distance, order 2) in
pick order, until one gives ships. In a fleet record's ships, each free
one (`0x20000`) without `0x803800` stops the search, unless the balance is
below 0: then it goes on, and a heavy one (bit 1) is taken
(`FUN_00475e70`, the balance grows). With ships taken: Move `0x201` of
them into the planner's fleet (target `+0x2c`), or with none Create Fleet
`0x270` for the system (`+0x30`).

`FUN_00475890` is the same over `FUN_0041ab40(0, 0x8001, 0xf000000e, 1)`
(`0x475928`): the side's fleets held for a sector plan (`+0x34` above 0),
own, a ship not damaged, none of en route, free, 8 and the roles; keyed on
the plan's id, order 1.

`FUN_00475c70`: the first held ship whose record has none of `0x2800`,
whose fleet is not busy (`FUN_00484510`: a role `0xf0000000` or standing
at the opponent's system) and whose `+0x7c` is at most the balance is
given back and leaves as a fleet of its own (Create Fleet `0x270`,
target `+0x30`). One ship a cycle.

### State 5 (`FUN_00474ce0`, `0x474d3e..0x474f9c`)

When the fleet's record carries regiments (`+0x80`, `values[12]` above 0)
at a system of the side (system record `+0x30` 1): the fleet's ships that
are the side's, held (not free) and not en route
(`FUN_0041ac80(fleet, 0x4000, 0x20800, 9, 1)`, `0x474db2`), and the
regiments aboard each (`FUN_00504c40(ship, 1)`): Move `0x201` of them
(`0x474f5e`) to the fleet's system (record `+0x28`, `0x474da4`).

### State 6 (`FUN_00474ff0`, `0x475021..0x475313`)

When the fleet's record has an own person aboard (`+0x38` `0x4000000`)
and is not en route, at a system of the side: each ship of the record not
en route (`0x800`) with people aboard (`0x600000`) gives its people
(`FUN_00536da0(ship, 1)`) with a record; all go into one list, and one
whose record has `+0x30` bit 2 (the side's own) allows the order
(`0x475212`). Move `0x201` of them (`0x4752d2`) to the fleet's system
(`0x4752fb`).

### State 7 (`FUN_00475330`)

With the system not blockaded and the fleet's record not en route at
another system: Move `0x201` of the fleet (`0x475434`) to the system.

### State 8 (`FUN_004742b0`)

Every held ship is given back (`FUN_00475f60`), then the fleet
(`FUN_00476080`); a system with a record loses `+0x30` `0x80000` and the
planner forgets it (`0x47436f`).

## Ported (2026-11-21)

`crates/rebellion-core/src/ai_module9.rs`, save v137
(`SaveState::ai_module9`, the last field; the AI's own encoding skips
it). The scheduler steps module 9 (`ai::step_module`) with a planner's
view of the pass (`PlannerIo`, as module 8's planners see it), its
refusal is `Module9::refused`, and the strategy's state 5 runs with
states 4 and 6 when module `0xd` steps (`Module9::balance`, controller
`+0x224` from the sector records). Controller `+0x164` is
`ai_records::ClassFloors::heavy_attack`.

Orders go through the validated paths: Create Fleet (`AIAction::CreateFleet`),
the fleet's Move home (`AIAction::MoveShips` of all its ships), the
regiments' and people's Moves (`AIAction::MoveRegiments`,
`AIAction::MoveCharacters`), and a new `AIAction::JoinShips` for the
ships' Move into the planner's fleet: `fleet_join::join_fleet` with the
ships for its mover, checked by `fleet_join::validate_join`.

port:
- A fleet's regiments and people ride its first ship's record, so the
  ship that carries them is that one (states 5 and 6).
- Controller `+0x164` is computed each pass from the side's Ship level;
  the original keeps what `FUN_0042e210` wrote at setup and at each
  Shipyard research notification. Its ranking's coin is not drawn: it
  cannot change the value.
- As for the regiments, one refused person refuses the order: a fleet
  carrying a prisoner of the other side has its people's Move refused
  (the validator's per-person check, `ai::accepts`).
- The scheduler tests' queued orders moved from module 9's slot to module
  `0xe`'s, the one module left without a counterpart.

Seeds 1-10 (dual AI, 5000 days), before (module 9 ordering nothing) and
after:

| Seed | Before: end | Systems A/E | Max fleets A/E | After: end | Systems A/E | Max fleets A/E |
|---|---|---|---|---|---|---|
| 1 | day 5000 | 32/28 | 27/34 | day 5000 | 34/19 | 27/20 |
| 2 | day 5000 | 17/29 | 17/22 | day 5000 | 32/14 | 27/12 |
| 3 | day 5000 | 25/26 | 32/18 | day 5000 | 26/54 | 33/35 |
| 4 | day 5000 | 17/39 | 22/30 | day 5000 | 33/34 | 41/32 |
| 5 | day 5000 | 33/24 | 20/15 | day 5000 | 43/14 | 23/13 |
| 6 | day 5000 | 48/16 | 37/22 | day 5000 | 41/19 | 25/32 |
| 7 | Empire (Death Star), day 2350 | 8/45 | 12/25 | day 5000 | 39/23 | 34/23 |
| 8 | day 5000 | 26/26 | 21/30 | day 5000 | 24/43 | 21/32 |
| 9 | Empire (Death Star), day 3241 | 32/24 | 24/23 | day 5000 | 40/25 | 40/26 |
| 10 | day 5000 | 22/50 | 25/42 | day 5000 | 30/34 | 32/21 |

After, no seed ends before day 5000, both sides hold systems on every
seed, and no side exceeds 41 fleets. The seed-42 golden is regenerated
(save v137 and module 9's orders); `a_sweep_run_counts_what_its_days_report`
runs 640 days, as seed 1's first ground battle moved from day 457 to day
635.
