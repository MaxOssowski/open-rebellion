//! Original Message Index bitmap shell and row interaction.
//!
//! `FUN_0042a240` constructs the window through `FUN_00466350`, while
//! `FUN_004665f0` composes the faction shell and ten message-category controls.
//! `FUN_00468ab0` fills the list, `FUN_00468f20` filters by category, and
//! `FUN_00468fb0` transitions between modes 1 (index) and 2 (single message).

use std::collections::BTreeSet;

use egui_macroquad::egui::{self, Color32};

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::encyclopedia::EncyclopediaState;
use crate::message_log::{GameMessage, MessageId, MessageLog, MessagePicture, MessageRail};
use rebellion_core::ids::SystemKey;

// ---------------------------------------------------------------------------
// Shell constants (unchanged)
// ---------------------------------------------------------------------------

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
/// Pane `0x34` (`FUN_004665f0`): the black title strip `0x2ab5` at
/// (12, 14) and the black text area `0x2ab4` at (12, 232), which mode 2
/// shows instead of the list pane `0x32`.
const MESSAGE_TITLE_STRIP: (u32, (f32, f32, f32, f32)) = (10_933, (12.0, 14.0, 400.0, 18.0));
const MESSAGE_TEXT_PANE: (u32, (f32, f32, f32, f32)) = (10_932, (12.0, 232.0, 400.0, 87.0));
/// Where a message's title starts in the title strip (launchbox frame
/// "event-imperial-future-jedi": the title begins 41 pixels into the
/// window); the chosen item's image left of it ([`title_icon_layout`]).
const MESSAGE_TITLE_X: f32 = 40.0;

// ---------------------------------------------------------------------------
// List geometry (FUN_004665f0, FUN_006082c0 in FUN_00468ab0)
// ---------------------------------------------------------------------------

/// List position within the window, from `FUN_004665f0`: (25, 108).
const LIST_X: f32 = 25.0;
const LIST_Y: f32 = 108.0;
/// List size: 368 by 194 (`0x170 x 0xc2`).
const LIST_WIDTH: f32 = 368.0;
const LIST_HEIGHT: f32 = 194.0;
/// Row dimensions from `FUN_006082c0(_, 0x15a, 0x15)`.
const ROW_WIDTH: f32 = 346.0;
const ROW_HEIGHT: f32 = 21.0;
/// Text offset within a row (`FUN_00468ab0`: `+0x30 = 0x20`).
const TEXT_INSET: f32 = 32.0;
/// The list heading's cell in the window (A0 2026-10-29).
const HEADING_AT: (f32, f32) = (35.0, 91.0);
/// The row text's cell below the row's top (A0 2026-10-29).
const ROW_TEXT_Y: f32 = 2.0;
/// The rows' fonts (`FUN_00468ab0`, `+0x38`): font 13 (the bold 12-pixel
/// cell, 10 ppem) for an unread message, font 10 once read (`FUN_00469de0`).
/// A0 tooltip/170-index: unread "Garrisons" is 47 by 7, Arial Bold at 10
/// ppem, its glyphs one pixel wide.
const ROW_FONT_UNREAD: u16 = 13;
const ROW_FONT_READ: u16 = 10;
/// An unchosen row's text, the list's grey (`FUN_00607ea0`, as the Finders).
const ROW_COLOR: Color32 = Color32::from_rgb(0x78, 0x78, 0x78);
/// A chosen row's bar, STRATEGY `0x2aa2` (Alliance) / `0x2aa3` (Empire)
/// (`FUN_00468ab0`), 356 by 21, of which A0 shows 352 by 17.
const ROW_CHOSEN_ALLIANCE: u32 = 0x2aa2;
const ROW_CHOSEN_EMPIRE: u32 = 0x2aa3;
const ROW_CHOSEN_SIZE: (f32, f32) = (352.0, 17.0);
/// A row's category icon cell's width and least height: the icons are 15
/// by 15 or 15 by 16, drawn in their own size.
const ROW_ICON_SIZE: f32 = 15.0;
/// Text area for single-message display (`FUN_00469de0`).
const TEXT_AREA_X: f32 = 17.0;
const TEXT_AREA_Y: f32 = 234.0;
/// Default width for non-encyclopedia messages (`0x18b`).
const TEXT_AREA_WIDTH: f32 = 395.0;
/// A question's text width (`FUN_00469de0`: `0x140` for kind 5).
const QUESTION_TEXT_AREA_WIDTH: f32 = 320.0;
const TEXT_AREA_HEIGHT: f32 = 80.0;

/// Visible rows the list shows at once: `194 / 21 = 9`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Nine whole rows of 21 in a list 194 tall."
)]
/// A row in whole pixels.
const ROW_PIXELS: usize = 21;
/// The list's bar width (RCDATA `0x299d`, 12) and arrow step. hyp: the
/// step is the Finders' 10 (`FUN_00607ea0`'s argument there).
const BAR_WIDTH: f32 = 12.0;
const LIST_STEP: f32 = 10.0;

/// The list's scroll range for `rows` rows, in pixels
/// ([`crate::list_scroll_bar::max_first`]).
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "A few hundred rows of 21 pixels."
)]
fn max_scroll(rows: usize) -> usize {
    crate::list_scroll_bar::max_first((rows * ROW_PIXELS) as f32, LIST_HEIGHT) as usize
}

// ---------------------------------------------------------------------------
// Category control specs (unchanged)
// ---------------------------------------------------------------------------

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

/// The category a cockpit command opens the window on (`FUN_00422ce0`):
/// F6 (`0x75`) All, the rail `0x136..0x13c` Popular Support through
/// Conflict, `0x13d` Advice and `0x13e` Chat.
#[must_use]
pub fn opening_category(command: u16) -> Option<u16> {
    match command {
        0x75 => Some(0x79),
        0x136..=0x13c => Some(command - 0x136 + 0x7a),
        0x13d => Some(0x82),
        0x13e => Some(0x81),
        _ => None,
    }
}

/// The window's screen rectangle. hyp: `FUN_00466350` places it at the
/// parent's origin with flags 7; the port centers it in the galaxy view, as
/// the Finders.
#[must_use]
pub fn window_rect(layout: CockpitLayout) -> egui::Rect {
    crate::mission_dialog::galaxy_centered_rect(layout, MESSAGE_INDEX_WIDTH, MESSAGE_INDEX_HEIGHT)
}

// ---------------------------------------------------------------------------
// Window buttons (FUN_004665f0)
// ---------------------------------------------------------------------------

/// One of the window's own bitmap buttons, in window coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowButtonSpec {
    pub command_id: u16,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub normal_resource: u32,
    pub pressed_resource: u32,
}

