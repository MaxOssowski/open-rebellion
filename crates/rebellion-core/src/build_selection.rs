//! What a production manager can build and what an order costs: the model
//! behind the Build Selection window (`ghidra/notes/manufacturing-build-selection.md`).
//!
//! `FUN_00537ff0` maps a manager to the classes its yards build and
//! `FUN_0052e580` lists those its side has researched; `FUN_00538220`
//! prices an order as each class cost times the quantity and asks the
//! manager for the best completion and deployment times.

use crate::delivery::{transit_days, Delivery};
use crate::ids::{DatId, SystemKey};
use crate::manufacturing::{
    completion_day, BuildableKind, FacilityBuild, ManufacturingState, ProductionArea, QueueItem,
};
use crate::research::{ResearchState, TechType};
use crate::world::GameWorld;

/// The class production family an area's yards build, and the yards' own
/// facility family (MANFACSD `production_family`, `FUN_00537ff0`: ships
/// `0x28`, regiments and special forces `0x29`, facilities `0x2a`).
#[must_use]
pub const fn area_family(area: ProductionArea) -> u8 {
    match area {
        ProductionArea::Shipyard => 0x28,
        ProductionArea::TrainingFacility => 0x29,
        ProductionArea::ConstructionYard => 0x2a,
    }
}

/// The research tree whose level gates an area's classes (side `+0x9c`,
/// `+0xa0`, `+0xa4` in `FUN_0052e580`).
const fn area_tech(area: ProductionArea) -> TechType {
    match area {
        ProductionArea::Shipyard => TechType::Ship,
        ProductionArea::TrainingFacility => TechType::Troop,
        ProductionArea::ConstructionYard => TechType::Facility,
    }
}

/// A catalog class as the kind an order names, for `is_alliance`'s side,
/// by its DAT family byte: TROOPSD `0x10`, SPECFCSD `0x3c`, DEFFACSD
/// `0x22..0x25`, MANFACSD `0x28..0x2a`, PROFACSD `0x2c..0x2d`
/// (`FUN_00458fe0`, `FUN_00537ff0`).
fn catalog_kind(class: DatId, is_alliance: bool) -> Option<BuildableKind> {
    let build = FacilityBuild { class, is_alliance };
    match class.family() {
        0x10 => Some(BuildableKind::Troop(class)),
        0x3c => Some(BuildableKind::SpecialForce(class)),
        0x22..=0x25 => Some(BuildableKind::DefenseFacility(build)),
        0x28..=0x2a => Some(BuildableKind::ManufacturingFacility(build)),
        0x2c..=0x2d => Some(BuildableKind::ProductionFacility(build)),
        _ => None,
    }
}

/// The classes `area`'s manager lists for a side: its family's classes that
/// side builds, has researched and that cost refined material, in `DatId`
/// order. `FUN_0052e580` lists through `FUN_0051cb20` -> `FUN_00567880`,
/// which keeps only classes with a refined cost (`FUN_0053b860` nonzero):
/// the Empire's Bounty Hunters (SPECFCSD 9, cost 0) are never listed (A0
/// build-selection-empire2/12-*: Sluis Van's Training Facility lists seven
/// classes, no Bounty Hunters). hyp: `FUN_0051cb20` walks the class
/// tables in id order.
#[must_use]
pub fn listed_classes(
    world: &GameWorld,
    research: &ResearchState,
    area: ProductionArea,
    is_alliance: bool,
) -> Vec<BuildableKind> {
    let tech = area_tech(area);
    let unlocked = |order| research.is_unlocked(is_alliance, order, tech);
    let serves = |alliance: bool, empire: bool| if is_alliance { alliance } else { empire };
    let mut listed: Vec<(u32, BuildableKind)> = Vec::new();
    if area == ProductionArea::Shipyard {
        listed.extend(
            world
                .capital_ship_classes
                .iter()
                .filter(|(_, c)| {
                    serves(c.is_alliance, c.is_empire)
                        && c.refined_material_cost != 0
                        && unlocked(c.research_order)
                })
                .map(|(key, c)| (c.dat_id.raw(), BuildableKind::CapitalShip(key))),
        );
        listed.extend(
            world
                .fighter_classes
                .iter()
                .filter(|(_, c)| {
                    serves(c.is_alliance, c.is_empire)
                        && c.refined_material_cost != 0
                        && unlocked(c.research_order)
                })
                .map(|(key, c)| (c.dat_id.raw(), BuildableKind::Fighter(key))),
        );
    } else {
        listed.extend(
            world
                .buildable_classes
                .iter()
                .filter_map(|(&class, value)| {
                    let kind = catalog_kind(class, is_alliance)?;
                    (ProductionArea::of(kind) == area
                        && value.serves(is_alliance)
                        && value.refined_material_cost != 0
                        && unlocked(value.research_order))
                    .then_some((class.raw(), kind))
                }),
        );
    }
    listed.sort_unstable_by_key(|(id, _)| *id);
    listed.into_iter().map(|(_, kind)| kind).collect()
}

