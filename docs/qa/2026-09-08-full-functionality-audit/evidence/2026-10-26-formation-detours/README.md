# A held member of a still formation takes a detour (P24)

Date: 2026-10-26. This ports the last part of the formation mover left out
of `../2026-10-26-formation-movers/`. A member held in its climb, as
`../2026-10-26-height-bands/` records it, could stay stuck behind a ship
for good.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`, plus
`objdump`; outside the repository). Two points of the first reading were
wrong:

- the timeout is 12 turns (`DAT_0066d858` = 12.0), not 2.5 s;
- "blocked" comes only from the vertical step, not from the level move.

## The original

`FUN_005ea740`, the mover's slot `+0x40`, runs once a turn for both
tactics. Surround's `0x5ebda0` jumps to it. Its member loop runs before
`+0x134` is updated.

1. A member is skipped unless its speed (vt `+0x90`) is above 0.
2. It takes a detour when all of these hold:
   - it is held (`+0x414` = 1);
   - the mover's velocity `+0x78` is zero;
   - `+0x134 ≥ 5`;
   - `+0x87c − +0x410 > 12`.

   The extra delta test that follows compares a squared length with 0 and
   never fails.
3. Taking it:
   - `+0x410` = now;
   - its old record is removed;
   - `+0x40c` = `(id + 1 + +0x40c) & 3`, where the id is `+0xec`;
   - a record with offset (20, 0, 0), (−20, 10, 5), (5, 0, 20) or
     (0, −10, −20), by index, goes into the mover's `+0xec` list
     (`FUN_005e9cc0`).
4. Otherwise, when it is not held, 12 turns have passed and a record
   exists, the record is removed. A member still held keeps its record.
5. `0x5eb040` gives a member with a record its raw offset as the distance,
   without the 1.0² zeroing. The member steers along it every frame.
6. Stand Off's turn (`0x5eae40`) calls `FUN_005ead90` on both lists, so
   detours turn with the slots. Surround's turn turns neither.
7. `+0x40c`, `+0x410` and `+0x414` start at 0 (`FUN_005b0f70`).

## The port

- `TacticalMemberState` gains `detour_index` (`+0x40c`) and `detour_stamp`
  (`+0x410`).
- `BattleSession::force_detours` holds the records
  (`TacticalForceDetour`: force, ship, offset). A rebuilt or vanished
  mover drops its force's records.
- `update_force_detours` runs in `update_force_movers`, once a turn,
  before `+0x134` moves on. Its clock is `step_event_stamp()`, which is
  `+0x87c` as the turn's updates see it.
- `step_force_anchor` keeps the angle of Stand Off's last turn in
  `slot_turn`. The frame turns the force's detours by it.
- `move_force_member` takes the record's offset as its distance.
- `port:` a ship's id is its index + 1, its object id elsewhere in the
  port.

## Tests

- `a_held_member_of_a_still_formation_takes_a_detour`:
  - the five still turns, counted before the update;
  - the index sequence for ids 1, 3, 4 and 6, which covers all four
    offsets;
  - no change at 12 turns and the next detour at 13;
  - removal 12 turns after an unblock, and only of that member's record;
  - a moving formation and a member without engine power take none.
- `a_detour_steers_the_member_and_turns_with_the_slots`:
  - the raw offset moves and lifts a member that stands at its place;
  - another member of the force stays put;
  - Stand Off's turn turns the offset;
  - a rebuilt mover drops it.

The new tests use the new state, so they do not compile without the
change.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2843 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T04-13-06-263Z-446501`) |
| Replay equivalence | passed |
| `cargo mutants` | 64 tested: 53 caught at first, 10 missed; after the wider tests 62 caught, 1 unviable, 1 missed |
| Release WASM SHA-256 | `c137da02229ffa873ebbaac758c4ae7035abaf8c94ff7f4b8a4003a4eba7095c` |

The first survivors were test gaps:

- two offsets never produced;
- `id + 1 + last` against `id + 1 − last`;
- the force and ship tests in record lookup and removal;
- the rotation's z terms.

The tests now cover them. The one left, `<` with `<=` on the state list's
resize guard, is equivalent.
