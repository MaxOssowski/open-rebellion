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
- `FUN_00425d00` is called from nowhere else. So `0x13..0x17`, `0x61` and
  `0x91..0x93`, which `FUN_0042b330` still handles, are never shown; the
  port needs no menu entry for them.

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
  `0x200` `FUN_006071e0`, `0x202` `FUN_00607010`) within the parent's
  rectangle (`FUN_006071a0`).
- Fonts (`FUN_0060eed0`): index 4 is the default 16-pixel cell, 10 a
  14-pixel one, both weight 400; the face name (`+0x38`) is not read.

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
unexplored mark and showed a destroyed system's values. port: an
unpopulated system still shows the unexplored mark, and so does an
unexplored one that the two counting displays find empty; the original's
mark for a side without a table (`FUN_00426e70` returns 0) is not read.
