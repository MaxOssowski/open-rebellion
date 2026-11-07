# The Move confirmation against A0 (2026-11-07)

This covers CMD-11-C024 (Move-order confirmation) and the confirmation window of the fleet-move cases.

## A0

Until now, QMP clicks had never registered for a Move target. A slower press (pointer still, then the button held for 0.4 s) worked here.

A new Easy Empire game, day 9: Fleet 1 at Coruscant, its menu, Confirmed Move, then Corsin. The captures are in `.artifacts/a0-vm/move-confirm/` (hashes in `a0-sha256.txt`):

| Capture | What it shows |
|---|---|
| `00-fleet-menu` | Move, Confirmed Move, Planetary Bombardment, Planetary Assault (grey), Encyclopedia, Status (grey), Scrap |
| `01-confirm-corsin` | The confirmation over the galaxy: "Transit time in days" / "Fleet 1: 4", the checkmark and the X |
| `02-tip-yes` | Hovering the checkmark shows "Yes" (REBDLOG `0x1300`) |
| `03-tip-no` | Hovering the X shows "No" (`0x1301`) |

The checkmark (STRATEGY 10926) and the X (10929) match A0 with their blue keyed: 0 differing pixels at the window's (355, 244) and (355, 281). That puts the window at (148, 52): `galaxy_centered_rect`, the spare width halved and truncated.

## Browser (`fleet-move.mjs`, Linux)

- **`confirmationOrigin`.** It rounded the half width up, giving the Alliance (86, 50). The port draws its window at (85, 49), with the spare halved and truncated, as A0 places the Empire's. Its clicks had hit anyway, inside the buttons. It now truncates.
- **`confirmed-move`.** It now compares both buttons, keyed, with the port's screen (both sides: 0 and 0), and on the Empire's side with A0 `01-confirm-corsin` (0 and 0).

The whole gate passes: `.artifacts/interface-parity/fleet-move-2026-11-07T10-09-22-614Z-3426439`.

The tips' texts "Yes" and "No" are A0's. Their boxes are not measured.

## The X, and a plain Move

**The X** (`04-x-cancelled`). It closed the window, and the fleet stayed at Coruscant. That is `FUN_0044f640`, which the `confirmed-move-cancelled` case checks.

**A plain Move** (`05-plain-move-no-question`). The fleet menu's Move released on Corsin asked nothing: Fleet 1 left at once. Its icon now stands at Corsin with the dotted, en route fleet art. The port's `move` case does the same: no confirmation, and the fleet in transit to the target.
