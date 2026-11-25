use super::*;
use rebellion_render::object_menu::{MenuObject, OrderGates};

/// The original galaxy as [`place_new_game`] leaves it for `scenario`,
/// played by `faction`.
fn new_game(scenario: Scenario, faction: CockpitFaction) -> GameWorld {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
    // The browser's seed (rebellion-data seeds.rs default_seed on wasm).
    let options = rebellion_core::world::SeedOptions {
        rng_seed: Some(0),
        ..Default::default()
    };
    let mut world = rebellion_data::load_game_data_with_options(&data, &options)
        .expect("load original game data");
    let mut movement = MovementState::default();
    place_new_game(
        request(scenario, faction),
        &mut world,
        &mut movement,
        &TroopTransportState::default(),
        &MissionState::default(),
    );
    world
}

fn named_system(world: &GameWorld, name: &str) -> SystemKey {
    world
        .systems
        .iter()
        .find(|(_, value)| value.name == name)
        .map(|(key, _)| key)
        .unwrap_or_else(|| panic!("{name} is in SYSTEMSD"))
}

fn shown(world: &GameWorld, side: Faction, name: &str) -> rebellion_core::world::System {
    rebellion_render::viewed_contents::apparent_system(world, side, named_system(world, name))
        .expect("the system is shown")
        .into_owned()
}

// A0 2026-11-19-fog-13 (an unedited new Empire game): e-02 (sha
// d9021b96bd53) shows every Rim system's unexplored cross but Yavin's
// smallest red (Alliance) mark; e-12 (3c48fff4112b) shows Yavin red with
// its icons and tracks, Sumitra's nine others bare. FUN_0051b990's
// level-10 push of Yavin (mode 1) syncs its explored bit.
#[test]
#[ignore = "requires original data/base DAT files"]
fn the_new_game_fixture_shows_the_empire_yavin_explored_and_held_by_the_alliance() {
    let world = new_game(Scenario::NewGame, CockpitFaction::Empire);
    let yavin = shown(&world, Faction::Empire, "Yavin");
    assert_eq!(yavin.exploration_status, ExplorationStatus::Explored);
    assert!(yavin.control.is_controlled_by(Faction::Alliance));
    for rim in ["Kabal", "Triton", "Linuri", "Tierfon"] {
        assert_eq!(
            shown(&world, Faction::Empire, rim).exploration_status,
            ExplorationStatus::Unexplored,
            "{rim}"
        );
    }
    // fog-14 a-01: to the Alliance every Rim system but its own is gray.
    let world = new_game(Scenario::NewGame, CockpitFaction::Alliance);
    assert_eq!(
        shown(&world, Faction::Alliance, "Kabal").exploration_status,
        ExplorationStatus::Unexplored
    );
}

/// `name`'s Defenses window as `faction` sees it in `world`: each
/// page's report (`FUN_004a9ce0` tabs, `FUN_004a90d0` rows and the
/// garrison line).
fn defenses_pages(
    world: &GameWorld,
    faction: CockpitFaction,
    name: &str,
) -> Vec<rebellion_render::defenses_window::DefensesWindowReport> {
    let system = named_system(world, name);
    let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
    let mut defenses = DefensesWindowState::default();
    assert!(defenses.open(world, system, (100, 100), faction, layout));
    let manufacturing = ManufacturingState::default();
    let deliveries = rebellion_core::delivery::DeliveryState::default();
    let fog = FogState::new(match faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    });
    let production = rebellion_render::defenses_window::DefensesProduction {
        manufacturing: &manufacturing,
        deliveries: &deliveries,
        regiments: &[],
        squadrons: &[],
    };
    DefensesPage::ALL
        .into_iter()
        .map(|page| {
            defenses.show_page(system, page);
            defenses
                .report(
                    world,
                    &fog,
                    &MissionState::default(),
                    production,
                    &EconomyState::default(),
                    system,
                )
                .expect("the window is open")
        })
        .collect()
}

// A0 2026-11-21-gid-units-17 (unedited new games): the Alliance's view
// of the Empire's core Bortras (a-25..a-27: 0b64b21d6a2b, 7f74bd9ba3fd,
// 31ac26bcca76) lists no personnel, regiments or squadrons and no
// Garrison Requirement line; the Empire's view of the Alliance's core
// Corfai (e-39..e-41: 6db0cb39783c, da5a2b9cf843, 07b95e61a662) the
// same. FUN_004a25c0 -> FUN_004f3220 reads the viewer's copy, which the
// setup push (FUN_0051b990) gives facilities but no units; FUN_004a90d0
// writes TEXTSTRA 6471 only when the copy's side is the viewer's.
// The browser seed's Corsin (Sesswenna) stands for Corfai: since the
// seed rolls every system's resources (471b20e1) its Corfai holds no
// units.
#[test]
#[ignore = "requires original data/base DAT files"]
fn the_new_game_fixture_s_enemy_core_defenses_windows_list_no_units_or_garrison() {
    for (faction, holder, name) in [
        (CockpitFaction::Alliance, 2, "Coruscant"),
        (CockpitFaction::Alliance, 2, "Commenor"),
        (CockpitFaction::Empire, 1, "Corsin"),
    ] {
        let world = new_game(Scenario::NewGame, faction);
        let live = &world.systems[named_system(&world, name)];
        assert!(
            live.ground_units.len() + live.fighters.len() > 0,
            "{name} holds units the copy must not show"
        );
        for report in defenses_pages(&world, faction, name) {
            assert_eq!(report.side, holder, "{name}");
            assert_eq!(report.garrison, None, "{name} {:?}", report.page);
            // Personnel, Trooper Regiments, Fighter Squadrons.
            assert_eq!(report.counts[..3], [0, 0, 0], "{name} {:?}", report.page);
        }
    }
}

// A0 2026-11-21-gid-units-17 e-44..e-46 (ab66aca7ac37, 2f530d9261ee,
// 4686560f93a4): the Empire's Yavin lists Leia Organa, Luke Skywalker,
// Han Solo, Wedge Antilles, Chewbacca and Jan Dodonna, two Alliance Army
// Regiments, an X-wing and a Y-wing: the setup's level-10 push
// (Event319_Handle(10, 2), FUN_0050d5a0 case 10: every child 0x01..0xff
// of the system, its fleets and their ships, synced in mode 1).
#[test]
#[ignore = "requires original data/base DAT files"]
fn the_new_game_fixture_s_empire_copy_of_yavin_lists_its_heroes_regiments_and_squadrons() {
    let world = new_game(Scenario::NewGame, CockpitFaction::Empire);
    let pages = defenses_pages(&world, CockpitFaction::Empire, "Yavin");
    let page = |page| {
        pages
            .iter()
            .find(|report| report.page == page)
            .expect("each page")
    };
    assert!(pages.iter().all(|report| report.side == 1));
    let personnel = &page(DefensesPage::Personnel).rows;
    for hero in [
        "Leia Organa",
        "Luke Skywalker",
        "Han Solo",
        "Wedge Antilles",
        "Chewbacca",
        "Jan Dodonna",
    ] {
        assert!(
            personnel.iter().any(|row| row == hero),
            "{hero}: {personnel:?}"
        );
    }
    // A0's two regiments and two squadrons are Yavin's fixed table
    // (FUN_0051acb0); the army seeds (FUN_0051b1f0) may add a planet group
    // there, so the copy lists what the system holds, A0's units among them.
    let yavin = &world.systems[named_system(&world, "Yavin")];
    let regiments = page(DefensesPage::Regiments).rows.len();
    assert_eq!(regiments, yavin.ground_units.len());
    assert!(regiments >= 2, "{regiments}");
    assert_eq!(page(DefensesPage::Regiments).garrison, None);
    let squadrons = &page(DefensesPage::Squadrons).rows;
    let held: Vec<_> = yavin
        .fighters
        .iter()
        .flat_map(|entry| {
            std::iter::repeat_n(world.fighter_classes[entry.class].name.clone(), entry.count as usize)
        })
        .collect();
    let mut sorted = squadrons.clone();
    sorted.sort();
    let mut held_sorted = held.clone();
    held_sorted.sort();
    assert_eq!(sorted, held_sorted, "{squadrons:?}");
    for name in ["X-wing", "Y-wing"] {
        assert!(squadrons.iter().any(|row| row == name), "{name}: {squadrons:?}");
    }
    assert!(page(DefensesPage::Shields).rows.is_empty());
    assert!(page(DefensesPage::Batteries).rows.is_empty());
}

// A0 2026-11-21-gid-units-17 a-23 (07807eddd3bf, the Alliance's
// Sesswenna on day 3): the Empire's core Coruscant, which holds
// regiments, squadrons and the Emperor but no defense facility (e-11..
// e-15), shows no defenses icon; Bortras, which holds a GenCore Level I
// (a-28), shows the Empire's. FUN_0045ce80 counts the viewer's copy
// (FUN_004f3220), and the setup push (FUN_0051b990) gives a core copy
// its facilities (0x20..0x2f) but no units. The browser seed's Vagran
// holds regiments and no defense facility, its Commenor and Coruscant
// defense facilities (fog-a0.mjs compares their icons with a-23). Until
// the seed rolled every system's resources (471b20e1) Commenor and
// Drall were the first case and Coruscant held one facility.
#[test]
#[ignore = "requires original data/base DAT files"]
fn the_new_game_fixture_lights_an_enemy_core_defenses_icon_by_its_facilities_alone() {
    let world = new_game(Scenario::NewGame, CockpitFaction::Alliance);
    let side = |name| {
        rebellion_render::quadrant_icons::quadrant_side(
            &world,
            &FogState::new(Faction::Alliance),
            &MissionState::default(),
            &MovementState::default(),
            rebellion_render::quadrant_icons::Travellers {
                regiments: &[],
                squadrons: &[],
            },
            Faction::Alliance,
            named_system(&world, name),
            rebellion_render::quadrant_icons::Quadrant::Defenses,
        )
    };
    let vagran = &world.systems[named_system(&world, "Vagran")];
    assert!(vagran.control.is_controlled_by(Faction::Empire));
    assert!(!vagran.ground_units.is_empty() && vagran.defense_facilities.is_empty());
    assert_eq!(side("Vagran"), None);
    for (name, facilities) in [("Commenor", 2), ("Coruscant", 2)] {
        let live = &world.systems[named_system(&world, name)];
        assert!(live.control.is_controlled_by(Faction::Empire), "{name}");
        assert_eq!(live.defense_facilities.len(), facilities, "{name}");
        assert_eq!(side(name), Some(2), "{name}");
    }
}

// A0 2026-11-21-gid-units-17: the Empire's Drall holds no squadron on
// its planet (e-31, a05fee9087eb) and one TIE Fighter aboard Fleet 2
// (e-26, acd453b96a53), and takes the smallest Fighter Squadrons mark
// (e-02): FUN_00503550 walks the copy's direct children, and a fleet's
// are its ships. Whether a planet group lands there is the army seeds'
// draw (FUN_0051b1f0, a uniform pick among the side's systems; seed 0
// lands one, so gid-a0.mjs allows a larger mark there); a squadron the
// seeds put in a fleet rides one of its ships, as A0's TIE Fighter does.
#[test]
#[ignore = "requires original data/base DAT files"]
fn the_new_game_fixture_s_drall_is_the_empire_s_and_its_fleets_squadrons_ride_their_ships() {
    let world = new_game(Scenario::NewGame, CockpitFaction::Empire);
    let drall = named_system(&world, "Drall");
    assert!(world.systems[drall]
        .control
        .is_controlled_by(Faction::Empire));
    assert!(world.systems[drall]
        .fighters
        .iter()
        .all(|entry| entry.carrier.is_none()));
    for fleet in world.fleets.values().filter(|fleet| fleet.location == drall) {
        for entry in &fleet.fighters {
            let carrier = entry.carrier.expect("a squadron aboard a ship");
            assert!(fleet.capital_ships.iter().any(|ship| ship.id == carrier));
        }
    }
}

// A0 2026-11-19-fog-14 e-05 (ed937647a151): Triton bare while the
// fleet is in hyperspace; e-06 (5048b9273d8b): its mark and tracks on
// arrival; e-13 (b2ef05cac7f5), e-28 (63ba3e75ac9b): the day-44 tracks
// kept after the fleet left.
#[test]
#[ignore = "requires original data/base DAT files"]
fn the_triton_fixtures_show_it_bare_in_flight_live_on_arrival_and_stale_after() {
    let enroute = new_game(Scenario::TritonEnroute, CockpitFaction::Empire);
    assert_eq!(
        shown(&enroute, Faction::Empire, "Triton").exploration_status,
        ExplorationStatus::Unexplored
    );
    let arrived = new_game(Scenario::TritonArrived, CockpitFaction::Empire);
    let live = shown(&arrived, Faction::Empire, "Triton");
    assert_eq!(live.exploration_status, ExplorationStatus::Explored);
    let stale_world = new_game(Scenario::TritonStale, CockpitFaction::Empire);
    let stale = shown(&stale_world, Faction::Empire, "Triton");
    let own = &stale_world.systems[named_system(&stale_world, "Triton")];
    assert_eq!(stale.exploration_status, ExplorationStatus::Explored);
    assert_eq!(
        (stale.total_energy, stale.raw_materials),
        (live.total_energy, live.raw_materials)
    );
    assert_ne!(
        (own.total_energy, own.raw_materials),
        (live.total_energy, live.raw_materials)
    );
    assert_eq!(
        shown(&stale_world, Faction::Empire, "Kabal").exploration_status,
        ExplorationStatus::Explored
    );
}

// A0 personnel-states/15, 17, 18 (Alliance, the Troop Finder's
// Display): Pzob's Defenses window at (118, 86), Mon Calamari's and
// Umgul's at (305, 86). FUN_0045c8e0 opens at the icon's top left,
// clamped into the galaxy view.
#[test]
#[ignore = "requires original data/base DAT files"]
fn a_finders_defenses_window_opens_where_a0_puts_it() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
    let world = rebellion_data::load_game_data(&data).expect("load original game data");
    let layout = CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
    for (name, a0) in [
        ("Pzob", (118.0, 86.0)),
        ("Mon Calamari", (305.0, 86.0)),
        ("Umgul", (305.0, 86.0)),
    ] {
        let key = world
            .systems
            .iter()
            .find(|(_, s)| s.name == name)
            .map(|(k, _)| k)
            .unwrap();
        let mut sectors = SectorWindowState::default();
        sectors.open_for_system(&world, key, CockpitFaction::Alliance);
        let point = sectors
            .quadrant_window_point(&world, layout, key, Quadrant::Defenses)
            .unwrap();
        let mut defenses = DefensesWindowState::default();
        defenses.open(&world, key, point, CockpitFaction::Alliance, layout);
        let origin = defenses.screen_rect(layout, key).unwrap().min;
        assert_eq!((origin.x, origin.y), a0, "{name}");
    }
}

// A0 status/03-ship: Corellian Corvette 1 in Fleet 1, undamaged. Its
// first page reads Awaiting Orders, Maintenance Cost 23, no capacity,
// nothing embarked, Ship Damaged No, Hyperdrive 80:80, Hull 500:500
// (FUN_00446fd0's rows over CAPSHPSD's Corellian Corvette).
#[test]
#[ignore = "requires original data/base DAT files"]
fn an_undamaged_corellian_corvettes_status_reads_as_a0() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
    let mut world = rebellion_data::load_game_data(&data).expect("load original game data");
    let (class, hull) = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| class.name == "Corellian Corvette")
        .map(|(key, class)| (key, class.hull))
        .expect("CAPSHPSD lists the Corellian Corvette");
    let here = world.systems.keys().next().unwrap();
    let fleet = world.fleets.insert(rebellion_core::world::Fleet {
        location: here,
        capital_ships: vec![ShipInstance::new(class, hull as i32, true)],
        is_alliance: true,
        ..Default::default()
    });
    world.rename_fleet(fleet, "Fleet 1");
    let missions = MissionState::default();
    let movement = MovementState::default();
    let transport = TroopTransportState::default();
    let manufacturing = ManufacturingState::default();
    let sources = rebellion_render::status_rows::StatusSources {
        world: &world,
        missions: &missions,
        movement: &movement,
        transport: &transport,
        manufacturing: &manufacturing,
        today: 180,
        objectives: rebellion_render::status_rows::ObjectiveSource {
            alliance: true,
            conditions_word: 0,
            standard: true,
        },
    };
    let rows: Vec<(String, String)> = rebellion_render::status_rows::ship_rows(sources, fleet, 0)
        .expect("rows")
        .into_iter()
        .map(|row| (row.label, row.value))
        .take(14)
        .collect();
    let a0: Vec<(String, String)> = [
        ("Class:", "Corellian Corvette"),
        ("Fleet:", "Fleet 1"),
        ("Status:", "Awaiting Orders"),
        ("Maintenance Cost:", "23"),
        ("Capacity:", ""),
        ("Fighter Squadrons:", "0"),
        ("Trooper Regiments:", "0"),
        ("Embarked:", ""),
        ("Fighter Squadrons:", "0"),
        ("Trooper Regiments:", "0"),
        ("Personnel:", "0"),
        ("Ship Damaged:", "No"),
        ("Hyperdrive Rating:", "80:80"),
        ("Hull Value:", "500:500"),
    ]
    .into_iter()
    .map(|(label, value)| (label.to_owned(), value.to_owned()))
    .collect();
    assert_eq!(rows, a0);
}

// A0 fleet2/05-medium-transport-1-status (day 714, after the battle at
// Umgul): Medium Transport 1 in Fleet 1, two regiments aboard, Ship
// Damaged Yes, Hyperdrive Rating 0:100 (both hyperdrives gone) and
// Hull Value 317:360 (FUN_00446fd0's rows; ship_condition's
// FUN_00500f60, FUN_00501780, FUN_00501820).
#[test]
#[ignore = "requires original data/base DAT files"]
fn a_damaged_medium_transports_status_reads_as_a0() {
    use rebellion_core::world::Subsystem;
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
    let mut world = rebellion_data::load_game_data(&data).expect("load original game data");
    let class = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| class.name == "Medium Transport")
        .map(|(key, _)| key)
        .expect("CAPSHPSD lists the Medium Transport");
    let here = world.systems.keys().next().unwrap();
    let mut ship = ShipInstance::new(class, 317, true);
    ship.set_hits(Subsystem::PrimaryHyperdrive, 1);
    ship.set_hits(Subsystem::BackupHyperdrive, 1);
    let fleet = world.fleets.insert(rebellion_core::world::Fleet {
        location: here,
        capital_ships: vec![ship],
        is_alliance: true,
        ..Default::default()
    });
    world.rename_fleet(fleet, "Fleet 1");
    let regiment_class = world
        .buildable_classes
        .keys()
        .copied()
        .find(|id| id.family() == 0x10)
        .expect("TROOPSD lists a regiment");
    let regiments: Vec<_> = (0..2)
        .map(|_| {
            let key = world.troops.insert(rebellion_core::world::TroopUnit {
                class_dat_id: regiment_class,
                is_alliance: true,
                regiment_strength: 1,
            });
            world.systems[here].ground_units.push(key);
            key
        })
        .collect();
    let mut transport = TroopTransportState::default();
    transport
        .embark(&mut world, fleet, &regiments)
        .expect("the transport carries two regiments");
    let missions = MissionState::default();
    let movement = MovementState::default();
    let manufacturing = ManufacturingState::default();
    let sources = rebellion_render::status_rows::StatusSources {
        world: &world,
        missions: &missions,
        movement: &movement,
        transport: &transport,
        manufacturing: &manufacturing,
        today: 714,
        objectives: rebellion_render::status_rows::ObjectiveSource {
            alliance: true,
            conditions_word: 0,
            standard: true,
        },
    };
    let rows: Vec<(String, String)> = rebellion_render::status_rows::ship_rows(sources, fleet, 0)
        .expect("rows")
        .into_iter()
        .map(|row| (row.label, row.value))
        .take(14)
        .collect();
    let a0: Vec<(String, String)> = [
        ("Class:", "Medium Transport"),
        ("Fleet:", "Fleet 1"),
        ("Status:", "Awaiting Orders"),
        ("Maintenance Cost:", "12"),
        ("Capacity:", ""),
        ("Fighter Squadrons:", "0"),
        ("Trooper Regiments:", "2"),
        ("Embarked:", ""),
        ("Fighter Squadrons:", "0"),
        ("Trooper Regiments:", "2"),
        ("Personnel:", "0"),
        ("Ship Damaged:", "Yes"),
        ("Hyperdrive Rating:", "0:100"),
        ("Hull Value:", "317:360"),
    ]
    .into_iter()
    .map(|(label, value)| (label.to_owned(), value.to_owned()))
    .collect();
    assert_eq!(rows, a0);

    // A0 fleet2/06-corellian-corvette-1-status (day 728): a hull hit
    // alone reads Ship Damaged Yes, Hyperdrive 80:80, Hull 240:500.
    let corvette = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| class.name == "Corellian Corvette")
        .map(|(key, _)| key)
        .expect("CAPSHPSD lists the Corellian Corvette");
    world.fleets[fleet]
        .capital_ships
        .push(ShipInstance::new(corvette, 240, true));
    let sources = rebellion_render::status_rows::StatusSources {
        world: &world,
        missions: &missions,
        movement: &movement,
        transport: &transport,
        manufacturing: &manufacturing,
        today: 728,
        objectives: rebellion_render::status_rows::ObjectiveSource {
            alliance: true,
            conditions_word: 0,
            standard: true,
        },
    };
    let rows: Vec<(String, String)> = rebellion_render::status_rows::ship_rows(sources, fleet, 1)
        .expect("rows")
        .into_iter()
        .map(|row| (row.label, row.value))
        .skip(11)
        .take(3)
        .collect();
    assert_eq!(
        rows,
        [
            ("Ship Damaged:".to_owned(), "Yes".to_owned()),
            ("Hyperdrive Rating:".to_owned(), "80:80".to_owned()),
            ("Hull Value:".to_owned(), "240:500".to_owned()),
        ]
    );
}

#[test]
fn scenario_table_is_stable_and_complete() {
    for value in 0..SCENARIO_COUNT {
        assert_eq!(Scenario::decode(value).unwrap() as u8, value);
    }
    assert!(Scenario::decode(SCENARIO_COUNT).is_none());
}

/// One system held by `side` with one recruited character of that side
/// on it, and the shipped MISSNSD Diplomacy record `0x51000010`.
fn diplomacy_world(side: Faction) -> GameWorld {
    use rebellion_core::world::{
        Character, MissionMemberRules, MissionRecord, MissionTargetRules, SkillPair, System,
    };
    let mut world = GameWorld::default();
    let here = world.systems.insert(System {
        dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
        name: "System".into(),
        exploration_status: ExplorationStatus::Explored,
        popularity_alliance: 0.5,
        popularity_empire: 0.5,
        is_populated: true,
        control: ControlKind::Controlled(side),
        ..Default::default()
    });
    world.characters.insert(Character {
        name: "Agent".into(),
        is_alliance: side == Faction::Alliance,
        is_empire: side == Faction::Empire,
        loyalty: SkillPair {
            base: 100,
            variance: 0,
        },
        current_system: Some(here),
        recruited: true,
        ..Default::default()
    });
    world.mission_records = vec![MissionRecord {
        dat_id: rebellion_core::ids::DatId::new(0x5100_0010),
        timer_min_days: 5,
        timer_spread_days: 10,
        repeats: true,
        hidden: false,
        detection_phases: true,
        can_resign: true,
        // MISSNSD column 10 of the shipped Diplomacy record.
        leave_rule: 1,
        rules: MissionTargetRules {
            container_loss_ends: true,
            explored_container: true,
            needs_populated_container: true,
            target_loss_ends: true,
            own_side_target: true,
            other_side_target: true,
            opponent_target: false,
            revolting_target: false,
            calm_target: true,
            prisoner_target: false,
            free_target: false,
        },
        members: MissionMemberRules {
            alliance: true,
            empire: true,
            special_force_mask: 0,
            character_mask: 0x1_0000,
        },
    }];
    world
}

fn request(scenario: Scenario, faction: CockpitFaction) -> FixtureRequest {
    FixtureRequest {
        scenario,
        faction,
        code: 0,
    }
}

/// `diplomacy_world` with its system named Yavin, Luke (a Jedi trainer)
/// and Jan Dodonna, and MISSNSD's Jedi Training record (`0x58000042`)
/// beside Diplomacy.
fn jedi_training_world() -> GameWorld {
    let mut world = diplomacy_world(Faction::Alliance);
    let here = world.systems.keys().next().unwrap();
    world.systems[here].name = "Yavin".into();
    for (name, trainer) in [("Luke Skywalker", true), ("Jan Dodonna", false)] {
        world.characters.insert(rebellion_core::world::Character {
            name: name.into(),
            is_major: trainer,
            is_jedi_trainer: trainer,
            is_empire: true,
            current_system: None,
            ..Default::default()
        });
    }
    let template = world.mission_records[0];
    world
        .mission_records
        .push(rebellion_core::world::MissionRecord {
            dat_id: rebellion_core::ids::DatId::new(0x5800_0042),
            repeats: false,
            ..template
        });
    world
}

#[test]
fn the_jedi_training_dialog_offers_jedi_training_first_to_luke_and_dodonna_on_yavin() {
    // A0 2026-11-16-edited-saves/x-54..x-56 (an Alliance game, day 813):
    // Luke, Force 100, and Dodonna selected on Yavin; the dialog shows
    // Jedi Training first (FUN_0046cab0 key 2), its agents the two.
    let mut world = jedi_training_world();
    apply(
        request(Scenario::JediTrainingDialog, CockpitFaction::Alliance),
        &mut world,
        &mut GameMode::MainMenu,
        &mut MissionFaction::Alliance,
        &mut CockpitState::new(CockpitFaction::Alliance),
        &mut GalaxyMapState::default(),
        &mut MovementState::default(),
        &mut ManufacturingState::default(),
        &mut EconomyState::default(),
        &mut MissionState::default(),
        &mut BlockadeState::default(),
        &mut SectorWindowState::default(),
        &mut SystemWindowState::default(),
        &mut FleetWindowState::default(),
        &mut DefensesWindowState::default(),
        &mut TroopTransportState::default(),
        &mut rebellion_core::uprising::UprisingState::new(),
    );
    let (luke, dodonna, yavin) = jedi_training_cast(&world).expect("the cast");
    assert_eq!(
        (
            world.characters[luke].force_level(),
            world.characters[dodonna].force_level()
        ),
        (100, 10)
    );
    assert!(world.characters[luke].dagobah_completed);
    for key in [luke, dodonna] {
        let value = &world.characters[key];
        assert_eq!(value.current_system, Some(yavin));
        assert!(value.is_alliance && !value.is_empire && value.recruited);
    }
    let mut dialog = MissionDialogState::default();
    assert!(open_mission_dialog(
        request(Scenario::JediTrainingDialog, CockpitFaction::Alliance),
        &world,
        &UprisingState::default(),
        &ResearchState::default(),
        &mut dialog,
    ));
    let open = dialog.dialog().expect("the dialog is open");
    assert_eq!(open.page(), MissionDialogPage::Mission);
    assert_eq!(open.kinds().first(), Some(&MissionKind::JediTraining));
    assert_eq!(
        open.agents(),
        [
            MissionMember::Character(luke),
            MissionMember::Character(dodonna)
        ]
    );
}

#[test]
fn the_jedi_training_messages_scenario_ends_lukes_training_with_dodonna_a_trainee() {
    // A0 2026-11-16-edited-saves/x-59, x-60: Luke's report on Yavin
    // and Dodonna's Force Growth, "My ranking is Trainee." (20..=79,
    // FUN_004486f0).
    let mut world = jedi_training_world();
    place_jedi_training(&mut world);
    let (luke, dodonna, yavin) = jedi_training_cast(&world).expect("the cast");
    let results = mission_results(
        request(Scenario::JediTrainingMessages, CockpitFaction::Alliance),
        &mut world,
    );
    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(
        (
            result.kind,
            result.character,
            result.target_system,
            result.returning_to
        ),
        (MissionKind::JediTraining, Some(luke), yavin, None)
    );
    assert_eq!(
        result.outcome,
        rebellion_core::missions::MissionOutcome::Success
    );
    let [rebellion_core::missions::MissionEffect::ForceTrainingGained { character, amount }] =
        result.effects.as_slice()
    else {
        unreachable!("one Force gain: {:?}", result.effects);
    };
    assert_eq!(*character, dodonna);
    let before = world.characters[dodonna].force_level();
    assert_eq!(
        rebellion_render::status_window::force_ranking(before),
        "Novice"
    );
    assert_eq!(
        rebellion_render::status_window::force_ranking(before + u32::try_from(*amount).unwrap()),
        "Trainee"
    );
    assert!(mission_results(
        request(Scenario::JediTrainingDialog, CockpitFaction::Alliance),
        &mut world
    )
    .is_empty());
    assert_eq!(Scenario::decode(135), Some(Scenario::JediTrainingDialog));
    assert_eq!(Scenario::decode(136), Some(Scenario::JediTrainingMessages));
}

#[test]
fn the_agents_scenario_opens_the_dialog_on_its_second_page() {
    let world = diplomacy_world(Faction::Alliance);
    let mut dialog = MissionDialogState::default();

    let opened = open_mission_dialog(
        request(Scenario::MissionDialogAgents, CockpitFaction::Alliance),
        &world,
        &UprisingState::default(),
        &ResearchState::default(),
        &mut dialog,
    );

    assert!(opened);
    let open = dialog.dialog().expect("the dialog is open");
    assert_eq!(open.page(), MissionDialogPage::Agents);
    assert_eq!(open.kind(), MissionKind::Diplomacy);
    assert_eq!(open.agents().len(), 1);
}

#[test]
fn the_mission_scenario_opens_the_empire_dialog_on_its_first_page() {
    let world = diplomacy_world(Faction::Empire);
    let mut dialog = MissionDialogState::default();

    let opened = open_mission_dialog(
        request(Scenario::MissionDialogMission, CockpitFaction::Empire),
        &world,
        &UprisingState::default(),
        &ResearchState::default(),
        &mut dialog,
    );

    assert!(opened);
    let open = dialog.dialog().expect("the dialog is open");
    assert_eq!(open.page(), MissionDialogPage::Mission);
    let Some(rebellion_render::mission_dialog::MissionDialogAction::Begin { faction, .. }) =
        dialog.begin()
    else {
        panic!("an open dialog begins its mission");
    };
    assert_eq!(faction, MissionFaction::Empire);
}

#[test]
fn other_scenarios_and_refused_teams_open_no_dialog() {
    // FUN_0042a320: with no legal kind nothing opens. An Alliance agent
    // cannot run an Empire mission (FUN_00583320).
    let world = diplomacy_world(Faction::Alliance);
    for request in [
        request(Scenario::Galaxy, CockpitFaction::Alliance),
        request(Scenario::MissionDialogAgents, CockpitFaction::Empire),
    ] {
        let mut dialog = MissionDialogState::default();

        let opened = open_mission_dialog(
            request,
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            &mut dialog,
        );

        assert!(!opened, "{:?}", request.scenario);
        assert!(!dialog.is_open());
    }
    let mut dialog = MissionDialogState::default();
    assert!(!open_mission_dialog(
        request(Scenario::MissionDialogAgents, CockpitFaction::Alliance),
        &GameWorld::default(),
        &UprisingState::default(),
        &ResearchState::default(),
        &mut dialog,
    ));
}

#[test]
fn the_enemy_character_scenario_frees_the_target_on_the_other_side_in_its_system() {
    // A0 kinds2/50: Veers' Mission released on Leia, free at Yavin.
    let mut world = diplomacy_world(Faction::Alliance);
    let here = world.systems.keys().next().unwrap();
    let mut second = world.systems[here].clone();
    second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
    let there = world.systems.insert(second);
    world.characters.insert(rebellion_core::world::Character {
        name: "Leia".into(),
        current_system: Some(there),
        ..Default::default()
    });
    let request = request(Scenario::EnemyCharacterTargeting, CockpitFaction::Alliance);
    let mut defenses = DefensesWindowState::default();
    apply(
        request,
        &mut world,
        &mut GameMode::MainMenu,
        &mut MissionFaction::Alliance,
        &mut CockpitState::new(CockpitFaction::Alliance),
        &mut GalaxyMapState::default(),
        &mut MovementState::default(),
        &mut ManufacturingState::default(),
        &mut EconomyState::default(),
        &mut MissionState::default(),
        &mut BlockadeState::default(),
        &mut SectorWindowState::default(),
        &mut SystemWindowState::default(),
        &mut FleetWindowState::default(),
        &mut defenses,
        &mut TroopTransportState::default(),
        &mut rebellion_core::uprising::UprisingState::new(),
    );
    let target = &world.characters[world.characters.keys().nth(1).unwrap()];
    assert!(!target.is_captive && target.is_empire && !target.is_alliance);
    assert_eq!(target.captured_by, None);
    assert_eq!(target.current_system, Some(there));
    assert!(defenses.is_open(here) && defenses.is_open(there));
}

#[test]
fn the_alliance_rescue_sends_han_solo_for_a_captive_leia_organa() {
    // A0 2026-11-15-partial-cells/rescue-dialog-han-on-leia-fleet90-
    // d1428 (sha256 44c8c052b951): Han Solo's Rescue on captive Leia.
    let mut world = diplomacy_world(Faction::Alliance);
    let here = world.systems.keys().next().unwrap();
    let mut second = world.systems[here].clone();
    second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
    world.systems.insert(second);
    for name in ["Bystander", "Han Solo", "Leia Organa"] {
        world.characters.insert(rebellion_core::world::Character {
            name: name.into(),
            is_alliance: true,
            recruited: true,
            current_system: Some(here),
            ..Default::default()
        });
    }
    let mut defenses = DefensesWindowState::default();
    apply(
        request(Scenario::RescueTargeting, CockpitFaction::Alliance),
        &mut world,
        &mut GameMode::MainMenu,
        &mut MissionFaction::Alliance,
        &mut CockpitState::new(CockpitFaction::Alliance),
        &mut GalaxyMapState::default(),
        &mut MovementState::default(),
        &mut ManufacturingState::default(),
        &mut EconomyState::default(),
        &mut MissionState::default(),
        &mut BlockadeState::default(),
        &mut SectorWindowState::default(),
        &mut SystemWindowState::default(),
        &mut FleetWindowState::default(),
        &mut defenses,
        &mut TroopTransportState::default(),
        &mut rebellion_core::uprising::UprisingState::new(),
    );
    let mut keys = world.characters.keys();
    let (agent, prisoner) = (keys.next().unwrap(), keys.next().unwrap());
    assert_eq!(world.characters[agent].name, "Han Solo");
    assert_eq!(world.characters[agent].current_system, Some(here));
    assert_eq!(world.characters[prisoner].name, "Leia Organa");
    assert!(world.characters[prisoner].is_captive);
}

