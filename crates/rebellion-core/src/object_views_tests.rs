use super::*;
use crate::dat::{ExplorationStatus, SectorGroup};
use crate::troop_transport::RegimentTransit;
use crate::world::{ControlKind, Fleet, Sector, System, TroopUnit};

/// An Imperial regiment class (TROOPSD family 0x10). port: test data.
const ARMY: DatId = DatId(0x1000_0002);

struct Setup {
    world: GameWorld,
    transport: TroopTransportState,
    movement: MovementState,
    coruscant: SystemKey,
    chandrila: SystemKey,
    regiment: TroopKey,
}

/// port: a test system with SYSTEMSD's family 0x90 id.
fn system(world: &mut GameWorld, id: u32, name: &str, control: ControlKind) -> SystemKey {
    let sector = world.sectors.keys().next().unwrap_or_else(|| {
        world.sectors.insert(Sector {
            dat_id: DatId::new(0x9200_0000),
            name: "Sector".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        })
    });
    world.systems.insert(System {
        last_arrival: None,
        dat_id: DatId::new(0x9000_0000 | id),
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
        control,
    })
}

/// Chandrila (Imperial) with an Imperial regiment on its surface, and
/// Coruscant (Imperial). port: test data.
fn setup() -> Setup {
    let mut world = GameWorld::default();
    let empire = ControlKind::Controlled(Faction::Empire);
    let coruscant = system(&mut world, 1, "Coruscant", empire);
    let chandrila = system(&mut world, 2, "Chandrila", empire);
    let regiment = world.troops.insert(TroopUnit {
        class_dat_id: ARMY,
        is_alliance: false,
        regiment_strength: 5,
    });
    world.systems[chandrila].ground_units.push(regiment);
    Setup {
        world,
        transport: TroopTransportState::new(),
        movement: MovementState::new(),
        coruscant,
        chandrila,
        regiment,
    }
}

fn fleet(world: &mut GameWorld, at: SystemKey, is_alliance: bool) -> crate::ids::FleetKey {
    world.fleets.insert(Fleet {
        location: at,
        capital_ships: vec![],
        fighters: vec![],
        characters: vec![],
        is_alliance,
        has_death_star: false,
        special_forces: Vec::new(),
    })
}

fn step(setup: &mut Setup) {
    advance(&mut setup.world, &setup.transport, &setup.movement);
}

fn alliance_view(setup: &Setup) -> Option<RegimentView> {
    let id = setup.world.object_views.regiment_id(setup.regiment)?;
    match setup.world.object_views.view(Faction::Alliance, id)? {
        ObjectView::Regiment(view) => Some(view.clone()),
    }
}

/// Takes the regiment off `from`'s surface onto the road to `to`.
fn send_to(setup: &mut Setup, from: SystemKey, to: SystemKey, arrival: u64) {
    let regiment = setup.regiment;
    setup.world.systems[from]
        .ground_units
        .retain(|key| *key != regiment);
    setup.transport.push_transit(RegimentTransit {
        troop: regiment,
        origin: from,
        leg: RegimentLeg::Surface(to),
        arrival_tick: arrival,
    });
}

/// The regiment lands on `at`.
fn land(setup: &mut Setup, at: SystemKey) {
    let regiment = setup.regiment;
    assert!(setup.transport.forget_transit(regiment));
    setup.world.systems[at].ground_units.push(regiment);
}

#[test]
fn a_queued_push_is_taken_by_the_days_step() {
    // Espionage's success queues Event319_Handle level 10 (FUN_00573610);
    // the day's step copies the system's regiments for the side.
    let mut setup = setup();
    let chandrila = setup.chandrila;
    setup
        .world
        .object_views
        .queue_push(Faction::Alliance, chandrila);
    step(&mut setup);
    assert!(!alliance_view(&setup).unwrap().live);
    assert!(setup.world.object_views.pushes.is_empty());
}

#[test]
fn a_gone_regiments_id_names_no_live_regiment() {
    let mut setup = setup();
    step(&mut setup);
    let id = setup
        .world
        .object_views
        .regiment_id(setup.regiment)
        .unwrap();
    setup.world.troops.remove(setup.regiment);
    assert_eq!(
        setup.world.object_views.regiment_key(&setup.world, id),
        None
    );
    assert_eq!(
        setup
            .world
            .object_views
            .regiment_key(&setup.world, DatId::new(0x1000_0099)),
        None
    );
}

