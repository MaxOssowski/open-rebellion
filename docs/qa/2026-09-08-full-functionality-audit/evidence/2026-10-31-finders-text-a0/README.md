# The Finders' text against A0 (interface text)

Date: 2026-10-31. Follows `../2026-10-31-encyclopedia-text-a0/`.

## Change

The System, Fleet, Troop and Personnel Finders now draw their text through
`theme::paint_gdi_text`:
- **Title.** Font 5, centred in its 350.
- **Name label.** Font 4.
- **Tab label.** Font 5.
- **List rows.** Font 10, starting 5 into the row.
  - System, Fleet and Personnel Finders: `DT_VCENTER` in each 20-pixel row.
  - Troop Finder: from the row's top (format `0x20`).
- **Name boxes.** egui edit fields, laid out in the font matched to GDI's
  widths.

The rows were already in the right font. A0
`personnel-finder-empire-day167-f5` shows "Veers - Coruscant" 88 wide and
"Menndo - Corfai" 76 wide, which is Arial at 11 ppem.

## Check

`encyclopedia-placement.mjs --window=finders --a0=.artifacts/a0-vm`
compares the Empire's Fleet, Troop and Personnel Finders with A0. All
windows are at (125, 52). The title, the name label and the tab label lie
within 1 pixel of A0's ink on every side.

On the old code (the four Finders from HEAD, rebuilt) the check failed:
the Fleet Finder's title ran 296 to 376 against A0's 298 to 373.

## Gates

Listed in `gates.txt`.
