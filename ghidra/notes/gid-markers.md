# The GID's system marks (read 2026-10-22)

`FUN_0042b330(view, system id, system, ..)` sets one system's mark for the
display chosen in the GID menu (the view's `+200`, the command ids of
`cockpit::GidMode::command_id`). Display Off (`0x80`) draws nothing. The
mark's side `s` starts as the system's holder (`+0x24` bits 6..7).
`FUN_00426e70(value, s, a, b, c, d, e)` picks the mark from side `s`'s
table (`+0x294` Alliance, `+0x2a4` Empire, `+0x2b4` side 3; side 0 has
none):

- largest when `value >= a`;
- large when `c <= value <= b`;
- medium when `e <= value <= d`;
- else smallest.

| Display | Value | `a, b, c, d, e` |
|---|---|---|
| `0x11` Popular Support | the player's support (`FUN_00507270`) | 81, 80, 60, 59, 50 |
| `0x12` Uprisings | system `+0x88` bit 2 | 1, 1, 1, 1, 1 |
| `0x21` / `0x22` fleets | `FUN_004feea0(system, side, 1 / 2)`, both sides | 3, 2, 2, 1, 1 |
| `0x43` / `0x44` personnel | see below | 1, 1, 1, 1, 1 |
| `0x51` energy | `+0x5c - +0x60` | 6, 5, 3, 2, 1 |
| `0x52` raw material | `+0x64 - +0x68` | 3, 2, 2, 1, 1 |
| `0x53` / `0x54` mines, refineries | `FUN_0052d370`, `FUN_0052cf40` | 6, 5, 3, 2, 1 |
| `0x61` | `FUN_0052cc20 > 0` | 1, 1, 1, 1, 1 |
| `0x62..0x64` yards | `FUN_0052c8c0`, `FUN_0052c5a0`, `FUN_0052c270` | 5, 4, 2, 1, 1 |
| `0x65..0x67` idle yards | 15 when the player's system has an idle area (`FUN_00509670(0 / 2 / 1)`, `+0x58` = 0), else 0 | 5, 4, 2 (3 for `0x67`), 1, 1 |
| `0x71`, `0x72`, `0x74`, `0x75` | troops, squadrons, shields, batteries (`FUN_005044f0`, `FUN_00503550`, `FUN_005276d0`, `FUN_005273d0 + FUN_00527150`) | 6, 5, 3, 2, 1 |
| `0x73` Death Star shields | `FUN_0051c0d0` | 1, 1, 1, 1, 1 |

**Fleets** (`0x21`, `0x22`): the player's count and the enemy's. With both,
the mark is the special art `0x27b0` and the value their sum. With only
the enemy's, the side is the enemy's. With neither, it is the holder's
for `0x21`; for `0x22` it is the holder's only when `+0x88` bit 1 is set,
else 0.

**Personnel** (`0x43` Idle, `0x44` Active), with `s` reset to the player's
side:

- the player's characters on the planet (`FUN_004f2640(system, 3, s)`) and
  special forces (`FUN_005040c0`);
- every character and special force aboard the capital ships of every
  fleet the system holds (`FUN_004ffe70`, `FUN_00502db0`, `FUN_004f25c0`,
  `FUN_005040c0`), counted for the player or the enemy by its side.

A character is busy (Active) when:
- it is not usable (`+0x50` bit 0);
- it is on a mission (`+0x78` bit 7);
- it is captive (`+0xac` bit 0);
- it is injured (`+0x94`);
- or it holds a command (`+0x96`).

A special force is busy when it is not usable or is on a mission. The
side is the player's when it counts anyone, else the enemy's when it
counts anyone, else the holder's.

Unexplored systems (`+0x88` bit 1 clear) and destroyed ones (`+0x50`
bit 3) skip the switch for most displays (the `0x17` branch at the top).

## Ported (2026-10-22)

`rebellion-render/src/lib.rs`:
- `gid_metric_size` follows the table;
- `gid_personnel` is the personnel count with its colour;
- `gid_marker_for_side` picks a side's mark.

Before this change:
- raw material used the 6/3/1 bands;
- the idle yards counted facilities;
- the personnel displays counted only the player's people, in bands.

Since the same day the fleet displays follow the rule above
(`gid_fleets`, `gid_fleet_mark`; STRATEGY 10160 is `0x27b0`). port: an
idle fleet orbits the system; an en route one has a move order to it.
The `+0x88` bit 1 test for Fleets Enroute with no fleet is not kept: an
unexplored system shows the unexplored mark first.

