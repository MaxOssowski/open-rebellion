# A mine en route against A0

Date: 2026-11-09. Covers CMD-09-C003 (mine en route).

## A0

Windows 10 VM, original REBEXE. This is the new Alliance game of
`2026-11-09-build-selection-classes-and-list-a0`.

1. On day 106, Kessel's facilities band was given the Destination Yavin.
2. A Mine was ordered: 20, 0, "80 Days", and a deployment of "96 Days"
   (`24-mine-to-yavin`). The band then read "Mine, Building: 1,
   Destination: Yavin" (`25-kessel-facilities-mine-to-yavin`).
3. The mine was done by day 188 and set out for Yavin.
4. On day 194, Yavin's Manufacturing window (at (314, 86)) was opened on
   its Mines page (`26-yavin-mines-page-mine-en-route-day194`).

The Mines page shows Yavin's two built mines, then the mine en route in
the third cell, drawn with motion lines. The empty deposits follow, three
to a row.

The captures are in `.artifacts/a0-vm/build-selection4/`, with their
SHA-256 in `SHA256SUMS`.

## The port

`FUN_00458fe0` draws a facility en route as its base + 3, STRATEGY 9004
for a mine. The port's `page_cells` lists the built facilities, then the
queued ones, then those en route, then the empty deposits.

The Build Selection fixture sent only an Orbital Shipyard to the primary
system. It now also sends a Mine (`place_deliveries`). The fixture unit
test `the_build_selection_scenario_sends_a_shipyard_and_a_mine_to_the_primary`
checks the Mines page. Its cells start with the built mine, the mine en
route (9004) and an empty deposit (9005). The test fails without the
fixture change, because only one delivery leaves.

`build-selection.mjs` (Linux Chromium) now checks the Mines page cell by
cell:

| Cell | Picture | Port vs resource | Resource vs A0 |
| --- | --- | --- | --- |
| first (9, 79) | 9001 built | 0 | — |
| second (78, 79) | 9002/9003 under construction | 0 | 0 (A0 `17`, third cell) |
| third (147, 79) | 9004 en route | 0 of 2345 | 0 (A0 `26`, third cell) |
| fourth (8, 117), the second row | 9005 deposit | 0 | — |

The deposit check used to look at the third cell. It moved to the fourth
cell, at the start of the second row, because the mine en route now
takes the third. A0 `26` shows the same layout: three cells to a row,
the deposits after the facilities.

Run: `.artifacts/interface-parity/build-selection-2026-11-09T17-39-54-785Z-2771003`.

## Not covered

- A refinery en route (CMD-09-C006).

## The mine's deployment time

The real-data test `kessel_to_yavin_deployment_days_match_a0`
(`build_selection_lists`, `--ignored`, original DAT files) now also
checks the Mine. A facility moves at the GNPRTB default speed
(`FUN_004f63f0`), and Kessel to Yavin takes 96 days, as A0 `24` shows.
The test passes 3 of 3. It was run by hand, since the batch does not
run ignored tests.

## Batch b159

Workspace tests 3496 passed, fixture tests 139; `build-selection` run
`build-selection-2026-11-09T18-12-09-181Z-2812614`. 22 of 23 interface gates passed on the first run.
`fleet-window` timed out loading pages: during that gate the host was
busy with a test build and another program (load average about 17).
Rerun alone, it passes.

Tactical matrix `2026-11-09T18-51-48-003Z-2855984`: 147 of 148. The one
failure was `production-trench-run-success`, which missed its
`201.webm` request. Rerun alone,
`2026-11-09T19-16-14-569Z-2897271`, it passes 4 of 4.
