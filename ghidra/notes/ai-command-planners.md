# The AI's movement planners (module `0xc`)

First read 2026-11-20 from the REBEXE decompile corpus
(`~/Games/rebexe-decompile/decompiled-typed`) and the vtables read from
`REBEXE.EXE` (image base `0x400000`): `FUN_004c8260`, `FUN_004c84a0`,
`FUN_004c84d0`, `FUN_004c85b0`, `FUN_004c8630`, `FUN_004c86b0`,
`FUN_004c8740`, `FUN_004c87c0`, `FUN_0042efe0`, `FUN_0042ef50`,
`FUN_0042ef90`, `FUN_0041b5d0`, `FUN_0041b600`, `FUN_004e57e0`,
`FUN_004eaaa0`, `FUN_004e5870`, `FUN_004bba10`, `FUN_004bbd60`,
`FUN_004bba90`, `FUN_004bbb00`, `FUN_004bbba0`, `FUN_004bbc50`,
`FUN_004bbf50`, `FUN_004bc040`, `FUN_004bc180`, `FUN_004bc290`,
`FUN_004bc410`, `FUN_004bc480`, `FUN_004bc520`, `FUN_004bc5d0`,
`FUN_004bc6d0`, `FUN_004bca20`, `FUN_0047a3c0`..`FUN_0047a870`,
`FUN_00479a40`..`FUN_00479fd0`, `FUN_00476cf0`, `FUN_00476d10`,
`FUN_00476de0`, `FUN_00478140`, `FUN_00479520`, `FUN_004b9290`,
`FUN_004b72d0`, `FUN_00401d20`, `FUN_00402230`, `FUN_0041b7e0`,
`FUN_00417fa0`. Read 2026-11-20 for the second stage: `FUN_004bc480`,
`FUN_004bc410`, `FUN_004bc520`, `FUN_004bc5d0`, `FUN_004bc6d0`,
`FUN_0047a050`, `FUN_004bca20`, `FUN_004bc040`, `FUN_004bc180`,
`FUN_0047a000`, `FUN_0047a3c0`, `FUN_00479f70`, `FUN_0042ef90`,
`FUN_00476cf0`, `FUN_00476d10`, `FUN_00476de0`, `FUN_00476e50`,
`FUN_00479520`, `FUN_00478140`, `FUN_004add40`, `FUN_004b1650`,
`FUN_004b16d0`, `FUN_004b1690`, `FUN_004b8b30`, `FUN_004b9290`,
`FUN_004b72d0`, `FUN_004b1800`, `FUN_00479560` (with its disassembly),
`FUN_004799d0`, `FUN_004adde0` (with its disassembly), `FUN_00432120`,
`FUN_00431f40`, `FUN_0041a460`, `FUN_0041b2b0`, `FUN_00477190`.

## What the module is

The audit tracker calls it "Moduł 0xc (ruchy flot)", and the port ran its
troop heuristic as module `0xc`. Neither is the original's module: module
`0xc` moves no fleet and no regiment. Its planners each hold one
**character** and post it somewhere, giving it a command there: the
fleet moves are module 8's sector plan objects (`ai-sector-plans.md`) and
the regiment moves module 6's (`ai-production-modules.md`).

`FUN_004c8260` builds the module (vtable `0x65c6b8`; base
`FUN_004e4fb0(this, 0xc, side)`): `+0x38..+0x4c` zero and a list at
`+0x50`. Its step (`FUN_004c84a0`) runs `FUN_004c84d0` for side 1 (the
step's `FUN_005f4960() == 1` guard is the module's `+4`, true for both
AI players in practice).

## The cycle (`FUN_004c84d0`, `+0x3c`)

