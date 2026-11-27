use super::*;
use rebellion_core::dat::ExplorationStatus;

#[test]
fn each_tab_has_its_tooltip_string() {
    // FUN_004a4b10 (A0 tooltip/142..145).
    assert_eq!(
        super::FleetWindowTab::ALL.map(super::FleetWindowTab::tooltip_string),
        [0x3823, 0x3820, 0x3821, 0x3822]
    );
}
use crate::cockpit::CockpitViewport;
use rebellion_core::dat::SectorGroup;
use rebellion_core::ids::{DatId, TroopKey};
use rebellion_core::world::{Fleet, Sector, System, TroopUnit};

/// The units of `window`'s right list `rows`.
fn keys_of(world: &GameWorld, window: &OpenFleetWindow, rows: &[usize]) -> Vec<RightKey> {
    let keys = right_keys(&right_items(
        world,
        &TroopTransportState::default(),
        &[],
        window.selected.single(),
        window.tab,
    ));
    rows.iter().map(|&row| keys[row].unwrap()).collect()
}

fn layout() -> CockpitLayout {
    let viewport = CockpitViewport {
        x: 0.0,
        y: 0.0,
        width: 640.0,
        height: 480.0,
    };
    CockpitLayout {
        canvas: viewport,
        galaxy: viewport,
        scale: 1.0,
    }
}

fn world(control: ControlKind) -> (GameWorld, SystemKey) {
    let mut world = GameWorld::default();
    let sector = world.sectors.insert(Sector {
        dat_id: DatId::new(36),
        name: "Sesswenna".into(),
        group: SectorGroup::Core,
        x: 317,
        y: 248,
        systems: Vec::new(),
    });
    let system = world.systems.insert(System {
        dat_id: DatId::new(100),
        name: "Sluis Van".into(),
        sector,
        x: 320,
        y: 250,
        exploration_status: ExplorationStatus::Explored,
        popularity_alliance: 0.5,
        popularity_empire: 0.5,
        is_populated: true,
        control,
        ..Default::default()
    });
    (world, system)
}

fn add_fleet(
    world: &mut GameWorld,
    system: SystemKey,
    is_alliance: bool,
    troops: u32,
) -> FleetKey {
    let class = world.capital_ship_classes.insert(CapitalShipClass {
        name: "Transport".into(),
        is_alliance,
        troop_capacity: troops,
        hull: 100,
        hyperdrive: 1,
        ..CapitalShipClass::default()
    });
    let fleet = world.insert_fleet(Fleet {
        location: system,
        capital_ships: vec![ShipInstance::new(class, 100, true)],
        is_alliance,
        ..Default::default()
    });
    world.systems[system].fleets.push(fleet);
    fleet
}

fn add_troop(world: &mut GameWorld, system: SystemKey) -> TroopKey {
    let troop = world.troops.insert(TroopUnit {
        class_dat_id: DatId::new(0x1000_0001),
        is_alliance: true,
        regiment_strength: 100,
    });
    world.systems[system].ground_units.push(troop);
    troop
}

/// Seen by the Alliance: the system is visible.
fn fog(system: SystemKey) -> FogState {
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(system);
    fog
}

fn window(system: SystemKey, selected: Option<FleetWindowEntry>) -> OpenFleetWindow {
    OpenFleetWindow {
        system,
        logical_position: (0, 0),
        selected: selected.map_or_else(ListSelection::default, ListSelection::only),
        selected_items: ListSelection::default(),
        expanded: Vec::new(),
        tab: FleetWindowTab::CapitalShips,
        rename: None,
        last_click: None,
        moving: None,
        roster: Vec::new(),
        right_scroll: 0.0,
        right_hold: None,
        left_scroll: 0.0,
        left_hold: None,
    }
}

#[test]
fn the_fleet_icon_shows_the_players_side_first_and_hides_without_fleets() {
    // FUN_0045ccc0 picks the player's side, then the other; FUN_0045d140
    // hides the overlay at a zero count; FUN_0045ca80 kind 0x10.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    crate::viewed_contents::see_live(&mut world, Faction::Alliance, system);
    let fog = fog(system);
    assert_eq!(
        fleet_icon(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog,
            Faction::Alliance,
            system
        ),
        None
    );

    add_fleet(&mut world, system, false, 0);
    assert_eq!(
        fleet_icon(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog,
            Faction::Alliance,
            system
        ),
        Some((10783, 10784))
    );
    add_fleet(&mut world, system, true, 0);
    assert_eq!(
        fleet_icon(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog,
            Faction::Alliance,
            system
        ),
        Some((10775, 10776))
    );
    assert_eq!(
        fleet_icon(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog,
            Faction::Empire,
            system
        ),
        Some((10783, 10784))
    );
}

#[test]
fn an_unseen_enemy_fleet_shows_no_icon() {
    // port: the count is the fleets the player can see.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    add_fleet(&mut world, system, false, 0);
    let unseen = FogState::new(Faction::Alliance);

    assert_eq!(
        fleet_icon(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &unseen,
            Faction::Alliance,
            system
        ),
        None
    );
}

#[test]
fn the_rail_icon_falls_back_to_the_systems_side() {
    // FUN_004a76e0: 0x2d10, 0x2d11, else 0x2d12.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    let fog = fog(system);
    assert_eq!(
        rail_icon(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog,
            Faction::Alliance,
            system
        ),
        11537
    );
    world.systems[system].control = ControlKind::Contested;
    assert_eq!(
        rail_icon(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog,
            Faction::Alliance,
            system
        ),
        11538
    );
    add_fleet(&mut world, system, true, 0);
    assert_eq!(
        rail_icon(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog,
            Faction::Alliance,
            system
        ),
        11536
    );
}

#[test]
fn the_title_strip_follows_the_players_fleets_then_the_system() {
    // FUN_004a3340: with fleets, the player's side when it has one there,
    // else the other; without, the system's side bits. Side 1 is
    // 10299/10200, side 2 10201/10302, other 10303/10304.
    let (mut world, system) = world(ControlKind::Uncontrolled);
    let player = Faction::Alliance;
    assert_eq!(title_side(&world, &[], player, system), 0);
    let enemy = add_fleet(&mut world, system, false, 0);
    assert_eq!(title_side(&world, &[enemy], player, system), 2);
    let own = add_fleet(&mut world, system, true, 0);
    assert_eq!(title_side(&world, &[enemy, own], player, system), 1);
    assert_eq!(title_side(&world, &[own], Faction::Empire, system), 1);

    assert_eq!(
        [title_resource(1, true), title_resource(1, false)],
        [10299, 10200]
    );
    assert_eq!(
        [title_resource(2, true), title_resource(2, false)],
        [10201, 10302]
    );
    assert_eq!(
        [title_resource(0, true), title_resource(3, false)],
        [10303, 10304]
    );
}

#[test]
fn tab_art_is_normal_disabled_or_selected_from_the_strips_base() {
    // FUN_004a4b10: b + k, b + 3 + k, b + 7 + k; the Empire's base is 50
    // higher.
    assert_eq!(
        tab_resource(10409, FleetWindowTab::CapitalShips, false, true),
        10409
    );
    assert_eq!(
        tab_resource(10409, FleetWindowTab::CapitalShips, true, true),
        10416
    );
    assert_eq!(
        tab_resource(10409, FleetWindowTab::Troops, false, false),
        10414
    );
    assert_eq!(
        tab_resource(10459, FleetWindowTab::Personnel, true, true),
        10469
    );
    assert_eq!(side_art(TAB_BASE, 2), 10459);
}

#[test]
fn a_selected_ship_lists_and_counts_what_it_carries() {
    // A0 fleet-aboard/10..12: Imperial Star Destroyer 1 selected lists
    // its TIE Fighter squadrons (6 and 6), its regiments (3 and 3) and
    // Veers; a ship's own slots (FUN_004a5c00, +0x23c/+0x26c,
    // +0x240/+0x270). Two ships here: the squadrons and regiments fill
    // the first ship's capacity, the rest go to the second.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 2);
    let class = world.fleets[fleet].capital_ships[0].class;
    world.capital_ship_classes[class].fighter_capacity = 2;
    let second = world.fleets[fleet].capital_ships[0].clone();
    world.fleets[fleet].capital_ships.push(second);
    world.number_ships();
    let fighter = world.fighter_classes.insert(rebellion_core::world::FighterClass {
        name: "X-wing".into(),
        is_alliance: true,
        ..Default::default()
    });
    world.fleets[fleet].fighters.push(rebellion_core::world::FighterEntry {
        class: fighter,
        count: 3,
        carrier: None,
        damage: None,
    });
    let troops: Vec<_> = (0..3).map(|_| add_troop(&mut world, system)).collect();
    let mut transport = TroopTransportState::default();
    transport.embark(&mut world, fleet, &troops).unwrap();
    let ship = |index| Some(FleetWindowEntry::Ship { fleet, index });
    let labels = |index, tab| {
        right_items(&world, &transport, &[], ship(index), tab)
            .into_iter()
            .map(|item| item.label)
            .collect::<Vec<_>>()
    };

    assert_eq!(labels(0, FleetWindowTab::Fighters), ["X-wing", "X-wing"]);
    assert_eq!(labels(1, FleetWindowTab::Fighters), ["X-wing"]);
    assert_eq!(tab_counts(&world, &transport, ship(0), FleetWindowTab::Fighters), Some((2, 2)));
    assert_eq!(tab_counts(&world, &transport, ship(1), FleetWindowTab::Fighters), Some((1, 2)));
    assert_eq!(labels(0, FleetWindowTab::Troops).len(), 2);
    assert_eq!(labels(1, FleetWindowTab::Troops).len(), 1);
    assert_eq!(tab_counts(&world, &transport, ship(0), FleetWindowTab::Troops), Some((2, 2)));
    assert!(labels(0, FleetWindowTab::CapitalShips).is_empty());

    // A character lists under the ship it is seated on; a special force
    // under the first ship.
    let seat = world.fleets[fleet].capital_ships[1].id;
    let officer = world.characters.insert(rebellion_core::world::Character {
        name: "Veers".into(),
        ship: Some(seat),
        ..Default::default()
    });
    world.fleets[fleet].characters.push(officer);
    let unit = world.special_forces.insert(rebellion_core::world::SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0002),
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
        on_hidden_mission: false,
    });
    world.fleets[fleet].special_forces.push(unit);
    let objects = |index| {
        right_items(&world, &transport, &[], ship(index), FleetWindowTab::Personnel)
            .into_iter()
            .map(|item| item.object)
            .collect::<Vec<_>>()
    };
    assert_eq!(objects(1), [Some(ItemObject::Character(officer))]);
    assert_eq!(objects(0), [Some(ItemObject::SpecialForce(unit))]);
}

#[test]
fn a_killed_or_other_sides_member_bound_for_a_fleet_is_not_aboard() {
    // The walk's mode 3 takes existing children only (+0x50 bit 6,
    // FUN_004f6b90), and FUN_00553410 refuses a Move into another
    // side's container (1/0x24, 1/0x28): a killed traveller or one of
    // the other side neither lists on the Personnel tab nor lights it.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 2);
    let traveller = |world: &mut GameWorld, is_alliance, is_empire, is_killed| {
        world.characters.insert(rebellion_core::world::Character {
            is_alliance,
            is_empire,
            is_killed,
            ..Default::default()
        })
    };
    let killed = traveller(&mut world, true, false, true);
    let imperial = traveller(&mut world, false, true, false);
    let unit = world.special_forces.insert(rebellion_core::world::SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: false,
        skills: [0; 8],
        on_mission: true,
        on_hidden_mission: false,
    });
    let arrivals = [
        (MissionMember::Character(killed), fleet),
        (MissionMember::Character(imperial), fleet),
        (MissionMember::SpecialForce(unit), fleet),
    ];
    let transport = TroopTransportState::default();
    let selected = Some(FleetWindowEntry::Fleet(fleet));
    assert!(!tab_enabled(&world, &transport, &arrivals, selected, FleetWindowTab::Personnel));
    assert!(right_items(&world, &transport, &arrivals, selected, FleetWindowTab::Personnel).is_empty());
    assert!(travelling_member_item(&world, MissionMember::Character(killed)).is_none());
    // A living member of the fleet's side is aboard.
    let own = traveller(&mut world, true, false, false);
    let arrivals = [(MissionMember::Character(own), fleet)];
    assert!(tab_enabled(&world, &transport, &arrivals, selected, FleetWindowTab::Personnel));
}

#[test]
fn units_on_their_way_aboard_are_listed_with_the_en_route_mark_and_light_their_tabs() {
    // FUN_00556390 puts a regiment, squadron or member sent by Move into
    // its target container at once, in hyperspace (+0x50 bit 4;
    // regiment-unload.md), so the fleet's tabs list it after its own and
    // FUN_0042c3b0 marks it en route although the fleet is in orbit.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 2);
    let other = add_fleet(&mut world, system, true, 2);
    // A third fleet with only an empty transit on its way.
    let third = add_fleet(&mut world, system, true, 2);
    let troop = add_troop(&mut world, system);
    let xwing = world
        .fighter_classes
        .insert(rebellion_core::world::FighterClass {
            dat_id: DatId::new(0x1c00_0003),
            name: "X-wing".into(),
            is_alliance: true,
            ..Default::default()
        });
    let character = world.characters.insert(rebellion_core::world::Character {
        name: "Wedge".into(),
        // The fleet's side: FUN_00553410 refuses another's.
        is_alliance: true,
        ..Default::default()
    });
    let selected = Some(FleetWindowEntry::Fleet(fleet));
    let empty = TroopTransportState::default();
    for tab in [
        FleetWindowTab::Fighters,
        FleetWindowTab::Troops,
        FleetWindowTab::Personnel,
    ] {
        assert!(!tab_enabled(&world, &empty, &[], selected, tab), "{tab:?}");
    }

    let mut value = serde_json::to_value(&empty).unwrap();
    value["transit"].as_array_mut().unwrap().push(
        serde_json::to_value(rebellion_core::troop_transport::RegimentTransit {
            troop,
            origin: system,
            leg: RegimentLeg::Fleet(fleet),
            arrival_tick: 9,
        })
        .unwrap(),
    );
    for (to, count) in [(fleet, 1), (other, 1), (third, 0)] {
        value["squadrons"]["transits"].as_array_mut().unwrap().push(
            serde_json::to_value(rebellion_core::squadrons::SquadronTransit {
                class: xwing,
                count,
                origin: system,
                to: SquadronPlace::Fleet(to),
                arrival_tick: 9,
                lost: Vec::new(),
            })
            .unwrap(),
        );
    }
    let transport: TroopTransportState = serde_json::from_value(value).unwrap();
    let arrivals = [(MissionMember::Character(character), fleet)];
    for tab in [
        FleetWindowTab::Fighters,
        FleetWindowTab::Troops,
        FleetWindowTab::Personnel,
    ] {
        assert!(
            tab_enabled(&world, &transport, &arrivals, selected, tab),
            "{tab:?}"
        );
    }
    let listed = |tab| right_items(&world, &transport, &arrivals, selected, tab);
    let [regiment] = &listed(FleetWindowTab::Troops)[..] else {
        panic!("{:?}", listed(FleetWindowTab::Troops));
    };
    assert_eq!(regiment.object, Some(ItemObject::Regiment(troop)));
    assert!(regiment.en_route);
    let [squadron] = &listed(FleetWindowTab::Fighters)[..] else {
        panic!("{:?}", listed(FleetWindowTab::Fighters));
    };
    let object = squadron.object.unwrap();
    assert_eq!(
        object.menu_object(),
        MenuObject::TravellingFighter {
            to: SquadronPlace::Fleet(fleet),
            class: xwing,
            arrival: 9,
        }
    );
    assert!(squadron.en_route);
    // FUN_004a6be0 prints the ships' +0x23c / +0x240, counts by the
    // list's walk (mode 3, CapShip_vt23c / CapShip_vt240), so the
    // travellers count as aboard: the fleet's one Transport (troop
    // capacity 2, no hangar) reads 1 of 2 regiments, 1 of 0 squadrons.
    assert_eq!(tab_counts(&world, &transport, selected, FleetWindowTab::Troops), Some((1, 2)));
    assert_eq!(tab_counts(&world, &transport, selected, FleetWindowTab::Fighters), Some((1, 0)));
    let [member] = &listed(FleetWindowTab::Personnel)[..] else {
        panic!("{:?}", listed(FleetWindowTab::Personnel));
    };
    assert_eq!(member.object, Some(ItemObject::Character(character)));
    assert!(member.en_route);
    // FUN_004f7800: an injured character is damaged.
    assert!(!member.damaged);
    let mut hurt = world.clone();
    hurt.characters[character].injury = 3;
    assert!(travelling_member_item(&hurt, MissionMember::Character(character)).unwrap().damaged);
    // The other fleet lists only its own squadron, and no member.
    let theirs = Some(FleetWindowEntry::Fleet(other));
    assert_eq!(
        right_items(
            &world,
            &transport,
            &arrivals,
            theirs,
            FleetWindowTab::Fighters
        )
        .len(),
        1
    );
    assert!(!tab_enabled(
        &world,
        &transport,
        &arrivals,
        theirs,
        FleetWindowTab::Personnel
    ));
    assert!(!tab_enabled(
        &world,
        &transport,
        &arrivals,
        theirs,
        FleetWindowTab::Troops
    ));
    // An empty squadron transit lights nothing and lists nothing.
    let last = Some(FleetWindowEntry::Fleet(third));
    assert!(!tab_enabled(
        &world,
        &transport,
        &arrivals,
        last,
        FleetWindowTab::Fighters
    ));
    assert!(right_items(&world, &transport, &arrivals, last, FleetWindowTab::Fighters).is_empty());
    // The window reads the arrivals the frame hands it.
    let mut state = opened(&world, system);
    assert!(state.select(system, FleetWindowEntry::Fleet(fleet)));
    let personnel = |state: &FleetWindowState| {
        state
            .report(
                &world,
                &rebellion_core::movement::MovementState::default(),
                &fog(system),
                &transport,
                system,
            )
            .unwrap()
            .enabled[3]
    };
    assert!(!personnel(&state));
    state.set_arrivals(arrivals.to_vec());
    assert!(personnel(&state));
}

#[test]
fn the_troops_tab_lights_only_with_regiments_aboard_and_counts_them() {
    // FUN_004a5c00: flag 2 enables the tab; the panel prints the
    // regiments aboard and the capacity (+0x240, +0x270).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 2);
    let troop = add_troop(&mut world, system);
    let mut transport = TroopTransportState::default();
    let selected = Some(FleetWindowEntry::Fleet(fleet));
    assert!(!tab_enabled(
        &world,
        &transport,
        &[],
        selected,
        FleetWindowTab::Troops
    ));
    assert!(tab_enabled(
        &world,
        &transport,
        &[],
        selected,
        FleetWindowTab::CapitalShips
    ));

    transport.embark(&mut world, fleet, &[troop]).unwrap();

    assert!(tab_enabled(
        &world,
        &transport,
        &[],
        selected,
        FleetWindowTab::Troops
    ));
    assert_eq!(
        tab_counts(&world, &transport, selected, FleetWindowTab::Troops),
        Some((1, 2))
    );
    assert_eq!(
        right_items(&world, &transport, &[], selected, FleetWindowTab::Troops),
        [RightItem {
            mini: Some(17_472),
            kind: MiniObject::Regiment(DatId::new(0x1000_0001)),
            label: "Alliance Fleet Regiment".into(),
            no_hyperdrive: false,
            contents: 0,
            damaged: false,
            captive: false,
            en_route: false,
            object: Some(ItemObject::Regiment(troop)),
        }]
    );
    // A ship's own contents are not modelled, so its tabs stay dark.
    let ship = Some(FleetWindowEntry::Ship { fleet, index: 0 });
    assert!(!tab_enabled(
        &world,
        &transport,
        &[],
        ship,
        FleetWindowTab::CapitalShips
    ));
}

#[test]
fn a_unit_seated_on_no_ship_names_the_first_living_ship_else_the_fleet() {
    // port: aboard_label's fallback (FUN_0044a2e0 always names a ship,
    // A0 2026-11-14-quadrants-status-known/21, 32).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let second = world.fleets[fleet].capital_ships[0].clone();
    world.fleets[fleet].capital_ships.push(second);
    world.fleets[fleet].capital_ships[0].alive = false;
    world.fleets[fleet].capital_ships[1].name = Some("Second".into());
    assert_eq!(aboard_label(&world, fleet, None).as_deref(), Some("Second"));
    world.fleets[fleet].capital_ships[1].alive = false;
    assert_eq!(aboard_label(&world, fleet, None), fleet_label(&world, fleet));
}

#[test]
fn the_fleets_left_count_follows_a_regiment_boarding_and_leaving() {
    // FUN_004a5c00 prints the regiments aboard (+0x240) at the panel's
    // left and the capacity (+0x270) at its right. A0
    // 2026-11-14-missions-transfer/70, 74, 78 (Fleet 1 at Drall,
    // capacity 2): 1 -> 2 as an Army Regiment boards, 2 -> 1 as it
    // leaves; the right count stays 2.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 2);
    let fleet_regiment = add_troop(&mut world, system);
    let army = add_troop(&mut world, system);
    let mut transport = TroopTransportState::default();
    transport.embark(&mut world, fleet, &[fleet_regiment]).unwrap();
    let counts = |world: &GameWorld, transport: &TroopTransportState| {
        tab_counts(
            world,
            transport,
            Some(FleetWindowEntry::Fleet(fleet)),
            FleetWindowTab::Troops,
        )
    };
    assert_eq!(counts(&world, &transport), Some((1, 2)));

    transport.embark(&mut world, fleet, &[army]).unwrap();
    assert_eq!(counts(&world, &transport), Some((2, 2)));

    transport.disembark_selected(&mut world, fleet, system, &[army]).unwrap();
    assert_eq!(counts(&world, &transport), Some((1, 2)));
}

#[test]
fn a_drop_on_a_fleet_entry_targets_it_and_between_entries_the_system() {
    // FUN_004a3130 step 2.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let first = add_fleet(&mut world, system, true, 1);
    let second = add_fleet(&mut world, system, false, 1);
    let open = window(system, None);
    let entries = left_entries(
        &world,
        &rebellion_core::movement::MovementState::default(),
        &fog(system),
        Faction::Alliance,
        &open,
    );
    assert_eq!(
        entries,
        [
            FleetWindowEntry::Fleet(first),
            FleetWindowEntry::Fleet(second)
        ]
    );
    let target = |point| drop_target(&world, Faction::Alliance, &open, &entries, point);

    assert_eq!(
        target((10.0, 30.0)),
        Some(ReleaseTarget::Fleet {
            fleet: first,
            system
        })
    );
    assert_eq!(
        target((94.0, 128.0)),
        Some(ReleaseTarget::Fleet {
            fleet: second,
            system
        })
    );
    assert_eq!(target((10.0, 130.0)), Some(ReleaseTarget::System(system)));
}

#[test]
fn a_drop_outside_the_lists_takes_the_selected_own_fleet_or_nothing() {
    // FUN_004a3130 step 1: the selected own-side fleet; none while own
    // fleets are listed but none is selected.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let own = add_fleet(&mut world, system, true, 1);
    let enemy = add_fleet(&mut world, system, false, 1);
    let fog = fog(system);
    let outside = (150.0, 20.0);
    let target = |selected| {
        let open = window(system, selected);
        let entries = left_entries(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog,
            Faction::Alliance,
            &open,
        );
        drop_target(&world, Faction::Alliance, &open, &entries, outside)
    };

    assert_eq!(target(None), None);
    assert_eq!(target(Some(FleetWindowEntry::Fleet(enemy))), None);
    assert_eq!(
        target(Some(FleetWindowEntry::Fleet(own))),
        Some(ReleaseTarget::Fleet { fleet: own, system })
    );
}

