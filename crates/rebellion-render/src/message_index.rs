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
    display_kind: Option<DisplayKind>,
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
    if let Some(kind) = display_kind {
        // FUN_004697b0: Display for a message of kind 3 or 5 with a target
        // (0x2916 / 0x2918, label 0x8030); kind 4's Encyclopedia shows
        // 0x2d20 / 0x2d47 (label 0x8031).
        let (y, art) = match (faction, kind) {
            (CockpitFaction::Alliance, DisplayKind::Object) => (0xc9, 0x2916),
            (CockpitFaction::Empire, DisplayKind::Object) => (0xcf, 0x2918),
            (CockpitFaction::Alliance, DisplayKind::Encyclopedia) => (0xc9, 0x2d20),
            (CockpitFaction::Empire, DisplayKind::Encyclopedia) => (0xcf, 0x2d47),
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

/// `FUN_0046a200` (mode 2): a scroll button with no neighbour on its side
/// is disabled (button vt `+0x10`, `FUN_006028a0`: state bit 2 and
/// `EnableWindow(.., 0)`) and shows its `FUN_00603150(2, ..)` art: `0x96`
/// shows `0x2aa9` (10921) on the oldest message, `0x9a` shows `0x2ac6`
/// (10950) on the newest. A disabled button keeps that art while pressed.
#[must_use]
pub fn disable_scroll_buttons(
    mut buttons: Vec<WindowButtonSpec>,
    older: bool,
    newer: bool,
) -> Vec<WindowButtonSpec> {
    for button in &mut buttons {
        let disabled = match button.command_id {
            0x96 => (!older).then_some(10_921),
            0x9a => (!newer).then_some(10_950),
            _ => None,
        };
        if let Some(art) = disabled {
            button.normal_resource = art;
            button.pressed_resource = art;
        }
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
    display: Option<DisplayKind>,
    question: bool,
    (older, newer): (bool, bool),
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
                disabled_rail_controls(faction, mode, list_empty, display.is_some())
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
            for button in disable_scroll_buttons(
                window_button_specs(faction, mode, silent, list_empty, display),
                older,
                newer,
            )
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
                let string = if button.command_id == 0x67 {
                    Some(display_button_string(display))
                } else {
                    button_tooltip_string(button.command_id, mode, silent)
                };
                if let Some(string) = string {
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
    // The panes lie under the frame: the base is keyed on its blue (252),
    // and its corner arcs inside the panes' rectangles show in A0. Every
    // non-blue base pixel over 10822 is the base's own colour in A0
    // empire-game3/index-joins-chosen (86 of 86, ignoring one where the two
    // agree) and message-index/category-1-day228 (498 of 498), and over
    // 0x2ab5 / 0x2ab4 in empire-game3/joins-enemy (27 and 45).
    let mut layers = match mode {
        IndexMode::List => vec![(
            INDEX_CONTENT,
            (
                INDEX_CONTENT_X,
                INDEX_CONTENT_Y,
                INDEX_CONTENT_WIDTH,
                INDEX_CONTENT_HEIGHT,
            ),
        )],
        IndexMode::SingleMessage => vec![MESSAGE_TITLE_STRIP, MESSAGE_TEXT_PANE],
    };
    layers.extend([
        (base, (0.0, 0.0, MESSAGE_INDEX_WIDTH, MESSAGE_INDEX_HEIGHT)),
        (rail, (412.0, 0.0, 58.0, 330.0)),
    ]);
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
    /// message's object (`FUN_00429440`): a Mission Report's mission while
    /// it runs (A0 mission-reports/report-display-missions-window-day235:
    /// the Missions window "Mission at Chandrila"), else its system, whose
    /// sector window opens with the system chosen.
    Display {
        system: SystemKey,
        mission: Option<u64>,
    },
    /// Display on a kind-4 message (`FUN_00467f10` -> `FUN_0041d6b0`): the
    /// app opens the Encyclopedia at the object.
    Encyclopedia(u32),
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
    /// Display's face as the scroll buttons left it, for a kind-8 message
    /// ([`Self::view_adjacent_keeping_display`]); `None` once the message
    /// was shown from the list (`FUN_004697b0` set it).
    carried_display: Option<Option<DisplayKind>>,
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
            carried_display: None,
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
            self.carried_display = None;
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
                self.carried_display = None;
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
    /// neighbour). `FUN_00467f10`: `0x96` takes the row's next link
    /// (`FUN_004ad730`, one row down) and `0x9a` its previous link
    /// (`FUN_004ad750`, one row up). The rows run newest first
    /// (`FUN_00468ab0`), so `forward` (`0x96`) opens the older message and
    /// `0x9a` the newer one.
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

    /// [`Self::view_adjacent`] for the scroll buttons (`FUN_00467f10`
    /// cases `0x96` / `0x9a`): they show the neighbour through
    /// `FUN_00469de0` alone, which sets Display (`0x67`) for kinds 3, 4
    /// and 5 only, so a kind-8 neighbour keeps Display's face as it was
    /// (A0 2026-11-17-edited-saves-6 d-11). [`display_kind`] reads it back.
    pub fn view_adjacent_keeping_display(
        &mut self,
        rows: &[&GameMessage],
        forward: bool,
    ) -> Option<MessageIndexAction> {
        let face = display_kind(rows, self);
        let ids: Vec<MessageId> = rows.iter().map(|message| message.id).collect();
        let action = self.view_adjacent(&ids, forward)?;
        self.carried_display = Some(face);
        Some(action)
    }

    /// Whether the open message has an older (`0x96`) and a newer (`0x9a`)
    /// neighbour in the category's rows (`FUN_0046a200`).
    #[must_use]
    pub fn scroll_neighbours(&self, rows: &[MessageId]) -> (bool, bool) {
        rows.iter()
            .position(|id| Some(*id) == self.viewing)
            .map_or((false, false), |current| {
                (current + 1 < rows.len(), current > 0)
            })
    }

    /// Return to mode 1 (list) from mode 2 (single message).
    pub fn return_to_list(&mut self) {
        self.mode = IndexMode::List;
        self.viewing = None;
        self.carried_display = None;
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
pub fn display_target(
    rows: &[&GameMessage],
    state: &MessageIndexState,
) -> Option<(SystemKey, Option<u64>)> {
    let message = display_message(rows, state)?;
    Some((message.system?, message.mission))
}

/// Whether Display (0x67) is lit: the message has a target
/// (`FUN_004697b0`, `FUN_004ece60`), placed or not, or is of kind 4, an
/// Encyclopedia entry. An unplaced one's press does nothing
/// ([`GameMessage::unplaced_target`]).
#[must_use]
pub fn display_lit(rows: &[&GameMessage], state: &MessageIndexState) -> bool {
    display_kind(rows, state).is_some()
}

/// What Display shows for the chosen or viewed message: kind 4's
/// Encyclopedia (`FUN_004697b0`, `FUN_00469de0`), or the object of a kind
/// 3 message, or of a kind 5 one with a target. Kind 8 leaves it grey.
/// port: mission and assault reports (kinds 3/5 by `+0x84`, 3/7 by
/// `+0x6c`) keep needing a system.
#[must_use]
pub fn display_kind(rows: &[&GameMessage], state: &MessageIndexState) -> Option<DisplayKind> {
    let message = display_message(rows, state)?;
    if let (true, IndexMode::SingleMessage, Some(face)) =
        (message.plain, state.mode, state.carried_display)
    {
        return face;
    }
    let report = message.mission.is_some()
        || message.continue_question.is_some()
        || message.assault_report.is_some();
    if message.encyclopedia.is_some() {
        Some(DisplayKind::Encyclopedia)
    } else if message.plain || message.rail == Some(MessageRail::Advice) {
        None
    } else if message.system.is_some() || message.unplaced_target || !report {
        Some(DisplayKind::Object)
    } else {
        None
    }
}

/// Display's two faces (`FUN_004697b0`): go to the message's object
/// (kinds 3 and 5), or open the Encyclopedia (kind 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayKind {
    Object,
    Encyclopedia,
}

/// Display's tooltip: 0x8030 "Display", or 0x8031 "Encylopedia" (sic) on
/// a kind-4 message (`FUN_004697b0`, `FUN_00469de0`).
#[must_use]
pub const fn display_button_string(display: Option<DisplayKind>) -> u16 {
    match display {
        Some(DisplayKind::Encyclopedia) => 0x8031,
        _ => 0x8030,
    }
}

/// The message Display acts on: the first chosen row's in mode 1, the
/// viewed message's in mode 2 (`FUN_00467f10`).
fn display_message<'a>(
    rows: &[&'a GameMessage],
    state: &MessageIndexState,
) -> Option<&'a GameMessage> {
    match state.mode {
        IndexMode::List => rows
            .iter()
            .find(|message| state.selected.contains(&message.id))
            .copied(),
        IndexMode::SingleMessage => rows
            .iter()
            .find(|message| state.viewing == Some(message.id))
            .copied(),
    }
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
        // A mask of 0 matches none of the tests: no icon.
        MessageRail::Unfiled => return None,
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
    let kind = display_kind(&rows, state);
    let encyclopedia = display_message(&rows, state).and_then(|message| message.encyclopedia);
    let question = viewed_question(&rows, state);
    let ids: Vec<MessageId> = rows.iter().map(|message| message.id).collect();
    match draw_window_buttons(
        ctx,
        cache,
        faction,
        origin,
        scale,
        state.mode,
        silent,
        rows.is_empty(),
        kind,
        question.is_some(),
        state.scroll_neighbours(&ids),
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
            // FUN_00467f10: kind 4 opens the Encyclopedia on +0x60
            // (FUN_0041d6b0); kinds 3 and 5 go to the object
            // (FUN_00429440); then close (0x28).
            if let Some(object) = encyclopedia {
                actions.push(MessageIndexAction::Encyclopedia(object));
                actions.push(MessageIndexAction::Close);
            } else if let Some((system, mission)) = target {
                actions.push(MessageIndexAction::Display { system, mission });
                actions.push(MessageIndexAction::Close);
            }
        }
        Some(0x90) => state.select_all(&rows),
        Some(0x28) => actions.push(MessageIndexAction::Close),
        Some(0x65) => actions.extend(state.toggle_display()),
        Some(0x91) => actions.extend(state.delete_selected(&rows)),
        Some(command @ (0x96 | 0x9a)) => {
            actions.extend(state.view_adjacent_keeping_display(&rows, command == 0x96));
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
        state.carried_display = None;
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
            let pitch = crate::theme::line_pitch(font);
            let measure = ui.painter().clone();
            let (wrap, content, barred) = text_wrap_lines(width, pitch, |wrap| {
                crate::theme::gdi_word_break(&msg.text, wrap, |line| {
                    crate::theme::gdi_text_width(&measure, font, line, scale)
                })
                .len()
            });
            let first = if barred { state.text_scroll.0 } else { 0.0 };
            let painter = ui.painter().with_clip_rect(area_rect);
            crate::theme::paint_gdi_text_block(
                &painter,
                scrolled_text_rect(area_rect.min, scale, wrap, first),
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

/// [`text_wrap`] for a text of `lines_at(wrap)` lines, `pitch` apart.
pub(crate) fn text_wrap_lines(
    width: f32,
    pitch: f32,
    lines_at: impl Fn(f32) -> usize,
) -> (f32, f32, bool) {
    #[expect(clippy::cast_precision_loss, reason = "A handful of lines.")]
    text_wrap(width, |wrap| lines_at(wrap) as f32 * pitch)
}

/// Where a text control's lines start on the screen: `wrap` wide from its
/// top-left `origin`, scrolled up by `first` pixels, as tall as the
/// control and the scrolled part.
pub(crate) fn scrolled_text_rect(
    origin: egui::Pos2,
    scale: f32,
    wrap: f32,
    first: f32,
) -> egui::Rect {
    on_screen(
        origin,
        scale,
        egui::Rect::from_min_size(
            egui::pos2(0.0, -first),
            egui::vec2(wrap, TEXT_AREA_HEIGHT + first),
        ),
    )
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
pub(crate) fn on_screen(origin: egui::Pos2, scale: f32, rect: egui::Rect) -> egui::Rect {
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
#[path = "message_index_tests.rs"]
mod tests;