/// A class of a production family as research reads it: its `DatId`, its
/// research order (`+0x50`) and difficulty (`+0x54`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResearchClass {
    pub id: DatId,
    pub order: u32,
    pub difficulty: u32,
}

/// `FUN_00567880`: the classes of `area`'s family that serve a side
/// (`+0x40` Alliance, `+0x44` Empire) and cost refined material
/// (`FUN_0053b860` nonzero), in `DatId` order. hyp: the original walks its
/// class tables in id order.
#[must_use]
pub fn research_classes(
    world: &GameWorld,
    area: ProductionArea,
    is_alliance: bool,
) -> Vec<ResearchClass> {
    let serves = |alliance: bool, empire: bool| if is_alliance { alliance } else { empire };
    let mut classes = Vec::new();
    if area == ProductionArea::Shipyard {
        classes.extend(
            world
                .capital_ship_classes
                .values()
                .filter(|c| serves(c.is_alliance, c.is_empire) && c.refined_material_cost != 0)
                .map(|c| ResearchClass {
                    id: c.dat_id,
                    order: c.research_order,
                    difficulty: c.research_difficulty,
                }),
        );
        classes.extend(
            world
                .fighter_classes
                .values()
                .filter(|c| serves(c.is_alliance, c.is_empire) && c.refined_material_cost != 0)
                .map(|c| ResearchClass {
                    id: c.dat_id,
                    order: c.research_order,
                    difficulty: c.research_difficulty,
                }),
        );
    } else {
        classes.extend(
            world
                .buildable_classes
                .iter()
                .filter(|(&class, value)| {
                    catalog_kind(class, is_alliance)
                        .is_some_and(|kind| ProductionArea::of(kind) == area)
                        && value.serves(is_alliance)
                        && value.refined_material_cost != 0
                })
                .map(|(&class, value)| ResearchClass {
                    id: class,
                    order: value.research_order,
                    difficulty: value.research_difficulty,
                }),
        );
    }
    classes.sort_unstable_by_key(|class| class.id.raw());
    classes
}

/// One unit's refined material and maintenance costs (class `+0x48`,
/// `+0x4c`; `FUN_0053b860`, `FUN_0053b870`).
#[must_use]
pub fn unit_costs(world: &GameWorld, kind: BuildableKind) -> Option<(u32, u32)> {
    match kind {
        BuildableKind::CapitalShip(key) => world
            .capital_ship_classes
            .get(key)
            .map(|c| (c.refined_material_cost, c.maintenance_cost)),
        BuildableKind::Fighter(key) => world
            .fighter_classes
            .get(key)
            .map(|c| (c.refined_material_cost, c.maintenance_cost)),
        _ => world
            .buildable_classes
            .get(&kind.class_dat_id()?)
            .map(|c| (c.refined_material_cost, c.maintenance_cost)),
    }
}

/// The days per unit of progress of each of `area`'s yards at `system` on
/// a side (`FUN_00520b70`: the yard class's `+0x5c`, MANFACSD
/// `processing_rate`). port: every yard counts; the original skips one
/// whose `+0x60` bit 0 is set (`FUN_00528b30`), which is untraced.
#[must_use]
pub fn yard_periods(
    world: &GameWorld,
    system: SystemKey,
    area: ProductionArea,
    is_alliance: bool,
) -> Vec<u32> {
    let Some(value) = world.systems.get(system) else {
        return Vec::new();
    };
    value
        .manufacturing_facilities
        .iter()
        .filter_map(|&key| world.manufacturing_facilities.get(key))
        .filter(|yard| {
            yard.side == crate::dat::Faction::of_alliance(is_alliance)
                && yard.class_dat_id.family() == area_family(area)
        })
        .filter_map(|yard| world.buildable_classes.get(&yard.class_dat_id))
        .map(|class| class.processing_rate)
        .collect()
}