/// Close (`0x28`, `0x2882`/`0x2888`), Display Message (`0x65`) and the
/// posting toggle (`0x66`) on the right rail,
/// and Select All (`0x90`, `0x2a94`) and Delete (`0x91`, `0x2a96`) under
/// the category row, as `FUN_004665f0` builds them. Select All and Delete
/// show in mode 1 only (`FUN_00468fb0`), the scroll buttons (`0x96`,
/// `0x9a`) in mode 2. The posting toggle shows `0x2a78` / `0x2a7a` while
/// the category alerts and `0x2ac7` / `0x2ac9` while it is `silent`
/// (`FUN_004697b0`, `FUN_00467f10`). Display Message shows `0x2a76` /
/// `0x29f2` in mode 1 and, as the way back, `0x2884` / `0x288a` in mode 2
/// (`FUN_00468fb0`); with no row in the list it is disabled
/// ([`disabled_rail_controls`], `FUN_004697b0`). port: the mode-2
/// Encyclopedia and Detail buttons are not drawn.
#[must_use]
pub fn window_button_specs(
    faction: CockpitFaction,
    mode: IndexMode,
    silent: bool,
    list_empty: bool,
    display_enabled: bool,
) -> Vec<WindowButtonSpec> {
    let (x, size, close, display, post_y, post) = match faction {
        CockpitFaction::Alliance => (
            0x1a7,
            (0x20, 0x1f),
            (0x19, 0x2882),
            (
                0x5d,
                if mode == IndexMode::List {
                    0x2a76
                } else {
                    0x2884
                },
            ),
            0x93,
            if silent {
                (0x2ac7, 0x2a79)
            } else {
                (0x2a78, 0x2a79)
            },
        ),
        CockpitFaction::Empire => (
            0x1aa,
            (0x2c, 0x29),
            (0x15, 0x2888),
            (
                0x59,
                if mode == IndexMode::List {
                    0x29f2
                } else {
                    0x288a
                },
            ),
            0x94,
            if silent {
                (0x2ac9, 0x2a7b)
            } else {
                (0x2a7a, 0x2a7b)
            },
        ),
    };
    let rail = |command_id, (y, normal): (u16, u32)| WindowButtonSpec {
        command_id,
        x,
        y,
        width: size.0,
        height: size.1,
        normal_resource: normal,
        pressed_resource: normal + 1,
    };
    let mut buttons = vec![rail(0x28, close)];
    if mode != IndexMode::List || !list_empty {
        buttons.push(rail(0x65, display));
    }
    buttons.push(WindowButtonSpec {
        y: post_y,
        normal_resource: post.0,
        pressed_resource: post.1,
        ..rail(0x66, (0, 0))
    });
    if display_enabled {
        // FUN_004697b0: Display for a message of kind 3 or 5 with a target
        // (0x2916 / 0x2918, label 0x8030).
        let (y, art) = match faction {
            CockpitFaction::Alliance => (0xc9, 0x2916),
            CockpitFaction::Empire => (0xcf, 0x2918),
        };
        buttons.push(WindowButtonSpec {
            y,
            ..rail(0x67, (0, art))
        });
    }
    if mode == IndexMode::SingleMessage {
        // FUN_004665f0 / FUN_00468fb0: the scroll buttons, 19 by 15 in the
        // title strip, shown in mode 2.
        for (command_id, x, normal) in [(0x96, 390, 10_919), (0x9a, 367, 10_948)] {
            buttons.push(WindowButtonSpec {
                command_id,
                x,
                y: 15,
                width: 19,
                height: 15,
                normal_resource: normal,
                pressed_resource: normal + 1,
            });
        }
    }
    if mode == IndexMode::List {
        buttons.push(WindowButtonSpec {
            command_id: 0x90,
            x: 0x11a,
            y: 0x57,
            width: 0x38,
            height: 0x14,
            normal_resource: 0x2a94,
            pressed_resource: 0x2a95,
        });
        buttons.push(WindowButtonSpec {
            command_id: 0x91,
            x: 0x154,
            y: 0x57,
            width: 0x38,
            height: 0x14,
            normal_resource: 0x2a96,
            pressed_resource: 0x2a97,
        });
    }
    buttons
}

/// The rail's disabled controls in their disabled art (`FUN_004665f0`'s
/// and `FUN_00468fb0`'s third bitmap, `FUN_00603150(2, ..)`): window
/// point, size and resource. Display (`0x67`) unless the chosen message has
/// a target to display, and Compose Chat (`0x68`) always; A0 (2026-10-29,
/// the Empire's Advice list) shows both so, 0x2ad0 and 0x2a81 exactly.
/// Display Message (`0x65`) in mode 1 while the list is empty
/// (`FUN_004697b0`: 0x2ad1 / 0x2ad2). hyp: Compose Chat stays disabled
/// without a multiplayer session.
#[must_use]
pub fn disabled_rail_controls(
    faction: CockpitFaction,
    mode: IndexMode,
    list_empty: bool,
    display: bool,
) -> Vec<(u16, u16, u16, u16, u32)> {
    let (mut controls, display_message) = match faction {
        CockpitFaction::Alliance => (
            vec![
                (0x1a7, 0xc9, 0x20, 0x1f, 0x2acf),
                (0x1a7, 0xff, 0x20, 0x1f, 0x2a7e),
            ],
            (0x1a7, 0x5d, 0x20, 0x1f, 0x2ad1),
        ),
        CockpitFaction::Empire => (
            vec![
                (0x1aa, 0xcf, 0x2c, 0x29, 0x2ad0),
                (0x1aa, 0x10a, 0x2c, 0x29, 0x2a81),
            ],
            (0x1aa, 0x59, 0x2c, 0x29, 0x2ad2),
        ),
    };
    if display {
        // Display (0x67) is enabled; [`window_button_specs`] holds it.
        controls.remove(0);
    }
    if mode == IndexMode::List && list_empty {
        controls.push(display_message);
    }
    controls
}

/// A rail or list button's tooltip (`FUN_004665f0`, TEXTSTRA): Close
/// 0x1954; Display Message 0x8020, renamed 0x8019 "Message Index" while
/// one message shows (`FUN_00468fb0`, A0 tooltip/90); the posting toggle
/// 0x8005 "Post Messages Silently", or 0x8006 "Post Messages with Alert"
/// for a silent category (`FUN_004697b0`, `FUN_00467f10`); Display 0x8030;
/// Select All 0x8008; Delete 0x8009. The scroll buttons have none.
#[must_use]
pub const fn button_tooltip_string(command: u16, mode: IndexMode, silent: bool) -> Option<u16> {
    match command {
        0x28 => Some(0x1954),
        0x65 => Some(match mode {
            IndexMode::List => 0x8020,
            IndexMode::SingleMessage => 0x8019,
        }),
        0x66 => Some(if silent { 0x8006 } else { 0x8005 }),
        0x67 => Some(0x8030),
        0x90 => Some(0x8008),
        0x91 => Some(0x8009),
        _ => None,
    }
}

/// A category control's tooltip (`FUN_004665f0`): its category's name,
/// TEXTSTRA 0x8010 "All Messages" and the rail's names (A0 tooltip/71..80).
#[must_use]
pub const fn category_tooltip_string(command: u16) -> Option<u16> {
    match command {
        0x79 => Some(0x8010), // All Messages
        0x7a => Some(0x8011), // Popular Support Messages
        0x7b => Some(0x8017), // Fleet Messages
        0x7c => Some(0x8014), // Mission Messages
        0x7d => Some(0x8012), // Resource Messages
        0x7e => Some(0x8013), // Manufacturing Messages
        0x7f => Some(0x8016), // Defense Messages
        0x80 => Some(0x8018), // Conflict Messages
        0x81 => Some(0x8015), // Chat Messages
        0x82 => Some(0x8032), // Advice Messages
        _ => None,
    }
}

/// Paint the window's buttons and return the one clicked.
fn draw_window_buttons(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: CockpitFaction,
    origin: egui::Pos2,
    scale: f32,
    mode: IndexMode,
    silent: bool,
    list_empty: bool,
    display: bool,
    question: bool,
) -> Option<u16> {
    let mut clicked = None;
    egui::Area::new(egui::Id::new("original-message-index-buttons"))
        .fixed_pos(origin)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            let window_rect = egui::Rect::from_min_size(
                origin,
                egui::vec2(MESSAGE_INDEX_WIDTH * scale, MESSAGE_INDEX_HEIGHT * scale),
            );
            let (pointer, primary_down) = ctx.input(|input| {
                (
                    input.pointer.interact_pos(),
                    input.pointer.button_down(egui::PointerButton::Primary),
                )
            });
            for (x, y, width, height, resource) in
                disabled_rail_controls(faction, mode, list_empty, display)
            {
                paint_strategy_resource(
                    ui.painter(),
                    ctx,
                    cache,
                    resource,
                    message_index_rect(
                        window_rect,
                        scale,
                        f32::from(x),
                        f32::from(y),
                        f32::from(width),
                        f32::from(height),
                    ),
                );
            }
            for button in window_button_specs(faction, mode, silent, list_empty, display)
                .into_iter()
                .chain(question_button_specs(mode, question))
            {
                let rect = message_index_rect(
                    window_rect,
                    scale,
                    f32::from(button.x),
                    f32::from(button.y),
                    f32::from(button.width),
                    f32::from(button.height),
                );
                let response = ui.interact(
                    rect,
                    ui.id().with(("message-index-button", button.command_id)),
                    egui::Sense::click(),
                );
                if let Some(string) = button_tooltip_string(button.command_id, mode, silent) {
                    crate::win_tooltip::response_tool(&response, rect, string);
                }
                let pressed = primary_down
                    && pointer.is_some_and(|point| message_index_rect_contains(rect, point));
                paint_strategy_resource(
                    ui.painter(),
                    ctx,
                    cache,
                    if pressed {
                        button.pressed_resource
                    } else {
                        button.normal_resource
                    },
                    rect,
                );
                if response.clicked() {
                    clicked = Some(button.command_id);
                }
            }
        });
    clicked
}

// ---------------------------------------------------------------------------
// Shell drawing (preserved)
// ---------------------------------------------------------------------------

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
    draw_shell(ctx, cache, faction, origin, scale, IndexMode::List, None)
}

/// Whether a category control shows its pressed art. The controls are a
/// radio group (`+0x160`): the chosen category stays pressed, as A0 shows
/// (2026-10-29, the Empire's Advice control matches 10849 exactly), and a
/// held press shows on any.
const fn category_pressed(chosen: Option<u16>, command: u16, held: bool) -> bool {
    held || matches!(chosen, Some(chosen) if chosen == command)
}

