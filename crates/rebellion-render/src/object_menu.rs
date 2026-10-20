//! The object pop-up menu a right-click opens (`FUN_004ac5c0`): on a
//! character, special force or regiment in the Defenses window, on a fleet,
//! ship or regiment in the Fleet window, on a sector window's icon, or on a
//! producer band in the Manufacturing window. Recovery notes:
//! `ghidra/notes/object-popup-menu.md`, `ghidra/notes/move-order.md` and
//! `ghidra/notes/fleet-join-split.md`.
//!
//! `FUN_0051d990` lists the orders the selection's class offers, sorts them
//! by their STRATEGY `RT_RCDATA` record's key, and always adds Encyclopedia
//! and Status. Each record names its text (word 5, TEXTSTRA), its parent
//! submenu (word 1) and its sort key (word 2).

use egui_macroquad::egui;
use rebellion_core::ids::{
    CharacterKey, DefenseFacilityKey, FighterKey, FleetKey, SpecialForceKey, SystemKey, TroopKey,
};
use rebellion_core::manufacturing::ProductionArea;
use rebellion_core::missions::MissionMember;

use crate::bmp_cache::BmpCache;
use crate::cockpit::{CockpitFaction, CockpitLayout, CockpitViewport};
use crate::game_menu::{
    draw_game_menu_with_submenu, GameMenuEntry, GameMenuPlacement, GameMenuResponse, GameMenuState,
    GameSubmenu,
};
use crate::quadrant_icons::Quadrant;
use crate::status_rows::StatusObject;

/// What an item does when chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectMenuCommand {
    Move,
    ConfirmedMove,
    Mission,
    Command,
    Encyclopedia,
    Status,
    Retire,
    Bombardment,
    /// A Planetary Bombardment target (`0x220..0x223`), by its kind.
    BombardmentTarget(u32),
    /// A Command order (`0x260..0x263`: None, Commander, Admiral, General),
    /// by its kind.
    GiveCommand(u16),
    Assault,
    Rename,
    Scrap,
    /// A capital ship's Create Fleet (`0x270`).
    CreateFleet,
    /// A facility's Destination (`0x214`).
    Destination,
    /// A facility's Reserved (`0x216`).
    Reserved,
    /// A mission's Abort (`0x250`).
    Abort,
    /// A production manager's Build (`0x210..0x212`).
    Build,
    /// A production manager's Stop (`0x213`).
    Stop,
}

/// The object a menu opens for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuObject {
    Character(CharacterKey),
    SpecialForce(SpecialForceKey),
    Fleet(FleetKey),
    /// A regiment (`0x10..0x13`).
    Troop(TroopKey),
    /// A capital ship (`0x14..0x1b`): the fleet that holds it, its index
    /// in [`Fleet::capital_ships`](rebellion_core::world::Fleet), and the
    /// fleet's roster when it was chosen (`fleet_join::roster`).
    Ship {
        fleet: FleetKey,
        index: usize,
        roster: u64,
    },
    /// A sector window's quadrant icon: its system, with the icon's kind
    /// (`FUN_004f5b10`: item `+0x68` and `flag >> 16`).
    SystemIcon {
        system: SystemKey,
        quadrant: Quadrant,
    },
    /// A Manufacturing window band: its system's production manager for
    /// `area` (families `0xa0..0xaf`, `FUN_00509670`).
    Producer {
        system: SystemKey,
        area: ProductionArea,
    },
    /// A Fleet window squadron item (`0x1c..0x1f`): the fleet and its
    /// squadron entry's index. port: squadrons are counts per class.
    Fighter {
        fleet: FleetKey,
        index: usize,
    },
    /// A Defenses window squadron on `system`'s planet (`0x1c..0x1f`), by
    /// its class. port: squadrons are counts per class.
    PlanetFighter {
        system: SystemKey,
        class: FighterKey,
    },
    /// A Defenses window facility (`0x20..0x27`).
    DefenseFacility(DefenseFacilityKey),
}

impl MenuObject {
    /// The object as a mission team member; a fleet or regiment is none.
    #[must_use]
    pub const fn mission_member(self) -> Option<MissionMember> {
        match self {
            Self::Character(key) => Some(MissionMember::Character(key)),
            Self::SpecialForce(key) => Some(MissionMember::SpecialForce(key)),
            Self::Fleet(_)
            | Self::Troop(_)
            | Self::Ship { .. }
            | Self::SystemIcon { .. }
            | Self::Producer { .. }
            | Self::Fighter { .. }
            | Self::PlanetFighter { .. }
            | Self::DefenseFacility(_) => None,
        }
    }

    /// The object's Status window (`FUN_0042a440`). A sector window's icon
    /// stands for its system, which has none.
    #[must_use]
    pub const fn status_object(self) -> Option<StatusObject> {
        match self {
            Self::Character(key) => Some(StatusObject::Character(key)),
            Self::SpecialForce(key) => Some(StatusObject::SpecialForce(key)),
            Self::Fleet(key) => Some(StatusObject::Fleet(key)),
            Self::Troop(key) => Some(StatusObject::Troop(key)),
            Self::Ship { fleet, index, .. } => Some(StatusObject::Ship { fleet, index }),
            Self::Producer { system, area } => Some(StatusObject::Producer { system, area }),
            Self::Fighter { fleet, index } => Some(StatusObject::Fighter { fleet, index }),
            Self::PlanetFighter { system, class } => {
                Some(StatusObject::PlanetFighter { system, class })
            }
            Self::DefenseFacility(key) => Some(StatusObject::DefenseFacility(key)),
            Self::SystemIcon { .. } => None,
        }
    }
}

/// One STRATEGY menu record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectMenuItem {
    /// The order kind, which names the record.
    pub kind: u16,
    pub command: ObjectMenuCommand,
    /// Record word 2.
    pub sort_key: u16,
    /// Record word 5.
    pub label_string_id: u16,
    pub label: &'static str,
    /// Record word 4: the item opens a submenu.
    pub submenu: bool,
}

const fn item(
    kind: u16,
    command: ObjectMenuCommand,
    sort_key: u16,
    label_string_id: u16,
    label: &'static str,
    submenu: bool,
) -> ObjectMenuItem {
    ObjectMenuItem {
        kind,
        command,
        sort_key,
        label_string_id,
        label,
        submenu,
    }
}

const MOVE: ObjectMenuItem = item(0x201, ObjectMenuCommand::Move, 10, 12312, "Move", false);
const CONFIRMED_MOVE: ObjectMenuItem = item(
    0x202,
    ObjectMenuCommand::ConfirmedMove,
    12,
    12311,
    "Confirmed Move",
    false,
);
const MISSION: ObjectMenuItem = item(
    0x240,
    ObjectMenuCommand::Mission,
    300,
    12320,
    "Mission",
    false,
);
/// The parent of None, Admiral, General and Commander (`0x260..0x263`).
const COMMAND: ObjectMenuItem = item(
    0x160,
    ObjectMenuCommand::Command,
    400,
    12352,
    "Command",
    true,
);
const ENCYCLOPEDIA: ObjectMenuItem = item(
    0x100,
    ObjectMenuCommand::Encyclopedia,
    1000,
    12292,
    "Encyclopedia",
    false,
);
const STATUS: ObjectMenuItem = item(
    0x103,
    ObjectMenuCommand::Status,
    1001,
    12293,
    "Status",
    false,
);
const RETIRE: ObjectMenuItem = item(
    0x242,
    ObjectMenuCommand::Retire,
    2002,
    12336,
    "Retire",
    false,
);

