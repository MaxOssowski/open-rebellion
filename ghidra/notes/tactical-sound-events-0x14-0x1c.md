---
title: "Tactical sound events 0x14..0x1e: who asks for them"
description: "The sound manager's events past the weapon hits: the superlaser beam 0x14, the selected unit's looping hum (slot +0x6c: fighter 0x15/0x16, capital ship 0x17..0x19 by full hull +0x3c8), the launch 0x1b and the opening arrival 0x1c/0x1e; 0x1a has no caller. Settled and ported 2026-11-16 for TAC-01-C012."
category: ghidra
created: 2026-11-15
updated: 2026-11-16
tags: [tactical, audio, sound-manager, TAC-01-C012]
---

# Tactical sound events 0x14..0x1e

Started 2026-11-15 for TAC-01-C012, from the typed corpus
(`~/Games/rebexe-decompile/decompiled-typed`) and REBEXE's `.rdata`. The
WAVE table (`FUN_005bae60`, `tactical.rs::original_audio_resource_base`)
gives events `0x14..0x1f` one WAVE each, 13054..13065.

## A0

`tac01-c012-corellia-take-command.wav` (2026-11-15-audio, sha256
`32afdc629e0b…`), Empire at Corellia, day 579. Offsets are seconds into
the WAV; Take Command was clicked at 10.04 s, the battle unpaused at
34.27 s and task force 1 selected at 54.52 s (`times.txt`).

- 10.99 s: TACTICAL 13064 (event `0x1e`), NCC 0.91 against the source,
  gain -4.6 dB, under the Battle Alert's score: the opening arrival. No
  13062 (`0x1c`), 13063 or 13065 anywhere in 9..56 s.
- 47.38 s: 13061 (event `0x1b`) alone, NCC 0.82: a launch.
- From 54.98 s, 0.46 s after task force 1 is selected: 13057 (event
  `0x17`) **looping**. Matching 0.4 s windows of the bed against a tiled
  13057 gives one implied start per cycle: 54.98, 58.38, 61.82, 65.23,
  68.60 s, a period of 3.40..3.44 s against the WAVE's 3.464 s (the VM's
  clock runs about 1 % fast against the 11025 Hz source here). Other
  effects ride on it: fighter-laser fire 13012/13013 (event 3) and 13015
  (event 4), 13034 (event `0x0d`) and 13048 (`0x12`).
- The bed stops while the battle is paused (silence from 190.7 s, through
  Game Options) and returns when it runs again (275.1 s), with Music Off.
- STRATEGY.DLL 621 and 1118 are not separate sounds. 621 (8.84 s)
  contains 13057 (NCC 0.94 of 13057 inside 621), and 1118 matches a
  tiled 13057 at 0.59: the earlier per-window matches were the hum. No
  STRATEGY WAVE is asked in battle: 1118 (`0x45e`) is a message sound
  (`FUN_00499de0` repaired / full strength, `FUN_004960f0` blockade; see
  `message-sounds.md`) and 621 (`0x26d`) is played by the side player's
  `Class_0065ad70_vtac` (`FUN_00486950`).

## The callers

| Event | WAVE | Caller | Meaning |
|---|---|---|---|
| `0x14` | 13054 | `FUN_005ba520` (event `0x44`) | the superlaser's beam starts; the sound needs the target still in the fight (`FUN_005ad870`), then `+0x894 = 1` and voice `0x91` when `+0x9ec` is 0, then the beam (`FUN_005c12d0`) |
| `0x15` / `0x16` | 13055 / 13056 | `Class_0066c028_vt6c` (`FUN_005b8df0`) | a fighter group's hum: `0x16 - (side +0x168 != 0)`, the Alliance being side 0 |
| `0x17` / `0x18` / `0x19` | 13057..13059 | `Class_0066c140_vt6c` (`FUN_005b0100`) | a capital ship's hum by its full hull `+0x3c8`: below 1100.0 (`0x66c47c`) `0x19`, below 2000.0 (`0x66c480`) `0x18`, else `0x17`; both sides alike. The same thresholds pick its destruction sound 9/10/11 (`Class_0066c140_vtd0`, `FUN_005b0170`) |
| `0x1a` | 13060 | none | no `FUN_005ba980` call in REBEXE pushes `0x1a` (all 102 call sites scanned), and neither slot `+0x6c` returns it |
| `0x1b` | 13061 | `FUN_005a9170`, from `FUN_005b8c70` (event `0x19`, `FUN_005b8b10`) | a squadron launches from its carrier: placed at the carrier, the hangar's counts lowered, `+0x170 = 2` |
| `0x1c` / `0x1e` | 13062 / 13064 | `FUN_005a6d60` | the opening arrival, below |

