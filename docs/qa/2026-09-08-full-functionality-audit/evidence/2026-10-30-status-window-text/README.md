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
