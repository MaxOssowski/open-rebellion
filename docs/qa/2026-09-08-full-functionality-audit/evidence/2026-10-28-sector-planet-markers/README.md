# Sector window: each planet shows its galaxy marker at the bottom left (P06)

Date: 2026-10-28. The reading is in `ghidra/notes/sector-quadrants.md`,
"The planet's galaxy marker". This closes the item that
`../2026-10-28-sector-names-and-marks/` left unread.

## The original

`FUN_0045bbb0` takes the system's GID marker from the galaxy view's map
(`FUN_0042d860`, the item's `+0x20`). It blits the marker at the bottom
left of the planet's bitmap, which is the 37-row picture plus 4 rows. A
destroyed system shows none.

In the A0 capture `sector-b.png` (Sesswenna, Popular Support), Chandrila and
Yaga Minor carry the neutral cyan diamond there and Averam the Alliance's
red sparkle. In `sector-a.png`, Vjun carries the red sparkle. Hashes are in
`../2026-10-28-sector-names-and-marks/`.

## The port

- `crates/rebellion-render/src/lib.rs`: `draw_galaxy_map` records every
  system's marker in `GalaxyMapState::gid_markers`, also for systems
  outside the view. With the display off the map is empty.
  `draw_gid_marker` now takes the resolved bitmap.
- `crates/rebellion-render/src/sector_window.rs`:
  - `SectorWindowState::set_gid_markers` takes that map;
  - each planet paints its marker keyed at (0, 41 - the marker's height),
    after the uprising mark;
  - a destroyed system paints none.
- `crates/rebellion-app/src/main.rs`: one line hands the map to the sector
  windows before they draw.

## Tests

`a_planet_shows_its_galaxy_marker_at_its_bitmaps_bottom_left` covers three
cases:
- the marker paints at the planet's top left plus 41 rows (the fixture has
  no bitmaps, so the marker's height is 0);
- a destroyed system paints none;
- with no markers (display off), nothing paints.

## Browser

In the `icons` case (Arch Chromium 155, run
`sector-quadrants-2026-10-28T16-09-05-754Z-3232797`), Akrit'tar carries the
Alliance's red sparkle, Ithor a small red mark and Kessel a green one, each
at its planet's bottom left, as in A0.

## Gates

- Workspace: 3032 tests passed. Provenance scan: no uncited items.
- Mutants (`rebellion-render`, the diff):
  - the first run missed two:
    - `draw_gid_marker`, which the new signature brought into the diff
      (macroquad drawing, which the GID browser gate covers);
    - the sign of the marker's offset, which a 0-height marker could not
      tell apart;
  - with a staged 2 by 1 marker in the test: 12 mutants, 9 caught,
    3 unviable.
- Browser:
  - `sector-quadrants.mjs`: 12 of 12 (run
    `sector-quadrants-2026-10-28T16-11-50-083Z-3259585`);
  - `fleet-window.mjs` `chrome`: 2 of 2;
  - GID `run.mjs --all`: 152 of 152 (run `2026-10-28T16-12-32-044Z-3262643`).
- rustfmt: only the file's baseline differences remain.
- Release WASM SHA-256 `a8e123a3caef4081e08c8317c6c39705a2aad8984ef2a22255858159163a1f14`.
