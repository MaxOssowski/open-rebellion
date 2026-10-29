//! The Fleet Finder (window type `0x15`, `FUN_00461750`;
//! `ghidra/notes/fleet-finder.md`).
//!
//! A 470 by 330 window the cockpit's Fleet Finder control (`0x12e`) or F3
//! opens (`FUN_0042a0c0`). It lists the side's fleets, or in its Ship
//! Finder mode their capital ships, under three tabs (all, Alliance,
//! Imperial), sorted by name (`FUN_00462be0`). Typing a name picks the row
//! it best begins (`FUN_00609650`); Display, Enter or a double click opens
//! the Sector and Fleet windows for the choice (`FUN_00429440`).

use egui_macroquad::egui;
use rebellion_core::dat::Faction;
use rebellion_core::fog::FogState;
use rebellion_core::movement::{listed_location, MovementState};
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::fleet_window::{paint_native, FleetWindowEntry};
use crate::mission_dialog::{button, galaxy_centered_rect};
use crate::system_window::{fleet_label, logical_rect, opposing_contents_visible, rect_contains};

pub const FLEET_FINDER_WIDTH: f32 = 470.0;
pub const FLEET_FINDER_HEIGHT: f32 = 330.0;

// STRATEGY bitmaps by side, Alliance then Empire (FUN_00461960).
const BACKGROUND: [u32; 2] = [10335, 10336];
const FLEET_PANEL: [u32; 2] = [10526, 10527];
const SHIP_PANEL: [u32; 2] = [10524, 10525];
const RAIL: [u32; 2] = [10586, 10590];
const PANEL_AT: (f32, f32) = (12.0, 13.0);
const RAIL_AT: (f32, f32) = (412.0, 0.0);

/// One bitmap button: its window rectangle and its normal and pressed art.
#[derive(Debug, Clone, Copy)]
struct Control {
    rect: (f32, f32, f32, f32),
    art: (u32, u32),
}

/// The side's Close (`200`), Display (`0xc9`), Ship Finder and Fleet Finder
/// (group `0xfa`, tags 2 and 1) controls (`FUN_00461960`).
struct SideControls {
    close: Control,
    display: Control,
    ship_finder: Control,
    fleet_finder: Control,
}

const CONTROLS: [SideControls; 2] = [
    SideControls {
        close: Control {
            rect: (423.0, 25.0, 32.0, 31.0),
            art: (10514, 10515),
        },
        display: Control {
            rect: (423.0, 93.0, 32.0, 31.0),
            art: (10518, 10519),
        },
        ship_finder: Control {
            rect: (423.0, 147.0, 32.0, 31.0),
            art: (10530, 10531),
        },
        fleet_finder: Control {
            rect: (423.0, 201.0, 32.0, 31.0),
            art: (10528, 10529),
        },
    },
    SideControls {
        close: Control {
            rect: (426.0, 21.0, 44.0, 41.0),
            art: (10516, 10517),
        },
        display: Control {
            rect: (426.0, 89.0, 44.0, 41.0),
            art: (10520, 10521),
        },
        ship_finder: Control {
            rect: (426.0, 143.0, 44.0, 41.0),
            art: (10534, 10535),
        },
        fleet_finder: Control {
            rect: (426.0, 197.0, 44.0, 41.0),
            art: (10532, 10533),
        },
    },
];

/// The side tabs (group `100`) at (36, 78): All, Alliance, Imperial.
const TABS: [Control; 3] = [
    Control {
        rect: (36.0, 78.0, 49.0, 41.0),
        art: (10500, 10501),
    },
    Control {
        rect: (88.0, 78.0, 49.0, 41.0),
        art: (10502, 10503),
    },
    Control {
        rect: (140.0, 78.0, 49.0, 41.0),
        art: (10505, 10506),
    },
];

// Window rectangles (x, y, width, height), FUN_00461960.
const TITLE_AT: (f32, f32) = (36.0, 14.0);
const NAME_LABEL_AT: (f32, f32) = (36.0, 48.0);
const NAME_BOX: (f32, f32, f32, f32) = (143.0, 45.0, 250.0, 16.0);
const TAB_LABEL_AT: (f32, f32) = (40.0, 119.0);
const LIST: (f32, f32, f32, f32) = (36.0, 138.0, 350.0, 165.0);
/// `FUN_00607ea0(.., 0xcc, 350, 20)`: each row is 350 by 20.
const ROW_HEIGHT: f32 = 20.0;
const ROW_PIXELS: usize = 20;
/// `FUN_00461960`: the list's text at x 5 (`+0xe8`), centred on its row
/// (`+0xe4 = 0x24`), in font 10, grey `0x787878` and the chosen row white
/// (`FUN_00607ea0`'s colours), as the Encyclopedia's list (A0 2026-10-29).
const ROW_TEXT_X: f32 = 5.0;
const ROW_COLOR: egui::Color32 = egui::Color32::from_rgb(0x78, 0x78, 0x78);
/// The list's arrow step, `FUN_00607ea0`'s argument 10, and its bar,
/// `FUN_0060a490(.., list, 7, 0x299d, ..)`: RCDATA 10653.
const LIST_STEP: f32 = 10.0;

/// Fleets (mode 1) or capital ships (mode 2, the Ship Finder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FinderMode {
    #[default]
    Fleets,
    Ships,
}

/// The side tab (`FUN_00462be0`'s 1, 2, 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FinderTab {
    #[default]
    All,
    Alliance,
    Imperial,
}

impl FinderTab {
    const ALL: [Self; 3] = [Self::All, Self::Alliance, Self::Imperial];

    /// Whether an object of `is_alliance` is listed: side bits `0x40` and
    /// `0x80` (`+0x24` bits 6-7).
    fn lists(self, is_alliance: bool) -> bool {
        match self {
            Self::All => true,
            Self::Alliance => is_alliance,
            Self::Imperial => !is_alliance,
        }
    }

    /// TEXTSTRA `0x1882..0x1884`, `0x1887..0x1889` for ships.
    const fn label(self, mode: FinderMode) -> &'static str {
        match (mode, self) {
            (FinderMode::Fleets, Self::All) => "All Fleets",
            (FinderMode::Fleets, Self::Alliance) => "Alliance Fleets",
            (FinderMode::Fleets, Self::Imperial) => "Imperial Fleets",
            (FinderMode::Ships, Self::All) => "All Ships",
            (FinderMode::Ships, Self::Alliance) => "Alliance Ships",
            (FinderMode::Ships, Self::Imperial) => "Imperial Ships",
        }
    }
}

impl FinderMode {
    /// TEXTSTRA `0x1880`/`0x1885` and `0x1881`/`0x1886`.
    const fn titles(self) -> (&'static str, &'static str) {
        match self {
            Self::Fleets => ("Fleet Finder", "Fleet Name"),
            Self::Ships => ("Ship Finder", "Ship Name"),
        }
    }
}

/// One list row: its object and name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinderRow {
    /// A fleet, or a capital ship by its index in its fleet's
    /// `capital_ships`: the Fleet window entry the open selects.
    pub object: FleetWindowEntry,
    pub name: String,
}

/// The rows `FUN_00462be0` lists for `mode` and `tab`, in name order
/// ignoring case (`FUN_0060a890` mode 2, `_stricmp`), ties in walk order.
///
/// hyp: the side's objects (`FUN_0053ef50`) are its own and the other
/// side's it can see; the port shows the other side's fleets where the
/// Fleet window does (`opposing_contents_visible`), at the system the
/// Fleet window lists it at: a fleet en route at its destination
/// (`listed_location`). A fleet's name is its own
/// ("Fleet N", `fleet_label`), a ship's its class's (`FUN_004f6270`).
#[must_use]
pub fn rows(
    world: &GameWorld,
    movement: &MovementState,
    fog: &FogState,
    player: Faction,
    mode: FinderMode,
    tab: FinderTab,
) -> Vec<FinderRow> {
    let player_is_alliance = player == Faction::Alliance;
    let mut rows = Vec::new();
    for (fleet, value) in &world.fleets {
        if !tab.lists(value.is_alliance) {
            continue;
        }
        let Some(location) = listed_location(movement, world, fleet) else {
            continue;
        };
        if value.is_alliance != player_is_alliance
            && !opposing_contents_visible(world, fog, player, location)
        {
            continue;
        }
        let Some(label) = fleet_label(world, fleet) else {
            continue;
        };
        match mode {
            FinderMode::Fleets => rows.push(FinderRow {
                object: FleetWindowEntry::Fleet(fleet),
                name: label,
            }),
            FinderMode::Ships => {
                for (index, ship) in value.capital_ships.iter().enumerate() {
                    let Some(class) = world.capital_ship_classes.get(ship.class) else {
                        continue;
                    };
                    if ship.alive {
                        rows.push(FinderRow {
                            object: FleetWindowEntry::Ship { fleet, index },
                            name: class.name.clone(),
                        });
                    }
                }
            }
        }
    }
    rows.sort_by_cached_key(|row| row.name.to_ascii_lowercase());
    rows
}

