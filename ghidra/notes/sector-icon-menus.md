---
title: "Sector Window Icons: Selection, Pop-up Menus and Drags"
description: "How a press on a sector window's quadrant icon selects it (FUN_004593e0, FUN_0045b1b0), which orders its right-click menu lists per kind (FUN_00507290, FUN_0050f2e0), and what each order acts on (FUN_00512700)"
category: "ghidra"
created: 2026-10-06
updated: 2026-11-11
tags: [sector-window, quadrant-icons, object-popup-menu, orders]
---

# Sector window icons: selection, menus and drags

Recovered 2026-10-06 with Ghidra 12.1.3 headless (read-only project). The
icons themselves, their show rules and the windows a double click opens are
in `sector-quadrants.md`; the menu machinery is in `object-popup-menu.md`.
The manual agrees: the mission icon's right-click menu offers Encyclopedia,
Status and Abort (p. 109, Fig. 3.50).

## Selection

The sector window's procedure `FUN_004593e0` (vtable `0x00659f18`, slot 5)
hit-tests every mouse message against the overlay list `+0x174` with
`FUN_0045cc10`: the first item whose rect holds the point, the fleet-status
overlay (flag `0x10000`) only where its bitmap pixel is opaque. Planet items
(list `+0x164`) are not in that list.

- `WM_LBUTTONDOWN` (`0x201`) and `WM_RBUTTONDOWN` (`0x204`): with no item
  under the point the procedure returns at once, so **the selection is kept**.
  Otherwise, for a shown item (`+0x3c` bit 2) and without Ctrl (`MK_CONTROL`,
  bit 3), `FUN_0045afc0(this, 0)` clears the selection and `FUN_0045b1b0`
  selects the item. A right press then goes on to the base handler
  (`FUN_004ac3a0`).
- `FUN_0045b1b0` sets the item's `+0x3c` bit 0, repaints it, and appends a
  selection entry to the window's list `+0x184` (`FUN_004f57b0`). The entry
  (`FUN_004f5b10`, vtable `0x0065d1f0`) holds the item's object id (`+0x68`,
  `FUN_0042d170`), which is the system, and the item's kind
  (`flag >> 16`) at `+0x1c`. `FUN_004f3220`, called first, only resolves the
  id to a system; its result is dropped.
- `WM_MOUSEMOVE` (`0x200`) with the button held drags the selection
  (`FUN_0060dc80`..`FUN_0060dce0`); `WM_LBUTTONUP` far enough away posts
  `0x29a` to the galaxy view (`move-order.md`, "A drag is a move").
- Slot 22 (`+0x58`, `FUN_0045b3b0`) copies `+0x184` out: it is the window's
  selection for menus and drags.

## The menu

Slot 7 (`+0x1c`, the `WM_RBUTTONUP` handler) is `FUN_004ac5c0`, the system
window's: it copies the selection and lists its orders through
`FUN_0051d990`. That asks each selected object's class (`+0x3c`) for its
orders, **passing the entry's kind**. The system class (vtable `0x0065e640`)
answers with `FUN_00507290` → `FUN_0050f2e0(kind & ~1)`, from static lists
`FUN_0050f0b0` builds once:

| Icon | Kind | Orders |
|---|---|---|
| top-left, facilities | 4 | `0x200`, `0x214`, `0x216` |
| bottom-left, defenses | 8 | `0x201`, `0x202`, `0x200` |
| top-right, fleets | `0x10` | `0x201`, `0x202`, `0x200`, `0x220..0x223`, `0x234` |
| bottom-right, missions | `0x40` | `0x250` |

Any other kind gives no list. `FUN_0051d990` adds Encyclopedia (`0x100`),
enabled for one selected object, and Status (`0x103`), enabled only when that
object is not a system (`0x90..0x97`): **an icon's Status is always
disabled.**

The STRATEGY `RT_RCDATA` records name the new kinds (word 2 the sort key,
word 5 the TEXTSTRA string):

| Kind | Sort | TEXTSTRA | Text |
|---|---|---|---|
| `0x214` | 120 | 12290 | Destination |
| `0x216` | 1002 | 12294 | Reserved (word 10: bitmap 11902, the check mark) |
| `0x250` | 2003 | 12362 | Abort |

