//! The original Status window (window type `0x1a`, `FUN_00442d70`; create
//! `FUN_00443130`; controls `FUN_00443020`, `FUN_004443a0`). The object
//! menu's Status (`0x103`) opens it (`FUN_0042a440`). Recovery notes:
//! `ghidra/notes/status-window.md`.
//!
//! A 379 by 272 window: a two-column list of labels and values on the left,
//! the object's portrait and name on the right, and the Encyclopedia and
//! Close buttons under them. Each object family has its own filler
//! (`status_rows.rs`); characters are `FUN_004486f0` (manual p. 101, Fig.
//! 3.46).

use egui_macroquad::egui;
use rebellion_core::ids::CharacterKey;
use rebellion_core::missions::{MemberTransit, MissionMember};
use rebellion_core::world::{GameWorld, Skill};

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::fleet_window::paint_native;
use crate::mission_dialog::{button, galaxy_centered_rect, paint};
use crate::status_rows::{status_view, IconRowLayout, StatusObject, StatusSources};
use crate::system_window::{fleet_label, logical_rect, rect_contains};

pub const STATUS_WINDOW_WIDTH: f32 = 379.0;
pub const STATUS_WINDOW_HEIGHT: f32 = 272.0;

/// The background (STRATEGY `0x2d22..`), by the object's side bits, for a
/// side-1 player; a player of another side takes the bitmap three on.
const BACKGROUND: [u32; 3] = [11_554, 11_555, 11_556];
/// `0x66`, Encyclopedia, and `0x65`, Close (`FUN_00443020`).
const ENCYCLOPEDIA: (u32, u32) = (11_552, 11_553);
const CLOSE: (u32, u32) = (10_370, 10_371);
const ENCYCLOPEDIA_RECT: (f32, f32, f32, f32) = (258.0, 218.0, 32.0, 31.0);
/// The Encyclopedia and Close buttons' tooltips (`FUN_00443020` via
/// `FUN_00606c90`): TEXTSTRA 0x1505 "Encyclopedia", 0x1954 "Close".
pub(crate) const ENCYCLOPEDIA_TOOLTIP: u16 = 0x1505;
pub(crate) const CLOSE_TOOLTIP: u16 = 0x1954;
const CLOSE_RECT: (f32, f32, f32, f32) = (324.0, 218.0, 32.0, 31.0);

/// `+0x130`: (15, 18), 211 by 18, font 5, format `0x21`.
const TITLE: (f32, f32, f32, f32) = (15.0, 18.0, 211.0, 18.0);
/// `+0x198`: (242, 137), 130 by 44, font 5, format `0x11`.
const NAME: (f32, f32, f32, f32) = (242.0, 137.0, 130.0, 44.0);
/// `+0x118`: the list, (18, 47), 208 by 204.
const LIST: (f32, f32, f32, f32) = (18.0, 47.0, 208.0, 204.0);
/// `FUN_00449e00`: labels at x 0 and values at x 103, each 103 wide.
const COLUMN: f32 = 103.0;
/// The list's font (`FUN_00449e00`): font 10, which GDI draws at 11 ppem on
/// 14-pixel lines (theme::original_font_metrics). A0 (2026-10-29, Veers):
/// rows 14 apart, caps 3 below a row's top; A0 status/06-fleet: "Not
/// Assigned" 63 by 10 and "Hyperdrive Rating:" 87 by 10, Arial at 11 ppem.
const LIST_FONT: u16 = 10;
/// The picture is centred on (307, 64) (`FUN_00443130`).
const PICTURE_CENTRE: (i32, i32) = (307, 64);
/// The Objectives' list: one column of cells 35 high (`FUN_006073d0(0x67,
/// 0xd0, 0x23, ..)`; the scroll bar leaves too little width for a second
/// 103-wide cell, `FUN_00609ae0`), each icon composed into a canvas of the
/// first icon's size, 37 by 37 (STRATEGY 10234), and the text at x 40
/// (`+0xe8 = 0x28`), centred on the row's height (`+0xe4 = 0x24`).
const ICON_ROW_HEIGHT: f32 = 35.0;
const ICON_CELL: i32 = 37;
const ICON_TEXT_X: f32 = 40.0;
/// hyp: the list blits its items one pixel in from its left edge (A0
/// 2026-10-29: the 25-wide portraits start at list x 7, the text at 42).
const ICON_ROW_INSET: f32 = 1.0;
/// The list's scroll bar (`FUN_00443130`: `FUN_0060a490(.., list, 7,
/// 0x299a, ..)`): STRATEGY RCDATA 10650.
const BAR_ART: crate::list_scroll_bar::BarArt = crate::list_scroll_bar::BarArt {
    width: 12,
    track: 10_654,
    up: [10_658, 10_660],
    down: [10_662, 10_664],
    top: 10_666,
    middle: 10_668,
    bottom: 10_669,
};
/// The list's arrow step, `FUN_00607ea0`'s argument 10, as the Battle
/// Results' list takes it.
const LIST_STEP: f32 = 10.0;
/// The Encyclopedia button's disabled art (`FUN_00443020`).
const ENCYCLOPEDIA_DISABLED: u32 = 11_612;

