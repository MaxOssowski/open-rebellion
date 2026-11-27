use super::*;
use rebellion_core::dat::ExplorationStatus;

#[test]
fn a_defenses_team_with_any_one_kind_of_member_is_not_empty() {
    // FUN_0053c4b0 refuses only a team with no member at all (1/0x16).
    assert!(DefensesTeam::default().is_empty());
    let troops = DefensesTeam {
        troops: vec![rebellion_core::ids::TroopKey::default()],
        ..Default::default()
    };
    let squadrons = DefensesTeam {
        squadrons: vec![rebellion_core::ids::FighterKey::default()],
        ..Default::default()
    };
    let members = DefensesTeam {
        members: vec![MissionMember::Character(CharacterKey::default())],
        ..Default::default()
    };
    for team in [troops, squadrons, members] {
        assert!(!team.is_empty(), "{team:?}");
    }
}
use rebellion_core::dat::SectorGroup;
use rebellion_core::ids::{CharacterKey, DatId};
use rebellion_core::missions::{MissionFaction, MissionKind, MissionRequest};
use rebellion_core::world::{
    Character, ControlKind, DefenseFacilityInstance, ManufacturingFacilityInstance,
    ProductionFacilityInstance, Sector, SpecialForceUnit, System, TroopUnit,
};

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

fn alliance_world() -> (GameWorld, SystemKey) {
    world(ControlKind::Controlled(Faction::Alliance))
}

fn side(
    world: &GameWorld,
    missions: &MissionState,
    player: Faction,
    system: SystemKey,
    quadrant: Quadrant,
) -> Option<u8> {
    // The player sees the system (a live copy, unless the test gave it
    // a frozen one), so every side's objects count.
    let mut world = world.clone();
    let id = world.systems[system].dat_id;
    if world.object_views.system(player, id).is_none() {
        crate::viewed_contents::see_live(&mut world, player, system);
    }
    let fog = FogState::new(player);
    quadrant_side(
        &world,
        &fog,
        missions,
        &rebellion_core::movement::MovementState::default(),
        Travellers::default(),
        player,
        system,
        quadrant,
    )
}

fn alliance_side(world: &GameWorld, system: SystemKey, quadrant: Quadrant) -> Option<u8> {
    side(
        world,
        &MissionState::new(),
        Faction::Alliance,
        system,
        quadrant,
    )
}

fn add_mine(world: &mut GameWorld, system: SystemKey, is_alliance: bool) {
    let mine = world
        .production_facilities
        .insert(ProductionFacilityInstance {
            class_dat_id: DatId::new(0x2c00_0001),
            side: rebellion_core::dat::Faction::of_alliance(is_alliance),
            is_mine: true,
        });
    world.systems[system].production_facilities.push(mine);
}

fn add_character(world: &mut GameWorld, system: SystemKey, is_alliance: bool) -> CharacterKey {
    world.characters.insert(Character {
        dat_id: DatId::new(832),
        name: "Agent".into(),
        is_alliance,
        is_empire: !is_alliance,
        current_system: Some(system),
        recruited: true,
        ..Default::default()
    })
}

fn send_on_mission(missions: &mut MissionState, member: CharacterKey, system: SystemKey) {
    missions.dispatch(MissionRequest::single(
        MissionKind::Diplomacy,
        MissionFaction::Alliance,
        member,
        system,
        None,
        0,
    ));
}

#[test]
fn each_kind_has_its_own_art_for_each_side() {
    // FUN_0045ca80: kinds 4 and 8 also have art for sides 0 and 3; the
    // fleet and mission kinds have none.
    let table = [
        (
            Quadrant::System,
            [
                Some((10787, 10788)),
                Some((10771, 10772)),
                Some((10779, 10780)),
                Some((10787, 10788)),
            ],
        ),
        (
            Quadrant::Defenses,
            [
                Some((10789, 10790)),
                Some((10773, 10774)),
                Some((10781, 10782)),
                Some((10789, 10790)),
            ],
        ),
        (
            Quadrant::Fleets,
            [None, Some((10775, 10776)), Some((10783, 10784)), None],
        ),
        (
            Quadrant::Missions,
            [None, Some((10777, 10778)), Some((10785, 10786)), None],
        ),
    ];
    for (quadrant, art) in table {
        for (side, expected) in (0u8..).zip(art) {
            assert_eq!(
                quadrant_art(quadrant, side),
                expected,
                "{quadrant:?} side {side}"
            );
        }
        assert_eq!(quadrant_art(quadrant, 4), None);
    }
}

