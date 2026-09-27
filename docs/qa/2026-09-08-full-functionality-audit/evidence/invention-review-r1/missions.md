# Missions Invention Audit

Scope: `crates/rebellion-core/src/missions.rs`, mission effect arms in `crates/rebellion-data/src/integrator.rs`.

## Findings

| Original fn + .c line | Port file:line | Verdict | Difference | Test name |
|---|---|---|---|---|
| `FUN_0055e470.c:12` FOILTB table lookup via `FUN_0053e340(0xc, ...)` | `missions.rs:740` `foil_prob()` quadratic `-0.001999*d^2 + 0.8879*d + 84.61` | **invented** | Port uses a rebellion2 Mission.cs quadratic instead of FOILTB piecewise-linear table. FOILTB.DAT is loaded (`lib.rs:94`) but never consulted for foil rolls; the quadratic replaces a 14-row table with different shape. Already ledgered as F-031 for `foil_prob`. | `foil_prob_matches_foiltb_table_at_shipped_thresholds` |
| `FUN_0055e470.c:12` detection input `((avg_team_esp - d.detection - sf_count) - officer_esp * GNPRTB[3589]/100) - GNPRTB[3584]` | `missions.rs:760-784` `compute_defense_score()` sums enemy character base espionage + system rating | **wrong** | Original detection rolls FOILTB per-defender with a composite input subtracting unit detection, special-force count, and two GNPRTB terms. Port collapses this into a single defense score summing enemy espionage, then feeds it to a quadratic. The entire detection phase (`FUN_0058a130`) is absent. | `detection_phase_uses_foiltb_per_defender_with_composite_input` |
| `FUN_0055e7e0` (uprising-incident.md:60) ESCAPETB input = `(p2 + p3) - p4 - p5` | `missions.rs:1140` uses `loyalty.base` as sole table input | **wrong** | Original escape formula is a 4-operand composite (per `sub_55cfb0` note). Port passes only loyalty, ignoring the other three terms. `sub_55cfb0` has no .c file. | `escape_uses_composite_input_not_loyalty_alone` |
| `FUN_00520cd0.c:27-32` leadership average over all mission members (team + decoy + captured via `FUN_00525bb0`) | `missions.rs:174` `skill_score()` reads one character | **wrong** | Original averages slot `+0x1f4` (leadership) across every member iterator. Port reads only the single agent's skill. Noted in `decoy-roll.md:251` and `coverage-map.md:107-110`. | `uprising_leadership_averages_all_mission_members` |
| `FUN_0055e410.c:12` decoy roll `(decoy_esp - defender_detection) - officer_esp * GNPRTB[3588]/100` via TDECOYTB/FDECOYTB | not ported | **missing** | Entire decoy phase absent. Already ledgered as F-019. | (ref F-019) |
| `missions.rs:158-159` Diplomacy coefficients `(0.005558, 0.7656, 20.15)`, Recruitment `(-0.001748, 0.8657, 11.923)` | `missions.rs:158-159` | **invented** | Sourced from rebellion2 Mission.cs, not from REBEXE.EXE MSTB tables. Comment says "from rebellion2" but no `hyp:` tag. Already ledgered as MissionKind::coefficients fallback. | (ref coefficients fallback) |
| `missions.rs:160-168` Sabotage..DeathStarSabotage coefficients | `missions.rs:160-168` | **invented** | Comment says "placeholder coefficients fit to approximate MSTB curves". No source, no `hyp:` tag. | `placeholder_coefficients_match_mstb_curves_within_5pct` |
| `missions.rs:326-335` tick_range values `(15,20)` Diplomacy, `(20,30)` Sabotage, etc. | `missions.rs:326-335` | **invented** | Comment says "Drawn from MISSNSD.DAT base_duration" but no specific MISSNSD row citation. The provenance scan marks `tick_range` as "cited" but the citation is the comment itself, not a verified MISSNSD field offset. | `tick_ranges_match_missnsd_base_duration_field` |
| `missions.rs:1049` sabotage `ticks_lost: 10` | `missions.rs:1049` | **invented** | Hardcoded 10 with comment "~10 ticks of production lost". No source from original game. | `sabotage_ticks_lost_matches_original_damage_formula` |
| `integrator.rs:1541-1550` sabotage removes facility entirely | `integrator.rs:1541-1550` | **wrong** | `MissionEffect::FacilitySabotaged` carries a `ticks_lost` field, but the integrator ignores it and destroys the facility outright (`remove`). The effect struct implies delay; the handler does deletion. | `sabotage_delays_facility_instead_of_destroying_it` |
| `missions.rs:1095` uprising `popularity_delta: 0.05` | `missions.rs:1095` | **invented** | Hardcoded 0.05 with comment "more impactful than diplomacy". No original source. The original uprising does not shift popularity by a fixed delta -- it triggers the uprising state machine (`FUN_0050c9f0`). | `incite_uprising_triggers_state_machine_not_popularity_shift` |
| `missions.rs:1109` DS sabotage `ticks_delayed: 50` | `missions.rs:1109` | **invented** | Hardcoded 50 with comment "significant setback". No original source. | `ds_sabotage_delay_matches_original_formula` |
| `missions.rs:1032` diplomacy `delta: 0.01` | `missions.rs:1032` | **invented** | Comment says "rebellion2 uses +1 on a 0-100 integer scale". Sourced from rebellion2, not REBEXE.EXE. The original fires `FUN_0050c9f0` (support change) with a GNPRTB value, not a fixed delta. | `diplomacy_support_shift_uses_gnprtb_not_fixed_delta` |

## Needs Decompile

- `FUN_0055cfb0` (sub_55cfb0): escape table input formula. The port cites it in `compute_table_input` docs but the .c file does not exist. ESCAPETB input `(p2 + p3) - p4 - p5` is from `uprising-incident.md:60` only.
- `FUN_00525040`: mission notification `0x1f0` sender. No .c file.
- `FUN_0055ae50`, `FUN_0055aed0`, `FUN_0055ae90`, `FUN_0055af50`, `FUN_0055b0a0`: the table-input composite functions cited in `compute_table_input` comments. No .c files; the port relies on the `uprising-incident.md` table consumer list.
- `FUN_00546ea0`: mission state-change entry (caller of `FUN_00547f60`). No .c file; `decoy-roll.md` describes it narratively.

## Summary

13 findings total (excluding 3 already-ledgered items referenced in-line). 4 wrong (detection phase collapse, escape input, sabotage destruction, uprising leadership averaging), 6 invented values (coefficients, tick ranges, deltas, delays), 1 missing (decoy phase, ref F-019), 2 structural mismatches (foil quadratic vs FOILTB, uprising effect vs state machine). 6 functions need decompile to close remaining gaps.
