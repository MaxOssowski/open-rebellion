# Hull size, heading and collision box in the tactical view (P24)

Date: 2026-10-25.

## Source

`REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`) was read statically and compared
with A0 TAC-01-C003 (ignored store; only the cell ID is cited).

## Findings

- **Hull meshes are scaled by 3.** The renderer's hull cache (`+0x198`,
  vtable `0x0066c990`) loads each mesh with `0x005d5b10`. That function
  calls `Load` for resource type `0x12d` (301), then
  `Scale(DAT_0066d2b0, DAT_0066d2b0, DAT_0066d2b0)` (`0x005d5ba3`).
  `DAT_0066d2b0` is 3.0. The port drew the authored size, so every hull came
  out a third of its size: in TAC-01-C003 the Victory Destroyer covers
  17 x 15 px, while an unscaled model would be about 9 x 7 px.
- **Hulls turn to face their heading.** `0x005d3500` (vtable `0x0066d198`
  + `0x1c`) is called from `FUN_005c1f10` with the hull's position and its
  `+0x10` forward vector `O`. It builds a matrix for
  `AddTransform(REPLACE)`:

  | Row | Value |
  |---|---|
  | local x | `(-Oz, 0, Ox)`, normalised |
  | local y | `(0, 1, 0)` |
  | local z | `-O` |
  | translation | the position |

  The port drew every hull without a rotation. It now passes the three axes
  to the shader, for both the vertices and the normals.
- **The collision box comes from the medium model, scaled.**
  - `FUN_005ab0e0` takes the box that vtable `+0x2c` returns. That is
    `0x005d43b0`, which returns `+0xcc`.
  - `FUN_005d26c0` fills `+0xcc` with `GetBox` of `+0x48`, the base + 1
    member of the family, after the scale.
  - The port used the base member's unscaled box.
  - The port also learned the box only after the first frame. The paused
    opening formation therefore moved once
    (`refresh_original_task_force_geometry`). The box is now read before the
    first frame, as the original creates it with the object.

## Tests

- `a_hull_mesh_loads_three_times_its_authored_size` checks the bounds,
  the vertices and the collision envelope.
- `a_hull_turns_about_the_vertical_to_face_its_forward_direction` compares
  `original_hull_axes` with the `0x005d3500` matrix. Its cases include a
  forward vector that is not unit length.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2759 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| Release WASM SHA-256 | `5a8d353f50ef159bfbddb85efe2c484cefab58a6f470243364a27cf7ef3ff2b2` |

`cargo mutants` left 26 mutants after the first run:

- After a new non-unit test case, all 10 `original_hull_axes` mutants are
  caught.
- The rest are in GPU drawing and in state that needs a loaded renderer
  (`participant_collision_envelope`, `apply_original_collision_envelopes`).
  The browser matrix's stable-scene and position checks cover them.

## Still open

- TAC-01-C003's battle extent is unknown, and it holds a Victory Destroyer
  where the port's fixtures use other hulls. The exact pixel size of a hull
  has therefore not yet been compared like for like.
- The port's hulls look darker than the original's (lighting).
