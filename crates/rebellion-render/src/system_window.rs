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
use rebellion_core::manufacturing::{BuildableKind, ManufacturingState, ProductionArea};
use rebellion_core::missions::{MissionState, MissionTarget, MissionTargetPick};
pub(crate) use rebellion_core::pictures::{character_mini_resource_id, character_picture};
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::fleet_window::{control_side, paint_native};
use crate::manufacturing_window::{
    band, band_strip, contents_visible, item_frame, yard_count, yard_page,
    FacilityItem, FacilityPage, BAND_AREAS, BAND_DESTINATION, BAND_FRAME, BAND_PORTRAIT,
    BAND_RECTS, BAND_STATUS, BAND_TITLE, BAND_UNITS, CELL, COLUMNS, COUNT_ORIGINS, LIST,
    PROGRESS_RECTS, YARD_COLUMN,
};
use crate::object_menu::{MenuObject, ObjectMenuCommand};
use crate::window_move::{self, MoveAnchor};

pub const SYSTEM_WINDOW_CLIENT_WIDTH: f32 = 226.0;
/// `FUN_0045aac0` creates type 9 at 226 by 304 (`0xe2` by `0x130`).
pub const SYSTEM_WINDOW_WIDTH: f32 = 226.0;
pub const SYSTEM_WINDOW_HEIGHT: f32 = 304.0;
/// The title strip's right end: the window's width less its 2-pixel right
/// border (A0 system-windows/07-icon-dbl).
const TITLE_STRIP_RIGHT: f32 = SYSTEM_WINDOW_WIDTH - 2.0;
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
const TITLE_WIDTH: f32 = 179.0;
/// The tab strip (`FUN_0060d590`) at (0, 20); each button is 36 by 33.
const TAB_STRIP_Y: f32 = 20.0;
const TAB_SIZE: (f32, f32) = (36.0, 33.0);
/// The page label `+0x5f`: font 4, white, format 1 (`DT_CENTER`), the
/// window's width wide at (0, 58).
const PAGE_LABEL_Y: f32 = 58.0;
/// Each yard count label `+0x19c`: format 1, 46 wide from x 6.
const COUNT_WIDTH: f32 = 46.0;
/// The count label keeps the default font (`FUN_00601880` ->
/// `FUN_00603850`, the game root's `+0xb0`): font 4's 16-row regular cell
/// ([`crate::DEFAULT_FONT`]).
const COUNT_FONT: u16 = crate::DEFAULT_FONT;
/// A band's progress bar, `0x200f0f0` (`FUN_004acec0`).
const PROGRESS_COLOR: egui::Color32 = crate::theme::PALETTE_YELLOW;
/// A facility picture sits 1 by 2 into its 69 by 40 cell.
const PICTURE_OFFSET: (f32, f32) = (1.0, 2.0);
/// An empty deposit is its 67 by 35 bitmap with no 69 by 40 canvas
/// (`FUN_004568a0`: `FUN_005fbd20(.., 0x232d, 10)` straight into the
/// item), at the cell's corner: A0 manufacturing/05-tab (Coruscant, window
/// at (203, 91)) and personnel-states/34 (Mon Calamari, at (314, 86)) put
/// every deposit there pixel for pixel, 1 by 2 above and left of a mine.
/// port: how the list places an item smaller than its cell is not traced;
/// the offset is A0's.
const DEPOSIT_OFFSET: (f32, f32) = (0.0, 0.0);

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

    /// The tab's tooltip (`FUN_00455060`, TEXTSTRA): 0x1835
    /// "Manufacturing", 0x1828 "Shipyards", 0x1830 "Training Facilities",
    /// 0x1832 "Construction Yards", 0x1827 "Refineries", 0x1826 "Mines"
    /// (A0 tooltip/124..129). The title-bar buttons have none.
    #[must_use]
    pub const fn tooltip_string(self) -> u16 {
        match self {
            Self::Overview => 0x1835,
            Self::Shipyards => 0x1828,
            Self::TrainingFacilities => 0x1830,
            Self::ConstructionYards => 0x1832,
            Self::Refineries => 0x1827,
            Self::Mines => 0x1826,
        }
    }

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
const RAIL_NAME_COLOR: egui::Color32 = crate::theme::PALETTE_YELLOW;

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
    /// Where a band drag began, at the canvas (`+0x24c`, `+0x250`, the
    /// first move with the button held, `FUN_004534f0` case `0x200`).
    band_drag_from: Option<(i16, i16)>,
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
    /// `FUN_004aa4a0`. Minimized, the window is hidden, not destroyed, so
    /// it comes back on its page (A0 personnel-states/38).
    Defenses {
        system: SystemKey,
        logical_position: (i16, i16),
        page: crate::defenses_window::DefensesPage,
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

/// Mutable modeless-window and rail state. The last visible window is focused.
///
/// The rail is twelve fixed slots (`FUN_00428b40`, the slot ids at the
/// cockpit's `+0x2f0`): a minimized window takes the first empty slot, a
/// restored one empties its slot (`FUN_00429020`), and with every slot
/// taken the oldest minimized window, by the age list at `+0x428`, is
/// destroyed and the new one takes its slot. A0 personnel-states/32 and
/// 33: a restored window leaves a gap, and the thirteenth replaces the
/// oldest in place.
#[derive(Debug)]
pub struct SystemWindowState {
    faction: CockpitFaction,
    windows: Vec<OpenSystemWindow>,
    /// The kind's claim on the shared window focus.
    focus: crate::window_focus::FocusStamp,
    rail: [Option<RailEntry>; REFERENCE_RAIL_SLOTS],
    /// The occupied slots, oldest minimized first (`+0x428`, count
    /// `+0x458`).
    rail_age: Vec<usize>,
    /// A window was created, not raised, since [`Self::take_created`]
    /// (`FUN_0045aac0` then calls `FUN_0041d2d0`).
    created: bool,
    /// The last press on the rail: its time and screen point, which the
    /// next press pairs with into a double click.
    rail_press: Option<(f64, egui::Pos2)>,
    /// Each open window's band titles as the window last built them.
    band_titles: Vec<BandTitle>,
}

/// What a band's title was built from: its units left and its current
/// product, the product's kind and the name its order gave it; none while
/// it shows nothing building.
type BandKey = Option<(usize, BuildableKind, Option<String>)>;

/// A band's product line as its window built it. `FUN_00457b40` writes it
/// only when the band is built, by `FUN_00458080` at the window's creation
/// and by `FUN_00457c90`'s full rebuild, never when the product is renamed
/// (`FUN_00517760`), so the line keeps what it read then.
///
/// hyp, fitting every A0: the window rebuilds a band when its units left or
/// its current product change; a Confirm of a single unit rebuilds before
/// the new product is numbered, so its line is the bare class name until
/// the next rebuild (A0 2026-11-13-band-title-timing/03, 10..39: "Alliance
/// Escort Carrier" for 23 days; 42: "Alliance Escort Carrier 5" once the
/// window is reopened), while one of several units names the first unit
/// with its number at once (2026-11-11-icon-menu-topic/66: "Carrack Light
/// Cruiser 2 / Building: 4").
#[derive(Debug, Clone, PartialEq)]
struct BandTitle {
    system: SystemKey,
    area: ProductionArea,
    key: BandKey,
    title: String,
}

/// What `system`'s `area` band's title is built from.
fn band_key(
    manufacturing: &ManufacturingState,
    visible: bool,
    system: SystemKey,
    area: ProductionArea,
) -> BandKey {
    let queue = manufacturing.queue(system, area).filter(|_| visible)?;
    let active = queue.active()?;
    Some((queue.len(), active.kind, active.name.clone()))
}

impl Default for SystemWindowState {
    fn default() -> Self {
        Self {
            faction: CockpitFaction::Alliance,
            windows: Vec::new(),
            focus: crate::window_focus::FocusStamp::default(),
            rail: std::array::from_fn(|_| None),
            rail_age: Vec::new(),
            created: false,
            rail_press: None,
            band_titles: Vec::new(),
        }
    }
}

impl SystemWindowState {
    /// When this kind last raised a window, while one is open: its claim
    /// on the shared focus ([`crate::window_focus::shared_focus`]).
    pub fn focus_stamp(&self) -> Option<u64> {
        (!self.windows.is_empty()).then(|| self.focus.stamp())
    }

    /// Whether this kind holds the shared focus, so its top window draws
    /// the focused title strip.
    pub fn set_holds_focus(&mut self, holds: bool) {
        self.focus.set_holds(holds);
    }

    /// Whether a window was created since the last call, which the
    /// agent's advice hears of (`FUN_0041d2d0`).
    pub fn take_created(&mut self) -> bool {
        std::mem::take(&mut self.created)
    }

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
        self.created = true;
        self.forget_band_titles(system);
        self.windows.push(OpenSystemWindow {
            system,
            logical_position,
            tab: SystemWindowTab::Overview,
            selected_item: None,
            selected_band: None,
            band_drag_from: None,
            moving: None,
        });
        self.focus.take();
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

    /// No band of `system`'s window stays selected ([`band_menu_deselects`]).
    pub fn deselect_band(&mut self, system: SystemKey) {
        if let Some(window) = self.window_mut(system) {
            window.selected_band = None;
        }
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
        let item = crate::manufacturing_window::shown_page_cells(
            world,
            fog,
            player,
            manufacturing,
            deliveries,
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
            Some(
                FacilityItem::Pending { .. } | FacilityItem::Deposit(_) | FacilityItem::Viewed { .. },
            ) => return Some(None),
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
        self.rail.iter().flatten().count()
    }

    /// Each occupied rail slot, in slot order: its index, its system and its
    /// window, "system", "fleet", "defenses" or "missions".
    pub fn rail_entries(&self) -> impl Iterator<Item = (usize, SystemKey, &'static str)> + '_ {
        self.rail.iter().enumerate().filter_map(|(index, entry)| {
            let entry = entry.as_ref()?;
            let kind = match entry {
                RailEntry::System(_) => "system",
                RailEntry::Fleet { .. } => "fleet",
                RailEntry::Defenses { .. } => "defenses",
                RailEntry::Missions { .. } => "missions",
            };
            Some((index, entry.system(), kind))
        })
    }

    /// The screen rect of the `index`th rail slot while it holds an entry.
    #[must_use]
    pub fn rail_slot_screen_rect(&self, layout: CockpitLayout, index: usize) -> Option<egui::Rect> {
        self.rail
            .get(index)?
            .as_ref()
            .map(|_| cockpit_rect(layout, rail_slot_rect(self.faction, index)))
    }

    pub fn clear(&mut self) {
        self.windows.clear();
        self.rail = std::array::from_fn(|_| None);
        self.rail_age.clear();
        self.band_titles.clear();
    }

    /// `system`'s `area` band as its visible window shows it: [`band`],
    /// its product line kept from when the window last built the band. The
    /// window builds it again when the band's units left or current product
    /// change ([`BandTitle`]).
    pub fn shown_band(
        &mut self,
        world: &GameWorld,
        manufacturing: &ManufacturingState,
        visible: bool,
        system: SystemKey,
        area: ProductionArea,
    ) -> crate::manufacturing_window::Band {
        let mut shown = band(world, manufacturing, visible, system, area);
        let key = band_key(manufacturing, visible, system, area);
        match self
            .band_titles
            .iter_mut()
            .find(|title| title.system == system && title.area == area)
        {
            Some(title) if title.key == key => shown.status.clone_from(&title.title),
            Some(title) => {
                title.key = key;
                title.title.clone_from(&shown.status);
            }
            None => self.band_titles.push(BandTitle {
                system,
                area,
                key,
                title: shown.status.clone(),
            }),
        }
        shown
    }

    /// The product line `system`'s window last built for its `area` band.
    #[must_use]
    pub fn band_status(&self, system: SystemKey, area: ProductionArea) -> Option<&str> {
        self.band_titles
            .iter()
            .find(|title| title.system == system && title.area == area)
            .map(|title| title.title.as_str())
    }

    /// Confirm has replaced `system`'s `area` production with `count`
    /// units (`FUN_00438980`). With the system's window open, a single
    /// unit's band is rebuilt before its product is numbered and keeps the
    /// bare class name (A0 2026-11-13-band-title-timing/03); several units'
    /// band names the first with its number (2026-11-11-icon-menu-topic/66).
    pub fn confirmed_build(
        &mut self,
        world: &GameWorld,
        manufacturing: &ManufacturingState,
        system: SystemKey,
        area: ProductionArea,
        count: u32,
    ) {
        if !self.windows.iter().any(|window| window.system == system) {
            return;
        }
        self.band_titles
            .retain(|title| !(title.system == system && title.area == area));
        let key = band_key(manufacturing, true, system, area);
        if count != 1 {
            return;
        }
        if let Some((_, kind, _)) = &key {
            let title = crate::manufacturing_window::product(world, *kind).0;
            self.band_titles.push(BandTitle {
                system,
                area,
                key,
                title,
            });
        }
    }

    /// A window closed or created anew builds its bands afresh.
    fn forget_band_titles(&mut self, system: SystemKey) {
        self.band_titles.retain(|title| title.system != system);
    }

    /// Empty the slot holding `entry`'s window (`FUN_00429020`): it leaves
    /// the age list and its slot stays free for the next window.
    fn take_rail(&mut self, entry: &RailEntry) -> Option<RailEntry> {
        let index = self
            .rail
            .iter()
            .position(|held| held.as_ref().is_some_and(|held| held.same_window(entry)))?;
        self.rail_age.retain(|&slot| slot != index);
        self.rail[index].take()
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
        self.focus.take();
        true
    }

    fn close(&mut self, system: SystemKey) {
        self.windows.retain(|window| window.system != system);
        self.forget_band_titles(system);
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

    /// Put a minimized Defenses window on the rail (`0x466`), on its page.
    pub fn minimize_defenses_window(
        &mut self,
        system: SystemKey,
        logical_position: (i16, i16),
        page: crate::defenses_window::DefensesPage,
    ) {
        self.push_rail(RailEntry::Defenses {
            system,
            logical_position,
            page,
        });
    }

    /// Take `system`'s minimized Fleet window off the rail and return where
    /// it stood: its sector icon's double click brings it back, as the
    /// Defenses window's does (A0 personnel-states/38). port: the Fleet
    /// icon's restore is not captured in A0; the windows share
    /// `FUN_0045aac0`.
    pub fn take_railed_fleet(&mut self, system: SystemKey) -> Option<(i16, i16)> {
        let entry = *self.rail.iter().flatten().find(
            |entry| matches!(entry, RailEntry::Fleet { system: held, .. } if *held == system),
        )?;
        self.take_rail(&entry);
        match entry {
            RailEntry::Fleet {
                logical_position, ..
            } => Some(logical_position),
            _ => None,
        }
    }

    /// Take `system`'s minimized Defenses window off the rail, with where it
    /// stood and its page: its sector icon's double click brings it back
    /// rather than opening another (A0 personnel-states/38, Mon Calamari's
    /// Trooper Regiments page back and its slot emptied).
    pub fn take_railed_defenses(
        &mut self,
        system: SystemKey,
    ) -> Option<((i16, i16), crate::defenses_window::DefensesPage)> {
        let entry = *self.rail.iter().flatten().find(
            |entry| matches!(entry, RailEntry::Defenses { system: held, .. } if *held == system),
        )?;
        self.take_rail(&entry);
        match entry {
            RailEntry::Defenses {
                logical_position,
                page,
                ..
            } => Some((logical_position, page)),
            _ => None,
        }
    }

    /// Take `system`'s minimized Missions window off the rail and return
    /// where it stood, as its sector icon brings back the Defenses window
    /// (A0 personnel-states/38). port: the Missions icon's restore is not
    /// captured in A0; the windows share `FUN_0045aac0`.
    pub fn take_railed_missions(&mut self, system: SystemKey) -> Option<(i16, i16)> {
        let entry = *self.rail.iter().flatten().find(
            |entry| matches!(entry, RailEntry::Missions { system: held, .. } if *held == system),
        )?;
        self.take_rail(&entry);
        match entry {
            RailEntry::Missions {
                logical_position, ..
            } => Some(logical_position),
            _ => None,
        }
    }

    /// Put a minimized Missions window on the rail (`0x466`).
    pub fn minimize_missions_window(&mut self, system: SystemKey, logical_position: (i16, i16)) {
        self.push_rail(RailEntry::Missions {
            system,
            logical_position,
        });
    }

    fn push_rail(&mut self, entry: RailEntry) {
        // FUN_00428b40: a slot already holding the window keeps it; else the
        // first empty slot; else the oldest's slot, its window destroyed.
        if let Some(held) = self
            .rail
            .iter_mut()
            .flatten()
            .find(|held| held.same_window(&entry))
        {
            *held = entry;
            return;
        }
        let index = match self.rail.iter().position(Option::is_none) {
            Some(index) => index,
            None => self.rail_age.remove(0),
        };
        self.rail[index] = Some(entry);
        self.rail_age.push(index);
    }

    fn restore(&mut self, system: SystemKey) -> bool {
        let Some(entry) = self
            .rail
            .iter()
            .flatten()
            .find(|entry| entry.is_system_window() && entry.system() == system)
            .cloned()
        else {
            return false;
        };
        if let Some(RailEntry::System(window)) = self.take_rail(&entry) {
            self.windows.push(window);
            self.focus.take();
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

/// Whether a band's menu, closing with `command` chosen (or none) and
/// Escape pressed or not, leaves no band selected. A0 build-selection2:
/// Escape clears the selection (`18`), and so do Build and Stop, whose
/// windows open over it (`05`, `07`, `11`, `15`); a click beside the menu
/// (`23`) and Destination (`21`) keep it. hyp: the windows' activation
/// clears it; the rest of the menu's rows are not captured and keep it.
#[must_use]
pub fn band_menu_deselects(command: Option<ObjectMenuCommand>, escaped: bool) -> bool {
    match command {
        None => escaped,
        Some(command) => matches!(command, ObjectMenuCommand::Build | ObjectMenuCommand::Stop),
    }
}

/// The window's selection (slot 22, `FUN_00453ee0`): on the overview (page
/// `0x67`) the selected band's manager, on a facility page the list's
/// selected item.
fn surface_menu_object(window: &OpenSystemWindow, system: SystemKey) -> Option<MenuObject> {
    match window.tab.page() {
        None => window
            .selected_band
            .map(|area| MenuObject::Producer { system, area }),
        Some(_) => window
            .selected_item
            .and_then(|item| item.menu_object(system)),
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
        page: crate::defenses_window::DefensesPage,
    },
    /// A click on a minimized Missions window's rail slot restores it.
    RestoreMissionsWindow {
        system: SystemKey,
        logical_position: (i16, i16),
    },
    /// A band dragged and released at a 640 by 480 canvas `point`
    /// (`FUN_004534f0` case `0x202`: past 4 pixels from where the drag
    /// began it posts `0x29a` to the main window, whose `FUN_00422ce0`
    /// gives window type 9 a Destination `0x214` against the `+0x70`
    /// object under the point, for the selected bands' managers).
    DropBand {
        system: SystemKey,
        area: ProductionArea,
        point: (i16, i16),
    },
}

/// The drag image of a band drag (`FUN_004534f0` case `0x200`:
/// `FUN_0060dbb0` with the window's `+0x254 + 4 * (n - 1)` for `n` selected
/// bands, hotspot (3, 3) by `FUN_0060dcb0`). `FUN_00456230` loads STRATEGY
/// 0x2841..0x2843 for the Alliance (`+0x9c` 1) and 0x2844..0x2846 for the
/// Empire. port: one band is selected at a time, so the first.
#[must_use]
pub const fn band_drag_image(faction: CockpitFaction) -> u32 {
    match faction {
        CockpitFaction::Alliance => 0x2841,
        CockpitFaction::Empire => 0x2844,
    }
}

/// The drag image's hotspot (`FUN_0060dcb0(3, 3)`).
pub const BAND_DRAG_HOTSPOT: (f32, f32) = (3.0, 3.0);

/// Whether a band drag released `to` from `from` drops: `FUN_004534f0`
/// case `0x202` ignores a release under 4 pixels from the start in both x
/// and y.
#[must_use]
pub fn band_drag_drops(from: (i16, i16), to: (i16, i16)) -> bool {
    (to.0 - from.0).abs() >= 4 || (to.1 - from.1).abs() >= 4
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
    /// A band drag released: the band and the canvas point.
    band_drop: Option<(ProductionArea, (i16, i16))>,
    /// A band drag began (`Some(point)`) or ended (`Some(None)`).
    band_drag: Option<Option<(i16, i16)>>,
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
    let focused_system = windows
        .last()
        .filter(|_| state.focus.holds())
        .map(|window| window.system);
    for window in windows {
        let system = window.system;
        let visible = contents_visible(world, fog, cockpit_faction(faction), system);
        let bands =
            BAND_AREAS.map(|area| state.shown_band(world, manufacturing, visible, system, area));
        let mut result = draw_system_window(
            ctx,
            sources,
            window,
            &bands,
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
        if let (Some(drag), Some(open)) = (result.band_drag, state.window_mut(system)) {
            open.band_drag_from = drag;
        }
        if let Some((area, point)) = result.band_drop {
            actions.push(SystemWindowAction::DropBand {
                system,
                area,
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
    let entries = state.rail;
    let mut restored = None;
    let mut slots = Vec::new();
    for (index, entry) in entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| Some((index, entry.as_ref()?)))
    {
        let Some(system) = world.systems.get(entry.system()) else {
            continue;
        };
        let logical = rail_slot_rect(faction, index);
        let screen_rect = cockpit_rect(layout, logical);
        egui::Area::new(egui::Id::new(("original-reference-rail", index)))
            .fixed_pos(screen_rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let (slot_rect, _) =
                    ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
                // FUN_00428d70: the window's icon (slot `+0x74`) at the
                // slot's corner, then its system's name after the icon in
                // font 10, yellow (`0x200f0f0`), clipped to the slot.
                let icon = match entry {
                    RailEntry::System(window) => rail_icon(world, window.system),
                    RailEntry::Defenses { system, .. } => {
                        crate::defenses_window::rail_icon(world, cockpit_faction(faction), *system)
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
                crate::theme::paint_gdi_text(
                    &painter,
                    logical_rect(
                        slot_rect,
                        layout.scale,
                        icon_width,
                        0.0,
                        slot_rect.width() / layout.scale - icon_width,
                        slot_rect.height() / layout.scale,
                    ),
                    layout.scale,
                    10,
                    &system.name,
                    RAIL_NAME_COLOR,
                    (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
                );
                slots.push((slot_rect, ui.layer_id(), *entry));
            });
    }
    // FUN_00422ce0: WM_LBUTTONDBLCLK (0x203) in one of the twelve slot
    // rectangles (+0xb8) restores its window (FUN_00429020); a single click
    // does nothing (A0 system-windows/10-restored, 12-rail-dbl). Windows
    // sends it for a second press on the rail within the double-click time
    // and rectangle of the first, so each press of the frame pairs in turn.
    let (time, presses) = ctx.input(|input| {
        let presses: Vec<egui::Pos2> = input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    ..
                } => Some(*pos),
                _ => None,
            })
            .collect();
        (input.time, presses)
    });
    for point in presses {
        let slot = slots.iter().find(|(rect, layer, _)| {
            rect_contains(*rect, point) && ctx.layer_id_at(point) == Some(*layer)
        });
        let Some((_, _, entry)) = slot else {
            state.rail_press = None;
            continue;
        };
        let paired = state
            .rail_press
            .is_some_and(|(at, first)| is_double_click(time - at, (point - first) / layout.scale));
        if paired {
            restored = Some(*entry);
            state.rail_press = None;
        } else {
            state.rail_press = Some((time, point));
        }
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
            state.take_rail(&entry);
            Some(SystemWindowAction::RestoreFleetWindow {
                system,
                logical_position,
            })
        }
        entry @ RailEntry::Defenses {
            system,
            logical_position,
            page,
        } => {
            state.take_rail(&entry);
            Some(SystemWindowAction::RestoreDefensesWindow {
                system,
                logical_position,
                page,
            })
        }
        entry @ RailEntry::Missions {
            system,
            logical_position,
        } => {
            state.take_rail(&entry);
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
    bands: &[crate::manufacturing_window::Band; 3],
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
    // A frozen copy keeps the side the system had when seen (FUN_004f9a60).
    let side = match crate::viewed_contents::system_sight(world, sources.fog, player, window.system) {
        crate::viewed_contents::SystemSight::Frozen(view) => control_side(view.control),
        _ => control_side(system.control),
    };
    let pages = SystemWindowTab::ALL.map(|tab| {
        tab.page().map(|page| {
            crate::manufacturing_window::shown_page_cells(
                world,
                sources.fog,
                player,
                sources.manufacturing,
                sources.deliveries,
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
            // The title strip ends inside the window's 2-pixel right
            // border: A0 system-windows/07-icon-dbl shows Yavin's red strip
            // to x 223 and the border's navy (0, 0, 128) in columns 224 and
            // 225 of its rows, as the 235-wide windows' strip (231 at x 2)
            // ends at their border.
            paint_native(
                &painter.with_clip_rect(logical_rect(
                    local,
                    scale,
                    0.0,
                    0.0,
                    TITLE_STRIP_RIGHT,
                    SYSTEM_WINDOW_HEIGHT,
                )),
                ctx,
                cache,
                DllSource::Strategy,
                crate::defenses_window::title_resource(side, focused),
                local,
                scale,
                2.0,
                2.0,
            );
            crate::theme::paint_gdi_text(
                &painter,
                logical_rect(local, scale, TITLE.0, TITLE.1, TITLE_WIDTH, TITLE.2),
                scale,
                5,
                &system.name,
                egui::Color32::BLACK,
                (
                    crate::theme::GdiAlign::Left,
                    crate::theme::GdiVAlign::VCenter,
                ),
            );

            // A title button's or tab's key pixel (its first DIB pixel, the
            // bottom-left corner; each bitmap holds just that one) shows the
            // window's own background there, not the title strip: A0
            // system-windows/07-icon-dbl shows 10297's (9, 11, 32) at the
            // three buttons' corners (3, 16), (195, 16), (209, 16) and its
            // white at each tab's (x, 52). The buttons and tabs are child
            // controls blitted keyed (FUN_00602d30 -> FUN_005fd0f0) over
            // their parent, the strip being a sibling.
            let key_corner = |cache: &mut BmpCache, x: f32, bottom: f32| {
                paint_native(
                    &painter.with_clip_rect(logical_rect(local, scale, x, bottom, 1.0, 1.0)),
                    ctx,
                    cache,
                    DllSource::Strategy,
                    WINDOW_BACKGROUND,
                    local,
                    scale,
                    0.0,
                    0.0,
                );
            };
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
                key_corner(cache, x, 3.0 + 13.0);
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
                crate::win_tooltip::response_tool(&response, rect, tab.tooltip_string());
                let pressed = tab == window.tab
                    || (primary_down && pointer.is_some_and(|point| rect_contains(rect, point)));
                paint(
                    cache,
                    DllSource::Strategy,
                    tab_resource(tab, side, pressed, empty),
                    tab.x(),
                    TAB_STRIP_Y,
                );
                key_corner(cache, tab.x(), TAB_STRIP_Y + TAB_SIZE.1 - 1.0);
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
                        crate::theme::paint_gdi_text(
                            &painter,
                            logical_rect(local, scale, x, y, COUNT_WIDTH, 16.0),
                            scale,
                            COUNT_FONT,
                            &yard_count(cells),
                            egui::Color32::WHITE,
                            (crate::theme::GdiAlign::Center, crate::theme::GdiVAlign::Top),
                        );

                        // FUN_00458080: the band's frame, its strip and its
                        // lines.
                        // The band as the window last built it
                        // (SystemWindowState::shown_band).
                        let shown = &bands[index];
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
                            crate::theme::paint_gdi_text(
                                &painter,
                                logical_rect(local, scale, bx + x, by + y, 160.0, 14.0),
                                scale,
                                10,
                                text,
                                colour,
                                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
                            );
                        };
                        let white = egui::Color32::WHITE;
                        line(
                            crate::manufacturing_window::band_title(area),
                            BAND_TITLE,
                            egui::Color32::BLACK,
                        );
                        line(&shown.status, BAND_STATUS, white);
                        if let Some(portrait) = shown.portrait {
                            paint(
                                cache,
                                DllSource::Gokres,
                                portrait,
                                bx + BAND_PORTRAIT.0,
                                by + BAND_PORTRAIT.1,
                            );
                        }
                        if let Some(units) = &shown.units {
                            line(units, BAND_UNITS, white);
                        }
                        line(&shown.destination, BAND_DESTINATION, white);

                        // FUN_004acec0: the progress bar, 0x200f0f0 over
                        // black: the palette's yellow, as A0 draws it
                        // (personnel-states/24).
                        let (px, py, width, height) = PROGRESS_RECTS[index];
                        let bar = logical_rect(local, scale, px, py, width, height);
                        painter.rect_filled(bar, 0.0, egui::Color32::BLACK);
                        if let Some(progress) = shown.progress {
                            #[expect(
                                clippy::cast_possible_truncation,
                                clippy::cast_sign_loss,
                                clippy::cast_precision_loss,
                                reason = "The bar is 160 pixels wide."
                            )]
                            let filled = crate::manufacturing_window::progress_pixels(
                                width as u32,
                                progress,
                            )
                            .min(width as u32) as f32;
                            let mut done = bar;
                            done.set_width(filled * scale);
                            painter.rect_filled(done, 0.0, PROGRESS_COLOR);
                        }

                        // A press selects the band; a right release opens
                        // its manager's menu (FUN_00453ee0, FUN_004ac5c0).
                        let rect = band_rect(local, scale, area);
                        let response = ui.interact(
                            rect,
                            ui.id().with((window.system, "band", index)),
                            egui::Sense::click_and_drag(),
                        );
                        if any_pressed && response.is_pointer_button_down_on() {
                            result.band = Some(area);
                            result.focus = true;
                        }
                        // FUN_004534f0 case 0x200: a move with the button
                        // held inside the bands' rect starts the drag; its
                        // image follows the pointer over every window, and
                        // the release (case 0x202) drops past 4 pixels.
                        if response.drag_started_by(egui::PointerButton::Primary) {
                            let from = ui
                                .input(|input| input.pointer.interact_pos())
                                .map(|point| canvas_point(layout, point));
                            result.band_drag = Some(from);
                        }
                        if response.dragged_by(egui::PointerButton::Primary) {
                            if let Some(point) = ui.input(|input| input.pointer.latest_pos()) {
                                let painter = ctx.layer_painter(crate::fleet_window::drag_layer());
                                let at = point
                                    - egui::vec2(BAND_DRAG_HOTSPOT.0, BAND_DRAG_HOTSPOT.1) * scale;
                                paint_native(
                                    &painter,
                                    ctx,
                                    cache,
                                    DllSource::Strategy,
                                    band_drag_image(faction),
                                    egui::Rect::from_min_size(at, egui::Vec2::ZERO),
                                    scale,
                                    0.0,
                                    0.0,
                                );
                            }
                        }
                        if response.drag_stopped_by(egui::PointerButton::Primary) {
                            let to = ui.input(|input| input.pointer.latest_pos());
                            if let (Some(from), Some(to)) = (window.band_drag_from, to) {
                                let to = canvas_point(layout, to);
                                if band_drag_drops(from, to) {
                                    result.band_drop = Some((area, to));
                                }
                            }
                            result.band_drag = Some(None);
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
                    crate::theme::paint_gdi_text(
                        &painter,
                        logical_rect(local, scale, 0.0, PAGE_LABEL_Y, SYSTEM_WINDOW_WIDTH, 16.0),
                        scale,
                        4,
                        window.tab.name(),
                        egui::Color32::WHITE,
                        (crate::theme::GdiAlign::Center, crate::theme::GdiVAlign::Top),
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
                    // FUN_004ac5c0, on the right release, opens the pop-up
                    // for slot 22's selection (FUN_00453ee0: the list's
                    // selected objects): over empty space, for nothing.
                    if list_response.secondary_clicked() {
                        if let Some(point) = list_response
                            .interact_pointer_pos()
                            .filter(|point| rect_contains(list, *point))
                        {
                            result.object_menu = Some((None, canvas_point(layout, point)));
                        }
                    }
                    for (index, cell) in cells.iter().enumerate() {
                        let rect = cell_rect(local, scale, index);
                        if !rect.intersects(list) {
                            break;
                        }
                        let (dx, dy) = if matches!(cell.item, FacilityItem::Deposit(_)) {
                            DEPOSIT_OFFSET
                        } else {
                            PICTURE_OFFSET
                        };
                        paint_native(
                            &list_painter,
                            ctx,
                            cache,
                            DllSource::Strategy,
                            cell.picture,
                            rect,
                            scale,
                            dx,
                            dy,
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
                        // (FUN_006083c0); the right release opens its menu.
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
                        if response.secondary_clicked() {
                            if let Some(point) = response
                                .interact_pointer_pos()
                                .filter(|point| rect_contains(rect.intersect(list), *point))
                            {
                                result.object_menu = Some((
                                    cell.item.menu_object(window.system),
                                    canvas_point(layout, point),
                                ));
                            }
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
            // A right release on the window's own surface, off its child
            // controls, also opens the pop-up: FUN_004534f0 passes every
            // 0x205 to the base procedure, which calls slot 7
            // (FUN_004ac5c0) for slot 22's selection (FUN_00453ee0). A
            // press off a band selects nothing and clears nothing, so the
            // menu is the selected band's, or the one for nothing
            // (Encyclopedia and Status, grey; A0 menus3/34).
            // The controls take their own clicks; the window's response
            // only sees the rest.
            if window_response.secondary_clicked() {
                if let Some(point) = window_response
                    .interact_pointer_pos()
                    .filter(|point| !controls.iter().any(|rect| rect_contains(*rect, *point)))
                {
                    result.object_menu = Some((
                        surface_menu_object(&window, window.system),
                        canvas_point(layout, point),
                    ));
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

/// Whether the player sees the other side's live objects at `system`
/// (`viewed_contents::system_sight`): it holds the system, its copy of the
/// system is live (`FUN_004f9a60`), the port's fog shows a system it holds
/// no copy of, or the debug "see everything" mode is on. A frozen copy is
/// not live: the windows that read copies show it instead.
pub(crate) fn opposing_contents_visible(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> bool {
    matches!(
        crate::viewed_contents::system_sight(world, fog, player, system),
        crate::viewed_contents::SystemSight::Live
    )
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

/// Whether a second press pairs with the first into a double click: within
/// the default double-click time (`GetDoubleClickTime`, 500 ms) and inside
/// the 4 by 4 rectangle around the first (`SM_CXDOUBLECLK`,
/// `SM_CYDOUBLECLK`), in original pixels.
pub(crate) fn is_double_click(seconds: f64, offset: egui::Vec2) -> bool {
    seconds <= 0.5 && offset.x.abs() <= 2.0 && offset.y.abs() <= 2.0
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
#[path = "system_window_tests.rs"]
mod tests;
