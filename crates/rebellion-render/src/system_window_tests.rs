use super::*;

#[test]
fn each_tab_has_its_tooltip_string() {
    // FUN_00455060 (A0 tooltip/124..129).
    assert_eq!(
        super::SystemWindowTab::ALL.map(super::SystemWindowTab::tooltip_string),
        [0x1835, 0x1828, 0x1830, 0x1832, 0x1827, 0x1826]
    );
}
use crate::cockpit::CockpitViewport;
use crate::fleet_window::tests::PAINTED;
use rebellion_core::dat::{ExplorationStatus, SectorGroup};
use rebellion_core::manufacturing::QueueItem;
use rebellion_core::world::ControlKind;
use rebellion_core::world::{
    CapitalShipClass, ManufacturingFacilityInstance, ProductionFacilityInstance, Sector, System,
};

fn layout(faction: CockpitFaction, scale: f32) -> CockpitLayout {
    let (x, width, height) = match faction {
        CockpitFaction::Alliance => (55.0, 485.0, 350.0),
        CockpitFaction::Empire => (120.0, 480.0, 355.0),
    };
    CockpitLayout {
        canvas: CockpitViewport {
            x: 10.0,
            y: 20.0,
            width: 640.0 * scale,
            height: 480.0 * scale,
        },
        galaxy: CockpitViewport {
            x: 10.0 + x * scale,
            y: 20.0 + 40.0 * scale,
            width: width * scale,
            height: height * scale,
        },
        scale,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
)]
fn fixture_world(count: usize) -> (GameWorld, Vec<SystemKey>) {
    let mut world = GameWorld::default();
    let sector = world.sectors.insert(Sector {
        dat_id: DatId::new(36),
        name: "Sesswenna".into(),
        group: SectorGroup::Core,
        x: 317,
        y: 248,
        systems: Vec::new(),
    });
    let mut systems = Vec::new();
    for index in 0..count {
        let system = world.systems.insert(System {
            dat_id: DatId::new(100 + index as u32),
            name: format!("System {index}"),
            sector,
            x: 320 + index as u16,
            y: 250,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 1.0,
            is_populated: true,
            total_energy: 8,
            raw_materials: 6,
            control: ControlKind::Controlled(Faction::Alliance),
            ..Default::default()
        });
        world.sectors[sector].systems.push(system);
        systems.push(system);
    }
    (world, systems)
}

fn add_yard(world: &mut GameWorld, system: SystemKey, class: u32) -> FacilityItem {
    let key = world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id: DatId::new(class),
            side: rebellion_core::dat::Faction::Alliance,
            is_shipyard: class >> 24 == 0x28,
        });
    world.systems[system].manufacturing_facilities.push(key);
    FacilityItem::Manufacturing(key)
}

fn add_mine(world: &mut GameWorld, system: SystemKey) -> FacilityItem {
    let key = world
        .production_facilities
        .insert(ProductionFacilityInstance {
            class_dat_id: DatId::new(0x2c00_0001),
            side: rebellion_core::dat::Faction::Alliance,
            is_mine: true,
        });
    world.systems[system].production_facilities.push(key);
    FacilityItem::Production(key)
}

/// A painted text's words, its pen's left and baseline, width and font
/// size.
type RailText = (String, egui::Pos2, f32, f32);

/// One painted line of text: `paint_gdi_text` paints it a character at
/// a time, so each run of one size, colour, baseline and clip is
/// joined.
struct TextLine {
    text: String,
    pos: egui::Pos2,
    width: f32,
    size: f32,
    colour: egui::Color32,
    baseline: f32,
    clip: egui::Rect,
}

fn text_lines(shapes: &[egui::epaint::ClippedShape]) -> Vec<TextLine> {
    let mut lines: Vec<TextLine> = Vec::new();
    for clipped in shapes {
        let egui::Shape::Text(text) = &clipped.shape else {
            continue;
        };
        let format = &text.galley.job.sections[0].format;
        let baseline = text.pos.y
            + text
                .galley
                .rows
                .first()
                .and_then(|row| row.glyphs.first())
                .map_or(0.0, |glyph| glyph.pos.y);
        let right = text.pos.x + text.galley.size().x;
        if let Some(line) = lines.last_mut().filter(|line| {
            line.size == format.font_id.size
                && line.colour == format.color
                && line.baseline == baseline
                && line.clip == clipped.clip_rect
        }) {
            line.text.push_str(text.galley.text());
            line.width = right - line.pos.x;
            continue;
        }
        lines.push(TextLine {
            text: text.galley.text().to_owned(),
            pos: text.pos,
            width: text.galley.size().x,
            size: format.font_id.size,
            colour: format.color,
            baseline,
            clip: clipped.clip_rect,
        });
    }
    lines
}

#[derive(Default)]
struct Run {
    actions: Vec<SystemWindowAction>,
    texts: Vec<RailText>,
    painted: Vec<(u32, egui::Pos2)>,
    /// Each text's clip rectangle, by its string.
    clips: Vec<(String, egui::Rect)>,
    /// Each text's colour, by its string.
    colours: Vec<(String, egui::Color32)>,
    /// Each filled rectangle and its colour.
    fills: Vec<(egui::Rect, egui::Color32)>,
}

/// Draw the windows and rail on the Alliance's layout at scale 2, one
/// frame per entry of `frames`, the Alliance seeing every system. Keeps
/// every action and the last frame's texts and bitmaps.
fn run_with(
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    state: &mut SystemWindowState,
    frames: Vec<Vec<egui::Event>>,
) -> Run {
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let fog = FogState::new(Faction::Alliance);
    let mut run = Run::default();
    for events in frames {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1000.0),
            )),
            events,
            ..Default::default()
        };
        PAINTED.with(|painted| painted.borrow_mut().clear());
        let output = ctx.run(input, |ctx| {
            run.actions.extend(draw_system_windows(
                ctx,
                world,
                &rebellion_core::movement::MovementState::default(),
                &fog,
                &MissionState::new(),
                manufacturing,
                &DeliveryState::new(),
                state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
            ));
        });
        run.painted = PAINTED.with(|painted| painted.take());
        let lines = text_lines(&output.shapes);
        run.clips = lines
            .iter()
            .map(|line| (line.text.clone(), line.clip))
            .collect();
        run.colours = lines
            .iter()
            .map(|line| (line.text.clone(), line.colour))
            .collect();
        run.fills = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect) => Some((rect.rect, rect.fill)),
                _ => None,
            })
            .collect();
        run.texts = lines
            .into_iter()
            .map(|line| {
                (
                    line.text,
                    egui::pos2(line.pos.x, line.baseline),
                    line.width,
                    line.size,
                )
            })
            .collect();
    }
    run
}

fn run_rail(
    world: &GameWorld,
    state: &mut SystemWindowState,
    frames: Vec<Vec<egui::Event>>,
) -> (Vec<SystemWindowAction>, Vec<RailText>) {
    let run = run_with(world, &ManufacturingState::new(), state, frames);
    // The rail tests read the last frame's bitmaps from the record.
    PAINTED.with(|painted| *painted.borrow_mut() = run.painted);
    (run.actions, run.texts)
}

const ORIGIN: (i16, i16) = (100, 60);

/// A logical point in the window opened at `ORIGIN` on the scale 2
/// layout.
fn at(x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(
        10.0 + (f32::from(ORIGIN.0) + x) * 2.0,
        20.0 + (f32::from(ORIGIN.1) + y) * 2.0,
    )
}

/// Where DT_TOP puts a line of font `font` whose box starts at (`x`,
/// `y`) in the window: its pen's left and its baseline, `tmAscent`
/// below the top (theme::original_font_metrics).
fn line_at(x: f32, y: f32, font: u16) -> egui::Pos2 {
    at(x, y + crate::theme::original_font_metrics(font).0)
}

fn opened(world: &GameWorld, system: SystemKey) -> SystemWindowState {
    let mut state = SystemWindowState::default();
    assert!(state.open(
        world,
        system,
        ORIGIN,
        CockpitFaction::Alliance,
        layout(CockpitFaction::Alliance, 2.0)
    ));
    state
}

#[test]
fn a_band_drag_drops_past_four_pixels_and_shows_its_sides_image() {
    // FUN_004534f0 case 0x202: a release under 4 pixels from the
    // drag's start in both x and y drops nothing. FUN_00456230 loads
    // STRATEGY 0x2841 (Alliance) or 0x2844 (Empire) as the one-band
    // drag image; A0 2026-11-21-build-dest-16/28, 30, 32 show the
    // Empire's green-strip band outline under the pointer.
    assert!(!band_drag_drops((100, 100), (103, 97)));
    assert!(band_drag_drops((100, 100), (104, 100)));
    assert!(band_drag_drops((100, 100), (100, 96)));
    assert_eq!(band_drag_image(CockpitFaction::Alliance), 10305);
    assert_eq!(band_drag_image(CockpitFaction::Empire), 10308);
}

