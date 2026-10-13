---
title: "Side resource monitors and the per-side resource pools"
description: "What the command center's Raw Materials, Refined Materials and Maintenance monitors read, and the side-object fields, setters and production hooks behind them"
category: ghidra
created: 2026-10-08
updated: 2026-10-13
tags: [economy, resources, maintenance, mine, refinery, command-center, readout]
---

# Side resource monitors

Decompiled with Ghidra 12.0.2 from a fresh import of the GOG `REBEXE.EXE`
(auto-analysis, no extensions), in a scratch project. Status: partial
recovery. Open items are listed at the end.

## The three readouts

`FUN_00422ce0` lines 185-258 create three 63x17 text objects at Alliance
(265,16), (360,16), (457,16) and Empire (175,19), (273,19), (371,19), with
font entry 8, `DrawTextA` format `0x26` (right, vertically centred, single
line) and the faction text color. Their tooltips are TEXTSTRA 5392 "Raw
Materials Monitor", 5393 "Refined Materials Monitor" and 5394 "Maintenance
Monitor".

`FUN_00422620` fills them through `FUN_00429200(command, value)`:

| Command | Readout | Value |
|---|---|---|
| `0x14c` | Raw materials (object `+0x178`) | side `+0x78` |
| `0x14d` | Refined materials (object `+0x1ac`) | side `+0x7c` |
| `0x14e` | Maintenance (object `+0x1e0`) | side `+0x58` (first int, via `FUN_0041bde0`) minus side `+0x74`; above 99999 it shows TEXTSTRA 4866 "MAX" |

The side object is `FUN_004f3dd0(side)`, id family `0xf3`.

## Side-object fields

| Field | Setter | Meaning |
|---|---|---|
| `+0x58/+0x5c` pair | `FUN_0052ec90` (notify `+0x1bc`) | Maintenance capacity (total, allocated). `FUN_005323c0` and `FUN_00532520` set it to the element-wise minimum of the `+0x60` and `+0x68` pairs. |
| `+0x60/+0x64` pair | `FUN_0052ed50` (notify `+0x1c0`) | Sum of the side's completed, unlocked mines' (`0x2c`) `+0x64` pairs (`FUN_0052fff0`). |
| `+0x68/+0x6c` pair | `FUN_0052ee10` (notify `+0x1c4`) | Same for refineries (`0x2d`). |
| `+0x70` | `FUN_0052eed0` | Maintenance requirement. `FUN_0052fff0` sets it and `+0x74` to the sum of `FUN_004f2990` (class maintenance cost, `FUN_0053b870`) over every object of the side with `+0x50` bit 6. |
| `+0x74` | `FUN_0052ef30` | Maintenance used (same sum). |
| `+0x78` | `FUN_0052ef90` | Raw materials on hand. |
| `+0x7c` | `FUN_0052eff0` | Refined materials on hand. |
| `+0x80`, `+0x84` | | Refineries and builders waiting for raw / refined material (`FUN_0052fbf0`, `FUN_0052fd30`). |
| `+0x90/+0x94/+0x98` | `FUN_0052f130/1a0/210` | Per-system list sums, timer `0x381` (`FUN_00530460`). |

`FUN_0052f670` raises the side's over-limit flag (`FUN_0052f3d0`) when
`+0x70` or `+0x74` exceeds `+0x58`; timer `0x382` (`FUN_00530350`) then
locks one random completed object. This is the manual's "maintenance below
zero" scrapping (manual p. 81).

`FUN_0052f6b0` and `FUN_0052f8f0` rebalance the allocated halves of the mine
and refinery pairs toward `+0x70` / `+0x74`, one facility at a time, through
`FUN_0055a820` (grow) and `FUN_0055a960` (shrink). Facility pairs are set by
`FUN_0055a6e0`; `FUN_0055ab60` zeroes the allocated half of a facility that
is incomplete, locked or contested.

## Setter mechanics (2026-10-13)

Re-decompiled through the REA MCP server (Ghidra 12.1.4, default
auto-analysis) from the same GOG `REBEXE.EXE`, sha256
`b3fe3997cab9a6e96403d638875dcba25484e4d8601751afec748471ac0ed6ab`.

