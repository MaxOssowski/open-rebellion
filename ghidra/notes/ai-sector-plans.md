# The AI's posture, sector plans and sector planners (modules `0xd` and 8)

First read 2026-10-15, from `FUN_004bf320`, `FUN_004bf390`, `FUN_004bf490`,
`FUN_004bf7c0`, `FUN_004e5440`, `FUN_004e5530`, `FUN_004e5560`,
`FUN_004e62b0`, `FUN_004e6c00`, `FUN_004e7990`, `FUN_004e8940`,
`FUN_004e9690`, `FUN_004ea060`, `FUN_004ead50`, `FUN_004e5b10`,
`FUN_00419e90`, `FUN_0041a060`, `FUN_004317e0`, `FUN_004321e0`,
`FUN_0041b7e0`, `FUN_0041ba80`, `FUN_004cf410` to `FUN_004cfac0`, and the
planner object `FUN_004762b0` to `FUN_00477630`. The queries and their
ranked lists are in `ai-galaxy-records.md` and `ai-mission-planning.md`
("Ranking", "Picking"): a pick takes a list's last entry, the least key
for order 2 and the greatest for order 1.

## Module `0xd`: the posture and the strategy

`FUN_004bf0d0` builds module `0xd`: `+0x5c` the posture, `+0x60` the
strategy, `+0x64` the list of sector plans. Its step (`FUN_004bf320`) runs
`FUN_004bf390` while `+0x38` is `0x3fb`: state 1 chooses the posture
(`FUN_004bf490`), state 2 steps the strategy (`FUN_004e5440`, one state a
call) until it returns 1, then `+0x38 = 0x3fc`. The Alliance (module
`+0x30` 1) then runs `FUN_004bf3e0` (states 3 to 5: `FUN_004bf9e0`,
`FUN_004bfe30`, `FUN_004c05e0`; not read); the Empire completes at once
(`FUN_004bf480`). The module does not complete in between, so a pass makes
one whole posture and strategy cycle.

### The posture (`FUN_004bf490`)

From the controller's tallies (`S` the controller, module `+0x2c`):
- the ratio `S+0x258 / S+0x288` (the own fleets' attack over the other
  side's, `FUN_0041ba80`: fleet records' `values[25]`; the own attack alone
  when the other is 0), weak below `DAT_0065c49c` = 1.5;
- spare regiments `d = S+0x25c - S+0x1fc + S+0x200` (regiments aboard own
  fleets, `values[11]`; less the own systems' garrison goals, sector
  `+0x9c`; plus regiments on own planets, sector `+0xf0`);
- `t = S+0x210`: over the sectors with `+0x2c` bit 8, the least `+0xa0`
  (the regiments their opponent systems need), a 0 replaced by the next;
- `n = S+0x1ac` own systems, `E = S+0x1e8` free energy, `R = S+0x1ec` free
  raw materials, `c = S+0x1c8` sectors without `+0x2c` bit 2;
- "busy": a module waits for the yards (controller `+8` != 0).

`FUN_0041b7e0` takes the sector values at `param_2 = sector + 0x70`, with
`param_3..5` the sector's `+0x24`, `+0x2c`, `+0x28` (the call at
`0x4187e7`).

The current posture stays while its own test holds:

| Posture | Kept while |
|---|---|
| 2 | (`E < n+1` or `R < n/2+1`) and (weak or `d <= t`) and not busy and `c != 0` |
| 3 | weak, not busy, `d > 0`, `E >= n/2+1`, `R >= n/4+1` |
| 4 | busy |
| 5 | not weak, not busy, `d <= t`, `E >= n/2+1`, `R >= n/4+1` |
| 6 | not weak, not busy, `d > 0` |
| 7 | as 2, with `d > 0` |
| 1, other | never |

Otherwise it clears controller `+4` bits `0x400000` and `0x800000` and
chooses ("poor": `E < n/2+1` or `R < n/4+1`, with `c > 0`):
- weak or busy: set `0x800000`; busy gives 4; else poor gives 2 (`d <= t`)
  or 7; else 1 (`d <= t`) or 3;
- else `t < d` gives 6; poor gives 2; else set `0x400000` and give 5.

Module 6 raises the garrison goals under `0x800000` and lowers them under
`0x400000` (`ai-production-modules.md`, module 6).

A strategy whose kind (slot `+0xc`) is not the posture goes
(`FUN_004e54f0`: the planners on controller `+0xd8` lose `+0x60` bit
`0x8000000`); the plan list `+0x64` stays. `FUN_004bf7c0` builds the new one
(posture 1 `FUN_004e60e0`, vtable `0x65c8c0`; 2 `FUN_004e77c0`,
`0x65c920`; 3 `FUN_004e8770`, `0x65c950`; 4 `FUN_004e94a0`, `0x65c980`;
5 `FUN_004e9e90`, `0x65c9b0`; 6 `FUN_004eab80`, `0x65c9e0`; 7
`FUN_004e6a30`, `0x65c8f0`), points its `+0x2c` at the module's `+0x64` and
sets controller `+4` bit 2. The base (`FUN_004e52c0`): `+4` the side
(module `+0x30`), `+8` the state, `+0xc` the controller, `+0x10` flags,
`+0x14` grow, `+0x18` shrink.

### The strategy's states (`FUN_004e5440`)

State 0 sets 1; then 1 to 6 one a call; state 6 returns 1 and leaves 1.
1. `FUN_004e5530`: with controller `+4` bit 2, clear `+0x10` bit 1, run
   slot `+0x14` (name the sectors), clear bit 2.
2. `FUN_004e5560`: review (below).
3. `FUN_004e5710`: the missions (`ai-mission-planning.md`).
4. `FUN_004e57e0`: slot `+0x1c` (`FUN_004eaaa0`) sets module `0xc`'s
   requests `+0x358`/`+0x354`.
5. Slot `+0x20` (`FUN_004e69d0`): adds or removes module `0xc` planners
   toward `S+0x224`.
6. `FUN_004e5870`: system `0x90000109`'s record `+0x30` bit `0x10000000`
   follows its bit 1; for the Alliance the same on controller `+0x28`'s
   system, else on a system found by `FUN_00419a00(0,0,0x80000000,...)`.

### Naming the sectors (slot `+0x14`)

Each class counts the plans (`FUN_005f50e0` on `+0x2c`), adds one with
`+0x14` or takes one with `+0x18`, and fills a new list up to that count.
An entry (`FUN_004ecac0`, vtable `0x65ca40`) holds the sector `+0x1c`, the
kind `+0x20` and a parameter `+0x24`; the new list numbers its entries
from 1 (`FUN_005f52c0` sets the counter `+8` to 1). A choice merges a
sector query with, where named, the sectors nearest the first old plan's
(`FUN_00419b80`, order 2) and, when there were plans, twice the planned
query (`FUN_00419e90`, order 2), and picks (`FUN_004357b0`). A sector
already in the new list is skipped (`FUN_004e5260`).

**The shared tail** (every class; masks are `+0x24`, `+0x28`, `+0x2c`):
- A: kind 3, `0x40000000`/-/`4`, key `0x26` order 1;
- B: kind 3, `+0x2c` `0x16` without 8, key `0x26` order 1;
- C: kind 3, `+0x2c` 6, key `0x26` order 1;
- D: kind 1, `+0x2c` `0x80000` without 4 (classes 1, 2, 7) or without 0
  (3, 4, 5, 6), key 4 order 1.
