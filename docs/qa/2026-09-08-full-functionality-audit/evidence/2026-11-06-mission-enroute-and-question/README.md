# A travelling mission's mark and a report's question, against A0 (2026-11-06)

This covers OBJ-12-C001 ("hyperspace"), OBJ-12-C015 ("mission hyperspace moving-star indicator") and OBJ-12-C011 ("continue").

## A0

The captures come from the Empire game in the Win10 VM and are in `.artifacts/a0-vm/mission-reports/`.
- **`missions-window-enroute-day276`.** Jerjerrod has been sent on Diplomacy from Ghorman to Chandrila. The Missions window "Mission at Chandrila" lists his mission. Chosen, its team shows his mini on the personnel backdrop, over the en route mark: grey panels with blue streaks.
- **`report-before-display-day234`.** Thrawn's Diplomacy report asks "Do you wish the mission to continue?" over the check and X buttons.

## Browser (Linux Chromium)

**`sector-quadrants.mjs` `missions-en-route`.** The Empire's quadrant fixture now sends Jerjerrod as the traveller. The case composes backdrop 11500, mark 11501 and his mini (`FUN_0042c3b0`). It then finds that composite in A0's team list at (290, 245) with **0 differing pixels**; the backdrop's key pixel shows the window through it and is skipped. The mark is still: the same pixels 1.5 s later. OBJ-12-C015's "moving star" is this still bitmap, as the earlier evidence reads it.

**`message-display.mjs`, the question.** The Empire's report's check (0x98) and X (0x99) cells, each 51 by 35, differ from A0's in **0 pixels**.