| Resource | Field | Setter | Mirror routine | Notifier |
|---|---|---|---|---|
| Raw materials | `+0x78` | `FUN_0052ef90(value, ctx)` | `FUN_00531a70` | `+0x1d0` |
| Refined materials | `+0x7c` | `FUN_0052eff0(value, ctx)` | `FUN_00531af0` | `+0x1d4` |
| Maintenance capacity | `+0x58/+0x5c` | `FUN_0052ec90(&pair, ctx)` | `FUN_005315e0` | `+0x1bc` |

`FUN_0052eed0` (`+0x70`, mirror `FUN_00531970`, notifier `+0x1c8`),
`FUN_0052ef30` (`+0x74`, `FUN_005319f0`, `+0x1cc`), `FUN_0052ed50` (`+0x60`
pair, `FUN_00531710`, `+0x1c0`) and `FUN_0052ee10` (`+0x68` pair,
`FUN_00531840`, `+0x1c4`) have the same shape. Each setter:

1. Returns 0 without writing unless `FUN_0053a000` holds:
   `(this+0x24 & 0x30) == 0`.
2. Writes only when the value differs. The pair setters compare both halves
   (`FUN_0051ca60`, the negation of `FUN_0051ca30`) and copy with
   `FUN_00520580`.
3. Calls the mirror routine on the objects in slots 1 and 2 of the table at
   `this+0x20` (`FUN_00539fd0`, `FUN_005844e0`; valid slots 0..2). The mirror
   runs only where `FUN_0053a010` holds (`+0x24 & 0x30 != 0`), copies the
   value from slot 0, and notifies through `+0x224` (raw), `+0x228` (refined)
   or `+0x210` (maintenance) when `FUN_004fd3d0` and `FUN_004fd600(side)`
   allow it.
4. Calls its own notifier with `(old, new, ctx)`.

No setter clamps; the bounds are in the callers.

**Callers.** These are all the resolved direct callers.

| Setter | Caller | Value set |
|---|---|---|
| `FUN_0052ef90` | `FUN_00530670` (mine cycle end) | `raw + 1`, when the facility's `+0x60` bit 2 is set and `FUN_0053a860(0, ctx)` succeeds |
| `FUN_0052ef90` | `FUN_0052fb30` (refinery input) | `raw - 1` when `raw > 0`; the out-flag reports whether a unit was taken |
| `FUN_0052eff0` | `FUN_005307e0` (refinery cycle end) | `refined + 1`, same two conditions as the mine |
| `FUN_0052eff0` | `FUN_0052fb80` (construction input) | `refined - 1` when `refined > 0` |
| `FUN_0052eff0` | `FUN_00530270` (scrap refund) | `refined + FUN_004f2980() / 2`, for types `0x14..0x16` with `+0x50` bit 2 |
| `FUN_0052ec90` | `FUN_005323c0`, `FUN_00532520` | see below |

`FUN_005323c0` and `FUN_00532520` decompile identically. Each reads the mine
pair (`FUN_004292e0`, `+0x60`) and the refinery pair (`FUN_00429300`,
`+0x68`) and sets total = min of the two totals, and allocated = min of the
two allocated halves, capped at that total. Neither has a direct caller;
their only references are the adjacent data words `0x660cb8` and `0x660cbc`.
That fits a side vtable based at `0x660af8` with the two functions in slots
`+0x1c0` and `+0x1c4`, the mine-pair and refinery-pair notifiers, so that a
change to either pair recomputes maintenance capacity. The side constructor
`FUN_0052d7b0` installs `0x00660af8`, which confirms the base.

In `FUN_0052fff0` a mine or refinery contributes its pair only with `+0x50`
bit 0 set and `+0x60` bit 0 clear; upkeep is summed for every object of the
side with `+0x50` bit 6, whatever its type.

## Production hooks

`FUN_00516360(object, ?, state)` runs on facility state changes (callers
`FUN_0051b7f0` at setup and timer handler `FUN_00578a40`):

| Facility | State 1 (cycle start) | State 3 (cycle end) |
|---|---|---|
| Mine `0x2c` | `FUN_00530650` | `FUN_00530670`: raw `+0x78` += 1 |
| Refinery `0x2d` | `FUN_005306b0`: take 1 raw (`FUN_0052fb30`), or queue a request | `FUN_005307e0`: refined `+0x7c` += 1 |
| Construction `0x28..0x2b` | `FUN_00530820`: take 1 refined (`FUN_0052fb80`), or queue a request | `FUN_00530950` |

