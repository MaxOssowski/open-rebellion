use super::*;
use crate::dat::{ExplorationStatus, Faction};
use crate::ids::{DatId, SectorKey};
use crate::manufacturing::QueueItem;
use crate::world::{CapitalShipClass, ControlKind, FighterClass, ShipInstance, System};

struct Setup {
    world: GameWorld,
    yard: SystemKey,
    there: SystemKey,
    carrack: crate::ids::CapitalShipKey,
    tie: crate::ids::FighterKey,
    manufacturing: ManufacturingState,
}

/// port: a test system at `x`.
fn system(world: &mut GameWorld, x: u16) -> SystemKey {
    world.systems.insert(System {
        last_arrival: None,
        dat_id: DatId::new(0x9000_0001 + u32::from(x)),
        name: "Corellia".into(),
        sector: SectorKey::default(),
        x,
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
        control: ControlKind::Controlled(Faction::Empire),
    })
}

/// port: an Imperial yard, a second system, a ship and a squadron class.
fn setup() -> Setup {
    let mut world = GameWorld::default();
    let yard = system(&mut world, 0);
    let there = system(&mut world, 200);
    let carrack = world.capital_ship_classes.insert(CapitalShipClass {
        is_alliance: false,
        hull: 50,
        ..CapitalShipClass::default()
    });
    let tie = world.fighter_classes.insert(FighterClass {
        is_alliance: false,
        ..FighterClass::default()
    });
    Setup {
        world,
        yard,
        there,
        carrack,
        tie,
        manufacturing: ManufacturingState::new(),
    }
}

impl Setup {
    /// port: a Build Selection order of `count` units, placed at once.
    fn order(&mut self, kind: BuildableKind, count: u32) {
        let items = (0..count)
            .map(|_| QueueItem::new(kind, 10).ordered(&mut self.world))
            .collect();
        self.manufacturing.build_units(self.yard, items);
        reconcile(&mut self.world, &self.manufacturing);
    }

    fn only_fleet(&self) -> FleetKey {
        let fleets: Vec<FleetKey> = self.world.fleets.keys().collect();
        assert_eq!(fleets.len(), 1);
        fleets[0]
    }
}

#[test]
fn an_ordered_ship_waits_unfinished_in_a_new_spare_outside_the_system_list_numbered_next_tick() {
    // FUN_0052a140 creates the product at the order, Existing and not
    // Completed (FUN_004f7d50(.., 1, 0)), in the target's bit-1 spare
    // (FUN_00552300 -> FUN_005097d0). A0 2026-11-15 run "u-2": a fleet
    // icon and "Fleet 4" right after the order. The spare is not Usable
    // (FUN_004fe540), so the system's mode-1 walks skip it.
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let fleet = s.only_fleet();
    assert!(s.world.fleets[fleet].capital_ships.is_empty());
    assert!(!s.world.systems[s.yard].fleets.contains(&fleet));
    assert!(is_staged(&s.world, fleet));
    assert_eq!(staged_fleets_at(&s.world, s.yard), vec![fleet]);
    // Unnumbered until the next tick's 0x300 handler numbers it (A0
    // 2026-11-15 "m-1": "Destination: Fleet" on the order day; "m-2":
    // "Fleet 4" by day 15).
    assert_eq!(s.world.fleet_name(fleet), Some("Fleet"));
    number_new_spares(&mut s.world);
    assert!(s
        .world
        .fleet_name(fleet)
        .is_some_and(|name| name.starts_with("Fleet ")));
    let named = s.world.fleet_name(fleet).map(str::to_owned);
    number_new_spares(&mut s.world);
    assert_eq!(s.world.fleet_name(fleet).map(str::to_owned), named);
    let held: Vec<&Product> = s.world.products.in_fleet(fleet).collect();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].kind, BuildableKind::CapitalShip(s.carrack));
    assert!(!held[0].is_alliance);
}

#[test]
fn reconciling_again_changes_nothing() {
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 2);
    let before = s.world.products.clone();
    let fleets = s.world.fleets.len();
    reconcile(&mut s.world, &s.manufacturing);
    assert_eq!(s.world.products, before);
    assert_eq!(s.world.fleets.len(), fleets);
}

#[test]
fn two_ordered_ships_share_the_spare() {
    // The spare takes every ship of the order (ship-deployment-fleet.md;
    // A0 2026-11-11-icon-menu-topic/66: four Carracks, one Fleet 4).
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 2);
    let fleet = s.only_fleet();
    assert_eq!(s.world.products.in_fleet(fleet).count(), 2);
}

#[test]
fn a_ship_order_joins_a_listed_spare() {
    // A numbered fleet stays its system's spare (FUN_0050bc60), and the
    // next order there joins it (A0 66: "Destination: Fleet 4").
    let mut s = setup();
    let fleet = s.world.insert_fleet(Fleet {
        location: s.yard,
        capital_ships: vec![ShipInstance::new(s.carrack, 50, false)],
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    s.world.systems[s.yard].fleets.push(fleet);
    s.world.set_ship_spare(s.yard, false, fleet);
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    assert_eq!(s.world.fleets.len(), 1);
    assert_eq!(s.world.products.in_fleet(fleet).count(), 1);
    assert!(!is_staged(&s.world, fleet));
}

#[test]
fn a_squadron_product_waits_in_its_target_system() {
    // System_vt78 (FUN_00507750) takes squadrons, regiments and special
    // forces without limit, and the planner scores the target first
    // (FUN_00552300). A0 "u-3": the unfinished TIE on Corellia's Fighter
    // Squadrons page.
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 1);
    assert!(s.world.fleets.is_empty());
    let held: Vec<&Product> = s.world.products.in_system(s.yard).collect();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].container, Container::System(s.yard));
}

#[test]
fn a_new_order_at_the_yard_removes_the_replaced_ship_and_its_emptied_spare() {
    // FUN_00529ab0 -> FUN_0052a2c0 scraps the queued products. A0
    // 2026-11-15 "u-3": the TIE order replaced the Carrack, and the
    // Carrack, Fleet 4 and its icon went.
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let fleet = s.only_fleet();
    s.order(BuildableKind::Fighter(s.tie), 1);
    assert!(!s.world.fleets.contains_key(fleet));
    assert!(s.world.ship_spare(s.yard, false).is_none());
    assert_eq!(s.world.products.all().len(), 1);
    assert_eq!(
        s.world.products.all()[0].kind,
        BuildableKind::Fighter(s.tie)
    );
}

#[test]
fn stopping_the_yard_removes_its_products() {
    // Stop (manual p. 84) drops every unit the area was building.
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 2);
    s.manufacturing.stop(s.yard, ProductionArea::Shipyard);
    reconcile(&mut s.world, &s.manufacturing);
    assert!(s.world.products.all().is_empty());
}

#[test]
fn a_destination_change_moves_the_queued_products() {
    // The Destination order's fire (FUN_0052a380) reparents each queued
    // product to the new target's container (ship-deployment-fleet.md).
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 1);
    s.manufacturing
        .set_destination(s.yard, ProductionArea::Shipyard, s.there);
    reconcile(&mut s.world, &s.manufacturing);
    let product = &s.world.products.all()[0];
    assert_eq!(product.container, Container::System(s.there));
    assert_eq!(product.target, s.there);
}

#[test]
fn a_finished_ship_completes_in_its_fleet_where_it_stands() {
    // FUN_0052bee0 sets Completed and changes no container; the yard and
    // the product stand in one system, so it does not travel.
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let fleet = s.only_fleet();
    let name = s.world.products.all()[0].name.clone();
    let settled = settle(
        &mut s.world,
        vec![CompletionEvent {
            system: s.yard,
            tick: 9,
            kind: BuildableKind::CapitalShip(s.carrack),
            name,
            fleet: None,
        }],
        Vec::new(),
    );
    assert!(settled.departures.is_empty());
    assert_eq!(settled.completions.len(), 1);
    assert_eq!(settled.completions[0].fleet, Some(fleet));
    assert_eq!(settled.completions[0].system, s.yard);
    assert!(s.world.products.all().is_empty());
}

