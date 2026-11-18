---
title: "A facility under construction or en route when its system changes sides"
description: "FUN_004f8680's Autorouting branch, the 0x307 handler's search and reason-2 destruction, and why no Facility Lost (0x303) follows a first trip"
category: "ghidra"
created: 2026-11-17
updated: 2026-11-18
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
(`FUN_00497690`, TEXTSTRA `0x70b0`/`0x70b1`) therefore needs at least a
facility that was already **rerouted once while en route** (a first
autoroute that found a place) and is then destroyed while still en route.
This also explains the earlier A0 runs with no Facility Lost (ledger note:
four Alliance routes, one Empire game, h-73, h-13). It is not enough: a
second side change after a reroute raises no visible Facility Lost either
(next section).

## A reroute, then a second side change (A0 part 8, 2026-11-18)

A0 `.artifacts/a0-vm/2026-11-18-edited-saves-8` (README "OBJ-09-C008"),
edited save ALHQ-d2-fac2 (065cd1ca5437; Talus the only Alliance system with
energy room): the yard built at Talus for Corsin left on day 46; Corsin
turned Imperial and the yard was rerouted to Talus (f-05, 5959bf3fa11e,
the en-route art on Talus's yards tab); Talus turned Imperial on day 47
(f-06). No Facility Lost: index All watched days 47-50 (f-07, f-07b), All
and Manufacturing at day 94 (f-08 423a00097b63, f-09 897748b1bf99). Host
scan of the day-94 save FAC2D94 (7398721add80) against FAC2 with
`2026-11-17-edited-saves-4/scripts/facscan.py` over families `0x20..0x2f`:
every new facility record is Imperial (`0x81` master) and none is in Talus
(`0x90000092`), so the yard was neither handed over nor kept; it was
destroyed. (The Alliance `0x24:0x6ad` in Corsin, `0x90000108`, is gone too:
the completed-facility cascade of the Corsin flip.) f-07 also shows no
"Talus Joins Enemy" or "Corsin Joins Enemy" row for these two flips.

Traced 2026-11-18:

1. **The reroute does raise the count.** `FUN_00556390` ends in slot
   `+0xa8` in both branches (bit 16 clear: at once; set: after
   `GameObj_SetEnrouteActive(different systems)`, skipped while
   HyperdriveActive), so EnrouteActive stays set. Slot `+0xa8` is
   `FUN_004f8630` -> `FUN_004f6fd0`: it re-parents the two view copies
   (`FUN_004f9d40`) and calls the master's slot `+0xf4` (`GameObj_vtf4`,
   `FUN_004faf50`) with the old and new containers. That raises `+0x42`
   by one while EnrouteActive and below `DAT_0065d254` = 255 (`.rdata`,
   REBEXE file offset `0x25ba54`), through `FUN_004f72f0`, which copies the
   byte to each view copy (`FUN_004fa2a0`, when `FUN_005404d0` passes the
   transaction's view mask, ctx `+8`). `vtf4` first calls
   `GameObj_SetAutorouting(0)`, whose `0x307` the handler drops at once
   (`param_2 == 0`, non-character). So after the reroute the count is 1.
2. **The second loss destroys it en route.** At the Talus flip the yard is
   Completed and EnrouteActive in Talus: `FUN_004f8680` sets Autorouting;
   `FUN_00515480`'s Completed branch only clears `+0x54 +0xc` when it equals
   the container (`FUN_004f6e30`, `FUN_00540450`, `FUN_004f7fa0`), the
   search fails, `+0xac(2)`. `FUN_004f7560` sets Destroyed on the master and
   `FUN_004fa580` copies it to each view copy and calls that copy's slot
   `+0x184`, `FUN_004fc080`. The notifier raises nothing for the master
   (`FUN_0053a010`: `+0x24 & 0x30`, the view index, must be set); a view
   copy with EnrouteActive and the count raises `0x302` then `0x303`.
3. **Where it can still drop.** `FUN_005400f0` writes the record's `+0x18`
   = the view index (`+0x24 >> 4 & 3`, setter `FUN_00540650`), and
   `FUN_004365d0` hands it to player 1 or 2 by it (else deletes it). The
   `0x303` record (factory `FUN_0043eca0`, class `0x00659778`) is kind 3
   (`FUN_00526f00`): `FUN_0051e770` files it in a group keyed by (kind,
   code, view, ctx `+0x14`, ctx `+0x18`) (`FUN_0051e910`, `FUN_00569060`)
   under a batch id from `FUN_004fd2e0` (hook `DAT_006b2af4`, not read).
   `FUN_00488de0` gives the whole batch the message type of the **first**
   record that maps. `0x302` of a non-system maps to nothing and `0x303`
   to `0x10` (Facility Lost), but a system's `0x100` maps to 4, System
   Joins (`FUN_00499760`), which keeps its message only when the system's
   `+0x88 & 0x2000` is set.
   **hyp:** the `0x303` rides in the side change's batch behind the system's
   `0x100`; the batch maps to type 4, whose builder drops it (no "Talus
   Joins Enemy" in f-07), and Facility Lost is never built. Not read: when
   the queued `0x307` (`FUN_0053f950` -> `FUN_0053fcb0`, ctx kept) is
   handled, the batch's record order, and `DAT_006b2af4`.
4. The builder `FUN_00497690` itself: the record's object in the reader's
   table (`FUN_004f2d10`, any family), its second key a system there
   (`FUN_004f3220`; the copy's container), once per builder; a facility
   (`0x20..0x2f`) always keeps it (`+0x40 = 1`), another family only with
   `+100` set (0 from `FUN_00497560`/`FUN_004975e0`).

**Recipe for an A0 Facility Lost (hyp):** a `0x303` whose batch has no
earlier mapped record, i.e. a rerouted facility (count 1) destroyed while
still en route by something other than a side change or a system's
destruction (a destroyed system's `0x302` maps to `0x15` first). Candidate:
the maintenance autoscrap (reason `0x15`; `FUN_004fc080` raises `0x302`,
`0x303`, then `0x304`, so `0x303` maps first, `0x10` over `0xc`). Edited
save as FAC2 but with the only free place far from Corsin (a long second
leg), flip Corsin, then before the yard arrives push the Alliance into a
maintenance shortfall (edit its stock or maintenance) and watch the
Manufacturing index for Facility Lost in place of Maintenance Shortfall.
Which object the autoscrap picks was not traced.

hyp: the side-view copies (`FUN_00539fd0(1)`/`(2)`) carry their own `+0x40`
and `+0x50` (`FUN_004fa2a0` syncs byte 2), so the notifier reads the same
values on each side's copy.

## Unfinished products

An unfinished facility (not Completed, so not EnrouteActive) takes the same
`FUN_004f8680` branch. With a place it moves there at once and its
completion starts the trip from its yard (`FUN_0052bee0`). With no place it
is destroyed with reason 2; it was never en route, so no `0x303` either.
Checked 2026-11-18: in `FUN_00515480` the not-Completed path only compares
ids (`FUN_00555540`) and falls through to the same search and `+0xac(2)`;
`FUN_0052bee0` sets Constructed, then EnrouteActive only when its two ids
(yard and container) differ, then Completed, so a moved unfinished
facility's completion starts its trip. `FUN_00528890` accepts a current
product only with `+0x50` bit `0x2` set and bits `0x1c` (Completed,
Destroyed, `0x10`) clear, so a destroyed one is no longer its manager's
product. hyp: the manager then takes its next unit; the queue walk after
`FUN_00528890` returns 0 was not read.

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
- not found and en route: removed, with no message (a first trip: count
  0; a trip already rerouted: A0 part 8 shows no message, section above);
- not found and unfinished (2026-11-18): destroyed with no message, with
  its queued unit. port: the world holds no queues, so the product waits in
  `Products::lost` (not saved) until `products::destroy_lost` takes it and
  its unit out (`products::destroy`); the tick runs it right after the
  economy step's side changes, before `reconcile` (simulation.rs, the app
  loop). A side change by occupation later in the tick is settled at the
  next tick's manufacturing step.

Before this, the port left the product in the lost system: an en-route
yard completed there on arrival.

Not changed (follow-ups):
- 2026-11-18 (part 8): the port already removes a rerouted yard with no
  place silently; pinned by
  `a_rerouted_facility_left_with_no_place_by_a_second_side_change_goes_without_a_message`
  (`products_tests.rs`). Open: `DeliveryState::advance` still files
  Facility Lost (`lost`) for a rerouted delivery whose destination is
  destroyed again. By the hyp of step 3 that batch maps to the destroyed
  system's `0x302` (type `0x15`) first, so the original may show none
  there either; no A0 for it.
- Fixed the same day: `DeliveryState::advance` filed `lost` (Facility
  Lost) for a delivery that reached a destroyed system and found no place
  on its first trip. It now files it as `scrapped` (silent); `lost` keeps
  only a delivery already rerouted (`Delivery::was_rerouted`, read back
  from the trip: a reroute starts from the destroyed system it was bound
  for). port: the `+0x42` count is not saved; a first trip whose yard is
  destroyed while it travels reads as a reroute.
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
`004f6410`, `004fae40`, `GameObj_vt11c` (`004f2b50`), `0052bee0`,
`00528890`. In part: `00550c90`, `00550de0`, `0054f5b0`.

2026-11-18 (part 8), in full: `004f6fd0`, `004f8630`, `004f2ab0`,
`004f6e30`, `00540450`, `004f7fa0`, `004ece80`, `004fba10`, `004fba70`,
`00556620`, `00556430`, `004f8240`, `004f81c0`, `004f80e0`, `004fa2a0`,
`004fa580`, `005404d0`, `004fd600`, `0053a010`, `0053a000`, `00540650`,
`004f2d10`, `004f3220`, `004f6b70`, `0053fe40`, `005400f0`, `00497690`,
`00497560`, `004975e0`, `004977b0`, `00498970`, `0043eca0`, `0043ec50`,
`00526f00`, `0051d910`, `0051e670`, `0051e6a0`, `0051e770`, `0051e910`,
`0051e9a0`, `00569060`, `005690b0`, `004365d0`, `004bef90`, `004befe0`,
`00488b50`, `0053f9c0`, `0053f950`, `0052bee0`. In part: `00488de0`,
`00489740`, `00568de0`, `00499760`.
