# Moving the Alliance headquarters (2026-11-04)

This covers surface ledger cells OBJ-13-C005 ("move HQ") and OBJ-13-C006 ("HQ in transit"). It also closes the open state of OBJ-13-C004 ("HQ item menu"). The reading is in `ghidra/notes/headquarters-move.md`.

## The original

- **Enabled rows.** The HQ's Move and Confirmed Move are enabled when three things hold (`FUN_00578c00`):
  - the HQ is the ordering side's (`FUN_005269b0` → `FUN_004f9860`, status 1);
  - it is not en route (status 4);
  - it is not held by a blockade. `FUN_005555e0` refuses `0x90`/4 when the system is blockaded and `FUN_0055a020` over its fleets is below 31.
- **The destination.** It must be the side's own system (`FUN_0053d430`, `1`/`0x28`). The HQ is no fleet, ship or regiment.
- **The move.** The command moves the HQ into its destination at once and marks it en route (`FUN_00556390`, `FUN_004f7640`). The trip runs at the GNPRTB 1 default speed (`FUN_004f63f0`).
- **Support.** The move shifts the Alliance's support by GNPRTB 7706 (5). It falls by 5 at a populated origin and rises by 5 at a populated destination (`System_vtc8` `FUN_00508660`, `FUN_0055a0f0`, `FUN_0050c9f0`). The manual (p. 134) names the drop at the origin.
- **What shows during the trip:**
  - The mark 904 shows over the destination, since `FUN_0045bbb0` reads mode 3, "existing".
  - The HQ's Status leads with the ETA (`FUN_004fd2b0`) and reads "Enroute" (`FUN_0044a470`).
  - A new Move is refused (status 4).
- **Confirmation.** Confirmed Move always asks (`FUN_00487cc0`). A Move asks only out of a blockaded system, and then with the evacuation text 0x7056 (`FUN_0049a350`).
- **Manual.** p. 134 says to drag the HQ icon onto a new system, or right-click and select Move.

## Before

- The HQ's Move and Confirmed Move were grey. A0 shows them white.
- A drag of 904 dropped nothing.

## Now

- **The core.** `rebellion_core::headquarters_move` (`move_enabled`, `plan`, `move_headquarters`, `advance`) moves the facility and the `is_headquarters` flag at once.
- **Victory.** The victory state's headquarters follows the move, since `FUN_00506110` finds the HQ by its object.
- **Support.** It shifts by ∓5 at the two systems, each only if populated.
- **Save.** The trip is saved in `TroopTransportState::headquarters`. The save format goes from v111 to v112.
- **The orders.** The menu rows, targeting, the sector-window drag (Ctrl for Confirmed Move) and the confirmation window (`ConfirmedOrder::MoveHeadquarters`) all issue the order.
- **Status.** It shows the ETA and "Enroute" until arrival.
- **port notes:**
  - A Fleet window's fleet as the destination is refused.
  - A destroyed system counts as another side's.
  - No arrival message is traced.

## Tests

Unit tests:
- In `headquarters_move.rs`:
  - `the_headquarters_moves_at_once_and_is_en_route_until_its_arrival_day`;
  - `the_headquarters_moves_only_to_its_sides_systems_and_only_for_its_side`;
  - `a_move_to_the_headquarters_own_system_changes_nothing`;
  - `a_blockade_holds_the_headquarters_while_the_blockaders_leave_it_under_31_percent`;
  - `the_move_costs_the_origin_its_support_and_gives_it_to_a_populated_destination`. Without the shift, the support stays 50/50;
  - `each_refusal_reads_as_its_own_reason`.
- In `blockade.rs`: `escape_percent` is split out of `withdraw_percent`, and the blockade tests keep passing.
- `status_rows.rs`: `the_headquarters_en_route_leads_with_its_eta_and_reads_enroute`.
- `object_menu.rs`: `the_headquarters_lists_move_confirmed_move_encyclopedia_and_status_without_scrap` now also checks that the gate enables both moves.
- `move_confirmation.rs`: `the_headquarters_move_reads_as_a_move_and_its_checkmark_sends_it`.
- `sector_window.rs`: `a_drag_from_the_headquarters_mark_drops_the_headquarters`.

