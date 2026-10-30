# The original movie flow: intro, new game, credits, ending (PRE-01, PRE-04, END-01)

Date: 2026-10-30. This follows `../2026-10-30-browser-cutscenes/`, which
made the browser play movies at all. The trace is in
`ghidra/notes/movie-flow.md`.

## The original

The movies are `MDATA\MDATA.NNN` Smacker files; the port's
`ref-videos/NNN.webm` are their conversions. 000 is 640 × 480; the others
are 640 × 324 with black bars.

| When | Code | Movie | Then |
|---|---|---|---|
| Start: the logos (LucasArts, Coolhand) | `FUN_004138a0`, state 1 | 000 (17 s) | state 2 |
| Start: card, STAR WARS title, crawl, opening cinematic | state 2 | 001 (169 s) | the main menu (state 3) |
| A side is chosen | state 7 | 003 (side 1) or 004 | the game (state 8) |
| Main menu credits (`0x73`, `FUN_00406000`) | `FUN_004019a0(0x6e)` | 005 (2:00) | the menu and its music |
| The game's end, Alliance player | `FUN_00425c90`, states 0x10/0x11 | 105 (Alliance wins) / 106 (Empire wins) | the main menu (state 3), game reset |
| The game's end, Empire player | states 0x12/0x13 | 108 (Empire wins) / 107 (Alliance wins) | the main menu |

**Skipping.**
- Escape or the left button ends any movie (`FUN_005f7240`).
- Any other key ends every movie except the trench run's 201/202
  (`FUN_00421790`'s `WM_KEYDOWN`, ids 0x70/0x71).
- A skip ends only the playing movie: skipping 000 starts 001.

**There is no end screen.** No dialog, continue or restart was found. The
credits are the movie; there are no credit strings.

## The port

`main.rs`:
- **`CutsceneKind`** has `Intro` (000) chained to `IntroCinematic` (001),
  `NewGame` (003/004, which then hands over to the existing start),
  `Credits` (005) and `Ending(105..108)` (`ending_cutscene`), plus the
  trench run.
- **Removed:**
  - the replacement victory modal ("Alliance Victory!") and the scrolling
    credits screen;
  - the story triggers that played 101..108 on Luke, Han and Vader events.
    105..108 are the endings. 101..104 belong to the Death Star destroying a
    system, the Alliance headquarters destroyed, Coruscant captured and the
    Death Star sabotaged, and they are not yet ported.
- **Skipping** follows `key_skips_cutscene`: Escape or a left click, or any
  other key outside the trench run.

## Browser (Arch Chromium 155, release WASM `b3c355c9…`)

`tools/interface-parity/cutscenes.mjs` passes all three cases
(`cutscenes-result.json`). The movies' hashes:
- 000 `0af15384…`
- 001 `8e18ded0…`
- 004 `292ed78e…`
- 005 `4d678ae3…`

| Case | Checks | Screenshots |
|---|---|---|
| `natural-end` | 000 plays to its end and 001 follows; Escape then ends it and the main menu shows | logos `36297e8d…`, cinematic `867ee4c7…` |
| `skips` | Escape on 000 starts 001, the menu still waiting; the key X ends 001 and the menu shows | `6137936a…`, `abf1e2d2…` |
| `credits-and-new-game` | the menu's Credits plays 005 and a click returns to the menu; Start as the Empire plays 004, then Escape starts the game | credits `e77e3e84…` (the credits text over stars), new game `768ab5f2…` |

## Still open

- **A0.** The VM is in a game, not at startup, so none of this flow has
  been compared with A0 yet.
- **The ending in a browser case.** It needs a fixture that ends a game;
  `ending_cutscene` is covered by unit tests.
- **The story movies 101..104** and their caption box.
- **Missing media.** The original shows "PathName unknown to VideoManager"
  and goes on; the port logs and goes on.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `the_games_end_plays_the_movie_of_its_winner_for_the_players_side`;
  - `any_key_skips_a_movie_but_the_trench_runs_take_only_escape`.
- Changed expectation: `a_finished_cutscene_restarts_the_score_of_the_mode_it_returns_to`.
  The logos no longer return to the menu's score, since 001 follows them.