/// The window's background layers, resource and window rectangle: the
/// side's base and rail, then the list pane `0x32` in mode 1 or the
/// message pane `0x34` in mode 2 (`FUN_004665f0`, `FUN_00468fb0`'s
/// `FUN_006075e0(0x32 | 0x34)`).
fn shell_layers(faction: CockpitFaction, mode: IndexMode) -> Vec<(u32, (f32, f32, f32, f32))> {
    let (base, rail) = match faction {
        CockpitFaction::Alliance => (INDEX_BASE_ALLIANCE, INDEX_RAIL_ALLIANCE),
        CockpitFaction::Empire => (INDEX_BASE_EMPIRE, INDEX_RAIL_EMPIRE),
    };
    let mut layers = vec![
        (base, (0.0, 0.0, MESSAGE_INDEX_WIDTH, MESSAGE_INDEX_HEIGHT)),
        (rail, (412.0, 0.0, 58.0, 330.0)),
    ];
    match mode {
        IndexMode::List => layers.push((
            INDEX_CONTENT,
            (
                INDEX_CONTENT_X,
                INDEX_CONTENT_Y,
                INDEX_CONTENT_WIDTH,
                INDEX_CONTENT_HEIGHT,
            ),
        )),
        IndexMode::SingleMessage => layers.extend([MESSAGE_TITLE_STRIP, MESSAGE_TEXT_PANE]),
    }
    layers
}

/// Paint the shell for `mode`; the category controls show in mode 1 only
/// (`FUN_00468fb0` hides the list and its controls for a message).
fn draw_shell(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: CockpitFaction,
    origin: egui::Pos2,
    scale: f32,
    mode: IndexMode,
    chosen: Option<u16>,
) -> Option<u16> {
    if scale <= 0.0 {
        return None;
    }
    let mut selected = None;
    egui::Area::new(egui::Id::new("original-message-index-shell"))
        .fixed_pos(origin)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            let size = egui::vec2(MESSAGE_INDEX_WIDTH * scale, MESSAGE_INDEX_HEIGHT * scale);
            let (window_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            // The window's own black background: `FUN_00466350` makes the
            // window 470 by 331, and the base's keyed bottom row shows it
            // (A0 modal2/17-msg row 382 is black across the window).
            ui.painter()
                .rect_filled(window_rect, 0.0, egui::Color32::BLACK);
            for (resource, (x, y, width, height)) in shell_layers(faction, mode) {
                paint_strategy_resource(
                    ui.painter(),
                    ctx,
                    cache,
                    resource,
                    message_index_rect(window_rect, scale, x, y, width, height),
                );
            }
            if mode != IndexMode::List {
                return;
            }

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
                if let Some(string) = category_tooltip_string(control.command_id) {
                    crate::win_tooltip::response_tool(&response, rect, string);
                }
                let pressed = category_pressed(
                    chosen,
                    control.command_id,
                    primary_down
                        && pointer.is_some_and(|point| message_index_rect_contains(rect, point)),
                );
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

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

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

/// The checkmark (`0x98`) and X (`0x99`) a Mission Report's question shows
/// beside its text in mode 2 (`FUN_004665f0` places them at (355, 244) and
/// (355, 281), 51 by 35; `FUN_00469de0` gives a kind-5 message the art
/// `0x2aae`/`0x2aaf` and `0x2ab1`/`0x2ab2`).
#[must_use]
pub fn question_button_specs(mode: IndexMode, question: bool) -> Vec<WindowButtonSpec> {
    if mode != IndexMode::SingleMessage || !question {
        return Vec::new();
    }
    [(0x98, 0xf4, 0x2aae), (0x99, 0x119, 0x2ab1)]
        .into_iter()
        .map(|(command_id, y, normal)| WindowButtonSpec {
            command_id,
            x: 0x163,
            y,
            width: 0x33,
            height: 0x23,
            normal_resource: normal,
            pressed_resource: normal + 1,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// IndexMode, Actions, State
// ---------------------------------------------------------------------------

/// Display mode of the Message Index window (`FUN_00468fb0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexMode {
    /// Mode 1: the scrollable message list.
    List,
    /// Mode 2: viewing a single message's text.
    SingleMessage,
}

/// What the Message Index window asks of the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageIndexAction {
    /// Entering the Advice category: drop game speed to Very Slow.
    /// `FUN_004697b0` case `0x82` -> `FUN_0041d3f0` -> `FUN_00487ff0(1)`.
    AdviceShown,
    /// Leaving the Advice category or closing the window: restore speed.
    /// `FUN_004697b0` other cases -> `FUN_0041d410` -> `FUN_00487ff0(0)`.
    AdviceHidden,
    /// Closing the window: mark messages in this category read.
    /// `None` for All Messages (mark everything read); `Some(rail)` for one.
    /// `FUN_0048a530`.
    MarkCategoryRead(Option<MessageRail>),
    /// Delete the selected messages from the log.
    /// `FUN_005f54a0` per selected message.
    DeleteSelected(Vec<MessageId>),
    /// A single message was opened for display (mode 2).
    /// The app should mark it read (`FUN_00469de0`: `+0x38 = 10`).
    MessageDisplayed(MessageId),
    /// The Close button (`0x28`): the app applies
    /// [`MessageIndexState::close`] and drops the window.
    Close,
    /// Display (`0x67`, `FUN_00467f10`): the app goes to the chosen
    /// message's object (`FUN_00429440`). port: a message carries its
    /// system, whose sector window opens with the system chosen.
    Display(SystemKey),
    /// A Mission Report's checkmark (`0x98`): the mission continues and
    /// the message asks no more.
    ContinueMission { message: MessageId },
    /// A Mission Report's X (`0x99`): the app aborts `mission` without
    /// asking (`FUN_0041ce20(order, 1)`), when its validator allows.
    AbortMission { message: MessageId, mission: u64 },
    /// The posting toggle (`0x66`, `FUN_00467f10`): the app flips the
    /// category's silent bit ([`MessageLog::toggle_silent`]); `None` is All
    /// Messages' default.
    ToggleSilent(Option<u16>),
}

/// Mutable state for the Message Index window.
#[derive(Debug, Clone)]
pub struct MessageIndexState {
    /// Current display mode.
    mode: IndexMode,
    /// The active category command (`0x79..0x82`), default All (`0x79`).
    category: u16,
    /// Selected message ids.
    selected: BTreeSet<MessageId>,
    /// Anchor row index for shift-click range selection (manual pp. 78-80).
    anchor: Option<usize>,
    /// The list's first pixel (its bar's `+0xe0`).
    scroll_offset: usize,
    /// A press held on the list's bar.
    scroll_hold: Option<crate::list_scroll_bar::Hold>,
    /// Message being viewed in mode 2.
    viewing: Option<MessageId>,
    /// Mode 2's text control (`FUN_0041ecf0`): its first pixel, for which
    /// message, and a press held on its bar.
    text_scroll: (f32, Option<MessageId>),
    text_scroll_hold: Option<crate::list_scroll_bar::Hold>,
    /// Last clicked row and when, for double-click detection.
    /// port: egui counts double clicks across widgets; the list keeps its own
    /// (same approach as `fleet_finder.rs`).
    last_clicked: Option<(usize, f64)>,
}

impl Default for MessageIndexState {
    fn default() -> Self {
        Self {
            mode: IndexMode::List,
            category: 0x79,
            selected: BTreeSet::new(),
            anchor: None,
            scroll_offset: 0,
            scroll_hold: None,
            viewing: None,
            text_scroll: (0.0, None),
            text_scroll_hold: None,
            last_clicked: None,
        }
    }
}

impl MessageIndexState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The active category command.
    #[must_use]
    pub fn category(&self) -> u16 {
        self.category
    }

    /// Current display mode.
    #[must_use]
    pub fn mode(&self) -> IndexMode {
        self.mode
    }

    /// The [`MessageRail`] for the active category, or `None` for All.
    #[must_use]
    pub fn active_rail(&self) -> Option<MessageRail> {
        category_rail(self.category)
    }

    /// The selected message ids.
    #[must_use]
    pub fn selected(&self) -> &BTreeSet<MessageId> {
        &self.selected
    }

    /// Select All (`0x90`, `FUN_00467f10`): choose every row the list
    /// shows.
    pub fn select_all(&mut self, rows: &[&GameMessage]) {
        self.selected = rows.iter().map(|message| message.id).collect();
    }

    /// Mode 2's text starts at its top for each message shown: the
    /// control is refilled (`FUN_00469de0` -> `FUN_0041fc30`).
    fn follow_text(&mut self, id: MessageId) {
        if self.text_scroll.1 != Some(id) {
            self.text_scroll = (0.0, Some(id));
            self.text_scroll_hold = None;
        }
    }

    /// Switch the category, returning actions the app must handle.
    ///
    /// `FUN_004697b0`: switching to `0x82` (Advice) drops speed; switching
    /// away restores it.
    pub fn set_category(&mut self, command: u16) -> Vec<MessageIndexAction> {
        if command == self.category {
            return Vec::new();
        }
        let mut actions = Vec::new();
        let was_advice = self.category == 0x82;
        let is_advice = command == 0x82;

        if was_advice && !is_advice {
            actions.push(MessageIndexAction::AdviceHidden);
        }
        if is_advice && !was_advice {
            actions.push(MessageIndexAction::AdviceShown);
        }

        self.category = command;
        self.selected.clear();
        self.anchor = None;
        self.scroll_offset = 0;
        self.last_clicked = None;

        if self.mode != IndexMode::List {
            self.mode = IndexMode::List;
            self.viewing = None;
        }

        actions
    }

    /// Actions the app must handle when the window closes.
    ///
    /// `FUN_0046a6a0`: restore speed (`FUN_0041d410`) and mark the active
    /// category read (`FUN_0048a530`).
    pub fn close(&self) -> Vec<MessageIndexAction> {
        vec![
            MessageIndexAction::AdviceHidden,
            MessageIndexAction::MarkCategoryRead(self.active_rail()),
        ]
    }

    /// Delete the currently selected messages, choosing the row after the
    /// first of them in `rows`. Returns a
    /// [`MessageIndexAction::DeleteSelected`] the app must apply.
    pub fn delete_selected(&mut self, rows: &[&GameMessage]) -> Option<MessageIndexAction> {
        if self.selected.is_empty() {
            return None;
        }
        let ids: Vec<MessageId> = self.selected.iter().copied().collect();
        // FUN_00467f10 (0x91, mode 1): the row after the first chosen one,
        // or before it at the list's end, is chosen once they go (A0
        // message-delete/del4, del6, del9). port: a successor that goes
        // too, which the original would choose freed, leaves none.
        let first = rows.iter().position(|row| self.selected.contains(&row.id));
        let successor = first
            .and_then(|at| {
                rows.get(at + 1)
                    .or_else(|| at.checked_sub(1).and_then(|before| rows.get(before)))
            })
            .map(|row| row.id)
            .filter(|id| !self.selected.contains(id));
        let remaining: Vec<MessageId> = rows
            .iter()
            .map(|row| row.id)
            .filter(|id| !self.selected.contains(id))
            .collect();
        self.selected.clear();
        self.anchor = None;
        self.last_clicked = None;
        if let Some(id) = successor {
            self.selected.insert(id);
            self.anchor = remaining.iter().position(|row| *row == id);
        }
        Some(MessageIndexAction::DeleteSelected(ids))
    }

    /// The Display Message button (`0x65`, `FUN_00468fb0`): in mode 1 it
    /// shows the first selected message, and an empty selection stays put;
    /// in mode 2 it returns to the list.
    pub fn toggle_display(&mut self) -> Option<MessageIndexAction> {
        match self.mode {
            IndexMode::List => {
                let id = *self.selected.first()?;
                self.mode = IndexMode::SingleMessage;
                self.viewing = Some(id);
                self.last_clicked = None;
                Some(MessageIndexAction::MessageDisplayed(id))
            }
            IndexMode::SingleMessage => {
                self.return_to_list();
                None
            }
        }
    }

    /// Mode 2's scroll buttons: show the neighbouring message in the
    /// category's list (`FUN_0046a200` enables each only while it has a
    /// neighbour). port: `0x9a` at x 367 is drawn as the up arrow (10948)
    /// and `0x96` at x 390 as the down arrow (10919), so `0x9a` goes to the
    /// earlier message and `0x96` to the later one (inferred from the art).
    pub fn view_adjacent(
        &mut self,
        rows: &[MessageId],
        forward: bool,
    ) -> Option<MessageIndexAction> {
        let current = rows.iter().position(|id| Some(*id) == self.viewing)?;
        let next = if forward {
            current.checked_add(1)?
        } else {
            current.checked_sub(1)?
        };
        let id = *rows.get(next)?;
        self.viewing = Some(id);
        Some(MessageIndexAction::MessageDisplayed(id))
    }

    /// Return to mode 1 (list) from mode 2 (single message).
    pub fn return_to_list(&mut self) {
        self.mode = IndexMode::List;
        self.viewing = None;
        self.last_clicked = None;
    }

    /// Build a report for the interface fixture, mirroring
    /// `FleetFinderReport`.
    #[must_use]
    pub fn report(&self, log: &MessageLog, player_is_alliance: bool) -> MessageIndexReport {
        let rows = filtered_messages(log, self.category, player_is_alliance);
        MessageIndexReport {
            mode: self.mode,
            category: self.category,
            rows: rows
                .iter()
                .map(|msg| MessageIndexRow {
                    id: msg.id,
                    text: msg.title.clone().unwrap_or_else(|| msg.text.clone()),
                    rail: msg
                        .rail
                        .expect("filtered_messages only returns rail messages"),
                    unread: msg.unread,
                    tick: msg.tick,
                })
                .collect(),
            selected: self.selected.clone(),
            scroll_offset: self.scroll_offset,
            viewing: self.viewing,
        }
    }
}

// ---------------------------------------------------------------------------
// Report types
// ---------------------------------------------------------------------------

/// One row in a [`MessageIndexReport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageIndexRow {
    pub id: MessageId,
    pub text: String,
    pub rail: MessageRail,
    pub unread: bool,
    pub tick: u64,
}

/// What the open Message Index shows, for the interface fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageIndexReport {
    pub mode: IndexMode,
    pub category: u16,
    pub rows: Vec<MessageIndexRow>,
    pub selected: BTreeSet<MessageId>,
    pub scroll_offset: usize,
    pub viewing: Option<MessageId>,
}

