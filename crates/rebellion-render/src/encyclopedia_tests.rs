#[test]
fn each_topic_shown_starts_its_body_at_the_top() {
    let mut state = EncyclopediaState::default();
    state.follow_topic("A-wing");
    state.original_topic_scroll.0 = 32.0;
    state.follow_topic("A-wing");
    assert_eq!(
        state.original_topic_scroll,
        (32.0, Some("A-wing".to_owned()))
    );
    state.follow_topic("Abduction");
    assert_eq!(
        state.original_topic_scroll,
        (0.0, Some("Abduction".to_owned()))
    );
    assert!(state.original_topic_scroll_hold.is_none());
}

#[test]
fn each_control_has_its_tooltip_string() {
    // FUN_0045ddc0 (A0 tooltip/101..116).
    let tips: Vec<_> = [
        0xfb, 0x67, 0x68, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x83, 0x84,
    ]
    .into_iter()
    .map(super::encyclopedia_tooltip_string)
    .collect();
    assert_eq!(
        tips,
        [
            0x1954, 0x1860, 0x1859, 0x1850, 0x1855, 0x1854, 0x1852, 0x1851, 0x1856, 0x1853,
            0x1858, 0x1857
        ]
        .map(Some)
    );
    assert_eq!(super::encyclopedia_tooltip_string(0x76), None);
}

use super::*;

fn synthetic_indexed_edata() -> Vec<u8> {
    const WIDTH: usize = 400;
    const HEIGHT: usize = 200;
    const PIXEL_OFFSET: usize = 1_078;
    let mut bytes = vec![0_u8; PIXEL_OFFSET + WIDTH * HEIGHT];
    let file_size = bytes.len() as u32;
    bytes[0..2].copy_from_slice(b"BM");
    bytes[2..6].copy_from_slice(&file_size.to_le_bytes());
    bytes[10..14].copy_from_slice(&(PIXEL_OFFSET as u32).to_le_bytes());
    bytes[14..18].copy_from_slice(&40_u32.to_le_bytes());
    bytes[18..22].copy_from_slice(&(WIDTH as i32).to_le_bytes());
    bytes[22..26].copy_from_slice(&(HEIGHT as i32).to_le_bytes());
    bytes[26..28].copy_from_slice(&1_u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&8_u16.to_le_bytes());
    bytes[34..38].copy_from_slice(&((WIDTH * HEIGHT) as u32).to_le_bytes());
    bytes[46..50].copy_from_slice(&256_u32.to_le_bytes());
    bytes[54 + 4..54 + 8].copy_from_slice(&[0x20, 0x80, 0xe0, 0]);
    bytes[PIXEL_OFFSET..].fill(1);
    bytes
}

#[test]
fn encyclopedia_defaults_to_original_parity() {
    let state = EncyclopediaState::new();
    assert_eq!(state.asset_profile, AssetRenderProfile::OriginalParity);
}

#[test]
fn changing_asset_profile_invalidates_cached_images() {
    let mut state = EncyclopediaState::new();
    state.textures.insert(42, None);

    state.set_asset_profile(AssetRenderProfile::FaithfulHd);

    assert!(state.textures.is_empty());
    assert_eq!(state.asset_profile, AssetRenderProfile::FaithfulHd);
}

#[test]
fn edata_filenames_preserve_the_original_three_digit_identity() {
    assert_eq!(edata_filename(1), "EDATA.001");
    assert_eq!(edata_filename(42), "EDATA.042");
    assert_eq!(edata_filename(192), "EDATA.192");
}

#[test]
fn original_edata_bytes_decode_at_source_size() {
    let ctx = egui::Context::default();
    let texture = load_image_bytes(
        &ctx,
        42,
        &synthetic_indexed_edata(),
        TextureOptions::NEAREST,
    )
    .expect("synthetic original EDATA should decode");

    assert_eq!(texture.size(), [400, 200]);
}

#[test]
fn original_index_categories_match_the_recovered_command_geometry() {
    let controls = encyclopedia_category_controls(crate::cockpit::CockpitFaction::Alliance);

    assert_eq!(
        controls.map(|control| (
            control.command_id,
            control.x,
            control.y,
            control.width,
            control.height,
        )),
        [
            (0x6f, 36, 78, 49, 41),
            (0x70, 88, 78, 49, 41),
            (0x71, 140, 78, 49, 41),
            (0x72, 192, 78, 49, 41),
            (0x73, 244, 78, 49, 41),
            (0x74, 296, 78, 49, 41),
            (0x75, 348, 78, 49, 41),
        ]
    );
}

#[test]
fn original_index_category_resources_preserve_faction_variants() {
    let alliance = encyclopedia_category_controls(crate::cockpit::CockpitFaction::Alliance);
    let empire = encyclopedia_category_controls(crate::cockpit::CockpitFaction::Empire);

    assert_eq!(
        alliance.map(|control| (control.normal_resource, control.pressed_resource)),
        [
            (10_340, 10_339),
            (10_350, 10_349),
            (10_348, 10_347),
            (10_344, 10_343),
            (11_616, 11_615),
            (10_352, 10_351),
            (10_346, 10_345),
        ]
    );
    assert_eq!(
        empire.map(|control| (control.normal_resource, control.pressed_resource)),
        [
            (10_340, 10_339),
            (10_350, 10_349),
            (10_360, 10_359),
            (10_356, 10_355),
            (11_618, 11_617),
            (10_362, 10_361),
            (10_358, 10_357),
        ]
    );
}

#[test]
fn original_index_rail_preserves_faction_geometry_and_selected_mode() {
    let alliance = encyclopedia_rail_controls(crate::cockpit::CockpitFaction::Alliance);
    let empire = encyclopedia_rail_controls(crate::cockpit::CockpitFaction::Empire);

    assert_eq!(
        alliance.map(|control| (
            control.command_id,
            control.x,
            control.y,
            control.width,
            control.height,
            control.normal_resource,
            control.pressed_resource,
            control.selected_in_index,
        )),
        [
            (0xfb, 423, 25, 32, 31, 10_370, 10_371, false),
            (0x67, 423, 93, 32, 31, 10_374, 10_375, false),
            (0x68, 423, 147, 32, 31, 10_372, 10_373, true),
        ]
    );
    assert_eq!(
        empire.map(|control| (
            control.command_id,
            control.x,
            control.y,
            control.width,
            control.height,
            control.normal_resource,
            control.pressed_resource,
            control.selected_in_index,
        )),
        [
            (0xfb, 426, 21, 44, 41, 10_376, 10_377, false),
            (0x67, 426, 89, 44, 41, 10_380, 10_381, false),
            (0x68, 426, 143, 44, 41, 10_378, 10_379, true),
        ]
    );
}

