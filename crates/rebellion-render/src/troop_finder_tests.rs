use super::*;

#[test]
fn a_tab_keeps_the_lists_place_clamped_then_scrolls_to_its_match() {
    // FUN_0046ea10 empties and refills the list without writing its
    // first visible pixel; FUN_0060a280 -> FUN_00609ae0 clamps it, and
    // FUN_0046df90 case 100's FUN_00609650 scrolls only as far as its row
    // needs (FUN_00609ed0).
    let mut state = TroopFinderState::default();
    state.open(CockpitFaction::Alliance);
    state.window.as_mut().unwrap().scroll = 30;
    state.set_tab(TroopTab::Imperial);
    assert_eq!(state.window.as_ref().unwrap().scroll, 30);
    let max = max_scroll(20);
    assert!(max > 30);
    assert_eq!(rebuilt_scroll(30, None, 20), 30);
    assert_eq!(rebuilt_scroll(30, Some(3), 20), 30);
    assert_eq!(rebuilt_scroll(30, Some(0), 20), 0);
    assert_eq!(rebuilt_scroll(10_000, None, 20), max);
    assert_eq!(rebuilt_scroll(10_000, Some(19), 20), max);
    assert_eq!(rebuilt_scroll(30, None, 2), 0);
}

#[test]
fn each_tab_has_its_tooltip_string() {
    // FUN_0046ce40: 0x1950 "Alliance Troops", 0x1951 "Imperial Troops".
    assert_eq!(
        TroopTab::ALL.map(TroopTab::tooltip_string),
        [0x1950, 0x1951]
    );
}
use crate::cockpit::CockpitViewport;
use rebellion_core::dat::{ExplorationStatus, Faction, SectorGroup};
use rebellion_core::ids::{DatId, FleetKey, SystemKey};
use rebellion_core::world::{
    CapitalShipClass, ControlKind, Fleet, Sector, ShipInstance, System, TroopUnit,
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

fn add_system(world: &mut GameWorld, name: &str, control: ControlKind) -> SystemKey {
    let sector = world.sectors.insert(Sector {
        dat_id: DatId::new(36),
        name: "Sesswenna".into(),
        group: SectorGroup::Core,
        x: 317,
        y: 248,
        systems: Vec::new(),
    });
    // Each system its own id, as SYSTEMSD's are: the player's copies
    // name their system by it.
    let id = 100 + u32::try_from(world.systems.len()).unwrap_or_default();
    world.systems.insert(System {
        dat_id: DatId::new(id),
        name: name.into(),
        sector,
        x: 320,
        y: 250,
        exploration_status: ExplorationStatus::Explored,
        popularity_alliance: 0.5,
        popularity_empire: 0.5,
        is_populated: true,
        control,
        ..Default::default()
    })
}

fn add_troop(
    world: &mut GameWorld,
    system: SystemKey,
    class_raw: u32,
    is_alliance: bool,
) -> TroopKey {
    let key = world.troops.insert(TroopUnit {
        class_dat_id: DatId::new(class_raw),
        is_alliance,
        regiment_strength: 100,
    });
    world.systems[system].ground_units.push(key);
    key
}

fn add_fleet(world: &mut GameWorld, system: SystemKey, is_alliance: bool) -> FleetKey {
    let class = world.capital_ship_classes.insert(CapitalShipClass {
        name: "Transport".into(),
        is_alliance,
        hull: 100,
        hyperdrive: 1,
        troop_capacity: 4,
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

fn names(rows: &[TroopFinderRow]) -> Vec<&str> {
    rows.iter().map(|row| row.name.as_str()).collect()
}

struct Galaxy {
    world: GameWorld,
}

fn galaxy() -> Galaxy {
    let mut world = GameWorld::default();
    let yavin = add_system(
        &mut world,
        "Yavin",
        ControlKind::Controlled(Faction::Alliance),
    );
    // Two Alliance army regiments on Yavin.
    add_troop(&mut world, yavin, 0x1000_0002, true);
    add_troop(&mut world, yavin, 0x1000_0002, true);
    // One Alliance fleet regiment.
    add_troop(&mut world, yavin, 0x1000_0001, true);
    let coruscant = add_system(
        &mut world,
        "Coruscant",
        ControlKind::Controlled(Faction::Empire),
    );
    // One Empire stormtrooper regiment on Coruscant.
    add_troop(&mut world, coruscant, 0x1000_0008, false);
    Galaxy { world }
}

#[test]
fn the_troop_finder_lists_systems_holding_the_tabs_regiments() {
    // FUN_0046ea10: walk systems, check FUN_00504cc0 for regiments
    // of the tab's side.
    let galaxy = galaxy();
    let transport = TroopTransportState::default();

    let fog = FogState::new(Faction::Alliance);
    let alliance = rows(
        &galaxy.world,
        &fog,
        Faction::Alliance,
        TroopTab::Alliance,
        &transport,
    );
    assert_eq!(names(&alliance), ["Yavin"]);
    assert_eq!(alliance[0].counts[0].count, 2, "Army Regiment column");
    assert_eq!(alliance[0].counts[1].count, 1, "Fleet Regiment column");
    assert_eq!(alliance[0].counts[2].count, 0);

    // A0 troop-other-side/test-day220-imperial-tab-empty: the other
    // side's regiments list only where the player sees them.
    let imperial = rows(
        &galaxy.world,
        &fog,
        Faction::Alliance,
        TroopTab::Imperial,
        &transport,
    );
    assert!(imperial.is_empty());
    let mut fog = FogState::new(Faction::Alliance);
    let coruscant = galaxy
        .world
        .systems
        .iter()
        .find(|(_, system)| system.name == "Coruscant")
        .map(|(key, _)| key)
        .unwrap();
    fog.reveal(coruscant);
    // The fog alone holds no copy: the tab lists the player's copies
    // (FUN_004f2d10, FUN_004f9a60).
    assert!(rows(&galaxy.world, &fog, Faction::Alliance, TroopTab::Imperial, &transport)
        .is_empty());
    let mut world = galaxy.world.clone();
    rebellion_core::object_views::seed_from_fog(
        &mut world,
        &transport,
        &rebellion_core::movement::MovementState::new(),
        [&fog, &FogState::new(Faction::Empire)],
    );
    let imperial = rows(
        &world,
        &fog,
        Faction::Alliance,
        TroopTab::Imperial,
        &transport,
    );
    assert_eq!(names(&imperial), ["Coruscant"]);
    assert_eq!(imperial[0].counts[0].count, 1, "Stormtrooper column");
    // The Empire sees its own regiments wherever they are.
    let own = rows(
        &galaxy.world,
        &FogState::new(Faction::Empire),
        Faction::Empire,
        TroopTab::Imperial,
        &transport,
    );
    assert_eq!(names(&own), ["Coruscant"]);
}

#[test]
fn a_fleet_carrying_regiments_is_listed_by_its_fleet_name() {
    // FUN_0046ea10: for each fleet at the system of the tab's side,
    // check FUN_00504c40 for regiments.
    let mut galaxy = galaxy();
    let yavin = galaxy.world.systems.iter().next().unwrap().0;
    let fleet = add_fleet(&mut galaxy.world, yavin, true);
    let troop = galaxy.world.troops.insert(TroopUnit {
        class_dat_id: DatId::new(0x1000_0002),
        is_alliance: true,
        regiment_strength: 100,
    });
    galaxy.world.systems[yavin].ground_units.push(troop);
    let mut transport = TroopTransportState::default();
    transport
        .embark(&mut galaxy.world, fleet, &[troop])
        .unwrap();

    let fog = FogState::new(Faction::Alliance);
    let listed = rows(
        &galaxy.world,
        &fog,
        Faction::Alliance,
        TroopTab::Alliance,
        &transport,
    );

    // Yavin (ground) and the fleet.
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().any(|r| r.fleet == Some(fleet)));
    let fleet_row = listed.iter().find(|r| r.fleet.is_some()).unwrap();
    assert_eq!(fleet_row.counts[0].count, 1);
}

#[test]
fn rows_are_sorted_by_name_ignoring_case() {
    // FUN_005f59f0: rows inserted in name order.
    let mut world = GameWorld::default();
    let beta = add_system(
        &mut world,
        "beta",
        ControlKind::Controlled(Faction::Alliance),
    );
    let alpha = add_system(
        &mut world,
        "Alpha",
        ControlKind::Controlled(Faction::Alliance),
    );
    add_troop(&mut world, beta, 0x1000_0002, true);
    add_troop(&mut world, alpha, 0x1000_0002, true);
    let transport = TroopTransportState::default();

    let fog = FogState::new(Faction::Alliance);
    let listed = rows(
        &world,
        &fog,
        Faction::Alliance,
        TroopTab::Alliance,
        &transport,
    );
    assert_eq!(names(&listed), ["Alpha", "beta"]);
}

#[test]
fn a_typed_name_chooses_the_first_row_it_best_begins() {
    // FUN_00609650: the same prefix-match the Fleet Finder uses.
    let rows = vec![
        TroopFinderRow {
            name: "Coruscant".into(),
            system: SystemKey::default(),
            fleet: None,
            counts: [TroopCount::default(); 5],
        },
        TroopFinderRow {
            name: "Yavin".into(),
            system: SystemKey::default(),
            fleet: None,
            counts: [TroopCount::default(); 5],
        },
    ];

    assert_eq!(prefix_choice(&rows, "ya"), Some(1));
    assert_eq!(prefix_choice(&rows, "cor"), Some(0));
    // FUN_00609650: with no row sharing the first letter, the first.
    assert_eq!(prefix_choice(&rows, "z"), Some(0));
    assert_eq!(prefix_choice(&rows, ""), Some(0));
}

#[test]
fn the_action_for_a_system_row_opens_defenses_and_a_fleet_row_opens_fleet() {
    // FUN_00429440: system → kind 10 (defenses), fleet → kind 4.
    let system = SystemKey::default();
    let fleet = FleetKey::default();

    let system_row = TroopFinderRow {
        name: "Yavin".into(),
        system,
        fleet: None,
        counts: [TroopCount::default(); 5],
    };
    assert_eq!(
        action_for_row(&system_row),
        TroopFinderAction::OpenDefenses { system }
    );

    let fleet_row = TroopFinderRow {
        name: "Fleet 1".into(),
        system,
        fleet: Some(fleet),
        counts: [TroopCount::default(); 5],
    };
    assert_eq!(
        action_for_row(&fleet_row),
        TroopFinderAction::OpenFleet { system, fleet }
    );
}

#[test]
fn six_whole_rows_fit_the_list() {
    // FUN_00607ea0: list 159 tall, rows 25 tall.
    assert_eq!(LIST_PIXELS / ROW_PIXELS, 6);
}

#[test]
fn the_list_scrolls_as_little_as_it_must_to_show_a_row() {
    // By pixels (FUN_0060a490): row 6 (150..175) needs 16; twenty rows
    // reach 500 + 5 - 159 = 346 (FUN_00609ae0).
    assert_eq!(scrolled_to(0, 5, 20), 0);
    assert_eq!(scrolled_to(0, 6, 20), 16);
    assert_eq!(scrolled_to(75, 1, 20), 25);
    assert_eq!(scrolled_to(75, 8, 20), 75);
    assert_eq!(scrolled_to(50, 8, 20), 66);
    assert_eq!(max_scroll(20), 346);
    assert_eq!(max_scroll(6), 0);
}

#[test]
fn opening_an_open_finder_keeps_its_state() {
    // FUN_0042a4d0 builds the window only when no type 0x16 is open.
    let mut state = TroopFinderState::default();
    assert!(state.open(CockpitFaction::Alliance));
    state.set_tab(TroopTab::Imperial);

    assert!(!state.open(CockpitFaction::Alliance));

    assert_eq!(state.window.as_ref().unwrap().tab, TroopTab::Imperial);
    assert!(state.is_open());
    state.close();
    assert!(!state.is_open());
}

#[test]
fn the_longest_common_beginning_wins_and_a_tie_keeps_the_first() {
    // FUN_00609650 keeps a row only for a strictly longer match.
    let rows: Vec<TroopFinderRow> = ["Corulag", "Coruscant", "Yavin"]
        .into_iter()
        .map(|name| TroopFinderRow {
            name: name.into(),
            system: SystemKey::default(),
            fleet: None,
            counts: [TroopCount::default(); 5],
        })
        .collect();
    assert_eq!(prefix_choice(&rows, "corus"), Some(1));
    assert_eq!(prefix_choice(&rows, "cor"), Some(0));
}

#[test]
fn each_tab_names_its_side_and_paints_its_own_cells() {
    // TEXTSTRA 0x1950/0x1951; panels 10540/10541 (FUN_006075e0);
    // FUN_0046d8d0's brushes: red with white text for the Alliance,
    // green with black for the Empire.
    assert_eq!(TroopTab::Alliance.label(), "Alliance Troops");
    assert_eq!(TroopTab::Imperial.label(), "Imperial Troops");
    assert_eq!(PANEL[TroopTab::Alliance.panel_index()], 10540);
    assert_eq!(PANEL[TroopTab::Imperial.panel_index()], 10541);
    assert_eq!(
        cell_colours(TroopTab::Alliance),
        (egui::Color32::from_rgb(255, 0, 0), egui::Color32::WHITE)
    );
    assert_eq!(
        cell_colours(TroopTab::Imperial),
        (egui::Color32::from_rgb(0, 255, 0), egui::Color32::BLACK)
    );
}

#[test]
fn a_new_tab_clears_the_choice() {
    // FUN_0046ea10: rebuilding clears the chosen row.
    let mut state = TroopFinderState::default();
    state.open(CockpitFaction::Alliance);
    state.window.as_mut().unwrap().chosen = Some(0);

    state.set_tab(TroopTab::Imperial);

    assert_eq!(state.window.as_ref().unwrap().chosen, None);
}

#[test]
fn an_edit_that_keeps_the_chosen_row_leaves_the_list_where_it_is() {
    // FUN_00609650 chooses and scrolls only to a row not chosen
    // already (+0x3c bit 0, 0x60973f): "s" then "s0" both choose S00.
    let mut world = GameWorld::default();
    for index in 0..12 {
        let system = add_system(
            &mut world,
            &format!("S{index:02}"),
            ControlKind::Controlled(Faction::Alliance),
        );
        add_troop(&mut world, system, 0x1000_0002, true);
    }
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let transport = TroopTransportState::default();
    let fog = FogState::new(Faction::Alliance);
    let mut state = TroopFinderState::default();
    state.open(CockpitFaction::Alliance);
    let mut frame = |state: &mut TroopFinderState, events: Vec<egui::Event>| {
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
                let _ = draw_troop_finder(
                    ctx,
                    &world,
                    &fog,
                    state,
                    layout(),
                    &mut cache,
                    &transport,
                );
            },
        );
    };
    frame(&mut state, vec![]);
    frame(&mut state, vec![egui::Event::Text("s".into())]);
    assert_eq!(state.window.as_ref().unwrap().chosen, Some(0));
    assert!(max_scroll(12) >= 40);
    state.window.as_mut().unwrap().scroll = 40;
    frame(&mut state, vec![egui::Event::Text("0".into())]);
    let window = state.window.as_ref().unwrap();
    assert_eq!((window.name.as_str(), window.chosen), ("s0", Some(0)));
    assert_eq!(window.scroll, 40);
}

#[test]
fn display_after_a_tab_with_no_rows_opens_the_row_chosen_before() {
    // FUN_0046df90 case 100 stores FUN_00609650's row in +0x17c only
    // when it finds one, and FUN_0046ea10's rebuild leaves +0x17c, so
    // an empty tab keeps Alpha; Display (case 0xc9) opens it.
    let mut world = GameWorld::default();
    let alpha = add_system(
        &mut world,
        "Alpha",
        ControlKind::Controlled(Faction::Alliance),
    );
    add_troop(&mut world, alpha, 0x1000_0002, true);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let transport = TroopTransportState::default();
    let fog = FogState::new(Faction::Alliance);
    let mut state = TroopFinderState::default();
    state.open(CockpitFaction::Alliance);
    let mut frame = |state: &mut TroopFinderState, events: Vec<egui::Event>| {
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
                action = draw_troop_finder(
                    ctx,
                    &world,
                    &fog,
                    state,
                    layout(),
                    &mut cache,
                    &transport,
                );
            },
        );
        action
    };
    frame(&mut state, vec![]);
    assert_eq!(state.window.as_ref().unwrap().chosen, Some(0));
    state.set_tab(TroopTab::Imperial);
    frame(&mut state, vec![]);
    assert_eq!(
        state.window.as_ref().unwrap().chosen,
        None,
        "no Imperial row"
    );
    let display = window_rect(layout()).min + egui::vec2(427.0 + 16.0, 93.0 + 15.0);
    let press = |pressed| egui::Event::PointerButton {
        pos: display,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    frame(&mut state, vec![egui::Event::PointerMoved(display)]);
    frame(&mut state, vec![press(true)]);
    assert_eq!(
        frame(&mut state, vec![press(false)]),
        Some(TroopFinderAction::OpenDefenses { system: alpha })
    );
}