/// The parent of the four bombardment targets (`0x220..0x223`).
const BOMBARDMENT: ObjectMenuItem = item(
    0x120,
    ObjectMenuCommand::Bombardment,
    200,
    12313,
    "Planetary Bombardment",
    true,
);
/// The bombardment targets under [`BOMBARDMENT`]: STRATEGY records
/// `0x220..0x223` (word 1 the parent `0x120`; `move-order.md`).
const BOMBARDMENT_TARGETS: [ObjectMenuItem; 4] = [
    item(
        0x220,
        ObjectMenuCommand::BombardmentTarget(0x220),
        202,
        12314,
        "Target Military Facilities",
        false,
    ),
    item(
        0x221,
        ObjectMenuCommand::BombardmentTarget(0x221),
        204,
        12315,
        "Target Civilian Facilities",
        false,
    ),
    item(
        0x222,
        ObjectMenuCommand::BombardmentTarget(0x222),
        206,
        12316,
        "General Bombardment",
        false,
    ),
    item(
        0x223,
        ObjectMenuCommand::BombardmentTarget(0x223),
        208,
        12317,
        "Destroy System",
        false,
    ),
];
/// The Command orders under [`COMMAND`]: STRATEGY records `0x260..0x263`
/// (word 1 the parent `0x160`, word 2 the sort key, word 5 the TEXTSTRA
/// string, word 10 the bitmap 11902; `character-command.md`).
const COMMAND_KINDS: [ObjectMenuItem; 4] = [
    item(
        0x260,
        ObjectMenuCommand::GiveCommand(0x260),
        402,
        12354,
        "None",
        false,
    ),
    item(
        0x261,
        ObjectMenuCommand::GiveCommand(0x261),
        409,
        12356,
        "Commander",
        false,
    ),
    item(
        0x262,
        ObjectMenuCommand::GiveCommand(0x262),
        406,
        12358,
        "Admiral",
        false,
    ),
    item(
        0x263,
        ObjectMenuCommand::GiveCommand(0x263),
        408,
        12360,
        "General",
        false,
    ),
];

const ASSAULT: ObjectMenuItem = item(
    0x234,
    ObjectMenuCommand::Assault,
    220,
    12318,
    "Planetary Assault",
    false,
);
const RENAME: ObjectMenuItem = item(
    0x203,
    ObjectMenuCommand::Rename,
    500,
    12291,
    "Rename",
    false,
);
const SCRAP: ObjectMenuItem = item(0x200, ObjectMenuCommand::Scrap, 2000, 12295, "Scrap", false);
/// STRATEGY.DLL record 624 (`ghidra/notes/fleet-join-split.md`).
const CREATE_FLEET: ObjectMenuItem = item(
    0x270,
    ObjectMenuCommand::CreateFleet,
    50,
    12319,
    "Create Fleet",
    false,
);
/// STRATEGY records 532, 534 and 592.
const DESTINATION: ObjectMenuItem = item(
    0x214,
    ObjectMenuCommand::Destination,
    120,
    12290,
    "Destination",
    false,
);
const RESERVED: ObjectMenuItem = item(
    0x216,
    ObjectMenuCommand::Reserved,
    1002,
    12294,
    "Reserved",
    false,
);
const ABORT: ObjectMenuItem = item(0x250, ObjectMenuCommand::Abort, 2003, 12362, "Abort", false);
/// STRATEGY records `0x210..0x216` (`ghidra/notes/manufacturing-build-selection.md`,
/// "Selection and menu"): Build for facilities, ships and troops, Stop, and a
/// manager's Rename.
const BUILD_FACILITIES: ObjectMenuItem =
    item(0x210, ObjectMenuCommand::Build, 100, 12288, "Build", false);
const BUILD_SHIPS: ObjectMenuItem =
    item(0x211, ObjectMenuCommand::Build, 100, 12288, "Build", false);
const BUILD_TROOPS: ObjectMenuItem =
    item(0x212, ObjectMenuCommand::Build, 100, 12288, "Build", false);
const STOP: ObjectMenuItem = item(0x213, ObjectMenuCommand::Stop, 110, 12289, "Stop", false);
const PRODUCER_RENAME: ObjectMenuItem = item(
    0x215,
    ObjectMenuCommand::Rename,
    500,
    12291,
    "Rename",
    false,
);

/// One row of an open object menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectMenuRow {
    pub item: ObjectMenuItem,
    pub enabled: bool,
}

impl ObjectMenuRow {
    #[must_use]
    pub const fn entry(&self) -> GameMenuEntry<'static> {
        GameMenuEntry {
            label: self.item.label,
            // FUN_00442590 flags bit 1: checked when the order's `+0x14`
            // says so. No order the port offers is ever checked: the
            // menu's fresh order (FUN_0051d990) leaves its `+0x40` list
            // empty, so FUN_0051fd30 (and the Command orders' FUN_00580d80
            // over it) answers 0.
            icon: None,
            enabled: self.enabled,
            submenu: self.item.submenu,
        }
    }
}