fn original_catalog_fixture() -> OriginalEncyclopediaCatalog {
    OriginalEncyclopediaCatalog::new(
        "Galactic Encyclopedia".into(),
        "Topic".into(),
        [
            "All Databases".into(),
            "System Database".into(),
            "Ship Database".into(),
            "Facilities Database".into(),
            "Missions Database".into(),
            "Troop Database".into(),
            "Personnel Database".into(),
        ],
        vec![
            OriginalEncyclopediaEntry {
                object_id: 0x9000_0001,
                name: "Allyuen".into(),
            },
            OriginalEncyclopediaEntry {
                object_id: 0x1400_0001,
                name: "Alliance Dreadnaught".into(),
            },
            OriginalEncyclopediaEntry {
                object_id: 0x1000_0001,
                name: "Alliance Army Regiment".into(),
            },
            OriginalEncyclopediaEntry {
                object_id: 0x3000_0001,
                name: "Ackbar".into(),
            },
        ],
    )
}

#[test]
fn original_catalog_uses_source_labels_and_family_filters() {
    let catalog = original_catalog_fixture();

    assert_eq!(catalog.len(), 4);
    assert!(!catalog.is_empty());
    assert_eq!(catalog.category_label(0x6f), Some("All Databases"));
    assert_eq!(catalog.category_label(0x75), Some("Personnel Database"));
    assert_eq!(catalog.entries_for(0x6f).len(), 4);
    assert_eq!(catalog.entries_for(0x70)[0].name, "Allyuen");
    assert_eq!(catalog.entries_for(0x71)[0].name, "Alliance Dreadnaught");
    assert_eq!(catalog.entries_for(0x74)[0].name, "Alliance Army Regiment");
    assert_eq!(catalog.entries_for(0x75)[0].name, "Ackbar");
}

#[test]
fn original_catalog_selection_preserves_identity_or_clears() {
    let catalog = original_catalog_fixture();
    let mut state = EncyclopediaState {
        original_selected_object_id: Some(0x1400_0001),
        ..EncyclopediaState::default()
    };

    reconcile_original_index_state(&mut state, &catalog, false);
    assert_eq!(state.original_selected_object_id, Some(0x1400_0001));

    state.original_category_command = 0x71;
    reconcile_original_index_state(&mut state, &catalog, true);
    assert_eq!(state.original_selected_object_id, Some(0x1400_0001));

    // Reconciling alone drops an object the list does not hold; the
    // category command then chooses the first row
    // (`a_category_keeps_its_object_or_chooses_its_first_row`).
    state.original_category_command = 0x70;
    reconcile_original_index_state(&mut state, &catalog, true);
    assert_eq!(state.original_selected_object_id, None);
    assert_eq!(state.original_scroll, 0);
}

#[test]
fn a_category_keeps_its_object_or_chooses_its_first_row() {
    // A0 encyclopedia/category-1: System Database chooses its first row
    // under an empty Topic box; ex 2, 3: with Adar Tallon chosen,
    // Personnel Database keeps him and System Database chooses Adega,
    // the box still "Adar Tallon".
    let catalog = original_catalog_fixture();
    let entries = |command| -> Vec<u32> {
        catalog.entries_for(command).iter().map(|entry| entry.object_id).collect()
    };
    let mut state = EncyclopediaState::default();
    choose_category_row(&mut state, &catalog);
    assert_eq!(state.original_selected_object_id, None, "All Databases chooses nothing");

    state.original_category_command = 0x70;
    choose_category_row(&mut state, &catalog);
    assert_eq!(state.original_selected_object_id, entries(0x70).first().copied());
    assert_eq!(state.original_topic_text.as_deref(), Some(""));

    let ship = entries(0x71)[0];
    let mut state = EncyclopediaState {
        original_selected_object_id: Some(ship),
        original_category_command: 0x71,
        ..EncyclopediaState::default()
    };
    choose_category_row(&mut state, &catalog);
    assert_eq!(state.original_selected_object_id, Some(ship), "the category holds it");
    assert_eq!(state.original_topic_text, None);

    state.original_category_command = 0x70;
    choose_category_row(&mut state, &catalog);
    assert_eq!(state.original_selected_object_id, entries(0x70).first().copied());
    let name = catalog
        .entries_for(0x6f)
        .into_iter()
        .find(|entry| entry.object_id == ship)
        .map(|entry| entry.name.clone());
    assert_eq!(state.original_topic_text, name);
}

#[test]
fn a_category_chooses_the_row_the_topic_box_best_begins() {
    // FUN_0045da70 case 0x6e: FUN_00609650 on the Topic box's text
    // (+0x128), which still names Corran Horn: "Co" is shared with
    // Corellia only, so it beats the first row, Adega.
    let catalog = OriginalEncyclopediaCatalog::new(
        "Galactic Encyclopedia".into(),
        "Topic".into(),
        std::array::from_fn(|index| format!("Category {index}")),
        vec![
            OriginalEncyclopediaEntry {
                object_id: 0x9000_0001,
                name: "Adega".into(),
            },
            OriginalEncyclopediaEntry {
                object_id: 0x9000_0002,
                name: "Alderaan".into(),
            },
            OriginalEncyclopediaEntry {
                object_id: 0x9000_0003,
                name: "Corellia".into(),
            },
            OriginalEncyclopediaEntry {
                object_id: 0x3000_0001,
                name: "Corran Horn".into(),
            },
        ],
    );
    let mut state = EncyclopediaState {
        original_selected_object_id: Some(0x3000_0001),
        ..EncyclopediaState::default()
    };
    change_category(&mut state, &catalog, 0x70);
    assert_eq!(state.original_selected_object_id, Some(0x9000_0003));
    assert_eq!(state.original_topic_text.as_deref(), Some("Corran Horn"));
}

