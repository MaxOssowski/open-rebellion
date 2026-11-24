# Bring your own data (browser)

Approved by the user on 2026-11-24 ("pracujmy nad trybem przynieś własne
dane"). Today the browser build loads `web/data/runtime.orpk` (148 MB),
built offline from an original install, so the page carries the game's
files and must stay behind a password. In this mode the player picks the
folder of their own copy (the GOG release) in the browser and the game
builds the same pack from it, locally; the server holds only the engine.

## What the pack holds (ORPK v3, 15,737 entries, sorted by kind and key)

| Kind | Entries | Source | Work |
|---|---|---|---|
| 0 game data | 51 DAT, 2 SPT, 187 `encyclopedia/assets/EDATA.*` | files of the install | copy |
| 0 game data | 9 JSON tables | TEXTSTRA.DLL etc. via `dat-dumper --extract-strings`, the Encyclopedia source via `stage-ui-assets` | port to Rust over our PE reader |
| 1 bitmap | 2,366 | DLL RT_BITMAP resources | DIB + BITMAPFILEHEADER (2,364 found verbatim in the DLLs) |
| 2 audio | 886 | DLL WAVE resources (870 verbatim), MDATA.3xx music | copy |
| 3 advisor frame | 11,752 | TYPE302 and RCDATA resources | verbatim |
| 4/5 tactical mesh/texture | 87 + 397 | TACTICAL.DLL, converted by `stage-ui-assets` tactical3d | port the conversion (~2,100 lines of Go) |
| (not in the pack) | movies | MDATA Smacker files, decoded offline to WebM | later stage |

## Stages

1. A PE resource reader and the pack builder in Rust
   (`rebellion-data::install`), native and WASM, for kinds 0-3: a native
   test builds the pack from the original install and compares every
   entry with the offline pack.
2. The JSON tables (TEXTSTRA strings, messages, advice, GID legends,
   Encyclopedia source) through the same reader.
3. The tactical meshes and textures.
4. The page: pick the folder (`<input webkitdirectory>`, Chromium and
   Firefox), read the files the builder needs, build the pack in the
   WASM app, keep it in IndexedDB for the next visit; a gate drives it.
5. Movies.
