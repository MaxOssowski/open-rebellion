use super::*;
use rebellion_core::dat::ExplorationStatus;

#[test]
fn each_tab_has_its_tooltip_string() {
    // FUN_004a8790 L182..L240.
    assert_eq!(
        super::DefensesPage::ALL.map(super::DefensesPage::tooltip_string),
        [0x1744, 0x1743, 0x1742, 0x1741, 0x1740]
    );
}
use crate::cockpit::CockpitViewport;
use crate::fleet_window::tests::PAINTED;
use rebellion_core::dat::SectorGroup;
use rebellion_core::economy::SystemEconomy;
use rebellion_core::ids::DatId;
use rebellion_core::missions::{MissionFaction, MissionKind, MissionRequest};
use rebellion_core::world::{
    Character, ControlKind, DefenseFacilityInstance, Sector, SpecialForceUnit, System,
    TroopUnit,
};

const KDY: u32 = 0x2200_0001;
const LNR: u32 = 0x2300_0002;
const GENCORE: u32 = 0x2400_0003;
const DEATH_STAR_SHIELD: u32 = 0x2500_0004;
const ALLIANCE_ARMY: u32 = 0x1000_0002;
const STORMTROOPERS: u32 = 0x1000_0006;

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

fn add_defense(world: &mut GameWorld, system: SystemKey, dat_id: u32, is_alliance: bool) {
    let key = world.defense_facilities.insert(DefenseFacilityInstance {
        class_dat_id: DatId::new(dat_id),
        side: rebellion_core::dat::Faction::of_alliance(is_alliance),
    });
    world.systems[system].defense_facilities.push(key);
}

fn add_troop(world: &mut GameWorld, system: SystemKey, dat_id: u32, is_alliance: bool) {
    let key = world.troops.insert(TroopUnit {
        class_dat_id: DatId::new(dat_id),
        is_alliance,
        regiment_strength: 100,
    });
    world.systems[system].ground_units.push(key);
}

fn add_character(
    world: &mut GameWorld,
    system: SystemKey,
    name: &str,
    is_alliance: bool,
) -> CharacterKey {
    world.characters.insert(Character {
        dat_id: DatId::new(832),
        name: name.into(),
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

fn seen(system: SystemKey) -> FogState {
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(system);
    fog
}

/// No unit queued or travelling anywhere.
fn idle() -> DefensesProduction<'static> {
    DefensesProduction {
        manufacturing: Box::leak(Box::default()),
        deliveries: Box::leak(Box::default()),
        regiments: &[],
        squadrons: &[],
    }
}

fn labels(
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    system: SystemKey,
    page: DefensesPage,
) -> Vec<String> {
    page_rows(
        world,
        fog,
        missions,
        idle(),
        Faction::Alliance,
        system,
        page,
    )
    .into_iter()
    .map(|row| row.label)
    .collect()
}

/// An Alliance system holding one of everything, beside the Empire's
/// equivalents, which the window never lists there.
fn stocked() -> (GameWorld, SystemKey, MissionState) {
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    for dat_id in [KDY, GENCORE, LNR, DEATH_STAR_SHIELD] {
        add_defense(&mut world, system, dat_id, true);
        add_defense(&mut world, system, dat_id, false);
    }
    add_troop(&mut world, system, ALLIANCE_ARMY, true);
    add_troop(&mut world, system, STORMTROOPERS, false);
    add_character(&mut world, system, "Idle", true);
    let away = add_character(&mut world, system, "Away", true);
    let hidden = add_character(&mut world, system, "Hidden", true);
    world.characters[hidden].on_hidden_mission = true;
    add_character(&mut world, system, "Enemy", false);
    let force = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
        on_hidden_mission: false,
    });
    world.systems[system].special_forces.push(force);
    for (raw, name, alliance, count) in [
        (0x1c00_0001, "X-wing", true, 2),
        (0x1c00_0005, "TIE Fighter", false, 1),
    ] {
        let class = world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                dat_id: DatId::new(raw),
                name: name.into(),
                is_alliance: alliance,
                is_empire: !alliance,
                ..Default::default()
            });
        rebellion_core::squadrons::add(&mut world.systems[system].fighters, class, count);
    }
    let mut missions = MissionState::new();
    send_on_mission(&mut missions, away, system);
    send_on_mission(&mut missions, hidden, system);
    (world, system, missions)
}

#[test]
fn each_page_lists_the_holding_sides_objects_of_its_families_in_walk_order() {
    // FUN_004a90d0: pages 5 and 4 keep families 0x22/0x23 and 0x24/0x25
    // of the FUN_00527050 walk; page 2 is FUN_00504cc0; page 1 keeps
    // members with no mission key or on a hidden mission (+0x78 bit 8);
    // page 3 is FUN_005039d0's squadrons on the planet; all filtered to
    // the system's side (+0x148).
    let (world, system, missions) = stocked();
    let fog = seen(system);
    let page = |page| labels(&world, &fog, &missions, system, page);

    assert_eq!(page(DefensesPage::Batteries), ["KDY-150", "LNR Series I"]);
    assert_eq!(
        page(DefensesPage::Shields),
        ["GenCore Level I", "Death Star Shield"]
    );
    assert_eq!(page(DefensesPage::Regiments), ["Alliance Army Regiment"]);
    assert_eq!(page(DefensesPage::Squadrons), ["X-wing", "X-wing"]);
    assert_eq!(
        page(DefensesPage::Personnel),
        ["Idle", "Hidden", "Guerrillas"]
    );
}

#[test]
fn members_on_their_way_are_listed_at_their_destination_characters_first() {
    // A travelling member's container is its destination; its Move is a
    // hidden mission (MISSNSD +0x5c). A0 specforces/03: Mon Calamari's
    // page lists the travelling Garm Bel Iblis and Adar Tallon among its
    // characters and the travelling Longprobe team after them.
    let (mut world, system, mut missions) = stocked();
    let walker = add_character(&mut world, system, "Walker", true);
    world.characters[walker].current_system = None;
    let force = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0003),
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
        on_hidden_mission: false,
    });
    let elsewhere = add_character(&mut world, system, "Elsewhere", true);
    world.characters[elsewhere].current_system = None;
    let other = world.systems.insert(world.systems[system].clone());
    for (member, to) in [
        (MissionMember::Character(walker), system),
        (MissionMember::SpecialForce(force), system),
        (MissionMember::Character(elsewhere), other),
    ] {
        missions.push_transit(rebellion_core::missions::MemberTransit {
            member,
            mission_id: 0,
            to,
            arrival: 9,
            abode: None,
        });
    }
    let fog = seen(system);
    assert_eq!(
        labels(&world, &fog, &missions, system, DefensesPage::Personnel),
        ["Idle", "Hidden", "Walker", "Guerrillas", "Longprobe Y-wing Recon Team"]
    );
    // An Empire system lists its own traveller (side 2).
    let (mut imperial, held) = self::world(ControlKind::Controlled(Faction::Empire));
    let trooper = add_character(&mut imperial, held, "Piett", false);
    imperial.characters[trooper].current_system = None;
    let mut imperial_missions = MissionState::new();
    imperial_missions.push_transit(rebellion_core::missions::MemberTransit {
        member: MissionMember::Character(trooper),
        mission_id: 0,
        to: held,
        arrival: 9,
        abode: None,
    });
    let rows = page_rows(
        &imperial,
        &seen(held),
        &imperial_missions,
        idle(),
        Faction::Empire,
        held,
        DefensesPage::Personnel,
    );
    assert_eq!(
        rows.into_iter().map(|row| row.label).collect::<Vec<_>>(),
        ["Piett"]
    );
}

#[test]
fn a_member_on_its_way_aboard_a_fleet_is_the_fleets_not_the_planets() {
    // A Move onto a fleet lands its member in the fleet, the Move's
    // target container (+0x74, FUN_00556390), at once; the page walks
    // the system's own children only (FUN_00513050 never descends), so
    // the member is the Fleet window's (quadrant_icons::fleet_arrivals).
    let (mut world, system, mut missions) = stocked();
    let rider = add_character(&mut world, system, "Rider", true);
    world.characters[rider].current_system = None;
    let fleet = rebellion_core::ids::FleetKey::default();
    let id = missions.dispatch(MissionRequest::single(
        MissionKind::Diplomacy,
        MissionFaction::Alliance,
        rider,
        system,
        None,
        0,
    ));
    assert!(missions.set_target_fleet(id, fleet));
    missions.push_transit(rebellion_core::missions::MemberTransit {
        member: MissionMember::Character(rider),
        mission_id: id,
        to: system,
        arrival: 9,
        abode: None,
    });
    let fog = seen(system);
    assert!(
        !labels(&world, &fog, &missions, system, DefensesPage::Personnel)
            .contains(&"Rider".to_owned())
    );
    assert_eq!(
        crate::quadrant_icons::fleet_arrivals(&missions),
        [(MissionMember::Character(rider), fleet)]
    );
}

#[test]
fn members_on_their_way_take_the_en_route_mark_over_their_backdrop() {
    // A0 specforces/03: Mon Calamari's travelling Garm Bel Iblis and
    // Adar Tallon sit on STRATEGY 11501 (the en route mark, FUN_0042c3b0
    // +0x50 bit 4) over 11500, Syub Snunb, who stays, on 11500 alone;
    // the Longprobe team (0x3c000003) on its own GOKRES mark, mini +
    // 0x1000.
    let (mut world, system, mut missions) = stocked();
    let walker = add_character(&mut world, system, "Walker", true);
    world.characters[walker].current_system = None;
    let force = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0003),
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
        on_hidden_mission: false,
    });
    for member in [
        MissionMember::Character(walker),
        MissionMember::SpecialForce(force),
    ] {
        missions.push_transit(rebellion_core::missions::MemberTransit {
            member,
            mission_id: 0,
            to: system,
            arrival: 9,
            abode: None,
        });
    }
    let rows = page_rows(
        &world,
        &seen(system),
        &missions,
        idle(),
        Faction::Alliance,
        system,
        DefensesPage::Personnel,
    );
    let marks = |label: &str| {
        let row = rows.iter().find(|row| row.label == label).expect(label);
        (row.mini, row_marks(&world, missions.en_route(), &[], row))
    };
    assert_eq!(
        marks("Walker").1,
        [(DllSource::Strategy, 11_500), (DllSource::Strategy, 11_501)]
    );
    assert_eq!(marks("Idle").1, [(DllSource::Strategy, 11_500)]);
    let (mini, team) = marks("Longprobe Y-wing Recon Team");
    assert_eq!(
        team,
        [
            (DllSource::Strategy, 11_500),
            (DllSource::Gokres, mini + 0x1000)
        ]
    );
}

