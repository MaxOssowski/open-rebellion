# The Fleet window against A0: split, transfer, merge and a ship's Confirmed Move (2026-11-02)

## A0 (VM, the `test` Alliance save, days 9..21)

Captures under `.artifacts/a0-vm/fleet3/` (hashes in `a0-sha256.txt`).
The Yavin Fleet window is at (305, 86), 235 by 304:

| Capture | State |
|---|---|
| `01` | Fleet 1, with Corellian Corvette 1 and Medium Transport 1 (two regiments), is selected and expanded. |
| `02` | A right click on Medium Transport 1. It selects the ship: frame, name, the Fighters tab with counters 0 and 0. Its menu has Confirmed Move white. |
| `03` | After Create Fleet: Fleet 6 holds the transport. Nothing is selected and the pane is gone. Fleet 1 stays expanded. |
| `04`, `05` | The corvette's Move, released on Fleet 6's entry. The corvette joins Fleet 6, Fleet 1 disbands, and nothing is selected. |
| `06` | Fleet 6, double-clicked: expanded, selected, still on Fighters. It lists the corvette first, then the transport. |
| `07`..`10` | Create Fleet on the corvette makes Fleet 7, listed after Fleet 6. Fleet 7's Move onto Fleet 6 merges it; Fleet 6 lists the corvette, then the transport. |
| `11`, `12` | The corvette's Confirmed Move onto a new fleet asks "Transit time in days / Corellian Corvette 1:  0" over the fleet picture. The checkmark joins it there. |

## The port's copy

`FleetWindowYavin` (code 85, `place_yavin`) recreates `01`. It now
reports the object menu.

`fleet-window.mjs` `alliance/a0-yavin` replays `02`..`10` with the
port's own menus and clicks, comparing each step with A0.
`alliance/a0-yavin-confirm` replays `11` and `12`.

## What the comparison found and changed

1. **Ship names in the left list.** The port drew none. Each ship entry
   now names its ship where a fleet entry does: (7, 3), font 10, clipped
   by the list ("Medium Transpor").

   Earlier A0 comparisons (Kimanan's ships tab, Vjun, Umgul, Selonia)
   counted these missing names as text differences. Their ceilings rise
   now that both sides draw the names.
2. **A right click selects** the left-list entry it opens a menu for.
3. **A ship selected over Capital Ships shows Fighters.** A ship has no
   Capital Ships tab. The fleet keeps Fighters when it is selected again.
4. **The selection drops when its fleet gains or loses a ship.** This
   covers Create Fleet, a ship's Move onto a fleet, and a fleet's Move
   onto another. hyp: the list rebuilds and loses its selection.
5. **A ship's Confirmed Move.**
   - It is white, as its Move is. The test that kept it grey had no A0
     source and is changed.
   - It opens the move confirmation with one line per ship. The checkmark
     joins the fleet, or forms a fleet at the system, as Move does.
6. **The move confirmation's text.**
   - Font 4, lines 16 apart, as A0 sets it. Before, it was egui's 11-point
     stand-in, smaller than A0's.
   - `DT_WORDBREAK` keeps a run of spaces inside a line (`theme::gdi_word_break`).
7. **The move confirmation's checkmark and X (10926..10930)** are keyed on
   their first pixel. The panel shows through their blue corners.

## Remaining differences

| State | Differing pixels | Ceiling |
|---|---|---|
| `01` | 2519 | 2700 |
| `03` | 1061 | 1200 |
| `05` | 365 | 500 |
| `11`, the dialog's 424 by 322 | 1557 | 1700 |
| `12` | 366 | 500 |

- The text is hinted differently, which is an accepted deviation.
- The new fleet's number differs: A0 shows "Fleet 6"; the port numbers it
  from its own world.

## Open

- **Order.** A0 lists a ship that joins a fleet before the fleet's own
  ship, and a new fleet after the older one. The port appends joined
  ships and sorts fleets by slotmap key; a disbanded fleet's slot is
  reused, so a new fleet can sort first.
  - hyp: both follow object creation order.
  - This is a core change, left for its own stage.
  - Steps `06`..`10` are captured and compared but not asserted.
- **A stale frame.** A0 `06` still frames Medium Transport 1, the entry
  last right-clicked. The list control's selection rules are untraced.

## Tests and run

- `an_expanded_fleet_s_ships_are_named_in_the_left_list_where_the_fleet_is`,
  `a_right_click_on_a_left_list_entry_selects_it_and_opens_its_menu`,
  `a_ship_selected_over_capital_ships_shows_fighters_and_its_fleet_keeps_them`,
  `the_selection_drops_when_its_fleet_gains_or_loses_a_ship`
  (`fleet_window.rs`): each fails without its change.
  `the_checkmark_carries_a_ships_confirmed_move_and_the_fleet_it_joins`
  (`move_confirmation.rs`),
  `word_break_keeps_a_run_of_spaces_inside_a_line_but_not_at_its_ends`
  (`theme.rs`),
  `the_move_confirmations_buttons_are_keyed_on_their_first_pixel`
  (`bmp_cache.rs`), `the_a0_yavin_scenario_selects_fleet_1_with_two_ships_and_two_regiments`
  (`interface_test_fixture.rs`). `a_capital_ships_move_and_create_fleet_follow_its_fleets_move_rule`
  now expects Confirmed Move enabled (A0 `02`).
- `cargo mutants --in-diff` on the render changes: 30 caught, 5 unviable
  (the flaky `status_window` marks test skipped, see 2026-11-02-fleet-window-a0-vjun-umgul).
- `fleet-window-2026-11-02T01-44-21-570Z-2884371` (Arch Chromium 155): 79
  cases pass. Test WASM
  `48a3870470f829c021a6688e5b7102220aaa9c8f3e4c3f70d3e1eb5c7c7c60a8`. All
  other gates of the batch pass (personnel-orders' confirmations with the
  new font among them); workspace 3310 tests, fixtures 109.
