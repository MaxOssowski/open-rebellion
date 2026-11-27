use super::*;
use rebellion_core::dat::ExplorationStatus;
use rebellion_core::manufacturing::{BuildableKind, QueueItem};
use rebellion_core::missions::{MissionKind, MissionRequest};
use rebellion_core::world::{
    BuildableClass, ClassStats, ControlKind, DefenseFacilityInstance, FighterClass,
    FighterEntry, Fleet, ManufacturingFacilityInstance, SpecialForceUnit, System, TroopUnit,
};

#[test]
fn a_ship_or_squadron_target_is_named_by_its_class() {
    let mut world = GameWorld::default();
    let class = world.capital_ship_classes.insert(CapitalShipClass {
        name: "Nebulon-B Frigate".into(),
        ..CapitalShipClass::default()
    });
    let fighter = world.fighter_classes.insert(FighterClass {
        name: "X-wing".into(),
        ..FighterClass::default()
    });
    let fleet = FleetKey::default();
    assert_eq!(
        object_name(&world, MissionTarget::CapitalShip { fleet, class }),
        Some("Nebulon-B Frigate".into())
    );
    assert_eq!(
        object_name(
            &world,
            MissionTarget::Fighter {
                fleet,
                class: fighter
            }
        ),
        Some("X-wing".into())
    );
    world.capital_ship_classes[class].name.clear();
    assert_eq!(
        object_name(&world, MissionTarget::CapitalShip { fleet, class }),
        None
    );
}

/// The states a Status window reads, all idle.
struct States {
    missions: MissionState,
    movement: MovementState,
    transport: TroopTransportState,
    manufacturing: ManufacturingState,
}

impl States {
    fn new() -> Self {
        Self {
            missions: MissionState::new(),
            movement: MovementState::new(),
            transport: TroopTransportState::new(),
            manufacturing: ManufacturingState::new(),
        }
    }

    fn sources<'a>(&'a self, world: &'a GameWorld) -> StatusSources<'a> {
        StatusSources {
            world,
            missions: &self.missions,
            movement: &self.movement,
            transport: &self.transport,
            manufacturing: &self.manufacturing,
            today: 7,
            objectives: ObjectiveSource::default(),
        }
    }
}

/// `state` with `entry` appended to its private list `field`, through
/// its serde form.
fn with_entry<T: serde::Serialize + serde::de::DeserializeOwned>(
    state: &T,
    field: &str,
    entry: impl serde::Serialize,
) -> T {
    let mut value = serde_json::to_value(state).unwrap();
    value[field]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(entry).unwrap());
    serde_json::from_value(value).unwrap()
}

fn system(world: &mut GameWorld, name: &str, control: ControlKind) -> SystemKey {
    world.systems.insert(System {
        dat_id: DatId::new(0x9000_0001),
        name: name.into(),
        exploration_status: ExplorationStatus::Explored,
        popularity_alliance: 0.5,
        popularity_empire: 0.5,
        is_populated: true,
        control,
        ..Default::default()
    })
}

fn catalog(world: &mut GameWorld, id: u32, name: &str, class: BuildableClass) -> DatId {
    let dat_id = DatId::new(id);
    world.buildable_classes.insert(
        dat_id,
        BuildableClass {
            name: name.into(),
            ..class
        },
    );
    dat_id
}

fn rows(rows: &[StatusRow]) -> Vec<(&str, &str)> {
    rows.iter()
        .map(|row| (row.label.as_str(), row.value.as_str()))
        .collect()
}

/// An Alliance fleet at `at` of two carriers (2 and 3 squadrons, 1
/// regiment each), the second damaged, with 4 squadrons and one
/// character aboard.
fn carrier_fleet(world: &mut GameWorld, at: SystemKey) -> FleetKey {
    let carrier = |name: &str, squadrons, hyperdrive| CapitalShipClass {
        dat_id: DatId::new(0x1400_0040),
        name: name.into(),
        is_alliance: true,
        fighter_capacity: squadrons,
        troop_capacity: 1,
        hull: 100,
        hyperdrive,
        maintenance_cost: 6,
        turbolaser_starboard: 3,
        turbolaser_port: 4,
        ..CapitalShipClass::default()
    };
    let small = world.capital_ship_classes.insert(CapitalShipClass {
        hyperdrive_if_damaged: 9,
        ..carrier("Small", 2, 0)
    });
    let large = world.capital_ship_classes.insert(carrier("Large", 3, 2));
    let fighter = world.fighter_classes.insert(FighterClass {
        dat_id: DatId::new(0x1c00_0001),
        name: "X-wing".into(),
        squadron_size: 12,
        shield_strength: 5,
        maintenance_cost: 2,
        ..FighterClass::default()
    });
    let mut damaged = ShipInstance::new(large, 100, true);
    damaged.hull_current = 60;
    let character = world.characters.insert(rebellion_core::world::Character::default());
    let fleet = world.insert_fleet(Fleet {
        location: at,
        capital_ships: vec![ShipInstance::new(small, 100, true), damaged],
        fighters: vec![FighterEntry {
            class: fighter,
            count: 4,
            carrier: None,
            damage: None,
        }],
        characters: vec![character],
        is_alliance: true,
        ..Default::default()
    });
    world.systems[at].fleets.push(fleet);
    fleet
}

#[test]
fn fleet_status_sums_its_ships_and_reads_no_commander() {
    // FUN_00449200: Status, the three command ranks, ship count, the
    // capacity and embarked blocks, personnel, damage and hyperdrive.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    let states = States::new();

    let rows = fleet_rows(states.sources(&world), fleet).unwrap();

    assert_eq!(
        super::tests::rows(&rows),
        vec![
            ("Status:", "Awaiting Orders"),
            ("Admiral:", "Not Assigned"),
            ("General:", "Not Assigned"),
            ("Commander:", "Not Assigned"),
            ("Number Of Ships", "2"),
            ("Capacity:", ""),
            ("Fighter Squadrons:", "5"),
            ("Trooper Regiments:", "2"),
            ("Embarked:", ""),
            ("Fighter Squadrons:", "4"),
            ("Trooper Regiments:", "0"),
            ("Personnel:", "1"),
            ("Damaged Ships:", "1"),
            ("Hyperdrive Rating:", "Yes"),
        ]
    );
}

#[test]
fn fleet_status_names_the_fleets_side_s_admiral_general_and_commander() {
    // FUN_00449200: 12358 "Admiral", 12360 "General", 12356 "Commander",
    // each FUN_004fd790(fleet, kind): the first of the fleet's side
    // aboard whose +0x96 is the kind, else 34658 "Not Assigned".
    use rebellion_core::character_command::CommandKind;
    let mut world = GameWorld::default();
    let here = system(
        &mut world,
        "Yavin",
        ControlKind::Controlled(Faction::Alliance),
    );
    let fleet = carrier_fleet(&mut world, here);
    let aboard = |world: &mut GameWorld, name: &str, alliance: bool, command| {
        world.characters.insert(rebellion_core::world::Character {
            name: name.into(),
            is_alliance: alliance,
            current_fleet: Some(fleet),
            command,
            ..Default::default()
        })
    };
    aboard(&mut world, "Ackbar", true, CommandKind::Admiral);
    aboard(&mut world, "Rieekan", true, CommandKind::None);
    // A prisoner of the other side aboard counts for neither.
    aboard(&mut world, "Ozzel", false, CommandKind::General);
    let states = States::new();
    let rows = fleet_rows(states.sources(&world), fleet).unwrap();
    let rows = super::tests::rows(&rows);
    assert_eq!(
        rows[1..4],
        [
            ("Admiral:", "Ackbar"),
            ("General:", "Not Assigned"),
            ("Commander:", "Not Assigned")
        ]
    );
}

#[test]
fn a_picture_carries_the_en_route_and_damage_marks() {
    // FUN_0042c3b0(.., 1, 1): a craft's GOKRES portrait + 0x1000 en
    // route, + 0x2000 damaged; a fleet's 10426 en route; a regiment's
    // 11514 backdrop.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let there = system(&mut world, "Hoth", ControlKind::Uncontrolled);
    let fleet = carrier_fleet(&mut world, here);
    let mut states = States::new();
    let marks = |states: &States, world: &GameWorld, object| {
        status_view(states.sources(world), object).unwrap().marks
    };
    let ship = |index| StatusObject::Ship { fleet, index };
    // At rest: the damaged second ship (hull 60 of 100) has its mark,
    // and so has its fleet (FUN_004feab0: any damaged ship).
    assert_eq!(
        marks(&states, &world, StatusObject::Fleet(fleet)),
        [(DllSource::Strategy, 10_427)]
    );
    assert_eq!(marks(&states, &world, ship(0)), []);
    let class = world.capital_ship_classes[world.fleets[fleet].capital_ships[1].class].dat_id;
    let portrait = capital_ship_mini_id(class).unwrap() - 0x4000;
    assert_eq!(
        marks(&states, &world, ship(1)),
        [(DllSource::Gokres, portrait + 0x2000)]
    );
    assert!(states.movement.order(fleet, here, there, 5));
    assert_eq!(
        marks(&states, &world, StatusObject::Fleet(fleet)),
        [(DllSource::Strategy, 10_426), (DllSource::Strategy, 10_427)]
    );
    assert_eq!(
        marks(&states, &world, ship(1)),
        [
            (DllSource::Gokres, portrait + 0x1000),
            (DllSource::Gokres, portrait + 0x2000)
        ]
    );
    let squadron = StatusObject::Fighter { fleet, index: 0 };
    let fighter = fighter_mini_id(DatId::new(0x1c00_0001)).unwrap() - 0x4000;
    assert_eq!(
        marks(&states, &world, squadron),
        [(DllSource::Gokres, fighter + 0x1000)]
    );
}

#[test]
fn a_regiment_and_a_special_force_take_their_own_marks() {
    // FUN_0042c3b0: 11514 behind every regiment; en route 11516, but
    // the GOKRES mark for classes 0x10000002 and 0x10000008; a special
    // force's 11520, but GOKRES for 0x3c000003 and 0x3c000005.
    let portrait = Some((DllSource::Gokres, 100));
    let infantry = DatId::new(0x1000_0001);
    let special = DatId::new(0x1000_0002);
    assert_eq!(
        regiment_marks(portrait, infantry, false),
        [(DllSource::Strategy, 11_514)]
    );
    assert_eq!(
        regiment_marks(portrait, infantry, true),
        [(DllSource::Strategy, 11_514), (DllSource::Strategy, 11_516)]
    );
    assert_eq!(
        regiment_marks(portrait, special, true),
        [(DllSource::Strategy, 11_514), (DllSource::Gokres, 0x1064)]
    );
    assert_eq!(
        regiment_marks(portrait, DatId::new(0x1000_0008), true)[1],
        (DllSource::Gokres, 0x1064)
    );
    let force = DatId::new(0x3c00_0001);
    assert_eq!(special_force_marks(portrait, force, false), []);
    assert_eq!(
        special_force_marks(portrait, force, true),
        [(DllSource::Strategy, 11_520)]
    );
    for class in [0x3c00_0003, 0x3c00_0005] {
        assert_eq!(
            special_force_marks(portrait, DatId::new(class), true),
            [(DllSource::Gokres, 0x1064)]
        );
    }
    assert_eq!(craft_marks(None, true, true), []);
}

