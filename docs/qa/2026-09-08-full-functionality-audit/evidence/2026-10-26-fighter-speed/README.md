# A squadron's speed adds its commander term (P24)

Date: 2026-10-26. Stage 5 of the fighter force mover. Follows
`../2026-10-26-fighter-squadron-frame/`.

## The original

`0x5ba190`, the speed (slot `+0x90`) of the fighter class `0x66c028`:

- 0 while `+0x3b4` is not 0;
- otherwise `base = FUN_005ad7e0(side)` (when the game's `+0x668` is 0;
  `FUN_005afc00` otherwise) `+ +0x3f8` (the sublight rating);
- `speed = base − (+0x36c × base × 0.25 (DAT_0066c6c4) + Σ FUN_005b1790
  over the tractor list +0x360)`, at least 0.

`FUN_005ad7e0` is the commander term, 1 through 9. It is the term the port
already reads for the trench run (`trench_commander_term`): the person in
the side's commander slot `+0x950`, or 1.0 with none. `FUN_0040c200` flags
only an Alliance person, so the Empire's term is 1.0.

The fighter mover's speed (`0x5ea740`) is the slowest member's `+0x90`.

## The port

- `BattleSession::alliance_commander_term` keeps the term unrounded. The
  trench run's integer rating is now taken from it.
- `original_fighter_speed` is `0x5ba190`. The squadron frame and the
  fighter mover's speed use it.
- `port:`
  - the port's squadrons keep no hit counter `+0x36c` and no tractor list
    `+0x360`;
  - `+0x3b4` is not mapped;
  - the game's `+0x668` is taken as 0, as in a tactical battle.
- A squadron with no sublight rating still flies at its commander term.

## Changed expectations

Tests that measure flight at a set speed now give their squadrons a
sublight rating one lower, since the test battles' commander term is 1.0,
so that the distances are unchanged:

- the `dogfight` helper: 10 + 1 = 11;
- the fighter group, frame, withdrawal and overshoot tests;
- `withdrawing_fighters_head_for_their_own_edge`, whose 0.4 rating now
  flies at 1.4 (0.35 a step): its expected positions moved by that.

In `a_fighter_group_moves_as_one_force`, a squadron with no sublight rating
now sets the force's speed at 1.0 rather than being skipped.

## Tests

- `a_squadron_s_speed_adds_its_commander_term`:
  - an Alliance term of 5 plus 6 gives 11;
  - the Empire adds 1, giving 7;
  - a negative sum floors at 0.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2858 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T10-46-55-609Z-3525773`) |
| Replay equivalence | passed |
| `cargo mutants` | 11 tested: 10 caught, 1 unviable |
| Release WASM SHA-256 | `beca2cf910c190789130e1f0891fdf46e7a50eac5973257c8baef17e74619468` |
