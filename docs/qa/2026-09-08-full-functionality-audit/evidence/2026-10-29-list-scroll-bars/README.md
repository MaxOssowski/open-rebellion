# The original scroll bars on the Status window and the Encyclopedia (P34, P35)

Date: 2026-10-29. Tracker items "Droidy-doradcy" (Objectives and Galaxy
Overview) and "Encyklopedia". The scroll bar was the difference left
against the A0 captures in both windows.

## The original

Both lists hold the "Scroll Bar" window (`FUN_0060a490`), drawn from a
STRATEGY RCDATA art record (`FUN_0060af70`).

| Window | Call | Art |
|---|---|---|
| Status | `FUN_00443130`: `FUN_0060a490(.., list, 7, 0x299a, ..)` | 10650: width 12, track 10654, arrows 10658/10660 and 10662/10664, thumb 10666/10668/10669 |
| Encyclopedia | `FUN_0045ddc0`: `FUN_0060a490(.., list, 7, 0x299d, ..)` | 10653, the Battle Results' |

- The lists scroll by pixels.
- An arrow steps 10 (the list's argument, as in the Battle Results).
- A click on the track pages by the list's height.
- The range ends 5 pixels past content that fills the list
  (`FUN_00609ae0`).
- A0 shows the thumb clipped to its height, with nothing of it below the
  bottom cap.

## The port

- **Status window** (`status_window.rs`):
  - `BAR_ART` and `list_bar`;
  - the shared `list_scroll_bar` reads the press, hold and release, and
    draws its pieces;
  - the wheel stays (port) within the bar's range.
- **Encyclopedia** (`encyclopedia.rs`):
  - the index scrolls by pixels (`original_scroll`, formerly a row
    index), and `original_scroll_hold` keeps the held press;
  - rows are drawn with the pixel offset and clipped to the list;
  - the port's two unpainted click areas gave way to the bar.
  - The scroll resets only when a selection leaves the list, or on a
    category change (`FUN_0045f100` rebuilds the list from its top). The
    previous rule reset it whenever nothing was selected, which is the
    opening state since s200. That made the list unscrollable.
- **Shared** (`list_scroll_bar.rs`): the thumb's last middle tile moves up
  to end at the thumb's bottom.
  - The middle tiles are uniform down their height, so this equals
    clipping.
  - It fixes a 4-pixel overhang the comparison showed, also in the Battle
    Results and Battle Alert bars.

## Comparison with A0

`bar-pixel-diff.txt` compares the bar columns, both Empire:

| Bar | Pixels differing |
|---|---|
| Encyclopedia, 12 by 160 | 2 of 1920 |
| Objectives, 12 by 204 | 2 of 2448 |

Before the tile fix, the Objectives bar had 42 differing pixels.

## Browser acceptance

The WASM hash is `09fe474d…`.

- `tools/interface-parity/encyclopedia-placement.mjs`
  (`…T11-27-29-572Z`) passed for both sides. It now also clicks the down
  arrow three times and checks that the list changed;
  `empire-encyclopedia-scrolled.png` shows it 30 pixels down, its top row
  half clipped.
- `tools/interface-parity/agent-objectives.mjs` (`…T11-26-55-434Z`)
  passed for both sides.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `the_list_bar_sits_at_the_lists_right_and_shows_only_when_rows_overflow`;
  - `a_click_on_the_bars_down_arrow_scrolls_the_list_ten_pixels`;
  - `a_held_arrow_draws_pressed_only_while_the_pointer_stays_on_it`;
  - `at_scale_two_the_bar_reads_the_pointer_in_window_pixels`;
  - `the_thumbs_middle_never_reaches_past_its_bottom`;
  - `the_index_list_scrolls_by_pixels_and_clips_its_rows`;
  - `the_index_bar_sits_right_of_the_list_and_its_down_arrow_steps_ten`;
  - `at_scale_two_the_down_arrow_still_steps_ten_and_a_wheel_notch_a_row`.
- Changed expectations, from rows to pixels:
  - `original_catalog_reveal_and_clamp_keep_selection_visible`: 80 and
    85, where it had 4 rows;
  - `eight_twenty_pixel_rows_fill_the_native_list`: the row rectangle
    takes a top;
  - `the_wheel_scrolls_the_list_only_as_far_as_its_rows_reach`: 51, the
    range's 5 extra pixels (`FUN_00609ae0`).
- `cargo mutants --in-place --in-diff`:
  - first run: 100 mutants, 45 missed, then 18 after the new tests;
  - the remaining 8 are equivalent:
    - the default point `(-1, -1)` for a missing pointer: `(1, 1)` is
      never on the bar either (4);
    - the reveal's `<` / `>` at their boundary, where both give the same
      scroll (2);
    - the wheel's `> EPSILON` (1);
    - the input phase's range in the Status window, which `scrolled`
      re-clamps with the right one (1).
