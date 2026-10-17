# The Jedi Training mission, 0x58 (F-038, F-040)

Date: 2026-10-18.

The port had no Jedi Training and no `ForceTraining`, so no character
could be trained. The reading is in `ghidra/notes/force-awareness.md`,
"Force growth".

## The original

- MISSNSD `0x58000042` (TEXTSTRA 11330 "Jedi Training", vtable
  `0x006667d8`): an own-side system, 60 + rand(0..=30) days, no repeat,
  detection on, resign allowed, characters of both sides.
- Teacher `+0xa8`: on entering phase 2, the phase slot `+0x1f4`
  (`FUN_00571410`) stores the first member who trains Jedi at Force
  GNPRTB 2581 (100) or more (`FUN_00570e40`, `FUN_004ed310`,
  `FUN_005703b0`).
- Rules (`FUN_00570690`, object checks `FUN_00570940`):
  - no teacher among the members refuses (`0x40`/`0x90`);
  - a member at Force 0 refuses (`0x40`/5);
  - a member other than the teacher at or above the teacher's Force
    refuses (`0x40`/`0xd`);
  - fewer than two members refuses (`0x40`/`0x94`).

  A failure writes -1 as the end code, which refuses an order and ends a
  running mission.
- Chance (`+0x274`, `FUN_005710b0` -> `FUN_0055c7c0`): teacher Force less
  the member's, in percent. The teacher's own is 0.
- Hit (`+0x27c`, `FUN_005712b0`): the pupil's Force becomes Force +
  rand(0..=(teacher - pupil) x GNPRTB 6173 (50) / 100) (`FUN_0055ca20`).
  The difference goes into `ForceTraining` `+0x90` (`FUN_004ee1d0`), and
  the result is 3 when it is not 0.
- Force `+0x8c` = `ForceExperience` `+0x8e` + `ForceTraining` `+0x90`
  (`FUN_004eee40`), clamped to `0x7fff`.

## Port

- `MissionKind::JediTraining`, record `0x58000042`, system target rules.
- `ActiveMission::teacher`, set on entering phase 2.
- `Character::force_training`, `add_force_training`. `force_level()` is now
  the sum (save v71).
- `can_teach_jedi`, `jedi_teacher`, `jedi_training_chance`, and the rules
  in `running_end_code` (end code `JEDI_TRAINING_REFUSED`, 0xff).
- Phase 10 rolls each member against the teacher and pushes
  `MissionEffect::ForceTrainingGained`, which the integrator and the app
  apply.
- The mission dialog lists it for a team that passes the rules (name
  TEXTSTRA 11330).
- port:
  - the dialog runs the object checks at creation, not the class-record
    ones (`FUN_00570bc0`);
  - no AI planner sends it: the AI's Jedi Training planner is not read.
    So the campaigns play the same games as before.

## Tests

- `missions.rs`:
  - `jedi_training_needs_a_teacher_at_force_100_and_weaker_pupils`: the
    dialog, every rule, and a running mission whose teacher left (0xff);
  - `entering_phase_two_chooses_the_first_member_who_may_teach`;
  - `a_jedi_training_pupil_gains_up_to_half_the_force_gap`: 200 seeds,
    gains in 1..=50 with 50 reached, success only with a gain, and the
    chance function.
- `world/mod.rs`: `the_force_level_sums_experience_and_training`.
- `integrator.rs`: `a_jedi_training_gain_raises_the_pupil_s_force_training`.
- Shown failing without the change (rules off, no gain, no teacher): the
  three mission tests fail.

## Campaigns

`rebellion-playtest data/base --ticks 600 --dual-ai`, seeds 1, 2, 3, 7,
13, 42 and 47, against `6309b70a`: all identical, as no AI sends the
mission.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`953867b02d67770e8cd7d17c2cee455643221beae1e7c8d554013927e4ea4b69`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 24.1 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.2 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The mission dialog was not opened for Jedi Training in the browser; the
  dialog's list is covered by the unit test above.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2315 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v71: the mission's
  teacher and the character's `ForceTraining`):
  - initial `v1:58473b5bf4582374` -> `v1:d077dd4c3c2b7247`;
  - final `v1:461650933b75014c` -> `v1:1d8dbb48d17c03de`;
  - two recordings are identical, and only the fingerprints changed in the
    manifest;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `3d96bf1d2b8d2e2f078e30bbc39418a2169ecf1bf12af70efe47a3191f9963ed`.
- `cargo mutants --in-diff`: 85 mutants, 75 caught, 6 unviable, 4 missed.
  - `gap > 0` -> `>=` is equivalent: a gap of 0 draws from 0..=0, so no
    gain.
  - `apply_panel_action` and `apply_mission_result` (app) are not called
    by the app's tests; the latter opens an audio device.
  - `convert_character` returning a default is the DAT loader's old gap.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the changed
  lines.

Hashes are in `SHA256SUMS`.