#[test]
fn a_moving_fleet_is_enroute_with_its_arrival_day() {
    // FUN_00449200 34628 "Enroute"; FUN_004fd2b0 "ETA Destination:".
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let there = system(&mut world, "Hoth", ControlKind::Uncontrolled);
    let fleet = carrier_fleet(&mut world, here);
    let mut states = States::new();
    assert!(states.movement.order(fleet, here, there, 5));

    let rows = fleet_rows(states.sources(&world), fleet).unwrap();

    assert_eq!(
        super::tests::rows(&rows[..2]),
        vec![("Status:", "Enroute"), ("ETA Destination:", "Day 12")]
    );
}

#[test]
fn each_ship_carries_the_fleets_squadrons_in_order_up_to_its_capacity() {
    // FUN_00446fd0 reads each ship's own embarked counts (+0x23c);
    // port: the fleet's 4 squadrons fill the first ship's 2 bays first.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    let states = States::new();
    let sources = states.sources(&world);
    let value = |index: usize, label: &str, nth: usize| -> String {
        ship_rows(sources, fleet, index)
            .unwrap()
            .into_iter()
            .filter(|row| row.label == label)
            .nth(nth)
            .unwrap()
            .value
    };

    assert_eq!(value(0, "Fighter Squadrons:", 1), "2");
    assert_eq!(value(1, "Fighter Squadrons:", 1), "2");
    assert_eq!(value(0, "Personnel:", 0), "1");
    assert_eq!(value(1, "Personnel:", 0), "0");
    assert_eq!(value(0, "Ship Damaged:", 0), "No");
    assert_eq!(value(1, "Ship Damaged:", 0), "Yes");
    assert_eq!(value(1, "Hull Value:", 0), "60:100");
    // A class with no hyperdrive shows its damaged rating (FUN_00446fd0).
    assert_eq!(value(0, "Hyperdrive Rating:", 0), "9:0");
    assert_eq!(value(1, "Hyperdrive Rating:", 0), "2:2");
}

