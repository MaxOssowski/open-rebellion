# The withdrawal warning; the invented partial retreat removed (P24)

Date: 2026-10-23.

## Source

- `FUN_005a0240` walks the player's capital ships (`+0x58`). On the first
  one that `FUN_005b1b70` says cannot jump (a hyperdrive and engine
  power), it queues voice `0x95` (Alliance) or `0x10e` (Empire).
- The original withdraws only a whole side (`FUN_005f1170` -> command
  `0x7a` -> `FUN_005ad530`). Since s60 the side's end destroys every unit
  without a working hyperdrive.

## Change

- `queue_withdrawal_hyperdrive_warning` asks
  `original_ship_hyperdrive_works`. Before, it asked only the hyperdrive
  and skipped ships already marked withdrawing.
- The port's R key ("retreat selected ships") is removed. It was a port
  invention that marked ships withdrawing without starting the side's
  time, so they would have flown to the edge forever. Under s60 a partial
  withdrawal would also have destroyed unselected ships.
- The unused `TacticalAction::RetreatSelected` and its `main.rs` arm are
  removed.

## Gates

- Workspace: 2730 passed, 0 failed.
- New test: `the_withdrawal_warns_of_a_ship_that_cannot_jump`.
- Mutants, with both packages' tests: 6 tested, 6 caught.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `ede2108b2b4fe86633c7efba6fbc36d39b74f25f6cb55c5f11350b2d55d7ed57`.
