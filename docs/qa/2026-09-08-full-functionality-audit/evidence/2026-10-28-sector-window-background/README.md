# The sector window's background: the galaxy under it, shaded (P06)

Date: 2026-10-28. The reading is in `ghidra/notes/sector-quadrants.md`,
"The sector window's background".

## The original

- The sector window's WM_PAINT (`FUN_004593e0`) works in three steps:
  1. it copies the galaxy view's picture under the window;
  2. it maps every pixel through a palette table (`FUN_005fe050`);
  3. it draws its own art on top.
- `FUN_00459e30` builds the table with `FUN_005fddd0(LAB_004ac550)`. Each
  palette colour's channels become `c / 6 + 50`, and each entry then takes
  the nearest palette entry by squared distance.
- A0, Windows 10 VM: the same pixels with the window closed and open
  (`galaxy-a.png` and `sector-a.png`, hashes in
  `../2026-10-28-reference-rail/`) map one to one, for example
  (36, 37, 71) to (59, 59, 59) and (39, 47, 124) to (54, 56, 67). The
  table computed on STRATEGY 903's palette gives exactly the captured
  colours for all eight colours sampled.

## The port

Before this change, the port filled the window with a flat (42, 42, 42).

Now:
- `bmp_cache.rs`:
  - `sector_shade` and `sector_shade_table` are the transform and the
    nearest-entry table;
  - `bmp_palette` reads an 8-bit BMP's palette;
  - `shade_bitmap` maps every pixel;
  - `BmpCache::get_shaded` caches the result as a texture.
- `sector_window.rs`: the window draws the shaded starfield under itself,
  at the place `draw_galaxy_backdrop` paints the starfield
  (`galaxy_backdrop_destination`). The flat fill stays as the fallback
  when the bitmap is missing.
- `SectorWindowState::set_backdrop` takes the GID mode's starfield (902 or
  903, `gid_backdrop_resource`). `main.rs` sets it each frame before the
  windows draw.
- The GID markers under the window are added in s188 (see the addendum).

## Tests

- `the_sector_shade_divides_each_channel_by_six_and_adds_fifty`
- `the_shade_table_takes_the_nearest_palette_entry`
- `a_shaded_bitmap_maps_each_pixel_through_its_palettes_table`

## Browser

The `icons` case in Arch Chromium 155 (run
`sector-quadrants-2026-10-28T15-42-45-688Z-3030345`). At logical
(70..130, 300..330) the port's sector window has the same colours as the
A0 capture in the same place: (54, 53, 50), (59, 59, 59) and
(54, 56, 67).

## Gates

- Workspace: 3031 tests passed. Provenance scan: no uncited items.
- Mutants (`rebellion-render`, the diff):
  - the first run missed six:
    - `get_shaded` returning `None` and its cache test;
    - `bmp_palette`'s slice end;
    - `set_backdrop`;
    - the texture coordinates' two divisions;
  - after adding a staged-bitmap cache test, a test of the backdrop by GID
    mode and a texture-coordinate test, and after simplifying the palette
    read: 45 mutants, 37 caught, 8 unviable.
- Browser:
  - `sector-quadrants.mjs`: 12 of 12 (run
    `sector-quadrants-2026-10-28T15-47-12-531Z-3115635`);
  - `fleet-window.mjs` `chrome`: 2 of 2;
  - GID `run.mjs --all`: 152 of 152 (run `2026-10-28T15-47-53-391Z-3118497`).

  These ran before the last tests were added; the code they ran is
  unchanged since.
- rustfmt: only the files' baseline differences remain.
- Release WASM SHA-256 `9613704dd55e0691ad0779bee6478c5cff63f013325fe62f59bd0288f6ab1d58`.

## Addendum (s188): the view's markers under the window, shaded

The original's copy is the whole galaxy view, so the GID markers the view
drew under the window show shaded too. The port now does the same:
- `draw_galaxy_map` records each drawn marker and its centre
  (`GalaxyMapState::drawn_gid_markers`);
- `main.rs` hands them to the sector windows
  (`SectorWindowState::set_drawn_gid_markers`);
- each window draws the markers under it from `BmpCache::get_shaded`.
  The markers' palette is STRATEGY 903's, so the table is the same.
- `shade_bitmap` keeps a keyed bitmap's blue matte transparent: the copy
  holds only what the view drew.

That removes the port note above.

Tests:
- `a_shaded_keyed_bitmap_keeps_its_matte_transparent`;
- `the_window_shows_the_views_markers_under_it_shaded`: a marker under
  the window draws at its centre and size, and one outside does not.

Gates:
- 3034 tests;
- provenance OK;
- mutants: 11 tested, 7 caught, 4 unviable;
- `sector-quadrants.mjs` 12/12 (`sector-quadrants-2026-10-28T16-33-33-291Z-3352360`);
- `fleet-window.mjs` `chrome` 2/2;
- GID 152/152 (`2026-10-28T16-34-16-850Z-3355164`);
- release WASM `c3c20396614a5d28afa6100bbc95fe5b2d5e0248a7e549811f6bf6fc51b0dcc9`.
