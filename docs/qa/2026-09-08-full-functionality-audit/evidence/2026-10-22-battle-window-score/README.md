# The battle windows play the original score (F-006 / TAC-07)

Date: 2026-10-22. The reading is in `ghidra/notes/battle-alert-window.md`,
"The battle windows' score".

## Change

- `rebellion-render/src/audio.rs`: `ORIGINAL_MUSIC` is `FUN_00417520`'s
  table at `0x006a8478`, read from REBEXE.EXE. `MusicTrack::Original`
  names a track in it.
  - `battle_alert_music` picks track 10 or 11 (`FUN_0044f860`): MDATA.307
    for the Alliance, MDATA.313 for the Empire.
  - `battle_summary_music` picks the track `FUN_0046f360` sets: the
    viewer's win plays 12 or 14 (MDATA.308, MDATA.314); otherwise 13 or
    15 (MDATA.309, MDATA.315).
- `TacticalState::screen_music` says which of them the shown window
  plays.
- `rebellion-app`:
  - `music_file` maps an original track to its staged `NNN.wav`.
  - The main loop plays the window's track while the Battle Alert or the
    Battle Summary shows, and restores the galaxy's or the menu's score
    when it closes.
  - The tactical battle keeps its MDATA.307 cue.
  - The user's uncommitted window-configuration change in `main.rs` is
    left out of this commit.
- `scripts/build-wasm.sh` stages MDATA.307, 313, 308, 309, 314 and 315 as
  `music/NNN.wav` for the browser. The runtime pack grows from 51.7 MB
  to 57.5 MB.
- `run.mjs`: the Battle Alert probe requires the 307/313 log, and the
  results probe the 308/314 log (both fixtures are the viewer's win).
- Before this change the Battle Alert kept the galaxy's score, and the
  Battle Summary kept the tactical battle's.

## Gates

- Tests: 3 new. Workspace 2643 passed, 0 failed.
- Mutants (render and app audio, tactical view): 9 caught, 3 unviable,
  0 missed. `main.rs` was left out, as it carries the user's change.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), all
  4 of 4 cases each:
  - `production-battle-alert-entry`: the alert logs mdata 307
    (Alliance) or 313 (Empire), then Take Command logs the combat
    track 307;
  - `production-battle-results-presentation`: mdata 308 or 314;
  - `production-tactical-audio-routing`: unchanged.
  See `score-logs.json`.
- Replay equivalence passed.
- Release WASM `d3b23362c8c5448e39b744af6d1131032eab55147c3ec0fe4c2ac50e652585fe`.
