//! Jabba's palace, first part: the bounty hunters
//! (`ghidra/notes/jabba-palace.md`).
//!
//! The mission manager's setup sets `BountyHuntersActive`, whose hook arms
//! timer `0x393`. When it fires, a Bounty Hunters unit is created in
//! Jabba's Palace and sent after Han on the Bounty mission
//! (`FUN_00546fa0`). A Han the mission captures autoroutes to the palace
//! (`FUN_00543040`).
//!
//! port: the original runs the autorouting from the manager's handlers;
//! the port starts it each day.

use serde::{Deserialize, Serialize};

use crate::ids::{AbodeKey, CharacterKey, SystemKey};
use crate::missions::{MissionFaction, MissionKind, MissionMember, MissionRequest, MissionState};
use crate::tick::TickEvent;
use crate::world::GameWorld;

/// Han's record (`0x33000243`, `FUN_005435f0` -> `FUN_00506f50`).
const HAN: u32 = 0x243;
/// Jabba's Palace (`0x98000481`, the galaxy's `+0xdc`, `FUN_00506ed0`).
const JABBAS_PALACE: u32 = 0x481;
/// The Bounty Hunters class (SPECFCSD `0x3c000009`, `FUN_00546fa0`).
const BOUNTY_HUNTERS: u32 = 0x3c00_0009;

/// GNPRTB 6148 (`DAT_006bb550`): timer `0x393`'s least delay
/// (`FUN_00549d60`).
const GNPRTB_BOUNTY_MIN: u16 = 6148;
/// GNPRTB 6148 as shipped.
const SHIPPED_BOUNTY_MIN: i64 = 300;
/// GNPRTB 6149 (`DAT_006bb548`): timer `0x393`'s spread.
const GNPRTB_BOUNTY_SPREAD: u16 = 6149;
/// GNPRTB 6149 as shipped.
const SHIPPED_BOUNTY_SPREAD: i64 = 300;
/// GNPRTB 6150 (`DAT_006bb5b8`): the chance the fire sends the hunters
/// (`FUN_00546fa0`).
const GNPRTB_BOUNTY_CHANCE: u16 = 6150;
/// GNPRTB 6150 as shipped.
const SHIPPED_BOUNTY_CHANCE: i64 = 100;

/// The mission manager's bounty state (save v77).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JabbaState {
    /// Whether the setup ran (the first advance runs it).
    pub armed: bool,
    /// `BountyHuntersActive`, manager `+0x58` bit `0x4000`.
    pub bounty_hunters_active: bool,
    /// The day timer `0x393` fires, while armed.
    pub bounty_due: Option<u64>,
}

/// What a day asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JabbaStep {
    /// Timer `0x393`'s fire (`FUN_00546fa0`): Bounty Hunters from the
    /// palace after Han, at the system he stands at.
    BountyHunt {
        han: CharacterKey,
        palace: AbodeKey,
        at: SystemKey,
        tick: u64,
    },
    /// A captive of the Bounty mission autoroutes from `from` to the
    /// palace (`FUN_00543040`).
    ToPalace {
        captive: CharacterKey,
        from: SystemKey,
        palace: AbodeKey,
        tick: u64,
    },
}

fn palace(world: &GameWorld) -> Option<AbodeKey> {
    world
        .abodes
        .iter()
        .find(|(_, abode)| abode.dat_id.index() == JABBAS_PALACE)
        .map(|(key, _)| key)
}

/// The system a character stands at, through its fleet. A character in an
/// abode has no system (`enter_abode`).
fn standing(world: &GameWorld, key: CharacterKey) -> Option<SystemKey> {
    let c = world.characters.get(key)?;
    match c.current_fleet {
        Some(fleet) => world.fleets.get(fleet).map(|f| f.location),
        None => c.current_system,
    }
}

/// Stateless system for the bounty hunters.
pub struct JabbaSystem;

impl JabbaSystem {
    /// The rolls [`Self::advance`] draws: timer `0x393`'s delay
    /// (`FUN_00549d60`) and the fire's chance (`FUN_00546fa0`).
    #[must_use]
    pub fn roll_budget() -> usize {
        2
    }