#[test]
fn a_product_moved_away_from_its_yard_travels_there_when_finished() {
    // An unfinished squadron's Move changes its container at once
    // (FUN_00556390: not Deployed, so not mobile); on completion
    // FUN_0052bee0 sets EnrouteActive, as the yard's system differs.
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 1);
    let id = s.world.products.all()[0].id;
    assert!(move_to(&mut s.world, id, Container::System(s.there)));
    let settled = settle(
        &mut s.world,
        vec![CompletionEvent {
            system: s.yard,
            tick: 12,
            kind: BuildableKind::Fighter(s.tie),
            name: None,
            fleet: None,
        }],
        Vec::new(),
    );
    // It stays in its container, Completed and EnrouteActive, until the
    // trip's arrival (FUN_0052bee0 changes no container; +0x44 is the
    // arrival day); no delivery leaves.
    assert!(settled.completions.is_empty());
    assert!(settled.departures.is_empty());
    assert_eq!(
        settled.trips,
        vec![Departure {
            origin: s.yard,
            destination: s.there,
            tick: 12,
            kind: BuildableKind::Fighter(s.tie),
            name: None,
        }]
    );
    let days = u64::from(crate::delivery::transit_days(
        &s.world,
        BuildableKind::Fighter(s.tie),
        s.yard,
        s.there,
    ));
    assert!(days > 0);
    assert_eq!(s.world.products.arrival(id), Some(12 + days));
    assert_eq!(
        s.world.products.all()[0].container,
        Container::System(s.there)
    );
    // The queue let go of it: reconciling keeps it.
    s.manufacturing.stop(s.yard, ProductionArea::Shipyard);
    reconcile(&mut s.world, &s.manufacturing);
    assert!(s.world.products.get(id).is_some());
    // The day before its arrival nothing happens; on the day it completes
    // in its container (event 0x387 -> FUN_004fba10 -> FUN_00556620).
    assert_eq!(arrive(&mut s.world, 11 + days), Arrived::default());
    let arrived = arrive(&mut s.world, 12 + days);
    assert_eq!(
        arrived.completions,
        vec![CompletionEvent {
            system: s.there,
            tick: 12 + days,
            kind: BuildableKind::Fighter(s.tie),
            name: None,
            fleet: None,
        }]
    );
    assert!(s.world.products.all().is_empty());
    assert_eq!(s.world.products.arrival(id), None);
}

#[test]
fn a_ship_finished_away_from_its_yard_arrives_in_its_fleet() {
    let mut s = setup();
    s.manufacturing.enqueue(
        s.yard,
        QueueItem::new(BuildableKind::CapitalShip(s.carrack), 5)
            .ordered(&mut s.world)
            .delivered_to(s.there),
    );
    reconcile(&mut s.world, &s.manufacturing);
    let spare = s.only_fleet();
    let name = s.world.products.all()[0].name.clone();
    let settled = settle(
        &mut s.world,
        Vec::new(),
        vec![Departure {
            origin: s.yard,
            destination: s.there,
            tick: 3,
            kind: BuildableKind::CapitalShip(s.carrack),
            name: name.clone(),
        }],
    );
    assert!(settled.departures.is_empty());
    assert_eq!(settled.trips.len(), 1);
    // A second ship of the same name finishing does not take it.
    assert_eq!(
        settle(
            &mut s.world,
            Vec::new(),
            vec![Departure {
                origin: s.yard,
                destination: s.there,
                tick: 4,
                kind: BuildableKind::CapitalShip(s.carrack),
                name: name.clone(),
            }],
        )
        .departures
        .len(),
        1
    );
    let arrived = arrive(&mut s.world, 1_000);
    assert_eq!(arrived.completions.len(), 1);
    assert_eq!(arrived.completions[0].fleet, Some(spare));
    assert_eq!(arrived.completions[0].system, s.there);
}

#[test]
fn a_product_arriving_at_a_destroyed_system_goes_on_as_a_delivery() {
    // FUN_004f8710: an EnrouteActive child of a destroyed system is not
    // destroyed but set Autorouting (crate::delivery reroutes it).
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 1);
    let id = s.world.products.all()[0].id;
    assert!(move_to(&mut s.world, id, Container::System(s.there)));
    settle(
        &mut s.world,
        vec![CompletionEvent {
            system: s.yard,
            tick: 1,
            kind: BuildableKind::Fighter(s.tie),
            name: None,
            fleet: None,
        }],
        Vec::new(),
    );
    s.world.systems[s.there].is_destroyed = true;
    let arrived = arrive(&mut s.world, 1_000);
    assert!(arrived.completions.is_empty());
    assert_eq!(arrived.departures.len(), 1);
    assert_eq!(arrived.departures[0].origin, s.there);
    assert_eq!(arrived.departures[0].destination, s.there);
    assert!(s.world.products.all().is_empty());
}

#[test]
fn a_moving_fleet_carries_its_unfinished_ships() {
    // The fleet's Move reparents the fleet (FUN_00556390 slot +0xa8);
    // Fleet_vtf4 splits no member off, so its unfinished ships go with it
    // (unfinished-products.md, "A moving fleet carries its unfinished
    // ships").
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let fleet = s.only_fleet();
    s.world.fleets[fleet].location = s.there;
    let product = &s.world.products.all()[0];
    assert_eq!(container_system(&s.world, product.container), Some(s.there));
}

#[test]
fn a_facility_order_places_an_unfinished_facility_in_its_system() {
    // A facility ordered from a construction yard is an object from the
    // start, standing on its destination's facility page, not completed
    // (unfinished-facility.md; A0 pending-facility/11).
    let mut s = setup();
    let build = crate::manufacturing::FacilityBuild {
        class: DatId::new(0x2c00_0001),
        is_alliance: false,
    };
    s.order(BuildableKind::ProductionFacility(build), 1);
    assert!(s.world.fleets.is_empty());
    let held: Vec<&Product> = s.world.products.in_system(s.yard).collect();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].kind, BuildableKind::ProductionFacility(build));
    assert!(is_facility(held[0].kind));
    assert!(!is_facility(BuildableKind::Fighter(s.tie)));
}

#[test]
fn a_facility_finished_away_from_its_yard_travels_inside_its_system_then_completes() {
    // FUN_0052bee0 sets EnrouteActive on a product whose container (the
    // facility's destination system) is not its yard's, then Completed,
    // moving nothing; the arrival (0x387 -> FUN_004fba10 ->
    // FUN_00556620) clears EnrouteActive (unfinished-products.md, "When
    // it becomes finished"; unfinished-facility.md: bit 4, "Enroute").
    let mut s = setup();
    let build = crate::manufacturing::FacilityBuild {
        class: DatId::new(0x2c00_0001),
        is_alliance: false,
    };
    let kind = BuildableKind::ProductionFacility(build);
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(kind, 5).delivered_to(s.there));
    reconcile(&mut s.world, &s.manufacturing);
    let id = s.world.products.all()[0].id;
    let departure = Departure {
        origin: s.yard,
        destination: s.there,
        tick: 1,
        kind,
        name: None,
    };
    let settled = settle(&mut s.world, Vec::new(), vec![departure.clone()]);
    assert!(settled.departures.is_empty());
    assert_eq!(settled.trips, vec![departure]);
    let days = crate::delivery::transit_days(&s.world, kind, s.yard, s.there);
    let arrival = 1 + u64::from(days);
    assert_eq!(s.world.products.arrival(id), Some(arrival));
    assert_eq!(s.world.products.all()[0].container, Container::System(s.there));
    assert_eq!(arrive(&mut s.world, arrival - 1), Arrived::default());
    let arrived = arrive(&mut s.world, arrival);
    assert_eq!(
        arrived.completions,
        vec![CompletionEvent {
            system: s.there,
            tick: arrival,
            kind,
            name: None,
            fleet: None,
        }]
    );
    assert!(s.world.products.all().is_empty());
}

