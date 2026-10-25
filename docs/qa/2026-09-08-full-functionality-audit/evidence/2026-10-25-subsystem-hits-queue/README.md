# Subsystem hits go through the turn queue (P24)

Date: 2026-10-25. Part of rebuilding the battle's turns
(`ghidra/notes/tactical-event-turns.md`), after
`../2026-10-25-weapon-shots-queue/`.

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`).

The hit event calls the target's slot `+0xe0`. That is `FUN_005b5f50` for
both capital ships (`0x66c140 + 0xe0`) and fighter groups (`0x66c560 +
0xe0`).

1. **Already destroyed.** With hull `+0x3cc` at 0 or below, it sets 0 and
   returns (`0x005b5f59`..`0x005b5f8b`).
2. **The damage** is at least 1.0 (`DAT_0066c514`). For a fighter group
   (slot `+0x20` = 1) it is divided by the ratio of the two units' slot
   `+0xbc`, not below 0.1 (`DAT_0066c518`). It comes off the shield
   `+0x3c0` at once.
3. **A shield gone below 0** carries into the hull at once (sound `0x11`):
   - **hull at 0 or below:** the unit is destroyed at once
     (`0x005b604a`..`0x005b60b1`: hull 0, explosion `0x005c13d0`, slot
     `+0xd0`, slot `+0x1c`);
   - **hull left, capital ship:** a draw `0..100` (`FUN_005a8a70`) plus
     the hull lost in percent picks a subsystem by 50 (`DAT_0066c520`) and
     the thresholds `DAT_0066c524`..`DAT_0066c51c`. Each choice posts
     event `0x1e`..`0x22` through slot `+0x28`, stamped `+0x87c`
     (`0x005b6151`..`0x005b6228`).
4. **A shield left above 0** has its own subsystem draw (`0x005b625b`
   on). It posts the shield hit as event `0x1e` the same way
   (`0x005b62f6`).
5. **The subsystem events** (`0x005b28d0`, `0x005b28a0`, `0x005b2870`,
   `0x005b2840`, `0x005b2900`) find the ship and call its slot `+0xa0`
   with 0..4: shields, weapons, tractor, engines, hyperdrive. They do not
   ask whether the ship is still in the fight.

## Change

- `hit_weapon_shot` now uses `apply_original_tactical_capital_hit`. It
  takes the shield and hull at once, but returns the subsystem hits.
- Each subsystem hit is posted as `SubsystemHit { ship, kind }` at the
  step's stamp; within a turn's events that is `+0x87c` = turn + 2.
- `hit_subsystem`, the event, adds the hit, cancels a tractor field the
  hit disables, and refreshes the subsystem conditions.
- `apply_original_capital_damage` keeps applying the hits at once. The
  abstract battle (`FUN_005b54d0`) and the fixtures use it, and they have
  no queue.

## Still open

A hull draw above 100 (`DAT_0066c51c`) sets the hull to 0 and posts a
type-3 event with the attacker as its source (`0x005b6235`..
`0x005b624e`). But `0x005b2b60`'s unit-source branch destroys only a
target whose hull is below 0 (`0x005b2c08`..`0x005b2c19`), and the
port's `original_hull_subsystem_kind` returns no hit above 100. What
removes such a ship has not been traced.

## Tests

- New: `a_hit_posts_its_subsystem_damage_through_the_queue`:
  - a laser hit that breaks a 10-point shield takes it at once;
  - its shield hit waits in the queue at stamp 7 (turn 5 + 2);
  - its event adds the hit.
- Changed: the battery test helper `resolve_test_shots` now also runs the
  subsystem events the hits posted, as the queue does after them.
  `live_ship_fire_uses_the_shared_subsystem_damage_path` keeps its
  expectations through it.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2785 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| `cargo mutants` | 10 tested: 5 caught, 4 unviable, 1 missed |
| Release WASM SHA-256 | `17f5531a1f3ac72848b31f40c3fe5bfc59ee25ba999b7f036afd48823ae09b98` |

The missed mutant turns the shield hit's `tractor_disabled |=` into `&=`.
It is equivalent: a shield hit never disables a tractor
(`add_subsystem_hit` returns true only for the tractor), and the flag
starts false, so both forms leave it false.
