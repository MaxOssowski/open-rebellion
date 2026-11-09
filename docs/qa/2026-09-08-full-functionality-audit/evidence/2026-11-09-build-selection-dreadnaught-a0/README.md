# Build Selection's ship values against A0

Date: 2026-11-09. Covers OBJ-08-C001 (ship).

## A0

`.artifacts/a0-vm/build-selection3/ships0.png` (SHA-256 in `SHA256SUMS`):
a new Expert Alliance game, day 5, Build Ships on Sluis Van, from the
2026-11-05 session (`2026-11-05-build-selection-order`). The window
stands at (192, 84). It shows Alliance Dreadnaught:

| Value | A0 |
| --- | --- |
| Refined material | 44 |
| Maintenance | 33 |
| Best Time To Completion | 176 Days |
| Best Time To Deployment | 0 Days |

`FUN_00538220` fills the four values: the class's refined cost and
maintenance times the number, the yards' best completion, and the
transit to the destination. 176 is 44 times one Orbital Shipyard's
processing period of 4.

## The browser comparison

`build-selection.mjs` (Alliance, Linux Chromium) now opens Build on the
fixture system's ships band, which has one Orbital Shipyard. It steps
the class list to Alliance Dreadnaught and checks:

- the values: `["44", "33", "176 Days", "0 Days"]`, as A0;
- the class picture in the shown item (window x 6..200, y 22..47, above
  the name): 0 of 5070 pixels differ from A0. The port's window also
  stands at (192, 84).
- Cancel closes the window and queues nothing.

A side-by-side look at the two windows shows the same layout. Only the
text glyphs differ, the font deviation the owner accepted on
2026-10-30. The window's chrome was already compared with the composed
original resources ("opened", 0 of 28 496 pixels).

Run: `.artifacts/interface-parity/build-selection-2026-11-09T10-58-57-053Z-2079337`
(`alliance/ships-dreadnaught.png`).

## Not covered

- Fighters (OBJ-08-C002): A0 lists X-wing and Y-wing among the ships,
  but no capture shows a fighter's values.
- The Empire's ships: no A0 capture.
- This is no new code. The check is a browser test of behaviour that was
  already ported, so there is no failing run without a change.

## Batch b154

23 of 23 interface gates on Linux; workspace tests 3496 passed, fixture
tests 139. `build-selection` run
`build-selection-2026-11-09T11-25-24-539Z-2117362` found the same values
and 0 differing picture pixels. Tactical matrix
`2026-11-09T12-10-22-754Z-2163018`: 148 of 148. The A0 VM was paused
for the batch.