#[test]
fn a_mission_released_on_a_facility_page_targets_the_facility_under_the_point() {
    // +0x68 is FUN_00458b80: unless the page is the overview (0x67), the
    // item in list +0x174 at (8, 77) under the point, else the system.
    let (mut world, systems) = fixture_world(1);
    let system = systems[0];
    let yard = add_yard(&mut world, system, 0x2800_0001);
    let mine = add_mine(&mut world, system);
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let mut state = SystemWindowState::default();
    assert!(state.open_tab(
        &world,
        system,
        SystemWindowTab::Shipyards,
        ORIGIN,
        CockpitFaction::Alliance,
        layout,
    ));
    let fog = FogState::new(Faction::Alliance);
    let layer = egui::LayerId::new(egui::Order::Foreground, area_id(system));
    let pick = |state: &SystemWindowState, point| {
        state.mission_target(
            &world,
            &fog,
            &ManufacturingState::new(),
            &DeliveryState::new(),
            layout,
            layer,
            point,
        )
    };
    let cell = |state: &SystemWindowState, index| {
        state
            .cell_screen_rect(layout, system, index)
            .unwrap()
            .center()
    };
    let FacilityItem::Manufacturing(yard) = yard else {
        unreachable!()
    };
    let FacilityItem::Production(mine) = mine else {
        unreachable!()
    };

    assert_eq!(
        pick(&state, cell(&state, 0)).unwrap().unwrap().object,
        Some(MissionTarget::ManufacturingFacility(yard))
    );
    // The empty cell next to it, and the window's frame, give the system.
    assert_eq!(
        pick(&state, cell(&state, 1)),
        Some(Some(MissionTargetPick::from(system)))
    );
    let frame = state.cell_screen_rect(layout, system, 0).unwrap().min - egui::vec2(4.0, 4.0);
    assert_eq!(
        pick(&state, frame),
        Some(Some(MissionTargetPick::from(system)))
    );
    state.select_tab(system, SystemWindowTab::Mines);
    assert_eq!(
        pick(&state, cell(&state, 0)).unwrap().unwrap().object,
        Some(MissionTarget::ProductionFacility(mine))
    );
    // An empty deposit's cell drops the order (hyp: no object).
    assert_eq!(pick(&state, cell(&state, 1)), Some(None));
    // The overview is page 0x67: always the system.
    state.select_tab(system, SystemWindowTab::Overview);
    assert_eq!(
        pick(&state, cell(&state, 0)),
        Some(Some(MissionTargetPick::from(system)))
    );
    let elsewhere = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("elsewhere"));
    assert_eq!(
        state.mission_target(
            &world,
            &fog,
            &ManufacturingState::new(),
            &DeliveryState::new(),
            layout,
            elsewhere,
            cell(&state, 0),
        ),
        None
    );
}

fn hover(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
    vec![
        vec![egui::Event::PointerMoved(point)],
        vec![egui::Event::PointerMoved(point)],
    ]
}

fn click_with(point: egui::Pos2, button: egui::PointerButton) -> Vec<Vec<egui::Event>> {
    let event = |pressed| egui::Event::PointerButton {
        pos: point,
        button,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let mut frames = hover(point);
    frames.extend([vec![event(true)], vec![event(false)], vec![]]);
    frames
}

fn click(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
    click_with(point, egui::PointerButton::Primary)
}

fn right_click(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
    click_with(point, egui::PointerButton::Secondary)
}

fn text<'a>(run: &'a Run, words: &str) -> &'a RailText {
    run.texts
        .iter()
        .find(|value| value.0 == words)
        .unwrap_or_else(|| panic!("no {words:?} in {:?}", run.texts))
}

#[test]
fn opening_clamps_to_faction_galaxy_and_deduplicates() {
    // FUN_0045aac0 clamps the 226 by 304 window into the galaxy view:
    // 55 + 485 - 226 and 40 + 350 - 304.
    let (world, systems) = fixture_world(1);
    let mut state = SystemWindowState::default();
    let layout = layout(CockpitFaction::Alliance, 2.0);
    assert!(state.open(
        &world,
        systems[0],
        (620, 470),
        CockpitFaction::Alliance,
        layout
    ));
    assert_eq!(state.windows[0].logical_position, (314, 86));
    assert_eq!(state.windows[0].tab, SystemWindowTab::Overview);
    // FUN_0045aac0 tells the agent of a created window only.
    assert!(state.take_created());
    assert!(state.open(&world, systems[0], (0, 0), CockpitFaction::Alliance, layout));
    assert!(!state.take_created());
    assert_eq!(state.window_count(), 1);
}

#[test]
fn a_full_rail_puts_the_thirteenth_in_the_oldest_slot_and_a_restore_leaves_a_gap() {
    // FUN_00428b40, FUN_00429020; A0 personnel-states/32: a restored
    // window empties its slot and its next minimize fills that gap;
    // 33: the thirteenth (Xyquine) takes the oldest's slot (Uvena's,
    // the second), not the end.
    let (world, systems) = fixture_world(14);
    let mut state = SystemWindowState::default();
    let layout = layout(CockpitFaction::Alliance, 1.0);
    let slot = |state: &SystemWindowState, index: usize| state.rail[index].as_ref().map(RailEntry::system);
    for system in &systems[..12] {
        state.open(&world, *system, (100, 50), CockpitFaction::Alliance, layout);
        assert!(state.minimize(*system));
    }
    assert_eq!(state.rail_count(), REFERENCE_RAIL_SLOTS);
    // Restoring the first leaves its slot empty; the others keep theirs.
    assert!(state.restore(systems[0]));
    assert_eq!(slot(&state, 0), None);
    assert_eq!(slot(&state, 1), Some(systems[1]));
    assert_eq!(state.rail_slot_screen_rect(layout, 0), None);
    // Minimized again it fills the gap and becomes the newest.
    assert!(state.minimize(systems[0]));
    assert_eq!(slot(&state, 0), Some(systems[0]));
    // The thirteenth replaces the oldest, systems[1], in its slot.
    state.open(&world, systems[12], (100, 50), CockpitFaction::Alliance, layout);
    assert!(state.minimize(systems[12]));
    assert_eq!(slot(&state, 1), Some(systems[12]));
    assert_eq!(slot(&state, 0), Some(systems[0]));
    assert_eq!(slot(&state, 11), Some(systems[11]));
    // And the next replaces systems[2].
    state.open(&world, systems[13], (100, 50), CockpitFaction::Alliance, layout);
    assert!(state.minimize(systems[13]));
    assert_eq!(slot(&state, 2), Some(systems[13]));
    assert_eq!(state.rail_count(), REFERENCE_RAIL_SLOTS);
}

#[test]
fn restore_preserves_position_tab_and_selection() {
    let (mut world, systems) = fixture_world(1);
    let mine = add_mine(&mut world, systems[0]);
    let mut state = SystemWindowState::default();
    let layout = layout(CockpitFaction::Alliance, 1.0);
    state.open(
        &world,
        systems[0],
        (100, 60),
        CockpitFaction::Alliance,
        layout,
    );
    state.select_tab(systems[0], SystemWindowTab::Mines);
    state.select_item(systems[0], Some(mine));
    state.minimize(systems[0]);
    assert!(state.restore(systems[0]));
    assert_eq!(state.window_count(), 1);
    assert_eq!(state.rail_count(), 0);
    assert_eq!(state.windows[0].logical_position, (100, 60));
    assert_eq!(state.windows[0].tab, SystemWindowTab::Mines);
    assert_eq!(state.windows[0].selected_item, Some(mine));

    // The same page keeps the selection; another clears it.
    state.select_tab(systems[0], SystemWindowTab::Mines);
    assert_eq!(state.windows[0].selected_item, Some(mine));
    state.select_tab(systems[0], SystemWindowTab::Refineries);
    assert_eq!(state.windows[0].selected_item, None);
}

#[test]
fn faction_change_clears_visible_and_minimized_windows() {
    let (world, systems) = fixture_world(2);
    let mut state = SystemWindowState::default();
    let alliance = layout(CockpitFaction::Alliance, 1.0);
    state.open(
        &world,
        systems[0],
        (100, 60),
        CockpitFaction::Alliance,
        alliance,
    );
    state.open(
        &world,
        systems[1],
        (100, 60),
        CockpitFaction::Alliance,
        alliance,
    );
    state.minimize(systems[0]);
    state.prepare_faction(CockpitFaction::Empire);
    assert_eq!(state.window_count(), 0);
    assert_eq!(state.rail_count(), 0);
}

#[test]
fn each_tab_shows_its_side_art_pressed_or_empty() {
    // FUN_00455060: the overview's art by side; a page's normal and
    // pressed art, its empty art in place of the normal one.
    assert_eq!(
        [1, 2, 3, 0].map(|side| tab_resource(SystemWindowTab::Overview, side, false, true)),
        [10_312, 10_315, 10_318, 10_318]
    );
    assert_eq!(
        [1, 2, 3].map(|side| tab_resource(SystemWindowTab::Overview, side, true, false)),
        [10_311, 10_314, 10_317]
    );
    let pages = [
        (SystemWindowTab::Shipyards, [10_327, 10_326, 10_328]),
        (
            SystemWindowTab::TrainingFacilities,
            [10_330, 10_329, 10_331],
        ),
        (SystemWindowTab::ConstructionYards, [10_333, 10_332, 10_334]),
        (SystemWindowTab::Refineries, [10_324, 10_323, 10_325]),
        (SystemWindowTab::Mines, [10_321, 10_320, 10_322]),
    ];
    for (tab, [normal, pressed, empty]) in pages {
        assert_eq!(tab_resource(tab, 1, false, false), normal, "{tab:?}");
        assert_eq!(tab_resource(tab, 2, true, true), pressed, "{tab:?}");
        assert_eq!(tab_resource(tab, 3, false, true), empty, "{tab:?}");
    }
    // The help messages (TEXTSTRA 6197, 6184, 6192, 6194, 6183, 6182)
    // and the strip's x positions.
    assert_eq!(
        SystemWindowTab::ALL.map(SystemWindowTab::name),
        [
            "Manufacturing",
            "Shipyards",
            "Training Facilities",
            "Construction Yards",
            "Refineries",
            "Mines"
        ]
    );
    assert_eq!(
        SystemWindowTab::ALL.map(SystemWindowTab::x),
        [0.0, 39.0, 77.0, 115.0, 152.0, 190.0]
    );
}

