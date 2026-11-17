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

The rate is side `+0xc4`, from `FUN_00487eb0`: Very Slow 600, Slow 60,
Medium 12, Fast 4 (its default case, an index outside 1..4, stores Very
Slow's index 1 with Slow's rate 60). `FUN_0051dea0` reads the larger of
the two sides' (the computer's side keeps the 1 that `FUN_00533300` sets,
since only the player's speed order `0x280` reaches `FUN_005305e0`).

Corrected 2026-11-17: a day has the rate plus one sub-ticks, not the rate.
`FUN_0051dea0` (the only writer of the day `+0x10`; its one call is
`FUN_0041e390` at `0x41e3e9`):

```c
+0xc += 1;
if (+0x24 == 0) +0x20 += 1;                     // the in-day count
else { +0x24 = 0; +0x20 = 0; +0x10 += 1; }      // the day rises
+0x1c = max(side A +0xc4, side B +0xc4);         // the rate
+0x24 = (+0x1c <= +0x20);
```

From `+0x20 = 0`, sub-ticks 1..rate count to the rate and the rate-th
sets `+0x24`; sub-tick rate + 1 raises the day and counts nothing. So a
day lasts (rate + 1) x 0.25 s: Very Slow 150.25 s, Slow 15.25 s, Medium
3.25 s, Fast 1.25 s, and `+0xc` rises rate + 1 times a day.

## A0

`.artifacts/a0-vm/speed-timing/` holds the day readout captured every
10 s, 15 s or 30 s:

| Speed | Days | Seconds | Seconds a day | Rate x 0.25 s |
|---|---|---|---|---|
| Fast | 398 -> 422 | 30.1 | 1.26 | 1.0 |
| Medium | 434 -> 448 | 45.1 | 3.22 | 3.0 |
| Slow | 454 -> 458 | 60.1 | about 15 | 15.0 |

The extra quarter of a second each day is the rollover sub-tick above,
not the day's own work: (rate + 1) x 0.25 s is 1.25, 3.25 and 15.25 s.
The 2026-11-15 audio run (Medium, idle) heard the advisor's clips every
75.15 s, its 300 sub-ticks (`FUN_00439bc0`): a sub-tick of about 0.2505 s,
so the loop's own work adds well under 1 % at every speed. That 75 s is
not a Very Slow day. No A0 capture has timed a Very Slow day; the rate
600 (`FUN_00487eb0` case 1) gives 150.25 s.

## The port

`rebellion_core::tick::ORIGINAL_RATE_UNIT_SECONDS` was an inference of
0.1 s (Very Slow one day a minute). It is now 0.25 s. Until 2026-11-17
the port's day was the bare rate x 0.25 s; now
`GameSpeed::sub_ticks_per_day` is the rate + 1 and `seconds_per_day`
its quarter-seconds, and the clock's sub-tick count (`GameClock::sub_tick`,
the advisor's and the messages' time) rises rate + 1 a day. port: the AI
still makes the rate's passes a day (`ai::passes_per_day`).
