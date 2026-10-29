# The Personnel and Troop Finders' text against the original (P09)

Date: 2026-10-29. Tracker item "Panel i menu floty, rodzina okien
statusu". This follows the Fleet Finder (`../2026-10-29-fleet-finder-text/`).

## A0 (original game, Windows 10 VM, Empire, day 167)

The captures stay outside Git; their hashes are in `a0-captures.sha256`.

| Capture | Key | Shows |
|---|---|---|
| `troop-finder-empire-day167-f4.png` | F4 | the title centred and bold; "Troop Location"; "Imperial Troops" bold; grey rows 25 apart with the text at the top of its row; the first row (Coruscant) white |
| `personnel-finder-empire-day167-f5.png` | F5 | the title centred; the label "Name"; grey rows 20 apart, centred |

## The original

| Finder | List | Format | Text |
|---|---|---|---|
| Personnel (`FUN_004637a0`) | `FUN_00607ea0(.., 0x25, 0x8f, 0x15d, 0xa1, ..)` | `+0xe4 = 0x24` | at x 5 (`+0xe8 = 5`), centred on its row |
| Troop (`FUN_0046ce40`) | `FUN_00607ea0(.., 0x25, 0x90, 0x15d, 0x9f, ..)` | `+0xe4 = 0x20` | at x 5, from the row's top (no `DT_VCENTER`) |

- Both keep the constructor's colours: grey `0x787878`, the chosen row
  white.
- Both have the bar of RCDATA 10653.
- The notes give the fonts: the title 5, the name label 4, the tab label 5.
- The Personnel Finder's label is TEXTSTRA `0x1891` "Name".

## The port

- `personnel_finder.rs` and `troop_finder.rs`:
  - the title is centred in its 350 pixels, in font 5;
  - the name label is in font 4 and the tab's label in font 5;
  - the list and the name box are in font 10;
  - rows: text at x 5 (centred for the Personnel Finder, from the top for
    the Troop Finder), grey, the chosen row white (it had been yellow).
- The Personnel Finder's label reads "Name" from TEXTSTRA `0x1891`. The
  port had "Character Name".

Still open:
- The two lists' scroll bars (art 10653) and pixel scrolling.
- Why the Troop Finder's first row is white at opening in A0. It is not
  traced; the port selects nothing.

## Browser comparison

The gate `tools/interface-parity/encyclopedia-placement.mjs
--window=finders` now opens F3, F4 and F5 in turn. Run `…T12-06-40-867Z`.
The titles, labels, fonts and grey rows match A0. The rows differ because
the games differ.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `the_title_labels_and_rows_take_the_originals_fonts_and_places`
    (Troop);
  - `at_scale_two_the_row_text_keeps_its_window_place` (Troop);
  - `the_title_and_label_take_the_originals_text_fonts_and_places`
    (Personnel, "Name");
  - `rows_sit_at_x_five_centred_grey_and_the_chosen_one_white_at_any_scale`
    (Personnel, a character row and a special forces row).
- `cargo mutants --in-place --in-diff`: 22 mutants, 8 missed at first; on
  rerun 17 caught, 5 unviable.
