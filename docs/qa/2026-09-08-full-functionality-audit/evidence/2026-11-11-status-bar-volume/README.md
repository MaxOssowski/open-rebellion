# The port's own volume bar removed

Date: 2026-11-11. The rule applied is the owner's 2026-11-10 decision: a
state that is absent from the original is removed after a trace.

## The original

The original sets the volumes in one place only: the Game Options window.

- **Volume setters.** The music setter `FUN_00401aa0` -> `FUN_00417670`
  and the effects setter `FUN_00401a60` -> `FUN_00416360` take a new value
  only from `FUN_00406b80`, commands 0x8d and 0x8e. Those are the window's
  two sliders.
- **Other callers.** `FUN_00405050`, `FUN_00422ce0` and `FUN_00417520` read
  the stored volume and apply it again on state changes (`FUN_00401a80` ->
  `FUN_00401a60`, and `FUN_00610cb0` with `DAT_006b14a8`). None of them
  takes a new value.
- **Mute.** The original has no global mute. Music is switched on or off
  by the Game Options button 0x99, `MusicSwitch`.
- **Manual.** The manual mentions volume only for the Game Options screen:
  Fig. 3.16 has the callouts "Music Volume" and "Sound Effects Volume", and
  the "SOUND EFFECTS VOLUME/MUSIC VOLUME" paragraph after it describes the
  sliders. It names no other volume or mute control.

## The port before

`rebellion_render::draw_audio_controls` drew an egui strip with a mute
toggle (a loudspeaker glyph), "Mus" and "SFX" sliders, and a "(no audio)"
label. The strip used linear 0..1 amplitudes. Its only caller was a
`main_menu_audio_options` bottom panel in `GameMode::LoadGame`.

No code assigns `GameMode::LoadGame`: the main menu's Load opens Game
Options. So the strip could not be reached.

## The change

The following were removed:
- `draw_audio_controls` and its egui import
  (`crates/rebellion-render/src/audio.rs`);
- its re-export (`crates/rebellion-render/src/lib.rs`);
- the bottom panel in `GameMode::LoadGame` and the import
  (`crates/rebellion-app/src/main.rs`).

The module doc now says that the volumes are set only in Game Options.
`AudioVolumeState` keeps its fields:
- `muted` serves `OPEN_REBELLION_MUTE`, a test switch;
- `music_muted` is the Play Music switch;
- `backend_available` selects the disabled music button art.

The audio engine and `rebellion_data::settings` are unchanged. The strip
had no keyboard shortcut and no test of its own. No test was removed.

## Tests

- `cargo test -p rebellion-render --lib`: 1420 pass, 6 ignored.
- `cargo test -p rebellion-app`: 140 pass, 3 ignored, plus 5 ignored.
- `cargo check -p rebellion-app --all-targets`: clean.

There is no new test, because no behaviour was added.

No browser gate under `tools/interface-parity` clicks or asserts the
strip. The search covered `draw_audio_controls`, `main_menu_audio`,
"Mus", "SFX", "no audio", the glyphs, and the save/load screen names. No
gate was run.

## Not done

- `GameMode::LoadGame` and its `draw_save_load` panel are a replacement
  state that cannot be reached. They remain for their own tracker item.
- No WASM build was made, and `cargo mutants` was not run.