#[test]
fn units_being_built_for_the_system_follow_its_own_with_the_unfinished_mark() {
    // The original creates the unit at the order, in its destination,
    // not yet completed (+0x50 bit 2 clear): A0 pending-facility/23
    // lists Pzob's A-wing after its X-wings, over STRATEGY 11570
    // (0x2d32, side 1; FUN_0042c3b0 draws it last). A defense facility
    // en route (bit 4) takes 11509 (0x2cf5, families 0x22..0x27).
    use rebellion_core::manufacturing::{Departure, FacilityBuild, QueueItem};
    let (mut world, system, missions) = stocked();
    let class_named = |name: &str| {
        world
            .fighter_classes
            .iter()
            .find(|(_, class)| class.name == name)
            .map(|(key, _)| key)
            .unwrap()
    };
    let xwing = class_named("X-wing");
    let tie = class_named("TIE Fighter");
    let build = |raw, is_alliance| {
        BuildableKind::DefenseFacility(FacilityBuild {
            class: DatId::new(raw),
            is_alliance,
        })
    };
    let mut manufacturing = ManufacturingState::default();
    for kind in [
        BuildableKind::Fighter(xwing),
        BuildableKind::Fighter(tie),
        build(KDY, true),
        build(KDY, false),
    ] {
        manufacturing.enqueue(system, QueueItem::new(kind, 5));
    }
    // The squadrons are products in the system from the order
    // (FUN_0052a140; rebellion_core::products).
    rebellion_core::products::reconcile(&mut world, &manufacturing);
    // The built rows, without the products.
    let mut bare = world.clone();
    bare.products = rebellion_core::products::Products::default();
    let mut deliveries = DeliveryState::default();
    deliveries.depart(
        &world,
        &[Departure {
            origin: system,
            destination: system,
            tick: 0,
            kind: build(GENCORE, true),
            name: None,
        }],
    );
    let arrival = deliveries.en_route()[0].arrival_tick;
    let production = DefensesProduction {
        manufacturing: &manufacturing,
        deliveries: &deliveries,
        regiments: &[],
        squadrons: &[],
    };
    let fog = seen(system);
    let rows = |page| {
        page_rows(
            &world,
            &fog,
            &missions,
            production,
            Faction::Alliance,
            system,
            page,
        )
    };
    let tail = |page| {
        let built = labels(&bare, &fog, &missions, system, page).len();
        rows(page).split_off(built)
    };

    let [squadron] = &tail(DefensesPage::Squadrons)[..] else {
        panic!("{:?}", tail(DefensesPage::Squadrons));
    };
    assert_eq!(squadron.label, "X-wing");
    assert_eq!(
        squadron.item.menu_object(),
        Some(MenuObject::PendingFighter {
            system,
            class: xwing,
            product: Some(1),
        })
    );
    assert_eq!(
        row_marks(&world, &[], &[], squadron),
        [(DllSource::Strategy, 11_570)]
    );

    let [battery] = &tail(DefensesPage::Batteries)[..] else {
        panic!("{:?}", tail(DefensesPage::Batteries));
    };
    assert_eq!(
        battery.label,
        defense_facility_mini(DatId::new(KDY)).unwrap().1
    );
    assert_eq!(
        row_marks(&world, &[], &[], battery),
        [(DllSource::Strategy, 11_570)]
    );
    assert_eq!(
        battery.item.menu_object(),
        Some(MenuObject::PendingFacility {
            system,
            class: DatId::new(KDY),
            owner: 1,
            arrival: None,
        })
    );

    let [shield] = &tail(DefensesPage::Shields)[..] else {
        panic!("{:?}", tail(DefensesPage::Shields));
    };
    assert_eq!(
        row_marks(&world, &[], &[], shield),
        [(DllSource::Strategy, 11_509)]
    );
    assert_eq!(
        shield.item.menu_object(),
        Some(MenuObject::PendingFacility {
            system,
            class: DatId::new(GENCORE),
            owner: 1,
            arrival: Some(arrival),
        })
    );
    // Regiments and personnel take nothing from the queues.
    for page in [DefensesPage::Personnel, DefensesPage::Regiments] {
        assert!(tail(page).is_empty());
    }
}

#[test]
fn a_regiment_in_training_lists_on_the_regiment_page_over_its_backdrop_and_mark() {
    // FUN_00504cc0 walks the regiments; one not yet trained (+0x50 bit 2
    // clear) takes FUN_0042c3b0's backdrop 11513 (0x2cf9), then 11570
    // (0x2d32, side 1) on top. FUN_00504b30's list: Move, Confirmed
    // Move, Scrap.
    use rebellion_core::manufacturing::QueueItem;
    let (mut world, system, missions) = stocked();
    for (raw, is_alliance) in [(ALLIANCE_ARMY, true), (STORMTROOPERS, false)] {
        world.buildable_classes.insert(
            DatId::new(raw),
            rebellion_core::world::BuildableClass {
                is_alliance,
                is_empire: !is_alliance,
                ..Default::default()
            },
        );
    }
    let mut manufacturing = ManufacturingState::default();
    for raw in [ALLIANCE_ARMY, STORMTROOPERS] {
        manufacturing.enqueue(
            system,
            QueueItem::new(BuildableKind::Troop(DatId::new(raw)), 5),
        );
    }
    rebellion_core::products::reconcile(&mut world, &manufacturing);
    let mut bare = world.clone();
    bare.products = rebellion_core::products::Products::default();
    let deliveries = DeliveryState::default();
    let production = DefensesProduction {
        manufacturing: &manufacturing,
        deliveries: &deliveries,
        regiments: &[],
        squadrons: &[],
    };
    let fog = seen(system);
    let built = labels(&bare, &fog, &missions, system, DefensesPage::Regiments).len();
    let rows = page_rows(
        &world,
        &fog,
        &missions,
        production,
        Faction::Alliance,
        system,
        DefensesPage::Regiments,
    );
    let [training] = &rows[built..] else {
        panic!("{rows:?}");
    };
    assert_eq!(
        training.label,
        troop_mini(DatId::new(ALLIANCE_ARMY)).unwrap().1
    );
    assert_eq!(
        row_marks(&world, &[], &[], training),
        [(DllSource::Strategy, 11_513), (DllSource::Strategy, 11_570)]
    );
    assert_eq!(
        training.item.menu_object(),
        Some(MenuObject::PendingTroop {
            system,
            class: DatId::new(ALLIANCE_ARMY),
            product: Some(1),
        })
    );
}

#[test]
fn a_finished_regiment_or_squadron_on_its_trip_takes_the_en_route_mark() {
    // FUN_0052bee0: finished away from its yard it is Completed and en
    // route (bit 4) in its container; FUN_0042c3b0 then lays the en
    // route mark over the backdrop, and no unfinished mark (A0
    // 2026-11-11-regiment-en-route/00 for a regiment on its way).
    let row = |kind, mini| DefensesRow {
        item: DefensesItem::Pending {
            system: SystemKey::default(),
            index: 0,
            kind,
            arrival: Some(9),
            product: Some(1),
        },
        mini,
        label: String::new(),
    };
    let world = GameWorld::default();
    let regiment = DatId::new(ALLIANCE_ARMY);
    let (mini, _) = troop_mini(regiment).unwrap();
    assert_eq!(
        row_marks(&world, &[], &[], &row(BuildableKind::Troop(regiment), mini)),
        crate::fleet_window::mini_marks(
            crate::fleet_window::MiniObject::Regiment(regiment),
            mini,
            true,
            false
        )
    );
    let squadron = row(BuildableKind::Fighter(FighterKey::default()), 17_000);
    let marks = row_marks(&world, &[], &[], &squadron);
    assert_eq!(
        marks,
        crate::fleet_window::mini_marks(
            crate::fleet_window::MiniObject::Craft,
            17_000,
            true,
            false
        )
    );
    assert!(!marks.contains(&(DllSource::Strategy, 11_572)));
    assert!(!marks.contains(&(DllSource::Strategy, 11_509)));
}

#[test]
fn squadrons_travelling_to_the_planet_list_after_its_own_with_the_en_route_mark() {
    // FUN_00556390 puts a squadron sent by Move in its destination at
    // once, in hyperspace (+0x50 bit 4), as A0 2026-11-11-regiment-en-
    // route/00 shows for a regiment; FUN_005039d0 lists it there and
    // FUN_0042c3b0 draws a craft's own en route mark (mini + 0x1000).
    let (mut world, system, missions) = stocked();
    let xwing = world.systems[system].fighters[0].class;
    let tie = world.systems[system].fighters[1].class;
    let other = world.systems.insert(world.systems[system].clone());
    let transit = |class, count, to| SquadronTransit {
        class,
        count,
        origin: other,
        to,
        arrival_tick: 77,
        lost: Vec::new(),
    };
    let squadrons = [
        transit(xwing, 2, SquadronPlace::Planet(system)),
        transit(xwing, 1, SquadronPlace::Planet(other)),
        // The other side's travel lists on its own side's page.
        transit(tie, 1, SquadronPlace::Planet(system)),
    ];
    let manufacturing = ManufacturingState::default();
    let deliveries = DeliveryState::default();
    let rows = page_rows(
        &world,
        &seen(system),
        &missions,
        DefensesProduction {
            manufacturing: &manufacturing,
            deliveries: &deliveries,
            regiments: &[],
            squadrons: &squadrons,
        },
        Faction::Alliance,
        system,
        DefensesPage::Squadrons,
    );
    assert_eq!(
        rows.iter().map(|row| row.item).collect::<Vec<_>>(),
        [
            DefensesItem::Squadron {
                system,
                class: xwing,
                nth: 0
            },
            DefensesItem::Squadron {
                system,
                class: xwing,
                nth: 1
            },
            DefensesItem::TravellingSquadron {
                system,
                class: xwing,
                arrival: 77,
                nth: 0
            },
            DefensesItem::TravellingSquadron {
                system,
                class: xwing,
                arrival: 77,
                nth: 1
            },
        ]
    );
    let travelling = &rows[2];
    assert_eq!(
        row_marks(&world, &[], &[], travelling),
        [(DllSource::Gokres, travelling.mini + 0x1000)]
    );
    assert_eq!(row_marks(&world, &[], &[], &rows[0]), []);
    assert_eq!(
        travelling.item.menu_object(),
        Some(MenuObject::TravellingFighter {
            to: SquadronPlace::Planet(system),
            class: xwing,
            arrival: 77
        })
    );
}

