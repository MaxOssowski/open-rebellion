# The Empire's training list has no Bounty Hunters

Date: 2026-11-10. Covers OBJ-08-C004 (special force) and OBJ-08-C003
(troop) for the Empire, and fixes the list's contents.

## A0

Windows 10 VM, original REBEXE. A new Empire game (Standard), day 7.
Build was chosen on Sluis Van's troops band; Sluis Van has one Training
Facility ("1:1"). The window stood at (255, 87). The list held seven
classes, in name order:

| File | Class | Refined | Maintenance | Completion | Deployment |
| --- | --- | --- | --- | --- | --- |
| `12-1-imperial-army-regiment` | Imperial Army Regiment | 6 | 3 | 24 Days | 0 Days |
| `12-2-imperial-commandos` | Imperial Commandos | 2 | 1 | 8 Days | 0 Days |
| `12-3-imperial-espionage-droid` | Imperial Espionage Droid | 1 | 1 | 4 Days | 0 Days |
| `12-4-imperial-fleet-regiment` | Imperial Fleet Regiment | 7 | 5 | 28 Days | 0 Days |
| `12-5-imperial-probe-droid` | Imperial Probe Droid | 1 | 1 | 4 Days | 0 Days |
| `12-6-noghri-death-commandos` | Noghri Death Commandos | 2 | 1 | 8 Days | 0 Days |
| `12-7-stormtrooper-regiment` | Stormtrooper Regiment | 9 | 6 | 36 Days | 0 Days |

The window opened on the first class, Imperial Army Regiment
(`11-sluis-van-imperial-army-regiment`). The scroll bar's thumb, 35 of
122 pixels for two visible items, fits a list of seven.

The captures are in `.artifacts/a0-vm/build-selection-empire2/`, with
their SHA-256 in `SHA256SUMS`.

## The defect

The port also listed **Bounty Hunters**, first by name. SPECFCSD record
9 (TEXTSTRA 9604) serves the Empire but costs 0 refined material and 0
maintenance. Bounty Hunters are the story's hunters, not a class to
train.

`FUN_0052e580` lists through `FUN_0051cb20`, then `FUN_00567880`
(`ghidra/notes/manufacturing-build-selection.md`,
`mission-lifecycle.md`). `FUN_00567880` walks the side's classes of
the family that have a refined cost: `FUN_0053b860`, class `+0x48`,
nonzero. The port's `research_classes` already applied that filter, and
`listed_classes` did not.

## The change

`build_selection::listed_classes` (rebellion-core) now keeps only
classes with a refined cost, for ships, fighters and the buildable
classes alike.

| Test | Result |
| --- | --- |
| `a_new_empire_games_training_facility_lists_seven_classes_as_a0` (rebellion-app, `build_selection_lists`, `--ignored`, original DAT files) | expects A0's seven names in order; failed before the change, with "Bounty Hunters" first |
| the other real-data list tests (Alliance shipyard and construction yard, deployment days) | still pass, 4 of 4 |
| two unit tests in `ai.rs` and one in `main.rs` (the Producer band gates) | their test classes now carry a nonzero refined cost, since a free class is no longer listed; the behaviour they test is unchanged |

The build-selection gate's over-maintenance case no longer needs to step
past Bounty Hunters. Its comment is updated.

## The browser comparison

`build-selection.mjs` (Empire, Linux Chromium) runs `classValuesCase`
on the troops band with the seven classes. The fixture's system has one
Training Facility, as Sluis Van. All four values are exact for each
class. Each picture (window 6..200 by 22..47) has 0 of 5070 pixels
differing from A0 at (255, 87).

Run: `.artifacts/interface-parity/build-selection-2026-11-10T00-51-41-556Z-3398273`.

## Batch b164

23 of 23 interface gates on Linux; workspace tests 3497 passed, fixture
tests 139; `build-selection` run `build-selection-2026-11-10T01-08-12-605Z-3436984`. The core diff's mutants
(`build_selection.rs`, run separately): 9 caught, 1 unviable.

Tactical matrix `2026-11-10T01-42-31-505Z-3490864`: 146 of 148. Both
failures were the trench run's movie fetch (`201.webm`, `202.webm`),
the known flake. Rerun alone, they pass 4 of 4 each
(`2026-11-10T01-56-09-951Z-3521697`, `2026-11-10T01-56-24-908Z-3522457`).
