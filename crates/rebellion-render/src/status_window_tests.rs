use super::*;
use crate::bmp_cache::DllSource;
use crate::cockpit::CockpitViewport;
use rebellion_core::dat::{ExplorationStatus, Faction, SectorGroup};
use rebellion_core::ids::{DatId, SystemKey};
use rebellion_core::world::{Character, ControlKind, Sector, SkillPair, System};

fn skill(base: u32) -> SkillPair {
    SkillPair { base, variance: 0 }
}

fn luke() -> Character {
    Character {
        // MJCHARSD index 578, TEXTSTRA 10306 (0x2842) "Luke Skywalker".
        dat_id: DatId::new(0x3000_0242),
        name: "Luke Skywalker".into(),
        is_alliance: true,
        is_empire: false,
        is_major: true,
        diplomacy: skill(50),
        espionage: skill(50),
        ship_design: skill(0),
        troop_training: skill(0),
        facility_design: skill(0),
        // MJCHARSD 578. Fig. 3.46 shows the effective values: diplomacy
        // and espionage 50 + 50 * 50 / 100 = 75 (FUN_0053e940), combat
        // 90 + 90 * 50 / 100 = 135 (FUN_0053e900), at Force 50, a
        // Trainee (20..79).
        combat: skill(90),
        leadership: skill(70),
        loyalty: skill(100),
        jedi_probability: 100,
        jedi_level: skill(50),
        can_be_admiral: true,
        can_be_commander: true,
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
        // Luke is known and potential (MJCHARSD 578), so his Force base
        // is the class 50 (FUN_004f11f0, FUN_004eee60).
        force_potential: true,
        force_aware: true,
        force_base: 50,
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

fn world_with_luke() -> (GameWorld, CharacterKey, SystemKey, SystemKey) {
    let mut world = GameWorld::default();
    let yavin = add_system(&mut world, "Yavin");
    let hoth = add_system(&mut world, "Hoth");
    let mut character = luke();
    character.current_system = Some(yavin);
    let key = world.characters.insert(character);
    (world, key, yavin, hoth)
}

fn pairs(rows: &[StatusRow]) -> Vec<(&str, &str)> {
    rows.iter()
        .map(|row| (row.label.as_str(), row.value.as_str()))
        .collect()
}

#[test]
fn the_window_reports_the_object_it_shows() {
    let (_, key, ..) = world_with_luke();
    let mut state = StatusWindowState::default();
    assert_eq!(state.object(), None);
    state.open_character(key);
    assert_eq!(state.object(), Some(StatusObject::Character(key)));
}

#[test]
fn a_commander_commands_its_system_or_the_fleet_it_is_aboard() {
    // FUN_004486f0 names +0xa8, the object FUN_00561bb0 keys the order
    // to (FUN_0055ed30): A0 personnel-states/11, General Neva at Pzob,
    // "Commanding: Pzob".
    let (mut world, key, yavin, _) = world_with_luke();
    world.characters[key].command = CommandKind::General;
    let commanding = |world: &GameWorld| {
        pairs(&character_rows(world, &[], &[], key).expect("rows"))[0].1.to_owned()
    };
    assert_eq!(commanding(&world), "Yavin");

    let fleet = world.fleets.insert(rebellion_core::world::Fleet {
        location: yavin,
        characters: vec![key],
        is_alliance: true,
        ..Default::default()
    });
    world.characters[key].current_fleet = Some(fleet);
    world.characters[key].command = CommandKind::Admiral;
    assert_eq!(commanding(&world), world.fleet_name(fleet).unwrap());

    world.characters[key].command = CommandKind::None;
    assert_eq!(commanding(&world), "None");
}

#[test]
fn a_character_in_an_abode_is_attached_to_it() {
    // An abode is the character's container like a system, so Attached
    // names it (FUN_0044a2e0 reads the container's name).
    let (mut world, key, ..) = world_with_luke();
    let dagobah = world.abodes.insert(rebellion_core::world::Abode {
        dat_id: DatId::new(0x441),
        name: "Dagobah".into(),
        side: Faction::Neutral,
        special_forces: Vec::new(),
    });
    let luke = &mut world.characters[key];
    luke.current_system = None;
    luke.current_abode = Some(dagobah);
    let rows = character_rows(&world, &[], &[], key).expect("rows");
    assert!(pairs(&rows).contains(&("Attached: ", "Dagobah")));
}

#[test]
fn a_character_at_a_system_lists_the_rows_of_figure_3_46() {
    // FUN_004486f0, in order; manual p. 101, Fig. 3.46 (Luke at Yavin,
    // Awaiting Orders, Trainee, 75/75/135/70, no R&D, Admiral Yes).
    let (world, key, ..) = world_with_luke();
    let rows = character_rows(&world, &[], &[], key).expect("rows");
    assert_eq!(
        pairs(&rows),
        [
            ("Commanding:", "None"),
            ("Attached: ", "Yavin"),
            ("Status:", "Awaiting Orders"),
            ("Force Ranking:", "Trainee"),
            ("Diplomacy Rating:", "75"),
            ("Espionage Rating:", "75"),
            ("Combat Rating:", "135"),
            ("Leadership Rating:", "70"),
            ("R&D Capabilities", ""),
            (" Ship Design", "No"),
            (" Troop Training", "No"),
            (" Facility Design", "No"),
            ("Possible Command Ranks", ""),
            ("Admiral:", "Yes"),
            ("General:", "No"),
            ("Commander:", "Yes"),
        ]
    );
}

#[test]
fn leadership_rating_is_the_effective_leadership_with_the_seat_of_power() {
    // FUN_004486f0 34601 "Leadership Rating:" reads +0x88 (slot +0x1f4):
    // with the Emperor seated at an Empire-held Coruscant (0x90000109,
    // FUN_005617b0) an Imperial character's +0x92 is G3076 (50), so 70
    // reads 105 (FUN_004eede0).
    let (mut world, _, yavin, _) = world_with_luke();
    let coruscant = add_system(&mut world, "Coruscant");
    world.systems[coruscant].dat_id = DatId::new(0x109);
    world.systems[coruscant].control = ControlKind::Controlled(Faction::Empire);
    world.characters.insert(Character {
        dat_id: DatId::new(0x3400_0280),
        name: "Emperor Palpatine".into(),
        is_alliance: false,
        is_empire: true,
        current_system: Some(coruscant),
        ..luke()
    });
    let vader = world.characters.insert(Character {
        name: "Darth Vader".into(),
        is_alliance: false,
        is_empire: true,
        current_system: Some(yavin),
        ..luke()
    });
    let leadership = |world: &GameWorld| {
        let rows = character_rows(world, &[], &[], vader).expect("rows");
        pairs(&rows)
            .into_iter()
            .find(|(label, _)| *label == "Leadership Rating:")
            .map(|(_, value)| value.to_owned())
    };
    assert_eq!(leadership(&world).as_deref(), Some("105"));
    world.systems[coruscant].control = ControlKind::Controlled(Faction::Alliance);
    assert_eq!(leadership(&world).as_deref(), Some("70"));
}

#[test]
fn a_skill_above_zero_is_an_r_and_d_capability() {
    // FUN_004486f0: +0x80, +0x82 and +0x84 show Yes when non-zero.
    let (mut world, key, ..) = world_with_luke();
    world.characters[key].troop_training = skill(1);
    let rows = character_rows(&world, &[], &[], key).expect("rows");
    let value = |label: &str| {
        rows.iter()
            .find(|row| row.label == label)
            .map(|row| row.value.clone())
    };
    assert_eq!(value(" Troop Training").as_deref(), Some("Yes"));
    assert_eq!(value(" Ship Design").as_deref(), Some("No"));
}

#[test]
fn a_character_on_its_way_is_enroute_with_its_destination_and_arrival_day() {
    // FUN_004486f0: +0x50 bit 4 first, then FUN_004fd2b0's "ETA
    // Destination:" and "Day " + the arrival; Attached names the
    // container, the destination of an object in hyperspace.
    let (mut world, key, _, hoth) = world_with_luke();
    world.characters[key].current_system = None;
    world.characters[key].on_mission = true;
    let transit = MemberTransit {
        member: MissionMember::Character(key),
        mission_id: 3,
        to: hoth,
        arrival: 42,
        abode: None,
    };
    let rows = character_rows(&world, &[transit], &[], key).expect("rows");
    assert_eq!(
        pairs(&rows[..4]),
        [
            ("Commanding:", "None"),
            ("Attached: ", "Hoth"),
            ("Status:", "Enroute"),
            ("ETA Destination:", "Day 42"),
        ]
    );
    assert_eq!(rows[4].label, "Force Ranking:");
}

#[test]
fn the_status_word_takes_the_first_state_that_holds() {
    // FUN_004486f0: captured before a mission; a hidden mission
    // (FUN_00520b70) reads as Awaiting Orders.
    let (mut world, key, ..) = world_with_luke();
    let status = |world: &GameWorld| {
        character_rows(world, &[], &[], key).expect("rows")[2]
            .value
            .clone()
    };
    world.characters[key].on_mission = true;
    assert_eq!(status(&world), "On Mission");
    world.characters[key].on_hidden_mission = true;
    assert_eq!(status(&world), "Awaiting Orders");
    world.characters[key].on_hidden_mission = false;
    world.characters[key].is_captive = true;
    assert_eq!(status(&world), "Captured");
    // Injured (+0x94 non-zero) comes after captured and before a
    // mission.
    world.characters[key].injury = 5;
    assert_eq!(status(&world), "Captured");
    world.characters[key].is_captive = false;
    assert_eq!(status(&world), "Injured");
}

#[test]
fn force_ranking_follows_the_force_value_thresholds() {
    // FUN_004486f0: <10, <20, <80, <100, <120, else Jedi Master.
    for (value, word) in [
        (0, "None"),
        (9, "None"),
        (10, "Novice"),
        (19, "Novice"),
        (20, "Trainee"),
        (79, "Trainee"),
        (80, "Jedi Student"),
        (99, "Jedi Student"),
        (100, "Jedi Knight"),
        (119, "Jedi Knight"),
        (120, "Jedi Master"),
    ] {
        assert_eq!(force_ranking(value), word, "{value}");
    }
}

#[test]
fn the_background_follows_the_objects_side_and_the_players() {
    // FUN_00443130: 0x2d22 + side, plus 3 for a player not of side 1.
    assert_eq!(background(0, CockpitFaction::Alliance), 11_554);
    assert_eq!(background(1, CockpitFaction::Alliance), 11_555);
    assert_eq!(background(2, CockpitFaction::Alliance), 11_556);
    assert_eq!(background(0, CockpitFaction::Empire), 11_557);
    assert_eq!(background(1, CockpitFaction::Empire), 11_558);
    assert_eq!(background(2, CockpitFaction::Empire), 11_559);
}

#[test]
fn a_characters_picture_is_its_portrait_its_mini_less_0x4000() {
    // FUN_0042c3b0(.., 1, 1): class +0x30 & 0xfff. Luke's TEXTSTRA id
    // is 0x2842, so his mini is 18498 and his portrait 2114.
    let (world, key, _, _) = world_with_luke();
    let view = Empty::new().view(&world, StatusObject::Character(key)).unwrap();
    assert_eq!(view.picture, Some((DllSource::Gokres, 2_114)));
    assert_eq!(view.title, "Character Status");
}

/// The other state the fillers read, empty.
pub(crate) struct Empty {
    missions: rebellion_core::missions::MissionState,
    movement: rebellion_core::movement::MovementState,
    transport: rebellion_core::troop_transport::TroopTransportState,
    manufacturing: rebellion_core::manufacturing::ManufacturingState,
}

impl Empty {
    pub(crate) fn new() -> Self {
        Self {
            missions: rebellion_core::missions::MissionState::new(),
            movement: rebellion_core::movement::MovementState::new(),
            transport: rebellion_core::troop_transport::TroopTransportState::new(),
            manufacturing: rebellion_core::manufacturing::ManufacturingState::new(),
        }
    }

    pub(crate) fn sources<'a>(&'a self, world: &'a GameWorld) -> StatusSources<'a> {
        StatusSources {
            world,
            missions: &self.missions,
            movement: &self.movement,
            transport: &self.transport,
            manufacturing: &self.manufacturing,
            today: 0,
            objectives: crate::status_rows::ObjectiveSource::default(),
        }
    }

    fn view(
        &self,
        world: &GameWorld,
        object: StatusObject,
    ) -> Option<crate::status_rows::StatusView> {
        crate::status_rows::status_view(self.sources(world), object)
    }
}

fn layout() -> CockpitLayout {
    CockpitLayout {
        canvas: CockpitViewport {
            x: 10.0,
            y: 20.0,
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

fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    }
}

/// Run two idle frames, then one per entry of `frames`, with the pointer
/// at window pixel `(x, y)`.
fn drive(
    world: &GameWorld,
    state: &mut StatusWindowState,
    point: (f32, f32),
    frames: Vec<Vec<egui::Event>>,
) -> Option<StatusWindowAction> {
    drive_with(
        world,
        state,
        point,
        frames,
        crate::status_rows::ObjectiveSource::default(),
        layout(),
    )
}

/// [`drive`] with the Objectives' source; the bitmaps painted go to
/// `PAINTED`, cleared first.
fn drive_with(
    world: &GameWorld,
    state: &mut StatusWindowState,
    (x, y): (f32, f32),
    frames: Vec<Vec<egui::Event>>,
    objectives: crate::status_rows::ObjectiveSource,
    layout: CockpitLayout,
) -> Option<StatusWindowAction> {
    crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
    let pos = window_rect(layout).min + egui::vec2(x, y) * layout.scale;
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let empty = Empty::new();
    let mut emitted = None;
    for extra in [vec![], vec![]].into_iter().chain(frames) {
        let mut events = vec![egui::Event::PointerMoved(pos)];
        events.extend(extra);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1100.0),
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            if let Some(action) = draw_status_window(
                ctx,
                StatusSources {
                    objectives,
                    ..empty.sources(world)
                },
                state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
            ) {
                emitted = Some(action);
            }
        });
    }
    emitted
}

