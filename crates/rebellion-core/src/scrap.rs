//! The Scrap order (`0x200`, `ghidra/notes/scrap-order.md`).
//!
//! The order (factory `FUN_0053dd20`, vtable `0x006619b0`) builds one
//! command per team member (vtable `0x006695a8`). The command checks its
//! object through the object's slot `+0x5c` (`FUN_004f28d0` and kin, over
//! `FUN_004f9860`) and executes slot `+0x98` → `+0xa0` (`FUN_00534c00`),
//! which destroys the object with reason `0x14` (slot `+0xac`,
//! `FUN_004f71d0`). The side then refunds half the class's refined
//! material cost (`FUN_00530270`, reasons `0x14..0x16`).

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::ids::{DatId, SystemKey, TroopKey};
use crate::production_cycle::ProductionState;
use crate::world::{FacilityRef, GameWorld};

/// The destroyed reason Scrap writes to the object's `+0x40`
/// (`FUN_00534c00`).
pub const SCRAP_REASON: u8 = 0x14;

/// What a Scrap order names: the facilities, the regiments, the squadrons
/// on a planet, the capital ships and the fleets.
///
/// port: squadrons aboard a fleet and regiments aboard a fleet take Scrap
/// in the original (`FUN_005575f0`) but are not ported yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrapTarget {
    Facility(FacilityRef),
    Troop(TroopKey),
    /// One squadron of `class` on `system`'s planet (squadron vtable
    /// `0x0065def0` slot `+0x5c`, `FUN_005575f0`). port: squadrons are
    /// counts per class.
    PlanetFighter {
        system: SystemKey,
        class: crate::ids::FighterKey,
    },
    /// One squadron of `class` aboard `fleet`: the same squadron slot
    /// `+0x5c` (`FUN_005575f0`) as on a planet. A0
    /// `2026-11-13-drag/16` shows Scrap white on three squadrons of a
    /// fleet. port: squadrons are counts per class in their fleet.
    FleetFighter {
        fleet: crate::ids::FleetKey,
        class: crate::ids::FighterKey,
    },
    /// The capital ship at `index` in `fleet` (vtables `0x65d650`,
    /// `0x65d930`, `0x65dc10`, slot `+0x5c` `FUN_005575f0`). port: a ship
    /// is named by its place in its fleet.
    Ship {
        fleet: crate::ids::FleetKey,
        index: usize,
    },
    /// A whole fleet (vtable `0x0065d438`, slot `+0x5c` `FUN_004fdbc0` →
    /// `FUN_004ffa70`). Destroyed with reason `0x14`, it hands the reason
    /// to every child (`FUN_004fb520`, `FUN_004f8710`): each ship,
    /// squadron and regiment aboard is scrapped and refunds half its cost.
    Fleet(crate::ids::FleetKey),
}

/// Why a Scrap order is refused: the second word of the status pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrapRefusal {
    /// `0x12`: the object does not resolve (`FUN_00504e60`).
    Unresolved,
    /// `1`: the object is not the ordering side's (`FUN_004f9860`,
    /// `+0x24` bits 6..7).
    NotOwnSide,
    /// `0x17`: the class takes no Scrap. A headquarters object
    /// (`0x20..0x21`, slot `+0x5c` `FUN_00526940`) refuses it.
    NotAccepted,
    /// `4`: the object is en route (`FUN_004f9860`, `+0x50` bit 4): a
    /// fleet in hyperspace.
    EnRoute,
}

impl ScrapRefusal {
    /// The status code the original reports.
    #[must_use]
    pub const fn code(self) -> u32 {
        // FUN_00504e60 (0x12), FUN_004f9860 (1 and 4), FUN_00526940
        // (0x17).
        match self {
            Self::Unresolved => 0x12,
            Self::NotOwnSide => 1,
            Self::NotAccepted => 0x17,
            Self::EnRoute => 4,
        }
    }
}

/// A Scrap that went through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scrapped {
    /// The system the object stood at.
    pub system: SystemKey,
    /// The refined material returned to the side.
    pub refund: i32,
}

/// The object's system, side and refined material cost.
fn resolve(world: &GameWorld, target: ScrapTarget) -> Option<(SystemKey, Faction, DatId)> {
    let (system, side, class) = match target {
        ScrapTarget::Facility(FacilityRef::Defense(key)) => {
            let f = world.defense_facilities.get(key)?;
            let system = world
                .systems
                .iter()
                .find(|(_, s)| s.defense_facilities.contains(&key))?
                .0;
            (system, f.side, f.class_dat_id)
        }
        ScrapTarget::Facility(FacilityRef::Manufacturing(key)) => {
            let f = world.manufacturing_facilities.get(key)?;
            let system = world
                .systems
                .iter()
                .find(|(_, s)| s.manufacturing_facilities.contains(&key))?
                .0;
            (system, f.side, f.class_dat_id)
        }
        ScrapTarget::Facility(FacilityRef::Production(key)) => {
            let f = world.production_facilities.get(key)?;
            let system = world
                .systems
                .iter()
                .find(|(_, s)| s.production_facilities.contains(&key))?
                .0;
            (system, f.side, f.class_dat_id)
        }
        ScrapTarget::Troop(key) => {
            let t = world.troops.get(key)?;
            let system = world
                .systems
                .iter()
                .find(|(_, s)| s.ground_units.contains(&key))?
                .0;
            let side = if t.is_alliance {
                Faction::Alliance
            } else {
                Faction::Empire
            };
            (system, side, t.class_dat_id)
        }
        ScrapTarget::PlanetFighter { system, class } => {
            let fighters = &world.systems.get(system)?.fighters;
            if !fighters.iter().any(|e| e.class == class && e.count > 0) {
                return None;
            }
            let c = world.fighter_classes.get(class)?;
            (system, Faction::of_alliance(c.is_alliance), c.dat_id)
        }
        ScrapTarget::FleetFighter { fleet, class } => {
            let f = world.fleets.get(fleet)?;
            if !f.fighters.iter().any(|e| e.class == class && e.count > 0) {
                return None;
            }
            let c = world.fighter_classes.get(class)?;
            (f.location, Faction::of_alliance(c.is_alliance), c.dat_id)
        }
        ScrapTarget::Ship { fleet, index } => {
            let f = world.fleets.get(fleet)?;
            let ship = f.capital_ships.get(index).filter(|s| s.alive)?;
            let c = world.capital_ship_classes.get(ship.class)?;
            (f.location, Faction::of_alliance(f.is_alliance), c.dat_id)
        }
        // A fleet has no class, so no cost of its own (FUN_004f2980).
        ScrapTarget::Fleet(fleet) => {
            let f = world.fleets.get(fleet)?;
            (
                f.location,
                Faction::of_alliance(f.is_alliance),
                DatId::new(0),
            )
        }
    };
    Some((system, side, class))
}

