# Hulls are lit once per face (P24)

Date: 2026-10-26. This closes the flat-shading item left open in
`../2026-10-26-rgb-cube-output/`.

## The original

Read statically in the game's `d3drm.dll` (SHA-256 `e04e3781…d4d6`, image
base `0x6d800000`; not in the repository):

- The device constructor `0x6d81a08b` (class `0x6d8508a8`) sets these
  defaults:
  - dither `+0x60` = 1;
  - shades `+0x64` = 32;
  - quality `+0x68` = 0x88, `D3DRMRENDER_FLAT` (flat shading, lights on,
    solid fill). `GetQuality` is `0x6d811347`.
- REBEXE turns dither off (`FUN_005c1c10`: `IDirect3DRMDevice2 +0x4c`,
  `SetDither(FALSE)`). It makes no device `SetQuality` call.
- `FUN_6d827ec2` takes, field by field, the lower of the device's and the
  mesh's quality (shade, light, fill). So whatever the meshes ask for, a
  hull is shaded flat: one light value per face.

The port had logged `render_quality=gouraud` and interpolated light between
vertex normals.

## The port

- `flat_shaded_meshes` (tactical_assets.rs) runs on every decoded mesh
  chunk. Each triangle gets its own three vertices, all carrying its
  plane's normal. That normal is turned to the side its vertex normals
  face; a degenerate triangle keeps their sum.
- A chunk splits into several meshes before its vertices would pass the
  `u16` index range.
- The family log and its browser probe now read `render_quality=flat`.

## Against A0

Scenario 37 (`production-hull-scale-a0`, 640 × 480), with the boxes and
counting of `../2026-10-26-rgb-cube-output/`:

| | Victory colours (outside the cube) | Transport colours (outside the cube) |
|---|---|---|
| A0 TAC-01-C003 | 12 (0) | 22 (11) |
| Colour cube, smooth light | 53 (40) | 78 (69) |
| Colour cube, flat light | 45 (34) | 81 (70) |

The Victory, enlarged next to A0, now shows flat palette-coloured facets.
The port's hull reads a little brighter than A0's; d3drm's exact RGB
lighting equation is still not decoded. The colours outside the cube are
still the browser's multisampled edges and the antialiased field lines,
which are the next stage.

## Tests

- `hull_faces_are_lit_flat_with_their_plane_normal`:
  - two faces that share an edge each take their plane's normal, turned to
    their vertex normals' side, for either winding;
  - a degenerate face keeps the summed normal;
  - 90,000 vertices split into 65,535 and the rest.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2838 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T02-29-34-007Z-3259270`) |
| Replay equivalence | passed |
| `cargo mutants` | 25 tested: 6 missed at first; after the winding case and the `u16::MAX` limit, 20 on `flat_shaded_meshes`: 18 caught, 1 unviable, 1 missed |
| Release WASM SHA-256 | `7b1b29260b0a160f59d447ed3fb473fc22b770f7556665806caadfffa926a563` |

The matrix ran before the split limit changed from `u16::MAX - 2` to
`u16::MAX`. That limit only changes chunks of more than 65,533 vertices,
which no battle mesh reaches. The release WASM was rebuilt after the
change.

The missed mutant, `< 0` with `<= 0` in the normal's turn, differs only for
a face exactly perpendicular to its vertex normals. `load_proof` (missed in
the first run) is a GPU loading path that only the browser matrix
exercises.
