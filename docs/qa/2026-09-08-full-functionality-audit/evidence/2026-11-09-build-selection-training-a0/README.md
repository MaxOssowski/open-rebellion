# Build Selection's regiments and special forces against A0

Date: 2026-11-09. Covers OBJ-08-C004 (special force) and adds the
Alliance to OBJ-08-C003 (troop). This continues
`2026-11-09-build-selection-classes-and-list-a0`.

## A0

Windows 10 VM, original REBEXE. This is the same new Alliance game as the
previous note. On Kessel, a Training Facility was ordered on day 51
(10, 10, "40 Days"). It finished on day 93. The troops band then read
"1:1".

Build on the troops band listed six classes by name: Alliance Army
Regiment, Alliance Fleet Regiment, Bothan Spies, Guerrillas,
Infiltrators, Longprobe Y-wing Recon Team. Each was chosen in turn. The
window stood at (192, 84) in every capture.

Captures are in `.artifacts/a0-vm/build-selection4/`, with their SHA-256 in
`SHA256SUMS`:

| File | Class | Refined | Maintenance | Completion | Deployment |
| --- | --- | --- | --- | --- | --- |
| `09-kessel-training-facility-day51` | Training Facility | 10 | 10 | 40 Days | 0 Days |
| `11-kessel-alliance-army-regiment-day103` | Alliance Army Regiment | 6 | 3 | 24 Days | 0 Days |
| `12-2-alliance-fleet-regiment` | Alliance Fleet Regiment | 8 | 6 | 32 Days | 0 Days |
| `12-3-bothan-spies` | Bothan Spies | 1 | 1 | 4 Days | 0 Days |
| `12-4-guerrillas` | Guerrillas | 1 | 1 | 4 Days | 0 Days |
| `12-5-infiltrators` | Infiltrators | 2 | 1 | 8 Days | 0 Days |
| `12-6-longprobe-y-wing-recon-team` | Longprobe Y-wing Recon Team | 2 | 1 | 8 Days | 0 Days |

`10-kessel-training-facility-building-day53` shows the Manufacturing
window after the order. The facilities band reads "Training Facility,
Building: 1", and the troops band "0:1". It is not compared yet.

## The browser comparison

`build-selection.mjs` (Alliance, Linux Chromium) now runs
`classValuesCase` on the troops band as well. The fixture's system has
one Training Facility. Each of the six classes shows A0's four values
exactly. Each class picture in the shown item (window 6..200 by 22..47)
has 0 of 5070 pixels differing from A0. The facilities band also checks
Training Facility against `09`: exact values and 0 differing pixels.

Run: `.artifacts/interface-parity/build-selection-2026-11-09T14-11-44-633Z-2394204`.

## Not covered

- The Empire's special forces. The Empire's Imperial Army Regiment was
  already compared with A0 `build-selection2/01`.
- The names' colours in the open list (see the previous note).

## Batch b156

23 of 23 interface gates on Linux; workspace tests 3496 passed, fixture
tests 139. In the batch, the `build-selection` run `build-selection-2026-11-09T14-30-00-602Z-2436859` found the same
values and pictures. Tactical matrix
`2026-11-09T15-03-54-866Z-2484773`: 147 of 148. The one failure was
`production-trench-run-failure` (Alliance, 1280x800), which missed its
`202.webm` request, the known flaky movie fetch. Rerun alone,
`2026-11-09T15-17-41-485Z-2517006`, it passes 4 of 4.
