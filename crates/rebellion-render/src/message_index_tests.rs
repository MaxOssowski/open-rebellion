
#[test]
fn each_control_has_its_tooltip_string() {
    use super::{button_tooltip_string, category_tooltip_string, IndexMode};
    // FUN_004665f0: the categories' names, A0 tooltip/71..80.
    let categories: Vec<_> = (0x79..=0x82).map(category_tooltip_string).collect();
    assert_eq!(
        categories,
        [0x8010, 0x8011, 0x8017, 0x8014, 0x8012, 0x8013, 0x8016, 0x8018, 0x8015, 0x8032]
            .map(Some)
    );
    assert_eq!(category_tooltip_string(0x83), None);
    let list = IndexMode::List;
    let single = IndexMode::SingleMessage;
    assert_eq!(button_tooltip_string(0x28, list, false), Some(0x1954));
    // FUN_00468fb0: Display Message, "Message Index" while one shows
    // (A0 tooltip/84, 90).
    assert_eq!(button_tooltip_string(0x65, list, false), Some(0x8020));
    assert_eq!(button_tooltip_string(0x65, single, false), Some(0x8019));
    // FUN_004697b0: the toggle names what it would do (A0 85).
    assert_eq!(button_tooltip_string(0x66, list, false), Some(0x8005));
    assert_eq!(button_tooltip_string(0x66, list, true), Some(0x8006));
    assert_eq!(button_tooltip_string(0x67, list, false), Some(0x8030));
    assert_eq!(button_tooltip_string(0x90, list, false), Some(0x8008));
    assert_eq!(button_tooltip_string(0x91, list, false), Some(0x8009));
    assert_eq!(button_tooltip_string(0x96, single, false), None);
}

#[test]
fn a_message_picture_sits_at_12_33_with_its_foreground_centred() {
    // FUN_0046a320: the background at (0xc, 0x21); the foreground
    // offset by half the size difference on each axis.
    let (back, front) =
        message_picture_rects(egui::vec2(400.0, 200.0), Some(egui::vec2(300.0, 151.0)));
    assert_eq!(
        back,
        egui::Rect::from_min_size(egui::pos2(12.0, 33.0), egui::vec2(400.0, 200.0))
    );
    assert_eq!(
        front,
        Some(egui::Rect::from_min_size(
            egui::pos2(62.0, 57.0),
            egui::vec2(300.0, 151.0)
        ))
    );
    // The text area starts right below the picture.
    assert_eq!(back.max.y + 1.0, TEXT_AREA_Y);
}
use super::*;
use crate::message_log::{MessageCategory, RailAudience};

// ── Geometry tests (preserved) ──────────────────────────────────────

#[test]
fn category_geometry_matches_the_recovered_native_controls() {
    let controls = message_category_control_specs(CockpitFaction::Alliance);

    assert_eq!(
        controls.map(|control| (control.command_id, control.x, control.width)),
        [
            (0x79, 0, 36),
            (0x7a, 38, 36),
            (0x7b, 76, 36),
            (0x7c, 114, 35),
            (0x7d, 151, 36),
            (0x7e, 189, 36),
            (0x7f, 227, 34),
            (0x80, 263, 36),
            (0x81, 301, 35),
            (0x82, 338, 37),
        ]
    );
    assert_eq!(
        controls.last().unwrap().x + controls.last().unwrap().width,
        375
    );
}

#[test]
fn category_resources_preserve_faction_variants() {
    let alliance = message_category_control_specs(CockpitFaction::Alliance);
    let empire = message_category_control_specs(CockpitFaction::Empire);

    assert_eq!(
        alliance.map(|control| (control.normal_resource, control.pressed_resource)),
        [
            (10_830, 10_831),
            (10_832, 10_833),
            (10_840, 10_841),
            (10_842, 10_843),
            (10_836, 10_837),
            (10_852, 10_853),
            (10_856, 10_857),
            (10_854, 10_855),
            (10_850, 10_851),
            (10_846, 10_847),
        ]
    );
    assert_eq!(
        empire.map(|control| (control.normal_resource, control.pressed_resource)),
        [
            (10_830, 10_831),
            (10_834, 10_835),
            (10_838, 10_839),
            (10_844, 10_845),
            (10_836, 10_837),
            (10_852, 10_853),
            (10_856, 10_857),
            (10_854, 10_855),
            (10_850, 10_851),
            (10_848, 10_849),
        ]
    );
}

#[test]
fn hit_testing_excludes_right_and_bottom_edges() {
    let rect = egui::Rect::from_min_max(egui::pos2(22.0, 46.0), egui::pos2(58.0, 87.0));

    assert!(message_index_rect_contains(rect, egui::pos2(22.0, 46.0)));
    assert!(message_index_rect_contains(
        rect,
        egui::pos2(57.999, 86.999)
    ));
    assert!(!message_index_rect_contains(rect, egui::pos2(58.0, 46.0)));
    assert!(!message_index_rect_contains(rect, egui::pos2(22.0, 87.0)));
}

// ── List geometry ───────────────────────────────────────────────────

#[test]
fn list_geometry_matches_fun_004665f0_and_fun_006082c0() {
    // FUN_004665f0: list at (25, 108), 368x194.
    // FUN_006082c0 in FUN_00468ab0: row 0x15a (346) by 0x15 (21).
    assert_eq!((LIST_X, LIST_Y), (25.0, 108.0));
    assert_eq!((LIST_WIDTH, LIST_HEIGHT), (368.0, 194.0));
    assert_eq!((ROW_WIDTH, ROW_HEIGHT), (346.0, 21.0));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "9.2"
    )]
    let whole_rows = (LIST_HEIGHT / ROW_HEIGHT) as usize;
    assert_eq!(whole_rows, 9);
}

#[test]
fn text_area_geometry_matches_fun_00469de0() {
    // FUN_00469de0: text area width 0x18b (395) for non-encyclopedia,
    // 0x140 (320) for type 5.
    assert_eq!((TEXT_AREA_X, TEXT_AREA_Y), (17.0, 234.0));
    assert_eq!(TEXT_AREA_WIDTH, 395.0);
    assert_eq!(TEXT_AREA_HEIGHT, 80.0);
}

// ── Category filter ─────────────────────────────────────────────────

#[test]
fn category_filter_masks_match_fun_004697b0() {
    // FUN_004697b0: the switch sets +0x158 for each command.
    assert_eq!(category_filter_mask(0x79), 0);
    assert_eq!(category_filter_mask(0x7a), 0x001);
    assert_eq!(category_filter_mask(0x7b), 0x080);
    assert_eq!(category_filter_mask(0x7c), 0x010);
    assert_eq!(category_filter_mask(0x7d), 0x004);
    assert_eq!(category_filter_mask(0x7e), 0x008);
    assert_eq!(category_filter_mask(0x7f), 0x040);
    assert_eq!(category_filter_mask(0x80), 0x100);
    assert_eq!(category_filter_mask(0x81), 0x020);
    assert_eq!(category_filter_mask(0x82), 0x200);
}

// ── Message filtering ───────────────────────────────────────────────

fn test_log() -> MessageLog {
    let mut log = MessageLog::new();
    log.push(
        GameMessage::new(1, "Fleet arrived", MessageCategory::Mission)
            .on_rail(MessageRail::Fleet, RailAudience::Both),
    );
    log.push(
        GameMessage::new(2, "Built ship", MessageCategory::Manufacturing)
            .on_rail(MessageRail::Manufacturing, RailAudience::Both),
    );
    log.push(
        GameMessage::new(3, "Advisor says", MessageCategory::Event)
            .on_rail(MessageRail::Advice, RailAudience::Both),
    );
    log.push(
        GameMessage::new(4, "Mission done", MessageCategory::Mission)
            .on_rail(MessageRail::Mission, RailAudience::Both),
    );
    log.push(GameMessage::new(5, "No rail", MessageCategory::Event));
    log
}

#[test]
fn the_list_scrolls_by_pixels_up_to_its_last_row_and_margin() {
    // FUN_00609ae0: content that fills the 194-pixel list gains 5.
    assert_eq!(max_scroll(0), 0);
    assert_eq!(max_scroll(9), 0);
    assert_eq!(max_scroll(10), 10 * 21 + 5 - 194);
    assert_eq!(max_scroll(31), 31 * 21 + 5 - 194);
}

#[test]
fn a_row_lays_out_its_bar_icon_text_and_click_area_scrolled_by_pixels() {
    // FUN_00468ab0 / A0 2026-10-29: the bar at the row's origin (352
    // by 17 shown), the icon at (1, 1), the text cell at (32, 2); the
    // list (25, 108) 368 by 194 ends its click area at the 12-wide bar.
    let list = egui::Rect::from_min_size(egui::pos2(50.0, 216.0), egui::vec2(736.0, 388.0));
    let row = row_layout(list, 2.0, 1, 0).unwrap();
    assert_eq!(
        row.bar,
        egui::Rect::from_min_size(egui::pos2(50.0, 258.0), egui::vec2(704.0, 34.0))
    );
    assert_eq!(
        row.icon,
        egui::Rect::from_min_size(egui::pos2(52.0, 260.0), egui::vec2(30.0, 30.0))
    );
    assert_eq!(row.text, egui::pos2(114.0, 262.0));
    assert_eq!(
        row.click,
        egui::Rect::from_min_max(egui::pos2(50.0, 258.0), egui::pos2(742.0, 300.0))
    );
    // A row's right end stops at the bar.
    let wide = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(350.0, 194.0));
    assert_eq!(row_layout(wide, 1.0, 0, 0).unwrap().click.max.x, 338.0);
    let wide = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(700.0, 388.0));
    assert_eq!(row_layout(wide, 2.0, 0, 0).unwrap().click.max.x, 676.0);
    let thirds = row_layout(list, 3.0, 0, 0).unwrap();
    assert_eq!(thirds.text, egui::pos2(50.0 + 96.0, 216.0 + 6.0));
    // Scrolled 10 pixels: the row moves up 10 and the first is cut.
    let scrolled = row_layout(list, 2.0, 1, 10).unwrap();
    assert_eq!(scrolled.text, egui::pos2(114.0, 242.0));
    let cut = row_layout(list, 2.0, 0, 10).unwrap();
    assert_eq!(cut.click.min.y, 216.0);
    // Rows wholly above or below the list are skipped; 194 / 21 shows
    // a tenth row's top.
    assert!(row_layout(list, 2.0, 0, 21).is_none());
    assert!(row_layout(list, 2.0, 0, 20).is_some());
    assert!(row_layout(list, 2.0, 9, 0).is_some());
    assert!(row_layout(list, 2.0, 10, 0).is_none());
    assert!(row_layout(list, 2.0, 10, 17).is_some());
}