/// The rows the menu shows for `selection`, in sort-key order.
///
/// A character's class offers Move, Confirmed Move, Retire and Mission
/// (`FUN_00536bc0`) and the Command ranks (`FUN_004f2400`); a special
/// force's only the first four (`FUN_00503fa0`). A fleet's class offers Move,
/// Confirmed Move, the bombardments, Assault, Rename and Scrap
/// (`FUN_004ff8e0`); the bombardment targets sit under their submenu parent.
/// A regiment's offers Move, Confirmed Move and Scrap (`FUN_00504b30`),
/// after a base list (`FUN_00558380`) that is untraced. A capital ship's
/// offers Move, Confirmed Move and Scrap (`FUN_00557ce0`), then Rename and
/// Create Fleet (`FUN_00502bd0`).
/// Kinds `0x204`, `0x241` and `0x268` are offered too but have no STRATEGY
/// record, so they never show.
/// A sector window's icon offers what its system's class gives the icon's
/// kind (`FUN_00507290` → `FUN_0050f2e0`): facilities Destination, Reserved
/// and Scrap; defenses Move, Confirmed Move and Scrap; fleets Move, Confirmed
/// Move, the bombardments, Assault and Scrap; missions Abort. Its object is a
/// system, so Status stays disabled. The fleet icon's Move and Confirmed Move
/// follow `gates.fleet_move` for the system's fleets of the player's side.
/// port: the icons' other orders stay disabled until their system-wide forms
/// (`FUN_00512700`) are ported.
/// A Manufacturing window band offers its manager class's orders
/// (`FUN_0052ae30`, `FUN_0055b580`, `FUN_0055b900`, `FUN_0055bd30`): Build,
/// Stop, Destination and Reserved, and for ships Rename. Build, Stop and
/// Destination follow `gates.build`, `gates.stop` and `gates.destination`;
/// port: Rename and Reserved stay disabled until the band's in-place rename
/// and the reserve bit are ported.
/// An empty selection lists only Encyclopedia and Status, both disabled.
///
/// - Mission is enabled when `gates.mission` says so
///   (`MissionState::mission_order_enabled`). port: the global gate
///   `FUN_0051de80` is taken as clear.
/// - A fleet's Move and Confirmed Move are enabled when `gates.fleet_move`
///   says so (`movement::fleet_move_enabled`, each order's `+0x18`).
/// - A regiment's Move is enabled when `gates.troop_move` says so. port:
///   its Confirmed Move stays disabled; loading onto a fleet in the same
///   system needs no transit confirmation.
/// - A squadron's Move is enabled when `gates.squadron_move` says so.
///   port: its Confirmed Move stays disabled, as a regiment's.
/// - A capital ship's Move and Create Fleet are enabled when
///   `gates.ship_move` says so (its fleet is the player's and in orbit).
///   port: its Confirmed Move stays disabled.
/// - A fleet's or fleet icon's Planetary Assault is enabled when
///   `gates.assault` says so (`assault::order_enabled`, `FUN_00581ae0`).
/// - Encyclopedia is enabled for a single selection.
/// - Status is enabled for any single selection that is not a system
///   (`FUN_0051d990`, `ghidra/notes/status-window.md`).
/// - Command, a submenu parent, is enabled (`FUN_00442590`); its rows are
///   [`object_submenu_rows`].
/// - A character's Move is enabled when `gates.character_move` says so.
///   port: its Confirmed Move (`0x202`), a special force's Move and Confirmed
///   Move, Retire and the other fleet orders stay disabled until their
///   windows and orders are ported.
#[must_use]
pub fn object_menu_rows(selection: Option<MenuObject>, gates: OrderGates) -> Vec<ObjectMenuRow> {
    // A fleet icon's Move rows act on the system's fleets of the player's
    // side (FUN_00512700, kind 0x10), under the same rule.
    let fleet = matches!(
        selection,
        Some(
            MenuObject::Fleet(_)
                | MenuObject::SystemIcon {
                    quadrant: Quadrant::Fleets,
                    ..
                }
        )
    );
    let troop = matches!(selection, Some(MenuObject::Troop(_)));
    let squadron = matches!(
        selection,
        Some(MenuObject::Fighter { .. } | MenuObject::PlanetFighter { .. })
    );
    let fleet_entry = matches!(selection, Some(MenuObject::Fleet(_)));
    let character = matches!(selection, Some(MenuObject::Character(_)));
    let ship = matches!(selection, Some(MenuObject::Ship { .. }));
    let producer = matches!(selection, Some(MenuObject::Producer { .. }));
    let offered: &[ObjectMenuItem] = match selection {
        Some(MenuObject::Character(_)) => &[MOVE, CONFIRMED_MOVE, RETIRE, MISSION, COMMAND],
        Some(MenuObject::SpecialForce(_)) => &[MOVE, CONFIRMED_MOVE, RETIRE, MISSION],
        Some(MenuObject::Fleet(_)) => &[MOVE, CONFIRMED_MOVE, BOMBARDMENT, ASSAULT, RENAME, SCRAP],
        Some(MenuObject::Troop(_)) => &[MOVE, CONFIRMED_MOVE, SCRAP],
        Some(MenuObject::Ship { .. }) => &[MOVE, CONFIRMED_MOVE, SCRAP, RENAME, CREATE_FLEET],
        Some(MenuObject::SystemIcon { quadrant, .. }) => match quadrant {
            Quadrant::System => &[SCRAP, DESTINATION, RESERVED],
            Quadrant::Defenses => &[MOVE, CONFIRMED_MOVE, SCRAP],
            Quadrant::Fleets => &[MOVE, CONFIRMED_MOVE, SCRAP, BOMBARDMENT, ASSAULT],
            Quadrant::Missions => &[ABORT],
        },
        Some(MenuObject::Producer { area, .. }) => match area {
            ProductionArea::Shipyard => {
                &[BUILD_SHIPS, STOP, DESTINATION, PRODUCER_RENAME, RESERVED]
            }
            ProductionArea::TrainingFacility => &[BUILD_TROOPS, STOP, DESTINATION, RESERVED],
            ProductionArea::ConstructionYard => &[BUILD_FACILITIES, STOP, DESTINATION, RESERVED],
        },
        // FUN_00502fe0 -> FUN_00557ce0: 0x201, 0x202, 0x204 (no STRATEGY
        // record) and 0x200.
        Some(MenuObject::Fighter { .. } | MenuObject::PlanetFighter { .. }) => {
            &[MOVE, CONFIRMED_MOVE, SCRAP]
        }
        // port: a defense facility's class order list (vtable `+0x3c`) is
        // untraced; FUN_0051d990's Encyclopedia and Status are always added.
        Some(MenuObject::DefenseFacility(_)) | None => &[],
    };
    let mut rows: Vec<ObjectMenuRow> = offered
        .iter()
        .chain(&[ENCYCLOPEDIA, STATUS])
        .map(|&item| ObjectMenuRow {
            item,
            enabled: match item.command {
                ObjectMenuCommand::Mission => gates.mission,
                ObjectMenuCommand::Move => {
                    (character && gates.character_move)
                        || (fleet && gates.fleet_move)
                        || (troop && gates.troop_move)
                        || (ship && gates.ship_move)
                        || (squadron && gates.squadron_move)
                }
                ObjectMenuCommand::CreateFleet => ship && gates.ship_move,
                ObjectMenuCommand::Rename => (fleet_entry || ship) && gates.rename,
                ObjectMenuCommand::Destination => gates.destination,
                ObjectMenuCommand::Stop => producer && gates.stop,
                ObjectMenuCommand::Build => producer && gates.build,
                ObjectMenuCommand::ConfirmedMove => fleet && gates.fleet_move,
                // FUN_00442590: a submenu parent (flags 8) is enabled.
                ObjectMenuCommand::Bombardment | ObjectMenuCommand::Command => true,
                ObjectMenuCommand::Assault => gates.assault,
                ObjectMenuCommand::Encyclopedia => selection.is_some(),
                // FUN_0051d990: a single selection that is not a system.
                ObjectMenuCommand::Status => {
                    selection.and_then(MenuObject::status_object).is_some()
                }
                _ => false,
            },
        })
        .collect();
    rows.sort_by_key(|row| row.item.sort_key);
    rows
}

/// The rows of the submenu for `selection`:
///
/// - Planetary Bombardment: a fleet's class lists the four targets
///   (`FUN_004ff8e0`), as does a sector window's fleet icon
///   (`FUN_00507290`); each is enabled by its gate;
/// - Command: a character's class lists None, Commander, Admiral and
///   General (`FUN_004f2400`), in their sort-key order, each enabled by its
///   check (`FUN_004ed750`); none is checked.
///
/// Empty for any other selection.
#[must_use]
pub fn object_submenu_rows(selection: Option<MenuObject>, gates: OrderGates) -> Vec<ObjectMenuRow> {
    if matches!(selection, Some(MenuObject::Character(_))) {
        let mut rows: Vec<ObjectMenuRow> = COMMAND_KINDS
            .iter()
            .zip(gates.command)
            .map(|(&item, enabled)| ObjectMenuRow { item, enabled })
            .collect();
        rows.sort_by_key(|row| row.item.sort_key);
        return rows;
    }
    let fleet = matches!(
        selection,
        Some(
            MenuObject::Fleet(_)
                | MenuObject::SystemIcon {
                    quadrant: Quadrant::Fleets,
                    ..
                }
        )
    );
    if !fleet {
        return Vec::new();
    }
    BOMBARDMENT_TARGETS
        .iter()
        .zip(gates.bombard)
        .map(|(&item, enabled)| ObjectMenuRow {
            item,
            enabled,
        })
        .collect()
}

/// Whether the selection passes each order's own rule, its `+0x18` slot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrderGates {
    pub mission: bool,
    pub fleet_move: bool,
    /// A character's Move (`0x201` -> `0x241`): enabled for any character
    /// that resolves (`FUN_004f4a00` only builds the command); the player's
    /// side reads it ([`rebellion_core::missions::character_side`]).
    pub character_move: bool,
    pub troop_move: bool,
    /// A squadron's Move: the player's, on a planet or aboard a fleet in
    /// orbit ([`rebellion_core::squadrons`]).
    pub squadron_move: bool,
    pub ship_move: bool,
    /// Rename (0x203): the selection is the player's and can take a name
    /// (`FUN_004f6e60` refuses a destroyed object).
    pub rename: bool,
    /// Destination (0x214) on a facility icon: the system has production
    /// areas of the player's (`FUN_00512700` kind 4); on a band, the band's
    /// area is the player's.
    pub destination: bool,
    /// Stop (0x213) on a band: the band's area is the player's and is
    /// building something.
    pub stop: bool,
    /// Build (0x210..0x212) on a band: the band's area is the player's and
    /// lists something to build.
    pub build: bool,
    /// The bombardment targets `0x220..0x223` (each order's `+0x18`,
    /// `FUN_00581e80`; `rebellion_core::bombardment::order_enabled`).
    pub bombard: [bool; 4],
    /// Planetary Assault (`0x234`, its `+0x18` `FUN_00581ae0`;
    /// `rebellion_core::assault::order_enabled`).
    pub assault: bool,
    /// The Command orders `0x260..0x263` on a character (`FUN_004ed750`,
    /// `rebellion_core::character_command::order_allowed`).
    pub command: [bool; 4],
}