/// What Build Selection shows for an order (`FUN_00538220`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildEstimate {
    /// Refined material, the class's times the quantity.
    pub refined_material: u32,
    /// Maintenance, the class's times the quantity.
    pub maintenance: u32,
    /// Best time to completion, in days; `None` when the order is not
    /// valid, which shows "n/a" in all four values and disables Confirm
    /// (`FUN_00439160`): the manager has no yard to build with, the
    /// order's maintenance exceeds its side's ([`exceeds_maintenance`]),
    /// or its destination has no room for the facilities
    /// ([`lacks_facility_room`]).
    pub completion: Option<u32>,
    /// Best time to deployment: the days from the manager's system to its
    /// destination.
    pub deployment: u32,
}

/// `FUN_0052e970`, from the build order's check (`FUN_005385f0`): an order
/// whose maintenance, the class's times the quantity, is above the side's
/// unallocated maintenance (side `+0x58` less `+0x5c`) sets the status
/// `{0xf3, 1}`. An order that costs no maintenance always passes.
#[must_use]
pub fn exceeds_maintenance(maintenance: u32, unallocated: i32) -> bool {
    maintenance != 0 && i64::from(maintenance) > i64::from(unallocated)
}

/// A facility kind's place in a system's room (`FUN_0054f5b0`'s ranges
/// `0x20..0x30` and `0x2c..0x2d`): `Some(true)` for a mine (PROFACSD family
/// `0x2c`), `Some(false)` for any other facility, `None` for a ship, a
/// squadron, a regiment or a special force.
fn facility_slot(kind: BuildableKind) -> Option<bool> {
    match kind {
        BuildableKind::ProductionFacility(build) => Some(build.class.family() == 0x2c),
        BuildableKind::DefenseFacility(_) | BuildableKind::ManufacturingFacility(_) => Some(false),
        _ => None,
    }
}

/// Whether `count` units of the facility `kind` find no room at `site`,
/// which makes the order not valid (`FUN_005385f0`: per unit
/// `FUN_00551a20` -> `FUN_005504d0`, `FUN_00552300`, `FUN_00552dd0`).
///
/// The system's room (vtable `0x0065e640` slot `+0x78`, `FUN_00507750`):
/// - for families `0x20..0x2f`: its energy (`+0x5c`, `FUN_00509650`) less
///   its facilities of those families (`FUN_00539d70`), refused with
///   `{0x90, 1}`;
/// - for a mine (`0x2c`), also: its raw materials (`+0x64`,
///   `FUN_00509660`) less its mines (`FUN_0052d610`), refused with
///   `{0x90, 2}`.
///
/// The walk has no side filter and mode 3, existing (the route context's
/// `+0x20`, `FUN_00551100`, is the 3 `FUN_005385f0` passes to
/// `FUN_00551190`), so both sides' facilities count. A product exists from
/// its order (`FUN_0052a140`, `FUN_004f7d50(.., 1, 0)`: created), so units
/// queued for `site` and on their way there count too. Each unit takes one
/// slot: `FUN_005504d0` asks one per unit, and `FUN_00550eb0` writes the
/// lowered room back (`FUN_0054fca0`, `FUN_0054ff00`).
#[must_use]
pub fn lacks_facility_room(
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    en_route: &[Delivery],
    site: SystemKey,
    kind: BuildableKind,
    count: u32,
) -> bool {
    let Some(mine) = facility_slot(kind) else {
        return false;
    };
    let Some(system) = world.systems.get(site) else {
        return true;
    };
    let built = system
        .defense_facilities
        .iter()
        .filter(|key| world.defense_facilities.contains_key(**key))
        .map(|_| false)
        .chain(
            system
                .manufacturing_facilities
                .iter()
                .filter(|key| world.manufacturing_facilities.contains_key(**key))
                .map(|_| false),
        )
        .chain(
            system
                .production_facilities
                .iter()
                .filter_map(|key| world.production_facilities.get(*key))
                .map(|facility| facility.is_mine),
        );
    let queued = manufacturing
        .queues()
        .iter()
        .flat_map(|((at, _), queue)| queue.items().iter().map(move |item| (*at, item)))
        .filter(|(at, item)| item.destination.unwrap_or(*at) == site)
        .filter_map(|(_, item)| facility_slot(item.kind));
    let travelling = en_route
        .iter()
        .filter(|delivery| delivery.destination == site)
        .filter_map(|delivery| facility_slot(delivery.kind));
    let (mut facilities, mut mines) = (0_u32, 0_u32);
    for is_mine in built.chain(queued).chain(travelling) {
        facilities += 1;
        mines += u32::from(is_mine);
    }
    let short = |slots: u8, taken: u32| u32::from(slots).saturating_sub(taken) < count;
    short(system.total_energy, facilities) || (mine && short(system.raw_materials, mines))
}

