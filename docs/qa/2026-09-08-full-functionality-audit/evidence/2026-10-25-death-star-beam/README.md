# The Death Star superlaser beam (P24)

Date: 2026-10-25.

## Source

`REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`), read statically.

- `FUN_005ba5e0` commits the shot and calls `FUN_005c1200` on the Death
  Star's visual. That visual is `FUN_005d4d10`'s backdrop object
  (vtable `0x0066d278`).
- Its `+0x20` method (`0x005d59e0`) builds the beam with `FUN_005da150` on
  the decal frame `+0x4c`, aimed at the target.
- `FUN_005da150` builds two meshes:
  - **A box.** Eight vertices at x, y = ±0.005 and z = 0..1, with
    `SetColorRGB(0, 1, 0)`.
  - **A cone.** Eight unit-ring vertices `(cos k45°, sin k45°, 0)` and an
    apex `(0, 0, 1)`, with eight faces round the apex and the same pure
    green. It is scaled `(0.06e, 0.06e, 0.15e)` (`DAT_0066d4e0`,
    `DAT_0066d4e8`) and translated `(0, 0, -10)`.

  The beam frame `LookAt`s the target and is stored at manager `+0x654`.
- `0x005da830` runs each frame from `0x005c2d00`.
  - On its first frame it measures the distance D to the target.
  - It scales the box along z cumulatively, so that its length is t·D. The
    timer t comes from `QueryPerformanceCounter` (`FUN_005fbbe0`) and is
    clamped to `DAT_0066d4bc` (1.0 s).
  - It clears `+0x28` once t passes 1.0.

  The beam therefore grows from the Death Star to the target in one second,
  then ends.

## Port change

- The beam starts at the Death Star decal's centre (`-2e`, `0.11e`,
  `-0.09e`). Before, it started at the Death Star's old field position.
- It grows over 1 s and is not drawn after that.
- It is pure green. The sub-pixel box is drawn as one device-pixel line, as
  the projectiles already are. The green cone is drawn unlit.
- Before, the beam was drawn as a projectile, 12 units long and 2 s long, in
  `(0.25, 1, 0.25)`.
- The browser journey now takes the beam frame at 0.9 s. In the first
  Imperial view the beam enters from the right after about 0.85 s, and it
  must be drawn from the decal toward the target
  (`[tactical_3d] death_star_beam ... source=FUN_005da150,0x005da830`).

## Still open

- The target's destruction is still applied 2 s after the shot
  (`ORIGINAL_PROJECTILE_DURATION_LONG`). The event that destroys the target
  in the original was not traced here. A0 TAC-05-C020's 2 s frames show the
  target gone between 2 s and 4 s after the shot.
- The cone's roll about its axis follows `LookAt`'s constraint, which has
  not been compared. The `cargo mutants` survivors in the ring's basis are
  equivalent: they give the same cone, turned.
- The beam has no A0 capture: frames 2 s apart missed it.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2760 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| Release WASM SHA-256 | `cea23b56ddf9923fc35efca8d55717d01a4cc6f548442f89e5429a61ed54e561` |

`cargo mutants` missed 41 mutants:

- The cone-mesh test now checks the 45° ring spacing and a vertical axis.
  Of the remaining cone mutants, only the equivalent basis ones survive.
- The rest are the GPU draw (`draw_death_star_beam`'s camera) and the
  renderer-backed `draw_tactical_death_star_beam`. The browser journey's log
  and green-pixel checks cover them.
