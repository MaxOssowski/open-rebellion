# The Fleet window's pane title is clipped to its label

Date: 2026-11-10. Follows `2026-11-10-fleet-window-scroll-bar-a0`. Adds
to OBJ-06-C010 and OBJ-10-C006.

## The original

`FUN_004a4b10` gives the selected fleet or ship its label +0x190 at
(100, 29), 128 by 25, format 0, font 5. `DrawTextA` without
`DT_NOCLIP` clips to that rectangle. A0 `fleet-aboard/10-isd1-fighters`
shows "Imperial Star Destroyer 1" cut at window x 228, after "Destro".

## The port before

The label was painted unclipped, so the name ran on past x 228 over
the picture panel's edge.

## The change (`fleet_window.rs`)

Both label calls, the fleet's and the ship's, paint under a clip of the
label's rectangle.

## Tests

- `a_selected_ship_is_framed_named_and_shows_three_tabs_lit_by_its_own_cargo`
  and `the_window_writes_its_names_and_counts_where_the_original_does`
  now check the clip of the ship's and the fleet's label. Each fails
  without its half of the change. The test-only `Text` record keeps the
  clip it was painted under.
- `fleet-window.mjs a0-coruscant` (Empire, Linux Chromium): the ship's
  tabs differ from A0 in 2967, 4016 and 2065 pixels, 51 fewer each than
  before (3018, 4067, 2116). What is left is the text's rasterising. The
  ceilings were lowered to 3050, 4100 and 2150.

  Run: `.artifacts/interface-parity/fleet-window-2026-11-10T16-28-07-299Z-1310415`.

## Mutants

`cargo mutants` on the diff (copy mode): 1 mutant, caught.

## Batch b176

23 of 23 interface gates on Linux, `fleet-window`
(`fleet-window-2026-11-10T16-41-27-453Z-1349612`) among them with the
lowered ceilings; workspace tests 3503 passed, fixture tests 140.
Tactical matrix `2026-11-10T17-29-09-269Z-1450730`: 148 of 148.