// ---------------------------------------------------------------------------
// Category helpers
// ---------------------------------------------------------------------------

/// The filter mask for a category command (`FUN_004697b0`).
///
/// All Messages (`0x79`) returns 0; the filter uses a special path for it.
#[must_use]
pub const fn category_filter_mask(command: u16) -> u16 {
    match command {
        0x79 => 0,     // All Messages
        0x7a => 0x001, // Popular Support
        0x7b => 0x080, // Fleet
        0x7c => 0x010, // Mission
        0x7d => 0x004, // Resource
        0x7e => 0x008, // Manufacturing
        0x7f => 0x040, // Defense
        0x80 => 0x100, // Conflict
        0x81 => 0x020, // Chat
        0x82 => 0x200, // Advice
        _ => 0,
    }
}

/// The posting option a category command reads and the toggle flips
/// (`FUN_00467f10`: `+0x158`, 0 for All Messages, reads the default).
#[must_use]
pub const fn posting_mask(command: u16) -> Option<u16> {
    match category_filter_mask(command) {
        0 => None,
        mask => Some(mask),
    }
}

/// The object Display (`0x67`) goes to: the first chosen row's in mode 1,
/// the viewed message's in mode 2 (`FUN_00467f10`), when the message has
/// one. `FUN_004697b0` enables Display for the kinds 3 and 5 with a target
/// (`FUN_004ece60`). port: a message with a system stands for those;
/// kind 4's Encyclopedia target is not carried.
#[must_use]
pub fn display_target(rows: &[&GameMessage], state: &MessageIndexState) -> Option<SystemKey> {
    let message = match state.mode {
        IndexMode::List => rows
            .iter()
            .find(|message| state.selected.contains(&message.id)),
        IndexMode::SingleMessage => rows
            .iter()
            .find(|message| state.viewing == Some(message.id)),
    }?;
    message.system
}

/// The viewed message's open question and the mission it names, in mode 2.
#[must_use]
pub fn viewed_question(rows: &[&GameMessage], state: &MessageIndexState) -> Option<(MessageId, u64)> {
    if state.mode != IndexMode::SingleMessage {
        return None;
    }
    let message = rows.iter().find(|message| state.viewing == Some(message.id))?;
    message.continue_question.map(|mission| (message.id, mission))
}

/// Whether the category posts silently, which the toggle's art shows
/// (`FUN_004697b0`: the option's bit 4).
#[must_use]
pub fn posting_silent(log: &MessageLog, command: u16) -> bool {
    log.posting_option(posting_mask(command)) & crate::message_log::SILENT != 0
}

