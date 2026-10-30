//! The Troop Finder (window type `0x16`, `FUN_0042a4d0`;
//! `ghidra/notes/troop-finder.md`).
//!
//! A 470 by 330 window the cockpit's Troop Finder control (`0x130`) or F5
//! opens. It lists systems and fleets that hold regiments of the selected
//! side under two tabs (Alliance, Imperial), sorted by name
//! (`FUN_0046ea10`). Each row shows five count cells by troop class.
//! Typing a name picks the row it best begins (`FUN_00609650`); Display,
//! Enter or a double click opens the Sector and System Defenses windows
//! for the choice (`FUN_00429440`).

use egui_macroquad::egui;
use rebellion_core::ids::{FleetKey, SystemKey, TroopKey};
use rebellion_core::troop_transport::TroopTransportState;
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::fleet_window::paint_native;
use crate::mission_dialog::{button, galaxy_centered_rect};
use crate::system_window::{fleet_label, logical_rect, rect_contains};

pub const TROOP_FINDER_WIDTH: f32 = 470.0;
pub const TROOP_FINDER_HEIGHT: f32 = 330.0;

// STRATEGY bitmaps by side, Alliance then Empire (FUN_0046ce40).
const BACKGROUND: [u32; 2] = [10335, 10336];
// Panel bitmap chosen by SELECTED TAB (FUN_0046ce40: 0x292c, 0x292d).
const PANEL: [u32; 2] = [10540, 10541];
const RAIL: [u32; 2] = [10584, 10588];
const PANEL_AT: (f32, f32) = (12.0, 13.0);
const RAIL_AT: (f32, f32) = (412.0, 0.0);

/// One bitmap button: its window rectangle and its normal and pressed art.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Control {
    rect: (f32, f32, f32, f32),
    art: (u32, u32),
}

/// The side's Close (`200`) and Display (`0xc9`) controls (FUN_0046ce40).
struct SideControls {
    close: Control,
    display: Control,
}

const CONTROLS: [SideControls; 2] = [
    // Alliance (side 1): (0x1ab, 0x19) = (427, 25) and (0x1ab, 0x5d) = (427, 93).
    SideControls {
        close: Control {
            rect: (427.0, 25.0, 32.0, 31.0),
            art: (10514, 10515),
        },
        display: Control {
            rect: (427.0, 93.0, 32.0, 31.0),
            art: (10518, 10519),
        },
    },
    // Empire (side 2): (0x1aa, 0x15) = (426, 21) and (0x1aa, 0x59) = (426, 89).
    SideControls {
        close: Control {
            rect: (426.0, 21.0, 44.0, 41.0),
            art: (10516, 10517),
        },
        display: Control {
            rect: (426.0, 89.0, 44.0, 41.0),
            art: (10520, 10521),
        },
    },
];

/// The side tabs (group `100`) at (36, 72): Alliance, Imperial.
/// FUN_0046ce40: tab 1 at (0, 0) 49x41 art 10502/10503; tab 2 at (52, 0)
/// 49x41 art 10505/10506.
const TABS: [Control; 2] = [
    Control {
        rect: (36.0, 72.0, 49.0, 41.0),
        art: (10502, 10503),
    },
    Control {
        rect: (88.0, 72.0, 49.0, 41.0),
        art: (10505, 10506),
    },
];

// Window rectangles (x, y, width, height), FUN_0046ce40.
const TITLE_AT: (f32, f32) = (36.0, 14.0); // 0x24, 0x0e
/// The list's text x (`+0xe8 = 5`) and unchosen colour (`0x787878`).
const ROW_TEXT_X: f32 = 5.0;
const ROW_COLOR: egui::Color32 = egui::Color32::from_rgb(0x78, 0x78, 0x78);
const NAME_LABEL_AT: (f32, f32) = (36.0, 48.0); // 0x24, 0x30
const NAME_BOX: (f32, f32, f32, f32) = (143.0, 45.0, 250.0, 18.0); // 0x8f, 0x2d
const TAB_LABEL_AT: (f32, f32) = (40.0, 119.0); // 0x28, 0x77
const LIST: (f32, f32, f32, f32) = (37.0, 144.0, 349.0, 159.0); // 0x25, 0x90
/// `FUN_00607ea0(.., 0xca, 330, 25)`: each row is 330 by 25.
const ROW_HEIGHT: f32 = 25.0;

/// The side tab (FUN_0046ea10's `param_1` 1 or 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TroopTab {
    #[default]
    Alliance,
    Imperial,
}

impl TroopTab {
    const ALL: [Self; 2] = [Self::Alliance, Self::Imperial];

    fn lists(self, is_alliance: bool) -> bool {
        match self {
            Self::Alliance => is_alliance,
            Self::Imperial => !is_alliance,
        }
    }

    /// TEXTSTRA `0x1950`/`0x1951`.
    const fn label(self) -> &'static str {
        match self {
            Self::Alliance => "Alliance Troops",
            Self::Imperial => "Imperial Troops",
        }
    }

    /// Index into the panel bitmap array: Alliance panel = 0, Imperial = 1.
    const fn panel_index(self) -> usize {
        match self {
            Self::Alliance => 0,
            Self::Imperial => 1,
        }
    }
}

/// Five troop DatIds per side (FUN_0046d8d0).
const ALLIANCE_TROOP_CLASSES: [u32; 5] = [
    0x1000_0002, // Army Regiment
    0x1000_0001, // Fleet Regiment
    0x1000_0005,
    0x1000_0003,
    0x1000_0004,
];
const EMPIRE_TROOP_CLASSES: [u32; 5] = [
    0x1000_0008, // Stormtrooper Regiment
    0x1000_000a, // ScoutTrooper Regiment
    0x1000_0007,
    0x1000_0006,
    0x1000_0009,
];

fn troop_classes(tab: TroopTab) -> &'static [u32; 5] {
    match tab {
        TroopTab::Alliance => &ALLIANCE_TROOP_CLASSES,
        TroopTab::Imperial => &EMPIRE_TROOP_CLASSES,
    }
}

