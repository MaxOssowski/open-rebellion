# A facility en route: its menu and Status against A0

Date: 2026-11-09. Covers OBJ-09-C003 (en route).

## A0

Windows 10 VM, original REBEXE. This is the new Alliance game of
`2026-11-09-refinery-en-route-a0`, on day 288. The Refinery from Kessel
travels to Yavin. On Yavin's Refineries page:

- a right click on it lists Encyclopedia and Status, with Scrap grey
  (`30-yavin-refinery-en-route-menu-day288`);
- Status opens the Manufacturing Status window at (108, 79)
  (`31-yavin-refinery-en-route-status-day288`).

The window's rows:

| Label | Value |
| --- | --- |
| Location: | Yavin |
| ETA Destination: | Day 370 |
| Status: | Enroute |
| Maintenance Cost: | 0 |
| Standard Processing Rate: | 5 |
| Bombardment Value: | 2 |

Its picture is the en-route art 9033 over the grid mark, and its name
reads "Refinery".

## The port

`status_rows::manufacturing_rows` (`FUN_00446c50`) already wrote this
order. The ETA row (`FUN_004fd2b0`) comes after Location, and the
Status word of an unfinished object travelling is 34628 "Enroute"
(`FUN_0044a470`).

`build-selection.mjs` (Linux Chromium, both sides) now right-clicks the
fixture's refinery en route, in the Refineries page's third cell. It
checks:

- the menu: Encyclopedia, Status, and Scrap disabled, as A0 `30`;
- the window: "Manufacturing Status" with A0's six labels in A0's order,
  "ETA Destination:" a "Day N", and the values Enroute, 0, 5, 2;
- for the Alliance, the picture (window 242..371 by 15..112) and the
  buttons (242..371 by 200..259) against A0 `31`: 0 pixels differ.

The fixture's day differs from A0's, so the ETA's day is not compared.
The texts' glyphs are the accepted font deviation. The label "Standard
Processing Rate:" wraps onto two lines in both.

Run: `.artifacts/interface-parity/build-selection-2026-11-09T22-18-26-500Z-3129483`.

## Batch b162

23 of 23 interface gates on Linux; workspace tests 3496 passed, fixture
tests 139; `build-selection` run `build-selection-2026-11-09T22-37-12-447Z-3170705`. Tactical matrix
`2026-11-09T23-11-12-317Z-3217722`: 148 of 148.
