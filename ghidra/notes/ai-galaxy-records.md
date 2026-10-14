# The AI's galaxy records: systems, sectors, classes, and queries

First read 2026-10-14, from `FUN_004321e0`, `FUN_00433e40`, `FUN_00432350`,
`FUN_00433b10`, `FUN_00431450`, `FUN_0042dd00`, `FUN_00419a00`,
`FUN_00431830`, `FUN_0041a340` and `FUN_0041bd20`. The production modules
(`ai-mission-planning.md`, "The fourteen modules") choose sectors and
systems by querying these records.

Offsets are into the record unless marked "system" (the game object). The
value array starts at `+0x48`: `v[k]` means `+0x48 + 4k`, and a query key
`k` sorts on `v[k]`.

## Refresh (`FUN_004321e0`)

A sector record refreshes when its dirty flag `+0x20` is set. It clears:
- `+0x28` to its `0x30000000` bits;
- `+0x2c` to its `0x28000000` bits;
- its values `+0x70..+0x1d8` (0x5a words).

It sets `+0x144` = 100000. Then, for each of its 10 system records
(`+0x48..+0x6c`), it refreshes the system (`FUN_00433e40`) and adds it in
(`FUN_00432350(v, &+0x28, &+0x30, &+0x2c)`). Afterwards:
- `+0x144` = 0 unless `+0x2c` bit `0x100` is set;
- `+0x8c` /= `+0x7c` when `+0x7c` > 0;
- `+0x70` == 10 sets `+0x28` bit `0x40000000`;
- `+0x74` == 10 sets `+0x2c` bit 1;
- `+0x78` == 10 sets `+0x2c` bit 2;
- `+0x19c` > 0 sets `+0x2c` bit `0x80`;
- `+0x30`, `+0x38` set `+0x2c` bits `0x4000000`/`0x10000` and `+0x18c` = 1
  or -1 (untraced).

## Side constants (`FUN_0041bd20`, at AI setup)

The side object (controller `+0x44`) holds:

| Field | Value | Field | Value |
|---|---|---|---|
| `+0x364` | 3 | `+0x37c` | 0 |
| `+0x368` | 3 | `+0x380` | 3 |
| `+0x36c` | 2 | `+0x384` | 9 |
| `+0x370` | 1 | `+0x388` | 0 |
| `+0x374` | 1 | `+0x38c` | 6 |
| `+0x378` | 0 | `+0x390` | 12 |

`+0xa4` is the count of the side's MNCHARSD classes (`ai-mission-planning.md`).

## Class records (`FUN_0042dd00`, tree lookup `FUN_0042e190`)

Every record shares these fields:
- `+0x1c`: the class's refined material cost (`FUN_0053b860`, class
  `+0x48`) for facilities;
- `+0x20`: its maintenance (`FUN_0053b870`, `+0x4c`);
- `+0x24`: research order (`+0x50`);
- `+0x28`: side (1 Alliance only, 2 Empire only, 0 both).

Per family:
- **Capital ships** `0x14..0x1b` (`FUN_00473830`/`FUN_004739b0`): bit
  `0x80`, and bits `0x400`, `0x100000`, 1, 2.
- **Fighters** `0x1c..0x1f` (`FUN_00473570`).
- **Regiments** `0x10..0x13` (`FUN_00473400`/`FUN_004734d0`): `+0x30` attack
  (class `+0x64`), `+0x34` defense (`+0x68`), `+0x38` (`+0x60`), `+0x3c`
  (`+0x5c`). Bit `0x1000` when defense <= attack, `0x2000` when attack <=
  defense.
- **Defenses** `0x22..0x27` (`FUN_004731c0`/`FUN_00473290`): `+0x30` (class
  `+0x5c`), `+0x34` (`+0x60`). Bits `0x20000` for `0x24`, `0x40000` for
  `0x25`, `0x80000` for `0x22`/`0x23`.
- **Yards** `0x28..0x2b` (`FUN_00472fd0`/`FUN_00473090`): `+0x30` (class
  `+0x5c`). Bits `0x4000` shipyard, `0x8000` training, `0x10000`
  construction yard.
- **Special forces** `0x3c..0x3f` (`FUN_00472be0`).

## System record (`FUN_00433e40`)

Identity: `+0x18` the system's DatId, `+0x1c` the record's side (1 or 2),
`+0x20` the system's holder (`+0x24 >> 6 & 3`), `+0x34` the map position
(`FUN_00509620`), `+0x40` the sector record.