#[test]
fn maps_original_system_item_resource_families_without_fallbacks() {
    // Source: GOKRES.DLL mini-icon resource blocks and DAT record families.
    assert_eq!(
        character_mini_resource_id(DatId::new(576), true),
        Some(18_496)
    );
    assert_eq!(
        character_mini_resource_id(DatId::new(641), true),
        Some(18_561)
    );
    assert_eq!(
        character_mini_resource_id(DatId::new(832), false),
        Some(19_008)
    );
    assert_eq!(
        character_mini_resource_id(DatId::new(923), false),
        Some(19_099)
    );

    assert_eq!(
        manufacturing_facility_mini(DatId::new(0x2800_0001)),
        Some((16_640, "Orbital Shipyard"))
    );
    assert_eq!(
        manufacturing_facility_mini(DatId::new(0x2a00_0006)),
        Some((16_645, "Advanced Construction Yard"))
    );
    assert_eq!(
        manufacturing_facility_mini(DatId::new(0x2000_0001)),
        Some((17_216, "Alliance Headquarters"))
    );
    assert_eq!(
        defense_facility_mini(DatId::new(0x2400_0003)),
        Some((16_898, "GenCore Level I"))
    );
    assert_eq!(
        defense_facility_mini(DatId::new(0x2500_0004)),
        Some((17_024, "Death Star Shield"))
    );
    assert_eq!(
        production_facility_mini(DatId::new(0x2c00_0001)),
        Some((16_385, "Mine"))
    );
    assert_eq!(
        production_facility_mini(DatId::new(0x2d00_0002)),
        Some((16_386, "Refinery"))
    );
    assert_eq!(
        troop_mini(DatId::new(0x1000_0001)),
        Some((17_472, "Alliance Fleet Regiment"))
    );
    assert_eq!(
        troop_mini(DatId::new(0x1000_000a)),
        Some((17_540, "Dark Trooper Regiment"))
    );
    assert_eq!(
        special_force_mini(DatId::new(0x3c00_0001)),
        Some((17_728, "Guerrillas"))
    );
    assert_eq!(
        special_force_mini(DatId::new(0x3c00_0009)),
        Some((17_796, "Bounty Hunters"))
    );

    assert_eq!(character_mini_resource_id(DatId::new(575), true), None);
    assert_eq!(manufacturing_facility_mini(DatId::new(0x2700_0001)), None);
    assert_eq!(defense_facility_mini(DatId::new(0x2400_0007)), None);
    assert_eq!(production_facility_mini(DatId::new(0x2d00_0003)), None);
    assert_eq!(troop_mini(DatId::new(0x1000_000b)), None);
    assert_eq!(special_force_mini(DatId::new(0x3c00_000a)), None);
}

#[test]
fn the_kind_claims_the_shared_focus_only_while_a_window_is_open() {
    // window_focus.rs: a kind with no window open makes no claim; one it
    // raises claims later than any stamp taken before.
    let (world, systems) = fixture_world(1);
    let mut state = SystemWindowState::default();
    assert_eq!(state.focus_stamp(), None);
    let mut earlier = crate::window_focus::FocusStamp::default();
    earlier.take();
    assert!(state.open(
        &world,
        systems[0],
        ORIGIN,
        CockpitFaction::Alliance,
        layout(CockpitFaction::Alliance, 2.0)
    ));
    let stamp = state.focus_stamp().expect("a window is open");
    assert!(stamp > earlier.stamp(), "{stamp} after {}", earlier.stamp());
    assert!(state.focus.holds());
    state.set_holds_focus(false);
    assert!(!state.focus.holds());
}

#[test]
fn the_window_draws_its_unfocused_strip_while_another_kind_holds_the_focus() {
    // FUN_00457690: +0x180 (side 1 10299) only while the window is the
    // galaxy view's focused child, else +0x184 (10200). A0
    // 2026-11-11-manufacturing-focus/02: a Defenses window opened
    // beside Averam turns Averam's strip to the unfocused art.
    let (mut world, systems) = fixture_world(1);
    add_yard(&mut world, systems[0], 0x2800_0001);
    let mut state = opened(&world, systems[0]);
    let mut defenses = crate::defenses_window::DefensesWindowState::default();
    defenses.open(
        &world,
        systems[0],
        ORIGIN,
        CockpitFaction::Alliance,
        layout(CockpitFaction::Alliance, 2.0),
    );
    let [system_focus, defenses_focus] =
        crate::window_focus::shared_focus([state.focus_stamp(), defenses.focus_stamp()]);
    assert_eq!((system_focus, defenses_focus), (false, true));
    state.set_holds_focus(system_focus);
    let run = run_with(
        &world,
        &ManufacturingState::new(),
        &mut state,
        hover(at(150.0, 10.0)),
    );
    assert!(
        run.painted.contains(&(10_200, at(2.0, 2.0))),
        "{:?}",
        run.painted
    );
    assert!(!run.painted.iter().any(|(id, _)| *id == 10_299));
}

#[test]
fn the_window_paints_its_chrome_tabs_and_the_overview_first() {
    // FUN_00455060 / FUN_00456230: 10297 at (0, 0), the focused side 1
    // strip at (2, 2), the title buttons at 3, 195 and 209, the six tabs
    // at (x, 20), the yard column at (6, 71), and each band's frame and
    // unselected side 1 strip at its corner (FUN_00458080).
    let (mut world, systems) = fixture_world(1);
    add_yard(&mut world, systems[0], 0x2800_0001);
    let mut state = opened(&world, systems[0]);
    let run = run_with(
        &world,
        &ManufacturingState::new(),
        &mut state,
        hover(at(150.0, 10.0)),
    );
    for (id, point) in [
        (WINDOW_BACKGROUND, at(0.0, 0.0)),
        (10_299, at(2.0, 2.0)),
        (SECTOR_NORMAL, at(3.0, 3.0)),
        (MINIMIZE_NORMAL, at(195.0, 3.0)),
        (CLOSE_NORMAL, at(209.0, 3.0)),
        (10_311, at(0.0, 20.0)),
        (10_327, at(39.0, 20.0)),
        (10_331, at(77.0, 20.0)),
        (10_334, at(115.0, 20.0)),
        (10_325, at(152.0, 20.0)),
        (10_322, at(190.0, 20.0)),
        (10_298, at(6.0, 71.0)),
        (10_290, at(55.0, 57.0)),
        (10_292, at(55.0, 57.0)),
        (10_292, at(55.0, 138.0)),
        (10_292, at(55.0, 219.0)),
    ] {
        assert!(
            run.painted.contains(&(id, point)),
            "{id} at {point:?} in {:?}",
            run.painted
        );
    }
    // The title at (19, 2), font 5; the bands' lines in font 10 from
    // (5, 1) and (5, 16) and (5, 57); the counts centred in the yard
    // column.
    let title = text(&run, "System 0");
    assert_eq!(
        (title.1.x, title.3),
        (at(19.0, 0.0).x, crate::theme::gdi_font(5, 2.0).size)
    );
    let heading = text(&run, "Ship Construction");
    // FUN_00458080: the title keeps the text object's black
    // (FUN_00601930), the lines are white; FUN_00457690's counts keep
    // the default font, a 16-row cell.
    let colour = |name: &str| {
        run.colours
            .iter()
            .find(|(text, _)| text == name)
            .map(|(_, colour)| *colour)
    };
    assert_eq!(colour("Ship Construction"), Some(egui::Color32::BLACK));
    assert_eq!(
        colour("No Ships are being built"),
        Some(egui::Color32::WHITE)
    );
    assert!(run.texts.iter().any(|value| value.0.contains(':')
        && value.3 == crate::theme::gdi_font(COUNT_FONT, 2.0).size));
    assert_eq!(
        (heading.1, heading.3),
        (
            line_at(60.0, 58.0, 10),
            crate::theme::gdi_font(10, 2.0).size
        )
    );
    assert_eq!(
        text(&run, "No Ships are being built").1,
        line_at(60.0, 73.0, 10)
    );
    assert_eq!(
        text(&run, "No Troops in training").1,
        line_at(60.0, 154.0, 10)
    );
    assert_eq!(
        text(&run, "Facilities Under Construction").1,
        line_at(60.0, 220.0, 10)
    );
    assert_eq!(
        run.texts
            .iter()
            .filter(|value| value.0 == "Destination: System 0")
            .count(),
        3
    );
    let ships = run
        .texts
        .iter()
        .find(|value| value.0 == "1:1")
        .unwrap_or_else(|| panic!("{:?}", run.texts));
    // DT_CENTER | DT_TOP in the 46-wide count box from (6, 119), the
    // default font: the baseline 13 below, the GDI width's spare halved.
    assert_eq!(ships.1.y, line_at(0.0, 119.0, COUNT_FONT).y);
    let (_, width) = crate::theme::gdi_pen_positions(COUNT_FONT, "1:1").unwrap();
    assert_eq!(ships.1.x, at(6.0 + ((46.0 - width) / 2.0).floor(), 0.0).x);
    assert_eq!(run.texts.iter().filter(|value| value.0 == "0:0").count(), 2);
    assert!(!run.texts.iter().any(|value| value.0 == "Manufacturing"));
}

#[test]
fn a_building_band_shows_its_product_and_units() {
    // FUN_00457c90: the product's name, its portrait (class +0x30 &
    // 0xfff) at (40, 15) and units at (5, 47).
    let (mut world, systems) = fixture_world(1);
    let class = world.capital_ship_classes.insert(CapitalShipClass {
        dat_id: DatId::new(0x1400_0040),
        name: "Corellian Corvette".into(),
        ..CapitalShipClass::default()
    });
    let mut manufacturing = ManufacturingState::new();
    manufacturing.build(
        systems[0],
        &QueueItem::new(BuildableKind::CapitalShip(class), 10).ordered(&mut world),
        2,
    );
    let mut state = opened(&world, systems[0]);
    let run = run_with(&world, &manufacturing, &mut state, hover(at(150.0, 10.0)));
    assert_eq!(
        text(&run, "Corellian Corvette 1").1,
        line_at(60.0, 73.0, 10)
    );
    assert_eq!(text(&run, "Building: 2").1, line_at(60.0, 104.0, 10));
    let portrait =
        crate::minis::capital_ship_mini_id(DatId::new(0x1400_0040)).unwrap() - 0x4000;
    assert!(
        run.painted.contains(&(portrait, at(95.0, 72.0))),
        "{:?}",
        run.painted
    );
    // FUN_004acec0: the progress bar's black, then its 0x200f0f0 part
    // (yellow) from the bar's left.
    // The area still fades in, so its colours are premultiplied.
    let bar = egui::Rect::from_min_size(at(56.0, 127.0), egui::vec2(320.0, 8.0));
    let yellow =
        |colour: egui::Color32| colour.r() > 0 && colour.r() == colour.g() && colour.b() == 0;
    assert!(run
        .fills
        .iter()
        .any(|(rect, colour)| *rect == bar && colour.r() == 0 && colour.b() == 0));
    assert!(
        run.fills.iter().any(|(rect, colour)| yellow(*colour)
            && rect.min == bar.min
            && rect.width() < bar.width()),
        "{:?}",
        run.fills
    );
}

