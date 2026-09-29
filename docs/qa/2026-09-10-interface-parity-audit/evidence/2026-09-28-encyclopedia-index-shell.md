# P63 Encyclopedia index-shell checkpoint

P63 reconstructs the original faction-specific Encyclopedia index shell and
its ten bitmap category controls. The implementation remains test-only while
the catalog, text, topic, and navigation contracts are recovered. Production
command `0x131` and F7 therefore continue to fail closed.

## Source contract

`FUN_00429f30`, `FUN_00466350`, and `FUN_004665f0` establish the window route,
shell composition, and control geometry. The detailed mapping is preserved in
[`ghidra/notes/encyclopedia-window.md`](../../../../ghidra/notes/encyclopedia-window.md).

- faction bases: STRATEGY 10335 and 10336;
- faction rails: STRATEGY 10820 and 10821 at 412,0;
- index content: STRATEGY 10822 at 12,13;
- native resource surface: 470 by 331;
- ten controls: commands `0x79` through `0x82` at 22,46, using 28 exact
  normal and pressed resources across both factions.

The owned source set contains 33 unique STRATEGY bitmaps with aggregate
SHA-256 `df0891b2b8182aebc4ada40ed3d72a38b3d4e1a090d21c41ead388425ee4679f`.

## Implementation

- `crates/rebellion-render/src/encyclopedia.rs` composes the source bitmaps at
  native coordinates and returns the recovered command ID only for an exact,
  right-and-bottom-exclusive hit.
- `crates/rebellion-app/src/main.rs` blocks strategic-map pointer input while
  an Encyclopedia surface is present, preventing modal clicks from opening an
  underlying system window.
- fixture scenario 40 is compiled only with `interface-test-fixtures` and is
  absent from production artifacts.
- `tools/interface-parity/encyclopedia-shell.mjs` launches a new muted pinned
  browser process for each faction, builds independent source composites, and
  compares normal, ten held, and outside-edge states.

## Verification

The durable result is summarized in
[`p63-encyclopedia-index-shell/summary.json`](p63-encyclopedia-index-shell/summary.json).
Raw owned bitmaps and browser captures remain under ignored local storage.

| Gate | Result |
|---|---|
| Renderer tests | 10 passed |
| Fixture decoding tests | 2 passed |
| Harness unit tests | 26 passed |
| Production and fixture WASM build | pass |
| Production fixture-token exclusion | pass |
| Fresh muted browser processes | 2 of 2 passed and closed |
| Startup requests | 8 of 8 returned HTTP 200 |
| Source comparisons | 24 of 24 exact |
| Pixels compared | 3,733,680; zero different |
| Browser diagnostics | zero errors |
| Independent browser-evidence visual review | pass; no P0-P3 findings |

The independent reviewer inspected the retained 24-state contact sheet. It
found intact textures and chrome, correct faction consistency, distinct held
states without neighboring-control disturbance, and normal outside-edge
states. This was a visual evidence review, not an independent browser rerun.

## Acceptance boundary

This checkpoint proves the faction shell, index composition, control bitmap
states, fixed geometry, modal pointer boundary, and one exact edge probe. It
does not prove category meanings, catalog ordering, stable entity bindings,
ENCYTEXT rendering, topic composition, previous/next behavior, contextual
entry, window dragging, 470-by-330 outer clipping, viewport variants, or A0
comparison. No `OBJ-01` cell is accepted by P63 alone.
