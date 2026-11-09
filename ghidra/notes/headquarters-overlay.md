# The headquarters overlay, its menu and its Status (read 2026-11-04)

The manual (p. 134, "Moving Alliance Headquarters", Fig. 3.82) says the
HQ icon can be dragged onto a new system, or right-clicked for Move and
Confirmed Move.

## The overlay

- `FUN_0045bbb0` adds the overlay when the viewer's record of the system
  holds a headquarters (state bit 8, `FUN_00526cf0(system, 3)`: families
  `0x20..0x21`). It is created with `FUN_00442130(0x10000, id)`, where the
  id is the HQ object's (`FUN_0052bed0`). The bitmap is STRATEGY 904
  (`0x388`), the rect is the planet's, `+0x3c` = 6, and `+0x54` = 1.
- The overlay list `+0x174` is a `FUN_0060a790(1)` list. Its comparator
  `Class_0066e148_vt4`, in mode 1, orders items by `+0xc` ascending, and
  `FUN_005f59f0` inserts in that order. The HQ overlay's `0x10000` is
  below every quadrant icon's `sys_id | 0x40000` and above, so it comes
  first.
- `FUN_0045cc10` returns the first item whose rect holds the point. For
  `+0xc == 0x10000` with a bitmap, it also requires `FUN_005fca00` to find
  an opaque pixel at the offset, with x and y both above 0. So the HQ
  takes a press wherever 904 is opaque, even under a quadrant icon. Its
  transparent pixels fall through to the icons.
- `sector-window-hit-test.md` calls this overlay the "fleet-status
  overlay". It is the headquarters overlay.

## Press, double click and menu

- A press selects it like an icon (`FUN_0045b1b0`). The entry holds the HQ
  object's id and kind `0x10000 >> 16` = 1.
- A double click finds it first and calls `FUN_0045aac0`. Kind 1 has no
  case there, so the window id is 0 and nothing opens.
- The right release (`FUN_004ac5c0` → `FUN_0051d990`) resolves the id to
  the HQ object and calls its slot `+0x3c`. The HQ vtable is
  `0x0065f280`, and that slot is `FUN_00526860` → `FUN_00526c00`.
  `FUN_00526c00` builds `FUN_004f2a10`'s empty object list plus `0x201`
  Move, `0x202` Confirmed Move and `0x204`. `0x204` has no STRATEGY
  record, so it never becomes a row. There is no Scrap.
- `FUN_0051d990` adds Encyclopedia, and Status, which is enabled because
  the object is no system.
- So the menu is Move, Confirmed Move, Encyclopedia, Status.

## Status

`FUN_00443130` gives families `0x20..0x27` to `FUN_00444e20`, Defense
Facility Status (TEXTSTRA 34403). The weapons and shield rows show only for
`0x22..0x27`. The HQ's rows are:
- Location;
- Status;
- Maintenance Cost;
- Bombardment Defense Strength. This is vtable `+0x1c0` (`0x558050` →
  `FUN_00520b60`, class `+0x58`, ALLFACSD's last word, 9) minus `+0x1c4`
  (`0x6158b0`, 0).

## Quadrant icons

The HQ object lights no quadrant icon:
- the facilities icon `FUN_0045cdc0` counts `FUN_0053b6e0`'s families
  `0x28..0x30`;
- the defenses icon `FUN_0045ce80` counts `FUN_00526fd0`'s `0x22..0x28`.

## Ported (2026-11-04)

- `sector_window.rs` tests the overlay before the icons
  (`headquarters_hit`, `is_resource_hit` on 904 with x and y above 0). A
  press selects it (`SelectedIcon::quadrant` = `None`), and the menu opens
  for `MenuObject::Headquarters`. A double click on it opens no icon's
  window.
- `object_menu.rs` gives `Headquarters` the rows Move, Confirmed Move,
  Encyclopedia and Status. The two moves are enabled since 2026-11-04
  (`headquarters-move.md`).
- `status_rows.rs` shows Defense Facility Status for a family `0x20..0x27`
  manufacturing-facility key.
- `quadrant_icons.rs` leaves the HQ out of the facilities count.

Open:
- the selected overlay's bitmap. `FUN_0060bd20(0x388, 0, ..)` sets no
  second bitmap. A0 headquarters/01 shows the part of the selected
  overlay that the menu leaves uncovered unchanged, and the port keeps
  drawing 904;
- (settled 2026-11-09) a planet has no double click of its own:
  `FUN_004593e0` case `0x203` hit-tests the overlay list only
  (`FUN_0045cc10`) and does nothing when no overlay takes the point (A0:
  Adega, Flax, Akrit'tar and Yavin open nothing).