/// Drall's Alliance Escort Carrier class with Escort Carriers 1..3
/// built and "Alliance Escort Carrier 4" in its shipyard's queue, its
/// window open (A0 2026-11-13-band-title-timing/00).
fn escort_carrier_at_drall() -> (
    GameWorld,
    SystemKey,
    rebellion_core::ids::CapitalShipKey,
    ManufacturingState,
    SystemWindowState,
) {
    let (mut world, systems) = fixture_world(1);
    let class = world.capital_ship_classes.insert(CapitalShipClass {
        dat_id: DatId::new(0x1400_0006),
        name: "Alliance Escort Carrier".into(),
        is_alliance: true,
        ..CapitalShipClass::default()
    });
    for _ in 0..3 {
        world.take_ship_number(true, DatId::new(0x1400_0006));
    }
    let mut manufacturing = ManufacturingState::new();
    manufacturing.build_units(
        systems[0],
        vec![QueueItem::new(BuildableKind::CapitalShip(class), 10).ordered(&mut world)],
    );
    let state = opened(&world, systems[0]);
    (world, systems[0], class, manufacturing, state)
}

/// Confirm `count` new units of `class` at `system`, as the app applies
/// it (build_selection::confirm_order, then the window hears of it).
fn confirm_carriers(
    world: &mut GameWorld,
    manufacturing: &mut ManufacturingState,
    state: &mut SystemWindowState,
    system: SystemKey,
    class: rebellion_core::ids::CapitalShipKey,
    count: u32,
) {
    let items = (0..count)
        .map(|_| QueueItem::new(BuildableKind::CapitalShip(class), 10).ordered(world))
        .collect();
    manufacturing.build_units(system, items);
    state.confirmed_build(
        world,
        manufacturing,
        system,
        ProductionArea::Shipyard,
        count,
    );
}

fn ship_line(
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    state: &mut SystemWindowState,
    system: SystemKey,
) -> String {
    state
        .shown_band(world, manufacturing, true, system, ProductionArea::Shipyard)
        .status
}

#[test]
fn a_single_unit_confirm_leaves_the_band_title_bare_until_the_window_is_rebuilt() {
    // FUN_00457b40 writes the line only when the band is built
    // (FUN_00458080, FUN_00457c90), never on a rename. A0
    // 2026-11-13-band-title-timing: 00 "Alliance Escort Carrier 4"; 03
    // right after a one-unit Confirm "Alliance Escort Carrier", which
    // 10..39 keep for 23 days while the progress bar grows.
    let (mut world, drall, class, mut manufacturing, mut state) = escort_carrier_at_drall();
    assert_eq!(
        ship_line(&world, &manufacturing, &mut state, drall),
        "Alliance Escort Carrier 4"
    );

    confirm_carriers(&mut world, &mut manufacturing, &mut state, drall, class, 1);
    assert_eq!(
        ship_line(&world, &manufacturing, &mut state, drall),
        "Alliance Escort Carrier"
    );
    let run = run_with(&world, &manufacturing, &mut state, hover(at(150.0, 10.0)));
    assert_eq!(
        text(&run, "Alliance Escort Carrier").1,
        line_at(60.0, 73.0, 10)
    );
    for _ in 0..3 {
        let _ = manufacturing
            .queue_mut(drall, ProductionArea::Shipyard)
            .add_point();
        assert_eq!(
            ship_line(&world, &manufacturing, &mut state, drall),
            "Alliance Escort Carrier"
        );
    }

    // port: a change of the units left rebuilds the band.
    manufacturing.enqueue(
        drall,
        QueueItem::new(BuildableKind::CapitalShip(class), 10).ordered(&mut world),
    );
    assert_eq!(
        ship_line(&world, &manufacturing, &mut state, drall),
        "Alliance Escort Carrier 5"
    );
}

#[test]
fn a_multi_unit_confirm_names_the_first_unit_with_its_number() {
    // A0 2026-11-11-icon-menu-topic/66: four Carrack Light Cruisers
    // confirmed, the band reads "Carrack Light Cruiser 2 / Building: 4"
    // at once (FUN_0052a140 numbers each unit at the order,
    // FUN_00517760).
    let (mut world, drall, class, mut manufacturing, mut state) = escort_carrier_at_drall();
    confirm_carriers(&mut world, &mut manufacturing, &mut state, drall, class, 3);
    let shown = state.shown_band(
        &world,
        &manufacturing,
        true,
        drall,
        ProductionArea::Shipyard,
    );
    assert_eq!(shown.status, "Alliance Escort Carrier 5");
    assert_eq!(shown.units.as_deref(), Some("Building: 3"));
    assert_eq!(
        state.band_status(drall, ProductionArea::Shipyard),
        Some("Alliance Escort Carrier 5")
    );
}

#[test]
fn each_band_of_each_window_keeps_its_own_title() {
    // Our own cache: a Confirm at one system leaves another window's
    // bands as they were built, and each band of a window its own line.
    let (mut world, systems) = fixture_world(2);
    let class = world.capital_ship_classes.insert(CapitalShipClass {
        dat_id: DatId::new(0x1400_0006),
        name: "Alliance Escort Carrier".into(),
        is_alliance: true,
        ..CapitalShipClass::default()
    });
    let mut manufacturing = ManufacturingState::new();
    let mut state = opened(&world, systems[0]);
    assert!(state.open(
        &world,
        systems[1],
        ORIGIN,
        CockpitFaction::Alliance,
        layout(CockpitFaction::Alliance, 2.0)
    ));
    for system in systems.iter().copied() {
        confirm_carriers(&mut world, &mut manufacturing, &mut state, system, class, 1);
        for area in BAND_AREAS {
            state.shown_band(&world, &manufacturing, true, system, area);
        }
    }
    confirm_carriers(&mut world, &mut manufacturing, &mut state, systems[1], class, 3);

    assert_eq!(
        ship_line(&world, &manufacturing, &mut state, systems[0]),
        "Alliance Escort Carrier"
    );
    assert_eq!(
        state.band_status(systems[0], ProductionArea::TrainingFacility),
        Some("No Troops in training")
    );
}

#[test]
fn a_reopened_manufacturing_window_names_the_current_product_with_its_number() {
    // A0 2026-11-13-band-title-timing/42: the window closed and opened
    // again (FUN_00458080 builds the band) reads "Alliance Escort
    // Carrier 5", the number its order took (41: Fleet 8 holds Escort
    // Carrier 5).
    let (mut world, drall, class, mut manufacturing, mut state) = escort_carrier_at_drall();
    confirm_carriers(&mut world, &mut manufacturing, &mut state, drall, class, 1);
    assert_eq!(
        ship_line(&world, &manufacturing, &mut state, drall),
        "Alliance Escort Carrier"
    );

    state.close(drall);
    assert!(state.open(
        &world,
        drall,
        ORIGIN,
        CockpitFaction::Alliance,
        layout(CockpitFaction::Alliance, 2.0)
    ));

    assert_eq!(
        ship_line(&world, &manufacturing, &mut state, drall),
        "Alliance Escort Carrier 5"
    );
}

#[test]
fn a_bands_progress_bar_fills_whole_pixels_rounded_down() {
    // The band's bar (Class_0065c058_vt14, FUN_004ad210) fills
    // (width * position) / range pixels in unsigned integer division:
    // A0 2026-11-11-manufacturing-focus/06 (Averam, day 38, an Alliance
    // Army Regiment of 6 points one point in) fills 26 of its 160, and
    // build-selection2/22 (Coruscant, three points of six) 80.
    for (done, pixels) in [(1, 26.0), (3, 80.0)] {
        let (world, systems) = fixture_world(1);
        let mut manufacturing = ManufacturingState::new();
        manufacturing.build(
            systems[0],
            &QueueItem::new(BuildableKind::Troop(DatId::new(0x1000_0002)), 6),
            1,
        );
        let queue = manufacturing.queue_mut(
            systems[0],
            rebellion_core::manufacturing::ProductionArea::TrainingFacility,
        );
        for _ in 0..done {
            let _ = queue.add_point();
        }
        let mut state = opened(&world, systems[0]);
        let run = run_with(&world, &manufacturing, &mut state, hover(at(150.0, 10.0)));
        // The troops band's bar at (56, 208), 160 by 4; the test draws
        // at scale 2.
        let bar = egui::Rect::from_min_size(at(56.0, 208.0), egui::vec2(320.0, 8.0));
        let yellow = |colour: egui::Color32| {
            colour.r() > 0 && colour.r() == colour.g() && colour.b() == 0
        };
        let filled = run
            .fills
            .iter()
            .find(|(rect, colour)| yellow(*colour) && rect.min == bar.min)
            .unwrap_or_else(|| panic!("{:?}", run.fills));
        assert_eq!(filled.0.width(), pixels * 2.0, "{done} of 6 points");
    }
}

#[test]
fn escape_build_and_stop_end_a_bands_selection_but_destination_and_a_click_beside_keep_it() {
    // A0 build-selection2/18 (Escape), 07 and 11 (Build, Stop), 21
    // (Destination), 23 (a click beside the menu).
    assert!(band_menu_deselects(None, true));
    assert!(!band_menu_deselects(None, false));
    assert!(band_menu_deselects(Some(ObjectMenuCommand::Build), false));
    assert!(band_menu_deselects(Some(ObjectMenuCommand::Stop), false));
    assert!(!band_menu_deselects(Some(ObjectMenuCommand::Destination), false));
    assert!(!band_menu_deselects(Some(ObjectMenuCommand::Reserved), false));
}

