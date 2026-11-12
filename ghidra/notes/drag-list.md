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
  ships as one `JoinFleet`/`MoveShips`, its characters and special forces as
  one Move mission, its regiments and squadrons one move each (port:).
- The drag image follows the pointer for the right list and the Defenses
  list. port: a left list drag draws no image.
- A menu opened on a selected item takes the whole selection
  (`ObjectMenuState::with_team`); Encyclopedia and Status grey for more than
  one (`FUN_0051d990`). port: only Move, Confirmed Move, a ships team's
  Create Fleet and a characters team's Mission are ported for a team.
- port: the Fleet window's left list keeps one selection (its right pane's
  union of several fleets, `FUN_004a5c00`, is not ported).
- port: a Defenses window release inside its own list is no drop (there
  `+0x70` gives only its own system).
- port: Ctrl+drop (`0x202`) is ported for the Defenses window only.

## Open

- A0: the drag image and cursor while dragging (Fleet window right list,
  two ships); a drop of an expanded left-list ship onto another fleet entry
  of the same list (confirms flags 0); Shift+click range in the Fleet
  window's right list.
