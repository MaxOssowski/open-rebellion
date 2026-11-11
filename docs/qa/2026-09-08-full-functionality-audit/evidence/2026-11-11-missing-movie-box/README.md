# The missing-movie box (PRE-01-C009, END-01-C011)

Date: 2026-11-11. Unit tests only. No browser gate, WASM build or A0 run
was made.

## The original

This checks the 2026-11-09 trace (`2026-11-09-movie-failure-trace/`)
against the decompile corpus.

- **The failure.** `FUN_005f6920` asks `FUN_005f6be0` for the movie's size.
  For a Smacker movie that is `FUN_005f71f0`, which calls `SmackOpen`. When
  this fails, `FUN_005f6920` calls `FUN_0060a9e0(path, "PathName unknown to
  VideoManager")`. The string is at file offset 0x2aa044.
- **The box.** `FUN_0060a9e0` is `MessageBeep(0x10)` and then
  `MessageBoxA(NULL, path, caption, 0x1010)`.
  - 0x1010 is MB_OK | MB_ICONHAND | MB_SYSTEMMODAL: one OK button and the
    stop icon.
  - Enter, Escape and Space answer it, or a click on OK.
- **The wait.** `MessageBoxA` returns only when the box is answered. Only
  then does `FUN_005f6920` post 0x403, so the flow waits for OK.
- **What follows.** `FUN_00421790` handles 0x403 in the same branch as
  0x402, the natural end of a movie: it stops, posts 0x405 and calls the
  movie's callback. So the flow goes on exactly as after a played movie.
  The 2026-11-09 hyp is settled.
- **The path.** It is the movie window's `TooBigLocation\MDATA\MDATA.NNN`
  (`FUN_004213e0`, `FUN_00421ba0`).
- **Behind the box.** The movie window, black, already exists. So does a
  story movie's caption window, created in WM_CREATE before the movie
  starts.

## The port before

- **Native.** `open_cutscene` returned `None`, and the flow went on at
  once. It also put "cutscene skipped …" into the Message Index, which the
  original never does.
- **Browser.** A `<video>` load error set state FAILED, which the player
  treated as finished. No box was shown.

## The change

- **`crates/rebellion-render/src/movie_error_box.rs` (new).**
  - It holds the caption, the input decision (`MovieErrorBox::handle`), a
    layout centred in the 640 by 480 client, and the painting.
  - Enter, Escape and Space release answer the box. So does a press and
    release on OK. Any other click does nothing.
  - hyp: the look is a Windows 10 message box at 96 DPI and has not been
    compared with a capture.
- **`crates/rebellion-render/src/video_player.rs`.** `VideoPlayer` now wraps
  the native or browser backend.
  - `VideoPlayer::unopened(path)` makes a movie that is only the box.
  - A browser load failure turns a playing movie into the box.
  - `stop()` (skip) does not answer the box.
  - `is_finished()` becomes true when `close_error_box()` is called.
  - Logs: `[cutscene] error_box path=… caption=…` when the box opens, and
    `[cutscene] error_box answered path=…` when it is answered.
  - port: the box shows the port's file: the movie path natively, or
    `data/video/<stem>.webm` in the browser.
- **`crates/rebellion-app/src/main.rs`.**
  - On an open failure, `open_cutscene` returns `VideoPlayer::unopened`. The
    caller then enters the movie's Cutscene mode, as in the original, where
    the movie window exists.
  - The Message Index entry is gone, and `msg_log` and `tick` were dropped
    from its arguments.
  - `answer_movie_error_box` paints the box over the black movie client,
    after any story caption, and feeds it the keys and clicks. OK ends the
    movie through the usual end path: the main menu, the logos-to-cinematic
    chain, the ending, story movies, the trench run.
- **`web/index.html`.**
  - FAILED (3) now means only that the file could not be loaded: the
    `error` event, or a rejected `play()` with `video.error` set.
  - A new REFUSED (4) covers a browser that will not start the movie even
    muted. The game goes on as before, without the box.
