# Escorting fighter groups close on and circle their ship (P24)

Date: 2026-10-26. Stage 7 of the fighter force mover. Follows
`../2026-10-26-squadron-height-bands/` and `../2026-10-25-fighters-closing/`.

Before, an escorting fighter group flew straight at its ship at its engine
power and stopped 10 units away. That stand-off was the port's own.

## The original

- `FUN_005ca6d0` (a task force's slot `+0x48`, the order's install), case 1
  (Escort):
  - clears the route list `+0x74`, but not the target list `+0x8c`;
  - clears `+0x8c` only when the order is left (the `iVar4 == 1` test at the
    top);
  - for a capital force (type 3) rebuilds the mover as Surround
    (`FUN_005cb1a0(1)`);
  - installs the executor class `0x66ceb8`. Its update `0x5ae1e0` is an
    empty `ret` and its `+8` (`FUN_005d0af0`) only clears its own `+0xc`.
- `FUN_005cb600` (slot `+0x1c`) is the only writer of `+0x8c`: it clears the
  latch `+0xb0`, empties the list and adds the object.
- `FUN_005c6610` (a fighter force's order step) tests no order other than
  3, 6 and 11. So an escorting group takes `LAB_005c6c6f` with the escorted
  ship from `+0x8c`, like any target:
  - one step toward it at radius 2.0;
  - within the radius, the circle round it.
- If that ship is no longer in the battle (`FUN_005ad870`), the step sets
  the latch `+0xb0` and stops moving the anchor.

## The port

- `BattleSession::fighter_force_target`: the escorted ship for order
  Escort, else the attack target.
- `fighter_forces` no longer leaves out escorting groups.
- `update_engagement_bands` and `advance_fighter_approach` use that target.
- `advance_original_escort_movement` keeps only the capital ships' escort.
  Its fighter loop (straight flight, 10-unit stand-off) is gone.
- `port:` when the escorted ship dies, `refresh_original_escort_targets`
  still clears the group's order (it keeps the white escort box from going
  stale). With no target, the force's anchor stands, as under the latch.

## Tests

- `an_escorting_fighter_group_closes_on_and_circles_its_ship`:
  - far off, the anchor moves exactly as an attack on the same ship would,
    and the squadron flies;
  - with a step within 2.0, the anchor goes onto the circle (radius 1.5,
    half of it above);
  - the force gets a band on the escorted ship;
  - with no ship to escort, it stands.
- `only_free_fighter_forces_close_on_targets` now includes the escorting
  group (`[0, 2]`), per the facts above.

Both tests fail without the change.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2864 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T11-46-58-554Z-88858`) |
| Replay equivalence | passed |
| `cargo mutants` | 9 tested: 8 caught, 1 unviable, 0 missed |
| Release WASM SHA-256 | `05d85d8b7600450d974635347a01d80894b7efd9e80522d13b70c09b79ed450b` |