#[test]
fn the_rescue_scenario_holds_a_prisoner_alone_in_the_enemy_prison_in_sight() {
    // The prisoner carries its captor's side (FUN_004eec60), so it lists
    // first on the prison's personnel page; the menu report aims at
    // that cell.
    let mut world = diplomacy_world(Faction::Alliance);
    let here = world.systems.keys().next().unwrap();
    let mut second = world.systems[here].clone();
    second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
    let prison = world.systems.insert(second);
    let bystander = world.characters.insert(rebellion_core::world::Character {
        name: "Bystander".into(),
        current_system: Some(prison),
        ..Default::default()
    });
    let request = request(Scenario::RescueTargeting, CockpitFaction::Alliance);
    let mut cockpit = CockpitState::new(CockpitFaction::Alliance);
    let mut defenses = DefensesWindowState::default();
    apply(
        request,
        &mut world,
        &mut GameMode::MainMenu,
        &mut MissionFaction::Alliance,
        &mut cockpit,
        &mut GalaxyMapState::default(),
        &mut MovementState::default(),
        &mut ManufacturingState::default(),
        &mut EconomyState::default(),
        &mut MissionState::default(),
        &mut BlockadeState::default(),
        &mut SectorWindowState::default(),
        &mut SystemWindowState::default(),
        &mut FleetWindowState::default(),
        &mut defenses,
        &mut TroopTransportState::default(),
        &mut rebellion_core::uprising::UprisingState::new(),
    );

    let prisoner = world.characters.keys().nth(1).unwrap();
    assert_eq!(prisoner, bystander);
    let held = &world.characters[prisoner];
    assert!(held.is_captive && held.is_alliance);
    assert_eq!(held.captured_by, Some(Faction::Empire));
    assert_eq!(held.current_system, Some(prison));
    assert_eq!(
        world.systems[prison].control,
        ControlKind::Controlled(Faction::Empire)
    );
    assert!(defenses.is_open(here) && defenses.is_open(prison));

    let (mut alliance, mut empire) = (
        FogState::new(Faction::Alliance),
        FogState::new(Faction::Empire),
    );
    reveal(request, &world, &mut alliance, &mut empire);
    assert!(alliance.is_visible(prison));

    let layout = cockpit.layout_for(640.0, 480.0);
    let menu = ObjectMenuState::new(
        Some(MenuObject::Character(
            world.characters.keys().next().unwrap(),
        )),
        OrderGates {
            mission: true,
            ..OrderGates::default()
        },
        (0, 0),
    );
    let rect = egui_macroquad::egui::Rect::from_min_size(
        egui_macroquad::egui::Pos2::ZERO,
        egui_macroquad::egui::vec2(10.0, 10.0),
    );
    let report = object_menu_report(
        request,
        (rect, None),
        &menu,
        &world,
        &SectorWindowState::default(),
        (
            &defenses,
            &SystemWindowState::default(),
            &FleetWindowState::default(),
        ),
    )
    .unwrap();
    let cell = defenses
        .cell_screen_rect(layout, prison, 0)
        .unwrap()
        .center();
    assert_eq!(
        (report.target_screen_x, report.target_screen_y),
        (cell.x, cell.y)
    );
}

#[test]
fn a_frozen_regiments_menu_reports_its_status_row_without_a_target_planet() {
    // The status-window gate's enemy-regiment-frozen case (b215: its
    // menu never reported): the scenario opens no sector window, so the
    // report targets no planet, as the Build Selection gates' do.
    let mut world = diplomacy_world(Faction::Alliance);
    let here = world.systems.keys().next().unwrap();
    let second = world.systems[here].clone();
    world.systems.insert(second);
    let rect = egui_macroquad::egui::Rect::from_min_size(
        egui_macroquad::egui::Pos2::ZERO,
        egui_macroquad::egui::vec2(10.0, 10.0),
    );
    for scenario in [Scenario::EnemyRegimentFrozen, Scenario::EnemyRegimentAtRest] {
        // b215: the scenario was not among those that report a menu.
        assert!(scenario.reports_object_menu());
        let menu = ObjectMenuState::new(
            Some(MenuObject::ViewedTroop {
                alliance: true,
                id: rebellion_core::ids::DatId::new(0x1000_0001),
            }),
            OrderGates::default(),
            (0, 0),
        );
        let report = object_menu_report(
            request(scenario, CockpitFaction::Alliance),
            (rect, None),
            &menu,
            &world,
            &SectorWindowState::default(),
            (
                &DefensesWindowState::default(),
                &SystemWindowState::default(),
                &FleetWindowState::default(),
            ),
        )
        .expect("the menu reports");
        assert!(report.status_window_row.is_some());
    }
}

#[test]
fn a_mission_icons_menu_reports_abort_without_a_target_planet() {
    // The Abort gate's menu has no target: no sector window shows the
    // second system, which the targeting scenarios need.
    let mut world = diplomacy_world(Faction::Alliance);
    let here = world.systems.keys().next().unwrap();
    let second = world.systems[here].clone();
    world.systems.insert(second);
    let rect = egui_macroquad::egui::Rect::from_min_size(
        egui_macroquad::egui::Pos2::ZERO,
        egui_macroquad::egui::vec2(10.0, 10.0),
    );
    let report = |scenario, abort| {
        let menu = ObjectMenuState::new(
            Some(MenuObject::SystemIcon {
                system: here,
                quadrant: Quadrant::Missions,
            }),
            OrderGates {
                abort,
                ..OrderGates::default()
            },
            (0, 0),
        );
        object_menu_report(
            request(scenario, CockpitFaction::Alliance),
            (rect, None),
            &menu,
            &world,
            &SectorWindowState::default(),
            (
                &DefensesWindowState::default(),
                &SystemWindowState::default(),
                &FleetWindowState::default(),
            ),
        )
        .map(|report| (report.abort_row, report.abort_enabled, report.retire_row))
    };

    assert_eq!(
        report(Scenario::Quadrants, true),
        Some((Some(2), true, None))
    );
    assert_eq!(
        report(Scenario::Quadrants, false),
        Some((Some(2), false, None))
    );
    assert_eq!(report(Scenario::MissionTargeting, true), None);
}

#[test]
fn the_fleets_both_sides_scenario_puts_one_fleet_of_each_side_in_a_seen_orbit() {
    // gid-fleet-marks.md: FUN_0042b330 case 0x21 draws 0x27b0 (STRATEGY
    // 10160) when the player's copy of a system holds usable fleets of
    // both sides; the player's own fleet there makes it seen
    // (FUN_0050ac80), so the enemy fleet's copy is live.
    assert_eq!(Scenario::decode(143), Some(Scenario::FleetsBothSides));
    assert_eq!(Scenario::FleetsBothSides.mode(), GidMode::IdleFleets);
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let applied =
            apply_to_loading_world_with(request(Scenario::FleetsBothSides, faction), |_| {});
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        let player = faction == CockpitFaction::Alliance;
        let sides: Vec<bool> = world.systems[primary]
            .fleets
            .iter()
            .filter(|&&fleet| !applied.movement.is_in_transit(fleet))
            .map(|&fleet| world.fleets[fleet].is_alliance)
            .collect();
        assert!(sides.contains(&player), "{faction:?}: {sides:?}");
        assert!(sides.contains(&!player), "{faction:?}: {sides:?}");
        let side = if player {
            Faction::Alliance
        } else {
            Faction::Empire
        };
        assert!(rebellion_core::object_views::system_seen(
            world,
            &applied.movement,
            side,
            primary
        ));
    }
}

#[test]
fn the_battery_scenario_opens_the_batteries_page_on_the_players_kdy_150() {
    // A0 status/22: Coruscant's Planetary Batteries page lists the
    // KDY-150 first, its menu Encyclopedia, Status, Scrap.
    let request = request(Scenario::BatteryStatus, CockpitFaction::Empire);
    let applied = apply_to_loading_world_with(request, |_| {});
    let world = &applied.world;
    let primary = world.systems.keys().next().unwrap();
    let first = world.systems[primary].defense_facilities[0];
    assert_eq!(
        world.defense_facilities[first].class_dat_id,
        rebellion_core::ids::DatId::new(0x2200_0001)
    );
    assert_eq!(world.defense_facilities[first].side, Faction::Empire);
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    assert!(applied
        .defenses
        .cell_screen_rect(layout, primary, 0)
        .is_some());
}

#[test]
fn the_uprising_scenario_makes_the_target_the_players_and_in_uprising() {
    // A0 kinds2/61: Subdue Uprising is offered on Selonia, the
    // Empire's, in uprising against it.
    let mut world = diplomacy_world(Faction::Empire);
    let here = world.systems.keys().next().unwrap();
    let mut second = world.systems[here].clone();
    second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
    let target = world.systems.insert(second);
    let mut uprisings = rebellion_core::uprising::UprisingState::new();
    apply(
        request(Scenario::UprisingTargeting, CockpitFaction::Empire),
        &mut world,
        &mut GameMode::MainMenu,
        &mut MissionFaction::Empire,
        &mut CockpitState::new(CockpitFaction::Empire),
        &mut GalaxyMapState::default(),
        &mut MovementState::default(),
        &mut ManufacturingState::default(),
        &mut EconomyState::default(),
        &mut MissionState::default(),
        &mut BlockadeState::default(),
        &mut SectorWindowState::default(),
        &mut SystemWindowState::default(),
        &mut FleetWindowState::default(),
        &mut DefensesWindowState::default(),
        &mut TroopTransportState::default(),
        &mut uprisings,
    );
    assert_eq!(
        world.systems[target].control,
        ControlKind::Uprising(Faction::Empire)
    );
    assert!(uprisings.is_uprising(target));
}

#[test]
fn the_research_scenarios_list_their_own_tree_alone_of_the_trees() {
    // FUN_0056c510: an own-side target with a working yard of the tree's
    // area, and a member with the tree's skill (FUN_0056c820). A0
    // 2026-11-11-mission-kinds/10: Tallon on Averam, one Training
    // Facility, is offered Troop Training Research.
    for (scenario, tree, cockpit_side, side) in [
        (
            Scenario::ResearchTargeting,
            MissionKind::ShipDesignResearch,
            CockpitFaction::Alliance,
            Faction::Alliance,
        ),
        (
            Scenario::ResearchTargeting,
            MissionKind::ShipDesignResearch,
            CockpitFaction::Empire,
            Faction::Empire,
        ),
        (
            Scenario::TroopResearchTargeting,
            MissionKind::TroopTrainingResearch,
            CockpitFaction::Alliance,
            Faction::Alliance,
        ),
        (
            Scenario::TroopResearchTargeting,
            MissionKind::TroopTrainingResearch,
            CockpitFaction::Empire,
            Faction::Empire,
        ),
    ] {
        let mut world = diplomacy_world(side);
        let here = world.systems.keys().next().unwrap();
        let mut second = world.systems[here].clone();
        second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
        let target = world.systems.insert(second);
        let template = world.mission_records[0];
        world.mission_records = [0x5100_0010, 0x5300_0020, 0x5300_0021, 0x5300_0022]
            .into_iter()
            .map(|id| rebellion_core::world::MissionRecord {
                dat_id: rebellion_core::ids::DatId::new(id),
                ..template
            })
            .collect();
        let mut missions = MissionState::default();
        apply(
            request(scenario, cockpit_side),
            &mut world,
            &mut GameMode::MainMenu,
            &mut MissionFaction::Alliance,
            &mut CockpitState::new(cockpit_side),
            &mut GalaxyMapState::default(),
            &mut MovementState::default(),
            &mut ManufacturingState::default(),
            &mut EconomyState::default(),
            &mut missions,
            &mut BlockadeState::default(),
            &mut SectorWindowState::default(),
            &mut SystemWindowState::default(),
            &mut FleetWindowState::default(),
            &mut DefensesWindowState::default(),
            &mut TroopTransportState::default(),
            &mut rebellion_core::uprising::UprisingState::new(),
        );
        let agent = MissionMember::Character(world.characters.keys().next().unwrap());
        let faction = if side == Faction::Alliance {
            MissionFaction::Alliance
        } else {
            MissionFaction::Empire
        };

        let kinds = available_kinds(
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            faction,
            &[agent],
            &[],
            target,
        );

        assert_eq!(world.systems[target].control, ControlKind::Controlled(side));
        let (own, other) = if side == Faction::Alliance {
            (
                world.systems[target].popularity_alliance,
                world.systems[target].popularity_empire,
            )
        } else {
            (
                world.systems[target].popularity_empire,
                world.systems[target].popularity_alliance,
            )
        };
        assert_eq!((own, other), (1.0, 0.0));
        assert!(world.recruit_pool_empty(side));
        assert_eq!(kinds, vec![tree], "{scenario:?} {side:?}");
        // The target's one yard: a shipyard for the ship tree only.
        let yards = &world.systems[target].manufacturing_facilities;
        assert_eq!(yards.len(), 1);
        assert_eq!(
            world.manufacturing_facilities[yards[0]].is_shipyard,
            tree == MissionKind::ShipDesignResearch,
            "{scenario:?}"
        );
    }
}

#[test]
fn the_research_diplomacy_scenario_boxes_research_then_diplomacy_then_espionage() {
    // A0 2026-11-13-mission-dialogs/40..43: Wedge Antilles on his own
    // Drall (a shipyard, support below full) lists Ship Design Research,
    // Diplomacy, Espionage: FUN_0046cab0's keys 4, 7, 9, not MISSNSD's
    // 0x10, 0x13, 0x20 order.
    for (cockpit_side, side, faction) in [
        (
            CockpitFaction::Alliance,
            Faction::Alliance,
            MissionFaction::Alliance,
        ),
        (
            CockpitFaction::Empire,
            Faction::Empire,
            MissionFaction::Empire,
        ),
    ] {
        let mut world = diplomacy_world(side);
        let here = world.systems.keys().next().unwrap();
        let mut second = world.systems[here].clone();
        second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
        let target = world.systems.insert(second);
        let template = world.mission_records[0];
        world.mission_records = [0x5100_0010, 0x5200_0013, 0x5300_0020]
            .into_iter()
            .map(|id| rebellion_core::world::MissionRecord {
                dat_id: rebellion_core::ids::DatId::new(id),
                ..template
            })
            .collect();
        let mut missions = MissionState::default();
        apply(
            request(Scenario::ResearchDiplomacyTargeting, cockpit_side),
            &mut world,
            &mut GameMode::MainMenu,
            &mut MissionFaction::Alliance,
            &mut CockpitState::new(cockpit_side),
            &mut GalaxyMapState::default(),
            &mut MovementState::default(),
            &mut ManufacturingState::default(),
            &mut EconomyState::default(),
            &mut missions,
            &mut BlockadeState::default(),
            &mut SectorWindowState::default(),
            &mut SystemWindowState::default(),
            &mut FleetWindowState::default(),
            &mut DefensesWindowState::default(),
            &mut TroopTransportState::default(),
            &mut rebellion_core::uprising::UprisingState::new(),
        );
        let agent = MissionMember::Character(world.characters.keys().next().unwrap());

        let kinds = available_kinds(
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            faction,
            &[agent],
            &[],
            target,
        );

        assert_eq!(
            rebellion_render::mission_dialog::box_order(kinds),
            [
                MissionKind::ShipDesignResearch,
                MissionKind::Diplomacy,
                MissionKind::Espionage
            ],
            "{side:?}"
        );
    }
}

#[test]
fn the_facility_research_scenario_boxes_facility_research_then_diplomacy_then_espionage() {
    // A0 2026-11-14-missions-transfer/11..14: Adar Tallon on his own
    // Jerijador (a construction yard alone, support below full) lists
    // Facility Design Research, Diplomacy, Espionage: FUN_0046cab0's
    // keys 5, 7, 9; FUN_0056c510 takes the yard of the facility tree
    // (MISSNSD 0x53000021).
    for (cockpit_side, side, faction) in [
        (
            CockpitFaction::Alliance,
            Faction::Alliance,
            MissionFaction::Alliance,
        ),
        (
            CockpitFaction::Empire,
            Faction::Empire,
            MissionFaction::Empire,
        ),
    ] {
        let mut world = diplomacy_world(side);
        let here = world.systems.keys().next().unwrap();
        let mut second = world.systems[here].clone();
        second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
        let target = world.systems.insert(second);
        let template = world.mission_records[0];
        world.mission_records = [
            0x5100_0010,
            0x5200_0013,
            0x5300_0020,
            0x5300_0021,
            0x5300_0022,
        ]
        .into_iter()
        .map(|id| rebellion_core::world::MissionRecord {
            dat_id: rebellion_core::ids::DatId::new(id),
            ..template
        })
        .collect();
        let mut missions = MissionState::default();
        apply(
            request(Scenario::FacilityResearchTargeting, cockpit_side),
            &mut world,
            &mut GameMode::MainMenu,
            &mut MissionFaction::Alliance,
            &mut CockpitState::new(cockpit_side),
            &mut GalaxyMapState::default(),
            &mut MovementState::default(),
            &mut ManufacturingState::default(),
            &mut EconomyState::default(),
            &mut missions,
            &mut BlockadeState::default(),
            &mut SectorWindowState::default(),
            &mut SystemWindowState::default(),
            &mut FleetWindowState::default(),
            &mut DefensesWindowState::default(),
            &mut TroopTransportState::default(),
            &mut rebellion_core::uprising::UprisingState::new(),
        );
        let agent = MissionMember::Character(world.characters.keys().next().unwrap());

        let kinds = available_kinds(
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            faction,
            &[agent],
            &[],
            target,
        );

        assert_eq!(
            rebellion_render::mission_dialog::box_order(kinds),
            [
                MissionKind::FacilityDesignResearch,
                MissionKind::Diplomacy,
                MissionKind::Espionage
            ],
            "{side:?}"
        );
        // A0's Jerijador holds one Construction Yard (family 0x2a), no
        // shipyard; the agent designs facilities alone.
        let yards = &world.systems[target].manufacturing_facilities;
        assert_eq!(yards.len(), 1);
        let yard = &world.manufacturing_facilities[yards[0]];
        assert_eq!(
            yard.class_dat_id,
            rebellion_core::ids::DatId::new(0x2a00_0001)
        );
        assert!(!yard.is_shipyard);
        let designer = world.characters.values().next().unwrap();
        assert_eq!(
            (
                designer.ship_design.base,
                designer.troop_training.base,
                designer.facility_design.base
            ),
            (0, 0, 40)
        );
    }
}

#[test]
fn the_targeting_scenario_frees_the_agent_and_shows_the_target_planet_clear_of_its_window() {
    let mut world = diplomacy_world(Faction::Alliance);
    let sector = world.sectors.insert(rebellion_core::world::Sector {
        dat_id: rebellion_core::ids::DatId::new(0x8000_0001),
        name: "Sector".into(),
        group: rebellion_core::dat::SectorGroup::Core,
        x: 0,
        y: 0,
        systems: Vec::new(),
    });
    let here = world.systems.keys().next().unwrap();
    world.systems[here].sector = sector;
    let mut second = world.systems[here].clone();
    second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
    second.name = "Target".into();
    second.x = 26;
    second.y = 20;
    let target = world.systems.insert(second);
    world.sectors[sector].systems = vec![here, target];
    let agent = world.characters.keys().next().unwrap();
    world.characters[agent].recruited = false;
    world.characters[agent].is_captive = true;
    let request = request(Scenario::MissionTargeting, CockpitFaction::Alliance);
    let mut cockpit = CockpitState::new(CockpitFaction::Alliance);
    let mut missions = MissionState::default();
    let mut sectors = SectorWindowState::default();
    let mut defenses = DefensesWindowState::default();

    apply(
        request,
        &mut world,
        &mut GameMode::MainMenu,
        &mut MissionFaction::Alliance,
        &mut cockpit,
        &mut GalaxyMapState::default(),
        &mut MovementState::default(),
        &mut ManufacturingState::default(),
        &mut EconomyState::default(),
        &mut missions,
        &mut BlockadeState::default(),
        &mut sectors,
        &mut SystemWindowState::default(),
        &mut FleetWindowState::default(),
        &mut defenses,
        &mut TroopTransportState::default(),
        &mut rebellion_core::uprising::UprisingState::new(),
    );

    // MissionState::mission_order_enabled enables the menu's Mission.
    assert!(missions.mission_order_enabled(
        &world,
        MissionFaction::Alliance,
        &[MissionMember::Character(agent)],
    ));
    let layout = cockpit.layout_for(640.0, 480.0);
    let galaxy = layout.galaxy;
    // The Defenses window's top-left corner, 5 pixels in from the
    // view's top-right.
    let corner = (
        galaxy.x + galaxy.width - DEFENSES_WINDOW_WIDTH - 5.0,
        galaxy.y + 5.0,
    );
    assert!(defenses.contains_screen_point(layout, corner));
    assert!(!defenses.contains_screen_point(layout, (corner.0 - 1.0, corner.1)));
    assert!(!defenses.contains_screen_point(layout, (corner.0, corner.1 - 1.0)));
    assert_eq!(sectors.window_count(), 1);

    let menu = ObjectMenuState::new(
        Some(MenuObject::Character(agent)),
        OrderGates {
            mission: true,
            ..OrderGates::default()
        },
        (0, 0),
    );
    let rect = egui_macroquad::egui::Rect::from_min_size(
        egui_macroquad::egui::pos2(100.0, 150.0),
        egui_macroquad::egui::vec2(120.0, 142.0),
    );
    let report = object_menu_report(
        request,
        (rect, None),
        &menu,
        &world,
        &sectors,
        (
            &DefensesWindowState::default(),
            &SystemWindowState::default(),
            &FleetWindowState::default(),
        ),
    )
    .unwrap();
    assert_eq!(
        (report.left, report.top, report.width, report.height),
        (100.0, 150.0, 120.0, 142.0)
    );
    assert_eq!((report.rows, report.mission_row), (7, Some(2)));
    assert_eq!(
        (report.target_dat_id, report.target_name),
        (0x9000_0002, "Target")
    );
    // The Alliance's first sector window sits at (60, 35); the target's
    // planet, at sector offset (26, 20), sits at (77, 70) in it
    // (FUN_00459e30 at 0x45a3d4) and is 37 by 37.
    let target = (report.target_screen_x, report.target_screen_y);
    assert_eq!(target, (60.0 + 77.0 + 18.5, 35.0 + 70.0 + 18.5));
    assert!(sectors.contains_screen_point(layout, target));
    assert!(!defenses.contains_screen_point(layout, target));
}

/// Three systems in one sector, the second the target in the targeting
/// layout; three fleets of the other side with no capital ship, two at the
/// primary system and one at the target; and one capital ship class.
fn fleet_world() -> GameWorld {
    use rebellion_core::world::{CapitalShipClass, Fleet};
    let mut world = diplomacy_world(Faction::Alliance);
    let sector = world.sectors.insert(rebellion_core::world::Sector {
        dat_id: rebellion_core::ids::DatId::new(0x8000_0001),
        name: "Sector".into(),
        group: rebellion_core::dat::SectorGroup::Core,
        x: 0,
        y: 0,
        systems: Vec::new(),
    });
    let here = world.systems.keys().next().unwrap();
    world.systems[here].sector = sector;
    let mut others = Vec::new();
    for (raw, x) in [(0x9000_0002, 26), (0x9000_0003, 300)] {
        let mut system = world.systems[here].clone();
        system.dat_id = rebellion_core::ids::DatId::new(raw);
        system.x = x;
        system.y = 20;
        others.push(world.systems.insert(system));
    }
    world.sectors[sector].systems = vec![here, others[0], others[1]];
    world.capital_ship_classes.insert(CapitalShipClass {
        hyperdrive: 80,
        ..CapitalShipClass::default()
    });
    for location in [here, here, others[0]] {
        world.fleets.insert(Fleet {
            location,
            ..Default::default()
        });
    }
    world
}

struct Applied {
    world: GameWorld,
    movement: MovementState,
    blockade: BlockadeState,
    map: GalaxyMapState,
    sectors: SectorWindowState,
    systems: SystemWindowState,
    fleets: FleetWindowState,
    defenses: DefensesWindowState,
    transport: TroopTransportState,
    missions: MissionState,
    manufacturing: ManufacturingState,
}

fn apply_to_fleet_world(request: FixtureRequest) -> Applied {
    let mut applied = Applied {
        world: fleet_world(),
        movement: MovementState::default(),
        blockade: BlockadeState::default(),
        map: GalaxyMapState::default(),
        sectors: SectorWindowState::default(),
        systems: SystemWindowState::default(),
        fleets: FleetWindowState::default(),
        defenses: DefensesWindowState::default(),
        transport: TroopTransportState::default(),
        missions: MissionState::default(),
        manufacturing: ManufacturingState::default(),
    };
    apply(
        request,
        &mut applied.world,
        &mut GameMode::MainMenu,
        &mut MissionFaction::Alliance,
        &mut CockpitState::new(request.faction),
        &mut applied.map,
        &mut applied.movement,
        &mut applied.manufacturing,
        &mut EconomyState::default(),
        &mut applied.missions,
        &mut applied.blockade,
        &mut applied.sectors,
        &mut applied.systems,
        &mut applied.fleets,
        &mut applied.defenses,
        &mut applied.transport,
        &mut rebellion_core::uprising::UprisingState::new(),
    );
    applied
}

/// [`fleet_world`] with its third system moved into a sector of its own,
/// where the loading scenarios send what they clear away.
fn apply_to_loading_world(request: FixtureRequest) -> Applied {
    apply_to_loading_world_with(request, |_| {})
}

fn apply_to_loading_world_with(
    request: FixtureRequest,
    prepare: impl FnOnce(&mut GameWorld),
) -> Applied {
    let mut world = fleet_world();
    let mut keys = world.systems.keys();
    let (here, elsewhere) = (keys.next().unwrap(), keys.nth(1).unwrap());
    let home = world.systems[here].sector;
    let mut far = world.sectors[home].clone();
    far.dat_id = rebellion_core::ids::DatId::new(0x8000_0002);
    far.systems = vec![elsewhere];
    let far = world.sectors.insert(far);
    world.sectors[home].systems.retain(|&key| key != elsewhere);
    world.systems[elsewhere].sector = far;
    prepare(&mut world);
    let mut applied = Applied {
        world,
        movement: MovementState::default(),
        blockade: BlockadeState::default(),
        map: GalaxyMapState::default(),
        sectors: SectorWindowState::default(),
        systems: SystemWindowState::default(),
        fleets: FleetWindowState::default(),
        defenses: DefensesWindowState::default(),
        transport: TroopTransportState::default(),
        missions: MissionState::default(),
        manufacturing: ManufacturingState::default(),
    };
    apply(
        request,
        &mut applied.world,
        &mut GameMode::MainMenu,
        &mut MissionFaction::Alliance,
        &mut CockpitState::new(request.faction),
        &mut applied.map,
        &mut applied.movement,
        &mut applied.manufacturing,
        &mut EconomyState::default(),
        &mut applied.missions,
        &mut applied.blockade,
        &mut applied.sectors,
        &mut applied.systems,
        &mut applied.fleets,
        &mut applied.defenses,
        &mut applied.transport,
        &mut rebellion_core::uprising::UprisingState::new(),
    );
    applied
}

// port: the fixture's own layout, copying A0 menus/23-fleet1-*.png.
#[test]
fn the_a0_kimanan_scenario_lists_fleet_1_loaded_and_fleet_4_en_route() {
    let request = request(Scenario::FleetWindowA0, CockpitFaction::Alliance);
    let applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Kimanan".into();
        world.systems[elsewhere].sector = world.systems[here].sector;
        for name in ["Corellian Corvette", "Medium Transport"] {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: name.into(),
                    is_alliance: true,
                    hull: 100,
                    hyperdrive: 80,
                    troop_capacity: 1,
                    ..Default::default()
                });
        }
    });
    let world = &applied.world;
    let kimanan = world.systems.keys().next().unwrap();
    let listed = rebellion_core::movement::listed_fleets(&applied.movement, world, kimanan);
    let names: Vec<_> = listed
        .iter()
        .map(|&fleet| world.fleet_name(fleet).unwrap())
        .collect();
    assert_eq!(names, ["Fleet 1", "Fleet 4"]);
    let (first, fourth) = (listed[0], listed[1]);
    assert_eq!(world.systems[kimanan].fleets, [first]);
    assert!(applied.movement.is_in_transit(fourth));
    assert_eq!(applied.transport.cargo(first).len(), 2);
    assert_eq!(world.ship_name(first, 1), Some("Medium Transport 1"));
    assert_eq!(world.ship_name(fourth, 0), Some("Corellian Corvette 2"));
    assert!(applied.fleets.is_open(kimanan));
    assert_eq!(
        applied
            .fleets
            .screen_rect(
                CockpitState::new(request.faction).layout_for(640.0, 480.0),
                kimanan
            )
            .map(|rect| rect.min),
        Some(egui_macroquad::egui::pos2(305.0, 86.0))
    );
}

#[test]
fn the_headquarters_taken_scenario_ends_the_alliance_headquarters_with_yavin() {
    // A0 2026-11-16-partial-cells (an Alliance game): v-17 (sha256
    // 2d0316e7) marks Yavin the headquarters; the Empire's assault takes
    // it (v-19, a2feadda, "Imperial troops have taken control of the
    // Alliance system Yavin"), and v-20 (c9066ff2) shows Imperial Yavin
    // without the mark. The control change removes the HQ object, whose
    // class serves the Alliance alone (FUN_004f8680, FUN_004f27d0).
    let request = request(Scenario::HeadquartersTaken, CockpitFaction::Alliance);
    let mut applied = apply_to_loading_world_with(request, |world| {
        seed_yavin_assault(world);
        world.buildable_classes.insert(
            rebellion_core::ids::DatId::new(0x2000_0001),
            rebellion_core::world::BuildableClass {
                name: "Alliance HQ".into(),
                is_alliance: true,
                ..Default::default()
            },
        );
    });
    let yavin = applied.world.systems.keys().next().unwrap();
    let hq = |world: &GameWorld| rebellion_render::sector_window::headquarters_object(world, yavin);
    assert!(applied.world.systems[yavin].is_headquarters);
    assert!(hq(&applied.world).is_some());
    assert_eq!(
        applied
            .world
            .systems
            .values()
            .filter(|s| s.is_headquarters)
            .count(),
        1
    );
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    assert!(applied
        .sectors
        .planet_screen_rect(&applied.world, layout, yavin)
        .is_some());
    assert!(!opens_the_assault_report(request));
    let reports = enemy_assault(request, &mut applied.world, &mut applied.transport);
    let [report] = &reports[..] else {
        panic!("one report: {reports:?}");
    };
    assert!(report.taken);
    let world = &applied.world;
    assert_eq!(
        world.systems[yavin].control,
        ControlKind::Controlled(Faction::Empire)
    );
    assert!(!world.systems[yavin].is_headquarters);
    assert!(hq(world).is_none());
}

#[test]
fn the_uprising_messages_scenario_files_selonia_begins_and_ends_then_yavin_begins() {
    // A0 2026-11-16-empire-hunt (an Empire game): h-38 (sha256
    // 4a220a83) "An uprising has begun on the Imperial controlled
    // system Selonia." over 0x3f2; h-34 (81ec8a4e) "Uprising Ends on
    // Selonia" over 0x3f3, FUN_00499460's picture for a system whose
    // side bits are 1 (the Alliance's); h-50 (5f56fc09) Yavin begins.
    use rebellion_render::message_pictures as pictures;
    let empire = request(Scenario::UprisingMessages, CockpitFaction::Empire);
    assert_eq!(
        uprising_messages(empire),
        vec![
            ("Selonia", Faction::Empire, true),
            ("Selonia", Faction::Alliance, false),
            ("Yavin", Faction::Empire, true),
        ]
    );
    assert_eq!(
        pictures::uprising_ends(Faction::Alliance).map(|picture| picture.background),
        Some(0x3f3)
    );
    assert_eq!(pictures::uprising_begins().background, 0x3f2);
    let other = request(Scenario::EventMessages, CockpitFaction::Empire);
    assert!(uprising_messages(other).is_empty());
    assert_eq!(Scenario::decode(133), Some(Scenario::UprisingMessages));
}

#[test]
fn the_ghorman_bombardment_scenario_opens_the_empires_strike_on_alliance_ghorman() {
    // A0 2026-11-17-edited-saves-5 (a new, unedited Empire game, day
    // 33): Fleet 1 (i-04: an Imperial Star Destroyer and an Imperial
    // Dreadnaught) bombards the Alliance's Ghorman; no side loses a
    // unit (i-03, i-04), and the Empire's own summary opens at once
    // (i-02, sha256 bc23a22b): "Orbital bombardment of Ghorman"
    // (0x7098) / "Imperial ships have conducted an orbital strike on
    // the Alliance system of Ghorman" (0x7099) over 0x2b9a.
    let request = request(Scenario::GhormanBombardment, CockpitFaction::Empire);
    let mut applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Ghorman".into();
        world.systems[elsewhere].sector = world.systems[here].sector;
        world.systems[here].control = ControlKind::Controlled(Faction::Empire);
        for name in [
            "Imperial Star Destroyer",
            "Imperial Dreadnaught",
            "Imperial Dreadnaught II",
        ] {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: name.into(),
                    is_empire: true,
                    hull: 100,
                    hyperdrive: 80,
                    bombardment_modifier: 1,
                    ..Default::default()
                });
        }
    });
    let world = &applied.world;
    let ghorman = world.systems.keys().next().unwrap();
    assert_eq!(
        world.systems[ghorman].control,
        ControlKind::Controlled(Faction::Alliance)
    );
    let [fleet] = &world.systems[ghorman].fleets[..] else {
        panic!("one fleet at Ghorman: {:?}", world.systems[ghorman].fleets);
    };
    let ships: Vec<_> = world.fleets[*fleet]
        .capital_ships
        .iter()
        .map(|ship| world.capital_ship_classes[ship.class].name.as_str())
        .collect();
    assert_eq!(ships, ["Imperial Star Destroyer", "Imperial Dreadnaught"]);
    assert!(!world.fleets[*fleet].is_alliance);
    assert!(opens_the_assault_report(request));
    // Only the Empire's fleets at Ghorman strike: an Alliance fleet
    // there and an Imperial one elsewhere stay out of the report.
    let imperial = world.fleets[*fleet].clone();
    let elsewhere = world.systems.keys().nth(1).unwrap();
    let mut away = imperial.clone();
    away.location = elsewhere;
    applied.world.insert_fleet(away);
    let mut defender = imperial;
    defender.is_alliance = true;
    applied.world.insert_fleet(defender);
    let reports = enemy_assault(request, &mut applied.world, &mut applied.transport);
    let [report] = &reports[..] else {
        panic!("one report: {reports:?}");
    };
    assert_eq!(report.system, ghorman);
    assert_eq!(report.attacker, Faction::Empire);
    assert_eq!(report.defender, Some(Faction::Alliance));
    assert!(!report.assault);
    assert!(report.opens_at_once(Faction::Empire));
    assert_eq!(
        (report.title(), report.body(), report.picture()),
        (0x7098, 0x7099, 0x2b9a)
    );
    // Fleet 1's two ships, none lost.
    assert_eq!(report.attackers.operational.len(), 2);
    assert!(report.attackers.destroyed.is_empty());
    assert_eq!(Scenario::decode(139), Some(Scenario::GhormanBombardment));
}