#[test]
fn an_empty_system_shows_no_icon() {
    // FUN_0045d140: a zero count removes the item.
    let (world, system) = alliance_world();
    for quadrant in Quadrant::ALL {
        assert_eq!(
            alliance_side(&world, system, quadrant),
            None,
            "{quadrant:?}"
        );
    }
}

#[test]
fn a_facility_icons_production_orders_need_a_yard_of_the_players_side() {
    // FUN_00512700 kind 4 (0x214, 0x216) gives the side's managers
    // (FUN_0052c170, FUN_00553350); FUN_00553770 refuses an empty team.
    let (mut world, system) = alliance_world();
    add_mine(&mut world, system, true);
    assert!(!facility_icon_has_managers(
        &world,
        system,
        Faction::Alliance
    ));
    let yard = world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id: DatId::new(0x2800_0001),
            side: Faction::Alliance,
            is_shipyard: true,
        });
    world.systems[system].manufacturing_facilities.push(yard);
    assert!(facility_icon_has_managers(
        &world,
        system,
        Faction::Alliance
    ));
    assert!(!facility_icon_has_managers(&world, system, Faction::Empire));
}

#[test]
fn the_system_icon_shows_the_systems_side_while_it_has_manufacturing_or_production() {
    // FUN_0045cdc0: FUN_0053b6e0 counts families 0x28..0x30; the state is
    // the system's side bits.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    add_mine(&mut world, system, false);
    assert_eq!(
        side(
            &world,
            &MissionState::new(),
            Faction::Empire,
            system,
            Quadrant::System
        ),
        Some(2)
    );

    let (mut world, system) = alliance_world();
    let yard = world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id: DatId::new(0x2800_0001),
            side: rebellion_core::dat::Faction::Alliance,
            is_shipyard: true,
        });
    world.systems[system].manufacturing_facilities.push(yard);
    assert_eq!(alliance_side(&world, system, Quadrant::System), Some(1));
}

#[test]
fn the_system_icon_takes_a_neutral_or_contested_systems_side() {
    // FUN_0045cdc0 passes the side bits 0 and 3 through; FUN_0045ca80
    // gives both the neutral art.
    for (control, expected) in [(ControlKind::Uncontrolled, 0), (ControlKind::Contested, 3)] {
        let (mut world, system) = world(control);
        add_mine(&mut world, system, true);
        assert_eq!(
            alliance_side(&world, system, Quadrant::System),
            Some(expected)
        );
    }
}

