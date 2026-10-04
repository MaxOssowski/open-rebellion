---
title: "The Sector Window's Quadrant Icons, the System Defenses Window, and the Missions Window"
description: "FUN_00459e30's four overlay items around each planet: their show rules (FUN_0045cdc0, FUN_0045ce80, FUN_0045ccc0, FUN_0045d090), art (FUN_0045ca80), the windows they open (types 9, 10, 4, 11), and the layout of the System Defenses (FUN_004a7790) and Missions (FUN_0049f130) windows"
category: "ghidra"
created: 2026-10-04
updated: 2026-10-04
tags: [sector-window, quadrant-icons, system-defenses, missions-window, strategy-resources]
---

# The sector window's quadrant icons

Recovered 2026-10-04 with Ghidra 12.1.3 headless (read-only project). Every
function named here has a `FUN_<address>.c` note in this directory. Functions
that the project leaves undefined were created in memory only
(`CreateAndDecompileTargets.java`). Bitmaps are STRATEGY (`FUN_006037f0(7)`)
unless stated. The fleet quadrant and the Fleet window are in
`fleet-window.md`.

## The four items

`FUN_00459e30` gives each system four overlay items (`FUN_00442130`). Each
item is `w` by `h` = 28 by 19, the 27 by 18 icon `0x2a13` (10771) plus one.
`(cx, cy)` is the planet's center `(x + 18, y + 18)`. The rects are
`(left, top, right, bottom)`, right and bottom exclusive (`:369-447`):

| Flag | Kind (`flag >> 16`) | Quadrant | Rect | Show rule | Opens (`FUN_0045aac0`) |
|---|---|---|---|---|---|
| `0x40000` | 4 | top-left | `(cx - 28, cy - 19, cx, cy)` | `FUN_0045cdc0` | type 9, the System window (`FUN_00452fc0`, 226 by 304) |
| `0x80000` | 8 | bottom-left | `(cx - 28, cy + 1, cx, cy + 20)` | `FUN_0045ce80` | type 10, System Defenses (`FUN_004a7790`, 235 by 304) |
| `0x100000` | `0x10` | top-right | `(cx + 1, cy - 19, cx + 29, cy)` | `FUN_0045ccc0` | type 4, the Fleet window (`FUN_004a2630`) |
| `0x400000` | `0x40` | bottom-right | `(cx + 1, cy + 1, cx + 29, cy + 20)` | `FUN_0045d090` | type 11, the Missions window (`FUN_0049f130`, 235 by 304) |

Each window id is `(system index & 0x3ff) << 6 | type`. There is one window
per id (`FUN_00604500` on the galaxy view's list `+0x6c`, as for the Fleet
window).

## Shared tail: FUN_0045d140(item, state, enabled)

- **State.** It is a side: 1, 2, or 0 and 3 for neither. It is stored in item
  `+0x3c` bits 2..4 (4, 8, `0x10`). A changed state repaints the item's two
  bitmaps from `FUN_0045ca80(kind, state, 0/1)` (`FUN_0060bd20`).
- **Enabled.** It is stored in item `+0x54`. Zero removes the item
  (`FUN_00600db0`) and clears `+0x3c` bit 1. Nonzero sets bit 1 and paints
  it. **An item with nothing to show is not drawn**, which is the same rule
  as the fleet icon.

## Art: FUN_0045ca80(kind, state, second)

| Kind | State 1 | State 2 | State 0 or 3 |
|---|---|---|---|
| 4 | 10771/10772 (`0x2a13/14`) | 10779/10780 (`0x2a1b/1c`) | 10787/10788 (`0x2a23/24`) |
| 8 | 10773/10774 (`0x2a15/16`) | 10781/10782 (`0x2a1d/1e`) | 10789/10790 (`0x2a25/26`) |
| `0x10` | 10775/10776 (`0x2a17/18`) | 10783/10784 (`0x2a1f/20`) | none |
| `0x40` | 10777/10778 (`0x2a19/1a`) | 10785/10786 (`0x2a21/22`) | none |

Any other state returns 0. The second id is the item's other state.
hyp: it is the pressed or highlighted state, as in `fleet-window.md`.

## The show rules

The DatId families come from the counting helpers. Each walks the system's
objects in a family range through `FUN_00513050`, with side filter 3 for
every side, and `FUN_00513180` counts them.

| Family range | Helper | What it covers |
|---|---|---|
| `0x10..0x14` | `FUN_00504c40` | regiments |
| `0x1c..0x20` | `FUN_005039d0` | fighter squadrons |
| `0x22..0x28` | `FUN_00526fd0` | defense facilities |
| `0x28..0x30` | `FUN_0053b6e0` | manufacturing (`0x28..0x2a`) and production (`0x2c..0x2d`) facilities |
| `0x30..0x40` | `FUN_00536da0` | characters and special forces |

