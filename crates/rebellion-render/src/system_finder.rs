//! The Planetary System Finder (window type `0x14`, `FUN_00460090`, vtable
//! `0x0065a000`; `ghidra/notes/system-finder.md`).
//!
//! A 470 by 330 window the cockpit's System Finder control (`0x12d`) or F2
//! opens (`FUN_0042a000`), a near clone of the Fleet Finder. It lists the
//! galaxy's systems under five tabs (all, Rebel, Imperial, neutral,
//! unexplored), sorted by name (`FUN_004611d0`). Typing a name picks the row
//! it best begins (`FUN_00609650`); Display, Enter or a double click opens
//! the window of the system's sector (`FUN_00429ce0`) and closes the
//! Finder.

use egui_macroquad::egui;
use rebellion_core::dat::{ExplorationStatus, Faction};
use rebellion_core::ids::SystemKey;
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::fleet_window::paint_native;
use crate::mission_dialog::{button, galaxy_centered_rect};
use crate::system_window::{logical_rect, rect_contains};

pub const SYSTEM_FINDER_WIDTH: f32 = 470.0;
pub const SYSTEM_FINDER_HEIGHT: f32 = 330.0;

// STRATEGY bitmaps by side, Alliance then Empire (FUN_00460270): one panel
// for both sides, unlike the Fleet Finder's.
const BACKGROUND: [u32; 2] = [10335, 10336];
const PANEL: u32 = 10522;
const RAIL: [u32; 2] = [10584, 10588];
const PANEL_AT: (f32, f32) = (12.0, 13.0);
const RAIL_AT: (f32, f32) = (412.0, 0.0);

#[derive(Debug, Clone, Copy)]
struct Control {
    rect: (f32, f32, f32, f32),
    art: (u32, u32),
}

/// Close (`200`) and Display (`0xc9`) by side (`FUN_00460270`).
const CLOSE: [Control; 2] = [
    Control {
        rect: (427.0, 25.0, 32.0, 31.0),
        art: (10514, 10515),
    },
    Control {
        rect: (426.0, 21.0, 44.0, 41.0),
        art: (10516, 10517),
    },
];
const DISPLAY: [Control; 2] = [
    Control {
        rect: (427.0, 93.0, 32.0, 31.0),
        art: (10518, 10519),
    },
    Control {
        rect: (426.0, 89.0, 44.0, 41.0),
        art: (10520, 10521),
    },
];

// Window rectangles (x, y, width, height), FUN_00460270.
const TITLE_AT: (f32, f32) = (36.0, 14.0);
const NAME_LABEL_AT: (f32, f32) = (36.0, 48.0);
/// The name box (`0xcb`), 18 high (the Fleet Finder's is 16).
const NAME_BOX: (f32, f32, f32, f32) = (143.0, 45.0, 250.0, 18.0);
const TAB_LABEL_AT: (f32, f32) = (40.0, 119.0);
const LIST: (f32, f32, f32, f32) = (36.0, 138.0, 350.0, 165.0);
/// The tab group (`100`) at (36, 78): each tab 49 by 42, 52 apart.
const TABS_AT: (f32, f32) = (36.0, 78.0);
const TAB_SIZE: (f32, f32) = (49.0, 42.0);
const TAB_PITCH: f32 = 52.0;
const ROW_HEIGHT: f32 = 20.0;
const ROW_PIXELS: usize = 20;
const LIST_PIXELS: usize = 165;
const BAR_WIDTH: f32 = 12.0;
/// The list's text at x 5, grey `0x787878` and the chosen row white
/// (`FUN_00607ea0`), in font 10, as the Fleet Finder's.
const ROW_TEXT_X: f32 = 5.0;
const ROW_COLOR: egui::Color32 = egui::Color32::from_rgb(0x78, 0x78, 0x78);
const LIST_STEP: f32 = 10.0;

/// TEXTSTRA `0x1865` and `0x1866`.
const TITLE: &str = "Planetary System Finder";
const NAME_LABEL: &str = "System Name";

/// The tab (`FUN_004611d0`'s 1..5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SystemTab {
    #[default]
    All,
    Rebel,
    Imperial,
    Neutral,
    Unexplored,
}

