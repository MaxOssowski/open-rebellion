---
title: "A facility under construction or en route when its system changes sides"
description: "FUN_004f8680's Autorouting branch, the 0x307 handler's search and reason-2 destruction, and why no Facility Lost (0x303) follows a first trip"
category: "ghidra"
created: 2026-11-17
updated: 2026-11-17
tags: [facility, side, control, autorouting, facility-lost, 0x303, 0x307, manufacturing]
---

# A facility in a system that changes sides

Traced 2026-11-17 from `~/Games/rebexe-decompile/decompiled-typed`, for
ledger cell OBJ-09-C008. It builds on `facility-ownership.md` (the
cascade), `unfinished-products.md` (products in their destination),
`build-delivery.md` (trips, the setters table) and `captive-escape.md`
(the autoroute search's steps). Lines marked hyp are readings the code
does not state outright.

## A0 (the case to explain)

`.artifacts/a0-vm/2026-11-17-edited-saves-6`, c-01..c-07, edited save
ALHQ-d2-fac1 (every Alliance system's energy full but Corsin's): a
Construction Yard built at Talus for Corsin travels with the en-route icon
9025. Corsin turns Imperial on day 46 (its regiments scrapped). No
Facility Lost appears (message index All on day 47, Manufacturing on day
57). A save right after the flip has no yard: not rerouted, not handed
over.

## The branch

1. The side change (`FUN_00510d70` -> `FUN_004f6f40` -> System slot `+0xf0`
   `FUN_00510820`) runs `FUN_004fae40`. That walks the system's objects in
   mode 3 (`FUN_004fcd00(system, 3)`, Existing, so unfinished and en-route
   products too) and calls slot `+0xb0` on each, then posts `0x302` and
   `0x318` for the system.
2. Every facility class has `FUN_004f8680` at `+0xb0` (`ProcFacil_*`,
   Mine, Refinery: `ghidra/generated/class-slots.tsv`). It reads the
   container's new side (`FUN_004f6b50`: `+0x1c`, then `+0x24 >> 6 & 3`)
   and the object's `+0x50`:
   - Completed (`0x4`) and not EnrouteActive (`0x20`): handed over or
     removed (`facility-ownership.md`).
   - **Otherwise** (not Completed, **or** EnrouteActive), unless Destroyed
     (`0x8`): `GameObj_SetAutorouting(1)` (`FUN_004f78e0`, `+0x50`
     `0x1000`). Its notifier slot `+0x13c` (`FUN_004f2bd0` ->
     `FUN_004fba70`) raises `0x307`, GameObjAutoroutingChanged
     (`ghidra/generated/events.tsv`).

   The en-route yard (Completed, EnrouteActive; icon 9025) takes this
   second branch. **`FUN_004f8680` itself removes nothing**: it neither
   hands the yard over nor destroys it.
3. The `0x307` handler `Event307_GameObjAutoroutingChanged_Handle`
   (`FUN_00515480`), for a non-character with the flag being set and the
   game flag `FUN_00506e60` (`+0xbc`) set:
   1. runs the autoroute search `FUN_005545e0(obj, system, .., 0, 1, &dest,
      &status)` from the object's system;
   2. **if the status is still an error** (`local_10 != -1`), calls slot
      `+0xac(2)`: `FUN_004f8660` -> `GameObj_SetDestroyedReason(2)`
      (`FUN_004f71d0`). Its slot `+0x100` (`FUN_004f2ad0` -> `FUN_004fb200`
      -> `FUN_004f7560`) sets Destroyed (`+0x50` `0x8`);
   3. for a Completed object not destroyed, `FUN_00555410` asks whether the
      destination lies in its own system; if not and its slot `+0x34`
      (speed; a facility's is `GetGnprtb_1`, `FUN_004f63f0`: GNPRTB 1 while
      the argument, EnrouteActive, is set) is 0, `+0xac(5)`;
   4. unless destroyed, `FUN_00556390` moves it to `dest`. Not Deployed
      (slot `+0x38`, `FUN_004f6410`, `+0x50` bit 16), an unfinished
      facility changes container at once (`+0xa8`); an en-route one keeps
      EnrouteActive and changes container.

## Why the search finds nothing for a facility

`FUN_005545e0` tries, in order, until the status becomes `-1`
(`captive-escape.md`): the current container (`FUN_00554740`), the `+0x70`
object (`FUN_00554ab0`), the nearest unblockaded system of the object's
side (`FUN_00554e80`), systems with a fleet of its side (`FUN_00554f80`),
blockaded systems of its side (`FUN_005550e0`), neutral systems
(`FUN_005551e0`), the other side's systems (`FUN_005552e0`). Each candidate
goes through the move validator `FUN_005571c0` (the checker
`FUN_005511d0`/`FUN_00551100` with order `0x204`; `FUN_005513a0` builds the
candidate's capacity table with `FUN_00550700`; the planner `FUN_00551630`
-> `FUN_00552300` keeps it only with no refusal for the mover's family).

For the facility row (`FUN_0054f5b0` rows `0x20..0x30`, `0x2c..0x2d`):
- **`System_vt74` (`FUN_005073d0`)** calls `FUN_00553410(system, side,
  side, 1, 1)`: `{1, 0x24}` for a system of another side, then a blockaded
  system (`+0x88` `0x20`) gives `{0x90, 4}`. It clears those refusals only
  for fleets (`0x08..0x10`) and characters (order `0x240`); never for a
  facility. So the current container (now the enemy's), neutral and enemy
  systems, and blockaded own systems all refuse.
- **`System_vt78` (`FUN_00507750`)** gives the room: energy (`+0x5c`,
  `FUN_00509650`) less the facilities there (`FUN_00539d70`), and for a
  mine also raw materials (`+0x64`, `FUN_00509660`) less the mines
  (`FUN_0052d610`). With no room `FUN_00550700` writes `{1, 0x27}`.
- hyp: a fleet takes no facility (the planner `FUN_00552300` places a
  mover in a system target, its spare, or its usable fleets' ships; no
  facility has ever been seen aboard), so the fleet step adds nothing.
  `Fleet_vt74`/`CapShip_vt74` were not read for the facility row.

So **a facility autoroutes only to the nearest unblockaded system of its
side that has room**. In A0's edited save every Alliance system's energy
was full, so the search ended in error and the yard was destroyed with
reason 2: that is the removal seen in the save.

## Why no Facility Lost (0x303)

`FUN_004fc080` is the Destroyed notifier (slot `+0x184`, `0x178 + 4 * 3`).
It always raises `0x302` GameObjDestroyedNotif, and raises `0x303`
GameObjDestroyedOnArrivalNotif **only when** the object is EnrouteActive
(`+0x50 & 0x20`) **and** `+0x40 & 0xff0000` is not 0. Then it raises
`0x305`/`0x306`/`0x304` for reasons 6, 7, `0x15`; reason 2 raises none.

`+0x40` byte 2 (`+0x42`) is the destination-change count:
- its setter `FUN_004f72f0` ("capshp.cpp: Invalid DestinationC..."),
- raised by one in `GameObj_vtf4` (`FUN_004faf50`, the container-change
  hook, slot `+0xf4`) when the object changes container while
  EnrouteActive, up to `DAT_0065d254`,
- cleared to 0 by `FUN_004fb760` (slot `+0x124`, the EnrouteActive-change
  hook) when the trip ends.

The A0 yard was on its first trip: it never changed container while en
route, so its count was 0 and only `0x302` was raised. Facility Lost
(`FUN_00497690`, TEXTSTRA `0x70b0`/`0x70b1`) therefore needs a facility
that was already **rerouted once while en route** (a first autoroute that
found a place) and is then destroyed while still en route (a second
autoroute that finds none, or another destruction). This also explains the
earlier A0 runs with no Facility Lost (ledger note: four Alliance routes,
one Empire game, h-73, h-13).

hyp: the side-view copies (`FUN_00539fd0(1)`/`(2)`) carry their own `+0x40`
and `+0x50` (`FUN_004fa2a0` syncs byte 2), so the notifier reads the same
values on each side's copy.

## Unfinished products

An unfinished facility (not Completed, so not EnrouteActive) takes the same
`FUN_004f8680` branch. With a place it moves there at once and its
completion starts the trip from its yard (`FUN_0052bee0`). With no place it
is destroyed with reason 2; it was never en route, so no `0x303` either.
hyp: its manager then drops it (`FUN_00528890` accepts a current product
only while it is not destroyed); not traced.

## Port (2026-11-17)

`rebellion_core::products::autoroute_facilities`, called from
`GameWorld::hand_over_facilities`:
- a facility product of another side than the new holder, in the system,
  goes to `facility_place`: the nearest system (`original_isqrt` distance)
  held by its side, not destroyed, not blockaded, with an energy slot (and
  a raw-material slot for a mine) left, counting built facilities and
  facility products there;
- found: its container changes. One en route keeps travelling; port: its
  arrival day becomes the old day plus the leg from the lost system (the
  original's `0x387` still fires on the old day and `FUN_00556620` starts
  the leg on);
- not found and en route: removed, with no message;
- port: not found and unfinished: left in place (the original destroys it;
  the port would also have to drop its queued unit).

Before this, the port left the product in the lost system: an en-route
yard completed there on arrival.

Not changed (follow-ups):
- `DeliveryState::advance` files `lost` (Facility Lost) for a delivery that
  reaches a destroyed system and finds no place on its first trip. By the
  trace above that loss raises no `0x303`; only a delivery already
  rerouted should report it.
- `products::is_alliance_of` gives a facility product `is_alliance =
  false`; the facility's side is read from its `FacilityBuild` instead.

## Read

In full: `004f8680`, `004f78e0`, `004f6b50`, `00515480`, `00555410`,
`00555460`, `00555540`, `00556390`, `005545e0`, `00554740`, `00554e80`,
`005571c0`, `005511d0`, `00551100`, `005515d0`, `00551630`, `005529a0`,
`00552000`, `00552f40`, `00550200`, `00553130`, `00552300`, `00552b10`,
`00552e80`, `005513a0`, `005531b0`, `00552ff0`, `00553410`, `005073d0`,
`00507750`, `00550700`, `004fc080`, `004f72f0`, `004faf50`, `004fb760`,
`004f8660`, `004f71d0`, `004f2ad0`, `004fb200`, `004f7560`, `004f63f0`,
`004f6410`, `004fae40`, `GameObj_vt11c` (`004f2b50`). In part:
`00550c90`, `00550de0`, `0054f5b0`.