/// One count cell in a row: the number of regiments and whether any are
/// not usable. port: the port has no usable bit for regiments; all are
/// treated as usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TroopCount {
    pub count: u32,
    /// port: always true (the port has no usable / `+0x50 bit 0` flag).
    pub all_usable: bool,
}

/// One list row: its name and the five troop-class counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TroopFinderRow {
    /// A system name or fleet name.
    pub name: String,
    /// The system this row represents (system row → the system; fleet
    /// row → the fleet's system).
    pub system: SystemKey,
    /// If the row represents a fleet at that system, the fleet key.
    pub fleet: Option<FleetKey>,
    /// Five columns of troop counts per class.
    pub counts: [TroopCount; 5],
}

/// A typed name chooses the first row it best begins, or the first row
/// (FUN_00609650, [`crate::list_prefix::best_prefix_row`]).
#[must_use]
pub fn prefix_choice(rows: &[TroopFinderRow], text: &str) -> Option<usize> {
    crate::list_prefix::best_prefix_row(rows.iter().map(|row| row.name.as_str()), text)
}

/// Count regiments of each class among the given troop keys.
fn count_troops(world: &GameWorld, troops: &[TroopKey], tab: TroopTab) -> [TroopCount; 5] {
    let classes = troop_classes(tab);
    let mut counts = [TroopCount {
        count: 0,
        all_usable: true,
    }; 5];
    for &key in troops {
        let Some(troop) = world.troops.get(key) else {
            continue;
        };
        if !tab.lists(troop.is_alliance) {
            continue;
        }
        let raw = troop.class_dat_id.raw();
        for (i, &class_raw) in classes.iter().enumerate() {
            if raw == class_raw {
                counts[i].count += 1;
                // port: all regiments treated as usable (no `+0x50` bit).
                break;
            }
        }
    }
    counts
}

/// The rows FUN_0046ea10 lists for `tab`, in name order ignoring case.
///
/// For each system, if it holds regiments of the tab's side (direct
/// children via `System.ground_units`), the system is listed. Then, for
/// each fleet at that system of the tab's side, if any of its ships
/// carries regiments (via `TroopTransportState`), a row named by the
/// fleet is listed. The walks (`FUN_00504cc0`, `FUN_004ffef0`) run in
/// mode 3, existing, and test no visibility, so the other side's tab lists
/// every regiment it holds.
#[must_use]
pub fn rows(
    world: &GameWorld,
    tab: TroopTab,
    transport: &TroopTransportState,
) -> Vec<TroopFinderRow> {
    let mut rows = Vec::new();

    for (system_key, system) in &world.systems {
        // Check direct ground troops at this system. FUN_00504cc0 walks
        // them in mode 3, existing (+0x50 bit 6), with no visibility test.
        let ground: Vec<TroopKey> = system
            .ground_units
            .iter()
            .copied()
            .filter(|&key| {
                world
                    .troops
                    .get(key)
                    .is_some_and(|troop| tab.lists(troop.is_alliance))
            })
            .collect();
        if !ground.is_empty() {
            let counts = count_troops(world, &ground, tab);
            rows.push(TroopFinderRow {
                name: system.name.clone(),
                system: system_key,
                fleet: None,
                counts,
            });
        }

        // Check fleets at this system.
        for &fleet_key in &system.fleets {
            let Some(fleet) = world.fleets.get(fleet_key) else {
                continue;
            };
            if !tab.lists(fleet.is_alliance) {
                continue;
            }
            let cargo = transport.cargo(fleet_key);
            if cargo.is_empty() {
                continue;
            }
            // FUN_00504c40: any regiment aboard lists the fleet, whatever
            // its class; the five columns count those they name.
            let counts = count_troops(world, cargo, tab);
            let Some(label) = fleet_label(world, fleet_key) else {
                continue;
            };
            rows.push(TroopFinderRow {
                name: label,
                system: system_key,
                fleet: Some(fleet_key),
                counts,
            });
        }
    }

    rows.sort_by_cached_key(|row| row.name.to_ascii_lowercase());
    rows
}

/// What the Troop Finder asks of the galaxy view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TroopFinderAction {
    /// System row: open the Sector window and the System Defenses window
    /// (FUN_00429440 kind 10) selecting the first regiment.
    OpenDefenses { system: SystemKey },
    /// Fleet row: open the Sector window and the Fleet window (kind 4)
    /// for the fleet's system.
    OpenFleet { system: SystemKey, fleet: FleetKey },
}

#[derive(Debug, Clone)]
struct OpenFinder {
    faction: CockpitFaction,
    tab: TroopTab,
    /// The name box's text (`0xcb`).
    name: String,
    /// The chosen row index into the current list.
    chosen: Option<usize>,
    /// The first row the list shows.
    /// The list's first visible pixel (it scrolls by pixels,
    /// `FUN_0060a490`).
    scroll: usize,
    /// A press held on the list's scroll bar.
    scroll_hold: Option<crate::list_scroll_bar::Hold>,
    /// The name box takes the focus when the window opens.
    focus_name: bool,
    /// The row the last click chose and when.
    last_clicked: Option<(usize, f64)>,
    /// The tab's notification (case `100`, `FUN_0046df90`) is due: once
    /// the list is built, the name box's text chooses its row
    /// (`FUN_00609650`), the first when none matches.
    rematch: bool,
}

/// The Troop Finder, one at most (FUN_0042a4d0).
#[derive(Debug, Clone, Default)]
pub struct TroopFinderState {
    window: Option<OpenFinder>,
}