#[test]
fn a_drop_with_no_own_fleet_listed_targets_the_system() {
    // FUN_004a3130 step 1: the subject when no own-side entry exists.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    let enemy = add_fleet(&mut world, system, false, 1);
    let open = window(system, Some(FleetWindowEntry::Fleet(enemy)));
    let entries = left_entries(
        &world,
        &rebellion_core::movement::MovementState::default(),
        &fog(system),
        Faction::Alliance,
        &open,
    );

    assert_eq!(
        drop_target(&world, Faction::Alliance, &open, &entries, (150.0, 20.0)),
        Some(ReleaseTarget::System(system))
    );
}

#[test]
fn a_drop_on_a_listed_ship_targets_its_fleet_only_on_the_ships_tab() {
    // FUN_004a3130 step 3. port: a ship stands for its fleet.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let own = add_fleet(&mut world, system, true, 1);
    let mut open = window(system, Some(FleetWindowEntry::Fleet(own)));
    let fog = fog(system);
    let ship_row = (150.0, 140.0);
    let empty_row = (150.0, 190.0);
    let entries = left_entries(
        &world,
        &rebellion_core::movement::MovementState::default(),
        &fog,
        Faction::Alliance,
        &open,
    );
    let fleet = Some(ReleaseTarget::Fleet { fleet: own, system });

    assert_eq!(
        drop_target(&world, Faction::Alliance, &open, &entries, ship_row),
        fleet
    );
    // Below the only ship, the default: the selected own fleet.
    assert_eq!(
        drop_target(&world, Faction::Alliance, &open, &entries, empty_row),
        fleet
    );
    open.selected = ListSelection::default();
    assert_eq!(
        drop_target(&world, Faction::Alliance, &open, &entries, ship_row),
        None
    );
    open.tab = FleetWindowTab::Troops;
    open.selected = ListSelection::only(FleetWindowEntry::Fleet(own));
    assert_eq!(
        drop_target(&world, Faction::Alliance, &open, &entries, ship_row),
        fleet
    );
}

#[test]
fn a_drop_on_the_right_list_of_several_entries_targets_the_listed_ships_fleet() {
    // FUN_004a3130: the ship under the point gives its fleet; with
    // several entries selected the list holds each fleet's ships, and
    // a selected ship itself (FUN_004a6be0, tab 0x66).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let one = add_fleet(&mut world, system, true, 1);
    let two = add_fleet(&mut world, system, true, 1);
    let mut open = window(system, None);
    open.expanded = vec![one, two];
    let ship = |fleet| FleetWindowEntry::Ship { fleet, index: 0 };
    let mut selection = ListSelection::only(ship(one));
    selection.press(Some(ship(two)), (0.0, 0.0), Modifiers { ctrl: true, shift: false }, &[]);
    open.selected = selection;
    let entries = left_entries(
        &world,
        &rebellion_core::movement::MovementState::default(),
        &fog(system),
        Faction::Alliance,
        &open,
    );
    let target = |open: &OpenFleetWindow, row: f32| {
        drop_target(&world, Faction::Alliance, open, &entries, (150.0, 140.0 + 50.0 * row))
    };
    assert_eq!(target(&open, 0.0), Some(ReleaseTarget::Fleet { fleet: one, system }));
    assert_eq!(target(&open, 1.0), Some(ReleaseTarget::Fleet { fleet: two, system }));
    // Below them, no one own fleet is selected: no target.
    assert_eq!(target(&open, 2.0), None);
    // One ship alone lists no ships.
    open.selected = ListSelection::only(ship(two));
    assert_eq!(target(&open, 0.0), None);
}

#[test]
fn several_selected_ships_list_themselves_and_light_the_tabs_any_of_them_lights() {
    // FUN_004a6be0 on tab 0x66 adds a selected ship itself; FUN_004a6980
    // keeps 0x66 a tab unless one ship alone is selected (+0x14c bit 8)
    // and lights 0x67..0x69 from the items' flags, or'ed.
    let (world, transport, _, [one, two], _) = two_fleet_window();
    let ship = |fleet| FleetWindowEntry::Ship { fleet, index: 0 };
    let both = [ship(one), ship(two)];
    let items = selection_items(&world, &MovementState::default(), &transport, &[], &both, FleetWindowTab::CapitalShips);
    assert_eq!(
        items.iter().map(|item| item.object).collect::<Vec<_>>(),
        [one, two].map(|fleet| Some(ItemObject::Ship {
            fleet,
            index: 0,
            roster: fleet_join::roster(&world, fleet).unwrap(),
        }))
    );
    let enabled = |selected: &[FleetWindowEntry], tab| {
        selection_tab_enabled(&world, &MovementState::default(), &transport, &[], selected, tab)
    };
    assert!(!enabled(&[], FleetWindowTab::CapitalShips));
    assert!(!enabled(&[ship(one)], FleetWindowTab::CapitalShips));
    assert!(enabled(&both, FleetWindowTab::CapitalShips));
    assert!(!enabled(&[ship(one)], FleetWindowTab::Troops));
    assert!(enabled(&both, FleetWindowTab::Troops));
    assert!(!enabled(&both, FleetWindowTab::Personnel));
}

#[test]
fn expanding_a_fleet_lists_its_ships_under_it() {
    // FUN_004a3d40: one entry per living capital ship while expanded.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    let mut open = window(system, None);
    open.expanded.push(fleet);

    assert_eq!(
        left_entries(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog(system),
            Faction::Alliance,
            &open
        ),
        [
            FleetWindowEntry::Fleet(fleet),
            FleetWindowEntry::Ship { fleet, index: 0 }
        ]
    );
}

#[test]
fn the_kind_claims_the_shared_focus_only_while_a_window_is_open() {
    // window_focus.rs: a kind with no window open makes no claim; one it
    // raises claims later than any stamp taken before.
    let (world, system) = world(ControlKind::Uncontrolled);
    let mut state = FleetWindowState::default();
    assert_eq!(state.focus_stamp(), None);
    let mut earlier = crate::window_focus::FocusStamp::default();
    earlier.take();
    assert!(state.open(&world, system, (10, 10), CockpitFaction::Alliance, layout()));
    let stamp = state.focus_stamp().expect("a window is open");
    assert!(stamp > earlier.stamp(), "{stamp} after {}", earlier.stamp());
    assert!(state.focus.holds());
    state.set_holds_focus(false);
    assert!(!state.focus.holds());
}

#[test]
fn a_system_has_one_fleet_window_placed_inside_the_galaxy_view() {
    // FUN_0045aac0: an open window comes to the front; a new one is
    // clamped into the view's rect.
    let (world, system) = world(ControlKind::Uncontrolled);
    let mut state = FleetWindowState::default();
    assert!(state.open(
        &world,
        system,
        (600, 400),
        CockpitFaction::Alliance,
        layout()
    ));
    // FUN_0045aac0 tells the agent of a created window only.
    assert!(state.take_created());
    assert!(state.open(&world, system, (10, 10), CockpitFaction::Alliance, layout()));
    assert!(!state.take_created());

    assert_eq!(state.window_count(), 1);
    assert_eq!(state.windows[0].logical_position, (405, 176));
}

#[test]
fn selecting_a_ship_from_outside_expands_its_fleet() {
    // FUN_00429440 selects the Fleet Finder's object through slot +0x6c;
    // a ship's entry lists only while its fleet is expanded.
    let (mut world, system) = world(ControlKind::Uncontrolled);
    let fleet = add_fleet(&mut world, system, true, 0);
    let mut state = FleetWindowState::default();
    let ship = FleetWindowEntry::Ship { fleet, index: 0 };
    assert!(!state.select(system, ship));
    assert!(state.open(&world, system, (0, 0), CockpitFaction::Alliance, layout()));
    state.windows[0].selected_items = ListSelection::only((ItemObject::Fleet(fleet), 0));

    assert!(state.select(system, ship));

    let window = &state.windows[0];
    assert_eq!(window.selected.single(), Some(ship));
    assert!(window.selected_items.is_empty());
    assert!(left_entries(
        &world,
        &rebellion_core::movement::MovementState::default(),
        &fog(system),
        Faction::Alliance,
        window
    )
    .contains(&ship));
    assert!(state.select(system, ship));
    assert_eq!(state.windows[0].expanded, [fleet]);
}

#[test]
fn a_minimized_window_comes_back_with_its_selection_and_tab() {
    // A0 personnel-states/39: Pzob's Fleet window, minimized with its
    // Arrow selected, comes back from the rail with the Arrow selected.
    let (mut world, system) = world(ControlKind::Uncontrolled);
    let fleet = add_fleet(&mut world, system, true, 0);
    let mut state = FleetWindowState::default();
    let ship = FleetWindowEntry::Ship { fleet, index: 0 };
    assert!(state.open(&world, system, (40, 30), CockpitFaction::Alliance, layout()));
    assert!(state.select(system, ship));
    state.windows[0].tab = FleetWindowTab::Fighters;
    let at = state.windows[0].logical_position;
    assert_eq!(state.hide(system), Some(at));
    assert_eq!(state.window_count(), 0);
    assert!(state.take_created());

    assert!(state.open(&world, system, (0, 0), CockpitFaction::Alliance, layout()));
    let window = &state.windows[0];
    assert_eq!(
        (window.selected.single(), window.tab, window.logical_position),
        (Some(ship), FleetWindowTab::Fighters, at)
    );
    assert!(state.hidden.is_empty());
}

/// The canvas 10 by 20 pixels in and twice the original size, so offsets
/// and scale both show.
fn scaled() -> CockpitLayout {
    let viewport = CockpitViewport {
        x: 10.0,
        y: 20.0,
        width: 1280.0,
        height: 960.0,
    };
    CockpitLayout {
        canvas: viewport,
        galaxy: viewport,
        scale: 2.0,
    }
}

/// A logical point in the window opened at `ORIGIN` on [`scaled`].
fn at(x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(
        10.0 + (f32::from(ORIGIN.0) + x) * 2.0,
        20.0 + (f32::from(ORIGIN.1) + y) * 2.0,
    )
}

const ORIGIN: (i16, i16) = (20, 30);

#[derive(Debug, Clone, PartialEq)]
struct Text {
    text: String,
    /// The line's pen at its first character and its baseline;
    /// `paint_gdi_text` paints it a character at a time, so each run of
    /// one size, colour and baseline is joined.
    pos: egui::Pos2,
    /// Where the last character's layout ends.
    end: f32,
    /// The clip rectangle the line is painted under.
    clip: egui::Rect,
    size: f32,
    color: egui::Color32,
}

impl Text {
    /// Whether DrawTextA set this line in the window's box (`x`, `y`,
    /// `w`, `h`) with font `font` and `align`, from the box's top
    /// (theme::gdi_text_origin).
    fn set_in(
        &self,
        (x, y, w, h): (f32, f32, f32, f32),
        font: u16,
        align: crate::theme::GdiAlign,
    ) -> bool {
        let width =
            crate::theme::gdi_pen_positions(font, &self.text).map_or(0.0, |(_, width)| width);
        let (left, baseline) = crate::theme::gdi_text_origin(
            (x, y, w, h),
            font,
            width,
            (align, crate::theme::GdiVAlign::Top),
        );
        self.pos == egui::pos2(at(left, 0.0).x, at(0.0, baseline).y)
    }
}

thread_local! {
    /// Each bitmap [`paint_native`] or the shared button art
    /// (`mission_dialog::paint`) was asked for and where, since the
    /// test cache holds no textures.
    pub(crate) static PAINTED: std::cell::RefCell<Vec<(u32, egui::Pos2)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Each key pixel [`paint_key_pixel`] repainted and where.
    pub(crate) static KEY_PIXELS: std::cell::RefCell<Vec<(KeyPixel, egui::Pos2)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[derive(Default)]
struct Run {
    actions: Vec<FleetWindowAction>,
    texts: Vec<Text>,
    lines: Vec<egui::Rect>,
    painted: Vec<(u32, egui::Pos2)>,
}

/// Draw the windows once per frame of events, on [`scaled`] for the
/// Alliance; the last frame's text and dots are kept.
fn run(
    world: &GameWorld,
    transport: &TroopTransportState,
    state: &mut FleetWindowState,
    faction: CockpitFaction,
    frames: Vec<Vec<egui::Event>>,
) -> Run {
    run_moving(
        world,
        &MovementState::default(),
        transport,
        state,
        faction,
        frames,
    )
}

/// [`run`] with fleets under `movement`'s orders.
fn run_moving(
    world: &GameWorld,
    movement: &MovementState,
    transport: &TroopTransportState,
    state: &mut FleetWindowState,
    faction: CockpitFaction,
    frames: Vec<Vec<egui::Event>>,
) -> Run {
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let fog =
        world
            .systems
            .keys()
            .fold(FogState::new(cockpit_faction(faction)), |mut fog, key| {
                fog.reveal(key);
                fog
            });
    let mut result = Run::default();
    for (index, events) in frames.into_iter().enumerate() {
        // The keys held come with the frame's button event.
        let modifiers = events
            .iter()
            .find_map(|event| match event {
                egui::Event::PointerButton { modifiers, .. } => Some(*modifiers),
                _ => None,
            })
            .unwrap_or_default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1000.0),
            )),
            time: Some(index as f64 * 0.05),
            events,
            modifiers,
            ..Default::default()
        };
        PAINTED.with(|painted| painted.borrow_mut().clear());
        let output = ctx.run(input, |ctx| {
            result.actions.extend(draw_fleet_windows(
                ctx,
                world,
                movement,
                &fog,
                transport,
                state,
                faction,
                scaled(),
                &mut cache,
            ));
        });
        result.painted = PAINTED.with(|painted| painted.take());
        result.texts.clear();
        result.lines.clear();
        for clipped in output.shapes {
            match clipped.shape {
                egui::Shape::Text(text) => {
                    let format = &text.galley.job.sections[0].format;
                    let baseline = text.pos.y
                        + text
                            .galley
                            .rows
                            .first()
                            .and_then(|row| row.glyphs.first())
                            .map_or(0.0, |glyph| glyph.pos.y);
                    match result.texts.last_mut() {
                        Some(line)
                            if line.size == format.font_id.size
                                && line.color == format.color
                                && line.pos.y == baseline
                                && line.clip == clipped.clip_rect
                                && (text.pos.x - line.end).abs() < line.size =>
                        {
                            line.text.push_str(text.galley.text());
                            line.end = text.pos.x + text.galley.size().x;
                        }
                        _ => result.texts.push(Text {
                            text: text.galley.text().to_owned(),
                            pos: egui::pos2(text.pos.x, baseline),
                            end: text.pos.x + text.galley.size().x,
                            size: format.font_id.size,
                            color: format.color,
                            clip: clipped.clip_rect,
                        }),
                    }
                }
                // White, faded in with its area (premultiplied).
                egui::Shape::Mesh(mesh)
                    if mesh.texture_id == egui::TextureId::default()
                        && mesh.vertices.len() == 4
                        && mesh.vertices.iter().all(|vertex| {
                            let fill = vertex.color;
                            fill.a() > 0 && [fill.r(), fill.g(), fill.b()] == [fill.a(); 3]
                        }) =>
                {
                    result.lines.push(mesh.calc_bounds());
                }
                _ => {}
            }
        }
    }
    result
}

fn hover(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
    vec![
        vec![egui::Event::PointerMoved(point)],
        vec![egui::Event::PointerMoved(point)],
    ]
}

fn press(point: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    }
}

fn click(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
    let mut frames = hover(point);
    frames.extend([vec![press(point, true)], vec![press(point, false)], vec![]]);
    frames
}

fn double_click(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
    let mut frames = hover(point);
    frames.extend([
        vec![press(point, true)],
        vec![press(point, false)],
        vec![press(point, true)],
        vec![press(point, false)],
        vec![],
    ]);
    frames
}

#[test]
fn a_mission_released_on_a_fleet_windows_lists_targets_the_object_under_the_point() {
    // +0x68 is FUN_004a2f80: the left list's entry under the point, the
    // right list's item under the point or else its selection, and the
    // subject elsewhere. A fleet (0x08..0x10) fits no class's target
    // family (FUN_005868c0, FUN_00586e20, FUN_00593500), so it drops the
    // order (inner None); a ship or squadron is an object target.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    crate::viewed_contents::see_live(&mut world, Faction::Alliance, system);
    let fleet = add_fleet(&mut world, system, false, 0);
    let transport_class = world.fleets[fleet].capital_ships[0].class;
    let death_star = world.capital_ship_classes.insert(CapitalShipClass {
        dat_id: DatId::new(rebellion_core::world::DEATH_STAR_CLASS_ID),
        name: "Death Star".into(),
        hull: 100,
        ..CapitalShipClass::default()
    });
    world.fleets[fleet]
        .capital_ships
        .insert(0, ShipInstance::new(death_star, 100, true));
    let tie = world
        .fighter_classes
        .insert(rebellion_core::world::FighterClass {
            name: "TIE".into(),
            ..Default::default()
        });
    world.fleets[fleet]
        .fighters
        .push(rebellion_core::world::FighterEntry {
            class: tie,
            count: 2,
            carrier: None,
            damage: None,
        });
    let officer = world.characters.insert(rebellion_core::world::Character {
        name: "Tarkin".into(),
        ..Default::default()
    });
    world.fleets[fleet].characters.push(officer);
    let fog = fog(system);
    let mut state = opened(&world, system);
    state.windows[0].expanded = vec![fleet];
    let layer = egui::LayerId::new(egui::Order::Foreground, area_id(system));
    let pick = |state: &FleetWindowState, point| {
        state
            .mission_target(
                &world,
                &MovementState::default(),
                &fog,
                &TroopTransportState::default(),
                scaled(),
                layer,
                point,
            )
            .unwrap()
    };
    let object = |state: &FleetWindowState, point| pick(state, point).unwrap().object;
    let here = Some(MissionTargetPick::from(system));
    let star = Some(MissionTarget::DeathStar(fleet));
    let transport = Some(MissionTarget::CapitalShip {
        fleet,
        class: transport_class,
    });
    let left = |row: f32| at(50.0, 29.0 + 25.0 + 50.0 * row);
    let right = |row: f32| at(160.0, 127.0 + 25.0 + 50.0 * row);

    // Left: the fleet drops the order; its ships are objects; past them
    // lies the subject.
    assert_eq!(pick(&state, left(0.0)), None);
    assert_eq!(object(&state, left(1.0)), star);
    assert_eq!(object(&state, left(2.0)), transport);
    assert_eq!(pick(&state, left(3.0)), here);
    // Right, with the fleet selected: its ships, then the selection.
    state.windows[0].selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
    assert_eq!(object(&state, right(0.0)), star);
    assert_eq!(object(&state, right(1.0)), transport);
    assert_eq!(pick(&state, right(2.0)), here);
    state.windows[0].selected_items =
        ListSelection::only(keys_of(&world, &state.windows[0], &[0])[0]);
    assert_eq!(object(&state, right(2.0)), star);
    state.windows[0].tab = FleetWindowTab::Fighters;
    state.windows[0].selected_items = ListSelection::default();
    assert_eq!(
        object(&state, right(1.0)),
        Some(MissionTarget::Fighter { fleet, class: tie })
    );
    state.windows[0].tab = FleetWindowTab::Personnel;
    assert_eq!(pick(&state, right(0.0)).unwrap().character, Some(officer));
    // Outside both lists: the subject.
    assert_eq!(pick(&state, at(150.0, 40.0)), here);
    let elsewhere = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("elsewhere"));
    assert_eq!(
        state.mission_target(
            &world,
            &MovementState::default(),
            &fog,
            &TroopTransportState::default(),
            scaled(),
            elsewhere,
            left(0.0),
        ),
        None
    );
}

fn opened(world: &GameWorld, system: SystemKey) -> FleetWindowState {
    let mut state = FleetWindowState::default();
    assert!(state.open(world, system, ORIGIN, CockpitFaction::Alliance, scaled()));
    state
}

/// Open `system`'s window with `fleet` selected on `tab`, press at
/// window pixel `from`, move to `to` and release there. Returns the
/// actions, the release point, and whether a drag was held before the
/// release.
fn drag_from_tab(
    world: &GameWorld,
    transport: &TroopTransportState,
    system: SystemKey,
    fleet: FleetKey,
    tab: FleetWindowTab,
    from: (f32, f32),
    to: (f32, f32),
) -> (Vec<FleetWindowAction>, egui::Pos2, bool) {
    let mut state = opened(world, system);
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
        window.tab = tab;
    }
    let (start, end) = (at(from.0, from.1), at(to.0, to.1));
    let mut frames = hover(start);
    frames.extend([
        vec![press(start, true)],
        vec![egui::Event::PointerMoved(end)],
    ]);
    let held = run(
        world,
        transport,
        &mut state,
        CockpitFaction::Alliance,
        frames,
    );
    let dragging = state.is_dragging();
    let released = run(
        world,
        transport,
        &mut state,
        CockpitFaction::Alliance,
        vec![vec![egui::Event::PointerMoved(end), press(end, false)]],
    );
    let mut actions = held.actions;
    actions.extend(released.actions);
    (actions, end, dragging)
}

fn regiment_drops(actions: &[FleetWindowAction]) -> Vec<(TroopKey, egui::Pos2)> {
    actions
        .iter()
        .flat_map(|action| match action {
            FleetWindowAction::DragRegiment { troops, point, .. } => {
                troops.iter().map(|troop| (*troop, *point)).collect()
            }
            _ => Vec::new(),
        })
        .collect()
}

fn fleet_with_regiment() -> (
    GameWorld,
    TroopTransportState,
    SystemKey,
    FleetKey,
    TroopKey,
) {
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 2);
    let troop = add_troop(&mut world, system);
    let mut transport = TroopTransportState::default();
    transport.embark(&mut world, fleet, &[troop]).unwrap();
    (world, transport, system, fleet, troop)
}

/// Press at screen point `from`, move to `to` and hold; then release.
/// Returns the window's position while held and after the release.
fn move_window(
    world: &GameWorld,
    state: &mut FleetWindowState,
    from: egui::Pos2,
    to: egui::Pos2,
) -> ((i16, i16), (i16, i16)) {
    let transport = TroopTransportState::default();
    let mut frames = hover(from);
    frames.extend([
        vec![press(from, true)],
        vec![egui::Event::PointerMoved(to)],
        vec![egui::Event::PointerMoved(to)],
    ]);
    run(world, &transport, state, CockpitFaction::Alliance, frames);
    let held = state.windows[0].logical_position;
    run(
        world,
        &transport,
        state,
        CockpitFaction::Alliance,
        vec![vec![press(to, false)], vec![]],
    );
    (held, state.windows[0].logical_position)
}

