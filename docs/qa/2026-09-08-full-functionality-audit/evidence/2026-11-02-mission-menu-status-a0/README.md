# A Missions window's mission menu and Mission Status (2026-11-02)

Surface ledger OBJ-12 C002, "underway" (Mission Status).

## A0

The A0 run used the Windows 10 VM and the Alliance's DS_flip_allian save, day 179. The captures are in `.artifacts/a0-vm/missionstatus/`, with hashes in `a0-sha256.txt`.

1. **`01-restored`.** The "Mission at Selonia" window is restored from the rail. Its mission is the Bothan Spies' Espionage.
2. **`02-mission-menu`.** A right click on the mission's row opens its menu. Encyclopedia and Status are white. Abort is grey, because the mission is still en route.
3. **`03-mission-status`.** Status opens "Mission Status" at (108, 79), with the mission's picture and "Espionage" in the name box. Its rows are:

   | Label | Value |
   |---|---|
   | Target | Selonia |
   | ETA Destination | Day 185 |
   | Team Size | 1 |
   | Decoys | 0 |

## The original

The Missions window (vtable `0x0065bd10`) keeps the base dialog's menu slots 7 and 8 (`FUN_004ac5c0`, `FUN_004ac730`). Its slot 22, `FUN_004a0400`, gives the menu the selected objects:

- of the left list (the missions);
- or of the right list (the members).

## The port

Before this change, the Missions window had no menu on a right click.

- **`MenuObject::Mission(id)`.** Its rows are the mission order Abort (0x250), then Encyclopedia and Status.
  - Status opens `StatusObject::Mission`, the existing Mission Status (`FUN_00445c60` rows).
  - Encyclopedia opens the kind's MISSNSD record.
  - Abort is gated and acts on that mission alone: `abort_missions` takes a `Mission(id)` selection.
- **`missions_window.rs`.** A right release on a mission row selects it and reports `OpenObjectMenu`. `main.rs` opens the menu there.
- **`sector-quadrants.mjs` `missions`.**
  - It right-clicks the first mission. The menu must read Encyclopedia, Status, Abort.
  - It chooses Status. For the Alliance, it compares "Mission Status" and the first label with A0 `03`: left, top and bottom within 2 pixels.
  - The fixture's Diplomacy is not en route, so its rows are Target, Team Size and Decoys, without the ETA. The port adds the ETA row while a member travels (`eta`), as A0 shows.

## Tests

- `a_right_click_on_a_mission_selects_it_and_opens_its_menu` (`missions_window.rs`).
- `a_missions_window_mission_offers_encyclopedia_status_and_abort` (`object_menu.rs`).

Run `sector-quadrants-2026-11-02T18-22-28-155Z-764477` (Arch Chromium): pass. Title (184..272, 100..109) in A0 against (184..273, 100..109). "Target:" at 126 in A0 against 129, the leading T's bearing (accepted text deviation). Batch `b27`: every other gate passed, workspace 3331, fixtures 116.

## Abort while the team travels and once it works (OBJ-12 C016, 2026-11-03)

**A0.**

- **`missionstatus/02-mission-menu`** (day 179). The Bothan Spies' Espionage at Selonia is still en route, so its menu greys Abort.
- **`personnel-states/12-mission-at-duros`, `13-mission-menu-abort-enabled`** (day 183). Leia Organa's Diplomacy at Duros is under way at its target. Its menu has Encyclopedia, Status and Abort, all white.

**The port.** `MissionState::abort_allowed` takes only a running mission: phase 8 (`FUN_00521600`, codes `0x40` and `0x41`). The `abort` case of `personnel-orders.mjs` (Linux, both sides) checks both states:

- an ordered mission's Abort is disabled and drawn in 0x808080;
- after a day runs the mission, its Abort is enabled and the checkmark ends the mission.

Run `personnel-orders-2026-11-02T22-48-26-834Z-1242152` (batch `b31`): pass.

**Open.** Choosing Abort in A0's Missions window showed no visible confirmation, and the mission went on (day 184). A dimmed frame flickered. The port asks with 0x7052. The cause is not traced.
