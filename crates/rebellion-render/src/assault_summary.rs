//! The original Assault Summary window (`FUN_0044c410`, vtable
//! `0x00659c78`; manual Figs. 3.66 and 3.67, p. 123). Recovery notes:
//! `ghidra/notes/assault-summary-window.md`.
//!
//! It shows one [`AssaultReport`]: the summary page (the picture, the
//! title and the outcome), each side's forces by category, and the Display
//! page that goes to the system. It shares the Battle Results window's
//! frame, rail and Close button (`FUN_0044c630`).

use std::collections::HashMap;

use egui_macroquad::egui;
use rebellion_core::assault_report::{AssaultReport, ReportCategory, ReportForces, ReportUnit};
use rebellion_core::dat::Faction;
use rebellion_core::ids::SystemKey;
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::fleet_window::paint_native;
use crate::message_log::MessageId;
use crate::mission_dialog::{galaxy_centered_rect, paint_centered};
use crate::panels::fleets::{capital_ship_mini_id, fighter_mini_id};
use crate::story_messages::{fill_message_text, side_parameter, MessageParameter};
use crate::system_window::{
    character_mini_resource_id, defense_facility_mini, exact_clicked, logical_rect,
    manufacturing_facility_mini, production_facility_mini, rect_contains, special_force_mini,
    troop_mini,
};

/// `FUN_0044c410`: 470 by 331 (`0x1d6` by `0x14b`).
pub const ASSAULT_SUMMARY_WIDTH: f32 = 470.0;
pub const ASSAULT_SUMMARY_HEIGHT: f32 = 331.0;

/// STRATEGY `0x285f` / `0x2860`: the frame, by the viewer's side.
const FRAME: [u32; 2] = [10335, 10336];
/// STRATEGY `0x2a44` / `0x2a45`: the rail, drawn at x 412.
const RAIL: [u32; 2] = [10820, 10821];
const RAIL_X: f32 = 412.0;
/// The page art at (12, 13): two columns `0x2a09`, three columns `0x2a0a`
/// (Personnel), Display `0x2a0b`.
const PAGE: (f32, f32) = (12.0, 13.0);
const FORCES_TWO_COLUMNS: u32 = 10761;
const FORCES_THREE_COLUMNS: u32 = 10762;
const DISPLAY_PAGE: u32 = 10763;

/// Close (`10`): `0x2882`/`0x2883` at (423, 25), 32 by 31, for the
/// Alliance; `0x2888`/`0x2889` at (426, 21), 44 by 41, for the Empire.
const CLOSE: [(u32, u32, f32, f32, f32, f32); 2] = [
    (10370, 10371, 423.0, 25.0, 32.0, 31.0),
    (10376, 10377, 426.0, 21.0, 44.0, 41.0),
];

/// The rail's buttons (`0x65..0x68`): x, the first y, the y offsets and the
/// size, by the viewer's side.
const RAIL_BUTTONS: [(f32, f32, [f32; 4], f32, f32); 2] = [
    (418.0, 88.0, [0.0, 53.0, 108.0, 162.0], 41.0, 41.0),
    (426.0, 89.0, [0.0, 59.0, 118.0, 177.0], 44.0, 41.0),
];

/// The category bar (`+0x160`) at (36, 59); each button 49 by 41 at these
/// x offsets (`FUN_0044c630`).
const CATEGORY_BAR: (f32, f32) = (36.0, 59.0);
const CATEGORY_X: [f32; 6] = [0.0, 62.0, 124.0, 186.0, 248.0, 312.0];
const CATEGORY_SIZE: (f32, f32) = (49.0, 41.0);

/// The list (`+0x120`) at (37, 136), 347 by 166.
const LIST: (f32, f32, f32, f32) = (37.0, 136.0, 347.0, 166.0);
/// `FUN_0044e500`'s cells: 165 by 70 in two columns, 110 by 50 in three.
const CELLS: [(f32, f32); 2] = [(165.0, 70.0), (110.0, 50.0)];
/// The column heads' x (y 117), 165 wide.
/// "Operational" `0x1932`, "Destroyed" `0x1933`; "Survivors" `0x1934`,
/// "Captured" `0x1935`, "Killed" `0x1936`.
const HEADS_TWO: [(f32, &str); 2] = [(38.0, "Operational"), (204.0, "Destroyed")];
const HEADS_THREE: [(f32, &str); 3] = [(38.0, "Survivors"), (165.0, "Captured"), (275.0, "Killed")];

/// Display's buttons (`0xb`, `0xc`), 169 by 96 at y 182.
const GOTO_SYSTEM: (f32, f32) = (29.0, 182.0);
const GOTO_FLEET: (f32, f32) = (226.0, 182.0);
const GOTO_SIZE: (f32, f32) = (169.0, 96.0);

/// The rail's pages (`FUN_0044e3d0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SummaryPage {
    /// `0x65`, "Summary".
    Summary,
    /// `0x66`, the attacker's forces.
    Attackers,
    /// `0x67`, "System Forces".
    SystemForces,
    /// `0x68`, "Display".
    Display,
}

impl SummaryPage {
    pub const ALL: [Self; 4] = [
        Self::Summary,
        Self::Attackers,
        Self::SystemForces,
        Self::Display,
    ];
}

/// One open window.
#[derive(Debug, Clone)]
struct OpenSummary {
    serial: u64,
    report: AssaultReport,
    title: String,
    body: String,
    /// The Display page's system button label: the system's name
    /// (`FUN_004f62d0`), or "Goto System" (`0x1945`) without one.
    system_name: String,
    message: Option<MessageId>,
    page: SummaryPage,
    category: ReportCategory,
    scroll: usize,
}

/// The open Assault Summary windows, the last on top.
#[derive(Debug, Clone, Default)]
pub struct AssaultSummaryState {
    windows: Vec<OpenSummary>,
    serial: u64,
}

/// What one window shows ([`AssaultSummaryState::report`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssaultSummaryReport {
    pub title: String,
    pub body: String,
    pub page: SummaryPage,
    pub category: ReportCategory,
    pub picture: u32,
    /// The shown columns' names (Operational, [Captured,] Destroyed).
    pub columns: Vec<Vec<String>>,
}

/// What leaves the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssaultSummaryAction {
    /// Close (`10`) or Escape: the message is marked (`FUN_00469d60`).
    Closed(Option<MessageId>),
    /// `0xb`: go to the system (`FUN_00429ce0`).
    GotoSystem(SystemKey),
    /// `0xc`: go to the system and open its Fleet window (`FUN_0045c8e0`,
    /// kind 4).
    GotoFleets(SystemKey),
}

