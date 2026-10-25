# The battle extent's source (P24)

Date: 2026-10-25.

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`),
`FUN_005ab650`:

- it sets the battle extent `+0x8cc` to 100.0 (`0x005ab683`,
  `0x42c80000`);
- as it places each unit it counts the unit's side (`+0x168`), leaving
  out a unit in state 1 (`0x005b69f0`: `+0x170` = 1);
- each time a side's count passes the other's, it subtracts
  `DAT_0066c2a0` = −3.0 from the extent (`0x005abb11`..`0x005abb4b`).

The extent is therefore `100 + 3 × the larger side's count`.

## Change

The port's `ORIGINAL_BATTLE_BASE_EXTENT` (100) and
`ORIGINAL_BATTLE_OBJECT_INCREMENT` (3) were tagged `hyp:` with no recorded
read, and named `FUN_005ec400`. They now cite `FUN_005ab650`; the values
and the formula are unchanged.

- `port:` what puts a unit in state 1 at placement is not traced; the port
  counts every living ship and fighter group with craft.
- Still `hyp:`: the lane constants (0.5, 20, 5). The 0.5 that
  `FUN_005ab650` reads (`DAT_0066c29c`, `0x005aba21`) halves a unit's
  bounding box, not a lane.

## Tests

`the_battle_extent_grows_by_three_a_unit_of_the_larger_side` (new): 100,
121 and 127 for 0/0, 4/7 and 9/2 units.

## Gates

No production code changed, only comments, so the browser matrix, WASM and
replay equivalence were not rerun.

| Gate | Result |
|---|---|
| Workspace tests | 2786 passed, 0 failed |
| Provenance scan | 0 uncited items |
