# Messages clear themselves after 25 days at Medium, against A0 (2026-11-05)

This covers CMD-08-C015 ("clear").

## A0

The VM's new Expert Alliance game, at Medium (the day readout's three of four bars). A script simulated each Battle Alert and every half minute opened All Messages and closed it again, which also walks the list (`FUN_0048a610(0)`). The captures are `.artifacts/a0-vm/message-expiry/`: `a0` on day 83, `q00`..`q35` on days 96..185. Each message below has its first and last day in the list and the first day it was gone:

| Message | First seen | Last seen | Gone by | Lifetime fits |
|---|---|---|---|---|
| Fleet Arrives at Averam (old) | ≤ 72 | 96 | 103 | 24..37 days |
| Saboteurs Strike at Averam | 83 (absent on 72) | 103 | 108 | 20..36 |
| Saboteurs Strike at Corsin | 103 (absent on 96) | 122 | 128 | 19..31 |
| Fleet Arrives at Umgul | 108 (absent on 103) | 133 | 138 | 25..34 |

Every interval holds 25 days, the base rule's 300 sub-ticks at Medium's 12 a day (`Class_0065c538_vt30`, `FUN_004c4fe0`; `ghidra/notes/message-expiry.md`). The faster clears seen earlier in that game (day 43 → gone by 62) came before the speed was set to Medium, and fit Slow's 5 days.

## Port

`message-display.mjs` (Linux, the Alliance), at the end of its run:
1. It closes the index and sets Medium from the day readout's menu.
2. It opens and closes the Mission list on days 19 and 28.
3. On day 19 every fixture message stands (12 rows). On day 28 Fleet Arrives, Emperor Palpatine Captured and the rest are gone. Only the Diplomacy mission's freshly re-posted questions remain.

The rule itself (`message_log.rs`, unit tests) was ported on 2026-11-04 (`2026-11-04-message-expiry/`).

Gate run: message-display-2026-11-05T15-17-43-401Z.
