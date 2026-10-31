# The Fleet window's text against A0 (interface text)

Date: 2026-10-31. Follows `../2026-10-31-system-window-text-a0/`.

## Change

The Fleet window's text now goes through `theme::paint_gdi_text`:
- **Title.** Font 5, `DT_LEFT | DT_VCENTER` in (19, 2, 180, 16).
- **Left list's fleet names.** Font 10.
- **Selected name label.** Font 5 in (100, 29, 128, 25).
- **Picture panel's counts.** Font 10. The capacity is `DT_RIGHT`; the new
  `GdiAlign::Right` covers it.
- **Right list's names.** Font 10, centred in the 125-wide cell.

## Left list's fleet name

The name's rectangle in its row (`FUN_004c7e10`'s item) is untraced. The
port had assumed (4, 5).

A0 `system-windows/09-fleet` sets "Fleet 4" with its pen at the window's
x 11 and its baseline at 43. With the list at (4, 29) and `tmAscent` 11,
that is (7, 3) in the row, and the port now uses (7, 3).

## Check

`fleet-window.mjs --a0=.artifacts/a0-vm/system-windows` compares the ink
of "Fleet" with A0. Both windows are anchored on their red title strips,
which the chrome case matches pixel for pixel.

| Placement | Port ink | A0 ink | Result |
|---|---|---|---|
| (7, 3), this stage | [10, 34, 30, 40] | [10, 33, 31, 40] | within 1 pixel on every side |
| (5, 1), a first try | [8, 32, 28, 38] | [10, 33, 31, 40] | fails |
| (4, 5), the old placement | starts at x 7, baseline 2 pixels low | [10, 33, 31, 40] | fails |

## Gates

All pass on Linux Chromium:
- fleet-window (with `--a0`), sector-quadrants (with `--a0`),
  sector-windows, mission-dialog, status-window, personnel-orders,
  tooltip, system-finder, fleet-finder, message-display;
- `run.mjs --all`, 152 of 152.

The fleet-window unit tests read each line's pen and baseline, through
`Text::set_in` (theme::gdi_text_origin).
