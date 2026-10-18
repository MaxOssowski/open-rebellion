# The AI's Abduction planner (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "The Abduction planner".

## The original

- Row 9 of the strategy (`FUN_004e8290`) keeps one Abduction planner
  all game.
- Its decoys (`FUN_00483410`) are taken even without a team.
- The target (`FUN_00483640`):
  - keeps its character while that stands free at the target system;
  - otherwise lists the systems holding an opponent's main character,
    sector by sector;
  - takes the first one where the decoys gathered at `+0x38` fill the
    cap (4) or reach half the system's defense (record `+0xbc`).
- The character (`FUN_00483d10`) is drawn among the opponent's main
  characters there, on the planet or aboard an opponent's fleet.

## Port

- `abduction_target`, `abduction_character`, and row 9 in
  `mission_strategy`.
- Removed the heuristic Abduction, which took the weakest enemy
  characters first.
- No save format change.

## Tests

`ai.rs`:

- new:
  - `an_abduction_goes_where_the_gathered_decoys_match_half_the_defense`;
  - `an_abduction_takes_only_opponents_main_characters_on_the_planet_or_aboard_their_fleets`;
- removed, because it tested the heuristic:
  `abduction_takes_the_weakest_enemy_first_up_to_the_covert_cap`;
- the Diplomacy, Recon and Espionage tests now read their own kind,
  because an Abduction planner always joins. The Espionage test gives the
  Abduction planner four better decoys of its own.
- `fail-without.txt`: with the defense test removed, the first new test
  fails.

## Campaigns (`playtest-abduction-planner.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the report-slots commit:

| | Before | After |
|---|---|---|
| Abduction orders | 185 | 558 |
| Abduction successes / failures | 129 / 8 | 136 / 276 |
| Systems at day 1500, summed (Alliance / Empire) | 607 / 539 | 599 / 540 |

## Gates

- `cargo test --workspace`: 2429 passed, 0 failed.
- Seed-42 golden regenerated for this cause (the Abduction planner):
  - initial unchanged (`v1:530e248f2599b293`);
  - final `v1:75f7a5f65cee3222` -> `v1:03a36c82837d1ce1`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `714414f754eecd700e5c532d11b5bd1a8325b28774256fd76861c9ed2c06b5e3`.
- The Fleet window gate passes all 48 cases (interface WASM
  `d5977d678960bcd7fb0d5087e20d70524009454004aca4a092c2fb6a17a1a47f`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 24.7 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 4.4 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`):
  - first run: 52 mutants, 26 missed;
  - after the character test and the keep checks: 5 missed;
  - all 5 are equivalent. The fleet-candidate test repeats the
    condition that sets system bit `0x2000`. It differs only for an
    opponent's main character en route aboard an opponent's fleet, which
    the port cannot place.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