Each loops picks while the list is short; still short after D sets
`+0x10` bit 1.

**The first choices** (one pick each, before the tail):
- 1 (`FUN_004e62b0`): none.
- 7 (`FUN_004e6c00`) and 2 (`FUN_004e7990`): kind 5, `+0x2c` without 2,
  key 4 order 2, with the near list. Class 7 then adds one kind 6:
  `0x408` without `0x40000`, key `0xc` order 2; none: `0x408`, key `0x48`
  order 2; neither with the near list; the first pick not yet named.
- 3 (`FUN_004e8940`): kind 6, `0x408` without `0x40000`, key `0xc` order
  2, near; none: `0x408` key `0x48` order 2, no near; none: kind 5,
  without 2, key 3 order 1, near.
- 4 (`FUN_004e9690`): kind 4 with `FUN_004e5b10`'s parameter (below).
- 5 (`FUN_004ea060`): kind 2, `0x40000`, key `0x48` order 1, near; none:
  `FUN_0041a060` (`+0x2c` 8 without `0x20000000`, keyed on
  `FUN_004317e0`, order 1), near on the same masks.
- 6 (`FUN_004ead50`): kind 6: with controller `+4` bit `0x400` (a sector's
  `+0x2c` bit `0x80000000`, `FUN_0041b7e0`) and the Empire (`+4 == 2`),
  `0x80000408`, key `0x4b` order 1, no other list; none: `0x408`, key
  `0x4b` order 1, near; none: `0x408` key `0x30` order 1, near on `+0x2c`
  8 (the planned lists on `0x408`); none: kind 1, `0x80100` key 4 order
  1, near; none: kind 5, without 2, key 3 order 1, near.

**`FUN_004e5b10`** (class 4): from bit `0x80` down, for each bit of
controller `+8` until a sector is found: the yard kind (1, `0x40`, `0x80`:
construction yards, sector `+0x124` = values `0x2d`, system `v[6]`,
parameter 3; 2, 8, `0x10`: shipyards, `+0x11c`, `v[4]`, 1; 4, `0x20`:
training facilities, `+0x120`, `v[5]`, 2); the sectors with `+0x2c` 2, key
`0xc` order 2, near, planned; for each pick whose yard value is above 0,
its systems with `+0x30` `0x20` without `0x101` by that `v` and by `v[20]`
(`FUN_0041a340`, order 2) until one has the yard; else the next sector.
The parameter is the last bit's even when no sector is found.

**`FUN_004317e0`**: over the sector's systems with `+0x30` `0x20` and
without 1, the days since each was seen (`FUN_00433de0`: the day
`FUN_004fcf00` less record `+0x44`), summed, divided by `+0x84 + +0x80`
when positive. `+0x44` is set to the day by the refresh when the system
is blockaded (`+0x88` bit `0x20`) and the record has `+0x30 & 6`
(`FUN_00433e40`), and by `FUN_00433dc0` from mission planners' reports
(`FUN_0047f2c0`, `FUN_0047f720`, `FUN_00480860`, `FUN_00480eb0`); it
starts at 0 (`FUN_00433b10`).

**Replacing.** When the new list has an entry, every old plan's sector
loses `+0x2c` bit `0x8000000`, `+0x30`/`+0x34` (`FUN_004317b0`, which also
clears `0x20000000`) and `+0x3c`; slot 4 empties `+0x64`; each new entry
sets its sector's `0x8000000`, `+0x30` = kind, `+0x34` = parameter
(`FUN_00431790`) and `+0x3c` = its id, and moves to `+0x64`. An empty new
list leaves the old one.

### The review (`FUN_004e5560`)