#[test]
fn a_band_menu_closing_leaves_no_band_selected_in_that_window_only() {
    // A0 build-selection2/17, 18: the facilities band lit for its menu,
    // then neither band after Escape.
    let (world, systems) = fixture_world(2);
    let mut state = opened(&world, systems[0]);
    state.open(
        &world,
        systems[1],
        (300, 76),
        CockpitFaction::Alliance,
        layout(CockpitFaction::Alliance, 1.0),
    );
    state.select_band(systems[0], ProductionArea::ConstructionYard);
    state.select_band(systems[1], ProductionArea::Shipyard);
    state.deselect_band(systems[0]);
    assert_eq!(
        state.selection(systems[0]),
        Some((SystemWindowTab::Overview, None))
    );
    assert_eq!(
        state.selection(systems[1]),
        Some((SystemWindowTab::Overview, Some(ProductionArea::Shipyard)))
    );
}

#[test]
fn a_press_on_a_band_selects_it_and_a_right_click_opens_its_menu() {
    // FUN_00458480's band rects; a press sets band +0x30 bit 0 and swaps
    // its strip (FUN_00456230); a right release opens the menu for the
    // selected managers (FUN_00453ee0, FUN_004ac5c0).
    let (world, systems) = fixture_world(1);
    let mut state = opened(&world, systems[0]);
    let manufacturing = ManufacturingState::new();
    let run = run_with(&world, &manufacturing, &mut state, click(at(100.0, 170.0)));
    assert_eq!(
        state.selection(systems[0]),
        Some((
            SystemWindowTab::Overview,
            Some(ProductionArea::TrainingFacility)
        ))
    );
    assert!(run
        .actions
        .contains(&SystemWindowAction::SelectSystem(systems[0])));
    assert!(!run
        .actions
        .iter()
        .any(|action| matches!(action, SystemWindowAction::OpenObjectMenu { .. })));
    let after = run_with(&world, &manufacturing, &mut state, hover(at(150.0, 10.0)));
    assert!(after.painted.contains(&(10_291, at(55.0, 138.0))));
    assert!(after.painted.contains(&(10_292, at(55.0, 57.0))));

    let menu = run_with(
        &world,
        &manufacturing,
        &mut state,
        right_click(at(100.0, 250.0)),
    );
    assert_eq!(
        state.selection(systems[0]),
        Some((
            SystemWindowTab::Overview,
            Some(ProductionArea::ConstructionYard)
        ))
    );
    assert!(menu.actions.contains(&SystemWindowAction::OpenObjectMenu {
        system: systems[0],
        selection: Some(MenuObject::Producer {
            system: systems[0],
            area: ProductionArea::ConstructionYard,
        }),
        point: (200, 310),
    }));
    assert_eq!(
        state.band_screen_rect(
            layout(CockpitFaction::Alliance, 2.0),
            systems[0],
            ProductionArea::Shipyard
        ),
        Some(egui::Rect::from_min_max(at(55.0, 57.0), at(221.0, 136.0)))
    );
}

/// A `width` by `height`, 8-bit BMP, every pixel palette entry 0.
fn solid_bmp(width: u32, height: u32) -> Vec<u8> {
    let stride = width.div_ceil(4) * 4;
    let offset = 14 + 40 + 4 * 2;
    let mut bytes = Vec::new();
    bytes.extend(b"BM");
    bytes.extend((offset + stride * height).to_le_bytes());
    bytes.extend([0; 4]);
    bytes.extend(offset.to_le_bytes());
    bytes.extend(40u32.to_le_bytes());
    bytes.extend(width.to_le_bytes());
    bytes.extend(height.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8u16.to_le_bytes());
    bytes.extend([0; 4]);
    bytes.extend((stride * height).to_le_bytes());
    bytes.extend([0; 8]);
    bytes.extend(2u32.to_le_bytes());
    bytes.extend([0; 4]);
    bytes.extend([0, 0, 255, 0, 0, 0, 0, 0]);
    bytes.extend(std::iter::repeat_n(0u8, (stride * height) as usize));
    bytes
}

/// Draw the opened window of a one-system world twice with `bitmaps`
/// (STRATEGY id, width, height) staged in a temporary base, and return
/// the clip rectangle of each mesh painted with one of `traced`'s
/// textures in the last frame.
#[cfg(not(target_arch = "wasm32"))]
fn traced_clips(name: &str, bitmaps: &[(u32, u32, u32)], traced: &[u32]) -> Vec<egui::Rect> {
    let base = std::env::temp_dir().join(format!(
        "open-rebellion-system-{name}-{}",
        std::process::id()
    ));
    let folder = base.join("strategy-dll").join("BMP");
    std::fs::create_dir_all(&folder).unwrap();
    for &(id, width, height) in bitmaps {
        std::fs::write(folder.join(format!("{id}.bmp")), solid_bmp(width, height)).unwrap();
    }
    let (world, systems) = fixture_world(1);
    let mut state = opened(&world, systems[0]);
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    cache.set_base_path(&base);
    let textures: Vec<egui::TextureId> = traced
        .iter()
        .filter_map(|&id| cache.get(&ctx, DllSource::Strategy, id).map(egui::TextureHandle::id))
        .collect();
    assert_eq!(textures.len(), traced.len());
    let fog = FogState::new(Faction::Alliance);
    let mut clips = Vec::new();
    for _ in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1000.0),
            )),
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            let _ = draw_system_windows(
                ctx,
                &world,
                &rebellion_core::movement::MovementState::default(),
                &fog,
                &MissionState::new(),
                &ManufacturingState::new(),
                &DeliveryState::new(),
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
            );
        });
        clips = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh) if textures.contains(&mesh.texture_id) => {
                    Some(clipped.clip_rect)
                }
                _ => None,
            })
            .collect();
    }
    std::fs::remove_dir_all(base).unwrap();
    clips
}

// A0 system-windows/07-icon-dbl: Yavin's red title strip (STRATEGY
// 10299, 231 wide at x 2) ends at x 223; columns 224 and 225 of its rows
// are the window's navy right border.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_title_strip_ends_inside_the_windows_right_border() {
    let titles = [10299, 10200, 10201, 10302, 10303, 10304];
    let bitmaps: Vec<_> = titles.iter().map(|&id| (id, 231, 18)).collect();
    let clips = traced_clips("title-strip", &bitmaps, &titles);
    assert_eq!(clips.len(), 1, "{clips:?}");
    assert!((clips[0].right() - at(224.0, 0.0).x).abs() < 0.01, "{clips:?}");
    assert!((clips[0].left() - at(0.0, 0.0).x).abs() < 0.01, "{clips:?}");
}

// A0 system-windows/07-icon-dbl: each title button's and tab's key
// pixel, its bottom-left corner, shows the window's background 10297
// ((9, 11, 32) at the buttons' (3, 16), (195, 16), (209, 16); white at
// the tabs' (x, 52)), not the title strip or the key colour.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_buttons_and_tabs_key_corners_show_the_windows_background() {
    let mut bitmaps = vec![(WINDOW_BACKGROUND, 226, 304)];
    for id in [SECTOR_NORMAL, MINIMIZE_NORMAL, CLOSE_NORMAL] {
        bitmaps.push((id, 14, 14));
    }
    for id in 10_311..=10_334 {
        bitmaps.push((id, 36, 33));
    }
    let clips = traced_clips("key-corners", &bitmaps, &[WINDOW_BACKGROUND]);
    let corner = |x: f32, y: f32| egui::Rect::from_min_max(at(x, y), at(x + 1.0, y + 1.0));
    let mut expected = vec![
        corner(SECTOR_X, 16.0),
        corner(MINIMIZE_X, 16.0),
        corner(CLOSE_X, 16.0),
    ];
    expected.extend(SystemWindowTab::ALL.map(|tab| corner(tab.x(), 52.0)));
    // The whole background first, then one clipped copy per corner.
    assert_eq!(clips.len(), 1 + expected.len(), "{clips:?}");
    for rect in expected {
        assert!(
            clips.iter().any(|clip| (clip.min - rect.min).length() < 0.01
                && (clip.max - rect.max).length() < 0.01),
            "{rect:?} not in {clips:?}"
        );
    }
}

#[test]
fn the_window_clips_its_title_strip_to_its_own_width() {
    // FUN_0045aac0 creates type 9 at 226 by 304, and its title strip
    // (STRATEGY 10299.., 231 wide at x 2) is the one the 235-wide Fleet,
    // Defenses and Missions windows use, so the window must cut it off.
    let (world, systems) = fixture_world(1);
    let mut state = opened(&world, systems[0]);
    let run = run_with(
        &world,
        &ManufacturingState::new(),
        &mut state,
        hover(at(150.0, 10.0)),
    );
    let name = &world.systems[systems[0]].name;
    let clip = run
        .clips
        .iter()
        .find(|(text, _)| text == name)
        .map(|(_, clip)| *clip)
        .expect("the title is drawn");
    let window =
        egui::Rect::from_min_max(at(0.0, 0.0), at(SYSTEM_WINDOW_WIDTH, SYSTEM_WINDOW_HEIGHT));
    assert!(window.contains_rect(clip), "{clip:?} in {window:?}");
    assert!((clip.right() - window.right()).abs() < 0.01, "{clip:?}");
}

// FUN_004ac5c0 opens the pop-up on the right release for slot 22's
// selection, the list's selected objects (FUN_00453ee0): a yard's menu.
#[test]
fn a_right_click_on_a_yard_opens_its_menu() {
    let (mut world, systems) = fixture_world(1);
    let yard = add_yard(&mut world, systems[0], 0x2800_0001);
    let FacilityItem::Manufacturing(key) = yard else {
        unreachable!()
    };
    let mut state = opened(&world, systems[0]);
    let manufacturing = ManufacturingState::new();
    let _ = run_with(&world, &manufacturing, &mut state, click(at(50.0, 30.0)));
    assert_eq!(state.windows[0].tab, SystemWindowTab::Shipyards);
    let right = run_with(
        &world,
        &manufacturing,
        &mut state,
        right_click(at(20.0, 90.0)),
    );
    assert_eq!(state.windows[0].selected_item, Some(yard));
    assert!(
        right.actions.contains(&SystemWindowAction::OpenObjectMenu {
            system: systems[0],
            selection: Some(MenuObject::ManufacturingFacility(key)),
            point: (120, 150),
        }),
        "{:?}",
        right.actions
    );
}

