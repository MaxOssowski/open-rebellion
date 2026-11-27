use super::*;
use rebellion_core::dat::{ExplorationStatus, SectorGroup};
use rebellion_core::ids::DatId;
use rebellion_core::world::{CapitalShipClass, ControlKind, Fleet, Sector, ShipInstance, System};

fn system(sector: rebellion_core::ids::SectorKey, x: u16) -> System {
    System {
        last_arrival: None,
        dat_id: DatId::new(0x9000_0000),
        name: format!("Sys{x}"),
        sector,
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
        control: ControlKind::Uncontrolled,
    }
}

/// Two Alliance fleets at one system, and a second system 40 away.
// FUN_004ff8e0 (a fleet's orders) and FUN_00507290 (the fleet icon's):
// the order's team is the fleet, or the side's fleets at the icon's
// system, and its target the system they orbit.
#[test]
fn the_bombardment_team_is_the_fleet_or_the_sides_fleets_at_the_icon() {
    let (mut world, fleets, _) = world_with_two_fleets();
    let origin = world.fleets[fleets[0]].location;
    let enemy = world.fleets.insert(Fleet {
        location: origin,
        capital_ships: vec![],
        fighters: vec![],
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    world.systems[origin].fleets.push(enemy);
    let icon = |quadrant| {
        Some(MenuObject::SystemIcon {
            system: origin,
            quadrant,
        })
    };
    assert_eq!(
        bombard_team(
            &MovementState::new(),
            &world,
            Some(MenuObject::Fleet(fleets[1])),
            true
        ),
        Some((vec![fleets[1]], origin))
    );
    assert_eq!(
        bombard_team(&MovementState::new(), &world, icon(Quadrant::Fleets), true),
        Some((fleets.clone(), origin))
    );
    assert_eq!(
        bombard_team(&MovementState::new(), &world, icon(Quadrant::Fleets), false),
        Some((vec![enemy], origin))
    );
    assert_eq!(
        bombard_team(
            &MovementState::new(),
            &world,
            icon(Quadrant::Defenses),
            true
        ),
        None
    );
    assert_eq!(
        bombard_team(&MovementState::new(), &world, None, true),
        None
    );
}

fn world_with_two_fleets() -> (GameWorld, Vec<FleetKey>, SystemKey) {
    let mut world = GameWorld::default();
    let sector = world.sectors.insert(Sector {
        dat_id: DatId::new(0x9200_0000),
        name: "Test".into(),
        group: SectorGroup::Core,
        x: 0,
        y: 0,
        systems: vec![],
    });
    let origin = world.systems.insert(system(sector, 0));
    let destination = world.systems.insert(system(sector, 40));
    let class = world.capital_ship_classes.insert(CapitalShipClass {
        name: "TestShip".into(),
        is_alliance: true,
        hull: 100,
        hyperdrive: 80,
        ..CapitalShipClass::default()
    });
    let fleets = (0..2)
        .map(|_| {
            let fleet = world.fleets.insert(Fleet {
                location: origin,
                capital_ships: vec![ShipInstance::new(class, 100, true)],
                fighters: vec![],
                characters: vec![],
                is_alliance: true,
                has_death_star: false,
                special_forces: Vec::new(),
            });
            world.systems[origin].fleets.push(fleet);
            fleet
        })
        .collect();
    (world, fleets, destination)
}

/// An Alliance Army Regiment (TROOPSD 1) aboard the first fleet.
fn embarked_regiment(
    world: &mut GameWorld,
    fleet: FleetKey,
) -> (TroopTransportState, rebellion_core::ids::TroopKey) {
    let class = DatId::new(0x1000_0001);
    world.buildable_classes.insert(
        class,
        rebellion_core::world::BuildableClass {
            name: "Alliance Army Regiment".into(),
            is_alliance: true,
            ..Default::default()
        },
    );
    let troop = world.troops.insert(rebellion_core::world::TroopUnit {
        class_dat_id: class,
        is_alliance: true,
        regiment_strength: 100,
    });
    let ship = world.fleets[fleet].capital_ships[0].class;
    world.capital_ship_classes[ship].troop_capacity = 2;
    let origin = world.fleets[fleet].location;
    world.systems[origin].control = ControlKind::Controlled(Faction::Alliance);
    world.systems[origin].ground_units.push(troop);
    let mut transport = TroopTransportState::default();
    transport
        .embark(world, fleet, &[troop])
        .expect("the regiment boards");
    world.systems[origin]
        .ground_units
        .retain(|&key| key != troop);
    (transport, troop)
}

#[test]
fn a_units_move_off_its_sides_blockaded_planet_asks_and_aboard_a_fleet_does_not() {
    // FUN_00487cc0 (0x00487cc0..0x00487e13): a 0x201 asks when the first
    // member's container is a system with the blockade bit of the
    // member's side; aboard a fleet the container is the fleet.
    let (mut world, _, system) = world_with_two_fleets();
    world.systems[system].control = ControlKind::Controlled(Faction::Alliance);
    let blockaded: HashSet<SystemKey> = [system].into_iter().collect();
    assert!(move_out_of_blockade_asks(
        &world,
        &blockaded,
        Some(system),
        MissionFaction::Alliance
    ));
    // Another side's member there, or a member aboard a fleet, does not.
    assert!(!move_out_of_blockade_asks(
        &world,
        &blockaded,
        Some(system),
        MissionFaction::Empire
    ));
    assert!(!move_out_of_blockade_asks(
        &world,
        &blockaded,
        None,
        MissionFaction::Alliance
    ));
    // No blockade there.
    assert!(!move_out_of_blockade_asks(
        &world,
        &HashSet::new(),
        Some(system),
        MissionFaction::Alliance
    ));
}

#[test]
fn an_embarked_regiments_confirmed_move_asks_with_its_name() {
    // FUN_00487cc0 asks for every 0x202, and FUN_0049a350 lists the
    // regiment by its class's name (FUN_0053c2e0), wherever it stands:
    // aboard a fleet as on a planet.
    let (mut world, fleets, _) = world_with_two_fleets();
    let origin = world.fleets[fleets[0]].location;
    let (transport, troop) = embarked_regiment(&mut world, fleets[0]);
    let mut log = MessageLog::new();
    let mut confirmation = MoveConfirmationState::default();
    confirm_troop_move(
        &world,
        &MovementState::new(),
        &transport,
        &HashSet::new(),
        MissionFaction::Alliance,
        troop,
        (origin, RegimentTarget::System(origin)),
        0,
        &mut log,
        &mut confirmation,
    );
    assert!(log.messages().is_empty(), "{:?}", log.messages());
    let window = confirmation.confirmation().expect("the confirmation opens");
    assert_eq!(window.lines, [("Alliance Army Regiment".to_string(), 0)]);
}

struct Issued {
    actions: Vec<PanelAction>,
    confirmation: MoveConfirmationState,
    log: MessageLog,
}

fn issue(world: &GameWorld, fleets: &[FleetKey], confirmed: bool, to: SystemKey) -> Issued {
    issue_onto(world, fleets, confirmed, ReleaseTarget::System(to))
}

fn issue_onto(
    world: &GameWorld,
    fleets: &[FleetKey],
    confirmed: bool,
    target: ReleaseTarget,
) -> Issued {
    let movement = MovementState::new();
    let blockaded = HashSet::new();
    let mut issued = Issued {
        actions: Vec::new(),
        confirmation: MoveConfirmationState::default(),
        log: MessageLog::new(),
    };
    issue_fleet_move(
        &FleetMoveContext {
            world,
            movement: &movement,
            blockaded: &blockaded,
            faction: MissionFaction::Alliance,
            tick: 0,
            icon: true,
        },
        fleets,
        confirmed,
        target,
        &mut issued.log,
        &mut issued.confirmation,
        &mut issued.actions,
    );
    issued
}

fn dispatched(actions: &[PanelAction]) -> Vec<FleetKey> {
    actions
        .iter()
        .filter_map(|action| match action {
            PanelAction::DispatchFleet { fleet, .. } => Some(*fleet),
            _ => None,
        })
        .collect()
}

#[test]
fn a_fleet_icons_move_departs_every_fleet_of_the_team() {
    // FUN_00486fb0 -> FUN_00487740: a valid Move that need not ask
    // submits the order, and every sub-order departs.
    let (world, fleets, destination) = world_with_two_fleets();
    let issued = issue(&world, &fleets, false, destination);
    assert_eq!(dispatched(&issued.actions), fleets);
    assert!(!issued.confirmation.is_open());
}

#[test]
fn a_fleet_icons_confirmed_move_asks_once_for_the_whole_team() {
    // FUN_00487cc0 asks for 0x202; the window lists a line per member
    // (FUN_0053c2e0) and departs nothing until the checkmark.
    let (world, fleets, destination) = world_with_two_fleets();
    let issued = issue(&world, &fleets, true, destination);
    assert!(issued.actions.is_empty());
    let window = issued
        .confirmation
        .confirmation()
        .expect("the window opens");
    assert_eq!(window.fleets, fleets);
    assert_eq!(window.lines.len(), 2);
    // FUN_0049a350: each line counts the member's days to go.
    assert!(window.lines.iter().all(|(_, days)| *days > 0));
}

#[test]
fn a_fleet_icons_move_onto_a_fleet_in_its_own_system_joins_each_member() {
    // FUN_00486fb0: a target fleet makes each sub-order a join
    // (FUN_004ffc90), which needs no departure where the target lies.
    let (mut world, fleets, _) = world_with_two_fleets();
    let origin = world.fleets[fleets[0]].location;
    let target = world.fleets.insert(world.fleets[fleets[0]].clone());
    world.systems[origin].fleets.push(target);
    let issued = issue_onto(
        &world,
        &fleets,
        false,
        ReleaseTarget::Fleet {
            fleet: target,
            system: origin,
        },
    );
    let joined: Vec<FleetKey> = issued
        .actions
        .iter()
        .filter_map(|action| match action {
            PanelAction::JoinFleet {
                mover: FleetMover::Fleet(fleet),
                target: onto,
            } if *onto == target => Some(*fleet),
            _ => None,
        })
        .collect();
    assert_eq!(joined, fleets);
    assert!(issued.log.messages().is_empty());
}

#[test]
fn an_empty_team_moving_onto_a_fleet_is_refused() {
    // FUN_0053c4b0 refuses a move with no members (1/0x16) before any
    // sub-order runs.
    let (world, fleets, _) = world_with_two_fleets();
    let origin = world.fleets[fleets[0]].location;
    let issued = issue_onto(
        &world,
        &[],
        false,
        ReleaseTarget::Fleet {
            fleet: fleets[0],
            system: origin,
        },
    );
    assert!(issued.actions.is_empty());
    assert_eq!(issued.log.messages().len(), 1);
}

#[test]
fn a_bands_stop_and_destination_belong_to_its_systems_holder() {
    // Manual p. 83-84: a construction yard's band takes Destination and
    // Stop; Stop clears what the band is building. Either side holds
    // its own bands.
    for (holder, other) in [
        (MissionFaction::Alliance, MissionFaction::Empire),
        (MissionFaction::Empire, MissionFaction::Alliance),
    ] {
        let mut world = GameWorld::default();
        let control = rebellion_core::world::ControlKind::Controlled(match holder {
            MissionFaction::Alliance => Faction::Alliance,
            MissionFaction::Empire => Faction::Empire,
        });
        let system = world.systems.insert(rebellion_core::world::System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
            name: "Bortras".into(),
            sector: rebellion_core::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
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
        });
        let class = world
            .capital_ship_classes
            .insert(rebellion_core::world::CapitalShipClass {
                is_alliance: true,
                is_empire: true,
                // A listed class costs refined material (FUN_00567880).
                refined_material_cost: 10,
                ..rebellion_core::world::CapitalShipClass::default()
            });
        let mut manufacturing = ManufacturingState::new();
        let band = Some(MenuObject::Producer {
            system,
            area: rebellion_core::manufacturing::ProductionArea::Shipyard,
        });
        let gates = |world: &GameWorld, manufacturing: &ManufacturingState, player| {
            order_gates(
                band,
                world,
                &MissionState::new(),
                &MovementState::new(),
                &TroopTransportState::new(),
                &DeliveryState::new(),
                manufacturing,
                &rebellion_core::research::ResearchState::new(),
                player,
                0,
            )
        };
        // Build's validation (FUN_00527d00) wants a usable yard: A0
        // menus3/30 greys Build on a ship band with no shipyard (0:0).
        let bare = gates(&world, &manufacturing, holder);
        assert!(bare.destination && !bare.build && !bare.stop);
        let yard = world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2800_0001),
                side: match holder {
                    MissionFaction::Alliance => Faction::Alliance,
                    MissionFaction::Empire => Faction::Empire,
                },
                is_shipyard: true,
            },
        );
        world.systems[system].manufacturing_facilities.push(yard);
        let idle = gates(&world, &manufacturing, holder);
        assert!(idle.destination && idle.build && !idle.stop);
        // Reserved (0x216) as Destination, checked while the manager is
        // Reserved (A0 menus3/30-33).
        assert!(idle.reserved && !idle.reserved_checked);
        manufacturing.set_reserved(
            system,
            rebellion_core::manufacturing::ProductionArea::Shipyard,
            true,
        );
        assert!(gates(&world, &manufacturing, holder).reserved_checked);
        manufacturing.set_reserved(
            system,
            rebellion_core::manufacturing::ProductionArea::Shipyard,
            false,
        );
        manufacturing.build(
            system,
            &rebellion_core::manufacturing::QueueItem::new(
                rebellion_core::manufacturing::BuildableKind::CapitalShip(class),
                10,
            ),
            1,
        );
        let building = gates(&world, &manufacturing, holder);
        assert!(building.destination && building.stop);
        assert!(!building.orders_stopped);
        // FUN_0051de80: the game over bit of the victory conditions word
        // stops every order (A0 pending-facility/33, day 633).
        let over = order_gates(
            band,
            &world,
            &MissionState::new(),
            &MovementState::new(),
            &TroopTransportState::new(),
            &DeliveryState::new(),
            &manufacturing,
            &rebellion_core::research::ResearchState::new(),
            holder,
            rebellion_core::victory::WORD_GAME_OVER | rebellion_core::victory::WORD_EMPIRE_WINS,
        );
        assert!(over.orders_stopped && over.stop);
        let foreign = gates(&world, &manufacturing, other);
        assert!(!foreign.destination && !foreign.build && !foreign.stop && !foreign.reserved);
        // With nothing its side may build, Build stays off.
        world.capital_ship_classes[class].research_order = 1;
        assert!(!gates(&world, &manufacturing, holder).build);
    }
}