// FUN_004a2630 sets the window's +0xb8 bit 4, so a left press on its own
// surface holds it (FUN_00606ee0); it follows the pointer unbounded
// (FUN_006071e0, +0xf4 = 1) and the drop puts it back inside the galaxy
// view (FUN_00607010). A press on a child control, the left list, goes to
// the list and moves nothing.
#[test]
fn the_window_moves_by_its_surface_and_drops_inside_the_galaxy_view() {
    let (world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let mut state = opened(&world, system);
    // The title strip between the sector and minimize buttons.
    let (held, dropped) = move_window(&world, &mut state, at(100.0, 10.0), at(130.0, 30.0));
    assert_eq!(held, (50, 50));
    assert_eq!(dropped, (50, 50));
    // Past the galaxy view's left edge while held; back on the drop.
    let from = at(130.0, 30.0);
    let (held, dropped) = move_window(&world, &mut state, from, from - egui::vec2(200.0, 0.0));
    assert_eq!(held, (-50, 50));
    assert_eq!(dropped, (0, 50));
    // The left list.
    let list = egui::pos2(10.0 + (0.0 + 40.0) * 2.0, 20.0 + (50.0 + 200.0) * 2.0);
    let (held, dropped) = move_window(&world, &mut state, list, list + egui::vec2(60.0, 60.0));
    assert_eq!((held, dropped), ((0, 50), (0, 50)));
}

/// Inside the first right-list item.
const FIRST_RIGHT_ITEM: (f32, f32) = (160.0, 150.0);

/// A left click with `modifiers` held.
fn click_with(point: egui::Pos2, modifiers: egui::Modifiers) -> Vec<Vec<egui::Event>> {
    let button = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    };
    let mut frames = hover(point);
    frames.extend([vec![button(true)], vec![button(false)], vec![]]);
    frames
}

/// A fleet of four ships at an Alliance system, open on its Capital
/// Ships tab.
fn four_ship_window() -> (GameWorld, SystemKey, FleetKey, FleetWindowState) {
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let class = world.fleets[fleet].capital_ships[0].class;
    for _ in 0..3 {
        world.fleets[fleet]
            .capital_ships
            .push(ShipInstance::new(class, 100, true));
    }
    let mut state = opened(&world, system);
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
        window.tab = FleetWindowTab::CapitalShips;
    }
    (world, system, fleet, state)
}

#[test]
fn ctrl_and_shift_clicks_frame_several_right_list_items() {
    // FUN_006083c0: FUN_004a4b10 gives the right list no flags, so not
    // 0x10000 (one selection): MK_CONTROL toggles an item, MK_SHIFT adds
    // the cells from the last press to this one (FUN_00609590), and a
    // plain press selects one (FUN_006094b0). Each selected item draws
    // its frame 10420 (FUN_004a6e70). Manual p. 120: "select multiple
    // ships by holding down the CTRL key while clicking".
    let (world, _, _, mut state) = four_ship_window();
    let transport = TroopTransportState::default();
    let row = |row: f32| at(160.0, 150.0 + 50.0 * row);
    let frames = |run: &Run| {
        run.painted
            .iter()
            .filter(|(id, _)| *id == side_art(RIGHT_ITEM_SELECTED, 1))
            .map(|(_, point)| *point)
            .collect::<Vec<_>>()
    };
    let click = |state: &mut FleetWindowState, point, modifiers| {
        run(
            &world,
            &transport,
            state,
            CockpitFaction::Alliance,
            click_with(point, modifiers),
        )
    };
    let _ = click(&mut state, row(0.0), egui::Modifiers::NONE);
    let two = click(&mut state, row(2.0), egui::Modifiers::CTRL);
    assert_eq!(frames(&two), [at(102.0, 128.0), at(102.0, 228.0)]);
    // Shift from row 2 back to row 1 adds row 1.
    let three = click(&mut state, row(1.0), egui::Modifiers::SHIFT);
    assert_eq!(
        frames(&three),
        [at(102.0, 128.0), at(102.0, 178.0), at(102.0, 228.0)]
    );
    // Ctrl on a selected item takes it out on the release.
    let _ = click(&mut state, row(0.0), egui::Modifiers::CTRL);
    assert_eq!(
        state.windows[0].selected_items.items(),
        keys_of(&world, &state.windows[0], &[2, 1])
    );
    // A plain click on a selected item leaves it alone.
    let one = click(&mut state, row(1.0), egui::Modifiers::NONE);
    assert_eq!(frames(&one), [at(102.0, 178.0)]);
}

#[test]
fn the_right_lists_cells_and_hits_follow_its_125_by_50_items() {
    // Our own layout of FUN_004a4b10's right list: 125 by 50 items one
    // under another, 133 wide (RIGHT_LIST).
    assert_eq!(
        right_cells(3),
        [
            (0, (0.0, 0.0), (125.0, 50.0)),
            (1, (0.0, 50.0), (125.0, 50.0)),
            (2, (0.0, 100.0), (125.0, 50.0)),
        ]
    );
    assert_eq!(right_hit((0.0, 0.0), 3), Some(0));
    assert_eq!(right_hit((10.0, 149.0), 3), Some(2));
    assert_eq!(right_hit((10.0, 150.0), 3), None);
    assert_eq!(right_hit((RIGHT_LIST.2, 10.0), 3), None);
    assert_eq!(right_hit((-1.0, 10.0), 3), None);
    assert_eq!(right_hit((10.0, -1.0), 3), None);
    assert_eq!(right_hit((RIGHT_LIST.2 - 1.0, 10.0), 3), Some(0));
}

#[test]
fn a_drop_carries_its_own_fleets_ships_and_squadrons_and_every_member() {
    // FUN_00422ce0 moves the selection as one order; the right list
    // holds one tab's objects. port: ships and squadrons of several
    // fleets (several fleets selected on the left) go one order per
    // fleet.
    let (mut world, system) = world(ControlKind::Uncontrolled);
    let (one, two) = (
        add_fleet(&mut world, system, true, 0),
        add_fleet(&mut world, system, true, 0),
    );
    let point = egui::pos2(1.0, 2.0);
    let ship = |fleet, index| ItemObject::Ship {
        fleet,
        index,
        roster: 7,
    };
    assert_eq!(
        right_drop(&[ship(one, 0), ship(two, 1), ship(one, 2)], point, false),
        [
            FleetWindowAction::DragShip {
                fleet: one,
                ships: vec![0, 2],
                roster: 7,
                point,
                confirmed: false,
            },
            FleetWindowAction::DragShip {
                fleet: two,
                ships: vec![1],
                roster: 7,
                point,
                confirmed: false,
            }
        ]
    );
    let fighter = |fleet, index| ItemObject::Fighter { fleet, index };
    assert_eq!(
        right_drop(&[fighter(one, 0), fighter(two, 1), fighter(one, 3)], point, false),
        [
            FleetWindowAction::DragSquadron {
                fleet: one,
                indices: vec![0, 3],
                point,
                confirmed: false,
            },
            FleetWindowAction::DragSquadron {
                fleet: two,
                indices: vec![1],
                point,
                confirmed: false,
            }
        ]
    );
    let person = world.characters.insert(Default::default());
    let unit = world.special_forces.insert(rebellion_core::world::SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
        on_hidden_mission: false,
    });
    assert_eq!(
        right_drop(
            &[ItemObject::Character(person), ItemObject::SpecialForce(unit)],
            point,
            false
        ),
        [FleetWindowAction::DragMembers {
            members: vec![
                MissionMember::Character(person),
                MissionMember::SpecialForce(unit)
            ],
            point,
            confirmed: false,
        }]
    );
    assert_eq!(
        left_drop(&[ship(two, 4)], point, false),
        [FleetWindowAction::DragShip {
            fleet: two,
            ships: vec![4],
            roster: 7,
            point,
            confirmed: false,
        }]
    );
    // Several left entries: the fleets as one fleet move, the ships
    // as their fleet's ship order.
    assert_eq!(
        left_drop(
            &[ItemObject::Fleet(one), ship(two, 4), ItemObject::Fleet(two)],
            point,
            true
        ),
        [
            FleetWindowAction::DragFleet {
                fleets: vec![one, two],
                point,
                confirmed: true,
            },
            FleetWindowAction::DragShip {
                fleet: two,
                ships: vec![4],
                roster: 7,
                point,
                confirmed: true,
            }
        ]
    );
}

#[test]
fn a_held_right_list_drag_paints_the_selected_items_where_the_pointer_moved() {
    // FUN_006083c0 case 0x200: the selected items' images (FUN_00609960),
    // the frame 10420 included, follow the pointer (FUN_0060df00).
    let (world, _, _, mut state) = four_ship_window();
    let transport = TroopTransportState::default();
    let start = at(160.0, 150.0);
    let mut frames = hover(start);
    frames.extend([
        vec![press(start, true)],
        vec![egui::Event::PointerMoved(at(140.0, 170.0))],
    ]);
    let held = run(&world, &transport, &mut state, CockpitFaction::Alliance, frames);
    let frame = side_art(RIGHT_ITEM_SELECTED, 1);
    assert!(held.painted.contains(&(frame, at(102.0, 128.0))));
    assert!(held.painted.contains(&(frame, at(82.0, 148.0))));
}

#[test]
fn a_held_left_list_drag_paints_the_selected_entry_where_the_pointer_moved() {
    // FUN_006083c0 case 0x200: every CoolDragList copies its selected
    // items' images (FUN_00609960) into its drag bitmap, which follows
    // the pointer (FUN_0060df00); the Fleet window's left list too.
    let (world, transport, system, _, _) = fleet_with_regiment();
    let mut state = opened(&world, system);
    let start = at(40.0, 40.0);
    let mut frames = hover(start);
    frames.extend([
        vec![press(start, true)],
        vec![egui::Event::PointerMoved(at(60.0, 70.0))],
    ]);
    let held = run(&world, &transport, &mut state, CockpitFaction::Alliance, frames);
    let frame = side_art(FLEET_SELECTED, 1);
    // The entry's image, 20 by 30 on from its place at the list's (4, 29).
    assert!(held.painted.contains(&(frame, at(24.0, 59.0))), "{:?}", held.painted);
}

/// Two Alliance fleets of one transport each at an Alliance system, a
/// regiment aboard the second, its window open.
fn two_fleet_window() -> (
    GameWorld,
    TroopTransportState,
    SystemKey,
    [FleetKey; 2],
    FleetWindowState,
) {
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let one = add_fleet(&mut world, system, true, 2);
    let two = add_fleet(&mut world, system, true, 2);
    let troop = add_troop(&mut world, system);
    let mut transport = TroopTransportState::default();
    transport.embark(&mut world, two, &[troop]).unwrap();
    let state = opened(&world, system);
    (world, transport, system, [one, two], state)
}

fn click_entry(
    world: &GameWorld,
    transport: &TroopTransportState,
    state: &mut FleetWindowState,
    system: SystemKey,
    row: usize,
    modifiers: egui::Modifiers,
) -> Run {
    let point = state.entry_screen_rect(scaled(), system, row).unwrap().center();
    run(world, transport, state, CockpitFaction::Alliance, click_with(point, modifiers))
}

#[test]
fn ctrl_clicks_select_several_fleets_and_the_right_pane_shows_them_all() {
    // FUN_004a4b10 gives the left list no flags either, so MK_CONTROL
    // adds an entry (FUN_006083c0). FUN_004a5c00 then empties the right
    // list and has each selected entry add its contents (FUN_004a6be0),
    // sums the counts into +0x198/+0x19c, or's the tabs' flags
    // (FUN_004a6980) and names nothing (FUN_00601aa0(0)).
    let (world, transport, system, [one, two], mut state) = two_fleet_window();
    let report = |state: &FleetWindowState| {
        state
            .report(&world, &MovementState::default(), &fog(system), &transport, system)
            .unwrap()
    };
    let _ = click_entry(&world, &transport, &mut state, system, 0, egui::Modifiers::NONE);
    assert_eq!(report(&state).items.len(), 1);
    assert!(!report(&state).enabled[2], "the first fleet carries no regiment");
    let _ = click_entry(&world, &transport, &mut state, system, 1, egui::Modifiers::CTRL);
    let both = report(&state);
    assert_eq!(both.selected_rows, [0, 1]);
    assert_eq!(both.selected, None);
    assert_eq!(both.items.len(), 2, "both fleets' ships");
    assert!(both.enabled[2], "the second fleet's regiment lights Troops");
    assert!(state.show_tab(system, FleetWindowTab::Troops));
    assert_eq!(report(&state).counts, Some((1, 4)));
    assert_eq!(report(&state).items.len(), 1);
    // Ctrl on a selected entry takes it out on the release.
    let _ = click_entry(&world, &transport, &mut state, system, 0, egui::Modifiers::CTRL);
    assert_eq!(report(&state).selected_rows, [1]);
    assert_eq!(report(&state).selected, Some(FleetWindowEntry::Fleet(two)));
    let _ = one;
}

#[test]
fn selecting_an_own_fleet_drops_the_other_sides_from_the_selection() {
    // FUN_004a5c00: when a selected item has +0x68 bit 8 (the viewer's
    // side), every selected item without it is deselected.
    let (mut world, transport, system, [one, _], _) = two_fleet_window();
    let enemy = add_fleet(&mut world, system, false, 0);
    let mut state = opened(&world, system);
    let rows = |state: &FleetWindowState| {
        state
            .report(&world, &MovementState::default(), &fog(system), &transport, system)
            .unwrap()
    };
    let enemy_row = rows(&state)
        .fleet_rows
        .iter()
        .find(|(fleet, _)| *fleet == enemy)
        .unwrap()
        .1;
    let own_row = rows(&state)
        .fleet_rows
        .iter()
        .find(|(fleet, _)| *fleet == one)
        .unwrap()
        .1;
    let _ = click_entry(&world, &transport, &mut state, system, enemy_row, egui::Modifiers::NONE);
    assert_eq!(rows(&state).selected, Some(FleetWindowEntry::Fleet(enemy)));
    let _ = click_entry(&world, &transport, &mut state, system, own_row, egui::Modifiers::CTRL);
    assert_eq!(rows(&state).selected, Some(FleetWindowEntry::Fleet(one)));
}

#[test]
fn a_drag_and_a_menu_from_a_selected_fleet_take_every_selected_fleet() {
    // The press on a selected entry holds its click (flag 0x10), so the
    // drop (0x29a) carries the selection (FUN_00422ce0 reads slot
    // +0x58), as the menu does (FUN_004ac5c0).
    let (world, transport, system, [one, two], mut state) = two_fleet_window();
    let _ = click_entry(&world, &transport, &mut state, system, 0, egui::Modifiers::NONE);
    let _ = click_entry(&world, &transport, &mut state, system, 1, egui::Modifiers::CTRL);
    let start = state.entry_screen_rect(scaled(), system, 1).unwrap().center();
    let end = at(300.0, 200.0);
    let mut frames = hover(start);
    frames.extend([
        vec![press(start, true)],
        vec![egui::Event::PointerMoved(end)],
        vec![egui::Event::PointerMoved(end), press(end, false)],
    ]);
    let dropped = run(&world, &transport, &mut state, CockpitFaction::Alliance, frames);
    assert!(
        dropped.actions.contains(&FleetWindowAction::DragFleet {
            fleets: vec![one, two],
            point: end,
            confirmed: false,
        }),
        "{:?}",
        dropped.actions
    );
    let point = state.entry_screen_rect(scaled(), system, 1).unwrap().center();
    let teams: Vec<Vec<MenuObject>> = right_click(&world, &transport, &mut state, point)
        .into_iter()
        .filter_map(|action| match action {
            FleetWindowAction::OpenObjectMenu { team, .. } => Some(team),
            _ => None,
        })
        .collect();
    assert_eq!(teams, [vec![MenuObject::Fleet(two), MenuObject::Fleet(one)]]);
}

#[test]
fn a_left_drag_image_holds_only_the_pressed_entry() {
    let (mut world, system) = world(ControlKind::Uncontrolled);
    let (one, two) = (
        add_fleet(&mut world, system, true, 0),
        add_fleet(&mut world, system, true, 0),
    );
    let image = |left| DragImage {
        offset: egui::Vec2::ZERO,
        left,
    };
    let fleet = image(Some(ItemObject::Fleet(one)));
    assert!(fleet.holds(FleetWindowEntry::Fleet(one)));
    assert!(!fleet.holds(FleetWindowEntry::Fleet(two)));
    assert!(!fleet.holds(FleetWindowEntry::Ship { fleet: one, index: 0 }));
    let ship = image(Some(ItemObject::Ship {
        fleet: one,
        index: 1,
        roster: 0,
    }));
    assert!(ship.holds(FleetWindowEntry::Ship { fleet: one, index: 1 }));
    assert!(!ship.holds(FleetWindowEntry::Ship { fleet: one, index: 0 }));
    assert!(!ship.holds(FleetWindowEntry::Ship { fleet: two, index: 1 }));
    assert!(!image(None).holds(FleetWindowEntry::Fleet(one)));
}