#[test]
fn the_opening_and_a_new_tab_choose_the_row_the_name_begins_or_the_first() {
    // FUN_0046ce40 opens with FUN_0060d7e0(side, 1), whose notification
    // (FUN_0046df90 case 100) rebuilds the list and lets the name box's
    // text choose a row (FUN_00609650): the A0 Troop Finder opens with
    // its first row white under an empty name box, on either tab (VM,
    // 2026-10-29).
    let mut world = GameWorld::default();
    for name in ["Alpha", "Beta"] {
        let system = add_system(&mut world, name, ControlKind::Controlled(Faction::Alliance));
        add_troop(&mut world, system, 0x1000_0002, true);
    }
    // An Imperial regiment where the Alliance sees it: on a planet the
    // Alliance holds.
    let coruscant = add_system(
        &mut world,
        "Coruscant",
        ControlKind::Controlled(Faction::Alliance),
    );
    add_troop(&mut world, coruscant, 0x1000_0008, false);
    // The day's step gives the Alliance its live copy (FUN_004f9a60).
    rebellion_core::object_views::advance(
        &mut world,
        &TroopTransportState::default(),
        &rebellion_core::movement::MovementState::new(),
    );
    let mut state = TroopFinderState::default();
    state.open(CockpitFaction::Alliance);
    assert_eq!(state.window.as_ref().unwrap().chosen, None);

    texts(&world, &mut state);
    assert_eq!(state.window.as_ref().unwrap().chosen, Some(0));

    state.window.as_mut().unwrap().name = "be".into();
    state.set_tab(TroopTab::Imperial);
    texts(&world, &mut state);
    assert_eq!(
        state.window.as_ref().unwrap().chosen,
        Some(0),
        "no row begins with \"be\""
    );
    state.set_tab(TroopTab::Alliance);
    texts(&world, &mut state);
    assert_eq!(state.window.as_ref().unwrap().chosen, Some(1), "Beta");
    assert_eq!(state.window.as_ref().unwrap().name, "be");
}