#[test]
fn the_chosen_bar_shows_its_top_left_and_the_heading_its_cell() {
    assert_eq!(
        chosen_bar_uv([356, 21]),
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(352.0 / 356.0, 17.0 / 21.0))
    );
    assert_eq!(
        heading_pos(egui::pos2(125.0, 52.0), 1.0),
        egui::pos2(160.0, 143.0)
    );
    assert_eq!(
        heading_pos(egui::pos2(10.0, 20.0), 2.0),
        egui::pos2(80.0, 202.0)
    );
    // Class_0065a1c0_vt44: the title's cell at (15, 20).
    assert_eq!(
        title_pos(egui::pos2(125.0, 52.0), 1.0),
        egui::pos2(140.0, 72.0)
    );
    assert_eq!(
        title_pos(egui::pos2(10.0, 20.0), 3.0),
        egui::pos2(55.0, 80.0)
    );
}

#[test]
fn the_posting_toggle_shows_its_state_and_reads_the_categorys_option() {
    // FUN_004697b0 / FUN_00467f10: 0x2a78 / 0x2a7a while the category
    // alerts, 0x2ac7 / 0x2ac9 while it is silent; All reads the default.
    let post = |faction, silent| {
        window_button_specs(faction, IndexMode::List, silent, false, None)
            .into_iter()
            .find(|button| button.command_id == 0x66)
            .map(|button| (button.y, button.normal_resource, button.pressed_resource))
    };
    assert_eq!(
        post(CockpitFaction::Alliance, false),
        Some((0x93, 0x2a78, 0x2a79))
    );
    assert_eq!(
        post(CockpitFaction::Alliance, true),
        Some((0x93, 0x2ac7, 0x2a79))
    );
    assert_eq!(
        post(CockpitFaction::Empire, false),
        Some((0x94, 0x2a7a, 0x2a7b))
    );
    assert_eq!(
        post(CockpitFaction::Empire, true),
        Some((0x94, 0x2ac9, 0x2a7b))
    );
    assert_eq!(posting_mask(0x79), None);
    assert_eq!(posting_mask(0x82), Some(0x200));
    assert_eq!(posting_mask(0x7b), Some(0x80));
}

#[test]
fn the_toggle_and_select_all_answer_their_clicks() {
    // FUN_00467f10: 0x66 flips the shown category's option, 0x90
    // chooses every row. Origin (0, 0), Alliance: 0x66 at (0x1a7,
    // 0x93) 32 by 31, 0x90 at (0x11a, 0x57) 56 by 20.
    let log = test_log();
    let mut state = MessageIndexState::new();
    state.set_category(0x7b);
    let post = egui::pos2(f32::from(0x1a7_u16) + 16.0, f32::from(0x93_u16) + 15.0);
    let result = drive(&log, &mut state, click_at(post));
    assert!(
        result
            .actions
            .contains(&MessageIndexAction::ToggleSilent(Some(0x80)))
    );
    let all = egui::pos2(f32::from(0x11a_u16) + 28.0, f32::from(0x57_u16) + 10.0);
    let mut everything = MessageIndexState::new();
    drive(&log, &mut everything, click_at(all));
    assert_eq!(everything.selected.len(), 3);
    let result = drive(&log, &mut everything, click_at(post));
    assert!(
        result
            .actions
            .contains(&MessageIndexAction::ToggleSilent(None))
    );
}

#[test]
fn display_and_compose_chat_sit_disabled_under_the_toggle() {
    // FUN_004665f0: 0x67 at y 0xc9 / 0xcf and 0x68 at 0xff / 0x10a,
    // disabled art 0x2acf / 0x2ad0 and 0x2a7e / 0x2a81 (A0 2026-10-29).
    assert_eq!(
        disabled_rail_controls(CockpitFaction::Alliance, IndexMode::List, false, false),
        [
            (0x1a7, 0xc9, 0x20, 0x1f, 0x2acf),
            (0x1a7, 0xff, 0x20, 0x1f, 0x2a7e)
        ]
    );
    assert_eq!(
        disabled_rail_controls(
            CockpitFaction::Empire,
            IndexMode::SingleMessage,
            true,
            false
        ),
        [
            (0x1aa, 0xcf, 0x2c, 0x29, 0x2ad0),
            (0x1aa, 0x10a, 0x2c, 0x29, 0x2a81)
        ]
    );
    // FUN_004697b0: an empty list disables Display Message (0x2ad1 /
    // 0x2ad2), which then answers no click.
    assert_eq!(
        disabled_rail_controls(CockpitFaction::Empire, IndexMode::List, true, false).last(),
        Some(&(0x1aa, 0x59, 0x2c, 0x29, 0x2ad2))
    );
    assert_eq!(
        disabled_rail_controls(CockpitFaction::Alliance, IndexMode::List, true, false).last(),
        Some(&(0x1a7, 0x5d, 0x20, 0x1f, 0x2ad1))
    );
    let clickable = |empty| {
        window_button_specs(
            CockpitFaction::Alliance,
            IndexMode::List,
            false,
            empty,
            None,
        )
        .iter()
        .any(|button| button.command_id == 0x65)
    };
    assert!(clickable(false));
    assert!(!clickable(true));
    assert!(window_button_specs(
        CockpitFaction::Empire,
        IndexMode::SingleMessage,
        false,
        true,
        None
    )
    .iter()
    .any(|button| button.command_id == 0x65 && button.normal_resource == 0x288a));
}

#[test]
fn the_chosen_category_stays_pressed_and_a_click_chooses_another() {
    assert!(category_pressed(Some(0x82), 0x82, false));
    assert!(!category_pressed(Some(0x82), 0x79, false));
    assert!(category_pressed(Some(0x82), 0x79, true));
    assert!(!category_pressed(None, 0x79, false));
    // FUN_004665f0: Fleet (0x7b) at (22 + 0x4c, 46), 36 by 41.
    let log = test_log();
    let mut state = MessageIndexState::new();
    let fleet = egui::pos2(22.0 + f32::from(0x4c_u16) + 18.0, 46.0 + 20.0);
    drive(&log, &mut state, click_at(fleet));
    assert_eq!(state.category(), 0x7b);
}

#[test]
fn the_bare_shell_reports_a_clicked_category() {
    // The interface fixture's shell (draw_message_index_shell): a click
    // on Fleet (0x7b) returns it.
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let fleet = egui::pos2(22.0 + f32::from(0x4c_u16) + 18.0, 46.0 + 20.0);
    let mut clicked = None;
    for (index, events) in [
        vec![egui::Event::PointerMoved(fleet)],
        vec![egui::Event::PointerMoved(fleet)],
        vec![press(fleet, true)],
        vec![press(fleet, false)],
    ]
    .into_iter()
    .enumerate()
    {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(640.0, 480.0),
            )),
            time: Some(index as f64 * 0.05),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            if let Some(command) = draw_message_index_shell(
                ctx,
                &mut cache,
                CockpitFaction::Alliance,
                egui::Pos2::ZERO,
                1.0,
            ) {
                clicked = Some(command);
            }
        });
    }
    assert_eq!(clicked, Some(0x7b));
}

#[test]
fn display_goes_to_the_chosen_messages_system_and_closes() {
    // FUN_004697b0 enables Display (0x67) for a message with a target;
    // FUN_00467f10 goes to it (FUN_00429440) and closes (0x28).
    use rebellion_core::ids::SystemKey;
    let system = SystemKey::default();
    let mut log = MessageLog::new();
    log.push(
        GameMessage::new(1, "No place", MessageCategory::Mission)
            .on_rail(MessageRail::Fleet, RailAudience::Both),
    );
    log.push(
        GameMessage::at_system(2, "Fleet arrived", MessageCategory::Mission, system)
            .on_rail(MessageRail::Fleet, RailAudience::Both),
    );
    let rows = filtered_messages(&log, 0x7b, true);
    let mut state = MessageIndexState::new();
    state.set_category(0x7b);
    // Newest first: row 0 has the system.
    state.selected.insert(rows[1].id);
    assert_eq!(display_target(&rows, &state), None);
    state.selected.insert(rows[0].id);
    assert_eq!(display_target(&rows, &state), Some((system, None)));
    state.selected.clear();
    state.selected.insert(rows[0].id);
    state.mode = IndexMode::SingleMessage;
    state.viewing = Some(rows[1].id);
    assert_eq!(display_target(&rows, &state), None);
    state.viewing = Some(rows[0].id);
    assert_eq!(display_target(&rows, &state), Some((system, None)));
    // A Mission Report names its mission (A0 report-display-missions-
    // window-day235).
    let mut reports = MessageLog::new();
    reports.push(
        GameMessage::at_system(3, "Report", MessageCategory::Mission, system)
            .with_mission(7)
            .on_rail(MessageRail::Fleet, RailAudience::Both),
    );
    let report_rows = filtered_messages(&reports, 0x7b, true);
    state.viewing = Some(report_rows[0].id);
    assert_eq!(display_target(&report_rows, &state), Some((system, Some(7))));

    // Enabled art and the click (Alliance: 0x67 at (0x1a7, 0xc9)).
    let display = window_button_specs(
        CockpitFaction::Alliance,
        IndexMode::List,
        false,
        false,
        Some(DisplayKind::Object),
    )
    .into_iter()
    .find(|button| button.command_id == 0x67)
    .map(|button| (button.y, button.normal_resource, button.pressed_resource));
    assert_eq!(display, Some((0xc9, 0x2916, 0x2917)));
    let empire = window_button_specs(
        CockpitFaction::Empire,
        IndexMode::List,
        false,
        false,
        Some(DisplayKind::Object),
    )
    .into_iter()
    .find(|button| button.command_id == 0x67)
    .map(|button| (button.y, button.normal_resource));
    assert_eq!(empire, Some((0xcf, 0x2918)));
    assert_eq!(
        disabled_rail_controls(CockpitFaction::Alliance, IndexMode::List, false, true),
        [(0x1a7, 0xff, 0x20, 0x1f, 0x2a7e)]
    );
    let mut chosen = MessageIndexState::new();
    chosen.set_category(0x7b);
    let post = egui::pos2(f32::from(0x1a7_u16) + 16.0, f32::from(0xc9_u16) + 15.0);
    drive(&log, &mut chosen, click_at(row_center(0)));
    let result = drive(&log, &mut chosen, click_at(post));
    assert!(
        result
            .actions
            .contains(&MessageIndexAction::Display {
                system,
                mission: None
            })
    );
    assert!(result.actions.contains(&MessageIndexAction::Close));
}

