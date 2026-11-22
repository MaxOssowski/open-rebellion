//! The original System Defenses window (window type 10, `FUN_004a7790`): a
//! system's batteries, shields, squadrons, regiments and personnel under five
//! tabs (manual Fig. 3.73, p. 125). Recovery notes:
//! `ghidra/notes/sector-quadrants.md`, "Type 10".
//!
//! It opens from the icon a sector window shows at the bottom left of a
//! planet (`FUN_0045ce80`, `FUN_0045aac0` kind 8). Every page lists only the
//! side that holds the system (`+0x148`), and a move released over the
//! window targets its system (`+0x70`, `FUN_004aa470`). A right release
//! opens the object pop-up menu for the list's selection: the class keeps
//! the base dialog's slots 7 and 8 (`FUN_004ac5c0`, `FUN_004ac730`), and
//! slot 22 (`FUN_004a7a20`) gives the list's selected items.

use egui_macroquad::egui;
use rebellion_core::dat::Faction;
use rebellion_core::delivery::DeliveryState;
use rebellion_core::economy::EconomyState;
use rebellion_core::fog::FogState;
use rebellion_core::ids::{
    CharacterKey, DatId, DefenseFacilityKey, FighterKey, SpecialForceKey, SystemKey, TroopKey,
};
use rebellion_core::manufacturing::{BuildableKind, ManufacturingState};
use rebellion_core::missions::{
    MemberTransit, MissionMember, MissionState, MissionTarget, MissionTargetPick,
};
use rebellion_core::squadrons::{SquadronPlace, SquadronTransit};
use rebellion_core::troop_transport::{RegimentLeg, RegimentTransit};
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::drag_list::{ListSelection, Modifiers};
use crate::fleet_window::{
    control_side, faction_side, fleet_side, paint_key_pixel, paint_native, KeyPixel,
};
use crate::object_menu::MenuObject;
use crate::minis::fighter_mini_id;
use crate::quadrant_icons::{mission_keys, system_members};
use crate::system_window::{
    canvas_point, character_picture, clamp_window_to_galaxy, defense_facility_mini, exact_clicked,
    logical_rect, opposing_contents_visible, rect_contains, special_force_mini, troop_mini,
};
use crate::window_move::{self, MoveAnchor};

pub const DEFENSES_WINDOW_WIDTH: f32 = 235.0;
pub const DEFENSES_WINDOW_HEIGHT: f32 = 304.0;

const BACKGROUND: u32 = 10577;
const CLOSE_NORMAL: u32 = 10108;
const CLOSE_PRESSED: u32 = 10109;
const MINIMIZE_NORMAL: u32 = 10253;
const MINIMIZE_PRESSED: u32 = 10254;
const SECTOR_NORMAL: u32 = 10209;
const SECTOR_PRESSED: u32 = 10208;
/// The row frame `+0x5d` keyed over a selected row's mini (`FUN_004a9ce0`):
/// 10578 for side 1, 10579 otherwise.
const ROW_FRAME: [u32; 2] = [10578, 10579];
/// A minimized Defenses window's rail icon (`FUN_004aa4a0`).
const RAIL_ICONS: [u32; 3] = [11533, 11534, 11535];

/// The tab strip (`FUN_0060d590`) at (0, 20); each button is 36 by 33.
const TAB_STRIP: (f32, f32) = (0.0, 20.0);
const TAB_SIZE: (f32, f32) = (36.0, 33.0);
/// The list (`FUN_00607ea0`, id `0xcb`) and its 70 by 70 grid cells.
const LIST: (f32, f32, f32, f32) = (7.0, 81.0, 222.0, 210.0);
const CELL: f32 = 70.0;
/// `FUN_00609ae0` wraps a row before a cell would pass the list's width
/// less a shown bar: 222 - 10 leaves three columns either way.
const COLUMNS: usize = 3;
/// The list's scroll bar (`FUN_004a8790`: `FUN_0060a490(.., 0x29cc, ..)`):
/// STRATEGY RCDATA 10700, `[10, 10701, 10702, 10703, 10704, 10705, 10706,
/// 10707, 10708, 10684]`. A0 2026-11-11-icon-menu-topic/05 and 06 show it
/// at the list's right edge, window x 219..228 from y 81, for Coruscant's
/// ten TIE Fighter squadrons.
const LIST_BAR_ART: crate::list_scroll_bar::BarArt = crate::list_scroll_bar::BarArt {
    width: 10,
    track: 10_701,
    up: [10_702, 10_703],
    down: [10_704, 10_705],
    top: 10_706,
    middle: 10_707,
    bottom: 10_708,
};
/// The list's arrow step: `FUN_004a8790` passes 10 as `FUN_00607ea0`'s
/// step (`+0xc0`).
const LIST_STEP: f32 = 10.0;
/// The text rect's offset in a cell (`+0xe8`, `+0xec`) and its right trim.
const TEXT_OFFSET: (f32, f32) = (1.0, 27.0);
const TEXT_RIGHT_TRIM: f32 = 3.0;
/// The title label `+0x49` (`FUN_004a8790`): from the sector button's width
/// plus 5, 16 high.
const TITLE: (f32, f32, f32) = (19.0, 2.0, 16.0);
/// The tab name `+0x5b` and the garrison line `+0x5c` (`FUN_004a8790`): x, y,
/// width and height.
const TAB_NAME: (f32, f32, f32, f32) = (2.0, 51.0, 231.0, 16.0);
const GARRISON: (f32, f32, f32, f32) = (2.0, 63.0, 228.0, 17.0);

/// The five pages (`FUN_004a90d0`), by tab id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefensesPage {
    /// Id 1: personnel not on a visible mission.
    Personnel,
    /// Id 2: regiments (`0x10..0x14`).
    Regiments,
    /// Id 3: fighter squadrons (`0x1c..0x20`).
    Squadrons,
    /// Id 4: GenCore shields (`0x24`) and the Death Star Shield (`0x25`).
    Shields,
    /// Id 5: KDY-150 (`0x22`) and LNR batteries (`0x23`).
    Batteries,
}

impl DefensesPage {
    /// The tab's tooltip (`FUN_004a8790`, TEXTSTRA): 0x1744 "Personnel",
    /// 0x1743 "Trooper Regiments", 0x1742 "Fighter Squadrons", 0x1741
    /// "Planetary Shields", 0x1740 "Planetary Batteries".
    #[must_use]
    pub const fn tooltip_string(self) -> u16 {
        match self {
            Self::Personnel => 0x1744,
            Self::Regiments => 0x1743,
            Self::Squadrons => 0x1742,
            Self::Shields => 0x1741,
            Self::Batteries => 0x1740,
        }
    }

    pub const ALL: [Self; 5] = [
        Self::Personnel,
        Self::Regiments,
        Self::Squadrons,
        Self::Shields,
        Self::Batteries,
    ];

    /// The button's x in the strip (`FUN_004a8790`).
    const fn x(self) -> f32 {
        match self {
            Self::Personnel => 28.0,
            Self::Regiments => 64.0,
            Self::Squadrons => 100.0,
            Self::Shields => 136.0,
            Self::Batteries => 172.0,
        }
    }

    /// The help message `0x1740 + (5 - id)`: TEXTSTRA 5952..5956.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Batteries => "Planetary Batteries",
            Self::Shields => "Planetary Shields",
            Self::Squadrons => "Fighter Squadrons",
            Self::Regiments => "Trooper Regiments",
            Self::Personnel => "Personnel",
        }
    }
}

/// The button's bitmap (`FUN_004a8790`, `FUN_004a9ce0`): the normal id when
/// the page lists something, the empty id two higher otherwise, and the
/// middle id while selected. Squadrons, regiments and personnel have art per
/// side; any side but 1 and 2 shows one gray bitmap. Batteries and shields
/// share their art across sides, and `FUN_0060d700` copies the pressed
/// bitmap into the selected state they leave unset.
#[must_use]
pub const fn tab_resource(page: DefensesPage, side: u8, selected: bool, empty: bool) -> u32 {
    let base = match (page, side) {
        (DefensesPage::Batteries, _) => 10550,
        (DefensesPage::Shields, _) => 10553,
        (DefensesPage::Squadrons, 1) => 10556,
        (DefensesPage::Squadrons, 2) => 10559,
        (DefensesPage::Squadrons, _) => return 10562,
        (DefensesPage::Regiments, 1) => 10563,
        (DefensesPage::Regiments, 2) => 10566,
        (DefensesPage::Regiments, _) => return 10569,
        (DefensesPage::Personnel, 1) => 10570,
        (DefensesPage::Personnel, 2) => 10573,
        (DefensesPage::Personnel, _) => return 10576,
    };
    if selected {
        base + 1
    } else if empty {
        base + 2
    } else {
        base
    }
}