#[test]
fn a_destroyed_unfinished_facility_leaves_its_queue() {
    // FUN_00511930 destroys a facility of the system in mode 3, an
    // unfinished one too; its manager loses the unit.
    let mut s = setup();
    let build = |raw| {
        BuildableKind::ProductionFacility(crate::manufacturing::FacilityBuild {
            class: DatId::new(raw),
            is_alliance: false,
        })
    };
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(build(0x2c00_0001), 5));
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(build(0x2d00_0002), 5));
    reconcile(&mut s.world, &s.manufacturing);
    let ids: Vec<u64> = s.world.products.all().iter().map(|p| p.id).collect();
    assert!(destroy(&mut s.world, &mut s.manufacturing, ids[1]));
    let queue = s
        .manufacturing
        .queue(s.yard, ProductionArea::ConstructionYard)
        .unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.items()[0].kind, build(0x2c00_0001));
    let before = s.world.products.clone();
    reconcile(&mut s.world, &s.manufacturing);
    assert_eq!(s.world.products, before);
    assert_eq!(s.world.products.all().len(), 1);
    assert!(!destroy(&mut s.world, &mut s.manufacturing, ids[1]));
}

#[test]
fn a_products_side_is_its_class_side() {
    // FUN_004f7d50 creates the product with its order's side; the port
    // reads the class's (CAPSHPSD, FIGHTSD, TROOPSD sides).
    let mut s = setup();
    let wing = s.world.fighter_classes.insert(FighterClass {
        is_alliance: true,
        ..FighterClass::default()
    });
    let regiment = DatId::new(0x1000_0001);
    s.world.buildable_classes.insert(
        regiment,
        crate::world::BuildableClass {
            is_alliance: true,
            ..crate::world::BuildableClass::default()
        },
    );
    let stormtroopers = DatId::new(0x1000_0002);
    s.order(BuildableKind::Fighter(wing), 1);
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(BuildableKind::Troop(regiment), 5));
    s.manufacturing.enqueue(
        s.yard,
        QueueItem::new(BuildableKind::Troop(stormtroopers), 5),
    );
    s.manufacturing.enqueue(
        s.yard,
        QueueItem::new(BuildableKind::CapitalShip(s.carrack), 5),
    );
    reconcile(&mut s.world, &s.manufacturing);
    let sides: Vec<(BuildableKind, bool)> = s
        .world
        .products
        .all()
        .iter()
        .map(|p| (p.kind, p.is_alliance))
        .collect();
    assert!(sides.contains(&(BuildableKind::Fighter(wing), true)));
    assert!(sides.contains(&(BuildableKind::Troop(regiment), true)));
    // A class without a record takes its yard's holder (the Empire here).
    assert!(sides.contains(&(BuildableKind::Troop(stormtroopers), false)));
    assert!(sides.contains(&(BuildableKind::CapitalShip(s.carrack), false)));
}

#[test]
fn a_later_order_joins_the_sides_staged_spare_and_the_other_side_gets_its_own() {
    // The bit-1 spare is per side (FUN_005097d0 with the order's side).
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let spare = s.only_fleet();
    let other_yard = system(&mut s.world, 400);
    s.manufacturing.enqueue(
        other_yard,
        QueueItem::new(BuildableKind::CapitalShip(s.carrack), 5).delivered_to(s.yard),
    );
    let rebel = s.world.capital_ship_classes.insert(CapitalShipClass {
        is_alliance: true,
        ..CapitalShipClass::default()
    });
    s.manufacturing.enqueue(
        s.there,
        QueueItem::new(BuildableKind::CapitalShip(rebel), 5).delivered_to(s.yard),
    );
    reconcile(&mut s.world, &s.manufacturing);
    assert_eq!(s.world.products.in_fleet(spare).count(), 2);
    assert_eq!(s.world.fleets.len(), 2);
    let rebels: Vec<FleetKey> = s
        .world
        .fleets
        .iter()
        .filter(|(_, f)| f.is_alliance)
        .map(|(k, _)| k)
        .collect();
    assert_eq!(rebels.len(), 1);
    assert_eq!(s.world.fleets[rebels[0]].location, s.yard);
    assert_eq!(staged_fleets_at(&s.world, s.yard).len(), 2);
    assert!(staged_fleets_at(&s.world, s.there).is_empty());
}

#[test]
fn a_listed_fleet_holding_a_product_is_not_staged() {
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let spare = s.only_fleet();
    s.world.systems[s.yard].fleets.push(spare);
    assert!(!is_staged(&s.world, spare));
    assert!(staged_fleets_at(&s.world, s.yard).is_empty());
}

#[test]
fn a_spare_elsewhere_is_not_reused() {
    // The spare is the target system's own (FUN_005097d0 on the target).
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let spare = s.only_fleet();
    s.world.fleets[spare].location = s.there;
    s.manufacturing.enqueue(
        s.there,
        QueueItem::new(BuildableKind::CapitalShip(s.carrack), 5).delivered_to(s.yard),
    );
    reconcile(&mut s.world, &s.manufacturing);
    assert_eq!(s.world.fleets.len(), 2);
}

#[test]
fn a_staged_spare_holding_anything_else_stays_when_its_products_go() {
    // Only a spare left with nothing goes (A0 2026-11-15 "u-3").
    for strip in 0..6 {
        let mut s = setup();
        s.order(BuildableKind::CapitalShip(s.carrack), 1);
        let spare = s.only_fleet();
        let carrack = s.carrack;
        let tie = s.tie;
        match strip {
            0 => s.world.fleets[spare]
                .capital_ships
                .push(ShipInstance::new(carrack, 50, false)),
            1 => s.world.fleets[spare]
                .fighters
                .push(crate::world::FighterEntry {
                    class: tie,
                    count: 1,
                    carrier: None,
                    damage: None,
                }),
            2 => s.world.fleets[spare]
                .special_forces
                .push(crate::ids::SpecialForceKey::default()),
            3 => s.world.fleets[spare].has_death_star = true,
            4 => s.world.fleets[spare]
                .characters
                .push(crate::ids::CharacterKey::default()),
            _ => s.world.systems[s.yard].fleets.push(spare),
        }
        s.manufacturing.stop(s.yard, ProductionArea::Shipyard);
        reconcile(&mut s.world, &s.manufacturing);
        assert!(s.world.fleets.contains_key(spare), "case {strip}");
    }
}

#[test]
fn a_ship_order_leaves_the_yards_regiment_products_alone() {
    // Each area's manager scraps only its own products (FUN_0052a2c0).
    let mut s = setup();
    s.manufacturing.enqueue(
        s.yard,
        QueueItem::new(BuildableKind::Troop(DatId::new(0x1000_0001)), 5),
    );
    reconcile(&mut s.world, &s.manufacturing);
    let regiment = s.world.products.all()[0].id;
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    s.order(BuildableKind::Fighter(s.tie), 1);
    assert!(s.world.products.get(regiment).is_some());
}

#[test]
fn reordering_the_same_class_replaces_the_named_product() {
    // FUN_00529ab0 scraps the queued products and FUN_0052a140 creates new
    // ones, named with the next numbers (QueueItem::ordered).
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let first = s.world.products.all()[0].id;
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    assert_eq!(s.world.products.all().len(), 1);
    assert_ne!(s.world.products.all()[0].id, first);
}

#[test]
fn a_cancelled_last_unit_drops_only_its_product() {
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 2);
    let first = s.world.products.all()[0].id;
    s.manufacturing
        .queue_mut(s.yard, ProductionArea::Shipyard)
        .cancel(1);
    reconcile(&mut s.world, &s.manufacturing);
    let ids: Vec<u64> = s.world.products.all().iter().map(|p| p.id).collect();
    assert_eq!(ids, vec![first]);
}

