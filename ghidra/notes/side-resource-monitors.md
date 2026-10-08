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

## Open

1. **A mine's or refinery's own capacity total** (facility `+0x64`, set
   through `FUN_0055a6e0`). The manual (p. 81) says each mine/refinery pair
   adds 50; no GNPRTB entry of 50 is read near this code, PROFACSD carries no
   50, and the day-0 survey above does not reproduce the Empire's opening
   value with it.
2. **The facility production cycle length**: the timer whose handler
   `FUN_00578a40` drives `FUN_00516360`'s state changes.
3. **`DAT_006b90e0`**, the switch without which `FUN_0052fb30` lets a
   refinery run without taking raw material.

Also open: whether the opening raw and refined stockpiles are always 0
(both recordings show 0 and 0 on day 0), and whether in-progress builds
count toward `+0x74` (the manual, p. 84, charges maintenance at the order).