#[test]
fn research_complete_displays_its_class_in_the_encyclopedia() {
    // Class 0x0065bb40 (Research Complete) answers slot +0x10 with 4:
    // FUN_004697b0 lights Display in 0x2d20 / 0x2d47 with 0x8031, and
    // FUN_00467f10 opens the Encyclopedia on +0x60 (FUN_0041d6b0), then
    // closes the index.
    let mut log = MessageLog::new();
    log.push(
        GameMessage::new(
            1,
            "Nebulon-B Frigate Research Complete",
            MessageCategory::Event,
        )
        .with_encyclopedia(0x1400_0002)
        .on_rail(MessageRail::Fleet, RailAudience::Both),
    );
    let rows = filtered_messages(&log, 0x7b, true);
    let mut state = MessageIndexState::new();
    state.set_category(0x7b);
    state.selected.insert(rows[0].id);
    assert_eq!(display_kind(&rows, &state), Some(DisplayKind::Encyclopedia));
    assert!(display_lit(&rows, &state));
    assert_eq!(display_target(&rows, &state), None);
    let art = |faction| {
        window_button_specs(
            faction,
            IndexMode::List,
            false,
            false,
            Some(DisplayKind::Encyclopedia),
        )
        .into_iter()
        .find(|button| button.command_id == 0x67)
        .map(|button| (button.y, button.normal_resource, button.pressed_resource))
    };
    assert_eq!(art(CockpitFaction::Alliance), Some((0xc9, 0x2d20, 0x2d21)));
    assert_eq!(art(CockpitFaction::Empire), Some((0xcf, 0x2d47, 0x2d48)));
    assert_eq!(
        display_button_string(Some(DisplayKind::Encyclopedia)),
        0x8031
    );
    assert_eq!(display_button_string(Some(DisplayKind::Object)), 0x8030);
    let mut chosen = MessageIndexState::new();
    chosen.set_category(0x7b);
    let post = egui::pos2(f32::from(0x1a7_u16) + 16.0, f32::from(0xc9_u16) + 15.0);
    drive(&log, &mut chosen, click_at(row_center(0)));
    let result = drive(&log, &mut chosen, click_at(post));
    assert!(
        result
            .actions
            .contains(&MessageIndexAction::Encyclopedia(0x1400_0002)),
        "{:?}",
        result.actions
    );
    assert!(result.actions.contains(&MessageIndexAction::Close));
}

#[test]
fn the_scroll_buttons_leave_display_as_it_was_on_a_kind_8_message() {
    // FUN_00467f10 cases 0x96 / 0x9a show the neighbour through
    // FUN_00469de0 alone, which sets Display (0x67) for kinds 3, 4 and
    // 5 only; FUN_004697b0, which greys it, runs on a list choice
    // (case 0x78). A0 2026-11-17-edited-saves-6 d-11 (sha256 in its
    // SHA256SUMS): Luke Goes to Dagobah (kind 8) reached with the
    // arrows from a kind-3 message keeps Display lit; chosen in the
    // list it is grey (A0 alliance-game2/luke-goes-to-dagobah).
    use rebellion_core::ids::SystemKey;
    let system = SystemKey::default();
    let mut log = MessageLog::new();
    log.push(
        GameMessage::at_system(1, "Enemy Mission Foiled", MessageCategory::Mission, system)
            .on_rail(MessageRail::Fleet, RailAudience::Both),
    );
    log.push(
        GameMessage::new(2, "Luke Goes to Dagobah", MessageCategory::Event)
            .plain()
            .on_rail(MessageRail::Fleet, RailAudience::Both),
    );
    let rows = filtered_messages(&log, 0x7b, true);
    let ids: Vec<MessageId> = rows.iter().map(|message| message.id).collect();
    let mut state = MessageIndexState::new();
    state.set_category(0x7b);
    // Newest first: row 0 is the kind-8 message.
    state.selected.insert(rows[0].id);
    assert_eq!(display_kind(&rows, &state), None);
    state.toggle_display();
    assert_eq!(display_kind(&rows, &state), None);
    // Down (0x96) to the older, kind-3 message: lit; back up (0x9a):
    // still lit, and its press goes nowhere.
    assert!(state.view_adjacent_keeping_display(&rows, true).is_some());
    assert_eq!(display_kind(&rows, &state), Some(DisplayKind::Object));
    assert!(state.view_adjacent_keeping_display(&rows, false).is_some());
    assert_eq!(state.viewing, Some(ids[0]));
    assert_eq!(display_kind(&rows, &state), Some(DisplayKind::Object));
    assert_eq!(display_target(&rows, &state), None);
    // Back to the list and Display Message again: grey (FUN_004697b0).
    state.toggle_display();
    state.toggle_display();
    assert_eq!(display_kind(&rows, &state), None);
}

#[test]
fn display_is_lit_for_a_kind_3_message_without_an_object() {
    // REBEXE vtables: Future Jedi Discovered's class 0x0065b520 takes
    // the generic 0x5a6730 at +0x10 (kind 3), and FUN_004697b0 lights
    // Display for kind 3 whatever the target; A0
    // research-display/08-future-jedi shows it lit. Advice (0x0065b188,
    // 0x48ad30: kind 8) stays grey, and so does a mission report
    // without a system (kind 5, FUN_004ece60).
    let mut log = MessageLog::new();
    log.push(
        GameMessage::new(1, "Kind 3", MessageCategory::Event)
            .on_rail(MessageRail::Fleet, RailAudience::Both),
    );
    log.push(
        GameMessage::new(2, "Advice", MessageCategory::Event)
            .on_rail(MessageRail::Advice, RailAudience::Both),
    );
    let mut report = GameMessage::new(3, "Report", MessageCategory::Event)
        .on_rail(MessageRail::Fleet, RailAudience::Both);
    report.mission = Some(7);
    log.push(report);
    // A report with a system (FUN_004ece60 finds the target) is lit.
    let mut placed = GameMessage::at_system(
        4,
        "Placed report",
        MessageCategory::Event,
        SystemKey::default(),
    )
    .on_rail(MessageRail::Fleet, RailAudience::Both);
    placed.mission = Some(8);
    log.push(placed);
    // A Mission Report asking to continue (kind 5, +0x84) without one is grey.
    let mut question = GameMessage::new(5, "Question", MessageCategory::Event)
        .on_rail(MessageRail::Fleet, RailAudience::Both);
    question.continue_question = Some(9);
    log.push(question);
    for (category, text, lit) in [
        (0x7b, "Kind 3", true),
        (0x7b, "Report", false),
        (0x7b, "Placed report", true),
        (0x7b, "Question", false),
    ] {
        let rows = filtered_messages(&log, category, true);
        let mut state = MessageIndexState::new();
        state.set_category(category);
        let row = rows.iter().find(|message| message.text == text).unwrap();
        state.selected.insert(row.id);
        assert_eq!(display_lit(&rows, &state), lit, "{text}");
        assert_eq!(
            display_target(&rows, &state).is_some(),
            text == "Placed report",
            "{text}"
        );
    }
    let advice = log
        .messages()
        .iter()
        .find(|message| message.text == "Advice")
        .unwrap();
    assert!(advice.rail == Some(MessageRail::Advice));
    let rows = vec![advice];
    let mut state = MessageIndexState::new();
    state.selected.insert(advice.id);
    state.set_category(0x7b);
    assert_eq!(display_kind(&rows, &state), None);
}

#[test]
fn display_is_lit_for_an_unplaced_target_and_its_press_does_nothing() {
    // A0 maintenance-shortfall/test-day1224-corvette-no-system: a
    // corvette lost in transit names no system, yet Display is lit
    // (FUN_004697b0 asks only for a target) and its press leaves the
    // index as it was (FUN_00467f10: FUN_004f2d10 finds no system).
    let mut log = MessageLog::new();
    log.push(
        GameMessage::new(1, "Corellian Corvette 2", MessageCategory::Event)
            .with_unplaced_target()
            .on_rail(MessageRail::Fleet, RailAudience::Both),
    );
    // Kind 8 (Research Exhausted's class 0x0065b0b0, vtable +0x10
    // 0x48ad30): grey.
    log.push(
        GameMessage::new(2, "No target", MessageCategory::Event)
            .plain()
            .on_rail(MessageRail::Fleet, RailAudience::Both),
    );
    let rows = filtered_messages(&log, 0x7b, true);
    let mut state = MessageIndexState::new();
    state.set_category(0x7b);
    state.selected.insert(rows[1].id);
    assert!(display_lit(&rows, &state));
    assert_eq!(display_target(&rows, &state), None);
    state.selected.clear();
    state.selected.insert(rows[0].id);
    assert!(!display_lit(&rows, &state));
    // The press: no action, the index stays open.
    let mut chosen = MessageIndexState::new();
    chosen.set_category(0x7b);
    let post = egui::pos2(f32::from(0x1a7_u16) + 16.0, f32::from(0xc9_u16) + 15.0);
    drive(&log, &mut chosen, click_at(row_center(1)));
    let result = drive(&log, &mut chosen, click_at(post));
    assert!(
        !result.actions.iter().any(|action| matches!(
            action,
            MessageIndexAction::Display { .. } | MessageIndexAction::Close
        )),
        "{:?}",
        result.actions
    );
}

