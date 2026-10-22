# Unpopulated systems take their holder's GID marks (P07)

Date: 2026-10-22. The reading is in `ghidra/notes/gid-markers.md`.

## Reading

- `FUN_00426ee0` fills the mark tables: side 1 `0x27a2..0x27a5`, side 2
  `0x27a6..0x27a9`, side 3 (no side) `0x27aa..0x27ad`. The unexplored
  mark `0x27ae` (10158) sits apart at the view's `+0x2c4`.
- `FUN_0042cde0` draws only that unexplored mark for an item with flag
  `0x80`, which `FUN_0042b330`'s gate sets for unexplored and destroyed
  systems. It draws the table mark for any other item.
- A holder is never 0 in play. `FUN_0050b610` gives an unpopulated
  system without regiments the no-side holder 3
  (`system-control.md`).

So an explored, unpopulated system shows a no-side mark sized by its
value.

## Change

The port's gate no longer asks for a population. Before this change it
gave every explored, unpopulated system the unexplored mark, on every
display.

hyp: Fleets Enroute at an unexplored system with no fleet still shows
the unexplored mark. There `FUN_00426e70` gets side 0 and returns no
mark, and whether the item keeps flag `0x80` is not read.

## Gates

- Tests: the GID visibility test gains the unpopulated system and fails
  with the old populated check (10158 for 10154). Workspace 2664
  passed, 0 failed.
- Mutants: 6 of 6 caught.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152. The screenshot is the Alliance's
  `uninhabited` case, whose first system is explored and unpopulated.
- Replay equivalence passed.
- Release WASM `d9eea3d2ca1ff20ae919b493a2f5b19fb54d666d38eeadcb73f42738d2f3fd7a`.
