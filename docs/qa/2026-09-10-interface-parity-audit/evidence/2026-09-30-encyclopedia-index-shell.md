# P64 Encyclopedia index-shell checkpoint

P64 reconstructs the original faction-specific Galactic Encyclopedia index
shell and its seven bitmap category controls. The dormant renderer compiles in
production, but only the browser verification route is test-only while the
catalog, labels, topic pages, navigation, contextual entry, and production
route are recovered.

## Source contract

`FUN_00429f30` establishes the Encyclopedia route and 470 by 330 client.
`FUN_0045d400` and `FUN_0045ddc0` establish the two display modes, faction
layers, seven category controls, and right rail. `FUN_0045f100`,
`FUN_0045f480`, `FUN_0045fa60`, `FUN_0045fd20`, and `FUN_0045fe60` establish
filtering, mode changes, EDATA and ENCYTEXT loading, linked topics, and keyboard
navigation. The exact table is preserved in
[`ghidra/notes/encyclopedia-window.md`](../../../../ghidra/notes/encyclopedia-window.md).

- faction bases: STRATEGY 10335 and 10336, native 470 by 331 and clipped to
  the 470 by 330 client;
- index content: STRATEGY 10338 at `(12,13)`;
- faction rails: STRATEGY 10585 and 10589 at `(412,0)`;
- seven controls: commands `0x6f` through `0x75` at `(36,78)`, using 28 exact
  normal and pressed resources across both factions;
- right rail: close `0xfb`, topic `0x67`, and index `0x68`, using twelve exact
  normal and pressed resources across both factions;
- command `0x75`: native 49- or 50-by-57 artwork clipped from source origin to
  its 49 by 41 control. It is not scaled or centered.

The owned source set contains 41 unique STRATEGY bitmaps with aggregate
SHA-256 `3a440072a73b1f69fa9134a9fd88e5ccf2994047cafb9c538cf72fc6875e2c11`.

## Implementation

- `crates/rebellion-render/src/encyclopedia.rs` composes the index shell at
  native coordinates, clips oversized resources, applies original palette-key
  hit masks, captures only opaque press origins, cancels outside releases, and
  returns only source command IDs.
- `crates/rebellion-app/src/main.rs` blocks strategic pointer input while the
  test-only modal fixture is present.
- fixture scenario 41 and its application route are compiled only with
  `interface-test-fixtures` and are absent from production artifacts. The
  unintegrated renderer itself remains available for the eventual production
  route.
- `tools/interface-parity/encyclopedia-index-shell.mjs` launches one fresh
  muted pinned browser per faction, builds independent source composites, and
  compares selection, held rail, clipping, outside-edge, transparent-pixel,
  press-capture, drag-cancel, and modal click-through states.

## Verification

The durable result is summarized in
[`p64-encyclopedia-index-shell/summary.json`](p64-encyclopedia-index-shell/summary.json).
Raw owned bitmaps and browser captures remain in ignored local storage.

| Gate | Result |
|---|---|
| Renderer tests | 10 passed |
| Fixture decoding tests | 2 passed |
| Scoped mutation runs | renderer: 58 draw/input survivors, 4 unviable; app: 2 caught, 8 main-loop survivors, 1 unviable |
| Production and fixture WASM build | pass |
| Production fixture-token exclusion | pass |
| Fresh muted browser processes | 2 of 2 passed and closed |
| Startup requests | 8 of 8 returned HTTP 200 |
| Source comparisons | 32 of 32 exact |
| Pixels compared | 4,963,200; zero different |
| Browser diagnostics | zero errors |
| Independent source adjudication | pass; mapping promoted within the stated boundary |
| Independent browser-evidence visual review | pass; no P0-P3 findings |

The independent visual reviewer inspected a retained two-faction contact sheet
covering fourteen representative browser states. It found consistent faction
chrome, isolated selected and held changes, stable neighboring controls,
undistorted native clipping on the final category, and stable transparent,
press-capture, drag-cancel, and modal-block negatives. The reviewer did not
independently rerun the harness.

The scoped mutation runs explain rather than conceal the rendering survivors.
`cargo test` does not execute egui paint or the WASM main loop, so mutations in
those paths survive its unit-test command. The independent browser compositor,
32 exact display comparisons, palette-key and press-capture probes, modal log
negative, and inspected contact sheet are the executable gates for those
paths. Two app mutations were caught by the fixture tests; compile-invalid
whole-table replacements account for the five unviable mutations.

## Acceptance boundary

This checkpoint proves the empty index bitmap composition, faction variants,
seven selected category states, three rail held states, native clipping, modal
pointer boundary, exact outside-edge and transparent-pixel rejection, opaque
press-origin capture, and drag-out cancellation. It does not prove localized
labels, populated rows, catalog ordering, category semantics beyond recovered
object-family ranges, topic pages, EDATA and ENCYTEXT bindings, previous and
next navigation, contextual entry, missing entries, close and mode routing,
window dragging, viewport variants, or original-runtime comparison. No
`OBJ-01` cell is accepted by P64 alone.