/// An object a page lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefensesItem {
    Character(CharacterKey),
    SpecialForce(SpecialForceKey),
    Troop(TroopKey),
    /// The viewing side's frozen copy of the other side's regiment `id`
    /// (`FUN_004f9a60`; `viewed_contents`). `alliance` is the viewing side.
    ViewedTroop { alliance: bool, id: DatId },
    /// The `index`th row of a page drawn from the player's frozen copy of
    /// `system` (`SystemView`, `SystemExtras::travellers`): a special
    /// force, squadron or facility, whose menu offers its Encyclopedia and
    /// Status.
    Viewed {
        system: SystemKey,
        index: usize,
        unit: crate::viewed_contents::ViewedUnit,
    },
    /// The `nth` squadron of `class` on `system`'s planet. port: squadrons
    /// are counts per class.
    Squadron {
        system: SystemKey,
        class: FighterKey,
        nth: u32,
    },
    Defense(DefenseFacilityKey),
    /// The `nth` squadron of `class` travelling to `system`'s planet by
    /// Move, arriving on day `arrival`: `FUN_00556390` puts it in its
    /// destination at once, in hyperspace (`+0x50` bit 4). port: the port
    /// holds it in `SquadronTransits` until it lands.
    TravellingSquadron {
        system: SystemKey,
        class: FighterKey,
        arrival: u64,
        nth: u32,
    },
    /// The `index`th unit queued or travelling for `system`: a squadron or
    /// a defense facility not yet completed (`+0x50` bit 2 clear), or a
    /// facility en route until `arrival` (bit 4).
    Pending {
        system: SystemKey,
        index: usize,
        kind: BuildableKind,
        arrival: Option<u64>,
        /// A regiment's or squadron's product
        /// (`rebellion_core::products`); none for a facility.
        product: Option<u64>,
    },
}

impl DefensesItem {
    /// The object its pop-up menu opens for: a character (`FUN_004ed350`),
    /// a special force (`FUN_00503b50`), a regiment (`FUN_00504b30`), a
    /// squadron (`FUN_00502fe0`) or a defense facility, whose menu holds
    /// Encyclopedia and Status (`FUN_0051d990`).
    const fn menu_object(self) -> Option<MenuObject> {
        match self {
            Self::Character(key) => Some(MenuObject::Character(key)),
            Self::SpecialForce(key) => Some(MenuObject::SpecialForce(key)),
            Self::Troop(key) => Some(MenuObject::Troop(key)),
            Self::ViewedTroop { alliance, id } => Some(MenuObject::ViewedTroop { alliance, id }),
            Self::Viewed { system, unit, .. } => Some(MenuObject::ViewedUnit { system, unit }),
            Self::Squadron { system, class, .. } => {
                Some(MenuObject::PlanetFighter { system, class })
            }
            Self::Defense(key) => Some(MenuObject::DefenseFacility(key)),
            Self::TravellingSquadron {
                system,
                class,
                arrival,
                ..
            } => Some(MenuObject::TravellingFighter {
                to: SquadronPlace::Planet(system),
                class,
                arrival,
            }),
            Self::Pending {
                system,
                kind,
                arrival,
                product,
                ..
            } => match kind {
                BuildableKind::Fighter(class) => Some(MenuObject::PendingFighter {
                    system,
                    class,
                    product,
                }),
                BuildableKind::Troop(class) => Some(MenuObject::PendingTroop {
                    system,
                    class,
                    product,
                }),
                BuildableKind::DefenseFacility(build) => Some(MenuObject::PendingFacility {
                    system,
                    class: build.class,
                    owner: fleet_side(build.is_alliance),
                    arrival,
                }),
                _ => None,
            },
        }
    }
}

/// The units the production queues and deliveries hold for the window's
/// systems. port: the original creates them at the order, as objects not
/// yet completed.
#[derive(Clone, Copy)]
pub struct DefensesProduction<'a> {
    pub manufacturing: &'a ManufacturingState,
    pub deliveries: &'a DeliveryState,
    /// The regiments travelling by Move (`TroopTransportState::transits`):
    /// those bound for a planet's surface list there en route.
    pub regiments: &'a [RegimentTransit],
    /// The squadrons travelling by Move (`SquadronTransits::transits`):
    /// those bound for a planet list there en route.
    pub squadrons: &'a [SquadronTransit],
}

/// What `FUN_0042c3b0(.., 0, 1)` draws under a row's mini: a personnel's
/// or regiment's backdrop, and a character's damage mark while it is
/// injured (`FUN_004f7800` sets `+0x50` bit `0x200` to injury != 0). An
/// unfinished unit (bit 2 clear) takes STRATEGY `0x2d32` (11570) for side 1,
/// else `0x2d34` (11572), on top; a defense facility en route (bit 4) takes
/// 11509 (`0x2cf5`, families `0x22..0x27`). A regiment travelling to the
/// planet (bit 4) takes its en route mark over its backdrop (A0
/// 2026-11-11-regiment-en-route/00). A character or special force on its
/// way to the planet (its Move's transit) takes its en route mark too: A0
/// specforces/03 draws STRATEGY 11501 under the travelling Garm Bel Iblis
/// and Adar Tallon, and 11500 alone under Syub Snunb. port: no other built
/// object in the window is en route (it leaves out personnel on a visible
/// mission).
fn row_marks(
    world: &GameWorld,
    members: &[MemberTransit],
    regiments: &[RegimentTransit],
    row: &DefensesRow,
) -> Vec<(DllSource, u32)> {
    if let DefensesItem::Pending { kind, arrival, .. } = row.item {
        let side_one = match kind {
            BuildableKind::Fighter(class) => world
                .fighter_classes
                .get(class)
                .is_some_and(|class| class.is_alliance),
            BuildableKind::DefenseFacility(build) => build.is_alliance,
            BuildableKind::Troop(class) => world
                .buildable_classes
                .get(&class)
                .is_some_and(|class| class.is_alliance),
            _ => false,
        };
        // A regiment keeps its backdrop, 11513, under the mark.
        let backdrop = match kind {
            BuildableKind::Troop(class) => crate::fleet_window::mini_marks(
                crate::fleet_window::MiniObject::Regiment(class),
                row.mini,
                false,
                false,
            ),
            _ => Vec::new(),
        };
        return match arrival {
            // A finished regiment or squadron travelling to the planet
            // (bit 4) takes its craft's en route mark over its backdrop,
            // no unfinished one (A0 2026-11-11-regiment-en-route/00).
            Some(_) if matches!(kind, BuildableKind::Troop(_) | BuildableKind::Fighter(_)) => {
                let object = match kind {
                    BuildableKind::Troop(class) => crate::fleet_window::MiniObject::Regiment(class),
                    _ => crate::fleet_window::MiniObject::Craft,
                };
                crate::fleet_window::mini_marks(object, row.mini, true, false)
            }
            Some(_) => vec![(DllSource::Strategy, 11_509)],
            None => backdrop
                .into_iter()
                .chain([(DllSource::Strategy, if side_one { 11_570 } else { 11_572 })])
                .collect(),
        };
    }
    use crate::fleet_window::MiniObject;
    let injured = matches!(row.item, DefensesItem::Character(key)
        if world.characters.get(key).is_some_and(|character| character.injury > 0));
    let travelling = |member: MissionMember| members.iter().any(|transit| transit.member == member);
    let en_route = match row.item {
        DefensesItem::Troop(key) => regiments.iter().any(|transit| transit.troop == key),
        DefensesItem::ViewedTroop { alliance, id } => {
            crate::viewed_contents::regiment_view(world, viewer(alliance), id)
                .is_some_and(|view| view.arrival.is_some())
        }
        DefensesItem::Viewed { unit, .. } => unit.arrival().is_some(),
        DefensesItem::Character(key) => travelling(MissionMember::Character(key)),
        DefensesItem::SpecialForce(key) => travelling(MissionMember::SpecialForce(key)),
        _ => false,
    };
    let object = match row.item {
        DefensesItem::Character(_) => MiniObject::Character,
        DefensesItem::SpecialForce(key) => world
            .special_forces
            .get(key)
            .map_or(MiniObject::Character, |force| MiniObject::SpecialForce(force.class_dat_id)),
        DefensesItem::Troop(key) => world
            .troops
            .get(key)
            .map_or(MiniObject::Craft, |troop| MiniObject::Regiment(troop.class_dat_id)),
        DefensesItem::ViewedTroop { alliance, id } => {
            crate::viewed_contents::regiment_view(world, viewer(alliance), id)
                .map_or(MiniObject::Craft, |view| MiniObject::Regiment(view.class))
        }
        DefensesItem::Viewed {
            unit: crate::viewed_contents::ViewedUnit::SpecialForce { class, .. },
            ..
        } => MiniObject::SpecialForce(class),
        DefensesItem::Squadron { .. }
        | DefensesItem::TravellingSquadron { .. }
        | DefensesItem::Defense(_)
        | DefensesItem::Viewed { .. }
        | DefensesItem::Pending { .. } => MiniObject::Craft,
    };
    let en_route = en_route || matches!(row.item, DefensesItem::TravellingSquadron { .. });
    crate::fleet_window::mini_marks(object, row.mini, en_route, injured)
}

/// One list row: the object, its GOKRES mini and its name (`FUN_004a9ab0`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct DefensesRow {
    item: DefensesItem,
    mini: u32,
    label: String,
}

