---
title: "Message sounds: what a message plays when it is displayed"
description: "FUN_00469de0 plays a message's +0x30 (STRATEGY.DLL) and +0x32 (the side's sprite DLL) and stops the last display's; the builders that set them"
category: ghidra
created: 2026-11-15
updated: 2026-11-16
tags: [messages, sounds, strategy-dll, message-window, preemption]
---

# Message sounds

Read 2026-11-15 from the typed corpus
(`~/Games/rebexe-decompile/decompiled-typed`), for CMD-07-C011 against A0
`cmd07-c011-preempt-attempt.wav` (2026-11-15-audio).

## The display

`FUN_00469de0` shows a message in the Message window (mode 2). After the
picture (`FUN_0046a320`, `message-pictures.md`) it:

1. reads the message's two shorts `+0x30` and `+0x32`;
2. calls `FUN_0046a6e0`, which stops the window's two sound objects
   (`+0x128`, `+0x140`) when their ids (`+0x12c`, `+0x144`) are not 0
   (`FUN_00610c80`; `FUN_00610c90(.., 1)` when the object's `+0x10` bit 5,
   the playing bit, is set);
3. keeps the new ids, `+0x30` with DLL slot 7 and `+0x32` with slot 9, and
   plays each that is not 0 (`FUN_00610c30` → `FUN_006115f0`).

Slot 7 is `strategy.dll` (`FUN_00421c70`: `FUN_005ff020(.., "strategy.dll",
7)`); slot 9 is the side's sprite DLL (`FUN_0042d650`). `FUN_0046a6e0` has
no other caller, so only the next display stops a message's sounds.
hyp: closing the window lets them play out (its destructor is not read).

A0 (`cmd07-c011-preempt-attempt.wav`, sha256 `9af84b4bb048…`): three arrow
presses at 1.5 s and 1.7 s apart restart STRATEGY WAVE 1122 at 46.65,
48.29 and 50.03 s (NCC 0.68/0.41/0.65 on the first second), and only the
last plays to its end (53.9 s = 50.03 + 3.88).

## Who sets them