impl AssaultSummaryState {
    /// Open a window for `report` on the Summary page (`FUN_0060d7e0(0x65)`)
    /// with its first category (`+0x180`).
    pub fn open(
        &mut self,
        world: &GameWorld,
        report: AssaultReport,
        texts: &HashMap<u16, String>,
        message: Option<MessageId>,
    ) {
        let (title, body) = summary_texts(&report, world, texts);
        let category = report.first_category();
        let system_name = world
            .systems
            .get(report.system)
            .map_or_else(|| "Goto System".to_owned(), |value| value.name.clone());
        self.serial += 1;
        self.windows.push(OpenSummary {
            serial: self.serial,
            report,
            title,
            body,
            system_name,
            message,
            page: SummaryPage::Summary,
            category,
            scroll: 0,
        });
    }

    #[must_use]
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn clear(&mut self) {
        self.windows.clear();
    }

    /// Show `page` on the top window, as its rail button does.
    pub fn show_page(&mut self, page: SummaryPage) -> bool {
        let Some(window) = self.windows.last_mut() else {
            return false;
        };
        window.page = page;
        true
    }

    /// Show `category` on the top window, as its button does.
    pub fn show_category(&mut self, category: ReportCategory) -> bool {
        let Some(window) = self.windows.last_mut() else {
            return false;
        };
        window.category = category;
        window.scroll = 0;
        true
    }

    /// The top window's contents.
    #[must_use]
    pub fn report(&self) -> Option<AssaultSummaryReport> {
        let window = self.windows.last()?;
        let forces = shown_forces(window);
        Some(AssaultSummaryReport {
            title: window.title.clone(),
            body: window.body.clone(),
            page: window.page,
            category: window.category,
            picture: window.report.picture(),
            columns: forces.map_or_else(Vec::new, |forces| {
                shown_columns(forces, window.category)
                    .into_iter()
                    .map(|column| column.iter().map(|unit| unit.name.clone()).collect())
                    .collect()
            }),
        })
    }
}

/// The window's title and outcome (`FUN_00496770`, TEXTSTRA RCDATA): the
/// title `0x7090` names the system; the body names the attacker, the
/// defender when the system had one, and the system.
#[must_use]
pub fn summary_texts(
    report: &AssaultReport,
    world: &GameWorld,
    texts: &HashMap<u16, String>,
) -> (String, String) {
    let system = world
        .systems
        .get(report.system)
        .map_or("", |value| value.name.as_str());
    // A system's text is its name for kinds 1, 2 and 4 (FUN_004f6030):
    // 0x7098 names it as {1:4}.
    let named = MessageParameter::Unit {
        name: system,
        class: "",
    };
    let attacker = side_parameter(report.attacker == Faction::Alliance, texts);
    let parameters: Vec<MessageParameter<'_>> = match report.defender {
        Some(defender) => vec![
            attacker,
            side_parameter(defender == Faction::Alliance, texts),
            named,
        ],
        None => vec![attacker, named],
    };
    let text = |id: u16, parameters: &[MessageParameter<'_>]| {
        texts
            .get(&id)
            .map(|template| fill_message_text(template, parameters))
            .unwrap_or_default()
    };
    (
        text(report.title(), &[named]),
        text(report.body(), &parameters),
    )
}

/// The window's screen rectangle. hyp: `FUN_0044c410` places it at the
/// galaxy view's origin (`FUN_00606380(.., 0, 0, ..)`); the port centres
/// it in the galaxy view, as the Message Index.
#[must_use]
pub fn window_rect(layout: CockpitLayout) -> egui::Rect {
    galaxy_centered_rect(layout, ASSAULT_SUMMARY_WIDTH, ASSAULT_SUMMARY_HEIGHT)
}

fn side_index(faction: CockpitFaction) -> usize {
    usize::from(faction == CockpitFaction::Empire)
}

/// The side's text colour (`+0x6c`: 1 red, 2 green, else yellow).
fn side_color(side: Option<Faction>) -> egui::Color32 {
    match side {
        Some(Faction::Alliance) => egui::Color32::from_rgb(255, 0, 0),
        Some(Faction::Empire) => egui::Color32::from_rgb(0, 255, 0),
        _ => egui::Color32::from_rgb(255, 255, 0),
    }
}

/// A forces button's art by the side it shows (`0x29ea`, `0x29ec`,
/// neutral `0x29f0`; the Empire viewer's `+10`).
fn forces_art(side: Option<Faction>, viewer: usize) -> (u32, u32) {
    let base = match side {
        Some(Faction::Alliance) => 10730,
        Some(Faction::Empire) => 10732,
        _ => 10736,
    };
    let base = base + 10 * viewer as u32;
    (base, base + 1)
}

/// The rail button's art (normal, pressed) and label.
fn rail_art(page: SummaryPage, report: &AssaultReport, viewer: usize) -> (u32, u32, String) {
    match page {
        SummaryPage::Summary => {
            let base = [10728, 10738][viewer];
            (base, base + 1, "Summary".into())
        }
        SummaryPage::Attackers => {
            let (normal, pressed) = forces_art(Some(report.attacker), viewer);
            let label = if report.attacker == Faction::Alliance {
                "Alliance Forces"
            } else {
                "Imperial Forces"
            };
            (normal, pressed, label.into())
        }
        SummaryPage::SystemForces => {
            let (normal, pressed) = forces_art(report.defender, viewer);
            (normal, pressed, "System Forces".into())
        }
        SummaryPage::Display => {
            let (normal, pressed) = [(10791, 10792), (10520, 10521)][viewer];
            (normal, pressed, "Display".into())
        }
    }
}

/// A category button's art (normal, pressed) by the viewer's side.
fn category_art(category: ReportCategory, viewer: usize) -> (u32, u32) {
    let (alliance, empire) = match category {
        ReportCategory::CapitalShips => ((10348, 10347), (10360, 10359)),
        ReportCategory::Fighters => ((10354, 10353), (10364, 10363)),
        ReportCategory::Manufacturing => ((10344, 10343), (10356, 10355)),
        ReportCategory::Defense => ((10342, 10341), (10342, 10341)),
        ReportCategory::Troops => ((10352, 10351), (10362, 10361)),
        ReportCategory::Personnel => ((10346, 10345), (10358, 10357)),
    };
    if viewer == 0 {
        alliance
    } else {
        empire
    }
}

fn shown_forces(window: &OpenSummary) -> Option<&ReportForces> {
    match window.page {
        SummaryPage::Attackers => Some(&window.report.attackers),
        SummaryPage::SystemForces => Some(&window.report.system_forces),
        SummaryPage::Summary | SummaryPage::Display => None,
    }
}

/// The columns `FUN_0044e500` fills: Operational and Destroyed, or for
/// Personnel Survivors, Captured and Killed.
fn shown_columns(forces: &ReportForces, category: ReportCategory) -> Vec<Vec<&ReportUnit>> {
    let [operational, captured, destroyed] = forces.columns(category);
    if category == ReportCategory::Personnel {
        vec![operational, captured, destroyed]
    } else {
        vec![operational, destroyed]
    }
}

