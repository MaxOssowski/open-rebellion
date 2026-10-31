# The Status window's text against A0 (interface text)

Date: 2026-10-31. Follows `../2026-10-31-fleet-window-text-a0/`.

## Change

- **Title.** Font 5, `DT_CENTER` from its box's top. Drawn through
  `theme::paint_gdi_text`.
- **Name.** Font 5, `DT_CENTER | DT_WORDBREAK`. The new
  `theme::paint_gdi_text_block` breaks it with `gdi_word_break` at GDI's
  widths and steps the lines by `tmHeight`.
- **Two-column rows.** Font 10, broken in their 103-wide columns, one
  14-pixel line (font 10's `tmHeight`) apart, each from its cell's top.
  - The old hypothesis drew these rows in font 8 with a 2-pixel drop, to
    match A0's narrow 8-pixel capitals.
  - That is what GDI's font 10 looks like at 11 ppem. A0 `status/06-fleet`
    shows "Not Assigned" at 63 by 10 and "Hyperdrive Rating:" at 87 by 10,
    both Arial at 11 ppem.
- **Icon rows.** Font 10, centred across the cell (Galaxy Overview) or
  `DT_VCENTER` on the 35-pixel row at x 40 (Objectives).

## Check

`status-window.mjs --a0=.artifacts/a0-vm/status` compares the Alliance's
Fleet Status with A0 `06-fleet`. A0's window is at (170, 81), where its
Close button matches the port's to 9 pixels in 1,000.

The title, the Status / Admiral / General / Commander labels and the first
value lie within 1 pixel of A0's ink on every side (`text-checks.json`).

On the old code (`status_window.rs` from HEAD, rebuilt) the title failed:
80 to 160 against A0's 84 to 157.

## Gates

All pass on Linux Chromium:
- status-window with `--a0`, sector-quadrants with `--a0`, fleet-window
  with `--a0`, sector-windows, mission-dialog, personnel-orders, tooltip,
  system-finder, fleet-finder, message-display;
- `run.mjs --all`, 152 of 152.
