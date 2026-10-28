# The GID caption: default font, side colour, top-aligned (P06, CMD-02)

Date: 2026-10-28.

## The original

`FUN_00425d00` writes the active display's name above the map:
- a text object in the default font (`FUN_00601880` -> `FUN_00603850`);
- format 1 (`DT_CENTER`, top-aligned), from the galaxy view's top left
  (`+0xcc`, `+0xd0`) across its width (`+0xd4 - +0xcc`);
- colour `0x20000ff` (red) for side 1 and `0x200ff00` (green) otherwise.

There is no bold pass.

A0, Windows 10 VM, Alliance, Popular Support (`galaxy-a.png`, hash in
`../2026-10-28-reference-rail/`):
- the caption is pure (255, 0, 0) and regular, with one-pixel strokes;
- it spans x 252..342 and y 43..55 on the canvas: 91 pixels wide.

Liberation Sans (Arial's metrics) gives 91 pixels at a 12.5-pixel em, a
14-row cell, and 103 at the 16-row cell. So the default font is the 14-row
regular cell, the size of font 10.

## The port

Before: an uncited 15-pixel proportional face, drawn twice with a 0.6-pixel
offset for weight, in (199, 41, 41) or (41, 166, 51), centred 8.5 rows below
the aperture top. That came from the resampled web frames (squakenet 0119,
0712).

Now (`crates/rebellion-render/src/lib.rs`, `draw_gid_caption`):
- one pass of `theme::original_font(DEFAULT_FONT)`, with `DEFAULT_FONT` =
  10 for the 14-row cell;
- `game_menu::faction_text_color`, which is `0x20000ff` / `0x200ff00`;
- hung from the aperture's top centre (`gid_caption_anchor`).

The System window's yard counts, also in the default font, now use
`DEFAULT_FONT`. This corrects `../2026-10-28-system-band-text/`, which took
their digits for a 16-row cell.

## Tests

`the_gid_caption_hangs_from_the_aperture_top_centre` replaces
`the_gid_caption_centres_where_the_original_frames_draw_it`. The old test
pinned a centre measured on resampled web frames. The code and the A0
capture place the caption's box at the aperture top.

The new test `the_gid_caption_writes_the_display_name_once_in_the_side_colour`
draws the caption in egui. It checks one text in green for the Empire, in
the default font, hung from the anchor, and none with the display off.

## Browser

GID `run.mjs --all`: 152 of 152 (run `2026-10-28T18-31-34-150Z-3768598`).
Its Alliance "Active Personnel" capture against the A0 "Popular Support"
crop:
- same height, colour and top;
- the port's antialiasing is softer.

## Gates

- Workspace: 3035 tests passed. Provenance scan: no uncited items.
- Mutants (`rebellion-render`, the diff):
  - `draw_gid_caption` emptied survived until the drawing test was
    added;
  - final run: 6 mutants, 6 caught.
- `sector-quadrants.mjs` 12 of 12; `fleet-window.mjs` `chrome` 2 of 2.
- rustfmt: the changed blocks of `lib.rs` are formatted on their own
  (the whole file pulls in its modules); `system_window.rs` is clean.
- Release WASM SHA-256 `9949a60c6762b9595eb87d98f72c2283ac6864afb2653f772ca63e1747a96a06`.
