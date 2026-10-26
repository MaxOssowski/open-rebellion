# Busy forces keep out of the commander's pairing (P24)

Date: 2026-10-26. Follows `../2026-10-26-computer-targets/`.

## The original

`FUN_005ed5f0` (the commander policy's slot `+0x0c`) returns 1, so
`FUN_005c9200` leaves the task force alone, when:

- the force's live order `+0x64` is 2 (Recover), 3 (withdraw) or 6 (the
  Death Star run); or
- it is a fighter force (class 4) with a member in state 1 or 3; or
- its live target is still eligible (`FUN_005ed550`), and its combined
  score is under 300 and not -1.

## The port

`AbstractBattle::keeps_target` now leaves out a force whose first member
is withdrawing (`retreating`) or on `Recover` or `AttackDeathStar`. The
battle 3D commander (`commander_targets`) and the abstract battle share
it, as both share `FUN_005ed5f0` in the original.

`port:` the fighter members' states 1 and 3 are not mapped to the port's
states yet.

## Tests

- `busy_computer_forces_keep_out_of_the_pairing`: of an idle ship, a
  withdrawing ship, a recovering group and a Death Star group on the
  computer's side, only the idle ship is paired. With the new test in place
  and the exclusion disabled, the test fails.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2847 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T06-37-37-643Z-1197703`) |
| Replay equivalence | passed |
| `cargo mutants` | 4 tested: 4 caught |
| Release WASM SHA-256 | `be6d51fbe68bb4a2372c633bd96567d227bd7155714ae99d81ddf0f3bd1dcec2` |
