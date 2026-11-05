# The Manufacturing list against A0 (2026-11-05)

This covers CMD-08-C006 ("build").

## A0

A new Alliance game. A Construction Yard was ordered on Chandrila, then the game ran at Fast while the Manufacturing tab was read every 25 seconds. The captures are in `.artifacts/a0-vm/manufacturing-messages/q00..q15`, days 17..346.
- **"Construction Yard Idle on Womrik".** It is listed from day 61 and stays, as Facility Idle lasts while its manager idles.
- **"Construction Yard Deployed at Chandrila".** It is listed on days 149..193 and gone by 215.
- **Each row's icon.** It is STRATEGY 0x2a9c (Manufacturing, Alliance), 0 of its 186 opaque pixels differing at the row's (1, 1).

## Port

- **The titles.** The port's Facility Idle (0x71f8) and Units Deployed (0x7086) titles read the same, and `fleet-window.mjs` checks them in the browser.
- **The fixture.** EventMessages now also files "Construction Yard Idle on Womrik" on the reader's Manufacturing rail.
- **`message-display.mjs`** (Linux, the Alliance) opens the Manufacturing tab and reads the report so it shows read. The chosen row's icon 0x2a9c is 0 differing in the port and in A0 `q06`, and the row's 32 by 17 head is 0 differing from A0's.
- **Other gates.** `tooltip.mjs` still passes.
- **Unit test and mutants.** The fixture's unit test checks the new message's title, rail and side; `post_conflict_report`'s mutants are caught.

Gate run: message-display-2026-11-05T22-49-26-933Z.
