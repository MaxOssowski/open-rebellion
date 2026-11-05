# The Empire's controls held, against A0 (2026-11-05)

This covers CMD-10-C004 ("Imperial pressed").

## A0

A new Empire game, day 3. The captures are in `.artifacts/a0-vm/controls-empire/`. Each control was held with the pointer moved off it, then released there (FUN_00602d30 keeps the pressed art while captured). The table lists the differing opaque pixels.

| Control | Rect | Held | Released off |
|---|---|---|---|
| Fleet Finder (0x12e) | (199, 434) 37×24 | 10017, 0 differing (`eh2`) | 10018, 0 (`eh3`) |
| Encyclopedia (0x131) | (465, 434) 35×24 | 10023, 20 differing (`ee1`) | 10024, 19 (`ee2`) |
| GID (0x132) | (519, 434) 37×25 | 10025, 0 differing (`eg1`) | |

The Encyclopedia's 20 pixels are its bitmap's column 35: the bitmap is 36 wide and the control's rect is 35, so the window clips it.

## Port

The port's Empire controls use the same pairs (`cockpit.rs`: 10017/10018, 10023/10024, 10025/10026).

`run.mjs` (the GID gate, Linux, scenario `galaxy`, 640×480) now checks the held Empire GID control against 10025, every opaque pixel. Its one key-colour pixel (0, 21) shows the shell. All four of the galaxy scenario's executions pass. Before, the gate only asserted that the held frame differed from the hover frame.

Gate run: 2026-11-05T22-34-16-630Z-2582353.
