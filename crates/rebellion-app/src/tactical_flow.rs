//! Production tactical-battle entry shared by campaign and test-only transports.

use std::collections::HashMap;

use rebellion_core::dat::Faction;
use rebellion_core::ids::{FleetKey, SystemKey};
use rebellion_core::troop_transport::TroopTransportState;
use rebellion_core::world::{ControlKind, GameWorld};
use rebellion_render::{
    BattleSession, CombatWinner, GameMessage, MessageCategory, MessageLog, MessageRail,
    RailAudience, TacticalState,
};

use crate::GameMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BattleEntry {
    pub system: SystemKey,
    pub attacker: FleetKey,
    pub defender: FleetKey,
    pub player_is_attacker: bool,
    pub tick: u64,
    pub rng_seed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattleEntryError {
    MissingSystem,
    SameFleet,
    MissingFleet,
    NotOrbitingSystem,
    SameFaction,
    MultipleDeathStars,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BattleReturn {
    pub winner: Option<CombatWinner>,
    pub winner_fleet: Option<FleetKey>,
    pub player_won: bool,
}

/// Compact strategic-world projection used to prove that a played tactical
/// result survives the transition back to the campaign. The fields are all
/// derived from the two source fleets named by the battle session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BattlePersistenceSnapshot {
    pub attacker_present: bool,
    pub defender_present: bool,
    pub attacker_capital_ships: usize,
    pub defender_capital_ships: usize,
    pub attacker_fighter_squadrons: u32,
    pub defender_fighter_squadrons: u32,
    pub attacker_has_death_star: bool,
    pub defender_has_death_star: bool,
}

/// Read the strategic state owned by a tactical session without mutating it.
pub(crate) fn persistence_snapshot(
    session: &BattleSession,
    world: &GameWorld,
) -> BattlePersistenceSnapshot {
    let fleet = |key| {
        world.fleets.get(key).map(|fleet| {
            (
                fleet.capital_ships.len(),
                fleet
                    .fighters
                    .iter()
                    .map(|fighter| fighter.count)
                    .sum::<u32>(),
                fleet.has_death_star,
            )
        })
    };
    let attacker = fleet(session.attacker_fleet);
    let defender = fleet(session.defender_fleet);
    BattlePersistenceSnapshot {
        attacker_present: attacker.is_some(),
        defender_present: defender.is_some(),
        attacker_capital_ships: attacker.map_or(0, |value| value.0),
        defender_capital_ships: defender.map_or(0, |value| value.0),
        attacker_fighter_squadrons: attacker.map_or(0, |value| value.1),
        defender_fighter_squadrons: defender.map_or(0, |value| value.1),
        attacker_has_death_star: attacker.is_some_and(|value| value.2),
        defender_has_death_star: defender.is_some_and(|value| value.2),
    }
}

/// Summarize a completed tactical session without mutating strategic state.
/// Auto-resolve uses this after its result has already been integrated.
pub(crate) fn summarize_results(session: &BattleSession) -> BattleReturn {
    BattleReturn {
        winner: session.winner,
        winner_fleet: match session.winner {
            Some(CombatWinner::Attacker) => Some(session.attacker_fleet),
            Some(CombatWinner::Defender) => Some(session.defender_fleet),
            Some(CombatWinner::Draw) | None => None,
        },
        player_won: match session.winner {
            Some(CombatWinner::Attacker) => session.player_is_attacker,
            Some(CombatWinner::Defender) => !session.player_is_attacker,
            Some(CombatWinner::Draw) | None => false,
        },
    }
}

/// Enter the actual tactical scene after validating the system and both fleets.
/// No state is changed when the request is invalid.
pub(crate) fn begin_player_battle(
    world: &GameWorld,
    entry: BattleEntry,
    tactical: &mut TacticalState,
    cooldowns: &mut HashMap<SystemKey, u64>,
    messages: &mut MessageLog,
    game_mode: &mut GameMode,
) -> Result<(), BattleEntryError> {
    let system = world
        .systems
        .get(entry.system)
        .ok_or(BattleEntryError::MissingSystem)?;
    if entry.attacker == entry.defender {
        return Err(BattleEntryError::SameFleet);
    }
    let attacker = world
        .fleets
        .get(entry.attacker)
        .ok_or(BattleEntryError::MissingFleet)?;
    let defender = world
        .fleets
        .get(entry.defender)
        .ok_or(BattleEntryError::MissingFleet)?;
    if attacker.location != entry.system
        || defender.location != entry.system
        || !system.fleets.contains(&entry.attacker)
        || !system.fleets.contains(&entry.defender)
    {
        return Err(BattleEntryError::NotOrbitingSystem);
    }
    if attacker.is_alliance == defender.is_alliance {
        return Err(BattleEntryError::SameFaction);
    }
    if attacker.has_death_star && defender.has_death_star {
        return Err(BattleEntryError::MultipleDeathStars);
    }

    tactical.begin_battle(
        world,
        entry.system,
        entry.attacker,
        entry.defender,
        entry.player_is_attacker,
        entry.tick,
        entry.rng_seed,
    );
    cooldowns.insert(entry.system, entry.tick);
    messages.push(GameMessage::at_system(
        entry.tick,
        format!("Battle at {} — entering tactical combat!", system.name),
        MessageCategory::Combat,
        entry.system,
    ));
    *game_mode = GameMode::TacticalCombat;
    Ok(())
}

/// Apply the production tactical session back to the strategic world.
/// Campaign entry and the direct test launcher share this exact identity and
/// loss transport instead of maintaining fixture-specific return logic.
pub(crate) fn apply_results(
    session: &BattleSession,
    world: &mut GameWorld,
    troop_transport: &mut TroopTransportState,
) -> BattleReturn {
    let battle_before = rebellion_core::loyalty::battle_watch(world, session.system);
    let attacker_is_alliance = world
        .fleets
        .get(session.attacker_fleet)
        .map(|fleet| fleet.is_alliance);
    // Every fleet that fought: the two that named the battle, then each
    // other fleet of either side whose units took part (FUN_0040c780).
    let mut fought = vec![session.attacker_fleet, session.defender_fleet];
    for fleet in session
        .ships
        .iter()
        .map(|ship| ship.fleet)
        .chain(session.fighters.iter().map(|fighter| fighter.fleet))
    {
        if !fought.contains(&fleet) {
            fought.push(fleet);
        }
    }
    for &fleet_key in &fought {
        let is_attacker = world
            .fleets
            .get(fleet_key)
            .is_some_and(|fleet| fleet.is_alliance == session.attacker_is_alliance);
        if let Some(fleet) = world.fleets.get_mut(fleet_key) {
            if fleet.has_death_star {
                fleet.has_death_star = session.death_star.is_some_and(|death_star| {
                    death_star.is_attacker == is_attacker && !death_star.destroyed
                });
            }

            for (roster_index, ship_inst) in fleet.capital_ships.iter_mut().enumerate() {
                if !ship_inst.alive {
                    continue;
                }
                if let Some(result) = session
                    .ships
                    .iter()
                    .find(|ship| ship.fleet == fleet_key && ship.fleet_ship_index == roster_index)
                {
                    ship_inst.hull_current = result.hull_current.clamp(0, result.hull_max);
                    // FUN_0040cfc0: the hit counters come back with the hull.
                    if let Some(class) = world.capital_ship_classes.get(ship_inst.class) {
                        ship_inst.subsystem_hits = result.strategic_hits(class);
                    }
                    // A ship that escaped (state 9, FUN_005af910) left the
                    // field but is not destroyed.
                    ship_inst.alive =
                        (result.alive || result.retreated) && ship_inst.hull_current > 0;
                    if !ship_inst.alive {
                        ship_inst.hull_current = 0;
                    }
                }
            }

        }
    }
    commit_squadrons(session, world, &fought);
    let mut stranded = Vec::new();
    for fleet_key in fought {
        // FUN_0040cfc0 -> FUN_00501ae0 (reason 0xc): each lost ship's
        // characters go adrift, then the ship leaves its fleet.
        stranded.extend(rebellion_core::missions::strand_lost_ships_passengers(
            world, fleet_key,
        ));
        if let Some(fleet) = world.fleets.get_mut(fleet_key) {
            fleet.capital_ships.retain(|ship| ship.alive);
        }
        let is_empty = world
            .fleets
            .get(fleet_key)
            .is_none_or(rebellion_core::world::Fleet::is_empty);
        if is_empty {
            // FUN_00556710 -> FUN_004ef2e0: the destroyed fleet's characters
            // go adrift where it stood (`missions::cast_adrift`).
            rebellion_core::missions::cast_adrift(world, fleet_key);
            if let Some(fleet) = world.fleets.get(fleet_key) {
                let location = fleet.location;
                if let Some(system) = world.systems.get_mut(location) {
                    system.fleets.retain(|&key| key != fleet_key);
                }
            }
            world.fleets.remove(fleet_key);
        }
    }
    // CapShip_vt11c's injury step for the characters the losses stranded:
    // at most four draws each (chance, two spreads, the death roll). port:
    // they continue the battle's rand stream (FUN_0061a310) as fractions.
    let mut rng = session.tactical_rng;
    let rolls: Vec<f64> = (0..4 * stranded.len())
        .map(|_| f64::from(rng.next_u15()) / 32_768.0)
        .collect();
    rebellion_core::missions::injure_passengers(world, &stranded, &rolls);
    troop_transport.destroy_untransportable_cargo(world);
    // FUN_00517270 -> FUN_00560d00: the winner's loyalty shift.
    let winner = attacker_is_alliance.and_then(|attacker| match session.winner {
        Some(CombatWinner::Attacker) => Some(Faction::of_alliance(attacker)),
        Some(CombatWinner::Defender) => Some(Faction::of_alliance(!attacker)),
        Some(CombatWinner::Draw) | None => None,
    });
    rebellion_core::loyalty::settle_battle(world, battle_before, session.system, winner);
    summarize_results(session)
}

/// `FUN_0040c780`'s commit of the squadrons: each one's count
/// (`FUN_0040d260`), then its parent (`FUN_0040d320`).
/// - A squadron docked on a carrier (`+0x170` 5) rides on it (`+0x164`,
///   set by the dock, `FUN_005b8570`).
/// - Any other survivor, in its hangar or still in the field, rides on the
///   carrier it launched from (`FUN_005b8c70`). A0 VM 2026-10-28, Battle at
///   Mon Calamari from the clean `resource turn 14` save: Red Group's seven
///   TIE squadrons flew around the Imperial Star Destroyer up to the last
///   frame, and afterwards six were aboard it and one aboard Victory
///   Destroyer 2, as before the battle (evidence
///   2026-10-28-squadron-parent-after-battle). The static reading of
///   `FUN_0040d320` would send them to the system; whatever docks them
///   first is not traced.
/// - A squadron whose carrier was lost dies with it when it was inside
///   (`FUN_005b2d20` -> `FUN_005b8d80`, the `GameObj_vt11c` cascade).
///   port: one in the field stays in its fleet on no ship, and
///   `number_ships` seats it; no clean run has shown this case.
/// - A planet's squadron (no carrier) stays in its sortie's fleet, and
///   `squadrons::recover` lands it again.
fn commit_squadrons(session: &BattleSession, world: &mut GameWorld, fought: &[FleetKey]) {
    let classes: Vec<Vec<(rebellion_core::ids::FighterKey, Option<u64>)>> = fought
        .iter()
        .map(|&key| {
            world
                .fleets
                .get(key)
                .map(|fleet| {
                    fleet
                        .fighters
                        .iter()
                        .map(|entry| (entry.class, entry.carrier))
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();
    for &key in fought {
        if let Some(fleet) = world.fleets.get_mut(key) {
            fleet.fighters.clear();
        }
    }
    let ship_id = |world: &GameWorld, index: usize| {
        let ship = session.ships.get(index)?;
        let id = world
            .fleets
            .get(ship.fleet)?
            .capital_ships
            .get(ship.fleet_ship_index)?
            .id;
        Some((ship.fleet, id))
    };
    for fighter in &session.fighters {
        let Some(position) = fought.iter().position(|&key| key == fighter.fleet) else {
            continue;
        };
        let Some(&(class, launched_from)) = classes[position].get(fighter.fleet_fighter_index)
        else {
            continue;
        };
        if fighter.squad_count == 0 {
            continue;
        }
        let docked = fighter.recovery_state
            == rebellion_core::tactical::TacticalFighterRecoveryState::Recovered;
        let (fleet_key, carrier) = match fighter.recovery_target.filter(|_| docked) {
            Some(index) => match ship_id(world, index) {
                Some((fleet, id)) => (fleet, Some(id)),
                None => continue,
            },
            None => (fighter.fleet, launched_from),
        };
        let Some(fleet) = world.fleets.get_mut(fleet_key) else {
            continue;
        };
        let alive = |id: u64| {
            fleet
                .capital_ships
                .iter()
                .any(|ship| ship.id == id && ship.alive)
        };
        match carrier {
            Some(id) if alive(id) => {
                rebellion_core::squadrons::put(&mut fleet.fighters, class, 1, Some(id));
            }
            Some(_) if docked || fighter.in_hangar => {}
            _ => rebellion_core::squadrons::put(&mut fleet.fighters, class, 1, None),
        }
    }
}

/// A planet with regiments takes the holder the economy gives it
/// (`rebellion_core::economy::system_holder`, `FUN_0050b610`, which the
/// original runs as soon as a system's regiments change, `FUN_005109f0`):
/// one side's regiments alone take it, two sides' keep the previous side.
/// The original fights on the ground only in a Planetary Assault
/// (`ghidra/notes/planetary-assault.md`). Carried regiments stay aboard
/// until an order moves them (`ghidra/notes/regiment-unload.md`). The
/// simulation's step 3b (`rebellion_data::simulation`) does the same.
pub(crate) fn occupy(world: &mut GameWorld, tick: u64, log: &mut MessageLog) {
    let systems: Vec<SystemKey> = world.systems.keys().collect();
    for system in systems {
        let Some(value) = world.systems.get(system) else {
            continue;
        };
        if value.ground_units.is_empty() {
            continue;
        }
        if let ControlKind::Controlled(winner) =
            rebellion_core::economy::system_holder(world, value)
        {
            if value.control != ControlKind::Controlled(winner) {
                apply_system_occupation(world, system, winner, tick, log);
            }
        }
    }
}

pub(crate) fn apply_system_occupation(
    world: &mut GameWorld,
    system: SystemKey,
    winner: Faction,
    tick: u64,
    log: &mut MessageLog,
) {
    let previous = world.systems.get(system).map(|value| value.control);
    if let Some(value) = world.systems.get_mut(system) {
        value.control = ControlKind::Controlled(winner);
    }
    // FUN_00510d70 → FUN_004f6f40: the system's facilities follow its new
    // holder, as the headless integrator's occupation does.
    world.hand_over_facilities(system);
    if previous != Some(ControlKind::Controlled(winner)) {
        let name = world
            .systems
            .get(system)
            .map_or("unknown", |value| value.name.as_str());
        log.push(
            GameMessage::at_system(
                tick,
                format!("{name} occupied by {winner:?}"),
                MessageCategory::Combat,
                system,
            )
            .on_rail(MessageRail::PopularSupport, RailAudience::Both),
        );
    }

    for (_, character) in &mut world.characters {
        let is_enemy = match winner {
            Faction::Alliance => character.is_empire,
            Faction::Empire => character.is_alliance,
            Faction::Neutral => false,
        };
        if character.current_system == Some(system) && is_enemy && !character.is_killed {
            character.is_captive = true;
            character.captured_by = Some(winner);
            character.capture_tick = Some(tick);
            character.current_fleet = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rebellion_core::dat::{ExplorationStatus, SectorGroup};
    use rebellion_core::ids::DatId;
    use rebellion_core::world::{
        CapitalShipClass, Character, ControlKind, Fleet, Sector, ShipInstance, System, TroopUnit,
    };

    fn opposing_empty_fleets() -> (GameWorld, SystemKey, FleetKey, FleetKey) {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(1),
            name: "Test Sector".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let system = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(2),
            name: "Test System".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
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
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Uncontrolled,
        });
        let make_fleet = |is_alliance| Fleet {
            location: system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance,
            has_death_star: false,
            special_forces: Vec::new(),
        };
        let attacker = world.fleets.insert(make_fleet(true));
        let defender = world.fleets.insert(make_fleet(false));
        world.systems[system].fleets = vec![attacker, defender];
        (world, system, attacker, defender)
    }

    /// An Alliance transport at a system of `control` carrying one
    /// regiment (not held), and with `defender` an Imperial regiment on
    /// the planet.
    fn transport_fixture(
        control: ControlKind,
        defender: bool,
    ) -> (
        GameWorld,
        SystemKey,
        FleetKey,
        rebellion_core::ids::TroopKey,
        TroopTransportState,
    ) {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(11),
            name: "Ground Test Sector".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let system = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(12),
            name: "Ground Test System".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
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
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control,
        });
        let ship_class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(13),
            name: "Transport".into(),
            is_alliance: true,
            hull: 100,
            troop_capacity: 1,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![ShipInstance::new(ship_class, 100, true)],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[system].fleets.push(fleet);
        let attacker = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(14),
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(attacker);
        if defender {
            let defender = world.troops.insert(TroopUnit {
                class_dat_id: DatId::new(15),
                is_alliance: false,
                regiment_strength: 100,
            });
            world.systems[system].ground_units.push(defender);
        }
        let mut transport = TroopTransportState::default();
        transport
            .embark(&mut world, fleet, &[attacker])
            .expect("fixture troop embarks");
        (world, system, fleet, attacker, transport)
    }

    #[test]
    fn an_occupation_hands_the_system_s_facilities_to_the_new_holder() {
        // FUN_00510d70 -> FUN_004f6f40: a change of holder passes each
        // facility its class serves to the new side (FUN_004f8680).
        let (mut world, system, _, _, _) =
            transport_fixture(ControlKind::Controlled(Faction::Empire), false);
        let class = DatId::new(0x2800_0001);
        world.buildable_classes.insert(
            class,
            rebellion_core::world::BuildableClass {
                is_alliance: true,
                is_empire: true,
                ..rebellion_core::world::BuildableClass::default()
            },
        );
        let yard = world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: class,
                side: Faction::Empire,
                is_shipyard: true,
            },
        );
        world.systems[system].manufacturing_facilities.push(yard);
        apply_system_occupation(
            &mut world,
            system,
            Faction::Alliance,
            43,
            &mut MessageLog::default(),
        );
        assert_eq!(world.manufacturing_facilities[yard].side, Faction::Alliance);
    }

    #[test]
    fn carried_regiments_stay_aboard_without_an_order() {
        // Nothing in the original lands cargo without an order
        // (ghidra/notes/regiment-unload.md, "What happens to the rest of the
        // cargo"); the port's automatic landing went on 2026-10-17. Over the
        // side's planet, another side's and a neutral one the regiment stays
        // aboard, and no planet changes hands.
        for control in [
            ControlKind::Controlled(Faction::Alliance),
            ControlKind::Controlled(Faction::Empire),
            ControlKind::Uncontrolled,
        ] {
            let (mut world, system, fleet, attacker, transport) = transport_fixture(control, false);
            let mut messages = MessageLog::default();
            occupy(&mut world, 41, &mut messages);
            assert_eq!(transport.cargo(fleet), &[attacker]);
            assert!(!world.systems[system].ground_units.contains(&attacker));
            assert_eq!(world.systems[system].control, control);
            assert!(messages.messages().is_empty());
        }
    }

    #[test]
    fn one_side_s_regiments_alone_take_the_planet_and_two_sides_stand_together() {
        // No ground battle outside a Planetary Assault
        // (ghidra/notes/planetary-assault.md); port: one side's regiments
        // alone occupy the planet.
        let (mut world, system, _, _, _) =
            transport_fixture(ControlKind::Controlled(Faction::Empire), true);
        occupy(&mut world, 42, &mut MessageLog::default());
        assert_eq!(
            world.systems[system].control,
            ControlKind::Controlled(Faction::Empire)
        );
        assert_eq!(world.systems[system].ground_units.len(), 1);
        // Both sides' regiments on the planet (the Alliance's landed by its
        // order): no change, no casualties.
        let (mut world, system, fleet, attacker, mut transport) =
            transport_fixture(ControlKind::Controlled(Faction::Alliance), true);
        transport
            .disembark_selected(&mut world, fleet, system, &[attacker])
            .expect("the regiment lands");
        occupy(&mut world, 42, &mut MessageLog::default());
        assert_eq!(world.systems[system].ground_units.len(), 2);
        assert!(world.troops.contains_key(attacker));
        assert_eq!(
            world.systems[system].control,
            ControlKind::Controlled(Faction::Alliance)
        );
        // The Alliance's regiment alone on an Empire planet takes it.
        let (mut world, system, fleet, attacker, mut transport) =
            transport_fixture(ControlKind::Controlled(Faction::Empire), false);
        transport
            .disembark_selected(&mut world, fleet, system, &[attacker])
            .expect("the regiment lands");
        let mut messages = MessageLog::default();
        occupy(&mut world, 43, &mut messages);
        assert_eq!(
            world.systems[system].control,
            ControlKind::Controlled(Faction::Alliance)
        );
        assert!(messages.messages().iter().any(|message| {
            message.rail == Some(MessageRail::PopularSupport)
                && message.text.contains("occupied by Alliance")
        }));
        // The Empire's regiment alone on an Alliance planet takes it, the
        // Alliance's still aboard not counting.
        let (mut world, system, fleet, _, transport) =
            transport_fixture(ControlKind::Controlled(Faction::Alliance), true);
        assert_eq!(transport.carried_count(fleet), 1);
        occupy(&mut world, 44, &mut MessageLog::default());
        assert_eq!(
            world.systems[system].control,
            ControlKind::Controlled(Faction::Empire)
        );
        // FUN_0050b610 counts every regiment (FUN_00504cc0), one at
        // strength 0 too: with both sides' the holder stays. (The port's
        // occupation skipped it until 2026-10-17.)
        let (mut world, system, _, _, _) =
            transport_fixture(ControlKind::Controlled(Faction::Alliance), true);
        let spent = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(14),
            is_alliance: true,
            regiment_strength: 0,
        });
        world.systems[system].ground_units.push(spent);
        occupy(&mut world, 45, &mut MessageLog::default());
        assert_eq!(
            world.systems[system].control,
            ControlKind::Controlled(Faction::Alliance)
        );
    }

    #[test]
    fn both_player_sides_enter_the_same_production_tactical_path() {
        let (world, system, attacker, defender) = opposing_empty_fleets();
        for player_is_attacker in [true, false] {
            let mut tactical = TacticalState::default();
            let mut cooldowns = HashMap::new();
            let mut messages = MessageLog::default();
            let mut mode = GameMode::Galaxy;
            begin_player_battle(
                &world,
                BattleEntry {
                    system,
                    attacker,
                    defender,
                    player_is_attacker,
                    tick: 17,
                    rng_seed: 0x1111_0001,
                },
                &mut tactical,
                &mut cooldowns,
                &mut messages,
                &mut mode,
            )
            .unwrap();
            assert_eq!(mode, GameMode::TacticalCombat);
            assert_eq!(cooldowns.get(&system), Some(&17));
            let session = tactical.session.as_ref().unwrap();
            assert_eq!(session.attacker_fleet, attacker);
            assert_eq!(session.defender_fleet, defender);
            assert_eq!(session.player_is_attacker, player_is_attacker);
            assert_eq!(session.phase, rebellion_render::BattlePhase::Combat);
            assert!(session.paused, "the original Tactical Display opens paused");
        }
    }

    #[test]
    fn every_fleet_at_the_system_fights_and_takes_its_own_result() {
        // FUN_0040c780 applies a battle's result to every fleet at the
        // system (FUN_004ffe70), so each fleet of both sides fights; the
        // headless campaign's resolve_system_space_combat does the same.
        let (mut world, system, attacker, defender) = opposing_empty_fleets();
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x1400_0001),
            name: "Corvette".into(),
            hull: 100,
            ..Default::default()
        });
        for fleet in [attacker, defender] {
            let is_alliance = world.fleets[fleet].is_alliance;
            world.fleets[fleet]
                .capital_ships
                .push(ShipInstance::new(class, 100, is_alliance));
        }
        let reserve = world.fleets.insert(world.fleets[attacker].clone());
        world.systems[system].fleets.push(reserve);
        let mut tactical = TacticalState::default();
        begin_player_battle(
            &world,
            BattleEntry {
                system,
                attacker,
                defender,
                player_is_attacker: true,
                tick: 17,
                rng_seed: 0x1111_0005,
            },
            &mut tactical,
            &mut HashMap::new(),
            &mut MessageLog::default(),
            &mut GameMode::Galaxy,
        )
        .unwrap();
        let mut session = tactical.session.clone().unwrap();
        let fleets: Vec<FleetKey> = session.ships.iter().map(|ship| ship.fleet).collect();
        assert_eq!(fleets, [attacker, reserve, defender]);

        // The reserve's ship is lost; the lead fleet's is hit.
        session.ships[1].alive = false;
        session.ships[1].hull_current = 0;
        session.ships[0].hull_current = 40;
        session.winner = Some(CombatWinner::Attacker);
        apply_results(&session, &mut world, &mut TroopTransportState::default());

        assert!(!world.fleets.contains_key(reserve));
        assert!(!world.systems[system].fleets.contains(&reserve));
        assert_eq!(world.fleets[attacker].capital_ships[0].hull_current, 40);
        assert_eq!(world.fleets[defender].capital_ships[0].hull_current, 100);
    }

    #[test]
    fn a_ship_that_escaped_the_battle_keeps_its_hull() {
        // FUN_005d14d0 step 5: a capital ship with a working hyperdrive
        // escapes (FUN_005af910, state 9); it is not destroyed, as in the
        // abstract battle (`AbstractBattle::retreat`).
        let (mut world, system, attacker, defender) = opposing_empty_fleets();
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x1400_0001),
            name: "Corvette".into(),
            hull: 100,
            ..Default::default()
        });
        for fleet in [attacker, defender] {
            let is_alliance = world.fleets[fleet].is_alliance;
            world.fleets[fleet]
                .capital_ships
                .push(ShipInstance::new(class, 100, is_alliance));
        }
        let mut session =
            BattleSession::new(&world, system, attacker, defender, true, 23, 0x1111_0006);
        // The tactical battle's leave: off the field, no longer alive there.
        session.ships[0].hull_current = 80;
        session.ships[0].alive = false;
        session.ships[0].retreated = true;
        session.winner = Some(CombatWinner::Defender);

        apply_results(&session, &mut world, &mut TroopTransportState::default());

        assert!(world.fleets.contains_key(attacker));
        let ship = &world.fleets[attacker].capital_ships[0];
        assert!(ship.alive);
        assert_eq!(ship.hull_current, 80);
    }

    #[test]
    fn a_ship_lost_in_the_battle_sets_its_characters_adrift() {
        // FUN_0040cfc0 -> FUN_00501ae0 (reason 0xc) destroys the lost ship;
        // its GameObj_vt11c (FUN_004fb520) sets the characters aboard
        // Adrift (FUN_004ef2e0). The fleet keeps the other ship's.
        let (mut world, system, attacker, defender) = opposing_empty_fleets();
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x1400_0001),
            name: "Corvette".into(),
            hull: 100,
            ..Default::default()
        });
        world.fleets[attacker]
            .capital_ships
            .push(ShipInstance::new(class, 100, true));
        for _ in 0..2 {
            world.fleets[defender]
                .capital_ships
                .push(ShipInstance::new(class, 100, false));
        }
        world.number_ships();
        let ids: Vec<u64> = world.fleets[defender]
            .capital_ships
            .iter()
            .map(|ship| ship.id)
            .collect();
        let officers: Vec<_> = ids
            .iter()
            .map(|&ship| {
                let key = world.characters.insert(Character {
                    is_empire: true,
                    current_system: Some(system),
                    current_fleet: Some(defender),
                    ship: Some(ship),
                    ..Character::default()
                });
                world.fleets[defender].characters.push(key);
                key
            })
            .collect();
        let mut session =
            BattleSession::new(&world, system, attacker, defender, true, 23, 0x1111_0007);
        let lost = session
            .ships
            .iter_mut()
            .find(|ship| ship.fleet == defender && ship.fleet_ship_index == 0)
            .unwrap();
        lost.hull_current = 0;
        lost.alive = false;
        session.winner = Some(CombatWinner::Attacker);

        apply_results(&session, &mut world, &mut TroopTransportState::default());

        assert_eq!(world.fleets[defender].capital_ships.len(), 1);
        assert_eq!(world.fleets[defender].characters, [officers[1]]);
        let officer = &world.characters[officers[0]];
        assert!(officer.adrift);
        assert_eq!((officer.current_fleet, officer.current_system), (None, Some(system)));
        assert!(!world.characters[officers[1]].adrift);
    }

    #[test]
    fn each_character_of_the_lost_ships_draws_its_own_injury() {
        // CapShip_vt11c's injury step (FUN_004ef640 -> FUN_0053ea00) for
        // three major officers of combat 0, three draws each: chance 100, injury rand(100) +
        // rand(GNPRTB 2570 (29)) + 1, at most 130. Each draws from the
        // battle's stream; a short or unscaled stream would give the
        // highest draws, 130.
        let (mut world, system, attacker, defender) = opposing_empty_fleets();
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x1400_0001),
            name: "Corvette".into(),
            hull: 100,
            ..Default::default()
        });
        world.fleets[attacker]
            .capital_ships
            .push(ShipInstance::new(class, 100, true));
        world.fleets[defender]
            .capital_ships
            .push(ShipInstance::new(class, 100, false));
        world.number_ships();
        let ship = world.fleets[defender].capital_ships[0].id;
        let officers: Vec<_> = (0..3)
            .map(|_| {
                let key = world.characters.insert(Character {
                    is_empire: true,
                    is_major: true,
                    current_system: Some(system),
                    current_fleet: Some(defender),
                    ship: Some(ship),
                    ..Character::default()
                });
                world.fleets[defender].characters.push(key);
                key
            })
            .collect();
        let mut session =
            BattleSession::new(&world, system, attacker, defender, true, 23, 0x1111_0008);
        let lost = session
            .ships
            .iter_mut()
            .find(|ship| ship.fleet == defender)
            .unwrap();
        lost.hull_current = 0;
        lost.alive = false;
        session.winner = Some(CombatWinner::Attacker);

        apply_results(&session, &mut world, &mut TroopTransportState::default());

        for officer in officers {
            let injury = world.characters[officer].injury;
            assert!((1..130).contains(&injury), "{injury}");
        }
    }

    #[test]
    fn a_surviving_squadron_rides_on_its_carrier_and_a_lost_carriers_dies_inside_it() {
        // A0 VM 2026-10-28 (Battle at Mon Calamari): squadrons still in the
        // field at the end were back aboard the carriers they launched
        // from. A docked one rides on the carrier it docked on (+0x164,
        // FUN_005b8570); a lost carrier's hangar dies with it
        // (FUN_005b2d20 -> FUN_005b8d80). port: one it launched stays in
        // the fleet on no ship. The planet takes none.
        let (mut world, system, attacker, defender) = opposing_empty_fleets();
        let carrier = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x1400_0001),
            name: "Carrier".into(),
            hull: 100,
            fighter_capacity: 2,
            ..Default::default()
        });
        let wing = world.fighter_classes.insert(rebellion_core::world::FighterClass {
            dat_id: DatId::new(0x1c00_0001),
            is_alliance: true,
            squadron_size: 12,
            sub_light_engine: 10,
            ..Default::default()
        });
        let other = world.fighter_classes.insert(rebellion_core::world::FighterClass {
            dat_id: DatId::new(0x1c00_0002),
            is_alliance: true,
            squadron_size: 12,
            sub_light_engine: 10,
            ..Default::default()
        });
        for _ in 0..2 {
            world.fleets[attacker]
                .capital_ships
                .push(ShipInstance::new(carrier, 100, true));
        }
        world.fleets[defender]
            .capital_ships
            .push(ShipInstance::new(carrier, 100, false));
        world.number_ships();
        let id = world.fleets[attacker].capital_ships[0].id;
        let kept = world.fleets[attacker].capital_ships[1].id;
        for (class, count, carrier) in [(wing, 2, id), (other, 1, id), (wing, 1, kept)] {
            world.fleets[attacker].fighters.push(rebellion_core::world::FighterEntry {
                class,
                count,
                carrier: Some(carrier),
            });
        }
        let mut session =
            BattleSession::new(&world, system, attacker, defender, true, 23, 0x1111_0009);
        let ours: Vec<usize> = (0..session.fighters.len())
            .filter(|&index| session.fighters[index].fleet == attacker)
            .collect();
        assert_eq!(ours.len(), 4);
        // All start in their hangars (no hyperdrive); one of the lost
        // carrier's launches.
        assert!(ours.iter().all(|&index| session.fighters[index].in_hangar));
        let out = ours[1];
        session.fighters[out].in_hangar = false;
        session.fighters[out].alive = true;
        // The lost carrier's other squadron docked on the surviving one.
        let kept_index = session
            .ships
            .iter()
            .position(|ship| ship.fleet == attacker && ship.fleet_ship_index == 1)
            .unwrap();
        let returning = ours[3];
        session.fighters[returning].in_hangar = false;
        session.fighters[returning].alive = true;
        let docked = ours[2];
        session.fighters[docked].in_hangar = false;
        session.fighters[docked].recovery_state =
            rebellion_core::tactical::TacticalFighterRecoveryState::Recovered;
        session.fighters[docked].recovery_target = Some(kept_index);
        let lost = session
            .ships
            .iter_mut()
            .find(|ship| ship.fleet == attacker && ship.fleet_ship_index == 0)
            .unwrap();
        lost.hull_current = 0;
        lost.alive = false;
        session.winner = Some(CombatWinner::Defender);

        apply_results(&session, &mut world, &mut TroopTransportState::default());

        // The surviving carrier holds its own squadron, back from the
        // field, and the one that docked on it; the lost carrier's hangar
        // squadron died with it; its launched one is in the fleet on no
        // ship, as the carrier it would seat on is full.
        let fleet = &world.fleets[attacker];
        assert_eq!(rebellion_core::squadrons::total(&fleet.fighters), 3);
        assert_eq!(rebellion_core::squadrons::aboard(&fleet.fighters, kept), 2);
        assert!(fleet
            .fighters
            .iter()
            .any(|entry| entry.class == other && entry.carrier == Some(kept)));
        assert!(fleet
            .fighters
            .iter()
            .any(|entry| entry.class == wing && entry.carrier.is_none() && entry.count == 1));
        assert!(world.systems[system].fighters.is_empty());
    }

    #[test]
    fn invalid_entry_does_not_mutate_battle_state() {
        let (world, system, attacker, _defender) = opposing_empty_fleets();
        let mut tactical = TacticalState::default();
        let mut cooldowns = HashMap::new();
        let mut messages = MessageLog::default();
        let mut mode = GameMode::Galaxy;
        let result = begin_player_battle(
            &world,
            BattleEntry {
                system,
                attacker,
                defender: attacker,
                player_is_attacker: true,
                tick: 17,
                rng_seed: 0x1111_0002,
            },
            &mut tactical,
            &mut cooldowns,
            &mut messages,
            &mut mode,
        );
        assert_eq!(result, Err(BattleEntryError::SameFleet));
        assert_eq!(mode, GameMode::Galaxy);
        assert!(tactical.session.is_none());
        assert!(cooldowns.is_empty());
    }

    #[test]
    fn entry_rejects_two_death_stars_without_mutating_battle_state() {
        let (mut world, system, attacker, defender) = opposing_empty_fleets();
        world.fleets[attacker].has_death_star = true;
        world.fleets[defender].has_death_star = true;
        let mut tactical = TacticalState::default();
        let mut cooldowns = HashMap::new();
        let mut messages = MessageLog::default();
        let mut mode = GameMode::Galaxy;

        let result = begin_player_battle(
            &world,
            BattleEntry {
                system,
                attacker,
                defender,
                player_is_attacker: true,
                tick: 17,
                rng_seed: 0x1111_0003,
            },
            &mut tactical,
            &mut cooldowns,
            &mut messages,
            &mut mode,
        );

        assert_eq!(result, Err(BattleEntryError::MultipleDeathStars));
        assert_eq!(mode, GameMode::Galaxy);
        assert!(tactical.session.is_none());
        assert!(cooldowns.is_empty());
    }

    #[test]
    fn destroyed_losing_fleet_sets_its_officers_adrift_and_clears_fleet_assignment() {
        let (mut world, system, attacker, defender) = opposing_empty_fleets();
        world.fleets[attacker].has_death_star = true;
        let captive = world.characters.insert(Character {
            name: "Defeated Officer".into(),
            is_empire: true,
            current_system: Some(system),
            current_fleet: Some(defender),
            ..Character::default()
        });
        world.fleets[defender].characters.push(captive);
        let mut session =
            BattleSession::new(&world, system, attacker, defender, true, 23, 0x1111_0004);
        session.winner = Some(CombatWinner::Attacker);

        let before = persistence_snapshot(&session, &world);
        assert!(before.attacker_present);
        assert!(before.defender_present);
        assert!(before.attacker_has_death_star);

        apply_results(&session, &mut world, &mut TroopTransportState::default());

        let after = persistence_snapshot(&session, &world);
        assert!(after.attacker_present);
        assert!(!after.defender_present);
        assert!(after.attacker_has_death_star);

        assert!(world.fleets.contains_key(attacker));
        assert!(!world.fleets.contains_key(defender));
        // FUN_00556710 -> FUN_004ef2e0: adrift, not captive (the capture
        // this test once asserted had no source; mission-lifecycle.md,
        // "Adrift and the abodes").
        let officer = &world.characters[captive];
        assert!(officer.adrift && !officer.is_captive);
        assert_eq!(officer.captured_by, None);
        assert_eq!(officer.current_system, Some(system));
        assert_eq!(officer.current_fleet, None);
    }

}
