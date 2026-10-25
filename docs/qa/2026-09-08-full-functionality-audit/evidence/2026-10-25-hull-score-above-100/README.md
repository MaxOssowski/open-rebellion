# The hull score above 100 cannot occur (P24)

Date: 2026-10-25. This closes the open item of
`../2026-10-25-subsystem-hits-queue/`: "a hull draw above 100 sets the
hull to 0 and posts a type-3 event; what removes such a ship has not been
traced."

## The branch

`FUN_005b5f50` (tactical hit, slot `+0xe0`) and `FUN_005b54d0` (the
abstract battle's hit) end alike:

- the score is `draw + (hull_after - hull_before) * 100 / hull_before`:
  - `0x005b6107`..`0x005b6129` in `FUN_005b5f50`;
  - `DAT_0066c51c` = 100.0;
  - the draw comes from `0x005a8a70` with 0 and 100 (`0x005b60f8`);
- 50, 65, 80, 90 and 95 (`DAT_0066c520`..`DAT_0066c530`) and 100 pick
  the subsystem hits `0x1e`..`0x22`;
- above 100, `0x005b6235` sets the hull to 0 (`edi` is 0 from
  `0x005b5fad`) and posts a type-3 event. `FUN_005b54d0` has the same
  branch at `0x005b59d3`, with `edi` zeroed at `0x005b57cc`.

## Why it is never taken

- The hit reaches the score only when the shield goes below 0
  (`0x005b6011`..`0x005b6022`).
- The damage is first raised to at least 1.0 (`DAT_0066c514`,
  `0x005b5ff4`..`0x005b6005`). The shield then falls by it, and the
  negative remainder is added to the hull (`0x005b6028`).
- So `hull_after < hull_before`. Both are positive: the hit returns at
  once for a hull at 0 or below (`0x005b5f59`..`0x005b5f8b`).
- Then `(hull_after - hull_before) * 100 / hull_before` is below 0, and
  the score is below the draw, at most 100. The `> 100` test at
  `0x005b6204` never holds.

The question "what removes such a ship" therefore has no case.

For the record, had the branch run:

- `0x005b2b60` destroys a target from a unit source only for a hull
  below 0 (`0x005b2c08`, `DAT_0066c4f0` = 0.0);
- further hits return at once (`0x005b5f59`);
- the strategic record (`0x005b0280`, read by `FUN_0040cfc0` through
  `0x00596870`) reports a hull under 1.0 as 1.0 (`DAT_0066c488`) for any
  state but 11.

## The port

`original_hull_subsystem_kind` returns no hit above 100, and
`original_capital_damage` computes the same score from the same rolls.
This matches the original for every reachable input, so nothing changes.

## Gates

Documentation only. No code changed.
