# The tactical battle's logic runs once a turn (P24)

Date: 2026-10-25. Second stage of rebuilding the battle's turns, after
`../2026-10-25-tactical-event-queue/`.

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`), recorded in
`ghidra/notes/tactical-event-turns.md`.

After a turn's events, on the next frame (`+0x8b0` = 1), the manager runs
the turn's updates, each given the turn length `+0x8a8`, 1000 ms
(`0x005bed3e`..`0x005bedb9`):

| Slot | Function | What it does |
|---|---|---|
| `+0x80` | `0x005a9d10` → `FUN_005c8780` | Each side's commander. It counts down the withdrawal time `+0x7c` by the 1000 ms, charges the Death Star's laser (`FUN_005ba7f0` at `0x005c88ec`), and reviews the battle when its count `+0x40` (−1.0 a call) reaches 0, so every sixth turn. |
| `+0x84` | `0x005a9d40` | Each task force's order step, slot `+0xc` (`FUN_005c6610` / `FUN_005cc300`), with 1000 ms. This includes the withdrawal's move (`0x005ec0d0`) and its jump decisions. |
| `+0x88` | `0x005a9de0` | Each unit in the fight: the recharge `FUN_005b6320` (1000 × `DAT_0066c26c` = 1.0 s), the capital ship's repair timer `FUN_005b0330` (slot `+0x9c`, every 50 000 ms), and the weapons `FUN_005b3a40` (slot `+0x88`, for capital ships `0x66c140` and fighter groups `0x66c560`). |

## Change

On a turn's first step (every fourth 250 ms step), after its events,
`run_turn_updates` runs:

- the Death Star's laser charge, for 1 s, and its ready voice;
- the AI commander's review (`ai_side_leaves`) at turns 1, 7, 13 and so
  on, instead of steps 1, 7, 13;
- the recharge of every ship and fighter group, for 1 s;
- the subsystem repair every 50 turns
  (`ORIGINAL_SUBSYSTEM_REPAIR_INTERVAL_TURNS`; before, every 200 steps:
  the same 50 s);
- the weapons of both sides' ships, the ships against fighters, and the
  fighter groups.

The withdrawal (`advance_withdrawal(turn)`) now runs as follows:

- **On the turn only:**
  - the commander's countdown, by 1000 ms;
  - the side's end (states 3 and 4);
  - the task forces' jump decisions;
  - the withdrawing fighter groups' move, a full unclamped step of
    engine power × 1 s.
- **Every step:** the jump flights, as `0x005a9ec0` runs them per frame.

A consequence of the full step: with engine power above twice the 1.0
arrival radius, a group can swing past its line and back
(`a_withdrawing_fighter_group_overshoots_its_line_by_a_full_step`). The
original's move does the same.

`port:`
- each part runs for all units in turn, not unit by unit;
- the per-step movement, escort, fighter recovery, trench run and the
  battle-end check still run every 250 ms step. Their place in the
  original's turn and frame has not yet been read.

## Tests

Expectations changed on this evidence:

- `the_ai_commander_reviews_every_sixth_turn` (was `..._sixth_step`):
  reviews at steps 4 and 28;
- `withdrawing_fighters_head_for_their_own_edge`: no move until step 4,
  then a 1 s move;
- `a_withdrawing_fighter_group_flies_to_its_edge_before_it_jumps`: moves
  at steps 4 and 8, decides at step 12 (stamp 5), jumps at step 24;
- `a_withdrawing_fighter_group_overshoots_its_line_by_a_full_step`: per
  turn, at 2.5 and at 10 per second;
- `after_the_time_a_ship_jumps_only_near_its_exit`: decided at the turn;
- `the_ai_side_of_a_tactical_battle_withdraws_like_the_player`: reviewed
  at step 4; its time ends at step 8, it jumps at step 20 and leaves at
  step 48;
- `a_fighter_group_decides_its_jump_once_after_the_time`,
  `a_ship_needs_engine_power_to_jump`,
  `a_task_force_jumps_its_ships_one_after_another`: decisions at the turn;
- render: `death_star_ready_voice_fires_once_only_when_the_players_laser_crosses_full_charge`
  (the edge at step 4),
  `losing_the_committed_death_star_attackers_routes_failure_once` and
  `recovered_fighter_clears_attack_fighters_without_capital_fire_or_audio`
  (fire at step 4).

With every step treated as a turn, 21 core and 6 render tests fail.

New tests:

- `the_battle_ends_when_a_side_has_nothing_left_in_it`: the battle-end
  check, moved but not changed, covered for fighter craft and the Death
  Star on both sides;
- `a_full_laser_does_not_repeat_its_ready_voice`;
- `a_damaged_ship_tries_its_repair_at_the_fiftieth_turn`: the repair comes
  at step 200, for the damaged ship only.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2776 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| `cargo mutants` | 51 tested; 10 missed in the first run; after the new tests 1 missed (the repair's `hits == 0`), which the repair test now catches |
| Release WASM SHA-256 | `5343e7aa61c89584718313df8a8f0147546879f96fcf114d5a67e865bc58327e` |

The browser run, the WASM and the equivalence check were made before the
three new tests, which change no production code.
