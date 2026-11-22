use super::*;

#[test]
fn each_tab_has_its_tooltip_string() {
    // FUN_004637a0: 0x1893, 0x1894 side tabs; 0x1898, 0x1899 view tabs.
    assert_eq!(
        PersonnelTab::ALL.map(PersonnelTab::tooltip_string),
        [0x1893, 0x1894]
    );
    assert_eq!(
        PersonnelView::ALL.map(PersonnelView::tooltip_string),
        [0x1898, 0x1899]
    );
}
use crate::cockpit::CockpitViewport;
use rebellion_core::dat::{ExplorationStatus, SectorGroup};
use rebellion_core::ids::DatId;
use rebellion_core::world::{
    Character, ControlKind, Sector, SkillPair, SpecialForceUnit, System,
};

fn layout() -> CockpitLayout {
    CockpitLayout {
        canvas: CockpitViewport {
            x: 0.0,
            y: 0.0,
            width: 640.0,
            height: 480.0,
        },
        galaxy: CockpitViewport {
            x: 65.0,
            y: 60.0,
            width: 485.0,
            height: 350.0,
        },
        scale: 1.0,
    }
}

fn add_system(world: &mut GameWorld, name: &str) -> SystemKey {
    let sector = world.sectors.insert(Sector {
        dat_id: DatId::new(36),
        name: "Sesswenna".into(),
        group: SectorGroup::Core,
        x: 317,
        y: 248,
        systems: Vec::new(),
    });
    world.systems.insert(System {
        dat_id: DatId::new(100),
        name: name.into(),
        sector,
        x: 320,
        y: 250,
        exploration_status: ExplorationStatus::Explored,
        popularity_alliance: 0.5,
        popularity_empire: 0.5,
        is_populated: true,
        control: ControlKind::Controlled(Faction::Alliance),
        ..Default::default()
    })
}

fn default_character() -> Character {
    Character {
        dat_id: DatId::new(0x3000_0001),
        name: String::new(),
        is_alliance: true,
        is_empire: false,
        is_major: true,
        diplomacy: SkillPair {
            base: 50,
            variance: 0,
        },
        espionage: SkillPair {
            base: 50,
            variance: 0,
        },
        ship_design: SkillPair {
            base: 50,
            variance: 0,
        },
        troop_training: SkillPair {
            base: 50,
            variance: 0,
        },
        facility_design: SkillPair {
            base: 50,
            variance: 0,
        },
        combat: SkillPair {
            base: 50,
            variance: 0,
        },
        leadership: SkillPair {
            base: 50,
            variance: 0,
        },
        loyalty: SkillPair {
            base: 50,
            variance: 0,
        },
        jedi_probability: 0,
        jedi_level: SkillPair {
            base: 0,
            variance: 0,
        },
        can_be_admiral: false,
        can_be_commander: false,
        can_be_general: false,
        is_unable_to_betray: false,
        is_jedi_trainer: false,
        is_known_jedi: false,
        hyperdrive_modifier: 0,
        enhanced_loyalty: 0,
        on_mission: false,
        on_hidden_mission: false,
        on_mandatory_mission: false,
        captured_by: None,
        capture_tick: None,
        is_captive: false,
        current_system: None,
        current_fleet: None,
        heritage_known: false,
        is_killed: false,
        is_retired: false,
        recruited: true,
        injury: 0,
        force_potential: false,
        force_aware: false,
        force_base: 0,
        force_discoverer: None,
        force_training: 0,
        current_abode: None,
        dagobah_completed: false,
        encounter: None,
        encounter_day: 0,
        escape_barred: false,
        adrift: false,
        bounty_captured: false,
        command: rebellion_core::character_command::CommandKind::None,
        command_names: Default::default(),
        ship: None,
    }
}

fn add_sf(
    world: &mut GameWorld,
    system: SystemKey,
    class_raw: u32,
    is_alliance: bool,
) -> SpecialForceKey {
    let key = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(class_raw),
        is_alliance,
        skills: [0; 8],
        on_mission: false,
    });
    world.systems[system].special_forces.push(key);
    key
}

#[test]
fn the_specforces_view_lists_systems_holding_the_tabs_sf_units() {
    // FUN_00464e10: walk systems, check for SF of the tab's side.
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");
    add_sf(&mut world, yavin, 0x3c00_0004, true);
    add_sf(&mut world, yavin, 0x3c00_0004, true);
    add_sf(&mut world, yavin, 0x3c00_0002, true);
    let fog = FogState::new(Faction::Alliance);

    let listed = specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);

    assert_eq!(listed.len(), 1);
    if let PersonnelFinderRow::SpecForces { counts, .. } = &listed[0] {
        assert_eq!(counts[0].count, 2, "first SF class column");
        assert_eq!(counts[1].count, 1, "second SF class column");
        assert_eq!(counts[2].count, 0);
        assert_eq!(counts[3].count, 0);
    } else {
        panic!("expected SpecForces row");
    }
}

#[test]
fn the_other_sides_specforces_list_only_where_the_player_sees_them() {
    // FUN_004f3d50: the list holds the player's side's views. A0
    // 2026-11-13-gid/E2: an Alliance game's Imperial SpecForces tab is
    // empty though the Empire holds special forces.
    let mut world = GameWorld::default();
    let coruscant = add_system(&mut world, "Coruscant");
    world.systems[coruscant].control =
        rebellion_core::world::ControlKind::Controlled(Faction::Empire);
    add_sf(&mut world, coruscant, 0x3c00_0007, false);
    let fog = FogState::new(Faction::Alliance);
    assert!(
        specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Imperial).is_empty()
    );
    // The Empire's own tab lists them; so does the Alliance's view of
    // a system it holds.
    assert_eq!(
        specforces_rows(&world, &fog, Faction::Empire, PersonnelTab::Imperial).len(),
        1
    );
    world.systems[coruscant].control =
        rebellion_core::world::ControlKind::Controlled(Faction::Alliance);
    assert_eq!(
        specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Imperial).len(),
        1
    );
}

#[test]
fn a_frozen_copy_lists_the_other_sides_units_where_it_saw_them() {
    // FUN_004f3d50: the list holds the player's side's views; the
    // Alliance's copy of Imperial Coruscant (FUN_004f9a60, frozen) keeps
    // its special force and its character there after both have gone.
    let mut world = GameWorld::default();
    let coruscant = add_system(&mut world, "Coruscant");
    world.systems[coruscant].control =
        rebellion_core::world::ControlKind::Controlled(Faction::Empire);
    let elsewhere = add_system(&mut world, "Kuat");
    world.systems[elsewhere].dat_id = DatId::new(101);
    world.systems[elsewhere].control =
        rebellion_core::world::ControlKind::Controlled(Faction::Empire);
    add_sf(&mut world, coruscant, 0x3c00_0007, false);
    // The Alliance's own unit there is not on the Imperial tab.
    add_sf(&mut world, coruscant, 0x3c00_0002, true);
    let vader = world.characters.insert(Character {
        name: "Vader".into(),
        is_alliance: false,
        is_empire: true,
        current_system: Some(coruscant),
        recruited: true,
        ..default_character()
    });
    let copy = rebellion_core::object_views::SystemView::of(
        &world,
        &world.systems[coruscant],
        false,
    );
    let id = world.systems[coruscant].dat_id;
    world.object_views.alliance_systems.insert(id, copy);
    world.systems[coruscant].special_forces.clear();
    world.characters[vader].current_system = Some(elsewhere);
    let fog = FogState::new(Faction::Alliance);

    let forces = specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Imperial);
    let [PersonnelFinderRow::SpecForces { name, system, counts, .. }] = &forces[..] else {
        panic!("{forces:?}");
    };
    assert_eq!((name.as_str(), *system), ("Coruscant", coruscant));
    assert_eq!(counts.iter().map(|count| count.count).sum::<u32>(), 1);
    let column = sf_classes(PersonnelTab::Imperial)
        .iter()
        .position(|raw| *raw == 0x3c00_0007)
        .unwrap();
    assert_eq!(counts[column].count, 1);
    let characters = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Imperial);
    let [PersonnelFinderRow::Character { key, location, system, .. }] = &characters[..] else {
        panic!("{characters:?}");
    };
    assert_eq!((*key, location.as_str(), *system), (vader, "Coruscant", Some(coruscant)));
    // Without the copy: neither.
    world.object_views.alliance_systems.clear();
    assert!(specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Imperial).is_empty());
    assert!(character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Imperial).is_empty());
}

/// Yaga Minor (Imperial) with Imperial Fleet 2 (one ship, "Imperial
/// Star Destroyer 3", Darth Vader aboard) and Alliance Fleet 3 in
/// orbit, Ghorman (Alliance) next to it. port: test data after A0
/// 2026-11-14-stale-views, second run.
struct Yaga {
    world: GameWorld,
    movement: rebellion_core::movement::MovementState,
    missions: rebellion_core::missions::MissionState,
    transport: rebellion_core::troop_transport::TroopTransportState,
    yaga: SystemKey,
    ghorman: SystemKey,
    imperial: FleetKey,
    alliance: FleetKey,
    vader: CharacterKey,
}