#[test]
fn the_ghorman_fleet_carries_the_empires_first_star_destroyer_and_dreadnaught() {
    // A0 2026-11-17-edited-saves-5/i-04 (sha256 prefix in the gate):
    // Fleet 1's ships read "Imperial Star Destroyer 1" and "Imperial
    // Dreadnaught 1". A ship's number is its side's count of the class
    // plus one (FUN_00517760 -> FUN_005302c0, never lowered), so when
    // the seed already holds the Empire's first of each (the browser's
    // seed 0 puts Imperial Dreadnaught 1 at Drall), Fleet 1 takes
    // those ships rather than making a second of each.
    let request = request(Scenario::GhormanBombardment, CockpitFaction::Empire);
    let applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Ghorman".into();
        world.systems[elsewhere].sector = world.systems[here].sector;
        let mut seeded = Vec::new();
        // Each class its own record, so each has its own counter.
        for (raw, name) in [
            (0x1400_0085, "Imperial Star Destroyer"),
            (0x1400_0087, "Imperial Dreadnaught"),
        ] {
            let value = rebellion_core::world::CapitalShipClass {
                dat_id: rebellion_core::ids::DatId::new(raw),
                name: name.into(),
                is_empire: true,
                hull: 100,
                hyperdrive: 80,
                bombardment_modifier: 1,
                ..Default::default()
            };
            let class = world.capital_ship_classes.insert(value);
            seeded.push(ShipInstance::new(class, 100, false));
        }
        let far = world.systems.keys().nth(2).unwrap();
        for ship in seeded {
            let fleet = world.insert_fleet(rebellion_core::world::Fleet {
                location: far,
                capital_ships: vec![ship],
                ..Default::default()
            });
            world.systems[far].fleets.push(fleet);
        }
        world.number_ships();
    });
    let world = &applied.world;
    let ghorman = world.systems.keys().next().unwrap();
    let [fleet] = &world.systems[ghorman].fleets[..] else {
        panic!("one fleet at Ghorman: {:?}", world.systems[ghorman].fleets);
    };
    let names: Vec<_> = world.fleets[*fleet]
        .capital_ships
        .iter()
        .map(|ship| ship.name.as_deref().unwrap_or_default())
        .collect();
    let first = ["Imperial Star Destroyer 1", "Imperial Dreadnaught 1"];
    assert_eq!(names, first);
    // Nobody else bears those names, and no second of either was made.
    let all: Vec<_> = world
        .fleets
        .values()
        .flat_map(|fleet| fleet.capital_ships.iter())
        .filter_map(|ship| ship.name.as_deref())
        .collect();
    assert_eq!(all.len(), 2, "{all:?}");
}

/// The `EnemyAssaultTaken` world: Yavin with GNPRTB as shipped, the
/// Imperial Star Destroyer and the two regiment classes.
fn seed_yavin_assault(world: &mut GameWorld) {
    let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
    let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
    world.systems[here].name = "Yavin".into();
    world.systems[elsewhere].sector = world.systems[here].sector;
    // GNPRTB as shipped (ghidra/notes/planetary-assault.md):
    // 0x200..0x205, and the garrison 0x1e51 / 0x1e52 that bounds
    // the landing (FUN_0058c810).
    let entry = |id: u32, value: i32| rebellion_core::world::GnprtbEntry {
        parameter_id: id,
        development: value,
        alliance_sp_easy: value,
        alliance_sp_medium: value,
        alliance_sp_hard: value,
        empire_sp_easy: value,
        empire_sp_medium: value,
        empire_sp_hard: value,
        multiplayer: value,
    };
    world.gnprtb = rebellion_core::world::GnprtbParams::new(vec![
        entry(0x200, 5),
        entry(0x201, 30),
        entry(0x202, 10),
        entry(0x203, 4),
        entry(0x204, 6),
        entry(0x205, 8),
        entry(0x1e51, 60),
        entry(0x1e52, -10),
    ]);
    world
        .capital_ship_classes
        .insert(rebellion_core::world::CapitalShipClass {
            name: "Imperial Star Destroyer".into(),
            is_empire: true,
            hull: 100,
            hyperdrive: 80,
            troop_capacity: 3,
            ..Default::default()
        });
    for (id, name, alliance) in [
        (0x1000_0007, "Stormtrooper Regiment", false),
        (0x1000_0001, "Alliance Army Regiment", true),
    ] {
        world.buildable_classes.insert(
            rebellion_core::ids::DatId::new(id),
            rebellion_core::world::BuildableClass {
                name: name.into(),
                is_alliance: alliance,
                is_empire: !alliance,
                ..Default::default()
            },
        );
    }
}

#[test]
fn the_ghorman_assault_scenario_takes_the_undefended_ghorman_and_opens_the_empires_summary() {
    // A0 2026-11-17-edited-saves-5 (a new, unedited Empire game, day
    // 33): Fleet 1 assaults the Alliance's Ghorman, which holds no
    // regiment; i-06 "Assault on Ghorman" / "Imperial troops have
    // taken control of the Alliance system Ghorman" (FUN_00496770,
    // TEXTSTRA 0x7090 / 0x7091) over the AT-ATs 0x2b99, opened at once
    // as the attacker's own report (FUN_00497530 kind 7); i-07 System
    // Forces' regiments "None" / "No Casualties"; i-08 Ghorman green.
    let request = request(Scenario::GhormanAssault, CockpitFaction::Empire);
    let mut applied = apply_to_loading_world_with(request, |world| {
        seed_yavin_assault(world);
        let here = world.systems.keys().next().unwrap();
        world.systems[here].name = "Ghorman".into();
    });
    let ghorman = applied.world.systems.keys().next().unwrap();
    assert!(applied.world.systems[ghorman].ground_units.is_empty());
    let [fleet] = &applied.world.systems[ghorman].fleets[..] else {
        panic!("one fleet at Ghorman");
    };
    assert_eq!(applied.transport.cargo(*fleet).len(), 1);
    assert!(opens_the_assault_report(request));
    let reports = enemy_assault(request, &mut applied.world, &mut applied.transport);
    let [report] = &reports[..] else {
        panic!("one report: {reports:?}");
    };
    assert!(report.assault && report.taken);
    assert_eq!(report.attacker, Faction::Empire);
    assert!(report.opens_at_once(Faction::Empire));
    assert_eq!(
        (report.title(), report.body(), report.picture()),
        (0x7090, 0x7091, 0x2b99)
    );
    assert!(report.attackers.destroyed.is_empty());
    let troops = report
        .system_forces
        .columns(rebellion_core::assault_report::ReportCategory::Troops);
    assert!(troops.iter().all(Vec::is_empty), "{troops:?}");
    assert_eq!(
        applied.world.systems[ghorman].control,
        ControlKind::Controlled(Faction::Empire)
    );
    assert_eq!(Scenario::decode(140), Some(Scenario::GhormanAssault));
}

#[test]
fn the_enemy_assault_scenario_takes_yavin_from_its_one_regiment_as_a0_shows() {
    // A0 assault-clean/01..03 (an unedited Alliance game, day 720): the
    // Empire took the Alliance's Yavin; Imperial Forces list
    // Stormtrooper Regiments, "No Casualties"; System Forces lost an
    // Alliance Army Regiment, Operational "None". The Alliance reads it
    // from its Conflict rail (FUN_00497530 kind 3).
    use rebellion_core::assault_report::ReportCategory;
    let enemy = request(Scenario::EnemyAssaultTaken, CockpitFaction::Alliance);
    let mut applied = apply_to_loading_world_with(enemy, seed_yavin_assault);
    let reports = enemy_assault(enemy, &mut applied.world, &mut applied.transport);
    let [report] = &reports[..] else {
        panic!("one report: {reports:?}");
    };
    assert_eq!(report.attacker, Faction::Empire);
    assert_eq!(report.defender, Some(Faction::Alliance));
    assert!(report.taken);
    assert!(!report.opens_at_once(Faction::Alliance));
    let name = |units: &[&rebellion_core::assault_report::ReportUnit]| {
        units
            .iter()
            .map(|unit| unit.name.clone())
            .collect::<Vec<_>>()
    };
    let [operational, _, destroyed] = report.attackers.columns(ReportCategory::Troops);
    assert_eq!(
        name(&operational),
        vec!["Stormtrooper Regiment".to_string(); ENEMY_ASSAULT_REGIMENTS]
    );
    assert!(destroyed.is_empty());
    let [operational, _, destroyed] = report.system_forces.columns(ReportCategory::Troops);
    assert!(operational.is_empty());
    assert_eq!(name(&destroyed), vec!["Alliance Army Regiment".to_string()]);
    // Any other scenario assaults nothing.
    let other = request(Scenario::FleetAssaultTaken, CockpitFaction::Alliance);
    assert!(enemy_assault(other, &mut applied.world, &mut applied.transport).is_empty());
}

#[test]
fn the_own_assault_scenario_is_repulsed_from_commenor_as_a0_shows() {
    // A0 2026-11-14-bombard-assault/09..11 (a new Alliance game, day
    // 148): the Alliance's Fleet 1 landed one Alliance Fleet Regiment
    // on Imperial Commenor; Alliance Forces lost it, System Forces
    // keep five Imperial Army Regiments, "No Casualties"; the summary
    // opens at once (FUN_00497530 kind 7).
    use rebellion_core::assault_report::ReportCategory;
    let own = request(Scenario::OwnAssaultRepulsed, CockpitFaction::Alliance);
    let mut applied = apply_to_loading_world_with(own, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Commenor".into();
        world.systems[elsewhere].sector = world.systems[here].sector;
        let entry = |id: u32, value: i32| rebellion_core::world::GnprtbEntry {
            parameter_id: id,
            development: value,
            alliance_sp_easy: value,
            alliance_sp_medium: value,
            alliance_sp_hard: value,
            empire_sp_easy: value,
            empire_sp_medium: value,
            empire_sp_hard: value,
            multiplayer: value,
        };
        world.gnprtb = rebellion_core::world::GnprtbParams::new(vec![
            entry(0x200, 5),
            entry(0x201, 30),
            entry(0x202, 10),
            entry(0x203, 4),
            entry(0x204, 6),
            entry(0x205, 8),
            entry(0x1e51, 60),
            entry(0x1e52, -10),
        ]);
        world
            .capital_ship_classes
            .insert(rebellion_core::world::CapitalShipClass {
                name: "Medium Transport".into(),
                is_alliance: true,
                hull: 100,
                hyperdrive: 80,
                troop_capacity: 2,
                ..Default::default()
            });
        for (id, name, alliance) in [
            (0x1000_0003, "Alliance Fleet Regiment", true),
            (0x1000_0005, "Imperial Army Regiment", false),
        ] {
            world.buildable_classes.insert(
                rebellion_core::ids::DatId::new(id),
                rebellion_core::world::BuildableClass {
                    name: name.into(),
                    is_alliance: alliance,
                    is_empire: !alliance,
                    ..Default::default()
                },
            );
        }
    });
    let reports = enemy_assault(own, &mut applied.world, &mut applied.transport);
    let [report] = &reports[..] else {
        panic!("one report: {reports:?}");
    };
    assert_eq!(report.attacker, Faction::Alliance);
    assert_eq!(report.defender, Some(Faction::Empire));
    assert!(!report.taken);
    assert!(report.opens_at_once(Faction::Alliance));
    let name = |units: &[&rebellion_core::assault_report::ReportUnit]| {
        units
            .iter()
            .map(|unit| unit.name.clone())
            .collect::<Vec<_>>()
    };
    let [operational, _, destroyed] = report.attackers.columns(ReportCategory::Troops);
    assert!(operational.is_empty());
    assert_eq!(
        name(&destroyed),
        vec!["Alliance Fleet Regiment".to_string()]
    );
    let [operational, _, destroyed] = report.system_forces.columns(ReportCategory::Troops);
    assert_eq!(
        name(&operational),
        vec!["Imperial Army Regiment".to_string(); OWN_ASSAULT_DEFENDERS]
    );
    assert!(destroyed.is_empty());
}

#[test]
fn the_a0_coruscant_scenario_loads_fleet_1_as_a0_shows_it() {
    // A0 fleet-aboard/00..12 (a new Empire game, day 10): Fleet 1's one
    // Imperial Star Destroyer carries six TIE Fighter squadrons, three
    // Stormtrooper Regiments and Veers.
    let request = request(Scenario::FleetWindowCoruscant, CockpitFaction::Empire);
    let applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Coruscant".into();
        world.systems[elsewhere].sector = world.systems[here].sector;
        world
            .capital_ship_classes
            .insert(rebellion_core::world::CapitalShipClass {
                name: "Imperial Star Destroyer".into(),
                is_empire: true,
                hull: 100,
                hyperdrive: 80,
                fighter_capacity: 6,
                troop_capacity: 3,
                ..Default::default()
            });
        world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                name: "TIE Fighter".into(),
                is_empire: true,
                ..Default::default()
            });
        world.buildable_classes.insert(
            rebellion_core::ids::DatId::new(0x1000_0007),
            rebellion_core::world::BuildableClass {
                name: "Stormtrooper Regiment".into(),
                is_empire: true,
                ..Default::default()
            },
        );
        world.characters.insert(rebellion_core::world::Character {
            name: "Veers".into(),
            ..Default::default()
        });
    });
    let world = &applied.world;
    let coruscant = world.systems.keys().next().unwrap();
    let [first] = world.systems[coruscant].fleets[..] else {
        panic!(
            "one fleet at Coruscant: {:?}",
            world.systems[coruscant].fleets
        );
    };
    let fleet = &world.fleets[first];
    assert_eq!(world.fleet_name(first), Some("Fleet 1"));
    assert_eq!(world.ship_name(first, 0), Some("Imperial Star Destroyer 1"));
    assert!(!fleet.is_alliance);
    assert_eq!(
        fleet.fighters.iter().map(|entry| entry.count).sum::<u32>(),
        6
    );
    let [veers] = fleet.characters[..] else {
        panic!("one character aboard");
    };
    // Seated on the numbered destroyer, so its entry shows the
    // personnel mark 10456 as A0's does.
    assert_eq!(
        world.characters[veers].ship,
        Some(fleet.capital_ships[0].id)
    );
    assert_ne!(world.characters[veers].ship, Some(0));
    assert_eq!(world.systems[coruscant].ground_units.len(), 0);
    assert!(applied.fleets.is_open(coruscant));
}

#[test]
fn the_probe_droid_aboard_scenario_loads_fleet_1_as_a0_f2_shows_it() {
    // A0 2026-11-16-partial-cells/f-2 (sha256 ce8c644c6e6a): Fleet 1's
    // Imperial Star Destroyer 1 marked with squadrons, regiments and
    // personnel, Victory Destroyer 1 with squadrons and regiments
    // (FUN_004a66a0 flags 1, 2, 4); f-3 (9b90894dbe9b) lists the
    // Imperial Probe Droid alone on the Personnel tab.
    let request = request(Scenario::SpecialForceAboard, CockpitFaction::Empire);
    let applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Coruscant".into();
        world.systems[elsewhere].sector = world.systems[here].sector;
        for (name, fighters, troops) in [
            ("Imperial Star Destroyer", 6, 3),
            ("Victory Destroyer", 2, 2),
        ] {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: name.into(),
                    is_empire: true,
                    hull: 100,
                    hyperdrive: 80,
                    fighter_capacity: fighters,
                    troop_capacity: troops,
                    ..Default::default()
                });
        }
        world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                name: "TIE Fighter".into(),
                is_empire: true,
                ..Default::default()
            });
        world.buildable_classes.insert(
            rebellion_core::ids::DatId::new(0x1000_0007),
            rebellion_core::world::BuildableClass {
                name: "Stormtrooper Regiment".into(),
                is_empire: true,
                ..Default::default()
            },
        );
    });
    let world = &applied.world;
    let coruscant = world.systems.keys().next().unwrap();
    let [first] = world.systems[coruscant].fleets[..] else {
        panic!(
            "one fleet at Coruscant: {:?}",
            world.systems[coruscant].fleets
        );
    };
    let fleet = &world.fleets[first];
    assert_eq!(world.fleet_name(first), Some("Fleet 1"));
    assert_eq!(world.ship_name(first, 0), Some("Imperial Star Destroyer 1"));
    assert_eq!(world.ship_name(first, 1), Some("Victory Destroyer 1"));
    for ship in &fleet.capital_ships {
        assert!(
            fleet
                .fighters
                .iter()
                .any(|entry| entry.carrier == Some(ship.id)),
            "squadrons aboard {}",
            ship.id
        );
    }
    // One regiment more than the Star Destroyer holds.
    assert_eq!(applied.transport.cargo(first).len(), 4);
    assert!(fleet.characters.is_empty());
    let [droid] = fleet.special_forces[..] else {
        panic!("the droid aboard: {:?}", fleet.special_forces);
    };
    assert_eq!(
        world.special_forces[droid].class_dat_id,
        rebellion_core::ids::DatId::new(0x3c00_0005)
    );
    assert!(applied.fleets.is_open(coruscant));
}

#[test]
fn the_a0_vjun_scenario_seats_admiral_bel_iblis_aboard_fleet_5() {
    // A0 fleet2/vjun-fleet5-admiral-personnel: the Admiral order renames
    // him "Admiral Bel Iblis" (FUN_004f17f0, TEXTSTRA name + 0x6c00).
    let request = request(Scenario::FleetWindowVjun, CockpitFaction::Alliance);
    let applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Vjun".into();
        world.systems[elsewhere].sector = world.systems[here].sector;
        world
            .capital_ship_classes
            .insert(rebellion_core::world::CapitalShipClass {
                name: "Corellian Corvette".into(),
                is_alliance: true,
                hull: 100,
                hyperdrive: 80,
                ..Default::default()
            });
        world.characters.insert(rebellion_core::world::Character {
            name: "Garm Bel Iblis".into(),
            command_names: [
                String::new(),
                "Commander Bel Iblis".into(),
                "Admiral Bel Iblis".into(),
                "General Bel Iblis".into(),
            ],
            ..Default::default()
        });
    });
    let world = &applied.world;
    let vjun = world.systems.keys().next().unwrap();
    let [fifth] = world.systems[vjun].fleets[..] else {
        panic!("one fleet at Vjun: {:?}", world.systems[vjun].fleets);
    };
    assert_eq!(world.fleet_name(fifth), Some("Fleet 5"));
    assert_eq!(world.ship_name(fifth, 0), Some("Corellian Corvette 2"));
    let [garm] = world.fleets[fifth].characters[..] else {
        panic!("one character aboard");
    };
    let garm = &world.characters[garm];
    assert_eq!(garm.name, "Admiral Bel Iblis");
    assert_eq!(garm.ship, Some(world.fleets[fifth].capital_ships[0].id));
    assert_ne!(garm.ship, Some(0));
    assert!(applied.fleets.is_open(vjun));
    assert!(applied.fleets.is_expanded(vjun, fifth));
    assert_eq!(
        applied
            .fleets
            .screen_rect(
                CockpitState::new(request.faction).layout_for(640.0, 480.0),
                vjun
            )
            .map(|rect| rect.min),
        Some(egui_macroquad::egui::pos2(305.0, 86.0))
    );
}

#[test]
fn the_a0_umgul_scenario_lists_fleet_1_damaged_and_fleet_3_loaded() {
    let request = request(Scenario::FleetWindowUmgul, CockpitFaction::Alliance);
    let applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Umgul".into();
        world.systems[elsewhere].sector = world.systems[here].sector;
        for (name, hull) in [("Corellian Corvette", 500), ("Medium Transport", 360)] {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: name.into(),
                    is_alliance: true,
                    hull,
                    hyperdrive: 100,
                    troop_capacity: 2,
                    ..Default::default()
                });
        }
    });
    let world = &applied.world;
    let umgul = world.systems.keys().next().unwrap();
    let names: Vec<_> = world.systems[umgul]
        .fleets
        .iter()
        .map(|&fleet| world.fleet_name(fleet).unwrap())
        .collect();
    assert_eq!(names, ["Fleet 1", "Fleet 3"]);
    let (first, third) = (
        world.systems[umgul].fleets[0],
        world.systems[umgul].fleets[1],
    );
    let hulls: Vec<_> = world.fleets[first]
        .capital_ships
        .iter()
        .map(|ship| ship.hull_current)
        .collect();
    assert_eq!(hulls, [240, 317]);
    let transport = &world.fleets[first].capital_ships[1];
    assert_eq!(
        rebellion_core::ship_condition::hyperdrive(
            &world.capital_ship_classes[transport.class],
            transport
        ),
        0
    );
    assert_eq!(world.fleets[third].capital_ships[0].hull_current, 359);
    assert_eq!(applied.transport.cargo(first).len(), 2);
    assert_eq!(applied.transport.cargo(third).len(), 2);
    assert!(world.systems[umgul].ground_units.is_empty());
    assert!(applied.fleets.is_expanded(umgul, first));
    assert_eq!(
        applied
            .fleets
            .screen_rect(
                CockpitState::new(request.faction).layout_for(640.0, 480.0),
                umgul
            )
            .map(|rect| rect.min),
        Some(egui_macroquad::egui::pos2(305.0, 86.0))
    );
}

#[test]
fn the_a0_selonia_scenario_selects_fleet_4_with_two_squadrons() {
    let request = request(Scenario::FleetWindowSelonia, CockpitFaction::Alliance);
    let applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Selonia".into();
        world.systems[elsewhere].sector = world.systems[here].sector;
        for (name, fighters) in [("Corellian Corvette", 0), ("Alliance Escort Carrier", 6)] {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: name.into(),
                    is_alliance: true,
                    hull: 500,
                    hyperdrive: 100,
                    fighter_capacity: fighters,
                    ..Default::default()
                });
        }
        for name in ["X-wing", "Y-wing"] {
            world
                .fighter_classes
                .insert(rebellion_core::world::FighterClass {
                    name: name.into(),
                    is_alliance: true,
                    ..Default::default()
                });
        }
    });
    let world = &applied.world;
    let selonia = world.systems.keys().next().unwrap();
    let listed = rebellion_core::movement::listed_fleets(&applied.movement, world, selonia);
    let names: Vec<_> = listed
        .iter()
        .map(|&fleet| world.fleet_name(fleet).unwrap())
        .collect();
    assert_eq!(names, ["Fleet 1", "Fleet 4"]);
    let (first, fourth) = (listed[0], listed[1]);
    assert!(applied.movement.is_in_transit(first));
    assert_eq!(world.systems[selonia].fleets, [fourth]);
    assert_eq!(world.fleets[first].capital_ships[0].hull_current, 240);
    let squadrons: Vec<_> = world.fleets[fourth]
        .fighters
        .iter()
        .map(|entry| {
            (
                world.fighter_classes[entry.class].name.as_str(),
                entry.count,
            )
        })
        .collect();
    assert_eq!(squadrons, [("X-wing", 1), ("Y-wing", 1)]);
    assert_eq!(
        world.ship_name(fourth, 0),
        Some("Alliance Escort Carrier 2")
    );
    assert!(applied.fleets.is_expanded(selonia, first));
    assert!(!applied.fleets.is_expanded(selonia, fourth));
}

#[test]
fn the_a0_yavin_scenario_selects_fleet_1_with_two_ships_and_two_regiments() {
    let request = request(Scenario::FleetWindowYavin, CockpitFaction::Alliance);
    let applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Yavin".into();
        world.systems[elsewhere].sector = world.systems[here].sector;
        for name in ["Corellian Corvette", "Medium Transport"] {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: name.into(),
                    is_alliance: true,
                    hull: 100,
                    hyperdrive: 100,
                    troop_capacity: 2,
                    ..Default::default()
                });
        }
    });
    let world = &applied.world;
    let yavin = world.systems.keys().next().unwrap();
    let [first] = world.systems[yavin].fleets[..] else {
        panic!("one fleet at Yavin: {:?}", world.systems[yavin].fleets);
    };
    assert_eq!(world.fleet_name(first), Some("Fleet 1"));
    assert_eq!(world.ship_name(first, 1), Some("Medium Transport 1"));
    assert_eq!(applied.transport.cargo(first).len(), 2);
    assert!(world.systems[yavin].ground_units.is_empty());
    assert!(applied.fleets.is_expanded(yavin, first));
}

#[test]
fn the_a0_destination_scenario_lists_fleet_8_en_route_at_qat_chrystac() {
    let request = request(Scenario::FleetWindowDestination, CockpitFaction::Alliance);
    let applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, there) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Yavin".into();
        world.systems[there].name = "Qat Chrystac".into();
        world.systems[there].sector = world.systems[here].sector;
        for name in ["Corellian Corvette", "Medium Transport"] {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: name.into(),
                    is_alliance: true,
                    hull: 100,
                    hyperdrive: 100,
                    troop_capacity: 2,
                    ..Default::default()
                });
        }
    });
    let world = &applied.world;
    let mut keys = world.systems.keys();
    let (yavin, qat) = (keys.next().unwrap(), keys.next().unwrap());
    let listed = rebellion_core::movement::listed_fleets(&applied.movement, world, qat);
    assert_eq!(listed.len(), 1);
    let fleet = listed[0];
    assert_eq!(world.fleet_name(fleet), Some("Fleet 8"));
    assert!(applied.movement.is_in_transit(fleet));
    assert_eq!(applied.transport.cargo(fleet).len(), 2);
    assert!(applied.fleets.is_open(qat));
    assert!(!applied.fleets.is_open(yavin));
}

#[test]
fn the_a0_blockade_scenario_puts_fleet_8_damaged_over_imperial_bortras() {
    let request = request(Scenario::FleetWindowBlockade, CockpitFaction::Alliance);
    let applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, there) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Yavin".into();
        world.systems[there].name = "Bortras".into();
        world.systems[there].sector = world.systems[here].sector;
        for name in ["Corellian Corvette", "Medium Transport"] {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: name.into(),
                    is_alliance: true,
                    hull: 100,
                    hyperdrive: 100,
                    troop_capacity: 2,
                    ..Default::default()
                });
        }
    });
    let world = &applied.world;
    let mut keys = world.systems.keys();
    let (_, bortras) = (keys.next().unwrap(), keys.next().unwrap());
    let [fleet] = world.systems[bortras].fleets[..] else {
        panic!("one fleet at Bortras");
    };
    assert_eq!(world.fleet_name(fleet), Some("Fleet 8"));
    assert_eq!(
        world.systems[bortras].control,
        ControlKind::Controlled(Faction::Empire)
    );
    assert_eq!(world.fleets[fleet].capital_ships[0].hull_current, 50);
    assert!(!applied.movement.is_in_transit(fleet));
    assert_eq!(applied.transport.cargo(fleet).len(), 2);
    assert!(applied.fleets.is_open(bortras));
    // The player's fleet alone over the other side's system blockades
    // it (A0 fleet4/05: "Bortras is now under blockade by Alliance
    // forces"); the sector-window case shows it so.
    assert!(BlockadeSystem::system_is_blockaded(
        world,
        &world.systems[bortras]
    ));
}

#[test]
fn the_training_scenario_puts_a0s_regiments_part_made_in_the_primarys_training_queue() {
    use rebellion_core::manufacturing::{BuildableKind, ProductionArea};
    for (faction, raw, count, points) in [
        (CockpitFaction::Alliance, 0x1000_0002, 1, 1),
        (CockpitFaction::Empire, 0x1000_0008, 3, 3),
    ] {
        let is_alliance = faction == CockpitFaction::Alliance;
        let mut world = fleet_world();
        let primary = world.systems.keys().next().unwrap();
        for (class, rate) in [(raw, 0), (0x2900_0002, 4)] {
            world.buildable_classes.insert(
                rebellion_core::ids::DatId::new(class),
                rebellion_core::world::BuildableClass {
                    refined_material_cost: 6,
                    processing_rate: rate,
                    ..Default::default()
                },
            );
        }
        let yard = world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2900_0002),
                side: Faction::of_alliance(is_alliance),
                is_shipyard: false,
            },
        );
        world.systems[primary].manufacturing_facilities.push(yard);
        let mut manufacturing = ManufacturingState::new();
        place_training(
            request(Scenario::BuildSelectionTraining, faction),
            &world,
            &mut manufacturing,
            primary,
        );
        let queue = manufacturing
            .queue(primary, ProductionArea::TrainingFacility)
            .expect("a training queue");
        assert_eq!(queue.len(), count, "{faction:?}");
        assert_eq!(queue.completed_points(), points, "{faction:?}");
        assert_eq!(
            queue.active().map(|item| (item.kind, item.cost)),
            Some((
                BuildableKind::Troop(rebellion_core::ids::DatId::new(raw)),
                6
            ))
        );
    }
}

#[test]
fn the_moddell_scenario_leaves_a0s_four_systems_explored_uninhabited_and_unheld() {
    let mut world = fleet_world();
    let keys: Vec<_> = world.systems.keys().collect();
    world.systems[keys[0]].name = "Agrilat".into();
    world.systems[keys[1]].name = "Annaj".into();
    world.systems[keys[0]].is_populated = true;
    world.systems[keys[0]].control = ControlKind::Controlled(Faction::Alliance);
    world.systems[keys[2]].is_populated = true;
    let other = world.systems[keys[2]].control;
    place_moddell_uninhabited(&mut world);
    for (key, energy, raw) in [(keys[0], 5, 5), (keys[1], 8, 4)] {
        let system = &world.systems[key];
        assert_eq!(system.exploration_status, ExplorationStatus::Explored);
        assert!(!system.is_populated);
        assert_eq!(system.control, ControlKind::Uncontrolled);
        assert_eq!((system.total_energy, system.raw_materials), (energy, raw));
        assert!(system.defense_facilities.is_empty() && system.production_facilities.is_empty());
    }
    // A system A0 does not show uninhabited keeps its state.
    assert!(world.systems[keys[2]].is_populated);
    assert_eq!(world.systems[keys[2]].control, other);
}

#[test]
fn the_a0_empty_scenario_opens_umgul_with_no_fleet_there() {
    let request = request(Scenario::FleetWindowEmpty, CockpitFaction::Alliance);
    let applied = apply_to_loading_world_with(request, |world| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[here].name = "Umgul".into();
        world.systems[elsewhere].sector = world.systems[here].sector;
    });
    let world = &applied.world;
    let umgul = world.systems.keys().next().unwrap();
    assert!(world.systems[umgul].fleets.is_empty());
    assert!(world.fleets.values().all(|fleet| fleet.location != umgul));
    assert!(applied.fleets.is_open(umgul));
    assert_eq!(
        applied
            .fleets
            .screen_rect(
                CockpitState::new(request.faction).layout_for(640.0, 480.0),
                umgul
            )
            .map(|rect| rect.min),
        Some(egui_macroquad::egui::pos2(305.0, 86.0))
    );
}

#[test]
fn the_unfinished_ship_scenarios_open_its_fleet_unfinished_then_on_its_trip() {
    // A0 2026-11-15 "m-1" and "m-4" (the scenarios' docs).
    let setup = |world: &mut GameWorld| {
        let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
        let (corfai, talus, other) = (
            keys.next().unwrap(),
            keys.next().unwrap(),
            keys.next().unwrap(),
        );
        world.systems[corfai].name = "Corfai".into();
        world.systems[talus].name = "Talus".into();
        world.systems[talus].sector = world.systems[corfai].sector;
        world.systems[other].sector = world.systems[corfai].sector;
        for (index, name) in ["Carrack Light Cruiser", "Imperial Dreadnaught", "Galleon"]
            .iter()
            .enumerate()
        {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    dat_id: rebellion_core::ids::DatId::new(0x1400_0100 + index as u32),
                    name: (*name).into(),
                    is_alliance: false,
                    hull: 100,
                    ..Default::default()
                });
        }
    };
    let request = request(Scenario::UnfinishedShipMini, CockpitFaction::Empire);
    let applied = apply_to_loading_world_with(request, setup);
    let world = &applied.world;
    let corfai = world.systems.keys().next().unwrap();
    let product = &world.products.all()[0];
    let rebellion_core::products::Container::Fleet(spare) = product.container else {
        panic!("the Carrack waits in a fleet");
    };
    assert_eq!(world.fleets[spare].location, corfai);
    assert!(world.products.arrival(product.id).is_none());
    assert!(applied.fleets.is_open(corfai));
    assert_eq!(
        applied
            .fleets
            .screen_rect(
                CockpitState::new(request.faction).layout_for(640.0, 480.0),
                corfai
            )
            .map(|rect| rect.min),
        Some(egui_macroquad::egui::pos2(174.0, 91.0))
    );

    let request = super::tests::request(Scenario::UnfinishedShipEnroute, CockpitFaction::Empire);
    let applied = apply_to_loading_world_with(request, setup);
    let world = &applied.world;
    let talus = world.systems.keys().nth(1).unwrap();
    let product = &world.products.all()[0];
    let rebellion_core::products::Container::Fleet(fleet) = product.container else {
        panic!("the Carrack stands in a fleet");
    };
    assert_eq!(world.fleets[fleet].location, talus);
    assert_eq!(world.fleets[fleet].capital_ships.len(), 2);
    assert!(world.products.arrival(product.id).is_some());
    assert!(applied.fleets.is_open(talus));
}

