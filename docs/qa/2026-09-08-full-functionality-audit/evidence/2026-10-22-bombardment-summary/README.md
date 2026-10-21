# The orbital bombardment's summary (F-006)

Date: 2026-10-22. The reading is in `ghidra/notes/assault-summary-window.md`,
"Ported (2026-10-22): the bombardment summary".

## Change

- `assault_report.rs`: `AssaultReport::start_bombardment` (notification
  `0x14f`, `+0x64` 0) and `finish_bombardment`. The second files the
  targets the bombardment destroyed under the system's Destroyed (`+0x80`)
  and the bombarding ships the batteries ended under the attacker's
  (`+0x88`). `title` gives `0x7098`, `body` gives `0x7099` (held) or
  `0x709a` (neutral). `picture` gives `0x2b9b`, `0x2b9c` or `0x2b9a`
  (`FUN_00496770`). `first_category` gives Capital Ships (`0x97`).
- `BombardEnv::bombard_reported` keeps the report. A Death Star's
  destruction of the system makes none.
- `story_messages.rs`: `MessageParameter::Unit` answers kinds 1, 2 and 4
  with the name (`FUN_004f6030`), so "Orbital bombardment of {1:4}" names
  the system.
- `rebellion-app`: the player's and the AI's bombardments go through
  `bombard_reported`. Their reports open the window for the attacker's
  side, or file a Conflict message for the other side, as assaults do.

## Gates

- Tests: 4 new (core 3, render 1). Workspace 2599 passed, 0 failed; the
  long-standing data test
  `seeds::tests::fixed_fleet_tables_only_target_special_systems` still
  fails as before.
- Mutants (diff of core, data and the two render files): 34 caught,
  0 missed, 3 unviable.
- `provenance-scan` 0 uncited.
- 24 dual-AI campaigns identical to HEAD.
- Browser, Arch Chromium 155.0.8059.84: Fleet window 58 of 58 pass. The
  `bombardment` case now waits for the summary ("Orbital bombardment of
  Akrit'tar", Capital Ships), then closes it with Escape. The
  `destroy-system` case checks that no summary opens
  (`fleet-window-bombardment.json`, `*.png`). Interface WASM
  `201b473e67c5fcc32aa3dbcf785d1a845c0995ba84f9d47ca019c376ae5fc914`.
- Replay equivalence passed. Frame timing: p99 0.8 / 1.7 ms, 0 frames
  over 50 ms.
- Release WASM `9d5723defe39b93dc221999fa4f44e29c2a3f06bc3408ba531fcc01a52610a67`.