`FUN_0052fb30` takes raw only when `DAT_006b90e0` is set; otherwise the
refinery proceeds without it. `FUN_0051c010(mode)` stores `mode` in
`DAT_006b2bb4` and sets `DAT_006b90e0 = (mode == 0x1a)`; `FUN_00513960`
returns the flag. Queued requests are retried by timer handlers
`FUN_00577a20` (raw) and `FUN_00577d40` (refined). Scrapping an object of
type `0x14..0x16` with `+0x50` bit 2 refunds half its refined cost
(`FUN_00530270`, `FUN_004f2980`).

## Day-0 maintenance survey (2026-10-08)

To test whether the manual's 50 per mine/refinery pair reproduces the
originals' opening values, the port's seeded worlds were measured with
capacity = 50 x min(own mines, own refineries) and upkeep = class
maintenance cost (`FUN_004f2990`) summed over the side's living ships,
fighter squadrons, regiments, special forces and facilities. Each row is 12
seeds (1..12) with the side as player.

Original opening values: squakenet 0119-0149 Alliance **296**, 0712-0732
Empire **162**; pravus 0121 Alliance **852**. Both squakenet games use the
small (Standard) galaxy.

| Galaxy | Difficulty | Alliance available | Empire available |
|---|---|---|---|
| Standard | Easy | -37 .. 340 (2 of 12 negative) | -287 .. 147 (9 of 12 negative) |
| Standard | Medium | 214 .. 786 | -155 .. 216 (6 of 12 negative) |
| Standard | Hard | 55 .. 695 | -249 .. 129 (8 of 12 negative) |
| Large | Medium | 359 .. 1218 | -29 .. 425 |
| Huge | Medium | 604 .. 1484 | 64 .. 968 |

Standard-galaxy Empire upkeep runs 321..487 against capacity 150..650. The
Alliance values are plausible; the Empire's are not: 23 of 36 Standard
seeds start below zero, where `FUN_0052f670` and timer `0x382` would begin
scrapping on the first day, while the original opens at 162. The model, or
the port's seeded Empire forces or facilities, is wrong somewhere, so the
Maintenance Monitor stays blank until the open items below close. The raw
and refined monitors stay blank too: the port has no per-side stockpiles.

## Diagnosis of the Empire result (2026-10-08)

Standard galaxy, Medium, side as player, seeds 1-3. Upkeep is the class
maintenance cost (`+0x4c`) per object, as `FUN_004f2990` reads it.

| | Empire 1 | Empire 2 | Empire 3 | Alliance 1 | Alliance 2 | Alliance 3 |
|---|---|---|---|---|---|---|
| Systems held | 4 | 4 | 4 | 7 | 7 | 7 |
| Mines / refineries | 10 / 12 | 14 / 10 | 7 / 17 | 14 / 31 | 18 / 21 | 22 / 28 |
| Capacity (50 x pairs) | 500 | 500 | 350 | 700 | 900 | 1100 |
| Orbital shipyards (13 each) | 20 = 260 | 19 = 247 | 14 = 182 | 22 = 286 | 26 = 338 | 15 = 195 |
| Other manufacturing | 30 | 20 | 30 | 20 | 20 | 10 |
| Defences | 30 | 21 | 31 | 26 | 54 | 86 |
| Capital ships | 71 (ISD) | 71 | 95 | 35 | 35 | 35 |
| Fighters | 30 | 27 | 18 | 8 | 8 | 8 |
| Regiments | 110 | 119 | 101 | 39 | 39 | 51 |
| Special forces | 14 | 13 | 5 | 6 | 6 | 4 |
| Death Star / HQ facility | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| Upkeep | 545 | 518 | 462 | 420 | 500 | 389 |
| Available | -45 | -18 | -112 | 280 | 400 | 711 |

Mines and refineries cost no upkeep (PROFACSD maintenance 0). No Death
Star exists on day 0, the Alliance Headquarters facility costs 0, and
Coruscant carries no separate HQ facility.

**What the upkeep sum counts.** `FUN_0052fff0` walks the global object
registry over families `0x10..0x3f` (`FUN_00505340`, `FUN_004f6010`, which
reach `FUN_0053f090`) and adds `FUN_004f2990` for every object of the side
with `+0x50` bit 6 set. Bit 6 is "existing", created and not destroyed
(`object-state-flags.md`), so objects still under construction count.
`FUN_004f2990` returns 0 when the object has no class (`+0x2c`). The range
covers regiments `0x10`, capital ships `0x14`, fighters `0x1c`, the Alliance
HQ `0x20`, defences `0x22..0x25`, manufacturing `0x28..0x2a`, mines and
refineries `0x2c..0x2d`, the Death Star `0x34` and special forces `0x3c`;
characters have no class cost. Nothing the port counts is exempt, and
nothing it skips is charged.

