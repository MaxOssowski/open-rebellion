# Game speed: the real time of a day

Recovered 2026-10-31 from REBEXE.EXE (SHA-256 `b3fe3997...ed6ab`) and
measured in A0 (Windows 10 VM).

## The sub-tick

The game's state machine `FUN_0041dff0` (case 0, `FUN_0041e390`) runs one
scheduler sub-tick (`FUN_0051dea0`, which raises `+0xc`) each time
`FUN_004010d0` reports that the game timer has elapsed:

```c
// FUN_00415e20 (via FUN_004010d0)
if (*(int *)(param_1 + 0x10c) == 0) {
  fVar1 = (float10)FUN_005fbbe0();          // seconds since the last reset
  if ((float10)_DAT_006588f0 < fVar1) {     // float 0.25 at 0x6588f0
    *(undefined4 *)(param_1 + 0x10c) = 1;
  }
}
```

- `FUN_005fbbe0` returns seconds: `timeGetTime()` milliseconds times the
  float 0.001 at `0x66deac`, or the performance counter times its period.
- `FUN_004010f0` clears the flag and resets the timer (`FUN_005fbbb0`)
  when the sub-tick runs.
- A sub-tick therefore takes 0.25 s plus the time the loop needs to come
  back to case 0.

A day has as many sub-ticks as the speed's rate (side `+0xc4`, from
`FUN_00487eb0`: Very Slow 600, Slow 60, Medium 12, Fast 4). So a day lasts
at least 150 s, 15 s, 3 s or 1 s.

## A0

`.artifacts/a0-vm/speed-timing/` holds the day readout captured every
10 s, 15 s or 30 s:

| Speed | Days | Seconds | Seconds a day | Rate x 0.25 s |
|---|---|---|---|---|
| Fast | 398 -> 422 | 30.1 | 1.26 | 1.0 |
| Medium | 434 -> 448 | 45.1 | 3.22 | 3.0 |
| Slow | 454 -> 458 | 60.1 | about 15 | 15.0 |

The extra quarter of a second or so each day is the day's own work in the
VM: the state machine's other cases between one day's sub-ticks.

## The port

`rebellion_core::tick::ORIGINAL_RATE_UNIT_SECONDS` was an inference of
0.1 s (Very Slow one day a minute). It is now 0.25 s. port: the port's
clock has no per-day work time, so its days are the bare rate x 0.25 s.
