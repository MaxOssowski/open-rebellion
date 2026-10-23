# Fighter hyperdrives in the withdrawal and the departure (P24, F-033)

Date: 2026-10-23.

## Source

- `FUN_005ad530` (the player's withdrawal), read with the task-force
  vtables from the audited `REBEXE.EXE`:
  - The first loop walks the side's task forces. A fighter task force
    (class 4: vtable `0x66bee0`, `FUN_005a6700`) takes order 2 when its
    first unit's hyperdrive count (`FUN_005b1bc0`) is 0, and order 3
    otherwise (disassembly `0x5ad5ea`..`0x5ad60f`). Every other task
    force takes order 3.
  - The second loop gives each capital ship its own task force
    (`FUN_005aded0`, class 3) with order 3.
  - Slot `+0x64` is `FUN_005ca530` (the pending order); `FUN_005ca6d0`
    installs the order's executor. Order 2 steps with `FUN_005cf980`, the
    recovery to a carrier. Order 3 steps with `FUN_005ae1e0`, which is
    empty: the force holds.
- `FUN_005d0f50` and `FUN_005d14d0` step 5 test each fighter group with
  `FUN_005b1b70` like a capital ship. A group with a working hyperdrive
  escapes by itself (`FUN_005af910`).
- FIGHTSD: the four Alliance classes have hyperdrive 60; of the Empire's,
  only record 8 has one.

## Change

- `TacticalFighter::has_hyperdrive`, from the class's primary or backup
  rating. hyp: the count is built as for a capital ship. port: fighters
  take no hyperdrive hits.
- Abstract battle (`abstract_battle::hyperdrive_works`): a fighter group
  with a hyperdrive and engine power can take its side away and escapes
  by itself. Before, the port gave fighters no hyperdrive, so they left
  only with an escaping carrier. That rule stays, as a hyp, for groups
  without a hyperdrive.
- Tactical withdrawal (`tactical_view::withdraw_player_fighters`): a group
  with a hyperdrive holds (order 3) and jumps when the side's time ends.
  A group without one takes the recovery to a carrier (order 2). Before,
  every group went to a carrier (hyp: "order 3 is the recovery", now
  disproved).
- Not ported (hyp: the units leave together when the time ends): the
  side's wait on its units' states 2/4/6/7/8 and event `0x2f` for the
  units without a hyperdrive (`FUN_005c8780`, `FUN_005ad870`,
  `FUN_005b1e40`). Also not ported: the trench-run cancel (event `0x3c`).

## Gates

- Workspace: 2710 passed, 0 failed. The new tests
  `a_withdrawal_holds_fighters_with_a_hyperdrive_then_they_jump` and
  `a_fighter_group_with_a_hyperdrive_leaves_without_a_carrier` fail
  without the change.
- Mutants on the touched core and render code: 27 tested, 27 caught. The
  first run missed 9 (class hyperdrive and withdrawal-end conditions,
  tested only from render); two core tests closed them.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed. The seed-42 golden is unchanged.
- Release WASM `505216adef92905ee57d79c5178bd5aeb4456f614b600bea5d95aab4d4842224`
  (the tests changed after it; the code it builds did not).
