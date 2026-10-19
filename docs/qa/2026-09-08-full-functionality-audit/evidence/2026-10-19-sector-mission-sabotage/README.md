# Sector planners' Sabotage targets, stage 4 (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "The Sabotage targets".

## The original

- For Sabotage (`0x12`), state 2 (`FUN_004bdbc0` -> `FUN_00476e80`) asks
  the plan object's slot `+0x30`.
- Each kind tries its own order of opponent asset kinds until one gives
  a system of the plan's sector. It uses `FUN_0041a340` on `v[29]`,
  order 2:
  - kind 1 (`FUN_004bab30`): the headquarters first for the Empire, then
    yards before regiments;
  - kind 2 (`FUN_004b99b0`): yards last;
  - kind 3 (`FUN_004b3f80`): defenses, then regiments before yards;
  - kinds 4 and 5 (`FUN_004b7b10`, `FUN_004b1f80`): no yards;
  - kind 6 (`FUN_004b5540`): the Empire's headquarters second.
- Kind 4's copy of its target `+0x5c` is always overwritten by its first
  query (`0x4b7c00`).
- `FUN_004ae480` then lists the active objects there of that step's kind
  and draws one (`FUN_0041cd80(count)`):
  - defenses `0x24`, `0x25`, `0x23` and `0x22`;
  - regiments;
  - the ships of fleets not the side's;
  - training facilities, shipyards, and construction yards unless
    controller `+4` has `0x40`;
  - refineries and mines.

## Port

- `sector_sabotage_target` and `assets_at`, called by state 2. The planner
  keeps the system and the object.
- port: the port has no headquarters object and no mission target for a
  squadron on a planet, so those steps give no target. Each kind lists in
  the world's order.
- port: states 3 to 8 are still not ported, so the held planners order
  nothing. The heuristic Sabotage stays.
- No save format change.

## Tests

- `ai.rs`:
  - `sabotage_lists_the_assets_of_its_step_s_kind_in_the_original_s_order`;
  - `each_plan_kind_sabotages_by_its_own_order_of_asset_kinds`;
  - `a_sector_planner_s_sabotage_planner_takes_a_system_and_an_object`.
    It includes controller `+4` `0x40`.
- All three fail without the change (`fail-without.txt`).

## Campaigns (`playtest-sector-sabotage.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against stage 3.

- All 24 diverge, only through the draws.
- With the picks made but their targets dropped, seeds 1, 2, 14 and 23 ran
  event for event as with them kept.

| | Before | After |
|---|---|---|
| Missions dispatched | 12,764 | 12,278 |
| Sabotage dispatched | 2,609 | 2,542 |
| Successes | 18,404 | 18,376 |
| Failures | 7,195 | 6,820 |
| Systems at day 1500, summed (Alliance / Empire) | 584 / 567 | 572 / 569 |

## Gates

- `cargo test --workspace`: 2452 passed, 0 failed.
- Seed-42 golden regenerated for this cause (the Sabotage targets enter
  the state):
  - the first change is at tick 15;
  - initial unchanged (`v1:20ee82a2951ad9aa`);
  - final `v1:aae0de7bbef402c1` -> `v1:dee3c02775ecfb47`;
  - two recordings are identical, and only the fingerprints changed;
  - `replay_manifest --include-ignored` passes;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `736b24e83adee4cd67415fead25f5a82d96d0f743322b8cec46cb426d091c176`.
- The Fleet window gate passes all 48 cases (interface WASM
  `f50e7eee13cb5b88fcca76f9a88f5e00aab500cdc507d7174b9bd098648a773c`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.0 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.4 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`):
  - 40 mutants: 35 caught, 2 unviable, 3 missed;
  - the 3 missed were all in the controller-bit read;
  - the construction-yard case of the wiring test now catches them,
    checked by hand.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
