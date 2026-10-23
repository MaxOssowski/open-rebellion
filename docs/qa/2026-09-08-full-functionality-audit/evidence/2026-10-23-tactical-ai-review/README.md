# The AI commander's withdrawal review in the tactical battle (P24)

Date: 2026-10-23.

## Source

- `FUN_005c8780`, the tactical commander's step:
  - Its count `+0x40` starts at 0 (`FUN_005c7cb0`) and falls by 1.0 a
    step (`_DAT_0066cbd4`). At 0 or under, it is set to 6.0
    (`DAT_0066cba4`) and the commander reviews the battle. So it reviews
    on the first step and every sixth after.
  - The review withdraws the side (`FUN_005ad530`) when all of these
    hold:
    - the enemy's strength is above 0 (`_DAT_0066cbb4`);
    - its own strength is under `+0x24` of the enemy's, or 0;
    - the side may leave (`FUN_005c7e80`);
    - no trench run is under way (`FUN_005d0ae0`).
- `FUN_005c7f70`, the tactical strength, matches `FUN_005d0ce0` line for
  line. It differs only in the manager's lists, a Death Star bonus of the
  same value (`_DAT_0066cba0` = 20,000,000) and `FUN_005ad930`. A capital
  ship drops out when an enemy task force's target is that ship
  (`FUN_005d2190`) and the force out-damages its shield recharge
  (`FUN_005cb650`).

## Change

- `abstract_battle::tactical_side_withdraws` replaces the abstract
  `side_should_leave` probe:
  - The task forces' targets come from the units' current attack
    targets. Before, the probe had no targets, so it never dropped a
    capital ship.
  - The enemy's strength must be above 0.
  - It leaves out the abstract stalemate mark.
  - hyp: a port task force's target is its first member's.
- `ai_side_leaves`:
  - reviews on the first step and every sixth after
    (`ORIGINAL_COMMANDER_REVIEW_STEPS`);
  - does nothing while a trench run is under way (port: from state 5's
    timer to its result).

## Gates

- Workspace: 2718 passed, 0 failed.
- New tests:
  - `a_tactical_side_counts_the_targets_of_the_enemys_task_forces`;
  - `the_ai_commander_reviews_every_sixth_step`.
- Mutants on the touched core code: 22 tested, 2 missed, both explained.
  - `< ABSTRACT_RETREAT_RATIO` replaced by `<=` differs only at an exact
    ratio of 0.33.
  - `!retreat_allowed || retreat_done` replaced by `&&` is equivalent:
    the probe's `can_leave` already refuses a side whose withdrawal has
    begun. The test still checks that a second review does not restart
    the time.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `d341b1d633db8c92cee8a2ea0d45b4094d91e6b3755942089507a4593be09eb3`
  (the tests changed after it; the code it builds did not).
