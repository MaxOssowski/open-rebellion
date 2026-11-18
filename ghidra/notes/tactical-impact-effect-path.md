# Tactical impact-effect path

The saved `REBEXE.EXE` Ghidra project establishes the target-attached tactical
effect dispatcher and its frame lifecycle.

## Event and resource mapping

`FUN_005a7500` registers the ship-hit messages in this order:

| Message | Dispatcher code |
|---|---:|
| `SHIP_TAKE_LASER_HIT` | 1 |
| `SHIP_TAKE_ION_HIT` | 2 |
| `SHIP_TAKE_TURBO_HIT` | 3 |
| `SHIP_TAKE_TORPEDO_HIT` | 4 |

`FUN_005d3e90` combines that code with the post-hit stage:

| Stage | Weapon branch | State bit | Type-303 base | Frames | Texture | Decal origin | Decal size |
|---|---|---:|---:|---:|---|---|---:|
| Hit | ion | `0x04` | 3180 | 6 | 128 x 64 | (32, 32) | 2.5 |
| Hit | other | `0x08` | 3060 | 6 | 128 x 64 | (32, 32) | 2.5 |
| Damage | ion | `0x10` | 3240 | 16 | 128 x 64 | (64, 32) | 5.0 |
| Damage | turbolaser | `0x40` | 3300 | 7 | 128 x 128 | (64, 64) | 5.0 |
| Damage | other | `0x20` | 3120 | 7 | 64 x 64 | (32, 32) | 5.0 |
| Destroyed | all | `0x80` | 3360 | 16 | 128 x 128 | (64, 64) | 7.5 |

The dispatcher rejects an incoming state whose bit value is not greater than
the active state. States in `0x04`/`0x08` are mutually exclusive, as are states
in `0x10`/`0x20`/`0x40`. Destruction can replace a lower state.

## Timing and rendering boundary

`FUN_005d39a0` preloads the exact bases and frame counts and sets each
texture's decal; see "Size on screen" below.
`FUN_005d41a0` keeps a counter at `+0x140` and adds one to it on every call
(`0x5d437c`), whether or not the image changes. It swaps the image to
`base + counter` on the first call and then only on a call that finds the
`+0xf0` timer past `0.1` s (`0x5d422d`, the float at `0x0066d154`), and
resets the timer there. The call that finds the counter at the stated count
removes the effect (`0x5d4301`). With the destroyed bit (`0x80`) set, the call
at counter 11 turns off both field loops (vtable `0x24` with 1 and 2) and calls
vtable 4, `FUN_005d3180`, whose vtable `0x30` (`FUN_005d3420`) deletes the
hull's mesh visual; the fireball runs on to 16.

The counter therefore counts scene renders, not tenths of a second.
`FUN_005c25b0` (scene vtable `0x24`) calls `FUN_005c2c80`, which calls
`FUN_005d41a0` for every effect. `FUN_0059a530` (the tactical idle step from
`FUN_0040a050`, called on every pass of the `PeekMessageA` loop in
`FUN_00413560`) busy-waits in `FUN_005c1490`/`FUN_005c1cf0` until the scene
timer at `+0x660`, reset at the start of each render, passes `0.02` s (the
float at `0x0066c954`). At that cap a call comes just over 20 ms after the
last, the fifth call after a swap is the first past 0.1 s, and the destroyed
sequence shows 3360, 3365, 3370 and 3375 over 16 renders (about 0.32 s). A
machine slower than 25 ms a render swaps after fewer calls and shows more of
the frames; the effect still lasts 16 renders. The renders, and so the
effects, also run while the battle is paused: `FUN_0059a530` calls
`FUN_005c1470` whether or not it stepped the battle.

The flight path is a different retained-mode object. Vtable slot 8 at
`FUN_005d3de0` constructs it through `FUN_005ee590`; it is not one of these
type-303 frames. Open Rebellion therefore retains its bounded procedural beam
until that geometry path is recovered. The two looped families at 3520 and
3620 are also left open because their shield, tractor, or gravity event
semantics are not yet authoritative.

## Size on screen (read 2026-11-18)