/// The name the confirmation lists for `target` (`FUN_0049a880` passes
/// the object's `+0x30`): a ship's own name, else its class's
/// (`FUN_004f6270`), and every other unit's class name.
#[must_use]
pub fn target_name(world: &GameWorld, target: ScrapTarget) -> Option<String> {
    match target {
        ScrapTarget::Ship { fleet, index } => {
            let ship = world.fleets.get(fleet)?.capital_ships.get(index)?;
            let class = world.capital_ship_classes.get(ship.class)?;
            Some(ship.name.clone().unwrap_or_else(|| class.name.clone()))
        }
        ScrapTarget::PlanetFighter { class, .. } | ScrapTarget::FleetFighter { class, .. } => {
            Some(world.fighter_classes.get(class)?.name.clone())
        }
        // A0 menus/25-fleet1-scrap-confirm.png lists "Fleet 1".
        ScrapTarget::Fleet(fleet) => world.fleet_name(fleet).map(str::to_owned),
        _ => {
            let (_, _, class) = resolve(world, target)?;
            Some(world.buildable_classes.get(&class)?.name.clone())
        }
    }
}

/// `FUN_004f2980` → `FUN_0053b860`: the class's refined material cost
/// (`+0x48`), 0 without a class.
fn refined_cost(world: &GameWorld, class: DatId) -> i32 {
    world.buildable_classes.get(&class).map_or(0, |c| {
        i32::try_from(c.refined_material_cost).unwrap_or(i32::MAX)
    })
}

/// Whether `side` may scrap `target`: the command's check (slot `+0x5c`).
///
/// `FUN_004f9860` also refuses an object that is not completed (`+0x50`
/// bit 2), destroyed (bit 3) or en route (bit 4). port: every unit and
/// facility in the world is completed, a removed one does not resolve, and
/// a regiment on a planet is never en route.
///
/// # Errors
///
/// The refusal, as above.
pub fn check(world: &GameWorld, side: Faction, target: ScrapTarget) -> Result<(), ScrapRefusal> {
    let (_, owner, class) = resolve(world, target).ok_or(ScrapRefusal::Unresolved)?;
    if (0x20..=0x21).contains(&class.family()) {
        return Err(ScrapRefusal::NotAccepted);
    }
    if owner != side || side == Faction::Neutral {
        return Err(ScrapRefusal::NotOwnSide);
    }
    Ok(())
}

/// [`check`] with the en-route refusal (status 4) `FUN_004f9860` gives a
/// fleet in hyperspace and each ship in it (`FUN_005575f0`): A0 greys an
/// arriving fleet's Scrap and its ship's
/// (`.artifacts/a0-vm/menus/22-fleet4-menu.png`, `29-own-ship-menu.png`).
/// port: every other target is never en route.
///
/// # Errors
///
/// The refusal of [`check`], or [`ScrapRefusal::EnRoute`].
pub fn check_with_movement(
    world: &GameWorld,
    movement: &crate::movement::MovementState,
    side: Faction,
    target: ScrapTarget,
) -> Result<(), ScrapRefusal> {
    check(world, side, target)?;
    match target {
        // A squadron aboard a fleet in hyperspace is en route with it
        // (FUN_004f8240).
        ScrapTarget::Fleet(fleet)
        | ScrapTarget::Ship { fleet, .. }
        | ScrapTarget::FleetFighter { fleet, .. }
            if movement.is_in_transit(fleet) =>
        {
            Err(ScrapRefusal::EnRoute)
        }
        _ => Ok(()),
    }
}

/// Scrap `target` for `side`: check it, remove it (reason `0x14`) and
/// refund half its class's refined material cost to the side's stock
/// (`FUN_00530270`: `refined + cost / 2`, the object completed).
///
/// port: the original hands the refund to waiting yards through the
/// stock's change notifier (slot `+0x1d4`); the port's yards take it on
/// their next daily pass.
///
/// # Errors
///
/// The refusal of [`check`].
pub fn scrap(
    world: &mut GameWorld,
    production: &mut ProductionState,
    transport: &mut crate::troop_transport::TroopTransportState,
    side: Faction,
    target: ScrapTarget,
) -> Result<Scrapped, ScrapRefusal> {
    check(world, side, target)?;
    let (system, owner, class) = resolve(world, target).ok_or(ScrapRefusal::Unresolved)?;
    let refund = match target {
        ScrapTarget::PlanetFighter { class, .. } | ScrapTarget::FleetFighter { class, .. } => {
            world.fighter_classes.get(class).map_or(0, |c| {
                i32::try_from(c.refined_material_cost).unwrap_or(i32::MAX)
            }) / 2
        }
        ScrapTarget::Ship { fleet, index } => {
            let ship = world.fleets[fleet].capital_ships[index].class;
            world.capital_ship_classes.get(ship).map_or(0, |c| {
                i32::try_from(c.refined_material_cost).unwrap_or(i32::MAX)
            }) / 2
        }
        ScrapTarget::Fleet(fleet) => fleet_refund(world, transport, fleet),
        _ => refined_cost(world, class) / 2,
    };
    match target {
        ScrapTarget::Facility(facility) => world.remove_facility(system, facility),
        ScrapTarget::Troop(key) => {
            if let Some(sys) = world.systems.get_mut(system) {
                sys.ground_units.retain(|&k| k != key);
            }
            world.troops.remove(key);
        }
        ScrapTarget::PlanetFighter { class, .. } => {
            if let Some(sys) = world.systems.get_mut(system) {
                crate::squadrons::take(&mut sys.fighters, class, 1);
            }
        }
        ScrapTarget::FleetFighter { fleet, class } => {
            if let Some(value) = world.fleets.get_mut(fleet) {
                crate::squadrons::take(&mut value.fighters, class, 1);
            }
            disband_if_empty(world, fleet);
        }
        ScrapTarget::Ship { fleet, index } => remove_ship(world, fleet, index),
        ScrapTarget::Fleet(fleet) => scrap_fleet(world, transport, fleet),
    }
    if let Some(stock) = production.stock_mut(owner) {
        stock.refined += refund;
    }
    Ok(Scrapped { system, refund })
}

