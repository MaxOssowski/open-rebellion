use super::*;

#[test]
fn a_tab_keeps_the_lists_place_clamped_then_scrolls_to_its_match() {
    // FUN_00462be0 empties and refills the list without writing its
    // first visible pixel; FUN_0060a280 -> FUN_00609ae0 clamps it, and
    // FUN_00462770 case 100's FUN_00609650 scrolls only as far as its row
    // needs (FUN_00609ed0).
    let mut state = FleetFinderState::default();
    state.open(CockpitFaction::Alliance);
    state.window.as_mut().unwrap().scroll = 30;
    state.set_tab(FinderTab::Imperial);
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
fn the_side_tabs_name_fleets_or_ships_by_mode() {
    // FUN_00461960 adds 0x1882..0x1884; FUN_00462be0 swaps in
    // 0x1887..0x1889 in Ship mode (A0 tooltip/41..43, 48, 49).
    let names = |mode| FinderTab::ALL.map(|tab| tab_tooltip_string(tab, mode));
    assert_eq!(names(FinderMode::Fleets), [0x1882, 0x1883, 0x1884]);
    assert_eq!(names(FinderMode::Ships), [0x1887, 0x1888, 0x1889]);
    assert_eq!(mode_tooltip_string(FinderMode::Ships), 0x1885);
    assert_eq!(mode_tooltip_string(FinderMode::Fleets), 0x1880);
}
use crate::cockpit::CockpitViewport;
use rebellion_core::dat::{ExplorationStatus, SectorGroup};
use rebellion_core::ids::{DatId, FleetKey, SystemKey};
use rebellion_core::world::{
    CapitalShipClass, ControlKind, Fleet, Sector, ShipInstance, System,
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
    world.systems.insert(System {
        // Each system its own id: the copies are keyed by it.
        dat_id: DatId::new(
            0x9000_0100 + u32::try_from(world.systems.len()).unwrap_or(0),
        ),
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

/// A fleet of one ship per class name, in orbit at `system`.
fn add_fleet(
    world: &mut GameWorld,
    system: SystemKey,
    is_alliance: bool,
    ships: &[&str],
) -> FleetKey {
    let capital_ships = ships
        .iter()
        .map(|name| {
            let class = world.capital_ship_classes.insert(CapitalShipClass {
                name: (*name).into(),
                is_alliance,
                hull: 100,
                hyperdrive: 1,
                ..CapitalShipClass::default()
            });
            ShipInstance::new(class, 100, true)
        })
        .collect();
    let fleet = world.insert_fleet(Fleet {
        location: system,
        capital_ships,
        is_alliance,
        ..Default::default()
    });
    world.systems[system].fleets.push(fleet);
    fleet
}

fn names(rows: &[FinderRow]) -> Vec<&str> {
    rows.iter().map(|row| row.name.as_str()).collect()
}

fn row(name: &str) -> FinderRow {
    FinderRow {
        object: FleetWindowEntry::Fleet(FleetKey::default()),
        name: name.into(),
    }
}

/// An Alliance system holding an Alliance fleet (Nebulon-B, corellian
/// Corvette), and an Imperial system the Alliance sees holding an
/// Imperial fleet (Victory Star Destroyer) and one it cannot see.
struct Galaxy {
    world: GameWorld,
    fog: FogState,
    own: FleetKey,
    seen: FleetKey,
    unseen: FleetKey,
}

fn galaxy() -> Galaxy {
    let mut world = GameWorld::default();
    let home = add_system(
        &mut world,
        "Yavin",
        ControlKind::Controlled(Faction::Alliance),
    );
    let seen_at = add_system(
        &mut world,
        "Sluis Van",
        ControlKind::Controlled(Faction::Empire),
    );
    let hidden = add_system(
        &mut world,
        "Coruscant",
        ControlKind::Controlled(Faction::Empire),
    );
    let own = add_fleet(&mut world, home, true, &["Nebulon-B", "corellian Corvette"]);
    let seen = add_fleet(&mut world, seen_at, false, &["Victory Star Destroyer"]);
    let unseen = add_fleet(&mut world, hidden, false, &["Imperial Star Destroyer"]);
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(seen_at);
    crate::viewed_contents::see_live(&mut world, Faction::Alliance, seen_at);
    Galaxy {
        world,
        fog,
        own,
        seen,
        unseen,
    }
}

#[test]
fn the_ship_finder_lists_living_ships_by_name_ignoring_case() {
    // FUN_00462be0 mode 2 walks capital ships; FUN_0060a890 mode 2
    // orders the rows with _stricmp (FUN_00626ad0).
    let mut galaxy = galaxy();
    galaxy.world.fleets[galaxy.own].capital_ships[0].alive = false;

    let rows = rows(
        &galaxy.world,
        &rebellion_core::movement::MovementState::default(),
        &galaxy.fog,
        Faction::Alliance,
        FinderMode::Ships,
        FinderTab::All,
    );

    assert_eq!(
        names(&rows),
        ["corellian Corvette", "Victory Star Destroyer"]
    );
    assert_eq!(
        rows[0].object,
        FleetWindowEntry::Ship {
            fleet: galaxy.own,
            index: 1
        }
    );
}

#[test]
fn the_fleet_finder_lists_own_fleets_and_the_other_sides_fleets_it_sees() {
    // hyp: FUN_0053ef50 walks the side's view of each object; the port
    // shows the other side where the Fleet window does.
    let galaxy = galaxy();

    let listed: Vec<_> = rows(
        &galaxy.world,
        &rebellion_core::movement::MovementState::default(),
        &galaxy.fog,
        Faction::Alliance,
        FinderMode::Fleets,
        FinderTab::All,
    )
    .into_iter()
    .map(|row| row.object)
    .collect();

    assert_eq!(
        listed,
        [
            FleetWindowEntry::Fleet(galaxy.own),
            FleetWindowEntry::Fleet(galaxy.seen)
        ]
    );
    assert!(!listed.contains(&FleetWindowEntry::Fleet(galaxy.unseen)));
}

#[test]
fn the_side_tabs_list_only_their_sides_objects() {
    // FUN_00462be0: tab 2 needs side bit 0x40, tab 3 bit 0x80.
    let galaxy = galaxy();
    let list = |tab| {
        names(&rows(
            &galaxy.world,
            &rebellion_core::movement::MovementState::default(),
            &galaxy.fog,
            Faction::Alliance,
            FinderMode::Ships,
            tab,
        ))
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>()
    };

    assert_eq!(
        list(FinderTab::Alliance),
        ["corellian Corvette", "Nebulon-B"]
    );
    assert_eq!(list(FinderTab::Imperial), ["Victory Star Destroyer"]);
}

#[test]
fn the_ship_finder_lists_an_unfinished_ship_by_its_name() {
    // A0 2026-11-15 "m-1-ship-finder-lists-unfinished-carrack-3": the
    // walk takes the fleet's ships in mode 3, the unfinished one too.
    use rebellion_core::manufacturing::{BuildableKind, ManufacturingState, QueueItem};
    let mut galaxy = galaxy();
    let home = galaxy.world.fleets[galaxy.own].location;
    galaxy.world.set_ship_spare(home, true, galaxy.own);
    let class = galaxy.world.fleets[galaxy.own].capital_ships[0].class;
    let mut manufacturing = ManufacturingState::new();
    manufacturing.enqueue(
        home,
        QueueItem::new(BuildableKind::CapitalShip(class), 5).ordered(&mut galaxy.world),
    );
    rebellion_core::products::reconcile(&mut galaxy.world, &manufacturing);
    let product = galaxy.world.products.all()[0].id;
    let name = galaxy.world.products.all()[0].name.clone().unwrap();
    let ships = rows(
        &galaxy.world,
        &rebellion_core::movement::MovementState::default(),
        &galaxy.fog,
        Faction::Alliance,
        FinderMode::Ships,
        FinderTab::Alliance,
    );
    assert!(ships.iter().any(|row| row.object
        == FleetWindowEntry::Product {
            fleet: galaxy.own,
            product
        }
        && row.name == name));
}

#[test]
fn a_fleet_made_by_create_fleet_is_listed_with_its_ship() {
    // FUN_005809c0 makes the fleet in the ship's system, where the
    // Finder's walk (FUN_00462be0) meets it.
    use rebellion_core::fleet_join::{create_fleet, roster};
    use rebellion_core::movement::MovementState;
    use rebellion_core::troop_transport::TroopTransportState;
    let mut galaxy = galaxy();
    let chosen = roster(&galaxy.world, galaxy.own).unwrap();
    let made = create_fleet(
        &mut galaxy.world,
        &MovementState::default(),
        &mut TroopTransportState::default(),
        galaxy.own,
        &[1],
        chosen,
        true,
    )
    .unwrap();
    let listed = |mode| {
        rows(
            &galaxy.world,
            &rebellion_core::movement::MovementState::default(),
            &galaxy.fog,
            Faction::Alliance,
            mode,
            FinderTab::Alliance,
        )
    };

    let fleets = listed(FinderMode::Fleets);
    let ships = listed(FinderMode::Ships);

    assert!(fleets
        .iter()
        .any(|row| row.object == FleetWindowEntry::Fleet(made) && row.name == "Fleet 2"));
    assert!(ships.iter().any(|row| row.object
        == FleetWindowEntry::Ship {
            fleet: made,
            index: 0
        }
        && row.name == "corellian Corvette"));
}

#[test]
fn a_fleet_in_hyperspace_is_judged_where_it_is_bound() {
    // move-order.md: a move puts the fleet in its destination at once,
    // so the Finder lists it as the destination's Fleet window does: an
    // unseen enemy fleet bound for a system the player sees is listed.
    let mut galaxy = galaxy();
    let seen_at = galaxy.world.fleets[galaxy.seen].location;
    let hidden = galaxy.world.fleets[galaxy.unseen].location;
    let listed = |galaxy: &Galaxy, movement: &MovementState| {
        rows(
            &galaxy.world,
            movement,
            &galaxy.fog,
            Faction::Alliance,
            FinderMode::Fleets,
            FinderTab::All,
        )
        .iter()
        .any(|row| row.object == FleetWindowEntry::Fleet(galaxy.unseen))
    };
    let mut movement = MovementState::default();
    assert!(!listed(&galaxy, &movement));
    assert!(movement.order(galaxy.unseen, hidden, seen_at, 5));
    galaxy.world.systems[hidden].fleets.clear();
    assert!(listed(&galaxy, &movement));
}

#[test]
fn a_copied_fleet_stays_while_the_sides_fleet_is_en_route_and_vanishes_on_its_arrival_day() {
    // FUN_004f9a60: the Alliance's copy of Yaga Minor stays frozen while
    // Fleet 3 is en route (+0x50 bit 4) and is rebuilt live from the
    // master on the day it arrives. A0 2026-11-14-stale-views second
    // run: Fleet 2 listed on day 90 (1x-yaga-replay-fleetwin-d90), gone
    // from the Yaga Minor Fleet window (1x-yaga-arrival-fleetwin-d91,
    // sha256 9a4c0b3a) and the Fleet Finder's Imperial Fleets
    // (1x-fleetfinder-empire-d91, e744caf2) on the ETA day 91.
    use rebellion_core::missions::MissionState;
    use rebellion_core::troop_transport::TroopTransportState;
    let mut world = GameWorld::default();
    let empire = ControlKind::Controlled(Faction::Empire);
    let yaga = add_system(&mut world, "Yaga Minor", empire);
    let ghorman = add_system(
        &mut world,
        "Ghorman",
        ControlKind::Controlled(Faction::Alliance),
    );
    world.systems[ghorman].dat_id = DatId::new(101);
    let kuat = add_system(&mut world, "Kuat", empire);
    world.systems[kuat].dat_id = DatId::new(102);
    let imperial = add_fleet(&mut world, yaga, false, &["Imperial Star Destroyer"]);
    let alliance = add_fleet(&mut world, yaga, true, &["Medium Transport"]);
    let mut movement = MovementState::new();
    let (missions, transport) = (MissionState::new(), TroopTransportState::new());
    let fog = FogState::new(Faction::Alliance);
    let day = |world: &mut GameWorld, movement: &MovementState| {
        rebellion_core::object_views::advance_extras(world, movement, &missions, &transport);
        rebellion_core::object_views::advance(world, &transport, movement);
    };
    let imperial_fleets = |world: &GameWorld, movement: &MovementState| {
        names(&rows(
            world,
            movement,
            &fog,
            Faction::Alliance,
            FinderMode::Fleets,
            FinderTab::Imperial,
        ))
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>()
    };
    let window = |world: &GameWorld| {
        crate::fleet_window::copy_entries(world, &fog, Faction::Alliance, yaga).len()
    };
    let fleet_2 = world.fleet_name(imperial).unwrap().to_owned();
    day(&mut world, &movement);
    // Fleet 3 withdraws; Fleet 2 then leaves for Kuat.
    world.fleets[alliance].location = ghorman;
    day(&mut world, &movement);
    world.fleets[imperial].location = kuat;
    day(&mut world, &movement);
    assert_eq!(imperial_fleets(&world, &movement), std::slice::from_ref(&fleet_2));
    assert_eq!(window(&world), 1);
    // Fleet 3 is ordered back; en route the day before its arrival.
    assert!(movement.order(alliance, ghorman, yaga, 6));
    world.fleets[alliance].location = yaga;
    day(&mut world, &movement);
    assert_eq!(imperial_fleets(&world, &movement), [fleet_2]);
    assert_eq!(window(&world), 1);
    // The arrival day ends the order (MovementSystem::advance).
    movement.cancel(alliance);
    day(&mut world, &movement);
    assert!(imperial_fleets(&world, &movement).is_empty());
    assert_eq!(window(&world), 0);
}

#[test]
fn a_typed_name_chooses_the_first_row_it_best_begins() {
    // FUN_00609650: the first row with the longest common prefix,
    // compared without case (FUN_005f3430 flag 0).
    let rows = [
        row("Calamari Cruiser"),
        row("Corellian Corvette"),
        row("Corellian Gunship"),
    ];

    assert_eq!(prefix_choice(&rows, "cor"), Some(1));
    assert_eq!(prefix_choice(&rows, "CORELLIAN G"), Some(2));
    assert_eq!(prefix_choice(&rows, "cx"), Some(0));
    assert_eq!(prefix_choice(&rows, "Coz"), Some(1));
}

#[test]
fn a_name_no_row_begins_with_chooses_the_first_row() {
    // FUN_00609650 starts its best row at the first (0x609676); the A0
    // Troop Finder opens with its first row chosen under an empty name
    // box (VM, 2026-10-29).
    let rows = [row("Calamari Cruiser"), row("Corellian Corvette")];

    assert_eq!(prefix_choice(&rows, "Nebulon"), Some(0));
    assert_eq!(prefix_choice(&rows, ""), Some(0));
    assert_eq!(prefix_choice(&[], ""), None);
}

#[test]
fn the_list_scrolls_as_little_as_it_must_to_show_a_row() {
    // By pixels in the 165-pixel list (FUN_0060a490): row 8 (160..180)
    // needs 15; twenty rows reach 400 + 5 - 165 = 240 (FUN_00609ae0).
    assert_eq!(scrolled_to(0, 7, 20), 0);
    assert_eq!(scrolled_to(0, 8, 20), 15);
    assert_eq!(scrolled_to(100, 2, 20), 40);
    assert_eq!(scrolled_to(100, 12, 20), 100);
    assert_eq!(scrolled_to(0, 19, 20), 235);
    assert_eq!(max_scroll(20), 240);
    assert_eq!(max_scroll(8), 0);
}

#[test]
fn the_opening_chooses_nothing_and_a_new_tab_chooses_the_rows_first() {
    // FUN_00461960 fills the list itself (FUN_00462be0(1, 1)) and
    // selects its tab without a notification (FUN_0060d7e0(1, 0)), as
    // the A0 capture's grey rows show; a tab's notification
    // (FUN_00462770 case 100) lets the empty name box choose the first
    // row (FUN_00609650).
    let galaxy = galaxy();
    let mut state = FleetFinderState::default();
    state.open(CockpitFaction::Alliance);
    drive(&galaxy, &mut state, (0.0, 0.0), vec![]);
    assert_eq!(state.window.as_ref().unwrap().chosen, None);

    state.set_tab(FinderTab::Alliance);
    drive(&galaxy, &mut state, (0.0, 0.0), vec![]);
    let list = rows(
        &galaxy.world,
        &rebellion_core::movement::MovementState::default(),
        &galaxy.fog,
        player_side(CockpitFaction::Alliance),
        FinderMode::Fleets,
        FinderTab::Alliance,
    );
    assert!(!list.is_empty());
    assert_eq!(state.window.as_ref().unwrap().chosen, Some(list[0].object));
}

#[test]
fn an_edit_that_keeps_the_chosen_row_leaves_the_list_where_it_is() {
    // FUN_00609650 chooses and scrolls only to a row not chosen
    // already (+0x3c bit 0, 0x60973f): "z" then "z0" both choose Z00.
    let mut galaxy = galaxy();
    let home = galaxy.world.fleets[galaxy.own].location;
    for index in 0..12 {
        let fleet = add_fleet(&mut galaxy.world, home, true, &["Nebulon-B"]);
        assert!(galaxy.world.rename_fleet(fleet, &format!("Z{index:02}")));
    }
    let mut state = FleetFinderState::default();
    state.open(CockpitFaction::Alliance);
    drive(
        &galaxy,
        &mut state,
        (0.0, 0.0),
        vec![vec![egui::Event::Text("z".into())]],
    );
    let chosen = state.window.as_ref().unwrap().chosen;
    assert!(chosen.is_some());
    // At least fourteen rows: the galaxy's fleets and Z00..Z11. Z00
    // lies above a scroll of 100 (the test fails without the rule).
    assert!(max_scroll(14) >= 100);
    // A fresh context: give the name box the keys again.
    let window = state.window.as_mut().unwrap();
    window.scroll = 100;
    window.focus_name = true;
    drive(
        &galaxy,
        &mut state,
        (0.0, 0.0),
        vec![vec![egui::Event::Text("0".into())]],
    );
    let window = state.window.as_ref().unwrap();
    assert_eq!((window.name.as_str(), window.chosen), ("z0", chosen));
    assert_eq!(window.scroll, 100);
}

#[test]
fn opening_an_open_finder_keeps_its_state() {
    // FUN_0042a0c0 builds the window only when no type 0x15 is open.
    let mut state = FleetFinderState::default();
    assert!(state.open(CockpitFaction::Alliance));
    state.set_tab(FinderTab::Imperial);

    assert!(!state.open(CockpitFaction::Alliance));

    assert_eq!(state.window.as_ref().unwrap().tab, FinderTab::Imperial);
}

#[test]
fn a_new_tab_clears_the_choice_and_a_new_mode_clears_the_name() {
    // FUN_00462be0 clears +0x16c; FUN_004632d0 empties the name box.
    let mut state = FleetFinderState::default();
    state.open(CockpitFaction::Empire);
    let window = state.window.as_mut().unwrap();
    window.chosen = Some(FleetWindowEntry::Fleet(FleetKey::default()));
    window.name = "Fleet".into();

    state.set_tab(FinderTab::Alliance);
    let window = state.window.as_ref().unwrap();
    assert_eq!(window.chosen, None);
    assert_eq!(window.name, "Fleet");

    state.window.as_mut().unwrap().chosen = Some(FleetWindowEntry::Fleet(FleetKey::default()));
    state.set_mode(FinderMode::Ships);
    let window = state.window.as_ref().unwrap();
    assert_eq!(window.name, "");
    assert_eq!(window.chosen, None);
    assert_eq!(window.tab, FinderTab::Alliance);
}

fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    }
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

/// Run two idle frames, then one frame per entry of `frames` with the
/// pointer at window point `(x, y)`; return the last action.
fn drive(
    galaxy: &Galaxy,
    state: &mut FleetFinderState,
    at: (f32, f32),
    frames: Vec<Vec<egui::Event>>,
) -> Option<FleetFinderAction> {
    drive_at(
        galaxy,
        state,
        frames.into_iter().map(|events| (at, events)).collect(),
    )
}

/// [`drive`], with the pointer at its own window point each frame.
fn drive_at(
    galaxy: &Galaxy,
    state: &mut FleetFinderState,
    frames: Vec<((f32, f32), Vec<egui::Event>)>,
) -> Option<FleetFinderAction> {
    let layout = layout();
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut emitted = None;
    let first = frames.first().map_or((0.0, 0.0), |(at, _)| *at);
    for ((x, y), extra) in [(first, vec![]), (first, vec![])].into_iter().chain(frames) {
        let pos = window_rect(layout).min + egui::vec2(x, y);
        let mut events = vec![egui::Event::PointerMoved(pos)];
        events.extend(extra.into_iter().map(|event| match event {
            egui::Event::PointerButton { pressed, .. } => press(pos, pressed),
            other => other,
        }));
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 520.0),
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            if let Some(action) = draw_fleet_finder(
                ctx,
                &galaxy.world,
                &rebellion_core::movement::MovementState::default(),
                &galaxy.fog,
                state,
                layout,
                &mut cache,
            ) {
                emitted = Some(action);
            }
        });
    }
    emitted
}