Which displays the player can reach (read 2026-10-22):
- `FUN_004511e0` builds the GID menu with exactly `0x11`, `0x12`; `0x21`,
  `0x22`; `0x43`, `0x44`; `0x51..0x54`; `0x62`, `0x65`, `0x63`, `0x66`,
  `0x64`, `0x67`; `0x71..0x75`; and Display Off `0x80`, the port's menu.
- Its window (`FUN_00451130`) sends command `0x140` with the display in
  the high word. `FUN_00422ce0` hands it to `FUN_00425d00`, and the
  accelerators `0xbca..0xbd2` hand it `0x11`, `0x12`, `0x21`, `0x22`,
  `0x43`, `0x44`, `0x65`, `0x66`, `0x67` (the port's Alt+1..9).
- `FUN_00425d00` is called from nowhere else, but command `0x140` has a
  second sender. Correction (read 2026-10-22):
  - `FUN_0041d8c0(display, ..)` sends `WM_COMMAND` (`display << 16 |
    0x140`) to the galaxy view.
  - `FUN_004c0fc0(code)` calls it: code 1 and `0xb` give `0x11`, 4 and
    `0x11` give `0x17`, 5 gives `0x20`, 6 gives `0x91`, 9 gives `0x80`,
    `0xe` gives `0x13`, `0xf` gives `0x15`, `0x10` gives `0x14`.
  - `FUN_004c0a60` (the advisor's renderer) runs it through
    `FUN_004c0f60` when the pending action `DAT_006b28c4` is 2. That
    action is set by `FUN_00439eb0` from a `Class_00658848` item's
    `+0x54`/`+0x58` (slot `0x20`, `FUN_00412400` -> `FUN_0041d140`).
  - So `0x13`, `0x14`, `0x15`, `0x17`, `0x20` and `0x91` can be shown,
    though not from the menu or the keys. Who makes those items is not
    traced; the port has none of these displays.

port: a killed character counts as not usable. The port's fleets carry
no special forces. The fleets a system holds are those in orbit and
those en route to it (`movement::listed_fleets`).

The legend windows (read 2026-10-22):
- `FUN_00426d00` opens the legend through `FUN_00452090`, which keeps the
  window's size at `+0x11c`/`+0x120`; `FUN_004522f0` paints it.
- `FUN_004522f0` picks one STRATEGY.DLL `RT_RCDATA` layout per display:
  `0x11` 0x27c7 (Alliance) or 0x27c8 (Empire), `0x12` 0x2d50, `0x21`
  0x2d51, `0x22` 0x2d52, `0x43` 0x2d53, `0x44` 0x2d54, `0x51..0x54`
  0x27cc..0x27cf, `0x62..0x64` 0x27d1..0x27d3, `0x65..0x67`
  0x2d55..0x2d57, `0x71..0x75` 0x27d4..0x27d8.
- `FUN_00452630` reads a layout as `u16` words: width, height, the bitmap
  module (7, STRATEGY.DLL), the string module (2, TEXTSTRA); then items
  up to a 0 word: kind 1 or 2 a bitmap (x, y, id), kind 3 a text (x, y,
  string id, font, red, green, blue), `y` 1 centring the text across the
  top.
- The 22 layouts hold 186 bitmaps and 186 texts; every text id is in
  TEXTSTRA's string table (0x1655 is "Loyalty to the Alliance").
- `dat_dumper::types::gid_legend` decodes them and re-encodes each to its
  bytes; `rebellion_data::gid_legends` gives them with their texts.

How the legend window behaves (read 2026-10-22):
- `FUN_00427270` makes the legend button `+0x248`: 30 x 30 at (55, 50)
  for the Alliance and (113, 50) for the Empire, STRATEGY 10168
  (`0x27b8`), command `0x134`.
- `FUN_00426d00` runs after every display change: from the menu
  (command `0x140` calls it even for the shown display) and from the
  keys (inside `FUN_00425d00`, which returns early for the shown
  display). Display Off destroys the legend and hides the button; any
  other display shows the button and creates the legend at `+0x11c`,
  `+0x120`, moved in so 180 x 240 fits the client.
- `FUN_00421c70` starts that position at (150, 120) and the display at
  `0x80`, Display Off. `FUN_00425d00` keeps the visible legend's
  position before destroying it.
- `FUN_00452630` shrinks the window to the layout's size; every layout is
  180 wide (95, 135 or 155 high).
- `FUN_00452240` paints the parent's pixels under the window first, so the
  galaxy shows through; then the layout and the frame.
- `FUN_00607740` frames it from 2-pixel tiles, grey outside, white inside:
  top `0x2779`, bottom `0x277a`, left `0x277b`, right `0x2778`, corners
  `0x2775`, `0x2774`, `0x2777`, `0x2776` (top-left, top-right,
  bottom-left, bottom-right; read from the tiles' pixels).
- The close button (`0x277c`, pressed `0x277d`, 14 x 14) sits at
  `width - 14 - 3`, 3. It posts `0x405` with the legend's id `0x43`;
  `FUN_00422ce0` case 3 hides the legend and shows the button. The button
  (command `0x134`) shows the legend and hides itself.
- The window drags by its body (`FUN_00606650`: `0x201` `FUN_00606ee0`,
  `0x200` `FUN_006071e0`, `0x202` `FUN_00607010`). `FUN_00606380` sets
  `+0xf4` to 1, so it moves live and unbounded while held. The drop
  moves it back inside the bounds `FUN_006071a0` took from the parent:
  to `left` when left of them, else to `right - width` when past them,
  and the same for the height. Ported the same day.
- Fonts (`FUN_0060eed0`): index 4 is the default 16-pixel cell, 10 a
  14-pixel one, both weight 400. The face is "Arial" for every font:
  `FUN_0060ee20` copies the string at `0x6ac7b8` to the table's `+0x38`
  (called at `0x5ff83a` on the table `0x6be4f8`). The port draws it with
  Liberation Sans 2.1.5 (SIL OFL), Arial's metric match, at the cell
  height (`theme::original_font`).

The reference capture 0119
(`docs/qa/2026-09-10-interface-parity-audit/reference-captures/`) shows
the game open on Popular Support with the legend button and no legend
window, as the port does; how the original leaves the constructor's
Display Off is not read. hyp: the button opens the legend before a
display change has created one (the original's `0x134` needs `+0x124`).

What a mark may reveal (read 2026-10-22): nothing in `FUN_0042b330` asks
what the player knows. The counts walk the system's children
(`FUN_00513090`, `object-state-flags.md`) for either side, so the fleet
and personnel displays show the enemy's fleets and the people aboard
them wherever they are. The only gate is the one at the top:
- unexplored (`+0x88` & 2, `System_SetExplored` in `setters.tsv`) gives
  the unexplored mark;
- so does destroyed (`+0x50` & 8) outside `0x20..0x22`, `0x43`, `0x44`,
  `0x92`, `0x93`;
- `0x22` Fleets Enroute and `0x44` Active Personnel skip the gate and
  count even an unexplored system.

Ported the same day (`gid_shows_unexplored`, `gid_counts_unexplored`).
Before, the port gave every unexplored or unpopulated system the
unexplored mark and showed a destroyed system's values.

The mark tables and how a mark is drawn (read 2026-10-22):
- `FUN_00426ee0` fills the tables, smallest to largest: side 1 (the
  Alliance) `0x27a5`, `0x27a4`, `0x27a3`, `0x27a2`; side 2 (the Empire)
  `0x27a6..0x27a9`; side 3 (no side) `0x27aa..0x27ad`. It loads `0x27ae`
  (10158) at the view's `+0x2c4` and `0x27b6` (10166) at `+0x2c8`.
- `FUN_0042cde0` draws a system's item: with its flag `0x80` (set by the
  gate above) only `+0x2c4`, the unexplored mark; else `+0x2c8` under the
  mark when flag 8 is set, then the mark (`+0x20`) when there is one.
- A system's holder is never 0 in play: `FUN_0050b610` takes the
  regiments' side, else the support side, which is 3 (no side) for an
  unpopulated system (`system-control.md`). So an explored, unpopulated
  system shows a no-side mark sized by its value, not the unexplored
  mark. Ported the same day: the port's gate no longer asks for a
  population.
- Side 0 reaches `FUN_00426e70` only from Fleets Enroute at an
  unexplored system with no fleet; it returns no mark. hyp: the port
  shows the unexplored mark there, whether the item keeps flag `0x80`
  not being read.

A click on the map closes the legend (read 2026-10-22): `FUN_00422ce0`'s
`WM_LBUTTONUP` in the galaxy view (`+0xc0` = 1), with an item under the
pointer (`+0x25c`, tracked on `WM_MOUSEMOVE`), posts `0x405` with the
legend's id when the legend is visible, then opens the item's system
(`FUN_0042d170`, `FUN_00429ce0`). Ported the same day
(`GidLegendWindow::close_for_map_click`). Since the same day the port
opens the system on the release too, after a press on the map
(`map_release_activates`); the galaxy view has no `WM_LBUTTONDOWN` case.
