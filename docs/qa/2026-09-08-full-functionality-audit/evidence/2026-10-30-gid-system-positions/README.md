# System items at the original's exact places (CMD-02)

Date: 2026-10-30. Surface ledger CMD-02 (Galaxy display and GID modes).

## The original

`FUN_00425d00` places each system's item when a display is built (asm
0x425f8d..0x426143):

- **The coordinates.** It reads the system's x and y (`FUN_00509620`,
  SYSTEMSD `x_position` and `y_position`).
- **The scale.** It divides each by `1023 + 1` (`0x658bd8`, `0x658bda`),
  then multiplies x by 607.0 (`0x658be0`) and y by 437.0 (`0x658be8`).
- **Rounding.** `__ftol` truncates (rounding mode chop).
- **The side's origin.** It adds (21, 25) for side 1 and (84, 27)
  otherwise.
- **The cell.** It sets the item's `+0x28`/`+0x2c` to that point and its
  rect to 17 by 17 from there.
- **Placement.** `FUN_0042cde0` blits every mark keyed from that corner
  (`FUN_005fd0f0`), whatever its size; nothing is centred.

So x = (sx × 607 >> 10) + 21 | 84 and y = (sy × 437 >> 10) + 25 | 27. The
scale does not depend on the galaxy size.

## What A0 shows

`findmarks.py` (this folder) searches each capture for the exact, keyed
STRATEGY marks 10130..10181. In the 21 A0 displays of
`../2026-10-30-gid-menu-a0/` it finds the marks of 100 systems, each at
exactly the corner the formula gives: 100 of 100.

The port had measured its mapping from resampled web frames
(`GALAXY_MAP_SCALE` 0.5946 and 0.4286, offset 7.7 and 6.7) and centred
each mark on that point. Its marks sat 1 or 2 pixels off.

## The port

`lib.rs`:
- **`GALAXY_MAP_SCALE`** is 607/1024 and 437/1024.
- **`CameraView::to_screen`** chops to whole logical pixels and returns the
  centre of the 17 × 17 cell. `GALAXY_MAP_OFFSET` is half a cell.
- **`draw_gid_marker`** hangs every mark from the cell's top left.
- **Other users of `to_screen`** (hover, fleets, fog) take the cell's
  centre.

## Comparison

The Linux GID run (interface WASM `3a24dcde…`) was searched with the same
script, across all of the port's Empire displays:
- 98 of the A0 corners appear in the port's marks exactly;
- the other two, systems 125 and 262, are corners the formula gives too.
  In the fixture something lies over their marks, so the search finds no
  bare bitmap there.

Before this change none of the 100 matched exactly.

## Browser

`run.mjs --all` on Arch Chromium 155: 152 of 152 pass (`gid-summary.json`).

## Tests

- `cargo test --workspace`: all pass.
- New test: `a_systems_cell_sits_where_fun_00425d00_places_its_item` (both
  sides, scales 1 and 2).
- `cargo mutants --in-place` on `to_screen` and `galaxy_camera`: every
  mutant caught.
- The same run listed 11 missed mutants outside this change, all
  untouched: the draw calls' `dest_size` and the unchanged
  `map_to_screen` wrapper, which the name filter matched.