**Seeding.** The original facility seeder `FUN_00566de0` runs one
`FUN_00559850` decision per energy slot (`system+0x5c`): a mine with chance
`(raw - mines) x DAT_006bb4bc`, otherwise a 0-99 roll on SYFCCRTB/SYFCRMTB.
The loop stops only when a lookup or a creation fails, and a roll of 0 (no
facility) does not stop it. SYFCCRTB gives a non-mine core slot a 43%
chance of an Orbital Shipyard (`0x28000001`), so 6-7 shipyards on a
12-energy core system is what the original generates; they carry about
half of each side's upkeep. Squakenet 0714 and 0726 show the original
Empire holding 3 loyal systems and 1 under military control, like the
port's 4.

**Verdict.** The evidence fits a different capacity rule, not wrong
seeding or exempt units. Matching the originals needs about 62-89 capacity
per Empire mine/refinery pair but only 31-51 per Alliance pair (taking
upkeep + 162 and upkeep + 296 over min(mines, refineries)), so no constant
per-pair value can fit both sides. Unknown difficulty in the two
recordings and untraced callers of `FUN_00566de0` remain caveats.

## Parameter scan and the facility pair (2026-10-08)

A side- and difficulty-dependent parameter could explain the 2:1 ratio, so
every GNPRTB (213) and SDPRTB (35) entry was scanned for a value of 62..89
for the Empire and 31..51 for the Alliance at the same difficulty, or an
Empire:Alliance ratio of 1.6..2.4. GNPRTB columns were compared player
Alliance against player Empire. SDPRTB was compared within each player and
difficulty column set, and also across them (Alliance player's Alliance
value against Empire player's Empire value, since each recording measures
the player's side).

- No entry falls in both ranges.
- SDPRTB 5168..5170, the seeding maintenance budget, are percentages
  (15..38) with at most a 38:25 side ratio.
- Only three entries reach about 2:1 in some column: SDPRTB 5139 (player
  side always 400, AI side 100..200; loaded into `DAT_006bb640` and read
  only by `FUN_0055d5e0`, which returns it beside the constant `0x54`),
  SDPRTB 5170 (Huge galaxy only) and SDPRTB 7680 (control-bucket percentage,
  read by `FUN_00558bb0` in seeding). None can make the player's own side
  differ between the two recordings, and none is read near the facility
  pair.

Correction to the earlier reading: vtable `0x00662760` (constructor
`FUN_0055a100`) is the processing-facility class. Slots `+0x200..+0x218` are
its `ProcFacil` notifiers: `ProcFacilState`, `ETC`, `Suspended`,
`PointPresent`, `PointProcessed`, `Processing` and `OnStartupCycle`
(`FUN_0053b3e0`..`FUN_0053b610`), which only log. The `+0x64` pair that
`FUN_0055a6e0` sets notifies through `+0x204`, the ETC notifier, so it holds
the facility's processing points (present, processed) rather than a fixed
capacity. Its writers are the side's rebalancers `FUN_0055a820` and
`FUN_0055a960`, plus `FUN_0055ab60` and `FUN_0055b010`, which zero the second
half. The side's maintenance capacity (`+0x58`, the element-wise minimum of
the mine and refinery point sums) therefore depends on how processing is
allocated, not on a per-facility constant. That fits a side-dependent
result. The Maintenance Monitor stays blank.

## Facility capacity, allocation and cycle period (2026-10-13)

Same REA session as "Setter mechanics". Stubs Ghidra left undefined
(`0x52d100`, `0x52d540`, `0x52d1a0`, `0x52d5e0`, `0x55b1c0`, `0x55b1e0`) were
read as bytes and decoded by hand; `FUN_0055a820` and `FUN_0055abf0` were
checked in assembly.

**Classes.** The mine/refinery base is constructed by `FUN_0055a600`
(vtable `0x00662778`, with pure virtuals at `+0x214` and `+0x218`), not by
`FUN_0055a100` (`0x00662760`) as the correction above says. The mine is
`FUN_0052ced0` (vtable `0x006606a0`) and the refinery `FUN_0052d300`
(`0x006608c8`); both are `0x70` bytes. The side is `FUN_0052d7b0`, vtable
`0x00660af8`, which confirms `FUN_005323c0` and `FUN_00532520` as its
`+0x1c0` / `+0x1c4` notifiers.

