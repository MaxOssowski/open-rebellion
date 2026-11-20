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
            damage: None,
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
        &setup.transport,
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
            lost: Vec::new(),
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
fn a_character_seen_leaving_for_an_unseen_system_is_copied_en_route_there() {
    // FUN_00556390 makes the destination the traveller's container and
    // FUN_004f9a60 freezes the side's copy with +0x50 bit 4 (en route), so
    // FUN_00465bb0 reads "Name - Destination ( Enroute ) " (0x8744).
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
    assert_eq!(
        setup
            .world
            .object_views
            .character_state(Faction::Alliance, vader),
        CharacterCopyState::default()
    );
    setup.world.characters[key].current_system = None;
    missions.push_transit(crate::missions::MemberTransit {
        member: crate::missions::MissionMember::Character(key),
        mission_id: 0,
        to: coruscant,
        arrival: 40,
        abode: None,
    });
    day(&mut setup, &missions);
    let views = &setup.world.object_views;
    assert_eq!(
        views.character(Faction::Alliance, vader),
        Some(&CharacterCopy {
            system: Some(setup.world.systems[coruscant].dat_id),
            ship: None,
        })
    );
    assert_eq!(
        views.character_state(Faction::Alliance, vader),
        CharacterCopyState {
            on_mission: false,
            hidden_mission: false,
            enroute: true,
        }
    );
    // The next days keep it so; once he is seen at Coruscant, the copy is
    // live there with no state.
    day(&mut setup, &missions);
    assert!(
        setup
            .world
            .object_views
            .character_state(Faction::Alliance, vader)
            .enroute
    );
    missions = crate::missions::MissionState::new();
    setup.world.characters[key].current_system = Some(coruscant);
    fleet(&mut setup.world, coruscant, true);
    day(&mut setup, &missions);
    assert_eq!(
        setup
            .world
            .object_views
            .character_state(Faction::Alliance, vader),
        CharacterCopyState::default()
    );
    assert_eq!(
        setup
            .world
            .object_views
            .character(Faction::Alliance, vader)
            .and_then(|copy| copy.system),
        Some(setup.world.systems[coruscant].dat_id)
    );
}

#[test]
fn a_character_copied_on_a_mission_keeps_its_mission_bits_until_seen_again() {
    // FUN_00465bb0: the copy's mission key (+0x68, FUN_0042d170) names a
    // mission, so its row reads "( On Mission )" (0x8745); FUN_00429440
    // opens the Missions window unless the mission is hidden
    // (FUN_00520b70).
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let missions = crate::missions::MissionState::new();
    let piett = DatId::new(0x3000_0002);
    let key = setup.world.characters.insert(crate::world::Character {
        dat_id: piett,
        is_empire: true,
        current_system: Some(chandrila),
        on_mission: true,
        on_hidden_mission: true,
        ..Default::default()
    });
    let watcher = fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    let state = |setup: &Setup| {
        setup
            .world
            .object_views
            .character_state(Faction::Alliance, piett)
    };
    assert_eq!(
        state(&setup),
        CharacterCopyState {
            on_mission: true,
            hidden_mission: true,
            enroute: false,
        }
    );
    // Out of sight the mission ends: the frozen copy keeps it.
    setup.world.fleets.remove(watcher);
    day(&mut setup, &missions);
    setup.world.characters[key].on_mission = false;
    setup.world.characters[key].on_hidden_mission = false;
    day(&mut setup, &missions);
    assert!(state(&setup).on_mission);
    // A captive's copy reads no mission (+0xac bit 0 comes first).
    fleet(&mut setup.world, chandrila, true);
    setup.world.characters[key].on_mission = true;
    setup.world.characters[key].is_captive = true;
    day(&mut setup, &missions);
    assert!(!state(&setup).on_mission);
    // Seen again free, the copy follows.
    setup.world.characters[key].on_mission = false;
    setup.world.characters[key].is_captive = false;
    day(&mut setup, &missions);
    assert_eq!(state(&setup), CharacterCopyState::default());
    // A stale hidden flag off a mission hides nothing.
    setup.world.characters[key].on_hidden_mission = true;
    day(&mut setup, &missions);
    assert_eq!(state(&setup), CharacterCopyState::default());
}

#[test]
fn a_characters_copy_keeps_the_command_word_it_held_when_it_froze() {
    // FUN_004ee350 sets the master's +0x96 and syncs each copy through
    // FUN_004f04f0, which writes only while FUN_005404d0 holds: a frozen
    // copy keeps its old command word for FUN_00465bb0.
    use crate::character_command::CommandKind;
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let mut missions = crate::missions::MissionState::new();
    let brandei = DatId::new(0x3000_0001);
    let key = setup.world.characters.insert(crate::world::Character {
        dat_id: brandei,
        is_empire: true,
        current_system: Some(chandrila),
        command: CommandKind::General,
        ..Default::default()
    });
    let watcher = fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    let command = |setup: &Setup| {
        setup
            .world
            .object_views
            .character_command(Faction::Alliance, brandei)
    };
    assert_eq!(command(&setup), CommandKind::General);
    // Live, the copy follows a change.
    setup.world.characters[key].command = CommandKind::Admiral;
    day(&mut setup, &missions);
    assert_eq!(command(&setup), CommandKind::Admiral);
    // Frozen, it keeps the word.
    setup.world.fleets.remove(watcher);
    day(&mut setup, &missions);
    setup.world.characters[key].command = CommandKind::None;
    day(&mut setup, &missions);
    assert_eq!(command(&setup), CommandKind::Admiral);
    // Seen again with none, the copy has none.
    let watcher = fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    assert_eq!(command(&setup), CommandKind::None);
    // A traveller seen leaving takes its word with it, en route.
    setup.world.characters[key].command = CommandKind::Commander;
    day(&mut setup, &missions);
    setup.world.characters[key].current_system = None;
    setup.world.characters[key].command = CommandKind::Commander;
    missions.push_transit(crate::missions::MemberTransit {
        member: crate::missions::MissionMember::Character(key),
        mission_id: 0,
        to: coruscant,
        arrival: 40,
        abode: None,
    });
    day(&mut setup, &missions);
    assert_eq!(command(&setup), CommandKind::Commander);
}