impl TroopFinderState {
    /// Open the Finder on the player's side tab with the focus in the
    /// name box (FUN_0046ce40). An open Finder stays as it is.
    pub fn open(&mut self, faction: CockpitFaction) -> bool {
        if self.window.is_some() {
            return false;
        }
        let tab = match faction {
            CockpitFaction::Alliance => TroopTab::Alliance,
            CockpitFaction::Empire => TroopTab::Imperial,
        };
        self.window = Some(OpenFinder {
            faction,
            tab,
            name: String::new(),
            chosen: None,
            scroll: 0,
            scroll_hold: None,
            focus_name: true,
            last_clicked: None,
            // FUN_0060d7e0(side, 1) notifies the tab as the window opens.
            rematch: true,
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

    /// Whether `point` falls on the open window.
    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        self.window.is_some() && rect_contains(window_rect(layout), egui::pos2(point.0, point.1))
    }

    /// Show `tab` (group `100`, `FUN_0046df90`): the list rebuilds
    /// (`FUN_0046ea10`), then the name box's text chooses a row.
    pub fn set_tab(&mut self, tab: TroopTab) {
        if let Some(window) = &mut self.window {
            if window.tab != tab {
                window.tab = tab;
                window.chosen = None;
                window.scroll = 0;
                window.last_clicked = None;
                window.rematch = true;
            }
        }
    }
}

const fn side(faction: CockpitFaction) -> usize {
    match faction {
        CockpitFaction::Alliance => 0,
        CockpitFaction::Empire => 1,
    }
}

const fn tab_index(tab: TroopTab) -> usize {
    match tab {
        TroopTab::Alliance => 0,
        TroopTab::Imperial => 1,
    }
}

/// hyp: `FUN_00606980` places it in the galaxy view; the port centres it.
fn window_rect(layout: CockpitLayout) -> egui::Rect {
    galaxy_centered_rect(layout, TROOP_FINDER_WIDTH, TROOP_FINDER_HEIGHT)
}

fn at(frame: egui::Rect, scale: f32, (x, y, w, h): (f32, f32, f32, f32)) -> egui::Rect {
    logical_rect(frame, scale, x, y, w, h)
}

/// The list's height and rows in pixels, its bar's width (RCDATA 10653,
/// `FUN_0046ce40`: `FUN_0060a490(.., list, 7, 0x299d, ..)`) and arrow step
/// (`FUN_00607ea0`'s argument 10).
const LIST_PIXELS: usize = 159;
const ROW_PIXELS: usize = 25;
const BAR_WIDTH: f32 = 12.0;
const LIST_STEP: f32 = 10.0;

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

/// The list's scroll range for `rows` rows, in pixels
/// ([`crate::list_scroll_bar::max_first`]).
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "A few hundred rows of 25 pixels."
)]
fn max_scroll(rows: usize) -> usize {
    crate::list_scroll_bar::max_first((rows * ROW_PIXELS) as f32, LIST.3) as usize
}

/// The five column cell colours (FUN_0046d8d0).
///
/// Brush index 0 = red (0x20000ff → RGB 255,0,0): tab Alliance, all usable.
/// Brush index 1 = green (0x200ff00 → RGB 0,255,0): tab Imperial, all usable.
/// Brush index 3 = dark red (0x80 → RGB 128,0,0): tab Alliance, some not usable.
/// Brush index 4 = dark green (0x8000 → RGB 0,128,0): tab Imperial, some not usable.
///
/// port: all regiments are treated as usable, so only bright colours appear.
fn cell_colours(tab: TroopTab) -> (egui::Color32, egui::Color32) {
    // (background_box, text_colour)
    match tab {
        TroopTab::Alliance => (egui::Color32::from_rgb(255, 0, 0), egui::Color32::WHITE),
        TroopTab::Imperial => (egui::Color32::from_rgb(0, 255, 0), egui::Color32::BLACK),
    }
}