#[derive(Debug, Clone, PartialEq)]
struct OpenDefensesWindow {
    system: SystemKey,
    logical_position: (i16, i16),
    page: DefensesPage,
    /// The list's selected rows (`FUN_00609410`), by the `CoolDragList`
    /// rules ([`crate::drag_list`]).
    selected: ListSelection<DefensesItem>,
    /// The list's first visible pixel (`+0xc4`) and a press held on its
    /// bar. A new page keeps it: `FUN_004a90d0` empties the list
    /// (`FUN_005f5b20`) and lays it out (`FUN_0060a280`), and the layout
    /// only clamps it to the range (`FUN_00609ae0`); the draw clamps it.
    scroll: f32,
    scroll_hold: Option<crate::list_scroll_bar::Hold>,
    /// Where the window was when a press on its surface took it
    /// (`crate::window_move`).
    moving: MoveAnchor,
}

/// `CoolDragList` posts `0x29a` only when the release lies more than this
/// squared distance (24 list pixels) from the press (`FUN_006083c0`).
const DRAG_DISTANCE_SQUARED: f32 = 24.0;

/// A left press on the list, held until its release: the list has
/// captured the mouse (`FUN_006083c0`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct DefensesDrag {
    system: SystemKey,
    press: egui::Pos2,
    list: egui::Rect,
    /// Pages 1 to 3 clear the list's `0x20000`, so their items drag; pages 4
    /// and 5 set it (`FUN_004a90d0`).
    draggable: bool,
}

/// How a held left press ended ([`DefensesWindowState::end_drag`]).
#[derive(Debug, Clone, Copy, PartialEq)]
enum DragEnd {
    /// The release posted `0x29a`.
    Drop { system: SystemKey, point: egui::Pos2 },
    /// A release that is no drop: the list's held click applies there.
    Release { system: SystemKey, point: egui::Pos2 },
}

/// The open Defenses windows. The last is focused and paints on top.
#[derive(Debug)]
pub struct DefensesWindowState {
    faction: CockpitFaction,
    windows: Vec<OpenDefensesWindow>,
    /// The kind's claim on the shared window focus.
    focus: crate::window_focus::FocusStamp,
    drag: Option<DefensesDrag>,
    /// A window was created, not raised, since [`Self::take_created`]
    /// (`FUN_0045aac0` then calls `FUN_0041d2d0`).
    created: bool,
}

impl Default for DefensesWindowState {
    fn default() -> Self {
        Self {
            faction: CockpitFaction::Alliance,
            windows: Vec::new(),
            focus: crate::window_focus::FocusStamp::default(),
            drag: None,
            created: false,
        }
    }
}

impl DefensesWindowState {
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

    /// Open `system`'s Defenses window at a logical point, clamped into the
    /// galaxy view, on the personnel page (`FUN_004a8790` selects tab 1), or
    /// bring its open window to the front: one per system (`FUN_0045aac0`).
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
        self.created = true;
        self.windows.push(OpenDefensesWindow {
            system,
            logical_position: clamp_window_to_galaxy(
                logical_position,
                layout,
                DEFENSES_WINDOW_WIDTH,
                DEFENSES_WINDOW_HEIGHT,
            ),
            page: DefensesPage::Personnel,
            selected: ListSelection::default(),
            scroll: 0.0,
            scroll_hold: None,
            moving: None,
        });
        self.focus.take();
        true
    }

    #[must_use]
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    #[must_use]
    pub fn is_open(&self, system: SystemKey) -> bool {
        self.windows.iter().any(|window| window.system == system)
    }

    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        let point = egui::pos2(point.0, point.1);
        self.windows
            .iter()
            .any(|window| rect_contains(window_screen_rect(window, layout), point))
    }

    /// The screen rect of `system`'s window.
    #[must_use]
    pub fn screen_rect(&self, layout: CockpitLayout, system: SystemKey) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(window_screen_rect(window, layout))
    }

    /// Show `page` in `system`'s open window, as a click on its tab does
    /// (`FUN_004a90d0` refills only for a different page).
    pub fn show_page(&mut self, system: SystemKey, page: DefensesPage) -> bool {
        let Some(window) = self.window_mut(system) else {
            return false;
        };
        if window.page != page {
            window.page = page;
            window.selected.clear();
        }
        true
    }

    /// The screen rect of `page`'s button in `system`'s window.
    #[must_use]
    pub fn tab_screen_rect(
        &self,
        layout: CockpitLayout,
        system: SystemKey,
        page: DefensesPage,
    ) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(tab_rect(
            window_screen_rect(window, layout),
            layout.scale,
            page,
        ))
    }

    /// The screen rect of the `index`th cell of `system`'s list.
    #[must_use]
    pub fn cell_screen_rect(
        &self,
        layout: CockpitLayout,
        system: SystemKey,
        index: usize,
    ) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(cell_rect(
            window_screen_rect(window, layout),
            layout.scale,
            index,
            window.scroll,
        ))
    }

    /// What `system`'s window shows, for the interface fixture.
    #[must_use]
    pub fn report(
        &self,
        world: &GameWorld,
        fog: &FogState,
        missions: &MissionState,
        production: DefensesProduction<'_>,
        economy: &EconomyState,
        system: SystemKey,
    ) -> Option<DefensesWindowReport> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        let player = cockpit_faction(self.faction);
        let rows = page_rows(
            world,
            fog,
            missions,
            production,
            player,
            system,
            window.page,
        );
        let selection: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| window.selected.contains(row.item))
            .map(|(index, _)| index)
            .collect();
        let selected = selection.first().copied();
        Some(DefensesWindowReport {
            origin: window.logical_position,
            side: shown_side(world, player, system),
            page: window.page,
            counts: DefensesPage::ALL.map(|page| {
                page_rows(world, fog, missions, production, player, system, page).len()
            }),
            rows: rows.into_iter().map(|row| row.label).collect(),
            selected,
            selection,
            garrison: garrison_line(world, economy, player, system, window.page),
        })
    }

    /// What a Mission released over the Defenses window egui draws as
    /// `layer` targets (`+0x68`, `FUN_004aa380`): the object in the list
    /// cell under `point` where the list shows it, else the window's
    /// system. `None` when `layer` is none of them.
    #[must_use]
    pub fn mission_target(
        &self,
        world: &GameWorld,
        fog: &FogState,
        missions: &MissionState,
        production: DefensesProduction<'_>,
        layout: CockpitLayout,
        layer: egui::LayerId,
        point: egui::Pos2,
    ) -> Option<MissionTargetPick> {
        let window = self
            .windows
            .iter()
            .find(|window| area_id(window.system) == layer.id)?;
        let player = cockpit_faction(self.faction);
        let screen = window_screen_rect(window, layout);
        let rows = page_rows(
            world,
            fog,
            missions,
            production,
            player,
            window.system,
            window.page,
        );
        let bar = list_bar(rows.len(), window.scroll);
        let scroll = bar_first(&bar, window.scroll);
        let width = rows_width(&bar);
        let in_list = rect_contains(
            logical_rect(screen, layout.scale, LIST.0, LIST.1, width, LIST.3),
            point,
        );
        let item = rows
            .into_iter()
            .enumerate()
            .find(|(index, _)| {
                in_list && rect_contains(cell_rect(screen, layout.scale, *index, scroll), point)
            })
            .map(|(_, row)| row.item);
        let mut pick = MissionTargetPick::from(window.system);
        match item {
            Some(DefensesItem::Character(key)) => pick.character = Some(key),
            Some(DefensesItem::SpecialForce(key)) => {
                pick.object = Some(MissionTarget::SpecialForce(key));
            }
            Some(DefensesItem::Troop(key)) => pick.object = Some(MissionTarget::Troop(key)),
            // hyp: a mission's order names the object by id (the copy's
            // id is the true object's, FUN_005844e0's group), so a copy
            // targets its regiment while that still exists.
            Some(DefensesItem::ViewedTroop { id, .. }) => {
                pick.object = world
                    .object_views
                    .regiment_key(world, id)
                    .map(MissionTarget::Troop);
            }
            Some(DefensesItem::Defense(key)) => {
                pick.object = Some(MissionTarget::DefenseFacility(key));
            }
            // port: a mission names a squadron aboard a fleet only
            // (MissionTarget::Fighter), so one on a planet is no target,
            // and an unfinished unit is a queue entry.
            Some(
                DefensesItem::Squadron { .. }
                | DefensesItem::TravellingSquadron { .. }
                | DefensesItem::Viewed { .. }
                | DefensesItem::Pending { .. },
            )
            | None => {}
        }
        Some(pick)
    }

    /// The system a move released over the Defenses window egui draws as
    /// `layer` takes (`+0x70`, `FUN_004aa470`: the subject wherever the point
    /// is), or `None` when `layer` is none of them.
    #[must_use]
    pub fn release_target(&self, layer: egui::LayerId) -> Option<SystemKey> {
        self.windows
            .iter()
            .find(|window| area_id(window.system) == layer.id)
            .map(|window| window.system)
    }

    pub fn clear(&mut self) {
        self.windows.clear();
        self.drag = None;
    }

    /// Whether a left press on a list is held: the list has captured the
    /// mouse (`FUN_006083c0`).
    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// How far the pointer has moved since a press held on `system`'s list
    /// of a page that drags: the list drags on the first move with the
    /// button down (`FUN_006083c0`, case `0x200`).
    fn drag_offset(&self, ctx: &egui::Context, system: SystemKey) -> Option<egui::Vec2> {
        let drag = self.drag.filter(|drag| drag.system == system && drag.draggable)?;
        let offset = ctx.input(|input| input.pointer.latest_pos())? - drag.press;
        (offset != egui::Vec2::ZERO).then_some(offset)
    }

    /// End a held press on the left release (`FUN_006083c0`, case
    /// `0x202`): more than `sqrt(24)` list pixels from the press it is the
    /// `0x29a` drop, and the galaxy view moves the list's selection against
    /// the window under the point (`FUN_00422ce0`, window type 10: `0x201`,
    /// `0x202` with Ctrl). The list's flags `0xa0000` (`FUN_004a8790`) leave
    /// `0x100000` clear, so the release need not leave the list.
    ///
    /// port: a release inside the list is no drop; there `+0x70` gives only
    /// the window's own system (`FUN_004aa470`), a move in place.
    ///
    /// The distance is the release's own point's (`lParam` of
    /// `WM_LBUTTONUP`, [`crate::drag_list::release_point`]).
    fn end_drag(&mut self, ctx: &egui::Context, scale: f32) -> Option<DragEnd> {
        let (released, down) = ctx.input(|input| {
            (
                input.pointer.primary_released(),
                input.pointer.primary_down(),
            )
        });
        let point = crate::drag_list::release_point(ctx);
        if !released {
            if !down {
                self.drag = None;
            }
            return None;
        }
        let drag = self.drag.take()?;
        let point = point?;
        let moved = (point - drag.press) / scale;
        if drag.draggable
            && moved.length_sq() > DRAG_DISTANCE_SQUARED
            && !rect_contains(drag.list, point)
        {
            Some(DragEnd::Drop {
                system: drag.system,
                point,
            })
        } else {
            Some(DragEnd::Release {
                system: drag.system,
                point,
            })
        }
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

    fn close(&mut self, system: SystemKey) -> Option<OpenDefensesWindow> {
        let index = self
            .windows
            .iter()
            .position(|window| window.system == system)?;
        Some(self.windows.remove(index))
    }

    fn window_mut(&mut self, system: SystemKey) -> Option<&mut OpenDefensesWindow> {
        self.windows
            .iter_mut()
            .find(|window| window.system == system)
    }
}