fn click(
    world: &GameWorld,
    state: &mut StatusWindowState,
    point: (f32, f32),
) -> Option<StatusWindowAction> {
    let pos = window_rect(layout()).min + egui::vec2(point.0, point.1);
    drive(
        world,
        state,
        point,
        vec![vec![press(pos, true)], vec![press(pos, false)]],
    )
}

#[test]
fn the_buttons_open_the_encyclopedia_or_close_and_both_close_the_window() {
    // FUN_00443020: 0x66 at (258, 218), 0x65 at (324, 218);
    // FUN_004443a0 closes (+0x30) for both.
    let (world, key, ..) = world_with_luke();
    for (point, expected) in [
        (
            (258.0 + 16.0, 218.0 + 15.0),
            StatusWindowAction::Encyclopedia,
        ),
        ((324.0 + 16.0, 218.0 + 15.0), StatusWindowAction::Close),
    ] {
        let mut state = StatusWindowState::default();
        state.open_character(key);
        assert_eq!(click(&world, &mut state, point), Some(expected));
        assert!(!state.is_open());
    }
}

#[test]
fn a_click_beside_the_buttons_keeps_the_window_open() {
    let (world, key, ..) = world_with_luke();
    for point in [(257.0, 230.0), (300.0, 230.0), (340.0, 249.5)] {
        let mut state = StatusWindowState::default();
        state.open_character(key);
        assert_eq!(click(&world, &mut state, point), None, "{point:?}");
        assert!(state.is_open());
    }
}