impl SystemTab {
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::Rebel,
        Self::Imperial,
        Self::Neutral,
        Self::Unexplored,
    ];

    const fn index(self) -> usize {
        match self {
            Self::All => 0,
            Self::Rebel => 1,
            Self::Imperial => 2,
            Self::Neutral => 3,
            Self::Unexplored => 4,
        }
    }

    /// Normal and pressed art: 10500/10501, 10502/10503, 10505/10506,
    /// 10508/10509, 10511/10512.
    const fn art(self) -> (u32, u32) {
        match self {
            Self::All => (10500, 10501),
            Self::Rebel => (10502, 10503),
            Self::Imperial => (10505, 10506),
            Self::Neutral => (10508, 10509),
            Self::Unexplored => (10511, 10512),
        }
    }

    /// TEXTSTRA `0x1867..0x1869`, `0x1870`, `0x1871`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All Systems",
            Self::Rebel => "Rebel Systems",
            Self::Imperial => "Imperial Systems",
            Self::Neutral => "Neutral Systems",
            Self::Unexplored => "Unexplored Systems",
        }
    }

    /// `FUN_004611d0`'s filter: the side bits (`+0x24 & 0xc0`: `0x40`
    /// Rebel, `0x80` Imperial, `0xc0` no side) and the explored bit
    /// (`+0x88 & 2`). The side tabs do not ask for exploration; Neutral
    /// does; Unexplored takes any side.
    ///
    /// hyp: the port's one exploration status and controlling side stand for
    /// what the viewing side knows of the system.
    fn lists(self, holder: Option<Faction>, explored: bool) -> bool {
        match self {
            Self::All => true,
            Self::Rebel => holder == Some(Faction::Alliance),
            Self::Imperial => holder == Some(Faction::Empire),
            Self::Neutral => {
                !matches!(holder, Some(Faction::Alliance | Faction::Empire)) && explored
            }
            Self::Unexplored => !explored,
        }
    }
}

/// One list row: a system and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemRow {
    pub system: SystemKey,
    pub name: String,
}

/// The rows `FUN_004611d0` lists for `tab`, in name order ignoring case
/// (`FUN_0060a790(2)`, `_stricmp`), ties in walk order. Every system is
/// walked (families `0x90..0x97`); no existence test, unlike the Fleet
/// Finder's.
#[must_use]
pub fn rows(world: &GameWorld, tab: SystemTab) -> Vec<SystemRow> {
    let mut rows: Vec<SystemRow> = world
        .systems
        .iter()
        .filter(|(_, system)| {
            tab.lists(
                system.control.faction(),
                system.exploration_status == ExplorationStatus::Explored,
            )
        })
        .map(|(key, system)| SystemRow {
            system: key,
            name: system.name.clone(),
        })
        .collect();
    rows.sort_by_cached_key(|row| row.name.to_ascii_lowercase());
    rows
}

/// The row a typed name chooses (`FUN_00609650`): the first with the
/// longest prefix in common with `text`, ignoring case; the first row when
/// none shares the first letter.
#[must_use]
pub fn prefix_choice(rows: &[SystemRow], text: &str) -> Option<usize> {
    crate::list_prefix::best_prefix_row(rows.iter().map(|row| row.name.as_str()), text)
}

/// What the Finder asks of the galaxy view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemFinderAction {
    /// Display, Enter or a double click: `FUN_00429ce0` opens (or finds) the
    /// window of the system's sector, then the Finder closes (`+0x30`). The
    /// system is not selected.
    OpenSector(SystemKey),
}

#[derive(Debug, Clone)]
struct OpenFinder {
    faction: CockpitFaction,
    tab: SystemTab,
    /// The name box's text (`0xcb`).
    name: String,
    /// `+0x160`: the chosen system. A tab change leaves it when the new
    /// list chooses nothing.
    chosen: Option<SystemKey>,
    scroll: usize,
    scroll_hold: Option<crate::list_scroll_bar::Hold>,
    focus_name: bool,
    last_clicked: Option<(usize, f64)>,
    /// A tab's command (`100`) is due: the name box's text chooses a row
    /// once the list is built.
    rematch: bool,
}

/// What the open Finder shows, for the interface fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemFinderReport {
    pub tab: SystemTab,
    pub name: String,
    pub rows: Vec<SystemRow>,
    pub chosen: Option<usize>,
    pub first_row: usize,
}

/// A Finder control the interface fixture locates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemFinderControl {
    Tab(SystemTab),
    Close,
    Display,
    NameBox,
    Row(usize),
}

