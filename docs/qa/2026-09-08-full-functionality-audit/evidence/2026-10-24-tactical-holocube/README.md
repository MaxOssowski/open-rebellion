# The tactical holocube (P24)

Date: 2026-10-24.

## Source

`ghidra/notes/tactical-panel-text.md` "Holocube":

- `FUN_005d4d10` scales a +-0.5 unit cube by the battle extent `+0x8cc`.
  That value reaches it from `FUN_005ab650` through `FUN_005c10a0` and
  `FUN_005c1d30`.
- The cube is drawn as two blue wireframe meshes, `SetColorRGB(0, 0, 1)`
  and `SetQuality(0x40)`, with opposite windings.
- `FUN_005d58c0` adds them when the option mask `0x2000000` is set.
- A0 TAC-01-C003 shows the edges pure blue, one pixel wide.

The port kept a `display_holocube` flag from the Game Options window but
never drew the cube.

## Change

- `tactical_holocube_edges` gives the cube's twelve edges at +-extent/2.
- `draw_tactical_holocube` projects them through the same retained-mode
  camera as the ships (`project_source_position`), in 64 steps so that parts
  behind the camera drop out. Each step is clipped to the aperture, inset
  by the line width; without the inset the 1280x800 viewport spilled a line
  one row out.
- The lines are drawn after the backdrop and planet and before the ships.
  That is `port:`: in the original they are depth-tested meshes, so a ship
  behind a line is covered by it there.

## Gates

- Workspace: 2747 passed, 0 failed. Render tests with the ignored
  bitmap-mask tests: 917 passed.
- New tests:
  - `the_holocube_spans_half_the_battle_extent_on_every_axis`
  - `the_holocube_edges_join_each_corner_to_its_three_neighbours`
  - `a_holocube_segment_is_clipped_to_the_aperture`
- Mutants, with both packages' tests: 74 tested, 52 caught, 1 unviable,
  21 missed:
  - 19 are in `draw_tactical_holocube`, a draw function that the browser
    suite's aperture-isolation checks cover. They caught the line spill
    above.
  - 2 are equivalent. `a | bit` and `a ^ bit` agree when `a & bit == 0`,
    and `p < 0` and `p <= 0` agree in the branch where `p != 0`.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144. That run was before
  the last clipping test was added, which changes only test code.
- Replay equivalence passed.
- Release WASM `67c3e1354b982ee82bbac596d88b70062a702318933a980720914234db7768d4`.