/// One list row: `FUN_0044b1d0` appends the label to one list and the value
/// to the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusRow {
    pub label: String,
    pub value: String,
}

fn row(label: &str, value: impl Into<String>) -> StatusRow {
    StatusRow {
        label: label.to_owned(),
        value: value.into(),
    }
}

/// TEXTSTRA 34646 "Yes" or 34645 "No".
const fn yes_no(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        "No"
    }
}

/// Force Ranking by the Force value `+0x8c` (`FUN_004486f0`).
#[must_use]
pub const fn force_ranking(value: u32) -> &'static str {
    match value {
        0..=9 => "None",
        10..=19 => "Novice",
        20..=79 => "Trainee",
        80..=99 => "Jedi Student",
        100..=119 => "Jedi Knight",
        _ => "Jedi Master",
    }
}

/// The Character Status rows (`FUN_004486f0`). `en_route` is the mission
/// transits (`MissionState::en_route`).
///
/// port: Commanding (`+0xa8`) is always "None": the port keeps no command
/// field. A character is en route while its
/// mission transit lasts; Attached then names the transit's destination,
/// the container of an object in hyperspace (`FUN_00556390`).
#[must_use]
pub fn character_rows(
    world: &GameWorld,
    en_route: &[MemberTransit],
    key: CharacterKey,
) -> Option<Vec<StatusRow>> {
    let character = world.characters.get(key)?;
    let transit = en_route
        .iter()
        .find(|transit| transit.member == MissionMember::Character(key));
    let attached = transit
        .and_then(|transit| world.systems.get(transit.to))
        .map(|system| system.name.clone())
        .or_else(|| {
            character
                .current_fleet
                .and_then(|fleet| fleet_label(world, fleet))
        })
        .or_else(|| {
            character
                .current_system
                .and_then(|system| world.systems.get(system))
                .map(|system| system.name.clone())
        })
        // An abode is a container like a system (FUN_00556390).
        .or_else(|| {
            character
                .current_abode
                .and_then(|abode| world.abodes.get(abode))
                .map(|abode| abode.name.clone())
        })
        .unwrap_or_default();
    // The first that holds: en route (+0x50 bit 4), captured (+0xac bit 0),
    // injured (+0x94), a mission that is not hidden (FUN_00520b70).
    let status = if transit.is_some() {
        "Enroute"
    } else if character.is_captive {
        "Captured"
    } else if character.injury != 0 {
        "Injured"
    } else if character.on_mission && !character.on_hidden_mission {
        "On Mission"
    } else {
        "Awaiting Orders"
    };
    let mut rows = vec![
        row("Commanding:", "None"),
        // FUN_0044a2e0: the literal, for the shipped TEXTSTRA 1.00.00
        // (FUN_00406850).
        row("Attached: ", attached),
        row("Status:", status),
    ];
    // FUN_004fd2b0: "ETA Destination:" and TEXTSTRA 14356 "Day " plus the
    // arrival day, only while en route.
    if let Some(transit) = transit {
        rows.push(row("ETA Destination:", format!("Day {}", transit.arrival)));
    }
    rows.extend([
        // +0x8c, the Force level (FUN_004eee40).
        row("Force Ranking:", force_ranking(character.force_level())),
        // +0x7c and +0x7e, the effective diplomacy and espionage (slots
        // +0x1dc, +0x1e0) with the Force share.
        row(
            "Diplomacy Rating:",
            character.effective_skill(Skill::Diplomacy).to_string(),
        ),
        row(
            "Espionage Rating:",
            character.effective_skill(Skill::Espionage).to_string(),
        ),
        // +0x86, the effective combat (slot +0x1f0).
        row("Combat Rating:", character.effective_combat().to_string()),
        // +0x88, the effective leadership (slot +0x1f4) with the seat of
        // power's +0x92 (FUN_004eede0).
        row(
            "Leadership Rating:",
            rebellion_core::seat_of_power::effective_leadership(world, character).to_string(),
        ),
        // TEXTSTRA 34608 "R&&D Capabilities", DrawText's escape for "&".
        row("R&D Capabilities", ""),
        row(" Ship Design", yes_no(character.ship_design.base != 0)),
        row(
            " Troop Training",
            yes_no(character.troop_training.base != 0),
        ),
        row(
            " Facility Design",
            yes_no(character.facility_design.base != 0),
        ),
        row("Possible Command Ranks", ""),
        row("Admiral:", yes_no(character.can_be_admiral)),
        row("General:", yes_no(character.can_be_general)),
        row("Commander:", yes_no(character.can_be_commander)),
    ]);
    Some(rows)
}