/// Price `count` units of `kind` at `system`'s `area` for a side whose
/// unallocated maintenance is `unallocated`
/// ([`crate::side_resources::MaintenanceTotals::unallocated`]), with
/// `en_route` the products on their way ([`crate::delivery::DeliveryState::en_route`]).
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "FUN_00538220 reads the manager, the order and the side's state together."
)]
pub fn estimate(
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    en_route: &[Delivery],
    system: SystemKey,
    area: ProductionArea,
    kind: BuildableKind,
    count: u32,
    is_alliance: bool,
    unallocated: i32,
) -> BuildEstimate {
    let (refined, maintenance) = unit_costs(world, kind).unwrap_or_default();
    let destination = manufacturing.destination(system, area).unwrap_or(system);
    let maintenance = maintenance.saturating_mul(count);
    BuildEstimate {
        refined_material: refined.saturating_mul(count),
        maintenance,
        completion: completion_day(
            &yard_periods(world, system, area, is_alliance),
            refined.saturating_mul(count),
        )
        .filter(|_| {
            !exceeds_maintenance(maintenance, unallocated)
                && !lacks_facility_room(world, manufacturing, en_route, destination, kind, count)
        }),
        deployment: transit_days(world, kind, system, destination),
    }
}

/// The queue items a confirmed order adds: `count` units, each needing
/// its class refined cost in build points (`FUN_0052a8f0`). `None` when no
/// yard can build (`FUN_00527d00`).
#[must_use]
pub fn order_items(
    world: &GameWorld,
    system: SystemKey,
    area: ProductionArea,
    kind: BuildableKind,
    count: u32,
    is_alliance: bool,
) -> Option<Vec<QueueItem>> {
    let (refined, _) = unit_costs(world, kind)?;
    completion_day(&yard_periods(world, system, area, is_alliance), refined)?;
    Some((0..count).map(|_| QueueItem::new(kind, refined)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{ExplorationStatus, Faction};
    use crate::world::{BuildableClass, ControlKind, ManufacturingFacilityInstance, System};

    fn system(world: &mut GameWorld, x: u16) -> SystemKey {
        world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "Bortras".into(),
            sector: crate::ids::SectorKey::default(),
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
            control: ControlKind::Controlled(Faction::Alliance),
        })
    }

    fn class(
        world: &mut GameWorld,
        id: u32,
        sides: (bool, bool),
        costs: (u32, u32),
        order: u32,
        rate: u32,
    ) {
        world.buildable_classes.insert(
            DatId::new(id),
            BuildableClass {
                name: String::new(),
                is_alliance: sides.0,
                is_empire: sides.1,
                refined_material_cost: costs.0,
                maintenance_cost: costs.1,
                research_order: order,
                research_difficulty: 0,
                processing_rate: rate,
                ..BuildableClass::default()
            },
        );
    }

    fn yard(world: &mut GameWorld, at: SystemKey, id: u32, is_alliance: bool) {
        let key = world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: DatId::new(id),
                side: crate::dat::Faction::of_alliance(is_alliance),
                is_shipyard: false,
            });
        world.systems[at].manufacturing_facilities.push(key);
    }

    /// MANFACSD: construction yard 4 days, advanced construction yard 2
    /// (research order 2); TROOPSD regiment 1 Alliance, 6 Empire; SPECFCSD
    /// unit 1 Alliance; DEFFACSD 4 (Death Star shield) Empire only.
    /// More unallocated maintenance than any order here costs.
    const ROOMY: i32 = 1_000;

    fn catalog() -> GameWorld {
        let mut world = GameWorld::default();
        class(&mut world, 0x2a00_0003, (true, true), (10, 10), 0, 4);
        class(&mut world, 0x2a00_0006, (true, true), (10, 10), 2, 2);
        class(&mut world, 0x2900_0002, (true, true), (10, 10), 0, 4);
        class(&mut world, 0x1000_0001, (true, false), (8, 6), 0, 0);
        class(&mut world, 0x1000_0006, (false, true), (9, 6), 0, 0);
        class(&mut world, 0x3c00_0001, (true, false), (1, 1), 0, 0);
        class(&mut world, 0x2500_0004, (false, true), (25, 20), 3, 0);
        class(&mut world, 0x2c00_0001, (true, true), (20, 0), 0, 5);
        world
    }

    // FUN_0052e580: the manager's family (0x29 regiments and special forces,
    // 0x2a facilities) for its side, up to its research level.
    #[test]
    fn a_manager_lists_its_familys_classes_its_side_has_researched() {
        let world = catalog();
        let mut research = ResearchState::new();
        let yard = |id| {
            BuildableKind::ManufacturingFacility(FacilityBuild {
                class: DatId::new(id),
                is_alliance: true,
            })
        };
        let mine = BuildableKind::ProductionFacility(FacilityBuild {
            class: DatId::new(0x2c00_0001),
            is_alliance: true,
        });
        assert_eq!(
            listed_classes(&world, &research, ProductionArea::ConstructionYard, true),
            [yard(0x2900_0002), yard(0x2a00_0003), mine]
        );
        research.alliance.facility = 2;
        assert_eq!(
            listed_classes(&world, &research, ProductionArea::ConstructionYard, true),
            [
                yard(0x2900_0002),
                yard(0x2a00_0003),
                yard(0x2a00_0006),
                mine
            ]
        );
        assert_eq!(
            listed_classes(&world, &research, ProductionArea::TrainingFacility, true),
            [
                BuildableKind::Troop(DatId::new(0x1000_0001)),
                BuildableKind::SpecialForce(DatId::new(0x3c00_0001))
            ]
        );
        assert_eq!(
            listed_classes(&world, &research, ProductionArea::TrainingFacility, false),
            [BuildableKind::Troop(DatId::new(0x1000_0006))]
        );
        // The Empire's facility list waits on its own level for the shield.
        research.empire.facility = 3;
        assert!(
            listed_classes(&world, &research, ProductionArea::ConstructionYard, false).contains(
                &BuildableKind::DefenseFacility(FacilityBuild {
                    class: DatId::new(0x2500_0004),
                    is_alliance: false,
                })
            )
        );
    }

    // FUN_0052e580 with family 0x28: the side's researched ship and fighter
    // classes, in id order; FUN_0053b860/FUN_0053b870 their costs.
    #[test]
    fn a_shipyard_lists_its_sides_researched_ships_and_fighters() {
        use crate::world::{CapitalShipClass, FighterClass};
        let mut world = GameWorld::default();
        let ship = |id, alliance, empire, order| CapitalShipClass {
            dat_id: DatId::new(id),
            is_alliance: alliance,
            is_empire: empire,
            refined_material_cost: 30,
            maintenance_cost: 25,
            research_order: order,
            ..CapitalShipClass::default()
        };
        let corvette = world
            .capital_ship_classes
            .insert(ship(0x1400_0002, true, false, 0));
        world
            .capital_ship_classes
            .insert(ship(0x1400_0001, false, true, 0));
        world
            .capital_ship_classes
            .insert(ship(0x1400_0003, true, false, 4));
        let wing = |id, alliance, empire, order| FighterClass {
            dat_id: DatId::new(id),
            is_alliance: alliance,
            is_empire: empire,
            refined_material_cost: 4,
            maintenance_cost: 3,
            research_order: order,
            ..FighterClass::default()
        };
        let x_wing = world
            .fighter_classes
            .insert(wing(0x1a00_0001, true, false, 0));
        world
            .fighter_classes
            .insert(wing(0x1a00_0002, false, true, 0));
        world
            .fighter_classes
            .insert(wing(0x1a00_0003, true, false, 2));
        let research = ResearchState::new();

        assert_eq!(
            listed_classes(&world, &research, ProductionArea::Shipyard, true),
            [
                BuildableKind::CapitalShip(corvette),
                BuildableKind::Fighter(x_wing)
            ]
        );
        assert_eq!(
            unit_costs(&world, BuildableKind::CapitalShip(corvette)),
            Some((30, 25))
        );
        assert_eq!(
            unit_costs(&world, BuildableKind::Fighter(x_wing)),
            Some((4, 3))
        );
    }

    // FUN_00538220: both costs times the quantity; FUN_00528d30 the best
    // completion from the manager's yards; FUN_00555f90 the trip to its
    // destination.
    #[test]
    fn an_estimate_prices_the_quantity_and_times_the_managers_yards() {
        let mut world = catalog();
        let home = system(&mut world, 0);
        let away = system(&mut world, 200);
        yard(&mut world, home, 0x2900_0002, true);
        yard(&mut world, home, 0x2900_0002, false);
        let regiment = BuildableKind::Troop(DatId::new(0x1000_0001));
        let mut manufacturing = ManufacturingState::new();

        let at_home = estimate(
            &world,
            &manufacturing,
            &[],
            home,
            ProductionArea::TrainingFacility,
            regiment,
            3,
            true,
            ROOMY,
        );
        assert_eq!(
            at_home,
            BuildEstimate {
                refined_material: 24,
                maintenance: 18,
                completion: Some(96),
                deployment: 0,
            }
        );

        manufacturing.set_destination(home, ProductionArea::TrainingFacility, away);
        let sent = estimate(
            &world,
            &manufacturing,
            &[],
            home,
            ProductionArea::TrainingFacility,
            regiment,
            1,
            true,
            ROOMY,
        );
        assert_eq!(sent.completion, Some(32));
        assert!(sent.deployment > 0);

        // No training facility of the side: nothing can build it.
        let empire = estimate(
            &world,
            &manufacturing,
            &[],
            away,
            ProductionArea::TrainingFacility,
            regiment,
            1,
            true,
            ROOMY,
        );
        assert_eq!(empire.completion, None);
    }

    // FUN_0052e970: the order's maintenance against side +0x58 less +0x5c;
    // status {0xf3, 1} makes FUN_00439160 show "n/a" and grey Confirm.
    #[test]
    fn an_order_whose_maintenance_exceeds_the_sides_unallocated_is_not_valid() {
        let mut world = catalog();
        let home = system(&mut world, 0);
        yard(&mut world, home, 0x2900_0002, true);
        let regiment = BuildableKind::Troop(DatId::new(0x1000_0001));
        let manufacturing = ManufacturingState::new();
        let priced = |count, unallocated| {
            estimate(
                &world,
                &manufacturing,
                &[],
                home,
                ProductionArea::TrainingFacility,
                regiment,
                count,
                true,
                unallocated,
            )
        };

        // Six maintenance a regiment: three take 18.
        assert_eq!(priced(3, 18).completion, Some(96));
        let refused = priced(3, 17);
        assert_eq!(refused.completion, None);
        assert_eq!((refused.refined_material, refused.maintenance), (24, 18));
        assert_eq!(priced(1, -402).completion, None);

        assert!(!exceeds_maintenance(0, -402));
        assert!(!exceeds_maintenance(10, 10));
        assert!(exceeds_maintenance(11, 10));
    }

    // FUN_005385f0 -> FUN_00551a20 -> FUN_00552300/FUN_00552dd0 against
    // the system's room (FUN_00507750): energy (+0x5c) less every existing
    // facility of 0x20..0x2f, both sides', and for a mine also raw
    // materials (+0x64) less the mines; refused {0x90, 1} and {0x90, 2}.
    // A0 2026-11-13-build-confirm/21, 23: Jerijador, one construction yard,
    // Construction Yard and GenCore Level I at number 1 read n/a with
    // Confirm grey.
    #[test]
    fn a_facility_order_needs_a_free_energy_slot_per_unit_at_its_destination() {
        use crate::world::{DefenseFacilityInstance, ProductionFacilityInstance};
        let mut world = catalog();
        let home = system(&mut world, 0);
        let away = system(&mut world, 200);
        yard(&mut world, home, 0x2a00_0003, true);
        yard(&mut world, home, 0x2900_0002, true);
        world.systems[home].total_energy = 4;
        world.systems[home].raw_materials = 1;
        world.systems[away].total_energy = 1;
        world.systems[away].raw_materials = 1;
        // The other side's shield counts against the same energy.
        let shield = world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: DatId::new(0x2500_0004),
            side: Faction::Empire,
        });
        world.systems[home].defense_facilities.push(shield);
        let facility = BuildableKind::ManufacturingFacility(FacilityBuild {
            class: DatId::new(0x2a00_0003),
            is_alliance: true,
        });
        let mine = BuildableKind::ProductionFacility(FacilityBuild {
            class: DatId::new(0x2c00_0001),
            is_alliance: true,
        });
        let regiment = BuildableKind::Troop(DatId::new(0x1000_0001));
        let yards = ProductionArea::ConstructionYard;
        let valid = |world: &GameWorld,
                     manufacturing: &ManufacturingState,
                     en_route: &[Delivery],
                     area,
                     kind,
                     count| {
            estimate(
                world,
                manufacturing,
                en_route,
                home,
                area,
                kind,
                count,
                true,
                ROOMY,
            )
            .completion
            .is_some()
        };
        let mut manufacturing = ManufacturingState::new();

        // Energy 4 less three facilities (two yards, the shield): one slot.
        assert!(valid(&world, &manufacturing, &[], yards, facility, 1));
        assert!(!valid(&world, &manufacturing, &[], yards, facility, 2));
        // Regiments take no slot.
        let training = ProductionArea::TrainingFacility;
        assert!(valid(&world, &manufacturing, &[], training, regiment, 5));
        // A mine takes an energy slot and a raw-material deposit.
        assert!(valid(&world, &manufacturing, &[], yards, mine, 1));
        let built = world
            .production_facilities
            .insert(ProductionFacilityInstance {
                class_dat_id: DatId::new(0x2c00_0001),
                side: Faction::Alliance,
                is_mine: true,
            });
        world.systems[away].production_facilities.push(built);
        assert!(lacks_facility_room(
            &world,
            &manufacturing,
            &[],
            away,
            mine,
            1
        ));
        world.systems[away].total_energy = 2;
        assert!(lacks_facility_room(
            &world,
            &manufacturing,
            &[],
            away,
            mine,
            1
        ));
        assert!(!lacks_facility_room(
            &world,
            &manufacturing,
            &[],
            away,
            facility,
            1
        ));
        world.systems[away].raw_materials = 2;
        assert!(!lacks_facility_room(
            &world,
            &manufacturing,
            &[],
            away,
            mine,
            1
        ));

        // A unit queued for the system holds its slot from its order on
        // (FUN_0052a140), here one built at away for home.
        manufacturing.enqueue(away, QueueItem::new(facility, 10).delivered_to(home));
        assert!(!valid(&world, &manufacturing, &[], yards, facility, 1));
        let mut manufacturing = ManufacturingState::new();
        // So does one on its way there.
        let travelling = Delivery {
            origin: away,
            destination: home,
            arrival_tick: 50,
            kind: facility,
            side: Faction::Alliance,
        };
        let en_route = [travelling];
        assert!(!valid(
            &world,
            &manufacturing,
            &en_route,
            yards,
            facility,
            1
        ));
        // The room is the destination's (FUN_004c27b0, the TargetKey):
        // away has energy 2 less its mine.
        manufacturing.set_destination(home, yards, away);
        assert!(valid(&world, &manufacturing, &en_route, yards, facility, 1));
        assert!(!valid(
            &world,
            &manufacturing,
            &en_route,
            yards,
            facility,
            2
        ));
    }

    #[test]
    fn an_order_queues_each_unit_at_its_class_refined_cost() {
        let mut world = catalog();
        let home = system(&mut world, 0);
        yard(&mut world, home, 0x2a00_0003, true);
        yard(&mut world, home, 0x2a00_0006, true);
        let kind = BuildableKind::ManufacturingFacility(FacilityBuild {
            class: DatId::new(0x2a00_0003),
            is_alliance: true,
        });
        // FUN_0052a8f0: RequiredPointCount is the class refined cost.
        let items = order_items(
            &world,
            home,
            ProductionArea::ConstructionYard,
            kind,
            2,
            true,
        )
        .expect("the yards can build");
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .all(|item| item.cost == 10 && item.kind == kind));
        assert!(order_items(
            &world,
            home,
            ProductionArea::TrainingFacility,
            kind,
            1,
            true
        )
        .is_none());
    }
}
