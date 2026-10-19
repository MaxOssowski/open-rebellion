//! The Final Battle: the mission manager's `FinalBattleReady` path
//! (`ghidra/notes/final-battle.md`).
//!
//! Once a captured Luke, outside Jabba's Palace, knows his heritage and
//! has Force GNPRTB 3078 (60) or more, with Vader and the Emperor free,
//! `FinalBattleReady` holds (`FUN_00544da0`). Vader is then sent to Luke
//! (`FUN_00544a20`, `FUN_0054ac70`), Vader takes Luke to the Emperor
//! (`FUN_00544be0`, `FUN_0054adf0`), and when the three meet the battle is
//! fought once (`FUN_00544fc0`, `FUN_0054b040`).
//!
//! port: the original runs these checks from the mission manager's handlers
//! (`FUN_00549910`, on many events); the port runs them each day, as it does
//! the Dagobah check. `DarthPickup` and the Pickup mission (`0x73`) are in
//! `jabba.rs`.

use serde::{Deserialize, Serialize};

use crate::ids::{AbodeKey, CharacterKey, FleetKey, SystemKey};
use crate::missions::{MissionFaction, MissionKind, MissionMember, MissionRequest, MissionState};
use crate::tick::TickEvent;
use crate::world::GameWorld;

/// Luke's record (`0x32000242`, `FUN_005434b0`).
const LUKE: u32 = 0x242;
/// Vader's record (`0x35000281`, `FUN_00543760`).
const VADER: u32 = 0x281;
/// The Emperor's record (`0x34000280`, `FUN_00543800` -> `FUN_005070d0`).
const EMPEROR: u32 = 0x280;
/// Jabba's Palace (`0x98000481`, `FUN_00544da0`), where `FinalBattleReady`
/// does not hold.
const JABBAS_PALACE: u32 = 0x481;

/// GNPRTB 3078 (`DAT_006bb740`): the Force at which Luke's own
/// `FinalBattleReady` holds (`FUN_0056f7a0`).
const GNPRTB_LUKE_READY_FORCE: u16 = 3078;
/// GNPRTB 3078 as shipped.
const SHIPPED_LUKE_READY_FORCE: i64 = 60;
/// GNPRTB 6151 (`DAT_006bb564`): the Force Luke needs to win
/// (`FUN_0054b040`).
const GNPRTB_LUKE_WINS_FORCE: u16 = 6151;
/// GNPRTB 6151 as shipped.
const SHIPPED_LUKE_WINS_FORCE: i64 = 100;
/// GNPRTB 6152 (`DAT_006bb544`): the base of Luke's injury in defeat
/// (`FUN_0055c650`).
const GNPRTB_DEFEAT_INJURY_BASE: u16 = 6152;
/// GNPRTB 6152 as shipped.
const SHIPPED_DEFEAT_INJURY_BASE: i64 = 1;
/// GNPRTB 6153 (`DAT_006bb590`): the spread of Luke's injury in defeat
/// (`FUN_0055c650`).
const GNPRTB_DEFEAT_INJURY_SPREAD: u16 = 6153;
/// GNPRTB 6153 as shipped.
const SHIPPED_DEFEAT_INJURY_SPREAD: i64 = 199;

/// The mission manager's Final Battle bits (save v76).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalBattleState {
    /// `FinalBattleReady`, manager `+0x58` bit `0x1000`.
    pub ready: bool,
    /// `FinalBattle`, manager `+0x58` bit `0x2000`: the battle was fought.
    pub fought: bool,
}

/// A character's container (`+0x1c`, `FUN_004f6b50`), which the checks
/// compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Fleet(FleetKey),
    System(SystemKey),
    Abode(AbodeKey),
}

/// What a day's checks ask for, in the order `FUN_00549910` runs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalBattleStep {
    /// `DarthToLukeFinalBattle`'s hook (`FUN_0054ac70`): a Move order for
    /// Vader to Luke's location (`FUN_00547340`).
    VaderToLuke {
        vader: CharacterKey,
        to: SystemKey,
        tick: u64,
    },
    /// `DarthToEmperorFinalBattle`'s hook (`FUN_0054adf0`): a Move order for
    /// Vader with the captive Luke to the Emperor's location.
    ToEmperor {
        vader: CharacterKey,
        luke: CharacterKey,
        to: SystemKey,
        tick: u64,
    },
    /// `FinalBattle`'s hook (`FUN_0054b040`).
    Battle(FinalBattle),
}

