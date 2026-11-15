---
title: "Unfinished products in their destination container"
description: "How REBEXE represents a ship, squadron, regiment or facility under construction: an Existing but not Completed child of its destination container, and which walks see it (windows, maintenance) or skip it (battle, Retreat's unit search, fleet usability and speed)"
category: "ghidra"
created: 2026-11-15
updated: 2026-11-15
tags: [manufacturing, fleets, spare-fleet, retreat, battle, maintenance, object-walks]
---

# Unfinished products

Traced 2026-11-15 from `~/Games/rebexe-decompile/decompiled-typed`, for
ledger cell EVT-02-C008 (user decision 2026-11-15: "do it as in the
original"). It builds on `build-delivery.md`, `ship-deployment-fleet.md`,
`object-state-flags.md`, `unfinished-facility.md` and
`per-side-object-views.md`. Lines marked hyp are readings the code does
not state outright.

## What an unfinished product is

- **Creation.** A build order's fire (`FUN_0057f890` -> manager `+0x1f0`
  `FUN_00529ab0` -> `FUN_0052a140`) creates each product with
  `FUN_004f7d50(class, side, ctx, 1, 0)`. That calls
  `GameObj_SetCreated(obj, 1)` and `GameObj_SetCompleted(obj, 0)`
  (`FUN_004f7d50`, last two calls). The ECX of the call is the planned
  container (`ship-deployment-fleet.md`, step 3).
- **Its bits.** Created (bit 1) is set, so Existing (bit 6, created and not
  destroyed, `0x004f7bc0`) is set. Completed (bit 2) is clear, so Usable
  (bit 0, existing and completed and not en route, `0x004f7b80`) is clear.
  "Unfinished" is therefore *Existing and not Completed*. The `+0x50`
  mask `0x20` is EnrouteActive (`object-state-flags.md`); `0x20` in the
  Retreat trace is the refusal **code** that `FUN_00553410` posts for a
  clear Completed bit, not a state bit.
- **Where it sits.** In its destination's container from the order on:
  a capital ship in the target system's bit-1 spare fleet
  (`ship-deployment-fleet.md`); hyp: a regiment or facility in the system
  itself, a squadron aboard a ship with room or on the planet (A0
  `pending-facility/23-tab0` shows an A-wing under construction on Pzob's
  Fighter Squadrons page, so the planet is one of its containers). The
  manufacturing manager keeps it in its queue (`+0x54 -> +0x18`) and
  accepts it as the current product while it is created and not completed,
  destroyed or en route (`FUN_00528890`).
- **Its container becomes real.** The product's Existing change raises
  `0x300` (`Event300_GameObjExistingChanged_Handle`, `FUN_005140c0`), which
  calls the container's slot `+0xc4`. For a fleet (`Fleet_vtc4`,
  `FUN_004fe870`) and a capital-ship child (`0x14..0x1b`) that is
  `FUN_004fe630`: if any member is Existing (`FUN_00502e30(fleet, 3)`), the
  fleet is set Created and Completed. So a spare that holds only
  unfinished ships is an Existing, Completed fleet, but not Usable:
  `Fleet_vt84` (`FUN_004fe540`) needs a member that is Completed and not
  EnrouteActive.

## When it becomes finished

`FUN_0052b960` completes the current product when progress `+0x5c`
reaches the cost `+0x68`; `FUN_0052bee0` sets Deployed (`0x10000`),
EnrouteActive (bit 5) when the manager's system differs from the
product's, then Completed (bit 2) (`build-delivery.md`). The product does
not change container. While it travels it is Completed but en route, so
still not Usable; it becomes Usable when the arrival (`0x387` ->
`FUN_004fba10` -> `FUN_00556620`) clears EnrouteActive.

## Which walks see it

The child walk's mode (`FUN_004f6b90`): 1 Usable, 2 en route, 3 Existing,
4 all (`object-state-flags.md`). An unfinished product passes modes 3 and 4
only.

