# A tactical hit on a capital ship draws once (P24)

Date: 2026-10-28. This follows `../2026-10-28-tactical-sound-events/`.
While tracing the hit sounds, it turned out that the port's tactical hit on
a capital ship followed the abstract battle's damage (`FUN_005b54d0`, whose
callers are `FUN_005aefc0`, `FUN_005af7a0` and `FUN_005d14d0`), not the 3D
battle's damage slots.

## The original

A capital ship's laser, turbolaser and torpedo slots (`FUN_005b5100`,
`FUN_005b5f50`, `FUN_005b5750`) take the damage, at least 1, off the
shield:
- **The shield holds** (what is left is not below 0). A draw of 0 to 100
  at most its share lost (at least 1) posts a shield hit (`0x1e`).
- **Through the shield.** The shield is 0, and the rest comes off the hull.
  - A ship left with hull draws 0 to 100 and adds the hull's change in
    percent. Above 50 it posts a subsystem hit by band: 65 shields, 80
    weapons, 90 tractor, 95 engines, 100 hyperdrive.
  - A destroyed ship draws nothing.

At most one draw per hit, and only for a unit whose `+0x124` is 0, which in
a single-player battle is every unit (`../2026-10-28-tactical-sound-events/`).

The abstract battle's `FUN_005b54d0` draws twice on a shot through the
shield: once for the shield it had, once for the hull. It draws even when
the shot destroys the ship.

## The port

- `apply_original_tactical_capital_hit` follows `FUN_005b5f50` and takes
  the stream. The abstract battle keeps `apply_original_capital_damage`
  (`FUN_005b54d0`).
- Tests:
  - `a_tactical_capital_hit_draws_once_and_not_when_it_destroys`: the
    draws per branch, the share lost, the bands, a ship already lost;
  - `a_capital_hit_sounds_the_shield_while_it_holds`.
- `live_ship_fire_uses_the_shared_subsystem_damage_path` had expected a
  shield hit from a shot through a shield of 10, which is the abstract
  model. It now gives the shield exactly the shot, so the 100% share lost
  hits and the hull is untouched.

## Gates

- Browser `--battle --all`: 148/148 passed.
- Workspace: 3010 tests passed. Provenance scan: no uncited items.
- Replay equivalence and the replay manifest passed.
- Mutants on `tactical.rs` (32): after the added cases, 26 caught and 6
  unviable.
