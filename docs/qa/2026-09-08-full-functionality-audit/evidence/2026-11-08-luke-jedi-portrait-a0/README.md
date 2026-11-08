# Luke's picture after Dagobah, and Future Jedi for the Alliance

Date: 2026-11-08. Adds to EVT-01-C014, which already passes.

## A0

An Alliance game in the Windows 10 VM, day 606..607. The captures are in
`.artifacts/a0-vm/future-jedi-alliance/`:

| File | SHA-256 |
|---|---|
| `00-index-day606.png` | `6180b161880c18534c5dd8e3c1f3289d380506cfd5dee96f173aa18eef26a4d1` |
| `01-future-jedi-dodonna-day607.png` | `0203966239e8a832a2609482a3588aec84af5314fa491d4d86b6f72e5d242fca` |

The All Messages list holds two Enemy Mission Foiled and one "Future Jedi
Discovered". Opened, it reads: "I have detected that Jan Dodonna has the
ability to use the Force.  Once I reach the rank of Jedi Knight, I should
conduct training in the use of the Force." This is body `0x717a`, so Luke is
below Force 100. The window is at (62, 49) and Display is lit (kind 3).

The portrait is Luke in black, the Jedi Knight's (STRATEGY 6224), not the
young Luke (6210).

## The original

- `FUN_0048f1e0` takes the trainer's portrait through `FUN_004c5000`. That
  function returns `(vtable +0x30 & 0xfff) + 0x1000`.
- Luke's `+0x30` is `Luke_vt30` (`FUN_0056f550`). It returns `0x850` when
  `Flagsb0 & DagobahCompleted` is set, and otherwise the class's picture
  (`FUN_004f63d0`).
- STRATEGY has only these major portraits: 6208..6211, 6224, 6272 and
  6273. 6224 is the Jedi Knight Luke. The compact image is 18512
  (`PORTRAIT_LUKE_SKYWALKER_JEDI_KNIGHT`).

## Port

- `rebellion_core::pictures::character_picture(&Character)` returns
  `0x4850` for Luke with `dagobah_completed`. For anyone else it returns the
  class's image.
- `story_messages::character_portrait_of` builds the story portrait from it.
- These callers that hold a character now use it:
  - the story builders (Dagobah, Future Jedi, Rescue Attempt, and others);
  - the Missions window and the mission dialog;
  - status rows;
  - the Defenses and Fleet windows;
  - the tactical contents and panel.
- Callers that only carry `(dat_id, major)` keep the class's picture:
  - message_pictures;
  - the assault summary's units;
  - `rebellion_core::tactical`.

The event-message fixture now has Luke, after Dagobah, find Jan Dodonna's
Force for the Alliance (`place_force_discoveries`).

## Tests

- `luke_after_dagobah_takes_the_jedi_knights_picture` (pictures).
- `luke_after_dagobah_speaks_with_the_jedi_knights_portrait`
  (story_messages).
- `the_event_messages_scenario_has_each_sides_trainer_find_a_force_user`.

Each fails without the change. The core mutants all fail (6 of 6 caught).

`message-display.mjs` (Linux, Alliance) checks the exact text, and the
picture, title image and rail are each 0 pixels from A0
`01-future-jedi-dodonna-day607`.
