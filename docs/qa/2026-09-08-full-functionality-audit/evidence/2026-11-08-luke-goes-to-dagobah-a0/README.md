# Luke Goes to Dagobah against A0

Date: 2026-11-08. Closes EVT-01-C017.

## A0

A new Alliance game in the Windows 10 VM (`alliance-game2`). The captures
are in `.artifacts/a0-vm/alliance-game2/`:

| File | SHA-256 |
|---|---|
| `luke-goes-to-dagobah.png` | `8578d48f80b83f2a88ecaac19b4cda3d3959eedf572ccb7faff2cae92c84d724` |
| `index-dagobah-chosen-day454.png` | `632c9f0aae66213f7b5912031f9a554cc522e862e81e65c060c64fea4eeff3e0` |

The message reads "Luke has been sent to Dagobah to be trained by Yoda."
This is `FUN_0048d190`, class `0x0065b380`, TEXTSTRA `0x71b8` / `0x71b9`,
over picture `0x421` (Yoda and Luke).

Display is grey both in the message view and with the row chosen in the
list. The class's `+0x10` is `0x48ad30` (kind 8), which the port follows
since b132 (`GameMessage::plain`).

The same game also gave these captures, kept for later:

- `informants-duros-day111.png`
- `han-solo-captured.png` ("Han Solo was captured by the Empire at Yavin.")
- `kothlis-joins-enemy.png`

## Browser

`message-display.mjs` (Linux, Alliance) loads the story fixture
(`fixture-code` 56, StoryMessages) and opens "Luke Goes to Dagobah", which
the fixture files through `story_messages::luke_goes_to_dagobah`. It checks
the exact text, and the picture, title image and rail (Display grey) are
each 0 pixels from A0.

Luke Leaves Dagobah has no A0 capture.
