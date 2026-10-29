# The Troop Finder opens on its first row (P09)

Date: 2026-10-29. Tracker item "Panel i menu floty, rodzina okien
statusu". This closes the open point "the Troop Finder's first row white
at opening in A0 is untraced" left by
`../2026-10-29-personnel-troop-finder-text/`.

## A0 (original game, Windows 10 VM, Empire, days 170..173)

The captures stay outside Git; their hashes are in `a0-captures.sha256`.

| Capture | What it shows |
|---|---|
| `f4-open-day170.png` | F4 with the cursor away from the list: Coruscant, the first row, white (255, 255, 255); the other rows grey (117, 114, 114); the name box empty |
| `fleet2-clicked.png` | a click on Fleet 2: Fleet 2 white, Coruscant grey, "Fleet 2" in the name box |
| `f4-reopened-day171.png` | Escape, then F4 again: Coruscant white again and the name box empty, so the choice does not carry over |
| `alliance-tab.png` | the Alliance tab: its only row, Yavin, white |
| `button-open-day173.png` | opened from the cockpit button, not F4: Coruscant white, so no key is involved |

The Fleet and Personnel Finders' A0 captures
(`../2026-10-29-fleet-finder-text/`, `../2026-10-29-personnel-troop-finder-text/`)
open with every row grey.

## The original

- `FUN_0046ce40` ends with `FUN_0060d7e0(side, 1)`. The 1 sends the tab's
  `WM_COMMAND` (case `100` of `FUN_0046df90`).
- Case `100` rebuilds the list (`FUN_0046ea10`), then calls
  `FUN_00609650(list, name box +0x98, 0, 0)` at `0x46e22f`.
- `FUN_00609650` (disassembly at `0x609650`):
  - The best row starts as the first (`0x609676`: `edi`, the first row, is
    stored as the best).
  - The guard `FUN_005f3040` measures the typed text: its length plus one
    (`FUN_00479ac0` returns 1), 0 only without a buffer.
  - A row whose first letter matches counts the letters it shares,
    through the terminator.
  - A longer count takes over; an equal one keeps the earlier row; a
    shorter one ends the loop.
  - The best row is chosen (`FUN_00609500`, bit 1 at `+0x3c`) and
    scrolled to (`FUN_00609ed0`).
  - So an empty name box, or text no row begins, chooses the first row.
- The Fleet Finder's case `100` (`FUN_00462770`) does the same. But
  `FUN_00461960` fills its list directly (`FUN_00462be0(1, 1)`) and selects
  its tab with `FUN_0060d7e0(1, 0)`, without the notification. So it opens
  with nothing chosen, as A0 shows.
- The Personnel Finder's case `100` (`FUN_00464890`) does not match a
  prefix.

## The port

- `list_prefix::best_prefix_row` is `FUN_00609650`'s rule, shared by the
  three Finders' `prefix_choice`. A text no row begins now chooses the
  first row, where the port had chosen none.
- hyp: the name box always holds a buffer, so the length guard never
  stops the match.
- The Troop Finder chooses the matching row once its list is built, on
  opening and on a new tab. The Fleet Finder does so on a new tab only.

## Browser

The gate `tools/interface-parity/encyclopedia-placement.mjs
--window=finders` passed for both sides at `…T14-21-32-128Z`. The WASM
hash is `49dd43b6…`. `empire/troop-finder.png` (`add7748a…`) shows the first
row, Corfai, white under an empty name box, as A0 does.
`alliance/troop-finder.png` is `5dfb20f4…`.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `list_prefix`: `an_empty_name_or_one_no_row_begins_chooses_the_first_row`,
    `the_longest_shared_beginning_wins_and_ties_keep_the_earlier_row`,
    `a_whole_name_beats_a_longer_one_and_a_shorter_match_ends_the_scan`;
  - `the_opening_and_a_new_tab_choose_the_row_the_name_begins_or_the_first`
    (Troop);
  - `the_opening_chooses_nothing_and_a_new_tab_chooses_the_rows_first`
    (Fleet).
- Changed expectations, for the trace and the A0 captures above:
  - `a_name_no_row_begins_with_chooses_nothing` (Fleet), now
    `..._chooses_the_first_row`;
  - the `"z"` and `""` cases of `a_typed_name_chooses_the_first_row_it_best_begins`
    (Personnel, Troop), now the first row;
  - `the_title_labels_and_rows_take_the_originals_fonts_and_places`
    (Troop): the opened Finder's first row is white.
- Without the change, the two new Finder tests and the Troop layout test
  fail.
- `cargo mutants --in-place --in-diff`: 38 mutants, 1 missed at first, a
  tie that broke the scan. A test now kills it.