/// The row a typed name chooses (`FUN_00609650`, [`best_prefix_row`]):
/// the first with the longest prefix in common with `text`, ignoring case;
/// the first row when none shares the first letter.
///
/// [`best_prefix_row`]: crate::list_prefix::best_prefix_row
#[must_use]
pub fn prefix_choice(rows: &[FinderRow], text: &str) -> Option<usize> {
    crate::list_prefix::best_prefix_row(rows.iter().map(|row| row.name.as_str()), text)
}

/// What the Finder asks of the galaxy view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetFinderAction {
    /// Display, Enter or a double click: `FUN_00429440` opens the Sector and
    /// Fleet windows for the object, then the Finder closes (`+0x30`).
    Open(FleetWindowEntry),
}

#[derive(Debug, Clone)]
struct OpenFinder {
    faction: CockpitFaction,
    mode: FinderMode,
    tab: FinderTab,
    /// The name box's text (`0xcd`).
    name: String,
    /// `+0x16c`: the chosen row's object.
    chosen: Option<FleetWindowEntry>,
    /// The list's first visible pixel (it scrolls by pixels,
    /// `FUN_0060a490`).
    scroll: usize,
    /// A press held on the list's scroll bar.
    scroll_hold: Option<crate::list_scroll_bar::Hold>,
    /// The name box takes the focus when the window opens.
    focus_name: bool,
    /// The row the last click chose and when: a second click on it soon
    /// after is the list's double click (0x309). port: egui counts a double
    /// click across widgets, and a third click as a triple, so the list
    /// keeps its own.
    last_clicked: Option<(usize, f64)>,
    /// A tab's notification (case `100`, `FUN_00462770`) is due: once the
    /// list is built, the name box's text chooses its row (`FUN_00609650`),
    /// the first when none matches. The window opens without it
    /// (`FUN_00461960` fills the list with `FUN_00462be0` directly).
    rematch: bool,
}

/// What the open Finder shows, for the interface fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetFinderReport {
    pub mode: FinderMode,
    pub tab: FinderTab,
    pub name: String,
    pub rows: Vec<FinderRow>,
    pub chosen: Option<usize>,
    pub first_row: usize,
}

/// The Fleet Finder, one at most (`FUN_0042a0c0`).
#[derive(Debug, Clone, Default)]
pub struct FleetFinderState {
    window: Option<OpenFinder>,
}