#[test]
fn a_category_keeps_the_lists_place_while_its_row_shows() {
    // FUN_0045f100 never sets the first visible pixel; FUN_00609ae0
    // clamps it and FUN_00609650 scrolls only as its row needs.
    let entries: Vec<OriginalEncyclopediaEntry> = (0..20)
        .map(|index| OriginalEncyclopediaEntry {
            object_id: 0x9000_0001 + index,
            name: format!("S{index:02}"),
        })
        .collect();
    let catalog = OriginalEncyclopediaCatalog::new(
        "Galactic Encyclopedia".into(),
        "Topic".into(),
        std::array::from_fn(|index| format!("Category {index}")),
        entries,
    );
    // Row 6 (120..140) shows at a scroll of 60 (60..220).
    let mut state = EncyclopediaState {
        original_selected_object_id: Some(0x9000_0007),
        original_scroll: 60,
        ..EncyclopediaState::default()
    };
    change_category(&mut state, &catalog, 0x70);
    assert_eq!(state.original_selected_object_id, Some(0x9000_0007));
    assert_eq!(state.original_scroll, 60);
    // A row above the shown part is scrolled to, as little as it needs.
    state.original_scroll = 150;
    change_category(&mut state, &catalog, 0x6f);
    assert_eq!(state.original_scroll, 120);
    // The shown category's icon, and any other command, leave the list.
    state.original_scroll = 150;
    change_category(&mut state, &catalog, 0x6f);
    assert_eq!(state.original_scroll, 150);
    change_category(&mut state, &catalog, 0x67);
    assert_eq!((state.original_category_command, state.original_scroll), (0x6f, 150));
}

#[test]
fn opening_starts_at_all_databases_with_nothing_selected() {
    // Each opening builds a new window (FUN_0045d400), whose list starts
    // at FUN_0045f100(0x6f, 1).
    let catalog = original_catalog_fixture();
    let mut state = EncyclopediaState {
        original_category_command: 0x71,
        original_selected_object_id: Some(0x1400_0001),
        original_scroll: 2,
        ..EncyclopediaState::default()
    };
    open_original_encyclopedia_at(&mut state, &catalog, None);
    assert!(state.open && !state.original_topic_mode);
    assert_eq!(state.original_category_command, 0x6f);
    assert_eq!(state.original_selected_object_id, None);
    assert_eq!(state.original_scroll, 0);
}

#[test]
fn the_index_lists_only_the_games_systems_once_they_are_set() {
    // FUN_00422620 adds the systems of the viewer's side
    // (FUN_004f31b0); the other families stay.
    let entry = |object_id, name: &str| OriginalEncyclopediaEntry {
        object_id,
        name: name.into(),
    };
    let mut catalog = OriginalEncyclopediaCatalog::new(
        "Galactic Encyclopedia".into(),
        "Topic".into(),
        std::array::from_fn(|index| format!("Category {index}")),
        vec![
            entry(0x9000_0001, "Abregado"),
            entry(0x9000_0002, "Coruscant"),
            entry(0x1400_0001, "Star Destroyer"),
        ],
    );
    let ids = |catalog: &OriginalEncyclopediaCatalog, command| -> Vec<u32> {
        catalog.entries_for(command).iter().map(|entry| entry.object_id).collect()
    };
    assert_eq!(ids(&catalog, 0x70), [0x9000_0001, 0x9000_0002]);
    catalog.set_galaxy_systems([0x9000_0002]);
    assert_eq!(ids(&catalog, 0x70), [0x9000_0002]);
    // A world system's DatId is the bare SYSTEMSD record id (the
    // fixture galaxy's systems read 120 and up).
    catalog.set_galaxy_systems([0x0000_0001]);
    assert_eq!(ids(&catalog, 0x70), [0x9000_0001]);
    catalog.set_galaxy_systems([0x9000_0002]);
    assert_eq!(ids(&catalog, 0x6f), [0x9000_0002, 0x1400_0001]);
    assert_eq!(ids(&catalog, 0x71), [0x1400_0001]);
}

#[test]
fn the_fleet_topic_steps_through_all_databases_by_its_name() {
    // A0 menus2/20-25: a fleet's Encyclopedia shows "Fleet"; next opens
    // Fwillsving, previous Flax, Flax's next Fleet again, and the index
    // lists Fleet between them, selected. FUN_00422620 lists the
    // FLEETSD class 0x08000004 in every index. Here "Alliance Cruiser"
    // sorts between Alliance Army Regiment and Alliance Dreadnaught.
    let base = original_catalog_fixture();
    let mut entries = base.entries.clone();
    entries.push(OriginalEncyclopediaEntry {
        object_id: 0x0800_0004,
        name: "Alliance Cruiser".into(),
    });
    let catalog = OriginalEncyclopediaCatalog::new(
        base.title.clone(),
        base.topic_label.clone(),
        base.category_labels.clone(),
        entries,
    );
    let mut state = EncyclopediaState::default();
    open_original_encyclopedia_at(&mut state, &catalog, Some(0x0800_0004));
    assert!(state.open && state.original_topic_mode);
    assert_eq!(state.original_selected_object_id, Some(0x0800_0004));
    assert_eq!(
        original_topic_neighbours(&state, &catalog),
        (Some(0x1000_0001), Some(0x1400_0001))
    );
    apply_original_topic_command(&mut state, &catalog, 0x84);
    assert_eq!(state.original_selected_object_id, Some(0x1400_0001));
    apply_original_topic_command(&mut state, &catalog, 0x83);
    assert_eq!(state.original_selected_object_id, Some(0x0800_0004));
    apply_original_topic_command(&mut state, &catalog, 0x83);
    assert_eq!(state.original_selected_object_id, Some(0x1000_0001));
    apply_original_topic_command(&mut state, &catalog, 0x84);
    assert_eq!(state.original_selected_object_id, Some(0x0800_0004));

    // The index keeps it listed and selected.
    apply_original_topic_command(&mut state, &catalog, 0x68);
    assert!(!state.original_topic_mode);
    assert_eq!(state.original_selected_object_id, Some(0x0800_0004));
    let ids: Vec<u32> = catalog
        .entries_for(0x6f)
        .iter()
        .map(|entry| entry.object_id)
        .collect();
    assert_eq!(
        ids,
        [0x3000_0001, 0x1000_0001, 0x0800_0004, 0x1400_0001, 0x9000_0001]
    );
    assert!(!catalog
        .entries_for(0x71)
        .iter()
        .any(|entry| entry.object_id == 0x0800_0004));

    // A new window opened any other way lists it too.
    open_original_encyclopedia_at(&mut state, &catalog, None);
    assert!(catalog
        .entries_for(0x6f)
        .iter()
        .any(|entry| entry.object_id == 0x0800_0004));
}

