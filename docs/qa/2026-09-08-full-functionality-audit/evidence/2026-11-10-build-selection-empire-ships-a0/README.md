# The Empire's ships and fighters in Build Selection against A0

Date: 2026-11-10. Adds the Empire to OBJ-08-C001 (ship) and OBJ-08-C002
(fighter).

## A0

Windows 10 VM, original REBEXE. A new Empire game (Standard). On day 16,
Chandrila's facilities band (one Construction Yard) ordered an Orbital
Shipyard. By day 98 it was done (ships band "1:1"). On day 129, Build on
the ships band showed eight classes in name order. The window stood at
(255, 87), and the side's unallocated maintenance was 126:

| File | Class | Refined | Maintenance | Completion | Deployment |
| --- | --- | --- | --- | --- | --- |
| `15-1-carrack-light-cruiser` | Carrack Light Cruiser | 26 | 24 | 104 Days | 0 Days |
| `15-2-death-star` | Death Star | n/a | n/a | n/a | n/a |
| `15-3-galleon` | Galleon | 10 | 10 | 40 Days | 0 Days |
| `15-4-imperial-dreadnaught` | Imperial Dreadnaught | 44 | 33 | 176 Days | 0 Days |
| `15-5-imperial-escort-carrier` | Imperial Escort Carrier | 34 | 30 | 136 Days | 0 Days |
| `15-6-imperial-star-destroyer` | Imperial Star Destroyer | 112 | 71 | 448 Days | 0 Days |
| `15-7-tie-fighter` | TIE Fighter | 3 | 3 | 12 Days | 0 Days |
| `15-8-victory-destroyer` | Victory Destroyer | 68 | 44 | 272 Days | 0 Days |

The Death Star costs 584 refined material and 600 maintenance (CAPSHPSD
136). 600 exceeds the 126 the side had free. So `FUN_0052e970` refuses
the order, `FUN_00439160` writes "n/a" in all four values, and Confirm
is grey. This is the same rule as OBJ-08-C008's `manufacturing/11`.

`13-chandrila-reopens-on-orbital-shipyard-day16`: the facilities
band's Build opened on Orbital Shipyard, not on the first class. An
Orbital Shipyard had been confirmed in an earlier Empire game, in the
same run of REBEXE. This agrees with `FUN_00437f80`, which reopens on
the class last confirmed by the same kind of manager. The memory is a
global (`DAT_006b289c`) and outlives a restarted game. The port does
not reproduce that across games, and it is not changed here.

The captures are in `.artifacts/a0-vm/build-selection-empire2/`, with
their SHA-256 in `SHA256SUMS`.

## The port

- **The list.** The real-data test
  `a_new_empire_games_shipyard_lists_eight_classes_as_a0`
  (`build_selection_lists`, `--ignored`, original DAT files) expects
  A0's eight names in order. It passed on the first run.
- **The browser.** `build-selection.mjs` (Empire, Linux Chromium) runs
  `classValuesCase` on the ships band. The fixture's system has one
  Orbital Shipyard, as Chandrila. Seven classes show A0's four values
  exactly. All eight pictures (window 6..200 by 22..47) have 0 of 5070
  pixels differing from A0.
- **The Death Star.** The fixture's side has more than 600 maintenance
  free, so its Death Star shows 584, 600, "2336 Days". Only the picture
  is compared. The refusal is the over-maintenance case, OBJ-08-C008.

Run: `.artifacts/interface-parity/build-selection-2026-11-10T02-06-40-900Z-3555182`.

## Batch b165

23 of 23 interface gates on Linux; workspace tests 3497 passed, fixture
tests 139; `build-selection` run `build-selection-2026-11-10T02-26-16-180Z-3599749`. Tactical matrix
`2026-11-10T03-00-47-307Z-3653840`: 148 of 148. The real-data tests
(`build_selection_lists`, `--ignored`) pass 5 of 5.
