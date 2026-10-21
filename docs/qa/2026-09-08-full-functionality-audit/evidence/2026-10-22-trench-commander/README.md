# The trench run's commander term (F-033)

Date: 2026-10-22. The reading is in `ghidra/notes/abstract-battle.md`, "The
Death Star in the abstract battle", under `FUN_005afc00`.

## Reading

- `FUN_0040c200` builds each battle person (`FUN_00596340` ->
  `FUN_00596c40`):
  - `+0x50` = combat (slot `+0x1f0`) / 20 (`fidivl DAT_006a80e8`);
  - `+0x58` = the person is the battle's `+0x1c` (Alliance) or `+0x24`
    (Empire). `FUN_005af440` registers that person at `+0x254`.
- `FUN_0040b0e0` fills `+0x1c` with `FUN_00509530`:
  - first `FUN_00509330`: the Commander (`+0x96` = 1) of the system's own
    side standing there;
  - then the side's fleets' Commanders, replacing it only on higher
    combat.
- `FUN_005afc00(0)` = `+0x50` + 1, clamped to 1..9; 1 without a person.

## Change

- `abstract_battle::trench_commander_term` computes this term, and
  `AbstractBattle::at_system` stores it as `trench_commander`. The trench
  run adds it in place of the floor 1.0.
- An Imperial pick at `+0x1c` (the system's Commander) gives 1.0, because
  `FUN_0040c200` flags only an Alliance person against `+0x1c`.
- `hyp:` every character at the battle has a battle person.

## Gates

- Campaigns (24 seeds, 1500 days) against HEAD `a84a8036`: 24 of 24
  identical. No AI fleet carried a Commander at a trench run.
- Tests:
  - 3 new tests (the pick and its clamp, the Imperial-system quirk and its
    tie, the shot adding the term) and an `at_system` assertion;
  - with the change neutralized, 3 fail;
  - workspace 2559 passed, 0 failed (run after the citation fix below).
- Mutants: the first run left 1 survivor, the tie rule; the tie test
  closed it. Final: 17 caught, 2 unviable, 0 missed.
- `provenance-scan`: the first gate run flagged `TRENCH_RUN_COMMANDER_CAP`,
  which shared a doc comment. It now has its own citation: 0 uncited.
- Browser, Arch Chromium 155.0.8059.84:
  - Fleet window 58 of 58;
  - replay equivalence passed;
  - frame timing p99 1.0 / 1.6 ms, 0 frames over 50 ms.
- Release WASM: `d242e4f6861ece2a6a774ec72457e291cde6f8455431e44c912608e2ff4d8d57`.
  It was built before the comment and test edits, which do not change it.
