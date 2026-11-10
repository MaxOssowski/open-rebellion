# Three invented galaxy-map overlays removed

Date: 2026-11-10. Owner's decision of the same day: what the original
does not have is removed. Code cleanup, adding to CMD-02-C022
(blockade).

## What was removed

`crates/rebellion-render/src/lib.rs` carried three public overlay
functions of the old macroquad galaxy map. Nothing in the workspace
called any of them:

| Function | Drew | Original |
| --- | --- | --- |
| `draw_blockade_indicators` | a pulsing red ring around each blockaded system | none: `FUN_0042b330` and `FUN_0042cde0` never read the blockade bit `+0x88 & 0x20`, so a blockaded system's item is its display's mark alone (CMD-02-C022) |
| `draw_facility_icons` | yellow, cyan and orange 5 by 5 squares beside each system for its facility kinds | none: the GID shows one mark per system for the chosen display (`ghidra/notes/gid-markers.md`) |
| `draw_sector_boundaries` | a convex-hull outline around each sector, coloured by sector group | none: a sector shows as its name over its cluster (A0 gid-selection/01) |

Their private helpers went with them: `map_to_screen`,
`in_map_viewport`, `convex_hull`, `sector_boundary_color` and the
`BlockadeState` import. 199 lines, then the helper block.

## Checks

- `cargo check --workspace` gives no new warnings. The removed helpers'
  "never used" warnings are gone.
- The rebellion-render tests pass, 1406 of them.
- None of the functions was called, so no browser frame changes. The
  batch's gates confirm it.

## Batch b170

23 of 23 interface gates on Linux; workspace tests 3498 passed, fixture
tests 139. The diff, deletions only, left no mutants. Tactical matrix
`2026-11-10T08-56-59-259Z-173607`: 148 of 148.