/// The [`MessageRail`] for a category command. All (`0x79`) has no single rail.
#[must_use]
pub const fn category_rail(command: u16) -> Option<MessageRail> {
    match command {
        0x7a => Some(MessageRail::PopularSupport),
        0x7b => Some(MessageRail::Fleet),
        0x7c => Some(MessageRail::Mission),
        0x7d => Some(MessageRail::Resource),
        0x7e => Some(MessageRail::Manufacturing),
        0x7f => Some(MessageRail::Defense),
        0x80 => Some(MessageRail::Conflict),
        0x81 => Some(MessageRail::Chat),
        0x82 => Some(MessageRail::Advice),
        _ => None,
    }
}

/// A row's category icon (`FUN_00468ab0`): the first category bit of the
/// message's mask, in the order the function tests them, picks a STRATEGY
/// bitmap, some per side.
#[must_use]
pub const fn row_icon(rail: MessageRail, faction: CockpitFaction) -> Option<u32> {
    let side = match faction {
        CockpitFaction::Alliance => 0,
        CockpitFaction::Empire => 1,
    };
    Some(match rail {
        MessageRail::PopularSupport => 0x2a9a + side,
        MessageRail::Defense => 0x2ad3,
        MessageRail::Manufacturing => 0x2a9c + side,
        MessageRail::Mission => 0x2a9e + side,
        MessageRail::Chat => 0x2aa4,
        MessageRail::Fleet => 0x2ad4 + side,
        MessageRail::Resource => 0x2ad6,
        MessageRail::Conflict => 0x2ad7,
        MessageRail::Advice => 0x2ad8 + side,
    })
}

/// The list's heading: the active category control's label, TEXTSTRA
/// `0x8010..0x8018` and `0x8032` (`FUN_004665f0` gives each control its
/// label; `FUN_004697b0` draws the chosen one's over the list).
#[must_use]
pub const fn category_heading(command: u16) -> Option<&'static str> {
    Some(match command {
        0x79 => "All Messages",
        0x7a => "Popular Support Messages",
        0x7b => "Fleet Messages",
        0x7c => "Mission Messages",
        0x7d => "Resource Messages",
        0x7e => "Manufacturing Messages",
        0x7f => "Defense Messages",
        0x80 => "Conflict Messages",
        0x81 => "Chat Messages",
        0x82 => "Advice Messages",
        _ => return None,
    })
}

/// Messages visible in the current category for the player's side, newest
/// first.
///
/// `FUN_00468ab0` walks the player's list from its last node
/// (`thunk_FUN_005f5080`) back by predecessors (`FUN_005f5c60`), appending
/// each item, so the newest message heads the list, as the A0 Advice list
/// shows (`ghidra/notes/message-index-rows.md`). `FUN_00468f20`: an item is
/// visible when `(filter & mask) != 0`, or when the category is All
/// (`0x79`) and the mask is not Advice (`0x200`).
#[must_use]
pub fn filtered_messages(
    log: &MessageLog,
    category: u16,
    player_is_alliance: bool,
) -> Vec<&GameMessage> {
    let filter_mask = category_filter_mask(category);
    log.messages()
        .iter()
        .rev()
        .filter(|msg| {
            let Some(rail) = msg.rail else {
                return false;
            };
            if !msg.audience.includes(player_is_alliance) {
                return false;
            }
            let msg_mask = rail.mask();
            // FUN_00468f20: visible = (filter & mask != 0) || (cat == 0x79 && mask != 0x200)
            (filter_mask & msg_mask) != 0
                || (category == 0x79 && msg_mask != MessageRail::Advice.mask())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Drawing: full Message Index (shell + rows)
// ---------------------------------------------------------------------------

/// Draw the Message Index window: the shell, category buttons, and the
/// message list with selection, double-click display, and delete.
///
/// Returns actions the app must handle (speed changes, mark read, delete,
/// message displayed).
#[expect(
    clippy::too_many_arguments,
    reason = "The window needs context, both picture sources, faction, origin, scale, log, state, and side."
)]
pub fn draw_message_index(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    artwork: &mut EncyclopediaState,
    faction: CockpitFaction,
    origin: egui::Pos2,
    scale: f32,
    log: &MessageLog,
    state: &mut MessageIndexState,
    player_is_alliance: bool,
) -> Vec<MessageIndexAction> {
    let mut actions = Vec::new();

    // The window stays above the sector windows, which raise themselves
    // each frame, as the Finders do; its own layers keep their order.
    for id in [
        "original-message-index-shell",
        "message-index-single-picture",
        "message-index-single-title",
        "message-index-list",
        "message-index-single-text",
        "original-message-index-buttons",
    ] {
        ctx.move_to_top(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new(id)));
    }

    // Paint the shell and handle category clicks.
    if let Some(command) = draw_shell(
        ctx,
        cache,
        faction,
        origin,
        scale,
        state.mode,
        Some(state.category),
    ) {
        actions.extend(state.set_category(command));
    }

    if scale <= 0.0 {
        return actions;
    }

    let rows = filtered_messages(log, state.category, player_is_alliance);

    // FUN_00468f20: if no visible row is selected, select the first.
    let any_visible_selected = rows.iter().any(|msg| state.selected.contains(&msg.id));
    if !any_visible_selected && !rows.is_empty() && state.mode == IndexMode::List {
        state.selected.clear();
        state.selected.insert(rows[0].id);
        state.anchor = Some(0);
    }

    // Clamp scroll.
    state.scroll_offset = state.scroll_offset.min(max_scroll(rows.len()));

    match state.mode {
        IndexMode::List => {
            let list_actions = draw_list_rows(ctx, cache, faction, origin, scale, &rows, state);
            actions.extend(list_actions);
        }
        IndexMode::SingleMessage => {
            draw_single_message(ctx, cache, artwork, faction, origin, scale, &rows, state);
        }
    }

    let posting = posting_mask(state.category);
    let silent = posting_silent(log, state.category);
    let target = display_target(&rows, state);
    let question = viewed_question(&rows, state);
    match draw_window_buttons(
        ctx,
        cache,
        faction,
        origin,
        scale,
        state.mode,
        silent,
        rows.is_empty(),
        target.is_some(),
        question.is_some(),
    ) {
        // FUN_00467f10 in mode 2: the checkmark keeps the mission going
        // (vt +0x38 drops the held Abort), the X sends the Abort with force
        // 1 (vt +0x3c -> FUN_0041ce20(order, 1)); both then run 0x65, back
        // to the list.
        Some(answer @ (0x98 | 0x99)) => {
            if let Some((message, mission)) = question {
                actions.push(if answer == 0x98 {
                    MessageIndexAction::ContinueMission { message }
                } else {
                    MessageIndexAction::AbortMission { message, mission }
                });
                actions.extend(state.toggle_display());
            }
        }
        Some(0x66) => actions.push(MessageIndexAction::ToggleSilent(posting)),
        Some(0x67) => {
            // FUN_00467f10: go to the object (FUN_00429440), then close
            // (0x28).
            if let Some(system) = target {
                actions.push(MessageIndexAction::Display(system));
                actions.push(MessageIndexAction::Close);
            }
        }
        Some(0x90) => state.select_all(&rows),
        Some(0x28) => actions.push(MessageIndexAction::Close),
        Some(0x65) => actions.extend(state.toggle_display()),
        Some(0x91) => actions.extend(state.delete_selected(&rows)),
        Some(command @ (0x96 | 0x9a)) => {
            let ids: Vec<MessageId> = rows.iter().map(|message| message.id).collect();
            actions.extend(state.view_adjacent(&ids, command == 0x96));
        }
        _ => {}
    }

    actions
}

