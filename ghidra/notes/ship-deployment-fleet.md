---
title: "A built ship's fleet: the system's bit-1 spare and the band's Destination"
description: "A capital-ship order creates its products inside the target system's bit-1 spare fleet at once; the manager's DeploymentKey (+0x78) names that container, so the band reads 'Destination: Fleet', then 'Fleet N'"
category: "ghidra"
created: 2026-11-11
updated: 2026-11-11
tags: [manufacturing, fleets, spare-fleet, system-window, band]
---

# A built ship's fleet

Traced 2026-11-11 from the typed decompile and the image (vtables read
from REBEXE.EXE), after A0 `2026-11-11-icon-menu-topic/66`.

## A0

Empire, Coruscant, with Fleet 1 (one Star Destroyer) and a new Orbital
Shipyard. The ship band read "Destination: Coruscant".
- Build Selection: four Carrack Light Cruisers, Confirm. At once the band
  read "Carrack Light Cruiser 2 / Building: 4 / Destination: Fleet"
  (`66`).
- The completed Carracks gathered in a new fleet, Fleet 4. Fleet 1 got
  none of them.
- A later single Carrack order read "Destination: Fleet 4".

## The manager's keys

The setters' offsets come from the untyped decompile. The notifier
strings are in the image.

| Offset | Key | Setter | Notifier |
|---|---|---|---|
| `+0x74` | ProductKey, the current product | `FUN_00529770` | `ManuMgrProductKeyNotif` (`FUN_0052bcb0`) |
| `+0x78` | DeploymentKey | `FUN_00529820` | `ManuMgrDeploymentKeyNotif` (`FUN_0052bd00`) |
| `+0x7c` | TargetKey, the Destination | `FUN_005298d0` | `ManuMgrTargetKeyNotif` |

`FUN_0052ac40` sets the DeploymentKey:
- with no product (`FUN_004ece40`), to the TargetKey;
- otherwise, to the product's container (`FUN_004f6b70` →
  `FUN_00540450`, the product's `+0x1c` parent).

It runs when the ProductKey changes (`vt220` `FUN_0052b9c0`), when the
TargetKey changes (`vt228` `FUN_0052b9f0`), and from `vt114`
`FUN_0052b820`.

The band shows the DeploymentKey's name (`FUN_00458080` →
`FUN_004591b0` reads `+0x78` → `FUN_00457f30` → `FUN_004f6270`). So an
idle band names the Destination system, and a band with a product names
the product's container.

## Where a ship order puts its products

1. **The leg, at Confirm.** The build orders `0x210`..`0x212` share
   slot `+0x50` `FUN_005385f0`. For each manager it reads the TargetKey
   (`FUN_004c27b0`, `0x538af3`). For each unit it calls
   `FUN_00551a20(type, target)` → `FUN_00552300` and appends a
   (container, type) node (`FUN_0053e020`).
2. **The planner `FUN_00552300`** scores, in order, keeping the best
   (`FUN_0054fbb0`: no error beats an error):
   - the target itself (`FUN_00552b10`);
   - for a system target (`0x90..0x98`), the side's spare: bit 2
     (`FUN_00509b40`) for route kind `0x270`, else bit 1
     (`FUN_005097d0`);
   - if an error remains, the ships of the system's fleets
     (`FUN_004ffef0`, `FUN_00502e30`).

   A system refuses capital ships (`FUN_00507750`), so a ship's container
   is the bit-1 spare.
3. **Fire.** `FUN_0057f890` → manager `+0x1f0` `FUN_00529ab0` →
   `FUN_0052a140` creates each product with `FUN_004f7d50`, with the
   node's container as ECX (`0x52a1cd`). The product sits in the spare from
   the order onward. Completion (`FUN_0052bee0`) only sets Deployed and
   Completed; it does not move the product.

## The spare becomes a fleet and stays the spare

- The existing-changed event `0x300` (`FUN_005140c0`) calls the
  container's `+0xc4` (`FUN_004fe870`). For a capital ship that is
  `FUN_004fe630`, which makes the fleet real (bit 0, Created,
  Completed). hyp: the same handler then numbers it (`FUN_00517760` at
  `0x5142ca`), as "Fleet N".
- `FUN_0050bc60` clears bit 1 only from a destroyed spare; it has no
  "holds anything" test (bit 2's `FUN_0050be00` has one). `Fleet_vtf4`
  `FUN_004ff570` clears both bits when the fleet changes container
  (reparent `FUN_004f6fd0`).
- So a numbered fleet stays its system's bit-1 spare until it leaves or
  is destroyed. The next ship order there joins it: A0's "Destination:
  Fleet 4".
- A spare is not existing, so it is never numbered and shows its record's
  name, TEXTSTRA 11523 "Fleet" (`fleet-names.md`).

## The Destination order (`0x214`)

- Its leg `FUN_00537180` runs `FUN_00551900` → `FUN_00552300` against the
  new target for each queued product, so ships route into that system's
  bit-1 spare.
- Its fire (`vt200` `FUN_00529cd0` → `FUN_0052a380`) sets the TargetKey,
  then reparents each product (`+0xa8` `FUN_004f8630` → `FUN_004f6fd0`).
- Untraced: the DeploymentKey is recomputed inside SetTargetKey, before
  the reparent loop.
- The facility manager's branch (`FUN_00553410`) is unread.

## Families

- Capital ships go to the spare (certain for the route; the system's
  refusal is `fleet-join-split.md`'s).
- hyp: regiments and facilities stay in the system, which scores first
  with no error.
- hyp: fighters go aboard a ship with room.
- The capacity tables (`FUN_00550af0`, `FUN_0054f5b0`) are unread.

## Read

In full: `005298d0`, `005175e0`, `0052b820`, `0052b8c0`, `0052a380`,
`00529cd0`, `0052ac40`, `005385f0`, `0057f540`, `0057f890`, `00529ab0`,
`0052a140`, `00552300`, `00551a20`, `00551900`, `00552860`,
`00552b10`, `0054fbb0`, `0054fc40`, `005097d0`, `0050bc60`,
`004fe630`, `004ff570`, `004fe810`, `004fe870`, `004f6fd0`,
`004f8630`, `004f7480`, `0040d760`, `004c27b0`, `004591b0`, `00540450`.

In part: `00537180`, `005140c0`, `0050be00`, `00529dd0`.
