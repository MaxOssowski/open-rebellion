# Hull rendering against A0 TAC-01-C003 (P24)

Date: 2026-10-25. This closes the open "hull lighting looks darker" and
"like-for-like hull pixel size" items of
`../2026-10-25-tactical-hull-frame/`.

## A like-for-like battle

A0 TAC-01-C003 (ignored store; only the cell ID is cited) is the first
paused frame of a battle at Mon Calamari:

- one Medium Transport (Alliance) against one Victory Destroyer (Empire);
- no fighters, so the extent is 103 (`FUN_005ab650`;
  `../2026-10-25-battle-extent-source/`).

A new test-only fixture, scenario 37 (`production-hull-scale-a0`), builds
the same battle. Its probe measures each hull's lit pixels around its
projected point.

- **Placement.** Both hulls stand where A0 shows them. The Victory
  projects to (370, 133) on the 640 × 480 canvas, in A0 about (371, 132).
- **Palette.** At first the fixture's system was the first empty one the
  galaxy loads (palette 5538), not Mon Calamari. Since
  `../2026-10-25-battle-palette-output/` it fights at Mon Calamari
  (palette 5537), as A0 does.
- **Look.** The port drew both hulls dark, flat and shapeless. On average
  the Victory was (76, 83, 90) and the transport (104, 92, 64); in A0 they
  are (181, 188, 196) and (187, 173, 170).

## Three causes

1. **Back faces were drawn.** With the directional light's term drawn
   alone, the diffuse term was 0 at every visible hull pixel. The visible
   normals pointed down (−y) while the camera looks from above, so the
   port was drawing each hull's far, inward faces.
   - Direct3D's `D3DCULL_CCW` keeps the clockwise facets. After the
     handedness reflection and the index reversal in `decode_mesh_object`,
     they reach GL counter-clockwise, but the pipeline kept `Clockwise`
     front faces.
   - With `CounterClockwise` the hulls show their lit outer faces, with
     A0's colours and contrast.
2. **The light was clamped before the texture.** The port took
   `min(1, 0.5 + 0.8 × cos)` and then multiplied by the texel, so no
   pixel could pass the brightest texel (228 here). A0's hulls reach 255.
   - The light (ambient 0.5 plus directional 0.8, `FUN_005d4d10`) now
     multiplies the texel unclamped, and only the final colour is clamped.
   - The hulls' means then come to (155, 167, 173) and (176, 159, 154),
     close to A0.
   - `hyp:` A0 is the evidence here; the retained-mode device's exact
     lighting path is not read.
   - Superseded in `../2026-10-26-d3dim-hull-lighting/`: d3dim clamps the
     light before the texture, as the port first did. A0's 255 peaks come
     from the specular, which the port then lacked.
3. **The detail level used a raw distance and a scale of 1.**
   `FUN_005d3770`:
   - takes the hull's offset from the camera along the camera's direction,
     made positive (`0x005d37b3`..`0x005d37e9`);
   - divides its thresholds 15 and 40 (`DAT_0066d158`, `DAT_0066d160`) by
     the active camera's slot `+0x14` (`0x005d98f0`). That is the camera's
     `+0x48`, the field of view: 0.2 at the battle's start (`0x005c1e42` →
     `0x005d9620`), times 0.9 or 1.1 for each zoom step (`0x005d9830`,
     `0x005d9860`; `DAT_0066d408`, `DAT_0066d410`), clamped to 0.005..1.5.

   The port compared the straight distance with 15 and 40, so every hull
   past 40 drew its untextured far model (base + 2; the manifest shows no
   texture for 2072 or 2552). Now the transport (depth about 136) takes its
   medium model, textured.

## Still open

- **The Victory's detail level.** At its depth (about 213 > 200) the port
  still draws its far model, which has no texture, where A0 shows a
  textured hull. Which frame `FUN_005d3770` measures from has not been
  confirmed: `GetPosition` against the camera controller's `+0x8` frame.
  With the camera's orbit pivot both hulls would take the close model.
- The exact Direct3D RM lighting equation and colour model.

## Tests

- `a_hull_s_detail_follows_its_depth_along_the_view_and_the_field` (new):
  depth along the view, made positive, and the thresholds 75 and 200 at
  field 0.2.
- `source_light_rig_preserves_retained_mode_direction_and_intensity`: its
  expectation changed on this evidence. The full light is 1.3, unclamped.
- `source_device_state_uses_nearest_gouraud_backface_depth_contract`: front
  faces are counter-clockwise.
- Browser:
  - the new `production-hull-scale-a0` scenario, 4 cases. It checks the
    layout (extent 103, lanes ±51.5), the two hulls (2070 at −51.5, 2550 at
    +51.5) and records each hull's lit-pixel box;
  - the production participants' families now depend on the battle;
  - their `production-tactical-resource-families` probe records the models
    actually drawn (from the scene log), instead of a fixed `[2012, 2512]`
    that the new thresholds no longer give;
  - the family log names `target_cull=back_ccw`.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2788 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (the 144 before plus the 4 new cases) |
| Replay equivalence | passed |
| `cargo mutants` | 10 tested: 6 caught, 1 unviable, 3 missed |
| Release WASM SHA-256 | `c290241f1fb8effd0a3d8204f543a2835d727f82854d702480e5793940e487ee` |

The three missed mutants:

- Deleting `front_face_order` from the pipeline is equivalent: miniquad's
  default is `CounterClockwise` (`miniquad-0.4.8`, `graphics.rs:747`).
- `draw_participants` and `load_proof` are the GPU draw and load paths; the
  browser matrix's scene, bounds and lit-pixel checks cover them.
