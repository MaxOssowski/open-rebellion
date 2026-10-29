# The Message Index rows as the original draws them (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy" (the Advice list). This
closes the open point of `../2026-10-29-agent-advice/`: the port's Message
Index rows differed from the A0 Advice list.

## The original

- **Order** (`FUN_00468ab0`). The function walks the player's messages from
  the tree's last node (`thunk_FUN_005f5080`) back by predecessors
  (`FUN_005f5c60`), so the newest heads the list.
  `ghidra/notes/message-index-rows.md` had "oldest first"; it is
  corrected.
- **Item.** Each item (346 by 21) gets:
  - its category's icon, by the first mask bit in
    `FUN_00468ab0`'s order (STRATEGY `0x2a9a..0x2ad9`, some per side),
    blitted keyed (`FUN_005fd0f0`);
  - the chosen bar `0x2aa2` (Alliance) / `0x2aa3` (Empire);
  - its text at `+0x30` = 32.
- **Heading.** Each category control's label comes from `FUN_004665f0`
  (TEXTSTRA `0x8010..0x8018`, `0x8032` "Advice Messages").
  `FUN_004697b0` redraws the chosen one's label over the list.
- **Bar.** The list's scroll bar is RCDATA `0x299d`.

## A0 (`../2026-10-29-agent-advice/`, `01-advice-index.png`, `40b45302…`)

Measured on the Empire's Advice list, window at (125, 52):

- **Chosen bar.** It sits at the row's origin (150, 160). It shows 352 by
  17, without the bitmap's 4-pixel shadow or blue corners.
- **Icon.** It sits at (151, 161), (1, 1) in the row. Its palette index 0
  (black) is transparent. The other 88 pixels match the bitmap (mean
  difference 5).
- **Text.**
  - "Bombardments" starts at x 184, 72 wide, with caps 7 high from 4
    below the row's top. That is 12-pixel regular Arial (Liberation Sans
    gives 75).
  - The colour is grey `(117, 114, 114)`; the chosen row is white.
  - These rows were unread, so the item's `+0x38` (13 or 10) is not the
    drawn font. hyp: the list draws every item in one font.
- **Heading.** "Advice Messages" is white, x 160..260 (100 wide, font 10's
  14-pixel cell: 101.9), with caps from y 145.

## The port (`message_index.rs`, `bmp_cache.rs`)

- `filtered_messages` lists newest first.
- `row_layout` places each row's chosen bar (only its top-left 352 by 17,
  `chosen_bar_uv`), its icon at (1, 1), its text at (32, 2), and its click
  area short of the scroll bar. `row_icon` and `category_heading` hold the
  tables.
- The rows are in `original_font(8)`: grey `0x787878` (the Finders'
  list grey) or white when chosen. The heading is in `original_font(10)`
  at (35, 91).
- The list scrolls by pixels with the original bar
  (`list_scroll_bar::ui_bar`, `RESULTS_ART` = RCDATA 10653 = `0x299d`).
  hyp: its arrow step is the Finders' 10. port: a wheel notch moves a row.
- The icons 10906..10911, 10916 and 10963..10969 load with black keyed out
  (`uses_black_key_transparency`).

## Browser (Arch Chromium, release WASM `80138336…`)

- `agent-advice.mjs` passed for both sides at `…T22-27-54-553Z`
  (`agent-advice-result.json`). Screenshots:

  | Side | Advice index | Message |
  |---|---|---|
  | Alliance | `32b0c020…` | `8f132e96…` |
  | Empire | `d54b05a0…` | `918a4d7e…` |

  Both show the heading, the keyed icons, the side's bar and grey rows,
  newest first, as A0 does.
- `agent-objectives.mjs` passed at `…T22-28-14-784Z`.
- `message-index-shell.mjs` fails here (run through a Linux browser
  manifest) with 470 different pixels, the window's last row, in every
  case. HEAD 90cfe39f fails the same way in a clean worktree. That failure
  is old and not this change's.

## Open

The heading's "Post Messages" control (`0x66`, bitmaps `0x2a78..0x2a7b`)
is not drawn: A0 shows it left of Delete. It switches a category between
silent and alerted posting.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `each_category_has_its_rows_icon_and_heading`;
  - `the_list_scrolls_by_pixels_up_to_its_last_row_and_margin`;
  - `a_row_lays_out_its_bar_icon_text_and_click_area_scrolled_by_pixels`;
  - `the_chosen_bar_shows_its_top_left_and_the_heading_its_cell`;
  - `a_wheel_notch_scrolls_a_row_and_a_click_takes_the_row_under_it`;
  - `message_index_row_icons_key_out_their_black`.
- Changed expectations, for the trace and A0 (newest first):
  - `all_messages_shows_everything_except_advice`;
  - `messages_for_the_other_side_are_hidden`;
  - `report_reflects_the_filtered_rows_and_selection`.
- `list_geometry_matches_fun_004665f0_and_fun_006082c0` keeps its nine
  whole rows, now computed in the test, as the constant is gone.
- `cargo mutants --in-place --in-diff` (excluding the app's `main.rs`):
  112 mutants. The first run missed 48 in the drawing; moving the geometry
  into `row_layout` and testing it left 7. Two more tests kill the
  click-area and text-offset ones. The rest are equivalent:
  - the frame's size passed to `ui_bar`, which places the bar from the
    frame's origin only (4);
  - `wheel > 0.0` against `>= 0.0` under the `wheel != 0.0` guard (1).