#[test]
fn a_drag_from_a_selected_ship_carries_every_selected_ship() {
    // A press on a selected item holds its click (flag 0x10), so the
    // drop (0x29a) carries the selection, which FUN_00422ce0 reads
    // through slot +0x58 and moves with one 0x201 (manual p. 120: "To
    // move ships [...] simply drag the item").
    let (world, _, fleet, mut state) = four_ship_window();
    let transport = TroopTransportState::default();
    let row = |row: f32| at(160.0, 150.0 + 50.0 * row);
    for (point, modifiers) in [
        (row(1.0), egui::Modifiers::NONE),
        (row(2.0), egui::Modifiers::CTRL),
    ] {
        let _ = run(
            &world,
            &transport,
            &mut state,
            CockpitFaction::Alliance,
            click_with(point, modifiers),
        );
    }
    let (start, end) = (row(2.0), at(40.0, 200.0));
    let mut frames = hover(start);
    frames.extend([
        vec![press(start, true)],
        vec![egui::Event::PointerMoved(end)],
        vec![egui::Event::PointerMoved(end), press(end, false)],
    ]);
    let dropped = run(&world, &transport, &mut state, CockpitFaction::Alliance, frames);
    let ships: Vec<_> = dropped
        .actions
        .iter()
        .filter_map(|action| match action {
            FleetWindowAction::DragShip { fleet: of, ships, .. } if *of == fleet => {
                Some(ships.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(ships, [vec![1, 2]]);
    // The drop keeps the selection.
    assert_eq!(
        state.windows[0].selected_items.items(),
        keys_of(&world, &state.windows[0], &[1, 2])
    );
}

#[test]
fn a_regiment_dragged_out_of_the_troops_tab_drops_where_the_button_comes_up() {
    // FUN_006083c0 captures the mouse on the press and posts 0x29a with
    // the release point; FUN_00422ce0 moves a type 4 selection with
    // 0x201 against the window under it.
    let (world, transport, system, fleet, troop) = fleet_with_regiment();

    let (actions, release, held) = drag_from_tab(
        &world,
        &transport,
        system,
        fleet,
        FleetWindowTab::Troops,
        FIRST_RIGHT_ITEM,
        (40.0, 200.0),
    );

    assert!(held);
    assert_eq!(regiment_drops(&actions), [(troop, release)]);
}

#[test]
fn show_tab_switches_an_open_window_and_refuses_a_closed_one() {
    // Our helper: the tab a click on its button would show.
    let (world, _, system, fleet, _) = fleet_with_regiment();
    let mut state = FleetWindowState::default();
    let layout = layout();
    state.open(&world, system, (5, 5), CockpitFaction::Alliance, layout);
    state.select(system, FleetWindowEntry::Fleet(fleet));

    assert!(state.show_tab(system, FleetWindowTab::Personnel));
    assert_eq!(
        state.selection(system),
        Some((Some(FleetWindowEntry::Fleet(fleet)), FleetWindowTab::Personnel))
    );
    assert!(!state.show_tab(SystemKey::default(), FleetWindowTab::Troops));
}

#[test]
fn a_character_dragged_out_of_the_personnel_tab_drops_where_the_button_comes_up() {
    // Manual p. 111: a character is moved by dragging its icon onto a
    // system or fleet; FUN_00422ce0 issues 0x201, which expands to the
    // Move mission order 0x241 (FUN_0053c810).
    let (mut world, transport, system, fleet, _) = fleet_with_regiment();
    let character = world.characters.insert(rebellion_core::world::Character {
        name: "Admiral".into(),
        is_alliance: true,
        current_fleet: Some(fleet),
        current_system: Some(system),
        ..Default::default()
    });
    world.fleets[fleet].characters.push(character);

    let (actions, release, held) = drag_from_tab(
        &world,
        &transport,
        system,
        fleet,
        FleetWindowTab::Personnel,
        FIRST_RIGHT_ITEM,
        (40.0, 200.0),
    );

    assert!(held);
    let drops: Vec<_> = actions
        .iter()
        .filter_map(|action| match action {
            FleetWindowAction::DragMembers { members, point, .. } => Some((members.clone(), *point)),
            _ => None,
        })
        .collect();
    assert_eq!(
        drops,
        [(vec![MissionMember::Character(character)], release)]
    );
}

#[test]
fn a_squadron_dragged_out_of_the_fighters_tab_drops_its_entry() {
    // FUN_006083c0 / FUN_00422ce0: the squadron's 0x201 against the
    // window under the release (FUN_00557ce0 lists Move for it).
    let (mut world, transport, system, fleet, _) = fleet_with_regiment();
    let fighter = world.fighter_classes.insert(Default::default());
    world.fleets[fleet]
        .fighters
        .push(rebellion_core::world::FighterEntry {
            class: fighter,
            count: 1,
            carrier: None,
            damage: None,
        });

    let (actions, release, held) = drag_from_tab(
        &world,
        &transport,
        system,
        fleet,
        FleetWindowTab::Fighters,
        FIRST_RIGHT_ITEM,
        (40.0, 200.0),
    );

    assert!(held);
    assert_eq!(
        actions
            .iter()
            .filter(|action| matches!(action, FleetWindowAction::DragSquadron { .. }))
            .collect::<Vec<_>>(),
        [&FleetWindowAction::DragSquadron {
            fleet,
            indices: vec![0],
            point: release,
            confirmed: false,
        }]
    );
}

#[test]
fn a_regiment_released_near_the_press_drops_nothing_and_inside_its_list_drops() {
    // FUN_006083c0: the squared distance must exceed 0x18; the release
    // need leave the list only when its flags hold 0x100000, and
    // FUN_004a4b10 sets none on the Fleet window's lists.
    let (world, transport, system, fleet, troop) = fleet_with_regiment();
    // 4 pixels up out of the list from its top edge: too near.
    let (actions, _, held) = drag_from_tab(
        &world,
        &transport,
        system,
        fleet,
        FleetWindowTab::Troops,
        (160.0, 128.0),
        (160.0, 124.0),
    );
    assert!(held);
    assert!(regiment_drops(&actions).is_empty());
    // Down the list, inside it: a drop.
    let (actions, release, held) = drag_from_tab(
        &world,
        &transport,
        system,
        fleet,
        FleetWindowTab::Troops,
        FIRST_RIGHT_ITEM,
        (160.0, 200.0),
    );
    assert!(held);
    assert_eq!(regiment_drops(&actions), [(troop, release)]);
}

#[test]
fn a_press_on_a_regiments_hidden_part_below_the_list_starts_no_drag() {
    // The fourth row starts 150 below the list's top and the list is 164
    // tall, so a press at row offset 23 lies in its cell but under the
    // list's bottom edge.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 4);
    let troops: Vec<_> = (0..4).map(|_| add_troop(&mut world, system)).collect();
    let mut transport = TroopTransportState::default();
    transport.embark(&mut world, fleet, &troops).unwrap();

    let (actions, _, held) = drag_from_tab(
        &world,
        &transport,
        system,
        fleet,
        FleetWindowTab::Troops,
        (160.0, 127.0 + 150.0 + 23.0),
        (40.0, 200.0),
    );

    assert!(!held);
    assert!(regiment_drops(&actions).is_empty());
}

#[test]
fn a_scrolled_right_list_moves_its_items_up_and_keeps_its_place_on_a_new_selection() {
    // The bar's first visible pixel (FUN_0060a490, RCDATA 10699; A0
    // fleet-aboard/10 shows it for six squadrons) moves every row up.
    let (world, _, system, _, _) = fleet_with_regiment();
    let mut state = opened(&world, system);
    state.window_mut(system).unwrap().right_scroll = 40.0;
    assert_eq!(
        state.item_screen_rect(scaled(), system, 1),
        Some(egui::Rect::from_min_max(at(101.0, 137.0), at(226.0, 187.0)))
    );
    // FUN_004a5c00 refills the list without touching its first visible
    // pixel +0xc4; FUN_00609ae0 only clamps it (the draw does).
    let fleet = world.systems[system].fleets[0];
    assert!(state.select(system, FleetWindowEntry::Fleet(fleet)));
    assert_eq!(state.window_mut(system).unwrap().right_scroll, 40.0);
    // The bar's art: width 10, its arrows keyed (bmp_cache).
    assert_eq!(LIST_BAR_ART.width, 10);
    assert_eq!(LIST_BAR_ART.up, [10_692, 10_693]);
}

#[test]
fn the_right_lists_rows_follow_its_scroll() {
    // Rows 50 high from y 127, 164 shown (FUN_004a4b10's list 200).
    assert_eq!(right_content(6), 300.0);
    assert_eq!(right_row_at(127.0, 0.0), 0);
    assert_eq!(right_row_at(180.0, 0.0), 1);
    assert_eq!(right_row_at(180.0, 60.0), 2);
    assert_eq!(right_row_top(0, 0.0), Some(127.0));
    assert_eq!(right_row_top(1, 10.0), Some(167.0));
    // A row scrolled wholly above the list, or below it, is not drawn;
    // one that shows a single pixel is.
    assert_eq!(right_row_top(0, 50.0), None);
    assert_eq!(right_row_top(0, 49.0), Some(78.0));
    assert_eq!(right_row_top(4, 0.0), None);
    assert_eq!(right_row_top(3, 0.0), Some(277.0));
    // The list ends at y 291: a row starting there is hidden.
    assert_eq!(right_row_top(4, 36.0), None);
    assert_eq!(right_row_top(4, 37.0), Some(290.0));
    assert_eq!(reset_when_hidden(0.0, None), None);
    assert_eq!(reset_when_hidden(30.0, None), Some((0.0, None)));
}

#[test]
fn another_tab_keeps_the_right_lists_place() {
    // The strip's 0x6a/0x6b (FUN_004a6390) refills through FUN_004a5c00,
    // which leaves +0xc4 to FUN_00609ae0's clamp.
    let mut window = window(SystemKey::default(), None);
    window.right_scroll = 40.0;
    window.show_tab(FleetWindowTab::Troops);
    assert_eq!(
        (window.tab, window.right_scroll),
        (FleetWindowTab::Troops, 40.0)
    );
}

#[test]
fn a_new_selection_draws_the_right_list_from_its_kept_place_clamped_to_the_range() {
    // FUN_00609ae0: +0xc4 stays unless past the new range (content + 5
    // - 164 for a full list), and a range of 0 (no bar) shows from 0.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let big = add_fleet(&mut world, system, true, 0);
    let class = world.fleets[big].capital_ships[0].class;
    for _ in 0..5 {
        world.fleets[big]
            .capital_ships
            .push(ShipInstance::new(class, 100, true));
    }
    let four = add_fleet(&mut world, system, true, 0);
    for _ in 0..3 {
        world.fleets[four]
            .capital_ships
            .push(ShipInstance::new(class, 100, true));
    }
    let small = add_fleet(&mut world, system, true, 0);
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    let frame = |state: &mut FleetWindowState| {
        run(
            &world,
            &transport,
            state,
            CockpitFaction::Alliance,
            hover(at(-50.0, 0.0)),
        );
        state.windows[0].right_scroll
    };
    // Six ships (300 high): kept at 100.
    assert!(state.select(system, FleetWindowEntry::Fleet(big)));
    state.window_mut(system).unwrap().right_scroll = 100.0;
    assert_eq!(frame(&mut state), 100.0);
    // A click on the second entry, four ships (200 high): clamped to
    // 200 + 5 - 164 = 41.
    run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        click(at(40.0, 29.0 + 75.0)),
    );
    assert_eq!(
        state.windows[0].selected.single(),
        Some(FleetWindowEntry::Fleet(four))
    );
    assert_eq!(frame(&mut state), 41.0);
    // One ship: no bar, from the top.
    assert!(state.select(system, FleetWindowEntry::Fleet(small)));
    assert_eq!(frame(&mut state), 0.0);
}

#[test]
fn the_left_lists_entries_follow_its_scroll() {
    // Entries 50 high from y 29, 266 shown (FUN_004a4b10's list 0xc9,
    // 0x5b by 0x10a, with its own 0x29cb bar).
    assert_eq!(left_content(7), 350.0);
    assert_eq!(left_row_at(29.0, 0.0), 0);
    assert_eq!(left_row_at(80.0, 0.0), 1);
    assert_eq!(left_row_at(80.0, 89.0), 2);
    assert_eq!(left_row_top(0, 0.0), Some(29.0));
    assert_eq!(left_row_top(6, 89.0), Some(240.0));
    // An entry scrolled wholly above the list, or below it, is not
    // drawn; one that shows a single pixel is.
    assert_eq!(left_row_top(0, 50.0), None);
    assert_eq!(left_row_top(0, 49.0), Some(-20.0));
    assert_eq!(left_row_top(5, 0.0), Some(279.0));
    assert_eq!(left_row_top(6, 0.0), None);
    // The list ends at y 295: an entry starting there is hidden.
    assert_eq!(left_row_top(6, 34.0), None);
    assert_eq!(left_row_top(6, 35.0), Some(294.0));
}

#[test]
fn a_scrolled_left_list_moves_its_entries_rects_and_drop_rows_up() {
    // The left list's bar (FUN_004a4b10: FUN_0060a490, 0x29cb) shows
    // from its first visible pixel; the +0x70 drop (FUN_004a3130 step
    // 2) takes the entry under the point.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleets: Vec<_> = (0..7)
        .map(|_| add_fleet(&mut world, system, true, 0))
        .collect();
    let mut state = opened(&world, system);
    state.window_mut(system).unwrap().left_scroll = 89.0;
    assert_eq!(
        state.entry_screen_rect(scaled(), system, 6),
        Some(egui::Rect::from_min_max(at(4.0, 240.0), at(95.0, 290.0)))
    );
    let open = state.windows[0].clone();
    let entries = left_entries(
        &world,
        &MovementState::default(),
        &fog(system),
        Faction::Alliance,
        &open,
    );
    assert_eq!(
        drop_target(&world, Faction::Alliance, &open, &entries, (40.0, 260.0)),
        Some(ReleaseTarget::Fleet {
            fleet: fleets[6],
            system
        })
    );
    let layer = egui::LayerId::new(egui::Order::Foreground, area_id(system));
    // FUN_004a2f80: the left list's entry under the point.
    let pick = |state: &FleetWindowState, y: f32| {
        state
            .mission_target(
                &world,
                &MovementState::default(),
                &fog(system),
                &TroopTransportState::default(),
                scaled(),
                layer,
                at(40.0, y),
            )
            .unwrap()
    };
    let ship = world.fleets[fleets[6]].capital_ships[0].class;
    state.windows[0].expanded = vec![fleets[6]];
    // Fleet 7 at 240, its ship at 290: the ship is an object target.
    assert_eq!(
        pick(&state, 292.0).unwrap().object,
        Some(MissionTarget::CapitalShip {
            fleet: fleets[6],
            class: ship
        })
    );
}

#[test]
fn a_click_on_a_scrolled_left_list_selects_the_entry_shown_and_the_bar_selects_nothing() {
    // FUN_004a6390: the left list's 0x29b posts 0x467 and refreshes the
    // right pane only, so the selection keeps the list's place (hyp).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleets: Vec<_> = (0..7)
        .map(|_| add_fleet(&mut world, system, true, 0))
        .collect();
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    state.window_mut(system).unwrap().left_scroll = 89.0;
    // A press on the bar's track (window x 85..95) pages it and
    // chooses no entry.
    run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        click(at(90.0, 200.0)),
    );
    assert_eq!(state.windows[0].selected.single(), None);
    state.window_mut(system).unwrap().left_scroll = 89.0;
    run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        click(at(40.0, 260.0)),
    );
    assert_eq!(
        (state.windows[0].selected.single(), state.windows[0].left_scroll),
        (Some(FleetWindowEntry::Fleet(fleets[6])), 89.0)
    );
}

#[test]
fn a_left_list_press_selects_its_entry_before_the_release() {
    // FUN_006083c0, case 0x201: a press on an unselected item clears the
    // list and sets its bit 0 at once (FUN_006094b0); A0
    // 2026-11-13-drag/25 frames the pressed row while the button is
    // still held.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleets: Vec<_> = (0..2)
        .map(|_| add_fleet(&mut world, system, true, 0))
        .collect();
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    let point = at(40.0, 29.0 + 50.0 + 25.0);
    let mut frames = hover(point);
    frames.push(vec![press(point, true)]);
    frames.push(vec![]);
    run(&world, &transport, &mut state, CockpitFaction::Alliance, frames);
    assert_eq!(
        state.windows[0].selected.items(),
        [FleetWindowEntry::Fleet(fleets[1])]
    );
}

#[test]
fn a_selection_from_outside_leaves_the_left_list_where_it_was_scrolled() {
    // The Fleet Finder's open (FUN_00429440) selects through slot +0x6c
    // (FUN_004a73c0): it marks the entry (+0x3c bit 0) and relays out
    // the list (FUN_0060a280: FUN_00609ae0, FUN_00609a00), which only
    // clamps the first visible pixel to the range; nothing scrolls the
    // entry into view.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleets: Vec<_> = (0..7)
        .map(|_| add_fleet(&mut world, system, true, 0))
        .collect();
    let mut state = opened(&world, system);
    state.window_mut(system).unwrap().left_scroll = 89.0;
    assert!(state.select(system, FleetWindowEntry::Fleet(fleets[0])));
    assert!(state.select(
        system,
        FleetWindowEntry::Ship {
            fleet: fleets[0],
            index: 0
        }
    ));
    assert_eq!(state.windows[0].left_scroll, 89.0);
}

#[test]
fn the_left_lists_bar_hides_and_returns_to_the_top_once_its_entries_fit() {
    // FUN_0060b5d0: the bar shows unless the whole range is in view.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    add_fleet(&mut world, system, true, 0);
    let mut state = opened(&world, system);
    state.window_mut(system).unwrap().left_scroll = 40.0;
    run(
        &world,
        &TroopTransportState::default(),
        &mut state,
        CockpitFaction::Alliance,
        hover(at(-50.0, 0.0)),
    );
    assert_eq!(state.windows[0].left_scroll, 0.0);
}

#[test]
fn a_right_list_items_rect_steps_down_one_row_height() {
    // Our own layout: 50-row items 125 wide from (101, 127).
    let (world, _, system, _, _) = fleet_with_regiment();
    let state = opened(&world, system);

    assert_eq!(
        state.item_screen_rect(scaled(), system, 1),
        Some(egui::Rect::from_min_max(at(101.0, 177.0), at(226.0, 227.0)))
    );
    assert_eq!(
        state.item_screen_rect(scaled(), system, 2),
        Some(egui::Rect::from_min_max(at(101.0, 227.0), at(226.0, 277.0)))
    );
    let mut other = world.clone();
    let missing = other.systems.insert(world.systems[system].clone());
    assert_eq!(state.item_screen_rect(scaled(), missing, 0), None);
}

#[test]
fn a_capital_ship_dragged_out_of_its_tab_drops_that_ship_where_the_button_comes_up() {
    // FUN_00422ce0 moves a type 4 selection with 0x201; manual p. 120:
    // "To move ships [...] from one fleet to another, simply drag the
    // item to their new destinations."
    let (world, transport, system, fleet, _) = fleet_with_regiment();

    let (actions, release, held) = drag_from_tab(
        &world,
        &transport,
        system,
        fleet,
        FleetWindowTab::CapitalShips,
        FIRST_RIGHT_ITEM,
        (40.0, 200.0),
    );

    assert!(held);
    assert!(regiment_drops(&actions).is_empty());
    assert_eq!(
        actions
            .iter()
            .filter(|action| matches!(action, FleetWindowAction::DragShip { .. }))
            .collect::<Vec<_>>(),
        [&FleetWindowAction::DragShip {
            fleet,
            ships: vec![0],
            roster: fleet_join::roster(&world, fleet).unwrap(),
            point: release,
            confirmed: false,
        }]
    );
}

#[test]
fn a_click_whose_pointer_moves_on_in_its_release_frame_drops_nothing() {
    // FUN_006083c0 case 0x202 measures the release by WM_LBUTTONUP's
    // own point (lParam), not where the pointer is when the frame is
    // read: a click on a fleet entry released in place is no 0x29a, so
    // no move of the fleet into itself, though the pointer moves on
    // past sqrt(24) list pixels before the frame runs.
    let (world, transport, system, fleet, _) = fleet_with_regiment();
    let entry = at(40.0, 40.0);
    let moved_on = at(46.0, 40.0);
    let mut state = opened(&world, system);
    let mut frames = hover(entry);
    frames.extend([
        vec![press(entry, true)],
        vec![press(entry, false), egui::Event::PointerMoved(moved_on)],
        vec![],
    ]);
    let clicked = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        frames,
    );
    assert!(
        !clicked.actions.iter().any(|action| matches!(
            action,
            FleetWindowAction::DragFleet { .. } | FleetWindowAction::DragShip { .. }
        )),
        "{:?}",
        clicked.actions
    );
    assert_eq!(
        state.windows[0].selected.single(),
        Some(FleetWindowEntry::Fleet(fleet))
    );

    // A release past sqrt(24) list pixels from the press drops at the
    // release's own point, though the pointer moves on.
    let released = at(46.0, 40.0);
    let mut state = opened(&world, system);
    let mut frames = hover(entry);
    frames.extend([
        vec![press(entry, true)],
        vec![egui::Event::PointerMoved(released)],
        vec![
            press(released, false),
            egui::Event::PointerMoved(at(300.0, 200.0)),
        ],
    ]);
    let dragged = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        frames,
    );
    assert!(dragged.actions.contains(&FleetWindowAction::DragFleet {
        fleets: vec![fleet],
        point: released,
        confirmed: false,
    }));
}

#[test]
fn a_fleet_dragged_out_of_the_left_list_drops_that_fleet_where_the_button_comes_up() {
    // fleet-window.md: a drag out of either list posts 0x29a, and
    // FUN_00422ce0 moves the type 4 selection with 0x201.
    let (world, transport, system, fleet, _) = fleet_with_regiment();
    let (actions, release, held) = drag_from_tab(
        &world,
        &transport,
        system,
        fleet,
        FleetWindowTab::CapitalShips,
        (40.0, 40.0),
        (300.0, 200.0),
    );
    assert!(held);
    assert_eq!(
        actions
            .iter()
            .filter(|action| matches!(action, FleetWindowAction::DragFleet { .. }))
            .collect::<Vec<_>>(),
        [&FleetWindowAction::DragFleet {
            fleets: vec![fleet],
            point: release,
            confirmed: false,
        }]
    );

    // A release inside the list it left drops too (flags 0,
    // FUN_004a4b10): the window's +0x70 then names what lies there.
    let (inside, release, _) = drag_from_tab(
        &world,
        &transport,
        system,
        fleet,
        FleetWindowTab::CapitalShips,
        (40.0, 40.0),
        (40.0, 200.0),
    );
    assert!(inside.contains(&FleetWindowAction::DragFleet {
        fleets: vec![fleet],
        point: release,
        confirmed: false,
    }));
}

#[test]
fn a_drop_released_with_ctrl_is_a_confirmed_move_from_either_list() {
    // FUN_00422ce0's 0x29a handler: GetAsyncKeyState(VK_CONTROL) makes
    // the order 0x202 instead of 0x201 for window types 4 and 10 alike.
    let (world, transport, system, fleet, troop) = fleet_with_regiment();
    let drop = |tab, from: (f32, f32), modifiers| {
        let mut state = opened(&world, system);
        if let Some(window) = state.window_mut(system) {
            window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
            window.tab = tab;
        }
        let (start, end) = (at(from.0, from.1), at(300.0, 200.0));
        let button = |point, pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        };
        let mut frames = hover(start);
        frames.extend([
            vec![button(start, true)],
            vec![egui::Event::PointerMoved(end)],
            vec![egui::Event::PointerMoved(end), button(end, false)],
        ]);
        run(&world, &transport, &mut state, CockpitFaction::Alliance, frames).actions
    };
    for (modifiers, confirmed) in [
        (egui::Modifiers::NONE, false),
        (egui::Modifiers::CTRL, true),
    ] {
        let left = drop(FleetWindowTab::CapitalShips, (40.0, 40.0), modifiers);
        assert!(
            left.iter().any(|action| *action
                == FleetWindowAction::DragFleet {
                    fleets: vec![fleet],
                    point: at(300.0, 200.0),
                    confirmed,
                }),
            "{left:?}"
        );
        let right = drop(FleetWindowTab::Troops, FIRST_RIGHT_ITEM, modifiers);
        assert!(
            right.iter().any(|action| *action
                == FleetWindowAction::DragRegiment {
                    troops: vec![troop],
                    point: at(300.0, 200.0),
                    confirmed,
                }),
            "{right:?}"
        );
    }
}

#[test]
fn a_capital_ship_item_stands_for_its_index_among_every_ship_of_the_fleet() {
    // Our own list: a destroyed ship is not listed but keeps its index.
    let (mut world, transport, _, fleet, _) = fleet_with_regiment();
    let class = world.fleets[fleet].capital_ships[0].class;
    world.fleets[fleet]
        .capital_ships
        .insert(0, ShipInstance::new(class, 100, true));
    world.fleets[fleet].capital_ships[0].alive = false;

    let items = right_items(
        &world,
        &transport,
        &[],
        Some(FleetWindowEntry::Fleet(fleet)),
        FleetWindowTab::CapitalShips,
    );

    assert_eq!(
        items.iter().map(|item| item.object).collect::<Vec<_>>(),
        [Some(ItemObject::Ship {
            fleet,
            index: 1,
            roster: fleet_join::roster(&world, fleet).unwrap(),
        })]
    );
}

fn right_click(
    world: &GameWorld,
    transport: &TroopTransportState,
    state: &mut FleetWindowState,
    point: egui::Pos2,
) -> Vec<FleetWindowAction> {
    let button = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let mut frames = hover(point);
    frames.extend([vec![button(true)], vec![button(false)], vec![]]);
    run(world, transport, state, CockpitFaction::Alliance, frames).actions
}

fn menus(actions: &[FleetWindowAction]) -> Vec<(MenuObject, (i16, i16))> {
    actions
        .iter()
        .filter_map(|action| match *action {
            FleetWindowAction::OpenObjectMenu {
                selection, point, ..
            } => Some((selection, point)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_right_click_on_a_ship_or_regiment_item_opens_its_menu_where_it_was_clicked() {
    // Manual p. 120: "Right-click on a ship to bring up its menu";
    // FUN_004ac5c0 opens the menu at the canvas point.
    let (world, transport, system, fleet, troop) = fleet_with_regiment();
    let mut state = opened(&world, system);
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
        window.tab = FleetWindowTab::CapitalShips;
    }

    let ship = right_click(&world, &transport, &mut state, at(160.0, 150.0));
    assert_eq!(
        menus(&ship),
        [(
            MenuObject::Ship {
                fleet,
                index: 0,
                roster: fleet_join::roster(&world, fleet).unwrap(),
            },
            (180, 180)
        )]
    );

    if let Some(window) = state.window_mut(system) {
        window.tab = FleetWindowTab::Troops;
    }
    let regiment = right_click(&world, &transport, &mut state, at(160.0, 150.0));
    assert_eq!(menus(&regiment), [(MenuObject::Troop(troop), (180, 180))]);
}

#[test]
fn a_right_click_on_a_fleet_entry_or_a_squadron_opens_its_menu() {
    // FUN_004a2c40 opens the pop-up menu (FUN_004ac5c0) for the hit
    // list's selected items: FUN_004ff8e0's fleet menu, and a squadron's
    // with FUN_0051d990's Encyclopedia and Status.
    let (mut world, transport, system, fleet, _) = fleet_with_regiment();
    for _ in 0..2 {
        let fighter = world.fighter_classes.insert(Default::default());
        world.fleets[fleet]
            .fighters
            .push(rebellion_core::world::FighterEntry {
                class: fighter,
                count: 1,
                carrier: None,
                damage: None,
            });
    }
    let mut state = opened(&world, system);
    let entry = state
        .entry_screen_rect(scaled(), system, 0)
        .unwrap()
        .center();

    let actions = right_click(&world, &transport, &mut state, entry);
    assert_eq!(
        menus(&actions)
            .into_iter()
            .map(|(selection, _)| selection)
            .collect::<Vec<_>>(),
        [MenuObject::Fleet(fleet)]
    );

    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
        window.tab = FleetWindowTab::Fighters;
    }
    let second = state
        .item_screen_rect(scaled(), system, 1)
        .unwrap()
        .center();
    let actions = right_click(&world, &transport, &mut state, second);
    assert_eq!(
        menus(&actions)
            .into_iter()
            .map(|(selection, _)| selection)
            .collect::<Vec<_>>(),
        [MenuObject::Fighter { fleet, index: 1 }]
    );
}

#[test]
fn a_selected_regiment_stays_selected_when_one_listed_above_it_leaves() {
    // The list's items are the objects (FUN_004a6e70 keeps the object
    // at +0x6c), so a regiment that lands moves no selection onto its
    // neighbour.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 3);
    let troops: Vec<TroopKey> = (0..3).map(|_| add_troop(&mut world, system)).collect();
    let mut transport = TroopTransportState::default();
    transport.embark(&mut world, fleet, &troops).unwrap();
    let mut state = opened(&world, system);
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
        window.tab = FleetWindowTab::Troops;
    }
    let listed = |world: &GameWorld, transport: &TroopTransportState| -> Vec<TroopKey> {
        right_items(world, transport, &[], Some(FleetWindowEntry::Fleet(fleet)), FleetWindowTab::Troops)
            .into_iter()
            .filter_map(|item| match item.object {
                Some(ItemObject::Regiment(troop)) => Some(troop),
                _ => None,
            })
            .collect()
    };
    let order = listed(&world, &transport);
    let point = state.item_screen_rect(scaled(), system, 1).unwrap().center();
    let _ = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        click_with(point, egui::Modifiers::NONE),
    );
    // The first listed regiment lands; the selected one is now first.
    transport
        .disembark_selected(&mut world, fleet, system, &[order[0]])
        .unwrap();
    let _ = run(&world, &transport, &mut state, CockpitFaction::Alliance, hover(point));
    let report = state
        .report(&world, &MovementState::default(), &fog(system), &transport, system)
        .unwrap();
    assert_eq!(listed(&world, &transport)[0], order[1]);
    assert_eq!(report.selected_items, [0]);
}