// FUN_004534f0 hands every right release (0x205) to the base procedure,
// which opens FUN_004ac5c0's pop-up for slot 22's selection; a press off
// the band rects (FUN_00458480) selects and clears nothing. A0
// menus3/34: a right click on Coruscant's window left of its bands, no
// band selected, opens Encyclopedia and Status, both grey.
#[test]
fn a_right_click_off_the_bands_opens_the_menu_for_the_selection_or_nothing() {
    let (mut world, systems) = fixture_world(1);
    let yard = add_yard(&mut world, systems[0], 0x2800_0001);
    let FacilityItem::Manufacturing(key) = yard else {
        unreachable!()
    };
    let mut state = opened(&world, systems[0]);
    let manufacturing = ManufacturingState::new();
    let menu_at = |run: &Run, selection, point| {
        run.actions.contains(&SystemWindowAction::OpenObjectMenu {
            system: systems[0],
            selection,
            point,
        })
    };

    // Beside the bands' pictures, nothing selected: the menu for nothing.
    let nothing = run_with(
        &world,
        &manufacturing,
        &mut state,
        right_click(at(20.0, 150.0)),
    );
    assert!(menu_at(&nothing, None, (120, 210)), "{:?}", nothing.actions);
    assert_eq!(
        state.selection(systems[0]),
        Some((SystemWindowTab::Overview, None))
    );
    // A left click there opens nothing.
    let left = run_with(&world, &manufacturing, &mut state, click(at(20.0, 150.0)));
    assert!(
        !left
            .actions
            .iter()
            .any(|action| matches!(action, SystemWindowAction::OpenObjectMenu { .. })),
        "{:?}",
        left.actions
    );

    // A band selected: the same spot opens its manager's menu and keeps it.
    let _ = run_with(&world, &manufacturing, &mut state, click(at(100.0, 170.0)));
    let band = run_with(
        &world,
        &manufacturing,
        &mut state,
        right_click(at(20.0, 150.0)),
    );
    assert!(
        menu_at(
            &band,
            Some(MenuObject::Producer {
                system: systems[0],
                area: ProductionArea::TrainingFacility,
            }),
            (120, 210)
        ),
        "{:?}",
        band.actions
    );
    assert_eq!(
        state.selection(systems[0]),
        Some((
            SystemWindowTab::Overview,
            Some(ProductionArea::TrainingFacility)
        ))
    );

    // A facility page: left of the list, the list's selected yard.
    let _ = run_with(&world, &manufacturing, &mut state, click(at(50.0, 30.0)));
    let _ = run_with(&world, &manufacturing, &mut state, click(at(20.0, 90.0)));
    assert_eq!(state.windows[0].selected_item, Some(yard));
    let page = run_with(
        &world,
        &manufacturing,
        &mut state,
        right_click(at(4.0, 150.0)),
    );
    assert!(
        menu_at(
            &page,
            Some(MenuObject::ManufacturingFacility(key)),
            (104, 210)
        ),
        "{:?}",
        page.actions
    );
    assert_eq!(state.windows[0].selected_item, Some(yard));
}

#[test]
fn a_facility_page_shows_its_label_pictures_and_selected_frame() {
    // FUN_004568a0: the page label at (0, 58), centred; the list at
    // (8, 77) in 69 by 40 cells, each picture 1 by 2 in; a selected
    // item adds its side frame (+0x15c). The mines page pads the
    // system's deposits with 9005.
    let (mut world, systems) = fixture_world(1);
    world.systems[systems[0]].raw_materials = 2;
    let mine = add_mine(&mut world, systems[0]);
    let mut state = opened(&world, systems[0]);
    let manufacturing = ManufacturingState::new();
    let _ = run_with(&world, &manufacturing, &mut state, click(at(200.0, 30.0)));
    assert_eq!(state.windows[0].tab, SystemWindowTab::Mines);

    let run = run_with(&world, &manufacturing, &mut state, click(at(20.0, 90.0)));
    assert_eq!(state.windows[0].selected_item, Some(mine));
    let after = run_with(&world, &manufacturing, &mut state, hover(at(150.0, 10.0)));
    assert!(
        after.painted.contains(&(9_001, at(9.0, 79.0))),
        "{:?}",
        after.painted
    );
    // The deposit's bitmap at its cell's corner: A0 manufacturing/05,
    // personnel-states/34.
    assert!(after.painted.contains(&(9_005, at(77.0, 77.0))), "{:?}", after.painted);
    assert!(after.painted.contains(&(10_262, at(9.0, 79.0))));
    assert!(after.painted.contains(&(10_320, at(190.0, 20.0))));
    let label = text(&after, "Mines");
    // DT_CENTER in the window's 226: the line's GDI width, its spare
    // halved in whole pixels.
    let (_, width) = crate::theme::gdi_pen_positions(4, "Mines").unwrap();
    assert_eq!(label.1.x, at(((226.0 - width) / 2.0).floor(), 0.0).x);
    // The page label `+0x5f` is font 4.
    assert_eq!(
        (label.1.y, label.3),
        (line_at(0.0, 58.0, 4).y, crate::theme::gdi_font(4, 2.0).size)
    );
    assert!(run
        .actions
        .contains(&SystemWindowAction::SelectSystem(systems[0])));

    // A right press selects too, and the release opens the menu for the
    // selection (FUN_004ac5c0, FUN_00453ee0). port: an empty deposit's
    // object is untraced, so its menu stands for nothing. A press on
    // empty list space clears the selection.
    let right = run_with(
        &world,
        &manufacturing,
        &mut state,
        right_click(at(80.0, 90.0)),
    );
    assert_eq!(
        state.windows[0].selected_item,
        Some(FacilityItem::Deposit(1))
    );
    assert!(right.actions.contains(&SystemWindowAction::OpenObjectMenu {
        system: systems[0],
        selection: None,
        point: (180, 150),
    }));
    let _ = run_with(&world, &manufacturing, &mut state, click(at(100.0, 250.0)));
    assert_eq!(state.windows[0].selected_item, None);
    assert_eq!(
        state.cell_screen_rect(layout(CockpitFaction::Alliance, 2.0), systems[0], 4),
        Some(egui::Rect::from_min_max(at(77.0, 117.0), at(146.0, 157.0)))
    );
}

/// Press at screen point `from`, move to `to` and hold; then release.
/// Returns the window's position while held and after the release.
fn move_window(
    world: &GameWorld,
    state: &mut SystemWindowState,
    from: egui::Pos2,
    to: egui::Pos2,
) -> ((i16, i16), (i16, i16)) {
    let manufacturing = ManufacturingState::new();
    let mut frames = hover(from);
    frames.extend([
        vec![egui::Event::PointerButton {
            pos: from,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::default(),
        }],
        vec![egui::Event::PointerMoved(to)],
        vec![egui::Event::PointerMoved(to)],
    ]);
    run_with(world, &manufacturing, state, frames);
    let held = state.windows[0].logical_position;
    let release = egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    };
    run_with(world, &manufacturing, state, vec![vec![release], vec![]]);
    (held, state.windows[0].logical_position)
}

// FUN_00452fc0 sets the window's +0xb8 bit 4, so a left press on its own
// surface holds it (FUN_00606ee0); it follows the pointer unbounded
// (FUN_006071e0) and the drop puts it back inside the galaxy view
// (FUN_00607010). A press on a band goes to the band.
#[test]
fn the_window_moves_by_its_surface_and_drops_inside_the_galaxy_view() {
    let (world, systems) = fixture_world(1);
    let mut state = opened(&world, systems[0]);
    // Hovering takes no focus.
    let manufacturing = ManufacturingState::new();
    let idle = run_with(&world, &manufacturing, &mut state, hover(at(100.0, 10.0)));
    assert!(idle.actions.is_empty());
    let (held, dropped) = move_window(&world, &mut state, at(100.0, 10.0), at(130.0, 30.0));
    assert_eq!((held, dropped), ((130, 80), (130, 80)));
    // Past the Alliance galaxy view's left edge (55) while held.
    let from = at(130.0, 30.0);
    let to = from - egui::vec2(200.0, 0.0);
    let (held, dropped) = move_window(&world, &mut state, from, to);
    assert_eq!((held, dropped), ((30, 80), (55, 80)));
    let window = window_screen_rect(state.windows[0], layout(CockpitFaction::Alliance, 2.0));
    let band = band_rect(window, 2.0, BAND_AREAS[0]).center();
    let (held, dropped) = move_window(&world, &mut state, band, band + egui::vec2(60.0, 60.0));
    assert_eq!((held, dropped), ((55, 80), (55, 80)));
}

#[test]
fn the_title_buttons_open_the_sector_minimize_and_close() {
    let (world, systems) = fixture_world(1);
    let manufacturing = ManufacturingState::new();
    let mut state = opened(&world, systems[0]);
    let sector = run_with(&world, &manufacturing, &mut state, click(at(9.0, 9.0)));
    assert!(sector
        .actions
        .contains(&SystemWindowAction::FocusSector(systems[0])));
    assert_eq!(state.window_count(), 0);

    let mut state = opened(&world, systems[0]);
    let _ = run_with(&world, &manufacturing, &mut state, click(at(201.0, 9.0)));
    assert_eq!((state.window_count(), state.rail_count()), (0, 1));

    let mut state = opened(&world, systems[0]);
    let _ = run_with(&world, &manufacturing, &mut state, click(at(215.0, 9.0)));
    assert_eq!((state.window_count(), state.rail_count()), (0, 0));
}