#[test]
fn each_kind_in_a_frozen_copy_lights_the_defenses_icon_alone() {
    // FUN_0045ce80's sum over the copy (FUN_004f2d10, FUN_004f9a60):
    // squadrons, special forces, characters and regiment copies each
    // light the icon on their own; the Alliance's own do not count as
    // the other side's.
    use rebellion_core::object_views::{apply, ObjectView, RegimentView, ViewPlace, ViewUpdate};
    let frozen = |world: &mut GameWorld, system: SystemKey| {
        let copy =
            rebellion_core::object_views::SystemView::of(world, &world.systems[system], false);
        let id = world.systems[system].dat_id;
        world.object_views.alliance_systems.insert(id, copy);
    };
    let empty = |world: &mut GameWorld, system: SystemKey| {
        world.systems[system].fighters.clear();
        world.systems[system].special_forces.clear();
        for character in world.characters.values_mut() {
            character.current_system = None;
        }
    };
    let imperial = || world(ControlKind::Controlled(Faction::Empire));

    let (mut world, system) = imperial();
    let tie = world.fighter_classes.insert(rebellion_core::world::FighterClass {
        dat_id: DatId::new(0x1c00_0005),
        is_alliance: false,
        ..Default::default()
    });
    world.systems[system].fighters.push(rebellion_core::world::FighterEntry {
        class: tie,
        count: 1,
        carrier: None,
        damage: None,
    });
    frozen(&mut world, system);
    empty(&mut world, system);
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), Some(2));
    world.fighter_classes[tie].is_alliance = true;
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), None);

    let (mut world, system) = imperial();
    let force = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: false,
        skills: [0; 8],
        on_mission: false,
    });
    world.systems[system].special_forces.push(force);
    frozen(&mut world, system);
    empty(&mut world, system);
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), Some(2));

    for is_alliance in [false, true] {
        let (mut world, system) = imperial();
        let key = add_character(&mut world, system, is_alliance);
        frozen(&mut world, system);
        empty(&mut world, system);
        assert_eq!(
            alliance_side(&world, system, Quadrant::Defenses),
            (!is_alliance).then_some(2),
            "{is_alliance}"
        );
        world.characters[key].current_system = Some(system);
    }
    // The Empire's own character in its copy of an Alliance system is
    // not the other side's.
    let (mut world, system) = self::world(ControlKind::Controlled(Faction::Alliance));
    add_character(&mut world, system, false);
    let copy = rebellion_core::object_views::SystemView::of(&world, &world.systems[system], false);
    let id = world.systems[system].dat_id;
    world.object_views.empire_systems.insert(id, copy);
    empty(&mut world, system);
    let mut fog = FogState::new(Faction::Empire);
    fog.reveal(system);
    let empire = |world: &GameWorld| {
        quadrant_side(
            world,
            &fog,
            &MissionState::new(),
            &rebellion_core::movement::MovementState::default(),
            Travellers::default(),
            Faction::Empire,
            system,
            Quadrant::Defenses,
        )
    };
    assert_eq!(empire(&world), None);

    let (mut world, system) = imperial();
    let id = world.systems[system].dat_id;
    frozen(&mut world, system);
    apply(
        &mut world,
        &[ViewUpdate::Set {
            side: Faction::Alliance,
            id: DatId::new(0x1000_0003),
            view: ObjectView::Regiment(RegimentView {
                class: DatId::new(0x1000_0001),
                is_alliance: false,
                strength: 1,
                place: ViewPlace::Surface(id),
                arrival: None,
                live: false,
            }),
        }],
    );
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), Some(2));
}

#[test]
fn a_frozen_copys_other_side_mission_lights_the_missions_icon_as_seen() {
    // FUN_004a1f60 over the player's copy (FUN_004f2d10, FUN_004f9a60):
    // the Empire's mission aimed at the system wins the icon as seen,
    // its agent gone from the player's sight, even after it has ended
    // (GameObj_SetReadyForDelete FUN_004f82d0 keeps the copy).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    let agent = add_character(&mut world, system, false);
    let mut missions = MissionState::new();
    missions.dispatch(rebellion_core::missions::MissionRequest::single(
        rebellion_core::missions::MissionKind::Diplomacy,
        rebellion_core::missions::MissionFaction::Empire,
        agent,
        system,
        None,
        0,
    ));
    let movement = rebellion_core::movement::MovementState::default();
    let copy = rebellion_core::object_views::SystemView::of(&world, &world.systems[system], false);
    let extras =
        rebellion_core::object_views::SystemExtras::of(&world, &movement, &missions, system, false);
    let id = world.systems[system].dat_id;
    world.object_views.alliance_systems.insert(id, copy);
    world.object_views.alliance_extras.insert(id, extras);
    world.characters[agent].current_system = None;
    let side = |world: &GameWorld, missions: &MissionState| {
        missions_side(world, &FogState::new(Faction::Alliance), missions, Faction::Alliance, system)
    };
    assert_eq!(side(&world, &missions), 2);
    assert_eq!(side(&world, &MissionState::new()), 2);
    world.object_views.alliance_extras.clear();
    assert_eq!(side(&world, &missions), 3);
}

