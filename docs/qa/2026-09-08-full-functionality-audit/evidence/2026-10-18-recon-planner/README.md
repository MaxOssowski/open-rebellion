# The AI's Reconnaissance planner (F-036, save v82)

Date: 2026-10-18. The user's grant of 2026-10-18 covers the save format
change for work until 06:00 on 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "The Reconnaissance planner".

## The original

- Row 11 of the strategy (`FUN_004e8290`) keeps one Reconnaissance
  planner per module 8 sector planner (controller `+0xd8`).
- The team (`FUN_0047f790`) is one unassigned member able for Recon.
- The target (`FUN_0047f970`) is a system not the side's:
  - its kept target;
  - else, in a sector near the last gathering place or drawn, a random
    unexplored one;
  - else, when every sector holds a headquarters or Coruscant, the one
    seen longest ago.
- The planner claims the sector (record `+0x28` `0x10000000`), and the
  refresh keeps the claim.

## Port

- `recon_target`, `release_sector`, and row 11 in `mission_strategy`.
- `MissionPlanner::sector`, `AIState::sector_claims`,
  `ai_records::apply_sector_claims` and `update_sector_state` (save v82).
- The Recon selector in `mission_planning`. It was not ported, so the AI
  sent no Reconnaissance.
- Removed the heuristic "reconnaissance". It sent Espionage missions to
  explored enemy systems.

## Tests

`ai.rs`:

- new:
  - `the_strategy_keeps_a_recon_planner_per_sector_planner`;
  - `a_recon_planner_claims_its_sector_and_a_second_finds_none_there`
    (also checks the order);
  - `a_recon_target_that_becomes_the_side_s_is_dropped_and_its_sector_freed`;
  - `while_a_sector_lacks_a_capital_recon_takes_only_unexplored_systems`;
  - `with_every_sector_holding_a_capital_recon_takes_the_system_seen_longest_ago`;
  - `the_refresh_keeps_the_sector_claims_and_a_claim_is_seen_at_once`;
- removed, because they tested the removed heuristic:
  - `reconnaissance_targets_explored_enemy_systems`;
  - `reconnaissance_scouts_only_explored_systems_the_enemy_holds`;
  - `reconnaissance_scouts_the_enemy_s_strongest_systems_first`.
- `fail-without.txt`: without the sector claim, and with at most one
  planner, 2 tests fail.

## Campaigns (`playtest-recon-planner.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the Subdue commit:

| | Before | After |
|---|---|---|
| Reconnaissance orders (successes) | 0 | 3,006 (2,864) |
| Espionage orders | 1,194 | 1,296 |
| Espionage successes / failures | 2,148 / 840 | 2,460 / 602 |
| Systems at day 1500, summed (Alliance / Empire) | 376 / 462 | 483 / 476 |

## Gates

- `cargo test --workspace`: 2423 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v82, the
  Reconnaissance planner):
  - initial `v1:6f750b483b72f71f` -> `v1:cb9430c7635d6e7e`;
  - final `v1:7ed462ebd95b759a` -> `v1:3c9e4e1739d78f43`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `f9a66361519684af0a81e57263a55c93ef7bd4eb16231337af102cc9589f2cda`.
- The Fleet window gate passes all 48 cases (interface WASM
  `4761fe55c8b76f970cb99baa1d41ad2ec93b1189d524bd66693ff214d437ee92`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.8 ms | 26.2 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.5 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`, `ai_records.rs`,
  `mission_planning.rs`):
  - first run: 34 mutants, 11 missed (the sector claims and the
    capital branch);
  - second run: 2 missed;
  - both were closed by
    `while_a_sector_lacks_a_capital_recon_takes_only_unexplored_systems`
    (verified by hand).
- `provenance-scan check`: 87 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