/// An open object pop-up menu.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectMenuState {
    selection: Option<MenuObject>,
    rows: Vec<ObjectMenuRow>,
    /// The Planetary Bombardment or Command submenu's rows, if the menu has
    /// one.
    submenu: Vec<ObjectMenuRow>,
    /// The highlight and the submenu's timer (`FUN_004aab50`).
    window: GameMenuState,
    /// The 640 by 480 canvas point of the right-button release.
    point: (f32, f32),
}

impl ObjectMenuState {
    /// Open the menu for `selection` at a canvas `point`.
    #[must_use]
    pub fn new(selection: Option<MenuObject>, gates: OrderGates, point: (i16, i16)) -> Self {
        Self {
            selection,
            rows: object_menu_rows(selection, gates),
            submenu: object_submenu_rows(selection, gates),
            window: GameMenuState::default(),
            point: (f32::from(point.0), f32::from(point.1)),
        }
    }

    /// The submenu row that issues `command`, counted from its top.
    #[must_use]
    pub fn submenu_row_of(&self, command: ObjectMenuCommand) -> Option<usize> {
        self.submenu
            .iter()
            .position(|row| row.item.command == command)
    }

    #[must_use]
    pub fn submenu_row_count(&self) -> usize {
        self.submenu.len()
    }

    /// Whether the submenu is shown.
    #[must_use]
    pub fn submenu_shown(&self) -> bool {
        self.window.submenu_shown()
    }

    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// The row that issues `command`, counted from the top.
    #[must_use]
    pub fn row_of(&self, command: ObjectMenuCommand) -> Option<usize> {
        self.rows.iter().position(|row| row.item.command == command)
    }

    /// Whether the row at `row` is enabled.
    #[must_use]
    pub fn row_enabled(&self, row: usize) -> bool {
        self.rows.get(row).is_some_and(|row| row.enabled)
    }
}

const MENU_ID: &str = "original_object_menu";

/// The open menu's screen rectangle, once egui has laid it out.
#[must_use]
pub fn object_menu_rect(ctx: &egui::Context) -> Option<egui::Rect> {
    ctx.memory(|memory| memory.area_rect(egui::Id::new(MENU_ID)))
}

/// The submenu's last window, shown or not (see
/// [`ObjectMenuState::submenu_shown`]).
#[must_use]
pub fn object_submenu_rect(ctx: &egui::Context) -> Option<egui::Rect> {
    ctx.memory(|memory| memory.area_rect(egui::Id::new(MENU_ID).with("submenu")))
}

/// The galaxy view in canvas coordinates: the menu's owner window
/// (`FUN_004ac5c0` maps the point into it and `FUN_00442380` opens there).
fn galaxy_owner(layout: CockpitLayout) -> CockpitViewport {
    let scale = layout.scale.max(f32::EPSILON);
    CockpitViewport {
        x: (layout.galaxy.x - layout.canvas.x) / scale,
        y: (layout.galaxy.y - layout.canvas.y) / scale,
        width: layout.galaxy.width / scale,
        height: layout.galaxy.height / scale,
    }
}

