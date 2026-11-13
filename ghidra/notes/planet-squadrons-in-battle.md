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
- The army seeds (CMUNEMTB, CMUNALTB, `apply_army_seed`) also drop their
  fighter and capital-ship items. The original distributes them through
  `FUN_0051b1b0`/`FUN_0051b1d0` -> `FUN_0051b1f0` (the last two calls
  of `FUN_0051aa50`), which is not traced here.
