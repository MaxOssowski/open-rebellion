# Window titles in font 5 (P06, P09)

Date: 2026-10-28. The reading is in `ghidra/notes/fleet-window.md`, "Title
fonts".

## The original

- The Fleet, System, System Defenses and Missions windows write their
  titles in font 5 (16 rows, bold), black, format `0x24`: one line,
  left-aligned and vertically centred, in the label from y 2, 16 high
  (`FUN_004a4b10`, `FUN_00452fc0`, `FUN_004a8790`, `FUN_004a2200`).
- The Missions window's target name and "Target:" labels are font 5 too.
- A0 (`.artifacts/a0-vm/2026-10-28-nav/system-vjun.png`): "Vjun" is bold
  black, with a capital about 11 rows tall.

## The port

Every one of these labels was drawn in an 11-pixel proportional font,
although the port's own comments named font 5. They now use
`theme::original_font(5)`, the bold 16-row cell. The Fleet window's title,
drawn from y 3 top-aligned, is now centred in rows 2..18 like the others.

The tests now load the original fonts (`theme::load_fonts`): font 5 is the
bold family, which the default egui context lacks.

## Tests

The four windows' chrome tests now expect font 5's size for the title. The
port's 22 (11 x 2) had no source. The Fleet window's test checks the
title's left edge, size and colour.

## Gates

- Workspace: 3034 tests passed. Provenance scan: no uncited items.
- Mutants (`rebellion-render`, the diff):
  - the Fleet window title's `2.0 + 8.0` survived as `-` and `*` until
    the test checked the title's vertical centre (the test text now
    records its height);
  - final run: 6 mutants, 6 caught.
- Browser:
  - `fleet-window.mjs`: 60 of 60, all cases (run
    `fleet-window-2026-10-28T17-18-34-618Z-3513547`);
  - `sector-quadrants.mjs`: 12 of 12 (run
    `sector-quadrants-2026-10-28T17-17-58-247Z-3510645`);
  - GID: 152 of 152 (run `2026-10-28T17-22-34-396Z-3526289`).
- rustfmt: only the files' baseline differences remain.
- Release WASM SHA-256 `af8e0a6db39e7bc4d3520236e1547ba6f37fab0933e46a4fd9bb70a8dca823e3`.