Refresh: return unless `+0x24` (dirty) is set; zero `v[0..0x3c]`, keep
`+0x28 & 0x3e00000` (the yard marks), `+0x30 & 0x35ff0000`,
`+0x2c & 0x60000000`.

**`+0x30` (side and kind):**
- 1: the holder is the record's side; 2: neither side; 4: the opponent.
  0x20 joins 1 or 4.
- 0x10: system `+0x50` bit 8.
- 0x40: a system of type `0x92..0x93`.
- 0x8 and 0x28: the system is populated (`+0x88` bit 0).
- 0x80: an active `0x25` defense.
- 0x100: more than one `0x24` defense.
- 0x200: another side's fleet here with a standing, active ship (fleet
  record `+0x38` bit `0x4000`, bit 1 clear: bit 1 marks the record side's
  own fleet, `FUN_00432e40`/`FUN_00433620`); 0x400 with fleet bit `0x10`.
- 0x800: a yard, mine, refinery or defense, but no `0x22`/`0x23` defense.
- 0x1000, 0x3000: a character with officer rank (`0x7000000`, `0x2000000`).
- 0x4000, 0x8000: own regiments whose class has bit `0x1000`/`0x2000`.
- 0x8000000: the system's id `0x121`.
- 0x40000000: id `0x109`.
- 0x80000000: the side's headquarters (`0x20..0x21` objects,
  `FUN_00526a60`).

**`+0x28` (state):**

| Bit | Meaning |
|---|---|
| 1 | uprising (system `+0x88` bit 2), populated systems only |
| 2 | blockade (system `+0x88` bit `0x20`) |
| `0x20`, `0x40` | own: free energy `v[0xf]` > 0, and free raw `v[0xe]` > 0 |
| `0x100`, `0x180` | room for a mine or refinery (two paths, below) |
| `0x200`, `0x400`, `0x800` | an idle shipyard, training facility, construction yard: an active yard, and its area's manager (`FUN_00509670`, area 0, 2, 1) has `+0x58` == 0 |
| `0x1000` | any active yard |
| `0x2000`, `0x10000`, `0x20000` | the garrison against its goal `v[0x16]` (below) |
| `0x4000`, `0x8000` | the defense balance `v[0x25]` < 0, or the battery part < 0 |
| `0x40000`, `0x100000` | `v[0x26]` < 0, `v[0x35]` < 0 |
| `0x80000` | an active regiment, own system |
| `0x200000`, `0x400000`, `0x3800000` | the AI's yard marks (ships, troops, facilities `0x800000`/`0x1000000`/`0x2000000`); kept across refreshes |
| `0x4000000` | own, no uprising or blockade, no mark |
| `0x10000000`, `0x20000000` | own regiments, class bits `0x1000`/`0x2000`, at strength |
| `0x40000000` | Empire with controller bits `0x1000000`, 4 and no `0x25` defense |
| `0x80000000` | `0x109`, or the headquarters |

**Values** (`v[k]` = `+0x48 + 4k`):