/// The System Finder, one at most (`FUN_0042a000`).
#[derive(Debug, Clone, Default)]
pub struct SystemFinderState {
    window: Option<OpenFinder>,
}

const fn side(faction: CockpitFaction) -> usize {
    match faction {
        CockpitFaction::Alliance => 0,
        CockpitFaction::Empire => 1,
    }
}

fn tab_rect(tab: SystemTab) -> (f32, f32, f32, f32) {
    #[expect(clippy::cast_precision_loss, reason = "Five tabs.")]
    let x = TABS_AT.0 + TAB_PITCH * tab.index() as f32;
    (x, TABS_AT.1, TAB_SIZE.0, TAB_SIZE.1)
}

/// `FUN_00606980` centres it in the galaxy view, as the Fleet Finder.
fn window_rect(layout: CockpitLayout) -> egui::Rect {
    galaxy_centered_rect(layout, SYSTEM_FINDER_WIDTH, SYSTEM_FINDER_HEIGHT)
}

fn at(frame: egui::Rect, scale: f32, (x, y, w, h): (f32, f32, f32, f32)) -> egui::Rect {
    logical_rect(frame, scale, x, y, w, h)
}

impl SystemFinderState {
    /// Open the Finder on its All tab with nothing chosen and the focus in
    /// the name box (`FUN_00460270` ends with `FUN_004611d0(1)`). An open
    /// Finder stays as it is.
    pub fn open(&mut self, faction: CockpitFaction) -> bool {
        if self.window.is_some() {
            return false;
        }
        self.window = Some(OpenFinder {
            faction,
            tab: SystemTab::All,
            name: String::new(),
            chosen: None,
            scroll: 0,
            scroll_hold: None,
            focus_name: true,
            last_clicked: None,
            rematch: false,
        });
        true
    }

    pub fn close(&mut self) {
        self.window = None;
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.window.is_some()
    }

    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        self.window.is_some() && rect_contains(window_rect(layout), egui::pos2(point.0, point.1))
    }

    #[must_use]
    pub fn report(&self, world: &GameWorld) -> Option<SystemFinderReport> {
        let window = self.window.as_ref()?;
        let rows = rows(world, window.tab);
        let chosen = window
            .chosen
            .and_then(|system| rows.iter().position(|row| row.system == system));
        Some(SystemFinderReport {
            tab: window.tab,
            name: window.name.clone(),
            rows,
            chosen,
            first_row: window.scroll / ROW_PIXELS,
        })
    }

    /// Where a control lies on screen, for the interface fixture.
    #[must_use]
    pub fn control_screen_rect(
        &self,
        layout: CockpitLayout,
        control: SystemFinderControl,
    ) -> Option<egui::Rect> {
        let window = self.window.as_ref()?;
        let rect = match control {
            SystemFinderControl::Tab(tab) => tab_rect(tab),
            SystemFinderControl::Close => CLOSE[side(window.faction)].rect,
            SystemFinderControl::Display => DISPLAY[side(window.faction)].rect,
            SystemFinderControl::NameBox => NAME_BOX,
            SystemFinderControl::Row(row) => {
                let top = row * ROW_PIXELS;
                (top >= window.scroll && top + ROW_PIXELS <= window.scroll + LIST_PIXELS)
                    .then_some(())?;
                #[expect(clippy::cast_precision_loss, reason = "A list's pixels.")]
                let y = LIST.1 + (top - window.scroll) as f32;
                (LIST.0, y, LIST.2 - BAR_WIDTH, ROW_HEIGHT)
            }
        };
        Some(at(window_rect(layout), layout.scale, rect))
    }

    /// The window's screen rectangle while it is open.
    #[must_use]
    pub fn screen_rect(&self, layout: CockpitLayout) -> Option<egui::Rect> {
        self.window.as_ref().map(|_| window_rect(layout))
    }

    /// Show `tab` (command `100`, `FUN_00460d10`): the list refills
    /// (`FUN_004611d0`), then the name box's text chooses a row
    /// (`FUN_00609650`). The chosen system stays until a row is chosen.
    pub fn set_tab(&mut self, tab: SystemTab) {
        if let Some(window) = &mut self.window {
            if window.tab != tab {
                window.tab = tab;
                window.scroll = 0;
                window.last_clicked = None;
                window.rematch = true;
            }
            // port: typing after a tab click reaches the name box, as A0
            // shows ("cor" + Enter after the tabs, 2026-10-30); egui's
            // button would keep the focus.
            window.focus_name = true;
        }
    }
}

