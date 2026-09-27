# Invention Audit: Strategic Combat + Death Star

## Findings

| Original fn + .c line | Port file:line | Verdict | Difference | Test name |
|---|---|---|---|---|
| FUN_0055d8c0.c:8 `iVar1 / DAT_006bb6e8` (transit divisor, value 5) | combat.rs:18-21 `GNPRTB_COMBAT_DIFFICULTY_MODIFIER: u16 = 0x1400` | **wrong** | 0x1400 is the transit distance divisor (build-delivery.md), not a combat difficulty modifier; shipped value 5, not a percentage | `gnprtb_0x1400_is_transit_divisor_not_combat_difficulty` |
| FUN_0055d8c0 + build-delivery.md confirmation | combat.rs:214-223, 1125-1133 difficulty_mod scaling | **invented** | Difficulty-scaled combat damage reads a transit parameter; no decompile shows FUN_0053e190 called from any combat vtable handler | `combat_damage_is_not_scaled_by_gnprtb_0x1400` |
| FUN_00542050.c: 1-line thunk to FUN_00618b60 (CRT) | combat.rs:238-263 Emperor 1.5x bonus | **invented** | Port admits "no recovered source (F-024)"; FUN_00542050 is a CRT wrapper, not a combat modifier; 1.5 multiplier is fabricated | `emperor_combat_bonus_has_no_original_source` |
| No decompile (vtable +0x1c4 handler unrecovered) | combat.rs:457-459 `variance = total_per * 0.2 * (roll*2-1)` | **invented** | +/-20% RNG variance on weapon damage has no decompile source; the vtable handler that computes per-weapon damage is unrecovered | `weapon_fire_variance_needs_decompile` |
| No decompile (vtable +0x1d4 handler unrecovered) | combat.rs:847-916 `atk_str + maneuver / 2.0`, attrition 0.4 | **invented** | Fighter dogfight power formula and 0.4 attrition rate are fabricated; FUN_005444e0.c is the orchestrator only, actual fighter math is in vtable +0x1d4 | `fighter_dogfight_formula_needs_decompile` |
| No decompile | combat.rs:781-821 `laser_power / 100` guaranteed kills | **invented** | "Every 100 laser points removes one squadron" rule has no decompile source; capital-ship anti-fighter screening is entirely port-authored | `capital_ship_anti_fighter_formula_needs_decompile` |
| No decompile | combat.rs:710-773 fighters_attack_ships | **invented** | Fighter-vs-capital-ship attack using overall_attack_strength, random target selection, shield absorption is port-authored; no vtable handler decompiled | `fighters_attack_ships_formula_needs_decompile` |
| FUN_004ee350.c: sets +0x96 to param_1, fires vtable +0x330 | combat.rs:1182-1253 ground per-unit resolution | **invented** | FUN_004ee350 is a setter, not a calculator; the hit probability formula, class attack/defense scaling, regiment_strength/100 are all port-authored | `ground_combat_hit_formula_needs_caller_decompile` |
| No decompile cited | combat.rs:1146-1159 `1.0 + total / 200.0` officer bonus | **invented** | Comment cites "community disassembly cross-reference" but no .c function or line; the 200.0 divisor is uncited | `officer_combat_bonus_divisor_is_uncited` |
| FUN_00560d50.c does not sum facility defense | combat.rs:1099-1121 facility_defense_bonus | **invented** | Ground combat decompile iterates troops (0x14-0x1b) but does not reference defense facility stats; bombardment_defense is a bombardment field | `ground_facility_defense_bonus_not_in_decompile` |
| FUN_00544130.c: orchestrator checks +0x78 bit7 and +0x58 bit6, dispatches vtable +0x1c8 | combat.rs:487-532 shield absorption math | **missing** | Ion 2x, recharge formula (shield_nibble/15 * shield_max), raw_consumed back-conversion are plausible but the actual vtable +0x1c8 handler is unrecovered | `shield_absorption_coefficients_need_vtable_handler` |
| FUN_005617b0.c: checks 0x90000109 (Coruscant) + alive flags | death_star.rs:241 `fire()` | **port-owned** | Superlaser fire path is unrecovered; port admits this. FUN_005617b0 is SeatOfPower, correctly excluded from fire preconditions | `superlaser_fire_path_is_port_authored` |
| No decompile | combat.rs:355-356 `weapon_nibble: 0x0f`, `shield_nibble` from class | **invented** | Snapshot initializes weapon_nibble to max (0x0f) without source; shield_nibble maps shield_recharge_rate but the C++ +0x64 nibble packing is unverified | `snapshot_fleet_nibble_init_is_uncited` |
| GNPRTB exhaustive search (death_star.rs comment) | death_star.rs:49 `DEATH_STAR_CONSTRUCTION_TICKS = 1825` | **invented** | Acknowledged "best available approximation"; not parameterized in binary | `death_star_construction_ticks_is_approximation` |
| FUN_00512480 (51-byte dispatcher) | death_star.rs:57 `NEARBY_WARNING_RADIUS = 300` | **invented** | Acknowledged "best available approximation"; FUN_00512480 has no embedded threshold | `nearby_warning_radius_is_approximation` |

## Needs decompile

- Vtable +0x1c4 handler: actual per-weapon damage calculation (weapon fire phase math)
- Vtable +0x1c8 handler: actual shield absorption formula (ion multiplier, recharge rate)
- Vtable +0x1d0 handler: actual hull damage application (difficulty scaling source)
- Vtable +0x1d4 handler: actual fighter engagement formula (dogfight, anti-ship, screening)
- FUN_00560d50 caller chain above FUN_004ee350: the function that computes the new regiment_strength value passed as param_1

## Summary

15 findings: 0 match, 10 invented, 2 wrong, 2 missing, 1 port-owned. The GNPRTB 0x1400 misidentification is the most critical wrong finding -- it poisons both space and ground combat difficulty scaling with a transit parameter (shipped value 5, not a percentage). The core combat math (weapon fire variance, fighter dogfight, ground hit probability, anti-fighter screening) is entirely port-authored because the vtable handlers that contain the original formulas remain unrecovered.