/// A unit's GOKRES picture (`FUN_0042c3b0`): the portrait (the mini less
/// `0x4000`) for a craft, a regiment or a facility, the mini for personnel
/// (`FUN_0044edb0`'s `param_3`).
fn unit_picture(unit: &ReportUnit) -> Option<u32> {
    let mini = match unit.category {
        ReportCategory::CapitalShips => capital_ship_mini_id(unit.dat_id),
        ReportCategory::Fighters => fighter_mini_id(unit.dat_id),
        ReportCategory::Manufacturing => manufacturing_facility_mini(unit.dat_id)
            .or_else(|| production_facility_mini(unit.dat_id))
            .map(|(id, _)| id),
        ReportCategory::Defense => defense_facility_mini(unit.dat_id).map(|(id, _)| id),
        ReportCategory::Troops => troop_mini(unit.dat_id).map(|(id, _)| id),
        ReportCategory::Personnel => {
            return special_force_mini(unit.dat_id)
                .map(|(id, _)| id)
                .or_else(|| character_mini_resource_id(unit.dat_id, unit.major));
        }
    }?;
    mini.checked_sub(0x4000)
}

#[derive(Default)]
struct DrawResult {
    close: bool,
    page: Option<SummaryPage>,
    category: Option<ReportCategory>,
    scroll: Option<usize>,
    action: Option<AssaultSummaryAction>,
}

/// Draw the open windows and report what leaves them.
pub fn draw_assault_summaries(
    ctx: &egui::Context,
    state: &mut AssaultSummaryState,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Vec<AssaultSummaryAction> {
    let mut actions = Vec::new();
    let top = state.windows.last().map(|window| window.serial);
    let windows = state.windows.clone();
    for window in &windows {
        let focused = top == Some(window.serial);
        let result = draw_window(ctx, window, focused, faction, layout, cache);
        actions.extend(state.apply(window.serial, &result));
    }
    actions
}

impl AssaultSummaryState {
    /// Carry out what one window's frame asked: a button that leaves
    /// closes it (`FUN_0044df90`: `0xb`/`0xc` close after going), Close
    /// marks its message; a rail or category button shows its page or
    /// category (`FUN_0044e3d0`, `FUN_0044e500` refill only for a new one).
    fn apply(&mut self, serial: u64, result: &DrawResult) -> Vec<AssaultSummaryAction> {
        let Some(index) = self.windows.iter().position(|open| open.serial == serial) else {
            return Vec::new();
        };
        let mut actions: Vec<_> = result.action.into_iter().collect();
        if result.close || result.action.is_some() {
            let closed = self.windows.remove(index);
            actions.push(AssaultSummaryAction::Closed(closed.message));
            return actions;
        }
        let open = &mut self.windows[index];
        if let Some(page) = result.page {
            open.page = page;
        }
        if let Some(category) = result.category {
            if open.category != category {
                open.category = category;
                open.scroll = 0;
            }
        }
        if let Some(scroll) = result.scroll {
            open.scroll = scroll;
        }
        actions
    }
}

/// The column heads (`FUN_0044c630`'s pages 3 and 4).
fn heads(category: ReportCategory) -> &'static [(f32, &'static str)] {
    if category == ReportCategory::Personnel {
        &HEADS_THREE
    } else {
        &HEADS_TWO
    }
}

/// The rows the list shows at once.
fn visible_rows(category: ReportCategory) -> usize {
    let (_, height) = CELLS[usize::from(category == ReportCategory::Personnel)];
    (LIST.3 / height) as usize
}

/// A cell's window point: the list's origin plus the column and the row.
fn cell_origin(category: ReportCategory, column: usize, row: usize) -> (f32, f32) {
    let (width, height) = CELLS[usize::from(category == ReportCategory::Personnel)];
    (LIST.0 + column as f32 * width, LIST.1 + row as f32 * height)
}

/// An empty column's line: "None" (`0x8526`) for the first or middle
/// column, "No Casualties" (`0x1941`) for the last (`FUN_0044e500`).
fn empty_label(column: usize, columns: usize) -> &'static str {
    if column + 1 == columns {
        "No Casualties"
    } else {
        "None"
    }
}

/// The wheel's scroll: down a row while rows remain below, up a row while
/// any are above. port: the list's scroll bar is not drawn.
fn scrolled(scroll: usize, wheel: f32, rows: usize, visible: usize) -> Option<usize> {
    if wheel < 0.0 && scroll + visible < rows {
        Some(scroll + 1)
    } else if wheel > 0.0 && scroll > 0 {
        Some(scroll - 1)
    } else {
        None
    }
}