#[test]
fn units_on_their_way_aboard_count_as_embarked_on_the_ship_that_seats_them() {
    // FUN_00446fd0 prints Embarked from the ship's slots +0x23c
    // (CapShip_vt23c FUN_00500fe0) and +0x240 (CapShip_vt240
    // FUN_00501040) and Personnel from FUN_004f25c0(ship, 3)'s
    // characters (0x30..0x3c); FUN_00449200 sums the same per ship.
    // All walk mode 3, and FUN_00556390 makes a unit moving onto a
    // fleet the child of its ship at once, so the travellers count
    // where fleet_window::ship_load seats them (FUN_00552300).
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let there = system(&mut world, "Bespin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, there);
    for (index, ship) in world.fleets[fleet].capital_ships.iter_mut().enumerate() {
        ship.id = 1 + index as u64;
    }
    let xwing = world.fleets[fleet].fighters[0].class;
    let troop = world.troops.insert(TroopUnit {
        class_dat_id: DatId::new(0x1000_0002),
        is_alliance: true,
        regiment_strength: 100,
    });
    let character = world.characters.insert(rebellion_core::world::Character {
        is_alliance: true,
        ..Default::default()
    });
    let mut states = States::new();
    states.transport = with_entry(
        &states.transport,
        "transit",
        rebellion_core::troop_transport::RegimentTransit {
            troop,
            origin: here,
            leg: RegimentLeg::Fleet(fleet),
            arrival_tick: 20,
        },
    );
    let mut value = serde_json::to_value(&states.transport).unwrap();
    value["squadrons"]["transits"].as_array_mut().unwrap().push(
        serde_json::to_value(rebellion_core::squadrons::SquadronTransit {
            class: xwing,
            count: 1,
            origin: here,
            to: rebellion_core::squadrons::SquadronPlace::Fleet(fleet),
            arrival_tick: 20,
            lost: Vec::new(),
        })
        .unwrap(),
    );
    states.transport = serde_json::from_value(value).unwrap();
    let id = states.missions.dispatch(MissionRequest::single(
        MissionKind::Diplomacy,
        MissionFaction::Alliance,
        character,
        there,
        None,
        0,
    ));
    states.missions.set_target_fleet(id, fleet);
    states.missions.push_transit(rebellion_core::missions::MemberTransit {
        member: MissionMember::Character(character),
        mission_id: id,
        to: there,
        arrival: 15,
        abode: None,
    });
    let sources = states.sources(&world);
    let embarked = |rows: Vec<StatusRow>| -> Vec<String> {
        let start = rows.iter().position(|row| row.label == "Embarked:").unwrap();
        rows[start + 1..start + 4].iter().map(|row| row.value.clone()).collect()
    };

    // Bays 2 and 3 hold the fleet's 4 squadrons 2 and 2: the squadron
    // on its way takes the second ship (1 left against 0). The
    // regiment takes the first of equals (1 and 1), the character the
    // first ship.
    assert_eq!(embarked(ship_rows(sources, fleet, 0).unwrap()), ["2", "1", "2"]);
    assert_eq!(embarked(ship_rows(sources, fleet, 1).unwrap()), ["3", "0", "0"]);
    assert_eq!(embarked(fleet_rows(sources, fleet).unwrap()), ["5", "1", "2"]);
    // port: an unnumbered ship still carries the fleet's characters
    // and those on their way in on its first alive ship.
    for ship in &mut world.fleets[fleet].capital_ships {
        ship.id = 0;
    }
    let sources = states.sources(&world);
    assert_eq!(embarked(ship_rows(sources, fleet, 0).unwrap()), ["2", "1", "2"]);
}

#[test]
fn fleet_status_personnel_counts_special_forces_and_capital_ship_status_does_not() {
    // FUN_00449200 sums each ship's FUN_00536da0(ship, 3) walk, families
    // 0x30..0x40; FUN_00446fd0 prints FUN_004f25c0(ship, 3)'s,
    // 0x30..0x3c. FUN_005131d0 keeps lo <= family < hi, so the special
    // forces (0x3c..0x3f) count in the Fleet Status only, aboard and,
    // by mode 3, on their way in (FUN_00556390).
    let mut world = GameWorld::default();
    let there = system(&mut world, "Bespin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, there);
    for (index, ship) in world.fleets[fleet].capital_ships.iter_mut().enumerate() {
        ship.id = 1 + index as u64;
    }
    let force = |world: &mut GameWorld| {
        world.special_forces.insert(rebellion_core::world::SpecialForceUnit {
            class_dat_id: DatId::new(0x3c00_0002),
            is_alliance: true,
            skills: [0; 8],
            on_mission: false,
            on_hidden_mission: false,
        })
    };
    let aboard = force(&mut world);
    world.fleets[fleet].special_forces.push(aboard);
    let travelling = force(&mut world);
    let mut states = States::new();
    let id = states.missions.dispatch(MissionRequest::single(
        MissionKind::Diplomacy,
        MissionFaction::Alliance,
        world.fleets[fleet].characters[0],
        there,
        None,
        0,
    ));
    states.missions.set_target_fleet(id, fleet);
    states.missions.push_transit(rebellion_core::missions::MemberTransit {
        member: MissionMember::SpecialForce(travelling),
        mission_id: id,
        to: there,
        arrival: 15,
        abode: None,
    });
    let sources = states.sources(&world);
    let personnel = |rows: Vec<StatusRow>| -> String {
        let start = rows.iter().position(|row| row.label == "Embarked:").unwrap();
        rows[start + 3].value.clone()
    };

    // The fleet's one character and its two special forces.
    assert_eq!(personnel(fleet_rows(sources, fleet).unwrap()), "3");
    // The ship that seats them all counts the character alone.
    assert_eq!(personnel(ship_rows(sources, fleet, 0).unwrap()), "1");
    assert_eq!(personnel(ship_rows(sources, fleet, 1).unwrap()), "0");
}

#[test]
fn a_ships_hits_and_hull_damage_lower_its_ratings() {
    // FUN_00446fd0's rows less FUN_00500fa0, FUN_00500e60, FUN_005010c0,
    // FUN_00500f60, FUN_00500f00 and FUN_00501270 (GNPRTB 2048 = 4),
    // and a hit primary hyperdrive (FUN_005011f0).
    use rebellion_core::world::{GnprtbEntry, GnprtbParams, Subsystem};
    let mut world = GameWorld::default();
    world.gnprtb = GnprtbParams::new(vec![GnprtbEntry {
        parameter_id: 2048,
        development: 4,
        alliance_sp_easy: 4,
        alliance_sp_medium: 4,
        alliance_sp_hard: 4,
        empire_sp_easy: 4,
        empire_sp_medium: 4,
        empire_sp_hard: 4,
        multiplayer: 4,
    }]);
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    let large = world.fleets[fleet].capital_ships[1].class;
    let class = &mut world.capital_ship_classes[large];
    class.hyperdrive_if_damaged = 5;
    class.shield_recharge_rate = 8;
    class.weapon_recharge_rate = 12;
    class.tractor_beam_power = 10;
    class.sub_light_engine = 20;
    class.bombardment_modifier = 10;
    let ship = &mut world.fleets[fleet].capital_ships[1];
    for (subsystem, hits) in [
        (Subsystem::Shield, 1),
        (Subsystem::Weapon, 2),
        (Subsystem::Tractor, 1),
        (Subsystem::PrimaryHyperdrive, 1),
    ] {
        ship.set_hits(subsystem, hits);
    }
    let states = States::new();
    let rows = ship_rows(states.sources(&world), fleet, 1).unwrap();
    let value = |label: &str| {
        super::tests::rows(&rows)
            .into_iter()
            .find(|(each, _)| *each == label)
            .unwrap()
            .1
            .to_owned()
    };
    // The hull is 60 of 100.
    assert_eq!(value("Shield Recharge Rate:"), "6:8");
    assert_eq!(value("Weapon Recharge Rate:"), "6:12");
    // 10 x 3 / 4 = 7; 7 x 60 / 100 = 4.
    assert_eq!(value("Tractor Beam Power:"), "4:10");
    // 20 x 4 / 4 x 60 / 100 = 12.
    assert_eq!(value("Sub-Light Engine Rating:"), "12:20");
    // 10 - 10 x 40 / 100.
    assert_eq!(value("Bombardment Modifier:"), "6");
    assert_eq!(value("Hyperdrive Rating:"), "5:2");
    // The starboard turbolaser: 3 x 2 / 4 x 60 / 100 = 0.
    let turbo: Vec<String> = super::tests::rows(&rows)
        .into_iter()
        .filter(|(label, _)| *label == "Turbo Laser:")
        .map(|(_, value)| value.to_owned())
        .collect();
    assert_eq!(turbo, ["0", "0", "0", "1"]);
    // Hits alone count as damage (FUN_00501c00).
    let ship = &mut world.fleets[fleet].capital_ships[0];
    ship.set_hits(Subsystem::Speed, 1);
    let rows = ship_rows(states.sources(&world), fleet, 0).unwrap();
    assert!(super::tests::rows(&rows).contains(&("Ship Damaged:", "Yes")));
}

#[test]
fn a_ship_on_its_way_to_join_a_fleet_names_that_fleet() {
    // FUN_004feca0 reparents a ship moved onto a fleet into it at once
    // (manual p. 122: "immediately be considered a member of the
    // fleet"), so its Status names the target under Fleet while it is
    // still Enroute.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let there = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let target = carrier_fleet(&mut world, here);
    let mover = carrier_fleet(&mut world, there);
    world.systems[there].fleets.retain(|&fleet| fleet != mover);
    let mut states = States::new();
    assert!(states.movement.order(mover, there, here, 5));
    states.movement.set_join(mover, target);

    let rows = ship_rows(states.sources(&world), mover, 0).unwrap();

    let name = fleet_label(&world, target).unwrap();
    assert_ne!(name, fleet_label(&world, mover).unwrap());
    assert_eq!(
        super::tests::rows(&rows[1..3]),
        [("Fleet:", name.as_str()), ("Status:", "Enroute")]
    );
}

#[test]
fn a_fleet_without_a_hyperdrive_ship_reads_no() {
    // FUN_00449200 34645 "No" when no ship can jump.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    world.fleets[fleet].capital_ships[1].alive = false;
    let states = States::new();

    let rows = fleet_rows(states.sources(&world), fleet).unwrap();

    assert_eq!(
        super::tests::rows(&rows).last(),
        Some(&("Hyperdrive Rating:", "No"))
    );
}

#[test]
fn a_ships_weapon_arcs_read_fore_aft_starboard_then_port() {
    // FUN_00446fd0 pushes arc 3 before arc 2 (0x447f2c..0x448650).
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    let states = States::new();

    let rows = ship_rows(states.sources(&world), fleet, 0).unwrap();
    let arcs: Vec<(&str, &str)> = super::tests::rows(&rows)
        .into_iter()
        .filter(|(label, _)| label.ends_with("Arc Rating:") || *label == "Turbo Laser:")
        .collect();

    assert_eq!(
        arcs,
        vec![
            ("Forward Weapons Arc Rating:", ""),
            ("Turbo Laser:", "0"),
            ("Aft Weapons Arc Rating:", ""),
            ("Turbo Laser:", "0"),
            ("Starboard Weapons Arc Rating:", ""),
            ("Turbo Laser:", "3"),
            ("Port Weapons Arc Rating:", ""),
            ("Turbo Laser:", "4"),
        ]
    );
}

#[test]
fn a_squadron_has_no_status_row_and_scales_shields_by_its_craft() {
    // FUN_004444c0: Attached, then the ratings; shield strength is per
    // craft times the squadron size. Attached names a ship, not the
    // fleet (A0 2026-11-14-quadrants-status-known/21); this squadron
    // rides on no ship, so the first one, here unnumbered.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    let states = States::new();

    let rows = fighter_rows(states.sources(&world), fleet, 0).unwrap();

    assert_eq!(
        super::tests::rows(&rows[..5]),
        vec![
            ("Attached: ", "Small"),
            ("Maintenance Cost:", "2"),
            ("Squadron Size:", "12:12"),
            ("Hyperdrive Rating:", "0"),
            ("Maximum Shield Strength:", "60:60"),
        ]
    );
}

#[test]
fn a_squadron_with_losses_shows_its_current_values_over_the_maximum() {
    // FUN_004444c0: Squadron Size is +0x244 less +0x254 (FUN_00503100,
    // the losses +0x60) over +0x244; each rating is the class value
    // times the size less it times the losses (+0x1e0 less +0x204), and
    // bombardment loses value x losses / size (FUN_00503110 ->
    // FUN_0053e170, truncated). A squadron with losses is damaged
    // (FUN_00503340): its portrait takes the + 0x2000 mark.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    let class = world.fleets[fleet].fighters[0].class;
    world.fighter_classes[class].bombardment_defense = 10;
    world.fighter_classes[class].laser_cannon_attack_strength = 2;
    world.fleets[fleet].fighters[0].damage = Some(rebellion_core::world::SquadronDamage {
        lost: 5,
        repair: None,
    });
    let states = States::new();

    let rows = fighter_rows(states.sources(&world), fleet, 0).unwrap();
    let shown = super::tests::rows(&rows);
    for expected in [
        ("Squadron Size:", "7:12"),
        ("Maximum Shield Strength:", "35:60"),
        ("Bombardment Value:", "6:10"),
        (" Laser Rating:", "14:24"),
    ] {
        assert!(shown.contains(&expected), "{expected:?} in {shown:?}");
    }
    let view = status_view(states.sources(&world), StatusObject::Fighter { fleet, index: 0 })
        .unwrap();
    assert!(view.marks.iter().any(|(_, id)| id & 0x2000 != 0));
}

#[test]
fn a_planets_squadron_shows_the_losses_of_the_first_of_its_class() {
    // FUN_004444c0 on a planet's squadron: port, named by class, the
    // first squadron of the class still there (+0x60 of that one).
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    let damaged_class = world.fleets[fleet].fighters[0].class;
    let other = world.fighter_classes.insert(world.fighter_classes[damaged_class].clone());
    let entry = |class, count, lost| rebellion_core::world::FighterEntry {
        class,
        count,
        carrier: None,
        damage: (lost > 0).then_some(rebellion_core::world::SquadronDamage { lost, repair: None }),
    };
    world.systems[here].fighters =
        vec![entry(other, 1, 0), entry(damaged_class, 0, 0), entry(damaged_class, 1, 5)];
    let states = States::new();
    let size = |class| {
        planet_fighter_rows(states.sources(&world), here, class).unwrap()[2].value.clone()
    };
    assert_eq!(size(damaged_class), "7:12");
    assert_eq!(size(other), "12:12");
}

#[test]
fn a_squadron_on_a_planet_is_attached_to_its_system() {
    // FUN_004444c0: Attached names the squadron's container, here the
    // system (FUN_005039d0); the ratings follow as aboard a fleet.
    let mut world = GameWorld::default();
    let here = system(
        &mut world,
        "Yavin",
        ControlKind::Controlled(Faction::Alliance),
    );
    let fleet = carrier_fleet(&mut world, here);
    let class = world.fleets[fleet].fighters[0].class;
    let states = States::new();

    let rows = planet_fighter_rows(states.sources(&world), here, class).unwrap();
    assert_eq!(
        super::tests::rows(&rows[..3]),
        vec![
            ("Attached: ", "Yavin"),
            ("Maintenance Cost:", "2"),
            ("Squadron Size:", "12:12"),
        ]
    );
    let view = status_view(
        states.sources(&world),
        StatusObject::PlanetFighter {
            system: here,
            class,
        },
    )
    .unwrap();
    assert_eq!(view.title, "Fighter Squadron Status");
    assert_eq!(view.name, "X-wing");
    assert!(planet_fighter_rows(states.sources(&world), SystemKey::default(), class).is_none());
}

#[test]
fn a_regiment_reads_its_system_and_class_strengths() {
    // FUN_00445280: Attached, Status, maintenance, attack, defense,
    // bombardment, detection.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let class = catalog(
        &mut world,
        0x1000_0002,
        "Alliance Army Regiment",
        BuildableClass {
            maintenance_cost: 1,
            stats: ClassStats {
                bombardment: 4,
                ..ClassStats::default()
            },
            ..BuildableClass::default()
        },
    );
    world.troop_classes.insert(
        class,
        rebellion_core::world::TroopClassDef {
            attack_strength: 7,
            defense_strength: 9,
            detection: 3,
            uprising_defense: 0,
        },
    );
    let troop = world.troops.insert(TroopUnit {
        class_dat_id: class,
        is_alliance: true,
        regiment_strength: 100,
    });
    world.systems[here].ground_units.push(troop);
    let states = States::new();

    let rows = troop_rows(states.sources(&world), troop).unwrap();

    assert_eq!(
        super::tests::rows(&rows),
        vec![
            ("Attached: ", "Hoth"),
            ("Status:", "Awaiting Orders"),
            ("Maintenance Cost:", "1"),
            ("Attack Strength:", "7"),
            ("Defense Strength:", "9"),
            ("Bombardment Value:", "4"),
            ("Detection Value:", "3"),
        ]
    );
    // FUN_0042c3b0: a regiment's picture stands on STRATEGY 11514.
    let view = status_view(states.sources(&world), StatusObject::Troop(troop)).unwrap();
    assert_eq!(view.marks, [(DllSource::Strategy, 11_514)]);
}

#[test]
fn a_frozen_copys_special_force_squadron_and_facility_read_their_status() {
    // FUN_004f2d10 reads the player's copy (FUN_004f9a60): a special
    // force (FUN_00445780) en route until day 50, a squadron
    // (FUN_004444c0) and a battery (FUN_00444e20) and a mine
    // (FUN_00446c50), each attached to the copied system.
    use crate::viewed_contents::ViewedUnit;
    let mut world = GameWorld::default();
    let here = system(&mut world, "Chandrila", ControlKind::Controlled(Faction::Empire));
    let force = catalog(&mut world, 0x3c00_0007, "Imperial Commandos", BuildableClass::default());
    let mut skills = [rebellion_core::world::SkillPair { base: 0, variance: 0 }; 8];
    skills[0].base = 11;
    skills[1].base = 12;
    skills[5].base = 13;
    skills[6].base = 14;
    world.special_force_classes.insert(
        force,
        rebellion_core::world::SpecialForceClassDef {
            skills,
            mission_mask: 0,
        },
    );
    let states = States::new();
    let view = |object| status_view(states.sources(&world), object).unwrap();
    let commandos = view(StatusObject::ViewedUnit {
        system: here,
        unit: ViewedUnit::SpecialForce {
            class: force,
            is_alliance: false,
            arrival: Some(50),
        },
    });
    assert_eq!(commandos.title, "Spec Forces Status");
    assert_eq!(commandos.name, "Imperial Commandos");
    assert_eq!(commandos.side, 1);
    assert_eq!(
        super::tests::rows(&commandos.rows),
        vec![
            ("Attached: ", "Chandrila"),
            ("Status:", "Enroute"),
            ("ETA Destination:", "Day 50"),
            ("Maintenance Cost:", "0"),
            ("Diplomacy Rating:", "11"),
            ("Espionage Rating:", "12"),
            ("Combat Rating:", "13"),
            ("Leadership Rating:", "14"),
        ]
    );
    assert!(!commandos.marks.is_empty());

    let battery = view(StatusObject::ViewedUnit {
        system: here,
        unit: ViewedUnit::Facility {
            class: DatId::new(0x2200_0001),
            owner: 2,
        },
    });
    assert_eq!(battery.title, "Defense Facility Status");
    assert_eq!(battery.side, 1);
    assert_eq!(super::tests::rows(&battery.rows[..2]), vec![("Location:", "Chandrila"), ("Status:", "Active")]);
    let mine = view(StatusObject::ViewedUnit {
        system: here,
        unit: ViewedUnit::Facility {
            class: DatId::new(0x2c00_0001),
            owner: 1,
        },
    });
    assert_eq!(mine.title, "Manufacturing Status");
    assert_eq!(mine.side, 0);
    assert_eq!(super::tests::rows(&mine.rows[..1]), vec![("Location:", "Chandrila")]);

    world.fighter_classes.insert(rebellion_core::world::FighterClass {
        dat_id: DatId::new(0x1c00_0005),
        name: "TIE Fighter".into(),
        is_alliance: false,
        ..Default::default()
    });
    let states = States::new();
    let tie = status_view(
        states.sources(&world),
        StatusObject::ViewedUnit {
            system: here,
            unit: ViewedUnit::Squadron {
                class: DatId::new(0x1c00_0005),
                is_alliance: false,
                arrival: None,
            },
        },
    )
    .unwrap();
    assert_eq!(tie.title, "Fighter Squadron Status");
    assert_eq!(tie.name, "TIE Fighter");
    // Seen en route: its ETA and its craft's en route mark.
    let travelling = status_view(
        states.sources(&world),
        StatusObject::ViewedUnit {
            system: here,
            unit: ViewedUnit::Squadron {
                class: DatId::new(0x1c00_0005),
                is_alliance: false,
                arrival: Some(70),
            },
        },
    )
    .unwrap();
    assert_eq!(
        super::tests::rows(&travelling.rows[..2]),
        vec![("Attached: ", "Chandrila"), ("ETA Destination:", "Day 70")]
    );
    assert!(!travelling.marks.is_empty());
    assert_eq!(
        ViewedUnit::Facility {
            class: DatId::new(0x2200_0001),
            owner: 2
        }
        .arrival(),
        None
    );
    assert_eq!(super::tests::rows(&tie.rows[..2]), vec![("Attached: ", "Chandrila"), ("Maintenance Cost:", "0")]);
    assert!(tie.marks.is_empty());
    assert!(status_view(
        states.sources(&world),
        StatusObject::ViewedUnit {
            system: here,
            unit: ViewedUnit::Squadron {
                class: DatId::new(0x1c00_0099),
                is_alliance: false,
                arrival: None,
            },
        },
    )
    .is_none());
}

#[test]
fn a_frozen_copy_of_an_enemy_regiment_reads_enroute_with_its_old_eta() {
    // A0 OBJ-02-C010 (18-chandrila-army-regiment-status-enroute-eta-
    // day171-on-day576-stale, 6a1269a1765f7c9b): on day 576 the
    // Alliance's Status window of an Imperial regiment on Chandrila
    // reads "Enroute", "ETA Destination: Day 171". The window reads the
    // side's copy (FUN_004f2d10 -> FUN_0053efd0), frozen while en route
    // (FUN_004f9a60).
    use rebellion_core::object_views::{apply, ObjectView, RegimentView, ViewPlace, ViewUpdate};
    let mut world = GameWorld::default();
    let chandrila = system(&mut world, "Chandrila", ControlKind::Controlled(Faction::Empire));
    let coruscant = system(&mut world, "Coruscant", ControlKind::Controlled(Faction::Empire));
    world.systems[coruscant].dat_id = DatId::new(0x9000_0002);
    let class = catalog(
        &mut world,
        0x1000_0001,
        "Imperial Army Regiment",
        BuildableClass::default(),
    );
    // The live regiment has long since landed on Coruscant.
    let troop = world.troops.insert(TroopUnit {
        class_dat_id: class,
        is_alliance: false,
        regiment_strength: 100,
    });
    world.systems[coruscant].ground_units.push(troop);
    let id = DatId::new(0x1000_0007);
    let place = ViewPlace::Surface(world.systems[chandrila].dat_id);
    apply(
        &mut world,
        &[ViewUpdate::Set {
            side: Faction::Alliance,
            id,
            view: ObjectView::Regiment(RegimentView {
                class,
                is_alliance: false,
                strength: 100,
                place,
                arrival: Some(171),
                live: false,
            }),
        }],
    );
    let states = States::new();

    let view = status_view(
        states.sources(&world),
        StatusObject::ViewedTroop { alliance: true, id },
    )
    .unwrap();
    assert_eq!(view.title, "Trooper Regiment Status");
    assert_eq!(view.name, "Imperial Army Regiment");
    assert_eq!(
        super::tests::rows(&view.rows[..3]),
        vec![
            ("Attached: ", "Chandrila"),
            ("Status:", "Enroute"),
            ("ETA Destination:", "Day 171"),
        ]
    );
    // FUN_0042c3b0: the en route mark over the backdrop.
    assert_eq!(
        view.marks,
        [(DllSource::Strategy, 11_514), (DllSource::Strategy, 11_516)]
    );
    // The Empire holds no such copy, and the live regiment reads true.
    assert!(status_view(
        states.sources(&world),
        StatusObject::ViewedTroop { alliance: false, id },
    )
    .is_none());
    let live = troop_rows(states.sources(&world), troop).unwrap();
    assert_eq!(
        super::tests::rows(&live[..2]),
        vec![("Attached: ", "Coruscant"), ("Status:", "Awaiting Orders")]
    );
    // A0 2026-11-14-stale-views/07-kothlis-reg1-status-d375: a copy
    // frozen on its planet, not en route, reads "Attached Kothlis,
    // Awaiting Orders" with no ETA row.
    let resting = DatId::new(0x1000_0008);
    let home = ViewPlace::Surface(world.systems[chandrila].dat_id);
    apply(
        &mut world,
        &[ViewUpdate::Set {
            side: Faction::Alliance,
            id: resting,
            view: ObjectView::Regiment(RegimentView {
                class,
                is_alliance: false,
                strength: 100,
                place: home,
                arrival: None,
                live: false,
            }),
        }],
    );
    let states = States::new();
    let at_rest = status_view(
        states.sources(&world),
        StatusObject::ViewedTroop {
            alliance: true,
            id: resting,
        },
    )
    .unwrap();
    assert_eq!(
        super::tests::rows(&at_rest.rows[..3]),
        vec![
            ("Attached: ", "Chandrila"),
            ("Status:", "Awaiting Orders"),
            ("Maintenance Cost:", "0"),
        ]
    );
}

#[test]
fn a_special_force_reads_skills_zero_one_five_and_six() {
    // FUN_00445780: slots +0x1dc, +0x1e0, +0x1f0, +0x1f4.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let unit = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: true,
        skills: [10, 11, 12, 13, 14, 15, 16, 17],
        on_mission: true,
        on_hidden_mission: false,
    });
    world.systems[here].special_forces.push(unit);
    let states = States::new();

    let rows = special_force_rows(states.sources(&world), unit).unwrap();

    assert_eq!(
        super::tests::rows(&rows),
        vec![
            ("Attached: ", "Hoth"),
            ("Status:", "On Mission"),
            ("Maintenance Cost:", "0"),
            ("Diplomacy Rating:", "10"),
            ("Espionage Rating:", "11"),
            ("Combat Rating:", "15"),
            ("Leadership Rating:", "16"),
        ]
    );
}

