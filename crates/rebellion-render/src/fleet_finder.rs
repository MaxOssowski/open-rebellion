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
    // The other side's fleets the player's frozen copies hold, as seen
    // then (FUN_004f2d10, FUN_004f9a60): their names, or their ships'.
    if tab.lists(!player_is_alliance) {
        for (system, _) in &world.systems {
            for entry in crate::fleet_window::copy_entries(world, fog, player, system) {
                let Some(copy) = entry.copy(world) else {
                    continue;
                };
                match mode {
                    FinderMode::Fleets => rows.push(FinderRow {
                        object: entry,
                        name: copy.name.clone(),
                    }),
                    FinderMode::Ships => {
                        for ship in &copy.ships {
                            let Some(class) = world
                                .capital_ship_classes
                                .values()
                                .find(|class| class.dat_id == ship.class)
                            else {
                                continue;
                            };
                            rows.push(FinderRow {
                                object: entry,
                                name: class.name.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
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
                // Its unfinished ships too, by their own names (A0
                // 2026-11-15 "m-1-ship-finder": "Carrack Light Cruiser 3"
                // under construction).
                for product in world.products.in_fleet(fleet) {
                    let rebellion_core::manufacturing::BuildableKind::CapitalShip(class) =
                        product.kind
                    else {
                        continue;
                    };
                    let Some(class) = world.capital_ship_classes.get(class) else {
                        continue;
                    };
                    rows.push(FinderRow {
                        object: FleetWindowEntry::Product {
                            fleet,
                            product: product.id,
                        },
                        name: product.name.clone().unwrap_or_else(|| class.name.clone()),
                    });
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

/// A side tab's tooltip (`FUN_00461960`, `FUN_00462be0`): TEXTSTRA 0x1882
/// "All Fleets", 0x1883 "Alliance Fleets", 0x1884 "Imperial Fleets"; in
/// Ship mode `TTM_UPDATETIPTEXT` swaps in 0x1887 "All Ships", 0x1888
/// "Alliance Ships", 0x1889 "Imperial Ships" (A0 tooltip/48, 49).
const fn tab_tooltip_string(tab: FinderTab, mode: FinderMode) -> u16 {
    let base = match mode {
        FinderMode::Fleets => 0x1882,
        FinderMode::Ships => 0x1887,
    };
    base + tab_index(tab) as u16
}

/// A mode tab's tooltip (`FUN_00461960`): 0x1885 "Ship Finder", 0x1880
/// "Fleet Finder"; both show whichever is chosen (A0 tooltip/46, 47, 66).
const fn mode_tooltip_string(mode: FinderMode) -> u16 {
    match mode {
        FinderMode::Ships => 0x1885,
        FinderMode::Fleets => 0x1880,
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
        window.scroll = rebuilt_scroll(window.scroll, choice, list.len());
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
    let mut step = None;
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
    // Escape closes the Finder even from the name box (FUN_00461590,
    // FUN_00463360): read it before the box, which would take it.
    let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let id = egui::Id::new("original-fleet-finder");
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
            let panel = match window.mode {
                FinderMode::Fleets => FLEET_PANEL[side],
                FinderMode::Ships => SHIP_PANEL[side],
            };
            // The panel lies under the base, whose rounded corners, keyed
            // on blue, cover the panel's (A0 System, Fleet, Troop and
            // Personnel Finders).
            paint(cache, panel, PANEL_AT);
            paint(cache, BACKGROUND[side], (0.0, 0.0));
            paint(cache, RAIL[side], RAIL_AT);

            // The title in font 5, centred in its 350 (A0 2026-10-29), the
            // name label in font 4, the tab's label in font 5, all white
            // (0x2ffffff); the list in font 10, the name box in font 4
            // (crate::name_box).
            let (title, label) = window.mode.titles();
            crate::theme::paint_gdi_text(
                &painter,
                at(frame, scale, (TITLE_AT.0, TITLE_AT.1, 350.0, 17.0)),
                scale,
                5,
                title,
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
                label,
                egui::Color32::WHITE,
                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
            );
            crate::theme::paint_gdi_text(
                &painter,
                at(frame, scale, (TAB_LABEL_AT.0, TAB_LABEL_AT.1, 300.0, 16.0)),
                scale,
                5,
                window.tab.label(window.mode),
                egui::Color32::WHITE,
                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
            );

            // hyp: the tab and mode groups show their current member in its
            // second bitmap, as the mission dialog's tab strip does
            // (FUN_0060d590); their pressed art is untraced.
            for candidate in FinderTab::ALL {
                let control = TABS[tab_index(candidate)];
                let down = candidate == window.tab;
                crate::win_tooltip::window_tool(
                    ui,
                    at(frame, scale, control.rect),
                    ("finder-tab", tab_index(candidate)),
                    tab_tooltip_string(candidate, window.mode),
                    true,
                );
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
                crate::win_tooltip::window_tool(
                    ui,
                    at(frame, scale, control.rect),
                    id,
                    mode_tooltip_string(candidate),
                    true,
                );
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
            for (control, id, string) in [
                (
                    controls.close,
                    "finder-close",
                    crate::system_finder::CLOSE_TOOLTIP,
                ),
                (
                    controls.display,
                    "finder-display",
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
            // A CoolStringField in the default font 4 (crate::name_box).
            let edit = crate::name_box::show(
                ui,
                name_rect,
                scale,
                egui::Id::new("original-fleet-finder-name"),
                &mut name,
            );
            if window.focus_name {
                edit.request_focus();
            }
            name_changed = edit.changed();
            enter = edit.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            step = crate::list_prefix::key_step(ui, edit.has_focus());

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
                crate::theme::paint_gdi_text(
                    &list_painter,
                    at(
                        frame,
                        scale,
                        (LIST.0 + ROW_TEXT_X, y, LIST.2 - ROW_TEXT_X, ROW_HEIGHT),
                    ),
                    scale,
                    10,
                    &row.name,
                    colour,
                    (
                        crate::theme::GdiAlign::Left,
                        crate::theme::GdiVAlign::VCenter,
                    ),
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

    let open_window = state.window.as_mut()?;
    open_window.focus_name = false;
    open_window.scroll = scroll;
    open_window.scroll_hold = scroll_hold;
    if name_changed {
        // FUN_00462a50 0x408: the best prefix match is chosen and shown.
        open_window.name.clone_from(&name);
        let choice = prefix_choice(&list, &name);
        let before = open_window
            .chosen
            .and_then(|object| list.iter().position(|row| row.object == object));
        open_window.chosen = choice.map(|index| list[index].object);
        if let Some(index) = crate::list_prefix::row_to_show(choice, before) {
            open_window.scroll = scrolled_to(open_window.scroll, index, list.len());
        }
    }
    if let Some((row, at)) = clicked_row {
        // 0x29b: the row's id is chosen and its name copied into the box.
        open_window.last_clicked = Some((row, at));
        open_window.chosen = Some(list[row].object);
        open_window.name.clone_from(&list[row].name);
    }
    // A key steps the chosen row and copies its name, as a click does (A0
    // finder-keys/).
    if let Some(step) = step {
        let current = list
            .iter()
            .position(|row| Some(row.object) == open_window.chosen);
        if let Some(row) = crate::list_prefix::stepped_row(current, list.len(), step) {
            open_window.chosen = Some(list[row].object);
            open_window.name.clone_from(&list[row].name);
            open_window.scroll = scrolled_to(open_window.scroll, row, list.len());
        }
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

/// The scroll after a tab's rebuild and rematch: the layout clamps the
/// list's first visible pixel to the new range (`FUN_0060a280` →
/// `FUN_00609ae0`; the rebuild, `FUN_00462be0`, never sets it), then
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
    reason = "A few hundred rows of 20 pixels."
)]
fn max_scroll(rows: usize) -> usize {
    crate::list_scroll_bar::max_first((rows * ROW_PIXELS) as f32, LIST.3) as usize
}

#[cfg(test)]
#[path = "fleet_finder_tests.rs"]
mod tests;