Clear `+0x14` and `+0x18`. For each plan whose sector has a planner
(`+0x38` on controller `+0xd8`): `FUN_004769f0(entry id)`; unless that
planner's `+0x60` has both `0x8000000` and `0x4000000`, the review stops
there. When every plan passed: for each planner, a missing `0x80000` marks
"not all held", `0x10000` counts a give-up, a missing `0x20000` sets
controller `+4` bit 2, and `0x8000000` is cleared. More than one give-up
(with the last planner's): bit 2 and `+0x18`. No give-up and all held and
`+0x10` bit 1 clear: bit 2 and `+0x14`. With no plan at all the review
passes and grows.

### The sector record's plan fields (`FUN_004321e0`)

`+0x30` the plan's kind, `+0x34` its parameter, `+0x38` the planner's id
(`FUN_004317d0`), `+0x3c` the plan's id. The refresh keeps `+0x2c &
0x28000000`; at its end a planned sector (`+0x30`) has `+0x2c`
`0x4000000`, and without a planner `0x10000` and `+0x18c` = -1; a planner
without a plan has `+0x18c` = 1. `FUN_0041b7e0` sums `+0x18c` into
controller `+0x204`.

`FUN_00419e90` walks the sector records (controller `+0x44`) on the same
masks as `FUN_00419a00` and keys each on `+0x3c`, 100 when 0. Merged twice
with order 2, it favours the sectors already planned, the lowest id first.

## Module 8: the sector planners (`FUN_004cf410`)

Fields `+0x38..+0x44`; the step `FUN_004cf600` runs `FUN_004cf630` when
ready:

| State | Action |
|---|---|
| other | 1 |
| 1 | `FUN_004cf700`: controller `+0x204 < 0` and no planner without `+0x60` `0x4000000`: a new planner (`FUN_0042f350` -> `FUN_004762b0`, on controller `+0xd8`); `+0x204 > 0`: drop a planner (`FUN_0042f3d0`: the first whose `+0x34` is no sector or whose sector has `+0x18c > 0`, reset by `FUN_00477190`). Then 2. |
| 2 | `FUN_004cf760`: the planner after `+0x44` (or the first) into `+0x40`; none: end. Then 3. |
| 3 | `FUN_004cf7c0`: keep the planner's sector while `+0x18c == 0` and `+0x38` is this planner; else the sector nearest it with `+0x2c` `0x10000` (`FUN_00419b80`, order 2), else one with `0x8010000`, else `0x10000` (`FUN_0041a1d0`: a random key `FUN_0041cd80(0x14)`, order 2), claimed by `FUN_004768b0`; then `FUN_004773f0` binds the plan object. None: end. Then 4. |
| 4 | `FUN_004cfac0` -> `FUN_004774c0`: the planner's step. An order completes the module and, while its cycle goes on, raises the priority; the cycle's end returns to 0. |

The planner (`FUN_004762b0`, `0xe8` bytes, vtable `0x65a3b0`): `+0x18` id,
`+0x1c` state, `+0x34` sector, `+0x40` a system (set by the plan object),
`+0x44` the plan's id, `+0x5c` the plan object, `+0x60` flags, `+0x68`
reset flag, `+0x6c..+0x90` ten counts, `+0x9c` the room
`FUN_00476f90` computes, `+0xa0..+0xac` and `+0xc8..+0xd0` per-kind counts
of the fleets and other units it holds (`FUN_00476ba0`, `FUN_00476d10`,
keyed by `FUN_004794b0`/`FUN_00479520`). `FUN_004768b0` releases the old
sector (`+0x38 = 0`) and claims a sector whose `+0x38` is 0, taking its
`+0x3c` (`FUN_004769f0`, which also tells the planner's lists). 
`FUN_004773f0` keeps a plan object whose kind (slot `+0x14`) is the
sector's `+0x30`, else builds one by kind (`FUN_00477630`: 1
`FUN_004ba580`, 2 `FUN_004b91a0`, 3 `FUN_004b37f0`, 4 `FUN_004b6fb0`, 5
`FUN_004b1720`, 6 `FUN_004b4d50`); without one it resets
(`FUN_00477190`: drops the plan object, releases the sector and the
units, cancels its missions, clears the system's `+0x30` bits
`0x70000`). The planner's step (`FUN_004774c0`, states 1 to 6:
`FUN_00477a40`, `FUN_00477cb0`, `FUN_00477e80`, `FUN_00478000`/
`FUN_00478140`, `FUN_004adb70`, `FUN_00479560`...) and the six plan
objects (about 7,000 decompiled lines) are not read yet.

## The planner's ships and the reports it gets (read 2026-10-15)

**A planner holds ships, not fleets.** Its list `+0x2c` holds capital
ship ids. `FUN_00478e00` takes a ship: the ship's record (`FUN_004f5940`)
loses `+0x24` `0x20000`, gets `+0x30` = the planner's id and `+0x34` =
its plan id, and the planner's counts grow by the record's values
(`+0x7c` into `+0x6c`, `+0x80` into `+0x70`, `+0x6c` into `+0x74`,
`+0x60` into `+0x78`, `+0x48` into `+0x80`, `+0x64` into `+0x88`, `+0x58`
into `+0x8c`; bits `0x80`, `0x400`, 2 count into `+0x7c`, `+0x84`,
`+0x90`), with request bits in `+0x64`. `FUN_004790d0` gives a ship back
(when `+0x30` is this planner: `0x20000` set, `0x3000000` cleared,
`+0x30`/`+0x34` cleared) and takes its values off. `FUN_004791d0` does the
same for a fleet record (`+0x38` bit 4 set: no planner holds the fleet,
the bit module 7's queries `0x100005` require). `FUN_00475e70` and
`FUN_00475f60` are another planner class's take and give (bit
`0x10000000`).

**The ship record** (`FUN_00484230`, vtable `0x65aa50`): the class's bits
`& 0x4ff`, `0x20000` (free), `+0x30`/`+0x34` 0. The refresh
(`FUN_00484630`) keeps `+0x24 & 0x1b0204ff`, sets `0x4000` for the
record's side's ship, `0x800` en route, `0x2000` an inactive ship
(object `+0x50` bit 4 clear), `0x1000`/`0x800000` damaged.

**Create Fleet** (`0x270`, `FUN_004f5cd0`): planner states use it to
split ships off a fleet (`FUN_00477a40`: the damaged ships, `0x801000`
without `0x800`, of the first held fleet with `0x3000` and without
`0xf0000802`; `FUN_004782a0`: free ships, `0x20000` without `0xc03800`,
of the own fleets nearest the planner's sector, `FUN_0041a600(sector,
0x8005, 2, 2)`, that fill a need the counts show).

**No order reports to a sector planner.** Mission Report `0x1f0` reaches
the mission planner whose `+0x28` is the mission (`FUN_00485f60` ->
`FUN_004195f0` -> `FUN_0042f7f0`, controller list `+0xa8`). A mission
planner that belongs to a sector planner (its `+0x70`) passes its kind
and target on (`FUN_004bdbc0` -> `FUN_00476e80` -> the plan object's
slots `+0x24..+0x30` for kinds `0x12`, `0x13`, `0x15`, `0x17`/`0x81`). So
the sector planner learns about its ships from the records each cycle,
and about missions only through mission planners.

## The planner's step (`FUN_004774c0`, read 2026-10-15)

Planner fields beyond the list above: `+0x24` the ships held, `+0x2c` the
fleets held, `+0x38` the flagship, `+0x3c` the movement planners' system,
`+0x40` the base, `+0x60` flags, `+0x64` requests, `+0x68` resync,
`+0x6c..+0x90` ten counts, `+0x94`/`+0x98` copies of counts 2 and 3,
`+0xa0..+0xac` and `+0xc8..+0xd0` its mission and movement planners by
kind, `+0xb0`/`+0xd4` those planners, `+0xbc..+0xc4` and `+0xe0`/`+0xe4`
the plan's requests. The plan object (`FUN_004ad8e0`): `+4` side, `+8`
state, `+0xc` controller, `+0x10` planner, `+0x2c` a fleet,
`+0x34..+0x58` ten wanted counts; 22 slots, kinds by slot `+0x14`
(`0x65c108` 5, `0x65c160` 3, `0x65c1b8` 6, `0x65c210` 4, `0x65c268` 2,
`0x65c2c0` 1).

With `+0x68` set the step first checks its lists against the records
(`FUN_00477810`: a ship still its own and the side's keeps its fleet
held, a free one taken by `FUN_00478fd0`, else given back;
`FUN_00477950`: a fleet not its own, not the side's or without ships is
given back). Without a plan object the cycle ends. Else by `+0x1c`:
1. clear `+0x60`/`+0x64`; `FUN_00477a40` (damaged ships off, Create
   Fleet); 2.
2. `FUN_00477cb0`: the counts from the held fleets' records (`+0xb4`,
   `+0xb8`, `+0x84`, `+0x78`, `+0x68`, `+0x7c`, `+0x70`, `+0x9c`; bits
   `0x10`, `0x20` counted), `+0x64` bits for the positive ones;
   `FUN_004adaf0`: the plan's slot `+0x50` (what it wants), less; 3.
3. `FUN_00477e80`: a count short (0, 1, 2, 3, 5, 4, 6, 9 below 0):
   `FUN_004782a0` takes free ships of the nearest free fleets, else
   `FUN_00478690` takes held ships of a later plan's fleets; short
   counts marked (`+0x64` 1, 2, 4, 8, `0x10`, `0x20`, `0x40`, `0x80`,
   `0x800`); with no order and a count to spare, `FUN_00478aa0` gives
   one ship back. Each order is Create Fleet `0x270` to `FUN_00479220`'s
   system (the base, the sector's headquarters, its own system nearest,
   the nearest other sector's). 4.
4. `FUN_00478000`: the plan's mission request (slot `+0x18`) and the
   mission planners held; `FUN_00478140`: the movement request (slot
   `+0x1c`) and the movement planners held. 5.
5. `FUN_004adb70`, the plan object's step, until it ends. 6.
6. `FUN_00479560` (the flagship among the held ships, with movement
   planners), `FUN_004799d0` (slot `+0x10` sets `+0x3c`, else cleared),
   slot `+0x20` (the report's plan bits) and `FUN_00479a00`: `+0x64 &
   0x4ff` clear gives `0x1c0000`; else nothing held (`0x9fe00000`) gives
   `0x10000`; else `0x100000`, and `0x80000` without a short `0x1f`. Then
   the default: state 1, `+0x68` set, the cycle ends.
An order sets `+0x68`.

Kind 3 (`0x65c160`): slot `+0x50` `FUN_004b4920` (wants at least 1.2
times the sector's `+0x190`, `+0x194` and `+0x198` + `+0x138`, or
controller `+0x164`/`+0x168`, never less than before; one laser ship;
a Death Star with controller `+4` `0x1000000`; the regiments held);
slot `+0x20` `FUN_004b3830` (`0x4000000`, with the sector `0x8000000`,
with an own system `0x20000`); slot `+0x18` `FUN_004b7150` (missions by
the counts of each kind held: Recon `0x54`, Espionage `0x52`, Sabotage
`0x69`, Abduction `0x62`/`0x63`); slot `+0x1c` `FUN_004b1800`
(movement 3 or 5).

## The plan object's step (`FUN_004adb70`, read 2026-10-16)

The plan object's fields beyond the list above: `+0x18` its pick list
(`FUN_00435790`/`FUN_004357b0`), `+0x28` the held fleet the step works
on, `+0x2c` the held fleet the next search starts from, `+0x30` the held
fleet state `0xe` walks from. The step returns an order and sets `*out`
when it ends (state 0); the planner's state 5 then goes to 6.

| State | Function | Next |
|---|---|---|
| 0, other | | 1 |
| 1 | `FUN_004af6d0` | 2 |
| 2 | slot `+0x34` | 3 with a base, else 5 |
| 3 | slot `+0x38` | 4 |
| 4 | `FUN_004b0120` | 5 |
| 5 | `FUN_004b04c0` | 6 with a fleet, else `0xe` |
| 6 | `FUN_004b06b0` | `0xe` with `*out`, else 7 |
| 7 | slot `+0x3c` | `0xd` with a target, else 8 |
| 8 | slot `+0x40` | `0xd`, else 10 |
| 10 | slot `+0x48` | `0xd`, else 9 |
| 9 | slot `+0x44` | `0xd`, else `0xc` |
| `0xc` | `FUN_004b0c90` | `0xe` |
| `0xd` | `FUN_004b0f90` | `0xe` |
| `0xe` | `FUN_004b1070` | 0 and `*out` once its walk ends |

Kind 3's slots (vtable `0x65c160`): `+0x34` `FUN_004af8a0`, `+0x38`
`FUN_004afce0` (both the base class's), `+0x3c` `FUN_004b4560`, `+0x40`
and `+0x48` `0x6158b0` (returns 0), `+0x44` `FUN_004b4730`, `+0x54`
`FUN_004b4a20`, `+0x24..+0x30` the mission planners' reports.

- **1, `FUN_004af6d0`:** the planner's sector record's ten systems
  (`+0x48..+0x6c`) lose `+0x30 & 0x1f00000`; each held fleet (planner
  list `+0x2c`) whose record's target `+0x2c` is a system marks it by its
  role (fleet `+0x38` `0x80000000` -> `0x1000000`, `0x10000000` ->
  `0x100000`, `0x20000000` -> `0x200000`, `0x40000000` -> `0x400000`).
- **2, `FUN_004af8a0`:** a base (planner `+0x40`) whose system record
  exists stays while it is the side's (`+0x30` 1), not blockaded (`+0x28`
  2) and, when the sector has a headquarters (sector `+0x24`
  `0x40000000`), the headquarters (`+0x28` `0x80000000`); it loses
  `0x30000`. Any other loses `0x70000` and is cleared. Without a base:
  `FUN_0041a340(sector, +0x28 0x80000000 forbidding 2, +0x30 1 forbidding
  0x40000, key 0x26, order 1)`, else the same without the headquarters
  bit, else `FUN_0041b2b0` with the second masks. A base with a record
  takes `0x40000` (and loses `0x30000`); true then.
- **3, `FUN_004afce0`:** `FUN_0041acd0(planner id, 0x40001, forbidding
  0xf0000002, key 0xc, order 1)` lists the held fleets carrying regiments
  (`FUN_0041acd0` walks controller `+0x78` for records whose `+0x30` is
  the planner). The first at a system (record `+0x28`) whose `+0x30` has
  1 gives Move `0x201` of the regiments aboard its ships
  (`FUN_0041ac80(fleet, 0x4000, forbidding 0x20800, key 9, order 1)`:
  the held, own ships not en route; `FUN_00504c40`) to that system.
- **4, `FUN_004b0120`:** when the base's record has active squadrons
  (`+0xac`, `v[25]`): `FUN_0041acd0(planner id, 1, forbidding 2, key
  0x17, order 1)`; the first fleet at the base ends the search, taken when
  its room (`+0xac`, `values[23]`) is above 0. Move `0x201` of
  `min(room, v[25])` squadrons of the base's planet (`FUN_005039d0`, mode
  1) to the fleet.
- **5, `FUN_004b04c0`:** from the held fleet `+0x2c` names (else the
  list's first), once round the list, the first whose record lacks
  `0xf0003802` becomes `+0x28`; `+0x2c` then names the fleet after it,
  cleared when that is where the search began or nothing was found.
- **6, `FUN_004b06b0`:** with the fleet's record: when it holds the
  planner's flagship (`+0x38`, `FUN_004333f0`), planner `+0x60`
  `0x1000000` sets `*out`; else `0x4000000` with `+0x3c` a system not
  blockaded gives Move `0x201` of the fleet to `+0x3c` and `*out`. Then a
  fleet with `0x4000000` and without 2 at a system with `+0x30` 1: the
  people aboard its ship records without `0x800` and with `0x600000`
  (`FUN_00536da0(ship, 1)`, person records controller `+0x8c`) all go
  into a Move `0x201` to that system, which is given when one of them
  has `+0x30` 2 and no movement planner of this planner holds it
  (`+0x28`, `FUN_00476e50` on planner list `+0xd4`).
- **7, kind 3 `FUN_004b4560`:** a fleet whose attack (`values[2]`) is
  above 0 takes the first of `FUN_0041a340(sector, +0x30 0x200
  forbidding 0x100000, key 0x27, order 1)` that `FUN_004ae150` passes:
  the system takes `0x100000`, the fleet's target `+0x2c` the system and
  its role `0x10000000`.
- **9, kind 3 `FUN_004b4730`:** a fleet whose `values[6]` is above 0
  takes the first of two merged lists (`+0x30` 4 forbidding `0x700200`;
  key 8 order 2, and key `0x10` order 1) that `FUN_004ae320` passes: the
  system takes `0x400000`, the fleet its target and role `0x40000000`.
- **`FUN_004ae150(fleet, system)`:** the enemy's strength there: with
  `+0x30` `0x200`, `v[39]`, `v[40]`, `v[43]`; with 4, plus `v[26]`,
  `v[27]`, `v[23]`. True when the first is above 0, the fleet's
  `values[2]` above 1.2 times it, and: the third is 0; or the fleet's
  `values[3]` is above the second and (with fleet `0x10` or system
  `0x400`) its `values[9]` above the third, (else) its `values[19]`
  above 0 or `values[9]` at least the third.
- **`FUN_004ae320(fleet, system, &code)`:** a system of the opponent
  (`+0x30` 4) whose sector record is `+0x3c` (`FUN_00432310`): the
  fleet's `values[3]` at least `max(v[27], controller +0x168)` when the
  fleet is elsewhere (0 at it), its `values[6]` above the system's
  `v[8]`, the system's `v[11]` 0; then with `x = (sector values[2] -
  values[4] + 1) / 2`: `x > 0` needs `v[7] + v[18] > x` and `v[18] > 0`
  (code `0x220`, Target Military Facilities); else `v[18] > 0` or `v[33]
  > 0` (code `0x222`, General Bombardment).
- **`0xc`, `FUN_004b0c90`:** with more than one held fleet,
  `FUN_0041ae20` lists the others (planner id, 1, forbidding
  `0xf0000002`, order 2); the first not at a system with an enemy fleet
  (`+0x30` `0x200`) is the target; else slot `+0x54`. Move `0x201` of the
  fleet to the target (a fleet: join it).
- **`+0x54`, kind 3 `FUN_004b4a20`:** the base not blockaded; else the
  sector's headquarters (`+0x28` `0x80000000` forbidding 2, `+0x30` 1,
  key 7, order 2); else `+0x28` `0x3e00000` (a yard) the same way, key
  `0x21`, order 1; else, when the fleet stands at a system of the
  opponent, the sector's system nearest it with `+0x30` forbidding
  `0x204` (`FUN_0041a400`, order 2).
- **`0xd`, `FUN_004b0f90`:** Move `0x201` of the fleet to its target.
- **`0xe`, `FUN_004b1070`:** from the held fleet `+0x30` names (else the
  first), each held fleet with a role and not en route: at its target
  (`+0x28 == +0x2c`) by role, `0x10000000` stays while `FUN_004ae150`
  holds; `0x40000000` gives `FUN_004b1330(fleet, 0x223)` (Destroy System)
  with fleet `0x10` and system `+0x110` above 0, else `FUN_004ae320`'s
  code; `0x20000000` (`FUN_004ae240`, code `0x234` Planetary Assault) and
  `0x80000000` (`FUN_004ae440`, `FUN_004b1440`) belong to other kinds. A
  fleet whose role fails, or that stands elsewhere, loses its role and
  target; after a `0x40000000` order it loses them too. An order stops
  the walk (`+0x30` the next fleet); the walk's end sets `*out` and
  clears `+0x30`. `FUN_004b1330(fleet, code)` builds the order `code`
  with the fleet to its target. Orders `0x220..0x223` and `0x234` are
  the Fleet menu's bombardments and assault (`move-order.md`).

The fleet record's `+0x28` is its location, written by each refresh
(`FUN_00433620`); `+0x2c` is the plan's target, written only by the
plan objects, so it persists across refreshes.

## Kinds 1 and 2's plan objects (read 2026-10-16)

The vtables (`FUN_004ad8e0`'s kinds) read from REBEXE.EXE:

| Slot | Kind 1 `0x65c2c0` | Kind 2 `0x65c268` | Kind 3 `0x65c160` |
|---|---|---|---|
| `+0x18` missions | `FUN_004b7150` | `FUN_004b7150` | `FUN_004b7150` |
| `+0x1c` movement | `FUN_004b9290` | `FUN_004b9290` | `FUN_004b1800` |
| `+0x20` report | `FUN_004ba5e0` | `FUN_004b9310` | `FUN_004b3830` |
| `+0x34`, `+0x38` | `FUN_004af8a0`, `FUN_004afce0` | the same | the same |
| `+0x3c` attacker | `FUN_004bafe0` | `FUN_004b9c90` | `FUN_004b4560` |
| `+0x40`, `+0x48` | `0x6158b0` (0) | `0x6158b0` | `0x6158b0` |
| `+0x44` bombarder | `FUN_004bb300` | `0x6158b0` | `FUN_004b4730` |
| `+0x4c` | `FUN_004b1690` | `FUN_004b1690` | `FUN_004b1690` |
| `+0x50` wants | `FUN_004bb510` | `FUN_004b9fe0` | `FUN_004b4920` |
| `+0x54` retreat | `FUN_004bb630` | `FUN_004ba0d0` | `FUN_004b4a20` |

Kinds 4, 5 and 6 (`0x65c210`, `0x65c108`, `0x65c1b8`) have their own
`+0x34`, `+0x38` and most other slots; they are not read here.

- **`+0x1c`, `FUN_004b9290`:** the same code as kind 3's `FUN_004b1800`
  (movement 3 with fleets held, `FUN_005f50e0` on planner `+0x2c`, and no
  movement planner `+0xc8`; else 5 with regiments `+0x98` and sector
  `+0x2c & 0x40008`).
- **`+0x20`:** each sets `0x4000000`, and `0x8000000` with the sector
  record; then `0x20000` when the sector's `+0x2c` has kind 3's `4`, kind
  1's `0x80000`, kind 2's `0x40000`. Kind 2 without `0x40000`: the sweep
  list below; while a listed system is not on the plan's visited list
  (`+0x5c`, `FUN_004f5940`), `0x20000`; when every one is, the sector
  record's `+0x2c` takes `0x20000000`.
- **`+0x3c`:** kind 1 and kind 2 first try kind 3's list (`+0x30`
  `0x200` forbidding `0x100000`, key `0x27`, order 1) with the fleet's
  `values[2]` (`+0x58`) above 0; with no target and its `values[3]`
  (`+0x5c`) above 0, a second list: `+0x2c` `0x400000`, `+0x30` 4,
  forbidding `+0x30` `0x100000`, key `0x1b`, order 1. Each list's first
  system that `FUN_004ae150` passes takes `0x100000`, and the fleet takes
  it as target with role `0x10000000`, as in kind 3. Kind 2 then adds the
  target to its visited list (`FUN_004f57b0` on `+0x5c`).
- **`+0x44`, kind 1 `FUN_004bb300`:** kind 3's bombarder with the fleet's
  `values[6]` (`+0x68`) above 0, over three merged lists of `+0x30` 4
  forbidding `0x700200`: key 8 order 2, key `0x21` order 1, key 0 order 1
  (kind 3: key 8 order 2, key `0x10` order 1). Same marks and role.
- **`+0x50` (the wanted counts):**
  - kind 1 `FUN_004bb510`: `+0x34` 1.2 x sector `+0x190` (kind 3 also
    takes controller `+0x164` as a floor; kind 1 does not); `+0x38` 1.2 x
    `+0x194`, floor controller `+0x168`; `+0x40` 1.2 x (`+0x138` +
    `+0x198`); `+0x48` 1.1 x `+0x144`, floor controller `+0x174` (kind 3
    leaves `+0x48`); `+0x3c`, `+0x44`, `+0x50` 0; `+0x4c` 1 with
    `FUN_005f4960() & 0x1000000`; `+0x54` planner `+0x98`; `+0x58` 1.
    Each count only rises.
  - kind 2 `FUN_004b9fe0`: as kind 3 with 1.5 (`* 3 / 2`) for 1.2:
    `+0x34` 1.5 x `+0x190`, floor `+0x164`; `+0x38` 1.5 x `+0x13c` +
    1.5 x `+0x194`, floor `+0x168`; `+0x40` 1.5 x (`+0x138` + `+0x198`),
    floor controller `+0x16c`; the rest as kind 3.
- **`+0x54` (where a fleet with nothing to do goes):**
  - kind 1 `FUN_004bb630`: when the planner holds exactly one fleet
    (`FUN_005f50e0` on planner `+0x2c` is 1) and the fleet's record lacks
    `+0x38 & 0xa00`: the pick of `FUN_0041a3a0(sector, +0x30 forbidding
    0x700201, order 1)`; then kind 3's chain (the base not blockaded, the
    headquarters, a yard, the nearest system when at the opponent's).
  - kind 2 `FUN_004ba0d0`: with exactly one fleet held, the record
    lacking `0xa00`, and its `values[2]` and `values[3]` at least
    controller `+0x164` and `+0x168`: the systems of `FUN_0041a3a0(sector,
    +0x30 0x20 forbidding 0x700201, order 1)` in rank order; the first
    not on the visited list becomes the target and joins the list. Else
    kind 3's chain.
- **`FUN_0041a3a0(sector, req28, req2c, req30, forbid28, forbid2c,
  forbid30, order)`:** `FUN_0041a340`'s masks with no key: each system
  record is ranked by `FUN_00433de0`, 0 for an own system (`+0x30` 1),
  else today (`FUN_004fcf00`, `FUN_0051ce00()->+0x10`) less the record's
  `+0x44`. `+0x44` is written by the refresh (`FUN_00433e40`) with today
  while the system is blockaded (`+0x88` `0x20`) and not the side's
  (`+0x30 & 6`), and kept otherwise.

So kind 1 is kind 3 with a second attack list (the opponent's systems
with `+0x2c` `0x400000`), a broader bombardment, a navy want and, for a
lone fleet, a patrol of the sector's systems not its own. Kind 2 sweeps
the sector: it sends its lone fleet to each listed system once, keeps the
visited ones on `+0x5c`, and reports the sector swept (`0x20000000`)
when none is left.

Open for the port: kind 2's visited list `+0x5c` must be added and saved.
The system record's `+0x44` is already kept, as `AIState::seen` (stage 1,
`FUN_004317e0`'s staleness). The fleet record's `+0x38` `0x200` is
regiments aboard and `0x800` en route (`ai_records::FleetRecord::build`);
`+0x30` `0x20` is a system held by either side (`FUN_00433e40`: `0x21`
own, `0x24` the opponent's).

## Kind 4's plan object (read 2026-10-16)

Kind 4 (vtable `0x65c210`) is the invasion of an enemy yard system: the
plan's parameter (sector record `+0x34`, `FUN_004e5b10`: 1 shipyards, 2
training facilities, 3 construction yards) names the yard kind; the plan
object keeps a target system at `+0x5c`, gathers regiments at its base,
loads them onto a held fleet and assaults the target.

| Slot | Function | Role |
|---|---|---|
| `+0x0c` | `FUN_004b7100` | the planner's request (`FUN_00476b70`) |
| `+0x18` | `FUN_004b7150` | missions (shared) |
| `+0x1c` | `FUN_004b72d0` | movement |
| `+0x20` | `FUN_004b70a0` | report |
| `+0x34` | `FUN_004b8070` | the base |
| `+0x38` | `FUN_004b8470` | regiments board at the base |
| `+0x3c` | `FUN_004b8880` | attacker |
| `+0x40` | `FUN_004b8ab0` | assault (state 8) |
| `+0x44`, `+0x48` | `0x6158b0` | 0 |
| `+0x4c` | `FUN_004b8b30` | true with planner `+0xcc` above 1 |
| `+0x50` | `FUN_004b8b70` | wants |
| `+0x54` | `FUN_004b8ca0` | retreat |

- **The target, `FUN_004b8f80`** (called first by `+0x50`): with the
  sector record, the yard key `k` = 4, 5, 6 for parameter 1, 2, 3 (system
  `v[4]` shipyards, `v[5]` training, `v[6]` construction); the merged
  lists of the sector's systems with `+0x30` `0x20` forbidding `0x101`
  (held by the opponent) by `v[k]` and by `v[0x14]`, both order 2. The
  first pick is written to `+0x5c` at once (`lea edi,[esi+0x5c]` before the
  loop, corrected 2026-10-16 from an earlier reading that kept the old
  target); while its record's `v[k]` is not above 0 the next pick replaces
  it, and with none left `+0x5c` is empty. The target is chosen anew each
  cycle.
- **`+0x0c`, `FUN_004b7100`:** with the target's and the base's records,
  the base's `v[20]` less its `v[16]`, plus the target's `v[52]`
  (`+0x118`); else 0. Modules 6 and `0xb` (`FUN_004ddb70`,
  `FUN_004c9cb0`) move regiments to the base while it is above 0
  (`ai-production-modules.md`).
- **`+0x1c`, `FUN_004b72d0`:** movement 4 when the planner has no
  movement planner of that kind (`+0xcc`), holds regiments (`+0x94`) and
  the sector has `+0x2c` 8; else kind 3's 3 and 5 (`FUN_004b1800`).
- **`+0x20`, `FUN_004b70a0`:** `0x4000000`, `0x8000000` with the sector
  record, `0x20000` while the target's record lacks `+0x30` 1 (it is not
  yet the side's).
- **`+0x34`, `FUN_004b8070`:** a base (planner `+0x40`) whose record is
  not the side's (`+0x30` 1) or is blockaded (`+0x28` 2) loses `0x50000`
  and is cleared; else it takes `0x50000` (when without `0x10000`) and
  the slot returns true. Without a base: `FUN_0041a340(sector, +0x28
  0x80000000 forbidding 2, +0x30 1 forbidding 0x40000, key 0x13, order
  1)`, else the same without the headquarters bit, else `FUN_0041b2b0`
  with those masks (the nearest other sector); the base found takes
  `0x50000`; true then. (Kind 3: mark `0x40000`, key `0x26`.)
- **`+0x38`, `FUN_004b8470`:** with regiments at the base (its record's
  `+0x94`, `v[19]`, above 0): of the planner's held fleets
  (`FUN_0041acd0(planner id, 1, forbidding 2, key 0x16)`) the first at the
  base (record `+0x28`); with its room (record `+0xa8`, `v[22]`) above 0,
  `min(room, v[19])` of the base's regiments, the strongest first (object
  slot `+0x1dc`, each once), board it: Move `0x201` of the regiments to
  the fleet (`FUN_00504c40`, `FUN_00504d40`, `FUN_004f5cd0`).
- **`+0x3c`, `FUN_004b8880`:** a fleet with `values[2]` above 0 first
  tries the target itself when it has an enemy fleet (`+0x30` `0x200`)
  and `FUN_004ae150` passes; else kind 3's list (`0x200` forbidding
  `0x100000`, key `0x27`, order 1). Same marks and role `0x10000000`.
- **`+0x40`, `FUN_004b8ab0`:** a fleet with `values[12]` (`+0x80`) above 0
  takes the target when `FUN_004ae240` passes: the system takes
  `0x200000`, the fleet the target and role `0x20000000`.
- **`FUN_004ae240(fleet, system)`:** the system has `+0x30` `0x20` and not
  `0x101`; at the fleet's own system both thresholds are 0, elsewhere `a`
  = max(system `v[27]`, controller `+0x168`) and `b` = system `v[23]`.
  True when the fleet's `values[3]` is at least `a`, its `values[12]` at
  least the system's `v[52]`, and `values[19]` above 0 or `values[9]` at
  least `b`. State `0xe` (`FUN_004b1070`) then gives the order `0x234`
  Planetary Assault at the target while it holds.
- **`+0x50`, `FUN_004b8b70`:** after `FUN_004b8f80`, kind 2's 1.5 times
  for `+0x34` (floor `+0x164`) and `+0x38` (the sector's `+0x194` plus the
  target's `v[27]`, floor `+0x168`), 1.1 times (`+0x138` + `+0x198`) for
  `+0x40` (floor `+0x16c`); `+0x3c` at least the target's `v[52]`; `+0x44`
  1 with controller flags `4` and the sector's `+0x19c` above 0; `+0x4c`
  1 with `0x1000000`; `+0x50` the planner's regiments `+0x94`; `+0x54`
  `+0x98`; `+0x58` 1. hyp: with no target `+0x3c` compares the value left
  from `+0x40`.
- **`+0x54`, `FUN_004b8ca0`:** the base (blockaded or not), else the
  headquarters (`+0x28` `0x80000000`, `+0x30` 1, key 7, order 2), else a
  yard (`0x3e00000`, key `0x21`, order 1), else at the opponent's the
  nearest system without `0x204` (kind 3's chain without the blockade
  tests).

Open for the port: the Planetary Assault order `0x234` (its resolution is
not read), the planner's request `+0x0c` and modules 6 and `0xb`'s moves
that read it, loading regiments onto a fleet as an AI order, and the
plan object's target `+0x5c` (saved).

## Ported (2026-10-16, save v57): kind 4's plan object

`ai_planner::ported_kind` takes kind 4. `ai_planner.rs`:
`SectorPlanner::choose_target` (`FUN_004b8f80`, the plan object's `+0x5c`,
`SectorPlanner::plan_target`), `want_invasion` (`FUN_004b8b70`), movement
4 (`FUN_004b72d0`), the report's `0x20000` while the target is not the
side's (`FUN_004b70a0`). `ai_plan.rs`: `invasion_base` (`FUN_004b8070`,
marks `0x50000`, key `0x13`), `board` (`FUN_004b8470`,
`AIAction::BoardRegiments`, new), the attacker trying the target first
(`FUN_004b8880`), `assault_target` (`FUN_004b8ab0`, role `0x20000000`,
`fleet_assaults` = `FUN_004ae240`), no bombarder (slot `+0x44`), the
retreat without the blockade tests (`FUN_004b8ca0`). State `0xe` now
settles the assault role as `FUN_004b1070` does: Destroy System with
fleet `0x10` and `v[50]`, else Planetary Assault `0x234` while
`FUN_004ae240` holds, else `FUN_004ae320`'s bombardment; the role stays
after an order and goes only when none is given. Save v57.

port:
- The planner's request (slot `+0x0c`, `FUN_004b7100`) and modules 6 and
  `0xb`'s moves of regiments to the base it drives are not ported, nor is
  slot `+0x4c` (`FUN_004b8b30`).
- hyp: the base's walk gives the side's regiments on the planet in its
  order; without a target `+0x3c` takes the `+0x40` value.

Campaigns (600 days, both sides AI), a temporary trace (removed): kind 4
planners name a target in seeds 42, 3 and 47 and board regiments (2, 3
and 3 orders), but no fleet binds the assault: the fleets carry at most
3 regiments where the targets need 4 or 5 (`v[52]`), the base receiving
none without the planner's request. No assault happens yet.

## Ported (2026-10-16, save v56): kind 2's plan object

`ai_planner::ported_kind` takes kind 2. `ai_planner.rs`: slot `+0x50`
`FUN_004b9fe0` (1.5 for 1.2, `+0x38` adding the sector's `+0x13c`, `+0x40`
with the floor controller `+0x16c`); the report `FUN_004b9310`
(`SectorPlanner::sweep`, `sweep_left`): `0x20000` with the sector's
`0x40000` or a system of the opponent left to visit, else the sector record
takes `0x20000000`. `ai_plan.rs`: slot `+0x3c` `FUN_004b9c90` (kind 1's
lists, the target joining the visited list), no bombarder (slot `+0x44`
`0x6158b0`), slot `+0x54` `FUN_004ba0d0` (a lone fleet to the first system
of the opponent not visited). `SectorPlanner::visited` is the plan object's
`+0x5c`, dropped with the object (`FUN_004773f0`, `FUN_00477190`).
`AIState::swept` keeps the sectors' `0x20000000`, which the port's rebuilt
records would lose (the refresh keeps `+0x2c & 0x28000000`); naming the
plans clears it for the old and new plans' sectors (`FUN_004317b0`,
`FUN_00431790`). Save v56.

hyp: controller `+0x16c`, like `+0x164` and `+0x168`, reads as 0.

Campaigns (600 days, both sides AI), a temporary trace (removed): kind 2
attackers bind in seeds 42, 1, 2, 3, 7 and 47; seed 3 sends a lone fleet to
three systems from day 463; seeds 1 and 2 mark their sector swept. The kind
2 planners now take ships and create fleets (Create Fleet 1 -> 3 in seed
42), so every campaign's counts move.

## Ported (2026-10-16, save v55): kind 1's plan object

`ai_planner::ported_kind`: a kind 1 planner now runs its own cycle (no
stand-in). `ai_planner.rs`: slot `+0x50` `FUN_004bb510` (`+0x34` without
the controller floor, `+0x48` 1.1 x the sector's `+0x144`), the shared
mission and movement slots, the report's `0x20000` from the sector's
`+0x2c` `0x80000` (`FUN_004ba5e0`). `ai_plan.rs`: slot `+0x3c`
`FUN_004bafe0`'s second list (`attack_from`), slot `+0x44`
`FUN_004bb300`'s three lists, slot `+0x54` `FUN_004bb630`'s lone fleet's
patrol (`Galaxy::system_query_by_age`, `FUN_0041a3a0`; `PlannerIo` gains
the day and `AIState::seen`). No save change.

hyp: controller `+0x174` (`+0x48`'s floor), like `+0x164` and `+0x168`,
has no writer found and reads as 0.

Campaigns (600 days, both sides AI): seeds 42, 1, 2, 3, 7, 11, 13 and 99
make no kind 1 planner; in seed 47 a kind 1 planner runs 713
cycles holding no fleet, so neither the patrol nor the second attack list
comes up, and every event count equals stage 4f's.

## Ported (2026-10-16, save v55): the plan object's bombardments

`FUN_004b1070` now gives its bombardment: a `0x40000000` fleet at its
target orders `AIAction::Bombard` (`FUN_004b1330`) with Destroy System
`0x223` when the fleet has `0x10` and the system `v[50]` above 0, else
`FUN_004ae320`'s code, then loses role and target. The order stops the
walk, `+0x30` (`SectorPlanner::plan_walk`) naming the next fleet; on the
last fleet the order ends the step too. The bombardment itself is
`bombardment-order.md`'s port.

- Seeds 42, 1, 2, 3, 7, 11, 13, 47 and 99 (600 days, both sides AI) and
  3, 5, 8, 21, 33, 64, 77, 123, 2024 and 4242 (900 days): no plan object
  binds a role, so the AI never bombards. Without the automatic
  bombardment after battles no bombardment happens in these campaigns.

## Ported (2026-10-16, save v54): the plan object's states 7 to `0xe`

`crates/rebellion-core/src/ai_plan.rs`: kind 3's step runs to its end.
1 marks each held fleet's target by its role; 7 binds an attacker
(`values[2]`) to the first enemy fleet's system it prevails at
(`FUN_004ae150`: `0x100000`, role `0x10000000`); 8 and 10 return 0; 9
binds a bombarder (`values[6]`) to the first system of the opponent it
can bombard (`FUN_004ae320`: `0x400000`, role `0x40000000`); `0xd` moves
the fleet to its target; `0xc` joins the nearest other held fleet without
an enemy fleet there (`FUN_0041ae20`, `AIAction::JoinFleet`, new), else
goes to slot `+0x54`'s system (`FUN_004b4a20`); `0xe` keeps an attacker
at its target while it prevails and takes every other role and target
back, then ends the step (`*out`), and the planner's state 5 goes to 6.
The fleet record's target `+0x2c` is kept with its hold. The fleet
heuristics no longer stand in for kind 3, and leave the fleets any
planner holds alone.

port:
- The bombardments `FUN_004b1070` gives at a `0x40000000` fleet's target
  (`FUN_004b1330`: `0x223` with fleet `0x10` and system `v[50]` above 0,
  else `FUN_004ae320`'s `0x220` or `0x222`) are not given: the Fleet
  menu's bombardment orders are not ported and the bombardment itself is
  unrecovered (F-032). The fleet loses its role and target as after the
  order, and with no order the walk always reaches its end, so its cursor
  `+0x30` stays clear.
- The port's move refuses a fleet's own system, which the original's
  accepts and does nothing with (`FUN_00578d00`); a move there gives no
  order.
- The stand-in for the other kinds skips a held fleet (record `+0x30`
  not 0): its plan object moves it.
- hyp: controller `+0x168` reads as 0 (`FUN_004ae320`'s shields).
- Seed 42 (600 days, both sides AI): the Alliance's planner holds one
  fleet at its base in the headquarters' sector, which has no enemy fleet
  or system of the opponent; states 7 and 9 find no target and `0xc` none
  but the base it stands at, so the plan gives no order. Without the
  stand-in for kind 3 the heuristics' fleet moves (both sides) fall from
  28 to 15 and the space battles from 12 to 5. In seed 3 the Alliance's
  planners send a fleet home twice (days 100 and 163) and bind a
  bombarder, which moves to its target (day 519) and there, with no
  bombardment, loses its role and takes it again each cycle.

## Ported (2026-10-16, save v53): the plan object's states 1 to 6

`crates/rebellion-core/src/ai_plan.rs`: the planner's state 5 runs the
plan object's step (`FUN_004adb70`) for kind 3: 1 clears the sector's
role marks; 2 keeps or chooses the base (`0x40000`); 3 unloads a held
fleet's regiments at an own system (`AIAction::MoveRegiments`); 4 boards
the base's squadrons onto a held fleet there
(`AIAction::MoveSquadronTeam`, new); 5 finds the next idle held fleet; 6
moves the flagship's fleet or sets down the fleet's people
(`AIAction::MoveCharacters`, new). A planner's reset gives up its base
and the base's marks `0x70000`. `SectorPlanner` keeps the plan object's
`+0x28` and `+0x2c`.

port:
- States 7 to `0xe` (the fleet's role and its orders) are the port's
  fleet heuristics, entered at state 7 or `0xe` and run once a cycle;
  their end ends the plan's step (state 0). No held fleet has a role or
  a target yet, so state 1 only clears the marks.
- A fleet's regiments and people are its cargo and characters, which its
  first ship's record carries; special forces are not aboard fleets.
- hyp: the planet's squadron walk of `FUN_004b0120` (`FUN_005039d0`,
  mode 1) gives the side's squadrons in the planet's order.
- No movement planner holds anyone and the port names no flagship, so
  the flagship's branch of state 6 does not come up.
- Seed 42 (600 days, both sides AI): the Alliance's planners take bases
  (its headquarters for the first); their held fleets carry no regiments
  or people and the bases no squadrons, so no state gives an order, and
  every event count equals stage 4a's. Nor in seeds 1, 2, 3, 7, 11, 13,
  47 and 99.

## Ported (2026-10-15, save v52): the planner's step, kind 3's wants

`crates/rebellion-core/src/ai_planner.rs`: the planner's lists, counts,
requests and the step's states 1 to 4 and 6 above, with kind 3's slots
`+0x50`, `+0x20`, `+0x18` and `+0x1c`. `AIState::fleet_holds` keeps the
fleet records' kept bits (`0xf000000c`), `+0x30` and `+0x34`. Module 8's
state 4 runs the step; an order completes the module and raises its
priority while the cycle goes on. `AIAction::CreateFleet` gains its
target (`+0x34`).

port:
- State 5 (the plan object's step) is the port's fleet heuristics,
  decided once a cycle; the other kinds' planners run them for the whole
  cycle.
- The planners hold no mission or movement planners: their counts are
  0, so kind 3 always asks for Recon or Espionage and, holding a fleet,
  for movement 3; the flagship and `+0x3c` stay clear.
- hyp: `FUN_00477a40`'s Create Fleet target is read from a list node,
  not an id; the port leaves the damaged ships in their system. The
  Create Fleet of `FUN_004782a0` and `FUN_00478aa0` reads its origin
  from an unset local; the port uses the ships' system.
- hyp: controller `+0x164`, `+0x168` have no writer found and read as 0.

## Ported (2026-10-15, save v51): the ships' records

port: `ShipInstance::id` and `GameWorld::number_ships` give each capital
ship an identity (the drivers number the ships before the AI's passes, at
a new world and at a load); `AIState::ship_holds` keeps what the AI sets
on a ship record (`+0x24 & 0x1b020000`, `+0x30`, `+0x34`) by that id, and
the refresh writes it into the records (`apply_ship_holds`), dropping the
holds of ships no longer among the side's records. `FleetRecord::ships`
lists the ship records (`ShipRecord`). `AIAction::CreateFleet` carries
`0x270` out through `fleet_join::create_fleet`, validated by
`fleet_join::check_create_fleet`. `Galaxy::fleets_near` is
`FUN_0041a600` (module 7's query, now shared). No module issues Create
Fleet or takes a ship yet.

## Ported (2026-10-15, save v50): module 8's planners

`crates/rebellion-core/src/ai_module8.rs`: `Module8` (`+0x3c..+0x44`, the
planner list controller `+0xd8` with its id counter) and `SectorPlanner`
(`+0x18` id, `+0x1c` step state, `+0x34` sector, `+0x44` plan id, the plan
object's kind, `+0x60` flags). States 1 to 3 of `FUN_004cf630`
(`FUN_004cf700`, `FUN_0042f350`, `FUN_0042f3d0`, `FUN_004cf760`,
`FUN_004cf7c0`, `FUN_004768b0`, `FUN_004773f0`, `FUN_00477190`); the
refresh writes each planner's sector `+0x38` (`mark_planners`). The
strategy's review now reads the planners (`FUN_004e5560` in full) and a
posture change clears their `0x8000000` (`FUN_004e54f0`).
`Planner::random` is `FUN_0041cd80(n)`; `FUN_0041a1d0`'s key draws
interleave with its tie coins.

port:
- The plan objects (`FUN_00477630`) and the planner's step
  (`FUN_004774c0`) are not ported. In state 4 a planner holding a plan
  runs the port's fleet heuristics instead, decided once a cycle and
  issued one order a step; without a planner module 8 orders nothing.
- No planner gains `+0x60` bits, so module 8 never adds a second planner
  and the strategy never grows past one plan.
- The planner holds no fleets, units or missions, so its reset releases
  none.

## Ported (2026-10-15, save v49)

`crates/rebellion-core/src/ai_strategy.rs`: `Strategy` (posture, kind,
`+0x10` bit 1, grow, shrink, the plans), the posture, states 1 and 2 and
the seven classes' slot `+0x14`. `SectorRecord` gains `plan`,
`plan_param`, `planner`, `plan_id` and `plan_bits`; `Galaxy` gains
`sectors_near` (`FUN_00419b80` from a sector) and `planned_sectors`
(`FUN_00419e90`). `Tallies` gains the posture's sums and `+0x204`;
`Controller` the garrison goals `+0x37c..+0x384`, which the system records
read; `AIState::seen` keeps record `+0x44`.

port:
- Module 8's planners are not ported, so no plan has one: the review
  grows the list only while it is empty, and the strategy keeps one
  planned sector.
- States 4 to 6 and the Alliance's states after the strategy are not
  ported; the research strategy (state 3) is the port's.
- `+0x44` is set from blockades only; the mission planners that set it
  are not ported.
- The strategy runs when module `0xd` decides anew; while the research
  strategy has undecided orders, a pass skips it.
- Posture 6's second fallback (`0x408` by `+0x130`) cannot pick when the
  first (`0x408` by `+0x19c`, the same masks and lists) found nothing; it
  is ported as read.