fn yaga() -> Yaga {
    let mut world = GameWorld::default();
    let yaga = add_system(&mut world, "Yaga Minor");
    world.systems[yaga].control = ControlKind::Controlled(Faction::Empire);
    let ghorman = add_system(&mut world, "Ghorman");
    world.systems[ghorman].dat_id = DatId::new(101);
    let class = world
        .capital_ship_classes
        .insert(rebellion_core::world::CapitalShipClass {
            dat_id: DatId::new(0x1400_0001),
            name: "Imperial Star Destroyer".into(),
            ..rebellion_core::world::CapitalShipClass::default()
        });
    let mut ship = rebellion_core::world::ShipInstance::new(class, 10, false);
    ship.id = 3;
    ship.name = Some("Imperial Star Destroyer 3".into());
    let imperial = world.insert_fleet(rebellion_core::world::Fleet {
        location: yaga,
        capital_ships: vec![ship],
        ..Default::default()
    });
    let vader = world.characters.insert(Character {
        dat_id: DatId::new(0x3000_0010),
        name: "Darth Vader".into(),
        is_alliance: false,
        is_empire: true,
        current_fleet: Some(imperial),
        ship: Some(3),
        ..default_character()
    });
    world.fleets[imperial].characters.push(vader);
    let alliance = world.insert_fleet(rebellion_core::world::Fleet {
        location: yaga,
        is_alliance: true,
        ..Default::default()
    });
    Yaga {
        world,
        movement: rebellion_core::movement::MovementState::new(),
        missions: rebellion_core::missions::MissionState::new(),
        transport: rebellion_core::troop_transport::TroopTransportState::new(),
        yaga,
        ghorman,
        imperial,
        alliance,
        vader,
    }
}

/// The day's view step (simulation 13f).
fn view_day(galaxy: &mut Yaga) {
    rebellion_core::object_views::advance_extras(
        &mut galaxy.world,
        &galaxy.movement,
        &galaxy.missions,
        &galaxy.transport,
    );
    rebellion_core::object_views::advance(
        &mut galaxy.world,
        &galaxy.transport,
        &galaxy.movement,
    );
}

fn imperial_names(galaxy: &Yaga) -> Vec<String> {
    character_rows(
        &galaxy.world,
        &FogState::new(Faction::Alliance),
        Faction::Alliance,
        PersonnelTab::Imperial,
    )
    .iter()
    .map(|row| row.display_name().to_owned())
    .collect()
}

#[test]
fn an_other_side_character_aboard_a_copied_fleet_names_the_copied_ship_then_location_unknown() {
    // FUN_00465bb0 names the container of the side's copy of the
    // character when it is a system or a capital ship, else keeps
    // 0x1895 "Location Unknown" (0x465c21). A0
    // 2026-11-14-stale-views second run: 1x-personnelfinder-empire-d85
    // (d57ed845) "Darth Vader - Imperial Star Destroyer 3" while the
    // Alliance's copy of Yaga Minor was frozen; d91 (1b8818d8) and d97
    // (56d33542) "Darth Vader - Location Unknown" once it went live
    // with Fleet 2 gone.
    let mut galaxy = yaga();
    view_day(&mut galaxy);
    // Fleet 3 withdraws to Ghorman; then Fleet 2 leaves Yaga Minor for
    // a system the Alliance does not see.
    galaxy.world.fleets[galaxy.alliance].location = galaxy.ghorman;
    view_day(&mut galaxy);
    let elsewhere = add_system(&mut galaxy.world, "Kuat");
    galaxy.world.systems[elsewhere].dat_id = DatId::new(102);
    galaxy.world.systems[elsewhere].control = ControlKind::Controlled(Faction::Empire);
    galaxy.world.fleets[galaxy.imperial].location = elsewhere;
    view_day(&mut galaxy);
    assert_eq!(
        imperial_names(&galaxy),
        ["Darth Vader - Imperial Star Destroyer 3"]
    );
    let rows = character_rows(
        &galaxy.world,
        &FogState::new(Faction::Alliance),
        Faction::Alliance,
        PersonnelTab::Imperial,
    );
    let [PersonnelFinderRow::Character {
        key,
        system,
        location,
        ..
    }] = &rows[..]
    else {
        panic!("{rows:?}");
    };
    assert_eq!(
        (*key, *system, location.as_str()),
        (galaxy.vader, Some(galaxy.yaga), "Yaga Minor")
    );
    // port: the copy keeps no state word; an injury out of sight does
    // not show.
    galaxy.world.characters[galaxy.vader].injury = 1;
    let rows = character_rows(
        &galaxy.world,
        &FogState::new(Faction::Alliance),
        Faction::Alliance,
        PersonnelTab::Imperial,
    );
    let [PersonnelFinderRow::Character { name, state, .. }] = &rows[..] else {
        panic!("{rows:?}");
    };
    assert_eq!(
        (name.as_str(), state.as_str()),
        ("Darth Vader - Imperial Star Destroyer 3", "")
    );
    galaxy.world.characters[galaxy.vader].injury = 0;
    // Fleet 3 is ordered back and is en route: the copy stays frozen.
    assert!(galaxy
        .movement
        .order(galaxy.alliance, galaxy.ghorman, galaxy.yaga, 6));
    galaxy.world.fleets[galaxy.alliance].location = galaxy.yaga;
    view_day(&mut galaxy);
    assert_eq!(
        imperial_names(&galaxy),
        ["Darth Vader - Imperial Star Destroyer 3"]
    );
    // The arrival day: the copy goes live without Fleet 2.
    galaxy.movement.cancel(galaxy.alliance);
    view_day(&mut galaxy);
    assert_eq!(imperial_names(&galaxy), ["Darth Vader - Location Unknown"]);
    let rows = character_rows(
        &galaxy.world,
        &FogState::new(Faction::Alliance),
        Faction::Alliance,
        PersonnelTab::Imperial,
    );
    let [PersonnelFinderRow::Character {
        system, location, ..
    }] = &rows[..]
    else {
        panic!("{rows:?}");
    };
    assert_eq!((*system, location.as_str()), (None, ""));
    // And stays so the next days.
    view_day(&mut galaxy);
    assert_eq!(imperial_names(&galaxy), ["Darth Vader - Location Unknown"]);
    // Seen live again, it names its ship.
    galaxy.world.fleets[galaxy.imperial].location = galaxy.yaga;
    view_day(&mut galaxy);
    assert_eq!(
        imperial_names(&galaxy),
        ["Darth Vader - Imperial Star Destroyer 3"]
    );
}

#[test]
fn display_on_a_copied_character_opens_its_copied_fleet_or_system_or_nothing() {
    // FUN_00429440 on the side's copy (FUN_004f2d10): a ship container
    // opens its fleet's system and the Fleet window (kind 4); a system
    // container the Defenses window (kind 10); no ship or system leaves
    // the target null, so nothing opens (FUN_00464890 case 0xc9 still
    // closes the Finder). The live character's place never shows.
    let mut galaxy = yaga();
    let piett = galaxy.world.characters.insert(Character {
        dat_id: DatId::new(0x3000_0011),
        name: "Piett".into(),
        is_alliance: false,
        is_empire: true,
        current_system: Some(galaxy.yaga),
        ..default_character()
    });
    view_day(&mut galaxy);
    galaxy.world.fleets[galaxy.alliance].location = galaxy.ghorman;
    view_day(&mut galaxy);
    let elsewhere = add_system(&mut galaxy.world, "Kuat");
    galaxy.world.systems[elsewhere].dat_id = DatId::new(102);
    galaxy.world.systems[elsewhere].control = ControlKind::Controlled(Faction::Empire);
    galaxy.world.fleets[galaxy.imperial].location = elsewhere;
    galaxy.world.characters[piett].current_system = Some(elsewhere);
    view_day(&mut galaxy);
    let actions = |galaxy: &Yaga| {
        character_rows(
            &galaxy.world,
            &FogState::new(Faction::Alliance),
            Faction::Alliance,
            PersonnelTab::Imperial,
        )
        .iter()
        .map(|row| action_for_row(row, &galaxy.world, PersonnelTab::Imperial))
        .collect::<Vec<_>>()
    };
    // Rows in name order: Darth Vader, Piett.
    assert_eq!(
        actions(&galaxy),
        [
            PersonnelFinderAction::OpenCopy(CopyPlace::Fleet(
                crate::fleet_window::FleetWindowEntry::Copy {
                    alliance: true,
                    system: galaxy.yaga,
                    fleet: 0,
                }
            )),
            PersonnelFinderAction::OpenCopy(CopyPlace::System(galaxy.yaga)),
        ]
    );
    // Seen again without either: nothing opens.
    galaxy.world.fleets[galaxy.alliance].location = galaxy.yaga;
    view_day(&mut galaxy);
    assert_eq!(
        actions(&galaxy),
        [
            PersonnelFinderAction::OpenCopy(CopyPlace::Nowhere),
            PersonnelFinderAction::OpenCopy(CopyPlace::Nowhere),
        ]
    );
}

