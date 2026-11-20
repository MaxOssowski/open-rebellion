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
`FUN_00417fa0`.

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
- Member (`FUN_004bc6d0`): bits 1, 3, 4 cleared; a member still its own
  and without `0x50` is kept (bit 1), bit 8 when at the target and `0xc`
  when it also holds a command; else released. Without one, the owner's
  query `FUN_00403460(0x800002, +0x50, 0x4f4000f1, 0, 6, 50, 10000, 1)`
  merged with `FUN_00403750` from `+0x48` (or the flagship's system,
  `FUN_00432120`), order 2; the pick held with `+0x2c` = `+0x4c`. Type 4
  (`FUN_0047a050`) forbids `0x474000f1`: a General reserved for research
  or diplomacy (`0x8000000`) is admitted.
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
  (`FUN_00474070`, vtable `0x65a3a0`; hyp: module 9's) at `+0x224` (sector
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
- Types 3, 4 and 5 and the sector planners' side (`FUN_004c8630`,
  `FUN_004c86b0`, `FUN_00478140`'s flags, `FUN_00479560` with
  `0x2000000`) are not ported: states 2 and 3 find no request, and the
  sector planners' counts `+0xc8..+0xd0` stay 0.
- A member's record is the port's (no injuries: `0x10` is never set).
- hyp: `FUN_00403750` walks a record's location up to a system; a
  character aboard a fleet is measured from the fleet's system, and one
  with none is left out of the merged pick.
- hyp: `FUN_004ece80` after a refused target in `FUN_0047a5b0` clears the
  target (`+0x28`), as its other uses clear the field just read.
