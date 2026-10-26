# Recovering squadrons fly their frames to the carrier's underside (P24)

Date: 2026-10-26. Stage 8 of the fighter force mover. Follows
`../2026-10-26-fighter-escort/`.

Before, a returning squadron flew straight at the carrier's centre at its
engine power. It docked within 2.0 of that centre.

## The original

- **The distance.** `0x5b8f70`'s distance (vt `+0x7c`, `0x5b4960`) goes to
  the fighter force's `0x5c7060`. When the force's order `+0x64` is 2
  (Recover) and the member's carrier `+0x164` is set:
  - `FUN_005a6360` returns the carrier's `+0x418` sub-object (vtable
    `0x66c110`);
  - that sub-object's vt `+4` (`0x5b0b80`) returns (`+0x30`, `+0x34` −
    `+0x8`, `+0x38`): the ship's position lowered by its vertical radius,
    the hull's underside;
  - d is the raw difference to it, with no 2.0 snap.
  
  Otherwise the base `0x5cb050` gives the place delta (`0x5ebe30`). The
  squadron then turns and flies as in any frame: its speed, its alignment
  and the height bands.
- **The anchor.** Order 2's install (`FUN_005ca6d0`, case 2) clears the
  route `+0x74` and the target `+0x8c`, so `FUN_005c6610` neither steps
  nor circles. The anchor stands.
- **Docking.** `FUN_005cf980`'s second loop works only in state 2:
  - a member within 2.0 (`DAT_0066d088`) of its carrier's underside enters
    state 3 and posts event `0x1a`;
  - `FUN_005b8630` then sets state 4.
- **The carrier.** `+0x164` is first the home carrier (`FUN_005b84a0`, at
  deployment). The reserving carrier replaces it when event `0x2c` lands
  (`FUN_005b1e10`).

## The port

- `advance_recovering_fighters`, each step from turn 3, before the capital
  moves:
  - a Recover squadron in state 2 flies `fly_fighter_squadron` toward its
    carrier's underside (`carrier_underside`);
  - one awaiting a carrier flies to its place round the anchor;
  - the anchor is not moved.
- `advance_original_fighter_recovery` no longer moves anything. In state 2
  it docks a member within 2.0 of the underside.
- `port:`
  - a squadron flies to the carrier that reserved it; the home carrier is
    not kept, so before a reservation it flies to its place;
  - `FUN_005cf980`'s docking pass is taken to come before the move, so a
    member already within 2.0 does not fly away first. The executor's
    place in the frame was not traced.

## Tests

- `a_returning_squadron_flies_its_frame_to_the_carrier_underside`:
  - an off-axis carrier with vertical radius 3;
  - the move matches the turn toward the underside at speed × alignment;
  - the anchor stands.
- `a_squadron_docks_within_two_of_the_carrier_underside`:
  - 1.5 below the underside docks, while the centre is 4.5 away;
  - at the centre it does not dock;
  - 1.9 and 2.1 to the side;
  - only state 2 docks.
- `a_squadron_with_no_carrier_flies_to_its_place`, and a squadron that is
  not recovering is not moved.
- `a_recovery_flies_only_squadrons_still_on_their_way`: a carrier of the
  other side resets the state, exactly 2.0 off still flies, within 2.0 it
  waits, and a withdrawing squadron does not fly.

Changed expectations, from the facts above:

- `the_ai_sides_fighters_withdraw_by_their_hyperdrive`:
  - the reserved squadron has flown round its place until then, so it is
    still in state 2 when reserved;
  - it docks later, which the test checks on a copy of the session.
- `waypoint_arrival_and_fighter_docking_complete_source_executors`
  (render):
  - the squadron starts 2.5 behind the carrier, facing it;
  - it flies 1.25 (speed 5 × 0.25 s) and docks.

- The browser fixture `configure_command_progression_fixture` and its
  probe in `tools/interface-parity/run.mjs`
  (`production-command-progression-presentation`):
  - the returning squadron starts 2.5 behind the carrier's underside,
    facing it;
  - after one frame the probe expects it 1.25 from the carrier, not 1.5.

The four tests fail without the change.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2868 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T12-41-52-906Z-416310`) |
| Replay equivalence | passed |
| `cargo mutants` | 28 tested: 19 caught, 1 unviable, 8 missed (first run: 7 missed, closed by `a_recovery_flies_only_squadrons_still_on_their_way` and a simpler filter) |
| Release WASM SHA-256 | `1d61887b2e96e71d751fad5f73722d7ffe1ea3b786dc2619805707c136cdf5c0` |

Of the 8 missed mutants in the final run:

- **Closed afterwards.** `<` with `<=` in the flight's docking gate fails
  the "exactly 2.0 off still flies" case, checked by hand.
- **Covered only by the browser matrix.** The other 7 are in the fixture
  `configure_command_progression_fixture` (the call itself and the
  start's y and z arithmetic). `production-command-progression-presentation`
  checks them.