#[test]
fn a_topic_steps_through_its_category_and_returns_to_the_index() {
    // FUN_0045fd20: previous / next exist when the list item has a
    // neighbour; FUN_0045fe60 and commands 0x83 / 0x84 step to it;
    // 0x68 returns to the index (FUN_0045f480). All Databases sorts
    // Ackbar, Alliance Army Regiment, Alliance Dreadnaught, Allyuen.
    let catalog = original_catalog_fixture();
    let mut state = EncyclopediaState {
        original_selected_object_id: Some(0x3000_0001),
        original_topic_mode: true,
        ..EncyclopediaState::default()
    };
    assert_eq!(
        original_topic_neighbours(&state, &catalog),
        (None, Some(0x1000_0001))
    );
    apply_original_topic_command(&mut state, &catalog, 0x83);
    assert_eq!(state.original_selected_object_id, Some(0x3000_0001));
    apply_original_topic_command(&mut state, &catalog, 0x84);
    assert_eq!(state.original_selected_object_id, Some(0x1000_0001));
    apply_original_topic_command(&mut state, &catalog, 0x84);
    apply_original_topic_command(&mut state, &catalog, 0x84);
    assert_eq!(state.original_selected_object_id, Some(0x9000_0001));
    assert_eq!(
        original_topic_neighbours(&state, &catalog),
        (Some(0x1400_0001), None)
    );
    apply_original_topic_command(&mut state, &catalog, 0x84);
    assert_eq!(state.original_selected_object_id, Some(0x9000_0001));
    // Within a category, only its objects.
    state.original_category_command = 0x70;
    assert_eq!(original_topic_neighbours(&state, &catalog), (None, None));
    assert!(state.original_topic_mode);
    apply_original_topic_command(&mut state, &catalog, 0x68);
    assert!(!state.original_topic_mode);
    // An unknown selection has no neighbours.
    state.original_selected_object_id = Some(0x5000_0001);
    assert_eq!(original_topic_neighbours(&state, &catalog), (None, None));
}

#[test]
fn the_frames_corner_arcs_lie_over_the_pane() {
    // A0 encyclopedia/category-0-alliance-day185, index-empire-day143,
    // ex1-adar-tallon and topic-abduction-empire-day166 show the base's
    // (10335 / 10336, keyed on blue) corner arcs inside the pane.
    use crate::cockpit::CockpitFaction;
    for (faction, base, rail) in [
        (CockpitFaction::Alliance, 10_335, 10_585),
        (CockpitFaction::Empire, 10_336, 10_589),
    ] {
        for pane in [(TOPIC_CONTENT, 12.0, 14.0), (INDEX_CONTENT, 12.0, 13.0)] {
            let ids = window_layers(faction, pane).map(|(id, x, y, _)| (id, x, y));
            assert_eq!(ids, [pane, (base, 0.0, 0.0), (rail, 412.0, 0.0)]);
        }
    }
}

fn run_topic(
    state: &mut EncyclopediaState,
    catalog: &OriginalEncyclopediaCatalog,
    scale: f32,
    events: Vec<egui::Event>,
) -> (Option<u16>, Vec<egui::epaint::ClippedShape>, egui::Context) {
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let topic = OriginalEncyclopediaTopic {
        title: "Ackbar".into(),
        body: Some("A Mon Calamari admiral.".into()),
        edata: None,
    };
    let mut activated = None;
    let screen = Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(1400.0, 1000.0),
    ));
    // egui draws an Area from its second frame (the first sizes it).
    let mut warm_up = EncyclopediaState {
        original_category_command: state.original_category_command,
        original_selected_object_id: state.original_selected_object_id,
        original_topic_mode: state.original_topic_mode,
        ..EncyclopediaState::default()
    };
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: screen,
            ..Default::default()
        },
        |ctx| {
            let _ = draw_encyclopedia_topic(
                ctx,
                &mut cache,
                crate::cockpit::CockpitFaction::Alliance,
                egui::pos2(10.0, 20.0),
                scale,
                &mut warm_up,
                catalog,
                &topic,
            );
        },
    );
    let output = ctx.run(
        egui::RawInput {
            screen_rect: screen,
            events,
            ..Default::default()
        },
        |ctx| {
            activated = draw_encyclopedia_topic(
                ctx,
                &mut cache,
                crate::cockpit::CockpitFaction::Alliance,
                egui::pos2(10.0, 20.0),
                scale,
                state,
                catalog,
                &topic,
            );
        },
    );
    (activated, output.shapes, ctx)
}

/// Twenty systems, S00..S19, in every category that lists systems.
fn systems_catalog() -> OriginalEncyclopediaCatalog {
    OriginalEncyclopediaCatalog::new(
        "Galactic Encyclopedia".into(),
        "Topic".into(),
        std::array::from_fn(|index| format!("Category {index}")),
        (0..20)
            .map(|index| OriginalEncyclopediaEntry {
                object_id: 0x9000_0001 + index,
                name: format!("S{index:02}"),
            })
            .collect(),
    )
}

#[test]
fn typing_reaches_a_freshly_opened_topic_box_and_chooses_the_best_row() {
    // A0 2026-11-11-icon-menu-topic/20-22: the opened index's box takes
    // "cor" with no press. Each edit's 0x408 (FUN_0045d8f0) chooses the
    // best prefix row and scrolls to it (FUN_00609650); Enter's 0x407
    // shows the chosen object's topic (FUN_0045f480(2)).
    let catalog = systems_catalog();
    let mut state = EncyclopediaState::default();
    open_original_encyclopedia_at(&mut state, &catalog, None);
    assert!(state.original_box_focused);
    run_index(
        &mut state,
        &catalog,
        (200.0, 54.0),
        vec![vec![egui::Event::Text("s1".into())]],
    );
    // "s1": S10 shares two letters, the first row to do so.
    assert_eq!(state.original_selected_object_id, Some(0x9000_000b));
    assert_eq!(state.original_topic_text.as_deref(), Some("s1"));
    // Row 10 (200..220) brought into the 160-pixel list.
    assert_eq!(state.original_scroll, 60);

    // An edit that keeps S10 leaves the list where it is (FUN_00609650
    // skips a row chosen already).
    state.original_scroll = 0;
    run_index(
        &mut state,
        &catalog,
        (0.0, 0.0),
        vec![vec![egui::Event::Text("0".into())]],
    );
    assert_eq!(state.original_topic_text.as_deref(), Some("s10"));
    assert_eq!(state.original_selected_object_id, Some(0x9000_000b));
    assert_eq!(state.original_scroll, 0);

    run_index(
        &mut state,
        &catalog,
        (0.0, 0.0),
        vec![vec![key(egui::Key::Enter)]],
    );
    assert!(state.original_topic_mode);
}

#[test]
fn only_the_open_indexs_focused_topic_box_holds_the_galaxy_keys() {
    // The focused edit gets the keys (encyclopedia-window.md, "Which
    // control holds the keys"); a topic shows its text control
    // (+0x13c, FUN_0045f480(2)) and a closed window holds nothing.
    let mut state = EncyclopediaState {
        open: true,
        original_box_focused: true,
        ..EncyclopediaState::default()
    };
    assert!(state.topic_box_holds_keys());
    state.original_topic_mode = true;
    assert!(!state.topic_box_holds_keys());
    state.original_topic_mode = false;
    state.open = false;
    assert!(!state.topic_box_holds_keys());
    state.open = true;
    state.original_box_focused = false;
    assert!(!state.topic_box_holds_keys());
}

