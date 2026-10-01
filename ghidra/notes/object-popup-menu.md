# Object pop-up menu and targeting

This note records how the player starts an order on an object: the right-click
pop-up menu, its items, and the targeting cursor that picks the order's target.
The mission dialog (`mission-dialog.md`) opens at the end of this path.

The manual says the same: "The general procedure to initiate a mission is to
right-click on a character or team and select Mission. When you do this, the
cursor changes to the cross hair, then select a target for the mission"
(p. 100, also pp. 39 and 57). A left-button drag onto the map is a move, not a
mission (below).

## Opening the menu

`FUN_006007b0`, the base window procedure, sends `WM_RBUTTONUP` (0x205) to the
window's vtable slot `+0x1c` with the cursor point. A list control
(`CoolDragList`, vtable `0x0066e0f0`) passes it to its parent through
`FUN_00609fc0`, which maps the point into the parent's coordinates. A system
window (vtable `0x00659e68`) handles it in `FUN_004ac5c0`:

1. Nothing happens while any window holds the mouse capture.
2. The window's selection (vtable `+0x58`) is copied to `+0x11c`.
3. `FUN_0041dcc0` → `FUN_004fcf20` → `FUN_0051d990` lists the orders the
   selection may take (below).
4. Each order becomes an item through `FUN_00442590(order, module 7, flags,
   x, y, galaxy view, text, 0x2ffffff, 0x2808080)`. Flags bit 0 is set when
   the order is disabled and bit 1 when it is checked. The text color is
   `0x20000ff` for side 1 and `0x200ff00` otherwise, as in the speed menu
   (`game_speed.rs`).
5. `FUN_00442380` opens the menu at the point, in the galaxy view.

## The items

`FUN_0051d990(side, selection, out)`:

- Each selected object's class lists its orders (class vtable `+0x3c`, given
  the object's status). The first object's list is kept, and each later list
  is intersected with it.
- For each order kind, a temporary order is built with the selection as its
  team and no target. The kind is listed when the order's vtable `+0x10`
  accepts it. The item is enabled when `FUN_0051de80` passes and vtable
  `+0x18` accepts it, and checked when `+0x14` says so.
- Encyclopedia (0x100) is always added, enabled for a single selection.
  Status (0x103) is always added, enabled for a single selection that is not
  a system (family `0x90..0x97`).

Each item reads a 26-byte `RT_RCDATA` record named by the order kind from
STRATEGY.DLL (`FUN_00442790`). Its words are: 0 the kind, 1 the parent
submenu, 2 the sort key, 5 the TEXTSTRA string, 7 and 8 the icon, 10 a second
bitmap, and 12 the module. The items a character may show:

| Kind | Sort | TEXTSTRA | Text |
|---|---|---|---|
| `0x201` | 10 | 12312 | Move |
| `0x202` | 12 | 12311 | Confirmed Move |
| `0x240` | 300 | 12320 | Mission |
| `0x260..` | — | 12354.. | Command submenu (parent `0x160`): None, Commander, Admiral, General |
| `0x100` | 1000 | 12292 | Encyclopedia |
| `0x103` | 1001 | 12293 | Status |
| `0x242` | 2002 | 12336 | Retire |

The manual's character menu (p. 99) shows Move, Confirmed Move, Mission,
Command, Encyclopedia and Status in that order. Which kinds a character's
class offers in each status (`+0x3c`) is not yet traced.

## The window

`FUN_00442860` builds the "Game Menu Window" already ported for the speed
menu (`2026-09-24-game-speed-recovery.md`): STRATEGY frame tiles
10100..10107, rows laid out by `FUN_00442a80`, submenu arrows 10117/10118
(side 1) or 10128/10129. A left-button release on an item (`FUN_004424c0`)
calls the owner's vtable `+0x20` with the item's kind; a press outside closes
the menu. For a system window that is `FUN_004ac730`, which calls
`FUN_0041cdf0(kind, selection, no target, 0)`.

## Targeting

`FUN_0041cdf0` → `FUN_00436020` → `FUN_00486fb0` handles the kind. For
`0x240`, `FUN_00487c50` builds the order (team = the selection) and
`FUN_0041d5e0` → `FUN_00429320` hands it to the galaxy view: mode `+0xc0` = 2,
the order at `+0xc4`, the mouse captured, and the cursor `+0x46c` set to the
targeting cursor `+0x468`. That cursor is REBEXE.EXE cursor group 1002
(`LoadCursorA(.., 0x3ea)` in `FUN_00422ce0`'s `WM_CREATE`): 32 by 32, 8-bit,
hotspot (12, 12).

The next `WM_LBUTTONUP` (0x202) in mode 2 (`FUN_00422ce0`):

1. Finds the child window under the cursor and the object under the point.
   For a `0x240` order it asks the child's vtable `+0x68` (an object, such
   as a system on the map); move orders (`0x201`, `0x202`, `0x214`) ask
   `+0x70` instead.
2. With no object, or an object the order rejects, the order is destroyed.
3. Otherwise, when the object is one of the team's own members
   (`FUN_004f5940`), the target becomes the first system found walking up
   its parents. The target is set (vtable `+0x2c`) and `FUN_0042a320` opens
   the mission dialog, or hands the order back when no kind is legal.
4. Mode returns to 1 and the capture is released.

Command `0x15e` in mode 2 destroys the pending order and restores mode 1.

## A drag is a move

`FUN_006083c0` (the `CoolDragList` procedure) captures the mouse on a press.
A release more than 5 pixels away and outside the list posts notification
`0x29a` with the screen point; the system window forwards it to the galaxy
view (`FUN_004534f0`). `FUN_00422ce0` takes the source's selection, hit-tests
the drop point, and issues order `0x201` (Move), or `0x202` (Confirmed Move)
with Ctrl held, for window types 1, 4 and 10; type 9 issues `0x214`.

## Ported

- 7a: `crates/rebellion-render/src/game_menu.rs` draws the Game Menu Window
  for any owner, and the speed menu now uses it. Items are white, the
  highlighted item takes the faction color, and disabled items are gray
  (`FUN_004abe10`, `FUN_004aba60`). The P60 speed menu had the first two
  swapped. A submenu parent without an icon reserves 20 pixels
  (`FUN_004abf60`). port: Escape closes the menu.
- The character pop-up, the targeting cursor and the hand-off to the mission
  dialog are 7b..7d.

## Supporting decompiles

`FUN_006007b0`, `FUN_00609fc0`, `FUN_004ac5c0`, `FUN_004fcf20`,
`FUN_0051d990`, `FUN_00442590`, `FUN_00442790`, `FUN_00442380`,
`FUN_00442430`, `FUN_00442860`, `FUN_00442a80`, `FUN_004422f0`,
`FUN_00442d10`, `FUN_004424c0`, `FUN_004aab50`, `FUN_004ab560`,
`FUN_004aba60`, `FUN_004ac730`, `FUN_0041cdf0`, `FUN_00436020`,
`FUN_00486fb0`, `FUN_00487c50`, `FUN_0041d5e0`, `FUN_00429320`,
`FUN_006083c0`, `FUN_004534f0`, `FUN_00422ce0`.