**Facility fields.** `+0x64` capacity, `+0x68` allocated (the pair
`FUN_0055a6e0` sets), `+0x6c` ProductionModifier (default 100, setter
`FUN_0055a7a0`).

**Capacity is 50.** Vtable slot `+0x114` (mine `0x52d100`, refinery
`0x52d540`) calls `FUN_0055b0c0` -> `FUN_0053b140` -> `FUN_004f2b30` ->
`FUN_004fb490`, which forwards to slot `+0x88`, and on success sets the pair
to `(50, 0)` with `FUN_0051c900(0x32, 0)` and `FUN_0055a6e0`. This is the
only code that writes a non-zero capacity; every other writer adds
`(0, +/-n)` or zeroes the allocated half, and the constructor zeroes both.
The manual's 50 per mine/refinery pair (p. 81) is therefore exact: side
capacity `+0x58` is `50 x min(mines, refineries)` over facilities with `+0x50`
bit 0 set and `+0x60` bit 0 clear.

**Allocation.** `FUN_0052f6b0` (mines, against `+0x70`) and `FUN_0052f8f0`
(refineries, against `+0x74`) move the side's allocated total toward the
upkeep sum, bounded by the side's capacity. For each eligible facility of
the side:

- grow (`FUN_0055a820`): `n = side.allocated x fac.capacity / side.capacity
  - fac.allocated + 1`, clamped to `0..remaining` and to `fac.capacity -
  fac.allocated`;
- shrink (`FUN_0055a960`): `n = fac.allocated - (side.allocated - 1) x
  fac.capacity / side.capacity`, clamped to `0..remaining`.

The pass repeats while any facility can still take or give.

**Cycle period.** The pair notifier (`+0x204`, `0x55b1c0`) runs
`FUN_0055aa80` then `FUN_0055abf0`; the modifier notifier (`+0x208`,
`0x55b1e0`) runs `FUN_0055abf0`. `FUN_0055abf0` calls slot `+0x218`
(`0x52d1a0` for both classes), which is `FUN_0055e2b0(rate, pair, modifier,
flag, &out)` with `rate` = class `+0x5c` (`FUN_00520b70`) and `flag` = NOT
facility `+0x60` bit 4, and stores `out` in the timer record at facility
`+0x54`, field `+0x20` (`FUN_00501ba0`), the period of timer `0x394`
(`FUN_0053b330`):

    step   = max(1, capacity x P / 100)        P = GNPRTB 0x1000 (DAT_006bb700)
    extra  = ceil(allocated / step)
    period = (rate + extra) x modifier / 100
    if facility +0x60 bit 4 is set: period = FUN_0053e290(period) + period / 2

Data: GNPRTB `parameter_id` `0x1000` is 20 in every column; PROFACSD
`processing_rate` is 5 for both the mine and the refinery (MANFACSD: 4 for
the three basic yards, 2 for the advanced). Inference: class `+0x5c` is
`processing_rate`. With capacity 50 the step is 10, so a mine or refinery
cycles every 5 days when none of its capacity is allocated to upkeep and
every 10 days when all of it is. One cycle yields one unit (production hooks
above).

**Cycle end roll.** `FUN_005166a0` reads the facility's record
(`FUN_005185c0`) and rolls `FUN_0053e2e0() < |+0x6c|` (`FUN_0053e290(99)`).
`FUN_00516360` computes the result but the mine and refinery branches do not
use the roll's out-flag.

**`DAT_006b90e0`.** `FUN_00513a50` sets mode `0x1b` on entry, builds the
world, and sets `0x1a` on exit; `FUN_00513430` sets 1 and `FUN_00513650`
sets 2; `FUN_0051c030` increments it. So refineries take raw material only
once world construction has finished.

**Consequence for the day-0 survey.** The survey above already used 50 per
pair, so its verdict ("a different capacity rule") is withdrawn: the rule is
confirmed and the Empire's negative opening values must come from the port's
seeded facility counts or upkeep sum, or from the two bits that gate a
facility's contribution.

## Closing the three open items (2026-10-13)

