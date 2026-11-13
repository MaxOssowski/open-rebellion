# Units on their way into a fleet: Status, counters, ships, joins

Follow-up to `2026-11-12-units-en-route-at-destination`. It closes that
page's three open items, a review finding on it, and the case of ships
moved one by one onto a fleet in another system.

## Trace

- **Attached.** `FUN_00556390` reparents a deployed object into its Move's
  target container (`+0x74`) at once. For a Move onto a fleet that is the
  fleet, so the Character and Spec Forces Status (`FUN_004486f0`,
  `FUN_00445780`) name the fleet under Attached while the member travels.
- **Counters.** `FUN_004a6be0` adds each ship's slot `+0x23c` (Fighters) or
  `+0x240` (Troops) to the window's `+0x198`, and `+0x26c` / `+0x270` to
  `+0x19c`; `FUN_004a5c00` prints both. In the capital ship vtables
  (REBEXE `.rdata`) those slots are `CapShip_vt23c` (`FUN_00500fe0`) and
  `CapShip_vt240` (`FUN_00501040`): `FUN_005039d0(ship, 3)` /
  `FUN_00504c40(ship, 3)` and `FUN_00513180`, a count of the ship's
  children by walk mode 3 (`+0x50` bit 6, existing;
  `object-state-flags.md`), the right list's own walk. A traveller is a
  child already, so it counts.
- **Which ship.** For a fleet destination `FUN_00552300` tries the fleet's
  usable ships of the side (`FUN_00502e30(fleet, 1, side)`) in child order
  through `FUN_00552b10`. That keeps a later candidate only when
  `FUN_0054fbb0(new, best)` is true (call at `0x552c87`, `ECX` = the new
  candidate): not refused, then unlimited room beats limited, then more
  room left (`+8`, set by `FUN_0054fca0` to the room less the team's
  count). All comparisons are strict, so equals keep the first. The room is
  `FUN_00500b40`: capacity less the mode-3 count for regiments and
  squadrons, unlimited for characters and special forces. So a regiment or
  squadron boards the ship with the most room left, the first of equals,
  and personnel board the first usable ship.
- **Content flags.** `FUN_004a66a0` walks the ship in mode 3 too, so a
  traveller marks the ship it boards.
- **Joins.** Manual p. 122: a ship moved onto a fleet in another sector
  "will immediately be considered a member of the fleet but will still be
  in hyperspace". `FUN_004ffc90` expands a fleet into its ships for a fleet
  target, and `FUN_004feca0` reparents each into the target at once
  (`fleet-join-split.md`).
- **Review finding.** Mode 3 takes existing children only, so a killed
  member is gone; `FUN_00553410` refuses a Move into another side's
  container (`1`/`0x24`, `1`/`0x28`), so a member of another side is never
  aboard.

## Change (presentation only; no simulation or save change)

1. `status_window::character_rows` takes the members bound for a fleet
   (`quadrant_icons::fleet_arrivals`); `transit_container` names the fleet,
   else the transit's system. `special_force_rows` uses it too.
2. `fleet_window::bound_members` lists only living members of the fleet's
   side.
3. The fleet's Fighters and Troops counters add the squadrons and
   regiments on their way in.
4. `fleet_window::ship_load` seats the travellers on the fleet's ships by
   the rule above, over the port's own seating (`ship_cargo`, which the
   battle keeps). A selected ship's tabs, counters and content flags use it.
5. While a fleet travelling to join another will join it
   (`fleet_join::joins_on_arrival`), the target's Fleet window lists no
   separate fleet, expands the target to the travelling ships, and its tabs
   list them and their cargo en route, lit and counted
   (`joining_fleets`, `window_items`, `window_tab_enabled`,
   `window_tab_counts`). The ship's Status names the target under Fleet.

## Verification