/// Draw the open Troop Finder, if any, and report an action.
#[expect(
    clippy::too_many_lines,
    reason = "The window paints in the original's control order; splitting it hides that order."
)]
pub fn draw_troop_finder(
    ctx: &egui::Context,
    world: &GameWorld,
    state: &mut TroopFinderState,
    layout: CockpitLayout,
    cache: &mut BmpCache,
    transport: &TroopTransportState,
) -> Option<TroopFinderAction> {
    let mut window = state.window.clone()?;
    let side_idx = side(window.faction);
    let controls = &CONTROLS[side_idx];
    let scale = layout.scale;
    let rect = window_rect(layout);
    let list = rows(world, window.tab, transport);
    if window.rematch {
        // FUN_0046df90 case 100: FUN_00609650 chooses and scrolls to it.
        window.rematch = false;
        window.chosen = prefix_choice(&list, &window.name);
        if let Some(index) = window.chosen {
            window.scroll = scrolled_to(window.scroll, index, list.len());
        }
        state.window.clone_from(&Some(window.clone()));
    }

    let mut close = false;
    let mut open_action: Option<TroopFinderAction> = None;
    let mut tab = None;
    let mut clicked_row = None;
    let mut name = window.name.clone();
    let mut name_changed = false;
    let mut enter = false;
    let mut wheel = 0.0;
    let mut scroll = window.scroll;
    let mut scroll_hold = window.scroll_hold;

    let id = egui::Id::new("original-troop-finder");
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
            paint(cache, PANEL[window.tab.panel_index()], PANEL_AT);
            paint(cache, BACKGROUND[side_idx], (0.0, 0.0));
            paint(cache, RAIL[side_idx], RAIL_AT);

            // Title (TEXTSTRA 0x1900) centred in its 350 in font 5, label
            // (0x1901) in font 4, the tab's label in font 5; the list and
            // the name box in font 10 (A0 2026-10-29).
            let title_font = crate::theme::original_font(5, scale);
            let label_font = crate::theme::original_font(4, scale);
            let font = crate::theme::original_font(10, scale);
            painter.text(
                at(frame, scale, (TITLE_AT.0 + 175.0, TITLE_AT.1, 0.0, 0.0)).min,
                egui::Align2::CENTER_TOP,
                "Troop Finder",
                title_font.clone(),
                egui::Color32::WHITE,
            );
            painter.text(
                at(frame, scale, (NAME_LABEL_AT.0, NAME_LABEL_AT.1, 0.0, 0.0)).min,
                egui::Align2::LEFT_TOP,
                "Troop Location",
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

            // Side tabs.
            for candidate in TroopTab::ALL {
                let control = TABS[tab_index(candidate)];
                let down = candidate == window.tab;
                if button(
                    ui,
                    cache,
                    at(frame, scale, control.rect),
                    ("troop-tab", tab_index(candidate)),
                    control.art,
                    down,
                    scale,
                ) {
                    tab = Some(candidate);
                }
            }

            // Close and Display buttons.
            if button(
                ui,
                cache,
                at(frame, scale, controls.close.rect),
                "troop-close",
                controls.close.art,
                false,
                scale,
            ) {
                close = true;
            }
            if button(
                ui,
                cache,
                at(frame, scale, controls.display.rect),
                "troop-display",
                controls.display.art,
                false,
                scale,
            ) {
                if let Some(chosen) = window.chosen {
                    if let Some(row) = list.get(chosen) {
                        open_action = Some(action_for_row(row));
                    }
                }
            }

            // The name box (0xcb).
            let name_rect = at(frame, scale, NAME_BOX);
            // port: the caret stays inside the box, as a Windows edit
            // control's does; egui's runs a row past its bottom.
            let outer_clip = ui.clip_rect();
            ui.set_clip_rect(name_rect.intersect(outer_clip));
            let edit = ui.put(
                name_rect,
                egui::TextEdit::singleline(&mut name)
                    .id(egui::Id::new("original-troop-finder-name"))
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

            // The list (0xca).
            let list_rect = at(frame, scale, LIST);
            let list_painter = painter.with_clip_rect(list_rect);
            let cell_font = egui::FontId::proportional((10.0 * scale).max(7.0));
            let (box_colour, text_on_box) = cell_colours(window.tab);

            // The bar, then the rows scrolled by pixels and clipped to the
            // list; a row's click area ends at the bar.
            #[expect(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "A list's pixels."
            )]
            {
                scroll = crate::list_scroll_bar::ui_bar(
                    ui,
                    cache,
                    frame,
                    scale,
                    crate::list_scroll_bar::RESULTS_ART,
                    LIST,
                    (list.len() * ROW_PIXELS) as f32,
                    scroll as f32,
                    &mut scroll_hold,
                    LIST_STEP,
                )
                .max(0.0) as usize;
            }
            for (offset, row) in list.iter().enumerate() {
                #[expect(clippy::cast_precision_loss, reason = "A list's pixels.")]
                let line = offset as f32 - scroll as f32 / ROW_HEIGHT;
                if (line + 1.0) * ROW_HEIGHT <= 0.0 || line * ROW_HEIGHT >= LIST.3 {
                    continue;
                }
                // The text keeps the whole row's place; the click area is
                // the row's part in the list.
                let full_row = at(
                    frame,
                    scale,
                    (LIST.0, LIST.1 + ROW_HEIGHT * line, LIST.2 - BAR_WIDTH, ROW_HEIGHT),
                );
                let row_rect = full_row.intersect(list_rect);
                // FUN_0046ce40: the text at x 5 (+0xe8) from the row's top
                // (+0xe4 = 0x20, DT_SINGLELINE without DT_VCENTER), grey
                // 0x787878 and the chosen row white (FUN_00607ea0's
                // colours; A0 2026-10-29).
                let name_colour = if window.chosen == Some(offset) {
                    egui::Color32::WHITE
                } else {
                    ROW_COLOR
                };
                list_painter.text(
                    full_row.left_top() + egui::vec2(ROW_TEXT_X * scale, 0.0),
                    egui::Align2::LEFT_TOP,
                    &row.name,
                    font.clone(),
                    name_colour,
                );

                // Five count columns at x = 194 + 28*i, y = 2 within the row
                // (FUN_0046d8d0).
                for (i, cell) in row.counts.iter().enumerate() {
                    if cell.count == 0 {
                        // Count 0 → "-" in white, no box.
                        let cell_x = 194.0 + 28.0 * i as f32;
                        let cell_pos = at(
                            frame,
                            scale,
                            (LIST.0 + cell_x, LIST.1 + ROW_HEIGHT * line + 2.0, 28.0, 0.0),
                        )
                        .min;
                        list_painter.text(
                            cell_pos + egui::vec2(10.0 * scale, 0.0),
                            egui::Align2::CENTER_TOP,
                            "-",
                            cell_font.clone(),
                            egui::Color32::WHITE,
                        );
                    } else {
                        // Count > 0 → a 21x16 box behind the number.
                        let cell_x = 194.0 + 28.0 * i as f32;
                        let box_rect = at(
                            frame,
                            scale,
                            (
                                LIST.0 + cell_x,
                                LIST.1 + ROW_HEIGHT * line + 2.0,
                                21.0,
                                16.0,
                            ),
                        );
                        list_painter.rect_filled(box_rect, 0.0, box_colour);
                        list_painter.text(
                            box_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            cell.count.to_string(),
                            cell_font.clone(),
                            text_on_box,
                        );
                    }
                }

                let response = ui.interact(
                    row_rect,
                    ui.id().with(("troop-row", offset)),
                    egui::Sense::click(),
                );
                if response.clicked() {
                    let (now, delay) = (
                        ui.input(|input| input.time),
                        ctx.options(|options| options.input_options.max_double_click_delay),
                    );
                    if window
                        .last_clicked
                        .is_some_and(|(last, at_time)| last == offset && now - at_time <= delay)
                    {
                        open_action = Some(action_for_row(row));
                    } else {
                        clicked_row = Some((offset, now));
                    }
                }
            }
            let pointer = ui.input(|input| input.pointer.hover_pos());
            if pointer.is_some_and(|point| rect_contains(list_rect, point)) {
                wheel = ui.input(|input| input.raw_scroll_delta.y);
            }
        });

    let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let open_window = state.window.as_mut()?;
    open_window.focus_name = false;
    open_window.scroll = scroll;
    open_window.scroll_hold = scroll_hold;
    if name_changed {
        open_window.name.clone_from(&name);
        let choice = prefix_choice(&list, &name);
        open_window.chosen = choice;
        if let Some(index) = choice {
            open_window.scroll = scrolled_to(open_window.scroll, index, list.len());
        }
    }
    if let Some((row, at_time)) = clicked_row {
        open_window.last_clicked = Some((row, at_time));
        open_window.chosen = Some(row);
        open_window.name.clone_from(&list[row].name);
    }
    if enter {
        if let Some(chosen) = open_window.chosen {
            if let Some(row) = list.get(chosen) {
                open_action = Some(action_for_row(row));
            }
        }
    }
    // port: the original has no wheel; a notch moves a row.
    if wheel != 0.0 {
        open_window.scroll = if wheel > 0.0 {
            open_window.scroll.saturating_sub(ROW_PIXELS)
        } else {
            (open_window.scroll + ROW_PIXELS).min(max_scroll(list.len()))
        };
    }
    if let Some(new_tab) = tab {
        state.set_tab(new_tab);
    }
    if let Some(action) = open_action {
        state.close();
        return Some(action);
    }
    if close || escape {
        state.close();
    }
    None
}

