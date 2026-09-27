# Invention Audit: BETRAYAL, JEDI, EVENTS, VICTORY

## Findings

| Original fn + .c line | Port file:line | Verdict | Difference | Test name |
|---|---|---|---|---|
| none recovered | betrayal.rs:22 `BETRAYAL_CHECK_INTERVAL=50` | invented | No original betrayal timer recovered; interval is unsourced | `betrayal_check_interval_matches_original_timer` |
| none recovered | betrayal.rs:109 `loyalty - 50` threshold | invented | Loyalty threshold 50 has no decompile source; UPRIS1TB lookup is plausible but uncited | `betrayal_loyalty_threshold_uses_original_cutoff` |
| `FUN_0057cca0` timer 0x388 -> `FUN_004ef100` .c:8 | no port counterpart | missing | Character injury field `+0x94` with GNPRTB 2563/2564 recovery; port has no injury field on Character | `injury_recovery_decrements_field_by_gnprtb_2563_or_2564` |
| `FUN_0057cff0` timer 0x389 | no port counterpart | missing | Character timer; action `FUN_00560a30` (149 lines) not decompiled | `timer_0x389_character_action_fires_on_schedule` |
| `FUN_0057d330` timer 0x38a -> `FUN_004ef150` | no port counterpart | missing | Bit 10 pulse while bit 8 on `+0xac`; port has no equivalent | `timer_0x38a_pulses_character_flag_bit` |
| none recovered | jedi.rs:51 `XP_TO_TRAINING=50` | invented | XP thresholds 50/150 have no decompile source; entity-system.md shows `force_experience` is a raw `short` accumulator | `jedi_xp_thresholds_match_original_tier_boundaries` |
| none recovered | jedi.rs:54 `XP_TO_EXPERIENCED=150` | invented | Same as above | `jedi_experienced_threshold_matches_original` |
| none recovered | jedi.rs:57 `DETECTION_CHECK_INTERVAL=30` | invented | No original Jedi detection cadence recovered; coverage-map.md marks this suspect | `jedi_detection_interval_matches_original_timer` |
| none recovered | jedi.rs:59-66 `DETECT_PROB_*` (0.05/0.15/0.30) | invented | Detection probabilities are unsourced; entity-system.md:192 shows a 3-function stat check, not a flat probability | `jedi_detection_probability_matches_original_formula` |
| `FUN_0058a3f0` entity-system.md:192 | jedi.rs:295 `detection_probability()` | wrong | Original uses a 3-condition formula (`FUN_0055e4d0` + `FUN_0055ff60` + `FUN_0058a530`) with `force_potential_raw`; port uses flat per-tier probabilities | `jedi_detection_uses_three_function_stat_check` |
| `FUN_004f1e00` .c:12 fires 0x1e1 | events.rs:72 `EVT_CHARACTER_FORCE=0x1e1` | match | Notification ID confirmed in decompile | -- |
| `FUN_004f1ea0` .c:12 fires 0x1e5 | events.rs:73 `EVT_FORCE_TRAINING=0x1e5` | match | Notification ID confirmed | -- |
| `FUN_004f20e0` .c:12 fires 0x362 | events.rs:80 `EVT_FORCE_DISCOVERED=0x362` | match | Notification ID confirmed | -- |
| `FUN_0054b7b0` .c:12 fires 0x221 | events.rs:77 `EVT_LUKE_DAGOBAH=0x221` | match | Notification ID confirmed | -- |
| `FUN_0056fc70` .c:12 fires 0x210 | events.rs:74 `EVT_DAGOBAH_COMPLETED=0x210` | match | Notification ID confirmed | -- |
| `FUN_00572b40` .c:12 fires 0x212 | events.rs:75 `EVT_BOUNTY_ATTACK=0x212` | match | Notification ID confirmed | -- |
| `FUN_0054ba00` .c:12 fires 0x220 | events.rs:76 `EVT_FINAL_BATTLE=0x220` | match | Notification ID confirmed | -- |
| timer 0x386 = `FUN_00565d90` support shift | events.rs:101 `EVT_SIDE_CHANGE=0x386` | wrong | 0x386 is a timer ID for support shift (GNPRTB 7693/7695); port assigns it to a "side change" story event | `evt_side_change_id_does_not_collide_with_timer_0x386` |
| timer 0x387 = `FUN_0057bf80` object arrival | events.rs:102 `EVT_JABBA_CAPTURES_CHEWIE=0x387` | wrong | 0x387 is a timer ID for object arrival after travel; port assigns it to Jabba captures Chewie | `evt_jabba_chewie_id_does_not_collide_with_timer_0x387` |
| no factory for 0x396 | story_events.rs:284 event 0x396 "Final Battle Imminent" | invented | 0x396 has no recovered factory in REBEXE.EXE; the entire "Vader dispatched/en route/reports" chain (0x393-0x396) is port-authored | `final_battle_imminent_chain_matches_original_mission_flow` |
| none recovered | story_events.rs:37-43 CARBONITE_STAGES (0x39C-0x3A0) | invented | 5-stage carbonite escape countdown is port-authored; no original event IDs 0x39C-0x3A0 recovered | `carbonite_escape_countdown_matches_original_timer_chain` |
| none recovered | story_events.rs:441-470 Han self-escape 0x384 prob 0.10 | invented | 0x384 is a timer ID for support shift; the self-escape probability and logic are port-authored | `han_self_escape_probability_matches_original` |
| none recovered | story_events.rs:475-498 Luke captured at palace 0x399 | invented | Event 0x399 has no recovered original; the Random(0.20) gate is unsourced | `luke_palace_capture_conditions_match_original` |
| none recovered | story_events.rs:686-709 Leia captured by Jabba 0x385 | invented | 0x385 is a timer ID for support shift; Leia capture scenario is port-authored | `leia_jabba_capture_conditions_match_original` |
| none recovered | story_events.rs:100-119 events 0x390, 0x391, 0x392 | invented | "Dagobah Calls" (0x390), "First Training Day" (0x391), "Yoda Agrees" (0x392) are enrichment; 0x390 and 0x392 are timer IDs in the original | `dagobah_enrichment_events_match_original_chain` |
| none recovered | story_events.rs:389-407 Emperor Arrival 0x230 | invented | EVT_EMPEROR_ARRIVAL conditions (tick 90, 100 Empire systems) are port-authored | `emperor_arrival_conditions_match_original_trigger` |
| `SideVictoryConditionsNotif` entity-system.md:456 | victory.rs:118 `VictorySystem::check` | partial | Victory check function address not isolated; port logic (HQ capture, bombardment, Death Star) is structurally plausible but the exact conditions are unverified against the decompile | `victory_conditions_match_original_side_check` |
| none recovered | victory.rs:214-227 `standard_leaders_captured` name lookup | invented | Leader names ("Emperor Palpatine", "Darth Vader", "Luke Skywalker", "Mon Mothma") matched by string; original likely uses entity IDs, not string comparison | `standard_victory_identifies_leaders_by_dat_id_not_name` |

## Needs Decompile

- `FUN_00560a30` (timer 0x389 action, 149 lines): character timer; unknown semantics.
- `FUN_0055e4d0`, `FUN_0055ff60`, `FUN_0058a530`: Jedi detection 3-function stat check chain; the port replaces this with flat per-tier probabilities.
- `SideVictoryConditionsNotif` handler: address not isolated; needed to verify exact victory condition logic.
- `FUN_00546fa0` (timer 0x393): galaxy manager action, unread.
- `FUN_00543af0` (timer 0x392): galaxy manager bit 0 action, unread.

## Summary

**9 match** (notification IDs confirmed against .c decompiles), **5 invented** (constants/thresholds with no source: betrayal interval/threshold, Jedi XP/detection values), **3 missing** (injury recovery timer 0x388 and two other character timers), **4 wrong** (Jedi detection formula replaced by flat probabilities; three timer ID collisions 0x386/0x387/0x384), **7 invented story events** (entire Jabba 5-case chain, carbonite countdown, Dagobah enrichment, Emperor Arrival, Final Battle sub-chain 0x393-0x396 all port-authored with unsourced tick/probability gates), **1 partial** (victory conditions structurally plausible but unverified).