fn click() -> Vec<Vec<egui::Event>> {
    let at = egui::Pos2::ZERO;
    vec![vec![press(at, true)], vec![press(at, false)]]
}

fn center((x, y, w, h): (f32, f32, f32, f32)) -> (f32, f32) {
    (x + w / 2.0, y + h / 2.0)
}

/// The window point at the middle of list row `row`.
fn list_row(row: usize) -> (f32, f32) {
    (
        LIST.0 + 40.0,
        LIST.1 + ROW_HEIGHT * row as f32 + ROW_HEIGHT / 2.0,
    )
}

fn opened(faction: CockpitFaction) -> FleetFinderState {
    let mut state = FleetFinderState::default();
    state.open(faction);
    state
}

#[test]
fn typing_a_name_then_enter_opens_the_fleet_it_best_matches() {
    // FUN_00462a50: 0x408 chooses the best prefix row; 0x407 opens it.
    let galaxy = galaxy();
    let mut state = opened(CockpitFaction::Alliance);
    state.set_mode(FinderMode::Ships);

    let action = drive(
        &galaxy,
        &mut state,
        (0.0, 0.0),
        vec![
            vec![egui::Event::Text("VIC".into())],
            vec![key(egui::Key::Enter)],
        ],
    );

    assert_eq!(
        action,
        Some(FleetFinderAction::Open(FleetWindowEntry::Ship {
            fleet: galaxy.seen,
            index: 0
        }))
    );
    assert!(!state.is_open());
}

