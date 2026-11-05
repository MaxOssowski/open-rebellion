# The "Resume Game Play?" alert holds the command center, as A0 (2026-11-05)

This covers CMD-05-C001 ("paused"), a correction to a port assumption.

## A0

The VM's Expert Alliance game on days 243..284. The captures are in `.artifacts/a0-vm/speed2/`.
- `pause-resume-question`, `pause-modal-0-sector-close`: Pause from the day readout's menu opens the alert. Two clicks on the open sector window's close box (285, 43) leave the window open.
- `pause-modal-4-after-resume`: after the checkmark, one click on the same box closes it.
- `pause-modal-1-paused`..`3-readout`: paused again, two clicks on the Popular Support rail open no Message Index, and a right click on the day readout opens no speed menu.

## Before

`game_speed.rs` said the alert "is a child window, not a modal dialog, so the rest of the command center stays usable". Only the alert's own rectangle blocked the pointer.

## Now

- **The app.** While a pause is pending or holding, the app counts the command center as held (`modal_window_open` includes `clock.pause_requested()`), as it does under the Message Index. The alert's own input is enabled unless another modal window is open.
- **The hold layer.** `draw_pause_alert` lays a pointer-taking layer over the whole screen (`Order::Foreground`) and draws the alert above it (`Order::Tooltip`). So the windows' own right clicks, the object menus among them, reach nothing until the checkmark or Enter resumes play.
- **`personnel-orders.mjs`** (Linux, both sides, the abort case). With the alert open, a right click on the agent opens no object menu. Without the change the menu opened (`'object-menu'`), and the assertion failed. After the checkmark, the menu opens and Abort runs as before.
- **The b79 batch.** It ran all 21 Linux gates, and every one passed. Its in-diff mutants: 2 caught.
