# The Message Index rail's unread light against A0 (2026-11-05)

This covers CMD-08-C002 ("unread").

## A0

A new Easy Alliance game. Its opening posts the agent's advice and opens the Message Index on Advice. The captures are in `.artifacts/a0-vm/message-rail/`.

| Capture | State | Advice control (0x13d, 27 by 22 at (3, 284)) |
|---|---|---|
| `easy-index-open` | the index open, advice still unread | lit art 10067 |
| `easy-index-closed` | after Escape | resting art 10057 |

The Escape marks Advice read (`FUN_0048a610`). The day-7 save "test" shows the same before and after the index: lit with its advice unread (`message-delete/del1`), resting after reading (`agent-menu/alliance-agent-menu-day53`).

In every capture, each of the nine Alliance controls differs from its bitmap only at the pixels that share the bitmap's first DIB pixel's colour (2 to 6 per control). There the cockpit shell shows through: each control is a CoolStrobeButton (`FUN_00427270`), blitted keyed on its first pixel (`FUN_005fd170`).

## Before

The rail's lit and resting art were drawn opaque, so 4 pixels differed from A0 on the Advice control.

## Now

- **The rail art.** `bmp_cache` keys the rail art on its first pixel: Empire 10030..10038 and 10040..10048, Alliance 10050..10058 and 10060..10068.
- **Unit tests.** `the_message_rails_controls_are_keyed_on_their_first_pixel` fails without the change. `cockpit::an_unread_category_lights_only_its_rail_control` and the `message_log` unread-mask tests cover the light.
- **`agent-advice.mjs`** (Linux, the Alliance, Easy). With the index open, the Advice control is 0 differing from A0 `easy-index-open`. After Escape it is 0 differing from `easy-index-closed`; before the key it was 4 each.
- **The b73 batch.** It ran all 21 Linux gates. `run` first failed: its rail check wanted every pixel opaque, which A0 shows is wrong for the key pixels. Rerun with those pixels skipped (and counted), it passed, as did the other 20.
