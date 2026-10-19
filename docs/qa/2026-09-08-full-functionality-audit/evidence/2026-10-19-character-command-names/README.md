# Character command: the name for each command (F-038, save v86)

Date: 2026-10-19. The user chose this option (save v86) for the name prefix
that stage 3 held back.

`ghidra/notes/character-command.md`. This is stage 3b of the tracker item
"Dowodzenie i jego utrata przy ranie".

## The original

- The CommandKind setter `FUN_004ee350` calls slot `+0x330`,
  `FUN_004f17f0`, with the old and new kinds.
- `FUN_004f17f0` renames the character (`FUN_004f6e60`) to TEXTSTRA of its
  name id (class record `+0x30`) plus:
  - `0x6800` for Commander;
  - `0x6c00` for Admiral;
  - `0x7000` for General.
- For example, 37440 "Commander Ackbar", 38464 "Admiral Ackbar" and 39488
  "General Ackbar" (`decoy-roll.md`). None renames the character back.

## Port

- `Character::command_names` (save v86) keeps the four names: the plain
  name, then Commander, Admiral and General. The loader reads them from
  TEXTSTRA for major and minor characters.
- `Character::set_command` sets the kind and the name.
  - `give`, `Character::injure` and `release_unfit` all set the command
    through it.
  - port: when TEXTSTRA has no name for the kind, the character keeps its
    name.

## Tests

- `character_command.rs`:
  `a_command_renames_the_character_and_its_loss_renames_it_back`. It fails
  without the rename (`fail-without.txt`).
- `tests/character_command_names.rs` (ignored; needs the game data):
  `a_character_loads_its_name_for_each_command` reads Ackbar's four names.
- `interface_test_fixture.rs`: scenario 68's agent is the first character
  whose class can be an admiral (Luke Skywalker). In fixture 44, Mon
  Mothma's class cannot command.

## Browser acceptance (`acceptance.json`)

Run with `agent-browser` on the interface build, fixture 68, both sides.
Each side right-clicked the agent and opened Command, then chose in turn:
1. Admiral;
2. Admiral again;
3. General.

- The log reads:
  - `character=Some("Admiral Skywalker") ... now=Some(Admiral)`;
  - `character=Some("Luke Skywalker") ... now=Some(None)`;
  - `character=Some("General Skywalker") ... now=Some(General)`.
- No console errors.
- Screenshots are not committed (original artwork):
  - `target/acceptance/character-command-2/{alliance,empire}-*.png`;
  - their SHA-256 are in `screenshots.sha256`.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all
24 runs are unchanged event for event against stage 2. The AI gives no
command yet (module `0xc`).

## Gates

- `cargo test --workspace`: 2462 passed, 0 failed.
- Seed-42 golden regenerated for this cause (save v86: the names are in the
  state):
  - initial `v1:4d8201e965d27089` -> `v1:094e77fcd3b4b666`;
  - final `v1:2229fcf258823750` -> `v1:beb63be8a01b2699`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `3ad33318ad9ba3ee3c55501261d0e4cae7de91a5b7ba02a732dfafef0aad4af1`.
- The Fleet window gate passes all 48 cases (interface WASM
  `b60935a24d8a37bf739add55fd52ad004ecbdb434775607bc393bd74c61a49dc`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 24.6 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.4 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`, `rebellion-data`):
  - 16 mutants, 13 caught by the unit tests;
  - the 3 loader mutants (`load_game_data_with_options`,
    `convert_character`) are caught by the ignored data test.
- `provenance-scan check`: 85 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
