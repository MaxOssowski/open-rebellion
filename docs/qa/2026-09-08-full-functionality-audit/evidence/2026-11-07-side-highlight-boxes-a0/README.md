# The side highlights box each ship (A0)

Date: 2026-11-07. Cells TAC-04-C006 (Alliance highlight) and TAC-04-C007
(Empire highlight).

## A0

- `TAC-04-C006` (Mon Calamari, Alliance): a pure red (255, 0, 0) box around
  the Alliance ship.
- `TAC-04-C007`: pure green (0, 255, 0) boxes around the Imperial ships.
- `.artifacts/a0-vm/research-display/32-empire-highlight.png` (a new Easy
  Empire game, the battle at Corsin on day 660, Take Command): the Empire
  toggle lights a green box around each Imperial ship, one pixel wide. The
  box's edges behind the hull are hidden by it.

The boxes are the hull's 3D box drawn as lines, not 2D brackets.
`FUN_0059f380` sets `+0x100` on every unit of a lit side each frame
(`+0x10c` for the Alliance toggle, `+0x110` for the Empire's, which
`FUN_0059ef40` and `FUN_0059ef20` flip on commands `0x14` and `0x15`). The
function that draws the box from that flag is not traced.

## The port

Before, the toggles changed only the 2D fallback markers: a red or green
square around a marker. Ships drawn from their meshes, which is every ship
in a real battle, showed nothing.

- `TacticalRenderObject::highlight` carries the colour
  (`tactical_highlight_color`: red for the Alliance, green for the Empire,
  while that side's toggle is on).
- `TacticalAssetRenderer::draw_participants` draws the twelve edges of the
  drawn mesh's local bounds (`highlight_box_edges`) with `draw_line_3d`.
  The lines come right after the hull, in the same 3D pass, so the depth
  test hides the edges behind hulls drawn before them.
- port: the box is the drawn level of detail's bounds. The original's box
  source is untraced.

## Tests

- `a_lit_side_draws_its_ships_boxes_red_or_green`: the colours by side and
  toggle, and the twelve axis-parallel edges between the bounds' corners.
- `run.mjs` (`--battle`, `production-participants`, Linux, both sides):
  both toggles on add more than 20 pure red and 20 pure green pixels in the
  battle view (Empire 640x480: 2 red before, 111 red and 79 green after),
  then both go off again.
- cargo mutants: 18 caught. The misses are the corner and edge
  comparisons, which give the same box when swapped; the render loop's
  arithmetic, which only the browser sees; and the side test that picks the
  colour (`draw_tactical_frame`), which the pixel count does not tell apart.

## Runs

Batch b123 (23 of 23 gates), then the whole tactical matrix: 148 of 148
on Linux Arch Chromium 155 (run `2026-11-07T16-37-47-906Z-577170`).
