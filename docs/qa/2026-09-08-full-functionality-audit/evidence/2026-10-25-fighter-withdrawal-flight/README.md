# A withdrawing fighter group's flight to its exit line (P24)

Date: 2026-10-25.

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`), from the
objdump disassembly. Ghidra 12.1.4 defines no function at these vtable
targets.

### `FUN_005c6610`, order 3

1. **Exit point.** The force's mover `+0x54` (`0x005c72b0`, size `0x144`,
   vtables `0x66cb50` and `0x66cb30`) gets the side vector times -1.0
   (`_DAT_0066cb10`). Its target point is:
   - x and y: the force's own (`mover +0x3c`);
   - z: the extent `+0x8cc` times the vector's z, times 0.5
     (`_DAT_0066cb14`).
2. **The move.** It calls the mover's slot `+0xc` (`0x005ec0d0`) with that
   point, the step's milliseconds and 1.0 (`0x005c66b7`).
3. **Jumps wait for arrival.** On a return of 1 (still moving), the step
   ends (`0x005c66e9`). Only on 0 does it go through the members for their
   jumps: working hyperdrive, within 8.0 of the line, the side's time over,
   then state 6 and event `0x2d`.

### `0x005ec0d0`, the mover's move

- `d = target - position` (position at the mover's `+0x3c`). If `|d|` is
  within the radius argument, it clears the velocity and returns 0.
- Otherwise it moves `d / |d| * speed * ms * 0.001` (`DAT_0066d894`). The
  step is not clamped to `|d|`, and it returns 1.
- The speed is slot `+0x34` (`0x005ec0b0`): `+0x124`, or 0 without members.
  `0x005ea740` sets `+0x124` to the lowest engine power (member slot
  `+0x90`) among members that have any.

`FUN_005cc300` (capital ships) makes the same call with radius 1.0 and the
same early exit (`0x005cc3ef`).

## Change

- The port jumped a withdrawing fighter group as soon as it was within 8.0
  of its line, even while still flying.
- It also clamped the last step to land exactly on the line.
- Now:
  - the group flies a full step at its engine power while more than 1.0
    (`ORIGINAL_FIGHTER_WITHDRAWAL_ARRIVAL_RADIUS`) from the line;
  - it can overshoot;
  - its jump is tried only from the step it has arrived.
- `port:` the group's position is its member's, and its engine power is
  its sublight rating.

## Tests

- `a_withdrawing_fighter_group_flies_to_its_edge_before_it_jumps`. Its
  expectation changed on this evidence: at 7.0 from the line the group is
  still moving, so it jumps only after reaching 1.0.
- `a_withdrawing_fighter_group_overshoots_its_line_by_a_full_step` (new):
  - starts 2.0 from the line, with a 2.5 step;
  - ends 0.5 beyond the line, then stays there.
- Both tests fail on the old code.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2767 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| `cargo mutants` | 12 of 12 viable mutants caught |
| Release WASM SHA-256 | `125613d82f42198a966a9c9e7af3db2b90b9239ed2e243eec759cd3499b32384` |

## Open

The capital ships' withdrawal (`FUN_005cc300`) is not yet checked against
this reading:

- the same arrival gate;
- its member choice: `0x005c6070` returns one member, where the port tries
  every ship;
- whether its mover is this class.
