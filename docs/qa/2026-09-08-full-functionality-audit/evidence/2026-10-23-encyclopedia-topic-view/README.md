# The Encyclopedia's topic mode (P35)

Date: 2026-10-23. Not routed: the production window is still the
replacement, as for the index (P64, P65).

## Source

- `FUN_0045ddc0` topic layout (`ghidra/notes/encyclopedia-window.md`):
  STRATEGY 10337 at `(12,14)`, EDATA 400 x 200 at `(12,31)`, the title in
  `(36,14)` 350 wide, the body in `(17,231,395,80)`, previous `0x83` at
  `(28,14,21,17)` (10385..10387) and next `0x84` at `(380,14,21,17)`
  (10382..10384); the rail makes `0x67` current.
- `FUN_0045fd20`: previous / next are available when the list item has a
  previous / next item (slots `+0x10`, `+0xc`), so they step through the
  current category and stop at its ends.
- `FUN_0045fe60`: Left / Right in topic mode are `0x83` / `0x84`.
- `FUN_0045f480`: `0x67` / `0x68` switch topic and index.

## Change

`encyclopedia::draw_encyclopedia_topic`, `OriginalEncyclopediaTopic`
(title, body, EDATA number from the P66A bindings),
`original_topic_neighbours`, `apply_original_topic_command`,
`EncyclopediaState::original_topic_mode`; the index catalog's `0x67`
opens the selected object's topic. port: the body's font size and line
spacing are the port's.

## Gates

- Workspace 2703 passed, 0 failed.
- Mutants on the touched render code: 39 tested; 10 alive after the
  second run, of which the egui tests then closed the two font sizes. The
  rest need a click on an original control, whose hit mask comes from
  STRATEGY.DLL (`BmpCache::is_resource_hit`), which unit tests do not
  load: the index catalog's return value and its `0x67` branch, and the
  rail's current-control flag in topic mode.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152 (no topic case yet).
- Replay equivalence passed.
- Release WASM `723a731df3a517f7f4664831c4eeee9d4c8c02d56b1e6650edc127088826076c`
  (the tests changed after it; the code it builds did not).
