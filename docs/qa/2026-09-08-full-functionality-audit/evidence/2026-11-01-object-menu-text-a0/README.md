# The object menu's text against A0 (2026-11-01)

## Problem

The Game Menu Window (`game_menu.rs`: object menus, the speed menu, the
Game Menu) still drew its text as egui's proportional font at a 14-pixel
em. The GDI text migration of 2026-10-31 covered the GID menu in
`cockpit.rs`, not this window. Against A0 `status/11-troop-menu.png` the
port's "Confirmed Move" was 103 pixels of ink where A0 has 90, and
"Encyclopedia" 84 where A0 has 76, so the whole menu was wider.

## The original

GDI's advances (`gdi_metrics.rs`, from Windows 10's arial.ttf) give
"Confirmed Move" 92 pixels and "Encyclopedia" 78 at regular 13 ppem, the
application default font 4 (`FUN_00603850`; cell 16, `tmHeight` 13 + 3).
No other size or weight in the table comes near: bold 13 gives 104, regular
11 gives 78.

## Change

`game_menu.rs` measures each row with GDI's pens for font 4 and paints it
with `theme::paint_gdi_text` (`DT_LEFT | DT_TOP`) at the row's text spot.
Rows stay 20 pixels: the 16-pixel line plus 4.

## Comparison (port's Empire regiment and squadron menus, Linux Chromium)

Offsets are from the frame's top-left corner; widths are of the ink.

| Row | A0 offset | Port offset | A0 width | Port width |
|---|---|---|---|---|
| Confirmed Move | (6, 26) | (6, 26) | 90 | 91 |
| Encyclopedia | (6, 46) | (6, 46) | 76 | 77 |
| Status | (6, 66) | (6, 67) | 36 | 36 |

The squadron menu (A0 `20-fighter-menu.png`) measures the same. Both
menus are now about 103 pixels wide, as A0's. The rows, their order, the
enabled state, the frame and the fill agree, as in
`2026-10-30-game-menu-chrome-a0`.

Accepted deviations, as on 2026-10-30:

- The glyphs are anti-aliased (Liberation Sans in egui) where GDI's are
  aliased.
- The fill has no palette step.

## Tests

- `text_sits_six_pixels_in_and_centred_in_its_row` now checks the
  baseline (2 + 13 below the row's top) and the width as GDI's plus 12.
  With the old measure it fails: "width 58.125 for text 45 at 1".
- The test harness reads lines with `theme::test_lines`, since GDI text is
  painted one glyph at a time.
- `a_label_outside_cp1252_is_measured_in_the_gdi_font` covers the fallback
  for text GDI's table does not cover.

## Mutants

`game_menu.rs --re 'measure|menu_window'`: 66 tested, 37 caught, 17
unviable, 12 missed. After the fallback test, `--re measure`: 14 tested,
4 caught, 10 unviable. The misses left in `menu_window`:

- lines 430-447: the icon and submenu-arrow placement, code this change
  does not touch;
- line 460: the text rectangle's size. With `DT_LEFT | DT_TOP` only its
  corner places the text, so scaling the size differently changes nothing
  (an equivalent mutant).
