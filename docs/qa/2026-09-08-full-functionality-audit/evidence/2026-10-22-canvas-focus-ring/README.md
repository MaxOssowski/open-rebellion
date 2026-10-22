# The browser drew a focus ring around the game (web shell)

Date: 2026-10-22.

## Finding

The previous stage (`2026-10-22-gid-legend-map-click/`) reported that a
legend dropped at (0, 0) lost its outer grey line. That was wrong: the
legend is drawn in full. The page's 1-pixel edge was (16, 16, 16) in
every frame:
- with and without the legend;
- at 640 x 480;
- in the letterbox black at 1280 x 800, where the game draws nothing.

The command-centre bitmap's own edge (STRATEGY 900) is (39, 91, 103) at
(0, 0) and (233, 235, 235) at (0, 200).

Cause: the page focuses the canvas (`canvas.focus()` in
`web/index.html`), and Chromium drew its focus ring over the canvas's
edge. The original's window has no such frame.

## Change

- `web/index.html`: `canvas:focus { outline: none; }`. The page's edge
  now shows the game's pixels: (39, 91, 103) and (233, 235, 235) at
  640 x 480; black in the letterbox.
- The GID legend probe drops the legend in the corner again. At
  640 x 480 it checks grey at (0, 0) and white at (1, 1), then the map
  click closing it.

## Gates

- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- Workspace 2666 passed, 0 failed. Provenance scan: 0 uncited items.
  Replay equivalence passed.
- No Rust change: the release WASM is unchanged,
  `a573ff0df8a0f855f48a00bd7dddd7ce44aef4503744a0d7f566eda89f329257`.