/// The background for an object of side index `side`, seen by `viewer`
/// (`FUN_00443130`); a side past 2 takes the third ("other") bitmap.
#[must_use]
pub fn background(side: usize, viewer: CockpitFaction) -> u32 {
    let base = BACKGROUND.get(side).copied().unwrap_or(BACKGROUND[2]);
    match viewer {
        CockpitFaction::Alliance => base,
        CockpitFaction::Empire => base + 3,
    }
}

/// What the player chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusWindowAction {
    /// `0x66`: close, then open the Encyclopedia (`FUN_0041d6b0`). port: on
    /// its index, as the object menu's Encyclopedia opens it.
    Encyclopedia,
    /// `0x65`, or Escape: close.
    Close,
}

#[derive(Debug, Clone, PartialEq)]
struct OpenStatusWindow {
    object: StatusObject,
    /// The list's scroll offset, in window pixels.
    scroll: f32,
    /// A press held on the list's scroll bar.
    scroll_hold: Option<crate::list_scroll_bar::Hold>,
}

/// Whether a Status window is open. There is one at a time: the galaxy view
/// builds it from its `0x468` handler, as the move confirmation.
#[derive(Debug, Clone, Default)]
pub struct StatusWindowState {
    window: Option<OpenStatusWindow>,
}

impl StatusWindowState {
    /// `FUN_0042a440` → the `0x468` handler: open on an object the player
    /// knows (`FUN_004f2d10`).
    pub fn open(&mut self, object: StatusObject) {
        self.window = Some(OpenStatusWindow {
            object,
            scroll: 0.0,
            scroll_hold: None,
        });
    }

    /// Open on a character.
    pub fn open_character(&mut self, character: CharacterKey) {
        self.open(StatusObject::Character(character));
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.window.is_some()
    }

    /// Whether `point` falls on the open window, so the galaxy map under it
    /// takes no input.
    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        self.window.is_some() && rect_contains(window_rect(layout), egui::pos2(point.0, point.1))
    }
}

/// The window's screen rectangle (`FUN_00606980` places it from the galaxy
/// view's rectangle). hyp: centred there, as the move confirmation is.
#[must_use]
pub fn window_rect(layout: CockpitLayout) -> egui::Rect {
    galaxy_centered_rect(layout, STATUS_WINDOW_WIDTH, STATUS_WINDOW_HEIGHT)
}