#[test]
fn a_mission_reports_question_answers_continue_or_abort_then_returns_to_the_list() {
    // FUN_004665f0 places 0x98 at (355, 244) and 0x99 at (355, 281);
    // FUN_00469de0 shows them, with 0x2aae / 0x2ab1, for a kind-5
    // message in mode 2, and FUN_00467f10 answers through the message's
    // vt +0x38 (continue) or +0x3c (the held Abort), then runs 0x65.
    let mut log = MessageLog::new();
    log.push(
        GameMessage::new(1, "Do you wish the mission to continue?", MessageCategory::Diplomacy)
            .on_rail(MessageRail::Mission, RailAudience::Both)
            .with_continue_question(7),
    );
    log.push(
        GameMessage::new(2, "A report that asks nothing", MessageCategory::Diplomacy)
            .on_rail(MessageRail::Mission, RailAudience::Both),
    );
    let rows = filtered_messages(&log, 0x7c, true);
    let (asking, plain) = (rows[1].id, rows[0].id);
    assert_eq!(
        question_button_specs(IndexMode::SingleMessage, true)
            .iter()
            .map(|b| (b.command_id, b.x, b.y, b.width, b.height, b.normal_resource, b.pressed_resource))
            .collect::<Vec<_>>(),
        [(0x98, 0x163, 0xf4, 0x33, 0x23, 0x2aae, 0x2aaf), (0x99, 0x163, 0x119, 0x33, 0x23, 0x2ab1, 0x2ab2)]
    );
    assert!(question_button_specs(IndexMode::List, true).is_empty());
    assert!(question_button_specs(IndexMode::SingleMessage, false).is_empty());

    let viewing = |id| {
        let mut state = MessageIndexState::new();
        state.set_category(0x7c);
        state.mode = IndexMode::SingleMessage;
        state.viewing = Some(id);
        state.selected.insert(id);
        state
    };
    assert_eq!(viewed_question(&rows, &viewing(plain)), None);
    assert_eq!(viewed_question(&rows, &viewing(asking)), Some((asking, 7)));

    let check = egui::pos2(355.0 + 25.0, 244.0 + 17.0);
    let cross = egui::pos2(355.0 + 25.0, 281.0 + 17.0);
    let mut state = viewing(asking);
    let result = drive(&log, &mut state, click_at(check));
    assert!(result.actions.contains(&MessageIndexAction::ContinueMission { message: asking }));
    assert_eq!(state.mode, IndexMode::List);
    let mut state = viewing(asking);
    let result = drive(&log, &mut state, click_at(cross));
    assert!(result
        .actions
        .contains(&MessageIndexAction::AbortMission { message: asking, mission: 7 }));
    assert_eq!(state.mode, IndexMode::List);
    // A message that asks nothing has no answers there.
    let mut state = viewing(plain);
    let result = drive(&log, &mut state, click_at(check));
    assert!(result.actions.iter().all(|action| !matches!(
        action,
        MessageIndexAction::ContinueMission { .. } | MessageIndexAction::AbortMission { .. }
    )));
    assert_eq!(state.mode, IndexMode::SingleMessage);

    // Either answer asks no more (vt +0x38 / +0x3c clear +0x90).
    assert_eq!(log.answer_continue_question(asking), Some(7));
    assert_eq!(log.answer_continue_question(asking), None);
}

#[test]
fn a_question_narrows_the_message_text_to_0x140() {
    // FUN_00469de0: the text control is 0x140 wide for a kind-5 message
    // and 0x18b otherwise, from (17, 234), 80 high (drawn at 1.5).
    let mut log = MessageLog::new();
    log.push(
        GameMessage::new(1, "Asks to continue", MessageCategory::Diplomacy)
            .on_rail(MessageRail::Mission, RailAudience::Both)
            .with_continue_question(3),
    );
    log.push(
        GameMessage::new(2, "Asks nothing", MessageCategory::Diplomacy)
            .on_rail(MessageRail::Mission, RailAudience::Both),
    );
    let rows = filtered_messages(&log, 0x7c, true);
    let ids: Vec<_> = rows.iter().map(|message| message.id).collect();
    for (id, text, width) in [(ids[1], "Asks to continue", 320.0), (ids[0], "Asks nothing", 395.0)] {
        let mut state = MessageIndexState::new();
        state.set_category(0x7c);
        state.mode = IndexMode::SingleMessage;
        state.viewing = Some(id);
        let lines = drive_scaled(&log, &mut state, vec![], 1.5).lines;
        let line = lines.iter().find(|line| line.text == text).expect(text);
        assert_eq!(
            line.clip,
            egui::Rect::from_min_size(
                egui::pos2(17.0 * 1.5, 234.0 * 1.5),
                egui::vec2(width * 1.5, 80.0 * 1.5)
            ),
            "{text}"
        );
    }
}

#[test]
fn the_toggle_reads_the_shown_categorys_silence() {
    let mut log = test_log();
    assert!(!posting_silent(&log, 0x82));
    log.toggle_silent(Some(0x200));
    assert!(posting_silent(&log, 0x82));
    assert!(!posting_silent(&log, 0x7b));
    assert!(!posting_silent(&log, 0x79));
    log.toggle_silent(None);
    assert!(posting_silent(&log, 0x79));
    assert!(posting_silent(&log, 0x7b));
}

#[test]
fn select_all_chooses_every_row_the_list_shows() {
    // FUN_00467f10 case 0x90: every shown item gets +0x3c bit 0.
    let log = test_log();
    let rows = filtered_messages(&log, 0x79, true);
    let mut state = MessageIndexState::new();
    state.select_all(&rows);
    assert_eq!(state.selected, rows.iter().map(|m| m.id).collect());
    assert_eq!(state.selected.len(), 3);
}

#[test]
fn each_category_has_its_rows_icon_and_heading() {
    // FUN_00468ab0's icons (some + 1 for the Empire) and the controls'
    // labels from FUN_004665f0 (TEXTSTRA 0x8010..0x8018, 0x8032).
    use CockpitFaction::{Alliance, Empire};
    let icons: Vec<(Option<u32>, Option<u32>)> = MessageRail::RAIL_ORDER
        .iter()
        .map(|&rail| (row_icon(rail, Alliance), row_icon(rail, Empire)))
        .collect();
    assert_eq!(
        icons,
        [
            (Some(0x2a9a), Some(0x2a9b)),
            (Some(0x2ad4), Some(0x2ad5)),
            (Some(0x2a9e), Some(0x2a9f)),
            (Some(0x2ad6), Some(0x2ad6)),
            (Some(0x2a9c), Some(0x2a9d)),
            (Some(0x2ad3), Some(0x2ad3)),
            (Some(0x2ad7), Some(0x2ad7)),
            (Some(0x2ad8), Some(0x2ad9)),
            (Some(0x2aa4), Some(0x2aa4)),
        ]
    );
    assert_eq!(category_heading(0x79), Some("All Messages"));
    assert_eq!(category_heading(0x7b), Some("Fleet Messages"));
    assert_eq!(category_heading(0x7d), Some("Resource Messages"));
    assert_eq!(category_heading(0x7f), Some("Defense Messages"));
    assert_eq!(category_heading(0x82), Some("Advice Messages"));
    assert_eq!(category_heading(0x83), None);
    let headings: Vec<_> = (0x79..=0x82).filter_map(category_heading).collect();
    assert_eq!(headings.len(), 10);
}

#[test]
fn all_messages_shows_everything_except_advice() {
    // FUN_00468f20: when category is 0x79 and mask is 0x200, the item
    // is hidden. All other masks are shown.
    let log = test_log();
    let rows = filtered_messages(&log, 0x79, true);
    let texts: Vec<&str> = rows.iter().map(|m| m.text.as_str()).collect();

    // Newest first (FUN_00468ab0 from the last node; A0 2026-10-29).
    assert_eq!(texts, ["Mission done", "Built ship", "Fleet arrived"]);
    assert!(!rows.iter().any(|m| m.rail == Some(MessageRail::Advice)));
}

#[test]
fn a_rescue_attempt_has_no_category_icon_and_lists_under_all_messages_alone() {
    // FUN_0048c3a0 leaves the class's +0x34 at FUN_004c4d90's 0
    // (FUN_006158b0 is `xor eax,eax; ret`): FUN_00468ab0 draws no
    // icon, FUN_00468f20 shows it only under All (0x79), and it lights
    // no rail control. A0 2026-11-18-edited-saves-7 b-03/b-11: the
    // Rescue Attempt rows have no icon, the Jabba rows the Mission one.
    let texts: std::collections::HashMap<u16, String> = [
        (0x71ea, "{1:1} Rescue Attempt"),
        (0x71eb, "taken to Jabba's palace."),
        (0x7156, "Jabba Captures {1:1}"),
        (0x7157, "{1:1} was captured by Jabba."),
    ]
    .into_iter()
    .map(|(id, text)| (id, text.to_owned()))
    .collect();
    let chewbacca = rebellion_core::world::Character {
        name: "Chewbacca".into(),
        dat_id: rebellion_core::ids::DatId::new(0x343),
        is_alliance: true,
        is_major: true,
        ..Default::default()
    };
    let rescue =
        crate::story_messages::rescue_attempt_message(&chewbacca, 9, &texts).expect("rescue");
    let jabba =
        crate::story_messages::jabba_captures_message(&chewbacca, 21, &texts).expect("jabba");
    let icon = |m: &GameMessage| m.rail.and_then(|r| row_icon(r, CockpitFaction::Alliance));
    assert_eq!(icon(&rescue), None);
    assert_eq!(icon(&jabba), Some(0x2a9e));
    let mut log = MessageLog::new();
    log.push(rescue);
    log.push(jabba);
    let titles = |category| -> Vec<String> {
        filtered_messages(&log, category, true)
            .iter()
            .filter_map(|m| m.title.clone())
            .collect()
    };
    assert_eq!(
        titles(0x79),
        ["Jabba Captures Chewbacca", "Chewbacca Rescue Attempt"]
    );
    assert_eq!(titles(0x7c), ["Jabba Captures Chewbacca"]);
    assert_eq!(log.unread_mask(true), MessageRail::Mission.mask());
}

