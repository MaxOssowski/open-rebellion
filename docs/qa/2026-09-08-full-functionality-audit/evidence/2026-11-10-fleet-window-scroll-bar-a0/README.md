# The Fleet window's right list gains its scroll bar

Date: 2026-11-10. Follows `2026-11-10-fleet-aboard-a0-captures`. Adds
to OBJ-06-C010 (contents) and OBJ-10-C006 (aboard fleet).

## The original

`FUN_004a4b10` gives each of the Fleet window's lists a scroll bar,
`FUN_0060a490(.., 0x29cb, ..)` (`ghidra/notes/fleet-window.md`). The bar
reads STRATEGY RCDATA 10699, read from STRATEGY.DLL with a PE resource
walk:

    [10, 10657, 10692, 10693, 10694, 10695, 10696, 10697, 10698, 10684]

That is width 10, track 10657, up arrow 10692/10693, down arrow
10694/10695, and thumb 10696/10697/10698, in the layout
`list_scroll_bar` already reads for RCDATA 0x299a, 10653 and 0x29fc.

A0 `fleet-aboard/10-isd1-fighters` shows the bar beside Imperial Star
Destroyer 1's six squadrons, which pass the list's 164-pixel height. It
stands at window x 224..233, from y 127 to 290: the right list (101,
127, 133 by 164) less the bar's width. The 13-wide pieces are clipped to
10 and centred, as the module draws them.

## The port before

No scroll bar, and no scrolling. A list showed its first rows only.

## The change (`fleet_window.rs`)

- `LIST_BAR_ART` holds RCDATA 10699.
- Each window keeps the right list's first visible pixel and the press
  held on its bar (`right_scroll`, `right_hold`). The right list draws
  its rows from that offset and clips them.
- The bar shows once the rows pass the list's height. It is drawn and
  driven by `list_scroll_bar::ui_bar`: the arrows step 10 pixels, which
  is not read for this window (`hyp:`, as the Status window's list). The
  track pages, and the thumb drags. A press on the bar does not choose
  the row under it.
- The row rectangles, the targeting release's row and the Capital Ships
  drop row all take the offset.
- A new selection or tab starts the list at its top again.
- The arrows 10692..10695 are keyed on pure blue (`bmp_cache`), as every
  other list bar's arrows are. Their bottom-left pixel is blue.

## Tests

- `a_scrolled_right_list_moves_its_items_up_and_resets_on_a_new_selection`
  (rebellion-render). A row's rectangle moves up by the offset, and
  `select` resets the offset. It fails without the change, where the
  rectangle stays at 177.
- `the_right_lists_rows_follow_its_scroll` and
  `another_tab_starts_its_list_at_the_top` check the row helpers,
  including the row that starts exactly at the list's bottom (y 291),
  and the reset on a tab change.
- `build_selections_buttons_key_out_their_first_pixels_colour` in `bmp_cache` now also lists 10692 and 10695 among its keyed ids.
- `fleet-window.mjs a0-coruscant` (Empire, Linux Chromium) checks the
  ship's Fighters tab:
  - the bar's 10 by 164 pixels are compared with A0, and 0 of 1640
    differ;
  - the whole window now differs from A0 in 3018 pixels, down from
    4292, all of it text and the pane title's clip. The ceiling was
    lowered to 3150.

  Run: `.artifacts/interface-parity/fleet-window-2026-11-10T13-42-44-329Z-703672`.

## Not done

- The left list's bar. Same art, 266 high. No A0 capture shows more than
  five entries, so its overflow is not compared.
- The wheel. The original's lists do not scroll by wheel (A0 kinds/02),
  and this one does not either.

## Mutants

`cargo mutants` on the diff of `fleet_window.rs` (copy mode): 65
mutants, 57 caught, 7 unviable, 1 missed (`<` to `<=` in
`right_row_top`). A new assertion on the row starting at y 291 closes
it.

## The trench movie check

The tactical matrix of b174 failed `production-trench-run-failure`
once more. Escape came before the page requested `202.webm`, although
the console already read `[cutscene] opened .../202.webm`. `run.mjs`
now takes either the request or that console line as the proof that
the movie opened. Run alone, both trench cases pass 4 of 4.

## Batch b175

23 of 23 interface gates on Linux, `fleet-window`
(`fleet-window-2026-11-10T15-11-02-273Z-1189397`) among them with the
bar check; workspace tests 3503 passed, fixture tests 140; the in-place
mutants on the render diff 60 caught, 7 unviable. Tactical matrix
`2026-11-10T16-02-19-520Z-1261631`: 148 of 148, the trench cases with
the new movie check.