/// What one Defenses window shows ([`DefensesWindowState::report`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefensesWindowReport {
    pub origin: (i16, i16),
    pub side: u8,
    pub page: DefensesPage,
    /// Each page's row count, in [`DefensesPage::ALL`] order.
    pub counts: [usize; 5],
    pub rows: Vec<String>,
    /// The first selected row.
    pub selected: Option<usize>,
    /// Every selected row, in list order.
    pub selection: Vec<usize>,
    pub garrison: Option<String>,
}

/// Actions that leave the Defenses window manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefensesWindowAction {
    /// The sector button (`0xca`) opens the subject's sector window
    /// (`FUN_00429ce0`).
    OpenSector(SystemKey),
    SelectSystem(SystemKey),
    /// The minimize button (`0xc9`) posts `0x466`: the window goes to the
    /// galaxy view's rail.
    Minimize {
        system: SystemKey,
        logical_position: (i16, i16),
        page: DefensesPage,
    },
    /// A right release on the list opens the object pop-up menu
    /// (`FUN_004ac5c0`) for the selection, at a 640 by 480 canvas point.
    OpenObjectMenu {
        system: SystemKey,
        selection: Option<MenuObject>,
        /// Every selected row's object, the clicked one first.
        team: Vec<MenuObject>,
        point: (i16, i16),
    },
    /// The list's selection dragged out of it (`0x29a`): the galaxy view
    /// issues `0x201` for it, `0x202` with Ctrl, against the window under
    /// the point (`FUN_00422ce0`, window type 10; manual p. 111). Its
    /// characters and special forces become the Move mission order `0x241`
    /// and the rest one `0x204` group (`FUN_0053c4b0`).
    DragSelection {
        system: SystemKey,
        items: Vec<DefensesItem>,
        point: egui::Pos2,
        confirmed: bool,
    },
}

/// The side the window shows (`+0x148`): the side bits of `player`'s copy
/// of the system (`FUN_004a8790`), refreshed when they change
/// (`FUN_004a7f50`).
fn shown_side(world: &GameWorld, player: Faction, system: SystemKey) -> u8 {
    // FUN_004a25c0 -> FUN_004f3220: the window's object is the viewer's
    // copy, and the tabs (FUN_004a9ce0's +0x148), title, garrison line
    // (FUN_004a90d0) and rail icon (FUN_004aa4a0) read its side bits.
    crate::viewed_contents::apparent_system(world, player, system)
        .map_or(0, |value| control_side(value.control))
}

/// A minimized Defenses window's rail icon (`FUN_004aa4a0`): 11533 for side
/// 1, 11534 for side 2, 11535 otherwise, by the side bits of `player`'s copy.
#[must_use]
pub fn rail_icon(world: &GameWorld, player: Faction, system: SystemKey) -> u32 {
    match shown_side(world, player, system) {
        1 => RAIL_ICONS[0],
        2 => RAIL_ICONS[1],
        _ => RAIL_ICONS[2],
    }
}

/// `FUN_004a9800`/`FUN_004a9ce0`: the title strips, by the shown side and
/// whether the window is the galaxy view's focused child. The Fleet window
/// and the Missions window (`FUN_004a2200`) use the same art.
pub(crate) fn title_resource(side: u8, focused: bool) -> u32 {
    match (side, focused) {
        (1, true) => 10299,
        (1, false) => 10200,
        (2, true) => 10201,
        (2, false) => 10302,
        (_, true) => 10303,
        (_, false) => 10304,
    }
}

/// A selected row's text colour (`+0xdc`, `FUN_004a9ce0`): `0x20000ff`
/// (red) for side 1, `0x200ff00` (green) for side 2, `0x2ffff00` (cyan)
/// otherwise. Unselected rows are white (`+0xd8`, `0x2ffffff`).
fn selected_color(side: u8) -> egui::Color32 {
    match side {
        1 => egui::Color32::from_rgb(255, 0, 0),
        2 => egui::Color32::from_rgb(0, 255, 0),
        _ => egui::Color32::from_rgb(0, 255, 255),
    }
}

/// `FUN_004a90d0`'s rows for `page`, each filtered to the shown side, in the
/// order the page's walk yields them (the list's sort mode 4 keeps it).
///
/// port: the player sees the other side's objects only where the System
/// window shows them (`opposing_contents_visible`). The walks follow the
/// port's lists: characters in key order, then special forces, regiments
/// and facilities in their system's order, and each squadron of the
/// planet's class entries in turn.
fn page_rows(
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    production: DefensesProduction<'_>,
    player: Faction,
    system: SystemKey,
    page: DefensesPage,
) -> Vec<DefensesRow> {
    // The player's copy's explored bit (ghidra/notes/fog-of-war.md); a
    // missing system is unexplored.
    if !rebellion_core::object_views::side_explored(world, player, system) {
        return Vec::new();
    }
    let sight = crate::viewed_contents::system_sight(world, fog, player, system);
    // A frozen copy keeps the system's side as it was seen.
    let side = match sight {
        crate::viewed_contents::SystemSight::Frozen(view) => control_side(view.control),
        _ => shown_side(world, player, system),
    };
    let other = side != faction_side(player);
    let visible = opposing_contents_visible(world, fog, player, system);
    // The other side's regiments are the player's side's copies of them
    // (FUN_004f2d10 -> FUN_0053efd0; FUN_004f9a60 freezes a copy out of
    // sight), unless the debug mode shows everything.
    if other && page == DefensesPage::Regiments && !crate::viewed_contents::sees_everything(world)
    {
        let mut rows = viewed_regiment_rows(world, player, side, system);
        if visible {
            rows.extend(pending_rows(world, production, side, system, page));
        }
        return rows;
    }
    if other {
        match sight {
            crate::viewed_contents::SystemSight::Live => {}
            crate::viewed_contents::SystemSight::Frozen(view) => {
                let travellers = crate::viewed_contents::frozen_extras(world, fog, player, system)
                    .map_or(&[][..], |extras| extras.travellers.as_slice());
                return frozen_rows(world, view, travellers, side, system, page);
            }
            crate::viewed_contents::SystemSight::Hidden => return Vec::new(),
        }
    }
    let mut rows = built_rows(world, missions, production, side, system, page);
    rows.extend(pending_rows(world, production, side, system, page));
    rows
}