#[test]
fn a_menu_on_one_of_several_selected_squadrons_of_a_class_takes_them_all() {
    // FUN_004ac5c0 takes the list's whole selection (slot +0x58): three
    // X-wing squadrons selected with Ctrl are three team members,
    // though the port's squadrons share their class entry.
    let (mut world, transport, system, fleet, _) = fleet_with_regiment();
    let fighter = world.fighter_classes.insert(Default::default());
    world.fleets[fleet]
        .fighters
        .push(rebellion_core::world::FighterEntry {
            class: fighter,
            count: 3,
            carrier: None,
            damage: None,
        });
    let mut state = opened(&world, system);
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
        window.tab = FleetWindowTab::Fighters;
    }
    for (row, modifiers) in [
        (0, egui::Modifiers::NONE),
        (1, egui::Modifiers::CTRL),
        (2, egui::Modifiers::CTRL),
    ] {
        let point = state.item_screen_rect(scaled(), system, row).unwrap().center();
        let _ = run(
            &world,
            &transport,
            &mut state,
            CockpitFaction::Alliance,
            click_with(point, modifiers),
        );
    }
    let point = state.item_screen_rect(scaled(), system, 1).unwrap().center();
    let teams: Vec<Vec<MenuObject>> = right_click(&world, &transport, &mut state, point)
        .into_iter()
        .filter_map(|action| match action {
            FleetWindowAction::OpenObjectMenu { team, .. } => Some(team),
            _ => None,
        })
        .collect();
    assert_eq!(teams, [vec![MenuObject::Fighter { fleet, index: 0 }; 3]]);
}

#[test]
fn the_windows_rects_follow_the_canvas_offset_and_scale() {
    // Our own layout: the list's 50-row items (FUN_004a3d40) and the tab
    // strip at (99 + x, 96), 31 by 28.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    let mut other_system = world.systems[system].clone();
    other_system.fleets.clear();
    let other = world.systems.insert(other_system);
    let missing = world.systems.insert(world.systems[other].clone());
    world.systems.remove(missing);
    let mut state = opened(&world, system);
    assert!(state.open(
        &world,
        other,
        (300, 100),
        CockpitFaction::Alliance,
        scaled()
    ));

    assert_eq!(state.window_count(), 2);
    assert!(state.is_open(system) && state.is_open(other) && !state.is_open(missing));
    assert_eq!(
        state.entry_screen_rect(scaled(), system, 2),
        Some(egui::Rect::from_min_size(
            at(4.0, 129.0),
            egui::vec2(182.0, 100.0)
        ))
    );
    assert_eq!(
        state.tab_screen_rect(scaled(), system, FleetWindowTab::Troops),
        Some(egui::Rect::from_min_size(
            at(164.0, 96.0),
            egui::vec2(62.0, 56.0)
        ))
    );
    assert_eq!(
        state.screen_rect(scaled(), system),
        Some(egui::Rect::from_min_size(
            at(0.0, 0.0),
            egui::vec2(FLEET_WINDOW_WIDTH * 2.0, FLEET_WINDOW_HEIGHT * 2.0)
        ))
    );
    assert_eq!(state.screen_rect(scaled(), missing), None);
    assert_eq!(state.entry_screen_rect(scaled(), missing, 0), None);
    assert_eq!(
        state.tab_screen_rect(scaled(), missing, FleetWindowTab::Troops),
        None
    );
    assert_eq!(state.selection(missing), None);
    assert_eq!(
        state.selection(system),
        Some((None, FleetWindowTab::CapitalShips))
    );
    let point = |p: egui::Pos2| (p.x, p.y);
    assert!(state.contains_screen_point(scaled(), point(at(234.0, 303.0))));
    assert!(!state.contains_screen_point(scaled(), point(at(235.5, 150.0))));
    assert!(!state.contains_screen_point(scaled(), point(at(-0.5, 150.0))));

    let fog = fog(system);
    let layer = egui::LayerId::new(egui::Order::Foreground, area_id(system));
    assert_eq!(
        state.release_target(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog,
            scaled(),
            layer,
            at(60.0, 40.0)
        ),
        Some(Some(ReleaseTarget::Fleet { fleet, system }))
    );
    // Below the only entry: the system.
    assert_eq!(
        state.release_target(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog,
            scaled(),
            layer,
            at(10.0, 90.0)
        ),
        Some(Some(ReleaseTarget::System(system)))
    );
    let elsewhere = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("elsewhere"));
    assert_eq!(
        state.release_target(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog,
            scaled(),
            elsewhere,
            at(10.0, 40.0)
        ),
        None
    );

    state.clear();
    assert_eq!(state.window_count(), 0);
}

#[test]
fn opening_a_window_for_the_other_side_closes_the_first_sides_windows() {
    // port: the port keeps one player's windows at a time.
    let (world, system) = world(ControlKind::Uncontrolled);
    let mut state = opened(&world, system);
    let other = world.systems.keys().next().unwrap();
    assert!(state.open(&world, other, ORIGIN, CockpitFaction::Empire, scaled()));
    assert_eq!(state.window_count(), 1);
    assert_eq!(state.faction, CockpitFaction::Empire);
}

#[test]
fn a_click_selects_a_fleet_then_a_lit_tab_and_the_close_button_closes_the_window() {
    // FUN_004a4b10: the list selection fills the right pane; control
    // 0x277c closes.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 2);
    let troop = add_troop(&mut world, system);
    let mut transport = TroopTransportState::default();
    transport.embark(&mut world, fleet, &[troop]).unwrap();
    let mut state = opened(&world, system);

    let selected = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        click(at(40.0, 50.0)),
    );
    assert_eq!(
        state.selection(system),
        Some((
            Some(FleetWindowEntry::Fleet(fleet)),
            FleetWindowTab::CapitalShips
        ))
    );
    assert!(selected
        .actions
        .contains(&FleetWindowAction::SelectSystem(system)));

    run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        click(at(115.0, 110.0)),
    );
    assert_eq!(
        state.selection(system).unwrap().1,
        FleetWindowTab::CapitalShips
    );
    run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        click(at(180.0, 110.0)),
    );
    assert_eq!(state.selection(system).unwrap().1, FleetWindowTab::Troops);
    assert_eq!(
        state
            .report(
                &world,
                &rebellion_core::movement::MovementState::default(),
                &fog(system),
                &transport,
                system
            )
            .unwrap()
            .items,
        ["Alliance Fleet Regiment"]
    );

    run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        click(at(225.0, 10.0)),
    );
    assert!(!state.is_open(system));
    assert_eq!(
        state.report(
            &world,
            &rebellion_core::movement::MovementState::default(),
            &fog(system),
            &transport,
            system
        ),
        None
    );
}

#[test]
fn a_disabled_tab_takes_the_click_and_shows_its_empty_list_selected() {
    // A0 menus/23-fleet1-fighters and -personnel: Fleet 1, with no
    // squadron or character aboard, shows the clicked Fighters and
    // Personnel tabs' empty lists, each button in its selected art
    // (10417, 10419), the others normal (10409) or disabled.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 2);
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    let tabs = |state: &mut FleetWindowState, x: f32| {
        run(&world, &transport, state, CockpitFaction::Alliance, click(at(x, 110.0)));
        state.selection(system).unwrap().1
    };
    run(&world, &transport, &mut state, CockpitFaction::Alliance, click(at(40.0, 50.0)));
    assert_eq!(state.selection(system).unwrap().0, Some(FleetWindowEntry::Fleet(fleet)));
    assert!(!tab_enabled(&world, &transport, &[], Some(FleetWindowEntry::Fleet(fleet)), FleetWindowTab::Fighters));
    assert_eq!(tabs(&mut state, 147.0), FleetWindowTab::Fighters);
    let report = state
        .report(&world, &rebellion_core::movement::MovementState::default(), &fog(system), &transport, system)
        .unwrap();
    assert!(report.items.is_empty(), "{:?}", report.items);
    assert_eq!(report.counts, Some((0, 0)));
    assert_eq!(tabs(&mut state, 211.0), FleetWindowTab::Personnel);
    assert_eq!(tab_resource(10409, FleetWindowTab::Fighters, true, false), 10417);
    assert_eq!(tab_resource(10409, FleetWindowTab::Personnel, true, false), 10419);
    assert_eq!(tab_resource(10409, FleetWindowTab::Fighters, false, false), 10413);
    assert_eq!(tab_resource(10409, FleetWindowTab::CapitalShips, false, true), 10409);
}

#[test]
fn a_ship_s_indicators_mark_the_regiments_it_holds_by_capacity_in_fleet_order() {
    // A0 menus/23-fleet1-ships: Fleet 1's two regiments mark Medium
    // Transport 1 (CAPSHPSD troop capacity 2) with 10405, not
    // Corellian Corvette 1 (capacity 0) (FUN_004a66a0 flag 2,
    // FUN_004a67a0).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 3);
    let ship = world.fleets[fleet].capital_ships[0].clone();
    world.fleets[fleet].capital_ships.extend([ship.clone(), ship]);
    let capacities = [0, 2, 2];
    for (index, capacity) in capacities.into_iter().enumerate() {
        let mut class = world.capital_ship_classes[world.fleets[fleet].capital_ships[index].class].clone();
        class.troop_capacity = capacity;
        class.fighter_capacity = 0;
        let key = world.capital_ship_classes.insert(class);
        world.fleets[fleet].capital_ships[index].class = key;
    }
    let mut transport = TroopTransportState::default();
    assert_eq!(ship_contents(&world, &transport, &[], fleet), [0, 0, 0]);
    let troops: Vec<_> = (0..3).map(|_| add_troop(&mut world, system)).collect();
    transport.embark(&mut world, fleet, &troops[..2]).unwrap();
    // Two fill the second ship and leave none for the third.
    assert_eq!(ship_contents(&world, &transport, &[], fleet), [0, CONTENT_REGIMENTS, 0]);
    transport.embark(&mut world, fleet, &troops[2..]).unwrap();
    // The third goes to the third.
    assert_eq!(
        ship_contents(&world, &transport, &[], fleet),
        [0, CONTENT_REGIMENTS, CONTENT_REGIMENTS]
    );
    world.fleets[fleet].capital_ships[1].alive = false;
    assert_eq!(ship_contents(&world, &transport, &[], fleet), [0, 0, CONTENT_REGIMENTS]);
}

#[test]
fn ships_on_their_way_to_join_a_fleet_are_its_members_in_hyperspace() {
    // Manual p. 122: a ship moved onto a fleet in another sector "will
    // immediately be considered a member of the fleet but will still be
    // in hyperspace"; FUN_004ffc90 routes each ship to the target fleet
    // and FUN_004feca0 reparents it at once, so the target's entry
    // expands to it and its tabs list it and its cargo, en route
    // (FUN_004f8240), and its slots count (FUN_004a6be0).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let mut far = world.systems[system].clone();
    far.name = "Bespin".into();
    far.x += 40;
    let elsewhere = world.systems.insert(far);
    let target = add_fleet(&mut world, system, true, 2);
    let mover = add_fleet(&mut world, elsewhere, true, 1);
    world.systems[elsewhere].fleets.clear();
    let troop = add_troop(&mut world, elsewhere);
    let mut transport = TroopTransportState::default();
    transport.embark(&mut world, mover, &[troop]).unwrap();
    let mut movement = MovementState::default();
    assert!(movement.order(mover, elsewhere, system, 5));
    movement.set_join(mover, target);
    // A fleet on a plain move to the system joins nothing: it is listed
    // as itself.
    let passer = add_fleet(&mut world, elsewhere, true, 0);
    world.systems[elsewhere].fleets.clear();
    assert!(movement.order(passer, elsewhere, system, 5));
    assert_eq!(joining_fleets(&world, &movement, target), [mover]);
    let mut open = window(system, Some(FleetWindowEntry::Fleet(target)));
    open.expanded.push(target);

    assert_eq!(
        left_entries(&world, &movement, &fog(system), Faction::Alliance, &open),
        [
            FleetWindowEntry::Fleet(target),
            FleetWindowEntry::Ship { fleet: target, index: 0 },
            FleetWindowEntry::Ship { fleet: mover, index: 0 },
            FleetWindowEntry::Fleet(passer),
        ]
    );
    let selected = open.selected.single();
    let ships = window_items(&world, &movement, &transport, &[], selected, FleetWindowTab::CapitalShips);
    assert_eq!(
        ships.iter().map(|item| item.en_route).collect::<Vec<_>>(),
        [false, true]
    );
    assert!(matches!(ships[1].object, Some(ItemObject::Ship { fleet, index: 0, .. }) if fleet == mover));
    let troops = window_items(&world, &movement, &transport, &[], selected, FleetWindowTab::Troops);
    assert_eq!(troops.len(), 1);
    assert_eq!(troops[0].object, Some(ItemObject::Regiment(troop)));
    assert!(troops[0].en_route);
    assert!(!tab_enabled(&world, &transport, &[], selected, FleetWindowTab::Troops));
    assert!(window_tab_enabled(&world, &movement, &transport, &[], selected, FleetWindowTab::Troops));
    // Transport capacities 2 and 1, one regiment aboard the mover.
    assert_eq!(
        window_tab_counts(&world, &movement, &transport, selected, FleetWindowTab::Troops),
        Some((1, 3))
    );

    // A target that leaves is no longer joined on arrival: the ships
    // are their own fleet again.
    assert!(movement.order(target, system, elsewhere, 5));
    open.expanded.push(mover);
    let entries = left_entries(&world, &movement, &fog(system), Faction::Alliance, &open);
    assert!(entries.contains(&FleetWindowEntry::Fleet(mover)), "{entries:?}");
    assert_eq!(
        window_items(&world, &movement, &transport, &[], selected, FleetWindowTab::CapitalShips).len(),
        1
    );
}

#[test]
fn a_unit_on_its_way_aboard_boards_the_ship_with_the_most_room_left() {
    // FUN_00552300 for a fleet destination tries the fleet's usable
    // ships (FUN_00502e30(fleet, 1, side)) in order; FUN_00552b10 keeps
    // a later one only when FUN_0054fbb0 finds it strictly better:
    // unlimited room, else more room left after the move (+8,
    // FUN_0054fca0; the room is FUN_00500b40's capacity less the
    // mode-3 count). Personnel are unlimited (0x30..0x40), so they
    // board the first ship. A selected ship lists and counts what was
    // seated on it (FUN_004a6be0, CapShip_vt23c / CapShip_vt240).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let ship = world.fleets[fleet].capital_ships[0].clone();
    world.fleets[fleet].capital_ships.extend([ship.clone(), ship]);
    for (index, (troops, hangar)) in [(2, 0), (2, 1), (1, 2)].into_iter().enumerate() {
        let mut class =
            world.capital_ship_classes[world.fleets[fleet].capital_ships[index].class].clone();
        class.troop_capacity = troops;
        class.fighter_capacity = hangar;
        let key = world.capital_ship_classes.insert(class);
        world.fleets[fleet].capital_ships[index].class = key;
        world.fleets[fleet].capital_ships[index].id = 1 + index as u64;
    }
    let xwing = world
        .fighter_classes
        .insert(rebellion_core::world::FighterClass {
            dat_id: DatId::new(0x1c00_0003),
            name: "X-wing".into(),
            is_alliance: true,
            ..Default::default()
        });
    let own = add_troop(&mut world, system);
    let incoming: Vec<_> = (0..2).map(|_| add_troop(&mut world, system)).collect();
    let mut transport = TroopTransportState::default();
    transport.embark(&mut world, fleet, &[own]).unwrap();
    let mut value = serde_json::to_value(&transport).unwrap();
    for &troop in &incoming {
        value["transit"].as_array_mut().unwrap().push(
            serde_json::to_value(rebellion_core::troop_transport::RegimentTransit {
                troop,
                origin: system,
                leg: RegimentLeg::Fleet(fleet),
                arrival_tick: 9,
            })
            .unwrap(),
        );
    }
    value["squadrons"]["transits"].as_array_mut().unwrap().push(
        serde_json::to_value(rebellion_core::squadrons::SquadronTransit {
            class: xwing,
            count: 2,
            origin: system,
            to: SquadronPlace::Fleet(fleet),
            arrival_tick: 9,
            lost: Vec::new(),
        })
        .unwrap(),
    );
    let transport: TroopTransportState = serde_json::from_value(value).unwrap();
    let character = world.characters.insert(rebellion_core::world::Character {
        is_alliance: true,
        ..Default::default()
    });
    let arrivals = [(MissionMember::Character(character), fleet)];

    let load = ship_load(&world, &transport, &arrivals, fleet);
    // The fleet's own regiment sits on the first ship (rooms 1, 2, 1):
    // the first on its way takes the second ship (2 left), the next
    // the first of three equals (1, 1, 1).
    assert_eq!(load[0].regiments, [own, incoming[1]]);
    assert_eq!(load[1].regiments, [incoming[0]]);
    assert!(load[2].regiments.is_empty());
    // Hangars 0, 1, 2: the first squadron takes the third ship, the
    // second the first of equals, the second ship.
    let seats: Vec<_> = load.iter().map(|each| each.incoming_squadrons.clone()).collect();
    assert_eq!(seats, [vec![], vec![1], vec![0]]);
    assert_eq!(load[0].characters, [character]);
    assert_eq!(
        ship_contents(&world, &transport, &arrivals, fleet),
        [
            CONTENT_REGIMENTS | CONTENT_PERSONNEL,
            CONTENT_REGIMENTS | CONTENT_SQUADRONS,
            CONTENT_SQUADRONS
        ]
    );
    // The battle's seating keeps only what is aboard.
    assert!(ship_cargo(&world, &transport, fleet)[1].regiments.is_empty());

    let ship = |index| Some(FleetWindowEntry::Ship { fleet, index });
    let regiments = |index| {
        right_items(&world, &transport, &arrivals, ship(index), FleetWindowTab::Troops)
            .into_iter()
            .map(|item| (item.object, item.en_route))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        regiments(0),
        [
            (Some(ItemObject::Regiment(own)), false),
            (Some(ItemObject::Regiment(incoming[1])), true)
        ]
    );
    assert_eq!(regiments(1), [(Some(ItemObject::Regiment(incoming[0])), true)]);
    assert_eq!(tab_counts(&world, &transport, ship(0), FleetWindowTab::Troops), Some((2, 2)));
    assert_eq!(tab_counts(&world, &transport, ship(2), FleetWindowTab::Fighters), Some((1, 2)));
    let squadrons = |index| {
        right_items(&world, &transport, &arrivals, ship(index), FleetWindowTab::Fighters).len()
    };
    assert_eq!([squadrons(0), squadrons(1), squadrons(2)], [0, 1, 1]);
    let personnel = right_items(&world, &transport, &arrivals, ship(0), FleetWindowTab::Personnel);
    assert_eq!(personnel.len(), 1);
    assert!(personnel[0].en_route);
    assert!(tab_enabled(&world, &transport, &arrivals, ship(2), FleetWindowTab::Fighters));
    assert!(!tab_enabled(&world, &transport, &arrivals, ship(2), FleetWindowTab::Troops));
    // A destroyed first ship is not usable (mode 1): the member boards
    // the next.
    world.fleets[fleet].capital_ships[0].alive = false;
    let load = ship_load(&world, &transport, &arrivals, fleet);
    assert!(load[0].characters.is_empty());
    assert_eq!(load[1].characters, [character]);
}

// hyp (as the regiments): squadrons fill the live ships' fighter
// capacities in fleet order.
#[test]
fn squadrons_fill_the_ships_fighter_capacities_in_fleet_order() {
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 3);
    let ship = world.fleets[fleet].capital_ships[0].clone();
    world.fleets[fleet].capital_ships.extend([ship.clone(), ship]);
    for index in 0..3 {
        let mut class =
            world.capital_ship_classes[world.fleets[fleet].capital_ships[index].class].clone();
        class.troop_capacity = 0;
        class.fighter_capacity = 1;
        let key = world.capital_ship_classes.insert(class);
        world.fleets[fleet].capital_ships[index].class = key;
    }
    let fighter = world
        .fighter_classes
        .insert(rebellion_core::world::FighterClass::default());
    world.fleets[fleet].fighters = vec![rebellion_core::world::FighterEntry {
        class: fighter,
        count: 2,
        carrier: None,
        damage: None,
    }];
    let transport = TroopTransportState::default();
    let squadrons = ship_cargo(&world, &transport, fleet)
        .iter()
        .map(|cargo| cargo.squadrons)
        .collect::<Vec<_>>();
    assert_eq!(squadrons, [1, 1, 0]);
    assert_eq!(
        ship_contents(&world, &transport, &[], fleet),
        [CONTENT_SQUADRONS, CONTENT_SQUADRONS, 0]
    );
}

#[test]
fn a_character_marks_the_ship_it_rides_on_and_an_unseated_one_the_first() {
    // A0 fleet2/vjun-fleet5-admiral-ships: Garm Bel Iblis aboard Fleet 5
    // marks Corellian Corvette 2 with 10406 (FUN_004a66a0 flag 4,
    // FUN_00536da0); seat_characters seats an unseated one on the first
    // living ship (FUN_00552300).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let ship = world.fleets[fleet].capital_ships[0].clone();
    world.fleets[fleet].capital_ships.extend([ship.clone(), ship]);
    for (index, ship) in world.fleets[fleet].capital_ships.iter_mut().enumerate() {
        ship.id = index as u64 + 1;
    }
    let transport = TroopTransportState::default();
    let rider = world.characters.insert(rebellion_core::world::Character {
        name: "Bel Iblis".into(),
        ship: Some(3),
        ..Default::default()
    });
    world.fleets[fleet].characters.push(rider);
    assert_eq!(ship_contents(&world, &transport, &[], fleet), [0, 0, CONTENT_PERSONNEL]);
    let unseated = world.characters.insert(rebellion_core::world::Character {
        name: "Mothma".into(),
        ..Default::default()
    });
    world.fleets[fleet].characters.push(unseated);
    assert_eq!(
        ship_contents(&world, &transport, &[], fleet),
        [CONTENT_PERSONNEL, 0, CONTENT_PERSONNEL]
    );
    world.fleets[fleet].capital_ships[0].alive = false;
    assert_eq!(
        ship_contents(&world, &transport, &[], fleet),
        [0, CONTENT_PERSONNEL, CONTENT_PERSONNEL]
    );
}

#[test]
fn the_title_buttons_minimize_the_window_and_open_its_sector() {
    // FUN_004a4b10: 0x280d minimizes to the rail (FUN_004a76e0); 0xca
    // restores the sector window.
    let (world, system) = world(ControlKind::Uncontrolled);
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    let sector = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        click(at(10.0, 10.0)),
    );
    assert!(sector
        .actions
        .contains(&FleetWindowAction::OpenSector(system)));
    assert!(state.is_open(system));

    let minimized = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        click(at(211.0, 10.0)),
    );
    assert!(minimized.actions.contains(&FleetWindowAction::Minimize {
        system,
        logical_position: ORIGIN,
    }));
    assert!(!state.is_open(system));
}

#[test]
fn a_click_on_a_back_windows_background_brings_it_forward() {
    // FUN_0045aac0's focus: the clicked child becomes the active one.
    let (mut world, system) = world(ControlKind::Uncontrolled);
    let copy = world.systems[system].clone();
    let other = world.systems.insert(copy);
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    assert!(state.open(
        &world,
        other,
        (400, 300),
        CockpitFaction::Alliance,
        scaled()
    ));
    assert_eq!(state.windows.last().unwrap().system, other);

    let focused = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        click(at(150.0, 200.0)),
    );
    assert_eq!(state.windows.last().unwrap().system, system);
    assert!(focused
        .actions
        .contains(&FleetWindowAction::SelectSystem(system)));
}