/// `FUN_00449e00`'s row placement from each row's (label, value) height: a
/// row is as tall as its taller cell, and a shorter value moves down to the
/// label's bottom. Returns each row's (label, value) top and the total.
/// An icon's top-left in its row's cell: centred in the 37 by 37 canvas
/// with integer halves (`FUN_005fcc30(.., (w0 - w) / 2, (h0 - h) / 2)`).
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "Bitmap sizes are small integers."
)]
fn icon_offset(width: usize, height: usize) -> (f32, f32) {
    (
        ((ICON_CELL - width as i32) / 2) as f32,
        ((ICON_CELL - height as i32) / 2) as f32,
    )
}

fn row_tops(heights: &[(f32, f32)]) -> (Vec<(f32, f32)>, f32) {
    let mut tops = Vec::with_capacity(heights.len());
    let mut y = 0.0;
    for &(label, value) in heights {
        tops.push((y, y + (label - value).max(0.0)));
        y += label.max(value);
    }
    (tops, y)
}

/// The picture's layers bottom to top (`FUN_0042c3b0`): the marks, then the
/// object keyed over them.
fn picture_layers(
    marks: &[(DllSource, u32)],
    picture: (DllSource, u32),
) -> impl Iterator<Item = (DllSource, u32)> + '_ {
    marks.iter().copied().chain(std::iter::once(picture))
}

/// The picture's top-left in the window: centred on (307, 64) with integer
/// halves (`FUN_00443130`).
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "Bitmap sizes are small integers."
)]
fn picture_origin(width: usize, height: usize) -> (f32, f32) {
    let (cx, cy) = PICTURE_CENTRE;
    (
        (cx - width as i32 / 2) as f32,
        (cy - height as i32 / 2) as f32,
    )
}

/// The list's scroll after a wheel `delta` (screen pixels), within the
/// scroll bar's range for rows `height` (screen pixels) high
/// ([`crate::list_scroll_bar::max_first`]). port: the original has no
/// wheel.
fn scrolled(scroll: f32, delta: f32, scale: f32, height: f32) -> f32 {
    (scroll - delta / scale).clamp(0.0, crate::list_scroll_bar::max_first(height / scale, LIST.3))
}

/// The list's scroll bar for rows `content` window pixels high.
fn list_bar(content: f32, scroll: f32) -> crate::list_scroll_bar::ScrollBar {
    crate::list_scroll_bar::ScrollBar::new(BAR_ART, LIST.0, LIST.1, LIST.2, LIST.3, content, scroll)
}

/// One laid-out row: its label's and value's lines and their tops.
type LaidRow = (Vec<String>, Vec<String>, f32, f32);

/// Break the rows' cells into the lines `DrawTextA` makes of them
/// (`DT_WORDBREAK` in the 103-wide columns, font 10), each line one
/// `tmHeight` high, and place them with [`row_tops`]. Logical pixels.
fn layout_rows(painter: &egui::Painter, rows: &[StatusRow], scale: f32) -> (Vec<LaidRow>, f32) {
    let (ascent, descent) = crate::theme::original_font_metrics(LIST_FONT);
    let pitch = ascent + descent;
    let lines = |text: &str| {
        crate::theme::gdi_word_break(text, COLUMN, |line| {
            crate::theme::gdi_text_width(painter, LIST_FONT, line, scale)
        })
    };
    let cells: Vec<_> = rows
        .iter()
        .map(|row| (lines(&row.label), lines(&row.value)))
        .collect();
    #[expect(clippy::cast_precision_loss, reason = "A handful of lines.")]
    let heights: Vec<_> = cells
        .iter()
        .map(|(label, value)| (label.len() as f32 * pitch, value.len() as f32 * pitch))
        .collect();
    let (tops, height) = row_tops(&heights);
    let laid = cells
        .into_iter()
        .zip(tops)
        .map(|((label, value), (label_y, value_y))| (label, value, label_y, value_y))
        .collect();
    (laid, height)
}