#[test]
fn a_category_icon_or_a_row_takes_the_keys_from_the_topic_box() {
    // FUN_0045da70 case 0x6e: SetFocus(+0x12c), the list; a row press
    // focuses the list itself (FUN_006083c0). Keys typed then do not
    // reach the box. The Index command (0x68) gives it back.
    let catalog = systems_catalog();
    let mut state = EncyclopediaState::default();
    open_original_encyclopedia_at(&mut state, &catalog, None);
    state.original_box_focused = true;
    // The System Database icon (its press is keyed on its bitmap, which
    // the test cache lacks, so the command is applied as draw does).
    command_focus(&mut state, 0x70);
    change_category(&mut state, &catalog, 0x70);
    assert!(!state.original_box_focused);
    run_index(
        &mut state,
        &catalog,
        (0.0, 0.0),
        vec![vec![egui::Event::Text("s1".into())]],
    );
    assert_eq!(state.original_selected_object_id, Some(0x9000_0001));
    assert_eq!(state.original_topic_text.as_deref(), Some(""));
    // Topic with nothing chosen presses Index, which focuses the box.
    state.original_selected_object_id = None;
    command_focus(&mut state, 0x67);
    assert!(state.original_box_focused);
    command_focus(&mut state, 0x71);
    assert!(!state.original_box_focused);
    // The Index command focuses the box (FUN_0045da70 case 0x66).
    command_focus(&mut state, 0x68);
    assert!(state.original_box_focused);
    command_focus(&mut state, 0x71);

    apply_original_topic_command(&mut state, &catalog, 0x68);
    assert!(state.original_box_focused);
    // A press on the second row (y 157..177).
    run_index(
        &mut state,
        &catalog,
        (100.0, 167.0),
        vec![
            vec![button((100.0, 167.0), true)],
            vec![button((100.0, 167.0), false)],
            vec![egui::Event::Text("s1".into())],
        ],
    );
    assert!(!state.original_box_focused);
    assert_eq!(state.original_topic_text, None);
    assert_eq!(state.original_selected_object_id, Some(0x9000_0002));
}

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn the_topics_keys_step_through_the_category() {
    // FUN_0045fe60: in topic mode Left and Right are 0x83 and 0x84.
    let catalog = original_catalog_fixture();
    let mut state = EncyclopediaState {
        original_selected_object_id: Some(0x3000_0001),
        original_topic_mode: true,
        ..EncyclopediaState::default()
    };
    let (activated, _, _) =
        run_topic(&mut state, &catalog, 1.0, vec![key(egui::Key::ArrowLeft)]);
    assert_eq!(activated, None);
    let (activated, _, _) =
        run_topic(&mut state, &catalog, 1.0, vec![key(egui::Key::ArrowRight)]);
    assert_eq!(activated, Some(0x84));
    assert_eq!(state.original_selected_object_id, Some(0x1000_0001));
    let (activated, _, _) =
        run_topic(&mut state, &catalog, 1.0, vec![key(egui::Key::ArrowLeft)]);
    assert_eq!(activated, Some(0x83));
    assert_eq!(state.original_selected_object_id, Some(0x3000_0001));
    // No window at a zero scale.
    let (activated, shapes, _) =
        run_topic(&mut state, &catalog, 0.0, vec![key(egui::Key::ArrowRight)]);
    assert_eq!(activated, None);
    assert!(shapes.is_empty());
}

/// Twelve systems, E00..E11.
fn twelve_entry_catalog() -> OriginalEncyclopediaCatalog {
    OriginalEncyclopediaCatalog::new(
        "Galactic Encyclopedia".into(),
        "Topic".into(),
        std::array::from_fn(|index| format!("Category {index}")),
        (0..12)
            .map(|index| OriginalEncyclopediaEntry {
                object_id: 0x9000_0000 + index,
                name: format!("E{index:02}"),
            })
            .collect(),
    )
}

/// Run the index at scale 1 from (10, 20): two idle frames, then one
/// per entry of `frames`, the pointer at window point `pointer`.
/// Returns the last frame's texts (name, top-left, size) and the
/// bitmaps painted in it.
fn run_index(
    state: &mut EncyclopediaState,
    catalog: &OriginalEncyclopediaCatalog,
    pointer: (f32, f32),
    frames: Vec<Vec<egui::Event>>,
) -> (Vec<(String, egui::Pos2, egui::Vec2)>, Vec<(u32, egui::Pos2)>) {
    run_index_at(state, catalog, pointer, frames, 1.0)
}

/// [`run_index`] at `scale`.
fn run_index_at(
    state: &mut EncyclopediaState,
    catalog: &OriginalEncyclopediaCatalog,
    pointer: (f32, f32),
    frames: Vec<Vec<egui::Event>>,
    scale: f32,
) -> (Vec<(String, egui::Pos2, egui::Vec2)>, Vec<(u32, egui::Pos2)>) {
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let pos = egui::pos2(10.0 + pointer.0 * scale, 20.0 + pointer.1 * scale);
    let mut texts = Vec::new();
    for extra in [vec![], vec![]].into_iter().chain(frames) {
        crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
        let mut events = vec![egui::Event::PointerMoved(pos)];
        events.extend(extra);
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                let _ = draw_encyclopedia_index_catalog(
                    ctx,
                    &mut cache,
                    crate::cockpit::CockpitFaction::Alliance,
                    egui::pos2(10.0, 20.0),
                    scale,
                    state,
                    catalog,
                );
            },
        );
        // Each line's text, pen and baseline, and width by font size.
        texts = crate::theme::test_lines(&output.shapes)
            .into_iter()
            .map(|line| (line.text, line.pen, egui::vec2(line.width, line.size)))
            .collect();
    }
    let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow().clone());
    (texts, painted)
}