/// `FUN_0044e180`: Left and Right step to the neighbouring category button.
fn stepped(category: ReportCategory, left: bool, right: bool) -> Option<ReportCategory> {
    let at = ReportCategory::ALL
        .iter()
        .position(|&each| each == category)
        .unwrap_or(0);
    if left && at > 0 {
        Some(ReportCategory::ALL[at - 1])
    } else if right && at + 1 < ReportCategory::ALL.len() {
        Some(ReportCategory::ALL[at + 1])
    } else {
        None
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "The window paints in FUN_0044c630's control order; splitting it hides that order."
)]
fn draw_window(
    ctx: &egui::Context,
    window: &OpenSummary,
    focused: bool,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> DrawResult {
    let mut result = DrawResult::default();
    let scale = layout.scale;
    let viewer = side_index(faction);
    let report = &window.report;
    let screen = window_rect(layout);
    let id = egui::Id::new(("original-assault-summary", window.serial));
    if focused {
        ctx.move_to_top(egui::LayerId::new(egui::Order::Tooltip, id));
    }
    egui::Area::new(id)
        .fixed_pos(screen.min)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            let (pointer, primary_down) = ctx.input(|input| {
                (
                    input.pointer.interact_pos(),
                    input.pointer.button_down(egui::PointerButton::Primary),
                )
            });
            let (local, _) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            let painter = ui.painter().clone();
            let held = |rect: egui::Rect| {
                primary_down && pointer.is_some_and(|point| rect_contains(rect, point))
            };
            let paint = |cache: &mut BmpCache, source: DllSource, id: u32, x: f32, y: f32| {
                paint_native(&painter, ctx, cache, source, id, local, scale, x, y);
            };
            let text = |text: &str, x: f32, y: f32, width: f32, size: f32, color| {
                let galley = painter.layout(
                    text.to_owned(),
                    egui::FontId::proportional((size * scale).max(6.0)),
                    color,
                    width * scale,
                );
                let rect = logical_rect(local, scale, x, y, width, 0.0);
                let left = rect.min.x + (rect.width() - galley.size().x) / 2.0;
                painter.galley(egui::pos2(left, rect.min.y), galley, color);
            };

            painter.rect_filled(local, 0.0, egui::Color32::BLACK);
            paint(cache, DllSource::Strategy, FRAME[viewer], 0.0, 0.0);
            let color = side_color(Some(report.attacker));

            match window.page {
                SummaryPage::Summary => {
                    // The picture +0x2e, the title +0x44 (12, 13), 400
                    // wide, centred, and the outcome +0x50 at (25, 225),
                    // 350 wide, over its black shadow at (26, 226).
                    paint(cache, DllSource::Strategy, report.picture(), PAGE.0, PAGE.1);
                    text(&window.title, 12.0, 13.0, 400.0, 14.0, color);
                    let body = |x: f32, y: f32, color| {
                        let galley = painter.layout(
                            window.body.clone(),
                            egui::FontId::proportional((11.0 * scale).max(6.0)),
                            color,
                            350.0 * scale,
                        );
                        painter.galley(logical_rect(local, scale, x, y, 0.0, 0.0).min, galley, color);
                    };
                    body(26.0, 226.0, egui::Color32::BLACK);
                    body(25.0, 225.0, color);
                }
                SummaryPage::Attackers | SummaryPage::SystemForces => {
                    let personnel = window.category == ReportCategory::Personnel;
                    let art = if personnel {
                        FORCES_THREE_COLUMNS
                    } else {
                        FORCES_TWO_COLUMNS
                    };
                    paint(cache, DllSource::Strategy, art, PAGE.0, PAGE.1);
                    text(&window.title, 12.0, 13.0, 400.0, 14.0, color);
                    for (index, category) in ReportCategory::ALL.into_iter().enumerate() {
                        let (normal, pressed) = category_art(category, viewer);
                        let (x, y) = (CATEGORY_BAR.0 + CATEGORY_X[index], CATEGORY_BAR.1);
                        let rect =
                            logical_rect(local, scale, x, y, CATEGORY_SIZE.0, CATEGORY_SIZE.1);
                        let response = ui.interact(
                            rect,
                            ui.id().with((window.serial, "category", index)),
                            egui::Sense::click(),
                        );
                        let down = category == window.category || held(rect);
                        paint(cache, DllSource::Strategy, if down { pressed } else { normal }, x, y);
                        if exact_clicked(&response, rect) {
                            result.category = Some(category);
                        }
                    }
                    for &(x, label) in heads(window.category) {
                        text(label, x, 117.0, 165.0, 10.0, egui::Color32::WHITE);
                    }
                    if let Some(forces) = shown_forces(window) {
                        let columns = shown_columns(forces, window.category);
                        let (cell_w, cell_h) = CELLS[usize::from(personnel)];
                        let visible = visible_rows(window.category);
                        let list = logical_rect(local, scale, LIST.0, LIST.1, LIST.2, LIST.3);
                        let hovered = pointer.is_some_and(|point| rect_contains(list, point));
                        if hovered {
                            let wheel = ctx.input(|input| input.raw_scroll_delta.y);
                            let rows = columns.iter().map(Vec::len).max().unwrap_or(0);
                            result.scroll = scrolled(window.scroll, wheel, rows, visible);
                        }
                        let list_painter = painter.with_clip_rect(list);
                        for (column, units) in columns.iter().enumerate() {
                            let (x, _) = cell_origin(window.category, column, 0);
                            if units.is_empty() && window.scroll == 0 {
                                let label = empty_label(column, columns.len());
                                text(label, x, LIST.1 + 4.0, cell_w, 10.0, egui::Color32::WHITE);
                            }
                            let shown = units.iter().skip(window.scroll).take(visible + 1);
                            for (row, unit) in shown.enumerate() {
                                let (x, y) = cell_origin(window.category, column, row);
                                let cell = logical_rect(local, scale, x, y, cell_w, cell_h);
                                let painted = unit_picture(unit).and_then(|picture| {
                                    paint_centered(
                                        &list_painter,
                                        ctx,
                                        cache,
                                        DllSource::Gokres,
                                        picture,
                                        cell,
                                        scale,
                                    )
                                });
                                if painted.is_none() {
                                    let galley = list_painter.layout(
                                        unit.name.clone(),
                                        egui::FontId::proportional((9.0 * scale).max(6.0)),
                                        egui::Color32::WHITE,
                                        cell_w * scale,
                                    );
                                    list_painter.galley(cell.min, galley, egui::Color32::WHITE);
                                }
                                let response = ui.interact(
                                    cell.intersect(list),
                                    ui.id().with((window.serial, "unit", column, row)),
                                    egui::Sense::hover(),
                                );
                                response.on_hover_text(&unit.name);
                            }
                        }
                    }
                }
                SummaryPage::Display => {
                    paint(cache, DllSource::Strategy, DISPLAY_PAGE, PAGE.0, PAGE.1);
                    // RCDATA 0x7013 above the two buttons. port: its
                    // placement (FUN_00601b30(0x19, 0xe1) on page 5) is
                    // not settled; it ends above the buttons here.
                    text(
                        "Select one of the following buttons to close this display and go directly to... ",
                        25.0,
                        140.0,
                        375.0,
                        11.0,
                        color,
                    );
                    let selected = [10767, 10768][viewer];
                    let fleet = [10765, 10766][viewer];
                    let system = window.system_name.clone();
                    for (index, ((x, y), normal, label)) in [
                        (GOTO_SYSTEM, 10764, system),
                        (GOTO_FLEET, fleet, "Fleet".to_owned()),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rect = logical_rect(local, scale, x, y, GOTO_SIZE.0, GOTO_SIZE.1);
                        let response = ui.interact(
                            rect,
                            ui.id().with((window.serial, "goto", index)),
                            egui::Sense::click(),
                        );
                        paint(cache, DllSource::Strategy, normal, x, y);
                        if held(rect) {
                            paint(cache, DllSource::Strategy, selected, x, y);
                        }
                        // The label, white, at the button's half height
                        // less 12 (FUN_006027e0).
                        text(
                            &label,
                            x,
                            y + GOTO_SIZE.1 / 2.0 - 12.0,
                            GOTO_SIZE.0,
                            12.0,
                            egui::Color32::WHITE,
                        );
                        if exact_clicked(&response, rect) {
                            result.action = Some(if index == 0 {
                                AssaultSummaryAction::GotoSystem(report.system)
                            } else {
                                AssaultSummaryAction::GotoFleets(report.system)
                            });
                        }
                    }
                }
            }

            paint(cache, DllSource::Strategy, RAIL[viewer], RAIL_X, 0.0);
            let (normal, pressed, x, y, width, height) = CLOSE[viewer];
            let rect = logical_rect(local, scale, x, y, width, height);
            let response = ui.interact(rect, ui.id().with((window.serial, "close")), egui::Sense::click());
            paint(cache, DllSource::Strategy, if held(rect) { pressed } else { normal }, x, y);
            if exact_clicked(&response, rect) {
                result.close = true;
            }
            let (x, top, offsets, width, height) = RAIL_BUTTONS[viewer];
            for (page, offset) in SummaryPage::ALL.into_iter().zip(offsets) {
                let (normal, pressed, label) = rail_art(page, report, viewer);
                let y = top + offset;
                let rect = logical_rect(local, scale, x, y, width, height);
                let response = ui.interact(
                    rect,
                    ui.id().with((window.serial, "rail", page)),
                    egui::Sense::click(),
                );
                let down = page == window.page || held(rect);
                paint(cache, DllSource::Strategy, if down { pressed } else { normal }, x, y);
                if exact_clicked(&response, rect) {
                    result.page = Some(page);
                }
                response.on_hover_text(label);
            }

            // FUN_0044e180: Escape closes; Left and Right step through the
            // category buttons on a forces page.
            if focused {
                let (escape, left, right) = ctx.input(|input| {
                    (
                        input.key_pressed(egui::Key::Escape),
                        input.key_pressed(egui::Key::ArrowLeft),
                        input.key_pressed(egui::Key::ArrowRight),
                    )
                });
                if escape {
                    result.close = true;
                }
                if matches!(window.page, SummaryPage::Attackers | SummaryPage::SystemForces) {
                    if let Some(category) = stepped(window.category, left, right) {
                        result.category = Some(category);
                    }
                }
            }
        });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rebellion_core::assault::AssaultOutcome;
    use rebellion_core::dat::{ExplorationStatus, SectorGroup};
    use rebellion_core::ids::DatId;
    use rebellion_core::troop_transport::TroopTransportState;
    use rebellion_core::world::{ControlKind, Sector, System, TroopUnit};

    use crate::fleet_window::tests::PAINTED;

    fn world(control: ControlKind) -> (GameWorld, SystemKey) {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(0x8000_0001),
            name: "Sector".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let system = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "Hoth".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control,
        });
        let troop = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(troop);
        (world, system)
    }

    fn report(world: &GameWorld, system: SystemKey) -> AssaultReport {
        let mut report = AssaultReport::start(
            world,
            &TroopTransportState::new(),
            Faction::Empire,
            &[],
            system,
        )
        .unwrap();
        report.finish(world, &AssaultOutcome::default());
        report
    }

    /// TEXTSTRA RCDATA 0x7090..0x7094 and the sides' adjectives
    /// (0x4d70/0x4d71) as shipped.
    fn texts() -> HashMap<u16, String> {
        [
            (0x7090, "Assault on {1:1}"),
            (
                0x7092,
                "{2:4} Troops have defended {3:1} from an {1:4} assault.",
            ),
            (
                0x7094,
                "The neutral system {2:1} has repulsed an attack by {1:4} troops.",
            ),
            (0x4d70, "Alliance"),
            (0x4d71, "Imperial"),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect()
    }

    #[test]
    fn a_held_system_names_the_attacker_the_defender_and_the_system() {
        // FUN_00496770: 0x7090 with the system; 0x7092 with the attacker,
        // the defender and the system.
        let (world, system) = world(ControlKind::Controlled(Faction::Alliance));
        let (title, body) = summary_texts(&report(&world, system), &world, &texts());
        assert_eq!(title, "Assault on Hoth");
        assert_eq!(
            body,
            "Alliance Troops have defended Hoth from an Imperial assault."
        );
    }

    #[test]
    fn a_bombardment_names_the_system_in_its_title() {
        // FUN_00496770: 0x7098 "Orbital bombardment of {1:4}", a system's
        // kind 4 being its name (FUN_004f6030); 0x7099 for a held system.
        let (world, system) = world(ControlKind::Controlled(Faction::Alliance));
        let report = AssaultReport::start_bombardment(
            &world,
            &TroopTransportState::new(),
            Faction::Empire,
            &[],
            system,
        )
        .unwrap();
        let mut texts = texts();
        texts.insert(0x7098, "Orbital bombardment of {1:4}".into());
        texts.insert(
            0x7099,
            "{1:4} ships have conducted an orbital strike on the {2:4} system of {3:1}".into(),
        );
        let (title, body) = summary_texts(&report, &world, &texts);
        assert_eq!(title, "Orbital bombardment of Hoth");
        assert_eq!(
            body,
            "Imperial ships have conducted an orbital strike on the Alliance system of Hoth"
        );
        let mut state = AssaultSummaryState::default();
        state.open(&world, report, &texts, None);
        assert_eq!(
            state.report().unwrap().category,
            ReportCategory::CapitalShips
        );
    }

    #[test]
    fn a_neutral_system_names_the_attacker_and_the_system() {
        // FUN_00496770: a neutral system's body takes two parameters.
        let (world, system) = world(ControlKind::Uncontrolled);
        let (_, body) = summary_texts(&report(&world, system), &world, &texts());
        assert_eq!(
            body,
            "The neutral system Hoth has repulsed an attack by Imperial troops."
        );
    }

    #[test]
    fn the_forces_buttons_take_the_art_of_the_side_they_show() {
        // FUN_0044c630: 0x66 by +0x6c, 0x67 by +0x68 (neutral 0x29f0); the
        // Empire viewer's art is 10 on.
        let (world, system) = world(ControlKind::Uncontrolled);
        let report = report(&world, system);
        let art = |page, viewer| {
            let (normal, pressed, _) = rail_art(page, &report, viewer);
            (normal, pressed)
        };
        assert_eq!(art(SummaryPage::Summary, 0), (0x29e8, 0x29e9));
        assert_eq!(art(SummaryPage::Attackers, 0), (0x29ec, 0x29ed));
        assert_eq!(art(SummaryPage::Attackers, 1), (0x29f6, 0x29f7));
        assert_eq!(art(SummaryPage::SystemForces, 0), (0x29f0, 0x29f1));
        assert_eq!(art(SummaryPage::SystemForces, 1), (0x29fa, 0x29fb));
        assert_eq!(art(SummaryPage::Display, 0), (0x2a27, 0x2a28));
        assert_eq!(art(SummaryPage::Display, 1), (0x2918, 0x2919));
        assert_eq!(forces_art(Some(Faction::Alliance), 0), (0x29ea, 0x29eb));
        assert_eq!(
            rail_art(SummaryPage::Attackers, &report, 0).2,
            "Imperial Forces"
        );
    }

    #[test]
    fn the_category_buttons_take_each_sides_art() {
        // FUN_0044c630: Defense Facilities shares 0x2866/0x2865.
        assert_eq!(
            category_art(ReportCategory::CapitalShips, 0),
            (0x286c, 0x286b)
        );
        assert_eq!(
            category_art(ReportCategory::CapitalShips, 1),
            (0x2878, 0x2877)
        );
        assert_eq!(category_art(ReportCategory::Fighters, 0), (0x2872, 0x2871));
        assert_eq!(category_art(ReportCategory::Fighters, 1), (0x287c, 0x287b));
        assert_eq!(
            category_art(ReportCategory::Manufacturing, 0),
            (0x2868, 0x2867)
        );
        assert_eq!(
            category_art(ReportCategory::Manufacturing, 1),
            (0x2874, 0x2873)
        );
        assert_eq!(category_art(ReportCategory::Defense, 0), (0x2866, 0x2865));
        assert_eq!(category_art(ReportCategory::Defense, 1), (0x2866, 0x2865));
        assert_eq!(category_art(ReportCategory::Troops, 0), (0x2870, 0x286f));
        assert_eq!(category_art(ReportCategory::Troops, 1), (0x287a, 0x2879));
        assert_eq!(category_art(ReportCategory::Personnel, 0), (0x286a, 0x2869));
        assert_eq!(category_art(ReportCategory::Personnel, 1), (0x2876, 0x2875));
    }

    #[test]
    fn a_window_opens_on_the_summary_with_the_regiments_and_follows_its_buttons() {
        // FUN_0044c630: 0x65 pressed, +0x180 0x9b after an assault;
        // FUN_0044e500 fills two columns, three for Personnel.
        let (world, system) = world(ControlKind::Controlled(Faction::Alliance));
        let mut state = AssaultSummaryState::default();
        state.open(&world, report(&world, system), &texts(), Some(7));
        let shown = state.report().unwrap();
        assert_eq!(shown.page, SummaryPage::Summary);
        assert_eq!(shown.category, ReportCategory::Troops);
        assert_eq!(shown.title, "Assault on Hoth");
        assert_eq!(shown.picture, 0x2b99);
        assert!(shown.columns.is_empty());
        assert!(state.show_page(SummaryPage::SystemForces));
        assert_eq!(state.report().unwrap().columns.len(), 2);
        assert!(state.show_category(ReportCategory::Personnel));
        assert_eq!(state.report().unwrap().columns.len(), 3);
        assert_eq!(state.window_count(), 1);
        state.clear();
        assert_eq!(state.window_count(), 0);
        assert!(!state.show_page(SummaryPage::Display));
        assert!(!state.show_category(ReportCategory::Troops));
    }

    #[test]
    fn the_arrows_step_through_the_categories_and_stop_at_the_ends() {
        // FUN_0044e180: 0x25 (Left) and 0x27 (Right).
        use ReportCategory::{CapitalShips, Fighters, Personnel, Troops};
        assert_eq!(stepped(Troops, true, false), Some(ReportCategory::Defense));
        assert_eq!(stepped(Troops, false, true), Some(Personnel));
        assert_eq!(stepped(CapitalShips, true, false), None);
        assert_eq!(stepped(CapitalShips, false, true), Some(Fighters));
        assert_eq!(stepped(Personnel, false, true), None);
        assert_eq!(stepped(Personnel, true, false), Some(Troops));
        assert_eq!(stepped(Troops, false, false), None);
    }

    #[test]
    fn the_wheel_scrolls_a_row_while_rows_remain() {
        // Two rows of 70 fit in the list's 166.
        assert_eq!(visible_rows(ReportCategory::Troops), 2);
        assert_eq!(visible_rows(ReportCategory::Personnel), 3);
        assert_eq!(scrolled(0, -1.0, 3, 2), Some(1));
        assert_eq!(scrolled(1, -1.0, 3, 2), None);
        assert_eq!(scrolled(0, -1.0, 2, 2), None);
        assert_eq!(scrolled(1, 1.0, 3, 2), Some(0));
        assert_eq!(scrolled(0, 1.0, 3, 2), None);
        assert_eq!(scrolled(1, 0.0, 3, 2), None);
        assert_eq!(scrolled(0, 0.0, 3, 2), None);
    }

    #[test]
    fn the_cells_and_heads_follow_the_columns() {
        // FUN_0044e500: 165 by 70 cells, 110 by 50 for Personnel, from
        // the list's (37, 136); FUN_0044c630's heads.
        assert_eq!(cell_origin(ReportCategory::Troops, 1, 2), (202.0, 276.0));
        assert_eq!(cell_origin(ReportCategory::Personnel, 2, 1), (257.0, 186.0));
        assert_eq!(heads(ReportCategory::Troops), &HEADS_TWO);
        assert_eq!(heads(ReportCategory::Personnel), &HEADS_THREE);
        assert_eq!(empty_label(0, 2), "None");
        assert_eq!(empty_label(1, 2), "No Casualties");
        assert_eq!(empty_label(1, 3), "None");
        assert_eq!(empty_label(2, 3), "No Casualties");
    }

    #[test]
    fn a_frame_result_closes_or_updates_its_window() {
        // FUN_0044df90: Close (10) marks the message; 0xb/0xc go and close.
        let (world, system) = world(ControlKind::Controlled(Faction::Alliance));
        let mut state = AssaultSummaryState::default();
        state.open(&world, report(&world, system), &texts(), Some(3));
        state.open(&world, report(&world, system), &texts(), None);
        let first = state.windows[0].serial;
        let second = state.windows[1].serial;
        assert_ne!(first, second);
        let update = DrawResult {
            page: Some(SummaryPage::Attackers),
            category: Some(ReportCategory::Personnel),
            scroll: Some(2),
            ..DrawResult::default()
        };
        assert!(state.apply(second, &update).is_empty());
        let open = &state.windows[1];
        assert_eq!(
            (open.page, open.category, open.scroll),
            (SummaryPage::Attackers, ReportCategory::Personnel, 2)
        );
        // A new category starts at the top; the same one keeps the scroll.
        let same = DrawResult {
            category: Some(ReportCategory::Personnel),
            ..DrawResult::default()
        };
        state.apply(second, &same);
        assert_eq!(state.windows[1].scroll, 2);
        let other = DrawResult {
            category: Some(ReportCategory::Troops),
            ..DrawResult::default()
        };
        state.apply(second, &other);
        assert_eq!(state.windows[1].scroll, 0);
        let goto = DrawResult {
            action: Some(AssaultSummaryAction::GotoFleets(system)),
            ..DrawResult::default()
        };
        assert_eq!(
            state.apply(second, &goto),
            [
                AssaultSummaryAction::GotoFleets(system),
                AssaultSummaryAction::Closed(None)
            ]
        );
        let close = DrawResult {
            close: true,
            ..DrawResult::default()
        };
        assert_eq!(
            state.apply(first, &close),
            [AssaultSummaryAction::Closed(Some(3))]
        );
        assert_eq!(state.window_count(), 0);
        assert!(state.apply(first, &close).is_empty());
    }

    #[test]
    fn the_viewer_and_the_attacker_pick_the_art_and_the_colour() {
        // FUN_0044c630: the viewer's side (+0x118 +0x9c) picks the art; the
        // attacker (+0x6c) the text colour, 1 red, 2 green, else yellow.
        assert_eq!(side_index(CockpitFaction::Alliance), 0);
        assert_eq!(side_index(CockpitFaction::Empire), 1);
        assert_eq!(
            side_color(Some(Faction::Alliance)),
            egui::Color32::from_rgb(255, 0, 0)
        );
        assert_eq!(
            side_color(Some(Faction::Empire)),
            egui::Color32::from_rgb(0, 255, 0)
        );
        assert_eq!(side_color(None), egui::Color32::from_rgb(255, 255, 0));
    }

    struct Frame {
        actions: Vec<AssaultSummaryAction>,
        texts: Vec<(String, egui::Pos2, egui::Color32)>,
        /// Each text's font size, its size and its wrap width.
        metrics: Vec<(String, f32, egui::Vec2, f32)>,
        painted: Vec<(u32, egui::Pos2)>,
    }

    /// The 640 by 480 canvas at twice its size.
    fn layout() -> CockpitLayout {
        crate::cockpit::CockpitState::new(CockpitFaction::Alliance).layout_for(1280.0, 960.0)
    }

    /// One egui frame per events list, at 640 by 480; the last frame's
    /// texts are kept.
    fn run(state: &mut AssaultSummaryState, frames: Vec<Vec<egui::Event>>) -> Frame {
        let ctx = egui::Context::default();
        let mut cache = BmpCache::new();
        let mut frame = Frame {
            actions: Vec::new(),
            texts: Vec::new(),
            metrics: Vec::new(),
            painted: Vec::new(),
        };
        // egui hides a new area for its first frame.
        let frames = [vec![], vec![]].into_iter().chain(frames);
        for (index, events) in frames.enumerate() {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 960.0),
                )),
                time: Some(index as f64 * 0.05),
                events,
                ..Default::default()
            };
            PAINTED.with(|painted| painted.borrow_mut().clear());
            let output = ctx.run(input, |ctx| {
                frame.actions.extend(draw_assault_summaries(
                    ctx,
                    state,
                    CockpitFaction::Alliance,
                    layout(),
                    &mut cache,
                ));
            });
            frame.painted = PAINTED.with(|painted| painted.take());
            frame.texts.clear();
            frame.metrics.clear();
            for clipped in output.shapes {
                if let egui::Shape::Text(text) = clipped.shape {
                    let format = &text.galley.job.sections[0].format;
                    let name = text.galley.text().to_owned();
                    frame.metrics.push((
                        name.clone(),
                        format.font_id.size,
                        text.galley.size(),
                        text.galley.job.wrap.max_width,
                    ));
                    frame.texts.push((name, text.pos, format.color));
                }
            }
        }
        frame
    }

    fn at(x: f32, y: f32) -> egui::Pos2 {
        window_rect(layout()).min + egui::vec2(x, y) * layout().scale
    }

    fn painted(frame: &Frame, id: u32) -> Vec<egui::Pos2> {
        frame
            .painted
            .iter()
            .filter(|(each, _)| *each == id)
            .map(|(_, pos)| *pos)
            .collect()
    }

    fn metrics(frame: &Frame, text: &str) -> (f32, egui::Vec2, f32) {
        frame
            .metrics
            .iter()
            .find(|(each, ..)| each == text)
            .map(|(_, font, size, wrap)| (*font, *size, *wrap))
            .unwrap()
    }

    fn click(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
        let button = |pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        vec![
            vec![egui::Event::PointerMoved(point)],
            vec![button(true)],
            vec![button(false)],
            vec![],
        ]
    }

    fn key(key: egui::Key) -> Vec<Vec<egui::Event>> {
        vec![vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }]]
    }

    /// An Imperial assault on an Alliance system with three regiments.
    fn opened(regiments: usize) -> (AssaultSummaryState, SystemKey) {
        let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
        for _ in 1..regiments {
            let troop = world.troops.insert(TroopUnit {
                class_dat_id: DatId::new(0x1000_0001),
                is_alliance: true,
                regiment_strength: 100,
            });
            world.systems[system].ground_units.push(troop);
        }
        let mut state = AssaultSummaryState::default();
        state.open(&world, report(&world, system), &texts(), Some(9));
        (state, system)
    }

    fn has(frame: &Frame, text: &str) -> Option<(egui::Pos2, egui::Color32)> {
        frame
            .texts
            .iter()
            .find(|(each, ..)| each == text)
            .map(|(_, pos, color)| (*pos, *color))
    }

    #[test]
    fn the_summary_shows_the_title_and_the_outcome_in_the_attackers_colour() {
        // FUN_0044c630: the title at (12, 13), 400 wide and centred; the
        // outcome at (25, 225) over its shadow; the Empire attacks (green).
        let (mut state, _) = opened(1);
        let frame = run(&mut state, vec![vec![]]);
        let (title, color) = has(&frame, "Assault on Hoth").unwrap();
        assert_eq!(color, egui::Color32::from_rgb(0, 255, 0));
        assert_eq!(layout().scale, 2.0);
        let (font, size, wrap) = metrics(&frame, "Assault on Hoth");
        assert_eq!((font, wrap), (28.0, 800.0));
        let centred = at(12.0, 0.0).x + (800.0 - size.x) / 2.0;
        assert_eq!(title, egui::pos2(centred, at(0.0, 13.0).y));
        let body = "Alliance Troops have defended Hoth from an Imperial assault.";
        let (font, _, wrap) = metrics(&frame, body);
        assert_eq!((font, wrap), (22.0, 700.0));
        // The frame and the attacker's picture (FUN_0044c630).
        assert_eq!(painted(&frame, FRAME[0]), [at(0.0, 0.0)]);
        assert_eq!(painted(&frame, 0x2b99), [at(12.0, 13.0)]);
        // The open page's rail button is down, the others up.
        assert_eq!(painted(&frame, 0x29e9).len(), 1);
        assert_eq!(painted(&frame, 0x29e8).len(), 0);
        assert_eq!(painted(&frame, 0x29ec), [at(418.0, 141.0)]);
        assert_eq!(painted(&frame, 0x29ed).len(), 0);
        let body = "Alliance Troops have defended Hoth from an Imperial assault.";
        let shown: Vec<_> = frame
            .texts
            .iter()
            .filter(|(text, ..)| text == body)
            .collect();
        assert_eq!(shown.len(), 2, "the outcome and its shadow");
        assert_eq!(shown[0].2, egui::Color32::BLACK);
        assert_eq!(shown[0].1, at(26.0, 226.0));
        assert_eq!(shown[1].1, at(25.0, 225.0));
        assert!(frame.actions.is_empty());
    }

    #[test]
    fn the_rail_shows_each_sides_forces_with_its_columns() {
        // FUN_0044e3d0: 0x66 at (418, 141) shows the attacker's forces,
        // 0x67 at (418, 196) the system's; FUN_0044e500 fills the columns.
        let (mut state, _) = opened(3);
        let frame = run(&mut state, click(at(438.0, 160.0)));
        assert_eq!(state.report().unwrap().page, SummaryPage::Attackers);
        assert!(has(&frame, "Operational").is_some());
        assert_eq!(
            has(&frame, "Destroyed").unwrap().0.y,
            has(&frame, "Operational").unwrap().0.y
        );
        // The attacker had no regiments: both columns are empty.
        assert_eq!(has(&frame, "None").unwrap().0.y, at(0.0, 140.0).y);
        assert!(has(&frame, "No Casualties").is_some());
        let (font, _, wrap) = metrics(&frame, "None");
        assert_eq!((font, wrap), (20.0, 330.0));
        assert_eq!(painted(&frame, FORCES_TWO_COLUMNS), [at(12.0, 13.0)]);
        assert_eq!(painted(&frame, FORCES_THREE_COLUMNS).len(), 0);
        // The shown category's button is down (Trooper Regiments).
        assert_eq!(painted(&frame, 0x286f), [at(36.0 + 248.0, 59.0)]);
        assert_eq!(painted(&frame, 0x2870).len(), 0);
        assert_eq!(painted(&frame, 0x286c), [at(36.0, 59.0)]);
        let frame = run(&mut state, click(at(438.0, 215.0)));
        assert_eq!(state.report().unwrap().page, SummaryPage::SystemForces);
        // No picture is loaded here, so each regiment shows its name.
        let names = frame
            .texts
            .iter()
            .filter(|(text, ..)| text.is_empty())
            .count();
        // Two rows fit; the third is drawn under the list's clip.
        assert_eq!(names, 3);
        let (font, _, wrap) = metrics(&frame, "");
        assert_eq!((font, wrap), (18.0, 330.0));
        assert!(has(&frame, "None").is_none());
        // The wheel scrolls the third into view.
        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -1.0),
            modifiers: egui::Modifiers::default(),
        };
        let over = at(100.0, 200.0);
        run(
            &mut state,
            vec![vec![egui::Event::PointerMoved(over)], vec![wheel]],
        );
        assert_eq!(state.windows[0].scroll, 1);
        assert!(frame.actions.is_empty());
    }

    #[test]
    fn the_category_buttons_and_the_arrows_pick_the_category() {
        // The bar at (36, 59), 62 apart; FUN_0044e180's arrows.
        let (mut state, _) = opened(1);
        state.show_page(SummaryPage::SystemForces);
        run(&mut state, click(at(36.0 + 62.0 + 20.0, 79.0)));
        assert_eq!(state.report().unwrap().category, ReportCategory::Fighters);
        run(&mut state, key(egui::Key::ArrowLeft));
        assert_eq!(
            state.report().unwrap().category,
            ReportCategory::CapitalShips
        );
        run(&mut state, key(egui::Key::ArrowRight));
        assert_eq!(state.report().unwrap().category, ReportCategory::Fighters);
        state.show_category(ReportCategory::Personnel);
        let frame = run(&mut state, vec![vec![]]);
        for head in ["Survivors", "Captured", "Killed"] {
            assert!(has(&frame, head).is_some(), "{head}");
        }
        assert_eq!(painted(&frame, FORCES_THREE_COLUMNS), [at(12.0, 13.0)]);
        // The arrows do nothing on the Summary page.
        state.show_page(SummaryPage::Summary);
        run(&mut state, key(egui::Key::ArrowLeft));
        assert_eq!(state.report().unwrap().category, ReportCategory::Personnel);
    }

    #[test]
    fn display_goes_to_the_system_and_closes() {
        // 0x68 at (418, 250); 0xb at (29, 182) names the system, 0xc at
        // (226, 182) "Fleet"; each goes and closes (FUN_0044df90).
        let (mut state, system) = opened(1);
        let frame = run(&mut state, click(at(438.0, 270.0)));
        assert_eq!(state.report().unwrap().page, SummaryPage::Display);
        assert_eq!(
            has(&frame, "Hoth").unwrap().0.y,
            at(0.0, 182.0 + 48.0 - 12.0).y
        );
        assert!(has(&frame, "Fleet").is_some());
        // Held, a destination draws its selected frame over it; a hover
        // alone leaves Close up.
        let hold = at(100.0, 230.0);
        let frame = run(
            &mut state,
            vec![
                vec![egui::Event::PointerMoved(hold)],
                vec![egui::Event::PointerButton {
                    pos: hold,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                }],
            ],
        );
        assert_eq!(painted(&frame, 10767), [at(29.0, 182.0)]);
        assert!(frame.actions.is_empty());
        let frame = run(
            &mut state,
            vec![vec![egui::Event::PointerMoved(at(438.0, 40.0))], vec![]],
        );
        assert_eq!(painted(&frame, 10370), [at(423.0, 25.0)]);
        assert_eq!(painted(&frame, 10371).len(), 0);
        let frame = run(&mut state, click(at(100.0, 230.0)));
        assert_eq!(
            frame.actions,
            [
                AssaultSummaryAction::GotoSystem(system),
                AssaultSummaryAction::Closed(Some(9))
            ]
        );
        let (mut state, system) = opened(1);
        state.show_page(SummaryPage::Display);
        let frame = run(&mut state, click(at(300.0, 230.0)));
        assert_eq!(frame.actions[0], AssaultSummaryAction::GotoFleets(system));
    }

    #[test]
    fn close_and_escape_close_the_window() {
        // Close (10) at (423, 25), and Escape (FUN_0044e180).
        let (mut state, _) = opened(1);
        let frame = run(&mut state, click(at(438.0, 40.0)));
        assert_eq!(frame.actions, [AssaultSummaryAction::Closed(Some(9))]);
        let (mut state, _) = opened(1);
        let frame = run(&mut state, key(egui::Key::Escape));
        assert_eq!(frame.actions, [AssaultSummaryAction::Closed(Some(9))]);
        assert_eq!(state.window_count(), 0);
    }

    #[test]
    fn only_the_top_window_takes_the_keys() {
        let (mut state, _) = opened(1);
        let (world, system) = world(ControlKind::Controlled(Faction::Alliance));
        state.open(&world, report(&world, system), &texts(), None);
        let frame = run(&mut state, key(egui::Key::Escape));
        assert_eq!(frame.actions, [AssaultSummaryAction::Closed(None)]);
        assert_eq!(state.window_count(), 1);
    }

    #[test]
    fn the_rows_draw_a_crafts_portrait_and_a_characters_mini() {
        // FUN_0044edb0 -> FUN_0042c3b0(.., param_3 == 0, 1): the portrait
        // (the mini less 0x4000) outside Personnel, the mini in it.
        let unit =
            |category, raw: u32| ReportUnit::new(category, DatId::new(raw), String::new(), true);
        let troop = unit(ReportCategory::Troops, 0x1000_0001);
        let mini = troop_mini(troop.dat_id).unwrap().0;
        assert_eq!(unit_picture(&troop), Some(mini - 0x4000));
        let leia = unit(ReportCategory::Personnel, 0x3000_0001);
        assert_eq!(
            unit_picture(&leia),
            character_mini_resource_id(leia.dat_id, true)
        );
    }
}