#[test]
fn the_other_sides_production_shows_only_where_its_contents_show() {
    // hyp: as the Defenses window's pages, the other side's facilities
    // and production show only where the player sees its objects.
    let (mut world, systems) = fixture_world(1);
    world.systems[systems[0]].control = ControlKind::Controlled(Faction::Empire);
    let class = world.capital_ship_classes.insert(CapitalShipClass {
        dat_id: DatId::new(0x1400_0040),
        name: "Star Destroyer".into(),
        ..CapitalShipClass::default()
    });
    let mut manufacturing = ManufacturingState::new();
    manufacturing.build(
        systems[0],
        &QueueItem::new(BuildableKind::CapitalShip(class), 10),
        1,
    );
    let mut state = opened(&world, systems[0]);
    let run = run_with(&world, &manufacturing, &mut state, hover(at(150.0, 10.0)));
    assert!(run
        .texts
        .iter()
        .any(|value| value.0 == "No Ships are being built"));
    assert!(!run.texts.iter().any(|value| value.0 == "Star Destroyer"));
    // The side 2 strip and tab art.
    assert!(run.painted.contains(&(10_294, at(55.0, 57.0))));
    assert!(run.painted.contains(&(10_314, at(0.0, 20.0))));
}

#[test]
fn a_minimized_defenses_window_keeps_its_own_rail_slot_and_restores_from_it() {
    // FUN_004aa4a0: a Defenses window minimizes (0x466) to its own rail
    // entry beside the same system's Fleet window, one per window; a
    // window already on the rail keeps its slot (FUN_00429020).
    let (world, systems) = fixture_world(1);
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let mut state = SystemWindowState::default();
    state.minimize_defenses_window(systems[0], (90, 70), crate::defenses_window::DefensesPage::Personnel);
    state.minimize_fleet_window(systems[0], (100, 80));
    state.minimize_defenses_window(systems[0], (90, 70), crate::defenses_window::DefensesPage::Personnel);
    assert_eq!(state.rail_count(), 2);
    assert_eq!(
        state.rail_entries().collect::<Vec<_>>(),
        [(0, systems[0], "defenses"), (1, systems[0], "fleet")]
    );

    let slot = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 0));
    assert_eq!(state.rail_slot_screen_rect(layout, 0), Some(slot));
    assert_eq!(state.rail_slot_screen_rect(layout, 2), None);
    let point = slot.center();
    let press = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
    let (actions, _) = run_rail(
        &world,
        &mut state,
        vec![
            vec![egui::Event::PointerMoved(point)],
            vec![egui::Event::PointerMoved(point)],
            // WM_LBUTTONDBLCLK, the second press, restores
            // (FUN_00422ce0 0x203).
            vec![press(true)],
            vec![press(false)],
            vec![press(true)],
        ],
    );
    let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.take());
    assert!(
        painted.contains(&(
            crate::defenses_window::rail_icon(&world, Faction::Alliance, systems[0]),
            slot.min
        )),
        "{painted:?}"
    );
    assert!(
        actions.contains(&SystemWindowAction::RestoreDefensesWindow {
            system: systems[0],
            logical_position: (90, 70),
            page: crate::defenses_window::DefensesPage::Personnel,
        })
    );
    assert_eq!(state.rail_count(), 1);
    assert!(matches!(state.rail[1], Some(RailEntry::Fleet { .. })));
}

#[test]
fn a_railed_defenses_window_keeps_its_page_and_its_icon_takes_it_back() {
    // A0 personnel-states/38: Mon Calamari's Defenses window, minimized
    // on Trooper Regiments, comes back on that page from its sector
    // icon's double click, and its rail slot empties.
    use crate::defenses_window::DefensesPage;
    let (_, systems) = fixture_world(2);
    let mut state = SystemWindowState::default();
    state.minimize_defenses_window(systems[0], (90, 70), DefensesPage::Regiments);
    state.minimize_defenses_window(systems[1], (40, 50), DefensesPage::Shields);
    assert_eq!(state.take_railed_defenses(systems[1]), Some(((40, 50), DefensesPage::Shields)));
    assert_eq!(state.rail_count(), 1);
    assert!(state.rail[1].is_none());
    assert_eq!(state.take_railed_defenses(systems[1]), None);
    state.minimize_fleet_window(systems[1], (60, 20));
    assert_eq!(state.take_railed_defenses(systems[1]), None);
    assert_eq!(state.take_railed_fleet(systems[1]), Some((60, 20)));
    assert!(state.rail[1].is_none());
    state.minimize_missions_window(systems[1], (30, 10));
    assert_eq!(state.take_railed_fleet(systems[1]), None);
    assert_eq!(state.take_railed_missions(systems[1]), Some((30, 10)));
    assert_eq!(state.take_railed_missions(systems[1]), None);
    assert!(state.rail[1].is_none());
    assert_eq!(
        state.rail[0],
        Some(RailEntry::Defenses {
            system: systems[0],
            logical_position: (90, 70),
            page: DefensesPage::Regiments,
        })
    );
}

#[test]
fn a_rail_double_click_pairs_presses_within_its_time_and_rectangle() {
    // Windows' defaults for WM_LBUTTONDBLCLK: 500 ms and 4 by 4 pixels.
    assert!(is_double_click(0.5, egui::vec2(2.0, -2.0)));
    assert!(!is_double_click(0.51, egui::Vec2::ZERO));
    assert!(!is_double_click(0.1, egui::vec2(2.5, 0.0)));
    assert!(!is_double_click(0.1, egui::vec2(0.0, -2.5)));
}

#[test]
fn a_press_off_the_rail_then_one_on_a_slot_restores_nothing() {
    // The minimize box's press and a slot's are not one double click
    // (they are in different windows; FUN_00422ce0 hears only 0x201).
    let (world, systems) = fixture_world(1);
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let mut state = SystemWindowState::default();
    state.minimize_defenses_window(systems[0], (90, 70), crate::defenses_window::DefensesPage::Personnel);
    let point = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 0)).center();
    let off = egui::pos2(point.x - 300.0, point.y + 200.0);
    let press = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let (actions, _) = run_rail(
        &world,
        &mut state,
        vec![
            vec![egui::Event::PointerMoved(off)],
            vec![press(off, true)],
            vec![press(off, false)],
            vec![egui::Event::PointerMoved(point)],
            vec![press(point, true)],
            vec![press(point, false)],
        ],
    );
    assert!(actions.is_empty(), "{actions:?}");
    assert_eq!(state.rail_count(), 1);
}

#[test]
fn a_double_click_inside_one_frame_restores_the_rail_window() {
    // Both presses of a fast WM_LBUTTONDBLCLK (FUN_00422ce0 0x203) can
    // reach one frame; the second still pairs with the first.
    let (world, systems) = fixture_world(1);
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let mut state = SystemWindowState::default();
    state.minimize_defenses_window(systems[0], (90, 70), crate::defenses_window::DefensesPage::Personnel);
    let point = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 0)).center();
    let press = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let (actions, _) = run_rail(
        &world,
        &mut state,
        vec![
            vec![egui::Event::PointerMoved(point)],
            vec![egui::Event::PointerMoved(point)],
            vec![press(true), press(false), press(true), press(false)],
        ],
    );
    assert_eq!(
        actions,
        [SystemWindowAction::RestoreDefensesWindow {
            system: systems[0],
            logical_position: (90, 70),
            page: crate::defenses_window::DefensesPage::Personnel,
        }]
    );
    assert_eq!(state.rail_count(), 0);
}

/// Runs `frames` of input on a rail holding one Defenses window and
/// returns the actions and the rail's remaining count.
fn rail_presses(
    frames: impl Fn(egui::Pos2) -> Vec<Vec<egui::Event>>,
) -> (Vec<SystemWindowAction>, usize) {
    let (world, systems) = fixture_world(1);
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let mut state = SystemWindowState::default();
    state.minimize_defenses_window(systems[0], (90, 70), crate::defenses_window::DefensesPage::Personnel);
    let point = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 0)).center();
    let (actions, _) = run_rail(&world, &mut state, frames(point));
    (actions, state.rail_count())
}

fn primary(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    }
}

#[test]
fn a_double_click_on_a_window_over_a_rail_slot_restores_nothing() {
    // The presses go to the window on top, not the rail (FUN_00422ce0
    // hears only its own messages).
    let (world, systems) = fixture_world(1);
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let mut state = SystemWindowState::default();
    state.minimize_defenses_window(systems[0], (90, 70), crate::defenses_window::DefensesPage::Personnel);
    let slot = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 0));
    let point = slot.center();
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    let fog = FogState::new(Faction::Alliance);
    let mut actions = Vec::new();
    for events in [
        vec![egui::Event::PointerMoved(point)],
        vec![egui::Event::PointerMoved(point)],
        vec![primary(point, true), primary(point, false)],
        vec![primary(point, true)],
    ] {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1000.0),
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            actions.extend(draw_system_windows(
                ctx,
                &world,
                &rebellion_core::movement::MovementState::default(),
                &fog,
                &MissionState::new(),
                &ManufacturingState::new(),
                &DeliveryState::new(),
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
            ));
            egui::Area::new(egui::Id::new("window-over-the-rail"))
                .fixed_pos(slot.min)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.allocate_exact_size(slot.size(), egui::Sense::click());
                });
        });
    }
    assert!(actions.is_empty(), "{actions:?}");
    assert_eq!(state.rail_count(), 1);
}

#[test]
fn a_late_double_click_pairs_presses_a_scaled_pixel_apart() {
    // SM_CXDOUBLECLK counts original pixels: three screen pixels on the
    // scale 2 layout are 1.5 of them. Forty frames (two thirds of a
    // second) pass first, so the pair is timed from the first press.
    let (actions, left) = rail_presses(|point| {
        let mut frames = vec![vec![egui::Event::PointerMoved(point)]; 40];
        let near = point + egui::vec2(3.0, -3.0);
        frames.extend([
            vec![primary(point, true)],
            vec![primary(point, false)],
            vec![egui::Event::PointerMoved(near)],
            vec![primary(near, true)],
        ]);
        frames
    });
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(left, 0);
}

