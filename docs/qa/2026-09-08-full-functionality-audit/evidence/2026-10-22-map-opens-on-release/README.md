# The galaxy map opens a system on the release (P07)

Date: 2026-10-22. The reading is in `ghidra/notes/gid-markers.md`.

## Reading

`FUN_00422ce0` has no `WM_LBUTTONDOWN` case in the galaxy view. Its
`WM_LBUTTONUP` opens the item under the pointer (`+0x25c`,
`FUN_00429ce0`). A press in another window keeps its release in that
window (the capture).

## Change

`map_release_activates` selects and opens a system on the release. The
release counts only after a press on the map and only over the map.
Before this change the port opened the system on the press.

## Gates

- Tests: 1 new (press, release, a click in one frame, a press in a
  window, a release over a window or empty space). The targeting test
  now also covers a click off the map. Workspace 2666 passed, 0 failed.
- Mutants: 8 of 8 caught, 1 unviable, after the off-map test moved into
  `map_press_selects`. Before that, one miss: `&&` to `||` in
  `draw_galaxy_map`.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152. This includes the legend probe, which
  opens the probe system with a click.
- Replay equivalence passed.
- Release WASM `a573ff0df8a0f855f48a00bd7dddd7ce44aef4503744a0d7f566eda89f329257`.