/// The battle's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Luke's Force is below GNPRTB 6151 (result 2): he stays captive,
    /// loses `CanEscape`, and takes `injury` (`FUN_00549a20`).
    EmpireWins { injury: i32 },
    /// Result 1: Luke goes free (slot `+0x218`) and captures Vader and the
    /// Emperor (slot `+0x20c`), who lose `CanEscape`.
    LukeWins,
}

/// The Final Battle as `FUN_0054b040` fights it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalBattle {
    pub luke: CharacterKey,
    pub vader: CharacterKey,
    pub emperor: CharacterKey,
    /// The system the three's location stands at.
    pub system: Option<SystemKey>,
    /// The location is a system (`0x90..0x97`), which both sides then
    /// observe.
    pub at_system: bool,
    pub outcome: Outcome,
    pub tick: u64,
}

fn find(world: &GameWorld, index: u32) -> Option<CharacterKey> {
    world
        .characters
        .iter()
        .find(|(_, c)| c.dat_id.index() == index && !c.is_killed)
        .map(|(key, _)| key)
}

/// `FUN_004f6b50`: the character's container, a fleet, an abode or a
/// system.
#[must_use]
pub fn place(world: &GameWorld, character: CharacterKey) -> Option<Place> {
    let c = world.characters.get(character)?;
    if let Some(fleet) = c.current_fleet {
        return Some(Place::Fleet(fleet));
    }
    if let Some(abode) = c.current_abode {
        return Some(Place::Abode(abode));
    }
    c.current_system.map(Place::System)
}

/// The system a place stands at. port: a Move order targets a system, so
/// the hooks send their members to the system of the location.
fn place_system(world: &GameWorld, place: Place) -> Option<SystemKey> {
    match place {
        Place::Fleet(fleet) => world.fleets.get(fleet).map(|f| f.location),
        Place::System(system) => Some(system),
        Place::Abode(_) => None,
    }
}

fn param(world: &GameWorld, id: u16, shipped: i64) -> i64 {
    crate::movement::gnprtb_or_shipped(world, id, shipped)
}

/// Luke's own `FinalBattleReady` (`+0xb0` bit 2): `KnowsHeritage` and Force
/// at least GNPRTB 3078 (`FUN_0056f7a0`).
///
/// port: the original stores the bit and sets it whenever Luke's Force
/// (slot `+0x31c`) or `KnowsHeritage` (slot `+0x3ec`) changes, its only
/// inputs; the port derives it.
#[must_use]
pub fn luke_ready(world: &GameWorld, luke: CharacterKey) -> bool {
    world.characters.get(luke).is_some_and(|c| {
        c.heritage_known
            && i64::from(c.force_level())
                >= param(world, GNPRTB_LUKE_READY_FORCE, SHIPPED_LUKE_READY_FORCE)
    })
}

/// `FUN_005439e0`: free, i.e. not captured, not autorouting.
///
/// port: the port has no usable bit (`+0x50` bit 0) or adrift bit (`+0x78`
/// bit `0x40`).
fn free(world: &GameWorld, missions: &MissionState, key: CharacterKey) -> bool {
    world
        .characters
        .get(key)
        .is_some_and(|c| !c.is_captive && !c.is_killed)
        && !missions.is_autorouting(key)
}

/// Stateless system for the Final Battle checks.
pub struct FinalBattleSystem;

impl FinalBattleSystem {
    /// The rolls [`Self::advance`] draws: Luke's injury in defeat.
    #[must_use]
    pub fn roll_budget() -> usize {
        1
    }

