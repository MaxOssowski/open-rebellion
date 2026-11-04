//! Moving the Alliance headquarters (manual p. 134, "Moving Alliance
//! Headquarters"; `ghidra/notes/headquarters-move.md`).
//!
//! The headquarters is the ALLFACSD object (family `0x20`). Its order list
//! (`FUN_00526c00`) holds Move `0x201` and Confirmed Move `0x202`, which
//! expand to one per-object move `0x204` (`FUN_0053ca50`) and its command
//! (`FUN_00578ab0`). The command changes the object's container at once and
//! marks it en route across systems (`FUN_00556390`, `FUN_004f7640`); the
//! trip takes the per-object transit time at the GNPRTB 1 default speed
//! (`FUN_004f63f0`, slot `+0x34`).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::blockade::BlockadeSystem;
use crate::dat::Faction;
use crate::ids::{ManufacturingFacilityKey, SystemKey};
use crate::movement::{default_speed, transit_ticks_between};
use crate::troop_transport::TroopTransportState;
use crate::victory::VictoryState;
use crate::world::GameWorld;

/// ALLFACSD's families (`FUN_00526cf0`, `0x20..0x22`).
const HEADQUARTERS_FAMILY: u8 = 0x20;

/// GNPRTB 7706 (5): the Alliance's support a headquarters gives the
/// populated system it stands in (`FUN_0055a0f0`).
const GNPRTB_HEADQUARTERS_SUPPORT: u16 = 7706;

/// `FUN_005555e0` refuses an ALLFACSD object's move out of a blockaded
/// system when `FUN_0055a020` over the system's fleets is below 31.
const BLOCKADE_ESCAPE_MINIMUM: u8 = 31;

/// The headquarters on its way: it already stands in its destination, marked
/// en route (`+0x50` bit 4) until the arrival day (`+0x44`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadquartersTransit {
    /// The system it left.
    pub origin: SystemKey,
    /// The system it moves to, and already stands in.
    pub destination: SystemKey,
    /// The game-day it arrives.
    pub arrival_tick: u64,
}

/// Why a headquarters move is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadquartersMoveError {
    /// No headquarters object stands in any system.
    Missing,
    /// The ordering side is not the headquarters' (`FUN_004f9860` 1).
    WrongSide,
    /// The headquarters is en route (`FUN_004f9860` 4).
    EnRoute,
    /// The headquarters stands in a blockaded system that its fleets
    /// cannot break (`FUN_005555e0`, `0x90`/4).
    Blockaded,
    /// The destination does not resolve (`1`/`0x22`).
    MissingDestination,
    /// The destination is not the ordering side's (`FUN_0053d430`,
    /// `1`/`0x28`). port: a destroyed system counts as another side's.
    OtherSideDestination,
}

impl fmt::Display for HeadquartersMoveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Missing => "there is no headquarters",
            Self::WrongSide => "the headquarters is not the ordering side's",
            Self::EnRoute => "the headquarters is en route",
            Self::Blockaded => "the headquarters is held by a blockade",
            Self::MissingDestination => "the destination does not exist",
            Self::OtherSideDestination => "the destination is not the side's",
        })
    }
}

/// What a headquarters move did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadquartersMove {
    /// The headquarters left for another system.
    Departed(HeadquartersTransit),
    /// The destination is its own system: the container does not change.
    Unchanged,
}

/// The headquarters object and the system it stands in.
#[must_use]
pub fn headquarters(world: &GameWorld) -> Option<(SystemKey, ManufacturingFacilityKey)> {
    world.systems.iter().find_map(|(system, value)| {
        value
            .manufacturing_facilities
            .iter()
            .copied()
            .find(|key| {
                world
                    .manufacturing_facilities
                    .get(*key)
                    .is_some_and(|facility| facility.class_dat_id.family() == HEADQUARTERS_FAMILY)
            })
            .map(|key| (system, key))
    })
}

/// Whether `alliance`'s Move and Confirmed Move rows are enabled for the
/// headquarters: the rows' `+0x18` (`FUN_00578c00`), the object check
/// `FUN_004f9860` (slot `+0x6c`, `FUN_005269b0`) and the blockade test
/// `FUN_005555e0`.
#[must_use]
pub fn move_enabled(world: &GameWorld, transport: &TroopTransportState, alliance: bool) -> bool {
    movable(world, transport, alliance).is_ok()
}

