# A click on a system closes the GID legend (P07)

Date: 2026-10-22. The reading is in `ghidra/notes/gid-markers.md`.

## Reading

`FUN_00422ce0` runs this on `WM_LBUTTONUP` in the galaxy view (`+0xc0`
= 1). The view tracks the item under the pointer on `WM_MOUSEMOVE`
(`+0x25c`). When a release lands on such an item:
- if the legend (`+0x124`) is visible, the view posts `0x405` with its id;
  that is case 3, which hides the legend and shows the legend button;
- then `FUN_00429ce0` opens the item's system.

## Change

`GidLegendWindow::close_for_map_click` closes a visible legend when the
map opens a system, and leaves a closed one as it is. The app logs
`command=0x405 destination=gid_legend status=closed_by_map_click`.

port: the port opens a system on the press, as before this change; the
original does it on the release.

## Gates

- Tests: 1 new (the click closes only a visible legend). Workspace 2665
  passed, 0 failed.
- Mutants: 2 of 2 caught.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152. Every case that chooses a display now
  also:
  - drags the legend to (10, 10) and checks its frame there at
    640 x 480;
  - clicks the probe system;
  - waits for the log line;
  - at 640 x 480, checks that the frame is gone.
- The browser probe waits two frames after each press before dragging.
  A press and a move sent in one frame reach macroquad at the moved
  position, so the map took a press made inside the legend (seen at
  (147, 110) during the drag).
- Replay equivalence passed.
- Release WASM `dda8d52d79c308e9eecfe35218b795784078cc65a052990f9c6b8db84511fb1c`.

Found on the way, not fixed: a legend dropped at the very top-left
corner (0, 0) loses its outer grey line at 640 x 480. Its pixels there
are (16, 16, 16).

Correction (same day): the legend was drawn in full. The (16, 16, 16)
edge was the browser's focus ring around the canvas, on every frame;
see `../2026-10-22-canvas-focus-ring/`.
