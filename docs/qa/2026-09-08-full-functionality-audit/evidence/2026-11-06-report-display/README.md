# A Mission Report's Display goes to its mission (2026-11-06)

This covers OBJ-12-C014 ("navigate to target") and CMD-08-C014 ("navigate to object").

## A0

The A0 run is the Empire game in the Win10 VM, day 234; the captures are in `.artifacts/a0-vm/mission-reports/`.
- **`report-before-display-day234`.** Thrawn's Diplomacy Mission Report at Chandrila is shown. The mission goes on, so the report asks to continue.
- **`report-display-missions-window-day235`.** Display (the rail's fourth button, tip "Display") closes the index and opens the Missions window "Mission at Chandrila". The window shows Diplomacy, Target Chandrila, and Thrawn on the team. It does not open the system's sector window alone.

## The source

`FUN_00467f10` handles `0x67`, which goes to the message's object through `FUN_00429440`. A Mission Report holds its mission (`+0x78`, `FUN_004927c0`), and `FUN_00429440` opens a mission in the Missions window (kind 11).

## Before

Display took every message to its system's sector window.

## Now

- **`GameMessage::mission`.** A Mission Report names its mission (`with_mission`, set by `apply_mission_result`).
- **`MessageIndexAction::Display`.** It carries the mission.
- **The app.** While the mission still runs, the app opens the Missions window at the system's quadrant point with the mission shown (`show_member` of its first member), as the Personnel Finder does.

**port.** An ended mission's report goes to its system. What `FUN_00429440` does with a mission that is gone is not traced.

## Tests

- `display_goes_to_the_chosen_messages_system_and_closes` now checks that a report carries its mission. Mutants on `display_target`: 9 caught.
- **Browser.** `message-display.mjs` (Empire; b98 ran the other 20 gates, and message-display passed alone after its case went back to the list first): the fixture's Mission Report names its running Diplomacy mission. Display closes the index, and the app logs `missions_window=true`.