**`+0x3c8` is the full hull.** `FUN_005b1770` returns `+0x3cc * x /
+0x3c8`, the current over the full hull (`tactical-fighter-combat.md`,
`tactical-panel-text.md`); `FUN_005b05c0` copies both from the unit's
setup record (`FUN_005b1c00`).

### The hum loops

`FUN_005ba980(event, param_3)` hands `param_3 == 0` to `FUN_005baa60`,
which stores it as bit 3 of the request's `+0x3c`:

- a request with bit 3 that cannot start now (`FUN_005bad50` returns 0)
  is dropped, where other requests wait in the queue;
- `FUN_005bad50` copies bit 3 into the play record (`+0x2c`, flags
  `+0x10`), and `FUN_00611580` passes `flags >> 3 & 1` to
  `FUN_00612630` -> `FUN_00613b50`, which calls
  `IDirectSoundBuffer::Play(0, 0, flags)` (vtable `+0x30`) with
  `DSBPLAY_LOOPING` (1) set from it.

So `FUN_005ba980(event, 0)` loops until stopped. Every caller but the
hum passes 1.

**Who asks it.** The tactical interface's states (`DAT_006bc464`, swapped
by `FUN_0059f350`) keep the hum's handle at `+0x50`. The unit is
`FUN_0059ed70()+4` (`FUN_00583c40`), set by `FUN_0051c9b0`:

- the enemy-unit state `0x66bba8` (constructor `FUN_005a1110`, slot
  `+0x3c` `FUN_005a11e0`) and the one-unit state `0x66bce8` (`FUN_005a1770`,
  not while paused) are built by a click on a unit (`Class_0066be30_vtc`,
  `0x005a0d10`: the other side's unit gets `FUN_005a1110`, one's own
  `FUN_005a1550`);
- the task-force state `0x66bc48` (`FUN_005a42e0`, not during the arrival,
  `+0x8b4` = 2);
- `0x66bbf8` (`FUN_005a5230`, the unit `FUN_005c6070`; hyp: the fighter
  group state).

The several-unit state `0x66bc98` asks no hum.

**What stops it.** Each frame, `FUN_0059f380` calls the state's slot
`+0x30` (`FUN_005a1400`, `FUN_005a17e0`, `FUN_005a4330`, `FUN_005a5270`,
`FUN_005a2e30`). With a handle, it stops and clears it (`FUN_005bab10`)
when the sound no longer plays (`+0x20` = 0) or the battle is paused
(`+0x8c8` not 0: `Class_0066c2d0_vt0` / `vt4` count pauses, and
`FUN_0059a530` steps the battle only at 0); with no handle and the
battle running, slot `+0x3c` asks again. A state that loses its unit
(`FUN_005ad870` = 0) gives way to the empty state, and the old state's
destructor (`FUN_0059e990`, base `0x66bd68`) stops the hum. Withdraw
does not stop it: A0 `tac01-withdraw-and-return.wav` keeps the bed.

### The opening arrival