#[test]
fn a_row_click_copies_its_name_and_display_opens_it() {
    // FUN_00462770: 0x29b copies the row's name into 0xcd; 0xc9 opens
    // +0x16c.
    let galaxy = galaxy();
    // The Empire lists its own two fleets; the Alliance its own and the
    // one it sees. Each side numbers its own fleets (FUN_00517760), so
    // the Alliance's row 1 is the Empire's Fleet 1, the Empire's its
    // Fleet 2.
    for (faction, second, name) in [
        (CockpitFaction::Alliance, galaxy.seen, "Fleet 1"),
        (CockpitFaction::Empire, galaxy.unseen, "Fleet 2"),
    ] {
        let mut state = opened(faction);
        assert_eq!(drive(&galaxy, &mut state, list_row(1), click()), None);
        let window = state.window.as_ref().unwrap();
        assert_eq!(window.chosen, Some(FleetWindowEntry::Fleet(second)));
        assert_eq!(window.name, name);

        let display = CONTROLS[side(faction)].display.rect;
        let action = drive(&galaxy, &mut state, center(display), click());

        assert_eq!(
            action,
            Some(FleetFinderAction::Open(FleetWindowEntry::Fleet(second))),
            "{faction:?}"
        );
        assert!(!state.is_open());
    }
}

