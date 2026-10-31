# The game fonts' sizes under GDI

Date: 2026-10-31.

## The original

`FUN_0060eed0` builds the game's fonts with `CreateFontIndirectA`:
- **Face.** "Arial", the only face name in REBEXE.
- **Charset, precision, quality.** `lfCharSet` 0, `lfOutPrecision` and
  `lfClipPrecision` 0, `lfQuality` 2 (`PROOF_QUALITY`).
- **Height and weight.** A positive `lfHeight` from the switch on the font
  index; see `theme::original_font_cell`.

A positive `lfHeight` is a cell height, ascent plus descent, and GDI
matches it against the font's hinted cells. It does not scale the em by
`cell * unitsPerEm / (usWinAscent + usWinDescent)`. Arial's `VDMX` table
(ratio 1:1) lists the hinted `yMax` and `yMin` for each ppem. GDI takes
the tallest `yMax - yMin` that fits `lfHeight`, at the smallest ppem with
that height.

Read from Windows 10's `arial.ttf` and `arialbd.ttf`, copied out of the A0
VM (SHA-256 `c9b76220…` and `766f06ac…`). The fonts stay outside Git; only
the table below is kept.

| Font | Cell | Weight | VDMX pick | ppem |
|---|---|---|---|---|
| 0, 4, others | 16 | 400 | 13 and 14 both 13 + 3 | 13 |
| 1, 5 | 16 | 700 | 13 and 14 both 13 + 3 | 13 |
| 2 | 14 | 700 | 11: 11 + 3 | 11 |
| 3 | 8 | 400 | below the table; hhea-rounded cell | 7 |
| 6 | 18 | 400 | 16: 15 + 3 | 16 |
| 7 | 18 | 700 | 15: 14 + 4 | 15 |
| 8, 9 | 12 | 400 | 9: 9 + 3 | 9 |
| 10 | 14 | 400 | 11: 11 + 3 | 11 |
| 11 | 24 | 400 | 21: 19 + 5 | 21 |
| 12 | 30 | 400 | 25: 23 + 5 | 25 |
| 13 | 12 | 900 (Arial Bold) | 10: 10 + 2 | 10 |

## A0 check

The text was rasterised with FreeType's v35 TrueType interpreter (the
classic hinting that GDI's black-and-white output follows), with the
advances from `hdmx` and the `kern` pairs:
- **Font 10 at 11 ppem.** Six of Churba's ten planet names match A0
  `sector-windows/04-left` pixel for pixel (Deyer, Gentes, Bespin, Hoth,
  New Cov, Storthus). The other four differ in single pixels of "a", "k"
  and "A" (rasteriser rounding).
- **Font 4 at 13 ppem.** The title "Churba" matches pixel for pixel.
- **Font 5 at bold 13.** "Resume Game Play?" in A0 `tooltip/321` differs
  in 2 of 131 × 13 pixels.
- **The kern pairs apply.** A0 draws "To" in "Tokmia" one pixel closer
  than the advances alone.

The tactical view had already fitted font 8 to 9.36 px and font 10 to
10.9 px from A0 (`tactical_view.rs`). That is about 0.78 of the cell, the
same ratio.

## The port

egui draws Liberation Sans, antialiased and unhinted:
- epaint rounds the cell to whole pixels, so not every em is available.
- Liberation's unhinted advances are wider than Arial's hinted ones.

`theme::original_font_egui_cell` therefore takes, for each font, the
whole cell whose Liberation widths come nearest to GDI's widths at the
ppem above, measured over 300 TEXTSTRA strings. The table below gives
egui's cell and the mean absolute width error per string. "Before" is the
error with the original `lfHeight` taken as egui's cell.

| Font | egui cell | Error | Before |
|---|---|---|---|
| 1, 5 | 15 | 1.40 | 7.83 |
| 2 | 13 | 1.54 | 6.29 |
| 3 | 8 | 0.65 | 0.65 |
| 4, others | 15 | 1.19 | 8.24 |
| 6 | 18 | 2.64 | 2.64 |
| 7 | 17 | 1.46 | 7.39 |
| 8, 9 | 11 | 1.02 | 7.82 |
| 10 | 12 | 1.26 | 9.60 |
| 11 | 23 | 2.88 | 4.36 |
| 12 | 28 | 0.83 | 8.14 |
| 13 | 11 | 3.31 | 4.56 |

Pixel-exact text would need GDI's hinted Arial glyphs, which may not be
shipped. That remains the accepted text deviation, now within a few
pixels a string.
