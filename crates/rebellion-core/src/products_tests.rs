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
    assert!(settled.completions.is_empty());
    assert_eq!(
        settled.departures,
        vec![Departure {
            origin: s.yard,
            destination: s.there,
            tick: 12,
            kind: BuildableKind::Fighter(s.tie),
            name: None,
        }]
    );
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
