# The GID menu, its submenus, captions and legend button against A0 (CMD-02)

Date: 2026-10-30. Surface ledger CMD-02 (Galaxy display and GID modes).

## A0 captures

These are new, from the Windows 10 VM (Empire, day 208); hashes are in
`a0-captures.sha256`:

- `00-menu.png`: the GID menu.
- `01-loyalty-sub.png` and `sub-*.png`: the six submenus.
- `d-*.png` and `03-uprisings.png`: all 21 displays, chosen from the menu,
  with their captions.
- `d-display-off.png`: Display Off.

The menus open on a press of the GID control (519, 434). A child is chosen
by releasing the left button over it; the keyboard does not choose.

## The original

- **The root window.** `FUN_00422ce0` creates it through `FUN_00451130`:
  160 by 160 at (420, 233) for the Empire and (370, 233) for side 1. The
  A0 frame is at (420, 233)..(579, 392).
- **The items** (`FUN_004511e0` → `FUN_004ab1f0`):
  - the six categories at x 30, y 10 + 22k, 20 high;
  - Display Off at (50, 142);
  - in a submenu, the children at x 30, y 7 + 22k;
  - text white, and the side's colour on the highlighted row
    (`FUN_004ab1b0`).
- **The submenus** (`FUN_004aa960`):
  - 240 wide, and 55, 55, 55, 100, 142 and 122 high for Loyalty through
    Defense;
  - placed by `FUN_004ab3a0`. In A0 they sit at x 183 and y 240, 262, 284,
    232, 212 and 254, as the rule gives.
- **What A0 measures:**
  - the category icons at (30, row top), opaque: 400 of 400 pixels equal;
  - the submenu arrow at (7, row top): 10129 dark, or 10128 lit on the
    highlighted row, 56 of 56 pixels equal;
  - the text from the row's top in a 16-row cell;
  - no mark on the current display: Popular Support's icon is untouched,
    400 of 400 pixels.
- **The default font** (the game root's `+0xb0`) is font 4, which
  `FUN_00420550(4)` sets from `FUN_00421790` and `FUN_0044f180`. That is
  `FUN_0060eed0`'s default 16-row regular cell. The A0 captions' capitals
  are 10 rows high, from y 43, under the aperture's top at 40: that cell.
  `2026-10-28-gid-caption/` had chosen the 14-row cell by width alone,
  before GDI's narrower hinted widths were known.
- **The legend button** (10168, `FUN_00427270`'s 30 by 30 at (113, 50)) is
  blitted keyed on its first pixel. A0 shows no white border.

## The port

- **`cockpit.rs`:** `gid_root_rect`, `gid_submenu_rect`, `paint_gid_row`
  and `draw_gid_menu` draw both windows with that geometry. They use the
  Game Menu Window's shaded fill (`game_menu::MENU_SHADE`), its frame
  (`paint_gid_frame_border`), and `game_menu::submenu_arrow`.
  - Removed: the port's own 21-pixel rows at (425, 230), the always-lit
    arrows, the egui frame fill, and the invented check mark on the current
    display.
  - port: the submenu opens on hover, without the original's 500 ms timer.
- **`lib.rs`:** `DEFAULT_FONT` is 4. The System window's yard counts use it
  too.
- **`bmp_cache.rs`:** 10168 joins the first-pixel key list.
- **`run.mjs`:** the GID gate clicks the original's rows and submenus, and
  checks the root frame at (370 | 420, 233), 160 by 160.

## Comparison (Empire, A0 against the port's `menu-root.png`, sha256 `1d7166ef…`)

- **The root menu.** The text rows fall on the same lines in all seven
  rows (246..258, 268..277, … 378..391). The icons, the arrows and the
  frame agree. Left: the fill's palette step, and the text runs about 9%
  wider (no GDI hinting), both accepted on 2026-10-30.
- **The captions.** Alliance Popular Support spans y 43..55 in both, and
  the Empire's displays agree in height and centre. They are wider by the
  hinting difference.
- **Not compared:** the system marks. The A0 game state (day 208) differs
  from the fixtures, and a display dims the whole galaxy, so a mark cannot
  be isolated against Display Off. The display cells stay partial until a
  matching state is captured.

## Browser (Arch Chromium 155, interface WASM `482e8007…`)

`run.mjs --all`: 152 of 152 pass (`gid-summary.json`). That includes every
display chosen through the new menu, for both sides, at 640 × 480 and
letterboxed 1280 × 800.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `the_gid_menu_and_its_submenus_sit_where_the_original_places_them`;
  - the default-font check in `the_gid_caption_hangs_from_the_aperture_top_centre`.
- The first-pixel key test now covers 10168.
- `cargo mutants --in-place` on `gid_root_rect` and `gid_submenu_rect`: 191
  mutants, 190 caught, 1 missed. The missed one turns `<` into `<=` in
  `FUN_004ab3a0`'s fit test. It differs only for a submenu that ends exactly
  on the root's bottom, which none of the six heights does. The drawing is
  covered by the GID gate.
