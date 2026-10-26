# Units move smoothly between steps (P24)

Date: 2026-10-26. The user asked for checks of movement smoothness, fighter
movement and single-ship attack orders. This stage settles the first;
the browser checks below cover all three.

## The problem

- The original moves its units every frame by that frame's milliseconds:
  the manager's frame function `0x5a9ec0` runs the forces' steps and the
  units' moves, and a turn is 1000 ms (`+0x8a8`).
- The port advances the battle in fixed 250 ms steps,
  `ORIGINAL_TACTICAL_STEP_MILLISECONDS`, at 4 steps a second at speed 1.
  That keeps a battle deterministic for replays and for the abstract
  battle, which run the same simulation.
- The renderer drew each unit where the last step left it, so ships and
  fighter groups jumped 4 times a second.

## The change

`port:` presentation only; the simulation is untouched.

- `TacticalState` keeps `TacticalPoses`: every ship's position and
  forward, and every fighter group's position, taken just before each
  step, with the step number.
- When the scene is built, a unit is drawn `step_accumulator` of the way
  from that pose to its current one (`presented_pose`; the forward is
  blended, then renormalized).
- Poses are used only for the step right after the one they were taken
  at (`before_latest_step`). A new battle, or a fixture stepped directly,
  draws the current poses.
- Impact and field effects follow their ship's drawn position.

## Browser checks

These used Arch Chromium 155 (swiftshader) at 640 × 480, fixture 65800
(`production-participants`: an Alliance Mon Calamari cruiser and fighter
group against an Imperial Strike cruiser and fighter group). The script
and its screenshots stayed in the session scratchpad.

1. **Smoothness.** The selected cruiser was ordered to attack capital
   ships, unpaused, and captured in a burst of 12 frames about 83 ms apart.
   Its screen centroid moved in every frame, about 0.8 px each:
   (17.2, 124.3) → (18.1, 123.4) → … → (24.7, 116.3). The old renderer
   would have shown three still frames, then a jump.
2. **Attack Capital Ships for one ship** (order 5, `capital_members=1`):
   - the cruiser takes the Strike cruiser as its target (the attack line
     is drawn) and closes on it;
   - from about 50 s both hulls show impacts;
   - it then holds in its band.
3. **Attack Fighters for one ship** (order 4): the cruiser targets the
   enemy fighter group and closes, and its projectiles fly from its
   position (186.6, 292.0) to the group (326.2, 171.1).
4. **A fighter group ordered to attack capital ships** (selected through
   its group button, order 5, `fighter_members=1`):
   - it flies to the Strike cruiser and reaches it by 20 s;
   - it is destroyed by 30 s, with the `FighterLosses` voice.

Found during the checks, and not part of this stage: **the computer side
never acts.** Its cruiser and fighter group stood still for the whole
70 s; the cruiser fired only at what came close. The port has only the AI
commander's withdrawal review (`FUN_005c8780`). Each review in the
original also runs `FUN_005c9200` and the commander's vt `+0x14` and
`+0x18`, which are not ported. Reading them is the next stage.

## Tests

- `units_are_drawn_between_their_last_two_steps`:
  - a quarter of the way;
  - both ends;
  - no previous pose;
  - a reversal that falls back to the current forward;
  - `TacticalPoses::of` and the step-number guard.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2845 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T05-29-12-366Z-977538`, before the guard's refactor into `before_latest_step`) |
| Replay equivalence | passed |
| `cargo mutants` | 13 tested: 7 caught, 2 unviable, 4 missed at first; after the extra tests 15 tested, 13 caught, 2 unviable |
| Release WASM SHA-256 | `5be44d19e7891ef9b95f5d8232440ee390476618c24416e646f264a43899c8d5` |

The guard's refactor keeps the same behaviour. The browser checks above
ran on the refactored build.
