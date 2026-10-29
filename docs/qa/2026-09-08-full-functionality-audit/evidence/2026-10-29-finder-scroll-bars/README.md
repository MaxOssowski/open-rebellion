# The Personnel and Troop Finders' scroll bars (P09)

Date: 2026-10-29. Tracker item "Panel i menu floty, rodzina okien
statusu". This closes the scroll bars left by
`../2026-10-29-personnel-troop-finder-text/`.

## The original

Both lists hold `FUN_0060a490(.., list, 7, 0x299d, ..)`, RCDATA 10653:
- Personnel: `FUN_004637a0`, list 161 high, rows 20;
- Troop: `FUN_0046ce40`, list 159 high, rows 25.

They scroll by pixels; the arrows step 10 (`FUN_00607ea0`'s argument).
The bar hides when everything fits (`FUN_0060b5d0`).

## The port

- `list_scroll_bar::ui_bar`, a shared egui helper: the press, hold and
  release on the bar, then its pieces, the held arrow drawn pressed. Both
  Finders use it.
- Both Finders:
  - scroll by pixels (`scroll`, formerly `first_row`) with `scroll_hold`;
  - a row's text keeps its whole row's place while its click area is
    clipped to the list and ends at the bar;
  - the wheel moves a row (port).

A bug found by the new tests and fixed before commit: the row text was
first anchored to the clipped row, which pushed a half-hidden row's text
to the list's edge.

## Browser

The gate `tools/interface-parity/encyclopedia-placement.mjs
--window=finders` passed for both sides at `…T12-36-27-596Z`. The WASM
hash is `281b5bdd…`. The new games' lists fit, so no bar shows, as in the
original. The A0 Troop Finder's bar (Empire, day 167) shows when its rows
overflow.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `the_bars_down_arrow_scrolls_the_list_ten_pixels_and_moves_its_rows`
    (Troop): the arrow steps 10, the rows move, rows below and above the
    list are not drawn, a press at x 370 chooses the row, a press on the
    track chooses none, the wheel, the pressed arrow;
  - `at_scale_two_the_row_text_keeps_its_window_place` (Troop), now also
    the bar at scale 2;
  - `scrolled_rows_clip_to_the_list_and_keep_their_centres` (Personnel):
    rows clipped above and below, the bar's column chooses nothing, x 370
    chooses row 4, the wheel.
- Changed expectations, from rows to pixels:
  - `the_list_scrolls_as_little_as_it_must_to_show_a_row` (both);
  - `eight_whole_rows_fit_the_list` and `six_whole_rows_fit_the_list`.
- `cargo mutants --in-place --in-diff`:
  - 133 mutants, 32 missed at first;
  - after the tests, the 10 left are equivalent:
    - the default pointer `(-1, -1)` (2);
    - `scrolled_to`'s boundaries (4);
    - the wheel's `>` behind its `!= 0` guard (2);
    - `(line + 1) * h <= 0` against `/ h`, the same sign (2).