#[test]
fn a_copied_rows_state_word_is_on_mission_before_enroute_and_only_with_a_place() {
    // FUN_00465bb0 on the side's copy: "On Mission" (0x8745) from the
    // copy's mission key, else "Enroute" (0x8744) from +0x50 bit 4, in
    // " ( " and " ) "; a copy in no ship or system jumps past the words
    // (0x465cb5) and reads "Location Unknown" alone. FUN_00429440 opens
    // the Missions window for a copy on a mission that is not hidden
    // (FUN_00520b70), else the Defenses window.
    use rebellion_core::object_views::{CharacterCopy, CharacterCopyState, SystemView};
    let mut world = GameWorld::default();
    let kuat = add_system(&mut world, "Kuat");
    world.systems[kuat].control = ControlKind::Controlled(Faction::Empire);
    let id = world.systems[kuat].dat_id;
    let view = SystemView::of(&world, &world.systems[kuat], false);
    world.object_views.alliance_systems.insert(id, view);
    let add = |world: &mut GameWorld, raw: u32, name: &str, at: Option<DatId>| {
        world.characters.insert(Character {
            dat_id: DatId::new(raw),
            name: name.into(),
            is_alliance: false,
            is_empire: true,
            ..default_character()
        });
        world.object_views.alliance_characters.insert(
            DatId::new(raw),
            CharacterCopy {
                system: at,
                ship: None,
            },
        );
    };
    add(&mut world, 0x3000_0021, "Ardus Kaine", Some(id));
    add(&mut world, 0x3000_0022, "Bevel Lemelisk", Some(id));
    add(&mut world, 0x3000_0023, "Crix Madine", Some(id));
    add(&mut world, 0x3000_0024, "Dodd", None);
    add(&mut world, 0x3000_0025, "Eryn", Some(id));
    let states = &mut world.object_views.alliance_character_states;
    let state = |on_mission, hidden_mission, enroute| CharacterCopyState {
        on_mission,
        hidden_mission,
        enroute,
    };
    states.insert(DatId::new(0x3000_0021), state(true, false, true));
    states.insert(DatId::new(0x3000_0022), state(false, false, true));
    states.insert(DatId::new(0x3000_0023), state(true, true, false));
    states.insert(DatId::new(0x3000_0024), state(false, false, true));
    let rows = character_rows(
        &world,
        &FogState::new(Faction::Alliance),
        Faction::Alliance,
        PersonnelTab::Imperial,
    );
    let texts: Vec<_> = rows.iter().map(PersonnelFinderRow::display_name).collect();
    assert_eq!(
        texts,
        [
            "Ardus Kaine - Kuat ( On Mission ) ",
            "Bevel Lemelisk - Kuat ( Enroute ) ",
            "Crix Madine - Kuat ( On Mission ) ",
            "Dodd - Location Unknown",
            "Eryn - Kuat",
        ]
    );
    let actions: Vec<_> = rows
        .iter()
        .map(|row| action_for_row(row, &world, PersonnelTab::Imperial))
        .collect();
    assert_eq!(
        actions,
        [
            PersonnelFinderAction::OpenCopy(CopyPlace::Missions(kuat)),
            PersonnelFinderAction::OpenCopy(CopyPlace::System(kuat)),
            PersonnelFinderAction::OpenCopy(CopyPlace::System(kuat)),
            PersonnelFinderAction::OpenCopy(CopyPlace::Nowhere),
            PersonnelFinderAction::OpenCopy(CopyPlace::System(kuat)),
        ]
    );
}

#[test]
fn a_copied_row_keeps_the_command_word_its_copy_held_when_it_froze() {
    // FUN_00464e10 hands FUN_00465bb0 the side's copy of the character
    // (FUN_004f2ec0 with the window's side), and FUN_00465bb0 reads its
    // +0x96 for 0x8802 Commander, 0x8801 Admiral, 0x8800 General after
    // the state word. The master's setter FUN_004ee350 reaches a copy
    // through FUN_004f04f0, which writes only while FUN_005404d0 holds:
    // a frozen copy keeps its old word. A0 2026-11-15-partial-cells/r3-
    // personnel-finder-imperial-no-captives-d1258 (6966a4fce5ca):
    // "Brandei - Yaga Minor ( General )", "Griff - Gorgon ( Admiral )".
    use rebellion_core::character_command::CommandKind;
    use rebellion_core::object_views::{CharacterCopy, CharacterCopyState, SystemView};
    let mut world = GameWorld::default();
    let kuat = add_system(&mut world, "Kuat");
    world.systems[kuat].control = ControlKind::Controlled(Faction::Empire);
    let id = world.systems[kuat].dat_id;
    let view = SystemView::of(&world, &world.systems[kuat], false);
    world.object_views.alliance_systems.insert(id, view);
    for (raw, name, at) in [
        (0x3000_0031, "Brandei", Some(id)),
        (0x3000_0032, "Griff", Some(id)),
        (0x3000_0033, "Needa", None),
    ] {
        world.characters.insert(Character {
            dat_id: DatId::new(raw),
            name: name.into(),
            is_alliance: false,
            is_empire: true,
            // The live character holds no command now.
            command: CommandKind::None,
            ..default_character()
        });
        world.object_views.alliance_characters.insert(
            DatId::new(raw),
            CharacterCopy {
                system: at,
                ship: None,
            },
        );
        world
            .object_views
            .alliance_character_commands
            .insert(DatId::new(raw), CommandKind::General);
    }
    world
        .object_views
        .alliance_character_commands
        .insert(DatId::new(0x3000_0032), CommandKind::Admiral);
    world.object_views.alliance_character_states.insert(
        DatId::new(0x3000_0032),
        CharacterCopyState {
            on_mission: false,
            hidden_mission: false,
            enroute: true,
        },
    );
    let rows = character_rows(
        &world,
        &FogState::new(Faction::Alliance),
        Faction::Alliance,
        PersonnelTab::Imperial,
    );
    let texts: Vec<_> = rows.iter().map(PersonnelFinderRow::display_name).collect();
    assert_eq!(
        texts,
        [
            "Brandei - Kuat ( General ) ",
            "Griff - Kuat ( Enroute )  ( Admiral ) ",
            // No container: no words at all (0x465cb5 -> 0x46606f).
            "Needa - Location Unknown",
        ]
    );
}

#[test]
fn an_own_captives_row_reads_the_players_frozen_copy_of_it() {
    // FUN_00464e10 hands FUN_00465bb0 the player's side's copy
    // (FUN_004f2ec0); a captive carries its captor's side
    // (FUN_004eec60), so the Alliance's copy froze where it last saw
    // him. A0 2026-11-18-edited-saves-7 b-19 (e4f53d49b065), day 31:
    // "Luke Skywalker - Jabba's Palace" (an abode copy: its name alone)
    // while the master stands at Coruscant; A0 personnel-captured/
    // finder-day628: "Jan Dodonna - Punisher ( Captured )".
    use rebellion_core::object_views::CharacterCopy;
    let mut world = GameWorld::default();
    let coruscant = add_system(&mut world, "Coruscant");
    let yavin = add_system(&mut world, "Yavin");
    let yavin_id = world.systems[yavin].dat_id;
    for (raw, name, copy) in [
        (
            0x3000_0041,
            "Luke Skywalker",
            Some(CharacterCopy {
                system: None,
                ship: Some("Jabba's Palace".into()),
            }),
        ),
        (
            0x3000_0042,
            "Jan Dodonna",
            Some(CharacterCopy {
                system: Some(yavin_id),
                ship: Some("Punisher".into()),
            }),
        ),
        // No copy taken yet: read live.
        (0x3000_0043, "Wedge Antilles", None),
    ] {
        world.characters.insert(Character {
            dat_id: DatId::new(raw),
            name: name.into(),
            is_captive: true,
            current_system: Some(coruscant),
            ..default_character()
        });
        if let Some(copy) = copy {
            world
                .object_views
                .alliance_characters
                .insert(DatId::new(raw), copy);
        }
    }
    let rows = character_rows(
        &world,
        &FogState::new(Faction::Alliance),
        Faction::Alliance,
        PersonnelTab::Alliance,
    );
    let texts: Vec<_> = rows.iter().map(PersonnelFinderRow::display_name).collect();
    assert_eq!(
        texts,
        [
            "Jan Dodonna - Punisher ( Captured ) ",
            "Luke Skywalker - Jabba's Palace",
            "Wedge Antilles - Coruscant ( Captured ) ",
        ]
    );
    // The palace copy opens nothing (FUN_00429440's abode branch).
    let luke = rows
        .iter()
        .find(|row| row.display_name().starts_with("Luke"))
        .unwrap();
    assert!(matches!(
        luke,
        PersonnelFinderRow::Character {
            copy: Some(CopyPlace::Nowhere),
            ..
        }
    ));
    // Freed, the player's own character reads live even with a copy.
    for (_, character) in &mut world.characters {
        character.is_captive = false;
    }
    let rows = character_rows(
        &world,
        &FogState::new(Faction::Alliance),
        Faction::Alliance,
        PersonnelTab::Alliance,
    );
    let texts: Vec<_> = rows.iter().map(PersonnelFinderRow::display_name).collect();
    assert_eq!(
        texts,
        [
            "Jan Dodonna - Coruscant",
            "Luke Skywalker - Coruscant",
            "Wedge Antilles - Coruscant",
        ]
    );
}