#[test]
fn a_special_force_aboard_a_fleet_is_attached_to_its_ship() {
    // FUN_00445780: Attached names the unit's container
    // (FUN_0044a2e0, FUN_004f62d0), the capital ship it rides on
    // (FUN_00536da0(ship, 3)); the port seats it on the fleet's first
    // ship (fleet_window::ship_cargo). hyp: as a character's, A0
    // 2026-11-14-quadrants-status-known/32.
    let mut world = GameWorld::default();
    let here = system(
        &mut world,
        "Yavin",
        ControlKind::Controlled(Faction::Alliance),
    );
    let fleet = carrier_fleet(&mut world, here);
    let unit = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
        on_hidden_mission: false,
    });
    world.fleets[fleet].special_forces.push(unit);
    world.number_ships();
    let states = States::new();

    let rows = special_force_rows(states.sources(&world), unit).unwrap();

    assert_eq!(super::tests::rows(&rows)[0], ("Attached: ", "Small 1"));
}

#[test]
fn a_squadron_aboard_a_fleet_is_attached_to_the_ship_that_carries_it() {
    // FUN_004444c0 names the squadron's container (FUN_0044a2e0,
    // FUN_004f62d0), the capital ship it rides on, not the fleet. A0
    // 2026-11-14-quadrants-status-known/21, 22: Fleet 7's two Y-wings
    // read "Alliance Escort Carrier 1" and "Alliance Escort Carrier 3";
    // 2026-11-13-icon-drag-move/09: an X-wing, "Alliance Escort Carrier 1".
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    world.number_ships();
    let class = world.fleets[fleet].fighters[0].class;
    let ids: Vec<u64> = world.fleets[fleet].capital_ships.iter().map(|ship| ship.id).collect();
    world.fleets[fleet].fighters = vec![
        FighterEntry { class, count: 1, carrier: Some(ids[1]), damage: None },
        FighterEntry { class, count: 1, carrier: Some(ids[0]), damage: None },
    ];
    let states = States::new();
    let attached =
        |index| fighter_rows(states.sources(&world), fleet, index).unwrap()[0].value.clone();

    assert_eq!(attached(0), "Large 2");
    assert_eq!(attached(1), "Small 1");
}

#[test]
fn a_regiment_aboard_a_fleet_is_attached_to_the_ship_that_holds_it() {
    // FUN_00445280 names the regiment's container (FUN_0044a2e0), the
    // capital ship that holds it. A0 2026-11-14-missions-transfer/36:
    // the Fleet Regiment aboard Fleet 1 reads "Attached: Medium
    // Transport 1" (Corellian Corvette 1 has no troop room). The port
    // seats regiments by troop room in fleet order (ship_cargo), and
    // one on its way in on the ship with the most room left (ship_load,
    // FUN_00552300).
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    // Room for none on the first ship and two on the second.
    let small = world.fleets[fleet].capital_ships[0].class;
    world.capital_ship_classes[small].troop_capacity = 0;
    let large = world.fleets[fleet].capital_ships[1].class;
    world.capital_ship_classes[large].troop_capacity = 2;
    world.number_ships();
    let troop = |world: &mut GameWorld| {
        world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1000_0002),
            is_alliance: true,
            regiment_strength: 100,
        })
    };
    let aboard = troop(&mut world);
    world.systems[here].ground_units.push(aboard);
    let mut states = States::new();
    states.transport.embark(&mut world, fleet, &[aboard]).unwrap();
    assert_eq!(
        troop_rows(states.sources(&world), aboard).unwrap()[0].value,
        "Large 2"
    );

    let coming = troop(&mut world);
    states.transport = with_entry(
        &states.transport,
        "transit",
        rebellion_core::troop_transport::RegimentTransit {
            troop: coming,
            origin: here,
            leg: RegimentLeg::Fleet(fleet),
            arrival_tick: 20,
        },
    );
    assert_eq!(
        troop_rows(states.sources(&world), coming).unwrap()[0].value,
        "Large 2"
    );
}

#[test]
fn a_character_aboard_a_fleet_is_attached_to_the_ship_it_rides() {
    // FUN_004486f0 names the character's container (FUN_0044a2e0), the
    // ship it is seated on (Character::ship), else the fleet's first
    // ship, where missions::seat_characters seats it. A0
    // 2026-11-14-quadrants-status-known/32 (on its way in).
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    world.number_ships();
    let rider = world.characters.insert(rebellion_core::world::Character {
        current_fleet: Some(fleet),
        ..Default::default()
    });
    world.fleets[fleet].characters.push(rider);
    let attached =
        |world: &GameWorld| character_rows(world, &[], &[], rider).unwrap()[1].value.clone();
    assert_eq!(attached(&world), "Small 1");
    world.characters[rider].ship = Some(world.fleets[fleet].capital_ships[1].id);
    assert_eq!(attached(&world), "Large 2");
}

#[test]
fn only_armed_defense_families_show_weapons_and_shield() {
    // FUN_00444e20 writes Weapons Rating and Shield Strength for
    // families 0x22..0x27 only.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let armed_stats = ClassStats {
        bombardment: 2,
        attack_strength: 30,
        shield_strength: 40,
    };
    let mut defense = |id| {
        let class = catalog(
            &mut world,
            id,
            "Defense",
            BuildableClass {
                stats: armed_stats,
                ..BuildableClass::default()
            },
        );
        let key = world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: class,
            side: Faction::Alliance,
        });
        world.systems[here].defense_facilities.push(key);
        key
    };
    let armed = defense(0x2200_0001);
    let unarmed = defense(0x2100_0001);
    let states = States::new();
    let sources = states.sources(&world);

    let labels = |key| -> Vec<String> {
        defense_facility_rows(sources, key)
            .unwrap()
            .into_iter()
            .map(|row| row.label)
            .collect()
    };

    assert_eq!(
        labels(armed),
        vec![
            "Location:",
            "Status:",
            "Maintenance Cost:",
            "Weapons Rating:",
            "Shield Strength",
            "Bombardment Defense Strength:",
        ]
    );
    assert_eq!(
        labels(unarmed),
        vec![
            "Location:",
            "Status:",
            "Maintenance Cost:",
            "Bombardment Defense Strength:",
        ]
    );
}