#[test]
fn specific_category_shows_only_its_rail() {
    // FUN_00468f20: visible when (filter & mask) != 0.
    let log = test_log();
    let fleet = filtered_messages(&log, 0x7b, true);
    assert_eq!(fleet.len(), 1);
    assert_eq!(fleet[0].text, "Fleet arrived");

    let mfg = filtered_messages(&log, 0x7e, true);
    assert_eq!(mfg.len(), 1);
    assert_eq!(mfg[0].text, "Built ship");

    let advice = filtered_messages(&log, 0x82, true);
    assert_eq!(advice.len(), 1);
    assert_eq!(advice[0].text, "Advisor says");
}

#[test]
fn no_rail_messages_never_appear_in_the_index() {
    let log = test_log();
    for cmd in [0x79, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e, 0x7f, 0x80, 0x81, 0x82] {
        let rows = filtered_messages(&log, cmd, true);
        assert!(
            !rows.iter().any(|m| m.text == "No rail"),
            "command {cmd:#x}",
        );
    }
}

#[test]
fn messages_for_the_other_side_are_hidden() {
    let mut log = MessageLog::new();
    log.push(
        GameMessage::new(1, "For alliance", MessageCategory::Mission)
            .on_rail(MessageRail::Fleet, RailAudience::Alliance),
    );
    log.push(
        GameMessage::new(2, "For empire", MessageCategory::Mission)
            .on_rail(MessageRail::Fleet, RailAudience::Empire),
    );
    log.push(
        GameMessage::new(3, "For both", MessageCategory::Mission)
            .on_rail(MessageRail::Fleet, RailAudience::Both),
    );

    // Newest first (FUN_00468ab0).
    let alliance = filtered_messages(&log, 0x7b, true);
    assert_eq!(alliance.len(), 2);
    assert_eq!(alliance[0].text, "For both");
    assert_eq!(alliance[1].text, "For alliance");

    let empire = filtered_messages(&log, 0x7b, false);
    assert_eq!(empire.len(), 2);
    assert_eq!(empire[0].text, "For both");
    assert_eq!(empire[1].text, "For empire");
}

// ── Advice speed ────────────────────────────────────────────────────

#[test]
fn switching_to_advice_emits_advice_shown() {
    // FUN_004697b0 case 0x82 -> FUN_0041d3f0 -> FUN_00487ff0(1).
    let mut state = MessageIndexState::new();
    let actions = state.set_category(0x82);
    assert_eq!(actions, [MessageIndexAction::AdviceShown]);
}

#[test]
fn switching_from_advice_emits_advice_hidden() {
    // FUN_004697b0 non-0x82 -> FUN_0041d410 -> FUN_00487ff0(0).
    let mut state = MessageIndexState::new();
    state.set_category(0x82);

    let actions = state.set_category(0x7b);

    assert_eq!(actions, [MessageIndexAction::AdviceHidden]);
}

#[test]
fn switching_to_same_category_emits_nothing() {
    let mut state = MessageIndexState::new();
    let actions = state.set_category(0x79);
    assert!(actions.is_empty());
}

#[test]
fn closing_the_window_emits_advice_hidden_and_mark_read() {
    // FUN_0046a6a0: close -> FUN_0041d410 (restore speed) + FUN_0048a530.
    let mut state = MessageIndexState::new();
    state.set_category(0x7b);

    let actions = state.close();

    assert!(actions.contains(&MessageIndexAction::AdviceHidden));
    assert!(actions.contains(&MessageIndexAction::MarkCategoryRead(Some(
        MessageRail::Fleet
    ))));
}

#[test]
fn closing_all_messages_marks_all_read() {
    // FUN_0048a530 with param 0: mark all read.
    let state = MessageIndexState::new(); // default is All (0x79)
    let actions = state.close();
    assert!(actions.contains(&MessageIndexAction::MarkCategoryRead(None)));
}

// ── Selection model ─────────────────────────────────────────────────

fn make_rows(log: &MessageLog) -> Vec<&GameMessage> {
    filtered_messages(log, 0x79, true)
}

#[test]
fn click_selects_one_row_clearing_others() {
    // Manual p. 43: "a click selects one message."
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();

    apply_click(&mut state, 0, &rows, egui::Modifiers::NONE);
    assert_eq!(state.selected, BTreeSet::from([rows[0].id]));

    apply_click(&mut state, 2, &rows, egui::Modifiers::NONE);
    assert_eq!(state.selected, BTreeSet::from([rows[2].id]));
}

#[test]
fn ctrl_click_toggles_an_individual_message() {
    // Manual p. 43: "Ctrl-click toggles an individual message."
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    let ctrl = egui::Modifiers {
        command: true,
        ..Default::default()
    };

    apply_click(&mut state, 0, &rows, egui::Modifiers::NONE);
    apply_click(&mut state, 1, &rows, ctrl);

    assert_eq!(state.selected, BTreeSet::from([rows[0].id, rows[1].id]));

    // Toggle off.
    apply_click(&mut state, 0, &rows, ctrl);
    assert_eq!(state.selected, BTreeSet::from([rows[1].id]));
}

#[test]
fn shift_click_selects_a_contiguous_range_from_the_anchor() {
    // Manual p. 43: "Shift-click selects a contiguous range."
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    let shift = egui::Modifiers {
        shift: true,
        ..Default::default()
    };

    // Anchor at row 0.
    apply_click(&mut state, 0, &rows, egui::Modifiers::NONE);

    // Shift-click row 2: selects 0, 1, 2.
    apply_click(&mut state, 2, &rows, shift);

    assert_eq!(
        state.selected,
        BTreeSet::from([rows[0].id, rows[1].id, rows[2].id])
    );
    assert_eq!(state.anchor, Some(0)); // anchor stays
}

#[test]
fn shift_click_adds_its_range_to_the_rows_already_chosen() {
    // A0 message-delete/sel0..sel2: a click on row 0, Ctrl on row 2,
    // then Shift on row 4 chooses 0 and 2..4; sh0..sh2: Shift back
    // inside a range keeps the rows past it. Here: row 0, Ctrl on 2,
    // Shift on 1 and then on 2 keep all three.
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    let shift = egui::Modifiers {
        shift: true,
        ..Default::default()
    };
    let ctrl = egui::Modifiers {
        command: true,
        ..Default::default()
    };
    let all = BTreeSet::from([rows[0].id, rows[1].id, rows[2].id]);
    apply_click(&mut state, 0, &rows, egui::Modifiers::NONE);
    apply_click(&mut state, 2, &rows, ctrl);
    apply_click(&mut state, 1, &rows, shift);
    assert_eq!(state.selected, all);
    apply_click(&mut state, 2, &rows, shift);
    assert_eq!(state.selected, all);
}

#[test]
fn shift_click_range_works_backwards() {
    // Manual p. 43: range is contiguous, direction doesn't matter.
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    let shift = egui::Modifiers {
        shift: true,
        ..Default::default()
    };

    apply_click(&mut state, 2, &rows, egui::Modifiers::NONE);
    apply_click(&mut state, 0, &rows, shift);

    assert_eq!(
        state.selected,
        BTreeSet::from([rows[0].id, rows[1].id, rows[2].id])
    );
}

#[test]
fn shift_click_without_anchor_acts_as_normal_click() {
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    let shift = egui::Modifiers {
        shift: true,
        ..Default::default()
    };

    apply_click(&mut state, 1, &rows, shift);

    assert_eq!(state.selected, BTreeSet::from([rows[1].id]));
    assert_eq!(state.anchor, Some(1));
}

// ── Delete ──────────────────────────────────────────────────────────

#[test]
fn delete_returns_the_ids_and_chooses_the_row_after_the_first() {
    // FUN_005f54a0: delete a single message by key, iterated over
    // the selection; FUN_00467f10 then chooses the row after the
    // first chosen one (A0 message-delete/del6: "The Battle Alert"
    // after "Positioning Fleets").
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    let ctrl = egui::Modifiers {
        command: true,
        ..Default::default()
    };

    apply_click(&mut state, 0, &rows, egui::Modifiers::NONE);
    apply_click(&mut state, 2, &rows, ctrl);

    let action = state.delete_selected(&rows);

    assert!(matches!(action, Some(MessageIndexAction::DeleteSelected(ids)) if ids.len() == 2));
    assert_eq!(state.selected, BTreeSet::from([rows[1].id]));
    assert_eq!(state.anchor, Some(0));
}

#[test]
fn deleting_the_last_row_chooses_the_one_before() {
    // FUN_00467f10: no next row, so the one before (A0
    // message-delete/del9: "Adjusting Time" after "Advice
    // Available").
    let log = test_log();
    let rows = make_rows(&log);
    let last = rows.len() - 1;
    let mut state = MessageIndexState::new();
    apply_click(&mut state, last, &rows, egui::Modifiers::NONE);
    assert!(state.delete_selected(&rows).is_some());
    assert_eq!(state.selected, BTreeSet::from([rows[last - 1].id]));
    assert_eq!(state.anchor, Some(last - 1));
}

#[test]
fn a_successor_deleted_too_leaves_no_row_chosen() {
    // port: the original would choose the freed row.
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    let shift = egui::Modifiers {
        shift: true,
        ..Default::default()
    };
    apply_click(&mut state, 0, &rows, egui::Modifiers::NONE);
    apply_click(&mut state, 1, &rows, shift);
    assert!(state.delete_selected(&rows).is_some());
    assert!(state.selected.is_empty());
    assert_eq!(state.anchor, None);
}

