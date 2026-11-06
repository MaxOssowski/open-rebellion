# Saboteurs Strike (2026-11-06)

This covers the message the victim's side reads when a sabotage mission destroys one of its objects. It belongs to OBJ-12-C003 (a mission's completion) and CMD-08-C011 (the Conflict rail).

## A0

The unedited "test" save (Alliance, day 650):
- `.artifacts/a0-vm/saboteurs-clean/test-day650-conflict-list.png` (sha256 `e13f50773405384cb2cd3c7bac882404e23c10a859aa52de54e9e87d6500b1be`) shows "Saboteurs Strike at Yavin" on the Conflict list;
- `test-day651-saboteurs-yavin.png` (sha256 `5c76769e59963835f83402c77b9df1e6fa45f8f9f3ada89d17e2f2bda6ba90ab`) shows the message: "The following units were destroyed by saboteurs at Yavin:" over "Alliance Army Regiment".

Its picture is STRATEGY 1013 (`0x3f5`), 0 differing pixels at (74, 82).

## The original

`FUN_0048b9a0` (class `0x65b218`, notification `0x305`) mirrors Maintenance Shortfall:
- **Texts** (RCDATA as shipped): title `0x7083` "Saboteurs Strike at {3:1}", body `0x7084` "The following units were destroyed by saboteurs at {3:1}:\n", and one `0x7085` "{1:1}\n" line per unit. The system is found up the object's containers.
- **Category** `+0x34` = `0x100`, the Conflict rail.
- **Advisor event** `0xc`; `+0x30` is `0x456`.
- **Reader.** `+0x40` keeps the message for the object's side. `FUN_0048bbf0` then sets the picture by the reader: `0x3f5` for side 1, `0x3f6` otherwise.

The Death Star is the exception. Its sabotage is the story movie of `FUN_0048d470`.

## Before

The port posted nothing when a sabotage destroyed a facility, a regiment, a special force, a ship or a squadron. Only the Death Star's movie was queued.

## Now

- `missions::sabotage_victim` reads the object's side and name (a ship's own name, else its class's; other units by class) before `destroy_target` removes it.
- `story_messages::saboteurs_texts` builds the title and body.
- The app's `saboteurs_strike_message` files the message:
  - on the Conflict rail, for the object's side;
  - with picture `units_destroyed` by the reader and advisor event `0xc`;
  - nothing for a neutral object.
- `apply_mission_result` posts it for every sabotaged object but the Death Star.

## Tests

- `saboteurs_strike_names_the_system_and_each_unit` (the A0 Yavin case).
- `saboteurs_strike_goes_to_the_victims_side_on_the_conflict_rail`:
  - the Alliance's regiment: Conflict, `0x3f5`, event `0xc`;
  - the Empire's: `0x3f6`;
  - a neutral or gone object posts none.
- `a_destroyed_object_leaves_its_system_and_the_world` names each kind before it goes, and returns none after.
- `a_sabotaged_ship_is_named_by_its_own_name_else_its_class`: a lost hull of the class is skipped.

**Mutants.**
- `sabotage_victim`: all caught after the ship test.
- `saboteurs_texts`: 5 caught.
- `saboteurs_strike_message`: caught. The missed mutants are `window_conf` and `draw_fullscreen_texture`, outside the filter.

## Browser (`message-display.mjs`, Linux, Alliance)

The EventMessages fixture posts Saboteurs Strike for its first Alliance regiment before its battle reports, so it lists third on the Conflict rail. The gate opens it and finds:
- "The following units were destroyed by saboteurs at Ithor:" over the regiment;
- background `0x3f5`;
- the picture differs from STRATEGY 1013 and from A0 in 0 pixels, and so does the title's item image.

Screenshot: `.artifacts/interface-parity/message-display-2026-11-06T19-52-18-186Z/alliance/saboteurs-strike.png` (sha256 `e0f566bdb7b54653f7276ad3bb08ac06b4f68ca250e9ba0ce513884bbccf69fb`).

## Also seen in the same game

**Display from an Enemy Mission Foiled row (CMD-08-C014).** On day 685 the Mission list held one "Enemy Mission Foiled" row (`.artifacts/a0-vm/display-clean/test-day685-mission-list.png`, sha256 `f796799151403fed5a86f5a944066bf15866a1f375104fa80bf10e53e94b9a68`). With that row chosen, Display (`0x67`) closed the index and opened the Sesswenna sector window, where the mission's target lies (`test-day686-display-opens-sesswenna.png`, sha256 `3a6251d59c03c0b4712f246a032b8535fa97584d3d6798ee7c243723fbed09ec`). The port files that message `at_system(ended.target_system)`, and `display_target` goes to the same sector.

**Expiry.** On day 609 the list held two earlier foiled rows, Selonia and Umgul. By day 685 both were gone, as the base expiry rule (`FUN_004c4fe0`) predicts for class `0x65b6c8`.

**Battle Summaries.**
- Day 672, the Empire's Battle at Averam (`.artifacts/a0-vm/battle-clean/10..12`): "The Alliance fleet is victorious." over "Averam has been successfully defended from Imperial forces. The Imperial fleet has withdrawn.". The Display page reads "The Alliance fleet is victorious. The world Averam is now under Alliance blockade.", the `0x7012` text even for the Alliance's own system.
- The unit test `the_alliances_defence_of_its_own_system_reads_as_a0` gets the same lines.
- Hashes: 10 `53b1242bdafdf0fde8dbf1f4d0705a653c03d400b828ae416472f0a35f7fd0c8`, 11 `973e8be94d723552f7b7e5473661463c3d7bfba8951b37bfcfb3405c0818879e`, 12 `4d7829bb438683b23537f7e82bef70142eb7498c747dbb5c16e70136030030de`.
