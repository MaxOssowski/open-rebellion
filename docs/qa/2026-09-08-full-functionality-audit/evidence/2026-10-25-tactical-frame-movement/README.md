# The battle's frame moves: from turn 3, within the field (P24)

Date: 2026-10-25. Third stage of rebuilding the battle's turns.

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`).

- The manager's frame function calls slot `+0x98` = `0x005a9ec0` with
  the frame's milliseconds (`0x005bef6b`). It does so only once `+0x878`
  has reached `+0x8ac` (3) and the sub-state `+0x8b4` is 4
  (`0x005bef1d`..`0x005bef33`), so the units do not move for the first
  three turns.
- `0x005a9ec0` walks the units (`+0x988`):
  - **State 2 (in the fight):** it calls the unit's slot `+0x8`. That is
    `FUN_005b2f30`, velocity and `FUN_005cd640`'s position, for both
    capital ships (`0x66c140`) and fighter groups (`0x66c560`). Then it
    holds each axis of the position within `+-2/3` of the battle extent
    `+0x8cc` (`DAT_0066c290` = −0.6667, `DAT_0066c294` = 0.6667;
    `0x005aa089`..`0x005aa17a`).
  - **State 7:** `FUN_005a6b40` flies the jump. When the flight is done
    it sets state 8 and posts event `0x2e` (`0x005aa2a3`..`0x005aa2e5`),
    as the port does since `../2026-10-25-tactical-event-queue/`.
- So movement is per frame, as the port's 250 ms step already ran it.

## Change

- The escort and unit movement start at turn 3, step 12.
- After its move, each ship and fighter group in the fight is held within
  `+-2/3` of the battle extent on each axis
  (`ORIGINAL_FIELD_LIMIT_EXTENT_SCALE`). A unit in its jump flight is not
  held.
- `port:` the fighter recovery still runs every step from the start. Its
  states belong to `FUN_005cf980`, a task-force order step that runs once
  a turn; its flight is not yet split from them.

## Tests

- `a_withdrawing_ship_drops_its_attack_and_heads_for_its_exit`: its
  expectation changed on this evidence. No waypoint before step 12, then
  the exit line.
- `units_left_behind_are_destroyed_at_the_sides_end`: the staying carrier
  is now placed far off. The test is about a group still on its way to it;
  with nothing moving for three turns, the group had docked on a carrier
  at the same point.
- New:
  - `nothing_moves_for_the_first_three_turns`;
  - `a_unit_in_the_fight_is_held_within_the_field`, which also pins the
    float at `0x66c294`, bits `0x3f2aaaab`.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2778 passed, 0 failed |
| Provenance scan | 0 uncited items (after naming `DAT_0066c294` in the constant's comment) |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| `cargo mutants` | 15 tested, 13 caught; the two misses on `2.0 / 3.0` are now caught by the bits check |
| Release WASM SHA-256 | `897f228aea2fb182454a662fa02cf290f40f026e16b0ffb04a759a8a467c85c4` |

The bits check and the comment were added after the browser run, the
WASM and the equivalence check; neither changes production code.
