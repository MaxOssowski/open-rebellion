# The Defenses window's title-button key pixels (2026-11-12)

Cell OBJ-13-C008 (Coruscant controlled) is partial. Its note ended: "the
title buttons' bottom-left key pixel is unsettled in A0 (black here, green
or red elsewhere)". The `sector-quadrants.mjs` case `defenses-other-side`
skipped those three pixels when it compared the Empire's Defenses window of
an Alliance-held system with A0 `coruscant-676/14-after-movie`.

This round traces what the original draws under those pixels. The port
already draws it, so no port code changes. The gate now compares the three
pixels too.

## A0 used

The PNGs stay in the git-ignored `.artifacts/a0-vm/` of the main checkout.

| File | SHA-256 | What it shows |
| --- | --- | --- |
| `coruscant-676/14-after-movie.png` | `c09a8f4832b492eb073ce68710912f1f6a7eb2c5be25078de3b565472b9e56ec` | The Empire's Defenses window of the Alliance-held Coruscant, red strip, at (200, 91) |
| `garrison-clean/test-day483-yavin-regiments.png` | `95003b7e9f8448a70596a3cc3943e0e29171f44912ccdf92928caf05474007a1` | The Alliance's own Defenses window (Yavin) |
| `status/17-def-tab.png` | `08616b81a885e4458aaa268bf254f02af5b338ebe61762ae6093f7da84dedea6` | The Empire's own Defenses window (Coruscant), green strip |
| `fleet-aboard/00-coruscant-fleet1-ships.png` | `55cb1d67f4c9111edcfeccd6374129d4cde8cbd21facb370e384c9a65a6a8635` | A Fleet window, green key pixels |
| `status/22-battery-menu.png` | `7f4c3cff8729282d9b479918c7e46b34db3f47477e85d3158d977bfb97e0660d` | A Defenses window (black) and a Fleet window (green) side by side |
| `menus2/11-mission-dialog-own-cell.png` | `ca5a223af655144352f3ddc71182266d111fc4b44698a0e92e190d913d30329f` | The Create Mission dialog, red at its close box's corner |

## What A0 shows

`coruscant-676/14`, window origin (200, 91), at window points:

| Point | A0 | Neighbours |
| --- | --- | --- |
| (3, 16), sector button | (0, 0, 0) | (2, 16) is the strip's red; (3, 15) is the button's white |
| (204, 16), minimize | (0, 0, 0) | |
| (218, 16), close | (0, 0, 0) | |

The same three pixels are black in `garrison-clean` (Alliance, own side)
and `status/17` (Empire, own side). So for the Defenses window the pixel is
black whatever the strip's colour.

The "green or red elsewhere" captures are other windows:

- **Green.** `fleet-aboard/00..02` and the right-hand window of `status/22`
  are Fleet windows. `status/22` shows green at window x 569 and 583, y 106,
  under a red strip. The Fleet window blits its buttons plain
  (`FUN_004a4b10`; settled in `2026-11-12-window-frames`, port
  `KeyPixel::Colour`).
- **Red.** `menus2/11` shows red (255, 0, 0) at (410, 54), the close box's
  corner of the Create Mission dialog, a different window class. It is not
  traced here.

## The trace

All functions are in `~/Games/rebexe-decompile/decompiled-typed/`.

1. `FUN_004a8790` (Defenses window create, class `0065be30` vt38) loads
   `0x2951` (10577) with `FUN_005fbd20` and hands it to `FUN_006075b0`.
   That stores it at the window's `+0xa4`, the window surface. There is no
   side branch: one id for every side.
2. The same function creates the three title buttons with `FUN_00606c90`
   (`0x277c/0x277d`, `0x280d/0x280e`, `0x27e1/0x27e0`).
3. `FUN_00606c90` passes each button `FUN_006073d0()`, which returns the
   window's `+0xa4`, and the button's x and y. `FUN_00602150` stores them
   as the button's backdrop and its `+0x10c`, `+0x110`.
4. `FUN_00602d30` (the button paint) copies the backdrop from that surface
   at (`+0x10c`, `+0x110`) into its composite `+0xa8` (`FUN_005fcc30`).
   Then it blits the art over it keyed on its first DIB pixel
   (`FUN_005fd0f0`). So the key pixel, the art's bottom-left corner,
   shows 10577 at (x, 16).
5. The title strip is painted by `FUN_004a9800` (vt44). It picks the strip
   by focus (`+0x14c` `+0xb8` against `+0x18`) and blits it with SRCCOPY
   (`FUN_005fc140`) straight to the paint DC. It never writes into
   `+0xa4`. `FUN_00457690` (the System window's paint) does the same.

So the key pixel shows the window background 10577 at the button's
corner, whatever the strip's side or focus. 10577 is black at (3, 16),
(204, 16) and (218, 16), which is what all three Defenses captures show.

## The port

`defenses_window.rs` already repaints each button's key pixel from 10577
(`paint_key_pixel`, `KeyPixel::Background(BACKGROUND)`), with no side or
focus condition. That fix came from round 5 of `2026-11-12-window-frames`.
The 2026-11-11 b191 run that left the pixel unsettled predates it.

- New test:
  `defenses_window::tests::the_title_buttons_key_pixels_show_the_background_whatever_the_strips_side_and_focus`.
  It opens an Empire-held system for the Alliance (the other side),
  unfocused, checks that side 2's unfocused strip 10302 is painted, and
  asserts all three key pixels repaint from 10577. It pins the traced
  behaviour; there is no code change for it to fail without. With the
  `paint_key_pixel` call removed it would fail, as the existing
  `the_title_buttons_key_pixels_show_the_windows_background` does.

## The gate

`sector-quadrants.mjs`, case `defenses-other-side`: the `buttonKeys` skip
is removed. The Empire case now compares every title-row pixel outside the
name's label (x 19..198) with A0 `coruscant-676/14`, the three key pixels
included.

Run on Linux, Arch Chromium, headless, 640 by 480, with the interface-test
WASM of this tree (`wasm_sha256`
`4d38223d394a41201f901010c77ac4d6051b9533cc9c7ca4cc3dede5c16c4dd5`):

- `sector-quadrants.mjs --no-build --only=empire/defenses-other-side
  --only=alliance/defenses-other-side`: 2 of 2 cases pass. Run
  `sector-quadrants-2026-11-12T11-58-57-217Z-946660` (in the agent
  worktree's `.artifacts/interface-parity/`, `result.json` SHA-256
  `432ee4ede300664fcf47ee70ab7312bee7334adb27dd1bc8cb09c59313cd5a4a`).
- Empire case against A0: 816 pixels compared (813 before, plus the three
  key pixels), 0 differ. The name's ink starts at (20, 5) in both and ends
  at y 14 in both ("Coruscant" in A0, the fixture's shorter name in the
  port).
- The A0 file was copied into the worktree's `.artifacts/a0-vm/` for the
  run; without it the case skips the A0 comparison.

## Checks

- `cargo test -p rebellion-render`: 1494 passed, 6 ignored.
- `cargo check --workspace`: ok.
- `rustfmt --check` on `defenses_window.rs`: nothing reported for the new
  test.
- Not run: the full batch, `run.mjs` and the tactical gates (other agents).

## Still open for OBJ-13-C008

- The fixture system is not Coruscant. No browser case opens Coruscant
  itself after it changes hands.
- The Create Mission dialog's red key pixel (`menus2/11`) is another
  window and is not traced here.

## Save format

Unchanged.
