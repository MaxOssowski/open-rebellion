# Capital forces close on their targets (P24)

Date: 2026-10-25. Until now the port moved capital ships only along
waypoints: maneuvers, navigation routes and the withdrawal. A ship ordered
to attack, or given a target by the AI's commander, stayed where it was
placed and fired only at what was already in range. The lanes are an
extent apart, at least 103, so most battles never closed. The trace is in
`ghidra/notes/tactical-collision-formation.md`, "Closing on a target",
from `FUN_005cc300`, `FUN_005edf80`, `FUN_005b49e0` and `FUN_005b7e50`.

## The original

Each frame from turn 3, a capital force with targets (`FUN_005cc300`,
slot `+0x14`) does nothing while its navigation route is under way or
when it holds (order 11). Otherwise it measures `r`, the distance from
its formation anchor to its first target, and compares `r + 2` with a
band [min, max] from its tactic object:

- Inside the band the mover stops.
- Stand Off closer than min holds while the target attacks one of the
  force (`0x005ca4b0` on `+0x1c`). Otherwise it backs off along its
  side's vector times `min × -1.1`.
- Otherwise it heads for the anchor plus the unit direction times a step,
  with radius 2:
  - Surround: 5.0;
  - Stand Off: `min + k (max - min)`, with k = 0.5, 0.75 or 0.9 by size,
    at least 2.

The tactic object recomputes its band once a turn (`0x005a9d40` -> force
slot `+0xc` -> object slot `+4`):

- Surround: 0 to 2.5.
- Stand Off (`FUN_005edf80`):
  - the force's weapons against its target add their worth on [0, range),
    and the target's weapons against the force's lead unit subtract
    theirs (`FUN_005b49e0`, worth = count × slot `+0x60` × strength ×
    modifier for each family of the strongest arc);
  - the bands split at every range (`FUN_005b7e50`);
  - the worthiest band within the force's own reach is taken (the later
    of equals);
  - min is at least 5 and an empty max is 10;
  - an unarmed force keeps max(target range × 1.2, 10) to 1e6.

## The port

- `update_engagement_bands` (once a turn, after the commanders) and
  `advance_engagement_approach` (each frame, before the moves) follow these
  rules for every capital force: each group slot by side, and each force of
  its own.
- The lead unit is the formation's (capability order). It takes the goal
  as its waypoint, and the rigid formation carries the members.
  (Superseded in `../2026-10-26-formation-movers/`: the force's mover
  anchor takes the goal and the members steer to their places.)
- Inside the band, or holding, the goal and heading are cleared.
- On arrival a waypoint now ends only a maneuver (`FUN_005a8f70`). An
  attack order outlives its route. Before, every order but Escort was
  cleared at a route's end.
- `port:` the object's `+0xc` (the reach beyond the target's) was not kept
  at first, since Stand Off's capital step does not use it; fighters use it
  (`../2026-10-25-fighters-closing/`). Escort keeps its own movement.

## Not ported

- The AI's switch to Surround for a force of more than 7
  (`0x005c9be0(1)`, under a `+0x34` condition not read).
- Fighter groups closing on targets: since `../2026-10-25-fighters-closing/`.

## Tests

- `a_force_s_band_follows_its_tactic_and_the_weapons_on_both_sides`
- `a_force_closes_on_its_target_and_stops_in_its_band`
- `a_stand_off_force_backs_off_unless_its_target_attacks_it`
- `an_attack_order_outlives_its_route`
- `a_unit_s_weapons_are_bands_worth_their_firepower`
- `stand_off_takes_the_worthiest_band_within_reach`
- `the_forces_and_their_targets_are_those_in_the_fight`
- `the_approach_step_aims_exactly`
- `an_imperial_stand_off_force_backs_off_toward_its_side`

No existing test changed.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2814 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | first run 121 tested, 50 missed; after the tests above, 114 tested, 106 caught, 2 unviable, 6 missed, of which 1 was then caught |
| Release WASM SHA-256 | `27679d2d60a38b63d698387c216c8080893f057d1b952e930ed376e3b7752938` |

The browser matrix ran before the unused reach computation was removed,
which changes nothing that runs. The WASM above was rebuilt afterwards.

The remaining mutants:

- `task_force < 8` to `<= 8`: a `task_force` is 0..7 or `u8::MAX`.
- `-5 * side`, `side * min * -1.1` with `+` or `/`: the vector is
  normalized next, and the sign is the same for both sides.
