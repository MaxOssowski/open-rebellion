# The jump's flight and sound (P24)

Date: 2026-10-23.

## Source

- `FUN_005a6b40` draws a jumping unit at `z - side_z * k * (0.001 t)^2`,
  where `t` is its jump clock `+0x50` (up to 4000 ms). The factor `k`
  is one of four globals picked by `(id + slot +0x24) & 3`.
- Start-up code (`0x5a69b0`..`0x5a6a7c`) sets the four globals as a
  distance over the squared seconds left of the jump:
  - `DAT_006bc47c` = 400 / 4^2 = 25;
  - `DAT_006bc488` = 575 / 3.75^2;
  - `DAT_006bc484` = 650 / 3.925^2;
  - `DAT_006bc494` = 325 / 3.725^2.

  The inputs are `_DAT_0066c24c` = 4000, `_DAT_0066c270`/`274`/`278`
  = 250/75/275 and `_DAT_0066c258`..`264` = 400/575/650/325.
- On the first frame of the jump, `FUN_005a6b40` queues sound event
  `0x1d` when the unit's slot `+0x20` is 1. That is the fighter class
  (vtable `0x66c028`, `FUN_0040f340`). A capital ship gets `0x1f`
  (`0x66c140`, `FUN_006158b0`).
- `FUN_005bae60`'s table (event 0 at `+0x28e` = 13000, two words an
  event, matching the port's 13033 for event `0x0d`) gives these
  TACTICAL.DLL WAVE 13063 and 13065.

## Change

- `ORIGINAL_JUMP_ACCELERATIONS` and `original_jump_travel`: a jumping
  ship or fighter group moves along its side's exit direction each step.
- `TacticalAudioEvent::FighterJump` (`0x1d`, 13063) and `CapitalJump`
  (`0x1f`, 13065) are queued once, at the jump's start.
- The new sounds are added to:
  - `audio::TACTICAL_SFX_ASSETS`;
  - the `dat-dumper --extract-tactical-sfx` mapping;
  - `scripts/build-wasm.sh`.
- Before: a jumping unit stood still and was silent.
- hyp:
  - the port's list index stands for `id + slot +0x24`;
  - each event has one variant (their count words are not read).

## Owned-original audio check (not committed)

Extracted from the audited install's TACTICAL.DLL into the scratchpad.
The game audio itself stays out of Git (`web/data/` is ignored):

| WAVE | File | SHA-256 |
|---|---|---|
| 13063 | tactical_event_1d_0.wav (PCM 16-bit stereo 44.1 kHz, 314084 B) | `0c6737d20e65b15d3cd6014acc3773a98b3b0d58becacd407e92135cf5271199` |
| 13065 | tactical_event_1f_0.wav (PCM 16-bit stereo 44.1 kHz, 573356 B) | `535d00a12ebbd1e1843689531a3cd93f7ecadf2af9b170530d4902df41b87587` |

## Gates

- Workspace: 2726 passed, 0 failed.
- One expectation changed on this evidence:
  `a_withdrawing_fighter_group_flies_to_its_edge_before_it_jumps`. The
  jump's first frame moves the group 25 * 0.25^2.
- New tests:
  - `a_jump_flies_its_unit_away_and_sounds_once` (400 units in 4 s at
    k = 25; one cue each);
  - `the_jump_factors_reach_their_distances`.
- Mutants, with both packages' tests:
  - The first run tested 66 and missed 12: the three later factors and
    the variant-count arm.
  - The factor test closes them; the rerun caught all 66.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `e10c8adb910456a8fcb1a6c45bcc5f0adc6e86c538cf8633a3c2043f6c7e729a`
  (the tests changed after it; the code it builds did not).
