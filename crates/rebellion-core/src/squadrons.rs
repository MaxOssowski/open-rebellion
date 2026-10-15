//! Fighter squadrons on a planet, and their Move (`0x201`) between a planet
//! and a fleet in orbit (`ghidra/notes/planet-squadrons.md`).
//!
//! A built squadron goes to its destination's system, not to a fleet: the
//! product joins the container its Destination names through slot `+0xa8`
//! (`FUN_0052a140`, `FUN_0052a380`). The system lists it among its own
//! objects (`FUN_005039d0`, families `0x1c..0x1f`), and a squadron there
//! counts toward its side's presence (`FUN_00509710` → `FUN_00503a50`).
//!
//! A squadron's class lists Move, Confirmed Move and Scrap
//! (vtable `0x0065def0` slot `+0x3c`, `FUN_00502fe0` → `FUN_00557ce0`:
//! `0x201`, `0x202`, `0x204`, `0x200`). A move onto a fleet needs the ships'
//! room: each ship's fighter capacity (`+0x26c`) less the squadrons aboard
//! (`FUN_00500b40`, refused with status `0x14`/1).
//!
//! port: the room is the whole fleet's, where the original fills one ship
//! at a time; a squadron's move to another system is not ported.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{FighterKey, FleetKey, SystemKey};
use crate::world::{FighterEntry, GameWorld};

/// Add `count` squadrons of `class` to `entries`.
pub fn add(entries: &mut Vec<FighterEntry>, class: FighterKey, count: u32) {
    if count == 0 {
        return;
    }
    if let Some(entry) = entries.iter_mut().find(|entry| entry.class == class) {
        entry.count = entry.count.saturating_add(count);
    } else {
        entries.push(FighterEntry { class, count });
    }
}

/// Take `count` squadrons of `class` from `entries`, dropping an emptied
/// entry. Nothing changes when there are fewer.
pub fn take(entries: &mut Vec<FighterEntry>, class: FighterKey, count: u32) -> bool {
    let Some(index) = entries.iter().position(|entry| entry.class == class) else {
        return false;
    };
    if entries[index].count < count {
        return false;
    }
    entries[index].count -= count;
    if entries[index].count == 0 {
        entries.remove(index);
    }
    true
}

/// The squadrons in `entries`.
#[must_use]
pub fn total(entries: &[FighterEntry]) -> u32 {
    entries
        .iter()
        .fold(0_u32, |sum, entry| sum.saturating_add(entry.count))
}

/// Whether a squadron of `class` serves the Alliance.
#[must_use]
pub fn is_alliance(world: &GameWorld, class: FighterKey) -> Option<bool> {
    world
        .fighter_classes
        .get(class)
        .map(|value| value.is_alliance)
}

/// The squadrons of one side on `system`'s planet.
#[must_use]
pub fn on_planet(world: &GameWorld, system: SystemKey, alliance: bool) -> u32 {
    world.systems.get(system).map_or(0, |value| {
        value
            .fighters
            .iter()
            .filter(|entry| is_alliance(world, entry.class) == Some(alliance))
            .fold(0_u32, |sum, entry| sum.saturating_add(entry.count))
    })
}

/// A fleet's room for more squadrons: its living ships' fighter capacity
/// (`+0x26c`, CAPSHPSD `fighter_capacity`) less the squadrons aboard
/// (`FUN_00500b40`).
#[must_use]
pub fn fleet_room(world: &GameWorld, fleet: FleetKey) -> u32 {
    let Some(value) = world.fleets.get(fleet) else {
        return 0;
    };
    let capacity = value
        .capital_ships
        .iter()
        .filter(|ship| ship.alive)
        .filter_map(|ship| world.capital_ship_classes.get(ship.class))
        .fold(0_u32, |sum, class| {
            sum.saturating_add(class.fighter_capacity)
        });
    capacity.saturating_sub(total(&value.fighters))
}

/// Where a squadron is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SquadronPlace {
    /// On a system's planet.
    Planet(SystemKey),
    /// Aboard a fleet.
    Fleet(FleetKey),
}

