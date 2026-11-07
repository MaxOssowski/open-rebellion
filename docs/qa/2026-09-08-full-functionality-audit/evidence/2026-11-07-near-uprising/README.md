# Near Uprising against an unedited Alliance game (2026-11-07)

This covers EVT-01-C005 (uprising).

## A0

The unedited "test" save (Alliance, day 795):
- `.artifacts/a0-vm/uprising-clean/test-day795-index.png` (sha256 `5ddb2ba06ea3c5fa3614fbf919f40f6eeaebe97fdd03072699f2a1c86acff624`) lists "Umgul Near Uprising";
- `test-day795-umgul-near-uprising.png` (sha256 `ddad83ec19d841c8722781c8ee27d9db2e5421eb141d2b686a9f829fddc7bc31`) shows it over "Unrest has pushed Umgul close to uprising.".

The picture is STRATEGY 1008 (`0x3f0`, the Alliance holder's), exact at (74, 82). The rail's Display (`0x67`) is enabled.

## The difference

The port's message matched the texts and the picture, but it named no system. So the index's Display stayed grey, where A0's was lit.

## Now

- `story_messages::near_uprising_message` takes the system's key and files the message `at_system`.
- The economy's `TroopSurplusChanged` passes it.
- The unit test asserts the message's system.
- The EventMessages fixture files the reader's own Umgul near uprising before the joins.

## Browser (`message-display.mjs`, Linux, Alliance)

The gate opens "Umgul Near Uprising" from the Popular Support list and reads the A0 text with background `0x3f0`. Compared with A0, 0 pixels differ in each of these:
- the picture;
- the title's item image;
- the whole rail, Display lit included.

Screenshot: `.artifacts/interface-parity/message-display-2026-11-06T22-59-43-213Z/alliance/near-uprising.png` (sha256 `0a56729e2875a14bb52fa86e9fb1cfc73e861d80895cbc0da0f889997ced029a`).

## The same for Character Captured and Escaped

A0 `messages-day633/capture-palpatine` also shows Display lit. Every A0 message captured so far does: Joins, Saboteurs Strike, Enemy Mission Foiled, Captured and Near Uprising. Across them, the rail's Display button is the same 0-differing art.

The port's `captured_message` had no system, so its Display was grey. Now:
- `captured_message` takes the capture's system;
- `escape_character` sets the place the captive was held at on both of its messages;
- the unit tests assert this.

**port:** Display then goes to that system. The original's target is the message object's (`FUN_004ece60`) and is not traced.

`message-display.mjs` (Linux, Alliance) now compares the capture message's whole rail with A0 `capture-palpatine`: 0 differing pixels. Screenshot: `.artifacts/interface-parity/message-display-2026-11-06T23-56-31-561Z/alliance/capture.png` (sha256 `496471e6650c54c5552108b256f276c71d2f1f8726b4e9d82b89bcff2b054a6c`).