| State | Function | Next |
|---|---|---|
| 0, other | | 1 |
| 1 | `FUN_004c85b0`: the strategy's requests | 2 |
| 2 | `FUN_004c8630`: from controller `+0xd8` (module 8's sector planners), cursor `+0x44`, the first with `+0xe0` or `+0xe4` set (`FUN_00476cf0`) into `+0x40` | 3 found, else 4 |
| 3 | `FUN_004c86b0`: that sector planner's requests | 4 |
| 4 | `FUN_004c8740`: from the list, cursor `+0x4c` (else the first), the first planner whose slot `+0x10` is 1 into `+0x48`, the one after it into `+0x4c` (0 at the end) | 5; none: 0, complete |
| 5 | `FUN_004c87c0`: slot `+0x14` (one state) of planner `+0x48`, with an out flag | see below |

State 5: an order completes the module and, while the planner's step is
not over (out 0), raises its priority `+0x34`; the planner's end (out 1)
completes the module and returns to state 0. A step with neither leaves
the module current, so the scheduler steps it again at once. So a cycle
runs exactly one planner, through its whole step, and gives at most one
order (the last state's).

**The strategy's requests** (`FUN_004c85b0`): controller `+0x354`, a
planner id, is cancelled (`FUN_0042ef90`: a sector planner's planner is
first taken off its owner, `FUN_00476de0`; every one gets `+0x20`
`0x80000000`); controller `+0x358`, a type, is made (`FUN_0042ef50` ->
`FUN_0042efe0`, added to the list) and, when the new planner's slot
`+0xc` returns 2 (the strategy's own), counted:
`FUN_0041b5d0` adds one to controller `+0x35c + 4 * slot +0x28` (type 1
index 0, type 6 index 1). Both words are then zeroed. **Nothing ever
takes one off `+0x35c`/`+0x360`**: the save routine `FUN_00417fa0`
writes them, and `FUN_0041b5d0` is their only other writer (the mission
planners' `+0x320..` have `FUN_0041b5a0` for that; this one has none).

**A sector planner's requests** (`FUN_004c86b0`): `+0xe4` (a held
movement planner's id) is cancelled as above; `+0xe0` (a type) is made
and, for slot `+0xc` 1, attached to the sector planner
(`FUN_00476d10`: `+0xe0` cleared, the planner id into its list, its
count `+0xc8 + 4 * FUN_00479520` raised: type 3 index 0, 4 index 1, 5
index 2; the planner's `+0x54` = the sector planner's id; slot `+0x20`
with the sector planner's `+0x44`).

## The planner types (`FUN_0042efe0`)

| Type | Ctor (size) | Vtable | Base | Slot `+0xc` | `+0x18` | `+0x1c` order | `+0x50` caps |
|---|---|---|---|---|---|---|---|
| 1 | `0047a430` (`0x4c`) | `0x65a490` | `004bba10` | 2 (strategy) | `0x40f340`: 1 | `0x263` General | (query `0x4000000`) |
| 3 | `0047a3c0` (`0x5c`) | `0x65a460` | `004bc290` | 1 (sector) | 3 | `0x262` Admiral | `0x1000000` |
| 4 | `0047a000` (`0x5c`) | `0x65a430` | `004bc290` | 1 | 4 | `0x263` General | `0x4000000` |
| 5 | `00479f70` (`0x5c`) | `0x65a400` | `004bc290` | 1 | 5 | `0x261` Commander | `0x2000000` |
| 6 | `00479a60` (`0x4c`) | `0x65a3c0` | `004bba10` | 2 | 6 | none (`0x6158b0`) | (query `0x402`) |

Fields (`FUN_004bbd60`): `+0x18` id, `+0x1c` step state, `+0x20` flags,
`+0x24` the side, `+0x28` target, `+0x2c` member, `+0x40`/`+0x44` from
the creator. Types 3 to 5 add `+0x48` a system, `+0x4c` the priority
the member's record takes (`+0x2c`), `+0x50` the caps bit, `+0x54` the
owning sector planner.

The record caps (`FUN_00401d20`): a character that is not one of the six
main characters gets `+0x34` bit `0x1000000` when its class may be an
Admiral (`FUN_004ed1c0`, class `+0xa8`), `0x2000000` a Commander
(`FUN_004ed1e0`, `+0xac`), `0x4000000` a General (`FUN_004ed200`,
`+0xb0`). The leader `0x34000280` also gets `+0x30` bit `0x400` (the
Emperor), under the same controller `+4` bit 0 test as the leader bit.
The `+0x30` bits `0x4000000`/`0x1000000`/`0x2000000` are the command held
(`FUN_00402230`, `+0x96` 1/2/3).

### Types 1 and 6: the strategy's posts (base step `FUN_004bbb00`)

States: 1, cancelled (`0x80000000`): `FUN_004bbba0`, end; else 3. 3:
slot `+0x34` (target), then 2. 2: slot `+0x38` (member); 0 ends, else 4.
4: `FUN_004bbc50`, end. The end resets the state to 0.

- **Ready** (slot `+0x10`, `FUN_004bba90`): not while the member's record
  has `0xa0` (en route, on a mission).
- **Target** (slot `+0x34`, type 1 `FUN_0047a480`, type 6
  `FUN_00479ad0`): clear bit 2; keep the target (bit 2) while its system
  record has `+0x30` `0x10000000` (type 6: `0x40000000` and 1), else slot
  `+0x2c` and choose (slot `+0x30`); a system chosen sets bit 2.
  - Type 1 (`FUN_0047a870`): `FUN_00419a00(0x40000000, 0, 0x10000000, 0,
    0, 0, key 0x26, order 1)`, then `FUN_0041a340(sector, 0x80000000, 0,
    0x10000000, 0, 0, 0x20000000, key 0x21, order 1)`; with no system, the
    same without `0x40000000` and `0x80000000`.
  - Type 6 (`FUN_00479e30`): `FUN_0041a1d0(0x40000000, 0, 0x40000000, ...,
    order 1)` (random keys) and `FUN_0041a460(sector, 0x80000000, 0,
    0x40000000, 0, 0, 0, order 1)`: Coruscant.
- **Release of the target** (slot `+0x2c`): type 1 `FUN_0047a850` clears
  the target record's `+0x30` `0x20000000`; type 6 none.
- **Member** (slot `+0x38`, type 1 `FUN_0047a5b0`, type 6 `FUN_00479c00`):
  clear bit 1; a member whose record is this planner's (`+0x28`) is kept
  (bit 1) while its `+0x30` lacks `0x50` (type 6 also needs `0x400`),
  else released (`0x800000` set, `+0x24`/`+0x28`/`+0x2c` cleared) and the
  target given up (slot `+0x2c`). Without a member: a target whose record
  is missing or already `0x20000000` (type 6: not the side's) is
  dropped; else the query, taken best first until one passes
  `FUN_004bc180` (not standing at a blockaded system), which is held
  (`0x800000` cleared, `+0x28` = id, `+0x2c` = 0, `FUN_004021b0`), and
  type 1 sets the target's `0x20000000`.
  - Type 1: `FUN_00403460(0x800002, caps 0x4000000, forbid 0x4f4000f1, 0,
    skill 6 leadership, 50, 10000, order 1)` merged with
    `FUN_00403750(target, 0x800002, 0x4000000, 0x4f4000f1, 0, order 2)`.
  - Type 6: `FUN_00403460(0x402, 0, 0x74000f1, 0, 6, 0, 10000, order 1)`.
- **Order** (`FUN_004bbc50`): a member whose record lacks `0x74000f1`
  (inactive, injured, en route, prisoner, on a mission, mandatory, or
  holding any command): at the target (`+0x38` == target) and with no
  command, slot `+0x18` not 6 gives slot `+0x1c`'s order
  (`FUN_004bbf50`: the Command order on the member); elsewhere a Move
  `0x201` to the target (`FUN_004bc040`).
- **Release** (`FUN_004bbba0`): out of the list (`FUN_0041b600`); a member
  still its own gets `0x800000` and, holding a command and not `0xa0`,
  the order `0x260` (no command); slot `+0x2c`; member cleared.

### Types 3, 4 and 5: a sector planner's officers (base step `FUN_004bc480`)

States: 1, cancelled: `FUN_004bc520` (as `FUN_004bbba0`, without slot
`+0x2c`), end; else 3. 3: `FUN_004bc5d0`, then 2. 2: slot `+0x28`
(member; `FUN_004bc6d0` types 3, 5, `FUN_0047a050` type 4), 0 ends, else
4. 4: `FUN_004bca20`, end.

- `FUN_004bc5d0`: from the owner (`+0x54`): target `+0x28` = the sector
  planner's `+0x38` (its flagship, a capital ship `0x14..0x1b`), `+0x48`
  = its `+0x3c` (a system); bit 2 when both are so.
- Member (`FUN_004bc6d0`): bits 1, 4, 8 and `0x10` cleared (`& ~0x1d`);
  a member still its own and without `0x50` is kept (bit 1), bit 8 when
  its place (record `+0x38`) is the target and `0xc` when it also holds a
  command; else released. Without one, the owner's query
  `FUN_00403460(0x800002, +0x50, 0x4f4000f1, 0, 6, 50, 10000, 1)` merged
  with `FUN_00403750` from `+0x48`, or without a station from the owner's
  sector's first system (`FUN_00432120(sector record, 0)` reads the
  record's system list `+0x48`, index 0; not the flagship's system, as
  first read), order 2; the pick held with record `+0x28` = the planner's
  id and `+0x2c` = `+0x4c`. Type 4 (`FUN_0047a050`) forbids `0x474000f1`:
  a General reserved for research or diplomacy (`0x8000000`) is admitted.
- `+0x4c` (slot `+0x20`, `FUN_004bc410`): the owner's plan id
  (`FUN_00476d10` passes the sector planner's `+0x44`); the held member's
  record `+0x2c` takes it too.
- Order (`FUN_004bca20`): a free member (no `0x74000f1`) aboard the
  flagship without a command takes slot `+0x1c`'s order (bit 4). A
  target not a ship: move to `+0x48` unless there (bit `0x10`). Else the
  member goes to the flagship when they share a fleet, or a system, and
  the flagship lacks `+0x24` `0x800` (bit 8); to `+0x48` when the systems
  differ; waits (bit `0x10`) when at `+0x48`.
- The sector planner's side (`FUN_00478140`, each cycle): `+0xe0`/`+0xe4`
  cleared, `+0x60` `0x7000000` cleared; each held movement planner sets
  `+0x60` `0x2000000` and `+0x64` `0x40000000`, flag `0x10` gives `+0x60`
  `0x4000000`, flag 8 without 4 gives `0x1000000`, with 4 and type 4
  `0x200000`; flag 1 gives `+0x64` `0x80000000`, else `0x400`. Then the
  plan's slot `+0x1c` fills `+0xe0` (kinds 1 to 3 `FUN_004b9290`: type 3
  while `+0xc8` < 1 and it holds a fleet, else type 5 while `+0xd0` < 1,
  `+0x98` > 0 and the sector has `+0x2c` `0x40008`; kinds 4 and 6
  `FUN_004b72d0`: type 4 first while `+0xcc` < 1, `+0x94` > 0 and `+0x2c`
  8), `+0x64` `0x200` with one, and `+0xe4` the first held planner
  `FUN_004add40` gives up. With `+0x60` `0x2000000`, `FUN_00479560` keeps
  or chooses the flagship (`+0x38`, record `+0x24` `0x1000000`).
- The counts `+0xc8 + 4 * FUN_00479520(type)` rise in `FUN_00476d10` and
  fall in `FUN_00476de0` only (`FUN_00477190` zeroes `+0x6c..+0x90`, not
  them), and `FUN_00476de0` runs for every cancellation of a planner with
  an owner (`FUN_0042ef90`), so a count is the planners of that type
  naming the sector planner in `+0x54`.
- `FUN_004add40` (give up, for `+0xe4`): type 3 `FUN_004b1650` (two or
  more held, or one with no fleet held); type 4 the plan's slot `+0x4c`
  (kinds 4 and 6 `FUN_004b8b30`: more than one; the others
  `FUN_004b1690`: any; both need the sector record); type 5
  `FUN_004b16d0` (more than one, or any with `+0x98` 0); a planner not
  found, 1.
- `FUN_00479560` with `0x2000000` (the lists from the disassembly: the
  new pick walks the held fleets `+0x2c`, the last loop the held ships
  `+0x24`): a flagship still in `+0x24`, the planner's (`+0x30`) and
  marked keeps the post unless the ship has `0x2000000` (passed over,
  which takes the mark off and chooses anew). Kept and out of hyperspace
  (`0x800`), the best ship of its fleet (`FUN_0041ac80(fleet, 0x4001,
  0x2803800, key 3, order 1)`) takes the mark when its `+0x50` is higher
  or the flagship lacks bit 1; with no such ship, the best with `0x4000`
  when its `+0x50` is higher. Else the best ship (`0x4000`, forbidding
  `0x2803800`) of each held fleet, the first strictly highest `+0x50`
  above 0, is marked. Then every held ship loses `0x2000000`.
- `FUN_004799d0` with `0x2000000`: the plan's slot `+0x10`, every kind's
  `FUN_004adde0`, on `+0x3c`: the base (`+0x40`) takes the post when its
  record is the side's (`+0x30` 1) and neither in uprising nor blockaded
  (`+0x28 & 3`); a station other than the base failing that test is
  cleared; with none, `FUN_0041a460(sector, req +0x28, req +0x2c, req
  +0x30, forbid +0x28, forbid +0x2c, forbid +0x30, order 1)` with
  `(0x80000000, 0, 1, 3, 0, 0)`, then `(0x3e00000, 0, 1, 3, 0, 0)`, then
  `(0, 0, 1, 3, 0, 0)`, then `FUN_0041b2b0(sector, 0, 1, 0, 3, 0, 0)` (as
  pushed at `0x4ae0f7`: `+0x2c` 1, not `+0x30` 1). Without `0x2000000`
  `+0x3c` is cleared.
- `FUN_004b06b0` (the plan object's state 6) leaves aboard a person of
  the side's whose record `+0x28` names a planner in the sector planner's
  list `+0xd4` (`FUN_00476e50`, `ECX` = plan `+0x10`, `+0xd4`); a person
  held by another planner, or by none, goes down with the rest.

## The strategy's side (module `0xd`)

- **State 4** (`FUN_004e57e0`, slot `+0x1c` = `FUN_004eaaa0` in every
  strategy class): a type 1 while `+0x35c` < `+0x220`, else a cancel while
  `+0x220` < `+0x35c`. The Empire (`+4 == 2`) asks for a type 6 when it
  asked for nothing else, side `+0xa4` (its MNCHARSD classes) is 0,
  controller `+4` has `0x80` and `+0x360` is 0; it cancels when it did not
  already, `+0xa4` > 0 or no `0x80`, and `+0x360` is 1. A type goes to
  `+0x358`; a cancel names in `+0x354` the first planner of the list whose
  slot `+0xc` is 2 and `+0x20` lacks bit 2, or 0.
- `+0x220` sums the sector records' `+0x188` (`FUN_0041b7e0`): their
  systems with record `+0x30` `0x10000000` (`FUN_00432350`, which also
  sets the sector's `+0x2c` `0x10000000` for one without `0x20000000`).
  That bit is shared: module `0xd`'s state 6 and the production modules'
  site claims (`FUN_004dd120`) both set it.
- Controller `+4` `0x80`: a sector record with `+0x2c` 4 and `0x200`
  (Coruscant held, `FUN_0041b7e0`).
- **State 5** (`FUN_004e69d0`) keeps planners of another kind
  (`FUN_00474070`, vtable `0x65a3a0`; module 9's, `ai-module-9.md`) at `+0x224` (sector
  `+0x1a0`), not module `0xc`'s as `ai-sector-plans.md` first read it.
- **State 6** (`FUN_004e5870`): system `0x90000109` (Coruscant) record
  `+0x30` takes `0x10000000` while it has bit 1, else loses it. The
  Alliance also: controller `+0x28`'s system keeps it while the side's
  headquarters (`+0x30` 1 and `0x80000000`), else loses it and `+0x28` is
  cleared; with no `+0x28`, `FUN_00419a00(0, 0, 0x80000000, ..., key 3,
  order 1)` and `FUN_0041a340(sector, 0, 0, 0x80000000, ..., key 0, order
  1)` find the headquarters, which takes the bit and `+0x28`.

## Ported (2026-11-20, save v129)

`crates/rebellion-core/src/ai_module_c.rs`: `ModuleC` (the cycle, the list,
the strategy's `+0x354`/`+0x358`, the counts `+0x35c`/`+0x360` kept as
the original keeps them, controller `+0x28`), types 1 and 6
(`CommandPlanner`), module `0xd`'s states 4 and 6 (`strategy_requests`,
`guard`), and `Planner::pick_officer` (`mission_planning.rs`) for the
member queries. Orders go through the validated paths: `AIAction::Command`
(new, `character_command::give`, checked by `order_allowed`) and the Move
pseudo-mission (`AIAction::DispatchMission` of `MissionKind::Move`). The
port's troop heuristic no longer runs as module `0xc`. The record gains
the Emperor's `0x400`. `SaveState::ai_command_planners` (v129, the last
field) carries the module, since `AIState` sits mid-body; v116 to v128
saves load with none.

port:
- Types 3, 4 and 5 and the sector planners' side were ported in the
  second stage (below).
- A member's record is the port's; an injured character has `0x10`
  (`FUN_00402230`, ported with the mission records), so an injured
  member is let go.
- hyp: `FUN_00403750` walks a record's location up to a system; a
  character aboard a fleet is measured from the fleet's system, and one
  with none is left out of the merged pick.
- hyp: `FUN_004ece80` after a refused target in `FUN_0047a5b0` clears the
  target (`+0x28`), as its other uses clear the field just read.

**Two Generals at one post (as read; to confirm with A0).** Kept as
read (decision 2026-11-20): the port does not mark a retarget. A type 1
planner that loses its target (the guard bit goes, for instance when a
production module moves its site claim) chooses another (slot `+0x30`)
but keeps its member, and nothing then sets the new target's
`0x20000000`: only a new member's pick does (`FUN_0047a5b0`). A second
planner can so choose the same system and post its own General there.
Each General standing there without a command takes `0x263`, and
`FUN_00561bb0` strips the other's, so the two trade the command every
cycle, an order each time. The port does the same: in the campaigns
below a few pairs give most of the Command orders (seed 2: 7,817).

Campaigns (`rebellion-playtest data/base --dual-ai --ticks 5000`, seeds 1
to 5, on main `c8539650` before and after):

| Seed | Before: end, A/E systems, most fleets | After: end, A/E systems, most fleets | Regiment Moves before/after | Command orders |
|---|---|---|---|---|
| 1 | day 5000, 27/26, 34 | day 5000, 17/36, 37 | 6,781 / 902 | 2,253 |
| 2 | day 5000, 17/29, 32 | day 5000, 24/22, 35 | 5,412 / 1,139 | 7,817 |
| 3 | day 2877 Empire (Death Star), 16/26, 23 | day 2548 Alliance (Coruscant taken), 19/23, 32 | 3,187 / 875 | 19 |
| 4 | day 3919 Empire (Death Star), 17/35, 18 | day 5000, 52/0, 41 | 6,385 / 2,258 | 1,625 |
| 5 | day 5000, 53/0, 51 | day 5000, 26/27, 21 | 6,487 / 1,663 | 830 |

The regiment moves left are module 6's; the troop heuristic gave the
rest. Each run ends in a victory or reaches day 5000; seed 4 after and
seed 5 before end with the Empire holding no system and no victory
declared, so that state is not new.

## Ported (2026-11-20, save v130): the sector planners' officers

`ai_module_c.rs`: the module's states 2 and 3 (`find_request`,
`sector_requests`: `FUN_004c8630`, `FUN_004c86b0`, `FUN_00476d10`), the
cancellation (`cancel`, `FUN_0042ef90` -> `FUN_00476de0`), types 3, 4
and 5 (`step_officer`: `read_owner` `FUN_004bc5d0`, `officer`
`FUN_004bc6d0`/`FUN_0047a050`, `board` `FUN_004bca20`) and the counts
(`officers_of`). The strategy's cancellation now names only its own
types 1 and 6 (slot `+0xc` 2). `ai_planner.rs`: `movement`
(`FUN_00478140` with the flags, `+0xe4` and the counts), `flagship`
(`FUN_00479560` with `0x2000000`) and `station` (`FUN_004799d0` ->
`FUN_004adde0`). `ai_module8::reset` cancels the planner's officers
(`FUN_00477190`), and `ai_plan.rs`'s `disembark` leaves the held officers
aboard (`FUN_00476e50`). Kind 6's report (`FUN_004b4d90`) now reads a
planner `+0x60` `0x200000` that a General aboard the flagship with a rank
sets. A Move to the flagship is the new `AIAction::BoardFleet`, carried
out as the player's Move to a fleet (`MissionState::set_target_fleet`).
`SaveState::ai_sector_links` (v130, the last field) keeps each
planner's `+0x54`, `+0x28` and `+0x48` and the module's `+0x40`/`+0x44`,
which `ModuleC`'s own encoding skips; v129 saves load with none (their
planners are all the strategy's).

port:
- A character's place is its fleet, not one of the fleet's ships: the
  port seats a fleet's people itself (`missions::seat_characters`) and a
  Move takes a character to a fleet. A member aboard the flagship's
  fleet stands at the flagship (bit 8), and one aboard another ship of
  that fleet is not sent to it.
- The member's record `+0x2c` (the owner's plan id, `FUN_004bc410`) has
  no port counterpart.
- The counts `+0xc8..+0xd0` are the planners naming the sector planner,
  which is what `FUN_00476d10` and `FUN_00476de0` keep them at.

Campaigns (`rebellion-playtest data/base --dual-ai --ticks 5000`, seeds 1
to 3; before = `60d2e20a`, after = this stage):

| Seed | Before: end, A/E systems, most fleets | After: end, A/E systems, most fleets | Command orders before/after | Move to a flagship |
|---|---|---|---|---|
| 1 | day 5000, 26/27, 35 | day 5000, 33/20, 55 | 2,644 / 306 (139 General, 94 Admiral, 65 Commander, 8 none) | 48 |
| 2 | day 5000, 22/24, 39 | day 5000, 26/20, 43 | 1,209 / 663 (378, 201, 77, 7) | 53 |
| 3 | day 5000, 20/22, 28 | day 3667, Alliance (Empire headquarters taken), 34/8, 38 | 2,293 / 1,294 (1,145, 85, 49, 15) | 79 |

Before, every Command order was a type 1 General's (`0x263`); after,
the sector planners post Admirals (`0x262`) and Commanders (`0x261`) as
well, and fewer Command orders are given in all (why the type 1 counts
fall was not traced). No run ended in a victory before; after, seed 3
ends on day 3667.

Two faults found in the campaign checks and fixed before these numbers:
- the member queries' forbidden masks were first typed `0x04f400f1` and
  `0x047400f1` for `0x4f4000f1` and `0x474000f1`; the first forbids
  `0x800000` (not held), so types 3 and 5 never found an officer (no
  Admiral or Commander order in three runs). A test of each type's
  capability and of the reserved bit now pins the masks.
- without the `FUN_00476e50` test in `disembark`, the plan object's
  state 6 set the boarded General down again each day (seed 2: 1,973
  Moves to a flagship).

Mutants (`cargo mutants --in-diff` on the touched files): core 302,
data 24; all caught but three equivalent ones: `|` -> `^` in the bits
`officer` clears (disjoint constants), `count(5) > 0` -> `>= 0` in the
type 5 give-up (only held type 5 planners are tested, so the count is at
least 1), and the test helper `planners_tail` -> 1 (the legacy bincode
decoder takes trailing bytes).
