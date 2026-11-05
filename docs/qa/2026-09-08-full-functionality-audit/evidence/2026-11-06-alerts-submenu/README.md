# The Message Alerts submenu against A0 (2026-11-06)

This covers CMD-07-C010 ("message filter submenu").

## A0

The day-628 Alliance game. The captures are in `.artifacts/a0-vm/personnel-captured/`.
- **`r2`.** A right click on R2-D2 opens "Messages" and "Message Alerts".
- **`r3`.** Over Message Alerts its submenu lists Loyalty, Fleets, Mission, Resources, Manufacturing, Defense, Conflict, Chat and Advice, each checked.
- **`r5`.** After a click on Fleets, its row is bare and the other eight are still checked. Fleets was turned back on afterwards.

## Port

`agent-advice.mjs` (Linux, the Alliance) now opens the submenu, finds all nine check cells white and clicks Fleets.
- **The log.** It reads `message_alerts mask=0x80 silent=true`, so FUN_00486fb0 flipped the Fleet rail's option.
- **The checks.** On reopening, Fleets is bare and the other eight are checked, as A0's `r5`.
- **The look.** The submenu stands where A0's does and reads the same, apart from the accepted text smoothing.

Gate run: agent-advice-2026-11-05T23-02-39-048Z.
