# The System window's text against A0 (CMD-04, interface text)

Date: 2026-10-31. Follows `../2026-10-31-gdi-text-placement/`.

## Change

The System window's text now goes through `theme::paint_gdi_text`, which
uses GDI's ppem, `tmAscent` baseline and Arial's pens:
- **Title.** Font 5, `DT_LEFT | DT_VCENTER` in (19, 2, 179, 16).
- **Bands' lines.** Font 10, `DT_TOP` at their text objects.
- **Yard counts.** The default font, `DT_CENTER` in (6, y, 46).
- **Page label.** Font 4, centred in the window's 226.
- **Reference rail's name.** Font 10, after the icon, from the slot's top.

## A0 check

`sector-quadrants.mjs --a0=.artifacts/a0-vm/system-windows` compares the
open System window with A0 `07-icon-dbl` (Yavin's window at (315, 87),
Alliance). The port's window stands one pixel right of and below its
reported origin; the tab strip's art matches A0 at that offset.

| Text | Port vs A0 | Old code |
|---|---|---|
| "No Ships are being built", "No Troops in training", "No Facilities are being built" | ink box within 1 pixel on every side; bottom rows exact | fails: "No Ships…" ended at x 189 against A0's 170 |
| the troops' and facilities' "0:0" counts | ink box within 1 pixel on every side | — |
| rail name (A0 `08-minimized`, slot (544, 61)) | left and bottom exact, top 1 pixel lower | — |

The old-code run (`system_window.rs` from HEAD, rebuilt, `--only=alliance/open-system`)
is how the "fails" result was obtained.

The boxes are in `text-checks.json`. The glyphs are still Liberation's, so
ascenders and side bearings may differ by a pixel.

## Gates

All pass on Linux Chromium:
- sector-quadrants with `--a0`;
- sector-windows, fleet-window, mission-dialog, status-window,
  personnel-orders, tooltip, system-finder, fleet-finder,
  message-display;
- `run.mjs --all`, 152 of 152.

`cargo test -p rebellion-render`: all pass. The system-window tests now
read each line's pen and baseline: `line_at` gives the box's top plus
`tmAscent`. Font sizes come from `gdi_font`.
