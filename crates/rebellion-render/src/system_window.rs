//! The original Manufacturing and Production window (window type 9,
//! `FUN_00452fc0`), which the port calls the System window, and the galaxy
//! view's reference rail of minimized windows.
//!
//! `REBEXE.EXE` creates it 226 by 304 (`FUN_0045aac0`). Six bitmap tabs show
//! the overview's three producer bands (ships, troops, facilities) or one of
//! five facility pages (shipyards, training facilities, construction yards,
//! refineries, mines). Manual pp. 82–86, Figs. 2.10, 3.24, 3.27. Recovery:
//! `ghidra/notes/manufacturing-build-selection.md`, "Window composition".
//! What the pages show is `manufacturing_window.rs`. Modeless windows can be
//! minimized into the faction-specific 12-slot rail and restored without
//! creating duplicates.

use egui_macroquad::egui;
use rebellion_core::dat::Faction;
use rebellion_core::delivery::DeliveryState;
use rebellion_core::fog::FogState;
use rebellion_core::ids::{DatId, FleetKey, SystemKey};
use rebellion_core::manufacturing::{ManufacturingState, ProductionArea};
use rebellion_core::missions::{MissionState, MissionTarget, MissionTargetPick};
pub(crate) use rebellion_core::pictures::character_mini_resource_id;
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::fleet_window::{control_side, paint_native};
use crate::manufacturing_window::{
    band, band_strip, contents_visible, item_frame, page_cells, yard_count, yard_page,
    FacilityItem, FacilityPage, BAND_AREAS, BAND_DESTINATION, BAND_FRAME, BAND_MINI, BAND_RECTS,
    BAND_STATUS, BAND_TITLE, BAND_UNITS, CELL, COLUMNS, COUNT_ORIGINS, LIST, PROGRESS_RECTS,
    YARD_COLUMN,
};
use crate::object_menu::MenuObject;
use crate::window_move::{self, MoveAnchor};

pub const SYSTEM_WINDOW_CLIENT_WIDTH: f32 = 226.0;
/// `FUN_0045aac0` creates type 9 at 226 by 304 (`0xe2` by `0x130`).
pub const SYSTEM_WINDOW_WIDTH: f32 = 226.0;
pub const SYSTEM_WINDOW_HEIGHT: f32 = 304.0;
pub const REFERENCE_RAIL_SLOTS: usize = 12;

const WINDOW_BACKGROUND: u32 = 10297;
const CLOSE_NORMAL: u32 = 10108;
const CLOSE_PRESSED: u32 = 10109;
const MINIMIZE_NORMAL: u32 = 10253;
const MINIMIZE_PRESSED: u32 = 10254;
const SECTOR_NORMAL: u32 = 10209;
const SECTOR_PRESSED: u32 = 10208;
/// The title buttons (`FUN_00455060`): the sector button at (3, 3); close
/// right-aligned 3 in (226 - 14 - 3), minimize just left of it.
const SECTOR_X: f32 = 3.0;
const MINIMIZE_X: f32 = 195.0;
const CLOSE_X: f32 = 209.0;
/// The title label `+0x49`: from the sector button's width plus 5, 179 by
/// 16, font 5, black, format `0x24` (left-aligned, vertically centred).
const TITLE: (f32, f32, f32) = (19.0, 2.0, 16.0);
/// The tab strip (`FUN_0060d590`) at (0, 20); each button is 36 by 33.
const TAB_STRIP_Y: f32 = 20.0;
const TAB_SIZE: (f32, f32) = (36.0, 33.0);
/// The page label `+0x5f`: font 4, white, format 1 (`DT_CENTER`), the
/// window's width wide at (0, 58).
const PAGE_LABEL_Y: f32 = 58.0;
/// Each yard count label `+0x19c`: format 1, 46 wide from x 6.
const COUNT_WIDTH: f32 = 46.0;
/// The count label keeps the default font (`FUN_00601880` ->
/// `FUN_00603850`, the game root's `+0xb0`). The A0 capture's digits stand
/// 10 rows tall, a 16-row cell, as the font table's default (index 4).
const COUNT_FONT: u16 = 4;
/// A band's progress bar, `0x200f0f0` (`FUN_004acec0`).
const PROGRESS_COLOR: egui::Color32 = egui::Color32::from_rgb(0xf0, 0xf0, 0x00);
/// A facility picture sits 1 by 2 into its 69 by 40 cell.
const PICTURE_OFFSET: (f32, f32) = (1.0, 2.0);

/// The six tabs (`FUN_00455060`, command `0x70`, pages `0x67..0x6c`), in
/// strip order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemWindowTab {
    /// `0x67`: the three producer bands.
    Overview,
    Shipyards,
    TrainingFacilities,
    ConstructionYards,
    Refineries,
    Mines,
}

impl SystemWindowTab {
    pub const ALL: [Self; 6] = [
        Self::Overview,
        Self::Shipyards,
        Self::TrainingFacilities,
        Self::ConstructionYards,
        Self::Refineries,
        Self::Mines,
    ];

    /// The button's x in the strip.
    #[must_use]
    pub const fn x(self) -> f32 {
        match self {
            Self::Overview => 0.0,
            Self::Shipyards => 39.0,
            Self::TrainingFacilities => 77.0,
            Self::ConstructionYards => 115.0,
            Self::Refineries => 152.0,
            Self::Mines => 190.0,
        }
    }

    /// The facility page the tab shows, or `None` for the overview.
    #[must_use]
    pub const fn page(self) -> Option<FacilityPage> {
        match self {
            Self::Overview => None,
            Self::Shipyards => Some(FacilityPage::Shipyards),
            Self::TrainingFacilities => Some(FacilityPage::TrainingFacilities),
            Self::ConstructionYards => Some(FacilityPage::ConstructionYards),
            Self::Refineries => Some(FacilityPage::Refineries),
            Self::Mines => Some(FacilityPage::Mines),
        }
    }

    /// The button's help message, which the page label shows: TEXTSTRA
    /// 6197, 6184, 6192, 6194, 6183, 6182.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Overview => "Manufacturing",
            Self::Shipyards => "Shipyards",
            Self::TrainingFacilities => "Training Facilities",
            Self::ConstructionYards => "Construction Yards",
            Self::Refineries => "Refineries",
            Self::Mines => "Mines",
        }
    }
}

/// A tab's bitmap (`FUN_00455060`, `FUN_00456230`): the overview's by the
/// shown side, normal or pressed; a page's normal, pressed, or its empty art
/// in place of the normal one when the page lists no facility. A selected
/// button shows its pressed art (`FUN_0060d700` copies it into the selected
/// state).
#[must_use]
pub const fn tab_resource(tab: SystemWindowTab, side: u8, pressed: bool, empty: bool) -> u32 {
    let (normal, down, none) = match tab {
        SystemWindowTab::Overview => {
            let (normal, down) = match side {
                1 => (10_312, 10_311),
                2 => (10_315, 10_314),
                _ => (10_318, 10_317),
            };
            return if pressed { down } else { normal };
        }
        SystemWindowTab::Shipyards => (10_327, 10_326, 10_328),
        SystemWindowTab::TrainingFacilities => (10_330, 10_329, 10_331),
        SystemWindowTab::ConstructionYards => (10_333, 10_332, 10_334),
        SystemWindowTab::Refineries => (10_324, 10_323, 10_325),
        SystemWindowTab::Mines => (10_321, 10_320, 10_322),
    };
    if pressed {
        down
    } else if empty {
        none
    } else {
        normal
    }
}

/// A minimized System window's rail icon (`Class_00659e68_vt74`,
/// `FUN_00458cf0`): 11530 (`0x2d0a`) for side 1, 11531 for side 2, 11532
/// otherwise, by the system's side bits (`+0x24 >> 6 & 3`).
const RAIL_ICONS: [u32; 3] = [11530, 11531, 11532];

/// The rail's name color, `0x200f0f0` (`FUN_00428d70`).
const RAIL_NAME_COLOR: egui::Color32 = egui::Color32::from_rgb(0xf0, 0xf0, 0x00);