/// Draw the open menu. Returns the chosen command and the selection it acts
/// on; a choice, a press outside or Escape closes the menu.
pub fn draw_object_menu(
    ctx: &egui::Context,
    menu: &mut Option<ObjectMenuState>,
    cache: &mut BmpCache,
    layout: CockpitLayout,
    faction: CockpitFaction,
    input_enabled: bool,
) -> Option<(ObjectMenuCommand, Option<MenuObject>)> {
    let open = menu.as_mut()?;
    let entries: Vec<GameMenuEntry<'static>> = open.rows.iter().map(ObjectMenuRow::entry).collect();
    let children: Vec<GameMenuEntry<'static>> =
        open.submenu.iter().map(ObjectMenuRow::entry).collect();
    let parent = open.rows.iter().position(|row| {
        matches!(
            row.item.command,
            ObjectMenuCommand::Bombardment | ObjectMenuCommand::Command
        )
    });
    let submenu = parent
        .filter(|_| !children.is_empty())
        .map(|parent| GameSubmenu {
            parent,
            entries: &children,
        });
    let placement = GameMenuPlacement {
        owner: galaxy_owner(layout),
        point: open.point,
    };
    match draw_game_menu_with_submenu(
        ctx,
        egui::Id::new(MENU_ID),
        cache,
        layout,
        faction,
        placement,
        &entries,
        submenu,
        &mut open.window,
        input_enabled,
    ) {
        GameMenuResponse::Open => None,
        GameMenuResponse::Dismissed => {
            *menu = None;
            None
        }
        GameMenuResponse::Chosen(index) => {
            let chosen = (open.rows[index].item.command, open.selection);
            *menu = None;
            Some(chosen)
        }
        GameMenuResponse::ChosenChild(index) => {
            let chosen = (open.submenu[index].item.command, open.selection);
            *menu = None;
            Some(chosen)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rebellion_core::ids::SystemKey;

    const MISSION_GATE: OrderGates = OrderGates {
        mission: true,
        fleet_move: false,
        character_move: false,
        troop_move: false,
        squadron_move: false,
        ship_move: false,
        rename: false,
        destination: false,
        stop: false,
        build: false,
        bombard: [false; 4],
        assault: false,
        command: [false; 4],
    };

    fn labels(rows: &[ObjectMenuRow]) -> Vec<&'static str> {
        rows.iter().map(|row| row.item.label).collect()
    }

    fn enabled(rows: &[ObjectMenuRow]) -> Vec<&'static str> {
        rows.iter()
            .filter(|row| row.enabled)
            .map(|row| row.item.label)
            .collect()
    }

    #[test]
    fn a_characters_menu_matches_the_manual_and_the_strategy_records() {
        // Manual p. 99 and Fig. 3.45; STRATEGY RT_RCDATA words 2 and 5;
        // FUN_004f2400 over FUN_00536bc0.
        let rows = object_menu_rows(
            Some(MenuObject::Character(CharacterKey::default())),
            MISSION_GATE,
        );
        assert_eq!(
            labels(&rows),
            [
                "Move",
                "Confirmed Move",
                "Mission",
                "Command",
                "Encyclopedia",
                "Status",
                "Retire"
            ]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
            [0x201, 0x202, 0x240, 0x160, 0x100, 0x103, 0x242]
        );
        assert_eq!(
            rows.iter()
                .map(|row| row.item.label_string_id)
                .collect::<Vec<_>>(),
            [12312, 12311, 12320, 12352, 12292, 12293, 12336]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.sort_key).collect::<Vec<_>>(),
            [10, 12, 300, 400, 1000, 1001, 2002]
        );
        // Record 0x160's word 4 marks the Command submenu parent.
        assert_eq!(
            rows.iter()
                .filter(|row| row.item.submenu)
                .map(|row| row.item.label)
                .collect::<Vec<_>>(),
            ["Command"]
        );
    }

    #[test]
    fn a_characters_command_submenu_lists_none_admiral_general_commander() {
        // STRATEGY RT_RCDATA 0x260..0x263: parent 0x160, sort 402, 409, 406,
        // 408, TEXTSTRA 12354, 12356, 12358, 12360, bitmap 11902; each row
        // enabled by FUN_004ed750, none checked (FUN_00580d80 ->
        // FUN_0051fd30 over the fresh order's empty +0x40 list).
        let gates = OrderGates {
            command: [false, true, true, false],
            ..OrderGates::default()
        };
        let rows = object_submenu_rows(Some(MenuObject::Character(CharacterKey::default())), gates);
        assert_eq!(labels(&rows), ["None", "Admiral", "General", "Commander"]);
        assert_eq!(
            rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
            [0x260, 0x262, 0x263, 0x261]
        );
        assert_eq!(
            rows.iter()
                .map(|row| row.item.label_string_id)
                .collect::<Vec<_>>(),
            [12354, 12358, 12360, 12356]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.sort_key).collect::<Vec<_>>(),
            [402, 406, 408, 409]
        );
        assert_eq!(enabled(&rows), ["Admiral", "Commander"]);
        assert_eq!(
            rows.iter().map(|row| row.entry().icon).collect::<Vec<_>>(),
            [None; 4]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.command).collect::<Vec<_>>(),
            [0x260, 0x262, 0x263, 0x261].map(ObjectMenuCommand::GiveCommand)
        );
        // A special force or a fleet has no Command rows.
        assert!(object_submenu_rows(
            Some(MenuObject::SpecialForce(SpecialForceKey::default())),
            gates
        )
        .is_empty());
        assert!(
            object_submenu_rows(Some(MenuObject::Fleet(FleetKey::default())), gates)
                .iter()
                .all(|row| !matches!(row.item.command, ObjectMenuCommand::GiveCommand(_)))
        );
    }

    #[test]
    fn a_special_forces_menu_has_no_command_submenu() {
        // FUN_00503fa0 copies only the unit list of FUN_00536bc0.
        let rows = object_menu_rows(
            Some(MenuObject::SpecialForce(SpecialForceKey::default())),
            MISSION_GATE,
        );
        assert_eq!(
            labels(&rows),
            [
                "Move",
                "Confirmed Move",
                "Mission",
                "Encyclopedia",
                "Status",
                "Retire"
            ]
        );
    }

    #[test]
    fn a_fleets_menu_matches_the_manual_and_the_strategy_records() {
        // Manual Fig. 3.64; FUN_004ff8e0's list; STRATEGY RT_RCDATA words 2,
        // 4 and 5 (ghidra/notes/move-order.md, "The Fleet menu").
        let rows = object_menu_rows(Some(MenuObject::Fleet(FleetKey::default())), MISSION_GATE);
        assert_eq!(
            labels(&rows),
            [
                "Move",
                "Confirmed Move",
                "Planetary Bombardment",
                "Planetary Assault",
                "Rename",
                "Encyclopedia",
                "Status",
                "Scrap"
            ]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
            [0x201, 0x202, 0x120, 0x234, 0x203, 0x100, 0x103, 0x200]
        );
        assert_eq!(
            rows.iter()
                .map(|row| row.item.label_string_id)
                .collect::<Vec<_>>(),
            [12312, 12311, 12313, 12318, 12291, 12292, 12293, 12295]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.sort_key).collect::<Vec<_>>(),
            [10, 12, 200, 220, 500, 1000, 1001, 2000]
        );
        assert_eq!(
            rows.iter()
                .filter(|row| row.item.submenu)
                .map(|row| row.item.label)
                .collect::<Vec<_>>(),
            ["Planetary Bombardment"]
        );
        // A fleet is no mission member, so Mission's gate never reaches it.
        // The bombardment parent is enabled (FUN_00442590 flags 8).
        assert_eq!(
            enabled(&rows),
            ["Planetary Bombardment", "Encyclopedia", "Status"]
        );
    }

    #[test]
    fn a_fleets_move_rows_follow_its_move_rule() {
        // FUN_0053c100 -> FUN_004fdc70: each move order's +0x18 enables its
        // row; the two rows share one rule.
        let fleet = Some(MenuObject::Fleet(FleetKey::default()));
        let gates = OrderGates {
            mission: false,
            fleet_move: true,
            character_move: false,
            troop_move: true,
            squadron_move: true,
            ship_move: true,
            rename: false,
            destination: false,
            stop: false,
            build: false,
            bombard: [false; 4],
            assault: false,
            command: [false; 4],
        };
        assert_eq!(
            enabled(&object_menu_rows(fleet, gates)),
            [
                "Move",
                "Confirmed Move",
                "Planetary Bombardment",
                "Encyclopedia",
                "Status"
            ]
        );
        assert_eq!(
            enabled(&object_menu_rows(fleet, OrderGates::default())),
            ["Planetary Bombardment", "Encyclopedia", "Status"]
        );
        // A character's Move follows its own gate; Command is a submenu
        // parent, enabled (FUN_00442590).
        let character = Some(MenuObject::Character(CharacterKey::default()));
        assert_eq!(
            enabled(&object_menu_rows(character, gates)),
            ["Command", "Encyclopedia", "Status"]
        );
    }

    #[test]
    fn a_characters_move_row_follows_its_move_rule() {
        // 0x201 -> 0x241 (FUN_0053c810): the row is enabled when the
        // character resolves (FUN_004f4a00 only builds the command). port:
        // its Confirmed Move (0x202) stays disabled.
        let gates = OrderGates {
            character_move: true,
            ..OrderGates::default()
        };
        let character = Some(MenuObject::Character(CharacterKey::default()));
        assert_eq!(
            enabled(&object_menu_rows(character, gates)),
            ["Move", "Command", "Encyclopedia", "Status"]
        );
        let unit = Some(MenuObject::SpecialForce(SpecialForceKey::default()));
        assert!(!enabled(&object_menu_rows(unit, gates)).contains(&"Move"));
    }

    #[test]
    fn rename_is_enabled_for_a_fleet_or_ship_its_rule_allows() {
        // 0x203 (TEXTSTRA 12291) on a fleet's and a ship's menu
        // (FUN_00502bd0); a character's menu does not offer it.
        let gates = OrderGates {
            rename: true,
            ..OrderGates::default()
        };
        let fleet = Some(MenuObject::Fleet(FleetKey::default()));
        let ship = Some(MenuObject::Ship {
            fleet: FleetKey::default(),
            index: 0,
            roster: 0,
        });
        for selection in [fleet, ship] {
            assert!(enabled(&object_menu_rows(selection, gates)).contains(&"Rename"));
            assert!(
                !enabled(&object_menu_rows(selection, OrderGates::default())).contains(&"Rename")
            );
        }
        let character = Some(MenuObject::Character(CharacterKey::default()));
        assert!(!enabled(&object_menu_rows(character, gates)).contains(&"Rename"));
    }

    #[test]
    fn a_facility_icons_destination_follows_its_rule() {
        // sector-icon-menus.md: Destination (0x214, TEXTSTRA 12290) on the
        // facility icon acts on the system's production areas.
        let gates = OrderGates {
            destination: true,
            ..OrderGates::default()
        };
        assert!(enabled(&object_menu_rows(icon(Quadrant::System), gates)).contains(&"Destination"));
        assert!(!enabled(&object_menu_rows(
            icon(Quadrant::System),
            OrderGates::default()
        ))
        .contains(&"Destination"));
    }

    #[test]
    fn a_bands_build_stop_and_destination_follow_their_rules() {
        // manufacturing-build-selection.md, "Selection and menu": a band's
        // manager offers Build (0x210..0x212), Stop (0x213), Destination
        // (0x214) and Reserved (0x216), and for ships Rename (0x215).
        let band = |area| {
            Some(MenuObject::Producer {
                system: SystemKey::default(),
                area,
            })
        };
        let gates = OrderGates {
            destination: true,
            stop: true,
            build: true,
            ..OrderGates::default()
        };
        for area in ProductionArea::ALL {
            assert_eq!(
                enabled(&object_menu_rows(band(area), gates)),
                ["Build", "Stop", "Destination", "Encyclopedia", "Status"]
            );
            assert_eq!(
                enabled(&object_menu_rows(band(area), OrderGates::default())),
                ["Encyclopedia", "Status"]
            );
        }
        // Build and Stop belong to the bands alone.
        let icon_rows = enabled(&object_menu_rows(icon(Quadrant::System), gates));
        assert!(!icon_rows.contains(&"Stop") && !icon_rows.contains(&"Build"));
    }

    fn icon(quadrant: Quadrant) -> Option<MenuObject> {
        Some(MenuObject::SystemIcon {
            system: SystemKey::default(),
            quadrant,
        })
    }

    #[test]
    fn each_sector_icons_menu_lists_its_kinds_orders() {
        // FUN_0051d990 asks the system (vtable 0x0065e640, +0x3c
        // FUN_00507290) for the orders of the selection's kind;
        // FUN_0050f2e0 gives kind 4 0x200, 0x214, 0x216; kind 8 0x201, 0x202,
        // 0x200; kind 0x10 0x201, 0x202, 0x200, 0x220..0x223, 0x234; kind
        // 0x40 0x250. STRATEGY RT_RCDATA words 2 and 5 sort and name them;
        // manual Fig. 3.50 shows the mission icon's Encyclopedia, Status,
        // Abort.
        let kinds = |quadrant| -> Vec<u16> {
            object_menu_rows(icon(quadrant), OrderGates::default())
                .iter()
                .map(|row| row.item.kind)
                .collect()
        };
        assert_eq!(
            kinds(Quadrant::Fleets),
            [0x201, 0x202, 0x120, 0x234, 0x100, 0x103, 0x200]
        );
        assert_eq!(
            kinds(Quadrant::Defenses),
            [0x201, 0x202, 0x100, 0x103, 0x200]
        );
        assert_eq!(kinds(Quadrant::System), [0x214, 0x100, 0x103, 0x216, 0x200]);
        assert_eq!(kinds(Quadrant::Missions), [0x100, 0x103, 0x250]);

        let rows = object_menu_rows(icon(Quadrant::System), OrderGates::default());
        assert_eq!(
            labels(&rows),
            ["Destination", "Encyclopedia", "Status", "Reserved", "Scrap"]
        );
        assert_eq!(
            rows.iter()
                .map(|row| (row.item.sort_key, row.item.label_string_id))
                .collect::<Vec<_>>(),
            [
                (120, 12290),
                (1000, 12292),
                (1001, 12293),
                (1002, 12294),
                (2000, 12295)
            ]
        );
        let abort = object_menu_rows(icon(Quadrant::Missions), OrderGates::default())[2].item;
        assert_eq!(
            (abort.label, abort.sort_key, abort.label_string_id),
            ("Abort", 2003, 12362)
        );
    }

    #[test]
    fn a_sector_icons_status_stays_disabled_because_its_object_is_a_system() {
        // FUN_0051d990: Encyclopedia is enabled for a single selection;
        // Status only when that object is not a system (0x90..0x97). An
        // icon's selection is its system (item +0x68, FUN_004f5b10).
        for quadrant in Quadrant::ALL {
            // The fleet icon's bombardment parent is enabled (flags 8).
            let parent = (quadrant == Quadrant::Fleets).then_some("Planetary Bombardment");
            let expected: Vec<&str> = parent.into_iter().chain(["Encyclopedia"]).collect();
            assert_eq!(
                enabled(&object_menu_rows(icon(quadrant), OrderGates::default())),
                expected,
                "{quadrant:?}"
            );
        }
    }

    #[test]
    fn a_fleet_icons_move_rows_follow_its_fleets_move_rule() {
        // FUN_0053c100 -> FUN_0053c4b0: the team is the system's fleets of
        // the order's side (FUN_00512700, kind 0x10); the other icons' Move
        // rows act on members whose moves are not ported.
        let every_gate = OrderGates {
            mission: true,
            fleet_move: true,
            character_move: true,
            troop_move: true,
            squadron_move: true,
            ship_move: true,
            rename: false,
            destination: false,
            stop: true,
            build: true,
            bombard: [false; 4],
            assault: false,
            command: [false; 4],
        };
        assert_eq!(
            enabled(&object_menu_rows(icon(Quadrant::Fleets), every_gate)),
            [
                "Move",
                "Confirmed Move",
                "Planetary Bombardment",
                "Encyclopedia"
            ]
        );
        for quadrant in [Quadrant::System, Quadrant::Defenses, Quadrant::Missions] {
            assert_eq!(
                enabled(&object_menu_rows(icon(quadrant), every_gate)),
                ["Encyclopedia"],
                "{quadrant:?}"
            );
        }
    }

    #[test]
    fn a_capital_ships_menu_offers_create_fleet_after_the_moves() {
        // FUN_00502bd0 (0x203, 0x270 after FUN_00557ce0's 0x201, 0x202,
        // 0x204, 0x200); sorted by STRATEGY record word 2 (record 624 is 50).
        let ship = Some(MenuObject::Ship {
            fleet: FleetKey::default(),
            index: 0,
            roster: 0,
        });
        let rows = object_menu_rows(ship, MISSION_GATE);
        assert_eq!(
            labels(&rows),
            [
                "Move",
                "Confirmed Move",
                "Create Fleet",
                "Rename",
                "Encyclopedia",
                "Status",
                "Scrap"
            ]
        );
        let create = rows[2].item;
        assert_eq!(
            (create.kind, create.sort_key, create.label_string_id),
            (0x270, 50, 12319)
        );
    }

    #[test]
    fn a_capital_ships_move_and_create_fleet_follow_its_fleets_move_rule() {
        // port: its fleet's side and orbit (FUN_004f9860); Confirmed Move
        // stays disabled.
        let ship = Some(MenuObject::Ship {
            fleet: FleetKey::default(),
            index: 1,
            roster: 0,
        });
        let gates = OrderGates {
            ship_move: true,
            ..OrderGates::default()
        };
        assert_eq!(
            enabled(&object_menu_rows(ship, gates)),
            ["Move", "Create Fleet", "Encyclopedia", "Status"]
        );
        assert_eq!(
            enabled(&object_menu_rows(ship, OrderGates::default())),
            ["Encyclopedia", "Status"]
        );
        let fleet = Some(MenuObject::Fleet(FleetKey::default()));
        assert_eq!(
            enabled(&object_menu_rows(fleet, gates)),
            ["Planetary Bombardment", "Encyclopedia", "Status"]
        );
    }

    #[test]
    fn a_regiments_menu_offers_move_confirmed_move_and_scrap() {
        // FUN_00504b30: 0x201, 0x202, 0x204 (no STRATEGY record) and 0x200.
        let rows = object_menu_rows(Some(MenuObject::Troop(TroopKey::default())), MISSION_GATE);
        assert_eq!(
            labels(&rows),
            ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
            [0x201, 0x202, 0x100, 0x103, 0x200]
        );
    }

    #[test]
    fn a_regiments_move_follows_its_gate_and_confirmed_move_stays_disabled() {
        // port: only Move is ported for a regiment (loading onto a fleet).
        let troop = Some(MenuObject::Troop(TroopKey::default()));
        let gates = OrderGates {
            troop_move: true,
            squadron_move: true,
            ..OrderGates::default()
        };
        assert_eq!(
            enabled(&object_menu_rows(troop, gates)),
            ["Move", "Encyclopedia", "Status"]
        );
        assert_eq!(
            enabled(&object_menu_rows(troop, OrderGates::default())),
            ["Encyclopedia", "Status"]
        );
        // The fleet's gate does not reach a regiment, nor the regiment's a
        // fleet.
        let fleet_gate = OrderGates {
            fleet_move: true,
            ..OrderGates::default()
        };
        assert_eq!(
            enabled(&object_menu_rows(troop, fleet_gate)),
            ["Encyclopedia", "Status"]
        );
        assert_eq!(
            enabled(&object_menu_rows(
                Some(MenuObject::Fleet(FleetKey::default())),
                gates
            )),
            ["Planetary Bombardment", "Encyclopedia", "Status"]
        );
    }

    #[test]
    fn only_characters_and_special_forces_are_mission_members() {
        let character = CharacterKey::default();
        let unit = SpecialForceKey::default();
        assert_eq!(
            MenuObject::Character(character).mission_member(),
            Some(MissionMember::Character(character))
        );
        assert_eq!(
            MenuObject::SpecialForce(unit).mission_member(),
            Some(MissionMember::SpecialForce(unit))
        );
        assert_eq!(
            MenuObject::Fleet(FleetKey::default()).mission_member(),
            None
        );
        assert_eq!(
            MenuObject::Troop(TroopKey::default()).mission_member(),
            None
        );
    }

    #[test]
    fn mission_follows_the_order_rule_and_encyclopedia_needs_one_selection() {
        // FUN_0051fe20 enables Mission; FUN_0051d990 enables Encyclopedia
        // for a single selection.
        let character = Some(MenuObject::Character(CharacterKey::default()));
        assert_eq!(
            enabled(&object_menu_rows(character, MISSION_GATE)),
            ["Mission", "Command", "Encyclopedia", "Status"]
        );
        assert_eq!(
            enabled(&object_menu_rows(character, OrderGates::default())),
            ["Command", "Encyclopedia", "Status"]
        );
    }

    #[test]
    fn status_is_enabled_for_any_single_object_that_is_not_a_system() {
        // FUN_0051d990 enables 0x103 for a single selection that is not a
        // system (ghidra/notes/status-window.md); a sector window's icon
        // stands for its system.
        let status = |selection| {
            enabled(&object_menu_rows(Some(selection), OrderGates::default())).contains(&"Status")
        };
        for selection in [
            MenuObject::Character(CharacterKey::default()),
            MenuObject::SpecialForce(SpecialForceKey::default()),
            MenuObject::Fleet(FleetKey::default()),
            MenuObject::Troop(TroopKey::default()),
            MenuObject::Ship {
                fleet: FleetKey::default(),
                index: 0,
                roster: 0,
            },
            MenuObject::Producer {
                system: SystemKey::default(),
                area: ProductionArea::Shipyard,
            },
            MenuObject::Fighter {
                fleet: FleetKey::default(),
                index: 0,
            },
            MenuObject::DefenseFacility(DefenseFacilityKey::default()),
        ] {
            assert!(status(selection), "{selection:?}");
        }
        assert!(!status(MenuObject::SystemIcon {
            system: SystemKey::default(),
            quadrant: Quadrant::Fleets,
        }));
    }

    #[test]
    fn a_squadron_lists_move_confirmed_move_and_scrap() {
        // FUN_00502fe0 -> FUN_00557ce0: a squadron's class lists 0x201,
        // 0x202, 0x204 (no STRATEGY record) and 0x200; FUN_0051d990 adds
        // 0x100 and 0x103.
        let gates = OrderGates {
            squadron_move: true,
            ..MISSION_GATE
        };
        for selection in [
            MenuObject::Fighter {
                fleet: FleetKey::default(),
                index: 1,
            },
            MenuObject::PlanetFighter {
                system: SystemKey::default(),
                class: FighterKey::default(),
            },
        ] {
            let rows = object_menu_rows(Some(selection), gates);
            assert_eq!(
                labels(&rows),
                ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"],
                "{selection:?}"
            );
            assert_eq!(
                enabled(&rows),
                ["Move", "Encyclopedia", "Status"],
                "{selection:?}"
            );
            assert_eq!(
                enabled(&object_menu_rows(Some(selection), MISSION_GATE)),
                ["Encyclopedia", "Status"],
                "{selection:?}"
            );
            assert_eq!(selection.mission_member(), None);
        }
        assert_eq!(
            MenuObject::Fighter {
                fleet: FleetKey::default(),
                index: 1,
            }
            .status_object(),
            Some(StatusObject::Fighter {
                fleet: FleetKey::default(),
                index: 1,
            })
        );
        assert_eq!(
            MenuObject::PlanetFighter {
                system: SystemKey::default(),
                class: FighterKey::default(),
            }
            .status_object(),
            Some(StatusObject::PlanetFighter {
                system: SystemKey::default(),
                class: FighterKey::default(),
            })
        );
    }

    #[test]
    fn a_defense_facility_lists_only_encyclopedia_and_status() {
        // FUN_0051d990 adds 0x100 and 0x103 to every menu; port: the
        // facility class's own order list is untraced.
        let selection = MenuObject::DefenseFacility(DefenseFacilityKey::default());
        let rows = object_menu_rows(Some(selection), MISSION_GATE);
        assert_eq!(labels(&rows), ["Encyclopedia", "Status"]);
        assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
        assert_eq!(selection.mission_member(), None);
        assert_eq!(
            selection.status_object(),
            Some(StatusObject::DefenseFacility(DefenseFacilityKey::default()))
        );
    }

    #[test]
    fn an_empty_selection_lists_encyclopedia_and_status_disabled() {
        // FUN_0051d990 always adds 0x100 and 0x103; neither is enabled
        // without exactly one selected object.
        let rows = object_menu_rows(None, MISSION_GATE);
        assert_eq!(labels(&rows), ["Encyclopedia", "Status"]);
        assert!(enabled(&rows).is_empty());
    }

    #[test]
    fn a_row_becomes_a_menu_entry_without_an_icon() {
        // The character records carry no icon (words 7 and 8 are zero).
        let rows = object_menu_rows(
            Some(MenuObject::Character(CharacterKey::default())),
            MISSION_GATE,
        );
        let command = rows.iter().find(|row| row.item.label == "Command").unwrap();
        assert_eq!(
            command.entry(),
            GameMenuEntry {
                label: "Command",
                icon: None,
                enabled: true,
                submenu: true,
            }
        );
        let mission = rows.iter().find(|row| row.item.label == "Mission").unwrap();
        assert!(mission.entry().enabled && !mission.entry().submenu);
    }

    fn galaxy_layout() -> CockpitLayout {
        crate::cockpit::CockpitState::new(CockpitFaction::Alliance).layout_for(1280.0, 960.0)
    }

    /// Draw `menu` for two layout frames, then click the centre of row
    /// `row`. Returns the choice and the menu's rectangle.
    fn click_row(
        menu: &mut Option<ObjectMenuState>,
        row: usize,
    ) -> (Option<(ObjectMenuCommand, Option<MenuObject>)>, egui::Rect) {
        let layout = galaxy_layout();
        let ctx = egui::Context::default();
        let mut cache = BmpCache::new();
        let count = menu.as_ref().map_or(0, |open| open.rows.len());
        let mut chosen = None;
        let mut rect = egui::Rect::NOTHING;
        for frame in 0..4 {
            let mut events = Vec::new();
            if frame >= 2 {
                rect = object_menu_rect(&ctx).expect("the menu is laid out");
                let height = (rect.height() - 2.0 * layout.scale) / count as f32;
                let pos = egui::pos2(
                    rect.center().x,
                    rect.min.y + 2.0 * layout.scale + height * (row as f32 + 0.5),
                );
                events.push(egui::Event::PointerMoved(pos));
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: frame == 2,
                    modifiers: egui::Modifiers::default(),
                });
            }
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 960.0),
                )),
                events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                if let Some(choice) = draw_object_menu(
                    ctx,
                    menu,
                    &mut cache,
                    layout,
                    CockpitFaction::Alliance,
                    true,
                ) {
                    chosen = Some(choice);
                }
            });
        }
        (chosen, rect)
    }

    #[test]
    fn a_characters_menu_lists_mission_third_of_seven_rows() {
        // STRATEGY RT_RCDATA sort keys: Move 10, Confirmed Move 12, Mission
        // 300, Command, Encyclopedia 1000, Status 1001, Retire 2002.
        let agent = Some(MenuObject::Character(CharacterKey::default()));
        let menu = ObjectMenuState::new(agent, MISSION_GATE, (100, 100));
        assert_eq!(menu.row_count(), 7);
        assert_eq!(menu.row_of(ObjectMenuCommand::Mission), Some(2));
        assert_eq!(menu.row_of(ObjectMenuCommand::Encyclopedia), Some(4));

        // FUN_0051d990 lists only Encyclopedia and Status for no selection.
        let empty = ObjectMenuState::new(None, MISSION_GATE, (100, 100));
        assert_eq!(empty.row_of(ObjectMenuCommand::Mission), None);
    }

    #[test]
    fn choosing_mission_returns_it_with_the_selection_and_closes_the_menu() {
        // FUN_004424c0 reports the kind to the owner's vtable +0x20
        // (FUN_004ac730), which acts on the copied selection +0x11c.
        let agent = Some(MenuObject::Character(CharacterKey::default()));
        let mut menu = Some(ObjectMenuState::new(agent, MISSION_GATE, (100, 100)));
        let (chosen, _) = click_row(&mut menu, 2);
        assert_eq!(chosen, Some((ObjectMenuCommand::Mission, agent)));
        assert_eq!(menu, None);
    }

    #[test]
    fn a_disabled_row_keeps_the_menu_open() {
        let agent = Some(MenuObject::Character(CharacterKey::default()));
        let mut menu = Some(ObjectMenuState::new(agent, MISSION_GATE, (100, 100)));
        let (chosen, _) = click_row(&mut menu, 0);
        assert_eq!(chosen, None);
        assert!(menu.is_some());
    }

    #[test]
    fn the_menu_opens_in_the_galaxy_view_and_flips_at_its_edges() {
        // FUN_00442860 flips at the owner's edges; the owner is the galaxy
        // view (55, 40) to (540, 390) for the Alliance (FUN_00421c70).
        let mut offset = galaxy_layout();
        offset.canvas.x += 10.0;
        offset.canvas.y += 20.0;
        offset.galaxy.x += 10.0;
        offset.galaxy.y += 20.0;
        assert_eq!(
            galaxy_owner(offset),
            CockpitViewport {
                x: 55.0,
                y: 40.0,
                width: 485.0,
                height: 350.0,
            }
        );
        let layout = galaxy_layout();
        assert_eq!(
            galaxy_owner(layout),
            CockpitViewport {
                x: 55.0,
                y: 40.0,
                width: 485.0,
                height: 350.0,
            }
        );
        let agent = Some(MenuObject::Character(CharacterKey::default()));
        let mut inside = Some(ObjectMenuState::new(agent, MISSION_GATE, (100, 100)));
        let (_, rect) = click_row(&mut inside, 0);
        assert_eq!(
            rect.min,
            egui::pos2(
                layout.canvas.x + 100.0 * layout.scale,
                layout.canvas.y + 100.0 * layout.scale
            )
        );
        // Near the galaxy view's bottom right, inside the canvas: it flips.
        let mut corner = Some(ObjectMenuState::new(agent, MISSION_GATE, (530, 380)));
        let (_, rect) = click_row(&mut corner, 0);
        assert_eq!(
            rect.max,
            egui::pos2(
                layout.canvas.x + 530.0 * layout.scale,
                layout.canvas.y + 380.0 * layout.scale
            )
        );
    }

    #[test]
    fn planetary_assault_follows_its_gate_on_a_fleet_and_a_fleet_icon() {
        // FUN_004ff8e0 and FUN_00507290 list 0x234 (STRATEGY sort 220,
        // TEXTSTRA 12318); its +0x18 is FUN_00581ae0
        // (assault::order_enabled).
        let assault = |selection, open: bool| {
            let gates = OrderGates {
                assault: open,
                ..OrderGates::default()
            };
            object_menu_rows(selection, gates)
                .into_iter()
                .find(|row| row.item.command == ObjectMenuCommand::Assault)
                .map(|row| (row.item.kind, row.item.sort_key, row.enabled))
        };
        for selection in [
            Some(MenuObject::Fleet(FleetKey::default())),
            icon(Quadrant::Fleets),
        ] {
            assert_eq!(assault(selection, true), Some((0x234, 220, true)));
            assert_eq!(assault(selection, false), Some((0x234, 220, false)));
        }
        // No other selection offers it.
        assert_eq!(assault(icon(Quadrant::System), true), None);
        assert_eq!(assault(None, true), None);
        // An open menu reports the row's state.
        let fleet = Some(MenuObject::Fleet(FleetKey::default()));
        for open in [true, false] {
            let gates = OrderGates {
                assault: open,
                ..OrderGates::default()
            };
            let menu = ObjectMenuState::new(fleet, gates, (0, 0));
            let row = menu.row_of(ObjectMenuCommand::Assault).unwrap();
            assert_eq!(menu.row_enabled(row), open);
        }
        let menu = ObjectMenuState::new(fleet, OrderGates::default(), (0, 0));
        assert!(!menu.row_enabled(menu.row_count()));
    }

    #[test]
    fn a_fleets_bombardment_submenu_lists_the_four_targets_by_their_gates() {
        // FUN_004ff8e0 lists 0x220..0x223; STRATEGY records 0x220..0x223
        // (parent 0x120, sort 202..208, TEXTSTRA 12314..12317); each is
        // enabled by its +0x18 (FUN_00581e80).
        let gates = OrderGates {
            bombard: [true, false, true, false],
            ..OrderGates::default()
        };
        for selection in [
            MenuObject::Fleet(FleetKey::default()),
            icon(Quadrant::Fleets).unwrap(),
        ] {
            let rows = object_submenu_rows(Some(selection), gates);
            assert_eq!(
                labels(&rows),
                [
                    "Target Military Facilities",
                    "Target Civilian Facilities",
                    "General Bombardment",
                    "Destroy System"
                ]
            );
            assert_eq!(
                rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
                [0x220, 0x221, 0x222, 0x223]
            );
            assert_eq!(
                rows.iter().map(|row| row.item.sort_key).collect::<Vec<_>>(),
                [202, 204, 206, 208]
            );
            assert_eq!(
                rows.iter()
                    .map(|row| row.item.label_string_id)
                    .collect::<Vec<_>>(),
                [12314, 12315, 12316, 12317]
            );
            assert_eq!(
                enabled(&rows),
                ["Target Military Facilities", "General Bombardment"]
            );
            assert_eq!(
                rows[3].item.command,
                ObjectMenuCommand::BombardmentTarget(0x223)
            );
        }
        // An open fleet menu finds each target's row; the parent is not in
        // the submenu, and the submenu waits for the highlight.
        let menu =
            ObjectMenuState::new(Some(MenuObject::Fleet(FleetKey::default())), gates, (0, 0));
        assert_eq!(
            menu.submenu_row_of(ObjectMenuCommand::BombardmentTarget(0x222)),
            Some(2)
        );
        assert_eq!(
            menu.submenu_row_of(ObjectMenuCommand::BombardmentTarget(0x220)),
            Some(0)
        );
        assert_eq!(menu.submenu_row_of(ObjectMenuCommand::Bombardment), None);
        assert_eq!(menu.submenu_row_count(), 4);
        assert!(!menu.submenu_shown());
        // Nothing else has the submenu.
        for selection in [
            Some(MenuObject::Troop(TroopKey::default())),
            icon(Quadrant::System),
            None,
        ] {
            assert_eq!(
                labels(&object_submenu_rows(selection, gates)),
                [] as [&str; 0]
            );
        }
    }
}
