# A hull's detail level, measured as the original measures it (P24)

Date: 2026-10-25. This closes the open item of
`../2026-10-25-hull-rendering-a0/`: the Victory Destroyer still drew its
untextured far model where A0 TAC-01-C003 shows a textured hull.

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`).

`FUN_005d3770` (`0x005d3773`..`0x005d37e9`) computes its depth from two
vectors:

- **The hull's position in camera coordinates.** The hull frame's
  `GetPosition` (slot `+0x60`) is read against the active camera's `+0x8`,
  the camera frame. `0x005d9eb0` creates that frame under the scene `+0xc`
  (`CreateFrame`, slot `+0x10`) and makes it the viewport's camera.
- **The camera's direction in world coordinates.** The camera's `+0x14`
  vector is filled by the camera frame's `GetOrientation` against the scene
  (slot `+0x7c`, with `+0x20` as up; `0x005d9778`..`0x005d978a` and
  `0x005d9d39`..`0x005d9d4b`).

The dot product of these two is made positive and compared with 15 and 40
divided by the field (`../2026-10-25-hull-rendering-a0/`). The original
mixes the two spaces: the value equals the camera-space depth only while
the camera looks straight down the world's z axis.

## Change

`original_participant_lod_view` now:

- takes the hull's offset back to source coordinates;
- expresses it on the camera's left-handed axes (right = up × direction,
  up, direction);
- dots that with the camera's world direction.

In A0 TAC-01-C003's battle this gives about 160 for the Victory and about
102 for the transport. Both take their medium, textured models (2551 and
2071). Before, the camera-space depth was about 213 for the Victory, which
gave its untextured far model 2552. The two hulls now match A0's look,
apart from the 8-bit palette.

## Still open

- The original draws through the 8-bit palette: each hull in A0 has only
  10 or 11 colours, against 50 to 190 in the port.
- The Victory's exact heading on screen.

## Tests

`a_hull_s_detail_follows_its_depth_along_the_view_and_the_field`: its
expectation changed on this evidence. Turned 45°, the target lies
200^0.5 ahead in camera coordinates, but its dot with the world direction
is 10.

The browser case `production-hull-scale-a0` now draws 2071 and 2551.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2788 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | 5 tested: 3 caught, 1 unviable, 1 missed. The miss (`target - position` to `+`) gave the same values in the test's on-axis cases; an off-axis case (camera (0, 0, -10) looking at (10, 5, 0)) was added and catches it (checked by hand) |
| Release WASM SHA-256 | `a7a4824f33116a600e8fddc53078cac12673aa3d43a747c38b9ab7db3d926b04` |
