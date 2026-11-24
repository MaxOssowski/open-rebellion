# Planet squadrons in a space battle

Date: 2026-11-13. Sources: `~/Games/rebexe-decompile/decompiled-typed/`
(outside the repository), the DAT files in `data/base`. Follows
`planet-squadrons.md` ("Battle", left untraced there) and
`tactical-chase-camera-distance.md` (the Coruscant extent 136).

## Which units a battle takes (`FUN_0040b0e0`)

Both battle kinds use one build. `FUN_0040a050` state 7 (tactical) and
state 8 (abstract) both call `FUN_0040afa0` and `FUN_0040b0e0`
(`battle-flow.md`). The system is the battle object's `+8`.

1. The commanders: `FUN_00509460` / `FUN_00509530` for sides 1 and 2.
2. Each fleet at the system, both sides, mode 3 (existing)
   (`FUN_004ffe70(system, 3)`, `0x40b3ee`):
   - its capital ships (`FUN_005001b0`) -> `FUN_0040b8a0`;
   - its Death Stars, families `0x18..0x1b` (`FUN_005003d0`) ->
     `FUN_0040bcd0`; then one more Death Star check through the
     ManuMgr (`FUN_00509670`, `0x40b4d2`);
   - for each capital ship (`FUN_00502db0`):
     - the squadrons aboard (`FUN_005039d0(ship, 1)`) -> `FUN_0040be90`;
     - its people (`FUN_004f25c0`) -> `FUN_0040c200`;
     - its regiments (`FUN_00504040`) -> `FUN_0040c4b0`;
     - its special forces (`FUN_00504c40`) -> `FUN_0040c5f0`.
3. Then the system's own squadrons: `FUN_005039d0(system, 1)` (`0x40b7d5`),
   families `0x1c..0x1f`, mode 1 (usable: existing, completed, not en
   route). The walk has no side filter, so it takes both sides. Each one
   goes to the same `FUN_0040be90` as a carried squadron.

Nothing else at the system joins:
- no defence facility, regiment or person on the planet;
- the only planet read is the Death Star shield, `FUN_0051c270(system, 1)`
  in `FUN_0040bcd0`.

The walk order is: every fleet's units first, then the planet squadrons.

## Where a planet squadron starts (`FUN_0040be90`)

`FUN_0040be90` reads the squadron's parent (`FUN_004f6b50`).

- Its type is tested against `0x14..0x1c`, a capital ship (`0x40bf0b`).
  Only a capital-ship parent sets the "aboard" argument of
  `FUN_005964c0`. The carrier's tactical id is then read through
  `FUN_004f6df0`.
- A planet squadron's parent is the system. So the argument is 0, and
  the squadron starts deployed, with no carrier.
- It counts in the battle extent (`FUN_005ab650`: every unit not in
  state 1, `tactical-chase-camera-distance.md`).

## How the losses come back (`FUN_0040c780`)

The result pass walks the same objects again: every fleet's capital ships
and their squadrons, then `FUN_005039d0(system, 1)` (the loops at
decompile lines 166..177 and 225..235). Each squadron takes two
steps.

- **`FUN_0040d260`, the count.** The module's count for the squadron
  (`FUN_005968e0`) is compared with the strategic count (`+0x244` less
  `+0x254`).
  - If it is above, `FUN_00503400` adds the difference.
  - If it is below, `FUN_00503370(n, 0xc)` removes the difference with
    reason `0xc`.
  - A wiped-out planet squadron is destroyed.
- **`FUN_0040d320`, the parent.**
  - A squadron the module saw dock (`local_24`, the carrier in
    `FUN_00596870`) moves aboard that carrier, if the carrier is on its
    side (`+0x24 & 0xc0`).
  - Otherwise, with no parent found (`FUN_004ece60` gives 0), it goes to
    the battle's system.
  - So a planet squadron that did not dock stays on the planet. One that
    docked on a carrier leaves with it.

`FUN_00516ef0` then counts each side's fleets and planet squadrons for the
end of battle (`planet-squadrons.md`, "Battle").

## Where Coruscant's planet squadrons come from (the seed)

The setup driver `FUN_005136d0` stage `0x12` is `FUN_0051aa50`. It runs
the fixed seeds:

| Function | Container | Table |
|---|---|---|
| `FUN_0051acb0` | system `0x92000121` (Yavin) | `0x50` (GNPRTB 5130..5131) |
| `FUN_0051ad90` | the Alliance HQ system | `0x51` (GNPRTB 5132..5133) |
| `FUN_0051ae20` | system `0x90000109` (Coruscant) | `0x52` (GNPRTB 5134..5135) |
| `FUN_0051af00` | a new fleet `0x08000004` at Yavin | `0x53` (`FUN_0055d5b0`) |
| `FUN_0051b080` | a new fleet `0x08000004` at Coruscant | `0x54` (GNPRTB 5138..5139) |

Each one calls `FUN_0051be90(container, table, first, last)`, and for each
group `FUN_0051bf30`:

- the group's first item is created in the container (`FUN_004f7d50`);
- each later item is created in that first object (`FUN_004f7cf0`).

If the first item is null (`FUN_004f26e0` gives 0), the later items go
into the container itself. A garrison group's first item is null
(`seeds.rs`, `apply_garrison_seed`), so its items are created in the
system:

- CMUNCRTB, the Coruscant garrison: 6 regiments `0x10000006` and 10 TIE
  Fighter squadrons `0x1c000005`;
- CMUNYVTB, Yavin: 2 regiments, 1 X-wing (`0x1c000003`), 1 Y-wing
  (`0x1c000004`);
- CMUNHQTB, the Alliance HQ: 3 regiments, 4 X-wing and 2 Y-wing squadrons.