#[test]
fn a_fleet_icons_move_is_enabled_only_where_its_side_has_fleets() {
    // FUN_0051d990 asks each order's +0x18; Move on the fleet icon
    // (kind 0x10) needs a team (FUN_0053c4b0) that may all move.
    let (world, fleets, destination) = world_with_two_fleets();
    let origin = world.fleets[fleets[0]].location;
    let gate = |system| {
        order_gates(
            Some(MenuObject::SystemIcon {
                system,
                quadrant: Quadrant::Fleets,
            }),
            &world,
            &MissionState::new(),
            &MovementState::new(),
            &TroopTransportState::new(),
            &DeliveryState::new(),
            &ManufacturingState::new(),
            &rebellion_core::research::ResearchState::new(),
            MissionFaction::Alliance,
            0,
        )
        .fleet_move
    };
    assert!(gate(origin));
    assert!(!gate(destination));
}

#[test]
fn a_sector_icons_scrap_takes_its_kinds_objects_of_the_side() {
    // FUN_00512700: the fleets icon's Scrap gives the system's fleets of
    // the order's side; an empty team refuses (FUN_0053c4b0), and one
    // member en route refuses them all (FUN_0053c1a0). A0 menus3/07,
    // 08: the Empire's fleet icon over Averam enables Scrap and asks
    // for Fleet 1.
    let (mut world, fleets, destination) = world_with_two_fleets();
    let origin = world.fleets[fleets[0]].location;
    let mut movement = MovementState::new();
    let gates = |world: &GameWorld, movement: &MovementState, system, player| {
        order_gates(
            Some(MenuObject::SystemIcon {
                system,
                quadrant: rebellion_render::quadrant_icons::Quadrant::Fleets,
            }),
            world,
            &MissionState::new(),
            movement,
            &TroopTransportState::new(),
            &DeliveryState::new(),
            &ManufacturingState::new(),
            &rebellion_core::research::ResearchState::new(),
            player,
            0,
        )
        .scrap
    };
    assert!(gates(&world, &movement, origin, MissionFaction::Alliance));
    assert!(!gates(&world, &movement, origin, MissionFaction::Empire));
    assert!(!gates(
        &world,
        &movement,
        destination,
        MissionFaction::Alliance
    ));
    let icon = MenuObject::SystemIcon {
        system: origin,
        quadrant: rebellion_render::quadrant_icons::Quadrant::Fleets,
    };
    assert_eq!(
        icon.scrap_team(&movement, &world, Faction::Alliance),
        fleets
            .iter()
            .map(|&fleet| rebellion_core::scrap::ScrapTarget::Fleet(fleet))
            .collect::<Vec<_>>()
    );
    rebellion_core::movement::begin_faction_fleet_transit(
        &mut movement,
        &mut world,
        fleets[0],
        destination,
        true,
    )
    .unwrap();
    // The en-route fleet left the orbit index; the other still may go.
    assert!(gates(&world, &movement, origin, MissionFaction::Alliance));
    // At its destination it is a member that refuses (FUN_00556390 puts
    // it there at once): A0 menus3/23 greys the icon's Move and Scrap
    // over a fleet arriving at Chandrila.
    assert!(!gates(
        &world,
        &movement,
        destination,
        MissionFaction::Alliance
    ));
    let move_gate = |world: &GameWorld, movement: &MovementState, system| {
        order_gates(
            Some(MenuObject::SystemIcon {
                system,
                quadrant: rebellion_render::quadrant_icons::Quadrant::Fleets,
            }),
            world,
            &MissionState::new(),
            movement,
            &TroopTransportState::new(),
            &DeliveryState::new(),
            &ManufacturingState::new(),
            &rebellion_core::research::ResearchState::new(),
            MissionFaction::Alliance,
            0,
        )
        .fleet_move
    };
    assert!(move_gate(&world, &movement, origin));
    assert!(!move_gate(&world, &movement, destination));
    // One orbiting there too: the arriving member still refuses them all.
    let staying = fleets[1];
    world.systems[origin].fleets.retain(|&f| f != staying);
    world.fleets[staying].location = destination;
    world.systems[destination].fleets.push(staying);
    assert!(!gates(
        &world,
        &movement,
        destination,
        MissionFaction::Alliance
    ));
    assert!(!move_gate(&world, &movement, destination));
}

