# A jumping fighter group drops out of the fight (P24)

Date: 2026-10-23.

## Source

The fighter weapon path (`FUN_005b3f10`) fires only from a unit in state
2, 4 or 6 (`FUN_005ad870`) that is not in state 6. The target query
`FUN_005cdf50` takes enemy units in state 2 or 4, not 6. A fighter group
whose jump has started (states 6, 7 and 8, `FUN_005c6610`,
`FUN_005b1e40`) therefore neither fires nor is a target. s58 applied the
same rule to capital ships.

## Change

- `TacticalFighter::jumping` is true once the group's jump clock has
  started.
- A jumping group is left out of:
  - fighter targets and strengths, as source and as target;
  - capital-ship fire at fighters;
  - attack-target eligibility, manual targets and the first target.

## Gates

- Workspace: 2723 passed, 0 failed.
- New test: `a_jumping_fighter_group_drops_out_of_the_fight`. Every
  check is true for the group before its jump and false once it jumps.
- Mutants, with both packages' tests: 36 tested, 2 missed.
  - The capital fire at a fighter target: the target refresh and
    eligibility already drop a jumping group, so the fire guard is a
    second layer (as in s58).
  - `squad_count > 0` replaced by `>= 0` in `fighter_target_strength`: a
    live group always has a craft.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `0b821269012b83717ec66fdd0be720ebda7448ed3d035a8f36363cf8c571d8d3`.