#[test]
fn a_copy_left_in_no_place_drops_its_command_word() {
    use crate::character_command::CommandKind;
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let missions = crate::missions::MissionState::new();
    let griff = DatId::new(0x3000_0002);
    let key = setup.world.characters.insert(crate::world::Character {
        dat_id: griff,
        is_empire: true,
        current_system: Some(chandrila),
        command: CommandKind::Admiral,
        ..Default::default()
    });
    let watcher = fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    setup.world.fleets.remove(watcher);
    day(&mut setup, &missions);
    setup.world.characters[key].current_system = Some(coruscant);
    fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    let views = &setup.world.object_views;
    assert_eq!(
        views.character(Faction::Alliance, griff),
        Some(&CharacterCopy::default())
    );
    assert_eq!(
        views.character_command(Faction::Alliance, griff),
        CommandKind::None
    );
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

#[test]
fn a_bombardment_pushes_the_system_into_both_sides_copies() {
    // Event319_Fire (FUN_005650f0): Event319_Handle level 11 for sides 1
    // and 2 after a bombardment.
    let mut setup = setup();
    let chandrila = setup.chandrila;
    crate::bombardment::apply(
        &mut setup.world,
        &crate::bombardment::BombardRequest {
            system: chandrila,
            side: Faction::Alliance,
            kind: 0,
            fleets: Vec::new(),
            alliance_hq: None,
        },
        &crate::bombardment::BombardmentOutcome::default(),
    );
    assert_eq!(
        setup.world.object_views.pushes,
        [(Faction::Alliance, chandrila), (Faction::Empire, chandrila)]
    );
}

#[test]
fn a_push_copies_the_systems_fleets_and_missions_frozen() {
    // Event319_Handle copies the system's children (fleets 0x08..0x0f).
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let missions = crate::missions::MissionState::new();
    setup.world.insert_fleet(Fleet {
        location: chandrila,
        capital_ships: vec![],
        fighters: vec![],
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    setup
        .world
        .object_views
        .queue_push(Faction::Alliance, chandrila);
    day(&mut setup, &missions);
    let extras = alliance_extras(&setup, chandrila).unwrap();
    assert!(!extras.live);
    assert_eq!(extras.fleets.len(), 1);
    assert!(setup
        .world
        .object_views
        .empire_extras
        .values()
        .all(|value| value.live));
}

fn alliance_details(setup: &Setup, at: SystemKey) -> Option<SystemDetails> {
    let id = setup.world.systems[at].dat_id;
    setup
        .world
        .object_views
        .system_details(Faction::Alliance, id)
        .cloned()
}

#[test]
fn a_frozen_copy_keeps_a_fleet_as_seen_after_it_has_moved_and_died() {
    // FUN_004f9a60 froze the Alliance's copy; GameObj_SetReadyForDelete
    // (FUN_004f82d0 -> FUN_004f6c70) waits for the destroyed bit on every
    // copy, so the side keeps its copy until it sees the system again.
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let missions = crate::missions::MissionState::new();
    let class = setup
        .world
        .capital_ship_classes
        .insert(crate::world::CapitalShipClass {
            dat_id: DatId::new(0x1400_0003),
            hull: 50,
            ..Default::default()
        });
    let tie = setup
        .world
        .fighter_classes
        .insert(crate::world::FighterClass {
            dat_id: DatId::new(0x1c00_0005),
            ..Default::default()
        });
    let vader = setup.world.characters.insert(crate::world::Character {
        dat_id: DatId::new(0x3000_0001),
        is_empire: true,
        ..Default::default()
    });
    let mut ship = crate::world::ShipInstance::new(class, 40, true);
    ship.name = Some("Victory Destroyer 1".into());
    let imperial = setup.world.insert_fleet(Fleet {
        location: chandrila,
        capital_ships: vec![ship],
        fighters: vec![crate::world::FighterEntry {
            class: tie,
            count: 3,
            carrier: None,
            damage: None,
        }],
        characters: vec![vader],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    let regiment = setup.regiment;
    setup.world.systems[chandrila].ground_units.clear();
    setup.transport.push_cargo(imperial, regiment);
    let watcher = fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    let seen = alliance_details(&setup, chandrila).unwrap();
    let copy = seen
        .fleets
        .iter()
        .find(|fleet| !fleet.is_alliance)
        .unwrap()
        .clone();
    assert_eq!(copy.name, setup.world.fleet_name(imperial).unwrap());
    assert_eq!(
        copy.ships,
        [ShipCopy {
            class: DatId::new(0x1400_0003),
            name: Some("Victory Destroyer 1".into()),
            hull: 40,
        }]
    );
    assert_eq!(copy.fighters, [(DatId::new(0x1c00_0005), 3)]);
    assert_eq!(copy.troops, [ARMY]);
    assert_eq!(copy.characters, [DatId::new(0x3000_0001)]);
    // The watcher leaves; the fleet moves on, then dies.
    setup.world.fleets.remove(watcher);
    day(&mut setup, &missions);
    setup.world.fleets[imperial].location = coruscant;
    day(&mut setup, &missions);
    setup.world.fleets.remove(imperial);
    day(&mut setup, &missions);
    let frozen = alliance_details(&setup, chandrila).unwrap();
    assert_eq!(
        frozen
            .fleets
            .iter()
            .filter(|fleet| !fleet.is_alliance)
            .cloned()
            .collect::<Vec<_>>(),
        [copy]
    );
    // Seen again, the copy is rebuilt without it.
    fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    assert!(alliance_details(&setup, chandrila)
        .unwrap()
        .fleets
        .iter()
        .all(|fleet| fleet.is_alliance));
}

#[test]
fn a_characters_copy_keeps_its_ship_until_its_system_copy_goes_live_without_it() {
    // FUN_00465bb0 names the container of the side's copy of a character
    // (a ship or a system), else 0x1895 "Location Unknown". A0
    // 2026-11-14-stale-views second run: 1x-personnelfinder-empire-d85
    // (d57ed845) "Darth Vader - Imperial Star Destroyer 3"; d91 (1b8818d8)
    // "Darth Vader - Location Unknown".
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let missions = crate::missions::MissionState::new();
    let class = setup
        .world
        .capital_ship_classes
        .insert(crate::world::CapitalShipClass {
            dat_id: DatId::new(0x1400_0001),
            name: "Imperial Star Destroyer".into(),
            ..Default::default()
        });
    let mut first = crate::world::ShipInstance::new(class, 40, true);
    first.id = 2;
    let mut ship = crate::world::ShipInstance::new(class, 40, true);
    ship.id = 3;
    ship.name = Some("Imperial Star Destroyer 3".into());
    let imperial = setup.world.insert_fleet(Fleet {
        location: chandrila,
        capital_ships: vec![first, ship],
        fighters: vec![],
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    let vader_id = DatId::new(0x3000_0001);
    let vader = setup.world.characters.insert(crate::world::Character {
        dat_id: vader_id,
        is_empire: true,
        current_fleet: Some(imperial),
        ship: Some(3),
        ..Default::default()
    });
    setup.world.fleets[imperial].characters.push(vader);
    let piett_id = DatId::new(0x3000_0002);
    setup.world.characters.insert(crate::world::Character {
        dat_id: piett_id,
        is_empire: true,
        current_system: Some(chandrila),
        ..Default::default()
    });
    // The Alliance's own character there takes no copy.
    let mon_id = DatId::new(0x3000_0003);
    setup.world.characters.insert(crate::world::Character {
        dat_id: mon_id,
        is_alliance: true,
        current_system: Some(chandrila),
        ..Default::default()
    });
    let chandrila_id = setup.world.systems[chandrila].dat_id;
    let watcher = fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    let copy = |setup: &Setup, id| {
        setup
            .world
            .object_views
            .character(Faction::Alliance, id)
            .cloned()
    };
    let aboard = CharacterCopy {
        system: Some(chandrila_id),
        ship: Some("Imperial Star Destroyer 3".into()),
    };
    assert_eq!(copy(&setup, vader_id), Some(aboard.clone()));
    assert_eq!(
        copy(&setup, piett_id),
        Some(CharacterCopy {
            system: Some(chandrila_id),
            ship: None,
        })
    );
    assert_eq!(copy(&setup, mon_id), None);
    // A character landing on a live system with its fleets unchanged is
    // copied the same day.
    let needa_id = DatId::new(0x3000_0004);
    setup.world.characters.insert(crate::world::Character {
        dat_id: needa_id,
        is_empire: true,
        current_system: Some(chandrila),
        ..Default::default()
    });
    day(&mut setup, &missions);
    assert_eq!(
        copy(&setup, needa_id).and_then(|copy| copy.system),
        Some(chandrila_id)
    );
    // The Empire holds Chandrila, so it copies the Alliance's character.
    assert_eq!(
        setup.world.object_views.character(Faction::Empire, mon_id),
        Some(&CharacterCopy {
            system: Some(chandrila_id),
            ship: None,
        })
    );
    // The watcher leaves; the fleet moves on out of sight: the copy stays.
    setup.world.fleets.remove(watcher);
    day(&mut setup, &missions);
    setup.world.fleets[imperial].location = coruscant;
    day(&mut setup, &missions);
    assert_eq!(copy(&setup, vader_id), Some(aboard));
    // Seen again without the fleet: the copy is in no place.
    let watcher = fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    assert_eq!(copy(&setup, vader_id), Some(CharacterCopy::default()));
    assert!(copy(&setup, piett_id).unwrap().system.is_some());
    // An unseated character aboard names its fleet; seen at Coruscant now.
    setup.world.fleets[watcher].location = coruscant;
    setup.world.characters[vader].ship = None;
    day(&mut setup, &missions);
    assert_eq!(
        copy(&setup, vader_id),
        Some(CharacterCopy {
            system: Some(setup.world.systems[coruscant].dat_id),
            ship: setup.world.fleet_name(imperial).map(str::to_owned),
        })
    );
}

#[test]
fn seeding_character_copies_takes_the_characters_the_system_and_fleet_copies_hold() {
    // The v120 migration: an older copy kept no seat, so a character aboard
    // is named by its copied fleet.
    let mut world = GameWorld::default();
    let empire = ControlKind::Controlled(Faction::Empire);
    let yaga = system(&mut world, 1, "Yaga Minor", empire);
    let id = world.systems[yaga].dat_id;
    let mut view = SystemView::of(&world, &world.systems[yaga], false);
    view.characters = vec![DatId::new(0x3000_0002)];
    world.object_views.alliance_systems.insert(id, view);
    let fleet = |is_alliance: bool, name: &str, character: u32| FleetDetail {
        serial: 1,
        name: name.into(),
        is_alliance,
        ships: vec![],
        fighters: vec![],
        troops: vec![],
        characters: vec![DatId::new(character)],
    };
    world.object_views.alliance_details.insert(
        id,
        SystemDetails {
            fleets: vec![
                fleet(false, "Fleet 2", 0x3000_0001),
                fleet(true, "Fleet 3", 0x3000_0003),
            ],
            ..Default::default()
        },
    );
    seed_characters_from_copies(&mut world);
    let views = &world.object_views;
    assert_eq!(
        views.character(Faction::Alliance, DatId::new(0x3000_0001)),
        Some(&CharacterCopy {
            system: Some(id),
            ship: Some("Fleet 2".into()),
        })
    );
    assert_eq!(
        views.character(Faction::Alliance, DatId::new(0x3000_0002)),
        Some(&CharacterCopy {
            system: Some(id),
            ship: None,
        })
    );
    assert_eq!(
        views.character(Faction::Alliance, DatId::new(0x3000_0003)),
        None
    );
    assert!(views.empire_characters.is_empty());
}

#[test]
fn a_seen_systems_missions_are_copied_with_their_members() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let mut missions = crate::missions::MissionState::new();
    let agent = setup.world.characters.insert(crate::world::Character {
        dat_id: DatId::new(0x3000_0009),
        is_empire: true,
        ..Default::default()
    });
    let decoy = setup
        .world
        .special_forces
        .insert(crate::world::SpecialForceUnit {
            class_dat_id: DatId::new(0x3c00_0006),
            is_alliance: false,
            skills: [0; 8],
            on_mission: true,
        });
    missions.dispatch(crate::missions::MissionRequest {
        kind: crate::missions::MissionKind::Sabotage,
        faction: crate::missions::MissionFaction::Empire,
        team: vec![crate::missions::MissionMember::Character(agent)],
        decoys: vec![crate::missions::MissionMember::SpecialForce(decoy)],
        target_system: chandrila,
        target_character: None,
        target_object: None,
        tick: 0,
    });
    fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    let details = alliance_details(&setup, chandrila).unwrap();
    let [mission] = &details.missions[..] else {
        panic!("{details:?}");
    };
    assert_eq!(mission.side, 2);
    assert_eq!(mission.kind, crate::missions::MissionKind::Sabotage);
    assert_eq!(
        mission.team,
        [MemberCopy::Character(DatId::new(0x3000_0009))]
    );
    assert_eq!(
        mission.decoys,
        [MemberCopy::SpecialForce(DatId::new(0x3c00_0006))]
    );
}

#[test]
fn seeding_details_copies_each_system_in_the_sides_fog() {
    let mut setup = setup();
    let mut fog_alliance = FogState::new(Faction::Alliance);
    fog_alliance.reveal(setup.chandrila);
    let fog_empire = FogState::new(Faction::Empire);
    let missions = crate::missions::MissionState::new();
    // An Imperial character on Chandrila, which the Alliance's fog shows
    // but it does not see.
    let piett = DatId::new(0x3000_0002);
    let chandrila = setup.chandrila;
    setup.world.characters.insert(crate::world::Character {
        dat_id: piett,
        is_empire: true,
        current_system: Some(chandrila),
        ..Default::default()
    });
    seed_details_from_fog(
        &mut setup.world,
        &setup.movement,
        &missions,
        &setup.transport,
        [&fog_alliance, &fog_empire],
    );
    assert_eq!(
        setup
            .world
            .object_views
            .character(Faction::Alliance, piett)
            .and_then(|copy| copy.system),
        Some(setup.world.systems[chandrila].dat_id)
    );
    assert!(alliance_details(&setup, setup.chandrila).is_some());
    assert!(alliance_details(&setup, setup.coruscant).is_none());
    assert_eq!(setup.world.object_views.empire_details.len(), 2);
}

#[test]
fn a_fleet_made_in_a_frozen_system_shows_there_with_nothing_in_it() {
    // A0 2026-11-14-stale-views/04-kothlis-battle-alert-d371 (no Imperial
    // fleet at Kothlis while the Alliance saw it), 05-sector-after-d374
    // and 06-kothlis-fleet16-d374-montage (Imperial Fleet 16 there after,
    // all four tabs empty), 08-fleetfinder-empire-d375 (listed): a new
    // object takes frozen side copies at once (FUN_0053f760); its ships,
    // unfinished then, never sync (FUN_005404d0).
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let missions = crate::missions::MissionState::new();
    let watcher = fleet(&mut setup.world, chandrila, true);
    day(&mut setup, &missions);
    setup.world.fleets.remove(watcher);
    day(&mut setup, &missions);
    // One that predates the copy, elsewhere, stays out of it.
    setup.world.insert_fleet(Fleet {
        location: coruscant,
        capital_ships: vec![],
        fighters: vec![],
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    let class = setup
        .world
        .capital_ship_classes
        .insert(crate::world::CapitalShipClass {
            dat_id: DatId::new(0x1400_0003),
            ..Default::default()
        });
    let made = setup.world.insert_fleet(Fleet {
        location: chandrila,
        capital_ships: vec![crate::world::ShipInstance::new(class, 10, true)],
        fighters: vec![],
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    // A new one that has already left is in hyperspace: not copied.
    let gone = setup.world.insert_fleet(Fleet {
        location: chandrila,
        capital_ships: vec![],
        fighters: vec![],
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    assert!(setup.movement.order(gone, chandrila, coruscant, 10));
    day(&mut setup, &missions);
    let copy = alliance_details(&setup, chandrila).unwrap();
    let imperial: Vec<&FleetDetail> = copy
        .fleets
        .iter()
        .filter(|fleet| !fleet.is_alliance)
        .collect();
    assert_eq!(imperial.len(), 1);
    assert_eq!(imperial[0].serial, setup.world.fleet_serial(made));
    assert_eq!(imperial[0].name, setup.world.fleet_name(made).unwrap());
    assert!(imperial[0].ships.is_empty());
    // Once only.
    day(&mut setup, &missions);
    assert_eq!(
        alliance_details(&setup, chandrila)
            .unwrap()
            .fleets
            .iter()
            .filter(|fleet| !fleet.is_alliance)
            .count(),
        1
    );
}

#[test]
fn a_fleet_in_hyperspace_is_not_in_its_systems_full_copy() {
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
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
    let missions = crate::missions::MissionState::new();
    let details = SystemDetails::of(
        &setup.world,
        &setup.movement,
        &missions,
        &setup.transport,
        chandrila,
    );
    assert!(details.fleets.is_empty());
    assert_eq!(details.last_serial, setup.world.fleet_serial(moving));
}

/// Jabba's Palace (family 0x98) with Luke held in it. port: test data.
fn palace_with_captive(setup: &mut Setup) -> (crate::ids::AbodeKey, crate::ids::CharacterKey) {
    let palace = setup.world.abodes.insert(crate::world::Abode {
        dat_id: DatId::new(0x9800_0003),
        name: "Jabba's Palace".into(),
        side: Faction::Empire,
        special_forces: Vec::new(),
    });
    let luke = setup.world.characters.insert(crate::world::Character {
        dat_id: DatId::new(0x3000_0010),
        name: "Luke Skywalker".into(),
        is_alliance: true,
        is_captive: true,
        current_abode: Some(palace),
        ..Default::default()
    });
    (palace, luke)
}

#[test]
fn an_own_captive_keeps_the_copy_it_froze_with_in_the_palace_until_the_side_sees_it() {
    // A0 2026-11-18-edited-saves-7 b-19 (e4f53d49b065): on day 31 the
    // Alliance's Finder reads "Luke Skywalker - Jabba's Palace" though
    // Jabba delivered him to Vader on day 22; the day-34 save holds the
    // Alliance's copy of Luke among its copy of the palace's children.
    // Character_SetCaptured (FUN_004ee800) syncs the copies, then
    // FUN_004eec60 gives the captive its captor's side, so the Alliance's
    // copy is live only where the Alliance sees him (FUN_004f9a60).
    let mut setup = setup();
    let coruscant = setup.coruscant;
    let (_, luke) = palace_with_captive(&mut setup);
    let id = setup.world.characters[luke].dat_id;
    let in_palace = CharacterCopy {
        system: None,
        ship: Some("Jabba's Palace".into()),
    };
    let copy = |setup: &Setup| {
        setup
            .world
            .object_views
            .character(Faction::Alliance, id)
            .cloned()
    };
    let mut missions = crate::missions::MissionState::new();
    day(&mut setup, &missions);
    assert_eq!(copy(&setup), Some(in_palace.clone()));
    // The Pickup takes him to Coruscant, which the Alliance does not see.
    setup.world.characters[luke].current_abode = None;
    missions.push_escapee(crate::missions::EscapeeTransit {
        character: luke,
        to: coruscant,
        arrival: 2,
        abode: None,
        ship: None,
    });
    day(&mut setup, &missions);
    assert_eq!(copy(&setup), Some(in_palace.clone()));
    let missions = crate::missions::MissionState::new();
    setup.world.characters[luke].current_system = Some(coruscant);
    day(&mut setup, &missions);
    assert_eq!(copy(&setup), Some(in_palace));
    // An Alliance fleet at Coruscant sees him there.
    let watcher = fleet(&mut setup.world, coruscant, true);
    day(&mut setup, &missions);
    let at_coruscant = CharacterCopy {
        system: Some(setup.world.systems[coruscant].dat_id),
        ship: None,
    };
    assert_eq!(copy(&setup), Some(at_coruscant.clone()));
    // The fleet leaves and he is moved on: the copy stays at Coruscant.
    setup.world.fleets.remove(watcher);
    day(&mut setup, &missions);
    setup.world.characters[luke].current_system = Some(setup.chandrila);
    day(&mut setup, &missions);
    assert_eq!(copy(&setup), Some(at_coruscant));
    // Freed, he is the Alliance's own again and read live: no copy.
    setup.world.characters[luke].is_captive = false;
    day(&mut setup, &missions);
    assert_eq!(copy(&setup), None);
}

#[test]
fn an_own_captive_taken_out_of_sight_is_copied_where_it_stands_with_no_state() {
    // The copy is live up to the capture (the side owns the character), so
    // it holds the captive where it was taken (A0 personnel-captured/
    // finder-day628: "Jan Dodonna - Punisher ( Captured )"). A trip's
    // destination is its container from the start (FUN_00556390).
    use crate::character_command::CommandKind;
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let missions = crate::missions::MissionState::new();
    let dodonna = setup.world.characters.insert(crate::world::Character {
        dat_id: DatId::new(0x3000_0011),
        is_alliance: true,
        is_captive: true,
        current_system: Some(chandrila),
        command: CommandKind::General,
        ..Default::default()
    });
    let id = setup.world.characters[dodonna].dat_id;
    setup
        .world
        .object_views
        .alliance_character_commands
        .insert(id, CommandKind::General);
    day(&mut setup, &missions);
    let copy = |setup: &Setup| {
        setup
            .world
            .object_views
            .character(Faction::Alliance, id)
            .cloned()
    };
    assert_eq!(
        copy(&setup),
        Some(CharacterCopy {
            system: Some(setup.world.systems[chandrila].dat_id),
            ship: None,
        })
    );
    assert_eq!(
        setup
            .world
            .object_views
            .character_command(Faction::Alliance, id),
        CommandKind::None
    );
    // Seen while he travels on: his copy stays where it froze.
    let mut missions = crate::missions::MissionState::new();
    setup.world.characters[dodonna].current_system = None;
    missions.push_transit(crate::missions::MemberTransit {
        member: crate::missions::MissionMember::Character(dodonna),
        mission_id: 0,
        to: coruscant,
        arrival: 9,
        abode: None,
    });
    day(&mut setup, &missions);
    assert_eq!(
        copy(&setup).unwrap().system,
        Some(setup.world.systems[chandrila].dat_id)
    );
    // One the side sees leave takes its destination.
    let watcher = fleet(&mut setup.world, chandrila, true);
    setup.world.characters[dodonna].current_system = Some(chandrila);
    let none = crate::missions::MissionState::new();
    day(&mut setup, &none);
    setup.world.characters[dodonna].current_system = None;
    day(&mut setup, &missions);
    assert_eq!(
        copy(&setup).unwrap().system,
        Some(setup.world.systems[coruscant].dat_id)
    );
    // Seen again where he stands, the copy follows him there.
    setup.world.characters[dodonna].current_system = Some(chandrila);
    day(&mut setup, &none);
    assert_eq!(
        copy(&setup).unwrap().system,
        Some(setup.world.systems[chandrila].dat_id)
    );
    setup.world.fleets.remove(watcher);
}

#[test]
fn an_own_captive_in_hyperspace_or_on_a_trip_is_copied_only_by_its_own_trip() {
    // FUN_00556390 makes a trip's destination the traveller's container;
    // a fleet in hyperspace gives its passengers no place the port copies.
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let (_, luke) = palace_with_captive(&mut setup);
    let id = setup.world.characters[luke].dat_id;
    setup.world.characters[luke].current_abode = None;
    let moving = fleet(&mut setup.world, chandrila, false);
    setup.world.fleets[moving].characters.push(luke);
    setup.world.characters[luke].current_fleet = Some(moving);
    assert!(setup.movement.order(moving, chandrila, coruscant, 10));
    let mut missions = crate::missions::MissionState::new();
    day(&mut setup, &missions);
    assert_eq!(
        setup.world.object_views.character(Faction::Alliance, id),
        None
    );
    // On foot: another escapee's trip is not his.
    setup.world.characters[luke].current_fleet = None;
    let other = setup.world.characters.insert(crate::world::Character {
        dat_id: DatId::new(0x3000_0012),
        is_alliance: true,
        ..Default::default()
    });
    for (character, to) in [(other, chandrila), (luke, coruscant)] {
        missions.push_escapee(crate::missions::EscapeeTransit {
            character,
            to,
            arrival: 9,
            abode: None,
            ship: None,
        });
    }
    day(&mut setup, &missions);
    assert_eq!(
        setup.world.object_views.character(Faction::Alliance, id),
        Some(&CharacterCopy {
            system: Some(setup.world.systems[coruscant].dat_id),
            ship: None,
        })
    );
}

/// A Rim system (SYSTEMSD family 0x92) held by `control`. port: test data.
/// port: a test system with SYSTEMSD's family 0x92 id, in a Rim sector.
fn rim_system(world: &mut GameWorld, id: u32, name: &str, control: ControlKind) -> SystemKey {
    let key = system(world, id, name, control);
    let sector = world.sectors.insert(Sector {
        dat_id: DatId::new(0x9200_0000 | id),
        name: "Rim".into(),
        group: SectorGroup::RimOuter,
        x: 0,
        y: 0,
        systems: vec![key],
    });
    world.systems[key].dat_id = DatId::new(0x9200_0000 | id);
    world.systems[key].sector = sector;
    key
}

#[test]
fn a_new_game_gives_each_side_a_full_copy_of_every_core_system_and_a_bare_one_of_each_rim_system() {
    // FUN_0051b990 (ghidra/notes/fog-of-war.md): core systems (0x90..0x91)
    // take mode 1 with their facilities (0x20..0x2f, FUN_00539ac0), Rim
    // systems mode 2 only (FUN_0050c8a0); no setup range holds squadrons.
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
            damage: None,
        });
    let yard =
        setup
            .world
            .manufacturing_facilities
            .insert(crate::world::ManufacturingFacilityInstance {
                class_dat_id: DatId::new(0x2800_0001),
                side: Faction::Empire,
                is_shipyard: true,
            });
    setup.world.systems[chandrila]
        .manufacturing_facilities
        .push(yard);
    let empire = ControlKind::Controlled(Faction::Empire);
    let rim = rim_system(&mut setup.world, 3, "Kessel", empire);

    super::setup(&mut setup.world);

    let views = &setup.world.object_views;
    let core = views
        .system(Faction::Alliance, setup.world.systems[chandrila].dat_id)
        .unwrap();
    assert!(!core.live);
    assert_eq!(core.control, empire);
    assert_eq!(core.manufacturing_facilities.len(), 1);
    assert!(core.fighters.is_empty());
    assert!(side_explored(&setup.world, Faction::Alliance, chandrila));
    let bare = views
        .system(Faction::Alliance, setup.world.systems[rim].dat_id)
        .unwrap();
    assert_eq!(bare, &SystemView::bare());
    assert!(!side_explored(&setup.world, Faction::Alliance, rim));
    // The Empire holds Kessel: its own system is explored to it.
    assert!(side_explored(&setup.world, Faction::Empire, rim));
}

#[test]
fn the_empire_starts_with_a_push_of_yavin() {
    // FUN_0051b990 at 0x51bba6: Event319_Handle(10, 2) on 0x92000121.
    let mut setup = setup();
    let alliance = ControlKind::Controlled(Faction::Alliance);
    let yavin = rim_system(&mut setup.world, 0x121, "Yavin", alliance);
    super::setup(&mut setup.world);
    assert_eq!(setup.world.object_views.pushes, [(Faction::Empire, yavin)]);
    step(&mut setup);
    let id = setup.world.systems[yavin].dat_id;
    let copy = setup
        .world
        .object_views
        .system(Faction::Empire, id)
        .unwrap();
    assert_eq!(copy.control, alliance);
    assert!(side_explored(&setup.world, Faction::Empire, yavin));
}

#[test]
fn a_rim_system_stays_explored_with_its_last_seen_side_after_the_fleet_leaves() {
    // FUN_0050ac80 makes the copy live while the fleet stands there
    // (FUN_004f9a60 rebuilds it, explored included); it freezes with the
    // values it had when the fleet left (ghidra/notes/fog-of-war.md).
    let mut setup = setup();
    let rim = rim_system(&mut setup.world, 3, "Kessel", ControlKind::Uncontrolled);
    super::setup(&mut setup.world);
    assert!(!side_explored(&setup.world, Faction::Alliance, rim));
    let scout = fleet(&mut setup.world, rim, true);
    step(&mut setup);
    assert!(side_explored(&setup.world, Faction::Alliance, rim));
    setup.world.fleets.remove(scout);
    setup.world.systems[rim].fleets.clear();
    step(&mut setup);
    setup.world.systems[rim].control = ControlKind::Controlled(Faction::Empire);
    step(&mut setup);
    let id = setup.world.systems[rim].dat_id;
    let copy = setup
        .world
        .object_views
        .system(Faction::Alliance, id)
        .unwrap();
    assert!(!copy.live);
    assert_eq!(copy.control, ControlKind::Uncontrolled);
    assert!(side_explored(&setup.world, Faction::Alliance, rim));
}

#[test]
fn a_push_explores_the_system_for_its_side_only() {
    // Event319_Handle syncs explored into the pushed side's copy
    // (System_SyncExplored FUN_0050fc80); the other side's copy is
    // untouched (ghidra/notes/fog-of-war.md).
    let mut setup = setup();
    let rim = rim_system(&mut setup.world, 3, "Kessel", ControlKind::Uncontrolled);
    super::setup(&mut setup.world);
    setup.world.object_views.queue_push(Faction::Alliance, rim);
    step(&mut setup);
    assert!(side_explored(&setup.world, Faction::Alliance, rim));
    assert!(!side_explored(&setup.world, Faction::Empire, rim));
}

#[test]
fn a_copy_taken_of_a_destroyed_system_holds_its_destroyed_bit() {
    // FUN_0045bbb0 reads +0x50 bit 3 of the viewer's copy: the side that
    // sees the system destroyed holds the bit; the other keeps its setup
    // copy (ghidra/notes/fog-of-war.md).
    let mut setup = setup();
    let rim = rim_system(&mut setup.world, 3, "Alderaan", ControlKind::Uncontrolled);
    super::setup(&mut setup.world);
    fleet(&mut setup.world, rim, true);
    setup.world.systems[rim].is_destroyed = true;
    step(&mut setup);
    assert!(side_destroyed(&setup.world, Faction::Alliance, rim));
    assert!(!side_destroyed(&setup.world, Faction::Empire, rim));
    // A side's own system shows itself, whatever its old copy held.
    setup.world.systems[rim].control = ControlKind::Controlled(Faction::Empire);
    assert!(side_destroyed(&setup.world, Faction::Empire, rim));
    // With no bits, a system shows its own.
    let mut bare = self::setup();
    bare.world.systems[bare.chandrila].is_destroyed = true;
    assert!(side_destroyed(
        &bare.world,
        Faction::Alliance,
        bare.chandrila
    ));
    assert!(!side_destroyed(
        &bare.world,
        Faction::Alliance,
        bare.coruscant
    ));
}

#[test]
fn explored_without_bits_follows_the_system() {
    // port: a world with no setup copies (a test, a fixture) and a v126
    // save's copies, which carry no bits, read the system's own
    // exploration, as the port showed it before the copies held the bit
    // (ghidra/notes/fog-of-war.md).
    let mut setup = setup();
    let rim = rim_system(&mut setup.world, 3, "Kessel", ControlKind::Uncontrolled);
    setup.world.systems[rim].exploration_status = ExplorationStatus::Unexplored;
    assert!(!side_explored(&setup.world, Faction::Alliance, rim));
    assert!(side_explored(
        &setup.world,
        Faction::Alliance,
        setup.chandrila
    ));
    let id = setup.world.systems[rim].dat_id;
    setup
        .world
        .object_views
        .alliance_systems
        .insert(id, SystemView::bare());
    assert!(!side_explored(&setup.world, Faction::Alliance, rim));
    setup.world.systems[rim].exploration_status = ExplorationStatus::Explored;
    assert!(side_explored(&setup.world, Faction::Alliance, rim));
    setup.world.systems[rim].exploration_status = ExplorationStatus::Unexplored;
    // The debug mode reads the system's own bit; a side's own system is
    // explored to it.
    setup.world.object_views.see_everything = true;
    assert!(!side_explored(&setup.world, Faction::Alliance, rim));
    setup.world.object_views.see_everything = false;
    setup.world.systems[rim].control = ControlKind::Controlled(Faction::Empire);
    assert!(side_explored(&setup.world, Faction::Empire, rim));
}

#[test]
fn a_seeded_copy_is_explored_as_the_system_is_unless_the_side_stands_there() {
    // port: a copy seeded from the fog (an older save, a fixture) takes the
    // system's own explored bit, or the explored bit a live copy takes when
    // the side's fleet stands there (FUN_004f9a60, FUN_0050fc80;
    // ghidra/notes/fog-of-war.md).
    let mut setup = setup();
    let far = rim_system(&mut setup.world, 3, "Ithor", ControlKind::Uncontrolled);
    let held = rim_system(&mut setup.world, 4, "Kessel", ControlKind::Uncontrolled);
    for key in [far, held] {
        setup.world.systems[key].exploration_status = ExplorationStatus::Unexplored;
    }
    fleet(&mut setup.world, held, true);
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(far);
    fog.reveal(held);
    seed_systems_from_fog(
        &mut setup.world,
        &setup.movement,
        [&fog, &FogState::new(Faction::Empire)],
    );
    assert!(!side_explored(&setup.world, Faction::Alliance, far));
    assert!(side_explored(&setup.world, Faction::Alliance, held));
    // A copy that freezes keeps its bit; a day's step does not explore it.
    step(&mut setup);
    assert!(!side_explored(&setup.world, Faction::Alliance, far));
    assert!(side_explored(&setup.world, Faction::Alliance, held));
}

#[test]
fn a_new_game_finds_a_loaded_galaxys_core_systems_and_yavin() {
    // The loader keeps SYSTEMSD's sequence id alone (Yavin 0x121) and its
    // family only as the explored bit; every family-0x90 record lies in a
    // Core sector (SECTORSD group 1). FUN_0051b990 copies the core systems
    // in full (FUN_00566c00) and pushes Yavin (0x92000121) to the Empire.
    let mut world = GameWorld::default();
    let place = |world: &mut GameWorld, id: u32, name: &str, group, control| {
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(id),
            name: name.into(),
            group,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let key = system(world, id, name, control);
        let value = &mut world.systems[key];
        value.dat_id = DatId::new(id);
        value.sector = sector;
        value.exploration_status = if group == SectorGroup::Core {
            ExplorationStatus::Explored
        } else {
            ExplorationStatus::Unexplored
        };
        world.sectors[sector].systems.push(key);
        key
    };
    let alliance = ControlKind::Controlled(Faction::Alliance);
    let corfai = place(&mut world, 0x0f, "Corfai", SectorGroup::Core, alliance);
    let yavin = place(&mut world, 0x121, "Yavin", SectorGroup::RimOuter, alliance);
    let kabal = place(
        &mut world,
        0x40,
        "Kabal",
        SectorGroup::RimInner,
        ControlKind::Uncontrolled,
    );
    super::setup(&mut world);
    assert_eq!(world.object_views.pushes, [(Faction::Empire, yavin)]);
    advance(
        &mut world,
        &TroopTransportState::new(),
        &MovementState::new(),
    );
    let copy = |key: SystemKey| {
        world
            .object_views
            .system(Faction::Empire, world.systems[key].dat_id)
            .unwrap()
            .clone()
    };
    assert_eq!(copy(corfai).control, alliance);
    assert!(side_explored(&world, Faction::Empire, corfai));
    assert_eq!(copy(yavin).control, alliance);
    assert!(side_explored(&world, Faction::Empire, yavin));
    assert_eq!(copy(kabal), SystemView::bare());
    assert!(!side_explored(&world, Faction::Empire, kabal));
}

fn revolt(uprisings: &mut crate::uprising::UprisingState, at: SystemKey) {
    uprisings.active_uprisings.insert(
        at,
        crate::uprising::ActiveUprising {
            started_tick: 1,
            next_incident_tick: None,
            next_support_tick: None,
            side: None,
        },
    );
}

// The mode-1 sync (FUN_0050e5b0 through CoreSystem_vt24 FUN_00566d00 ->
// FUN_00566f80) copies the uprising bit (+0x88 bit 2) with the other fields;
// a frozen copy (FUN_005404d0) keeps it, and the sector window's planet
// (FUN_0045bbb0, marks 905/906) reads the viewer's copy
// (ghidra/notes/fog-of-war.md).
#[test]
fn a_frozen_copy_keeps_the_uprising_it_was_last_synced_with() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let mut uprisings = crate::uprising::UprisingState::default();
    revolt(&mut uprisings, chandrila);
    let watcher = fleet(&mut setup.world, chandrila, true);
    let day = |setup: &mut Setup, uprisings: &crate::uprising::UprisingState| {
        advance_with_uprisings(&mut setup.world, &setup.transport, &setup.movement, uprisings);
    };
    day(&mut setup, &uprisings);
    // Live: the system's own bit.
    assert!(side_uprising(&setup.world, Faction::Alliance, chandrila, &uprisings));
    setup.world.fleets.remove(watcher);
    day(&mut setup, &uprisings);
    // The revolt ends out of sight: the frozen copy still shows it.
    uprisings.active_uprisings.clear();
    day(&mut setup, &uprisings);
    assert!(side_uprising(&setup.world, Faction::Alliance, chandrila, &uprisings));
    // The holder sees its own system calm.
    assert!(!side_uprising(&setup.world, Faction::Empire, chandrila, &uprisings));
    // A push syncs the copy again.
    setup.world.object_views.queue_push(Faction::Alliance, chandrila);
    day(&mut setup, &uprisings);
    assert!(!side_uprising(&setup.world, Faction::Alliance, chandrila, &uprisings));
    // A revolt that starts out of sight stays unseen.
    revolt(&mut uprisings, chandrila);
    day(&mut setup, &uprisings);
    assert!(!side_uprising(&setup.world, Faction::Alliance, chandrila, &uprisings));
}

#[test]
fn seeding_uprisings_gives_each_copy_its_systems_bit() {
    let mut setup = setup();
    let (chandrila, coruscant) = (setup.chandrila, setup.coruscant);
    let mut uprisings = crate::uprising::UprisingState::default();
    revolt(&mut uprisings, chandrila);
    super::setup(&mut setup.world);
    seed_uprisings(&mut setup.world, &uprisings);
    assert!(side_uprising(&setup.world, Faction::Alliance, chandrila, &uprisings));
    assert!(!side_uprising(&setup.world, Faction::Alliance, coruscant, &uprisings));
}

// A live copy (FUN_004f9a60) shows the system as it is: a revolt that
// starts in sight shows at once, even before the copy's next sync.
#[test]
fn a_live_copy_shows_a_revolt_that_starts_in_sight() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let mut uprisings = crate::uprising::UprisingState::default();
    fleet(&mut setup.world, chandrila, true);
    advance_with_uprisings(&mut setup.world, &setup.transport, &setup.movement, &uprisings);
    advance_with_uprisings(&mut setup.world, &setup.transport, &setup.movement, &uprisings);
    revolt(&mut uprisings, chandrila);
    assert!(side_uprising(&setup.world, Faction::Alliance, chandrila, &uprisings));
}

// port: the debug "see everything" mode shows each system's own bit.
#[test]
fn seeing_everything_shows_the_systems_own_uprising() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let mut uprisings = crate::uprising::UprisingState::default();
    super::setup(&mut setup.world);
    revolt(&mut uprisings, chandrila);
    assert!(!side_uprising(&setup.world, Faction::Alliance, chandrila, &uprisings));
    setup.world.object_views.see_everything = true;
    assert!(side_uprising(&setup.world, Faction::Alliance, chandrila, &uprisings));
}

// FUN_0051b990: a new campaign's start gives each side its setup copies.
#[test]
fn a_campaign_start_gives_each_side_a_copy_of_the_other_sides_core_systems() {
    let mut setup = setup();
    let chandrila = setup.chandrila;
    let id = setup.world.systems[chandrila].dat_id;
    start(
        &mut setup.world,
        &setup.movement,
        &crate::missions::MissionState::new(),
        &setup.transport,
        &crate::uprising::UprisingState::default(),
    );
    let copy = setup.world.object_views.system(Faction::Alliance, id).unwrap();
    assert!(!copy.live);
}
