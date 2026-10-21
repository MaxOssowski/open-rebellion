# The Assault Summary window (F-006)

Date: 2026-10-22. The reading is in `ghidra/notes/assault-summary-window.md`.

## Change

- `rebellion-core/src/assault_report.rs`: `AssaultReport` collects what
  the original's report message (vtable `0x0065b870`) collects: the
  system's units (`0x150`) and the attacking fleets' ships, squadrons,
  regiments and personnel (`0x182`). It files the assault's losses under
  Destroyed (`0x302`) and reads whether the attacker now holds the system
  (notification 2, `FUN_00496770`). It picks the outcome text
  (`0x7091..0x7094`), the picture (`0x2b98`/`0x2b99`) and whether the
  window opens at once (`FUN_00497530`).
- `BombardEnv::assault` keeps one report per assault.
- `rebellion-render/src/assault_summary.rs`: the window (`FUN_0044c410`,
  `FUN_0044c630`). It has the Battle Results frame and rail, Close, the
  pages Summary / attacker's forces / System Forces / Display, six
  categories with Operational and Destroyed columns (Survivors, Captured,
  Killed for Personnel), and the Display page's system and fleet buttons.
  Escape closes it; Left and Right step through the categories
  (`FUN_0044e180`).
- `rebellion-app`: the attacker's side sees the window at once. The other
  side gets a Conflict message with the title, the outcome and the
  picture. Displaying it in the Message Index opens the window and closes
  the index (`FUN_00469de0`). The Escape chain leaves the key to the open
  window.

port and hyp notes are in the reading's "Ported" section. The bombardment
summary is not ported.

## Manual cross-check (p. 123, Figs. 3.66, 3.67)

| Claim | Status |
|---|---|
| The window comes up when a fleet attempts a planetary assault | settled: the attacker's side, kind 7 |
| It is a message when the opponent assaults | settled: kind 3, Conflict rail, opens the window |
| Imperial / Alliance Forces buttons show a damage summary | settled: `0x66` the attacker's, `0x67` the system's |
| Tabs for capital ships, fighters, manufacturing, defensive facilities, troopers, personnel | settled: six categories |
| Operational and destroyed troopers | settled: two columns |
| Close window, Go to system | settled: Close, Display page `0xb` |

## Gates

- Tests: 26 new (core 8 + 1 extended, render 17). Workspace 2595 passed,
  0 failed; the long-standing data test
  `seeds::tests::fixed_fleet_tables_only_target_special_systems` still
  fails as before.
- A new test found a bug, which is fixed: the arrow-key check reset a
  clicked category.
- Mutants: the core diff has no survivor (the first run's misses were
  all drawing code in `assault_summary.rs`); after the egui tests,
  `assault_summary.rs` has 181 caught and 0 missed (6 unviable).
- `provenance-scan` 0 uncited.
- 24 dual-AI campaigns identical to HEAD (the report only reads the world).
- Browser, Arch Chromium 155.0.8059.84 (temporary Linux manifest): Fleet
  window 58 of 58 pass. The `assault` case now waits for the window and
  checks its title and outcome. It opens the attacker's forces and the
  System Forces, then closes the window with Escape, for both sides
  (`fleet-window-assault.json`, `*-assault-*.png`). Interface WASM
  `0f377d272a901d619993b0c8d9db841937ef673f8a9f13f6a16b552ca36aa4c1`.
- Replay equivalence passed (`replay-equivalence.json`). Frame timing:
  p99 0.8 / 1.6 ms, 0 frames over 50 ms (`frame-timing.json`).
- Release WASM `9e92120203555adcf29daf552018e623939fbd1466c05b24bb42af68ffcfa17c`.

The fixture's defending regiments use the player's regiment class, so
both columns name the same class.
