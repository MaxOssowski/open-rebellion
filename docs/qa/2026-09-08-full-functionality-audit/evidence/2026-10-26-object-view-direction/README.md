# Each hull's specular uses its own view direction (P24)

Date: 2026-10-26. This settles the viewer model left as `hyp:` in
`../2026-10-26-d3dim-hull-lighting/`, and with it the darker transport.

## The original

Static reading of d3dim.dll (the A0 guest's) and the game's d3drm.dll:

- d3dim's `FUN_74017fc4` flags a light by the size of the struct that
  `IDirect3DLight::SetLight` receives:
  - 0x4c, the DX3 `D3DLIGHT`, gives flag 1;
  - 0x50, `D3DLIGHT2`, gives flag 2.
- `FUN_7402029b` picks the light function from the table at `0x740473f4`
  (installed at `0x74009b6e`). For a directional light:
  - flag 1 takes `FUN_74020ff0`, the non-local viewer;
  - flag 2 takes `FUN_74020ae0`, the local one.
- With flag 1, `FUN_7402029b` takes the eye into the model's frame. It
  inverts the view through `+0x1068` and `+0x1038` (`FUN_7401feff`), then
  applies the world-to-model matrix `+0x1084` (`FUN_7401ff51`). It stores
  `h = normalize(normalize(eye) + l)`. The view direction therefore runs
  from each object's origin to the eye, recomputed per world matrix.
- d3drm's `FUN_6d8410d6` writes the 0x4c struct unless the device's render
  mode (`+0x90`) has `D3DRMRENDERMODE_VIEWDEPENDENTSPECULAR` (0x10).
  - The device constructor (`0x6d81a08b`) sets `+0x90` to 0.
  - REBEXE creates its device through `IDirect3DRM2`
    (`FUN_005d6930`, `FUN_005d6e10`). It calls `SetDither` and `Update` on
    it, and no `SetRenderMode` was found.
- In flat mode d3drm gives each face's first vertex the face normal
  (`FUN_6d827ec2`). With the non-local viewer the vertex position plays no
  part.

## The port

- `set_view_direction(material, eye, origin)` sets
  `normalize(eye − origin)` for each drawn object, next to its
  `ObjectTranslation` (`original_view_direction`).
- The single-hull proof draws its hull at the origin.

## Against A0

Scenario 37 (`production-hull-scale-a0`), 640 × 480, run
`2026-10-26T05-10-47-739Z-907708`, hull pixels only:

| | Victory mean RGB | Victory white pixels | Transport mean RGB | Transport white pixels |
|---|---|---|---|---|
| A0 TAC-01-C003 | (180.5, 188.4, 195.7) | 13 | (188.3, 174.3, 170.7) | 30 |
| One scene direction (before) | (181.7, 188.1, 196.6) | 18 | (177.3, 160.2, 155.9) | 14 |
| Per object (now) | (178.2, 185.5, 193.6) | 11 | (187.2, 173.6, 170.2) | 29 |

- Both hulls now sit within 3 per channel of A0's means.
- Their white specular counts are within 2 of A0's.
- A per-pixel simulation of the same frame had predicted transport
  (187.6, 173.8, 170.3) with about 29 whites. Its first fit had the
  transport's heading and the light's z wrong; the predictions here come
  from the corrected fit.

Per the user's call (2026-10-26), the hull lighting topic closes here: the
render is well past the 80% likeness asked for.

Remaining small unknowns:

- d3dim's 256-entry pow table, where the port uses an exact `pow`;
- the Victory's position in A0, which is 1 to 2 px off the port's.

## Tests

- `the_view_direction_runs_from_each_object_to_the_eye`.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2844 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | 8 tested: 3 caught, 5 missed |
| Release WASM SHA-256 | `bd0912036b348bcefe66849b6623cd73789978b740e4552d5de7b031fd2e38c3` |

The 5 missed mutants are GPU draw paths:

- `set_view_direction` as a no-op;
- `draw_proof` and its camera arithmetic;
- `draw_participants`.

No `cargo` test draws them. The image comparison above checks their
effect.
