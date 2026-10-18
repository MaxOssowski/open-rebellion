# The assassination injures, and kills only a minor character (F-038)

Date: 2026-10-18.

The port's Assassination always killed its target. The original injures
it, and the injury kills only a minor character. Reading Jabba's palace
led here: the AI's assassins had been killing Han before the bounty
hunters ever came. Details are in `ghidra/notes/character-injury.md`
("Assassination and death by injury").

## The original

- `FUN_00576620`, the Assassination hit: each member that wins its chance
  calls the target's slot `+0x2ec` (`FUN_004ef690`):
  - `FUN_0053eae0` rolls the injury: with a chance of GNPRTB 2574 (100),
    rand(0..=2574) + rand(0..=2576 (29)) + 2575 (1);
  - slot `+0x2f0` (`FUN_004ef6d0`, reason 7) adds it, then asks slot
    `+0x2e0` whether it kills.
- Slot `+0x2e0`:
  - major characters (vtable `0x006657f0`, family `0x30`):
    `FUN_006158b0`, never;
  - minor characters (vtable `0x00665be0`, family `0x38`): `FUN_0056f160`
    -> `FUN_0053eb30`, a GNPRTB 2577 (25) roll.
- The result 3 and the combat raise (GNPRTB 6164) need the target
  destroyed (`+0x50` bit 3).
- The manual's mission table (p. 105) agrees: "You cannot kill primary
  characters such as Luke Skywalker, but injuring them makes them easier
  to capture."

## Port

- `mission_detection::assassination_injury` (`FUN_0053eae0`) and
  `dies_of_injury` (slot `+0x2e0`; a major character draws nothing).
- `missions.rs`, the Assassination arm:
  - each successful member injures the target until it is destroyed;
  - a minor character's death is `CharacterKilled`;
  - the result 3 and the raise only follow a destroyed target.
- No save format change. The seed-42 golden is unchanged: no
  assassination resolves in its 25 days.
- Not ported yet: the same death roll after the other injuries (the
  detection's evasion and capture injuries, the Force duel, the uprising
  incident).

## Tests

- `missions.rs`:
  - `an_assassination_only_injures_a_major_character`: two injuries of
    1..=130, no kill, no raise, a failure;
  - `an_assassination_kills_a_minor_character_its_injury_kills` (GNPRTB
    2577 set to 100): one injury, one kill, both members raised, a
    success.
  - These replace
    `an_assassination_kills_its_target_once_and_raises_every_successful_member`,
    whose always-kill expectation was the port's own, not the original's.
- `mission_detection.rs`:
  `the_assassination_injury_and_a_minor_character_s_death_follow_their_rolls`.
- Shown failing without the change (`fail-without.txt`): with the death
  roll replaced by an always-kill, the major-character test fails.

## Campaigns (`playtest-assassination.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days. The
"before" runs are the previous stage's (`plc1`).

| | Before | After |
|---|---|---|
| Assassinations resolved | 488 (66 successes) | 1039 (0 successes) |
| Killed: Han, Mon Mothma, Leia, Luke | 22, 21, 18, 5 | 0 |
| Bounty Hunters' strikes (captures) | 16 (0) | 24 (2) |
| Palace rescues | 0 | 2 |

- The AI only targets major characters, so no assassination succeeds.
- Han now lives long enough for the hunters in every seed:
  - seed 16: captured day 399, in the palace day 400, rescued by
    Chewbacca day 412;
  - seed 19: captured day 571, in the palace day 572, rescued by Luke day
    584.

  This is the first time Jabba's palace story ran in a campaign.

## Gates

- `cargo test --workspace`: 2395 passed, 0 failed.
- Replay equivalence passes with the unchanged golden
  (`v1:5474ae69134d1255` -> `v1:4cd75bad1e5062b8`) and 0 browser errors.
  Release WASM SHA-256
  `a8ff7a9a2b3f802e6814243242c52637d846e9e98ed730437b26c548f2216949`.
- The Fleet window gate passes all 48 cases (interface WASM
  `912d7c72f10d29f9f10d6022a101245fdc468c20886cb1c4291d103e1058ac51`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 25.0 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 4.1 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`missions.rs`, `mission_detection.rs`): 15
  mutants, 14 caught, 1 unviable.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt on the changed lines: clean.
- Native GUI acceptance was not run for this change.

Hashes of the committed files are in `SHA256SUMS`.