/// Draw the open window, if any, and report what the player chose.
pub fn draw_status_window(
    ctx: &egui::Context,
    sources: StatusSources<'_>,
    state: &mut StatusWindowState,
    viewer: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Option<StatusWindowAction> {
    let open = state.window.as_mut()?;
    // The window closes once its object is gone.
    let Some(view) = status_view(sources, open.object) else {
        state.window = None;
        return None;
    };
    let scale = layout.scale;
    let rect = window_rect(layout);
    let mut action = None;

    // Above the modeless windows, as the move confirmation is.
    egui::Area::new(egui::Id::new("original-status-window"))
        .fixed_pos(rect.min)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            let (frame, _) = ui.allocate_exact_size(rect.size(), egui::Sense::hover());
            let painter = ui.painter().with_clip_rect(frame);
            let at = |(x, y, w, h): (f32, f32, f32, f32)| logical_rect(frame, scale, x, y, w, h);

            paint(
                &painter,
                ctx,
                cache,
                background(view.side, viewer),
                frame,
                scale,
            );

            // The title and name in font 5, the list in font 10
            // (FUN_00443130, FUN_00449e00), as DrawTextA sets them
            // (theme::paint_gdi_text). hyp: the production manager's font
            // 0x11 is not mapped; its title takes font 5.
            use crate::theme::{GdiAlign, GdiVAlign};

            // Format 0x21 centres one line without DT_VCENTER: the text
            // hangs from the box's top (A0 2026-10-29: caps from y 21).
            crate::theme::paint_gdi_text(
                &painter,
                at(TITLE),
                scale,
                5,
                view.title,
                egui::Color32::WHITE,
                (GdiAlign::Center, GdiVAlign::Top),
            );

            // The picture, keyed (FUN_005fd0f0), centred with integer
            // offsets on (307, 64).
            // FUN_0042c3b0 draws the marks into the picture's own bitmap
            // first, at its top-left, so they show within its size and under
            // the object.
            if let Some((source, picture)) = view.picture {
                if let Some([width, height]) = cache.original_resource_size(source, picture) {
                    let (x, y) = picture_origin(width, height);
                    let bounds = logical_rect(frame, scale, x, y, width as f32, height as f32);
                    let layers = painter.with_clip_rect(bounds.intersect(painter.clip_rect()));
                    for (layer_source, layer) in picture_layers(&view.marks, (source, picture)) {
                        paint_native(&layers, ctx, cache, layer_source, layer, frame, scale, x, y);
                    }
                }
            }

            // Format 0x11 (DT_CENTER | DT_WORDBREAK): centred across, from
            // the box's top (A0 2026-10-29: "Objectives" at y 218 + 3).
            let name = at(NAME);
            let _ = crate::theme::paint_gdi_text_block(
                &painter.with_clip_rect(name),
                name,
                scale,
                5,
                &view.name,
                egui::Color32::WHITE,
                GdiAlign::Center,
            );

            // The list. port: the wheel scrolls it; no scroll bar is drawn.
            let list = at(LIST);
            let (laid, mut height) = layout_rows(&painter, &view.rows, scale);
            height *= scale;
            #[expect(clippy::cast_precision_loss, reason = "A handful of rows.")]
            if !view.icon_rows.is_empty() {
                height = view.icon_rows.len() as f32 * ICON_ROW_HEIGHT * scale;
            }
            let list_response =
                ui.interact(list, ui.id().with("status-list"), egui::Sense::hover());
            let wheel = if list_response.hovered() {
                ui.ctx().input(|input| input.smooth_scroll_delta.y)
            } else {
                0.0
            };
            // The bar: the pointer's press, hold and release on it, then the
            // wheel.
            let (pointer, pressed, down, released, now) = ui.ctx().input(|input| {
                (
                    input.pointer.interact_pos(),
                    input.pointer.button_pressed(egui::PointerButton::Primary),
                    input.pointer.button_down(egui::PointerButton::Primary),
                    input.pointer.button_released(egui::PointerButton::Primary),
                    input.time,
                )
            });
            let pointer_at = pointer.map_or((-1.0, -1.0), |point| {
                ((point.x - frame.min.x) / scale, (point.y - frame.min.y) / scale)
            });
            let bar = list_bar(height / scale, open.scroll);
            open.scroll = crate::list_scroll_bar::update(
                &bar,
                &mut open.scroll_hold,
                crate::list_scroll_bar::Pointer {
                    at: pointer_at,
                    pressed,
                    down,
                    released,
                },
                LIST_STEP,
                now,
            );
            open.scroll = scrolled(open.scroll, wheel, scale, height);
            let list_painter = painter.with_clip_rect(list);
            let top = list.min.y - open.scroll * scale;
            for (index, row) in view.icon_rows.iter().enumerate() {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "A handful of rows."
                )]
                let top = LIST.1 - open.scroll + index as f32 * ICON_ROW_HEIGHT;
                let overview = view.icon_layout == IconRowLayout::Overview;
                if let Some([width, height]) = cache.original_resource_size(row.source, row.icon) {
                    let (dx, dy) = if overview {
                        (0.0, 0.0)
                    } else {
                        icon_offset(width, height)
                    };
                    paint_native(
                        &list_painter,
                        ctx,
                        cache,
                        row.source,
                        row.icon,
                        frame,
                        scale,
                        LIST.0 + ICON_ROW_INSET + dx,
                        top + dy,
                    );
                }
                let (rect, format) = if overview {
                    // Centred across the cell less its 3 trimmed pixels,
                    // from the row's top.
                    (
                        (LIST.0 + ICON_ROW_INSET, top, LIST.2 - 3.0, ICON_ROW_HEIGHT),
                        (GdiAlign::Center, GdiVAlign::Top),
                    )
                } else {
                    // At x 40, centred on the row's height (format 0x24).
                    (
                        (
                            LIST.0 + ICON_ROW_INSET + ICON_TEXT_X,
                            top,
                            LIST.2 - ICON_ROW_INSET - ICON_TEXT_X,
                            ICON_ROW_HEIGHT,
                        ),
                        (GdiAlign::Left, GdiVAlign::VCenter),
                    )
                };
                crate::theme::paint_gdi_text(
                    &list_painter,
                    at(rect),
                    scale,
                    LIST_FONT,
                    &row.text,
                    egui::Color32::WHITE,
                    format,
                );
            }
            // The held arrow draws pressed while the pointer stays on it.
            let bar = list_bar(height / scale, open.scroll);
            let held = open.scroll_hold.map(|hold| hold.part).filter(|part| {
                matches!(
                    part,
                    crate::list_scroll_bar::Part::Up | crate::list_scroll_bar::Part::Down
                ) && down
                    && bar.part_at(pointer_at) == Some(*part)
            });
            let (bar_x, bar_y, bar_w, bar_h) = bar.rect();
            let bar_painter = painter.with_clip_rect(logical_rect(frame, scale, bar_x, bar_y, bar_w, bar_h));
            for piece in bar.pieces(held) {
                paint_native(
                    &bar_painter,
                    ctx,
                    cache,
                    DllSource::Strategy,
                    piece.id,
                    frame,
                    scale,
                    piece.x,
                    piece.y,
                );
            }
            // Each cell's lines from its top (DT_TOP), one tmHeight apart;
            // labels at the list's left, values 103 to their right.
            let (ascent, descent) = crate::theme::original_font_metrics(LIST_FONT);
            let pitch = ascent + descent;
            for (label, value, label_y, value_y) in laid {
                for (lines, x, y) in [(label, 0.0, label_y), (value, COLUMN, value_y)] {
                    for (index, line) in lines.iter().enumerate() {
                        #[expect(clippy::cast_precision_loss, reason = "A handful of lines.")]
                        let line_top = y + index as f32 * pitch;
                        crate::theme::paint_gdi_text(
                            &list_painter,
                            egui::Rect::from_min_size(
                                egui::pos2(list.min.x + x * scale, top + line_top * scale),
                                egui::vec2(COLUMN * scale, pitch * scale),
                            ),
                            scale,
                            LIST_FONT,
                            line,
                            egui::Color32::WHITE,
                            (GdiAlign::Left, GdiVAlign::Top),
                        );
                    }
                }
            }

            // FUN_00443020: Encyclopedia TEXTSTRA 0x1505 and Close 0x1954
            // (A0 tooltip/122, 123); a disabled Encyclopedia shows none.
            crate::win_tooltip::window_tool(
                ui,
                at(ENCYCLOPEDIA_RECT),
                "status-encyclopedia",
                ENCYCLOPEDIA_TOOLTIP,
                view.encyclopedia,
            );
            crate::win_tooltip::window_tool(
                ui,
                at(CLOSE_RECT),
                "status-close",
                CLOSE_TOOLTIP,
                true,
            );
            if !view.encyclopedia {
                paint(
                    ui.painter(),
                    ctx,
                    cache,
                    ENCYCLOPEDIA_DISABLED,
                    at(ENCYCLOPEDIA_RECT),
                    scale,
                );
            } else if button(
                ui,
                cache,
                at(ENCYCLOPEDIA_RECT),
                "status-encyclopedia",
                ENCYCLOPEDIA,
                false,
                scale,
            ) {
                action = Some(StatusWindowAction::Encyclopedia);
            }
            if button(
                ui,
                cache,
                at(CLOSE_RECT),
                "status-close",
                CLOSE,
                false,
                scale,
            ) {
                action = Some(StatusWindowAction::Close);
            }
        });

    // port: Escape closes; no key slot is traced.
    if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        action = Some(StatusWindowAction::Close);
    }
    if action.is_some() {
        state.window = None;
    }
    action
}

