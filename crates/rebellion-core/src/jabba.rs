//! Jabba's palace: the bounty hunters and the rescues
//! (`ghidra/notes/jabba-palace.md`).
//!
//! The mission manager's setup sets `BountyHuntersActive`, whose hook arms
//! timer `0x393`. When it fires, a Bounty Hunters unit is created in
//! Jabba's Palace and sent after Han on the Bounty mission
//! (`FUN_00546fa0`). A Han the mission captures autoroutes to the palace
//! (`FUN_00543040`). Once he is held there (`HanCapturedAtPalace`), Luke,
//! Leia and Chewbacca each go on a Palace mission to free him
//! (`FUN_00544130`, `FUN_005443f0`, `FUN_005444e0`).
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
/// The rescuers the Palace hooks send, in `FUN_00549910`'s order: Luke
/// (`0x32000242`, `FUN_0054a910`), Leia (`0x31000241`, `FUN_0054aa60`) and
/// Chewbacca (`0x38000343`, `FUN_0054ab10`).
const RESCUERS: [u32; 3] = [0x242, 0x241, 0x343];
/// Luke, Leia, Han and Chewbacca, whom `DarthPickup` waits to see held
/// (`FUN_005445d0`).
const LUKE: u32 = 0x242;
/// Leia (`0x31000241`, `FUN_00543550`).
const LEIA: u32 = 0x241;
/// Chewbacca (`0x38000343`, `FUN_00543690`).
const CHEWBACCA: u32 = 0x343;
/// Vader (`0x35000281`), the Pickup's member (`FUN_0054abc0`).
const VADER: u32 = 0x281;
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
    /// `LukePalace`, `LeiaPalace` or `ChewbaccaPalace` pulses: `rescuer` goes on
    /// a Palace mission (`0x64000044`) from `from` for the captive Han.
    Rescue {
        rescuer: CharacterKey,
        han: CharacterKey,
        from: SystemKey,
        palace: AbodeKey,
        tick: u64,
    },
    /// `DarthPickup` pulses: Vader goes from `from` on the Pickup mission
    /// (`0x73000082`) to collect the palace's prisoners.
    Pickup {
        vader: CharacterKey,
        from: SystemKey,
        palace: AbodeKey,
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

fn find(world: &GameWorld, index: u32) -> Option<CharacterKey> {
    world
        .characters
        .iter()
        .find(|(_, c)| c.dat_id.index() == index && !c.is_killed)
        .map(|(key, _)| key)
}

/// Whether `character` is a prisoner held in Jabba's Palace: one Jabba
/// captured (`FUN_00490340` case 5 compares the captor with `0x98000481`).
#[must_use]
pub fn held_at_palace(world: &GameWorld, character: CharacterKey) -> bool {
    world.characters.get(character).is_some_and(|c| {
        c.is_captive
            && !c.is_killed
            && c.current_abode.is_some()
            && c.current_abode == palace(world)
    })
}

/// `HanCapturedAtPalace` (`FUN_005442f0`, `+0x58` bit `0x40`): Han is
/// captured, on no mission and not adrift (`+0x78 & 0xc0`), not
/// autorouting, and held in Jabba's Palace.
///
/// port: derived each day from its inputs; "usable" (`+0x50` bit 0) has
/// no port field.
#[must_use]
pub fn han_captured_at_palace(world: &GameWorld, missions: &MissionState) -> bool {
    let (Some(han), Some(palace)) = (find(world, HAN), palace(world)) else {
        return false;
    };
    let c = &world.characters[han];
    c.is_captive
        && !c.on_mission
        && !c.on_mandatory_mission
        && !missions.is_autorouting(han)
        && c.current_abode == Some(palace)
}

/// `DarthPickup` (`FUN_005445d0`): Luke, Leia and Han captured,
/// Chewbacca captured or destroyed (`+0x50` bit 3), some prisoner in the
/// palace (`FUN_004f2640(palace, 1, 2)`: of the Empire's side, as a
/// prisoner's side is its captor's), and Vader located, free
/// (`FUN_005439b0`) and on no mission.
///
/// port: when Vader is on another mission than a Pickup, the original
/// asks him to resign it (`FUN_00534640`); the port does not.
fn darth_pickup(
    world: &GameWorld,
    missions: &MissionState,
    palace: AbodeKey,
    tick: u64,
) -> Option<JabbaStep> {
    let any = |index: u32| {
        world
            .characters
            .iter()
            .find(|(_, c)| c.dat_id.index() == index)
            .map(|(_, c)| c)
    };
    let held = |index: u32| any(index).is_some_and(|c| c.is_captive && !c.is_killed);
    if !(held(LUKE) && held(LEIA) && held(HAN)) {
        return None;
    }
    if !any(CHEWBACCA).is_some_and(|c| c.is_killed || c.is_captive) {
        return None;
    }
    let prisoner = world.characters.values().any(|c| {
        c.is_captive
            && !c.is_killed
            && c.current_abode == Some(palace)
            && c.captured_by == Some(crate::dat::Faction::Empire)
    });
    let vader = find(world, VADER)?;
    let c = &world.characters[vader];
    if !prisoner
        || c.is_captive
        || c.on_mission
        || c.on_mandatory_mission
        || missions.is_autorouting(vader)
    {
        return None;
    }
    Some(JabbaStep::Pickup {
        vader,
        from: standing(world, vader)?,
        palace,
        tick,
    })
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
        let han = find(world, HAN);
        let held = han_captured_at_palace(world, missions);
        // FUN_0054a9c0, HanCapturedAtPalace's hook: BountyHuntersActive
        // clears, which cancels timer 0x393. port: the Alliance's
        // observation of Han's location (FUN_00556c70(Han, 1)) is not
        // ported, as an abode has no system to reveal.
        if held {
            state.bounty_hunters_active = false;
            state.bounty_due = None;
        }
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
        // FUN_00544130, FUN_005443f0, FUN_005444e0: with Han held, each
        // rescuer located and free (FUN_005439b0: not captured, not
        // autorouting, not adrift) and on no mission (FUN_005439e0) pulses
        // its flag, whose hook sends it on the Palace mission.
        // port: Luke's other branch, on the Dagobah mission in phase 8
        // (slot +0x1d8), is not read.
        if let Some(han) = han.filter(|_| held) {
            for index in RESCUERS {
                let Some(rescuer) = find(world, index) else {
                    continue;
                };
                let c = &world.characters[rescuer];
                if c.is_captive
                    || c.on_mission
                    || c.on_mandatory_mission
                    || missions.is_autorouting(rescuer)
                {
                    continue;
                }
                if let Some(from) = standing(world, rescuer) {
                    steps.push(JabbaStep::Rescue {
                        rescuer,
                        han,
                        from,
                        palace,
                        tick,
                    });
                }
            }
        }
        steps.extend(darth_pickup(world, missions, palace, tick));
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
        JabbaStep::Rescue {
            rescuer,
            han,
            from,
            palace,
            tick,
        } => {
            // FUN_005479c0: record 0x64000044, target Han, member the
            // rescuer; the mission travels to the palace (FUN_00555b30).
            let request = MissionRequest::single(
                MissionKind::Palace,
                MissionFaction::Alliance,
                rescuer,
                from,
                Some(han),
                tick,
            );
            let id = missions.dispatch_guarded(request, world).ok()?;
            missions.aim_at_abode(id, palace);
            Some(id)
        }
        JabbaStep::Pickup {
            vader,
            from,
            palace,
            tick,
        } => {
            // FUN_0054abc0 -> FUN_005479c0: record 0x73000082, target the
            // palace, member Vader.
            let request = MissionRequest::single(
                MissionKind::Pickup,
                MissionFaction::Empire,
                vader,
                from,
                None,
                tick,
            );
            let id = missions.dispatch_guarded(request, world).ok()?;
            missions.aim_at_abode(id, palace);
            Some(id)
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
            last_arrival: None,
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

    /// Han held in the palace, with Luke, Leia and Chewbacca at `home`.
    fn held_world() -> (
        GameWorld,
        CharacterKey,
        SystemKey,
        AbodeKey,
        [CharacterKey; 3],
    ) {
        let (mut world, han, home, palace) = world();
        let c = &mut world.characters[han];
        c.is_captive = true;
        c.captured_by = Some(Faction::Empire);
        c.current_system = None;
        c.current_abode = Some(palace);
        let rescuers = RESCUERS.map(|index| {
            world.characters.insert(Character {
                dat_id: DatId::new(index),
                is_alliance: true,
                current_system: Some(home),
                ..Character::default()
            })
        });
        (world, han, home, palace, rescuers)
    }

    #[test]
    fn han_captured_at_palace_needs_him_held_there_and_settled() {
        // FUN_005442f0: captured, on no mission and not adrift, not
        // autorouting, located in Jabba's Palace 0x98000481.
        let (mut world, han, home, _, _) = held_world();
        let missions = MissionState::new();
        assert!(han_captured_at_palace(&world, &missions));
        let unsettled: [fn(&mut Character); 4] = [
            |c| c.is_captive = false,
            |c| c.on_mission = true,
            |c| c.on_mandatory_mission = true,
            |c| c.current_abode = None,
        ];
        for edit in unsettled {
            let mut w = world.clone();
            edit(&mut w.characters[han]);
            assert!(!han_captured_at_palace(&w, &missions));
        }
        let mut routing = MissionState::new();
        routing.push_escapee(crate::missions::EscapeeTransit {
            character: han,
            to: home,
            arrival: 9,
            abode: None,
        });
        assert!(!han_captured_at_palace(&world, &routing));
        world.characters[han].is_killed = true;
        assert!(!han_captured_at_palace(&world, &missions));
    }

    #[test]
    fn darth_pickup_waits_for_the_four_held_and_a_free_vader() {
        // FUN_005445d0: Luke, Leia and Han captured, Chewbacca captured or
        // destroyed (+0x50 bit 3), a prisoner held for the Empire in the
        // palace, Vader free (FUN_005439b0) and on no mission.
        let (mut world, han, home, palace, rescuers) = held_world();
        for key in rescuers {
            world.characters[key].is_captive = true;
            world.characters[key].captured_by = Some(Faction::Empire);
        }
        let vader = world.characters.insert(Character {
            dat_id: DatId::new(VADER),
            is_empire: true,
            current_system: Some(home),
            ..Character::default()
        });
        let missions = MissionState::new();
        let pickup = Some(JabbaStep::Pickup {
            vader,
            from: home,
            palace,
            tick: 3,
        });
        assert_eq!(darth_pickup(&world, &missions, palace, 3), pickup);
        // Chewbacca destroyed instead of held.
        let mut w = world.clone();
        w.characters[rescuers[2]].is_captive = false;
        w.characters[rescuers[2]].is_killed = true;
        assert_eq!(darth_pickup(&w, &missions, palace, 3), pickup);
        type Edit = fn(&mut GameWorld, CharacterKey, [CharacterKey; 3], CharacterKey);
        let unready: [Edit; 9] = [
            |w, _, r, _| w.characters[r[0]].is_captive = false,
            |w, _, r, _| w.characters[r[1]].is_captive = false,
            |w, h, _, _| w.characters[h].is_captive = false,
            |w, _, r, _| w.characters[r[2]].is_captive = false,
            |w, h, _, _| w.characters[h].captured_by = Some(Faction::Alliance),
            |w, _, _, v| w.characters[v].is_captive = true,
            |w, _, _, v| w.characters[v].on_mission = true,
            |w, _, _, v| w.characters[v].on_mandatory_mission = true,
            |w, _, _, v| w.characters[v].current_system = None,
        ];
        for (i, edit) in unready.into_iter().enumerate() {
            let mut w = world.clone();
            edit(&mut w, han, rescuers, vader);
            assert_eq!(darth_pickup(&w, &missions, palace, 3), None, "case {i}");
        }
        let mut routing = MissionState::new();
        routing.push_escapee(crate::missions::EscapeeTransit {
            character: vader,
            to: home,
            arrival: 9,
            abode: None,
        });
        assert_eq!(darth_pickup(&world, &routing, palace, 3), None);
        // The day's advance sends it after the rescues (none: all held).
        let mut state = JabbaState {
            armed: true,
            ..JabbaState::default()
        };
        assert_eq!(
            day(&mut state, &world, 3, &[]),
            pickup.into_iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn only_a_prisoner_in_the_palace_is_held_by_jabba() {
        // FUN_00490340 case 5 compares the captor with 0x98000481.
        let (mut world, han, _, _, rescuers) = held_world();
        assert!(held_at_palace(&world, han));
        assert!(!held_at_palace(&world, rescuers[0]));
        world.characters[han].is_killed = true;
        assert!(!held_at_palace(&world, han));
        world.characters[han].is_killed = false;
        world.characters[han].is_captive = false;
        assert!(!held_at_palace(&world, han));
        world.characters[rescuers[0]].is_captive = true;
        assert!(!held_at_palace(&world, rescuers[0]), "held elsewhere");
    }

    #[test]
    fn han_held_at_the_palace_cancels_the_hunters_and_sends_his_friends() {
        // FUN_0054a9c0 clears BountyHuntersActive (timer 0x393 goes);
        // FUN_00544130, FUN_005443f0 and FUN_005444e0 send Luke, Leia and
        // Chewbacca in that order, each free and on no mission.
        let (world, han, home, palace, rescuers) = held_world();
        let mut state = JabbaState {
            armed: true,
            bounty_hunters_active: true,
            bounty_due: Some(50),
        };
        let steps = day(&mut state, &world, 7, &[0.5]);
        assert!(!state.bounty_hunters_active);
        assert_eq!(state.bounty_due, None);
        assert_eq!(
            steps,
            rescuers.map(|rescuer| JabbaStep::Rescue {
                rescuer,
                han,
                from: home,
                palace,
                tick: 7,
            })
        );
        let busy: [fn(&mut Character); 4] = [
            |c| c.is_captive = true,
            |c| c.on_mission = true,
            |c| c.on_mandatory_mission = true,
            |c| c.current_system = None,
        ];
        for edit in busy {
            let mut w = world.clone();
            edit(&mut w.characters[rescuers[1]]);
            let steps = day(&mut state, &w, 8, &[]);
            assert_eq!(steps.len(), 2);
            assert!(!steps.iter().any(
                |s| matches!(s, JabbaStep::Rescue { rescuer, .. } if *rescuer == rescuers[1])
            ));
        }
        let mut routing = MissionState::new();
        routing.push_escapee(crate::missions::EscapeeTransit {
            character: rescuers[0],
            to: home,
            arrival: 9,
            abode: None,
        });
        let steps =
            JabbaSystem::advance(&mut state, &world, &routing, &[TickEvent { tick: 8 }], &[]);
        assert_eq!(steps.len(), 2);
        // No rescue while Han is free.
        let mut free = world.clone();
        free.characters[han].is_captive = false;
        assert_eq!(day(&mut state, &free, 9, &[]), []);
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