/// The action for opening a row (FUN_00429440).
fn action_for_row(row: &TroopFinderRow) -> TroopFinderAction {
    if let Some(fleet) = row.fleet {
        TroopFinderAction::OpenFleet {
            system: row.system,
            fleet,
        }
    } else {
        TroopFinderAction::OpenDefenses { system: row.system }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cockpit::CockpitViewport;
    use rebellion_core::dat::{ExplorationStatus, Faction, SectorGroup};
    use rebellion_core::ids::{DatId, FleetKey, SystemKey};
    use rebellion_core::world::{
        CapitalShipClass, ControlKind, Fleet, Sector, ShipInstance, System, TroopUnit,
    };

    fn layout() -> CockpitLayout {
        CockpitLayout {
            canvas: CockpitViewport {
                x: 0.0,
                y: 0.0,
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

    fn add_system(world: &mut GameWorld, name: &str, control: ControlKind) -> SystemKey {
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
            control,
        })
    }

    fn add_troop(
        world: &mut GameWorld,
        system: SystemKey,
        class_raw: u32,
        is_alliance: bool,
    ) -> TroopKey {
        let key = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(class_raw),
            is_alliance,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(key);
        key
    }

    fn add_fleet(world: &mut GameWorld, system: SystemKey, is_alliance: bool) -> FleetKey {
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            name: "Transport".into(),
            is_alliance,
            hull: 100,
            hyperdrive: 1,
            troop_capacity: 4,
            ..CapitalShipClass::default()
        });
        let fleet = world.insert_fleet(Fleet {
            location: system,
            capital_ships: vec![ShipInstance::new(class, 100, true)],
            fighters: Vec::new(),
            characters: Vec::new(),
            is_alliance,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[system].fleets.push(fleet);
        fleet
    }

    fn names(rows: &[TroopFinderRow]) -> Vec<&str> {
        rows.iter().map(|row| row.name.as_str()).collect()
    }

    struct Galaxy {
        world: GameWorld,
    }

    fn galaxy() -> Galaxy {
        let mut world = GameWorld::default();
        let yavin = add_system(
            &mut world,
            "Yavin",
            ControlKind::Controlled(Faction::Alliance),
        );
        // Two Alliance army regiments on Yavin.
        add_troop(&mut world, yavin, 0x1000_0002, true);
        add_troop(&mut world, yavin, 0x1000_0002, true);
        // One Alliance fleet regiment.
        add_troop(&mut world, yavin, 0x1000_0001, true);
        let coruscant = add_system(
            &mut world,
            "Coruscant",
            ControlKind::Controlled(Faction::Empire),
        );
        // One Empire stormtrooper regiment on Coruscant.
        add_troop(&mut world, coruscant, 0x1000_0008, false);
        Galaxy { world }
    }

    #[test]
    fn the_troop_finder_lists_systems_holding_the_tabs_regiments() {
        // FUN_0046ea10: walk systems, check FUN_00504cc0 for regiments
        // of the tab's side.
        let galaxy = galaxy();
        let transport = TroopTransportState::default();

        let alliance = rows(&galaxy.world, TroopTab::Alliance, &transport);
        assert_eq!(names(&alliance), ["Yavin"]);
        assert_eq!(alliance[0].counts[0].count, 2, "Army Regiment column");
        assert_eq!(alliance[0].counts[1].count, 1, "Fleet Regiment column");
        assert_eq!(alliance[0].counts[2].count, 0);

        let imperial = rows(&galaxy.world, TroopTab::Imperial, &transport);
        // FUN_0046ea10 walks all systems; ground troops are shown
        // regardless of fog.
        assert_eq!(names(&imperial), ["Coruscant"]);
        assert_eq!(imperial[0].counts[0].count, 1, "Stormtrooper column");
    }

    #[test]
    fn a_fleet_carrying_regiments_is_listed_by_its_fleet_name() {
        // FUN_0046ea10: for each fleet at the system of the tab's side,
        // check FUN_00504c40 for regiments.
        let mut galaxy = galaxy();
        let yavin = galaxy.world.systems.iter().next().unwrap().0;
        let fleet = add_fleet(&mut galaxy.world, yavin, true);
        let troop = galaxy.world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1000_0002),
            is_alliance: true,
            regiment_strength: 100,
        });
        galaxy.world.systems[yavin].ground_units.push(troop);
        let mut transport = TroopTransportState::default();
        transport
            .embark(&mut galaxy.world, fleet, &[troop])
            .unwrap();

        let listed = rows(&galaxy.world, TroopTab::Alliance, &transport);

        // Yavin (ground) and the fleet.
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().any(|r| r.fleet == Some(fleet)));
        let fleet_row = listed.iter().find(|r| r.fleet.is_some()).unwrap();
        assert_eq!(fleet_row.counts[0].count, 1);
    }

    #[test]
    fn rows_are_sorted_by_name_ignoring_case() {
        // FUN_005f59f0: rows inserted in name order.
        let mut world = GameWorld::default();
        let beta = add_system(
            &mut world,
            "beta",
            ControlKind::Controlled(Faction::Alliance),
        );
        let alpha = add_system(
            &mut world,
            "Alpha",
            ControlKind::Controlled(Faction::Alliance),
        );
        add_troop(&mut world, beta, 0x1000_0002, true);
        add_troop(&mut world, alpha, 0x1000_0002, true);
        let transport = TroopTransportState::default();

        let listed = rows(&world, TroopTab::Alliance, &transport);
        assert_eq!(names(&listed), ["Alpha", "beta"]);
    }

    #[test]
    fn a_typed_name_chooses_the_first_row_it_best_begins() {
        // FUN_00609650: the same prefix-match the Fleet Finder uses.
        let rows = vec![
            TroopFinderRow {
                name: "Coruscant".into(),
                system: SystemKey::default(),
                fleet: None,
                counts: [TroopCount::default(); 5],
            },
            TroopFinderRow {
                name: "Yavin".into(),
                system: SystemKey::default(),
                fleet: None,
                counts: [TroopCount::default(); 5],
            },
        ];

        assert_eq!(prefix_choice(&rows, "ya"), Some(1));
        assert_eq!(prefix_choice(&rows, "cor"), Some(0));
        // FUN_00609650: with no row sharing the first letter, the first.
        assert_eq!(prefix_choice(&rows, "z"), Some(0));
        assert_eq!(prefix_choice(&rows, ""), Some(0));
    }

    #[test]
    fn the_action_for_a_system_row_opens_defenses_and_a_fleet_row_opens_fleet() {
        // FUN_00429440: system → kind 10 (defenses), fleet → kind 4.
        let system = SystemKey::default();
        let fleet = FleetKey::default();

        let system_row = TroopFinderRow {
            name: "Yavin".into(),
            system,
            fleet: None,
            counts: [TroopCount::default(); 5],
        };
        assert_eq!(
            action_for_row(&system_row),
            TroopFinderAction::OpenDefenses { system }
        );

        let fleet_row = TroopFinderRow {
            name: "Fleet 1".into(),
            system,
            fleet: Some(fleet),
            counts: [TroopCount::default(); 5],
        };
        assert_eq!(
            action_for_row(&fleet_row),
            TroopFinderAction::OpenFleet { system, fleet }
        );
    }

    #[test]
    fn six_whole_rows_fit_the_list() {
        // FUN_00607ea0: list 159 tall, rows 25 tall.
        assert_eq!(LIST_PIXELS / ROW_PIXELS, 6);
    }

    #[test]
    fn the_list_scrolls_as_little_as_it_must_to_show_a_row() {
        // By pixels (FUN_0060a490): row 6 (150..175) needs 16; twenty rows
        // reach 500 + 5 - 159 = 346 (FUN_00609ae0).
        assert_eq!(scrolled_to(0, 5, 20), 0);
        assert_eq!(scrolled_to(0, 6, 20), 16);
        assert_eq!(scrolled_to(75, 1, 20), 25);
        assert_eq!(scrolled_to(75, 8, 20), 75);
        assert_eq!(scrolled_to(50, 8, 20), 66);
        assert_eq!(max_scroll(20), 346);
        assert_eq!(max_scroll(6), 0);
    }

    #[test]
    fn opening_an_open_finder_keeps_its_state() {
        // FUN_0042a4d0 builds the window only when no type 0x16 is open.
        let mut state = TroopFinderState::default();
        assert!(state.open(CockpitFaction::Alliance));
        state.set_tab(TroopTab::Imperial);

        assert!(!state.open(CockpitFaction::Alliance));

        assert_eq!(state.window.as_ref().unwrap().tab, TroopTab::Imperial);
        assert!(state.is_open());
        state.close();
        assert!(!state.is_open());
    }

    #[test]
    fn the_longest_common_beginning_wins_and_a_tie_keeps_the_first() {
        // FUN_00609650 keeps a row only for a strictly longer match.
        let rows: Vec<TroopFinderRow> = ["Corulag", "Coruscant", "Yavin"]
            .into_iter()
            .map(|name| TroopFinderRow {
                name: name.into(),
                system: SystemKey::default(),
                fleet: None,
                counts: [TroopCount::default(); 5],
            })
            .collect();
        assert_eq!(prefix_choice(&rows, "corus"), Some(1));
        assert_eq!(prefix_choice(&rows, "cor"), Some(0));
    }

    #[test]
    fn each_tab_names_its_side_and_paints_its_own_cells() {
        // TEXTSTRA 0x1950/0x1951; panels 10540/10541 (FUN_006075e0);
        // FUN_0046d8d0's brushes: red with white text for the Alliance,
        // green with black for the Empire.
        assert_eq!(TroopTab::Alliance.label(), "Alliance Troops");
        assert_eq!(TroopTab::Imperial.label(), "Imperial Troops");
        assert_eq!(PANEL[TroopTab::Alliance.panel_index()], 10540);
        assert_eq!(PANEL[TroopTab::Imperial.panel_index()], 10541);
        assert_eq!(
            cell_colours(TroopTab::Alliance),
            (egui::Color32::from_rgb(255, 0, 0), egui::Color32::WHITE)
        );
        assert_eq!(
            cell_colours(TroopTab::Imperial),
            (egui::Color32::from_rgb(0, 255, 0), egui::Color32::BLACK)
        );
    }

    #[test]
    fn a_new_tab_clears_the_choice() {
        // FUN_0046ea10: rebuilding clears the chosen row.
        let mut state = TroopFinderState::default();
        state.open(CockpitFaction::Alliance);
        state.window.as_mut().unwrap().chosen = Some(0);

        state.set_tab(TroopTab::Imperial);

        assert_eq!(state.window.as_ref().unwrap().chosen, None);
    }

    #[test]
    fn the_opening_and_a_new_tab_choose_the_row_the_name_begins_or_the_first() {
        // FUN_0046ce40 opens with FUN_0060d7e0(side, 1), whose notification
        // (FUN_0046df90 case 100) rebuilds the list and lets the name box's
        // text choose a row (FUN_00609650): the A0 Troop Finder opens with
        // its first row white under an empty name box, on either tab (VM,
        // 2026-10-29).
        let mut world = GameWorld::default();
        for name in ["Alpha", "Beta"] {
            let system = add_system(&mut world, name, ControlKind::Controlled(Faction::Alliance));
            add_troop(&mut world, system, 0x1000_0002, true);
        }
        let coruscant = add_system(
            &mut world,
            "Coruscant",
            ControlKind::Controlled(Faction::Empire),
        );
        add_troop(&mut world, coruscant, 0x1000_0008, false);
        let mut state = TroopFinderState::default();
        state.open(CockpitFaction::Alliance);
        assert_eq!(state.window.as_ref().unwrap().chosen, None);

        texts(&world, &mut state);
        assert_eq!(state.window.as_ref().unwrap().chosen, Some(0));

        state.window.as_mut().unwrap().name = "be".into();
        state.set_tab(TroopTab::Imperial);
        texts(&world, &mut state);
        assert_eq!(
            state.window.as_ref().unwrap().chosen,
            Some(0),
            "no row begins with \"be\""
        );
        state.set_tab(TroopTab::Alliance);
        texts(&world, &mut state);
        assert_eq!(state.window.as_ref().unwrap().chosen, Some(1), "Beta");
        assert_eq!(state.window.as_ref().unwrap().name, "be");
    }

    #[test]
    fn escape_closes_the_finder() {
        let mut state = TroopFinderState::default();
        state.open(CockpitFaction::Alliance);
        let galaxy = galaxy();
        let transport = TroopTransportState::default();
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let layout = layout();

        // Run a frame with escape.
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 520.0),
            )),
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            draw_troop_finder(
                ctx,
                &galaxy.world,
                &mut state,
                layout,
                &mut cache,
                &transport,
            );
        });

        assert!(!state.is_open());
    }

    /// Two frames of the Finder; the last frame's texts as (text, top-left,
    /// size, font size, colour).
    fn texts(world: &GameWorld, state: &mut TroopFinderState) -> Vec<(String, egui::Pos2, egui::Vec2, f32, egui::Color32)> {
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let transport = TroopTransportState::default();
        let mut shapes = Vec::new();
        for _ in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 520.0))),
                    ..Default::default()
                },
                |ctx| {
                    let _ = draw_troop_finder(ctx, world, state, layout(), &mut cache, &transport);
                },
            );
            shapes = output.shapes;
        }
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => {
                    let format = &text.galley.job.sections[0].format;
                    Some((text.galley.text().to_owned(), text.pos, text.galley.size(), format.font_id.size, format.color))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_title_labels_and_rows_take_the_originals_fonts_and_places() {
        // FUN_0046ce40: the title centred in (36, 14, 350) in font 5, the
        // label in font 4; the list's text at x 5 from the row's top
        // (+0xe4 = 0x20), grey 0x787878 and the chosen row white (A0
        // 2026-10-29).
        let galaxy = galaxy();
        let mut state = TroopFinderState::default();
        state.open(CockpitFaction::Alliance);
        state.set_tab(TroopTab::Alliance);
        let origin = window_rect(layout()).min;
        let shown = texts(&galaxy.world, &mut state);
        let find = |name: &str| shown.iter().find(|(text, ..)| text == name).cloned().unwrap();
        let (_, pos, size, font, _) = find("Troop Finder");
        assert!((pos.x + size.x / 2.0 - origin.x - 211.0).abs() < 0.01, "{pos:?}");
        assert_eq!(font, crate::theme::original_font(5, 1.0).size);
        assert_eq!(find("Troop Location").3, crate::theme::original_font(4, 1.0).size);
        let (_, pos, _, font, colour) = find("Yavin");
        assert_eq!(pos, origin + egui::vec2(LIST.0 + 5.0, LIST.1));
        assert_eq!(font, crate::theme::original_font(10, 1.0).size);
        // The opening chooses the first row (FUN_0046df90 case 100).
        assert_eq!(colour, egui::Color32::WHITE);
        state.window.as_mut().unwrap().chosen = None;
        let shown = texts(&galaxy.world, &mut state);
        assert_eq!(shown.iter().find(|(text, ..)| text == "Yavin").unwrap().4, ROW_COLOR);
    }

    #[test]
    fn the_bars_down_arrow_scrolls_the_list_ten_pixels_and_moves_its_rows() {
        // FUN_0046ce40's bar (RCDATA 10653) at the list's right, x 374;
        // its down arrow at y 294..303 steps the list's 10. Eight systems
        // of 25-pixel rows (200) overflow the 159-pixel list.
        let mut world = GameWorld::default();
        for index in 0..8 {
            let system = add_system(&mut world, &format!("S{index}"), ControlKind::Controlled(Faction::Alliance));
            add_troop(&mut world, system, 0x1000_0002, true);
        }
        let mut state = TroopFinderState::default();
        state.open(CockpitFaction::Alliance);
        state.set_tab(TroopTab::Alliance);
        let origin = window_rect(layout()).min;
        let arrow = origin + egui::vec2(380.0, 299.0);
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let transport = TroopTransportState::default();
        let mut shapes = Vec::new();
        for pressed in [None, None, Some(true), Some(false), None] {
            crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
            let mut events = vec![egui::Event::PointerMoved(arrow)];
            if let Some(pressed) = pressed {
                events.push(egui::Event::PointerButton {
                    pos: arrow,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                });
            }
            shapes = ctx
                .run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 520.0))),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        let _ = draw_troop_finder(ctx, &world, &mut state, layout(), &mut cache, &transport);
                    },
                )
                .shapes;
        }
        assert_eq!(state.window.as_ref().unwrap().scroll, 10);
        let first = shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text() == "S0" => Some(text.pos),
                _ => None,
            })
            .unwrap();
        assert_eq!(first, origin + egui::vec2(LIST.0 + 5.0, LIST.1 - 10.0));
        // Row 6 (140..165) shows; row 7 (165..190) is below the list.
        let named = |name: &str| {
            shapes.iter().any(|clipped| matches!(&clipped.shape, egui::Shape::Text(text) if text.galley.text() == name))
        };
        assert!(named("S6"));
        assert!(!named("S7"));
        let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow().clone());
        assert!(painted.contains(&(10_685, origin + egui::vec2(374.0, 144.0))), "{painted:?}");
        assert!(painted.contains(&(10_687, origin + egui::vec2(374.0, 294.0))), "{painted:?}");

        // A press at x 370 chooses the row; a wheel notch moves a row
        // (port); held on the arrow, it draws pressed.
        let mut frame = |state: &mut TroopFinderState, at: egui::Pos2, event: Option<egui::Event>| {
            crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
            let mut events = vec![egui::Event::PointerMoved(at)];
            events.extend(event);
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 520.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    let _ = draw_troop_finder(ctx, &world, state, layout(), &mut cache, &transport);
                },
            );
        };
        let press = |at: egui::Pos2, pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        let near_bar = origin + egui::vec2(370.0, LIST.1 + 30.0);
        frame(&mut state, near_bar, Some(press(near_bar, true)));
        frame(&mut state, near_bar, Some(press(near_bar, false)));
        assert_eq!(state.window.as_ref().unwrap().chosen, Some(1));
        let wheel = |delta: f32| egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, delta),
            modifiers: egui::Modifiers::default(),
        };
        frame(&mut state, near_bar, Some(wheel(-1.0)));
        assert_eq!(state.window.as_ref().unwrap().scroll, 35);
        frame(&mut state, near_bar, Some(wheel(1.0)));
        assert_eq!(state.window.as_ref().unwrap().scroll, 10);
        frame(&mut state, arrow, Some(press(arrow, true)));
        frame(&mut state, arrow, None);
        let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow().clone());
        assert!(painted.iter().any(|(id, _)| *id == 10_688), "{painted:?}");
        frame(&mut state, origin, None);
        let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow().clone());
        assert!(!painted.iter().any(|(id, _)| *id == 10_688), "{painted:?}");
        // A press on the bar's track, in the row's height, chooses no row.
        state.window.as_mut().unwrap().chosen = None;
        let track = origin + egui::vec2(380.0, LIST.1 + 120.0);
        frame(&mut state, track, Some(press(track, true)));
        frame(&mut state, track, Some(press(track, false)));
        assert_eq!(state.window.as_ref().unwrap().chosen, None);
        // Scrolled 30, row 0 (-30..-5) is wholly above the list: not drawn.
        state.window.as_mut().unwrap().scroll = 30;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 520.0))),
                ..Default::default()
            },
            |ctx| {
                let _ = draw_troop_finder(ctx, &world, &mut state, layout(), &mut cache, &transport);
            },
        );
        assert!(!output
            .shapes
            .iter()
            .any(|clipped| matches!(&clipped.shape, egui::Shape::Text(text) if text.galley.text() == "S0")));
    }

    #[test]
    fn at_scale_two_the_row_text_keeps_its_window_place() {
        let galaxy = galaxy();
        let layout = CockpitLayout {
            scale: 2.0,
            canvas: CockpitViewport {
                x: 0.0,
                y: 0.0,
                width: 1280.0,
                height: 960.0,
            },
            galaxy: CockpitViewport {
                x: 130.0,
                y: 120.0,
                width: 970.0,
                height: 700.0,
            },
        };
        let mut state = TroopFinderState::default();
        state.open(CockpitFaction::Alliance);
        state.set_tab(TroopTab::Alliance);
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let transport = TroopTransportState::default();
        let mut shapes = Vec::new();
        for _ in 0..2 {
            shapes = ctx
                .run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1100.0))),
                        ..Default::default()
                    },
                    |ctx| {
                        let _ = draw_troop_finder(ctx, &galaxy.world, &mut state, layout, &mut cache, &transport);
                    },
                )
                .shapes;
        }
        let yavin = shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text() == "Yavin" => Some(text.pos),
                _ => None,
            })
            .unwrap();
        assert_eq!(yavin, window_rect(layout).min + egui::vec2(LIST.0 + 5.0, LIST.1) * 2.0);
        // The bar reads the pointer in window pixels: eight systems, a click
        // on the down arrow at scale 2 steps 10.
        let mut world = GameWorld::default();
        for index in 0..8 {
            let system = add_system(&mut world, &format!("S{index}"), ControlKind::Controlled(Faction::Alliance));
            add_troop(&mut world, system, 0x1000_0002, true);
        }
        let arrow = window_rect(layout).min + egui::vec2(380.0, 299.0) * 2.0;
        for pressed in [None, None, Some(true), Some(false)] {
            let mut events = vec![egui::Event::PointerMoved(arrow)];
            events.extend(pressed.map(|pressed| egui::Event::PointerButton {
                pos: arrow,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::default(),
            }));
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1100.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    let _ = draw_troop_finder(ctx, &world, &mut state, layout, &mut cache, &transport);
                },
            );
        }
        assert_eq!(state.window.as_ref().unwrap().scroll, 10);
    }

    #[test]
    fn the_finder_blocks_only_its_own_rectangle() {
        let mut state = TroopFinderState::default();
        let rect = window_rect(layout());
        assert!(!state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));

        state.open(CockpitFaction::Alliance);

        assert!(state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));
        assert!(!state.contains_screen_point(layout(), (rect.max.x + 1.0, rect.center().y)));
    }

    #[test]
    fn count_troops_tallies_by_class_for_the_tabs_side() {
        // FUN_0046d8d0: five columns, each the count of its DatId.
        let mut world = GameWorld::default();
        let system = add_system(
            &mut world,
            "Yavin",
            ControlKind::Controlled(Faction::Alliance),
        );
        let k1 = add_troop(&mut world, system, 0x1000_0002, true); // col 0
        let k2 = add_troop(&mut world, system, 0x1000_0002, true); // col 0
        let k3 = add_troop(&mut world, system, 0x1000_0004, true); // col 4
        let k4 = add_troop(&mut world, system, 0x1000_0008, false); // empire, ignored

        let keys = vec![k1, k2, k3, k4];
        let counts = count_troops(&world, &keys, TroopTab::Alliance);
        assert_eq!(counts[0].count, 2);
        assert_eq!(counts[1].count, 0);
        assert_eq!(counts[4].count, 1);
        // port: all usable.
        assert!(counts[0].all_usable);
    }

    #[test]
    fn the_troop_finder_opens_on_the_players_side_tab() {
        // FUN_0046ce40: FUN_0060d7e0(+0x144, side, 1) opens the player's tab.
        let mut state = TroopFinderState::default();
        state.open(CockpitFaction::Alliance);
        assert_eq!(state.window.as_ref().unwrap().tab, TroopTab::Alliance);

        let mut state = TroopFinderState::default();
        state.open(CockpitFaction::Empire);
        assert_eq!(state.window.as_ref().unwrap().tab, TroopTab::Imperial);
    }
}
