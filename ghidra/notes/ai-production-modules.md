# The AI's production modules: a first read

Started 2026-10-14. The modules are listed in `ai-mission-planning.md`, "The
fourteen modules", and the records they query are in
`ai-galaxy-records.md`. This note collects each module's state machine as
it is read. Nothing here is ported yet.

## Module 1 (`FUN_004da950`, step `FUN_004dabe0`)

Step `FUN_004dabe0`, from `+0x38` (0, or `0x3ec` within a cycle):

- When it is not the module's turn (`FUN_005f4960() != 1`), clear `+0x38`
  and `+0x3c` and complete.
- `+0x38 == 0x3ec`: run the cycle `FUN_004db2f0`.
- Otherwise:
  - `FUN_004daca0` keeps the previous site. It walks the module's list of
    systems: a system whose record lost `+0x30` bit 1 or its mark
    `0x800000` drops the mark. A system that is calm (`+0x28 & 3 == 0`)
    with an idle construction yard (`0x800`) is taken, storing its yard
    capacity `v[6]` at `+0x54`.
  - `FUN_004daf10` chooses a new site and marks it `0x800000`. It takes
    the sector by queries over sector bit `0x2000` with keys 4 (order 2),
    `0x15` and 8 (order 1). Then it takes the system by `+0x28` bit
    `0x800`, `+0x30` bit 1, forbidding `0x3800000`, with keys 6, 0 and
    `0x33` (order 1). Failing that, it queries sector `+0x2c` bit 4 by key
    `0x14`, and systems by `+0x30` bit 1 with key 6. With no system it sets
    the controller's `+4` bit `0x40`.
  - Either success sets `+0x38 = 0x3ec`.
  - Otherwise, when controller `+0xc == 0x80` and its flags hold `0x40`,
    run `FUN_004dcc40`. Then `FUN_004dd120`, and complete.

Cycle `FUN_004db2f0`, by `+0x3c`:

| State | Action |
|---|---|
| 1 | Clear the controller's `+8` bit `0x80`. If controller `+0xc != 0x80`, go to 10; else `FUN_004199b0`, go to 2. |
| 2 | `FUN_004db510`: the module's system with an own calm record, free room `v[0x33] > 0` and the fewest construction yards; else query a site (sector bit `0x80`, keys 4/`0x15`/8; systems forbidding `0x3800003` and `+0x30` `0x40000000`, keys 6/0/`0x33`) and mark it `0x800000`. None: complete. |
| 3 | `FUN_004db820`: order Scrap `0x200` on the site's unmarked shipyards and training facilities and its `0x25` defenses. With no free energy (`v[15] == 0`) and nothing else to scrap, also mines or refineries, by the raw/refined balance (controller `+0x1d0` vs `+0x1d4`). An order completes the step and raises `+0x34`. |
| 4 | `FUN_004dbd60`: the construction yards wanted (`FUN_004dd050`, divided by a class's figure, `FUN_0041b230(.., 0x2a, 0x10000, 0x4000)`), capped by the free energy `v[15]`, the room `+0x50` and the capacity `+0x54`. |
| 5 | `FUN_004dbe70`: a site with one construction yard too many (`v[6] >= 2`) orders Scrap `0x200` on one; a site with none drops its mark. |
| 6 | Mine or refinery: refinery class `0x2d000002` (`FUN_004dc3f0`) when controller `+0x1d4 < +0x1d0`, else mine `0x2c000001` (`FUN_004dc150`). Failure ends the cycle; success sets `+0x4c = 1` and goes to 7. |
| 7 | `FUN_004dc650`: an order, raising `+0x34`. |
| 8 | `FUN_004dc9f0`: Destination `0x214`, raising `+0x34`, go to 9. |
| 9 | `FUN_004dcb30`: Build `0x210`, end. |
| 10 | `FUN_004dcc40`, end. |

Site choice for state 6:

- **Mine** (`FUN_004dc150`). Sectors by `+0x24` bit `0x100`, keys 7, 10
  and 9 (order 1). Then systems by `+0x28` bit `0x80`, keys `0xe` (free
  raw), 0 (support) and `0xf` (free energy) (order 1). Failing that, the
  sector nearest the yard (`+0x44`) with `+0x24` bit `0x800`
  (`FUN_00419b80`, order 2), then its system nearest with `+0x28` bit
  `0x40` (`FUN_0041a400`, order 2).
- **Refinery** (`FUN_004dc3f0`). Sectors by `+0x24` bit `0x200`, keys 7
  and 9. Then systems by `+0x28` bit `0x100`, `+0x30` bit 1, forbidding
  `0x3e00003`, keys `0xf` and 0. Failing that, the nearest sector with
  `+0x24` bit `0x400`, then its nearest system with `+0x28` bit `0x20`.

Open: `FUN_004dc650`, `FUN_004dc9f0`, `FUN_004dcb30`,
`FUN_004dcc40`, `FUN_004dd050`, `FUN_004dd120`, `FUN_004199b0`,
`FUN_0041b230`; the controller fields `+0xc`, `+0x1d0`, `+0x1d4`. The
Scrap order (`0x200`) that modules 1, 3, 6, 7, `0xa` and `0xb` issue is
ported for facilities and regiments (`scrap-order.md`).