#[test]
fn a_fleet_carrying_the_tabs_sf_units_is_a_row_of_its_own() {
    // FUN_00464e10: after a system's own row, each fleet of the tab's
    // side there whose ships carry special forces gets a row
    // (FUN_00442130 with the fleet), counted over the fleet
    // (FUN_00465540, type 8).
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");
    let unit = add_sf(&mut world, yavin, 0x3c00_0002, true);
    world.systems[yavin].special_forces.clear();
    let fleet = world.insert_fleet(rebellion_core::world::Fleet {
        location: yavin,
        is_alliance: true,
        special_forces: vec![unit],
        ..Default::default()
    });
    world.systems[yavin].fleets.push(fleet);
    let fog = FogState::new(Faction::Alliance);

    let listed = specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
    assert_eq!(listed.len(), 1);
    let PersonnelFinderRow::SpecForces {
        name,
        system,
        fleet: row_fleet,
        counts,
    } = &listed[0]
    else {
        panic!("expected a SpecForces row");
    };
    assert_eq!(
        Some(name.clone()),
        crate::system_window::fleet_label(&world, fleet)
    );
    assert_eq!((*system, *row_fleet), (yavin, Some(fleet)));
    assert_eq!(counts[1].count, 1);
    // The other side's tab lists nothing.
    assert!(
        specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Imperial).is_empty()
    );
}

#[test]
fn the_characters_view_lists_recruited_living_characters_of_the_tabs_side() {
    // FUN_00464e10 view==2: walk characters of the side, call
    // FUN_00465bb0 for each.
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");

    let mut luke = default_character();
    luke.name = "Luke Skywalker".into();
    luke.is_alliance = true;
    luke.current_system = Some(yavin);
    luke.recruited = true;
    world.characters.insert(luke);

    let mut vader = default_character();
    vader.name = "Darth Vader".into();
    vader.is_alliance = false;
    vader.is_empire = true;
    vader.current_system = Some(yavin);
    vader.recruited = true;
    world.characters.insert(vader);

    // Not recruited: hidden.
    let mut hidden = default_character();
    hidden.name = "Hidden".into();
    hidden.is_alliance = true;
    hidden.recruited = false;
    world.characters.insert(hidden);

    let fog = FogState::new(Faction::Alliance);

    let alliance = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
    assert_eq!(alliance.len(), 1);
    assert!(alliance[0].display_name().starts_with("Luke Skywalker"));

    // A0 personnel-other-side/test-day8-imperial-tab-empty: the other
    // side's character is listed only where the player sees it.
    world.systems[yavin].control = ControlKind::Uncontrolled;
    let imperial = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Imperial);
    assert!(imperial.is_empty());
    world.systems[yavin].control = ControlKind::Controlled(Faction::Alliance);
    let imperial = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Imperial);
    assert_eq!(imperial.len(), 1);
    assert!(imperial[0].display_name().starts_with("Darth Vader"));
    // The Empire sees its own characters wherever they are.
    world.systems[yavin].control = ControlKind::Uncontrolled;
    let own = character_rows(
        &world,
        &FogState::new(Faction::Empire),
        Faction::Empire,
        PersonnelTab::Imperial,
    );
    assert_eq!(own.len(), 1);
}

#[test]
fn a_chosen_character_row_copies_only_the_characters_name_into_the_box() {
    // FUN_00464890 case 0xca, 0x29b: FUN_00604f90 takes the record's
    // name (FUN_00583c40); A0 2026-11-17-edited-saves-6 d-02
    // (bab3f3b8d33e) and d-05 (33acd757c002) read "Luke Skywalker"
    // and "Han Solo" in the box for "Luke Skywalker - Dagobah" and
    // "Han Solo - Sluis Van ( Captured )".
    let mut world = GameWorld::default();
    let sluis = add_system(&mut world, "Sluis Van");
    let dagobah = world.abodes.insert(rebellion_core::world::Abode {
        dat_id: rebellion_core::ids::DatId::new(0x441),
        name: "Dagobah".into(),
        side: Faction::Neutral,
        special_forces: Vec::new(),
    });
    let mut luke = default_character();
    luke.name = "Luke Skywalker".into();
    luke.current_abode = Some(dagobah);
    world.characters.insert(luke);
    let mut han = default_character();
    han.name = "Han Solo".into();
    han.current_system = Some(sluis);
    han.is_captive = true;
    world.characters.insert(han);

    let fog = FogState::new(Faction::Alliance);
    let rows = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
    let shown: Vec<_> = rows
        .iter()
        .map(|row| (row.display_name().to_owned(), box_name(row, &world)))
        .collect();
    assert_eq!(
        shown,
        [
            (
                "Han Solo - Sluis Van ( Captured ) ".to_owned(),
                Some("Han Solo".to_owned())
            ),
            (
                "Luke Skywalker - Dagobah".to_owned(),
                Some("Luke Skywalker".to_owned())
            ),
        ]
    );
}

#[test]
fn a_chosen_retired_or_killed_characters_row_leaves_the_name_box_alone() {
    // FUN_00464890 case 0xca, 0x29b copies the name only when the
    // object exists (+0x50 bit 6: created and not destroyed,
    // 0x004f7bc0). A0 2026-11-18-edited-saves-9 (an unedited Empire
    // game) d-04 5b7a43b193d5 and d-08 2f1d872c5a2e: "Ozzel - Retired"
    // chosen, the name box empty.
    let mut world = GameWorld::default();
    for (name, retired) in [("Ozzel", true), ("Needa", false)] {
        let mut character = default_character();
        character.name = name.into();
        character.is_alliance = false;
        character.is_empire = true;
        character.mark_killed();
        character.is_retired = retired;
        world.characters.insert(character);
    }

    let fog = FogState::new(Faction::Empire);
    let rows = character_rows(&world, &fog, Faction::Empire, PersonnelTab::Imperial);
    let shown: Vec<_> = rows
        .iter()
        .map(|row| (row.display_name().to_owned(), box_name(row, &world)))
        .collect();
    assert_eq!(
        shown,
        [
            ("Needa - Killed".to_owned(), None),
            ("Ozzel - Retired".to_owned(), None),
        ]
    );
}

#[test]
fn display_opens_the_fleet_the_missions_or_the_defenses_window() {
    // FUN_00429440 family 0x30: a fleet parent opens kind 4; at a
    // system, a mission that is not hidden (FUN_00520b70 == 0) opens
    // kind 11, anything else kind 10. Manual p. 100: "system defenses,
    // fleet or mission".
    let mut character = default_character();
    assert_eq!(character_target(&character), CharacterTarget::Defenses);
    character.on_mission = true;
    assert_eq!(character_target(&character), CharacterTarget::Missions);
    character.on_hidden_mission = true;
    assert_eq!(character_target(&character), CharacterTarget::Defenses);
    let fleet = FleetKey::default();
    character.current_fleet = Some(fleet);
    character.on_hidden_mission = false;
    assert_eq!(character_target(&character), CharacterTarget::Fleet(fleet));
}

#[test]
fn a_character_on_mission_shows_its_state_in_the_name() {
    // FUN_00465bb0: "Name - Location ( On Mission ) ".
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");

    let mut luke = default_character();
    luke.name = "Luke Skywalker".into();
    luke.current_system = Some(yavin);
    luke.on_mission = true;
    world.characters.insert(luke);

    let fog = FogState::new(Faction::Alliance);
    let listed = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
    assert!(listed[0].display_name().contains("On Mission"));
}