#[test]
fn a_producer_reads_no_facilities_idle_then_its_queue() {
    // FUN_0044acd0: 34671 "No Facilities", 34670 "Idle", 34673
    // "Training" with the items left and the completion day.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let mut states = States::new();
    let status = |world: &GameWorld, states: &States| -> Vec<(String, String)> {
        producer_rows(states.sources(world), here, ProductionArea::TrainingFacility)
            .unwrap()
            .into_iter()
            .map(|row| (row.label, row.value))
            .collect()
    };
    let pair = |label: &str, value: &str| (label.to_owned(), value.to_owned());

    assert_eq!(status(&world, &states)[1], pair("Status:", "No Facilities"));

    let class_dat_id = catalog(
        &mut world,
        0x2900_0001,
        "Training Facility",
        BuildableClass {
            processing_rate: 2,
            ..BuildableClass::default()
        },
    );
    let yard = world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id,
            side: Faction::Alliance,
            is_shipyard: false,
        });
    world.systems[here].manufacturing_facilities.push(yard);
    assert_eq!(status(&world, &states)[1], pair("Status:", "Idle"));

    states.manufacturing.enqueue(
        here,
        QueueItem::new(BuildableKind::Troop(DatId::new(0x1000_0002)), 9),
    );
    let rows = status(&world, &states);
    assert_eq!(rows[1], pair("Status:", "Training"));
    assert_eq!(rows[2], pair("Items to Build:", "1"));
    // FUN_00528d30: nine points at one every two days, from day 7.
    assert_eq!(rows[3], pair("Estimated Day of Completion:", "25"));
    assert_eq!(rows.len(), 4);
}

#[test]
fn a_mission_counts_its_decoys_in_the_team() {
    // FUN_00445c60: Target, Team Size (members and decoys), Decoys.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Bespin", ControlKind::Uncontrolled);
    let mut states = States::new();
    let a = world.characters.insert(rebellion_core::world::Character::default());
    let b = world.characters.insert(rebellion_core::world::Character::default());
    let id = states.missions.dispatch(MissionRequest {
        decoys: vec![MissionMember::Character(b)],
        ..MissionRequest::single(MissionKind::Diplomacy, MissionFaction::Empire, a, here, None, 0)
    });

    let rows = mission_rows(states.sources(&world), id).unwrap();

    assert_eq!(
        super::tests::rows(&rows),
        vec![("Target:", "Bespin"), ("Team Size:", "2"), ("Decoys:", "1")]
    );
}

#[test]
fn each_family_has_its_title_and_picture() {
    // FUN_00443130 titles; FUN_0042c3b0(.., 1, 1): STRATEGY 10425 for a
    // side-1 fleet, a class's +0x30 resource & 0xfff (its list mini less
    // 0x4000), the producer's GOKRES 264 with the manager font.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    let states = States::new();
    let sources = states.sources(&world);
    let view = |object| status_view(sources, object).unwrap();

    let fleet_view = view(StatusObject::Fleet(fleet));
    assert_eq!(fleet_view.title, "Fleet Status");
    assert_eq!(fleet_view.picture, Some((DllSource::Strategy, 10_425)));
    assert_eq!(fleet_view.side, 0);

    let ship = view(StatusObject::Ship { fleet, index: 0 });
    assert_eq!(ship.title, "Capital Ship Status");
    let mini = capital_ship_mini_id(DatId::new(0x1400_0040)).unwrap();
    assert_eq!(ship.picture, Some((DllSource::Gokres, mini - 0x4000)));

    let fighter = view(StatusObject::Fighter { fleet, index: 0 });
    assert_eq!(fighter.title, "Fighter Squadron Status");
    let mini = fighter_mini_id(DatId::new(0x1c00_0001)).unwrap();
    assert_eq!(fighter.picture, Some((DllSource::Gokres, mini - 0x4000)));

    let producer = view(StatusObject::Producer {
        system: here,
        area: ProductionArea::TrainingFacility,
    });
    assert_eq!(producer.title, "Troops in Training");
    assert!(producer.manager_title);
    assert_eq!(producer.picture, Some((DllSource::Gokres, 264)));
    assert!(!fleet_view.manager_title);
}

#[test]
fn a_producers_status_names_its_manager_class() {
    // FUN_004f62d0: the manager's class name, TEXTSTRA 11524..11526.
    // A0 pending-facility/34: "Ship Construction" over GOKRES 262 (0 of
    // 10728 pixels off), named "Shipyard", at Pzob.
    let mut world = GameWorld::default();
    let here = system(
        &mut world,
        "Pzob",
        ControlKind::Controlled(Faction::Alliance),
    );
    let states = States::new();
    let names = [
        ProductionArea::Shipyard,
        ProductionArea::ConstructionYard,
        ProductionArea::TrainingFacility,
    ]
    .map(|area| {
        let view = status_view(
            states.sources(&world),
            StatusObject::Producer { system: here, area },
        )
        .unwrap();
        (view.name, view.picture)
    });
    assert_eq!(
        names,
        [
            ("Shipyard".to_owned(), Some((DllSource::Gokres, 262))),
            ("Construction".to_owned(), Some((DllSource::Gokres, 263))),
            ("Training".to_owned(), Some((DllSource::Gokres, 264))),
        ]
    );
}

#[test]
fn an_empire_fleet_shows_the_other_fleet_picture() {
    // FUN_0042c3b0: STRATEGY 10475 for any side but 1.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Coruscant", ControlKind::Controlled(Faction::Empire));
    let fleet = carrier_fleet(&mut world, here);
    world.fleets[fleet].is_alliance = false;
    let states = States::new();

    let view = status_view(states.sources(&world), StatusObject::Fleet(fleet)).unwrap();

    assert_eq!(view.picture, Some((DllSource::Strategy, 10_475)));
    assert_eq!(view.side, 1);
}

#[test]
fn a_facility_takes_its_own_side_for_the_background() {
    // FUN_00443130 picks the background by the object's side bits; a
    // contested system's facility keeps its last holder
    // (ghidra/notes/facility-ownership.md).
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Contested);
    let yard = world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id: DatId::new(0x2900_0001),
            side: Faction::Empire,
            is_shipyard: false,
        });
    world.systems[here].manufacturing_facilities.push(yard);
    let states = States::new();

    let view = status_view(
        states.sources(&world),
        StatusObject::ManufacturingFacility(yard),
    )
    .unwrap();

    assert_eq!(view.title, "Manufacturing Status");
    assert_eq!(view.side, 1);
}

#[test]
fn the_headquarters_shows_defense_facility_status_with_its_bombardment_defense() {
    // FUN_00443130 gives families 0x20..0x27 to FUN_00444e20 (TEXTSTRA
    // 34403); weapons and shield rows only for 0x22..0x27. The HQ's
    // vtable 0x0065f280 +0x1c0 is 0x558050 (FUN_00520b60, class +0x58)
    // and +0x1c4 is 0x6158b0 (0).
    let mut world = GameWorld::default();
    let here = system(
        &mut world,
        "Yavin",
        ControlKind::Controlled(Faction::Alliance),
    );
    let class = DatId::new(0x2000_0001);
    world.buildable_classes.insert(
        class,
        rebellion_core::world::BuildableClass {
            maintenance_cost: 2,
            stats: rebellion_core::world::ClassStats {
                bombardment: 9,
                attack_strength: 7,
                shield_strength: 5,
            },
            processing_rate: 3,
            ..Default::default()
        },
    );
    let hq = world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id: class,
            side: Faction::Alliance,
            is_shipyard: false,
        });
    world.systems[here].manufacturing_facilities.push(hq);
    let states = States::new();

    let view = status_view(
        states.sources(&world),
        StatusObject::ManufacturingFacility(hq),
    )
    .unwrap();

    assert_eq!(view.title, "Defense Facility Status");
    assert_eq!(view.picture, Some((DllSource::Gokres, 17_216 - 0x4000)));
    assert_eq!(
        rows(&view.rows),
        [
            ("Location:", "Yavin"),
            ("Status:", "Active"),
            ("Maintenance Cost:", "2"),
            ("Bombardment Defense Strength:", "9"),
        ]
    );
}

#[test]
fn the_headquarters_en_route_leads_with_its_eta_and_reads_enroute() {
    // FUN_00444e20: FUN_004fd2b0's ETA rows before Location, then
    // FUN_0044a470's 34628 "Enroute" for +0x50 bit 4.
    let mut world = GameWorld::default();
    let here = system(
        &mut world,
        "Yavin",
        ControlKind::Controlled(Faction::Alliance),
    );
    let class = DatId::new(0x2000_0001);
    let hq = world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id: class,
            side: Faction::Alliance,
            is_shipyard: false,
        });
    world.systems[here].manufacturing_facilities.push(hq);
    let mut states = States::new();
    states.transport.headquarters =
        Some(rebellion_core::headquarters_move::HeadquartersTransit {
            origin: SystemKey::default(),
            destination: here,
            arrival_tick: 40,
        });

    let view = status_view(
        states.sources(&world),
        StatusObject::ManufacturingFacility(hq),
    )
    .unwrap();

    assert_eq!(
        rows(&view.rows),
        [
            ("ETA Destination:", "Day 40"),
            ("Location:", "Yavin"),
            ("Status:", "Enroute"),
            ("Maintenance Cost:", "0"),
            ("Bombardment Defense Strength:", "0"),
        ]
    );
    // FUN_0042c3b0: 11506 under an en route facility's portrait.
    assert_eq!(view.marks, [(DllSource::Strategy, 11_506)]);
    // A yard where the HQ is bound is not the HQ: no ETA, no mark.
    let yard = world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id: DatId::new(0x2800_0001),
            side: Faction::Alliance,
            is_shipyard: true,
        });
    world.systems[here].manufacturing_facilities.push(yard);
    let yard_view = status_view(
        states.sources(&world),
        StatusObject::ManufacturingFacility(yard),
    )
    .unwrap();
    assert!(yard_view.marks.is_empty());
    assert_eq!(yard_view.rows[1], row("Status:", "Active"));
}

