# The Encyclopedia's and the Assault Summary's frames lie over their panes

Date: 2026-11-09. Follows `2026-11-09-message-index-frame-over-panes`,
which found the same order in the Message Index.

## The defect

Both windows use the Message Index's base, STRATEGY 10335 (Alliance) or
10336 (Empire), keyed on its blue (palette index 252). Its top corners
have white arcs that reach into the pane at (12, 13) or (12, 14). The
port painted the base first and the pane or page art over it, so the
arcs were covered.

## A0

For every base pixel that is not blue and lies inside the pane's
rectangle, A0 shows the base's colour:

| Window | A0 capture | Origin | Base's colour |
| --- | --- | --- | --- |
| Encyclopedia index (10338) | `encyclopedia/category-0-alliance-day185` | (62, 50) | 498 of 498 (2 more agree with both) |
| Encyclopedia index (10338) | `encyclopedia/index-empire-day143` | (125, 52) | 86 of 86 (1 more) |
| Encyclopedia topic (10337) | `encyclopedia/ex1-adar-tallon`, `topic-step-2` | (62, 50) | 499 of 499 (1 more) |
| Encyclopedia topic (10337) | `encyclopedia/topic-abduction-empire-day166` | (125, 52) | 474 of 474 |
| Assault Summary, Summary page | `alliance-game2/assault-on-yavin-summary` | (62, 49) | 442 of 442 |
| Assault Summary, every page | `messages-day633/assault-coruscant*` (8 captures) | (62, 49) | 500 of 500 each |

## The change

- `encyclopedia.rs`: `window_layers` gives the pane, then the base, then
  the rail, for the index and the topic. The unit test
  `the_frames_corner_arcs_lie_over_the_pane` checks the order for both
  sides and both panes.
- `assault_summary.rs`: `page_art` picks the page's art (the picture, the
  forces columns or the Display page), painted before the frame. The unit
  test `the_summary_shows_the_title_and_the_outcome_in_the_attackers_colour`
  now also checks that the picture is painted before the frame.

Both tests fail without the change.

## Browser

- `encyclopedia-index-shell.mjs` composed its expected shell in the
  port's old order. It now lays the pane, then the base keyed on blue,
  then the rail, and compares the window's top corners (x 0..29 and
  384..411, y 0..19) with A0 `category-0-alliance-day185` and
  `index-empire-day143`.
- `fleet-window.mjs` compares the Alliance's Assault Summary's top
  corners (x 0..29 and 384..413, y 0..21) with A0 `assault-coruscant`.
  The Empire's fixture is compared with A0 `assault-clean`, which shows
  the Alliance's frame, so its corners are not compared.

## Batch b150

23 of 23 interface gates on Linux; workspace tests 3494 passed, fixture
tests 139. Tactical matrix `2026-11-09T05-16-01-969Z-1573829`: 148 of 148.
Measured: the Encyclopedia's corners 0 pixels from both A0 captures; the
Alliance's Assault Summary corners 0 pixels from A0.

Mutants: 189 of 190 caught. The survivor replaces `paint_window_layers`
with nothing. Unit tests run without the DLL, so no bitmap is painted
there either way; `encyclopedia-index-shell.mjs` compares the whole
window with the DLL's bitmaps and fails when nothing is painted.
