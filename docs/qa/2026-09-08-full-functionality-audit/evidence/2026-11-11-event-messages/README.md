# Event messages and Display to the Encyclopedia against A0

Date: 2026-11-11. Adds to CMD-08-C014. Records the A0 captures still needed
for EVT-01-C002, C005, C015, C018 and C019, and for CMD-08-C004.

## A0 search

All 129 capture sets under `.artifacts/a0-vm/` were searched by name and
through the evidence READMEs. None shows any of these messages:

- a Natural Disaster message (EVT-01-C002);
- Uprising Begins or Uprising Ends (EVT-01-C005; only Near Uprising is in
  `uprising-clean/`, and it is already compared);
- a Jedi Training message (EVT-01-C015);
- Jabba's palace, the bounty hunters' capture, or Rescue Attempt
  (EVT-01-C018; only `health-2026-11-09/04-han-bounty-hunters-day372.png`
  exists, and it is already compared);
- a step with the Message Summary's arrows (CMD-08-C004).

`2026-11-11-captured-member/00-chewbacca-incite-uprising-coruscant-day60.png`
is the mission dialog for Incite Uprising, not a message.

So no new event message was compared today. The one A0 capture with
unused detail was `research-complete/02-display-opens-encyclopedia.png`
(SHA-256 `239f8d2da52b00ba23589d314a6338f1eb81ab32aa6f80a14a8b7ecec616ac08`).
It shows where Display on Research Complete leads: the Nebulon-B
Frigate's Encyclopedia topic.

## Two defects found in that topic

1. Tab stops. The body text has tabs between each label and its value
   (`Refined Material Cost:\t34`, `Shield Strength:\t\t\t300`). The port
   could not measure a tab with GDI's advances. So it drew the whole line
   with egui's layout, and the values fell at x 226, 220, 210, 218 and 218.
   A0 sets all five values at x 223, the body's left plus 144. The labels
   end at different places and use one to three tabs. Of the stop widths
   only 24 pixels fits all five lines. The original draws the body with
   `DrawTextA` (`FUN_0041fd00`, format 0x2410). The width is measured, not
   traced, and is tagged `port:` in `theme::GDI_TAB_STOP`.
   Test: `a_tab_moves_the_pen_to_the_next_twenty_four_pixel_stop` (fails
   without the fix: no pen positions for a line with a tab).
2. Arrow keying. The topic's previous and next arrows (0x83, 0x84) are
   strobe buttons (`FUN_0045ddc0` calls `FUN_00602150` with 0x2891/0x2892
   and 0x288e/0x288f). Such buttons are blitted keyed on their first pixel.
   The port drew a blue box around each arrow. A0 shows the title bar's
   grey there. STRATEGY 10382, 10383, 10385 and 10386 are now keyed.
   The unavailable art (10384, 10387) has no A0 capture and stays unkeyed.
   Test: `the_encyclopedia_topics_arrows_are_keyed_on_their_first_pixel`
   (fails without the fix).

## Browser gate

`message-display.mjs` now compares the topic after Display on Research
Complete (Alliance) with A0 `research-complete/02`:

- the 400 x 190 picture area: 0 pixels differ;
- the previous and next arrows: 0 pixels differ;
- the first lit column of each body value: 224 on all five lines. A0 has
  223 on all five. The pen is at the same stop; egui rasters these digits
  one column right of GDI, while the labels' first column (x 80) agrees.
  Before the fix the values fell at 226, 220, 210, 218 and 218.

Font 4's glyph shapes are not compared; GDI and egui raster them apart.
The title's text pixels differ for the same reason.

Runs (Linux, `/usr/bin/chromium`, `--no-build`, the worktree's fixture
WASM):

- `message-display-2026-11-11T18-38-41-174Z`: before the arrow and wait
  changes were built; failed on the new check (no picture yet, values 224).
- `message-display-2026-11-11T18-45-27-021Z`: failed; the worktree's runtime
  pack had no EDATA (the build needs `REBELLION_EDATA_DIR` set to the
  original install's `EData`).
- `message-display-2026-11-11T18-52-52-907Z`: pass, both sides. Result:
  `picture_a0` 0, `previous_arrow_a0` 0, `next_arrow_a0` 0,
  `value_left` [224 x 5], `value_left_a0` [223 x 5].

The run directories are under the worktree's
`.artifacts/interface-parity/`.

Unit tests: `cargo test -p rebellion-render` 1464 passed.
Mutants: `theme.rs` (`gdi_pen_positions|paint_gdi_text$`) 63 of 63 caught;
`bmp_cache.rs` (`uses_first_pixel_key`) 6 of 6 caught.
`provenance-scan check`: ok.

## A0 captures still needed

- EVT-01-C002: a Natural Disaster message opened in the Message Index
  (its picture, title and text), in any game. A system with a facility and
  some time may be needed.
- EVT-01-C005: an Uprising Begins message and an Uprising Ends message,
  each opened. Umgul in the unedited Alliance game was near uprising at
  day 795; running on from there may give both.
- EVT-01-C015: the Jedi Training messages (Luke with Yoda, or Vader
  training a character), each opened.
- EVT-01-C018: Han Solo captured by bounty hunters and taken to Jabba's
  palace, and a Rescue Attempt message, each opened.
- EVT-01-C019: the bombardment and the captures on the second side
  (the note lists them as compared on one side only).
- CMD-08-C004: two captures of one open message, before and after one
  press of an arrow (0x96 or 0x9a), to show which arrow goes to the older
  message.

## Batch b192

- 29 of 29 interface gates on Linux, among them `message-display`
  (`message-display-2026-11-11T19-47-15-648Z`) and `briefing`.
- Workspace tests: 3586 passed, 0 failed. Fixture tests: 156. Provenance
  is ok.
- Tactical matrix `2026-11-11T20-06-11-086Z-2039305`: 152 of 152.