fn movable(
    world: &GameWorld,
    transport: &TroopTransportState,
    alliance: bool,
) -> Result<(SystemKey, ManufacturingFacilityKey), HeadquartersMoveError> {
    let (system, key) = headquarters(world).ok_or(HeadquartersMoveError::Missing)?;
    let facility = world
        .manufacturing_facilities
        .get(key)
        .ok_or(HeadquartersMoveError::Missing)?;
    let side = if alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    if facility.side != side {
        return Err(HeadquartersMoveError::WrongSide);
    }
    if transport.headquarters.is_some() {
        return Err(HeadquartersMoveError::EnRoute);
    }
    let value = world
        .systems
        .get(system)
        .ok_or(HeadquartersMoveError::Missing)?;
    if BlockadeSystem::system_is_blockaded(world, value)
        && BlockadeSystem::escape_percent(world, value) < BLOCKADE_ESCAPE_MINIMUM
    {
        return Err(HeadquartersMoveError::Blockaded);
    }
    Ok((system, key))
}

/// The headquarters' origin and its trip's days to `destination`
/// (`FUN_0053d200`, which the confirmation lists), or why it may not go.
pub fn plan(
    world: &GameWorld,
    transport: &TroopTransportState,
    alliance: bool,
    destination: SystemKey,
) -> Result<(SystemKey, u32), HeadquartersMoveError> {
    let (origin, _) = movable(world, transport, alliance)?;
    let target = world
        .systems
        .get(destination)
        .ok_or(HeadquartersMoveError::MissingDestination)?;
    let side = if alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    if target.is_destroyed || !target.control.is_controlled_by(side) {
        return Err(HeadquartersMoveError::OtherSideDestination);
    }
    Ok((origin, transit_days(world, origin, destination)))
}

/// `FUN_00555b30`: no days within one system, else the default speed's.
fn transit_days(world: &GameWorld, origin: SystemKey, destination: SystemKey) -> u32 {
    if origin == destination {
        return 0;
    }
    let position = |key: SystemKey| world.systems.get(key).map_or((0, 0), |s| (s.x, s.y));
    transit_ticks_between(
        world,
        position(origin),
        position(destination),
        default_speed(world),
    )
}

/// Move the headquarters to `destination` (`FUN_00578f30` →
/// `FUN_00556390`): it changes system at once and is en route until its
/// arrival day. The victory state's headquarters follows it, since the
/// original finds the headquarters by its object (`FUN_00506110`).
///
/// Each system's content change (`System_vtc8`, `FUN_00508660`) shifts the
/// Alliance's support by GNPRTB 7706 for an ALLFACSD object: down where it
/// leaves, up where it enters, each only on a populated system
/// (`FUN_0050c9f0`, `+0x88` bit 0). The manual (p. 134): "a small drop in
/// popular support at the system in which it was originally located".
pub fn move_headquarters(
    world: &mut GameWorld,
    transport: &mut TroopTransportState,
    victory: &mut VictoryState,
    alliance: bool,
    destination: SystemKey,
    now: u64,
) -> Result<HeadquartersMove, HeadquartersMoveError> {
    let (origin, days) = plan(world, transport, alliance, destination)?;
    if origin == destination {
        return Ok(HeadquartersMove::Unchanged);
    }
    let (_, key) = headquarters(world).ok_or(HeadquartersMoveError::Missing)?;
    let from = &mut world.systems[origin];
    from.manufacturing_facilities.retain(|&value| value != key);
    from.is_headquarters = false;
    let to = &mut world.systems[destination];
    to.manufacturing_facilities.push(key);
    to.is_headquarters = true;
    victory.alliance_hq = destination;
    let support = world
        .gnprtb
        .value(GNPRTB_HEADQUARTERS_SUPPORT, world.difficulty_index);
    for (system, delta) in [(origin, -support), (destination, support)] {
        if world.systems[system].is_populated {
            crate::uprising::apply_support_change(world, system, Faction::Alliance, delta);
        }
    }
    let transit = HeadquartersTransit {
        origin,
        destination,
        arrival_tick: now + u64::from(days),
    };
    transport.headquarters = Some(transit);
    Ok(HeadquartersMove::Departed(transit))
}