#[test]
fn a_regiment_gets_an_id_of_its_class_family_and_keeps_it() {
    let mut setup = setup();
    step(&mut setup);
    let id = setup
        .world
        .object_views
        .regiment_id(setup.regiment)
        .unwrap();
    assert_eq!(id, DatId::new(0x1000_0001));
    step(&mut setup);
    assert_eq!(
        setup.world.object_views.regiment_id(setup.regiment),
        Some(id)
    );
    assert_eq!(
        setup.world.object_views.regiment_key(&setup.world, id),
        Some(setup.regiment)
    );
    let second = setup.world.troops.insert(TroopUnit {
        class_dat_id: DatId(0x1100_0001),
        is_alliance: true,
        regiment_strength: 1,
    });
    step(&mut setup);
    assert_eq!(
        setup.world.object_views.regiment_id(second),
        Some(DatId::new(0x1100_0002))
    );
    setup.world.troops.remove(second);
    step(&mut setup);
    assert_eq!(setup.world.object_views.regiment_ids.len(), 1);
}

#[test]
fn a_side_holds_no_view_of_an_object_it_never_saw() {
    // FUN_004f9a60: no seen bit, so the copy never goes live.
    let mut setup = setup();
    step(&mut setup);
    assert_eq!(alliance_view(&setup), None);
    assert!(setup.world.object_views.empire.is_empty());
}

#[test]
fn a_fleet_in_the_system_makes_the_sides_view_live() {
    // FUN_0050ac80: the Alliance's seen bit with an Alliance fleet in
    // Chandrila; FUN_004f7bf0: the regiment takes it.
    let mut setup = setup();
    let chandrila = setup.chandrila;
    fleet(&mut setup.world, chandrila, true);
    step(&mut setup);
    let view = alliance_view(&setup).unwrap();
    assert!(view.live);
    assert_eq!(view.place, ViewPlace::Surface(DatId::new(0x9000_0002)));
    assert_eq!(view.arrival, None);
    assert_eq!(view.strength, 5);
    assert_eq!(view.class, ARMY);
    assert!(!view.is_alliance);
}

#[test]
fn the_other_sides_fleet_does_not_see_for_the_side() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    fleet(&mut setup.world, chandrila, false);
    step(&mut setup);
    assert_eq!(alliance_view(&setup), None);
}

#[test]
fn holding_the_system_sees_it_without_a_fleet() {
    // FUN_0050ac80: the system's own side bits.
    let mut setup = setup();
    setup.world.systems[setup.chandrila].control = ControlKind::Controlled(Faction::Alliance);
    step(&mut setup);
    assert!(alliance_view(&setup).unwrap().live);
}

#[test]
fn a_fleet_in_hyperspace_does_not_see_the_system_it_left() {
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let moving = fleet(&mut setup.world, chandrila, true);
    assert!(setup.movement.order(moving, chandrila, coruscant, 10));
    step(&mut setup);
    assert_eq!(alliance_view(&setup), None);
}

#[test]
fn a_regiment_leaving_a_seen_system_freezes_en_route_with_its_eta() {
    // A0 OBJ-02-C010 (18-chandrila-army-regiment-status-enroute-eta-
    // day171-on-day576-stale, 6a1269a1765f7c9b): the copy reads "Enroute,
    // ETA Destination Day 171" long after. FUN_004f9a60: en route
    // (+0x50 bit 4) freezes the copy.
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let watcher = fleet(&mut setup.world, chandrila, true);
    step(&mut setup);
    assert!(alliance_view(&setup).unwrap().live);
    send_to(&mut setup, chandrila, coruscant, 171);
    step(&mut setup);
    let frozen = alliance_view(&setup).unwrap();
    assert!(!frozen.live);
    assert_eq!(frozen.arrival, Some(171));
    // An object in hyperspace is in its destination (FUN_00556390).
    assert_eq!(frozen.place, ViewPlace::Surface(DatId::new(0x9000_0001)));
    // It lands on Coruscant, which no Alliance fleet sees: the view stays
    // en route.
    setup.world.fleets.remove(watcher);
    land(&mut setup, coruscant);
    step(&mut setup);
    step(&mut setup);
    assert_eq!(alliance_view(&setup), Some(frozen));
}

#[test]
fn a_frozen_view_goes_live_again_in_full_when_the_side_sees_it() {
    // FUN_004f9a60: frozen (2) to live (1) rebuilds the copy (vtable
    // +0x24, FUN_004f8340).
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let watcher = fleet(&mut setup.world, chandrila, true);
    step(&mut setup);
    send_to(&mut setup, chandrila, coruscant, 171);
    step(&mut setup);
    land(&mut setup, coruscant);
    let regiment = setup.regiment;
    setup.world.troops[regiment].regiment_strength = 2;
    step(&mut setup);
    assert!(!alliance_view(&setup).unwrap().live);
    setup.world.fleets[watcher].location = coruscant;
    step(&mut setup);
    let view = alliance_view(&setup).unwrap();
    assert!(view.live);
    assert_eq!(view.arrival, None);
    assert_eq!(view.strength, 2);
    assert_eq!(view.place, ViewPlace::Surface(DatId::new(0x9000_0001)));
}

