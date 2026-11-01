# Manufacturing band menus and Reserved against A0 (2026-11-01)

A0: the Windows 10 VM, the Empire Death Star save (slot 4) on day 573,
Coruscant's Manufacturing window: no shipyard and no training facility
(0:0), one construction yard (1:1). Captures `.artifacts/a0-vm/menus3/30`
to `33`, hashes in `a0-sha256.txt`.

## What A0 shows

- Every band lists Build, Stop, Destination, (ships only) Rename,
  Encyclopedia, Status, Reserved, as the port does (sort keys 100, 110,
  120, 500, 1000, 1001, 1002).
- Build is grey on the ship and troop bands (no yard of the area) and
  white on the facilities band; Stop is grey on all (nothing building);
  Destination, Encyclopedia, Status and Reserved are white; Rename grey.
- Reserved chosen on the facilities band, its menu shows the check mark
  (STRATEGY 11902, the record's word 10) left of "Reserved" (`33`). The
  menu is 122 pixels tall in both states; its text moves right by the
  icon column.

## Changes

1. **Build needs a yard.** `FUN_00527d00`, Build's validation, refuses a
   manager with no usable yard (`(0xa0, 3)`). The port enabled Build on
   any band of the player's system; it now also wants a yard of the area
   (`ai_module1::has_yard`). `a_bands_stop_and_destination_belong_to_its_systems_holder`
   expected Build on a band with no yard; A0 `30`/`31` settle that it
   is grey, so that expectation changes, and the test now adds a
   shipyard before expecting Build.
2. **Reserved.** `ManuMgr_vt1fc` (`0x216`) sets the manager's
   `Reserved` (`ManuMgr_SetReserved`, `FUN_00529710`).
   `ManufacturingState` keeps each area's mark (**save v110**), the
   band's row is enabled as Destination is (hyp: its `+0x1dc` is not
   read) and checked while the mark is set, and choosing it flips the
   mark (`PanelAction::ToggleReserved`, logged `command=0x216`). The
   player agent's records turn the mark into bits `0x400..0x1000` and the
   sector's `0x10000..0x40000` (`FUN_004ebe40`, `FUN_004c6830`); the
   port's Build agent reads the idle bits only, so no agent reads the mark
   yet (open). The facility icon's Reserved (kind 4, every manager) is not
   ported and stays grey.
3. **A check mark does not grow the row.** `FUN_004aba60` draws a
   checked item's mark from `+0x1c` when `+0x5c` is set, and
   `FUN_004abbf0` counts only the item's own icon `+0x14` in the row's
   height; the port counted the mark's 20 pixels. `GameMenuEntry::check`
   now keeps it out of the height (`entry_icon_size`); A0's Agent menu
   (`agent-menu/empire-agent-menu-day137.png`) keeps its checked rows 20
   pixels tall as well.

`build-selection.mjs` (Linux Chromium 155, both sides) chooses Reserved
on the troop band, reopens the menu and checks the mark: width 90 to 110,
height 122 both (A0 90 to 107, 122 both). Run `build-selection-2026-11-01T12-55-15-188Z-383390`:

```
152107a32c8283f455aec66d1c5e970a4fcab0ccaa41b8120aa889c91a721559  alliance/reserved-menu.png
455160434f647ec5f0e424fe0586a1b607a072de6d6297088e94fb5b162e2b90  empire/reserved-menu.png
885cea2eb7626e69019d7dc9caf8280026a560ee4fb7cbacfa139a269bc3a389  alliance/reserve-menu.png
5eb92760b6948be7cf795e97e6a06044dbb296008b8e6f052ffcebf17f648988  empire/reserve-menu.png
```

Tests: `each_production_area_keeps_its_own_reserved_mark`,
`a_bands_build_stop_and_destination_follow_their_rules` (Reserved rows),
`a_bands_stop_and_destination_belong_to_its_systems_holder` (Build's yard,
Reserved gates), `entry_icon_size` in `game_menu` tests. Mutants:
render 21 caught, 13 unviable; core 3 caught.

## Open

- The checked menu is 110 pixels wide in the port and 107 in A0: A0's
  text moves 17 pixels for the mark, the port's 20 (the mark is 20
  wide; `FUN_004abf60` measures `+0x20`, not read further).
- No agent reads Reserved yet; the facility icon's Reserved is not ported.