#[test]
fn an_unfinished_refinery_reads_under_construction_and_en_route_with_its_marks() {
    // A0 pending-facility/13 (Commenor, built at Pzob): Manufacturing
    // Status, "Location: Commenor", "Status: Under Construction"
    // (FUN_0044a470, +0x50 bit 2 clear), the refinery's portrait over
    // STRATEGY 11571 (FUN_0042c3b0, side 1). En route (bit 4) it reads
    // FUN_004fd2b0's ETA after Location (FUN_00446c50), "Enroute" and
    // 11506.
    let mut world = GameWorld::default();
    let pzob = system(
        &mut world,
        "Pzob",
        ControlKind::Controlled(Faction::Alliance),
    );
    let commenor = system(
        &mut world,
        "Commenor",
        ControlKind::Controlled(Faction::Alliance),
    );
    let class = DatId::new(0x2d00_0002);
    world.buildable_classes.insert(
        class,
        rebellion_core::world::BuildableClass {
            name: "Refinery".into(),
            processing_rate: 5,
            stats: rebellion_core::world::ClassStats {
                bombardment: 2,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let mut states = States::new();
    let refinery =
        BuildableKind::ProductionFacility(rebellion_core::manufacturing::FacilityBuild {
            class,
            is_alliance: true,
        });
    states
        .manufacturing
        .enqueue(pzob, QueueItem::new(refinery, 5).delivered_to(commenor));
    let pending = |arrival| StatusObject::PendingFacility {
        system: commenor,
        class,
        owner: 1,
        arrival,
    };

    let building = status_view(states.sources(&world), pending(None)).unwrap();
    assert_eq!(building.title, "Manufacturing Status");
    assert_eq!(building.name, "Refinery");
    assert_eq!(building.picture, Some((DllSource::Gokres, 16_386 - 0x4000)));
    assert_eq!(building.marks, [(DllSource::Strategy, 11_571)]);
    assert_eq!(
        rows(&building.rows),
        [
            ("Location:", "Commenor"),
            ("Status:", "Under Construction"),
            ("Maintenance Cost:", "0"),
            ("Standard Processing Rate:", "5"),
            ("Bombardment Value:", "2"),
        ]
    );
    let travelling = status_view(states.sources(&world), pending(Some(12))).unwrap();
    assert_eq!(travelling.marks, [(DllSource::Strategy, 11_506)]);
    assert_eq!(
        rows(&travelling.rows)[..3],
        [
            ("Location:", "Commenor"),
            ("ETA Destination:", "Day 12"),
            ("Status:", "Enroute"),
        ]
    );
    // The Empire's takes 11573 and its own background row.
    let empire = StatusObject::PendingFacility {
        system: commenor,
        class,
        owner: 2,
        arrival: None,
    };
    let imperial = status_view(states.sources(&world), empire).unwrap();
    assert_eq!(imperial.marks, [(DllSource::Strategy, 11_573)]);
    assert_eq!((building.side, imperial.side), (0, 1));
    // Gone once it arrives (today 7 in these sources) or leaves the
    // queue.
    assert!(status_view(states.sources(&world), pending(Some(6))).is_none());
    states.manufacturing = ManufacturingState::new();
    assert!(status_view(states.sources(&world), pending(None)).is_none());
}

#[test]
fn an_unfinished_battery_reads_under_construction_and_en_route_with_its_marks() {
    // FUN_00443130 fills 0x20..0x27 with FUN_00444e20: the ETA before
    // Location, then FUN_0044a470's 34647 "Under Construction" (bit 2
    // clear) or 34628 "Enroute" (bit 4). FUN_0042c3b0(.., 1, 1) lays
    // 11571 (side 1) / 11573 under it unfinished, and 11510 (0x2cf5 + 1,
    // families 0x22..0x27) en route.
    let mut world = GameWorld::default();
    let here = system(
        &mut world,
        "Pzob",
        ControlKind::Controlled(Faction::Alliance),
    );
    let class = DatId::new(0x2200_0001);
    world.buildable_classes.insert(
        class,
        rebellion_core::world::BuildableClass {
            name: "KDY v-150 Planet Defender".into(),
            stats: rebellion_core::world::ClassStats {
                attack_strength: 3,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let mut states = States::new();
    let battery =
        BuildableKind::DefenseFacility(rebellion_core::manufacturing::FacilityBuild {
            class,
            is_alliance: true,
        });
    states
        .manufacturing
        .enqueue(here, QueueItem::new(battery, 5));
    let pending = |owner, arrival| StatusObject::PendingFacility {
        system: here,
        class,
        owner,
        arrival,
    };

    let building = status_view(states.sources(&world), pending(1, None)).unwrap();
    assert_eq!(building.title, "Defense Facility Status");
    assert_eq!(building.marks, [(DllSource::Strategy, 11_571)]);
    assert_eq!(
        building.picture,
        picture(defense_facility_mini(class).map(|(mini, _)| mini))
    );
    assert_eq!(
        rows(&building.rows)[..3],
        [
            ("Location:", "Pzob"),
            ("Status:", "Under Construction"),
            ("Maintenance Cost:", "0"),
        ]
    );
    let imperial = status_view(states.sources(&world), pending(2, None)).unwrap();
    assert_eq!(imperial.marks, [(DllSource::Strategy, 11_573)]);
    let travelling = status_view(states.sources(&world), pending(1, Some(12))).unwrap();
    assert_eq!(travelling.marks, [(DllSource::Strategy, 11_510)]);
    assert_eq!(
        rows(&travelling.rows)[..3],
        [
            ("ETA Destination:", "Day 12"),
            ("Location:", "Pzob"),
            ("Status:", "Enroute"),
        ]
    );
    states.manufacturing = ManufacturingState::new();
    assert!(status_view(states.sources(&world), pending(1, None)).is_none());
}

#[test]
fn a_regiment_in_training_reads_training_over_its_backdrop_and_mark() {
    // FUN_00445280: +0x50 bit 4 clear and bit 2 clear write 0x8771,
    // TEXTSTRA 34673 "Training". FUN_0042c3b0(.., 1, 1) lays the
    // regiment's backdrop 11514 (0x2cf9 + 1), then 11571 (side 1).
    let mut world = GameWorld::default();
    let here = system(
        &mut world,
        "Pzob",
        ControlKind::Controlled(Faction::Alliance),
    );
    let class = DatId::new(0x1000_0002);
    world.buildable_classes.insert(
        class,
        rebellion_core::world::BuildableClass {
            name: "Alliance Army Regiment".into(),
            is_alliance: true,
            ..Default::default()
        },
    );
    let mut states = States::new();
    states
        .manufacturing
        .enqueue(here, QueueItem::new(BuildableKind::Troop(class), 5));
    rebellion_core::products::reconcile(&mut world, &states.manufacturing);
    let pending = StatusObject::PendingTroop {
        system: here,
        class,
    };

    let training = status_view(states.sources(&world), pending).unwrap();
    assert_eq!(training.title, "Trooper Regiment Status");
    assert_eq!(
        training.marks,
        [(DllSource::Strategy, 11_514), (DllSource::Strategy, 11_571)]
    );
    assert_eq!(
        rows(&training.rows)[..3],
        [
            ("Attached: ", "Pzob"),
            ("Status:", "Training"),
            ("Maintenance Cost:", "0"),
        ]
    );
    // The regiment stands at its container's system only.
    let elsewhere = system(
        &mut world,
        "Talus",
        ControlKind::Controlled(Faction::Alliance),
    );
    assert!(status_view(
        states.sources(&world),
        StatusObject::PendingTroop {
            system: elsewhere,
            class,
        }
    )
    .is_none());
    states.manufacturing = ManufacturingState::new();
    rebellion_core::products::reconcile(&mut world, &states.manufacturing);
    assert!(status_view(states.sources(&world), pending).is_none());
}

#[test]
fn an_unfinished_squadron_shows_the_planet_squadrons_status_over_its_mark() {
    // FUN_004444c0 on a squadron not yet completed: A0
    // pending-facility/25 reads "Fighter Squadron Status", Attached:
    // Pzob and the class rows over the red grid, 11571 for side 1
    // (FUN_0042c3b0); 11573 for side 2.
    let mut world = GameWorld::default();
    let here = system(
        &mut world,
        "Pzob",
        ControlKind::Controlled(Faction::Alliance),
    );
    let mut class = |raw, name: &str, alliance| {
        world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                dat_id: DatId::new(raw),
                name: name.into(),
                is_alliance: alliance,
                is_empire: !alliance,
                ..Default::default()
            })
    };
    let awing = class(0x1c00_0001, "A-wing", true);
    let tie = class(0x1c00_0005, "TIE Fighter", false);
    let mut states = States::new();
    for fighter in [awing, tie] {
        states
            .manufacturing
            .enqueue(here, QueueItem::new(BuildableKind::Fighter(fighter), 5));
    }
    rebellion_core::products::reconcile(&mut world, &states.manufacturing);
    let pending = |class| StatusObject::PendingFighter {
        system: here,
        class,
    };

    let building = status_view(states.sources(&world), pending(awing)).unwrap();
    assert_eq!(building.title, "Fighter Squadron Status");
    assert_eq!(building.name, "A-wing");
    assert_eq!(building.marks, [(DllSource::Strategy, 11_571)]);
    assert_eq!(
        building.rows,
        planet_fighter_rows(states.sources(&world), here, awing).unwrap()
    );
    assert_eq!(rows(&building.rows)[0], ("Attached: ", "Pzob"));
    let imperial = status_view(states.sources(&world), pending(tie)).unwrap();
    assert_eq!(imperial.marks, [(DllSource::Strategy, 11_573)]);
    assert_eq!((building.side, imperial.side), (0, 1));
    // Another class's squadron in the queue keeps no A-wing's open.
    states.manufacturing = ManufacturingState::new();
    states
        .manufacturing
        .enqueue(here, QueueItem::new(BuildableKind::Fighter(tie), 5));
    rebellion_core::products::reconcile(&mut world, &states.manufacturing);
    assert!(status_view(states.sources(&world), pending(awing)).is_none());
    assert!(status_view(states.sources(&world), pending(tie)).is_some());
}

#[test]
fn an_unfinished_ship_reads_under_construction_in_its_fleet_over_its_mark() {
    // FUN_00446fd0 on a ship not yet completed: FUN_0044a470 writes
    // 34647 "Under Construction" (bit 2 clear). A0 2026-11-15
    // "u-2-carrack-status": Fleet 4, Under Construction, Maintenance
    // Cost 24, Hull 1000:1000, the picture over the Empire's grid
    // (FUN_0042c3b0: 11573 for side 2).
    let mut world = GameWorld::default();
    let here = system(
        &mut world,
        "Corellia",
        ControlKind::Controlled(Faction::Empire),
    );
    let carrack = world
        .capital_ship_classes
        .insert(rebellion_core::world::CapitalShipClass {
            dat_id: DatId::new(0x1400_0010),
            name: "Carrack Light Cruiser".into(),
            is_alliance: false,
            is_empire: true,
            hull: 1000,
            maintenance_cost: 24,
            ..Default::default()
        });
    let mut states = States::new();
    states.manufacturing.enqueue(
        here,
        QueueItem::new(BuildableKind::CapitalShip(carrack), 5).ordered(&mut world),
    );
    rebellion_core::products::reconcile(&mut world, &states.manufacturing);
    let product = world.products.all()[0].id;
    let view = status_view(
        states.sources(&world),
        StatusObject::PendingShip { product },
    )
    .unwrap();
    assert_eq!(view.title, "Capital Ship Status");
    assert_eq!(view.name, "Carrack Light Cruiser 1");
    assert_eq!(view.side, 1);
    assert_eq!(view.marks, [(DllSource::Strategy, 11_573)]);
    let fleet = world.fleets.keys().next().unwrap();
    let fleet_name = world.fleet_name(fleet).unwrap().to_owned();
    let read = rows(&view.rows);
    assert_eq!(
        read[..4],
        [
            ("Class:", "Carrack Light Cruiser"),
            ("Fleet:", fleet_name.as_str()),
            ("Status:", "Under Construction"),
            ("Maintenance Cost:", "24"),
        ]
    );
    assert!(read.contains(&("Hull Value:", "1000:1000")));
    assert!(status_view(states.sources(&world), StatusObject::PendingShip { product: 99 })
        .is_none());
    // Finished away from its yard, it travels in its fleet: FUN_0044a470
    // writes 34628 "Enroute", then FUN_004fd2b0's ETA, over the craft's
    // en route mark.
    let fleet_at = world.fleets[fleet].location;
    let elsewhere = system(
        &mut world,
        "Kuat",
        ControlKind::Controlled(Faction::Empire),
    );
    world.systems[elsewhere].x = 300;
    world.fleets[fleet].location = elsewhere;
    let name = world.products.all()[0].name.clone();
    rebellion_core::products::settle(
        &mut world,
        vec![rebellion_core::manufacturing::CompletionEvent {
            system: fleet_at,
            tick: 7,
            kind: BuildableKind::CapitalShip(carrack),
            name,
            fleet: None,
        }],
        Vec::new(),
    );
    let arrival = world.products.arrival(product).unwrap();
    let view = status_view(
        states.sources(&world),
        StatusObject::PendingShip { product },
    )
    .unwrap();
    let read = rows(&view.rows);
    let eta = format!("Day {arrival}");
    assert_eq!(read[2], ("Status:", "Enroute"));
    assert_eq!(read[3], ("ETA Destination:", eta.as_str()));
    assert!(!view.marks.contains(&(DllSource::Strategy, 11_573)));
}

#[test]
fn a_travelling_squadron_is_attached_to_its_destination_with_its_eta() {
    // FUN_004444c0 with FUN_00556390: a squadron in hyperspace is in its
    // destination's container, then FUN_004fd2b0's "ETA Destination:";
    // FUN_0042c3b0(.., 1, 1) lays its own en route mark (portrait +
    // 0x1000) under the portrait. It has no "Status:" row.
    let mut world = GameWorld::default();
    let here = system(
        &mut world,
        "Hoth",
        ControlKind::Controlled(Faction::Alliance),
    );
    let there = system(&mut world, "Bespin", ControlKind::Uncontrolled);
    let xwing = world
        .fighter_classes
        .insert(rebellion_core::world::FighterClass {
            dat_id: DatId::new(0x1c00_0001),
            name: "X-wing".into(),
            is_alliance: true,
            ..Default::default()
        });
    let mut states = States::new();
    let mut value = serde_json::to_value(&states.transport).unwrap();
    value["squadrons"]["transits"].as_array_mut().unwrap().push(
        serde_json::to_value(rebellion_core::squadrons::SquadronTransit {
            class: xwing,
            count: 1,
            origin: here,
            to: rebellion_core::squadrons::SquadronPlace::Planet(there),
            arrival_tick: 20,
            lost: Vec::new(),
        })
        .unwrap(),
    );
    states.transport = serde_json::from_value(value).unwrap();
    let travelling = |arrival| StatusObject::TravellingFighter {
        to: rebellion_core::squadrons::SquadronPlace::Planet(there),
        class: xwing,
        arrival,
    };

    let view = status_view(states.sources(&world), travelling(20)).unwrap();
    assert_eq!(view.title, "Fighter Squadron Status");
    assert_eq!(view.name, "X-wing");
    let portrait = fighter_mini_id(DatId::new(0x1c00_0001)).unwrap() - 0x4000;
    assert_eq!(view.marks, [(DllSource::Gokres, portrait + 0x1000)]);
    assert_eq!(
        super::tests::rows(&view.rows[..3]),
        vec![
            ("Attached: ", "Bespin"),
            ("ETA Destination:", "Day 20"),
            ("Maintenance Cost:", "0"),
        ]
    );
    // Once it has landed the window has nothing to show.
    assert!(status_view(states.sources(&world), travelling(21)).is_none());
}

#[test]
fn a_squadron_on_its_way_into_a_fleet_is_attached_to_the_ship_it_boards() {
    // FUN_00556390 seats a squadron Move onto a fleet on the ship with
    // the most room left (FUN_00552300, fleet_window::ship_load), and
    // FUN_004444c0 names that ship (FUN_0044a2e0), as A0
    // 2026-11-14-quadrants-status-known/21 names a squadron's carrier.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, here);
    // Room: Small 2, Large 3; the fleet's own four squadrons fill both.
    world.fleets[fleet].fighters.clear();
    world.number_ships();
    let class = world.fighter_classes.keys().next().unwrap();
    let mut states = States::new();
    let mut value = serde_json::to_value(&states.transport).unwrap();
    for arrival in [20, 21] {
        value["squadrons"]["transits"].as_array_mut().unwrap().push(
            serde_json::to_value(rebellion_core::squadrons::SquadronTransit {
                class,
                count: 1,
                origin: here,
                to: SquadronPlace::Fleet(fleet),
                arrival_tick: arrival,
                lost: Vec::new(),
            })
            .unwrap(),
        );
    }
    states.transport = serde_json::from_value(value).unwrap();
    let attached = |arrival| {
        travelling_fighter_rows(
            states.sources(&world),
            SquadronPlace::Fleet(fleet),
            class,
            arrival,
        )
            .unwrap()[0]
            .value
            .clone()
    };
    // The first takes Large (3 left), the second Small (2 left against 2).
    assert_eq!(attached(20), world.ship_name(fleet, 1).unwrap());
    assert_eq!(attached(21), world.ship_name(fleet, 0).unwrap());
}

#[test]
fn a_travelling_regiment_is_attached_to_its_destination() {
    // FUN_00445280 with FUN_00556390: an object in hyperspace is
    // attached to its destination; 34628 "Enroute" and the ETA.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let there = system(&mut world, "Bespin", ControlKind::Uncontrolled);
    let troop = world.troops.insert(TroopUnit {
        class_dat_id: DatId::new(0x1000_0002),
        is_alliance: true,
        regiment_strength: 100,
    });
    let mut states = States::new();
    states.transport = with_entry(
        &states.transport,
        "transit",
        rebellion_core::troop_transport::RegimentTransit {
            troop,
            origin: here,
            leg: RegimentLeg::Surface(there),
            arrival_tick: 20,
        },
    );

    let rows = troop_rows(states.sources(&world), troop).unwrap();

    assert_eq!(
        super::tests::rows(&rows[..3]),
        vec![
            ("Attached: ", "Bespin"),
            ("Status:", "Enroute"),
            ("ETA Destination:", "Day 20"),
        ]
    );
}

#[test]
fn a_special_force_on_its_way_is_attached_to_the_target() {
    // FUN_00445780: Attached names the transit's destination, then
    // "Enroute" and the ETA.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let there = system(&mut world, "Bespin", ControlKind::Uncontrolled);
    let unit = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: true,
        skills: [0; 8],
        on_mission: true,
        on_hidden_mission: false,
    });
    world.systems[here].special_forces.push(unit);
    let mut states = States::new();
    states.missions = with_entry(
        &states.missions,
        "en_route",
        rebellion_core::missions::MemberTransit {
            member: MissionMember::SpecialForce(unit),
            mission_id: 1,
            to: there,
            arrival: 15,
            abode: None,
        },
    );

    let rows = special_force_rows(states.sources(&world), unit).unwrap();

    assert_eq!(
        super::tests::rows(&rows[..3]),
        vec![
            ("Attached: ", "Bespin"),
            ("Status:", "Enroute"),
            ("ETA Destination:", "Day 15"),
        ]
    );
    // FUN_0042c3b0: its picture carries STRATEGY 11520 en route.
    let view = status_view(states.sources(&world), StatusObject::SpecialForce(unit)).unwrap();
    assert_eq!(view.marks, [(DllSource::Strategy, 11_520)]);
}

#[test]
fn a_member_on_its_way_aboard_a_fleet_is_attached_to_its_first_ship() {
    // FUN_00556390 reparents a member into its Move's target container
    // (+0x74) at once. For a Move onto a fleet, FUN_00552300 seats it on
    // the fleet's first usable ship (characters and special forces have
    // unlimited room), so FUN_004486f0 and FUN_00445780 name that ship
    // under Attached (FUN_0044a2e0, FUN_004f62d0) while the member
    // travels, then "Enroute" and the ETA (FUN_004fd2b0). A0
    // 2026-11-14-quadrants-status-known/32: Chewbacca on his way into
    // Fleet 7 reads "Attached: Alliance Escort Carrier 1".
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let there = system(&mut world, "Bespin", ControlKind::Controlled(Faction::Alliance));
    let fleet = carrier_fleet(&mut world, there);
    // The special force boards another fleet there.
    let other = carrier_fleet(&mut world, there);
    world.number_ships();
    let name = world.ship_name(fleet, 0).unwrap().to_owned();
    let other_name = world.ship_name(other, 0).unwrap().to_owned();
    assert_eq!((name.as_str(), other_name.as_str()), ("Small 1", "Small 3"));
    let character = world
        .characters
        .insert(rebellion_core::world::Character::default());
    let unit = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: true,
        skills: [0; 8],
        on_mission: true,
        on_hidden_mission: false,
    });
    world.systems[here].special_forces.push(unit);
    let mut states = States::new();
    // Each member's own Move, whose target container is its fleet.
    for (member, arrival, fleet) in [
        (MissionMember::Character(character), 15, fleet),
        (MissionMember::SpecialForce(unit), 16, other),
    ] {
        let id = states.missions.dispatch(MissionRequest::single(
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            character,
            there,
            None,
            0,
        ));
        states.missions.set_target_fleet(id, fleet);
        states.missions.push_transit(rebellion_core::missions::MemberTransit {
            member,
            mission_id: id,
            to: there,
            arrival,
            abode: None,
        });
    }

    let view = status_view(states.sources(&world), StatusObject::Character(character)).unwrap();
    assert_eq!(
        super::tests::rows(&view.rows[1..4]),
        vec![
            ("Attached: ", name.as_str()),
            ("Status:", "Enroute"),
            ("ETA Destination:", "Day 15"),
        ]
    );
    let rows = special_force_rows(states.sources(&world), unit).unwrap();
    assert_eq!(
        super::tests::rows(&rows[..3]),
        vec![
            ("Attached: ", other_name.as_str()),
            ("Status:", "Enroute"),
            ("ETA Destination:", "Day 16"),
        ]
    );
}