#[test]
fn a_frozen_copy_lights_the_icons_as_it_was_seen() {
    // The galaxy view reads the player's side's copy (FUN_004f2d10);
    // FUN_004f9a60 froze it with an Imperial mine and shield, so both
    // icons keep the Empire's art after the live ones are gone, and its
    // side bits stay the Empire's.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    add_mine(&mut world, system, false);
    let shield = world.defense_facilities.insert(DefenseFacilityInstance {
        class_dat_id: DatId::new(0x2400_0003),
        side: rebellion_core::dat::Faction::Empire,
    });
    world.systems[system].defense_facilities.push(shield);
    let copy = rebellion_core::object_views::SystemView::of(&world, &world.systems[system], false);
    world.object_views.alliance_systems.insert(world.systems[system].dat_id, copy);
    world.systems[system].production_facilities.clear();
    world.systems[system].defense_facilities.clear();
    world.systems[system].control = ControlKind::Uncontrolled;
    assert_eq!(alliance_side(&world, system, Quadrant::System), Some(2));
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), Some(2));
    // Without the copy, the fog shows the live system: nothing.
    world.object_views.alliance_systems.clear();
    assert_eq!(alliance_side(&world, system, Quadrant::System), None);
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), None);
}

#[test]
fn the_headquarters_lights_neither_the_system_nor_the_defenses_icon() {
    // FUN_0045cdc0 counts FUN_0053b6e0's families 0x28..0x30 and
    // FUN_0045ce80 FUN_00526fd0's 0x22..0x28; the HQ is family 0x20.
    let (mut world, system) = alliance_world();
    let hq = world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id: DatId::new(0x2000_0001),
            side: rebellion_core::dat::Faction::Alliance,
            is_shipyard: false,
        });
    world.systems[system].manufacturing_facilities.push(hq);
    assert_eq!(alliance_side(&world, system, Quadrant::System), None);
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), None);
}

#[test]
fn defenses_do_not_light_the_system_icon() {
    // FUN_0053b6e0's range 0x28..0x30 excludes the defense families.
    let (mut world, system) = alliance_world();
    let shield = world.defense_facilities.insert(DefenseFacilityInstance {
        class_dat_id: DatId::new(0x2400_0003),
        side: rebellion_core::dat::Faction::Alliance,
    });
    world.systems[system].defense_facilities.push(shield);
    assert_eq!(alliance_side(&world, system, Quadrant::System), None);
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), Some(1));
}

#[test]
fn a_regiment_or_a_special_force_lights_the_defenses_icon() {
    // FUN_0045ce80: FUN_00504c40 counts regiments, FUN_00536da0 the
    // families 0x30..0x40 that hold special forces.
    let (mut world, system) = alliance_world();
    let regiment = world.troops.insert(TroopUnit {
        class_dat_id: DatId::new(0x1000_0001),
        is_alliance: true,
        regiment_strength: 100,
    });
    world.systems[system].ground_units.push(regiment);
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), Some(1));

    let (mut world, system) = alliance_world();
    let force = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
    });
    world.systems[system].special_forces.push(force);
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), Some(1));
    assert_eq!(alliance_side(&world, system, Quadrant::System), None);
}

#[test]
fn a_regiment_or_squadron_on_its_way_lights_its_destinations_defenses_icon() {
    // FUN_0045ce80 counts the system's regiments (FUN_00504c40) and
    // squadrons (FUN_005039d0); FUN_00556390 puts one sent by Move in
    // its destination at once (A0 2026-11-11-regiment-en-route/00).
    let (mut world, system) = alliance_world();
    let other = world.systems.insert(world.systems[system].clone());
    let regiment = world.troops.insert(TroopUnit {
        class_dat_id: DatId::new(0x1000_0001),
        is_alliance: true,
        regiment_strength: 100,
    });
    let xwing = world
        .fighter_classes
        .insert(rebellion_core::world::FighterClass {
            is_alliance: true,
            ..Default::default()
        });
    let regiments = |to| {
        [RegimentTransit {
            troop: regiment,
            origin: other,
            leg: RegimentLeg::Surface(to),
            arrival_tick: 9,
        }]
    };
    let squadrons = |to| {
        [SquadronTransit {
            class: xwing,
            count: 1,
            origin: other,
            to: SquadronPlace::Planet(to),
            arrival_tick: 9,
            lost: Vec::new(),
        }]
    };
    let defenses = |travellers| {
        quadrant_side(
            &world,
            &FogState::new(Faction::Alliance),
            &MissionState::new(),
            &rebellion_core::movement::MovementState::default(),
            travellers,
            Faction::Alliance,
            system,
            Quadrant::Defenses,
        )
    };
    assert_eq!(defenses(Travellers::default()), None);
    let to_here = regiments(system);
    let elsewhere = regiments(other);
    assert_eq!(
        defenses(Travellers {
            regiments: &to_here,
            squadrons: &[],
        }),
        Some(1)
    );
    assert_eq!(
        defenses(Travellers {
            regiments: &elsewhere,
            squadrons: &[],
        }),
        None
    );
    let to_here = squadrons(system);
    let elsewhere = squadrons(other);
    assert_eq!(
        defenses(Travellers {
            regiments: &[],
            squadrons: &to_here,
        }),
        Some(1)
    );
    assert_eq!(
        defenses(Travellers {
            regiments: &[],
            squadrons: &elsewhere,
        }),
        None
    );
}

