# The trench run's approach moves the group's anchor (P24)

Date: 2026-10-26. Stage 9, the last, of the fighter force mover. Follows
`../2026-10-26-fighter-recovery-flight/`.

Before, the approach moved every member of the run's group by the same
straight step, measured from its first member, at the slowest member's
engine power.

## The original

`FUN_005c6610`'s order 6 branch (the fighter force's slot `+0x14`, each
frame):

- the mover's forward `+0x1c` becomes the side vector turned a quarter
  about y, `(-side.z, 0, side.x)` (`0x005c6848`..);
- while the result object's `+0x2c` is 2 (the approach), the anchor steps
  (`0x005ec0d0`) toward forward × half the extent (battle `+0x8cc` × 0.5)
  with the move's radius 0.2 (`0x3e4ccccd`, `0x005c68f0`). The step runs at
  the mover's speed, the slowest member's (`0x5ba190`);
- on arrival, when the result's latch (vt `+0x10`) is clear, it sets the
  latch (vt `+0xc(1)`) and posts state 5 (`FUN_005cfe80(5)`, `0x005c6924`);
- each squadron flies its own frame (`0x5b8f70`) to its place round the
  anchor (`0x5cb050` → `0x5ebe30`).

## The port

`advance_trench_run_approach`:

- sets the group mover's forward;
- steps its anchor with `step_fighter_anchor` (radius 0.2) at the mover's
  speed;
- posts state 5 on arrival;
- flies the members on order 6 to their places with
  `fly_fighter_squadron`.

The pending-event check in `trench_run_approaching` serves as the latch.

`port:` the anchor's step is flown in 60 Hz frames
(`PORT_MOVER_FRAME_MILLISECONDS`). At 250 ms a step would pass the point
by more than the radius and never arrive. The members fly once per step.

## Changed expectations

These follow from the facts above:

- `a_trench_run_approach_moves_only_its_group_toward_its_side_s_point`:
  - the anchor moves 250 ms at the slowest speed (commander term plus
    sublight) toward the point;
  - the mover faces ∓x;
  - the members on order 6 fly, and the others stay.
- `a_trench_run_flies_its_approach_before_the_run_begins`:
  - at speed 2.0 (1 + 1) the anchor arrives in step 19, not 23;
  - the event is stamped 7, and state 5 runs at turn 8, step 32;
  - state 5 is posted once.

Both fail on the previous code.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2868 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T12-56-39-240Z-446033`) |
| Replay equivalence | passed |
| `cargo mutants` | 18 tested: 14 caught, 1 timeout (caught), 3 missed |
| Release WASM SHA-256 | `47e7cdd32633d21d3ad7b65da50e186efe4f5289d9c9465a869190d1e90904e2` |

The 3 missed mutants:

- **Closed afterwards.** `&&` with `||` in finding the group's mover. The
  test's other group is now group 0, whose mover comes first, and the
  mutant, applied by hand, fails it.
- **Equivalent.** `*` with `/` in the point's y and z. Both forward
  components are 0, so the change gives 0 / h = 0 · h.
- **The timeout.** `>` with `>=` in the frame loop never ends, so it is
  caught.
