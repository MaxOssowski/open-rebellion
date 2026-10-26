# The recovery runs on its executor's turns (P24)

Date: 2026-10-26. This follows `../2026-10-26-fighter-recovery-flight/`. The
port used to reserve a carrier when the Recover order's executor was
installed, set state 2 at once, check the docking every 250 ms step, and
take a docking squadron in on the next step. In the original, the executor
works once a turn and the states move through events.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

### The executor

- `FUN_005ca6d0` case 2 installs executor `0x66cdf8`. Its slot 8
  (`FUN_005cf940`) clears its latch `+4` and sets each member's state
  `+0x35c` to 0.
- Its slot 4 (`FUN_005cf980`) runs from the force's slot `+0x10`
  (`FUN_005c65e0`): once a turn, in the updates, never from the frame.

### Each turn

For each member in the fight still in state 0:

1. **Home carrier.** Its home carrier (`+0x164`) is tried first. See the
   `port:` notes below.
2. **Side's ships.** Then the side's capital ships (`+0x58`) in order, each
   in the fight. The first whose hangar has room takes it (`FUN_005b84e0`:
   `+0xc` below `+0x10`, then `+0xc`++), and the member gets state 1 and
   posts event `0x2c`. A ship that is no carrier has no room (capacity 0).
3. **Death Star.** For the Empire, the Death Star's hangar is tried next.
4. **No carrier found:**
   - the group's voice plays once (latch `+0x10`, events `0x8b`..`0x8e` or
     `0x109`..`0x10c`);
   - every hangar it tried was full (`FUN_005b8480`). Once (latch `+4`):
     - with its side's withdrawal under way (`+0x84`), event `0x2f` goes
       to each member still in state 0, and `FUN_005b2720` destroys it;
     - otherwise the force's slot `+0x64`, `SetOrder` (`0x005ca530`), sets
       order 0 for the whole group.

Then, for every member:

- in state 2, within 2.0 (`DAT_0066d088`) of its carrier's point, state 3
  and event `0x1a`;
- with all members in state 4, the "recovered" voice, once (`+0x10`).

### The events

- `0x2c` (`FUN_005b1e10`): state 2, and `+0x164` is the reserving carrier.
- `0x1a` (`FUN_005b8bd0`): with the carrier in the fight (or the Death
  Star operational), `FUN_005b8630` takes the squadron in:
  - `+8`++;
  - state 4, and `+0x170` = 5 (out of the fight);
  - the "recovered" voice when it was the player's group's last member.

`+0xc` keeps the docked squadron: it counts what a carrier holds as well
as what it reserved.

### The flight

`0x5c7060` aims a member on order 2 with a carrier at that carrier's
underside, in any state, and does not stop within 2.0. A member waiting in
state 0 with no carrier flies to its place.

## The port

| Original | Port |
|---|---|
| executor `0x66cdf8` and its latches | `BattleSession::recovery_executors`: one `TacticalRecoveryExecutor` per group, with `voiced` (`+0x10`) and `settled` (`+4`) |
| slot 8 (`FUN_005cf940`) | `begin_original_fighter_recovery` now only resets the states and installs the executor |
| slot 4 (`FUN_005cf980`) | `advance_recovery_executors`, called in `run_turn_updates` after the movers' update |
| event `0x2c` | `TacticalTurnEvent::FighterReserved` |
| event `0x1a` | `FighterDocked` |
| event `0x2f` | `FighterLeftBehind` |
| `+0x10` less `+0xc` | `carrier_room`: a carrier's capacity less the squadrons in states 1 to 4 bound for it |

- `advance_recovering_fighters` flies states 1, 2 and 3 to the carrier,
  with no stop within 2.0.
- The old per-step `advance_original_fighter_recovery` is removed.
- `run_turn_event` is now public, for fixtures and tests.

`port:` choices:

