# The sector classes' selectors and the defense decoy cap, stage 5 (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "The sector classes' selectors"
and "Decoy selectors".

## The original

- `FUN_0042f830` builds a planner's class from its kind and subtype.
- Espionage of a subtype other than 6 (a sector planner's request):
  - team: `FUN_004808c0`, the strategy's `FUN_00480f30` query, for two
    members;
  - decoys: `FUN_00480aa0`, by the same query.
- `0x62` of subtype 4, and `0x63`: class `0x65a9b0` (`FUN_00482b30`).
  - For the Alliance this is Abduction with `+0x7c` 0x10; for the Empire,
    Assassination with 0x40.
  - Selectors: `FUN_00482ce0` and `FUN_00482ec0`.
- Reconnaissance: `FUN_0047f320` matches `FUN_0047f790` word for word.
- The decoy cap of Incite (`FUN_00480110`), Sabotage (`FUN_0047d720`),
  sector Espionage (`FUN_00480aa0`) and class `0x65a9b0` (`FUN_00482ec0`):
  - it is `(rec.+0xbc + 2) / 2` of the target `+0x34`'s record, and 0
    with no record;
  - `+0xbc` is `v[29]`, the system's defense (`ai-galaxy-records.md`,
    values from `+0x48`).

## Port

- `mission_planning::selectors(kind, subtype)`, `defense_cap`, and
  `Planner::refill` and `Planner::plan` with the target's `v[29]`.
- `ai::strategy_subtype`: the strategy's planners take subtype 5 (Recon)
  and 6 (Espionage) from the kind (`FUN_004e8290` rows 11 and 12). Only a
  sector planner keeps its own subtype (save v84). No save format change.
- **Correction.** The port read `+0xbc` as the mission kind a posture
  recommends, and took one decoy (a `hyp:`). The same reading stood in
  the Ghidra note, which is corrected. `abduction_target` already read
  `+0xbc` as `v[29]`.
- The held sector planners still do not run states 3 to 8. Only the
  strategy's and the heuristics' planners use the new cap so far.

## Tests

- `mission_planning.rs`:
  - `a_defense_capped_selector_takes_half_the_target_s_defense_plus_one`;
  - `a_sector_planner_s_espionage_takes_two_and_decoys_by_the_same_query`;
  - `a_sector_planner_s_abduction_caps_its_decoys_by_the_target_s_defense`.
- `ai.rs`: `the_strategy_s_espionage_planner_keeps_its_one_member_class`.
- The existing `plan` tests now pass `v[29] = 0`, which gives the one
  decoy they asserted, `(0 + 2) / 2`.
- `decoy_cap` now reads the selectors only; its values are unchanged.
- `fail-without.txt`: with the subtypes ignored and the cap fixed at 1, 5
  tests fail.

## Campaigns (`playtest-sector-selectors.json`, `decoys.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against stage 4.

| Decoys per mission | Before | After |
|---|---|---|
| Incite | 0.52 (88 / 169) | 2.41 (318 / 132) |
| Sabotage | 0.65 (1,657 / 2,542) | 1.66 (3,773 / 2,268) |
| Assassination | 0.85 (267 / 314) | 1.90 (605 / 319) |
| Abduction (fixed cap 4) | 3.18 | 3.24 |

| | Before | After |
|---|---|---|
| Missions dispatched | 12,278 | 11,976 |
| Successes | 18,376 | 18,199 |
| Failures | 6,820 | 7,283 |
| Systems at day 1500, summed (Alliance / Empire) | 572 / 569 | 605 / 544 |

## Gates

- `cargo test --workspace`: 2456 passed, 0 failed.
- `replay_manifest --include-ignored`: 2 passed. The seed-42 golden is
  unchanged (`v1:20ee82a2951ad9aa` -> `v1:dee3c02775ecfb47`).
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `739062057fc304515e9095fe2262301b61395e5fe04a282b1e1bbeb29e8f03e8`).
- The Fleet window gate passes all 48 cases (interface WASM
  `109ac7224e40297f0ee404e0b1614cc7ba8f085af61df61610d7d456966aec4b`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 24.5 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 5.0 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - first run: 41 mutants, 5 missed;
  - second run: 40 mutants, 31 caught, 8 unviable, 1 missed;
  - the missed one deletes `strategy_subtype`'s Recon arm, which is
    equivalent: both Recon classes select alike.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