// The primary's room in the two refusing Build Selection scenarios
// (FUN_005385f0 -> FUN_00551a20 -> FUN_00507750): over maintenance it
// keeps a free energy slot and raw deposit, so a mine and a refinery,
// which FUN_0052e970 passes for costing no maintenance, stay
// buildable. In BuildSelectionFacilityRoom only the slot of the unit
// queued on its own facilities band is left, which FUN_00552210 gives
// back to an order on that band: one facility fits ({0x90, 1} at
// two), a mine finds no raw deposit ({0x90, 2}).
#[test]
fn the_refusing_build_selection_scenarios_leave_the_primary_its_traced_room() {
    use rebellion_core::manufacturing::{BuildableKind, FacilityBuild, ProductionArea};
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let is_alliance = faction == CockpitFaction::Alliance;
        let facility = |raw: u32| {
            let build = FacilityBuild {
                class: rebellion_core::ids::DatId::new(raw),
                is_alliance,
            };
            match raw >> 24 {
                0x2c | 0x2d => BuildableKind::ProductionFacility(build),
                0x22..=0x27 => BuildableKind::DefenseFacility(build),
                _ => BuildableKind::ManufacturingFacility(build),
            }
        };
        let (yard, mine, refinery, shield) = (
            facility(0x2a00_0001),
            facility(0x2c00_0001),
            facility(0x2d00_0002),
            facility(0x2500_0004),
        );
        for (scenario, queued, cases) in [
            (
                Scenario::BuildSelectionOverMaintenance,
                0,
                [
                    (yard, 1, true),
                    (shield, 1, true),
                    (mine, 1, true),
                    (refinery, 1, true),
                    (yard, 2, false),
                ],
            ),
            (
                Scenario::BuildSelectionFacilityRoom,
                1,
                [
                    (yard, 1, true),
                    (shield, 1, true),
                    (refinery, 1, true),
                    (mine, 1, false),
                    (yard, 2, false),
                ],
            ),
        ] {
            // The other side's mine and refinery at the primary take
            // an energy slot each, the mine a raw deposit.
            let other = rebellion_core::dat::Faction::of_alliance(!is_alliance);
            let applied = apply_to_loading_world_with(request(scenario, faction), |world| {
                let here = world.systems.keys().next().unwrap();
                for (raw, is_mine) in [(0x2c00_0001, true), (0x2d00_0002, false)] {
                    let key = world.production_facilities.insert(
                        rebellion_core::world::ProductionFacilityInstance {
                            class_dat_id: rebellion_core::ids::DatId::new(raw),
                            side: other,
                            is_mine,
                        },
                    );
                    world.systems[here].production_facilities.push(key);
                }
            });
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            let band = (primary, ProductionArea::ConstructionYard);
            let kinds: Vec<_> = applied
                .manufacturing
                .queue(band.0, band.1)
                .map(|queue| queue.items().iter().map(|item| item.kind).collect())
                .unwrap_or_default();
            assert_eq!(kinds, vec![yard; queued]);
            for (kind, count, fits) in cases {
                assert_eq!(
                    !rebellion_core::build_selection::lacks_facility_room(
                        world,
                        &applied.manufacturing,
                        &[],
                        band,
                        primary,
                        kind,
                        count,
                    ),
                    fits,
                    "{scenario:?} {faction:?} {kind:?} x{count}"
                );
            }
            // Another band's order gets no slot back.
            if queued == 1 {
                assert!(rebellion_core::build_selection::lacks_facility_room(
                    world,
                    &applied.manufacturing,
                    &[],
                    (primary, ProductionArea::Shipyard),
                    primary,
                    yard,
                    1,
                ));
            }
        }
    }
}

// A0 manufacturing/11: the Empire at -402 sees "n/a" and a grey
// Confirm; FUN_0052e970 refuses what the side's unallocated
// maintenance cannot take.
#[test]
fn the_over_maintenance_scenario_leaves_the_side_no_capacity_and_its_yards() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let side = rebellion_core::dat::Faction::of_alliance(faction == CockpitFaction::Alliance);
        let request = request(Scenario::BuildSelectionOverMaintenance, faction);
        let applied = apply_to_loading_world_with(request, |world| {
            let here = world.systems.keys().next().unwrap();
            let other =
                rebellion_core::dat::Faction::of_alliance(faction != CockpitFaction::Alliance);
            for (is_mine, holder) in [(true, side), (false, side), (true, other)] {
                let key = world.production_facilities.insert(
                    rebellion_core::world::ProductionFacilityInstance {
                        class_dat_id: rebellion_core::ids::DatId::new(0x2c00_0001),
                        side: holder,
                        is_mine,
                    },
                );
                world.systems[here].production_facilities.push(key);
            }
        });
        let world = &applied.world;
        assert!(world
            .production_facilities
            .values()
            .all(|facility| facility.side != side));
        assert_eq!(world.production_facilities.len(), 1);
        assert!(world
            .systems
            .values()
            .flat_map(|system| &system.production_facilities)
            .all(|key| world.production_facilities.contains_key(*key)));
        let totals = rebellion_core::side_resources::maintenance_totals(
            world,
            &ManufacturingState::new(),
            &[],
            side,
            |_| false,
        );
        assert_eq!(totals.unallocated(), 0);
        let yards = world
            .manufacturing_facilities
            .values()
            .filter(|yard| yard.side == side)
            .count();
        assert!(yards >= 3);
    }
}

// A0 build-selection2/21: a band's Destination is released on a planet
// in a sector window; the gate needs the secondary's planet on screen.
#[test]
fn the_destination_scenario_shows_the_secondary_planet_beside_the_open_window() {
    let request = request(Scenario::BuildSelectionDestination, CockpitFaction::Empire);
    let applied = apply_to_loading_world_with(request, |_| {});
    let world = &applied.world;
    let mut keys = world.systems.keys();
    let (primary, secondary) = (keys.next().unwrap(), keys.next().unwrap());
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let planet = applied
        .sectors
        .planet_screen_rect(world, layout, secondary)
        .unwrap();
    let band = applied
        .systems
        .band_screen_rect(
            layout,
            primary,
            rebellion_core::manufacturing::ProductionArea::Shipyard,
        )
        .unwrap();
    // The planet is not under the window.
    assert!(planet.center().x < band.min.x - 55.0, "{planet:?} {band:?}");
    assert!(applied
        .systems
        .band_screen_rect(
            layout,
            primary,
            rebellion_core::manufacturing::ProductionArea::TrainingFacility
        )
        .is_some());
}

// FUN_0048bc60: the EventMessages scenario files a battle report on its
// reader's Conflict rail (A0 messages-day633/index-tab7).
#[test]
fn the_event_messages_scenario_files_a_conflict_report_for_its_reader() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let mut log = rebellion_render::MessageLog::new();
        post_conflict_report(request(Scenario::EventMessages, faction), &mut log);
        let [idle, older, newer] = log.messages() else {
            panic!("{:?}", log.messages());
        };
        assert_eq!(
            idle.title.as_deref(),
            Some("Construction Yard Idle on Womrik")
        );
        assert_eq!(
            idle.rail,
            Some(rebellion_render::MessageRail::Manufacturing)
        );
        assert_eq!(older.title.as_deref(), Some("Assault on Umgul"));
        assert_eq!(newer.title.as_deref(), Some("Assault on Coruscant"));
        for message in [older, newer] {
            assert_eq!(message.rail, Some(rebellion_render::MessageRail::Conflict));
        }
        for message in [idle, older, newer] {
            assert!(message.unread);
            assert_eq!(
                message.audience,
                rebellion_render::message_log::RailAudience::side(
                    faction == CockpitFaction::Alliance
                )
            );
        }
        let mut none = rebellion_render::MessageLog::new();
        post_conflict_report(request(Scenario::StoryMessages, faction), &mut none);
        assert!(none.messages().is_empty());
    }
}

// FUN_004981c0 case 1: A0 events-day673/04 shows the Alliance "Fleet
// Arrives at Sluis Van" for Fleet 1; the scenario names that pair for the
// Alliance reader only.
#[test]
fn the_event_messages_scenario_brings_fleet_1_to_sluis_van_for_the_alliance() {
    let mut world = fleet_world();
    let sluis = world.systems.keys().nth(1).unwrap();
    world.systems[sluis].name = "Sluis Van".into();
    let alliance = request(Scenario::EventMessages, CockpitFaction::Alliance);
    assert_eq!(arrived_fleet(alliance, &world), None, "no Fleet 1 yet");
    let fleet = world.insert_fleet(rebellion_core::world::Fleet {
        location: sluis,
        is_alliance: true,
        ..Default::default()
    });
    assert!(world.rename_fleet(fleet, "Fleet 1"));
    assert_eq!(arrived_fleet(alliance, &world), Some((fleet, sluis)));
    assert_eq!(
        arrived_fleet(
            request(Scenario::EventMessages, CockpitFaction::Empire),
            &world
        ),
        None
    );
    assert_eq!(
        arrived_fleet(
            request(Scenario::StoryMessages, CockpitFaction::Alliance),
            &world
        ),
        None
    );
}

// A0 2026-11-17-edited-saves-6 d-07 (sha256 in its SHA256SUMS): the
// Alliance's All Messages newest first; 2026-11-18-edited-saves-9 a-07:
// Chewbacca Captured in the Empire cockpit. Our own fixture's order.
#[test]
fn the_capture_messages_scenario_files_d07s_rows_and_the_empires_capture_of_chewbacca() {
    let texts: std::collections::HashMap<u16, String> = [
        (0x7039, "{1:1} Joins Enemy"),
        (0x703c, "{1:1} has decided to join the {2:1}."),
        (0x713e, "Enemy Mission Foiled"),
        (
            0x713f,
            "An enemy {1:1} mission to {2:1} has been foiled by our forces.",
        ),
        (0x7158, "{1:1} Captured"),
        (0x7159, "{1:1} was captured by the {3:1} at {2:1}."),
        (0x715a, "{1:1} Captured"),
        (0x715b, "{1:1} was captured at {2:1}."),
        (0x7160, "{1:1} Injured"),
        (0x7161, "{1:1} has been injured."),
        (0x71b8, "Luke Goes to Dagobah"),
        (
            0x71b9,
            "Luke has been sent to Dagobah to be trained by Yoda.",
        ),
        (0x4d50, "Alliance"),
        (0x4d51, "Empire"),
    ]
    .into_iter()
    .map(|(id, text)| (id, text.to_owned()))
    .collect();
    let prepare = |world: &mut GameWorld| {
        let keys: Vec<_> = world.systems.keys().take(3).collect();
        for (key, name) in keys.into_iter().zip(["Yavin", "Corfai", "Vagran"]) {
            world.systems[key].name = name.into();
        }
        for name in ["Han Solo", "Leia Organa", "Chewbacca"] {
            world.characters.insert(rebellion_core::world::Character {
                name: name.into(),
                is_alliance: true,
                is_major: true,
                ..Default::default()
            });
        }
    };
    let titles = |faction| {
        let request = request(Scenario::CaptureMessages, faction);
        let mut applied = apply_to_loading_world_with(request, prepare);
        let mut log = rebellion_render::MessageLog::new();
        post_capture_messages(request, &mut applied.world, &mut log, &texts);
        let han = applied
            .world
            .characters
            .values()
            .find(|c| c.name == "Han Solo");
        assert_eq!(han.map(|c| c.injury), Some(0), "the world keeps Han unhurt");
        log.messages()
            .iter()
            .rev()
            .map(|message| {
                (
                    message.title.clone().unwrap_or_default(),
                    message.text.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    let alliance = titles(CockpitFaction::Alliance);
    assert_eq!(
        alliance
            .iter()
            .map(|(title, _)| title.as_str())
            .collect::<Vec<_>>(),
        [
            "Corfai Joins Enemy",
            "Vagran Joins Enemy",
            "Luke Goes to Dagobah",
            "Enemy Mission Foiled",
            "Han Solo Captured",
            "Han Solo Injured",
            "Enemy Mission Foiled",
        ]
    );
    // d-08, d-09, d-10 word for word.
    assert_eq!(
        alliance[4].1,
        "Han Solo was captured by the Empire at Yavin."
    );
    assert_eq!(alliance[5].1, "Han Solo has been injured.");
    assert_eq!(
        alliance[3].1,
        "An enemy Espionage mission to Yavin has been foiled by our forces."
    );
    // FUN_00490340 case 4 for the captor (0x715a / 0x715b), as A0
    // 2026-11-18-edited-saves-9 a-07 (0510b22a2f72, an unedited Empire
    // game) reads it.
    assert_eq!(
        titles(CockpitFaction::Empire),
        [(
            "Chewbacca Captured".to_owned(),
            "Chewbacca was captured at Yavin.".to_owned()
        )]
    );
    assert_eq!(Scenario::decode(142), Some(Scenario::CaptureMessages));
}

#[test]
fn jabba_delivers_scenario_shows_each_reader_its_a0_background() {
    // FUN_0048b6e0: 0x42b when Luke (0x32000242) holds DagobahCompleted,
    // else 0x42a. A0 2026-11-18-edited-saves-9 c-13 (8aa20451a9f9, an
    // Empire reader whose Luke had left Dagobah) shows STRATEGY 1067 =
    // 0x42b; 2026-11-18-edited-saves-7 b-12 (ec2681b5c72b, an Alliance
    // reader, Luke untrained) 1066 = 0x42a.
    let texts: std::collections::HashMap<u16, String> = [
        (0x7200, "Jabba Delivers Prisoners"),
        (
            0x7201,
            "Jabba has deliverd Luke, Han, Chewbacca and Leia to Darth Vader. They are now prisoners of the Empire.",
        ),
    ]
    .into_iter()
    .map(|(id, text)| (id, text.to_owned()))
    .collect();
    let prepare = |world: &mut GameWorld| {
        world.characters.insert(rebellion_core::world::Character {
            dat_id: rebellion_core::ids::DatId::new(0x3200_0242),
            name: "Luke Skywalker".into(),
            is_alliance: true,
            is_major: true,
            ..Default::default()
        });
    };
    let background = |faction| {
        let request = request(Scenario::JabbaDelivers, faction);
        let mut applied = apply_to_loading_world_with(request, prepare);
        let mut log = rebellion_render::MessageLog::new();
        post_jabba_delivers(request, &mut applied.world, &mut log, &texts);
        let messages = log.messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].title.as_deref(),
            Some("Jabba Delivers Prisoners")
        );
        messages[0].picture.map(|picture| picture.background)
    };
    assert_eq!(background(CockpitFaction::Empire), Some(0x42b));
    assert_eq!(background(CockpitFaction::Alliance), Some(0x42a));
    assert_eq!(Scenario::decode(151), Some(Scenario::JabbaDelivers));
}

/// GNPRTB 7684 and 7685 as shipped, 5 and 2
/// (`ghidra/notes/blockade-troop-withdrawal.md`, `FUN_00558bb0`).
fn shipped_withdraw_penalties(world: &mut GameWorld) {
    let entry = |id: u16, value: i32| rebellion_core::world::GnprtbEntry {
        parameter_id: u32::from(id),
        development: value,
        alliance_sp_easy: value,
        alliance_sp_medium: value,
        alliance_sp_hard: value,
        empire_sp_easy: value,
        empire_sp_medium: value,
        empire_sp_hard: value,
        multiplayer: value,
    };
    world.gnprtb = rebellion_core::world::GnprtbParams::new(vec![
        entry(
            rebellion_core::blockade::GNPRTB_WITHDRAW_PENALTY_PER_CAPSHIP,
            5,
        ),
        entry(
            rebellion_core::blockade::GNPRTB_WITHDRAW_PENALTY_PER_FIGHTER,
            2,
        ),
    ]);
}

#[test]
fn the_blockade_running_scenario_rolls_a_regiment_leaving_on_its_own_not_the_fleets_cargo() {
    // FUN_0050b310: a blockaded system without a KDY-150 has the
    // withdraw percent FUN_0055a020 gives, 100 - 20 ships * 5 = 0, which
    // a regiment whose own Move brought it there copies (FUN_00514a60 ->
    // FUN_00508660 -> FUN_0050c540). FUN_00504990 rolls only the moved
    // object (FUN_00556430's in-transit bit): the regiment that leaves on
    // its own, kept only when random(0..=99) < 0, never; the cargo of a
    // departing fleet is not rolled (ghidra/notes/move-order.md,
    // "Re-traced 2026-11-19"; A0 2026-11-19-blockade-15 d-04..d-21).
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let request = request(Scenario::BlockadeRunning, faction);
        let mut applied = apply_to_loading_world_with(request, shipped_withdraw_penalties);
        let mut systems = applied.world.systems.keys();
        let primary = systems.next().unwrap();
        let target = loading_target(&applied.world, primary).unwrap();
        let fleet = applied.world.fleets.keys().next().unwrap();
        let alliance = faction == CockpitFaction::Alliance;

        assert!(request.scenario.uses_the_fleet_window());
        assert!(!runs_the_clock(request));
        assert_eq!(
            applied.world.systems[primary].control,
            ControlKind::Controlled(if alliance {
                Faction::Empire
            } else {
                Faction::Alliance
            })
        );
        assert!(applied.blockade.is_blockaded(primary), "{faction:?}");
        // Held by the other side's support (FUN_0050b610).
        let held = &applied.world.systems[primary];
        let (enemy_support, own_support) = if alliance {
            (held.popularity_empire, held.popularity_alliance)
        } else {
            (held.popularity_alliance, held.popularity_empire)
        };
        assert!((enemy_support - 1.0).abs() < f32::EPSILON, "{faction:?}");
        assert!(own_support.abs() < f32::EPSILON, "{faction:?}");
        assert!(applied.world.systems[primary].defense_facilities.is_empty());
        assert_eq!(applied.world.fleets[fleet].capital_ships.len(), 20);
        let cargo = applied.transport.cargo(fleet).to_vec();
        assert_eq!(cargo.len(), 2, "{faction:?}");
        for &troop in &cargo {
            assert_eq!(
                applied
                    .blockade
                    .regiment_withdraw(troop)
                    .map(|value| value.withdraw_percent),
                Some(0)
            );
        }

        // The first regiment leaves on its own for the target.
        let blockaded = applied.blockade.blockaded_systems().clone();
        let moved = applied.transport.move_regiment(
            &mut applied.world,
            &applied.movement,
            &blockaded,
            cargo[0],
            rebellion_core::troop_transport::RegimentTarget::System(target),
            2,
        );
        assert!(
            matches!(
                moved,
                Ok(rebellion_core::troop_transport::RegimentMove::Departed(_))
            ),
            "{moved:?}"
        );
        let running = BlockadeSystem::running_regiments(
            &mut applied.blockade,
            &applied.world,
            &applied.transport,
        );
        // Roll 0.0 is the best draw, 99 (BlockadeSystem::resolve_running).
        assert_eq!(
            BlockadeSystem::resolve_running(&running, &[0.0], 2),
            [rebellion_core::blockade::BlockadeEvent::TroopDestroyed {
                system: primary,
                troop: cargo[0],
                tick: 2,
            }]
        );

        // The fleet leaves with the second: nothing rolls.
        assert!(applied.movement.order(fleet, primary, target, 3));
        assert!(BlockadeSystem::running_regiments(
            &mut applied.blockade,
            &applied.world,
            &applied.transport,
        )
        .is_empty());
        assert_eq!(applied.transport.cargo(fleet), [cargo[1]]);
    }
    assert_eq!(Scenario::decode(152), Some(Scenario::BlockadeRunning));
}

#[test]
fn the_blockade_running_report_keeps_the_players_last_evacuation_losses() {
    // Our report: the reader's latest line of either form.
    let line = |text: &str, alliance: bool| {
        let mut message = GameMessage::new(1, text, rebellion_render::MessageCategory::Combat)
            .with_title("Evacuation Losses");
        message.audience = rebellion_render::message_log::RailAudience::side(alliance);
        message
    };
    let request = request(Scenario::BlockadeRunning, CockpitFaction::Alliance);
    let messages = [
        line("Troops destroyed by blockade at Mine", true),
        line(
            "The following units were lost running an enemy blockade of Theirs.",
            false,
        ),
        line("Fleet 3 has initiated a blockade of Mine.", true),
    ];
    assert_eq!(
        last_loss(request, &messages).map(|message| message.text.as_str()),
        Some("Troops destroyed by blockade at Mine")
    );
    let ours = [
        line("Troops destroyed by blockade at Mine", true),
        line(
            "The following units were lost running an enemy blockade of Mine.",
            true,
        ),
    ];
    assert_eq!(
        last_loss(request, &ours).map(|message| message.text.as_str()),
        Some("The following units were lost running an enemy blockade of Mine.")
    );
    assert!(last_loss(request, &messages[1..]).is_none());
}

#[test]
fn the_squadron_blockaded_scenario_refuses_the_squadrons_move_to_the_target() {
    // FUN_005073d0: a blockaded system refuses a squadron's 0x204
    // with 0x90/4 (squadrons::check_route).
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let request = request(Scenario::SquadronBlockaded, faction);
        let applied = apply_to_loading_world_with(request, |world| {
            for alliance in [true, false] {
                world
                    .fighter_classes
                    .insert(rebellion_core::world::FighterClass {
                        is_alliance: alliance,
                        is_empire: !alliance,
                        ..Default::default()
                    });
            }
        });
        let primary = applied.world.systems.keys().next().unwrap();
        let target = loading_target(&applied.world, primary).unwrap();
        let alliance = faction == CockpitFaction::Alliance;
        assert!(request.scenario.uses_the_fleet_window());
        assert!(applied.blockade.is_blockaded(target), "{faction:?}");
        assert!(!applied.blockade.is_blockaded(primary), "{faction:?}");
        let class = applied.world.systems[primary].fighters[0].class;
        assert_eq!(
            applied.transport.squadrons.check(
                &applied.world,
                alliance,
                class,
                1,
                rebellion_core::squadrons::SquadronPlace::Planet(primary),
                rebellion_core::squadrons::SquadronPlace::Planet(target),
            ),
            Err(rebellion_core::squadrons::SquadronMoveError::DestinationBlockaded)
        );
    }
    assert_eq!(Scenario::decode(154), Some(Scenario::SquadronBlockaded));
}

#[test]
fn the_blockade_running_scenario_needs_one_ship_without_a_penalty() {
    // Our fixture: with no GNPRTB 7684 the percent stays 100 whatever
    // the ships, so the fleet keeps its one.
    let request = request(Scenario::BlockadeRunning, CockpitFaction::Alliance);
    let applied = apply_to_loading_world_with(request, |_| {});
    let fleet = applied.world.fleets.keys().next().unwrap();
    assert_eq!(applied.world.fleets[fleet].capital_ships.len(), 1);
}

#[test]
fn the_captive_transport_scenario_holds_a_captive_beside_its_jailer() {
    // Manual p. 96: the captor moves a captive with a jailer.
    // FUN_0054bb90 refuses the captive alone (0x40/0x91); the pair's
    // 0x241 keeps the agent in its team.
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let request = request(Scenario::CaptiveTransport, faction);
        let applied = apply_to_loading_world_with(request, |world| {
            world.characters.insert(rebellion_core::world::Character {
                name: "Admiral".into(),
                can_be_admiral: true,
                ..Default::default()
            });
            for name in ["Prisoner A", "Prisoner B"] {
                world.characters.insert(rebellion_core::world::Character {
                    name: name.into(),
                    is_alliance: faction != CockpitFaction::Alliance,
                    is_empire: faction == CockpitFaction::Alliance,
                    ..Default::default()
                });
            }
        });
        let primary = applied.world.systems.keys().next().unwrap();
        let alliance = faction == CockpitFaction::Alliance;
        let player = if alliance {
            MissionFaction::Alliance
        } else {
            MissionFaction::Empire
        };
        assert!(request.scenario.reports_object_menu());
        let here: Vec<_> = applied
            .world
            .characters
            .iter()
            .filter(|(_, c)| c.current_system == Some(primary))
            .map(|(key, _)| key)
            .collect();
        let [agent, captive] = here[..] else {
            panic!("{faction:?}: {here:?}");
        };
        assert!(!applied.world.characters[agent].is_captive);
        assert!(applied.world.characters[captive].is_captive);
        let state = &applied.missions;
        let member = MissionMember::Character;
        assert!(!state.move_order_enabled(&applied.world, player, &[member(captive)]));
        assert!(state.move_order_enabled(
            &applied.world,
            player,
            &[member(agent), member(captive)]
        ));
    }
    assert_eq!(Scenario::decode(160), Some(Scenario::CaptiveTransport));
}

#[test]
fn the_character_evacuation_scenario_blockades_the_agents_own_system() {
    // FUN_0050b8e0: a system held by one side with only the other's
    // fleet in orbit is blockaded; FUN_0049a350 then warns a member
    // leaving it (move_confirmation.rs EVACUATION).
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let request = request(Scenario::CharacterEvacuation, faction);
        let applied = apply_to_loading_world_with(request, |world| {
            world.characters.insert(rebellion_core::world::Character {
                name: "Admiral".into(),
                can_be_admiral: true,
                ..Default::default()
            });
        });
        let primary = applied.world.systems.keys().next().unwrap();
        let alliance = faction == CockpitFaction::Alliance;
        let player = if alliance {
            Faction::Alliance
        } else {
            Faction::Empire
        };

        assert!(request.scenario.reports_object_menu());
        assert_eq!(
            applied.world.systems[primary].control,
            ControlKind::Controlled(player)
        );
        assert!(applied.blockade.is_blockaded(primary), "{faction:?}");
        let orbiting = &applied.world.systems[primary].fleets;
        assert_eq!(orbiting.len(), 1, "{faction:?}");
        assert_eq!(
            Some(&orbiting[0]),
            applied.world.fleets.keys().next().as_ref()
        );
        assert_ne!(applied.world.fleets[orbiting[0]].is_alliance, alliance);
        // The Move's target is the player's own (FUN_004f4b60).
        let secondary = applied.world.systems.keys().nth(1).unwrap();
        assert_eq!(
            applied.world.systems[secondary].control,
            ControlKind::Controlled(player)
        );
        assert!(applied.world.characters.values().any(|character| {
            character.current_system == Some(primary)
                && character.current_fleet.is_none()
                && character.recruited
                && !character.on_mission
                && character.is_alliance == alliance
        }));
    }
    assert_eq!(Scenario::decode(153), Some(Scenario::CharacterEvacuation));
}

// FUN_00490340: A0 alliance-game2/han-solo-captured and
// empire-game3/ozzel-captured show each side's own character captured
// (0x7158 / 0x7159); the scenario files that for its reader, under the
// Emperor's capture.
#[test]
fn the_event_messages_scenario_files_the_readers_own_capture() {
    let texts: std::collections::HashMap<u16, String> = [
        (0x7158, "{1:1} Captured"),
        (0x7159, "{1:1} was captured by the {3:1} at {2:1}."),
        (0x715a, "{1:1} Captured"),
        (0x715b, "{1:1} was captured at {2:1}."),
        (0x4d50, "Alliance"),
        (0x4d51, "Empire"),
    ]
    .into_iter()
    .map(|(id, text)| (id, text.to_owned()))
    .collect();
    for (faction, name, alliance, text) in [
        (
            CockpitFaction::Alliance,
            "Han Solo",
            true,
            "Han Solo was captured by the Empire at Yavin.",
        ),
        (
            CockpitFaction::Empire,
            "Ozzel",
            false,
            "Ozzel was captured by the Alliance at Tralus.",
        ),
    ] {
        let request = request(Scenario::EventMessages, faction);
        let applied = apply_to_loading_world_with(request, |world| {
            world.characters.insert(rebellion_core::world::Character {
                name: name.into(),
                is_alliance: alliance,
                is_empire: !alliance,
                ..Default::default()
            });
        });
        let mut log = rebellion_render::MessageLog::new();
        post_own_capture_message(request, &applied.world, &mut log, &texts);
        let [message] = log.messages() else {
            panic!("{:?}", log.messages());
        };
        assert_eq!(message.text, text);
        let mut none = rebellion_render::MessageLog::new();
        post_own_capture_message(
            super::tests::request(Scenario::StoryMessages, faction),
            &applied.world,
            &mut none,
            &texts,
        );
        assert!(none.messages().is_empty());
    }
}

// FUN_00499760: A0 message-index/category-1-day228 lists the Alliance's
// systems joining the Empire; A0 empire-game3/joins-enemy shows the
// Empire's Commenor joining the Alliance. Each reader gets its own.
#[test]
fn the_event_messages_scenario_joins_systems_to_the_readers_enemy() {
    let alliance = request(Scenario::EventMessages, CockpitFaction::Alliance);
    let empire = request(Scenario::EventMessages, CockpitFaction::Empire);
    assert_eq!(
        systems_joining_the_empire(alliance),
        vec![("Corellia", 0), ("Commenor", 0)]
    );
    assert!(systems_joining_the_empire(empire).is_empty());
    assert_eq!(systems_joining_the_alliance(empire), vec![("Commenor", 0)]);
    assert!(systems_joining_the_alliance(alliance).is_empty());
    assert!(
        systems_joining_the_alliance(request(Scenario::StoryMessages, CockpitFaction::Empire))
            .is_empty()
    );
}

// FUN_0048fdf0: A0 alliance-game2/jan-dodonna-killed shows the
// Alliance's "Jan Dodonna Killed"; the scenario names him for that
// reader only.
#[test]
fn the_event_messages_scenario_reports_jan_dodonna_killed_for_the_alliance() {
    let mut world = fleet_world();
    let dodonna = world.characters.insert(rebellion_core::world::Character {
        name: "Jan Dodonna".into(),
        is_alliance: true,
        ..Default::default()
    });
    assert_eq!(
        killed_character(
            request(Scenario::EventMessages, CockpitFaction::Alliance),
            &world
        ),
        Some(dodonna)
    );
    // The Empire's reader takes Jerjerrod, whom this world lacks.
    assert_eq!(
        killed_character(
            request(Scenario::EventMessages, CockpitFaction::Empire),
            &world
        ),
        None
    );
    let jerjerrod = world.characters.insert(rebellion_core::world::Character {
        name: "Jerjerrod".into(),
        is_empire: true,
        ..Default::default()
    });
    assert_eq!(
        killed_character(
            request(Scenario::EventMessages, CockpitFaction::Empire),
            &world
        ),
        Some(jerjerrod)
    );
    assert_eq!(
        killed_character(
            request(Scenario::StoryMessages, CockpitFaction::Alliance),
            &world
        ),
        None
    );
}

// FUN_00499130: A0 alliance-game2/smuggling-benefits shows the
// Alliance's "Smuggling Benefits" from Yavin; the scenario names Yavin
// for that reader only.
#[test]
fn the_event_messages_scenario_smuggles_from_yavin_for_the_alliance() {
    let mut world = fleet_world();
    let yavin = world.systems.keys().nth(1).unwrap();
    world.systems[yavin].name = "Yavin".into();
    assert_eq!(
        smuggled_system(
            request(Scenario::EventMessages, CockpitFaction::Alliance),
            &world
        ),
        Some(yavin)
    );
    // The Empire's reader takes Coruscant, which this world lacks.
    assert_eq!(
        smuggled_system(
            request(Scenario::EventMessages, CockpitFaction::Empire),
            &world
        ),
        None
    );
    assert_eq!(
        smuggled_system(
            request(Scenario::StoryMessages, CockpitFaction::Alliance),
            &world
        ),
        None
    );
}

// FUN_004960f0: A0 events-day673/02 shows the Alliance "Yavin Under
// Blockade"; the scenario blockades Yavin for that reader only.
#[test]
fn the_event_messages_scenario_blockades_yavin_for_the_alliance() {
    let mut world = fleet_world();
    let yavin = world.systems.keys().nth(1).unwrap();
    world.systems[yavin].name = "Yavin".into();
    assert_eq!(
        blockaded_system(
            request(Scenario::EventMessages, CockpitFaction::Alliance),
            &world
        ),
        Some(yavin)
    );
    assert_eq!(
        blockaded_system(
            request(Scenario::EventMessages, CockpitFaction::Empire),
            &world
        ),
        None
    );
    assert_eq!(
        blockaded_system(
            request(Scenario::StoryMessages, CockpitFaction::Alliance),
            &world
        ),
        None
    );
}

// FUN_0048f1e0 (timer 0x389): A0 research-display/08-future-jedi shows
// the Empire Vader's "Future Jedi Discovered" for Ozzel, and
// future-jedi-alliance/01 the Alliance Luke's for Jan Dodonna.
#[test]
fn the_event_messages_scenario_has_each_sides_trainer_find_a_force_user() {
    assert_eq!(
        force_discoveries(request(Scenario::EventMessages, CockpitFaction::Empire)),
        vec![("Vader", "Ozzel")]
    );
    assert_eq!(
        force_discoveries(request(Scenario::EventMessages, CockpitFaction::Alliance)),
        vec![("Luke", "Dodonna")]
    );
    assert!(force_discoveries(request(Scenario::StoryMessages, CockpitFaction::Empire)).is_empty());
}

// FUN_00490340: the EventMessages scenario files the Emperor's capture
// at Corfai for its reader (A0 messages-day633, read by the Empire).
#[test]
fn the_event_messages_scenario_files_the_emperors_capture() {
    let texts: std::collections::HashMap<u16, String> = [
        (0x7158, "{1:1} Captured"),
        (0x7159, "{1:1} was captured by the {3:1} at {2:1}."),
        (0x715a, "{1:1} Captured"),
        (0x715b, "{1:1} was captured at {2:1}."),
        (0x4d50, "Alliance"),
    ]
    .into_iter()
    .map(|(id, text)| (id, text.to_owned()))
    .collect();
    let request = request(Scenario::EventMessages, CockpitFaction::Alliance);
    let applied = apply_to_loading_world_with(request, |world| {
        world.characters.insert(rebellion_core::world::Character {
            name: "Emperor Palpatine".into(),
            dat_id: rebellion_core::ids::DatId::new(0x3000_0280),
            is_major: true,
            is_empire: true,
            ..Default::default()
        });
    });
    let mut log = rebellion_render::MessageLog::new();
    post_capture_message(request, &applied.world, &mut log, &texts);
    let [message] = log.messages() else {
        panic!("{:?}", log.messages());
    };
    assert_eq!(message.title.as_deref(), Some("Emperor Palpatine Captured"));
    // The Alliance reads the enemy's captive by case 4 (A0
    // empire-clean/31-needa-captured gives the own side case 3).
    assert_eq!(message.text, "Emperor Palpatine was captured at Corfai.");
    let mut none = rebellion_render::MessageLog::new();
    post_capture_message(
        super::tests::request(Scenario::StoryMessages, CockpitFaction::Alliance),
        &applied.world,
        &mut none,
        &texts,
    );
    assert!(none.messages().is_empty());
}