#[test]
fn a_missions_eta_is_its_first_members_transit_for_that_mission() {
    // FUN_00445c60 reads the ETA of the mission's first member.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Bespin", ControlKind::Uncontrolled);
    let mut states = States::new();
    let a = world.characters.insert(rebellion_core::world::Character::default());
    let id = states.missions.dispatch(MissionRequest::single(
        MissionKind::Diplomacy,
        MissionFaction::Alliance,
        a,
        here,
        None,
        0,
    ));
    let transit = |mission_id, arrival| rebellion_core::missions::MemberTransit {
        member: MissionMember::Character(a),
        mission_id,
        to: here,
        arrival,
        abode: None,
    };
    states.missions = with_entry(&states.missions, "en_route", transit(id + 1, 50));
    states.missions = with_entry(&states.missions, "en_route", transit(id, 30));

    let rows = mission_rows(states.sources(&world), id).unwrap();

    assert_eq!(rows[1], row("ETA Destination:", "Day 30"));
}

#[test]
fn a_producer_without_yards_lists_no_queue() {
    // FUN_0044acd0 writes the items and day only for a manager with
    // facilities.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let mut states = States::new();
    states.manufacturing.enqueue(
        here,
        QueueItem::new(BuildableKind::Troop(DatId::new(0x1000_0002)), 9),
    );

    let rows =
        producer_rows(states.sources(&world), here, ProductionArea::TrainingFacility).unwrap();

    assert_eq!(
        super::tests::rows(&rows),
        vec![("Location:", "Hoth"), ("Status:", "No Facilities")]
    );
}

#[test]
fn a_production_facility_reads_its_rate_and_bombardment() {
    // FUN_00446c50: Location, Status, maintenance, processing rate,
    // bombardment; a neutral facility takes the "other" background.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Kessel", ControlKind::Uncontrolled);
    let class = catalog(
        &mut world,
        0x2c00_0001,
        "Mine",
        BuildableClass {
            maintenance_cost: 1,
            processing_rate: 3,
            stats: ClassStats {
                bombardment: 8,
                ..ClassStats::default()
            },
            ..BuildableClass::default()
        },
    );
    let mine = world
        .production_facilities
        .insert(rebellion_core::world::ProductionFacilityInstance {
            class_dat_id: class,
            side: Faction::Neutral,
            is_mine: true,
        });
    world.systems[here].production_facilities.push(mine);
    let states = States::new();

    let view = status_view(
        states.sources(&world),
        StatusObject::ProductionFacility(mine),
    )
    .unwrap();

    assert_eq!(view.name, "Mine");
    assert_eq!(view.side, 2);
    assert_eq!(
        super::tests::rows(&view.rows),
        vec![
            ("Location:", "Kessel"),
            ("Status:", "Active"),
            ("Maintenance Cost:", "1"),
            ("Standard Processing Rate:", "3"),
            ("Bombardment Value:", "8"),
        ]
    );
}