#[test]
fn a_travelling_character_names_its_destination_and_enroute() {
    // A0 specforces/00: "Adar Tallon - Mon Calamari ( Enroute )", the
    // trip's destination as its container (FUN_00465bb0).
    let mut world = GameWorld::default();
    let mon_calamari = add_system(&mut world, "Mon Calamari");
    let mut adar = default_character();
    adar.name = "Adar Tallon".into();
    adar.current_system = None;
    let key = world.characters.insert(adar);

    let fog = FogState::new(Faction::Alliance);
    let listed = character_rows_travelling(
        &world,
        &fog,
        Faction::Alliance,
        PersonnelTab::Alliance,
        &[(key, mon_calamari)],
    );
    assert_eq!(listed[0].display_name(), "Adar Tallon - Mon Calamari ( Enroute ) ");
    // With no container FUN_00465bb0 jumps past the state and command
    // words (0x465c0c, 0x465cb5 to 0x46606f): "Location Unknown" alone.
    let unknown = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
    assert_eq!(unknown[0].display_name(), "Adar Tallon - Location Unknown");
    // The window keeps the destinations main sets each frame.
    let mut state = PersonnelFinderState::default();
    state.set_destinations(vec![(key, mon_calamari)]);
    assert_eq!(state.destinations, [(key, mon_calamari)]);
}

#[test]
fn the_open_characters_view_reports_its_rows_with_destinations() {
    let mut world = GameWorld::default();
    let mon_calamari = add_system(&mut world, "Mon Calamari");
    let mut adar = default_character();
    adar.name = "Adar Tallon".into();
    adar.current_system = None;
    let key = world.characters.insert(adar);
    let fog = FogState::new(Faction::Alliance);
    let mut state = PersonnelFinderState::default();
    state.set_destinations(vec![(key, mon_calamari)]);
    assert_eq!(state.shown_character_rows(&world, &fog), None, "closed");

    state.open(CockpitFaction::Alliance);
    assert_eq!(
        state.shown_character_rows(&world, &fog),
        Some(vec!["Adar Tallon - Mon Calamari ( Enroute ) ".to_owned()])
    );
    state.set_view(PersonnelView::SpecForces);
    assert_eq!(state.shown_character_rows(&world, &fog), None, "SpecForces view");
}

#[test]
fn a_character_in_no_place_reads_location_unknown_without_words() {
    // FUN_00465bb0: with no container (vtable +0xc, FUN_004ece60 on its
    // id) the row keeps 0x1895 "Location Unknown" and jumps past the
    // Captured, On Mission, Injured, Enroute and command words
    // (0x465cb5 -> 0x46606f).
    let mut world = GameWorld::default();
    let mut ozzel = default_character();
    ozzel.name = "Ozzel".into();
    ozzel.injury = 40;
    ozzel.command = rebellion_core::character_command::CommandKind::Admiral;
    world.characters.insert(ozzel);
    let fog = FogState::new(Faction::Alliance);
    let listed = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
    assert_eq!(listed[0].display_name(), "Ozzel - Location Unknown");
}

#[test]
fn a_killed_character_is_listed_as_killed() {
    // FUN_00465bb0's destroyed branch; the reference capture lists
    // "Ackbar - Killed" (steam-guide guide-023.png).
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");

    let mut luke = default_character();
    luke.name = "Luke Skywalker".into();
    luke.current_system = Some(yavin);
    luke.is_killed = true;
    world.characters.insert(luke);

    let fog = FogState::new(Faction::Alliance);
    let listed = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
    assert_eq!(listed[0].display_name(), "Luke Skywalker - Killed");
}

#[test]
fn a_retired_character_is_listed_as_retired() {
    // FUN_00465bb0: destruction reason 0x14..0x16 reads 0x8799
    // "Retired"; a Retire order destroys with 0x14 (FUN_00534c00).
    let mut world = GameWorld::default();
    let mut han = default_character();
    han.name = "Han Solo".into();
    han.is_killed = true;
    han.is_retired = true;
    world.characters.insert(han);

    let fog = FogState::new(Faction::Alliance);
    let listed = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
    assert_eq!(listed[0].display_name(), "Han Solo - Retired");
}

#[test]
fn a_rows_state_word_sits_in_spaced_brackets_and_captured_comes_first() {
    // FUN_00465bb0: " ( " + the first of Captured, On Mission, Injured,
    // Enroute + " ) " (DAT_006a8790, DAT_006a878c).
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");
    let mut leia = default_character();
    leia.name = "Leia".into();
    leia.current_system = Some(yavin);
    leia.on_mission = true;
    leia.is_captive = true;
    world.characters.insert(leia);
    let mut han = default_character();
    han.name = "Han".into();
    world.characters.insert(han);
    // Aboard a fleet is not en route, wherever the fleet is.
    let mut wedge = default_character();
    wedge.name = "Wedge".into();
    wedge.current_fleet = Some(rebellion_core::ids::FleetKey::default());
    world.characters.insert(wedge);

    let fog = FogState::new(Faction::Alliance);
    let listed = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
    assert_eq!(
        rows(
            &world,
            &fog,
            Faction::Alliance,
            PersonnelTab::Alliance,
            PersonnelView::Characters
        ),
        listed
    );
    let names: Vec<&str> = listed
        .iter()
        .map(PersonnelFinderRow::display_name)
        .collect();
    assert_eq!(
        names,
        [
            // No container: no state word (0x465cb5 -> 0x46606f).
            "Han - Location Unknown",
            "Leia - Yavin ( Captured ) ",
            "Wedge - Location Unknown"
        ]
    );
}

#[test]
fn a_character_aboard_names_its_ship_and_one_unseated_its_fleet() {
    // FUN_00465bb0 names the container, the ship a character rides
    // (A0: "Jan Dodonna - Punisher ( Captured )").
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");
    let class = world
        .capital_ship_classes
        .insert(rebellion_core::world::CapitalShipClass {
            name: "Galleon".into(),
            ..rebellion_core::world::CapitalShipClass::default()
        });
    let mut first = rebellion_core::world::ShipInstance::new(class, 10, false);
    first.id = 7;
    let mut second = rebellion_core::world::ShipInstance::new(class, 10, false);
    second.id = 8;
    second.name = Some("Punisher".into());
    let fleet = world.fleets.insert(rebellion_core::world::Fleet {
        location: yavin,
        capital_ships: vec![first, second],
        ..Default::default()
    });
    let mut dodonna = default_character();
    dodonna.name = "Jan Dodonna".into();
    dodonna.current_fleet = Some(fleet);
    dodonna.ship = Some(8);
    dodonna.is_captive = true;
    let dodonna = world.characters.insert(dodonna);
    assert_eq!(
        character_text(&world, &world.characters[dodonna], None),
        "Jan Dodonna - Punisher ( Captured ) "
    );
    world.characters[dodonna].ship = Some(7);
    assert_eq!(
        character_text(&world, &world.characters[dodonna], None),
        "Jan Dodonna - Galleon ( Captured ) "
    );
    world.characters[dodonna].ship = None;
    assert_eq!(
        character_text(&world, &world.characters[dodonna], None),
        format!(
            "Jan Dodonna - {} ( Captured ) ",
            crate::system_window::fleet_label(&world, fleet).unwrap()
        )
    );
    world.characters[dodonna].ship = Some(9);
    assert_eq!(
        character_text(&world, &world.characters[dodonna], None),
        format!(
            "Jan Dodonna - {} ( Captured ) ",
            crate::system_window::fleet_label(&world, fleet).unwrap()
        )
    );
}

#[test]
fn a_row_adds_injury_and_command_and_an_abode_names_only_the_place() {
    // FUN_00465bb0: Injured (0x8747, +0x94) after Captured and On
    // Mission; the command word (+0x96: 1 Commander 0x8802, 2 Admiral
    // 0x8801, 3 General 0x8800) after the state; an abode (family
    // 0x98) gives its name and nothing more.
    use rebellion_core::character_command::CommandKind;
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");
    let jail = world.abodes.insert(rebellion_core::world::Abode {
        dat_id: rebellion_core::ids::DatId::new(0x9800_0480),
        name: "Empire Jail".into(),
        side: Faction::Empire,
        special_forces: Vec::new(),
    });
    let mut ackbar = default_character();
    ackbar.name = "Ackbar".into();
    ackbar.current_system = Some(yavin);
    ackbar.injury = 3;
    ackbar.command = CommandKind::Admiral;
    world.characters.insert(ackbar);
    let mut dodonna = default_character();
    dodonna.name = "Dodonna".into();
    dodonna.current_system = Some(yavin);
    dodonna.command = CommandKind::General;
    world.characters.insert(dodonna);
    let mut leia = default_character();
    leia.name = "Leia".into();
    leia.current_abode = Some(jail);
    leia.is_captive = true;
    leia.command = CommandKind::Commander;
    world.characters.insert(leia);

    let fog = FogState::new(Faction::Alliance);
    let listed = character_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
    let names: Vec<&str> = listed
        .iter()
        .map(PersonnelFinderRow::display_name)
        .collect();
    assert_eq!(
        names,
        [
            "Ackbar - Yavin ( Injured )  ( Admiral ) ",
            "Dodonna - Yavin ( General ) ",
            "Leia - Empire Jail"
        ]
    );
}