#[cfg(test)]
mod tests {
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
            last_arrival: None,
            dat_id: DatId::new(100),
            name: name.into(),
            sector,
            x: 320,
            y: 250,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: Vec::new(),
            ground_units: Vec::new(),
            special_forces: Vec::new(),
            defense_facilities: Vec::new(),
            manufacturing_facilities: Vec::new(),
            production_facilities: Vec::new(),
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Alliance),
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
        let rows = character_rows(&world, &[], key).expect("rows");
        assert!(pairs(&rows).contains(&("Attached: ", "Dagobah")));
    }

    #[test]
    fn a_character_at_a_system_lists_the_rows_of_figure_3_46() {
        // FUN_004486f0, in order; manual p. 101, Fig. 3.46 (Luke at Yavin,
        // Awaiting Orders, Trainee, 75/75/135/70, no R&D, Admiral Yes).
        let (world, key, ..) = world_with_luke();
        let rows = character_rows(&world, &[], key).expect("rows");
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
            let rows = character_rows(world, &[], vader).expect("rows");
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
        let rows = character_rows(&world, &[], key).expect("rows");
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
        let rows = character_rows(&world, &[transit], key).expect("rows");
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
            character_rows(world, &[], key).expect("rows")[2]
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
            // DT_VCENTER on the 35-pixel row: (35 - 14) / 2 = 10 down, then
            // tmAscent 11.
            #[expect(clippy::cast_precision_loss, reason = "Two rows.")]
            let pen = at(59.0, 44.0 + 10.0 + 11.0 + 35.0 * index as f32);
            assert!(near(*pos, pen), "{pos:?}");
            assert_eq!(*font, crate::theme::gdi_font(10, 2.0).size);
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
        // DT_CENTER in the 205-wide cell, from its top (tmAscent 11).
        let (_, pos, ..) = find(&texts, "001  0001");
        let (_, width) = crate::theme::gdi_pen_positions(10, "001  0001").unwrap();
        assert!(
            near(*pos, at(19.0 + ((205.0 - width) / 2.0).floor(), 47.0 + 11.0)),
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
}
