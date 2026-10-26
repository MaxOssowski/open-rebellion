# Withdrawing fighter groups fly on their anchor (P24)

Date: 2026-10-26. Stage 2 of the fighter force mover, following
`../2026-10-26-fighter-forces/`.

## The original

`FUN_005c6610`, the fighter force's order step, on order 3 (withdraw):

- sets the mover's forward to the reversed side vector;
- steps the anchor toward its own x and y at the exit line's z
  (`0x005ec0d0`, radius 1.0);
- tries the members' jumps (within 8.0 of the line, event `0x2d`) only
  once that step returns 0, the anchor having arrived.

The members fly to their places from the anchor.

## The port

- `advance_withdrawing_fighters` steps each withdrawing group's mover once
  a frame (`step_fighter_anchor`, `ORIGINAL_WITHDRAWAL_ARRIVAL_RADIUS`) and
  tries the jumps only after its arrival. Before, each squadron flew
  alone.
- `port:` the members stand at their places (the anchor less the slot)
  until the squadron frame `0x5b8f70` is ported, as in stage 1.
- The trench run already moves its participants together at the slowest
  speed, and the port's fighters have no routes, so those branches wait
  for later stages.

## Tests

- `a_withdrawing_fighter_group_flies_on_its_anchor`:
  - the anchor steps 4 × 0.25 from the lead toward the line;
  - the second member stands at its place, in all three axes;
  - no member jumps while the anchor is still under way, even the one
    already at the line.
- Changed expectations, with the original's one force per group as their
  evidence:
  - `withdrawing_fighters_head_for_their_own_edge` puts each squadron in a
    group of its own, so each flies alone as the test means;
  - `the_ai_sides_fighters_withdraw_by_their_hyperdrive` and
    `a_withdrawal_holds_fighters_with_a_hyperdrive_then_they_jump` move the
    group's anchor too when they put a squadron on its exit line by hand.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2854 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T09-49-24-297Z-2855920`) |
| Replay equivalence | passed |
| `cargo mutants` | 13 tested: 10 caught, 3 missed |
| Release WASM SHA-256 | `16610474295e858739771917ca8b4fe24631c1b4114d87cfe76dc92f99e60d3f` |

Of the 3 missed:

- the place's `y` (2 mutants) was a gap. The test now checks `y`, and the
  mutant, applied by hand, fails it;
- the place's `z` with `+` is equivalent, as the slot's z is always 0.