#[test]
fn deleting_the_only_row_chooses_none() {
    let log = test_log();
    let rows = make_rows(&log);
    let only = &rows[..1];
    let mut state = MessageIndexState::new();
    apply_click(&mut state, 0, only, egui::Modifiers::NONE);
    assert!(state.delete_selected(only).is_some());
    assert!(state.selected.is_empty());
    assert_eq!(state.anchor, None);
}

#[test]
fn delete_with_nothing_selected_returns_none() {
    let mut state = MessageIndexState::new();
    assert!(state.delete_selected(&[]).is_none());
}

// ── Window buttons ──────────────────────────────────────────────────
#[test]
fn each_category_command_names_the_rail_its_mask_filters() {
    // FUN_004697b0: commands 0x7a..0x82 filter by the masks 0x001..0x200,
    // one rail's bit each; All (0x79) names none.
    for command in 0x7a..=0x82 {
        let rail = category_rail(command).expect("a one-rail category");
        assert_eq!(
            rail.mask(),
            category_filter_mask(command),
            "command 0x{command:x}"
        );
    }
    assert_eq!(category_rail(0x79), None);
}

#[test]
fn only_entering_or_leaving_advice_moves_the_speed_and_a_switch_returns_to_the_list() {
    // FUN_004697b0: 0x82 calls FUN_0041d3f0, the others FUN_0041d410;
    // the hold guard (FUN_00487ff0) makes a switch between two others
    // change nothing. A switch rebuilds the list (FUN_00468ab0).
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    assert_eq!(state.set_category(0x7b), []);
    assert_eq!(state.set_category(0x7c), []);
    assert_eq!(state.category(), 0x7c);
    assert_eq!(state.set_category(0x82), [MessageIndexAction::AdviceShown]);
    assert_eq!(state.set_category(0x82), []);
    assert_eq!(state.set_category(0x79), [MessageIndexAction::AdviceHidden]);

    apply_click(&mut state, 0, &rows, egui::Modifiers::NONE);
    assert_eq!(state.selected(), &BTreeSet::from([rows[0].id]));
    state.toggle_display();
    assert_eq!(state.mode(), IndexMode::SingleMessage);
    state.set_category(0x7a);
    assert_eq!(state.mode(), IndexMode::List);
    assert_eq!(state.viewing, None);
    assert!(state.selected().is_empty());
}

#[test]
fn each_cockpit_command_opens_its_own_category() {
    // FUN_00422ce0: 0x75 -> 0x79, 0x136..0x13c -> 0x7a..0x80,
    // 0x13d -> 0x82 (Advice), 0x13e -> 0x81 (Chat).
    assert_eq!(opening_category(0x75), Some(0x79));
    assert_eq!(opening_category(0x136), Some(0x7a));
    assert_eq!(opening_category(0x13c), Some(0x80));
    assert_eq!(opening_category(0x13d), Some(0x82));
    assert_eq!(opening_category(0x13e), Some(0x81));
    assert_eq!(opening_category(0x13f), None);
    assert_eq!(opening_category(0x135), None);
}

#[test]
fn the_display_button_shows_the_selected_message_then_returns_to_the_list() {
    // FUN_00468fb0(0x65): mode 1 with a selection goes to mode 2; an
    // empty selection stays; mode 2 goes back to the list.
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    assert_eq!(state.toggle_display(), None);
    assert_eq!(state.mode, IndexMode::List);

    apply_click(&mut state, 1, &rows, egui::Modifiers::NONE);
    assert_eq!(
        state.toggle_display(),
        Some(MessageIndexAction::MessageDisplayed(rows[1].id))
    );
    assert_eq!(state.mode, IndexMode::SingleMessage);
    assert_eq!(state.viewing, Some(rows[1].id));

    assert_eq!(state.toggle_display(), None);
    assert_eq!(state.mode, IndexMode::List);
    assert_eq!(state.viewing, None);
}

#[test]
fn close_and_display_sit_on_the_rail_and_delete_shows_in_the_list_only() {
    // FUN_004665f0: 0x28, 0x65 and 0x66 at x 0x1a7 (Alliance) / 0x1aa
    // (Empire); 0x90 at (0x11a, 0x57) and 0x91 at (0x154, 0x57), shown
    // in mode 1 (FUN_00468fb0).
    let list = window_button_specs(
        CockpitFaction::Alliance,
        IndexMode::List,
        false,
        false,
        None,
    );
    assert_eq!(
        list.iter()
            .map(|button| (
                button.command_id,
                button.x,
                button.y,
                button.normal_resource
            ))
            .collect::<Vec<_>>(),
        [
            (0x28, 0x1a7, 0x19, 0x2882),
            // FUN_00468fb0: mode 1's Display Message art (A0: the
            // Empire's 0x29f2 exactly).
            (0x65, 0x1a7, 0x5d, 0x2a76),
            (0x66, 0x1a7, 0x93, 0x2a78),
            (0x90, 0x11a, 0x57, 0x2a94),
            (0x91, 0x154, 0x57, 0x2a96)
        ]
    );
    let empire = window_button_specs(
        CockpitFaction::Empire,
        IndexMode::SingleMessage,
        true,
        true,
        None,
    );
    assert_eq!(
        empire
            .iter()
            .map(|button| (
                button.command_id,
                button.x,
                button.width,
                button.pressed_resource
            ))
            .collect::<Vec<_>>(),
        [
            (0x28, 0x1aa, 0x2c, 0x2889),
            (0x65, 0x1aa, 0x2c, 0x288b),
            (0x66, 0x1aa, 0x2c, 0x2a7b),
            // FUN_00468fb0 shows the scroll buttons in mode 2.
            (0x96, 390, 19, 10_920),
            (0x9a, 367, 19, 10_949)
        ]
    );
}

#[test]
fn the_shell_paints_the_windows_black_background_under_its_base() {
    // FUN_00466350 makes the window 470 by 331, here at scale 2; the
    // base's keyed last row shows black in A0 modal2/17-msg (screen
    // row 382).
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    // A new Area shows from its second frame, once faded in.
    let mut frame = |time| {
        let input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        ctx.run(input, |ctx| {
            draw_shell(
                ctx,
                &mut cache,
                CockpitFaction::Alliance,
                egui::pos2(85.0, 55.0),
                2.0,
                IndexMode::List,
                None,
            );
        })
    };
    frame(0.0);
    frame(1.0);
    let output = frame(2.0);
    let backgrounds = output
        .shapes
        .iter()
        .filter(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) => {
                rect.fill == Color32::BLACK
                    && rect.rect
                        == egui::Rect::from_min_size(
                            egui::pos2(85.0, 55.0),
                            egui::vec2(940.0, 662.0),
                        )
            }
            _ => false,
        })
        .count();
    assert_eq!(backgrounds, 1);
}

#[test]
fn the_frames_corner_arcs_lie_over_the_panes() {
    // A0 empire-game3/index-joins-chosen and message-index/
    // category-1-day228 show the base's (10336 / 10335, keyed on blue)
    // corner arcs inside 10822's rectangle; empire-game3/joins-enemy
    // shows them over 0x2ab5 and 0x2ab4.
    for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        for mode in [IndexMode::List, IndexMode::SingleMessage] {
            let layers = shell_layers(faction, mode);
            let base = layers
                .iter()
                .position(|(_, (x, y, _, _))| *x == 0.0 && *y == 0.0)
                .expect("the base");
            let last_pane = layers
                .iter()
                .rposition(|(_, (x, _, _, _))| *x == 12.0)
                .expect("a pane");
            assert!(last_pane < base, "{faction:?} {mode:?}: {layers:?}");
        }
    }
}

#[test]
fn mode_two_shows_the_message_pane_without_the_list_or_its_controls() {
    // FUN_004665f0 panes: 0x32 holds the list content 10822; 0x34 the
    // title strip 0x2ab5 at (12, 14) and the text area 0x2ab4 at
    // (12, 232). FUN_00468fb0 selects 0x34 for a message.
    let ids = |mode| {
        shell_layers(CockpitFaction::Alliance, mode)
            .into_iter()
            .map(|(resource, (x, y, _, _))| (resource, x, y))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(IndexMode::List),
        [
            (10_822, 12.0, 13.0),
            (10_335, 0.0, 0.0),
            (10_820, 412.0, 0.0)
        ]
    );
    assert_eq!(
        ids(IndexMode::SingleMessage),
        [
            (10_933, 12.0, 14.0),
            (10_932, 12.0, 232.0),
            (10_335, 0.0, 0.0),
            (10_820, 412.0, 0.0)
        ]
    );
    // The text area sits inside the black pane.
    let (_, (x, y, width, height)) = MESSAGE_TEXT_PANE;
    assert!(TEXT_AREA_X >= x && TEXT_AREA_Y >= y);
    assert!(TEXT_AREA_X + TEXT_AREA_WIDTH <= x + width);
    assert!(TEXT_AREA_Y + TEXT_AREA_HEIGHT <= y + height);
}

#[test]
fn the_scroll_buttons_step_through_the_categorys_messages() {
    // FUN_0046a200 enables each scroll button only with a neighbour.
    // FUN_00467f10: forward is 0x96, the row's next link (FUN_004ad730),
    // back is 0x9a, its previous link (FUN_004ad750).
    let mut state = MessageIndexState::new();
    state.mode = IndexMode::SingleMessage;
    state.viewing = Some(2);
    let rows = [1, 2, 3];
    assert_eq!(
        state.view_adjacent(&rows, true),
        Some(MessageIndexAction::MessageDisplayed(3))
    );
    assert_eq!(state.view_adjacent(&rows, true), None);
    assert_eq!(state.viewing, Some(3));
    assert_eq!(
        state.view_adjacent(&rows, false),
        Some(MessageIndexAction::MessageDisplayed(2))
    );
    state.viewing = Some(1);
    assert_eq!(state.view_adjacent(&rows, false), None);
    state.viewing = Some(9);
    assert_eq!(state.view_adjacent(&rows, true), None);
}