#[test]
fn a_fleet_in_hyperspace_takes_neither_rename_nor_scrap() {
    // FUN_004f9860 refuses an object en route (status 4): A0 greys
    // Rename and Scrap on a fleet in hyperspace and on its ship
    // (a0-vm/menus/22-fleet4-menu.png, 29-own-ship-menu.png).
    let (mut world, fleets, destination) = world_with_two_fleets();
    let fleet = fleets[0];
    let mut movement = MovementState::new();
    let gates_for = |world: &GameWorld, movement: &MovementState, selection, player| {
        order_gates(
            Some(selection),
            world,
            &MissionState::new(),
            movement,
            &TroopTransportState::new(),
            &DeliveryState::new(),
            &ManufacturingState::new(),
            &rebellion_core::research::ResearchState::new(),
            player,
            0,
        )
    };
    let gates = |world: &GameWorld, movement: &MovementState, selection| {
        gates_for(world, movement, selection, MissionFaction::Alliance)
    };
    let ship = MenuObject::Ship {
        fleet,
        index: 0,
        roster: 0,
    };
    for selection in [MenuObject::Fleet(fleet), ship] {
        let home = gates(&world, &movement, selection);
        assert!(home.rename && home.scrap, "{selection:?}");
        // The other side's fleet takes neither (A0 status/02, 05).
        let theirs = gates_for(&world, &movement, selection, MissionFaction::Empire);
        assert!(!theirs.rename && !theirs.scrap, "{selection:?}");
    }
    rebellion_core::movement::begin_faction_fleet_transit(
        &mut movement,
        &mut world,
        fleet,
        destination,
        true,
    )
    .unwrap();
    for selection in [MenuObject::Fleet(fleet), ship] {
        let away = gates(&world, &movement, selection);
        assert!(!away.rename && !away.scrap, "{selection:?}");
    }
}

#[test]
fn one_refused_fleet_keeps_the_whole_team_home() {
    // FUN_0053c1a0 runs every sub-order's validator; FUN_00487c90 reports
    // the refusal and nothing departs.
    let (mut world, fleets, destination) = world_with_two_fleets();
    world.fleets[fleets[1]].capital_ships.clear();
    let issued = issue(&world, &fleets, false, destination);
    assert!(issued.actions.is_empty());
    assert!(!issued.confirmation.is_open());
    assert_eq!(issued.log.messages().len(), 1);
}

#[test]
fn a_fleet_icons_move_never_asks_even_out_of_a_blockaded_system() {
    // FUN_00487cc0 (0x00487cc0..0x00487e13) asks a 0x201 only when its
    // first team id's container is a blockaded system of its side. A
    // fleet's first id is the fleet, in its system; a fleet icon's is
    // the system (FUN_004f5b10), in its sector, so it never asks.
    let (mut world, fleets, destination) = world_with_two_fleets();
    let origin = world.fleets[fleets[0]].location;
    world.systems[origin].control = ControlKind::Controlled(Faction::Alliance);
    let movement = MovementState::new();
    let blockaded: HashSet<SystemKey> = [origin].into_iter().collect();
    let run = |team: &[FleetKey], icon: bool, confirmed: bool| {
        let mut issued = Issued {
            actions: Vec::new(),
            confirmation: MoveConfirmationState::default(),
            log: MessageLog::new(),
        };
        issue_fleet_move(
            &FleetMoveContext {
                world: &world,
                movement: &movement,
                blockaded: &blockaded,
                faction: MissionFaction::Alliance,
                tick: 0,
                icon,
            },
            team,
            confirmed,
            ReleaseTarget::System(destination),
            &mut issued.log,
            &mut issued.confirmation,
            &mut issued.actions,
        );
        issued
    };
    let icon = run(&fleets, true, false);
    assert!(!icon.confirmation.is_open());
    assert_eq!(dispatched(&icon.actions), fleets);
    // Its Confirmed Move (0x202) still asks.
    assert!(run(&fleets, true, true).confirmation.is_open());
    // One fleet's own Move out of the blockade asks.
    let own = run(&fleets[..1], false, false);
    assert!(own.confirmation.is_open());
    assert!(own.actions.is_empty());
}

#[test]
fn a_characters_menu_move_is_disabled_while_it_is_on_a_mission_or_travelling() {
    // The 0x241's +0x18: FUN_0051fe20 -> command 0x250's FUN_0054d1a0 ->
    // FUN_005429e0 -> FUN_0054c110, member slot +0x1c8, refuses a
    // member on a mission (+0x78 bit 7) or en route (+0x50 bit 4).
    use rebellion_core::world::Character;
    let (mut world, _, destination) = world_with_two_fleets();
    let agent = world.characters.insert(Character {
        name: "Agent".into(),
        is_alliance: true,
        current_system: Some(destination),
        recruited: true,
        ..Character::default()
    });
    let gate = |world: &GameWorld, missions: &MissionState| {
        order_gates(
            Some(MenuObject::Character(agent)),
            world,
            missions,
            &MovementState::new(),
            &TroopTransportState::new(),
            &DeliveryState::new(),
            &ManufacturingState::new(),
            &rebellion_core::research::ResearchState::new(),
            MissionFaction::Alliance,
            0,
        )
        .character_move
    };
    assert!(gate(&world, &MissionState::new()));
    let mut travelling = MissionState::new();
    travelling.dispatch(rebellion_core::missions::MissionRequest::single(
        MissionKind::Move,
        MissionFaction::Alliance,
        agent,
        destination,
        None,
        0,
    ));
    assert!(!gate(&world, &travelling), "a member of a mission");
    world.characters[agent].on_mission = true;
    assert!(!gate(&world, &MissionState::new()), "on a mission");
}

