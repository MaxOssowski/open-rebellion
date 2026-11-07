# Research Complete's Display opens the Encyclopedia (kind 4)

Date: 2026-11-07. Cells: CMD-08-C014 (navigate to object), CMD-11-C012
(Message Index tooltips, the 0x8031 variant).

## The original

The Message Index asks each message its kind, through the message's vtable
slot `+0x10`:

- `FUN_004697b0` (list mode) and `FUN_00469de0` (one message) give Display
  (`0x67`) one of two faces. For kind 3, or kind 5 with a target
  (`FUN_004ece60`), it shows `0x2916` / `0x2917` (Alliance) or `0x2918` /
  `0x2919` (Empire), labelled `0x8030` "Display". For kind 4 it shows
  `0x2d20` / `0x2d21` (Alliance) or `0x2d47` / `0x2d48` (Empire), labelled
  `0x8031` "Encylopedia" (sic).
- `FUN_00467f10` case `0x67`: kinds 3 and 5 go to the message's object
  `+0x5c` (`FUN_00429440`). Kind 4 opens the Encyclopedia on the message's
  `+0x60` (`FUN_0041d6b0` → `FUN_00429f30`). Then it closes the index.

Which messages are kind 4 was not traced before. The message classes'
vtables run from `0x65b0b0` to `0x65bc10` (`story-message-classes.md`).
Reading slot `+0x10` of each class's vtable in `REBEXE.EXE` (SHA-256
`b3fe3997...ed6ab`):

| Slot `+0x10` | Kind | Classes |
|---|---|---|
| `0x5a6730` | 3 | Future Jedi Discovered (`0x65b520`), the class at `0x65b408`, and most others |
| `0x48ad30` | 8 | `0x65b0b0`, `0x65b188` (advice), `0x65ba20` and four more |
| `0x48d640`, `0x4953a0` | 6 | `0x65b3c8`, `0x65b450`, `0x65b568`, `0x65b758` |
| `0x404c20` | 5 | `0x65bbd0` (the Mission Report question) |
| `0x497530` | 3 or 7 | `0x65b870`, the Assault Summary reports |
| `0x492390` (`+0x84`) | 3 or 5 | `0x65b6c8` Mission Foiled (its constructor `FUN_00491190` sets 3), `0x65b710` Mission Report (`FUN_004927c0` sets 3 or 5) |
| `0x5a6700` | **4** | `0x65bb40` only: Research Complete (`FUN_00499aa0`) |

So Research Complete is the only kind-4 message. Its `+0x60` is the
researched class, which `FUN_00499aa0` takes for the text (`0x7048`,
`0x7049`). The Future Jedi Discovered reading agrees with A0: the new Easy
Empire game's Future Jedi Discovered (day 107,
`.artifacts/a0-vm/research-display/08-future-jedi.png`) shows Display as
10520 (`0x2918`) at (551, 259), kind 3.

## The port

Before, Research Complete had no target, so Display stayed disabled. The
`message-display.mjs` gate asserted exactly that.

- `GameMessage::encyclopedia` holds the object. The research tally's
  Research Complete gets the class from
  `story_messages::research_complete_class`, which also gives the texts.
- `message_index::display_kind` gives `DisplayKind::Encyclopedia` for such
  a message. `window_button_specs` draws `0x2d20` / `0x2d47`, the tooltip is
  `0x8031` (`display_button_string`), and the press returns
  `MessageIndexAction::Encyclopedia(object)` and Close.
- The app opens the original Encyclopedia at the object
  (`open_original_encyclopedia_at`) and logs `[interface] message_index
  encyclopedia object=…`.
- The catalog's id for the class. The port's capital ship and fighter
  classes keep their bare record id. The original catalog keys them under
  CAPSHPSD's family `0x14` and FIGHTSD's `0x1c` (`0x14000043` Nebulon-B
  Frigate, `0x1c000007` TIE Bomber). A bare `0x07` had resolved to
  `0x3c000007`, Imperial Commandos. `encyclopedia::class_entry` adds the
  family, and `research_complete_encyclopedia_object` uses it.
- The same fix applies to the object menus: a ship's or squadron's
  Encyclopedia (`original_encyclopedia_object`) now gives the class's
  catalog id. Before, a TIE Bomber squadron's bare `0x07` would have
  resolved to Imperial Commandos too; no gate opened one.
- The EventMessages fixture's Research Complete names the reader's first
  ship research class: the Nebulon-B Frigate (Alliance) or the TIE Bomber
  (Empire).
- `GameMessage` is not saved, so the save format is unchanged.

## Not compared with A0

No A0 capture of a Research Complete message exists yet. A player's side
researches only through Design missions, and the VM's Empire game had none
running.

## Browser

`message-display.mjs` (Linux, both sides) chooses Research Complete and
finds Display drawn as `0x2d20` / `0x2d47`, every opaque pixel. On a fresh
fixture, it presses Display and finds the index closed. The log reads
`object=0x14000043 topic=Some(..)` (Alliance) and `object=0x1c000007
topic=Some(..)` (Empire), and the Encyclopedia shows the class's topic.