/// End the headquarters' trip when its day has come (event `0x387`,
/// `FUN_004fb520`): it is no longer en route.
pub fn advance(transport: &mut TroopTransportState, now: u64) -> Option<HeadquartersTransit> {
    let transit = transport.headquarters?;
    if transit.arrival_tick > now {
        return None;
    }
    transport.headquarters = None;
    Some(transit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{ExplorationStatus, SectorGroup};
    use crate::ids::DatId;
    use crate::world::{
        CapitalShipClass, ControlKind, Fleet, GnprtbEntry, GnprtbParams,
        ManufacturingFacilityInstance, Sector, ShipInstance, System,
    };

    fn add_system(world: &mut GameWorld, x: u16, side: Faction) -> SystemKey {
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(1),
            name: "Test".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(2),
            name: "System".into(),
            sector,
            x,
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
            control: ControlKind::Controlled(side),
        })
    }

    /// An Alliance HQ at x = 0, another Alliance system at x = 200 and an
    /// Imperial one at x = 400.
    fn galaxy() -> (GameWorld, [SystemKey; 3], VictoryState) {
        let mut world = GameWorld::default();
        let home = add_system(&mut world, 0, Faction::Alliance);
        let other = add_system(&mut world, 200, Faction::Alliance);
        let enemy = add_system(&mut world, 400, Faction::Empire);
        let hq = world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: DatId::new(0x2000_0001),
                side: Faction::Alliance,
                is_shipyard: false,
            });
        world.systems[home].manufacturing_facilities.push(hq);
        world.systems[home].is_headquarters = true;
        let victory = VictoryState::new(home, enemy);
        (world, [home, other, enemy], victory)
    }

    #[test]
    fn the_headquarters_moves_at_once_and_is_en_route_until_its_arrival_day() {
        // FUN_00556390: the container changes at once; FUN_004f63f0: the
        // default speed; FUN_00506110 finds the HQ by its object.
        let (mut world, [home, other, _], mut victory) = galaxy();
        let mut transport = TroopTransportState::default();
        let days = transit_days(&world, home, other);
        assert!(days > 0);
        let moved =
            move_headquarters(&mut world, &mut transport, &mut victory, true, other, 10).unwrap();
        let transit = HeadquartersTransit {
            origin: home,
            destination: other,
            arrival_tick: 10 + u64::from(days),
        };
        assert_eq!(moved, HeadquartersMove::Departed(transit));
        assert_eq!(headquarters(&world).map(|(system, _)| system), Some(other));
        assert!(world.systems[other].is_headquarters && !world.systems[home].is_headquarters);
        assert_eq!(victory.alliance_hq, other);
        // FUN_004f9860 4: no new Move while en route.
        assert!(!move_enabled(&world, &transport, true));
        assert_eq!(
            plan(&world, &transport, true, home),
            Err(HeadquartersMoveError::EnRoute)
        );
        assert_eq!(advance(&mut transport, transit.arrival_tick - 1), None);
        assert_eq!(advance(&mut transport, transit.arrival_tick), Some(transit));
        assert!(move_enabled(&world, &transport, true));
    }

    #[test]
    fn the_move_costs_the_origin_its_support_and_gives_it_to_a_populated_destination() {
        // FUN_00508660 (System_vtc8): GNPRTB 7706 for an ALLFACSD object,
        // -7706 where it leaves, +7706 where it enters (FUN_0055a0f0), on a
        // populated system only (FUN_0050c9f0). Manual p. 134.
        let (mut world, [home, other, _], mut victory) = galaxy();
        world.gnprtb = GnprtbParams::new(vec![GnprtbEntry {
            parameter_id: u32::from(GNPRTB_HEADQUARTERS_SUPPORT),
            development: 5,
            alliance_sp_easy: 5,
            alliance_sp_medium: 5,
            alliance_sp_hard: 5,
            empire_sp_easy: 5,
            empire_sp_medium: 5,
            empire_sp_hard: 5,
            multiplayer: 5,
        }]);
        let support = |world: &GameWorld, system: SystemKey| {
            (world.systems[system].popularity_alliance * 100.0).round() as i32
        };
        let mut transport = TroopTransportState::default();
        move_headquarters(&mut world, &mut transport, &mut victory, true, other, 0).unwrap();
        assert_eq!((support(&world, home), support(&world, other)), (45, 55));
        // Back to an unpopulated home: only the populated origin loses.
        world.systems[home].is_populated = false;
        transport.headquarters = None;
        move_headquarters(&mut world, &mut transport, &mut victory, true, home, 9).unwrap();
        assert_eq!((support(&world, home), support(&world, other)), (45, 50));
    }

    #[test]
    fn the_headquarters_moves_only_to_its_sides_systems_and_only_for_its_side() {
        // FUN_0053d430 1/0x28: the HQ is no fleet, ship or regiment;
        // FUN_004f9860 1: the order's side.
        let (world, [home, other, enemy], _) = galaxy();
        let transport = TroopTransportState::default();
        assert_eq!(
            plan(&world, &transport, true, enemy),
            Err(HeadquartersMoveError::OtherSideDestination)
        );
        assert_eq!(
            plan(&world, &transport, false, other),
            Err(HeadquartersMoveError::WrongSide)
        );
        assert!(!move_enabled(&world, &transport, false));
        assert_eq!(plan(&world, &transport, true, home), Ok((home, 0)));
        let mut world = world;
        world.systems[other].is_destroyed = true;
        assert_eq!(
            plan(&world, &transport, true, other),
            Err(HeadquartersMoveError::OtherSideDestination)
        );
    }

    #[test]
    fn a_move_to_the_headquarters_own_system_changes_nothing() {
        // FUN_00556390 within one system: no transit.
        let (mut world, [home, ..], mut victory) = galaxy();
        let mut transport = TroopTransportState::default();
        assert_eq!(
            move_headquarters(&mut world, &mut transport, &mut victory, true, home, 3),
            Ok(HeadquartersMove::Unchanged)
        );
        assert_eq!(transport.headquarters, None);
        assert!(world.systems[home].is_headquarters);
    }

    #[test]
    fn a_blockade_holds_the_headquarters_while_the_blockaders_leave_it_under_31_percent() {
        // FUN_005555e0: 0x90/4 when FUN_0055a020(ships, fighters) < 31,
        // 100 - ships * GNPRTB 7684 (23 here).
        let (mut world, [home, other, _], _) = galaxy();
        let penalty = 23;
        world.gnprtb = GnprtbParams::new(vec![GnprtbEntry {
            parameter_id: u32::from(crate::blockade::GNPRTB_WITHDRAW_PENALTY_PER_CAPSHIP),
            development: penalty,
            alliance_sp_easy: penalty,
            alliance_sp_medium: penalty,
            alliance_sp_hard: penalty,
            empire_sp_easy: penalty,
            empire_sp_medium: penalty,
            empire_sp_hard: penalty,
            multiplayer: penalty,
        }]);
        let class = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        let fleet = world.fleets.insert(Fleet {
            location: home,
            capital_ships: (0..4)
                .map(|_| ShipInstance::new(class, 100, false))
                .collect(),
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[home].fleets.push(fleet);
        let transport = TroopTransportState::default();
        // 100 - 4 * 23 = 8.
        assert_eq!(
            plan(&world, &transport, true, other),
            Err(HeadquartersMoveError::Blockaded)
        );
        assert!(!move_enabled(&world, &transport, true));
        // 100 - 3 * 23 = 31, not below 31.
        world.fleets[fleet].capital_ships.pop();
        assert!(plan(&world, &transport, true, other).is_ok());
    }

    #[test]
    fn each_refusal_reads_as_its_own_reason() {
        // port: the line that stands in for FUN_00487c90's advisor reaction.
        assert_eq!(
            HeadquartersMoveError::Blockaded.to_string(),
            "the headquarters is held by a blockade"
        );
        assert_eq!(
            HeadquartersMoveError::OtherSideDestination.to_string(),
            "the destination is not the side's"
        );
    }
}
