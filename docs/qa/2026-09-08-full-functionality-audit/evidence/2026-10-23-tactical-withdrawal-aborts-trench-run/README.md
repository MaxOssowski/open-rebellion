# A withdrawal aborts its side's trench run (P24)

Date: 2026-10-23.

## Source

- `FUN_005ad530` posts event `0x3c` (state 8) for each withdrawing fighter
  task force on the trench run (order 6). The event runs `FUN_005caf20`,
  then `FUN_005caf50`, then `FUN_005cfec0`.
- `FUN_005cfec0` state 8 ends a run that is under way without a result.
  Each participant gets back the hull and shields that `FUN_005d08d0`
  kept when the run began: `+0x3d0` goes back to `+0x3cc` and `+0x3c4`
  to `+0x3c0`. The participant returns to state 2, and the group takes
  order 3. The run's executor goes with the new order (`FUN_005d0950`
  clears `DAT_006bc978`).

## Change

- `BattleSession::trench_run_saved` keeps the participants' hull and
  shields in `prepare_original_trench_run`, before the maneuver damage.
- `abort_original_trench_run` restores them and revives a participant
  the damage killed. It then clears the timer, the pending outcome, the
  participants and the group.
- `withdraw_fighters(side)` aborts the run first when it is the side's
  own: a participant or a live group of the side is on order 6.
- Before: the run went on after its group withdrew and could still
  destroy the Death Star.

## Gates

- Workspace: 2725 passed, 0 failed.
- New test: `withdrawing_aborts_the_trench_run`. It also checks that the
  other side's withdrawal leaves the run alone.
- Mutants, with both packages' tests: 14 tested, 2 missed, both
  equivalent.
  - `is_none() || is_some()` replaced by `&&`: with no run there is
    nothing to restore, and a run with an outcome has no timer.
  - `squad_count > 0` replaced by `>= 0`: a restored participant always
    has hull.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `9dd5826118aa8b078d9f9b4884c1a13991ac206fd24b621669ec2a2a83865a42`
  (the test changed after it; the code it builds did not).
