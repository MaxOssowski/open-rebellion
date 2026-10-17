# The stand-in fleet heuristics removed (F-036)

Date: 2026-10-17.

The user decided to remove the port's fleet heuristics. Until stage 6d
they stood in for the sector planners' plan objects. Since 6d every kind
that `FUN_00477630` makes (1 to 6) has its own plan object, so the
heuristics ran only for a kind outside 1 to 6, which never occurs. Note:
`ghidra/notes/ai-sector-plans.md`, "Removed (2026-10-17, save v58)".

## Removed

- `ai.rs`:
  - the fleet heuristics `evaluate_fleet_deployment`, `evaluate_ds_escort`
    and `select_ds_target`;
  - their helpers `can_dispatch_to_system`, `can_dispatch_fleet`,
    `fleet_strength`, `evaluate_galaxy_state` and `score_attack_target`,
    and `GalaxyState`;
  - module 8's branch of `run_module` and the scheduler's `Step8::Plan`
    path;
  - `AIState::battle_cooldowns` and `AISystem::record_battle`. Only the
    attack scoring read them.
- `ai_module8.rs`: `Step8::Plan` and `Module8::decided`.
- `ai_planner.rs`: `PlannerStep::StandIn` and `plan_done`. A planner whose
  kind has no plan object ends its cycle, as kind 0 does.
- `rebellion-data/src/simulation.rs`: the calls to `record_battle` after a
  battle.
- Kept: `AiConfig`'s `alliance_deploy_budget`, `empire_deploy_budget` and
  `ds_retreat_strength_ratio`, now unused. Removing them would change the
  config file format.
- `agent_docs/systems/ai-parity-tracker.md` marks the removed rows.

Save format: v58, since `AIState` and `Module8` lost fields. There is no
migration, by the project's rule (`agent_docs/save-load.md`).

## Tests that changed

- Removed, because they tested the removed heuristics: twelve tests.
  - Five tested target choice and dispatch: `fleet_directed_to_attack_weak_enemy_system`,
    `fleet_attacks_enemy_force_orbiting_neutral_system`,
    `outmatched_fleet_raids_weak_territory_instead_of_suicidal_interception`,
    `stationed_hq_defender_is_not_dispatched` and
    `assembled_hq_wave_can_launch_and_leave_future_production_to_defend`.
  - One tested how the heuristics treated held fleets:
    `the_fleet_heuristics_leave_a_fleet_a_sector_planner_holds_to_its_plan`.
  - Four tested the dispatch helpers: `can_dispatch_to_system_*` (three)
    and `can_dispatch_fleet_rejects_no_alive_ships`.
  - Two tested the Death Star: `death_star_targets_highest_value_when_no_hq`
    and `death_star_retreats_when_outgunned`.
- Scheduler tests:
  - `a_pass_issues_one_order...`, `a_module_with_orders_left...`,
    `equal_priorities...` and `the_day_s_tenth_order...` used module 8
    because the stand-in queued its orders. They now queue orders in module
    `0xc`'s list, which still works that way. Each checks the same
    scheduler rule as before.
  - `a_refused_module_eight_order_ends_its_cycle_and_its_orders` is now
    `a_refused_module_eight_order_ends_its_cycle`. A kind 3 plan's state
    `0xd` moves its fleet to a destroyed system, the validation refuses
    the move, and module 8's cycle ends. Module 8 no longer has queued
    orders to drop.
- `a_planner_of_another_kind_with_nothing_to_order_ends_its_cycle` is now
  `a_planner_without_a_plan_object_ends_its_cycle`. The planner's state
  stays 0, where the stand-in used to set it to 1.
- `ai_planner.rs`, two cases: a kind 0 or kind 7 planner returns `Done`
  where kind 7 used to return `StandIn`.
- `ai_module8.rs`: the assertion on `decided` went with the field.

## Campaigns

`rebellion-playtest data/base --ticks 600 --dual-ai` for seeds 1, 2, 3, 7,
13, 42 and 47, compared with the previous build (`ea2a0baa`): every event
stream is identical apart from timing (`playtest-identical.txt`).

## Browser

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast. Each
frame's work time was measured:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3599 | 0.6 ms | 0.8 ms | 24.5 ms |
| Fast | 60 s | 3601 | 0.6 ms | 1.5 ms | 3.2 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The Fleet window gate passes all 48 cases.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2246 passed, 0 failed.
- Neutralized: module 8's refused-order handling, and the planner's end
  without a plan object. Both fail their tests.
- `cargo test -p rebellion-data --tests -- --include-ignored`: the
  goldens pass. Two tests fail, as they did before:
  `fixed_fleet_tables_only_target_special_systems` and
  `telemetry_coverage`.
- Seed-42 golden regenerated for save v58 and the removal:
  `v1:a8343f6c6630924e` -> `v1:51f96ff9faae705b`, final
  `v1:e523a780152b284e` -> `v1:d2e439b6e18aff1f`. Two recordings are
  identical, and only the fingerprints changed in the manifest. The replay
  equivalence check passes with 0 browser errors. WASM SHA-256
  `0631e5fa3314ca71342c3ae624ef23eb63db2594b4e7297f79f2b9fba619b71a`.
- `cargo mutants --in-diff` (core and data): 24 mutants, 19 caught, 5
  unviable.
- `provenance-scan check`: 101 uncited items, one fewer than the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