#[test]
fn a_regiment_travelling_to_the_planet_lists_on_its_page_with_the_en_route_mark() {
    // A0 2026-11-11-regiment-en-route/00: Averam's Trooper Regiments
    // page lists the Alliance Army Regiment moving there from Yavin,
    // its mini over the striped en route mark (FUN_0042c3b0 bit 4; class
    // 0x10000002 takes its own GOKRES mark, the mini plus 0x1000).
    let (mut world, system, missions) = stocked();
    let built = labels(
        &world,
        &seen(system),
        &missions,
        system,
        DefensesPage::Regiments,
    )
    .len();
    let moving = world.troops.insert(rebellion_core::world::TroopUnit {
        class_dat_id: DatId::new(ALLIANCE_ARMY),
        is_alliance: true,
        regiment_strength: 100,
    });
    let elsewhere = world.troops.insert(rebellion_core::world::TroopUnit {
        class_dat_id: DatId::new(ALLIANCE_ARMY),
        is_alliance: true,
        regiment_strength: 100,
    });
    let other_system = world.systems.insert(world.systems[system].clone());
    let regiments = [
        RegimentTransit {
            troop: moving,
            origin: other_system,
            leg: RegimentLeg::Surface(system),
            arrival_tick: 135,
        },
        RegimentTransit {
            troop: elsewhere,
            origin: system,
            leg: RegimentLeg::Surface(other_system),
            arrival_tick: 135,
        },
    ];
    let manufacturing = ManufacturingState::default();
    let deliveries = DeliveryState::default();
    let rows = page_rows(
        &world,
        &seen(system),
        &missions,
        DefensesProduction {
            manufacturing: &manufacturing,
            deliveries: &deliveries,
            regiments: &regiments,
            squadrons: &[],
        },
        Faction::Alliance,
        system,
        DefensesPage::Regiments,
    );
    let [travelling] = &rows[built..] else {
        panic!("{rows:?}");
    };
    assert_eq!(travelling.item, DefensesItem::Troop(moving));
    assert_eq!(
        travelling.item.menu_object(),
        Some(MenuObject::Troop(moving))
    );
    assert_eq!(
        row_marks(&world, &[], &regiments, travelling),
        [
            (DllSource::Strategy, 11_513),
            (DllSource::Gokres, travelling.mini + 0x1000)
        ]
    );
    assert_eq!(
        row_marks(&world, &[], &[], travelling),
        [(DllSource::Strategy, 11_513)]
    );
    // The planet's own regiments, not travelling, keep the backdrop
    // alone (FUN_0042c3b0: no +0x50 bit 4).
    assert!(built > 0);
    for staying in &rows[..built] {
        assert_eq!(
            row_marks(&world, &[], &regiments, staying),
            [(DllSource::Strategy, 11_513)]
        );
    }
}

