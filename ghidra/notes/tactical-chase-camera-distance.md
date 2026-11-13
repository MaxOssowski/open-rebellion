# TAC-04-C014: why the port's chase camera looked 1.25x too far (2026-11-13)

Static trace plus a geometric fit of the existing A0 capture. No code changed.

## Result

The chase camera (`Class_0066d478_vtc`, `0x005d9b10`) is ported correctly.
The gap comes from the battle, not the camera: A0's Battle at Coruscant has
a battle extent of 136, the port's fixture 43 has 109. 136 / 109 = 1.248.

Every world position in a battle scales with the extent. That includes the
capital lanes (+/-0.5 extent), the nav grids (k x 0.125 extent,
`FUN_0059f800`) and the free camera's distance (1.7 extent, `FUN_005c1d30` ->
`FUN_005d9490`, `DAT_0066c958` = 1.7f). Mesh sizes do not scale with it: the
hull box is 3 x the X-file box, and the nav tetrahedron is 1.5
(`FUN_005c1c10` -> `FUN_005d8c30(0x3fc00000)`). The chase offset
(-e, e, 2e) comes from the mesh box, so in A0's battle it is 0.80 of what the
port's battle gives, relative to the world.

The free camera's image is unchanged by the extent, because positions and
distance scale together. Only sizes show it, which explains two things:

- the second stage's "port pyramids about 1.2x linear / 1.45x area" in the
  free view (TAC-04-C001);
- the Star Destroyer's width there: A0 14 px against the port's 17 px.

## The fit (A0 `interface-parity/a0/TAC-04-C014/original`, sha256 8f2c212e…)

Model: the port's chase eye (carrier at (0, 0, -0.5 ext), facing +z, eye
(e, e, -0.5 ext - 2e), e = 3.855), K = 1095 px per unit tangent (tan 0.2 over
219 rows), centre (237, 249), tetra 1.5. With extent 109 the model reproduces
the port's own frame (run 2026-11-12T13-37-10-859Z) to about 2-3 px and to
within a few percent in area.

Nine A0 pyramids (4-connected, 20 px or more, merged chains left out), with
the nearest model pyramid:

| Hypothesis | Mean centroid error | A0 area / model area |
| --- | --- | --- |
| port as is (ext 109, e 3.855) | 21.5 px | - |
| eye only x0.79 (ext 109) | 1.9 px | 0.60-0.67 (model ~1.55x too big) |
| ext 133 (11 objects) | 3.65 px | 0.96 |
| **ext 136 (12 objects)** | **2.05 px** | **0.995 (0.94-1.05 each)** |
| ext 139 (13 objects) | 2.18 px | 1.03 |

The positions alone cannot tell "eye x0.8" from "world x1.25". Projection
is scale-invariant about the eye, so the second stage's fit of 0.80 was
right about positions. The areas, though, fit only the world-scale
hypothesis. The far Star Destroyer agrees:

- In the chase it is 23 px wide in A0 and 29 px in the port. It is 116
  units away, so an eye change cannot shrink it.
- Its position (-28, +27) from the centre matches e / (ext + 2e) at ext 136.

The X-wing and Y-wing at A0's (85, 400) and (282, 400) fit either extent,
because they sit 20 units in front of the carrier in both battles.

## Where 12 comes from

The extent is 100 + 3 x (the larger side's active count). The count is
`FUN_005ab650` `0x005abb11`..`0x005abb4b`: `fsubs DAT_0066c2a0` per unit
past the other side, for objects where `FUN_005b69f0` returns 0, sides by
`+0x168`.

The Alliance side is 3 (carrier, X-wing, Y-wing), so the Empire side counted
12. The A0 manifest notes for the same save (TAC-02-C004, TAC-05-C009) list
the Judicator (an ISD, Fleet 3) carrying 5 TIE squadrons, plus 11 Coruscant
planet TIE squadrons. That is 16 squadrons. The port's stand-in defender is
one ISD with 6 inactive TIE squadrons, so it counts 1.

hyp: the 12 are the ISD plus the 11 planet squadrons, and the 5 carried
squadrons do not count. This is not traced. The port's own rule ("every
living ship and fighter group with craft counts", `tactical.rs`
`ORIGINAL_BATTLE_OBJECT_INCREMENT`) is marked `port:`.

## Statically ruled out (all match the port)

- d3drm.dll (game copy, sha256 e04e3781…d4d6, base 0x6d800000):
  - SetFront `0x6d81604b` -> `+0x10c`; SetBack `0x6d8160b0` -> `+0xdc`.
  - SetField `0x6d816115` writes the plane (-f, f, -f, f) at
    `+0x110..+0x11c`.
  - `FUN_6d83dba5` rebuilds the projection on every Render (`0x6d815c67`)
    with scale = front / field per axis (`+0x1ac`, `+0x1b0`), so the
    tangent is field / front. Uniform scaling only applies the 440/438
    aspect.
  - Chase 0.02 / 0.1 is the same angle as free 0.2 / 1.0.
- d3drm MeshBuilder Scale (`FUN_6d826870`) sets dirty bit 1. GetBox
  (`FUN_6d826a18` -> `FUN_6d824e8f` -> `FUN_6d824cad`) recomputes the box
  from the scaled vertices. So e is the scaled box: 3.85496 (0x4076b7a5) for
  2051. All X-file vertices are face-referenced, so the box is not inflated.
- REBEXE: `0x005d5b10` does Load then Scale(3.0, `DAT_0066d2b0` =
  0x40400000). The hull's vt+0x2c (`0x005d43b0`) is `lea 0xcc(%ecx)`.
  GetBox is at `0x005d2ae2` on `+0x48` (base+1).
  - e = max(X span, Y span), from the disassembly at `0x5d9b80`..`0x5d9ba4`.
  - SetPosition(-e, e, 2e) at `0x5d9c81`..`0x5d9ca6`.
  - SetOrientation(0, 0, -1, 0, 1, 0) at `0x5d9ce5`.
- d3drm SetPosition (`FUN_6d81340e`) uses the reference frame's world
  transform. The hull frame (`0x005d3500`) has unit rows. The camera frame
  and the hulls share parent `+0x138`.
- No 0.8 or 1.25 constant is used in the tactical code
  (`0x580000`..`0x5e0000`). The only immediate 0x3f4ccccd is the light.

## To close the cell

- Either give fixture 43 a defender roster whose active count is 12, which
  needs the day-2 save's Imperial roster at Coruscant,
- or trace which units `FUN_005b69f0` counts and rebuild that roster.

Then the chase test can tighten from 40 px to a few pixels without touching
the camera.

Optional runtime confirmation in the A0 VM, same save and journey:

- At `0x5ac5bc` (the call to `FUN_005c10a0`), `eax` = `[esi+0x8cc]` should
  be 0x43080000 (136.0). The fixture gives 0x42da0000 (109.0).
- At `0x5d9ba4` (`fstps 0x34(%esi)`), ST0 should be 0x4076b7a5 (3.855),
  the same as the port.
- The two side counters `[esp+0x4c]` and `[esp+0x50]` after the
  `0x5abb11` loop should be 3 and 12.
