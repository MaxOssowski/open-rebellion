# The Battle Summary's forces and Display pages follow the A0 captures (P24)

Date: 2026-10-24.

## Source

- `FUN_00471200`, read through REA/Ghidra on `REBEXE.EXE`, draws two labels
  on the forces page:
  - the category label at window (37, 99) in font 4 (16 px cell), white;
  - the side's rail label at (12, 40), 400 wide, centred, in font 6, red
    `0xff` or green `0xff00`.
- A0 TAC-07-C005, in window coordinates:
  - the title's capitals (font `0xb`) start at row 22, the rail label's
    at 45 and the category's at 102 (10 rows);
  - the column heads' capitals start at row 121 (10 rows), and an empty
    column's label (12 rows, font 6) at 149;
  - a unit's name starts 7 rows into its name strip.
  - Font 4 fits Liberation Sans at an em of 13.75 and the names at 11.
    Both are `port:`, measured from the capture.
- A0 TAC-07-C013: the Display page keeps the title. Its capitals start at
  row 22.
- No pure blue shows on the category bar: the category buttons
  10345-10364 key out their blue.

## Change

- The forces page texts take those fonts and rows. Each text's baseline is
  its first capital row plus 0.7 em.
- The Display page draws the title.
- The category buttons key out their blue. Before this, a blue box showed
  beside the selected Personnel button.
- `run.mjs` leaves out the title rows of the Display art, and the whole list
  width (window 37 to 384) on the forces art, since the font 6 labels are
  wider.

## Gates

- Workspace: 2749 passed, 0 failed. Render tests with the ignored
  bitmap-mask tests: 919 passed.
- Changed expectations, on the captures above:
  - `the_forces_page_draws_the_originals_layout`
  - `the_display_page_lays_out_its_buttons_like_the_original`, which also
    checks the Display page's title.
- Mutants, with both packages' tests: 28 tested, 26 caught, 2 unviable.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `3b3ea5355fa574d5cf1bb2c2b6bec1de4be521286b0e8ea2640cbb801f240527`.

## Still open

- On the Display page the second line, "Select the following button…",
  is bold in the original. The port has no bold face loaded for macroquad
  yet.
