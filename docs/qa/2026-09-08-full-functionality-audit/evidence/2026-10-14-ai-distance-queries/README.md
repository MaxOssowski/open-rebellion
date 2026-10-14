# The AI's nearest-site queries (F-036)

Date: 2026-10-14.

Module 1 picks a mine or refinery site near its construction yard when
the ranked queries find none (`ghidra/notes/ai-production-modules.md`).
It asks two queries, read from the decompiles:

- **`FUN_00419b80` (nearest sector):** walks the origin object up its
  containers to a sector, then lists every sector record passing the
  masks on `+0x24`, `+0x28`, `+0x2c`. The key is the squared distance
  (`FUN_0041b7b0`: `dx*dx + dy*dy` over packed shorts, 32-bit) between the
  records' map positions `+0x40` (SECTORSD `x_position`/`y_position`).
- **`FUN_0041a400` → `FUN_00431c00` (nearest system):** walks the origin
  up to a system, then lists the given sector's system records passing the
  masks on `+0x28`, `+0x2c`, `+0x30`. The key is the squared distance
  between the records' `+0x34` positions (SYSTEMSD `x_position`/
  `y_position`).

Both lists are sorted by `FUN_005f59f0`, with a coin flip on equal keys
(`FUN_0041c250`). Module 1 asks with order 2, which puts the farthest
first (`FUN_0041c250` returns `order == 1` for a lower key). It then
reads the list through `FUN_004357b0`, which takes the highest place,
that is the last entry, so the result is the nearest site.

## Port

- `SystemRecord::position` and `SectorRecord::position`, set from the
  world's map coordinates as shorts.
- `Galaxy::sectors_by_distance(origin, masks, order, coin)` and
  `Galaxy::systems_by_distance(sector, origin, masks, order, coin)`. The
  origin is a system. With no record for it, the list is empty, as when
  the original's walk finds no container.
- Nothing calls them yet, so the simulation and the save are unchanged.

## Gates

- `cargo test --workspace --no-fail-fast`: 1834 passed, 0 failed. New
  tests:
  - `sectors_rank_by_squared_distance_from_the_origin_s_sector`, which
    also checks that order 2 and `pick` give the nearest;
  - `a_sector_s_systems_rank_by_squared_distance_from_the_origin_system`;
  - `a_squared_distance_wraps_like_the_original_s_imul`.
- Seed-42 golden unchanged (`v1:6685c6f382ede031`); no save or app change.
- `cargo mutants --in-diff`: 30 mutants, 27 caught, 3 unviable, none
  missed.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in `ai_records.rs`. rustfmt: clean.

Hashes are in `SHA256SUMS`.
