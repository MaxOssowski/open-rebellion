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
   | Retreat output 3, `FUN_005513a0`'s mode-4 walk | 4 | may post `0x20` (**open**) | differs if it does |
   | Fleet Usable / mobile / speed | 3, Completed only | ignored | same for mixed fleets; a fleet of only unfinished ships cannot exist in the port |
   | Capital ship as its own mover (`CapShip_vt74`) | object | refused `0x20` | n/a (not an object) |
   | Squadron/regiment Move | object | allowed (A0 `pending-facility/24`) | differs: grey (known port note) |
   | Fleet window lists (`FUN_004a3340`) | 3 | listed with mark 11570/11572 | differs: ships not listed |
   | Defenses window pages | 3 | listed | same (`DefensesItem::Pending`) |
   | Fleet icon (`FUN_004feea0(.., 3)`) | 3 | a spare with only unfinished ships shows the icon (hyp) | differs |
   | Maintenance (`FUN_0052fff0`) | bit 6 | costs from the order, and while en route | queued items cost; **deliveries en route do not** |
   | Enemy's view copy (`FUN_004f9a60`) | object | frozen | n/a |

5. It is finished when progress reaches the cost (`FUN_0052b960` ->
   `FUN_0052bee0`): Deployed, EnrouteActive when the yard's system differs,
   Completed. It never changes container on completion.

So for battle and for Retreat's outputs 1 and 2 the port's current split
(products outside fleets) already gives the original's result. The
differences are in the windows, the fleet icon, maintenance during
delivery, the unfinished Death Star, a moving fleet's unfinished members
(open), and possibly Retreat output 3 (open).

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
- **Open trace.** The motivating case, Retreat output 3, turns on
  `FUN_005513a0`, which is not settled. Porting the containers without it
  would not close EVT-02-C008.

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
   `0x40b4d2`), the capital-ship mover refusal, squadron/regiment Move
   enabled while unfinished, scrap of an unfinished product (refund rule
   to trace), then Retreat output 3 once `FUN_005513a0` is settled.
6. **Save.** Bump `SAVE_VERSION` to 124 with a migration that, for each
   queue item, creates its unfinished member in the planned container and
   records the identity in the item, and turns each `Delivery` into a
   completed, en-route member of its destination's container.
7. **Tests.** One behaviour test per consumer citing its walk
   (`FUN_005001b0(fleet, 1)` for battle, `FUN_004a3340` mode 3 for the
   Fleet window, `FUN_0052fff0` for maintenance, and so on), the migration
   round trip, and the seed-42 replay golden regenerated in its own commit
   (cause: products in containers).

## What to settle before porting

- `FUN_005513a0`: which object the route record's `+0xc` key names, and
  whether a `0x20` posted by a member's `vt74` there fails
  `FUN_005545e0`'s acceptance (`status[1] == -1`).
- The containers of regiment, squadron and special-force products
  (`FUN_00552300`, `FUN_00550af0`, `FUN_0054f5b0`).
- A moving fleet's unfinished members (hyp: they go with it).
- A0, four steps: order a capital ship at a system with no fleet of the
  side; check (1) the fleet icon appears, (2) the Fleet window lists the
  ship with the red grid, (3) the maintenance monitor rises at the order,
  (4) a battle there before completion leaves the ship out of the forces
  list.

## Small finding outside the architecture

`side_resources::maintenance_totals` charges queued items but not
deliveries en route. In the original a product en route is Existing and
costs maintenance (`FUN_0052fff0` sums every object with bit 6). This can
be fixed on its own (count `DeliveryState::en_route()` by side) before the
architecture change.

No code changed, so no tests, mutants, provenance scan or browser gates
ran.
