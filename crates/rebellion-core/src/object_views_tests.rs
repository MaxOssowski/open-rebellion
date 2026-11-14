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

/// port: a KDY-150 (DEFFACSD family 0x22) on `at`, the Empire's.
fn battery(world: &mut GameWorld, at: SystemKey) -> crate::ids::DefenseFacilityKey {
    let key = world
        .defense_facilities
        .insert(crate::world::DefenseFacilityInstance {
            class_dat_id: DatId::new(0x2200_0001),
            side: Faction::Empire,
        });
    world.systems[at].defense_facilities.push(key);
    key
}

fn alliance_system_view(setup: &Setup) -> Option<SystemView> {
    let id = setup.world.systems[setup.chandrila].dat_id;
    setup
        .world
        .object_views
        .system(Faction::Alliance, id)
        .cloned()
}

#[test]
fn a_seen_system_is_copied_live_with_its_facilities_and_fields() {
    // FUN_0050ac80: the Alliance fleet sets the seen bit; FUN_004f7bf0: the
    // system's facilities take it.
    let mut setup = setup();
    let chandrila = setup.chandrila;
    battery(&mut setup.world, chandrila);
    setup.world.systems[chandrila].total_energy = 7;
    step(&mut setup);
    assert_eq!(alliance_system_view(&setup), None);
    fleet(&mut setup.world, chandrila, true);
    step(&mut setup);
    let view = alliance_system_view(&setup).unwrap();
    assert!(view.live);
    assert_eq!(view.total_energy, 7);
    assert_eq!(view.control, ControlKind::Controlled(Faction::Empire));
    assert_eq!(
        view.defense_facilities,
        [FacilityView {
            class: DatId::new(0x2200_0001),
            side: Faction::Empire,
            is_mine: false,
        }]
    );
}

#[test]
fn an_unseen_system_keeps_its_copy_as_it_was() {
    // FUN_004f9a60: the seen bit gone, the copy freezes (FUN_005404d0).
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let watcher = fleet(&mut setup.world, chandrila, true);
    step(&mut setup);
    setup.world.fleets.remove(watcher);
    step(&mut setup);
    battery(&mut setup.world, chandrila);
    setup.world.systems[chandrila].control = ControlKind::Uncontrolled;
    step(&mut setup);
    let view = alliance_system_view(&setup).unwrap();
    assert!(!view.live);
    assert!(view.defense_facilities.is_empty());
    assert_eq!(view.control, ControlKind::Controlled(Faction::Empire));
    // Seen again, it is rebuilt.
    fleet(&mut setup.world, chandrila, true);
    step(&mut setup);
    let view = alliance_system_view(&setup).unwrap();
    assert!(view.live);
    assert_eq!(view.defense_facilities.len(), 1);
    assert_eq!(view.control, ControlKind::Uncontrolled);
}

#[test]
fn a_push_copies_the_system_frozen() {
    // Event319_Handle (FUN_0050d5a0) levels 8 and 10 copy the whole system.
    let mut setup = setup();
    let chandrila = setup.chandrila;
    battery(&mut setup.world, chandrila);
    setup
        .world
        .object_views
        .queue_push(Faction::Alliance, chandrila);
    step(&mut setup);
    let view = alliance_system_view(&setup).unwrap();
    assert!(!view.live);
    assert_eq!(view.defense_facilities.len(), 1);
}

#[test]
fn a_system_copy_holds_its_squadrons_special_forces_and_characters() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let tie = setup
        .world
        .fighter_classes
        .insert(crate::world::FighterClass {
            dat_id: DatId::new(0x1c00_0005),
            ..Default::default()
        });
    setup.world.systems[chandrila]
        .fighters
        .push(crate::world::FighterEntry {
            class: tie,
            count: 3,
            carrier: None,
        });
    let force = setup
        .world
        .special_forces
        .insert(crate::world::SpecialForceUnit {
            class_dat_id: DatId::new(0x3c00_0002),
            is_alliance: false,
            skills: [0; 8],
            on_mission: false,
        });
    setup.world.systems[chandrila].special_forces.push(force);
    let vader = DatId::new(0x3000_0001);
    setup.world.characters.insert(crate::world::Character {
        dat_id: vader,
        current_system: Some(chandrila),
        ..Default::default()
    });
    fleet(&mut setup.world, chandrila, true);
    // A killed character is not copied.
    setup.world.characters.insert(crate::world::Character {
        dat_id: DatId::new(0x3000_0002),
        current_system: Some(chandrila),
        is_killed: true,
        ..Default::default()
    });
    step(&mut setup);
    let view = alliance_system_view(&setup).unwrap();
    assert_eq!(view.fighters, [(DatId::new(0x1c00_0005), 3)]);
    assert_eq!(view.special_forces, [(DatId::new(0x3c00_0002), false)]);
    assert_eq!(view.characters, [vader]);
}