/// `page`'s rows of the other side (`side`) from the player's frozen copy
/// of the system (`FUN_004f2d10` reads the side's copy; `FUN_004f9a60`
/// froze it): its characters, then special forces; its planet squadrons;
/// its shields or batteries. The regiments are copied one by one
/// ([`viewed_regiment_rows`]). port: a character's row is the live
/// character the copy names; the others open no menu.
fn frozen_rows(
    world: &GameWorld,
    view: &rebellion_core::object_views::SystemView,
    travellers: &[rebellion_core::object_views::TravellerCopy],
    side: u8,
    system: SystemKey,
    page: DefensesPage,
) -> Vec<DefensesRow> {
    use crate::viewed_contents::ViewedUnit;
    use rebellion_core::object_views::TravellerCopy;
    let mut rows = Vec::new();
    let viewed = |rows: &Vec<DefensesRow>, unit| DefensesItem::Viewed {
        system,
        index: rows.len(),
        unit,
    };
    // The units it saw leave for the system, en route there
    // (FUN_00556390; FUN_004f9a60 froze them in hyperspace).
    let travelling_characters = travellers.iter().filter_map(|traveller| match traveller {
        TravellerCopy::Character { id, .. } => Some(*id),
        _ => None,
    });
    let travelling_forces = travellers.iter().filter_map(|traveller| match traveller {
        TravellerCopy::SpecialForce {
            class,
            is_alliance,
            arrival,
        } => Some((*class, *is_alliance, Some(*arrival))),
        _ => None,
    });
    let travelling_squadrons = travellers.iter().filter_map(|traveller| match traveller {
        TravellerCopy::Squadron {
            class,
            is_alliance,
            count,
            arrival,
        } => Some((*class, *is_alliance, *count, Some(*arrival))),
        _ => None,
    });
    match page {
        DefensesPage::Personnel => {
            for id in view.characters.iter().copied().chain(travelling_characters) {
                let Some(key) = crate::viewed_contents::character_key(world, id) else {
                    continue;
                };
                let character = &world.characters[key];
                let character_side = match rebellion_core::missions::character_side(character) {
                    Some(Faction::Alliance) => 1,
                    Some(Faction::Empire) => 2,
                    _ => 0,
                };
                if character_side != side {
                    continue;
                }
                let Some(mini) = character_picture(character) else {
                    continue;
                };
                rows.push(DefensesRow {
                    item: DefensesItem::Character(key),
                    mini,
                    label: character.name.clone(),
                });
            }
            let forces = view
                .special_forces
                .iter()
                .map(|(class, is_alliance)| (*class, *is_alliance, None))
                .chain(travelling_forces);
            for (class, is_alliance, arrival) in forces {
                let Some((mini, label)) = special_force_mini(class) else {
                    continue;
                };
                if fleet_side(is_alliance) == side {
                    let unit = ViewedUnit::SpecialForce {
                        class,
                        is_alliance,
                        arrival,
                    };
                    rows.push(DefensesRow {
                        item: viewed(&rows, unit),
                        mini,
                        label: label.into(),
                    });
                }
            }
        }
        DefensesPage::Regiments => {}
        DefensesPage::Squadrons => {
            let squadrons = view
                .fighters
                .iter()
                .map(|(class, count)| (*class, None, *count, None))
                .chain(
                    travelling_squadrons
                        .map(|(class, is_alliance, count, arrival)| (class, Some(is_alliance), count, arrival)),
                );
            for (class, _, count, arrival) in squadrons {
                let Some(fighter) = world.fighter_classes.values().find(|value| value.dat_id == class)
                else {
                    continue;
                };
                let Some(mini) = fighter_mini_id(class) else {
                    continue;
                };
                if fleet_side(fighter.is_alliance) != side {
                    continue;
                }
                let unit = ViewedUnit::Squadron {
                    class,
                    is_alliance: fighter.is_alliance,
                    arrival,
                };
                for _ in 0..count {
                    rows.push(DefensesRow {
                        item: viewed(&rows, unit),
                        mini,
                        label: fighter.name.clone(),
                    });
                }
            }
        }
        DefensesPage::Shields | DefensesPage::Batteries => {
            for facility in &view.defense_facilities {
                if !defense_page_family(page, facility.class.family())
                    || crate::fleet_window::faction_side(facility.side) != side
                {
                    continue;
                }
                let Some((mini, label)) = defense_facility_mini(facility.class) else {
                    continue;
                };
                let unit = ViewedUnit::Facility {
                    class: facility.class,
                    owner: crate::fleet_window::faction_side(facility.side),
                };
                rows.push(DefensesRow {
                    item: viewed(&rows, unit),
                    mini,
                    label: label.into(),
                });
            }
        }
    }
    rows
}

/// The faction a `ViewedTroop`'s `alliance` names.
const fn viewer(alliance: bool) -> Faction {
    if alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    }
}

/// The Regiments page's rows of the other side (`side`) for `player`: its
/// copies of the regiments on `system`'s planet, live or frozen
/// (`viewed_contents::viewed_surface_regiments`).
fn viewed_regiment_rows(
    world: &GameWorld,
    player: Faction,
    side: u8,
    system: SystemKey,
) -> Vec<DefensesRow> {
    use crate::viewed_contents::{regiment_view, viewed_surface_regiments, ViewedRegiment};
    viewed_surface_regiments(world, player, system)
        .into_iter()
        .filter_map(|regiment| {
            let (item, class, is_alliance) = match regiment {
                ViewedRegiment::Live(key) => {
                    let troop = world.troops.get(key)?;
                    (DefensesItem::Troop(key), troop.class_dat_id, troop.is_alliance)
                }
                ViewedRegiment::View(id) => {
                    let view = regiment_view(world, player, id)?;
                    (
                        DefensesItem::ViewedTroop {
                            alliance: player == Faction::Alliance,
                            id,
                        },
                        view.class,
                        view.is_alliance,
                    )
                }
            };
            if fleet_side(is_alliance) != side {
                return None;
            }
            let (mini, label) = troop_mini(class)?;
            Some(DefensesRow {
                item,
                mini,
                label: label.into(),
            })
        })
        .collect()
}

/// The units being built in `system` (its unfinished regiments and
/// squadrons, `rebellion_core::products`; its facilities from the queues),
/// then those travelling there (en route), that `page` lists for `side`:
/// squadrons of the side's classes on page 2, its regiments in training on
/// page 1, its shields or batteries on pages 3 and 4. An unfinished unit
/// lists where it stands, which a Move may have changed. port: a squadron
/// or regiment en route is not listed.
fn pending_rows(
    world: &GameWorld,
    production: DefensesProduction<'_>,
    side: u8,
    system: SystemKey,
    page: DefensesPage,
) -> Vec<DefensesRow> {
    let queued = world
        .products
        .in_system(system)
        .map(|product| {
            (
                product.kind,
                world.products.arrival(product.id),
                Some(product.id),
            )
        })
        .chain(
            crate::manufacturing_window::pending_units(production.manufacturing, system)
                .filter(|&kind| !rebellion_core::products::is_tracked(kind))
                .map(|kind| (kind, None, None)),
        )
        .chain(
            production
                .deliveries
                .en_route()
                .iter()
                .filter(|delivery| delivery.destination == system)
                .filter(|delivery| matches!(delivery.kind, BuildableKind::DefenseFacility(_)))
                .map(|delivery| (delivery.kind, Some(delivery.arrival_tick), None)),
        );
    queued
        .enumerate()
        .filter_map(|(index, (kind, arrival, product))| {
            let (unit_side, mini, label) = match (page, kind) {
                (DefensesPage::Regiments, BuildableKind::Troop(class)) => {
                    let (mini, label) = troop_mini(class)?;
                    let alliance = world.buildable_classes.get(&class)?.is_alliance;
                    (fleet_side(alliance), mini, label.into())
                }
                (DefensesPage::Squadrons, BuildableKind::Fighter(class)) => {
                    let class = world.fighter_classes.get(class)?;
                    (
                        fleet_side(class.is_alliance),
                        fighter_mini_id(class.dat_id)?,
                        class.name.clone(),
                    )
                }
                (
                    DefensesPage::Shields | DefensesPage::Batteries,
                    BuildableKind::DefenseFacility(build),
                ) => {
                    if !defense_page_family(page, build.class.family()) {
                        return None;
                    }
                    let (mini, label) = defense_facility_mini(build.class)?;
                    (fleet_side(build.is_alliance), mini, label.into())
                }
                _ => return None,
            };
            (unit_side == side).then_some(DefensesRow {
                item: DefensesItem::Pending {
                    system,
                    index,
                    kind,
                    arrival,
                    product,
                },
                mini,
                label,
            })
        })
        .collect()
}

/// `FUN_00527050`'s families: 0x24 and 0x25 on page 4, 0x22 and 0x23 on
/// page 5.
fn defense_page_family(page: DefensesPage, family: u8) -> bool {
    if page == DefensesPage::Shields {
        matches!(family, 0x24 | 0x25)
    } else {
        matches!(family, 0x22 | 0x23)
    }
}

