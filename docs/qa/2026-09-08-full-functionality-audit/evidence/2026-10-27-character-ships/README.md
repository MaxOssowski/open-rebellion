# Characters ride on a ship and go adrift when it is lost (P24, save v107)

Date: 2026-10-27. Until now the port kept a fleet's characters on the
fleet itself, so a lost ship took no one with it. Only a fleet destroyed
outright set its characters Adrift (`missions::cast_adrift`). The user
approved the save format change on 2026-10-27 ("Napraw punkt 1").

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`, outside
the repository). The vtable slots were read at their PE file offsets.

## The original

### The container

- A character's container is a capital ship. `FUN_004fe160` lets a fleet
  hold only capital ships (`0x14..0x1c`). `CapShip_vt78` (`FUN_00500b40`)
  gives the character families `0x30..0x40` unlimited room on any ship.
- The route's leg builder `FUN_00552300` weighs the fleet with
  `FUN_00552b10` and then each of its own-side ships
  (`FUN_00502e30(fleet, 1, side)`). A later candidate replaces the kept one
  only when `FUN_0054fbb0` finds it strictly better.

### A ship's loss

`CapShip_vt11c` (`0x00502840`) runs the base `GameObj_vt11c`
(`FUN_004fb520`), which calls each child's slot `+0xb4`. Every character
vtable (`0x0065ca70`, `0x006657f0`, `0x00665be0` and others) holds
`FUN_004ef2e0` there:

```
if (!(Flags78 & OnMission) && !(Flags50 & 8)) {
  if (reason == 0x14) SetAutorouting(1); else Role_SetAdrift(1);
}
```

A battle's loss comes from `FUN_0040cfc0` -> `FUN_00501ae0(-dmg, 0xc, ...)`,
which calls `+0xac` with reason `0xc` once the hull damage reaches the
maximum. The battle's commit (`FUN_0040c780`) only reads each ship's
characters; it does not move them.

After the Adrift step, `CapShip_vt11c` rolls an injury for each character
aboard (reason 1 to `0x10`, `+0x2e8`, `FUN_004ef640` -> `FUN_0053ea00`).
That roll is the next stage.

## The port

- `Character::ship` (save v107) holds the `ShipInstance::id` the
  character rides on.
- `missions::seat_characters`, called from `GameWorld::number_ships`,
  seats a character on its fleet's first living ship when its seat names
  none of the fleet's ships.
- `missions::strand_passengers(world, fleet, ship)` moves the characters
  aboard a lost ship out of the fleet to its system. Each one on no
  mission is set Adrift, and `MissionSystem::send_adrift` then settles it
  or sends it away. `strand_lost_ships_passengers` does this for each
  dead ship of a fleet. Callers:
  - the abstract battle (`integrator::apply_space_combat_result_inner`);
  - the tactical battle (`tactical_flow::apply_results`);
  - `scrap::destroy_ship` (bombardment, sabotage).
- A split (`fleet_join::move_ships_into`) takes the characters aboard the
  moved ships with them.
- The seed-42 replay golden is regenerated for the format change:
  - the initial fingerprint goes from `v1:ba9d8c7e73c0e2cd` to
    `v1:a910734608cb0256`;
  - the final one from `v1:59b9d1aa26541572` to `v1:b1900e6064ece1c3`.

Not exact:
- hyp: the first living ship is the seat. The leg builder keeps the first
  of equal candidates, and every ship has room, but the fields it weighs
  are untraced. A drop onto a ship row in the fleet window names that ship
  in the original; the port does not offer that drop.
- port: scrapping (reason `0x14`, Autorouting) keeps the old behaviour.
  The character stays in the fleet and takes another seat.
- The injury roll for a lost ship's passengers is not ported yet.

## Tests

- `a_destroyed_ships_characters_go_adrift_and_the_others_stay` (scrap)
- `characters_ride_on_the_first_living_ship_of_their_fleet` (world)
- `moved_ships_take_the_characters_aboard_them` (fleet_join)
- `a_lost_ships_characters_go_adrift_and_the_fleet_keeps_the_rest`
  (integrator)
- `a_ship_lost_in_the_battle_sets_its_characters_adrift` (tactical_flow)
- `a_round_trip_keeps_the_ship_a_character_rides_on` (save)

Without the change, the scrap, fleet_join and integrator tests fail. One
fault showed up during the work: seating a character before its lost ship
was removed moved it to a living ship. Seats on any ship still in the
fleet are now kept.

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2983 passed, 0 failed |
| `replay_manifest -- --ignored` (original data) | 2 passed, with the regenerated golden |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 22 tested: 20 caught, 1 unviable, 1 missed |
| Release WASM SHA-256 | `4372896c7454e3654f8cb070b0fa8c2555ec3dc886d24c134a6f91b4b570d299` |

The missed mutant replaces `rebellion-data`'s `convert_character` with
`Default::default()`. The stage added only its `ship: None` line. The
function converts the original DAT records, which the unit tests do not
load; the ignored original-data tests cover it.