#[test]
fn escape_closes_the_window() {
    let (world, key, ..) = world_with_luke();
    let mut state = StatusWindowState::default();
    state.open_character(key);
    let escape = egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    };
    assert_eq!(
        drive(&world, &mut state, (0.0, 0.0), vec![vec![escape]]),
        Some(StatusWindowAction::Close)
    );
    assert!(!state.is_open());
}

#[test]
fn the_window_paints_its_background_and_takes_the_pointer_only_over_itself() {
    // FUN_00442d70: 379 by 272; the side-1 character's background for a
    // side-1 player is 11554.
    let (world, key, ..) = world_with_luke();
    let mut state = StatusWindowState::default();
    state.open_character(key);
    crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
    let _ = drive(&world, &mut state, (0.0, 0.0), vec![]);
    let rect = window_rect(layout());
    let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow().clone());
    assert!(painted.contains(&(11_554, rect.min)), "{painted:?}");
    assert!(painted.iter().any(|(id, _)| *id == ENCYCLOPEDIA.0));
    assert!(painted.iter().any(|(id, _)| *id == CLOSE.0));

    assert_eq!(
        rect.size(),
        egui::vec2(STATUS_WINDOW_WIDTH, STATUS_WINDOW_HEIGHT)
    );
    assert!(state.contains_screen_point(layout(), (rect.min.x, rect.min.y)));
    assert!(!state.contains_screen_point(layout(), (rect.min.x - 1.0, rect.min.y)));
    assert!(
        !StatusWindowState::default().contains_screen_point(layout(), (rect.min.x, rect.min.y))
    );
}

