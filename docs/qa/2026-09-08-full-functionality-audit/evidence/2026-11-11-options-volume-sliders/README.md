# Game Options volume sliders: transfer, thumb and defaults (PRE-03-C009, PRE-03-C010)

Date: 2026-11-11. Ledger cells: PRE-03-C009 (music slider) and PRE-03-C010
(sound slider). This stage has unit tests only. No browser gate, A0 run or
WASM build was made.

## The original

Sources: the decompile corpus (`FUN_<addr>.c`) and
`ghidra/notes/game-options-text.md` §4. The 2026-11-11 additions there are
marked.

- **Sent value.** Commands 0x8d (music) and 0x8e (effects) in
  `FUN_00406b80` turn the slider percent v into a DirectSound volume:
  `floor(log10(v * 0.01) * 2000)` hundredths of a decibel, or -9600 for 0.
  - Music goes to `FUN_00401aa0` -> `FUN_00417670`, which writes
    `MusicVolume` = -vol and applies it to the stream.
  - Effects go to `FUN_00401a60` -> `FUN_00416360`, which writes `Volume`
    and calls `FUN_00610cd0`.
- **Opened value.** `FUN_00407180` (0x40773b..0x407780) gives 0 below
  -9600, else `floor(10^(vol * 0.01 * 0.05) * 100)`. Both directions floor,
  so a percent other than 1, 10 and 100 reopens one lower (49 -> -620 -> 48).
- **Defaults.** `FUN_005f41b0` returns 0 for an absent registry value.
  `FUN_004173c0` (MusicVolume, MusicSwitch) and `FUN_00413000` (Volume)
  therefore start at 0 dB: full volume, shown as 100. A0 `200-options`
  shows the music thumb clipped at the track's right end, as at 100.
- **Interaction.** `Class_0066e010_vt14`, `FUN_00604950`, `FUN_006048f0`
  and `FUN_00601080`:
  - A press anywhere in the 201 by 54 window takes the capture
    (`SetCapture`). It moves the thumb when the pointer is strictly inside
    and between the 5-pixel margins. It sends nothing.
  - A held move sends `+0xdc` while the pointer is strictly inside
    (`FUN_006048f0`).
  - The release sends `+0xdc` wherever the pointer is.
  - The thumb centre is the pointer itself (`+0xc8`). It stays there until
    the window is built again.
  - `+0xdc` = `cx * 100 / 201` of the last paint. So a click that moves
    nothing sends the opened thumb's value: 48 opens at x 96 and sends 47.

## The port before

- The gain was linear, v / 100, within 0.12 % of DirectSound's amplitude.
- A reopened slider showed v rather than the original's v - 1, a 2-pixel
  thumb offset.
- During a drag the thumb was drawn at `floor(201 * v / 100)` rather than
  under the pointer. At slider x 100 that put its centre at 98, not 100.
- The volume changed on the press. A press on a key-coloured track pixel
  did nothing. A click in the margins sent nothing.
- The music default was 0.8, where the original's is full.
- Persistence was already in place (`rebellion_data::settings`: native
  `saves/settings.json`, browser `localStorage` `rebellion_settings_v1`).

## The change

- `crates/rebellion-render/src/game_options.rs`:
  - `slider_hundredths` (`FUN_00406b80`) and `slider_percent`
    (`FUN_00407180`) do the two conversions.
  - `hundredths_gain` gives DirectSound's amplitude `10^(vol / 2000)`, which
    feeds the mixer.
  - `gain_hundredths` is its exact inverse. It carries a port: tag for
    other amplitudes.
  - `GameOptionsState::slider_thumbs` holds the thumb under the pointer for
    the current visit. `set_origin` resets it, since `FUN_00407180` rebuilds
    the sliders at each opening.
  - `move_thumb`, `send_slider` and `inside_slider` replace `set_slider`.
  - The press, move and release dispatch follows `Class_0066e010_vt14`.
- `crates/rebellion-render/src/audio.rs`: the default `music_volume` is now
  1.0.
- `crates/rebellion-data/src/settings.rs`: doc comments only. The stored
  volumes are now the amplitudes of the hundredths. The format is unchanged
  (two f32 fields), so there is no save or settings version bump. An older
  linear value such as 0.49 reads as the nearest hundredth (-620), which
  opens at 48.
- `tools/interface-parity/game-options.mjs` has new expectations, which
  have not been run:
  - The dragged thumb starts at 392 + 95 (centre 100, under the pointer).
  - The stored music amplitude rounds to -620 hundredths.
  - After the reload the thumb starts at 392 + 91 (centre 96, percent 48).
  - The result field `reloaded_slider_differing` became `reloaded_columns`.