#[test]
fn rows_are_sorted_by_name_ignoring_case() {
    // FUN_005f59f0: rows inserted in name order.
    let mut world = GameWorld::default();
    let beta = add_system(&mut world, "beta");
    let alpha = add_system(&mut world, "Alpha");
    add_sf(&mut world, beta, 0x3c00_0004, true);
    add_sf(&mut world, alpha, 0x3c00_0004, true);
    let fog = FogState::new(Faction::Alliance);

    let listed = specforces_rows(&world, &fog, Faction::Alliance, PersonnelTab::Alliance);
    let names: Vec<&str> = listed.iter().map(|r| r.display_name()).collect();
    assert_eq!(names, ["Alpha", "beta"]);
}

#[test]
fn a_typed_name_chooses_the_first_row_it_best_begins() {
    // FUN_00609650: prefix match across both row kinds.
    let rows = vec![
        PersonnelFinderRow::SpecForces {
            name: "Coruscant".into(),
            system: SystemKey::default(),
            fleet: None,
            counts: [SfCount::default(); 4],
        },
        PersonnelFinderRow::SpecForces {
            name: "Yavin".into(),
            system: SystemKey::default(),
            fleet: None,
            counts: [SfCount::default(); 4],
        },
    ];
    assert_eq!(prefix_choice(&rows, "ya"), Some(1));
    assert_eq!(prefix_choice(&rows, "cor"), Some(0));
    // FUN_00609650: with no row sharing the first letter, the first.
    assert_eq!(prefix_choice(&rows, "z"), Some(0));
    assert_eq!(prefix_choice(&rows, ""), Some(0));
}

#[test]
fn the_action_for_a_character_row_opens_character_and_sf_row_its_first_unit() {
    // FUN_00429440: a character by family 0x30; a SpecForces row by its
    // system's first special force of the tab (A0 specforces/02, 03).
    let mut world = GameWorld::default();
    let system = add_system(&mut world, "Yavin");
    let character = CharacterKey::default();

    let char_row = PersonnelFinderRow::Character {
        key: character,
        name: "Luke - Yavin".into(),
        location: "Yavin".into(),
        state: String::new(),
        system: Some(system),
        copy: None,
    };
    assert_eq!(
        action_for_row(&char_row, &world, PersonnelTab::Alliance),
        PersonnelFinderAction::OpenCharacter {
            character,
            system: Some(system),
        }
    );

    let imperial = add_sf(&mut world, system, 0x3c00_0001, false);
    let first = add_sf(&mut world, system, 0x3c00_0002, true);
    add_sf(&mut world, system, 0x3c00_0003, true);
    let sf_row = PersonnelFinderRow::SpecForces {
        name: "Yavin".into(),
        system,
        fleet: None,
        counts: [SfCount::default(); 4],
    };
    assert_eq!(
        action_for_row(&sf_row, &world, PersonnelTab::Alliance),
        PersonnelFinderAction::OpenSystem {
            system,
            unit: Some(first),
        }
    );
    assert_eq!(
        action_for_row(&sf_row, &world, PersonnelTab::Imperial),
        PersonnelFinderAction::OpenSystem {
            system,
            unit: Some(imperial),
        }
    );
}

#[test]
fn scrolled_rows_clip_to_the_list_and_keep_their_centres() {
    // Twelve characters scrolled 30 pixels: row 0 (-30..-10) is gone,
    // row 1 (-10..10) centres on the list's top, row 9 (150..170) shows
    // its upper part, row 10 (170..) nothing. A press in the bar's
    // column chooses nothing; one at x 370 chooses row 4; a wheel notch
    // moves a row (port).
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");
    for index in 0..12 {
        let mut character = default_character();
        character.name = format!("C{index:02}");
        character.is_alliance = true;
        character.current_system = Some(yavin);
        character.recruited = true;
        world.characters.insert(character);
    }
    let fog = FogState::new(Faction::Alliance);
    let origin = window_rect(layout()).min;
    let run = |state: &mut PersonnelFinderState, at: egui::Pos2, extra: Vec<egui::Event>| {
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let mut shapes = Vec::new();
        let frames = [vec![], vec![]].into_iter().chain(extra.into_iter().map(|event| vec![event]));
        for events in frames {
            let mut input = vec![egui::Event::PointerMoved(at)];
            input.extend(events);
            shapes = ctx
                .run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 520.0))),
                        events: input,
                        ..Default::default()
                    },
                    |ctx| {
                        let _ = draw_personnel_finder(ctx, &world, &fog, state, layout(), &mut cache);
                    },
                )
                .shapes;
        }
        shapes
    };
    let press = |at: egui::Pos2, pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let opened = || {
        let mut state = PersonnelFinderState::default();
        state.open(CockpitFaction::Alliance);
        state.window.as_mut().unwrap().scroll = 30;
        state
    };

    let mut state = opened();
    let in_bar = origin + egui::vec2(380.0, LIST.1 + 50.0);
    let shapes = run(&mut state, in_bar, vec![press(in_bar, true), press(in_bar, false)]);
    // Each row DT_VCENTER in its 20 (3 down, then tmAscent 11); row k's
    // top at LIST.1 + 20 k - 30.
    let lines = crate::theme::test_lines(&shapes);
    let baseline = |name: &str| {
        lines
            .iter()
            .find(|line| line.text.starts_with(name))
            .map(|line| line.pen.y - origin.y)
    };
    assert_eq!(baseline("C00 "), None);
    assert!((baseline("C01 ").unwrap() - (LIST.1 - 10.0 + 14.0)).abs() < 0.01);
    assert!((baseline("C09 ").unwrap() - (LIST.1 + 150.0 + 14.0)).abs() < 0.01);
    assert_eq!(baseline("C10 "), None);
    assert_eq!(state.window.as_ref().unwrap().chosen, None);

    let mut state = opened();
    let near_bar = origin + egui::vec2(370.0, LIST.1 + 50.0);
    let _ = run(&mut state, near_bar, vec![press(near_bar, true), press(near_bar, false)]);
    assert_eq!(state.window.as_ref().unwrap().chosen, Some(4));

    let wheel = |delta: f32| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, delta),
        modifiers: egui::Modifiers::default(),
    };
    let mut state = opened();
    let _ = run(&mut state, near_bar, vec![wheel(-1.0)]);
    assert_eq!(state.window.as_ref().unwrap().scroll, 50);
    let _ = run(&mut state, near_bar, vec![wheel(1.0)]);
    assert_eq!(state.window.as_ref().unwrap().scroll, 30);
}

#[test]
fn eight_whole_rows_fit_the_list() {
    // FUN_00607ea0: list 161 tall, rows 20 tall.
    assert_eq!(LIST_PIXELS / ROW_PIXELS, 8);
}

#[test]
fn the_list_scrolls_as_little_as_it_must_to_show_a_row() {
    // By pixels (FUN_0060a490): row 8 (160..180) needs 19; twenty rows
    // reach 400 + 5 - 161 = 244 (FUN_00609ae0).
    assert_eq!(scrolled_to(0, 7, 20), 0);
    assert_eq!(scrolled_to(0, 8, 20), 19);
    assert_eq!(scrolled_to(60, 1, 20), 20);
    assert_eq!(scrolled_to(60, 10, 20), 60);
    assert_eq!(scrolled_to(40, 10, 20), 59);
    assert_eq!(max_scroll(20), 244);
    assert_eq!(max_scroll(8), 0);
}

#[test]
fn the_longest_common_beginning_wins_and_a_tie_keeps_the_first() {
    // FUN_00609650 keeps a row only for a strictly longer match.
    let rows: Vec<PersonnelFinderRow> = ["Corulag", "Coruscant", "Yavin"]
        .into_iter()
        .map(|name| PersonnelFinderRow::SpecForces {
            name: name.into(),
            system: SystemKey::default(),
            fleet: None,
            counts: [SfCount::default(); 4],
        })
        .collect();
    assert_eq!(prefix_choice(&rows, "corus"), Some(1));
    assert_eq!(prefix_choice(&rows, "cor"), Some(0));
}

#[test]
fn each_side_tab_names_its_side_and_paints_its_own_cells() {
    // TEXTSTRA 0x1893/0x1894; FUN_00465540's brushes as the Troop
    // Finder's: red with white text, green with black.
    assert_eq!(PersonnelTab::Alliance.label(), "Alliance Personnel");
    assert_eq!(PersonnelTab::Imperial.label(), "Imperial Personnel");
    assert_eq!(
        sf_cell_colours(PersonnelTab::Alliance, true),
        (egui::Color32::from_rgb(255, 0, 0), egui::Color32::WHITE)
    );
    assert_eq!(
        sf_cell_colours(PersonnelTab::Imperial, true),
        (egui::Color32::from_rgb(0, 255, 0), egui::Color32::BLACK)
    );
}