/// Draw the list rows in mode 1 and handle interaction.
fn draw_list_rows(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: CockpitFaction,
    origin: egui::Pos2,
    scale: f32,
    rows: &[&GameMessage],
    state: &mut MessageIndexState,
) -> Vec<MessageIndexAction> {
    let mut actions = Vec::new();
    let mut clicked_row: Option<(usize, f64)> = None;
    let mut display_message: Option<MessageId> = None;
    let mut wheel = 0.0_f32;

    let list_id = egui::Id::new("message-index-list");
    egui::Area::new(list_id)
        .fixed_pos(egui::pos2(
            origin.x + LIST_X * scale,
            origin.y + LIST_Y * scale,
        ))
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            let list_size = egui::vec2(LIST_WIDTH * scale, LIST_HEIGHT * scale);
            let (list_rect, _) = ui.allocate_exact_size(list_size, egui::Sense::hover());
            let painter = ui.painter().with_clip_rect(list_rect);

            // Mode 1's title (`Class_0065a1c0_vt44`): TEXTSTRA 0x8019 in
            // font 11 at (15, 20). A0 shows it white, from x 141 with caps
            // from y 76. The chosen category's label heads the list, white
            // in the default font: A0 tooltip/170-index's "Advice Messages"
            // is 101 wide, Arial at 13 ppem.
            use crate::theme::{paint_gdi_text, GdiAlign, GdiVAlign};
            let line = |at: egui::Pos2, width: f32| {
                egui::Rect::from_min_size(at, egui::vec2(width * scale, 30.0 * scale))
            };
            paint_gdi_text(
                ui.painter(),
                line(title_pos(origin, scale), 300.0),
                scale,
                11,
                "Message Index",
                Color32::WHITE,
                (GdiAlign::Left, GdiVAlign::Top),
            );
            if let Some(heading) = category_heading(state.category) {
                paint_gdi_text(
                    ui.painter(),
                    line(heading_pos(origin, scale), 300.0),
                    scale,
                    crate::DEFAULT_FONT,
                    heading,
                    Color32::WHITE,
                    (GdiAlign::Left, GdiVAlign::Top),
                );
            }

            // Each row in its own font (unread or read), grey, the chosen
            // one white (A0 2026-10-29, as the Finders' FUN_00607ea0
            // colours).
            let chosen_bar = match faction {
                CockpitFaction::Alliance => ROW_CHOSEN_ALLIANCE,
                CockpitFaction::Empire => ROW_CHOSEN_EMPIRE,
            };

            // The bar (RCDATA 0x299d, `+0x94`), then the rows scrolled by
            // pixels and clipped to the list; a row's click area ends at
            // the bar.
            #[expect(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "A list's pixels."
            )]
            {
                let frame = egui::Rect::from_min_size(
                    origin,
                    egui::vec2(MESSAGE_INDEX_WIDTH * scale, MESSAGE_INDEX_HEIGHT * scale),
                );
                state.scroll_offset = crate::list_scroll_bar::ui_bar(
                    ui,
                    cache,
                    frame,
                    scale,
                    crate::list_scroll_bar::RESULTS_ART,
                    (LIST_X, LIST_Y, LIST_WIDTH, LIST_HEIGHT),
                    rows.len() as f32 * ROW_HEIGHT,
                    state.scroll_offset as f32,
                    &mut state.scroll_hold,
                    LIST_STEP,
                )
                .max(0.0) as usize;
            }
            for (row_index, msg) in rows.iter().enumerate() {
                let Some(layout) = row_layout(list_rect, scale, row_index, state.scroll_offset)
                else {
                    continue;
                };

                let is_selected = state.selected.contains(&msg.id);

                // FUN_00468ab0: a chosen item shows the side's bar
                // (0x2aa2 / 0x2aa3) at its origin, then the category's icon
                // (15 by 15) at (1, 1) (A0 2026-10-29). hyp: only the bar's
                // top-left 352 by 17 shows; A0 has neither its 4-pixel
                // shadow nor its blue corners.
                let icon = msg.rail.and_then(|rail| row_icon(rail, faction));
                // The bar leaves the icon's cell to the list's own
                // background: A0 messages-day633/index-tab3 shows the stars
                // under the Mission icon (x 1..15, y 1..16 of the row), the
                // bar's red around it. hyp: the item's selected image is
                // the bar with the icon copied in whole (FUN_005fcc30), and
                // the list blits it keyed on black (FUN_005fc710).
                if is_selected {
                    let icon_size = icon
                        .and_then(|id| cache.get(ctx, DllSource::Strategy, id))
                        .map(|texture| texture.size_vec2());
                    if let Some(texture) = cache.get(ctx, DllSource::Strategy, chosen_bar) {
                        let full = chosen_bar_uv(texture.size());
                        for (part, uv) in bar_around_icon(layout.bar, full, icon_size, scale) {
                            painter.image(texture.id(), part, uv, Color32::WHITE);
                        }
                    }
                }
                // The icon at (1, 1) in its own size (15 by 15 or 15 by
                // 16), keyed on black (FUN_005fcc30 into the item image);
                // an unchosen row's image is the icon's size, so it holds
                // all but the icon's last column and row.
                if let Some(icon) = icon {
                    if let Some(texture) = cache.get(ctx, DllSource::Strategy, icon) {
                        let (size, uv) = row_icon_shown(texture.size_vec2(), is_selected);
                        let rect = egui::Rect::from_min_size(layout.icon.min, size * scale);
                        painter.image(texture.id(), rect, uv, Color32::WHITE);
                    }
                }

                // The text at the item's +0x30 (32), its line 2 below the
                // row's top (A0 tooltip/170-index: unread caps from 5,
                // tmAscent 10).
                paint_gdi_text(
                    &painter,
                    line(layout.text, ROW_WIDTH - TEXT_INSET),
                    scale,
                    if msg.unread {
                        ROW_FONT_UNREAD
                    } else {
                        ROW_FONT_READ
                    },
                    msg.title.as_deref().unwrap_or(&msg.text),
                    if is_selected {
                        Color32::WHITE
                    } else {
                        ROW_COLOR
                    },
                    (GdiAlign::Left, GdiVAlign::Top),
                );

                // Click interaction.
                let response = ui.interact(
                    layout.click,
                    ui.id().with(("msg-index-row", row_index)),
                    egui::Sense::click(),
                );
                if response.clicked() {
                    let now = ui.input(|input| input.time);
                    let delay = ctx.options(|options| options.input_options.max_double_click_delay);

                    if state
                        .last_clicked
                        .is_some_and(|(last, at)| last == row_index && now - at <= delay)
                    {
                        // Double click: display message (manual p. 43).
                        display_message = Some(msg.id);
                    } else {
                        clicked_row = Some((row_index, now));
                    }
                }
            }

            // Scroll wheel over the list.
            let pointer = ui.input(|input| input.pointer.hover_pos());
            if pointer.is_some_and(|p| message_index_rect_contains(list_rect, p)) {
                wheel = ui.input(|input| input.raw_scroll_delta.y);
            }
        });

    // Apply click.
    if let Some((row_index, now)) = clicked_row {
        state.last_clicked = Some((row_index, now));
        let modifiers = ctx.input(|i| i.modifiers);
        apply_click(state, row_index, rows, modifiers);
    }

    // Apply double-click: transition to mode 2.
    if let Some(id) = display_message {
        state.mode = IndexMode::SingleMessage;
        state.viewing = Some(id);
        state.last_clicked = None;
        actions.push(MessageIndexAction::MessageDisplayed(id));
    }

    // port: the original has no wheel; a notch moves a row.
    if wheel != 0.0 {
        state.scroll_offset = if wheel > 0.0 {
            state.scroll_offset.saturating_sub(ROW_PIXELS)
        } else {
            (state.scroll_offset + ROW_PIXELS).min(max_scroll(rows.len()))
        };
    }

    actions
}

/// Where one list row draws, in screen points.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RowLayout {
    /// The chosen row's bar, its shown 352 by 17 at the row's origin.
    bar: egui::Rect,
    /// The category icon, 15 by 15 at (1, 1).
    icon: egui::Rect,
    /// The text's cell, at (32, 2).
    text: egui::Pos2,
    /// The click area: the row within the list, short of the scroll bar.
    click: egui::Rect,
}

/// Row `index`'s layout in a list at `list_rect`, scrolled to pixel
/// `scroll`, or `None` when it lies wholly outside the list.
#[expect(clippy::cast_precision_loss, reason = "A list's pixels.")]
fn row_layout(list_rect: egui::Rect, scale: f32, index: usize, scroll: usize) -> Option<RowLayout> {
    let top = index as f32 * ROW_HEIGHT - scroll as f32;
    if top + ROW_HEIGHT <= 0.0 || top >= LIST_HEIGHT {
        return None;
    }
    let origin = list_rect.min + egui::vec2(0.0, top * scale);
    let row = egui::Rect::from_min_size(origin, egui::vec2(ROW_WIDTH * scale, ROW_HEIGHT * scale));
    let short_of_bar = egui::Rect::from_min_max(
        list_rect.min,
        egui::pos2(list_rect.max.x - BAR_WIDTH * scale, list_rect.max.y),
    );
    Some(RowLayout {
        bar: egui::Rect::from_min_size(
            origin,
            egui::vec2(ROW_CHOSEN_SIZE.0 * scale, ROW_CHOSEN_SIZE.1 * scale),
        ),
        icon: egui::Rect::from_min_size(
            origin + egui::vec2(scale, scale),
            egui::vec2(ROW_ICON_SIZE * scale, ROW_ICON_SIZE * scale),
        ),
        text: origin + egui::vec2(TEXT_INSET * scale, ROW_TEXT_Y * scale),
        click: row.intersect(short_of_bar),
    })
}

