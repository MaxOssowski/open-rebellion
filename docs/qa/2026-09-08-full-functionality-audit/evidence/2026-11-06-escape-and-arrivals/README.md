# Character Escaped, Headquarters Arrives and unit arrivals from TEXTSTRA (2026-11-06)

This covers EVT-01-C011 ("capture"), part of EVT-01-C007 ("arrival"), and CMD-08's event messages. These messages had kept the port's own English or been missing.

## Character Escaped

`FUN_00490340` handles notification `0x1e0` with the escape flag `+0x98` set (cases 1 and 2).
- **The captor's side** reads `0x715c` "{1} Escaped" over `0x715d` "{1} escaped from the {3} at {2}.", with its own side's name.
- **The escapee's side** reads `0x715e` over `0x715f` "{1} has escaped from imprisonment at {2}.".
- **Picture and speaker.** Background `0x405`. The speaker is Mon Mothma `0x26`, Leia `0x18`, Luke `0x25`, Han `0x19`, the Emperor `0x27` or Vader `0x28`. Any other escapee takes `0x24` on its own side.
- **hyp.** Which reader takes which case follows the capture's A0-settled order, since the class tests the same side.

**Before.** The port posted "{name} has escaped captivity!" at both escape sites.

**Now.** The app's `escape_character` frees the captive and posts `story_messages::escaped_message` to each side, naming the system it was held at.

## Headquarters Arrives

`FUN_004981c0` case 4 (family `0x20..0x21`) reads `0x707c` "Headquarters Arrives" over `0x707d` "The Alliance Headquarters has arrived at {2:1}". Its category is `0x40` (the Defense rail), its picture `0x3fe`, its event 6. The Empire's reader posts nothing.

**Before.** The port posted no message for this; it said "no arrival message is traced".

**Now.** `headquarters_arrival_message` posts it when the headquarters' trip ends.

## Regiments and squadrons travelling on their own

`FUN_004981c0` case 3: `0x707a` "Units Arrive at {2:1}" over `0x707b` and the unit's `0x707e` line.

**Before.** The port posted "Regiment arrived at …" and "Squadron arrived at …".

**Now.** They go through the existing `arrival_texts` with the unit's kind. The port's line is kept only when the texts are missing.

## Tests

**Unit tests.**
- `an_escape_reads_from_the_captors_side_and_the_escapees`: both readers, each major's speaker, a minor escapee.
- `an_escape_frees_the_captive_and_tells_both_sides`: the captor taken from `captured_by`, or else the side it escaped from.
- `the_headquarters_arrival_names_its_system`.
- `the_headquarters_arrival_goes_to_the_alliances_defense_rail`.

**Mutants.** `escaped_message`: 11 caught, 1 unviable. `escape_character`: all caught. `headquarters_arrival_texts`: 5 caught. `headquarters_arrival_message`: all caught. The missed mutants are in `window_conf` and `draw_fullscreen_texture`, outside the filter.

No A0 capture of these messages exists.

## Character Health, and two messages taken off the index (later the same day)

**Character Health.** `FUN_0048fdf0` (notification `0x106`) titles its messages from TEXTSTRA:
- Injured: `0x7160` over `0x7161`;
- Healed: `0x7162` over `0x7163`;
- Killed, for a destroyed character: `0x7164` over `0x7365`, on background `0x42e` or `0x433`.

**Before.** The port wrote "X Injured: X has been injured." as one line with no title, and the Death Star's victims "X has been killed.".

**Now.** `story_messages::character_health_texts` builds the title and body, and `push_character_health` takes the texts. The port's line stays only when the texts are missing; the existing tests check that fallback. A destroyed character reads Killed, and the Death Star's cleanup posts through it.

**Two messages off the index.** Death Star Sabotaged (`FUN_0048d470`) and Alliance Headquarters Destroyed (`FUN_0048e5a0`) are story movies only (kind 6, `movie-flow.md`). The port already queues those movies. It had also filed its own "The Death Star has been sabotaged." and "Alliance headquarters destroyed at …" rows on the Mission and Resource rails. Those lines now stay in the log, off the Message Index.

**Gate.** `fleet-window.mjs` `travel` now expects the regiment's arrival as "Units Arrive at {system}", the title of `0x707a`.

**Tests.**
- `character_health_takes_its_titles_and_bodies`.
- `character_health_reads_its_textstra_title_and_body_and_a_death`.

**Mutants.** `character_health_texts`: 5 caught. `push_character_health`: all caught; the missed mutants are in `window_conf` and `draw_fullscreen_texture`, outside the filter.

## Maintenance Shortfall against A0

- A0: `.artifacts/a0-vm/maintenance-shortfall/alliance-view-coruscant-day163.png`
  (sha256 `a31c0bc3bacb7446f17c97edac61494edee2ca19456161c1a96139c7acd1eded`),
  DS_flip_allian, day 163.
- Texts: "Maintenance Shortfall" and "The following units have been destroyed
  at Coruscant due to a maintenance shortfall:" followed by "TIE Fighter". The
  `maintenance_texts` test now asserts this case.
- Picture: STRATEGY 1014 (`0x3f6`) matches at screen (74, 82) with no differing
  sampled pixel.
- Correction (same day): DS_flip_allian is a host-edited save. It builds its
  messages for the Empire behind an Alliance cockpit: on day 306 it shows
  "Ghorman Joins Enemy" over "...join the Alliance."
  (`.artifacts/a0-vm/battle-display/joins-ghorman-day306.png`, sha256
  `953b9464bcd498e769c2f7ccdb5c3992b39b7ef4b20bbb717463498897ec08ef`).
- So this capture is the Empire's own message, with the picture
  `FUN_0048bbf0` gives reader 2, as in the port. The first reading, the
  message shown to the other side, was wrong, and there is no audience
  difference.

## Correction (2026-11-07)

Which side reads which escape text was the wrong way round. The escapee's own side reads `0x715c` / `0x715d`, and the captor's side reads `0x715e` / `0x715f`. See `../2026-11-07-capture-side-corrected/`.