fn button(pos: (f32, f32), pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: egui::pos2(10.0 + pos.0, 20.0 + pos.1),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn at_scale_two_the_down_arrow_still_steps_ten_and_a_wheel_notch_a_row() {
    // The bar reads the pointer in window pixels at any scale. port: the
    // original has no wheel; a notch moves a 20-pixel row.
    let catalog = twelve_entry_catalog();
    let arrow = (380.0, 292.0);
    let at = |(x, y): (f32, f32)| egui::pos2(10.0 + x * 2.0, 20.0 + y * 2.0);
    let press = |pressed| egui::Event::PointerButton {
        pos: at(arrow),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let mut state = EncyclopediaState::default();
    let _ = run_index_at(&mut state, &catalog, arrow, vec![vec![press(true)], vec![press(false)]], 2.0);
    assert_eq!(state.original_scroll, 10);

    let wheel = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -40.0),
        modifiers: egui::Modifiers::NONE,
    };
    let mut state = EncyclopediaState::default();
    let _ = run_index(&mut state, &catalog, (200.0, 200.0), vec![vec![wheel.clone()]]);
    assert_eq!(state.original_scroll, 20);
    // Outside the list the wheel does nothing.
    let mut state = EncyclopediaState::default();
    let _ = run_index(&mut state, &catalog, (200.0, 20.0), vec![vec![wheel]]);
    assert_eq!(state.original_scroll, 0);
}

#[test]
fn a_bare_character_or_system_id_finds_its_entry() {
    // A world character's DatId is its MJCHARSD id alone (0x240 for
    // 0x3000_0240): the menu's Encyclopedia opens its topic (A0
    // encyclopedia/ctx3-luke).
    let entry = |object_id, name: &str| OriginalEncyclopediaEntry {
        object_id,
        name: name.into(),
    };
    let catalog = OriginalEncyclopediaCatalog::new(
        "Galactic Encyclopedia".into(),
        "Topic".into(),
        std::array::from_fn(|index| format!("Category {index}")),
        vec![
            entry(0x3000_0240, "Luke Skywalker"),
            entry(0x1400_0240, "Star Destroyer"),
            entry(0x9000_0007, "Yavin"),
            entry(0x3100_0007, "Bothan Spies"),
        ],
    );
    assert_eq!(catalog.resolve_object(0x3000_0240), Some(0x3000_0240));
    assert_eq!(catalog.resolve_object(0x240), Some(0x3000_0240));
    assert_eq!(catalog.resolve_object(0x7), None, "two families share it");
    assert_eq!(catalog.resolve_object(0x1400_0999), None);
    let mut state = EncyclopediaState::default();
    open_original_encyclopedia_at(&mut state, &catalog, Some(0x240));
    assert_eq!(state.original_selected_object_id, Some(0x3000_0240));
    assert!(state.original_topic_mode);
}

#[test]
fn a_double_click_on_a_row_shows_its_topic() {
    // FUN_0045da70 0x65: 0x309 presses 0x67 (A0 encyclopedia/topic-step-2:
    // a double click on Abduction shows its topic); one click chooses.
    let catalog = twelve_entry_catalog();
    let row = (100.0, 137.0 + 20.0 + 10.0);
    let click = vec![vec![button(row, true)], vec![button(row, false)]];
    let mut state = EncyclopediaState::default();
    let _ = run_index(&mut state, &catalog, row, click.clone());
    let chosen = state.original_selected_object_id;
    assert!(chosen.is_some());
    assert!(!state.original_topic_mode);

    let mut state = EncyclopediaState::default();
    let mut twice = click.clone();
    twice.extend(click.clone());
    let _ = run_index(&mut state, &catalog, row, twice);
    assert_eq!(state.original_selected_object_id, chosen);
    assert!(state.original_topic_mode);

    // A second click on another row, or after the double-click time,
    // only chooses.
    let object = chosen.unwrap();
    for last in [(object ^ 1, 0.0), (object, -10.0)] {
        let mut state = EncyclopediaState {
            original_last_click: Some(last),
            ..EncyclopediaState::default()
        };
        let _ = run_index(&mut state, &catalog, row, click.clone());
        assert_eq!(state.original_selected_object_id, chosen);
        assert!(!state.original_topic_mode, "{last:?}");
        assert_eq!(state.original_last_click.map(|(id, _)| id), Some(object));
    }
}

#[test]
fn the_index_list_scrolls_by_pixels_and_clips_its_rows() {
    // The list scrolls by pixels (FUN_0060a490): scrolled 30, E01's row
    // sits at 137 + 20 - 30 = 127; E00 is gone; E09, at 287, still shows
    // its top; E10 does not. Each name is DT_VCENTER in its 20 rows:
    // (20 - 16) / 2 = 2 down, then the default font's tmAscent 13.
    let catalog = twelve_entry_catalog();
    let mut state = EncyclopediaState {
        original_scroll: 30,
        ..EncyclopediaState::default()
    };
    let (texts, _) = run_index(&mut state, &catalog, (0.0, 0.0), vec![]);
    let baseline = |name: &str| {
        texts
            .iter()
            .find(|(text, ..)| text == name)
            .map(|(_, pen, _)| pen.y)
    };
    assert_eq!(baseline("E00"), None);
    assert_eq!(baseline("E01"), Some(20.0 + 127.0 + 15.0));
    assert_eq!(baseline("E09"), Some(20.0 + 287.0 + 15.0));
    assert_eq!(baseline("E10"), None);
    assert_eq!(state.original_scroll, 30);
}

#[test]
fn the_index_bar_sits_right_of_the_list_and_its_down_arrow_steps_ten() {
    // FUN_0045ddc0's bar (RCDATA 10653) at the list's right, x 374, from
    // y 137 to 297; a click on the down arrow scrolls by the list's 10.
    let catalog = twelve_entry_catalog();
    let mut state = EncyclopediaState::default();
    let arrow = (380.0, 292.0);
    let (_, painted) = run_index(
        &mut state,
        &catalog,
        arrow,
        vec![vec![button(arrow, true)], vec![button(arrow, false)]],
    );
    assert_eq!(state.original_scroll, 10);
    let at = |x: f32, y: f32| egui::pos2(10.0 + x, 20.0 + y);
    assert!(painted.contains(&(10_685, at(374.0, 137.0))), "{painted:?}");
    assert!(painted.contains(&(10_687, at(374.0, 288.0))), "{painted:?}");
    assert!(painted.contains(&(10_656, at(374.0, 146.0))), "{painted:?}");
    // Held on the arrow, it draws pressed; off it, not.
    let mut state = EncyclopediaState::default();
    let (_, painted) = run_index(&mut state, &catalog, arrow, vec![vec![button(arrow, true)], vec![]]);
    assert!(painted.iter().any(|(id, _)| *id == 10_688), "{painted:?}");
    let mut state = EncyclopediaState::default();
    let (_, painted) =
        run_index(&mut state, &catalog, (100.0, 100.0), vec![vec![button(arrow, true)], vec![]]);
    assert!(!painted.iter().any(|(id, _)| *id == 10_688), "{painted:?}");
    // A row clicked under the bar's column is not selected.
    let mut state = EncyclopediaState::default();
    let row = (378.0, 150.0);
    let _ = run_index(&mut state, &catalog, row, vec![vec![button(row, true)], vec![button(row, false)]]);
    assert_eq!(state.original_selected_object_id, None);
}

