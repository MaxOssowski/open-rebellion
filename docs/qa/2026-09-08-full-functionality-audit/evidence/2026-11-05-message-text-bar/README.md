# The shown message's text scroll bar against A0 (2026-11-05)

This covers CMD-08-C009 ("event"): the remaining gap in one message shown.

## A0

`agent-advice/02-advice-message`, the Empire's advice "Planetary Assaults". The long text wraps short of a 12-wide bar at the text control's right edge. The control is 395 by 80 at the window's (17, 234), so the bar spans x 525..537 of the Empire's window at 125. It has the up arrow at (525, 286), the down arrow at (525, 357) and a thumb between them.

## The source

- **`FUN_004665f0`.** It creates the text control with `FUN_0041ecf0(.., 0x11, 0xea, 0x18b, 0x50, ..)`.
- **`FUN_0041ecf0`.** It sets the bar art id `0x299a` at `+0x136` and the font `FUN_006002b0(4)`.
- **`FUN_0041efe0`.** On create it makes the bar, `FUN_0060a490(.., 7, 0x299a, ..)`, with the Status window's art, RCDATA 10650.

## Now

- **The bar.** The port's text control wraps at its full width while the text fits its 80-pixel height. Otherwise it wraps 12 pixels short and shows the bar (`list_scroll_bar::ui_bar`, `status_window::BAR_ART`), stepping a line, 16 pixels. hyp: whether the original wraps short of the bar only when it shows.
- **The text.** It scrolls by the bar's first pixel, and the scroll resets when another message is shown.
- **Unit test.** `a_long_text_wraps_beside_the_bar_and_a_short_one_does_not` fails without the change. `text_wrap` mutants: 25 caught.
- **`agent-advice.mjs`** (Linux, the Empire; its newest advice is long). The bar's up and down arrows, 13 by 9 at (525, 286) and (525, 357), are 0 differing from A0. The thumb is not compared, because the advice texts differ.
- **The b77 batch.** It ran all 21 Linux gates, and every one passed. Its in-diff run left 15 mutants in `draw_single_message`'s paint arithmetic (scale products, which equal their alternatives at the gates' scale 1, and the scroll reset); the gates' pixel checks cover the drawing.