#[test]
fn a_shorter_cell_sits_at_the_bottom_of_its_row_and_rows_stack() {
    // FUN_00449e00: the row advances by its taller cell; a value shorter
    // than its label moves down by the difference, a taller one does not
    // move the label.
    let (tops, height) = row_tops(&[(20.0, 10.0), (10.0, 30.0), (5.0, 5.0)]);
    assert_eq!(tops, [(0.0, 10.0), (20.0, 20.0), (50.0, 50.0)]);
    assert_eq!(height, 55.0);
}

#[test]
fn the_picture_centres_on_307_64_with_integer_halves() {
    // FUN_00443130: (0x133 - w / 2, 0x40 - h / 2).
    assert_eq!(picture_origin(80, 80), (267.0, 24.0));
    assert_eq!(picture_origin(81, 79), (267.0, 25.0));
}

#[test]
fn an_objective_icon_is_centred_in_the_first_icons_37_pixel_canvas() {
    // FUN_0044b2d0: FUN_005fcc30(canvas, (w0 - w) / 2, (h0 - h) / 2).
    assert_eq!(icon_offset(37, 37), (0.0, 0.0));
    assert_eq!(icon_offset(25, 25), (6.0, 6.0));
    assert_eq!(icon_offset(24, 26), (6.0, 5.0));
}

#[test]
fn the_list_bar_sits_at_the_lists_right_and_shows_only_when_rows_overflow() {
    // FUN_0060a490(.., list, 7, 0x299a, ..): RCDATA 10650's 12-wide bar
    // at the list's right edge (18 + 208 - 12); six 35-pixel Objectives
    // rows (210) overflow the 204-pixel list, two do not.
    let bar = list_bar(210.0, 0.0);
    assert!(bar.visible());
    assert_eq!(bar.rect(), (214.0, 47.0, 12.0, 204.0));
    assert_eq!(bar.max, 11.0);
    let pieces = bar.pieces(None);
    assert_eq!(pieces.first().map(|piece| piece.id), Some(10_654));
    assert!(pieces.iter().any(|piece| piece.id == 10_658));
    assert!(pieces.iter().any(|piece| piece.id == 10_662));
    assert!(!list_bar(70.0, 0.0).visible());
}

/// The Alliance's six Objectives rows (210 pixels) in the 204-pixel list.
fn six_rows() -> crate::status_rows::ObjectiveSource {
    crate::status_rows::ObjectiveSource {
        alliance: true,
        conditions_word: 0,
        standard: true,
    }
}

fn painted() -> Vec<(u32, egui::Pos2)> {
    crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow().clone())
}

#[test]
fn a_click_on_the_bars_down_arrow_scrolls_the_list_ten_pixels() {
    // FUN_0060a490's arrow steps by the list's 10 when released over it.
    // The bar is at the list's right, (214, 47) to (226, 251): the down
    // arrow fills its last 9 rows.
    let world = GameWorld::default();
    let mut state = StatusWindowState::default();
    state.open(StatusObject::Objectives);
    let arrow = (220.0, 246.0);
    let _ = drive_with(&world, &mut state, arrow, vec![vec![press_at(arrow, true)], vec![press_at(arrow, false)]], six_rows(), layout());
    assert_eq!(state.window.as_ref().unwrap().scroll, 10.0);
    // Its pieces: the track, then the up and down arrows.
    let origin = window_rect(layout()).min;
    let at = |x: f32, y: f32| origin + egui::vec2(x, y);
    let pieces = painted();
    assert!(pieces.contains(&(10_654, at(214.0, 56.0))), "{pieces:?}");
    assert!(pieces.contains(&(10_658, at(214.0, 47.0))), "{pieces:?}");
    assert!(pieces.contains(&(10_662, at(214.0, 242.0))), "{pieces:?}");
}