#[test]
fn a_double_click_counts_only_when_both_clicks_land_close_in_the_list() {
    // Win32 posts WM_LBUTTONDBLCLK to the window under both clicks only
    // within the 4 by 4 double-click rectangle (SM_CXDOUBLECLK). A click
    // on another window (a popup menu) and then on a fleet's entry, or
    // two clicks 3 pixels apart, are two single clicks.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    add_fleet(&mut world, system, true, 1);
    let transport = TroopTransportState::default();
    let entries = |state: &FleetWindowState| {
        state
            .report(
                &world,
                &rebellion_core::movement::MovementState::default(),
                &fog(system),
                &transport,
                system,
            )
            .unwrap()
            .entries
    };
    let clicks = |points: &[egui::Pos2]| {
        let mut state = opened(&world, system);
        let frames = points.iter().flat_map(|&point| click(point)).collect();
        run(
            &world,
            &transport,
            &mut state,
            CockpitFaction::Alliance,
            frames,
        );
        entries(&state)
    };
    // Each click 0.25 s after the last, inside egui's 0.3 s.
    let elsewhere = egui::pos2(1300.0, 900.0);
    assert_eq!(clicks(&[elsewhere, at(40.0, 50.0)]), 1);
    assert_eq!(clicks(&[at(40.0, 50.0), at(43.0, 50.0)]), 1);
    assert_eq!(clicks(&[at(40.0, 50.0), at(40.0, 53.0)]), 1);
    assert_eq!(clicks(&[at(40.0, 50.0), at(42.0, 52.0)]), 2);
    // egui takes the third click for a triple; the list still sees its
    // double click.
    assert_eq!(clicks(&[elsewhere, at(40.0, 50.0), at(40.0, 50.0)]), 2);
    // A double click ends the pair: a third click starts another.
    assert_eq!(clicks(&[at(40.0, 50.0), at(40.0, 50.0), at(40.0, 50.0)]), 2);
}

#[test]
fn a_double_click_on_a_fleet_lists_its_ships_and_another_hides_them() {
    // FUN_004a3d40: a fleet's entry expands into its capital ships.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    add_fleet(&mut world, system, true, 1);
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    let entries = |state: &FleetWindowState| {
        state
            .report(
                &world,
                &rebellion_core::movement::MovementState::default(),
                &fog(system),
                &transport,
                system,
            )
            .unwrap()
            .entries
    };

    run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        double_click(at(40.0, 50.0)),
    );
    assert_eq!(entries(&state), 2);
    run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        double_click(at(40.0, 50.0)),
    );
    assert_eq!(entries(&state), 1);
}

#[test]
fn the_window_writes_its_names_and_counts_where_the_original_does() {
    // FUN_004a4b10: the title label 5 past the restore button, black; the
    // selected name centred at x 164, red (0x20000ff) for side 1 and green
    // otherwise; FUN_004a5c00's counts in the picture panel. The list
    // label's place is A0 system-windows/09-fleet's ("Fleet 4": pen 7 in,
    // line 3 down in its row); the right list's centred names are our
    // own.
    for (faction, color) in [
        (CockpitFaction::Alliance, egui::Color32::from_rgb(255, 0, 0)),
        (CockpitFaction::Empire, egui::Color32::from_rgb(0, 255, 0)),
    ] {
        let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
        crate::viewed_contents::see_live(&mut world, Faction::Empire, system);
        let fleet = add_fleet(&mut world, system, true, 2);
        let troop = add_troop(&mut world, system);
        let mut transport = TroopTransportState::default();
        transport.embark(&mut world, fleet, &[troop]).unwrap();
        let mut state = FleetWindowState::default();
        assert!(state.open(&world, system, ORIGIN, faction, scaled()));
        if let Some(window) = state.window_mut(system) {
            window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
            window.tab = FleetWindowTab::Troops;
        }

        let texts = run(
            &world,
            &transport,
            &mut state,
            faction,
            hover(at(-50.0, 0.0)),
        )
        .texts;
        let white = egui::Color32::WHITE;
        use crate::theme::GdiAlign::{Center, Left, Right};
        // FUN_004a4b10: the lists and counts in font 10, the selected
        // label +0x190 at (100, 29), 128 by 25, left and top, in font 5.
        // DrawTextA sets each line from its box's top.
        let font10 = crate::theme::gdi_font(10, 2.0).size;
        let font5 = crate::theme::gdi_font(5, 2.0).size;
        for (label, rect, font, align, size, color) in [
            ("Fleet 1", (11.0, 32.0, 84.0, 14.0), 10, Left, font10, white),
            ("Fleet 1", (100.0, 29.0, 128.0, 25.0), 5, Left, font5, color),
            ("1", (101.0, 44.0, 61.0, 14.0), 10, Left, font10, white),
            ("2", (162.0, 44.0, 61.0, 14.0), 10, Right, font10, white),
            (
                "Alliance Fleet Regiment",
                (101.0, 164.0, 125.0, 14.0),
                10,
                Center,
                font10,
                white,
            ),
        ] {
            assert!(
                texts.iter().any(|text| text.text == label
                    && text.set_in(rect, font, align)
                    && text.size == size
                    && text.color == color),
                "{faction:?}: {label} in {rect:?} in {texts:?}"
            );
        }
        // The label is clipped to its box (A0 fleet-aboard/10).
        assert!(texts.iter().any(|text| text.size == font5
            && text.text == "Fleet 1"
            && text.clip == egui::Rect::from_min_max(at(100.0, 29.0), at(228.0, 54.0))));
        // FUN_004a4b10: the title from x 19, centred in rows 2..18
        // (DT_VCENTER: (16 - 16) / 2, then tmAscent 13), font 5, black.
        let title = texts.iter().find(|text| text.text == "Sluis Van").unwrap();
        assert_eq!(
            (title.pos, title.size, title.color),
            (
                egui::pos2(at(19.0, 0.0).x, at(0.0, 2.0 + 13.0).y),
                crate::theme::gdi_font(5, 2.0).size,
                egui::Color32::BLACK
            )
        );
    }
}

#[test]
fn expanding_an_entry_marks_only_that_fleet_in_that_open_window() {
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let other = add_fleet(&mut world, system, true, 0);
    let mut state = opened(&world, system);
    assert!(!state.is_expanded(system, fleet));
    assert!(state.expand(system, fleet));
    assert!(state.expand(system, fleet));
    assert!(state.is_expanded(system, fleet));
    assert!(!state.is_expanded(system, other));
    assert_eq!(state.window_mut(system).unwrap().expanded, [fleet]);
    // A system with no open window.
    let copy = world.systems[system].clone();
    let elsewhere = world.systems.insert(copy);
    assert!(!state.expand(elsewhere, fleet));
    assert!(!state.is_expanded(elsewhere, fleet));
}

#[test]
fn a_selected_ship_is_framed_named_and_shows_three_tabs_lit_by_its_own_cargo() {
    // A0 fleet2/umgul-corvette1-selected and -transport1-selected: the
    // ship's entry takes 10401, the label +0x190 its name, the strip
    // drops Capital Ships, and only the transport's Troops tab is lit.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let mut class = world.capital_ship_classes[world.fleets[fleet].capital_ships[0].class].clone();
    class.name = "Medium Transport".into();
    class.troop_capacity = 2;
    let carrier = world.capital_ship_classes.insert(class);
    world.fleets[fleet].capital_ships.push(ShipInstance::new(carrier, 100, true));
    let mut transport = TroopTransportState::default();
    let troops: Vec<_> = (0..2).map(|_| add_troop(&mut world, system)).collect();
    transport.embark(&mut world, fleet, &troops).unwrap();
    let ship = |index| Some(FleetWindowEntry::Ship { fleet, index });
    let ships = |index| [FleetWindowEntry::Ship { fleet, index }];
    assert!(!tab_enabled(&world, &transport, &[], ship(0), FleetWindowTab::Troops));
    assert!(tab_enabled(&world, &transport, &[], ship(1), FleetWindowTab::Troops));
    assert!(!tab_enabled(&world, &transport, &[], ship(1), FleetWindowTab::Personnel));
    assert_eq!(
        shown_tabs(&ships(1)).collect::<Vec<_>>(),
        [FleetWindowTab::Fighters, FleetWindowTab::Troops, FleetWindowTab::Personnel]
    );
    assert_eq!(shown_tabs(&[FleetWindowEntry::Fleet(fleet)]).count(), 4);
    // Squadrons fill the fighter capacity of the ship that has it.
    let squadron = world.fighter_classes.insert(Default::default());
    world.fleets[fleet].fighters.push(rebellion_core::world::FighterEntry {
        class: squadron,
        count: 1,
        carrier: None,
        damage: None,
    });
    assert!(!tab_enabled(&world, &transport, &[], ship(1), FleetWindowTab::Fighters));
    world.capital_ship_classes[carrier].fighter_capacity = 1;
    assert!(tab_enabled(&world, &transport, &[], ship(1), FleetWindowTab::Fighters));
    assert!(!tab_enabled(&world, &transport, &[], ship(0), FleetWindowTab::Fighters));

    let mut state = opened(&world, system);
    state.select(system, ship(1).unwrap());
    let output = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        hover(at(-50.0, 0.0)),
    );
    // Rows of 50 from y 29: the fleet, its first ship, the transport.
    assert!(output.painted.contains(&(FLEET_SELECTED, at(4.0, 129.0))));
    assert!(!output.painted.contains(&(FLEET_SELECTED, at(4.0, 29.0))));
    // No art where Capital Ships' tab stands (TAB_STRIP + TAB_X[0]).
    assert!(!output.painted.iter().any(|&(_, point)| point == at(100.0, 96.0)));
    assert!(output.painted.iter().any(|&(_, point)| point == at(132.0, 96.0)));
    let font5 = crate::theme::gdi_font(5, 2.0).size;
    // Clipped to its box: A0 fleet-aboard/10 cuts a long name at x 228.
    let clip = egui::Rect::from_min_max(at(100.0, 29.0), at(228.0, 54.0));
    assert!(
        output.texts.iter().any(|text| text.text == "Medium Transport"
            && text.set_in((100.0, 29.0, 128.0, 25.0), 5, crate::theme::GdiAlign::Left)
            && text.size == font5
            && text.color == label_color(1)
            && text.clip == clip),
        "{:?}",
        output.texts
    );
}

// FUN_004a6980 -> FUN_0060da10(strip, pane, 2, 67): each tab's backdrop
// is the strip's copy of the pane read from (0, 0), so its column c
// shows the pane at window x 99 + c. A0 fleet2/vjun-fleet5-admiral-*
// show it in the 30-wide Capital Ships and Personnel tabs' column 30:
// black at (130, 96) and (226, 122), where the pane itself is (59, 59,
// 59) and (30, 29, 24).
#[test]
fn each_tab_shows_the_pane_from_the_strips_left_edge_under_its_art() {
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
    }
    let painted = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        hover(at(-50.0, 0.0)),
    )
    .painted;
    for (pane_x, tab_x) in [
        (98.0, 100.0),
        (130.0, 132.0),
        (162.0, 164.0),
        (194.0, 196.0),
    ] {
        let pane = painted
            .iter()
            .position(|&entry| entry == (PANE_ART, at(pane_x, 29.0)));
        let art = painted
            .iter()
            .position(|&(id, point)| (10409..=10419).contains(&id) && point == at(tab_x, 96.0));
        assert!(
            pane.zip(art).is_some_and(|(pane, art)| pane < art),
            "{pane_x}: {pane:?} {art:?} in {painted:?}"
        );
    }
}

#[test]
fn an_enemy_fleet_selected_shows_the_other_sides_art() {
    // FUN_004a4b10 / FUN_004a3340: the title strip, pane art, selection
    // frame, picture and tab strip of the other side (Alliance + 50) when
    // the viewer's side has no fleet listed; the no-hyperdrive mark 10430
    // and the selected item 10420 take the fleet's side too.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    crate::viewed_contents::see_live(&mut world, Faction::Alliance, system);
    let enemy = add_fleet(&mut world, system, false, 0);
    let class = world.fleets[enemy].capital_ships[0].class;
    world.capital_ship_classes[class].hyperdrive = 0;
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(enemy));
        window.selected_items = ListSelection::only(keys_of(&world, window, &[0])[0]);
        window.expanded.push(enemy);
    }

    let painted = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        hover(at(-50.0, 0.0)),
    )
    .painted;
    for (id, x, y) in [
        (10201, 2.0, 2.0),
        (10457, 97.0, 29.0),
        (10451, 4.0, 29.0),
        (10450, 9.0, 34.0),
        (10475, 100.0, 42.0),
        (10466, 100.0, 96.0),
        (10463, 132.0, 96.0),
        (10480, 13.0, 112.0),
        (10470, 102.0, 128.0),
        (10480, 110.0, 150.0),
        (CLOSE_NORMAL, 218.0, 3.0),
    ] {
        assert!(
            painted.contains(&(id, at(x, y))),
            "{id} at ({x}, {y}) in {painted:?}"
        );
    }
    assert!(
        !painted.iter().any(|(id, _)| *id == 10302),
        "the window is focused"
    );
}

#[test]
fn a_fleet_carrying_regiments_shows_10405_under_its_entry_its_ship_and_its_item() {
    // FUN_004a67a0 flag 2: 10405 at (41, 33) in a left-list entry (the
    // fleet's, as A0 menus/23 shows, and its ship's) and at (41, 23) in
    // the right item; no squadron or personnel indicator.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 2);
    let troop = add_troop(&mut world, system);
    let mut transport = TroopTransportState::default();
    transport.embark(&mut world, fleet, &[troop]).unwrap();
    let mut state = opened(&world, system);
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
        window.expanded.push(fleet);
    }
    let painted = run_moving(
        &world,
        &MovementState::default(),
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        hover(at(-50.0, 0.0)),
    )
    .painted;
    for (x, y) in [(45.0, 62.0), (45.0, 112.0), (142.0, 150.0)] {
        assert!(painted.contains(&(10405, at(x, y))), "({x}, {y}) in {painted:?}");
    }
    assert_eq!(painted.iter().filter(|(id, _)| *id == 10405).count(), 3);
    assert!(!painted.iter().any(|(id, _)| *id == 10404 || *id == 10406), "{painted:?}");
}

#[test]
fn a_fleet_in_hyperspace_shows_its_overlays_where_it_is_bound() {
    // move-order.md: the fleet joins its destination at once; there
    // FUN_004a37c0 draws 10423 at the entry's (5, 5) on +0x50 bit 4 and
    // FUN_004a5c00 10426 over the one selected fleet's picture. Its ships
    // are en route with it (FUN_004f8240), so FUN_0042c3b0 draws each
    // mini's own mark (+0x1000) under the ship entry's mini at (5, 15)
    // (FUN_004a3d40) and the right item's at (28, 4) (FUN_004a6e70).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let class = world.fleets[fleet].capital_ships[0].class;
    // The MC80 Liberty cruiser's class.
    world.capital_ship_classes[class].dat_id = DatId::new(0x1400_0040);
    let mini = capital_ship_mini_id(world.capital_ship_classes[class].dat_id).unwrap();
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
        window.expanded.push(fleet);
    }
    let mut movement = MovementState::default();
    let paint = |state: &mut FleetWindowState, movement: &MovementState| {
        run_moving(
            &world,
            movement,
            &transport,
            state,
            CockpitFaction::Alliance,
            hover(at(-50.0, 0.0)),
        )
        .painted
    };

    let orbiting = paint(&mut state, &movement);
    assert!(orbiting.contains(&(10400, at(9.0, 34.0))));
    assert!(!orbiting
        .iter()
        .any(|(id, _)| *id == 10423 || *id == 10426 || *id == mini + 0x1000));

    // Bound here from elsewhere: out of the orbit index, under an order.
    assert!(movement.order(fleet, system, system, 5));
    let en_route = paint(&mut state, &movement);
    assert!(en_route.contains(&(10423, at(9.0, 34.0))), "{en_route:?}");
    assert!(en_route.contains(&(10426, at(100.0, 42.0))), "{en_route:?}");
    assert!(
        en_route.contains(&(mini + 0x1000, at(9.0, 94.0))),
        "{en_route:?}"
    );
    assert!(
        en_route.contains(&(mini + 0x1000, at(129.0, 131.0))),
        "{en_route:?}"
    );
    // FUN_004a5c00 blits 10426 into the panel first, then the picture
    // keyed over it.
    let order = |painted: &[(u32, egui::Pos2)], id| painted.iter().position(|(x, _)| *x == id);
    assert!(
        order(&en_route, 10426) < order(&en_route, 10425),
        "{en_route:?}"
    );

    // One ship selected: its portrait (FUN_0042c3b0(.., 1, 1), the mini
    // less 0x4000) at the panel's origin, over its own mark (+0x1000)
    // while en route. FUN_0042c3b0 blits each mark into the picture's
    // bitmap first (0x42ca71..0x42cc87) and the object keyed last
    // (0x42ccd9), so the mark shows behind it (manual pp. 96-97: the
    // starfield behind the portrait). The same holds for the minis.
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Ship { fleet, index: 0 });
    }
    let portrait = mini - 0x4000;
    let ship = paint(&mut state, &movement);
    let panel = at(100.0, 42.0);
    assert!(ship.contains(&(portrait, panel)), "{ship:?}");
    assert!(ship.contains(&(portrait + 0x1000, panel)), "{ship:?}");
    assert!(order(&ship, portrait + 0x1000) < order(&ship, portrait));
    assert!(order(&en_route, mini + 0x1000) < order(&en_route, mini));
    let orbiting = paint(&mut state, &MovementState::default());
    assert!(orbiting.contains(&(portrait, panel)), "{orbiting:?}");
    assert!(!orbiting.iter().any(|(id, _)| *id == portrait + 0x1000));
}

#[test]
fn a_damaged_ship_shows_its_damage_mark_under_its_mini_and_portrait() {
    // FUN_0042c3b0 (+0x50 bit 0x200): GOKRES mini + 0x2000 ((class &
    // 0xfff) + 0x6000) and portrait + 0x2000, under the object; a ship
    // is damaged with hull damage or a hit (FUN_00501c00).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let class = world.fleets[fleet].capital_ships[0].class;
    world.capital_ship_classes[class].dat_id = DatId::new(0x1400_0040);
    world.capital_ship_classes[class].hull = 100;
    world.fleets[fleet].capital_ships[0].hull_current = 100;
    let mini = capital_ship_mini_id(world.capital_ship_classes[class].dat_id).unwrap();
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleet));
        window.expanded.push(fleet);
    }
    let movement = MovementState::default();
    let paint = |world: &GameWorld, state: &mut FleetWindowState| {
        run_moving(
            world,
            &movement,
            &transport,
            state,
            CockpitFaction::Alliance,
            hover(at(-50.0, 0.0)),
        )
        .painted
    };
    let whole = paint(&world, &mut state);
    assert!(!whole.iter().any(|(id, _)| *id == mini + 0x2000));
    world.fleets[fleet].capital_ships[0]
        .set_hits(rebellion_core::world::Subsystem::Tractor, 1);
    let hit = paint(&world, &mut state);
    let order = |painted: &[(u32, egui::Pos2)], id, at: egui::Pos2| {
        painted.iter().position(|entry| *entry == (id, at))
    };
    for spot in [at(9.0, 94.0), at(129.0, 131.0)] {
        let mark = order(&hit, mini + 0x2000, spot);
        assert!(mark.is_some(), "{spot:?} {hit:?}");
        assert!(mark < order(&hit, mini, spot), "{hit:?}");
    }
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Ship { fleet, index: 0 });
    }
    let portrait = mini - 0x4000;
    let panel = at(100.0, 42.0);
    let selected = paint(&world, &mut state);
    let mark = order(&selected, portrait + 0x2000, panel);
    assert!(mark.is_some(), "{selected:?}");
    assert!(mark < order(&selected, portrait, panel));
}

#[test]
fn an_injured_character_aboard_is_a_damaged_item() {
    // FUN_004f7800: injury != 0 sets the damaged bit, which
    // FUN_0042c3b0 marks with STRATEGY 11502 under the mini.
    let (mut world, transport, _, fleet, _) = fleet_with_regiment();
    let character = world.characters.insert(rebellion_core::world::Character {
        dat_id: DatId::new(832),
        name: "Han".into(),
        is_alliance: true,
        ..Default::default()
    });
    world.fleets[fleet].characters.push(character);
    let selected = Some(FleetWindowEntry::Fleet(fleet));
    let item = |world: &GameWorld| {
        right_items(world, &transport, &[], selected, FleetWindowTab::Personnel)[0].clone()
    };
    assert!(!item(&world).damaged);
    world.characters[character].injury = 1;
    let hurt = item(&world);
    assert!(hurt.damaged);
    assert_eq!(
        mini_marks(hurt.kind, hurt.mini.unwrap(), false, hurt.damaged),
        [(DllSource::Strategy, 11_500), (DllSource::Strategy, 11_502)]
    );
    // A captive (+0xac bit 0) is marked over its mini, GOKRES + 0x3000.
    assert!(!hurt.captive);
    world.characters[character].is_captive = true;
    assert!(item(&world).captive);
    assert_eq!(captive_mark(19_008), (DllSource::Gokres, 0x7a40));
}

#[test]
fn a_fleet_with_a_damaged_ship_takes_its_damage_marks_under_its_art() {
    // FUN_004feab0: a fleet is damaged while a ship is; FUN_004a37c0
    // and FUN_004a5c00 draw 10424 / 10427 (+ 50 for the Empire) into
    // the entry's and the picture's bitmaps before the art itself.
    assert_eq!(fleet_entry_layers(1, false, false), [FLEET_FRAME]);
    assert_eq!(
        fleet_entry_layers(1, true, true),
        [EN_ROUTE_ENTRY, DAMAGED_ENTRY, FLEET_FRAME]
    );
    assert_eq!(
        fleet_entry_layers(2, false, true),
        [DAMAGED_ENTRY + 50, FLEET_FRAME + 50]
    );
    assert_eq!(
        fleet_picture_layers(1, true, true),
        [EN_ROUTE_PICTURE, DAMAGED_PICTURE, FLEET_PICTURE]
    );
    assert_eq!(fleet_picture_layers(2, false, false), [FLEET_PICTURE + 50]);
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let class = world.fleets[fleet].capital_ships[0].class;
    world.capital_ship_classes[class].hull = 100;
    world.fleets[fleet].capital_ships[0].hull_current = 100;
    assert!(!fleet_damaged(&world, fleet));
    world.fleets[fleet].capital_ships[0].hull_current = 90;
    assert!(fleet_damaged(&world, fleet));
    // A lost ship does not count.
    world.fleets[fleet].capital_ships[0].alive = false;
    assert!(!fleet_damaged(&world, fleet));
}

#[test]
fn the_marks_of_a_mini_stack_en_route_then_damaged() {
    // FUN_0042c3b0: the backdrop, en route, then damaged; personnel
    // take STRATEGY 11502 for damage.
    assert_eq!(mini_marks(MiniObject::Craft, 17_473, false, false), []);
    assert_eq!(
        mini_marks(MiniObject::Craft, 17_473, true, true),
        [(DllSource::Gokres, 21_569), (DllSource::Gokres, 25_665)]
    );
    // Personnel stand on 11500 and regiments on 11513 (0x42c6b3,
    // 0x42c748), whatever the marks.
    assert_eq!(
        mini_marks(MiniObject::Character, 17_473, false, true),
        [(DllSource::Strategy, 11_500), (DllSource::Strategy, 11_502)]
    );
    assert_eq!(
        mini_marks(MiniObject::Regiment(DatId::new(0x1000_0001)), 17_473, true, false),
        [(DllSource::Strategy, 11_513), (DllSource::Strategy, 11_515)]
    );
    assert_eq!(
        mini_backdrop(MiniObject::SpecialForce(DatId::new(0x3c00_0001))),
        Some((DllSource::Strategy, 11_500))
    );
    assert_eq!(
        damage_mark(MiniObject::SpecialForce(DatId::new(0x3c00_0001)), 1),
        (DllSource::Strategy, 11_502)
    );
    assert_eq!(
        damage_mark(MiniObject::Regiment(DatId::new(0x1000_0001)), 1),
        (DllSource::Gokres, 0x2001)
    );
}