#[test]
fn display_with_nothing_chosen_keeps_the_finder_open() {
    let galaxy = galaxy();
    let mut state = opened(CockpitFaction::Alliance);
    let display = CONTROLS[0].display.rect;

    assert_eq!(drive(&galaxy, &mut state, center(display), click()), None);
    assert!(state.is_open());
}

#[test]
fn a_double_click_on_a_row_opens_it() {
    // FUN_00462770: list notification 0x309 opens the row.
    let galaxy = galaxy();
    let mut state = opened(CockpitFaction::Alliance);
    let at = egui::Pos2::ZERO;

    let action = drive(
        &galaxy,
        &mut state,
        list_row(0),
        vec![
            vec![press(at, true)],
            vec![press(at, false)],
            vec![press(at, true)],
            vec![press(at, false)],
        ],
    );

    assert_eq!(
        action,
        Some(FleetFinderAction::Open(FleetWindowEntry::Fleet(galaxy.own)))
    );
}

#[test]
fn a_click_elsewhere_then_on_a_row_only_chooses_the_row() {
    // FUN_00462770's 0x309 is the list's own double click: both clicks
    // on the row.
    let galaxy = galaxy();
    let mut state = opened(CockpitFaction::Alliance);
    let at = egui::Pos2::ZERO;
    let display = center(CONTROLS[0].display.rect);

    let action = drive_at(
        &galaxy,
        &mut state,
        vec![
            (display, vec![press(at, true)]),
            (display, vec![press(at, false)]),
            (list_row(0), vec![press(at, true)]),
            (list_row(0), vec![press(at, false)]),
        ],
    );

    assert_eq!(action, None);
    let window = state.window.as_ref().unwrap();
    assert_eq!(window.chosen, Some(FleetWindowEntry::Fleet(galaxy.own)));
}

