---
title: "Sector Window Icons: Selection, Pop-up Menus and Drags"
description: "How a press on a sector window's quadrant icon selects it (FUN_004593e0, FUN_0045b1b0), which orders its right-click menu lists per kind (FUN_00507290, FUN_0050f2e0), and what each order acts on (FUN_00512700)"
category: "ghidra"
created: 2026-10-06
updated: 2026-10-06
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

## Ported

- `sector_window.rs`: each open window keeps its icon selection. A left or
  right press on a shown icon sets it; a press anywhere else keeps it. A
  right release over the window opens the object menu for it
  (`SectorWindowAction::OpenObjectMenu`).
- `object_menu.rs`: `MenuObject::SystemIcon` lists the rows above; only
  Encyclopedia is enabled. port: the orders stay disabled until their
  system-wide forms are ported; the selected icon's second bitmap is not
  drawn; Ctrl's multiple selection is not ported.
- Not yet: the icon drags.

## Supporting decompiles

`FUN_004593e0`, `FUN_0045cc10`, `FUN_0045b1b0`, `FUN_0045afc0`,
`FUN_004f5b10`, `FUN_0042d170`, `FUN_00442130`, `FUN_004f3220`,
`FUN_0045b3b0`, `FUN_004ac3a0`, `FUN_004ac5c0`, `FUN_0051d990`,
`FUN_00507290`, `FUN_0050f0b0`, `FUN_0050f2e0`, `FUN_00512700`,
`FUN_00553350`, `FUN_0052c170`, `FUN_0053efd0`.
