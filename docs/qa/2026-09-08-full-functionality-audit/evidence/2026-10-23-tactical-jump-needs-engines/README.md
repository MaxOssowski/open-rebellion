# A jump needs engine power (P24)

Date: 2026-10-23.

## Source

- `FUN_005b1b70`, a unit's "working hyperdrive", wants two things:
  - installed hyperdrives beyond their hits (`FUN_005b1bc0`);
  - engine power (slot `+0x90`).
- The tactical withdrawal asks it for the jump (`FUN_005cc300`), the
  side's wait and its event `0x2f` (`FUN_005c8780`), and the Withdraw
  command (`FUN_005a8a90` -> `FUN_005c7e80`).
- `FUN_005b2720` destroys only a unit still in the fight (`FUN_005ad870`:
  state 2, 4 or 6), so a unit already in its jump finishes it.

## Change

- `tactical::original_ship_hyperdrive_works(ships, field_effects,
  index)` adds the engine power (`original_tactical_mobility`).
- It is used for the jump, the side's wait, the side's end and
  `tactical_withdraw_enabled`.
- A unit in its jump is waited on and not destroyed.
- Before: a ship with dead engines but a hyperdrive jumped away.

## Gates

- Workspace: 2729 passed, 0 failed.
- Tests that held a ship still by giving it no engine power now give it
  power instead. The render carrier test gives `test_ship` its engines.
- New tests:
  - `a_ship_needs_engine_power_to_jump` (the engines also die mid-jump);
  - `withdraw_needs_a_ship_that_can_jump`.
- Mutants, with both packages' tests: 34 tested, 1 missed, equivalent.
  In the fighters' wait, `works || jumping` replaced by `&&` makes no
  difference, because a fighter group's hyperdrive and sublight rating
  never change in a battle.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144. That includes the
  Withdraw cases with production ships.
- Replay equivalence passed.
- Release WASM `63124629c674e0bf418bb80145b8f12f21570dbaa8c656473062a67875cd2530`.
