# A large computer force switches to Surround (P24)

Date: 2026-10-25. This follows `../2026-10-25-fighters-closing/`. Every
capital force kept the tactic it started with, which is Stand Off. In the
original, a large force on the computer's side switches itself to Surround.
Source: `~/Games/rebexe-decompile/decompiled-typed/FUN_005cc300.c` (the
branch at `0x005ccdda`), `FUN_005c5eb0.c` and `FUN_005c9be0.c`, outside the
repository.

## The original

`FUN_005cc300` (the capital force's per-frame approach, see
`../2026-10-25-closing-on-targets/`) ends its out-of-band branch with a
check. A force switches when all four of these hold:

- its tactic object's `+0x38` is 2 (Stand Off);
- its `+0x34` is 0 (a local force, not a remote one);
- its side's entry in the battle's side table (`+0x8`) is 1 (the computer
  commands that side);
- `FUN_005c5eb0` returns more than 7. That function counts the force's
  members by walking its list (slots `+0x10` and `+0x8`).

When they hold, the force calls `FUN_005c9be0(1)`, the SetTactic routine.
SetTactic:

- writes the tactic to `+0x38`;
- posts event `0x3d` (a task force's tactic) with the force's id `+0x18`,
  the new tactic, and whether the force is on the local side.

So a computer force of 8 or more ships in Stand Off takes Surround when it
is out of its band. A force of 7 does not, and neither does a player's
force.

## The port

`BattleSession::advance_engagement_approach` (tactical.rs), at the same
place in the out-of-band branch: when the lead unit's tactic is Stand Off,
its side is not the player's, and the force has more than 7 members, every
member's tactic becomes Surround. The step posts
`TacticalTurnEvent::TaskForceTactic` with the force's side, its task force
and Surround. As with the other tactic events, it posts only for task forces
0 through 7.

- `port:` the computer's side is the one the player does not command
  (`is_attacker != player_is_attacker`). The port has no networked remote
  forces, so `+0x34` is always 0.
- The step of that frame already uses Surround's approach step.

## Tests

- `a_large_computer_force_switches_to_surround` (abstract_battle.rs): a
  computer force of 8 takes Surround and posts the event; a force of 7 and
  a player's force of 8 keep Stand Off and post nothing.

## Gates

The test fails without the change: the computer force of 8 gives
`(false, false)` in place of `(true, true)`.

| Gate | Result |
|---|---|
| Workspace tests | 2820 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | 13 tested: 12 caught, 1 missed |
| Release WASM SHA-256 | `e517ded3bb73fde4faeece777c85af62c890c7cd7371d10f1ba46a988d574fe0` |

The missed mutant is equivalent. It changes the event guard `task_force < 8`
to `<= 8`. `capital_forces` builds forces of more than one member only from
the slots 0 through 7. A unit without a group (task force 255) is a force of
its own, of one member. So a force of more than 7 members always has a task
force below 8.
