# The minis' backdrops (UI, P08/P09)

Date: 2026-10-22. The reading is in `ghidra/notes/fleet-window.md`
("En route marks").

## Finding

Before any mark, and whatever its last argument, `FUN_0042c3b0(.., 0, ..)`
blits a backdrop into a mini's bitmap (`FUN_005fcc30` at `0x42c6b3` and
`0x42c748`): STRATEGY 11500 for personnel (`0x30..0x3f`) and 11513 for a
regiment, both 61 by 25. The port drew the minis straight on the list.

## Change

- `fleet_window.rs`: `mini_backdrop`; `mini_marks` starts with it. The
  Fleet window's Troops and Personnel tabs draw it.
- `defenses_window.rs`: `row_marks` picks the row's kind (personnel,
  special force, regiment, craft), so the personnel and regiment pages
  draw their backdrops.
- `missions_window.rs`: members draw theirs through `mini_marks`.
- `mission_dialog.rs`: `member_layers` draws the backdrop, and 11502 for
  an injured character, under the agents' and decoys' minis.
- `bmp_cache.rs`: STRATEGY 11500 and 11513 are keyed with the marks.
- Three of tonight's own test expectations gain the backdrop.

## Gates

- Tests pass: workspace 2614, 0 failed.
- Mutants on the window diff: 8 caught, 0 missed, 8 unviable.
- Browser, Arch Chromium 155.0.8059.84: Fleet window 58 of 58, sector
  quadrants 12 of 12, mission dialog pass (both sides; it needed REBEXE
  cursor 1002 staged locally through `extract-dll-resources.py REBEXE.EXE
  --cursors`; the staged file is game data and is not committed).
  Interface WASM
  `8d459cf2f7b53c7a8c69b3ed657cc9c27e970bc4c38618cffb08ffcb528cb8c8`.
- Replay equivalence passed.
- Release WASM `9a576be188233a1fedc65204a4486e7936ce8369c1bcc93e4400648ceb308608`.
