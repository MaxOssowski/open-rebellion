# Navigation markers, the order line and held panel buttons against A0

Date: 2026-11-07 (evening). Cells: TAC-04-C001..C003 (navigation points and
routes), TAC-02-C010 (task force held), TAC-05-C013 (maneuver held),
TAC-03-C016/C017 (the selected ship's order line).

## The markers

A0 TAC-04-C002 and TAC-04-C003 (Wine) draw each navigation point as a small
shaded tetrahedron: two shades per marker, (255, 0, 0) and (201, 3, 0) for
the middle layer, (255, 255, 255) and (206, 206, 198) for the top one,
(0, 138, 255) and (0, 92, 216) for the bottom one. The first point of the
ordered route is yellow ((255, 255, 0), (255, 213, 0)), and in C003 the second
point is magenta (255, 0, 255). Neither has an outline or a ring. The VM's
2026-10-27 captures show the same tetrahedra, smaller at that zoom, with the
selected point (128, 0, 128).

The port drew flat 2D triangles of a fixed 5-pixel radius with a white
outline. It enlarged route points and put a white ring around them, and it
did not colour them.

The original:

- `FUN_005d8c30`, called once from `FUN_005c1c10` with 1.5
  (`push 0x3fc00000`), builds one mesh: four vertices (0, -0.2041, 0.5774),
  (0.5, -0.2041, -0.2887), (-0.5, -0.2041, -0.2887), (0, 0.6124, 0). That is
  a regular tetrahedron of edge 1 with its apex up; its face normals are
  (±0.8165, 0.3333, 0.4714), (0, 0.3333, -0.9428) and (0, -1, 0). The mesh
  is scaled by the argument (`Scale`, meshbuilder slot `0x34`) and drawn
  solid, lit and flat (`SetQuality(0x88)`). Five copies take
  `SetColorRGB`:
  - `DAT_006bcbe0` (1, 0, 0)
  - `DAT_006bcbe4` (1, 1, 1)
  - `DAT_006bcbe8` (0, 0.541, 1)
  - `DAT_006bcbec` (1, 1, 0)
  - `DAT_006bcbf0` (1, 0, 1)
- `FUN_005d90b0` puts a marker at the point. It sets only the frame's
  position, not its orientation. Its vtable `0x66d3b0` shows a copy in
  three ways:
  - `+0x8` the layer copy;
  - `+0x10` the yellow copy;
  - `+0xc` the magenta copy.

  `+0x4` hides the marker, and `+0x14` is an empty stub.
- `FUN_0059f560` picks the state, and the first match wins:
  - `+0x34` hidden;
  - `+0x20` yellow;
  - `+0x3c` (the stub);
  - `+0x2c` magenta;
  - otherwise the layer colour.
- `FUN_005a0840` walks the selected unit's route (`+0x80`) and sets `+0x20`
  on its first point and `+0x2c` on every later one.

The port now:

- draws each shown point as that tetrahedron (`navigation_marker_faces`,
  edge 1.5). Each face is lit flat with the hulls' light: ambient 128 and
  directional 204 on the face colour (`original_flat_face_color`). Markers
  are depth-tested against the hulls (`draw_navigation_markers`).
- colours a point by `navigation_point_tint`:
  - yellow if it is the first point of a selected unit's route;
  - magenta if it is a later point, or the selected point (the VM's purple
    is the magenta mesh's unlit face, (128, 0, 128), which the unit test
    reproduces);
  - otherwise the layer's material colour.

  The layer-to-colour mapping (top white, middle red, bottom blue) stays
  the one the VM showed.
- drops the outline, the ring and the enlargement.

`run.mjs` 'navigation-route-markers' (Linux, both sides, 640x480 and
letterboxed) orders a two-point route. It finds yellow pixels at the first
point (23 at 640x480, Alliance), magenta pixels at the second (17), and no
magenta at the first.

Shading is not pixel-exact. The VM's palette mode gives (148, 6, 0) for red,
and Wine gives two shades per marker. The port's faces are the d3dim
formula's values. The marker's orientation follows the scene's axes after
the port's z reflection, which is a hypothesis from the frame's untouched
orientation.

## The order line

A0 shows "Navigating" once a route is ordered (TAC-04-C002, C003). It shows
"Attacking Victory Destroyer 1" (TAC-02, TAC-03-C016) and "Attacking Swift"
while a target is held, and "Attack Capital Ships" with no target
(TAC-03-C017). The port always printed the order's word.

`FUN_005e4b40` takes the order's word: TEXTTACT `0xdde1`..`0xddeb`, or
`0xdd8f` "No Orders" by default. Then it asks the unit's mover
(`FUN_005a6460`), in this order:

- If its route (`+0x80`) holds a point, the line reads `0xdd8e`
  "Navigating". If the unit's speed (vt `+0x90`, `FUN_005b17f0`) is 0, it
  reads `0xdd91` "Disabled" instead.
- Otherwise, if it holds a target (`+0x98`), the line takes the target's
  name after a space (`FUN_005b1610`; `DAT_006a7f40` is " "). The word
  becomes `0xdd8d` "Attacking" unless the mover's `+0x68` is 1.

The port does the same (`TacticalShipActivity`), with the speed from
`original_tactical_mobility`. It never sets `+0x68` (port: noted in the
code). The panel logs `[tactical_panel] orders_line=…` when the line
changes. In `run.mjs`:

- 'navigation-route-markers' finds "Navigating".
- 'attacking-orders-line' (the attack fixture's selected ship) finds
  "Attacking" and the target's name.

The fighter panel's line is not changed. Its function is another one and
was not traced here.

## Held buttons

- A0 TAC-02-C010 holds Task Force 1. Its slot is the selected look (1006,
  white '1'), pixel for pixel A0 TAC-02-C011's except under the pointer (37
  pixels). `run.mjs` 'task-force-1-held' and 'task-force-1-number-held'
  check the same art on the held port button.
- A0 TAC-05-C013 holds Hammer. The button is 1138 (every pixel but the
  pointer's), and the rest of the Maneuvers panel, the preview included, is
  the open panel's (0 pixels off outside the pointer, against the port's
  maneuvers-open frame). The port chose the maneuver on the press and drew
  its preview 2103 at once (6,601 pixels changed). Side-panel buttons now
  act on a release over the same button, as the Battle Options panel's do;
  the HUD's buttons still act on the press. `run.mjs` 'hammer-held' and
  'hammer-held-preview-unchanged' check it.

## Runs

Linux, Arch Chromium 155. Targeted reruns, all passing:

- 2026-11-07T20-04-26-053Z-894285 (navigation camera)
- 2026-11-07T20-04-58-413Z-895387 (attack targeting)
- 2026-11-07T20-05-11-884Z-896087 (group)
- 2026-11-07T20-05-43-481Z-897338 (command assignment)

The batch b125 passed 23 of 23 gates. fleet-finder and encyclopedia-index-shell failed on timing while scoped mutants were compiling, and both passed when rerun with nothing else running. The full tactical matrix passed 148 of 148 (run 2026-11-07T21-17-16-802Z-1499842).

## Withdrawal frame (TAC-06-C002)

No code change. The full matrix's withdrawal frame
(`production-battle-options-withdrawal/640x480/battle-options-withdraw-started.png`,
Alliance, run 2026-11-07T17-45-20-539Z-731820) against A0 TAC-06-C002:

| Region | Off from A0 |
|---|---|
| Task-force bar (0..295 × 0..20) | 0 of 6,195 |
| Emblem panel (478..635 × 24..267) | 0 of 38,552 |
| Lower HUD (478..639 × 268..479) | 0 of 34,344 |
| Fighter bar | 199 of 7,224 |

Every fighter-bar difference lies in x 336..429. The fixture keeps an
assigned squadron, which A0's battle did not have.

## A0 in the VM (Windows 10), the Empire game's battle at Coruscant

Day 676 of the unsaved Easy Empire game. Take Command was pressed on the
Battle Alert, and the captures are in `.artifacts/a0-vm/coruscant-676/`.

- 05: Task Force 1 selected. The Imperial Star Destroyer's panel reads "No
  Orders".
- 07, 08: navigation set 1 shown and zoomed. A right click on a corner point
  turns it yellow: (128, 128, 0) on the unlit faces, (255, 255, 0) on the lit
  one. The panel then reads "Navigating".
- 09: a Ctrl right click on a second point turns it magenta: (128, 0, 128)
  and (255, 0, 255).

These are the port's flat-face values for the yellow and magenta meshes
under ambient 128 and directional 204.

- 03, 04: the Windows tooltips "Unassigned Task Force" and "Task Force 1"
  over the group bar.
- 05: the contents list. Icon 2240 (a character) is at (495, 154), then
  Stormtrooper Regiment icons 2317 every 32 pixels. The fourth cell is
  clipped at x 619. A scroll bar sits at y 189: arrows 1351 at x 495 and
  1353 at x 599, and the thumb 1355, 1356, 1357 from x 515. The port has not
  matched this yet (next batch).
- 11: Get Immediate Result gives "The Imperial fleet is defeated." /
  "Coruscant is now under blockade by Alliance forces. The Imperial fleet
  has withdrawn to Sullust.", in solid green.

## Mutants

The in-place batch run over the diff tested 97 mutants and left 45
survivors. Two tests then closed the logic ones:

- `each_navigation_marker_face_is_lit_by_its_outward_normal`
- `each_shown_point_gets_a_marker_in_its_state_colour`

A scoped rerun in a copied tree tested 53 mutants and left 8:

- seven field deletions in `draw_navigation_markers`' `Camera3D`. That is
  GPU drawing, which `navigation-route-markers` covers in the browser.
- `<` to `<=` in the outward test, which is equivalent: no face of the
  tetrahedron is edge-on to its centre.

The other survivors are input and drawing paths (`begin_battle`,
`draw_original_tactical_hud`, `handle_original_tactical_controls`,
`draw_tactical_frame`). The browser probes above cover them.
