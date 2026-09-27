# AI System Invention Audit

## What drives the original AI

The original Rebellion AI is event-driven and distributed, not polled. There is no daily AI tick. `FUN_00508250` (the 18-validator chain) runs once at setup stage `0x17` of `FUN_005136d0`, not per-tick. Support recalculation (`FUN_0050b230`) fires from field hooks (`FUN_00510820`, `FUN_005109f0`) when inputs change. The 6-function pipeline (`FUN_00519d00` galaxy eval, `FUN_00537180`/`FUN_005385f0` system iteration, `FUN_00508660` entity dispatch) triggers through the CNotifyObject observer system. The port replaces this with `AISystem::advance()` polling every `tick_interval` (default 7) days. Both `AI_TICK_INTERVAL = 5` and `AiConfig::tick_interval = 7` are invented; the `0x1f0` message cited as the AI trigger is a mission-UI notification (`FUN_00525040`), confirmed in `timer-scheduler.md`.

## Findings

Already ledgered as F-031 (reference only): `queue_len >= 3`, `150*150` troop radius, `ESPIONAGE_SKILL_THRESHOLD 50`.

| Original fn + .c line | Port file:line | Verdict | Difference | Test name |
|---|---|---|---|---|
| no counterpart; AI is event-driven | ai.rs:50 `AI_TICK_INTERVAL = 5` | invented | Original has no AI poll interval; 5 is arbitrary | `ai_tick_interval_matches_original_ai_cadence` |
| no counterpart | tuning.rs:104 `tick_interval: 7` | invented | AiConfig default 7 likewise has no source; tagged "original unknown" but used as parity | `ai_config_tick_interval_has_original_source` |
| `FUN_0053e190.c:5` -> `FUN_0053e170(a,b,DAT_00661a88)` -> `FUN_0053e150(b*a, DAT_00661a88)` | ai.rs:1655 `(control_ratio * 0.8 + 0.1).clamp(0.1, 0.9)` | wrong | Original is scaled integer division by `DAT_00661a88` (system count); port invents a linear formula with magic constants 0.8, 0.1, 0.9 | `aggression_curve_matches_fun_0053e190_ratio` |
| `FUN_00506ea0.c:7` returns `*(DAT_006b2bb0 + 0xc4)` (Alliance) / `+0xc8` (Empire) | tuning.rs:119-120 `alliance_deploy_budget: 0.6`, `empire_deploy_budget: 0.8` | invented | Values at `+0xc4`/`+0xc8` are not recovered from the binary; 0.6/0.8 are guesses | `deploy_budgets_match_fun_00506ea0_offsets` |
| `FUN_0050ac80.c` routes by faction bits `+0x24 & 0xc0`; no skill threshold | ai.rs:54 `DIPLOMACY_SKILL_THRESHOLD = 60` | invented | Cited as rebellion2, not REBEXE; original character placement has no diplomacy score gate | `diplomacy_threshold_has_rebexe_source` |
| no counterpart | ai.rs:57 `MAX_CONSTRUCTION_YARDS = 5` | port-owned | Original production routing (`FUN_00508660` family handlers) has no yard cap; port needs one for its heuristic loop | `max_yards_matches_original_production_cap` |
| no counterpart | ai.rs:61 `DIPLOMACY_TARGET_POPULARITY_CAP = 0.8` | port-owned | Original doesn't score systems by popularity for mission dispatch | `diplomacy_pop_cap_matches_original_target_rule` |
| no counterpart | ai.rs:70 `COVERT_MIN_SUCCESS_PROB = 0.30` | port-owned | Original has no pre-dispatch probability gate; the port adds one to avoid wasting operatives | `covert_prob_gate_matches_original_dispatch_rule` |
| no counterpart | ai.rs:74 `MAX_COVERT_OPS_PER_EVAL = 3` | port-owned | Replaces original entity budget tracking (`+0x58`, `+0x5c`, `+0x64`); structurally different | `covert_ops_cap_matches_original_budget_system` |
| no counterpart | ai.rs:577 `jedi_probability > 50` | invented | Original AI has no Jedi-skip gating; entirely port-added behavior | `jedi_skip_threshold_has_original_source` |
| no counterpart | ai.rs:628 `diplomacy_score > 30` | invented | Fallback diplomacy threshold for majors; no original counterpart | `major_diplomacy_fallback_matches_original_rule` |
| no counterpart | ai.rs:686 `enemy_pop > 0.5` | invented | Incite uprising target threshold; original incite logic not recovered | `incite_target_threshold_matches_original_rule` |
| no counterpart | ai.rs:1043 `combat_score < 30` | invented | Rescue operative combat floor; original rescue logic not recovered | `rescue_combat_floor_matches_original_rule` |
| no counterpart | ai.rs:1223 `skill >= 30` | invented | Research character skill floor; original research assignment not recovered | `research_skill_floor_matches_original_rule` |
| no counterpart | ai.rs:1370,1392,1405 `ticks: 15/25/30` | invented | Build times for troops/defense/mfg; original uses object-level timer delays, not hardcoded ticks | `production_ticks_match_original_build_delays` |
| no counterpart | ai.rs:1545 `facility_count * 10` | invented | System strength facility weighting; original (`FUN_00502020`) counts facilities without a multiplier | `facility_strength_weight_matches_fun_00502020` |
| no counterpart | ai.rs:1958 `support < 0.4` | invented | Uprising prevention threshold; original uprising is timer `0x38d` with no AI-side prevention pass | `uprising_prevention_threshold_has_source` |
| no counterpart | ai.rs:2008 `dispatched >= 2` | invented | Cap on uprising-prevention diplomats; no original counterpart | `uprising_diplomat_cap_has_source` |
| no counterpart | ai.rs:2183 `10000` / ai.rs:2186 `* 100` | invented | DS target scoring constants; original DS targeting not recovered | `ds_target_scoring_matches_original_rule` |
| no counterpart | ai.rs:2447 `<= available_strength * 3` | invented | Fleet force ratio gate; original uses `FUN_00502020` boolean adequacy, not a ratio | `fleet_force_ratio_matches_original_adequacy` |
| no counterpart | ai.rs:2500 `aggression > 0.5` | invented | Offensive threshold; depends on the invented aggression formula above | `offensive_threshold_matches_original_aggression` |
| multiple uses | ai.rs:595,607,621,635,810,899,928,967,1062,1171,2003 `duration_roll: 0.5` | invented | All 11 mission dispatch calls use 0.5; original mission durations use per-mission timer delays (`FUN_00586130`) | `mission_duration_roll_matches_original_timer_delays` |
| `FUN_00502020.c` counts all entities at a system | ai.rs:1518-1546 `system_strength` | wrong | Original counts ships + troops + facilities without weighting; port multiplies facilities by 10 and ignores fighter contribution | `system_strength_formula_matches_fun_00502020` |

## Needs decompile

- `FUN_0050a1b0` (faction-conditioned character placement): read but has indirect vtable dispatch at `FUN_004f7720`/`FUN_004f7790`; need those callees to recover character assignment rules and verify whether a diplomacy threshold exists.
- `FUN_00520580` (movement order issuer): called by both `FUN_00537180` and `FUN_005385f0` to issue fleet deployment; parameters not decoded.
- `FUN_0053e150` (clamped scaled division): called by `FUN_0053e170`; needed to recover the actual aggression formula behind `FUN_0053e190`.
- `FUN_005202d0` (system pre-validation): called early in `FUN_00537180` before deployment decisions.

## Summary

22 findings (excluding 3 already ledgered). 2 are **wrong** (aggression formula, system_strength weighting), 15 are **invented** (constants with no original counterpart that claim or imply parity), 5 are **port-owned** (structurally necessary because the port's polled architecture replaces the original's event-driven entity budgets). 4 functions need decompile to close the remaining gaps.
