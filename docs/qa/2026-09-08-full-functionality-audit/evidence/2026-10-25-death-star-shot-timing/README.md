# When the superlaser's beam starts and its target dies (P24)

Date: 2026-10-25.

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`), from the
objdump disassembly.

### The battle manager's event turns

Both battle managers (`0x005be160`, vtable `0x66c7c8`; `0x005bf830`,
vtable `0x66c880`) run events in turns.

- **Turn length.** `+0x8a8 = 0x3e8`, 1000 ms (`0x005bea9c`, `0x005c0545`),
  measured with `timeGetTime` against `+0x89c`.
- **At each turn** the manager advances `+0x878` (`0x005bec3c`). It then
  runs the events stamped `+0x878 - 1` (`FUN_005a96b0`, which runs a stamp
  equal to its argument, drops older ones and keeps newer ones).
- **New events are stamped `+0x87c`.** Slot `+0x28` (`0x005ad4a0`) stamps
  `+0x87c`, and slot `+0x2c` queues the event by side. `+0x87c` starts at
  `+0x8ac` = 3 (`0x005a877b`, `0x005be233`, `0x005bf929`, never written
  otherwise). It also advances once per turn (`0x005bedf6`), so it stays
  three turns ahead of `+0x878`.

### The shot, `0x005ba720`

Both the player's target click (`0x005a588e`) and the AI (`0x005c8c33`)
call it. It:

1. clears the charge `+0x6c`;
2. posts event `0x44` through slot `+0x28`, stamped `+0x87c`;
3. builds a type-3 event (`0x005ce500`: time `+0x20`, type `+0x28`, target
   `+0x29`, source `+0x2d`), stamped `+0x87c + 1.0` (`DAT_0066c70c`), and
   queues it through slot `+0x2c`.

The dispatcher (`0x005a9780`, table `0x5a9bc4`, type − 3) sends:

- **Event `0x44` to `0x005ba520`.** It starts the beam visual
  (`0x005c12d0`) and, for a target still in the fight, queues sounds
  `0x14` and, for the Alliance player, `0x91`.
- **Type 3 to `0x005b2b60`.** With the Death Star as its source
  (`FUN_005a9330`, id `+0x18`) and a target still in the fight
  (`FUN_005ad870`), it destroys the target (slot `+0xd0`) and starts its
  explosion (`0x005c13d0`, 4).

A shot in turn k therefore starts its beam at turn k + 4. It kills the
target one turn later, at k + 5, as the beam's 1 s growth (`0x005da830`)
ends.

## A0

TAC-05-C020's run (`~/VMs/rebellion-win10/ds-frames`, ignored store) was
taken by QMP every 2.02 s after the shot was ordered while paused and the
battle unpaused:

| Frame | Seconds after `run-00` | Target |
|---|---|---|
| `run-01` | 2.04 | present |
| `run-02` | 4.06 | present |
| `run-03` | 6.08 | gone |

By the turns, the kill falls between 4 and 5 s of battle time after the
unpause. The port's old 2 s would have removed the target before `run-01`.
The beam README of the same day said "between 2 s and 4 s"; the frames
show 4 to 6 s. No frame catches the beam: none of `run-00` to `run-04`
has a pure green pixel in the aperture.

## Change

- Before, `commit_death_star_shot` started the beam at once and destroyed
  the target 2 s of presentation time later
  (`ORIGINAL_PROJECTILE_DURATION_LONG`).
- Now the shot is stamped for the turns:
  - the beam starts (`superlaser_fired`) at turn k + 4, aimed at the
    target where it is then;
  - the type-3 event at turn k + 5 kills the target only if it is still
    in the fight and the Death Star is not destroyed.
- New constants: `ORIGINAL_TACTICAL_TURN_STEPS` (4) and
  `ORIGINAL_TACTICAL_EVENT_LEAD_TURNS` (3).
- The beam is drawn only once it has started.
- The browser journey waits for `superlaser_fired`, then takes the beam
  frame 0.9 s later as before.
- `port:`
  - the turn boundary falls on every fourth 250 ms step, counted from the
    first combat step;
  - the beam's start and the kill run at the start of that step.

## Not changed

Every event posted through slot `+0x28` is stamped this way. If the
player's other orders (movement, attack, withdrawal) take that path, they
also wait three to four turns in the original; this was not traced. The
port applies them at once. A general event turn queue would change the
port's architecture, so it is left for a separate decision.

## Tests

- `a_superlaser_shot_fires_after_the_event_lead_and_kills_a_turn_later`
  (was `death_star_superlaser_resolves_after_the_source_long_projectile_duration`).
  Its expectations changed on this evidence:
  - presentation time alone neither fires nor kills;
  - the beam starts at step 16 (turn 4), aimed at the moved target;
  - the kill comes at step 20 (turn 5).
- `a_superlaser_shot_spares_a_target_that_has_jumped_or_a_lost_death_star`
  (new): a shot from step 6 is stamped for steps 20 and 24, and a jumping
  target or a destroyed Death Star leaves the target alive.
- Both tests fail with the turn step removed from `step()`.
- `the_beam_is_drawn_only_from_its_start_through_its_second` (new): the
  drawn share is none before the start, `elapsed` through 1.0 s, and none
  after.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2770 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed (before the beam-progress helper was split out; the check is unchanged) |
| Replay equivalence | passed |
| `cargo mutants` | 37 tested, 36 caught |
| Release WASM SHA-256 | `659d08260625dd196866b55214563edd7f0faf1cffb59d4703b6dc293b815f49` |

The missed mutant replaces the GPU draw `draw_tactical_death_star_beam`
with `()`. It has no unit harness; the browser journey's
`death_star_beam ... visible=true` log and green-pixel check cover it.