// FUN_00458fe0: a built facility (`+0x50` bit 2 set, bit 4 clear) takes
// its type's base whatever its side; FUN_00539f50's records 4, 5 and 6
// have 0x2332, 0x233a and 0x2342 (9010, 9018, 9026).
#[test]
fn the_advanced_bases_scenario_shows_each_built_advanced_base_first_on_its_page() {
    use rebellion_render::manufacturing_window::{page_cells, FacilityPage};
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let applied =
            apply_to_loading_world_with(request(Scenario::AdvancedBases, faction), |_| {});
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        let pictures = |page| -> Vec<u32> {
            page_cells(
                world,
                &ManufacturingState::default(),
                &rebellion_core::delivery::DeliveryState::default(),
                true,
                primary,
                page,
            )
            .iter()
            .map(|cell| cell.picture)
            .collect()
        };
        assert_eq!(pictures(FacilityPage::Shipyards), [9_010], "{faction:?}");
        assert_eq!(
            pictures(FacilityPage::TrainingFacilities),
            [9_018],
            "{faction:?}"
        );
        assert_eq!(
            pictures(FacilityPage::ConstructionYards),
            [9_026],
            "{faction:?}"
        );
        // Only the Advanced Shipyard builds ships.
        let shipyards: Vec<bool> = world.systems[primary]
            .manufacturing_facilities
            .iter()
            .map(|&key| world.manufacturing_facilities[key].is_shipyard)
            .collect();
        assert_eq!(shipyards, [true, false, false], "{faction:?}");
    }
}

// FUN_00458fe0: an object en route takes its base + 3; the Build
// Selection scenario sends a shipyard, a mine and a refinery to the
// primary (A0 personnel-states/34, 9009; build-selection4/26, 9004;
// build-selection4/29, 9033).
#[test]
fn the_build_selection_scenario_sends_a_shipyard_a_mine_and_a_refinery_to_the_primary() {
    use rebellion_render::manufacturing_window::{page_cells, FacilityPage};
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let request = request(Scenario::BuildSelection, faction);
        let applied = apply_to_loading_world_with(request, |_| {});
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        let mut deliveries = rebellion_core::delivery::DeliveryState::default();
        place_deliveries(request, world, &mut deliveries);
        let [shipyard, mine, refinery] = deliveries.en_route() else {
            panic!("{:?}", deliveries.en_route());
        };
        for delivery in [shipyard, mine, refinery] {
            assert_eq!(delivery.destination, primary);
            assert!(delivery.arrival_tick > 100_000);
        }
        let cells = |page| {
            page_cells(
                world,
                &ManufacturingState::default(),
                &deliveries,
                true,
                primary,
                page,
            )
        };
        assert_eq!(
            cells(FacilityPage::Shipyards)
                .last()
                .map(|cell| cell.picture),
            Some(9_009)
        );
        // The mine en route follows the built mine, before the empty
        // deposits (A0 build-selection4/26: Yavin's two built mines,
        // then the one en route, then the deposits).
        let mines: Vec<u32> = cells(FacilityPage::Mines)
            .iter()
            .map(|cell| cell.picture)
            .collect();
        assert_eq!(mines[..3], [9_001, 9_004, 9_005], "{mines:?}");
        // A0 build-selection4/29: Yavin's refinery en route, 9033.
        let refineries: Vec<u32> = cells(FacilityPage::Refineries)
            .iter()
            .map(|cell| cell.picture)
            .collect();
        assert_eq!(refineries, [9_030, 9_033], "{refineries:?}");
        let mut none = rebellion_core::delivery::DeliveryState::default();
        place_deliveries(
            super::tests::request(Scenario::BuildSelectionDestination, faction),
            world,
            &mut none,
        );
        assert!(none.en_route().is_empty());
    }
}

// build-selection.mjs reads every facility class's values at the
// primary at number 1 (A0 build-selection4, build-selection-empire2),
// so the primary keeps a free energy slot and raw deposit for each
// (FUN_00507750, the room FUN_005385f0's order needs) with the three
// products sent to it on their way.
#[test]
fn the_build_selection_scenarios_primary_has_room_for_one_more_facility_of_each_class() {
    use rebellion_core::manufacturing::{BuildableKind, FacilityBuild};
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let request = request(Scenario::BuildSelection, faction);
        let applied = apply_to_loading_world_with(request, |_| {});
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        let mut deliveries = rebellion_core::delivery::DeliveryState::default();
        place_deliveries(request, world, &mut deliveries);
        let is_alliance = faction == CockpitFaction::Alliance;
        // A defense, a yard of each kind, a mine and a refinery.
        let classes = [
            0x2200_0001,
            0x2800_0001,
            0x2900_0005,
            0x2a00_0001,
            0x2c00_0001,
            0x2d00_0002,
        ]
        .map(|raw| {
            let class = rebellion_core::ids::DatId::new(raw);
            let build = FacilityBuild { class, is_alliance };
            match class.family() {
                0x22 => BuildableKind::DefenseFacility(build),
                0x2c | 0x2d => BuildableKind::ProductionFacility(build),
                _ => BuildableKind::ManufacturingFacility(build),
            }
        });
        for kind in classes {
            assert!(
                !rebellion_core::build_selection::lacks_facility_room(
                    world,
                    &applied.manufacturing,
                    deliveries.en_route(),
                    (
                        primary,
                        rebellion_core::manufacturing::ProductionArea::ConstructionYard
                    ),
                    primary,
                    kind,
                    1,
                ),
                "{faction:?} {kind:?}"
            );
        }
    }
}

// FUN_00509710: the other side's squadrons make it present beside the
// player's fleet, so a battle is due.
#[test]
fn the_squadron_battle_scenario_puts_two_enemy_squadrons_under_the_fleet() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let alliance = faction == CockpitFaction::Alliance;
        let request = request(Scenario::SquadronBattle, faction);
        let applied = apply_to_loading_world_with(request, |world| {
            for side in [true, false] {
                world
                    .fighter_classes
                    .insert(rebellion_core::world::FighterClass {
                        is_alliance: side,
                        is_empire: !side,
                        ..Default::default()
                    });
            }
        });
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        assert_eq!(
            rebellion_core::squadrons::on_planet(world, primary, !alliance),
            2
        );
        assert_eq!(
            rebellion_core::squadrons::on_planet(world, primary, alliance),
            0
        );
        assert_eq!(
            rebellion_core::squadrons::sides_present(world, primary),
            (true, true)
        );
    }
}

// FUN_00516730: an enemy capital ship whose class projects a gravity
// well bars the Battle Alert's Retreat (A0 2026-11-17-edited-saves-4:
// d-01 with the Interdictor, 10976; e-01 without it, 10974).
#[test]
fn the_gravity_well_battle_scenario_bars_the_players_retreat_by_an_enemy_projector() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let alliance = faction == CockpitFaction::Alliance;
        let request = request(Scenario::GravityWellBattle, faction);
        let applied = apply_to_loading_world_with(request, |world| {
            for side in [true, false] {
                for projector in [0, 4] {
                    world
                        .capital_ship_classes
                        .insert(rebellion_core::world::CapitalShipClass {
                            is_alliance: side,
                            hull: 1200,
                            hyperdrive: 80,
                            gravity_well_projector: projector,
                            ..Default::default()
                        });
                }
                world
                    .fighter_classes
                    .insert(rebellion_core::world::FighterClass {
                        is_alliance: side,
                        is_empire: !side,
                        ..Default::default()
                    });
            }
        });
        let mut world = applied.world;
        let primary = world.systems.keys().next().unwrap();
        let enemy: Vec<_> = world.systems[primary]
            .fleets
            .iter()
            .map(|&key| &world.fleets[key])
            .filter(|fleet| fleet.is_alliance != alliance)
            .collect();
        assert_eq!(enemy.len(), 1, "{faction:?}");
        assert_eq!(enemy[0].capital_ships.len(), 1, "{faction:?}");
        let ship = &enemy[0].capital_ships[0];
        let class = ship.class;
        assert_eq!(ship.hull_current, 1200, "{faction:?}");
        assert_eq!(world.capital_ship_classes[class].is_alliance, !alliance);
        assert_eq!(world.capital_ship_classes[class].gravity_well_projector, 4);
        assert_eq!(
            rebellion_core::squadrons::on_planet(&world, primary, !alliance),
            0
        );
        assert_eq!(
            rebellion_core::squadrons::sides_present(&world, primary),
            (true, true)
        );
        assert!(
            !rebellion_core::abstract_battle::retreat_offered(&world, primary, alliance),
            "{faction:?}"
        );
        // The same battle without the projector offers Retreat.
        world.capital_ship_classes[class].gravity_well_projector = 0;
        assert!(
            rebellion_core::abstract_battle::retreat_offered(&world, primary, alliance),
            "{faction:?}"
        );
    }
}

// port: the fixture's own layout. A product one point from its cost
// completes on the next yard cycle (FUN_0052b960) and departs for its
// destination (FUN_0052bee0).
#[test]
fn the_delivery_scenario_queues_a_ship_one_point_short_bound_for_the_target() {
    let request = request(Scenario::ProductionDelivery, CockpitFaction::Alliance);
    let applied = apply_to_loading_world_with(request, |world| {
        world
            .capital_ship_classes
            .insert(rebellion_core::world::CapitalShipClass {
                is_alliance: true,
                hull: 300,
                hyperdrive: 80,
                troop_capacity: 2,
                refined_material_cost: 6,
                ..Default::default()
            });
    });
    let world = &applied.world;
    let primary = world.systems.keys().next().unwrap();
    let target = loading_target(world, primary).unwrap();
    let mut manufacturing = ManufacturingState::default();
    place_delivery(request, world, &mut manufacturing, primary);
    let queue = manufacturing
        .queue(
            primary,
            rebellion_core::manufacturing::ProductionArea::Shipyard,
        )
        .unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.completed_points(), 5);
    assert_eq!(queue.active().unwrap().destination, Some(target));
    assert!(runs_the_clock(request));
    assert!(world.systems[primary]
        .manufacturing_facilities
        .iter()
        .any(|&key| world.manufacturing_facilities[key].is_shipyard));
}

// port: the fixture's own layout. A revolt's incident timer (event
// 0x38d, FUN_0050d030) and the disaster timer (event 0x38f,
// FUN_00511930) fire when due; the disaster spares a system with
// neither energy nor raw materials.
// port: the fixture's own layout. FUN_0050b610 gives a system with no
// regiment the side its support favours, and System Joins
// (FUN_00499760) reports the change.
#[test]
fn the_support_flip_scenario_gives_the_target_s_support_to_the_other_side() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let alliance = faction == CockpitFaction::Alliance;
        let request = request(Scenario::SupportFlip, faction);
        let applied = apply_to_loading_world_with(request, |_| {});
        let primary = applied.world.systems.keys().next().unwrap();
        let target = loading_target(&applied.world, primary).unwrap();
        let target_name = applied.world.systems[target].name.clone();
        let system = &applied.world.systems[target];
        assert_eq!(system.popularity_alliance, if alliance { 0.0 } else { 1.0 });
        assert_eq!(system.popularity_empire, if alliance { 1.0 } else { 0.0 });
        assert!(system.ground_units.is_empty());
        assert!(runs_the_clock(request));
        let line = |title: &str, player: bool| {
            GameMessage::new(
                1,
                "text".to_string(),
                rebellion_render::message_log::MessageCategory::Diplomacy,
            )
            .with_title(title.to_string())
            .on_rail(
                rebellion_render::message_log::MessageRail::PopularSupport,
                rebellion_render::message_log::RailAudience::side(alliance == player),
            )
        };
        let observed = fleet_load_observation(
            request,
            loading_regiment(&applied.world, primary).unwrap(),
            &applied.world,
            &FogState::new(Faction::of_alliance(alliance)),
            &applied.movement,
            &applied.transport,
            &FleetWindowState::default(),
            false,
            &[
                line(&format!("{target_name} Joins Enemy"), true),
                line("Elsewhere Joins Enemy", true),
                line(&format!("{target_name} Joins"), false),
            ],
        )
        .unwrap();
        assert_eq!(
            observed.last_joins_title,
            Some(format!("{target_name} Joins Enemy"))
        );
        // A system joining the player's own side reads "Joins".
        let observed = fleet_load_observation(
            request,
            loading_regiment(&applied.world, primary).unwrap(),
            &applied.world,
            &FogState::new(Faction::of_alliance(alliance)),
            &applied.movement,
            &applied.transport,
            &FleetWindowState::default(),
            false,
            &[line(&format!("{target_name} Joins"), true)],
        )
        .unwrap();
        assert_eq!(
            observed.last_joins_title,
            Some(format!("{target_name} Joins"))
        );
    }
}

// port: the fixture's own layout. A fleet arriving at the other side's
// system blockades it (blockade::system_is_blockaded); Blockade
// (FUN_004960f0) then speaks to the player.
#[test]
fn the_blockade_scenario_sends_the_fleet_to_a_system_the_other_side_holds() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let alliance = faction == CockpitFaction::Alliance;
        let request = request(Scenario::BlockadeArrival, faction);
        let mut applied = apply_to_loading_world_with(request, |_| {});
        let primary = applied.world.systems.keys().next().unwrap();
        let target = loading_target(&applied.world, primary).unwrap();
        let fleet = applied.world.fleets.keys().next().unwrap();
        place_blockade_arrival(request, &mut applied.world, &mut applied.movement, primary);
        let enemy = if alliance {
            Faction::Empire
        } else {
            Faction::Alliance
        };
        let system = &applied.world.systems[target];
        assert_eq!(system.control, ControlKind::Controlled(enemy));
        assert_eq!(system.popularity_alliance, if alliance { 0.0 } else { 1.0 });
        assert_eq!(system.popularity_empire, if alliance { 1.0 } else { 0.0 });
        assert_eq!(
            applied.movement.get(fleet).map(|order| order.destination),
            Some(target)
        );
        assert!(runs_the_clock(request));

        // The report keeps the player's Blockade title only.
        let line = |title: &str, player: bool| {
            GameMessage::new(
                1,
                "text".to_string(),
                rebellion_render::message_log::MessageCategory::Combat,
            )
            .with_title(title.to_string())
            .on_rail(
                rebellion_render::message_log::MessageRail::Conflict,
                rebellion_render::message_log::RailAudience::side(alliance == player),
            )
        };
        let observed = fleet_load_observation(
            request,
            loading_regiment(&applied.world, primary).unwrap(),
            &applied.world,
            &FogState::new(Faction::of_alliance(alliance)),
            &applied.movement,
            &applied.transport,
            &FleetWindowState::default(),
            false,
            &[
                line("Fleet Initiates Blockade of Mine", true),
                line("Theirs Under Blockade", false),
            ],
        )
        .unwrap();
        assert_eq!(
            observed.last_blockade_title.as_deref(),
            Some("Fleet Initiates Blockade of Mine")
        );
    }
}

#[test]
fn the_incident_scenario_revolts_the_primary_system_with_both_timers_due_soon() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let request = request(Scenario::UprisingIncident, faction);
        let mut applied = apply_to_loading_world_with(request, |_| {});
        let world = &mut applied.world;
        let primary = world.systems.keys().next().unwrap();
        let empty = world.systems.keys().nth(1).unwrap();
        world.systems[empty].total_energy = 0;
        world.systems[empty].raw_materials = 0;
        let mined = world.systems.keys().nth(2).unwrap();
        world.systems[mined].total_energy = 0;
        world.systems[mined].raw_materials = 3;
        let mut uprisings = rebellion_core::uprising::UprisingState::default();
        place_incidents(request, world, &mut uprisings, primary);
        let side = if faction == CockpitFaction::Alliance {
            Faction::Alliance
        } else {
            Faction::Empire
        };
        assert_eq!(world.systems[primary].control, ControlKind::Uprising(side));
        let uprising = &uprisings.active_uprisings[&primary];
        assert_eq!(uprising.next_incident_tick, Some(5));
        assert_eq!(uprising.side, Some(side));
        assert_eq!(uprisings.next_disaster_tick, Some(8));
        assert!(world
            .systems
            .values()
            .all(|system| system.total_energy > 0 || system.raw_materials > 0));
        assert_eq!(world.systems[empty].total_energy, 1);
        assert_eq!(world.systems[mined].total_energy, 0);
        assert!(runs_the_clock(request));

        // The report keeps the player's incident line only, and any
        // disaster line.
        let alliance = faction == CockpitFaction::Alliance;
        let line = |text: &str, player: bool| {
            GameMessage::new(
                1,
                text.to_string(),
                rebellion_render::message_log::MessageCategory::Diplomacy,
            )
            .on_rail(
                rebellion_render::message_log::MessageRail::PopularSupport,
                rebellion_render::message_log::RailAudience::side(alliance == player),
            )
        };
        let observed = fleet_load_observation(
            request,
            loading_regiment(world, primary).unwrap(),
            world,
            &FogState::new(Faction::of_alliance(alliance)),
            &applied.movement,
            &applied.transport,
            &FleetWindowState::default(),
            false,
            &[
                line("Uprising incident at Mine", true),
                line("Uprising incident at Theirs", false),
                line("Natural disaster strikes There", false),
                line("There has been a natural disaster on Held.", false)
                    .with_title("Natural Disaster on Held".to_string()),
            ],
        )
        .unwrap();
        assert!(observed.primary_in_revolt);
        assert_eq!(
            observed.last_incident_message.as_deref(),
            Some("Uprising incident at Mine")
        );
        assert_eq!(
            observed.last_disaster_message.as_deref(),
            Some("There has been a natural disaster on Held.")
        );
        assert_eq!(
            observed.last_disaster_title.as_deref(),
            Some("Natural Disaster on Held")
        );
    }
}

// port: the fixture's own layout. Destroy System needs a Death Star in
// the fleet (FUN_00582440, 8/8 without one); CAPSHPSD 136 is the
// Empire's only.
#[test]
fn the_destroy_system_scenario_gives_only_the_empire_s_fleet_a_death_star() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let alliance = faction == CockpitFaction::Alliance;
        let request = request(Scenario::FleetDestroySystem, faction);
        let applied = apply_to_loading_world_with(request, |world| {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    is_alliance: alliance,
                    hull: 300,
                    hyperdrive: 80,
                    troop_capacity: 2,
                    bombardment_modifier: 5,
                    ..Default::default()
                });
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    dat_id: rebellion_core::ids::DatId::new(
                        rebellion_core::world::DEATH_STAR_CLASS_ID,
                    ),
                    is_alliance: false,
                    is_empire: true,
                    hull: 25_000,
                    hyperdrive: 80,
                    bombardment_modifier: 15,
                    ..Default::default()
                });
            world.characters.insert(rebellion_core::world::Character {
                is_alliance: !alliance,
                is_empire: alliance,
                ..Default::default()
            });
        });
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        let fleet = world.fleets.keys().next().unwrap();
        assert_eq!(world.fleets[fleet].has_death_star, !alliance);
        assert_eq!(
            rebellion_core::bombardment::order_enabled(
                world,
                &applied.movement,
                alliance,
                &[fleet],
                primary,
                rebellion_core::bombardment::DESTROY_SYSTEM,
            ),
            !alliance
        );
        assert!(world.characters.values().any(
            |c| c.is_alliance != alliance && rebellion_core::death_star::stands_in(c, primary)
        ));
    }
}

// port: the fixture's own layout; the order's gate is
// bombardment::order_enabled (FUN_00553770, FUN_00582440).
#[test]
fn the_bombardment_scenario_lets_the_fleet_bombard_two_enemy_regiments() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let alliance = faction == CockpitFaction::Alliance;
        let request = request(Scenario::FleetBombardment, faction);
        let applied = apply_to_loading_world_with(request, |world| {
            // Before each side's class that fits, one without room for
            // regiments (the gate's setup needs it), one that does not
            // bombard and one without a hyperdrive; the other side's
            // fitting class comes first.
            let other = !alliance;
            for (side, room, bombard, hyperdrive) in [
                (other, 2, 5, 80),
                (alliance, 0, 5, 80),
                (alliance, 2, 0, 80),
                (alliance, 2, 5, 0),
                (alliance, 2, 5, 80),
            ] {
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        is_alliance: side,
                        hull: 300,
                        hyperdrive,
                        troop_capacity: room,
                        bombardment_modifier: bombard,
                        ..Default::default()
                    });
            }
        });
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        let fleet = world.fleets.keys().next().unwrap();
        let system = &world.systems[primary];
        assert_eq!(
            GameWorld::facility_holder(system.control),
            Some(Faction::of_alliance(!alliance))
        );
        assert_eq!(system.defense_facilities.len(), 0);
        assert_eq!(system.ground_units.len(), 2);
        assert!(system
            .ground_units
            .iter()
            .all(|&key| world.troops[key].is_alliance != alliance));
        assert_eq!(world.fleets[fleet].capital_ships.len(), 1);
        assert_eq!(world.fleets[fleet].capital_ships[0].hull_current, 300);
        let class = &world.capital_ship_classes[world.fleets[fleet].capital_ships[0].class];
        assert_eq!(
            (
                class.is_alliance,
                class.troop_capacity,
                class.bombardment_modifier,
                class.hyperdrive
            ),
            (alliance, 2, 5, 80)
        );
        assert!(rebellion_core::bombardment::fleet_bombardment(world, fleet) > 0);
        assert!(rebellion_core::bombardment::order_enabled(
            world,
            &applied.movement,
            alliance,
            &[fleet],
            primary,
            rebellion_core::bombardment::GENERAL,
        ));
        // port: the fixture's own report counts the other side's
        // regiments and keeps the last bombardment line.
        let line = GameMessage::new(
            1,
            "Orbital bombardment at X".to_string(),
            rebellion_render::message_log::MessageCategory::Combat,
        );
        let observed = fleet_load_observation(
            request,
            loading_regiment(world, primary).unwrap(),
            world,
            &FogState::new(Faction::of_alliance(alliance)),
            &applied.movement,
            &applied.transport,
            &FleetWindowState::default(),
            false,
            &[line],
        )
        .unwrap();
        assert_eq!(observed.enemy_regiments, 2);
        assert_eq!(
            observed.last_bombardment_message.as_deref(),
            Some("Orbital bombardment at X")
        );
    }
}

// port: the fixture's own layout; the order's gate is
// assault::order_enabled (FUN_00581ae0, FUN_00553770).
#[test]
fn the_assault_scenario_lets_the_fleet_assault_with_a_regiment_aboard() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let alliance = faction == CockpitFaction::Alliance;
        let assault = |scenario| {
            let applied = apply_to_loading_world_with(request(scenario, faction), |world| {
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        is_alliance: alliance,
                        hull: 300,
                        hyperdrive: 80,
                        troop_capacity: 2,
                        bombardment_modifier: 5,
                        ..Default::default()
                    });
            });
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            let fleet = world.fleets.keys().next().unwrap();
            let aboard = applied.transport.cargo(fleet).to_vec();
            let enabled = rebellion_core::assault::order_enabled(
                world,
                &applied.movement,
                &applied.transport,
                alliance,
                &[fleet],
                primary,
            );
            (applied, aboard, enabled)
        };
        let (applied, aboard, enabled) = assault(Scenario::FleetAssault);
        assert_eq!(aboard.len(), 1);
        assert_eq!(applied.world.troops[aboard[0]].is_alliance, alliance);
        assert!(enabled);
        // The taken variant leaves the planet no regiment of the other
        // side's, so the assault takes it (0x7091).
        // It carries six, as A0 messages-day633's Alliance Forces.
        let (taken, aboard, enabled) = assault(Scenario::FleetAssaultTaken);
        assert_eq!(aboard.len(), 6);
        assert!(enabled);
        let target = taken.world.systems.keys().next().unwrap();
        assert!(taken.world.systems[target]
            .ground_units
            .iter()
            .all(|key| taken.world.troops[*key].is_alliance == alliance));
        // The bombardment variant carries none, so the order is off.
        let (_, aboard, enabled) = assault(Scenario::FleetBombardment);
        assert!(aboard.is_empty());
        assert!(!enabled);
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        let line = GameMessage::new(
            1,
            "Planetary assault at X".to_string(),
            rebellion_render::message_log::MessageCategory::Combat,
        );
        let observed = fleet_load_observation(
            request(Scenario::FleetAssault, faction),
            loading_regiment(world, primary).unwrap(),
            world,
            &FogState::new(Faction::of_alliance(alliance)),
            &applied.movement,
            &applied.transport,
            &FleetWindowState::default(),
            false,
            &[line],
        )
        .unwrap();
        assert_eq!(observed.enemy_regiments, 2);
        assert_eq!(
            observed.last_assault_message.as_deref(),
            Some("Planetary assault at X")
        );
        assert_eq!(observed.last_bombardment_message, None);
    }
}

// port: the fixture's own layout; the room follows FUN_00500b40.
#[test]
fn the_squadron_load_scenario_puts_two_squadrons_beside_a_fighter_carrier() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let alliance = faction == CockpitFaction::Alliance;
        let request = request(Scenario::SquadronLoad, faction);
        let applied = apply_to_loading_world_with(request, |world| {
            for side in [true, false] {
                world
                    .fighter_classes
                    .insert(rebellion_core::world::FighterClass {
                        is_alliance: side,
                        is_empire: !side,
                        ..Default::default()
                    });
            }
        });
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        let fleet = world.fleets.keys().next().unwrap();
        assert_eq!(
            rebellion_core::squadrons::on_planet(world, primary, alliance),
            2
        );
        assert_eq!(
            rebellion_core::squadrons::on_planet(world, primary, !alliance),
            0
        );
        assert!(rebellion_core::squadrons::fleet_room(world, fleet) >= 1);
        assert!(world.fleets[fleet].fighters.is_empty());
        let setup = fleet_load_setup(request, world, &applied.sectors, &applied.defenses).unwrap();
        assert_eq!(
            setup.fighter_room,
            rebellion_core::squadrons::fleet_room(world, fleet)
        );
    }
}

// A0 2026-11-15 "m-5": the regiment ordered at Talus waited on the
// planet, beside the living one, before its drop onto Fleet 3.
#[test]
fn the_unfinished_load_scenario_orders_a_regiment_that_waits_on_the_planet() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let request = request(Scenario::FleetLoadUnfinished, faction);
        let applied = apply_to_loading_world(request);
        let world = &applied.world;
        let here = world.systems.keys().next().unwrap();
        let fleet = world.fleets.keys().next().unwrap();
        let regiment = loading_regiment(world, here).unwrap();
        let products: Vec<_> = world.products.in_system(here).collect();
        assert_eq!(products.len(), 1, "{faction:?}");
        assert_eq!(
            products[0].kind,
            rebellion_core::manufacturing::BuildableKind::Troop(
                world.troops[regiment].class_dat_id
            )
        );
        assert_eq!(world.products.in_fleet(fleet).count(), 0);
        let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
        let setup = fleet_load_setup(request, world, &applied.sectors, &applied.defenses).unwrap();
        assert!(applied
            .defenses
            .contains_screen_point(layout, setup.pending_item.unwrap()));
    }
}

// port: the fixture's own layout; the load itself follows FUN_00500b40.
#[test]
fn the_fleet_load_scenario_leaves_one_carrier_and_one_movable_regiment_at_the_players_system() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let request = request(Scenario::FleetLoad, faction);
        let mut applied = apply_to_loading_world(request);
        let world = &applied.world;
        let mut keys = world.systems.keys();
        let (here, target) = (keys.next().unwrap(), keys.next().unwrap());
        let fleet = world.fleets.keys().next().unwrap();
        let is_alliance = faction == CockpitFaction::Alliance;
        let player = if is_alliance {
            Faction::Alliance
        } else {
            Faction::Empire
        };

        assert_eq!(world.systems[here].fleets, [fleet], "{faction:?}");
        assert!(world.systems[target].fleets.is_empty(), "{faction:?}");
        assert_eq!(world.fleets[fleet].is_alliance, is_alliance);
        let [ship] = world.fleets[fleet].capital_ships.as_slice() else {
            panic!("{faction:?}: the fleet holds one capital ship");
        };
        let class = &world.capital_ship_classes[ship.class];
        assert!(
            class.troop_capacity > 0 && class.hyperdrive > 0,
            "{faction:?}"
        );
        assert_eq!(
            world.systems[target].control,
            ControlKind::Controlled(player)
        );
        assert!(world.systems[target].ground_units.is_empty());
        let regiment = loading_regiment(world, here).unwrap();
        assert_eq!(world.systems[here].ground_units, [regiment]);
        assert_eq!(world.troops[regiment].is_alliance, is_alliance);
        assert!(applied.transport.is_empty());
        assert!(applied
            .transport
            .regiment_move_enabled(world, regiment, is_alliance));

        let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
        let setup = fleet_load_setup(request, world, &applied.sectors, &applied.defenses).unwrap();
        assert_eq!(setup.target_dat_id, world.systems[target].dat_id.raw());
        assert_eq!(setup.target_name, world.systems[target].name);
        assert_eq!(setup.capacity, Some(class.troop_capacity));
        // The Defenses window lies on the galaxy view's left, the
        // primary system's sector window in the right column.
        assert!(applied
            .defenses
            .contains_screen_point(layout, setup.troop_item.unwrap()));
        assert!(!applied.defenses.contains_screen_point(layout, setup.icon));
        assert!(applied.sectors.contains_screen_point(layout, setup.icon));
        assert!(applied
            .sectors
            .contains_screen_point(layout, setup.target_planet));

        applied
            .transport
            .embark(&mut applied.world, fleet, &[regiment])
            .unwrap();
    }
}

#[test]
fn the_destination_scenario_adds_a_player_shipyard_and_shows_its_facility_icon() {
    // port: the fixture's own layout for the Destination gate
    // (ghidra/notes/production-destination.md).
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let player_shipyards = |scenario| {
            let applied = apply_to_loading_world(request(scenario, faction));
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            let setup = fleet_load_setup(
                request(scenario, faction),
                world,
                &applied.sectors,
                &applied.defenses,
            )
            .unwrap();
            let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
            if let Some(icon) = setup.system_icon {
                assert!(
                    applied.sectors.contains_screen_point(layout, icon),
                    "{faction:?}"
                );
            }
            let shipyards = world.systems[primary]
                .manufacturing_facilities
                .iter()
                .filter(|key| {
                    let facility = &world.manufacturing_facilities[**key];
                    facility.is_shipyard
                        && (facility.side == rebellion_core::dat::Faction::Alliance)
                            == (faction == CockpitFaction::Alliance)
                })
                .count();
            (shipyards, setup.system_icon.is_some())
        };
        let (before, _) = player_shipyards(Scenario::FleetLoad);
        assert_eq!(
            player_shipyards(Scenario::ProductionDestination),
            (before + 1, true),
            "{faction:?}"
        );
    }
}

#[test]
fn the_joining_scenario_puts_a_player_fleet_of_one_ship_beside_one_of_two() {
    // port: the fixture's own layout for the join gate
    // (ghidra/notes/fleet-join-split.md).
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let applied = apply_to_loading_world(request(Scenario::FleetJoin, faction));
        let world = &applied.world;
        let here = world.systems.keys().next().unwrap();
        let first = world.fleets.keys().next().unwrap();
        let is_alliance = faction == CockpitFaction::Alliance;
        let [a, b] = world.systems[here].fleets.as_slice() else {
            panic!("{faction:?}: two fleets orbit the primary system");
        };
        assert_eq!(*a, first, "{faction:?}");
        for (fleet, ships) in [(*a, 2), (*b, 1)] {
            let value = &world.fleets[fleet];
            assert_eq!(value.capital_ships.len(), ships, "{faction:?}");
            assert!(value.capital_ships.iter().all(|ship| ship.alive));
            assert_eq!(value.is_alliance, is_alliance);
            assert!(!value.has_death_star && value.characters.is_empty());
            assert!(!applied.movement.is_in_transit(fleet));
        }
    }
}

#[test]
fn the_en_route_joining_scenario_sends_the_one_ship_fleet_onto_the_other() {
    // port: the fixture's own layout for the en route join case.
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let applied = apply_to_loading_world(request(Scenario::FleetJoinEnroute, faction));
        let world = &applied.world;
        let here = world.systems.keys().next().unwrap();
        let first = world.fleets.keys().next().unwrap();
        assert_eq!(world.systems[here].fleets, [first], "{faction:?}");
        let order = applied
            .movement
            .orders()
            .values()
            .find(|order| order.join == Some(first))
            .expect("a fleet on its way to join the first");
        assert_eq!(order.destination, here);
        assert_ne!(world.fleets[order.fleet].location, here);
        assert_eq!(world.fleets[order.fleet].capital_ships.len(), 1);
        assert_eq!(world.fleets[first].capital_ships.len(), 2);
        assert!(rebellion_core::fleet_join::joins_on_arrival(
            world,
            &applied.movement,
            first,
            here,
            faction == CockpitFaction::Alliance
        ));
    }
}

#[test]
fn the_fleet_load_observation_lists_each_fleet_with_its_ships_and_entry() {
    // port: the fixture's own report.
    let request = request(Scenario::FleetJoin, CockpitFaction::Alliance);
    let applied = apply_to_loading_world(request);
    let here = applied.world.systems.keys().next().unwrap();
    let regiment = loading_regiment(&applied.world, here).unwrap();
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(here);
    let layout = CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
    let mut fleets = FleetWindowState::default();
    fleets.open(
        &applied.world,
        here,
        (300, 80),
        CockpitFaction::Alliance,
        layout,
    );
    let observe = |confirmation_open| {
        fleet_load_observation(
            request,
            regiment,
            &applied.world,
            &fog,
            &applied.movement,
            &applied.transport,
            &fleets,
            confirmation_open,
            &[],
        )
        .unwrap()
    };
    let row = |row| screen_center(fleets.entry_screen_rect(layout, here, row).unwrap());

    let observed = observe(false);
    assert_eq!(observed.entries, 2);
    assert_eq!(
        observed.fleets,
        [
            FleetEntryObservation {
                ships: 2,
                entry: row(0)
            },
            FleetEntryObservation {
                ships: 1,
                entry: row(1)
            },
        ]
    );
    assert_ne!(row(0), row(1));
    assert!(!observed.confirmation_open);
    assert!(observe(true).confirmation_open);
}

#[test]
fn the_fleet_load_scenario_picks_a_carrier_of_the_players_side_that_can_jump() {
    // port: the fixture's own choice of a regiment carrier.
    use rebellion_core::world::CapitalShipClass;
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let is_alliance = faction == CockpitFaction::Alliance;
        let class = |is_alliance, troop_capacity, hyperdrive| CapitalShipClass {
            is_alliance,
            troop_capacity,
            hyperdrive,
            hull: 100,
            ..CapitalShipClass::default()
        };
        let mut carrier = None;
        let applied = apply_to_loading_world_with(request(Scenario::FleetLoad, faction), |world| {
            world.capital_ship_classes.insert(class(is_alliance, 0, 80));
            world.capital_ship_classes.insert(class(is_alliance, 2, 0));
            world
                .capital_ship_classes
                .insert(class(!is_alliance, 2, 80));
            carrier = Some(world.capital_ship_classes.insert(class(is_alliance, 3, 80)));
        });
        let fleet = applied.world.fleets.keys().next().unwrap();
        assert_eq!(
            applied.world.fleets[fleet].capital_ships[0].class,
            carrier.unwrap(),
            "{faction:?}"
        );
    }
}