The base constructor `FUN_004c4d90` zeroes both. Builders that set `+0x30`
(and how many writes of `+0x32`, a speaker's voice from the sprite DLL):

| Builder | `+0x30` | `+0x32` writes |
|---|---|---|
| `FUN_0048b2e0` advice (code `0x2a`) | `0x461` side 1, else `0x462` (written as `param_1 + 0xc`, an `int*`) | 0 |
| `FUN_00498e90` the other side's message (code `0x29`) | `0x461` / `0x462` | 0 |
| `FUN_0049a350` | `0x465`, `0x461 + (side != 1)`, `0x463 + (side != 1)` | 0 |
| `FUN_0048be60` yard idle | `0x44d` | 0 |
| `FUN_0048e940` disaster / resource change | `0x44d`, `0x44e` | 0 |
| `FUN_00499130` | `0x44f` | 0 |
| `FUN_00499760` system joins | `0x450`, `0x451` | 0 |
| `FUN_00498cb0` near uprising | `0x452`, `0x453` | 0 |
| `FUN_00499460` uprising | `0x454`, `0x455` | 0 |
| `FUN_0048b9a0`, `FUN_0048bbf0`, `FUN_00498970` saboteurs / shortfall | `0x456` | 0 |
| `FUN_00497940` new unit deployed | `0x457 + (side != 1)` | 0 |
| `FUN_00499de0` repaired / full strength | `0x45e`, `0x457 + (side != 1)` | 0 |
| `FUN_0048abc0`, `FUN_00499aa0` research | `0x459` | 0 |
| `FUN_004981c0` arrivals | `0x45a`, `0x45d`, `0x45b + (side != 1)` | 0 |
| `FUN_004960f0` blockade | `0x45e` | 0 |
| `FUN_00495e90` evacuation losses | `0x45f`, `0x460` | 0 |
| `FUN_00491060` informants | `0x44c` | 0 |
| `FUN_00490340` captured / escaped | `0x44c`, `0x469`, `0x478` | 7 |
| `FUN_0048df20` | `0x46a`, `0x46b`, `0x46e`..`0x471` | 6 |
| `FUN_0048d190` Luke to/from Dagobah | `0x46c` | 1 |
| `FUN_0048dc50` | `0x46d` | 1 |
| `FUN_0048e3c0` | `0x472`, `0x473` | 2 |
| `FUN_0048b050` assassins, `FUN_0048fdf0` | `0x474` | 0 / 1 |
| `FUN_0048fa00` bounty hunters | `0x475`..`0x477` | 2 |
| `FUN_0048b6e0` Jabba | `0x479`, `0x47a` | 1 |
| `FUN_0048ce80` the Emperor at Coruscant | `0x47b` | 1 |
| `FUN_0048ae40`, `FUN_0048c4c0`, `FUN_0048c9c0`, `FUN_0048d0f0`, `FUN_0048e1d0`, `FUN_0048ed80`, `FUN_0048f1e0`, `FUN_004955b0` | `0x47c + (side != 1)` | 0..7 |
| `FUN_004927c0` mission reports, `FUN_00491500` | `0x454`, `0x455`, `0x47e`..`0x480`, `0x47c + (side != 1)` | 12 / 7 |

## The port (2026-11-15)

- `message_sound::MessageSoundPlayer` is `FUN_00469de0`'s stop-then-play;
  `MessageLog::display_message` queues a displayed message's sounds, and
  the app plays them at the effects volume through the advisor-voice
  engine.
- `tools/stage-ui-assets` stages STRATEGY.DLL's 66 WAVEs and the runtime
  pack carries them as `advisor/strategy-dll/<id>.wav`.
- 2026-11-16: every builder above with a port counterpart sets its sounds
  (`MessageSounds::*` in `message_sound.rs`, one constructor per builder).
- `+0x32` is `FUN_004c4990(list)`: slot 9's `RT_RCDATA` `list` is a count
  and that many WAVE ids; a count above 1 draws one (`FUN_0041cd80`).
  ALSPRITE and EMSPRITE hold the same lists `0xc8..0x139`; `0x136`
  (`FUN_0048f1e0`, Vader training) is missing, so that voice is 0. The
  port draws when the message is filed (`MessageLog::push`), from its own
  generator (port: the original's is global).
- A major character's lists run from a base: Mon Mothma `0xf0`, Leia
  `0xdc`, Luke `0xc8` (`0xd2` after Dagobah, `+0xb0` bit 3), Han `0xe6`,
  the Emperor `0x104`, Vader `0xfa`. Personnel Arrive takes the base,
  a Mission Report base + 2 (success) or + 3 (failure; Mission Foiled
  too), an escape base + 6, a healing base + 7.
- `FUN_0048b6e0` and `FUN_0048e3c0` test Luke's (`FUN_004f3e50`) bit 3,
  not a side record's.
- No port counterpart, so no sound: `FUN_00498e90` (the other side's
  message, multiplayer, out of scope) and `FUN_0049a350` (the order
  confirmation, whose window `FUN_0044f180` plays nothing).
- Ported 2026-11-16 with their texts, pictures, rails and triggers:
  `FUN_0048ae40` Recruitment Done (class `0x65b0f8`: mask `0x10`, kind 8;
  posted when a Recruitment takes the side's last pool character,
  `FUN_0055fc80` -> `FUN_0052f590`), `FUN_00499de0` Capital Ship
  Repaired (class `0x65bb88`: kind 3, mask `0x80` for a ship; posted
  when a repair clears the ship's damage), `FUN_00491500` codes 6..10
  and `0xc` (Mission Failed / Aborted from the leader, `0x412 + (side !=
  1)` with the portrait), and the Research missions' reports (family
  `0x53`: picture `0x3f9`, `0x47f`). port: Squadron at Full Strength
  (`0x7040` / `0x7042`) has its texts and sound but no trigger, as the
  port's squadrons keep no strength below full.
