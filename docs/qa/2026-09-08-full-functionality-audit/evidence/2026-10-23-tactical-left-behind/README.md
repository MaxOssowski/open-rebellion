# Units left behind by a tactical withdrawal are destroyed (P24)

Date: 2026-10-23.

## Source

- `FUN_005ad530` gives every capital ship of the withdrawing side order 3,
  with or without a hyperdrive. Its second loop runs over `+0x58` and
  calls `FUN_005aded0`. So every ship heads for its edge
  (`FUN_005cc300`).
- `FUN_005c8780`, state 3, waits while a unit with a working hyperdrive
  (`FUN_005b1b70`) is still in the fight or jumping. Then it sets state 4
  and posts event `0x2f` for each unit of the side without one.
- `FUN_005b2720` (the event's handler) then destroys such a unit if it
  is still in the fight with hull. It sets state 2 and calls slot
  `+0x1c`, which is `FUN_005b2d20` for both the capital-ship (`0x66c140`)
  and the fighter (`0x66c560`) vtables. From state 2, `FUN_005b2d20`
  zeroes the hull `+0x3cc`, takes the unit out of its task force and sets
  state `0xb`, destroyed. The abstract departure (`FUN_005d14d0` step 5)
  agrees: units without a hyperdrive are destroyed.
- For the player's side, `FUN_005ad530` also takes the withdrawing task
  forces off the command panel. The fighter groups go through
  `FUN_005a0320` (flags `+0x154`), the capital task forces through
  `FUN_005a0410` (`+0x164`). `FUN_0059f5d0` and `FUN_0059f5f0` gate the
  selection commands in `FUN_0059fd40`.

## Change

- `begin_withdrawal` gives every living capital ship of the side order 3.
- After the side's time, a unit that cannot jump waits at its edge.
- When no unit with a working hyperdrive is left in the fight, every
  living unit of the side without one is destroyed (hull 0, not
  retreated), ships and fighter groups alike.
- `select_fighter_group` refuses while the player's side withdraws.
  Capital selection already skipped withdrawing ships.
- This corrects s54–s57, which had such units return to the fight and
  let fighter groups bound for an escaped carrier leave with it (a
  hypothesis, now dropped).

## Gates

- Workspace: 2724 passed, 0 failed.
- Six tests changed their expectations on this evidence. Four are in
  core:
  - `the_ai_side_of_a_tactical_battle_withdraws_like_the_player`;
  - `the_players_withdrawal_ends_after_the_sides_time`;
  - `the_players_fighters_with_a_hyperdrive_jump_when_the_time_ends`;
  - `a_fighter_group_bound_for_an_escaped_carrier_goes_with_it`, now
    `units_left_behind_are_destroyed_at_the_sides_end`.

  Two are in render:
  - `a_withdrawal_sends_the_fighters_to_their_carriers_and_away`;
  - `a_withdrawal_holds_fighters_with_a_hyperdrive_then_they_jump`.
- New test: `a_withdrawing_side_cannot_select_its_groups`.
- Mutants, with both packages' tests: 52 tested, 52 caught.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `bbf7cefb570c10b790db78643da267f09fad459fcd17a8cfae073fbc38cf2149`.