#[test]
fn the_shield_hit_scenario_holds_an_undamaged_hull_with_one_shield_recharge_hit() {
    // A0 2026-11-15-partial-cells e-vd2 (8bce5f1cbf27): the Empire's
    // Victory Destroyer 2, Ship Damaged Yes with hull 1800:1800 and
    // Shield Recharge Rate 8:10: one hit against the shield recharge
    // (+0x64 bits 0..3, FUN_00501510; FUN_00500fa0).
    use rebellion_core::world::Subsystem;
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let applied =
            apply_to_loading_world_with(request(Scenario::ShipShieldHit, faction), |world| {
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        name: "Victory Destroyer".into(),
                        is_alliance: false,
                        hull: 1800,
                        hyperdrive: 60,
                        troop_capacity: 2,
                        fighter_capacity: 2,
                        shield_recharge_rate: 10,
                        ..Default::default()
                    });
            });
        let fleet = applied.world.fleets.keys().next().unwrap();
        let ship = &applied.world.fleets[fleet].capital_ships[0];
        let class = &applied.world.capital_ship_classes[ship.class];
        assert_eq!(ship.hits(Subsystem::Shield), 1, "{faction:?}");
        assert_eq!(ship.subsystem_hits, 1, "{faction:?}");
        assert_eq!(ship.hull_current, class.hull as i32, "{faction:?}");
        if faction == CockpitFaction::Empire {
            assert_eq!(class.name, "Victory Destroyer");
            assert_eq!(ship.name.as_deref(), Some("Victory Destroyer 2"));
        }
    }
}

#[test]
fn the_ship_repaired_scenario_mends_the_shield_hit_at_its_sides_shipyard() {
    // A0 2026-11-16-empire-hunt (an Empire game): Victory Destroyer 2,
    // damaged (h-3, 96f59062, Shield Recharge 8:10 in h-4), is whole
    // after an Orbital Shipyard deploys at Coruscant: h-28 (34d1e021)
    // "Ship Damaged: No", h-29 (fd2acfbd) 10:10, Hull 1800:1800; h-82 /
    // h-83 show the change from day 603 to 604. The port's repair:
    // timer 0x380 with FastRepair (FUN_0050c580, FUN_0050bb70), each
    // fire FUN_00501c50.
    use rebellion_core::world::Subsystem;
    let faction = CockpitFaction::Empire;
    let victory = rebellion_core::world::CapitalShipClass {
        name: "Victory Destroyer".into(),
        is_alliance: false,
        hull: 1800,
        hyperdrive: 60,
        troop_capacity: 2,
        fighter_capacity: 2,
        shield_recharge_rate: 10,
        damage_control: 50,
        ..Default::default()
    };
    let applied = apply_to_loading_world_with(request(Scenario::ShipRepaired, faction), |world| {
        world.capital_ship_classes.insert(victory);
    });
    let fleet = applied.world.fleets.keys().next().unwrap();
    let ship = &applied.world.fleets[fleet].capital_ships[0];
    let class = &applied.world.capital_ship_classes[ship.class];
    assert_eq!(class.name, "Victory Destroyer");
    assert_eq!(ship.subsystem_hits, 0);
    assert_eq!(ship.hits(Subsystem::Shield), 0);
    assert_eq!(ship.hull_current, class.hull as i32);
    let at = applied.world.fleets[fleet].location;
    assert!(applied.world.systems[at]
        .manufacturing_facilities
        .iter()
        .any(|&key| applied.world.manufacturing_facilities[key].is_shipyard));
}

#[test]
fn without_a_carrier_the_fleet_load_scenario_builds_one_for_the_players_side() {
    // port: the fixture's fallback class.
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let applied = apply_to_loading_world(request(Scenario::FleetLoad, faction));
        let fleet = applied.world.fleets.keys().next().unwrap();
        let class =
            &applied.world.capital_ship_classes[applied.world.fleets[fleet].capital_ships[0].class];
        assert_eq!(class.name, "Transport");
        assert_eq!(class.is_alliance, faction == CockpitFaction::Alliance);
        assert_eq!(
            (class.hull, class.hyperdrive, class.troop_capacity),
            (100, 80, 2)
        );
    }
}

#[test]
fn the_refused_unload_scenario_starts_with_the_regiment_held_aboard_and_the_target_in_the_system_window(
) {
    // port: the fixture's own layout; FUN_0053d430 refuses the drop
    // (other side, populated).
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let request = request(Scenario::RegimentUnloadRefused, faction);
        let applied = apply_to_loading_world(request);
        let world = &applied.world;
        let mut keys = world.systems.keys();
        let (here, target) = (keys.next().unwrap(), keys.next().unwrap());
        let fleet = world.fleets.keys().next().unwrap();
        let is_alliance = faction == CockpitFaction::Alliance;
        let enemy = if is_alliance {
            Faction::Empire
        } else {
            Faction::Alliance
        };

        assert_eq!(
            world.systems[target].control,
            ControlKind::Controlled(enemy),
            "{faction:?}"
        );
        assert!(world.systems[target].is_populated);
        assert!(world.systems[here].ground_units.is_empty());
        let [regiment] = applied.transport.cargo(fleet) else {
            panic!("{faction:?}: one regiment starts aboard");
        };
        assert_eq!(world.troops[*regiment].is_alliance, is_alliance);
        assert_eq!(applied.defenses.window_count(), 1);
        assert!(applied.defenses.is_open(target));

        let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
        let setup = fleet_load_setup(request, world, &applied.sectors, &applied.defenses).unwrap();
        assert_eq!(setup.troop_item, None);
        assert!(applied
            .defenses
            .contains_screen_point(layout, setup.defenses_window));

        let mut watch = FleetLoadWatch::default();
        let start = watch
            .next(
                request,
                world,
                &FogState::new(enemy),
                &applied.movement,
                &applied.transport,
                &FleetWindowState::default(),
                false,
                &[],
                None,
                None,
            )
            .unwrap();
        assert!(start.aboard, "{faction:?}");
        assert_eq!((start.cargo, start.troop_system_dat_id), (1, None));
    }
}

#[test]
fn the_fleet_load_scenario_leaves_fleets_elsewhere_where_they_are() {
    // port: only fleets at the loading and target systems are moved.
    let mut kept = None;
    let applied = apply_to_loading_world_with(
        request(Scenario::FleetLoad, CockpitFaction::Alliance),
        |world| {
            let copy = world.systems.values().nth(2).unwrap().clone();
            let fourth = world.systems.insert(copy);
            let fleet = world
                .fleets
                .insert(world.fleets.values().next().unwrap().clone());
            world.fleets[fleet].location = fourth;
            kept = Some((fleet, fourth));
        },
    );
    let (fleet, fourth) = kept.unwrap();
    assert_eq!(applied.world.fleets[fleet].location, fourth);
}

#[test]
fn the_fleet_load_scenarios_regiment_is_of_the_players_troop_class() {
    // port: the fixture copies the class of a regiment of the player's side.
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let is_alliance = faction == CockpitFaction::Alliance;
        let applied = apply_to_loading_world_with(request(Scenario::FleetLoad, faction), |world| {
            for (side, raw) in [(!is_alliance, 0x1000_0010), (is_alliance, 0x1000_0020)] {
                world.troops.insert(TroopUnit {
                    class_dat_id: rebellion_core::ids::DatId::new(raw),
                    is_alliance: side,
                    regiment_strength: 100,
                });
            }
        });
        let here = applied.world.systems.keys().next().unwrap();
        let regiment = loading_regiment(&applied.world, here).unwrap();
        assert_eq!(
            applied.world.troops[regiment].class_dat_id.raw(),
            0x1000_0020,
            "{faction:?}"
        );
    }
}

#[test]
fn the_alliance_fleet_load_regiment_is_an_army_regiment_when_the_side_has_one() {
    // A0 assault-clean's second row: an Alliance Army Regiment (TROOPSD 2).
    let applied = apply_to_loading_world_with(request(Scenario::FleetLoad, CockpitFaction::Alliance), |world| {
        for raw in [0x1000_0020, 0x1000_0002] {
            world.troops.insert(TroopUnit {
                class_dat_id: rebellion_core::ids::DatId::new(raw),
                is_alliance: true,
                regiment_strength: 100,
            });
        }
    });
    let here = applied.world.systems.keys().next().unwrap();
    let regiment = loading_regiment(&applied.world, here).unwrap();
    assert_eq!(applied.world.troops[regiment].class_dat_id.raw(), 0x1000_0002);
}

#[test]
fn the_fleet_load_scenarios_defenses_window_sits_five_pixels_into_the_galaxy_view() {
    // port: the fixture's own placement.
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let applied = apply_to_loading_world(request(Scenario::FleetLoad, faction));
        let galaxy = CockpitState::new(faction).layout_for(640.0, 480.0).galaxy;
        let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
        let inside = |x: f32, y: f32| applied.defenses.contains_screen_point(layout, (x, y));
        let (x, y) = (galaxy.x + 5.0, galaxy.y + 5.0);
        assert!(inside(x, y), "{faction:?}");
        assert!(!inside(x - 0.5, y) && !inside(x, y - 0.5), "{faction:?}");
    }
}

#[test]
fn the_fleet_load_watch_reports_only_its_own_regiment_as_travelling() {
    // port: the fixture's own report of TroopTransportState::transits.
    use rebellion_core::troop_transport::RegimentTarget;
    let request = request(Scenario::FleetLoad, CockpitFaction::Alliance);
    let mut applied = apply_to_loading_world(request);
    let mut keys = applied.world.systems.keys();
    let (here, target) = (keys.next().unwrap(), keys.next().unwrap());
    let regiment = loading_regiment(&applied.world, here).unwrap();
    let other = applied
        .world
        .troops
        .insert(applied.world.troops[regiment].clone());
    applied.world.systems[here].ground_units.push(other);
    let fog = FogState::new(Faction::Alliance);
    let fleets = FleetWindowState::default();
    let observe = |applied: &Applied| {
        fleet_load_observation(
            request,
            regiment,
            &applied.world,
            &fog,
            &applied.movement,
            &applied.transport,
            &fleets,
            false,
            &[],
        )
        .unwrap()
    };

    let blockaded = std::collections::HashSet::new();
    applied
        .transport
        .move_regiment(
            &mut applied.world,
            &applied.movement,
            &blockaded,
            other,
            RegimentTarget::System(target),
            0,
        )
        .unwrap();
    assert!(!observe(&applied).regiment_travelling);

    applied
        .transport
        .move_regiment(
            &mut applied.world,
            &applied.movement,
            &blockaded,
            regiment,
            RegimentTarget::System(target),
            0,
        )
        .unwrap();
    let travelling = observe(&applied);
    assert!(travelling.regiment_travelling);
    assert_eq!(travelling.troop_system_dat_id, None);

    use rebellion_render::MessageCategory;
    let line = |text: &str| GameMessage::new(0, text.to_string(), MessageCategory::Event);
    let messages = [
        line("Regiment move rejected: the destination is blockaded"),
        line("Regiment arrived at Cathar"),
        // The AI's own arrival goes to its side's index only.
        line("Regiment arrived at Svivren").on_rail(
            rebellion_render::MessageRail::Mission,
            rebellion_render::RailAudience::Empire,
        ),
        line("Luke Skywalker has reached Jedi Knight tier"),
    ];
    let later = fleet_load_observation(
        request,
        regiment,
        &applied.world,
        &fog,
        &applied.movement,
        &applied.transport,
        &fleets,
        false,
        &messages,
    )
    .unwrap();
    assert_eq!(
        later.last_regiment_message.as_deref(),
        Some("Regiment arrived at Cathar")
    );
}

#[test]
fn the_fleet_load_watch_reports_each_change_once() {
    // port: the fixture's own report.
    let request = request(Scenario::FleetLoad, CockpitFaction::Alliance);
    let mut applied = apply_to_loading_world(request);
    let here = applied.world.systems.keys().next().unwrap();
    let fleet = applied.world.fleets.keys().next().unwrap();
    let regiment = loading_regiment(&applied.world, here).unwrap();
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(here);
    let mut fleets = FleetWindowState::default();
    let mut watch = FleetLoadWatch::default();
    let layout = CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
    let mut next = |applied: &Applied, fleets: &FleetWindowState| {
        watch.next(
            request,
            &applied.world,
            &fog,
            &applied.movement,
            &applied.transport,
            fleets,
            false,
            &[],
            None,
            None,
        )
    };

    let start = next(&applied, &fleets).unwrap();
    assert_eq!(start.status, "fleet-load");
    assert!(!start.window_open && !start.aboard);
    assert_eq!(
        start.troop_system_dat_id,
        Some(applied.world.systems[here].dat_id.raw())
    );
    assert_eq!(next(&applied, &fleets), None);

    fleets.open(
        &applied.world,
        here,
        (300, 80),
        CockpitFaction::Alliance,
        layout,
    );
    let opened = next(&applied, &fleets).unwrap();
    assert!(opened.window_open);
    assert_eq!((opened.origin, opened.entries), (Some((300, 80)), 1));
    assert_eq!(
        opened.fleet_entry,
        Some((300.0 + 4.0 + 45.5, 80.0 + 29.0 + 25.0))
    );
    assert_eq!(
        opened.troops_tab,
        Some((300.0 + 99.0 + 65.0 + 15.5, 80.0 + 96.0 + 14.0))
    );

    applied
        .transport
        .embark(&mut applied.world, fleet, &[regiment])
        .unwrap();
    let loaded = next(&applied, &fleets).unwrap();
    assert!(loaded.aboard);
    assert_eq!((loaded.cargo, loaded.troop_system_dat_id), (1, None));

    let moving = FixtureRequest {
        scenario: Scenario::FleetMove,
        ..request
    };
    assert_eq!(
        FleetLoadWatch::default().next(
            moving,
            &applied.world,
            &fog,
            &applied.movement,
            &applied.transport,
            &fleets,
            false,
            &[],
            None,
            None,
        ),
        None
    );
}

#[test]
fn the_speed_menu_watch_reports_the_menu_when_it_opens_or_moves() {
    // port: the fixture's own report; the menu lists five speeds
    // (FUN_0042d190).
    use egui_macroquad::egui;
    let request = request(Scenario::FleetLoad, CockpitFaction::Alliance);
    let rect = egui::Rect::from_min_size(egui::pos2(100.0, 30.0), egui::vec2(90.0, 72.0));
    let mut watch = SpeedMenuWatch::default();

    assert_eq!(watch.next(request, None), None);
    assert_eq!(
        watch.next(request, Some(rect)),
        Some(FixtureSpeedMenu {
            status: "speed-menu",
            code: request.code,
            left: 100.0,
            top: 30.0,
            width: 90.0,
            height: 72.0,
            rows: 5,
        })
    );
    assert_eq!(watch.next(request, Some(rect)), None);
    assert_eq!(watch.next(request, None), None);
    assert!(watch.next(request, Some(rect)).is_some());
    let moving = FixtureRequest {
        scenario: Scenario::FleetMove,
        ..request
    };
    assert_eq!(SpeedMenuWatch::default().next(moving, Some(rect)), None);
    // The mission icon's gate runs the clock to start its missions.
    let quadrants = FixtureRequest {
        scenario: Scenario::Quadrants,
        ..request
    };
    assert!(SpeedMenuWatch::default()
        .next(quadrants, Some(rect))
        .is_some());
}

#[test]
fn the_fleet_load_setup_reports_the_day_readouts_centre() {
    // GameSpeed: the readout is the Game Speed control (day_readout_rect).
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let loading = request(Scenario::FleetLoad, faction);
        let applied = apply_to_loading_world(loading);
        let setup =
            fleet_load_setup(loading, &applied.world, &applied.sectors, &applied.defenses).unwrap();
        let day = day_readout_rect(faction);
        assert_eq!(
            setup.day_readout,
            (day.x + day.width / 2.0, day.y + day.height / 2.0),
            "{faction:?}"
        );
        assert_eq!(setup.status, "fleet-load-setup");
        let moving = request(Scenario::FleetMove, faction);
        assert_eq!(
            fleet_load_setup(moving, &applied.world, &applied.sectors, &applied.defenses),
            None
        );
    }
}

#[test]
fn the_full_fleet_load_scenario_leaves_no_room_for_the_regiment() {
    // FUN_00500b40: the room is the capacity less the regiments aboard.
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let mut applied = apply_to_loading_world(request(Scenario::FleetLoadFull, faction));
        let here = applied.world.systems.keys().next().unwrap();
        let fleet = applied.world.fleets.keys().next().unwrap();
        let capacity = TroopTransportState::fleet_capacity(&applied.world, fleet).unwrap();
        assert_eq!(applied.transport.carried_count(fleet), capacity as usize);
        let regiment = loading_regiment(&applied.world, here).unwrap();
        assert_eq!(applied.world.systems[here].ground_units, [regiment]);
        assert_eq!(
            applied
                .transport
                .embark(&mut applied.world, fleet, &[regiment]),
            Err(
                rebellion_core::troop_transport::TroopTransportError::CapacityExceeded {
                    capacity,
                    requested: capacity + 1,
                }
            )
        );
    }
}

#[test]
fn the_fleet_move_scenario_leaves_the_players_fleet_alone_and_movable_beside_its_target() {
    use rebellion_core::movement::{fleet_move_confirms, validate_fleet_dispatch};
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let request = request(Scenario::FleetMove, faction);
        let applied = apply_to_fleet_world(request);
        let world = &applied.world;
        let mut systems = world.systems.keys();
        let (here, target, elsewhere) = (
            systems.next().unwrap(),
            systems.next().unwrap(),
            systems.next().unwrap(),
        );
        let fleets: Vec<_> = world.fleets.keys().collect();
        let (fleet, other, away) = (fleets[0], fleets[1], fleets[2]);
        let is_alliance = faction == CockpitFaction::Alliance;

        assert_eq!(world.systems[here].fleets, [fleet], "{faction:?}");
        assert_eq!(world.fleets[other].location, elsewhere);
        assert_eq!(world.fleets[away].location, target);
        assert_eq!(world.fleets[fleet].is_alliance, is_alliance);
        assert_eq!(
            validate_fleet_dispatch(&applied.movement, world, fleet, target, is_alliance),
            Ok(())
        );
        assert!(!fleet_move_confirms(
            world,
            applied.blockade.blockaded_systems(),
            fleet,
            false
        ));

        let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
        // The Fleet window's top-left corner, 5 pixels in from the
        // galaxy view's top-right.
        let galaxy = layout.galaxy;
        let corner = (
            galaxy.x + galaxy.width - FLEET_WINDOW_WIDTH - 5.0,
            galaxy.y + 5.0,
        );
        assert!(applied.fleets.contains_screen_point(layout, corner));
        assert!(!applied
            .fleets
            .contains_screen_point(layout, (corner.0 - 1.0, corner.1)));
        assert!(!applied
            .fleets
            .contains_screen_point(layout, (corner.0, corner.1 - 1.0)));
        let setup = fleet_move_setup(request, world, &applied.sectors, &applied.fleets)
            .expect("the scenario reports its points");
        let item = (setup.fleet_item_x, setup.fleet_item_y);
        let planet = (setup.target_screen_x, setup.target_screen_y);
        assert!(applied.fleets.contains_screen_point(layout, item));
        assert!(applied.sectors.contains_screen_point(layout, planet));
        assert!(!applied.fleets.contains_screen_point(layout, planet));
        assert_eq!(
            (setup.primary_dat_id, setup.target_dat_id),
            (0x9000_0001, 0x9000_0002)
        );
        assert_eq!(setup.systems.len(), 3);
        // The sector window's planets, the target's among them, give the
        // gate points that select a system where no bare map is left.
        assert!(!setup.sector_planets.is_empty());
        assert!(setup.sector_planets.iter().all(|point| applied
            .sectors
            .contains_screen_point(layout, (point.x, point.y))));
        assert!(setup
            .sector_planets
            .iter()
            .any(|point| point.dat_id == setup.target_dat_id && (point.x, point.y) == planet));

        let observed = fleet_move_observation(
            request,
            world,
            &applied.movement,
            false,
            &applied.map,
            (
                &applied.fleets,
                &FogState::new(Faction::Alliance),
                &applied.transport,
            ),
            &[],
        )
        .unwrap();
        assert_eq!(
            (
                observed.in_transit,
                observed.destination_dat_id,
                observed.selected_system_dat_id,
                observed.last_message,
            ),
            (false, None, None, None)
        );
    }
}

#[test]
fn the_blockade_variant_leaves_the_fleet_blockading_an_enemy_system() {
    use rebellion_core::movement::{fleet_move_confirms, validate_fleet_dispatch};
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let applied = apply_to_fleet_world(request(Scenario::FleetMoveBlockade, faction));
        let world = &applied.world;
        let mut systems = world.systems.keys();
        let (here, target) = (systems.next().unwrap(), systems.next().unwrap());
        let fleet = world.fleets.keys().next().unwrap();
        let blockaded = applied.blockade.blockaded_systems();
        let is_alliance = faction == CockpitFaction::Alliance;

        assert!(blockaded.contains(&here), "{faction:?}");
        // No order path refuses a fleet leaving a blockade: the System
        // window's 0x214 carries only facilities (FUN_00422ce0 type 9,
        // FUN_004568a0), and FUN_005073d0 lifts its 0x90/4 for fleets.
        assert_eq!(
            validate_fleet_dispatch(&applied.movement, world, fleet, target, is_alliance),
            Ok(())
        );
        // The enemy holds the system, so a menu Move does not ask.
        assert!(!fleet_move_confirms(world, blockaded, fleet, false));
    }
}

#[test]
fn a_dispatched_fleet_reports_its_transit_and_the_last_message() {
    let request = request(Scenario::FleetMove, CockpitFaction::Alliance);
    let mut applied = apply_to_fleet_world(request);
    let fleet = applied.world.fleets.keys().next().unwrap();
    let target = applied.world.systems.keys().nth(1).unwrap();
    assert!(begin_fleet_transit(
        &mut applied.movement,
        &mut applied.world,
        fleet,
        target,
        5
    ));
    applied.map.selected_system = Some(target);
    let message = GameMessage::new(
        0,
        "Fleet move rejected",
        rebellion_render::MessageCategory::Event,
    );

    let fog = FogState::new(Faction::Alliance);
    let observe = |applied: &Applied, message: &GameMessage| {
        fleet_move_observation(
            request,
            &applied.world,
            &applied.movement,
            true,
            &applied.map,
            (&applied.fleets, &fog, &applied.transport),
            std::slice::from_ref(message),
        )
        .unwrap()
    };
    let observed = observe(&applied, &message);
    // No window at the destination is open yet.
    assert_eq!(observed.destination_entry, None);

    assert_eq!(
        (
            observed.in_transit,
            observed.destination_dat_id,
            observed.confirmation_open,
            observed.selected_system_dat_id,
            observed.last_message.as_deref(),
        ),
        (
            true,
            Some(0x9000_0002),
            true,
            Some(0x9000_0002),
            Some("Fleet move rejected")
        )
    );

    // move-order.md: the destination's Fleet window lists the fleet
    // en route (a0-vm/menus/18-fleet4.png), so the gate can open its
    // menu there.
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let point = applied
        .sectors
        .fleet_window_point(&applied.world, layout, target)
        .unwrap();
    assert!(applied
        .fleets
        .open(&applied.world, target, point, request.faction, layout));
    let opened = observe(&applied, &message);
    let [x, y] = opened.destination_entry.unwrap();
    let window = applied.fleets.screen_rect(layout, target).unwrap();
    assert!(window.contains(egui_macroquad::egui::pos2(x, y)));
    // Its ships are on the right list once the fleet is the selection.
    assert_eq!(opened.destination_item, None);
    assert!(applied
        .fleets
        .select(target, FleetWindowEntry::Fleet(fleet)));
    let [x, y] = observe(&applied, &message).destination_item.unwrap();
    let item = applied.fleets.item_screen_rect(layout, target, 0).unwrap();
    assert_eq!(egui_macroquad::egui::pos2(x, y), item.center());
}

#[test]
fn only_the_fleet_move_scenarios_report_fleet_moves() {
    let request = request(Scenario::MissionTargeting, CockpitFaction::Alliance);
    let applied = apply_to_fleet_world(request);

    assert_eq!(
        fleet_move_setup(request, &applied.world, &applied.sectors, &applied.fleets),
        None
    );
    assert_eq!(
        fleet_move_observation(
            request,
            &applied.world,
            &applied.movement,
            false,
            &applied.map,
            (
                &applied.fleets,
                &FogState::new(Faction::Alliance),
                &applied.transport
            ),
            &[]
        ),
        None
    );
}

/// The quadrant scenario on the loading world (three systems, the third
/// in its own sector) plus a fourth in that sector, with one Alliance
/// fleet that carries the world's first agent, a bystander fleet and
/// agent at the fourth system, an agent waiting at the second system,
/// and three more agents of each side nowhere.
fn apply_quadrants(faction: CockpitFaction) -> Applied {
    apply_quadrants_as(Scenario::Quadrants, faction)
}

fn apply_quadrants_as(scenario: Scenario, faction: CockpitFaction) -> Applied {
    apply_to_loading_world_with(request(scenario, faction), |world| {
        use rebellion_core::world::{Character, Fleet};
        let carried = world.characters.keys().next().unwrap();
        let mut keys = world.systems.keys();
        let (second, third) = (keys.nth(1).unwrap(), keys.next().unwrap());
        let mut fourth = world.systems[third].clone();
        fourth.dat_id = rebellion_core::ids::DatId::new(0x9000_0004);
        let fourth = world.systems.insert(fourth);
        let far = world.systems[third].sector;
        world.sectors[far].systems.push(fourth);
        world.characters[carried].current_system = Some(third);
        for (location, is_alliance) in [(fourth, false), (fourth, true)] {
            world.fleets.insert(Fleet {
                location,
                is_alliance,
                ..Default::default()
            });
        }
        for (name, location) in [("Bystander", fourth), ("Waiting", second)] {
            world.characters.insert(Character {
                name: name.into(),
                is_alliance: false,
                is_empire: false,
                current_system: Some(location),
                ..Default::default()
            });
        }
        world.fleets.insert(Fleet {
            location: third,
            characters: vec![carried],
            is_alliance: true,
            ..Default::default()
        });
        for (index, is_alliance) in [true, true, true, false, false, false]
            .into_iter()
            .enumerate()
        {
            world.characters.insert(Character {
                name: format!("Agent {index}"),
                is_alliance,
                is_empire: !is_alliance,
                recruited: true,
                ..Default::default()
            });
        }
    })
}

#[test]
fn the_quadrant_scenario_gives_each_checked_icon_its_side() {
    // port: the fixture's own layout; the sides follow FUN_0045cdc0,
    // FUN_0045ce80, FUN_0045ccc0 and FUN_004a1f60.
    use rebellion_render::quadrant_icons::quadrant_side;
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let applied = apply_quadrants(faction);
        let world = &applied.world;
        let mut keys = world.systems.keys();
        let (primary, second) = (keys.next().unwrap(), keys.next().unwrap());
        let (player, own, other) = match faction {
            CockpitFaction::Alliance => (Faction::Alliance, 1, 2),
            CockpitFaction::Empire => (Faction::Empire, 2, 1),
        };
        let fog = FogState::new(player);
        let side = |system, quadrant| {
            quadrant_side(
                world,
                &fog,
                &applied.missions,
                &applied.movement,
                rebellion_render::quadrant_icons::Travellers::default(),
                player,
                system,
                quadrant,
            )
        };
        assert_eq!(side(primary, Quadrant::System), Some(own), "{faction:?}");
        assert_eq!(side(primary, Quadrant::Defenses), Some(own), "{faction:?}");
        assert_eq!(side(primary, Quadrant::Fleets), Some(own), "{faction:?}");
        assert_eq!(
            side(primary, Quadrant::Missions),
            Some(other),
            "{faction:?}"
        );
        assert_eq!(side(second, Quadrant::System), None, "{faction:?}");
        assert_eq!(side(second, Quadrant::Defenses), Some(other), "{faction:?}");
        assert_eq!(side(second, Quadrant::Fleets), None, "{faction:?}");
        assert_eq!(side(second, Quadrant::Missions), Some(own), "{faction:?}");
        assert_eq!(applied.missions.missions().len(), 3, "{faction:?}");
        assert_eq!(applied.sectors.window_count(), 1, "{faction:?}");

        // Only what the scenario placed lies at the two systems.
        let is_alliance = faction == CockpitFaction::Alliance;
        let own_fleets: Vec<_> = world
            .fleets
            .iter()
            .filter(|(_, fleet)| fleet.location == primary)
            .map(|(_, fleet)| fleet.is_alliance)
            .collect();
        assert_eq!(own_fleets, [is_alliance], "{faction:?}");
        assert!(world.fleets.values().all(|fleet| fleet.location != second));
        let at = |system| {
            world
                .characters
                .values()
                .filter(|character| character.current_system == Some(system))
                .count()
        };
        // The second system's agent is in hyperspace to it.
        assert_eq!((at(primary), at(second)), (2, 0), "{faction:?}");
        let bound: Vec<_> = applied
            .missions
            .en_route()
            .iter()
            .map(|transit| transit.to)
            .collect();
        assert_eq!(bound, [second], "{faction:?}");
        // What lies at the fourth system stays there.
        let fourth = world.systems.keys().nth(3).unwrap();
        assert_eq!(at(fourth), 1, "{faction:?}");
        assert_eq!(
            world
                .fleets
                .values()
                .filter(|fleet| fleet.location == fourth)
                .count(),
            2,
            "{faction:?}"
        );
        // Each mission is sent by its agent's side.
        for mission in applied.missions.missions() {
            let MissionMember::Character(agent) = mission.team[0] else {
                panic!("the scenario sends characters");
            };
            let sent_by_alliance = mission.faction == MissionFaction::Alliance;
            assert_eq!(
                sent_by_alliance, world.characters[agent].is_alliance,
                "{faction:?}"
            );
        }
    }
}

#[test]
fn the_regiment_scenarios_put_a_regiment_product_queued_or_travelling_in_the_primary() {
    // port: the fixture's own stocking for A0 2026-11-19-edited-saves-10
    // c-01 (queued) and c-05 (trained, on its way in).
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        for (scenario, travelling) in [
            (Scenario::QuadrantsRegimentQueued, false),
            (Scenario::QuadrantsRegimentEnRoute, true),
        ] {
            let applied = apply_quadrants_as(scenario, faction);
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            let held: Vec<_> = world.products.in_system(primary).collect();
            assert_eq!(held.len(), 1, "{scenario:?} {faction:?}");
            assert_eq!(
                held[0].is_alliance,
                faction == CockpitFaction::Alliance,
                "{scenario:?} {faction:?}"
            );
            assert_ne!(held[0].yard, primary, "{scenario:?} {faction:?}");
            assert_eq!(
                world.products.arrival(held[0].id).is_some(),
                travelling,
                "{scenario:?} {faction:?}"
            );
        }
    }
}

#[test]
fn the_free_team_scenario_leaves_the_players_agent_at_the_primary_free_to_move() {
    // port: the fixture's own stocking; the team is FUN_00512700 kind 8
    // and its Move check the 0x241's FUN_0054c110 (ghidra/notes/
    // sector-icon-menus.md), as A0 2026-11-13-defenses-missions-drop/00
    // enables Jerijador's defenses icon's Move.
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let applied = apply_quadrants_as(Scenario::QuadrantsFreeTeam, faction);
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        let (player, side) = match faction {
            CockpitFaction::Alliance => (Faction::Alliance, MissionFaction::Alliance),
            CockpitFaction::Empire => (Faction::Empire, MissionFaction::Empire),
        };
        // The other side's agent at the primary and the player's in
        // hyperspace to the second keep their missions.
        assert_eq!(applied.missions.missions().len(), 2, "{faction:?}");
        let team = rebellion_render::quadrant_icons::defenses_move_team(
            world,
            &applied.missions,
            player,
            primary,
        );
        assert_eq!(
            (team.members.len(), team.troops.len()),
            (1, 1),
            "{faction:?}"
        );
        let MissionMember::Character(agent) = team.members[0] else {
            panic!("the team's member is the player's agent");
        };
        assert!(!world.characters[agent].on_mission, "{faction:?}");
        assert!(
            applied
                .missions
                .move_order_enabled(world, side, &team.members),
            "{faction:?}"
        );
        // The battery stays behind the team.
        assert_eq!(
            world.systems[primary].defense_facilities.len(),
            1,
            "{faction:?}"
        );
    }
}

#[test]
fn the_frozen_headquarters_scenario_gives_the_empire_a_copy_holding_the_headquarters() {
    // FUN_0045bbb0 marks a system whose viewer copy holds a family 0x20
    // object (FUN_00526cf0); A0 2026-11-16-edited-saves/x-29 shows the
    // mark to the Empire. The Alliance holds its own system: no copy.
    let in_sector = |world: &mut GameWorld| {
        // Three more systems in the primary's sector.
        let here = world.systems.keys().next().unwrap();
        let sector = world.systems[here].sector;
        for index in 0..3u16 {
            let mut copy = world.systems[here].clone();
            copy.dat_id = rebellion_core::ids::DatId::new(0x9000_0010 + u32::from(index));
            copy.x = copy.x.saturating_add(200 * (index + 1));
            copy.manufacturing_facilities.clear();
            copy.production_facilities.clear();
            copy.defense_facilities.clear();
            let copy = world.systems.insert(copy);
            world.sectors[sector].systems.push(copy);
        }
    };
    let applied = apply_to_loading_world_with(
        request(
            Scenario::QuadrantsHeadquartersFrozen,
            CockpitFaction::Empire,
        ),
        in_sector,
    );
    let world = &applied.world;
    let primary = world.systems.keys().next().unwrap();
    let headquarters = mark_systems(world, primary)[0];
    assert!(world.systems[headquarters].is_headquarters);
    let view = world
        .object_views
        .system(Faction::Empire, world.systems[headquarters].dat_id)
        .expect("the Empire's copy");
    assert!(!view.live);
    assert!(view
        .manufacturing_facilities
        .iter()
        .any(|facility| facility.class.family() == 0x20));
    let alliance = apply_to_loading_world_with(
        request(
            Scenario::QuadrantsHeadquartersFrozen,
            CockpitFaction::Alliance,
        ),
        in_sector,
    );
    let id = alliance.world.systems[headquarters].dat_id;
    assert!(alliance
        .world
        .object_views
        .system(Faction::Empire, id)
        .is_none());
    assert_eq!(
        Scenario::decode(137),
        Some(Scenario::QuadrantsHeadquartersFrozen)
    );
}