#[test]
fn each_kind_of_mini_takes_its_own_en_route_mark() {
    // FUN_0042c3b0: GOKRES (class & 0xfff) + 0x5000 by default, kept for
    // craft, regiments 0x10000002/0x10000008 and special forces
    // 0x3c000003/0x3c000005; 0x2cfb (11515) for other regiments and
    // 0x2ced (11501) for characters and other special forces.
    let mark = |object| en_route_mark(object, 17_473);
    let own = (DllSource::Gokres, 21_569);
    let regiment = |raw| MiniObject::Regiment(DatId::new(raw));
    let force = |raw| MiniObject::SpecialForce(DatId::new(raw));
    assert_eq!(mark(MiniObject::Craft), own);
    assert_eq!(mark(regiment(0x1000_0002)), own);
    assert_eq!(mark(regiment(0x1000_0008)), own);
    assert_eq!(mark(regiment(0x1000_0001)), (DllSource::Strategy, 11_515));
    assert_eq!(mark(MiniObject::Character), (DllSource::Strategy, 11_501));
    assert_eq!(mark(force(0x3c00_0003)), own);
    assert_eq!(mark(force(0x3c00_0005)), own);
    assert_eq!(mark(force(0x3c00_0001)), (DllSource::Strategy, 11_501));
}

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    }
}

fn renames(actions: &[FleetWindowAction]) -> Vec<(FleetWindowEntry, String)> {
    actions
        .iter()
        .filter_map(|action| match action {
            FleetWindowAction::Rename { entry, name } => Some((*entry, name.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn rename_edits_the_name_in_place_and_enter_issues_it() {
    // FUN_004ac7a0 opens the field holding the name, all selected, so
    // typing replaces it; FUN_004ac950 issues it on Enter.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    let entry = FleetWindowEntry::Fleet(fleet);
    assert!(state.begin_rename(&world, system, entry));
    assert!(state.renaming());
    assert_eq!(state.selection(system).unwrap().0, Some(entry));

    let mut frames = hover(at(-50.0, 0.0));
    frames.push(vec![egui::Event::Text("Rogue".into())]);
    frames.push(vec![key(egui::Key::Enter)]);
    frames.push(vec![]);
    let run = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        frames,
    );
    assert_eq!(renames(&run.actions), [(entry, "Rogue".to_owned())]);
    assert!(!state.renaming());
}

#[test]
fn a_ship_of_the_selected_fleet_is_renamed_in_the_right_list() {
    // A0 menus3/40-43: Rename on Victory Destroyer 2 from the right
    // list opens the field on its name under its picture, Fleet 1
    // staying selected, and Enter renames the ship.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let class = world.fleets[fleet].capital_ships[0].class;
    world.fleets[fleet]
        .capital_ships
        .insert(0, ShipInstance::new(class, 0, true));
    world.fleets[fleet].capital_ships[0].alive = false;
    world.fleets[fleet]
        .capital_ships
        .push(ShipInstance::new(class, 100, true));
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    assert!(state.select(system, FleetWindowEntry::Fleet(fleet)));
    let ship = FleetWindowEntry::Ship { fleet, index: 2 };
    assert!(state.begin_rename(&world, system, ship));
    assert_eq!(
        state.selection(system).unwrap().0,
        Some(FleetWindowEntry::Fleet(fleet))
    );
    // The dead first ship is not listed: index 2 is the second row.
    assert_eq!(state.windows[0].rename.as_ref().unwrap().item, Some(1));
    assert_eq!(ship_item_row(&world, fleet, 0), None);
    assert_eq!(ship_item_row(&world, fleet, 1), Some(0));

    let mut frames = hover(at(-50.0, 0.0));
    frames.push(vec![egui::Event::Text("Rogue".into())]);
    frames.push(vec![key(egui::Key::Enter)]);
    frames.push(vec![]);
    let run = run(&world, &transport, &mut state, CockpitFaction::Alliance, frames);
    assert_eq!(renames(&run.actions), [(ship, "Rogue".to_owned())]);

    // With the ship itself chosen on the left, the field stays there.
    let mut state = opened(&world, system);
    assert!(state.begin_rename(&world, system, ship));
    assert_eq!(state.selection(system).unwrap().0, Some(ship));
    assert_eq!(state.windows[0].rename.as_ref().unwrap().item, None);
}

#[test]
fn a_squadron_with_losses_lists_with_the_damaged_mark() {
    // FUN_00503340 sets a squadron with losses (+0x60) Damaged (+0x50
    // bit 0x200); FUN_0042c3b0(.., 0, 1) marks its mini.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    let wing = world.fighter_classes.insert(rebellion_core::world::FighterClass {
        dat_id: DatId::new(0x1c00_0001),
        name: "X-wing".into(),
        is_alliance: true,
        squadron_size: 12,
        ..Default::default()
    });
    rebellion_core::squadrons::add(&mut world.fleets[fleet].fighters, wing, 1);
    world.fleets[fleet].fighters.push(rebellion_core::world::FighterEntry {
        class: wing,
        count: 1,
        carrier: None,
        damage: Some(rebellion_core::world::SquadronDamage { lost: 2, repair: None }),
    });
    let transport = TroopTransportState::default();
    let selected = Some(FleetWindowEntry::Fleet(fleet));
    let fighters = right_items(&world, &transport, &[], selected, FleetWindowTab::Fighters);
    assert_eq!(
        fighters.iter().map(|item| item.damaged).collect::<Vec<_>>(),
        [false, true]
    );
}

#[test]
fn an_unfinished_regiment_or_squadron_aboard_lists_in_its_tab_and_drags() {
    // FUN_00504c40(ship, 3) and FUN_005039d0(ship, 3) walk mode 3: an
    // unfinished regiment or squadron aboard lists (A0 2026-11-15 "m-5":
    // the Army Regiment in Fleet 3's regiments tab, on the grid). A
    // drag of an unfinished unit is its Move (A0 "m-3").
    use rebellion_core::manufacturing::{BuildableKind, ManufacturingState, QueueItem};
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    let ship_class = world.fleets[fleet].capital_ships[0].class;
    world.capital_ship_classes[ship_class].troop_capacity = 2;
    world.capital_ship_classes[ship_class].fighter_capacity = 2;
    let army = DatId::new(0x1000_0002);
    world.buildable_classes.insert(
        army,
        rebellion_core::world::BuildableClass {
            is_alliance: true,
            ..Default::default()
        },
    );
    let wing = world.fighter_classes.insert(rebellion_core::world::FighterClass {
        dat_id: DatId::new(0x1c00_0001),
        name: "X-wing".into(),
        is_alliance: true,
        ..Default::default()
    });
    let mut manufacturing = ManufacturingState::new();
    manufacturing.enqueue(system, QueueItem::new(BuildableKind::Troop(army), 5));
    manufacturing.enqueue(system, QueueItem::new(BuildableKind::Fighter(wing), 5));
    rebellion_core::products::reconcile(&mut world, &manufacturing);
    let id_of = |world: &GameWorld, kind| {
        world.products.all().iter().find(|p| p.kind == kind).unwrap().id
    };
    let ids = vec![
        id_of(&world, BuildableKind::Troop(army)),
        id_of(&world, BuildableKind::Fighter(wing)),
    ];
    for &id in &ids {
        assert!(rebellion_core::products::move_to_target(&mut world, id, system, Some(fleet), 0));
    }
    let transport = TroopTransportState::default();
    let selected = Some(FleetWindowEntry::Fleet(fleet));
    assert!(tab_enabled(&world, &transport, &[], selected, FleetWindowTab::Troops));
    let troops = right_items(&world, &transport, &[], selected, FleetWindowTab::Troops);
    let regiment = ItemObject::ProductTroop {
        product: ids[0],
        system,
        class: army,
    };
    assert_eq!(troops.len(), 1);
    assert_eq!(troops[0].object, Some(regiment));
    assert_eq!(
        regiment.menu_object(),
        MenuObject::PendingTroop {
            system,
            class: army,
            product: Some(ids[0]),
        }
    );
    let fighters = right_items(&world, &transport, &[], selected, FleetWindowTab::Fighters);
    let squadron = ItemObject::ProductFighter {
        product: ids[1],
        system,
        class: wing,
    };
    assert_eq!(fighters.len(), 1);
    assert_eq!(fighters[0].object, Some(squadron));
    assert_eq!(
        squadron.menu_object(),
        MenuObject::PendingFighter {
            system,
            class: wing,
            product: Some(ids[1]),
        }
    );
    let point = egui::pos2(10.0, 20.0);
    assert_eq!(
        right_drop(&[regiment, squadron], point, true),
        vec![FleetWindowAction::DragProducts {
            products: ids.clone(),
            point,
            confirmed: true,
        }]
    );
    assert_eq!(
        left_drop(&[ItemObject::Product(9)], point, false),
        vec![FleetWindowAction::DragProducts {
            products: vec![9],
            point,
            confirmed: false,
        }]
    );
    assert!(product_drops(&[ItemObject::Fleet(fleet)], point, false).is_empty());
}

#[test]
fn an_unfinished_ships_rename_opens_on_its_tree_entry_and_names_it() {
    // 0x203 on the unfinished ship (A0 2026-11-15 "u-2": Rename white):
    // the field opens on its entry in its fleet's tree, holding its
    // name (FUN_004f6270), and Enter issues the rename for it.
    use rebellion_core::manufacturing::{BuildableKind, ManufacturingState, QueueItem};
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    world.set_ship_spare(system, true, fleet);
    let class = world.fleets[fleet].capital_ships[0].class;
    let mut manufacturing = ManufacturingState::new();
    manufacturing.enqueue(
        system,
        QueueItem::new(BuildableKind::CapitalShip(class), 5).ordered(&mut world),
    );
    rebellion_core::products::reconcile(&mut world, &manufacturing);
    let product = world.products.all()[0].id;
    let entry = FleetWindowEntry::Product { fleet, product };
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    assert!(state.expand(system, fleet));
    assert!(state.begin_rename(&world, system, entry));
    assert_eq!(state.selection(system).unwrap().0, Some(entry));
    let named = world.products.all()[0].name.clone().unwrap();
    assert_eq!(state.windows[0].rename.as_ref().unwrap().item, None);

    let mut frames = hover(at(-50.0, 0.0));
    frames.push(vec![egui::Event::Text("Rogue".into())]);
    frames.push(vec![key(egui::Key::Enter)]);
    frames.push(vec![]);
    let run = run(&world, &transport, &mut state, CockpitFaction::Alliance, frames);
    let issued = renames(&run.actions);
    assert_eq!(issued.len(), 1);
    assert_eq!(issued[0].0, entry);
    assert_ne!(issued[0].1, named);
}

#[test]
fn an_unnamed_unfinished_ship_is_renamed_from_its_class_name() {
    // FUN_004f6270: no name of its own, the class's.
    use rebellion_core::manufacturing::{BuildableKind, ManufacturingState, QueueItem};
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    world.set_ship_spare(system, true, fleet);
    let class = world.fleets[fleet].capital_ships[0].class;
    let mut manufacturing = ManufacturingState::new();
    manufacturing.enqueue(system, QueueItem::new(BuildableKind::CapitalShip(class), 5));
    rebellion_core::products::reconcile(&mut world, &manufacturing);
    let product = world.products.all()[0].id;
    let entry = FleetWindowEntry::Product { fleet, product };
    let mut state = opened(&world, system);
    assert!(state.expand(system, fleet));
    assert!(state.begin_rename(&world, system, entry));
    assert_eq!(
        state.windows[0].rename.as_ref().unwrap().text,
        world.capital_ship_classes[class].name
    );
}

#[test]
fn the_rename_field_shows_the_selected_name_black_where_the_name_was() {
    // A0 menus3/41 and 46: the field opens on the name line, the whole
    // name selected black on white; a ship's in the right list's name
    // line (+37 under the item, 125 wide, centred) with the item's own
    // label hidden, a fleet's on its left entry from the entry's left.
    // Typed text is white.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let _second = add_fleet(&mut world, system, true, 0);
    let third = add_fleet(&mut world, system, true, 0);
    let class = world.fleets[fleet].capital_ships[0].class;
    for _ in 0..2 {
        world.fleets[fleet]
            .capital_ships
            .push(ShipInstance::new(class, 100, true));
    }
    let transport = TroopTransportState::default();
    let open_on = |entry| {
        let mut state = opened(&world, system);
        assert!(state.select(system, FleetWindowEntry::Fleet(fleet)));
        assert!(state.begin_rename(&world, system, entry));
        state
    };
    // The third ship: row 2 of the right list.
    let ship = FleetWindowEntry::Ship { fleet, index: 2 };
    let mut state = open_on(ship);
    let shown = run(&world, &transport, &mut state, CockpitFaction::Alliance, hover(at(-50.0, 0.0)));
    let named = |colour| -> Vec<egui::Pos2> {
        shown
            .texts
            .iter()
            .filter(|text| text.text == "Transport" && text.color == colour)
            .map(|text| text.pos)
            .collect()
    };
    let labels = named(egui::Color32::WHITE);
    let [field] = named(egui::Color32::BLACK)[..] else {
        panic!("one black field: {:?}", named(egui::Color32::BLACK));
    };
    // The first two rows keep their labels; the edited third has none.
    assert_eq!(labels.len(), 2, "{labels:?}");
    let pitch = RIGHT_ITEM_HEIGHT * 2.0;
    assert!((labels[1].y - labels[0].y - pitch).abs() < 1.0, "{labels:?}");
    assert!((field.y - labels[0].y - 2.0 * pitch).abs() < 4.0, "{field:?} {labels:?}");
    let end = shown
        .texts
        .iter()
        .find(|text| text.pos == field)
        .unwrap()
        .end;
    let centre = at(RIGHT_LIST.0 + 62.5, 0.0).x;
    assert!(((field.x + end) / 2.0 - centre).abs() < 4.0, "{field:?} {end} {centre}");

    let mut state = open_on(ship);
    let mut frames = hover(at(-50.0, 0.0));
    frames.push(vec![egui::Event::Text("Ro".into())]);
    frames.push(vec![]);
    let typed = run(&world, &transport, &mut state, CockpitFaction::Alliance, frames);
    let ro = typed.texts.iter().find(|text| text.text == "Ro").unwrap();
    assert_eq!(ro.color, egui::Color32::WHITE);

    // The third fleet's left entry, row 2: the field sits on its
    // label's line, from its left.
    let mut state = opened(&world, system);
    assert!(state.begin_rename(&world, system, FleetWindowEntry::Fleet(third)));
    let shown = run(&world, &transport, &mut state, CockpitFaction::Alliance, hover(at(-50.0, 0.0)));
    let name = world.fleet_name(third).unwrap().to_owned();
    let find = |colour| {
        shown
            .texts
            .iter()
            .find(|text| text.text == name && text.color == colour)
            .unwrap_or_else(|| panic!("{name:?} {colour:?}"))
            .pos
    };
    let (field, label) = (find(egui::Color32::BLACK), find(egui::Color32::WHITE));
    assert!((field.y - label.y).abs() < 4.0, "{field:?} {label:?}");
    let entry = at(LEFT_LIST.0 + 4.0, LEFT_LIST.1 + 2.0 * LEFT_ITEM_HEIGHT + 3.0);
    // The field's text starts just inside it (egui's margin).
    assert!(field.x >= entry.x && field.x - entry.x < 6.0, "{field:?} {entry:?}");
    assert!(field.y > entry.y, "{field:?} {entry:?}");
}

/// A fleet of `ships` live ships, selected on its Capital ships tab,
/// renaming ship `index` with the right list scrolled to `scroll`.
fn renaming_in_a_scrolled_right_list(
    ships: usize,
    index: usize,
    scroll: f32,
) -> (GameWorld, SystemKey, FleetWindowState) {
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let class = world.fleets[fleet].capital_ships[0].class;
    for _ in 1..ships {
        world.fleets[fleet]
            .capital_ships
            .push(ShipInstance::new(class, 100, true));
    }
    let mut state = opened(&world, system);
    assert!(state.select(system, FleetWindowEntry::Fleet(fleet)));
    state.window_mut(system).unwrap().right_scroll = scroll;
    assert!(state.begin_rename(&world, system, FleetWindowEntry::Ship { fleet, index }));
    (world, system, state)
}

#[test]
fn the_rename_field_opens_on_a_scrolled_right_items_shown_name_line() {
    // FUN_004a71d0: the field takes the item's rect, which FUN_00609e20
    // keeps less the list's first visible pixel (FUN_00609dd0).
    let (world, _, mut state) = renaming_in_a_scrolled_right_list(6, 4, 100.0);
    let shown = run(
        &world,
        &TroopTransportState::default(),
        &mut state,
        CockpitFaction::Alliance,
        hover(at(-50.0, 0.0)),
    );
    let field = shown
        .texts
        .iter()
        .find(|text| text.text == "Transport" && text.color == egui::Color32::BLACK)
        .unwrap_or_else(|| panic!("no field: {:?}", shown.texts))
        .pos;
    // Row 4 shows from y 127 + 200 - 100 = 227; its name line is +37.
    let line = at(0.0, 227.0 + 37.0).y;
    assert!(
        field.y > line && field.y - line < 2.0 * 14.0,
        "{field:?} {line}"
    );
    assert!(state.renaming());
}

#[test]
fn the_rename_field_opens_on_a_scrolled_left_entrys_shown_name_line() {
    // FUN_004a71d0, as for the right list: the entry's rect less the
    // left list's first visible pixel.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleets: Vec<_> = (0..7)
        .map(|_| add_fleet(&mut world, system, true, 0))
        .collect();
    let mut state = opened(&world, system);
    state.window_mut(system).unwrap().left_scroll = 89.0;
    assert!(state.begin_rename(&world, system, FleetWindowEntry::Fleet(fleets[6])));
    let shown = run(
        &world,
        &TroopTransportState::default(),
        &mut state,
        CockpitFaction::Alliance,
        hover(at(-50.0, 0.0)),
    );
    let name = world.fleet_name(fleets[6]).unwrap().to_owned();
    let field = shown
        .texts
        .iter()
        .find(|text| text.text == name && text.color == egui::Color32::BLACK)
        .unwrap_or_else(|| panic!("no field: {:?}", shown.texts))
        .pos;
    // Fleet 7 shows from y 29 + 300 - 89 = 240; the field is at +3.
    let line = at(0.0, 240.0 + 3.0).y;
    assert!(
        field.y > line && field.y - line < 2.0 * 15.0,
        "{field:?} {line}"
    );
}

#[test]
fn a_press_on_a_lists_scroll_bar_ends_the_rename_unissued() {
    // The bar (FUN_0060f640, base FUN_0060aca0) posts 0x40e on a press;
    // the list relays it (FUN_006083c0) and the window drops the edit
    // (FUN_004ac3a0 → vtable +0x80, FUN_004aca40).
    let (world, system, mut state) = renaming_in_a_scrolled_right_list(6, 4, 100.0);
    // The right bar's track, window x 224..233.
    let run = run(
        &world,
        &TroopTransportState::default(),
        &mut state,
        CockpitFaction::Alliance,
        click(at(228.0, 200.0)),
    );
    assert!(renames(&run.actions).is_empty());
    assert!(!state.renaming());
    assert!(state.is_open(system));
}


#[test]
fn an_empty_rename_keeps_its_field_and_escape_drops_it_unissued() {
    // FUN_004ac950 issues nothing for an empty field and leaves it open.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    assert!(state.begin_rename(&world, system, FleetWindowEntry::Fleet(fleet)));

    let mut frames = hover(at(-50.0, 0.0));
    frames.push(vec![key(egui::Key::Backspace)]);
    frames.push(vec![key(egui::Key::Enter)]);
    frames.push(vec![]);
    let run = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        frames,
    );
    assert!(renames(&run.actions).is_empty());
    assert!(state.renaming(), "the field stays open");

    let mut frames = hover(at(-50.0, 0.0));
    frames.push(vec![key(egui::Key::Escape)]);
    frames.push(vec![]);
    let run_escape = super::tests::run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        frames,
    );
    assert!(renames(&run_escape.actions).is_empty());
    assert!(!state.renaming());
}

#[test]
fn a_button_draws_pressed_only_while_the_mouse_is_down_on_it() {
    // FUN_00602d30's button states: 0x277c's 10108 normal, 10109 pressed.
    let (world, system) = world(ControlKind::Uncontrolled);
    let transport = TroopTransportState::default();
    let close = at(225.0, 10.0);
    let mut state = opened(&world, system);
    let hovered = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        hover(close),
    )
    .painted;
    assert!(hovered.contains(&(CLOSE_NORMAL, at(218.0, 3.0))));
    assert!(!hovered.iter().any(|(id, _)| *id == CLOSE_PRESSED));

    let mut frames = hover(close);
    frames.push(vec![press(close, true)]);
    frames.push(vec![]);
    let held = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        frames,
    )
    .painted;
    assert!(held.contains(&(CLOSE_PRESSED, at(218.0, 3.0))));
}

#[test]
fn the_list_draws_the_rows_that_fit_and_the_right_list_too() {
    // Our own: the 266-high list holds six 50-high rows, the 164-high right
    // list four.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleets: Vec<_> = (0..7)
        .map(|_| add_fleet(&mut world, system, true, 5))
        .collect();
    let troops: Vec<_> = (0..5).map(|_| add_troop(&mut world, system)).collect();
    let mut transport = TroopTransportState::default();
    transport.embark(&mut world, fleets[0], &troops).unwrap();
    let mut state = opened(&world, system);
    if let Some(window) = state.window_mut(system) {
        window.selected = ListSelection::only(FleetWindowEntry::Fleet(fleets[0]));
        window.tab = FleetWindowTab::Troops;
    }

    let texts = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        hover(at(-50.0, 0.0)),
    )
    .texts;
    let count = |prefix: &str, size: f32| {
        texts
            .iter()
            .filter(|text| text.text.starts_with(prefix) && text.size == size)
            .count()
    };
    // FUN_004a4b10: both lists write in font 10.
    let font = crate::theme::gdi_font(10, 2.0).size;
    assert_eq!(count("Fleet ", font), 6);
    assert_eq!(count("Alliance Fleet Regiment", font), 4);
}

#[test]
fn a_right_click_on_a_left_list_entry_selects_it_and_opens_its_menu() {
    // A0 fleet3/02: Medium Transport 1 right-clicked is framed and named.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    state.select(system, FleetWindowEntry::Fleet(fleet));
    state.expand(system, fleet);
    // The second row: the ship.
    let actions = right_click(&world, &transport, &mut state, at(40.0, 104.0));
    assert_eq!(menus(&actions).len(), 1);
    assert_eq!(
        state.window_mut(system).unwrap().selected.single(),
        Some(FleetWindowEntry::Ship { fleet, index: 0 })
    );
}

#[test]
fn a_ship_selected_over_capital_ships_shows_fighters_and_its_fleet_keeps_them() {
    // A0 fleet3/02, 06.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let mut state = opened(&world, system);
    assert!(state.select(system, FleetWindowEntry::Ship { fleet, index: 0 }));
    assert_eq!(state.window_mut(system).unwrap().tab, FleetWindowTab::Fighters);
    assert!(state.select(system, FleetWindowEntry::Fleet(fleet)));
    assert_eq!(state.window_mut(system).unwrap().tab, FleetWindowTab::Fighters);
    state.show_tab(system, FleetWindowTab::Personnel);
    assert!(state.select(system, FleetWindowEntry::Ship { fleet, index: 0 }));
    assert_eq!(state.window_mut(system).unwrap().tab, FleetWindowTab::Personnel);
}