#[test]
fn escape_closes_the_finder() {
    let mut state = TroopFinderState::default();
    state.open(CockpitFaction::Alliance);
    let galaxy = galaxy();
    let transport = TroopTransportState::default();
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let layout = layout();

    // Run a frame with escape.
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
        draw_troop_finder(
            ctx,
            &galaxy.world,
            &FogState::new(Faction::Alliance),
            &mut state,
            layout,
            &mut cache,
            &transport,
        );
    });

    assert!(!state.is_open());
}

/// Two frames of the Finder; the last frame's texts as (text, top-left,
/// size, font size, colour).
fn texts(world: &GameWorld, state: &mut TroopFinderState) -> Vec<(String, egui::Pos2, egui::Vec2, f32, egui::Color32)> {
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let transport = TroopTransportState::default();
    let mut shapes = Vec::new();
    for _ in 0..2 {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 520.0))),
                ..Default::default()
            },
            |ctx| {
                let _ = draw_troop_finder(
                    ctx,
                    world,
                    &FogState::new(Faction::Alliance),
                    state,
                    layout(),
                    &mut cache,
                    &transport,
                );
            },
        );
        shapes = output.shapes;
    }
    // Each line's text, pen and baseline, width, font size and colour.
    crate::theme::test_lines(&shapes)
        .into_iter()
        .map(|line| {
            (
                line.text,
                line.pen,
                egui::vec2(line.width, 0.0),
                line.size,
                line.color,
            )
        })
        .collect()
}