/// Why a squadron's move is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquadronMoveError {
    MissingFleet,
    MissingSystem,
    /// No squadron of the class is at the place.
    MissingSquadron,
    /// The squadron is not the side's.
    WrongSide,
    /// A fleet in hyperspace takes no orders (manual p. 121).
    InTransit,
    /// The destination fleet is another side's (`FUN_00553410`, `1`/`0x24`).
    OtherSideDestination,
    /// The destination planet is held by another side. hyp: the regiment's
    /// rule (`FUN_0053d430`, `1`/`0x28`).
    OtherSidePlanet,
    /// A move onto the place the squadron is at.
    IntoItself,
    /// The fleet has no room (`FUN_00500b40`, `0x14`/1).
    NoRoom,
    /// port: a squadron's move to another system is not ported.
    OtherSystem,
}

impl fmt::Display for SquadronMoveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingFleet => "the fleet no longer exists",
            Self::MissingSystem => "the system no longer exists",
            Self::MissingSquadron => "the squadron is not there",
            Self::WrongSide => "the squadron belongs to another side",
            Self::InTransit => "the fleet is in hyperspace",
            Self::OtherSideDestination => "the destination fleet belongs to another side",
            Self::OtherSidePlanet => "the destination planet belongs to another side",
            Self::IntoItself => "the squadron is already there",
            Self::NoRoom => "the fleet has no room for fighters",
            Self::OtherSystem => "squadrons cannot yet travel between systems",
        })
    }
}

/// The system a place is in, and whether the place is usable: a planet
/// always, a fleet only in orbit (in its system's fleet list).
fn place_system(world: &GameWorld, place: SquadronPlace) -> Result<SystemKey, SquadronMoveError> {
    match place {
        SquadronPlace::Planet(system) => {
            if world.systems.contains_key(system) {
                Ok(system)
            } else {
                Err(SquadronMoveError::MissingSystem)
            }
        }
        SquadronPlace::Fleet(fleet) => {
            let value = world
                .fleets
                .get(fleet)
                .ok_or(SquadronMoveError::MissingFleet)?;
            let system = world
                .systems
                .get(value.location)
                .ok_or(SquadronMoveError::MissingSystem)?;
            if system.fleets.contains(&fleet) {
                Ok(value.location)
            } else {
                Err(SquadronMoveError::InTransit)
            }
        }
    }
}

fn entries(world: &GameWorld, place: SquadronPlace) -> &[FighterEntry] {
    match place {
        SquadronPlace::Planet(system) => world
            .systems
            .get(system)
            .map_or(&[], |value| value.fighters.as_slice()),
        SquadronPlace::Fleet(fleet) => world
            .fleets
            .get(fleet)
            .map_or(&[], |value| value.fighters.as_slice()),
    }
}

fn entries_mut(world: &mut GameWorld, place: SquadronPlace) -> Option<&mut Vec<FighterEntry>> {
    match place {
        SquadronPlace::Planet(system) => world
            .systems
            .get_mut(system)
            .map(|value| &mut value.fighters),
        SquadronPlace::Fleet(fleet) => world.fleets.get_mut(fleet).map(|value| &mut value.fighters),
    }
}

/// Whether a squadron's pop-up menu enables Move for the side `alliance`:
/// its own squadron of `class` at `place`, on a planet or aboard a fleet in
/// orbit (`FUN_004f9860`'s side and en-route rules). hyp: the object check's
/// other rules are untraced for a squadron.
#[must_use]
pub fn move_enabled(
    world: &GameWorld,
    alliance: bool,
    class: FighterKey,
    place: SquadronPlace,
) -> bool {
    is_alliance(world, class) == Some(alliance)
        && place_system(world, place).is_ok()
        && entries(world, place)
            .iter()
            .any(|entry| entry.class == class && entry.count > 0)
}

/// Check a Move of `count` squadrons of `class` from `from` to `to` for the
/// side `alliance`, as [`move_squadrons`] would, without moving them.
///
/// # Errors
/// Returns why the move is refused.
pub fn check_move(
    world: &GameWorld,
    alliance: bool,
    class: FighterKey,
    count: u32,
    from: SquadronPlace,
    to: SquadronPlace,
) -> Result<(), SquadronMoveError> {
    if from == to {
        return Err(SquadronMoveError::IntoItself);
    }
    let origin = place_system(world, from)?;
    if is_alliance(world, class) != Some(alliance) {
        return Err(SquadronMoveError::WrongSide);
    }
    let held = entries(world, from)
        .iter()
        .find(|entry| entry.class == class)
        .map_or(0, |entry| entry.count);
    if count == 0 || held < count {
        return Err(SquadronMoveError::MissingSquadron);
    }
    let destination = place_system(world, to)?;
    match to {
        SquadronPlace::Fleet(fleet) => {
            if world.fleets.get(fleet).map(|value| value.is_alliance) != Some(alliance) {
                return Err(SquadronMoveError::OtherSideDestination);
            }
        }
        SquadronPlace::Planet(system) => {
            let side = crate::dat::Faction::of_alliance(alliance);
            let holder = world.systems.get(system).and_then(crate::uprising::holder);
            if holder.is_some_and(|holder| holder != side) {
                return Err(SquadronMoveError::OtherSidePlanet);
            }
        }
    }
    if destination != origin {
        return Err(SquadronMoveError::OtherSystem);
    }
    if let SquadronPlace::Fleet(fleet) = to {
        if fleet_room(world, fleet) < count {
            return Err(SquadronMoveError::NoRoom);
        }
    }
    Ok(())
}

