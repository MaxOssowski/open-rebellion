---
title: "Story Events System"
description: "Scripted story chains, telemetry twins, and event chaining mechanics"
category: "agent-docs"
created: 2026-04-12
updated: 2026-10-18
tags: [story-events, event-chains, jabba, final-battle, bounty-hunters, dagobah]
---

# Story Events System

`story_events.rs` — Scripted story chains defined via `define_story_events()`.

## Architecture

Events are registered in `EventState` by `define_story_events(&mut state, &world)`. The function receives character keys by name-search (`find_character`), and all event definitions are gated by `if let Some(char)` to handle worlds where characters don't exist.

**Single-pass linear evaluation**: `EventSystem::advance()` evaluates events in definition order (vector index 0..n). When an event fires, its ID is inserted into `fired_ids` immediately inline, so events defined LATER in the same pass can chain off earlier events via `EventFired`. This is NOT a fixpoint loop.

**Same-tick chaining rule**: Events MUST be defined AFTER their prerequisites to chain in the same tick. Events defined BEFORE their prerequisite fire on the NEXT tick.

## Chains

### Chain 1: Luke Dagobah Training
`0x390` (prerequisite gate) → `0x392` (Yoda teacher) → `EVT_LUKE_DAGOBAH` (0x221) → `0x391` (first training) → `EVT_DAGOBAH_COMPLETED` (0x210)

### Chain 2: Final Battle (removed)
The port's `0x393`..`0x396` and `0x220` events are gone (save v76). The mission manager's FinalBattleReady path runs in `final_battle.rs` (`ghidra/notes/final-battle.md`), and the encounter class posts The Final Battle (`story_messages::final_battle_message`). Only Luke's confrontation with Vader (`FUN_00560200`) and Leia's discovery set `heritage_known` (save v75). DarthPickup and the Pickup mission (`0x73`) run in `jabba.rs`.

### Chains 3 and 4: Bounty Hunters and Jabba's Palace (removed)
The port's `0x397`..`0x39b`, `0x380`..`0x387`, `0x212`, `0x200`, `0x231` and carbonite `0x39c`..`0x3a0` events are gone (save v77). The mission manager's BountyHuntersActive path runs in `jabba.rs` (`ghidra/notes/jabba-palace.md`): timer `0x393`, the Bounty Hunters unit, the Bounty mission (`0x65000083`) and Han's trip to the palace. The app posts notification `0x212` (`story_messages::bounty_messages`). HanCapturedAtPalace and the Palace missions (`0x64000044`) are ported too, with Jabba Captures (`story_messages::jabba_captures_message`); DarthPickup and the Pickup mission (`0x73000082`) follow, with Jabba Delivers Prisoners (`story_messages::jabba_delivers_message`).

### Emperor Arrival (removed)
The port's `0x230` (day 90 and 100 Empire systems) is gone (save v78). The character manager's SeatOfPower runs in `seat_of_power.rs`, and the app posts Emperor Arrives at Coruscant (`story_messages::emperor_arrives_message`).

## Key Patterns

### OR-branch consolidator
Since `EventCondition` uses AND logic, OR semantics require multiple event definitions with the same `id`, each gated on a different trigger + `EventNotFired { id: SELF }` to prevent duplicates.

### Random roll budget
`EventSystem::advance()` panics if more `EventCondition::Random` conditions are evaluated than rolls provided. Conditions short-circuit on AND failure, so a failed `TickAtLeast` before `Random` means no roll consumed. Tests must provide enough rolls for all Random conditions actually reached.

## Constants (events.rs)

`EVT_BOUNTY_ATTACK` (0x212), `EVT_HAN_RESCUE` (0x200), `EVT_JABBA_PRISONERS` (0x231), `EVT_JABBA_CAPTURES_CHEWIE` (0x387) and `EVT_HAN_PERMANENT_FREEZE`/`EVT_HAN_CARBONITE_FAIL_1..5` (0x39b..0x3a0) remain as ids, but no story event defines them since save v77. Jabba Captures is posted for the Palace mission's failed rescuer (`story_messages::jabba_captures_message`), not for `0x387`.
