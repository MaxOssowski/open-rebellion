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
    for (fleet_key, is_attacker) in [
        (session.attacker_fleet, true),
        (session.defender_fleet, false),
    ] {
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
                if let Some(result) = session.ships.iter().find(|ship| {
                    ship.is_attacker == is_attacker && ship.fleet_ship_index == roster_index
                }) {
                    ship_inst.hull_current = result.hull_current.clamp(0, result.hull_max);
                    ship_inst.alive = result.alive && ship_inst.hull_current > 0;
                    if !ship_inst.alive {
                        ship_inst.hull_current = 0;
                    }
                }
            }
            fleet.capital_ships.retain(|ship| ship.alive);

            let mut survivors = vec![0_u32; fleet.fighters.len()];
            for fighter in session
                .fighters
                .iter()
                .filter(|fighter| fighter.is_attacker == is_attacker)
            {
                if fighter.squad_count > 0 {
                    if let Some(count) = survivors.get_mut(fighter.fleet_fighter_index) {
                        *count = count.saturating_add(1);
                    }
                }
            }
            for (entry, surviving_squadrons) in fleet.fighters.iter_mut().zip(survivors) {
                entry.count = surviving_squadrons;
            }
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
        apply_system_occupation(&mut world, system, Faction::Alliance, 43, &mut MessageLog::default());
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