#[test]
fn a_double_click_on_a_row_opens_it_after_a_click_elsewhere() {
    // egui counts the row's second click as a triple click here; the
    // list's own double click (0x309) is both clicks on the row.
    let galaxy = galaxy();
    let mut state = opened(CockpitFaction::Alliance);
    let at = egui::Pos2::ZERO;
    let display = center(CONTROLS[0].display.rect);
    let mut frames = vec![
        (display, vec![press(at, true)]),
        (display, vec![press(at, false)]),
    ];
    for _ in 0..2 {
        frames.push((list_row(0), vec![press(at, true)]));
        frames.push((list_row(0), vec![press(at, false)]));
    }

    let action = drive_at(&galaxy, &mut state, frames);

    assert_eq!(
        action,
        Some(FleetFinderAction::Open(FleetWindowEntry::Fleet(galaxy.own)))
    );
}

#[test]
fn a_second_click_long_after_the_first_only_chooses_the_row() {
    let galaxy = galaxy();
    let mut state = opened(CockpitFaction::Alliance);
    state.window.as_mut().unwrap().last_clicked = Some((0, -10.0));

    assert_eq!(drive(&galaxy, &mut state, list_row(0), click()), None);

    assert!(state.is_open());
}

#[test]
fn close_and_escape_close_the_finder() {
    // FUN_00462770 200 and FUN_00463360's Escape both close (+0x30).
    let galaxy = galaxy();
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let mut state = opened(faction);
        let close = CONTROLS[side(faction)].close.rect;
        assert_eq!(drive(&galaxy, &mut state, center(close), click()), None);
        assert!(!state.is_open(), "{faction:?}");
    }

    let mut state = opened(CockpitFaction::Alliance);
    drive(
        &galaxy,
        &mut state,
        (0.0, 0.0),
        vec![vec![key(egui::Key::Escape)]],
    );
    assert!(!state.is_open());
}

#[test]
fn the_tabs_and_mode_buttons_switch_the_list() {
    // FUN_0060d590 tabs at (36,78); group 0xfa's Ship Finder above its
    // Fleet Finder (FUN_00461960).
    let galaxy = galaxy();
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        let mut state = opened(faction);
        drive(&galaxy, &mut state, center(TABS[2].rect), click());
        assert_eq!(state.window.as_ref().unwrap().tab, FinderTab::Imperial);

        let controls = &CONTROLS[side(faction)];
        drive(
            &galaxy,
            &mut state,
            center(controls.ship_finder.rect),
            click(),
        );
        let report = state
            .report(
                &galaxy.world,
                &rebellion_core::movement::MovementState::default(),
                &galaxy.fog,
                Faction::Alliance,
            )
            .unwrap();
        assert_eq!(report.mode, FinderMode::Ships, "{faction:?}");
        assert_eq!(names(&report.rows), ["Victory Star Destroyer"]);

        drive(
            &galaxy,
            &mut state,
            center(controls.fleet_finder.rect),
            click(),
        );
        assert_eq!(state.window.as_ref().unwrap().mode, FinderMode::Fleets);
    }
}

/// The canvas 10 by 20 pixels in and twice the original size, so offsets
/// and scale both show.
fn scaled() -> CockpitLayout {
    CockpitLayout {
        canvas: CockpitViewport {
            x: 10.0,
            y: 20.0,
            width: 1280.0,
            height: 960.0,
        },
        galaxy: CockpitViewport {
            x: 140.0,
            y: 140.0,
            width: 970.0,
            height: 700.0,
        },
        scale: 2.0,
    }
}

/// A painted line ([`crate::theme::TestLine`]): `pos` is its pen's left
/// and its baseline.
struct Text {
    text: String,
    pos: egui::Pos2,
    size: f32,
    color: egui::Color32,
}

#[derive(Default)]
struct Run {
    action: Option<FleetFinderAction>,
    painted: Vec<(u32, egui::Pos2)>,
    texts: Vec<Text>,
}

impl Run {
    fn painted_at(&self, id: u32) -> Option<egui::Pos2> {
        self.painted
            .iter()
            .find(|(painted, _)| *painted == id)
            .map(|(_, at)| *at)
    }
}

