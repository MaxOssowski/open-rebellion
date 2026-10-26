# A unit's weapon turn opens one turn in three (P24)

Date: 2026-10-26. Before this stage, every capital ship and fighter group
with a target in range fired on every 1000 ms turn. In the original, each
unit draws first and fires only on one draw in three. A unit on Escort
never fires.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository, checked against the `objdump` disassembly).

## The original

### The turn's unit loop

`0x005a9de0` (the battle's slot `+0x88`, run once a turn) walks the unit
list `+0x988`. Capital ships and fighter groups share the list, in the
order they entered the fight (`FUN_005a9170`). For each unit in the fight
(`FUN_005ad870`):

1. the recharge (`FUN_005b6320`);
2. for a capital ship only (slot `+0x20` is 0), the repair (slot `+0x9c`,
   `FUN_005b0330`). Every 50 turns it calls `FUN_005b1490`, which draws;
3. while `+0x124` is clear, the weapons (slot `+0x88`, `FUN_005b3a40`).

So a unit's repair draws come just before its own weapon draw.

`+0x124` is set only by the second battle manager (`0x66c880` slot
`+0x64`, `0x005c0170`), for units of the side that is not local. The
single-player manager (`0x66c7c8`, `0x005be400`) always clears it.

### The front of `FUN_005b3a40`

Both unit classes use this slot: capital ships, and fighter groups
(`0x66c028 + 0x88`). In order:

| Address | Check |
|---|---|
| `0x5b3a51`, `0x5b3a5e` | in the fight and not in state 6 (its jump) |
| `0x5b3a6b`..`0x5b3a8c` | some arc is ready: `+0x1e0`, the first arc record's ready flag (`+0x17c + 0x64`), four records `0x74` apart |
| `0x5b3a92`..`0x5b3aa4` | `FUN_005a8a70(0, 2)` (`FUN_0061a310 % 3`) must be 0 |
| `0x5b3adf`..`0x5b3ae5` | its force's order `+0x64` must not be 1 (Escort) |

Only then does it look at targets.

Notes:

- A unit with no ready arc returns before the draw, so it does not draw.
- An escort draws, then returns. It neither fires nor uses its tractor
  beam (`FUN_005b2140` comes later).
- A later check (`0x5b3b7e`) also tests order 1, but it cannot be reached
  with order 1.

### The manual

The manual says an escort will "surround the friendly ship and take
opportunity fire at nearby enemy vessels" (tactics chapter, Missions). The
executable contradicts this: order 1 returns at `0x5b3ae5`. The behaviour
follows the executable.

## The port

`BattleSession::run_turn_updates` now goes unit by unit:

- capital ships, each with its repair draws and then its weapon turn;
- then fighter groups, each with its weapon turn.

`weapon_turn_opens` holds the gate:

- a ready arc;
- the draw `tactical_rng.inclusive(0, 2)` (`ORIGINAL_WEAPON_TURN_DRAW_MAX`);
- the Escort check.

The unit's fire then runs as before: `fire_side` and
`fire_capitals_at_fighters_from` for a capital ship, `fighter_step_from`
for a group.

`port:` choices:

- The list order `+0x988` (units by their entry into the fight) is not
  traced. The port takes its capital ships first, then its fighter groups.
- The repair runs only for a ship that is not jumping. In the original it
  also runs in state 6, when weapons no longer do.
- The targets after the gate are the port's, not yet those of
  `FUN_005b3a40`: the force target, the force's nearest enemy, and
  fire at fighters only. That is the next stage.

## The draw count and the Death Star probe

`BattleSession::weapon_draws` counts the weapon turns' draws. It is a
diagnostic only. The battle's trench-run launch log prints it, and so does
the fixture's ready report (`tactical_test_fixture.rs`).

The browser probe `production-death-star-presentation` asserted that the
tactical stream at the run's launch was still the stream at the fixture's
ready point: "the trench-run order drew from the shared tactical RNG before
state 5". With the weapon turns, the units draw during the three turns
before the launch, so that equality failed (145 of 148 in the first run,
`2026-10-26T17-23-36-551Z-771928`).

The probe now advances the ready stream by exactly the weapon draws between
the two points (`advanceTacticalRng`, MSVC `rand`). Any other draw still
fails it, so the probe keeps its power. The constructions of
`BattleSession` in the test sessions, the fixture and `main.rs` set the
count to 0.

## Changed tests

Four tests assumed fire on the first turn. Each now starts from a stream
whose first weapon draws come up 0 (`stream_opening_weapon_turns`), with a
comment citing `FUN_005b3a40`. No assertion changed:

- `a_shot_fires_and_hits_on_its_turns_while_its_units_fight`;
- `a_withdrawing_ship_can_be_hit_until_its_jump_starts`;
- `dead_and_jumping_ships_drop_out_of_the_fight`;
- `recovered_fighter_clears_attack_fighters_without_capital_fire_or_audio`
  (render).

## New tests

In `abstract_battle.rs`:

- `a_unit_fires_only_when_its_draw_comes_up_zero`: draws 0, 1 and 2, with
  exactly one draw taken;
- `a_unit_with_no_ready_arc_does_not_draw`;
- `an_escort_draws_but_holds_its_fire`;
- `a_fighter_group_takes_the_same_weapon_turn`;
- `a_unit_out_of_the_fight_takes_no_weapon_turn`: a dead or jumping ship,
  and a lost, empty, jumping or docked group, each failing one condition,
  with an unarmed enemy that keeps the battle going.

Without the gate (fire every turn, no draw), the first, third and fourth
fail. The second passes there too, since the old code did not draw. A
first version of the last test had no enemy: the battle ended at once,
the test passed vacuously, and 5 mutants survived. With the enemy it fails
on each of them.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2873 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T17-42-13-947Z-1090041`; Arch Chromium 155, not the pinned build) |
| Replay equivalence | passed |
| `cargo mutants` | 39 tested: 38 caught, 1 unviable |
| Release WASM SHA-256 | `6036b5063d775b56ba60f28c6b955971be7ec02f6162d45d234be9314f7da09a` |

The first matrix run (145 of 148) also failed
`empire/production-battle-options-withdrawal/1280x800-letterboxed` with
"paused tactical render stalled during the 60-frame timing sample", while
the tests and mutants ran beside it. Run alone, it passed.
