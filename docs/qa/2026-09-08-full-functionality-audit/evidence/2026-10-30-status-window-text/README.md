# The Status window's rows and title sit as A0 shows (P09)

Date: 2026-10-30. Tracker item "Panel i menu floty, rodzina okien statusu":
the Status family compared with the original. This uses the A0 captures
of `../2026-10-30-status-window-buttons/` (`a0-captures.sha256` there),
window at (170, 81).

## A0

Measured on the Empire's Character Status (Veers), Trooper Regiment,
Fleet and Capital Ship Status:

- **Two-column rows** (`FUN_00449e00`, labels at x 0 and values at x 103
  of the list at (18, 47)):
  - the rows are 14 apart: the caps of the first row at y 50, of the
    second at 64;
  - the glyphs are 12-pixel cells: "Attached:" is 45 wide with caps 8
    high, and the value "Coruscant" is 49 wide;
  - font 10, as the Troop Finder draws it, sets the same word 58 wide in
    a 14-pixel cell. A0's Galaxy Overview rows ("0054", 27 wide) are font
    10 too.
  - "Possible Command Ranks" wraps after "Command".
- **Title** (font 5, format `0x21`). It shows caps from y 21 (cap height
  10, font 5's 16-pixel bold), hanging from the box's top at 18: the
  format has no `DT_VCENTER`.
- **Name** (font 5): caps from y 140, the box's top at 137.

The code sets font 10 for the rows (`FUN_00449e00`:
`FUN_00601c60(10)`). Why A0 shows smaller glyphs there on 14-pixel lines
is not traced.

## The port (`status_window.rs`)

- **The two-column rows** lay out in `original_font(8)` (12-pixel
  glyphs). Each line is 14 high (`TextFormat::line_height`), and the rows
  draw 2 lower (`row_glyph_drop`), so their caps start 3 below the row's
  top. hyp: the font, from A0. The icon rows (Objectives, Galaxy
  Overview) keep font 10.
- **The title** hangs from its box's top (`CENTER_TOP`) instead of
  centring in it.

## Comparison

Run `status-window.mjs` at `…T23-54-50-192Z` (interface WASM
`c1957d55…`, `status-window-result.json`). All ten cases passed; the
background and buttons match exactly.

Against A0, the first row's top and bottom, the title's top and the
name's top are identical in all four families:

| Family | A0 (row 1, title, name) | Port |
|---|---|---|
| Character | 50..59, 21, 140 | 50..59, 21, 140 |
| Trooper Regiment | 50..57, 21, 140 | 50..57, 21, 140 |
| Fleet | 50..57, 21, 140 | 50..57, 21, 140 |
| Capital Ship | 50..57, 21, 140 | 50..57, 21, 140 |

Before this change the port's rows were in font 10. "Leadership Rating:"
and "Possible Command Ranks" wrapped where A0 keeps them on one line.

Regression: `agent-objectives.mjs` (release WASM) passed at
`…T23-52-44-180Z`.

## Rows' content against A0

The port's rows (`status_rows.rs`) give the same labels, in the same
order and with the same value formats, as A0 shows:

- **Fighter Squadron Status** (TIE Fighter):
  - Attached, Maintenance Cost, Squadron Size "12:12", Hyperdrive
    Rating, Maximum Shield Strength "0:0", Sub-Light Engine Rating,
    Maneuverability, Detection Rating, Bombardment Value "0:0";
  - then the Weapons Rating heading over " Laser Rating" "60:60",
    " Ion Cannon" and " Torpedoes".
- **Defense Facility Status** (KDY-150): Location, Status "Active",
  Maintenance Cost, Weapons Rating, "Shield Strength" (no colon, as A0
  shows), Bombardment Defense Strength.
- **Manufacturing Status** (Training Facility): Location, Status
  "Active", Maintenance Cost, Standard Processing Rate, Bombardment Value.
- **Capital Ship Status**:
  - the first rows up to Hull Value;
  - the four Weapons Arc headings, each over Turbo Laser, " Ion Cannon"
    and Laser Cannon.

  The rows between Hull Value and the arcs (Damage Control Rating ..
  Bombardment Modifier) fall between the captured pages.
- **Character Status** (Veers, the Emperor): every row through the
  command ranks, including Force Ranking "Jedi Master".
- **Fleet Status** and **Trooper Regiment Status**: every row.

The object menu greys Status for a sector window's icon and for a planet,
as A0 shows (`object_menu.rs`, `FUN_0051d990`).

## Open

- **Not covered by the gate.** The Fighter Squadron, Defense Facility and
  Manufacturing captures are not opened by the gate. They share the
  filler's layout.
- **Not captured.** No A0 capture shows Mission, Spec Forces, Production
  Manager or System Status.
- **Width.** The title's and name's widths differ by GDI's hinting
  (e.g. "Character Status" 104 in A0, 115 in the port).

## Tests

- `cargo test --workspace`: all pass.
- New test: `the_rows_are_white_on_their_pitch_and_drop_with_the_scale`.
- Changed expectations, for A0:
  - `the_title_name_and_rows_sit_where_the_window_lays_them_out`: the
    rows' font is 8, their pitch 14 and their drop 2;
  - the same test: the title hangs from its box's top;
  - `a_scrolled_list_draws_its_rows_higher`: the 2-pixel drop.
- `cargo mutants --in-place --in-diff` on `status_window.rs`: 32 mutants,
  28 caught, 4 unviable.

## 2026-11-02: a training facility's Status against A0

`status-window.mjs` has a new case, `manufacturing-facility`, for each side. It opens the Training Facilities page of the Build Selection fixture's system (code 55), then the first cell's Status. The window's chrome is compared as in the other cases.

For the Empire, `facilityStatusText` measures the rows against A0 `status/29-manufacturing` (the Training Facility at Coruscant, window at (170, 81)): the title, Location, Status and its value "Active", Maintenance Cost, the processing rate's two lines, and Bombardment Value.

Run `status-window-2026-11-02T11-37-11-853Z-4173079` (Linux Chromium 155, test WASM `19679add…`):

- Every label's box is 1 pixel right of and below A0's, with the same bottom.
- "Active" is at (121, 64..150) in A0 and (121, 65..150) in the port.
- The Location value is not compared: A0's facility stands at Coruscant, the fixture's at its own system.

## 2026-11-02: a capital ship's first page against A0

`status-window.mjs` `capital-ship` (Empire) now measures the first page's title and fourteen labels, Class to Hull Value, against A0 `status/03-ship` (Corellian Corvette 1, window at (170, 81)).

- Every label is within 1 pixel of A0's top and bottom and within 3 of its right edge.
- Every left edge is within 3 pixels. The rows starting with "T" (Trooper Regiments) stand 3 pixels right of GDI's, Liberation Sans's side bearing: the accepted text deviation.
- The values are not compared. The fixture's ship is another class (a damaged Victory Destroyer, hull 100:1800), while A0's corvette was undamaged ("Ship Damaged: No", 500:500).

Run `status-window-2026-11-02T11-40-57-722Z-4179358` (Linux Chromium 155): pass.

## 2026-11-02: a fighter squadron's Status against A0, and indented rows

`status-window.mjs` has a new case, `fighter`. It uses the Squadron Load fixture (code 65): a squadron on the planet, opened from the Defenses window's squadron page. For the Empire it measures the title and the labels against A0 `status/21-fighter` (a TIE Fighter squadron at Coruscant).

- **Skipped rows.** A0's pointer covers rows 9 and 10, so they are skipped.
- **Indented rows.** A0 draws " Laser Rating:", " Ion Cannon:" and " Torpedoes:" one space in, under "Weapons Rating:". The port drew them flush. `theme::gdi_word_break` dropped every leading space, including a paragraph's own. It now keeps a paragraph's leading spaces and drops only the spaces that would start a broken line.
- **Result.** The port's " Laser Rating:" now starts at x 22, against A0's 21.
- **Test.** `word_break_keeps_a_paragraphs_leading_space_but_not_a_broken_lines` (`theme.rs`) fails without the change.
- **Run.** `status-window-2026-11-02T11-47-44-115Z-4188930` (test WASM `6551a474…`): pass.

## 2026-11-02: a KDY-150's Status against A0

Scenario `BatteryStatus` (91, code 92) opens the primary system's Defenses window at the galaxy view's right edge on its Planetary Batteries page. The player's KDY-150 (DEFFACSD `0x22000001`) is listed first.

`status-window.mjs` `battery` opens its Status from that cell, as A0 `status/22`, `23` did at Coruscant. For the Empire it measures the title, the seven label lines and "Active" against A0 `23-battery`. Every box is within 1 pixel.

Run `status-window-2026-11-02T12-28-35-103Z-194847` (test WASM `e5ea0f43…`): pass.

Test: `the_battery_scenario_opens_the_batteries_page_on_the_players_kdy_150`.

## 2026-11-02: a character's item menu against A0

A0 `status/14-char-menu` (Veers at Coruscant) shows a 124 by 142 menu: Move, Confirmed Move, Mission, Command, Encyclopedia, Status, Retire.

The `character` case of `status-window.mjs` now asserts the same size and the same rows in the same order on both sides. Run `status-window-2026-11-02T12-32-02-384Z-199904`: pass.

Command's state follows the character. Veers can take command (white, with the submenu arrow). The fixture's agent cannot.