/// Draw on [`scaled`] for the Alliance's view once per frame, each frame
/// with the pointer at its window point; the last frame's art and text
/// are kept.
fn run(
    galaxy: &Galaxy,
    state: &mut FleetFinderState,
    frames: Vec<((f32, f32), Vec<egui::Event>)>,
) -> Run {
    let layout = scaled();
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut result = Run::default();
    let first = frames.first().map_or((0.0, 0.0), |(at, _)| *at);
    let frames = [(first, vec![]), (first, vec![])].into_iter().chain(frames);
    for (index, ((x, y), extra)) in frames.enumerate() {
        let pos = window_rect(layout).min + egui::vec2(x, y) * 2.0;
        let mut events = vec![egui::Event::PointerMoved(pos)];
        events.extend(extra.into_iter().map(|event| match event {
            egui::Event::PointerButton { pressed, .. } => press(pos, pressed),
            other => other,
        }));
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1000.0),
            )),
            time: Some(index as f64 * 0.05),
            events,
            ..Default::default()
        };
        crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
        let output = ctx.run(input, |ctx| {
            if let Some(action) = draw_fleet_finder(
                ctx,
                &galaxy.world,
                &rebellion_core::movement::MovementState::default(),
                &galaxy.fog,
                state,
                layout,
                &mut cache,
            ) {
                result.action = Some(action);
            }
        });
        result.painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.take());
        result.texts = crate::theme::test_lines(&output.shapes)
            .into_iter()
            .map(|line| Text {
                text: line.text,
                pos: line.pen,
                size: line.size,
                color: line.color,
            })
            .collect();
    }
    result
}

fn idle(galaxy: &Galaxy, state: &mut FleetFinderState) -> Run {
    run(galaxy, state, vec![((0.0, 0.0), vec![])])
}

/// The galaxy with `count` more Alliance fleets at the Alliance's
/// system, for a list longer than the eight rows it shows.
fn crowded(count: usize) -> Galaxy {
    let mut galaxy = galaxy();
    let home = galaxy.world.fleets[galaxy.own].location;
    for _ in 0..count {
        add_fleet(&mut galaxy.world, home, true, &["Nebulon-B"]);
    }
    galaxy
}

#[test]
fn each_side_paints_its_own_art_in_place() {
    // FUN_00461960: the base, panel at (12, 13), rail at (412, 0), and
    // the buttons at the side's rectangles; the tabs at (36 + 52k, 78).
    // hyp: the current tab and mode show their second bitmap.
    let galaxy = galaxy();
    for (faction, base, panel, ship_panel, rail, buttons) in [
        (
            CockpitFaction::Alliance,
            10335,
            10526,
            10524,
            10586,
            [
                (10514, (423.0, 25.0)),
                (10518, (423.0, 93.0)),
                (10530, (423.0, 147.0)),
                (10529, (423.0, 201.0)),
            ],
        ),
        (
            CockpitFaction::Empire,
            10336,
            10527,
            10525,
            10590,
            [
                (10516, (426.0, 21.0)),
                (10520, (426.0, 89.0)),
                (10534, (426.0, 143.0)),
                (10533, (426.0, 197.0)),
            ],
        ),
    ] {
        let mut state = opened(faction);
        let origin = window_rect(scaled()).min;
        let at = |(x, y): (f32, f32)| Some(origin + egui::vec2(x, y) * 2.0);

        let drawn = idle(&galaxy, &mut state);

        assert_eq!(drawn.painted_at(base), at((0.0, 0.0)), "{faction:?}");
        assert_eq!(drawn.painted_at(panel), at((12.0, 13.0)));
        assert_eq!(drawn.painted_at(rail), at((412.0, 0.0)));
        for (id, point) in buttons {
            assert_eq!(drawn.painted_at(id), at(point), "{faction:?} {id}");
        }
        for (id, x) in [(10501, 36.0), (10502, 88.0), (10505, 140.0)] {
            assert_eq!(drawn.painted_at(id), at((x, 78.0)), "{faction:?} {id}");
        }
        assert_eq!(drawn.painted_at(ship_panel), None);

        state.set_mode(FinderMode::Ships);
        state.set_tab(FinderTab::Imperial);
        let drawn = idle(&galaxy, &mut state);

        assert_eq!(drawn.painted_at(ship_panel), at((12.0, 13.0)));
        assert_eq!(drawn.painted_at(panel), None);
        let (ship, fleet) = (buttons[2], buttons[3]);
        assert_eq!(drawn.painted_at(ship.0 + 1), at(ship.1), "{faction:?}");
        assert_eq!(drawn.painted_at(fleet.0 - 1), at(fleet.1), "{faction:?}");
        for (id, x) in [(10500, 36.0), (10502, 88.0), (10506, 140.0)] {
            assert_eq!(drawn.painted_at(id), at((x, 78.0)), "{faction:?} {id}");
        }
    }
}

#[test]
fn the_title_label_and_tab_name_follow_the_mode_and_tab() {
    // TEXTSTRA 0x1880/0x1885 centred in (36, 14, 350) in font 5 (A0
    // 2026-10-29), 0x1881/0x1886 at (36, 48) in font 4 and the tab's
    // 0x1882..0x1889 at (40, 119) in font 5, white (0x2ffffff).
    let galaxy = galaxy();
    let mut state = opened(CockpitFaction::Empire);
    let origin = window_rect(scaled()).min;
    let shown = |run: &Run, text: &str| {
        run.texts
            .iter()
            .find(|shown| shown.text == text)
            .map(|shown| (shown.pos - origin, shown.size, shown.color))
    };

    let drawn = idle(&galaxy, &mut state);

    // DrawTextA from each box's top: the baselines tmAscent 13 (fonts
    // 4 and 5) below; the title's spare width halved.
    let white = egui::Color32::WHITE;
    let font = |id| crate::theme::gdi_font(id, 2.0).size;
    let (_, width) = crate::theme::gdi_pen_positions(5, "Fleet Finder").unwrap();
    assert_eq!(
        shown(&drawn, "Fleet Finder"),
        Some((
            egui::vec2(
                (36.0 + ((350.0 - width) / 2.0).floor()) * 2.0,
                (14.0 + 13.0) * 2.0
            ),
            font(5),
            white
        ))
    );
    assert_eq!(
        shown(&drawn, "Fleet Name"),
        Some((egui::vec2(72.0, (48.0 + 13.0) * 2.0), font(4), white))
    );
    assert_eq!(
        shown(&drawn, "All Fleets"),
        Some((egui::vec2(80.0, (119.0 + 13.0) * 2.0), font(5), white))
    );

    state.set_mode(FinderMode::Ships);
    state.set_tab(FinderTab::Alliance);
    let drawn = idle(&galaxy, &mut state);

    assert!(shown(&drawn, "Ship Finder").is_some());
    assert!(shown(&drawn, "Ship Name").is_some());
    assert!(shown(&drawn, "Alliance Ships").is_some());
    assert!(shown(&drawn, "Fleet Finder").is_none());
    let labels: Vec<_> = [FinderMode::Fleets, FinderMode::Ships]
        .into_iter()
        .flat_map(|mode| FinderTab::ALL.map(|tab| tab.label(mode)))
        .collect();
    assert_eq!(
        labels,
        [
            "All Fleets",
            "Alliance Fleets",
            "Imperial Fleets",
            "All Ships",
            "Alliance Ships",
            "Imperial Ships"
        ]
    );
}

