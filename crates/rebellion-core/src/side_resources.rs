//! A side's maintenance totals: the capacity its mines and refineries
//! give and the upkeep its forces charge
//! (`ghidra/notes/side-resource-monitors.md`).

use crate::dat::Faction;
use crate::ids::{DatId, SystemKey};
use crate::manufacturing::{BuildableKind, ManufacturingState};
use crate::world::GameWorld;

/// The capacity every mine and every refinery has from creation: slot
/// `+0x114` of both classes (`0x52d100`, `0x52d540`) sets the pair to
/// `(50, 0)` with `FUN_0051c900(0x32, 0)` and `FUN_0055a6e0`.
pub const FACILITY_CAPACITY: i32 = 50;

/// The Maintenance monitor shows TEXTSTRA 4866 "MAX" above this value
/// (`FUN_00429200`, command `0x14e`).
pub const MONITOR_MAX: i32 = 99_999;

/// One side's totals as `FUN_0052fff0` leaves them on the side object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MaintenanceTotals {
    /// Side `+0x60`: the summed capacity of the side's counted mines.
    pub mine_capacity: i32,
    /// Side `+0x68`: the same for its refineries.
    pub refinery_capacity: i32,
    /// Side `+0x70` and `+0x74`: the summed class maintenance cost.
    pub upkeep: i32,
}

impl MaintenanceTotals {
    /// Side `+0x58`: the smaller of the two pools (`FUN_005323c0`).
    #[must_use]
    pub fn capacity(self) -> i32 {
        self.mine_capacity.min(self.refinery_capacity)
    }

    /// The Maintenance monitor's value: side `+0x58` less `+0x74`
    /// (`FUN_00422620`).
    #[must_use]
    pub fn available(self) -> i32 {
        self.capacity() - self.upkeep
    }

    /// `FUN_0052f670`: the side is over its limit when upkeep exceeds
    /// capacity.
    #[must_use]
    pub fn is_over_limit(self) -> bool {
        self.upkeep > self.capacity()
    }
}