#[test]
fn opening_an_open_finder_keeps_its_state() {
    // FUN_0042a4d0 builds the window only when no type 0x17 is open.
    let mut state = PersonnelFinderState::default();
    assert!(state.open(CockpitFaction::Alliance));
    state.set_tab(PersonnelTab::Imperial);

    assert!(!state.open(CockpitFaction::Alliance));
    assert_eq!(state.window.as_ref().unwrap().tab, PersonnelTab::Imperial);
    assert!(state.is_open());
    state.close();
    assert!(!state.is_open());
}

#[test]
fn the_shown_side_tabs_report_the_tab_and_each_buttons_centre() {
    let mut state = PersonnelFinderState::default();
    assert_eq!(state.shown_side_tabs(layout()), None);
    state.open(CockpitFaction::Alliance);
    let frame = window_rect(layout());
    let (alliance, tabs) = state.shown_side_tabs(layout()).unwrap();
    assert!(alliance);
    assert_eq!(tabs[0] - frame.min, egui::vec2(36.0 + 24.5, 72.0 + 20.5));
    assert_eq!(tabs[1] - frame.min, egui::vec2(88.0 + 24.5, 72.0 + 20.5));
    state.set_tab(PersonnelTab::Imperial);
    assert!(!state.shown_side_tabs(layout()).unwrap().0);
}

#[test]
fn a_new_side_tab_clears_the_choice() {
    // FUN_00464e10: tab switch rebuilds the list.
    let mut state = PersonnelFinderState::default();
    state.open(CockpitFaction::Alliance);
    state.window.as_mut().unwrap().chosen = Some(0);

    state.set_tab(PersonnelTab::Imperial);
    assert_eq!(state.window.as_ref().unwrap().chosen, None);
}

#[test]
fn a_new_view_tab_clears_the_choice() {
    // FUN_00464e10: view switch rebuilds the list.
    let mut state = PersonnelFinderState::default();
    state.open(CockpitFaction::Alliance);
    state.window.as_mut().unwrap().chosen = Some(0);

    state.set_view(PersonnelView::SpecForces);
    assert_eq!(state.window.as_ref().unwrap().chosen, None);
}

#[test]
fn an_edit_that_keeps_the_chosen_row_leaves_the_list_where_it_is() {
    // FUN_00609650 chooses and scrolls only to a row not chosen
    // already (+0x3c bit 0, 0x60973f): "c" then "c0" both choose C00,
    // so the second edit leaves the list scrolled away from it.
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");
    for index in 0..12 {
        let mut character = default_character();
        character.name = format!("C{index:02}");
        character.is_alliance = true;
        character.current_system = Some(yavin);
        character.recruited = true;
        world.characters.insert(character);
    }
    let fog = FogState::new(Faction::Alliance);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut state = PersonnelFinderState::default();
    state.open(CockpitFaction::Alliance);
    let mut frame = |state: &mut PersonnelFinderState, events: Vec<egui::Event>| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(700.0, 520.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                let _ = draw_personnel_finder(ctx, &world, &fog, state, layout(), &mut cache);
            },
        );
    };
    frame(&mut state, vec![]);
    frame(&mut state, vec![egui::Event::Text("c".into())]);
    assert_eq!(state.window.as_ref().unwrap().chosen, Some(0));
    state.window.as_mut().unwrap().scroll = 80;
    frame(&mut state, vec![egui::Event::Text("0".into())]);
    let window = state.window.as_ref().unwrap();
    assert_eq!((window.name.as_str(), window.chosen), ("c0", Some(0)));
    assert_eq!(window.scroll, 80);
    // "c1" chooses C10, another row: the list scrolls to it.
    frame(&mut state, vec![egui::Event::Text("1".into())]);
    assert_eq!(state.window.as_ref().unwrap().chosen, Some(1));
}

#[test]
fn display_after_a_tab_change_opens_the_name_typed_before_it() {
    // FUN_00464c20 0x408 stores the typed name's best row in +0x178;
    // case 100 of FUN_00464890 rebuilds the list (FUN_00464e10) without
    // a prefix match and leaves +0x178; Display (case 0xc9) opens it.
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");
    let mut luke = default_character();
    luke.name = "Luke".into();
    luke.is_alliance = true;
    luke.current_system = Some(yavin);
    luke.recruited = true;
    let luke = world.characters.insert(luke);
    let fog = FogState::new(Faction::Alliance);
    let origin = window_rect(layout()).min;
    let display = origin + egui::vec2(423.0 + 16.0, 93.0 + 15.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut state = PersonnelFinderState::default();
    state.open(CockpitFaction::Alliance);
    let mut frame = |state: &mut PersonnelFinderState, events: Vec<egui::Event>| {
        let mut action = None;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(700.0, 520.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                action = draw_personnel_finder(ctx, &world, &fog, state, layout(), &mut cache);
            },
        );
        action
    };
    frame(&mut state, vec![]);
    frame(&mut state, vec![egui::Event::Text("lu".into())]);
    assert_eq!(state.window.as_ref().unwrap().chosen, Some(0));
    state.set_tab(PersonnelTab::Imperial);
    assert_eq!(state.window.as_ref().unwrap().chosen, None);
    let press = |pressed| egui::Event::PointerButton {
        pos: display,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    frame(&mut state, vec![egui::Event::PointerMoved(display)]);
    frame(&mut state, vec![press(true)]);
    let opened = frame(&mut state, vec![press(false)]);
    assert_eq!(
        opened,
        Some(PersonnelFinderAction::OpenCharacter {
            character: luke,
            system: Some(yavin),
        })
    );

    // Enter (0x407, FUN_00464c20) opens +0x178 the same way.
    let mut state = PersonnelFinderState::default();
    state.open(CockpitFaction::Alliance);
    frame(&mut state, vec![]);
    frame(&mut state, vec![egui::Event::Text("lu".into())]);
    state.set_tab(PersonnelTab::Imperial);
    frame(&mut state, vec![]);
    assert_eq!(state.window.as_ref().unwrap().chosen, None);
    let enter = egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    };
    assert_eq!(
        frame(&mut state, vec![enter]),
        Some(PersonnelFinderAction::OpenCharacter {
            character: luke,
            system: Some(yavin),
        })
    );
}

#[test]
fn a_tab_keeps_the_lists_place_clamped_to_its_new_rows() {
    // FUN_00464e10 swaps the list's items (FUN_00608280) and lays it out
    // (FUN_0060a280 -> FUN_00609ae0), which clamps the first visible
    // pixel +0xc4 to the new range and never sets it to 0.
    let mut world = GameWorld::default();
    let mut systems = Vec::new();
    for index in 0..12 {
        let system = add_system(&mut world, &format!("S{index:02}"));
        systems.push(system);
        let mut character = default_character();
        character.name = format!("C{index:02}");
        character.is_alliance = true;
        character.current_system = Some(system);
        character.recruited = true;
        world.characters.insert(character);
    }
    let fog = FogState::new(Faction::Alliance);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut frame = |world: &GameWorld, state: &mut PersonnelFinderState| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(700.0, 520.0),
                )),
                ..Default::default()
            },
            |ctx| {
                let _ = draw_personnel_finder(ctx, world, &fog, state, layout(), &mut cache);
            },
        );
        state.window.as_ref().unwrap().scroll
    };
    // Twelve SpecForces rows of each side too (12 x 20 + 5 - 161 = 84
    // to scroll).
    for &system in &systems {
        add_sf(&mut world, system, 0x3c00_0002, true);
        add_sf(&mut world, system, 0x3c00_0006, false);
    }
    let mut state = PersonnelFinderState::default();
    state.open(CockpitFaction::Alliance);
    state.window.as_mut().unwrap().scroll = 30;
    assert_eq!(frame(&world, &mut state), 30);
    state.set_view(PersonnelView::SpecForces);
    assert_eq!(frame(&world, &mut state), 30, "kept by the view");
    state.set_tab(PersonnelTab::Imperial);
    assert_eq!(frame(&world, &mut state), 30, "kept by the side");
    // No Imperial character is listed: the range is 0.
    state.set_view(PersonnelView::Characters);
    assert_eq!(frame(&world, &mut state), 0, "clamped");
}

#[test]
fn the_finder_opens_with_the_characters_view_and_players_side_tab() {
    // FUN_004637a0: FUN_0060d7e0(+0x148, 3, 1), view 3, the Characters
    // button (TEXTSTRA 0x1899); FUN_0060d7e0(+0x144, side, 1), the
    // player's side.
    let mut state = PersonnelFinderState::default();
    state.open(CockpitFaction::Alliance);
    let window = state.window.as_ref().unwrap();
    assert_eq!(window.view, PersonnelView::Characters);
    assert_eq!(window.tab, PersonnelTab::Alliance);

    let mut state = PersonnelFinderState::default();
    state.open(CockpitFaction::Empire);
    let window = state.window.as_ref().unwrap();
    assert_eq!(window.view, PersonnelView::Characters);
    assert_eq!(window.tab, PersonnelTab::Imperial);
}

