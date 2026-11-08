# Neutral and destroyed systems in the sector window, a battle's Load, and the contents' screen colours

Date: 2026-11-08 (night). Cells:

- CMD-03-C002 (neutral)
- CMD-03-C009 (destroyed)
- PRE-03-C006 (disabled)
- TAC-03-C012 (contents)

## Neutral and destroyed systems

A0 VM `2026-10-28-nav/sector-b.png` (the Sesswenna sector, the Alliance,
day 103): every explored neutral system has a white name and STRATEGY 10787
by its planet. The bitmap was found exact at Yaga Minor (142, 58), Chandrila
(64, 76), Ghorman (217, 118), Corsin (231, 234), Uvena (175, 266), Balmorra
(115, 273) and Sylvren (64, 332).

A0 VM `menus3/05-after-destroy.png`: Averam after the Death Star is the
asteroid field 10240 with a white name and no tracks.

The port already drew both states (`quadrant_icons`: side 0 gives 10787;
`sector_window::planet_art`, `DESTROYED_NAME_COLOR`). The ledger's
"cyan name" was stale. What was missing was a browser case.

The quadrant fixture now takes the sector's third and fourth systems
(`mark_systems`):

- an explored neutral one carrying the player's mine. The port shows
  another side's facility only with intel; the icon takes the system's
  side either way.
- a destroyed one.

`sector-quadrants.mjs` 'icons' (Linux, both sides) finds:

- 10787 beside the neutral planet, every pixel;
- the neutral name white, with no side colour;
- 10240 in place of the destroyed planet, every pixel but its black key.

## A battle's Load

A0 TAC-06-C007 and the Corsin battle (a0-vm/research-display/34) show,
in a battle's Game Options, every Save grey (10048) and an occupied slot's
Load lit (10049). The port has offered that since b123, but no browser case
had a save.

`run.mjs` now seeds slot 0 before the battle-options page loads, using the
build's localStorage keys `rebellion_save_v{SAVE_VERSION}_0` and
`rebellion_meta_v…`. A body that does not decode still occupies its slot.
It checks the battle's Game Options for:

- 10048 at (34, 81);
- 10049 at (287, 81);
- 10051 (empty slot 1) at (287, 123).

Linux, both sides, 640x480 and letterboxed.

## The contents' screen colours

The VM's 8-bit screen shows the list's icons colour-matched to the battle
palette. The Stormtroopers' entry 199, (238, 142, 142) in the bitmap, shows
as (194, 158, 149), the nearest colour (entry 112) in 23 of the 27 battle
palettes.

The port now draws each contents icon through the battle palette's
nearest colours:

- `BmpCache::get_macroquad_in_screen_palette`, with GDI's
  `GetNearestPaletteIndex` as `map_to_palette` already does;
- the palette from the runtime pack's ORTPAL record (`battle_palette`,
  `ortpal_rgb`).

The HUD's buttons keep their indices. The gate's VM comparison no longer
skips that colour: the regiment cells and the bar are 0 pixels from
a0-vm/coruscant-676/05.

## Mutants

The survivors are:

- `draw_contents_icon`, `draw_tactical_contents` and
  `get_macroquad_in_screen_palette`, which draw on the GPU;
- `battle_palette`, which reads the runtime pack's cache. That cache is
  empty in unit tests; `ortpal_rgb` has its own test.

The gate's VM comparison fails without them: the Stormtroopers' entry
would keep (238, 142, 142).

## A0 VM extras (the Empire game's last days)

- `.artifacts/a0-vm/sullust-809/`: the Battle at Sullust, where the
  Emperor and Vader were. Simulate gives "The Imperial fleet is defeated."
  / "Sullust is now under blockade by Alliance forces. The Imperial fleet
  has withdrawn to Filve.", and the Sesswenna and Sluis sectors turn red.
- `.artifacts/a0-vm/empire-end-809/`: the defeat movie, then the crawl
  ("Your failure is now complete. The Galactic Empire lies in ruins and
  the galaxy is in the hands of the Rebels. … The Emperor and Lord Vader
  are lost. … Even though the Empire is defeated, a new dark warrior may
  yet arise …"), then the main menu. This is END-01-C004's state again; that cell
  already passes.

The VM holds no game in progress now.

## Runs

Batch b128: 23 of 23 gates, Linux, Arch Chromium 155. Tactical matrix 148
of 148 (run 2026-11-08T02-12-29-570Z-2484542).
