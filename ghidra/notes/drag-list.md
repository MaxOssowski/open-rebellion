# The CoolDragList control: selection and drag

Read 2026-11-12 from `~/Games/rebexe-decompile/decompiled-typed/`
(`FUN_006083c0.c` read in full; the helpers spot-checked). The class is
vtable `0x0066e0f0`, created by `FUN_00607ea0`; its window procedure is
`FUN_006083c0`. The Fleet window's two lists, the System Defenses window's
list, the Missions window's lists and the Manufacturing window's facility
list are all instances.

## Flags (`+0xf4`)

| Bit | Meaning | Read in |
|---|---|---|
| 1 | a drag is under way | `FUN_006083c0` case `0x200` sets it |
| 2 | the anchor `+0xf8` is valid | set after every press |
| `0x10` | the press landed on a selected item; its click waits for the release | case `0x201`/`0x204`, case `0x202` |
| `0x10000` | single selection: the keys of a press are ignored | case `0x201`, `FUN_00609500` |
| `0x20000` | no drag | case `0x200` (`& 0x20001`) |
| `0x80000` | grid layout | `FUN_00609ae0` |
| `0x100000` | a drop must leave the list | case `0x202` |
| `0x200000` | presses go to the parent | case `0x201` |

Who sets them:

- Fleet window (`FUN_004a4b10`): neither list's flags are written, so
  multiple selection, drags, and drops inside the window (hyp: the
  allocation leaves `+0xf4` zero; `FUN_00607ea0` does not write it).
- System Defenses (`FUN_004a8790`): `0xa0000`; `FUN_004a90d0` clears
  `0x20000` on pages 1..3 and sets it on pages 4 and 5.
- Manufacturing (`FUN_00455060`, list `+0x174`): `0xa0000`, no drag.
- Missions (`FUN_0049f540`): mission list `0x20000`, member list `0xa0000`.

## A press (`WM_LBUTTONDOWN` `0x201`, `WM_RBUTTONDOWN` `0x204`)

1. A right press while a drag runs is ignored.
2. Posts `0x40e` to the parent; the item under the point is found from the
   point in content pixels (`FUN_00609d80`, `FUN_00609d10`).
3. A left press captures the mouse (`FUN_00601080`) and keeps the press
   point (`+0xfc`).
4. With `0x10000`, the keys are dropped.
5. The item is already selected: flag `0x10`, nothing changes.
6. Otherwise, with `MK_CONTROL`: the item's bit 0 toggles (no item: nothing),
   `0x29b` is posted. With `MK_SHIFT`: `FUN_00609590(anchor, point)` sets bit
   0 on every item whose cell (`+0x28`/`+0x2c`, `+0xcc` by `+0xd0`) meets the
   rectangle between the two points, edges included, without clearing the
   others. Neither: `FUN_006094b0` clears every item, the item's bit 0 is set
   (no item: only the clear), `0x29b` is posted.
7. The anchor becomes this press's point (flag 2).

## Moving (`WM_MOUSEMOVE` `0x200`)

With the capture, the left button down and neither `1` nor `0x20000`, the
first move starts the drag (flag 1): `FUN_0060dbb0` copies the list's drag
bitmap `+0x110`, list-sized, into which `FUN_00609960` blits each selected
item's image (the selected image, `FUN_0060bd00`) at its visible place; the
text is drawn later by `WM_PAINT`, so it is not in it. `FUN_0060dce0`
starts it at the press point (or the first selected item's corner when the
press lay outside the list) and `FUN_0060df00` moves it with the pointer on
the frame window, child clipping off (`FUN_0060dce0`'s `& ~WS_CLIPCHILDREN`).
The cursor stays `+0xb0` unless `+0xc8` is set (unset here).

## Release (`WM_LBUTTONUP` `0x202`)

- During a drag: if the release lies more than `sqrt(24)` pixels from the
  press (squared distance above `0x18`) and outside the client rect, or
  anywhere when `0x100000` is clear, `0x29a` goes to the parent with the
  cursor position, and flag `0x10` is cleared.
- Flag `0x10` still set: the held click applies at the release point, with
  the same Ctrl / Shift / plain rules (anchor = the press point).
- The capture is released; flag `0x10` cleared.

`0x29a` reaches the galaxy view (`FUN_00422ce0`), which reads the source
window's selection (slot `+0x58`) and moves it against the drop window
(`move-order.md`, "A drag is a move").

## Port (2026-11-12)

- `crates/rebellion-render/src/drag_list.rs` (`ListSelection`) holds these
  rules; the Fleet window's right list and the Defenses window's list use
  them. The selection is drawn with each item's own selected image (frame
  10420, Fleet window; the side frame 10578/10579 and the side text colour,
  Defenses window).
- A drop carries the whole selection: the Defenses window's as one team
  move (`issue_team_move`, split as `FUN_0053c4b0`); the Fleet window's
  ships as one `JoinFleet`/`MoveShips`, its fleets as one fleet move, and
  its regiments, squadrons, characters and special forces as one team move
  (`fleet_drop_order`, 2026-11-13), so one refusal refuses the order.
- `FUN_00422ce0`'s `0x29a` handler reads `GetAsyncKeyState(VK_CONTROL)`
  before its switch on the source window's type, so Ctrl makes the drop
  `0x202` for types 4 (Fleet) and 10 (Defenses) alike; both are ported.
- The drag image follows the pointer for every list: the Fleet window's
  right list, its left list (2026-11-13: the pressed entry, which the
  press selects, with frame 10401 and its mini and marks, no text) and the
  Defenses list.
