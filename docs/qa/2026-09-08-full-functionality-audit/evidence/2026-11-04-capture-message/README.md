# The Character Captured message (2026-11-04)

This covers EVT-01-C011 ("capture"). The reading is in `ghidra/notes/story-message-classes.md`, "Character Captured".

## A0

The VM's Alliance game, day 633. The hashes are in `a0-sha256.txt`.
- **The index** (`index-tab0`, `index-tab3`). The Mission Messages list "Emperor Palpatine Captured".
- **The message** (`capture-palpatine`, the index window at (62, 49)).
  - The title is "Emperor Palpatine Captured".
  - The text is "Emperor Palpatine was captured by the Alliance at Corfai."
  - The picture is STRATEGY 1049 (`0x419`) at (74, 82), all 80000 pixels exact.

## Before

A mission capture posted no message. Only Jabba's captures had one. `message_pictures::captured` was never used.

## Now

`story_messages::captured_message` is `FUN_00490340` cases 3 and 4.
- **The text.** A captive of the reader's enemy reads TEXTSTRA `0x7158` / `0x7159`, with the captor's side as parameter 3. A captive of the reader's own side reads `0x715a` / `0x715b`.
- **The picture and speaker.** The background is `(captive side != 1) + 0x418`, and the speaker is `captured_event`.
- **The posting.** `main.rs` posts it for each side on a mission's `CharacterCaptured`, unless Jabba's Palace holds the captive.
- **hyp.** Which side's message takes which case rests on A0's one capture. The class's side test does not settle it.
- **port.**
  - Escapes (`0x715c`..`0x715f`) post no message.
  - Han's no-message case (`0x33000243`, `+0xb0` bit 2) is not modelled.

## Tests

- **Unit tests.**
  - `a_capture_reads_the_captor_to_the_other_side_and_the_place_to_its_own` (story_messages);
  - `the_event_messages_scenario_files_the_emperors_capture` (fixture).
- **`message-display.mjs`** (Linux, both sides). The EventMessages fixture files the Emperor's capture at Corfai.
  - The index lists "Emperor Palpatine Captured". Display Message shows A0's title and text for the Alliance, and "Emperor Palpatine was captured at Corfai." for the Empire.
  - The picture is 1049 at the window's (12, 33), every pixel exact. For the Alliance that is A0's (74, 82), and A0 matches the same bitmap.
  - The gate's row helper now scrolls from the top, one notch for each row past the ninth, since the list holds 11 rows.
