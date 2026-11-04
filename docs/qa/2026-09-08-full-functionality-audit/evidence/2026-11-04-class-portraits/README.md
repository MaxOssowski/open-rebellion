# A product's picture is its class portrait (2026-11-04)

This covers the Manufacturing window's bands and Build Selection's class box. The reading is in `ghidra/notes/manufacturing-build-selection.md`.

## The original

- **The band.** `FUN_00457c90` loads GOKRES `class +0x30 & 0xfff` through `FUN_005fbd20(gokres, id, 10)`. That is the 122 by 50 portrait. The mini is that id plus `0x4000` (`FUN_0042c3b0`).
- **Build Selection.** `FUN_00437880` reads the same id and centres it in the 195 by 63 item.

## A0

The hashes are in `a0-sha256.txt`.
- **`build-selection2/07-confirmed`** (Empire, Coruscant, window at (203, 91)). The troops band shows the Imperial Army Regiment's portrait 1154 at the band's (40, 15). Between the band's lines, 0 of 580 opaque pixels are off.
- **`build-selection/10-build`** (Alliance, window at (192, 84)). The box shows the Construction Yard's portrait 258, centred, with 0 pixels off.
- **`pending-facility/32-mine-build-selection`.** The box shows the Mine's portrait, GOKRES 1, centred, with 0 sampled pixels off.
- **The name.** Both Build Selections ink the class name in the window's rows 75..84, whatever the portrait's extent.
- **`pending-facility/30-pzob-bands`.** The A-wing and the refinery stand portrait-sized in Pzob's bands.

## Before

The port drew the class mini, 61 by 25, in both places. In Build Selection it put the name under the mini.

## Now

- **`product`.** It returns the portrait, the mini less `0x4000`. `product_mini` keeps the mini for the deployment message picture (`message_pictures::deployed`).
- **The band.** It paints the portrait at `BAND_PORTRAIT`, (40, 15).
- **Build Selection.**
  - It centres the portrait in the box.
  - The name starts at `NAME_TOP`, 49 pixels below the item's top. This is a hyp: a fixed place that matches A0's rows.

## Tests

Unit tests:
- `a_product_shows_its_class_portrait_not_its_mini`;
- `the_class_name_sits_at_a_fixed_height_in_the_box`;
- `a_building_band_shows_its_product_and_units` (system_window) and `a_building_band_shows_its_product_units_progress_and_destination` (manufacturing_window) now expect the portrait.

`build-selection.mjs` (Linux, both sides):
- **`classPortraitCase`.** The opened Build Selection's portrait matches its bitmap exactly. A0 `10-build`'s portrait matches 258 exactly. The name's first and last ink rows match A0's within a pixel and two pixels.
- **`bandPortrait`.** The confirmed troops band's portrait matches exactly between its lines. For the Empire it also matches A0 `07` exactly.