    /// The setup (`FUN_00549d60`: `BountyHuntersActive`, whose hook
    /// `FUN_0054b740` arms timer `0x393`), the timer's single fire
    /// (`FUN_0054e970` -> `FUN_00546fa0`), and the day's autorouting of
    /// the Bounty mission's captives.
    ///
    /// port: the setup runs on the first advance, as Dagobah's does; the
    /// game-type test on the manager's `+0x24` is not read.
    pub fn advance(
        state: &mut JabbaState,
        world: &GameWorld,
        missions: &MissionState,
        tick_events: &[TickEvent],
        rolls: &[f64],
    ) -> Vec<JabbaStep> {
        let Some(tick) = tick_events.last().map(|event| event.tick) else {
            return Vec::new();
        };
        let param = |id, shipped| {
            i32::try_from(crate::movement::gnprtb_or_shipped(world, id, shipped)).unwrap_or(0)
        };
        let mut rolls = crate::uprising::Rolls::new(rolls);
        if !state.armed {
            state.armed = true;
            state.bounty_hunters_active = true;
            state.bounty_due = Some(
                tick + crate::uprising::timer_delay(
                    param(GNPRTB_BOUNTY_MIN, SHIPPED_BOUNTY_MIN),
                    param(GNPRTB_BOUNTY_SPREAD, SHIPPED_BOUNTY_SPREAD),
                    &mut rolls,
                ),
            );
        }
        let mut steps = Vec::new();
        let Some(palace) = palace(world) else {
            return steps;
        };
        let han = world
            .characters
            .iter()
            .find(|(_, c)| c.dat_id.index() == HAN && !c.is_killed)
            .map(|(key, _)| key);
        if state.bounty_hunters_active && state.bounty_due.is_some_and(|due| due <= tick) {
            state.bounty_due = None;
            let chance = crate::movement::gnprtb_or_shipped(
                world,
                GNPRTB_BOUNTY_CHANCE,
                SHIPPED_BOUNTY_CHANCE,
            );
            let hit = crate::uprising::Draws::chance(
                &mut rolls,
                i32::try_from(chance).unwrap_or(i32::MAX),
            );
            // Han located, not captured and usable.
            if let Some(han) = han.filter(|_| hit) {
                let free = world.characters.get(han).is_some_and(|c| !c.is_captive);
                if let Some(at) = standing(world, han).filter(|_| free) {
                    steps.push(JabbaStep::BountyHunt {
                        han,
                        palace,
                        at,
                        tick,
                    });
                }
            }
        }
        // FUN_004f18e0 sets Autorouting on the capture; FUN_00543040 sends
        // a captive of the Bounty mission to the palace.
        for (key, c) in &world.characters {
            if c.bounty_captured
                && c.is_captive
                && !c.is_killed
                && c.current_abode != Some(palace)
                && !missions.is_autorouting(key)
            {
                if let Some(from) = standing(world, key) {
                    steps.push(JabbaStep::ToPalace {
                        captive: key,
                        from,
                        palace,
                        tick,
                    });
                }
            }
        }
        steps
    }
}

