# The Game Menu Window's frame, fill and arrows against A0 (OBJ-14, CMD-05)

Date: 2026-10-30. Surface ledger OBJ-14 (original item-menu trees) and the
speed menu (CMD-05). This is the first comparison of the object menu with
the original: A0 `status/14-char-menu.png` (Veers's menu on Coruscant) and
`build-orders/12-speed.png` (the speed menu), hashes in
`a0-captures.sha256`. It is compared with the port's Empire character menu
from `personnel-orders.mjs` (`empire-character-retire/agent-menu.png`,
sha256 `341088a3…`).

## What A0 showed

- **The frame.** The top and bottom are two solid lines: grey 192 outside,
  white inside at the top, and white inside then grey 192 at the bottom.
  The sides are a grey 192 and white pair on every second row only, from
  the frame's third row; the rows between show the fill. The port tiled
  the strips the other way round: a dotted top and bottom, and solid sides.
- **The fill.** A dimmed copy of what lies behind the menu.
- **The submenu arrow.** The green arrow beside Command has no box around
  it. The port drew it on black.
- **The rows** (Move, Confirmed Move, Mission, Command, Encyclopedia,
  Status, Retire) and their places agree.

## The original

- **Frame tiles.** `FUN_00442860` (and `FUN_004511e0` for the GID root)
  hands `FUN_00607740` the tiles in the order 10105, 10106, 10107, 10104,
  10101, 10100, 10103, 10102. The A0 pixels place them as follows:
  - corners 10101 top left, 10100 top right, 10103 bottom left, 10102 bottom
    right;
  - 10105 (grey over white) and 10106 (white over grey), 1 by 2, in every
    column of the top and bottom;
  - 10107 and 10104, 2 by 1, on the sides, every second row from the third.
  
  hyp: the side loop steps by a 2-pixel tile's height.
- **Fill.** `Class_0065bef0_vt38` builds `FUN_005fddd0`'s shade table from
  `LAB_004ac550`, the sector window's transform. `FUN_004aab50`'s paint
  copies what is under the menu through it (`FUN_005fcc30`,
  `FUN_005fe050`): each channel `c` becomes `c / 6 + 50`, then the
  nearest palette entry.
- **Arrows.** The paint blits the submenu arrows keyed (`FUN_005fd0f0`). The
  key is the bitmap's first pixel (`FUN_005fd170`), which is black in
  10117/10118 and 10128/10129.

## The port

- **`cockpit.rs`:** `frame_tiles` lays the tiles as above, and
  `paint_gid_frame_border` draws them. This serves the Game Menu Window,
  the speed menu and the GID popup.
- **`game_menu.rs`:** `MENU_SHADE` is a premultiplied (50, 50, 50) at
  alpha 213 over what is behind, which leaves `50 + c × 42 / 255`.
  port: there is no palette step, so the fill is within a few levels of A0
  rather than equal to it.
- **`bmp_cache.rs`:** the submenu arrows join the first-pixel key list.
- **`run.mjs`:**
  - its GID root frame check now expects the A0 layout. It was the port's
    own layout before;
  - its Encyclopedia close point follows the window's place since s200,
    (62, 50) and (125, 52). The scenario had failed since 2026-10-29 on the
    old origin (85, 75).

## Comparison with A0

The frame's structure, the arrow's surround and the rows agree. The
differences that remain:

- **Fill level.** Over the Coruscant window, the port shows (54, 57, 67)
  where A0 shows (69, 67, 68) and (75, 75, 75): the palette step.
- **Width.** "Confirmed Move" runs about 9% wider in the port, as in every
  window: Liberation Sans with egui has no GDI hinting. So the port's menu
  is 137 pixels wide where A0's is 124.

## Browser (Arch Chromium 155, interface WASM `8a4175a2…`)

| Gate | Result |
|---|---|
| `personnel-orders.mjs` | pass (`personnel-orders-result.json`) |
| `fleet-window.mjs` | pass (`fleet-window-result.json`) |
| `run.mjs` (GID) | 4 of 4 pass; the root frame matches its tiles in 944 pixels (`gid-summary.json`) |

## Tests

- `cargo test --workspace`: all pass.
- New test: `the_frame_lays_its_tiles_as_the_a0_menus_show`.
- New test: `the_frame_paints_its_tiles_scaled_from_the_rect`.
- `build_selections_buttons_key_out_their_first_pixels_colour` now covers
  the arrows too.
- `cargo mutants --in-place` on the frame functions: 130 mutants, 126
  caught, 3 unviable, 1 missed. The missed one turns a tile's size `* scale`
  into `/ scale`. The GID gate covers it: it checks the root frame's 944
  pixels against the tiles.
- A first, unscoped mutants run was stopped by the system for low memory.
  It left `cockpit.rs` mutated, so the file was restored from the saved
  diff and checked by sha256 before going on.
