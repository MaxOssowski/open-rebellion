# Smuggling Benefits against A0: the picture was missing

Date: 2026-11-08. Closes EVT-01-C003.

## A0

`.artifacts/a0-vm/alliance-game2/smuggling-benefits.png` (SHA-256
`9bdebe1df2db5ff2d741bb6c2586a9d3f3271b3e3cd6a62bc6a2f93d7cf279c2`) comes
from a new Alliance game in the Windows 10 VM, day 642. The Empire held
Yavin by then.

The message reads "Smuggling Benefits" / "Smugglers from Yavin are
providing us with additional resources." This is `FUN_00499130`, the other
side's system with smuggling beginning (`0x7062` / `0x7066`). The picture
is the Millennium Falcon over a night base, and Display is lit (the class
`0x0065ba68` is kind 3).

## The defect

The port's `smuggling_message` drew no picture. It carried the note
"port: the pictures are not drawn", and the text named `0x3ec` and `0x44f`.

The picture in A0 matches STRATEGY 1004 (`0x3ec`) in every sampled pixel.
STRATEGY has no `0x44f` bitmap.

The message now carries the background `0x3ec`. The test
`smuggling_speaks_when_it_begins_or_ends_by_whose_system_it_is` checks it
and fails without the change.

## Browser

The event-message fixture (Alliance) begins smuggling on an Empire-held
Yavin through `story_messages::smuggling_message`
(`interface_test_fixture::smuggled_system`). It lists last on Popular
Support.

`message-display.mjs` (Linux, Alliance) checks the exact text, and the
picture, title image and rail are each 0 pixels from A0.

These variants have no A0 capture: own-system losses (`0x7060` / `0x7064`),
and the endings (`0x7061`, `0x7063`).
