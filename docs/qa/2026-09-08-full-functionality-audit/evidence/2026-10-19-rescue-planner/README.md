# The AI's Rescue planner (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "The Rescue planner".

## The original

- Row 10 of the strategy (`FUN_004e8290`) keeps one Rescue planner while
  a character of the side is a prisoner (controller bit `0x200`,
  `FUN_0041bc10`).
- The target (`FUN_0047e620`):
  - is chosen anew each cycle. Its keep test `(flags | 0x20) == 0`, at
    `0x47e800`, never holds;
  - is the first prison where the gathered decoys reach half the
    defense or fill the cap.
- A prisoner of the side there is drawn on the planet or aboard an
  opponent's fleet (`FUN_0047ecf0`).

## Port

- `rescue_target`, `rescue_character`, and row 10 in `mission_strategy`.
- Removed the heuristic Rescue (one per captive), the last group of
  mission heuristics.
- No save format change.

## Tests

`ai.rs`:

- new:
  - `the_strategy_keeps_a_rescue_planner_while_a_character_of_the_side_is_a_prisoner`
    (also checks the order);
  - `a_rescue_target_is_chosen_anew_where_the_decoys_cover_half_the_defense`;
  - `a_rescue_counts_the_decoys_where_it_gathers_and_reaches_prisoners_aboard_enemy_fleets`;
- removed, because it tested the heuristic:
  `each_captive_ally_gets_a_rescue_at_its_prison`;
- `only_standing_diplomacy_planners_count_against_the_want_and_each_has_its_own_id`
  now uses a Death Star Sabotage planner as the other standing kind. Row
  8 is not ported, and the Rescue row now manages Rescue planners.
- `fail-without.txt`: with the want forced to one and no defense test,
  4 tests fail.

## Campaigns (`playtest-rescue-planner.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the prisoner-side fix:

| | Before | After |
|---|---|---|
| Rescue orders | 429 | 327 |
| Rescue successes / failures | 193 / 6 | 278 / 21 |
| Systems at day 1500, summed (Alliance / Empire) | 607 / 536 | 570 / 579 |

## Gates

- `cargo test --workspace`: 2432 passed, 0 failed.
- Seed-42 golden regenerated for this cause (the Rescue planner):
  - initial unchanged (`v1:530e248f2599b293`);
  - final `v1:03a36c82837d1ce1` -> `v1:49ba43802d0d2840`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `b9857e0a0ec526cd78203671fe55f2cebe03248b4c73bd6b8ba3dfe2492a3062`.
- The Fleet window gate passes all 48 cases (interface WASM
  `4c7beea0d2cad1af0bc1090fba0585d4cb2801ccbd93e427551bda50659ac7b4`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.1 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.3 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`):
  - first run: 49 mutants, 20 missed;
  - after the decoy and fleet test: 7 missed;
  - all 7 are equivalent. The system-bit gates and the fleet carry test
    repeat the conditions that set those bits.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
