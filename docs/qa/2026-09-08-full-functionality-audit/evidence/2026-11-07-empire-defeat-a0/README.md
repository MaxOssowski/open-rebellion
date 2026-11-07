# The Empire's defeat in A0 (2026-11-07)

This covers END-01-C004 (Imperial defeat) and C007..C010 (natural completion, continue, restart, return to menu), as A0 evidence. No browser case ends a game yet, so these stay partial.

## A0

A new Easy Empire game, "Standard Game". The captures are in `.artifacts/a0-vm/empire-clean/`:

| Capture | What it shows | sha256 |
|---|---|---|
| `60-summary-destroyed-coruscant-day800` | Day 800, the Alliance's second attack on Coruscant: "The Imperial fleet is defeated." / "Coruscant is now under blockade by Alliance forces. The Imperial fleet has been completely destroyed." | `f6e76fc5a263f3ab4d405ecce51b913b36a21fadbbdf91dea075d33a0d9c04b7` |
| `61-movie-coruscant-seized` | The Escape from the summary was followed by MDATA.103 captioned "The Alliance has seized the Imperial capital system of Coruscant." (caption ink x 4..380, rows 407..419) | `5d5ee12b91b91498c0b1b2d885d68e859fce68d517aa7e6ae367986b4c6cc9c1` |
| `62-defeat-movie` | A few seconds of the galaxy, then the Empire's defeat movie | `fab801b5cffd84adbca49264972e56585d915c9e9553390593bebeba7865e5d0` |
| `63-defeat-crawl` | Its crawl, "The Emperor and Lord Vader are lost..." | `53518b8ccb6c57cac6fc15a489a1b26315eeee8389ba17dc3a090c7fcf00423a` |
| `64-main-menu-after-defeat` | The main menu ("Standard Game") | `11636ecc979456d18ebfbecc13c0d8ad7a2938c7ce4930145a430363d9ece1a9` |

The defeat movie ran to its end without a key, and then the main menu came. No dialog came, and no Continue or Restart.

This matches `ghidra/notes/movie-flow.md` (state 0x13, MDATA.107, then state 3) and the port's `ending_cutscene(true, false) == 107` and its return to the main menu.

**Not settled.** The game was Standard. In the port's reading (`victory::conditions_word`), the Alliance also needs the Emperor and Vader captive. Both stood on Coruscant on day 712, and whether they were taken with it was not observed.

## The port in the browser (b118)

**Before.**
- No browser case ended a game.
- The victory check opened the ending movie in the same tick as the victory conditions word. So the word's story movies (MDATA.103 for Coruscant, MDATA.102 for the headquarters) never played ahead of it.
- In A0, 103 played first and then the defeat movie (`61`, `62`).

**Now.**
- `main.rs` keeps the chosen ending as `pending_ending`. It opens the movie once the galaxy shows, no cutscene plays, and the story movies queued ahead have played (`FUN_00422ce0` waits on pending animations).
- The new fixture scenario `EndingDefeat` (97, code 98) sets the other side's win conditions:
  - for the Empire's player: Coruscant the Alliance's, without the Empire's regiments, all support the Alliance's, and the Emperor and Vader captive;
  - for the Alliance's player: Luke and Mon Mothma captive.
- `prepare-site.mjs` stages MDATA.105..108.

**`ending.mjs`** (new, Linux Chromium, both sides) waits for each movie, skips the story movies with Escape, lets the Alliance's defeat movie run to its natural end and skips the Empire's, and then requires:
- the Empire's 103 before 107, as A0;
- the defeat movie of the player's side, 106 or 107;
- the main menu after it, with no other movie.

Run `.artifacts/interface-parity/ending-2026-11-07T09-03-59-669Z` (the b118 batch, the Alliance's 106 to its natural end): pass. The Alliance saw 102, 106; the Empire saw 102, 103, 107. The fixtures' galaxies hold no Alliance headquarters, so the word's HQ bit also queues 102.

**`story-movie.mjs`.** The StoryMovie scenario also queues 103 through `victory_conditions_movies(0x100)`. Its caption's ink is `[5, 407, 379, 419]` against A0's `[4, 407, 380, 419]`, within a pixel. Run `/home/retromax/Games/open-rebellion/.artifacts/interface-parity/story-movie-2026-11-07T08-16-05-589Z`: pass.

**Batches.** `story-movie` and `ending` now run in each batch after `tooltip`, so a batch has 23 gates.

**Not compared.** The port's main menu keeps its two added controls at the top right, the music toggle and the Fleet Registry; A0's has none.

## Both sides' wins too (b119)

The fixture scenario `EndingVictory` (98, code 99) gives the player's side the win. `ending.mjs` now runs four cases:

| Case | Movies played |
|---|---|
| Alliance defeat | 102, 106 |
| Empire defeat | 102, 103, 107 |
| Alliance victory | 102, 103, 105 |
| Empire victory | 102, 108 |

The defeat and victory movies are the four of `FUN_00425c90`, and the main menu follows each. Run `.artifacts/interface-parity/ending-2026-11-07T09-59-19-228Z` (the b119 batch): pass. No A0 capture of a win exists.

## The Alliance's defeat in A0, skipped (2026-11-07)

The unedited "test" save, reloaded, ran on Fast and auto-resolved its battles. The captures are in `.artifacts/a0-vm/alliance-defeat/` (hashes in `a0-sha256.txt`):
- **Day 1224, `00`.** The galaxy shows no Alliance system left.
- **`01`, `02`.** The Alliance's defeat movie, then its crawl "The Alliance is doomed. Under your command, the Rebel Alliance has been led to its destruction..." (MDATA.106, state 0x11).
- **`03`.** A click and Escape during the crawl ended the movie at once. The main menu followed ("Standard Game"), with no dialog.

This is the port's `ending_cutscene(false, true) == 106`, which `ending.mjs`'s Alliance defeat plays, and any key ends an ending movie as it ends any movie. Whether a story movie (102) played before 106 was not captured.
