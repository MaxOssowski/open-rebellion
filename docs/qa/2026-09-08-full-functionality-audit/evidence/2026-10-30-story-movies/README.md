# Story movies 101–104 and their captions

Date: 2026-10-30. Feature F-008 (cutscenes). The story movies were the
last movies the port did not play. No surface-ledger cell covers them.

## The original

The trace is in `ghidra/notes/movie-flow.md`, "Story movies".

**Which messages are movies.** Four story message classes report kind 6
through their slot `+0x10`:
- System Destroyed by the Death Star (`FUN_004950d0`, vtable `0x65b758`);
- Alliance Headquarters Destroyed (`FUN_0048e5a0`);
- Coruscant Captured (`FUN_0048f790`);
- Death Star Sabotaged (`FUN_0048d470`).

**Kind 6 files only a movie.** `FUN_0048a060` sends kind 6 through the
jump table at `0x48a168`, entry 5, to `0x48a149`. That branch only adds
the message to the message manager's movie list `+0x28`. This was checked
by hand in the disassembly. So these four post no Message Index row, no
alert and no advisor event.

**Playback** (`FUN_00488b90`): while the game is not paused, it plays the
oldest movie, one at a time:
- the galaxy window posts `0x482`;
- the music stops;
- `FUN_004019a0(+0x5c, +0x50)` plays the movie under its caption.

| movie | message | caption (body) |
|---|---|---|
| MDATA.101 | System Destroyed by the Death Star | `0x70c9` (a side's system) or `0x70ca` (a neutral one) |
| MDATA.102 | Alliance Headquarters Destroyed | `0x7189` |
| MDATA.103 | Coruscant Captured | `0x7171` |
| MDATA.104 | Death Star Sabotaged | `0x71be` |

**The caption** (`FUN_0041ecf0`) is a 580 × 70 field at (4, 404), under
the letterboxed movie, which fills rows 78..401. It is white font 4 on
black, starts at the top left and wraps by words.

**After the movie,** the same galaxy view returns and its music restarts.

## The port

- **Queue:** `MessageLog::queue_story_movie` and `take_story_movie` hold
  the list.
- **Builders:** `victory_conditions_movies`, `death_star_sabotaged_movie`
  and `death_star_destroyed_movie` replace the three message builders.
  Before, they posted ordinary messages on the Mission or Story rail.
- **Playback (`main.rs`):**
  - the oldest movie plays as `CutsceneKind::Story` when the galaxy view
    runs unpaused;
  - `draw_story_caption` draws the caption;
  - the movie's end returns to the galaxy view and its music.
- **Movie placement:** every movie now fits into the 640 × 480 client
  (`movie_client`). In the browser, the `<video>` covers only that 4:3
  client and is transparent around the movie, so the canvas's black and
  the caption show through.
- **Fixture 78 `StoryMovie`** queues Death Star Sabotaged and runs the
  clock.
- **Port differences:**
  - the clock holds during the movie, while the original's probably keeps
    running (hyp);
  - a waiting movie is not saved;
  - the caption field's scroll bar is not drawn.

## Browser (Arch Chromium 155, interface WASM `62a8d63b…`)

**`story-movie.mjs`** passes for both sides (`story-movie-result.json`).
For each side:
- MDATA.104 starts by itself.
- The log names its caption, "The Rebel Alliance has sabotaged the Death
  Star at Endor.".
- The screenshot shows:
  - about 800 white caption pixels in the field;
  - the movie in rows 78..401;
  - nothing else.
- No message is posted.
- The fixture's galaxy has no Alliance headquarters, so the first day's
  victory word queues MDATA.102 behind it. Escape ends 104 alone; 102 then
  plays under its own caption, and a second Escape returns to the cockpit.

**Other gates:**
- `cutscenes.mjs` passes on the release site (`cutscenes-result.json`,
  WASM `f3322e7f…`): the intro, the skips, the credits and the new-game
  movie, now in the 4:3 client.
- `message-display.mjs` passes (`message-display-result.json`).

Movie hashes (`web/data/video`, from `assets/references/ref-videos`; first 16
hex digits of sha256): 101 `a3d030669cfca3ae`, 102 `6c4959f6c97c1a4a`,
103 `251780b5036b3e6f`, 104 `8f817e66067c439c`.

**Not done:**
- no comparison with A0 (no VM capture of a story movie yet);
- no native GUI run.

## Tests

- **Changed tests.** These expectations change, with the evidence above
  (`0x48a149`):
  - `a_victory_conditions_change_queues_the_hq_and_coruscant_movies`
    (formerly `..._posts_the_hq_and_coruscant_messages`);
  - the Death Star Sabotaged half of
    `the_assassins_and_death_star_sabotage_messages_name_their_places`.
- **New tests:**
  - `story_movies_wait_in_order_and_post_no_message`;
  - the Story cases in
    `any_key_skips_a_movie_but_the_trench_runs_take_only_escape` and
    `a_finished_cutscene_restarts_the_score_of_the_mode_it_returns_to`;
  - the destroyed-system movie in
    `the_death_star_names_the_side_whose_system_it_destroyed`.
- `cargo test --workspace`: all pass.
- `cargo mutants --in-place` on the five new functions: 14 mutants, 8
  caught, 4 unviable and 2 missed (the queue's two). After the queue test
  was added, a rerun on the queue caught both.
