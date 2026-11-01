# The Fleet window against A0: a character aboard, damage, a selected ship (2026-11-02)

## A0 (VM Alliance game, days 635..728)

Captures under `.artifacts/a0-vm/fleet2/` (hashes in `a0-sha256.txt`),
each Fleet window at (305, 86), 235 by 304:

- `vjun-fleet5-admiral-{tab}` (day 676): Fleet 5 at Vjun, Corellian
  Corvette 2, with Garm Bel Iblis aboard. He was dragged from the Vjun
  personnel page onto the fleet's entry, then given Command > Admiral
  (`02-`, `03-` show the Command submenu on the planet and aboard). The
  fleet is selected and expanded. `vjun-fleet5-garm-{tab}` is the state
  before the order, not expanded.
- `umgul-fleet1-damaged-{tab}` (day 722), after the battle at Umgul:
  Fleet 1 with Corellian Corvette 1 (Hull Value 240:500, `06-`) and
  Medium Transport 1 (317:360, Hyperdrive Rating 0:100, `05-`) and two
  Alliance Fleet Regiments, selected and expanded; Fleet 3, Medium
  Transport 2 with two Alliance Army Regiments, damaged too.
- `umgul-corvette1-selected`, `umgul-transport1-selected`: the same window
  after a click on each ship's entry in the left list.
- `umgul-fleet1-{tab}`, `umgul-fleet3-{tab}`: the same fleets before the
  battle (the Kimanan state of 2026-11-01 again).
- `01-yavin-blockade-report`: "Yavin is now under blockade by Imperial
  forces" after the battle at Yavin.

## The port's copies

Fixture scenarios (`interface_test_fixture.rs`):

- `FleetWindowVjun` (code 81, `place_vjun`): Fleet 5 with Corellian
  Corvette 2 at full hull; Garm boarded (`missions::board_fleet`), seated
  (`seat_characters`) and given Admiral (`set_command`).
- `FleetWindowUmgul` (code 82, `place_umgul`): A0's hulls; Medium
  Transport 1's primary hyperdrive hit (CAPSHPSD `hyperdrive_if_damaged`
  is 0); Medium Transport 2 one point short of its hull (A0 shows its mark;
  its hull was not read).

`fleet-window.mjs` cases `alliance/a0-vjun` and `alliance/a0-umgul` compare
each tab and, for Umgul, each selected ship with A0
(`--a0-fleet2=DIR`, default `.artifacts/a0-vm/fleet2`).

## What the comparison found and changed

1. **Personnel indicator** (`FUN_004a66a0` flag 4, 10406 at x 57): A0
   marks Fleet 5's entry and Corellian Corvette 2. A character marks the
   ship it rides on (`Character::ship`), an unseated one the first living
   ship (`seat_characters`' rule); hyp: a special force, which keeps no
   seat, marks the first ship.
2. **Tree lines**: A0 draws them solid, `LineTo` leaving the end point
   (x 6 from y 35 to 84 unbroken; branches over x 6..8), under the
   selection frame. The port drew one-pixel dots that egui's feathering
   lost in the browser; it now draws solid unfeathered lines first.
3. **No-hyperdrive mark** (flag `0x40`, 10430 at x 9): A0 marks Medium
   Transport 1 and Fleet 1's entry. The mark is the ship's speed
   (`FUN_00500820`) being 0: its hyperdrive less hits, else its backup's,
   else the default only while its DeploymentCount (`+0x41`, raised by
   `FUN_004fb310` on each move) is 0; hyp: a ship in a fleet has moved.
   A fleet's speed (`FUN_004fd900`) is 0 once a ship's is, so its entry
   carries the mark. The port marked only a class without a hyperdrive.
4. **A selected ship** (`umgul-*-selected`): its entry takes the frame
   10401; the label `+0x190` names it; the strip shows three tabs (no
   Capital Ships: `b + 3` is Personnel's normal art, so there is no
   disabled art for it); its tabs light by its own content flags (the
   transport's Troops). The port drew no frame and no name and four tabs.

## Remaining differences

Only GDI text hinting (the accepted text-width deviation) and, on the
selected-ship captures, nothing else:

| State | Differing pixels | Ceiling |
|---|---|---|
| Vjun ships / fighters / troops / personnel | 1337 / 884 / 884 / 1245 | 1500 / 1000 / 1000 / 1400 |
| Umgul ships / fighters / troops / personnel | 2278 / 1311 / 2678 / 1213 | 2400 / 1400 / 2800 / 1300 |
| Umgul Corellian Corvette 1 / Medium Transport 1 selected | 1648 / 1647 | 1800 |

Kimanan (`a0-kimanan`) fell from 2066 to 2061 pixels on its ships tab
with the solid branch.

## Not yet compared

- A selected ship's lists: A0's Troops list for the transport was not
  captured (a second battle interrupted it); the port lists nothing for a
  ship.
- Fighters aboard (no carrier in this game), split, merge and transfer.

## Tests

- `a_character_marks_the_ship_it_rides_on_and_an_unseated_one_the_first`,
  `the_tree_lines_are_solid_and_leave_out_their_end_points` (replaces
  `the_tree_is_dotted_one_pixel_on_and_one_off`, which A0 contradicts),
  `a_ship_whose_hyperdrives_are_both_hit_carries_the_indicator_and_so_does_its_fleet`,
  `a_selected_ship_is_framed_named_and_shows_three_tabs_lit_by_its_own_cargo`
  (`fleet_window.rs`): each fails without its change.
- `expanding_an_entry_marks_only_that_fleet_in_that_open_window`.
- `the_a0_vjun_scenario_seats_admiral_bel_iblis_aboard_fleet_5`,
  `the_a0_umgul_scenario_lists_fleet_1_damaged_and_fleet_3_loaded`
  (`interface_test_fixture.rs`).

## Run

`fleet-window-2026-11-01T23-16-48-048Z-2400896` (Arch Chromium 155, not
the pinned build): 75 cases pass, with the counts above. Test WASM
`3e7bd6e41902685a7eebbf73fb552782573eecb8f6e5948009e574273722ed48`.
The batch's other gates (fleet-move, sector-quadrants, personnel-orders,
status-window, build-selection, mission-dialog, the finders, the
registry, both index shells, run --all, encyclopedia-art) pass; the
workspace passes 3299 tests and the fixtures 106.

`cargo mutants --in-diff` on `fleet_window.rs`: the first run missed 10
(`expand`, `is_expanded`, a ship's Fighters tab, the line's width); with
the tests above all 43 viable mutants are caught.
`status_window::tests::the_marks_are_painted_under_the_picture_at_its_origin`
failed in two of cargo-mutants' baseline runs (its portrait 2114 not
painted) and passed in the workspace run, 40 lone runs and 13 full-suite
runs; the last mutants run skipped it. It is flaky and not changed here.
