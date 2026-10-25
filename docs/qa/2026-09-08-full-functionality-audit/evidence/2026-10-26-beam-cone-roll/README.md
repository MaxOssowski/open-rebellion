# The superlaser cone rolls as LookAt does (P24)

Date: 2026-10-26. This closes the open "cone's roll about its axis" of
`../2026-10-25-death-star-beam/`.

## The original

REBEXE.EXE:

- At the end of `FUN_005da150` (`0x005da6d8`..`0x005da6f5`), the beam frame
  `+0x2c` calls `LookAt` (IDirect3DRMFrame slot `+0x90`) with:
  - the target's frame (`+0x60`);
  - the Death Star decal frame as reference;
  - constraint 0, `D3DRMCONSTRAIN_Z` (`ebx`, cleared at `0x005da176`).
- Every frame, `0x005da830` repeats the call (`0x005da8ca`..`0x005da8e2`).
- `FUN_005d4d10` builds the decal frame's parents with `SetPosition` only,
  never `SetOrientation`. So the reference frame's axes are the scene's.

`d3drm.dll` shipped with the game (Microsoft's, SHA-256 `e04e3781…d4d6`,
image base `0x6d800000`):

- Frame vtable `0x6d84f368`, slot 36 → `FUN_6d81fa3a`. It takes the unit
  direction `d` from this frame to the target in the reference frame, and
  sends constraint 0 to `FUN_6d81f86b`.
- `FUN_6d81f86b` builds the up vector from `s = sqrt(1 - d.y²)`:
  - when `s ≥ 1e-4`: `(-d.y·d.x/s, s, -d.y·d.z/s)`, that is, the
    reference +Y made perpendicular to `d`;
  - otherwise: `(-d.y, s, 0)`.
- It then calls SetOrientation(reference, d, up) (`FUN_6d8130b8`).

## The port

`death_star_beam_cone_mesh` (tactical_assets.rs) took world +Y as its
reference, but switched to +X once `|axis.y| ≥ 0.99`. Up to that bound its
ring was the original's: the same up, the same eight vertices. For beams
between that bound and `s < 1e-4` it rolled the ring differently.

- `death_star_beam_up` now gives `FUN_6d81f86b`'s up vector, fallback
  included.
- The ring's side is `up × axis`. That is the vector the port used before,
  so the vertex order and the faces' winding do not change.

## Tests

- `the_superlaser_cone_rolls_as_look_at_with_the_z_constraint`:
  - for the axis (0.1, 0.995, 0), normalized, ring vertex 2 lies along the
    perpendicular +Y;
  - for the axis (0.05, 0.99, 0.08), normalized, it lies along the full
    perpendicular +Y, z component included;
  - for an exactly vertical axis it lies along (-1, 0, 0);
  - for a level axis it lies along +Y.
- With the old reference choice the test fails on the first case.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2822 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | 18 tested: 14 caught at first, 3 missed, 1 unviable; after the off-axis case 17 caught, 1 unviable |
| Release WASM SHA-256 | `ec664e8ad606e092c652adb0a630802bac06e90cfca3ec368719b2fd609fe07c` |

The three first survivors were in the up vector's z component, which the
first cases left at zero. The off-axis case was added after the WASM build
and changes only the test.