- Unit tests, each shown failing without its change:
  `a_member_on_its_way_aboard_a_fleet_is_attached_to_the_fleet`
  (`("Attached: ", "Bespin")` against `"Fleet 1"`),
  `a_killed_or_other_sides_member_bound_for_a_fleet_is_not_aboard`,
  `units_on_their_way_aboard_are_listed_with_the_en_route_mark_and_light_their_tabs`
  (now also the counters: `Some((0, 2))` against `Some((1, 2))`),
  `a_unit_on_its_way_aboard_boards_the_ship_with_the_most_room_left`,
  `ships_on_their_way_to_join_a_fleet_are_its_members_in_hyperspace`,
  `a_ship_on_its_way_to_join_a_fleet_names_that_fleet`
  (`"Fleet 2"` against `"Fleet 1"`); fixture test
  `the_en_route_joining_scenario_sends_the_one_ship_fleet_onto_the_other`.
- New fixture `FleetJoinEnroute` (scenario 111, code 112).
- `cargo mutants` on the touched functions (`bound_members`, `ship_load`,
  `joining_fleets`, `join_target`, `window_*`, `tab_counts`,
  `incoming_*`, `transit_container`, `special_force_rows`, `ship_rows`,
  `ship_contents`, `left_entries`): 102 mutants, 92 caught, 7 unviable,
  3 missed; the tests were strengthened for the three (a destroyed first
  ship, a fleet on a plain move, members bound for different fleets);
  a rerun on `ship_load`, `joining_fleets` and `transit_container`: 41
  mutants, 40 caught, 1 unviable. The test changes come after the gate
  WASM; they touch no code it runs.
- Browser, Linux Chromium (`.browser-linux.json`), test WASM
  `5d5a2e6b333a6fb83d22b9c3e0b0903ac4204f72f5b0ddfca4827dfde60da2d0`:
  - `status-window.mjs`, run `status-window-2026-11-12T23-39-07-969Z-895720`:
    32 of 32 cases pass. New `fleet-character-en-route` (both sides):
    Attached "Fleet 1", Enroute, "Day 99". `fleet-squadron-en-route` now
    also checks the Troops and Fighters counters equal the listed items.
    (Two earlier runs, made while `cargo mutants` loaded the machine, each
    timed out once in a different case: `empire/fleet-character-en-route`
    in `...23-27-52-114Z-634204`, `empire/force-character` in
    `...23-33-49-596Z-783277`.)
  - `fleet-window.mjs`, run `fleet-window-2026-11-12T23-43-45-078Z-940382`:
    95 of 98 pass, new `join-en-route` on both sides (one fleet listed of
    two ships, three on its Capital Ships tab). The three failures, two
    A0 files missing from the worktree and one confirmation that did not
    open, pass in run `fleet-window-2026-11-12T23-57-05-761Z-1048658`
    (3 of 3) once the A0 files were copied.

## Open

- port: the travelling ships of a join follow the target's own in the
  lists; the original orders a container's children by object key.
- port: the seats are recomputed from the port's own seating
  (`ship_cargo`, a hyp) each frame; the original fixes each seat when the
  order executes.
- ~~The Capital Ship Status Embarked counts (`ship_rows`) leave travellers
  out~~: closed below.
- No A0 capture shows any of these cases.

## Follow-up: Embarked in the Capital Ship and Fleet Status

- **Trace.** The Capital Ship Status (`FUN_00446fd0`) prints Embarked
  Fighter Squadrons from slot `+0x23c` (`CapShip_vt23c`, `FUN_00500fe0`),
  Trooper Regiments from `+0x240` (`CapShip_vt240`, `FUN_00501040`), and
  Personnel from `FUN_004f25c0(ship, 3)` (families `0x30..0x3c`, counted
  by `FUN_00513180`); the Fleet Status (`FUN_00449200`) sums the same per
  ship. All walk mode 3, the Fleet window's own walk, so the travellers
  count (`ghidra/notes/status-window.md`).
