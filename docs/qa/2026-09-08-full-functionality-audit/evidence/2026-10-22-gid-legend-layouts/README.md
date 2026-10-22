# The GID legend layouts reach the game (P07, stage A)

Date: 2026-10-22. The reading is in `ghidra/notes/gid-markers.md`.

## Change

- `dat_dumper::types::gid_legend` decodes and encodes the legend layouts
  that `FUN_004522f0` loads through `FUN_00452630`: STRATEGY.DLL
  `RT_RCDATA` 0x27c7..0x27d8 and 0x2d50..0x2d57, one per display (two
  for Popular Support). A layout gives the window's size, then bitmaps
  (kind 1/2: x, y, id) and texts (kind 3: x, y, TEXTSTRA string, font,
  red, green, blue), up to a 0 word.
- `dat-dumper --extract-strings` also writes `gid-legends.json` when
  STRATEGY.DLL is present; the browser build stages and packs it.
- `rebellion_data::gid_legends` gives the layouts with their texts read
  from TEXTSTRA's string table (`rebellion_core::gid_legend`): native
  builds read the player's DLLs, browser builds the extracted table.
- Nothing draws them yet; stage B replaces the port's single compact
  legend (STRATEGY 10168) with these windows.

## Gates

- On the player's STRATEGY.DLL: 22 layouts, each re-encoded to its
  original bytes; 186 texts, every string found in TEXTSTRA.
- Tests: 4 new (decode/round trip, refusal, text resolution, and an
  ignored test on `data/base`). Workspace 2652 passed, 0 failed; the
  `data/base` test passed with `--include-ignored`.
- Mutants: data crate 2 of 3 caught, the miss is the WASM-only
  `set_gid_legends_json`, which the native build does not compile;
  dat-dumper 40 caught, 6 unviable, 5 missed (`main`, and `load`'s
  two, each listed twice). `load`'s two misses (`Ok(empty)`,
  the `RT_RCDATA` test inverted) fail the `data/base` test by hand,
  since cargo-mutants will not run another package's test target for
  it. `main -> Ok(())` is the CLI; its output is checked by the
  extraction above and the WASM build log.
- Runtime pack test: a new case that fails without `gid-legends.json`
  in `TEXT_TABLES`.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build),
  the GID scenario popular-support, 4 of 4 cases: loading still works.
- Replay equivalence passed.
- Release WASM `99db4f08ca1e33aaf35da2713329aa6970a4aa8178985476d325490bf99a4794`.