#[test]
fn a_destination_change_moves_ships_to_the_new_spare_in_id_order() {
    // FUN_0052a380 reparents every queued product; each ship goes to the
    // new target's spare, and the emptied one goes.
    let mut s = setup();
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(BuildableKind::Fighter(s.tie), 5));
    s.manufacturing.enqueue(
        s.yard,
        QueueItem::new(BuildableKind::CapitalShip(s.carrack), 5),
    );
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(BuildableKind::Fighter(s.tie), 5));
    reconcile(&mut s.world, &s.manufacturing);
    let old = s.only_fleet();
    s.manufacturing
        .set_destination(s.yard, ProductionArea::Shipyard, s.there);
    reconcile(&mut s.world, &s.manufacturing);
    assert!(!s.world.fleets.contains_key(old));
    let new = s.only_fleet();
    assert_eq!(s.world.fleets[new].location, s.there);
    let all: Vec<u64> = s.world.products.all().iter().map(|p| p.id).collect();
    assert_eq!(all, vec![1, 2, 3]);
}

#[test]
fn a_product_whose_fleet_is_gone_is_placed_again() {
    // port: a product left with no container goes back to its target's
    // spare; one whose fleet stands is left alone.
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let gone = s.only_fleet();
    s.world.fleets.remove(gone);
    reconcile(&mut s.world, &s.manufacturing);
    let again = s.only_fleet();
    assert_ne!(again, gone);
    assert_eq!(s.world.products.in_fleet(again).count(), 1);
    let before = s.world.products.clone();
    reconcile(&mut s.world, &s.manufacturing);
    assert_eq!(s.world.products, before);
}

#[test]
fn a_finished_unit_settles_its_own_product() {
    // FUN_0052b960 completes the manager's current product; the port finds
    // it by yard, kind and name.
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 1);
    let other_yard = system(&mut s.world, 400);
    s.manufacturing
        .enqueue(other_yard, QueueItem::new(BuildableKind::Fighter(s.tie), 5));
    s.manufacturing.enqueue(
        s.yard,
        QueueItem::new(BuildableKind::Troop(DatId::new(0x1000_0001)), 5),
    );
    reconcile(&mut s.world, &s.manufacturing);
    let before: Vec<(u64, SystemKey)> = s
        .world
        .products
        .all()
        .iter()
        .map(|p| (p.id, p.yard))
        .collect();
    let mut advance = crate::manufacturing::ManufacturingAdvance {
        completions: vec![CompletionEvent {
            system: other_yard,
            tick: 3,
            kind: BuildableKind::Fighter(s.tie),
            name: None,
            fleet: None,
        }],
        newly_idle: Vec::new(),
        departures: Vec::new(),
        trips: Vec::new(),
    };
    settle_advance(&mut s.world, &mut advance);
    let left: Vec<(u64, SystemKey)> = s
        .world
        .products
        .all()
        .iter()
        .map(|p| (p.id, p.yard))
        .collect();
    let gone: Vec<_> = before.iter().filter(|p| !left.contains(p)).collect();
    assert_eq!(gone.len(), 1);
    assert_eq!(gone[0].1, other_yard);
    assert_eq!(advance.completions.len(), 1);
    assert_eq!(advance.completions[0].system, other_yard);
}

#[test]
fn a_departure_settles_where_its_product_stands() {
    // A product moved to its yard's own system finishes there; one left at
    // its target travels there (FUN_0052bee0).
    let mut s = setup();
    s.manufacturing.enqueue(
        s.yard,
        QueueItem::new(BuildableKind::Fighter(s.tie), 5).delivered_to(s.there),
    );
    reconcile(&mut s.world, &s.manufacturing);
    let id = s.world.products.all()[0].id;
    assert!(move_to(&mut s.world, id, Container::System(s.yard)));
    let departure = Departure {
        origin: s.yard,
        destination: s.there,
        tick: 4,
        kind: BuildableKind::Fighter(s.tie),
        name: None,
    };
    let mut advance = crate::manufacturing::ManufacturingAdvance {
        completions: Vec::new(),
        newly_idle: Vec::new(),
        departures: vec![departure.clone()],
        trips: Vec::new(),
    };
    settle_advance(&mut s.world, &mut advance);
    assert!(advance.departures.is_empty());
    assert_eq!(advance.completions[0].system, s.yard);
    // With no product, the departure passes through.
    let settled = settle(&mut s.world, Vec::new(), vec![departure.clone()]);
    assert_eq!(settled.departures, vec![departure]);
    assert!(!move_to(&mut s.world, 99, Container::System(s.yard)));
}

