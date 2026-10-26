# The computer's commander gives its forces targets (P24)

Date: 2026-10-26. Found during the browser checks of
`../2026-10-26-smooth-unit-motion/`: the computer side never acted. Its
cruiser and fighter group stood still for 70 s.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`, plus
`objdump`; outside the repository).

## The original

- `FUN_005be160` builds a commander for each side: vtables `0x66c7a0`
  (side 0) and `0x66c778` (side 1), both built by `FUN_005c7cb0`. In single
  player, `FUN_005be350` gives the side that is not local mode 1, the
  computer.
- Each turn, the commander's step `FUN_005c8780` rebuilds its target list
  `+0x90` (`FUN_005c8e10`): the enemy's capital ships and fighters, by
  priority `FUN_005ed8e0`.
- On each review (count `+0x40`, every sixth), it calls `FUN_005c9200`
  before the withdrawal test:
  - its task forces that do not keep their target (`FUN_005ed5f0`) are
    paired with that list (`FUN_005ec740`, limit 180);
  - a force led by a laser carrier (`FUN_005b7ce0`) is paired with the
    fighters first, the others with the ships first.

  This is the same pairing the port already runs in the abstract battle.
- **The commander gives no orders, only targets.** `FUN_005a8fc0` links a
  force to its target:
  - it sets the commander's own view at once (`FUN_005cb950`);
  - it posts event `0x36` at the command stamp. The handler
    (`FUN_005cae10` → `FUN_005cb600`) replaces the force's live target list
    `+0x8c` three turns later.
- The approach (`FUN_005cc300`, `FUN_005c6610`) and the fire code follow
  the first live target (`FUN_005d2190`). No order is needed.
- No unit starts with an order or a target (`FUN_005ab650`), and there is
  no default target.

## The port

- `abstract_battle::commander_targets(session, alliance)`:
  - runs the abstract battle's `assign_targets` on a copy of the session.
    Each task force's current target is its first member's;
  - returns every member's target.
- `BattleSession::ai_side_leaves`, the computer commander's review, now
  links targets first, then runs the withdrawal test.
  - A member whose target changes gets `TacticalTurnEvent::TaskForceTarget`
    (`0x36`) at the step's event stamp.
  - A kept target posts nothing.
- The event's handler sets each living member's `attack_target`.
- `port:` the port keeps a target per unit, so every member of a force
  takes the force's target.

Not yet ported:

- the keep-target exclusions for orders 2, 3 and 6;
- Observe handing the player's side to the commander;
- the Empire's AI superlaser shot;
- the Alliance's AI trench run;
- the first-review regroup (`FUN_005c8300`).

## Browser check

Fixture 65800, unpaused with no player orders, 60 s, frames every 10 s.
The script and frames stayed in the session scratchpad.

- The Imperial Strike cruiser now flies from its lane toward the
  Alliance cruiser and stops in its band.
- The Imperial TIE group attacks the Alliance fighters. The Alliance voices
  `FighterLosses` (`0x6d`) and `TargetDestroyed` (`0x60`) play.
- Projectile frames were logged in 661 scene updates.

Before this stage the same run showed no movement and no fire until the
player's ship came close.

## Tests

- `the_computer_commander_links_its_forces_to_targets_on_its_command_turn`:
  - turn 1's review queues two links at stamp 3, with no target yet;
  - from turn 4 the turbolaser cruiser targets the player's ship and the
    laser TIE group the player's fighters;
  - the player's units get none;
  - a kept target posts no new link at the turn-7 review;
  - the cruiser then closes.
- These tests had assumed an idle enemy:
  - the closing test now holds the computer's ship without engines, so
    the distances still measure the player's force;
  - three queue tests now leave the commander's links out of their
    counts.

The new test needs the new event, so it does not compile without the
change.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2846 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T06-16-55-681Z-1130104`) |
| Replay equivalence | passed |
| `cargo mutants` | 6 tested: 5 caught, 1 unviable |
| Release WASM SHA-256 | `1554080341414fa607e22eec8fcd2bb5ab4b9c9a7283a658934185d55b09f703` |