// port: the fixture's own stocking for OBJ-13-C001. Hidden: the
// Empire's fog shows the headquarters system but it has no copy of it
// (FUN_0045bbb0 reads only the copy, FUN_004f3220). Live: its copy is
// live and holds the headquarters object.
#[test]
fn the_hidden_and_live_headquarters_scenarios_give_the_empire_no_copy_and_a_live_one() {
    let in_sector = |world: &mut GameWorld| {
        let here = world.systems.keys().next().unwrap();
        let sector = world.systems[here].sector;
        for index in 0..3u16 {
            let mut copy = world.systems[here].clone();
            copy.dat_id = rebellion_core::ids::DatId::new(0x9000_0010 + u32::from(index));
            copy.x = copy.x.saturating_add(200 * (index + 1));
            copy.manufacturing_facilities.clear();
            copy.production_facilities.clear();
            copy.defense_facilities.clear();
            let copy = world.systems.insert(copy);
            world.sectors[sector].systems.push(copy);
        }
    };
    let hidden_request = request(
        Scenario::QuadrantsHeadquartersHidden,
        CockpitFaction::Empire,
    );
    let hidden = apply_to_loading_world_with(hidden_request, in_sector);
    let world = &hidden.world;
    let primary = world.systems.keys().next().unwrap();
    let headquarters = mark_systems(world, primary)[0];
    assert!(world.systems[headquarters].is_headquarters);
    let id = world.systems[headquarters].dat_id;
    assert!(world.object_views.system(Faction::Empire, id).is_none());
    let mut fog = FogState::new(Faction::Empire);
    assert_eq!(
        reveal_without_copy(hidden_request, world, &mut fog),
        Some(headquarters)
    );
    assert!(fog.is_visible(headquarters));
    let mut alliance_fog = FogState::new(Faction::Empire);
    assert_eq!(
        reveal_without_copy(
            request(
                Scenario::QuadrantsHeadquartersHidden,
                CockpitFaction::Alliance
            ),
            world,
            &mut alliance_fog,
        ),
        None
    );

    let live = apply_to_loading_world_with(
        request(Scenario::QuadrantsHeadquartersLive, CockpitFaction::Empire),
        in_sector,
    );
    let view = live
        .world
        .object_views
        .system(Faction::Empire, id)
        .expect("the Empire's copy");
    assert!(view.live);
    assert!(view
        .manufacturing_facilities
        .iter()
        .any(|facility| facility.class.family() == 0x20));
    assert_eq!(
        [Scenario::decode(147), Scenario::decode(148)],
        [
            Some(Scenario::QuadrantsHeadquartersHidden),
            Some(Scenario::QuadrantsHeadquartersLive)
        ]
    );
}

#[test]
fn the_facility_scenario_gives_the_primarys_facility_icon_managers_of_the_player() {
    // port: the fixture's own stocking; the facility icon's team is
    // FUN_00512700 kind 4 (quadrant_icons::facility_icon_has_managers),
    // as A0 2026-11-13-facility-icon-drop/08 enables Jerijador's
    // Destination.
    for (faction, player) in [
        (CockpitFaction::Alliance, Faction::Alliance),
        (CockpitFaction::Empire, Faction::Empire),
    ] {
        let plain = apply_quadrants(faction);
        let primary = plain.world.systems.keys().next().unwrap();
        assert!(
            !rebellion_render::quadrant_icons::facility_icon_has_managers(
                &plain.world,
                primary,
                player
            ),
            "{faction:?}"
        );
        let applied = apply_quadrants_as(Scenario::QuadrantsFacility, faction);
        assert!(
            rebellion_render::quadrant_icons::facility_icon_has_managers(
                &applied.world,
                primary,
                player
            ),
            "{faction:?}"
        );
    }
}

#[test]
fn the_quadrant_scenario_leaves_an_agent_aboard_a_fleet_where_it_is() {
    let applied = apply_quadrants(CockpitFaction::Alliance);
    let world = &applied.world;
    let carried = world.characters.keys().next().unwrap();
    let third = world.systems.keys().nth(2).unwrap();
    assert_eq!(world.characters[carried].current_system, Some(third));
    assert!(!world.characters[carried].on_mission);
    assert!(applied
        .missions
        .missions()
        .iter()
        .all(|mission| !mission.team.contains(&MissionMember::Character(carried))));
}

#[test]
fn the_quadrant_setup_reports_the_headquarters_trip_only_for_the_alliances_revolt() {
    // port: the gate reads the Status window's ETA against this trip
    // (headquarters_move::plan), the move's own days.
    for (faction, alliance) in [
        (CockpitFaction::Alliance, true),
        (CockpitFaction::Empire, false),
    ] {
        let request = request(Scenario::Quadrants, faction);
        let applied = apply_to_loading_world_with(request, |world| {
            // Three more systems in the primary's sector, 200 apart.
            let here = world.systems.keys().next().unwrap();
            let sector = world.systems[here].sector;
            for index in 0..3u16 {
                let mut copy = world.systems[here].clone();
                copy.dat_id = rebellion_core::ids::DatId::new(0x9000_0010 + u32::from(index));
                copy.x = copy.x.saturating_add(200 * (index + 1));
                copy.manufacturing_facilities.clear();
                copy.production_facilities.clear();
                copy.defense_facilities.clear();
                let copy = world.systems.insert(copy);
                world.sectors[sector].systems.push(copy);
            }
        });
        let world = &applied.world;
        let report = quadrant_setup(request, world, &applied.sectors).unwrap();
        let primary = world.systems.keys().next().unwrap();
        let marks = mark_systems(world, primary);
        assert!(marks.len() > 1, "the sector holds the HQ and a revolt");
        let (origin, revolt) = (marks[0], marks[1]);
        let expected = rebellion_core::headquarters_move::plan(
            world,
            &rebellion_core::troop_transport::TroopTransportState::default(),
            true,
            revolt,
        );
        if alliance {
            let (from, days) = expected.unwrap();
            assert_eq!(from, origin);
            assert!(days > 0, "the revolt lies 200 away");
            assert_eq!(report.headquarters_trip_days, Some(days));
        } else {
            // The Empire's revolt is no Alliance destination.
            assert!(expected.is_err());
            assert_eq!(report.headquarters_trip_days, None);
        }
    }
}

#[test]
fn the_quadrant_setup_reports_both_systems_four_rects_in_kind_order() {
    let applied = apply_quadrants(CockpitFaction::Empire);
    let request = request(Scenario::Quadrants, CockpitFaction::Empire);
    let report = quadrant_setup(request, &applied.world, &applied.sectors).unwrap();
    let world = &applied.world;
    let mut keys = world.systems.keys();
    let (primary, second) = (keys.next().unwrap(), keys.next().unwrap());
    let layout = CockpitState::new(CockpitFaction::Empire).layout_for(640.0, 480.0);
    assert_eq!(report.status, "quadrant-setup");
    assert_eq!(report.primary_dat_id, world.systems[primary].dat_id.raw());
    assert_eq!(report.second_dat_id, world.systems[second].dat_id.raw());
    assert_eq!(report.scale, layout.scale);
    let names = ["system", "defenses", "fleets", "missions"];
    let kinds = [
        Quadrant::System,
        Quadrant::Defenses,
        Quadrant::Fleets,
        Quadrant::Missions,
    ];
    let mut expected = Vec::new();
    for system in [primary, second] {
        for (quadrant, name) in kinds.into_iter().zip(names) {
            let rect = applied
                .sectors
                .quadrant_screen_rect(world, layout, system, quadrant)
                .unwrap();
            expected.push(FixtureQuadrant {
                system_dat_id: world.systems[system].dat_id.raw(),
                quadrant: name,
                rect: [rect.min.x, rect.min.y, rect.width(), rect.height()],
            });
        }
    }
    assert_eq!(report.quadrants, expected);
    // The sector's third and fourth systems, when it has them: neutral
    // with a mine, and destroyed.
    let marks = mark_systems(world, primary);
    assert_eq!(report.neutral.is_some(), marks.len() > 2);
    assert_eq!(report.destroyed.is_some(), marks.len() > 3);
    if let [_, _, neutral, ruin] = marks[..] {
        assert_eq!(world.systems[neutral].control, ControlKind::Uncontrolled);
        assert!(!world.systems[neutral].production_facilities.is_empty());
        assert!(world.systems[ruin].is_destroyed);
    }
    assert!(quadrant_setup(
        super::tests::request(Scenario::FleetLoad, CockpitFaction::Empire),
        world,
        &applied.sectors
    )
    .is_none());
}

/// A0 coruscant-676/14 (the VM's Empire game, day 679, after the
/// Alliance seized Coruscant): the Empire's Defenses window of Coruscant
/// shows the Alliance's side, Personnel pressed over an empty list,
/// Trooper Regiments full and the other tabs empty.
#[test]
fn the_seized_coruscant_scenario_leaves_the_alliance_one_regiment_there_in_the_empires_sight() {
    let request = request(Scenario::CoruscantSeized, CockpitFaction::Empire);
    let applied = apply_to_loading_world_with(request, |world| {
        let held = world.systems.keys().nth(1).unwrap();
        world.systems[held].name = "Coruscant".into();
        world.systems[held].control = ControlKind::Controlled(Faction::Empire);
        // A third system in the sector, which the quadrant scenario
        // would mark; this one marks nothing.
        let mut third = world.systems[held].clone();
        third.name = "Third".into();
        third.dat_id = rebellion_core::ids::DatId::new(0x9000_0005);
        let third = world.systems.insert(third);
        let sector = world.systems[held].sector;
        world.sectors[sector].systems.push(third);
        // `apply` moves the world's first character to the primary
        // system, so the one at Coruscant is a second.
        let mut agent = world.characters.values().next().unwrap().clone();
        agent.name = "Bystander".into();
        agent.current_system = Some(held);
        world.characters.insert(agent);
        let stormtroopers = world.troops.insert(TroopUnit {
            class_dat_id: rebellion_core::ids::DatId::new(0x1000_0006),
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[held].ground_units.push(stormtroopers);
    });
    let world = &applied.world;
    let here = world.systems.keys().next().unwrap();
    let held = super::coruscant(world).unwrap();
    assert_eq!(
        world.systems[held].control,
        ControlKind::Controlled(Faction::Alliance)
    );
    let [regiment] = world.systems[held].ground_units[..] else {
        panic!("{:?}", world.systems[held].ground_units);
    };
    assert!(world.troops[regiment].is_alliance);
    assert!(world.fleets.values().all(|fleet| fleet.location != held));
    // What stood there went to another sector.
    let agent = world
        .characters
        .values()
        .find(|character| character.name == "Bystander")
        .unwrap();
    let moved_to = agent.current_system.unwrap();
    assert_ne!(world.systems[moved_to].sector, world.systems[held].sector);
    assert!(world
        .characters
        .values()
        .all(|character| character.current_system != Some(held)));

    // The quadrant gate's second system is Coruscant.
    let setup = quadrant_setup(request, world, &applied.sectors).unwrap();
    assert_eq!(
        (setup.primary_dat_id, setup.second_dat_id),
        (
            world.systems[here].dat_id.raw(),
            world.systems[held].dat_id.raw()
        )
    );
    assert_eq!(setup.quadrants.len(), 8);
    assert_eq!((setup.headquarters, setup.uprising), (None, None));

    let mut fog_alliance = FogState::new(Faction::Alliance);
    let mut fog = FogState::new(Faction::Empire);
    reveal(request, world, &mut fog_alliance, &mut fog);
    // main.rs seeds each side's views from the fixture's fog after
    // reveal (object_views::seed_from_fog): the Empire's copy of the
    // Alliance regiment it sees on Coruscant.
    let mut seeded = world.clone();
    rebellion_core::object_views::seed_from_fog(
        &mut seeded,
        &rebellion_core::troop_transport::TroopTransportState::default(),
        &MovementState::new(),
        [&fog_alliance, &fog],
    );
    let world = &seeded;
    let economy = EconomyState::default();
    let manufacturing = rebellion_core::manufacturing::ManufacturingState::default();
    let deliveries = rebellion_core::delivery::DeliveryState::default();
    let missions_windows = MissionsWindowState::default();
    let mut defenses = DefensesWindowState::default();
    let layout = CockpitState::new(CockpitFaction::Empire).layout_for(640.0, 480.0);
    assert!(defenses.open(world, held, (200, 91), CockpitFaction::Empire, layout));
    let shown = QuadrantWatch::default()
        .next(
            request,
            world,
            &QuadrantWindows {
                systems: &applied.systems,
                defenses: &defenses,
                missions_windows: &missions_windows,
                fog: &fog,
                missions: &applied.missions,
                production: rebellion_render::DefensesProduction {
                    manufacturing: &manufacturing,
                    deliveries: &deliveries,
                    regiments: &[],
                    squadrons: &[],
                },
                economy: &economy,
            },
        )
        .unwrap();
    let [window] = &shown.defenses_windows[..] else {
        panic!("{:?}", shown.defenses_windows);
    };
    assert_eq!((window.side, window.page), (1, "personnel"));
    assert_eq!(window.counts, [0, 1, 0, 0, 0]);
}

/// A0 2026-11-11-icon-menu-topic/05 (the VM's Empire game): Coruscant's
/// Fighter Squadrons page lists ten TIE Fighter squadrons, past the
/// list's three lines, and the fixture reports every cell's rect.
#[test]
fn the_coruscant_squadrons_scenario_lists_ten_tie_fighters_there_with_every_cell() {
    let request = request(Scenario::CoruscantSquadrons, CockpitFaction::Empire);
    let applied = apply_to_loading_world_with(request, |world| {
        let held = world.systems.keys().nth(1).unwrap();
        world.systems[held].name = "Coruscant".into();
        // The Empire's other classes and the Alliance's own TIE-named
        // one are not the squadrons placed.
        for (raw, name, is_alliance) in [
            (0x1c00_0006, "TIE Interceptor", false),
            (0x1c00_0001, "TIE Fighter", true),
        ] {
            world
                .fighter_classes
                .insert(rebellion_core::world::FighterClass {
                    dat_id: rebellion_core::ids::DatId::new(raw),
                    name: name.into(),
                    is_alliance,
                    is_empire: !is_alliance,
                    ..Default::default()
                });
        }
        world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                dat_id: rebellion_core::ids::DatId::new(0x1c00_0005),
                name: "TIE Fighter".into(),
                is_empire: true,
                ..Default::default()
            });
    });
    let world = &applied.world;
    let held = super::coruscant(world).unwrap();
    assert_eq!(
        world.systems[held].control,
        ControlKind::Controlled(Faction::Empire)
    );
    let [entry] = &world.systems[held].fighters[..] else {
        panic!("{:?}", world.systems[held].fighters);
    };
    assert_eq!((entry.count, entry.carrier), (10, None));
    assert_eq!(world.fighter_classes[entry.class].dat_id.raw(), 0x1c00_0005);
    let setup = quadrant_setup(request, world, &applied.sectors).unwrap();
    assert_eq!(setup.second_dat_id, world.systems[held].dat_id.raw());

    let mut fog_alliance = FogState::new(Faction::Alliance);
    let mut fog = FogState::new(Faction::Empire);
    fog.reveal(held);
    reveal(request, world, &mut fog_alliance, &mut fog);
    let economy = EconomyState::default();
    let manufacturing = rebellion_core::manufacturing::ManufacturingState::default();
    let deliveries = rebellion_core::delivery::DeliveryState::default();
    let missions_windows = MissionsWindowState::default();
    let mut defenses = DefensesWindowState::default();
    let layout = CockpitState::new(CockpitFaction::Empire).layout_for(640.0, 480.0);
    assert!(defenses.open(world, held, (203, 91), CockpitFaction::Empire, layout));
    assert!(defenses.show_page(held, DefensesPage::Squadrons));
    let shown = QuadrantWatch::default()
        .next(
            request,
            world,
            &QuadrantWindows {
                systems: &applied.systems,
                defenses: &defenses,
                missions_windows: &missions_windows,
                fog: &fog,
                missions: &applied.missions,
                production: rebellion_render::DefensesProduction {
                    manufacturing: &manufacturing,
                    deliveries: &deliveries,
                    regiments: &[],
                    squadrons: &[],
                },
                economy: &economy,
            },
        )
        .unwrap();
    let [window] = &shown.defenses_windows[..] else {
        panic!("{:?}", shown.defenses_windows);
    };
    assert_eq!((window.side, window.page), (2, "squadrons"));
    assert_eq!(window.rows.len(), 10);
    // The tenth cell, the fourth line's first, starts below the list.
    assert_eq!(window.cells.len(), 10);
    assert_eq!(window.cells[9][1] - window.cells[0][1], 210.0);
}

#[test]
fn the_quadrant_watch_reports_the_open_windows_and_the_rail_when_they_change() {
    let mut applied = apply_quadrants(CockpitFaction::Alliance);
    let request = request(Scenario::Quadrants, CockpitFaction::Alliance);
    let primary = applied.world.systems.keys().next().unwrap();
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(primary);
    let economy = EconomyState::default();
    let manufacturing = rebellion_core::manufacturing::ManufacturingState::default();
    let deliveries = rebellion_core::delivery::DeliveryState::default();
    let mut defenses = DefensesWindowState::default();
    let mut missions_windows = MissionsWindowState::default();
    let mut watch = QuadrantWatch::default();
    macro_rules! next {
        ($watch:expr) => {
            $watch.next(
                request,
                &applied.world,
                &QuadrantWindows {
                    systems: &applied.systems,
                    defenses: &defenses,
                    missions_windows: &missions_windows,
                    fog: &fog,
                    missions: &applied.missions,
                    production: rebellion_render::DefensesProduction {
                        manufacturing: &manufacturing,
                        deliveries: &deliveries,
                        regiments: &[],
                        squadrons: &[],
                    },
                    economy: &economy,
                },
            )
        };
    }
    let first = next!(watch).unwrap();
    assert_eq!(first.status, "quadrant-observation");
    assert!(first.system_windows.is_empty() && first.defenses_windows.is_empty());
    assert!(first.missions_windows.is_empty());
    assert!(first.rail.is_empty());
    assert!(next!(watch).is_none());

    let layout = CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
    let dat = applied.world.systems[primary].dat_id.raw();
    assert!(applied.systems.open(
        &applied.world,
        primary,
        (100, 50),
        CockpitFaction::Alliance,
        layout
    ));
    let opened = next!(watch).unwrap();
    assert_eq!(opened.system_windows, [(dat, (100, 50))]);

    // The primary system's Defenses window: the player's regiment and
    // KDY-150, the garrison line on the regiment page, and each tab's
    // and listed cell's rect.
    assert!(defenses.open(
        &applied.world,
        primary,
        (200, 60),
        CockpitFaction::Alliance,
        layout
    ));
    let shown = next!(watch).unwrap();
    let [window] = &shown.defenses_windows[..] else {
        panic!("{:?}", shown.defenses_windows);
    };
    assert_eq!(window.system_dat_id, dat);
    assert_eq!(window.origin, (200, 60));
    assert_eq!((window.side, window.page), (1, "personnel"));
    assert_eq!(window.counts, [0, 1, 0, 0, 1]);
    assert!(window.rows.is_empty() && window.cells.is_empty());
    assert_eq!((window.selected, window.garrison.as_deref()), (None, None));
    let tab = |name: &str| {
        window
            .tabs
            .iter()
            .find(|(tab, _)| *tab == name)
            .map(|(_, rect)| *rect)
    };
    let origin = |x: f32, y: f32| {
        [
            layout.canvas.x + (200.0 + x) * layout.scale,
            layout.canvas.y + (60.0 + y) * layout.scale,
        ]
    };
    let [left, top] = origin(64.0, 20.0);
    assert_eq!(
        tab("regiments"),
        Some([left, top, 36.0 * layout.scale, 33.0 * layout.scale])
    );
    assert_eq!(window.tabs.len(), 5);

    applied.systems.minimize_defenses_window(
        primary,
        (200, 60),
        rebellion_render::defenses_window::DefensesPage::Personnel,
    );
    let railed = next!(watch).unwrap();
    let slot = applied.systems.rail_slot_screen_rect(layout, 0).unwrap();
    assert_eq!(railed.rail, [(dat, "defenses", super::rect_array(slot))]);

    // The primary system's Missions window: the player's Diplomacy
    // mission first (selected, its agent listed), then the other
    // side's; both target the system.
    assert!(missions_windows.open(
        &applied.world,
        &fog,
        &applied.missions,
        primary,
        (150, 40),
        CockpitFaction::Alliance,
        layout
    ));
    let shown = next!(watch).unwrap();
    let [window] = &shown.missions_windows[..] else {
        panic!("{:?}", shown.missions_windows);
    };
    assert_eq!((window.system_dat_id, window.origin), (dat, (150, 40)));
    assert_eq!(
        window.rows,
        [
            ("Diplomacy".to_string(), 1, 0x4c10),
            ("Diplomacy".to_string(), 2, 0x5c10)
        ]
    );
    assert_eq!(window.side, 2);
    assert_eq!(
        (window.selected, window.tab, window.tab_side),
        (Some(0), "agents", Some(1))
    );
    assert_eq!(window.members.len(), 1);
    assert_eq!(window.member_rects.len(), 1);
    assert_eq!(
        window.target.as_deref(),
        Some(applied.world.systems[primary].name.as_str())
    );
    let [left, top] = [
        layout.canvas.x + (150.0 + 5.0) * layout.scale,
        layout.canvas.y + (40.0 + 74.0) * layout.scale,
    ];
    assert_eq!(
        window.row_rects[1],
        [left, top, 90.0 * layout.scale, 50.0 * layout.scale]
    );
    assert_eq!(window.tabs.len(), 2);

    applied.systems.minimize_missions_window(primary, (150, 40));
    let railed = next!(watch).unwrap();
    assert_eq!(railed.rail.last().map(|entry| entry.1), Some("missions"));

    let other = super::tests::request(Scenario::FleetLoad, CockpitFaction::Alliance);
    assert!(QuadrantWatch::default()
        .next(
            other,
            &applied.world,
            &QuadrantWindows {
                systems: &applied.systems,
                defenses: &defenses,
                missions_windows: &missions_windows,
                fog: &fog,
                missions: &applied.missions,
                production: rebellion_render::DefensesProduction {
                    manufacturing: &manufacturing,
                    deliveries: &deliveries,
                    regiments: &[],
                    squadrons: &[],
                },
                economy: &economy,
            },
        )
        .is_none());
}

#[test]
fn the_subdue_scenario_revolts_the_primary_and_sends_the_agent_to_subdue_it() {
    // Our fixture: a revolt the player's garrison may end, an agent on
    // Subdue Uprising (MISSNSD 0x57000080) at it, and the clock Fast.
    let mut world = diplomacy_world(Faction::Alliance);
    let here = world.systems.keys().next().unwrap();
    world.characters.insert(rebellion_core::world::Character {
        name: "Agent".into(),
        is_alliance: true,
        current_system: Some(here),
        ..Default::default()
    });
    let request = request(Scenario::SubdueSuccess, CockpitFaction::Alliance);
    let mut missions = MissionState::default();
    let mut uprisings = rebellion_core::uprising::UprisingState::new();
    apply(
        request,
        &mut world,
        &mut GameMode::MainMenu,
        &mut MissionFaction::Alliance,
        &mut CockpitState::new(CockpitFaction::Alliance),
        &mut GalaxyMapState::default(),
        &mut MovementState::default(),
        &mut ManufacturingState::default(),
        &mut EconomyState::default(),
        &mut missions,
        &mut BlockadeState::default(),
        &mut SectorWindowState::default(),
        &mut SystemWindowState::default(),
        &mut FleetWindowState::default(),
        &mut DefensesWindowState::default(),
        &mut TroopTransportState::default(),
        &mut uprisings,
    );

    assert!(runs_the_clock(request));
    assert!(!runs_the_clock(super::tests::request(
        Scenario::CharacterMove,
        CockpitFaction::Alliance
    )));
    assert!(uprisings.is_uprising(here));
    assert_eq!(
        world.systems[here].control,
        ControlKind::Uprising(Faction::Alliance)
    );
    let regiments = world.systems[here]
        .ground_units
        .iter()
        .filter(|&&key| world.troops[key].is_alliance)
        .count();
    assert!(regiments >= 8, "{regiments}");
    let kinds: Vec<_> = missions.missions().iter().map(|m| m.kind).collect();
    assert_eq!(kinds, [MissionKind::SubdueUprising]);
}

// A0 sfstatus/03: the Alliance's Infiltrators en route to Yavin, listed
// there; FUN_0042c3b0 gives class 2 the STRATEGY 11520 mark.
#[test]
fn the_special_force_enroute_scenario_sends_a_team_to_the_primary_system() {
    for (faction, class, alliance) in [
        (CockpitFaction::Alliance, 0x3c00_0002, true),
        (CockpitFaction::Empire, 0x3c00_0007, false),
    ] {
        let applied = apply_to_fleet_world(request(Scenario::SpecialForceEnroute, faction));
        let primary = applied.world.systems.keys().next().unwrap();
        let [transit] = applied.missions.en_route() else {
            panic!("{:?}", applied.missions.en_route());
        };
        let MissionMember::SpecialForce(unit) = transit.member else {
            panic!("{transit:?}");
        };
        assert_eq!(transit.to, primary);
        assert_eq!(transit.abode, None);
        let unit = &applied.world.special_forces[unit];
        assert_eq!(
            (unit.class_dat_id.raw(), unit.is_alliance),
            (class, alliance)
        );
        assert!(applied.world.systems[primary].special_forces.is_empty());
    }
    let recon = apply_to_fleet_world(request(Scenario::ReconTargeting, CockpitFaction::Alliance));
    assert!(recon.missions.en_route().is_empty());
}

// A0 2026-11-11-regiment-en-route/00: Averam lists the Alliance Army
// Regiment moving there from Yavin.
#[test]
fn the_regiment_enroute_scenario_moves_a_regiment_to_the_primary_system() {
    for (faction, class, alliance) in [
        (CockpitFaction::Alliance, 0x1000_0002, true),
        (CockpitFaction::Empire, 0x1000_0008, false),
    ] {
        // The primary's own regiment leaves its list; the other side's
        // stays.
        let mut kept = None;
        let applied =
            apply_to_loading_world_with(request(Scenario::RegimentEnroute, faction), |world| {
                let primary = world.systems.keys().next().unwrap();
                for is_alliance in [alliance, !alliance] {
                    let troop = world.troops.insert(rebellion_core::world::TroopUnit {
                        class_dat_id: rebellion_core::ids::DatId::new(class),
                        is_alliance,
                        regiment_strength: 100,
                    });
                    world.systems[primary].ground_units.push(troop);
                    kept = Some(troop);
                }
            });
        let primary = applied.world.systems.keys().next().unwrap();
        assert_eq!(applied.world.systems[primary].ground_units, [kept.unwrap()]);
        let [transit] = applied.transport.transits() else {
            panic!("{:?}", applied.transport.transits());
        };
        assert_eq!(
            transit.leg,
            rebellion_core::troop_transport::RegimentLeg::Surface(primary)
        );
        assert_ne!(transit.origin, primary);
        let troop = &applied.world.troops[transit.troop];
        assert_eq!(
            (troop.class_dat_id.raw(), troop.is_alliance),
            (class, alliance)
        );
    }
    let recon = apply_to_fleet_world(request(Scenario::ReconTargeting, CockpitFaction::Alliance));
    assert!(recon.transport.transits().is_empty());
}

// A0 2026-11-14-stale-views (07-kothlis-def-d374-montage
// 66b9306bfc519f1a, 07-kothlis-fac-d374-montage 3cd77ca5110e162a,
// 06-kothlis-fleet16-d374-montage 955559b56955015b): the player's
// frozen copy of the other side's system keeps its regiments,
// squadrons and facilities after the live system lost them, and a
// fleet made since is copied with its name only (FUN_004f9a60,
// FUN_0053f760). The fixture's start does not refresh it.
#[test]
fn the_stale_scenario_freezes_the_second_systems_copy_as_seen() {
    for (faction, alliance) in [
        (CockpitFaction::Alliance, true),
        (CockpitFaction::Empire, false),
    ] {
        let mut applied = apply_quadrants_as(Scenario::QuadrantsStale, faction);
        let primary = applied.world.systems.keys().next().unwrap();
        let second = loading_target(&applied.world, primary).unwrap();
        let viewer = Faction::of_alliance(alliance);
        // main.rs seeds from the player's fog after the fixture.
        let mut fog_alliance = FogState::new(Faction::Alliance);
        let mut fog_empire = FogState::new(Faction::Empire);
        rebellion_core::fog::FogSystem::seed(&mut fog_alliance, &applied.world);
        rebellion_core::fog::FogSystem::seed(&mut fog_empire, &applied.world);
        assert!(!applied.world.systems[second]
            .control
            .is_controlled_by(viewer));
        let movement = MovementState::new();
        rebellion_core::object_views::seed_from_fog(
            &mut applied.world,
            &applied.transport,
            &movement,
            [&fog_alliance, &fog_empire],
        );
        let world = &applied.world;
        let id = world.systems[second].dat_id;
        let views = &world.object_views;
        let copy = views.system(viewer, id).unwrap();
        assert!(!copy.live);
        assert_eq!(copy.manufacturing_facilities.len(), 1);
        assert_eq!(copy.production_facilities.len(), 10);
        assert_eq!(world.systems[second].production_facilities.len(), 1);
        let regiments =
            rebellion_render::viewed_contents::viewed_surface_regiments(world, viewer, second);
        assert_eq!(regiments.len(), 3);
        assert_eq!(world.systems[second].ground_units.len(), 1);
        let details = views.system_details(viewer, id).unwrap();
        let last = details.fleets.last().unwrap();
        assert_eq!(last.is_alliance, !alliance);
        assert!(last.ships.is_empty() && last.fighters.is_empty());
        assert!(!views.see_everything);
        let seeing = apply_quadrants_as(Scenario::QuadrantsStaleSeeEverything, faction);
        assert!(seeing.world.object_views.see_everything);
    }
}

// A0 2026-11-14-quadrants-status-known/18 (6a1269a1765f7c9b): the
// Alliance's frozen copy of an Imperial Army Regiment on Imperial
// Chandrila reads "Enroute", "ETA Destination: Day 171" on day 576
// (FUN_004f2d10 reads the side's copy; FUN_004f9a60 froze it).
#[test]
fn the_enemy_regiment_frozen_scenario_lists_the_players_frozen_copy() {
    use rebellion_render::status_rows::{status_view, StatusObject};
    use rebellion_render::viewed_contents::{viewed_surface_regiments, ViewedRegiment};
    for (faction, class, alliance) in [
        (CockpitFaction::Alliance, 0x1000_0008, true),
        (CockpitFaction::Empire, 0x1000_0002, false),
    ] {
        let applied =
            apply_to_loading_world_with(request(Scenario::EnemyRegimentFrozen, faction), |_| {});
        let world = &applied.world;
        let secondary = world.systems.keys().nth(1).unwrap();
        let viewer = rebellion_core::dat::Faction::of_alliance(alliance);
        assert_eq!(
            world.systems[secondary].control,
            ControlKind::Controlled(rebellion_core::dat::Faction::of_alliance(!alliance))
        );
        assert!(applied.defenses.is_open(secondary));
        let [ViewedRegiment::View(id)] = viewed_surface_regiments(world, viewer, secondary)[..]
        else {
            panic!("{:?}", viewed_surface_regiments(world, viewer, secondary));
        };
        let manufacturing = rebellion_core::manufacturing::ManufacturingState::default();
        let missions = MissionState::new();
        let movement = MovementState::new();
        let sources = rebellion_render::status_rows::StatusSources {
            world,
            missions: &missions,
            movement: &movement,
            transport: &applied.transport,
            manufacturing: &manufacturing,
            today: 576,
            objectives: rebellion_render::status_rows::ObjectiveSource::default(),
        };
        let view = status_view(sources, StatusObject::ViewedTroop { alliance, id }).unwrap();
        assert_eq!(view.title, "Trooper Regiment Status");
        assert_eq!(
            view.rows[1..3]
                .iter()
                .map(|row| (row.label.as_str(), row.value.as_str()))
                .collect::<Vec<_>>(),
            [("Status:", "Enroute"), ("ETA Destination:", "Day 171")]
        );
        let rebellion_core::object_views::ObjectView::Regiment(copy) =
            world.object_views.view(viewer, id).unwrap();
        assert_eq!(copy.class.raw(), class);
        assert!(!copy.live);
    }
}

#[test]
fn the_squadron_enroute_scenario_sends_the_other_sides_fleet_off_the_second_planet() {
    // FUN_005073d0 refuses a squadron a blockaded planet (0x90/4), so the
    // fixture moves the other side's fleet from the second planet to a
    // fourth system; every other fleet stays.
    let applied = apply_to_loading_world_with(
        request(Scenario::SquadronEnroute, CockpitFaction::Alliance),
        |world| {
            world
                .fighter_classes
                .insert(rebellion_core::world::FighterClass {
                    dat_id: rebellion_core::ids::DatId::new(0x1c00_0001),
                    is_alliance: true,
                    ..Default::default()
                });
            let first = world.systems.keys().next().unwrap();
            let mut fourth = world.systems[first].clone();
            fourth.name = "Fourth".into();
            fourth.fleets.clear();
            let fourth = world.systems.insert(fourth);
            let sector = world.systems[fourth].sector;
            world.sectors[sector].systems.push(fourth);
        },
    );
    let keys: Vec<_> = applied.world.systems.keys().collect();
    let (primary, secondary, fourth) = (keys[0], keys[1], keys[3]);
    let at = |system| {
        applied
            .world
            .fleets
            .values()
            .filter(|fleet| fleet.location == system)
            .map(|fleet| fleet.is_alliance)
            .collect::<Vec<_>>()
    };
    assert_eq!(at(secondary), Vec::<bool>::new());
    assert_eq!(at(fourth), [false]);
    assert_eq!(at(primary), [true, false]);
    assert_eq!(applied.world.systems[fourth].fleets.len(), 1);
    assert!(applied.world.systems[secondary].fleets.is_empty());
    assert_eq!(applied.transport.squadrons.transits().len(), 2);
}

