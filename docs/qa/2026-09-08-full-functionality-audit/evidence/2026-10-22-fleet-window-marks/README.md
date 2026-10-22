# Marks under the minis and portraits; the damage mark (UI, P09)

Date: 2026-10-22. The reading is in `ghidra/notes/fleet-window.md`
("En route marks", corrected) and `ghidra/notes/status-window.md`.

## Finding

`FUN_0042c3b0` blits each mark into the object's bitmap first
(`0x42ca71`..`0x42cc87`: the mark is `ECX`, the bitmap the pushed
destination) and the object keyed over them last (`0x42ccd9`). The marks
show behind the object, as the manual's "starfield behind the portrait"
says (pp. 96-97). The port drew them over the object in the Fleet and
Missions windows. One Fleet window test asserted the old order; its
expectation changes on this evidence.

## Change

- `fleet_window.rs`: `mini_marks` and `damage_mark`. The ship entries,
  the right list and the one-ship picture paint the marks under the
  object. A damaged ship (`status_rows::damaged`, `FUN_00501c00`) takes
  its GOKRES mark: mini `+0x2000`, which is `(class & 0xfff) + 0x6000`,
  or portrait `+0x2000`. Personnel use STRATEGY 11502 (not drawn yet: the
  port keeps no damage flag for them).
- `missions_window.rs`: the en route mark goes under the member's mini.
- `bmp_cache.rs`: GOKRES `0x6640..0x678f` (mini damage marks) are keyed.

## Gates

- Tests: 2 new, 1 changed (order), all pass. Workspace 2612 passed, 0
  failed.
- Mutants on the diff: 9 caught, 0 missed, 5 unviable.
- Browser, Arch Chromium 155.0.8059.84: Fleet window 58 of 58, Status
  window 10 of 10, sector quadrants 12 of 12 (with the Missions window).
  Interface WASM
  `079374cb1689b0e060fdb0eba6d6e7c5c2045555cb952b343299ea1c6bcbc7e2`.
- Replay equivalence passed.
- Release WASM `843719e40478b2f08cf2d2adce578a3ff681ad8ab9504413eec0dcbc0dc51959`.
