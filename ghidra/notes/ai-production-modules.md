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