/// The built objects `page` lists for `side` at `system`.
fn built_rows(
    world: &GameWorld,
    missions: &MissionState,
    production: DefensesProduction<'_>,
    side: u8,
    system: SystemKey,
    page: DefensesPage,
) -> Vec<DefensesRow> {
    let Some(value) = world.systems.get(system) else {
        return Vec::new();
    };
    match page {
        // FUN_00536e20: members with no mission key, or on a hidden mission
        // (`+0x78` bit 8).
        DefensesPage::Personnel => {
            let keys = mission_keys(missions);
            // A member on its way is already in its destination's container:
            // the Move mission is a hidden one (MISSNSD +0x5c). A0
            // specforces/03: Mon Calamari lists Garm Bel Iblis, Adar Tallon
            // and the Longprobe team while they travel there.
            let travelling = missions
                .en_route()
                .iter()
                .filter(|transit| {
                    transit.to == system
                        && transit.abode.is_none()
                        && crate::quadrant_icons::bound_fleet(missions, transit).is_none()
                })
                .filter_map(|transit| {
                    let side = match transit.member {
                        MissionMember::Character(key) => {
                            match rebellion_core::missions::character_side(world.characters.get(key)?) {
                                Some(rebellion_core::dat::Faction::Alliance) => 1,
                                Some(rebellion_core::dat::Faction::Empire) => 2,
                                _ => 0,
                            }
                        }
                        MissionMember::SpecialForce(key) => {
                            fleet_side(world.special_forces.get(key)?.is_alliance)
                        }
                    };
                    Some((transit.member, side, true))
                });
            let mut rows = system_members(world, system)
                .chain(travelling)
                .filter(|(member, member_side, hidden)| {
                    *member_side == side && (*hidden || !keys.contains_key(member))
                })
                .filter_map(|(member, _, _)| match member {
                    MissionMember::Character(key) => {
                        let character = world.characters.get(key)?;
                        Some(DefensesRow {
                            item: DefensesItem::Character(key),
                            mini: character_picture(character)?,
                            label: character.name.clone(),
                        })
                    }
                    MissionMember::SpecialForce(key) => {
                        let force = world.special_forces.get(key)?;
                        let (mini, label) = special_force_mini(force.class_dat_id)?;
                        Some(DefensesRow {
                            item: DefensesItem::SpecialForce(key),
                            mini,
                            label: label.into(),
                        })
                    }
                })
                .collect::<Vec<_>>();
            // The characters, then the special forces (A0 specforces/03).
            rows.sort_by_key(|row| matches!(row.item, DefensesItem::SpecialForce(_)));
            rows
        }
        // FUN_00504cc0. A regiment on its way to the planet is already in
        // its destination (FUN_00556390): A0 2026-11-11-regiment-en-route/00
        // lists Averam's travelling Alliance Army Regiment first, then the
        // one in training. port: after the planet's own.
        DefensesPage::Regiments => value
            .ground_units
            .iter()
            .copied()
            .chain(
                production
                    .regiments
                    .iter()
                    .filter(|transit| transit.leg == RegimentLeg::Surface(system))
                    .map(|transit| transit.troop),
            )
            .filter_map(|key| {
                let troop = world.troops.get(key)?;
                if fleet_side(troop.is_alliance) != side {
                    return None;
                }
                let (mini, label) = troop_mini(troop.class_dat_id)?;
                Some(DefensesRow {
                    item: DefensesItem::Troop(key),
                    mini,
                    label: label.into(),
                })
            })
            .collect(),
        // FUN_005039d0: the squadrons on the planet (0x1c..0x1f). One on its
        // way there by Move is already in it (FUN_00556390), as a regiment
        // is (A0 2026-11-11-regiment-en-route/00). port: after the planet's
        // own.
        DefensesPage::Squadrons => {
            let travelling = production
                .squadrons
                .iter()
                .filter(|transit| transit.to == SquadronPlace::Planet(system))
                .map(|transit| (transit.class, transit.count, Some(transit.arrival_tick)));
            value
                .fighters
                .iter()
                .map(|entry| (entry.class, entry.count, None))
                .chain(travelling)
                .filter_map(|(key, count, arrival)| {
                    let class = world.fighter_classes.get(key)?;
                    if fleet_side(class.is_alliance) != side {
                        return None;
                    }
                    let mini = fighter_mini_id(class.dat_id)?;
                    Some((0..count).map(move |nth| DefensesRow {
                        item: match arrival {
                            None => DefensesItem::Squadron {
                                system,
                                class: key,
                                nth,
                            },
                            Some(arrival) => DefensesItem::TravellingSquadron {
                                system,
                                class: key,
                                arrival,
                                nth,
                            },
                        },
                        mini,
                        label: class.name.clone(),
                    }))
                })
                .flatten()
                .collect()
        }
        // FUN_00527050, keeping families 0x24 and 0x25 (page 4) or 0x22 and
        // 0x23 (page 5).
        DefensesPage::Shields | DefensesPage::Batteries => value
            .defense_facilities
            .iter()
            .filter_map(|&key| {
                let facility = world.defense_facilities.get(key)?;
                if !defense_page_family(page, facility.class_dat_id.family())
                    || crate::fleet_window::faction_side(facility.side) != side
                {
                    return None;
                }
                let (mini, label) = defense_facility_mini(facility.class_dat_id)?;
                Some(DefensesRow {
                    item: DefensesItem::Defense(key),
                    mini,
                    label: label.into(),
                })
            })
            .collect(),
    }
}

/// The shared tail of `FUN_004a90d0`: on the regiment page, when the shown
/// side is the player's (`FUN_0041cdb0`), TEXTSTRA 6471 "Garrison
/// Requirement: " and the system's stored requirement (`+0x80`).
fn garrison_line(
    world: &GameWorld,
    economy: &EconomyState,
    player: Faction,
    system: SystemKey,
    page: DefensesPage,
) -> Option<String> {
    if page != DefensesPage::Regiments || shown_side(world, player, system) != faction_side(player)
    {
        return None;
    }
    let requirement = economy
        .per_system
        .get(&system)
        .map_or(0, |value| value.garrison_requirement);
    Some(format!("Garrison Requirement: {requirement}"))
}

fn cockpit_faction(faction: CockpitFaction) -> Faction {
    match faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    }
}

fn area_id(system: SystemKey) -> egui::Id {
    egui::Id::new(("original-defenses-window", system))
}

fn window_screen_rect(window: &OpenDefensesWindow, layout: CockpitLayout) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            layout.canvas.x + f32::from(window.logical_position.0) * layout.scale,
            layout.canvas.y + f32::from(window.logical_position.1) * layout.scale,
        ),
        egui::vec2(
            DEFENSES_WINDOW_WIDTH * layout.scale,
            DEFENSES_WINDOW_HEIGHT * layout.scale,
        ),
    )
}

fn tab_rect(window: egui::Rect, scale: f32, page: DefensesPage) -> egui::Rect {
    logical_rect(
        window,
        scale,
        TAB_STRIP.0 + page.x(),
        TAB_STRIP.1,
        TAB_SIZE.0,
        TAB_SIZE.1,
    )
}

/// The `index`th grid cell's corner in list pixels (`FUN_00609ae0`).
#[expect(clippy::cast_precision_loss, reason = "Cell indexes are small.")]
fn list_cell(index: usize) -> (f32, f32) {
    ((index % COLUMNS) as f32 * CELL, (index / COLUMNS) as f32 * CELL)
}

/// The `index`th grid cell (`FUN_00609ae0`): row-major from the list's
/// corner, three to a row, the list showing from pixel `scroll`.
#[expect(clippy::cast_precision_loss, reason = "Cell indexes are small.")]
fn cell_rect(window: egui::Rect, scale: f32, index: usize, scroll: f32) -> egui::Rect {
    let column = (index % COLUMNS) as f32;
    let row = (index / COLUMNS) as f32;
    logical_rect(
        window,
        scale,
        LIST.0 + column * CELL,
        LIST.1 + row * CELL - scroll,
        CELL,
        CELL,
    )
}

/// The grid's height for `rows` rows (`FUN_00609ae0`): a cell's height per
/// started line of three.
#[expect(clippy::cast_precision_loss, reason = "Row counts are small.")]
fn list_content(rows: usize) -> f32 {
    rows.div_ceil(COLUMNS) as f32 * CELL
}

/// The list's bar for `rows` rows shown from pixel `scroll`
/// (`FUN_0060a490` with RCDATA 10700, the list's own rectangle).
fn list_bar(rows: usize, scroll: f32) -> crate::list_scroll_bar::ScrollBar {
    crate::list_scroll_bar::ScrollBar::new(
        LIST_BAR_ART,
        LIST.0,
        LIST.1,
        LIST.2,
        LIST.3,
        list_content(rows),
        scroll,
    )
}

/// The first visible pixel `bar` shows: `scroll` within its range, or 0
/// once everything fits and the bar hides (`FUN_00609ae0` clamps `+0xc4`).
fn bar_first(bar: &crate::list_scroll_bar::ScrollBar, scroll: f32) -> f32 {
    if bar.visible() {
        scroll.clamp(0.0, bar.max)
    } else {
        0.0
    }
}

/// The width of the list the rows take: all of it, less a shown bar.
fn rows_width(bar: &crate::list_scroll_bar::ScrollBar) -> f32 {
    if bar.visible() {
        bar.x - LIST.0
    } else {
        LIST.2
    }
}

