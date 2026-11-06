# The Popular Support list against A0 (2026-11-06)

This covers CMD-08-C008 ("diplomacy").

## A0

`.artifacts/a0-vm/message-index/category-1-day228` shows the Alliance on day 228. The Message Index is on Popular Support Messages, with two rows:
- "Commenor Joins Enemy", chosen;
- "Corellia Joins Enemy" below it.

## The source

System Joins / Joins Enemy / Declares Neutrality is `FUN_00499760` (notification `0x100`): TEXTSTRA `0x7039` "{1:1} Joins Enemy" for a system joining the other side. It is filed on the Popular Support rail (`0x7a`, "Popular Support Messages").

## Now

**`system_joins_message`.** The app's building of the message is factored out into this function, and the economy's control change posts through it.

**Fixture.** The EventMessages fixture files Corellia, then Commenor, joining the Empire through the same function.

**Browser (`message-display.mjs`, Alliance, Linux Chromium).**
- The second category tab opens the Popular Support list.
- Its rows read "Commenor Joins Enemy" over "Corellia Joins Enemy", newest first as in A0.
- With row 0 chosen, each row's head (the category icon and the bar's start, 32 by 17) differs from A0 in **0 pixels** (`support_heads_a0` [0, 0]).
- The rows' text is the accepted text deviation.

The b96 batch ran with the fixture change: all 21 gates passed. `message-display.mjs` then passed with the new case.
