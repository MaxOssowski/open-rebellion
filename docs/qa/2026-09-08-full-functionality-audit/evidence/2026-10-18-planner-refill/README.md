# AI mission planners refill their teams (F-036)

Date: 2026-10-18.

`ghidra/notes/ai-mission-planning.md`, "The refill".

## The original

`FUN_004bced0` runs states 3 and 4, the team and decoy selectors (planner
slots `+0x48` and `+0x4c`), on every cycle. A member dropped in state 2 is
replaced up to the class's team size and decoy cap.

## Port

- `Planner::refill` runs the selectors for the missing places and holds
  the new picks.
- `AISystem::mission_planner_cycle` holds the planner's own members, then
  refills after state 2.
- port: a team still empty after state 3 ends the planner.
- No save format change.

## Tests

- `ai.rs`: `a_mission_planner_refills_its_team_to_the_class_s_size`. It
  checks that a full team takes no one and that a dropped member is
  replaced by the free fighter. It fails without the refill
  (`fail-without.txt`).
- `a_mission_planner_gathers_its_team_before_the_order` now also checks
  that a second AI tick on the same day leaves the planner as it is.

## Campaigns (`playtest-planner-refill.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the research-reserve commit:

| | Before | After |
|---|---|---|
| Missions dispatched | 12,217 | 8,605 |
| Two-member teams | 2,145 (18%) | 3,233 (38%) |
| Successes | 12,030 | 10,757 |
| Failures | 8,826 | 7,543 |
| Systems at day 1500, summed (Alliance / Empire) | 579 / 352 | 557 / 347 |

## Gates

- `cargo test --workspace`: 2415 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the refill):
  - initial unchanged (`v1:5ba66653ae1fb76d`);
  - final `v1:2d45f0777ed8cd0d` -> `v1:f7fe1a11ab5f9ffd`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `b757a7e0075c433e5d2f06ef61a5f3083e6843a52cef034808b94745fc41720c`.
- The Fleet window gate passes all 48 cases (interface WASM
  `5bdd134e45201aa61ef98da98f3f90ed01e51e8013afab146ffda1bd9a0d3d5d`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.8 ms | 30.1 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.5 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - first run: 1 missed, the same-day return;
  - second run: 10 mutants, 7 caught, 3 unviable.
- `provenance-scan check`: 88 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