#[test]
fn a_fleet_leaving_freezes_the_view_as_it_stood() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let watcher = fleet(&mut setup.world, chandrila, true);
    step(&mut setup);
    setup.world.fleets.remove(watcher);
    step(&mut setup);
    setup.world.troops[setup.regiment].regiment_strength = 1;
    step(&mut setup);
    let view = alliance_view(&setup).unwrap();
    assert!(!view.live);
    assert_eq!(view.strength, 5);
}

#[test]
fn a_live_view_follows_each_change() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    fleet(&mut setup.world, chandrila, true);
    step(&mut setup);
    setup.world.troops[setup.regiment].regiment_strength = 3;
    step(&mut setup);
    assert_eq!(alliance_view(&setup).unwrap().strength, 3);
}

#[test]
fn a_regiment_destroyed_in_sight_drops_its_view_and_one_out_of_sight_stays() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let regiment = setup.regiment;
    let watcher = fleet(&mut setup.world, chandrila, true);
    step(&mut setup);
    let id = setup.world.object_views.regiment_id(regiment).unwrap();
    // Out of sight: the frozen copy stays (hyp).
    let mut out_of_sight = setup.world.clone();
    out_of_sight.fleets.remove(watcher);
    advance(&mut out_of_sight, &setup.transport, &setup.movement);
    out_of_sight.troops.remove(regiment);
    out_of_sight.systems[chandrila].ground_units.clear();
    advance(&mut out_of_sight, &setup.transport, &setup.movement);
    assert!(out_of_sight
        .object_views
        .view(Faction::Alliance, id)
        .is_some());
    // In sight: dropped.
    setup.world.troops.remove(regiment);
    setup.world.systems[chandrila].ground_units.clear();
    step(&mut setup);
    assert!(setup
        .world
        .object_views
        .view(Faction::Alliance, id)
        .is_none());
}

#[test]
fn the_own_sides_objects_get_no_view() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    fleet(&mut setup.world, chandrila, true);
    step(&mut setup);
    assert!(setup.world.object_views.empire.is_empty());
    assert_eq!(setup.world.object_views.alliance.len(), 1);
}

#[test]
fn an_intelligence_push_copies_the_systems_regiments_frozen() {
    // Event319_Handle (FUN_0050d5a0) level 10 for Espionage
    // (FUN_00573610), level 8 for Reconnaissance (FUN_0056bee0).
    let mut setup = setup();
    number_regiments(&mut setup.world);
    let updates = push_system(
        &setup.world,
        &setup.transport,
        &setup.movement,
        Faction::Alliance,
        setup.chandrila,
    );
    apply(&mut setup.world, &updates);
    let view = alliance_view(&setup).unwrap();
    assert!(!view.live);
    assert_eq!(view.strength, 5);
    assert!(push_system(
        &setup.world,
        &setup.transport,
        &setup.movement,
        Faction::Alliance,
        setup.coruscant,
    )
    .is_empty());
    // A frozen copy from a push does not follow the regiment.
    setup.world.troops[setup.regiment].regiment_strength = 1;
    step(&mut setup);
    assert_eq!(alliance_view(&setup), Some(view));
}

#[test]
fn seeding_copies_the_regiments_of_systems_in_the_sides_fog() {
    let mut setup = setup();
    let mut fog_alliance = FogState::new(Faction::Alliance);
    let fog_empire = FogState::new(Faction::Empire);
    seed_from_fog(
        &mut setup.world,
        &setup.transport,
        &setup.movement,
        [&fog_alliance, &fog_empire],
    );
    assert_eq!(alliance_view(&setup), None);
    fog_alliance.reveal(setup.chandrila);
    seed_from_fog(
        &mut setup.world,
        &setup.transport,
        &setup.movement,
        [&fog_alliance, &fog_empire],
    );
    let view = alliance_view(&setup).unwrap();
    assert!(!view.live);
    assert_eq!(view.strength, 5);
}

#[test]
fn seeding_copies_a_seen_systems_regiments_live() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    fleet(&mut setup.world, chandrila, true);
    let fog_alliance = FogState::new(Faction::Alliance);
    let fog_empire = FogState::new(Faction::Empire);
    seed_from_fog(
        &mut setup.world,
        &setup.transport,
        &setup.movement,
        [&fog_alliance, &fog_empire],
    );
    assert!(alliance_view(&setup).unwrap().live);
}

#[test]
fn a_regiment_in_a_fleets_cargo_is_placed_in_the_fleets_system() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let regiment = setup.regiment;
    let carrier = fleet(&mut setup.world, chandrila, false);
    setup.world.systems[chandrila].ground_units.clear();
    setup.transport.push_cargo(carrier, regiment);
    fleet(&mut setup.world, chandrila, true);
    step(&mut setup);
    let view = alliance_view(&setup).unwrap();
    assert!(view.live);
    assert_eq!(
        view.place,
        ViewPlace::Fleet {
            system: DatId::new(0x9000_0002),
            fleet_name: setup
                .world
                .fleet_name(carrier)
                .unwrap_or_default()
                .to_owned(),
        }
    );
}
