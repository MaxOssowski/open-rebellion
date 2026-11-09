# A mine and a shipyard under construction against A0

Date: 2026-11-09. Covers CMD-09-C002 (mine building) and CMD-09-C008
(shipyard building).

## A0

Windows 10 VM, original REBEXE. This is the new Alliance game of
`2026-11-09-build-selection-classes-and-list-a0`, on day 104. Kessel's
Manufacturing window stood at (73, 86). Its facilities band ordered, one
after the other:

1. a Mine (20, 0, "80 Days");
2. an Orbital Shipyard (20, 13, "80 Days");
3. a Refinery (20, 0, "80 Days").

Each Confirm replaced the band's order. Afterwards the band read
"Refinery, Building: 1", and only the Refineries page showed a facility
under construction. This agrees with the port's rule that Confirm
replaces the band's units (`ghidra/notes/manufacturing-build-selection.md`).

A Mine was then ordered again, and after it an Orbital Shipyard. The
captures are in `.artifacts/a0-vm/build-selection4/`, with their SHA-256
in `SHA256SUMS`:

| File | Shows |
| --- | --- |
| `13-kessel-mine` | Build Selection on Mine: 20, 0, "80 Days", "0 Days" |
| `14-kessel-orbital-shipyard` | Orbital Shipyard: 20, 13, "80 Days", "0 Days" |
| `15-kessel-refinery` | Refinery: 20, 0, "80 Days", "0 Days" |
| `16-kessel-band-mine-building` | the band: "Mine, Building: 1" |
| `17-kessel-mines-page-mine-building` | Mines page: two built mines, then one under construction in the third cell |
| `18-kessel-shipyards-page-shipyard-building` | Shipyards page: the built shipyard, then one under construction in the second cell |

## The browser comparison

`build-selection.mjs` already compared the port's facility pages with
the original resources. `FUN_00458fe0` gives the pictures: base + 2 for
side 1, under construction. The gate now also compares those resources
with A0, at A0's cells:

| Label | Picture | A0 cell (window) | Opaque pixels | Differing from A0 |
| --- | --- | --- | --- | --- |
| `shipyard-building` | STRATEGY 9008 | (78, 79), `18` | 2147 | 0 |
| `mine-building` | STRATEGY 9003 | (147, 79), `17` | 2145 | 0 |

The port's pictures differ from those resources in 0 pixels, so the
port matches A0 here. A0 places a facility under construction after the
built ones, as the port does. The fixture has one built mine, so its
mine sits in the second cell, and A0's two built mines put it in the
third.

Run: `.artifacts/interface-parity/build-selection-2026-11-09T15-23-26-948Z-2524905`.

## Not covered

- The values of Mine, Orbital Shipyard and Refinery (`13`..`15`) are
  captured but not yet compared in the gate.
- Facilities en route (CMD-09-C003, C006).

## Batch b157

23 of 23 interface gates on Linux; workspace tests 3496 passed, fixture
tests 139; `build-selection` run `build-selection-2026-11-09T15-39-20-333Z-2558092`. Tactical matrix
`2026-11-09T16-13-03-147Z-2607937`: 148 of 148.