#[test]
fn escape_closes_the_finder() {
    let mut state = PersonnelFinderState::default();
    state.open(CockpitFaction::Alliance);
    let world = GameWorld::default();
    let fog = FogState::new(Faction::Alliance);
    let mut cache = BmpCache::new();
    let layout = layout();
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);

    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(700.0, 520.0),
        )),
        events: vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }],
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| {
        draw_personnel_finder(ctx, &world, &fog, &mut state, layout, &mut cache);
    });

    assert!(!state.is_open());
}

#[test]
fn the_title_and_label_take_the_originals_text_fonts_and_places() {
    // FUN_004637a0: the title centred in (36, 14, 350) in font 5, the
    // label TEXTSTRA 0x1891 "Name" in font 4 (A0 2026-10-29).
    let mut state = PersonnelFinderState::default();
    state.open(CockpitFaction::Alliance);
    let world = GameWorld::default();
    let fog = FogState::new(Faction::Alliance);
    let mut cache = BmpCache::new();
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut shapes = Vec::new();
    for _ in 0..2 {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 520.0))),
                ..Default::default()
            },
            |ctx| {
                let _ = draw_personnel_finder(ctx, &world, &fog, &mut state, layout(), &mut cache);
            },
        );
        shapes = output.shapes;
    }
    let lines = crate::theme::test_lines(&shapes);
    let find = |wanted: &str| {
        lines
            .iter()
            .find(|line| line.text == wanted)
            .map(|line| (line.pen, line.size))
    };
    // DrawTextA from each box's top, the baselines tmAscent 13 below.
    let origin = window_rect(layout()).min;
    let (_, width) = crate::theme::gdi_pen_positions(5, "Personnel Finder").unwrap();
    let (pen, font) = find("Personnel Finder").unwrap();
    assert_eq!(
        pen,
        origin + egui::vec2(36.0 + ((350.0 - width) / 2.0).floor(), 14.0 + 13.0)
    );
    assert_eq!(font, crate::theme::gdi_font(5, 1.0).size);
    let (pen, font) = find("Name").unwrap();
    assert_eq!(pen, origin + egui::vec2(36.0, 48.0 + 13.0));
    assert_eq!(font, crate::theme::gdi_font(4, 1.0).size);
    assert!(find("Character Name").is_none());
}

#[test]
fn rows_sit_at_x_five_centred_grey_and_the_chosen_one_white_at_any_scale() {
    // FUN_004637a0: +0xe8 = 5, +0xe4 = 0x24; FUN_00607ea0's colours.
    // At scale 2: Luke's row and, in the special forces view, Yavin's.
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");
    let mut luke = default_character();
    luke.name = "Luke Skywalker".into();
    luke.is_alliance = true;
    luke.current_system = Some(yavin);
    luke.recruited = true;
    world.characters.insert(luke);
    add_sf(&mut world, yavin, 0x3c00_0001, true);
    let fog = FogState::new(Faction::Alliance);
    let layout = CockpitLayout {
        scale: 2.0,
        canvas: crate::cockpit::CockpitViewport {
            x: 0.0,
            y: 0.0,
            width: 1280.0,
            height: 960.0,
        },
        galaxy: crate::cockpit::CockpitViewport {
            x: 130.0,
            y: 120.0,
            width: 970.0,
            height: 700.0,
        },
    };
    let origin = window_rect(layout).min;
    let mut state = PersonnelFinderState::default();
    state.open(CockpitFaction::Alliance);
    let row = |state: &mut PersonnelFinderState| {
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let mut shapes = Vec::new();
        for _ in 0..2 {
            shapes = ctx
                .run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1400.0, 1100.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        let _ = draw_personnel_finder(ctx, &world, &fog, state, layout, &mut cache);
                    },
                )
                .shapes;
        }
        crate::theme::test_lines(&shapes)
            .into_iter()
            .find(|line| (line.pen.x - origin.x - (LIST.0 + 5.0) * 2.0).abs() < 0.01)
            .map(|line| (line.text, line.pen.y - origin.y, line.color))
            .unwrap()
    };
    // DT_VCENTER in the 20-pixel row: 3 down, then tmAscent 11.
    let (text, baseline, colour) = row(&mut state);
    assert!(text.starts_with("Luke Skywalker"), "{text}");
    assert!(
        (baseline - (LIST.1 + 14.0) * 2.0).abs() < 0.01,
        "{baseline}"
    );
    assert_eq!(colour, ROW_COLOR);
    state.window.as_mut().unwrap().chosen = Some(0);
    assert_eq!(row(&mut state).2, egui::Color32::WHITE);
    let window = state.window.as_mut().unwrap();
    window.view = PersonnelView::SpecForces;
    window.chosen = None;
    let (text, baseline, colour) = row(&mut state);
    assert!(text.starts_with("Yavin"), "{text}");
    assert!(
        (baseline - (LIST.1 + 14.0) * 2.0).abs() < 0.01,
        "{baseline}"
    );
    assert_eq!(colour, ROW_COLOR);
}

#[test]
fn the_finder_blocks_only_its_own_rectangle() {
    let mut state = PersonnelFinderState::default();
    let rect = window_rect(layout());
    assert!(!state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));

    state.open(CockpitFaction::Alliance);
    assert!(state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));
    assert!(!state.contains_screen_point(layout(), (rect.max.x + 1.0, rect.center().y)));
}

#[test]
fn count_sf_tallies_by_class_for_the_tabs_side() {
    // FUN_00465540: four columns, each the count of its DatId.
    let mut world = GameWorld::default();
    let system = add_system(&mut world, "Yavin");
    let k1 = add_sf(&mut world, system, 0x3c00_0004, true); // col 0
    let k2 = add_sf(&mut world, system, 0x3c00_0004, true); // col 0
    let k3 = add_sf(&mut world, system, 0x3c00_0003, true); // col 3
    let _k4 = add_sf(&mut world, system, 0x3c00_0007, false); // empire, ignored

    let keys = vec![k1, k2, k3];
    let counts = count_sf(&world, &keys, PersonnelTab::Alliance);
    assert_eq!(counts[0].count, 2);
    assert_eq!(counts[1].count, 0);
    assert_eq!(counts[3].count, 1);
    assert!(counts[0].all_usable);
}

#[test]
fn a_column_with_a_unit_away_is_drawn_in_the_sides_dark_brush() {
    // FUN_00465540 flag 4 (+0x50 bit 0 clear); A0 specforces/01 draws
    // the boxes of units en route or on a mission dark red.
    let mut world = GameWorld::default();
    let system = add_system(&mut world, "Tralus");
    let away = add_sf(&mut world, system, 0x3c00_0003, true);
    let here = add_sf(&mut world, system, 0x3c00_0004, true);
    world.special_forces[away].on_mission = true;
    let counts = count_sf(&world, &[away, here], PersonnelTab::Alliance);
    assert!(!counts[3].all_usable);
    assert!(counts[0].all_usable);
    assert_eq!(
        sf_cell_colours(PersonnelTab::Alliance, false).0,
        egui::Color32::from_rgb(128, 0, 0)
    );
    assert_eq!(
        sf_cell_colours(PersonnelTab::Imperial, false).0,
        egui::Color32::from_rgb(0, 128, 0)
    );
    assert_eq!(
        sf_cell_colours(PersonnelTab::Alliance, true).0,
        egui::Color32::from_rgb(255, 0, 0)
    );
}

#[test]
fn each_view_has_its_own_panel_and_button() {
    // FUN_004637a0 / FUN_00464e10: view 3, Characters, shows 10536 or
    // 10537 and its face button 10542 sits at the top; view 2,
    // SpecForces, shows 10538 or 10539 (the column icons) and its
    // soldiers button 10546 sits 0x36 lower.
    let characters = PersonnelView::Characters.index();
    let specforces = PersonnelView::SpecForces.index();
    assert_eq!(PANELS[characters], [10536, 10537]);
    assert_eq!(PANELS[specforces], [10538, 10539]);
    assert_eq!(VIEW_TAB_ART[0][characters], (10542, 10543));
    assert_eq!(VIEW_TAB_ART[1][specforces], (10548, 10549));
    assert!(PersonnelView::Characters.button_offset().abs() < f32::EPSILON);
    assert!((PersonnelView::SpecForces.button_offset() - 54.0).abs() < f32::EPSILON);
}

#[test]
fn the_tab_label_y_differs_by_view() {
    // FUN_00464e10: view 2, SpecForces, y = 0x77 = 119; else 0x73 = 115.
    assert!((PersonnelView::SpecForces.label_y() - 119.0).abs() < f32::EPSILON);
    assert!((PersonnelView::Characters.label_y() - 115.0).abs() < f32::EPSILON);
}
