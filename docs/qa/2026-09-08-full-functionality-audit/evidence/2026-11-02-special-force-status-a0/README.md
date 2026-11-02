# A special force's menu and Status, against A0 (2026-11-02)

Surface ledger OBJ-10 C002, "special-force types".

## A0

The A0 run used the Windows 10 VM and the Alliance's DS_flip_allian save, day 180. The captures are in `.artifacts/a0-vm/sfstatus/`, with hashes in `a0-sha256.txt`.

- **`01-yavin`.** Yavin's Defenses window, restored from the rail. Its personnel page lists Chewbacca, Orrimaarko and the Infiltrators.
- **`02-sf-menu`.** A right click on the Infiltrators opens a 104 by 122 menu: Move, Confirmed Move, Mission, Encyclopedia, Status, Retire. The unit is en route, so its name is red and its orders are grey.
- **`03-sf-status`.** Status opens "Spec Forces Status" at (108, 79). Its rows:

  | Label | Value |
  |---|---|
  | Attached | Yavin |
  | Status | Enroute |
  | ETA Destination | Day 249 |
  | Maintenance Cost | 1 |
  | Diplomacy Rating | 0 |
  | Espionage Rating | 60 |
  | Combat Rating | 55 |
  | Leadership Rating | 0 |

## The port

`status_rows::special_force_rows` (`FUN_00445780`) gives the same labels in the same order, with the ETA row only while the unit travels.

`status-window.mjs` has a new case, `special-force`. It uses the Recon fixture (code 63), whose recon team stands first on the Defenses window's personnel page. On both sides:

- the menu's size and rows equal A0's;
- the window's chrome is compared as in the other cases.

For the Alliance, the title and the first two labels are within 1 pixel of A0's. The fixture's team is not en route, so the later rows sit one row higher than A0's and are not compared.

Run `status-window-2026-11-02T19-48-43-334Z-846008`: pass.