fn rail_icon(world: &GameWorld, system: SystemKey) -> u32 {
    match world
        .systems
        .get(system)
        .map_or(0, |value| control_side(value.control))
    {
        1 => RAIL_ICONS[0],
        2 => RAIL_ICONS[1],
        _ => RAIL_ICONS[2],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenSystemWindow {
    system: SystemKey,
    logical_position: (i16, i16),
    tab: SystemWindowTab,
    /// A facility page's selected item.
    selected_item: Option<FacilityItem>,
    /// The overview's selected band (band `+0x30` bit 0). port: one band;
    /// Ctrl's toggle is not ported.
    selected_band: Option<ProductionArea>,
    /// Where the window was when a press on its surface took it
    /// (`crate::window_move`).
    moving: MoveAnchor,
}

/// A minimized window on the galaxy view's rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RailEntry {
    System(OpenSystemWindow),
    /// A Fleet window (type 4); its icon comes from `FUN_004a76e0`.
    Fleet {
        system: SystemKey,
        logical_position: (i16, i16),
    },
    /// A System Defenses window (type 10); its icon comes from
    /// `FUN_004aa4a0`.
    Defenses {
        system: SystemKey,
        logical_position: (i16, i16),
    },
    /// A Missions window (type 11); its icon comes from `FUN_004a21c0`.
    Missions {
        system: SystemKey,
        logical_position: (i16, i16),
    },
}

impl RailEntry {
    const fn system(&self) -> SystemKey {
        match self {
            Self::System(window) => window.system,
            Self::Fleet { system, .. }
            | Self::Defenses { system, .. }
            | Self::Missions { system, .. } => *system,
        }
    }

    const fn is_system_window(&self) -> bool {
        matches!(self, Self::System(_))
    }

    /// One slot per window: the window type and its system.
    fn same_window(&self, other: &Self) -> bool {
        self.system() == other.system()
            && std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// Mutable modeless-window and rail state. The last visible window is focused;
/// the first rail entry occupies slot one and is the oldest minimized window.
#[derive(Debug)]
pub struct SystemWindowState {
    faction: CockpitFaction,
    windows: Vec<OpenSystemWindow>,
    rail: Vec<RailEntry>,
}

impl Default for SystemWindowState {
    fn default() -> Self {
        Self {
            faction: CockpitFaction::Alliance,
            windows: Vec::new(),
            rail: Vec::new(),
        }
    }
}

impl SystemWindowState {
    /// Open at the original logical double-click point, clamped so the client
    /// surface remains inside the recovered galaxy aperture, on the overview
    /// (`FUN_00452fc0` starts on page `0x67`). Existing visible or minimized
    /// windows focus or restore instead of duplicating.
    pub fn open(
        &mut self,
        world: &GameWorld,
        system: SystemKey,
        logical_position: (i16, i16),
        faction: CockpitFaction,
        layout: CockpitLayout,
    ) -> bool {
        self.prepare_faction(faction);
        if !world.systems.contains_key(system) {
            return false;
        }
        if self.focus(system) {
            return true;
        }
        if self.restore(system) {
            return true;
        }
        let logical_position = clamp_to_galaxy(logical_position, layout);
        self.windows.push(OpenSystemWindow {
            system,
            logical_position,
            tab: SystemWindowTab::Overview,
            selected_item: None,
            selected_band: None,
            moving: None,
        });
        true
    }

    /// Open `system`'s window on `tab`.
    pub fn open_tab(
        &mut self,
        world: &GameWorld,
        system: SystemKey,
        tab: SystemWindowTab,
        logical_position: (i16, i16),
        faction: CockpitFaction,
        layout: CockpitLayout,
    ) -> bool {
        if !self.open(world, system, logical_position, faction, layout) {
            return false;
        }
        self.select_tab(system, tab);
        true
    }

    /// The screen rect of `tab`'s button in `system`'s visible window.
    #[must_use]
    pub fn tab_screen_rect(
        &self,
        layout: CockpitLayout,
        system: SystemKey,
        tab: SystemWindowTab,
    ) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(tab_rect(
            window_screen_rect(*window, layout),
            layout.scale,
            tab,
        ))
    }

    /// The screen rect of `area`'s overview band in `system`'s visible
    /// window.
    #[must_use]
    pub fn band_screen_rect(
        &self,
        layout: CockpitLayout,
        system: SystemKey,
        area: ProductionArea,
    ) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(band_rect(
            window_screen_rect(*window, layout),
            layout.scale,
            area,
        ))
    }

    /// The screen rect of the `index`th cell of `system`'s facility page.
    #[must_use]
    pub fn cell_screen_rect(
        &self,
        layout: CockpitLayout,
        system: SystemKey,
        index: usize,
    ) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(cell_rect(
            window_screen_rect(*window, layout),
            layout.scale,
            index,
        ))
    }

    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        self.windows.iter().any(|window| {
            let rect = window_screen_rect(*window, layout);
            point.0 >= rect.min.x
                && point.0 < rect.max.x
                && point.1 >= rect.min.y
                && point.1 < rect.max.y
        })
    }

    #[must_use]
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Each visible window's system and logical position, back to front.
    pub fn open_windows(&self) -> impl Iterator<Item = (SystemKey, (i16, i16))> + '_ {
        self.windows
            .iter()
            .map(|window| (window.system, window.logical_position))
    }

    /// The tab and selected band of `system`'s visible window.
    #[must_use]
    pub fn selection(
        &self,
        system: SystemKey,
    ) -> Option<(SystemWindowTab, Option<ProductionArea>)> {
        self.windows
            .iter()
            .find(|window| window.system == system)
            .map(|window| (window.tab, window.selected_band))
    }

    /// What a Mission released over the window egui draws as `layer`
    /// targets (`+0x68`, `FUN_00458b80`): on a facility page, the facility
    /// in the list cell under `point` (list `+0x174` at (8, 77)); on the
    /// overview (page `0x67`) or anywhere else, the window's system. `None`
    /// when `layer` is none of these windows.
    ///
    /// The inner `None` is a cell no mission takes, so the order is dropped:
    /// a unit queued or under construction is not completed, which the
    /// object rules refuse (`FUN_00593500`, `+0x50` bit 2), and one
    /// travelling is en route (rule 5); hyp: an empty deposit's item names no
    /// object. port: the order is dropped at once instead of being handed
    /// back (`FUN_0041ce20`).
    #[expect(
        clippy::too_many_arguments,
        reason = "The page's cells read the world, the player's view and production."
    )]
    #[must_use]
    pub fn mission_target(
        &self,
        world: &GameWorld,
        fog: &FogState,
        manufacturing: &ManufacturingState,
        deliveries: &DeliveryState,
        layout: CockpitLayout,
        layer: egui::LayerId,
        point: egui::Pos2,
    ) -> Option<Option<MissionTargetPick>> {
        let window = self
            .windows
            .iter()
            .find(|window| area_id(window.system) == layer.id)?;
        let mut pick = MissionTargetPick::from(window.system);
        let Some(page) = window.tab.page() else {
            return Some(Some(pick));
        };
        let screen = window_screen_rect(*window, layout);
        let list = logical_rect(screen, layout.scale, LIST.0, LIST.1, LIST.2, LIST.3);
        if !rect_contains(list, point) {
            return Some(Some(pick));
        }
        let player = cockpit_faction(self.faction);
        let visible = contents_visible(world, fog, player, window.system);
        let item = page_cells(
            world,
            manufacturing,
            deliveries,
            visible,
            window.system,
            page,
        )
        .into_iter()
        .enumerate()
        .find(|(index, _)| rect_contains(cell_rect(screen, layout.scale, *index), point))
        .map(|(_, cell)| cell.item);
        pick.object = match item {
            Some(FacilityItem::Manufacturing(key)) => {
                Some(MissionTarget::ManufacturingFacility(key))
            }
            Some(FacilityItem::Production(key)) => Some(MissionTarget::ProductionFacility(key)),
            Some(FacilityItem::Pending(_) | FacilityItem::Deposit(_)) => return Some(None),
            None => None,
        };
        Some(Some(pick))
    }

    /// The destination a targeting release takes from the system window
    /// egui draws as `layer`: its own system wherever the point lies
    /// (`+0x70`, `FUN_004aa470`). `None` when `layer` is no open system
    /// window.
    #[must_use]
    pub fn release_target(&self, layer: egui::LayerId) -> Option<SystemKey> {
        self.windows
            .iter()
            .find(|window| area_id(window.system) == layer.id)
            .map(|window| window.system)
    }

    #[must_use]
    pub fn rail_count(&self) -> usize {
        self.rail.len()
    }

    /// Each rail entry, oldest first: its system and its window, "system",
    /// "fleet", "defenses" or "missions".
    pub fn rail_entries(&self) -> impl Iterator<Item = (SystemKey, &'static str)> + '_ {
        self.rail.iter().map(|entry| {
            let kind = match entry {
                RailEntry::System(_) => "system",
                RailEntry::Fleet { .. } => "fleet",
                RailEntry::Defenses { .. } => "defenses",
                RailEntry::Missions { .. } => "missions",
            };
            (entry.system(), kind)
        })
    }

    /// The screen rect of the `index`th rail slot while it holds an entry.
    #[must_use]
    pub fn rail_slot_screen_rect(&self, layout: CockpitLayout, index: usize) -> Option<egui::Rect> {
        (index < self.rail.len()).then(|| cockpit_rect(layout, rail_slot_rect(self.faction, index)))
    }

    pub fn clear(&mut self) {
        self.windows.clear();
        self.rail.clear();
    }

    fn prepare_faction(&mut self, faction: CockpitFaction) {
        if self.faction != faction {
            self.faction = faction;
            self.clear();
        }
    }

    fn focus(&mut self, system: SystemKey) -> bool {
        let Some(index) = self
            .windows
            .iter()
            .position(|window| window.system == system)
        else {
            return false;
        };
        let window = self.windows.remove(index);
        self.windows.push(window);
        true
    }

    fn close(&mut self, system: SystemKey) {
        self.windows.retain(|window| window.system != system);
    }

    fn minimize(&mut self, system: SystemKey) -> bool {
        let Some(index) = self
            .windows
            .iter()
            .position(|window| window.system == system)
        else {
            return false;
        };
        let window = self.windows.remove(index);
        self.push_rail(RailEntry::System(window));
        true
    }

    /// Put a minimized Fleet window on the rail (`0x466`).
    pub fn minimize_fleet_window(&mut self, system: SystemKey, logical_position: (i16, i16)) {
        self.push_rail(RailEntry::Fleet {
            system,
            logical_position,
        });
    }

    /// Put a minimized Defenses window on the rail (`0x466`).
    pub fn minimize_defenses_window(&mut self, system: SystemKey, logical_position: (i16, i16)) {
        self.push_rail(RailEntry::Defenses {
            system,
            logical_position,
        });
    }

    /// Put a minimized Missions window on the rail (`0x466`).
    pub fn minimize_missions_window(&mut self, system: SystemKey, logical_position: (i16, i16)) {
        self.push_rail(RailEntry::Missions {
            system,
            logical_position,
        });
    }

    fn push_rail(&mut self, entry: RailEntry) {
        self.rail.retain(|held| !held.same_window(&entry));
        if self.rail.len() == REFERENCE_RAIL_SLOTS {
            self.rail.remove(0);
        }
        self.rail.push(entry);
    }

    fn restore(&mut self, system: SystemKey) -> bool {
        let Some(index) = self
            .rail
            .iter()
            .position(|entry| entry.is_system_window() && entry.system() == system)
        else {
            return false;
        };
        if let RailEntry::System(window) = self.rail.remove(index) {
            self.windows.push(window);
        }
        true
    }

    fn window_mut(&mut self, system: SystemKey) -> Option<&mut OpenSystemWindow> {
        self.windows
            .iter_mut()
            .find(|window| window.system == system)
    }

    /// Show `tab`'s page; a new page clears the item selection
    /// (`FUN_004568a0` refills the list).
    fn select_tab(&mut self, system: SystemKey, tab: SystemWindowTab) {
        if let Some(window) = self.window_mut(system) {
            if window.tab != tab {
                window.tab = tab;
                window.selected_item = None;
            }
        }
        self.focus(system);
    }

    fn select_item(&mut self, system: SystemKey, item: Option<FacilityItem>) {
        if let Some(window) = self.window_mut(system) {
            window.selected_item = item;
        }
        self.focus(system);
    }

    fn select_band(&mut self, system: SystemKey, area: ProductionArea) {
        if let Some(window) = self.window_mut(system) {
            window.selected_band = Some(area);
        }
        self.focus(system);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemWindowAction {
    FocusSector(SystemKey),
    SelectSystem(SystemKey),
    /// A right-button release on a band opens the object pop-up menu
    /// (`FUN_004ac5c0`) for the selection, at a 640 by 480 canvas point.
    OpenObjectMenu {
        system: SystemKey,
        selection: Option<MenuObject>,
        point: (i16, i16),
    },
    /// A click on a minimized Fleet window's rail slot restores it.
    RestoreFleetWindow {
        system: SystemKey,
        logical_position: (i16, i16),
    },
    /// A click on a minimized Defenses window's rail slot restores it.
    RestoreDefensesWindow {
        system: SystemKey,
        logical_position: (i16, i16),
    },
    /// A click on a minimized Missions window's rail slot restores it.
    RestoreMissionsWindow {
        system: SystemKey,
        logical_position: (i16, i16),
    },
}

#[derive(Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Each flag is one independent title-bar or list outcome of a frame."
)]
struct WindowDrawResult {
    focus: bool,
    close: bool,
    minimize: bool,
    focus_sector: bool,
    tab: Option<SystemWindowTab>,
    /// A press on a facility page's list: the item under it, or none.
    item: Option<Option<FacilityItem>>,
    band: Option<ProductionArea>,
    object_menu: Option<(Option<MenuObject>, (i16, i16))>,
    /// The pointer's travel since a press on the window's surface.
    travel: Option<egui::Vec2>,
}

/// What a frame's drawing reads.
#[derive(Clone, Copy)]
struct Sources<'a> {
    world: &'a GameWorld,
    manufacturing: &'a ManufacturingState,
    deliveries: &'a DeliveryState,
    fog: &'a FogState,
}

