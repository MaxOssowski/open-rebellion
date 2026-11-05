# Build Selection's list order against A0 (2026-11-05)

This covers OBJ-08-C001 (ships), C002 (fighters), C005 (facilities) and C007 (available).

## A0

A new Expert-difficulty Alliance game, days 3..5, Build Facilities and Build Ships from the Agent menu on Sluis Van. The captures are in `.artifacts/a0-vm/build-selection3/`. The class list was scrolled through, ten pixels per arrow click.
- **Facilities** (`b1`, `l*`, `m*`): Construction Yard, GenCore Level I, KDY-150, LNR Series I, Mine, Orbital Shipyard, Refinery, Training Facility.
- **Ships** (`ships0`, `k*`): Alliance Dreadnaught, Alliance Escort Carrier, Bulk Cruiser, Bulk Transport, Corellian Corvette, Medium Transport, X-wing, Y-wing. The fighters stand among the ships by name.

Both lists are in name order. `FUN_00437880` inserts each class's item into a sorted list (`FUN_005f59f0`, the list's compare at vtable `+4`).

## Before

The port's dialog showed `listed_classes`' `DatId` order. For facilities that was KDY-150, LNR Series I, GenCore Level I, Orbital Shipyard, Training Facility, Construction Yard, Mine, Refinery: the same classes in another order.

## Now

- **`build_selection::in_list_order`.** It sorts the dialog's classes by their names (hyp: byte order; the list's compare function is not read). Both places that open the dialog (the band menu's Build and the Agent menu's Build orders) call it.
- **The AI.** It keeps `listed_classes`' id order, so its choices and the replay goldens are unchanged.
- **Unit test.** `the_list_shows_its_classes_by_name` (rebellion-render) covers the sort. Mutants: 1 caught, 1 unviable.
- **Real-data tests** (rebellion-app, `--ignored`, on the original DAT files).
  - `a_new_alliance_games_construction_yard_lists_the_facilities_as_a0` matches A0's eight facilities in order. Without the change it failed with the id order above.
  - `a_new_alliance_games_shipyard_lists_ships_and_fighters_by_name_as_a0` matches A0's eight ships and fighters in order.
- **The b84 batch.** 20 of its 21 Linux gates passed. `build-selection` failed: its two cases assumed a training facility opens on a regiment. Sorted by name, the Empire's opens on Bounty Hunters.
  - The portrait case now knows the special forces' portraits (their minis less 0x4000) and skips pure blue, the GOKRES portraits' key (bmp_cache), since 1412's first pixel is not blue.
  - The over-maintenance case steps to the Imperial Army Regiment that A0 manufacturing/11 shows: Bounty Hunters cost no maintenance, so their Confirm stays live, as an order with no upkeep passes.
  - With those changes `build-selection` passes, and so does `agent-build` (outside the batch).

## Still open

The Training Facility's list (regiments and special forces, C004) and the defenses (C006) are not captured in A0. The port's Empire training list starts with Bounty Hunters by name; A0's Empire capture (manufacturing/11) shows the Imperial Army Regiment, which may be the class last built there (`DAT_006b28a0`).
