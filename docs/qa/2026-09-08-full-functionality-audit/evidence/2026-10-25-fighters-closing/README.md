# Fighter groups close on their targets (P24)

Date: 2026-10-25. This follows `../2026-10-25-closing-on-targets/`. A
fighter group ordered to attack, or given a target, used to stay where it
was. It moved only to escort, recover, run the trench or withdraw. Source:
`~/Games/rebexe-decompile/decompiled-typed/FUN_005c6610.c`, outside the
repository.

## The original

`FUN_005c6610` (a fighter force's slot `+0x14`, each frame from turn 3)
handles a force that:

- is past the withdrawal (order 3) and trench-run (order 6) branches;
- has no navigation route under way;
- has a target (`+0x8c`) that is still in the fight (`+0xb0`).

Unless it holds (order 11), it then:

1. takes `d`, the target's position less the anchor. When `d` is zero,
   the side's vector replaces it.
2. heads for `anchor + unit(d) × step`, where the step is the tactic
   object's `+0xc` (slot `+0xc`). The move has radius 2.0 (`0x005ec0d0`,
   the fighter mover's step; corrected 2026-10-26 from `0x005eb1a0`, see
   `../2026-10-26-fighter-forces/`):
   beyond it, a full step at the force's speed (it returns 1); within it,
   it stops (returns 0).
3. on arrival (0), circles the target:
   - the angle `+0xc0` grows by the frame's milliseconds × π × 0.0002
     (`DAT_0066cafc`, `DAT_0066cb20`) and wraps at 2π;
   - side 1 (the Empire) negates the angle each time;
   - the radius is the target's size (`+0xc`) × 2.4 (`DAT_0066cb28`), at
     least 1.5 (`DAT_0066cb08`);
   - the anchor goes to the target plus `(cos r - sin r, r/2, cos r + sin r)`.

A force's tactic object starts as Stand Off (`FUN_005c9db0`: `+0x38` =
`+0x3c` = 2, object `0x66ccd0`), for fighters as for capital ships. Its
`+0xc` is set by `FUN_005edf80` once a turn:

- 5.0 from the constructor (`DAT_0066cc94`);
- the force's reach beyond the target's, when it has one;
- otherwise a third into the best band (`DAT_0066dab8`);
- for an unarmed force, max(target range × 1.2, 10).

Surround's `+0xc` is 5.0.

With the data: an A-wing's lasers reach 12 (FIGHTSD record 1, TEXTSTRA 9792). Against a
capital ship that outranges it, its best band is [0, 12) and its step 4.
So it flies at its target, rather than circling at once.

## The port

- `TacticalEngagementBand` now keeps the object's `+0xc` as `step`
  (restoring the reach computation of the previous stage).
- `stand_off_band` takes any units (capital ships or fighter groups) and
  weighs the target's weapons by the lead unit's type.
- `update_engagement_bands` also covers `fighter_forces`: in the fight,
  not withdrawing, recovering, escorting, holding or on the trench run.
- `advance_fighter_approach` (each step from turn 3, before the moves)
  flies or circles as above. `fighter_orbit_angles` holds `+0xc0`.
- `port:` as in the withdrawal, each fighter group entry is its own force
  and its position the force's anchor. The step is the port's 250 ms.
  The target's size is its collision envelope's planar diameter (0 for a
  fighter group).

## Tests

- `a_fighter_force_flies_at_its_target`: the A-wing-like group's band
  (5, 12) and step 4; no movement before step 12, then 11 × 0.25.
- `a_fighter_force_in_reach_circles_its_target`: the angle, the Empire's
  negation, the wrap, radius 1.5 and 4.8.
- `a_fighter_force_flies_in_three_dimensions`: a diagonal flight, a step
  of exactly 2 already circling, and the Imperial side vector.
- `stand_off_steps_a_third_into_its_band`: step `4 + 8/3` from [4, 12).
- `only_free_fighter_forces_close_on_targets`.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2817 passed, 0 failed (before the last two tests) |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | 136 tested: 114 caught at first; after the last two tests 132 caught, 1 unviable, 3 missed |
| Release WASM SHA-256 | `e1e26afeef50a6b8fea3a25e178dda672390eca24b9b03eee57e90c9c0f75cb0` |

The three missed mutants are equivalent:

- the reach test `reach <= hi && target < hi` with `||`, or with `<=`: a
  later band only replaces the reach with a value at most 0 or equal, and
  only a reach above 0 is used;
- `len() < fighters.len()` with `<=` before the angle list's resize: a
  resize to the same length changes nothing.