    /// The day's checks: `FinalBattleReady` (`FUN_00544da0`), then
    /// `DarthToLuke` (`FUN_00544a20`), `DarthToEmperor` (`FUN_00544be0`) and
    /// `FinalBattle` (`FUN_00544fc0`), with what their hooks ask for.
    pub fn advance(
        state: &mut FinalBattleState,
        world: &GameWorld,
        missions: &MissionState,
        tick_events: &[TickEvent],
        rolls: &[f64],
    ) -> Vec<FinalBattleStep> {
        let Some(tick) = tick_events.last().map(|event| event.tick) else {
            return Vec::new();
        };
        let cast = (find(world, LUKE), find(world, VADER), find(world, EMPEROR));
        let (Some(luke), Some(vader), Some(emperor)) = cast else {
            state.ready = false;
            return Vec::new();
        };
        let places = (
            place(world, luke),
            place(world, vader),
            place(world, emperor),
        );
        let (Some(at_luke), Some(at_vader), Some(at_emperor)) = places else {
            state.ready = false;
            return Vec::new();
        };
        state.ready = !state.fought && Self::ready(world, missions, luke, vader, emperor, at_luke);
        let mut steps = Vec::new();
        if !state.ready {
            return steps;
        }
        // FUN_00544a20: Vader elsewhere than Luke.
        if at_vader != at_luke {
            if let Some(to) = place_system(world, at_luke) {
                steps.push(FinalBattleStep::VaderToLuke { vader, to, tick });
            }
            return steps;
        }
        // FUN_00544be0: Vader with Luke, the Emperor elsewhere. Its hook
        // acts only when both are the Empire's and on no mission.
        if at_vader != at_emperor {
            let empire = |key: CharacterKey| {
                world.characters.get(key).is_some_and(|c| {
                    crate::missions::character_side(c) == Some(crate::dat::Faction::Empire)
                        && !c.on_mission
                })
            };
            if empire(vader) && empire(luke) {
                if let Some(to) = place_system(world, at_emperor) {
                    steps.push(FinalBattleStep::ToEmperor {
                        vader,
                        luke,
                        to,
                        tick,
                    });
                }
            }
            return steps;
        }
        // FUN_00544fc0: the three together. `FinalBattle` is set and
        // `FinalBattleReady` re-run, which clears it.
        state.fought = true;
        state.ready = false;
        let force = i64::from(world.characters[luke].force_level());
        let outcome = if force < param(world, GNPRTB_LUKE_WINS_FORCE, SHIPPED_LUKE_WINS_FORCE) {
            let mut rolls = crate::uprising::Rolls::new(rolls);
            let spread = param(
                world,
                GNPRTB_DEFEAT_INJURY_SPREAD,
                SHIPPED_DEFEAT_INJURY_SPREAD,
            );
            let base = param(world, GNPRTB_DEFEAT_INJURY_BASE, SHIPPED_DEFEAT_INJURY_BASE);
            let drawn =
                crate::uprising::Draws::draw(&mut rolls, i32::try_from(spread).unwrap_or(i32::MAX));
            Outcome::EmpireWins {
                injury: drawn.saturating_add(i32::try_from(base).unwrap_or(0)),
            }
        } else {
            Outcome::LukeWins
        };
        steps.push(FinalBattleStep::Battle(FinalBattle {
            luke,
            vader,
            emperor,
            system: place_system(world, at_luke),
            at_system: matches!(at_luke, Place::System(_)),
            outcome,
            tick,
        }));
        steps
    }

    /// `FUN_00544da0`'s conditions: Vader and the Emperor free; Luke
    /// captured, on no mission, not autorouting and not in Jabba's Palace,
    /// with his own `FinalBattleReady`.
    fn ready(
        world: &GameWorld,
        missions: &MissionState,
        luke: CharacterKey,
        vader: CharacterKey,
        emperor: CharacterKey,
        at_luke: Place,
    ) -> bool {
        let c = &world.characters[luke];
        let in_palace = matches!(at_luke, Place::Abode(abode)
            if world.abodes.get(abode).is_some_and(|a| a.dat_id.index() == JABBAS_PALACE));
        free(world, missions, vader)
            && free(world, missions, emperor)
            && c.is_captive
            && !c.on_mission
            && !missions.is_autorouting(luke)
            && !in_palace
            && luke_ready(world, luke)
    }
}