#[test]
fn seeding_copies_each_system_in_the_sides_fog() {
    let mut setup = setup();
    let mut fog_alliance = FogState::new(Faction::Alliance);
    fog_alliance.reveal(setup.chandrila);
    let fog_empire = FogState::new(Faction::Empire);
    seed_systems_from_fog(
        &mut setup.world,
        &setup.movement,
        [&fog_alliance, &fog_empire],
    );
    assert!(!alliance_system_view(&setup).unwrap().live);
    let coruscant = setup.world.systems[setup.coruscant].dat_id;
    assert!(setup
        .world
        .object_views
        .system(Faction::Alliance, coruscant)
        .is_none());
    // The Empire holds both: they are live in its copies.
    let empire = &setup.world.object_views.empire_systems;
    assert_eq!(empire.len(), 2);
    assert!(empire.values().all(|view| view.live));
}

fn day(setup: &mut Setup, missions: &crate::missions::MissionState) {
    advance_extras(
        &mut setup.world,
        &setup.movement,
        missions,
        &setup.transport.squadrons,
    );
    step(setup);
}

fn alliance_extras(setup: &Setup, at: SystemKey) -> Option<SystemExtras> {
    let id = setup.world.systems[at].dat_id;
    setup
        .world
        .object_views
        .system_extras(Faction::Alliance, id)
        .cloned()
}