/// Draw the faction rail and every visible Manufacturing and Production
/// window.
///
/// port: the window's keys (`FUN_00458980`) are not ported; the galaxy
/// view's windows keep no shared keyboard focus to route them by.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit state and rendering inputs at this UI boundary; the rail's Missions icon reads the mission state."
)]
pub fn draw_system_windows(
    ctx: &egui::Context,
    world: &GameWorld,
    movement: &rebellion_core::movement::MovementState,
    fog: &FogState,
    missions: &MissionState,
    manufacturing: &ManufacturingState,
    deliveries: &DeliveryState,
    state: &mut SystemWindowState,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Vec<SystemWindowAction> {
    state.prepare_faction(faction);
    let mut actions: Vec<SystemWindowAction> = draw_reference_rail(
        ctx, world, movement, fog, missions, state, faction, layout, cache,
    )
    .into_iter()
    .collect();
    let sources = Sources {
        world,
        manufacturing,
        deliveries,
        fog,
    };
    let windows = state.windows.clone();
    let focused_system = windows.last().map(|window| window.system);
    for window in windows {
        let system = window.system;
        let mut result = draw_system_window(
            ctx,
            sources,
            window,
            focused_system == Some(system),
            faction,
            layout,
            cache,
        );
        if result.close {
            state.close(system);
            continue;
        }
        if result.minimize {
            state.minimize(system);
            continue;
        }
        if result.focus_sector {
            actions.push(SystemWindowAction::FocusSector(system));
            state.close(system);
            continue;
        }
        if let Some(open) = state.window_mut(system) {
            if result.travel.is_some() && open.moving.is_none() {
                result.focus = true;
            }
            window_move::follow(
                &mut open.logical_position,
                &mut open.moving,
                result.travel,
                |position| clamp_to_galaxy(position, layout),
            );
        }
        if let Some(tab) = result.tab {
            state.select_tab(system, tab);
        }
        if let Some(item) = result.item {
            state.select_item(system, item);
        }
        if let Some(area) = result.band {
            state.select_band(system, area);
        }
        if result.focus {
            state.focus(system);
            actions.push(SystemWindowAction::SelectSystem(system));
        }
        if let Some((selection, point)) = result.object_menu {
            actions.push(SystemWindowAction::OpenObjectMenu {
                system,
                selection,
                point,
            });
        }
    }
    actions
}

#[expect(
    clippy::too_many_arguments,
    reason = "The rail's icons read the world, fog and mission state."
)]
fn draw_reference_rail(
    ctx: &egui::Context,
    world: &GameWorld,
    movement: &rebellion_core::movement::MovementState,
    fog: &FogState,
    missions: &MissionState,
    state: &mut SystemWindowState,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Option<SystemWindowAction> {
    let entries = state.rail.clone();
    let mut restored = None;
    for (index, entry) in entries.iter().enumerate() {
        let Some(system) = world.systems.get(entry.system()) else {
            continue;
        };
        let logical = rail_slot_rect(faction, index);
        let screen_rect = cockpit_rect(layout, logical);
        egui::Area::new(egui::Id::new(("original-reference-rail", index)))
            .fixed_pos(screen_rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let (slot_rect, response) =
                    ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
                // FUN_00428d70: the window's icon (slot `+0x74`) at the
                // slot's corner, then its system's name after the icon in
                // font 10, yellow (`0x200f0f0`), clipped to the slot.
                let icon = match entry {
                    RailEntry::System(window) => rail_icon(world, window.system),
                    RailEntry::Defenses { system, .. } => {
                        crate::defenses_window::rail_icon(world, *system)
                    }
                    RailEntry::Missions { system, .. } => {
                        crate::missions_window::rail_icon(world, fog, missions, faction, *system)
                    }
                    RailEntry::Fleet { system, .. } => crate::fleet_window::rail_icon(
                        world,
                        movement,
                        fog,
                        cockpit_faction(faction),
                        *system,
                    ),
                };
                let painter = ui.painter().with_clip_rect(slot_rect);
                paint_native(
                    &painter,
                    ctx,
                    cache,
                    DllSource::Strategy,
                    icon,
                    slot_rect,
                    layout.scale,
                    0.0,
                    0.0,
                );
                let icon_width = cache
                    .original_resource_size(DllSource::Strategy, icon)
                    .map_or(0.0, |[width, _]| width as f32);
                painter.text(
                    logical_point(slot_rect, layout.scale, icon_width, 0.0),
                    egui::Align2::LEFT_TOP,
                    &system.name,
                    crate::theme::original_font(10, layout.scale),
                    RAIL_NAME_COLOR,
                );
                if exact_clicked(&response, slot_rect) {
                    restored = Some(*entry);
                }
            });
    }
    match restored? {
        RailEntry::System(window) => {
            state.restore(window.system);
            None
        }
        entry @ RailEntry::Fleet {
            system,
            logical_position,
        } => {
            state.rail.retain(|held| !held.same_window(&entry));
            Some(SystemWindowAction::RestoreFleetWindow {
                system,
                logical_position,
            })
        }
        entry @ RailEntry::Defenses {
            system,
            logical_position,
        } => {
            state.rail.retain(|held| !held.same_window(&entry));
            Some(SystemWindowAction::RestoreDefensesWindow {
                system,
                logical_position,
            })
        }
        entry @ RailEntry::Missions {
            system,
            logical_position,
        } => {
            state.rail.retain(|held| !held.same_window(&entry));
            Some(SystemWindowAction::RestoreMissionsWindow {
                system,
                logical_position,
            })
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Keep the window's ordered paint and input pass together, as the Defenses window does."
)]
fn draw_system_window(
    ctx: &egui::Context,
    sources: Sources<'_>,
    window: OpenSystemWindow,
    focused: bool,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> WindowDrawResult {
    let mut result = WindowDrawResult::default();
    let world = sources.world;
    let Some(system) = world.systems.get(window.system) else {
        result.close = true;
        return result;
    };
    let player = cockpit_faction(faction);
    let side = control_side(system.control);
    let visible = contents_visible(world, sources.fog, player, window.system);
    let pages = SystemWindowTab::ALL.map(|tab| {
        tab.page().map(|page| {
            page_cells(
                world,
                sources.manufacturing,
                sources.deliveries,
                visible,
                window.system,
                page,
            )
        })
    });
    let scale = layout.scale;
    let screen_rect = window_screen_rect(window, layout);
    let id = area_id(window.system);
    if focused {
        ctx.move_to_top(egui::LayerId::new(egui::Order::Foreground, id));
    }
    let area = egui::Area::new(id)
        .fixed_pos(screen_rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let (pointer, primary_down) = ctx.input(|input| {
                (
                    input.pointer.interact_pos(),
                    input.pointer.button_down(egui::PointerButton::Primary),
                )
            });
            let (local, window_response) =
                ui.allocate_exact_size(screen_rect.size(), egui::Sense::click_and_drag());
            // The window clips its children: the 231-wide title strip the
            // 235-wide Fleet, Defenses and Missions windows share ends at
            // this window's 226.
            let painter = ui.painter().with_clip_rect(local);
            let paint = |cache: &mut BmpCache, source: DllSource, id: u32, x: f32, y: f32| {
                paint_native(&painter, ctx, cache, source, id, local, scale, x, y);
            };

            painter.rect_filled(local, 0.0, egui::Color32::BLACK);
            paint(cache, DllSource::Strategy, WINDOW_BACKGROUND, 0.0, 0.0);
            paint(
                cache,
                DllSource::Strategy,
                crate::defenses_window::title_resource(side, focused),
                2.0,
                2.0,
            );
            painter.text(
                logical_rect(local, scale, TITLE.0, TITLE.1 + TITLE.2 / 2.0, 0.0, 0.0).min,
                egui::Align2::LEFT_CENTER,
                &system.name,
                crate::theme::original_font(5, scale),
                egui::Color32::BLACK,
            );

            let buttons = [
                (SECTOR_NORMAL, SECTOR_PRESSED, SECTOR_X, "sector"),
                (MINIMIZE_NORMAL, MINIMIZE_PRESSED, MINIMIZE_X, "minimize"),
                (CLOSE_NORMAL, CLOSE_PRESSED, CLOSE_X, "close"),
            ];
            let mut clicked = [false; 3];
            for (index, (normal, pressed, x, name)) in buttons.into_iter().enumerate() {
                let rect = logical_rect(local, scale, x, 3.0, 14.0, 14.0);
                let response = ui.interact(
                    rect,
                    ui.id().with((window.system, name)),
                    egui::Sense::click(),
                );
                let down = primary_down && pointer.is_some_and(|point| rect_contains(rect, point));
                paint(
                    cache,
                    DllSource::Strategy,
                    if down { pressed } else { normal },
                    x,
                    3.0,
                );
                clicked[index] = exact_clicked(&response, rect);
            }
            result.focus_sector = clicked[0];
            result.minimize = clicked[1];
            result.close = clicked[2];

            // The tabs. port: an empty page's tab still opens it, as the
            // Defenses window's do; its help message is not shown.
            for (tab, cells) in SystemWindowTab::ALL.into_iter().zip(&pages) {
                let rect = tab_rect(local, scale, tab);
                let empty = cells
                    .as_deref()
                    .is_some_and(crate::manufacturing_window::page_is_empty);
                let response = ui.interact(
                    rect,
                    ui.id().with((window.system, tab)),
                    egui::Sense::click(),
                );
                let pressed = tab == window.tab
                    || (primary_down && pointer.is_some_and(|point| rect_contains(rect, point)));
                paint(
                    cache,
                    DllSource::Strategy,
                    tab_resource(tab, side, pressed, empty),
                    tab.x(),
                    TAB_STRIP_Y,
                );
                if exact_clicked(&response, rect) {
                    result.tab = Some(tab);
                    result.focus = true;
                }
            }

            let (any_pressed, right_pressed) = ctx.input(|input| {
                (
                    input.pointer.button_pressed(egui::PointerButton::Primary)
                        || input.pointer.button_pressed(egui::PointerButton::Secondary),
                    input.pointer.button_pressed(egui::PointerButton::Secondary),
                )
            });
            match window.tab.page() {
                None => {
                    // FUN_00457690: the yard column and its counts.
                    paint(
                        cache,
                        DllSource::Strategy,
                        YARD_COLUMN.0,
                        YARD_COLUMN.1,
                        YARD_COLUMN.2,
                    );
                    for (index, area) in BAND_AREAS.into_iter().enumerate() {
                        let cells = pages[SystemWindowTab::ALL
                            .iter()
                            .position(|tab| tab.page() == Some(yard_page(area)))
                            .unwrap_or_default()]
                        .as_deref()
                        .unwrap_or_default();
                        let (x, y) = COUNT_ORIGINS[index];
                        painter.text(
                            logical_point(local, scale, x + COUNT_WIDTH / 2.0, y),
                            egui::Align2::CENTER_TOP,
                            yard_count(cells),
                            crate::theme::original_font(COUNT_FONT, scale),
                            egui::Color32::WHITE,
                        );

                        // FUN_00458080: the band's frame, its strip and its
                        // lines.
                        let shown =
                            band(world, sources.manufacturing, visible, window.system, area);
                        let (bx, by, _, _) = BAND_RECTS[index];
                        let selected = window.selected_band == Some(area);
                        paint(cache, DllSource::Strategy, BAND_FRAME, bx, by);
                        paint(
                            cache,
                            DllSource::Strategy,
                            band_strip(side, selected),
                            bx,
                            by,
                        );
                        // FUN_00458080: every line in font 10; the title
                        // keeps the text object's default colour, black
                        // (0x2000000, FUN_00601930), the others white
                        // (0x2ffffff).
                        let line = |text: &str, (x, y): (f32, f32), colour| {
                            painter.text(
                                logical_point(local, scale, bx + x, by + y),
                                egui::Align2::LEFT_TOP,
                                text,
                                crate::theme::original_font(10, scale),
                                colour,
                            );
                        };
                        let white = egui::Color32::WHITE;
                        line(
                            crate::manufacturing_window::band_title(area),
                            BAND_TITLE,
                            egui::Color32::BLACK,
                        );
                        line(&shown.status, BAND_STATUS, white);
                        if let Some(mini) = shown.mini {
                            paint(
                                cache,
                                DllSource::Gokres,
                                mini,
                                bx + BAND_MINI.0,
                                by + BAND_MINI.1,
                            );
                        }
                        if let Some(units) = &shown.units {
                            line(units, BAND_UNITS, white);
                        }
                        line(&shown.destination, BAND_DESTINATION, white);

                        // FUN_004acec0: the progress bar, 0x200f0f0 over
                        // black. In COLORREF order that is red 0xf0, green
                        // 0xf0, blue 0: yellow, as the rail's name in the
                        // same colour shows in A0.
                        let (px, py, width, height) = PROGRESS_RECTS[index];
                        let bar = logical_rect(local, scale, px, py, width, height);
                        painter.rect_filled(bar, 0.0, egui::Color32::BLACK);
                        if let Some(progress) = shown.progress {
                            let mut done = bar;
                            done.set_width(bar.width() * progress.clamp(0.0, 1.0));
                            painter.rect_filled(done, 0.0, PROGRESS_COLOR);
                        }

                        // A press selects the band; a right release opens
                        // its manager's menu (FUN_00453ee0, FUN_004ac5c0).
                        let rect = band_rect(local, scale, area);
                        let response = ui.interact(
                            rect,
                            ui.id().with((window.system, "band", index)),
                            egui::Sense::click(),
                        );
                        if any_pressed && response.is_pointer_button_down_on() {
                            result.band = Some(area);
                            result.focus = true;
                        }
                        if response.secondary_clicked() {
                            if let Some(point) = response
                                .interact_pointer_pos()
                                .filter(|point| rect_contains(rect, *point))
                            {
                                result.object_menu = Some((
                                    Some(MenuObject::Producer {
                                        system: window.system,
                                        area,
                                    }),
                                    canvas_point(layout, point),
                                ));
                            }
                        }
                    }
                }
                Some(_) => {
                    // The page label, then the list (FUN_004568a0).
                    painter.text(
                        logical_point(local, scale, SYSTEM_WINDOW_WIDTH / 2.0, PAGE_LABEL_Y),
                        egui::Align2::CENTER_TOP,
                        window.tab.name(),
                        egui::FontId::proportional((10.0 * scale).max(7.0)),
                        egui::Color32::WHITE,
                    );
                    let cells = pages[SystemWindowTab::ALL
                        .iter()
                        .position(|tab| *tab == window.tab)
                        .unwrap_or_default()]
                    .as_deref()
                    .unwrap_or_default();
                    let list =
                        logical_rect(local, scale, LIST.0, LIST.1, LIST.2, LIST.3).intersect(local);
                    let list_painter = painter.with_clip_rect(list);
                    // A press on the list's empty space clears the
                    // selection (FUN_006094b0).
                    let list_response = ui.interact(
                        list,
                        ui.id().with((window.system, "list")),
                        egui::Sense::click(),
                    );
                    if any_pressed && list_response.is_pointer_button_down_on() {
                        result.item = Some(None);
                        result.focus = true;
                    }
                    for (index, cell) in cells.iter().enumerate() {
                        let rect = cell_rect(local, scale, index);
                        if !rect.intersects(list) {
                            break;
                        }
                        paint_native(
                            &list_painter,
                            ctx,
                            cache,
                            DllSource::Strategy,
                            cell.picture,
                            rect,
                            scale,
                            PICTURE_OFFSET.0,
                            PICTURE_OFFSET.1,
                        );
                        if window.selected_item == Some(cell.item) {
                            paint_native(
                                &list_painter,
                                ctx,
                                cache,
                                DllSource::Strategy,
                                item_frame(side),
                                rect,
                                scale,
                                PICTURE_OFFSET.0,
                                PICTURE_OFFSET.1,
                            );
                        }
                        // A left or right press selects the item
                        // (FUN_006083c0). port: a facility's class menu
                        // (Encyclopedia, Status, Scrap) is not ported, so a
                        // right release opens none.
                        let response = ui.interact(
                            rect.intersect(list),
                            ui.id().with((window.system, "cell", index)),
                            egui::Sense::click(),
                        );
                        if (exact_clicked(&response, rect)
                            || (right_pressed && response.is_pointer_button_down_on()))
                            && result.item.is_none()
                        {
                            result.item = Some(Some(cell.item));
                            result.focus = true;
                        }
                    }
                }
            }

            if window_response.clicked() || clicked.iter().any(|&value| value) {
                result.focus = true;
            }
            // The child controls: the title buttons, the tabs, and the
            // overview's bands or a facility page's list.
            let mut controls: Vec<egui::Rect> = [SECTOR_X, MINIMIZE_X, CLOSE_X]
                .into_iter()
                .map(|x| logical_rect(local, scale, x, 3.0, 14.0, 14.0))
                .collect();
            controls.extend(SystemWindowTab::ALL.map(|tab| tab_rect(local, scale, tab)));
            match window.tab.page() {
                None => controls.extend(BAND_AREAS.map(|area| band_rect(local, scale, area))),
                Some(_) => {
                    controls.push(logical_rect(local, scale, LIST.0, LIST.1, LIST.2, LIST.3))
                }
            }
            result.travel =
                window_move::held_travel(ui, &window_response, &controls, window.moving, scale);
        });
    if area.response.clicked() {
        result.focus = true;
    }
    result
}

fn area_id(system: SystemKey) -> egui::Id {
    egui::Id::new(("original-system-window", system))
}

fn tab_rect(window: egui::Rect, scale: f32, tab: SystemWindowTab) -> egui::Rect {
    logical_rect(window, scale, tab.x(), TAB_STRIP_Y, TAB_SIZE.0, TAB_SIZE.1)
}

fn band_rect(window: egui::Rect, scale: f32, area: ProductionArea) -> egui::Rect {
    let index = BAND_AREAS
        .iter()
        .position(|value| *value == area)
        .unwrap_or_default();
    let (x, y, width, height) = BAND_RECTS[index];
    logical_rect(window, scale, x, y, width, height)
}

/// The `index`th cell of a facility page (`FUN_00609ae0`): row-major from
/// the list's corner, three to a row.
#[expect(clippy::cast_precision_loss, reason = "Cell indexes are small.")]
fn cell_rect(window: egui::Rect, scale: f32, index: usize) -> egui::Rect {
    let column = (index % COLUMNS) as f32;
    let row = (index / COLUMNS) as f32;
    logical_rect(
        window,
        scale,
        LIST.0 + column * CELL.0,
        LIST.1 + row * CELL.1,
        CELL.0,
        CELL.1,
    )
}

/// A screen point in 640 by 480 canvas coordinates, truncated as a Win32
/// `POINT` is.
#[expect(
    clippy::cast_possible_truncation,
    reason = "Canvas coordinates fit an i16, as the original's POINT words do."
)]
pub(crate) fn canvas_point(layout: CockpitLayout, point: egui::Pos2) -> (i16, i16) {
    let scale = layout.scale.max(f32::EPSILON);
    (
        ((point.x - layout.canvas.x) / scale).floor() as i16,
        ((point.y - layout.canvas.y) / scale).floor() as i16,
    )
}