#[test]
fn the_empires_unfinished_unit_takes_its_sides_mark() {
    // FUN_0042c3b0: 0x2d34 (11572) when +0x24's side bits are not 1.
    use rebellion_core::manufacturing::{FacilityBuild, QueueItem};
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    let mut manufacturing = ManufacturingState::default();
    manufacturing.enqueue(
        system,
        QueueItem::new(
            BuildableKind::DefenseFacility(FacilityBuild {
                class: DatId::new(KDY),
                is_alliance: false,
            }),
            5,
        ),
    );
    rebellion_core::products::reconcile(&mut world, &manufacturing);
    let deliveries = DeliveryState::default();
    let production = DefensesProduction {
        manufacturing: &manufacturing,
        deliveries: &deliveries,
        regiments: &[],
        squadrons: &[],
    };
    let rows = page_rows(
        &world,
        &seen(system),
        &MissionState::new(),
        production,
        Faction::Empire,
        system,
        DefensesPage::Batteries,
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(
        row_marks(&world, &[], &[], &rows[0]),
        [(DllSource::Strategy, 11_572)]
    );
}

#[test]
fn a_squadron_row_opens_its_class_s_menu_and_names_no_mission_target() {
    // FUN_004a7a20 gives the row's object; a squadron's class menu is
    // FUN_00502fe0's. port: a mission names squadrons aboard fleets only.
    let (world, system, missions) = stocked();
    let rows = page_rows(
        &world,
        &seen(system),
        &missions,
        idle(),
        Faction::Alliance,
        system,
        DefensesPage::Squadrons,
    );
    let DefensesItem::Squadron { class, nth, .. } = rows[1].item else {
        panic!("{:?}", rows[1].item);
    };
    assert_eq!(nth, 1);
    assert_eq!(
        rows[1].item.menu_object(),
        Some(MenuObject::PlanetFighter { system, class })
    );
}

#[test]
fn an_enemy_held_system_lists_the_enemys_objects_only_while_the_player_sees_them() {
    // +0x148 is the system's side, not the player's. port: the other
    // side's objects show only where the System window shows them.
    let (mut world, system, missions) = stocked();
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    let unseen = FogState::new(Faction::Alliance);
    // With no copy of the system the Alliance sees nothing there.
    assert!(labels(&world, &unseen, &missions, system, DefensesPage::Batteries).is_empty());
    // The fog's system gives the Alliance its copies of the regiments
    // there (object_views::seed_from_fog; FUN_004f9a60) and a live
    // copy of the system.
    seed_views(&mut world, &seen(system));

    assert_eq!(
        labels(
            &world,
            &seen(system),
            &missions,
            system,
            DefensesPage::Regiments
        ),
        ["Stormtrooper Regiment"]
    );
    assert_eq!(
        labels(
            &world,
            &seen(system),
            &missions,
            system,
            DefensesPage::Squadrons
        ),
        ["TIE Fighter"]
    );
    assert_eq!(
        labels(
            &world,
            &seen(system),
            &missions,
            system,
            DefensesPage::Personnel
        ),
        ["Enemy"]
    );
}

/// Each side's copies of the other side's regiments in the systems
/// `fog` shows (the v116 migration's rule), and live copies of those
/// systems, so the rest shows live.
fn seed_views(world: &mut GameWorld, fog: &FogState) {
    rebellion_core::object_views::seed_from_fog(
        world,
        &rebellion_core::troop_transport::TroopTransportState::new(),
        &rebellion_core::movement::MovementState::new(),
        [fog, &FogState::new(Faction::Empire)],
    );
    world.object_views.alliance_systems.clear();
    world.object_views.empire_systems.clear();
    // The fog's systems stand for systems the Alliance sees: a live
    // copy (FUN_004f9a60; ghidra/notes/fog-of-war.md).
    let shown: Vec<SystemKey> = world
        .systems
        .keys()
        .filter(|key| fog.is_visible(*key))
        .collect();
    for system in shown {
        crate::viewed_contents::see_live(world, fog.faction, system);
    }
}

#[test]
fn the_enemys_regiments_are_the_players_copies_not_the_live_ones() {
    // FUN_004f2d10 -> FUN_0053efd0: the window reads the player's side's
    // copy; FUN_004f9a60 keeps one only where the side has seen it. A
    // fog that shows the system without a copy lists no regiment.
    let (mut world, system, missions) = stocked();
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    let fog = seen(system);
    assert!(labels(&world, &fog, &missions, system, DefensesPage::Regiments).is_empty());

    // A copy taken out of sight stays when the regiment leaves.
    seed_views(&mut world, &fog);
    let key = world.systems[system]
        .ground_units
        .iter()
        .copied()
        .find(|key| !world.troops[*key].is_alliance)
        .unwrap();
    world.systems[system].ground_units.retain(|unit| *unit != key);
    let rows = page_rows(
        &world,
        &fog,
        &missions,
        idle(),
        Faction::Alliance,
        system,
        DefensesPage::Regiments,
    );
    assert_eq!(
        rows.iter().map(|row| row.label.as_str()).collect::<Vec<_>>(),
        ["Stormtrooper Regiment"]
    );
    let id = world.object_views.regiment_id(key).unwrap();
    assert_eq!(rows[0].item, DefensesItem::ViewedTroop { alliance: true, id });
    assert_eq!(
        rows[0].item.menu_object(),
        Some(MenuObject::ViewedTroop { alliance: true, id })
    );
    // The Empire's own page lists its live regiments only.
    assert!(page_rows(
        &world,
        &fog,
        &missions,
        idle(),
        Faction::Empire,
        system,
        DefensesPage::Regiments,
    )
    .is_empty());
}

#[test]
fn a_live_copy_lists_the_live_regiment() {
    // FUN_004f9a60: an Alliance fleet in the system keeps the copy live.
    let (mut world, system, missions) = stocked();
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    world.fleets.insert(rebellion_core::world::Fleet {
        location: system,
        is_alliance: true,
        ..Default::default()
    });
    rebellion_core::object_views::advance(
        &mut world,
        &rebellion_core::troop_transport::TroopTransportState::new(),
        &rebellion_core::movement::MovementState::new(),
    );
    let rows = page_rows(
        &world,
        &FogState::new(Faction::Alliance),
        &missions,
        idle(),
        Faction::Alliance,
        system,
        DefensesPage::Regiments,
    );
    assert_eq!(rows.len(), 1);
    assert!(matches!(rows[0].item, DefensesItem::Troop(_)));
}

#[test]
fn a_frozen_copy_en_route_takes_the_en_route_mark() {
    // A0 OBJ-02-C010 (18-chandrila-army-regiment-status-enroute-eta-
    // day171-on-day576-stale, 6a1269a1765f7c9b): the copy is en route
    // (+0x50 bit 4) long after; FUN_0042c3b0 draws the regiment's
    // en route mark.
    use rebellion_core::object_views::{apply, ObjectView, RegimentView, ViewPlace, ViewUpdate};
    let (mut world, system, missions) = stocked();
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    let id = DatId::new(0x1000_0042);
    let place = ViewPlace::Surface(world.systems[system].dat_id);
    apply(
        &mut world,
        &[ViewUpdate::Set {
            side: Faction::Alliance,
            id,
            view: ObjectView::Regiment(RegimentView {
                class: DatId::new(STORMTROOPERS),
                is_alliance: false,
                strength: 100,
                place,
                arrival: Some(171),
                live: false,
            }),
        }],
    );
    let rows = page_rows(
        &world,
        &FogState::new(Faction::Alliance),
        &missions,
        idle(),
        Faction::Alliance,
        system,
        DefensesPage::Regiments,
    );
    assert_eq!(rows.len(), 1);
    let marks = row_marks(&world, &[], &[], &rows[0]);
    let still = DefensesRow {
        item: DefensesItem::Troop(world.systems[system].ground_units[0]),
        ..rows[0].clone()
    };
    assert_ne!(marks, row_marks(&world, &[], &[], &still));
    assert_eq!(
        marks,
        crate::fleet_window::mini_marks(
            crate::fleet_window::MiniObject::Regiment(DatId::new(STORMTROOPERS)),
            rows[0].mini,
            true,
            false,
        )
    );
}

/// The Alliance's frozen copy of `system` as it stands now
/// (`SystemView`, `FUN_004f9a60`).
fn freeze(world: &mut GameWorld, system: SystemKey) {
    let view = rebellion_core::object_views::SystemView::of(world, &world.systems[system], false);
    let id = world.systems[system].dat_id;
    world.object_views.alliance_systems.insert(id, view);
}

#[test]
fn a_frozen_system_lists_the_enemys_objects_as_the_copy_holds_them() {
    // FUN_004f2d10 reads the player's copy; FUN_004f9a60 froze it: the
    // facilities and squadrons the copy holds stay listed after the
    // live ones are gone, and a battery built since is not.
    let (mut world, system, missions) = stocked();
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    // The copy names its characters by id (the test characters share
    // one): the others take ids of their own.
    for (n, character) in world.characters.values_mut().enumerate() {
        if character.name != "Enemy" {
            character.dat_id = DatId::new(0x3000_0100 + u32::try_from(n).unwrap_or_default());
        }
    }
    freeze(&mut world, system);
    let batteries = labels(&world, &FogState::new(Faction::Alliance), &missions, system, DefensesPage::Batteries);
    let squadrons = labels(&world, &FogState::new(Faction::Alliance), &missions, system, DefensesPage::Squadrons);
    let personnel = labels(&world, &FogState::new(Faction::Alliance), &missions, system, DefensesPage::Personnel);
    assert!(!batteries.is_empty());
    assert_eq!(squadrons, ["TIE Fighter"]);
    assert_eq!(personnel, ["Enemy"]);
    world.systems[system].defense_facilities.clear();
    world.systems[system].fighters.clear();
    // The fog shows the system now, but the copy is frozen: the live
    // objects stay hidden.
    let fog = seen(system);
    assert_eq!(
        labels(&world, &fog, &missions, system, DefensesPage::Batteries),
        batteries
    );
    assert_eq!(
        labels(&world, &fog, &missions, system, DefensesPage::Squadrons),
        ["TIE Fighter"]
    );
    let rows = page_rows(&world, &fog, &missions, idle(), Faction::Alliance, system, DefensesPage::Squadrons);
    let DefensesItem::Viewed { index: 0, unit, .. } = rows[0].item else {
        panic!("{:?}", rows[0].item);
    };
    assert_eq!(
        rows[0].item.menu_object(),
        Some(MenuObject::ViewedUnit { system, unit })
    );
    // The copy keeps the side the system had: neutral now, the window
    // still shows the Empire's objects.
    world.systems[system].control = ControlKind::Uncontrolled;
    assert_eq!(
        labels(&world, &fog, &missions, system, DefensesPage::Squadrons),
        ["TIE Fighter"]
    );
    // With no copy and no fog, nothing.
    world.object_views.alliance_systems.clear();
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    assert!(labels(
        &world,
        &FogState::new(Faction::Alliance),
        &missions,
        system,
        DefensesPage::Batteries
    )
    .is_empty());
}

#[test]
fn a_frozen_copy_lists_only_the_shown_sides_objects_of_each_page() {
    // FUN_004a90d0 filters each page to the shown side and its families,
    // over the copy too: the Alliance's own facilities and characters in
    // the Empire's frozen copy of an Alliance system are the Empire's
    // other side's; shields and batteries keep their families.
    let (mut world, system, missions) = stocked();
    for (n, character) in world.characters.values_mut().enumerate() {
        if character.name != "Idle" {
            character.dat_id = DatId::new(0x3000_0100 + u32::try_from(n).unwrap_or_default());
        }
    }
    // One Imperial battery fewer than the Alliance's, so the sides'
    // lists differ.
    let imperial = world.systems[system]
        .defense_facilities
        .iter()
        .position(|key| {
            let facility = &world.defense_facilities[*key];
            facility.side == Faction::Empire && defense_page_family(DefensesPage::Batteries, facility.class_dat_id.family())
        })
        .unwrap();
    world.systems[system].defense_facilities.remove(imperial);
    let view = rebellion_core::object_views::SystemView::of(&world, &world.systems[system], false);
    let id = world.systems[system].dat_id;
    world.object_views.empire_systems.insert(id, view);
    let empire = |world: &GameWorld, page| {
        page_rows(world, &FogState::new(Faction::Empire), &missions, idle(), Faction::Empire, system, page)
            .into_iter()
            .map(|row| row.label)
            .collect::<Vec<_>>()
    };
    let live = |page| {
        let mut fog = FogState::new(Faction::Alliance);
        fog.reveal(system);
        page_rows(&world, &fog, &missions, idle(), Faction::Alliance, system, page)
            .into_iter()
            .map(|row| row.label)
            .collect::<Vec<_>>()
    };
    assert_eq!(empire(&world, DefensesPage::Batteries), live(DefensesPage::Batteries));
    assert_eq!(empire(&world, DefensesPage::Shields), live(DefensesPage::Shields));
    assert_ne!(
        empire(&world, DefensesPage::Batteries),
        empire(&world, DefensesPage::Shields)
    );
    assert!(empire(&world, DefensesPage::Personnel).contains(&"Idle".to_owned()));
}

#[test]
fn units_seen_leaving_for_a_frozen_system_list_there_en_route() {
    // FUN_00556390 puts a unit in its destination at once, in
    // hyperspace; FUN_004f9a60 froze the player's copy of it (A0
    // OBJ-02-C010's regiment, for a special force and a squadron).
    use crate::viewed_contents::ViewedUnit;
    use rebellion_core::object_views::{SystemExtras, TravellerCopy};
    let (mut world, system, missions) = stocked();
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    world.systems[system].fighters.clear();
    world.systems[system].special_forces.clear();
    freeze(&mut world, system);
    let tie = world
        .fighter_classes
        .values()
        .find(|class| !class.is_alliance)
        .unwrap()
        .dat_id;
    let id = world.systems[system].dat_id;
    world.object_views.alliance_extras.insert(
        id,
        SystemExtras {
            live: false,
            fleets: Vec::new(),
            missions: Vec::new(),
            travellers: vec![
                TravellerCopy::Squadron {
                    class: tie,
                    is_alliance: false,
                    count: 2,
                    arrival: 60,
                },
                TravellerCopy::SpecialForce {
                    class: DatId::new(0x3c00_0007),
                    is_alliance: false,
                    arrival: 61,
                },
                TravellerCopy::Character {
                    id: DatId::new(832),
                    arrival: 62,
                },
            ],
        },
    );
    let rows = |page| {
        page_rows(
            &world,
            &FogState::new(Faction::Alliance),
            &missions,
            idle(),
            Faction::Alliance,
            system,
            page,
        )
    };
    let squadrons = rows(DefensesPage::Squadrons);
    assert_eq!(squadrons.len(), 2);
    let DefensesItem::Viewed { unit, .. } = squadrons[1].item else {
        panic!("{:?}", squadrons[1].item);
    };
    assert_eq!(
        unit,
        ViewedUnit::Squadron {
            class: tie,
            is_alliance: false,
            arrival: Some(60)
        }
    );
    let marks = row_marks(&world, &[], &[], &squadrons[0]);
    assert_eq!(
        marks,
        crate::fleet_window::mini_marks(
            crate::fleet_window::MiniObject::Craft,
            squadrons[0].mini,
            true,
            false
        )
    );
    // An Imperial character seen leaving, not in the copy's own list.
    // (It keeps the test characters' portrait id; the others take ids
    // of their own, as the copy names characters by id.)
    for (n, character) in world.characters.values_mut().enumerate() {
        character.dat_id = DatId::new(0x3000_0100 + u32::try_from(n).unwrap_or_default());
    }
    let traveller = add_character(&mut world, system, "Traveller", false);
    world.characters[traveller].current_system = None;
    world
        .object_views
        .alliance_systems
        .get_mut(&id)
        .unwrap()
        .characters
        .clear();
    let rows = |page| {
        page_rows(
            &world,
            &FogState::new(Faction::Alliance),
            &missions,
            idle(),
            Faction::Alliance,
            system,
            page,
        )
    };
    let personnel = rows(DefensesPage::Personnel);
    assert!(personnel.iter().any(|row| row.label == "Traveller"));
    let force = personnel
        .iter()
        .find(|row| matches!(row.item, DefensesItem::Viewed { .. }))
        .unwrap();
    assert_eq!(
        row_marks(&world, &[], &[], force),
        crate::fleet_window::mini_marks(
            crate::fleet_window::MiniObject::SpecialForce(DatId::new(0x3c00_0007)),
            force.mini,
            true,
            false
        )
    );
}

#[test]
fn see_everything_lists_the_enemys_live_objects_past_the_fog_and_the_views() {
    // port: the debug "see everything" mode (ObjectViews::see_everything).
    let (mut world, system, missions) = stocked();
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    let unseen = FogState::new(Faction::Alliance);
    assert!(labels(&world, &unseen, &missions, system, DefensesPage::Regiments).is_empty());
    assert!(labels(&world, &unseen, &missions, system, DefensesPage::Batteries).is_empty());
    world.object_views.see_everything = true;
    assert_eq!(
        labels(&world, &unseen, &missions, system, DefensesPage::Regiments),
        ["Stormtrooper Regiment"]
    );
    assert!(!labels(&world, &unseen, &missions, system, DefensesPage::Batteries).is_empty());
    assert!(matches!(
        page_rows(
            &world,
            &unseen,
            &missions,
            idle(),
            Faction::Alliance,
            system,
            DefensesPage::Regiments,
        )[0]
        .item,
        DefensesItem::Troop(_)
    ));
}

#[test]
fn an_unheld_or_unexplored_system_lists_nothing_of_either_side() {
    // Side bits 0 or 3 match neither side's objects.
    let (mut world, system, missions) = stocked();
    let fog = seen(system);
    for control in [ControlKind::Uncontrolled, ControlKind::Contested] {
        world.systems[system].control = control;
        for page in DefensesPage::ALL {
            assert!(labels(&world, &fog, &missions, system, page).is_empty());
        }
    }
    // A system the player's copy holds unexplored (a side's own system
    // is always explored to it: ghidra/notes/fog-of-war.md).
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    crate::viewed_contents::see_unexplored(&mut world, Faction::Alliance, system);
    assert!(labels(&world, &fog, &missions, system, DefensesPage::Batteries).is_empty());
}

#[test]
fn each_tab_shows_its_side_art_selected_or_empty() {
    // FUN_004a8790 and FUN_004a9ce0: normal, the empty id two higher,
    // the middle id selected; FUN_0060d700 copies the pressed bitmap
    // into the unset selected state of tabs 4 and 5.
    let cases = [
        (DefensesPage::Batteries, 1, [10550, 10551, 10552]),
        (DefensesPage::Batteries, 3, [10550, 10551, 10552]),
        (DefensesPage::Shields, 2, [10553, 10554, 10555]),
        (DefensesPage::Squadrons, 1, [10556, 10557, 10558]),
        (DefensesPage::Squadrons, 2, [10559, 10560, 10561]),
        (DefensesPage::Squadrons, 0, [10562; 3]),
        (DefensesPage::Regiments, 1, [10563, 10564, 10565]),
        (DefensesPage::Regiments, 2, [10566, 10567, 10568]),
        (DefensesPage::Regiments, 3, [10569; 3]),
        (DefensesPage::Personnel, 1, [10570, 10571, 10572]),
        (DefensesPage::Personnel, 2, [10573, 10574, 10575]),
        (DefensesPage::Personnel, 0, [10576; 3]),
    ];
    for (page, side, [normal, selected, empty]) in cases {
        assert_eq!(
            [
                tab_resource(page, side, false, false),
                tab_resource(page, side, true, true),
                tab_resource(page, side, false, true),
            ],
            [normal, selected, empty],
            "{page:?} side {side}"
        );
    }
    // TEXTSTRA 5952..5956.
    assert_eq!(
        DefensesPage::ALL.map(DefensesPage::name),
        [
            "Personnel",
            "Trooper Regiments",
            "Fighter Squadrons",
            "Planetary Shields",
            "Planetary Batteries",
        ]
    );
}

#[test]
fn the_garrison_line_shows_on_the_regiment_page_of_the_players_own_system() {
    // FUN_004a90d0's tail: TEXTSTRA 6471 and +0x80 on page 2 when the
    // side bits are the player's (FUN_0041cdb0).
    let (world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let mut economy = EconomyState::default();
    economy.per_system.insert(
        system,
        SystemEconomy {
            garrison_requirement: 3,
            ..SystemEconomy::default()
        },
    );
    let line = |player, page| garrison_line(&world, &economy, player, system, page);

    assert_eq!(
        line(Faction::Alliance, DefensesPage::Regiments).as_deref(),
        Some("Garrison Requirement: 3")
    );
    assert_eq!(line(Faction::Alliance, DefensesPage::Personnel), None);
    assert_eq!(line(Faction::Empire, DefensesPage::Regiments), None);
    assert_eq!(
        garrison_line(
            &world,
            &EconomyState::default(),
            Faction::Alliance,
            system,
            DefensesPage::Regiments
        )
        .as_deref(),
        Some("Garrison Requirement: 0")
    );
}

#[test]
fn a_mission_released_on_a_listed_object_targets_it_else_the_window_system() {
    // +0x68 is FUN_004aa380: the item in list +0x164 under the point,
    // else the window's subject. A prisoner lists with its captor's
    // personnel, since it carries the captor's side (FUN_004eec60).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    let guard = add_character(&mut world, system, "Guard", false);
    let prisoner = add_character(&mut world, system, "Prisoner", true);
    world.characters[prisoner].is_captive = true;
    world.characters[prisoner].captured_by = Some(Faction::Empire);
    let troop = world.troops.insert(TroopUnit {
        class_dat_id: DatId::new(0x1000_0001),
        is_alliance: false,
        regiment_strength: 100,
    });
    world.systems[system].ground_units.push(troop);
    let extras: Vec<_> = (0..5)
        .map(|_| add_character(&mut world, system, "Officer", false))
        .collect();
    let missions = MissionState::new();
    let fog = seen(system);
    // The Alliance's copy of the regiment, from its fog (the v116
    // migration's rule): a frozen copy targets its regiment by id.
    seed_views(&mut world, &fog);
    let mut state = opened(&world, system);
    let layer = egui::LayerId::new(egui::Order::Foreground, area_id(system));
    let cell = |state: &DefensesWindowState, index| {
        state
            .cell_screen_rect(scaled(), system, index)
            .unwrap()
            .center()
    };
    let pick = |state: &DefensesWindowState, layer, point| {
        state.mission_target(&world, &fog, &missions, idle(), scaled(), layer, point)
    };

    let first = pick(&state, layer, cell(&state, 0)).unwrap();
    assert_eq!((first.system, first.character), (system, Some(guard)));
    assert_eq!(
        pick(&state, layer, cell(&state, 1)).unwrap().character,
        Some(prisoner)
    );
    // The seventh cell, on the third row of three, holds the last officer.
    assert_eq!(
        pick(&state, layer, cell(&state, 6)).unwrap().character,
        Some(extras[4])
    );
    assert_eq!(
        pick(&state, layer, cell(&state, 7)),
        Some(MissionTargetPick::from(system))
    );
    let elsewhere = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("elsewhere"));
    assert_eq!(pick(&state, elsewhere, cell(&state, 0)), None);

    assert!(state.show_page(system, DefensesPage::Regiments));
    assert_eq!(
        pick(&state, layer, cell(&state, 0)).unwrap().object,
        Some(MissionTarget::Troop(troop))
    );
}

#[test]
fn the_window_takes_its_side_from_the_viewers_frozen_copy() {
    // FUN_004a25c0 -> FUN_004f3220: the window's object is the viewer's
    // copy. Its side bits (+0x24 >> 6) choose the tabs' art
    // (FUN_004a9ce0, +0x148), the title strip and the rail icon
    // (FUN_004aa4a0). The Alliance's frozen copy keeps the Empire's
    // side after the system fell neutral out of its sight.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    let view =
        rebellion_core::object_views::SystemView::of(&world, &world.systems[system], false);
    let id = world.systems[system].dat_id;
    world.object_views.alliance_systems.insert(id, view);
    world.systems[system].control = ControlKind::Uncontrolled;

    assert_eq!(rail_icon(&world, Faction::Alliance, system), 11534);
    assert_eq!(shown_side(&world, Faction::Alliance, system), 2);
    // With no copy (a test's world), the system's own side.
    assert_eq!(rail_icon(&world, Faction::Empire, system), 11535);
}

#[test]
fn the_rail_icon_title_strip_and_selected_color_follow_the_systems_side() {
    // FUN_004aa4a0: 11533, 11534, else 11535; FUN_004a9800's strips;
    // FUN_004a9ce0's +0xdc.
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    assert_eq!(rail_icon(&world, Faction::Empire, system), 11534);
    world.systems[system].control = ControlKind::Controlled(Faction::Alliance);
    assert_eq!(rail_icon(&world, Faction::Empire, system), 11533);
    world.systems[system].control = ControlKind::Contested;
    assert_eq!(rail_icon(&world, Faction::Empire, system), 11535);

    assert_eq!(
        [1, 2, 3].map(|side| [title_resource(side, true), title_resource(side, false)]),
        [[10299, 10200], [10201, 10302], [10303, 10304]]
    );
    assert_eq!(selected_color(1), egui::Color32::from_rgb(255, 0, 0));
    assert_eq!(selected_color(2), egui::Color32::from_rgb(0, 255, 0));
    assert_eq!(selected_color(0), egui::Color32::from_rgb(0, 255, 255));
}

/// The canvas 10 by 20 pixels in and twice the original size.
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

const ORIGIN: (i16, i16) = (20, 30);

/// A logical point in the window opened at `ORIGIN` on [`scaled`].
fn at(x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(
        10.0 + (f32::from(ORIGIN.0) + x) * 2.0,
        20.0 + (f32::from(ORIGIN.1) + y) * 2.0,
    )
}

#[test]
fn the_kind_claims_the_shared_focus_only_while_a_window_is_open() {
    // window_focus.rs: a kind with no window open makes no claim; one it
    // raises claims later than any stamp taken before.
    let (world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let mut state = DefensesWindowState::default();
    assert_eq!(state.focus_stamp(), None);
    let mut earlier = crate::window_focus::FocusStamp::default();
    earlier.take();
    assert!(state.open(&world, system, ORIGIN, CockpitFaction::Alliance, scaled()));
    let stamp = state.focus_stamp().expect("a window is open");
    assert!(stamp > earlier.stamp(), "{stamp} after {}", earlier.stamp());
    assert!(state.focus.holds());
    state.set_holds_focus(false);
    assert!(!state.focus.holds());
}

fn opened(world: &GameWorld, system: SystemKey) -> DefensesWindowState {
    let mut state = DefensesWindowState::default();
    assert!(state.open(world, system, ORIGIN, CockpitFaction::Alliance, scaled()));
    state
}

#[test]
fn the_windows_rects_and_release_target_follow_the_canvas_offset_and_scale() {
    // The strip at (0, 20) with 36 by 33 buttons; the list at (7, 81)
    // with 70 by 70 cells (FUN_004a8790, FUN_00609ae0); +0x70 gives the
    // subject (FUN_004aa470).
    let (world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let mut state = opened(&world, system);
    // FUN_0045aac0 tells the agent of a created window only.
    assert!(state.take_created());
    assert!(state.open(
        &world,
        system,
        (300, 100),
        CockpitFaction::Alliance,
        scaled()
    ));
    assert!(!state.take_created());
    assert_eq!(state.window_count(), 1);
    assert!(state.is_open(system));

    assert_eq!(
        state.tab_screen_rect(scaled(), system, DefensesPage::Shields),
        Some(egui::Rect::from_min_size(
            at(136.0, 20.0),
            egui::vec2(72.0, 66.0)
        ))
    );
    assert_eq!(
        state.cell_screen_rect(scaled(), system, 4),
        Some(egui::Rect::from_min_size(
            at(77.0, 151.0),
            egui::vec2(140.0, 140.0)
        ))
    );
    let point = |p: egui::Pos2| (p.x, p.y);
    assert!(state.contains_screen_point(scaled(), point(at(234.0, 303.0))));
    assert!(!state.contains_screen_point(scaled(), point(at(235.5, 150.0))));

    let layer = egui::LayerId::new(egui::Order::Foreground, area_id(system));
    assert_eq!(state.release_target(layer), Some(system));
    let elsewhere = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("elsewhere"));
    assert_eq!(state.release_target(elsewhere), None);

    assert!(state.open(&world, system, ORIGIN, CockpitFaction::Empire, scaled()));
    assert_eq!(state.faction, CockpitFaction::Empire);
    state.clear();
    assert_eq!(state.window_count(), 0);
    assert!(!state.is_open(system));
}

#[derive(Debug, Clone, PartialEq)]
/// A painted line ([`crate::theme::TestLine`]): `pos` is its pen's left
/// and its baseline.
struct Text {
    text: String,
    pos: egui::Pos2,
    color: egui::Color32,
    font: f32,
}

#[derive(Default)]
struct Run {
    actions: Vec<DefensesWindowAction>,
    texts: Vec<Text>,
    painted: Vec<(u32, egui::Pos2)>,
}

/// Draw the windows once per frame of events, on [`scaled`] for the
/// Alliance; the last frame's text and bitmaps are kept.
fn run(
    world: &GameWorld,
    missions: &MissionState,
    state: &mut DefensesWindowState,
    frames: Vec<Vec<egui::Event>>,
) -> Run {
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let fog = world
        .systems
        .keys()
        .fold(FogState::new(Faction::Alliance), |mut fog, key| {
            fog.reveal(key);
            fog
        });
    let mut economy = EconomyState::default();
    for key in world.systems.keys() {
        economy.per_system.insert(
            key,
            SystemEconomy {
                garrison_requirement: 2,
                ..SystemEconomy::default()
            },
        );
    }
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
            result.actions.extend(draw_defenses_windows(
                ctx,
                world,
                &fog,
                missions,
                idle(),
                &economy,
                state,
                CockpitFaction::Alliance,
                scaled(),
                &mut cache,
            ));
        });
        result.painted = PAINTED.with(|painted| painted.take());
        result.texts = crate::theme::test_lines(&output.shapes)
            .into_iter()
            .map(|line| Text {
                text: line.text,
                pos: line.pen,
                color: line.color,
                font: line.size,
            })
            .collect();
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
    click_with(point, egui::Modifiers::default())
}

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

