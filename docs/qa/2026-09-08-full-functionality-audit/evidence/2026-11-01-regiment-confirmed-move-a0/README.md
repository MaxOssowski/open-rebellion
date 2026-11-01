# A regiment's Confirmed Move (2026-11-01)

## A0

`11-troop-menu.png` (hash in `a0-sha256.txt`; the capture stays in
`.artifacts/a0-vm/status/`) shows the player's regiment with Move and
Confirmed Move both enabled. The port drew Confirmed Move disabled.

## Change

- `object_menu`: a regiment's Confirmed Move follows the same gate as its
  Move.
- `targeting::TargetOrder::TroopMove` carries `confirmed`. On release a
  Confirmed Move opens the move confirmation with the regiment's line
  (`FUN_0053c2e0`: class name and days) and the evacuation warning when the
  regiment stands on a blockaded planet (`FUN_0049a350`).
- `TroopTransportState::move_days` gives the days the move would take
  without carrying it out. port: none within the regiment's own system.
- The checkmark resubmits the regiment's move to the fleet or system
  (`FUN_0041ce20(order, 1)`); the X destroys it.

## Result (Linux Chromium)

`fleet-window.mjs` case `confirmed-load` passes for both sides
(`gate-fleet-window-confirmed-load.txt`). The regiment's menu shows
Confirmed Move at row 1, enabled. The confirmation reads "Transit time in
days / Alliance Army Regiment: 0". The X keeps the regiment on its planet;
the checkmark boards it. Screenshot hashes are in
`port-screenshots-sha256.txt`.

The menu test `a_regiments_move_and_confirmed_move_follow_its_gate` fails
without the rule (Confirmed Move missing from the enabled rows).

## Mutants

- `troop_transport.rs --re 'move_days|leg_system|regiment_transit_ticks'`:
  6 tested, 6 caught.
- `move_confirmation.rs --re answer`: 8 tested, 7 caught, 1 unviable.
- `object_menu.rs --re object_menu_rows`: 42 tested, 41 caught, 1 unviable.

## Open

No pixel comparison of the confirmation against A0. A squadron's and a
capital ship's Confirmed Move stay disabled (no A0 capture of them enabled).
