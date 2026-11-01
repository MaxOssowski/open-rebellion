# A squadron's Confirmed Move (2026-11-01)

## A0

`20-fighter-menu.png` (an Empire planet squadron at Coruscant) shows Move,
Confirmed Move, Encyclopedia, Status and Scrap, all enabled.
`22-battery-menu.png` shows a battery's Encyclopedia, Status and Scrap,
the rows the port lists. The captures stay in `.artifacts/a0-vm/status/`;
their hashes are in `a0-sha256.txt`.

## Change

- `object_menu`: a squadron's Confirmed Move follows its Move gate.
- `TargetOrder::SquadronMove` carries `confirmed`. On release a Confirmed
  Move opens the move confirmation with the class name and days
  (`FUN_0053c2e0`), and the evacuation warning from a blockaded planet
  (`FUN_0049a350`).
- `SquadronTransits::move_days` gives the days at the class's speed
  (`FUN_00502f80`) without moving. port: none within one system.
- The checkmark resubmits the squadron's Move (`FUN_0041ce20(order, 1)`).

## Result (Linux Chromium)

`fleet-window.mjs` case `squadron-confirmed-load` passes for both sides
(`gate-fleet-window-squadron-confirmed-load.txt`). The menu shows
Confirmed Move at row 1, enabled. The confirmation reads "Transit time in
days / TIE Fighter: 0" for the Empire. The X keeps both squadrons on the
planet; the checkmark boards one. Screenshot hashes are in
`port-screenshots-sha256.txt`.

`a_squadron_lists_move_confirmed_move_and_scrap` fails without the rule
(Confirmed Move missing from the enabled rows).

## Mutants

- `squadrons.rs --re 'move_days|squadron_transit_ticks'`: 5 tested, 5 caught.
- `move_confirmation.rs --re answer`: 8 tested, 7 caught, 1 unviable.
- `object_menu.rs --re object_menu_rows`: 44 tested, 43 caught, 1 unviable.

## Open

No pixel comparison of the menus against A0. A capital ship's Confirmed
Move stays disabled; no A0 capture of a ship's menu exists.
