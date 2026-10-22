# The last GID mark assumptions settled (P07)

Date: 2026-10-22. The reading is in `ghidra/notes/gid-markers.md`.

## Reading

After its switch, `FUN_0042b330` rebuilds the item's flags
(`0x42c0e5..0x42c191`):
- 8 when the system has children of type 3;
- one bit by the mark's side, from the jump table at `0x42c39c`: side 0
  gives `0x80`, side 1 `0x10`, side 2 `0x20`, side 3 `0x40`.

`FUN_0042cde0` draws only the unexplored mark (`+0x2c4`, `0x27ae`) for an
item with flag `0x80`.

- Fleets Enroute at an unexplored system with no fleet takes side 0. It
  therefore shows the unexplored mark. The port already did this; it was
  an unconfirmed assumption and is now confirmed.
- Active Personnel counting nobody takes the holder's side, even at an
  unexplored system. It therefore shows the holder's smallest mark. The
  port gave the unexplored mark there; it now follows the original.

## Gates

- Tests: the visibility test now expects the holder's smallest mark. It
  fails on the old code (10158 for 10150). Workspace 2667 passed,
  0 failed.
- Mutants: 2 of 2 caught.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- Replay equivalence passed.
- Release WASM `dc9dcd7ca3f384224681151d312e286ca37a7d925c169ef33cb3df922e1eb443`.
