# Game Options against A0 (PRE-03)

Date: 2026-10-31. Linux Chromium (swiftshader), the interface test build.

## What A0 showed

A0 `tooltip/200-options.png` (hash in `a0-captures.sha256`) shows Game
Options opened from the galaxy in an Empire game, with six saves, the first
named "test". Matching COMMON.DLL's bitmaps against it pixel by pixel showed
that the port differed in several ways:
- **Positions.** The port used the upstream P58-B12 positions, one to three
  pixels off. A0 has the bitmaps at `FUN_00407180`'s literal arguments:
  Save (34, y), Load (287, y), Restart (76, 381), Return (162, 382), Exit
  (248, 381), Play Music (352, 76), switches (357, 311 / 337 / 365 / 392 /
  419), sliders at (392, 130) and (392, 190).
- **Switch art.** An "on" switch shows its pressed art (`FUN_00603150(4,
  ..)`). A0's five "on" switches show 10044. The port showed 10043 for "on"
  and 10045 for "off".
- **Return art.** A0 shows 10023, the Empire's frame. The window mode 8 and
  the player's side pick it. The port always showed 10020.
- **Keying.** Restart and Exit have a white key (palette index 11). Return,
  the crests and the tracks key on their corner pixel. The port drew those
  pixels.
- **Palette entry 254.** The background and the tracks show it as white;
  the DIBs have cyan there. The port drew cyan.
- **Text.** A0 draws the headings in 18-pixel bold Arial, bright green and
  centred. The labels and their On/Off are bright green when on and dark
  green when off. The names are white with a black shadow. A side crest
  sits beside each named save, and an empty row is blank. The port used a
  green monospace face and "Saved Game N" names, and showed no crests.

## The original

`ghidra/notes/game-options.md` (controls, switch states, Return art,
keying) and `ghidra/notes/game-options-text.md` (text objects, save rows,
slider transfer, version label).

## The port

- `game_options.rs`:
  - draws the texts with `draw_original_text`: the game font table, GDI
    whole-pixel `DT_VCENTER` and ascent, clipped headings, `DT_NOCLIP` text
    objects;
  - uses the constructor's rectangles;
  - shows a set switch's pressed art;
  - picks Return's art with `return_art`;
  - draws crests from `SaveSlotInfo.player_is_alliance`;
  - leaves an empty row's name blank;
  - draws the slider thumb at `floor(201 * v / 100)`, clipped, and takes a
    drag only inside the 5-pixel margins;
  - saves a blank name, which the original also allows (`FUN_00406b80` ->
    `FUN_00408000`).
- `bmp_cache.rs`:
  - adds `get_macroquad_with_entry`, which replaces one palette entry (254
    to white);
  - keys COMMON 10020..10057 on the first pixel;
  - reads that colour from 255-colour tables.
- Save metadata: native and browser saves record the player's side
  (browser JSON field `player_is_alliance`, optional; older metadata reads
  as unknown and shows no crest). The save format version is unchanged.

## Gate

`tools/interface-parity/game-options.mjs` (`game-options-result.json`, pass):
- **Empty list.** No name and no crest in any of the six rows.
- **Save.** Typing "test" into row 1 and pressing its Save logs
  `save_state_fingerprint slot=0`.
- **Whole window.** Of the 201,491 pixels outside the texts, the five
  differing rows, the crest, the sliders and A0's pointer, 0 differ from A0.
- **Crest.** The Empire crest matches A0's Empire row (row 4) pixel for
  pixel, and its key pixel shows the background.
- **Sliders.** They differ from A0 only in two thumb runs each: the volumes
  differ. The music thumb at 100 per cent sits clipped at columns 588..592,
  as A0's does.
- **Texts.** Ink boxes match within 1 pixel (left or centre, top, bottom).
  Widths are up to 11 per cent wider ("Tactical Display Options" 185
  against 167 pixels), which is the accepted text deviation of 2026-10-30.
- **Drag.** Dragging the music thumb to slider x 100 puts it at columns
  485..495 (centre 98 = floor(201 * 49 / 100)).

Also run after the change:
- `tooltip.mjs` with the A0 tips, both factions: pass.
- `main-menu-rest.mjs`: pass.
- the GID catalog (`run.mjs --all`): 152 of 152 before the change.

## Not covered

- **Version label.** The port shows its own version ("1.0.0 Open
  Rebellion" where the original shows "1.01.00 English (United States)").
  It is left out of the comparison.
- **Battle context.** The grey labels and the 10026 Return art from a
  battle have no A0 capture; a unit test covers the art choice.

## Settings (second stage, same day)

The original keeps the options in the registry (`MusicVolume`, `Volume`,
`MusicSwitch`, `DisplayOpts`; `ghidra/notes/game-options.md`,
"Persistence"). The port kept nothing. `rebellion_data::settings` now
stores the same four values, natively in `saves/settings.json` and in the
browser in `localStorage` (`rebellion_settings_v1`). `main.rs` restores them
at start-up and writes them when they change. A muted acceptance run
(`OPEN_REBELLION_MUTE`) leaves them alone.

The gate now also reloads the page after the drag. The stored settings
hold `"music_volume":0.49`, and the reopened music slider is identical to
the one before the reload (`reloaded_slider_differing: 0`).

The slider's transfer to DirectSound (`floor(2000 * log10(v / 100))`
hundredths of a dB) gives an amplitude within 0.12 per cent of v / 100,
which is the port's linear gain. The audible level was not measured against
A0.
