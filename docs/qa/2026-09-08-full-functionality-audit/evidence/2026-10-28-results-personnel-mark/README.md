# An injured character wears its damage mark in the results (P24)

Date: 2026-10-28. This follows `../2026-10-27-jump-turn-list-scene-result-marks/`,
which gave the results' ships and squadrons their damage marks. Two pages
were still left without one.

## The original

- On the Personnel page, `FUN_004714e0` passes `local_b0` = 1 to
  `FUN_004724a0`. That makes `FUN_0042c3b0`'s picture argument 0, so the
  page draws the character's mini.
- A damaged object (`+0x50` bit `0x200`) in the personnel range
  (`0x30..0x3f`) takes STRATEGY 11502 (`0x2cee`) over its backdrop 11500.
  A character is damaged while its injury is not 0 (`FUN_004f7800`).
- Regiments take no mark. `GameObj_SetDamaged` has five callers: ships
  (`FUN_00501c00`), squadrons (`FUN_00503340`), fleets (`FUN_004feab0`)
  and characters (`FUN_004f7800`, `FUN_004f1720`). None of them is a
  regiment, so a regiment's bit is never set.

## The port

- The Personnel entries take `fleet_window::damage_mark(Character, ..)`,
  which is STRATEGY 11502, when the character's injury is above 0.
- Test: `an_injured_character_in_the_results_wears_the_damage_mark`. It
  failed before the change.

## Gates

- Browser `--battle --all`: 148/148 passed.
- Workspace: 3008 tests passed. Provenance scan: no uncited items.
- Replay equivalence and the replay manifest passed.
- Mutants: 5 in all, 4 caught and 1 unviable.
