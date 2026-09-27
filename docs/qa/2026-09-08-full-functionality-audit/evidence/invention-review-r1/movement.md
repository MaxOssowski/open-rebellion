# Invention Audit: MOVEMENT, FOG, BLOCKADE

Scope: movement.rs, fog.rs, blockade.rs, bombardment.rs, tuning.rs MovementConfig, integrator.rs movement/blockade arms. All findings verified against .c decompiles, not .md notes alone.

## Findings

| Original fn + .c line | Port file:line | Verdict | Difference | Failing test name |
|---|---|---|---|---|
| FUN_0055d8c0.c:11 `iVar1 / DAT_006bb6e8` (GNPRTB[5120]=5), FUN_0053e190 difficulty mod | movement.rs:143 `distance * distance_scale / slowest_hyperdrive` | **invented** | Original: `(isqrt(dx^2+dy^2) / 5) * speed / 100` with difficulty modifier. Port: `ceil(sqrt(dx^2+dy^2) * 2 / slowest_hyperdrive)`. Completely different divisor, multiplier, and speed source. | `transit_ticks_match_recovered_formula_for_known_distance` |
| FUN_0055d860.c:5 `FUN_0053e1d0(...)` (integer sqrt) | movement.rs:125 `.sqrt()` (f64) | **wrong** | Original uses integer square root (binary search in FUN_0053e1d0). Port uses floating-point sqrt. Small rounding differences at all distances. | `distance_uses_integer_square_root` |
| movement.rs:59 `DISTANCE_SCALE=2` | tuning.rs:146 | **invented** | No `*2` multiplier exists in the original. The divisor is GNPRTB[5120]=5. The port invented this constant. Provenance scan: uncited. | `transit_does_not_apply_invented_distance_scale` |
| movement.rs:62 `MIN_TRANSIT_TICKS=10` | tuning.rs:147 | **invented** | Original minimum is 1 (FUN_0055d8c0.c:13 `iVar2 = 1`). Port clamps to 10. Provenance scan: uncited. | `minimum_transit_is_one_tick_not_ten` |
| movement.rs:65 `DEFAULT_FIGHTER_HYPERDRIVE=60` | tuning.rs:148 | **invented** | No default fighter hyperdrive in the original formula. Speed comes from object slot `+0x34`, not from a fleet's slowest capital ship. Provenance scan: uncited. | `fighter_fleet_uses_per_object_speed_not_default_hyperdrive` |
| FUN_0055d8c0.c:11 `FUN_0053e190(iVar1/DAT_006bb6e8, param_3)` | movement.rs:142-143 (no difficulty) | **missing** | Original applies a difficulty modifier via FUN_0053e190 -> FUN_0053e170 with DAT_00661a88. Port has no difficulty adjustment. | `transit_time_applies_difficulty_modifier` |
| FUN_00556430.c:59-68 per-object transit | movement.rs:83-155 fleet-level transit | **wrong** | Original moves individual objects (ship, regiment, facility) with their own speed slot `+0x34`. Port moves entire fleets with the slowest ship's hyperdrive. Related: F-030 (build delivery). | `individual_object_transits_independently_of_fleet` |
| bombardment.rs:1-202 entire file | bombardment.rs:* | **invented** | build-delivery.md proves FUN_00556430->FUN_00555d30->FUN_00555b30->FUN_0055d8c0 is the transit-time chain, not bombardment. The `short[2]` stats in FUN_00509620 are system coordinates at `+0x4c`, not combat stats. The "bombardment_modifier" and "Euclidean power ratio" are coordinate distance and travel time. The entire bombardment.rs is built on this misread. | `bombardment_module_does_not_use_transit_chain_as_damage` |
| bombardment.rs:190 `def_stat += 10` fallback | bombardment.rs:190 | **invented** | Provenance scan: uncited literal `10`. A placeholder defense value with no RE source. Moot given the entire module is misattributed. | `defense_fallback_has_provenance` |
| fog.rs:141 `SENSOR_MULTIPLIER=15.0` | fog.rs:141 | **invented** | No Ghidra source for this constant. No decompile references sensor radius, detection range, or the value 15. Provenance scan: uncited. | `sensor_multiplier_matches_original_detection_formula` |
| fog.rs:175 `progress() >= 0.5` reveal | fog.rs:175 | **invented** | No Ghidra source for revealing destination at 50% transit progress. Provenance scan: uncited. The original fog/visibility system is unrecovered. | `half_progress_reveal_has_original_source` |
| fog.rs:1-229 entire system | fog.rs:* | **port-owned** | The entire fog system (monotonic reveal, seed from fleet locations, sensor radius) has no recovered Ghidra source. Module header says "simplified Phase 0". Acceptable as augmentation if tagged. | `fog_system_tagged_as_port_augmentation` |
| blockade.rs:306-335 `withdraw_percent` | blockade.rs:306 | **match** | Matches FUN_0050b310 -> FUN_0055a020 -> FUN_0053e120. KDY-150 check, GNPRTB 7684/7685 penalties, clamp to 0..100 all confirmed against .c decompiles. | -- |
| blockade.rs:345-385 `running_regiments` | blockade.rs:345 | **match** | Correctly models FUN_00508660 -> FUN_0050c540 -> FUN_00504470 copy, FUN_00504960 reset-on-activation, and FUN_004f7640 departure trigger. | -- |
| blockade.rs:393-417 `resolve_running` | blockade.rs:407 | **match** | Roll logic matches FUN_00504990 -> FUN_0053e2f0 -> FUN_0053e2e0: draw in 0..=99, survive when draw < percent. | -- |
| blockade.rs:426-450 `system_is_blockaded` | blockade.rs:426 | **match** | Hostile fleet present and zero defenders matches the `+0x88` bit 5 precondition in FUN_0050b310. Neutral systems correctly excluded. | -- |

## Needs decompile

- Object speed slot `+0x34` per class (ship, fighter, regiment, facility): required to implement the real transit formula.
- FUN_0053e170 (difficulty modifier helper called by FUN_0053e190): only the .c stub exists (`FUN_0053e190.c`); the helper itself is missing.
- The real orbital bombardment entry point and formula: FUN_004ff840 (referenced in `combat-formulas.md:182`) or an unknown function. The chain attributed to bombardment is transit.

## Summary

**5 invented, 2 wrong, 1 missing, 4 match, 2 port-owned.** The movement transit formula is entirely invented: all three constants (DISTANCE_SCALE, MIN_TRANSIT_TICKS, DEFAULT_FIGHTER_HYPERDRIVE) lack provenance and the formula structure differs from the recovered `FUN_0055d8c0` chain. bombardment.rs is founded on a misread of the transit chain as a damage formula. The blockade troop-withdrawal system (withdraw_percent, running_regiments, resolve_running) is correct and well-sourced. fog.rs is a port-owned augmentation with two invented constants.
