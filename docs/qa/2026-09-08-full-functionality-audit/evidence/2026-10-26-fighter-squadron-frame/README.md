# Fighter squadrons fly their own frame (P24)

Date: 2026-10-26. Stages 3 and 4 of the fighter force mover
(`../2026-10-26-fighter-forces/`, `../2026-10-26-fighter-withdrawal-anchor/`).
Squadrons no longer stand at their places: each one turns and flies.

## The original

The fighter class `0x66c028` has its own frame, slot `+0x8` = `0x5b8f70`:

1. **Distance.** `d` comes from force slot `+0x18` (`0x5c7060`):
   - on order 3 (withdraw), the distance along z to the exit line, and a
     squadron at the line does not move;
   - otherwise the mover's slot `+0x24` (`0x5ebe30`): the place (the anchor
     less the slot) less the position. Within 2.0 of the place (squared
     4.0, `DAT_0066d864`) it is the mover's velocity instead.
2. **Heading** (`0x5b9a20`): `d` in three dimensions when it is not zero;
   else the bearing to the force's target; else the mover's forward.
3. **Turn** (`0x5b97a0`), with no dead band:
   - `q = rate × ms / (angle × 1000)`;
   - at 1 or more, the forward snaps onto the heading;
   - otherwise the level part turns by `acos(f.x·h.x + f.z·h.z) × q`,
     signed by `h.z·f.x − h.x·f.z`, the height moves `q` of the way, and the
     forward is renormalized.

   The rate is set in `FUN_005b9c60`: π/12, or π × min(m + 1, 9) / 18. That
   is the capital ships' formula.
4. **Alignment:** `f·h`, at least 0.25 (`DAT_0066c6c4`). So a squadron
   always moves, even facing away.
5. **Move:** velocity = forward × speed × alignment, in three dimensions.
   No snap, no cap at the mover's speed, no overlap undo.

## The port

- `BattleSession::fighter_frames` holds each squadron's forward and
  velocity. A squadron starts facing its side, as every unit does at
  placement (`FUN_005ab650`).
- The routines:
  - `turn_original_fighter` is `0x5b97a0`;
  - `fighter_place_delta` is `0x5ebe30`;
  - `fly_fighter_squadron` is `0x5b8f70`.
- Where they run:
  - `advance_fighter_approach` flies each free member toward its place
    after its force's anchor step;
  - `advance_withdrawing_fighters` flies each withdrawing member along z
    to the line, after the anchor's step and the jump checks.
- `port:`
  - a squadron's speed is still its sublight rating (stage 5);
  - the height bands are not applied yet (stage 6);
  - recovery, escort and the trench run keep their own movement
    (stage 7).

## Changed expectations

These tests had asserted squadrons standing on the anchor or flying
straight:

- The tests of `FUN_005c6610`'s step (`a_fighter_force_flies_in_three_dimensions`,
  `a_fighter_force_in_reach_circles_its_target`,
  `a_withdrawing_fighter_group_overshoots_its_line_by_a_full_step`) now
  read the force's anchor, which that step moves.
- The withdrawal tests (`withdrawing_fighters_head_for_their_own_edge`,
  `a_withdrawing_fighter_group_flies_to_its_edge_before_it_jumps`) start
  their squadrons facing their exits (`face_exits`), so each flies at full
  speed with no turn.
  - In the second, the squadron now flies on to the line itself while the
    anchor stops within 1.0 of it, and it jumps from the line.
- `a_fighter_group_moves_as_one_force` now expects the lead to fly at its
  own speed, and the member to close on its place.
- `a_withdrawing_fighter_group_flies_on_its_anchor` now expects the member
  already at the line to hold.

## Tests

- `a_squadron_turns_by_its_share_of_the_angle`:
  - the snap and the quarter turn, both ways;
  - a climb, whose level part still turns by `acos(0) × q`, as the routine
    reads;
  - a tiny angle turning, since there is no dead band;
  - rate 0.
- `a_squadron_flies_its_forward_at_no_less_than_a_quarter_speed`: a
  squadron facing away moves on at a quarter speed while it turns; within
  2.0 of its place it heads along the mover's velocity.
- `a_squadron_turns_and_flies_off_the_axes`:
  - turns from general forwards, against values worked out independently
    from the same formulas;
  - a member flying a diagonal to its slot place;
  - `0x5ebe30` at exactly 2.0 and just beyond;
  - the place on every axis.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2857 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T10-12-57-358Z-2962651`) |
| Replay equivalence | passed |
| `cargo mutants` | 98 tested: 68 caught at first, 27 missed, 3 unviable; after the off-axis tests 89 caught, 6 missed; the two cases added last kill 2 of those, each checked by hand |
| Release WASM SHA-256 | `7ec1b117f7a60b06fe170cd7f8962eb9638a0de33030ea1c4e7e774df564db49` |

The first tests flew mostly along z, where `f.x` = 0 and `f.z` = 1, so
multiplying and dividing by them agreed. The 4 survivors left are
equivalent:

- the early return's `||` with `&&` (2 mutants): for a unit forward, a zero
  angle, rate or time gives the same forward either way;
- the frame list's resize guard `<` with `<=`;
- the place's z, `anchor.z − offset.z` with `+`: the slot's z is always 0.