#[test]
fn an_alliance_defense_takes_the_side_one_background_and_its_class_name() {
    // FUN_00443130 background by side; FUN_0044a210 the class's name.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Hoth", ControlKind::Controlled(Faction::Alliance));
    let class = catalog(&mut world, 0x2200_0001, "Planetary Shield", BuildableClass::default());
    let shield = world.defense_facilities.insert(DefenseFacilityInstance {
        class_dat_id: class,
        side: Faction::Alliance,
    });
    world.systems[here].defense_facilities.push(shield);
    let states = States::new();

    let view =
        status_view(states.sources(&world), StatusObject::DefenseFacility(shield)).unwrap();

    assert_eq!(view.title, "Defense Facility Status");
    assert_eq!(view.name, "Planetary Shield");
    assert_eq!(view.side, 0);
    assert_eq!(view.rows[0], row("Location:", "Hoth"));
}

#[test]
fn a_producer_takes_its_systems_holder_for_the_background() {
    // FUN_00443130: a manager's side bits are its system's.
    let mut world = GameWorld::default();
    let empire = system(&mut world, "Kuat", ControlKind::Controlled(Faction::Empire));
    let neutral = system(&mut world, "Kessel", ControlKind::Uncontrolled);
    let states = States::new();
    let side = |system| {
        status_view(
            states.sources(&world),
            StatusObject::Producer {
                system,
                area: ProductionArea::Shipyard,
            },
        )
        .unwrap()
        .side
    };

    assert_eq!(side(empire), 1);
    assert_eq!(side(neutral), 2);
}

#[test]
fn an_empire_missions_picture_is_offset_by_0x1000() {
    // FUN_00443130: GOKRES class +0x30 & 0xfff, plus 0x1000 unless the
    // side bits are 1.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Bespin", ControlKind::Uncontrolled);
    let mut states = States::new();
    let a = world.characters.insert(rebellion_core::world::Character::default());
    let request = |faction| {
        MissionRequest::single(MissionKind::Diplomacy, faction, a, here, None, 0)
    };
    let alliance = states.missions.dispatch(request(MissionFaction::Alliance));
    let empire = states.missions.dispatch(request(MissionFaction::Empire));
    let sources = states.sources(&world);
    let view = |id| status_view(sources, StatusObject::Mission(id)).unwrap();

    let (theirs, ours) = (view(empire), view(alliance));
    assert_eq!(theirs.title, "Mission Status");
    assert_eq!((ours.side, theirs.side), (0, 1));
    let Some((DllSource::Gokres, base)) = ours.picture else {
        panic!("a mission has a GOKRES picture");
    };
    assert_eq!(theirs.picture, Some((DllSource::Gokres, base + 0x1000)));
}

#[test]
fn the_galaxy_overview_groups_the_sides_objects_by_class_in_family_order() {
    // FUN_0044a7a0 groups the viewer's objects by class with their count
    // and summed maintenance; FUN_0044aa20 rows them with the list mini,
    // "%03d" + "  " (DAT_006a874c) + "%04d", keyed by the family.
    let mut world = GameWorld::default();
    let here = system(&mut world, "Yavin", ControlKind::Controlled(Faction::Alliance));
    // Two ships of one class (maintenance 6), four squadrons (2).
    carrier_fleet(&mut world, here);
    let x_wing = world.fighter_classes.keys().next().unwrap();
    // A planet's squadron takes its class's side.
    world.fighter_classes[x_wing].is_alliance = true;
    world.systems[here].fighters.push(FighterEntry {
        class: x_wing,
        count: 1,
        carrier: None,
        damage: None,
    });
    let class = |world: &mut GameWorld, id, maintenance_cost| {
        catalog(world, id, "Class", BuildableClass {
            maintenance_cost,
            ..BuildableClass::default()
        })
    };
    let army = class(&mut world, 0x1000_0002, 1);
    let fleet_regiment = class(&mut world, 0x1000_0001, 3);
    let stormtroopers = class(&mut world, 0x1000_0006, 1);
    for (class_dat_id, is_alliance) in
        [(army, true), (fleet_regiment, true), (army, true), (stormtroopers, false)]
    {
        world.troops.insert(TroopUnit {
            class_dat_id,
            is_alliance,
            regiment_strength: 100,
        });
    }
    // The Alliance's KDY-150 (family 0x22), the Empire's LNR (0x23).
    let kdy = class(&mut world, 0x2200_0001, 4);
    let lnr = class(&mut world, 0x2300_0002, 3);
    for (class_dat_id, side) in [(kdy, Faction::Alliance), (lnr, Faction::Empire)] {
        world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id,
            side,
        });
    }
    let yard = class(&mut world, 0x2800_0001, 5);
    world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id: yard,
            side: Faction::Alliance,
            is_shipyard: true,
        });
    let mine = class(&mut world, 0x2c00_0001, 0);
    world
        .production_facilities
        .insert(rebellion_core::world::ProductionFacilityInstance {
            class_dat_id: mine,
            side: Faction::Alliance,
            is_mine: true,
        });
    let guerrillas = class(&mut world, 0x3c00_0001, 2);
    world.special_forces.insert(SpecialForceUnit {
        class_dat_id: guerrillas,
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
        on_hidden_mission: false,
    });

    let rows: Vec<_> = overview_rows(&world, true)
        .into_iter()
        .map(|row| (row.source, row.icon, row.text))
        .collect();
    let gokres = |icon: u32, text: &str| (DllSource::Gokres, icon, text.to_owned());
    assert_eq!(
        rows,
        [
            gokres(17_472, "001  0003"),
            gokres(17_473, "002  0002"),
            gokres(capital_ship_mini_id(DatId::new(0x1400_0040)).unwrap(), "002  0012"),
            gokres(fighter_mini_id(DatId::new(0x1c00_0001)).unwrap(), "005  0010"),
            gokres(16_896, "001  0004"),
            gokres(16_640, "001  0005"),
            gokres(16_385, "001  0000"),
            gokres(17_728, "001  0002"),
        ]
    );
    // The Empire sees its own regiment only.
    let empire: Vec<_> = overview_rows(&world, false)
        .into_iter()
        .map(|row| row.text)
        .collect();
    assert_eq!(empire, ["001  0001", "001  0003"]);
}

#[test]
fn the_galaxy_overview_window_names_the_galaxy_and_lays_out_minis() {
    // FUN_00443130 for 0xf1: STRATEGY 11590, TEXTSTRA 12563, no
    // Encyclopedia, the list's own format.
    let world = GameWorld::default();
    let states = States::new();
    let view = status_view(states.sources(&world), StatusObject::GalaxyOverview).unwrap();
    assert_eq!(view.name, "Galaxy Overview");
    assert_eq!(view.picture, Some((DllSource::Strategy, 11_590)));
    assert_eq!(view.icon_layout, IconRowLayout::Overview);
    assert!(!view.encyclopedia);
    // The galaxy's background: 11554 for a side-1 viewer, else 11558.
    let mut sources = states.sources(&world);
    for (alliance, faction, background) in [
        (true, crate::cockpit::CockpitFaction::Alliance, 11_554),
        (false, crate::cockpit::CockpitFaction::Empire, 11_558),
    ] {
        sources.objectives.alliance = alliance;
        let view = status_view(sources, StatusObject::GalaxyOverview).unwrap();
        assert_eq!(crate::status_window::background(view.side, faction), background);
    }
}

fn texts(source: ObjectiveSource) -> Vec<String> {
    objective_rows(source).into_iter().map(|row| row.text).collect()
}

#[test]
fn each_side_lists_its_own_objectives_first() {
    // FUN_0044b2d0 keys Coruscant, Emperor, Vader, HQ, Luke, Mon Mothma
    // 1..6; FUN_0044bb70 keys them 4, 5, 6, 1, 2, 3. A0 2026-10-29 shows
    // the Empire's order.
    let alliance = ObjectiveSource {
        alliance: true,
        conditions_word: 0,
        standard: true,
    };
    assert_eq!(
        texts(alliance),
        [
            "Control Coruscant",
            "Capture Palpatine",
            "Capture Vader",
            "Defend Headquarters",
            "Defend Luke",
            "Defend Mon Mothma",
        ]
    );
    assert_eq!(
        texts(ObjectiveSource {
            alliance: false,
            ..alliance
        }),
        [
            "Destroy Rebel HQ",
            "Capture Skywalker",
            "Capture Mothma",
            "Defend Coruscant",
            "Defend the Emperor",
            "Defend Vader",
        ]
    );
    let icons: Vec<_> = objective_rows(alliance).iter().map(|row| row.icon).collect();
    assert_eq!(icons, [10_234, 11_013, 11_055, 904, 11_028, 11_032]);
}

#[test]
fn a_met_condition_changes_its_text_on_both_sides() {
    // FUN_0044b2d0 / FUN_0044bb70 test the word's bits 0x300, 0x2000,
    // 0x1000, 1, 0x10 and 0x20.
    let word = 0x200 | 0x2000 | 0x1 | 0x20;
    assert_eq!(
        texts(ObjectiveSource {
            alliance: true,
            conditions_word: word,
            standard: true,
        }),
        [
            "Coruscant Taken",
            "Emperor Captive",
            "Capture Vader",
            "Headquarters Lost",
            "Defend Luke",
            "Mon Mothma Captured",
        ]
    );
    assert_eq!(
        texts(ObjectiveSource {
            alliance: false,
            conditions_word: 0x100 | 0x1000 | 0x10,
            standard: true,
        }),
        [
            "Destroy Rebel HQ",
            "Skywalker Captive",
            "Capture Mothma",
            "Coruscant Lost",
            "Defend the Emperor",
            "Vader Captured",
        ]
    );
}

#[test]
fn a_headquarters_only_game_lists_no_leaders() {
    // The leader rows sit under `local_30` (game settings +8 == 0).
    let source = |alliance| ObjectiveSource {
        alliance,
        conditions_word: 0,
        standard: false,
    };
    assert_eq!(texts(source(true)), ["Control Coruscant", "Defend Headquarters"]);
    assert_eq!(texts(source(false)), ["Destroy Rebel HQ", "Defend Coruscant"]);
}

#[test]
fn the_objectives_window_shows_the_galaxy_picture_and_no_encyclopedia() {
    // FUN_00443130 on the empty id: STRATEGY 11590, TEXTSTRA 12564, the
    // background row of a side-1 viewer (11554) or another (11558), and
    // FUN_00443020 disables 0x66.
    let world = GameWorld::default();
    let states = States::new();
    let mut sources = states.sources(&world);
    sources.objectives = ObjectiveSource {
        alliance: false,
        conditions_word: 0,
        standard: true,
    };
    let view = status_view(sources, StatusObject::Objectives).unwrap();
    assert_eq!(view.picture, Some((DllSource::Strategy, 11_590)));
    assert_eq!(view.name, "Objectives");
    assert_eq!(view.title, "");
    assert!(!view.encyclopedia);
    assert!(view.rows.is_empty());
    assert_eq!(view.icon_rows.len(), 6);
    assert_eq!(
        crate::status_window::background(view.side, crate::cockpit::CockpitFaction::Empire),
        11_558
    );
    sources.objectives.alliance = true;
    let view = status_view(sources, StatusObject::Objectives).unwrap();
    assert_eq!(
        crate::status_window::background(view.side, crate::cockpit::CockpitFaction::Alliance),
        11_554
    );
}