A character's mission key is `+0x68` (`FUN_0042d170`). `FUN_004ece60` is
false only for the empty key: id `2` with family 0. Role flags `+0x78` bit 8
is OnHiddenMission (`decoy-roll.md`). A member is **on a visible mission**
when its key is set and bit 8 is clear.

- **Kind 4, top-left** (`FUN_0045cdc0`).
  - The state is the system's side bits `(+0x24 >> 6) & 3`.
  - Enabled is the count of manufacturing and production facilities at the
    system.
  - A zero count forces state 0, but the item is hidden then anyway.
  - The icon shows when the system has a shipyard, training facility,
    construction yard, mine or refinery. It takes the system's side colour.
- **Kind 8, bottom-left** (`FUN_0045ce80`).
  - The state is the system's side bits.
  - Enabled is the sum of the defense facilities, regiments and fighter
    squadrons, plus every character or special force at the system that is
    **not** on a visible mission (no key, or OnHiddenMission).
- **Kind `0x40`, bottom-right** (`FUN_0045d090`). The state is
  `FUN_004a1f60(system, player side)`. It walks the characters and special
  forces on a visible mission and collects their distinct mission keys
  (`FUN_004f5940`, `FUN_004f44b0`), split by the member's side bits into the
  player's set and the other side's set. It returns:
  - the other side when that set is non-empty;
  - otherwise the player's side when the player's set is non-empty;
  - otherwise 3.
  
  The item is disabled for state 0 or 3. **The other side's missions take
  precedence.**
- **Kind `0x10`, top-right** (`FUN_0045ccc0`): see `fleet-window.md`.

The object these rules read is the galaxy view's system. hyp: it is the
player side's view (`FUN_00539fd0`), so enemy objects appear only as the
player knows them. The port must decide this with a `port:` rule, using the
visibility already used by the System and Fleet windows.

## Type 10: the System Defenses window (FUN_004a7790)

Manual p. 125, Fig. 3.73. The class is vtable `0x0065be30`, over
`FUN_004ac120` (`CustomDialogBox`), 235 by 304. GOKRES is `+0x15c`. The
subject system is `+0x144`.

Vtable slots: `[0]` `FUN_004a78d0`, `[5]` `FUN_004a86d0` (window proc),
`[9]`/`[10]` `FUN_004aa6d0`/`FUN_004aa760`, `[14]` `FUN_004a8790` (create),
`[17]` `FUN_004a9800`, `[18]` `FUN_004a9890`, `[19]` `FUN_004aa2b0`,
`[22..25]` `FUN_004a7a20`, `FUN_004a7b40`, `FUN_004a7f10`, `FUN_004a7f50`
(refresh), `[26]` `FUN_004aa380` (the item under a point), `[27]`
`FUN_004aa4d0`, `[29]` `FUN_004aa4a0`.

- **Side shown** (`+0x148`): the system's side bits (`FUN_004a8790:42`).
  `FUN_004a7f50` refreshes it when they change.
- **Background**: `0x2951` (10577, GOKRES-sized, `FUN_005fbd20(.., 10)`).
  Title buttons are `0x277c/0x277d` (close, id 200), `0x280d/0x280e`, and
  `0x27e1/0x27e0` (id `0xca`), as in the other windows.
- **Tab strip** (`FUN_0060d590` at (0, 20), 304 by 33). There are five
  buttons, 36 by 33, each with help message `0x1740 + n`:

  | Id | x | Content | Count helpers | Art (side 1 / side 2 / other; normal, disabled = normal + 2) |
  |---|---|---|---|---|
  | 5 | 172 | KDY-150 (`0x22`) and LNR batteries (`0x23`) | `FUN_005273d0` + `FUN_00527150` | `0x2936`/`0x2937`, empty `0x2938`; every side |
  | 4 | 136 | GenCore shields (`0x24`) and the Death Star Shield (`0x25`) | `FUN_005276d0` + `FUN_0051c0d0` | `0x2939`/`0x293a`, empty `0x293b`; every side |
  | 3 | 100 | fighter squadrons (`0x1c..0x20`) | `FUN_00503550` | `0x293c`/`0x293d` and `0x293e`, `0x293f`/`0x2940` and `0x2941`, else `0x2942` |
  | 2 | 64 | regiments (`0x10..0x14`) | `FUN_005044f0` | `0x2943`/`0x2944` and `0x2945`, `0x2946`/`0x2947` and `0x2948`, else `0x2949` |
  | 1 | 28 | personnel not on a visible mission (`0x30..0x40`) | `FUN_00536e20` walk | `0x294a`/`0x294b` and `0x294c`, `0x294d`/`0x294e` and `0x294f`, else `0x2950` |

  `FUN_004a9ce0` sets each button's normal bitmap: the "has some" id when its
  count is nonzero, the empty id otherwise. The defense facility families
  come from DEFFACSD.DAT:
  - KDY-150 `0x22`;
  - LNR Series I and II `0x23`;
  - GenCore Level I and II `0x24`;
  - Death Star Shield `0x25`.

  Every count is filtered to the shown side (`+0x148`).
