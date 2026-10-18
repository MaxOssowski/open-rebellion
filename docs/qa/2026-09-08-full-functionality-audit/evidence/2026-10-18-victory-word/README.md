# The game is decided by the victory conditions word (F-005)

Date: 2026-10-18.

`ghidra/notes/victory-conditions.md`, "The check".

## The original

- `FUN_005180e0` builds the word daily. Its win bits depend on the game
  type (galaxy `+0x60`):
  - standard: the Empire wins with Luke and Mon Mothma captured and the
    Alliance HQ object destroyed; the Alliance wins with Vader and the
    Emperor captured and Coruscant held or destroyed;
  - headquarters: the HQ alone, or Coruscant alone.
- The game-over bit `0x10000000` is set with either win.

## Port

- `VictorySystem::check` reads `conditions_word`. With the game-over bit,
  the Alliance bit gives `HqCaptured`; otherwise the Empire wins by
  `DeathStarVictory` (HQ system destroyed) or `HqDestroyed`.
- Removed the port's own rules (`headquarters_objectives`,
  `standard_leaders_captured`) and their tests, which asserted the
  2026-09-10 contract.
- Where the binary differs from the manual ("On to Victory!") and that
  contract, the port follows the binary:
  - no Imperial assault is needed after the bombardment;
  - a destroyed Coruscant counts for the Alliance;
  - any captor counts.
  `agent_docs/systems/victory.md` and
  `docs/reference/campaign-history/official-campaign-contract.md` record
  the difference.
- No save format change.

## Tests

- `victory.rs`:
  - `the_headquarters_game_ends_on_coruscant_held_or_destroyed_or_the_hq_lost`;
  - `the_standard_game_also_needs_the_opposing_leaders_captured`;
  - `no_game_ends_without_its_four_characters_and_coruscant`.
- All three fail against the old check (`fail-without.txt`).

## Campaigns (`playtest-victory-word.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:

- the word changed 65 times; no game ended, as before;
- the AI captures single leaders (bits `0x10`, `0x20`, `0x1000`,
  `0x2000`) and loses them again; it never holds both.

## Gates

- `cargo test --workspace`: 2408 passed, 0 failed.
- Replay equivalence passes with the unchanged golden
  (`v1:5ba66653ae1fb76d` -> `v1:f7fe1a11ab5f9ffd`) and 0 browser errors.
  Release WASM SHA-256
  `e667ac4dfa010e99bb73693f8c3c784f3f337267a3a3ab52ac3c52a6777f3c40`.
- The Fleet window gate passes all 48 cases (interface WASM
  `6a3632a06a72431a8fc29d2df2b83938e9f7e250e2d342818ba88b71462ba98d`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.8 ms | 30.2 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`victory.rs`): 8 mutants, 7 caught,
  1 unviable.
- `provenance-scan check`: 88 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
