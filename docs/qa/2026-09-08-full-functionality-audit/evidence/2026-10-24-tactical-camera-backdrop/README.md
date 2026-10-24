# The tactical camera, the planet and the Death Star backdrop (P24)

Date: 2026-10-24.

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`) in Ghidra 12.1.4,
checked against the A0 captures of the original in the Windows 10 QEMU/KVM
guest (`.artifacts/interface-parity/a0/`, ignored; only cell IDs are cited).

## Findings

### The camera orbits the vertical axis

`FUN_005d9640` builds the camera frame in three calls:

| Address | Call | Arguments |
|---|---|---|
| `0x005d9686` | `AddRotation` | `REPLACE`, axis `(0, 1, 0)`, yaw `+0x3c` |
| `0x005d96bd` | `AddRotation` | `BEFORE`, axis `(1, 0, 0)`, pitch `+0x38` |
| `0x005d96eb` | `AddTranslation` | `BEFORE`, `(0, 0, -distance)` |

The yaw turns about +Y, and the pitch raises the camera above the horizontal.
In source space the camera sits at
`(-d cos p sin y, d sin p, -d cos p cos y)` with local up
`(sin p sin y, cos p, sin p cos y)`.

The port read the yaw axis as Z, which placed the camera `pitch` degrees from
the Z axis. From 30 degrees off a face, the holocube looks the same under both
models, so no earlier holocube comparison exposed the error. Ships, fighters,
navigation points and the backdrops all appeared in the wrong places.

Checks against the captures, at battle extent 100 in a 440 x 440 view at
(16, 28):

| Cell | Camera | Original | Old port | New port |
|---|---|---|---|---|
| TAC-01-C002 | Empire, first frame | holocube corner `(+x, -y, -z)` at (157, 290) | (-403, 334) | (157, 288) |
| TAC-01-C008 | Empire, 16 x Rotate Left | holocube corner `(+x, -y, +z)` at (105, 298); planet at top right | off screen | (105, 297); planet centre (462, -63) |
| TAC-03-C020 | Empire, 120 x left, 8 x zoom out | Death Star disc about 74 px wide near (267, 90) | behind the camera | (222, 82), 75 px |
| TAC-04-C008 | Alliance, Tilt Up held | corner moves up to (157, 205) | | pitch about 42 degrees |

Rotate Left lowers the yaw and Tilt Up raises the pitch, as the port already
did.

### Planet and Death Star decals

`FUN_005d4d10(scene, e, _, planet_resource, death_star_resource)` builds the
backdrop. `e` is the battle extent.

- **Planet.** The planet is built only while Show Planet is on (`+0x680`).
  - Its frame stands at `(2e, rand(0.2e) - 0.1e, rand(0.2e) - 0.1e)`. The
    `SetPosition` x argument is the slot written at `0x005d540d`, and
    `DAT_0066d23c` is 0.1.
  - The decal is the system's planet texture. Its origin is the texture
    centre `(0x80, 0x80)`, `SetDecalSize(e, e)`, and decal transparency is on.
  - The port had drawn it as a flat bitmap on the left edge of the view
    (marked provisional). It now draws a decal `e` wide at `(2e, 0, 0)`, with
    black transparent: retained mode's default transparent colour.
  - The planet is outside the first view of both sides, as in TAC-01-C002
    and C003. TAC-01-C008 shows it after the camera turns.
- **Death Star.** The Death Star's frame stands at
  `(-2e, rand(0.4e) - 0.2e, rand(0.4e) - 0.2e)` (`DAT_0066d248` -2,
  `DAT_0066d244` 0.2). Its child sits at `(0, 0.11e, -0.09e)` and carries a
  `0.5e` decal.
  - The earlier port (531894cd) had put the -2e on z. The `SetPosition`
    slots show it is x.

`port:` the random offsets come from a local stream and are taken at their
centre.

## Tests

- `the_camera_orbits_the_vertical_axis_as_the_original_captures_show`
  projects the corners and backdrops for the four cells. It fails on the old
  pose: (-403, 334) instead of (157, 290).
- `original_camera_preserves_faction_pose_clip_and_field_contract` now
  expects the two sides' cameras, 180 degrees apart, at the same height
  (`d sin 30`) and at mirrored x and z. The old expectations were derived
  from the Z-axis reading.
- `the_death_star_hangs_behind_the_battle_opposite_the_planet` and
  `the_planet_hangs_two_extents_out_on_x_as_wide_as_the_battle`.
- The browser harness expects the planet load with black transparency, and
  its scene log with `visible=false` in the first frame.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2756 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| Release WASM SHA-256 | `afac8ec687d62504864676180bd9caece5f848f9d88d04fc679237590b83b4d2` |

Some fixture and harness choices had assumed the old view, and now follow
the new camera:

- The Death Star laser target is moved to `x = -20`. The group fixture
  spreads the player's ships along their lane.
- The deselection click goes to the right of the Alliance ship, which now
  stands at the view's left edge.
- The escort target is the friendly hull nearest the view centre.
- The fighter level-of-detail order follows which group is nearer the
  camera.

The selection frame's lines are now snapped to whole pixels. A frame at a
fractional position blended into grey and failed the manual's white-box
check.

`cargo mutants` missed 51 mutants:

- Two pure decoders were missed. A new test now catches all 14 of their
  mutants.
- The rest are in GPU drawing (`draw_planet`, `draw_decal_texture`,
  `draw_projected_frame`), the texture load, and the browser fixtures. The
  browser matrix's logs and screenshots cover them.

Open: the port's hulls look smaller than in TAC-01-C002.
