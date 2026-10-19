//! The port's scripted story events.
//!
//! Every chain the port once scripted here has been replaced by the
//! original's own rules (the comments in [`define_story_events`] name
//! them), so it defines none. It stays as the hook a scenario's scripted
//! events would use.

use crate::events::EventState;
#[cfg(test)]
use crate::ids::CharacterKey;
use crate::world::GameWorld;

/// Search world.characters for a character whose name contains `needle` (case-insensitive).
#[cfg(test)]
fn find_character(world: &GameWorld, needle: &str) -> Option<CharacterKey> {
    let needle_lower = needle.to_lowercase();
    world
        .characters
        .iter()
        .find(|(_, c)| c.name.to_lowercase().contains(&needle_lower))
        .map(|(key, _)| key)
}

/// Register the port's scripted story events into `state`: none remain.
pub fn define_story_events(state: &mut EventState, world: &GameWorld) {
    let _ = (state, world);

    // The port's Force milestones (0x1e1 "Force Awakening", 0x1e5 "Force
    // Training Milestone") and "Leia Discovers the Force" (0x363) are gone:
    // 0x1e1 and 0x1e5 are the original's CharacterForceNotif and
    // CharacterForceTrainingNotif (entity-system.md), not messages; the
    // tiers they waited for are jedi.rs's own; and Leia's discovery is the
    // trainer's 0x362 (crate::force_discovery). See
    // ghidra/notes/jedi-rs-replacement.md.

    // The Dagobah chain (0x390 "Dagobah Calls", 0x392 "Yoda Agrees", 0x221,
    // 0x391 "First Training Day", 0x210) is gone: the mission manager
    // sends Luke to Dagobah on timer 0x392 (crate::dagobah,
    // ghidra/notes/dagobah.md), and the events that waited for 0x210 wait
    // for Luke's DagobahCompleted.

    // The Final Battle chain (0x393 "Vader Dispatched", 0x394, 0x395,
    // 0x396 "Final Battle Imminent", 0x220 "The Final Battle") is gone:
    // the mission manager runs the original's FinalBattleReady path
    // (crate::final_battle, ghidra/notes/final-battle.md).

    // The port's Emperor Arrival (0x230: day 90 and 100 Empire systems)
    // is gone: the character manager's SeatOfPower sends 0x230
    // (crate::seat_of_power, story-message-classes.md row 16).

    // The Jabba's palace chain (0x380..0x386, 0x399, 0x39a, 0x231,
    // 0x200) and the bounty hunters (0x397, 0x212, 0x398, the carbonite
    // countdown, 0x39b) are gone: the mission manager sends the Bounty
    // Hunters on timer 0x393 (crate::jabba, ghidra/notes/jabba-palace.md).

    // -----------------------------------------------------------------------
    // System notification events (non-character-specific)
    // Registered from original game event IDs found in community disassembly.
    // These fire based on game state conditions, not specific characters.
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // NOTE: 8 random notification events REMOVED (Phase 3b, Knesset Shamash).
    //
    // The original game generates incidents (uprising 0x152, informant 0x153,
    // disaster 0x154, resource 0x155) via state-transition logic in the economy
    // tick (FUN_0050a970), NOT random per-tick probability. Bits 16-19 of the
    // system's field_0x88 are compared against neutral galaxy state; incidents
    // fire only when these bits differ (state change).
    //
    // Maintenance shortfall is the side's autoscrap timer 0x382 (autoscrap.rs).
    // Saboteur/traitor are triggered by mission/loyalty systems, not random events.
    //
    // These will be wired into the economy tick in Phase 3b implementation.
    // -----------------------------------------------------------------------
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EVT_INFORMANT_INTEL;
    use crate::events::EVT_MAINTENANCE_SHORTFALL_EVENT;
    use crate::events::EVT_MANUFACTURING_IDLE;
    use crate::events::EVT_NATURAL_DISASTER;
    use crate::events::EVT_RESOURCE_DISCOVERY;
    use crate::events::EVT_SABOTEUR_DETECTED;
    use crate::events::EVT_SUPPORT_CHANGE;
    use crate::events::EVT_TRAITOR_REVEALED;
    use crate::events::{EventCondition, EventState, EventSystem, GameEvent, SystemTag};
    use crate::tick::TickEvent;
    use crate::world::Character;

    fn tick(n: u64) -> Vec<TickEvent> {
        vec![TickEvent { tick: n }]
    }

    fn make_character(name: &str, is_alliance: bool) -> Character {
        Character {
            name: name.into(),
            is_alliance,
            is_empire: !is_alliance,
            is_major: true,
            ..Default::default()
        }
    }

    fn make_world_with_characters() -> GameWorld {
        let mut world = GameWorld::default();
        world
            .characters
            .insert(make_character("Luke Skywalker", true));
        world
            .characters
            .insert(make_character("Darth Vader", false));
        world
            .characters
            .insert(make_character("Han Solo", true));
        {
            let mut yoda = make_character("Yoda", true);
            yoda.is_jedi_trainer = true;
            world.characters.insert(yoda);
        }
        world
            .characters
            .insert(make_character("Princess Leia", true));
        world
            .characters
            .insert(make_character("Chewbacca", true));
        world
    }

    #[test]
    fn define_story_events_empty_world_does_not_panic() {
        let world = GameWorld::default();
        let mut state = EventState::new();
        define_story_events(&mut state, &world);
        // No characters found — no story events registered
        // (notification events removed in Phase 3b — they belong in economy tick)
        assert!(state.events().is_empty());
    }

    #[test]
    fn the_invented_dagobah_chain_is_gone() {
        // The mission manager sends Luke to Dagobah on timer 0x392
        // (FUN_00549d60, FUN_00544030, FUN_0054a860; ghidra/notes/dagobah.md),
        // so the port's own 0x390, 0x392, 0x221, 0x391 and 0x210 events,
        // which no original code raises this way, are no longer defined.
        let world = make_world_with_characters();
        let mut state = EventState::new();
        define_story_events(&mut state, &world);
        for id in [0x390, 0x392, 0x221, 0x391, 0x210] {
            assert!(
                state.events().iter().all(|event| event.id != id),
                "{id:#x} is still defined"
            );
        }
    }

    // --- Task 8: New condition variant tests ---

    #[test]
    fn character_is_captive_evaluates_correctly() {
        let mut world = GameWorld::default();
        let han = world
            .characters
            .insert(make_character("Han Solo", true));

        let mut state = EventState::new();
        state.define(GameEvent {
            id: 1,
            name: "captive_test".into(),
            conditions: vec![EventCondition::CharacterIsCaptive { character: han }],
            actions: vec![],
            is_repeatable: false,
            enabled: true,
            system_tag: SystemTag::Story,
        });

        // Han not captive — should not fire
        let fired = EventSystem::advance(&mut state, &world, &tick(1), &[]);
        assert!(fired.is_empty(), "Han is not captive");

        // Make Han captive
        world.characters.get_mut(han).unwrap().is_captive = true;
        let mut state2 = EventState::new();
        state2.define(GameEvent {
            id: 2,
            name: "captive_test_true".into(),
            conditions: vec![EventCondition::CharacterIsCaptive { character: han }],
            actions: vec![],
            is_repeatable: false,
            enabled: true,
            system_tag: SystemTag::Story,
        });

        let fired2 = EventSystem::advance(&mut state2, &world, &tick(1), &[]);
        assert_eq!(fired2.len(), 1, "Han is captive, should fire");
    }

    #[test]
    fn event_not_fired_returns_true_when_unfired() {
        let world = GameWorld::default();
        let mut state = EventState::new();

        // Event 1: fires immediately. Event 2: requires event 99 NOT fired.
        state.define(GameEvent {
            id: 2,
            name: "not_fired_test".into(),
            conditions: vec![EventCondition::EventNotFired { id: 99 }],
            actions: vec![],
            is_repeatable: false,
            enabled: true,
            system_tag: SystemTag::Story,
        });

        let fired = EventSystem::advance(&mut state, &world, &tick(1), &[]);
        assert_eq!(
            fired.len(),
            1,
            "event 99 never fired, so EventNotFired should be true"
        );

        // Now define and fire event 99, then check EventNotFired again
        let mut state2 = EventState::new();
        state2.define(GameEvent {
            id: 99,
            name: "prereq".into(),
            conditions: vec![],
            actions: vec![],
            is_repeatable: false,
            enabled: true,
            system_tag: SystemTag::Story,
        });
        state2.define(GameEvent {
            id: 3,
            name: "not_fired_should_fail".into(),
            conditions: vec![EventCondition::EventNotFired { id: 99 }],
            actions: vec![],
            is_repeatable: false,
            enabled: true,
            system_tag: SystemTag::Story,
        });

        let fired2 = EventSystem::advance(&mut state2, &world, &tick(1), &[]);
        // Event 99 fires first, then event 3 checks EventNotFired(99) — should fail
        assert!(
            !fired2.iter().any(|f| f.event_id == 3),
            "event 99 fired in same advance, so EventNotFired(99) should be false"
        );
    }

    #[test]
    fn the_invented_force_events_are_gone() {
        // 0x1e1 and 0x1e5 are the original's CharacterForceNotif and
        // CharacterForceTrainingNotif (entity-system.md), raised on a field
        // change, not messages; the tiers the port's milestones waited for
        // have no source; Leia's discovery is the trainer's 0x362
        // (force_discovery.rs), so the port's 0x363 is no longer defined
        // (ghidra/notes/jedi-rs-replacement.md).
        let mut world = make_world_with_characters();
        let luke = find_character(&world, "Luke").unwrap();
        world.characters[luke].dagobah_completed = true;
        let mut state = EventState::new();
        define_story_events(&mut state, &world);
        assert!(state.events().is_empty());
        assert!(EventSystem::advance(&mut state, &world, &tick(100), &[]).is_empty());
    }

    #[test]
    fn the_invented_jabba_chain_is_gone() {
        // The mission manager sends the Bounty Hunters on timer 0x393
        // (FUN_00549d60, FUN_00546fa0; ghidra/notes/jabba-palace.md), so the
        // port's own bounty, carbonite and palace events, which no original
        // code raises this way, are no longer defined.
        let mut world = make_world_with_characters();
        world.characters.insert(make_character("Emperor Palpatine", false));
        let mut state = EventState::new();
        define_story_events(&mut state, &world);
        let gone = [
            0x200, 0x212, 0x231, 0x380, 0x381, 0x382, 0x383, 0x384, 0x385, 0x386, 0x387, 0x397,
            0x398, 0x399, 0x39a, 0x39b,
        ];
        for id in gone {
            assert!(
                !state.events().iter().any(|e| e.id == id),
                "event {id:#x} is still defined"
            );
        }
    }

    #[test]
    fn the_invented_final_battle_chain_is_gone() {
        // The mission manager runs the original's FinalBattleReady path
        // (FUN_00544da0 .. FUN_0054b040; ghidra/notes/final-battle.md), so
        // the port's own 0x393, 0x394, 0x395, 0x396 and 0x220 events, which
        // no original code raises this way, are no longer defined.
        let mut world = make_world_with_characters();
        world.characters.insert(make_character("Emperor Palpatine", false));
        let mut state = EventState::new();
        define_story_events(&mut state, &world);
        for id in [0x393, 0x394, 0x395, 0x396, crate::events::EVT_FINAL_BATTLE] {
            assert!(
                !state.events().iter().any(|e| e.id == id),
                "event {id:#x} is still defined"
            );
        }
    }

    // notification_events_fire_with_random_rolls — REMOVED (Phase 3b)
    // notification_events_need_correct_tick — REMOVED (Phase 3b)
    // Notification events moved to economy tick state-transition model.

    #[test]
    fn the_invented_emperor_arrival_is_gone() {
        // story-message-classes.md row 16: 0x230 is SeatOfPower
        // (FUN_005617b0), not a day-90 dominance gate.
        let mut world = make_world_with_characters();
        world.characters.insert(make_character("Emperor Palpatine", false));
        let mut state = EventState::new();
        define_story_events(&mut state, &world);
        assert!(!state.events().iter().any(|e| e.id == 0x230));
    }

    /// CI guard: notification events must NEVER use `EventCondition::Random`.
    ///
    /// The original game generates these on state transitions (bit-difference
    /// comparison against galaxy state), not random per-tick probability.
    /// See the Phase 3b (Knesset Shamash) comment above `define_story_events`.
    #[test]
    fn notification_events_never_use_random() {
        let banned_ids: &[u32] = &[
            EVT_SUPPORT_CHANGE,              // 0x100
            EVT_INFORMANT_INTEL,             // 0x153
            EVT_NATURAL_DISASTER,            // 0x154
            EVT_RESOURCE_DISCOVERY,          // 0x155
            EVT_MANUFACTURING_IDLE,          // 0x160
            EVT_MAINTENANCE_SHORTFALL_EVENT, // 0x304
            EVT_SABOTEUR_DETECTED,           // 0x305
            EVT_TRAITOR_REVEALED,            // 0x361
        ];

        let world = make_world_with_characters();
        let mut state = EventState::new();
        define_story_events(&mut state, &world);

        for event in state.events() {
            if banned_ids.contains(&event.id) {
                for cond in &event.conditions {
                    assert!(
                        !matches!(cond, EventCondition::Random { .. }),
                        "Event 0x{:x} ({}) must not use EventCondition::Random — \
                         notification events fire on state transitions, not random rolls. \
                         See story_events.rs Phase 3b comment.",
                        event.id,
                        event.name
                    );
                }
            }
        }
    }
}
