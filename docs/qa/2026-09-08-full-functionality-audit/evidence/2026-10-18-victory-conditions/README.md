# The victory conditions word: Coruscant Captured and Alliance Headquarters Destroyed (F-038, save v79)

Date: 2026-10-18.

Story message rows 20 and 21 (`ghidra/notes/story-message-classes.md`)
follow the victory conditions word, read in
`ghidra/notes/victory-conditions.md`. The user approved the save change
to v79 for this stage.

## The original

- `FUN_005180e0` builds galaxy `+0x74` from the following, with the wins
  by game type (`+0x60`):
  - the Alliance HQ (`0x1`);
  - the captures of Luke, Mon Mothma, Vader and the Emperor (`0x10`,
    `0x20`, `0x1000`, `0x2000`);
  - Coruscant held by the Alliance or destroyed (`0x100`, `0x200`).
- Both sides copy the word to `+0xbc` (`FUN_00518460`, `FUN_0052f600`).
  Each change posts `0x128` with the old and new words (`FUN_00532f90`).
- Row 21, Alliance Headquarters Destroyed (`FUN_0048e5a0`): the new word
  has bit `0x1`.
- Row 20, Coruscant Captured (`FUN_0048f790`): Coruscant is the
  Alliance's.
- Both go to both sides and have no once flag.

## Port

- `victory.rs`:
  - the `WORD_*` bits;
  - `conditions_word`, which includes the game-type wins and the rule
    for a day when both would win;
  - `advance_conditions_word`, which reports `(old, new)`.
- `VictoryState::conditions_word` holds the word (save v79, serde
  default).
- Simulation step 14 and the app run it daily before the port's own
  victory check. The telemetry records `victory_conditions` with the old
  and new words.
- `story_messages::victory_conditions_messages`. The app posts it on a
  change, and the fixture posts it for the word `0x101`.
- port:
  - the headquarters is the Alliance HQ system, lost when it is destroyed
    or no longer holds the HQ;
  - the check runs daily.
- Not ported: deciding the game by the word's win bits. The port's
  `VictorySystem::check` still decides it (F-005).
- 2 of the 21 story message classes remain unmapped: 13 (Rescue
  Attempt) and 17 (Discovers Traitor).

## Tests

- `victory.rs`:
  - `the_victory_conditions_word_carries_its_bits`;
  - `the_word_s_wins_follow_the_game_type`;
  - `when_both_would_win_the_recorded_win_stands`;
  - `a_change_of_the_word_is_reported_once`.
- `story_messages.rs`:
  `a_victory_conditions_change_posts_the_hq_and_coruscant_messages`.
- Shown failing without the change (`fail-without.txt`): with the tie
  rule, the headquarters game type and the change report reverted, 3
  tests fail.

## Campaigns (`playtest-victory-conditions.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days.

- The word changed 292 times, from the four victory characters' captures
  and releases. Mon Mothma's capture is the most common change.
- No campaign lost the Alliance HQ or Coruscant (bits `0x1`, `0x100`,
  `0x200`), so neither row 20 nor row 21 posted in play. They are shown
  by the tests and the fixture.

## Browser

- The story-message fixture (Alliance, code 312), checked with
  `agent-browser`:
  - "Alliance Headquarters Destroyed" / "The Empire has destroyed the
    Alliance Headquarters.";
  - "Coruscant Captured" / "The Alliance has seized the Imperial capital
    system of Coruscant.";
  - neither has a picture, as the builders set no background.
- Screenshots not committed (original artwork):
  - `eb6e9c8f778a6846d8ddefff8642448a121ff26dfd34d74ff35332bf1282a96e
    target/acceptance/story-messages-2/hq-destroyed.png`;
  - `d49fdde4e3988f5d0faeff617de01b26729a1e0fab58b78ed8cc57596e6b45af
    target/acceptance/story-messages-2/coruscant-captured.png`.
- The Fleet window gate passes all 48 cases (interface WASM
  `01657b1c757824cff28b08b993456fb44b70dfdecf144ab722d6cb9d7b588667`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 25.9 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.5 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.

## Gates

- `cargo test --workspace`: 2406 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v79, the word):
  - initial `v1:d8f4a9b46e07169c` -> `v1:e8ceaa386f41cc96`;
  - final `v1:fd4bcf87c87325d6` -> `v1:475c0c0f35968fba`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `90137d9f0f8cbd188cc7b9c978f6839dc6c36b03d2a4c27add9763d822fc2479`.
- `cargo mutants --in-diff` (`victory.rs`): 30 mutants, 29 caught, 1
  missed. The miss, `|` to `^` in folding the bits, is equivalent: the
  bits are distinct.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) on the changed lines: clean, apart from the story
  builders' shared `implicit_hasher` and two similar binding names in the
  tests.

Hashes of the committed files are in `SHA256SUMS`.