#[test]
fn ctrl_and_shift_clicks_frame_several_rows_and_a_plain_click_one() {
    // FUN_006083c0: the list (flags 0xa0000, FUN_004a8790) is no
    // single-selection list (0x10000), so MK_CONTROL toggles a row and
    // MK_SHIFT adds the cells from the last press to this one
    // (FUN_00609590); each selected row paints its selected image, the
    // frame over the mini (FUN_0060bd00).
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let frames = |run: &Run| {
        run.painted
            .iter()
            .filter(|(id, _)| *id == ROW_FRAME[0])
            .map(|(_, point)| *point)
            .collect::<Vec<_>>()
    };
    let _ = run(&world, &missions, &mut state, click(at(30.0, 100.0)));
    let two = run(
        &world,
        &missions,
        &mut state,
        click_with(at(170.0, 100.0), egui::Modifiers::CTRL),
    );
    assert_eq!(frames(&two), [at(7.0, 81.0), at(147.0, 81.0)]);
    let report = |state: &DefensesWindowState| {
        state
            .report(
                &world,
                &seen(system),
                &missions,
                idle(),
                &EconomyState::default(),
                system,
            )
            .unwrap()
            .selection
    };
    assert_eq!(report(&state), [0, 2]);
    // Ctrl again on a selected row takes it out on the release.
    let _ = run(
        &world,
        &missions,
        &mut state,
        click_with(at(30.0, 100.0), egui::Modifiers::CTRL),
    );
    assert_eq!(report(&state), [2]);
    // Shift from the last press (row 0) to row 1 adds both.
    let _ = run(
        &world,
        &missions,
        &mut state,
        click_with(at(100.0, 100.0), egui::Modifiers::SHIFT),
    );
    assert_eq!(report(&state), [0, 1, 2]);
    // A plain click on a selected row leaves it alone on the release.
    let one = run(&world, &missions, &mut state, click(at(100.0, 100.0)));
    assert_eq!(frames(&one), [at(77.0, 81.0)]);
    assert_eq!(report(&state), [1]);
}

