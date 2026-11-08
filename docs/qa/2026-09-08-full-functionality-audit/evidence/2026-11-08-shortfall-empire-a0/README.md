# The Empire's Maintenance Shortfall against A0

Date: 2026-11-08. Adds the Empire's side to EVT-01-C006, which already
passes for the Alliance.

## A0

`.artifacts/a0-vm/empire-game3/maintenance-shortfall.png` (SHA-256
`a58edce7cf07f38937868dd661a075d4150e98766d1b15fbf9d0611b1ff0fc8a`) comes
from a new Empire game in the Windows 10 VM, day 50. It reads "The
following units have been destroyed at Coruscant due to a maintenance
shortfall: TIE Fighter".

- **Picture:** the Empire's `0x3f6`, STRATEGY 1014.
- **Window:** at (125, 52).
- **Display:** lit.

## Browser

`message-display.mjs` (Linux, Empire) opens Manufacturing Messages (the
window's (225, 64)) and shows the fixture's Maintenance Shortfall. The
picture, title image and rail are each 0 pixels from A0.

The fixture names another unit, a corvette in transit, so the text is not
compared on this side. The Alliance's text is compared word for word.