#[test]
fn presses_more_than_half_a_second_apart_restore_nothing() {
    // GetDoubleClickTime's default is 500 ms: forty frames lie between.
    let (actions, left) = rail_presses(|point| {
        let mut frames = vec![
            vec![egui::Event::PointerMoved(point)],
            vec![egui::Event::PointerMoved(point)],
            vec![primary(point, true)],
            vec![primary(point, false)],
        ];
        frames.extend(vec![vec![egui::Event::PointerMoved(point)]; 40]);
        frames.push(vec![primary(point, true)]);
        frames
    });
    assert!(actions.is_empty(), "{actions:?}");
    assert_eq!(left, 1);
}

#[test]
fn a_single_click_on_a_rail_slot_restores_nothing() {
    // FUN_00422ce0 restores only on WM_LBUTTONDBLCLK (0x203); A0
    // system-windows/10-restored: a click leaves the slot.
    let (world, systems) = fixture_world(1);
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let mut state = SystemWindowState::default();
    state.minimize_defenses_window(systems[0], (90, 70), crate::defenses_window::DefensesPage::Personnel);
    let point = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 0)).center();
    let press = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let (actions, _) = run_rail(
        &world,
        &mut state,
        vec![
            vec![egui::Event::PointerMoved(point)],
            vec![egui::Event::PointerMoved(point)],
            vec![press(true)],
            vec![press(false)],
        ],
    );
    assert!(actions.is_empty(), "{actions:?}");
    assert_eq!(state.rail_count(), 1);
}

#[test]
fn a_minimized_missions_window_keeps_its_own_rail_slot_and_restores_from_it() {
    // FUN_004a21c0: a Missions window minimizes (0x466) to its own rail
    // entry beside the same system's Defenses window, one per window;
    // with no mission there its icon is side 3's 11541.
    let (world, systems) = fixture_world(1);
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let mut state = SystemWindowState::default();
    state.minimize_defenses_window(systems[0], (90, 70), crate::defenses_window::DefensesPage::Personnel);
    state.minimize_missions_window(systems[0], (60, 40));
    state.minimize_missions_window(systems[0], (60, 40));
    assert_eq!(
        state.rail_entries().collect::<Vec<_>>(),
        [(0, systems[0], "defenses"), (1, systems[0], "missions")]
    );

    let slot = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 1));
    let point = slot.center();
    let press = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
    let (actions, _) = run_rail(
        &world,
        &mut state,
        vec![
            vec![egui::Event::PointerMoved(point)],
            vec![egui::Event::PointerMoved(point)],
            // WM_LBUTTONDBLCLK, the second press, restores
            // (FUN_00422ce0 0x203).
            vec![press(true)],
            vec![press(false)],
            vec![press(true)],
        ],
    );
    let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.take());
    assert!(painted.contains(&(11541, slot.min)), "{painted:?}");
    assert!(
        actions.contains(&SystemWindowAction::RestoreMissionsWindow {
            system: systems[0],
            logical_position: (60, 40),
        })
    );
    assert_eq!(state.rail_count(), 1);
    assert!(matches!(state.rail[0], Some(RailEntry::Defenses { .. })));
}

#[test]
fn a_minimized_fleet_window_keeps_one_rail_slot_beside_its_systems_window() {
    // FUN_004a76e0: a Fleet window minimizes to the rail as its own
    // entry; the rail holds one entry per window.
    let (world, systems) = fixture_world(2);
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let mut state = SystemWindowState::default();
    state.minimize_fleet_window(systems[0], (100, 80));
    state.minimize_fleet_window(systems[0], (100, 80));
    assert_eq!(state.rail_count(), 1);
    state.open(
        &world,
        systems[0],
        (60, 40),
        CockpitFaction::Alliance,
        layout,
    );
    assert!(state.minimize(systems[0]));
    state.minimize_fleet_window(systems[1], (120, 90));
    assert_eq!(state.rail_count(), 3);
    assert_eq!(
        state
            .rail_entries()
            .map(|(_, _, kind)| kind)
            .collect::<Vec<_>>(),
        ["fleet", "system", "fleet"]
    );

    // FUN_00428d70: each slot draws its window's icon (slot +0x74) at the
    // corner and the system's name after the icon in font 10; the System
    // window's icon is 11530 for side 1 (FUN_00458cf0). The fixture has
    // no bitmaps, so the icon's width is 0 here.
    let slot = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 0));
    let second = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 1));
    let away = egui::pos2(2.0, 2.0);
    let run = run_with(
        &world,
        &ManufacturingState::new(),
        &mut state,
        vec![
            vec![egui::Event::PointerMoved(away)],
            vec![egui::Event::PointerMoved(away)],
        ],
    );
    let font = crate::theme::gdi_font(10, 2.0).size;
    // DT_TOP: the baseline tmAscent (11) below the slot's top.
    for corner in [slot.min, second.min].map(|corner| corner + egui::vec2(0.0, 22.0)) {
        assert!(
            run.texts
                .iter()
                .any(|(text, pos, _, size)| text == "System 0"
                    && *pos == corner
                    && *size == font),
            "{:?}",
            run.texts
        );
    }
    assert!(run
        .painted
        .iter()
        .any(|&(id, pos)| id == RAIL_ICONS[0] && pos == second.min));

    let point = slot.center();
    let press = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let (actions, _) = run_rail(
        &world,
        &mut state,
        vec![
            vec![egui::Event::PointerMoved(point)],
            vec![egui::Event::PointerMoved(point)],
            // WM_LBUTTONDBLCLK, the second press, restores
            // (FUN_00422ce0 0x203).
            vec![press(true)],
            vec![press(false)],
            vec![press(true)],
        ],
    );
    assert!(actions.contains(&SystemWindowAction::RestoreFleetWindow {
        system: systems[0],
        logical_position: (100, 80),
    }));
    // The restored window's slot stays empty (A0 personnel-states/32).
    assert_eq!(state.rail_count(), 2);
    assert!(state.rail[0].is_none());
    assert!(state.rail[1].as_ref().unwrap().is_system_window());
    assert_eq!(state.rail[1].as_ref().unwrap().system(), systems[0]);
    assert_eq!(state.rail[2].as_ref().unwrap().system(), systems[1]);
}
// FUN_00458cf0: a System window's rail icon by its system's side bits.
#[test]
fn a_system_windows_rail_icon_follows_the_systems_side() {
    let (mut world, systems) = fixture_world(1);
    let mut icon = |control| {
        world.systems[systems[0]].control = control;
        rail_icon(&world, systems[0])
    };
    assert_eq!(icon(ControlKind::Controlled(Faction::Alliance)), 11530);
    assert_eq!(icon(ControlKind::Controlled(Faction::Empire)), 11531);
    assert_eq!(icon(ControlKind::Uncontrolled), 11532);
    assert_eq!(icon(ControlKind::Contested), 11532);
}

#[test]
fn reference_rail_rectangles_differ_by_faction() {
    // The galaxy view's SetRect calls at 0x421f3b (side 1, 544..606) and
    // 0x42207e (side 2, 21..75), 18 rows high.
    assert_eq!(
        rail_slot_rect(CockpitFaction::Alliance, 0),
        (544.0, 61.0, 62.0, 18.0)
    );
    assert_eq!(
        rail_slot_rect(CockpitFaction::Alliance, 11),
        (544.0, 301.0, 62.0, 18.0)
    );
    assert_eq!(
        rail_slot_rect(CockpitFaction::Empire, 0),
        (21.0, 48.0, 54.0, 18.0)
    );
    assert_eq!(
        rail_slot_rect(CockpitFaction::Empire, 11),
        (21.0, 319.0, 54.0, 18.0)
    );
}

#[test]
fn a_window_opens_on_the_tab_it_is_asked_for() {
    // Our own: a window opened on a page.
    let (mut world, systems) = fixture_world(2);
    let layout = layout(CockpitFaction::Alliance, 2.0);
    let mut state = SystemWindowState::default();
    let missing = systems[1];
    world.systems.remove(missing);

    assert!(state.open_tab(
        &world,
        systems[0],
        SystemWindowTab::Refineries,
        (60, 40),
        CockpitFaction::Alliance,
        layout
    ));
    assert_eq!(state.windows[0].tab, SystemWindowTab::Refineries);
    assert!(!state.open_tab(
        &world,
        missing,
        SystemWindowTab::Refineries,
        (60, 40),
        CockpitFaction::Alliance,
        layout
    ));
    assert_eq!(state.windows.len(), 1);

    // FUN_0060d590: the tab buttons sit at (x, 20), 36 by 33.
    let origin = window_screen_rect(state.windows[0], layout).min;
    assert_eq!(
        state.tab_screen_rect(layout, systems[0], SystemWindowTab::Refineries),
        Some(egui::Rect::from_min_size(
            origin + egui::vec2(152.0, 20.0) * 2.0,
            egui::vec2(72.0, 66.0)
        ))
    );
    assert_eq!(
        state.tab_screen_rect(layout, missing, SystemWindowTab::Refineries),
        None
    );
}

#[test]
fn system_window_occlusion_uses_scaled_exclusive_edges() {
    // FUN_0045aac0: 226 by 304 at (100, 60), scale 2, canvas at (10, 20).
    let (world, systems) = fixture_world(1);
    let mut state = SystemWindowState::default();
    let layout = layout(CockpitFaction::Alliance, 2.0);
    state.open(
        &world,
        systems[0],
        (100, 60),
        CockpitFaction::Alliance,
        layout,
    );
    assert!(state.contains_screen_point(layout, (210.0, 140.0)));
    assert!(state.contains_screen_point(layout, (661.9, 747.9)));
    assert!(!state.contains_screen_point(layout, (662.0, 140.0)));
    assert!(!state.contains_screen_point(layout, (210.0, 748.0)));
    let layer = egui::LayerId::new(egui::Order::Foreground, area_id(systems[0]));
    assert_eq!(state.release_target(layer), Some(systems[0]));
}
