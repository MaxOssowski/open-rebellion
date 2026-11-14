# Research and recon dialogs, regiment transfer: A0 of 2026-11-14

Cells: OBJ-11-C011 (Reconnaissance), OBJ-11-C015 (Facility Research),
OBJ-10-C010 (regiment transfer), OBJ-10-C006 (special forces aboard).

## A0

Original REBEXE in the offline Windows 10 QEMU VM, the unsaved Alliance game of
2026-11-13-defenses-missions-drop, 2026-11-14 01:25..02:04 CET, days 400-540.
Captures stay out of Git: `.artifacts/a0-vm/2026-11-14-missions-transfer/` (86 PNGs,
README, SHA256SUMS). Key captures:

| SHA-256 | capture | what |
|---|---|---|
| `2093a8e8a5f7909598e1352cd53d995314b6af144cd1a79d670e63e240d163f9` | 12 | Adar Tallon on his own Jerijador (a Construction Yard alone): Create Mission opens on Facility Design Research; GOKRES 3105 at screen (238,124), 0 of 8,320 pixels differ |
| `e22b1b2101982dd716f9468b77288fbd9f025bb8bef64634bded0b7b1750cc38` | 13 | its kind list open |
| `0d1d5f597ea8b1fd9021383c279dd9e47e983ee90bab3d59f87f85d69d34fc88` | 14-4 | list: Diplomacy after Facility Design Research |
| `3c1d93763a6c3c918eea9e09e3809669601b4b79e6c056c92a952e63b6e56388` | 14-12 | list end: Espionage |
| `3dff304f02027706009d5c4f967014904d4c2241ab678bdcda35462140412810` | 09 | Tallon on Drall (shipyard alone): list ends Ship Design Research, Diplomacy, Espionage; no Troop or Facility research |
| `d870ce499d9d5cbc0eb0ca2801c5dc61aefcd48853280c25890fbface06ff768` | 90 | Jerijador's Training Facility Build Selection: regiments only, no special force |
| `f256fcf141273bbfa769a220042589630bd23aaa9052cea331ee812a5190dc0b` | 36 | a fleet regiment's Status: Attached Medium Transport 1, Enroute |
| `2f0ce0b5d03a015cbc0dbd2e6fcb34f4944e87f52f150061f99effc7ed4379e1` | 38 | Fleet 1 Status: Enroute, capacity troopers 2, embarked 1 |
| `e00b853b78c30a9f011ce0fd113a38253bf383bdc0d42c71796690fafb90dc5c` | 54 | drag onto an en-route fleet: nothing |
| `9e9e671eed229a7524e4b87c7095410271b4d8136050a3bbc964a315eefacf12` | 61 | menu Move onto an en-route fleet: nothing |
| `211d4a4deea6036d8eec3befb915ad377b26ba98a29ac5aabc8fa6e3a76f0f62` | 72 | at rest: the regiment's drag image over Fleet 1's entry |
| `679a25f638c485984b88f44d0a11f5726220d060a53c4f25fedb1e0276274d6e` | 74 | loaded at once: Troops tab lists Fleet and Army Regiment |
| `8a049e1b37cde49a4faa5eb40af0829dfa837c6e6ac7c0afe46261ac41bd9f05` | 78 | dragged back onto the Defenses window: unloaded at once |
| `c31fd7055c0969b901107fbccacd34d3dc5763443bf7ad12b342d5944c0bfc51` | 82 | loaded by the regiment's menu Move |
| `56cb2e1cdad5cf29981c074afaee8bcb94da0cba8133a366b67546ec969523b6` | 85 | the full fleet (capacity 2) refuses a third: nothing on screen |

## Findings

- **Facility Design Research (C015).** A0 lists it first, then Diplomacy and
  Espionage (`FUN_0046cab0` keys 5, 7, 9), for a facility designer on his own
  system with a construction yard and support below full; a system with a
  shipyard alone offers Ship Design Research only of the three trees
  (`FUN_0056c510`: a working yard of the tree's area). The port matched
  before this change; the new fixture and gate case assert it.
- **Reconnaissance (C011).** Not reachable in this A0 game: the Alliance has no
  recon special force and its Training Facility builds no special force. The
  dialog for this kind still has no A0.
- **Transfer (C010).** A0 moves a regiment between the planet and a fleet at
  rest by drag either way and by menu Move, at once, with no window or
  message; a fleet en route refuses both directions, and a full fleet refuses,
  with nothing on screen. The port matched on every point (gate cases load,
  drag-load, unload, full; core tests for the en-route refusals,
  `FUN_00553410` `1`/`0x21`, `FUN_004f9860` `1`/`1`). The en-route tests now
  cite A0.
- **Special forces aboard (C006).** Not reached: no special force is buildable
  and the Bothan Spies are on a mission.

## Port changes

- `Scenario::FacilityResearchTargeting` (120, gate code 121): the research
  fixture with a facility designer, one construction yard, support below full.
  Test `the_facility_research_scenario_boxes_facility_research_then_diplomacy_then_espionage`
  (both sides).
- `mission-dialog.mjs` case `facility-research`: kinds and GOKRES 3105/7201 at
  (70, 86).
- `fleet-window.mjs` case `drag-load`: a regiment dragged from the Defenses
  window onto the fleet entry boards at once (A0 71..74).

## Port runs (Arch Chromium 155.0.8059.84, not the pinned build)

WASM `6172b8d897cb684d93be780dee65222cbceb7a1c4ee4c90e17e9b3f33aa0c06f`, from the
worktree at base 9ece9168 plus this change.

- `mission-dialog.mjs --only=facility-research,recon,research-diplomacy,troop-research`:
  pass, both sides; every kind icon 0 differing pixels at (70, 86).
- `fleet-window.mjs --only=alliance/load,alliance/drag-load,alliance/unload,alliance/full,empire/drag-load,empire/unload`:
  pass.

## Checks

- `cargo test -p rebellion-app --features interface-test-fixtures`: 192 passed
  (`SCENARIO_COUNT` 119); `cargo test -p rebellion-core troop_transport`: 40 passed.
- `cargo mutants --in-place --in-diff` on the fixture diff: 14 mutants, 12 caught,
  1 unviable, 1 missed: `emit_mission_flow` replaced with `()`, the browser
  telemetry the mission-dialog gate reads (the gate cannot pass without it); no
  unit test observes it. The research scenarios' yard and skills are now asserted
  (killed the `is_shipyard` and skill mutants).
- `provenance-scan check`: 0 uncited items.