`sector-quadrants.mjs` (Linux Chromium):
- **`headquarters-menu`.** The Alliance's rows are now all white, as A0 headquarters/01 shows.
- **`headquarters-move` (new).** The steps are:
  1. Move on the HQ, then a click on the Alliance's other system (Ithor).
  2. 904 leaves its origin: 376 of 376 pixels differ.
  3. 904 stands over Ithor: 0 of 376 differ.
  4. The HQ's menu there greys both moves.
  5. Its Status reads "ETA Destination: Day 6", "Location: Ithor" and "Status: Enroute".

  The Empire does not see this HQ, so its case only checks that.

Run `sector-quadrants-2026-11-04T13-50-06-245Z-1504521` (`alliance/headquarters-menu`, `alliance/headquarters-move`, `empire/headquarters-move`): pass.

Full batch b55:
- the workspace tests (3364) and fixture tests (116) pass, and provenance-scan is clean;
- all 21 Linux gates pass, `sector-quadrants` in full among them.

Mutants:
- `rebellion-render` in-diff: 20 tested, 15 caught, 5 unviable.
- `rebellion-core` in-diff: 2 survived, the Display text and the `< 31` bound. `each_refusal_reads_as_its_own_reason` and the exact 31% case of the blockade test close them.
- On the rerun of `headquarters_move.rs`, 27 mutants were caught and 2 were unviable.
- `move_headquarters` after the support shift: 5 caught, 1 unviable.

The support shift was added after the batch. On a rebuilt WASM, the workspace tests (3366) and fixture tests (116) pass, provenance-scan is clean, and `sector-quadrants` passes in full (`sector-quadrants-2026-11-04T14-52-16-591Z-1820899`).

## A0

Captured in the Windows 10 VM (the Alliance save, days 555 and 556, the Moddell sector window). The files are in `.artifacts/a0-vm/headquarters-move/`.

- **The HQ did move.** On day 488 (`../2026-11-04-headquarters-overlay/`, `02-hq-status`) its Status read "Location: Pzob". On day 556 (`06-status`) it read "Location: Gandolo IV", "Status: Active", with no ETA row. It reached Gandolo IV during this session's attempts or earlier; which order sent it is not known.
- **The open window kept its mark.** The open sector window still drew 904 over Pzob (`07-closed-status`). Only after the window was closed and opened again did 904 stand over Gandolo IV (`09-reopened`).
  - In the original, the overlay (one id `0x10000` per window) is rebuilt by a system's refresh (`FUN_0045b770` → `FUN_0045bbb0`). The refresh reads the side's record of the system (`FUN_004f3220`), and this move sent no refresh.
  - The port redraws 904 at once. This is an open difference.
- **The menu after arrival.** The HQ's menu at Gandolo IV again shows all four rows white (`11-menu`).
- **Input that did not register.** None of these gave a confirmation window, and no later move happened:
  - a drag of 904 (`01-drag`, `02-drag`);
  - a Ctrl+drag (`10-ctrl-drag`);
  - a targeting release on Hozrel X (`04-targeting`, `05-release-*`);
  - a Move from the menu released on Pzob (`13-menu` to `17-status-after`, day 561, still "Location: Gandolo IV").

  Input over QMP is unreliable here, as for the other orders' targeting.
- **Hashes.** `a0-sha256.txt` lists the captures.

## Open

- **The stale mark.** Which notice refreshes the HQ overlay after a move. Until that is known, the port's immediate redraw stays a difference from A0.
- **No A0 en route.** There is no A0 capture of the HQ while it is en route: no "Enroute" Status and no confirmation window.
- **Untraced.** The fleet destination's leg, and the arrival notice.
