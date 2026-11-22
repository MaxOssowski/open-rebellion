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
use rebellion_core::character_command::CommandKind;
use rebellion_core::ids::{CharacterKey, FleetKey};
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
/// The picture list's font (`FUN_00443130`: the list's `vt+0x18`,
/// `FUN_00600290`, sets `+0x7c` to `FUN_0060eea0(4)`): the Objectives' and
/// the Galaxy Overview's rows. A0 agent-menu/empire-objectives-day137 and
/// empire-galaxy-overview-day141-scroll6 draw them larger than the rows.
const ICON_LIST_FONT: u16 = 4;
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
pub(crate) const BAR_ART: crate::list_scroll_bar::BarArt = crate::list_scroll_bar::BarArt {
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
/// transits (`MissionState::en_route`), and `bound` the members among them
/// whose Move lands them aboard a fleet (`quadrant_icons::fleet_arrivals`).
///
/// Commanding (`+0xa8`) names the object the character commands: the
/// Command order (`FUN_00561bb0`, `FUN_004ee740`) keys it to the fleet of the
/// ship it is aboard, else to its system (`FUN_0055ed30`), and a move clears
/// the command (`FUN_00560d50`). port: so the commanded object is where a
/// character with a command now stands. A character is en route while its
/// mission transit lasts; Attached then names the transit's destination,
/// the container of an object in hyperspace (`FUN_00556390`).
#[must_use]
pub fn character_rows(
    world: &GameWorld,
    en_route: &[MemberTransit],
    bound: &[(MissionMember, FleetKey)],
    key: CharacterKey,
) -> Option<Vec<StatusRow>> {
    let character = world.characters.get(key)?;
    let transit = en_route
        .iter()
        .find(|transit| transit.member == MissionMember::Character(key));
    let attached = transit
        .and_then(|transit| transit_container(world, bound, transit))
        // Aboard a fleet, the ship it is seated on, else the fleet's first
        // (fleet_window::ship_cargo).
        .or_else(|| {
            character.current_fleet.and_then(|fleet| {
                let seat = character
                    .ship
                    .and_then(|id| {
                        world.fleets.get(fleet)?.capital_ships.iter().position(|ship| ship.id == id)
                    })
                    .or_else(|| crate::fleet_window::first_seat(world, fleet));
                crate::fleet_window::aboard_label(world, fleet, seat)
            })
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
    // A0 personnel-states/11: General Neva at Pzob, "Commanding: Pzob".
    let commanding = (character.command != CommandKind::None)
        .then(|| {
            character
                .current_fleet
                .and_then(|fleet| fleet_label(world, fleet))
                .or_else(|| {
                    character
                        .current_system
                        .and_then(|system| world.systems.get(system))
                        .map(|system| system.name.clone())
                })
        })
        .flatten()
        .unwrap_or_else(|| "None".to_owned());
    let mut rows = vec![
        row("Commanding:", commanding),
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

/// The name of the container a member in hyperspace is in: `FUN_00556390`
/// reparents it into its Move's target container (`+0x74`) at once: a ship
/// of the fleet when the Move targets one (`bound`,
/// `quadrant_icons::fleet_arrivals`), else the transit's system.
pub(crate) fn transit_container(
    world: &GameWorld,
    bound: &[(MissionMember, FleetKey)],
    transit: &MemberTransit,
) -> Option<String> {
    match bound.iter().find(|(member, _)| *member == transit.member) {
        // FUN_00552300 seats it on the fleet's first usable ship.
        Some(&(_, fleet)) => crate::fleet_window::aboard_label(
            world,
            fleet,
            crate::fleet_window::first_seat(world, fleet),
        ),
        None => world.systems.get(transit.to).map(|system| system.name.clone()),
    }
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

    /// The object the open window shows.
    #[must_use]
    pub fn object(&self) -> Option<StatusObject> {
        self.window.as_ref().map(|window| window.object)
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
                    ICON_LIST_FONT,
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
#[path = "status_window_tests.rs"]
mod tests;