/// Whether `shapes` hold a line reading `name`.
fn named(shapes: &[egui::epaint::ClippedShape], name: &str) -> bool {
    crate::theme::test_lines(shapes)
        .iter()
        .any(|line| line.text == name)
}

/// The pen and baseline of the line reading `name` in `shapes`.
fn pen(shapes: &[egui::epaint::ClippedShape], name: &str) -> Option<egui::Pos2> {
    crate::theme::test_lines(shapes)
        .into_iter()
        .find(|line| line.text == name)
        .map(|line| line.pen)
}

#[test]
fn the_title_labels_and_rows_take_the_originals_fonts_and_places() {
    // FUN_0046ce40: the title centred in (36, 14, 350) in font 5, the
    // label in font 4; the list's text at x 5 from the row's top
    // (+0xe4 = 0x20), grey 0x787878 and the chosen row white (A0
    // 2026-10-29).
    let galaxy = galaxy();
    let mut state = TroopFinderState::default();
    state.open(CockpitFaction::Alliance);
    state.set_tab(TroopTab::Alliance);
    let origin = window_rect(layout()).min;
    let shown = texts(&galaxy.world, &mut state);
    let find = |name: &str| shown.iter().find(|(text, ..)| text == name).cloned().unwrap();
    // DrawTextA from each box's top: the title's spare width halved and
    // its baseline tmAscent 13 down; a row's text from the row's top
    // (format 0x20), tmAscent 11 down.
    let (_, width) = crate::theme::gdi_pen_positions(5, "Troop Finder").unwrap();
    let (_, pos, _, font, _) = find("Troop Finder");
    assert_eq!(
        pos,
        origin + egui::vec2(36.0 + ((350.0 - width) / 2.0).floor(), 14.0 + 13.0)
    );
    assert_eq!(font, crate::theme::gdi_font(5, 1.0).size);
    assert_eq!(
        find("Troop Location").3,
        crate::theme::gdi_font(4, 1.0).size
    );
    let (_, pos, _, font, colour) = find("Yavin");
    assert_eq!(pos, origin + egui::vec2(LIST.0 + 5.0, LIST.1 + 11.0));
    assert_eq!(font, crate::theme::gdi_font(10, 1.0).size);
    // The opening chooses the first row (FUN_0046df90 case 100).
    assert_eq!(colour, egui::Color32::WHITE);
    state.window.as_mut().unwrap().chosen = None;
    let shown = texts(&galaxy.world, &mut state);
    assert_eq!(shown.iter().find(|(text, ..)| text == "Yavin").unwrap().4, ROW_COLOR);
}

