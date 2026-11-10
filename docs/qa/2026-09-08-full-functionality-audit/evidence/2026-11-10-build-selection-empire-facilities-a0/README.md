# The Empire's facility classes in Build Selection against A0

Date: 2026-11-10. Adds the Empire to OBJ-08-C005 (facility) and
OBJ-08-C006 (defense). Also records what A0 showed about maintenance at
the order.

## A0

Windows 10 VM, original REBEXE. A new Empire game (Standard), day 6.
Coruscant held two Construction Yards and no shipyard or training
facility (its facilities band "2:2"). Build on that band listed eight
classes by name. The Advanced Training Facility was not yet researched.
The window stood at (255, 87). Captures are in
`.artifacts/a0-vm/build-selection-empire2/`, with their SHA-256 in
`SHA256SUMS`:

| File | Class | Refined | Maintenance | Completion | Deployment |
| --- | --- | --- | --- | --- | --- |
| `01-construction-yard` | Construction Yard | 10 | 10 | 20 Days | 0 Days |
| `02-gencore` | GenCore Level I | 12 | 12 | 24 Days | 0 Days |
| `03-kdy150` | KDY-150 | 4 | 4 | 8 Days | 0 Days |
| `04-lnr` | LNR Series I | 7 | 7 | 16 Days | 0 Days |
| `05-mine` | Mine | 20 | 0 | 40 Days | 0 Days |
| `06-orbital-shipyard` | Orbital Shipyard | 20 | 13 | 40 Days | 0 Days |
| `07-refinery` | Refinery | 20 | 0 | 40 Days | 0 Days |
| `08-training-facility` | Training Facility | 10 | 10 | 20 Days | 0 Days |

The refined material and maintenance are the Alliance's values from
`build-selection4`. With two yards the completion is half a single
yard's, except LNR Series I, which takes 16 days, not 14.

## The browser comparison

`build-selection.mjs` (Empire, Linux Chromium) now runs
`classValuesCase` on the facilities band with these eight classes. For
each class:

- refined material, maintenance and deployment match exactly;
- the class picture in the shown item (window 6..200 by 22..47) has 0 of
  5070 pixels differing from A0 at (255, 87).

The fixture's Empire system has one Construction Yard. Its completion
times are a single yard's (40, 48, 16, 28, 80, 80, 80, 40) and are not
compared with A0's two-yard times.

Run: `.artifacts/interface-parity/build-selection-2026-11-09T23-35-51-621Z-3248972`.

## The two-yard completion

The new core test `two_construction_yards_finish_as_a0_shows`
(`manufacturing.rs`) feeds `completion_day` two yards with period 4,
the Construction Yard's from `build-selection4`. It expects A0's times:

- 10 gives 20 days, 12 gives 24, 4 gives 8 and 20 gives 40;
- 7 gives 16: two yards finish 2 points every 4 days, so the 7th point
  falls on day 16;
- one yard with 7 gives 28.

It passed on the first run. The port already computed these, and no
code changed.

## Maintenance at the order

`09a-before-confirm-maintenance-103` shows the maintenance counter at
103 before Confirm on an Orbital Shipyard (maintenance 13).
`09-coruscant-shipyard-ordered-maintenance-90` shows 90 after it. The
original takes the maintenance when the order is placed, as manual
p. 84 says.

The port does the same. `side_resources::maintenance_totals` adds every
queued item's upkeep (its loop over `manufacturing.queues()`), so the
unallocated maintenance drops at the order. The line in
`ghidra/notes/manufacturing-build-selection.md`, "port: maintenance is
shown, not charged", was stale. It now says what A0 and the port do.

## Not covered

- The Empire's ships, fighters, regiments and special forces. Three new
  Empire games had no shipyard or training facility. In one of them, a
  lost battle at Kothlis drove the maintenance to −137
  (`10-after-kothlis-blockade-maintenance-minus137`), and Build Selection
  would then show "n/a".

## Batch b163

23 of 23 interface gates on Linux; workspace tests 3497 passed (the new
core test among them), fixture tests 139; `build-selection` run
`build-selection-2026-11-09T23-55-21-162Z-3290145`. Tactical matrix `2026-11-10T00-29-35-206Z-3343956`: 148 of 148.