#[derive(Default)]
struct WindowDrawResult {
    focus: bool,
    close: bool,
    minimize: bool,
    open_sector: bool,
    page: Option<DefensesPage>,
    /// A left or right press on the list: the item under it, the list
    /// point and the keys held (`FUN_006083c0`).
    press: Option<(Option<DefensesItem>, (f32, f32), Modifiers)>,
    /// The item and list point under a held left press's release.
    release: Option<(Option<DefensesItem>, (f32, f32))>,
    /// The listed items and their cells, for a Shift press's range.
    cells: Vec<(DefensesItem, (f32, f32), (f32, f32))>,
    /// The selection and canvas point of a right release on the list.
    object_menu: Option<(Option<MenuObject>, (i16, i16))>,
    /// A left press on the list, held for a drag.
    drag: Option<DefensesDrag>,
    /// The pointer's travel since a press on the window's surface.
    travel: Option<egui::Vec2>,
    /// The list's first visible pixel and bar hold after this frame.
    scroll: Option<(f32, Option<crate::list_scroll_bar::Hold>)>,
}

/// Draw every open Defenses window.
///
/// port: the window's keys (`FUN_004aa2b0`: Escape closes, left and right
/// step the tabs) are not ported; the galaxy view's windows keep no shared
/// keyboard focus to route them by.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit state and rendering inputs at this UI boundary, as the Fleet window does."
)]
pub fn draw_defenses_windows(
    ctx: &egui::Context,
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    production: DefensesProduction<'_>,
    economy: &EconomyState,
    state: &mut DefensesWindowState,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Vec<DefensesWindowAction> {
    state.prepare_faction(faction);
    let windows = state.windows.clone();
    let focused_system = windows
        .last()
        .filter(|_| state.focus.holds())
        .map(|window| window.system);
    let ended = state.end_drag(ctx, layout.scale);
    let mut actions = Vec::new();
    for window in &windows {
        let released = match ended {
            Some(DragEnd::Release { system, point }) if system == window.system => Some(point),
            _ => None,
        };
        let mut result = draw_defenses_window(
            ctx,
            world,
            fog,
            missions,
            production,
            economy,
            window,
            focused_system == Some(window.system),
            faction,
            layout,
            cache,
            released,
            state.drag_offset(ctx, window.system),
        );
        if let Some(DragEnd::Drop { system, point }) = ended {
            if system == window.system {
                let items: Vec<DefensesItem> = result
                    .cells
                    .iter()
                    .map(|(item, _, _)| *item)
                    .filter(|item| window.selected.contains(*item))
                    .collect();
                if !items.is_empty() {
                    actions.push(DefensesWindowAction::DragSelection {
                        system,
                        items,
                        point,
                        confirmed: crate::drag_list::modifiers(ctx).ctrl,
                    });
                }
                if let Some(open) = state.window_mut(system) {
                    open.selected.dropped();
                }
            }
        }
        let system = window.system;
        if result.close || !world.systems.contains_key(system) {
            state.close(system);
            continue;
        }
        if result.minimize {
            if let Some(closed) = state.close(system) {
                actions.push(DefensesWindowAction::Minimize {
                    system,
                    logical_position: closed.logical_position,
                    page: closed.page,
                });
            }
            continue;
        }
        if result.open_sector {
            actions.push(DefensesWindowAction::OpenSector(system));
        }
        if let Some(open) = state.window_mut(system) {
            if result.travel.is_some() && open.moving.is_none() {
                result.focus = true;
            }
            window_move::follow(
                &mut open.logical_position,
                &mut open.moving,
                result.travel,
                |position| {
                    clamp_window_to_galaxy(
                        position,
                        layout,
                        DEFENSES_WINDOW_WIDTH,
                        DEFENSES_WINDOW_HEIGHT,
                    )
                },
            );
            // FUN_004a90d0 refills only for a different page.
            if let Some(page) = result.page.filter(|&page| page != open.page) {
                open.page = page;
                open.selected.clear();
            }
            // An item the page no longer lists is no longer selected.
            if result.page.is_none() {
                open.selected
                    .retain(|item| result.cells.iter().any(|(listed, _, _)| listed == item));
            }
            if let Some((hit, point, modifiers)) = result.press {
                open.selected.press(hit, point, modifiers, &result.cells);
            }
            if let Some((hit, point)) = result.release {
                open.selected
                    .release(hit, point, crate::drag_list::modifiers(ctx), &result.cells);
            }
            if let Some((first, hold)) = result.scroll {
                open.scroll = first;
                open.scroll_hold = hold;
            }
        }
        if result.focus {
            state.focus(system);
            actions.push(DefensesWindowAction::SelectSystem(system));
        }
        if let Some((selection, point)) = result.object_menu {
            // The menu takes the list's whole selection (slot 22,
            // FUN_004a7a20), the clicked row first.
            // Squadrons of one class share their object, so each selected
            // row stays in the team.
            let team = selection.map_or_else(Vec::new, |clicked| {
                let selected: Vec<MenuObject> = result
                    .cells
                    .iter()
                    .filter(|(item, _, _)| window.selected.contains(*item))
                    .filter_map(|(item, _, _)| item.menu_object())
                    .collect();
                if selected.contains(&clicked) {
                    crate::drag_list::clicked_first(clicked, selected)
                } else {
                    vec![clicked]
                }
            });
            actions.push(DefensesWindowAction::OpenObjectMenu {
                system,
                selection,
                team,
                point,
            });
        }
        if let Some(drag) = result.drag {
            state.drag = Some(drag);
        }
    }
    actions
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Keep the window's ordered paint and input pass together, as the Fleet window does."
)]
fn draw_defenses_window(
    ctx: &egui::Context,
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    production: DefensesProduction<'_>,
    economy: &EconomyState,
    window: &OpenDefensesWindow,
    focused: bool,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
    released: Option<egui::Pos2>,
    drag_image: Option<egui::Vec2>,
) -> WindowDrawResult {
    let mut result = WindowDrawResult::default();
    let Some(system) = world.systems.get(window.system) else {
        result.close = true;
        return result;
    };
    let player = cockpit_faction(faction);
    let side = shown_side(world, player, window.system);
    let rows = page_rows(
        world,
        fog,
        missions,
        production,
        player,
        window.system,
        window.page,
    );
    let scale = layout.scale;
    let screen_rect = window_screen_rect(window, layout);
    // Each row's cell in list pixels (`FUN_00609ae0`'s grid).
    result.cells = rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.item, list_cell(index), (CELL, CELL)))
        .collect();
    // List points are content pixels: the client point plus the first
    // visible pixel (FUN_00609d80); the rows take the list less a shown bar.
    let (first, rows_wide) = {
        let bar = list_bar(rows.len(), window.scroll);
        (bar_first(&bar, window.scroll), rows_width(&bar))
    };
    let list_point = |point: egui::Pos2| {
        (
            (point.x - screen_rect.min.x) / scale - LIST.0,
            (point.y - screen_rect.min.y) / scale - LIST.1 + first,
        )
    };
    let item_at = |(x, y): (f32, f32)| {
        (x >= 0.0 && y >= first && x < rows_wide && y < first + LIST.3)
            .then(|| {
                result
                    .cells
                    .iter()
                    .find(|(_, origin, _)| {
                        x >= origin.0 && x < origin.0 + CELL && y >= origin.1 && y < origin.1 + CELL
                    })
                    .map(|(item, _, _)| *item)
            })
            .flatten()
    };
    if let Some(point) = released {
        let at = list_point(point);
        result.release = Some((item_at(at), at));
    }
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
            let painter = ui.painter().clone();
            let paint = |cache: &mut BmpCache, id: u32, x: f32, y: f32| {
                paint_native(
                    &painter,
                    ctx,
                    cache,
                    DllSource::Strategy,
                    id,
                    local,
                    scale,
                    x,
                    y,
                );
            };

            painter.rect_filled(local, 0.0, egui::Color32::BLACK);
            paint(cache, BACKGROUND, 0.0, 0.0);
            paint(cache, title_resource(side, focused), 2.0, 2.0);
            // The title label `+0x49` starts after the sector button, in
            // font 5 with format `0x24`: one line, left-aligned and
            // vertically centred (`FUN_004a8790`).
            crate::theme::paint_gdi_text(
                &painter,
                logical_rect(local, scale, TITLE.0, TITLE.1, 180.0, TITLE.2),
                scale,
                5,
                &system.name,
                egui::Color32::BLACK,
                (
                    crate::theme::GdiAlign::Left,
                    crate::theme::GdiVAlign::VCenter,
                ),
            );

            let buttons = [
                (SECTOR_NORMAL, SECTOR_PRESSED, 3.0, "sector"),
                (MINIMIZE_NORMAL, MINIMIZE_PRESSED, 204.0, "minimize"),
                (CLOSE_NORMAL, CLOSE_PRESSED, 218.0, "close"),
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
                paint(cache, if down { pressed } else { normal }, x, 3.0);
                // The buttons are children blitted keyed over the window
                // (FUN_004a8790 makes them with FUN_00606c90; FUN_00602d30
                // -> FUN_005fd0f0): A0 garrison-clean/test-day483-yavin-
                // regiments shows 10577's black at (3, 16), (204, 16) and
                // (218, 16), not the key's green or the title strip.
                paint_key_pixel(
                    &painter,
                    ctx,
                    cache,
                    local,
                    scale,
                    x,
                    16.0,
                    KeyPixel::Background(BACKGROUND),
                );
                clicked[index] = exact_clicked(&response, rect);
            }
            result.open_sector = clicked[0];
            result.minimize = clicked[1];
            result.close = clicked[2];

            // The tabs. No tab is ever disabled; an empty one opens its
            // empty page. Each has its tip (`FUN_004a8790` via
            // `FUN_00600a40`).
            for page in DefensesPage::ALL {
                let rect = tab_rect(local, scale, page);
                let empty = page_rows(
                    world,
                    fog,
                    missions,
                    production,
                    player,
                    window.system,
                    page,
                )
                .is_empty();
                paint_native(
                    &painter,
                    ctx,
                    cache,
                    DllSource::Strategy,
                    tab_resource(page, side, page == window.page, empty),
                    rect,
                    scale,
                    0.0,
                    0.0,
                );
                let response = ui.interact(
                    rect,
                    ui.id().with((window.system, page)),
                    egui::Sense::click(),
                );
                crate::win_tooltip::response_tool(&response, rect, page.tooltip_string());
                if exact_clicked(&response, rect) {
                    result.page = Some(page);
                    result.focus = true;
                }
            }

            // `+0x5b` and `+0x5c`: white, font 4, format `0x25`: one line,
            // centred both ways in its rect.
            let centred = (
                crate::theme::GdiAlign::Center,
                crate::theme::GdiVAlign::VCenter,
            );
            crate::theme::paint_gdi_text(
                &painter,
                logical_rect(local, scale, TAB_NAME.0, TAB_NAME.1, TAB_NAME.2, TAB_NAME.3),
                scale,
                4,
                window.page.name(),
                egui::Color32::WHITE,
                centred,
            );
            if let Some(line) = garrison_line(world, economy, player, window.system, window.page) {
                crate::theme::paint_gdi_text(
                    &painter,
                    logical_rect(local, scale, GARRISON.0, GARRISON.1, GARRISON.2, GARRISON.3),
                    scale,
                    4,
                    &line,
                    egui::Color32::WHITE,
                    centred,
                );
            }

            // The list: each row's image keyed at its cell's corner, the
            // frame over the mini while selected (`FUN_0060bd00`), and its
            // name word-wrapped from (1, 27), shown from the bar's first
            // pixel.
            let list = logical_rect(local, scale, LIST.0, LIST.1, LIST.2, LIST.3);
            let list_painter = painter.with_clip_rect(list);
            let bar = list_bar(rows.len(), window.scroll);
            let scroll = bar_first(&bar, window.scroll);
            let width = rows_width(&bar);
            // The bar is a child window over the list's right edge
            // (FUN_0060fa80): it takes its own presses.
            let rows_area = logical_rect(local, scale, LIST.0, LIST.1, width, LIST.3);
            // The list control under the rows (FUN_006083c0): a left or
            // right press selects by the CoolDragList rules (on empty
            // space, FUN_006094b0 clears), and a right release on empty
            // space opens the menu for the empty selection.
            let list_response = ui.interact(
                rows_area,
                ui.id().with((window.system, "list")),
                egui::Sense::click(),
            );
            let (left_pressed, any_pressed, release_point, press_origin) = ctx.input(|input| {
                (
                    input.pointer.button_pressed(egui::PointerButton::Primary),
                    input.pointer.button_pressed(egui::PointerButton::Primary)
                        || input.pointer.button_pressed(egui::PointerButton::Secondary),
                    input.pointer.interact_pos(),
                    input.pointer.press_origin(),
                )
            });
            let pressed_here = |response: &egui::Response| {
                any_pressed && response.is_pointer_button_down_on()
            };
            let mut row_pressed = false;
            if list_response.secondary_clicked() {
                if let Some(point) = release_point.filter(|point| rect_contains(rows_area, *point))
                {
                    result.object_menu = Some((None, canvas_point(layout, point)));
                }
            }
            // The list writes in font 10 (FUN_004a8790: its slot +0x18
            // with 10).
            let frame = if side == 1 {
                ROW_FRAME[0]
            } else {
                ROW_FRAME[1]
            };
            for (index, row) in rows.iter().enumerate() {
                let cell = cell_rect(local, scale, index, scroll);
                // Only a cell with a row of pixels in the list shows.
                if cell.min.y >= list.max.y || cell.max.y <= list.min.y {
                    continue;
                }
                let selected = window.selected.contains(row.item);
                // FUN_0042c3b0: an injured character's damage mark under
                // its mini (`+0x50` bit `0x200`, FUN_004f7800).
                // A captive's mark goes over the mini (FUN_0042c3b0).
                let captive = matches!(row.item, DefensesItem::Character(key)
                    if world.characters.get(key).is_some_and(|value| value.is_captive))
                .then(|| crate::fleet_window::captive_mark(row.mini));
                // The row's image; while a drag is held, the selected rows'
                // images follow the pointer (FUN_00609960 copies them into
                // the list-sized drag bitmap, FUN_0060dbb0 / FUN_0060df00).
                let mut targets = vec![(list_painter.clone(), cell)];
                if let (true, Some(offset)) = (selected, drag_image) {
                    targets.push((
                        ctx.layer_painter(crate::fleet_window::drag_layer())
                            .with_clip_rect(list.translate(offset)),
                        cell.translate(offset),
                    ));
                }
                for (target, cell) in targets {
                    for (source, id) in row_marks(world, missions.en_route(), production.regiments, row)
                        .into_iter()
                        .chain(std::iter::once((DllSource::Gokres, row.mini)))
                        .chain(captive)
                    {
                        paint_native(&target, ctx, cache, source, id, cell, scale, 0.0, 0.0);
                    }
                    if selected {
                        paint_native(
                            &target,
                            ctx,
                            cache,
                            DllSource::Strategy,
                            frame,
                            cell,
                            scale,
                            0.0,
                            0.0,
                        );
                    }
                }
                let color = if selected {
                    selected_color(side)
                } else {
                    egui::Color32::WHITE
                };
                // DT_WORDBREAK in the cell less its trim, font 10.
                let _ = crate::theme::paint_gdi_text_block(
                    &list_painter,
                    logical_rect(
                        cell,
                        scale,
                        TEXT_OFFSET.0,
                        TEXT_OFFSET.1,
                        CELL - TEXT_RIGHT_TRIM,
                        CELL - TEXT_OFFSET.1,
                    ),
                    scale,
                    10,
                    &row.label,
                    color,
                    crate::theme::GdiAlign::Left,
                );
                let cell_area = cell.intersect(rows_area);
                let response = ui.interact(
                    cell_area,
                    ui.id().with((window.system, "row", index)),
                    egui::Sense::click(),
                );
                // A left or right press selects (FUN_006083c0 shares the
                // WM_LBUTTONDOWN path); a right release opens the menu.
                if pressed_here(&response) {
                    row_pressed = true;
                }
                if response.secondary_clicked() {
                    if let (Some(object), Some(point)) = (
                        row.item.menu_object(),
                        response
                            .interact_pointer_pos()
                            .filter(|point| rect_contains(cell_area, *point)),
                    ) {
                        result.object_menu = Some((Some(object), canvas_point(layout, point)));
                    }
                }
            }
            // The bar (FUN_0060a490) shows once the rows pass the list's
            // height; its arrows, track and thumb scroll it.
            if bar.visible() {
                let mut hold = window.scroll_hold;
                let first = crate::list_scroll_bar::ui_bar(
                    ui,
                    cache,
                    local,
                    scale,
                    LIST_BAR_ART,
                    LIST,
                    list_content(rows.len()),
                    scroll,
                    &mut hold,
                    LIST_STEP,
                );
                result.scroll = Some((first, hold));
            } else if window.scroll != 0.0 || window.scroll_hold.is_some() {
                result.scroll = Some((0.0, None));
            }

            // The press, wherever it landed in the list, selects by the
            // item under it; a left press captures the mouse for a drag.
            if let (true, Some(press)) = (
                row_pressed || pressed_here(&list_response),
                press_origin.filter(|point| rect_contains(rows_area, *point)),
            ) {
                let at = list_point(press);
                result.press = Some((item_at(at), at, crate::drag_list::modifiers(ctx)));
                result.focus = true;
                if left_pressed {
                    result.drag = Some(DefensesDrag {
                        system: window.system,
                        press,
                        list,
                        draggable: matches!(
                            window.page,
                            DefensesPage::Personnel | DefensesPage::Regiments | DefensesPage::Squadrons
                        ),
                    });
                }
            }

            if window_response.clicked() || clicked.iter().any(|&value| value) {
                result.focus = true;
            }
            // The child controls: the title buttons, the tabs and the list.
            let mut controls: Vec<egui::Rect> = [3.0, 204.0, 218.0]
                .into_iter()
                .map(|x| logical_rect(local, scale, x, 3.0, 14.0, 14.0))
                .collect();
            controls.extend(DefensesPage::ALL.map(|page| tab_rect(local, scale, page)));
            controls.push(logical_rect(local, scale, LIST.0, LIST.1, LIST.2, LIST.3));
            result.travel =
                window_move::held_travel(ui, &window_response, &controls, window.moving, scale);
        });
    if area.response.clicked() {
        result.focus = true;
    }
    result
}

#[cfg(test)]
#[path = "defenses_window_tests.rs"]
mod tests;