/// Half the refined cost of every ship, squadron and regiment aboard
/// `fleet`, each its own refund (`FUN_00530270` per child; A0: Fleet 1's
/// corvette 14, transport 10 and two regiments 8 returned 20,
/// `docs/qa/2026-09-08-full-functionality-audit/evidence/2026-11-01-fleet-scrap-a0`).
fn fleet_refund(
    world: &GameWorld,
    transport: &crate::troop_transport::TroopTransportState,
    fleet: crate::ids::FleetKey,
) -> i32 {
    let half = |cost: u32| i32::try_from(cost).unwrap_or(i32::MAX) / 2;
    let Some(value) = world.fleets.get(fleet) else {
        return 0;
    };
    let ships: i32 = value
        .capital_ships
        .iter()
        .filter(|ship| ship.alive)
        .filter_map(|ship| world.capital_ship_classes.get(ship.class))
        .map(|class| half(class.refined_material_cost))
        .sum();
    let squadrons: i32 = value
        .fighters
        .iter()
        .filter_map(|entry| {
            let class = world.fighter_classes.get(entry.class)?;
            Some(half(class.refined_material_cost) * i32::try_from(entry.count).unwrap_or(0))
        })
        .sum();
    let regiments: i32 = transport
        .cargo(fleet)
        .iter()
        .filter_map(|&troop| world.troops.get(troop))
        .map(|troop| refined_cost(world, troop.class_dat_id) / 2)
        .sum();
    ships + squadrons + regiments
}

/// Remove `fleet` with everything aboard: its ships and squadrons, the
/// regiments in its cargo; the characters aboard go adrift at its system
/// (`FUN_004ef2e0`: a character whose container was scrapped starts its
/// autorouting; port: the adrift settling stands in) and the special
/// forces aboard land there.
fn scrap_fleet(
    world: &mut GameWorld,
    transport: &mut crate::troop_transport::TroopTransportState,
    fleet: crate::ids::FleetKey,
) {
    transport.destroy_fleet_cargo(world, fleet);
    let Some(value) = world.fleets.remove(fleet) else {
        return;
    };
    let location = value.location;
    for key in value.characters {
        if let Some(c) = world.characters.get_mut(key) {
            c.move_to(None, Some(location));
            c.ship = None;
            if !c.is_killed && !c.on_mission {
                c.adrift = true;
            }
        }
    }
    if let Some(system) = world.systems.get_mut(location) {
        system.fleets.retain(|&key| key != fleet);
        system.special_forces.extend(value.special_forces);
    }
}

/// Take the ship at `index` out of `fleet`; a fleet left holding nothing
/// disbands (`FUN_004fe630`). The regiments its remaining ships have no
/// room for are lost with the next cargo check
/// ([`crate::troop_transport::TroopTransportState::destroy_untransportable_cargo`]).
pub(crate) fn remove_ship(world: &mut GameWorld, fleet: crate::ids::FleetKey, index: usize) {
    world.fleets[fleet].capital_ships.remove(index);
    disband_if_empty(world, fleet);
}

/// Destroy the ship at `index` of `fleet` (`SetDestroyedReason`,
/// `FUN_004f71d0`, through slot `+0xac`). Its destroyed bit (`Flags50`
/// `0x8`, `FUN_004f7560`) runs `GameObj_vt11c` (`FUN_004fb520`), which hands
/// the reason to each child that is Completed and not en route
/// (`FUN_004f8710`, `FUN_004f7ed0`): the squadrons docked on the ship are
/// destroyed with it, and the characters aboard go adrift
/// ([`crate::missions::strand_passengers`]). The ship then leaves its fleet
/// as [`remove_ship`].
pub(crate) fn destroy_ship(world: &mut GameWorld, fleet: crate::ids::FleetKey, index: usize) {
    let Some(value) = world.fleets.get_mut(fleet) else {
        return;
    };
    let Some(ship) = value.capital_ships.get(index) else {
        return;
    };
    let id = ship.id;
    if id != 0 {
        value.fighters.retain(|entry| entry.carrier != Some(id));
        crate::missions::strand_passengers(world, fleet, id);
    }
    remove_ship(world, fleet, index);
}

/// Disband `fleet` when it holds no ship, squadron, character or special
/// force (`FUN_004fe630`).
pub(crate) fn disband_if_empty(world: &mut GameWorld, fleet: crate::ids::FleetKey) {
    let Some(value) = world.fleets.get(fleet) else {
        return;
    };
    if !(value.capital_ships.is_empty()
        && value.fighters.is_empty()
        && value.characters.is_empty()
        && value.special_forces.is_empty())
    {
        return;
    }
    let location = value.location;
    world.fleets.remove(fleet);
    if let Some(system) = world.systems.get_mut(location) {
        system.fleets.retain(|&key| key != fleet);
    }
}

/// What a sector window icon's Scrap gives for `side` (`FUN_00512700` kind
/// `& ~1`, each list filtered to the side by `FUN_00553350`;
/// `ghidra/notes/sector-icon-menus.md`):
/// - the facilities icon (kind 4) the system's manufacturing, then
///   production facilities (`FUN_0053b6e0`);
/// - the defenses icon (kind 8) its regiments, squadrons and defense
///   facilities, one target a squadron;
/// - the fleets icon (`0x10`) its fleets (`FUN_004ffe70`), those en route
///   there included ([`crate::movement::system_side_fleets`]), which
///   refuse (A0 menus3/23).
///
/// A0 menus3/08, 12 and 13 list Fleet 1; three Stormtrooper Regiments;
/// Training Facility, Construction Yard and Mine.
/// port: the order within each list follows the port's system lists.
#[must_use]
pub fn icon_targets(
    movement: &crate::movement::MovementState,
    world: &GameWorld,
    system: SystemKey,
    kind: u8,
    side: Faction,
) -> Vec<ScrapTarget> {
    let Some(value) = world.systems.get(system) else {
        return Vec::new();
    };
    let candidates: Vec<ScrapTarget> = match kind {
        4 => value
            .manufacturing_facilities
            .iter()
            .map(|&key| ScrapTarget::Facility(FacilityRef::Manufacturing(key)))
            .chain(
                value
                    .production_facilities
                    .iter()
                    .map(|&key| ScrapTarget::Facility(FacilityRef::Production(key))),
            )
            .collect(),
        8 => value
            .ground_units
            .iter()
            .map(|&key| ScrapTarget::Troop(key))
            .chain(value.fighters.iter().flat_map(|entry| {
                std::iter::repeat_n(
                    ScrapTarget::PlanetFighter {
                        system,
                        class: entry.class,
                    },
                    entry.count as usize,
                )
            }))
            .chain(
                value
                    .defense_facilities
                    .iter()
                    .map(|&key| ScrapTarget::Facility(FacilityRef::Defense(key))),
            )
            .collect(),
        0x10 => crate::movement::listed_fleets(movement, world, system)
            .into_iter()
            .map(ScrapTarget::Fleet)
            .collect(),
        _ => Vec::new(),
    };
    candidates
        .into_iter()
        .filter(|&target| resolve(world, target).is_some_and(|(_, owner, _)| owner == side))
        .collect()
}

