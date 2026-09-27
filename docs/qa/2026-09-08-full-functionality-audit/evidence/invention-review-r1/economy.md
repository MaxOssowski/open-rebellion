# Invention Audit: ECONOMY, SUPPORT, RESEARCH

## Findings

| Original fn + .c line | Port file:line | Verdict | Difference | Test name |
|---|---|---|---|---|
| FUN_00508250 setup stage 0x17 | economy.rs:5 | **wrong** | Port header says "Runs every tick"; original runs once at setup stage 0x17, support recalculated by field hooks | `economy_pipeline_fires_on_field_hook_not_every_tick` |
| Timers 0x384-0x386 (FUN_0050caa0/cad0/cb20) | economy.rs:727 | **invented** | Original fires support shifts on per-system timers with random delays; port runs `calculate_support_drift` every tick for every system | `support_drift_fires_only_on_timer_expiry` |
| FUN_00559b60:3 doubles param_6 (fleets) | economy.rs:802 | **wrong** | FUN_00559c40 passes param_6 (from FUN_00509020) through FUN_00559b60; port doubles troops instead | `empire_strong_support_doubles_fleet_suppression_not_troops` |
| FUN_0050c9f0:5 calls FUN_00559be0 | economy.rs:727 | **missing** | FUN_00559be0 conditionally divides positive Alliance shifts and negative Empire shifts by GNPRTB 7681; port omits this entirely | `support_shift_divides_by_gnprtb_7681_for_alliance_positive` |
| FUN_0050c9f0:10 adds current support before clamp | economy.rs:727 | **wrong** | Original adds current support to delta then clamps 0-100; port clamps drift independently of current support | `support_shift_adds_current_support_before_clamping` |
| FUN_0050b610:15 no energy threshold | economy.rs:900 | **invented** | Original sets control=0 (uncontrolled) when no troops from either side; port preserves control if energy >= GNPRTB 7760 | `no_troops_both_sides_sets_control_uncontrolled` |
| FUN_0050b610:7 uses FUN_0055a080 loyalty | economy.rs:882 | **wrong** | Original resolves faction from loyalty via FUN_0055a080 and bit-field checks; port uses simplified branch on troop presence only | `system_control_checks_loyalty_when_no_troops` |
| FUN_0050b310:3 blockade withdraw % | economy.rs:478 | **wrong** | FUN_0050b310 computes troop withdraw percent at field +0x74; port misreads it as KDY production modifier via GNPRTB 7684/7685 | `kdy_penalty_uses_blockade_withdraw_not_production_modifier` |
| Timer 0x381 FUN_00530460 | (none) | **missing** | Original re-sums three per-system resource quantities per side every 30+rand(30) days; port has no periodic tally | `faction_resource_tally_runs_on_timer_0x381` |
| EVT 0x304 FUN_00511780 | economy.rs:603 | **invented** | Port invents 30-tick cooldown per faction from GNPRTB 7694; original fires 0x304 from FUN_00511780 with no per-faction cadence | `maintenance_shortfall_fires_from_hook_not_cooldown` |
| GNPRTB 7686/7687/7688 mapping | economy.rs:45-47 | **wrong** | Port assigns fleet=7686(10), fighter=7687(5), troop=7688(2); decompile shows param_4*7686, param_5*7687, adjusted_param_6*7688 but param-to-unit mapping is unverified | `support_drift_influence_ids_match_original_unit_types` |
| research.rs sourced from rebellion2 | research.rs:12 | **invented** | Entire file cites `rebellion2/Faction.cs`; RESEARCH_MIN_TICKS=10, RESEARCH_DEFAULT_TICKS=30, RESEARCH_MAX_LEVEL=15 all uncited against REBEXE | `research_tick_constants_match_original_binary` |
| integrator apply_research_results | integrator.rs | **invented** | Research level-up integrator has no Ghidra backing; inherits rebellion2 provenance from research.rs | `research_integrator_matches_original_level_up_path` |
| F-029 informant/resource triggers | economy.rs:1018 | *(ledgered)* | Port invents troop_surplus < 0 and overcapped triggers; original uses timers 0x38e/0x390 | — |
| F-031 SupportTier 20/30/40/60 | economy.rs:159 | *(ledgered)* | Tier boundaries uncited | — |

## Needs Decompile

| Function | Why |
|---|---|
| FUN_00509020, FUN_00509090, FUN_00508fb0, FUN_00509140 | Confirm which returns fleet / fighter / troop counts for support drift; resolves GNPRTB 7686/7687/7688 unit assignment and Empire doubling target |
| FUN_0053e170 | Collection rate integer division leaf; port formula is plausible but unverified |
| FUN_0055a080 | Loyalty-to-faction mapping used in FUN_0050b610; port omits this path entirely |
| FUN_0055a020 | Called by FUN_0050b310 for blockade withdraw; port misreads the whole function |
| FUN_00511780 | Maintenance shortfall sender; needed to verify original 0x304 trigger conditions |

## Summary

**12 new findings** (4 wrong, 4 invented, 2 missing, 2 wrong/unverified), plus 2 already-ledgered references. The economy pipeline runs on an invented per-tick cadence instead of the original's setup-once + field-hook + timer architecture. The research system is entirely from rebellion2 with zero REBEXE.EXE citations. Five functions need decompile to close the remaining ambiguities, the most critical being FUN_00509020/00509090/00508fb0/00509140 which would resolve the Empire doubling target and GNPRTB influence ID assignments.
