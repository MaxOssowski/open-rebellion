# Invention Audit: Manufacturing, Repair, Troop Transport

All 38 functions across manufacturing.rs, repair.rs, and troop_transport.rs are **uncited** in the provenance scan.

## Findings

| Original fn + .c line | Port file:line | Verdict | Difference | Failing test name |
|---|---|---|---|---|
| FUN_0052b960 (manager tick, vtable +0x208) — **no .c** | manufacturing.rs:202 `advance_ticks` | **missing** | Port decrements `ticks_remaining` by 1/day; original increments mgr `+0x5c` at unknown rate until `+0x5c >= +0x68` (cost). Rate may depend on facility count or GNPRTB. | `build_progress_accrues_at_facility_rate_not_one_per_day` |
| FUN_0052b960 + build-delivery.md:39-41 (per-facility manager with own queue at `+0x54 -> +0x18`) | manufacturing.rs:231-237 `ManufacturingState` | **wrong** | Port keeps ONE queue per SystemKey. Original keeps one queue per manufacturing facility. Two shipyards at one system build two items in parallel. | `two_yards_at_one_system_build_two_items_simultaneously` |
| FUN_00528890.c:19 (`+0x50 & 2` and `+0x50 & 0x1c == 0`, side match) | manufacturing.rs:202 `advance_ticks` | **missing** | Original filters queued items: must be created, not completed/destroyed/en-route, same faction as facility. Port has no acceptance filter. | `queue_skips_completed_or_wrong_faction_items` |
| FUN_00511300.c:88 (`+0x88 & 0x20`, bit 5 = has_shipyard) | repair.rs:91 | **wrong** | Port checks `sys.manufacturing_facilities.is_empty()`. Original checks `+0x88 bit 5` (has_shipyard). Not all manufacturing facilities are shipyards; `ManufacturingFacilityInstance` has `is_shipyard` but repair.rs ignores it. | `repair_requires_shipyard_not_any_manufacturing_facility` |
| FUN_00509890.c:18,26 + FUN_0050ce80.c:14 (GNPRTB timer 0x386, repair rate from facility `+0x24 >> 6 & 3`) | repair.rs:115-116 (`class.damage_control`) | **invented** | Port uses ship class `damage_control` as repair rate. Original uses timer 0x386 with GNPRTB parameters 7693/7695, gated by `+0x88 bit 5`, reading a 2-bit rate from the facility object. `damage_control` has no decompile evidence as a repair rate. | `repair_rate_uses_gnprtb_facility_parameter_not_ship_damage_control` |
| Timer 0x386 (timer-scheduler.md:142, periodic with rand delay) | repair.rs:82 (single check per advance call) | **wrong** | Port fires repair once per `advance()` call regardless of tick count. Original uses a repeating timer with randomized delay. Multi-tick frames under-repair in the port. | `repair_applies_per_tick_not_per_frame` |
| economy-systems.md:211 (blockade troop destruction) | troop_transport.rs (absent) | **missing** | Original destroys regiments transported through a blockaded system. Port has no blockade-kills-cargo path. | `blockaded_transit_destroys_embarked_regiments` |
| ai.rs:1325 `capship_class.refined_material_cost.max(20)` used as `ticks` | ai.rs:1325 | **invented** | AI equates `refined_material_cost` directly with build duration in days. Without the FUN_0052b960 decompile confirming rate = 1/day, this is an assumption. | `ai_build_duration_matches_original_cost_to_rate_formula` |

## Needs Decompile

- **FUN_0052b960** — manufacturing manager tick (vtable `+0x208`). Contains the progress increment logic (`+0x5c` advancement rate). Without this, the port's 1-tick-per-day countdown cannot be verified. This is the single most important missing decompile for manufacturing correctness.
- **FUN_00512440** — `CombatUnitFastRepair` notification handler. Needed to confirm the full repair flow and which units qualify.

## Summary

8 findings: 2 invented, 3 wrong, 3 missing. The manufacturing system's single-queue-per-system design and the repair system's use of `damage_control` as the repair rate are the most impactful inventions — both contradict decompiled evidence. The manager tick `FUN_0052b960` remains the critical missing decompile.