#[test]
fn a_held_arrow_draws_pressed_only_while_the_pointer_stays_on_it() {
    let world = GameWorld::default();
    let mut state = StatusWindowState::default();
    state.open(StatusObject::Objectives);
    let arrow = (220.0, 246.0);
    // The last frame's down arrow: pressed (10664) or not (10662).
    let last_arrow = || {
        painted()
            .iter()
            .rev()
            .find(|(id, _)| matches!(id, 10_662 | 10_664))
            .map(|(id, _)| *id)
    };
    let _ = drive_with(&world, &mut state, arrow, vec![vec![press_at(arrow, true)], vec![]], six_rows(), layout());
    assert_eq!(last_arrow(), Some(10_664));
    // Moved off it, the arrow is drawn up again.
    let mut state = StatusWindowState::default();
    state.open(StatusObject::Objectives);
    let _ = drive_with(&world, &mut state, (100.0, 100.0), vec![vec![press_at(arrow, true)], vec![]], six_rows(), layout());
    assert_eq!(last_arrow(), Some(10_662));
}

#[test]
fn at_scale_two_the_bar_reads_the_pointer_in_window_pixels() {
    let world = GameWorld::default();
    let mut state = StatusWindowState::default();
    state.open(StatusObject::Objectives);
    let arrow = (220.0, 246.0);
    let press = |pressed| {
        super::tests::press(window_rect(scaled()).min + egui::vec2(arrow.0, arrow.1) * 2.0, pressed)
    };
    let _ = drive_with(&world, &mut state, arrow, vec![vec![press(true)], vec![press(false)]], six_rows(), scaled());
    assert_eq!(state.window.as_ref().unwrap().scroll, 10.0);
    // Held there, the arrow draws pressed.
    let mut state = StatusWindowState::default();
    state.open(StatusObject::Objectives);
    let _ = drive_with(&world, &mut state, arrow, vec![vec![press(true)], vec![]], six_rows(), scaled());
    let last = painted()
        .iter()
        .rev()
        .find(|(id, _)| matches!(id, 10_662 | 10_664))
        .map(|(id, _)| *id);
    assert_eq!(last, Some(10_664));
    // A click on the track under the thumb pages by the list's height,
    // up to the range's end, 11 (FUN_0060fd50).
    let mut state = StatusWindowState::default();
    state.open(StatusObject::Objectives);
    let track = (220.0, 237.0);
    let page = |pressed| {
        super::tests::press(window_rect(scaled()).min + egui::vec2(track.0, track.1) * 2.0, pressed)
    };
    let _ = drive_with(&world, &mut state, track, vec![vec![page(true)], vec![page(false)]], six_rows(), scaled());
    assert_eq!(state.window.as_ref().unwrap().scroll, 11.0);
    // The thumb is the same at any scale: its bottom cap at window
    // (214, 56 + 176 - 6).
    let cap = window_rect(scaled()).min + egui::vec2(214.0, 226.0) * 2.0;
    assert!(painted().contains(&(10_669, cap)), "{:?}", painted());
}

/// A press or release at a window point (scale 1).
fn press_at((x, y): (f32, f32), pressed: bool) -> egui::Event {
    press(window_rect(layout()).min + egui::vec2(x, y), pressed)
}

#[test]
fn the_wheel_scrolls_the_list_only_as_far_as_its_rows_reach() {
    // port: the wheel scrolls within the bar's range: at scale 2, 500
    // screen pixels of rows overflow the 204-pixel list by 46 window
    // pixels, and content that fills a list gains 5 (FUN_00609ae0).
    assert_eq!(scrolled(0.0, -20.0, 2.0, 500.0), 10.0);
    assert_eq!(scrolled(10.0, 8.0, 2.0, 500.0), 6.0);
    assert_eq!(scrolled(40.0, -200.0, 2.0, 500.0), 51.0);
    assert_eq!(scrolled(5.0, 100.0, 2.0, 500.0), 0.0);
    assert_eq!(scrolled(5.0, -100.0, 2.0, 300.0), 0.0);
}

#[test]
fn a_side_past_the_third_takes_the_other_background() {
    assert_eq!(background(3, CockpitFaction::Alliance), 11_556);
    assert_eq!(background(3, CockpitFaction::Empire), 11_559);
}

fn scaled() -> CockpitLayout {
    CockpitLayout {
        canvas: CockpitViewport {
            x: 20.0,
            y: 40.0,
            width: 1280.0,
            height: 960.0,
        },
        galaxy: CockpitViewport {
            x: 130.0,
            y: 120.0,
            width: 970.0,
            height: 700.0,
        },
        scale: 2.0,
    }
}

/// A drawn text: its string, top-left, size and font size.
/// A painted line: its text, pen and baseline, laid-out size and font
/// size.
type Text = (String, egui::Pos2, egui::Vec2, f32);

/// Two frames at scale 2; the last frame's text shapes as (text,
/// top-left, size, font size), and the bitmaps painted.
fn render(
    world: &GameWorld,
    state: &mut StatusWindowState,
) -> (Vec<Text>, Vec<(u32, egui::Pos2)>) {
    render_with(world, state, crate::status_rows::ObjectiveSource::default())
}

