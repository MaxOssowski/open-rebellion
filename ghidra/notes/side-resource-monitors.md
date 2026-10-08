---
title: "Side resource monitors and the per-side resource pools"
description: "What the command center's Raw Materials, Refined Materials and Maintenance monitors read, and the side-object fields, setters and production hooks behind them"
category: ghidra
created: 2026-10-08
updated: 2026-10-08
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

## Production hooks

`FUN_00516360(object, ?, state)` runs on facility state changes (callers
`FUN_0051b7f0` at setup and timer handler `FUN_00578a40`):

| Facility | State 1 (cycle start) | State 3 (cycle end) |
|---|---|---|
| Mine `0x2c` | `FUN_00530650` | `FUN_00530670`: raw `+0x78` += 1 |
| Refinery `0x2d` | `FUN_005306b0`: take 1 raw (`FUN_0052fb30`), or queue a request | `FUN_005307e0`: refined `+0x7c` += 1 |
| Construction `0x28..0x2b` | `FUN_00530820`: take 1 refined (`FUN_0052fb80`), or queue a request | `FUN_00530950` |

`FUN_0052fb30` takes raw only when `DAT_006b90e0` is set; otherwise the
refinery proceeds without it. Queued requests are retried by timer handlers
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

## Open

1. **How a mine's or refinery's processing points (`+0x64` pair) are
   seeded and grow**, and so how they add up to maintenance capacity. The
   manual (p. 81) gives 50 per mine/refinery pair; no GNPRTB or SDPRTB entry
   fits the day-0 values (scan above), and the next step is the
   `ProcFacil` point cycle (`FUN_0055a820`, `FUN_0055a960` and the
   `FUN_00578a40` timer).
2. **The facility production cycle length**: the timer whose handler
   `FUN_00578a40` drives `FUN_00516360`'s state changes.
3. **`DAT_006b90e0`**, the switch without which `FUN_0052fb30` lets a
   refinery run without taking raw material.

Also open: whether the opening raw and refined stockpiles are always 0
(both recordings show 0 and 0 on day 0), and whether in-progress builds
count toward `+0x74` (the manual, p. 84, charges maintenance at the order).