// FUN_00556390 puts a squadron sent by Move in its destination at once.
#[test]
fn the_squadron_enroute_scenario_moves_a_squadron_to_the_primary_planet() {
    for (faction, index, alliance) in [
        (CockpitFaction::Alliance, 1, true),
        (CockpitFaction::Empire, 5, false),
    ] {
        let applied =
            apply_to_loading_world_with(request(Scenario::SquadronEnroute, faction), |world| {
                for (raw, alliance) in [(0x1c00_0001, true), (0x1c00_0005, false)] {
                    world
                        .fighter_classes
                        .insert(rebellion_core::world::FighterClass {
                            dat_id: rebellion_core::ids::DatId::new(raw),
                            is_alliance: alliance,
                            is_empire: !alliance,
                            ..Default::default()
                        });
                }
            });
        let mut keys = applied.world.systems.keys();
        let (primary, secondary) = (keys.next().unwrap(), keys.next().unwrap());
        let [first, second] = applied.transport.squadrons.transits() else {
            panic!("{:?}", applied.transport.squadrons.transits());
        };
        for (transit, to) in [(first, primary), (second, secondary)] {
            assert_eq!(
                transit.to,
                rebellion_core::squadrons::SquadronPlace::Planet(to)
            );
            assert_ne!(transit.origin, to);
            assert!(transit.arrival_tick > 0);
            let class = &applied.world.fighter_classes[transit.class];
            assert_eq!((class.dat_id.index(), class.is_alliance), (index, alliance));
        }
        let empty = &applied.world.systems[secondary];
        assert!(empty.ground_units.is_empty() && empty.fighters.is_empty());
        assert!(empty.defense_facilities.is_empty() && empty.special_forces.is_empty());
    }
}

// FUN_00556390 puts each object sent by Move into its target container,
// here the fleet, at once (regiment-unload.md).
#[test]
fn the_fleet_arrivals_scenario_sends_a_regiment_squadron_character_and_special_force_aboard() {
    for (faction, index) in [(CockpitFaction::Alliance, 1), (CockpitFaction::Empire, 5)] {
        let alliance = faction == CockpitFaction::Alliance;
        let applied =
            apply_to_loading_world_with(request(Scenario::FleetArrivals, faction), |world| {
                for (raw, alliance) in [(0x1c00_0001, true), (0x1c00_0005, false)] {
                    world
                        .fighter_classes
                        .insert(rebellion_core::world::FighterClass {
                            dat_id: rebellion_core::ids::DatId::new(raw),
                            is_alliance: alliance,
                            is_empire: !alliance,
                            hyperdrive: 1,
                            ..Default::default()
                        });
                }
                let here = world.systems.keys().next().unwrap();
                world.characters.insert(rebellion_core::world::Character {
                    name: "Rider".into(),
                    is_alliance: alliance,
                    is_empire: !alliance,
                    recruited: true,
                    current_system: Some(here),
                    ..Default::default()
                });
            });
        let fleet = applied.world.fleets.keys().next().unwrap();
        let [regiment] = applied.transport.transits() else {
            panic!("{faction:?} {:?}", applied.transport.transits());
        };
        assert_eq!(
            regiment.leg,
            rebellion_core::troop_transport::RegimentLeg::Fleet(fleet)
        );
        let [squadron] = applied.transport.squadrons.transits() else {
            panic!("{faction:?} {:?}", applied.transport.squadrons.transits());
        };
        assert_eq!(
            squadron.to,
            rebellion_core::squadrons::SquadronPlace::Fleet(fleet)
        );
        assert_eq!(
            applied.world.fighter_classes[squadron.class].dat_id.index(),
            index
        );
        let arrivals = rebellion_render::quadrant_icons::fleet_arrivals(&applied.missions);
        assert_eq!(arrivals.len(), 2, "{faction:?}");
        assert!(arrivals.iter().all(|&(_, to)| to == fleet));
        assert!(matches!(arrivals[0].0, MissionMember::Character(_)));
        let MissionMember::SpecialForce(force) = arrivals[1].0 else {
            panic!("{faction:?} {arrivals:?}");
        };
        assert_eq!(applied.world.special_forces[force].is_alliance, alliance);
        assert!(applied.transport.cargo(fleet).is_empty());
        assert!(applied.world.fleets[fleet].fighters.is_empty());
    }
}

// FUN_00465bb0's destroyed branch reads "Name - Killed" (0x8798); Steam
// guide guide-023 lists "Ackbar - Killed" in the Personnel Finder.
#[test]
fn the_personnel_states_scenario_lists_its_sixth_character_as_killed() {
    let names = ["One", "Two", "Three", "Four", "Five", "Six"];
    let applied = apply_to_loading_world_with(
        request(Scenario::PersonnelStates, CockpitFaction::Alliance),
        |world| {
            world.characters.clear();
            let here = world.systems.keys().next().unwrap();
            let mut last = None;
            for name in names {
                last = Some(world.characters.insert(rebellion_core::world::Character {
                    name: name.into(),
                    is_alliance: true,
                    recruited: true,
                    current_system: Some(here),
                    ..Default::default()
                }));
            }
            // The sixth rides a fleet, which must drop it when it dies.
            let fleet = world.fleets.keys().next().expect("a fleet");
            world.fleets[fleet].characters.push(last.unwrap());
        },
    );
    let world = &applied.world;
    let killed: Vec<_> = world
        .characters
        .values()
        .filter(|character| character.is_killed)
        .map(|character| character.name.as_str())
        .collect();
    assert_eq!(killed, ["Six"]);
    let six = world.characters.values().find(|c| c.name == "Six").unwrap();
    assert_eq!((six.current_system, six.current_fleet), (None, None));
    assert!(world.fleets.values().all(|fleet| fleet
        .characters
        .iter()
        .all(|&key| world.characters[key].name != "Six")));
    let fog = rebellion_core::fog::FogState::new(Faction::Alliance);
    let mut finder = rebellion_render::personnel_finder::PersonnelFinderState::default();
    finder.open(CockpitFaction::Alliance);
    let texts = finder.shown_character_rows(world, &fog).unwrap();
    assert!(texts.iter().any(|text| text == "Six - Killed"), "{texts:?}");
}

// A0 2026-11-14-stale-views second run: the Alliance's Imperial tab
// reads "Darth Vader - Imperial Star Destroyer 3" while its copy of
// Yaga Minor is frozen (1x-personnelfinder-empire-d85, d57ed845), and
// "Darth Vader - Location Unknown" once the copy went live without him
// (d91, 1b8818d8); FUN_00465bb0 keeps 0x1895 for a copy in no ship or
// system.
#[test]
fn the_personnel_copied_scenario_lists_a_copied_ship_a_lost_copy_and_copied_states() {
    let applied = apply_to_loading_world_with(
        request(Scenario::PersonnelCopied, CockpitFaction::Alliance),
        |world| {
            let here = world.systems.keys().next().unwrap();
            for (id, name) in [
                (0x3000_0101, "Darth Vader"),
                (0x3000_0102, "Piett"),
                (0x3000_0103, "Thrawn"),
                (0x3000_0104, "Veers"),
            ] {
                world.characters.insert(rebellion_core::world::Character {
                    dat_id: rebellion_core::ids::DatId::new(id),
                    name: name.into(),
                    is_empire: true,
                    recruited: true,
                    current_system: Some(here),
                    ..Default::default()
                });
            }
            for class in world.capital_ship_classes.values_mut() {
                if !class.is_alliance {
                    class.name = "Imperial Star Destroyer".into();
                }
            }
        },
    );
    let world = &applied.world;
    let fog = rebellion_core::fog::FogState::new(Faction::Alliance);
    let rows = rebellion_render::personnel_finder::character_rows(
        world,
        &fog,
        Faction::Alliance,
        rebellion_render::personnel_finder::PersonnelTab::Imperial,
    );
    let texts: Vec<_> = rows
        .iter()
        .map(|row| match row {
            rebellion_render::personnel_finder::PersonnelFinderRow::Character { name, .. } => {
                name.as_str()
            }
            rebellion_render::personnel_finder::PersonnelFinderRow::SpecForces { .. } => "",
        })
        .collect();
    assert_eq!(
        texts,
        [
            "Darth Vader - Imperial Star Destroyer 1 ( Admiral ) ",
            "Piett - Location Unknown",
            "Thrawn - System ( Enroute ) ",
            "Veers - System ( On Mission ) ",
        ]
    );
}

// FUN_00465bb0: destruction reason 0x14..0x16 reads "Retired" (0x8799,
// a Retire order, FUN_00534c00); a character in an abode (container
// family 0x98..0x9f) names the abode alone. The Empire's galaxy
// recruits few characters, so the scenario recruits up to eight.
#[test]
fn the_personnel_states_scenario_retires_its_seventh_and_rests_its_eighth_in_an_abode() {
    let names = [
        "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight",
    ];
    let applied = apply_to_loading_world_with(
        request(Scenario::PersonnelStates, CockpitFaction::Alliance),
        |world| {
            world.characters.clear();
            let here = world.systems.keys().next().unwrap();
            for (index, name) in names.into_iter().enumerate() {
                world.characters.insert(rebellion_core::world::Character {
                    name: name.into(),
                    is_alliance: true,
                    // The last ones are not recruited yet.
                    recruited: index < 5,
                    current_system: Some(here),
                    ..Default::default()
                });
            }
            world.abodes.insert(rebellion_core::world::Abode {
                dat_id: rebellion_core::ids::DatId::new(0x9800_0001),
                name: "Dagobah".into(),
                side: Faction::Neutral,
                special_forces: Vec::new(),
            });
        },
    );
    let world = &applied.world;
    assert!(world.characters.values().all(|c| c.recruited));
    let fog = rebellion_core::fog::FogState::new(Faction::Alliance);
    let mut finder = rebellion_render::personnel_finder::PersonnelFinderState::default();
    finder.open(CockpitFaction::Alliance);
    let texts = finder.shown_character_rows(world, &fog).unwrap();
    assert!(
        texts.iter().any(|text| text == "Seven - Retired"),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|text| text == "Eight - Dagobah"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|text| text == "Six - Killed"), "{texts:?}");
}

/// A galaxy holding d-01's and b-01's characters, systems and the
/// Dagobah abode, plus one more Alliance character the scenario does
/// not recruit.
fn seed_abode_roster(world: &mut GameWorld) {
    world.characters.clear();
    let first = world.systems.keys().next().unwrap();
    while world.systems.len() < 5 {
        let mut copy = world.systems[first].clone();
        copy.dat_id = rebellion_core::ids::DatId::new(0x9000_0400 + world.systems.len() as u32);
        copy.fleets.clear();
        world.systems.insert(copy);
    }
    let systems: Vec<_> = world.systems.keys().collect();
    let names = ["Yavin", "Sluis Van", "Corsin", "Chazwa", "Fwillsving"];
    for (key, name) in systems.iter().zip(names) {
        world.systems[*key].name = name.into();
    }
    for (index, name) in [
        "Chewbacca",
        "Han Solo",
        "Huoba Neva",
        "Jan Dodonna",
        "Leia Organa",
        "Luke Skywalker",
        "Mon Mothma",
        "Wedge Antilles",
        "Garm Bel Iblis",
        "Syub Snunb",
        "Talon Karrde",
    ]
    .into_iter()
    .enumerate()
    {
        world.characters.insert(rebellion_core::world::Character {
            dat_id: rebellion_core::ids::DatId::new(0x3000_0200 + index as u32),
            name: name.into(),
            is_alliance: true,
            recruited: true,
            current_system: Some(systems[0]),
            ..Default::default()
        });
    }
    world.abodes.insert(rebellion_core::world::Abode {
        dat_id: rebellion_core::ids::DatId::new(0x441),
        name: "Dagobah".into(),
        side: Faction::Neutral,
        special_forces: Vec::new(),
    });
    // Luke rides a fleet beside an Imperial captive; the fleet drops
    // Luke for the abode and keeps the other.
    let vader = world.characters.insert(rebellion_core::world::Character {
        dat_id: rebellion_core::ids::DatId::new(0x3800_0001),
        name: "Darth Vader".into(),
        is_empire: true,
        recruited: true,
        ..Default::default()
    });
    let luke = world
        .characters
        .iter()
        .find(|(_, character)| character.name == "Luke Skywalker")
        .map(|(key, _)| key)
        .unwrap();
    let fleet = world.fleets.keys().next().expect("a fleet");
    world.fleets[fleet].characters.extend([vader, luke]);
    world.characters[luke].current_fleet = Some(fleet);
}

// A0 2026-11-17-edited-saves-6 d-01 (a50b31353dcd): the Alliance's own
// rows, Luke in the Dagobah abode (0x98000441) named alone
// (FUN_00465bb0, container family 0x98..0x9f) and Han held at Sluis Van
// (0x8746); b-01 (c8e08a5d4c6d): the Empire's copies, Luke's in no place
// (0x1895 alone), Garm's a General at Fwillsving.
#[test]
fn the_personnel_abode_scenario_lists_the_a0_rows_for_each_side() {
    let alliance = apply_to_loading_world_with(
        request(Scenario::PersonnelAbode, CockpitFaction::Alliance),
        seed_abode_roster,
    );
    let fog = rebellion_core::fog::FogState::new(Faction::Alliance);
    let mut finder = rebellion_render::personnel_finder::PersonnelFinderState::default();
    finder.open(CockpitFaction::Alliance);
    assert_eq!(
        finder.shown_character_rows(&alliance.world, &fog).unwrap(),
        [
            "Chewbacca - Yavin",
            "Han Solo - Sluis Van ( Captured ) ",
            "Huoba Neva - Corsin",
            "Jan Dodonna - Yavin",
            "Leia Organa - Yavin",
            "Luke Skywalker - Dagobah",
            "Mon Mothma - Chazwa",
            "Wedge Antilles - Yavin",
        ]
    );
    let seated: Vec<_> = alliance
        .world
        .fleets
        .values()
        .flat_map(|fleet| fleet.characters.iter())
        .map(|&key| alliance.world.characters[key].name.as_str())
        .collect();
    assert_eq!(seated, ["Darth Vader"]);

    let empire = apply_to_loading_world_with(
        request(Scenario::PersonnelAbode, CockpitFaction::Empire),
        seed_abode_roster,
    );
    let fog = rebellion_core::fog::FogState::new(Faction::Empire);
    let rows = rebellion_render::personnel_finder::character_rows(
        &empire.world,
        &fog,
        Faction::Empire,
        rebellion_render::personnel_finder::PersonnelTab::Alliance,
    );
    let texts: Vec<_> = rows
        .iter()
        .map(|row| match row {
            rebellion_render::personnel_finder::PersonnelFinderRow::Character { name, .. } => {
                name.as_str()
            }
            rebellion_render::personnel_finder::PersonnelFinderRow::SpecForces { .. } => "",
        })
        .collect();
    assert_eq!(
        texts,
        [
            "Chewbacca - Location Unknown",
            "Garm Bel Iblis - Fwillsving ( General ) ",
            "Han Solo - Fwillsving",
            "Jan Dodonna - Location Unknown",
            "Leia Organa - Location Unknown",
            "Luke Skywalker - Location Unknown",
            "Syub Snunb - Location Unknown",
            "Wedge Antilles - Location Unknown",
        ]
    );
}

/// c-14's characters, Coruscant, Yavin and a Victory Destroyer class,
/// plus Alliance characters the scenario does not recruit.
fn seed_pickup_roster(world: &mut GameWorld) {
    seed_abode_roster(world);
    let systems: Vec<_> = world.systems.keys().collect();
    world.systems[systems[1]].name = "Coruscant".into();
    world.systems[systems[1]].control =
        rebellion_core::world::ControlKind::Controlled(Faction::Empire);
    world
        .capital_ship_classes
        .insert(rebellion_core::world::CapitalShipClass {
            name: "Victory Destroyer".into(),
            hull: 30,
            ..Default::default()
        });
    for (index, name) in ["Ma'w'shiye", "Tura Raftican"].into_iter().enumerate() {
        world.characters.insert(rebellion_core::world::Character {
            dat_id: rebellion_core::ids::DatId::new(0x3000_0280 + index as u32),
            name: name.into(),
            is_alliance: true,
            recruited: true,
            current_system: Some(systems[2]),
            ..Default::default()
        });
    }
}

// A0 2026-11-18-edited-saves-9 c-14 (88e1928e1f1a, day 531): the
// Empire's Alliance tab right after Jabba Delivers Prisoners: the three
// prisoners aboard Vader's ship read live with the Captured word
// (FUN_0056da80, FUN_00465bb0), Chewbacca (destroyed, FUN_005445d0)
// and four others from the Empire's frozen copies.
#[test]
fn the_personnel_pickup_scenario_lists_c14s_alliance_rows() {
    let applied = apply_to_loading_world_with(
        request(Scenario::PersonnelPickup, CockpitFaction::Empire),
        seed_pickup_roster,
    );
    let fog = rebellion_core::fog::FogState::new(Faction::Empire);
    let rows = rebellion_render::personnel_finder::character_rows(
        &applied.world,
        &fog,
        Faction::Empire,
        rebellion_render::personnel_finder::PersonnelTab::Alliance,
    );
    let texts: Vec<_> = rows
        .iter()
        .map(|row| match row {
            rebellion_render::personnel_finder::PersonnelFinderRow::Character { name, .. } => {
                name.as_str()
            }
            rebellion_render::personnel_finder::PersonnelFinderRow::SpecForces { .. } => "",
        })
        .collect();
    assert_eq!(
        texts,
        [
            "Chewbacca - Yavin",
            "Han Solo - Victory Destroyer 1 ( Captured ) ",
            "Jan Dodonna - Location Unknown",
            "Leia Organa - Victory Destroyer 1 ( Captured ) ",
            "Luke Skywalker - Victory Destroyer 1 ( Captured ) ",
            "Ma'w'shiye - Yavin ( Enroute ) ",
            "Tura Raftican - Yavin",
            "Wedge Antilles - Yavin",
        ]
    );
    let chewbacca = applied
        .world
        .characters
        .values()
        .find(|character| character.name == "Chewbacca")
        .unwrap();
    assert!(chewbacca.is_killed);
    // Each fleet's seated characters and its first ship's class: Vader
    // stays the Empire's in the fleet he rode; the prisoners ride the
    // new fleet's Victory Destroyer.
    let mut seated: Vec<(Vec<(&str, bool)>, &str)> = applied
        .world
        .fleets
        .values()
        .filter(|fleet| !fleet.characters.is_empty())
        .map(|fleet| {
            let names = fleet
                .characters
                .iter()
                .map(|&key| &applied.world.characters[key])
                .map(|character| (character.name.as_str(), character.is_alliance))
                .collect();
            let class = fleet.capital_ships.first().map_or("", |ship| {
                applied.world.capital_ship_classes[ship.class].name.as_str()
            });
            (names, class)
        })
        .collect();
    seated.sort();
    assert_eq!(
        seated,
        [
            (vec![("Darth Vader", false)], ""),
            (
                vec![
                    ("Han Solo", true),
                    ("Leia Organa", true),
                    ("Luke Skywalker", true)
                ],
                "Victory Destroyer"
            ),
        ]
    );
}

/// b-19's characters, systems and Jabba's Palace, plus one more
/// Alliance character the scenario does not recruit.
fn seed_palace_roster(world: &mut GameWorld) {
    seed_abode_roster(world);
    let systems: Vec<_> = world.systems.keys().collect();
    world.systems[systems[1]].name = "Coruscant".into();
    world.systems[systems[1]].control =
        rebellion_core::world::ControlKind::Controlled(Faction::Empire);
    world.abodes.insert(rebellion_core::world::Abode {
        dat_id: rebellion_core::ids::DatId::new(0x443),
        name: "Jabba's Palace".into(),
        side: Faction::Empire,
        special_forces: Vec::new(),
    });
}

fn seed_retired_roster(world: &mut GameWorld) {
    seed_abode_roster(world);
    let systems: Vec<_> = world.systems.keys().collect();
    world.systems[systems[1]].name = "Coruscant".into();
    world.systems[systems[2]].name = "Corfai".into();
    for (index, name) in [
        "Emperor Palpatine",
        "Jerjerrod",
        "Menndo",
        "Needa",
        "Ozzel",
        "Piett",
        "Veers",
        "Mara Jade",
    ]
    .into_iter()
    .enumerate()
    {
        world.characters.insert(rebellion_core::world::Character {
            dat_id: rebellion_core::ids::DatId::new(0x3000_0300 + index as u32),
            name: name.into(),
            is_alliance: false,
            is_empire: true,
            recruited: true,
            current_system: Some(systems[4]),
            ..Default::default()
        });
    }
}

// A0 2026-11-18-edited-saves-9 d-03 (b9ea91b80dbb, an unedited Empire
// game, day 132): the Imperial tab lists "Ozzel - Retired" (0x8799,
// FUN_00465bb0) among d-03's eight; the three aboard ships in A0
// stand at Coruscant here.
#[test]
fn the_personnel_retired_scenario_lists_d03s_imperial_rows() {
    let applied = apply_to_loading_world_with(
        request(Scenario::PersonnelRetired, CockpitFaction::Empire),
        seed_retired_roster,
    );
    let fog = rebellion_core::fog::FogState::new(Faction::Empire);
    let mut finder = rebellion_render::personnel_finder::PersonnelFinderState::default();
    finder.open(CockpitFaction::Empire);
    assert_eq!(
        finder.shown_character_rows(&applied.world, &fog).unwrap(),
        [
            "Darth Vader - Yavin ( On Mission ) ",
            "Emperor Palpatine - Coruscant",
            "Jerjerrod - Coruscant",
            "Menndo - Corfai",
            "Needa - Coruscant",
            "Ozzel - Retired",
            "Piett - Coruscant",
            "Veers - Coruscant",
        ]
    );
}

// A0 2026-11-18-edited-saves-7 b-19 (e4f53d49b065, day 31): the
// Alliance's copies of its four prisoners stay in Jabba's Palace
// (FUN_004eec60, FUN_004f9a60) and read the abode's name alone
// (FUN_00465bb0), though the prisoners stand at Coruscant.
#[test]
fn the_personnel_palace_scenario_lists_b19s_rows() {
    let applied = apply_to_loading_world_with(
        request(Scenario::PersonnelPalace, CockpitFaction::Alliance),
        seed_palace_roster,
    );
    let fog = rebellion_core::fog::FogState::new(Faction::Alliance);
    let mut finder = rebellion_render::personnel_finder::PersonnelFinderState::default();
    finder.open(CockpitFaction::Alliance);
    assert_eq!(
        finder.shown_character_rows(&applied.world, &fog).unwrap(),
        [
            "Chewbacca - Jabba's Palace",
            "Han Solo - Jabba's Palace",
            "Huoba Neva - Corsin",
            "Jan Dodonna - Yavin",
            "Leia Organa - Jabba's Palace",
            "Luke Skywalker - Jabba's Palace",
            "Mon Mothma - Chazwa",
            "Wedge Antilles - Yavin",
        ]
    );
    let luke = applied
        .world
        .characters
        .values()
        .find(|character| character.name == "Luke Skywalker")
        .unwrap();
    assert!(luke.is_captive && luke.current_abode.is_none());
    assert_eq!(
        luke.current_system
            .map(|system| applied.world.systems[system].name.as_str()),
        Some("Coruscant")
    );
    // Only the listed characters are touched: Vader stays the
    // Empire's, seated in the fleet the prisoners left.
    let seated: Vec<_> = applied
        .world
        .fleets
        .values()
        .flat_map(|fleet| fleet.characters.iter())
        .map(|&key| &applied.world.characters[key])
        .map(|character| (character.name.as_str(), character.is_alliance))
        .collect();
    assert_eq!(seated, [("Darth Vader", false)]);
}

#[test]
fn the_targeting_fleet_move_and_fleet_window_scenarios_report_the_object_menu() {
    let reporting: Vec<_> = (0..SCENARIO_COUNT)
        .filter_map(Scenario::decode)
        .filter(|scenario| scenario.reports_object_menu())
        .collect();

    assert_eq!(
        reporting,
        [
            Scenario::MissionTargeting,
            Scenario::FleetMove,
            Scenario::FleetMoveBlockade,
            Scenario::FleetLoad,
            Scenario::FleetLoadFull,
            Scenario::Quadrants,
            Scenario::RegimentUnloadRefused,
            Scenario::FleetJoin,
            Scenario::ProductionDestination,
            Scenario::BuildSelection,
            Scenario::RescueTargeting,
            Scenario::SabotageTargeting,
            Scenario::DeathStarTargeting,
            Scenario::ShipTargeting,
            Scenario::FleetTargeting,
            Scenario::ReconTargeting,
            Scenario::ResearchTargeting,
            Scenario::SquadronLoad,
            Scenario::SquadronBattle,
            Scenario::FleetBombardment,
            Scenario::FleetAssault,
            Scenario::CharacterCommand,
            Scenario::CharacterMove,
            Scenario::CharacterBoard,
            Scenario::CharacterAboard,
            Scenario::FleetDestroySystem,
            Scenario::ProductionDelivery,
            Scenario::UprisingIncident,
            Scenario::BlockadeArrival,
            Scenario::SupportFlip,
            Scenario::FleetWindowA0,
            Scenario::FleetWindowYavin,
            Scenario::BuildSelectionOverMaintenance,
            Scenario::BuildSelectionDestination,
            Scenario::EnemyCharacterTargeting,
            Scenario::UprisingTargeting,
            Scenario::BatteryStatus,
            Scenario::PersonnelStates,
            Scenario::ForceStatus,
            Scenario::BuildSelectionDefensesDestination,
            Scenario::BuildSelectionFleetDestination,
            Scenario::FleetAssaultTaken,
            Scenario::BuildSelectionTraining,
            Scenario::TroopResearchTargeting,
            Scenario::SpecialForceEnroute,
            Scenario::RegimentEnroute,
            Scenario::SquadronEnroute,
            Scenario::FleetArrivals,
            Scenario::FleetJoinEnroute,
            Scenario::ResearchDiplomacyTargeting,
            Scenario::QuadrantsNeutralFleet,
            Scenario::QuadrantsFreeTeam,
            Scenario::QuadrantsFacility,
            Scenario::BuildSelectionFacilityRoom,
            Scenario::FacilityResearchTargeting,
            Scenario::EnemyRegimentFrozen,
            Scenario::QuadrantsStale,
            Scenario::QuadrantsStaleSeeEverything,
            Scenario::EnemyRegimentAtRest,
            Scenario::FleetLoadUnfinished,
            Scenario::ShipShieldHit,
            Scenario::SpecialForceAboard,
            Scenario::ShipRepaired,
            Scenario::GravityWellBattle,
            Scenario::PersonnelAbode,
            Scenario::PersonnelRetired,
            Scenario::QuadrantsRegimentQueued,
            Scenario::QuadrantsRegimentEnRoute,
            Scenario::BlockadeRunning,
            Scenario::CharacterEvacuation,
            Scenario::SquadronBlockaded,
            Scenario::EndingAssaultCoruscant,
            Scenario::CaptiveTransport,
        ]
    );
}

fn finder_observation(
    applied: &Applied,
    finder: &FleetFinderState,
    faction: CockpitFaction,
) -> FleetFinderObservation {
    let player = if faction == CockpitFaction::Alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    fleet_finder_observation(
        request(Scenario::FleetFinder, faction),
        &applied.world,
        &FinderWindows {
            fog: &FogState::new(player),
            movement: &applied.movement,
            finder,
            fleets: &FleetWindowState::default(),
            sectors: &applied.sectors,
            map: &applied.map,
            left_panel_open: false,
        },
    )
    .unwrap()
}

// port: the fixture's own layout; the rows follow FUN_00462be0.
#[test]
fn the_fleet_finder_scenario_lists_a_second_own_fleet_and_the_other_sides_fleet_at_home() {
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let applied = apply_to_fleet_world(request(Scenario::FleetFinder, faction));
        let world = &applied.world;
        let is_alliance = faction == CockpitFaction::Alliance;
        let keys: Vec<_> = world.systems.keys().take(10).collect();
        let (home, third) = (
            world.systems[keys[0]].dat_id.raw(),
            world.systems[keys[2]].dat_id.raw(),
        );
        assert_eq!(world.fleets.len(), 5);
        let mut finder = FleetFinderState::default();
        assert!(finder_observation(&applied, &finder, faction)
            .rows
            .is_empty());
        finder.open(faction);

        let observed = finder_observation(&applied, &finder, faction);

        assert!(observed.open);
        assert_eq!(observed.mode, Some("fleets"));
        assert_eq!(observed.tab, Some("all"));
        let row = |side: bool, system: u32| {
            observed
                .rows
                .iter()
                .any(|row| row.is_alliance == side && row.system_dat_id == Some(system))
        };
        assert!(row(!is_alliance, home), "{faction:?}: {:?}", observed.rows);
        assert!(row(is_alliance, third), "{faction:?}: {:?}", observed.rows);
        let controls = observed.controls.unwrap();
        assert_eq!(controls.rows.len(), observed.rows.len());
        assert!(controls.rows.iter().all(Option::is_some));
        assert_eq!(observed.player_is_alliance, is_alliance);
        // FUN_00427270: the control's rectangle, (157, 407) 27 by 15 for
        // the Alliance and (199, 434) 37 by 24 for the Empire.
        let button = if is_alliance {
            (170.5, 414.5)
        } else {
            (217.5, 446.0)
        };
        assert_eq!(observed.cockpit_button, button);
        // The first fleet stays first; the other side's copy is the last.
        let fleet = |ordinal| observed.rows.iter().find(|row| row.fleet == Some(ordinal));
        let first = fleet(0).unwrap();
        assert_eq!(
            (first.kind, first.ship, first.is_alliance),
            ("fleet", None, is_alliance)
        );
        assert_eq!(fleet(4).map(|row| row.is_alliance), Some(!is_alliance));
    }
}

#[test]
fn the_finder_observation_names_ships_and_the_fleet_windows_selection() {
    let faction = CockpitFaction::Alliance;
    let mut applied = apply_to_fleet_world(request(Scenario::FleetFinder, faction));
    let class = applied.world.capital_ship_classes.keys().next().unwrap();
    let first = applied.world.fleets.keys().next().unwrap();
    let home = applied.world.fleets[first].location;
    applied.world.fleets[first].capital_ships = vec![
        ShipInstance::new(class, 100, true),
        ShipInstance::new(class, 100, true),
    ];
    let mut finder = FleetFinderState::default();
    finder.open(faction);
    let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
    let mut fleets = FleetWindowState::default();
    assert!(fleets.open(&applied.world, home, (100, 100), faction, layout));
    assert!(fleets.select(
        home,
        FleetWindowEntry::Ship {
            fleet: first,
            index: 1
        }
    ));
    let observe = |finder: &FleetFinderState| {
        fleet_finder_observation(
            request(Scenario::FleetFinder, faction),
            &applied.world,
            &FinderWindows {
                fog: &FogState::new(Faction::Alliance),
                movement: &applied.movement,
                finder,
                fleets: &fleets,
                sectors: &applied.sectors,
                map: &applied.map,
                left_panel_open: true,
            },
        )
        .unwrap()
    };
    let observed = observe(&finder);

    let window = observed.fleet_windows.first().unwrap();
    assert_eq!(observed.fleet_windows.len(), 1);
    assert_eq!(
        window.system_dat_id,
        applied.world.systems[home].dat_id.raw()
    );
    assert_eq!(
        (window.selected, window.fleet, window.ship),
        (Some("ship"), Some(0), Some(1))
    );
    // FUN_004a2630: the Fleet window is 235 by 304, where it opened.
    let rect = fleets.screen_rect(layout, home).unwrap();
    assert_eq!(window.rect, Some((rect.min.x, rect.min.y, 235.0, 304.0)));
    assert!(observed.left_panel_open);

    finder.set_mode(FinderMode::Ships);
    let observed = observe(&finder);
    let row = observed
        .rows
        .iter()
        .find(|row| row.fleet == Some(0) && row.ship == Some(1));
    assert_eq!(row.map(|row| row.kind), Some("ship"));
}

#[test]
fn the_finder_watch_reports_only_a_changed_finder() {
    let faction = CockpitFaction::Empire;
    let applied = apply_to_fleet_world(request(Scenario::FleetFinder, faction));
    let fog = FogState::new(Faction::Empire);
    let fleets = FleetWindowState::default();
    let mut finder = FleetFinderState::default();
    let mut watch = FleetFinderWatch::default();
    let next = |watch: &mut FleetFinderWatch, scenario, finder: &FleetFinderState| {
        watch.next(
            request(scenario, faction),
            &applied.world,
            &FinderWindows {
                fog: &fog,
                movement: &applied.movement,
                finder,
                fleets: &fleets,
                sectors: &applied.sectors,
                map: &applied.map,
                left_panel_open: false,
            },
        )
    };

    let finder_scenario = Scenario::FleetFinder;
    assert!(next(&mut watch, finder_scenario, &finder).is_some_and(|observed| !observed.open));
    assert!(next(&mut watch, finder_scenario, &finder).is_none());
    finder.open(faction);
    assert!(next(&mut watch, finder_scenario, &finder).is_some_and(|observed| observed.open));
    assert!(next(&mut watch, finder_scenario, &finder).is_none());
    assert!(next(&mut watch, Scenario::FleetJoin, &finder).is_none());
}

#[test]
fn the_watch_passes_each_new_observation_once() {
    let mut watch = FleetMoveWatch::default();
    let first = FleetMoveObservation {
        status: "fleet-move",
        code: 0,
        in_transit: false,
        destination_dat_id: None,
        confirmation_open: false,
        selected_system_dat_id: None,
        last_message: None,
        destination_entry: None,
        destination_item: None,
    };
    let second = FleetMoveObservation {
        in_transit: true,
        ..first.clone()
    };

    assert_eq!(watch.changed(Some(first.clone())), Some(&first));
    assert_eq!(watch.changed(Some(first.clone())), None);
    assert_eq!(watch.changed(None), None);
    assert_eq!(watch.changed(Some(second.clone())), Some(&second));
}

#[test]
fn a_system_point_matches_the_original_imperial_frame() {
    // FUN_00422ce0 neither zooms nor pans: the map keeps one framing.
    // Squakenet frame 0712 (Imperial cockpit) shows system 0x121,
    // DAT (747, 490), at canvas (535.5, 244.5) within 2 px.
    let world = diplomacy_world(Faction::Alliance);
    let mut system = world.systems.values().next().unwrap().clone();
    system.x = 747;
    system.y = 490;

    let (x, y) = screen_point(&system, CockpitFaction::Empire);
    assert!((x - 535.5).abs() < 2.0, "x = {x}");
    assert!((y - 244.5).abs() < 2.0, "y = {y}");
}

#[test]
fn fixture_fingerprint_is_stable() {
    assert_eq!(
        fnv1a64(b"gid/alliance/popular-support"),
        0x432f_ad5e_fe03_453d
    );
}
