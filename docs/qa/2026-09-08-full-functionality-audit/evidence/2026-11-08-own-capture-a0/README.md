# Character Captured for the reader's own character, both sides, against A0

Date: 2026-11-08. Adds to EVT-01-C011, which already passes.

## A0

| File | SHA-256 | Game |
|---|---|---|
| `.artifacts/a0-vm/alliance-game2/han-solo-captured.png` | `eff306068adce7f82392241dc49af484fad6a50020e866bcb642225e8e5b2784` | a new Alliance game |
| `.artifacts/a0-vm/empire-game3/ozzel-captured.png` | `f2654f58074f06896844aa53951ce5ea07f4f011f5342a743e1221813c288535` | a new Empire game, day 353 |

- **Alliance.** "Han Solo was captured by the Empire at Yavin.", over
  `0x418` (STRATEGY 1048).
- **Empire.** "Ozzel was captured by the Alliance at Tralus.", over `0x419`
  (STRATEGY 1049).

Both confirm that the captive's own side reads `0x7158` / `0x7159`, naming
the captor. The cell's note called that reading a hypothesis. The
Emperor's capture, which the Alliance reads (`messages-day633`), is the
other side's text, `0x715a` / `0x715b`.

## Port

The event-message fixture files each reader's own capture through
`story_messages::captured_message` (`post_own_capture_message`), under the
Emperor's capture. The test is
`the_event_messages_scenario_files_the_readers_own_capture`.

`message-display.mjs` (Linux, both sides) checks the exact text, and the
picture, title image and rail (Display lit) are each 0 pixels from A0.

The extra row pushed some rows below the list's nine visible rows. The
gate's clicks on them now scroll the list first (`shown()`).