#[test]
fn a_moved_unfinished_ship_goes_to_the_targets_spare_or_fleet() {
    // hyp: FUN_00556390 reparents a ship that is not Deployed at once, into
    // the container the planner gives the target (FUN_00552300).
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let old = s.only_fleet();
    let id = s.world.products.all()[0].id;
    assert!(move_to_target(&mut s.world, id, s.there, None, 0));
    assert!(!s.world.fleets.contains_key(old));
    let new = s.only_fleet();
    assert_eq!(s.world.fleets[new].location, s.there);
    assert!(is_staged(&s.world, new));

    let listed = s.world.insert_fleet(Fleet {
        location: s.yard,
        capital_ships: vec![ShipInstance::new(s.carrack, 50, false)],
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    s.world.systems[s.yard].fleets.push(listed);
    assert!(move_to_target(&mut s.world, id, s.yard, Some(listed), 0));
    assert_eq!(s.world.products.in_fleet(listed).count(), 1);
    assert!(!s.world.fleets.contains_key(new));
    // To the system with a spare there, it joins the spare.
    s.world.set_ship_spare(s.there, false, listed);
    s.world.fleets[listed].location = s.there;
    assert!(move_to_target(&mut s.world, id, s.there, None, 0));
    assert_eq!(s.world.products.in_fleet(listed).count(), 1);
}

#[test]
fn a_moved_unfinished_squadron_lands_on_the_system_and_refuses_a_fleet() {
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 1);
    let id = s.world.products.all()[0].id;
    let fleet = s.world.insert_fleet(Fleet {
        location: s.there,
        capital_ships: Vec::new(),
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    assert!(!move_to_target(&mut s.world, id, s.there, Some(fleet), 0));
    assert!(move_to_target(&mut s.world, id, s.there, None, 0));
    assert_eq!(
        s.world.products.all()[0].container,
        Container::System(s.there)
    );
    assert!(!move_to_target(&mut s.world, 99, s.there, None, 0));
}

#[test]
fn a_moved_unfinished_ship_skips_a_fleet_of_the_other_side_and_keeps_its_place_in_order() {
    // hyp: a released fleet takes the ship only when it is of its side;
    // otherwise the ship goes to the system's spare. The products stay in
    // order of creation.
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 3);
    let ids: Vec<u64> = s.world.products.all().iter().map(|p| p.id).collect();
    let rebels = s.world.insert_fleet(Fleet {
        location: s.there,
        capital_ships: Vec::new(),
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    s.world.systems[s.there].fleets.push(rebels);
    assert!(move_to_target(
        &mut s.world,
        ids[1],
        s.there,
        Some(rebels),
        0
    ));
    assert_eq!(s.world.products.in_fleet(rebels).count(), 0);
    let moved = s.world.products.get(ids[1]).unwrap().container;
    let Container::Fleet(spare) = moved else {
        panic!("{moved:?}");
    };
    assert_eq!(s.world.fleets[spare].location, s.there);
    assert!(!s.world.fleets[spare].is_alliance);
    let after: Vec<u64> = s.world.products.all().iter().map(|p| p.id).collect();
    assert_eq!(after, ids);
}

#[test]
fn a_product_on_its_trip_takes_no_move() {
    // FUN_004f9860 refuses an object en route.
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 1);
    let id = s.world.products.all()[0].id;
    assert!(move_to(&mut s.world, id, Container::System(s.there)));
    settle(
        &mut s.world,
        vec![CompletionEvent {
            system: s.yard,
            tick: 1,
            kind: BuildableKind::Fighter(s.tie),
            name: None,
            fleet: None,
        }],
        Vec::new(),
    );
    assert!(s.world.products.arrival(id).is_some());
    assert!(!move_to(&mut s.world, id, Container::System(s.yard)));
    assert!(!move_to_target(&mut s.world, id, s.yard, None, 0));
    assert_eq!(s.world.products.travelling().count(), 1);
}

#[test]
fn a_ship_on_its_trip_takes_no_move_to_a_system_without_a_spare() {
    // FUN_004f9860 refuses an object en route, whatever the target.
    let mut s = setup();
    s.manufacturing.enqueue(
        s.yard,
        QueueItem::new(BuildableKind::CapitalShip(s.carrack), 5)
            .ordered(&mut s.world)
            .delivered_to(s.there),
    );
    reconcile(&mut s.world, &s.manufacturing);
    let id = s.world.products.all()[0].id;
    let name = s.world.products.all()[0].name.clone();
    settle(
        &mut s.world,
        Vec::new(),
        vec![Departure {
            origin: s.yard,
            destination: s.there,
            tick: 2,
            kind: BuildableKind::CapitalShip(s.carrack),
            name,
        }],
    );
    let before = s.world.products.clone();
    assert!(!move_to_target(&mut s.world, id, s.yard, None, 0));
    assert_eq!(s.world.products, before);
}

#[test]
fn renaming_an_unfinished_ship_renames_its_queued_unit_too() {
    // 0x203 on the unfinished object (A0 2026-11-15 "u-2": Rename white);
    // the queued unit's name is the finished ship's (QueueItem::name), so
    // the next reconcile keeps the product.
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 2);
    let second = s.world.products.all()[1].id;
    assert!(rename(
        &mut s.world,
        &mut s.manufacturing,
        second,
        "Liberty".into()
    ));
    let items = s
        .manufacturing
        .queue(s.yard, ProductionArea::Shipyard)
        .unwrap()
        .items();
    assert_eq!(items[1].name.as_deref(), Some("Liberty"));
    assert_ne!(items[0].name.as_deref(), Some("Liberty"));
    let before = s.world.products.clone();
    reconcile(&mut s.world, &s.manufacturing);
    assert_eq!(s.world.products, before);
    assert_eq!(
        s.world.products.get(second).unwrap().name.as_deref(),
        Some("Liberty")
    );
    assert!(!rename(&mut s.world, &mut s.manufacturing, 99, "X".into()));
}

#[test]
fn a_rename_finds_its_unit_in_its_own_yards_queue() {
    // The product's place counts only its own manager's units.
    let mut s = setup();
    let other_yard = system(&mut s.world, 400);
    let other = QueueItem::new(BuildableKind::CapitalShip(s.carrack), 5).ordered(&mut s.world);
    s.manufacturing.enqueue(other_yard, other);
    reconcile(&mut s.world, &s.manufacturing);
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let mine = s
        .world
        .products
        .all()
        .iter()
        .find(|p| p.yard == s.yard)
        .unwrap()
        .id;
    assert!(rename(
        &mut s.world,
        &mut s.manufacturing,
        mine,
        "Liberty".into()
    ));
    let names = |yard| {
        s.manufacturing
            .queue(yard, ProductionArea::Shipyard)
            .unwrap()
            .items()[0]
            .name
            .clone()
    };
    assert_eq!(names(s.yard).as_deref(), Some("Liberty"));
    assert_ne!(names(other_yard).as_deref(), Some("Liberty"));
}

#[test]
fn an_unfinished_units_move_asks_only_out_of_its_sides_blockaded_system() {
    // FUN_00487cc0, case 0x201: the first member's container is a system
    // with +0x88 bit 0x20 (blockaded) and the member's side.
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 1);
    let squadron = s.world.products.all()[0].id;
    assert!(!move_asks(&s.world, &[squadron]));
    let enemy = s.world.insert_fleet(Fleet {
        location: s.yard,
        capital_ships: vec![ShipInstance::new(s.carrack, 50, true)],
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    s.world.systems[s.yard].fleets.push(enemy);
    assert!(move_asks(&s.world, &[squadron]));
    // An Alliance unit at the Empire's blockaded system does not ask.
    let alliance_wing = s.world.fighter_classes.insert(FighterClass {
        is_alliance: true,
        ..FighterClass::default()
    });
    s.manufacturing.enqueue(
        s.yard,
        QueueItem::new(BuildableKind::Fighter(alliance_wing), 5),
    );
    reconcile(&mut s.world, &s.manufacturing);
    let other = s.world.products.all()[1].id;
    assert!(!move_asks(&s.world, &[other]));
    // A ship stands in a fleet.
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let ship = s
        .world
        .products
        .all()
        .iter()
        .find(|p| matches!(p.kind, BuildableKind::CapitalShip(_)))
        .unwrap()
        .id;
    assert!(!move_asks(&s.world, &[ship]));
    assert!(!move_asks(&s.world, &[]));
}

/// port: an Imperial fleet of one ship of `fighters` and `troops` capacity
/// at the yard.
fn carrier(s: &mut Setup, fighters: u32, troops: u32) -> FleetKey {
    let class = s.world.capital_ship_classes.insert(CapitalShipClass {
        is_alliance: false,
        hull: 50,
        fighter_capacity: fighters,
        troop_capacity: troops,
        ..CapitalShipClass::default()
    });
    let fleet = s.world.insert_fleet(Fleet {
        location: s.yard,
        capital_ships: vec![ShipInstance::new(class, 50, false)],
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    s.world.systems[s.yard].fleets.push(fleet);
    fleet
}

#[test]
fn an_unfinished_squadron_boards_a_fleet_with_room_and_not_one_without() {
    // FUN_00552300 weighs a fleet target's usable ships; each ship's room
    // (vt78, fighter capacity +0x26c less those aboard in mode 3) decides,
    // and with none the leg refuses (0x27).
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 2);
    let ids: Vec<u64> = s.world.products.all().iter().map(|p| p.id).collect();
    let small = carrier(&mut s, 1, 0);
    assert!(room_aboard(
        &s.world,
        small,
        BuildableKind::Fighter(s.tie),
        false,
        0
    ));
    assert!(move_to_target(&mut s.world, ids[0], s.yard, Some(small), 0));
    assert_eq!(
        s.world.products.get(ids[0]).unwrap().container,
        Container::Fleet(small)
    );
    // The one being built aboard takes the room.
    assert!(!move_to_target(
        &mut s.world,
        ids[1],
        s.yard,
        Some(small),
        0
    ));
    // The other side's fleet, or one with no usable ship, takes none.
    assert!(!room_aboard(
        &s.world,
        small,
        BuildableKind::Fighter(s.tie),
        true,
        0
    ));
    s.world.fleets[small].capital_ships[0].alive = false;
    assert!(!room_aboard(
        &s.world,
        small,
        BuildableKind::Fighter(s.tie),
        false,
        0
    ));
}

#[test]
fn an_unfinished_regiment_boards_a_fleet_with_troop_room() {
    // As a squadron, against the troop capacity less the regiments carried
    // and those being built aboard; a special force needs no room.
    let mut s = setup();
    let regiment = BuildableKind::Troop(DatId::new(0x1000_0001));
    let transport = carrier(&mut s, 0, 2);
    // Another side's fleet takes none, room or not.
    assert!(!room_aboard(&s.world, transport, regiment, true, 0));
    assert!(room_aboard(&s.world, transport, regiment, false, 1));
    assert!(!room_aboard(&s.world, transport, regiment, false, 2));
    s.manufacturing.enqueue(s.yard, QueueItem::new(regiment, 5));
    reconcile(&mut s.world, &s.manufacturing);
    let id = s.world.products.all()[0].id;
    assert!(move_to_target(&mut s.world, id, s.yard, Some(transport), 0));
    assert!(!room_aboard(&s.world, transport, regiment, false, 1));
    assert!(room_aboard(
        &s.world,
        transport,
        BuildableKind::SpecialForce(DatId::new(0x3c00_0001)),
        false,
        9
    ));
    assert!(!room_aboard(
        &s.world,
        transport,
        BuildableKind::CapitalShip(s.carrack),
        false,
        0
    ));
}

#[test]
fn a_regiment_or_special_force_finished_aboard_boards_its_fleet() {
    // Completion puts the unit on its fleet's planet; it goes aboard as its
    // product stood (TroopTransportState::embark; Fleet::special_forces).
    let mut s = setup();
    let fleet = carrier(&mut s, 0, 2);
    let class = DatId::new(0x1000_0001);
    let troop = s.world.troops.insert(crate::world::TroopUnit {
        class_dat_id: class,
        is_alliance: false,
        regiment_strength: 100,
    });
    s.world.systems[s.yard].ground_units.push(troop);
    let force_class = DatId::new(0x3c00_0001);
    let unit = s
        .world
        .special_forces
        .insert(crate::world::SpecialForceUnit {
            class_dat_id: force_class,
            is_alliance: false,
            skills: [0; 8],
            on_mission: false,
        });
    s.world.systems[s.yard].special_forces.push(unit);
    let mut transport = crate::troop_transport::TroopTransportState::default();
    let done = |kind| CompletionEvent {
        system: s.yard,
        tick: 1,
        kind,
        name: None,
        fleet: Some(fleet),
    };
    board(
        &mut s.world,
        &mut transport,
        &[
            done(BuildableKind::Troop(class)),
            done(BuildableKind::SpecialForce(force_class)),
        ],
    );
    assert_eq!(transport.carried_count(fleet), 1);
    assert!(!s.world.systems[s.yard].ground_units.contains(&troop));
    assert_eq!(s.world.fleets[fleet].special_forces, vec![unit]);
    assert!(!s.world.systems[s.yard].special_forces.contains(&unit));
    // A completion with no fleet, or one standing elsewhere, boards nothing.
    let mut elsewhere = done(BuildableKind::Troop(class));
    elsewhere.system = s.there;
    board(&mut s.world, &mut transport, &[elsewhere]);
    assert_eq!(transport.carried_count(fleet), 1);
}

#[test]
fn a_regiment_or_squadron_on_its_trip_into_a_system_is_pending_there() {
    // CMD-03-C018: the defenses icon's Move greys while a regiment or
    // squadron of the side is on its way in (FUN_004f9860 code 4); a queued
    // one does not (FUN_0053c100).
    let mut s = setup();
    s.order(BuildableKind::Fighter(s.tie), 1);
    let id = s.world.products.all()[0].id;
    assert!(move_to(&mut s.world, id, Container::System(s.there)));
    assert!(!surface_arrival_pending(&s.world, s.there, false));
    settle(
        &mut s.world,
        vec![CompletionEvent {
            system: s.yard,
            tick: 1,
            kind: BuildableKind::Fighter(s.tie),
            name: None,
            fleet: None,
        }],
        Vec::new(),
    );
    assert!(surface_arrival_pending(&s.world, s.there, false));
    assert!(!surface_arrival_pending(&s.world, s.there, true));
    assert!(!surface_arrival_pending(&s.world, s.yard, false));
}

#[test]
fn the_current_ship_product_is_the_yards_oldest_still_being_built() {
    // FUN_00509670(system, 0): the system's ship manager; its ProductKey
    // (+0x74) is the first queued unit not yet completed.
    let mut s = setup();
    let regiment = BuildableKind::Troop(DatId::new(0x1000_0001));
    s.manufacturing.enqueue(s.yard, QueueItem::new(regiment, 5));
    let other_yard = system(&mut s.world, 400);
    s.manufacturing
        .enqueue(other_yard, QueueItem::new(BuildableKind::Fighter(s.tie), 5));
    reconcile(&mut s.world, &s.manufacturing);
    assert!(current_ship_product(&s.world, s.yard).is_none());
    s.manufacturing.enqueue(
        s.yard,
        QueueItem::new(BuildableKind::Fighter(s.tie), 5).delivered_to(s.there),
    );
    s.manufacturing.enqueue(
        s.yard,
        QueueItem::new(BuildableKind::CapitalShip(s.carrack), 5),
    );
    reconcile(&mut s.world, &s.manufacturing);
    let first = current_ship_product(&s.world, s.yard).unwrap();
    assert_eq!(first.kind, BuildableKind::Fighter(s.tie));
    assert_eq!(first.yard, s.yard);
    // Finished and on its trip, it is no longer the current product.
    settle(
        &mut s.world,
        Vec::new(),
        vec![Departure {
            origin: s.yard,
            destination: s.there,
            tick: 1,
            kind: BuildableKind::Fighter(s.tie),
            name: None,
        }],
    );
    assert_eq!(
        current_ship_product(&s.world, s.yard).unwrap().kind,
        BuildableKind::CapitalShip(s.carrack)
    );
}

#[test]
fn a_staged_spare_moves_at_once_carrying_its_unfinished_ship() {
    // A0 2026-11-15 "m-2": the spare holding only the unfinished Carrack
    // took a Move to Talus; the ship stayed under construction and the
    // build stayed at Corfai, naming it. Not mobile (FUN_004fda10),
    // FUN_00556390 moves it at once; leaving, it is no spare
    // (FUN_004ff570).
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let spare = s.only_fleet();
    let mut movement = crate::movement::MovementState::new();
    assert!(crate::movement::fleet_move_enabled(
        &movement, &s.world, spare, false
    ));
    let departure = crate::movement::begin_faction_fleet_transit(
        &mut movement,
        &mut s.world,
        spare,
        s.there,
        false,
    )
    .expect("the move is accepted");
    assert_eq!(departure.transit_ticks, 0);
    assert!(!movement.is_in_transit(spare));
    assert_eq!(s.world.fleets[spare].location, s.there);
    assert!(is_staged(&s.world, spare));
    assert_eq!(staged_fleets_at(&s.world, s.there), vec![spare]);
    assert!(staged_fleets_at(&s.world, s.yard).is_empty());
    assert_eq!(s.world.ship_spare(s.yard, false), None);
    assert_eq!(spare_at(&s.world, s.yard, false), None);
    let product = &s.world.products.all()[0];
    assert_eq!(container_system(&s.world, product.container), Some(s.there));
    assert_eq!(deployment_fleet(&s.world, s.yard), Some(spare));
    // The queue keeps it there.
    let before = s.world.products.clone();
    reconcile(&mut s.world, &s.manufacturing);
    assert_eq!(s.world.products, before);
    // A fleet with ships still needs its hyperdrive; a listed one is no
    // staged spare.
    assert!(relocate_staged(&mut s.world, spare, s.yard));
    assert!(relocate_staged(&mut s.world, spare, s.there));
    s.world.systems[s.there].fleets.push(spare);
    assert!(!relocate_staged(&mut s.world, spare, s.yard));
}

#[test]
fn the_band_names_the_container_its_current_product_was_dropped_in() {
    // A0 2026-11-15 "m-3": the Carrack dropped on Fleet 3 left Fleet 4,
    // which went, and the band read "Destination: Fleet 3" (the
    // DeploymentKey is the product's container, FUN_0052ac40).
    let mut s = setup();
    s.order(BuildableKind::CapitalShip(s.carrack), 1);
    let spare = s.only_fleet();
    let id = s.world.products.all()[0].id;
    assert_eq!(deployment_fleet(&s.world, s.yard), Some(spare));
    let other = s.world.insert_fleet(Fleet {
        location: s.yard,
        capital_ships: vec![ShipInstance::new(s.carrack, 50, false)],
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    s.world.systems[s.yard].fleets.push(other);
    assert!(move_to_target(&mut s.world, id, s.yard, Some(other), 0));
    assert!(!s.world.fleets.contains_key(spare));
    assert_eq!(deployment_fleet(&s.world, s.yard), Some(other));
    s.order(BuildableKind::Fighter(s.tie), 1);
    assert_eq!(deployment_fleet(&s.world, s.yard), None);
}

/// port: an Alliance construction yard (MANFACSD family `0x2a`) built at
/// the yard for `there`, finished and on its way, both systems Alliance.
fn alliance_yard_en_route(s: &mut Setup) -> (u64, BuildableKind, u64) {
    for key in [s.yard, s.there] {
        s.world.systems[key].control = ControlKind::Controlled(Faction::Alliance);
    }
    let kind = BuildableKind::ManufacturingFacility(crate::manufacturing::FacilityBuild {
        class: DatId::new(0x2a00_0001),
        is_alliance: true,
    });
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(kind, 5).delivered_to(s.there));
    reconcile(&mut s.world, &s.manufacturing);
    let id = s.world.products.all()[0].id;
    settle(
        &mut s.world,
        Vec::new(),
        vec![Departure {
            origin: s.yard,
            destination: s.there,
            tick: 1,
            kind,
            name: None,
        }],
    );
    let arrival = s.world.products.arrival(id).expect("travelling");
    // Finished, it has left its queue (FUN_0052bee0).
    s.manufacturing
        .queue_mut(s.yard, ProductionArea::of(kind))
        .cancel(0);
    (id, kind, arrival)
}

#[test]
fn a_facility_en_route_to_a_system_lost_with_no_room_elsewhere_is_removed_without_a_message() {
    // FUN_004f8680: a facility Completed and EnrouteActive is not handed
    // over but set Autorouting; the 0x307 handler (FUN_00515480) finds no
    // system of its side with room (FUN_005545e0; System_vt74
    // FUN_005073d0 refuses other sides for a facility, System_vt78
    // FUN_00507750 has no energy left) and destroys it with reason 2
    // (slot +0xac). FUN_004fc080 raises 0x303 only for a destination-change
    // count (+0x42) not 0, so a first trip files no Facility Lost.
    // A0 .artifacts/a0-vm/2026-11-17-edited-saves-6 c-01..c-07: the yard
    // built at Talus for Corsin went when Corsin turned Imperial, with no
    // Facility Lost.
    let mut s = setup();
    let (id, _, _) = alliance_yard_en_route(&mut s);
    s.world.systems[s.yard].total_energy = 0;
    s.world.systems[s.there].control = ControlKind::Controlled(Faction::Empire);
    let routed = autoroute_facilities(&mut s.world, s.there, Faction::Empire);
    assert_eq!(routed.removed, vec![id]);
    assert!(routed.moved.is_empty());
    assert!(s.world.products.all().is_empty());
    assert_eq!(arrive(&mut s.world, u64::MAX), Arrived::default());
}

#[test]
fn a_rerouted_facility_left_with_no_place_by_a_second_side_change_goes_without_a_message() {
    // The first loss moves the yard while it is en route (FUN_00515480 ->
    // FUN_00556390 -> slot +0xa8, FUN_004f6fd0 -> slot +0xf4,
    // GameObj_vtf4 FUN_004faf50 raises the destination-change count +0x42
    // to 1). The second finds no place and destroys it with reason 2
    // (slot +0xac). Yet no Facility Lost: the 0x303 UI record (class
    // 0x00659778) has `ret 4` at slot +0x1c, so FUN_00497690 is never fed.
    // A0
    // .artifacts/a0-vm/2026-11-18-edited-saves-8 f-05 (5959bf3fa11e, the
    // yard rerouted to Talus), f-06 (Talus lost on day 47), f-07..f-09
    // (423a00097b63, 897748b1bf99: no Facility Lost to day 94), and the
    // day-94 save FAC2D94 (7398721add80) holds no yard of either side at
    // Talus.
    let mut s = setup();
    let (id, _, _) = alliance_yard_en_route(&mut s);
    s.world.systems[s.there].control = ControlKind::Controlled(Faction::Empire);
    let first = autoroute_facilities(&mut s.world, s.there, Faction::Empire);
    assert_eq!(first.moved, vec![(id, s.yard)]);
    assert!(s.world.products.arrival(id).is_some());
    s.world.systems[s.yard].control = ControlKind::Controlled(Faction::Empire);
    let second = autoroute_facilities(&mut s.world, s.yard, Faction::Empire);
    assert_eq!(
        second,
        Autorouted {
            removed: vec![id],
            ..Autorouted::default()
        }
    );
    assert!(s.world.products.all().is_empty());
    assert!(destroy_lost(&mut s.world, &mut s.manufacturing).is_empty());
    assert_eq!(arrive(&mut s.world, u64::MAX), Arrived::default());
}

#[test]
fn a_facility_en_route_to_a_system_lost_goes_on_to_the_nearest_system_of_its_side_with_room() {
    // FUN_00515480 moves it (FUN_00556390) to the search's place: the
    // nearest unblockaded system of its side with room (FUN_00554e80). Its
    // pending arrival still fires on the old day; FUN_00556620 then starts
    // the leg on (port: the old day plus that leg).
    let mut s = setup();
    let (id, kind, arrival) = alliance_yard_en_route(&mut s);
    // A nearer Alliance system with no energy left is passed over.
    let full = system(&mut s.world, 150);
    s.world.systems[full].control = ControlKind::Controlled(Faction::Alliance);
    s.world.systems[full].total_energy = 0;
    s.world.systems[s.there].control = ControlKind::Controlled(Faction::Empire);
    s.world.hand_over_facilities(s.there);
    let product = s.world.products.get(id).expect("kept");
    assert_eq!(product.container, Container::System(s.yard));
    let leg = crate::delivery::transit_days(&s.world, kind, s.there, s.yard);
    assert!(leg > 0);
    assert_eq!(s.world.products.arrival(id), Some(arrival + u64::from(leg)));
}

#[test]
fn an_unfinished_facility_in_a_system_lost_moves_at_once_to_a_system_of_its_side() {
    // FUN_004f8680 sets Autorouting on a facility not Completed; not
    // Deployed (slot +0x38), it changes container at once (FUN_00556390),
    // and its completion starts the trip from its yard.
    let mut s = setup();
    for key in [s.yard, s.there] {
        s.world.systems[key].control = ControlKind::Controlled(Faction::Alliance);
    }
    let kind = BuildableKind::ManufacturingFacility(crate::manufacturing::FacilityBuild {
        class: DatId::new(0x2a00_0001),
        is_alliance: true,
    });
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(kind, 5).delivered_to(s.there));
    reconcile(&mut s.world, &s.manufacturing);
    let id = s.world.products.all()[0].id;
    s.world.systems[s.there].control = ControlKind::Controlled(Faction::Empire);
    let routed = autoroute_facilities(&mut s.world, s.there, Faction::Empire);
    assert_eq!(routed.moved, vec![(id, s.yard)]);
    assert_eq!(s.world.products.arrival(id), None);
    let container = |s: &Setup| s.world.products.all()[0].container;
    assert_eq!(container(&s), Container::System(s.yard));
    // Its queue keeps it: the next reconcile leaves the product there.
    reconcile(&mut s.world, &s.manufacturing);
    assert_eq!(container(&s), Container::System(s.yard));
}

#[test]
fn an_unfinished_facility_with_no_place_is_destroyed_with_its_queued_unit() {
    // FUN_004f8680 sets Autorouting on a facility not Completed; the 0x307
    // handler (FUN_00515480) finds no system of its side with room
    // (FUN_005545e0) and destroys it with reason 2 (slot +0xac,
    // FUN_004f8660). Not EnrouteActive, it raises no 0x303 (FUN_004fc080);
    // its manager no longer takes it (FUN_00528890 wants +0x50 & 0x1c clear).
    let mut s = setup();
    for key in [s.yard, s.there] {
        s.world.systems[key].control = ControlKind::Controlled(Faction::Alliance);
    }
    s.world.systems[s.yard].total_energy = 0;
    let kind = BuildableKind::ManufacturingFacility(crate::manufacturing::FacilityBuild {
        class: DatId::new(0x2a00_0001),
        is_alliance: true,
    });
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(kind, 5).delivered_to(s.there));
    // A second unit, bound for the yard, which stays Alliance.
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(kind, 5).delivered_to(s.yard));
    reconcile(&mut s.world, &s.manufacturing);
    let ids: Vec<u64> = s.world.products.all().iter().map(|p| p.id).collect();
    assert_eq!(ids.len(), 2);
    s.world.systems[s.there].control = ControlKind::Controlled(Faction::Empire);
    s.world.hand_over_facilities(s.there);
    assert_eq!(
        destroy_lost(&mut s.world, &mut s.manufacturing),
        vec![ids[0]]
    );
    let area = ProductionArea::of(kind);
    let queue = s.manufacturing.queue(s.yard, area).expect("queue");
    assert_eq!(queue.items().len(), 1);
    assert_eq!(queue.items()[0].destination, Some(s.yard));
    // The next reconcile makes no new product for it.
    reconcile(&mut s.world, &s.manufacturing);
    let left: Vec<u64> = s.world.products.all().iter().map(|p| p.id).collect();
    assert_eq!(left, vec![ids[1]]);
    assert!(destroy_lost(&mut s.world, &mut s.manufacturing).is_empty());
}

#[test]
fn an_autoroute_with_no_place_reports_an_unfinished_facility_as_destroyed() {
    let mut s = setup();
    for key in [s.yard, s.there] {
        s.world.systems[key].control = ControlKind::Controlled(Faction::Alliance);
    }
    s.world.systems[s.yard].total_energy = 0;
    let kind = BuildableKind::ProductionFacility(crate::manufacturing::FacilityBuild {
        class: DatId::new(0x2c00_0001),
        is_alliance: true,
    });
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(kind, 5).delivered_to(s.there));
    reconcile(&mut s.world, &s.manufacturing);
    let id = s.world.products.all()[0].id;
    s.world.systems[s.there].control = ControlKind::Controlled(Faction::Empire);
    let routed = autoroute_facilities(&mut s.world, s.there, Faction::Empire);
    assert_eq!(
        routed,
        Autorouted {
            destroyed: vec![id],
            ..Autorouted::default()
        }
    );
    // A second autoroute before the queues are settled lists it once.
    autoroute_facilities(&mut s.world, s.there, Faction::Empire);
    assert_eq!(destroy_lost(&mut s.world, &mut s.manufacturing), vec![id]);
    assert!(s.world.products.all().is_empty());
}

#[test]
fn a_mine_needs_a_raw_material_slot_where_it_autoroutes() {
    // System_vt78 (FUN_00507750): a mine (0x2c) also takes raw materials
    // (+0x64) less the mines there (FUN_0052d610).
    let mut s = setup();
    for key in [s.yard, s.there] {
        s.world.systems[key].control = ControlKind::Controlled(Faction::Alliance);
    }
    let far = system(&mut s.world, 400);
    s.world.systems[far].control = ControlKind::Controlled(Faction::Alliance);
    s.world.systems[s.yard].raw_materials = 0;
    let kind = BuildableKind::ProductionFacility(crate::manufacturing::FacilityBuild {
        class: DatId::new(0x2c00_0001),
        is_alliance: true,
    });
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(kind, 5).delivered_to(s.there));
    reconcile(&mut s.world, &s.manufacturing);
    let id = s.world.products.all()[0].id;
    s.world.systems[s.there].control = ControlKind::Controlled(Faction::Empire);
    let routed = autoroute_facilities(&mut s.world, s.there, Faction::Empire);
    assert_eq!(routed.moved, vec![(id, far)]);
}

