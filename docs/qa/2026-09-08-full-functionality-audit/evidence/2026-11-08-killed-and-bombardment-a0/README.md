# Jan Dodonna Killed and the Empire's bombardment, against A0

Date: 2026-11-08. Closes EVT-01-C016 and EVT-03-C003.

## A0

Both captures come from a new Alliance game in the Windows 10 VM
(`.artifacts/a0-vm/alliance-game2/`):

| File | SHA-256 |
|---|---|
| `jan-dodonna-killed.png` | `9a79158cd25c41e41ea6f7c855dcf3f0426e4f943479a52b0d84113e6f731d7a` |
| `orbital-bombardment-yavin.png` | `10fc6c1c681ccb7621345505640ce2efc399ff4a7107b0b998c44ed4ed22b9db` |
| `index-bombardment-chosen.png` | `4f6377455ad919c2ada65d430dd428323ddeb1e57d3997d8f8f2a104d3e5d304` |

- **Jan Dodonna Killed.** "Jan Dodonna has been killed." (`FUN_0048fdf0`,
  `0x7164` / `0x7365`) over the Alliance's coffin `0x42e`. Display is lit
  (kind 3).
- **Orbital bombardment of Yavin.** The Empire struck the Alliance's
  Yavin, seen by the Alliance: "Imperial ships have conducted an orbital
  strike on the Alliance system of Yavin" (`FUN_00496770`, `0x7098`). The
  title and body are in the attacker's green. The picture is `0x2b9b`
  (STRATEGY 11163, a system loss); sampled pixels match the bitmap
  exactly.

## Browser (Linux)

**`message-display.mjs` (Alliance).** The event-message fixture reports Jan
Dodonna killed through the app's `push_character_health`
(`interface_test_fixture::killed_character`). He stays alive in the world,
because the scenario's Future Jedi names him. The gate checks the exact
text, and the picture, title image and rail are each 0 pixels from A0.

**`fleet-window.mjs` 'bombardment' (Empire).** The Empire's General
Bombardment summary is checked against A0:

- the body reads A0's text;
- the title's ink is centred as A0's;
- the body's first line is at A0's left and top, within 2 px;
- the picture is compared with A0 in every pixel, but only when the
  port's strike also cost the system. The picture is chosen by the losses.

In the recorded run, the title's centre differs by 3 px and the body's top
by 2 px.

The Alliance's bombardment of an Empire system has no clean A0 capture.
The 2026-11-07 Umgul capture checks its text in a unit test.
