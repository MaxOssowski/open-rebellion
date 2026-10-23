# Withdrawing ships stay in the fight until their jump (P24)

Date: 2026-10-23.

## Source

- The capital weapon loop `FUN_005b3a40` (also `FUN_005b3f10`) fires only
  from a unit that `FUN_005ad870` counts as in the fight (state 2, 4 or
  6) and that is not in state 6.
- Its target query `FUN_005cdf50` takes enemy units in state 2 or 4, not
  6.
- Order 3 (`FUN_005ad530`) leaves a unit in state 2. Only its jump moves
  it to state 6, then 7 and 8 (`FUN_005cc300`, `FUN_005b1e40`). So a
  withdrawing ship fires and can be hit through its whole withdrawal,
  until its jump starts.

## Change

- `TacticalShip::jumping` is true once the jump's clock has started.
- The core combat filters used to leave out every withdrawing ship, a
  leftover of the port's invented 5 s retreat. They now leave out only a
  jumping one. These filters cover:
  - capital weapon candidates and the per-side combat lists;
  - `fire_side` sources and targets;
  - capital fire at fighters;
  - fighter targets and strengths;
  - attack-target eligibility, the first target and the target refresh;
  - collision.
- Not changed (port): the player's selection, task-force and escort
  controls in the window still skip withdrawing ships.
- Not changed (port): fighter groups are never left out, so a jumping
  group can still fire and be hit for its 4000 ms.

## Gates

- Workspace: 2722 passed, 0 failed.
- One unit test and one browser fixture used a withdrawing enemy as the
  example of an ineligible target. On this evidence it is now one whose
  jump has started:
  - `attack_target_lifecycle_reacquires_same_class_and_clears_exhausted_lists`;
  - `configure_attack_target_lifecycle_fixture`.
  The first browser run failed 4 cases of
  `production-attack-target-lifecycle-presentation` until the fixture
  changed. The second run passed.
- New tests:
  - `a_withdrawing_ship_can_be_hit_until_its_jump_starts` fails with the
    old target filter.
  - `dead_and_jumping_ships_drop_out_of_the_fight`.
- Mutants, run with both packages' tests (`--test-package rebellion-core
  --test-package rebellion-render`): 69 tested, 15 missed.
  - All 15 are in filters where only the predicate was renamed
    (`retreating` to `jumping()`).
  - The exclusion of dead and jumping ships is layered: the target
    refresh, the candidate collection and the fire guard each apply it.
    So breaking any one layer is masked by the others. The broad new test
    checks the outcome and shows none of them firing or hit.
  - The render fixture function is exercised by the browser suite only.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144 on the second run.
- Replay equivalence passed.
- Release WASM `2292aa4ceae8c7beac0f604ab276a00c5a8dc345a2f223d1910b5ed422b5ae82`.