#[test]
fn an_autorouted_facility_takes_the_nearest_system_on_both_axes() {
    // FUN_00554e80 walks FUN_00554150's row, sorted by the FUN_0055d860
    // distance (isqrt of dx^2 + dy^2).
    let mut s = setup();
    let (id, _, _) = alliance_yard_en_route(&mut s);
    s.world.systems[s.there].y = 300;
    // From (200, 300): (290, 370) is 114 away, (0, 300) 200, (290, 0)
    // 313, the yard at (0, 0) 360.
    let at = |s: &mut Setup, x: u16, y: u16| {
        let key = system(&mut s.world, x);
        s.world.systems[key].y = y;
        s.world.systems[key].control = ControlKind::Controlled(Faction::Alliance);
        key
    };
    let near = at(&mut s, 290, 370);
    at(&mut s, 0, 300);
    at(&mut s, 290, 0);
    s.world.systems[s.there].control = ControlKind::Controlled(Faction::Empire);
    let routed = autoroute_facilities(&mut s.world, s.there, Faction::Empire);
    assert_eq!(routed.moved, vec![(id, near)]);
}

#[test]
fn a_system_its_facilities_already_fill_takes_no_autorouted_facility() {
    // System_vt78 (FUN_00507750): the room is the energy less every
    // facility there, built or a product (mode 3); for a mine, raw
    // materials less the mines there too.
    let mut s = setup();
    let (id, _, _) = alliance_yard_en_route(&mut s);
    let far = system(&mut s.world, 600);
    s.world.systems[far].control = ControlKind::Controlled(Faction::Alliance);
    // The yard: energy 2, one built mine and one mine product.
    let mine = |is_alliance| {
        BuildableKind::ProductionFacility(crate::manufacturing::FacilityBuild {
            class: DatId::new(0x2c00_0001),
            is_alliance,
        })
    };
    s.world.systems[s.yard].total_energy = 2;
    let built = s
        .world
        .production_facilities
        .insert(crate::world::ProductionFacilityInstance {
            class_dat_id: DatId::new(0x2c00_0001),
            side: Faction::Alliance,
            is_mine: true,
        });
    s.world.systems[s.yard].production_facilities.push(built);
    s.manufacturing
        .enqueue(s.there, QueueItem::new(mine(true), 5).delivered_to(s.yard));
    reconcile(&mut s.world, &s.manufacturing);
    s.world.systems[s.there].control = ControlKind::Controlled(Faction::Empire);
    let routed = autoroute_facilities(&mut s.world, s.there, Faction::Empire);
    assert_eq!(routed.moved, vec![(id, far)]);
    // With a third energy slot the yard takes the construction yard, but
    // not a mine while its raw materials (2) are taken by the two mines.
    s.world.systems[s.yard].total_energy = 3;
    s.world.systems[s.yard].raw_materials = 2;
    assert_eq!(
        facility_place(&s.world, true, (200, 0), mine(true)),
        Some(far)
    );
    let yard_kind = s.world.products.get(id).unwrap().kind;
    assert_eq!(
        facility_place(&s.world, true, (200, 0), yard_kind),
        Some(s.yard)
    );
}

#[test]
fn a_facility_product_of_the_new_holders_side_stays() {
    // FUN_005545e0's first step (FUN_00554740) accepts the current
    // container when it takes the facility: an Imperial product in a
    // system turning Imperial stays.
    let mut s = setup();
    let kind = BuildableKind::ProductionFacility(crate::manufacturing::FacilityBuild {
        class: DatId::new(0x2c00_0001),
        is_alliance: false,
    });
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(kind, 5).delivered_to(s.there));
    reconcile(&mut s.world, &s.manufacturing);
    // An Alliance system with room nearby takes nothing from it.
    let rebel = system(&mut s.world, 210);
    s.world.systems[rebel].control = ControlKind::Controlled(Faction::Alliance);
    let routed = autoroute_facilities(&mut s.world, s.there, Faction::Empire);
    assert_eq!(routed, Autorouted::default());
    let product = &s.world.products.all()[0];
    assert_eq!(product.container, Container::System(s.there));
}