| k | Offset | Value |
|---|---|---|
| 0 | `+0x48` | support: 100 for own, then `FUN_00507270` (the side's support) when populated |
| 1 | `+0x4c` | system `+0x5c` (energy) |
| 2, 3 | `+0x50`, `+0x54` | the side's mines, refineries (`FUN_0052d370`, `FUN_0052cf40`) |
| 4, 5, 6 | `+0x58..+0x60` | shipyards, training facilities, construction yards |
| 7 | `+0x64` | `0x24` defenses |
| 8 | `+0x68` | their class `+0x34`, active |
| 9 | `+0x6c` | the battery goal |
| 10 | `+0x70` | `0x25` defenses |
| 11 | `+0x74` | `0x22`/`0x23` defenses |
| 12 | `+0x78` | their class `+0x30`, active |
| 13 | `+0x7c` | the `0x22`/`0x23` goal |
| 14 | `+0x80` | system `+0x64` - `+0x68` (free raw) |
| 15 | `+0x84` | system `+0x5c` - `+0x60` (free energy) |
| 16 | `+0x88` | regiments (`FUN_00504c40`, `0x10..0x13`) |
| 17 | `+0x8c` | regiments counted toward the garrison goal |
| 18 | `+0x90` | active regiments |
| 19..21 | `+0x94..+0x9c` | garrison surplus, -, deficit |
| 20 | `+0x98` | the support term, at most 6 (doubled in an uprising) |
| 22 | `+0xa0` | the garrison goal |
| 23..25 | `+0xa4..+0xac` | squadrons on the planet (`FUN_005039d0`, `0x1c..0x1f`): all, counted, active |
| 26..28 | `+0xb0..+0xb8` | their strengths (`FUN_00473610..630`) |
| 29 | `+0xbc` | foreign fleet records' `+0xa4`, and for an opponent's system `v[18]` + `v[25]` |
| 30, 31, 32 | `+0xc0..+0xc8` | maintenance of regiments, defenses, squadrons |
| 33..36 | `+0xcc..+0xd8` | maintenance of yards: all, shipyards, training, construction |
| 37 | `+0xdc` | the defense balance |
| 38 | `+0xe0` | regiments against the goal |
| 39..43 | `+0xe4..+0xf4` | foreign fleets' records `+0xb4`, `+0xb8`, `+0x60`, -, `+0x70` |
| 44..48 | `+0xf8..+0x108` | characters |
| 50 | `+0x110` | the headquarters counts |
| 51 | `+0x114` | free facility slots: energy - (marked yards + max(`0x24`, goal) + max(`0x22`, goal) + HQ) |
| 52 | `+0x118` | |
| 53 | `+0x11c` | regiments against the goal |

**Goals** (own systems, `+0x30` bit 1):
- support term `v[20]` += (79 - support) / 10 when support < 70, at most 6;
- `0x24` goal `v[9]` and `0x22` goal `v[13]` come from the side constants
  `+0x364..+0x378` by HQ, marks, and energy;
- the garrison goal `v[22]` is side `+0x37c` (unmarked), `+0x380`
  (marked), `+0x384` (HQ), raised to `v[20]`;
- squadrons against the goal `v[38]`, `v[53]` subtract `+0x388`/`+0x38c`/
  `+0x390`; an unmarked system with no counted regiment takes all its
  squadrons.

Ported 2026-10-14 as `ai_records::SystemRecord` (with `FleetRecord` and
`ClassRecords`); the port's assumptions are in its doc comment. The
yard marks live in `AIState::yard_marks` (save v40) and reach the record
through `Galaxy::build`.

Opponent and neutral systems get their own support term and `+0x2c` bits.

**`+0x2c`** (summaries for the queries and the strategy):

| Bit | Meaning |
|---|---|
| 1, 0x200, 0x80 | own; own or opponent; not own |
| 2 | no uprising or blockade, side `+0xa4` > 0, not `0x121` |
| 4, 8, 0x10 | research site for ships, troops, facilities (`ai-mission-planning.md`) |
| 0x20 | an uprising, no own fleet |
| 0x40 | support below 70 (`0x10000000`), no own fleet, no uprising; or neutral with no fleet |
| 0x80 | not own |
| 0x100 | opponent with support above 50 |
| 0x400, 0x1000, 0x4000, 0x4000000 | character presence |
| 0x800, 0x2000, 0x8000 | fleet record bits |
| 0x10000, 0x20000 | opponent with mines or refineries; with yards |
| 0x40000..0x400000 | opponent with `0x24`, `0x25` (active), `0x22`, fleets, regiments |
| 0x800000, 0x1000000 | an own fleet; with fleet bit `0x10` (Alliance) |
| 0x2000000 | the headquarters |
| 0x8000000 | explored (system `+0x88` bit 1); otherwise the refresh stops after `0x80` |
| 0x10000000 | support below 70 |

## Sector record (`FUN_00432350`, per system)

Own systems (`+0x30` bit 1) add:
- `+0x2c` bit 4 and `+0x7c` += 1;
- support and fleet counts into `+0x8c..+0xdc`;
- the yard bits:
  - `+0x2c` `0x800000`, `0x1000000`/`0x2000000` from `+0x30` `0x40000`,
    `0x10000`, `0x20000`;
  - `+0x24` bit 8 without a blockade;
  - with no uprising or blockade, not `0x8000000`, and free slots
    `v[0x33]` > 0: `+0x24` bit `0x20` with no ship mark, `0x40` with no
    troop mark, `0x80` with no facility mark;
  - `0x100`/`0x200`/`0x400`/`0x800` from `+0x28` `0x80`/`0x100`/`0x20`/
    `0x40` without uprising or blockade;
  - `0x1000` (a facility mark and an idle construction yard);
  - `0x2000` (an idle construction yard, no facility mark);
  - `0x20000000` (any mark), with marked capacity `+0xb0`/`+0xb8`/`+0xc0`
    and unmarked `+0xb4`/`+0xbc`/`+0xc4`;
  - `0x8000`/`0x10000` from `v[0x25]` < 0 / > 0, and `0x4000` with a mark.

Opponent systems (`+0x30` bit 4) set `+0x2c` bit 8. Neutral systems set bit
`0x10`. Then for every system, the `+0x2c` summaries count into
`+0x150..+0x184` and set `+0x28` bits 1..`0x8000`:

| System `+0x2c` | Sector count | Sector `+0x28` |
|---|---|---|
| 1 | `+0x150` | |
| 2 | `+0x160` | 8 |
| 4 | `+0x154` | 1 |
| 8 | `+0x158` | 2 |
| `0x10` | `+0x15c` | 4 |
| `0x20` | `+0x164` | `0x10` |
| `0x40` | `+0x168` | `0x20` (`0x4000020` without `0x20000000`) |
| `0x80` | `+0x16c` | `0x40` |
| `0xc00` | `+0x170` | `0x80` |
| `0x3000` | `+0x174` | `0x100` |
| `0x100` | `+0x178` | `0x200` |
| `0x200` | `+0x17c` | `0x400` |
| `0x10000` | | `0x800` |
| `0x20000` | | `0x1000` |
| `0x800000` | | `0x2000` |
| `0xc000` | `+0x180` | `0x4000` |
| `0x1000000` | `+0x184` | `0x8000` |

Ported 2026-10-14 as `ai_records::SectorRecord` (the sector's own system
count stands for the original's fixed 10).

## Queries

`FUN_00419a00(req24, req28, req2c, forbid24, forbid28, forbid2c, key,
order)` lists every sector record whose `+0x24`, `+0x28`, `+0x2c` hold the
required bits and none of the forbidden ones, keyed by `+0x70 + 4·key`.

`FUN_0041a340(sector, req28, req2c, req30, forbid28, forbid2c, forbid30,
key, order)` (-> `FUN_00431830`) lists that sector's system records the
same way over `+0x28`, `+0x2c`, `+0x30`, keyed by `v[key]`.

Order 1 sorts ascending, order 2 descending, with a coin flip on equal
keys (`FUN_0041c250`). `FUN_0041c230` ranks from 1, and `FUN_004357b0`
takes the highest rank sum over all the lists it merges
(`ai-mission-planning.md`, "Picking"). Ported as `ai_records::Galaxy::{sector_query,
system_query}`, `ranked` and `pick`.

**By distance** (read 2026-10-14). `FUN_00419b80(origin, req24, req28,
req2c, forbid24, forbid28, forbid2c, order)` walks `origin` up its
containers (`+0x1c`) to a sector (`0x80..0x8f`); with none it returns no
list. It then takes that sector's record (`FUN_0041bd90`) and lists every
sector record passing the masks, keyed by the squared distance
(`FUN_0041b7b0`: `dx*dx + dy*dy` over packed shorts, 32-bit) between the
records' `+0x40` positions. `FUN_0041a400(sector, origin, req28, req2c,
req30, forbid28, forbid2c, forbid30, order)` takes `sector`'s record and
calls `FUN_00431c00`. That walks `origin` up to a system (`0x90..0x97`),
takes its record (`FUN_00403d30`), and lists the sector's system records
passing the masks, keyed by the squared distance between `+0x34`
positions.

Module 1 asks with order 2 (farthest first) and reads the list through
`FUN_004357b0`. That takes the highest place, the last entry, so it
gets the nearest. Ported as `Galaxy::{sectors_by_distance,
systems_by_distance}`, with the origin given as its system. The record
positions are `SectorRecord::position` and `SystemRecord::position`.

## Fleet records (`FUN_00432e40`, `FUN_00433620`, `FUN_00433780`)

The constructor stores the record's side at `+0x20`, the fleet's owner at
`+0x24`, and `+0x38` bit 4. A refresh keeps `+0x38 & 0xf000000c` and zeroes
`+0x50..+0xc4`. It sets bit 1 when the owner is the record's side and bit 2
when the fleet travels. Then it adds in each capital ship's record
(`FUN_00484630`): hull, attack (class `+0x30` scaled by the weapon recharge
left, `FUN_004738d0`), shields, fighters, regiments and people aboard.
Fleet bits `0x4000` (a standing active ship), `0x8000`, `0x20000`/`0x40000`
(fighters/regiments aboard), and counts at `+0x50..+0xbc`; `0x80000`/
`0x100000` when troop or hangar room is left. Ported as
`ai_records::FleetRecord`.

## Open

- The thresholds the system record reads through `+0x40` (the
  controller's `+0x138` object, `+0x24`/`+0x28`); no writer is found.
- System object `+0x50` bit 3, and the ship slot `+0x34(1)` test.
