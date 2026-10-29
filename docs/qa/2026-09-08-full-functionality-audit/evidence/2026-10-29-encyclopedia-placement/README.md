# The Encyclopedia's placement and index list against the original (P35)

Date: 2026-10-29. Tracker item "Encyklopedia". The two things left were:
- the window's position, which was a hyp;
- a comparison with the original.

The reading is in `ghidra/notes/encyclopedia-window.md`, "Production
routing" and "The index list".

## The original

- **Placement.** `FUN_0045ddc0` places the window with `FUN_00606980` in
  the galaxy view's rectangle: centred, with truncating halves.
  - Empire: (125, 52), from the A0 capture (exact over the frame) and
    old-games-07.
  - Alliance: (62, 50), from kritikanstvo 281117 and 281123.
- **The list.** `FUN_00607ea0` sets it up:
  - cells 350 by 20, so eight rows;
  - text at x 5, centred on the row, in font 10;
  - grey `0x787878`, and white for the selected row with no other mark.
- **Labels.** Title and category label in font 5, "Topic" in font 4.
- **Opening.** Each opening builds a new window. Its list starts at All
  Databases with nothing selected (`FUN_0045f100(0x6f, 1)`).
- **Systems.** The objects come from `FUN_00422620`. Its systems are those
  of the viewer's side, so they are the game's galaxy.

## A0 (original game, Windows 10 VM, Empire)

The captures stay outside Git; their hashes are in `a0-captures.sha256`.

| Capture | Shows |
|---|---|
| `index-empire-day143.png` | F7: the index at (125, 52), grey rows 20 apart, no selection, no Abregado or Adrift |
| `index-empire-day145-abduction.png` | after a click on Abduction: its row white, the selector "Abduction" |

## The port

- `encyclopedia.rs`:
  - `encyclopedia_origin`, through `galaxy_centered_rect`;
  - eight 20-pixel rows, text at x 5 centred, grey or white, no bar;
  - fonts 5 / 4 / 10;
  - no automatic first-row selection;
  - `open_original_encyclopedia_at` resets to All Databases;
  - `OriginalEncyclopediaCatalog::set_galaxy_systems`.
- `main.rs`:
  - the window's origin;
  - F7 opens a fresh index;
  - while the window is open, the catalog's systems are the world's.
- Cleanup: the unused import `Mark` in `advisor_schedule.rs`, from s197.

## Browser acceptance (`tools/interface-parity/encyclopedia-placement.mjs`)

Run `encyclopedia-placement-2026-10-29T10-42-07-023Z` passed for both
sides. The WASM hash is `da9d3b02…`. For each side the test starts a new
game, skips the briefing and presses F7. The original window opened
(`status=opened_original`). No console errors appeared, and every request
returned below 400.

Measured on the screenshots by matching the base bitmap (mean difference
0.0 over the frame's top and bottom strips):
- the Empire's window at (125, 52);
- the Alliance's at (62, 50).

Compared with A0:
- The title, Topic, the category icons, All Databases and the grey rows
  20 apart match.
- No row is selected and the selector is empty.
- Row 5 reads Adega in A0 and Adrift in the port: the two games' galaxies
  differ. Abregado is absent from both.
- port: no scroll bar.

The fixture gate `encyclopedia-index-shell.mjs` (the frame's bitmaps and
controls at a fixed fixture origin) was not rerun. This stage changes
neither its bitmaps nor its controls.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `the_window_sits_centred_in_each_sides_galaxy_view`;
  - `opening_starts_at_all_databases_with_nothing_selected`;
  - `the_index_lists_only_the_games_systems_once_they_are_set`;
  - `index_rows_are_grey_but_the_selected_white_centred_in_twenty_pixel_rows`.
- Changed expectations. The trace and A0 both contradict the old
  guesses:
  - `original_catalog_selection_preserves_identity_or_clears`, formerly
    `..._resets_to_first_match`: a rebuilt list selects nothing
    (`FUN_0045f100`);
  - `original_catalog_reveal_and_clamp_keep_selection_visible`: scroll 4,
    not 3, with eight visible rows;
  - `eight_twenty_pixel_rows_fill_the_native_list` replaces
    `original_index_ninth_row_is_clipped_...`: 18-pixel rows were a
    guess, and `FUN_00607ea0` gives 20.
- `cargo mutants --in-place --in-diff` on `encyclopedia.rs`:
  - first run: 16 mutants, 8 missed in the row drawing;
  - after the drawing test: 16 mutants, 15 caught, 1 unviable.
