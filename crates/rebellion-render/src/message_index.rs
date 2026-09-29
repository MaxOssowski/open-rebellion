//! Original Message Index bitmap shell.
//!
//! `FUN_0042a240` constructs the window through `FUN_00466350`, while
//! `FUN_004665f0` composes the faction shell and ten message-category controls.
//! This module deliberately contains no replacement message interface.

use egui_macroquad::egui::{self, Color32};

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::CockpitFaction;

/// Native Message Index resource surface.
pub const MESSAGE_INDEX_WIDTH: f32 = 470.0;
pub const MESSAGE_INDEX_HEIGHT: f32 = 331.0;

const INDEX_CONTENT_X: f32 = 12.0;
const INDEX_CONTENT_Y: f32 = 13.0;
const INDEX_CONTENT_WIDTH: f32 = 400.0;
const INDEX_CONTENT_HEIGHT: f32 = 306.0;
const INDEX_CONTROLS_X: f32 = 22.0;
const INDEX_CONTROLS_Y: f32 = 46.0;

const INDEX_BASE_ALLIANCE: u32 = 10_335;
const INDEX_BASE_EMPIRE: u32 = 10_336;
const INDEX_RAIL_ALLIANCE: u32 = 10_820;
const INDEX_RAIL_EMPIRE: u32 = 10_821;
const INDEX_CONTENT: u32 = 10_822;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MessageCategoryControlSpec {
    command_id: u16,
    x: u16,
    width: u16,
    normal_resource: u32,
    pressed_resource: u32,
}

const fn message_category_control_specs(
    faction: CockpitFaction,
) -> [MessageCategoryControlSpec; 10] {
    let (
        faction_normal,
        faction_pressed,
        fleet_normal,
        fleet_pressed,
        mission_normal,
        mission_pressed,
        advice_normal,
        advice_pressed,
    ) = match faction {
        CockpitFaction::Alliance => (
            10_832, 10_833, 10_840, 10_841, 10_842, 10_843, 10_846, 10_847,
        ),
        CockpitFaction::Empire => (
            10_834, 10_835, 10_838, 10_839, 10_844, 10_845, 10_848, 10_849,
        ),
    };
    [
        MessageCategoryControlSpec {
            command_id: 0x79,
            x: 0,
            width: 36,
            normal_resource: 10_830,
            pressed_resource: 10_831,
        },
        MessageCategoryControlSpec {
            command_id: 0x7a,
            x: 38,
            width: 36,
            normal_resource: faction_normal,
            pressed_resource: faction_pressed,
        },
        MessageCategoryControlSpec {
            command_id: 0x7b,
            x: 76,
            width: 36,
            normal_resource: fleet_normal,
            pressed_resource: fleet_pressed,
        },
        MessageCategoryControlSpec {
            command_id: 0x7c,
            x: 114,
            width: 35,
            normal_resource: mission_normal,
            pressed_resource: mission_pressed,
        },
        MessageCategoryControlSpec {
            command_id: 0x7d,
            x: 151,
            width: 36,
            normal_resource: 10_836,
            pressed_resource: 10_837,
        },
        MessageCategoryControlSpec {
            command_id: 0x7e,
            x: 189,
            width: 36,
            normal_resource: 10_852,
            pressed_resource: 10_853,
        },
        MessageCategoryControlSpec {
            command_id: 0x7f,
            x: 227,
            width: 34,
            normal_resource: 10_856,
            pressed_resource: 10_857,
        },
        MessageCategoryControlSpec {
            command_id: 0x80,
            x: 263,
            width: 36,
            normal_resource: 10_854,
            pressed_resource: 10_855,
        },
        MessageCategoryControlSpec {
            command_id: 0x81,
            x: 301,
            width: 35,
            normal_resource: 10_850,
            pressed_resource: 10_851,
        },
        MessageCategoryControlSpec {
            command_id: 0x82,
            x: 338,
            width: 37,
            normal_resource: advice_normal,
            pressed_resource: advice_pressed,
        },
    ]
}