#[test]
fn a_special_forces_menu_move_is_disabled_while_it_is_on_a_mission_or_travelling() {
    // FUN_0054c110 asks the unit's slot +0x1c8 (FUN_00533ce0, over
    // FUN_004f9860): another side's, on a mission or en route refuses.
    use rebellion_core::world::SpecialForceUnit;
    let (mut world, _, destination) = world_with_two_fleets();
    let unit = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
        on_hidden_mission: false,
    });
    world.systems[destination].special_forces.push(unit);
    let gate = |world: &GameWorld, missions: &MissionState, faction| {
        order_gates(
            Some(MenuObject::SpecialForce(unit)),
            world,
            missions,
            &MovementState::new(),
            &TroopTransportState::new(),
            &DeliveryState::new(),
            &ManufacturingState::new(),
            &rebellion_core::research::ResearchState::new(),
            faction,
            0,
        )
        .special_force_move
    };
    let free = MissionState::new();
    assert!(gate(&world, &free, MissionFaction::Alliance));
    assert!(
        !gate(&world, &free, MissionFaction::Empire),
        "another side's"
    );
    let mut travelling = MissionState::new();
    travelling.dispatch(rebellion_core::missions::MissionRequest {
        kind: MissionKind::Move,
        faction: MissionFaction::Alliance,
        team: vec![MissionMember::SpecialForce(unit)],
        decoys: Vec::new(),
        target_system: destination,
        target_character: None,
        target_object: None,
        tick: 0,
    });
    assert!(
        !gate(&world, &travelling, MissionFaction::Alliance),
        "a member of a mission"
    );
    world.special_forces[unit].on_mission = true;
    assert!(
        !gate(&world, &free, MissionFaction::Alliance),
        "on a mission"
    );
}

/// The Alliance's two held systems 40 apart; at the first, a regiment,
/// two X-wing squadrons, a character and a special force of the
/// Alliance, and the Empire's regiment.
struct DefensesWorld {
    world: GameWorld,
    origin: SystemKey,
    destination: SystemKey,
    troop: rebellion_core::ids::TroopKey,
    x_wing: rebellion_core::ids::FighterKey,
    agent: rebellion_core::ids::CharacterKey,
    force: rebellion_core::ids::SpecialForceKey,
}

fn defenses_world() -> DefensesWorld {
    use rebellion_core::world::{
        BuildableClass, Character, FighterClass, SpecialForceUnit, TroopUnit,
    };
    let (mut world, _, destination) = world_with_two_fleets();
    let origin = world.fleets.values().next().expect("a fleet").location;
    // Only the defenses team stays; the fleets are not kind 8's.
    world.fleets.clear();
    world.systems[origin].fleets.clear();
    for system in [origin, destination] {
        world.systems[system].control = ControlKind::Controlled(Faction::Alliance);
    }
    let named = |world: &mut GameWorld, raw: u32, name: &str| {
        let id = DatId::new(raw);
        world.buildable_classes.insert(
            id,
            BuildableClass {
                name: name.into(),
                ..BuildableClass::default()
            },
        );
        id
    };
    let regiment_class = named(&mut world, 0x1000_0001, "Alliance Army Regiment");
    let force_class = named(&mut world, 0x3c00_0001, "Bothan Spies");
    let troop = world.troops.insert(TroopUnit {
        class_dat_id: regiment_class,
        is_alliance: true,
        regiment_strength: 100,
    });
    let enemy = world.troops.insert(TroopUnit {
        class_dat_id: regiment_class,
        is_alliance: false,
        regiment_strength: 100,
    });
    world.systems[origin].ground_units.extend([troop, enemy]);
    let x_wing = world.fighter_classes.insert(FighterClass {
        name: "X-wing Squadron".into(),
        is_alliance: true,
        ..FighterClass::default()
    });
    rebellion_core::squadrons::add(&mut world.systems[origin].fighters, x_wing, 2);
    let agent = world.characters.insert(Character {
        name: "Agent".into(),
        is_alliance: true,
        current_system: Some(origin),
        recruited: true,
        ..Character::default()
    });
    let force = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: force_class,
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
        on_hidden_mission: false,
    });
    world.systems[origin].special_forces.push(force);
    DefensesWorld {
        world,
        origin,
        destination,
        troop,
        x_wing,
        agent,
        force,
    }
}

fn issue_defenses(
    world: &GameWorld,
    missions: &MissionState,
    system: SystemKey,
    confirmed: bool,
    to: SystemKey,
) -> Issued {
    let movement = MovementState::new();
    let transport = TroopTransportState::new();
    let blockaded = HashSet::new();
    let mut issued = Issued {
        actions: Vec::new(),
        confirmation: MoveConfirmationState::default(),
        log: MessageLog::new(),
    };
    issue_defenses_move(
        &DefensesMoveContext {
            world,
            movement: &movement,
            missions,
            transport: &transport,
            deliveries: &DeliveryState::new(),
            blockaded: &blockaded,
            faction: MissionFaction::Alliance,
            tick: 0,
        },
        system,
        confirmed,
        ReleaseTarget::System(to),
        &mut issued.log,
        &mut issued.confirmation,
        &mut issued.actions,
    );
    issued
}

#[test]
fn a_defenses_icons_move_sends_one_member_mission_and_the_group_at_once() {
    // FUN_0053c4b0 splits the kind 8 team (FUN_00512700): one 0x241 for
    // the characters and special forces (FUN_0053c810), one 0x204 for
    // the regiments and squadrons (FUN_0053ca50). FUN_00487cc0 asks
    // nothing for 0x201: its first team id is the system, whose
    // container is no system (0x00487cc0..0x00487e13).
    let w = defenses_world();
    let issued = issue_defenses(
        &w.world,
        &MissionState::new(),
        w.origin,
        false,
        w.destination,
    );
    assert!(!issued.confirmation.is_open());
    assert!(issued.log.messages().is_empty());
    let to = SquadronPlace::Planet(w.destination);
    assert_eq!(
        format!("{:?}", issued.actions),
        format!(
            "{:?}",
            [
                PanelAction::MoveMembers {
                    members: vec![
                        MissionMember::Character(w.agent),
                        MissionMember::SpecialForce(w.force),
                    ],
                    system: w.destination,
                    fleet: None,
                },
                PanelAction::MoveRegiment {
                    troop: w.troop,
                    target: RegimentTarget::System(w.destination),
                },
                PanelAction::MoveSquadron {
                    class: w.x_wing,
                    from: SquadronPlace::Planet(w.origin),
                    to,
                },
                PanelAction::MoveSquadron {
                    class: w.x_wing,
                    from: SquadronPlace::Planet(w.origin),
                    to,
                },
            ]
        )
    );
}

#[test]
fn a_defenses_icons_confirmed_move_asks_once_with_a_line_per_member() {
    // FUN_00487cc0 asks for 0x202; FUN_0053c2e0 collects a line per
    // member from the 0x241 (FUN_004f4850) then the 0x204
    // (FUN_0053d200). FUN_0049a350 gives no evacuation warning: its
    // first member is the system.
    let w = defenses_world();
    let issued = issue_defenses(
        &w.world,
        &MissionState::new(),
        w.origin,
        true,
        w.destination,
    );
    assert!(issued.actions.is_empty());
    let window = issued
        .confirmation
        .confirmation()
        .expect("the window opens");
    assert_eq!(window.characters, [w.agent]);
    assert_eq!(window.special_forces, [w.force]);
    assert_eq!(window.troops, [w.troop]);
    assert_eq!(window.squadrons.len(), 2);
    assert!(!window.blockaded);
    let names: Vec<&str> = window.lines.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Agent",
            "Bothan Spies",
            "Alliance Army Regiment",
            "X-wing Squadron",
            "X-wing Squadron"
        ]
    );
    assert!(window.lines.iter().all(|(_, days)| *days > 0));
}

#[test]
fn one_busy_member_keeps_the_whole_defenses_team_home() {
    // FUN_0053c1a0 runs every sub-order's validator: the 0x241's
    // member check (FUN_0054c110 -> +0x1c8) refuses a member on a
    // mission, and FUN_00487c90 reports it; nothing moves.
    let mut w = defenses_world();
    w.world.characters[w.agent].on_mission = true;
    let issued = issue_defenses(
        &w.world,
        &MissionState::new(),
        w.origin,
        false,
        w.destination,
    );
    assert!(issued.actions.is_empty());
    assert!(!issued.confirmation.is_open());
    assert_eq!(issued.log.messages().len(), 1);
}