/// Carry out a step. A hunt creates the Bounty Hunters unit in the palace
/// for the palace's side (`FUN_004f7d50`), gives the Empire Han's location
/// (`FUN_00556c70(Han, 2)`; port: the port reveals the system) and
/// dispatches the Bounty mission (`0x65000083`, target Han). Returns the
/// mission's id.
///
/// port: the unit takes its class's base skills; the shipped class has no
/// variance to roll.
pub fn perform(
    step: &JabbaStep,
    missions: &mut MissionState,
    world: &mut GameWorld,
) -> Option<u64> {
    match *step {
        JabbaStep::BountyHunt {
            han,
            palace,
            at,
            tick,
        } => {
            let class_dat_id = crate::ids::DatId::new(BOUNTY_HUNTERS);
            let class = world.special_force_classes.get(&class_dat_id)?;
            let skills = class.skills.map(|pair| pair.base);
            let is_alliance = world
                .abodes
                .get(palace)
                .is_some_and(|abode| abode.side == crate::dat::Faction::Alliance);
            let unit = world.special_forces.insert(crate::world::SpecialForceUnit {
                class_dat_id,
                is_alliance,
                skills,
                on_mission: false,
            });
            world.abodes.get_mut(palace)?.special_forces.push(unit);
            if let Some(system) = world.systems.get_mut(at) {
                system.exploration_status = crate::dat::ExplorationStatus::Explored;
            }
            let request = MissionRequest {
                kind: MissionKind::Bounty,
                faction: if is_alliance {
                    MissionFaction::Alliance
                } else {
                    MissionFaction::Empire
                },
                team: vec![MissionMember::SpecialForce(unit)],
                decoys: Vec::new(),
                target_system: at,
                target_character: Some(han),
                target_object: None,
                tick,
            };
            missions.dispatch_guarded(request, world).ok()
        }
        JabbaStep::ToPalace {
            captive,
            from,
            palace,
            tick,
        } => {
            crate::missions::move_member(world, MissionMember::Character(captive), None);
            missions.autoroute_to_abode(captive, from, palace, tick);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::Faction;
    use crate::ids::DatId;
    use crate::world::{Abode, Character, ControlKind, System};

    fn world() -> (GameWorld, CharacterKey, SystemKey, AbodeKey) {
        let mut world = GameWorld::default();
        let home = world.systems.insert(System {
            dat_id: DatId::new(0x9000_0001),
            name: "Tatooine".into(),
            sector: crate::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Unexplored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
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
            fighters: vec![],
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Alliance),
        });
        let han = world.characters.insert(Character {
            dat_id: DatId::new(HAN),
            name: "Han Solo".into(),
            is_alliance: true,
            current_system: Some(home),
            ..Character::default()
        });
        let palace = world.abodes.insert(Abode {
            dat_id: DatId::new(JABBAS_PALACE),
            name: "Jabba's Palace".into(),
            side: Faction::Empire,
            special_forces: Vec::new(),
        });
        (world, han, home, palace)
    }

    fn day(state: &mut JabbaState, world: &GameWorld, tick: u64, rolls: &[f64]) -> Vec<JabbaStep> {
        JabbaSystem::advance(
            state,
            world,
            &MissionState::new(),
            &[TickEvent { tick }],
            rolls,
        )
    }

    #[test]
    fn the_setup_activates_the_bounty_hunters_and_arms_timer_0x393() {
        // FUN_00549d60 sets BountyHuntersActive; its hook arms timer 0x393
        // for GNPRTB 6148 (300) + rand(0..=6149 (300)) days.
        for (roll, delay) in [(0.0, 300), (0.999, 600)] {
            let (world, ..) = world();
            let mut state = JabbaState::default();
            day(&mut state, &world, 5, &[roll]);
            assert!(state.armed && state.bounty_hunters_active);
            assert_eq!(state.bounty_due, Some(5 + delay), "roll {roll}");
        }
    }

    #[test]
    fn timer_0x393_sends_the_hunters_once_for_a_free_located_han() {
        // FUN_0054e970 -> FUN_00546fa0: chance GNPRTB 6150 (100), Han
        // located and not captured.
        let (world, han, home, palace) = world();
        let mut state = JabbaState {
            armed: true,
            bounty_hunters_active: true,
            bounty_due: Some(40),
        };
        assert_eq!(day(&mut state, &world, 39, &[0.5]), []);
        assert_eq!(
            day(&mut state, &world, 40, &[0.5]),
            [JabbaStep::BountyHunt {
                han,
                palace,
                at: home,
                tick: 40,
            }]
        );
        assert_eq!(state.bounty_due, None);
        assert_eq!(day(&mut state, &world, 41, &[0.5]), []);
    }

    #[test]
    fn no_hunt_for_a_captured_han_or_without_the_palace_or_the_active_flag() {
        let fire = |world: &GameWorld, active: bool| {
            let mut state = JabbaState {
                armed: true,
                bounty_hunters_active: active,
                bounty_due: Some(1),
            };
            day(&mut state, world, 1, &[0.5])
        };
        let (mut world, han, _, palace) = world();
        assert_eq!(fire(&world, false), []);
        world.characters[han].is_captive = true;
        assert_eq!(fire(&world, true), []);
        world.characters[han].is_captive = false;
        world.characters[han].is_killed = true;
        assert_eq!(fire(&world, true), []);
        world.characters[han].is_killed = false;
        world.abodes.remove(palace);
        assert_eq!(fire(&world, true), []);
    }

    #[test]
    fn only_a_captive_of_the_bounty_mission_autoroutes_to_the_palace() {
        // FUN_00543040 maps a Bounty mission destination (family 0x65) to
        // the palace; other captives stay.
        let (mut world, han, home, palace) = world();
        let mut state = JabbaState {
            armed: true,
            ..JabbaState::default()
        };
        world.characters[han].is_captive = true;
        assert_eq!(day(&mut state, &world, 3, &[]), []);
        world.characters[han].bounty_captured = true;
        assert_eq!(
            day(&mut state, &world, 3, &[]),
            [JabbaStep::ToPalace {
                captive: han,
                from: home,
                palace,
                tick: 3,
            }]
        );
        world.characters[han].current_abode = Some(palace);
        world.characters[han].current_system = None;
        assert_eq!(day(&mut state, &world, 4, &[]), []);
    }

    #[test]
    fn the_hunters_take_their_class_s_skills_in_the_palace_and_reveal_han() {
        // FUN_004f7d50 creates the unit for the palace's side;
        // FUN_00556c70(Han, 2) gives the Empire Han's location.
        let (mut world, han, home, palace) = world();
        world.special_force_classes.insert(
            DatId::new(BOUNTY_HUNTERS),
            crate::world::SpecialForceClassDef {
                skills: [crate::world::SkillPair {
                    base: 7,
                    variance: 0,
                }; 8],
                mission_mask: 0x1000,
            },
        );
        let mut missions = MissionState::new();
        let hunt = JabbaStep::BountyHunt {
            han,
            palace,
            at: home,
            tick: 2,
        };
        perform(&hunt, &mut missions, &mut world).expect("a Bounty mission");
        let unit = world.abodes[palace].special_forces[0];
        assert_eq!(world.special_forces[unit].skills, [7; 8]);
        assert!(!world.special_forces[unit].is_alliance);
        assert_eq!(
            world.systems[home].exploration_status,
            crate::dat::ExplorationStatus::Explored
        );
        let mission = &missions.missions()[0];
        assert_eq!(mission.target_character, Some(han));
        assert_eq!(mission.faction, MissionFaction::Empire);
    }

    #[test]
    fn the_checks_draw_two_rolls() {
        // Timer 0x393's delay and the fire's chance.
        assert_eq!(JabbaSystem::roll_budget(), 2);
    }
}