/// Paint the recovered Message Index shell and its ten original category
/// controls.
///
/// The command identities are All (`0x79`), Popular Support (`0x7a`), Fleet
/// (`0x7b`), Mission (`0x7c`), Resource (`0x7d`), Manufacturing (`0x7e`),
/// Defense (`0x7f`), Conflict (`0x80`), Chat (`0x81`), and Advice (`0x82`).
pub fn draw_message_index_shell(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: CockpitFaction,
    origin: egui::Pos2,
    scale: f32,
) -> Option<u16> {
    if scale <= 0.0 {
        return None;
    }
    let mut selected = None;
    egui::Area::new(egui::Id::new("original-message-index-shell"))
        .fixed_pos(origin)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let size = egui::vec2(MESSAGE_INDEX_WIDTH * scale, MESSAGE_INDEX_HEIGHT * scale);
            let (window_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            let (base, rail) = match faction {
                CockpitFaction::Alliance => (INDEX_BASE_ALLIANCE, INDEX_RAIL_ALLIANCE),
                CockpitFaction::Empire => (INDEX_BASE_EMPIRE, INDEX_RAIL_EMPIRE),
            };
            paint_strategy_resource(ui.painter(), ctx, cache, base, window_rect);
            paint_strategy_resource(
                ui.painter(),
                ctx,
                cache,
                rail,
                message_index_rect(window_rect, scale, 412.0, 0.0, 58.0, 330.0),
            );
            paint_strategy_resource(
                ui.painter(),
                ctx,
                cache,
                INDEX_CONTENT,
                message_index_rect(
                    window_rect,
                    scale,
                    INDEX_CONTENT_X,
                    INDEX_CONTENT_Y,
                    INDEX_CONTENT_WIDTH,
                    INDEX_CONTENT_HEIGHT,
                ),
            );

            let (pointer, primary_down) = ctx.input(|input| {
                (
                    input.pointer.interact_pos(),
                    input.pointer.button_down(egui::PointerButton::Primary),
                )
            });
            for control in message_category_control_specs(faction) {
                let rect = message_index_rect(
                    window_rect,
                    scale,
                    INDEX_CONTROLS_X + f32::from(control.x),
                    INDEX_CONTROLS_Y,
                    f32::from(control.width),
                    41.0,
                );
                let response = ui.interact(
                    rect,
                    ui.id().with(("message-index-category", control.command_id)),
                    egui::Sense::click(),
                );
                let pressed = primary_down
                    && pointer.is_some_and(|point| message_index_rect_contains(rect, point));
                paint_strategy_resource(
                    ui.painter(),
                    ctx,
                    cache,
                    if pressed {
                        control.pressed_resource
                    } else {
                        control.normal_resource
                    },
                    rect,
                );
                if response.clicked()
                    && response
                        .interact_pointer_pos()
                        .is_some_and(|point| message_index_rect_contains(rect, point))
                {
                    selected = Some(control.command_id);
                }
            }
        });
    selected
}

fn message_index_rect(
    parent: egui::Rect,
    scale: f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(parent.min.x + x * scale, parent.min.y + y * scale),
        egui::vec2(width * scale, height * scale),
    )
}

fn message_index_rect_contains(rect: egui::Rect, point: egui::Pos2) -> bool {
    point.x >= rect.min.x && point.x < rect.max.x && point.y >= rect.min.y && point.y < rect.max.y
}

fn paint_strategy_resource(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    resource_id: u32,
    rect: egui::Rect,
) {
    let Some(texture) = cache.get(ctx, DllSource::Strategy, resource_id) else {
        return;
    };
    painter.image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// Fixed-position test adapter for deterministic browser inspection.
#[cfg(feature = "interface-test-fixtures")]
pub fn draw_message_index_fixture(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: CockpitFaction,
) {
    let _ = draw_message_index_shell(ctx, cache, faction, egui::pos2(85.0, 55.0), 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