impl FleetFinderState {
    /// Open the Finder in Fleet Finder mode on its All tab with the focus in
    /// the name box (`FUN_00461960`). An open Finder stays as it is.
    pub fn open(&mut self, faction: CockpitFaction) -> bool {
        if self.window.is_some() {
            return false;
        }
        self.window = Some(OpenFinder {
            faction,
            mode: FinderMode::Fleets,
            tab: FinderTab::All,
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

    /// Whether `point` falls on the open window, so the galaxy map under it
    /// takes no input.
    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        self.window.is_some() && rect_contains(window_rect(layout), egui::pos2(point.0, point.1))
    }

    #[must_use]
    pub fn report(
        &self,
        world: &GameWorld,
        movement: &MovementState,
        fog: &FogState,
        player: Faction,
    ) -> Option<FleetFinderReport> {
        let window = self.window.as_ref()?;
        let rows = rows(world, movement, fog, player, window.mode, window.tab);
        let chosen = window
            .chosen
            .and_then(|object| rows.iter().position(|row| row.object == object));
        Some(FleetFinderReport {
            mode: window.mode,
            tab: window.tab,
            name: window.name.clone(),
            rows,
            chosen,
            first_row: window.scroll / ROW_PIXELS,
        })
    }

    /// Where a control lies on screen: a tab, the Close, Display, Ship
    /// Finder or Fleet Finder button, the name box, or a row the list shows,
    /// for the interface fixture.
    #[must_use]
    pub fn control_screen_rect(
        &self,
        layout: CockpitLayout,
        control: FinderControl,
    ) -> Option<egui::Rect> {
        let window = self.window.as_ref()?;
        let side = &CONTROLS[side(window.faction)];
        let rect = match control {
            FinderControl::Tab(tab) => TABS[tab_index(tab)].rect,
            FinderControl::Close => side.close.rect,
            FinderControl::Display => side.display.rect,
            FinderControl::ShipFinder => side.ship_finder.rect,
            FinderControl::FleetFinder => side.fleet_finder.rect,
            FinderControl::NameBox => NAME_BOX,
            FinderControl::Row(row) => {
                // A row wholly in view.
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

    /// Show `tab` (group `100`, `FUN_00462770`): the list rebuilds
    /// (`FUN_00462be0`), then the name box's text chooses a row.
    pub fn set_tab(&mut self, tab: FinderTab) {
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

    /// `FUN_004632d0`: the name box empties and the list rebuilds for the
    /// tab in use.
    pub fn set_mode(&mut self, mode: FinderMode) {
        if let Some(window) = &mut self.window {
            if window.mode != mode {
                window.mode = mode;
                window.name.clear();
                window.chosen = None;
                window.scroll = 0;
                window.last_clicked = None;
            }
        }
    }
}

/// A Finder control the interface fixture locates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinderControl {
    Tab(FinderTab),
    Close,
    Display,
    ShipFinder,
    FleetFinder,
    NameBox,
    Row(usize),
}

const fn side(faction: CockpitFaction) -> usize {
    match faction {
        CockpitFaction::Alliance => 0,
        CockpitFaction::Empire => 1,
    }
}

const fn tab_index(tab: FinderTab) -> usize {
    match tab {
        FinderTab::All => 0,
        FinderTab::Alliance => 1,
        FinderTab::Imperial => 2,
    }
}

const fn player_side(faction: CockpitFaction) -> Faction {
    match faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    }
}

/// hyp: `FUN_00606980` places it in the galaxy view; the port centers it,
/// as the move confirmation.
fn window_rect(layout: CockpitLayout) -> egui::Rect {
    galaxy_centered_rect(layout, FLEET_FINDER_WIDTH, FLEET_FINDER_HEIGHT)
}

fn at(frame: egui::Rect, scale: f32, (x, y, w, h): (f32, f32, f32, f32)) -> egui::Rect {
    logical_rect(frame, scale, x, y, w, h)
}

/// Rows the list shows at once.
/// The list's height in pixels, and the bar's width (RCDATA 10653).
const LIST_PIXELS: usize = 165;
const BAR_WIDTH: f32 = 12.0;

/// Draw the open Finder, if any, and report an Open.
#[expect(
    clippy::too_many_lines,
    reason = "The window paints in the original's control order; splitting it hides that order."
)]
pub fn draw_fleet_finder(
    ctx: &egui::Context,
    world: &GameWorld,
    movement: &MovementState,
    fog: &FogState,
    state: &mut FleetFinderState,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Option<FleetFinderAction> {
    let mut window = state.window.clone()?;
    let side = side(window.faction);
    let controls = &CONTROLS[side];
    let scale = layout.scale;
    let rect = window_rect(layout);
    let list = rows(
        world,
        movement,
        fog,
        player_side(window.faction),
        window.mode,
        window.tab,
    );
    if window.rematch {
        // FUN_00462770 case 100: FUN_00609650 chooses and scrolls to it.
        window.rematch = false;
        let choice = prefix_choice(&list, &window.name);
        window.chosen = choice.map(|index| list[index].object);
        if let Some(index) = choice {
            window.scroll = scrolled_to(window.scroll, index, list.len());
        }
        state.window.clone_from(&Some(window.clone()));
    }
    let chosen_row = window
        .chosen
        .and_then(|object| list.iter().position(|row| row.object == object));

    let mut close = false;
    let mut open = None;
    let mut tab = None;
    let mut mode = None;
    let mut clicked_row = None;
    let mut name = window.name.clone();
    let mut name_changed = false;
    let mut enter = false;
    let mut wheel = 0.0;
    let mut scroll = window.scroll;
    let mut scroll_hold = window.scroll_hold;

    // FUN_0042a0c0 opens the Finder above the galaxy view's modeless child
    // windows. Those windows share Foreground and raise the focused one each
    // frame, so the Finder uses the popup layer, like the mission dialog and
    // game menu.
    let id = egui::Id::new("original-fleet-finder");
    ctx.move_to_top(egui::LayerId::new(egui::Order::Tooltip, id));
    egui::Area::new(id)
        .fixed_pos(rect.min)
        .order(egui::Order::Tooltip)
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
            paint(cache, BACKGROUND[side], (0.0, 0.0));
            let panel = match window.mode {
                FinderMode::Fleets => FLEET_PANEL[side],
                FinderMode::Ships => SHIP_PANEL[side],
            };
            paint(cache, panel, PANEL_AT);
            paint(cache, RAIL[side], RAIL_AT);

            // The title in font 5, centred in its 350 (A0 2026-10-29), the
            // name label in font 4, the tab's label in font 5, all white
            // (0x2ffffff); the list and the name box in font 10.
            let (title, label) = window.mode.titles();
            let title_font = crate::theme::original_font(5, scale);
            let label_font = crate::theme::original_font(4, scale);
            let font = crate::theme::original_font(10, scale);
            painter.text(
                at(frame, scale, (TITLE_AT.0 + 175.0, TITLE_AT.1, 0.0, 0.0)).min,
                egui::Align2::CENTER_TOP,
                title,
                title_font.clone(),
                egui::Color32::WHITE,
            );
            painter.text(
                at(frame, scale, (NAME_LABEL_AT.0, NAME_LABEL_AT.1, 0.0, 0.0)).min,
                egui::Align2::LEFT_TOP,
                label,
                label_font,
                egui::Color32::WHITE,
            );
            painter.text(
                at(frame, scale, (TAB_LABEL_AT.0, TAB_LABEL_AT.1, 0.0, 0.0)).min,
                egui::Align2::LEFT_TOP,
                window.tab.label(window.mode),
                title_font,
                egui::Color32::WHITE,
            );

            // hyp: the tab and mode groups show their current member in its
            // second bitmap, as the mission dialog's tab strip does
            // (FUN_0060d590); their pressed art is untraced.
            for candidate in FinderTab::ALL {
                let control = TABS[tab_index(candidate)];
                let down = candidate == window.tab;
                if button(
                    ui,
                    cache,
                    at(frame, scale, control.rect),
                    ("finder-tab", tab_index(candidate)),
                    control.art,
                    down,
                    scale,
                ) {
                    tab = Some(candidate);
                }
            }
            for (candidate, control, id) in [
                (FinderMode::Ships, controls.ship_finder, "finder-ships"),
                (FinderMode::Fleets, controls.fleet_finder, "finder-fleets"),
            ] {
                if button(
                    ui,
                    cache,
                    at(frame, scale, control.rect),
                    id,
                    control.art,
                    candidate == window.mode,
                    scale,
                ) {
                    mode = Some(candidate);
                }
            }
            if button(
                ui,
                cache,
                at(frame, scale, controls.close.rect),
                "finder-close",
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
                "finder-display",
                controls.display.art,
                false,
                scale,
            ) {
                open = window.chosen;
            }

            // The name box (0xcd): white text, no frame.
            let name_rect = at(frame, scale, NAME_BOX);
            let edit = ui.put(
                name_rect,
                egui::TextEdit::singleline(&mut name)
                    .id(egui::Id::new("original-fleet-finder-name"))
                    .frame(false)
                    .font(font.clone())
                    .text_color(egui::Color32::WHITE)
                    .desired_width(name_rect.width()),
            );
            if window.focus_name {
                edit.request_focus();
            }
            name_changed = edit.changed();
            enter = edit.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));

            // The list (0xcc), scrolled by pixels with its bar.
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
                ((point.x - frame.min.x) / scale, (point.y - frame.min.y) / scale)
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
                    ui.id().with(("finder-row", offset)),
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
                        open = Some(row.object);
                    } else {
                        clicked_row = Some((offset, now));
                    }
                }
            }
            // The bar, the held arrow pressed while the pointer stays on it.
            let bar = bar_at(offset_px);
            let held = scroll_hold.map(|hold| hold.part).filter(|part| {
                matches!(
                    part,
                    crate::list_scroll_bar::Part::Up | crate::list_scroll_bar::Part::Down
                ) && down
                    && bar.part_at(pointer_at) == Some(*part)
            });
            let (bar_x, bar_y, bar_w, bar_h) = bar.rect();
            let bar_painter = painter.with_clip_rect(at(frame, scale, (bar_x, bar_y, bar_w, bar_h)));
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

    let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let open_window = state.window.as_mut()?;
    open_window.focus_name = false;
    open_window.scroll = scroll;
    open_window.scroll_hold = scroll_hold;
    if name_changed {
        // FUN_00462a50 0x408: the best prefix match is chosen and shown.
        open_window.name.clone_from(&name);
        let choice = prefix_choice(&list, &name);
        open_window.chosen = choice.map(|index| list[index].object);
        if let Some(index) = choice {
            open_window.scroll = scrolled_to(open_window.scroll, index, list.len());
        }
    }
    if let Some((row, at)) = clicked_row {
        // 0x29b: the row's id is chosen and its name copied into the box.
        open_window.last_clicked = Some((row, at));
        open_window.chosen = Some(list[row].object);
        open_window.name.clone_from(&list[row].name);
    }
    if enter {
        open = open.or(open_window.chosen);
    }
    // port: the original has no wheel; a notch moves a row.
    if wheel != 0.0 {
        open_window.scroll = if wheel > 0.0 {
            open_window.scroll.saturating_sub(ROW_PIXELS)
        } else {
            (open_window.scroll + ROW_PIXELS).min(max_scroll(list.len()))
        };
    }
    if let Some(tab) = tab {
        state.set_tab(tab);
    }
    if let Some(mode) = mode {
        state.set_mode(mode);
    }
    if let Some(object) = open {
        state.close();
        return Some(FleetFinderAction::Open(object));
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

/// The list's scroll range for `rows` rows, in pixels
/// ([`crate::list_scroll_bar::max_first`]).
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
    use crate::cockpit::CockpitViewport;
    use rebellion_core::dat::{ExplorationStatus, SectorGroup};
    use rebellion_core::ids::{DatId, FleetKey, SystemKey};
    use rebellion_core::world::{
        CapitalShipClass, ControlKind, Fleet, Sector, ShipInstance, System,
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

    /// A fleet of one ship per class name, in orbit at `system`.
    fn add_fleet(
        world: &mut GameWorld,
        system: SystemKey,
        is_alliance: bool,
        ships: &[&str],
    ) -> FleetKey {
        let capital_ships = ships
            .iter()
            .map(|name| {
                let class = world.capital_ship_classes.insert(CapitalShipClass {
                    name: (*name).into(),
                    is_alliance,
                    hull: 100,
                    hyperdrive: 1,
                    ..CapitalShipClass::default()
                });
                ShipInstance::new(class, 100, true)
            })
            .collect();
        let fleet = world.insert_fleet(Fleet {
            location: system,
            capital_ships,
            fighters: Vec::new(),
            characters: Vec::new(),
            is_alliance,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[system].fleets.push(fleet);
        fleet
    }

    fn names(rows: &[FinderRow]) -> Vec<&str> {
        rows.iter().map(|row| row.name.as_str()).collect()
    }

    fn row(name: &str) -> FinderRow {
        FinderRow {
            object: FleetWindowEntry::Fleet(FleetKey::default()),
            name: name.into(),
        }
    }

    /// An Alliance system holding an Alliance fleet (Nebulon-B, corellian
    /// Corvette), and an Imperial system the Alliance sees holding an
    /// Imperial fleet (Victory Star Destroyer) and one it cannot see.
    struct Galaxy {
        world: GameWorld,
        fog: FogState,
        own: FleetKey,
        seen: FleetKey,
        unseen: FleetKey,
    }

    fn galaxy() -> Galaxy {
        let mut world = GameWorld::default();
        let home = add_system(
            &mut world,
            "Yavin",
            ControlKind::Controlled(Faction::Alliance),
        );
        let seen_at = add_system(
            &mut world,
            "Sluis Van",
            ControlKind::Controlled(Faction::Empire),
        );
        let hidden = add_system(
            &mut world,
            "Coruscant",
            ControlKind::Controlled(Faction::Empire),
        );
        let own = add_fleet(&mut world, home, true, &["Nebulon-B", "corellian Corvette"]);
        let seen = add_fleet(&mut world, seen_at, false, &["Victory Star Destroyer"]);
        let unseen = add_fleet(&mut world, hidden, false, &["Imperial Star Destroyer"]);
        let mut fog = FogState::new(Faction::Alliance);
        fog.reveal(seen_at);
        Galaxy {
            world,
            fog,
            own,
            seen,
            unseen,
        }
    }

    #[test]
    fn the_ship_finder_lists_living_ships_by_name_ignoring_case() {
        // FUN_00462be0 mode 2 walks capital ships; FUN_0060a890 mode 2
        // orders the rows with _stricmp (FUN_00626ad0).
        let mut galaxy = galaxy();
        galaxy.world.fleets[galaxy.own].capital_ships[0].alive = false;

        let rows = rows(
            &galaxy.world,
            &rebellion_core::movement::MovementState::default(),
            &galaxy.fog,
            Faction::Alliance,
            FinderMode::Ships,
            FinderTab::All,
        );

        assert_eq!(
            names(&rows),
            ["corellian Corvette", "Victory Star Destroyer"]
        );
        assert_eq!(
            rows[0].object,
            FleetWindowEntry::Ship {
                fleet: galaxy.own,
                index: 1
            }
        );
    }

    #[test]
    fn the_fleet_finder_lists_own_fleets_and_the_other_sides_fleets_it_sees() {
        // hyp: FUN_0053ef50 walks the side's view of each object; the port
        // shows the other side where the Fleet window does.
        let galaxy = galaxy();

        let listed: Vec<_> = rows(
            &galaxy.world,
            &rebellion_core::movement::MovementState::default(),
            &galaxy.fog,
            Faction::Alliance,
            FinderMode::Fleets,
            FinderTab::All,
        )
        .into_iter()
        .map(|row| row.object)
        .collect();

        assert_eq!(
            listed,
            [
                FleetWindowEntry::Fleet(galaxy.own),
                FleetWindowEntry::Fleet(galaxy.seen)
            ]
        );
        assert!(!listed.contains(&FleetWindowEntry::Fleet(galaxy.unseen)));
    }

    #[test]
    fn the_side_tabs_list_only_their_sides_objects() {
        // FUN_00462be0: tab 2 needs side bit 0x40, tab 3 bit 0x80.
        let galaxy = galaxy();
        let list = |tab| {
            names(&rows(
                &galaxy.world,
                &rebellion_core::movement::MovementState::default(),
                &galaxy.fog,
                Faction::Alliance,
                FinderMode::Ships,
                tab,
            ))
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
        };

        assert_eq!(
            list(FinderTab::Alliance),
            ["corellian Corvette", "Nebulon-B"]
        );
        assert_eq!(list(FinderTab::Imperial), ["Victory Star Destroyer"]);
    }

    #[test]
    fn a_fleet_made_by_create_fleet_is_listed_with_its_ship() {
        // FUN_005809c0 makes the fleet in the ship's system, where the
        // Finder's walk (FUN_00462be0) meets it.
        use rebellion_core::fleet_join::{create_fleet, roster};
        use rebellion_core::movement::MovementState;
        use rebellion_core::troop_transport::TroopTransportState;
        let mut galaxy = galaxy();
        let chosen = roster(&galaxy.world, galaxy.own).unwrap();
        let made = create_fleet(
            &mut galaxy.world,
            &MovementState::default(),
            &mut TroopTransportState::default(),
            galaxy.own,
            &[1],
            chosen,
            true,
        )
        .unwrap();
        let listed = |mode| {
            rows(
                &galaxy.world,
                &rebellion_core::movement::MovementState::default(),
                &galaxy.fog,
                Faction::Alliance,
                mode,
                FinderTab::Alliance,
            )
        };

        let fleets = listed(FinderMode::Fleets);
        let ships = listed(FinderMode::Ships);

        assert!(fleets
            .iter()
            .any(|row| row.object == FleetWindowEntry::Fleet(made) && row.name == "Fleet 2"));
        assert!(ships.iter().any(|row| row.object
            == FleetWindowEntry::Ship {
                fleet: made,
                index: 0
            }
            && row.name == "corellian Corvette"));
    }

    #[test]
    fn a_fleet_in_hyperspace_is_judged_where_it_is_bound() {
        // move-order.md: a move puts the fleet in its destination at once,
        // so the Finder lists it as the destination's Fleet window does: an
        // unseen enemy fleet bound for a system the player sees is listed.
        let mut galaxy = galaxy();
        let seen_at = galaxy.world.fleets[galaxy.seen].location;
        let hidden = galaxy.world.fleets[galaxy.unseen].location;
        let listed = |galaxy: &Galaxy, movement: &MovementState| {
            rows(
                &galaxy.world,
                movement,
                &galaxy.fog,
                Faction::Alliance,
                FinderMode::Fleets,
                FinderTab::All,
            )
            .iter()
            .any(|row| row.object == FleetWindowEntry::Fleet(galaxy.unseen))
        };
        let mut movement = MovementState::default();
        assert!(!listed(&galaxy, &movement));
        assert!(movement.order(galaxy.unseen, hidden, seen_at, 5));
        galaxy.world.systems[hidden].fleets.clear();
        assert!(listed(&galaxy, &movement));
    }

    #[test]
    fn a_typed_name_chooses_the_first_row_it_best_begins() {
        // FUN_00609650: the first row with the longest common prefix,
        // compared without case (FUN_005f3430 flag 0).
        let rows = [
            row("Calamari Cruiser"),
            row("Corellian Corvette"),
            row("Corellian Gunship"),
        ];

        assert_eq!(prefix_choice(&rows, "cor"), Some(1));
        assert_eq!(prefix_choice(&rows, "CORELLIAN G"), Some(2));
        assert_eq!(prefix_choice(&rows, "cx"), Some(0));
        assert_eq!(prefix_choice(&rows, "Coz"), Some(1));
    }

    #[test]
    fn a_name_no_row_begins_with_chooses_the_first_row() {
        // FUN_00609650 starts its best row at the first (0x609676); the A0
        // Troop Finder opens with its first row chosen under an empty name
        // box (VM, 2026-10-29).
        let rows = [row("Calamari Cruiser"), row("Corellian Corvette")];

        assert_eq!(prefix_choice(&rows, "Nebulon"), Some(0));
        assert_eq!(prefix_choice(&rows, ""), Some(0));
        assert_eq!(prefix_choice(&[], ""), None);
    }

    #[test]
    fn the_list_scrolls_as_little_as_it_must_to_show_a_row() {
        // By pixels in the 165-pixel list (FUN_0060a490): row 8 (160..180)
        // needs 15; twenty rows reach 400 + 5 - 165 = 240 (FUN_00609ae0).
        assert_eq!(scrolled_to(0, 7, 20), 0);
        assert_eq!(scrolled_to(0, 8, 20), 15);
        assert_eq!(scrolled_to(100, 2, 20), 40);
        assert_eq!(scrolled_to(100, 12, 20), 100);
        assert_eq!(scrolled_to(0, 19, 20), 235);
        assert_eq!(max_scroll(20), 240);
        assert_eq!(max_scroll(8), 0);
    }

    #[test]
    fn the_opening_chooses_nothing_and_a_new_tab_chooses_the_rows_first() {
        // FUN_00461960 fills the list itself (FUN_00462be0(1, 1)) and
        // selects its tab without a notification (FUN_0060d7e0(1, 0)), as
        // the A0 capture's grey rows show; a tab's notification
        // (FUN_00462770 case 100) lets the empty name box choose the first
        // row (FUN_00609650).
        let galaxy = galaxy();
        let mut state = FleetFinderState::default();
        state.open(CockpitFaction::Alliance);
        drive(&galaxy, &mut state, (0.0, 0.0), vec![]);
        assert_eq!(state.window.as_ref().unwrap().chosen, None);

        state.set_tab(FinderTab::Alliance);
        drive(&galaxy, &mut state, (0.0, 0.0), vec![]);
        let list = rows(
            &galaxy.world,
            &rebellion_core::movement::MovementState::default(),
            &galaxy.fog,
            player_side(CockpitFaction::Alliance),
            FinderMode::Fleets,
            FinderTab::Alliance,
        );
        assert!(!list.is_empty());
        assert_eq!(state.window.as_ref().unwrap().chosen, Some(list[0].object));
    }

    #[test]
    fn opening_an_open_finder_keeps_its_state() {
        // FUN_0042a0c0 builds the window only when no type 0x15 is open.
        let mut state = FleetFinderState::default();
        assert!(state.open(CockpitFaction::Alliance));
        state.set_tab(FinderTab::Imperial);

        assert!(!state.open(CockpitFaction::Alliance));

        assert_eq!(state.window.as_ref().unwrap().tab, FinderTab::Imperial);
    }

    #[test]
    fn a_new_tab_clears_the_choice_and_a_new_mode_clears_the_name() {
        // FUN_00462be0 clears +0x16c; FUN_004632d0 empties the name box.
        let mut state = FleetFinderState::default();
        state.open(CockpitFaction::Empire);
        let window = state.window.as_mut().unwrap();
        window.chosen = Some(FleetWindowEntry::Fleet(FleetKey::default()));
        window.name = "Fleet".into();

        state.set_tab(FinderTab::Alliance);
        let window = state.window.as_ref().unwrap();
        assert_eq!(window.chosen, None);
        assert_eq!(window.name, "Fleet");

        state.window.as_mut().unwrap().chosen = Some(FleetWindowEntry::Fleet(FleetKey::default()));
        state.set_mode(FinderMode::Ships);
        let window = state.window.as_ref().unwrap();
        assert_eq!(window.name, "");
        assert_eq!(window.chosen, None);
        assert_eq!(window.tab, FinderTab::Alliance);
    }

    fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }
    }

    /// Run two idle frames, then one frame per entry of `frames` with the
    /// pointer at window point `(x, y)`; return the last action.
    fn drive(
        galaxy: &Galaxy,
        state: &mut FleetFinderState,
        at: (f32, f32),
        frames: Vec<Vec<egui::Event>>,
    ) -> Option<FleetFinderAction> {
        drive_at(
            galaxy,
            state,
            frames.into_iter().map(|events| (at, events)).collect(),
        )
    }

    /// [`drive`], with the pointer at its own window point each frame.
    fn drive_at(
        galaxy: &Galaxy,
        state: &mut FleetFinderState,
        frames: Vec<((f32, f32), Vec<egui::Event>)>,
    ) -> Option<FleetFinderAction> {
        let layout = layout();
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let mut emitted = None;
        let first = frames.first().map_or((0.0, 0.0), |(at, _)| *at);
        for ((x, y), extra) in [(first, vec![]), (first, vec![])].into_iter().chain(frames) {
            let pos = window_rect(layout).min + egui::vec2(x, y);
            let mut events = vec![egui::Event::PointerMoved(pos)];
            events.extend(extra.into_iter().map(|event| match event {
                egui::Event::PointerButton { pressed, .. } => press(pos, pressed),
                other => other,
            }));
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(700.0, 520.0),
                )),
                events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                if let Some(action) = draw_fleet_finder(
                    ctx,
                    &galaxy.world,
                    &rebellion_core::movement::MovementState::default(),
                    &galaxy.fog,
                    state,
                    layout,
                    &mut cache,
                ) {
                    emitted = Some(action);
                }
            });
        }
        emitted
    }

    fn click() -> Vec<Vec<egui::Event>> {
        let at = egui::Pos2::ZERO;
        vec![vec![press(at, true)], vec![press(at, false)]]
    }

    fn center((x, y, w, h): (f32, f32, f32, f32)) -> (f32, f32) {
        (x + w / 2.0, y + h / 2.0)
    }

    /// The window point at the middle of list row `row`.
    fn list_row(row: usize) -> (f32, f32) {
        (
            LIST.0 + 40.0,
            LIST.1 + ROW_HEIGHT * row as f32 + ROW_HEIGHT / 2.0,
        )
    }

    fn opened(faction: CockpitFaction) -> FleetFinderState {
        let mut state = FleetFinderState::default();
        state.open(faction);
        state
    }

    #[test]
    fn typing_a_name_then_enter_opens_the_fleet_it_best_matches() {
        // FUN_00462a50: 0x408 chooses the best prefix row; 0x407 opens it.
        let galaxy = galaxy();
        let mut state = opened(CockpitFaction::Alliance);
        state.set_mode(FinderMode::Ships);

        let action = drive(
            &galaxy,
            &mut state,
            (0.0, 0.0),
            vec![
                vec![egui::Event::Text("VIC".into())],
                vec![key(egui::Key::Enter)],
            ],
        );

        assert_eq!(
            action,
            Some(FleetFinderAction::Open(FleetWindowEntry::Ship {
                fleet: galaxy.seen,
                index: 0
            }))
        );
        assert!(!state.is_open());
    }

    #[test]
    fn a_row_click_copies_its_name_and_display_opens_it() {
        // FUN_00462770: 0x29b copies the row's name into 0xcd; 0xc9 opens
        // +0x16c.
        let galaxy = galaxy();
        // The Empire lists its own two fleets; the Alliance its own and the
        // one it sees. Each side numbers its own fleets (FUN_00517760), so
        // the Alliance's row 1 is the Empire's Fleet 1, the Empire's its
        // Fleet 2.
        for (faction, second, name) in [
            (CockpitFaction::Alliance, galaxy.seen, "Fleet 1"),
            (CockpitFaction::Empire, galaxy.unseen, "Fleet 2"),
        ] {
            let mut state = opened(faction);
            assert_eq!(drive(&galaxy, &mut state, list_row(1), click()), None);
            let window = state.window.as_ref().unwrap();
            assert_eq!(window.chosen, Some(FleetWindowEntry::Fleet(second)));
            assert_eq!(window.name, name);

            let display = CONTROLS[side(faction)].display.rect;
            let action = drive(&galaxy, &mut state, center(display), click());

            assert_eq!(
                action,
                Some(FleetFinderAction::Open(FleetWindowEntry::Fleet(second))),
                "{faction:?}"
            );
            assert!(!state.is_open());
        }
    }

    #[test]
    fn display_with_nothing_chosen_keeps_the_finder_open() {
        let galaxy = galaxy();
        let mut state = opened(CockpitFaction::Alliance);
        let display = CONTROLS[0].display.rect;

        assert_eq!(drive(&galaxy, &mut state, center(display), click()), None);
        assert!(state.is_open());
    }

    #[test]
    fn a_double_click_on_a_row_opens_it() {
        // FUN_00462770: list notification 0x309 opens the row.
        let galaxy = galaxy();
        let mut state = opened(CockpitFaction::Alliance);
        let at = egui::Pos2::ZERO;

        let action = drive(
            &galaxy,
            &mut state,
            list_row(0),
            vec![
                vec![press(at, true)],
                vec![press(at, false)],
                vec![press(at, true)],
                vec![press(at, false)],
            ],
        );

        assert_eq!(
            action,
            Some(FleetFinderAction::Open(FleetWindowEntry::Fleet(galaxy.own)))
        );
    }

    #[test]
    fn a_click_elsewhere_then_on_a_row_only_chooses_the_row() {
        // FUN_00462770's 0x309 is the list's own double click: both clicks
        // on the row.
        let galaxy = galaxy();
        let mut state = opened(CockpitFaction::Alliance);
        let at = egui::Pos2::ZERO;
        let display = center(CONTROLS[0].display.rect);

        let action = drive_at(
            &galaxy,
            &mut state,
            vec![
                (display, vec![press(at, true)]),
                (display, vec![press(at, false)]),
                (list_row(0), vec![press(at, true)]),
                (list_row(0), vec![press(at, false)]),
            ],
        );

        assert_eq!(action, None);
        let window = state.window.as_ref().unwrap();
        assert_eq!(window.chosen, Some(FleetWindowEntry::Fleet(galaxy.own)));
    }

    #[test]
    fn a_double_click_on_a_row_opens_it_after_a_click_elsewhere() {
        // egui counts the row's second click as a triple click here; the
        // list's own double click (0x309) is both clicks on the row.
        let galaxy = galaxy();
        let mut state = opened(CockpitFaction::Alliance);
        let at = egui::Pos2::ZERO;
        let display = center(CONTROLS[0].display.rect);
        let mut frames = vec![
            (display, vec![press(at, true)]),
            (display, vec![press(at, false)]),
        ];
        for _ in 0..2 {
            frames.push((list_row(0), vec![press(at, true)]));
            frames.push((list_row(0), vec![press(at, false)]));
        }

        let action = drive_at(&galaxy, &mut state, frames);

        assert_eq!(
            action,
            Some(FleetFinderAction::Open(FleetWindowEntry::Fleet(galaxy.own)))
        );
    }

    #[test]
    fn a_second_click_long_after_the_first_only_chooses_the_row() {
        let galaxy = galaxy();
        let mut state = opened(CockpitFaction::Alliance);
        state.window.as_mut().unwrap().last_clicked = Some((0, -10.0));

        assert_eq!(drive(&galaxy, &mut state, list_row(0), click()), None);

        assert!(state.is_open());
    }

    #[test]
    fn close_and_escape_close_the_finder() {
        // FUN_00462770 200 and FUN_00463360's Escape both close (+0x30).
        let galaxy = galaxy();
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let mut state = opened(faction);
            let close = CONTROLS[side(faction)].close.rect;
            assert_eq!(drive(&galaxy, &mut state, center(close), click()), None);
            assert!(!state.is_open(), "{faction:?}");
        }

        let mut state = opened(CockpitFaction::Alliance);
        drive(
            &galaxy,
            &mut state,
            (0.0, 0.0),
            vec![vec![key(egui::Key::Escape)]],
        );
        assert!(!state.is_open());
    }

    #[test]
    fn the_tabs_and_mode_buttons_switch_the_list() {
        // FUN_0060d590 tabs at (36,78); group 0xfa's Ship Finder above its
        // Fleet Finder (FUN_00461960).
        let galaxy = galaxy();
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let mut state = opened(faction);
            drive(&galaxy, &mut state, center(TABS[2].rect), click());
            assert_eq!(state.window.as_ref().unwrap().tab, FinderTab::Imperial);

            let controls = &CONTROLS[side(faction)];
            drive(
                &galaxy,
                &mut state,
                center(controls.ship_finder.rect),
                click(),
            );
            let report = state
                .report(
                    &galaxy.world,
                    &rebellion_core::movement::MovementState::default(),
                    &galaxy.fog,
                    Faction::Alliance,
                )
                .unwrap();
            assert_eq!(report.mode, FinderMode::Ships, "{faction:?}");
            assert_eq!(names(&report.rows), ["Victory Star Destroyer"]);

            drive(
                &galaxy,
                &mut state,
                center(controls.fleet_finder.rect),
                click(),
            );
            assert_eq!(state.window.as_ref().unwrap().mode, FinderMode::Fleets);
        }
    }

    /// The canvas 10 by 20 pixels in and twice the original size, so offsets
    /// and scale both show.
    fn scaled() -> CockpitLayout {
        CockpitLayout {
            canvas: CockpitViewport {
                x: 10.0,
                y: 20.0,
                width: 1280.0,
                height: 960.0,
            },
            galaxy: CockpitViewport {
                x: 140.0,
                y: 140.0,
                width: 970.0,
                height: 700.0,
            },
            scale: 2.0,
        }
    }

    struct Text {
        text: String,
        pos: egui::Pos2,
        width: f32,
        height: f32,
        size: f32,
        color: egui::Color32,
    }

    #[derive(Default)]
    struct Run {
        action: Option<FleetFinderAction>,
        painted: Vec<(u32, egui::Pos2)>,
        texts: Vec<Text>,
    }

    impl Run {
        fn painted_at(&self, id: u32) -> Option<egui::Pos2> {
            self.painted
                .iter()
                .find(|(painted, _)| *painted == id)
                .map(|(_, at)| *at)
        }
    }

    /// Draw on [`scaled`] for the Alliance's view once per frame, each frame
    /// with the pointer at its window point; the last frame's art and text
    /// are kept.
    fn run(
        galaxy: &Galaxy,
        state: &mut FleetFinderState,
        frames: Vec<((f32, f32), Vec<egui::Event>)>,
    ) -> Run {
        let layout = scaled();
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let mut result = Run::default();
        let first = frames.first().map_or((0.0, 0.0), |(at, _)| *at);
        let frames = [(first, vec![]), (first, vec![])].into_iter().chain(frames);
        for (index, ((x, y), extra)) in frames.enumerate() {
            let pos = window_rect(layout).min + egui::vec2(x, y) * 2.0;
            let mut events = vec![egui::Event::PointerMoved(pos)];
            events.extend(extra.into_iter().map(|event| match event {
                egui::Event::PointerButton { pressed, .. } => press(pos, pressed),
                other => other,
            }));
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1000.0),
                )),
                time: Some(index as f64 * 0.05),
                events,
                ..Default::default()
            };
            crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
            let output = ctx.run(input, |ctx| {
                if let Some(action) = draw_fleet_finder(
                    ctx,
                    &galaxy.world,
                    &rebellion_core::movement::MovementState::default(),
                    &galaxy.fog,
                    state,
                    layout,
                    &mut cache,
                ) {
                    result.action = Some(action);
                }
            });
            result.painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.take());
            result.texts = output
                .shapes
                .into_iter()
                .filter_map(|clipped| match clipped.shape {
                    egui::Shape::Text(text) => {
                        let format = &text.galley.job.sections[0].format;
                        Some(Text {
                            text: text.galley.text().to_owned(),
                            pos: text.pos,
                            width: text.galley.size().x,
                            height: text.galley.size().y,
                            size: format.font_id.size,
                            color: format.color,
                        })
                    }
                    _ => None,
                })
                .collect();
        }
        result
    }

    fn idle(galaxy: &Galaxy, state: &mut FleetFinderState) -> Run {
        run(galaxy, state, vec![((0.0, 0.0), vec![])])
    }

    /// The galaxy with `count` more Alliance fleets at the Alliance's
    /// system, for a list longer than the eight rows it shows.
    fn crowded(count: usize) -> Galaxy {
        let mut galaxy = galaxy();
        let home = galaxy.world.fleets[galaxy.own].location;
        for _ in 0..count {
            add_fleet(&mut galaxy.world, home, true, &["Nebulon-B"]);
        }
        galaxy
    }

    #[test]
    fn each_side_paints_its_own_art_in_place() {
        // FUN_00461960: the base, panel at (12, 13), rail at (412, 0), and
        // the buttons at the side's rectangles; the tabs at (36 + 52k, 78).
        // hyp: the current tab and mode show their second bitmap.
        let galaxy = galaxy();
        for (faction, base, panel, ship_panel, rail, buttons) in [
            (
                CockpitFaction::Alliance,
                10335,
                10526,
                10524,
                10586,
                [
                    (10514, (423.0, 25.0)),
                    (10518, (423.0, 93.0)),
                    (10530, (423.0, 147.0)),
                    (10529, (423.0, 201.0)),
                ],
            ),
            (
                CockpitFaction::Empire,
                10336,
                10527,
                10525,
                10590,
                [
                    (10516, (426.0, 21.0)),
                    (10520, (426.0, 89.0)),
                    (10534, (426.0, 143.0)),
                    (10533, (426.0, 197.0)),
                ],
            ),
        ] {
            let mut state = opened(faction);
            let origin = window_rect(scaled()).min;
            let at = |(x, y): (f32, f32)| Some(origin + egui::vec2(x, y) * 2.0);

            let drawn = idle(&galaxy, &mut state);

            assert_eq!(drawn.painted_at(base), at((0.0, 0.0)), "{faction:?}");
            assert_eq!(drawn.painted_at(panel), at((12.0, 13.0)));
            assert_eq!(drawn.painted_at(rail), at((412.0, 0.0)));
            for (id, point) in buttons {
                assert_eq!(drawn.painted_at(id), at(point), "{faction:?} {id}");
            }
            for (id, x) in [(10501, 36.0), (10502, 88.0), (10505, 140.0)] {
                assert_eq!(drawn.painted_at(id), at((x, 78.0)), "{faction:?} {id}");
            }
            assert_eq!(drawn.painted_at(ship_panel), None);

            state.set_mode(FinderMode::Ships);
            state.set_tab(FinderTab::Imperial);
            let drawn = idle(&galaxy, &mut state);

            assert_eq!(drawn.painted_at(ship_panel), at((12.0, 13.0)));
            assert_eq!(drawn.painted_at(panel), None);
            let (ship, fleet) = (buttons[2], buttons[3]);
            assert_eq!(drawn.painted_at(ship.0 + 1), at(ship.1), "{faction:?}");
            assert_eq!(drawn.painted_at(fleet.0 - 1), at(fleet.1), "{faction:?}");
            for (id, x) in [(10500, 36.0), (10502, 88.0), (10506, 140.0)] {
                assert_eq!(drawn.painted_at(id), at((x, 78.0)), "{faction:?} {id}");
            }
        }
    }

    #[test]
    fn the_title_label_and_tab_name_follow_the_mode_and_tab() {
        // TEXTSTRA 0x1880/0x1885 centred in (36, 14, 350) in font 5 (A0
        // 2026-10-29), 0x1881/0x1886 at (36, 48) in font 4 and the tab's
        // 0x1882..0x1889 at (40, 119) in font 5, white (0x2ffffff).
        let galaxy = galaxy();
        let mut state = opened(CockpitFaction::Empire);
        let origin = window_rect(scaled()).min;
        let shown = |run: &Run, text: &str| {
            run.texts
                .iter()
                .find(|shown| shown.text == text)
                .map(|shown| (shown.pos - origin, shown.size, shown.color))
        };

        let drawn = idle(&galaxy, &mut state);

        let white = egui::Color32::WHITE;
        let font = |id| crate::theme::original_font(id, 2.0).size;
        let title = drawn.texts.iter().find(|shown| shown.text == "Fleet Finder").unwrap();
        assert!(
            (title.pos.x + title.width / 2.0 - origin.x - (36.0 + 175.0) * 2.0).abs() < 0.01,
            "{:?}",
            title.pos
        );
        assert_eq!((title.pos.y - origin.y, title.size, title.color), (28.0, font(5), white));
        assert_eq!(
            shown(&drawn, "Fleet Name"),
            Some((egui::vec2(72.0, 96.0), font(4), white))
        );
        assert_eq!(
            shown(&drawn, "All Fleets"),
            Some((egui::vec2(80.0, 238.0), font(5), white))
        );

        state.set_mode(FinderMode::Ships);
        state.set_tab(FinderTab::Alliance);
        let drawn = idle(&galaxy, &mut state);

        assert!(shown(&drawn, "Ship Finder").is_some());
        assert!(shown(&drawn, "Ship Name").is_some());
        assert!(shown(&drawn, "Alliance Ships").is_some());
        assert!(shown(&drawn, "Fleet Finder").is_none());
        let labels: Vec<_> = [FinderMode::Fleets, FinderMode::Ships]
            .into_iter()
            .flat_map(|mode| FinderTab::ALL.map(|tab| tab.label(mode)))
            .collect();
        assert_eq!(
            labels,
            [
                "All Fleets",
                "Alliance Fleets",
                "Imperial Fleets",
                "All Ships",
                "Alliance Ships",
                "Imperial Ships"
            ]
        );
    }

    #[test]
    fn the_list_shows_its_rows_from_the_scroll_grey_and_the_chosen_one_white() {
        // FUN_00607ea0: rows 350 by 20 from (36, 138), the text at x 5 in
        // font 10, grey 0x787878 and the chosen one white (the
        // constructor's colours; A0 2026-10-29). Scrolled two rows, the
        // 165-pixel list shows eight whole rows and the top of a ninth.
        let galaxy = crowded(9);
        let list = rows(
            &galaxy.world,
            &rebellion_core::movement::MovementState::default(),
            &galaxy.fog,
            Faction::Alliance,
            FinderMode::Fleets,
            FinderTab::All,
        );
        assert_eq!(list.len(), 11);
        let mut state = opened(CockpitFaction::Alliance);
        let window = state.window.as_mut().unwrap();
        window.scroll = 40;
        window.chosen = Some(list[3].object);
        let origin = window_rect(scaled()).min;

        let drawn = idle(&galaxy, &mut state);

        let shown: Vec<_> = drawn
            .texts
            .iter()
            // The rows, below the tab's label at the same x.
            .filter(|text| (text.pos.x - origin.x - 82.0).abs() < 0.01 && text.pos.y > origin.y + 260.0)
            .collect();
        assert_eq!(shown.len(), 9);
        for (line, text) in shown.iter().enumerate() {
            assert_eq!(text.text, list[2 + line].name);
            let middle = text.pos.y + text.height / 2.0 - origin.y;
            assert!(
                (middle - (296.0 + 40.0 * line as f32)).abs() < 0.01,
                "{line}: {middle}"
            );
            let colour = if line == 1 {
                egui::Color32::WHITE
            } else {
                ROW_COLOR
            };
            assert_eq!(text.color, colour, "{line}");
        }
    }

    #[test]
    fn the_bars_down_arrow_scrolls_ten_and_the_report_counts_whole_rows() {
        // FUN_0060a490 (RCDATA 10653) right of the list, x 374: its down
        // arrow at y 294..303 steps the list's 10 when released over it.
        let galaxy = crowded(9);
        let mut state = opened(CockpitFaction::Alliance);
        let arrow = (380.0, 299.0);
        let click = vec![(arrow, vec![press(egui::Pos2::ZERO, true)]), (arrow, vec![press(egui::Pos2::ZERO, false)])];
        let drawn = run(&galaxy, &mut state, click.clone());
        assert_eq!(state.window.as_ref().unwrap().scroll, 10);
        let report = |state: &FleetFinderState| {
            state
                .report(
                    &galaxy.world,
                    &rebellion_core::movement::MovementState::default(),
                    &galaxy.fog,
                    Faction::Alliance,
                )
                .unwrap()
                .first_row
        };
        assert_eq!(report(&state), 0);
        let _ = run(&galaxy, &mut state, [click.clone(), click].concat());
        assert_eq!(state.window.as_ref().unwrap().scroll, 30);
        assert_eq!(report(&state), 1, "30 pixels: the second row is the first");
        let origin = window_rect(scaled()).min;
        assert_eq!(drawn.painted_at(10_685), Some(origin + egui::vec2(374.0, 138.0) * 2.0));
        // Held on it, the arrow draws pressed.
        let mut state = opened(CockpitFaction::Alliance);
        let held = run(&galaxy, &mut state, vec![(arrow, vec![press(egui::Pos2::ZERO, true)]), (arrow, vec![])]);
        assert!(held.painted_at(10_688).is_some());
        let mut state = opened(CockpitFaction::Alliance);
        let off = run(
            &galaxy,
            &mut state,
            vec![(arrow, vec![press(egui::Pos2::ZERO, true)]), ((200.0, 200.0), vec![])],
        );
        assert!(off.painted_at(10_688).is_none());
    }

    #[test]
    fn scrolled_rows_clip_to_the_list_and_the_bars_column_chooses_nothing() {
        // Scrolled 30 pixels, row 0 (108..128) is gone, row 1 shows its
        // lower half, row 9 (288..308) its upper part, row 10 nothing.
        let galaxy = crowded(9);
        let list = rows(
            &galaxy.world,
            &rebellion_core::movement::MovementState::default(),
            &galaxy.fog,
            Faction::Alliance,
            FinderMode::Fleets,
            FinderTab::All,
        );
        let mut state = opened(CockpitFaction::Alliance);
        state.window.as_mut().unwrap().scroll = 30;
        let origin = window_rect(scaled()).min;
        let drawn = idle(&galaxy, &mut state);
        let shown: Vec<_> = drawn
            .texts
            .iter()
            .filter(|text| (text.pos.x - origin.x - 82.0).abs() < 0.01 && text.pos.y > origin.y + 200.0)
            .map(|text| text.text.clone())
            .collect();
        let names: Vec<_> = list[1..10].iter().map(|row| row.name.clone()).collect();
        assert_eq!(shown, names);
        // A click on row 2's height in the bar's column (x 374..386).
        let mut state = opened(CockpitFaction::Alliance);
        let at = (380.0, 138.0 + 50.0);
        let _ = run(
            &galaxy,
            &mut state,
            vec![(at, vec![press(egui::Pos2::ZERO, true)]), (at, vec![press(egui::Pos2::ZERO, false)])],
        );
        assert_eq!(state.window.as_ref().unwrap().chosen, None);
    }

    fn wheel(delta: f32) -> egui::Event {
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, delta),
            modifiers: egui::Modifiers::default(),
        }
    }

    #[test]
    fn the_wheel_over_the_list_scrolls_it_a_row_at_a_time_within_its_rows() {
        // port: the original has no wheel; a notch moves a 20-pixel row,
        // within the bar's range (11 rows: 60 pixels, three rows).
        let galaxy = crowded(9);
        let mut state = opened(CockpitFaction::Alliance);
        let first_row = |state: &FleetFinderState| state.window.as_ref().unwrap().scroll / ROW_PIXELS;
        let over = list_row(3);

        run(&galaxy, &mut state, vec![(over, vec![wheel(-1.0)])]);
        assert_eq!(first_row(&state), 1);
        run(&galaxy, &mut state, vec![(over, vec![wheel(-1.0)]); 4]);
        assert_eq!(first_row(&state), 3, "11 rows, 8 shown");
        run(&galaxy, &mut state, vec![(over, vec![wheel(1.0)])]);
        assert_eq!(first_row(&state), 2);
        run(&galaxy, &mut state, vec![((20.0, 20.0), vec![wheel(-1.0)])]);
        assert_eq!(first_row(&state), 2, "the wheel beside the list");
        run(&galaxy, &mut state, vec![(over, vec![wheel(1.0)]); 4]);
        assert_eq!(first_row(&state), 0);
    }

    #[test]
    fn leaving_the_name_box_without_enter_opens_nothing() {
        // FUN_00462a50: only 0x407, Enter, opens from the name box.
        let galaxy = galaxy();
        let mut state = opened(CockpitFaction::Alliance);
        let at = egui::Pos2::ZERO;

        let drawn = run(
            &galaxy,
            &mut state,
            vec![
                ((150.0, 50.0), vec![egui::Event::Text("Fleet".into())]),
                ((20.0, 300.0), vec![press(at, true)]),
                ((20.0, 300.0), vec![press(at, false)]),
            ],
        );

        assert_eq!(drawn.action, None);
        let window = state.window.as_ref().unwrap();
        assert_eq!(window.name, "Fleet");
        assert!(window.chosen.is_some());
    }

    #[test]
    fn the_report_names_the_chosen_row_by_its_place() {
        let galaxy = galaxy();
        let mut state = opened(CockpitFaction::Alliance);
        state.window.as_mut().unwrap().chosen = Some(FleetWindowEntry::Fleet(galaxy.seen));

        let report = state
            .report(
                &galaxy.world,
                &rebellion_core::movement::MovementState::default(),
                &galaxy.fog,
                Faction::Alliance,
            )
            .unwrap();

        assert_eq!(report.chosen, Some(1));
        assert_eq!(report.rows[1].object, FleetWindowEntry::Fleet(galaxy.seen));
    }

    #[test]
    fn the_controls_lie_where_they_draw_and_rows_above_the_first_are_hidden() {
        let mut state = FleetFinderState::default();
        let layout = scaled();
        assert_eq!(state.screen_rect(layout), None);
        assert_eq!(
            state.control_screen_rect(layout, FinderControl::Close),
            None
        );
        state.open(CockpitFaction::Empire);
        state.window.as_mut().unwrap().scroll = 20;
        let origin = window_rect(layout).min;
        let rect = |(x, y, w, h): (f32, f32, f32, f32)| {
            Some(egui::Rect::from_min_size(
                origin + egui::vec2(x, y) * 2.0,
                egui::vec2(w, h) * 2.0,
            ))
        };

        assert_eq!(state.screen_rect(layout), Some(window_rect(layout)));
        assert_eq!(
            state.control_screen_rect(layout, FinderControl::Row(0)),
            None
        );
        assert!(state
            .control_screen_rect(layout, FinderControl::Row(8))
            .is_some());
        assert_eq!(
            state.control_screen_rect(layout, FinderControl::Row(9)),
            None
        );
        assert_eq!(
            state.control_screen_rect(layout, FinderControl::Row(3)),
            rect((36.0, 178.0, 338.0, 20.0))
        );
        assert_eq!(
            state.control_screen_rect(layout, FinderControl::Close),
            rect((426.0, 21.0, 44.0, 41.0))
        );
        assert_eq!(
            state.control_screen_rect(layout, FinderControl::Tab(FinderTab::Imperial)),
            rect((140.0, 78.0, 49.0, 41.0))
        );
        assert_eq!(
            state.control_screen_rect(layout, FinderControl::NameBox),
            rect((143.0, 45.0, 250.0, 16.0))
        );
    }

    #[test]
    fn the_finder_blocks_only_its_own_rectangle() {
        let mut state = FleetFinderState::default();
        let rect = window_rect(layout());
        assert!(!state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));

        state.open(CockpitFaction::Alliance);

        assert!(state.contains_screen_point(layout(), (rect.center().x, rect.center().y)));
        assert!(!state.contains_screen_point(layout(), (rect.max.x + 1.0, rect.center().y)));
        assert_eq!(rect.size(), egui::vec2(470.0, 330.0));
    }

    #[test]
    fn the_finder_stays_above_a_modeless_window_that_raises_itself_every_frame() {
        // FUN_0042a0c0 opens the Finder over the galaxy view's child windows;
        // a focused Fleet window raises itself every frame (fleet_window.rs).
        let galaxy = galaxy();
        let layout = layout();
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let mut state = opened(CockpitFaction::Alliance);
        let modeless = egui::Id::new("raising-fleet-window");
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(700.0, 520.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                let _ = draw_fleet_finder(
                    ctx,
                    &galaxy.world,
                    &rebellion_core::movement::MovementState::default(),
                    &galaxy.fog,
                    &mut state,
                    layout,
                    &mut cache,
                );
                // A focused child window may raise itself later in the same
                // frame. The original Finder remains the top-level owner.
                ctx.move_to_top(egui::LayerId::new(egui::Order::Foreground, modeless));
                egui::Area::new(modeless)
                    .order(egui::Order::Foreground)
                    .fixed_pos(egui::Pos2::ZERO)
                    .show(ctx, |ui| {
                        ui.allocate_response(egui::vec2(700.0, 520.0), egui::Sense::click());
                    });
            });
        }

        assert_eq!(
            ctx.layer_id_at(window_rect(layout).center())
                .map(|layer| layer.id),
            Some(egui::Id::new("original-fleet-finder"))
        );
    }
}
