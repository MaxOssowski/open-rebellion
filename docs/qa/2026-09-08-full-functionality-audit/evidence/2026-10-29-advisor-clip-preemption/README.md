# The advisor's clips stop as the original's do: queueing and the skip's flush (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy", "przerywanie
komunikatów" (preemption).

## The original

- **Queueing stops the playing clip.** The player's slot 8
  (`FUN_00472930`) queues a clip, and when one is playing it calls that
  clip's stop (vt `+0x24`).
- **A skip flushes both players.** `FUN_0041da30` → `FUN_0042d620` →
  `FUN_00472ba0` for `0xdead` and `0xbeef`: the queue empties
  (`FUN_005f5ee0`) and the current clip is stopped. The clip is not
  dropped.
- **Each class stops its own way:**

| Op | Stop | Effect in its step |
|---|---|---|
| 4 | `FUN_00404740`: `+0x64 = 1` | `FUN_00404360` stops its sounds (`FUN_00404020(1)`, `FUN_00404040(1)`) and ends |
| `0x13` | the same | `FUN_00412b80` runs out its frames, then ends without waiting for its voice |
| `0x11`, `0x15` | `FUN_00412090`: `+0x50 = 0` | `FUN_00412260` ends; the end slot runs the action (or, for `0x15`, sets the hold's flag) |
| `0x14` | `FUN_00411d30`: `+0x54 = 1` | not read by `FUN_00411b60`; it keeps holding |

## The port (`advisor_clip.rs`)

- `ClipPlayer::queue` stops the current clip.
- `ClipPlayer::flush` (the skip's) empties the queue and stops the current
  clip on both players.
- `ClipPlayer::clear` stays for resets (a new game, another side).
- A stopped op-4 clip emits `ClipEvent::StopVoice`; the app stops that
  voice (`AudioEngine::stop_advisor_voice`) and logs
  `[advisor] clip voice stopped`.
- Before this change a skip dropped the clip at once and its voice played
  on.

## Browser (`tools/interface-parity/briefing.mjs`)

The skip case now asserts that the skip stops the first voice's clip and
cuts that voice. Run `…T15-40-17-947Z` passed for all four cases. The WASM
hash is `be6d83cb…`.
- The skip logs show `albrief-dll` WAVE 1155 and `embrief-dll` WAVE 1147
  stopped.
- The full cases stop nothing.

Regression runs:
- `agent-objectives.mjs` passed at `…T15-48-17-983Z`;
- `advisor-events.mjs` passed at `…T15-49-43-971Z`.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `a_new_clip_stops_the_playing_one_each_class_its_own_way`;
  - `a_flush_empties_the_queue_and_stops_the_playing_clip`.
- `cargo mutants --in-place --in-diff`: 20 mutants, 19 caught, 1
  unviable.
