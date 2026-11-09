# A refinery en route against A0

Date: 2026-11-09. Covers CMD-09-C006 (refinery en route). This follows
`2026-11-09-mine-en-route-a0`.

## A0

Windows 10 VM, original REBEXE. This is the same new Alliance game:

1. On day 194, Kessel's facilities band was given the Destination Yavin
   again. A Refinery was ordered: 20, 0, "80 Days", with a deployment of
   "96 Days" (`27-refinery-to-yavin`). The band then read "Refinery,
   Building: 1, Destination: Yavin" (`28-kessel-facilities-refinery-to-yavin`).
2. The refinery was done near day 274 and set out for Yavin.
3. On day 287, Yavin's Manufacturing window (at (314, 86)) was opened on
   its Refineries page (`29-yavin-refineries-page-refinery-en-route-day287`).
   Yavin has no built refinery, so the page holds only the refinery en
   route, in the first cell, drawn with motion lines.

The captures are in `.artifacts/a0-vm/build-selection4/`, with their
SHA-256 in `SHA256SUMS`.

## The port

`FUN_00458fe0` draws a facility en route as base + 3, STRATEGY 9033 for a
refinery.

The Build Selection fixture now also sends a Refinery to the primary
(`place_deliveries`). The fixture unit test
`the_build_selection_scenario_sends_a_shipyard_a_mine_and_a_refinery_to_the_primary`
expects the Refineries page to hold `[9030, 9033]`: the built refinery,
then the one en route. It fails without the change, because only two
deliveries leave.

`build-selection.mjs` (Linux Chromium) finds 9033 in the Refineries
page's third cell, after the built refinery and the one under
construction:

- the port's cell against the resource: 0 of 2345 opaque pixels differ;
- the resource against A0 `29`'s first cell (9, 79): 0 differ.

The real-data test `kessel_to_yavin_deployment_days_match_a0`
(`--ignored`, original DAT files) now also checks the Refinery's 96
days. It passes 3 of 3, run by hand.

Run: `.artifacts/interface-parity/build-selection-2026-11-09T19-42-02-673Z-2907128`.

## Batch b160

23 of 23 interface gates on Linux; workspace tests 3496 passed, fixture
tests 139; `build-selection` run `build-selection-2026-11-09T20-01-56-610Z-2941722`. Tactical matrix
`2026-11-09T20-41-12-961Z-2985350`: 148 of 148.
