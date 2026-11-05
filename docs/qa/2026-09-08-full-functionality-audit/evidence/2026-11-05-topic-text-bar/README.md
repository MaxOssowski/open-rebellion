# The Encyclopedia topic body's scroll bar, against A0 (2026-11-05)

This covers OBJ-01-C008 ("topic text"), in part.

## A0

`encyclopedia/topic-step-5` and `topic-step-6` show the A-wing's topic for the Alliance, with the window at (62, 52). Its body, a list of statistics, outgrows the 80-pixel text control and shows the control's bar at its right edge. The up arrow is at the window's (399, 231) and the down arrow at (399, 302), each 13 by 9, with a thumb between them.

## The source

The topic body is the same text object as the Message Index's: `FUN_0041ecf0` with `FUN_006002b0(4)`, and its bar from `FUN_0041efe0`: `FUN_0060a490(.., 7, 0x299a, ..)`, the Status window's art RCDATA 10650.

## Before

The port painted the body clipped at 80 pixels, with no bar.

## Now

- **The bar.** `draw_encyclopedia_topic` uses `message_index::text_wrap` (b77). The body wraps at 395 while it fits, otherwise 12 pixels short beside `list_scroll_bar::ui_bar` with `status_window::BAR_ART`, stepping a 16-pixel line. The scroll resets for each topic.
- **`personnel-states.mjs`** (Linux, the Alliance). On A-wing's topic, the bar's up and down arrows are 0 differing from A0 `topic-step-6`. The thumb is not compared, since it follows the line count, which depends on the text metrics.
- **The b81 batch.** It ran all 21 Linux gates, and every one passed. Its in-diff run left 12 mutants in the topic body's paint arithmetic (scale products and the per-topic reset). They are the same pattern b78 closed for the Message Index with `on_screen` and a reset method, and are left for the next stage.

## Still open

The Abduction body's third line breaks one word earlier than A0's (Liberation Sans against GDI Arial).