/// Draw the open Finder, if any, and report an open.
#[expect(
    clippy::too_many_lines,
    reason = "The window paints in the original's control order; splitting it hides that order."
)]
pub fn draw_system_finder(
    ctx: &egui::Context,
    world: &GameWorld,
    state: &mut SystemFinderState,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Option<SystemFinderAction> {
    let mut window = state.window.clone()?;
    let side = side(window.faction);
    let scale = layout.scale;
    let rect = window_rect(layout);
    let list = rows(world, window.tab);
    if window.rematch {
        window.rematch = false;
        if let Some(index) = prefix_choice(&list, &window.name) {
            window.chosen = Some(list[index].system);
            window.scroll = scrolled_to(window.scroll, index, list.len());
        }
        state.window.clone_from(&Some(window.clone()));
    }
    let chosen_row = window
        .chosen
        .and_then(|system| list.iter().position(|row| row.system == system));

    let mut close = false;
    let mut open = false;
    let mut double_clicked = None;
    let mut tab = None;
    let mut clicked_row = None;
    let mut name = window.name.clone();
    let mut name_changed = false;
    let mut enter = false;
    let mut wheel = 0.0;
    let mut scroll = window.scroll;
    let mut scroll_hold = window.scroll_hold;

    // Escape closes the Finder even from the name box (FUN_00461590,
    // FUN_00463360): read it before the box, which would take it.
    let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let id = egui::Id::new("original-system-finder");
    ctx.move_to_top(egui::LayerId::new(egui::Order::Tooltip, id));
    egui::Area::new(id)
        .fixed_pos(rect.min)
        .order(egui::Order::Tooltip)
        // The original's window shows whole at once.
        .fade_in(false)
        .show(ctx, |ui| {
            let (frame, _) = ui.allocate_exact_size(rect.size(), egui::Sense::click());
            let painter = ui.painter().with_clip_rect(frame);
            let paint = |cache: &mut BmpCache, id: u32, (x, y): (f32, f32)| {
                paint_native(
                    &painter,
                    ctx,
                    cache,
                    DllSource::Strategy,
                    id,
                    frame,
                    scale,
                    x,
                    y,
                );
            };
            painter.rect_filled(frame, 0.0, egui::Color32::BLACK);
            // The panel lies under the base, whose rounded corners, keyed
            // on blue, cover the panel's (A0 System, Fleet, Troop and
            // Personnel Finders).
            paint(cache, PANEL, PANEL_AT);
            paint(cache, BACKGROUND[side], (0.0, 0.0));
            paint(cache, RAIL[side], RAIL_AT);

            // The title, bold font 5 centred in its 350; the label font 4;
            // the tab's heading font 5; all white (0x2ffffff).
            let title_font = crate::theme::original_font(5, scale);
            let label_font = crate::theme::original_font(4, scale);
            let font = crate::theme::original_font(10, scale);
            painter.text(
                at(frame, scale, (TITLE_AT.0 + 175.0, TITLE_AT.1, 0.0, 0.0)).min,
                egui::Align2::CENTER_TOP,
                TITLE,
                title_font.clone(),
                egui::Color32::WHITE,
            );
            painter.text(
                at(frame, scale, (NAME_LABEL_AT.0, NAME_LABEL_AT.1, 0.0, 0.0)).min,
                egui::Align2::LEFT_TOP,
                NAME_LABEL,
                label_font,
                egui::Color32::WHITE,
            );
            painter.text(
                at(frame, scale, (TAB_LABEL_AT.0, TAB_LABEL_AT.1, 0.0, 0.0)).min,
                egui::Align2::LEFT_TOP,
                window.tab.label(),
                title_font,
                egui::Color32::WHITE,
            );

            for candidate in SystemTab::ALL {
                if button(
                    ui,
                    cache,
                    at(frame, scale, tab_rect(candidate)),
                    ("system-finder-tab", candidate.index()),
                    candidate.art(),
                    candidate == window.tab,
                    scale,
                ) {
                    tab = Some(candidate);
                }
            }
            if button(
                ui,
                cache,
                at(frame, scale, CLOSE[side].rect),
                "system-finder-close",
                CLOSE[side].art,
                false,
                scale,
            ) {
                close = true;
            }
            if button(
                ui,
                cache,
                at(frame, scale, DISPLAY[side].rect),
                "system-finder-display",
                DISPLAY[side].art,
                false,
                scale,
            ) {
                open = true;
            }

            let name_rect = at(frame, scale, NAME_BOX);
            // port: the caret stays inside the box, as a Windows edit
            // control's does; egui's runs a row past its bottom.
            let outer_clip = ui.clip_rect();
            ui.set_clip_rect(name_rect.intersect(outer_clip));
            let edit = ui.put(
                name_rect,
                egui::TextEdit::singleline(&mut name)
                    .id(egui::Id::new("original-system-finder-name"))
                    .frame(false)
                    .font(font.clone())
                    .text_color(egui::Color32::WHITE)
                    .desired_width(name_rect.width()),
            );
            ui.set_clip_rect(outer_clip);
            if window.focus_name {
                edit.request_focus();
            }
            name_changed = edit.changed();
            enter = edit.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));

            let list_rect = at(frame, scale, LIST);
            let list_painter = painter.with_clip_rect(list_rect);
            #[expect(clippy::cast_precision_loss, reason = "A list's pixels.")]
            let content = (list.len() * ROW_PIXELS) as f32;
            let bar_at = |first: f32| {
                crate::list_scroll_bar::ScrollBar::new(
                    crate::list_scroll_bar::RESULTS_ART,
                    LIST.0,
                    LIST.1,
                    LIST.2,
                    LIST.3,
                    content,
                    first,
                )
            };
            let (pointer, pressed, down, released, now) = ui.input(|input| {
                (
                    input.pointer.interact_pos(),
                    input.pointer.button_pressed(egui::PointerButton::Primary),
                    input.pointer.button_down(egui::PointerButton::Primary),
                    input.pointer.button_released(egui::PointerButton::Primary),
                    input.time,
                )
            });
            let pointer_at = pointer.map_or((-1.0, -1.0), |point| {
                (
                    (point.x - frame.min.x) / scale,
                    (point.y - frame.min.y) / scale,
                )
            });
            #[expect(clippy::cast_precision_loss, reason = "A list's pixels.")]
            let first = crate::list_scroll_bar::update(
                &bar_at(scroll as f32),
                &mut scroll_hold,
                crate::list_scroll_bar::Pointer {
                    at: pointer_at,
                    pressed,
                    down,
                    released,
                },
                LIST_STEP,
                now,
            );
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "Within the bar's range."
            )]
            {
                scroll = first.max(0.0) as usize;
            }
            #[expect(clippy::cast_precision_loss, reason = "A list's pixels.")]
            let offset_px = scroll as f32;
            for (offset, row) in list.iter().enumerate() {
                #[expect(clippy::cast_precision_loss, reason = "A few hundred rows.")]
                let y = LIST.1 + offset as f32 * ROW_HEIGHT - offset_px;
                if y + ROW_HEIGHT <= LIST.1 || y >= LIST.1 + LIST.3 {
                    continue;
                }
                let row_rect = at(frame, scale, (LIST.0, y, LIST.2 - BAR_WIDTH, ROW_HEIGHT))
                    .intersect(list_rect);
                let colour = if chosen_row == Some(offset) {
                    egui::Color32::WHITE
                } else {
                    ROW_COLOR
                };
                list_painter.text(
                    at(
                        frame,
                        scale,
                        (LIST.0 + ROW_TEXT_X, y + ROW_HEIGHT / 2.0, 0.0, 0.0),
                    )
                    .min,
                    egui::Align2::LEFT_CENTER,
                    &row.name,
                    font.clone(),
                    colour,
                );
                let response = ui.interact(
                    row_rect,
                    ui.id().with(("system-finder-row", offset)),
                    egui::Sense::click(),
                );
                if response.clicked() {
                    let (now, delay) = (
                        ui.input(|input| input.time),
                        ctx.options(|options| options.input_options.max_double_click_delay),
                    );
                    if window
                        .last_clicked
                        .is_some_and(|(last, at)| last == offset && now - at <= delay)
                    {
                        double_clicked = Some(row.system);
                    } else {
                        clicked_row = Some((offset, now));
                    }
                }
            }
            let bar = bar_at(offset_px);
            let held = scroll_hold.map(|hold| hold.part).filter(|part| {
                matches!(
                    part,
                    crate::list_scroll_bar::Part::Up | crate::list_scroll_bar::Part::Down
                ) && down
                    && bar.part_at(pointer_at) == Some(*part)
            });
            let (bar_x, bar_y, bar_w, bar_h) = bar.rect();
            let bar_painter =
                painter.with_clip_rect(at(frame, scale, (bar_x, bar_y, bar_w, bar_h)));
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
            let pointer = ui.input(|input| input.pointer.hover_pos());
            if pointer.is_some_and(|point| rect_contains(list_rect, point)) {
                wheel = ui.input(|input| input.raw_scroll_delta.y);
            }
        });

    let open_window = state.window.as_mut()?;
    open_window.focus_name = false;
    open_window.scroll = scroll;
    open_window.scroll_hold = scroll_hold;
    if name_changed {
        // FUN_00461000 0x408: a non-empty name chooses its best row; an
        // empty one, or none, clears the choice.
        open_window.name.clone_from(&name);
        let choice = (!name.is_empty())
            .then(|| prefix_choice(&list, &name))
            .flatten();
        open_window.chosen = choice.map(|index| list[index].system);
        if let Some(index) = choice {
            open_window.scroll = scrolled_to(open_window.scroll, index, list.len());
        }
    }
    if let Some((row, at)) = clicked_row {
        // 0x29b: the row's system is chosen and its name copied into the box.
        open_window.last_clicked = Some((row, at));
        open_window.chosen = Some(list[row].system);
        open_window.name.clone_from(&list[row].name);
    }
    // port: the original has no wheel; a notch moves a row.
    if wheel != 0.0 {
        open_window.scroll = if wheel > 0.0 {
            open_window.scroll.saturating_sub(ROW_PIXELS)
        } else {
            (open_window.scroll + ROW_PIXELS).min(max_scroll(list.len()))
        };
    }
    let chosen = open_window.chosen;
    if let Some(tab) = tab {
        state.set_tab(tab);
    }
    // Display (0xc9) and Enter (0x407) open the chosen system's sector; a
    // double click (0x309) its row's. With nothing chosen nothing happens.
    let target = double_clicked.or_else(|| (open || enter).then_some(chosen).flatten());
    if let Some(system) = target {
        state.close();
        return Some(SystemFinderAction::OpenSector(system));
    }
    if close || escape {
        state.close();
    }
    None
}

