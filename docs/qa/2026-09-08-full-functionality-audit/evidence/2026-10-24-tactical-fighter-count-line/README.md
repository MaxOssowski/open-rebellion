# The fighter panel's count line (P24)

Date: 2026-10-24.

## Source

`ghidra/notes/tactical-panel-text.md` (code at `0x5e1c80`-`0x5e1f37`,
`FUN_005c6530`, `FUN_005c6570`, `FUN_005ba330`, `FUN_005ba380`):

- The original tries the group's fighter types `0x1d..0x24` in order. The
  first one present picks TEXTTACT `56773 + k` ("X-wings:" ... "Defenders:")
  and portrait 2030 + k.
- It writes, at panel (0x17, 0x92) in font 8, the label, the number of
  squadrons of that type, and the group's craft as
  `sum trunc(hull / 8)` "/" `sum trunc(maximum hull / 8)`, where each
  squadron counts at least 1.
- A0 TAC-02-C002 reads "X-wings: 1 12/12", with capitals from row 176 at
  x 505.

## Change

- `tactical_fighter_count` builds the line from the selected group's live
  squadrons. The type index is the tactical ordinal minus 29, the same
  pairing as the portrait.
- The panel draws the line at (505, 176).
- `run.mjs` leaves the line's 12 rows out of the fighter panel and portrait
  probes, since it lies over the portrait's foot as in the original.

## Gates

- Workspace: 2742 passed, 0 failed. Render tests with the ignored
  bitmap-mask tests: 912 passed.
- New tests:
  - `a_full_x_wing_squadron_reads_one_twelve_of_twelve`
  - `the_count_line_names_the_first_type_and_sums_every_squadron`
  - `a_squadron_outside_the_eight_types_gives_no_count_line`
  - `the_count_line_counts_only_the_players_selected_group`
- Mutants, with both packages' tests: 16 tested, 13 caught, 2 unviable,
  1 missed. The missed one is `draw_original_tactical_hud` -> `()`, covered
  by the browser probes.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144. This ran before the
  last two tests were added; those change only test code.
- Replay equivalence passed.
- Release WASM `7e736d4010cc324552ec8639602fb2801191a03a9bd9356c145e9db161d5bc98`.