#[test]
fn a_held_drag_paints_the_selected_rows_images_where_the_pointer_moved() {
    // FUN_006083c0 case 0x200: the first move with the button down
    // starts the drag image, the selected items' images as the list
    // lays them out (FUN_00609960), moved with the pointer
    // (FUN_0060df00).
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let start = at(100.0, 100.0);
    let mut frames = hover(start);
    frames.extend([
        vec![press(start, true)],
        vec![egui::Event::PointerMoved(at(110.0, 120.0))],
    ]);
    let held = run(&world, &missions, &mut state, frames);
    assert!(held.painted.contains(&(ROW_FRAME[0], at(77.0, 81.0))));
    assert!(held.painted.contains(&(ROW_FRAME[0], at(87.0, 101.0))));
    // The shield page drags nothing, so it draws no image.
    let _ = run(
        &world,
        &missions,
        &mut state,
        vec![vec![press(at(110.0, 120.0), false)], vec![]],
    );
    let _ = run(&world, &missions, &mut state, click(at(150.0, 30.0)));
    let mut frames = hover(at(30.0, 100.0));
    frames.extend([
        vec![press(at(30.0, 100.0), true)],
        vec![egui::Event::PointerMoved(at(40.0, 120.0))],
    ]);
    let shields = run(&world, &missions, &mut state, frames);
    assert!(!shields
        .painted
        .iter()
        .any(|(id, point)| ROW_FRAME.contains(id) && *point == at(17.0, 101.0)));
}

#[test]
fn a_drag_carries_every_selected_row_and_the_shield_page_drags_nothing() {
    // FUN_006083c0 posts 0x29a for the whole selection (FUN_00422ce0
    // asks the window's slot 22, FUN_004a7a20); a press on a selected
    // row holds its click back (flag 0x10), so the selection stays.
    // Pages 4 and 5 set 0x20000 (FUN_004a90d0): no drag.
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let _ = run(&world, &missions, &mut state, click(at(30.0, 100.0)));
    let _ = run(
        &world,
        &missions,
        &mut state,
        click_with(at(100.0, 100.0), egui::Modifiers::CTRL),
    );
    let outside = at(-60.0, 40.0);
    let drag = |state: &mut DefensesWindowState, from: egui::Pos2| {
        let mut frames = hover(from);
        frames.extend([
            vec![press(from, true)],
            vec![egui::Event::PointerMoved(outside)],
            vec![egui::Event::PointerMoved(outside), press(outside, false)],
        ]);
        run(&world, &missions, state, frames)
            .actions
            .into_iter()
            .filter_map(|action| match action {
                DefensesWindowAction::DragSelection { items, .. } => Some(items),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let dropped = drag(&mut state, at(100.0, 100.0));
    assert_eq!(dropped.len(), 1);
    assert_eq!(dropped[0].len(), 2);
    assert_eq!(state.windows[0].selected.items().len(), 2);
    let _ = run(&world, &missions, &mut state, click(at(150.0, 30.0)));
    assert_eq!(state.windows[0].page, DefensesPage::Shields);
    assert!(drag(&mut state, at(30.0, 100.0)).is_empty());
}

fn text<'a>(run: &'a Run, text: &str) -> &'a Text {
    run.texts
        .iter()
        .find(|value| value.text == text)
        .unwrap_or_else(|| panic!("no {text:?} in {:?}", run.texts))
}

#[test]
fn an_injured_characters_damage_mark_goes_under_its_mini() {
    // FUN_004f7800 sets +0x50 bit 0x200 while injured; FUN_0042c3b0
    // then draws STRATEGY 11502 into the mini's bitmap under the mini.
    let (mut world, system, missions) = stocked();
    let idle = world
        .characters
        .iter()
        .find(|(_, value)| value.name == "Idle")
        .map(|(key, _)| key)
        .unwrap();
    let mut state = opened(&world, system);
    let quiet = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    assert!(!quiet.painted.iter().any(|(id, _)| *id == 11_502));
    world.characters[idle].injury = 3;
    let hurt = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    let cell = at(7.0, 81.0);
    let mark = hurt.painted.iter().position(|entry| *entry == (11_502, cell));
    let mini = hurt.painted.iter().position(|entry| *entry == (18_176 + 832, cell));
    assert!(mark.is_some() && mark < mini, "{:?}", hurt.painted);
    assert_eq!(
        hurt.painted.iter().filter(|(id, _)| *id == 11_502).count(),
        1
    );
    // A captive (+0xac bit 0) takes GOKRES mini + 0x3000 over the mini.
    assert!(!hurt.painted.iter().any(|(id, _)| *id == 18_176 + 832 + 0x3000));
    // The other side's character held here by the Alliance carries its
    // captor's side (FUN_004eec60), so the Alliance's page lists it.
    let enemy = world
        .characters
        .iter()
        .find(|(_, value)| value.name == "Enemy")
        .map(|(key, _)| key)
        .unwrap();
    world.characters[enemy].is_captive = true;
    world.characters[enemy].captured_by = Some(Faction::Alliance);
    let held = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    let (index, &(_, spot)) = held
        .painted
        .iter()
        .enumerate()
        .find(|(_, (id, _))| *id == 18_176 + 832 + 0x3000)
        .unwrap_or_else(|| panic!("{:?}", held.painted));
    let mini = held.painted.iter().position(|entry| *entry == (18_176 + 832, spot));
    assert!(mini.is_some_and(|mini| mini < index), "{:?}", held.painted);
}

#[test]
fn the_window_paints_its_chrome_tabs_and_the_personnel_page_first() {
    // FUN_004a8790 selects tab 1; 10577 at (0, 0), the focused side 1
    // strip at (2, 2), the five buttons at (x, 20), each page's empty
    // art where it lists nothing (FUN_004a9ce0), the rows' minis at
    // their cells' corners and their names from (1, 27).
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let run = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));

    for (id, point) in [
        (BACKGROUND, at(0.0, 0.0)),
        (10299, at(2.0, 2.0)),
        (SECTOR_NORMAL, at(3.0, 3.0)),
        (MINIMIZE_NORMAL, at(204.0, 3.0)),
        (CLOSE_NORMAL, at(218.0, 3.0)),
        (10571, at(28.0, 20.0)),
        (10563, at(64.0, 20.0)),
        (10556, at(100.0, 20.0)),
        (10553, at(136.0, 20.0)),
        (10550, at(172.0, 20.0)),
        (18_176 + 832, at(7.0, 81.0)),
        (18_176 + 832, at(77.0, 81.0)),
        (17_728, at(147.0, 81.0)),
    ] {
        assert!(
            run.painted.contains(&(id, point)),
            "{id} at {point:?} in {:?}",
            run.painted
        );
    }
    assert!(!run.painted.iter().any(|(id, _)| ROW_FRAME.contains(id)));
    // The label from the cell's (1, 27), broken in the cell less 3
    // (FUN_00609ae0's text rect, theme::paint_gdi_text_block), in font
    // 10 (FUN_004a8790: the list's slot +0x18 with 10); the baseline
    // tmAscent 11 below.
    let idle = text(&run, "Idle");
    assert_eq!(idle.pos, at(8.0, 108.0 + 11.0));
    assert_eq!(idle.font, crate::theme::gdi_font(10, 2.0).size);
    assert_eq!(text(&run, "Hidden").color, egui::Color32::WHITE);
    // `+0x49` at (19, 2), 16 high: left-aligned, vertically centred
    // ((16 - 16) / 2, then tmAscent 13).
    let title = text(&run, "Sluis Van");
    assert_eq!(
        (title.pos, title.font),
        (at(19.0, 2.0 + 13.0), crate::theme::gdi_font(5, 2.0).size)
    );
    // `+0x5b` at (2, 51), 231 by 16: centred both ways, in font 4
    // (FUN_004a8790, FUN_00601c60(4)).
    let name = text(&run, "Personnel");
    assert_eq!(name.font, crate::theme::gdi_font(4, 2.0).size);
    let (_, width) = crate::theme::gdi_pen_positions(4, "Personnel").unwrap();
    assert_eq!(
        name.pos,
        at(2.0 + ((231.0 - width) / 2.0).floor(), 51.0 + 13.0)
    );
    assert!(!run
        .texts
        .iter()
        .any(|value| value.text.starts_with("Garrison")));
}

#[test]
fn a_character_dragged_out_of_the_list_drops_where_the_button_comes_up() {
    // FUN_006083c0 captures the mouse on the press and posts 0x29a with
    // the release point; FUN_00422ce0 moves a window type 10 selection
    // with 0x201 (fleet-window.md), the Move mission order 0x241 for a
    // character (manual p. 111). A release inside the list drops
    // nothing.
    let (world, system, missions) = stocked();
    let hidden = world
        .characters
        .iter()
        .find(|(_, c)| c.name == "Hidden")
        .map(|(key, _)| key)
        .expect("the hidden agent");
    let drag_from = |start: egui::Pos2, to: egui::Pos2| {
        let mut state = opened(&world, system);
        let mut frames = hover(start);
        frames.extend([vec![press(start, true)], vec![egui::Event::PointerMoved(to)]]);
        let held = run(&world, &missions, &mut state, frames);
        let dragging = state.is_dragging();
        let released = run(
            &world,
            &missions,
            &mut state,
            vec![vec![egui::Event::PointerMoved(to), press(to, false)]],
        );
        let drops: Vec<_> = held
            .actions
            .iter()
            .chain(&released.actions)
            .filter_map(|action| match action {
                DefensesWindowAction::DragSelection { items, point, .. } => {
                    Some((items.clone(), *point))
                }
                _ => None,
            })
            .collect();
        (dragging, drops, state.is_dragging())
    };

    let drag = |to| drag_from(at(100.0, 100.0), to);
    let outside = at(-60.0, 40.0);
    assert_eq!(drag(outside), (true, vec![(vec![DefensesItem::Character(hidden)], outside)], false));
    assert_eq!(drag(at(150.0, 100.0)), (true, vec![], false));
    // The distance is in list pixels: 5 above the press (25 > 24) drops,
    // 3.5 (12.25) does not, though both lie above the list (y 81).
    let near = at(100.0, 78.0);
    assert_eq!(
        drag_from(at(100.0, 83.0), near),
        (true, vec![(vec![DefensesItem::Character(hidden)], near)], false)
    );
    assert_eq!(
        drag_from(at(100.0, 83.0), at(100.0, 79.5)),
        (true, vec![], false)
    );
}

