# fleet-move gate runs on Linux (2026-11-01)

## Problem

`fleet-move.mjs` never passed on this machine (every run since 2026-10-16
failed). Its confirmed-move case needs a point inside the confirmation window
that would select a system if the window did not take the pointer. It looked
only for bare map within 8 pixels of a system, but the fixture's sector window
(x 60..295) and Fleet window (x 300..535) cover the whole confirmation area
(Alliance x 93..433), so no such point exists: "no bare map point lies within
the pick radius of a system" (`gate-fleet-move-before.txt`).

## Change

- The fixture's fleet-move setup also reports each planet the open sector
  windows show (`sector_planets`, clipped to the window), checked in
  `the_fleet_move_scenario_leaves_the_players_fleet_alone_and_movable_beside_its_target`.
- The gate's probe falls back to such a planet inside the area: a click on it
  selects its system (`SectorWindowAction::SelectSystem`), so the control
  still proves the point would reach the game.

## Result (Linux Chromium)

All 12 cases pass (`gate-fleet-move.txt`). The confirmed-move probe is planet
122 at (234, 85) for the Alliance and (294, 90) for the Empire: the click
changes nothing while the window is open, and selects 122 after the checkmark.

No A0 comparison: the cells stay partial.

## Mutants

`--re fleet_move_setup`: 13 tested, 9 caught, 1 unviable, 3 missed:
`emit_fleet_move_setup` (the browser emit, covered by this gate) and two
`FleetLoadObservation` fields matched by the pattern, outside this change.
