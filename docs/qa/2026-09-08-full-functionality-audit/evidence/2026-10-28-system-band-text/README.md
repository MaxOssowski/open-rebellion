# System window bands: black titles, font-10 lines, default-font counts, a yellow bar (P06, P11)

Date: 2026-10-28. The reading is in
`ghidra/notes/manufacturing-build-selection.md`, "Band and count text".

## The original

- `FUN_00458080` writes the band title in font 10. It keeps the text
  object's default black (`0x2000000`, `FUN_00601930`). The status, units
  and destination lines are font 10, white.
- `FUN_00457690`'s yard counts are white and centred, in the default font.
  The A0 digits stand 10 rows tall, which fits a 16-row cell.
- The progress bar is `0x200f0f0`. In COLORREF order that is yellow.

A0, Windows 10 VM (`.artifacts/a0-vm/2026-10-28-nav/system-vjun.png`,
SHA-256 `2602c287241b2d38e319270e3d00d85816606d1e7ae81da99a0cf96883f09ecd`):
the Vjun window shows "Ship Construction" black on the red strip, and its
lines and the "0:0" count white.

## The port

Before: every band line and count was drawn white in a 9-pixel
proportional font. The progress bar was light gray, from a misread of
`0x200f0f0`.

Now (`crates/rebellion-render/src/system_window.rs`):
- the title is black and the other lines white, all in
  `theme::original_font(10)`;
- the counts use `COUNT_FONT` (4, the 16-row default);
- the bar is `PROGRESS_COLOR`, yellow.

`manufacturing_window.rs`'s comments are corrected.

## Tests

- `the_window_paints_its_chrome_tabs_and_the_overview_first`: the title
  is black, the status line white, and the count's font is the default.
  The heading's size changes from the port's 18 to font 10's size for this
  recovered rule.
- `a_building_band_shows_its_product_and_units`: the bar's black rect, and
  the yellow fill from its left.

## Browser

In `build-selection.mjs` (Arch Chromium 155, run
`build-selection-2026-10-28T16-51-49-684Z-3399135`), the overview matches
the A0 band, apart from the font's antialiasing. The script's known
`opened` failure (6 of 28,646 pixels, present at HEAD before these
stages) is unchanged.

## Gates

- Workspace: 3034 tests passed. Provenance scan: no uncited items.
- Mutants (`rebellion-render`, the diff):
  - the progress bar's `width * progress` survived as `/` and `+` while
    the test checked only the fill's left edge;
  - the test now also requires the fill to be narrower than the bar. At
    progress 0, `/` gives infinity and `+` the full width;
  - final run: 3 mutants, 3 caught.
- Browser:
  - `sector-quadrants.mjs`: 12 of 12 (run
    `sector-quadrants-2026-10-28T16-54-02-076Z-3413947`);
  - `fleet-window.mjs` `chrome`: 2 of 2;
  - GID: 152 of 152 (run `2026-10-28T16-54-44-924Z-3416803`).
- rustfmt: `system_window.rs` is clean.
- Release WASM SHA-256 `98a485aa58859018ca22b171ba023817e489d848218903278a4e21f2142d6a66`.

**Correction (2026-10-28, s192).** The default font is a 14-row cell, not
16: the GID caption, also in the default font, measures 91 pixels for
"Popular Support" in A0, which is Arial at a 14-row cell. `COUNT_FONT` is
now `DEFAULT_FONT`, with font 10's cell. See `../2026-10-28-gid-caption/`.