#[test]
fn a_member_on_its_way_aboard_a_fleet_counts_for_the_fleet_not_the_system() {
    // FUN_00556390 puts a Move's member in the target container +0x74,
    // here a fleet, and FUN_0045ce80's walks (FUN_00513050) never
    // descend into it: the defenses icon and its Move team leave the
    // member out; one bound for the system counts.
    let (mut world, system) = alliance_world();
    let rider = add_character(&mut world, system, true);
    world.characters[rider].current_system = None;
    let mut missions = MissionState::new();
    let id = missions.dispatch(MissionRequest::single(
        MissionKind::Diplomacy,
        MissionFaction::Alliance,
        rider,
        system,
        None,
        0,
    ));
    missions.push_transit(rebellion_core::missions::MemberTransit {
        member: MissionMember::Character(rider),
        mission_id: id,
        to: system,
        arrival: 9,
        abode: None,
    });
    // The dispatched mission's team holds the rider; a hidden Move keeps
    // the rider idle (FUN_0045ce80's +0x78 bit 8 test).
    world.characters[rider].on_hidden_mission = true;
    let team = |missions: &MissionState| {
        defenses_move_team(&world, missions, Faction::Alliance, system).members
    };
    assert_eq!(
        side(&world, &missions, Faction::Alliance, system, Quadrant::Defenses),
        Some(1)
    );
    assert_eq!(team(&missions), [MissionMember::Character(rider)]);
    assert!(missions.set_target_fleet(id, rebellion_core::ids::FleetKey::default()));
    assert_eq!(
        side(&world, &missions, Faction::Alliance, system, Quadrant::Defenses),
        None
    );
    assert!(team(&missions).is_empty());
}

#[test]
fn a_squadron_on_the_planet_lights_the_defenses_icon() {
    // FUN_0045ce80: FUN_005039d0 counts the squadrons on the planet.
    let (mut world, system) = alliance_world();
    let x_wing = world
        .fighter_classes
        .insert(rebellion_core::world::FighterClass {
            is_alliance: true,
            ..Default::default()
        });
    rebellion_core::squadrons::add(&mut world.systems[system].fighters, x_wing, 1);
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), Some(1));
    assert_eq!(alliance_side(&world, system, Quadrant::System), None);
}

#[test]
fn a_defenses_icons_move_team_is_its_sides_regiments_squadrons_and_personnel() {
    // FUN_00512700 kind 8 for 0x201/0x202: FUN_00504c40 (regiments),
    // FUN_005039d0 (squadrons), FUN_00536da0 (characters and special
    // forces), each through FUN_00553350's side filter. Defense
    // facilities are Scrap's only (FUN_00527050).
    let (mut world, system) = alliance_world();
    let troop = |world: &mut GameWorld, is_alliance| {
        let key = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(key);
        key
    };
    let ours = troop(&mut world, true);
    troop(&mut world, false);
    let class = |world: &mut GameWorld, is_alliance| {
        world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                is_alliance,
                ..Default::default()
            })
    };
    let x_wing = class(&mut world, true);
    let tie = class(&mut world, false);
    rebellion_core::squadrons::add(&mut world.systems[system].fighters, x_wing, 2);
    rebellion_core::squadrons::add(&mut world.systems[system].fighters, tie, 1);
    let agent = add_character(&mut world, system, true);
    add_character(&mut world, system, false);
    let force = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
    });
    world.systems[system].special_forces.push(force);
    let facility = world.defense_facilities.insert(DefenseFacilityInstance {
        class_dat_id: DatId::new(0x2200_0001),
        side: Faction::Alliance,
    });
    world.systems[system].defense_facilities.push(facility);

    let team = defenses_move_team(&world, &MissionState::new(), Faction::Alliance, system);
    assert_eq!(team.troops, [ours]);
    assert_eq!(team.squadrons, [x_wing, x_wing]);
    assert_eq!(
        team.members,
        [
            MissionMember::Character(agent),
            MissionMember::SpecialForce(force)
        ]
    );
    assert!(!team.is_empty());
    let (empty, elsewhere) = alliance_world();
    assert!(
        defenses_move_team(&empty, &MissionState::new(), Faction::Alliance, elsewhere)
            .is_empty()
    );
}