| Consumer | Walk | Sees an unfinished product? |
|---|---|---|
| Battle build (`FUN_0040b0e0`) | fleets mode 3 (`FUN_004ffe70(system, 3)`), then each fleet's capital ships `FUN_005001b0(fleet, 1)`, Death Stars `FUN_005003d0(fleet, 1)`, ships for squadrons `FUN_00502db0(fleet, 1)`, squadrons aboard `FUN_005039d0(ship, 1)`, planet squadrons `FUN_005039d0(system, 1)` | No: every unit walk is mode 1. One exception: after the Death Stars, the system's ManuMgr (`FUN_00509670`) is asked for its product; a Death Star product (`0x18..0x1b`) whose Completed bit is clear goes to `FUN_0040bcd0` (`FUN_0040b0e0`, `0x40b4d2..0x40b551`). An unfinished Death Star takes part in the battle. |
| Battle result (`FUN_0040c780`) | the same objects again (`planet-squadrons-in-battle.md`) | No (mode 1). |
| Retreat, gravity well (`FUN_00516730` output 1) | enemy fleets `FUN_004ffef0(system, 1, side)`, their ships `FUN_00502e30(fleet, 1, side)` | No. |
| Retreat, a unit that can move (output 2) | viewer fleets `FUN_004ffef0(system, 1, side)` and their ships `FUN_00502e30(fleet, 1, side)`, slot `+0x34` != 0; then planet squadrons `FUN_00503a50(system, 1, side)` | No: a fleet holding only unfinished ships is not Usable, so it is not walked; a mixed fleet is found through its finished ships. |
| Retreat, the destination (output 3) | `FUN_005545e0` -> `FUN_005571c0` -> the unit's `vt74`. For a fleet, `Fleet_vt74` (`FUN_004fe0d0`) passes "check Completed" only when `(+0x58 & 6) == 0` (not a spare); a capital ship (`CapShip_vt74`, `FUN_00500ac0`) always passes it. `FUN_00553410` then posts `0x20` for a clear Completed bit. | The unit is the fleet, whose own Completed bit is set by `FUN_004fe630` once it holds any Existing member, and which is skipped when it is a spare. So a fleet's unfinished members do not raise `0x20` through the unit check. **Open:** see "FUN_005513a0" below. |
| Fleet Usable (`Fleet_vt84`, `FUN_004fe540`) | members mode 3, needs one Completed and not EnrouteActive | Not counted toward Usable. |
| Fleet mobile (`Fleet_vt38`, `FUN_004fda10`) and speed (`FUN_004fd900`) | members mode 3, Completed and not EnrouteActive | Not counted: a fleet of only unfinished ships cannot move; a mixed fleet moves at its finished ships' speed. |
| Capital ship as a mover (`CapShip_vt74`) | the ship itself | Refused, code `0x20`. Squadrons and regiments do not call `FUN_00553410` through `vt74` (its only `vt74` callers are Fleet, CapShip and System), matching A0 `pending-facility/24-awing-menu`: an unfinished A-wing's Move is white. |
| Fleet window lists (`FUN_004a3340`) | fleets `FUN_004ffe70(system, 3)`; ships `FUN_00502db0(fleet, 3)`; tabs: squadrons `FUN_005039d0(ship, 3)`, regiments `FUN_00504c40(ship, 3)`, people `FUN_00536da0(ship, 3)` (`fleet-window.md`) | Yes: listed, with the unfinished mark over the mini (`FUN_0042c3b0`: 11570 side 1, 11572 otherwise). |
| Fleet icon in a sector window (`FUN_004feea0(system, side, 3)`) | fleets mode 3 | Yes: a spare that holds only unfinished ships is Existing, so the icon shows. hyp, not seen in A0. |
| Maintenance (`FUN_0052fff0`) | every object of the side with `+0x50` bit 6 (`side-resource-monitors.md`) | Yes: it costs maintenance from the order. A product en route (Completed, Existing) also costs. |
| Status window (`FUN_0044a470` and the fleet/ship rows, `status-window.md`) | the object | "Under Construction" (34647) while Completed is clear. |
| The enemy's view copy (`FUN_004f9a60`) | the object | The owner's copy is always live. The other side's copy is live only with the side's seen bit **and** Completed **and** not en route, so the enemy's copy of an unfinished product stays frozen (`per-side-object-views.md`). |

## Open

- **`FUN_005513a0`.** Reached from `FUN_005571c0` for each route
  candidate, it walks the fleets at an object resolved from the route
  record's `+0xc` key (`FUN_00552ff0` -> `FUN_0050c640` / `FUN_00553130`)
  in mode 4 (`FUN_004ffe70(.., 4)`) and every ship of each fleet in mode 4
  (`FUN_00502db0(fleet, 4)`), and runs `FUN_00552d10` on each. That calls
  `FUN_00550700`, which calls the object's `vt74` for each of nine table
  rows (`FUN_0054f5b0`) and records the status. For a capital ship
  `vt74` checks Completed, so an unfinished ship in those fleets posts
  `0x20`. Not settled: whether the key at `+0xc` is the candidate
  destination or the unit's own system, and whether a posted `0x20` in
  that walk makes `FUN_005545e0`'s `status[1]` differ from -1 (Retreat's
  output 3). If it does, an unfinished ship in a fleet at the battle's
  system can disable Retreat, as `2026-11-12-evt-02-c008-cause` suspected.
- The containers of a regiment, squadron and special-force product
  (`FUN_00552300` scoring, `FUN_00550af0`/`FUN_0054f5b0` capacity tables).
- What happens to unfinished members when their fleet moves: they are its
  children, so hyp: they travel with it and `FUN_0052bee0` then starts a
  delivery from the yard's system to wherever the fleet is.
- Whether `0x300` numbers a spare at once (A0 66 read "Destination: Fleet"
  right after the order, then "Fleet 4" after completion).
- Where a battle is triggered and whether a spare holding only unfinished
  ships counts as presence (`FUN_00512280`, `FUN_005124c0`).

## Read

In full: `004f7d50`, `0052a140`, `00553410`, `004fe0d0`, `00500ac0`,
`004fe630`, `004fe870`, `004fe540`, `004fda10`, `00516730`, `005571c0`,
`005511d0`, `005513a0`, `00552d10`, `00552e80`, `00552ff0`, `005001b0`,
`00502db0`. In part: `0040b0e0` (lines 195..390), `005140c0`,
`00550700` (the `vt74`/`vt78` loop).
