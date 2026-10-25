# The tactical battle's event turn queue (P24)

Date: 2026-10-25. The user chose to rebuild the battle's turns as the
original runs them. This is the first stage.

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`). The full
reading, with every queued event, is in
`ghidra/notes/tactical-event-turns.md`.

- **Turns.** The battle manager (`0x005be160`, `0x005bf830`) runs events
  in 1000 ms turns (`+0x8a8`).
  - At each turn `+0x878` advances. Once it has reached the lead
    `+0x8ac` = 3, `FUN_005a96b0` runs the events stamped `+0x878 - 1`.
  - Then the turn's updates run with 1000 ms: the side commanders
    (`+0x80`, `FUN_005c8780`), the task forces (`+0x84`, `0x005a9d40`) and
    the units (`+0x88`, `0x005a9de0`).
  - Then `+0x87c`, the stamp for new events, advances. It stays three
    turns ahead of `+0x878`.
- **Timing.** An event posted in turn k's updates is stamped k + 2 and
  runs at turn k + 3. One posted from the interface between turns is
  stamped k + 3 and runs at turn k + 4.
- **The jump.**
  - When `FUN_005cc300` (capital ships) or `FUN_005c6610` (fighters)
    decides a jump, the unit enters state 6 and posts event `0x2d`
    through slot `+0x28` (`0x005cc53c`, `0x005c6809`). State 6 is still
    in the fight (`FUN_005ad870` takes 2, 4 and 6).
  - At its turn `FUN_005b1e40` starts the flight (state 7) if the
    hyperdrive still works. Otherwise `FUN_005b2720` destroys the unit.
  - The flight (`FUN_005a6b40`) runs 4000 ms and then posts event `0x2e`
    (state 8). Its turn (`FUN_005b26d0` → `FUN_005a90c0`) removes the unit
    (state 9) from its task force.

## Change

- `BattleSession::turn_events` is the queue, a list of
  `TacticalQueuedEvent { stamp, event }`.
  - `command_event_stamp` is `+0x87c` for the interface:
    `combat_tick / 4 + 3`.
  - `step_event_stamp` is the stamp within a step, one less on a turn's
    first step.
  - `run_turn_events` runs, at the start of each turn's first step, the
    events stamped turn − 1.
- The superlaser now posts `DeathStarBeam` (`0x44`) and `DeathStarKill`
  (type 3) on the queue. Its timing is unchanged from
  `../2026-10-25-death-star-shot-timing/`.
- A withdrawal jump now takes the original's states:
  - the decision posts `JumpStart` (`0x2d`). Until it runs, the unit stays
    in the fight, is not tried again and makes no sound;
  - the event starts the flight and its sound, or destroys a unit that has
    lost its hyperdrive or engine power;
  - at the end of the flight the unit waits where it is (state 8) and
    posts `JumpEnd` (`0x2e`), which removes it;
  - so the next member of a capital task force is tried only once the
    first is gone.
- Example: a jump decided on step 1 starts at step 16 (turn 4). Its flight
  ends on step 31, and the unit leaves at step 44 (turn 11). Before, it
  started at once and left at step 16.
- `port:`
  - one queue for both sides, in posting order;
  - the turn boundary is every fourth 250 ms step from the first combat
    step;
  - the flight still advances by the 250 ms step;
  - a ship that loses its hyperdrive while its `0x2d` waits is destroyed
    by the side's end (event `0x2f`, not yet queued) if the side does not
    wait on another unit.

## Not yet changed

Further stages will move these onto turns, as the original runs them:

- the turn's logic (weapons, recharge, commanders, task forces) still runs
  every 250 ms step, not once per 1000 ms turn;
- weapon hits, subsystem damage and repair, `0x2f` and the other events
  in the inventory still apply at once;
- the per-frame slot `+0x98` runs only from turn 3, so in the original
  nothing moves for the first three seconds.

## Tests

Eleven core tests' expectations changed on this evidence, all on the
jump's timing:

- `the_ai_side_of_a_tactical_battle_withdraws_like_the_player`
- `the_players_withdrawal_ends_after_the_sides_time` (stamps 32 and 39;
  the jump starts at step 132 and the ship leaves at step 160)
- `units_left_behind_are_destroyed_at_the_sides_end`
- `the_players_fighters_with_a_hyperdrive_jump_when_the_time_ends`
- `a_withdrawing_fighter_group_flies_to_its_edge_before_it_jumps`
- `after_the_time_a_ship_jumps_only_near_its_exit` (now checks the queued
  `0x2d`)
- `a_task_force_jumps_its_ships_one_after_another` (the second member is
  tried only after the first leaves, at step 44)
- `only_the_withdrawing_units_leave`
- `the_ai_sides_fighters_withdraw_by_their_hyperdrive`
- `a_jump_flies_its_unit_away_and_sounds_once` (no sound in state 6;
  state 8 holds its place)

Two render tests (`a_withdrawal_sends_the_fighters_to_their_carriers_and_away`,
`a_withdrawal_holds_fighters_with_a_hyperdrive_then_they_jump`) changed the
same way.

`a_ship_needs_engine_power_to_jump` changes the same way and adds a case: a ship that loses its
engine power while its `0x2d` waits is destroyed by that event at step 16.

Ten core tests fail with the queue's run removed from `step()`.

New tests:

- `queued_events_run_at_their_turn_from_the_third`: nothing runs before
  turn 3; at step 12 stamp 2 runs and stamps 0 and 1 are dropped; stamp 3
  runs at step 16.
- `a_jump_event_skips_a_unit_out_of_the_fight`: no flight for a destroyed
  unit, and no restart for one already flying.
- `a_fighter_group_decides_its_jump_once_after_the_time`: no decision
  while time is left, then exactly one queued `0x2d`, and none without a
  hyperdrive.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2773 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| `cargo mutants` | 76 tested, 66 caught, 1 unviable; after the flying case, 2 more caught |
| Release WASM SHA-256 | `262c3d011fe9c2dc60d82f853649f7da6a5bbc2fcbde7d69435ecfe5d3b4e4ca` |

The browser run, the WASM and the equivalence check were made before the
three new tests, which change no production code.

The seven surviving mutants are on the 8.0 exit test
(`-((exit - z) * side_z) <= 8.0`) of both jump decisions. They are
equivalent: the decision needs the force within the 1.0 arrival radius
first, and every mutated form is then also within 8.0.