So the menus read, in sort order:

- facilities: Destination, Encyclopedia, Status, Reserved, Scrap;
- defenses: Move, Confirmed Move, Encyclopedia, Status, Scrap;
- fleets: Move, Confirmed Move, Planetary Bombardment (submenu), Planetary
  Assault, Encyclopedia, Status, Scrap;
- missions: Encyclopedia, Status, Abort.

## What each order acts on

An order built on an icon's entry resolves its team through the system
class's slot `+0x2c`, `FUN_00512700(system, order, kind, out)`. The order's
side must be 1 or 2. Every list is filtered to that side by `FUN_00553350`
(the object's side bits `+0x24 >> 6 & 3`):

- kind 4: Scrap gives the manufacturing and production facilities
  (`FUN_0053b6e0`); Destination and Reserved the family `0xa0..0xaf` objects
  (`FUN_0052c170`; untraced: what they are);
- kind 8: Scrap gives regiments, fighter squadrons and defense facilities;
  Move and Confirmed Move give regiments, fighter squadrons, then characters
  and special forces; `0x240` and `0x242` give the characters (not listed
  for this kind);
- kind `0x10`: every order gives the system's fleets (`FUN_004ffe70`);
- kind `0x40`: the characters and special forces on a visible mission
  (`+0x78` bit 8 clear).

Move then splits the members into sub-orders (`move-order.md`, "Expansion
into sub-orders").

Kind `0x40` does not look at the order's kind: any order on the mission
icon, a drag's Move included, gets the mission members.

## The facility icon's Reserved (read 2026-11-10)

`0x216` is order class `0x006612a8` (`FUN_0051f4b0` registers
`FUN_00539090` -> `FUN_00539040`). Its slot `+0x48` (`Class_006612a8_vt48`)
creates one `Event216` (vtable `0x00669aa8`) per team member through
`+0x4c` (`FUN_005201c0` -> `FUN_005535b0`), so on an icon there is one per
manager of the side.

- Enabled (`+0x18`, `FUN_0051fe20`): each event's `+0x18`
  (`Event216_vt18`) asks its manager's `+0x1d8` (`ManuMgr_vt1d8`: the object
  check `FUN_004f9860`), and `FUN_00553770` refuses an empty team (`0x16`)
  or mixed sides (`0x14`).
- Checked (`+0x14`, `FUN_0051fd30`, read from the image at
  `0x0051fd30..0x0051fe14`): `FUN_00553720` walks the events, noting whether
  some answer yes and some no (`Event216_vt14` -> `ManuMgr_vt1d4`, the
  manager's Reserved). With the order's `+0x54` non-zero the row is checked
  when all answer yes; Reserved's `+0x54` is `FUN_006158b0`, which returns 0,
  so **the row is checked when any manager is Reserved**.
- Chosen (`+0x50`, `Class_006612a8_vt50`): each event's value `+0x50` is
  `+0x14 == 0`, the opposite of the mark; `Event216_Fire` hands it to the
  manager's `+0x1fc` (`ManuMgr_vt1fc` -> `+0x1dc`, then
  `ManuMgr_SetReserved`). So a marked row clears every manager and an
  unmarked one reserves them all.

## Drags by kind (read 2026-11-10)

`FUN_00422ce0`'s `0x29a` handler, window type 1 (`move-order.md`, "A drag
is a move"), walks the selection: an entry whose kind (`+0x1c`) has bit 4
(the facility icon) is copied into a new entry (`FUN_004f5b10(id, 4)`) for
a `0x214` order against the drop window's `+0x68`; every other entry goes
into `0x201`, or `0x202` with Ctrl, against `+0x70`. So:

- facilities: Destination for the system's managers of the side, against
  the object under the point. `FUN_00537180` refuses a manager's
  destination that is no system (`1`/`0x25`); `FUN_00487cc0` has no case for
  `0x214`, so nothing asks and Ctrl changes nothing;
- defenses: Move or Confirmed Move of the regiments, squadrons, characters
  and special forces (`FUN_00512700` kind 8), split into one `0x241` order
  for the characters and special forces and one `0x204` group order for the
  rest (`FUN_0053c810`, `FUN_0053ca50`);
- fleets: Move or Confirmed Move of the system's fleets;
- missions: Move or Confirmed Move of the members on visible missions
  (kind `0x40` ignores the order kind). It is refused at validation
  (read 2026-11-11, below): every member has a mission. Not run in A0.

## The defenses icon's Move (read 2026-11-11)

`FUN_00512700` kind 8 for `0x201`/`0x202` (read in full) walks the
system's children three times, each through `FUN_00553350` with the
order's side: `FUN_00504c40` (families `0x10..0x13`, regiments),
`FUN_005039d0` (`0x1c..0x1f`, squadrons) and `FUN_00536da0` (`0x30..0x3f`,
characters and special forces). The walk has no mission filter, so a
member on a mission at the system, or on its way there, is in the team.
Defense facilities belong to Scrap's team only (`FUN_00527050`).

`FUN_0053c4b0` (read in full) resolves the team and refuses an empty one
(`1`/`0x16`). It then calls `FUN_0053c810` until it builds no more orders:
`FUN_0053cbc0` takes the built (`Completed`) members of families
`0x30..0x3f` that share the first one's `+0xc` value into one `0x241`.
At an icon they all share the system, so there is one `0x241`. Last,
`FUN_0053ca50` builds one `0x204` for the rest, only when any are left
(`FUN_005f50e0`).

- Enabled (`FUN_0053c100`): `FUN_0053c4b0`, then each sub-order's `+0x18`.
  - The `0x241`'s `+0x18` is `FUN_0051fe20`. It builds command `0x250`
    (`FUN_004f4a00`, vtable `0x00661e28`). `FUN_00553770` then runs the
    command's `+0x18` (`Event250_vt18`, `FUN_0054d1a0` -> `FUN_005429e0`),
    which asks each member's `+0x1c8` (`FUN_0054c110`). That check refuses
    a member that is not the side's, travelling or on a mission. For
    `0x241` it does not refuse an unrecruited member or a prisoner
    (`object-popup-menu.md`). `FUN_0054bf00` needs one place.
  - The `0x204`'s `+0x18` checks each member's move command
    (`FUN_00578c00`).
- Validated (`FUN_0053c1a0`): each sub-order's `+0x1c`. For `0x241` this
  is `FUN_0051ff30`: its `+0x18` again, then `FUN_004f4b60` (the target's
  side, `1`/`0x28`), then each command's `+0x1c` (`FUN_00553960`).
  `Event250_vt1c` (`FUN_0054d280`, vtable `0x00661e28` slot `+0x1c`, read
  from the image) calls `FUN_00542b60`. That reaches `FUN_0054c200` ->
  `FUN_00522b30`, which refuses a member that already has a mission. For
  `0x204` it is `FUN_0053d430`'s `1`/`0x28` rule and the per-member
  commands. A refusal goes to `FUN_00487c90`, and nothing moves.
- Asked (`FUN_00487cc0`, read from the image at `0x00487cc0..0x00487e13`):
  the result word `[esp+0x10]` starts at 0.
  - `0x202` always asks.
  - `0x201` asks only when the first team id's container (`+0x1c`) is a
    system (`0x90..0x97`) that is blockaded and of the same side.
  - The icon's entry is the system. Its container is a sector, so **an
    icon's Move never asks**.
  - `FUN_0049a350` reads the same first member, so a Confirmed Move has
    no evacuation warning (text `0x7057`, pictures 1018/1019).
- Lines (`FUN_0053c2e0`): the sub-orders in list order, so the `0x241`'s
  members (`FUN_004f4850`, at the mission's speed `FUN_00548370`) come
  before the `0x204`'s (`FUN_0053d200`).

**The missions icon's drop.** Kind `0x40` gives the side's members on
visible missions whatever the order. Every member has a mission, so the
`0x241` fails at validation:
- its `+0x18` through `FUN_0054c110`, on a mission (`+0x78` bit 7);
- its command's `+0x1c` through `FUN_00522b30`, which needs an empty
  mission key `+0x68`.

An empty team is `1`/`0x16`. So a drop on a window that names a target
always reaches `FUN_00487c90`, with or without Ctrl, and nothing moves. A
drop with no target is dropped silently (`FUN_004ece60`).

Manual: no passage names the defenses or missions icon's Move or drag
(searched "Defense icon", "Defenses icon", "drag the"). p. 109 (Fig. 3.50)
covers the mission icon's menu only.

## Ported

- `sector_window.rs`: each open window keeps its icon selection. A left or
  right press on a shown icon sets it; a press anywhere else keeps it. A
  right release over the window opens the object menu for it
  (`SectorWindowAction::OpenObjectMenu`).
- `object_menu.rs`: `MenuObject::SystemIcon` lists the rows above.
  Encyclopedia is enabled, and so are the fleet icon's Move and Confirmed
  Move when the system has fleets of the player's side and every one may
  move (`movement::fleets_move_enabled`: `FUN_0053c4b0` refuses an empty
  team with `1`/`0x16`, then each sub-order's `+0x18`).
- The fleet icon's Move targets with the system's fleets of the player's
  side as its team (`movement::system_side_fleets`). `issue_fleet_move`
  (`main.rs`) validates every member (`FUN_0053c1a0`: one refusal refuses
  all), asks once when any member's `FUN_00487cc0` rule asks, listing a line
  per fleet, or departs them all; onto a fleet, each joins it.
- The missions icon's Abort (`0x250`, order `FUN_00580580`, vtable
  `0x669b70`) is enabled when the player has a visible mission in the
  system (`quadrant_icons::own_visible_missions`, the team of
  `FUN_00512700` kind `0x40` filtered to the side): its `+0x18`
  (`FUN_0051fe20` -> `FUN_00553770`) refuses only an empty team (`0x16`) or
  mixed sides (`0x14`) and checks no phase. Its validator `+0x1c`
  (`FUN_0051ff30`) adds `FUN_00553960`: each event `0x261`'s `+0x1c`
  (`FUN_0054e240`) -> the mission's `+0x1c4` (`FUN_00521700`) -> `+0x1c0`
  (`FUN_00521600`). A refused Abort goes to `FUN_00487c90` and asks
  nothing (A0 personnel-states/37, 2026-11-03). A passing one asks with TEXTSTRA
  `0x7052` and picture `0x40c`/`0x40d` (`FUN_0049a350` case `0x250`); the
  answer builds event `0x261` (`FUN_0054e090`, vtable `0x661f20`), which
  checks each mission again through slot `+0x1c0` (`FUN_00521600`: the
  side's, refused in phase 4 with `0x40` and in any phase but 8 with
  `0x41`, `MissionState::abort_allowed`) and fires `+0x1d4`
  (`FUN_005233a0` -> `+0x1dc(2)`), ending it with code 2
  (`MissionState::abort`).
- A left press on a shown icon keeps its canvas point; a left release more
  than four pixels away in x or y (`FUN_004593e0` case `0x202`) drops the
  selection (`SectorWindowAction::DropSelection`), a Confirmed Move with
  Ctrl held. The fleet icon's drop moves its fleets against what the window
  under the point gives (`+0x70`, `targeting::release_destination`).
- The facility icon's Reserved (2026-11-10) is enabled when the system
  has managers of the player's (port: a manufacturing facility of the
  side, `quadrant_icons::facility_icon_has_managers`, as for Destination),
  checked while any area is Reserved (`ManufacturingState::any_reserved`),
  and sets every area to the opposite of the mark
  (`ManufacturingState::toggle_system_reserved`).
- The facility icon's drop (2026-11-10) sets every area's Destination to
  the system `+0x68` names (`targeting::destination_drop_target`), with no
  question; a point naming an object below the system sets nothing. port:
  `+0x68` is read through the Mission release's hit test, so a planet's
  squadron, an unfinished unit or a Missions window's mission answers its
  system where the original refuses; the refusal's advisor reaction
  (`FUN_00487c90`) is not played.
- The defenses icon's Move and Confirmed Move (2026-11-11):
  - Its team is `quadrant_icons::defenses_move_team`.
  - Enabled: `defenses_move_enabled` (`main.rs`).
    `MissionState::move_order_enabled` stands for the `0x241`'s `+0x18`.
    `regiment_move_enabled` and `squadrons::move_enabled` stand for the
    `0x204`'s.
  - The menu starts targeting (`TargetOrder::DefensesMove`). The release
    and the drop call `issue_defenses_move`, a Confirmed Move with Ctrl.
  - Any refusal refuses the whole order.
  - A Move asks nothing. It sends one Move mission with every character
    and special force (`PanelAction::MoveMembers`), and a Move for each
    regiment and each squadron.
  - A Confirmed Move asks once, with a line per member: the members first
    at the mission's speed (`missions::team_move_days`), then the
    regiments and squadrons. The checkmark sends the same orders.
  - port: each regiment, and each squadron class with all its squadrons,
    runs its own move check. For a group of one kind this holds
    `FUN_0053d430`'s group rule. A mixed group meets the squadrons'
    refusal of another side's planet. The room of a fleet target is
    checked per kind, not for the whole group.
- The missions icon's drop on a target (2026-11-11) is refused. A0
  2026-11-13-defenses-missions-drop/24..30 (Kubindi's missions icon
  dropped on Jerijador, the Alliance) shows no window, no change and no
  Message Index entry, as `FUN_00487c90` files no message. Since
  2026-11-13 every refused order of the interface reaches the side's
  advisor (`MessageLog::refuse` -> `AdvisorSchedule::refused`, slot
  `+0xc`); the port's line stays off every rail, so no index lists it.
  hyp: most refusals' two status words are not read and take the
  generic reaction; a character's or special force's Move to another
  side's system sends `1`/`0x28`.
- The drag image is not drawn, and Ctrl's multiple selection is not
  ported.
- The fleet icon's Move never asks (2026-11-11). By `FUN_00487cc0`
  above, an icon's `0x201` reads the system as its first team id
  (`FleetMoveContext::icon`, `TargetOrder::FleetMove { icon }`); only its
  Confirmed Move asks. One fleet's own Move keeps `fleet_move_confirms`.
- The character menu's Move and Confirmed Move (2026-11-11) take the
  `0x241`'s `+0x18` (`MissionState::move_order_enabled`, `FUN_0054c110`):
  they are grey for a character on a mission or travelling. A special
  force's (`OrderGates::special_force_move`) take the same rule through
  the unit's slot `+0x1c8` (`FUN_00533ce0`, over `FUN_004f9860`). (The selected icon's second
  bitmap is drawn: `sector-quadrants.mjs` checks all 20 bitmaps of
  `FUN_0045ca80`'s matrix, each also found exactly in A0, 2026-11-10.)

## Supporting decompiles

`FUN_004593e0`, `FUN_0045cc10`, `FUN_0045b1b0`, `FUN_0045afc0`,
`FUN_004f5b10`, `FUN_0042d170`, `FUN_00442130`, `FUN_004f3220`,
`FUN_0045b3b0`, `FUN_004ac3a0`, `FUN_004ac5c0`, `FUN_0051d990`,
`FUN_00507290`, `FUN_0050f0b0`, `FUN_0050f2e0`, `FUN_00512700`,
`FUN_00553350`, `FUN_0052c170`, `FUN_0053efd0`, `FUN_00539040`,
`FUN_0051fd30`, `FUN_00553720`, `FUN_006158b0`, `FUN_005201c0`,
`FUN_005535b0`, `FUN_0057ed20`, `FUN_0057eef0`, `FUN_0057efb0`,
`FUN_0057f150`, `FUN_00539180`, `FUN_00528410`, `FUN_00528430`,
`FUN_00529c30`, `FUN_00422ce0`, `FUN_00504c40`, `FUN_005039d0`,
`FUN_00536da0`, `FUN_0053c4b0`, `FUN_0053c810`, `FUN_0053cbc0`,
`FUN_0053ca50`, `FUN_0053c100`, `FUN_0053d430`, `FUN_0051fe20`,
`FUN_00553770`, `FUN_0051ff30`,
`FUN_00553960`, `FUN_0054d1a0`, `FUN_0054d280`, `FUN_005429e0`,
`FUN_00542b60`, `FUN_0054c110`, `FUN_00487cc0`, `FUN_0049a350`,
`FUN_0053c2e0`, `FUN_004f4850`.
