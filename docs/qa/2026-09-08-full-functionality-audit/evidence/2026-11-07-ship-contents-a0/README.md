# The selected ship's contents list, the escort and squadron lines, and the battle windows' text

Date: 2026-11-07 (night). Cells:

- TAC-03-C012 (contents)
- TAC-01-C004 (running)
- TAC-07-C012 (go to fleet)
- TAC-03-C016 (the order line)

## The contents list

The port drew a ship's characters in the panel 1302 as GOKRES 61x25
images, two at most, at (492 + 63i, 156). It did not list regiments or
special forces.

The originals:

- A0 TAC-03-C012 (Wine): the Medium Transport's two Alliance Army Regiments
  are TACTICAL 2301 at (495, 154) and (527, 154).
- A0 VM `.artifacts/a0-vm/coruscant-676/05` (Windows 10, the Empire game's
  battle at Coruscant, day 676): the Imperial Star Destroyer's list holds
  icon 2240 (a character) at (495, 154), then Stormtrooper Regiment icons
  2317 every 32 pixels. The fourth cell is clipped at x 619. A scroll bar
  sits on row 189: arrows 1351 at 495 and 1353 at 599, and the thumb 1355
  at 515, 1356 tiled from 519, 1357 at 591.

The code:

- `FUN_005e4a30` makes the list: a CoolDragList (`FUN_00607ea0`) of 124 by
  45 at panel (13, 126), cells 0x20 by 0x20, with its scroll bar from
  TACTICAL 1350's set (`FUN_0060a560`).
- `FUN_00597610` builds the icon table: 79 pairs of (key, TACTICAL
  bitmap). `FUN_005975d0` looks a key up.
- The keys are the units' TEXTSTRA name ids less 0x2000:
  - TROOPSD's `text_stra_dll_id` runs 0x2440..0x2444 and 0x2480..0x2484,
    and the keys 0x440..0x444, 0x480..0x484;
  - the special forces have 0x2540.., the major characters 0x2840.., the
    minor ones 0x2a40...

  Each key is also the unit's STRATEGY mini less 0x4000. The Alliance Army
  Regiment's mini is 17473 = 0x4441, its key 0x441, its icon 2301, as A0
  shows. The port's mini tables (`troop_mini`, `special_force_mini`,
  `character_mini_resource_id`) therefore give every key.
- `FUN_0040b0e0` hands the battle each ship's characters, then its special
  forces (`FUN_00504040`), then its regiments (`FUN_00504c40`).

The port now:

- seats each hull's units where the Fleet window seats them. That rule is
  now shared as `fleet_window::ship_cargo`: regiments fill the live ships'
  troop capacities in fleet order; a character rides on its seat ship, or
  else on the first living ship; special forces ride on the first ship.
  The rule was previously an inference recorded in the Fleet window, and
  is unchanged.
- runs `TacticalState::board_contents` right after `begin_battle`
  (`tactical_flow::begin_player_battle`, which now takes the troop
  transport).
- draws the list as the original does (`draw_tactical_contents`): icons
  from (495, 154) every 32 pixels, clipped to the list. With more icons
  than the 124 pixels hold, it draws the bar: thumb width = floor(84 ×
  124 / content). That gives 81 for four icons, as the VM capture shows.
- moves the list a cell per arrow click, clamped to the overflow, and draws
  the held arrow pressed (1352, 1354). The bar's track click and drag are
  not ported.

The core no longer seats the characters itself, and `TacticalShipContent`
keeps the unit's mini.

Gates:

- The `detail-escort` fixture now carries a character and three of its
  side's regiments on a ship with room for them.
- `run.mjs` checks the first three icons at (495 + 32i, 154) and the bar's
  pieces (1351, 1353, 1355 at 515, 1357 at 591).
- One right-arrow click moves the thumb to 518 and the second icon to 523.
  The left arrow brings the thumb back.