/// Carry out a step's hook: the Move orders (`FUN_00547340`) or the battle.
/// Returns the Move order's mission id where one was dispatched.
pub fn perform(
    step: &FinalBattleStep,
    missions: &mut MissionState,
    world: &mut GameWorld,
) -> Option<u64> {
    match *step {
        FinalBattleStep::VaderToLuke { vader, to, tick } => move_order(
            missions,
            world,
            MissionFaction::Empire,
            vec![MissionMember::Character(vader)],
            to,
            tick,
        ),
        FinalBattleStep::ToEmperor {
            vader,
            luke,
            to,
            tick,
        } => move_order(
            missions,
            world,
            MissionFaction::Empire,
            vec![
                MissionMember::Character(vader),
                MissionMember::Character(luke),
            ],
            to,
            tick,
        ),
        FinalBattleStep::Battle(battle) => fight(&battle, missions, world),
    }
}

/// A Move order (`0x41000001`) for `members` to `to` (`FUN_00547ba0`).
fn move_order(
    missions: &mut MissionState,
    world: &mut GameWorld,
    faction: MissionFaction,
    members: Vec<MissionMember>,
    to: SystemKey,
    tick: u64,
) -> Option<u64> {
    let request = MissionRequest {
        kind: MissionKind::Move,
        faction,
        team: members,
        decoys: Vec::new(),
        target_system: to,
        target_character: None,
        target_object: None,
        tick,
    };
    missions.dispatch_guarded(request, world).ok()
}