- **Change.** `status_rows::travellers` takes each ship's squadrons and
  regiments on their way in from `fleet_window::ship_load`;
  `incoming_characters` counts the characters bound for the fleet
  (`fleet_window::bound_members`). `ship_rows` adds them to the ship's
  Embarked rows (the characters, like the fleet's own, on its first alive
  ship: port), and `fleet_rows` adds them to the fleet's.
- **Unit test.**
  `units_on_their_way_aboard_count_as_embarked_on_the_ship_that_seats_them`
  failed without the change (`["2", "0", "1"]` against `["2", "1", "2"]`).
- **Browser.** New `status-window.mjs` case `fleet-ship-en-route` (both
  sides): the one-ship fleet's Embarked rows equal the Fleet window's
  Fighters and Troops counters and Personnel list. The fixture bridge
  (`emit_status_rows`) now reports the ship's rows in FleetArrivals.
  Linux Chromium, test WASM
  `b16cb0e7fbc3f952dc03bd6d60d5e5cf6378a31b91183b9aa7e5a0d77f8aa79e`, run
  `status-window-2026-11-13T00-37-53-703Z-1503194`: 34 of 34 pass; the
  new case reads Embarked 1, 1, 1 on both sides. An earlier run
  (`...00-28-27-533Z-1333942`) showed Personnel 0: the fixture's ship is
  unnumbered (`id` 0), so `ship_load` seats no character; the characters
  now count from `bound_members` on the first alive ship.
- `cargo mutants` on `travellers`, `incoming_characters`, `fleet_rows`,
  `ship_rows`: 40 mutants, 37 caught, 3 unviable, 0 missed.
- port: the Fleet Status Personnel counts characters only, as before;
  the original also counts special forces (`0x30..0x3f`), aboard or on
  their way.

## Follow-up: special forces in the Fleet Status Personnel

- **Trace.** The Fleet Status (`FUN_00449200`) counts Personnel per ship
  with `FUN_00536da0(ship, 3)`, whose range is `0x30..0x40`; the Capital
  Ship Status (`FUN_00446fd0`) with `FUN_004f25c0(ship, 3)`, range
  `0x30..0x3c`. Both are counted by `FUN_00513180`, whose filter
  `FUN_005131d0` rejects `family < lo || hi <= family`, so the ranges are
  half-open: the Fleet Status counts characters (`0x30..0x3b`) and special
  forces (`0x3c..0x3f`), the Capital Ship Status characters only. Mode 3
  takes the members on their way in, as above.
- **Change.** `status_rows::incoming_personnel` counts the characters and
  special forces bound for the fleet (`fleet_window::bound_members`).
  `fleet_rows` Personnel adds `Fleet::special_forces` and the special
  forces on their way in; `ship_rows` keeps the characters only. The
  `FleetArrivals` fixture sends a special force of the player's side
  (SPECFCSD 2 or 7) on the character's Move after it, and the fixture
  bridge reports the Fleet Status rows there too.
- **Unit test.**
  `fleet_status_personnel_counts_special_forces_and_capital_ship_status_does_not`
  failed without the change (`"1"` against `"3"`); fixture test
  `the_fleet_arrivals_scenario_sends_a_regiment_squadron_character_and_special_force_aboard`.
- **Browser.** Linux Chromium, test WASM
  `57853a077bfdcb003e1c69668e8e5ccec1ac4e2d4d4b31bbdc8b599c8b09ef03`, run
  `status-window-2026-11-13T00-52-31-929Z-1577288`: 36 of 36 pass. New
  case `fleet-status-en-route` (both sides): Embarked 1, 1, 2, the Fleet
  window's counters and whole Personnel list (the character and the
  special force). `fleet-ship-en-route` now reads Personnel 1, the list's
  characters only; `fleet-squadron-en-route` and
  `fleet-character-en-route` list two members, the character first.
- `cargo mutants` on `incoming_personnel`, `fleet_rows`, `ship_rows`: 45
  mutants, 42 caught, 3 unviable, 0 missed.
- ~~port: the Fleet Status Personnel counts characters only~~: closed.
