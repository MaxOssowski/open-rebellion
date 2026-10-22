# The GID fleet displays mark both sides (P07)

Date: 2026-10-22. The reading is in `ghidra/notes/gid-markers.md`.

## Change

- `gid_fleets` counts the player's and the enemy's fleets for Idle
  Fleets (in orbit) and Fleets Enroute (moving to the system), as
  `FUN_004feea0(system, side, 1 / 2)` does for each side.
- `gid_fleet_mark` follows `FUN_0042b330` cases `0x20..0x22`:
  - both sides give the special art `0x27b0`, STRATEGY 10160
    (`GID_BOTH_SIDES`);
  - otherwise the mark takes the counting side's colour and its count's
    size (3, 2, 2, 1, 1);
  - with no fleet, the holder's smallest mark.
- Before this change the port counted only the player's fleets and drew
  them in the holder's colour.

port: an idle fleet orbits the system; an en route one has a move order
to it. The original's `+0x88` bit 1 test for an empty Fleets Enroute
display is not kept, because an unexplored system shows the unexplored
mark first.

## Gates

- Tests: 2 new. Workspace 2647 passed, 0 failed.
- Mutants: 22 caught, 0 missed after the display-level case was added.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  GID scenarios fleets, fleets-enroute, personnel and popular-support,
  4 of 4 cases each.
- Replay equivalence passed.
- Release WASM `1c3db4bf6c239be2b68ccde0cf220651deca949eb7536e8b0f6c747a9f4a4065`
  (built before the test-only addition).
