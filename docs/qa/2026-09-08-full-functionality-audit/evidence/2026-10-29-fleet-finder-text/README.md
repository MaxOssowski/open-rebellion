# The Fleet Finder's text, rows and scroll bar against the original (P09)

Date: 2026-10-29. Tracker item "Panel i menu floty, rodzina okien
statusu", the review of the fleet family's windows. The reading is in
`ghidra/notes/fleet-finder.md`.

## A0 (original game, Windows 10 VM, Empire, day 167)

`fleet-finder-empire-day167.png` stays outside Git; its hash is in
`a0-captures.sha256`. F3 opens the Fleet Finder. It shows:
- the title centred, bold;
- "Fleet Name" in a larger font;
- "All Fleets" bold;
- the rows Fleet 1, Fleet 1, Fleet 2, Fleet 3, Fleet 4, grey, 20 apart,
  at x 41;
- the scroll bar beside the list.

## The original

`FUN_00461960` sets up the list the same way the Encyclopedia sets up its
index:
- `FUN_00607ea0(.., 0x24, 0x8a, 0x15e, 0xa5, ..)`: (36, 138), 350 by 165,
  rows 350 by 20;
- `+0xe4 = 0x24`, `+0xe8 = 5`;
- the constructor's colours: grey `0x787878`, the chosen row white;
- the bar `FUN_0060a490(.., list, 7, 0x299d, ..)`.

The note already gave the fonts: the title in font 5, the name label in
font 4, the tab's label in font 5.

## The port (`fleet_finder.rs`)

- The title is centred in its 350 pixels, in font 5. The name label is in
  font 4, the tab's label in font 5. The list and name box are in font 10.
  The stand-in sizes are gone.
- Rows:
  - text at x 5, centred on the row;
  - grey, with the chosen row white, where it had been yellow (a hyp);
  - scrolled by pixels (`scroll`; the report's `first_row` is now
    `scroll / 20`) with the bar, RCDATA 10653, arrow step 10;
  - clipped to the list. A row's hit area ends at the bar.
- The wheel moves a row (port).

## Browser comparison

The gate `tools/interface-parity/encyclopedia-placement.mjs
--window=finders` ran at `…T11-45-19-451Z`. The WASM hash is `bf68ea74…`.
`empire-fleet-finder.png` matches A0's title, labels, tab label, grey
rows and placement. The fleets differ because the two games differ.

The Personnel and Troop Finders still draw stand-in fonts and their own
rows. They are the next stage: multi-column rows, and the Troop Finder's
rows are 25 pixels tall.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `the_bars_down_arrow_scrolls_ten_and_the_report_counts_whole_rows`;
  - `scrolled_rows_clip_to_the_list_and_the_bars_column_chooses_nothing`.
- Changed expectations, for the trace and A0 above:
  - `the_list_scrolls_as_little_as_it_must_to_show_a_row`, now in pixels;
  - `the_list_shows_its_rows_from_the_scroll_grey_and_the_chosen_one_white`,
    formerly `..._chosen_one_in_yellow`: text at x 5, nine rows shown in
    part at a 40-pixel scroll, grey and white;
  - `the_wheel_over_the_list_scrolls_it_a_row_at_a_time_within_its_rows`:
    reads `scroll / 20`;
  - `the_controls_lie_where_they_draw_and_rows_above_the_first_are_hidden`:
    a row's hit area is 338 wide;
  - `the_title_label_and_tab_name_follow_the_mode_and_tab`: the title
    centred, fonts 5 / 4 / 5.
- `cargo mutants --in-place --in-diff`:
  - 92 mutants, 22 missed; after the new tests, 5;
  - all 5 are equivalent:
    - the default point `(-1, -1)` (2);
    - the wheel's `> 0` behind its `!= 0` guard (1);
    - `scrolled_to`'s boundaries, where both give the same scroll (2).