/// [`render`] with the Objectives' source.
fn render_with(
    world: &GameWorld,
    state: &mut StatusWindowState,
    objectives: crate::status_rows::ObjectiveSource,
) -> (Vec<Text>, Vec<(u32, egui::Pos2)>) {
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    cache.set_base_path(staged_portrait());
    let empty = Empty::new();
    let mut output = None;
    crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
    for _ in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1100.0),
            )),
            ..Default::default()
        };
        output = Some(ctx.run(input, |ctx| {
            let _ = draw_status_window(
                ctx,
                StatusSources {
                    objectives,
                    ..empty.sources(world)
                },
                state,
                CockpitFaction::Alliance,
                scaled(),
                &mut cache,
            );
        }));
    }
    // paint_gdi_text paints a line a character at a time: join each run
    // of one size and baseline into the line, at its pen and baseline.
    fn collect(shape: &egui::Shape, out: &mut Vec<Text>) {
        match shape {
            egui::Shape::Text(text) => {
                let size = text
                    .galley
                    .job
                    .sections
                    .first()
                    .map_or(0.0, |section| section.format.font_id.size);
                let baseline = text.pos.y
                    + text
                        .galley
                        .rows
                        .first()
                        .and_then(|row| row.glyphs.first())
                        .map_or(0.0, |glyph| glyph.pos.y);
                let right = text.pos.x + text.galley.size().x;
                match out.last_mut() {
                    Some(line)
                        if line.3 == size
                            && line.1.y == baseline
                            && (text.pos.x - (line.1.x + line.2.x)).abs() < size =>
                    {
                        line.0.push_str(text.galley.text());
                        line.2.x = right - line.1.x;
                    }
                    _ => out.push((
                        text.galley.text().to_owned(),
                        egui::pos2(text.pos.x, baseline),
                        text.galley.size(),
                        size,
                    )),
                }
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut texts = Vec::new();
    for clipped in &output.expect("a frame").shapes {
        collect(&clipped.shape, &mut texts);
    }
    let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow().clone());
    (texts, painted)
}

/// A staged 80 by 80 GOKRES 2114, Luke's portrait's size, in a fresh
/// directory.
fn staged_portrait() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("status-portrait-{}", std::process::id()));
    let dir = root.join("gokres-dll/BMP");
    std::fs::create_dir_all(&dir).unwrap();
    // An 8-bit indexed BMP, as the original resources are.
    let (side, offset) = (80usize, 54usize + 1024);
    let mut bmp = vec![1_u8; offset + side * side];
    bmp[..2].copy_from_slice(b"BM");
    bmp[10..14].copy_from_slice(&u32::try_from(offset).unwrap().to_le_bytes());
    bmp[14..18].copy_from_slice(&40u32.to_le_bytes());
    bmp[18..22].copy_from_slice(&i32::try_from(side).unwrap().to_le_bytes());
    bmp[22..26].copy_from_slice(&i32::try_from(side).unwrap().to_le_bytes());
    bmp[26..28].copy_from_slice(&1u16.to_le_bytes());
    bmp[28..30].copy_from_slice(&8u16.to_le_bytes());
    bmp[30..54].fill(0);
    std::fs::write(dir.join("2114.bmp"), bmp).unwrap();
    // The Objectives' icons: the 37 by 37 planet and 25 by 25 portraits.
    let strategy = root.join("strategy-dll/BMP");
    std::fs::create_dir_all(&strategy).unwrap();
    for (id, side) in [(10_234, 37), (11_013, 25), (11_055, 25), (904, 37), (11_028, 25), (11_032, 25)] {
        std::fs::write(strategy.join(format!("{id}.bmp")), indexed_bmp(side, side)).unwrap();
    }
    // A regiment's 61 by 25 list mini.
    std::fs::write(dir.join("17473.bmp"), indexed_bmp(61, 25)).unwrap();
    root
}

/// A `width` by `height` 8-bit indexed BMP.
fn indexed_bmp(width: usize, height: usize) -> Vec<u8> {
    let stride = width.div_ceil(4) * 4;
    let offset = 54usize + 1024;
    let mut bmp = vec![1_u8; offset + stride * height];
    bmp[..2].copy_from_slice(b"BM");
    bmp[10..14].copy_from_slice(&u32::try_from(offset).unwrap().to_le_bytes());
    bmp[14..18].copy_from_slice(&40u32.to_le_bytes());
    bmp[18..22].copy_from_slice(&i32::try_from(width).unwrap().to_le_bytes());
    bmp[22..26].copy_from_slice(&i32::try_from(height).unwrap().to_le_bytes());
    bmp[26..28].copy_from_slice(&1u16.to_le_bytes());
    bmp[28..30].copy_from_slice(&8u16.to_le_bytes());
    bmp[30..54].fill(0);
    bmp
}

fn find<'a>(texts: &'a [Text], text: &str) -> &'a Text {
    texts
        .iter()
        .find(|(found, ..)| found == text)
        .unwrap_or_else(|| panic!("{text} not drawn in {texts:?}"))
}

fn near(a: egui::Pos2, b: egui::Pos2) -> bool {
    (a - b).length() < 0.01
}

