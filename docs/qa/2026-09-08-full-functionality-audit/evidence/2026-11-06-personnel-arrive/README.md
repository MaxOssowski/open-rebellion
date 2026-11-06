# Personnel Arrive (2026-11-06)

This covers the arrival of personnel, part of EVT-01-C007 ("arrival"). The cell stays partial, because A0 shows only this message's title, not its body.

## The source

`Class_0065b798_vt14` (`FUN_004955b0`) handles notification `0x200`; see `ghidra/notes/message-pictures.md`, "Personnel Arrive in detail".
- **What arrives.** A character or special force lands at a system (not an abode). The special force `0x3c000009` stays silent.
- **Which missions.** The moving mission must be of family `0x40..0x4f`: Move, Return or Adrift.
- **A major leader.** It titles the message "{name} Arrives at {system}". The body is "I have arrived at {system}.  ", or `0x70c4` followed by a `0x70c2` line for each other member.
- **Any other leader.** The title is "Personnel Arrive at {system}". The body is `0x70c3` followed by a line for every member.
- **Picture and speaker.** The reader's own background is `0x412`/`0x413`, with the leader's portrait. The speaker comes from the speaker map, or `0x15`.

## A0

- `.artifacts/a0-vm/mission-reports/index-day914-top` (the Expert Alliance game) lists "Mon Mothma Arrives at Klatooine" on the Mission rail.
- The Empire game's mission trips (the Emperor's Espionage to Chandrila and back, Thrawn's Diplomacy at Chandrila) posted no arrival messages. This matches the family test.
- An attempt to capture a Move's body failed: the Emperor's Move to Averam was not accepted. The body is therefore not captured.

## Before

The port posted no Personnel Arrive message.

## Now

- `MissionAdvance::arrivals` gathers each Move, Return or Adrift landing by mission, place and day (`missions::note_arrival`).
- `story_messages::personnel_arrive_texts` builds the title and body.
- The app's `push_personnel_arrivals` files the message on the side's Mission rail.

## Tests

**Unit tests.**
- `personnel_arrive_speaks_through_a_major_leader_and_lists_the_others`: alone, with one and with two companions, a minor leader, and no texts.
- `only_a_move_return_or_adrift_landing_is_reported_and_landings_group`: Diplomacy, no mission, an abode and the silent special force report nothing; landings group by tick.
- `a_move_takes_its_member_to_the_target_and_ends_with_no_result` now checks the arrival.
- `a_move_posts_personnel_arrive_to_its_side`: Mon Mothma's speaker is 0x17, her portrait is shown, the background is 0x412, and a minor leader's speaker is 0x15.

**Mutants.** `personnel_arrive_texts`: 8 caught. `note_arrival`: 12 caught. `push_personnel_arrivals`: all caught; the 7 missed are in `window_conf` and `draw_fullscreen_texture`, outside the filter.

**Browser.** `message-display.mjs` (Alliance) files Mon Mothma's landing at Klatooine through `push_personnel_arrivals`. The message reads "Mon Mothma Arrives at Klatooine" / "I have arrived at Klatooine.  " over 0x412.