/// A Scrap order's team: the validator (`FUN_0051ff30`) checks every
/// member's command before the order executes them all (`FUN_005539f0`),
/// so one refusal keeps every member.
///
/// # Errors
///
/// The first member's refusal of [`check`].
pub fn scrap_team(
    world: &mut GameWorld,
    production: &mut ProductionState,
    transport: &mut crate::troop_transport::TroopTransportState,
    side: Faction,
    team: &[ScrapTarget],
) -> Result<Vec<Scrapped>, ScrapRefusal> {
    for &target in team {
        check(world, side, target)?;
    }
    team.iter()
        .map(|&target| scrap(world, production, transport, side, target))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::troop_transport::TroopTransportState;
    use crate::world::{
        BuildableClass, DefenseFacilityInstance, ManufacturingFacilityInstance,
        ProductionFacilityInstance, System, TroopUnit,
    };

    struct Fixture {
        world: GameWorld,
        system: SystemKey,
    }

    fn fixture() -> Fixture {
        let mut world = GameWorld::default();
        let system = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "System".into(),
            sector: crate::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 10,
            raw_materials: 5,
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
            control: crate::world::ControlKind::Controlled(Faction::Alliance),
        });
        Fixture { world, system }
    }

    fn class(world: &mut GameWorld, raw: u32, refined: u32) -> DatId {
        let id = DatId::new(raw);
        world.buildable_classes.insert(
            id,
            BuildableClass {
                refined_material_cost: refined,
                ..BuildableClass::default()
            },
        );
        id
    }

    fn mine(f: &mut Fixture, side: Faction, refined: u32) -> FacilityRef {
        let class_dat_id = class(&mut f.world, 0x2c00_0001, refined);
        let key = f
            .world
            .production_facilities
            .insert(ProductionFacilityInstance {
                class_dat_id,
                side,
                is_mine: true,
            });
        f.world.systems[f.system].production_facilities.push(key);
        FacilityRef::Production(key)
    }

    #[test]
    fn scrapping_a_facility_removes_it_and_refunds_half_its_refined_cost() {
        // FUN_00534c00 destroys with reason 0x14; FUN_00530270 adds
        // class +0x48 / 2 = 7 / 2 = 3 to the side's refined (+0x7c).
        let mut f = fixture();
        let target = ScrapTarget::Facility(mine(&mut f, Faction::Alliance, 7));
        let mut production = ProductionState::new();
        production.stock_mut(Faction::Alliance).unwrap().refined = 2;

        let done = scrap(
            &mut f.world,
            &mut production,
            &mut TroopTransportState::new(),
            Faction::Alliance,
            target,
        )
        .unwrap();

        assert_eq!(
            done,
            Scrapped {
                system: f.system,
                refund: 3
            }
        );
        assert!(f.world.production_facilities.is_empty());
        assert!(f.world.systems[f.system].production_facilities.is_empty());
        assert_eq!(production.stock(Faction::Alliance).unwrap().refined, 5);
        assert_eq!(production.stock(Faction::Empire).unwrap().refined, 0);
    }

    #[test]
    fn an_icons_scrap_takes_its_kinds_objects_of_the_side() {
        // FUN_00512700: kind 4 the manufacturing then production
        // facilities, kind 8 regiments, squadrons and defense facilities,
        // 0x10 the fleets; FUN_00553350 keeps the order's side. A0
        // menus3/08, 12, 13.
        let mut f = fixture();
        let mine = mine(&mut f, Faction::Empire, 4);
        let _theirs = self::mine(&mut f, Faction::Alliance, 4);
        let yard = f
            .world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: DatId::new(0x2800_0001),
                side: Faction::Empire,
                is_shipyard: false,
            });
        f.world.systems[f.system].manufacturing_facilities.push(yard);
        let gun = f.world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: DatId::new(0x2000_0001),
            side: Faction::Empire,
        });
        f.world.systems[f.system].defense_facilities.push(gun);
        let troop = f.world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance: false,
            regiment_strength: 100,
        });
        let rebels = f.world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1000_0002),
            is_alliance: true,
            regiment_strength: 100,
        });
        f.world.systems[f.system].ground_units.extend([troop, rebels]);
        let tie = f.world.fighter_classes.insert(crate::world::FighterClass {
            is_alliance: false,
            ..Default::default()
        });
        crate::squadrons::add(&mut f.world.systems[f.system].fighters, tie, 2);
        let fleet = |f: &mut Fixture, is_alliance| {
            let key = f.world.insert_fleet(crate::world::Fleet {
                location: f.system,
                capital_ships: vec![],
                fighters: vec![],
                characters: vec![],
                is_alliance,
                has_death_star: false,
                special_forces: Vec::new(),
            });
            f.world.systems[f.system].fleets.push(key);
            key
        };
        let ours = fleet(&mut f, false);
        let _rebel_fleet = fleet(&mut f, true);
        let movement = crate::movement::MovementState::new();
        let targets = |kind| icon_targets(&movement, &f.world, f.system, kind, Faction::Empire);

        assert_eq!(
            targets(4),
            [
                ScrapTarget::Facility(FacilityRef::Manufacturing(yard)),
                ScrapTarget::Facility(mine),
            ]
        );
        let squadron = ScrapTarget::PlanetFighter {
            system: f.system,
            class: tie,
        };
        assert_eq!(
            targets(8),
            [
                ScrapTarget::Troop(troop),
                squadron,
                squadron,
                ScrapTarget::Facility(FacilityRef::Defense(gun)),
            ]
        );
        assert_eq!(targets(0x10), [ScrapTarget::Fleet(ours)]);
        assert!(targets(0x40).is_empty());
        assert!(
            icon_targets(&movement, &f.world, SystemKey::default(), 4, Faction::Empire).is_empty()
        );
    }

    #[test]
    fn the_confirmation_names_a_facility_by_its_class_and_a_ship_by_its_own_name() {
        // FUN_0049a880 lists each object's +0x30 name; FUN_004f6270 gives
        // a ship's own name before its class's.
        let mut f = fixture();
        let target = ScrapTarget::Facility(mine(&mut f, Faction::Alliance, 7));
        f.world
            .buildable_classes
            .get_mut(&DatId::new(0x2c00_0001))
            .unwrap()
            .name = "Mine".into();
        assert_eq!(target_name(&f.world, target).as_deref(), Some("Mine"));

        let class = f
            .world
            .capital_ship_classes
            .insert(crate::world::CapitalShipClass {
                is_alliance: true,
                name: "Corellian Corvette".into(),
                ..Default::default()
            });
        let mut ships = crate::world::ShipInstance::make(class, 10, true, 2);
        ships[1].name = Some("Tantive IV".into());
        let fleet = f.world.insert_fleet(crate::world::Fleet {
            location: f.system,
            capital_ships: ships,
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        let ship = |index| target_name(&f.world, ScrapTarget::Ship { fleet, index });
        assert_eq!(ship(0).as_deref(), Some("Corellian Corvette"));
        assert_eq!(ship(1).as_deref(), Some("Tantive IV"));
        assert_eq!(ship(2), None);

        // A planet's squadron goes by its fighter class's name.
        let x_wing = f.world.fighter_classes.insert(crate::world::FighterClass {
            is_alliance: true,
            name: "X-wing".into(),
            ..Default::default()
        });
        crate::squadrons::add(&mut f.world.systems[f.system].fighters, x_wing, 1);
        let squadron = ScrapTarget::PlanetFighter {
            system: f.system,
            class: x_wing,
        };
        assert_eq!(target_name(&f.world, squadron).as_deref(), Some("X-wing"));
    }

    #[test]
    fn a_capital_ship_is_scrapped_out_of_its_fleet_which_disbands_when_emptied() {
        // A ship's check is FUN_005575f0 (vtables 0x65d650.., slot +0x5c);
        // FUN_00530270 refunds +0x48 / 2 = 11 / 2 = 5. The fleet whose last
        // ship goes disbands (FUN_004fe630); one keeping a squadron stays.
        let mut f = fixture();
        let class = f
            .world
            .capital_ship_classes
            .insert(crate::world::CapitalShipClass {
                is_alliance: true,
                refined_material_cost: 11,
                ..Default::default()
            });
        let wing = f.world.fighter_classes.insert(crate::world::FighterClass {
            is_alliance: true,
            ..Default::default()
        });
        let fleet = |f: &mut Fixture, ships: usize, fighters| {
            let key = f.world.insert_fleet(crate::world::Fleet {
                location: f.system,
                capital_ships: crate::world::ShipInstance::make(class, 10, true, ships as u32),
                fighters,
                characters: vec![],
                is_alliance: true,
                has_death_star: false,
                special_forces: Vec::new(),
            });
            f.world.systems[f.system].fleets.push(key);
            key
        };
        let pair = fleet(&mut f, 2, vec![]);
        let carrier = fleet(
            &mut f,
            1,
            vec![crate::world::FighterEntry {
                class: wing,
                count: 1,
                carrier: None,
            }],
        );
        let mut production = ProductionState::new();
        let first = ScrapTarget::Ship {
            fleet: pair,
            index: 1,
        };
        assert_eq!(
            check(&f.world, Faction::Empire, first),
            Err(ScrapRefusal::NotOwnSide)
        );
        let done = scrap(
            &mut f.world,
            &mut production,
            &mut TroopTransportState::new(),
            Faction::Alliance,
            first,
        )
        .unwrap();
        assert_eq!((done.system, done.refund), (f.system, 5));
        assert_eq!(f.world.fleets[pair].capital_ships.len(), 1);
        assert_eq!(
            check(&f.world, Faction::Alliance, first),
            Err(ScrapRefusal::Unresolved)
        );
        let last = ScrapTarget::Ship {
            fleet: pair,
            index: 0,
        };
        scrap(
            &mut f.world,
            &mut production,
            &mut TroopTransportState::new(),
            Faction::Alliance,
            last,
        )
        .unwrap();
        assert!(!f.world.fleets.contains_key(pair));
        assert!(!f.world.systems[f.system].fleets.contains(&pair));
        let alone = ScrapTarget::Ship {
            fleet: carrier,
            index: 0,
        };
        scrap(
            &mut f.world,
            &mut production,
            &mut TroopTransportState::new(),
            Faction::Alliance,
            alone,
        )
        .unwrap();
        assert!(f.world.fleets[carrier].capital_ships.is_empty());
        assert!(f.world.systems[f.system].fleets.contains(&carrier));
        assert_eq!(production.stock(Faction::Alliance).unwrap().refined, 15);
        // A destroyed ship does not resolve.
        let wreck = fleet(&mut f, 1, vec![]);
        f.world.fleets[wreck].capital_ships[0].alive = false;
        assert_eq!(
            check(
                &f.world,
                Faction::Alliance,
                ScrapTarget::Ship {
                    fleet: wreck,
                    index: 0
                }
            ),
            Err(ScrapRefusal::Unresolved)
        );
    }

    // GameObj_vt11c (FUN_004fb520) -> FUN_004f8710 -> FUN_004f7ed0: the
    // squadrons docked on a destroyed ship go with it; the fleet's other
    // ships keep theirs, and a fleet left empty disbands.
    #[test]
    fn a_destroyed_ships_docked_squadrons_are_destroyed_with_it() {
        let mut f = fixture();
        let class = f.world.capital_ship_classes.insert(crate::world::CapitalShipClass::default());
        let fighter = f.world.fighter_classes.insert(crate::world::FighterClass::default());
        let mut ships = crate::world::ShipInstance::make(class, 10, true, 2);
        ships[0].id = 7;
        ships[1].id = 8;
        let key = f.world.insert_fleet(crate::world::Fleet {
            location: f.system,
            capital_ships: ships,
            fighters: vec![
                crate::world::FighterEntry { class: fighter, count: 2, carrier: Some(7) },
                crate::world::FighterEntry { class: fighter, count: 1, carrier: Some(8) },
                crate::world::FighterEntry { class: fighter, count: 1, carrier: None },
            ],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        f.world.systems[f.system].fleets.push(key);
        destroy_ship(&mut f.world, key, 0);
        let fleet = &f.world.fleets[key];
        assert_eq!(fleet.capital_ships.len(), 1);
        assert_eq!(
            fleet.fighters.iter().map(|entry| (entry.count, entry.carrier)).collect::<Vec<_>>(),
            [(1, Some(8)), (1, None)]
        );
        // An unnumbered ship (id 0) takes no squadron with it.
        f.world.fleets[key].capital_ships[0].id = 0;
        f.world.fleets[key].fighters[0].carrier = Some(0);
        destroy_ship(&mut f.world, key, 0);
        assert_eq!(f.world.fleets[key].fighters.len(), 2);
        // A fleet left with nothing disbands.
        f.world.fleets[key].fighters.clear();
        f.world.fleets[key].capital_ships = crate::world::ShipInstance::make(class, 10, true, 1);
        destroy_ship(&mut f.world, key, 0);
        assert!(!f.world.fleets.contains_key(key));
    }

    #[test]
    fn a_destroyed_ships_characters_go_adrift_and_the_others_stay() {
        // GameObj_vt11c (FUN_004fb520) runs each child's slot +0xb4; a
        // character's (FUN_004ef2e0) on no mission goes Adrift
        // (Role_SetAdrift FUN_00534790) where it stands. Characters on the
        // fleet's other ships are not its children.
        let mut f = fixture();
        let class = f.world.capital_ship_classes.insert(crate::world::CapitalShipClass::default());
        let mut ships = crate::world::ShipInstance::make(class, 10, true, 2);
        ships[0].id = 7;
        ships[1].id = 8;
        let mut character = |ship, on_mission| {
            f.world.characters.insert(crate::world::Character {
                ship: Some(ship),
                on_mission,
                current_system: Some(f.system),
                ..Default::default()
            })
        };
        let aboard = character(7, false);
        let on_mission = character(7, true);
        let other = character(8, false);
        let key = f.world.insert_fleet(crate::world::Fleet {
            location: f.system,
            capital_ships: ships,
            fighters: vec![],
            characters: vec![aboard, on_mission, other],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        f.world.systems[f.system].fleets.push(key);
        for c in [aboard, on_mission, other] {
            f.world.characters[c].current_fleet = Some(key);
        }
        destroy_ship(&mut f.world, key, 0);
        assert_eq!(f.world.fleets[key].characters, [other]);
        let c = &f.world.characters[aboard];
        assert!(c.adrift);
        assert_eq!((c.current_fleet, c.current_system, c.ship), (None, Some(f.system), None));
        // A character on a mission is not set Adrift (Flags78 OnMission).
        assert!(!f.world.characters[on_mission].adrift);
        assert_eq!(f.world.characters[on_mission].current_fleet, None);
        assert!(!f.world.characters[other].adrift);
        assert_eq!(f.world.characters[other].current_fleet, Some(key));
    }

    #[test]
    fn a_fleet_is_scrapped_with_everything_aboard_each_refunding_half() {
        // A0 (evidence 2026-11-01-fleet-scrap-a0): Fleet 1's corvette 14,
        // transport 10 and two regiments 8 returned (14 + 10 + 8 + 8) / 2
        // = 20 the same day; the fleet hands reason 0x14 to every child
        // (FUN_004fb520, FUN_004f8710), each refunding half (FUN_00530270).
        // Here two squadrons of 7 add 3 each. The character aboard goes
        // adrift (FUN_004ef2e0), one on a mission does not (Flags78
        // OnMission); the special force lands at the system.
        let mut f = fixture();
        let ship = |f: &mut Fixture, refined| {
            f.world
                .capital_ship_classes
                .insert(crate::world::CapitalShipClass {
                    is_alliance: true,
                    refined_material_cost: refined,
                    troop_capacity: 2,
                    ..Default::default()
                })
        };
        let corvette = ship(&mut f, 14);
        let transport_class = ship(&mut f, 10);
        let wing = f.world.fighter_classes.insert(crate::world::FighterClass {
            is_alliance: true,
            refined_material_cost: 7,
            ..Default::default()
        });
        let mut ships = crate::world::ShipInstance::make(corvette, 10, true, 1);
        ships.extend(crate::world::ShipInstance::make(
            transport_class,
            10,
            true,
            1,
        ));
        let captain = f.world.characters.insert(crate::world::Character {
            current_system: Some(f.system),
            ..Default::default()
        });
        let agent = f.world.characters.insert(crate::world::Character {
            current_system: Some(f.system),
            on_mission: true,
            ..Default::default()
        });
        let unit = f
            .world
            .special_forces
            .insert(crate::world::SpecialForceUnit {
                class_dat_id: DatId::new(0x1400_0001),
                is_alliance: true,
                skills: [0; 8],
                on_mission: false,
            });
        let fleet = f.world.insert_fleet(crate::world::Fleet {
            location: f.system,
            capital_ships: ships,
            fighters: vec![crate::world::FighterEntry {
                class: wing,
                count: 2,
                carrier: None,
            }],
            characters: vec![captain, agent],
            is_alliance: true,
            has_death_star: false,
            special_forces: vec![unit],
        });
        f.world.systems[f.system].fleets.push(fleet);
        f.world.characters[captain].current_fleet = Some(fleet);
        let regiment = class(&mut f.world, 0x1000_0001, 8);
        let troops: Vec<TroopKey> = (0..2)
            .map(|_| {
                f.world.troops.insert(TroopUnit {
                    class_dat_id: regiment,
                    is_alliance: true,
                    regiment_strength: 100,
                })
            })
            .collect();
        f.world.systems[f.system].ground_units.extend(&troops);
        let mut transport = TroopTransportState::new();
        transport.embark(&mut f.world, fleet, &troops).unwrap();
        let target = ScrapTarget::Fleet(fleet);
        assert_eq!(target_name(&f.world, target).as_deref(), Some("Fleet 1"));
        assert_eq!(
            check(&f.world, Faction::Empire, target),
            Err(ScrapRefusal::NotOwnSide)
        );
        let mut production = ProductionState::new();

        let done = scrap(
            &mut f.world,
            &mut production,
            &mut transport,
            Faction::Alliance,
            target,
        )
        .unwrap();

        assert_eq!((done.system, done.refund), (f.system, 20 + 6));
        assert_eq!(production.stock(Faction::Alliance).unwrap().refined, 26);
        assert!(!f.world.fleets.contains_key(fleet));
        assert!(!f.world.systems[f.system].fleets.contains(&fleet));
        assert!(f.world.troops.is_empty());
        assert!(transport.cargo(fleet).is_empty());
        let c = &f.world.characters[captain];
        assert!(c.adrift);
        assert_eq!((c.current_fleet, c.current_system), (None, Some(f.system)));
        assert!(!f.world.characters[agent].adrift);
        assert_eq!(f.world.characters[agent].current_fleet, None);
        assert_eq!(f.world.systems[f.system].special_forces, [unit]);
        assert_eq!(
            check(&f.world, Faction::Alliance, target),
            Err(ScrapRefusal::Unresolved)
        );
    }

    #[test]
    fn a_fleet_in_hyperspace_refuses_scrap() {
        // FUN_004ffa70 -> FUN_004f9860 status 4: A0 greys an arriving
        // fleet's Scrap (menus/22-fleet4-menu.png).
        let mut f = fixture();
        let class = f
            .world
            .capital_ship_classes
            .insert(crate::world::CapitalShipClass {
                is_alliance: true,
                hyperdrive: 100,
                ..Default::default()
            });
        let there = f.world.systems.insert(f.world.systems[f.system].clone());
        f.world.systems[there].x = 50;
        let fleet = f.world.insert_fleet(crate::world::Fleet {
            location: f.system,
            capital_ships: crate::world::ShipInstance::make(class, 10, true, 1),
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        f.world.systems[f.system].fleets.push(fleet);
        let mut movement = crate::movement::MovementState::new();
        let target = ScrapTarget::Fleet(fleet);
        let ship = ScrapTarget::Ship { fleet, index: 0 };
        // A squadron aboard is en route with its fleet (FUN_004f8240).
        let x_wing = f.world.fighter_classes.insert(crate::world::FighterClass {
            is_alliance: true,
            ..Default::default()
        });
        crate::squadrons::add(&mut f.world.fleets[fleet].fighters, x_wing, 1);
        let squadron = ScrapTarget::FleetFighter {
            fleet,
            class: x_wing,
        };
        for target in [target, ship, squadron] {
            assert_eq!(
                check_with_movement(&f.world, &movement, Faction::Alliance, target),
                Ok(())
            );
        }
        crate::movement::begin_faction_fleet_transit(
            &mut movement,
            &mut f.world,
            fleet,
            there,
            true,
        )
        .unwrap();
        assert!(movement.is_in_transit(fleet));
        for target in [target, ship, squadron] {
            assert_eq!(
                check_with_movement(&f.world, &movement, Faction::Alliance, target),
                Err(ScrapRefusal::EnRoute)
            );
        }
        assert_eq!(ScrapRefusal::EnRoute.code(), 4);
        assert_eq!(
            check_with_movement(&f.world, &movement, Faction::Empire, target),
            Err(ScrapRefusal::NotOwnSide)
        );
    }

    #[test]
    fn a_squadron_aboard_a_fleet_is_scrapped_one_at_a_time_and_an_emptied_fleet_disbands() {
        // The squadron's slot +0x5c (FUN_005575f0) as on a planet; A0
        // 2026-11-13-drag/16 shows Scrap white on a fleet's squadrons.
        // FUN_00530270 refunds +0x48 / 2 = 9 / 2 = 4 each.
        let mut f = fixture();
        let x_wing = f.world.fighter_classes.insert(crate::world::FighterClass {
            is_alliance: true,
            refined_material_cost: 9,
            name: "X-wing".into(),
            ..Default::default()
        });
        let fleet = f.world.insert_fleet(crate::world::Fleet {
            location: f.system,
            capital_ships: Vec::new(),
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        f.world.systems[f.system].fleets.push(fleet);
        crate::squadrons::add(&mut f.world.fleets[fleet].fighters, x_wing, 2);
        let target = ScrapTarget::FleetFighter {
            fleet,
            class: x_wing,
        };
        assert_eq!(target_name(&f.world, target).as_deref(), Some("X-wing"));
        assert_eq!(
            check(&f.world, Faction::Empire, target),
            Err(ScrapRefusal::NotOwnSide)
        );
        let mut production = ProductionState::new();
        let mut scrap_one = |world: &mut GameWorld| {
            scrap(
                world,
                &mut production,
                &mut TroopTransportState::new(),
                Faction::Alliance,
                target,
            )
        };
        let done = scrap_one(&mut f.world).unwrap();
        assert_eq!(done.refund, 4);
        assert_eq!(done.system, f.system);
        assert_eq!(crate::squadrons::total(&f.world.fleets[fleet].fighters), 1);
        scrap_one(&mut f.world).unwrap();
        assert!(!f.world.fleets.contains_key(fleet), "the emptied fleet disbands");
        assert_eq!(
            check(&f.world, Faction::Alliance, target),
            Err(ScrapRefusal::Unresolved)
        );
        // An emptied entry, or only another class, does not resolve.
        let a_wing = f.world.fighter_classes.insert(crate::world::FighterClass {
            is_alliance: true,
            ..Default::default()
        });
        let other = f.world.insert_fleet(crate::world::Fleet {
            location: f.system,
            capital_ships: Vec::new(),
            fighters: vec![
                crate::world::FighterEntry {
                    class: x_wing,
                    count: 0,
                    carrier: None,
                },
                crate::world::FighterEntry {
                    class: a_wing,
                    count: 1,
                    carrier: None,
                },
            ],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        assert_eq!(
            check(
                &f.world,
                Faction::Alliance,
                ScrapTarget::FleetFighter {
                    fleet: other,
                    class: x_wing
                }
            ),
            Err(ScrapRefusal::Unresolved)
        );
    }

    #[test]
    fn a_squadron_on_a_planet_is_scrapped_one_at_a_time() {
        // The squadron's check is FUN_005575f0 (slot +0x5c, the object
        // check FUN_004f9860); FUN_00530270 refunds +0x48 / 2 = 9 / 2 = 4.
        let mut f = fixture();
        let x_wing = f.world.fighter_classes.insert(crate::world::FighterClass {
            is_alliance: true,
            refined_material_cost: 9,
            ..Default::default()
        });
        crate::squadrons::add(&mut f.world.systems[f.system].fighters, x_wing, 2);
        let target = ScrapTarget::PlanetFighter {
            system: f.system,
            class: x_wing,
        };
        let mut production = ProductionState::new();

        assert_eq!(
            check(&f.world, Faction::Empire, target),
            Err(ScrapRefusal::NotOwnSide)
        );
        let done = scrap(
            &mut f.world,
            &mut production,
            &mut TroopTransportState::new(),
            Faction::Alliance,
            target,
        )
        .unwrap();
        assert_eq!(done.refund, 4);
        assert_eq!(
            crate::squadrons::total(&f.world.systems[f.system].fighters),
            1
        );
        scrap(
            &mut f.world,
            &mut production,
            &mut TroopTransportState::new(),
            Faction::Alliance,
            target,
        )
        .unwrap();
        assert!(f.world.systems[f.system].fighters.is_empty());
        assert_eq!(
            check(&f.world, Faction::Alliance, target),
            Err(ScrapRefusal::Unresolved)
        );
        assert_eq!(production.stock(Faction::Alliance).unwrap().refined, 8);
        // An emptied entry, or only another class, does not resolve.
        let a_wing = f.world.fighter_classes.insert(crate::world::FighterClass {
            is_alliance: true,
            ..Default::default()
        });
        f.world.systems[f.system].fighters = vec![
            crate::world::FighterEntry {
                class: x_wing,
                count: 0,
                carrier: None,
            },
            crate::world::FighterEntry {
                class: a_wing,
                count: 1,
                carrier: None,
            },
        ];
        assert_eq!(
            check(&f.world, Faction::Alliance, target),
            Err(ScrapRefusal::Unresolved)
        );
    }

    #[test]
    fn each_facility_kind_and_a_regiment_can_be_scrapped() {
        // Slot +0x58 accepts 0x200 for every facility class
        // (FUN_00539c20) and a regiment (FUN_00504b30).
        let mut f = fixture();
        let yard = class(&mut f.world, 0x2a00_0001, 4);
        let yard = f
            .world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: yard,
                side: Faction::Empire,
                is_shipyard: false,
            });
        f.world.systems[f.system]
            .manufacturing_facilities
            .push(yard);
        let gun = class(&mut f.world, 0x2500_0001, 6);
        let gun = f.world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: gun,
            side: Faction::Empire,
        });
        f.world.systems[f.system].defense_facilities.push(gun);
        let regiment = class(&mut f.world, 0x1000_0001, 9);
        let troop = f.world.troops.insert(TroopUnit {
            class_dat_id: regiment,
            is_alliance: false,
            regiment_strength: 100,
        });
        f.world.systems[f.system].ground_units.push(troop);
        let mut production = ProductionState::new();

        for (target, refund) in [
            (ScrapTarget::Facility(FacilityRef::Manufacturing(yard)), 2),
            (ScrapTarget::Facility(FacilityRef::Defense(gun)), 3),
            (ScrapTarget::Troop(troop), 4),
        ] {
            let done = scrap(
                &mut f.world,
                &mut production,
                &mut TroopTransportState::new(),
                Faction::Empire,
                target,
            )
            .unwrap();
            assert_eq!(done.refund, refund, "{target:?}");
        }

        let sys = &f.world.systems[f.system];
        assert!(sys.manufacturing_facilities.is_empty());
        assert!(sys.defense_facilities.is_empty());
        assert!(sys.ground_units.is_empty());
        assert!(f.world.troops.is_empty());
        assert_eq!(production.stock(Faction::Empire).unwrap().refined, 9);
    }

    #[test]
    fn another_side_s_object_is_refused_and_kept() {
        // FUN_004f9860: the object's side must be the order's (status 1).
        let mut f = fixture();
        let target = ScrapTarget::Facility(mine(&mut f, Faction::Alliance, 7));
        let mut production = ProductionState::new();

        for side in [Faction::Empire, Faction::Neutral] {
            assert_eq!(
                scrap(
                    &mut f.world,
                    &mut production,
                    &mut TroopTransportState::new(),
                    side,
                    target
                ),
                Err(ScrapRefusal::NotOwnSide)
            );
        }
        assert_eq!(ScrapRefusal::NotOwnSide.code(), 1);
        assert_eq!(f.world.production_facilities.len(), 1);
        assert_eq!(production.stock(Faction::Empire).unwrap().refined, 0);
    }

    #[test]
    fn a_neutral_facility_is_no_side_s_to_scrap() {
        // FUN_004f9860: a side of 0 never matches the order's.
        let mut f = fixture();
        let target = ScrapTarget::Facility(mine(&mut f, Faction::Neutral, 7));

        assert_eq!(
            check(&f.world, Faction::Neutral, target),
            Err(ScrapRefusal::NotOwnSide)
        );
    }

    #[test]
    fn a_missing_object_or_a_regiment_off_the_planet_does_not_resolve() {
        // FUN_00579790 → FUN_00504e60: status 0x12.
        let mut f = fixture();
        let target = ScrapTarget::Facility(mine(&mut f, Faction::Alliance, 7));
        let mut production = ProductionState::new();
        scrap(
            &mut f.world,
            &mut production,
            &mut TroopTransportState::new(),
            Faction::Alliance,
            target,
        )
        .unwrap();

        assert_eq!(
            check(&f.world, Faction::Alliance, target),
            Err(ScrapRefusal::Unresolved)
        );
        let regiment = class(&mut f.world, 0x1000_0001, 9);
        let aboard = f.world.troops.insert(TroopUnit {
            class_dat_id: regiment,
            is_alliance: true,
            regiment_strength: 100,
        });
        assert_eq!(
            check(&f.world, Faction::Alliance, ScrapTarget::Troop(aboard)),
            Err(ScrapRefusal::Unresolved)
        );
        assert_eq!(ScrapRefusal::Unresolved.code(), 0x12);
    }

    #[test]
    fn a_headquarters_object_refuses_scrap() {
        // FUN_00526940, the 0x20 class's slot +0x5c: status 0x17.
        let mut f = fixture();
        let hq = class(&mut f.world, 0x2000_0001, 8);
        let hq = f.world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: hq,
            side: Faction::Alliance,
        });
        f.world.systems[f.system].defense_facilities.push(hq);

        assert_eq!(
            check(
                &f.world,
                Faction::Alliance,
                ScrapTarget::Facility(FacilityRef::Defense(hq))
            ),
            Err(ScrapRefusal::NotAccepted)
        );
        assert_eq!(ScrapRefusal::NotAccepted.code(), 0x17);
    }

    #[test]
    fn one_refused_member_keeps_the_whole_team() {
        // FUN_0051ff30 validates every command before FUN_005539f0 runs
        // them: the Empire's mine refuses, so the Alliance's stays too.
        let mut f = fixture();
        let own = ScrapTarget::Facility(mine(&mut f, Faction::Alliance, 6));
        let theirs = ScrapTarget::Facility(mine(&mut f, Faction::Empire, 6));
        let mut production = ProductionState::new();

        assert_eq!(
            scrap_team(
                &mut f.world,
                &mut production,
                &mut TroopTransportState::new(),
                Faction::Alliance,
                &[own, theirs]
            ),
            Err(ScrapRefusal::NotOwnSide)
        );
        assert_eq!(f.world.production_facilities.len(), 2);

        let done = scrap_team(
            &mut f.world,
            &mut production,
            &mut TroopTransportState::new(),
            Faction::Alliance,
            &[own],
        )
        .unwrap();
        assert_eq!(done.len(), 1);
        assert_eq!(f.world.production_facilities.len(), 1);
        assert_eq!(production.stock(Faction::Alliance).unwrap().refined, 3);
    }

    #[test]
    fn a_class_with_no_record_refunds_nothing() {
        // FUN_004f2980: no class (+0x2c) gives 0.
        let mut f = fixture();
        let target = ScrapTarget::Facility(mine(&mut f, Faction::Alliance, 7));
        f.world.buildable_classes.clear();
        let mut production = ProductionState::new();

        let done = scrap(
            &mut f.world,
            &mut production,
            &mut TroopTransportState::new(),
            Faction::Alliance,
            target,
        )
        .unwrap();

        assert_eq!(done.refund, 0);
    }
}