#[test]
fn the_title_name_and_rows_sit_where_the_window_lays_them_out() {
    // FUN_00443130 at scale 2: the title centred in (15, 18, 211, 18)
    // in font 5, the name centred across (242, 137, 130, 44) from its
    // top (format 0x11) in font 5, labels at the list's (18, 47) and
    // values 103 to their right (FUN_00449e00) in font 10.
    let (world, key, ..) = world_with_luke();
    let mut state = StatusWindowState::default();
    state.open_character(key);
    let (texts, _) = render(&world, &mut state);
    let frame = window_rect(scaled()).min;
    let at = |x: f32, y: f32| frame + egui::vec2(x, y) * 2.0;

    // DrawTextA, as theme::paint_gdi_text sets it: the title centred
    // in its 211 from the box's top (format 0x21, A0 2026-10-29: caps
    // from y 21, tmAscent 13), the name centred in its 130.
    let centred = |text: &str, font: u16, (x, y, w): (f32, f32, f32)| {
        let (_, width) = crate::theme::gdi_pen_positions(font, text).unwrap();
        at(
            x + ((w - width) / 2.0).floor(),
            y + crate::theme::original_font_metrics(font).0,
        )
    };
    let (_, pos, _, font) = find(&texts, "Character Status");
    assert!(
        near(*pos, centred("Character Status", 5, (15.0, 18.0, 211.0))),
        "{pos:?}"
    );
    assert_eq!(*font, crate::theme::gdi_font(5, 2.0).size);
    let (_, pos, _, font) = find(&texts, "Luke Skywalker");
    assert!(
        near(*pos, centred("Luke Skywalker", 5, (242.0, 137.0, 130.0))),
        "{pos:?}"
    );
    assert_eq!(*font, crate::theme::gdi_font(5, 2.0).size);

    // A0 (2026-10-29, Veers): rows 14 apart, caps 3 below a row's top:
    // font 10's 11 ppem lines (tmAscent 11, tmHeight 14).
    let (_, commanding, _, font) = find(&texts, "Commanding:");
    assert!(near(*commanding, at(18.0, 47.0 + 11.0)), "{commanding:?}");
    assert_eq!(*font, crate::theme::gdi_font(10, 2.0).size);
    let (_, none, ..) = find(&texts, "None");
    assert!(near(*none, at(18.0 + 103.0, 47.0 + 11.0)), "{none:?}");
    let (_, yavin, ..) = find(&texts, "Yavin");
    assert!(near(*yavin, *none + egui::vec2(0.0, 28.0)), "{yavin:?}");
    let (_, attached, ..) = find(&texts, "Attached:");
    assert!(
        near(*attached, *commanding + egui::vec2(0.0, 28.0)),
        "{attached:?}"
    );
}

#[test]
fn the_rows_break_in_their_columns_on_font_tens_lines() {
    // A0 (2026-10-29): rows 14 apart, a wrapped cell a line more:
    // DT_WORDBREAK in the 103-wide column, font 10's 14-pixel line.
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let rows = [
        StatusRow {
            label: "Attached:".to_owned(),
            value: "Coruscant".to_owned(),
        },
        StatusRow {
            label: "Possible Command Ranks".to_owned(),
            value: String::new(),
        },
    ];
    let mut laid = None;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        let painter = ctx.layer_painter(egui::LayerId::background());
        laid = Some(layout_rows(&painter, &rows, 1.0));
    });
    let (laid, height) = laid.unwrap();
    assert_eq!(laid[0].0, ["Attached:"]);
    assert_eq!(laid[0].1, ["Coruscant"]);
    assert_eq!(laid[1].0, ["Possible Command", "Ranks"]);
    assert_eq!(laid[1].2, 14.0);
    assert_eq!(height, 14.0 * 3.0, "the second label wraps to two lines");
}

#[test]
fn a_scrolled_list_draws_its_rows_higher() {
    // A name that wraps over many lines pushes the rows past the list.
    let (mut world, key, yavin, _) = world_with_luke();
    world.systems[yavin].name = "Yavin ".repeat(40);
    let mut state = StatusWindowState::default();
    state.open_character(key);
    if let Some(open) = state.window.as_mut() {
        open.scroll = 10.0;
    }
    let (texts, _) = render(&world, &mut state);
    let frame = window_rect(scaled()).min;
    let (_, commanding, ..) = find(&texts, "Commanding:");
    assert!(
        near(
            *commanding,
            frame + egui::vec2(18.0, 47.0 + 11.0 - 10.0) * 2.0
        ),
        "{commanding:?}"
    );
}

#[test]
fn the_portrait_and_the_sides_background_are_painted() {
    // FUN_00443130: Luke's 80 by 80 portrait at (267, 24); an Empire
    // character's background 11555 and a neutral one's 11556 for a
    // side-1 player. Reads the staged GOKRES and STRATEGY bitmaps.
    let (mut world, key, ..) = world_with_luke();
    let mut state = StatusWindowState::default();
    state.open_character(key);
    let frame = window_rect(scaled()).min;
    let (_, painted) = render(&world, &mut state);
    assert!(painted.contains(&(11_554, frame)), "{painted:?}");
    assert!(
        painted.contains(&(2_114, frame + egui::vec2(267.0, 24.0) * 2.0)),
        "{painted:?}"
    );

    world.characters[key].is_alliance = false;
    world.characters[key].is_empire = true;
    let (_, painted) = render(&world, &mut state);
    assert!(painted.contains(&(11_555, frame)), "{painted:?}");
    world.characters[key].is_empire = false;
    let (_, painted) = render(&world, &mut state);
    assert!(painted.contains(&(11_556, frame)), "{painted:?}");
}

#[test]
fn the_marks_come_before_the_picture() {
    let marks = [(DllSource::Strategy, 11_514), (DllSource::Gokres, 0x1064)];
    let layers: Vec<_> = picture_layers(&marks, (DllSource::Gokres, 0x64)).collect();
    assert_eq!(
        layers,
        [
            (DllSource::Strategy, 11_514),
            (DllSource::Gokres, 0x1064),
            (DllSource::Gokres, 0x64)
        ]
    );
}