/// The scroll that shows `row` whole, moving the list as little as
/// possible, within the range of `rows` rows.
fn scrolled_to(scroll: usize, row: usize, rows: usize) -> usize {
    let top = row * ROW_PIXELS;
    let bottom = top + ROW_PIXELS;
    if top < scroll {
        top
    } else if bottom > scroll + LIST_PIXELS {
        (bottom - LIST_PIXELS).min(max_scroll(rows))
    } else {
        scroll
    }
}

#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "A few hundred rows of 20 pixels."
)]
fn max_scroll(rows: usize) -> usize {
    crate::list_scroll_bar::max_first((rows * ROW_PIXELS) as f32, LIST.3) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use rebellion_core::dat::SectorGroup;
    use rebellion_core::ids::DatId;
    use rebellion_core::world::{ControlKind, Sector, System};

    fn add(world: &mut GameWorld, name: &str, control: ControlKind, explored: bool) -> SystemKey {
        let sector = world.sectors.keys().next().unwrap_or_else(|| {
            world.sectors.insert(Sector {
                dat_id: DatId::new(0x8000_0001),
                name: "Sector".into(),
                group: SectorGroup::Core,
                x: 0,
                y: 0,
                systems: vec![],
            })
        });
        world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: name.into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: if explored {
                ExplorationStatus::Explored
            } else {
                ExplorationStatus::Unexplored
            },
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
            fighters: vec![],
            is_headquarters: false,
            is_destroyed: false,
            control,
        })
    }

    fn galaxy() -> GameWorld {
        let mut world = GameWorld::default();
        add(
            &mut world,
            "yavin",
            ControlKind::Controlled(Faction::Alliance),
            true,
        );
        add(
            &mut world,
            "Coruscant",
            ControlKind::Controlled(Faction::Empire),
            true,
        );
        add(&mut world, "Bespin", ControlKind::Uncontrolled, true);
        add(&mut world, "Adega", ControlKind::Uncontrolled, false);
        add(
            &mut world,
            "Kessel",
            ControlKind::Controlled(Faction::Alliance),
            false,
        );
        world
    }

    fn names(rows: &[SystemRow]) -> Vec<&str> {
        rows.iter().map(|row| row.name.as_str()).collect()
    }

    #[test]
    fn each_tab_lists_its_systems_by_name_ignoring_case() {
        // FUN_004611d0: All every system; Rebel 0x40 and Imperial 0x80 on
        // the side bits alone; Neutral 0xc0 when explored; Unexplored when
        // +0x88 bit 1 is clear, any side.
        let world = galaxy();
        assert_eq!(
            names(&rows(&world, SystemTab::All)),
            ["Adega", "Bespin", "Coruscant", "Kessel", "yavin"]
        );
        assert_eq!(names(&rows(&world, SystemTab::Rebel)), ["Kessel", "yavin"]);
        assert_eq!(names(&rows(&world, SystemTab::Imperial)), ["Coruscant"]);
        assert_eq!(names(&rows(&world, SystemTab::Neutral)), ["Bespin"]);
        assert_eq!(
            names(&rows(&world, SystemTab::Unexplored)),
            ["Adega", "Kessel"]
        );
    }

    #[test]
    fn the_tabs_carry_their_art_headings_and_places() {
        // FUN_00460270: tabs 49 x 42 from (36, 78), 52 apart; TEXTSTRA
        // 0x1867..0x1871.
        let expected = [
            (36.0, (10500, 10501), "All Systems"),
            (88.0, (10502, 10503), "Rebel Systems"),
            (140.0, (10505, 10506), "Imperial Systems"),
            (192.0, (10508, 10509), "Neutral Systems"),
            (244.0, (10511, 10512), "Unexplored Systems"),
        ];
        for (tab, (x, art, label)) in SystemTab::ALL.into_iter().zip(expected) {
            assert_eq!(tab_rect(tab), (x, 78.0, 49.0, 42.0));
            assert_eq!(tab.art(), art);
            assert_eq!(tab.label(), label);
        }
    }

    #[test]
    fn opening_chooses_nothing_and_a_tab_chooses_its_first_row() {
        // FUN_00460270 fills All without a choice; command 100 runs
        // FUN_00609650 with the empty name: the first row (A0 side tabs).
        let world = galaxy();
        let mut state = SystemFinderState::default();
        assert!(state.open(CockpitFaction::Empire));
        assert!(!state.open(CockpitFaction::Empire), "one Finder at most");
        assert_eq!(state.report(&world).unwrap().chosen, None);
        state.set_tab(SystemTab::Rebel);
        let window = state.window.as_mut().unwrap();
        assert!(window.rematch);
        // The rematch runs at the next draw; as draw does:
        let list = rows(&world, SystemTab::Rebel);
        window.chosen = prefix_choice(&list, "").map(|index| list[index].system);
        assert_eq!(state.report(&world).unwrap().chosen, Some(0));
    }

    #[test]
    fn escape_closes_the_finder_even_with_the_name_box_focused() {
        // FUN_00461590: Escape (0x1b) closes; the name box holds the focus
        // from the opening (FUN_00460270's SetFocus).
        let world = galaxy();
        let mut state = SystemFinderState::default();
        state.open(CockpitFaction::Alliance);
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let layout =
            crate::cockpit::CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
        for events in [
            vec![],
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
        ] {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(640.0, 480.0),
                )),
                events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                draw_system_finder(ctx, &world, &mut state, layout, &mut cache);
            });
        }
        assert!(!state.is_open());
    }

    #[test]
    fn a_typed_name_chooses_the_row_it_best_begins() {
        // FUN_00609650 on "cor".
        let world = galaxy();
        let list = rows(&world, SystemTab::All);
        assert_eq!(prefix_choice(&list, "cor"), Some(2));
        assert_eq!(prefix_choice(&list, "YAV"), Some(4));
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
    }

    #[test]
    fn the_window_and_its_controls_lie_where_the_original_places_them() {
        // A0 .artifacts/a0-vm/system-finder/01-f2.png: the Empire's 470 x 330
        // window at (125, 52), Close at (426, 21) and Display at (426, 89)
        // in it (FUN_00460270); the Alliance's buttons at x 427.
        let empire =
            crate::cockpit::CockpitState::new(CockpitFaction::Empire).layout_for(640.0, 480.0);
        let alliance =
            crate::cockpit::CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
        let mut state = SystemFinderState::default();
        assert_eq!(state.screen_rect(empire), None);
        assert!(!state.is_open());
        assert!(!state.contains_screen_point(empire, (200.0, 200.0)));
        assert_eq!(
            state.control_screen_rect(empire, SystemFinderControl::Close),
            None
        );
        state.open(CockpitFaction::Empire);
        assert!(state.is_open());
        assert_eq!(
            state.screen_rect(empire),
            Some(rect(125.0, 52.0, 470.0, 330.0))
        );
        assert!(state.contains_screen_point(empire, (125.0, 52.0)));
        assert!(!state.contains_screen_point(empire, (124.0, 52.0)));
        assert!(!state.contains_screen_point(empire, (125.0, 51.0)));
        let control = |state: &SystemFinderState, layout, control| {
            state.control_screen_rect(layout, control).unwrap()
        };
        assert_eq!(
            control(&state, empire, SystemFinderControl::Close),
            rect(551.0, 73.0, 44.0, 41.0)
        );
        assert_eq!(
            control(&state, empire, SystemFinderControl::Display),
            rect(551.0, 141.0, 44.0, 41.0)
        );
        assert_eq!(
            control(&state, empire, SystemFinderControl::NameBox),
            rect(268.0, 97.0, 250.0, 18.0)
        );
        assert_eq!(
            control(
                &state,
                empire,
                SystemFinderControl::Tab(SystemTab::Imperial)
            ),
            rect(265.0, 130.0, 49.0, 42.0)
        );
        // The list's rows are 20 high from (36, 138); a row shows only whole
        // in the 165-pixel list.
        assert_eq!(
            control(&state, empire, SystemFinderControl::Row(0)),
            rect(161.0, 190.0, 338.0, 20.0)
        );
        assert_eq!(
            control(&state, empire, SystemFinderControl::Row(7)),
            rect(161.0, 330.0, 338.0, 20.0)
        );
        assert_eq!(
            state.control_screen_rect(empire, SystemFinderControl::Row(8)),
            None
        );
        state.window.as_mut().unwrap().scroll = 30;
        assert_eq!(
            state.control_screen_rect(empire, SystemFinderControl::Row(1)),
            None
        );
        assert_eq!(
            control(&state, empire, SystemFinderControl::Row(2)),
            rect(161.0, 200.0, 338.0, 20.0)
        );
        assert_eq!(
            control(&state, empire, SystemFinderControl::Row(8)),
            rect(161.0, 320.0, 338.0, 20.0)
        );
        assert_eq!(
            state.control_screen_rect(empire, SystemFinderControl::Row(9)),
            None
        );
        assert_eq!(state.report(&galaxy()).unwrap().first_row, 1);

        let mut state = SystemFinderState::default();
        state.open(CockpitFaction::Alliance);
        let window = state.screen_rect(alliance).unwrap();
        assert_eq!(
            control(&state, alliance, SystemFinderControl::Close),
            rect(window.min.x + 427.0, window.min.y + 25.0, 32.0, 31.0)
        );
        assert_eq!(
            control(&state, alliance, SystemFinderControl::Display),
            rect(window.min.x + 427.0, window.min.y + 93.0, 32.0, 31.0)
        );
    }

    #[test]
    fn the_list_scrolls_as_little_as_it_must_to_show_a_row() {
        // By pixels in the 165-pixel list, as the Fleet Finder's
        // (FUN_0060a490, FUN_00609ae0): row 8 (160..180) needs 15.
        assert_eq!(scrolled_to(0, 7, 20), 0);
        assert_eq!(scrolled_to(0, 8, 20), 15);
        assert_eq!(scrolled_to(100, 2, 20), 40);
        assert_eq!(scrolled_to(100, 5, 20), 100);
        assert_eq!(scrolled_to(100, 12, 20), 100);
        assert_eq!(scrolled_to(100, 13, 20), 115);
        assert_eq!(scrolled_to(0, 19, 20), 235);
        assert_eq!(scrolled_to(0, 30, 20), 240);
        assert_eq!(max_scroll(20), 240);
        assert_eq!(max_scroll(8), 0);
    }
}