#[test]
fn the_scroll_buttons_show_their_disabled_art_at_the_ends_of_the_list() {
    // FUN_0046a200: a scroll button with no neighbour on its side takes
    // its FUN_00603150(2, ..) art (FUN_004665f0): 0x96 0x2aa9 (10921) on
    // the oldest message, 0x9a 0x2ac6 (10950) on the newest.
    let mut state = MessageIndexState::new();
    state.mode = IndexMode::SingleMessage;
    // Rows run newest first (FUN_00468ab0).
    let rows = [3, 2, 1];
    let art = |state: &MessageIndexState| {
        let (older, newer) = state.scroll_neighbours(&rows);
        disable_scroll_buttons(
            window_button_specs(
                CockpitFaction::Alliance,
                IndexMode::SingleMessage,
                false,
                false,
                None,
            ),
            older,
            newer,
        )
        .into_iter()
        .filter(|button| matches!(button.command_id, 0x96 | 0x9a))
        .map(|button| {
            (
                button.command_id,
                button.normal_resource,
                button.pressed_resource,
            )
        })
        .collect::<Vec<_>>()
    };
    state.viewing = Some(3);
    assert_eq!(
        art(&state),
        [(0x96, 10_919, 10_920), (0x9a, 10_950, 10_950)]
    );
    state.viewing = Some(2);
    assert_eq!(
        art(&state),
        [(0x96, 10_919, 10_920), (0x9a, 10_948, 10_949)]
    );
    state.viewing = Some(1);
    assert_eq!(
        art(&state),
        [(0x96, 10_921, 10_921), (0x9a, 10_948, 10_949)]
    );
}

// ── Category switch state reset ─────────────────────────────────────

#[test]
fn category_switch_clears_selection_and_scroll() {
    // FUN_004697b0: rebuilds the list (FUN_00468ab0) on category change.
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();

    apply_click(&mut state, 1, &rows, egui::Modifiers::NONE);
    state.scroll_offset = 5;

    state.set_category(0x7b);

    assert!(state.selected.is_empty());
    assert_eq!(state.scroll_offset, 0);
    assert_eq!(state.anchor, None);
}

// ── Report ──────────────────────────────────────────────────────────

#[test]
fn report_reflects_the_filtered_rows_and_selection() {
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    apply_click(&mut state, 1, &rows, egui::Modifiers::NONE);

    let report = state.report(&log, true);

    assert_eq!(report.mode, IndexMode::List);
    assert_eq!(report.category, 0x79);
    assert_eq!(report.rows.len(), 3); // All except Advice and no-rail
    assert_eq!(report.selected, BTreeSet::from([rows[1].id]));
    // Newest first (FUN_00468ab0).
    assert_eq!(report.rows[0].text, "Mission done");
    assert_eq!(report.rows[0].rail, MessageRail::Mission);
    assert!(report.rows[0].unread);
}

// ── Headless egui interaction tests ──────────────────────────────────

/// Build a `MessageLog` with enough fleet messages for scroll testing.
#[allow(dead_code)]
fn scroll_log(count: usize) -> MessageLog {
    let mut log = MessageLog::new();
    for i in 0..count {
        log.push(
            GameMessage::new(
                i as u64,
                format!("Fleet message {i}"),
                MessageCategory::Mission,
            )
            .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
    }
    log
}

fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    }
}

fn press_with(pos: egui::Pos2, pressed: bool, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    }
}

/// The screen position of the center of list row `line` (0-based
/// visible line) at scale 1 with origin (0, 0).
fn row_center(line: usize) -> egui::Pos2 {
    egui::pos2(
        LIST_X + TEXT_INSET + 40.0,
        LIST_Y + ROW_HEIGHT * line as f32 + ROW_HEIGHT / 2.0,
    )
}

struct DriveResult {
    actions: Vec<MessageIndexAction>,
    /// The last frame's lines of text.
    lines: Vec<crate::theme::TestLine>,
}

/// Run two idle frames, then one frame per entry in `frames` with the
/// given events, returning the collected actions.
fn drive(
    log: &MessageLog,
    state: &mut MessageIndexState,
    frames: Vec<(egui::Pos2, Vec<egui::Event>)>,
) -> DriveResult {
    drive_scaled(log, state, frames, 1.0)
}

/// [`drive`] with the window drawn at `scale`.
fn drive_scaled(
    log: &MessageLog,
    state: &mut MessageIndexState,
    frames: Vec<(egui::Pos2, Vec<egui::Event>)>,
    scale: f32,
) -> DriveResult {
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let origin = egui::Pos2::ZERO;
    let mut all_actions = Vec::new();
    let mut lines = Vec::new();

    let first = frames.first().map_or(egui::Pos2::ZERO, |(pos, _)| *pos);
    let full_frames = [(first, vec![]), (first, vec![])].into_iter().chain(frames);

    for (index, (pos, events)) in full_frames.enumerate() {
        // Extract modifier state from PointerButton events so that
        // RawInput::modifiers (read by ctx.input(|i| i.modifiers))
        // reflects which keys are held during this frame.
        let frame_modifiers = events
            .iter()
            .filter_map(|event| match event {
                egui::Event::PointerButton { modifiers, .. } => Some(*modifiers),
                _ => None,
            })
            .next_back()
            .unwrap_or_default();

        let mut raw_events = vec![egui::Event::PointerMoved(pos)];
        raw_events.extend(events.into_iter().map(|event| match event {
            egui::Event::PointerButton {
                pressed, modifiers, ..
            } => egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers,
            },
            other => other,
        }));
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(640.0, 480.0),
            )),
            time: Some(index as f64 * 0.05),
            modifiers: frame_modifiers,
            events: raw_events,
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            let actions = draw_message_index(
                ctx,
                &mut cache,
                &mut EncyclopediaState::new(),
                CockpitFaction::Alliance,
                origin,
                scale,
                log,
                state,
                true,
            );
            all_actions.extend(actions);
        });
        lines = crate::theme::test_lines(&output.shapes);
    }

    DriveResult {
        actions: all_actions,
        lines,
    }
}

#[test]
fn each_message_shown_starts_its_text_at_the_top() {
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    state.follow_text(rows[0].id);
    state.text_scroll.0 = 32.0;
    state.follow_text(rows[0].id);
    assert_eq!(state.text_scroll, (32.0, Some(rows[0].id)));
    state.follow_text(rows[1].id);
    assert_eq!(state.text_scroll, (0.0, Some(rows[1].id)));
    assert!(state.text_scroll_hold.is_none());
}

#[test]
fn a_text_controls_lines_take_their_pitch_and_scroll_up_by_the_first_pixel() {
    // Seven 16-pixel lines (112) overflow the 80-pixel control: they
    // wrap beside the 12-wide bar; five (80) fit at the full width.
    let seven = |wrap: f32| if wrap >= 395.0 { 6 } else { 7 };
    assert_eq!(text_wrap_lines(395.0, 16.0, seven), (383.0, 112.0, true));
    assert_eq!(text_wrap_lines(395.0, 16.0, |_| 5), (395.0, 80.0, false));
    // Scrolled 16 pixels at scale 2 from (10, 20): the lines start 32
    // above, 383 wide, the control's 80 and the 16 tall.
    assert_eq!(
        scrolled_text_rect(egui::pos2(10.0, 20.0), 2.0, 383.0, 16.0),
        egui::Rect::from_min_size(egui::pos2(10.0, -12.0), egui::vec2(766.0, 192.0))
    );
}

#[test]
fn a_long_text_wraps_beside_the_bar_and_a_short_one_does_not() {
    // A0 agent-advice/02-advice-message: the long advice wraps short of
    // the control's 12-wide bar (FUN_0060a490(.., 0x299a, ..)).
    let lines = |wrap: f32| if wrap >= 395.0 { 96.0 } else { 112.0 };
    assert_eq!(text_wrap(395.0, lines), (383.0, 112.0, true));
    assert_eq!(text_wrap(395.0, |_| 80.0), (395.0, 80.0, false));
    assert_eq!(text_wrap(395.0, |_| 16.0), (395.0, 16.0, false));
}

#[test]
fn a_window_rectangle_lands_on_the_screen_scaled_from_the_origin() {
    let rect = egui::Rect::from_min_size(egui::pos2(15.0, 16.0), egui::vec2(15.0, 16.0));
    assert_eq!(
        on_screen(egui::pos2(100.0, 50.0), 2.0, rect),
        egui::Rect::from_min_size(egui::pos2(130.0, 82.0), egui::vec2(30.0, 32.0))
    );
    // Class_0065a1c0_vt44: the title from (40, 13).
    assert_eq!(TITLE_TEXT_CELL.min, egui::pos2(40.0, 13.0));
}

#[test]
fn the_titles_image_stands_at_its_own_size_with_the_cut_icon() {
    // Class_0065a1c0_vt44 -> FUN_005fc140 with no destination: A0
    // messages-day633/capture-palpatine shows 0x2a9e's image at
    // (15, 16), message-rail/fleet-arrives-averam 0x2ad4's at (15, 15),
    // each icon cut by its last column and row.
    let (image, icon, uv) = title_icon_layout(egui::vec2(15.0, 16.0));
    assert_eq!(
        image,
        egui::Rect::from_min_size(egui::pos2(15.0, 16.0), egui::vec2(15.0, 16.0))
    );
    assert_eq!(
        icon,
        egui::Rect::from_min_size(egui::pos2(16.0, 17.0), egui::vec2(14.0, 15.0))
    );
    assert_eq!(uv.max, egui::pos2(14.0 / 15.0, 15.0 / 16.0));
    let (image, icon, _) = title_icon_layout(egui::vec2(15.0, 15.0));
    assert_eq!(image.min, egui::pos2(15.0, 15.0));
    assert_eq!(icon.min, egui::pos2(16.0, 16.0));
}

