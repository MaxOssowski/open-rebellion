# The Status window's buttons key out their blue, as A0 shows (P09)

Date: 2026-10-30. Tracker item "Panel i menu floty, rodzina okien statusu"
(the Status window family compared with the original).

## A0 (Windows 10 VM, Empire, days 179..186)

The captures stay outside Git; their hashes are in `a0-captures.sha256`
(`.artifacts/a0-vm/status/`). They show the Status window of eight
families, all opened through the object menu's Status:

| Capture | Family |
|---|---|
| `03-ship.png`, `04-ship-page2..5.png` | Capital Ship Status (an Alliance Corellian Corvette, all its rows) |
| `06-fleet.png` | Fleet Status (Alliance Fleet 1) |
| `12-troop.png` | Trooper Regiment Status (Stormtrooper Regiment) |
| `15-char-veers.png`, `16-char-veers-p2.png` | Character Status (Veers) |
| `33-emperor.png`, `34-emperor-p2.png` | Character Status (Emperor Palpatine, Jedi Master) |
| `21-fighter.png` | Fighter Squadron Status (TIE Fighter) |
| `23-battery.png` | Defense Facility Status (KDY-150) |
| `29-manufacturing.png` | Manufacturing Status (Training Facility) |

Neither the planet's Status (the object menu greys it for a system,
`FUN_0051d990`) nor the Personnel Finder's could be opened.

## What this stage settles

- **Close (10370).** It shows the background through its blue corners in
  A0. The port already keyed it, since 08dda06b. The gate
  `status-window.mjs` composed its expected window unkeyed, so it had
  failed 12 pixels per case since then. It now blits the buttons keyed.
- **Encyclopedia (11552).** A0 shows its corners keyed too; the port drew
  them blue. `bmp_cache` now keys 11552. Its pressed art 11553, blue
  inside, has no capture and stays opaque.

## Browser (Arch Chromium 155, through a Linux browser manifest)

`status-window.mjs` passed all ten cases (two sides by five families) at
the run whose result is `status-window-result.json`. The interface WASM
is `09ec2cbb…`.

## Open

- The rows, title and name: A0 measures smaller fonts than the port's
  (see the next stage).

## Tests

- `cargo test -p rebellion-render`: all pass.
- `message_index_rail_buttons_key_out_their_blue` now also covers 11552
  (keyed) and 11553 (opaque).
