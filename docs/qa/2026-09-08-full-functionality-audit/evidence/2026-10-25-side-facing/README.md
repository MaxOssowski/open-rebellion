# Each side starts facing the other (P24)

Date: 2026-10-25. This closes the open "Victory's exact heading on screen"
of `../2026-10-25-hull-detail-depth/`. The port started every capital ship
facing +z. Its Imperial Victory in the like-for-like battle (scenario 37,
A0 TAC-01-C003) therefore faced away from the Alliance transport.

## The original

- The unit constructor `FUN_005b0f70` (`0x005b0f8e`, `0x005b1024`) sets the
  forward vector `+0x10` to (0, 0, 1).
- `FUN_005ab650` loops over every unit in `+0x9a0` before the lane
  placement. At `0x005ab921`..`0x005ab93a` it copies `FUN_005b5ad0(unit)`
  into `+0x10`..`+0x1c`.
- For a unit with no force object (`+0x174` 0), `FUN_005b5ad0` returns
  `FUN_005a7490(DAT_006bc4a0, side)`. That is the vector at `+0x48`..`+0x54`
  of the side's object (`+0x8 + side × 4`).
- `FUN_005be160` and `FUN_005bf830` build the side objects:
  - side 0, the Alliance (class `0x66c7a0`): (0, 0, +1);
  - side 1, the Empire (class `0x66c778`): (0, 0, -1).
- `FUN_005ab650` puts the Empire's lane at +0.5 × extent and the Alliance's
  at -0.5 × extent (`0x005abb66`). So each side faces the other's lane.

## Against A0

The agent's silhouette fit used the port's camera for scenario 37 (pitch
30, yaw -30, distance 175.1, field 0.2) and the 2551 mesh. It compared the
Victory's shape at each heading with both frames by overlap (IoU):

| Frame | Best heading | IoU at 0° | IoU at 180° |
|---|---|---|---|
| Port, before this change | 0° | 0.87 | 0.28 |
| A0 TAC-01-C003 | 175° | 0.44 | 0.58 |

In A0 the Victory's nose points at the transport, at the lower left: the
projection of -z. The transport fits best at 0° (0.81). The A0 frame was
read in place from the ignored store; only its cell ID is cited.

## The port

- `original_side_vector(is_alliance)` (tactical.rs) gives the side's
  vector.
- `assign_original_world_positions` sets each capital ship's
  `source_forward` to it as it places the ship. The desired forward is
  untouched, as in the original.
- `port:` fighter groups keep no forward vector in the port, so only
  capital ships change.

## Not determined

- `FUN_005b5ad0`'s other branch, for a unit with a force object
  (`+0x174` not 0), was not traced. At placement no unit has a force yet.

## Tests

- `each_side_starts_facing_the_other` (abstract_battle.rs): after
  placement the Alliance ship faces (0, 0, 1) and the Imperial one
  (0, 0, -1). Without the change the Imperial ship faces (0, 0, 1) and the
  test fails.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2821 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | 4 tested: 3 caught, 1 unviable |
| Release WASM SHA-256 | `8809c502fb3827636bf4cfbda5dc0d76b85f9c578ef2321c665a55aefeba59a9` |

The matrix's scenario 37 frame (`production-hull-scale-a0`, 640 x 480,
run `2026-10-25T22-40-09-797Z-100051`) was inspected next to A0
TAC-01-C003, enlarged around the Victory. Both now show the same outline:
the nose at the lower left, toward the transport, and the tower at the
upper right.