/// Whether the player sees the other side's objects at `system`: it holds
/// the system, or its fog shows the system now.
pub(crate) fn opposing_contents_visible(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> bool {
    world
        .systems
        .get(system)
        .is_some_and(|value| value.control.is_controlled_by(player))
        || (fog.faction == player && fog.is_visible(system))
}

/// A fleet's name, as its windows list it (`FUN_004f6270`): "Fleet N"
/// from its side's counter (`GameWorld::fleet_name`). Rename is not ported.
#[must_use]
pub fn fleet_label(world: &GameWorld, fleet: FleetKey) -> Option<String> {
    world.fleet_name(fleet).map(str::to_owned)
}

pub(crate) fn manufacturing_facility_mini(dat_id: DatId) -> Option<(u32, &'static str)> {
    let label = match dat_id.index() {
        1 => "Orbital Shipyard",
        2 => "Training Facility",
        3 => "Construction Yard",
        4 => "Advanced Shipyard",
        5 => "Advanced Training Facility",
        6 => "Advanced Construction Yard",
        _ => return None,
    };
    matches!(dat_id.family(), 0x20 | 0x28..=0x2a).then_some(if dat_id.family() == 0x20 {
        (17_216, "Alliance Headquarters")
    } else {
        (16_639 + dat_id.index(), label)
    })
}

pub(crate) fn defense_facility_mini(dat_id: DatId) -> Option<(u32, &'static str)> {
    let (resource_id, label) = match dat_id.index() {
        1 => (16_896, "KDY-150"),
        2 => (16_897, "LNR Series I"),
        3 => (16_898, "GenCore Level I"),
        4 => (17_024, "Death Star Shield"),
        5 => (16_899, "LNR Series II"),
        6 => (16_900, "GenCore Level II"),
        _ => return None,
    };
    matches!(dat_id.family(), 0x22..=0x25).then_some((resource_id, label))
}

pub(crate) fn production_facility_mini(dat_id: DatId) -> Option<(u32, &'static str)> {
    if !matches!(dat_id.family(), 0x2c..=0x2d) {
        return None;
    }
    match dat_id.index() {
        1 => Some((16_385, "Mine")),
        2 => Some((16_386, "Refinery")),
        _ => None,
    }
}

pub(crate) fn troop_mini(dat_id: DatId) -> Option<(u32, &'static str)> {
    if dat_id.family() != 0x10 {
        return None;
    }
    let (resource_id, label) = match dat_id.index() {
        1 => (17_472, "Alliance Fleet Regiment"),
        2 => (17_473, "Alliance Army Regiment"),
        3 => (17_474, "Sullustan Regiment"),
        4 => (17_475, "Wookiee Regiment"),
        5 => (17_476, "Mon Calamari Regiment"),
        6 => (17_536, "Stormtrooper Regiment"),
        7 => (17_537, "Imperial Fleet Regiment"),
        8 => (17_538, "Imperial Army Regiment"),
        9 => (17_539, "War Droid Regiment"),
        10 => (17_540, "Dark Trooper Regiment"),
        _ => return None,
    };
    Some((resource_id, label))
}

pub(crate) fn special_force_mini(dat_id: DatId) -> Option<(u32, &'static str)> {
    if dat_id.family() != 0x3c {
        return None;
    }
    let (resource_id, label) = match dat_id.index() {
        1 => (17_728, "Guerrillas"),
        2 => (17_729, "Infiltrators"),
        3 => (17_730, "Longprobe Y-wing Recon Team"),
        4 => (17_731, "Bothan Spies"),
        5 => (17_792, "Imperial Probe Droid"),
        6 => (17_793, "Imperial Espionage Droid"),
        7 => (17_794, "Imperial Commandos"),
        8 => (17_795, "Noghri Death Commandos"),
        9 => (17_796, "Bounty Hunters"),
        _ => return None,
    };
    Some((resource_id, label))
}

fn cockpit_faction(faction: CockpitFaction) -> Faction {
    match faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    }
}

fn clamp_to_galaxy(position: (i16, i16), layout: CockpitLayout) -> (i16, i16) {
    clamp_window_to_galaxy(position, layout, SYSTEM_WINDOW_WIDTH, SYSTEM_WINDOW_HEIGHT)
}

/// Clamp a window of `width` by `height` logical pixels into the galaxy
/// view (`FUN_0045aac0`, the view's `+0xcc..+0xd8` rect).
#[expect(
    clippy::cast_possible_truncation,
    reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
)]
pub(crate) fn clamp_window_to_galaxy(
    position: (i16, i16),
    layout: CockpitLayout,
    width: f32,
    height: f32,
) -> (i16, i16) {
    let scale = layout.scale.max(f32::EPSILON);
    let min_x = ((layout.galaxy.x - layout.canvas.x) / scale).round();
    let min_y = ((layout.galaxy.y - layout.canvas.y) / scale).round();
    let max_x = (min_x + layout.galaxy.width / scale - width).max(min_x);
    let max_y = (min_y + layout.galaxy.height / scale - height).max(min_y);
    (
        f32::from(position.0).clamp(min_x, max_x).round() as i16,
        f32::from(position.1).clamp(min_y, max_y).round() as i16,
    )
}