The table's last two columns were once read as a "draw size" in pixels.
They are the decal's anchor texel and its size in model units:

- `FUN_005d39a0` (`0x5d39a0..0x5d3ab6`) calls `FUN_005d38d0(base, count,
  out, size, origin_x, origin_y)` for each family. `size` is
  `DAT_0066d164` (5.0), times `DAT_0066d168` (0.5) for 3060 and 3180 and
  times `DAT_0066d16c` (1.5) for 3360. 3520 and 3620 (the fields) take 5.0
  and (64, 64).
- `FUN_005d38d0` calls, on every frame's texture, slot `0x3c` (SetColors
  256), `0x40` (SetShades 1), `0x50` (SetDecalTransparency 1), `0x44`
  (SetDecalSize(size, size), `0x5d396c`) and `0x48` (SetDecalOrigin(x,
  y), `0x5d397e`). It never calls `0x4c` (SetDecalScale), so the default
  holds and the decal scales with depth: it is `size` units square at the
  effect frame's depth, its texture stretched over that square, the
  origin texel on the frame. The port's old table had 3300's origin as
  (32, 32); `0x5d3a9a` pushes (64, 64).
- The effect frame `+0x128` is created in `FUN_005d26c0` as a child of the
  hull's frame `+0x2c` (IDirect3DRM `CreateFrame`, slot `0x10`). No code
  scales `+0x2c`, so the units are the scene's (the hull meshes are
  scaled 3.0 at load, `0x005d5b10`).
- Before each swap `FUN_005d41a0` calls `FUN_005d2f50(+0x128, 1.0)`: the
  camera's position in the hull frame (`GetPosition`, slot `0x60`),
  normalised to `DAT_0066d17c` (1.0) unless it is zero, times 1.0, is the
  effect frame's position (`SetPosition`, slot `0xf4`). The decal sits one
  unit toward the camera from the hull's centre.

A0 check (`.artifacts/a0-vm/2026-11-18-edited-saves-8`, run 1, local
only): the camera is the Empire's default one (yaw 150, pitch 30, field
0.2; the holocube lands on every edge pixel of l-04). The superlaser's
target's 3365 (seq-l1-beam k1218) and 3370 (k1245) frames fit a decal
30.3-32 px wide centred at (368.6, 134.7)
(`tools/interface-parity/fireball-frame-a0.py --window=340,115,400,160
--shape`). Run 2's camera (l-10) fits yaw 180, pitch 29.8, field 1.405
on the eight cube corners (0.23 px² in all; the planet and the Death
Star land on their sides); both runs' fire triangulates to (0, 0,
-0.498e) with the rays 0.0005e apart, the Alliance capital lane's first
slot (`FUN_005ab650`). The extent e is the one free scale: the
medium-LOD Imperial Dreadnaught (2641) at (0, 0, 0.5e) lights exactly
A0 t-04's 84 hull pixels at e = 127 (IoU 1.0; 0.93 at 124, 0.96 at 130).
hyp: e = 127 (nine units on the larger side) is fitted, not read from
the save. At e = 127 the depth is 263.4, so the decal is 7.5 x 219 /
0.2 / 262.4 = 31.3 px against A0's 30.3-32. Run 2's decal comes to
4.3 px (A0: a 4 x 2 px 3370). The port drew every 3360 frame 64 px wide
(63.4 in the 440-px viewport) whatever the distance.

Port (applied 2026-11-18): `OriginalTacticalEffectSequence::decal_size`
and `decal_origin` (`rebellion-core/src/tactical.rs`) carry the two
columns; `TacticalAssetRenderer::impact_decal_rect` (`tactical_assets.rs`)
moves the frame one unit toward the camera
(`ORIGINAL_IMPACT_DECAL_CAMERA_OFFSET`), takes `size` units across at that
depth, and hangs the texture from its origin texel. port: the original
sets the offset only at each swap, in the hull's frame; the port takes it
every frame from the current camera (the hull turns little in 0.1 s).
Still open: the fields (3520, 3620) are drawn at a fixed 128 px, not as
their 5.0-unit decals.

This note records conclusions only. The saved Ghidra database, generated
decompilation output, and proprietary resource bytes remain untracked.
