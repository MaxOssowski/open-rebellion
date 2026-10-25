# The tactical voice announcer (P24)

Date: 2026-10-25.

## Source

### Original recordings

The A0 recordings from the Windows 10 guest stay in the ignored VM folder;
only their results are cited here. The original's voice banks (VOICEFXA,
VOICEFXE) and TACTICAL.DLL sounds were extracted with `dat-dumper` to a
scratch folder and matched against the recordings by FFT normalised cross
correlation at 11 025 Hz (numpy in a scratch venv; nothing added to the
project).

- **TAC-01-C012** (Empire, with a Death Star; Take Command at 9 s, resume at
  36 s):

  | Time | Sound | ncc | Meaning |
  |---|---|---|---|
  | 14.08 s | 15001 | 0.96 | event 0x9a, battle ready |
  | 20.09 s | 15131 | 0.87 | 0x11c, Death Star fighter screen too thin; the battle is still paused |
  | 39.74 s | 15124 | 0.97 | 0x115, Death Star under attack |
  | 53.72 s | 15132 | 0.67 | 0x11d, attack continuing |

  From 63 s TACTICAL event `0x0e` (turbolaser) repeats every 1–3 s.
- **Coruscant** (Empire, no Death Star): 15001 at 9.7 s and no other line.

### `FUN_005ada10(elapsed)`

`FUN_005ada10` runs on each manager update (`0x005aad29`), paused or not.
Its state is `+0xe94`, its countdown `+0xe6c` and the last result `+0xe68`.

- **States 0–3** play the opening table `+0xe74`. For the Alliance it is
  0x20, 0x90, 0x93, 0x94; for the Empire, 0x9a. The gap is
  `DAT_0066c238` (6000 ms). The countdown starts at 0, so the first line
  plays at once.
  - **State 0** goes to state 1 for the Alliance facing an operational Death
    Star (`FUN_005ba420`), otherwise to 1000.
  - **State 1** goes to state 2 (0x93, shield) when the Death Star's `+0x70`
    shield is up. It goes to state 3 (0x94, fighter screen) when the player
    has groups, `FUN_005ad930` is shut, no run is under way and there are no
    more Alliance than Imperial groups (`+0x988`, `+0x168`). Otherwise it
    goes to 1000.
  - **States 2 and 3** go to 1000.
- **State 1000** waits 6000 ms, then goes to state 4.
- **State 4** watches `FUN_005ad930`.
  - When it opens: 0x92 for the Alliance, or 0x11c for the Empire with
    groups of its own.
  - When it shuts: 0x94 for the Alliance, if both sides have groups, no run
    is under way and no player group is on order 6.

The port's `queue_battle_ready_voice` already played the opening line at
Take Command. It now also takes state 0's step. `advance_voice_announcer`
runs every frame of the combat phase, paused or not.

`attack_death_star_open` is `FUN_005ad930` without the mission control's
selection. The Attack Death Star control now uses it.

## Still open

- **0x11d.** `FUN_005d0340` (approach, result state 4) plays the group's
  "started" line for the Alliance player and 0x115 for the Empire.
  `FUN_005cfec0` (state 5) then plays "committed" or 0x11d. The port enters
  state 5 at once and does not model the 4→5 gap. TAC-01-C012 shows 14 s
  between 0x115 and 0x11d, so 0x11d is not played yet.
- **hyp: side lists.** The side lists `+0x64` of the manager's `+8`/`+0xc`
  objects are read as the side's fighter groups.
- **State 0's `+0x8b4 == 2` guard** is not modelled.
- **Groups in `+0x170` state 10** are counted as gone only when they are
  dead.

## Tests

- `the_empire_hears_its_fighter_screen_is_thin_six_seconds_after_battle_ready`
  reproduces the TAC-01-C012 timing: 15001, then 15131 at 6000 ms, while
  paused.
- `the_alliance_hears_the_death_star_warnings_in_order` covers 14001,
  14113, 14117/14116, 14115 and 14117.
- `the_death_star_announcer_keeps_quiet_when_its_conditions_fail` covers the
  guards.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2762 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| Release WASM SHA-256 | `1c21ee73b8b1be818cd9fcae33acb7e3077368cae5e5c517b64c8d06b4a4533c` |

`cargo mutants`: every announcer mutant is caught after the guard tests. The
survivors left are the two lines in `draw_tactical_view` that pass the frame
time.
