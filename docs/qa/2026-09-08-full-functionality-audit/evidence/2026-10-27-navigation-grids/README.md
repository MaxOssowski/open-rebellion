# Navigation points are the original's four grids (P24)

Date: 2026-10-27. The port's navigation points were A1 placeholders: 4 to
6 hand-placed points a set, "until the executable's point producer and A0
positions can be compared". This stage replaces them with the original's
producer.

## The original

### The producer, `FUN_0059f800`

The battle setup `FUN_005ab650` calls it at `0x5ac5d2` with the
battle extent `[battle+0x8cc]`:

- base = extent × 0.125 (`DAT_0066bb4c`, `0x3e000000`).
- For k = 1 to 4 (`cmp ebx,4` at `0x59fab2`), step = k × base.
- Three nested loops, x outermost, then y, then z, each over -1, 0, 1.
  Each point is placed at (x, y, z) × step (`FUN_0059fb10`, at the
  point's `+0x44`..`+0x4c`).
- k = 1 makes all 27 points. For k ≥ 2 the centre is skipped, so those
  grids hold 26.
- Each point records its layer, y + 1, at `+0x54` (`0x59f933`).
- Set k - 1 goes to list `ctx + 0x114 + (k - 1) × 0xc`.
- All four sets start hidden (`ctx + 0x144..0x150` cleared).
- A hidden point is not pickable (`+0x34`, `FUN_0059fbd0`).

### The HUD buttons

The buttons are made in `FUN_0059c260`, 27 × 27, at y 272:

| Button | x | Command (`FUN_0059b130`) | Set (`FUN_0059fd40` -> `FUN_0059fbd0`) | Spacing |
|---|---|---|---|---|
| `0xad12` | 485 | `0x10` | 3 | 0.5 × extent |
| `0xad13` | 524 | `0x11` | 2 | 0.375 × extent |
| `0xad14` | 563 | `0x12` | 1 | 0.25 × extent |
| `0xad15` | 602 | `0x13` | 0 | 0.125 × extent, with the centre |

### The A0 run (Windows VM, Battle at Yavin)

- Each set is a 3 × 3 × 3 grid of 3D markers.
- The leftmost button's grid is the widest and the rightmost's the
  tightest.
- The colour follows the layer: the top layer white, the middle one dark
  red (148, 6, 0), the bottom one blue (0, 46, 178).

| Capture (`.artifacts/a0-vm/2026-10-27-controls/`) | SHA-256 |
|---|---|
| `nav-set-1-only.png` (HUD button 1) | `fd2f074c6c4a97b249dd863e9a84c6a7bfa6d58a48915f7d8b6199adf46856a2` |
| `nav-set-2-only.png` (HUD button 2) | `3b1803e29e2f146475d80e6ee51de0f85482231a8a60f7cae926a961d7fca832` |
| `nav-set-3-only.png` (HUD button 3) | `9ea44906383f3f4afecfbf359a785525d4f86d860ef3a5b842db0c3a8396f89b` |
| `nav-set-4-only.png` (HUD button 4) | `7b820139a1f83c4d33aa44ebb44fec678f9f5c9c56d81fd11facfc89966cb1ba` |

## The port

- `original_navigation_point_sets` in `crates/rebellion-core/src/tactical.rs`
  builds the grids. Index b is HUD button b, the grid of k = 4 - b. The
  points come in the original's order and spacing
  (`ORIGINAL_NAVIGATION_SPACING`).
- `navigation_layer_color` in `crates/rebellion-render/src/tactical_view.rs`
  colours a point by its layer (its y). The colours are the observed ones.
- The simulation does not read the navigation points, so replays are
  unchanged.
- The grids now put points under ships too. A right click takes a ship or
  squadron under the pointer before a navigation point
  (`no_unit_under_pointer`), as the left click already did; port: this
  stands in for the original's frontmost pick (`FUN_005c2d20`).
- The browser navigation journey now picks points that no participant
  covers (24 px clear), where before it took the first two in the
  aperture. With the denser grid, one of those lay under an enemy ship in
  the Empire run, and the click targeted the ship.

Not exact:
- hyp: that `+0x54` selects the colour is inferred, because the code that
  reads it is not traced. The colours come from the A0 frames.
- port: the markers stay the port's flat triangles, not the original's 3D
  marker mesh.

## Tests

- `the_navigation_sets_are_the_originals_grids`
- `a_navigation_points_colour_follows_its_layer`
- `a_unit_under_the_pointer_comes_before_a_navigation_point`

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2973 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | first run 144 passed, 4 failed (`production-navigation-camera-presentation`); after the right-click priority and the journey's point choice, 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 38 tested: 33 caught, 2 unviable, 3 missed |
| Release WASM SHA-256 | `2cb0b4d7e5442d5972eb81f5949454faf86805e6e5098b8d8a88981f26e9256d` |

The three missed mutants all sit in `draw_tactical_navigation_points` and
`handle_combat_input`, which need a live macroquad window:
- either function replaced with `()`;
- the right-button test `&&` against `||`.
