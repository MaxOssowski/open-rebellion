# Sector planners' Recon and Espionage targets, stage 2 (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "The Recon and Espionage
targets".

## The original

- State 2 of a held mission planner's cycle (`FUN_004bda60` ->
  `FUN_004bdbc0`) asks the owning sector planner `+0x70` for its
  target, through `FUN_00476e80` and the plan object's slots:
  - Recon `0x15` -> `+0x2c`;
  - Espionage `0x13` -> `+0x28`;
  - Sabotage `0x12` -> `+0x30`;
  - Abduction and Assassination `0x17`, `0x81` -> `+0x24`.
- Recon (all six kinds) takes the sector's unexplored system not the
  side's seen longest ago (`FUN_0041a3a0`, `+0x2c` `0x80` forbidding
  `0x8000000`, order 1), else any such system.
- Espionage takes the opponent's system seen longest ago (`+0x2c`
  `0x200`, `+0x30` 4). Kinds 3 to 6 then fall back to a random system
  with `0x200` (`FUN_0041a460`).
- Without a target, state 9 (`FUN_004be4c0`) ends the cycle.

## Port

- `sector_mission_target`, and state 2 in
  `AISystem::mission_planner_cycle`, once a day per held planner.
- port:
  - the Sabotage and Abduction/Assassination slots give no target yet;
  - states 3 to 8 are not ported, so the held planners still order
    nothing.
- No save format change.

## Tests

- `ai.rs`:
  - `a_sector_planner_s_plan_gives_recon_an_unexplored_system_and_espionage_the_oldest_seen`;
  - `a_sector_planner_s_mission_planner_takes_its_target_once_a_day`.
- Both fail without the change (`fail-without.txt`).

## Campaigns (`playtest-sector-targets.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the stage-1 commit.

- All 24 diverge. The picks draw from the planner's random numbers,
  which shifts the draws that follow, as the original's picks do.
- The stored targets themselves change nothing. With the picks made but
  their targets dropped, seeds 1, 2, 14 and 23 ran event for event as
  with them kept.

| | Before | After |
|---|---|---|
| Missions dispatched | 12,256 | 13,470 |
| Two-member teams | 3,568 | 4,471 |
| Successes | 18,522 | 18,609 |
| Failures | 7,170 | 7,392 |
| Systems at day 1500, summed (Alliance / Empire) | 589 / 564 | 567 / 569 |

## Gates

- `cargo test --workspace`: 2445 passed, 0 failed.
- `cargo test -p rebellion-data --test replay_manifest --
  --include-ignored`: 2 passed.
- Seed-42 golden regenerated for this cause (the held planners'
  targets):
  - initial unchanged (`v1:20ee82a2951ad9aa`);
  - final `v1:14ec5ed9881bd79b` -> `v1:aae0de7bbef402c1`;
  - the first change is at tick 10;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `902b1909a717fdd3895e491c69a730978a8b295046f3788501302eee5b8dd98d`.
- The Fleet window gate passes all 48 cases (interface WASM
  `6e2fb3a00505f122b8c81ec1ecd65470138802e45a61af0f9397b8cff7b8c66b`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 24.3 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.7 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`): 8 mutants, 8 caught.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