#[test]
fn index_rows_are_grey_but_the_selected_white_centred_in_twenty_pixel_rows() {
    // FUN_0045ddc0's list: text at x 5 of the 350-wide cell, centred on
    // its 20-pixel row (+0xe4 = 0x24), font 10, grey 0x787878 or white
    // when selected (FUN_00607ea0); the title and category label in
    // font 5. At scale 2 from (10, 20).
    let catalog = original_catalog_fixture();
    let mut state = EncyclopediaState {
        original_selected_object_id: Some(0x1000_0001),
        ..EncyclopediaState::default()
    };
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut shapes = Vec::new();
    for _ in 0..2 {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1000.0),
                )),
                ..Default::default()
            },
            |ctx| {
                let _ = draw_encyclopedia_index_catalog(
                    ctx,
                    &mut cache,
                    crate::cockpit::CockpitFaction::Alliance,
                    egui::pos2(10.0, 20.0),
                    2.0,
                    &mut state,
                    &catalog,
                );
            },
        );
        shapes = output.shapes;
    }
    // The selector also names the selected object; the rows start at
    // the cell's x 5.
    let row_x = 10.0 + 41.0 * 2.0;
    let lines = crate::theme::test_lines(&shapes);
    let text = |wanted: &str| {
        lines
            .iter()
            .filter(|line| !wanted.starts_with("Alliance") || line.pen.x == row_x)
            .find(|line| line.text == wanted)
            .unwrap_or_else(|| panic!("{wanted} not drawn in {lines:?}"))
    };
    // The default font (A0: 13 ppem): DT_VCENTER in 20 rows is
    // (20 - 16) / 2 = 2 down, then tmAscent 13.
    let ackbar = text("Ackbar");
    assert_eq!(ackbar.pen, egui::pos2(row_x, 20.0 + (137.0 + 15.0) * 2.0));
    assert_eq!(
        ackbar.size,
        crate::theme::gdi_font(crate::DEFAULT_FONT, 2.0).size
    );
    assert_eq!(ackbar.color, ORIGINAL_INDEX_ROW_COLOR);
    let regiment = text("Alliance Army Regiment");
    assert_eq!(regiment.pen.y, 20.0 + (157.0 + 15.0) * 2.0);
    assert_eq!(regiment.color, Color32::WHITE);
    assert_eq!(
        text("Galactic Encyclopedia").size,
        crate::theme::gdi_font(5, 2.0).size
    );
    assert_eq!(
        text("All Databases").size,
        crate::theme::gdi_font(5, 2.0).size
    );
    assert_eq!(text("Topic").size, crate::theme::gdi_font(4, 2.0).size);
}

#[test]
fn the_topic_lays_out_its_title_and_body_at_the_original_points() {
    // FUN_0045ddc0: the 470 x 330 client; the title in (36, 14, 350);
    // the body in (17, 231, 395, 80); here at scale 2 from (10, 20).
    let catalog = original_catalog_fixture();
    let mut state = EncyclopediaState {
        original_selected_object_id: Some(0x3000_0001),
        original_topic_mode: true,
        ..EncyclopediaState::default()
    };
    let (_, shapes, ctx) = run_topic(&mut state, &catalog, 2.0, Vec::new());
    let area = ctx
        .memory(|memory| memory.area_rect(egui::Id::new("original-encyclopedia-topic")))
        .expect("the topic window");
    assert_eq!(area.min, egui::pos2(10.0, 20.0));
    assert_eq!(area.size(), egui::vec2(470.0 * 2.0, 330.0 * 2.0));
    // The title in font 5 (the label +0x120), from its box's top
    // (tmAscent 13); the body in the default font, which GDI draws at 13
    // ppem on 16-pixel lines (A0 2026-10-29: lines 16 apart), white.
    let lines = crate::theme::test_lines(&shapes);
    let title = lines
        .iter()
        .find(|line| line.text == "Ackbar")
        .expect("the title");
    assert_eq!(title.size, crate::theme::gdi_font(5, 2.0).size);
    assert_eq!(title.pen.y, 20.0 + (14.0 + 13.0) * 2.0);
    assert_eq!(
        title.clip,
        egui::Rect::from_min_size(egui::pos2(10.0 + 36.0 * 2.0, 48.0), egui::vec2(700.0, 34.0))
    );
    // The body's later lines step by tmHeight
    // (theme::a_text_block_steps_its_lines_by_tm_height).
    let body = lines
        .iter()
        .find(|line| line.text.starts_with("A Mon Calamari"))
        .expect("the body");
    assert_eq!(
        body.size,
        crate::theme::gdi_font(crate::DEFAULT_FONT, 2.0).size
    );
    assert_eq!(
        body.pen,
        egui::pos2(10.0 + 17.0 * 2.0, 20.0 + (231.0 + 13.0) * 2.0)
    );
    assert_eq!(body.color, Color32::WHITE);
    assert_eq!(body.clip.size(), egui::vec2(395.0 * 2.0, 80.0 * 2.0));
}

#[test]
fn a_menu_object_opens_the_encyclopedia_at_its_topic() {
    // FUN_0045d400 resolves the initial object: its topic when the
    // catalog lists it, else the index.
    use crate::object_menu::MenuObject;
    let catalog = original_catalog_fixture();
    let mut world = GameWorld::default();
    let ackbar = world.characters.insert(rebellion_core::world::Character {
        dat_id: rebellion_core::ids::DatId::new(0x3000_0001),
        ..Default::default()
    });
    let unknown = world.characters.insert(rebellion_core::world::Character {
        dat_id: rebellion_core::ids::DatId::new(0x3000_0099),
        ..Default::default()
    });
    assert_eq!(
        original_encyclopedia_object(&world, MenuObject::Character(ackbar)),
        Some(0x3000_0001)
    );
    // A fleet opens its FLEETSD class, which FUN_00422620 lists
    // (evidence/2026-11-12-encyclopedia-generic-topics).
    assert_eq!(
        original_encyclopedia_object(&world, MenuObject::Fleet(Default::default())),
        Some(0x0800_0004)
    );
    // A squadron's class keeps its bare FIGHTSD id (TIE Bomber 0x07);
    // the catalog's entry is 0x1c000007, not Imperial Commandos'
    // 0x3c000007.
    let bomber = world
        .fighter_classes
        .insert(rebellion_core::world::FighterClass {
            dat_id: rebellion_core::ids::DatId::new(0x07),
            ..Default::default()
        });
    assert_eq!(
        original_encyclopedia_object(
            &world,
            MenuObject::PlanetFighter {
                system: Default::default(),
                class: bomber
            }
        ),
        Some(0x1c00_0007)
    );
    assert_eq!(
        class_entry(rebellion_core::ids::DatId::new(0x43), CAPITAL_SHIP_FAMILY),
        0x1400_0043
    );
    assert_eq!(
        class_entry(
            rebellion_core::ids::DatId::new(0x2200_0001),
            CAPITAL_SHIP_FAMILY
        ),
        0x2200_0001
    );

    let mut state = EncyclopediaState {
        original_category_command: 0x70,
        ..EncyclopediaState::default()
    };
    open_original_encyclopedia_at(&mut state, &catalog, Some(0x3000_0001));
    assert!(state.open && state.original_topic_mode);
    assert_eq!(state.original_category_command, 0x6f);
    assert_eq!(state.original_selected_object_id, Some(0x3000_0001));

    let mut state = EncyclopediaState::default();
    let object = original_encyclopedia_object(&world, MenuObject::Character(unknown));
    open_original_encyclopedia_at(&mut state, &catalog, object);
    assert!(state.open && !state.original_topic_mode);
    assert_eq!(state.original_selected_object_id, None);
}

