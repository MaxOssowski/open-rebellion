# Unfinished products in their containers: trace and design (2026-11-15)

Ledger cells: EVT-02-C008 (Retreat disabled), CMD-03-C018 (the defenses
icon's Move greying). User decision 2026-11-15: "do it as in the
original"; the architecture change and a save bump are approved.

**Verdict.** The trace is committed (`ghidra/notes/unfinished-products.md`).
The port is not changed in this stage. The change touches every container
the port has, and the trace leaves the one question that motivated it
(does an unfinished ship disable Retreat?) open. Below are the findings,
the design, and what must be settled first.

## What the trace found

1. An unfinished product is **Existing but not Completed**: the order's
   fire creates it with Created 1 and Completed 0 (`FUN_004f7d50` from
   `FUN_0052a140`). It is never Usable until it is completed and not en
   route. `0x20` in the earlier Retreat trace is `FUN_00553410`'s refusal
   code for a clear Completed bit, not a state bit (`+0x50` `0x20` is
   EnrouteActive).
2. It sits in its destination container from the order: a ship in the
   target system's bit-1 spare fleet; the other families' containers are
   still hyp.
3. Any Existing member makes its fleet Created and Completed
   (`FUN_004fe630`, from the `0x300` handler through `Fleet_vtc4`). The
   fleet is Usable only with a Completed, not travelling member
   (`FUN_004fe540`), and can move only with one (`FUN_004fda10`,
   `FUN_004fd900`).
4. Walks in mode 1 (Usable) skip it, and walks in mode 3 or 4 (Existing,
   all) see it:

   | Consumer | Mode | Effect of an unfinished product | Port today |
   |---|---|---|---|
   | Battle units (`FUN_0040b0e0`, `FUN_0040c780`) | 1 | does not fight | same: it is not in the fleet |
   | Unfinished Death Star (`FUN_0040b0e0` via the ManuMgr, `0x40b4d2`) | product | **fights** | differs: not modelled |
   | Retreat outputs 1 and 2 (`FUN_00516730`) | 1 | ignored | same |
   | Retreat output 3, the unit's own check | fleet | none (a fleet with an Existing member is Completed; a spare skips the check) | same |
   | Retreat output 3, `FUN_005513a0`'s mode-4 walk | 4 | the member's `0x20` only marks it as no container in the capacity table; the candidate is not refused (Stage A) | same |
   | Fleet Usable / mobile / speed | 3, Completed only | ignored | same for mixed fleets; a fleet of only unfinished ships cannot exist in the port |
   | A ship as a container (`CapShip_vt74`, called on the objects at a destination) | object | refused `0x20`: it carries nothing | same (port carriers are finished ships) |
   | Unfinished ship's or its fleet's Move (A0 `u-2-*`) | object | white; the effect is open (see the note) | n/a (not an object) |
   | Squadron/regiment Move | object | allowed; it changes container at once, with no trip (`FUN_00556390`, not Deployed) | differs: grey (Stage B) |
   | Fleet window lists (`FUN_004a3340`) | 3 | listed with mark 11570/11572 | differs: ships not listed |
   | Defenses window pages | 3 | listed | same (`DefensesItem::Pending`) |
   | Fleet icon (`FUN_004feea0(.., 3)`) | 3 | a spare with only unfinished ships shows the icon and is numbered at once (A0 `u-2`) | differs |
   | Maintenance (`FUN_0052fff0`) | bit 6 | costs from the order, and while en route | same since Stage A (deliveries en route now cost) |
   | Enemy's view copy (`FUN_004f9a60`) | object | frozen | n/a |

5. It is finished when progress reaches the cost (`FUN_0052b960` ->
   `FUN_0052bee0`): Deployed, EnrouteActive when the yard's system differs,
   Completed. It never changes container on completion.

So for battle and for Retreat's outputs 1 and 2 the port's current split
(products outside fleets) already gives the original's result. The
differences are in the windows, the fleet icon, the unfinished Death
Star, a moving fleet's unfinished ships, and the Move of an unfinished
squadron or regiment. Retreat output 3 is not one of them (Stage A).

## Why the port is not changed in this stage

- **Size.** The port's containers are read directly in about 3,300
  places (`.capital_ships`, `.fighters`, `.ground_units`,
  `.special_forces` and the three facility lists, 40+ files; the largest
  are `abstract_battle.rs` 276, `tactical_view.rs` 248, `tactical.rs` 160,
  `interface_test_fixture.rs` 155, `fleet_window.rs` 126, `missions.rs`
  121, `squadrons.rs` 95, `integrator.rs` 73). Each read is a walk whose
  mode must be traced before the flag is honoured; a read left unaudited
  would let an unfinished ship fight, be counted as strength by the AI,
  scrap, repair, bombard or carry troops.
- **Representation.** Squadrons are counted entries (`FighterEntry {
  class, count, carrier }`), not objects, so an unfinished squadron needs
  its own entry kind or a split count; facilities have no state field.
- **Open trace (settled in Stage A).** Retreat output 3 turned on
  `FUN_005513a0`. Stage A shows an unfinished member does not refuse the
  candidate, so the container change does not close EVT-02-C008.

## Design (for the next stage)

1. **State.** Add `completed: bool` (serde default true) to `ShipInstance`,
   `TroopUnit`, `SpecialForceUnit` and the three facility instances; split
   `FighterEntry` into finished and unfinished entries (an `unfinished:
   bool` on the entry; a count never mixes the two). Add an en-route
   record per member (`arrival_tick`, `destination`) for products
   travelling after completion, replacing `DeliveryState`'s list.
2. **Walk helpers.** Add mode-named iterators on `Fleet` and `System`
   (`usable_ships()`, `existing_ships()`, and the same for squadrons,
   regiments, special forces, facilities) and replace every direct read
   with the helper whose mode the trace gives. Defaulting all existing
   reads to the Usable helper keeps today's behaviour; each mode-3 or
   mode-4 consumer is then switched with its own citation and test.
3. **Order (GameEffect).** A build order's fire emits
   `GameEffect::ProductPlaced { system, queue index, container, member }`.
   The integrator creates the member, unfinished, in the planned container
   (ships: the side's bit-1 spare at the target, created if absent and
   made real per `FUN_004fe630`), and the queue item keeps the member's
   identity (fleet DatId plus `ShipInstance::id`, or the unit's key).
   Economy still precedes manufacturing.
4. **Completion.** `CompletionEvent` becomes "set Completed on this
   member", plus EnrouteActive and an arrival tick when the yard's system
   differs from the member's (`FUN_0052bee0`). Arrival clears it. A
   destroyed destination keeps today's reroute rule.
5. **Consumers.** In order: maintenance (count Existing, so deliveries en
   route cost), the Fleet window and fleet icon (mode 3, marks
   11570/11572), the unfinished Death Star in battle (`FUN_0040b0e0`
   `0x40b4d2`), unfinished ships never chosen as carriers, Move enabled
   for an unfinished ship, fleet, squadron or regiment (A0 `u-2`, `u-3`),
   Scrap grey until completion. Retreat output 3 needs no change
   (Stage A).
6. **Save.** Bump `SAVE_VERSION` to 124 with a migration that, for each
   queue item, creates its unfinished member in the planned container and
   records the identity in the item, and turns each `Delivery` into a
   completed, en-route member of its destination's container.
7. **Tests.** One behaviour test per consumer citing its walk
   (`FUN_005001b0(fleet, 1)` for battle, `FUN_004a3340` mode 3 for the
   Fleet window, `FUN_0052fff0` for maintenance, and so on), the migration
   round trip, and the seed-42 replay golden regenerated in its own commit
   (cause: products in containers).

## Stage A (2026-11-15)

**Settled by static trace** (`ghidra/notes/unfinished-products.md`,
"Settled"):
- `FUN_005513a0` runs on the leg checker that `FUN_005571c0` builds, with
  the candidate's key. Its `+0xc` is the checker's status pair, not a key.
  Members' `vt74` answers fill the per-family capacity table, not that
  status. An unfinished member therefore never refuses a Retreat
  candidate.
- A regiment, squadron or special-force product lives in its target
  system (`System_vt78` accepts them without limit, and `FUN_00552300`
  scores the target first). Only capital ships go into the bit-1 spare.
  Unfinished ships are never chosen as carriers (mode-1 step 3).
- A moving fleet carries its unfinished ships (`FUN_00556390`,
  `Fleet_vtf4` splits nothing). They get no en-route bit. On completion
  the ship travels from the yard to wherever its fleet is.
- An unfinished squadron or regiment that is moved changes container at
  once, with no trip (`FUN_00556390`: not Deployed, so not mobile).

**Design consequences for Stage B.** Nothing in the core design changes,
but the work gets smaller:
- Only capital ships need a fleet member flag. Regiments, squadrons and
  special forces become unfinished members of `System::ground_units`,
  `System::fighters` and `System::special_forces`. Facilities become
  unfinished members of the system's facility lists.
- The order places a capital ship in the side's spare fleet at the target.
  Everything else goes to the target system.
- Completion sets Completed, and starts a delivery trip from the yard when
  the member's system differs: a trip per member (as today's `Delivery`,
  but keyed to the member), not a container change.
- Carrier choice and fleet speed and mobility skip unfinished ships.

**Fixed in Stage A.** `maintenance_totals` now charges deliveries en route
(`FUN_0052fff0`, every Existing object). Test:
`a_product_en_route_is_charged_to_its_side`.

**Not done in Stage A: Move for an unfinished squadron or regiment.**
The original's Move moves the product into the target container at once
(a planet, or a carrier ship through `FUN_00552300`). Today the port has
no object to move:
- the row is a queue entry, keyed by system and class only;
- a queue item can name only a destination system, never a ship.

Turning the row white would offer an order that cannot carry out the
original's effect. It goes into Stage B, where the product is a system
member.

**A0 run "u-"** (`.artifacts/a0-vm/2026-11-15-partial-cells/README.md`,
summarised in the note) answered the four checks:
1. the fleet icon appears, and the spare is "Fleet 4" at once;
2. the Fleet window lists the ship over the side's grid, and Status reads
   "Under Construction";
3. maintenance drops at Confirm, as the port already charges queued
   items;
4. with only the unfinished Carrack there, the Alliance took Corellia with
   no battle alert.

It also showed:
- an unfinished ship, its fleet and an unfinished squadron all keep Move
  and Confirmed Move white, with Scrap grey until completion;
- a second order at the same yard replaces the first, and the spare goes
  with it (the port already replaces the area's queue).

The first trace's "a capital ship refuses a Move while unfinished" was a
misreading: `CapShip_vt74` is the container check. Stage B therefore also
enables Move for an unfinished ship and its fleet. What that Move does is
open.

**Stage B additions from A0:**
- removing the spare when its only products are replaced or scrapped;
- the band's one-day lag in naming the new fleet;
- Status "Under Construction" for a ship. A squadron has no Status row.

## Small finding outside the architecture

`side_resources::maintenance_totals` charged queued items but not
deliveries en route. In the original a product en route is Existing and
costs maintenance (`FUN_0052fff0` sums every object with bit 6). Fixed in
Stage A.

## Stage B (2026-11-15)

Ported in `rebellion_core::products` (save v124):

- Each ordered ship, squadron, regiment or special force is a `Product`
  record in its container from the order on: a ship in the side's bit-1
  spare at the target, the others in the target system.
- Two ways the port differs in form, not in what the walks see:
  - The record stands in for the original's Existing-not-Completed object.
    The port's object lists keep only finished units, so its mode-1 walks
    (battle, Retreat, carriers, fleet speed) skip products as the
    original's do. The mode-3 consumers read the records: the Defenses
    window rows, the Fleet window's Capital Ships tab, the fleet icon and
    the Status windows.
  - A spare holding only unfinished ships is *staged*: it is numbered at
    once, and `movement::listed_fleets` lists it, but it is not in
    `System::fleets`, so blockades and battles ignore it (A0 "u-" run 2).
    Its first finished ship lists it.
- **Placement.** `products::reconcile` runs after every panel action, at
  the manufacturing step and before the day's views:
  - it places new orders' units;
  - it drops replaced, stopped or cancelled ones, with a spare they empty
    (A0 "u-3");
  - it follows a Destination change.
- **Completion.** `products::settle` finishes a product in its container.
  A ship completes in its own fleet. A product away from its yard travels
  from the yard to it.
- **Moves.**
  - A moving fleet carries its products.
  - A Move of an unfinished squadron or regiment puts it in the released
    system at once.
  - hyp: an unfinished ship goes into a released fleet of its side, else
    into the released system's spare.
- **Menus and Status.**
  - Move is white and Scrap grey for an unfinished squadron, regiment or
    ship.
  - An unfinished ship's Status reads "Under Construction" over the
    side's grid.
- Maintenance still counts the queue, which equals the products.

Still port deviations:

- **Ship menu.** An unfinished ship's Rename is grey (A0 white).
- **Fleet window.** The left list's tree does not list an unfinished
  ship, and its drag is not ported.
- **Moves.** A squadron or regiment released on a Fleet window is
  refused, and Confirmed Move asks nothing.
- **Fleet icon.** A staged spare takes no icon Move or Scrap.
- **Trips.** A finished product on its trip is a delivery, not a member
  of its container. Its staged spare waits empty and invisible until the
  ship arrives.
- **Facilities** keep their queue-built rows.
- **Death Star.** The unfinished Death Star's part in a battle is not
  ported.
