# An injured character's damage mark (UI, P08)

Date: 2026-10-22. The reading is in `ghidra/notes/character-injury.md`
(`FUN_004f7800` sets `+0x50` bit `0x200` to injury != 0) and
`ghidra/notes/fleet-window.md` (`FUN_0042c3b0`: STRATEGY 11502 under a
personnel mini).

## Change

- Fleet window, Personnel tab: an injured character's item is damaged and
  takes 11502 under its mini.
- Defenses window, personnel page: the same (`row_marks`).
- Missions window: a member's marks are a list (`mini_marks`): en route,
  then damaged for an injured character.
- `bmp_cache.rs`: STRATEGY 11501, 11502 and 11515 are keyed with the other
  marks, as the original keys the composed bitmap.

## Gates

- Tests: 2 new, 1 extended (one assertion now checks that no mark is
  drawn at all, which is stricter). Workspace 2614 passed, 0 failed.
- Mutants on the window diff: 14 caught, 0 missed, 4 unviable.
- Browser, Arch Chromium 155.0.8059.84: Fleet window 58 of 58, sector
  quadrants 12 of 12 (Defenses and Missions windows). Interface WASM
  `bf618937a46cf55c93bbaffb28d9c01c572799ee3198d905672b1a6649bb8394`.
- Replay equivalence passed.
- Release WASM `9703bda69c564a703e507d9c8390f2539c28dd4f1c511f4a3f34c3328b1e2807`.