#[test]
fn the_selection_drops_when_its_fleet_gains_or_loses_a_ship() {
    // A0 fleet3/03, 05, 10: no selection after Create Fleet, a ship's
    // Move onto a fleet, or a fleet's Move onto another.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    state.select(system, FleetWindowEntry::Fleet(fleet));
    let frame = |world: &GameWorld, state: &mut FleetWindowState| {
        run(world, &transport, state, CockpitFaction::Alliance, hover(at(-50.0, 0.0)));
    };
    frame(&world, &mut state);
    frame(&world, &mut state);
    assert_eq!(state.window_mut(system).unwrap().selected.single(), Some(FleetWindowEntry::Fleet(fleet)));
    let ship = world.fleets[fleet].capital_ships[0].clone();
    world.fleets[fleet].capital_ships.push(ship);
    frame(&world, &mut state);
    assert_eq!(state.window_mut(system).unwrap().selected.single(), None);
    // A new selection is kept.
    state.select(system, FleetWindowEntry::Fleet(fleet));
    frame(&world, &mut state);
    frame(&world, &mut state);
    assert_eq!(state.window_mut(system).unwrap().selected.single(), Some(FleetWindowEntry::Fleet(fleet)));
}

#[test]
fn an_expanded_fleet_s_ships_are_named_in_the_left_list_where_the_fleet_is() {
    // A0 fleet3/01: "Corellian Corvette" and "Medium Transpor" in the
    // left list, set like "Fleet 1" at the row's (7, 3), clipped by it.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 0);
    assert!(world.rename_ship(fleet, 0, "Medium Transport 1"));
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    state.expand(system, fleet);
    let texts = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        hover(at(-50.0, 0.0)),
    )
    .texts;
    // The second row: the list's (4, 29) plus 50.
    assert!(
        texts.iter().any(|text| text.text == "Medium Transport 1"
            && text.set_in((11.0, 82.0, 84.0, 14.0), 10, crate::theme::GdiAlign::Left)),
        "{texts:?}"
    );
}

#[test]
fn the_tree_lines_are_solid_and_leave_out_their_end_points() {
    // A0 fleet2/vjun-fleet5-admiral-ships: x 6 runs from y 35 to 84
    // unbroken; the last ship's branch covers x 6..8 at y 85 (LineTo
    // leaves its end point). An expanded fleet's line runs down past
    // its ships; the last ship's stops at its branch.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    let class = world.fleets[fleet].capital_ships[0].class;
    world.fleets[fleet]
        .capital_ships
        .push(ShipInstance::new(class, 100, true));
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    state.window_mut(system).unwrap().expanded.push(fleet);

    let lines = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        hover(at(-50.0, 0.0)),
    )
    .lines;
    // A pixel is lit when the line covers it to its far corner.
    let lit = |x: f32, y: f32| {
        let point = at(x, y) + egui::vec2(1.5, 1.5);
        lines.iter().any(|line| line.contains(point))
    };
    // The fleet's branch and its line down (item 0 at y 29).
    assert!((6..=8).all(|x| lit(x as f32, 35.0)) && !lit(9.0, 35.0));
    assert!((35..=78).all(|y| lit(6.0, y as f32)));
    // The first ship's line through its row (item 1 at y 79).
    assert!((79..=128).all(|y| lit(6.0, y as f32)));
    // The last ship's line stops at its branch (item 2 at y 129).
    assert!((129..=134).all(|y| lit(6.0, y as f32)) && !lit(6.0, 136.0));
    assert!((6..=8).all(|x| lit(x as f32, 135.0)) && !lit(9.0, 135.0));
    assert!(!lit(5.0, 60.0) && !lit(7.0, 60.0));
}

#[test]
fn the_other_tabs_light_with_squadrons_and_characters_aboard() {
    // FUN_004a5c00: flags 1 (squadrons) and 3 (characters).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    let transport = TroopTransportState::default();
    let selected = Some(FleetWindowEntry::Fleet(fleet));
    let lit = |world: &GameWorld, tab| tab_enabled(world, &transport, &[], selected, tab);
    assert!(!lit(&world, FleetWindowTab::Fighters));
    assert!(!lit(&world, FleetWindowTab::Personnel));

    let squadron = world.fighter_classes.insert(Default::default());
    world.fleets[fleet]
        .fighters
        .push(rebellion_core::world::FighterEntry {
            class: squadron,
            count: 0,
            carrier: None,
            damage: None,
        });
    assert!(!lit(&world, FleetWindowTab::Fighters));
    world.fleets[fleet].fighters[0].count = 1;
    assert!(lit(&world, FleetWindowTab::Fighters));
    let character = world.characters.insert(Default::default());
    world.fleets[fleet].characters.push(character);
    assert!(lit(&world, FleetWindowTab::Personnel));
}

#[test]
fn the_personnel_tab_lists_special_forces_aboard_after_the_characters() {
    // FUN_00536da0(ship, 3) gives the Personnel tab the characters and
    // special forces aboard; a unit alone lights the tab (FUN_004a5c00
    // flag 3), and its right-click menu is a special force's.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    let transport = TroopTransportState::default();
    let selected = Some(FleetWindowEntry::Fleet(fleet));
    let unit = world
        .special_forces
        .insert(rebellion_core::world::SpecialForceUnit {
            class_dat_id: rebellion_core::ids::DatId::new(0x3c00_0004),
            is_alliance: true,
            skills: [0; 8],
            on_mission: false,
            on_hidden_mission: false,
        });
    world.fleets[fleet].special_forces.push(unit);
    assert!(tab_enabled(
        &world,
        &transport,
        &[],
        selected,
        FleetWindowTab::Personnel
    ));
    let character = world.characters.insert(rebellion_core::world::Character {
        name: "Mon Mothma".into(),
        ..Default::default()
    });
    world.fleets[fleet].characters.push(character);

    let items = right_items(&world, &transport, &[], selected, FleetWindowTab::Personnel);
    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["Mon Mothma", "Bothan Spies"]
    );
    assert_eq!(items[1].mini, Some(17_731));
    assert_eq!(items[1].object, Some(ItemObject::SpecialForce(unit)));
    assert_eq!(
        ItemObject::SpecialForce(unit).menu_object(),
        MenuObject::SpecialForce(unit)
    );
}

#[test]
fn a_renamed_ship_is_listed_by_its_own_name() {
    // FUN_004f6270: the ship's +0x34 name, else its class's.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    let transport = TroopTransportState::default();
    let selected = Some(FleetWindowEntry::Fleet(fleet));
    let labels = |world: &GameWorld| {
        right_items(world, &transport, &[], selected, FleetWindowTab::CapitalShips)
            .into_iter()
            .map(|item| item.label)
            .collect::<Vec<_>>()
    };
    let class = world.fleets[fleet].capital_ships[0].class;
    assert_eq!(
        labels(&world),
        [world.capital_ship_classes[class].name.clone()]
    );
    assert!(world.rename_ship(fleet, 0, "Liberty"));
    assert_eq!(labels(&world), ["Liberty"]);
}

#[test]
fn an_unfinished_ship_lists_after_its_fleets_ships_on_the_sides_grid() {
    // FUN_004a6be0 walks the fleet's ships in mode 3 (Existing), so a
    // ship being built lists; FUN_0042c3b0 lays 0x2d32 (11570, side 1)
    // or 0x2d34 (11572) under its mini. A0 2026-11-15 "u-2": Fleet 4
    // lists "Carrack Light Cruiser 1" on the Empire's green grid.
    use rebellion_core::manufacturing::{BuildableKind, ManufacturingState, QueueItem};
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    world.set_ship_spare(system, true, fleet);
    let class = world.fleets[fleet].capital_ships[0].class;
    let mut manufacturing = ManufacturingState::new();
    manufacturing.enqueue(
        system,
        QueueItem::new(BuildableKind::CapitalShip(class), 5).ordered(&mut world),
    );
    rebellion_core::products::reconcile(&mut world, &manufacturing);
    let product = world.products.all()[0].id;
    let transport = TroopTransportState::default();
    let items = right_items(
        &world,
        &transport,
        &[],
        Some(FleetWindowEntry::Fleet(fleet)),
        FleetWindowTab::CapitalShips,
    );
    assert_eq!(items.len(), 2);
    assert_eq!(items[1].object, Some(ItemObject::Product(product)));
    assert_eq!(items[1].label, world.products.all()[0].name.clone().unwrap());
    assert_eq!(items[1].mini, items[0].mini);
    assert_eq!(
        ItemObject::Product(product).menu_object(),
        MenuObject::PendingShip { product }
    );
    assert_eq!(unfinished_mini_mark(true), (DllSource::Strategy, 11_570));
    assert_eq!(unfinished_mini_mark(false), (DllSource::Strategy, 11_572));
    // FUN_004a3d40: the expanded fleet's entry lists it after its ships.
    let mut open = window(system, Some(FleetWindowEntry::Fleet(fleet)));
    let movement = MovementState::default();
    assert_eq!(
        left_entries(&world, &movement, &fog(system), Faction::Alliance, &open),
        [FleetWindowEntry::Fleet(fleet)]
    );
    open.expanded.push(fleet);
    let entry = FleetWindowEntry::Product { fleet, product };
    assert_eq!(
        left_entries(&world, &movement, &fog(system), Faction::Alliance, &open),
        [
            FleetWindowEntry::Fleet(fleet),
            FleetWindowEntry::Ship { fleet, index: 0 },
            entry,
        ]
    );
    assert_eq!(entry.fleet(), fleet);
    assert_eq!(entry_object(&world, entry), ItemObject::Product(product));
    // It carries nothing: every tab is dark for it alone.
    for tab in [
        FleetWindowTab::CapitalShips,
        FleetWindowTab::Fighters,
        FleetWindowTab::Troops,
        FleetWindowTab::Personnel,
    ] {
        assert!(!tab_enabled(&world, &transport, &[], Some(entry), tab));
    }
}

#[test]
fn a_ship_without_a_hyperdrive_carries_the_indicator() {
    // FUN_004a6be0: flag 0x40 marks a ship with no hyperdrive (10430).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    let transport = TroopTransportState::default();
    let selected = Some(FleetWindowEntry::Fleet(fleet));
    let flags = |world: &GameWorld| {
        right_items(world, &transport, &[], selected, FleetWindowTab::CapitalShips)
            .iter()
            .map(|item| item.no_hyperdrive)
            .collect::<Vec<_>>()
    };
    assert_eq!(flags(&world), [false]);
    let class = world.fleets[fleet].capital_ships[0].class;
    world.capital_ship_classes[class].hyperdrive = 0;
    assert_eq!(flags(&world), [true]);
}

#[test]
fn a_ship_whose_hyperdrives_are_both_hit_carries_the_indicator_and_so_does_its_fleet() {
    // A0 fleet2/umgul-fleet1-damaged: Medium Transport 1 (Hyperdrive
    // Rating 0:100) and Fleet 1's entry carry 10430 (FUN_00500820,
    // FUN_004fd900); Corellian Corvette 1, damaged too, does not.
    use rebellion_core::world::Subsystem;
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let fleet = add_fleet(&mut world, system, true, 1);
    let class = world.fleets[fleet].capital_ships[0].class;
    world.capital_ship_classes[class].hyperdrive = 100;
    world.capital_ship_classes[class].hyperdrive_if_damaged = 160;
    let transport = TroopTransportState::default();
    let selected = Some(FleetWindowEntry::Fleet(fleet));
    let flags = |world: &GameWorld| {
        right_items(world, &transport, &[], selected, FleetWindowTab::CapitalShips)
            .iter()
            .map(|item| item.no_hyperdrive)
            .collect::<Vec<_>>()
    };
    world.fleets[fleet].capital_ships[0].set_hits(Subsystem::PrimaryHyperdrive, 1);
    assert_eq!(flags(&world), [false]);
    assert!(!fleet_stranded(&world, fleet));
    world.fleets[fleet].capital_ships[0].set_hits(Subsystem::BackupHyperdrive, 1);
    assert_eq!(flags(&world), [true]);
    assert!(fleet_stranded(&world, fleet));
    world.fleets[fleet].capital_ships[0].alive = false;
    assert!(!fleet_stranded(&world, fleet));
}

#[test]
fn a_drop_on_an_enemy_fleets_listed_ship_targets_that_fleet() {
    // FUN_004a3130 step 3 on tab 0x66: the right list's item, whatever
    // its side; elsewhere the default refuses (an own fleet is listed).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    add_fleet(&mut world, system, true, 1);
    let enemy = add_fleet(&mut world, system, false, 1);
    let mut open = window(system, Some(FleetWindowEntry::Fleet(enemy)));
    let entries = left_entries(
        &world,
        &rebellion_core::movement::MovementState::default(),
        &fog(system),
        Faction::Alliance,
        &open,
    );
    let target = |open: &OpenFleetWindow, point| {
        drop_target(&world, Faction::Alliance, open, &entries, point)
    };
    let enemy_fleet = Some(ReleaseTarget::Fleet {
        fleet: enemy,
        system,
    });

    assert_eq!(target(&open, (150.0, 127.0)), enemy_fleet);
    assert_eq!(target(&open, (150.0, 176.9)), enemy_fleet);
    assert_eq!(target(&open, (150.0, 177.0)), None);
    assert_eq!(target(&open, (150.0, 126.9)), None);
    open.tab = FleetWindowTab::Troops;
    assert_eq!(target(&open, (150.0, 140.0)), None);
}

#[test]
fn the_hit_rectangles_exclude_their_right_and_bottom_edges() {
    // Win32 PtInRect.
    assert!(in_rect(LEFT_LIST, (4.0, 29.0)));
    assert!(in_rect(LEFT_LIST, (94.9, 294.9)));
    assert!(!in_rect(LEFT_LIST, (95.0, 100.0)));
    assert!(!in_rect(LEFT_LIST, (50.0, 295.0)));
    assert!(!in_rect(LEFT_LIST, (3.9, 100.0)));
    assert!(!in_rect(LEFT_LIST, (50.0, 28.9)));
}

#[test]
fn the_fleet_picture_is_centred_on_whole_pixels_in_its_panel() {
    // FUN_004a5c00: 10425 is 122 wide in the 125-wide panel at x 100.
    assert_eq!(picture_x(122.0), 101.0);
    assert_eq!(picture_x(119.0), 103.0);
    assert_eq!(picture_x(125.0), 100.0);
}

#[test]
fn the_normal_tab_art_counts_up_from_the_base() {
    // FUN_004a4b10: normal b + k.
    assert_eq!(
        tab_resource(10409, FleetWindowTab::Fighters, false, true),
        10410
    );
    assert_eq!(
        tab_resource(10459, FleetWindowTab::Personnel, false, true),
        10462
    );
}

#[test]
fn the_fleet_icon_falls_back_to_the_other_side_at_an_unheld_system() {
    // FUN_0045ccc0: the player's side, then the other side, before the
    // system's own side bits.
    for (player, enemy_is_alliance, side) in
        [(Faction::Alliance, false, 2), (Faction::Empire, true, 1)]
    {
        let (mut world, system) = world(ControlKind::Uncontrolled);
        add_fleet(&mut world, system, enemy_is_alliance, 0);
        crate::viewed_contents::see_live(&mut world, player, system);
        let mut fog = FogState::new(player);
        fog.reveal(system);
        assert_eq!(
            icon_side(
                &world,
                &rebellion_core::movement::MovementState::default(),
                &fog,
                player,
                system
            ),
            (side, 1),
            "{player:?}"
        );
    }
}

// A0 fleet2/umgul-empty and system-windows/09-fleet: the Fleet window
// shows each title button's green key pixel (0, 255, 0) at its (x, 16),
// the close box's too (FUN_004a4b10's buttons over the surface +0xa4),
// not the title strip under it.
#[test]
fn the_close_boxs_key_pixel_shows_its_green() {
    let (world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let transport = TroopTransportState::default();
    let mut state = opened(&world, system);
    KEY_PIXELS.with(|pixels| pixels.borrow_mut().clear());
    let _ = run(
        &world,
        &transport,
        &mut state,
        CockpitFaction::Alliance,
        hover(at(-50.0, 0.0)),
    );
    let pixels = KEY_PIXELS.with(|pixels| pixels.take());
    assert!(
        pixels.contains(&(KeyPixel::Colour(CLOSE_KEY), at(218.0, 16.0))),
        "{pixels:?}"
    );
    assert_eq!(CLOSE_KEY, egui::Color32::from_rgb(0, 255, 0));
}

#[test]
fn a_frozen_copy_lists_the_other_sides_fleets_as_seen_after_they_have_died() {
    // FUN_004f2d10 reads the player's copy; FUN_004f9a60 froze it with
    // an Imperial fleet in orbit; GameObj_SetReadyForDelete
    // (FUN_004f82d0 -> FUN_004f6c70) keeps the copy until the side
    // sees the system again. The Fleet window, its icon and the Fleet
    // Finder list the fleet as seen, its ships and squadrons in the
    // right list, after it has died.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    let imperial = add_fleet(&mut world, system, false, 0);
    let tie = world.fighter_classes.insert(rebellion_core::world::FighterClass {
        dat_id: DatId::new(0x1c00_0005),
        name: "TIE Fighter".into(),
        ..Default::default()
    });
    world.fleets[imperial].fighters.push(rebellion_core::world::FighterEntry {
        class: tie,
        count: 2,
        carrier: None,
        damage: None,
    });
    world.fleets[imperial].capital_ships[0].hull_current = 40;
    // A second ship at full hull is not damaged (100 of 100).
    let full = world.fleets[imperial].capital_ships[0].class;
    world.fleets[imperial]
        .capital_ships
        .push(ShipInstance::new(full, 100, true));
    world.capital_ship_classes[full].fighter_capacity = 4;
    let name = world.fleet_name(imperial).unwrap().to_owned();
    let missions = rebellion_core::missions::MissionState::new();
    let movement = MovementState::default();
    let transport = TroopTransportState::default();
    let copy = rebellion_core::object_views::SystemView::of(&world, &world.systems[system], false);
    let extras =
        rebellion_core::object_views::SystemExtras::of(&world, &movement, &missions, system, false);
    let details = rebellion_core::object_views::SystemDetails::of(
        &world, &movement, &missions, &transport, system,
    );
    let id = world.systems[system].dat_id;
    world.object_views.alliance_systems.insert(id, copy);
    world.object_views.alliance_extras.insert(id, extras);
    world.object_views.alliance_details.insert(id, details);
    world.fleets.remove(imperial);
    world.systems[system].fleets.clear();
    let unseen = FogState::new(Faction::Alliance);
    let entries = copy_entries(&world, &unseen, Faction::Alliance, system);
    let copied = FleetWindowEntry::Copy {
        alliance: true,
        system,
        fleet: 0,
    };
    assert_eq!(entries, [copied]);
    assert_eq!(copied.copy(&world).unwrap().name, name);
    assert_eq!(copied.fleet(), FleetKey::default());
    assert_eq!(icon_side(&world, &movement, &unseen, Faction::Alliance, system), (2, 1));
    let ships = right_items(&world, &transport, &[], Some(copied), FleetWindowTab::CapitalShips);
    assert_eq!(ships.len(), 2);
    assert!(ships[0].damaged);
    assert!(!ships[1].damaged);
    let fighters = right_items(&world, &transport, &[], Some(copied), FleetWindowTab::Fighters);
    assert_eq!(
        fighters.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(),
        ["TIE Fighter", "TIE Fighter"]
    );
    assert!(tab_enabled(&world, &transport, &[], Some(copied), FleetWindowTab::CapitalShips));
    // The copy's squadrons against its ships' room (2 of 2 x 4).
    assert_eq!(tab_counts(&world, &transport, Some(copied), FleetWindowTab::Fighters), Some((2, 8)));
    assert!(tab_enabled(&world, &transport, &[], Some(copied), FleetWindowTab::Fighters));
    assert!(!tab_enabled(&world, &transport, &[], Some(copied), FleetWindowTab::Troops));
    assert!(!tab_enabled(&world, &transport, &[], Some(copied), FleetWindowTab::Personnel));
    // The Fleet Finder's ships view lists the copied ship by class.
    let ship_rows = crate::fleet_finder::rows(
        &world,
        &movement,
        &unseen,
        Faction::Alliance,
        crate::fleet_finder::FinderMode::Ships,
        crate::fleet_finder::FinderTab::Imperial,
    );
    assert_eq!(
        ship_rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
        ["Transport", "Transport"]
    );
    let rows = crate::fleet_finder::rows(
        &world,
        &movement,
        &unseen,
        Faction::Alliance,
        crate::fleet_finder::FinderMode::Fleets,
        crate::fleet_finder::FinderTab::Imperial,
    );
    assert_eq!(rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(), [name.as_str()]);
    // Without the copy, nothing of the Empire's.
    world.object_views.alliance_details.clear();
    assert!(copy_entries(&world, &unseen, Faction::Alliance, system).is_empty());
    assert!(crate::fleet_finder::rows(
        &world,
        &movement,
        &unseen,
        Faction::Alliance,
        crate::fleet_finder::FinderMode::Fleets,
        crate::fleet_finder::FinderTab::Imperial,
    )
    .is_empty());
}

#[test]
fn a_fleet_copied_with_its_name_only_opens_four_lit_empty_tabs() {
    // A0 2026-11-14-stale-views/06-kothlis-fleet16-d374-montage (sha256
    // prefix 955559b56955015b): Imperial Fleet 16, made in Kothlis after
    // the Alliance last saw it, lists with all four tabs selectable and
    // empty, Fighters and Troops printing 0 and 0 (FUN_0053f760's
    // frozen copy; its ships never sync).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    let id = world.systems[system].dat_id;
    world.object_views.alliance_systems.insert(
        id,
        rebellion_core::object_views::SystemView::of(&world, &world.systems[system], false),
    );
    world.object_views.alliance_extras.insert(id, Default::default());
    world.object_views.alliance_details.insert(
        id,
        rebellion_core::object_views::SystemDetails {
            fleets: vec![rebellion_core::object_views::FleetDetail {
                serial: 16,
                name: "Fleet 16".into(),
                is_alliance: false,
                ships: Vec::new(),
                fighters: Vec::new(),
                troops: Vec::new(),
                characters: Vec::new(),
            }],
            missions: Vec::new(),
            last_serial: 15,
        },
    );
    let unseen = FogState::new(Faction::Alliance);
    let transport = TroopTransportState::default();
    let [copied] = copy_entries(&world, &unseen, Faction::Alliance, system)[..] else {
        panic!("one copy");
    };
    assert_eq!(copied.copy(&world).unwrap().name, "Fleet 16");
    for tab in [
        FleetWindowTab::CapitalShips,
        FleetWindowTab::Fighters,
        FleetWindowTab::Troops,
        FleetWindowTab::Personnel,
    ] {
        assert!(right_items(&world, &transport, &[], Some(copied), tab).is_empty());
        assert!(tab_enabled(&world, &transport, &[], Some(copied), tab));
    }
    // The Fighters and Troops tabs print 0 and 0.
    assert_eq!(tab_counts(&world, &transport, Some(copied), FleetWindowTab::Fighters), Some((0, 0)));
    assert_eq!(tab_counts(&world, &transport, Some(copied), FleetWindowTab::Troops), Some((0, 0)));
    assert_eq!(tab_counts(&world, &transport, Some(copied), FleetWindowTab::Personnel), None);
}