#[test]
fn a_drag_is_measured_and_dropped_at_the_release_events_own_point() {
    // FUN_006083c0 case 0x202 measures WM_LBUTTONUP's own point (lParam)
    // from the press: a release just outside the list drops there, and
    // a release where the press lay drops nothing, though the pointer
    // moves on within the release's frame.
    let (world, system, missions) = stocked();
    let start = at(100.0, 83.0);
    let release = |held_at: egui::Pos2, up_at: egui::Pos2, moved_on: egui::Pos2| {
        let mut state = opened(&world, system);
        let mut frames = hover(start);
        frames.extend([
            vec![press(start, true)],
            vec![egui::Event::PointerMoved(held_at)],
            vec![press(up_at, false), egui::Event::PointerMoved(moved_on)],
        ]);
        run(&world, &missions, &mut state, frames)
            .actions
            .into_iter()
            .filter_map(|action| match action {
                DefensesWindowAction::DragSelection { point, .. } => Some(point),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let near = at(100.0, 78.0);
    let outside = at(-60.0, 40.0);
    assert_eq!(release(near, near, outside), vec![near]);
    assert_eq!(release(start, start, outside), vec![]);
}

#[test]
fn a_click_on_a_row_frames_it_in_the_sides_color_and_a_new_tab_clears_it() {
    // FUN_0060bd00 paints the selected image, the mini with the side
    // frame keyed over it; +0xdc colours its text. FUN_004a90d0 refills
    // only for a different page.
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);

    let selected = run(&world, &missions, &mut state, click(at(100.0, 100.0)));
    assert!(selected.painted.contains(&(ROW_FRAME[0], at(77.0, 81.0))));
    assert_eq!(
        text(&selected, "Hidden").color,
        egui::Color32::from_rgb(255, 0, 0)
    );
    assert_eq!(
        selected.actions,
        [DefensesWindowAction::SelectSystem(system)]
    );
    let report = state
        .report(
            &world,
            &seen(system),
            &missions,
            idle(),
            &EconomyState::default(),
            system,
        )
        .unwrap();
    assert_eq!(report.selected, Some(1));
    assert_eq!(report.counts, [3, 1, 2, 2, 2]);

    // The same tab keeps the selection; another clears it.
    let _ = run(&world, &missions, &mut state, click(at(40.0, 30.0)));
    assert_eq!(
        state.windows[0].selected.single(),
        Some(DefensesItem::Character(
            world
                .characters
                .iter()
                .find(|(_, value)| value.name == "Hidden")
                .unwrap()
                .0
        ))
    );
    let regiments = run(&world, &missions, &mut state, click(at(80.0, 30.0)));
    assert_eq!(state.windows[0].page, DefensesPage::Regiments);
    assert!(state.windows[0].selected.is_empty());
    // The regiment page of the player's own system shows the garrison.
    // `+0x5c` at (2, 63), 228 by 17: centred both ways.
    let garrison = text(&regiments, "Garrison Requirement: 2");
    let (_, width) = crate::theme::gdi_pen_positions(4, "Garrison Requirement: 2").unwrap();
    assert_eq!(
        garrison.pos,
        at(2.0 + ((228.0 - width) / 2.0).floor(), 63.0 + 13.0)
    );
    assert!(regiments.painted.contains(&(10564, at(64.0, 20.0))));
    assert!(regiments.painted.contains(&(17_473, at(7.0, 81.0))));
}

/// Press at screen point `from`, move to `to` and hold; then release.
/// Returns the window's position while held and after the release.
fn move_window(
    world: &GameWorld,
    missions: &MissionState,
    state: &mut DefensesWindowState,
    from: egui::Pos2,
    to: egui::Pos2,
) -> ((i16, i16), (i16, i16)) {
    let mut frames = hover(from);
    frames.extend([
        vec![press(from, true)],
        vec![egui::Event::PointerMoved(to)],
        vec![egui::Event::PointerMoved(to)],
    ]);
    run(world, missions, state, frames);
    let held = state.windows[0].logical_position;
    run(world, missions, state, vec![vec![press(to, false)], vec![]]);
    (held, state.windows[0].logical_position)
}

// The window's constructor sets +0xb8 bit 4, so a left press on its own
// surface holds it (FUN_00606ee0); it follows the pointer unbounded
// (FUN_006071e0) and the drop puts it back inside the galaxy view
// (FUN_00607010). A press on the list goes to the list.
#[test]
fn the_window_moves_by_its_surface_and_drops_inside_the_galaxy_view() {
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let (held, dropped) = move_window(
        &world,
        &missions,
        &mut state,
        at(100.0, 10.0),
        at(130.0, 30.0),
    );
    assert_eq!((held, dropped), ((50, 50), (50, 50)));
    let from = at(130.0, 30.0);
    let to = from - egui::vec2(200.0, 0.0);
    let (held, dropped) = move_window(&world, &missions, &mut state, from, to);
    assert_eq!((held, dropped), ((-50, 50), (0, 50)));
    let list = egui::pos2(10.0 + 100.0 * 2.0, 20.0 + (50.0 + 200.0) * 2.0);
    let (held, dropped) = move_window(
        &world,
        &missions,
        &mut state,
        list,
        list + egui::vec2(60.0, 60.0),
    );
    assert_eq!((held, dropped), ((0, 50), (0, 50)));
}

#[test]
fn the_title_buttons_open_the_sector_minimize_and_close() {
    // 0xca opens the sector window (FUN_00429ce0); 0xc9 posts 0x466;
    // 200 closes.
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let sector = run(&world, &missions, &mut state, click(at(9.0, 9.0)));
    assert_eq!(
        sector.actions,
        [
            DefensesWindowAction::OpenSector(system),
            DefensesWindowAction::SelectSystem(system)
        ]
    );

    let minimized = run(&world, &missions, &mut state, click(at(210.0, 9.0)));
    assert_eq!(
        minimized.actions,
        [DefensesWindowAction::Minimize {
            system,
            logical_position: ORIGIN,
            page: DefensesPage::Personnel,
        }]
    );
    assert_eq!(state.window_count(), 0);

    let mut state = opened(&world, system);
    let closed = run(&world, &missions, &mut state, click(at(224.0, 9.0)));
    assert!(closed.actions.is_empty());
    assert_eq!(state.window_count(), 0);
}

#[test]
fn a_title_button_shows_its_pressed_art_only_while_the_button_is_held_over_it() {
    // FUN_004a8790's buttons swap to their pressed bitmaps under a held
    // pointer only.
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let hovered = run(&world, &missions, &mut state, hover(at(210.0, 9.0)));
    assert!(hovered.painted.contains(&(MINIMIZE_NORMAL, at(204.0, 3.0))));

    let mut frames = hover(at(224.0, 9.0));
    frames.push(vec![press(at(224.0, 9.0), true)]);
    let held = run(&world, &missions, &mut state, frames);
    assert!(held.painted.contains(&(CLOSE_PRESSED, at(218.0, 3.0))));
    assert!(held.painted.contains(&(MINIMIZE_NORMAL, at(204.0, 3.0))));
    assert_eq!(state.window_count(), 1);
}

fn right_click(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
    let button = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let mut frames = hover(point);
    frames.extend([vec![button(true)], vec![button(false)], vec![]]);
    frames
}

fn menus(run: &Run) -> Vec<(Option<MenuObject>, (i16, i16))> {
    run.actions
        .iter()
        .filter_map(|action| match *action {
            DefensesWindowAction::OpenObjectMenu {
                selection, point, ..
            } => Some((selection, point)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_right_click_on_a_character_selects_it_and_opens_its_menu_at_the_cursor() {
    // FUN_004ac5c0 (the class's slot 7) opens the menu for slot 22's
    // selection (FUN_004a7a20), the list's selected rows, at the
    // release point in 640 by 480 canvas pixels.
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let hidden = world
        .characters
        .iter()
        .find(|(_, value)| value.name == "Hidden")
        .unwrap()
        .0;

    let opened = run(&world, &missions, &mut state, right_click(at(100.0, 100.0)));
    assert_eq!(
        state.windows[0].selected.single(),
        Some(DefensesItem::Character(hidden))
    );
    assert_eq!(
        menus(&opened),
        [(Some(MenuObject::Character(hidden)), (120, 130))]
    );
}

#[test]
fn a_menu_on_one_of_two_selected_squadrons_of_a_class_takes_both() {
    // FUN_004ac5c0 takes slot 22's whole selection (FUN_004a7a20): two
    // X-wing squadrons selected with Ctrl are two team members, though
    // both stand for the planet's X-wing entry.
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    state.windows[0].page = DefensesPage::Squadrons;
    let _ = run(&world, &missions, &mut state, click(at(30.0, 100.0)));
    let _ = run(
        &world,
        &missions,
        &mut state,
        click_with(at(100.0, 100.0), egui::Modifiers::CTRL),
    );
    assert_eq!(state.windows[0].selected.items().len(), 2);
    let opened = run(&world, &missions, &mut state, right_click(at(30.0, 100.0)));
    let teams: Vec<Vec<MenuObject>> = opened
        .actions
        .iter()
        .filter_map(|action| match action {
            DefensesWindowAction::OpenObjectMenu { team, .. } => Some(team.clone()),
            _ => None,
        })
        .collect();
    let [team] = &teams[..] else {
        panic!("{teams:?}");
    };
    assert_eq!(team.len(), 2, "{team:?}");
    assert_eq!(team[0], team[1]);
    assert!(matches!(team[0], MenuObject::PlanetFighter { .. }));
}

#[test]
fn a_right_click_on_a_regiment_opens_its_menu() {
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let _ = run(&world, &missions, &mut state, click(at(80.0, 30.0)));
    let troop = world.systems[system].ground_units[0];
    let opened = run(&world, &missions, &mut state, right_click(at(30.0, 100.0)));
    assert_eq!(
        menus(&opened),
        [(Some(MenuObject::Troop(troop)), (50, 130))]
    );
}

#[test]
fn a_right_click_on_a_defense_facility_opens_its_menu() {
    // FUN_004ac5c0 opens the pop-up for slot 22's selection
    // (FUN_004a7a20); FUN_0051d990 gives it Encyclopedia and Status.
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let _ = run(&world, &missions, &mut state, click(at(150.0, 30.0)));
    assert_eq!(state.windows[0].page, DefensesPage::Shields);
    let opened = run(&world, &missions, &mut state, right_click(at(30.0, 100.0)));
    let Some(DefensesItem::Defense(shield)) = state.windows[0].selected.single() else {
        panic!("the right press selects the shield");
    };
    assert_eq!(
        menus(&opened),
        [(Some(MenuObject::DefenseFacility(shield)), (50, 130))]
    );
}

#[test]
fn a_right_click_on_empty_list_space_clears_the_selection_and_opens_an_empty_menu() {
    // FUN_006094b0 clears the selection on a press over no item; the
    // release then opens the menu for nothing (Encyclopedia and Status,
    // both disabled).
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let _ = run(&world, &missions, &mut state, click(at(100.0, 100.0)));
    assert!(!state.windows[0].selected.is_empty());

    let empty = run(&world, &missions, &mut state, right_click(at(100.0, 250.0)));
    assert!(state.windows[0].selected.is_empty());
    assert_eq!(menus(&empty), [(None, (120, 280))]);

    // Above the list, a right click opens nothing.
    let outside = run(&world, &missions, &mut state, right_click(at(100.0, 60.0)));
    assert!(menus(&outside).is_empty());
}

#[test]
fn showing_a_page_clears_the_selection_only_when_the_page_changes() {
    // FUN_004a90d0 refills only for a different page, as a tab click.
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    let _ = run(&world, &missions, &mut state, click(at(100.0, 100.0)));
    assert!(state.show_page(system, DefensesPage::Personnel));
    assert!(!state.windows[0].selected.is_empty());
    assert!(state.show_page(system, DefensesPage::Regiments));
    assert_eq!(state.windows[0].page, DefensesPage::Regiments);
    assert!(state.windows[0].selected.is_empty());

    let mut other = world.clone();
    let missing = other.systems.insert(world.systems[system].clone());
    assert!(!state.show_page(missing, DefensesPage::Shields));
    assert_eq!(
        state.screen_rect(scaled(), system),
        Some(egui::Rect::from_min_size(
            at(0.0, 0.0),
            egui::vec2(470.0, 608.0)
        ))
    );
    assert_eq!(state.screen_rect(scaled(), missing), None);
}

/// The Alliance personnel `stocked` lists with `extras` more
/// characters, and the window open on them.
fn crowded(extras: usize) -> (GameWorld, SystemKey, MissionState, Vec<DefensesItem>) {
    let (mut world, system, missions) = stocked();
    for index in 0..extras {
        add_character(&mut world, system, &format!("Extra {index}"), true);
    }
    let items = page_rows(
        &world,
        &seen(system),
        &missions,
        idle(),
        Faction::Alliance,
        system,
        DefensesPage::Personnel,
    )
    .into_iter()
    .map(|row| row.item)
    .collect();
    (world, system, missions, items)
}

#[test]
fn a_page_past_three_rows_shows_the_list_bar_at_the_lists_right_edge() {
    // FUN_004a8790 gives the list (7, 81, 222 by 210) the bar of RCDATA
    // 10700 (FUN_0060a490 with 0x29cc): 10 wide at x 219, its 13-wide
    // arrows one column left (FUN_0060fa80), the thumb from the track's
    // top (FUN_0060fca0). A0 2026-11-11-icon-menu-topic/05 shows it so
    // for Coruscant's ten squadrons.
    let (world, system, missions, items) = crowded(9);
    // Eleven characters, then the special force.
    assert_eq!(items.len(), 12);
    assert!(matches!(items[11], DefensesItem::SpecialForce(_)));
    let mut state = opened(&world, system);
    let drawn = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    for (id, x, y) in [
        (LIST_BAR_ART.up[0], 218.0, 81.0),
        (LIST_BAR_ART.down[0], 218.0, 282.0),
        (LIST_BAR_ART.track, 219.0, 90.0),
        (LIST_BAR_ART.top, 219.0, 90.0),
    ] {
        assert!(
            drawn.painted.contains(&(id, at(x, y))),
            "{id} at ({x}, {y})"
        );
    }
    // Three full rows show; the fourth starts below the list.
    let minis = drawn
        .painted
        .iter()
        .filter(|(id, _)| *id == 18_176 + 832)
        .count();
    assert_eq!(minis, 9);
    // Two lines fit: no bar (FUN_0060b5d0).
    let (world, system, missions, items) = crowded(3);
    assert_eq!(items.len(), 6);
    let mut state = opened(&world, system);
    let drawn = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    assert!(!drawn
        .painted
        .iter()
        .any(|(id, _)| *id == LIST_BAR_ART.track));
}

#[test]
fn the_list_bar_scrolls_the_last_cells_into_view_and_onto_their_targets() {
    // Twelve rows take four lines, 280 pixels: the bar scrolls 280 + 5 -
    // 210 = 75 (FUN_00609ae0). Its down arrow steps 10 (FUN_004a8790's
    // step); a press below the thumb pages by the list's height, to the
    // end. A0 2026-11-11-icon-menu-topic/06: scrolled to the end, the
    // second line shows from y 81 - 5 and the last below it.
    let (world, system, missions, items) = crowded(9);
    let mut state = opened(&world, system);
    run(&world, &missions, &mut state, click(at(224.0, 286.0)));
    assert_eq!(state.windows[0].scroll, 10.0);
    run(&world, &missions, &mut state, click(at(224.0, 270.0)));
    assert_eq!(state.windows[0].scroll, 75.0);
    let drawn = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    // The thumb's top: 81 + 9 + (192 - 141) at the end.
    assert!(drawn
        .painted
        .contains(&(LIST_BAR_ART.top, at(219.0, 141.0))));
    // The twelfth cell, the last line's third, at (147, 81 + 210 - 75).
    let last = state.cell_screen_rect(scaled(), system, 11).unwrap();
    assert_eq!(last.min, at(147.0, 216.0));
    let minis = drawn
        .painted
        .iter()
        .filter(|(id, _)| *id == 18_176 + 832)
        .count();
    // The first line has gone; the other three show, part of the
    // second's cut by the list's top: the fourth to the eleventh
    // characters, then the special force.
    assert_eq!(minis, 8);
    // A mission released on it targets it; one on the bar, nothing.
    let fog = seen(system);
    let layer = egui::LayerId::new(egui::Order::Foreground, area_id(system));
    let pick = |point| {
        state
            .mission_target(&world, &fog, &missions, idle(), scaled(), layer, point)
            .unwrap()
    };
    let DefensesItem::Character(character) = items[10] else {
        panic!("{:?}", items[10]);
    };
    let eleventh = state.cell_screen_rect(scaled(), system, 10).unwrap();
    assert_eq!(pick(eleventh.center()).character, Some(character));
    assert_eq!(pick(at(224.0, 230.0)), MissionTargetPick::from(system));
    // A click on it selects it, and the list keeps its place.
    run(&world, &missions, &mut state, click(eleventh.center()));
    assert_eq!(state.windows[0].selected.single(), Some(items[10]));
    assert_eq!(state.windows[0].scroll, 75.0);
    // The bar is a child window over the list (FUN_0060fa80): a press
    // on its up arrow scrolls and leaves the list's selection alone
    // (FUN_006094b0 clears it only for the list's own empty space).
    run(&world, &missions, &mut state, click(at(224.0, 85.0)));
    assert_eq!(state.windows[0].scroll, 65.0);
    assert_eq!(state.windows[0].selected.single(), Some(items[10]));
    // A right release on the bar opens no menu for the list.
    let menus = run(&world, &missions, &mut state, right_click(at(224.0, 230.0)));
    assert!(
        !menus
            .actions
            .iter()
            .any(|action| matches!(action, DefensesWindowAction::OpenObjectMenu { .. })),
        "{:?}",
        menus.actions
    );
}

#[test]
fn a_new_page_keeps_the_lists_place_within_its_range() {
    // FUN_004a90d0 empties and lays out the list (FUN_005f5b20,
    // FUN_0060a280); FUN_00609ae0 only clamps +0xc4, to 0 when the
    // page fits.
    let (world, system, missions, _) = crowded(9);
    let mut state = opened(&world, system);
    state.windows[0].scroll = 40.0;
    run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    assert_eq!(state.windows[0].scroll, 40.0);
    assert!(state.show_page(system, DefensesPage::Regiments));
    run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    assert_eq!(state.windows[0].scroll, 0.0);
}

// A0 garrison-clean/test-day483-yavin-regiments: each title button's
// key pixel, its bottom-left corner, shows the window's background
// 10577 (black at (3, 16), (204, 16), (218, 16)): the buttons are
// children blitted keyed (FUN_00602d30 -> FUN_005fd0f0).
#[test]
fn the_title_buttons_key_pixels_show_the_windows_background() {
    let (world, system, missions) = stocked();
    let mut state = opened(&world, system);
    crate::fleet_window::tests::KEY_PIXELS.with(|pixels| pixels.borrow_mut().clear());
    let _ = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    let pixels = crate::fleet_window::tests::KEY_PIXELS.with(|pixels| pixels.take());
    for x in [3.0, 204.0, 218.0] {
        assert!(
            pixels.contains(&(KeyPixel::Background(BACKGROUND), at(x, 16.0))),
            "{x} in {pixels:?}"
        );
    }
}

// FUN_00606c90 hands each title button FUN_006073d0's window surface
// +0xa4, which FUN_006075b0 set to 10577 (FUN_004a8790); FUN_00602d30
// copies the backdrop from it at the button's +0x10c/+0x110 and keys the
// art over it (FUN_005fd0f0). The title strip is blitted to the window's
// DC only (FUN_004a9800 by side and focus), never into +0xa4, so the key
// pixel ignores both. A0 coruscant-676/14-after-movie: the Empire's
// view of the Alliance-held Coruscant shows black at (3, 16), (204, 16)
// and (218, 16) under the red strip.
#[test]
fn the_title_buttons_key_pixels_show_the_background_whatever_the_strips_side_and_focus() {
    let (world, system) = world(ControlKind::Controlled(Faction::Empire));
    let missions = MissionState::new();
    let mut state = opened(&world, system);
    state.set_holds_focus(false);
    crate::fleet_window::tests::KEY_PIXELS.with(|pixels| pixels.borrow_mut().clear());
    let run = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    let pixels = crate::fleet_window::tests::KEY_PIXELS.with(|pixels| pixels.take());
    // The other side's strip, unfocused: side 2's 10302.
    assert!(
        run.painted.contains(&(10302, at(2.0, 2.0))),
        "{:?}",
        run.painted
    );
    for x in [3.0, 204.0, 218.0] {
        assert!(
            pixels.contains(&(KeyPixel::Background(BACKGROUND), at(x, 16.0))),
            "{x} in {pixels:?}"
        );
    }
}