fn window_screen_rect(window: OpenSystemWindow, layout: CockpitLayout) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            layout.canvas.x + f32::from(window.logical_position.0) * layout.scale,
            layout.canvas.y + f32::from(window.logical_position.1) * layout.scale,
        ),
        egui::vec2(
            SYSTEM_WINDOW_WIDTH * layout.scale,
            SYSTEM_WINDOW_HEIGHT * layout.scale,
        ),
    )
}

fn rail_slot_rect(faction: CockpitFaction, index: usize) -> (f32, f32, f32, f32) {
    const ALLIANCE_Y: [f32; 12] = [
        61.0, 83.0, 105.0, 127.0, 149.0, 171.0, 193.0, 214.0, 236.0, 258.0, 279.0, 301.0,
    ];
    const EMPIRE_Y: [f32; 12] = [
        48.0, 72.0, 98.0, 122.0, 147.0, 172.0, 196.0, 221.0, 245.0, 270.0, 295.0, 319.0,
    ];
    match faction {
        CockpitFaction::Alliance => (544.0, ALLIANCE_Y[index], 62.0, 18.0),
        CockpitFaction::Empire => (21.0, EMPIRE_Y[index], 54.0, 18.0),
    }
}

fn cockpit_rect(layout: CockpitLayout, logical: (f32, f32, f32, f32)) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            layout.canvas.x + logical.0 * layout.scale,
            layout.canvas.y + logical.1 * layout.scale,
        ),
        egui::vec2(logical.2 * layout.scale, logical.3 * layout.scale),
    )
}