A fleet group's first item is its capital ship. Its squadrons are created
aboard that ship: CMUNEFTB, the Imperial Star Destroyer with 6 TIE
squadrons aboard.

These agree with A0's day-2 Empire save (manifest TAC-02-C004,
TAC-05-C009): the Judicator with its TIE squadrons aboard, and 11 TIE
squadrons on Coruscant.
- hyp: 10 of the 11 are the seed; the eleventh was built or moved by
  day 2.
- The table-id-to-file mapping (`0x52` = CMUNCRTB) is read from the
  files' contents and the port's existing routing. It was not traced to
  the DAT loader.

## Port (2026-11-13)

- **Battle build.** Campaign battles already take the planet squadrons,
  in the tactical and the abstract mode.
  - `squadrons::scramble` (main.rs combat trigger,
    `rebellion-data/src/simulation.rs`) moves each side's planet
    squadrons into its first fleet at the system. With no fleet, they go
    into a group of their own. `squadrons::recover` lands the survivors
    after the battle.
  - Unboarded (`carrier: None`), so `expand_fleet` starts them deployed
    with no home carrier, as `FUN_0040be90` does.
  - `commit_squadrons` / `recover` give back the count (`FUN_0040d260`)
    and the planet as the parent (`FUN_0040d320`).
- **Seed (fixed).** `apply_garrison_seed` dropped every fighter item
  (`dispatch_ground_item` routes only regiments and special forces). A
  new game had no planet squadrons at Coruscant, Yavin or the HQ. It now
  lands them on the planet (`System::fighters`).
  - Test: `a_new_game_lands_the_garrison_tables_fighter_squadrons_on_their_planets`.
- **Fixture.** The tactical test fixture entered battles through
  `begin_player_battle` without the scramble. `coruscant_chase_fleets`
  therefore put A0's 11 TIE squadrons into the home fleet as a stand-in.
  - Now they stand on Coruscant's planet: the 10 seeded plus one.
  - The fixture's entry scrambles them as the campaign's does, and
    `battle_sorties` lands them afterwards.
  - The tactical session is unchanged, field for field, at the browser
    seed 0. That covers the layout, every ship and fighter, their
    positions, groups and hangar state.

port:
- The walk order differs. The original takes every fleet's units and
  then the planet squadrons. The port appends a side's planet squadrons
  to that side's first fleet, ahead of its later fleets, and orders the
  attacker's fleets before the defender's.
- A planet squadron that docks on a carrier during the battle stays in
  its sortie fleet. `recover` lands it again by class count.
- The army seeds are traced below (2026-11-24) and ported.

## The army seeds (2026-11-24)

`FUN_0051aa50` ends with `FUN_0051b1b0` (side 1, the Alliance) and
`FUN_0051b1d0` (side 2), both `FUN_0051b1f0(side, ...)`:

- **Budget.** `FUN_0055d670` (Alliance) / `FUN_0055d6d0` (Empire) take
  the side's `SideMaintState - MaintCapacityAllocated` (side `+0x58` less
  `+0x5c`, the unallocated maintenance capacity) and the galaxy size
  (`+0x5c`, 1..3), and return it times SDPRTB 5168/5169/5170 over 100
  (`FUN_0053e190` -> `FUN_0053e170` -> `FUN_0053e150`, divisor
  `DAT_00661a88`).
- **Group.** `FUN_0055d730` / `FUN_0055d780` roll `FUN_0053e2c0(1, 100)`
  and look it up in seed table 3 / 4 (`FUN_0055bed0` -> `FUN_0058b7e0` ->
  `FUN_00595090`): the walk stops at the first group whose threshold
  (`+0x20`, the DAT group's third field) exceeds the roll and returns the
  group before it; past every threshold, the last. CMUNALTB: 1-8 ship
  0x41, 9-12 ship 0x44 with an X-wing and a Y-wing, 13-26 ship 0x45, 27-35
  ship 0x46 with two regiments, 36-39 an X-wing, 40-53 a Y-wing, 54-57 and
  58-88 regiments, 89-100 special forces.
- **Cost.** `FUN_0051be20`: the group's count (`+0x24`, 1) times the sum
  of its items' class slot `+0x24` (hyp: maintenance). The cost comes off
  the budget first; the first group that takes it below zero ends the
  loop, uncreated.
- **Place.** A system of the side picked uniformly (`FUN_0053e290` over
  the count of family `0x90..0x98` objects with the side's bits).
- **Container.** If the group's first item is a capital ship (families
  `0x14..0x1b`), `FUN_004ffef0(system, 4, side)` walks the side's fleets
  there and takes the first, or `FUN_004f7d50` makes a fleet `0x08000004`;
  `FUN_0051bf30` then creates the ship in it and every later item
  (squadrons, regiments, special forces) inside the ship. Any other group
  goes to `FUN_0051bf30` with the system: a lone squadron is the planet's.
- So no seed makes a fleet of squadrons alone, and A0's eleventh TIE
  squadron on Coruscant on day 2 fits a CMUNEMTB group 6 landing there.

Port (`seeds::seed_maintenance_budget_units`, `create_group`): as above.
The port's Yavin/HQ placement of the whole army tables (`apply_army_seed`,
from the community `seed.js`) is gone from the main paths. The seeded
regiments aboard wait in `GameWorld::seeded_cargo` until the campaign's
`TroopTransportState::take_seeded_cargo`. The fixed fleets (CMUNEFTB,
CMUNAFTB) now carry their regiments aboard as well; they were dropped.
port: the facilities' sides are set before the budget is read
(`assign_facility_sides`), as the original creates them with their
system's side.