**When capacity is assigned.** Slot `+0x114` is the notifier of
`FUN_004f7480`, the setter of `+0x50` bit 1 (created); it has no other
caller in the game logic. `FUN_004f7d50`, which the seeders call with
`(…, 1, 1)`, sets created and then completed (`FUN_004f74f0`, bit 2, slot
`+0x118`), so a seeded mine or refinery has its capacity of 50 from
creation. A facility built in play is created by `FUN_004f7d50` too (see the
build paragraph further down), so it gets it when it is ordered.
It counts toward the side's capacity only once usable (`+0x50` bit 0).

**Period randomisation.** `FUN_005f5700(n)` returns `rand / (0x7ffffffe /
n)`, a value in `0..n-1` (0 when `n <= 1`), so `FUN_0053e290(p)` is `0..p`
and the randomised period is `p/2 .. p + p/2`. Facility `+0x60` bit 4 is set
by `FUN_0053a9b0`, called from the ProcFacilState notifier `FUN_0053b1d0(old,
new)` (slot `+0x1cc`, `0x55b160`): entering state 1 sets it to `old == 4`,
entering state 4 clears it, other changes keep it. Its own notifier (slot
`+0x1e4`, `0x55b190`) recomputes the period. So only the first cycle after a
facility leaves state 4 is randomised. The state machine is `FUN_0053aa40`:
a facility with no side goes to 4; 1 -> 2 when `+0x60` bit 1 is set; 2 -> 3
when bit 2 is set (timer `0x394` fired); 3 -> 1 when bit 1 is clear.

**The negative Empire values: the port misreads the seed table.**
`FUN_00559850` rolls `FUN_0053e290(100)` (`0..100`) and looks it up with
`FUN_00595090` (checked in assembly): it walks the entries in order to the
first whose threshold `+0x20` is greater than the roll and returns that
entry's predecessor (`FUN_005f5c60`), or the last entry when none is
greater. The thresholds are lower bounds, and the first entry (threshold 0,
class 0) is a real outcome: the lookup succeeds, nothing is created and the
energy slot is spent.

| Roll | SYFCCRTB (core) | Share |
|---|---|---|
| 0..35 | nothing | 36/101 |
| 36..78 | refinery `0x2d000002` | 43/101 |
| 79..81 | `0x28000001` | 3/101 |
| 82..84 | `0x29000002` | 3/101 |
| 85..87 | `0x2a000003` | 3/101 |
| 88..95 | `0x24000003` | 8/101 |
| 96..98 | `0x23000002` | 3/101 |
| 99..100 | `0x22000001` | 2/101 |

SYFCRMTB (rim): 0..90 nothing, 91..95 refinery, 96 `0x28000001`, 97
`0x29000002`, 98 `0x2a000003`, 99 `0x24000003`, 100 `0x23000002`.

The port (`seeds.rs` `parse_facility_weights`, `pick_weighted_facility`)
drops the class-0 entry, rolls `0..99` and takes the first entry whose
threshold is above the roll, treating thresholds as upper bounds. A non-mine
core slot therefore becomes a refinery 36% of the time and a `0x28000001`
shipyard 43% of the time, and is never left empty; the original gives 43%
refineries, 3% shipyards and 36% empty. The 14..26 orbital shipyards per
side in the diagnosis above (182..338 upkeep) are this error, not what the
original generates, and the "Seeding" paragraph there is wrong on that
point.

**Fix and re-survey (2026-10-13).** `seeds.rs` now keeps the class-0 row,
rolls `0..=100` and selects by lower bound (`facility_for_roll`). Standard
galaxy, 12 seeds per row, side as player, capacity `50 x min(mines,
refineries)` less the class upkeep of the side's ships, fighters, regiments,
special forces and facilities:

| Difficulty | Alliance available | Empire available |
|---|---|---|
| Easy | -6 .. 385 (1 of 12 negative) | -41 .. 255 (1 of 12 negative) |
| Medium | 447 .. 1014 | -27 .. 385 (1 of 12 negative) |
| Hard | 319 .. 871 | -153 .. 348 (1 of 12 negative) |

Orbital shipyards per side fall from 14..26 to 0..4. The recorded 162
(Empire) lies inside every Empire row and 296 (Alliance) inside the Easy
row; the recordings' difficulty is unknown, so this is consistent with the
original, not a match.

## Recalculation triggers, opening values and the cycle roll (2026-10-13)

Second REA session (same binary and analysis profile).

**When the side totals are recomputed.** `FUN_0052fff0` has six callers.
Five are event handlers; each event is raised by the change notifier of one
flag, and the handler recomputes the side or sides concerned.

