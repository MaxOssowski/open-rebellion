# Members climb within the nearby ships' height bands (P24)

Date: 2026-10-26. This ports the vertical step left out of
`../2026-10-26-formation-movers/`. The port moved a member up or down
toward its place and only undid the step when it overlapped a ship.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`, plus
`objdump`; outside the repository).

## The original

### The search

`FUN_005b2f30` (a unit's frame, slot `+0x8`) first resets its state:

- it clears `+0x414` (blocked);
- it returns when its speed is 0;
- it sets the floor `+0x404` to `-FLT_MAX` and the ceiling `+0x408` to
  `+FLT_MAX`.

When the member's distance `d` to its place is exactly zero, it skips the
whole vertical pass (`0x5b39e0`). Otherwise it calls
`FUN_005cdbf0(unit, s·ms·0.001)` on the battle's capital-ship tree
(`+0x8f8`). Fighter groups live in another tree (`+0x900`). The search:

- walks out from the unit on each side of the tree, which is sorted by x;
- stops at the first neighbour whose `|Δx|` exceeds both reaches. A reach
  is 0.5 × speed + 5/9 × planar diameter (vt `+0x8c`, `DAT_0066c534`);
- keeps a neighbour when `|Δz|` is within both reaches and `|Δy|` is
  within both half-heights plus 0.4 of the frame's travel
  (`DAT_0066d03c`).

### The band

- A neighbour above lowers the ceiling to its y less both half-heights.
- A neighbour below raises the floor to its y plus both half-heights.
- A neighbour at exactly the same height is placed afterwards:
  - above when its id is higher and nothing is below;
  - below when its id is lower or equal and nothing is above;
  - with only one side taken, on that side;
  - with both taken, above when the room below exceeds the room above.
- The member is "inside" a limit it already passes.

### The step

The step is `s × 0.4 × ms × 0.001`:

- **Inside both:** held. The code's alternative, a step toward
  `(ceiling − floor) / 2 − y`, cannot run: inside both, the ceiling lies
  below the floor, so `2h > ceiling − floor` always holds.
- **Inside the ceiling only:** it steps down, at most the step.
- **Inside the floor only:** it steps up, at most the step.
- **Otherwise:** it moves toward `d.y`, at most the step, and no farther
  than the ceiling when climbing past a ship above, or the floor when
  diving past a ship below. A move of 0 is held.

When `y` changed, `+0x414` becomes whether the member now overlaps a
nearby ship (`FUN_005b2e60`, against the search's list only), and an
overlap restores `y`. The level move's overlap test also uses only that
list, and does not set `+0x414`.

## The port

- `original_height_band_step` (tactical.rs) is the band and the step.
- `BattleSession::nearby_capitals` is the search. It sorts the battle's
  ships by x, then index, and leaves out destroyed, jumping and
  envelope-less ships.
- `move_force_member`:
  - skips the pass when `d` is zero;
  - steps through the band;
  - tests overlaps against the nearby list only;
  - records `+0x414` in `BattleSession::member_states`
    (`TacticalMemberState::blocked`), for the detour of the next stage.
- `port:` ids are ship indices. The exact tie order of equal x in the
  original tree was not read; the port breaks ties by index.

## Tests

- `a_member_climbs_within_its_height_band`:
  - the free step;
  - the ceiling and the floor, each limiting only its own direction;
  - a floor exactly at its height, which it is not inside;
  - held climbs and pinned members;
  - stepping out of a passed limit;
  - every same-height rule, including equal room.
- `nearby_capitals_stop_at_the_first_beyond_reach`:
  - the walk's stop;
  - a neighbour exactly at the reach;
  - the z and y limits;
  - speed widening the reach;
  - destroyed, jumping and envelope-less ships left out.
- `a_held_climb_marks_the_member_blocked`: a member under a stationary ship
  is held and blocked; with the ship away it climbs and is not blocked.

The new tests use the new functions and state, so they do not compile
without the change.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2841 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T03-27-27-480Z-3486759`) |
| Replay equivalence | passed |
| `cargo mutants` | 138 tested: 101 caught at first, 36 missed, 1 unviable |
| Release WASM SHA-256 | `3c027d67f0e17954348fa0556f7573855a005e8a2df29d6c5cd1158cac9e24e3` |

The first tests kept the member at height 0 and x 0, where `a - b` and
`a + b` agree. With the tests moved off zero and the new cases, a targeted
rerun on the band and the search left 17 of 126 missed. All 17 are
equivalent:

- the dead "fits" branch (its condition and its step): 14 mutants;
- `>` and `<` against 0 for `dy` after the `dy == 0` return: 2 mutants;
- the redundant second same-height rule and the both-sides test, which the
  fallback already settles: 2 mutants. One of these is also counted with
  the 14 above.

One more first-run survivor was a real gap: `ceiling - floor` with
`ceiling / floor`. It now fails the pinned-low case. Applied by hand, the
mutant fails the test.

The resize guard `<` with `<=` on `member_states` is equivalent.