#[test]
fn personnel_light_the_defenses_icon_only_off_a_visible_mission() {
    // FUN_0045ce80 counts a member when FUN_004ece60 finds no mission key
    // (+0x68) or role +0x78 bit 8 (OnHiddenMission) is set.
    let (mut world, system) = alliance_world();
    let agent = add_character(&mut world, system, true);
    let player = Faction::Alliance;
    assert_eq!(
        side(
            &world,
            &MissionState::new(),
            player,
            system,
            Quadrant::Defenses
        ),
        Some(1)
    );

    let mut missions = MissionState::new();
    send_on_mission(&mut missions, agent, system);
    assert_eq!(
        side(&world, &missions, player, system, Quadrant::Defenses),
        None
    );

    world.characters[agent].on_hidden_mission = true;
    assert_eq!(
        side(&world, &missions, player, system, Quadrant::Defenses),
        Some(1)
    );
}

#[test]
fn the_missions_icon_shows_the_players_side_for_its_own_visible_mission() {
    // FUN_0045d090 / FUN_004a1f60: the player's set only, so its side.
    let (mut world, system) = alliance_world();
    let agent = add_character(&mut world, system, true);
    let mut missions = MissionState::new();
    let player = Faction::Alliance;
    assert_eq!(
        side(&world, &missions, player, system, Quadrant::Missions),
        None
    );

    send_on_mission(&mut missions, agent, system);
    assert_eq!(
        side(&world, &missions, player, system, Quadrant::Missions),
        Some(1)
    );
    // An Empire player sees the same mission as the other side's.
    assert_eq!(
        side(
            &world,
            &missions,
            Faction::Empire,
            system,
            Quadrant::Missions
        ),
        Some(1)
    );
}

#[test]
fn a_mission_whose_team_is_on_its_way_lights_its_targets_icon() {
    // FUN_00556430 then FUN_00556390 put a travelling member in the
    // target's container at once (mission-lifecycle.md), so
    // FUN_004a1f60's and FUN_00553350's walks of the target find it.
    let (mut world, system) = alliance_world();
    let agent = add_character(&mut world, system, true);
    let mut missions = MissionState::new();
    // A mission of an agent elsewhere takes id 0.
    let away = add_character(&mut world, system, true);
    world.characters[away].current_system = None;
    send_on_mission(&mut missions, away, system);
    send_on_mission(&mut missions, agent, system);
    world.characters[agent].current_system = None;
    let player = Faction::Alliance;
    assert_eq!(side(&world, &missions, player, system, Quadrant::Missions), None);

    missions.push_transit(rebellion_core::missions::MemberTransit {
        member: MissionMember::Character(agent),
        mission_id: 1,
        to: system,
        arrival: 9,
        abode: None,
    });
    assert_eq!(side(&world, &missions, player, system, Quadrant::Missions), Some(1));
    let fog = FogState::new(player);
    assert_eq!(own_visible_missions(&world, &fog, &missions, player, system), [1]);
    // On its way, an agent off any mission is idle personnel there.
    let walker = add_character(&mut world, system, true);
    world.characters[walker].current_system = None;
    missions.push_transit(rebellion_core::missions::MemberTransit {
        member: MissionMember::Character(walker),
        mission_id: 0,
        to: system,
        arrival: 9,
        abode: None,
    });
    assert_eq!(side(&world, &missions, player, system, Quadrant::Defenses), Some(1));
}