#[test]
fn a_seen_systems_fleets_are_copied_and_stay_when_the_side_looks_away() {
    // FUN_004f9a60: an Imperial fleet in Chandrila copied while the
    // Alliance's fleet there sets the seen bit (FUN_0050ac80), then frozen:
    // it stays listed after it leaves.
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let missions = crate::missions::MissionState::new();
    let imperial = setup.world.insert_fleet(Fleet {
        location: chandrila,
        capital_ships: vec![],
        fighters: vec![],
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    let watcher = setup.world.insert_fleet(Fleet {
        location: chandrila,
        capital_ships: vec![],
        fighters: vec![],
        characters: vec![],
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    day(&mut setup, &missions);
    let extras = alliance_extras(&setup, chandrila).unwrap();
    assert!(extras.live);
    let serials: Vec<u64> = extras.fleets.iter().map(|fleet| fleet.serial).collect();
    assert_eq!(
        serials,
        [
            setup.world.fleet_serial(imperial),
            setup.world.fleet_serial(watcher)
        ]
    );
    assert!(!extras.fleets[0].is_alliance);
    setup.world.fleets.remove(watcher);
    day(&mut setup, &missions);
    setup.world.fleets.remove(imperial);
    day(&mut setup, &missions);
    let extras = alliance_extras(&setup, chandrila).unwrap();
    assert!(!extras.live);
    assert_eq!(extras.fleets.len(), 1);
    assert!(!extras.fleets[0].is_alliance);
}

#[test]
fn a_seen_systems_missions_are_copied_by_id_and_side() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let mut missions = crate::missions::MissionState::new();
    let agent = setup.world.characters.insert(crate::world::Character {
        dat_id: DatId::new(0x3000_0009),
        is_empire: true,
        current_system: Some(setup.coruscant),
        ..Default::default()
    });
    missions.dispatch(crate::missions::MissionRequest::single(
        crate::missions::MissionKind::Diplomacy,
        crate::missions::MissionFaction::Empire,
        agent,
        chandrila,
        None,
        0,
    ));
    fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    let extras = alliance_extras(&setup, chandrila).unwrap();
    assert_eq!(extras.missions.len(), 1);
    assert_eq!(extras.missions[0].1, 2);
}

#[test]
fn a_squadron_seen_leaving_for_an_unseen_system_is_copied_there_en_route() {
    // FUN_00556390 puts it in its destination at once, in hyperspace;
    // FUN_004f9a60 freezes the copy there (+0x50 bit 4), as A0 OBJ-02-C010's
    // regiment.
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let missions = crate::missions::MissionState::new();
    let tie = setup
        .world
        .fighter_classes
        .insert(crate::world::FighterClass {
            dat_id: DatId::new(0x1c00_0005),
            is_alliance: false,
            ..Default::default()
        });
    fleet(&mut setup.world, chandrila, true);
    setup
        .transport
        .squadrons
        .push_transit_for_test(crate::squadrons::SquadronTransit {
            class: tie,
            count: 2,
            origin: chandrila,
            to: crate::squadrons::SquadronPlace::Planet(coruscant),
            arrival_tick: 171,
        });
    day(&mut setup, &missions);
    let extras = alliance_extras(&setup, coruscant).unwrap();
    assert!(!extras.live);
    assert_eq!(
        extras.travellers,
        [TravellerCopy::Squadron {
            class: DatId::new(0x1c00_0005),
            is_alliance: false,
            count: 2,
            arrival: 171,
        }]
    );
    assert_eq!(extras.travellers[0].arrival(), 171);
    // Seen live, the destination's copy is rebuilt without it.
    fleet(&mut setup.world, coruscant, true);
    day(&mut setup, &missions);
    assert!(alliance_extras(&setup, coruscant)
        .unwrap()
        .travellers
        .is_empty());
}

#[test]
fn a_character_seen_the_day_before_and_now_on_its_way_is_copied_en_route() {
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let mut missions = crate::missions::MissionState::new();
    let vader = DatId::new(0x3000_0001);
    let key = setup.world.characters.insert(crate::world::Character {
        dat_id: vader,
        is_empire: true,
        current_system: Some(chandrila),
        ..Default::default()
    });
    fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    // He leaves for unseen Coruscant.
    setup.world.characters[key].current_system = None;
    missions.push_transit(crate::missions::MemberTransit {
        member: crate::missions::MissionMember::Character(key),
        mission_id: 0,
        to: coruscant,
        arrival: 40,
        abode: None,
    });
    day(&mut setup, &missions);
    assert_eq!(
        alliance_extras(&setup, coruscant).unwrap().travellers,
        [TravellerCopy::Character {
            id: vader,
            arrival: 40
        }]
    );
    // An Alliance character is the Alliance's own.
    assert!(setup
        .world
        .object_views
        .empire_extras
        .values()
        .all(|e| e.travellers.is_empty()));
}

#[test]
fn seeding_extras_copies_each_system_in_the_sides_fog() {
    let mut setup = setup();
    let mut fog_alliance = FogState::new(Faction::Alliance);
    fog_alliance.reveal(setup.chandrila);
    let fog_empire = FogState::new(Faction::Empire);
    let missions = crate::missions::MissionState::new();
    seed_extras_from_fog(
        &mut setup.world,
        &setup.movement,
        &missions,
        [&fog_alliance, &fog_empire],
    );
    assert!(!alliance_extras(&setup, setup.chandrila).unwrap().live);
    assert!(alliance_extras(&setup, setup.coruscant).is_none());
    assert_eq!(setup.world.object_views.empire_extras.len(), 2);
}

#[test]
fn a_fleet_in_hyperspace_is_not_in_its_systems_copy() {
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let missions = crate::missions::MissionState::new();
    let moving = setup.world.insert_fleet(Fleet {
        location: chandrila,
        capital_ships: vec![],
        fighters: vec![],
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    assert!(setup.movement.order(moving, chandrila, coruscant, 10));
    fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    let extras = alliance_extras(&setup, chandrila).unwrap();
    assert!(extras.fleets.iter().all(|copy| copy.is_alliance));
}

#[test]
fn only_a_unit_seen_the_day_before_is_copied_leaving_and_only_for_an_unseen_place() {
    // port: a member's transit names no origin, so the side must have held
    // it in a live copy the day before; an abode or a seen destination
    // takes no traveller copy.
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let mut missions = crate::missions::MissionState::new();
    let seen = setup
        .world
        .special_forces
        .insert(crate::world::SpecialForceUnit {
            class_dat_id: DatId::new(0x3c00_0007),
            is_alliance: false,
            skills: [0; 8],
            on_mission: false,
        });
    setup.world.systems[chandrila].special_forces.push(seen);
    let unseen = setup
        .world
        .special_forces
        .insert(crate::world::SpecialForceUnit {
            class_dat_id: DatId::new(0x3c00_0006),
            is_alliance: false,
            skills: [0; 8],
            on_mission: false,
        });
    let stranger = setup.world.characters.insert(crate::world::Character {
        dat_id: DatId::new(0x3000_0005),
        is_empire: true,
        ..Default::default()
    });
    fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    setup.world.systems[chandrila].special_forces.clear();
    for member in [
        crate::missions::MissionMember::SpecialForce(seen),
        crate::missions::MissionMember::SpecialForce(unseen),
        crate::missions::MissionMember::Character(stranger),
    ] {
        missions.push_transit(crate::missions::MemberTransit {
            member,
            mission_id: 0,
            to: coruscant,
            arrival: 50,
            abode: None,
        });
    }
    day(&mut setup, &missions);
    assert_eq!(
        alliance_extras(&setup, coruscant).unwrap().travellers,
        [TravellerCopy::SpecialForce {
            class: DatId::new(0x3c00_0007),
            is_alliance: false,
            arrival: 50,
        }]
    );
    // To a seen destination: no copy (the live system shows it).
    let mut setup = self::setup();
    let chandrila = setup.chandrila;
    let vader = setup.world.characters.insert(crate::world::Character {
        dat_id: DatId::new(0x3000_0001),
        is_empire: true,
        current_system: Some(chandrila),
        ..Default::default()
    });
    let mut missions = crate::missions::MissionState::new();
    fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    setup.world.characters[vader].current_system = None;
    missions.push_transit(crate::missions::MemberTransit {
        member: crate::missions::MissionMember::Character(vader),
        mission_id: 0,
        to: chandrila,
        arrival: 50,
        abode: None,
    });
    day(&mut setup, &missions);
    assert!(alliance_extras(&setup, chandrila)
        .unwrap()
        .travellers
        .is_empty());
}
