# Sector window status tracks show only where their refresh shows them (P06)

Date: 2026-10-28. CMD-03's uncommon states: unexplored, uninhabited and
destroyed systems. The reading is in `ghidra/notes/sector-quadrants.md`,
"When the status tracks show".

## The original

The tracks are created hidden (`FUN_00459e30`):
- energy and raw materials show for an explored system (`FUN_0045c240`,
  `FUN_0045c450`, record `+0x88` bit 1);
- popular support shows for a populated system with a side
  (`FUN_0045baf0`);
- a destroyed system shows none.

In A0 (`sector-a.png`, Moddell, hash in
`../2026-10-28-sector-names-and-marks/`) the unexplored planets carry no
track, and Vjun, explored, carries all three.

## The port

Before, the port drew all three tracks for every planet. Now
`track_shown` filters them:
- energy and raw materials need the system explored;
- support also needs it populated;
- a destroyed system shows none.

port: the support track also needs the system explored, because the
viewer's record of an unexplored system holds no side, and the port has no
per-viewer record. The support row is now the constant
`SUPPORT_TRACK_ROW`.

## Tests

`the_tracks_show_only_where_their_refresh_shows_them` covers four cases:
- explored and populated: all three;
- unpopulated: no support;
- unexplored: none;
- destroyed: none.

- `an_unpopulated_planet_draws_its_slots_but_no_support_track` checks the
  drawn window: the energy and raw-material rows are there, the support
  row is not.

## Gates

- Workspace: 3038 tests passed. Provenance scan: no uncited items.
- Mutants (`rebellion-render`, the diff):
  - the drawing filter's `row - planet_y` survived as `+` and `/` until the
    drawn-window test was added;
  - final run: 106 mutants, 106 caught.
- Browser:
  - `sector-quadrants.mjs`: 12 of 12 (run
    `sector-quadrants-2026-10-28T19-21-23-328Z-4095975`);
  - `fleet-window.mjs` `chrome`: 2 of 2;
  - GID: 152 of 152 (run `2026-10-28T19-22-06-060Z-4098879`);
  - `build-selection.mjs`: only its known 6-pixel `opened` failure.
- rustfmt: only the file's baseline differences remain.
- Release WASM SHA-256 `9457f71d89a9efd2277bd94a5e99464507e1e771bb04385062e6a189042616f4`.