- A menu opened on a selected item takes the whole selection
  (`ObjectMenuState::with_team`); Encyclopedia and Status grey for more than
  one (`FUN_0051d990`). `FUN_0051d990` lists the intersection of the
  members' order lists (class slot `+0x3c`) and enables each order whose
  `+0x18` passes with the whole selection as its team. For a team the port
  enables Move and Confirmed Move (a fleets team too), a ships team's
  Create Fleet, a members team's Mission and Retire (`0x242`, each member's
  check), and Scrap (`0x200`, one command per member, `FUN_005201c0`):
  one confirmation lists every member (`team_confirmation`), one refusal
  refuses all. The Command orders refuse more than one object
  (`FUN_00580db0`). port: Planetary Bombardment and Assault for a team of
  fleets (`FUN_00553770` then `FUN_00582440` / `FUN_00581ae0` over the
  team's fleets), Rename and Abort stay disabled for a team, and rows
  outside the intersection are drawn disabled instead of left out.
- The Fleet window's left list takes several entries (2026-11-13).
  `FUN_004a5c00` (read in full) then: counts the selected items (`+0x3c`
  bit 0); when one has item flag `+0x68` bit 8 (the viewer's side: without
  one the picture takes the other side's art, `0x32 - ...`) it deselects
  every item without it; with two or more it clears `+0x14c` bit 8 (so
  Capital Ships stays a tab), names nothing (`FUN_00601aa0(0)`) and draws
  the fleet picture with the last selected item's en route and damage
  flags; it empties the right list (`FUN_004a6980`) and has each selected
  item add its contents for the tab (`FUN_004a6be0`: a fleet recurses into
  its ships, a ship adds itself on `0x66`, its regiments `FUN_005039d0`
  and their slots `+0x23c`/`+0x26c` on `0x67`, its squadrons
  `FUN_00504c40` and `+0x240`/`+0x270` on `0x68`, its personnel
  `FUN_00536da0` otherwise), summing the counts into `+0x198`/`+0x19c`.
  `FUN_004a6980` lights `0x67..0x69` from the selected items' `+0x54`
  flags, or'ed. Ported as `selection_items`, `selection_tab_enabled`,
  `selection_counts` and the press rule in `draw_fleet_windows`; a drop or
  menu from a selected entry takes every selected entry. port: a left
  click applies the press and its release together (the original selects
  on the press); ships of several fleets drop as one order per fleet.
- port: a Defenses window release inside its own list is no drop (there
  `+0x70` gives only its own system).

## A0 2026-11-13-drag (read 2026-11-13)

Captures and coordinates in `.artifacts/a0-vm/2026-11-13-drag/NOTES.txt`;
the gates `fleet-window.mjs` (`a0-drag-right`, `a0-drag-left`,
`ship-spare`) and `sector-quadrants.mjs` (`defenses-a0-regiments`) compare
the port with them.

- The right list's frame is 10420 at the item's (1, 1): 114 by 37 (x 407..520
  in 01; NOTES' 406 is a typo, x 406 is black), its bottom-left pixel the
  blue key. The drag image (03) is the same two outlines and the items' art,
  no names, at the press offset (58, 18); the left list's frame shows
  through it. A0's pointer covers x 336..362, y 138..164 there.
- The left list's selection is 10401's outline (70 by 47 at the entry's
  (3, 0)). A press on a child row frames it while the button is held (25):
  the press selects. Its drag image is that outline and the row's art, no
  name (25, 26). The outline stays over a long name (25: x 325 red through
  the name's rows), so the port draws a ship row's frame after its name.
- The Defenses list's frame is 10578 (61 by 25, bottom-left keyed) on the
  image; the name turns pure red (255, 0, 0), no frame round it (31). The
  drag image is the framed images on the 70-pixel pitch, no names (32).
- 22-23 (nothing happens) is not the expanded left list. The drop point
  (local (51, 253)) is in the left list on no entry, so `+0x70`
  (`FUN_004a3130`, read in the image at `0x4a3240..0x4a3300`: the left
  list `+0x160` hit-tested with `FUN_00609d80`/`FUN_00609d10`) gives the
  subject, the system. A ship's Move to its own system goes into the side's
  bit-1 spare there (`FUN_00552300` -> `FUN_005097d0`,
  `ship-deployment-fleet.md`), which stays the spare once it holds ships
  (`FUN_0050bc60`). Fleet 6 was made by that same drop in 20-21, so it is
  the spare, and its own ship's move puts it where it already is. 05-06 and
  20-21 met no numbered spare and made one. Port:
  `fleet_join::move_ships_to_system` joins the spare, or makes the fleet
  the spare.
- `+0x58` (`FUN_004a2c80`) with 0 reads the left list's selection when
  `+0x14c` bit 1 is clear, else the right list's. Bit 1 is set by a menu
  opened from the right list (`FUN_004a2c40`, list id 200) and cleared by
  one from the left (0xc9), and set or cleared by an outside select
  (`FUN_004a73c0`). So a drag carries the list whose menu opened last, not
  necessarily the dragged list. port: a drag carries its own list's
  selection (hyp: the captures 08..12 and 25..27 cannot tell them apart).

## Open

- A0: a drop of an expanded left-list ship onto another fleet entry of the
  same list (confirms flags 0).
- The `+0x14c` bit 1 source rule above, against a capture that drags from
  one list after a menu from the other.
