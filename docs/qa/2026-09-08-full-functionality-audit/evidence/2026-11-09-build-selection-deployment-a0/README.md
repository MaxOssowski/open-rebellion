# Build Selection's deployment time to another system against A0

Date: 2026-11-09. Covers OBJ-08-C010 (destination). Also adds Mine,
Orbital Shipyard and Refinery to OBJ-08-C005.

## A0

Windows 10 VM, original REBEXE. This is the new Alliance game of
`2026-11-09-build-selection-classes-and-list-a0`, on day 105.

Kessel's ships band was given the Destination Yavin. The band menu's
Destination was chosen, then Yavin's planet was clicked in the Sumitra
sector window. Afterwards the band read "Destination: Yavin". Build on
that band then showed these values:

| File | Class | Completion | Deployment |
| --- | --- | --- | --- |
| `20-dreadnaught-to-yavin` | Alliance Dreadnaught | 176 Days | 76 Days |
| `21-x-wing-to-yavin` | X-wing | 20 Days | 57 Days |
| `22-escort-carrier-to-yavin` | Alliance Escort Carrier | 128 Days | 76 Days |
| `23-corellian-corvette-to-yavin` | Corellian Corvette | 56 Days | 76 Days |

`19-kessel-ships-destination-yavin` shows the band with its new
destination. The captures are in `.artifacts/a0-vm/build-selection4/`,
with their SHA-256 in `SHA256SUMS`.

## The port

The deployment value is `delivery::transit_days`. Its source chain is
`FUN_00555f90`, then `FUN_00555b30`, the product's transit from its yard
to the destination. Capital ships and fighters move at their classes'
speeds. A new real-data test checks it on the original DAT files:
`kessel_to_yavin_deployment_days_match_a0` (rebellion-app,
`build_selection_lists`, `--ignored`).

The test finds Kessel and Yavin by name and lists the shipyard's classes
for a new Alliance game. It expects 76 days for Dreadnaught, Escort
Carrier and Corvette, and 57 for X-wing. It passed on the first run. The
port already computed A0's times, and no code changed.

The browser fixture's systems are not Kessel and Yavin, so the gate does
not compare these times. Its existing `destination` cases check that the
band takes the destination (A0 `build-selection2/21`, OBJ-09-C010).

## Mine, Orbital Shipyard and Refinery

`build-selection.mjs` now also checks three classes against
`build-selection4/13`..`15`, each with exact values and 0 differing
picture pixels:

- Mine: 20, 0, "80 Days", "0 Days";
- Orbital Shipyard: 20, 13, "80 Days", "0 Days";
- Refinery: 20, 0, "80 Days", "0 Days".

Run: `.artifacts/interface-parity/build-selection-2026-11-09T16-29-18-046Z-2641922`.

## Batch b158

23 of 23 interface gates on Linux; workspace tests 3496 passed, fixture
tests 139; `build-selection` run `build-selection-2026-11-09T16-45-09-476Z-2680422`. Tactical matrix
`2026-11-09T17-18-43-644Z-2733764`: 148 of 148. The real-data tests
(`build_selection_lists`, `--ignored`) pass 3 of 3.
