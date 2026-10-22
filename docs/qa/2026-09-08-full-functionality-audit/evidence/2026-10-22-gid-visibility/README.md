# What the GID marks reveal (P07)

Date: 2026-10-22. The reading is in `ghidra/notes/gid-markers.md`.

## Reading

Nothing in `FUN_0042b330` asks what the player knows. Its counts walk a
system's children (`FUN_00513090`) for either side, so the fleet and
personnel displays show the enemy's fleets, and the people aboard them,
wherever they are. The original has no fog on these marks. The port must
therefore show what the original shows, not less and not more.

The one gate sits at the top of `FUN_0042b330`:
- an unexplored system (`+0x88` & 2, `System_SetExplored`) shows the
  unexplored mark;
- so does a destroyed one (`+0x50` & 8) outside the fleet and personnel
  displays (`0x20..0x22`, `0x43`, `0x44`);
- Fleets Enroute (`0x22`) and Active Personnel (`0x44`) skip the gate and
  count even an unexplored system.

## Change

- `gid_shows_unexplored` and `gid_counts_unexplored` follow that gate.
- Before this change, the port showed a destroyed system's values on the
  economy, facility and defence displays, and its support on Popular
  Support; the original shows the unexplored mark there.
- Before this change, the port hid an enemy fleet en route to an
  unexplored system, and the busy people aboard one; the original shows
  them.

port: an unpopulated system keeps the unexplored mark. So does an
unexplored system that the two counting displays find empty. The
original's mark for a side without a table (`FUN_00426e70` returns 0) is
not read.

The reference capture 0119 shows the game open on Popular Support with
the legend button and no legend window, as the port does. The notes and
`gid_legend.rs` now say so, in place of the earlier claim that the
original starts with Display Off.

## Gates

- Tests: 1 new (unexplored and destroyed systems across the displays).
  It fails on the old gate (the fleet en route to an unexplored system).
  Workspace 2664 passed, 0 failed.
- Mutants: 25 of 25 caught.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152 cases.
- Replay equivalence passed.
- Release WASM `119af6fcf7ca2da8a9e4975d606e23dddc5fb43d3c55427cbd27e25adc56cd8d`.