/// The part of a row's icon (`size`, unscaled) shown at the row's (1, 1)
/// and its texture coordinates. `FUN_00468ab0` blits the icon at (1, 1)
/// into an item image of the icon's own size, so an unchosen row loses
/// the icon's last column and row (A0 messages-day633/index-tab7: "Assault
/// on Umgul" lacks 0x2ad7's column 14, which the chosen row shows).
fn row_icon_shown(size: egui::Vec2, chosen: bool) -> (egui::Vec2, egui::Rect) {
    if chosen || size.x < 1.0 || size.y < 1.0 {
        return (
            size,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        );
    }
    let shown = size - egui::vec2(1.0, 1.0);
    (
        shown,
        egui::Rect::from_min_max(
            egui::Pos2::ZERO,
            egui::pos2(shown.x / size.x, shown.y / size.y),
        ),
    )
}

/// The chosen bar's pieces around the icon's cell at (1, 1) of `size`
/// (the icon's own, unscaled): the bar's column 0, its row 0 above the
/// icon, the strip right of the icon and the rows below it. With no icon,
/// the whole bar.
fn bar_around_icon(
    bar: egui::Rect,
    uv: egui::Rect,
    size: Option<egui::Vec2>,
    scale: f32,
) -> Vec<(egui::Rect, egui::Rect)> {
    let Some(size) = size else {
        return vec![(bar, uv)];
    };
    // A logical rectangle of the bar, on the screen and in the texture.
    let piece = |x0: f32, y0: f32, x1: f32, y1: f32| {
        let (width, height) = (bar.width() / scale, bar.height() / scale);
        let (x1, y1) = (x1.min(width), y1.min(height));
        (
            egui::Rect::from_min_max(
                bar.min + egui::vec2(x0, y0) * scale,
                bar.min + egui::vec2(x1, y1) * scale,
            ),
            egui::Rect::from_min_max(
                uv.min + egui::vec2(x0 / width * uv.width(), y0 / height * uv.height()),
                uv.min + egui::vec2(x1 / width * uv.width(), y1 / height * uv.height()),
            ),
        )
    };
    let (right, bottom) = (1.0 + size.x, 1.0 + size.y);
    vec![
        piece(0.0, 0.0, 1.0, bottom),
        piece(1.0, 0.0, right, 1.0),
        piece(right, 0.0, f32::MAX, f32::MAX),
        piece(0.0, bottom, right, f32::MAX),
    ]
    .into_iter()
    .filter(|(rect, _)| rect.width() > 0.0 && rect.height() > 0.0)
    .collect()
}

/// The part of the chosen bar's texture that shows: its top-left
/// [`ROW_CHOSEN_SIZE`].
#[expect(clippy::cast_precision_loss, reason = "A bitmap's pixels.")]
fn chosen_bar_uv([width, height]: [usize; 2]) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::Pos2::ZERO,
        egui::pos2(
            ROW_CHOSEN_SIZE.0 / width as f32,
            ROW_CHOSEN_SIZE.1 / height as f32,
        ),
    )
}

/// Mode 1's title cell on the screen (`FUN_00601b30(0xf, 0x14)`).
fn title_pos(origin: egui::Pos2, scale: f32) -> egui::Pos2 {
    origin + egui::vec2(15.0 * scale, 20.0 * scale)
}

/// The list heading's cell on the screen.
fn heading_pos(origin: egui::Pos2, scale: f32) -> egui::Pos2 {
    origin + egui::vec2(HEADING_AT.0 * scale, HEADING_AT.1 * scale)
}

/// Apply a click with modifiers to the selection.
///
/// Manual pp. 78-80: click selects one, Ctrl toggles, Shift extends range.
fn apply_click(
    state: &mut MessageIndexState,
    row_index: usize,
    rows: &[&GameMessage],
    modifiers: egui::Modifiers,
) {
    let Some(msg) = rows.get(row_index) else {
        return;
    };

    if modifiers.ctrl || modifiers.command {
        // Ctrl-click: toggle one message (manual p. 43).
        if !state.selected.remove(&msg.id) {
            state.selected.insert(msg.id);
        }
        state.anchor = Some(row_index);
    } else if modifiers.shift {
        // Shift-click: contiguous range from anchor (manual p. 43), added
        // to the rows already chosen (A0 message-delete/sel2: row 0 stays
        // chosen beside the range 2..4; sh2: a shorter range keeps 3..4).
        if let Some(anchor) = state.anchor {
            let start = anchor.min(row_index);
            let end = anchor.max(row_index);
            for i in start..=end {
                if let Some(m) = rows.get(i) {
                    state.selected.insert(m.id);
                }
            }
            // anchor stays
        } else {
            // No anchor: treat as normal click.
            state.selected.clear();
            state.selected.insert(msg.id);
            state.anchor = Some(row_index);
        }
    } else {
        // Normal click: select one, clear the rest.
        state.selected.clear();
        state.selected.insert(msg.id);
        state.anchor = Some(row_index);
    }
}

/// Draw the single-message text in mode 2.
#[expect(
    clippy::too_many_arguments,
    reason = "Mode 2 paints the picture, the item's image and the title together."
)]
fn draw_single_message(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    artwork: &mut EncyclopediaState,
    faction: CockpitFaction,
    origin: egui::Pos2,
    scale: f32,
    rows: &[&GameMessage],
    state: &mut MessageIndexState,
) {
    let Some(id) = state.viewing else {
        return;
    };
    let Some(msg) = rows.iter().find(|m| m.id == id) else {
        return;
    };
    state.follow_text(id);

    if let Some(picture) = msg.picture {
        draw_message_picture(ctx, cache, artwork, origin, scale, picture);
    }
    let icon = msg.rail.and_then(|rail| row_icon(rail, faction));
    if let Some(title) = &msg.title {
        draw_message_title(ctx, cache, origin, scale, title, icon);
    }

    let text_id = egui::Id::new("message-index-single-text");
    egui::Area::new(text_id)
        .fixed_pos(egui::pos2(
            origin.x + TEXT_AREA_X * scale,
            origin.y + TEXT_AREA_Y * scale,
        ))
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            // FUN_00469de0: 0x140 for a kind-5 message, beside its answers.
            let width = if msg.continue_question.is_some() {
                QUESTION_TEXT_AREA_WIDTH
            } else {
                TEXT_AREA_WIDTH
            };
            let area_size = egui::vec2(width * scale, TEXT_AREA_HEIGHT * scale);
            let (area_rect, _) = ui.allocate_exact_size(area_size, egui::Sense::hover());

            // The text control writes in font 4 (FUN_0041ecf0:
            // FUN_006002b0(4); A0 messages-day633/capture-palpatine:
            // "Emperor" is 48 wide as in the title, its ink from (18, 237))
            // and wraps at its width (launchbox frame
            // "event-imperial-future-jedi" shows three wrapped lines).
            let font = crate::DEFAULT_FONT;
            let (ascent, descent) = crate::theme::original_font_metrics(font);
            let pitch = ascent + descent;
            let measure = ui.painter().clone();
            let (wrap, content, barred) = text_wrap(width, |wrap| {
                #[expect(clippy::cast_precision_loss, reason = "A handful of lines.")]
                let lines = crate::theme::gdi_word_break(&msg.text, wrap, |line| {
                    crate::theme::gdi_text_width(&measure, font, line, scale)
                })
                .len() as f32;
                lines * pitch
            });
            let first = if barred { state.text_scroll.0 } else { 0.0 };
            let painter = ui.painter().with_clip_rect(area_rect);
            crate::theme::paint_gdi_text_block(
                &painter,
                on_screen(
                    area_rect.min,
                    scale,
                    egui::Rect::from_min_size(
                        egui::pos2(0.0, -first),
                        egui::vec2(wrap, TEXT_AREA_HEIGHT + first),
                    ),
                ),
                scale,
                font,
                &msg.text,
                Color32::WHITE,
                crate::theme::GdiAlign::Left,
            );
            // FUN_0041efe0 (create): FUN_0060a490(.., 7, 0x299a, ..), the
            // Status window's 12-wide bar at the control's right edge (A0
            // agent-advice/02-advice-message: a long advice's bar at
            // x 525..537 of the Empire's window at 125).
            if barred {
                state.text_scroll.0 = crate::list_scroll_bar::ui_bar(
                    ui,
                    cache,
                    egui::Rect::from_min_size(origin, egui::Vec2::ZERO),
                    scale,
                    crate::status_window::BAR_ART,
                    (TEXT_AREA_X, TEXT_AREA_Y, width, TEXT_AREA_HEIGHT),
                    content,
                    state.text_scroll.0,
                    &mut state.text_scroll_hold,
                    pitch,
                );
            }
        });
}