#[test]
fn the_bars_down_arrow_scrolls_the_list_ten_pixels_and_moves_its_rows() {
    // FUN_0046ce40's bar (RCDATA 10653) at the list's right, x 374;
    // its down arrow at y 294..303 steps the list's 10. Eight systems
    // of 25-pixel rows (200) overflow the 159-pixel list.
    let mut world = GameWorld::default();
    for index in 0..8 {
        let system = add_system(&mut world, &format!("S{index}"), ControlKind::Controlled(Faction::Alliance));
        add_troop(&mut world, system, 0x1000_0002, true);
    }
    let mut state = TroopFinderState::default();
    state.open(CockpitFaction::Alliance);
    state.set_tab(TroopTab::Alliance);
    let origin = window_rect(layout()).min;
    let arrow = origin + egui::vec2(380.0, 299.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let transport = TroopTransportState::default();
    let mut shapes = Vec::new();
    for pressed in [None, None, Some(true), Some(false), None] {
        crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
        let mut events = vec![egui::Event::PointerMoved(arrow)];
        if let Some(pressed) = pressed {
            events.push(egui::Event::PointerButton {
                pos: arrow,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::default(),
            });
        }
        shapes = ctx
            .run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 520.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    let _ = draw_troop_finder(
                        ctx,
                        &world,
                        &FogState::new(Faction::Alliance),
                        &mut state,
                        layout(),
                        &mut cache,
                        &transport,
                    );
                },
            )
            .shapes;
    }
    assert_eq!(state.window.as_ref().unwrap().scroll, 10);
    let first = pen(&shapes, "S0").unwrap();
    assert_eq!(
        first,
        origin + egui::vec2(LIST.0 + 5.0, LIST.1 - 10.0 + 11.0)
    );
    // Row 6 (140..165) shows; row 7 (165..190) is below the list.
    assert!(named(&shapes, "S6"));
    assert!(!named(&shapes, "S7"));
    let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow().clone());
    assert!(painted.contains(&(10_685, origin + egui::vec2(374.0, 144.0))), "{painted:?}");
    assert!(painted.contains(&(10_687, origin + egui::vec2(374.0, 294.0))), "{painted:?}");

    // A press at x 370 chooses the row; a wheel notch moves a row
    // (port); held on the arrow, it draws pressed.
    let mut frame =
        |state: &mut TroopFinderState, at: egui::Pos2, event: Option<egui::Event>| {
            crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
            let mut events = vec![egui::Event::PointerMoved(at)];
            events.extend(event);
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
                    let _ = draw_troop_finder(
                        ctx,
                        &world,
                        &FogState::new(Faction::Alliance),
                        state,
                        layout(),
                        &mut cache,
                        &transport,
                    );
                },
            );
        };
    let press = |at: egui::Pos2, pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let near_bar = origin + egui::vec2(370.0, LIST.1 + 30.0);
    frame(&mut state, near_bar, Some(press(near_bar, true)));
    frame(&mut state, near_bar, Some(press(near_bar, false)));
    assert_eq!(state.window.as_ref().unwrap().chosen, Some(1));
    let wheel = |delta: f32| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, delta),
        modifiers: egui::Modifiers::default(),
    };
    frame(&mut state, near_bar, Some(wheel(-1.0)));
    assert_eq!(state.window.as_ref().unwrap().scroll, 35);
    frame(&mut state, near_bar, Some(wheel(1.0)));
    assert_eq!(state.window.as_ref().unwrap().scroll, 10);
    frame(&mut state, arrow, Some(press(arrow, true)));
    frame(&mut state, arrow, None);
    let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow().clone());
    assert!(painted.iter().any(|(id, _)| *id == 10_688), "{painted:?}");
    frame(&mut state, origin, None);
    let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow().clone());
    assert!(!painted.iter().any(|(id, _)| *id == 10_688), "{painted:?}");
    // A press on the bar's track, in the row's height, chooses no row.
    state.window.as_mut().unwrap().chosen = None;
    let track = origin + egui::vec2(380.0, LIST.1 + 120.0);
    frame(&mut state, track, Some(press(track, true)));
    frame(&mut state, track, Some(press(track, false)));
    assert_eq!(state.window.as_ref().unwrap().chosen, None);
    // Scrolled 30, row 0 (-30..-5) is wholly above the list: not drawn.
    state.window.as_mut().unwrap().scroll = 30;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 520.0))),
            ..Default::default()
        },
        |ctx| {
            let _ = draw_troop_finder(
                ctx,
                &world,
                &FogState::new(Faction::Alliance),
                &mut state,
                layout(),
                &mut cache,
                &transport,
            );
        },
    );
    assert!(!named(&output.shapes, "S0"));
}

