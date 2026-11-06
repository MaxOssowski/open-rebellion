# A running mission's Abort question against A0 (2026-11-06)

This covers OBJ-12-C007 ("aborted") and OBJ-12-C012 ("cancel").

## A0

The A0 run is the Empire game in the Win10 VM, day 277; the captures are in `.artifacts/a0-vm/mission-reports/`.
- **`mission-menu-day277`.** In the Missions window "Mission at Chandrila", a right click on Thrawn's Diplomacy opens the menu Encyclopedia, Status and Abort.
- **`abort-question-day277`.** Abort asks "Are you sure you want to abort this mission?" (0x7052) over the Empire's picture, with the check and X buttons. The X kept the mission.

## Browser (`personnel-orders.mjs` `abort`, Linux Chromium)

The running mission's Abort asks the same question. On the Empire's side, the confirmation's picture (165..555, 70..280) and its two buttons (500..552, 293..367) differ from A0 in **0 pixels**. The text is the accepted deviation. The case's checkmark then aborts the mission, as before.

This gate change ran alone after the b99 batch, which passed all 21 gates.