/// The item's image in mode 2's title (`Class_0065a1c0_vt44`), relative to
/// the window, for an icon of `size`: the image is the icon's size, black,
/// with the icon at (1, 1) cut by its last column and row (`FUN_00468ab0`).
/// `FUN_005fc140` is called with no destination, so `SetDIBitsToDevice`
/// puts it at the image's own width and height (A0 messages-day633/
/// capture-palpatine: 0x2a9e, 15 by 16, at (15, 16); message-rail/
/// fleet-arrives-averam: 0x2ad4, 15 by 15, at (15, 15)). Returns the image
/// rectangle, the icon's rectangle and its texture coordinates.
fn title_icon_layout(size: egui::Vec2) -> (egui::Rect, egui::Rect, egui::Rect) {
    let image = egui::Rect::from_min_size(egui::pos2(size.x, size.y), size);
    let (shown, uv) = row_icon_shown(size, false);
    let icon = egui::Rect::from_min_size(image.min + egui::vec2(1.0, 1.0), shown);
    (image, icon, uv)
}

/// The text control's wrap width, its lines' height and whether its bar
/// shows, for a control `width` wide whose text takes `height_at(wrap)`.
/// hyp: the control wraps at its full width while its text fits the
/// 80-pixel height, else beside its 12-wide bar (A0 agent-advice/
/// 02-advice-message wraps a long advice short of the bar).
pub(crate) fn text_wrap(width: f32, height_at: impl Fn(f32) -> f32) -> (f32, f32, bool) {
    let full = height_at(width);
    if full <= TEXT_AREA_HEIGHT {
        return (width, full, false);
    }
    let wrap = width - f32::from(crate::status_window::BAR_ART.width);
    (wrap, height_at(wrap), true)
}

/// Paint a message's title in white across the title strip:
/// `Class_0065a1c0_vt44` writes it in font 4 at (40, 13), after the
/// chosen item's image.
fn draw_message_title(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    origin: egui::Pos2,
    scale: f32,
    title: &str,
    icon: Option<u32>,
) {
    let texture = icon.and_then(|id| cache.get(ctx, DllSource::Strategy, id));
    egui::Area::new(egui::Id::new("message-index-single-title"))
        .fixed_pos(origin)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            if let Some(texture) = texture {
                let (image, icon, uv) = title_icon_layout(texture.size_vec2());
                ui.painter()
                    .rect_filled(on_screen(origin, scale, image), 0.0, Color32::BLACK);
                ui.painter().image(
                    texture.id(),
                    on_screen(origin, scale, icon),
                    uv,
                    Color32::WHITE,
                );
            }
            crate::theme::paint_gdi_text(
                ui.painter(),
                on_screen(origin, scale, TITLE_TEXT_CELL),
                scale,
                4,
                title,
                Color32::WHITE,
                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
            );
        });
}

/// Mode 2's title cell (`Class_0065a1c0_vt44`: `FUN_00601b30(0x28, 0xd)`),
/// as wide as the title strip after it.
const TITLE_TEXT_CELL: egui::Rect = egui::Rect {
    min: egui::pos2(MESSAGE_TITLE_X, 13.0),
    max: egui::pos2(412.0, 43.0),
};

/// A rectangle in the window's own pixels on the screen.
fn on_screen(origin: egui::Pos2, scale: f32, rect: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(origin + rect.min.to_vec2() * scale, rect.size() * scale)
}

/// Where a message's picture sits in the window (`FUN_0046a320`:
/// `FUN_006073d0(0xc, 0x21, ..)`), above the text area.
const PICTURE_ORIGIN: (f32, f32) = (12.0, 33.0);

/// The rectangles of a message's background and its foreground, centred
/// on it (`FUN_0046a320`: `FUN_005fd0f0` at the halved size differences),
/// in window pixels.
fn message_picture_rects(
    background: egui::Vec2,
    foreground: Option<egui::Vec2>,
) -> (egui::Rect, Option<egui::Rect>) {
    let origin = egui::pos2(PICTURE_ORIGIN.0, PICTURE_ORIGIN.1);
    let back = egui::Rect::from_min_size(origin, background);
    let front = foreground.map(|size| {
        let offset = ((background - size) / 2.0).floor();
        egui::Rect::from_min_size(origin + offset, size)
    });
    (back, front)
}

/// Paint a message's picture: its background, then its foreground keyed
/// over it (the blue matte is transparent, `bmp_cache`).
///
/// `FUN_0046a280`: the background is a STRATEGY bitmap, or the object's
/// Encyclopedia artwork where STRATEGY has none (`FUN_0045f970`).
fn draw_message_picture(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    artwork: &mut EncyclopediaState,
    origin: egui::Pos2,
    scale: f32,
    picture: MessagePicture,
) {
    let Some(background) = cache
        .get(ctx, DllSource::Strategy, picture.background)
        .cloned()
        .or_else(|| {
            crate::message_pictures::artwork_edata(picture.background)
                .and_then(|file| artwork.artwork_texture(ctx, file))
        })
    else {
        return;
    };
    let foreground = picture
        .foreground
        .and_then(|id| cache.get(ctx, DllSource::Strategy, id).cloned());
    let (back, front) = message_picture_rects(
        background.size_vec2(),
        foreground.as_ref().map(|texture| texture.size_vec2()),
    );
    let screen = |rect: egui::Rect| {
        egui::Rect::from_min_size(origin + rect.min.to_vec2() * scale, rect.size() * scale)
    };
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    egui::Area::new(egui::Id::new("message-index-single-picture"))
        .fixed_pos(origin)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            let painter = ui.painter();
            painter.image(background.id(), screen(back), uv, Color32::WHITE);
            if let (Some(texture), Some(rect)) = (foreground, front) {
                painter.image(texture.id(), screen(rect), uv, Color32::WHITE);
            }
        });
}

// ---------------------------------------------------------------------------
// Fixture adapter (preserved)
// ---------------------------------------------------------------------------

/// Fixed-position test adapter for deterministic browser inspection.
#[cfg(feature = "interface-test-fixtures")]
pub fn draw_message_index_fixture(
    ctx: &egui::Context,
    cache: &mut BmpCache,
    faction: CockpitFaction,
) {
    let _ = draw_message_index_shell(ctx, cache, faction, egui::pos2(85.0, 55.0), 1.0);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {

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
            window_button_specs(faction, IndexMode::List, silent, false, false)
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
                false,
            )
            .iter()
            .any(|button| button.command_id == 0x65)
        };
        assert!(clickable(false));
        assert!(!clickable(true));
        assert!(
            window_button_specs(
                CockpitFaction::Empire,
                IndexMode::SingleMessage,
                false,
                true,
                false
            )
            .iter()
            .any(|button| button.command_id == 0x65 && button.normal_resource == 0x288a)
        );
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
        assert_eq!(display_target(&rows, &state), Some(system));
        state.selected.clear();
        state.selected.insert(rows[0].id);
        state.mode = IndexMode::SingleMessage;
        state.viewing = Some(rows[1].id);
        assert_eq!(display_target(&rows, &state), None);
        state.viewing = Some(rows[0].id);
        assert_eq!(display_target(&rows, &state), Some(system));

        // Enabled art and the click (Alliance: 0x67 at (0x1a7, 0xc9)).
        let display = window_button_specs(
            CockpitFaction::Alliance,
            IndexMode::List,
            false,
            false,
            true,
        )
        .into_iter()
        .find(|button| button.command_id == 0x67)
        .map(|button| (button.y, button.normal_resource, button.pressed_resource));
        assert_eq!(display, Some((0xc9, 0x2916, 0x2917)));
        let empire =
            window_button_specs(CockpitFaction::Empire, IndexMode::List, false, false, true)
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
                .contains(&MessageIndexAction::Display(system))
        );
        assert!(result.actions.contains(&MessageIndexAction::Close));
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
            false,
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
            false,
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
                (10_335, 0.0, 0.0),
                (10_820, 412.0, 0.0),
                (10_822, 12.0, 13.0)
            ]
        );
        assert_eq!(
            ids(IndexMode::SingleMessage),
            [
                (10_335, 0.0, 0.0),
                (10_820, 412.0, 0.0),
                (10_933, 12.0, 14.0),
                (10_932, 12.0, 232.0)
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
        // FUN_0046a200 enables each scroll button only with a neighbour;
        // forward is the down arrow (0x96), back the up arrow (0x9a).
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
}