#[test]
fn at_scale_two_the_row_text_keeps_its_window_place() {
    let galaxy = galaxy();
    let layout = CockpitLayout {
        scale: 2.0,
        canvas: CockpitViewport {
            x: 0.0,
            y: 0.0,
            width: 1280.0,
            height: 960.0,
        },
        galaxy: CockpitViewport {
            x: 130.0,
            y: 120.0,
            width: 970.0,
            height: 700.0,
        },
    };
    let mut state = TroopFinderState::default();
    state.open(CockpitFaction::Alliance);
    state.set_tab(TroopTab::Alliance);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let transport = TroopTransportState::default();
    let mut shapes = Vec::new();
    for _ in 0..2 {
        shapes = ctx
            .run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1100.0))),
                    ..Default::default()
                },
                |ctx| {
                    let _ = draw_troop_finder(
                        ctx,
                        &galaxy.world,
                        &FogState::new(Faction::Alliance),
                        &mut state,
                        layout,
                        &mut cache,
                        &transport,
                    );
                },
            )
            .shapes;
    }
    let yavin = pen(&shapes, "Yavin").unwrap();
    assert_eq!(
        yavin,
        window_rect(layout).min + egui::vec2(LIST.0 + 5.0, LIST.1 + 11.0) * 2.0
    );
    // The bar reads the pointer in window pixels: eight systems, a click
    // on the down arrow at scale 2 steps 10.
    let mut world = GameWorld::default();
    for index in 0..8 {
        let system = add_system(&mut world, &format!("S{index}"), ControlKind::Controlled(Faction::Alliance));
        add_troop(&mut world, system, 0x1000_0002, true);
    }
    let arrow = window_rect(layout).min + egui::vec2(380.0, 299.0) * 2.0;
    for pressed in [None, None, Some(true), Some(false)] {
        let mut events = vec![egui::Event::PointerMoved(arrow)];
        events.extend(pressed.map(|pressed| egui::Event::PointerButton {
            pos: arrow,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }));
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1100.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                let _ = draw_troop_finder(
                    ctx,
                    &world,
                    &FogState::new(Faction::Alliance),
                    &mut state,
                    layout,
                    &mut cache,
                    &transport,
                );
            },
        );
    }
    assert_eq!(state.window.as_ref().unwrap().scroll, 10);
}

