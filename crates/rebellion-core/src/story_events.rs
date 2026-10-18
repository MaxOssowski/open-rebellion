//! Scripted story events from the original game.
//!
//! Defines the 4 story event chains as `GameEvent` data structures.
//! Called at game start to register events in `EventState`.

use crate::events::{
    EventAction, EventCondition, EventState, GameEvent, SystemTag, EVT_CHARACTER_FORCE,
    EVT_EMPEROR_ARRIVAL, EVT_FORCE_TRAINING, EVT_LEIA_FORCE,
};
use crate::ids::CharacterKey;
use crate::world::{ForceTier, GameWorld};

/// Search world.characters for a character whose name contains `needle` (case-insensitive).
fn find_character(world: &GameWorld, needle: &str) -> Option<CharacterKey> {
    let needle_lower = needle.to_lowercase();
    world
        .characters
        .iter()
        .find(|(_, c)| c.name.to_lowercase().contains(&needle_lower))
        .map(|(key, _)| key)
}

/// Register all scripted story event chains into `state`.
///
/// Characters are looked up by name from `world`. If a required character
/// is not found, that chain is silently skipped (graceful degradation for
/// mods or scenarios without those characters).
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
pub fn define_story_events(state: &mut EventState, world: &GameWorld) {
    let luke = find_character(world, "Luke");
    let leia = find_character(world, "Leia");
    let emperor = find_character(world, "Palpatine").or_else(|| find_character(world, "Emperor"));

    // -----------------------------------------------------------------------
    // Force milestone events (Task 3)
    // -----------------------------------------------------------------------

    if let Some(luke) = luke {
        // Force Awareness milestone — fires when Luke reaches Aware
        state.define(GameEvent {
            id: EVT_CHARACTER_FORCE,
            name: "Force Awakening".into(),
            conditions: vec![
                EventCondition::CharacterHasForceLevel {
                    character: luke,
                    min_tier: ForceTier::Aware,
                },
                // is_repeatable: false handles one-shot — no self-ref EventNotFired needed
            ],
            actions: vec![EventAction::DisplayMessage {
                text: "A disturbance in the Force... Luke's potential has been noticed.".into(),
            }],
            is_repeatable: false,
            enabled: true,
            system_tag: SystemTag::Story,
        });

        // Force Training milestone — fires when Luke reaches Training
        state.define(GameEvent {
            id: EVT_FORCE_TRAINING,
            name: "Force Training Milestone".into(),
            conditions: vec![
                EventCondition::CharacterHasForceLevel {
                    character: luke,
                    min_tier: ForceTier::Training,
                },
                // is_repeatable: false handles one-shot
            ],
            actions: vec![EventAction::DisplayMessage {
                text: "Luke's Jedi training progresses.".into(),
            }],
            is_repeatable: false,
            enabled: true,
            system_tag: SystemTag::Story,
        });

        // -------------------------------------------------------------------
        // Chain 1: Luke Dagobah Training (with enrichment — Task 4)
        // -------------------------------------------------------------------

        // The Dagobah chain (0x390 "Dagobah Calls", 0x392 "Yoda Agrees", 0x221,
        // 0x391 "First Training Day", 0x210) is gone: the mission manager
        // sends Luke to Dagobah on timer 0x392 (crate::dagobah,
        // ghidra/notes/dagobah.md), and the events that waited for 0x210 wait
        // for Luke's DagobahCompleted.

        // The Final Battle chain (0x393 "Vader Dispatched", 0x394, 0x395,
        // 0x396 "Final Battle Imminent", 0x220 "The Final Battle") is gone:
        // the mission manager runs the original's FinalBattleReady path
        // (crate::final_battle, ghidra/notes/final-battle.md).

        // -------------------------------------------------------------------
        // Leia Force Discovery (0x362) — Leia discovers Force sensitivity
        // Requires Luke's Dagobah training to be complete first
        // -------------------------------------------------------------------
        if let Some(leia) = leia {
            state.define(GameEvent {
                id: EVT_LEIA_FORCE,
                name: "Leia Discovers the Force".into(),
                conditions: vec![
                    EventCondition::CharacterExists { character: leia },
                    // Luke must have completed training first
                    EventCondition::CharacterDagobahCompleted { character: luke },
                    EventCondition::TickAtLeast { tick: 80 },
                ],
                actions: vec![
                    EventAction::ModifyForceTier {
                        character: leia,
                        new_tier: ForceTier::Aware,
                    },
                    EventAction::DisplayMessage {
                        text: "Princess Leia has discovered her connection to the Force!".into(),
                    },
                ],
                is_repeatable: false,
                enabled: true,
                system_tag: SystemTag::Story,
            });
        }

        // -------------------------------------------------------------------
        // Emperor Arrival (0x230) — Emperor travels to the front
        // -------------------------------------------------------------------
        if let Some(emperor) = emperor {
            state.define(GameEvent {
                id: EVT_EMPEROR_ARRIVAL,
                name: "Emperor Arrives".into(),
                conditions: vec![
                    EventCondition::CharacterExists { character: emperor },
                    EventCondition::TickAtLeast { tick: 90 },
                    EventCondition::FactionControlsNSystems {
                        faction: crate::dat::Faction::Empire,
                        min_count: 100,
                    },
                ],
                actions: vec![EventAction::DisplayMessage {
                    text: "The Emperor has arrived to oversee operations personally.".into(),
                }],
                is_repeatable: false,
                enabled: true,
                system_tag: SystemTag::Story,
            });
        }

        // The Jabba's palace chain (0x380..0x386, 0x399, 0x39a, 0x231,
        // 0x200) and the bounty hunters (0x397, 0x212, 0x398, the carbonite
        // countdown, 0x39b) are gone: the mission manager sends the Bounty
        // Hunters on timer 0x393 (crate::jabba, ghidra/notes/jabba-palace.md).
    }

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
    use crate::events::{EventState, EventSystem};
    use crate::tick::TickEvent;
    use crate::world::ControlKind;
    use crate::world::{Character, ForceTier};

    fn tick(n: u64) -> Vec<TickEvent> {
        vec![TickEvent { tick: n }]
    }

    fn make_character(name: &str, force_tier: ForceTier, is_alliance: bool) -> Character {
        Character {
            name: name.into(),
            is_alliance,
            is_empire: !is_alliance,
            is_major: true,
            force_tier,
            ..Default::default()
        }
    }

    fn make_world_with_characters() -> GameWorld {
        let mut world = GameWorld::default();
        world
            .characters
            .insert(make_character("Luke Skywalker", ForceTier::Aware, true));
        world
            .characters
            .insert(make_character("Darth Vader", ForceTier::Experienced, false));
        world
            .characters
            .insert(make_character("Han Solo", ForceTier::None, true));
        {
            let mut yoda = make_character("Yoda", ForceTier::Experienced, true);
            yoda.is_jedi_trainer = true;
            world.characters.insert(yoda);
        }
        world
            .characters
            .insert(make_character("Princess Leia", ForceTier::None, true));
        world
            .characters
            .insert(make_character("Chewbacca", ForceTier::None, true));
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

    #[test]
    fn one_shot_story_events_do_not_refire() {
        let world = make_world_with_characters();
        let mut state = EventState::new();
        define_story_events(&mut state, &world);

        // Fire the Force Awakening milestone (Luke is Aware) at tick 50
        let fired1 = EventSystem::advance(&mut state, &world, &tick(50), &[]);
        let count1 = fired1
            .iter()
            .filter(|f| f.event_id == EVT_CHARACTER_FORCE)
            .count();
        assert_eq!(count1, 1);

        // Try again at tick 51 — should NOT fire again (one-shot)
        let fired2 = EventSystem::advance(&mut state, &world, &tick(51), &[]);
        let count2 = fired2
            .iter()
            .filter(|f| f.event_id == EVT_CHARACTER_FORCE)
            .count();
        assert_eq!(count2, 0, "one-shot event should not fire again");
    }

    // --- Task 8: New condition variant tests ---

    #[test]
    fn character_force_experience_evaluates_at_threshold() {
        let mut world = GameWorld::default();
        let luke = world
            .characters
            .insert(make_character("Luke", ForceTier::Aware, true));

        // Set force_experience to 25
        world.characters.get_mut(luke).unwrap().force_experience = 25;

        let mut state = EventState::new();
        state.define(GameEvent {
            id: 1,
            name: "xp_test".into(),
            conditions: vec![EventCondition::CharacterForceExperience {
                character: luke,
                min_xp: 20,
            }],
            actions: vec![],
            is_repeatable: false,
            enabled: true,
            system_tag: SystemTag::Story,
        });

        let fired = EventSystem::advance(&mut state, &world, &tick(1), &[]);
        assert_eq!(fired.len(), 1, "25 >= 20 should fire");

        // Reset with higher threshold
        let mut state2 = EventState::new();
        state2.define(GameEvent {
            id: 2,
            name: "xp_test_fail".into(),
            conditions: vec![EventCondition::CharacterForceExperience {
                character: luke,
                min_xp: 30,
            }],
            actions: vec![],
            is_repeatable: false,
            enabled: true,
            system_tag: SystemTag::Story,
        });

        let fired2 = EventSystem::advance(&mut state2, &world, &tick(1), &[]);
        assert!(fired2.is_empty(), "25 < 30 should not fire");
    }

    #[test]
    fn character_is_captive_evaluates_correctly() {
        let mut world = GameWorld::default();
        let han = world
            .characters
            .insert(make_character("Han Solo", ForceTier::None, true));

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
    fn force_milestone_events_fire_at_correct_tiers() {
        let world = make_world_with_characters();
        let mut state = EventState::new();
        define_story_events(&mut state, &world);

        // Luke starts as Aware — Force Awakening milestone should fire
        let fired = EventSystem::advance(&mut state, &world, &tick(1), &[]);
        assert!(
            fired.iter().any(|f| f.event_id == EVT_CHARACTER_FORCE),
            "Force Awakening should fire for Aware Luke"
        );

        // Force Training milestone requires Training tier — should NOT fire yet
        assert!(
            !fired.iter().any(|f| f.event_id == EVT_FORCE_TRAINING),
            "Force Training should not fire while Luke is only Aware"
        );
    }

    #[test]
    fn the_invented_jabba_chain_is_gone() {
        // The mission manager sends the Bounty Hunters on timer 0x393
        // (FUN_00549d60, FUN_00546fa0; ghidra/notes/jabba-palace.md), so the
        // port's own bounty, carbonite and palace events, which no original
        // code raises this way, are no longer defined.
        let mut world = make_world_with_characters();
        world.characters.insert(make_character(
            "Emperor Palpatine",
            ForceTier::Experienced,
            false,
        ));
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
        world.characters.insert(make_character(
            "Emperor Palpatine",
            ForceTier::Experienced,
            false,
        ));
        let mut state = EventState::new();
        define_story_events(&mut state, &world);
        for id in [0x393, 0x394, 0x395, 0x396, crate::events::EVT_FINAL_BATTLE] {
            assert!(
                !state.events().iter().any(|e| e.id == id),
                "event {id:#x} is still defined"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Phase 3 tests: Emperor, Leia Force
    // -----------------------------------------------------------------------

    #[test]
    fn leia_force_discovery_fires_after_dagobah() {
        let mut world = make_world_with_characters();
        let mut state = EventState::new();
        define_story_events(&mut state, &world);

        EventSystem::advance(&mut state, &world, &tick(30), &[]);
        EventSystem::advance(&mut state, &world, &tick(50), &[]);

        // Complete Dagobah: the mission's end sets Luke's DagobahCompleted
        // (FUN_005751d0).
        let luke_key = find_character(&world, "Luke").unwrap();
        world
            .characters
            .get_mut(luke_key)
            .unwrap()
            .dagobah_completed = true;
        EventSystem::advance(&mut state, &world, &tick(60), &[]);

        // Leia Force discovery (0x362) fires at tick 80 after Dagobah complete
        let fired = EventSystem::advance(&mut state, &world, &tick(80), &[]);
        assert!(
            fired.iter().any(|f| f.event_id == EVT_LEIA_FORCE),
            "Leia Force discovery should fire after Dagobah completion. Events: {:?}",
            fired
                .iter()
                .map(|f| (f.event_id, &f.event_name))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn leia_force_does_not_fire_before_dagobah() {
        let world = make_world_with_characters();
        let mut state = EventState::new();
        define_story_events(&mut state, &world);

        // At tick 80 without Dagobah completion — should NOT fire
        let fired = EventSystem::advance(&mut state, &world, &tick(80), &[]);
        assert!(
            !fired.iter().any(|f| f.event_id == EVT_LEIA_FORCE),
            "Leia Force should NOT fire before Dagobah completion"
        );
    }

    // notification_events_fire_with_random_rolls — REMOVED (Phase 3b)
    // notification_events_need_correct_tick — REMOVED (Phase 3b)
    // Notification events moved to economy tick state-transition model.

    #[test]
    fn emperor_palpatine_adds_emperor_specific_events() {
        let mut world = make_world_with_characters();
        world.characters.insert(make_character(
            "Emperor Palpatine",
            ForceTier::Experienced,
            false,
        ));

        let mut state = EventState::new();
        define_story_events(&mut state, &world);

        // Verify Emperor Arrival event (0x230) is registered
        assert!(
            state.events().iter().any(|e| e.id == EVT_EMPEROR_ARRIVAL),
            "EVT_EMPEROR_ARRIVAL should be registered when Emperor exists"
        );
    }

    #[test]
    fn emperor_arrival_fires_with_empire_dominance() {
        let mut world = make_world_with_characters();
        world.characters.insert(make_character(
            "Emperor Palpatine",
            ForceTier::Experienced,
            false,
        ));

        // Need 100+ Empire systems
        let sector_key = world.sectors.insert(crate::world::Sector {
            dat_id: crate::ids::DatId::new(0),
            name: "Sector".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        for i in 0..110 {
            world.systems.insert(crate::world::System {
                dat_id: crate::ids::DatId::new(i),
                name: format!("System_{i}"),
                sector: sector_key,
                x: 0,
                y: 0,
                exploration_status: crate::dat::ExplorationStatus::Explored,
                popularity_alliance: 0.3,
                popularity_empire: 0.7,
                is_populated: true,
                total_energy: 0,
                raw_materials: 0,
                espionage_rating: 0.0,
                fleets: vec![],
                ground_units: vec![],
                special_forces: vec![],
                defense_facilities: vec![],
                manufacturing_facilities: vec![],
                production_facilities: vec![],
                fighters: Vec::new(),
                is_headquarters: false,
                is_destroyed: false,
                control: ControlKind::Controlled(crate::dat::Faction::Empire),
            });
        }

        let mut state = EventState::new();
        define_story_events(&mut state, &world);

        let fired = EventSystem::advance(&mut state, &world, &tick(90), &[]);
        assert!(
            fired.iter().any(|f| f.event_id == EVT_EMPEROR_ARRIVAL),
            "Emperor Arrival should fire at tick 90 with 110 Empire systems"
        );
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