#[test]
fn a_recon_team_lights_the_missions_icon_of_the_unexplored_system_it_goes_to() {
    // User report 2026-11-27 from the original: a Recon mission's icon
    // shows at its target, which is unexplored. The team is in the
    // target's container while it travels (FUN_00556430, FUN_00556390);
    // only the Missions icon lights, with the player's own side.
    let (mut world, system) = world(ControlKind::Uncontrolled);
    world.systems[system].exploration_status = ExplorationStatus::Unexplored;
    let agent = add_character(&mut world, system, true);
    world.characters[agent].current_system = None;
    let mut missions = MissionState::new();
    let player = Faction::Alliance;
    assert!(!rebellion_core::object_views::side_explored(
        &world, player, system
    ));
    assert_eq!(
        side(&world, &missions, player, system, Quadrant::Missions),
        None
    );

    missions.dispatch(MissionRequest::single(
        MissionKind::Reconnaissance,
        MissionFaction::Alliance,
        agent,
        system,
        None,
        0,
    ));
    missions.push_transit(rebellion_core::missions::MemberTransit {
        member: MissionMember::Character(agent),
        mission_id: 0,
        to: system,
        arrival: 9,
        abode: None,
    });
    assert_eq!(
        side(&world, &missions, player, system, Quadrant::Missions),
        Some(1)
    );
    let fog = FogState::new(player);
    assert_eq!(missions_side(&world, &fog, &missions, player, system), 1);
    for quadrant in [Quadrant::System, Quadrant::Defenses, Quadrant::Fleets] {
        assert_eq!(side(&world, &missions, player, system, quadrant), None);
    }
    // The other side sees nothing there.
    assert_eq!(
        side(
            &world,
            &missions,
            Faction::Empire,
            system,
            Quadrant::Missions
        ),
        None
    );
}

#[test]
fn a_member_moving_away_counts_at_its_target_and_not_where_it_left() {
    // FUN_00556390 reparents a moving member into the target at once.
    // A0 2026-11-14-quadrants-status-known/02: Adar Tallon, Averam's
    // only member, Moved to Yavin; Averam's Defenses icon goes at once.
    // /04: Wedge Antilles Moved from Yavin lights Averam's icon again
    // while he travels (/07 lists him "Averam ( Enroute )").
    let (mut world, source) = alliance_world();
    let target = world.systems.insert(world.systems[source].clone());
    let walker = add_character(&mut world, source, true);
    let player = Faction::Alliance;
    let mut missions = MissionState::new();
    assert_eq!(side(&world, &missions, player, source, Quadrant::Defenses), Some(1));
    assert_eq!(side(&world, &missions, player, target, Quadrant::Defenses), None);

    // Phase 4 (MissionState::depart) takes it off the map into a transit.
    world.characters[walker].current_system = None;
    missions.push_transit(rebellion_core::missions::MemberTransit {
        member: MissionMember::Character(walker),
        mission_id: 0,
        to: target,
        arrival: 9,
        abode: None,
    });
    assert_eq!(side(&world, &missions, player, source, Quadrant::Defenses), None);
    assert_eq!(side(&world, &missions, player, target, Quadrant::Defenses), Some(1));
}

#[test]
fn an_icons_abort_names_the_players_visible_missions_there() {
    // FUN_00512700 kind 0x40 and FUN_00553350: the player's members on
    // a visible mission; event 0x261 takes each one's mission.
    let (mut world, system) = alliance_world();
    let ours = add_character(&mut world, system, true);
    let hidden = add_character(&mut world, system, true);
    let theirs = add_character(&mut world, system, false);
    let mut missions = MissionState::new();
    send_on_mission(&mut missions, ours, system);
    send_on_mission(&mut missions, hidden, system);
    send_on_mission(&mut missions, theirs, system);
    world.characters[hidden].on_hidden_mission = true;
    let fog = FogState::new(Faction::Alliance);
    let ids = own_visible_missions(&world, &fog, &missions, Faction::Alliance, system);
    assert_eq!(ids.len(), 1);
    let first = missions.missions().front().map(|m| m.id);
    assert_eq!(ids.first().copied(), first);
}

