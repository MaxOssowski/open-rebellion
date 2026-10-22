# The tactical battle's commander rating (P24)

Date: 2026-10-23.

## The original

- The trench run compares an inclusive 1..100 draw with
  `FUN_005ad7e0`'s commander rating.
- `FUN_005ad7e0` reads the side's commander slot `+0x950`. With a person
  there, the rating is its `+0x50` minus `DAT_0066c27c` (-1.0), clamped to
  `DAT_0066c298` (1.0) .. `DAT_0066c2b8` (9.0); with none it is 1.0.
  (Constants read from `REBEXE.EXE`.)
- The only writer of `+0x950` is `FUN_005a93e0` (called from
  `FUN_00596340`), for a person flagged `+0x58`, the flag `FUN_0040c200`
  gives the Alliance's battle commander.
- The abstract battle reads the same person and constants
  (`DAT_0066c3e4` = -1.0, `FUN_0040d680`), ported as
  `abstract_battle::trench_commander_term`.

## Change

`BattleSession::new_for_fleets`, which the 3D battle also starts from,
sets `trench_run_success_rating` from `trench_commander_term` instead of
the unassigned fallback 1. The integer draw drops the fraction.

## Gates

- Workspace 2682 passed, 0 failed. The new test
  `a_tactical_session_takes_the_alliance_commanders_rating` fails without
  the change (left 1, right 6).
- Mutants: the diff gives one mutant (the whole function), unviable; the
  change has no operator to mutate.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144. The fixtures set the
  rating explicitly.
- Replay equivalence passed.
- Release WASM `b04fc4da9f3353bf6e4ff2ceef74d3439980769e7b9203061af0ab3776230c16`.
