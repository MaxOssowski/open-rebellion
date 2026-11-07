# Maintenance Shortfall for a ship in hyperspace (2026-11-07)

This covers EVT-01-C006 (shortfall) and CMD-08-C014 (Display).

## A0

The unedited "test" save (Alliance, day 1224):
- `.artifacts/a0-vm/maintenance-shortfall/test-day1224-index.png` (sha256 `cae9be50830326bc74bd432267ac28662d447e33d85d947ca8af6b30a07e5035`) lists "Maintenance Shortfall" first;
- `test-day1224-corvette-no-system.png` (sha256 `de9a33ead75083b3565ba09a90d1fdf2bf0c5252086239b3c4ec6663e08258ef`) shows it.

What A0 shows:
- "The following units have been destroyed at  due to a maintenance shortfall:" over "Corellian Corvette 2". The system is empty: `FUN_00498970` finds no system up a fleet in hyperspace.
- The picture is STRATEGY 1013 (`0x3f5`, the Alliance reader's), 0 differing pixels.
- Display is lit (its art 0 pixels from Near Uprising's lit button).
- Pressed, Display did nothing and the index stayed open (`FUN_00467f10` asks `FUN_004f2d10` for a system first).

## Three differences

1. **The unit's name.** The port listed the class's name. The object's `+0x30` is the ship's own name, its class name numbered (`FUN_004f6270`).
2. **The place.** The port named the fleet's `location`, even for a fleet in transit.
3. **Display.** A message without a system had a grey Display in the port.

## Now

- `autoscrap::candidates` names a capital ship through `GameWorld::ship_name`. `the_pick_runs_over_the_side_s_maintained_objects_by_type` asserts the class name, then the ship's own once it has one.
- `main.rs`:
  - checks whether a scrapped ship's or squadron's fleet is in transit (`MovementState::is_in_transit`) before the unit is removed;
  - `maintenance_message` then leaves the system out and marks the target unplaced.
- `GameMessage::unplaced_target` and `with_unplaced_target`: such a message lights Display (`message_index::display_lit`), and its press does nothing.
- **Tests:**
  - `display_is_lit_for_an_unplaced_target_and_its_press_does_nothing`;
  - `a_maintenance_shortfall_in_transit_names_no_system_but_keeps_its_target`: A0's text, no system, unplaced, Manufacturing, `0x3f5`, event `0xc`; with a system `0x3f6` for the Empire.