#[test]
fn the_list_shows_its_rows_from_the_scroll_grey_and_the_chosen_one_white() {
    // FUN_00607ea0: rows 350 by 20 from (36, 138), the text at x 5 in
    // font 10, grey 0x787878 and the chosen one white (the
    // constructor's colours; A0 2026-10-29). Scrolled two rows, the
    // 165-pixel list shows eight whole rows and the top of a ninth.
    let galaxy = crowded(9);
    let list = rows(
        &galaxy.world,
        &rebellion_core::movement::MovementState::default(),
        &galaxy.fog,
        Faction::Alliance,
        FinderMode::Fleets,
        FinderTab::All,
    );
    assert_eq!(list.len(), 11);
    let mut state = opened(CockpitFaction::Alliance);
    let window = state.window.as_mut().unwrap();
    window.scroll = 40;
    window.chosen = Some(list[3].object);
    let origin = window_rect(scaled()).min;

    let drawn = idle(&galaxy, &mut state);

    let shown: Vec<_> = drawn
        .texts
        .iter()
        // The rows, below the tab's label at the same x.
        .filter(|text| (text.pos.x - origin.x - 82.0).abs() < 0.01 && text.pos.y > origin.y + 260.0)
        .collect();
    assert_eq!(shown.len(), 9);
    for (line, text) in shown.iter().enumerate() {
        assert_eq!(text.text, list[2 + line].name);
        // DT_VCENTER in the 20-pixel row from 138 + 20 * line: 3 down,
        // then tmAscent 11.
        let baseline = text.pos.y - origin.y;
        assert!(
            (baseline - (138.0 + 3.0 + 11.0 + 20.0 * line as f32) * 2.0).abs() < 0.01,
            "{line}: {baseline}"
        );
        let colour = if line == 1 {
            egui::Color32::WHITE
        } else {
            ROW_COLOR
        };
        assert_eq!(text.color, colour, "{line}");
    }
}

#[test]
fn the_bars_down_arrow_scrolls_ten_and_the_report_counts_whole_rows() {
    // FUN_0060a490 (RCDATA 10653) right of the list, x 374: its down
    // arrow at y 294..303 steps the list's 10 when released over it.
    let galaxy = crowded(9);
    let mut state = opened(CockpitFaction::Alliance);
    let arrow = (380.0, 299.0);
    let click = vec![(arrow, vec![press(egui::Pos2::ZERO, true)]), (arrow, vec![press(egui::Pos2::ZERO, false)])];
    let drawn = run(&galaxy, &mut state, click.clone());
    assert_eq!(state.window.as_ref().unwrap().scroll, 10);
    let report = |state: &FleetFinderState| {
        state
            .report(
                &galaxy.world,
                &rebellion_core::movement::MovementState::default(),
                &galaxy.fog,
                Faction::Alliance,
            )
            .unwrap()
            .first_row
    };
    assert_eq!(report(&state), 0);
    let _ = run(&galaxy, &mut state, [click.clone(), click].concat());
    assert_eq!(state.window.as_ref().unwrap().scroll, 30);
    assert_eq!(report(&state), 1, "30 pixels: the second row is the first");
    let origin = window_rect(scaled()).min;
    assert_eq!(drawn.painted_at(10_685), Some(origin + egui::vec2(374.0, 138.0) * 2.0));
    // Held on it, the arrow draws pressed.
    let mut state = opened(CockpitFaction::Alliance);
    let held = run(&galaxy, &mut state, vec![(arrow, vec![press(egui::Pos2::ZERO, true)]), (arrow, vec![])]);
    assert!(held.painted_at(10_688).is_some());
    let mut state = opened(CockpitFaction::Alliance);
    let off = run(
        &galaxy,
        &mut state,
        vec![(arrow, vec![press(egui::Pos2::ZERO, true)]), ((200.0, 200.0), vec![])],
    );
    assert!(off.painted_at(10_688).is_none());
}

#[test]
fn scrolled_rows_clip_to_the_list_and_the_bars_column_chooses_nothing() {
    // Scrolled 30 pixels, row 0 (108..128) is gone, row 1 shows its
    // lower half, row 9 (288..308) its upper part, row 10 nothing.
    let galaxy = crowded(9);
    let list = rows(
        &galaxy.world,
        &rebellion_core::movement::MovementState::default(),
        &galaxy.fog,
        Faction::Alliance,
        FinderMode::Fleets,
        FinderTab::All,
    );
    let mut state = opened(CockpitFaction::Alliance);
    state.window.as_mut().unwrap().scroll = 30;
    let origin = window_rect(scaled()).min;
    let drawn = idle(&galaxy, &mut state);
    let shown: Vec<_> = drawn
        .texts
        .iter()
        .filter(|text| (text.pos.x - origin.x - 82.0).abs() < 0.01 && text.pos.y > origin.y + 200.0)
        .map(|text| text.text.clone())
        .collect();
    let names: Vec<_> = list[1..10].iter().map(|row| row.name.clone()).collect();
    assert_eq!(shown, names);
    // A click on row 2's height in the bar's column (x 374..386).
    let mut state = opened(CockpitFaction::Alliance);
    let at = (380.0, 138.0 + 50.0);
    let _ = run(
        &galaxy,
        &mut state,
        vec![(at, vec![press(egui::Pos2::ZERO, true)]), (at, vec![press(egui::Pos2::ZERO, false)])],
    );
    assert_eq!(state.window.as_ref().unwrap().chosen, None);
}

fn wheel(delta: f32) -> egui::Event {
    egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, delta),
        modifiers: egui::Modifiers::default(),
    }
}

