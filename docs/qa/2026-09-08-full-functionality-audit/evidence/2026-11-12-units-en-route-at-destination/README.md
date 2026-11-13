# Units on the move appear at their destination

Follow-up to `2026-11-12-members-en-route-mark` (the user's report: moving
units should already appear at their destination, on the hyperspace
background). It closes that page's three open items.

## Trace

- `FUN_00556390` (decompiled-typed; `ghidra/notes/regiment-unload.md`,
  "Execute"): for a deployed object moving to another system it sets the
  in-transit bit (`FUN_004f7640`, `+0x50` bit 4) and calls slot `+0xa8`
  (`FUN_004f8630` -> `FUN_004f6fd0`), which reparents the object into its
  destination container at once. The same execute serves regiments,
  squadrons, ships and the members of a Move mission (its target container
  `+0x74`; `fleet-join-split.md`, `headquarters-move.md`).
- `FUN_00513050` / `FUN_00513120` walk one container's direct children and
  never descend (`object-state-flags.md`, "Child-list walk"). So an object
  moving onto a planet is in the system's lists, and one moving into a fleet
  is in the fleet's lists, not the system's.
- `FUN_0042c3b0` (line 187): `+0x50` bit `0x10` set and bit 8 clear draws
  the en route mark under the mini: a craft's own GOKRES mark, the mini +
  0x1000 (`fleet_window::en_route_mark`).
- `FUN_0045ce80`, the Defenses icon: the sum of `FUN_00526fd0` (defense
  facilities), `FUN_00504c40` (regiments), `FUN_005039d0` (squadrons) and
  the idle members of `FUN_00536da0`, all over the system's children.
- `FUN_004444c0`, the Fighter Squadron Status: Attached names the
  container; `FUN_004fd2b0` adds "ETA Destination:" while the object is en
  route; no Status row.
- `FUN_004f9860` refuses orders to an object in hyperspace: A0
  2026-11-11-regiment-en-route/03 greys Move, Confirmed Move and Scrap on an
  en route regiment.

The port keeps travelling regiments, squadrons and members in transit lists
until they land (`TroopTransportState`, `SquadronTransits`,
`MissionState::en_route`). The change is presentation only: the lists are
read where the original's container walks would find the object. No
simulation or save change.

## What changed

1. **Squadrons sent by Move to a planet** list on the destination's
   Defenses window, Fighter Squadrons page, after the planet's own, over
   the en route mark (`DefensesItem::TravellingSquadron`). Their menu
   (`MenuObject::TravellingFighter`) greys Move, Confirmed Move and Scrap;
   their Status (`StatusObject::TravellingFighter`) reads Attached
   (destination) and the ETA, with the portrait's own en route mark.
2. **Units moving into a fleet**: regiments (`RegimentLeg::Fleet`),
   squadrons (`SquadronPlace::Fleet`) and members whose Move targets the
   fleet (`ActiveMission::target_fleet`, `quadrant_icons::fleet_arrivals`)
   list on the fleet's Troops, Fighters and Personnel tabs after its own,
   each with its en route mark, and light those tabs. Members bound for a
   fleet leave the system's Defenses page and icon counts and the defenses
   icon's Move team.
3. **The Defenses icon** counts regiments and squadrons travelling to the
   planet (`quadrant_icons::Travellers`), so a planet with nothing else
   shows it.

## Verification

- Unit tests (each shown failing without its change):
  `squadrons_travelling_to_the_planet_list_after_its_own_with_the_en_route_mark`,
  `a_squadron_in_hyperspace_greys_its_move_rows_and_scrap`,
  `a_travelling_squadron_is_attached_to_its_destination_with_its_eta`,
  `a_regiment_or_squadron_on_its_way_lights_its_destinations_defenses_icon`,
  `units_on_their_way_aboard_are_listed_with_the_en_route_mark_and_light_their_tabs`,
  `a_member_on_its_way_aboard_a_fleet_is_the_fleets_not_the_planets`,
  `a_member_on_its_way_aboard_a_fleet_counts_for_the_fleet_not_the_system`;
  fixture tests for the new scenarios.
- New fixtures: `SquadronEnroute` (scenario 109, code 110) and
  `FleetArrivals` (scenario 110, code 111).
- Browser, Linux Chromium, `status-window.mjs`, both sides:
  - `squadron-en-route`: the cell on the Defenses window against GOKRES
    mini + 0x1000 under the mini (17984 A-wing, 18048 TIE Fighter), the
    second system's defenses icon against STRATEGY 10773 / 10781, the menu
    states and the Status rows and marks.
  - `fleet-squadron-en-route`: the Fleet window's three tabs lit with one
    item each, the squadron's cell against the same composite, its menu
    and Status.
  - Run `status-window-2026-11-12T19-29-11-715Z-3953361`: 30 of 30 cases
    pass (test WASM `d2c3a6aea2fd0dad2bbf0297ac8fe211542445e6c8319981b2f5c73abb8e056f`).
    Cells: 0 of 564 (A-wing) and 0 of 390 (TIE Fighter) unkeyed pixels
    differ, on the Defenses window and in the Fleet window; the icon: 0 of
    48. Status: Attached the destination system or "Fleet 1", then the ETA.
  - Regressions on the same WASM: `fleet-window.mjs` (run
    `fleet-window-2026-11-12T19-32-47-890Z-4040003`) and
    `sector-quadrants.mjs` (run `sector-quadrants-2026-11-12T19-42-10-133Z-56596`)
    pass.

No A0 capture shows a travelling squadron or a unit moving into a fleet;
these cases rest on the trace and the A0 regiment capture.

## Open

- A member moving into a fleet: its Status still names the destination
  system, not the fleet (`status_window::character_rows` reads the transit's
  system).
- The Fleet window's aboard counts (`FUN_004a5c00`, ship slots `+0x23c`,
  `+0x26c`) leave travellers out; whether the original's slots count them
  is untraced.
- A selected ship's tabs do not list travellers: the port seats cargo by
  fleet, and which ship a traveller is reparented to is untraced.

Closed 2026-11-13 by `2026-11-13-travellers-in-fleets`.