`FUN_0059a530` calls `FUN_0059a6a0` -> `FUN_005aacb0` whether or not the
battle is paused. State `+0x8b4` 1 sets up the view (`FUN_005ae1f0`) and
moves to 2; state 2 runs `FUN_005a6d60` with the frame's time * 0.4
(`0x66c230`) until 1500.0 (`0x66c248`), then 4. On the first frame
(`+0xe9c` = 1 from the constructor `FUN_005a7500`, `+0xea0` = 0.0 =
`0x66c254`) each unit of the viewer's side (`+0x8e8`) not in a hangar
(`FUN_005b69f0`: `+0x170` = 1) asks `0x1c` when its slot `+0x20` is 1
(a fighter group), else `0x1e`, then flies in from behind.

## The effect channels

The limit is the sound manager's, not DirectSound's: `FUN_00613b50` only
sets the volume (vtable `+0x3c`) and calls `Play` on the request's own
buffer; it picks no channel.

`FUN_005bae60` gives each event a class (`+0x544` + 4·event): `0x20`
words of 2 (events below `0x20`), then `0xfe` of 1 (the voices from
`0x20`), `0x1f` of 3 from `0x11e`, and a few voices set to 1. Per class
it sets a limit (`+0x1c` + 4·class: 1, 1, **5**, 1) and a wait
(`+0x3c` + 4·class: class 1 8000 ms, class 2 **225 ms** = `0xe1`, class
3 3000 ms).

- `FUN_005baa60` (each request) first runs `FUN_005bab50`, then
  `FUN_005bad50`, which starts the request when `FUN_005bad00` counts
  fewer than the limit of its class among the sounds started; else the
  request waits (bit 3 clear) or is dropped (bit 3, the hum).
- `FUN_005bab50` (also twice a frame from `FUN_0059f380`) forgets the
  sounds that stopped playing (`+0x3c` bit 5, from `GetStatus` in
  `FUN_00612890`), drops a waiting request older than its class's wait
  (four times longer for class 1 while paused), and starts the waiting
  ones in order while their class has room.

So at most five tactical effects (weapons, hits, hums, launches,
arrivals, jumps) play at once, the hum taking one while it loops. The
arrival sounds last 1.78 s and 3.70 s, so at most five of them sound.

## The port (2026-11-16)

- `TacticalAudioEvent` has `SuperlaserBeam`, the five hums,
  `FighterLaunch`, `FighterArrival` and `CapitalArrival`
  (`capital_hum`, `fighter_hum`); `TACTICAL_SFX_ASSETS` and
  `dat-dumper --extract-tactical-sfx` stage 13054..13059, 13061, 13062
  and 13064.
- `start_death_star_beam` queues `0x14` when the target is in the fight;
  `launch_fighter` queues `0x1b`; Take Command calls
  `queue_arrival_sounds` (the player's capital ships, then its fighter
  groups out of the hangars, at most 5).
- `BattleSession::selection_hum` names the hum while a selection state is
  open (`selection_state_open`: set by a unit click, a task-force or
  fighter-group button and capital cycling; cleared by a click on empty
  space, a navigation point or a Ctrl join), the battle runs and the unit
  is in the fight. The application holds it as one looping playback
  (`AudioEngine::start_tactical_loop`) and stops it when it changes.
- port: the port opens the battle with the first player ship selected;
  that is not a selection state, so no hum until the player selects
  (A0: none before task force 1).
- `start_death_star_beam` also queues voice `0x91` (VOICEFXA 14114) for
  an Alliance player (`+0x9ec` = 0).
- `audio::TacticalEffectChannels` (2026-11-16) holds the five class-2
  channels in the application: every effect cue waits there, starts while
  fewer than five play (a WAVE holds its channel for its length, from its
  `data` chunk), and is dropped after 225 ms; the hum takes a channel or
  is asked again next frame. The browser gate
  `production-tactical-audio-routing` sees 5 of the fixture's 45 requests
  start and 40 dropped, then the hum start on task force 1, stop on
  pause, return, and stop on a click on empty space.

Open:
- the voice classes (1 and 3, one at a time, 8000 / 3000 ms waits) are
  not ported this way;
- the arrival's flight itself (`FUN_005a6d60` positions) is not ported.