#[test]
fn the_other_sides_missions_take_the_missions_icon() {
    // FUN_004a1f60 returns the other side whenever its set is non-empty.
    let (mut world, system) = alliance_world();
    let ours = add_character(&mut world, system, true);
    let theirs = add_character(&mut world, system, false);
    let mut missions = MissionState::new();
    send_on_mission(&mut missions, ours, system);
    send_on_mission(&mut missions, theirs, system);
    assert_eq!(
        side(
            &world,
            &missions,
            Faction::Alliance,
            system,
            Quadrant::Missions
        ),
        Some(2)
    );
    assert_eq!(
        side(
            &world,
            &missions,
            Faction::Empire,
            system,
            Quadrant::Missions
        ),
        Some(1)
    );
}

#[test]
fn a_hidden_mission_does_not_light_the_missions_icon() {
    // FUN_004a1f60 skips members with role +0x78 bit 8.
    let (mut world, system) = alliance_world();
    let agent = add_character(&mut world, system, true);
    world.characters[agent].on_hidden_mission = true;
    let mut missions = MissionState::new();
    send_on_mission(&mut missions, agent, system);
    assert_eq!(
        side(
            &world,
            &missions,
            Faction::Alliance,
            system,
            Quadrant::Missions
        ),
        None
    );
}

#[test]
fn members_elsewhere_or_killed_do_not_count() {
    // FUN_00536da0 walks only the system's own members.
    let (mut world, system) = alliance_world();
    let away = add_character(&mut world, system, true);
    world.characters[away].current_system = None;
    let dead = add_character(&mut world, system, true);
    world.characters[dead].is_killed = true;
    assert_eq!(alliance_side(&world, system, Quadrant::Defenses), None);
}

#[test]
fn hidden_enemy_contents_light_no_icon_until_the_player_sees_the_system() {
    // port: the System window's visibility (opposing_contents_visible).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    add_mine(&mut world, system, false);
    let missions = MissionState::new();
    let fog = FogState::new(Faction::Alliance);
    assert_eq!(
        quadrant_side(
            &world,
            &fog,
            &missions,
            &rebellion_core::movement::MovementState::default(),
            Travellers::default(),
            Faction::Alliance,
            system,
            Quadrant::System
        ),
        None
    );
    crate::viewed_contents::see_live(&mut world, Faction::Alliance, system);
    assert_eq!(
        quadrant_side(
            &world,
            &fog,
            &missions,
            &rebellion_core::movement::MovementState::default(),
            Travellers::default(),
            Faction::Alliance,
            system,
            Quadrant::System
        ),
        Some(2)
    );
    // The player's own objects show at an enemy system it cannot see.
    add_mine(&mut world, system, true);
    let blind = FogState::new(Faction::Alliance);
    assert_eq!(
        quadrant_side(
            &world,
            &blind,
            &missions,
            &rebellion_core::movement::MovementState::default(),
            Travellers::default(),
            Faction::Alliance,
            system,
            Quadrant::System
        ),
        Some(2)
    );
}

#[test]
fn an_unexplored_system_shows_no_icon() {
    // port: as the System window, an unexplored system lists nothing.
    // The Empire's copy of the Alliance's system is unexplored (a
    // bare setup copy, ghidra/notes/fog-of-war.md).
    let (mut world, system) = alliance_world();
    add_mine(&mut world, system, true);
    crate::viewed_contents::see_unexplored(&mut world, Faction::Empire, system);
    assert_eq!(
        side(
            &world,
            &MissionState::new(),
            Faction::Empire,
            system,
            Quadrant::System
        ),
        None
    );
}

#[test]
fn quadrant_icon_pairs_the_side_with_its_art() {
    let (mut world, system) = alliance_world();
    add_mine(&mut world, system, true);
    let fog = FogState::new(Faction::Alliance);
    let missions = MissionState::new();
    assert_eq!(
        quadrant_icon(
            &world,
            &fog,
            &missions,
            &rebellion_core::movement::MovementState::default(),
            Travellers::default(),
            Faction::Alliance,
            system,
            Quadrant::System
        ),
        Some((10771, 10772))
    );
    assert_eq!(
        quadrant_icon(
            &world,
            &fog,
            &missions,
            &rebellion_core::movement::MovementState::default(),
            Travellers::default(),
            Faction::Alliance,
            system,
            Quadrant::Defenses
        ),
        None
    );
}