- The home carrier (`+0x164` from a squadron's launch, `FUN_005b8c70`) is
  not kept. Squadrons do not start in hangars in the port (see the open
  question below). So a member tries only its side's ships.
- The Empire's Death Star takes no squadron.

Open: in the original, squadrons without a working hyperdrive start in
their carrier's hangar (`FUN_005964c0`, `FUN_005b8570`) and launch one by
one (`FUN_005b8800`, event `0x19`, `FUN_005b8c70`). The port's world
records no carrier for a squadron, so this waits for a decision on the
save format.

## Gravity wells, settled

- No code adds a gravity target to a unit. The slots `+0x44` / `+0x4c`
  (`FUN_005b24d0`, `0x5b2480`) have no caller on a unit.
- The tactical dispatcher `FUN_005a96b0` has no case for `0x2a` or `0x2b`.
- Nothing reads the flags `+0x3a4` / `+0x3a8`: not `FUN_005b1b70`, not
  `FUN_005b17f0`, not `FUN_005ad530`.

So gravity wells are inert in the original's 3D battle. Their one effect is
strategic: no retreat is offered against an Interdictor. The port already
has that (`abstract_battle.rs`,
`retreat_is_offered_unless_an_interdictor_or_nowhere_to_go_bars_it`).
`set_gravity_field` stays a fixture-only visual path, as in the original,
where nothing sets it.

## Changed tests

| Test | Change |
|---|---|
| `a_squadron_docks_within_two_of_the_carrier_underside` | runs the executor's turn |
| `a_recovery_flies_only_squadrons_still_on_their_way` | Within 2.0 a squadron now flies on; the docking waits for the turn. A carrier of the other side leaves the state as it is; the port used to reset it. |
| `a_recovery_reserves_one_free_place_on_a_living_carrier` | It reserves on the executor's turn (state 1). The member that finds no room sets the whole group's order to 0. |
| `the_ai_sides_fighters_withdraw_by_their_hyperdrive` | state 1 at step 16, state 3 at step 28 (`0x2c` and the same turn's docking pass), state 4 at step 40 (`0x1a`) |
| `waypoint_arrival_and_fighter_docking_complete_source_executors` (render) | the executor's turn, then the queued `0x1a` |
| `fighter_recovery_warns_each_player_group_once_when_carriers_are_full` (render) | the voice comes on the executor's turn |
| `a_withdrawal_sends_the_fighters_to_their_carriers_and_away` (render) | The squadron is in state 1 at step 16. The two squadrons added later are on order 2. The one bound for the staying carrier docks through its queued `0x1a`: orbiting its carrier beyond 2.0, a squadron is not caught by a once-a-turn check. |
| `committed_maneuver_hold_and_recover_enter_original_executors` (render) | state 1 after the turn |
| `mission_confirm_assigns_recover_to_the_whole_selected_fighter_group` (render) | the voice after the executor's turn |
| `recovered_fighter_clears_attack_fighters_without_capital_fire_or_audio` (render) | its docking squadron's `0x1a` is queued |

Browser fixtures:

- `configure_command_progression_fixture`:
  - installs the executors after the groups are set;
  - runs the docking squadron's `0x1a` first, so it is taken in where it
    stands (state 4). A state 3 squadron flies on until its event;
  - flies its returning squadron and runs the executor's turn (state 3, as
    the probe expects).
- `configure_command_execution_fixture`: after the order's executors, it
  runs the recovery executor's turn and its `0x2c`, so the probe's state 2
  holds.

The first matrix runs failed these two cases (8, then 4 executions) on
those fixture orders. After the fixes both passed 4 of 4.

## New tests

- `a_reserved_squadron_heads_for_its_carrier_through_event_0x2c`;
- `a_carrier_full_of_recovered_squadrons_takes_no_more`: also no docking
  with the carrier lost;
- `a_withdrawing_recovery_with_no_room_leaves_its_squadrons_behind`.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2902 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 148 in run `2026-10-26T20-56-09-448Z-3301288`. Its 4 failures (`production-command-progression-presentation`) and the execution case passed after the fixture fix (runs `2026-10-26T21-15-19-005Z-3502174` and `2026-10-26T21-15-31-369Z-3502974`, 4 of 4 each). The fixtures are not in the release build. Arch Chromium 155, not the pinned build |
| Replay equivalence | passed |
| `cargo mutants` | 55 tested: 52 caught, 1 unviable, 2 missed (first run: 10 missed) |
| Release WASM SHA-256 | `b49b7e2883021612e137553f4c999082b09b1eb75795d49f3eb48941343b7ca1` |

After the first mutation run:

- the executor's "all in" branch was dropped: a docked member leaves the
  group's order, so it never held;
- the executor list's pruning and an index check were dropped as well,
  since nothing depended on them;
- `the_last_docked_member_of_a_player_group_plays_its_voice` was added.

The two missed mutants empty the two browser fixtures. The browser cases
above check them.