| Event | Raised by | Flag | Handler (fire slot) | Recomputes |
|---|---|---|---|---|
| `0x300` | `FUN_004fb810`, slot `+0x128` of `FUN_004f76b0` | object `+0x50` bit 6, existing | `FUN_005140c0` (`FUN_0057a730`) | the object's side, for types `0x10..0x3f` |
| `0x301` | `FUN_004fb310`, slot `+0x110` of `FUN_004f7410` | object `+0x50` bit 0, usable | `FUN_00514510` (`FUN_0057a1e0`) | the object's side, same types |
| `0x302` | `FUN_004fae40`, slot `+0xf0` of `FUN_004f6f40` | object side | `FUN_00514890` (`FUN_0057b4b0`) | the old side and the new side |
| `0x303` | `FUN_00510f20`, slot `+0x214` of `FUN_0050a4a0` | system `+0x88` bit 2, uprising | `FUN_00515cd0` (`FUN_00563b20`) | both sides |
| `0x305` | `FUN_00511300`, slot `+0x220` of `FUN_0050a5f0` | system `+0x88` bit 5, blockade | `FUN_00515f40` (`FUN_00564070`) | both sides |

The sixth caller, `FUN_0051b750`, recomputes both sides from the setup
driver `FUN_005136d0`. The `0x300` fire slot is confirmed through its class
vtable (`0x00669688`, constructor `FUN_0057a500`, factory `FUN_0057a5e0`);
the other four are matched by the same factory-then-fire layout
(inference). The senders of `0x303` and `0x305` return early unless
`DAT_006b90e0` is set.

**Refund on destruction.** In the `0x300` handler, when the object's
destroyed bit (`+0x50` bit 3) is set, `FUN_00530270` adds half the class
refined cost for a completed object of type `0x14..0x16`. The test is the
destroyed bit, not a scrap order, so it applies to any destruction.

**Builds count from the order.** `FUN_0052a140` creates each product with
`FUN_004f7d50(…, 1, 0)`: created, not completed. The object is existing
(bit 6) at once, so its upkeep is charged from then, as the manual says
(p. 84); a mine or refinery under construction has its capacity of 50 but
adds to the side's capacity only once usable. The other callers of
`FUN_004f7480` are not facility paths: `FUN_004fe630` (a spare fleet
becoming real), `FUN_00519800` (object `0xf1000001`) and `FUN_0055fe70`
(recruiting a character).

**Opening stockpiles are 0.** The side constructor calls `FUN_00533300`,
which zeroes `+0x70..+0xbc`, raw `+0x78` and refined `+0x7c` included. The
raw and refined setters have no callers besides the production hooks and
the refund, so nothing seeds a stockpile.

**The cycle-end roll is discarded.** `FUN_005166a0` takes the facility's
system (`FUN_005185c0`) and rolls `rand(0..99) < |system +0x6c|`, the
system's production modifier (`economy-systems.md`), not the facility's.
`FUN_00516360` never reads the out-flag for mines or refineries, so the roll
changes nothing but does draw one random number per cycle end.

**Over-limit lock.** Re-read: `FUN_0052f3d0` sets side `+0xa8`;
`FUN_00530350` counts the objects passing `FUN_0052e530` while capacity
`+0x58` is below `+0x70` or `+0x74`, picks one with `FUN_0053e290(count - 1)`
and calls `FUN_004f7950(1)` on it, as `timer-scheduler.md` records.

## Waiting queues, the blocked bit and the lock (2026-10-13)

**Blocked.** `FUN_0050bbb0`, called by the senders of `0x303` and `0x305`,
walks the system's processing facilities (`FUN_0053b6e0(system, 3)`) and
sets each one's `+0x60` bit 0 (`FUN_0053a7f0`, its only caller) to
`(system +0x88 & 0x24) != 0`: the system is in uprising or blockaded. That
is why both events recompute the side totals.

**Waiting queues.** Side `+0x88` (raw) and `+0x8c` (refined) are FIFO lists
of requests; `+0x80` and `+0x84` count them (`FUN_0052f050`, `FUN_0052f0c0`).
A request holds the facility's key and the facility's cycle counter
(`FUN_0053a6e0`). A refinery or yard that finds no stock at cycle start
appends one (`FUN_00533570`, `FUN_00533610`). The stock notifiers (side slots
`+0x1d0` `0x532690`, `+0x1d4` `0x5326c0`, decoded from bytes) start the retry
when the stock rises from 0 while requests wait (`FUN_0052fe50`,
`FUN_0052ff20`, not decompiled). Each retry (`FUN_0052fbf0`, `FUN_0052fd30`,
fired by `FUN_00577a20` / `FUN_00577d40`) pops the head request; if its
facility still exists, is in state 1 and has the same cycle counter, it
takes one unit and starts the cycle (`FUN_0053a860(1)`); the request is
dropped either way. The retry repeats while requests and stock both remain.