#[test]
fn the_marks_are_painted_under_the_picture_at_its_origin() {
    // FUN_0042c3b0 draws the marks into the picture's bitmap first, at
    // its (0, 0); the window blits the result at the picture's origin.
    let (world, key, ..) = world_with_luke();
    let mut state = StatusWindowState::default();
    state.open_character(key);
    let frame = window_rect(scaled()).min;
    let (_, painted) = render(&world, &mut state);
    let origin = frame + egui::vec2(267.0, 24.0) * 2.0;
    let picture = painted.iter().position(|entry| *entry == (2_114, origin));
    assert!(picture.is_some(), "{painted:?}");
    // A character's portrait has no mark (FUN_0042c3b0: 0x2ced wraps).
    assert!(!painted.iter().any(|(id, at)| *at == origin && *id != 2_114));
}

#[test]
fn the_objectives_rows_stack_35_apart_with_centred_icons_and_text_at_40() {
    // FUN_0044b2d0 rows in the list at (18, 47): each icon centred in a
    // 37 by 37 cell one pixel in (A0 2026-10-29), the text at x 40 from
    // that cell, centred on the 35-pixel row; scrolled by 3.
    let world = GameWorld::default();
    let mut state = StatusWindowState::default();
    state.open(StatusObject::Objectives);
    if let Some(open) = state.window.as_mut() {
        open.scroll = 3.0;
    }
    let source = crate::status_rows::ObjectiveSource {
        alliance: true,
        conditions_word: 0,
        standard: true,
    };
    let (texts, painted) = render_with(&world, &mut state, source);
    let frame = window_rect(scaled()).min;
    let at = |x: f32, y: f32| frame + egui::vec2(x, y) * 2.0;
    assert!(painted.contains(&(10_234, at(19.0, 44.0))), "{painted:?}");
    assert!(painted.contains(&(11_013, at(25.0, 85.0))), "{painted:?}");
    assert!(painted.contains(&(11_032, at(25.0, 225.0))), "{painted:?}");
    for (index, text) in ["Control Coruscant", "Capture Palpatine"].iter().enumerate() {
        let (_, pos, _, font) = find(&texts, text);
        // The picture list's font 4 (FUN_00600290): DT_VCENTER on the
        // 35-pixel row, (35 - 16) / 2 = 9 down, then tmAscent 13.
        #[expect(clippy::cast_precision_loss, reason = "Two rows.")]
        let pen = at(59.0, 44.0 + 9.0 + 13.0 + 35.0 * index as f32);
        assert!(near(*pos, pen), "{pos:?}");
        assert_eq!(*font, crate::theme::gdi_font(4, 2.0).size);
    }
    // FUN_00443020: no Encyclopedia for the empty id; the name from the
    // box's top, its baseline tmAscent 13 below.
    assert!(painted.contains(&(11_612, at(258.0, 218.0))), "{painted:?}");
    let (_, pos, ..) = find(&texts, "Objectives");
    assert!((pos.y - at(0.0, 137.0 + 13.0).y).abs() < 0.01, "{pos:?}");
}

#[test]
fn the_overview_rows_put_the_mini_at_the_cells_corner_and_centre_the_text() {
    // FUN_0044aa20 rows in the list's own format (+0xe4 = 9): the mini
    // at the cell's top-left one pixel in, the text centred across the
    // 208-wide cell less 3, from its top (A0 2026-10-29: text from x 77
    // to 132, its glyphs from y 3).
    let mut world = GameWorld::default();
    world.buildable_classes.insert(
        DatId::new(0x1000_0002),
        rebellion_core::world::BuildableClass {
            maintenance_cost: 1,
            ..rebellion_core::world::BuildableClass::default()
        },
    );
    world.troops.insert(rebellion_core::world::TroopUnit {
        class_dat_id: DatId::new(0x1000_0002),
        is_alliance: true,
        regiment_strength: 100,
    });
    let mut state = StatusWindowState::default();
    state.open(StatusObject::GalaxyOverview);
    let source = crate::status_rows::ObjectiveSource {
        alliance: true,
        conditions_word: 0,
        standard: true,
    };
    let (texts, painted) = render_with(&world, &mut state, source);
    let frame = window_rect(scaled()).min;
    let at = |x: f32, y: f32| frame + egui::vec2(x, y) * 2.0;
    assert!(painted.contains(&(17_473, at(19.0, 47.0))), "{painted:?}");
    // DT_CENTER in the 205-wide cell, from its top, in the picture
    // list's font 4 (tmAscent 13).
    let (_, pos, ..) = find(&texts, "001  0001");
    let (_, width) = crate::theme::gdi_pen_positions(4, "001  0001").unwrap();
    assert!(
        near(*pos, at(19.0 + ((205.0 - width) / 2.0).floor(), 47.0 + 13.0)),
        "{pos:?}"
    );
    let (_, pos, ..) = find(&texts, "Galaxy Overview");
    assert!((pos.y - at(0.0, 137.0 + 13.0).y).abs() < 0.01, "{pos:?}");
}

#[test]
fn a_window_for_a_character_that_is_gone_closes() {
    let (mut world, key, ..) = world_with_luke();
    let mut state = StatusWindowState::default();
    state.open_character(key);
    world.characters.remove(key);
    let _ = drive(&world, &mut state, (0.0, 0.0), vec![]);
    assert!(!state.is_open());
}
