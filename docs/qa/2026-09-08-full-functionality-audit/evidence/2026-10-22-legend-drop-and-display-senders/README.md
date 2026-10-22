# The legend's drop, and a second sender of GID displays (P07)

Date: 2026-10-22. The reading is in `ghidra/notes/gid-markers.md`.

## The legend's drag

- `FUN_00606380` sets every window's `+0xf4` to 1 (live drag), and the
  legend's constructor `FUN_00452090` keeps it.
- While held, `FUN_006071e0` moves the window with the pointer, without
  bounds.
- On the drop, `FUN_00607010` moves it back inside the bounds
  `FUN_006071a0` took from the parent. It goes to `left` when it is
  left of them, else to `right - width` when it passes them; the same
  for the height.
- Before this change the port clamped the window while it was held. It
  now follows the original. A window held past the command centre is
  cut at the canvas edge.

## Correction: GID displays reach the view from the advisor too

The notes said that displays `0x13..0x17` and `0x91..0x93` are never
shown. That was wrong: command `0x140` has a second sender.
- `FUN_0041d8c0(display, ..)` sends `WM_COMMAND (display << 16 | 0x140)`.
- `FUN_004c0fc0(code)` calls it with `0x11`, `0x13`, `0x14`, `0x15`,
  `0x17`, `0x20`, `0x91` or `0x80`. `FUN_004c0a60`, the advisor's
  renderer, runs it through `FUN_004c0f60` for a pending action of
  kind 2.
- That action is set from a `Class_00658848` item (`+0x54`, `+0x58`).

Who makes those items is not traced, and the port has none of these
displays. This is now an open item.

## Gates

- Tests: the drag test now holds the window past the edges and checks
  the drop. Workspace 2667 passed, 0 failed.
- Mutants: 10 of 13 caught, 1 unviable. Of the 3 missed:
  - `<` to `<=` and `>` to `>=` in the drop are equivalent: at the bound
    both give the same position;
  - `draw_gid_legend` needs an egui frame; the browser probe drags the
    legend into the corner and checks its frame there.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- Replay equivalence passed.
- Release WASM `0b8e4a52fb93b97a7a1e6b0e1bb6cacb867dae9509b65ece08b833d8f5688146`.
  It was built before a test-only addition.
