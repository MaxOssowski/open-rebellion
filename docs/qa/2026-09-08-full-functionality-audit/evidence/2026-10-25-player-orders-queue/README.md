# Player orders through the turn queue (P24)

Date: 2026-10-25. This continues `../2026-10-25-tactical-event-queue/`:
a player's order now takes effect on its turn, as in the original.

## The original

- `SetOrder` (`0x005ca530`, task-force slot `+0x64` in vtables `0x66bee0`
  and `0x66bf70`):
  - stores the new order in `+0x68` at once;
  - posts event `0xa` through the manager's slot `+0x28` (`0x005ca684`),
    so with the command stamp `+0x87c`, three turns ahead.
- The event's handler (`0x005caba0`) calls slot `+0x48`, `FUN_005ca6d0`.
  It takes the order from the event, stores it in `+0x64` and creates the
  order's executor. Until then the task force keeps its old executor.
- The formation or tactic (`0x005cb160`) follows the same pattern: `+0x5c`
  at once, event `0xb` to rebuild the mover `+0x54`.

## The port

- `assign_selected_command` still sets each selected unit's order, tactic
  and cleared escort at once (`+0x68`).
- It then posts one `TacticalTurnEvent::OrderExecutor` for the units given
  the order, at `command_event_stamp` (turn + 3).
- On that turn, `run_order_executor` runs what the render crate used to run
  at once:
  - the capital-ship manoeuvre and hold executors;
  - the attack target acquisition and the Death Star attack (trench run);
  - the fighter recovery (`FUN_005cf980`).
- `port:` a unit that has left the battle by then is skipped: it has no
  task force for the event to reach.
- The executors moved from `tactical_view.rs` to `rebellion-core` with
  their citations, so the queue can call them.
- The `trench_run_launch` log now comes from the executor, when the run
  actually begins, instead of from the click.
- Test fixtures that present an order's result (command execution, attack
  targeting) run the posted executors at once
  (`run_posted_order_executors`), as the trench-run fixture already skips
  the approach.

## Still open

- The withdrawal (`FUN_005ad530`) also gives its orders through `SetOrder`:
  2 or 3 for fighter groups (`0x005ad604`, `0x005ad60f`), 3 for capital
  ships (`0x005ad6b8`), and state 8 for a trench run (`0x005ad5e7`). The
  port still applies them at once. This is the next stage.
- The formation mover (`+0x54`, event `0xb`) is not ported. The tactic is
  stored but nothing moves by it.

## Tests

- `an_order_s_executor_waits_for_its_command_turn` (new, core): an order
  posted on turn 0 sets no waypoint until turn 4's first step (step 16).
- Four render tests had asserted the executors' effects right after the
  order and failed with this change:
  - `committed_maneuver_hold_and_recover_enter_original_executors`;
  - `attack_orders_acquire_first_eligible_typed_target_for_both_unit_classes`;
  - `attack_executor_preserves_live_engagement_and_replaces_invalid_target`;
  - `mission_confirm_assigns_recover_to_the_whole_selected_fighter_group`.

  They now run the posted executors (`run_posted_orders`) before asserting
  them. The first also checks that no waypoint exists before that.
- New targeted tests for the moved code:
  - `each_manoeuvre_aims_at_the_opponent_furthest_along_its_side`
    (including the first of two equal scores);
  - `a_left_hook_turns_its_leg_an_eighth_of_pi`;
  - `an_order_s_executor_skips_units_gone_from_the_battle`;
  - `the_death_star_attack_takes_the_first_flying_group_against_a_standing_station`.
- Browser: the assignment journey and the Alliance Death Star journey
  resume the battle after the order and wait for the executor's voice and
  launch log.
- In the last render test, the order of the two voices changed on this evidence:
  - the mission voice plays when the order is given;
  - the "no room" voice comes from the recovery executor (`FUN_005cf980`),
    so it now plays after it.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2793 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | 107 tested: 103 caught, 2 unviable, 2 missed |
| Release WASM SHA-256 | `3c403e1c5bc3722e146ce18f4d933ed037f3dc1c0e6242cc0c1f3675645a1324` |

The two missed mutants empty the command-execution and attack-targeting
fixtures. The browser matrix covers them: its first run of this stage,
before the fixtures ran their executors, failed exactly those journeys.

