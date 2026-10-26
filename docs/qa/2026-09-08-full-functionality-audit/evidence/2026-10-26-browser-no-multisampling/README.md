# The browser canvas draws without multisampling (P24)

Date: 2026-10-26. This closes the last item of the 8-bit output
(`../2026-10-26-rgb-cube-output/`, `../2026-10-26-flat-hull-shading/`).
After the colour cube and the flat light, the browser frame still had
colours outside the cube. They were blended hull edges and blended field
lines.

## Why

- The vendored miniquad `web/gl.js` created its context with
  `canvas.getContext("webgl")` (and `"webgl2"`). The browser default is
  `antialias: true`, so every edge was multisampled.
- The original drew into an 8-bit surface through d3dim's RGB emulation
  (`FUN_7403e4f0`), which blends no edges. Its field lines are solid
  (0, 0, 255): 1309 pixels of A0 TAC-01-C003's space area have that one
  blue.
- Native macroquad already renders without multisampling.

## The change

`web/gl.js` asks for `{ antialias: false }` in both contexts. Nothing else
in the context changes.

## Against A0

Scenario 37 (`production-hull-scale-a0`), 640 × 480, run
`2026-10-26T02-50-17-588Z-3408033`, with the boxes and counting of
`../2026-10-26-rgb-cube-output/`:

| | Victory colours (outside the cube) | Transport colours (outside the cube) |
|---|---|---|
| A0 TAC-01-C003 | 12 (0) | 22 (11) |
| Before: cube and flat light | 45 (34) | 81 (70) |
| Now | 11 (0) | 27 (15) |

- The Victory now uses only cube colours, one fewer than A0.
- Both transport boxes take in more than the hull: the window frame at
  the left in both, and in the port the yellow selection corners, since
  the port's frame has the transport selected.
- Enlarged side by side, the hulls show the same palette colours and flat
  facets, and the field lines are solid in both.
- The port's hulls remain a little brighter than A0's. d3drm's exact RGB
  lighting equation was not decoded.

## Gates

| Gate | Result |
|---|---|
| Browser tactical matrix | 148 of 148 passed |
| Browser interface matrix (`gid`, all) | 152 of 152 passed. Before the change the same matrix passed 152 of 152 (run `2026-10-26T02-14-10-371Z-3225716`); after it, run `2026-10-26T03-02-00-722Z-3437014` |
| Workspace tests, mutants, replay equivalence | not rerun: no Rust code changed |
| Release WASM | unchanged (`7b1b2926…`) |

The browser runs used Arch Chromium 155.0.8059.84 (`/usr/bin/chromium`)
through a temporary Linux manifest, not the pinned macOS Chrome for
Testing.
