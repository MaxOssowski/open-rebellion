# The advisor's quips play their op-0x13 clips (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy". The quips were
scheduled since `../2026-10-29-advisor-reactions/`, but their clips ended
at once: the clip player did not play op `0x13`.

## The original

A message entry's record is `[op, clip record, a, b]` (`FUN_0042b1d0`).
The quips use op `0x13`:
- C3POACT `0x50..0x55` hold ALSPRITE records 10301..10306;
- IMP22ACT `0x4e..0x53` hold EMSPRITE records 11301..11306;
- each record names a definition, 900, 901 or 903..906.

`FUN_00403ed0(0x13)` clones `Class_006588a8`. Its vtable, read from
REBEXE, against op 4's `Class_00658520`:

| Slot | Op 4 | Op `0x13` |
|---|---|---|
| `+0xc` set-up | `FUN_00404220` | `FUN_00412a90`, the same body |
| `+0x14` action | `FUN_00403ef0` (empty) | `FUN_00403ef0` (empty) |
| `+0x18` step | `FUN_00404360` | `FUN_00412b80` |
| `+0x1c` load | `FUN_00404440` | `FUN_00412c40` |
| `+0x20` end | `FUN_00404730` | `FUN_00404730` |
| `+0x24` stop | `FUN_00404740` (`+0x64 = 1`) | `FUN_00404740` |
| `+0x2c` op | 4 | `0x13` |

The two loaders' decompiles are identical, instruction for instruction.
Both read the same definition record: sound, WAVE and slot, frame count,
first frame. The steps differ only once `+0x64` is set:
- op 4 stops its sounds and ends at once;
- op `0x13` lets its frames run out, then ends without waiting for its
  voice.

Neither class takes an action. No op-4 or op-`0x13` entry in either
`.SPT` carries one, through its own a and b or its record's.

The definitions are on disk:
- ALSPRITE: WAVE 1291, 9 or 16 frames from BMP 19600..19690;
- EMSPRITE: WAVEs 22001..22006, 16 frames from BMP 8700..8790.

Each WAVE is 0.1 s long (22050 Hz, stereo, 8-bit).

## The port

- `advisor_clip::OP_UNSTOPPED_DEFINITION` (`0x13`) plays as op 4 does.
- port: the stop (`+0x64`) belongs to message preemption, which is not
  ported, so the two ops play alike.

## Browser (`tools/interface-parity/agent-objectives.mjs`)

The gate gains a step after the Garrisons reaction:
1. It opens the Game Speed menu on the day readout and chooses Fast. The
   game opens paused, and Alt+keypad + does not step from Pause.
2. It answers each battle with the Battle Alert's Simulate and closes the
   results.
3. It waits for the quip's voice.

The port's generator rolls 6, 7, then 4 from its seed, so the third idle
end after fifty quiet days plays message `0x54` (Alliance) or `0x52`
(Empire).

Run `…T14-02-18-673Z` passed for both sides. The WASM hash is `82f13969…`.

| Side | Day | Voice | Battles simulated | `quip.png` sha256 |
|---|---|---|---|---|
| Alliance | 56 | `alsprite-dll` WAVE 1291 | 2 (Ithor) | `4d9194ca…` |
| Empire | 58 | `emsprite-dll` WAVE 22005 | 0 | `b7c8c9b7…` |

Each screenshot shows the droid in its quip pose. Screenshots stay
outside Git. The logs are `{alliance,empire}-advisor.log`.

## Tests

- `cargo test --workspace`: all pass.
- Changed:
  - `a_definition_starts_its_voice_adds_a_frame_a_tick_and_ends_after_its_voice`
    now plays both ops;
  - `an_unplayable_clip_ends_at_once_with_its_action` now uses op `0x14`,
    which is still not played, in place of `0x13`.
- New: `every_quip_clip_plays_each_of_its_frames_to_the_end`, which reads
  the staged sprite DLLs and is ignored by default. It passes with
  `--include-ignored`.
- Without the change, both the changed definition test and the new test
  fail.
- `cargo mutants --in-place --in-diff`: 3 mutants, 2 caught, 1 unviable.
