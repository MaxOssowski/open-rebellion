---
title: "Tactical sound events 0x14..0x1e: who asks for them"
description: "The sound manager's events past the weapon hits: unit vtable slot +0x6c (fighter 0x15/0x16, capital ship 0x17..0x19 by +0x3c8), the order sounds that start it, 0x14, 0x1b and the jump-in pair 0x1c/0x1e. Started 2026-11-15 for TAC-01-C012."
category: ghidra
created: 2026-11-15
updated: 2026-11-15
tags: [tactical, audio, sound-manager, TAC-01-C012]
---

# Tactical sound events 0x14..0x1e

Started 2026-11-15 for TAC-01-C012, from the typed corpus
(`~/Games/rebexe-decompile/decompiled-typed`) and REBEXE's `.rdata`. The
WAVE table (`FUN_005bae60`, `tactical.rs::original_audio_resource_base`)
gives events `0x14..0x1f` one WAVE each, 13054..13065.

## A0

`tac01-c012-corellia-take-command.wav` (2026-11-15-audio, sha256
`32afdc629e0b…`), Empire at Corellia, day 579:

- the continuous bed from 55.0 s, 0.4 s after task force 1 is selected,
  is a dense mix. Best matches per 0.4 s window (NCC > 0.85, 33 windows
  in 55..120 s) are TACTICAL 13057 (event `0x17`, 16 windows, the first
  at 54.98 s from its start), STRATEGY.DLL WAVE 1118 (12), fighter-laser
  fire 13012/13013 (event 3) and 13015 (event 4), 13034 (event `0x0d`),
  13048 (`0x12`) and STRATEGY 621;
- 13061 (event `0x1b`) plays alone at 47.38 s, unprompted, and again at
  55.41 s;
- gain about -0.5 dB against the sources; the bed follows the effects
  slider and ignores Music Off.

## The callers read so far

| Event | WAVE | Caller | Meaning (hyp where marked) |
|---|---|---|---|
| `0x14` | 13054 | `FUN_005ba520` | after `FUN_005ad870` on the object `FUN_005a6340` finds; sets `+0x894 = 1`, then `0x91` (voice) when `+0x9ec` is 0 |
| `0x15` / `0x16` | 13055 / 13056 | `Class_0066c028_vt6c` (`FUN_005b8df0`) | a fighter group's slot `+0x6c`: `0x16 - (side +0x168 != 0)` |
| `0x17` / `0x18` / `0x19` | 13057..13059 | `Class_0066c140_vt6c` (`FUN_005b0100`) | a capital ship's slot `+0x6c` by its float `+0x3c8`: below 1100.0 (`0x66c47c`) `0x19`, below 2000.0 (`0x66c480`) `0x18`, else `0x17`; both sides alike |
| `0x1b` | 13061 | `FUN_005a9170` | after moving the object `+0x174`'s position (`+0x30..+0x3c`), then `+0x170 = 2` |
| `0x1c` / `0x1e` | 13062 / 13064 | `FUN_005a6d60` | for each unit, once while `+0xea0` equals `_DAT_0066c254`: `0x1c` when its slot `+0x20` is 1 (fighter group), else `0x1e`; hyp: the start of a jump, before `0x1d` / `0x1f` (`FUN_005a6b40`) |
| `0x1a` | 13060 | not found yet | |

Slot `+0x6c` is asked by order handlers that keep the request's handle
in their `+0x50` while it is 0: `FUN_005a11e0`, `Class_0066bce8_vt3c`
(`FUN_005a1770`, unless `+0x8c8`), `Class_0066bc48_vt3c` (`FUN_005a42e0`,
unless `+0x8b4` is 2) and `Class_0066bbf8_vt3c` (`FUN_005a5230`). Each asks
the object `FUN_00583c40` (or `FUN_005c6070`) returns and passes its event
to `FUN_005ba980(event, 0)`. Every other caller passes 1. `FUN_005ba980`
hands `param_3 == 0` to `FUN_005baa60`, which stores it as bit 3 of the
request's `+0x3c`. hyp: bit 3 marks a held (looping) sound whose handle
the order keeps to stop it later. That fits A0's bed: it starts when the
task force is selected, and it never stops through orders, pause and Withdraw.

Open:
- what `+0x3c8` is (a capital ship's size or speed);
- whether bit 3 loops, and who stops the `+0x50` handle;
- 0x1a's caller, and where STRATEGY 1118 / 621 come from in battle (they
  are not TACTICAL.DLL WAVEs; the tactical sound manager's ids start at
  13000).

## The port

`TacticalAudioEvent` covers 0..5, `0x0c`..`0x0e`, `0x11`..`0x13`, `0x1d`
and `0x1f`. None of `0x14`..`0x1c` or `0x1e` plays, so the port's battle
has no unit hum and no jump start sound.
