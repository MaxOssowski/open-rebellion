# The game speeds' real time against A0 (CMD-05-C002..C005)

Date: 2026-10-31.

## A0

The A0 VM's Empire game was set to Fast, Medium and Slow from the Game
Speed menu on the day readout. Its screen was captured every 10, 15 and
30 seconds. The host's clock times of the captures are in
`capture-times.txt`, and the captures' hashes in `a0-captures.sha256`.
The day readouts read:

| Speed | Days | Seconds | Seconds a day |
|---|---|---|---|
| Fast | 398 to 422 | 30.1 | 1.26 |
| Medium | 434 to 448 | 45.1 | 3.22 |
| Slow | 454 to 458 | 60.1 | about 15 |

## The original

`ghidra/notes/game-speed-timing.md`:
- `FUN_00415e20` runs a sub-tick once 0.25 s have passed. It uses the float
  0.25 at 0x6588f0, and `FUN_005fbbe0` measures seconds as `timeGetTime`
  times 0.001.
- A day has the speed's rate of sub-ticks: Very Slow 600, Slow 60, Medium
  12, Fast 4.
- So a day lasts at least 150 s, 15 s, 3 s or 1 s. A0's extra quarter
  second or so each day is the day's own work.

## The port

`ORIGINAL_RATE_UNIT_SECONDS` was an inferred 0.1 s, which ran every speed
2.5 times too fast. It is now 0.25 s. The clock tests now use the
recovered day lengths. Each test's real times are scaled by 2.5, so it
asserts the same behaviour against the new day.

Very Slow was not timed in A0; at 150 s a day that would take minutes.
Its length follows from the same rule and the rate 600.

## Linux browser

Measured on Linux Chromium from the `[ai] day=` console lines' wall-clock
spacing:
- **Medium.** `personnel-orders.mjs` (abort case, both sides) asserts each
  day 2.6..3.4 s apart. It measured 2.98, 3.00, 2.98 and 3.00 s.
- **Fast.** `agent-objectives.mjs` (both sides) asserts a median spacing of
  0.85..1.15 s. It measured 0.999 and 1.000 s.

port: A0's days run about 0.2 to 0.26 s longer, from the original's own
per-day work in the VM. That work time depends on the machine; the port's
clock has none.

The other interface gates were rerun with the new day length:
fleet-window, personnel-orders, agent-objectives, agent-advice,
agent-build, advisor-events, briefing, sector-quadrants, mission-dialog,
status-window, build-selection, message-display, and the GID catalog
(152/152). All pass. fleet-move fails as before this change: its probe
finds no bare map point under the confirmation window.