**Lock.** `FUN_0052e530` makes an object eligible when it is existing,
completed, of the side, and has a non-zero class upkeep. The Locked notifier
(slot `+0x140`, `0x4fbaa0`) calls the object's slot `+0xe0`. What that slot
recomputes, how a lock clears and the period of timer `0x382` (GNPRTB 7168
= 10 is an inference in `timer-scheduler.md`) are not traced.

## Yards and refined material (2026-10-13)

Third REA session. A yard is a facility of type `0x28..0x2b`; its manager is
the one its system keeps for the yard's production area
(`FUN_0052eb60`: facility slot `+0x204` into `FUN_00509670`).

- **Cycle start** (`FUN_00530820`): in state 1 and not already processing,
  the yard takes one refined material (`FUN_0052fb80`); with none it queues
  a request on side `+0x8c` (`FUN_00533610`, counted in `+0x84`) and waits.
  `FUN_0052fd30` serves those requests first come first served, as for raw.
- **Cycle length** (`FUN_0052cc90`): the class rate `+0x5c` as it is, 4 or 2
  days in MANFACSD. No allocation term and no randomising.
- **Cycle end** (`FUN_00530950`): with `+0x60` bit 2 set, the manager gets
  one unit through `FUN_0052a430`: progress `+0x5c` while it is below the
  current product's cost `+0x68`, otherwise the surplus `+0x60`.
- **A new current product** (`FUN_00529d70`) starts with progress
  `min(cost, surplus)` and the surplus is reduced by that.
- **How many yards run** (`FUN_00529dd0`, after every unit): the manager's
  yards are those of its area and side with `+0x50` bit 0 (`FUN_0052bde0`),
  less the blocked ones. With `needed = +0x6c - +0x60 - +0x5c`, where `+0x6c`
  is the summed refined cost of everything queued (`FUN_0052a9c0`), it
  wakes inactive yards (fastest first, `FUN_0053aac0`) while fewer than
  `needed` are in states 1..3, and puts running yards to state 4 (slowest
  first, state 1 before 2 before 3, `FUN_0053ab10`) while more are. A
  manager with no yard at all scraps its queue (`FUN_0052a2c0`, slot `+0xac`
  with cause `0x16`).
- **Completion** (`FUN_0052b960`): the product is done when progress is
  non-zero and at least the cost.

So a product costs its class refined cost in refined material, one unit per
yard cycle, and idle yards neither run nor draw.

## Port status (2026-10-13)

- `rebellion-core/src/side_resources.rs`: capacity, upkeep and the
  Maintenance monitor's value, recomputed on demand instead of on events
  `0x300..0x305`.
- `rebellion-core/src/production_cycle.rs`: both stockpiles, the upkeep
  allocation, the mine and refinery cycles and the raw request queue,
  advanced once per day. A cycle's length is fixed when it starts, and the
  ProductionModifier stays 100.
- `rebellion-render/src/resource_monitors.rs`: the three monitors. Browser captures of day 0 are in
  `docs/qa/2026-09-10-interface-parity-audit/evidence/2026-10-13-maintenance-monitor/`.
- `production_cycle::supply_yards` and
  `ManufacturingSystem::advance_supplied`: the yards' refined intake. The
  build queue counts days, so a yard of period `p` draws one unit on each day
  divisible by `p` while its area has an item, and an area that cannot draw
  all of a day's units stalls for that day. The request queue, the surplus
  and the scrapping of a yardless queue are not ported.
- All three monitors read; the day-0 capture is
  `browser-alliance-day0-three-monitors.png` in the same folder.
- Not ported: the refund on destruction (`FUN_00530270`); the over-limit
  lock (timer `0x382`); and the cycle-end roll's random draw. `economy.rs`
  still runs its own maintenance-shortfall cooldown.

## Open

1. **The remaining negative openings** (1 seed in 12): whether the
   original can also open below zero, or the port's upkeep or seeded forces
   still differ.
2. **The effect of Locked** (object slot `+0xe0`), its release, and the
   `0x382` period.