- Unit tests:
  - `a_units_contents_icon_follows_its_mini`
  - `the_contents_bar_shows_once_the_icons_overflow_the_list`
  - `a_ships_contents_list_its_characters_then_forces_then_regiments`

### One colour

Against the VM capture, the regiment cells and the bar are 0 pixels off,
except one pixel per trooper figure.

The icons' palette entries 160..245 are all (238, 142, 142). The VM's 8-bit
screen shows the Stormtroopers' entry 199 as (194, 158, 149), the nearest
colour in 23 of the 27 battle palettes (their constant entry 112). So the
list's icons are colour-matched to the battle palette, as GDI does for a
DIB on a palette device. The HUD's buttons are copied by index instead:
the subsystem icons' entries 140 and 146 take the battle palette's (230, 0,
0) and (255, 230, 60), although (255, 0, 0) and (255, 255, 0) exist in it.

Wine (A0 TAC-03-C012) keeps the bitmap's own colour, and so does the port.
The gate skips those pixels. Matching the 8-bit screen is open.

## The escort and squadron lines

The fighter panel's line (`FUN_005e16a0`) switches on the mover's `+0x68`,
the order code: 1 Escort, 2 Recover, 3 Withdraw, 4..10 the attacks and
maneuvers. So `FUN_005e4b40`'s "`+0x68` != 1" means "not an escort". An
escort keeps "Escort" (`0xdde1`) before the escorted ship's name; every
other held target reads "Attacking" and the name.

The squadron's line follows its route ("Navigating") and target the same
way, without the capital panel's speed test.

The port does both (`TacticalShipActivity::of_fighter`, `Escorting`).
`run.mjs` 'escort-orders-line' finds "Escort" and the ship's name after
the direct friendly right click. The unit test is
`an_escort_names_its_ship_and_a_squadron_its_route_or_target`.

## The battle windows' text

The battle windows' texts (Battle Alert, Battle Results, the Display page)
were rastered at 10 pixels and scaled up. That blurred them and dimmed
their colour to at most 191 (Alliance red).

A0 TAC-07-C012 draws the "Select one of the following buttons…" lines in
solid (255, 0, 0): 2,162 pixels. The VM's Empire result
(coruscant-676/11) is solid green.

`battle_window_text_params` now rasters each text at its own size. In
`run.mjs`, 'battle-results-instruction-solid' counts the solid pixels of
those lines at 640x480 and needs 200 (the port had none; Liberation Sans's anti-aliased edges keep the count below A0's). The glyph shapes stay Liberation
Sans, an accepted deviation.

## Mutants

The batch's in-place run over the diff tested 153 mutants and left 54
survivors. Tests added after the run close the logic ones:

- `squadrons_fill_the_ships_fighter_capacities_in_fleet_order`, for
  `ship_cargo`'s `-=`;
- `the_fighter_panel_line_names_the_squadrons_target`, for
  `fighter_orders_line`;
- `a_battle_window_text_rasters_at_its_size`, for
  `battle_window_text_params`.

The rest are drawing and input paths:

- `draw_tactical_contents`'s geometry, which the gate compares with the VM
  capture pixel for pixel at 640x480;
- `prewarm_tactical_font_atlas`, whose absence shows as the letterboxed
  "deleted texture" errors the gate rejects;
- `begin_battle`, `draw_original_tactical_hud` and
  `handle_original_tactical_controls`, which the browser probes cover.

## Runs

Batch b126: 23 of 23 gates, Linux, Arch Chromium 155. Tactical matrix 148
of 148 (run 2026-11-07T23-06-24-102Z-1973124).

A0 VM extras from the same night are in `.artifacts/a0-vm/coruscant-676/12..14`:

- Closing the result window played the Coruscant movie under 0x7171.
- The Personnel Finder lists "Veers - Balmorra ( Captured )" (`.artifacts/a0-vm/mission-kinds-679/02`).
- A mission against the now Alliance-held Coruscant offers Incite Uprising
  and Espionage only (`.artifacts/a0-vm/mission-kinds-679/07`, `08`).
