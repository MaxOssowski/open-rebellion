# The Encyclopedia's text against A0 (interface text)

Date: 2026-10-31. Follows `../2026-10-31-status-window-text-a0/`.

## Findings

- **List rows and selector name.** The index list's rows and the
  selector's name were drawn in font 10, a stand-in for the selector's
  font 100. Font 100 is not in `FUN_0060eed0`'s table, so GDI falls back
  to the default font. A0 `encyclopedia/index-empire-day143` sets
  "Advanced Construction Yard" 162 wide: Arial at 13 ppem, which is font
  4. Font 10 gives 140. Both now use the default font.
- **Topic body.** The body had been drawn at a measured 13-pixel em with
  16-pixel lines. That is GDI's default font: 13 ppem, `tmHeight` 16. It
  now goes through `theme::paint_gdi_text_block` (`DT_WORDBREAK`) in that
  font.
- **Title and labels.** The title (font 5), "Topic" (font 4), the category
  label (font 5) and the rows (`DT_VCENTER` in 20) go through
  `theme::paint_gdi_text`.

## Check

`encyclopedia-placement.mjs --a0=.artifacts/a0-vm/encyclopedia` compares
the Empire's index and its Abduction topic with A0. Both windows are at
(125, 52). Each box lies within 1 pixel of A0's ink
(`encyclopedia-placement-result.json`):
- **Index:** the title, "Topic", "All Databases", Ackbar, Adar Tallon,
  Advanced Construction Yard.
- **Topic:** the title and the body's first three lines. The body breaks
  at the same words as A0's.

On the old code (`encyclopedia.rs` from HEAD, rebuilt) the index title
failed: 260 to 412 against A0's 266 to 404.

## Gates

All pass on Linux Chromium:
- encyclopedia-placement with `--a0`, both sides;
- status-window with `--a0`;
- message-display.

The tooltip run and `run.mjs --all` are listed in `gates.txt`.
`encyclopedia-index-shell` keeps the 51–62 and 107–398 pixel differences
recorded before this work.