#[test]
fn the_wheel_over_the_list_scrolls_it_a_row_at_a_time_within_its_rows() {
    // port: the original has no wheel; a notch moves a 20-pixel row,
    // within the bar's range (11 rows: 60 pixels, three rows).
    let galaxy = crowded(9);
    let mut state = opened(CockpitFaction::Alliance);
    let first_row = |state: &FleetFinderState| state.window.as_ref().unwrap().scroll / ROW_PIXELS;
    let over = list_row(3);

    run(&galaxy, &mut state, vec![(over, vec![wheel(-1.0)])]);
    assert_eq!(first_row(&state), 1);
    run(&galaxy, &mut state, vec![(over, vec![wheel(-1.0)]); 4]);
    assert_eq!(first_row(&state), 3, "11 rows, 8 shown");
    run(&galaxy, &mut state, vec![(over, vec![wheel(1.0)])]);
    assert_eq!(first_row(&state), 2);
    run(&galaxy, &mut state, vec![((20.0, 20.0), vec![wheel(-1.0)])]);
    assert_eq!(first_row(&state), 2, "the wheel beside the list");
    run(&galaxy, &mut state, vec![(over, vec![wheel(1.0)]); 4]);
    assert_eq!(first_row(&state), 0);
}

#[test]
fn leaving_the_name_box_without_enter_opens_nothing() {
    // FUN_00462a50: only 0x407, Enter, opens from the name box.
    let galaxy = galaxy();
    let mut state = opened(CockpitFaction::Alliance);
    let at = egui::Pos2::ZERO;

    let drawn = run(
        &galaxy,
        &mut state,
        vec![
            ((150.0, 50.0), vec![egui::Event::Text("Fleet".into())]),
            ((20.0, 300.0), vec![press(at, true)]),
            ((20.0, 300.0), vec![press(at, false)]),
        ],
    );

    assert_eq!(drawn.action, None);
    let window = state.window.as_ref().unwrap();
    assert_eq!(window.name, "Fleet");
    assert!(window.chosen.is_some());
}

#[test]
fn the_report_names_the_chosen_row_by_its_place() {
    let galaxy = galaxy();
    let mut state = opened(CockpitFaction::Alliance);
    state.window.as_mut().unwrap().chosen = Some(FleetWindowEntry::Fleet(galaxy.seen));

    let report = state
        .report(
            &galaxy.world,
            &rebellion_core::movement::MovementState::default(),
            &galaxy.fog,
            Faction::Alliance,
        )
        .unwrap();

    assert_eq!(report.chosen, Some(1));
    assert_eq!(report.rows[1].object, FleetWindowEntry::Fleet(galaxy.seen));
}

#[test]
fn the_controls_lie_where_they_draw_and_rows_above_the_first_are_hidden() {
    let mut state = FleetFinderState::default();
    let layout = scaled();
    assert_eq!(state.screen_rect(layout), None);
    assert_eq!(
        state.control_screen_rect(layout, FinderControl::Close),
        None
    );
    state.open(CockpitFaction::Empire);
    state.window.as_mut().unwrap().scroll = 20;
    let origin = window_rect(layout).min;
    let rect = |(x, y, w, h): (f32, f32, f32, f32)| {
        Some(egui::Rect::from_min_size(
            origin + egui::vec2(x, y) * 2.0,
            egui::vec2(w, h) * 2.0,
        ))
    };

    assert_eq!(state.screen_rect(layout), Some(window_rect(layout)));
    assert_eq!(
        state.control_screen_rect(layout, FinderControl::Row(0)),
        None
    );
    assert!(state
        .control_screen_rect(layout, FinderControl::Row(8))
        .is_some());
    assert_eq!(
        state.control_screen_rect(layout, FinderControl::Row(9)),
        None
    );
    assert_eq!(
        state.control_screen_rect(layout, FinderControl::Row(3)),
        rect((36.0, 178.0, 338.0, 20.0))
    );
    assert_eq!(
        state.control_screen_rect(layout, FinderControl::Close),
        rect((426.0, 21.0, 44.0, 41.0))
    );
    assert_eq!(
        state.control_screen_rect(layout, FinderControl::Tab(FinderTab::Imperial)),
        rect((140.0, 78.0, 49.0, 41.0))
    );
    assert_eq!(
        state.control_screen_rect(layout, FinderControl::NameBox),
        rect((143.0, 45.0, 250.0, 16.0))
    );
}

#[test]
fn the_finder_blocks_only_its_own_rectangle() {
    let mut state = FleetFinderState::default();
    let rect = window_rect(layout());
    assert!(!state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));

    state.open(CockpitFaction::Alliance);

    assert!(state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));
    assert!(!state.contains_screen_point(layout(), (rect.max.x + 1.0, rect.center().y)));
    assert_eq!(rect.size(), egui::vec2(470.0, 330.0));
}

#[test]
fn the_finder_stays_above_a_modeless_window_that_raises_itself_every_frame() {
    // FUN_0042a0c0 opens the Finder over the galaxy view's child windows;
    // a focused Fleet window raises itself every frame (fleet_window.rs).
    let galaxy = galaxy();
    let layout = layout();
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut state = opened(CockpitFaction::Alliance);
    let modeless = egui::Id::new("raising-fleet-window");
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 520.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            let _ = draw_fleet_finder(
                ctx,
                &galaxy.world,
                &rebellion_core::movement::MovementState::default(),
                &galaxy.fog,
                &mut state,
                layout,
                &mut cache,
            );
            // A focused child window may raise itself later in the same
            // frame. The original Finder remains the top-level owner.
            ctx.move_to_top(egui::LayerId::new(egui::Order::Foreground, modeless));
            egui::Area::new(modeless)
                .order(egui::Order::Foreground)
                .fixed_pos(egui::Pos2::ZERO)
                .show(ctx, |ui| {
                    ui.allocate_response(egui::vec2(700.0, 520.0), egui::Sense::click());
                });
        });
    }

    assert_eq!(
        ctx.layer_id_at(window_rect(layout).center())
            .map(|layer| layer.id),
        Some(egui::Id::new("original-fleet-finder"))
    );
}