#[test]
fn a_defenses_icons_move_is_enabled_for_a_free_team_of_the_side() {
    // FUN_0053c100: FUN_0053c4b0 refuses an empty team (1/0x16), then
    // each sub-order's +0x18 checks its members.
    let mut w = defenses_world();
    let gate = |world: &GameWorld, system| {
        order_gates(
            Some(MenuObject::SystemIcon {
                system,
                quadrant: Quadrant::Defenses,
            }),
            world,
            &MissionState::new(),
            &MovementState::new(),
            &TroopTransportState::new(),
            &DeliveryState::new(),
            &ManufacturingState::new(),
            &rebellion_core::research::ResearchState::new(),
            MissionFaction::Alliance,
            0,
        )
        .defenses_move
    };
    assert!(gate(&w.world, w.origin));
    assert!(!gate(&w.world, w.destination));
    w.world.characters[w.agent].on_mission = true;
    assert!(!gate(&w.world, w.origin));
}

#[test]
fn a_defenses_icons_move_and_scrap_are_disabled_while_a_squadron_flies_in() {
    // FUN_005039d0 (mode 3) keeps a squadron in flight, as it already sits
    // in its target's container; its own move and scrap commands refuse.
    // A0 2026-11-11-icon-menu-topic/30 (grey) and 33 (white after it lands).
    let mut w = defenses_world();
    let troop = w.world.troops.insert(rebellion_core::world::TroopUnit {
        class_dat_id: DatId::new(0x1000_0001),
        is_alliance: true,
        regiment_strength: 100,
    });
    w.world.systems[w.destination].ground_units.push(troop);
    let gates = |world: &GameWorld, transport: &TroopTransportState| {
        let gates = order_gates(
            Some(MenuObject::SystemIcon {
                system: w.destination,
                quadrant: Quadrant::Defenses,
            }),
            world,
            &MissionState::new(),
            &MovementState::new(),
            transport,
            &DeliveryState::new(),
            &ManufacturingState::new(),
            &rebellion_core::research::ResearchState::new(),
            MissionFaction::Alliance,
            0,
        );
        (gates.defenses_move, gates.scrap)
    };
    let mut transport = TroopTransportState::new();
    assert_eq!(gates(&w.world, &transport), (true, true));
    let x_wing = w.world.systems[w.origin].fighters[0].class;
    let moved = transport
        .squadrons
        .move_squadrons(
            &mut w.world,
            true,
            x_wing,
            1,
            SquadronPlace::Planet(w.origin),
            SquadronPlace::Planet(w.destination),
            0,
        )
        .expect("the squadron departs");
    assert!(matches!(
        moved,
        rebellion_core::squadrons::SquadronMove::Departed(_)
    ));
    assert_eq!(gates(&w.world, &transport), (false, false));
}

#[test]
fn a_defenses_icons_move_and_scrap_are_disabled_while_a_built_regiment_is_on_its_way() {
    // FUN_005150c0 -> FUN_00556430: a product travels already inside its
    // destination's container, en route (ghidra/notes/build-delivery.md),
    // so the defenses icon's walks (mode 3) hold it and its own move and
    // scrap commands refuse it. A0 2026-11-11-icon-menu-topic/00..18:
    // Coruscant's icon grey with every visible member free.
    let mut w = defenses_world();
    let troop = w.world.troops.insert(rebellion_core::world::TroopUnit {
        class_dat_id: DatId::new(0x1000_0001),
        is_alliance: true,
        regiment_strength: 100,
    });
    w.world.systems[w.destination].ground_units.push(troop);
    let gates = |world: &GameWorld, deliveries: &DeliveryState| {
        let gates = order_gates(
            Some(MenuObject::SystemIcon {
                system: w.destination,
                quadrant: Quadrant::Defenses,
            }),
            world,
            &MissionState::new(),
            &MovementState::new(),
            &TroopTransportState::new(),
            deliveries,
            &ManufacturingState::new(),
            &rebellion_core::research::ResearchState::new(),
            MissionFaction::Alliance,
            0,
        );
        (gates.defenses_move, gates.scrap)
    };
    let mut deliveries = DeliveryState::new();
    assert_eq!(gates(&w.world, &deliveries), (true, true));
    deliveries.depart(
        &w.world,
        &[rebellion_core::manufacturing::Departure {
            origin: w.origin,
            destination: w.destination,
            tick: 0,
            kind: rebellion_core::manufacturing::BuildableKind::Troop(DatId::new(0x1000_0001)),
            name: None,
        }],
    );
    assert_eq!(deliveries.en_route()[0].side, Faction::Alliance);
    assert_eq!(gates(&w.world, &deliveries), (false, false));
}

#[test]
fn a_defenses_icons_scrap_is_disabled_while_a_regiment_for_the_system_is_queued_or_on_its_way() {
    // FUN_00512700 kind 8, order 0x200: Scrap's team walks the
    // regiments (FUN_00504c40), squadrons (FUN_00503a50) and defense
    // facilities (FUN_00527050) in mode 3, so an unfinished product
    // ordered for the system, or a finished one travelling in, is in
    // it; its scrap check FUN_004f9860 refuses status 2 (not completed)
    // and 4 (en route) (ghidra/notes/scrap-order.md). Move skips status
    // 2 for 0x204 (FUN_0053c100). A0 2026-11-19-edited-saves-10 (the
    // Empire, Corfai): a-08 all white; c-01 queued, Move white and Scrap
    // grey; c-05 trained and en route, Move and Scrap grey; c-09 white.
    use rebellion_core::manufacturing::{BuildableKind, CompletionEvent, QueueItem};
    let mut w = defenses_world();
    let troop = w.world.troops.insert(rebellion_core::world::TroopUnit {
        class_dat_id: DatId::new(0x1000_0001),
        is_alliance: true,
        regiment_strength: 100,
    });
    w.world.systems[w.destination].ground_units.push(troop);
    if let Some(class) = w.world.buildable_classes.get_mut(&DatId::new(0x1000_0001)) {
        class.is_alliance = true;
    }
    let gates = |world: &GameWorld| {
        let gates = order_gates(
            Some(MenuObject::SystemIcon {
                system: w.destination,
                quadrant: Quadrant::Defenses,
            }),
            world,
            &MissionState::new(),
            &MovementState::new(),
            &TroopTransportState::new(),
            &DeliveryState::new(),
            &ManufacturingState::new(),
            &rebellion_core::research::ResearchState::new(),
            MissionFaction::Alliance,
            0,
        );
        (gates.defenses_move, gates.scrap)
    };
    assert_eq!(gates(&w.world), (true, true));
    let kind = BuildableKind::Troop(DatId::new(0x1000_0001));
    let mut manufacturing = ManufacturingState::new();
    manufacturing.enqueue(
        w.origin,
        QueueItem::new(kind, 10).delivered_to(w.destination),
    );
    rebellion_core::products::reconcile(&mut w.world, &manufacturing);
    assert_eq!(w.world.products.in_system(w.destination).count(), 1);
    assert_eq!(gates(&w.world), (true, false), "queued");
    rebellion_core::products::settle(
        &mut w.world,
        vec![CompletionEvent {
            system: w.origin,
            tick: 0,
            kind,
            name: None,
            fleet: None,
        }],
        Vec::new(),
    );
    let id = w.world.products.all()[0].id;
    let day = w.world.products.arrival(id).expect("on its trip");
    assert_eq!(gates(&w.world), (false, false), "en route");
    rebellion_core::products::arrive(&mut w.world, day);
    assert!(w.world.products.all().is_empty());
    assert_eq!(gates(&w.world), (true, true), "arrived");
}

