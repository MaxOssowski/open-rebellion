# Death Star save: Destroy System and the icons' Scrap against A0 (2026-11-01)

A0: the Windows 10 VM, VM load slot 4 (the editor-assisted Empire save of
2026-10-24, one Imperial Star Destroyer retyped to the Death Star;
`2026-10-24-death-star-a0`), days 156-188, the Sesswenna sector.
Captures in `.artifacts/a0-vm/menus3/`, hashes in `a0-sha256.txt`.
The save was edited only to give the Death Star; the menus and orders are
the unchanged `REBEXE.EXE`.

## What A0 shows

- `01`, `02`, `03`: the Death Star fleet's icon over neutral Averam:
  Move, Confirmed Move, Planetary Bombardment, Encyclopedia, Scrap
  enabled, Planetary Assault and Status grey; the bombardment submenu
  lists Destroy System enabled. The Empire's highlight is green.
- `04-destroy-1..6`: Destroy System runs at once, with no confirmation:
  movie 101 under "The Death Star has destroyed the neutral system
  Averam." (the port: `death_star_destroyed_movie`, OBJ-14-C013).
- `05`: after it Averam shows the asteroid field over the sector window's
  own background, its name white, its tracks gone; the fleet stays.
- `06`, `07`: the fleet icon there now greys the bombardment parent;
  Scrap stays enabled.
- `08`: the fleet icon's Scrap asks "Are you sure you want to scrap the
  following units?" with "Fleet 1".
- `10`, `11`: Coruscant's facilities icon (Destination, Encyclopedia,
  Status grey, Reserved, Scrap) and defenses icon (Move, Confirmed Move,
  Encyclopedia, Status grey, Scrap).
- `12`: the defenses icon's Scrap lists the system's Stormtrooper
  Regiments; `13`: the facilities icon's lists Training Facility,
  Construction Yard, Mine. Both cancelled with the X.

## Changes

1. **The destroyed system's picture and name.** `FUN_0060bd20` loads
   STRATEGY 10240 keyed (`FUN_005fbd20(.., 0x2800, 2)`, as the marks
   0x388..0x38a): the port drew its black square. It is now keyed on its
   first pixel (`uses_first_pixel_key`). The name colour `0x2ffff00` is
   the unheld systems', which A0 draws white (`00`, `05`); the port drew
   cyan. Port run `fleet-window-2026-11-01T11-41-51-758Z-236963` (`fleet-window.mjs destroy-system`,
   both sides):

   ```
e76b3e235ece72019c6bfa69a7ebfcbdb316515ca35bf942031ce33e99aa63c6  empire-destroy-system/destroy-done.png
   ```

2. **An icon's Scrap.** `FUN_00512700` gives the order its team by the
   icon's kind: facilities (4) the manufacturing then production
   facilities, defenses (8) the regiments, squadrons and defense
   facilities, fleets (`0x10`) the fleets, each filtered to the side
   (`ghidra/notes/sector-icon-menus.md`). The port never enabled Scrap on
   an icon. Now `scrap::icon_targets` builds the team,
   `MenuObject::scrap_team` gives it for any selection, `order_gates`
   enables Scrap when the team is not empty and every member passes
   (`FUN_0053c4b0`, `FUN_0053c1a0`), and the confirmation lists every
   member's name. `sector-quadrants.mjs defenses-icon-scrap` (Linux
   Chromium 155, both sides) scraps the fixture's regiment and battery
   from the defenses icon: counts `[0, 1, 0, 0, 1]` then all zero. Run
   `sector-quadrants-2026-11-01T11-53-20-368Z-244414`:

   ```
74dba9b8aa96914316729d8596d1b38c399e392e52b6701ab2a574970d8d7f1e  alliance-defenses-icon-scrap/defenses-icon-menu-screen.png
d56365300b93b155fb19c3b6d3cf6221bdbc61ee80b69713b5211869fdc2833b  alliance-defenses-icon-scrap/defenses-icon-scrap-confirmation-screen.png
2ed4390d4fb96320ca11f12a1ae8b9f315ea5f28ae62bcea9e1ed4a3fcbb265f  alliance-defenses-icon-scrap/defenses-icon-scrapped-screen.png
ce6095a655172e68336ea92a0f3d01fafd7c80d1504491596bb9560b6b92154b  empire-defenses-icon-scrap/defenses-icon-menu-screen.png
06d15a1e4654503fc36aef53ac4eb0d9f72d760366f326bd03e61c0240f4c467  empire-defenses-icon-scrap/defenses-icon-scrap-confirmation-screen.png
2bcfb8befb25f6d918139b9bead0f29c8e7a2f13b1f3349bc6dd2901870a67f6  empire-defenses-icon-scrap/defenses-icon-scrapped-screen.png
   ```

Tests: `an_icons_scrap_takes_its_kinds_objects_of_the_side` (core),
`an_icons_scrap_team_is_its_kinds_objects_and_another_selection_its_own`,
`a_scrap_row_follows_its_gate_for_the_units_it_names` (icons added),
`a_sector_icons_scrap_takes_its_kinds_objects_of_the_side` (fails
without the team), `build_selections_buttons_key_out_their_first_pixels_colour` (10240 keyed), `a_destroyed_system_shows_the_asteroid_field` (white name). Mutants:
`scrap.rs` 5 caught, 1 unviable; `object_menu.rs`/`quadrant_icons.rs`
7 caught, 2 unviable.

3. **A fleet icon's team holds the fleets arriving there.** A0 `23`: the
   fleet icon over Chandrila, where Fleet 1 is arriving from Averam (`20`,
   `21`: the menu before leaving, and Averam's window empty after), greys
   every order but Encyclopedia, Move and Scrap included. A move puts the
   fleet in its destination at once (`FUN_00556390`), so `FUN_004ffe70`
   gives it as a member, and a member en route refuses the whole order
   (`FUN_0053c1a0`). `movement::system_side_fleets` and the fleets
   icon's `scrap::icon_targets` now take the fleets `listed_fleets` gives
   (orbit and arriving), so an icon over one orbiting and one arriving
   fleet refuses Move, the bombardments, Assault and Scrap.
   `a_sector_icons_scrap_takes_its_kinds_objects_of_the_side` fails with
   the orbit-only team.

## Open

- The fleets icon's and facilities icon's Scrap have no browser case
  (the defenses icon's does); their teams are unit-tested.
- Planetary Assault (C012): no A0 capture of it enabled yet.