- Ghidra notes: `ghidra/notes/game-options.md` (Persistence) and
  `ghidra/notes/game-options-text.md` §4 (interaction).

## Tests

`cargo test -p rebellion-render --lib` passes: 1419 tests, 6 ignored.
`cargo test -p rebellion-data --lib settings` passes: 4 tests.
`cargo check -p rebellion-app` is clean.

New or rewritten tests:

| Test | What it checks |
|---|---|
| `game_options::a_slider_percent_becomes_floor_2000_log10_hundredths_of_a_decibel` | the sent value |
| `game_options::a_slider_opens_at_floor_100_times_ten_to_the_volume_over_2000` | the opened value |
| `game_options::a_sent_percent_reopens_one_lower_except_exact_powers_of_ten` | the round trip |
| `game_options::the_mixer_gain_is_directsound_s_amplitude_for_the_volume` | the gain and its inverse |
| `game_options::a_slider_opens_its_thumb_at_the_percent_its_volume_reads_back_as` | replaces `a_slider_thumb_centres_on_its_whole_percent`; 0.5 now opens at 98, per `FUN_00407180` |
| `game_options::a_slider_drag_takes_only_points_inside_its_five_pixel_margins` | rewritten: the thumb at the pointer, the y bounds, the amplitude, the reset on reopening |
| `game_options::a_click_that_does_not_move_the_thumb_sends_the_opened_percent` | a click that moves nothing |
| `game_options::a_slider_takes_a_press_anywhere_strictly_inside_its_window` | the `FUN_006048f0` bounds |
| `audio::both_volumes_start_full_without_stored_settings` | the defaults |
| `audio::music_toggle_preserves_sound_effects` | expectation changed from 0.8 to 1.0, per `FUN_004173c0` and `FUN_005f41b0` |

Each was shown failing without its change:
- **Old behaviour together.** With the old thumb formula, the linear gain
  with no send before a drag, the inclusive slider rectangle and the 0.8
  default restored, six tests fail: the two `audio` tests, the click, the
  drag, the opened thumb and the press bounds.
- **Linear gain alone.** Restoring only the linear gain fails the drag and
  the click tests.
- **Pure conversions.** The four conversion tests cover functions that did
  not exist before. Without the change they do not compile.

The files were restored afterwards and are byte-identical to the
pre-mutation copies (`cmp`).

## Not done

- **Not run:** `cargo mutants`, `provenance-scan` and the browser gate
  `game-options.mjs`. rebellion-core is untouched, so `provenance-scan` does
  not apply. The gate's new expectations need a WASM build and a Linux run.
- **Audibility:** not measured against A0. There is no recording of the
  original's level at a known slider setting.
- **x87 precision (hyp):** the arithmetic is f64, the MSVC default 53-bit
  control word. Under 64-bit precision the percent 10 may reopen as 9, and
  1 as 0. Direct3D may also lower precision after a battle. A0 can settle
  this: drag to x 20 (10 %), reopen, and read the thumb.
- **Rejected pointer points:** the original writes them to `+0xc8`, so a
  later full WM_PAINT would show them. The port does not model this.
- **Tactical music:** `FUN_00596b90` also gets the music volume in
  tactical states. It is not traced here.
- **Status-bar slider:** the port's own egui control
  (`audio::draw_audio_controls`, a replacement panel) still writes linear
  amplitudes. The Game Options window reads them as the nearest hundredth.

## Batch b181

23 of 23 interface gates on Linux; workspace tests passed, fixture
tests 140; tactical matrix `2026-11-10T23-58-12-701Z-2551780`: 148 of
148. `game-options.mjs` (not in the batch, run with
`--a0=.artifacts/a0-vm/tooltip`) passes with the new expectations:
`.artifacts/interface-parity/game-options-2026-11-11T00-13-32-352Z`.

In-place mutants on the render diff missed 16 mutants in
`game_options.rs`. Two (`slider_percent`'s `<` to `==` and to `<=`) are
equivalent: every volume below -9600 already floors to 0. The others
are closed in the next commit: the held move and the press target leave
the frame code as tested functions, the thumb's clip gets its own
helper, and `gain_hundredths` and `inside_slider` gain edge cases. The
remaining overlay ones (`!state.modal()` in the tooltip and the name
boxes) are drawing code that `game-options.mjs` checks against A0.

## game-options-smoke.mjs

This gate is not in the batch and has not run green for some time. Its
save keys named format version 14, and the save format is far past
that. They now take the version from the first key written. Its clicks
still follow an older main menu: in this run the game closed before
the campaign opened (`alliance-campaign-options.png`). It needs its own
rewrite; the panel steps removed here are listed in
`2026-11-11-load-game-screen-removed`.
