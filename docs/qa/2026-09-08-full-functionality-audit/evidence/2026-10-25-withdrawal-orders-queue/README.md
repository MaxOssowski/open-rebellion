# Withdrawal orders through the turn queue (P24)

Date: 2026-10-25. This follows `../2026-10-25-player-orders-queue/`: the
withdrawal gives its orders through the same `SetOrder`, so their
executors also wait for their turn.

## The original

`FUN_005ad530` (the withdrawal of one side):

- sets the side's time (`+0x7c`) and the side's `+0x84` at once;
- for each fighter group of the side:
  - on the trench run (`+0x64` = 6), posts `0x3c` with state 8
    (`0x005ad5e7`), which aborts the run on its turn (`FUN_005cfec0`);
  - calls slot `+0x64`, `SetOrder` (`0x005ca530` in vtables `0x66bee0`
    and `0x66bf70`): 2 for a group without hyperdrives (`0x005ad604`), 3
    for the others (`0x005ad60f`);
- for each capital ship, moves it into a task force of its own
  (`FUN_005aded0`, event `0x30`), drops its attack (`FUN_005a8d30`) and
  calls `SetOrder(3)` on that force (`0x005ad6b8`).

`SetOrder` stores the order at once and releases the force's target
objects (`+0x80`, `+0x98`; jump table `0x5ca694`), then posts `0xa`. The
order's executor (`FUN_005ca6d0`) is installed only when that event runs:
`FUN_005cc300` for a capital ship's force, `FUN_005c6610` for a fighter
group, `FUN_005cf980` for a recovery.

## The port

- The order itself stays immediate: `retreating`, `order`, the dropped
  attack and escort, and the side's time.
- New: `BattleSession::withdrawal_executors`, the units whose order-3
  executor is installed. Event `TacticalTurnEvent::WithdrawalExecutor`
  (`0xa` with order 3) adds them on its turn.
- Only an installed executor moves a unit toward its exit and decides its
  jump. Until then the unit keeps its course.
- A group without hyperdrives gets its recovery through the queued
  `OrderExecutor` (order 2).
- The trench run's abort is the queued state 8 (`TrenchRunState(8)`).
- The stamp follows the caller:
  - the command stamp (turn + 3) from the interface's Withdraw;
  - the step's stamp from the AI commander's review, which runs in a
    turn's updates (`FUN_005c8780`).
- `port:` a group on the trench run whose craft the run's draws have
  taken is still ordered. The hull `FUN_005d08d0` kept comes back with
  state 8, and the group then withdraws.

## Not yet ported

- The capital ship's own task force (`FUN_005aded0`, event `0x30`).
- `SetOrder`'s refusal of a new order for a force on the trench run or a
  recovery of a side whose `+0x8` is 1 and that is not the local one
  (`0x005ca538`..`0x005ca577`). What that `+0x8` means is not read.

## Tests

Changed on this evidence (all asserted the executors at once):

- `a_withdrawing_ship_drops_its_attack_and_heads_for_its_exit`: the exit
  waypoint appears at step 16, not 12.
- `the_ai_side_of_a_tactical_battle_withdraws_like_the_player`: the
  executor is installed at step 16. The jump runs at step 32 and the ship
  leaves at step 60 (was 24 and 52).
- `the_ai_sides_fighters_withdraw_by_their_hyperdrive`: the recovery
  reserves its carrier at step 16. Within that step's recovery the leg has
  reached the carrier (Docking). The jump runs at 32 and the group leaves
  at 60.
- `withdrawing_aborts_the_trench_run` and
  `a_trench_run_order_is_heard_by_its_side_and_ends_with_a_withdrawal`:
  state 8 is queued and ends the run at step 16.
  - The first test gains an Alliance ship. Otherwise the battle would end
    before that turn: the run's draws have taken the runner's craft.
- Render: `a_withdrawal_sends_the_fighters_to_their_carriers_and_away` and
  `a_withdrawal_holds_fighters_with_a_hyperdrive_then_they_jump`. The
  carrier is reserved at step 16; the unit leaves at step 60.

New: `only_a_withdrawing_unit_with_its_executor_decides_its_jump`. Of
units at their exit after the side's time, only the one alive, of the
side, given the order and with its executor decides its jump. The event
installs each executor once and none for a destroyed unit. Without this
stage, the unit whose event has not yet run would decide too.

Ten tests set `retreating` by hand to model a withdrawal already under
way. They now also install its executor (`install_withdrawal_executors`).
Their timings are unchanged.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2794 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` (core and render) | 41 tested: 35 caught, 1 unviable, 5 missed, then closed |
| Release WASM SHA-256 | `a88445c195ff6293d9abb39093691621f616e882abd481fc5b08370578f71f02` |

How the five missed mutants were closed:

- The new test catches four of them. A core-only rerun with it missed
  three:
  - two new ones in the group condition of `withdraw_fighters_stamped`.
    The render tests catch these ("an empty group takes no order"), and the
    full run did not report them;
  - the fifth original one, in the ship loop's executor clause. The test
    then gained a unit given the order but without its executor, and
    catches it.

The browser matrix and the WASM ran before the new test, which changes no
code.