#[test]
fn an_unchosen_rows_icon_loses_its_last_column_and_row() {
    // FUN_00468ab0: the icon at (1, 1) of an image its own size; A0
    // messages-day633/index-tab7 shows 0x2ad7's column 14 only on the
    // chosen row.
    let (size, uv) = row_icon_shown(egui::vec2(15.0, 16.0), false);
    assert_eq!(size, egui::vec2(14.0, 15.0));
    assert_eq!(uv.min, egui::Pos2::ZERO);
    assert_eq!(uv.max, egui::pos2(14.0 / 15.0, 15.0 / 16.0));
    let (size, uv) = row_icon_shown(egui::vec2(15.0, 16.0), true);
    assert_eq!(size, egui::vec2(15.0, 16.0));
    assert_eq!(uv.max, egui::pos2(1.0, 1.0));
    // A one-pixel icon at (1, 1) of its own one-pixel image shows none.
    for (thin, shown) in [
        (egui::vec2(1.0, 5.0), egui::vec2(0.0, 4.0)),
        (egui::vec2(5.0, 1.0), egui::vec2(4.0, 0.0)),
    ] {
        assert_eq!(row_icon_shown(thin, false).0, shown);
    }
    for empty in [egui::vec2(0.0, 5.0), egui::vec2(5.0, 0.0)] {
        let (size, uv) = row_icon_shown(empty, false);
        assert_eq!(size, empty);
        assert_eq!(uv.max, egui::pos2(1.0, 1.0));
    }
}

#[test]
fn the_chosen_bar_leaves_the_icons_cell_to_the_list() {
    // A0 messages-day633/index-tab3: under the chosen Mission row's icon
    // (15 by 16 at (1, 1)) the list's background shows; the bar's red
    // covers the rest of its 352 by 17 (x 0 and y 0, x from 16, y 17).
    let bar = egui::Rect::from_min_size(egui::pos2(100.0, 200.0), egui::vec2(704.0, 34.0));
    let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(0.5, 0.25));
    let pieces = bar_around_icon(bar, uv, Some(egui::vec2(15.0, 16.0)), 2.0);
    let area: f32 = pieces.iter().map(|(rect, _)| rect.area()).sum();
    assert_eq!(area, 704.0 * 34.0 - 30.0 * 32.0);
    let icon_cell = egui::Rect::from_min_size(egui::pos2(102.0, 202.0), egui::vec2(30.0, 32.0));
    for (rect, _) in &pieces {
        assert!(
            rect.intersect(icon_cell).area() <= 0.0,
            "{rect:?} covers the icon"
        );
    }
    // The pieces' texture coordinates follow their places on the bar.
    let (right, right_uv) = pieces
        .iter()
        .find(|(rect, _)| rect.min.x == 132.0)
        .copied()
        .unwrap();
    assert_eq!(right.max.x, 804.0);
    assert_eq!(right_uv.min.x, 16.0 / 352.0 * 0.5);
    assert_eq!(right_uv.max.y, 0.25);
    // Every piece samples the texture where it lies on the bar: its
    // logical corners over 352 by 17, scaled into the UV rectangle.
    // A 15 by 16 icon fills the bar's height below row 0: three pieces.
    assert_eq!(pieces.len(), 3);
    for (rect, piece_uv) in &pieces {
        let to_uv = |point: egui::Pos2| {
            let logical = (point - bar.min) / 2.0;
            egui::pos2(logical.x / 352.0 * 0.5, logical.y / 17.0 * 0.25)
        };
        assert_eq!(piece_uv.min, to_uv(rect.min), "{rect:?}");
        assert_eq!(piece_uv.max, to_uv(rect.max), "{rect:?}");
    }
    // A 15 by 15 icon leaves the bar's last row below it: four pieces.
    let short = bar_around_icon(bar, uv, Some(egui::vec2(15.0, 15.0)), 2.0);
    assert_eq!(short.len(), 4);
    let (_, below_uv) = short
        .iter()
        .find(|(rect, _)| rect.min.y == 232.0 && rect.max.y == 234.0)
        .copied()
        .unwrap();
    assert_eq!(below_uv.min.y, 16.0 / 17.0 * 0.25);
    assert_eq!(below_uv.max.y, 0.25);
    // No icon: the whole bar.
    assert_eq!(bar_around_icon(bar, uv, None, 2.0), [(bar, uv)]);
}

#[test]
fn an_unread_row_takes_font_13_and_a_read_one_font_10() {
    // FUN_00468ab0 sets an item's +0x38 to 13 for an unread message, 10
    // once read (FUN_00469de0). A0 tooltip/170-index: unread
    // "Garrisons" is 47 by 7, Arial Bold at 10 ppem (font 13's bold
    // 12-pixel cell). The heading is the default font ("Advice
    // Messages" 101 wide, 13 ppem).
    let mut log = scroll_log(2);
    let read = log.messages().last().map(|msg| msg.id).unwrap();
    log.mark_message_read(read);
    let mut state = MessageIndexState::new();
    state.category = 0x7b;
    let lines = drive(&log, &mut state, vec![]).lines;
    let size = |text: &str| {
        lines
            .iter()
            .find(|line| line.text == text)
            .map(|line| line.size)
    };
    let unread = log.messages().iter().find(|msg| msg.unread).unwrap();
    assert_eq!(
        size(&unread.text),
        Some(crate::theme::gdi_font(13, 1.0).size)
    );
    let read = log.messages().iter().find(|msg| !msg.unread).unwrap();
    assert_eq!(size(&read.text), Some(crate::theme::gdi_font(10, 1.0).size));
    assert_eq!(
        size(category_heading(state.category).unwrap()),
        Some(crate::theme::gdi_font(crate::DEFAULT_FONT, 1.0).size)
    );
}

#[test]
fn a_wheel_notch_scrolls_a_row_and_a_click_takes_the_row_under_it() {
    // port: the original list has no wheel; a notch moves 21 pixels,
    // up to the last row and the 5-pixel margin.
    let log = scroll_log(12);
    let mut state = MessageIndexState::new();
    state.set_category(0x7b);
    let over = row_center(2);
    let notch = |dy: f32| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, dy),
        modifiers: egui::Modifiers::default(),
    };
    drive(&log, &mut state, vec![(over, vec![notch(-1.0)])]);
    assert_eq!(state.scroll_offset, 21);
    for _ in 0..5 {
        drive(&log, &mut state, vec![(over, vec![notch(-1.0)])]);
    }
    assert_eq!(state.scroll_offset, max_scroll(12));
    drive(&log, &mut state, vec![(over, vec![notch(1.0)])]);
    assert_eq!(state.scroll_offset, max_scroll(12) - 21);
    state.scroll_offset = 21;
    drive(&log, &mut state, click_at(row_center(0)));
    // Newest first, so the second row is message 10.
    let rows = filtered_messages(&log, 0x7b, true);
    assert_eq!(state.selected, BTreeSet::from([rows[1].id]));
    assert_eq!(rows[1].text, "Fleet message 10");
    // Below the list, the wheel scrolls nothing.
    drive(
        &log,
        &mut state,
        vec![(egui::pos2(100.0, 320.0), vec![notch(1.0)])],
    );
    assert_eq!(state.scroll_offset, 21);
}

fn click_at(pos: egui::Pos2) -> Vec<(egui::Pos2, Vec<egui::Event>)> {
    vec![
        (pos, vec![press(pos, true)]),
        (pos, vec![press(pos, false)]),
    ]
}

fn ctrl_click_at(pos: egui::Pos2) -> Vec<(egui::Pos2, Vec<egui::Event>)> {
    let mods = egui::Modifiers {
        command: true,
        ..Default::default()
    };
    vec![
        (pos, vec![press_with(pos, true, mods)]),
        (pos, vec![press_with(pos, false, mods)]),
    ]
}

fn shift_click_at(pos: egui::Pos2) -> Vec<(egui::Pos2, Vec<egui::Event>)> {
    let mods = egui::Modifiers {
        shift: true,
        ..Default::default()
    };
    vec![
        (pos, vec![press_with(pos, true, mods)]),
        (pos, vec![press_with(pos, false, mods)]),
    ]
}

#[test]
fn egui_click_selects_one_row() {
    // Manual p. 43: a click selects one message.
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    state.set_category(0x79);

    drive(&log, &mut state, click_at(row_center(1)));

    assert_eq!(state.selected, BTreeSet::from([rows[1].id]));
}

#[test]
fn egui_ctrl_click_toggles_selection() {
    // Manual p. 43: Ctrl-click toggles an individual message.
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    state.set_category(0x79);

    let mut frames = click_at(row_center(0));
    frames.extend(ctrl_click_at(row_center(2)));
    drive(&log, &mut state, frames);

    assert_eq!(state.selected, BTreeSet::from([rows[0].id, rows[2].id]));
}

#[test]
fn egui_shift_click_selects_contiguous_range() {
    // Manual p. 43: Shift-click selects a contiguous range.
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    state.set_category(0x79);

    let mut frames = click_at(row_center(0));
    frames.extend(shift_click_at(row_center(2)));
    drive(&log, &mut state, frames);

    assert_eq!(
        state.selected,
        BTreeSet::from([rows[0].id, rows[1].id, rows[2].id])
    );
}

#[test]
fn egui_double_click_switches_to_mode_2_and_marks_read() {
    // Manual p. 43: double-click opens (reads) the message.
    // FUN_00469de0: +0x38 = 10 (read font).
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    state.set_category(0x79);

    let pos = row_center(1);
    let frames = vec![
        (pos, vec![press(pos, true)]),
        (pos, vec![press(pos, false)]),
        (pos, vec![press(pos, true)]),
        (pos, vec![press(pos, false)]),
    ];
    let result = drive(&log, &mut state, frames);

    assert_eq!(state.mode, IndexMode::SingleMessage);
    assert_eq!(state.viewing, Some(rows[1].id));
    assert!(result
        .actions
        .contains(&MessageIndexAction::MessageDisplayed(rows[1].id)));
}

#[test]
fn egui_category_switch_resets_state_and_emits_actions() {
    // FUN_004697b0: switching category rebuilds the list.
    let log = test_log();
    let rows = make_rows(&log);
    let mut state = MessageIndexState::new();
    apply_click(&mut state, 1, &rows, egui::Modifiers::NONE);

    let actions = state.set_category(0x82);

    assert!(actions.contains(&MessageIndexAction::AdviceShown));
    assert!(state.selected.is_empty());
    assert_eq!(state.category, 0x82);
}
