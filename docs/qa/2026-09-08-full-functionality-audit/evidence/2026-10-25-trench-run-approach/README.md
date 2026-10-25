# The trench run's approach and voice 0x11d (P24)

Date: 2026-10-25. This closes the open 0x11d item of
`../2026-10-25-tactical-voice-announcer/`. It also corrects the cadence of
the task forces' order steps from `../2026-10-25-tactical-turn-updates/`.

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`).

1. **The order.** The fighter task force's order 6 case (`0x005caae9`)
   creates the result object `0x005d0820`:
   - it sets `DAT_006bc978` = 1, which `FUN_005d0ae0` reads as "a run is
     under way";
   - it posts event `0x3c` with state 2 through `FUN_005cfe80` (slot
     `+0x28`, stamped `+0x87c`; `0x005d0884`).
2. **The start line.** The result's update (`0x005d0430`) calls slot
   `+0x8` (`0x005d0340`) on its first run (`+0x10` = 0). That sets state 2
   and plays the group's start line (0x139, 0x12d, 0x131, 0x135) for the
   Alliance's player, or 0x115 for the Empire's. The task force's slot
   `+0x10` (`0x005c65e0`) runs that update once a turn, from `0x005a9d40`.
3. **The approach.** The task force vtables start at `0x66bee0` (fighters)
   and `0x66bf70` (capital ships). The order step is slot `+0x14`:
   `FUN_005c6610`, or `FUN_005cc300` for capital ships. The frame function
   `0x005a9ec0` calls it with the frame's milliseconds (`0x005a9fec`..
   `0x005aa034`), from turn 3. In order 6:
   - while the result's state `+0x2c` is 2 (`0x005c6887`), the force's
     mover flies toward the side vector turned about y,
     `(-side.z, 0, side.x)`, times half the extent
     (`0x005c6848`..`0x005c68f5`);
   - the move's radius is 0.2 (`0x3e4ccccd`, `0x005c68f0`). The move
     (`0x005ec0d0`) is unclamped, but a frame's step is far below the
     radius;
   - on arrival, with the result's slot `+0x10` clear, it posts state 5
     (`0x005c6924`).

   For the Alliance (side vector (0, 0, 1)) the point is (−0.5e, 0, 0),
   toward the Death Star's side of the field.
4. **The run.** Event `0x3c` (`FUN_005caf20` → `FUN_005caf50`, for a
   task force on order 6) calls `FUN_005cfec0(5)`:
   - it starts the 120 000 ms timer (`DAT_0066d080`);
   - it plays the group's follow-up line for the Alliance's player, or
     0x11d for the Empire's;
   - it runs `FUN_005d04e0`'s draws.

   At the timer's end `0x005d0430` posts state 6 or 7.

A0 TAC-01-C012 (Empire) has 0x115 at 39.74 s and 0x11d (TACTICAL.DLL
15132) at 53.72 s. That leaves 14 s for the event lead, the approach and
the lead again.

## Change

- **The order.** `BattleSession::begin_original_trench_run` (called by the
  Attack Death Star order):
  - marks the run under way;
  - plays only the start line or 0x115;
  - posts `TrenchRunState(2)`.

  Before, the order started the 120 s timer at once and played the start
  and follow-up lines together.
- **The approach.** `advance_trench_run_approach` runs every step from turn
  3, once state 2 has run. It moves the group toward
  `(-side_z * 0.5e, 0, 0)` at the slowest member's engine power. Within
  0.2 it posts `TrenchRunState(5)`.
- **The run.** `commit_original_trench_run` (state 5):
  - starts the 120 s timer;
  - plays the follow-up line, or 0x11d
    (`TacticalVoiceCue::death_star_attack_continuing`);
  - runs `prepare_original_trench_run`.

  It logs `[tactical_death_star] trench_run_state=5`.
- **"Under way".** `trench_run_under_way` (`FUN_005d0ae0`) now covers the
  approach too. The voice announcer's checks, the AI review, a second
  order and the withdrawal's abort use it. An abort during the approach
  drops the run's queued events.
- **The fixture.** The browser fixture's forced outcome goes straight to
  state 5.
- **Correction of the earlier stage.** The withdrawal's task-force steps
  (the fighters' move to the line, the jump decisions) run every step from
  turn 3 again, not once a turn. The commander's countdown and the side's
  end stay once a turn. A jump decided at the first frame step, step 12,
  starts at step 24 and the unit leaves at step 52.
- `port:`
  - the force's position is its first member's, and each member moves by
    the same step;
  - each 250 ms step is flown in 60 Hz frames
    (`PORT_MOVER_FRAME_MILLISECONDS`). In a single 250 ms step a fighter
    group (4.4 units a second in the browser fixture) passes the point by
    more than the 0.2 radius and swings back and forth without arriving;
    the browser run showed exactly that before the frames were added;
  - the order itself is applied at once. The original's order event is not
    yet read, so the start line plays at the order.

## Tests

- New:
  - `a_trench_run_flies_its_approach_before_the_run_begins`: state 2 runs
    at step 16; arrival within 0.2 during step 23 posts state 5 (stamped
    8); the run starts at step 36 with the follow-up line. It fails with
    the approach removed.
  - `a_trench_run_approach_moves_only_its_group_toward_its_side_s_point`:
    only living members of the run's group with craft and order 6 move, at
    the slowest power, toward −x for the Alliance and +x for the Empire.
  - `a_trench_run_order_is_heard_by_its_side_and_ends_with_a_withdrawal`.
  - `a_second_attack_death_star_order_waits_for_the_run_in_its_approach`.
  - `the_ai_commander_reviews_every_sixth_turn` gains an approach case.
- Changed on this evidence:
  - `death_star_attack_entry_routes_each_local_perspective_once`: the order
    plays only the start line or 0x115 and posts state 2 at stamp 3. State
    5 plays the follow-up line, or 0x11d, and the first chatter.
  - `attack_death_star_order_uses_the_timed_native_result_producer`,
    `successful_trench_run_applies_the_source_half_group_casualty_rule` and
    `losing_the_committed_death_star_attackers_routes_failure_once` are
    about the run itself, so they now enter state 5 explicitly after the
    order.
  - The withdrawal tests follow the frame step again:
    - `withdrawing_fighters_head_for_their_own_edge`;
    - `a_withdrawing_fighter_group_flies_to_its_edge_before_it_jumps`;
    - `a_withdrawing_fighter_group_overshoots_its_line_by_a_full_step`;
    - `after_the_time_a_ship_jumps_only_near_its_exit`;
    - `the_ai_side_of_a_tactical_battle_withdraws_like_the_player`;
    - `the_players_fighters_with_a_hyperdrive_jump_when_the_time_ends`;
    - `the_ai_sides_fighters_withdraw_by_their_hyperdrive`;
    - `a_jump_flies_its_unit_away_and_sounds_once`;
    - `a_ship_needs_engine_power_to_jump`;
    - `a_task_force_jumps_its_ships_one_after_another`;
    - `only_the_withdrawing_units_leave`;
    - `a_fighter_group_decides_its_jump_once_after_the_time`;
    - two render withdrawal tests.

    Each now decides at step 12, jumps at step 24 and leaves at step 52.
  - `units_left_behind_are_destroyed_at_the_sides_end` and
    `a_withdrawal_sends_the_fighters_to_their_carriers_and_away`: the groups
    the tests describe as still on their way to a carrier get no engine
    power. With the carrier leaving later they had docked first.
- Browser: the Death Star presentation journey now:
  - expects `status=approach remaining_ms=0` at the order and no RNG use
    before state 5, for both factions;
  - unpauses and waits for `trench_run_state=5`;
  - then checks the follow-up voice.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2782 passed, 0 failed |
| Provenance scan | 0 uncited items (after citing `FUN_005c6610` on the frame helper) |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| `cargo mutants` | 104 tested: 99 caught, 1 unviable, 1 timeout (the frame loop's `>` to `>=` never ends), 3 missed |
| Release WASM SHA-256 | `747f85cb7d77b4edc9c8e57e748de20361fa031a9b73ed189ca68ee787c00ccb` |

The three missed mutants:

- `abort_original_trench_run`'s `||` to `&&`: with no run, the mutant
  goes on to clear state that is already empty. The chatter cursor it sets
  is read only while a run's timer runs.
- `ai_side_leaves`' `|| retreat_done` to `&&`: equivalent, because
  `tactical_side_withdraws` checks the side's retreat order itself.
- `configure_trench_run_outcome_fixture` replaced with `()`: a browser
  fixture, covered by the trench-run browser cases (all passed).

The test changes after the browser run (the z axis, the frame check, the
review cases) change no production code.
