# Injured, Healed and the bounty attack against A0

Date: 2026-11-09. Covers EVT-01-C012 (injury), EVT-01-C013 (recovery),
EVT-01-C018 (Jabba and bounty hunters) and OBJ-03-C010 (the Personnel
Finder's injured row). Until now these messages had
unit tests only, with no A0 capture.

## A0

Windows 10 VM, original REBEXE (provenance in the A0 VM notes). The save
`test` (an unedited Alliance game, day 7) was loaded at Fast speed. Six
agents were sent on Incite Uprising missions: Han Solo, Jan Dodonna,
Wedge Antilles and Luke Skywalker to Coruscant, and Chewbacca and Leia
Organa to Bortras. The Message Index was kept on All. Every capture is a
lossless QMP screendump at 640x480, in
`.artifacts/a0-vm/health-2026-11-09/` (SHA-256 in `SHA256SUMS`):

| File | Shows |
| --- | --- |
| `00-list-day244.png` | the list: Chewbacca Captured, Leia Organa Injured, Leia's mission foiled |
| `01-leia-injured-day270.png` | Leia Organa Injured: "Leia Organa has been injured." |
| `02-yavin-leia-injured-mark-day281.png` | Yavin's Personnel page: Leia back home, the injury mark under her mini |
| `00-list-day372.png` | the list: Leia Organa Healed, Han Solo Attacked by Bounty Hunters |
| `03-leia-healed-day372.png` | Leia Organa Healed: "Leia Organa has been healed of all injuries." |
| `04-han-bounty-hunters-day372.png` | Han Solo Attacked by Bounty Hunters: "Han Solo was attacked by bounty hunters, who failed to capture him." |
| `05-saboteurs-selonia-day372.png` | Saboteurs Strike at Selonia (not used here) |
| `06-personnel-finder-captured-day839.png` | Personnel Finder: four characters "( Captured )" |
| `08-yavin-wedge-injured-mark-day944.png` | Yavin's Personnel page: Wedge back, injured, with the injury mark |
| `07-personnel-finder-wedge-injured-day954.png` | Personnel Finder: "Wedge Antilles - Averam ( Injured )" |

Han Solo and Wedge Antilles were sent on a second Incite Uprising at
Coruscant on day 839 so that a free character would be injured.

Each message window stands at (62, 49), the pointer outside it.

The A0 pictures:

- Injured: STRATEGY 1050 (`0x41a`) at (74, 82), every pixel.
- Healed: `0x41b` with Leia's portrait.
- Bounty attack: STRATEGY 1052 (`0x41c`), every pixel.

These are the port's choices: `message_pictures::injured`, `healed` and
`story_messages::bounty_messages`.

## The browser comparison

The Alliance EventMessages fixture now posts Leia's Injured message,
then her Healed message (`interface_test_fixture::injured_character`,
`push_character_health`). The fixture injures her only for the message,
so its world stays the same. The story fixture already posted the bounty
attack.

`message-display.mjs` (Alliance, Linux Chromium) opens each message from
the list and checks the title and text word for word. It then compares
three regions with A0: the picture (window 12..411 by 33..232), the
title image and the rail.

| Message | Picture | Title image | Rail |
| --- | --- | --- | --- |
| Leia Organa Injured | 0 | 0 | 0 |
| Leia Organa Healed | 0 | 0 | 0 |
| Han Solo Attacked by Bounty Hunters | 0 | 0 | 0 |

The text glyphs are not compared pixel for pixel. That is the font
deviation the owner accepted on 2026-10-30.

## The Personnel Finder's injured row (OBJ-03-C010)

A0 `07-personnel-finder-wedge-injured-day954` reads "Wedge Antilles -
Averam ( Injured )". The port builds the row in `personnel_finder.rs`
(`FUN_00465bb0`: the first of Captured, On Mission and Injured `0x8747`).
The PersonnelStates fixture now has a fifth Alliance character, injured
and free. `personnel-states.mjs` (Alliance) expects exactly one row that
ends in " ( Injured ) ", in the form "Name - System".

## Not covered

- The Empire's side of these messages, including Bounty Hunters Locate
  Solo. No Empire capture was made.
- Han Solo Captured by Bounty Hunters, Jabba's palace and the Rescue
  Attempt.
- The port's rule for when it posts Injured is still `hyp:`. It posts
  when an injury is added and when healing ends; the original builder may
  run on every change of the injury. The capture shows one Injured and one
  Healed message for Leia across 100 days, which agrees with the port.

## Tests

The new fixture helper is test scaffolding behind the
`interface-test-fixtures` feature. The browser gate is its test: without
the helper, `message-display.mjs` fails because the list has no "Leia
Organa Injured" row. No unit test covers the helper, so a mutant of
`injured_character` would be caught only by that browser gate. The texts
and pictures were already covered by unit tests in `story_messages.rs`
and `message_pictures.rs`.

## Batch b153

Workspace tests 3496 passed, fixture tests 139. On the first run the
host was overloaded (load average 21, with the A0 VM running), and three
gates timed out: agent-objectives, message-display and the tactical
matrix (140 of 148: four stalled timing samples, three hover hashes that
differed, one movie not fetched in time). With the VM paused they
were rerun on the same tree together with the new personnel-states
check: personnel-states, agent-objectives and message-display pass, and
the matrix `2026-11-09T10-37-52-618Z-2049367` passes 148 of 148. The other 20 gates passed on the
first run.
