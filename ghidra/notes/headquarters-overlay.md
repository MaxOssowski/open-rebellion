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

## The Empire's view (read 2026-11-16)

`FUN_0045bbb0` takes the system from the viewer's table (`FUN_004f3220` on
`+0x194` -> `+0x9c`) and walks that record's families `0x20..0x22`
(`FUN_00526cf0(record, 3)`, `FUN_00513120`); any child sets bit 8 and the
overlay. It does not ask whether the copy is live: a copy the Empire froze
while the headquarters stood there keeps 904 until the Empire sees the
system again. A0 2026-11-16-edited-saves (Empire games; their saves carry a
Death Star edit, the headquarters is not edited): x-39 (Tokmia under the
Empire's blockade, day 45) and x-29 (Duros, day 527) show 904 to the Empire.
The edit does not enter the rule: it only brought the Empire's fleet there.

Ported 2026-11-16: `sector_window::shows_alliance_headquarters` reads the
player's copy (`viewed_contents::system_sight`): live, the Alliance-held
system marked `is_headquarters`; frozen, a copied family `0x20..0x21`
facility. sector-quadrants.mjs `empire/headquarters-frozen`
(`Scenario::QuadrantsHeadquartersFrozen`) compares 904 over the planet, and
904 in x-29 and x-39, every opaque pixel.

## Can the Empire see 904 (traced 2026-11-18, OBJ-13-C001)

Yes. There is no "HQ known" bit and no game-type test: 904 follows the
Empire's copy of the HQ object (`per-side-object-views.md`).

- `FUN_0045bbb0` walks the viewer's copy of the system (`FUN_004f3220`)
  with `FUN_00526cf0(record, 3)` -> `FUN_00513050`: families `0x20..0x22`,
  mode 3. `FUN_005131d0` -> `FUN_004f6b90` mode 3 is the child copy's
  `+0x50` bit 6 (existing). Any such child in the copy's list sets bit 8.
- The HQ vtable `0x0065f280` keeps the default seen rules: `+0x8c`
  `FUN_004f7bf0`, `+0x90` `FUN_004f7c70`, and `+0x4c` is `FUN_0040f340`
  (returns 1). So the HQ's Empire seen bit (`+0x50` `0x100`) is its
  system's, unless the HQ's own `+0x50` bit `0x20` (EnrouteActive) is set.
  The system's Empire bit is `FUN_0050ac80`: an Imperial fleet in it, or
  the Empire holds it (never, while the HQ stands: `facility-ownership.md`).
- `FUN_004f9a60` makes the HQ's Empire copy live with that bit, completed
  (`0x4`) and not en route (`0x10`), and rebuilds it from the master.
- Setup: `FUN_005136d0` stage `0x18` -> `FUN_0051b940` -> `FUN_0051b990`
  on both copies pushes the sectors (`FUN_004f8340`), their systems
  (`FUN_0050c8a0`, `0x90..0x98`) and the systems' `0xa0..0xaf` children
  (`FUN_00529a40`) only, then `Event319_Handle(10, ..)` on Yavin
  (`0x92000121`, at `0x51bba6`). No facility, so no HQ, enters the Empire's
  copies at setup, except whatever stands on Yavin. This is why A0
  2026-11-16-empire-hunt h-43 lists the six Yavin leaders to the Empire
  but not Mon Mothma (on the HQ system, manual p. 37).
- Later pushes that carry family `0x20` (`Event319_Handle` table): levels
  8 (Reconnaissance), 10 (Espionage), 11 (bombardment), 12 (core-system
  notification).

So the Empire sees 904 from the moment an Imperial fleet is in the HQ's
system (not in transit), and keeps it in a copy frozen after a fleet's
visit or a push. A0: x-39 (Tokmia, the HQ of the HQ-victory game EHQ2,
under blockade, day 45) and x-29 (Duros, frozen, day 527).

The empire-hunt captures h-17, h-44, h-48 and h-76 show Yavin, which is
not the HQ of that game (manual p. 67: the HQ starts on a random Galactic
Rim system and "The Empire does not know the location of Alliance
headquarters"; h-43 has no Mon Mothma at Yavin). They are no evidence for
or against the mark.

Not yet captured: the Empire's view of the real HQ system before any
sight. Recipe: load `EHQ2-d2.SAV` (sha256 `ae8048c027ae...`, unedited new
Empire HQ-victory game, day 2; the HQ is Tokmia, Churba), open Churba from
the GID: Tokmia with no 904. Then Fleet 1 (Coruscant) Move to Tokmia, run
Fast: when the fleet arrives (the Battle at Tokmia alert, about day 41-45)
the Churba window should show 904 (live copy), and x-39 after the battle.

Ported 2026-11-18: `shows_alliance_headquarters` no longer takes the
port's fog fallback (a system revealed by a fleet's sensor reach or the
half-way reveal, with no copy) as a live copy. It needs the player's own
system, a live copy (`object_views`), a frozen copy holding the HQ, or the
debug see-everything mode.

Gate (sector-quadrants.mjs, Empire): `headquarters-hidden`
(`Scenario::QuadrantsHeadquartersHidden`: the Empire's fog takes the HQ
system after the copies are seeded, no copy) asserts no 904 over the
planet; `headquarters-live` (`Scenario::QuadrantsHeadquartersLive`, the
Empire's copy live) asserts 904's every opaque pixel there and its place
against the status tracks under it, `(+1, +39)` from 904's corner to the
tracks' first segment, the same as in A0 x-39 (904 at (194, 71), the
tracks at (195, 110)).

Untraced: whether the Empire's live copy of a destination system lists an
HQ that is moving there (its copy is frozen, `0x10`).
