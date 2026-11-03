# A mission icon's Abort in A0 (2026-11-03)

This covers surface ledger cell OBJ-12 C007, "aborted".

A0 ran the Alliance save at days 308–312 (Windows 10 VM). The captures are `.artifacts/a0-vm/personnel-states/37-*` (ignored); the hashes are in `a0-sha256.txt`.

## What A0 shows

- **`37-sullust-mission-menu-abort-grey`.** The menu of the mission icon over Sullust has Abort and Status grey.
- **`37-sullust-missions-window-empty`.** The icon's Missions window, "Mission at Sullust", lists no mission.
- **`37-mon-calamari-mission-menu-abort-highlighted`, `37-mon-calamari-abort-keyboard`.** Mon Calamari's mission icon menu has Abort enabled. It was chosen twice: once with the mouse and once with Down, Down and Enter.
- **`37-after-abort-no-question`, `37-after-keyboard-abort-no-question`.** Both times the menu closes, no question opens, and the missions stay.
- **`37-mon-calamari-missions-window`.** They are still there: two Facility Design Research missions with Adar Tallon.

## The original

The Abort order `0x250` uses vtable `0x669b70`, and its two checks differ.

**`+0x18`, `FUN_0051fe20`: the menu's enable.** It builds event `0x261` once for each mission (`+0x4c`, `FUN_005201c0` → `FUN_005535b0`, which checks no phase). It then walks the team in `FUN_00553770`:
- an empty team is refused with `0x16`, which is Sullust's grey;
- members of different sides are refused with `0x14`.

**`+0x1c`, `FUN_0051ff30`: the order's validator.** It runs `+0x18`, then `+0x50` (`FUN_005202d0`), then `FUN_00553960`. That function calls each event's `+0x1c` (`FUN_0054e240`), which calls the mission's `+0x1c4`. `+0x1c4` is `FUN_00521700`, which calls `+0x1c0`, `FUN_00521600`. That last check refuses phase 4 with `0x40` and any phase but 8 with `0x41`.

`FUN_00487740` runs the validator first. On a refusal it hands the status to `FUN_00487c90` (the advisor's reaction, no text) and asks nothing. Only an order that passes reaches `FUN_00487cc0`, which asks for `0x250`.

So A0's Mon Calamari missions were offered Abort but were not in phase 8.

## The port

Before this change, the menu offered Abort only when every mission passed `abort_allowed` (phase 8), so an ordered mission's Abort was grey.

Now `abort_missions` (`main.rs`) returns the side's visible missions, and the menu offers Abort when there are any. When Abort is chosen:
- if any mission is not in phase 8, the order is refused (`command=0x250 ... status=refused reason=0x41`) and nothing asks;
- otherwise the question opens as before.

`personnel-orders.mjs`'s `abort` case runs on Linux for both sides:

1. An ordered mission's Abort is now drawn white and enabled. The old gate asserted it grey, and that expectation is changed on the A0 evidence above.
2. Choosing it logs the refusal, and no mission is aborted.
3. Two days later the running mission's Abort asks, and the checkmark aborts it.

Run `personnel-orders-2026-11-03T21-20-21-990Z-3639942`: pass.

## Open

- A0 was not captured with an Abort that asks, on a running mission. The question's art (TEXTSTRA `0x7052`, picture `0x40c`/`0x40d`) rests on `FUN_0049a350` and the port.
