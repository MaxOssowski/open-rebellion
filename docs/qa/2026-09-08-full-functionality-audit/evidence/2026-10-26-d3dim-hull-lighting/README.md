# Hulls are lit as d3dim lights them, specular included (P24)

Date: 2026-10-26. This settles the "exact lighting equation" left open by
`../2026-10-25-hull-rendering-a0/`, `../2026-10-26-flat-hull-shading/` and
`../2026-10-26-browser-no-multisampling/`. The port's hulls were a little
brighter than A0's, and its transport lacked A0's white band.

Sources were read statically: the game's `d3drm.dll` (SHA-256
`e04e3781…d4d6`) and the A0 guest's `d3dim.dll` (see
`../2026-10-26-rgb-cube-output/`). Neither is in the repository.

## The original

d3drm.dll:

- d3drm does not light. It asks d3dim to, with
  `D3DPROCESSVERTICES_TRANSFORMLIGHT` (`0x6d83a311`). It sets the material
  per group (`D3DOP_STATELIGHT MATERIAL`, `0x6d83a2e5`).
- The material (`FUN_6d81c589`):
  - diffuse = the face colour;
  - ambient = the same colour;
  - specular, power and emissive come from the RM material. The hull
    materials carry specular (1, 1, 1), power 15 and emissive 0.
- `FUN_6d837223` turns `SPECULARENABLE` (render state `0x1d`) on for every
  group with a material.
- The ambient lights' sum is packed as `min(255, trunc(sum × 256))`
  (`FUN_6d83df1e`, the double 256.0 at `0x6d802828`), so 0.5 becomes 128.
  The directional light's 0.8 stays a float.
- d3drm sets no texture blend. d3dim's device starts with
  `D3DTBLEND_MODULATE` (`FUN_7400399f`).

d3dim.dll:

- `FUN_7402029b` sets up each light on a 0–255 scale:
  - the diffuse factor is diffuse × 255 × 0.8 = 204;
  - the specular factor is specular × 255 × 0.8;
  - the base is 128 × ambient + 255 × emissive.
- A directional light (`FUN_74020ff0`) adds, when n·l > 0, the diffuse
  factor × n·l and the specular factor × (n·h)^power (`FUN_74020945`, a
  256-entry pow table).
- The vertex colour is rounded (`fistp`), then clamped to 0..255
  (`0x74025ff6`..`0x74026065`). The light is clamped before the texture.
- The rasteriser's modulate (`FUN_7403a950`, `FUN_7403a9d0`) and specular
  add (`FUN_7403c550`) give, per channel,
  `min(255, ((t × d) >> 8) + s)`.
- The result goes to the colour cube
  (`../2026-10-26-rgb-cube-output/`).

## The port

The hull shaders (GLSL and Metal) now follow that path:

- The vertex stage computes, on the 0–255 scale:
  - `d = clamp(round(128 × tint + 204 × tint × n·l + 255 × emissive))`;
  - `s = clamp(round(204 × specular × (n·h)^power))`, only when `n·l > 0`.
- The fragment stage computes `min(255, floor(t × d / 256) + s)` per
  channel, then the cube.
- `decode_mesh_object` keeps each material's specular colour and power. A
  chunk carries them, and they reach the shader as `MaterialSpecular`.
- `ORIGINAL_AMBIENT_LIGHT_BYTE` = 128.
- `ORIGINAL_SPECULAR_ENABLED` is now true.
- The family log reads `specular=true material=ambient_diffuse_specular_emissive`,
  and its probe checks that.

`hyp:` the half vector uses one view direction for the scene, from the
camera's target to its eye. This is d3dim's non-local viewer
(`FUN_74020ff0`); the local one is `FUN_74020ae0`, and the light's flag
that chooses between them was not settled. With one direction, a face's
specular is as flat as its diffuse light.

## Against A0

Scenario 37 (`production-hull-scale-a0`), 640 × 480, run
`2026-10-26T04-41-47-025Z-819063`. Hull pixels only: the field lines and
the selection corners are left out.

| | Victory mean RGB | Victory white pixels | Transport mean RGB | Transport white pixels |
|---|---|---|---|---|
| A0 TAC-01-C003 | (180.5, 188.4, 195.7) | 13 | (188.3, 174.3, 170.7) | 30 |
| Port before | (192.5, 197.5, 207.7) | 32 | (186.4, 173.6, 170.5) | 3 |
| Port now | (181.7, 188.1, 196.6) | 18 | (177.3, 160.2, 155.9) | 14 |

- The Victory's mean now matches A0's within 1.2 per channel.
- Enlarged side by side, the transport now shows A0's white band along its
  lower edge, thinner than in A0.
- Its mean is darker than A0's by about 11 to 15 per channel, over a
  similar area (239 hull pixels against 232). The cause is not settled.
  One candidate is the viewer model above: with a local viewer each face's
  half vector would differ, which moves where the band falls.
- The Victory keeps 11 colours, all from the cube.

## Tests

- `source_light_rig_preserves_retained_mode_direction_and_intensity`. Its
  expectations changed on this evidence; the full light was 1.3,
  unclamped. It now checks `original_face_light`, the shader's equation in
  Rust:
  - 128 facing away, with no specular;
  - 255 facing the light, specular 204 along the half vector;
  - a half-lit grey material and emissive;
  - power 15 away from the half vector;
  - `original_hull_channel`'s `(t × d) >> 8 + s`, clamped at 255.
- `source_device_state_uses_nearest_gouraud_backface_depth_contract` now
  expects specular on (`FUN_6d837223`).

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2843 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | 12 tested: 10 missed, 2 unviable |
| Release WASM SHA-256 | `5812a27dcc57bcfa78878ba3c470974da6635165daf6cc135b7c67bfc5650eea` |

All 10 survivors are GPU paths that no `cargo` test can draw:

- `set_view_direction` and its subtraction;
- `draw_proof` and its camera direction;
- `draw_participants`;
- `load_proof`;
- `load_participant_family`;
- the material's uniform list.

The browser matrix exercises them: a missing uniform list or family fails
its scenarios. The view direction shows only in the image comparison
above.
