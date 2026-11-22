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
use rebellion_core::dat::Faction;
use rebellion_core::fog::FogState;
use rebellion_core::ids::{FleetKey, SystemKey, TroopKey};
use rebellion_core::troop_transport::TroopTransportState;
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::fleet_window::paint_native;
use crate::mission_dialog::{button, galaxy_centered_rect};
use crate::system_window::{fleet_label, logical_rect, opposing_contents_visible, rect_contains};

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

    /// The tab's tooltip (`FUN_0046ce40`): TEXTSTRA 0x1950 "Alliance
    /// Troops", 0x1951 "Imperial Troops".
    const fn tooltip_string(self) -> u16 {
        match self {
            Self::Alliance => 0x1950,
            Self::Imperial => 0x1951,
        }
    }

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
/// mode 3, existing, over the player's side's views of the systems.
///
/// A0 troop-other-side/test-day220-imperial-tab-empty (an unedited
/// Alliance game, day 220) lists no Imperial regiment. The other side's
/// tab walks the player's side's copies (`FUN_004f2d10`, `FUN_004f9a60`):
/// each system with copies of that side's regiments on its planet
/// ([`viewed_rows`]). port: copies aboard a fleet are not listed (the port
/// keeps no copies of fleets); the debug "see everything" mode lists the
/// live regiments.
#[must_use]
pub fn rows(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    tab: TroopTab,
    transport: &TroopTransportState,
) -> Vec<TroopFinderRow> {
    if !tab.lists(player == Faction::Alliance) && !crate::viewed_contents::sees_everything(world) {
        return viewed_rows(world, player, tab);
    }
    let mut rows = Vec::new();

    for (system_key, system) in &world.systems {
        if !tab.lists(player == Faction::Alliance)
            && !opposing_contents_visible(world, fog, player, system_key)
        {
            continue;
        }
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

/// The other side's tab from `player`'s copies: each system with copies of
/// the tab side's regiments on its planet, counted by class, in name order.
fn viewed_rows(world: &GameWorld, player: Faction, tab: TroopTab) -> Vec<TroopFinderRow> {
    let classes = troop_classes(tab);
    let mut rows = Vec::new();
    for (system_key, system) in &world.systems {
        let mut counts = [TroopCount {
            count: 0,
            all_usable: true,
        }; 5];
        let mut any = false;
        for regiment in
            crate::viewed_contents::viewed_surface_regiments(world, player, system_key)
        {
            let (class, is_alliance) = match regiment {
                crate::viewed_contents::ViewedRegiment::Live(key) => {
                    let Some(troop) = world.troops.get(key) else {
                        continue;
                    };
                    (troop.class_dat_id, troop.is_alliance)
                }
                crate::viewed_contents::ViewedRegiment::View(id) => {
                    let Some(view) = crate::viewed_contents::regiment_view(world, player, id) else {
                        continue;
                    };
                    (view.class, view.is_alliance)
                }
            };
            if !tab.lists(is_alliance) {
                continue;
            }
            any = true;
            if let Some(i) = classes.iter().position(|raw| *raw == class.raw()) {
                counts[i].count += 1;
            }
        }
        if any {
            rows.push(TroopFinderRow {
                name: system.name.clone(),
                system: system_key,
                fleet: None,
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
    /// The object Display and Enter open, `+0x17c`: set by a typed name
    /// (`0x408`, `FUN_0046e5f0`), a click or a key step. A tab's rebuild
    /// (`FUN_0046ea10`) leaves it, and case `100` of `FUN_0046df90`
    /// replaces it only when `FUN_00609650` finds a row, so an empty list
    /// keeps the one chosen before.
    target: Option<TroopFinderRow>,
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
            target: None,
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

/// The scroll after a tab's rebuild and rematch: the layout clamps the
/// list's first visible pixel to the new range (`FUN_0060a280` →
/// `FUN_00609ae0`; the rebuild, `FUN_0046ea10`, never sets it), then
/// `FUN_00609650` scrolls as little as shows its row (`FUN_00609ed0`).
fn rebuilt_scroll(scroll: usize, choice: Option<usize>, rows: usize) -> usize {
    let scroll = scroll.min(max_scroll(rows));
    choice.map_or(scroll, |row| scrolled_to(scroll, row, rows))
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
    fog: &FogState,
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
    let player = match window.faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    };
    let list = rows(world, fog, player, window.tab, transport);
    if window.rematch {
        // FUN_0046df90 case 100: FUN_00609650 chooses and scrolls to it.
        window.rematch = false;
        window.chosen = prefix_choice(&list, &window.name);
        if let Some(index) = window.chosen {
            window.target = Some(list[index].clone());
        }
        window.scroll = rebuilt_scroll(window.scroll, window.chosen, list.len());
        state.window.clone_from(&Some(window.clone()));
    }

    let mut close = false;
    let mut open_action: Option<TroopFinderAction> = None;
    let mut tab = None;
    let mut clicked_row = None;
    let mut step = None;
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
            // (0x1901) in font 4, the tab's label in font 5; the list in
            // font 10 (A0 2026-10-29), the name box in font 4
            // (crate::name_box).
            crate::theme::paint_gdi_text(
                &painter,
                at(frame, scale, (TITLE_AT.0, TITLE_AT.1, 350.0, 17.0)),
                scale,
                5,
                "Troop Finder",
                egui::Color32::WHITE,
                (crate::theme::GdiAlign::Center, crate::theme::GdiVAlign::Top),
            );
            crate::theme::paint_gdi_text(
                &painter,
                at(
                    frame,
                    scale,
                    (NAME_LABEL_AT.0, NAME_LABEL_AT.1, 100.0, 16.0),
                ),
                scale,
                4,
                "Troop Location",
                egui::Color32::WHITE,
                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
            );
            crate::theme::paint_gdi_text(
                &painter,
                at(frame, scale, (TAB_LABEL_AT.0, TAB_LABEL_AT.1, 300.0, 16.0)),
                scale,
                5,
                window.tab.label(),
                egui::Color32::WHITE,
                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
            );

            // Side tabs.
            for candidate in TroopTab::ALL {
                let control = TABS[tab_index(candidate)];
                let down = candidate == window.tab;
                crate::win_tooltip::window_tool(
                    ui,
                    at(frame, scale, control.rect),
                    ("troop-tab", tab_index(candidate)),
                    candidate.tooltip_string(),
                    true,
                );
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
            for (control, id, string) in [
                (
                    controls.close,
                    "troop-close",
                    crate::system_finder::CLOSE_TOOLTIP,
                ),
                (
                    controls.display,
                    "troop-display",
                    crate::system_finder::DISPLAY_TOOLTIP,
                ),
            ] {
                crate::win_tooltip::window_tool(
                    ui,
                    at(frame, scale, control.rect),
                    id,
                    string,
                    true,
                );
            }
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
                // Case 0xc9: +0x17c.
                if let Some(row) = &window.target {
                    open_action = Some(action_for_row(row));
                }
            }

            // The name box (0xcb).
            let name_rect = at(frame, scale, NAME_BOX);
            // A CoolStringField in the default font 4 (crate::name_box).
            let edit = crate::name_box::show(
                ui,
                name_rect,
                scale,
                egui::Id::new("original-troop-finder-name"),
                &mut name,
            );
            if window.focus_name {
                edit.request_focus();
            }
            name_changed = edit.changed();
            enter = edit.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            step = crate::list_prefix::key_step(ui, edit.has_focus());

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
                crate::theme::paint_gdi_text(
                    &list_painter,
                    egui::Rect::from_min_max(
                        full_row.min + egui::vec2(ROW_TEXT_X * scale, 0.0),
                        full_row.max,
                    ),
                    scale,
                    10,
                    &row.name,
                    name_colour,
                    (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
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
        // FUN_0046e5f0 0x408: +0x17c takes the best row's object, or is
        // emptied with no row.
        let choice = prefix_choice(&list, &name);
        let before = open_window.chosen;
        open_window.chosen = choice;
        open_window.target = choice.map(|index| list[index].clone());
        if let Some(index) = crate::list_prefix::row_to_show(choice, before) {
            open_window.scroll = scrolled_to(open_window.scroll, index, list.len());
        }
    }
    if let Some((row, at_time)) = clicked_row {
        open_window.last_clicked = Some((row, at_time));
        open_window.chosen = Some(row);
        open_window.target = Some(list[row].clone());
        open_window.name.clone_from(&list[row].name);
    }
    // A key steps the chosen row and copies its name, as a click does (A0
    // finder-keys/, the Fleet Finder's list).
    if let Some(step) = step {
        if let Some(row) = crate::list_prefix::stepped_row(open_window.chosen, list.len(), step) {
            open_window.chosen = Some(row);
            open_window.target = Some(list[row].clone());
            open_window.name = list[row].name.clone();
            open_window.scroll = scrolled_to(open_window.scroll, row, list.len());
        }
    }
    if enter {
        // 0x407: +0x17c.
        if let Some(row) = &open_window.target {
            open_action = Some(action_for_row(row));
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
#[path = "troop_finder_tests.rs"]
mod tests;
