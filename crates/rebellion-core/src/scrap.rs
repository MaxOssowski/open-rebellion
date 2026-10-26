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
/// on a planet and the capital ships.
///
/// port: squadrons aboard a fleet, fleets and regiments aboard a fleet
/// take Scrap in the original (`FUN_005575f0`, `FUN_004fdbc0`) but are not
/// ported yet.
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
    /// The capital ship at `index` in `fleet` (vtables `0x65d650`,
    /// `0x65d930`, `0x65dc10`, slot `+0x5c` `FUN_005575f0`). port: a ship
    /// is named by its place in its fleet.
    Ship {
        fleet: crate::ids::FleetKey,
        index: usize,
    },
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
}

impl ScrapRefusal {
    /// The status code the original reports.
    #[must_use]
    pub const fn code(self) -> u32 {
        // FUN_00504e60 (0x12), FUN_004f9860 (1), FUN_00526940 (0x17).
        match self {
            Self::Unresolved => 0x12,
            Self::NotOwnSide => 1,
            Self::NotAccepted => 0x17,
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
        ScrapTarget::Ship { fleet, index } => {
            let f = world.fleets.get(fleet)?;
            let ship = f.capital_ships.get(index).filter(|s| s.alive)?;
            let c = world.capital_ship_classes.get(ship.class)?;
            (f.location, Faction::of_alliance(f.is_alliance), c.dat_id)
        }
    };
    Some((system, side, class))
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
    side: Faction,
    target: ScrapTarget,
) -> Result<Scrapped, ScrapRefusal> {
    check(world, side, target)?;
    let (system, owner, class) = resolve(world, target).ok_or(ScrapRefusal::Unresolved)?;
    let refund = match target {
        ScrapTarget::PlanetFighter { class, .. } => {
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
        ScrapTarget::Ship { fleet, index } => remove_ship(world, fleet, index),
    }
    if let Some(stock) = production.stock_mut(owner) {
        stock.refined += refund;
    }
    Ok(Scrapped { system, refund })
}

/// Take the ship at `index` out of `fleet`; a fleet left holding nothing
/// disbands (`FUN_004fe630`). The regiments its remaining ships have no
/// room for are lost with the next cargo check
/// ([`crate::troop_transport::TroopTransportState::destroy_untransportable_cargo`]).
pub(crate) fn remove_ship(world: &mut GameWorld, fleet: crate::ids::FleetKey, index: usize) {
    world.fleets[fleet].capital_ships.remove(index);
    disband_if_empty(world, fleet);
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
    side: Faction,
    team: &[ScrapTarget],
) -> Result<Vec<Scrapped>, ScrapRefusal> {
    for &target in team {
        check(world, side, target)?;
    }
    team.iter()
        .map(|&target| scrap(world, production, side, target))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let done = scrap(&mut f.world, &mut production, Faction::Alliance, target).unwrap();

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
        let done = scrap(&mut f.world, &mut production, Faction::Alliance, first).unwrap();
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
        scrap(&mut f.world, &mut production, Faction::Alliance, last).unwrap();
        assert!(!f.world.fleets.contains_key(pair));
        assert!(!f.world.systems[f.system].fleets.contains(&pair));
        let alone = ScrapTarget::Ship {
            fleet: carrier,
            index: 0,
        };
        scrap(&mut f.world, &mut production, Faction::Alliance, alone).unwrap();
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
        let done = scrap(&mut f.world, &mut production, Faction::Alliance, target).unwrap();
        assert_eq!(done.refund, 4);
        assert_eq!(
            crate::squadrons::total(&f.world.systems[f.system].fighters),
            1
        );
        scrap(&mut f.world, &mut production, Faction::Alliance, target).unwrap();
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
            let done = scrap(&mut f.world, &mut production, Faction::Empire, target).unwrap();
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
                scrap(&mut f.world, &mut production, side, target),
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
        scrap(&mut f.world, &mut production, Faction::Alliance, target).unwrap();

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
                Faction::Alliance,
                &[own, theirs]
            ),
            Err(ScrapRefusal::NotOwnSide)
        );
        assert_eq!(f.world.production_facilities.len(), 2);

        let done = scrap_team(&mut f.world, &mut production, Faction::Alliance, &[own]).unwrap();
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

        let done = scrap(&mut f.world, &mut production, Faction::Alliance, target).unwrap();

        assert_eq!(done.refund, 0);
    }
}