/// `side`'s totals. `blocked` tells whether a system is in uprising or
/// blockaded (`+0x88 & 0x24`): `FUN_0050bbb0` then sets facility `+0x60`
/// bit 0, and `FUN_0052fff0` leaves such a mine or refinery out of the
/// pools. Upkeep still counts it.
///
/// port: a facility, ship or unit exists in the world only once completed,
/// so "usable" (`+0x50` bit 0) is every facility present, and the objects
/// under construction (`FUN_0052a140` creates them at the order) are the
/// queued items of the side's systems.
#[must_use]
pub fn maintenance_totals(
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    side: Faction,
    blocked: impl Fn(SystemKey) -> bool,
) -> MaintenanceTotals {
    let class_cost = |class: DatId| {
        world
            .buildable_classes
            .get(&class)
            .map_or(0, |value| value.maintenance_cost)
    };
    let ship_cost = |class| {
        world
            .capital_ship_classes
            .get(class)
            .map_or(0, |value| value.maintenance_cost)
    };
    let fighter_cost = |class| {
        world
            .fighter_classes
            .get(class)
            .map_or(0, |value| value.maintenance_cost)
    };
    let is_alliance = match side {
        Faction::Alliance => Some(true),
        Faction::Empire => Some(false),
        Faction::Neutral => None,
    };

    let mut upkeep = 0u32;
    let (mut mines, mut refineries) = (0i32, 0i32);

    for fleet in world.fleets.values() {
        if Some(fleet.is_alliance) != is_alliance {
            continue;
        }
        for ship in fleet.capital_ships.iter().filter(|ship| ship.alive) {
            upkeep += ship_cost(ship.class);
        }
        for entry in &fleet.fighters {
            upkeep += fighter_cost(entry.class) * entry.count;
        }
    }
    // The squadrons on the planets (ghidra/notes/planet-squadrons.md).
    for entry in world.systems.values().flat_map(|system| &system.fighters) {
        let alliance = world
            .fighter_classes
            .get(entry.class)
            .map(|value| value.is_alliance);
        if alliance.is_some() && alliance == is_alliance {
            upkeep += fighter_cost(entry.class) * entry.count;
        }
    }
    for troop in world.troops.values() {
        if Some(troop.is_alliance) == is_alliance {
            upkeep += class_cost(troop.class_dat_id);
        }
    }
    for unit in world.special_forces.values() {
        if Some(unit.is_alliance) == is_alliance {
            upkeep += class_cost(unit.class_dat_id);
        }
    }
    for facility in world.defense_facilities.values() {
        if facility.side == side {
            upkeep += class_cost(facility.class_dat_id);
        }
    }
    for facility in world.manufacturing_facilities.values() {
        if facility.side == side {
            upkeep += class_cost(facility.class_dat_id);
        }
    }
    for (system, value) in &world.systems {
        let counted = !blocked(system);
        for &key in &value.production_facilities {
            let Some(facility) = world.production_facilities.get(key) else {
                continue;
            };
            if facility.side != side {
                continue;
            }
            upkeep += class_cost(facility.class_dat_id);
            if counted {
                if facility.is_mine {
                    mines += 1;
                } else {
                    refineries += 1;
                }
            }
        }
    }
    for (&(system, _), queue) in manufacturing.queues() {
        let holder = world
            .systems
            .get(system)
            .and_then(|value| GameWorld::facility_holder(value.control));
        if holder != Some(side) {
            continue;
        }
        for item in queue.items() {
            upkeep += match item.kind {
                BuildableKind::CapitalShip(class) => ship_cost(class),
                BuildableKind::Fighter(class) => fighter_cost(class),
                kind => kind.class_dat_id().map_or(0, class_cost),
            };
        }
    }

    MaintenanceTotals {
        mine_capacity: mines * FACILITY_CAPACITY,
        refinery_capacity: refineries * FACILITY_CAPACITY,
        upkeep: i32::try_from(upkeep).unwrap_or(i32::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::ExplorationStatus;
    use crate::manufacturing::{FacilityBuild, QueueItem};
    use crate::world::{
        BuildableClass, CapitalShipClass, ControlKind, DefenseFacilityInstance, FighterClass,
        FighterEntry, Fleet, ManufacturingFacilityInstance, ProductionFacilityInstance,
        ShipInstance, SpecialForceUnit, System, TroopUnit,
    };

    const MINE: u32 = 0x2c00_0001;
    const REFINERY: u32 = 0x2d00_0002;

    fn system(world: &mut GameWorld, control: ControlKind) -> SystemKey {
        world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "Here".into(),
            sector: crate::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 10,
            raw_materials: 5,
            espionage_rating: 0.0,
            fleets: Vec::new(),
            ground_units: Vec::new(),
            special_forces: Vec::new(),
            defense_facilities: Vec::new(),
            manufacturing_facilities: Vec::new(),
            production_facilities: Vec::new(),
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control,
        })
    }

    fn class(world: &mut GameWorld, id: u32, maintenance_cost: u32) -> DatId {
        let class = DatId::new(id);
        world.buildable_classes.insert(
            class,
            BuildableClass {
                maintenance_cost,
                ..BuildableClass::default()
            },
        );
        class
    }

    fn production(world: &mut GameWorld, at: SystemKey, id: u32, side: Faction, count: usize) {
        for _ in 0..count {
            let key = world
                .production_facilities
                .insert(ProductionFacilityInstance {
                    class_dat_id: DatId::new(id),
                    side,
                    is_mine: id == MINE,
                });
            world.systems[at].production_facilities.push(key);
        }
    }

    fn totals(world: &GameWorld, side: Faction) -> MaintenanceTotals {
        maintenance_totals(world, &ManufacturingState::new(), side, |_| false)
    }

    #[test]
    fn capacity_is_50_for_each_mine_and_refinery_pair() {
        // 0x52d100 / 0x52d540 give every mine and refinery (50, 0);
        // FUN_005323c0 takes the smaller of the side's two sums.
        let mut world = GameWorld::default();
        let here = system(&mut world, ControlKind::Controlled(Faction::Empire));
        production(&mut world, here, MINE, Faction::Empire, 3);
        production(&mut world, here, REFINERY, Faction::Empire, 5);

        let empire = totals(&world, Faction::Empire);
        assert_eq!(empire.mine_capacity, 150);
        assert_eq!(empire.refinery_capacity, 250);
        assert_eq!(empire.capacity(), 150);
    }

    #[test]
    fn a_side_counts_only_its_own_mines_and_refineries() {
        // FUN_0052fff0 tests (object +0x24 ^ side +0x24) & 0xc0.
        let mut world = GameWorld::default();
        let here = system(&mut world, ControlKind::Controlled(Faction::Empire));
        production(&mut world, here, MINE, Faction::Empire, 2);
        production(&mut world, here, REFINERY, Faction::Empire, 2);
        production(&mut world, here, MINE, Faction::Neutral, 4);
        production(&mut world, here, REFINERY, Faction::Alliance, 4);

        assert_eq!(totals(&world, Faction::Empire).capacity(), 100);
        assert_eq!(totals(&world, Faction::Alliance).capacity(), 0);
        assert_eq!(totals(&world, Faction::Alliance).refinery_capacity, 200);
    }

    #[test]
    fn a_mine_in_a_blocked_system_gives_no_capacity_but_keeps_its_upkeep() {
        // FUN_0050bbb0 sets facility +0x60 bit 0 from system +0x88 & 0x24
        // (uprising or blockade); FUN_0052fff0 then skips its pair but has
        // already added its class cost.
        let mut world = GameWorld::default();
        class(&mut world, MINE, 2);
        class(&mut world, REFINERY, 3);
        let free = system(&mut world, ControlKind::Controlled(Faction::Alliance));
        let blocked = system(&mut world, ControlKind::Controlled(Faction::Alliance));
        production(&mut world, free, MINE, Faction::Alliance, 1);
        production(&mut world, free, REFINERY, Faction::Alliance, 1);
        production(&mut world, blocked, MINE, Faction::Alliance, 2);
        production(&mut world, blocked, REFINERY, Faction::Alliance, 2);

        let alliance = maintenance_totals(
            &world,
            &ManufacturingState::new(),
            Faction::Alliance,
            |at| at == blocked,
        );
        assert_eq!(alliance.mine_capacity, 50);
        assert_eq!(alliance.refinery_capacity, 50);
        assert_eq!(alliance.upkeep, 3 * 2 + 3 * 3);
    }

    #[test]
    fn upkeep_sums_the_class_cost_of_everything_the_side_owns() {
        // FUN_0052fff0 adds FUN_004f2990 (class +0x4c) for every existing
        // object of the side in families 0x10..0x3f.
        let mut world = GameWorld::default();
        let here = system(&mut world, ControlKind::Controlled(Faction::Empire));
        let regiment = class(&mut world, 0x1000_0001, 2);
        let commando = class(&mut world, 0x3c00_0001, 3);
        let battery = class(&mut world, 0x2200_0001, 5);
        let yard = class(&mut world, 0x2800_0001, 13);
        let destroyer = world.capital_ship_classes.insert(CapitalShipClass {
            maintenance_cost: 35,
            ..CapitalShipClass::default()
        });
        let tie = world.fighter_classes.insert(FighterClass {
            maintenance_cost: 3,
            ..FighterClass::default()
        });

        let mut wreck = ShipInstance::new(destroyer, 100, false);
        wreck.alive = false;
        world.fleets.insert(Fleet {
            location: here,
            capital_ships: vec![ShipInstance::new(destroyer, 100, false), wreck],
            fighters: vec![FighterEntry {
                class: tie,
                count: 4,
            }],
            characters: Vec::new(),
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.fleets.insert(Fleet {
            location: here,
            capital_ships: vec![ShipInstance::new(destroyer, 100, true)],
            fighters: Vec::new(),
            characters: Vec::new(),
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        for is_alliance in [false, false, true] {
            world.troops.insert(TroopUnit {
                class_dat_id: regiment,
                is_alliance,
                regiment_strength: 10,
            });
        }
        world.special_forces.insert(SpecialForceUnit {
            class_dat_id: commando,
            is_alliance: false,
            skills: [0; 8],
            on_mission: false,
        });
        world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: battery,
            side: Faction::Empire,
        });
        world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: yard,
                side: Faction::Empire,
                is_shipyard: true,
            });
        world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: yard,
                side: Faction::Neutral,
                is_shipyard: true,
            });

        assert_eq!(
            totals(&world, Faction::Empire).upkeep,
            35 + 4 * 3 + 2 * 2 + 3 + 5 + 13
        );
        assert_eq!(totals(&world, Faction::Alliance).upkeep, 35 + 2);
        assert_eq!(totals(&world, Faction::Neutral).upkeep, 13);
    }

    #[test]
    fn upkeep_counts_the_squadrons_on_a_planet_for_their_class_s_side() {
        // FUN_0052fff0 counts every existing object of the side in families
        // 0x10..0x3f, a squadron on a planet (FUN_005039d0) among them.
        let mut world = GameWorld::default();
        let here = system(&mut world, ControlKind::Controlled(Faction::Alliance));
        let x_wing = world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            maintenance_cost: 3,
            ..FighterClass::default()
        });
        let tie = world.fighter_classes.insert(FighterClass {
            is_empire: true,
            maintenance_cost: 2,
            ..FighterClass::default()
        });
        world.systems[here].fighters = vec![
            FighterEntry {
                class: x_wing,
                count: 2,
            },
            FighterEntry {
                class: tie,
                count: 5,
            },
        ];

        assert_eq!(totals(&world, Faction::Alliance).upkeep, 2 * 3);
        assert_eq!(totals(&world, Faction::Empire).upkeep, 5 * 2);
        assert_eq!(totals(&world, Faction::Neutral).upkeep, 0);
    }

    #[test]
    fn a_queued_build_is_charged_to_the_systems_holder_from_the_order() {
        // FUN_0052a140 creates each product with FUN_004f7d50(.., 1, 0):
        // created and so existing, which is what FUN_0052fff0 counts
        // (manual p. 84).
        let mut world = GameWorld::default();
        let here = system(&mut world, ControlKind::Controlled(Faction::Alliance));
        let there = system(&mut world, ControlKind::Controlled(Faction::Empire));
        let regiment = class(&mut world, 0x1000_0001, 2);
        let mine = class(&mut world, MINE, 7);
        let cruiser = world.capital_ship_classes.insert(CapitalShipClass {
            maintenance_cost: 20,
            ..CapitalShipClass::default()
        });
        let wing = world.fighter_classes.insert(FighterClass {
            maintenance_cost: 4,
            ..FighterClass::default()
        });

        let mut manufacturing = ManufacturingState::new();
        for kind in [
            BuildableKind::CapitalShip(cruiser),
            BuildableKind::Fighter(wing),
            BuildableKind::Troop(regiment),
            BuildableKind::ProductionFacility(FacilityBuild {
                class: mine,
                is_alliance: true,
            }),
        ] {
            manufacturing.enqueue(here, QueueItem::new(kind, 10));
        }
        manufacturing.enqueue(there, QueueItem::new(BuildableKind::Troop(regiment), 10));

        let alliance = maintenance_totals(&world, &manufacturing, Faction::Alliance, |_| false);
        assert_eq!(alliance.upkeep, 20 + 4 + 2 + 7);
        // The mine under construction is not usable yet, so it adds nothing
        // to the pool.
        assert_eq!(alliance.mine_capacity, 0);
        let empire = maintenance_totals(&world, &manufacturing, Faction::Empire, |_| false);
        assert_eq!(empire.upkeep, 2);
    }

    #[test]
    fn the_monitor_value_is_capacity_less_upkeep_and_may_go_negative() {
        // FUN_00422620: side +0x58 less +0x74; FUN_0052f670 flags the side
        // when upkeep exceeds capacity.
        let within = MaintenanceTotals {
            mine_capacity: 300,
            refinery_capacity: 250,
            upkeep: 250,
        };
        assert_eq!(within.available(), 0);
        assert!(!within.is_over_limit());

        let over = MaintenanceTotals {
            upkeep: 251,
            ..within
        };
        assert_eq!(over.available(), -1);
        assert!(over.is_over_limit());
    }
}
