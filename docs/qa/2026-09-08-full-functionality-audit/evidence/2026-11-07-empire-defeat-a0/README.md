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