#[test]
fn a_topics_artwork_name_gives_its_edata_number() {
    assert_eq!(
        OriginalEncyclopediaTopic::edata_number("EDATA.014"),
        Some(14)
    );
    assert_eq!(
        OriginalEncyclopediaTopic::edata_number("edata.187"),
        Some(187)
    );
    assert_eq!(OriginalEncyclopediaTopic::edata_number("EDATA.x"), None);
    assert_eq!(OriginalEncyclopediaTopic::edata_number("BMP.014"), None);
}

#[test]
fn original_category_ranges_preserve_every_exclusive_boundary() {
    assert!(original_category_contains(0x6f, 0));
    for (command, start, end) in [
        (0x70, 0x90, 0x98),
        (0x71, 0x14, 0x20),
        (0x72, 0x20, 0x30),
        (0x73, 0x40, 0x80),
        (0x74, 0x10, 0x14),
        (0x75, 0x30, 0x40),
    ] {
        assert!(!original_category_contains(command, start - 1));
        assert!(original_category_contains(command, start));
        assert!(original_category_contains(command, end - 1));
        assert!(!original_category_contains(command, end));
    }
    assert!(!original_category_contains(0x76, 0x30));
}

#[test]
fn original_catalog_reveal_and_clamp_keep_selection_visible() {
    let entries = (0..12)
        .map(|index| OriginalEncyclopediaEntry {
            object_id: 0x9000_0000 | index,
            name: format!("System {index:02}"),
        })
        .collect();
    let catalog = OriginalEncyclopediaCatalog::new(
        "Galactic Encyclopedia".into(),
        "Topic".into(),
        std::array::from_fn(|index| format!("Category {index}")),
        entries,
    );
    let mut state = EncyclopediaState {
        original_category_command: 0x70,
        original_selected_object_id: Some(0x9000_000b),
        original_scroll: 0,
        ..EncyclopediaState::default()
    };

    // Twelve entries of 20 pixels in the 160-pixel list (FUN_00607ea0),
    // scrolled by pixels: the last one shows from pixel 80, and the
    // range ends at 240 + 5 - 160 (FUN_00609ae0).
    reconcile_original_index_state(&mut state, &catalog, true);
    assert_eq!(state.original_scroll, 80);

    state.original_selected_object_id = Some(0x9000_0000);
    reconcile_original_index_state(&mut state, &catalog, true);
    assert_eq!(state.original_scroll, 0);

    state.original_scroll = usize::MAX;
    reconcile_original_index_state(&mut state, &catalog, false);
    assert_eq!(state.original_scroll, 85);

    // FUN_0045f100 rebuilds the list with nothing selected.
    state.original_category_command = 0xffff;
    state.original_selected_object_id = Some(0xffff_ffff);
    reconcile_original_index_state(&mut state, &catalog, false);
    assert_eq!(state.original_category_command, 0x6f);
    assert_eq!(state.original_selected_object_id, None);
    assert_eq!(state.original_scroll, 0);
}

#[test]
fn eight_twenty_pixel_rows_fill_the_native_list() {
    // FUN_00607ea0(.., 0x24, 0x89, 0x15e, 0xa0, .., 0x15e, 0x14, ..):
    // 350 by 20 cells in the 160-pixel list (A0 2026-10-29: eight rows,
    // 20 apart).
    let parent = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(470.0, 330.0));
    let list = original_index_list_rect(parent, 1.0);

    for row in 0..ORIGINAL_INDEX_VISIBLE_ROWS {
        #[expect(clippy::cast_precision_loss, reason = "Eight rows.")]
        let rect = original_index_row_rect(parent, 1.0, 137.0 + row as f32 * 20.0);
        assert!(rect.min.y >= list.min.y);
        assert!(rect.max.y <= list.max.y);
        assert_eq!(rect.height(), 20.0);
    }
    assert_eq!(ORIGINAL_INDEX_VISIBLE_ROWS, 8);
    assert_eq!(original_index_row_rect(parent, 1.0, 277.0).max.y, list.max.y);
    // A row scrolled half out is clipped to the list.
    assert_eq!(original_index_row_rect(parent, 1.0, 127.0).height(), 10.0);
}

#[test]
fn the_window_sits_centred_in_each_sides_galaxy_view() {
    // A0 2026-10-29 (Empire): (125, 52); captures 281117 and 281123
    // (Alliance): (62, 50). FUN_00606980's truncating halves of the
    // galaxy view (FUN_00421c70) less 470 by 330.
    use crate::cockpit::{CockpitFaction, CockpitLayout, CockpitViewport};
    for (faction, origin) in [
        (CockpitFaction::Alliance, (62.0, 50.0)),
        (CockpitFaction::Empire, (125.0, 52.0)),
    ] {
        let (x, y, width, height) = crate::cockpit::galaxy_aperture(faction);
        let layout = CockpitLayout {
            canvas: CockpitViewport {
                x: 10.0,
                y: 20.0,
                width: 1280.0,
                height: 960.0,
            },
            galaxy: CockpitViewport {
                x: 10.0 + x * 2.0,
                y: 20.0 + y * 2.0,
                width: width * 2.0,
                height: height * 2.0,
            },
            scale: 2.0,
        };
        assert_eq!(
            encyclopedia_origin(layout),
            egui::pos2(10.0 + origin.0 * 2.0, 20.0 + origin.1 * 2.0)
        );
    }
}
