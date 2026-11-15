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
fn an_ordered_ship_waits_unfinished_in_a_new_numbered_spare_outside_the_system_list() {
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
    assert!(s
        .world
        .fleet_name(fleet)
        .is_some_and(|name| name.starts_with("Fleet ")));
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
fn a_facility_order_makes_no_product() {
    // port: facilities keep their queue-built pages (unfinished-facility.md).
    let mut s = setup();
    let build = crate::manufacturing::FacilityBuild {
        class: DatId::new(0x2c00_0001),
        is_alliance: false,
    };
    s.order(BuildableKind::ProductionFacility(build), 1);
    assert!(s.world.products.all().is_empty());
    assert!(s.world.fleets.is_empty());
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
    assert!(move_to_target(&mut s.world, id, s.there, None));
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
    assert!(move_to_target(&mut s.world, id, s.yard, Some(listed)));
    assert_eq!(s.world.products.in_fleet(listed).count(), 1);
    assert!(!s.world.fleets.contains_key(new));
    // To the system with a spare there, it joins the spare.
    s.world.set_ship_spare(s.there, false, listed);
    s.world.fleets[listed].location = s.there;
    assert!(move_to_target(&mut s.world, id, s.there, None));
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
    assert!(!move_to_target(&mut s.world, id, s.there, Some(fleet)));
    assert!(move_to_target(&mut s.world, id, s.there, None));
    assert_eq!(
        s.world.products.all()[0].container,
        Container::System(s.there)
    );
    assert!(!move_to_target(&mut s.world, 99, s.there, None));
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
    assert!(move_to_target(&mut s.world, ids[1], s.there, Some(rebels)));
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
    assert!(!move_to_target(&mut s.world, id, s.yard, None));
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
    assert!(!move_to_target(&mut s.world, id, s.yard, None));
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
    s.manufacturing
        .enqueue(s.yard, QueueItem::new(BuildableKind::Fighter(alliance_wing), 5));
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