- **`tools/interface-parity/prepare-site.mjs`** also stages 003, 004 and
  005, so that New Game and Credits do not hit the box on gate sites.

## Tests

`cargo test -p rebellion-render --lib -- movie_error_box video_player` and
`cargo test -p rebellion-app` pass.

| Test | What it covers |
|---|---|
| `movie_error_box::the_box_shows_the_originals_caption` | the caption and path |
| `movie_error_box::enter_escape_and_space_answer_the_ok_only_box` | MB_OK keys |
| `movie_error_box::only_a_click_on_ok_closes_the_box` | the system-modal box takes no other click; down on OK and up off it presses nothing |
| `movie_error_box::the_box_is_centred_and_ok_sits_in_its_button_band` | layout sanity, a port helper |
| `video_player::wrapper_tests::an_unopened_movie_waits_for_its_box_then_ends_like_a_played_one` | the wait, the skip and the end |

The three behaviour tests (keys, click, wait) were shown failing without
their change:
- Escape not answering the box;
- a release on OK pressing without a press;
- `is_finished` ignoring the box.

The files were restored afterwards (`cmp`).

`cargo check` is clean natively (`--all-targets`) and for wasm32 (with and
without `interface-test-fixtures`). No new Clippy warnings are in the
touched code.

## Browser gates

None of the gates covers a missing movie, and none was run. These play
movies on their own site and would now stop at the box if a movie were
missing there:
- `cutscenes.mjs`, whose server returns 404 for an absent file;
- `story-movie.mjs`;
- `ending.mjs`;
- `run.mjs` (trench run, 201/202);
- the gates that boot through the intro without a fixture:
  `main-menu-states.mjs`, `main-menu-rest.mjs`, `game-options.mjs`,
  `sector-windows.mjs`, `game-options-smoke.mjs` and the `tooltip.mjs`
  main-menu case.

The batch site must ship every movie those gates reach.

## Not done

- `MessageBeep(MB_ICONHAND)`, the system's Critical Stop sound, has no
  asset in the port. Only the log line stands for it.
- The box's look and its cursor are not compared with A0. The movie window
  hides the cursor (`FUN_00414830`). Whether it stays hidden over the box
  is not known, and the port shows it.
- No A0 capture exists. One could be made by renaming an MDATA file in the
  VM.
- The native decode of a frame that fails during playback still ends the
  movie silently. The original's failure path covers only the open.
- `open_cutscene` itself (open failure to box) has no unit test: it needs
  the native audio engine. The wrapper test covers the box's behaviour.

## Batch b184

- In-place mutants on the render diff missed 12 in `video_player.rs`:
  - the `web::VideoPlayer` ones (`load_failed`) are compiled only for
    wasm32, so the native test build cannot tell them apart;
  - native `load_failed -> true`, and the wrapper's `advance`,
    `current_frame`, `is_finished` and `stop` on a playing movie, need a
    real movie and the native audio engine; the browser gates below play,
    skip and end real movies through the same wrapper;
  - `log_box` only logs.
- 22 of 23 interface gates passed at once. `fleet-registry` failed: the
  site now ships the new game's movie (004), which plays where the
  missing file used to end at once, and the gate waited a fixed 4
  seconds. It now skips the movie with Escape, as a player would, and
  passes: `fleet-registry-2026-11-11T04-01-34-840Z-3573393`.
- Outside the batch: `cutscenes` (`cutscenes-2026-11-11T03-55-54-221Z`),
  `main-menu-states` (`main-menu-states-2026-11-11T04-00-53-426Z`),
  `main-menu-rest` (`main-menu-rest-2026-11-11T04-01-23-495Z`) and
  `game-options` (`game-options-2026-11-11T04-00-04-015Z`) pass.
- Tactical matrix `2026-11-11T03-42-32-154Z-3537737`: 148 of 148.