/// The Alliance's fleet at its held origin carrying two regiments, and
/// the transport that holds them.
fn fleet_with_two_regiments() -> (
    DefensesWorld,
    FleetKey,
    TroopTransportState,
    [rebellion_core::ids::TroopKey; 2],
) {
    let mut w = defenses_world();
    let class = w.world.capital_ship_classes.insert(CapitalShipClass {
        name: "Transport".into(),
        is_alliance: true,
        hull: 100,
        hyperdrive: 80,
        troop_capacity: 4,
        ..CapitalShipClass::default()
    });
    let fleet = w.world.fleets.insert(Fleet {
        location: w.origin,
        capital_ships: vec![ShipInstance::new(class, 100, true)],
        fighters: vec![],
        characters: vec![],
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    w.world.systems[w.origin].fleets.push(fleet);
    let second = w.world.troops.insert(w.world.troops[w.troop].clone());
    w.world.systems[w.origin].ground_units.push(second);
    let mut transport = TroopTransportState::new();
    transport
        .embark(&mut w.world, fleet, &[w.troop, second])
        .expect("both regiments board");
    let troops = [w.troop, second];
    (w, fleet, transport, troops)
}

fn issue_fleet_drop(
    world: &GameWorld,
    transport: &TroopTransportState,
    action: FleetWindowAction,
    to: SystemKey,
) -> Issued {
    let (FleetDropOrder::Team { place, team }, _, confirmed) =
        fleet_drop_order(world, action).expect("a drop order")
    else {
        panic!("a team drop");
    };
    let movement = MovementState::new();
    let blockaded = HashSet::new();
    let mut issued = Issued {
        actions: Vec::new(),
        confirmation: MoveConfirmationState::default(),
        log: MessageLog::new(),
    };
    issue_team_move(
        &DefensesMoveContext {
            world,
            movement: &movement,
            missions: &MissionState::new(),
            transport,
            deliveries: &DeliveryState::new(),
            blockaded: &blockaded,
            faction: MissionFaction::Alliance,
            tick: 0,
        },
        &team,
        place.unwrap_or((to, SquadronPlace::Planet(to))),
        false,
        confirmed,
        ReleaseTarget::System(to),
        &mut issued.log,
        &mut issued.confirmation,
        &mut issued.actions,
    );
    issued
}

#[test]
fn regiments_dropped_from_a_fleet_window_move_as_one_order_that_one_refusal_stops() {
    // FUN_00422ce0, window type 4: the selection's one 0x201; the
    // regiments go in its 0x204 group (FUN_0053c4b0), whose validator
    // FUN_0053c1a0 runs every member's, so one refusal refuses the
    // order (FUN_00487740 -> FUN_00487c90).
    let (w, _, transport, troops) = fleet_with_two_regiments();
    let drop =
        |troops: Vec<rebellion_core::ids::TroopKey>, confirmed| FleetWindowAction::DragRegiment {
            troops,
            point: egui_macroquad::egui::Pos2::ZERO,
            confirmed,
        };
    let both = issue_fleet_drop(
        &w.world,
        &transport,
        drop(troops.to_vec(), false),
        w.destination,
    );
    assert_eq!(
        format!("{:?}", both.actions),
        format!(
            "{:?}",
            troops.map(|troop| PanelAction::MoveRegiment {
                troop,
                target: RegimentTarget::System(w.destination),
            })
        )
    );
    // The second regiment is gone by the release: nothing moves.
    let mut world = w.world.clone();
    world.troops.remove(troops[1]);
    let mut refused = issue_fleet_drop(
        &world,
        &transport,
        drop(troops.to_vec(), false),
        w.destination,
    );
    assert!(refused.actions.is_empty(), "{:?}", refused.actions);
    assert!(!refused.confirmation.is_open());
    // FUN_00487c90 hands the refusal to the side's advisor and files no
    // message (A0 2026-11-13-defenses-missions-drop/30): the port's line
    // stays off every rail.
    assert_eq!(refused.log.take_refusals(), [UNTRACED_REFUSAL]);
    assert!(refused
        .log
        .messages()
        .iter()
        .all(|message| message.rail.is_none()));
    assert_eq!(refused.log.messages().len(), 1);
    // The refusal names the regiment's reason, as its own move's does.
    assert!(
        refused.log.messages()[0]
            .text
            .starts_with("Regiment move rejected: "),
        "{:?}",
        refused.log.messages()[0].text
    );
    // With Ctrl it is 0x202: one confirmation lists both.
    let confirmed = issue_fleet_drop(
        &w.world,
        &transport,
        drop(troops.to_vec(), true),
        w.destination,
    );
    assert!(confirmed.actions.is_empty());
    let window = confirmed
        .confirmation
        .confirmation()
        .expect("the window opens");
    assert_eq!(window.troops, troops);
    assert_eq!(window.lines.len(), 2);
}

#[test]
fn squadrons_dropped_from_a_fleet_window_leave_their_fleet_as_one_order() {
    // FUN_00422ce0, window type 4: the squadrons' one 0x201, out of
    // their fleet (FUN_0053c4b0 -> FUN_0053ca50's 0x204 group).
    let (mut w, fleet, transport, _) = fleet_with_two_regiments();
    rebellion_core::squadrons::add(&mut w.world.fleets[fleet].fighters, w.x_wing, 2);
    let drop = |confirmed| FleetWindowAction::DragSquadron {
        fleet,
        indices: vec![0],
        point: egui_macroquad::egui::Pos2::ZERO,
        confirmed,
    };
    assert_eq!(
        fleet_drop_order(&w.world, drop(false)).map(|(order, _, confirmed)| (order, confirmed)),
        Some((
            FleetDropOrder::Team {
                place: Some((w.origin, SquadronPlace::Fleet(fleet))),
                team: rebellion_render::quadrant_icons::DefensesTeam {
                    squadrons: vec![w.x_wing],
                    ..Default::default()
                },
            },
            false
        ))
    );
    let issued = issue_fleet_drop(&w.world, &transport, drop(true), w.destination);
    let window = issued.confirmation.confirmation().expect("Ctrl asks");
    assert_eq!(window.squadrons, [(w.x_wing, SquadronPlace::Fleet(fleet))]);
}

/// A window team's Move (`icon` false) out of `w.origin` to
/// `w.destination`, with `blockaded` blockaded.
fn issue_blockaded_team(
    w: &DefensesWorld,
    transport: &TroopTransportState,
    team: &rebellion_render::quadrant_icons::DefensesTeam,
    place: (SystemKey, SquadronPlace),
    icon: bool,
    blockaded: &[SystemKey],
) -> Issued {
    let movement = MovementState::new();
    let blockaded: HashSet<SystemKey> = blockaded.iter().copied().collect();
    let mut issued = Issued {
        actions: Vec::new(),
        confirmation: MoveConfirmationState::default(),
        log: MessageLog::new(),
    };
    issue_team_move(
        &DefensesMoveContext {
            world: &w.world,
            movement: &movement,
            missions: &MissionState::new(),
            transport,
            deliveries: &DeliveryState::new(),
            blockaded: &blockaded,
            faction: MissionFaction::Alliance,
            tick: 0,
        },
        team,
        place,
        icon,
        false,
        ReleaseTarget::System(w.destination),
        &mut issued.log,
        &mut issued.confirmation,
        &mut issued.actions,
    );
    issued
}

#[test]
fn a_window_teams_move_off_a_blockaded_planet_of_its_side_asks_with_the_evacuation_warning() {
    // FUN_00487cc0 (0x201): the first team id's container (+0x1c) is a
    // system with +0x88 bit 0x20 whose side bits are the member's, so
    // the order asks (FUN_0048a340); FUN_0049a350 takes 0x7056 for a
    // first member that is no fleet in a blockaded system.
    use rebellion_render::quadrant_icons::DefensesTeam;
    let mut w = defenses_world();
    let transport = TroopTransportState::new();
    let planet = (w.origin, SquadronPlace::Planet(w.origin));
    for team in [
        DefensesTeam {
            troops: vec![w.troop],
            ..DefensesTeam::default()
        },
        DefensesTeam {
            squadrons: vec![w.x_wing, w.x_wing],
            ..DefensesTeam::default()
        },
        DefensesTeam {
            members: vec![
                MissionMember::Character(w.agent),
                MissionMember::SpecialForce(w.force),
            ],
            ..DefensesTeam::default()
        },
    ] {
        let asked = issue_blockaded_team(&w, &transport, &team, planet, false, &[w.origin]);
        assert!(asked.actions.is_empty(), "{team:?}");
        let window = asked.confirmation.confirmation().expect("the move asks");
        assert!(window.blockaded, "{team:?}");
        // Not blockaded: it leaves at once.
        let free = issue_blockaded_team(&w, &transport, &team, planet, false, &[]);
        assert!(!free.confirmation.is_open());
        assert!(!free.actions.is_empty());
        // The defenses icon's first id is the system: it never asks.
        let icon = issue_blockaded_team(&w, &transport, &team, planet, true, &[w.origin]);
        assert!(!icon.confirmation.is_open());
    }
    // Another side's blockaded planet: the side bits differ, no ask.
    w.world.systems[w.origin].control = ControlKind::Controlled(Faction::Empire);
    let team = DefensesTeam {
        troops: vec![w.troop],
        ..DefensesTeam::default()
    };
    let other = issue_blockaded_team(&w, &transport, &team, planet, false, &[w.origin]);
    assert!(!other.confirmation.is_open());
}

#[test]
fn regiments_aboard_a_fleet_in_a_blockaded_system_move_without_asking() {
    // FUN_00487cc0: an embarked regiment's container is its ship, no
    // system (regiment-unload.md).
    let (w, fleet, transport, troops) = fleet_with_two_regiments();
    let team = rebellion_render::quadrant_icons::DefensesTeam {
        troops: troops.to_vec(),
        ..Default::default()
    };
    let issued = issue_blockaded_team(
        &w,
        &transport,
        &team,
        (w.origin, SquadronPlace::Fleet(fleet)),
        false,
        &[w.origin],
    );
    assert!(!issued.confirmation.is_open());
    assert_eq!(issued.actions.len(), 2);
}

#[test]
fn a_menu_move_on_several_selected_fleets_moves_them_as_one_team() {
    // FUN_004ac5c0 takes the Fleet window's left list selection (slot
    // +0x58); its 0x201 has every fleet in its team.
    let (world, fleets, _) = world_with_two_fleets();
    let team: Vec<MenuObject> = fleets
        .iter()
        .map(|&fleet| MenuObject::Fleet(fleet))
        .collect();
    for (command, confirmed) in [
        (ObjectMenuCommand::Move, false),
        (ObjectMenuCommand::ConfirmedMove, true),
    ] {
        let Some(TeamMenuOrder::Target(order)) = team_menu_order(&world, command, &team, None)
        else {
            panic!("a target order");
        };
        assert_eq!(
            order,
            TargetOrder::FleetMove {
                fleets: fleets.clone(),
                confirmed,
                icon: false,
            }
        );
    }
}

#[test]
fn a_team_scrap_asks_once_for_every_member_and_one_refusal_refuses_it() {
    // FUN_005201c0 makes one command per member; FUN_0051ff30 runs each
    // command's check, so one refusal refuses the order; FUN_00487cc0
    // asks for 0x200 and FUN_0049a880 lists each name (0x7054).
    use rebellion_core::scrap::ScrapTarget;
    let mut w = defenses_world();
    let second = w.world.troops.insert(w.world.troops[w.troop].clone());
    w.world.systems[w.origin].ground_units.push(second);
    let movement = MovementState::new();
    let missions = MissionState::new();
    let scrap = |world: &GameWorld, team: &[MenuObject]| {
        team_confirmation(
            world,
            &movement,
            &missions,
            MissionFaction::Alliance,
            ObjectMenuCommand::Scrap,
            team,
        )
    };
    let team = [MenuObject::Troop(w.troop), MenuObject::Troop(second)];
    let window = scrap(&w.world, &team).unwrap().expect("the scrap asks");
    assert_eq!(window.order, ConfirmedOrder::Scrap);
    assert_eq!(
        window.scrap,
        [ScrapTarget::Troop(w.troop), ScrapTarget::Troop(second)]
    );
    assert_eq!(window.lines.len(), 2);
    // The Empire's regiment refuses its command: nothing is asked.
    let enemy = w.world.systems[w.origin]
        .ground_units
        .iter()
        .copied()
        .find(|&key| !w.world.troops[key].is_alliance)
        .unwrap();
    assert_eq!(
        scrap(
            &w.world,
            &[MenuObject::Troop(w.troop), MenuObject::Troop(enemy)]
        ),
        Some(Err("Scrap refused"))
    );
    // A fleet's ships go from the last, as each leaves the list.
    let (world, fleets, _) = world_with_two_fleets();
    let mut world = world;
    let class = world.fleets[fleets[0]].capital_ships[0].class;
    world.fleets[fleets[0]]
        .capital_ships
        .push(ShipInstance::new(class, 100, true));
    let ship = |index| MenuObject::Ship {
        fleet: fleets[0],
        index,
        roster: 0,
    };
    let ships = scrap(&world, &[ship(0), ship(1)]).unwrap().unwrap();
    assert_eq!(
        ships.scrap,
        [
            ScrapTarget::Ship {
                fleet: fleets[0],
                index: 1
            },
            ScrapTarget::Ship {
                fleet: fleets[0],
                index: 0
            }
        ]
    );
    // Another order is not this function's.
    assert_eq!(scrap_or(&world, ObjectMenuCommand::Move, &[ship(0)]), None);
}

#[test]
fn three_selected_fleet_squadrons_take_scrap_as_one_team() {
    // A0 2026-11-13-drag/16: three squadrons of a fleet selected, Scrap
    // white; FUN_005201c0 makes one command per member (FUN_005575f0).
    use rebellion_core::scrap::ScrapTarget;
    let mut w = defenses_world();
    let fleet = w.world.fleets.insert(Fleet {
        location: w.origin,
        capital_ships: Vec::new(),
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    w.world.systems[w.origin].fleets.push(fleet);
    rebellion_core::squadrons::add(&mut w.world.fleets[fleet].fighters, w.x_wing, 3);
    let squadron = MenuObject::Fighter { fleet, index: 0 };
    let gates = order_gates(
        Some(squadron),
        &w.world,
        &MissionState::new(),
        &MovementState::new(),
        &TroopTransportState::new(),
        &DeliveryState::new(),
        &ManufacturingState::new(),
        &rebellion_core::research::ResearchState::new(),
        MissionFaction::Alliance,
        0,
    );
    assert!(gates.scrap);
    let window = scrap_or(&w.world, ObjectMenuCommand::Scrap, &[squadron; 3])
        .unwrap()
        .expect("the scrap asks");
    assert_eq!(
        window.scrap,
        [ScrapTarget::FleetFighter {
            fleet,
            class: w.x_wing
        }; 3]
    );
    assert_eq!(window.lines.len(), 3);
}

fn scrap_or(
    world: &GameWorld,
    command: ObjectMenuCommand,
    team: &[MenuObject],
) -> Option<Result<MoveConfirmation, &'static str>> {
    team_confirmation(
        world,
        &MovementState::new(),
        &MissionState::new(),
        MissionFaction::Alliance,
        command,
        team,
    )
}

#[test]
fn a_team_retire_asks_once_for_every_member_and_one_refusal_refuses_it() {
    // FUN_00487740 validates the 0x242 for each member (FUN_004ed460,
    // FUN_00533ba0); FUN_0049a350 case 0x242 lists each name.
    let mut w = defenses_world();
    let team = [
        MenuObject::Character(w.agent),
        MenuObject::SpecialForce(w.force),
    ];
    let window = scrap_or(&w.world, ObjectMenuCommand::Retire, &team)
        .unwrap()
        .expect("the retire asks");
    assert_eq!(window.order, ConfirmedOrder::Retire);
    assert_eq!(window.characters, [w.agent]);
    assert_eq!(window.special_forces, [w.force]);
    assert_eq!(window.lines.len(), 2);
    w.world.characters[w.agent].on_mission = true;
    assert_eq!(
        scrap_or(&w.world, ObjectMenuCommand::Retire, &team),
        Some(Err("Retire refused"))
    );
    assert_eq!(
        scrap_or(
            &w.world,
            ObjectMenuCommand::Retire,
            &[MenuObject::Troop(w.troop)]
        ),
        Some(Err("Retire refused"))
    );
}

#[test]
fn a_team_menus_move_gate_takes_each_members_own_move_gate() {
    // FUN_0051d990: the 0x201's +0x18 with the whole team runs each
    // member's own command check; one object keeps its own gates.
    use rebellion_render::object_menu::OrderGates;
    let ship = MenuObject::Ship {
        fleet: FleetKey::default(),
        index: 0,
        roster: 0,
    };
    let kinds = [
        (
            ship,
            OrderGates {
                ship_move: true,
                ..OrderGates::default()
            },
        ),
        (
            MenuObject::Troop(Default::default()),
            OrderGates {
                troop_move: true,
                ..OrderGates::default()
            },
        ),
        (
            MenuObject::Fighter {
                fleet: FleetKey::default(),
                index: 0,
            },
            OrderGates {
                squadron_move: true,
                ..OrderGates::default()
            },
        ),
        (
            MenuObject::PlanetFighter {
                system: SystemKey::default(),
                class: Default::default(),
            },
            OrderGates {
                squadron_move: true,
                ..OrderGates::default()
            },
        ),
        (
            MenuObject::Character(Default::default()),
            OrderGates {
                character_move: true,
                ..OrderGates::default()
            },
        ),
        (
            MenuObject::SpecialForce(Default::default()),
            OrderGates {
                special_force_move: true,
                ..OrderGates::default()
            },
        ),
        (
            MenuObject::Fleet(FleetKey::default()),
            OrderGates {
                fleet_move: true,
                ..OrderGates::default()
            },
        ),
    ];
    for (object, own) in kinds {
        let team = team_gates(&[object, object], |_| own, None);
        assert!(team.troop_move && team.fleet_move, "{object:?}");
        let refused = team_gates(&[object, object], |_| OrderGates::default(), None);
        assert!(!refused.troop_move, "{object:?}");
    }
    let one = team_gates(&[ship], |_| kinds[0].1, None);
    assert!(one.ship_move && !one.troop_move);
}

#[test]
fn a_team_menus_orders_take_every_member() {
    // FUN_00487c50 builds the order with the selection as its team: a
    // Move splits as FUN_0053c4b0 does, Mission takes the members,
    // Create Fleet the ships of one fleet (manual p. 120).
    let mut w = defenses_world();
    let (world_fleets, fleets, _) = world_with_two_fleets();
    let _ = world_fleets;
    let fleet = w.world.fleets.insert(Fleet {
        location: w.origin,
        capital_ships: Vec::new(),
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    rebellion_core::squadrons::add(&mut w.world.fleets[fleet].fighters, w.x_wing, 1);
    let team = [
        MenuObject::Character(w.agent),
        MenuObject::SpecialForce(w.force),
        MenuObject::Troop(w.troop),
        MenuObject::PlanetFighter {
            system: w.origin,
            class: w.x_wing,
        },
        MenuObject::Fighter { fleet, index: 0 },
    ];
    let place = Some((w.origin, SquadronPlace::Planet(w.origin)));
    let Some(TeamMenuOrder::Target(order)) =
        team_menu_order(&w.world, ObjectMenuCommand::Move, &team, place)
    else {
        panic!("a team move");
    };
    assert_eq!(
        order,
        TargetOrder::TeamMove {
            system: w.origin,
            from: SquadronPlace::Planet(w.origin),
            team: rebellion_render::quadrant_icons::DefensesTeam {
                members: vec![
                    MissionMember::Character(w.agent),
                    MissionMember::SpecialForce(w.force)
                ],
                troops: vec![w.troop],
                squadrons: vec![w.x_wing, w.x_wing],
            },
            confirmed: false,
        }
    );
    let Some(TeamMenuOrder::Target(mission)) =
        team_menu_order(&w.world, ObjectMenuCommand::Mission, &team[..2], None)
    else {
        panic!("a mission");
    };
    assert_eq!(
        mission,
        TargetOrder::Mission(vec![
            MissionMember::Character(w.agent),
            MissionMember::SpecialForce(w.force)
        ])
    );
    let ship = |fleet, index| MenuObject::Ship {
        fleet,
        index,
        roster: 3,
    };
    assert!(matches!(
        team_menu_order(&w.world, ObjectMenuCommand::CreateFleet, &[ship(fleets[0], 0), ship(fleets[0], 2)], None),
        Some(TeamMenuOrder::Panel(PanelAction::CreateFleet { ref ships, .. })) if *ships == [0, 2]
    ));
    // Ships of two fleets make no one order.
    assert!(team_menu_order(
        &w.world,
        ObjectMenuCommand::CreateFleet,
        &[ship(fleets[0], 0), ship(fleets[1], 0)],
        None
    )
    .is_none());
}

#[test]
fn a_team_of_jailer_and_captive_takes_its_move_from_the_pair() {
    // FUN_0053cbc0 puts a team's characters and special forces in one
    // 0x241, whose FUN_0054bb90 lets a prisoner go with a free member
    // (manual p. 96): the pair's Move is the pair's check, not each
    // member's own (a lone prisoner's is grey).
    let mut world = GameWorld::default();
    let captive = world.characters.insert(Default::default());
    let jailer = world.characters.insert(Default::default());
    let team = [
        MenuObject::Character(captive),
        MenuObject::Character(jailer),
    ];
    let own = |object: MenuObject| OrderGates {
        character_move: object == MenuObject::Character(jailer),
        ..OrderGates::default()
    };
    let pair = |members: &[MissionMember]| members.contains(&MissionMember::Character(jailer));
    assert!(team_gates(&team, own, Some(&pair)).character_move);
    assert!(!team_gates(&team, own, None).character_move);
    let alone = [
        MenuObject::Character(captive),
        MenuObject::Character(captive),
    ];
    assert!(!team_gates(&alone, own, Some(&pair)).character_move);
    // A unit can be the jailer too (FUN_00533ce0).
    let unit = world
        .special_forces
        .insert(rebellion_core::world::SpecialForceUnit {
            class_dat_id: rebellion_core::ids::DatId::new(0x3c00_0003),
            is_alliance: false,
            skills: [0; 8],
            on_mission: false,
            on_hidden_mission: false,
        });
    let mixed = [
        MenuObject::Character(captive),
        MenuObject::SpecialForce(unit),
    ];
    let with_unit =
        |members: &[MissionMember]| members.contains(&MissionMember::SpecialForce(unit));
    let gates = team_gates(&mixed, |_| OrderGates::default(), Some(&with_unit));
    assert!(gates.character_move && gates.special_force_move);
}

#[test]
fn a_team_menus_gates_take_scrap_and_retire_from_every_member() {
    // FUN_0051d990: the order's +0x18 with the whole team.
    let gates = |scrap, retire| rebellion_render::object_menu::OrderGates {
        scrap,
        retire,
        ..Default::default()
    };
    let team = [
        MenuObject::Troop(Default::default()),
        MenuObject::Fleet(Default::default()),
    ];
    let both = team_gates(
        &team,
        |object| match object {
            MenuObject::Troop(_) => gates(true, true),
            _ => gates(true, false),
        },
        None,
    );
    assert!(both.scrap);
    assert!(!both.retire);
    let one = team_gates(
        &team,
        |object| match object {
            MenuObject::Troop(_) => gates(false, true),
            _ => gates(true, true),
        },
        None,
    );
    assert!(!one.scrap);
    assert!(one.retire);
}

#[test]
fn a_fleet_windows_ctrl_drop_of_a_fleet_or_ships_is_a_confirmed_move() {
    // FUN_00422ce0: GetAsyncKeyState(VK_CONTROL) makes the drop 0x202
    // for window type 4 as for type 10.
    let (world, fleets, _) = world_with_two_fleets();
    let point = egui_macroquad::egui::Pos2::ZERO;
    for confirmed in [false, true] {
        assert_eq!(
            fleet_drop_order(
                &world,
                FleetWindowAction::DragFleet {
                    fleets: vec![fleets[0]],
                    point,
                    confirmed,
                }
            ),
            Some((FleetDropOrder::Fleets(vec![fleets[0]]), point, confirmed))
        );
        assert_eq!(
            fleet_drop_order(
                &world,
                FleetWindowAction::DragShip {
                    fleet: fleets[1],
                    ships: vec![0],
                    roster: 9,
                    point,
                    confirmed,
                }
            ),
            Some((
                FleetDropOrder::Ships(ShipTeam {
                    fleet: fleets[1],
                    ships: vec![0],
                    roster: 9,
                }),
                point,
                confirmed
            ))
        );
        let members = vec![MissionMember::Character(
            rebellion_core::ids::CharacterKey::default(),
        )];
        assert_eq!(
            fleet_drop_order(
                &world,
                FleetWindowAction::DragMembers {
                    members: members.clone(),
                    point,
                    confirmed,
                }
            ),
            Some((
                FleetDropOrder::Team {
                    place: None,
                    team: rebellion_render::quadrant_icons::DefensesTeam {
                        members,
                        ..Default::default()
                    },
                },
                point,
                confirmed
            ))
        );
    }
}
