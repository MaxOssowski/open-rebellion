# The Display page's second line is bold (P24)

Date: 2026-10-24.

## Source

`FUN_0046f360`, disassembled at `0x46f9f0`-`0x46fac0` in `REBEXE.EXE`:

- After the first label at (24, 40), the code calls `FUN_00601c60(7)`, the
  table's font 7 (18 px cell, weight 700). It then places the second label
  at (24, `0x82`).
- A0 TAC-07-C013 shows the second line, "Select the following button…",
  in bold.

## Change

- `PageOp::Wrapped` carries `bold`. The Display page sets it on its second
  line.
- The wrapped and plain window texts draw and measure with Liberation Sans
  Bold when `bold` is set. The bold face is already in the repository
  (`theme::ARIAL_BOLD`) and is prewarmed in the font atlas.

## Gates

- Workspace: 2749 passed, 0 failed. Render tests with the ignored
  bitmap-mask tests: 919 passed.
- `the_display_page_lays_out_its_buttons_like_the_original` now also checks
  that only the second line is bold.
- Mutants, with both packages' tests: 31 tested, 3 caught, 4 unviable,
  24 missed. All the missed mutants are in font loading, text measuring
  and drawing code that needs a GPU context. The browser suite's Display
  page frames cover them.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `98339498558fb673c1195bad38643be77031fa371c5661fadb20939188a82520`.