/// `FUN_0054b040`'s outcome:
///
/// - at a system, both sides observe it (`FUN_0050d5a0(6, 1)`, `(6, 2)`;
///   port: the port reveals the system, as for Espionage);
/// - the Empire wins: Luke loses `CanEscape` (`FUN_004eeb10(0)`) and takes
///   his injury (`FUN_00549a20`);
/// - Luke wins: he goes free (`FUN_004ef5b0`), and Vader and the Emperor
///   are captured by his side (slot `+0x20c`) and lose `CanEscape`;
/// - then, when the winner stands where another side holds the place, the
///   three move to the winner's autoroute destination (`FUN_00543040`) by
///   a Move order (`FUN_00547480`).
///
/// port: the winner's freeing also sets Autorouting in the original
/// (`FUN_004f18e0`); the port moves Luke by the hook's Move order only.
/// The three moved are hyp: Luke, Vader and the Emperor (`0x54b5b6`).
fn fight(battle: &FinalBattle, missions: &mut MissionState, world: &mut GameWorld) -> Option<u64> {
    if battle.at_system {
        if let Some(system) = battle.system.and_then(|key| world.systems.get_mut(key)) {
            system.exploration_status = crate::dat::ExplorationStatus::Explored;
        }
    }
    let winner = match battle.outcome {
        Outcome::EmpireWins { injury } => {
            if let Some(c) = world.characters.get_mut(battle.luke) {
                c.injure(injury);
                c.escape_barred = true;
            }
            battle.vader
        }
        Outcome::LukeWins => {
            if let Some(c) = world.characters.get_mut(battle.luke) {
                c.is_captive = false;
                c.captured_by = None;
                c.capture_tick = None;
            }
            if let Some(system) = battle.system {
                for captive in [battle.vader, battle.emperor] {
                    crate::missions::capture_character(
                        world,
                        captive,
                        MissionFaction::Alliance,
                        system,
                    );
                    if let Some(c) = world.characters.get_mut(captive) {
                        c.escape_barred = true;
                    }
                }
            }
            battle.luke
        }
    };
    let side = world
        .characters
        .get(winner)
        .and_then(crate::missions::character_side)?;
    let here = battle.system?;
    let holder = world.systems.get(here).and_then(crate::uprising::holder);
    if holder == Some(side) {
        return None;
    }
    let to = crate::missions::escape_destination(world, winner, here)?;
    let faction = if side == crate::dat::Faction::Alliance {
        MissionFaction::Alliance
    } else {
        MissionFaction::Empire
    };
    move_order(
        missions,
        world,
        faction,
        vec![
            MissionMember::Character(winner),
            MissionMember::Character(battle.luke),
            MissionMember::Character(battle.vader),
            MissionMember::Character(battle.emperor),
        ],
        to,
        battle.tick,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::Faction;
    use crate::ids::DatId;
    use crate::world::{Abode, Character, ControlKind, System};

    fn system(world: &mut GameWorld, name: &str, x: u16, control: ControlKind) -> SystemKey {
        world.systems.insert(System {
            dat_id: DatId::new(0x9000_0000 | u32::try_from(world.systems.len()).unwrap()),
            name: name.into(),
            sector: crate::ids::SectorKey::default(),
            x,
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
            control,
        })
    }

    fn character(world: &mut GameWorld, index: u32, alliance: bool, at: SystemKey) -> CharacterKey {
        world.characters.insert(Character {
            dat_id: DatId::new(index),
            is_alliance: alliance,
            is_empire: !alliance,
            is_major: true,
            recruited: true,
            current_system: Some(at),
            ..Character::default()
        })
    }

    struct Cast {
        world: GameWorld,
        luke: CharacterKey,
        vader: CharacterKey,
        emperor: CharacterKey,
        prison: SystemKey,
        court: SystemKey,
        yavin: SystemKey,
    }

    /// Luke held by the Empire at a prison system, knowing his heritage at
    /// Force 80; Vader at the court with the Emperor; an Alliance system
    /// far away.
    fn cast() -> Cast {
        let mut world = GameWorld::default();
        // The shipped MISSNSD Move record 0x41000001: both sides, character
        // members (target flags 0x10000), hidden, 0 days, own-side targets.
        world.mission_records.push(crate::world::MissionRecord {
            dat_id: DatId::new(0x4100_0001),
            timer_min_days: 0,
            timer_spread_days: 0,
            repeats: false,
            hidden: true,
            detection_phases: true,
            can_resign: false,
            leave_rule: 0,
            rules: crate::world::MissionTargetRules {
                own_side_target: true,
                ..crate::world::MissionTargetRules::default()
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0xf0f,
                character_mask: 0x1_0000,
            },
        });
        let empire = ControlKind::Controlled(Faction::Empire);
        let prison = system(&mut world, "Prison", 10, empire);
        let court = system(&mut world, "Coruscant", 20, empire);
        let yavin = system(
            &mut world,
            "Yavin",
            100,
            ControlKind::Controlled(Faction::Alliance),
        );
        let luke = character(&mut world, LUKE, true, prison);
        let vader = character(&mut world, VADER, false, court);
        let emperor = character(&mut world, EMPEROR, false, court);
        crate::missions::capture_character(&mut world, luke, MissionFaction::Empire, prison);
        let c = &mut world.characters[luke];
        c.heritage_known = true;
        c.force_base = 80;
        Cast {
            world,
            luke,
            vader,
            emperor,
            prison,
            court,
            yavin,
        }
    }

    fn day(state: &mut FinalBattleState, cast: &Cast, roll: f64) -> Vec<FinalBattleStep> {
        FinalBattleSystem::advance(
            state,
            &cast.world,
            &MissionState::new(),
            &[TickEvent { tick: 9 }],
            &[roll],
        )
    }

    #[test]
    fn luke_s_own_ready_needs_his_heritage_and_force_sixty() {
        // FUN_0056f7a0: KnowsHeritage (+0xb0 bit 0) and Force >= GNPRTB
        // 3078 (60).
        let mut cast = cast();
        assert!(luke_ready(&cast.world, cast.luke));
        cast.world.characters[cast.luke].force_base = 59;
        assert!(!luke_ready(&cast.world, cast.luke));
        cast.world.characters[cast.luke].force_base = 60;
        cast.world.characters[cast.luke].heritage_known = false;
        assert!(!luke_ready(&cast.world, cast.luke));
    }

    #[test]
    fn final_battle_ready_needs_a_captured_ready_luke_and_a_free_vader_and_emperor() {
        // FUN_00544da0.
        let mut cast = cast();
        let mut state = FinalBattleState::default();
        day(&mut state, &cast, 0.5);
        assert!(state.ready);

        cast.world.characters[cast.vader].is_captive = true;
        day(&mut state, &cast, 0.5);
        assert!(!state.ready, "Vader held");
        cast.world.characters[cast.vader].is_captive = false;

        cast.world.characters[cast.luke].is_captive = false;
        day(&mut state, &cast, 0.5);
        assert!(!state.ready, "Luke free");
        cast.world.characters[cast.luke].is_captive = true;

        cast.world.characters[cast.luke].on_mission = true;
        day(&mut state, &cast, 0.5);
        assert!(!state.ready, "Luke on a mission");
        cast.world.characters[cast.luke].on_mission = false;

        cast.world.characters[cast.luke].heritage_known = false;
        day(&mut state, &cast, 0.5);
        assert!(!state.ready, "Luke not ready");
    }

    #[test]
    fn a_luke_held_in_jabba_s_palace_is_not_ready() {
        // FUN_00544da0 compares Luke's location with 0x98000481.
        let mut cast = cast();
        let palace = cast.world.abodes.insert(Abode {
            dat_id: DatId::new(JABBAS_PALACE),
            name: "Jabba's Palace".into(),
            side: Faction::Empire,
            special_forces: Vec::new(),
        });
        cast.world.characters[cast.luke].current_abode = Some(palace);
        let mut state = FinalBattleState::default();
        assert_eq!(day(&mut state, &cast, 0.5), []);
        assert!(!state.ready);
    }

    #[test]
    fn vader_is_sent_to_luke_by_a_move_order() {
        // FUN_00544a20 pulses `DarthToLukeFinalBattle` while Vader stands
        // elsewhere; its hook FUN_0054ac70 orders Vader to Luke's location
        // (FUN_00547340, record 0x41000001).
        let mut cast = cast();
        let mut state = FinalBattleState::default();
        let steps = day(&mut state, &cast, 0.5);
        assert_eq!(
            steps,
            [FinalBattleStep::VaderToLuke {
                vader: cast.vader,
                to: cast.prison,
                tick: 9,
            }]
        );
        let mut missions = MissionState::new();
        let id = perform(&steps[0], &mut missions, &mut cast.world).expect("a Move order");
        let mission = missions.missions().iter().find(|m| m.id == id).unwrap();
        assert_eq!(mission.kind, MissionKind::Move);
        assert_eq!(mission.team, [MissionMember::Character(cast.vader)]);
        assert_eq!(mission.target_system, cast.prison);
    }

    #[test]
    fn vader_takes_the_captive_luke_to_the_emperor() {
        // FUN_00544be0 with Vader beside Luke and the Emperor elsewhere; the
        // hook FUN_0054adf0 moves Vader with Luke, the Empire's prisoner, to
        // the Emperor (the prisoner goes to the captured list).
        let mut cast = cast();
        cast.world.characters[cast.vader].current_system = Some(cast.prison);
        let mut state = FinalBattleState::default();
        let steps = day(&mut state, &cast, 0.5);
        assert_eq!(
            steps,
            [FinalBattleStep::ToEmperor {
                vader: cast.vader,
                luke: cast.luke,
                to: cast.court,
                tick: 9,
            }]
        );
        let mut missions = MissionState::new();
        perform(&steps[0], &mut missions, &mut cast.world).expect("a Move order");
        let mission = &missions.missions()[0];
        assert_eq!(mission.team, [MissionMember::Character(cast.vader)]);
        assert_eq!(mission.captured, [MissionMember::Character(cast.luke)]);
        assert_eq!(mission.target_system, cast.court);
    }

    #[test]
    fn the_emperor_hook_needs_both_the_empire_s_and_free_of_missions() {
        // FUN_0054adf0: Vader and Luke on the Empire's side (+0x24 & 0xc0 ==
        // 0x80) and on no mission (+0x78 bit 0x80).
        let mut cast = cast();
        cast.world.characters[cast.vader].current_system = Some(cast.prison);
        cast.world.characters[cast.vader].on_mission = true;
        assert_eq!(day(&mut FinalBattleState::default(), &cast, 0.5), []);
        cast.world.characters[cast.vader].on_mission = false;
        cast.world.characters[cast.luke].captured_by = None;
        assert_eq!(day(&mut FinalBattleState::default(), &cast, 0.5), []);
    }

    #[test]
    fn the_checks_draw_one_roll() {
        // One draw: Luke's injury in defeat (FUN_0055c650).
        assert_eq!(FinalBattleSystem::roll_budget(), 1);
    }

    #[test]
    fn the_battle_is_fought_once_when_the_three_meet() {
        // FUN_00544fc0 sets `FinalBattle` and re-runs `FinalBattleReady`, which
        // `FinalBattle` then keeps clear.
        let mut cast = cast();
        cast.world.characters[cast.luke].current_system = Some(cast.court);
        let mut state = FinalBattleState::default();
        let steps = day(&mut state, &cast, 0.5);
        assert!(matches!(steps.as_slice(), [FinalBattleStep::Battle(_)]));
        assert!(state.fought && !state.ready);
        assert_eq!(day(&mut state, &cast, 0.5), []);
        assert!(!state.ready);
    }

    #[test]
    fn luke_below_force_100_is_injured_and_stays_captive_for_good() {
        // FUN_0054b040: Force < GNPRTB 6151 (100); injury rand(0..=6153
        // (199)) + 6152 (1) (FUN_0055c650); CanEscape cleared
        // (FUN_004eeb10(0)).
        for roll in [0.0, 0.5, 0.999] {
            let mut cast = cast();
            cast.world.characters[cast.luke].current_system = Some(cast.court);
            let steps = day(&mut FinalBattleState::default(), &cast, roll);
            let [FinalBattleStep::Battle(battle)] = steps.as_slice() else {
                panic!("a battle");
            };
            let Outcome::EmpireWins { injury } = battle.outcome else {
                panic!("the Empire wins");
            };
            assert!((1..=200).contains(&injury), "roll {roll}: {injury}");
            let mut missions = MissionState::new();
            assert_eq!(perform(&steps[0], &mut missions, &mut cast.world), None);
            let luke = &cast.world.characters[cast.luke];
            assert!(luke.is_captive && luke.escape_barred);
            assert_eq!(i32::from(luke.injury), injury);
            assert_eq!(
                cast.world.systems[cast.court].exploration_status,
                crate::dat::ExplorationStatus::Explored
            );
        }
    }

    #[test]
    fn luke_at_force_100_wins_captures_both_and_leaves_with_them() {
        // FUN_0054b040: Luke goes free (FUN_004ef5b0), Vader and the
        // Emperor are captured by his side and lose CanEscape; Luke, in a
        // system the Empire holds, takes the three to his autoroute
        // destination by a Move order (FUN_00543040, FUN_00547480).
        let mut cast = cast();
        cast.world.characters[cast.luke].current_system = Some(cast.court);
        cast.world.characters[cast.luke].force_base = 100;
        let steps = day(&mut FinalBattleState::default(), &cast, 0.5);
        let [FinalBattleStep::Battle(battle)] = steps.as_slice() else {
            panic!("a battle");
        };
        assert_eq!(battle.outcome, Outcome::LukeWins);
        let mut missions = MissionState::new();
        perform(&steps[0], &mut missions, &mut cast.world).expect("the move home");
        let luke = &cast.world.characters[cast.luke];
        assert!(!luke.is_captive);
        for captive in [cast.vader, cast.emperor] {
            let c = &cast.world.characters[captive];
            assert!(c.is_captive && c.escape_barred);
            assert_eq!(c.captured_by, Some(Faction::Alliance));
        }
        let mission = &missions.missions()[0];
        assert_eq!(mission.team, [MissionMember::Character(cast.luke)]);
        assert_eq!(
            mission.captured,
            [
                MissionMember::Character(cast.vader),
                MissionMember::Character(cast.emperor)
            ]
        );
        assert_eq!(mission.target_system, cast.yavin);
    }

    #[test]
    fn a_barred_prisoner_gets_no_escape_timer() {
        // CanEscape cleared (FUN_004eeb10(0)) cancels timer 0x38a
        // (FUN_004f1ba0), so the prisoner never rolls ESCAPETB.
        let mut cast = cast();
        let mut missions = MissionState::new();
        crate::missions::MissionSystem::advance_escapes(&mut missions, &cast.world, 1, &[0.5; 4]);
        assert_eq!(missions.escape_timers().len(), 1);
        cast.world.characters[cast.luke].escape_barred = true;
        crate::missions::MissionSystem::advance_escapes(&mut missions, &cast.world, 2, &[0.5; 4]);
        assert_eq!(missions.escape_timers(), []);
    }

    #[test]
    fn a_new_capture_lets_a_barred_prisoner_escape_again() {
        // FUN_004f1a80: a capture sets CanEscape.
        let mut cast = cast();
        cast.world.characters[cast.vader].escape_barred = true;
        crate::missions::capture_character(
            &mut cast.world,
            cast.vader,
            MissionFaction::Alliance,
            cast.yavin,
        );
        assert!(!cast.world.characters[cast.vader].escape_barred);
    }
}
