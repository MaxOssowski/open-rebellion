# The uprising incident's line stays off the Message Index (0x152)

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The original

- `SystemUprisingIncidentNotif` (`FUN_00512580`) posts `0x152`.
- The notification-to-message map (`FUN_00488de0`) gives `0x152` no
  message type; it maps its neighbours `0x14d`, `0x14e`, `0x14f`, `0x150`,
  `0x153`, `0x154` and `0x155`.
- So the incident has no message of its own. Its effects do: an injured
  character, a destroyed facility.

## What the port did before

The app filed "Uprising incident at X" on the Popular Support rail of the
holder's Message Index.

## Port

- The line stays in the app's log, off the Message Index (no rail).
- port: the log line is the port's.

## Gates (`f33`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `2d7903db80da2dcb6013e018b20ca62727c62a160ceaffa60fdc45ab7089cbdd`).
- The Fleet window suite passes 54 cases. Its `incident` case still sees
  "Uprising incident at Akrit'tar" in the log for both sides.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- No Rust test covers this; the change is the app's filing of one line.

Hashes of the committed files are in `SHA256SUMS`.
