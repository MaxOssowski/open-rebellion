# A withdrawing capital-ship task force jumps one ship at a time (P24)

Date: 2026-10-25.

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`), from the
objdump disassembly.

### `FUN_005cc300`, order 3

1. **The move.** It aims the force's mover `+0x54` at its own x and y and
   at the exit line's z, as `FUN_005c6610` does for fighters. It calls the
   mover's slot `+0xc` with radius 1.0 (`0x005cc3ba`). While the move
   returns 1 (still moving), the step ends (`0x005cc3ef`).
2. **The mover depends on the formation** (`0x005cb1bf`..`0x005cb3b6`):

   | Mover | Vtables | Move (slot `+0xc`) |
   |---|---|---|
   | `0x005c72b0` | `0x66cb50` / `0x66cb30` | `0x005ec0d0` |
   | `0x005ec310` | `0x66da08` / `0x66d9e8` | `0x005eb1a0` |
   | `0x005eb860` | `0x66d9a0` / `0x66d980` | `0x005eb540` |

   `0x005eb1a0` has the same contract as `0x005ec0d0`: within the radius it
   clears the velocity and returns 0, and otherwise it steps and returns 1.
3. **One member.** Only after arrival and the side's time
   (`_DAT_0066d000`) does it take one member, `0x005c6070`: the first node
   of the mover's list (slot `+0x10` twice). It starts that member's jump
   (state 6, event `0x2d`) if the member:
   - is still in the fight (`FUN_005ad870`);
   - has a working hyperdrive (`FUN_005b1b70`);
   - is within 8.0 of the line (`_DAT_0066cff0`);
   - is not in state 6, 7 or 8;
   - has `+0x124` clear.

   Otherwise the step ends for the whole force.
4. **The member stays first.** A member leaves the list only when it is
   gone: event `0x2e` → `FUN_005a90c0` → `FUN_005aa760` → `0x005a93b0` →
   `0x005ceb70` on the battle's task-force lists `+0x8f8` / `+0x900`.
   - While its 4000 ms jump runs, the next member is not tried.
   - A first member that cannot jump holds the force's other members,
     and the side waits on them (`FUN_005c8780`, state 3).

## Change

- Before, the port started the jump of every withdrawing ship within 8.0
  of its line in the same step.
- Now `advance_withdrawal` tries only each task force's first live member
  (`original_task_force_first_members`), and only once it is within the
  arrival radius (`ORIGINAL_WITHDRAWAL_ARRIVAL_RADIUS`, now shared with
  the fighters).
- `port:`
  - the force's position is its first member's;
  - members are in the port's list order;
  - the port's ship movement still snaps to the waypoint, not the
    formation mover's unclamped step.

## Tests

- `a_task_force_jumps_its_ships_one_after_another` (new):
  - the second ship waits through the first one's 4000 ms jump, then
    jumps;
  - behind a first ship without a hyperdrive, it never jumps.
- `after_the_time_a_ship_jumps_only_near_its_exit`. Its expectations
  changed on this evidence:
  - 7.5 and -30 from the line are still moving, so no jump;
  - the jump is checked at 0.5.
- `only_the_withdrawing_units_leave` and `a_ship_needs_engine_power_to_jump`
  put their second ship in its own task force. These tests are about
  something else, and in the original a whole force withdraws together
  (`FUN_005ad530`).
- The new test and the changed one fail with the gate removed.
- `after_the_time_a_ship_jumps_only_near_its_exit` also checks exactly 1.0
  (arrived: `|d|` not above the radius), added for the one missed mutant.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2768 passed, 0 failed (before the 1.0 case; core rerun: 1529 passed) |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| `cargo mutants` | 21 tested, 20 caught; the miss (`>` to `>=` on the radius) is now caught by the 1.0 case |
| Release WASM SHA-256 | `0588cabddae4ad20de4dcbef34d93a72f4f615bc7f977179e2e70cd7db9e4b04` |