#[test]
fn the_finder_blocks_only_its_own_rectangle() {
    let mut state = TroopFinderState::default();
    let rect = window_rect(layout());
    assert!(!state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));

    state.open(CockpitFaction::Alliance);

    assert!(state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));
    assert!(!state.contains_screen_point(layout(), (rect.max.x + 1.0, rect.center().y)));
}

#[test]
fn count_troops_tallies_by_class_for_the_tabs_side() {
    // FUN_0046d8d0: five columns, each the count of its DatId.
    let mut world = GameWorld::default();
    let system = add_system(
        &mut world,
        "Yavin",
        ControlKind::Controlled(Faction::Alliance),
    );
    let k1 = add_troop(&mut world, system, 0x1000_0002, true); // col 0
    let k2 = add_troop(&mut world, system, 0x1000_0002, true); // col 0
    let k3 = add_troop(&mut world, system, 0x1000_0004, true); // col 4
    let k4 = add_troop(&mut world, system, 0x1000_0008, false); // empire, ignored

    let keys = vec![k1, k2, k3, k4];
    let counts = count_troops(&world, &keys, TroopTab::Alliance);
    assert_eq!(counts[0].count, 2);
    assert_eq!(counts[1].count, 0);
    assert_eq!(counts[4].count, 1);
    // port: all usable.
    assert!(counts[0].all_usable);
}

#[test]
fn the_troop_finder_opens_on_the_players_side_tab() {
    // FUN_0046ce40: FUN_0060d7e0(+0x144, side, 1) opens the player's tab.
    let mut state = TroopFinderState::default();
    state.open(CockpitFaction::Alliance);
    assert_eq!(state.window.as_ref().unwrap().tab, TroopTab::Alliance);

    let mut state = TroopFinderState::default();
    state.open(CockpitFaction::Empire);
    assert_eq!(state.window.as_ref().unwrap().tab, TroopTab::Imperial);
}
