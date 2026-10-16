# The AI's production modules: a first read

Started 2026-10-14. The modules are listed in `ai-mission-planning.md`, "The
fourteen modules", and the records they query are in
`ai-galaxy-records.md`. This note collects each module's state machine as
it is read.

## Module 1 (`FUN_004da950`, step `FUN_004dabe0`)

Read in full 2026-10-14, the uncertain decompiles (`FUN_004db2f0`,
`FUN_004db820`, `FUN_004dbd60`, `FUN_004daca0`) checked in assembly.
Module 1 keeps the side's construction yards ("sites") and builds mines,
refineries and more yards with them. The manual's agent does the same
for the player under Manage Production (p. 87).

### Fields

The module base (`FUN_004e4fb0`) holds:

| Field | Meaning |
|---|---|
| `+0x1c` | ready (1, set by slot `+0x10` `FUN_004cc990`) |
| `+0x20` | complete |
| `+0x24` | a percent, 4 for module 1 |
| `+0x2c` | the controller (the side's AI player state) |
| `+0x30` | the side |
| `+0x34` | the priority |

Module 1 adds:

| Field | Meaning |
|---|---|
| `+0x38` | 0, or `0x3ec` while a cycle runs |
| `+0x3c` | the cycle's state |
| `+0x40` | the target: a sector, then a system |
| `+0x44` | the yard: a site system, then its construction manager (`FUN_00509670(system, 1)`, family `0xa0..0xa1`) |
| `+0x48` | the class to build |
| `+0x4c` | how many |
| `+0x50` | the target's free room `v[0x33]` |
| `+0x54` | the yard's construction yards `v[6]` |
| `+0x58` | the sites, a list of systems marked `0x800000` |
| `+0x60` | the claimed sites, record `+0x30` bit `0x10000000` |

The constructor sets `+0x24 = 4` and clears `+0x38`, `+0x3c`, `+0x4c`,
`+0x50`, `+0x54`.

### Controller fields

The AI player's tallies (phase 4, `FUN_00418500` case 5) feed:

- `+0x1d0`: the side's mines, the sum of its sector records' `+0xa8` over
  `FUN_0041b7e0` (own systems' `v[2]`);
- `+0x1d4`: the side's refineries, `+0xac` (`v[3]`);
- `+0x184`: the side's maintenance capacity (side `+0x58`, `FUN_0041bde0`,
  set at setup by `FUN_004182a0` and later through slot `+0x64`);
- `+0x188`: the side's maintenance requirement (side `+0x70`, slot `+0x68`).

Three words share the construction yards between the modules that use
them (`FUN_004199b0`):

- `+8`: the requests, one bit a module (module 1 asks with `0x80`; module
  2 clears `0x40`, `FUN_004e2400` state 7);
- `+0xc`: the bit that holds the yards now;
- `+0x10`: the next free bit.

`FUN_004199b0` passes the yards on. With no request it hands `+0x10` to
`+0xc` and shifts `+0x10` right, from `0x80` again after 1. With requests
it shifts `+0xc` right (wrapping to `0x80`) until it meets a requesting
bit, and clears that request.

Controller `+4` bit `0x40` means no site is left (`FUN_004daf10`); no code
clears it (searched for every mask of `+4`).

### The step (`FUN_004dabe0`)

1. Not ready (`+0x1c != 1`): clear `+0x38`, `+0x3c`; complete.
2. `+0x38 == 0x3ec`: run the cycle. Any other value: clear `+0x38`.
3. `+0x38 == 0`:
   - `FUN_004daca0` keeps a site. Success: `+0x38 = 0x3ec` (the cycle
     state stays as it was).
   - Else `FUN_004daf10` finds one. Success: `+0x38 = 0x3ec`,
     `+0x3c = 0`.
   - Else, unless controller `+0xc == 0x80` with `+4` bit `0x40` clear,
     run `FUN_004dcc40`. Then `FUN_004dd120`, clear `+0x38`, `+0x3c`, and
     complete.

Neither success completes the module, so the scheduler steps it again at
once (`FUN_00485b40`: a module without `+0x20` stays current).

**`FUN_004daca0`** clears `+0x54` and walks `+0x58`:
- a site with no record leaves the list;
- one that is not the side's (`+0x30` bit 1 clear) or lost its mark
  `0x800000` drops the mark and leaves;
- one with no idle construction yard (`+0x28` bit `0x800` clear) stays,
  unless it is the list's only site and has no construction yard
  (`v[6] == 0`): then it drops the mark and leaves;
- the first calm one (`+0x28 & 3 == 0`) with an idle yard is taken:
  `+0x54 = v[6]`, `+0x44` = the system. The walk stops.

Then `+0x44` becomes the system's construction manager. Success needs a
manager.

**`FUN_004daf10`** runs only with `+0x58` empty, and clears `+0x54`:
- sector: `FUN_00419a00` over sector `+0x24` bit `0x2000`, keys 4
  (order 2), `0x15` and 8 (order 1), picked (`FUN_004357b0`) into `+0x40`;
- system: `FUN_0041a340` over that sector, `+0x28` bit `0x800`, `+0x30`
  bit 1, forbidding `+0x28` `0x3800000`, keys 6, 0, `0x33` (order 1);
- failing that, sector `+0x2c` bit 4, key `0x14`; system `+0x30` bit 1,
  key 6;
- no system: controller `+4 |= 0x40`, fail;
- a system: add it to `+0x58`, set its mark (clearing `0x3800000` first),
  mark the record dirty. Success when it has an idle construction yard
  (`0x800`) and a manager: `+0x54 = v[6]`, `+0x44` = the manager.

### The cycle (`FUN_004db2f0`), by `+0x3c`

| State | Action |
|---|---|
| other | `+0x3c = 1`. |
| 1 | Clear controller `+8` bit `0x80`. Controller `+0xc != 0x80`: go to 10. Else `FUN_004199b0`, go to 2. |
| 2 | `FUN_004db510` chooses the target. None: `+0x38 = 0`, `+0x3c = 0`, complete. Else go to 3. |
| 3 | `FUN_004db820`: a Scrap order or none; with one, complete and `+0x34 += 1`. Go to 4. |
| 4 | `+0x3c` = `FUN_004dbd60` (5, 6 or 8). 0: `+0x38 = 0`, complete. |
| 5 | `FUN_004dbe70`: a Scrap order or none. `+0x3c = 0`, `+0x38 = 0`, complete. |
| 6 | The refinery class `0x2d000002` (`FUN_004dc3f0`) when controller `+0x1d4 < +0x1d0`, else the mine `0x2c000001` (`FUN_004dc150`), into `+0x48`. Success: `+0x4c = 1`, `+0x3c = 7`. Failure: `+0x3c = 0`, `+0x38 = 0`. Either way state 7 runs at once. |
| 7 | `FUN_004dc650`: a Scrap order or none; with one, complete and `+0x34 += 1`. `+0x3c = 8`. |
| 8 | `FUN_004dc9f0`: a Destination order. With one, `+0x3c = 9`, complete, `+0x34 += 1`. Without: `+0x3c = 0`, `+0x38 = 0`, complete. |
| 9 | `FUN_004dcb30`: a Build order. `+0x3c = 0`, `+0x38 = 0`, complete. |
| 10 | `FUN_004dcc40`, then `+0x3c = 0`, `+0x38 = 0`, complete. |

State 6's failure sets `+0x3c = 0` before state 7 sets it to 8, so the
next cycle resumes at state 8 once `FUN_004daca0` keeps a site; `+0x40`
then holds no system and state 8 orders nothing.

**State 2 (`FUN_004db510`)** clears `+0x50` and takes from `+0x58` the
own, calm site with free room `v[0x33] > 0` and the fewest construction
yards (`v[6]` below the best so far, from 1000): `+0x40` = it, `+0x50 =
v[0x33]`. With none it queries a new one: sector `+0x24` bit `0x80`,
keys 4 (order 2), `0x15`, 8; system `+0x30` bit 1, forbidding `+0x28`
`0x3800003` and `+0x30` `0x40000000`, keys 6, 0, `0x33`. A system found
is added to `+0x58` and marked `0x800000`. Success means a target.

**State 3 (`FUN_004db820`)**, at a target system with a record:
- unless it is marked `0x200000`, every shipyard (`0x28`) joins the team;
- unless marked `0x400000`, every training facility (`0x29`);
- every `0x25` defense;
- when the target has no free energy (`v[15] == 0`) and the last list
  added nothing: one refinery (`0x2d`) when controller `+0x1d4 > +0x1d0`
  or the target has no mine (`v[2] == 0`); then, if still nothing, one
  mine (`0x2c`) when `+0x1d0 >= +0x1d4` or it has no refinery (`v[3] ==
  0`).

A team gives a Scrap order `0x200` for the side.

**State 4 (`FUN_004dbd60`)**: `+0x4c = 0`; no target system gives 0.
`w` = `FUN_004dd050`:
- `w > 0`: `FUN_0041b230(&+0x48, 0x2a, 0x10000, 0x4000, 2)` chooses a
  construction yard class into `+0x48` and returns its maintenance `m`;
  `+0x4c = w / m`, capped by `min(+0x50, v[15])` and `+0x54`: 8, or 6
  when that is 0;
- otherwise 6. The code stores 5 for `w < 0`, but then finds `+0x4c == 0`
  and stores 6 over it (`0x4dbe48` falls into `0x4dbe50`), so state 5 is
  never reached.

**`FUN_004dd050`**: the maintenance the side wants in construction yards
less what its sites hold. That is `+0x24 * capacity / 100 - Σ v[36]` over
`+0x58` (`v[36]` = `+0xd8`, the construction yards' maintenance), and 0
when `capacity - requirement` is below it.

**`FUN_0041b230` → `FUN_0042e3c0`**: the side's buildable classes of the
family (`FUN_0052e580`) whose class record has the flag bits (`0x10000`,
a construction yard), ranked by the key the record's slot `+4` gives for
`0x4000` (`FUN_00473060`: `+0x30`, the yard's processing rate, class
`+0x5c`) and picked in the order: order 2 gives the lowest rate, the
fastest yard. It writes the class and returns its maintenance (`+0x20`),
0 with none.

**State 5 (`FUN_004dbe70`)** walks `+0x58`:
- a site with no construction yard (`v[6] < 1`) leaves, dropping its
  mark;
- one with no active yard (`+0x28` bit `0x1000` clear) is passed over;
- one with a single construction yard stops the walk; it leaves (and
  drops its mark) when the list holds more than one site;
- one with two or more is chosen: its first construction yard
  (`0x2a`) is ordered Scrap.

**State 7 (`FUN_004dc650`)**, at an unmarked (`+0x28 & 0x3e00000 == 0`)
target system: every shipyard, training facility and construction yard
joins a Scrap team.

**State 8 (`FUN_004dc9f0`)**: with a target system and a manager at
`+0x44`, Destination `0x214` on the manager, targeting `+0x40`.

**State 9 (`FUN_004dcb30`)**: with a manager and a class, Build `0x210` on
the manager: class `+0x48`, count `+0x4c`.

**State 10 (`FUN_004dcc40`)** clears `+0x40`; `w` = `FUN_004dd050`:
- `w > 0`: walk sectors with `+0x24` bit `0x80` by key `0x15` (picked one
  by one, best first); in a sector with unmarked construction yards
  (`+0xc4 > 0`), walk its systems (`+0x30` bit 1, forbidding `0x3800003`
  and `+0x30` `0x40000000`, key 6) for one with yards (`v[6] > 0`) whose
  yard maintenance fits (`v[36] <= w`). Found: add it to `+0x58` and mark
  it `0x800000`.
- None found, and `+0x58` holds fewer than two sites none of which (the
  first) has a construction yard: when the yard class's maintenance
  (`FUN_0041b230`, order 2) is at most `w`, request the yards (controller
  `+8 |= 0x80`).

**`FUN_004dd120`** rebuilds `+0x60` with one site: from `+0x58`, the site
with the most construction yards that has active regiments (`v[18] > 0`)
and is not claimed by another module (record `+0x30` bit `0x10000000` set
only if already in `+0x60`). The old sites lose `0x10000000`, the new one
gets it.

**A refused order** (`FUN_00485b40` calls slot `+0x18`, `FUN_004dd810`):
complete, `+0x38 = 0`, `+0x3c = 0`, `+0x34 = 0`.

### Site choice for state 6

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

The nearest-site queries are ported (`ai-galaxy-records.md`, "By
distance"); order 2 with `FUN_004357b0`'s pick gives the nearest.

## Module 2 (`FUN_004e1e50`, step `FUN_004e2120`)

Read in full 2026-10-15, the queries' arguments checked in assembly.
Module 2 keeps its own sites (marked `0x1000000`) and builds a mine or a
refinery at a time with them, or more construction yards. Half of its
functions are module 1's with the fields moved: the assembly matches
instruction for instruction except for the offsets, marks and token
(`FUN_004e3a40` = `FUN_004db820` on `+0x48`; `FUN_004e4020` =
`FUN_004dbe70`; `FUN_004e4300` = `FUN_004dc9f0` with the target as an
argument; `FUN_004e4440` = `FUN_004dcb30` without its class check;
`FUN_004e4540` = `FUN_004dcc40`; `FUN_004e4950` = `FUN_004dd050`;
`FUN_004e4a20` = `FUN_004dd120`).

### Fields

The constructor sets `+0x24 = 3` (the percent of the capacity it wants in
construction yards) and clears `+0x38`, `+0x3c`, `+0x54`, `+0x58`,
`+0x5c`.

| Field | Meaning |
|---|---|
| `+0x38` | 0, or the cycle: `0x3ea` mine, `0x3eb` refinery, `0x3ec` yards |
| `+0x3c` | the cycle's state |
| `+0x40` | the mine site |
| `+0x44` | the refinery site |
| `+0x48` | the target |
| `+0x4c` | the yard: a site, then its construction manager |
| `+0x50` | the class to build |
| `+0x54` | how many |
| `+0x58` | the borrowed yard's construction yards |
| `+0x5c` | the target's free room `v[0x33]` |
| `+0x60` | the sites, marked `0x1000000` |
| `+0x68` | the claimed sites (record `+0x30` bit `0x10000000`) |

### The step (`FUN_004e2120`)

1. Not ready: clear `+0x38`, `+0x3c`; complete.
2. `+0x38 == 0`: `FUN_004e21d0` keeps a site. Success: `+0x38 =
   0x3eb` when controller `+0x1d4 <= +0x1d0` (refineries no more than
   mines), else `0x3ea`. Failure: `+0x38 = 0x3ec`. Neither completes.
3. `0x3ea`, `0x3eb`: `FUN_004e25c0`; `0x3ec`: `FUN_004e2400`; any other
   value: clear `+0x38`.

**`FUN_004e21d0`** walks `+0x60` into `+0x4c`:
- a site that is not a system, or has no record, leaves;
- one not the side's (`+0x30` bit 1) or that lost its mark `0x1000000`
  drops the mark (dirty) and leaves;
- the first with construction yards (`v[6] > 0`), an idle one (`+0x28`
  bit `0x800`) and calm (`& 3 == 0`) is taken; others stay.

Then `+0x4c` becomes the side's construction manager there. Success needs
one; otherwise `+0x4c` is cleared.

### The mine and refinery cycle (`FUN_004e25c0`), by `+0x3c`

The site is `+0x44` for `0x3eb`, `+0x40` for `0x3ea`.

| State | Action |
|---|---|
| other | `+0x3c = 1`. |
| 1 | `FUN_004e2760`: 2 when it finds a system over budget, else 3. |
| 2 | `FUN_004e2940`: a Scrap order or none; with one, complete and `+0x34 += 1`. Go to 3. |
| 3 | `FUN_004e2f10` (refinery) or `FUN_004e2c70` (mine): a site; go to 4. None: end. |
| 4 | `FUN_004e31b0(site)`: a Scrap order or none (as state 2). Go to 8. |
| 8 | `FUN_004e4300(site)`: Destination; with one, `+0x3c = 9`, complete, `+0x34 += 1`. None: end. |
| 9 | `+0x50` = `0x2d000002` (refinery) or `0x2c000001` (mine), `+0x54 = 1`, `FUN_004e4440`: Build. End. |

End: `+0x3c = 0`, `+0x38 = 0`, complete.

**`FUN_004e2760`**, only when the capacity `+0x184` is below the
requirement `+0x188`: sectors by `+0x24` bits `0xe0`, keys `0x15`, `0x11`,
`0x13` (order 1), picked into `+0x48`; its systems with `+0x30` bit 1,
forbidding `+0x28` `0x3e00000`, keys 6, 4, 5 (order 1), picked into
`+0x48`. A system there gives 0 (state 2), anything else 1.

**`FUN_004e2940`**, at a system `+0x48` with a record: the side's first
shipyard (`FUN_0052cab0`), else its first training facility
(`FUN_0052c740`), else its first construction yard (`FUN_0052c420`). One
member gives a Scrap order.

**`FUN_004e2c70` (mine)**: keep `+0x40` when it is a calm system with
`+0x28` bit `0x80`. Otherwise the sector nearest the yard `+0x4c` with
`+0x24` bit `0x100` (`FUN_00419b80`, order 2), then its nearest system
with `+0x28` bit `0x80`, forbidding 3 (`FUN_0041a400`, order 2); failing
that, sector bit `0x800` and system bit `0x40`. Success: `+0x40` is a
system. **`FUN_004e2f10` (refinery)** is the same on `+0x44` with bits
`0x100` (kept), sector `0x200` then system `0x100`, and sector `0x400`
then system `0x20`.

**`FUN_004e31b0(site)`**, at a system with a record: every shipyard unless
the site is marked `0x200000`, every training facility unless
`0x400000`, every construction yard unless `0x3800000`. A team gives a
Scrap order.

### The yard cycle (`FUN_004e2400`), by `+0x3c`

| State | Action |
|---|---|
| other | `+0x3c = 1`. |
| 1, 2 | As in the other cycle. |
| 3 | `FUN_004e3530` chooses the target: go to 7. None: end. |
| 4 | `FUN_004e3a40` (module 1's state 3 on `+0x48`): a Scrap order or none. Go to 5. |
| 5 | `+0x3c` = `FUN_004e3f80` (8, 6 or 0); 0 ends. |
| 6 | `FUN_004e4020` (module 1's state 5): a Scrap order or none. End. |
| 7 | Clear controller `+8` bit `0x40`. With `+0xc == 0x40` and `FUN_004e3810`: `FUN_004199b0`, go to 4. Else go to 10. |
| 8 | `FUN_004e4300(+0x48)`: as in the other cycle. |
| 9 | `FUN_004e4440`: Build. End. |
| 10 | `FUN_004e4540` (module 1's state 10, mark `0x1000000`, request `0x40`), then `FUN_004e4a20` (module 1's `FUN_004dd120`). End. |

**`FUN_004e3530`** clears `+0x48` and `+0x5c` and takes from `+0x60` the
own, calm site with room `v[0x33] > 0` and the fewest construction yards
(below 1000): `+0x5c = v[0x33]`. With none: sectors by `+0x24` bit `0x80`,
key `0x15` (order 1) and key 4 (order 2), picked; their systems with
`+0x30` bit 1, forbidding `+0x28` `0x3800003` and `+0x30` `0x40000000`,
key 6, and key `0x33` too when `+0x60` is not empty (order 1), picked.
Success means a system. One with a record joins `+0x60` and gets the
mark `0x1000000` (the other marks are not cleared).

**`FUN_004e3810`** borrows module 1's yard nearest the target: the
sector nearest `+0x48` with `+0x24` bit `0x1000` (a calm system with
module 1's mark and an idle construction yard; `FUN_00419b80`, order 2),
then its system with `+0x28` bits `0x800800`, `+0x30` bit 1, forbidding
`+0x28` 3, key 6 (order 1). With a record and a manager there: `+0x58 =
v[6]`, less one when above 1, and `+0x4c` = the manager. Success then
adds the target to `+0x60` (unless listed) and marks it `0x1000000`.

**`FUN_004e3f80`**: `+0x54 = 0`; `w` = `FUN_004e4950` (3% of the
capacity less the sites' `v[36]`, 0 when unaffordable):
- `w < 0`: 6; `w == 0`: 0;
- `w > 0`: the construction yard class (`FUN_0041b230`, as module 1) into
  `+0x50`; `+0x54 = w / m`. When positive it is capped by `+0x58` and by
  `min(v[15], +0x5c)` of the target's record, and the result is 8 (even
  if the cap makes it 0). Otherwise 0.

So state 6 is reachable here, unlike module 1's state 5. A target found
by query leaves `+0x5c` at 0, so its Build asks for 0 yards.

## Module 3 (`FUN_004d6b20`, step `FUN_004d6d30`)

Read in full 2026-10-15, the queries' arguments taken from the assembly.
Module 3 balances the side's planetary defenses: it scraps a surplus
(record `v[37]` above 0), builds where defenses fall short (`v[37]`
below 0, or Coruscant short of shields), and builds mines, refineries and
construction yards with its own sites (mark `0x2000000`). Like module 2,
many of its functions are module 1's or 2's with the fields moved
(assembly matches but for offsets, marks and token): `FUN_004d7ac0` =
`FUN_004e21d0`; `FUN_004d8fe0` = `FUN_004e3a40` on `+0x58`; `FUN_004d95c0`
= `FUN_004e4020`; `FUN_004d9e70` = `FUN_004e4300`; `FUN_004d9fb0` =
`FUN_004e4440`; `FUN_004da0d0` = `FUN_004e4540` (request bit 1);
`FUN_004da610` = `FUN_004e4a20`; `FUN_004d9b00` = `FUN_004dc650` without
its mark check or record.

### Fields

The constructor sets `+0x24 = 9` and clears `+0x38`, `+0x3c`, `+0x60`,
`+0x64`, `+0x68`.

| Field | Meaning |
|---|---|
| `+0x38` | the cycle: `0x3ed` defense surplus, `0x3ee` defenses and mines, `0x3ec` yards |
| `+0x3c` | the cycle's state |
| `+0x40` | the merged query lists (as module 2's `+0x70`) |
| `+0x50` | the yard: a site, then its construction manager |
| `+0x54` | the site to scrap at or build at |
| `+0x58` | the yard cycle's target |
| `+0x5c` | the class to build |
| `+0x60` | how many |
| `+0x64` | the borrowed yard's construction yards |
| `+0x68` | the target's free room `v[0x33]` |
| `+0x6c` | the sites, marked `0x2000000` |
| `+0x74` | the claimed sites |

Controller `+0x1dc` is the side's defense maintenance: the sector
records' `+0x100` (system `v[31]`) summed by `FUN_0041b7e0`.

### The step (`FUN_004d6d30`)

Not ready: clear `+0x38`, `+0x3c`; complete. `+0x38` `0x3ec`:
`FUN_004d7060`; `0x3ed`: `FUN_004d6da0`; `0x3ee`: `FUN_004d6e70`; any
other value: `+0x38 = 0x3ed`, `+0x3c = 0`. The cycles hand on in turn:
`0x3ed` to `0x3ee` to `0x3ec` to `0x3ed`.

### The surplus cycle (`FUN_004d6da0`)

| State | Action |
|---|---|
| other | `+0x3c = 1`. |
| 1 | `FUN_004d71e0` finds a system: go to 2. None: `+0x38 = 0x3ee`, `+0x3c = 0`. |
| 2 | `FUN_004d7330`: a Scrap order or none. `+0x38 = 0x3ee`, `+0x3c = 0`, complete (no raise). |
| 3, 7 | Unreachable (nothing sets them in this cycle): `FUN_004d76a0`, `FUN_004d8520`, then `0x3ec`. |

**`FUN_004d71e0`**: sectors by `+0x24` bit `0x10000`, key `0x1d` (order
1), picked into `+0x54`; their systems with `+0x30` bit 1, key `0x25`
(order 1), picked. Success: a system whose record has `v[37] > 0`.

**`FUN_004d7330`**, at `+0x54` with a record: up to `v[7] - v[9]` of the
side's `0x24` defenses (`FUN_005278c0`), then up to `v[11] - v[13]`
batteries, the `0x22` ones first (`FUN_00527550`), then `0x23`
(`FUN_005272e0`). A team gives a Scrap order.

### The defense cycle (`FUN_004d6e70`)

| State | Action |
|---|---|
| other | `+0x3c = 4`. |
| 4 | `FUN_004d7ac0` keeps a site (module 2's `FUN_004e21d0`, mark `0x2000000`, `+0x50`): go to 1. None: `+0x38 = 0x3ec`, `+0x3c = 0`. |
| 1 | `FUN_004d76f0` finds a system short of defenses: 6, else 9. |
| 6 | `FUN_004d7cf0`: a Scrap order or none (complete and raise with one). Go to 5. |
| 5 | `+0x3c` = `FUN_004d8460` (`0xb`, 9 or 7). |
| 7 | `FUN_004d8520`: a Scrap order ends the cycle; none goes to 9. |
| 9 | The refinery `0x2d000002` (`FUN_004d99d0`) when `+0x1d4 < +0x1d0`, else the mine `0x2c000001` (`FUN_004d98a0`), into `+0x5c`. Success: `+0x60 = 1`, go to 10. Failure: `+0x38 = 0x3ec`, `+0x3c = 0`. Either way state 10 runs at once. |
| 10 | `FUN_004d9b00`: a Scrap order or none (complete and raise with one). `+0x3c = 0xb`. |
| `0xb` | `FUN_004d9e70(+0x54)`: Destination; with one, `+0x3c = 0xc`, complete, raise. None: end. |
| `0xc` | `FUN_004d9fb0`: Build. End. |

End: `+0x3c = 0`, `+0x38 = 0x3ec`, complete. As in module 1, a failed
state 9 still runs state 10, which overwrites `+0x3c` with `0xb`, and the
next step runs the yard cycle's state `0xb` with `+0x58`.

**`FUN_004d76f0`** puts Coruscant (`0x90000109`) in `+0x54`:
- not the side's, or blockaded (`+0x28` bit 2): cleared;
- `+0x28` bit `0x40000000`, or fewer `0x24` defenses than the side's
  `+0x36c` (`v[7] < 2`): found.

Otherwise:
- sectors by `+0x24` `0x4000` key `0x1c` (order 1), merged with the
  sectors nearest the yard `+0x50` with `0x4000` (`FUN_00419b80`, order
  2), picked; their systems with `+0x28` `0x4000`, `+0x30` 1, forbidding
  `+0x28` `0x4000003`, key `0x25` (order 2), picked;
- no system: Coruscant again, found when it is the side's, not
  blockaded, and `v[37] < 0`;
- still none: the sector nearest the yard with `0x8000`, its systems with
  `0x4000`, `+0x30` 1, forbidding 3, key `0x25` (order 2);
- a system: found when its record has `v[37] < 0`;
- none: when `FUN_004da4d0 > 0` and the side's `+0x374` is below 2,
  `FUN_00418240(4)` raises `+0x374` by one and marks every system record
  dirty.

**`FUN_004d7cf0`**, at `+0x54`: every shipyard unless marked `0x200000`,
training facility unless `0x400000`, construction yard unless
`0x3800000`. With none and no free energy (`v[15] == 0`): one refinery
when `+0x1d0 < +0x1d4` or `v[2] == 0`; else one mine when `+0x1d4 <=
+0x1d0` or `v[3] == 0`; else one shipyard, else one training facility,
else one construction yard. A team gives a Scrap order.

**`FUN_004d8460`**: `+0x60 = 0`; `w` = `FUN_004da4d0`: 90% of `+0x24`
(9) percent of the capacity, less `+0x1dc`, and 0 when `capacity -
requirement` is below it.
- `w < 0`: 7; `w == 0`: 9;
- `w > 0`, with a record at `+0x54` that has free energy (`v[15] > 0`):
  the class by `FUN_0041b230(&+0x5c, 0x2a, flags, key, 1)`: `0x40000`
  by key `0x10000` when `+0x28` has `0x40000000`; `0x20000` by
  `0x10000` when it has `0x8000` (short of shields); else `0x80000` by
  `0x40000`. Key `0x10000` is the class record's `+0x34` (shields),
  `0x40000` its `+0x30` (attack) (`FUN_00473250`); order 1 picks the
  highest. When the class's maintenance is at most `w`, `+0x60 = 1` and
  the result is `0xb`; otherwise 9.

**`FUN_004d8520`** chooses `+0x54`: the sector with `0x10000` by key
`0x1d` and its system by `0x25` (as `FUN_004d71e0`); failing that,
sectors `0x200e0` key 3 and systems with `+0x30` 1, forbidding `+0x28`
`0x3e00000` and `+0x30` `0x40000000`, keys 7 and `0xb`; failing that,
sectors `0x20000` key 3 and systems forbidding only `+0x30`
`0x40000000`, keys 7 and `0xb` (all order 1). At a system: the side's
first `0x22` defense, else its first `0x23`, else its first `0x24`, as a
Scrap order.

**`FUN_004d98a0` (mine)**: the sector nearest the yard `+0x50` with
`0x100`, its nearest system with `+0x28` `0x80`, forbidding `0x3e00003`
(order 2), into `+0x54`. **`FUN_004d99d0` (refinery)**: sector `0x200`,
system `0x100` with `+0x30` 1, forbidding `0x3e00003`. Success: a
system. There is no kept site and no fallback pair.

**`FUN_004d9b00`**: at `+0x54`, every shipyard, training facility and
construction yard of the side, as a Scrap order.

### The yard cycle (`FUN_004d7060`)

| State | Action |
|---|---|
| other | `+0x3c = 1`. |
| 1 | `FUN_004d8ad0` chooses the target `+0x58`: go to 4. None: end. |
| 4 | Clear controller `+8` bit 1. With `+0xc == 1` and `FUN_004d8db0`: `FUN_004199b0`, go to 6. Else go to `0xd`. |
| 6 | `FUN_004d8fe0` (module 1's state 3 on `+0x58`): a Scrap order or none. Go to 5. |
| 5 | `+0x3c` = `FUN_004d9520` (`0xb`, 8 or 0). 0: `+0x38 = 0x3ed`, complete. |
| 8 | `FUN_004d95c0` (module 1's state 5): a Scrap order or none. End. |
| `0xb` | `FUN_004d9e70(+0x58)`: Destination (as above). |
| `0xc` | `FUN_004d9fb0`: Build. End. |
| `0xd` | `FUN_004da0d0` (adopt or request bit 1, `FUN_004da530`), then `FUN_004da610` (claim). End. |

End: `+0x3c = 0`, `+0x38 = 0x3ed`, complete.

**`FUN_004d8ad0`** is module 2's `FUN_004e3530` on `+0x58`, `+0x68` and
`+0x6c`, except that the sector list by key 4 (order 2) joins only when
`+0x6c` is not empty, as the systems' key `0x33` list does.

**`FUN_004d8db0`** is module 2's `FUN_004e3810`, except that the system
is the one nearest the target (`FUN_0041a400` from `+0x58`, `+0x28`
`0x800800`, `+0x30` 1, forbidding 3, order 2) rather than by key 6;
`+0x64` takes the yards.

**`FUN_004d9520`** is module 2's `FUN_004e3f80` on `+0x58`, `+0x64` and
`+0x68` with `w` = `FUN_004da530` (10% of `+0x24` percent of the
capacity, less the sites' `v[36]`), and gives `0xb` and 8 for 8 and 6.

## Module 5 (`FUN_004cfb20`, step `FUN_004cfed0`)

Read in full 2026-10-15, the queries' arguments taken from the assembly.
Module 5 trains special forces at its own training facility sites (mark
`0x400000`), and with a yard it borrows from module 1 builds more
training facilities there. Its yard cycle is module 3's adapted to
training facilities (assembly compared): `FUN_004d0860` =
`FUN_004d8ad0`, `FUN_004d0d30` = `FUN_004d8fe0`, `FUN_004d1270` =
`FUN_004d9520`, `FUN_004d1310` = `FUN_004d95c0`, `FUN_004d1710` =
`FUN_004d9e70`, `FUN_004d1850` = `FUN_004d9fb0`, `FUN_004d1970` ~
`FUN_004da0d0`, `FUN_004d1ed0` = `FUN_004da610`, `FUN_004d1df0` ~
`FUN_004da530`, with the differences below.

### Fields

The constructor sets `+0x24 = 4` and clears `+0x38`, `+0x3c`, the short
`+0x40`, `+0x48..+0x54`.

| Field | Meaning |
|---|---|
| `+0x38` | the cycle: `0x3f4` special forces, `0x3ec` yards |
| `+0x3c` | the cycle's state |
| `+0x40` | the next special force kind, `0x5a..0x5d` (short) |
| `+0x44` | the target's free room `v[0x33]` |
| `+0x48` | how many |
| `+0x4c` | the borrowed yard's construction yards |
| `+0x50` | the special force class flags (`0x1000000..0x8000000`) |
| `+0x54` | the class key (1, the maintenance) |
| `+0x5c` | the yard cycle's target |
| `+0x60` | the yard: a site, then its manager |
| `+0x64` | the class to build |
| `+0x68` | the merged query lists |
| `+0x78` | the sites, marked `0x400000` |
| `+0x80` | the claimed sites |

Controller tallies (phase 4, while controller `+4` has `0x10000000`;
`FUN_0041bc10` over the object records `+0x8c`):
- `+0x2c4`: the summed `+0x64` of the side's own records. `FUN_00402230`
  sets `+0x64` only for non-characters: the class maintenance
  (`FUN_004f2990` -> `FUN_0053b870`). So it is the special forces' upkeep.
- `+0x2dc..+0x2e8`: own non-character records by the kind bit in `+0x34`
  (`0x10000000..0x80000000`, `FUN_00402e80`: `0x3c000001`/`5`,
  `2`/`6`, `3`/`7`, `4`/`8`); `+0x2ec..+0x2f8` the same with `+0x34`
  bit `0x800000` as well. No code sets `+0x34` bit `0x800000`
  (`FUN_00402720` sets bits 0 to 13; a search for writes found none), so
  `+0x2ec..+0x2f8` stay 0.
- `+0x2fc`: the most of the sector records' `+0x140` (`FUN_0041b7e0`).

### The step (`FUN_004cfed0`)

Not ready: return 0 (nothing cleared). `+0x38` `0x3ec`: `FUN_004d0010`;
`0x3f4`: `FUN_004cff20`; any other value: `+0x38 = 0x3f4`. The cycles
alternate.

### The special force cycle (`FUN_004cff20`)

| State | Action |
|---|---|
| other | `+0x3c = 9`. |
| 9 | `FUN_004d01a0` keeps a training site: 1. None: 6. |
| 1 | `FUN_004d0480` picks a kind short of units: 5. None: 6. |
| 5 | `+0x3c` = `FUN_004d0590` (`0xb`, 2 or 0); 0: `+0x38 = 0x3ec`, complete. |
| 6 | `FUN_004d0570`: the special force budget below 0: 2. Else end. |
| 2 | `FUN_004d0610`: a Retire order or none. End. |
| `0xb` | `FUN_004d15f0`: Build `0x212`. End. |

End: `+0x3c = 0`, `+0x38 = 0x3ec`, complete.

**`FUN_004d01a0`** is module 2's `FUN_004e21d0` on `+0x78` and `+0x60`
with mark `0x400000`, the training facilities `v[5]` and the idle
training bit `+0x28` `0x400`, taking the manager of kind 2 (troops,
`FUN_00509670(system, 2)`). With no site it tries controller `+0x14`
under the same conditions; that field's writer was not found.

**`FUN_004d0480`** starts `+0x40` at `0x5a` and walks the four kinds once
from there: for kind `k` it sets `+0x48` to 1 when controller `+0x184 +
4k` (`+0x2ec..+0x2f8`) is below `max(+0x2fc, 8)`, and moves `+0x40` on
(`0x5a` -> `0x5b` -> `0x5c` -> `0x5d` -> `0x5a`) with `+0x50` =
`0x1000000`, `0x2000000`, `0x4000000`, `0x8000000` for the kind just
tested and `+0x54 = 1`. It stops at the first kind with `+0x48 > 0`.
Since `+0x2ec..+0x2f8` stay 0, the first kind tested always passes.

**`FUN_004d1d90`**: `+0x24` (4) percent of the capacity, 70% of that,
less `+0x2c4`; 0 when `capacity - requirement` is below it.
`FUN_004d0570` is that budget `< 0`.

**`FUN_004d0590`**: `w` = `FUN_004d1d90`: `w < 0` gives 2, `w == 0` 0.
Otherwise the class from the training facility's list
(`FUN_0041b230(&+0x64, 0x29, +0x50, +0x54, 1)`: the kind's flag, by
maintenance, order 1: the dearest); when `m * +0x48 > w`, `+0x48 = w /
m`. With `+0x48 > 0` it clears controller `+0x18` and gives `0xb`;
otherwise 0.

**`FUN_004d0610`** queries the object records (`FUN_00403460`) for
flags `0x802000`, none of `0xa0`, by `+0x64` (index 9) from 0 to 1000,
order 1, then for `0x2000`; a special force found (`0x3c..0x3f`) gets a
Retire order `0x242`. No code sets record flag `0x2000`, and
`FUN_00402230` clears it at every refresh (mask `0x7080440e`), so no
Retire is ever given.

**`FUN_004d15f0`**: with a troop manager (`0xa4..0xa5`) at `+0x60`, Build
`0x212` of `+0x64`, count `+0x48`.

### The yard cycle (`FUN_004d0010`)

| State | Action |
|---|---|
| other | `+0x3c = 3`. |
| 3 | `FUN_004d0860` chooses the target `+0x5c`: 9. None: 7. |
| 9 | Clear controller `+8` bit 4. With `+0xc == 4` and `FUN_004d0b40`: `FUN_004199b0`, go to 4. Else `0xc`. |
| 4 | `FUN_004d0d30`: a Scrap order or none (complete and raise with one). Go to 5. |
| 5 | `+0x3c` = `FUN_004d1270` (10, 8 or 0); 0: `+0x38 = 0x3f4`, complete. |
| 7 | `FUN_004d0d10`: the training budget below 0: 8. Else `+0x38 = 0x3f4`, `+0x3c = 0`. |
| 8 | `FUN_004d1310`: a Scrap order or none. End. |
| 10 | `FUN_004d1710(+0x5c)`: Destination; with one, `0xb`, complete, raise. None: end. |
| `0xb` | `FUN_004d1850`: Build. End. |
| `0xc` | `FUN_004d1970` (adopt or request bit 4), then `FUN_004d1ed0` (claim). End. |

End: `+0x3c = 0`, `+0x38 = 0x3f4`, complete.

Differences from module 3's yard cycle:
- `FUN_004d0860`: sectors by `+0x24` `0x40`, key `0x13` (and key 4 with
  sites); systems forbidding `+0x28` `0x400003`, key 5 (and `0x33` with
  sites); the walk compares training facilities `v[5]`; mark `0x400000`.
- `FUN_004d0b40`: no distance: the sector with `0x1000` by key `0x14`
  (order 1), its system with `0x800800`, `+0x30` 1, forbidding 3, by key
  6 (order 1); `+0x4c` = `v[6]`, less one when above 1. The target is
  not added to the sites.
- `FUN_004d0d30`: shipyards unless `0x200000`, construction yards (not
  training facilities) unless `0x3800000`, then as module 1.
- `FUN_004d1270`: the training facility class (`0x8000`, key `0x4000`,
  order 2); 10 and 8 for 8 and 6.
- `FUN_004d1df0`: 30% of 4% of the capacity, less the sites' `v[35]`.
- `FUN_004d1310`: training facilities `v[5]` and family `0x29`, mark
  `0x400000`.
- `FUN_004d1970`: runs only while `+0x78` holds fewer than two sites;
  sectors by `0x40`, key `0x13`, sector `+0xbc`; systems forbidding
  `0x400003`, key 5, `v[5] > 0`, `v[35] <= w`; mark `0x400000`; the
  request is bit 4 with the training facility class.
- `FUN_004d1ed0`: the most training facilities `v[5]`.

## Module 6 (`FUN_004dd460`, step `FUN_004dd7a0`)

Read 2026-10-15, the queries' arguments taken from the assembly. Module 6
balances the side's regiments. It moves them from systems with a surplus
to systems short of garrison, trains regiments at training facility
sites and sends them on, and builds training facilities. It shares
module 5's sites: the same mark `0x400000` on its own list `+0x7c`.

The constructor sets `+0x24 = 0x16` (22), `+0x78 = 1`, and clears
`+0x38`, `+0x3c`, `+0x60..+0x74`.

| Field | Meaning |
|---|---|
| `+0x38` | the cycle: `0x3ef` moves, `0x3f0` regiments, `0x3ec` yards |
| `+0x50` | the source system, or the troop manager |
| `+0x54` | the destination system |
| `+0x58` | the yard cycle's target |
| `+0x5c` | the class to build |
| `+0x60` | how many |
| `+0x64` | the site's training facilities `v[5]` |
| `+0x68` | the target's free room `v[0x33]` |
| `+0x6c`, `+0x70`, `+0x74` | the class choice's flags, key and order |
| `+0x78` | the destination query to start from (1 to 4) |
| `+0x7c` | the sites, marked `0x400000` |
| `+0x84` | the claimed sites |
| `+0x28` | bits 1, 2, 4, 8: a destination query found no system |

### The step (`FUN_004dd7a0`)

Not ready: clear `+0x38`, `+0x3c`; complete. `0x3ec`: `FUN_004dda00`;
`0x3ef`: `FUN_004dd840`; `0x3f0`: `FUN_004dd8d0`; any other value: `+0x38
= 0x3ef`, `+0x3c = 0`. The cycles run in turn: `0x3ef`, `0x3f0`, `0x3ec`.

### The move cycle (`FUN_004dd840`)

States 1 to 4 try `FUN_004ddb70`, `FUN_004de130`, `FUN_004de6f0` and
`FUN_004deba0` in turn; the first that gives an order ends the cycle
(`+0x38 = 0x3f0`, complete), as does state 4 either way. Each builds a
Move `0x201` of regiments to a destination `+0x54`:
- the destination: a system short of garrison, by the sector query
  (`FUN_00419a00` sector `0x40010` or `0x40000`, key `0x1e`, order 2, then
  systems with `+0x28` `0x2001` or `0x2000`, forbidding 2, by key `0x15`)
  or, in the first two, by `FUN_00419e90` over the sector planners
  (sector `+0x2c` `0x2000000`) and the systems with `+0x30` `0x20001` or
  `0x10001`, forbidding 2, by key `0x10`;
- how many: the destination's `v[21]` (`+0x9c`), or in the first two the
  count the sector's fleet planner asks for (`FUN_0042f310` finds the
  planner of module 8 whose `+0x40` is the system, `FUN_00476b70` asks
  it); at most the source's `v[19]` (`+0x94`);
- the source: the sector nearest the destination with `+0x24` `0x80000`
  (and in the first and last `+0x2c` `0x1000`, or failing that sector
  `0x280000`/`0x180000`), its system with `+0x28` `0x10000` or `0x18000`,
  forbidding 2 and `+0x30` `0x30000`, by key `0x13` (order 2);
- the team: one at a time, the source's regiment (`FUN_00504c40`) with
  the lowest slot `+0x1e0` not yet taken; the first two take only classes
  with record flag `0x2000` (defensive).

The first two run only while controller `+4` has `0x20`, which
`FUN_0041b7e0` sets for a sector record with `+0x2c` `0x2000000`.

### The regiment cycle (`FUN_004dd8d0`)

| State | Action |
|---|---|
| other | `+0x3c = 5`. |
| 5 | `FUN_004df160` keeps a training site (module 5's `FUN_004d01a0` on `+0x7c` and `+0x50`, `+0x64 = v[5]`; fallback controller `+0x18`): 6. None: `0xb`. |
| 6 | `FUN_004df440` chooses the destination: 7. None: `0xb`. |
| 7 | `+0x3c` = `FUN_004df9a0` (`0xc`, 8 or 0); 0: `+0x38 = 0x3ec`, complete. |
| 8 | `FUN_004dfa80`: a Move or none. End. |
| `0xb` | `FUN_004dfa70` always gives 0: `+0x38 = 0x3ec`, `+0x3c = 0`. |
| `0xc` | `FUN_004e0a30(+0x54)`: Destination `0x214` on the troop manager `+0x50` (`0xa4..0xa5`); with one, `0xd`, complete, raise. None: end. |
| `0xd` | `FUN_004e0b70`: Build `0x212` on `+0x50`, class `+0x5c`, count `+0x60`. End. |

End: `+0x3c = 0`, `+0x38 = 0x3ec`, complete. State 8 is unreachable
(`FUN_004dfa70` gives 0).

**`FUN_004df440`** clears `+0x60`, `+0x6c..+0x74` and `+0x28` bits
`0xf`, then runs the destination queries in an order set by `+0x78`
(1: `e1060`, `e0ef0`, `e1430`, `e12e0`, `e11a0`; 2: `e0ef0`, `e1430`,
`e1430`, `e12e0`, `e11a0`, `e1060`; 3: `e12e0`, `e11a0`, `e1060`,
`e0ef0`, `e1430`; 4: `e11a0`, `e1060`, `e0ef0`, `e1430`, `e12e0`), each
only while `+0x54` holds no system, and moves `+0x78` on (4 back to 1).
Success: `+0x54` is a system with a record. When all four query bits
are set, controller `+4` has `0x800000` and `FUN_004e19c0 > 0`, it
raises side `+0x384` (below 12), `+0x380` (below 6) and `+0x37c` (below
4) by one (`FUN_00418240(8, 7, 6)`).

The queries, each from the troop manager `+0x50` (order 2 by distance
where named; systems by key `0x15`, order 2) and setting `+0x60` from the
system's `v[21]`:
- `FUN_004e1060`: nearest sector `0x800000`; system `+0x28` `0x2000`,
  `+0x30` 1, forbidding `0x4000003`; flags `0x2000`, key `0x2000`, order
  1. None: `+0x28 |= 1`.
- `FUN_004e0ef0`: nearest sector `0x400000`; system `0x20000`, forbidding
  3; when the system has `0x83e00000`, as `e1060`, else flags `0x2000`,
  key 1, order 2. None: `+0x28 |= 8`.
- `FUN_004e1430`: nearest sector `0x40000`; system `0x2000`, forbidding 3;
  flags `0x2000`, key 1, order 2. None: `+0x28 |= 4`.
- `FUN_004e11a0`: sector `0x40000000` by key `0x1e`; system `0x2000`,
  forbidding 3; as `e1060`. None: `+0x28 |= 2`.
- `FUN_004e12e0`: the system that holds the troop manager; `+0x60 =
  +0x64`; flags `0x1000`, key `0x1000`, order 1.

**`FUN_004df9a0`**: `w` = `FUN_004e19c0` (80% of 22% of the capacity,
less controller `+0x250` and `+0x1d8`); `w < 0` gives 8, `w == 0` 0. Else
the class from the training facility's list (`FUN_0041b230(&+0x5c, 0x29,
+0x6c, +0x70, +0x74)`); when `m * +0x60 > w`, `+0x60 = w / m`; capped by
`+0x64`; with `+0x60 > 0` it clears controller `+0x14` (module 5's
fallback site, as module 5 clears `+0x18`, this module's) and gives
`0xc`. Then, when controller `+4` has `0x400000`
and `w < 1`, it lowers `+0x384` (above 9), `+0x380` (above 3) and
`+0x37c` (above 0) by one (`FUN_00418270`).

Controller `+4` bits `0x400000` and `0x800000` are the strategy module's
(`0xd`, around `0x4bf6ab` and `0x4bf6ca`).

### The yard cycle (`FUN_004dda00`)

Module 5's yard cycle (training facilities, mark `0x400000`) with these
differences: the token is `0x20`; the borrow (`FUN_004dffd0`) takes the
sector nearest the target with `0x1000` and its system with `0x800800` by
key 6 (order 1), and adds no site; the budget (`FUN_004e1a20`) is 20% of
22% of the capacity less the sites' `v[35]`; the plan (`FUN_004e06b0`)
gives `0xc`, `0xa` or 0; a choice with no target ends the cycle (no
budget check); the adoption (`FUN_004e15b0`) is not limited to fewer
than two sites.

| State | Action |
|---|---|
| other | 6 |
| 6 | `FUN_004dfcf0` (module 5's `FUN_004d0860`) chooses the target `+0x58`: 5. None: end. |
| 5 | Clear controller `+8` bit `0x20`; with `+0xc == 0x20` and `FUN_004dffd0`: `FUN_004199b0`, 9. Else `0xe`. |
| 9 | `FUN_004e0170` (module 5's `FUN_004d0d30`): a Scrap or none. 7. |
| 7 | `+0x3c` = `FUN_004e06b0`; 0: `+0x38 = 0x3ef`, complete. |
| 10 | `FUN_004e0750` (module 5's `FUN_004d1310`): a Scrap or none. End. |
| `0xc` | `FUN_004e0c90(+0x58)`: Destination; `0xd`. |
| `0xd` | `FUN_004e0dd0`: Build. End. |
| `0xe` | `FUN_004e15b0` (adopt or request `0x20`), `FUN_004e1b00` (claim by `v[5]`). End. |

End: `+0x3c = 0`, `+0x38 = 0x3ef`, complete.

### Open for the port

- The sector fleet planners of module 8 (`FUN_0042f310`, `FUN_00476b70`).
- Controller `+0x250` (from `FUN_0041ba80` over the fleet records),
  `+0x18`, and the strategy's `+4` bits.
- The regiment record's slot `+0x1e0` and its keys `0x1000`, `0x2000`.

## Module 7 (`FUN_004d2ef0`, step `FUN_004d3100`)

Read 2026-10-15, the queries' arguments taken from the assembly and the
shared functions compared instruction by instruction. Module 7 balances
the side's fighter squadrons on planets: it sends a planet's surplus to a
fleet with hangar room or a system short of squadrons, or scraps it; builds
squadrons at its shipyard sites for systems short of them; and builds
shipyards. Its sites carry the mark `0x200000`.

The constructor sets `+0x24 = 0x11` (17) and clears `+0x38`, `+0x3c`,
`+0x60..+0x68`.

| Field | Meaning |
|---|---|
| `+0x38` | the cycle: `0x3f1` surplus, `0x3f2` squadrons, `0x3ec` yards |
| `+0x50` | the source system, or the site, then its manager |
| `+0x54` | where squadrons go: a fleet or a system |
| `+0x58` | the yard cycle's target |
| `+0x5c` | the class to build |
| `+0x60` | how many |
| `+0x64` | the site's shipyards `v[4]`, or the borrowed yard's construction yards |
| `+0x68` | the target's free room `v[0x33]` |
| `+0x6c` | the sites, marked `0x200000` |
| `+0x74` | the claimed sites |

### The step (`FUN_004d3100`)

Not ready: clear `+0x38`, `+0x3c`; complete. `0x3ec`: `FUN_004d3350`;
`0x3f1`: `FUN_004d3170`; `0x3f2`: `FUN_004d3220`; any other value: `+0x38
= 0x3f1`, `+0x3c = 0`. The cycles run in turn: `0x3f1`, `0x3f2`, `0x3ec`.

### The surplus cycle (`FUN_004d3170`)

| State | Action |
|---|---|
| other | 1 |
| 1 | `FUN_004d34c0` finds the source: 2. None: `+0x38 = 0x3f2`, `+0x3c = 0`, not complete. |
| 2 | `FUN_004d3610` chooses the destination: 3. None: 4. |
| 3 | `FUN_004d3a90`: a Move ends the cycle; none: 4. |
| 4 | `FUN_004d3db0`: a Scrap or none. End. |

End: `+0x3c = 0`, `+0x38 = 0x3f2`, complete.

- `FUN_004d34c0`: sectors with `+0x24` `0x2000000` (a squadron surplus)
  by key `0x22`, order 1; their systems with `+0x30` 1, forbidding
  `+0x28` 2 and `+0x30` `0x40000`, by key `0x35`, order 1, into `+0x50`.
  Found when `v[53] > 0`.
- `FUN_004d3610`, into `+0x54`: the fleet records with `0x100009`,
  forbidding 2, by distance from the source's sector (`FUN_0041a600`,
  order 2) merged with the same by key `0x17` (hangar room, `FUN_0041a4c0`,
  order 1); failing that the same with `0x100005`; failing that the sector
  nearest the source with `0x10000000` and its system with `+0x28`
  `0x140000`, `+0x30` 1, forbidding 2, by key `0x26` (order 2); failing
  that module 8's sector planners (`FUN_00419e90`, `FUN_00476f90`). Fleet
  record bit 8 has no writer found; 4 is the constructor's; `0x100000` is
  hangar room left (`v[23] > 0`).
- `FUN_004d3a90`: how many: a system's `-v[38]`, a fleet's hangar room
  (record `+0xac`), at most the source's `v[53]`. The loop takes the
  squadron with the lowest slot `+0x1f4` (`FUN_005031e0`: slot `+0x244`,
  the fighters left, times `FUN_005574f0`) among the source's squadrons
  (`FUN_005039d0` mode 1) on each pass, without leaving out the one taken
  (no `FUN_004f5940` check, unlike module 6), so the team holds one
  squadron. A team gives Move `0x201` to `+0x54`.
- `FUN_004d3db0`: the same team, while the source's `v[38] > 0`, as Scrap
  `0x200`.

### The squadron cycle (`FUN_004d3220`)

| State | Action |
|---|---|
| other | 5 |
| 5 | `FUN_004d3ff0` keeps a site: 2. None: 10. |
| 2 | `FUN_004d4370` chooses the system: 6. None: 10. |
| 6 | `+0x3c` = `FUN_004d46e0` (`0xb`, 7 or 0); 0: `+0x38 = 0x3ec`, complete. |
| 10 | `FUN_004d47b0`: over budget: 7. Else `+0x38 = 0x3ec`, `+0x3c = 0`, not complete. |
| 7 | `FUN_004d4800`: a Move or Scrap. End. |
| `0xb` | `FUN_004d5dd0(+0x54)`: Destination on the shipyard manager (`0xa2..0xa4`); with one, `0xc`, complete, raise. None: end. |
| `0xc` | `FUN_004d5f10`: Build `0x211`, class `+0x5c`, count `+0x60`. End. |

End: `+0x3c = 0`, `+0x38 = 0x3ec`, complete.

- `FUN_004d3ff0` is module 2's `FUN_004e21d0` on `+0x6c` with the mark
  `0x200000`, the idle bit `0x200` and `v[4]` into `+0x64`, falling back
  on controller `+0x20`, then `+0x24`; the manager is the shipyard's
  (`FUN_00509670(0)`).
- `FUN_004d4370`, from the site: the sector nearest it with `0x4000000`
  and its system with `+0x28` `0x40000`, `+0x30` 1, forbidding
  `0x4000003`, by key `0x26` (order 2); failing that sector `0x40000000`,
  then `0x1000000`, with systems forbidding 3. Found when `v[38] < 0`.
  With no system and `FUN_004d66a0 > 0` it raises side `+0x388`, `+0x38c`
  and `+0x390` by one (`FUN_00418240(9, 10, 11)`, no limit) and marks the
  records dirty.
- `FUN_004d46e0`: `w` = `FUN_004d66a0`; `w < 0` gives 7, `w == 0` 0. Else
  `+0x60 = -v[38]` and the class by `FUN_0041b230(&+0x5c, 0x28, 0x200000,
  key, order)`: key 1 (maintenance), order 2 for a system with no
  `0x83e00000`; key `0x400` (`+0x34`, `FUN_00473640`), order 1 otherwise.
  When `m * count > w`, `count = w / m`; capped by `+0x64`; `0xb`.
- `FUN_004d66a0`: 80% of `+0x24` percent of the capacity, less controller
  `+0x1e0` (the sector records' `+0x104`, own systems' `v[32]`); 0 when
  the free maintenance falls short. `FUN_004d47b0`: 90% of it, less
  `+0x1e0`, below 0.
- `FUN_004d4800`: the source (`+0x50`): sectors `0x2000000` by key `0x22`
  and systems `+0x30` 1, forbidding `+0x30` `0x40000`, by key `0x35`
  (order 1); failing that sectors `0x8000000`, forbidding `0x40000000`, by
  key `0x19`, systems `+0x28` `0x80000`, forbidding `0x80000000` and
  `+0x30` `0x40000`, by key `0x17` (order 2); failing that the same
  without the forbidden bits. The weakest squadron as in `FUN_004d3a90`;
  the destination as in `FUN_004d3610` but for the short system; a fleet
  gives Move `0x201`, else Scrap `0x200`. No source gives no order.

### The yard cycle (`FUN_004d3350`)

Module 6's yard cycle (`FUN_004dda00`) for shipyards, renumbered: the
token is 8; the borrow is module 2's (`FUN_004d52e0` = `FUN_004e3810`,
clearing `+0x64` first, adding the target as a site with `0x200000`); the
plan gives `0xb`, 9 or 0.

| State | Action |
|---|---|
| other | 2 |
| 2 | `FUN_004d5000` (`FUN_004dfcf0`: sector `0x20` key `0x11`, systems forbidding `0x200003` key 4) chooses the target `+0x58`: 5. None: end. |
| 5 | Clear controller `+8` bit 8; with `+0xc == 8` and `FUN_004d52e0`: `FUN_004199b0`, 8. Else `0xd`. |
| 8 | `FUN_004d5510`: a Scrap or none (complete and raise with one). 6. |
| 6 | `+0x3c` = `FUN_004d5a50` (`0xb`, 9 or 0); 0: `+0x38 = 0x3f1`, complete. |
| 9 | `FUN_004d5af0`: a Scrap or none. End. |
| `0xb` | `FUN_004d6030(+0x58)`: Destination; `0xc`. |
| `0xc` | `FUN_004d6170`: Build. End. |
| `0xd` | `FUN_004d6290` (adopt or request bit 8; sector `+0xb4`, upkeep `v[34]`), `FUN_004d67e0` (claim). End. |

End: `+0x3c = 0`, `+0x38 = 0x3f1`, complete.

- `FUN_004d5510`: training facilities unless `0x400000`, construction
  yards unless `0x3800000`, Death Star shields, then module 1's refinery
  or mine; never shipyards.
- `FUN_004d5a50`: the shipyard class (`0x4000`); `FUN_004d6700`: 20% of
  `+0x24` percent of the capacity, less the sites' `v[34]`.

### Module 7 ported (2026-10-15)

`crates/rebellion-core/src/ai_module7.rs`. Save v47.
- The squadron Move is `AIAction::MoveSquadrons`; across systems a
  squadron travels on its own (`squadrons::SquadronTransits`, in
  `TroopTransportState`), at its FIGHTSD speed (`FUN_00502f80`).
- A squadron on a planet takes Scrap (`ScrapTarget::PlanetFighter`).
- Side `+0x388..+0x390` are `Controller::squadron_goals`, read by the
  system records.
- The shared yard steps take `YardKind::SHIPYARD`; `scrap_at` spares two
  families per kind.
- The heuristic's squadron loading is removed: the surplus cycle sends a
  planet's squadrons onto fleets.

port: fleet record bit 8 is never set, so the first fleet query finds
none; module 8's planners are not ported; the fallbacks controller `+0x20`
and `+0x24` have no counterpart; every squadron is at full strength, so
the weakest is the smallest class.

## Module `0xb` (`FUN_004c8820`, step `FUN_004c8b30`)

Read 2026-10-15, the queries' arguments taken from the assembly and the
shared functions compared instruction by instruction. Module `0xb` keeps
the side's capital ships: it sends damaged ships to a shipyard site and
the Death Star home, builds capital ships at its shipyard sites, scraps
one when their upkeep is over budget, and builds shipyards. Its sites
carry the mark `0x200000`, as module 7's do.

The constructor sets `+0x24 = 0x1e` (30) and clears `+0x38`, `+0x3c` and
`+0x60..+0x74`.

| Field | Meaning |
|---|---|
| `+0x28` | the roles tried: bits `0x1000000..0x40000000` (module base word) |
| `+0x38` | the cycle: `0x3f8` fleets, `0x3f9` ships, `0x3ec` yards |
| `+0x50` | the site, then its shipyard manager; in the yard cycle the borrowed construction yard's manager |
| `+0x54` | the fleet the fleet cycle works on |
| `+0x58` | the yard cycle's target |
| `+0x5c` | the class to build |
| `+0x60`, `+0x64`, `+0x68` | the role: class flags, class key, its tried bit |
| `+0x6c` | how many |
| `+0x70` | the borrowed yard's construction yards (`v[6]`, less one above 1) |
| `+0x74` | the target's free room `v[0x33]` (`FUN_004cacc0`) |
| `+0x78` | the sites, marked `0x200000` |
| `+0x80` | the claimed sites |

### The step (`FUN_004c8b30`)

Not ready: clear `+0x38`, `+0x3c`; complete. `0x3ec`: `FUN_004c8d20`;
`0x3f8`: `FUN_004c8ba0`; `0x3f9`: `FUN_004c8c20`; any other value: `+0x38
= 0x3f8`, `+0x3c = 0`. The cycles run in turn: `0x3f8`, `0x3f9`, `0x3ec`.

### The fleet cycle (`FUN_004c8ba0`)

| State | Action |
|---|---|
| other | 2 |
| 2 | `FUN_004c8e90`: a Move or none; 3. With one, complete and raise. |
| 3 | `FUN_004c94c0`: a Move, a Create Fleet or none; 4. With one, complete and raise. |
| 4 | `FUN_004c9cb0`: a Move or none. `+0x3c = 0`, `+0x38 = 0x3f9`, complete. |

- `FUN_004c8e90` sends damaged ships to a shipyard. The fleets with record
  bits `0x1005` (own, a damaged ship that can move), forbidding `0x802`
  (en route), by key 0 (ships), order 1 (`FUN_0041a4c0`), are walked best
  first into `+0x54`: first for one at an opponent's system (`+0x30` bit
  4), failing that for one at a system without the mark `0x200000`. Its
  ships whose record has `0x1000` and not `0x800800` form the team; the
  destination is the system nearest the fleet with the mark `0x200000`
  and no blockade (`FUN_00419b80`: sector `+0x24` 8, `+0x2c` `0x20000`;
  `FUN_0041a400`: `+0x28` `0x200000`, forbidding 2; both order 2), else
  the nearest own system without a blockade (sector `+0x2c` 4; system
  `+0x30` 1, forbidding `+0x28` 2). A team and a system give Move `0x201`.
- The ship record (`FUN_00484630`) sets `0x1000` for a damaged ship
  (object `+0x50` bit `0x200`, `GameObj_SetDamaged`) whose slot `+0x34(1)`
  gives a speed, and `0x800000` for one that has none. A capital ship's
  slot `+0x34` is `FUN_00500820` (vtables `0x65d650`, `0x65d930`,
  `0x65dc10`): its hyperdrive less its damage, else its damaged rating,
  else the default speed `DAT_006b9050` while byte `+0x41` is clear (the
  port's `movement::capital_ship_speed`). `FUN_00501c00` sets Damaged
  while the hull damage (slot `+0x274`, `+0x60`) or a system damage nibble
  (`FUN_005006a0`, `+0x64`) is not 0. A ship with `0x1000` keeps no
  bombardment (`+0x48`, class `+0x38`), and its fleet takes `0x1000`
  instead of counting it among the armed ships (`FUN_00433780`).
- `FUN_004c94c0` walks the fleets with `5` (own), forbidding `0x3802`
  (en route, damaged), by key 0, order 2, until one gives an order. For a
  fleet with no inactive ship (`+0x38` `0x400` clear), at a system with a
  record:
  - with the Death Star (`0x10`), while Coruscant (`0x90000109`) is the
    side's and the fleet is elsewhere: Move the fleet there;
  - else, when its system is not an own base (`+0x30` 1 and `0x40000`):
    the base nearest it in its sector (`FUN_0041a400`, `+0x30` `0x40001`,
    forbidding `+0x28` 2), else, in a sector with a headquarters (`+0x24`
    `0x40000000`), the headquarters (`+0x28` `0x80000000`, `+0x30` 1);
    else the nearest base in another sector (`FUN_0041b2b0`: the sectors
    by distance, skipping the fleet's own), else the nearest own system in
    another sector. Move the fleet there.
  A fleet with an inactive ship and a standing one (`0x4000`) gives Create
  Fleet (`0x270`) to its standing own ships (`FUN_0041ac80`: ship `0x4000`,
  forbidding `0x803800`).
- The bases (`+0x30` `0x40000`, kept across refreshes) are set by the
  fleet plan objects' slots `FUN_004b24b0` (`0x60000`), `FUN_004b5b20` and
  `FUN_004b8070` (`0x50000`), in vtables `0x65c100`, `0x65c1b0` and
  `0x65c210`.
- `FUN_004c9cb0` (first read) runs while controller `+4` has `0x30`: it
  needs a module 8 sector planner (`FUN_00419e90`, sector `+0x24`
  `0x8000000`) whose request (`FUN_00476b70`) is above 0 to name a
  system. It then walks the side's fleets (`0x5`, forbidding 2, key `0xc`)
  and their own ships (`FUN_0041ac80`: `0x24000`, forbidding `0x800`, key
  9) for the regiments aboard (`FUN_00504c40`) and gives Move `0x201`.
  With no planner it names no system and gives nothing.

### The ship cycle (`FUN_004c8c20`)

| State | Action |
|---|---|
| other | 8 |
| 8 | `FUN_004ca300` keeps a site: 5. None: `0xd`. |
| 5 | `FUN_004ca5e0` chooses the role: 9. |
| 9 | `+0x3c` = `FUN_004ca910` (`0xf`, 10 or 0); 0: `+0x38 = 0x3ec`, complete. |
| `0xd` | `FUN_004caad0`: over budget: 10. Else `+0x3c = 0`, `+0x38 = 0x3ec`, not complete. |
| 10 | `FUN_004cab20`: a Scrap or none. End. |
| `0xf` | `FUN_004cba90`: Build `0x211`, class `+0x5c`, count `+0x6c`, on the manager `+0x50`. End. |

End: `+0x3c = 0`, `+0x38 = 0x3ec`, complete. No Destination precedes the
Build.

- `FUN_004ca300` is module 5's `FUN_004d01a0` for shipyards: the first
  site still the side's and marked `0x200000` with shipyards (`v[4] > 0`),
  idle (`+0x28` `0x200`) and calm (`+0x28 & 3` clear), its manager area 0;
  a site that lost its mark leaves the list; falling back on controller
  `+0x1c`.
- `FUN_004ca5e0` counts the requests of module 8's sector planners
  (controller `+0xd8`, request `+0x64` bits 4, 1, `0x80`, `0x10`, 8 and,
  with controller `+4` bit `0x1000000`, `0x800`) whose role was not
  tried, and takes the role with the most. With none:
  - troop carriers when the fleets' troop room (controller `+0x260`) is
    below the most regiments a system needs to be taken (`+0x20c`, the
    sector records' greatest `+0x134`, systems' `v[52]`);
  - the Death Star for the Empire (`+0x30 == 2`) with two sites or more,
    no Death Star in its fleets (controller `+4` bit 4) and the role not
    tried;
  - armed ships when its fleets have none (`+0x268 < 1`);
  - ships with lasers when none (`+0x26c < 1`);
  - carriers when the fleets' hangar room `+0x27c` is at most controller
    `+0x16c` (no writer found).

  | Role | Class flags `+0x60` | Key `+0x64` | Tried `+0x68` |
  |---|---|---|---|
  | troop carrier | `0x100004` | `0x100`: troop capacity | `0x4000000` |
  | armed | `0x100001` | `0x80000`: attack per maintenance | `0x1000000` |
  | lasers | `0x100002` | `0x400`: the laser figure | `0x2000000` |
  | bombardment | `0x100800` | `0x800`: bombardment | `0x10000000` |
  | carrier | `0x100102` | `0x80`: fighter capacity | `0x8000000` |
  | interdictor | `0x100400` | `0x10000`: shield recharge | `0x40000000` |
  | Death Star | `0x80` | `0x200`: attack | `0x20000000` |

  The keys are the ship class record's slot `+4` (`FUN_00473930`, vtable
  `0x65a398`).
- `FUN_004ca910` clears the tried bits, then with `w` = `FUN_004cc220`:
  `w < 0` gives 10, `w == 0` 0. Else the side's shipyard classes with the
  role's flags (`FUN_0041b270(0x28, +0x60, +0x64, 1)`, `FUN_0042e720`) are
  taken best first until one's maintenance is at most `w`: count 1,
  `0xf`. With none, the role's bit is tried (`+0x28 |= +0x68`), 0. With no
  role (flags 0) every shipyard class qualifies, fighters too, at key 0.
- `FUN_004cc220`: 90% of `+0x24` percent of the capacity, less the
  capital ships' upkeep (controller `+0x24c`); 0 when the side's free
  maintenance falls short. `FUN_004caad0`: the same, below 0.
- `FUN_004cab20`: the side's fleet not en route and with no inactive ship
  (`5`, forbidding `0x402`) with the least upkeep (key `0xe`, order 2),
  its own ship (`0x24000`, forbidding `0x2800`) with the least
  maintenance (key `0xb`, order 2): Scrap `0x200`.
- The controller sums (`FUN_0041ba80`, the side's own fleet records):
  `+0x24c` `+0x88` (upkeep), `+0x260` `+0xbc` (the troop carriers' room),
  `+0x268` `+0x98` (armed ships), `+0x26c` `+0x9c` (ships with lasers),
  `+0x27c` `+0x78` (hangar room); a Death Star fleet (`0x10`) sets `+4`
  bit 4.

### The yard cycle (`FUN_004c8d20`)

Module 7's yard cycle (`FUN_004d3350`) renumbered, with the token 2 and
module 2's borrow (`FUN_004cafa0` = `FUN_004e3810`, on `+0x58`, marking
`0x200000`). Every state function matches module 7's but for the fields:
`FUN_004cacc0` = `FUN_004d5000`, `FUN_004cb1d0` = `FUN_004d5510`,
`FUN_004cb710` = `FUN_004d5a50` (giving `0xe`, `0xc` or 0),
`FUN_004cb7b0` = `FUN_004d5af0`, `FUN_004cbbb0` = `FUN_004d6030`,
`FUN_004cbcf0` = `FUN_004d6170`, `FUN_004cbe10` = `FUN_004d6290` (token
2), `FUN_004cc360` = `FUN_004d67e0`.

| State | Action |
|---|---|
| other | 7 |
| 7 | `FUN_004cacc0` chooses the target `+0x58`: 8. None: end. |
| 8 | Clear controller `+8` bit 2; with `+0xc == 2` and `FUN_004cafa0`: `FUN_004199b0`, `0xb`. Else `0x10`. |
| `0xb` | `FUN_004cb1d0`: a Scrap or none (complete and raise with one). 9. |
| 9 | `+0x3c` = `FUN_004cb710` (`0xe`, `0xc` or 0); 0: `+0x38 = 0x3f8`, complete. |
| `0xc` | `FUN_004cb7b0`: a Scrap or none. End. |
| `0xe` | `FUN_004cbbb0(+0x58)`: Destination; with one, `0xf`, complete, raise. None: end. |
| `0xf` | `FUN_004cbcf0`: Build. End. |
| `0x10` | `FUN_004cbe10` (adopt or request bit 2), `FUN_004cc360` (claim). End. |

End: `+0x3c = 0`, `+0x38 = 0x3f8`, complete.

- `FUN_004cc280`: 10% of `+0x24` percent of the capacity, less the sites'
  `v[36]` (`+0xd8`; module 7's `FUN_004d6700` reads `v[34]`). It is module
  3's `FUN_004da530` on another list.

### Module `0xb` ported (2026-10-15)

`crates/rebellion-core/src/ai_module_b.rs`. Save v48.
- Ship records take Damaged (`ai_records::ship_damaged`: hull below the
  class's) as `0x1000`, or `0x800000` without a speed
  (`movement::capital_ship_speed`).
- `AIAction::MoveShips` is the Move of ships or a fleet, carried out by
  `fleet_join::move_ships_to_system`; its validation is the whole fleet's
  dispatch check, so a move to the fleet's own system is refused.
- `ScrapTarget::Ship` scraps a capital ship (`FUN_005575f0`): half its
  refined cost back; a fleet left with nothing disbands, and the regiments
  beyond the room left are lost.
- `Tallies` gains `ship_upkeep` (`+0x24c`), `troop_room` (`+0x260`),
  `armed` (`+0x268`), `lasers` (`+0x26c`), `hangars` (`+0x27c`),
  `death_star` (`+4` bit 4) and `troops_wanted` (`+0x20c`).
- Module 2's borrow is the shared `ai_module2::borrow_yard`, and module
  3's budget `ai_module3::tenth_less_yards`.
- The production heuristic builds no ships or fighters any more.

port:
- `FUN_004c94c0`'s move to a base is left out. Kind 3's plan objects now
  mark bases (`0x40000`, `ai_plan.rs`), but the other kinds' do not, and
  the fallback would send every idle fleet to another sector at each
  step; the move waits for its own stage. Only the Death Star's return
  runs.
- `FUN_004c9cb0` and the role requests of `FUN_004ca5e0` need module 8's
  sector planners; the bombardment and interdictor roles never come up.
- A fleet of squadrons alone (the port's seeding) is passed over by the
  fleet queries.
- hyp: controller `+0x16c` has no writer found and reads as 0.

### The scheduler, read again (`FUN_00485b40`)

Phase 3 steps the current module (`+0x80`). A returned order is
validated (slot `+0x1c`); a refused one goes to the module's slot `+0x18`;
an accepted one is submitted and counted unless it is a Rename (`0x203`).
A module that did not set `+0x20` stays current and steps again at the
next call; a complete one is no longer ready and the call returns 1 (the
tick ends) only if this step gave an order. So a module may take several
steps, and give an order, within one tick.

`FUN_004357b0` takes the entry with the highest place sum and removes it
from the first list, dropping the entries missing from another list;
calling it again gives the next best.

### Ported (2026-10-15)

`crates/rebellion-core/src/ai_module1.rs`, save v41:
- `Module1` holds `+0x38..+0x60`, with `step` (`FUN_004dabe0`) and every
  cycle state; `Controller` holds the yard token (`+8`, `+0xc`, `+0x10`;
  `FUN_004199b0`) and `+4`'s `0x40`; `Tallies` reads the mines and
  refineries from the sector records and the maintenance capacity and
  requirement from `side_resources::maintenance_totals`.
- The orders are `AIAction::Scrap { team }`, `SetDestination` and
  `Build { count }`. `ai::accepts` validates them (slot `+0x1c`); a
  refusal runs `Module1::refused` (`FUN_004dd810`).
- The scheduler steps a module until it completes (`FUN_00485b40`).
- The record `+0x30` bits a refresh keeps (`KEPT_KIND`) live in
  `AIState::record_kinds`; module 1's claim `0x10000000` is one.

port:
- A construction manager stands while the side has a construction yard
  in the system (hyp, `has_construction_yard`).
- Only module 1 of the eight modules that share the yard token is ported,
  so a pass hands on the yards that an unported module holds.
- The ranked lists' coin draws from the planner's generator.

On the seed-42 campaign (dual AI, 600 days) each side orders
construction yards on day 1 (Destination, then Build). From then on its
sites' yards stay busy with the heuristics' queues or blockaded, so no
site is idle and the cycle waits. The original waits under the same
conditions.

### Module 2 ported (2026-10-15)

`crates/rebellion-core/src/ai_module2.rs`, save v42. `Module2` holds
`+0x38..+0x68`, with `step` (`FUN_004e2120`) and both cycles. The steps it
shares with module 1 are free functions in `ai_module1.rs` that take the
module's fields (`scrap_at`, `wanted`, `yard_class`, `scrap_spare_yard`,
`nearest`, `destination`, `build`, `adopt_or_request`, `claim`).

port:
- The site lists `+0x58`/`+0x60` are sorted trees (`FUN_005f4f10` inserts
  by the list's comparator, slot `+8`, and `+0x10` threads them in order);
  both ports keep them in insertion order. The comparator is not read.
- The pass hands the yard token on unless module 1 or 2 holds it.
- The production heuristic's manufacturing facilities, which stood in for
  module 2, now stand in for module `0xa` (shipyard placement).

On the seed-42 campaign (dual AI, 600 days) the Empire's module 2 scraps
one manufacturing facility on day 2, then orders a mine at Xyquine from
Coruscant's yard (Destination, then Build). From day 215 it orders a mine
or a refinery every 12 days from Yaga Minor's yard, at Yaga Minor,
Coruscant, Xyquine and Sullust in turn. The Alliance's module 2 orders
nothing in this campaign.

### Module 3 ported (2026-10-15)

`crates/rebellion-core/src/ai_module3.rs`, save v43. `Module3` holds
`+0x38..+0x74` with the step (`FUN_004d6d30`) and the three cycles. The
steps shared with modules 1 and 2 are the free functions of
`ai_module1.rs` and `ai_module2.rs` (`keep_site`, `scrap_in_the_way`
joined them); `best_class` generalizes `FUN_0041b230` to any class
flags and key, and `adopt_or_request` takes the module's budget.
Controller `+0x374` is `Controller::shield_goal`, which the system
records read (`SystemInputs::shield_goal`); `Tallies::defenses` is
`+0x1dc`.

port:
- The heuristic's defenses, which stood in for module 3, are gone: no
  other module builds defenses.
- The pass hands the yard token on unless module 1, 2 or 3 holds it.

On the seed-42 campaign (dual AI, 600 days) module 3 scraps a surplus
defense four times for the Empire (days 1 to 4) and once for the Alliance
(day 1), and once scraps a facility at its yard target (`FUN_004d8fe0`).
Its defense cycle never starts: it needs one of its own sites with an
idle construction yard (`FUN_004d7ac0`). It borrows module 1's yard at
times, but its yard budget (`FUN_004da530`, 10% of 9% of a capacity of
650 to 1000, so 5 to 9) stays below a construction yard's maintenance
(10), so it never builds or adopts one. The AI therefore builds no
defenses in that campaign.

### Module 5 ported (2026-10-15)

`crates/rebellion-core/src/ai_module5.rs`, save v44. `Module5` holds
`+0x38..+0x80` with the step (`FUN_004cfed0`) and both cycles. The steps
it shares with modules 1 to 3 take a `YardKind` (`CONSTRUCTION`,
`TRAINING`): the yards' family, record count and upkeep, class flag,
idle bit, the yard family a target scrap spares by mark, and the adoption
query's bits and keys. `best_class` takes the manager's area and a
`ClassKey` (a record value or the maintenance). The troop cycle's tallies
are `Tallies::special_upkeep` (`+0x2c4`), `special_goal` (`+0x2fc`) and
`special_ready` (`+0x2ec..+0x2f8`, always 0, as above). A Build's
validator needs the side's yard of its class's area.

port:
- The heuristic's regiments, which stood in for module 5, now stand in
  for module 6 (garrison balance): module 5 trains only special forces.
- Controller `+0x14` (the fallback site) and `+0x18` (cleared before a
  Build) have no port counterpart.
- No Retire is given, as in the original (record flag `0x2000`).

On the seed-42 campaign (dual AI, 600 days) the Alliance trains special
forces at Vagran from day 75 (44 Build orders, the four basic kinds in
turn) and the Empire at Yaga Minor from day 391 (21). With more agents
the Alliance's missions rise from 17 (module 3's run) to 192.

### Module 6 ported (2026-10-15)

`crates/rebellion-core/src/ai_module6.rs`, save v45. `Module6` holds
`+0x38..+0x84` with the step (`FUN_004dd7a0`) and the three cycles. The
regiment Move is `AIAction::MoveRegiments`; its validation runs
`TroopTransportState::check_move_regiment` for each member, and the
integrator and the interactive loop carry it out with `move_regiment`.
`Tallies` gains `regiment_upkeep` (`+0x1d8`), `cargo_upkeep` (`+0x250`)
and `sector_short` (controller `+4` bit `0x20`). A Destination's validator
needs the side's yard of its area.

port:
- The first two moves (`FUN_004ddb70`, `FUN_004de130`) take their count
  from module 8's sector planners, which are not ported, so they move
  nothing.
- The raise and lowering of the side's garrison goals need the strategy
  module's controller `+4` bits `0x800000` and `0x400000`. Ported
  2026-10-15 with the posture (`ai-sector-plans.md`, save v49): the goals
  live on the controller (`+0x37c..+0x384`) and the records read them.
- Controller `+0x14` and `+0x18` have no counterpart.
- The heuristic's regiments are gone: module 6 trains them.

On the seed-42 campaign (dual AI, 600 days) the Empire moves regiments
five times (day 1 to Yaga Minor, day 4 five to Drall) and trains 5
regiments from day 487; the Alliance moves regiments five times (Hoth,
then Ghorman) and trains 10 from day 3, each sent on with a Destination.
Module 5 trains no special forces in this campaign: a temporary trace
shows it keeping one or two sites, none with an idle training facility
(`FUN_004d01a0`), while module 6, which runs first, trains regiments at
the training facilities of the sites both mark `0x400000`.
