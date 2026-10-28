# The window reference rail: the System window's icon, the yellow name (P06)

Date: 2026-10-28. The audit's open item was "exact active/inactive rail
thumbnails" (UIP-F-011, CMD-04). The reading is in
`ghidra/notes/sector-quadrants.md`, "The window reference rail".

## The original

- `FUN_00428d70` draws a rail slot:
  - the minimized window's icon (its slot `+0x74`) keyed at the slot's
    corner;
  - the system's name after the icon, at x = the icon's width and y = 0,
    in font 10 and color `0x200f0f0` (yellow).

  Every window type uses the same layout. There is no separate active or
  inactive thumbnail.
- The System window's icon (`FUN_00458cf0`) depends on the system's side:
  11530 for side 1, 11531 for side 2, and 11532 otherwise.
- The twelve slot rects come from `SetRect` at `0x421f3b` (side 1) and
  `0x42207e` (side 2). They match the port's constants, which until now
  had no source.
- A0, Windows 10 VM, original `REBEXE.EXE`, the "test" save (Alliance), day
  10. The System window of Vjun minimized to the rail as the red factory
  icon with "Vjun" in yellow after it. There is no blue behind the icon.

  | Capture (`.artifacts/a0-vm/2026-10-28-nav/`, ignored) | SHA-256 |
  |---|---|
  | `galaxy-a.png`, the galaxy view | `6e5ccf7739fece0cb058d64de7f1930998fb5cf4208c31e363da498a875584d3` |
  | `sector-a.png`, the Moddell sector window | `e88f5b40c9357721d95f9550e0cc5bdcdd68569c4f0753e5caf120577cbae017` |
  | `system-vjun.png`, the System window | `2602c287241b2d38e319270e3d00d85816606d1e7ae81da99a0cf96883f09ecd` |
  | `rail-vjun0.png`, minimized to the rail | `1e92a3e232847d0d5a5b3c41d150ce1274925c6f5348bb2024f64a3df27f4a1c` |

## The port

Before this change:
- the port drew a System window's slot as the window's background with a
  dark wash and the name centered, 7 points, in a relationship color;
- the other windows' names sat at (12, 2), 9 points, in that color;
- the rail icons 11530..11532 were opaque.

Now (`crates/rebellion-render/src/system_window.rs`, `draw_reference_rail`,
`rail_icon`):
- every slot draws its window's icon, then the name in font 10, yellow,
  at the icon's width, clipped to the slot;
- 11530..11532 key out their blue matte (`bmp_cache.rs`).

The relationship colors and the unused `paint_resource` are removed.

## Tests

- `a_system_windows_rail_icon_follows_the_systems_side`: new.
- `a_minimized_fleet_window_keeps_one_rail_slot_beside_its_systems_window`:
  its text expectations changed for `FUN_00428d70`. It used to expect
  (12, 2) at 9 points and 7 points for the System window. Now the name sits
  at the slot's corner (the fixture has no bitmaps, so the icon's width is
  0) in font 10. The System window's slot also paints 11530.
- `the_defenses_windows_tabs_frames_and_rail_icons_key_out_their_blue_matte`:
  11530 and 11532 are now keyed; the opaque boundary moves to 11529.
- `reference_rail_rectangles_differ_by_faction` now cites the `SetRect`
  calls.

## Browser

Arch Chromium 155, run through a temporary Linux manifest.
`sector-quadrants.mjs`'s `open-system` case now minimizes the System window
and compares the rail icon pixel for pixel: 11530 for the Alliance, 11531
for the Empire. Before the matte fix, the icon showed on a blue square;
after it, the slot matches the A0 capture, apart from the font's
antialiasing.

| Capture | SHA-256 |
|---|---|
| `sector-quadrants-2026-10-28T14-42-51-007Z-2728647/alliance-open-system/system-railed-screen.png` | `2c574308698b718e7ac08bc864b2b26da971e48e1990a0bccec1d21fd9b7e9ef` |
| `.../empire-open-system/system-railed-screen.png` | `31b8edf97305772af3482fb2b946e59494bdf33a05c8fdd273f8ea71d1e124a3` |

## Gates

- Workspace: 3022 tests passed. Provenance scan: no uncited items.
- Mutants (`rebellion-render`, the diff): the first run missed deleting
  the side 2 arm of `rail_icon`. After the new test: 8 mutants, 7 caught,
  1 unviable.
- Browser: `sector-quadrants.mjs` 12 of 12 (run
  `sector-quadrants-2026-10-28T14-50-01-452Z-2776590`); `fleet-window.mjs`
  `chrome`, which minimizes to and restores from the rail, 2 of 2 (run
  `fleet-window-2026-10-28T14-50-36-322Z-2778839`).
- rustfmt: `system_window.rs` is clean, as it was before.
- Release WASM SHA-256 `80113df63b3e117a020dcbc9f112499a4d2849839b0083e77c726fc709dc0515`.