pub(crate) fn logical_rect(
    parent: egui::Rect,
    scale: f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> egui::Rect {
    egui::Rect::from_min_size(
        logical_point(parent, scale, x, y),
        egui::vec2(width * scale, height * scale),
    )
}

fn logical_point(parent: egui::Rect, scale: f32, x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(parent.min.x + x * scale, parent.min.y + y * scale)
}

pub(crate) fn exact_clicked(response: &egui::Response, rect: egui::Rect) -> bool {
    response.clicked()
        && response
            .interact_pointer_pos()
            .is_some_and(|point| rect_contains(rect, point))
}

pub(crate) fn rect_contains(rect: egui::Rect, point: egui::Pos2) -> bool {
    point.x >= rect.min.x && point.x < rect.max.x && point.y >= rect.min.y && point.y < rect.max.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cockpit::CockpitViewport;
    use crate::fleet_window::tests::PAINTED;
    use rebellion_core::dat::{ExplorationStatus, SectorGroup};
    use rebellion_core::manufacturing::{BuildableKind, QueueItem};
    use rebellion_core::world::ControlKind;
    use rebellion_core::world::{
        CapitalShipClass, ManufacturingFacilityInstance, ProductionFacilityInstance, Sector, System,
    };

    fn layout(faction: CockpitFaction, scale: f32) -> CockpitLayout {
        let (x, width, height) = match faction {
            CockpitFaction::Alliance => (55.0, 485.0, 350.0),
            CockpitFaction::Empire => (120.0, 480.0, 355.0),
        };
        CockpitLayout {
            canvas: CockpitViewport {
                x: 10.0,
                y: 20.0,
                width: 640.0 * scale,
                height: 480.0 * scale,
            },
            galaxy: CockpitViewport {
                x: 10.0 + x * scale,
                y: 20.0 + 40.0 * scale,
                width: width * scale,
                height: height * scale,
            },
            scale,
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
    )]
    fn fixture_world(count: usize) -> (GameWorld, Vec<SystemKey>) {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(36),
            name: "Sesswenna".into(),
            group: SectorGroup::Core,
            x: 317,
            y: 248,
            systems: Vec::new(),
        });
        let mut systems = Vec::new();
        for index in 0..count {
            let system = world.systems.insert(System {
                last_arrival: None,
                dat_id: DatId::new(100 + index as u32),
                name: format!("System {index}"),
                sector,
                x: 320 + index as u16,
                y: 250,
                exploration_status: ExplorationStatus::Explored,
                popularity_alliance: 1.0,
                popularity_empire: 0.0,
                is_populated: true,
                total_energy: 8,
                raw_materials: 6,
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
            });
            world.sectors[sector].systems.push(system);
            systems.push(system);
        }
        (world, systems)
    }

    fn add_yard(world: &mut GameWorld, system: SystemKey, class: u32) -> FacilityItem {
        let key = world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: DatId::new(class),
                side: rebellion_core::dat::Faction::Alliance,
                is_shipyard: class >> 24 == 0x28,
            });
        world.systems[system].manufacturing_facilities.push(key);
        FacilityItem::Manufacturing(key)
    }

    fn add_mine(world: &mut GameWorld, system: SystemKey) -> FacilityItem {
        let key = world
            .production_facilities
            .insert(ProductionFacilityInstance {
                class_dat_id: DatId::new(0x2c00_0001),
                side: rebellion_core::dat::Faction::Alliance,
                is_mine: true,
            });
        world.systems[system].production_facilities.push(key);
        FacilityItem::Production(key)
    }

    /// A painted text's words, top-left corner, width and font size.
    type RailText = (String, egui::Pos2, f32, f32);

    #[derive(Default)]
    struct Run {
        actions: Vec<SystemWindowAction>,
        texts: Vec<RailText>,
        painted: Vec<(u32, egui::Pos2)>,
        /// Each text's clip rectangle, by its string.
        clips: Vec<(String, egui::Rect)>,
        /// Each text's colour, by its string.
        colours: Vec<(String, egui::Color32)>,
        /// Each filled rectangle and its colour.
        fills: Vec<(egui::Rect, egui::Color32)>,
    }

    /// Draw the windows and rail on the Alliance's layout at scale 2, one
    /// frame per entry of `frames`, the Alliance seeing every system. Keeps
    /// every action and the last frame's texts and bitmaps.
    fn run_with(
        world: &GameWorld,
        manufacturing: &ManufacturingState,
        state: &mut SystemWindowState,
        frames: Vec<Vec<egui::Event>>,
    ) -> Run {
        let layout = layout(CockpitFaction::Alliance, 2.0);
        let ctx = egui::Context::default();
        crate::theme::load_fonts(&ctx);
        let mut cache = BmpCache::new();
        let fog = FogState::new(Faction::Alliance);
        let mut run = Run::default();
        for events in frames {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1000.0),
                )),
                events,
                ..Default::default()
            };
            PAINTED.with(|painted| painted.borrow_mut().clear());
            let output = ctx.run(input, |ctx| {
                run.actions.extend(draw_system_windows(
                    ctx,
                    world,
                    &rebellion_core::movement::MovementState::default(),
                    &fog,
                    &MissionState::new(),
                    manufacturing,
                    &DeliveryState::new(),
                    state,
                    CockpitFaction::Alliance,
                    layout,
                    &mut cache,
                ));
            });
            run.painted = PAINTED.with(|painted| painted.take());
            run.clips = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => {
                        Some((text.galley.text().to_owned(), clipped.clip_rect))
                    }
                    _ => None,
                })
                .collect();
            run.colours = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some((
                        text.galley.text().to_owned(),
                        text.galley.job.sections[0].format.color,
                    )),
                    _ => None,
                })
                .collect();
            run.fills = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect) => Some((rect.rect, rect.fill)),
                    _ => None,
                })
                .collect();
            run.texts = output
                .shapes
                .into_iter()
                .filter_map(|clipped| match clipped.shape {
                    egui::Shape::Text(text) => Some((
                        text.galley.text().to_owned(),
                        text.pos,
                        text.galley.size().x,
                        text.galley.job.sections[0].format.font_id.size,
                    )),
                    _ => None,
                })
                .collect();
        }
        run
    }

    fn run_rail(
        world: &GameWorld,
        state: &mut SystemWindowState,
        frames: Vec<Vec<egui::Event>>,
    ) -> (Vec<SystemWindowAction>, Vec<RailText>) {
        let run = run_with(world, &ManufacturingState::new(), state, frames);
        // The rail tests read the last frame's bitmaps from the record.
        PAINTED.with(|painted| *painted.borrow_mut() = run.painted);
        (run.actions, run.texts)
    }

    const ORIGIN: (i16, i16) = (100, 60);

    /// A logical point in the window opened at `ORIGIN` on the scale 2
    /// layout.
    fn at(x: f32, y: f32) -> egui::Pos2 {
        egui::pos2(
            10.0 + (f32::from(ORIGIN.0) + x) * 2.0,
            20.0 + (f32::from(ORIGIN.1) + y) * 2.0,
        )
    }

    fn opened(world: &GameWorld, system: SystemKey) -> SystemWindowState {
        let mut state = SystemWindowState::default();
        assert!(state.open(
            world,
            system,
            ORIGIN,
            CockpitFaction::Alliance,
            layout(CockpitFaction::Alliance, 2.0)
        ));
        state
    }

    #[test]
    fn a_mission_released_on_a_facility_page_targets_the_facility_under_the_point() {
        // +0x68 is FUN_00458b80: unless the page is the overview (0x67), the
        // item in list +0x174 at (8, 77) under the point, else the system.
        let (mut world, systems) = fixture_world(1);
        let system = systems[0];
        let yard = add_yard(&mut world, system, 0x2800_0001);
        let mine = add_mine(&mut world, system);
        let layout = layout(CockpitFaction::Alliance, 2.0);
        let mut state = SystemWindowState::default();
        assert!(state.open_tab(
            &world,
            system,
            SystemWindowTab::Shipyards,
            ORIGIN,
            CockpitFaction::Alliance,
            layout,
        ));
        let fog = FogState::new(Faction::Alliance);
        let layer = egui::LayerId::new(egui::Order::Foreground, area_id(system));
        let pick = |state: &SystemWindowState, point| {
            state.mission_target(
                &world,
                &fog,
                &ManufacturingState::new(),
                &DeliveryState::new(),
                layout,
                layer,
                point,
            )
        };
        let cell = |state: &SystemWindowState, index| {
            state
                .cell_screen_rect(layout, system, index)
                .unwrap()
                .center()
        };
        let FacilityItem::Manufacturing(yard) = yard else {
            unreachable!()
        };
        let FacilityItem::Production(mine) = mine else {
            unreachable!()
        };

        assert_eq!(
            pick(&state, cell(&state, 0)).unwrap().unwrap().object,
            Some(MissionTarget::ManufacturingFacility(yard))
        );
        // The empty cell next to it, and the window's frame, give the system.
        assert_eq!(
            pick(&state, cell(&state, 1)),
            Some(Some(MissionTargetPick::from(system)))
        );
        let frame = state.cell_screen_rect(layout, system, 0).unwrap().min - egui::vec2(4.0, 4.0);
        assert_eq!(
            pick(&state, frame),
            Some(Some(MissionTargetPick::from(system)))
        );
        state.select_tab(system, SystemWindowTab::Mines);
        assert_eq!(
            pick(&state, cell(&state, 0)).unwrap().unwrap().object,
            Some(MissionTarget::ProductionFacility(mine))
        );
        // An empty deposit's cell drops the order (hyp: no object).
        assert_eq!(pick(&state, cell(&state, 1)), Some(None));
        // The overview is page 0x67: always the system.
        state.select_tab(system, SystemWindowTab::Overview);
        assert_eq!(
            pick(&state, cell(&state, 0)),
            Some(Some(MissionTargetPick::from(system)))
        );
        let elsewhere = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("elsewhere"));
        assert_eq!(
            state.mission_target(
                &world,
                &fog,
                &ManufacturingState::new(),
                &DeliveryState::new(),
                layout,
                elsewhere,
                cell(&state, 0),
            ),
            None
        );
    }

    fn hover(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
        vec![
            vec![egui::Event::PointerMoved(point)],
            vec![egui::Event::PointerMoved(point)],
        ]
    }

    fn click_with(point: egui::Pos2, button: egui::PointerButton) -> Vec<Vec<egui::Event>> {
        let event = |pressed| egui::Event::PointerButton {
            pos: point,
            button,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        let mut frames = hover(point);
        frames.extend([vec![event(true)], vec![event(false)], vec![]]);
        frames
    }

    fn click(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
        click_with(point, egui::PointerButton::Primary)
    }

    fn right_click(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
        click_with(point, egui::PointerButton::Secondary)
    }

    fn text<'a>(run: &'a Run, words: &str) -> &'a RailText {
        run.texts
            .iter()
            .find(|value| value.0 == words)
            .unwrap_or_else(|| panic!("no {words:?} in {:?}", run.texts))
    }

    #[test]
    fn opening_clamps_to_faction_galaxy_and_deduplicates() {
        // FUN_0045aac0 clamps the 226 by 304 window into the galaxy view:
        // 55 + 485 - 226 and 40 + 350 - 304.
        let (world, systems) = fixture_world(1);
        let mut state = SystemWindowState::default();
        let layout = layout(CockpitFaction::Alliance, 2.0);
        assert!(state.open(
            &world,
            systems[0],
            (620, 470),
            CockpitFaction::Alliance,
            layout
        ));
        assert_eq!(state.windows[0].logical_position, (314, 86));
        assert_eq!(state.windows[0].tab, SystemWindowTab::Overview);
        assert!(state.open(&world, systems[0], (0, 0), CockpitFaction::Alliance, layout));
        assert_eq!(state.window_count(), 1);
    }

    #[test]
    fn minimize_caps_at_twelve_and_evicts_oldest() {
        let (world, systems) = fixture_world(13);
        let mut state = SystemWindowState::default();
        let layout = layout(CockpitFaction::Alliance, 1.0);
        for system in &systems {
            state.open(&world, *system, (100, 50), CockpitFaction::Alliance, layout);
            assert!(state.minimize(*system));
        }
        assert_eq!(state.rail_count(), REFERENCE_RAIL_SLOTS);
        assert_eq!(state.rail[0].system(), systems[1]);
        assert_eq!(state.rail[11].system(), systems[12]);
    }

    #[test]
    fn restore_preserves_position_tab_and_selection() {
        let (mut world, systems) = fixture_world(1);
        let mine = add_mine(&mut world, systems[0]);
        let mut state = SystemWindowState::default();
        let layout = layout(CockpitFaction::Alliance, 1.0);
        state.open(
            &world,
            systems[0],
            (100, 60),
            CockpitFaction::Alliance,
            layout,
        );
        state.select_tab(systems[0], SystemWindowTab::Mines);
        state.select_item(systems[0], Some(mine));
        state.minimize(systems[0]);
        assert!(state.restore(systems[0]));
        assert_eq!(state.window_count(), 1);
        assert_eq!(state.rail_count(), 0);
        assert_eq!(state.windows[0].logical_position, (100, 60));
        assert_eq!(state.windows[0].tab, SystemWindowTab::Mines);
        assert_eq!(state.windows[0].selected_item, Some(mine));

        // The same page keeps the selection; another clears it.
        state.select_tab(systems[0], SystemWindowTab::Mines);
        assert_eq!(state.windows[0].selected_item, Some(mine));
        state.select_tab(systems[0], SystemWindowTab::Refineries);
        assert_eq!(state.windows[0].selected_item, None);
    }

    #[test]
    fn faction_change_clears_visible_and_minimized_windows() {
        let (world, systems) = fixture_world(2);
        let mut state = SystemWindowState::default();
        let alliance = layout(CockpitFaction::Alliance, 1.0);
        state.open(
            &world,
            systems[0],
            (100, 60),
            CockpitFaction::Alliance,
            alliance,
        );
        state.open(
            &world,
            systems[1],
            (100, 60),
            CockpitFaction::Alliance,
            alliance,
        );
        state.minimize(systems[0]);
        state.prepare_faction(CockpitFaction::Empire);
        assert_eq!(state.window_count(), 0);
        assert_eq!(state.rail_count(), 0);
    }

    #[test]
    fn each_tab_shows_its_side_art_pressed_or_empty() {
        // FUN_00455060: the overview's art by side; a page's normal and
        // pressed art, its empty art in place of the normal one.
        assert_eq!(
            [1, 2, 3, 0].map(|side| tab_resource(SystemWindowTab::Overview, side, false, true)),
            [10_312, 10_315, 10_318, 10_318]
        );
        assert_eq!(
            [1, 2, 3].map(|side| tab_resource(SystemWindowTab::Overview, side, true, false)),
            [10_311, 10_314, 10_317]
        );
        let pages = [
            (SystemWindowTab::Shipyards, [10_327, 10_326, 10_328]),
            (
                SystemWindowTab::TrainingFacilities,
                [10_330, 10_329, 10_331],
            ),
            (SystemWindowTab::ConstructionYards, [10_333, 10_332, 10_334]),
            (SystemWindowTab::Refineries, [10_324, 10_323, 10_325]),
            (SystemWindowTab::Mines, [10_321, 10_320, 10_322]),
        ];
        for (tab, [normal, pressed, empty]) in pages {
            assert_eq!(tab_resource(tab, 1, false, false), normal, "{tab:?}");
            assert_eq!(tab_resource(tab, 2, true, true), pressed, "{tab:?}");
            assert_eq!(tab_resource(tab, 3, false, true), empty, "{tab:?}");
        }
        // The help messages (TEXTSTRA 6197, 6184, 6192, 6194, 6183, 6182)
        // and the strip's x positions.
        assert_eq!(
            SystemWindowTab::ALL.map(SystemWindowTab::name),
            [
                "Manufacturing",
                "Shipyards",
                "Training Facilities",
                "Construction Yards",
                "Refineries",
                "Mines"
            ]
        );
        assert_eq!(
            SystemWindowTab::ALL.map(SystemWindowTab::x),
            [0.0, 39.0, 77.0, 115.0, 152.0, 190.0]
        );
    }

    #[test]
    fn maps_original_system_item_resource_families_without_fallbacks() {
        // Source: GOKRES.DLL mini-icon resource blocks and DAT record families.
        assert_eq!(
            character_mini_resource_id(DatId::new(576), true),
            Some(18_496)
        );
        assert_eq!(
            character_mini_resource_id(DatId::new(641), true),
            Some(18_561)
        );
        assert_eq!(
            character_mini_resource_id(DatId::new(832), false),
            Some(19_008)
        );
        assert_eq!(
            character_mini_resource_id(DatId::new(923), false),
            Some(19_099)
        );

        assert_eq!(
            manufacturing_facility_mini(DatId::new(0x2800_0001)),
            Some((16_640, "Orbital Shipyard"))
        );
        assert_eq!(
            manufacturing_facility_mini(DatId::new(0x2a00_0006)),
            Some((16_645, "Advanced Construction Yard"))
        );
        assert_eq!(
            manufacturing_facility_mini(DatId::new(0x2000_0001)),
            Some((17_216, "Alliance Headquarters"))
        );
        assert_eq!(
            defense_facility_mini(DatId::new(0x2400_0003)),
            Some((16_898, "GenCore Level I"))
        );
        assert_eq!(
            defense_facility_mini(DatId::new(0x2500_0004)),
            Some((17_024, "Death Star Shield"))
        );
        assert_eq!(
            production_facility_mini(DatId::new(0x2c00_0001)),
            Some((16_385, "Mine"))
        );
        assert_eq!(
            production_facility_mini(DatId::new(0x2d00_0002)),
            Some((16_386, "Refinery"))
        );
        assert_eq!(
            troop_mini(DatId::new(0x1000_0001)),
            Some((17_472, "Alliance Fleet Regiment"))
        );
        assert_eq!(
            troop_mini(DatId::new(0x1000_000a)),
            Some((17_540, "Dark Trooper Regiment"))
        );
        assert_eq!(
            special_force_mini(DatId::new(0x3c00_0001)),
            Some((17_728, "Guerrillas"))
        );
        assert_eq!(
            special_force_mini(DatId::new(0x3c00_0009)),
            Some((17_796, "Bounty Hunters"))
        );

        assert_eq!(character_mini_resource_id(DatId::new(575), true), None);
        assert_eq!(manufacturing_facility_mini(DatId::new(0x2700_0001)), None);
        assert_eq!(defense_facility_mini(DatId::new(0x2400_0007)), None);
        assert_eq!(production_facility_mini(DatId::new(0x2d00_0003)), None);
        assert_eq!(troop_mini(DatId::new(0x1000_000b)), None);
        assert_eq!(special_force_mini(DatId::new(0x3c00_000a)), None);
    }

    #[test]
    fn the_window_paints_its_chrome_tabs_and_the_overview_first() {
        // FUN_00455060 / FUN_00456230: 10297 at (0, 0), the focused side 1
        // strip at (2, 2), the title buttons at 3, 195 and 209, the six tabs
        // at (x, 20), the yard column at (6, 71), and each band's frame and
        // unselected side 1 strip at its corner (FUN_00458080).
        let (mut world, systems) = fixture_world(1);
        add_yard(&mut world, systems[0], 0x2800_0001);
        let mut state = opened(&world, systems[0]);
        let run = run_with(
            &world,
            &ManufacturingState::new(),
            &mut state,
            hover(at(150.0, 10.0)),
        );
        for (id, point) in [
            (WINDOW_BACKGROUND, at(0.0, 0.0)),
            (10_299, at(2.0, 2.0)),
            (SECTOR_NORMAL, at(3.0, 3.0)),
            (MINIMIZE_NORMAL, at(195.0, 3.0)),
            (CLOSE_NORMAL, at(209.0, 3.0)),
            (10_311, at(0.0, 20.0)),
            (10_327, at(39.0, 20.0)),
            (10_331, at(77.0, 20.0)),
            (10_334, at(115.0, 20.0)),
            (10_325, at(152.0, 20.0)),
            (10_322, at(190.0, 20.0)),
            (10_298, at(6.0, 71.0)),
            (10_290, at(55.0, 57.0)),
            (10_292, at(55.0, 57.0)),
            (10_292, at(55.0, 138.0)),
            (10_292, at(55.0, 219.0)),
        ] {
            assert!(
                run.painted.contains(&(id, point)),
                "{id} at {point:?} in {:?}",
                run.painted
            );
        }
        // The title at (19, 2), font 5; the bands' lines in font 10 from
        // (5, 1) and (5, 16) and (5, 57); the counts centred in the yard
        // column.
        let title = text(&run, "System 0");
        assert_eq!(
            (title.1.x, title.3),
            (at(19.0, 0.0).x, crate::theme::original_font(5, 2.0).size)
        );
        let heading = text(&run, "Ship Construction");
        // FUN_00458080: the title keeps the text object's black
        // (FUN_00601930), the lines are white; FUN_00457690's counts keep
        // the default font, a 16-row cell.
        let colour = |name: &str| {
            run.colours
                .iter()
                .find(|(text, _)| text == name)
                .map(|(_, colour)| *colour)
        };
        assert_eq!(colour("Ship Construction"), Some(egui::Color32::BLACK));
        assert_eq!(
            colour("No Ships are being built"),
            Some(egui::Color32::WHITE)
        );
        assert!(run.texts.iter().any(|value| value.0.contains(':')
            && value.3 == crate::theme::original_font(COUNT_FONT, 2.0).size));
        assert_eq!(
            (heading.1, heading.3),
            (at(60.0, 58.0), crate::theme::original_font(10, 2.0).size)
        );
        assert_eq!(text(&run, "No Ships are being built").1, at(60.0, 73.0));
        assert_eq!(text(&run, "No Troops in training").1, at(60.0, 154.0));
        assert_eq!(
            text(&run, "Facilities Under Construction").1,
            at(60.0, 220.0)
        );
        assert_eq!(
            run.texts
                .iter()
                .filter(|value| value.0 == "Destination: System 0")
                .count(),
            3
        );
        let ships = run
            .texts
            .iter()
            .find(|value| value.0 == "1:1")
            .unwrap_or_else(|| panic!("{:?}", run.texts));
        assert_eq!(ships.1.y, at(0.0, 119.0).y);
        assert!((ships.1.x + ships.2 / 2.0 - at(29.0, 0.0).x).abs() < 0.01);
        assert_eq!(run.texts.iter().filter(|value| value.0 == "0:0").count(), 2);
        assert!(!run.texts.iter().any(|value| value.0 == "Manufacturing"));
    }

    #[test]
    fn a_building_band_shows_its_product_and_units() {
        // FUN_00457c90: the product's name, mini at (40, 15) and units at
        // (5, 47).
        let (mut world, systems) = fixture_world(1);
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x1400_0040),
            name: "Corellian Corvette".into(),
            ..CapitalShipClass::default()
        });
        let mut manufacturing = ManufacturingState::new();
        manufacturing.build(
            systems[0],
            &QueueItem::new(BuildableKind::CapitalShip(class), 10),
            2,
        );
        let mut state = opened(&world, systems[0]);
        let run = run_with(&world, &manufacturing, &mut state, hover(at(150.0, 10.0)));
        assert_eq!(text(&run, "Corellian Corvette").1, at(60.0, 73.0));
        assert_eq!(text(&run, "Building: 2").1, at(60.0, 104.0));
        let mini = crate::minis::capital_ship_mini_id(DatId::new(0x1400_0040)).unwrap();
        assert!(
            run.painted.contains(&(mini, at(95.0, 72.0))),
            "{:?}",
            run.painted
        );
        // FUN_004acec0: the progress bar's black, then its 0x200f0f0 part
        // (yellow) from the bar's left.
        // The area still fades in, so its colours are premultiplied.
        let bar = egui::Rect::from_min_size(at(56.0, 127.0), egui::vec2(320.0, 8.0));
        let yellow =
            |colour: egui::Color32| colour.r() > 0 && colour.r() == colour.g() && colour.b() == 0;
        assert!(run
            .fills
            .iter()
            .any(|(rect, colour)| *rect == bar && colour.r() == 0 && colour.b() == 0));
        assert!(
            run.fills.iter().any(|(rect, colour)| yellow(*colour)
                && rect.min == bar.min
                && rect.width() < bar.width()),
            "{:?}",
            run.fills
        );
    }

    #[test]
    fn a_press_on_a_band_selects_it_and_a_right_click_opens_its_menu() {
        // FUN_00458480's band rects; a press sets band +0x30 bit 0 and swaps
        // its strip (FUN_00456230); a right release opens the menu for the
        // selected managers (FUN_00453ee0, FUN_004ac5c0).
        let (world, systems) = fixture_world(1);
        let mut state = opened(&world, systems[0]);
        let manufacturing = ManufacturingState::new();
        let run = run_with(&world, &manufacturing, &mut state, click(at(100.0, 170.0)));
        assert_eq!(
            state.selection(systems[0]),
            Some((
                SystemWindowTab::Overview,
                Some(ProductionArea::TrainingFacility)
            ))
        );
        assert!(run
            .actions
            .contains(&SystemWindowAction::SelectSystem(systems[0])));
        assert!(!run
            .actions
            .iter()
            .any(|action| matches!(action, SystemWindowAction::OpenObjectMenu { .. })));
        let after = run_with(&world, &manufacturing, &mut state, hover(at(150.0, 10.0)));
        assert!(after.painted.contains(&(10_291, at(55.0, 138.0))));
        assert!(after.painted.contains(&(10_292, at(55.0, 57.0))));

        let menu = run_with(
            &world,
            &manufacturing,
            &mut state,
            right_click(at(100.0, 250.0)),
        );
        assert_eq!(
            state.selection(systems[0]),
            Some((
                SystemWindowTab::Overview,
                Some(ProductionArea::ConstructionYard)
            ))
        );
        assert!(menu.actions.contains(&SystemWindowAction::OpenObjectMenu {
            system: systems[0],
            selection: Some(MenuObject::Producer {
                system: systems[0],
                area: ProductionArea::ConstructionYard,
            }),
            point: (200, 310),
        }));
        assert_eq!(
            state.band_screen_rect(
                layout(CockpitFaction::Alliance, 2.0),
                systems[0],
                ProductionArea::Shipyard
            ),
            Some(egui::Rect::from_min_max(at(55.0, 57.0), at(221.0, 136.0)))
        );
    }

    #[test]
    fn the_window_clips_its_title_strip_to_its_own_width() {
        // FUN_0045aac0 creates type 9 at 226 by 304, and its title strip
        // (STRATEGY 10299.., 231 wide at x 2) is the one the 235-wide Fleet,
        // Defenses and Missions windows use, so the window must cut it off.
        let (world, systems) = fixture_world(1);
        let mut state = opened(&world, systems[0]);
        let run = run_with(
            &world,
            &ManufacturingState::new(),
            &mut state,
            hover(at(150.0, 10.0)),
        );
        let name = &world.systems[systems[0]].name;
        let clip = run
            .clips
            .iter()
            .find(|(text, _)| text == name)
            .map(|(_, clip)| *clip)
            .expect("the title is drawn");
        let window =
            egui::Rect::from_min_max(at(0.0, 0.0), at(SYSTEM_WINDOW_WIDTH, SYSTEM_WINDOW_HEIGHT));
        assert!(window.contains_rect(clip), "{clip:?} in {window:?}");
        assert!((clip.right() - window.right()).abs() < 0.01, "{clip:?}");
    }

    #[test]
    fn a_facility_page_shows_its_label_pictures_and_selected_frame() {
        // FUN_004568a0: the page label at (0, 58), centred; the list at
        // (8, 77) in 69 by 40 cells, each picture 1 by 2 in; a selected
        // item adds its side frame (+0x15c). The mines page pads the
        // system's deposits with 9005.
        let (mut world, systems) = fixture_world(1);
        world.systems[systems[0]].raw_materials = 2;
        let mine = add_mine(&mut world, systems[0]);
        let mut state = opened(&world, systems[0]);
        let manufacturing = ManufacturingState::new();
        let _ = run_with(&world, &manufacturing, &mut state, click(at(200.0, 30.0)));
        assert_eq!(state.windows[0].tab, SystemWindowTab::Mines);

        let run = run_with(&world, &manufacturing, &mut state, click(at(20.0, 90.0)));
        assert_eq!(state.windows[0].selected_item, Some(mine));
        let after = run_with(&world, &manufacturing, &mut state, hover(at(150.0, 10.0)));
        assert!(
            after.painted.contains(&(9_001, at(9.0, 79.0))),
            "{:?}",
            after.painted
        );
        assert!(after.painted.contains(&(9_005, at(78.0, 79.0))));
        assert!(after.painted.contains(&(10_262, at(9.0, 79.0))));
        assert!(after.painted.contains(&(10_320, at(190.0, 20.0))));
        let label = text(&after, "Mines");
        assert!((label.1.x + label.2 / 2.0 - at(113.0, 0.0).x).abs() < 0.01);
        assert_eq!((label.1.y, label.3), (at(0.0, 58.0).y, 20.0));
        assert!(run
            .actions
            .contains(&SystemWindowAction::SelectSystem(systems[0])));

        // A right press selects too and opens nothing; a press on empty list
        // space clears the selection.
        let right = run_with(
            &world,
            &manufacturing,
            &mut state,
            right_click(at(80.0, 90.0)),
        );
        assert_eq!(
            state.windows[0].selected_item,
            Some(FacilityItem::Deposit(1))
        );
        assert!(!right
            .actions
            .iter()
            .any(|action| matches!(action, SystemWindowAction::OpenObjectMenu { .. })));
        let _ = run_with(&world, &manufacturing, &mut state, click(at(100.0, 250.0)));
        assert_eq!(state.windows[0].selected_item, None);
        assert_eq!(
            state.cell_screen_rect(layout(CockpitFaction::Alliance, 2.0), systems[0], 4),
            Some(egui::Rect::from_min_max(at(77.0, 117.0), at(146.0, 157.0)))
        );
    }

    /// Press at screen point `from`, move to `to` and hold; then release.
    /// Returns the window's position while held and after the release.
    fn move_window(
        world: &GameWorld,
        state: &mut SystemWindowState,
        from: egui::Pos2,
        to: egui::Pos2,
    ) -> ((i16, i16), (i16, i16)) {
        let manufacturing = ManufacturingState::new();
        let mut frames = hover(from);
        frames.extend([
            vec![egui::Event::PointerButton {
                pos: from,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            }],
            vec![egui::Event::PointerMoved(to)],
            vec![egui::Event::PointerMoved(to)],
        ]);
        run_with(world, &manufacturing, state, frames);
        let held = state.windows[0].logical_position;
        let release = egui::Event::PointerButton {
            pos: to,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        };
        run_with(world, &manufacturing, state, vec![vec![release], vec![]]);
        (held, state.windows[0].logical_position)
    }

    // FUN_00452fc0 sets the window's +0xb8 bit 4, so a left press on its own
    // surface holds it (FUN_00606ee0); it follows the pointer unbounded
    // (FUN_006071e0) and the drop puts it back inside the galaxy view
    // (FUN_00607010). A press on a band goes to the band.
    #[test]
    fn the_window_moves_by_its_surface_and_drops_inside_the_galaxy_view() {
        let (world, systems) = fixture_world(1);
        let mut state = opened(&world, systems[0]);
        // Hovering takes no focus.
        let manufacturing = ManufacturingState::new();
        let idle = run_with(&world, &manufacturing, &mut state, hover(at(100.0, 10.0)));
        assert!(idle.actions.is_empty());
        let (held, dropped) = move_window(&world, &mut state, at(100.0, 10.0), at(130.0, 30.0));
        assert_eq!((held, dropped), ((130, 80), (130, 80)));
        // Past the Alliance galaxy view's left edge (55) while held.
        let from = at(130.0, 30.0);
        let to = from - egui::vec2(200.0, 0.0);
        let (held, dropped) = move_window(&world, &mut state, from, to);
        assert_eq!((held, dropped), ((30, 80), (55, 80)));
        let window = window_screen_rect(state.windows[0], layout(CockpitFaction::Alliance, 2.0));
        let band = band_rect(window, 2.0, BAND_AREAS[0]).center();
        let (held, dropped) = move_window(&world, &mut state, band, band + egui::vec2(60.0, 60.0));
        assert_eq!((held, dropped), ((55, 80), (55, 80)));
    }

    #[test]
    fn the_title_buttons_open_the_sector_minimize_and_close() {
        let (world, systems) = fixture_world(1);
        let manufacturing = ManufacturingState::new();
        let mut state = opened(&world, systems[0]);
        let sector = run_with(&world, &manufacturing, &mut state, click(at(9.0, 9.0)));
        assert!(sector
            .actions
            .contains(&SystemWindowAction::FocusSector(systems[0])));
        assert_eq!(state.window_count(), 0);

        let mut state = opened(&world, systems[0]);
        let _ = run_with(&world, &manufacturing, &mut state, click(at(201.0, 9.0)));
        assert_eq!((state.window_count(), state.rail_count()), (0, 1));

        let mut state = opened(&world, systems[0]);
        let _ = run_with(&world, &manufacturing, &mut state, click(at(215.0, 9.0)));
        assert_eq!((state.window_count(), state.rail_count()), (0, 0));
    }

    #[test]
    fn the_other_sides_production_shows_only_where_its_contents_show() {
        // hyp: as the Defenses window's pages, the other side's facilities
        // and production show only where the player sees its objects.
        let (mut world, systems) = fixture_world(1);
        world.systems[systems[0]].control = ControlKind::Controlled(Faction::Empire);
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x1400_0040),
            name: "Star Destroyer".into(),
            ..CapitalShipClass::default()
        });
        let mut manufacturing = ManufacturingState::new();
        manufacturing.build(
            systems[0],
            &QueueItem::new(BuildableKind::CapitalShip(class), 10),
            1,
        );
        let mut state = opened(&world, systems[0]);
        let run = run_with(&world, &manufacturing, &mut state, hover(at(150.0, 10.0)));
        assert!(run
            .texts
            .iter()
            .any(|value| value.0 == "No Ships are being built"));
        assert!(!run.texts.iter().any(|value| value.0 == "Star Destroyer"));
        // The side 2 strip and tab art.
        assert!(run.painted.contains(&(10_294, at(55.0, 57.0))));
        assert!(run.painted.contains(&(10_314, at(0.0, 20.0))));
    }

    #[test]
    fn a_minimized_defenses_window_keeps_its_own_rail_slot_and_restores_from_it() {
        // FUN_004aa4a0: a Defenses window minimizes (0x466) to its own rail
        // entry beside the same system's Fleet window, one per window.
        let (world, systems) = fixture_world(1);
        let layout = layout(CockpitFaction::Alliance, 2.0);
        let mut state = SystemWindowState::default();
        state.minimize_defenses_window(systems[0], (90, 70));
        state.minimize_fleet_window(systems[0], (100, 80));
        state.minimize_defenses_window(systems[0], (90, 70));
        assert_eq!(state.rail_count(), 2);
        assert_eq!(
            state.rail_entries().collect::<Vec<_>>(),
            [(systems[0], "fleet"), (systems[0], "defenses")]
        );

        let slot = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 1));
        assert_eq!(state.rail_slot_screen_rect(layout, 1), Some(slot));
        assert_eq!(state.rail_slot_screen_rect(layout, 2), None);
        let point = slot.center();
        let press = |pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
        let (actions, _) = run_rail(
            &world,
            &mut state,
            vec![
                vec![egui::Event::PointerMoved(point)],
                vec![egui::Event::PointerMoved(point)],
                vec![press(true)],
                vec![press(false)],
            ],
        );
        let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.take());
        assert!(
            painted.contains(&(
                crate::defenses_window::rail_icon(&world, systems[0]),
                slot.min
            )),
            "{painted:?}"
        );
        assert!(
            actions.contains(&SystemWindowAction::RestoreDefensesWindow {
                system: systems[0],
                logical_position: (90, 70),
            })
        );
        assert_eq!(state.rail_count(), 1);
        assert!(matches!(state.rail[0], RailEntry::Fleet { .. }));
    }

    #[test]
    fn a_minimized_missions_window_keeps_its_own_rail_slot_and_restores_from_it() {
        // FUN_004a21c0: a Missions window minimizes (0x466) to its own rail
        // entry beside the same system's Defenses window, one per window;
        // with no mission there its icon is side 3's 11541.
        let (world, systems) = fixture_world(1);
        let layout = layout(CockpitFaction::Alliance, 2.0);
        let mut state = SystemWindowState::default();
        state.minimize_defenses_window(systems[0], (90, 70));
        state.minimize_missions_window(systems[0], (60, 40));
        state.minimize_missions_window(systems[0], (60, 40));
        assert_eq!(
            state.rail_entries().collect::<Vec<_>>(),
            [(systems[0], "defenses"), (systems[0], "missions")]
        );

        let slot = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 1));
        let point = slot.center();
        let press = |pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
        let (actions, _) = run_rail(
            &world,
            &mut state,
            vec![
                vec![egui::Event::PointerMoved(point)],
                vec![egui::Event::PointerMoved(point)],
                vec![press(true)],
                vec![press(false)],
            ],
        );
        let painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.take());
        assert!(painted.contains(&(11541, slot.min)), "{painted:?}");
        assert!(
            actions.contains(&SystemWindowAction::RestoreMissionsWindow {
                system: systems[0],
                logical_position: (60, 40),
            })
        );
        assert_eq!(state.rail_count(), 1);
        assert!(matches!(state.rail[0], RailEntry::Defenses { .. }));
    }

    #[test]
    fn a_minimized_fleet_window_keeps_one_rail_slot_beside_its_systems_window() {
        // FUN_004a76e0: a Fleet window minimizes to the rail as its own
        // entry; the rail holds one entry per window.
        let (world, systems) = fixture_world(2);
        let layout = layout(CockpitFaction::Alliance, 2.0);
        let mut state = SystemWindowState::default();
        state.minimize_fleet_window(systems[0], (100, 80));
        state.minimize_fleet_window(systems[0], (100, 80));
        assert_eq!(state.rail_count(), 1);
        state.open(
            &world,
            systems[0],
            (60, 40),
            CockpitFaction::Alliance,
            layout,
        );
        assert!(state.minimize(systems[0]));
        state.minimize_fleet_window(systems[1], (120, 90));
        assert_eq!(state.rail_count(), 3);
        assert_eq!(
            state
                .rail_entries()
                .map(|(_, kind)| kind)
                .collect::<Vec<_>>(),
            ["fleet", "system", "fleet"]
        );

        // FUN_00428d70: each slot draws its window's icon (slot +0x74) at the
        // corner and the system's name after the icon in font 10; the System
        // window's icon is 11530 for side 1 (FUN_00458cf0). The fixture has
        // no bitmaps, so the icon's width is 0 here.
        let slot = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 0));
        let second = cockpit_rect(layout, rail_slot_rect(CockpitFaction::Alliance, 1));
        let away = egui::pos2(2.0, 2.0);
        let run = run_with(
            &world,
            &ManufacturingState::new(),
            &mut state,
            vec![
                vec![egui::Event::PointerMoved(away)],
                vec![egui::Event::PointerMoved(away)],
            ],
        );
        let font = crate::theme::original_font(10, 2.0).size;
        for corner in [slot.min, second.min] {
            assert!(
                run.texts
                    .iter()
                    .any(|(text, pos, _, size)| text == "System 0"
                        && *pos == corner
                        && *size == font),
                "{:?}",
                run.texts
            );
        }
        assert!(run
            .painted
            .iter()
            .any(|&(id, pos)| id == RAIL_ICONS[0] && pos == second.min));

        let point = slot.center();
        let press = |pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        let (actions, _) = run_rail(
            &world,
            &mut state,
            vec![
                vec![egui::Event::PointerMoved(point)],
                vec![egui::Event::PointerMoved(point)],
                vec![press(true)],
                vec![press(false)],
            ],
        );
        assert!(actions.contains(&SystemWindowAction::RestoreFleetWindow {
            system: systems[0],
            logical_position: (100, 80),
        }));
        assert_eq!(state.rail_count(), 2);
        assert!(state.rail[0].is_system_window());
        assert_eq!(state.rail[0].system(), systems[0]);
        assert_eq!(state.rail[1].system(), systems[1]);
    }
    // FUN_00458cf0: a System window's rail icon by its system's side bits.
    #[test]
    fn a_system_windows_rail_icon_follows_the_systems_side() {
        let (mut world, systems) = fixture_world(1);
        let mut icon = |control| {
            world.systems[systems[0]].control = control;
            rail_icon(&world, systems[0])
        };
        assert_eq!(icon(ControlKind::Controlled(Faction::Alliance)), 11530);
        assert_eq!(icon(ControlKind::Controlled(Faction::Empire)), 11531);
        assert_eq!(icon(ControlKind::Uncontrolled), 11532);
        assert_eq!(icon(ControlKind::Contested), 11532);
    }

    #[test]
    fn reference_rail_rectangles_differ_by_faction() {
        // The galaxy view's SetRect calls at 0x421f3b (side 1, 544..606) and
        // 0x42207e (side 2, 21..75), 18 rows high.
        assert_eq!(
            rail_slot_rect(CockpitFaction::Alliance, 0),
            (544.0, 61.0, 62.0, 18.0)
        );
        assert_eq!(
            rail_slot_rect(CockpitFaction::Alliance, 11),
            (544.0, 301.0, 62.0, 18.0)
        );
        assert_eq!(
            rail_slot_rect(CockpitFaction::Empire, 0),
            (21.0, 48.0, 54.0, 18.0)
        );
        assert_eq!(
            rail_slot_rect(CockpitFaction::Empire, 11),
            (21.0, 319.0, 54.0, 18.0)
        );
    }

    #[test]
    fn a_window_opens_on_the_tab_it_is_asked_for() {
        // Our own: a window opened on a page.
        let (mut world, systems) = fixture_world(2);
        let layout = layout(CockpitFaction::Alliance, 2.0);
        let mut state = SystemWindowState::default();
        let missing = systems[1];
        world.systems.remove(missing);

        assert!(state.open_tab(
            &world,
            systems[0],
            SystemWindowTab::Refineries,
            (60, 40),
            CockpitFaction::Alliance,
            layout
        ));
        assert_eq!(state.windows[0].tab, SystemWindowTab::Refineries);
        assert!(!state.open_tab(
            &world,
            missing,
            SystemWindowTab::Refineries,
            (60, 40),
            CockpitFaction::Alliance,
            layout
        ));
        assert_eq!(state.windows.len(), 1);

        // FUN_0060d590: the tab buttons sit at (x, 20), 36 by 33.
        let origin = window_screen_rect(state.windows[0], layout).min;
        assert_eq!(
            state.tab_screen_rect(layout, systems[0], SystemWindowTab::Refineries),
            Some(egui::Rect::from_min_size(
                origin + egui::vec2(152.0, 20.0) * 2.0,
                egui::vec2(72.0, 66.0)
            ))
        );
        assert_eq!(
            state.tab_screen_rect(layout, missing, SystemWindowTab::Refineries),
            None
        );
    }

    #[test]
    fn system_window_occlusion_uses_scaled_exclusive_edges() {
        // FUN_0045aac0: 226 by 304 at (100, 60), scale 2, canvas at (10, 20).
        let (world, systems) = fixture_world(1);
        let mut state = SystemWindowState::default();
        let layout = layout(CockpitFaction::Alliance, 2.0);
        state.open(
            &world,
            systems[0],
            (100, 60),
            CockpitFaction::Alliance,
            layout,
        );
        assert!(state.contains_screen_point(layout, (210.0, 140.0)));
        assert!(state.contains_screen_point(layout, (661.9, 747.9)));
        assert!(!state.contains_screen_point(layout, (662.0, 140.0)));
        assert!(!state.contains_screen_point(layout, (210.0, 748.0)));
        let layer = egui::LayerId::new(egui::Order::Foreground, area_id(systems[0]));
        assert_eq!(state.release_target(layer), Some(systems[0]));
    }
}