/// Move `count` squadrons of `class` from `from` to `to` within one
/// system, for the side `alliance`. A fleet left with no ship, squadron or
/// character disbands (`FUN_004fe630`).
///
/// # Errors
/// Returns [`check_move`]'s refusal, and then nothing changes.
pub fn move_squadrons(
    world: &mut GameWorld,
    alliance: bool,
    class: FighterKey,
    count: u32,
    from: SquadronPlace,
    to: SquadronPlace,
) -> Result<(), SquadronMoveError> {
    check_move(world, alliance, class, count, from, to)?;
    if let Some(source) = entries_mut(world, from) {
        take(source, class, count);
    }
    if let Some(target) = entries_mut(world, to) {
        add(target, class, count);
    }
    if let SquadronPlace::Fleet(fleet) = from {
        disband_if_empty(world, fleet);
    }
    Ok(())
}

/// Remove `fleet` when it holds nothing: no ship, squadron or character.
///
/// port: the original's fleet holds only capital ships, and disbands when
/// the last leaves (`FUN_004fe630`); the port's seeding makes fleets of
/// squadrons alone, which disband when their last squadron leaves.
fn disband_if_empty(world: &mut GameWorld, fleet: FleetKey) {
    let empty = world.fleets.get(fleet).is_some_and(|value| {
        value.capital_ships.is_empty() && value.fighters.is_empty() && value.characters.is_empty()
    });
    if !empty {
        return;
    }
    if let Some(value) = world.fleets.remove(fleet) {
        if let Some(system) = world.systems.get_mut(value.location) {
            system.fleets.retain(|&key| key != fleet);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{ExplorationStatus, Faction, SectorGroup};
    use crate::ids::DatId;
    use crate::world::{
        CapitalShipClass, ControlKind, FighterClass, Fleet, Sector, ShipInstance, System,
    };

    struct Setup {
        world: GameWorld,
        here: SystemKey,
        there: SystemKey,
        fleet: FleetKey,
        x_wing: FighterKey,
        tie: FighterKey,
    }

    /// An Alliance carrier fleet of `capacity` squadrons at `here`, both
    /// systems held by the Alliance.
    fn setup(capacity: u32) -> Setup {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(1),
            name: "Test".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let system = |name: &str| System {
            dat_id: DatId::new(0x9000_0001),
            name: name.into(),
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
            control: ControlKind::Controlled(Faction::Alliance),
        };
        let here = world.systems.insert(system("Here"));
        let there = world.systems.insert(system("There"));
        let carrier = world.capital_ship_classes.insert(CapitalShipClass {
            name: "Carrier".into(),
            is_alliance: true,
            fighter_capacity: capacity,
            hull: 100,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: here,
            capital_ships: vec![ShipInstance::new(carrier, 100, true)],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        world.systems[here].fleets.push(fleet);
        let x_wing = world.fighter_classes.insert(FighterClass {
            name: "X-wing".into(),
            is_alliance: true,
            ..FighterClass::default()
        });
        let tie = world.fighter_classes.insert(FighterClass {
            name: "TIE".into(),
            is_empire: true,
            ..FighterClass::default()
        });
        Setup {
            world,
            here,
            there,
            fleet,
            x_wing,
            tie,
        }
    }

    #[test]
    fn squadrons_add_to_their_class_entry_and_leave_an_emptied_one() {
        let s = setup(0);
        let mut entries = Vec::new();
        add(&mut entries, s.x_wing, 2);
        add(&mut entries, s.tie, 1);
        add(&mut entries, s.x_wing, 1);
        add(&mut entries, s.tie, 0);
        assert_eq!(total(&entries), 4);
        assert_eq!(entries.len(), 2);
        assert!(!take(&mut entries, s.tie, 2));
        assert!(take(&mut entries, s.tie, 1));
        assert_eq!(entries.len(), 1);
        assert!(!take(&mut entries, s.tie, 1));
        assert!(take(&mut entries, s.x_wing, 3));
        assert!(entries.is_empty());
    }

    #[test]
    fn a_fleet_s_room_is_its_living_ships_capacity_less_the_squadrons_aboard() {
        // FUN_00500b40: +0x26c (CAPSHPSD fighter_capacity) less the
        // squadrons aboard.
        let mut s = setup(3);
        s.world.fleets[s.fleet].fighters.push(FighterEntry {
            class: s.x_wing,
            count: 1,
        });
        assert_eq!(fleet_room(&s.world, s.fleet), 2);
        s.world.fleets[s.fleet].capital_ships[0].alive = false;
        assert_eq!(fleet_room(&s.world, s.fleet), 0);
        assert_eq!(fleet_room(&s.world, FleetKey::default()), 0);
    }

    #[test]
    fn a_side_counts_only_its_own_squadrons_on_a_planet() {
        let mut s = setup(0);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 2);
        add(&mut s.world.systems[s.here].fighters, s.tie, 1);
        assert_eq!(on_planet(&s.world, s.here, true), 2);
        assert_eq!(on_planet(&s.world, s.here, false), 1);
        assert_eq!(on_planet(&s.world, s.there, true), 0);
    }

    #[test]
    fn a_planet_s_squadrons_board_a_fleet_in_orbit_up_to_its_room() {
        // FUN_00557ce0: a squadron's Move (0x201); FUN_00500b40 refuses a
        // fleet without room (0x14/1).
        let mut s = setup(2);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 3);
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        assert_eq!(
            move_squadrons(&mut s.world, true, s.x_wing, 3, planet, fleet),
            Err(SquadronMoveError::NoRoom)
        );
        assert_eq!(total(&s.world.systems[s.here].fighters), 3);
        move_squadrons(&mut s.world, true, s.x_wing, 2, planet, fleet).unwrap();
        assert_eq!(total(&s.world.fleets[s.fleet].fighters), 2);
        assert_eq!(total(&s.world.systems[s.here].fighters), 1);
        assert_eq!(fleet_room(&s.world, s.fleet), 0);
    }

    #[test]
    fn a_fleet_s_squadrons_land_on_its_own_planet() {
        let mut s = setup(2);
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 2);
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        move_squadrons(&mut s.world, true, s.x_wing, 1, fleet, planet).unwrap();
        assert_eq!(total(&s.world.fleets[s.fleet].fighters), 1);
        assert_eq!(on_planet(&s.world, s.here, true), 1);
        // The carrier stays: a fleet with a ship does not disband.
        move_squadrons(&mut s.world, true, s.x_wing, 1, fleet, planet).unwrap();
        assert!(s.world.fleets.contains_key(s.fleet));
    }

    #[test]
    fn a_fleet_of_squadrons_alone_disbands_when_the_last_lands() {
        let mut s = setup(0);
        s.world.fleets[s.fleet].capital_ships.clear();
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 2);
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        move_squadrons(&mut s.world, true, s.x_wing, 1, fleet, planet).unwrap();
        assert!(s.world.fleets.contains_key(s.fleet));
        move_squadrons(&mut s.world, true, s.x_wing, 1, fleet, planet).unwrap();
        assert!(!s.world.fleets.contains_key(s.fleet));
        assert!(s.world.systems[s.here].fleets.is_empty());
        assert_eq!(on_planet(&s.world, s.here, true), 2);
    }

    #[test]
    fn a_squadron_move_is_refused_for_the_wrong_side_count_or_place() {
        let mut s = setup(4);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 1);
        add(&mut s.world.systems[s.here].fighters, s.tie, 1);
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        let check = |world: &GameWorld, alliance, class, count, from, to| {
            check_move(world, alliance, class, count, from, to)
        };
        assert_eq!(
            check(&s.world, true, s.tie, 1, planet, fleet),
            Err(SquadronMoveError::WrongSide)
        );
        // FUN_00553410 (1/0x24): an Empire squadron onto an Alliance fleet.
        assert_eq!(
            check(&s.world, false, s.tie, 1, planet, fleet),
            Err(SquadronMoveError::OtherSideDestination)
        );
        assert_eq!(
            check(&s.world, true, s.x_wing, 2, planet, fleet),
            Err(SquadronMoveError::MissingSquadron)
        );
        assert_eq!(
            check(&s.world, true, s.x_wing, 0, planet, fleet),
            Err(SquadronMoveError::MissingSquadron)
        );
        assert_eq!(
            check(&s.world, true, s.x_wing, 1, planet, planet),
            Err(SquadronMoveError::IntoItself)
        );
        // port: no squadron travels to another system yet.
        assert_eq!(
            check(
                &s.world,
                true,
                s.x_wing,
                1,
                planet,
                SquadronPlace::Planet(s.there)
            ),
            Err(SquadronMoveError::OtherSystem)
        );
        // hyp: FUN_0053d430's 1/0x28 refuses a planet another side holds.
        s.world.systems[s.here].control = ControlKind::Controlled(Faction::Empire);
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 1);
        assert_eq!(
            check(&s.world, true, s.x_wing, 1, fleet, planet),
            Err(SquadronMoveError::OtherSidePlanet)
        );
        assert_eq!(
            check(
                &s.world,
                true,
                s.x_wing,
                1,
                SquadronPlace::Planet(SystemKey::default()),
                fleet
            ),
            Err(SquadronMoveError::MissingSystem)
        );
        assert_eq!(
            check(
                &s.world,
                true,
                s.x_wing,
                1,
                SquadronPlace::Fleet(FleetKey::default()),
                planet
            ),
            Err(SquadronMoveError::MissingFleet)
        );
    }

    #[test]
    fn a_fleet_in_hyperspace_neither_gives_nor_takes_squadrons() {
        // Manual p. 121: a fleet in hyperspace takes no orders.
        let mut s = setup(2);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 1);
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 1);
        s.world.systems[s.here].fleets.clear();
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        assert_eq!(
            check_move(&s.world, true, s.x_wing, 1, planet, fleet),
            Err(SquadronMoveError::InTransit)
        );
        assert_eq!(
            check_move(&s.world, true, s.x_wing, 1, fleet, planet),
            Err(SquadronMoveError::InTransit)
        );
    }

    #[test]
    fn move_is_enabled_for_the_side_s_own_squadron_on_a_planet_or_fleet_in_orbit() {
        // FUN_004f9860: the object is the order's side and not en route.
        let mut s = setup(1);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 1);
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 1);
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        assert!(move_enabled(&s.world, true, s.x_wing, planet));
        assert!(move_enabled(&s.world, true, s.x_wing, fleet));
        assert!(!move_enabled(&s.world, false, s.x_wing, planet));
        assert!(!move_enabled(&s.world, true, s.tie, planet));
        assert!(!move_enabled(
            &s.world,
            true,
            s.x_wing,
            SquadronPlace::Planet(s.there)
        ));
        // Another class of the side, or an emptied entry, has no squadron
        // to move.
        let a_wing = s.world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            ..FighterClass::default()
        });
        assert!(!move_enabled(&s.world, true, a_wing, planet));
        s.world.systems[s.there].fighters.push(FighterEntry {
            class: s.x_wing,
            count: 0,
        });
        assert!(!move_enabled(
            &s.world,
            true,
            s.x_wing,
            SquadronPlace::Planet(s.there)
        ));
        s.world.systems[s.here].fleets.clear();
        assert!(!move_enabled(&s.world, true, s.x_wing, fleet));
    }

    #[test]
    fn a_refusal_reads_as_the_player_s_message() {
        // Our own wording for the advisor's line the app shows.
        assert_eq!(
            SquadronMoveError::NoRoom.to_string(),
            "the fleet has no room for fighters"
        );
        assert_eq!(
            SquadronMoveError::OtherSystem.to_string(),
            "squadrons cannot yet travel between systems"
        );
    }

    #[test]
    fn squadrons_pass_between_two_fleets_in_one_system() {
        let mut s = setup(2);
        let other = s.world.fleets.insert(Fleet {
            location: s.here,
            ..s.world.fleets[s.fleet].clone()
        });
        s.world.systems[s.here].fleets.push(other);
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 2);
        move_squadrons(
            &mut s.world,
            true,
            s.x_wing,
            2,
            SquadronPlace::Fleet(s.fleet),
            SquadronPlace::Fleet(other),
        )
        .unwrap();
        assert_eq!(total(&s.world.fleets[other].fighters), 2);
        assert!(s.world.fleets[s.fleet].fighters.is_empty());
    }
}
