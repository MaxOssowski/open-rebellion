# Build Selection against A0: keyed buttons and the original fonts (OBJ-08)

Date: 2026-10-30. Surface ledger OBJ-08 (Build Selection dialog). This is
the first pixel comparison with the original: A0 `build-orders/15-t01.png`,
sha256 `787c9d96…` (Windows 10 VM, Empire, the agent's Build Troops on
Coruscant; `../2026-10-30-agent-build-orders/`). In that capture the window
is at (255, 87).

## What A0 showed

`build-selection.mjs` compared the port only with its own composition of
the raw bitmaps, so it passed while the port differed from A0. The
differences were:

1. **Button corners.** The list arrow (10606), the step arrows (10610,
   10612) and the bottom buttons (10592..10597) showed their key colour:
   magenta, pink and blue. A0 shows the window behind them.
2. **Fonts.** The title, labels and values were drawn smaller than A0, in
   generic sizes.
3. **Days.** The "s" of "Days" spilled one pixel past its 60-pixel field.
   This was the gate's long-standing 6-pixel failure.

## The original

- **Button keying.** A `CoolStrobeButton` paints its bitmap keyed
  (`FUN_00602d30` → `FUN_005fd0f0`). The key is the palette index of the
  bitmap's first byte of bits, which `FUN_005fd170` stores when the bitmap
  loads. For a bottom-up DIB, that is the bottom-left pixel: index 65 for
  10606, 174 for 10592..10597 and 252 for 10610..10613. The earlier
  port-wide black, blue and lime key lists are special cases of this rule.
- **Fonts.** `FUN_00438500` and `FUN_00437f80` set them:
  - the title in font 5, format `0x21` (centred, from the box's top);
  - "Number to build:" and the two costs in font 4;
  - the time labels and the times in font 10;
  - the class list in font 10;
  - the number edit in font 4.
  
  All fonts are Arial (`FUN_005ff830`, `FUN_0060eed0`).
- **Clipping.** The values draw with `DrawText`, which clips to its
  rectangle (hyp: no `DT_NOCLIP`).

## The port

- `bmp_cache.rs`: `uses_first_pixel_key` and `first_pixel_colour` key
  10592..10597, 10606..10607 and 10610..10613 on their first pixel's colour.
  port: keying is by colour, so a palette index that shares the key's colour
  is keyed too.
- `build_selection.rs`: uses the original fonts (`theme::original_font`),
  hangs the title from the top, and clips the times to their field.
- **The gates now compose the expected chrome keyed:**
  - `build-selection.mjs` and `mission-dialog.mjs` blit the button bitmaps
    keyed on their first pixel, so the expected chrome follows the original
    rule.
  - `build-selection.mjs` widens the time labels' masks by 5 pixels (to
    x 140). The port's unhinted Arial metrics run "Best Time To Completion:"
    to x 137, where GDI's hinted glyphs end at 126 in A0. This is the known
    font-metric difference: Liberation Sans with egui has no GDI hinting.
  - `mission-dialog.mjs` puts the Empire's dialog at x 230, not 231.
    `FUN_00606980` centres truncating (120 + (480 − 259) / 2), as A0 showed
    on 2026-10-29. The gate had failed for the Empire since then.

## Comparison with A0 (window at (255, 87))

- **Matching:** the background, the title strip, the close box, the list
  arrow, the bottom buttons and their keyed corners, and the step arrows'
  black surround.
- **Measured text, A0 against the port:**
  - the title's caps (font 5): equal height and placement;
  - "Number to build:" right edge: 135 against 136;
  - the time labels' cap height: 9 against 9 (font 10). Their width: 116
    against 126. This is GDI hinting, not the font size;
  - the times' right edge: 198 against 199.
- **Not compared:**
  - The class shown differs: A0 opened on the Imperial Army Regiment, the
    class last built (`DAT_006b28a0`), while the fixture opens on its first
    class. So the item picture and the name's height are not compared.
  - A0 shows the number edit's caret, which the port does not draw.

## Browser (Arch Chromium 155, interface WASM `50bad2fa…`)

Both gates pass for both sides:
- `build-selection.mjs`: "opened" has 0 of 28,646 pixels different; Confirm
  and Stop work (`build-selection-result.json`).
- `mission-dialog.mjs`: every chrome comparison has 0 different pixels
  (`mission-dialog-result.json`).

## Tests

- `cargo test --workspace`: all pass.
- New test: `build_selections_buttons_key_out_their_first_pixels_colour`.
- Changed expectation: `texts_scale_with_the_window_and_name_the_class` now
  expects the original fonts (`FUN_00438500`, `FUN_00437f80`). The old
  sizes were the port's own.
- `cargo mutants --in-place --in-diff` on `bmp_cache.rs` and
  `build_selection.rs`: 15 mutants, 10 caught, 5 unviable.

## Still open for OBJ-08

- A0 captures of:
  - the ship, fighter, special force, facility and defense lists;
  - an unbuildable class ("n/a", Confirm disabled);
  - a changed quantity;
  - Cancel.
- The number edit's caret.
- The insufficient-resources state: the port does not charge maintenance
  or draw down refined material.