- **Side bitmaps** (`FUN_004a9ce0` `+0x5f/+0x60/+0x5d`):
  - side 1: `0x283b`, `0x27d8`, `0x2952`;
  - side 2: `0x27d9`, `0x283e`, `0x2953`;
  - side 3: `0x283f`, `0x2840`, `0x2953`.
- **List** (`FUN_00607ea0` at (7, 81), 222 by 210):
  - item art `0x29cc`;
  - row height `0x1b`;
  - text colour `0x20000ff` for side 1, `0x200ff00` for side 2, `0x2ffff00`
    otherwise (COLORREF: red, green, cyan).
  
  `FUN_004a90d0(page)` fills it, and page 1 is selected first.
- **Labels**: the title `+0x49` is the system name (`FUN_004f62d0`) at
  (title button width + 5, 2), 16 high, font `0x24`. `+0x5b` is at (2, 51),
  231 by 16; `+0x5c` is at (2, 63), 228 by 17. Both are white, font `0x25`.
  `FUN_00601b30` sets an item's position, and its `+0`/`+4` fields are its
  width and height.

Still to trace before the port (phase 1c):
- `FUN_004a90d0`'s rows per page and their text;
- `FUN_004a86d0`/`FUN_004a9890` input;
- the drop rules in `FUN_004aa380`/`FUN_004aa4d0` (`move-order.md` row 10);
- minimize and its rail icon.

## Type 11: the Missions window (FUN_0049f130)

The class is vtable `0x0065bd10`, over `FUN_004ac120`, 235 by 304. The
subject system is `+0x144`, and `+0x1c8` holds the selected mission's key.

Vtable slots: `[0]` `FUN_0049f320`, `[5]` `FUN_0049f4b0` (window proc), `[9]`/`[10]`
`FUN_004a0b00`/`FUN_004a0b90`, `[14]` `FUN_0049f540` (create), `[17]`
`FUN_0049fef0`, `[18]` `FUN_004a0120`, `[19]` `FUN_004a0e00`, `[22..25]`
`FUN_004a0400`, `FUN_004a0560`, `FUN_004a06d0`, `FUN_004a0870`, `[26]`
`FUN_004a09a0`, `[27]` `FUN_004a1e10`, `[29]` `FUN_004a21c0`.

- **Subject.** `FUN_004a1590` lists the distinct missions of the characters
  and special forces at the system that are on a visible mission (the same
  walk as `FUN_004a1f60`). Its title text is `0x2b77`, or `0x2b78` when the
  player is not side 1 (`+0x9c`).
- **Background**: `0x2b9d` (11165). Title buttons: `0x27e1/0x27e0` (id
  `0xca`), `0x277c/0x277d` (id `0x14`), `0x280d/0x280e` (id `0x15`).
- **Mission list** `+0x1b8`: at (5, 24), 94 by 275, art `0x29fc`, white,
  `+0xe4` 16.
- **Second list** `+0x1c0`: at (107, 145), 117 by 147, art `0x29fc`, white.
  hyp: the selected mission's members.
- **Tab strip** (`FUN_0060d590`, id `0x16`): at (105, 127), 122 by 16. It
  has two buttons, 61 by 43:
  - id `0x17`: `0x2d28`/`0x2d29`, message `0x8500`;
  - id `0x18`: `0x2d2a`/`0x2d2b`, message `0x8501`.
- **Text**: `+0x174` at (109, 91), 113 by 32, white, font `0x11`; `+0x178`
  at (109, 25), 113 by 16, white, font 1.
- **Picture** at (108, 37): `FUN_004a09a0` hit-tests it (`move-order.md` row
  11).

Still to trace before the port (phase 1d):
- the row text and art of both lists;
- what each tab shows;
- the picture's source;
- input and drop rules;
- minimize.

## Port notes

- The System window (type 9) is already ported (`system_window.rs`). Kind 4
  needs only its icon and the double click.
- The rules above give each icon's state and visibility. Phase 1b ports them
  with these tests:
  - each kind is hidden at zero;
  - kind 4 and kind 8 take the system's side;
  - kind 8 counts personnel only when they are not on a visible mission;
  - kind `0x40` prefers the other side's missions.
