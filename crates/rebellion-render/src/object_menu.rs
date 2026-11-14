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
    CharacterKey, DefenseFacilityKey, FighterKey, FleetKey, ManufacturingFacilityKey,
    ProductionFacilityKey, SpecialForceKey, SystemKey, TroopKey,
};
use rebellion_core::manufacturing::ProductionArea;
use rebellion_core::missions::MissionMember;
use rebellion_core::scrap::ScrapTarget;
use rebellion_core::world::FacilityRef;

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
    /// The viewing side's frozen copy of the other side's regiment `id`
    /// (`FUN_004f9a60`; `viewed_contents`): its menu offers what an enemy
    /// regiment's does, all grey.
    ViewedTroop {
        alliance: bool,
        id: rebellion_core::ids::DatId,
    },
    /// An object of the player's frozen copy of `system` with no live key
    /// (`viewed_contents::ViewedUnit`): its menu holds Encyclopedia and
    /// Status (`FUN_0051d990`).
    ViewedUnit {
        system: SystemKey,
        unit: crate::viewed_contents::ViewedUnit,
    },
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
    /// A System window yard (`0x28..0x2a`).
    ManufacturingFacility(ManufacturingFacilityKey),
    /// A System window mine or refinery (`0x2c..0x2d`).
    ProductionFacility(ProductionFacilityKey),
    /// The Alliance headquarters (ALLFACSD, `0x20`), chosen by its sector
    /// window overlay (id `0x10000`, `FUN_0045bbb0`); the port keeps it
    /// with the yards.
    Headquarters(ManufacturingFacilityKey),
    /// A facility page's unfinished unit for `system`: one being built
    /// (`+0x50` bit 2 clear), or travelling there until `arrival` (bit 4).
    /// `owner` is its side, 1 or 2.
    PendingFacility {
        system: SystemKey,
        class: rebellion_core::ids::DatId,
        owner: u8,
        arrival: Option<u64>,
    },
    /// A Defenses window squadron being built for `system`'s planet
    /// (`0x1c..0x1f`, `+0x50` bit 2 clear), by its class.
    PendingFighter {
        system: SystemKey,
        class: FighterKey,
    },
    /// A squadron of `class` travelling by Move to `to`, a planet (the
    /// Defenses window) or a fleet (the Fleet window), until day `arrival`
    /// (`0x1c..0x1f`, `+0x50` bit 4): `FUN_00556390` puts it in its
    /// destination at once.
    TravellingFighter {
        to: rebellion_core::squadrons::SquadronPlace,
        class: FighterKey,
        arrival: u64,
    },
    /// A Defenses window regiment in training for `system` (`0x10..0x13`,
    /// `+0x50` bit 2 clear), by its class.
    PendingTroop {
        system: SystemKey,
        class: rebellion_core::ids::DatId,
    },
    /// A Missions window's mission, by its id (`0x50..0x7f`): the window's
    /// slot 22 (`FUN_004a0400`) gives the left list's selected missions to
    /// the menu (A0 missionstatus/02: Encyclopedia, Status, Abort).
    Mission(u64),
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
            | Self::ViewedTroop { .. }
            | Self::ViewedUnit { .. }
            | Self::Ship { .. }
            | Self::SystemIcon { .. }
            | Self::Producer { .. }
            | Self::Fighter { .. }
            | Self::PlanetFighter { .. }
            | Self::DefenseFacility(_)
            | Self::ManufacturingFacility(_)
            | Self::ProductionFacility(_)
            | Self::Headquarters(_)
            | Self::PendingFacility { .. }
            | Self::PendingFighter { .. }
            | Self::TravellingFighter { .. }
            | Self::PendingTroop { .. }
            | Self::Mission(_) => None,
        }
    }

    /// What a Scrap order (`0x200`) names for the object
    /// (`rebellion_core::scrap`). A squadron aboard a fleet and a sector
    /// window's icon name theirs through [`Self::scrap_team`], which reads
    /// the world.
    #[must_use]
    pub const fn scrap_target(self) -> Option<ScrapTarget> {
        match self {
            Self::Troop(key) => Some(ScrapTarget::Troop(key)),
            Self::Ship { fleet, index, .. } => Some(ScrapTarget::Ship { fleet, index }),
            Self::PlanetFighter { system, class } => {
                Some(ScrapTarget::PlanetFighter { system, class })
            }
            Self::DefenseFacility(key) => Some(ScrapTarget::Facility(FacilityRef::Defense(key))),
            Self::ManufacturingFacility(key) => {
                Some(ScrapTarget::Facility(FacilityRef::Manufacturing(key)))
            }
            Self::ProductionFacility(key) => {
                Some(ScrapTarget::Facility(FacilityRef::Production(key)))
            }
            Self::Fleet(fleet) => Some(ScrapTarget::Fleet(fleet)),
            Self::Character(_)
            | Self::SpecialForce(_)
            | Self::ViewedTroop { .. }
            | Self::ViewedUnit { .. }
            | Self::SystemIcon { .. }
            | Self::Producer { .. }
            | Self::Fighter { .. }
            | Self::Headquarters(_)
            | Self::PendingFacility { .. }
            | Self::PendingFighter { .. }
            | Self::TravellingFighter { .. }
            | Self::PendingTroop { .. }
            | Self::Mission(_) => None,
        }
    }

    /// The objects a Scrap on this selection acts on for `side`: the one
    /// object, or for a sector window's icon its kind's objects of the
    /// side (`rebellion_core::scrap::icon_targets`, `FUN_00512700`).
    #[must_use]
    pub fn scrap_team(
        self,
        movement: &rebellion_core::movement::MovementState,
        world: &rebellion_core::world::GameWorld,
        side: rebellion_core::dat::Faction,
    ) -> Vec<ScrapTarget> {
        match self {
            Self::SystemIcon { system, quadrant } => rebellion_core::scrap::icon_targets(
                movement,
                world,
                system,
                quadrant.kind(),
                side,
            ),
            // A squadron aboard a fleet, by its entry's class
            // (FUN_005575f0; A0 2026-11-13-drag/16).
            Self::Fighter { fleet, index } => world
                .fleets
                .get(fleet)
                .and_then(|value| value.fighters.get(index))
                .map(|entry| ScrapTarget::FleetFighter {
                    fleet,
                    class: entry.class,
                })
                .into_iter()
                .collect(),
            other => other.scrap_target().into_iter().collect(),
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
            Self::ViewedTroop { alliance, id } => Some(StatusObject::ViewedTroop { alliance, id }),
            Self::ViewedUnit { system, unit } => Some(StatusObject::ViewedUnit { system, unit }),
            Self::Ship { fleet, index, .. } => Some(StatusObject::Ship { fleet, index }),
            Self::Producer { system, area } => Some(StatusObject::Producer { system, area }),
            Self::Fighter { fleet, index } => Some(StatusObject::Fighter { fleet, index }),
            Self::PlanetFighter { system, class } => {
                Some(StatusObject::PlanetFighter { system, class })
            }
            Self::DefenseFacility(key) => Some(StatusObject::DefenseFacility(key)),
            Self::ManufacturingFacility(key) | Self::Headquarters(key) => {
                Some(StatusObject::ManufacturingFacility(key))
            }
            Self::ProductionFacility(key) => Some(StatusObject::ProductionFacility(key)),
            Self::PendingFacility {
                system,
                class,
                owner,
                arrival,
            } => Some(StatusObject::PendingFacility {
                system,
                class,
                owner,
                arrival,
            }),
            Self::PendingFighter { system, class } => {
                Some(StatusObject::PendingFighter { system, class })
            }
            Self::TravellingFighter { to, class, arrival } => {
                Some(StatusObject::TravellingFighter { to, class, arrival })
            }
            Self::PendingTroop { system, class } => {
                Some(StatusObject::PendingTroop { system, class })
            }
            Self::Mission(id) => Some(StatusObject::Mission(id)),
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
    /// Drawn with the record's check mark (`FUN_00442590` flags bit 1).
    pub checked: bool,
}

/// The check mark a checked row shows: the Reserved record's word 10,
/// STRATEGY 11902 (`ghidra/notes/sector-icon-menus.md`; A0 menus3/33).
const CHECK_MARK: u32 = 11_902;

impl ObjectMenuRow {
    #[must_use]
    pub const fn entry(&self) -> GameMenuEntry<'static> {
        GameMenuEntry {
            label: self.item.label,
            // FUN_00442590 flags bit 1: checked when the order's `+0x14`
            // says so. The menu's fresh order (FUN_0051d990) leaves its
            // `+0x40` list empty, so the Command orders' FUN_00580d80
            // answers 0; a band's Reserved shows its manager's mark (A0
            // menus3/33).
            icon: if self.checked { Some(CHECK_MARK) } else { None },
            enabled: self.enabled,
            submenu: self.item.submenu,
            check: self.checked,
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
/// follow `gates.fleet_move` for the system's fleets of the player's side;
/// the facility icon's Reserved follows `gates.reserved` and is checked by
/// `gates.reserved_checked`, as a band's is (its team is every manager of
/// the system, `FUN_00512700` kind 4). The defenses icon's Move and
/// Confirmed Move follow `gates.defenses_move` for its mixed team
/// (regiments, squadrons, characters and special forces, split by
/// `FUN_0053c810` and `FUN_0053ca50`).
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
/// - A regiment's Move and Confirmed Move are enabled when
///   `gates.troop_move` says so (A0 `status/11-troop-menu.png`).
/// - A squadron's Move and Confirmed Move are enabled when
///   `gates.squadron_move` says so (A0 `status/20-fighter-menu.png`).
/// - A capital ship's Move and Create Fleet are enabled when
///   `gates.ship_move` says so (its fleet is the player's and in orbit).
///   port: its Confirmed Move stays disabled.
/// - A fleet's or fleet icon's Planetary Assault is enabled when
///   `gates.assault` says so (`assault::order_enabled`, `FUN_00581ae0`).
/// - Encyclopedia is enabled for a single selection.
/// - Status is enabled for any single selection that is not a system
///   (`FUN_0051d990`, `ghidra/notes/status-window.md`).
/// - Planetary Bombardment and Command, submenu parents, are enabled when
///   one of their rows is (hyp, from A0); the rows are
///   [`object_submenu_rows`].
/// - A character's Move and Confirmed Move are enabled when
///   `gates.character_move` says so; a character's or a special force's
///   Retire when `gates.retire` does; a special force's Move and
///   Confirmed Move when `gates.special_force_move` does. port: the other
///   fleet orders stay disabled until their windows and orders are ported.
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
    let special_force = matches!(selection, Some(MenuObject::SpecialForce(_)));
    let ship = matches!(selection, Some(MenuObject::Ship { .. }));
    let producer = matches!(selection, Some(MenuObject::Producer { .. }));
    // The facility icon's Reserved acts on every manager of the system
    // (FUN_00512700 kind 4 -> FUN_0052c170), a band's on its own.
    let managers = producer
        || matches!(
            selection,
            Some(MenuObject::SystemIcon {
                quadrant: Quadrant::System,
                ..
            })
        );
    let headquarters = matches!(selection, Some(MenuObject::Headquarters(_)));
    // The defenses icon's Move rows act on its kind 8 team (FUN_00512700).
    let defenses = matches!(
        selection,
        Some(MenuObject::SystemIcon {
            quadrant: Quadrant::Defenses,
            ..
        })
    );
    let offered: &[ObjectMenuItem] = match selection {
        Some(MenuObject::Character(_)) => &[MOVE, CONFIRMED_MOVE, RETIRE, MISSION, COMMAND],
        Some(MenuObject::SpecialForce(_)) => &[MOVE, CONFIRMED_MOVE, RETIRE, MISSION],
        Some(MenuObject::Fleet(_)) => &[MOVE, CONFIRMED_MOVE, BOMBARDMENT, ASSAULT, RENAME, SCRAP],
        // A regiment in training keeps them, its Scrap grey for want of a
        // target. port: its Move rows are grey too (a queue entry).
        Some(
            MenuObject::Troop(_)
            | MenuObject::PendingTroop { .. }
            | MenuObject::ViewedTroop { .. },
        ) => &[MOVE, CONFIRMED_MOVE, SCRAP],
        Some(MenuObject::Ship { .. }) => &[MOVE, CONFIRMED_MOVE, SCRAP, RENAME, CREATE_FLEET],
        Some(MenuObject::SystemIcon { quadrant, .. }) => match quadrant {
            Quadrant::System => &[SCRAP, DESTINATION, RESERVED],
            Quadrant::Defenses => &[MOVE, CONFIRMED_MOVE, SCRAP],
            Quadrant::Fleets => &[MOVE, CONFIRMED_MOVE, SCRAP, BOMBARDMENT, ASSAULT],
            Quadrant::Missions => &[ABORT],
        },
        Some(MenuObject::Mission(_)) => &[ABORT],
        Some(MenuObject::Producer { area, .. }) => match area {
            ProductionArea::Shipyard => {
                &[BUILD_SHIPS, STOP, DESTINATION, PRODUCER_RENAME, RESERVED]
            }
            ProductionArea::TrainingFacility => &[BUILD_TROOPS, STOP, DESTINATION, RESERVED],
            ProductionArea::ConstructionYard => &[BUILD_FACILITIES, STOP, DESTINATION, RESERVED],
        },
        // FUN_00502fe0 -> FUN_00557ce0: 0x201, 0x202, 0x204 (no STRATEGY
        // record) and 0x200.
        // An unfinished squadron keeps them (A0 pending-facility/24); its
        // Scrap names no target, so it is grey. port: its Move rows are grey
        // too, where A0 offers them: an unfinished unit is a queue entry.
        // One in hyperspace keeps them all grey, as an en route regiment's
        // are (A0 2026-11-11-regiment-en-route/03; FUN_004f9860 refuses an
        // object in hyperspace).
        Some(
            MenuObject::Fighter { .. }
            | MenuObject::PlanetFighter { .. }
            | MenuObject::PendingFighter { .. }
            | MenuObject::TravellingFighter { .. },
        ) => &[MOVE, CONFIRMED_MOVE, SCRAP],
        // Every facility class (defenses, yards, mines and refineries)
        // keeps the base object's order list, slot +0x3c (FUN_005399e0):
        // the one order 0x200 that FUN_00539c20 puts in it. FUN_0051d990
        // adds Encyclopedia and Status.
        // An unfinished unit keeps its class's list too; its Scrap names no
        // target, so it is grey (A0 pending-facility/12).
        Some(
            MenuObject::DefenseFacility(_)
            | MenuObject::ManufacturingFacility(_)
            | MenuObject::ProductionFacility(_)
            | MenuObject::PendingFacility { .. },
        ) => &[SCRAP],
        // The headquarters' slot +0x3c (FUN_00526860 -> FUN_00526c00):
        // 0x201, 0x202 and 0x204 (no STRATEGY record) after FUN_004f2a10's
        // empty object list; no Scrap.
        Some(MenuObject::Headquarters(_)) => &[MOVE, CONFIRMED_MOVE],
        Some(MenuObject::ViewedUnit { .. }) | None => &[],
    };
    let mut rows: Vec<ObjectMenuRow> = offered
        .iter()
        .chain(&[ENCYCLOPEDIA, STATUS])
        .map(|&item| ObjectMenuRow {
            item,
            checked: item.command == ObjectMenuCommand::Reserved
                && managers
                && gates.reserved_checked,
            enabled: match item.command {
                ObjectMenuCommand::Mission => gates.mission,
                ObjectMenuCommand::Move => {
                    (character && gates.character_move)
                        || (special_force && gates.special_force_move)
                        || (fleet && gates.fleet_move)
                        || (troop && gates.troop_move)
                        || (ship && gates.ship_move)
                        || (squadron && gates.squadron_move)
                        || (headquarters && gates.headquarters_move)
                        || (defenses && gates.defenses_move)
                }
                ObjectMenuCommand::CreateFleet => ship && gates.ship_move,
                ObjectMenuCommand::Rename => (fleet_entry || ship) && gates.rename,
                ObjectMenuCommand::Destination => gates.destination,
                // Reserved (0x216): each Event216's +0x18 asks its
                // manager's +0x1d8 (FUN_004f9860), FUN_00553770 an
                // unempty team; ManuMgr_vt1fc sets it.
                ObjectMenuCommand::Reserved => managers && gates.reserved,
                ObjectMenuCommand::Stop => producer && gates.stop,
                ObjectMenuCommand::Build => producer && gates.build,
                // A0 fleet3/02: a ship's Confirmed Move is white with its
                // Move.
                ObjectMenuCommand::ConfirmedMove => {
                    (fleet && gates.fleet_move)
                        || (ship && gates.ship_move)
                        || (troop && gates.troop_move)
                        || (squadron && gates.squadron_move)
                        || (character && gates.character_move)
                        || (special_force && gates.special_force_move)
                        || (headquarters && gates.headquarters_move)
                        || (defenses && gates.defenses_move)
                }
                // hyp: a submenu parent is enabled when one of its
                // children is. A0 shows Planetary Bombardment grey, its
                // submenu never opening, on an enemy fleet
                // (status/05-fleet-menu.png) and on the player's fleet
                // icon at its own system (menus/08-fleet-icon-menu.png),
                // where no target is enabled; FUN_004aba60 greys a row
                // whose +0x58 is clear.
                ObjectMenuCommand::Bombardment => gates.bombard.contains(&true),
                ObjectMenuCommand::Command => gates.command.contains(&true),
                ObjectMenuCommand::Assault => gates.assault,
                ObjectMenuCommand::Abort => gates.abort,
                ObjectMenuCommand::Retire => {
                    matches!(
                        selection,
                        Some(MenuObject::Character(_) | MenuObject::SpecialForce(_))
                    ) && gates.retire
                }
                // The gate checks the order's whole team: the object, or an
                // icon's kind's objects (MenuObject::scrap_team).
                ObjectMenuCommand::Scrap => selection.is_some() && gates.scrap,
                ObjectMenuCommand::Encyclopedia => selection.is_some(),
                // FUN_0051d990: a single selection that is not a system.
                ObjectMenuCommand::Status => {
                    selection.and_then(MenuObject::status_object).is_some()
                }
                _ => false,
            },
        })
        .collect();
    // FUN_0051d990 enables an order only while the global gate
    // FUN_0051de80 is clear; Encyclopedia and Status skip it.
    if gates.orders_stopped {
        for row in &mut rows {
            if !matches!(
                row.item.command,
                ObjectMenuCommand::Encyclopedia | ObjectMenuCommand::Status
            ) {
                row.enabled = false;
            }
        }
    }
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
            .map(|(&item, enabled)| ObjectMenuRow {
                item,
                enabled: enabled && !gates.orders_stopped,
                checked: false,
            })
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
            enabled: enabled && !gates.orders_stopped,
            checked: false,
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
    /// A special force's Move (`0x201` -> `0x241`): the player's unit, as a
    /// character's (`FUN_00536bc0` lists the same unit orders).
    pub special_force_move: bool,
    /// A squadron's Move: the player's, on a planet or aboard a fleet in
    /// orbit ([`rebellion_core::squadrons`]).
    pub squadron_move: bool,
    /// The headquarters' Move ([`rebellion_core::headquarters_move::move_enabled`]).
    pub headquarters_move: bool,
    /// A defenses icon's Move and Confirmed Move (`FUN_0053c100`): the
    /// team `FUN_00512700` kind 8 gives is not empty (`FUN_0053c4b0`,
    /// `1`/`0x16`), and each sub-order's `+0x18` passes: the `0x241` of its
    /// characters and special forces
    /// ([`rebellion_core::missions::MissionState::move_order_enabled`]) and
    /// the `0x204` group's per-member command checks for its regiments and
    /// squadrons (`FUN_00578c00`).
    pub defenses_move: bool,
    pub ship_move: bool,
    /// Rename (0x203): the selection is the player's and can take a name
    /// (`FUN_004f6e60` refuses a destroyed object).
    pub rename: bool,
    /// Destination (0x214) on a facility icon: the system has production
    /// areas of the player's (`FUN_00512700` kind 4); on a band, the band's
    /// area is the player's.
    pub destination: bool,
    /// Reserved (0x216) on a band: enabled as Destination is (hyp: its
    /// `+0x1dc` is not read; A0 menus3/30-32 enable it on every band of
    /// the player's system, Destination too), and checked while the band's
    /// manager is Reserved. On a facility icon: enabled when the system has
    /// managers of the player's (the team `FUN_00512700` kind 4 gives;
    /// `FUN_00553770` refuses an empty one with `0x16`), and checked while
    /// any of them is Reserved (`FUN_0051fd30`,
    /// `ManufacturingState::any_reserved`).
    pub reserved: bool,
    pub reserved_checked: bool,
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
    /// Abort (`0x250`) on a mission icon: every mission it names passes
    /// `FUN_00521600` (`MissionState::abort_allowed`).
    pub abort: bool,
    /// Retire (`0x242`) on a character (`FUN_004ed460`,
    /// `rebellion_core::character_retire::order_allowed`) or a special
    /// force (`FUN_00533ba0`, `special_force_allowed`).
    pub retire: bool,
    /// Scrap (`0x200`): the object passes its slot `+0x5c` check, the
    /// player's and not a headquarters (`rebellion_core::scrap::check`).
    pub scrap: bool,
    /// The global gate `FUN_0051de80`: the victory conditions word's bit
    /// `0x10000000` (the game is over, `FUN_005180e0`) greys every order.
    /// A0 pending-facility/33 and 36 (day 633 of a game the Alliance had
    /// lost, its clock stopped) grey a band's Build, Stop, Destination,
    /// Rename and Reserved and leave Encyclopedia and Status white.
    pub orders_stopped: bool,
}

/// An open object pop-up menu.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectMenuState {
    selection: Option<MenuObject>,
    /// Every object the window had selected, the clicked one first, when
    /// the menu opens for several ([`Self::with_team`]).
    team: Vec<MenuObject>,
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
            team: selection.into_iter().collect(),
            rows: object_menu_rows(selection, gates),
            submenu: object_submenu_rows(selection, gates),
            window: GameMenuState::default(),
            point: (f32::from(point.0), f32::from(point.1)),
        }
    }

    /// The menu for a window's whole selection, `team`, its clicked object
    /// first (`FUN_004ac5c0` with the window's slot `+0x58`). For several
    /// objects `FUN_0051d990` enables Encyclopedia and Status only for a
    /// single selection; the orders take the whole team.
    ///
    /// Each order is enabled when its `+0x18` passes for the whole team
    /// (the gates, [`OrderGates`], hold the team's): Scrap (`0x200`,
    /// `FUN_005201c0` one command per member) and Retire (`0x242`, a team
    /// of characters and special forces) take every member; the Command
    /// orders refuse more than one object (`FUN_00580db0`).
    ///
    /// port: a team keeps only the orders ported for several objects: Move
    /// and Confirmed Move, a team of ships' Create Fleet, a team of
    /// characters' and special forces' Mission and Retire, and Scrap. The
    /// rest are drawn disabled, and the menu opens no submenu.
    #[must_use]
    pub fn with_team(mut self, team: Vec<MenuObject>) -> Self {
        if team.len() > 1 {
            let ships = team.iter().all(|object| matches!(object, MenuObject::Ship { .. }));
            let members = team.iter().all(|object| object.mission_member().is_some());
            for row in &mut self.rows {
                row.enabled &= match row.item.command {
                    ObjectMenuCommand::Move
                    | ObjectMenuCommand::ConfirmedMove
                    | ObjectMenuCommand::Scrap => true,
                    ObjectMenuCommand::CreateFleet => ships,
                    ObjectMenuCommand::Mission | ObjectMenuCommand::Retire => members,
                    _ => false,
                };
            }
            self.submenu.clear();
        }
        self.team = team;
        self
    }

    /// Every object the menu was opened for.
    #[must_use]
    pub fn team(&self) -> &[MenuObject] {
        &self.team
    }

    /// A right-button release on an open Finder (`on_finder`) opens the
    /// menu there. The System, Fleet, Personnel and Troop Finders (vtables
    /// `0x65a000`, `0x65a090`, `0x65a130`, `0x65a270`) take slot 7,
    /// `FUN_004ac5c0`, for the release, and their selection slot `+0x58`,
    /// `FUN_00461740`, gives nothing: Encyclopedia and Status, both
    /// disabled (A0 personnel-states/03, 04).
    #[must_use]
    pub fn for_finder(
        ctx: &egui::Context,
        layout: CockpitLayout,
        on_finder: impl Fn((f32, f32)) -> bool,
    ) -> Option<Self> {
        let point = ctx.input(|input| {
            input
                .pointer
                .button_released(egui::PointerButton::Secondary)
                .then(|| input.pointer.interact_pos())
                .flatten()
        })?;
        on_finder((point.x, point.y)).then(|| {
            Self::new(
                None,
                OrderGates::default(),
                crate::system_window::canvas_point(layout, point),
            )
        })
    }

    /// The object the menu was opened for.
    #[must_use]
    pub fn selection(&self) -> Option<MenuObject> {
        self.selection
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

    /// Whether the row that issues `command` carries its check mark.
    #[must_use]
    pub fn row_checked(&self, command: ObjectMenuCommand) -> bool {
        self.rows
            .iter()
            .any(|row| row.item.command == command && row.checked)
    }

    /// Each row's label and whether it is enabled, top to bottom.
    pub fn row_states(&self) -> impl Iterator<Item = (&'static str, bool)> + '_ {
        self.rows.iter().map(|row| (row.item.label, row.enabled))
    }

    /// Whether the submenu's row at `row` is enabled.
    #[must_use]
    pub fn submenu_row_enabled(&self, row: usize) -> bool {
        self.submenu.get(row).is_some_and(|row| row.enabled)
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
        special_force_move: false,
        squadron_move: false,
        headquarters_move: false,
        defenses_move: false,
        ship_move: false,
        rename: false,
        destination: false,
        reserved: false,
        reserved_checked: false,
        stop: false,
        build: false,
        bombard: [false; 4],
        assault: false,
        command: [false; 4],
        retire: false,
        scrap: false,
        orders_stopped: false,
        abort: false,
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
    fn a_missions_window_mission_offers_encyclopedia_status_and_abort() {
        // A0 missionstatus/02: Encyclopedia and Status white, Abort grey
        // until the mission may be aborted; Status opens Mission Status.
        let mission = Some(MenuObject::Mission(7));
        let rows = object_menu_rows(mission, OrderGates::default());
        assert_eq!(labels(&rows), ["Encyclopedia", "Status", "Abort"]);
        assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
        let abortable = OrderGates {
            abort: true,
            ..OrderGates::default()
        };
        assert!(enabled(&object_menu_rows(mission, abortable)).contains(&"Abort"));
        assert_eq!(
            MenuObject::Mission(7).status_object(),
            Some(StatusObject::Mission(7))
        );
    }

    #[test]
    fn the_menu_reports_each_rows_label_and_state_in_order() {
        let selection = Some(MenuObject::Character(CharacterKey::default()));
        let rows = object_menu_rows(selection, MISSION_GATE);
        let menu = ObjectMenuState::new(selection, MISSION_GATE, (0, 0));
        assert_eq!(menu.selection(), selection);
        assert!(!menu.row_checked(ObjectMenuCommand::Move));
        let states: Vec<_> = menu.row_states().collect();
        assert_eq!(
            states,
            rows.iter()
                .map(|row| (row.item.label, row.enabled))
                .collect::<Vec<_>>()
        );
        assert!(states.iter().any(|&(_, on)| on) && states.iter().any(|&(_, on)| !on));
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
        // No bombardment target is enabled, so neither is their parent
        // (A0 status/05-fleet-menu.png).
        assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
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
            special_force_move: false,
            squadron_move: true,
            headquarters_move: true,
            defenses_move: false,
            ship_move: true,
            rename: false,
            destination: false,
            reserved: false,
            reserved_checked: false,
            stop: false,
            build: false,
            bombard: [false; 4],
            assault: false,
            command: [false; 4],
            retire: false,
            scrap: false,
            orders_stopped: false,
            abort: false,
        };
        assert_eq!(
            enabled(&object_menu_rows(fleet, gates)),
            ["Move", "Confirmed Move", "Encyclopedia", "Status"]
        );
        assert_eq!(
            enabled(&object_menu_rows(fleet, OrderGates::default())),
            ["Encyclopedia", "Status"]
        );
        // A character's Move follows its own gate; Command, a submenu
        // parent, follows its rows (none enabled here).
        let character = Some(MenuObject::Character(CharacterKey::default()));
        assert_eq!(
            enabled(&object_menu_rows(character, gates)),
            ["Encyclopedia", "Status"]
        );
    }

    #[test]
    fn a_characters_move_row_follows_its_move_rule() {
        // 0x201 and 0x202 -> 0x241 (FUN_0053c810): the rows are enabled
        // when the character resolves (FUN_004f4a00 only builds the
        // command).
        let gates = OrderGates {
            character_move: true,
            ..OrderGates::default()
        };
        let character = Some(MenuObject::Character(CharacterKey::default()));
        assert_eq!(
            enabled(&object_menu_rows(character, gates)),
            ["Move", "Confirmed Move", "Encyclopedia", "Status"]
        );
        let unit = Some(MenuObject::SpecialForce(SpecialForceKey::default()));
        assert!(!enabled(&object_menu_rows(unit, gates)).contains(&"Move"));
    }

    #[test]
    fn a_special_forces_move_rows_follow_their_own_rule() {
        // FUN_00536bc0: a unit lists Move and Confirmed Move as a character
        // does; both expand to the Move mission 0x241.
        let gates = OrderGates {
            special_force_move: true,
            ..OrderGates::default()
        };
        let unit = Some(MenuObject::SpecialForce(SpecialForceKey::default()));
        assert_eq!(
            enabled(&object_menu_rows(unit, gates)),
            ["Move", "Confirmed Move", "Encyclopedia", "Status"]
        );
        // The unit's gate opens no character's rows.
        let character = Some(MenuObject::Character(CharacterKey::default()));
        let rows = enabled(&object_menu_rows(character, gates));
        assert!(!rows.contains(&"Move") && !rows.contains(&"Confirmed Move"));
    }

    #[test]
    fn a_characters_retire_row_follows_its_rule() {
        // 0x242 (TEXTSTRA 12336, sort 2002): enabled through event 0x230,
        // for a character (FUN_004ed460) or a special force (FUN_00533ba0;
        // both classes list the unit orders of FUN_00536bc0); not for a
        // regiment, whose list has no 0x242.
        let gates = OrderGates {
            retire: true,
            abort: false,
            ..OrderGates::default()
        };
        let character = Some(MenuObject::Character(CharacterKey::default()));
        assert_eq!(
            enabled(&object_menu_rows(character, gates)),
            ["Encyclopedia", "Status", "Retire"]
        );
        let unit = Some(MenuObject::SpecialForce(SpecialForceKey::default()));
        assert!(enabled(&object_menu_rows(unit, gates)).contains(&"Retire"));
        let troop = Some(MenuObject::Troop(TroopKey::default()));
        // A frozen copy's unit offers only Encyclopedia and Status
        // (FUN_0051d990).
        let viewed = object_menu_rows(
            Some(MenuObject::ViewedUnit {
                system: SystemKey::default(),
                unit: crate::viewed_contents::ViewedUnit::Facility {
                    class: rebellion_core::ids::DatId::new(0x2200_0001),
                    owner: 2,
                },
            }),
            MISSION_GATE,
        );
        assert_eq!(
            viewed.iter().map(|row| row.item).collect::<Vec<_>>(),
            [ENCYCLOPEDIA, STATUS]
        );
        assert!(!object_menu_rows(troop, gates)
            .iter()
            .any(|row| row.item.label == "Retire"));
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
    fn the_end_of_the_game_greys_every_order() {
        // FUN_0051d990 asks FUN_0051de80 for each order: the victory
        // conditions word's bit 0x10000000. A0 pending-facility/33 (day
        // 633, the game over) greys the ships band's Build, Stop,
        // Destination, Rename and Reserved and keeps Encyclopedia and Status.
        let band = Some(MenuObject::Producer {
            system: SystemKey::default(),
            area: ProductionArea::Shipyard,
        });
        let open = OrderGates {
            destination: true,
            stop: true,
            build: true,
            reserved: true,
            ..OrderGates::default()
        };
        assert_eq!(
            enabled(&object_menu_rows(band, open)),
            ["Build", "Stop", "Destination", "Encyclopedia", "Status", "Reserved"]
        );
        let stopped = OrderGates {
            orders_stopped: true,
            ..open
        };
        assert_eq!(
            enabled(&object_menu_rows(band, stopped)),
            ["Encyclopedia", "Status"]
        );
        // The submenus' rows too.
        let fleet = Some(MenuObject::Fleet(FleetKey::default()));
        let bombard = OrderGates {
            bombard: [true; 4],
            ..OrderGates::default()
        };
        assert!(object_submenu_rows(fleet, bombard).iter().all(|row| row.enabled));
        assert!(
            object_submenu_rows(
                fleet,
                OrderGates {
                    orders_stopped: true,
                    ..bombard
                }
            )
            .iter()
            .all(|row| !row.enabled)
        );
        let character = Some(MenuObject::Character(CharacterKey::default()));
        let command = OrderGates {
            command: [true; 4],
            orders_stopped: true,
            ..OrderGates::default()
        };
        assert!(object_submenu_rows(character, command).iter().all(|row| !row.enabled));
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
        // A0 menus3/30-33: Reserved is enabled on the player's bands and
        // carries the check mark (STRATEGY 11902) while the manager is
        // Reserved; a checked row draws the mark in the icon column.
        let reserve = OrderGates {
            reserved: true,
            ..OrderGates::default()
        };
        let row = |gates: OrderGates| {
            object_menu_rows(band(ProductionArea::ConstructionYard), gates)
                .into_iter()
                .find(|row| row.item.command == ObjectMenuCommand::Reserved)
                .unwrap()
        };
        assert!(row(reserve).enabled && !row(reserve).checked);
        assert_eq!(row(reserve).entry().icon, None);
        let checked = row(OrderGates {
            reserved_checked: true,
            ..reserve
        });
        assert!(checked.checked);
        assert_eq!(checked.entry().icon, Some(11_902));
        let menu = ObjectMenuState::new(
            band(ProductionArea::ConstructionYard),
            OrderGates {
                reserved_checked: true,
                ..reserve
            },
            (0, 0),
        );
        assert!(menu.row_checked(ObjectMenuCommand::Reserved));
        assert!(!menu.row_checked(ObjectMenuCommand::Destination));
        assert!(!row(OrderGates::default()).enabled);
        // The facility icon's Reserved follows the same gates for every
        // manager of the system (FUN_00512700 kind 4 -> FUN_0052c170;
        // Event216_vt14 -> ManuMgr_vt1d4 marks it).
        let icon_reserved = |gates: OrderGates| {
            object_menu_rows(icon(Quadrant::System), gates)
                .into_iter()
                .find(|row| row.item.command == ObjectMenuCommand::Reserved)
                .unwrap()
        };
        let marked = icon_reserved(OrderGates {
            reserved_checked: true,
            ..reserve
        });
        assert!(marked.enabled && marked.checked);
        assert_eq!(marked.entry().icon, Some(11_902));
        assert!(icon_reserved(reserve).enabled && !icon_reserved(reserve).checked);
        assert!(!icon_reserved(OrderGates::default()).enabled);
        // No other icon offers it.
        assert!(object_menu_rows(icon(Quadrant::Defenses), reserve)
            .iter()
            .all(|row| row.item.command != ObjectMenuCommand::Reserved));
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
        // Abort follows its rule (FUN_00521600 through event 0x261).
        let gates = OrderGates {
            abort: true,
            ..OrderGates::default()
        };
        assert_eq!(
            enabled(&object_menu_rows(icon(Quadrant::Missions), gates)),
            ["Encyclopedia", "Abort"]
        );
    }

    #[test]
    fn a_right_release_on_a_finder_opens_encyclopedia_and_status_disabled() {
        // A0 personnel-states/03, 04: the Personnel Finder's row menu,
        // Encyclopedia and Status grey; slot +0x58 is FUN_00461740.
        let layout =
            crate::cockpit::CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
        let release = |at: egui::Pos2, button| {
            let ctx = egui::Context::default();
            let mut opened = None;
            for events in [
                vec![egui::Event::PointerMoved(at)],
                vec![egui::Event::PointerButton {
                    pos: at,
                    button,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
                vec![egui::Event::PointerButton {
                    pos: at,
                    button,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            ] {
                let input = egui::RawInput {
                    events,
                    ..Default::default()
                };
                let _ = ctx.run(input, |ctx| {
                    opened = opened.take().or_else(|| {
                        ObjectMenuState::for_finder(ctx, layout, |(x, _)| x < 300.0)
                    });
                });
            }
            opened
        };
        let menu = release(egui::pos2(200.0, 343.0), egui::PointerButton::Secondary)
            .expect("the release on the Finder opens the menu");
        assert_eq!(menu.selection(), None);
        assert_eq!(labels(&menu.rows), ["Encyclopedia", "Status"]);
        assert!(enabled(&menu.rows).is_empty());
        assert_eq!(menu.point, (200.0, 343.0));
        assert_eq!(
            release(egui::pos2(400.0, 343.0), egui::PointerButton::Secondary),
            None,
            "off the Finder"
        );
        assert_eq!(
            release(egui::pos2(200.0, 343.0), egui::PointerButton::Primary),
            None,
            "a left release"
        );
    }

    #[test]
    fn a_sector_icons_status_stays_disabled_because_its_object_is_a_system() {
        // FUN_0051d990: Encyclopedia is enabled for a single selection;
        // Status only when that object is not a system (0x90..0x97). An
        // icon's selection is its system (item +0x68, FUN_004f5b10).
        for quadrant in Quadrant::ALL {
            // With no target enabled the fleet icon's bombardment parent is
            // disabled too (A0 menus/08-fleet-icon-menu.png).
            assert_eq!(
                enabled(&object_menu_rows(icon(quadrant), OrderGates::default())),
                ["Encyclopedia"],
                "{quadrant:?}"
            );
        }
    }

    #[test]
    fn a_fleet_icons_move_rows_follow_its_fleets_move_rule() {
        // FUN_0053c100 -> FUN_0053c4b0: the team is the system's fleets of
        // the order's side (FUN_00512700, kind 0x10); the other icons' Move
        // rows follow their own teams' rules.
        let every_gate = OrderGates {
            mission: true,
            fleet_move: true,
            character_move: true,
            troop_move: true,
            special_force_move: false,
            squadron_move: true,
            headquarters_move: true,
            defenses_move: false,
            ship_move: true,
            rename: false,
            destination: false,
            reserved: false,
            reserved_checked: false,
            stop: true,
            build: true,
            bombard: [false; 4],
            assault: false,
            command: [false; 4],
            retire: false,
            scrap: false,
            orders_stopped: false,
            abort: false,
        };
        assert_eq!(
            enabled(&object_menu_rows(icon(Quadrant::Fleets), every_gate)),
            ["Move", "Confirmed Move", "Encyclopedia"]
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
    fn a_defenses_icons_move_rows_follow_its_mixed_teams_move_rule() {
        // FUN_0050f2e0 lists 0x201 and 0x202 for kind 8; FUN_0053c100 asks
        // FUN_0053c4b0 and each sub-order's +0x18 for the team FUN_00512700
        // kind 8 gives (gates.defenses_move). The other icons ignore it.
        let gates = OrderGates {
            defenses_move: true,
            ..OrderGates::default()
        };
        assert_eq!(
            enabled(&object_menu_rows(icon(Quadrant::Defenses), gates)),
            ["Move", "Confirmed Move", "Encyclopedia"]
        );
        for quadrant in [Quadrant::System, Quadrant::Fleets, Quadrant::Missions] {
            assert_eq!(
                enabled(&object_menu_rows(icon(quadrant), gates)),
                ["Encyclopedia"],
                "{quadrant:?}"
            );
        }
        // The members' own Move gates do not open the icon's rows.
        let members = OrderGates {
            troop_move: true,
            character_move: true,
            squadron_move: true,
            special_force_move: true,
            ..OrderGates::default()
        };
        assert_eq!(
            enabled(&object_menu_rows(icon(Quadrant::Defenses), members)),
            ["Encyclopedia"]
        );
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
    fn a_menu_for_several_ships_keeps_the_team_orders_and_greys_encyclopedia_and_status() {
        // FUN_0051d990: Encyclopedia and Status only for a single
        // selection; the orders take the whole team (manual p. 120: Ctrl
        // selects ships, then Create Fleet).
        let ship = |index| MenuObject::Ship {
            fleet: FleetKey::default(),
            index,
            roster: 0,
        };
        let gates = OrderGates {
            ship_move: true,
            ..OrderGates::default()
        };
        let menu = ObjectMenuState::new(Some(ship(0)), gates, (0, 0))
            .with_team(vec![ship(0), ship(2)]);
        assert_eq!(enabled(&menu.rows), ["Move", "Confirmed Move", "Create Fleet"]);
        assert_eq!(menu.team(), [ship(0), ship(2)]);
        // Characters together keep Mission (one team, FUN_00487c50).
        let mut world = rebellion_core::world::GameWorld::default();
        let (a, b) = (
            world.characters.insert(Default::default()),
            world.characters.insert(Default::default()),
        );
        let both = ObjectMenuState::new(Some(MenuObject::Character(a)), MISSION_GATE, (0, 0))
            .with_team(vec![MenuObject::Character(a), MenuObject::Character(b)]);
        assert!(enabled(&both.rows).contains(&"Mission"), "{:?}", enabled(&both.rows));
        // One object keeps its single menu.
        let one = ObjectMenuState::new(Some(ship(0)), gates, (0, 0)).with_team(vec![ship(0)]);
        assert_eq!(
            enabled(&one.rows),
            ["Move", "Confirmed Move", "Create Fleet", "Encyclopedia", "Status"]
        );
    }

    #[test]
    fn a_team_menu_enables_scrap_and_a_member_teams_retire_by_the_teams_gates() {
        // FUN_0051d990: each order's +0x18 runs with the whole selection as
        // its team; Scrap's (FUN_0051fe20 over FUN_005201c0's command per
        // member) and Retire's pass when every member's does.
        let ship = |index| MenuObject::Ship {
            fleet: FleetKey::default(),
            index,
            roster: 0,
        };
        let gates = OrderGates {
            ship_move: true,
            scrap: true,
            ..OrderGates::default()
        };
        let ships = ObjectMenuState::new(Some(ship(0)), gates, (0, 0))
            .with_team(vec![ship(0), ship(2)]);
        assert!(enabled(&ships.rows).contains(&"Scrap"), "{:?}", enabled(&ships.rows));
        let refused = ObjectMenuState::new(
            Some(ship(0)),
            OrderGates {
                scrap: false,
                ..gates
            },
            (0, 0),
        )
        .with_team(vec![ship(0), ship(2)]);
        assert!(!enabled(&refused.rows).contains(&"Scrap"));
        let mut world = rebellion_core::world::GameWorld::default();
        let (a, b) = (
            world.characters.insert(Default::default()),
            world.characters.insert(Default::default()),
        );
        let retire = OrderGates {
            retire: true,
            ..MISSION_GATE
        };
        let both = ObjectMenuState::new(Some(MenuObject::Character(a)), retire, (0, 0))
            .with_team(vec![MenuObject::Character(a), MenuObject::Character(b)]);
        assert!(enabled(&both.rows).contains(&"Retire"), "{:?}", enabled(&both.rows));
        // The Command orders take one object (FUN_00580db0).
        assert!(both.submenu.is_empty());
    }

    #[test]
    fn a_capital_ships_move_and_create_fleet_follow_its_fleets_move_rule() {
        // port: its fleet's side and orbit (FUN_004f9860). A0 fleet3/02:
        // Medium Transport 1's Confirmed Move is white with its Move (it was
        // grey here before that capture).
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
            ["Move", "Confirmed Move", "Create Fleet", "Encyclopedia", "Status"]
        );
        assert_eq!(
            enabled(&object_menu_rows(ship, OrderGates::default())),
            ["Encyclopedia", "Status"]
        );
        let fleet = Some(MenuObject::Fleet(FleetKey::default()));
        assert_eq!(
            enabled(&object_menu_rows(fleet, gates)),
            ["Encyclopedia", "Status"]
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
    fn a_regiments_move_and_confirmed_move_follow_its_gate() {
        // A0 .artifacts/a0-vm/status/11-troop-menu.png: the player's
        // regiment shows Move and Confirmed Move enabled (FUN_00504b30).
        let troop = Some(MenuObject::Troop(TroopKey::default()));
        let gates = OrderGates {
            troop_move: true,
            special_force_move: false,
            squadron_move: true,
            ..OrderGates::default()
        };
        assert_eq!(
            enabled(&object_menu_rows(troop, gates)),
            ["Move", "Confirmed Move", "Encyclopedia", "Status"]
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
            ["Encyclopedia", "Status"]
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
            ["Mission", "Encyclopedia", "Status"]
        );
        assert_eq!(
            enabled(&object_menu_rows(character, OrderGates::default())),
            ["Encyclopedia", "Status"]
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
            // A0 .artifacts/a0-vm/status/20-fighter-menu.png: the player's
            // planet squadron shows Move and Confirmed Move enabled.
            assert_eq!(
                enabled(&rows),
                ["Move", "Confirmed Move", "Encyclopedia", "Status"],
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
    fn the_headquarters_lists_move_confirmed_move_encyclopedia_and_status_without_scrap() {
        // The HQ class's slot +0x3c (vtable 0x0065f280, FUN_00526860 ->
        // FUN_00526c00): 0x201, 0x202 and 0x204, which has no STRATEGY
        // record. FUN_0051d990 adds 0x100 and 0x103, Status enabled since
        // the object is no system. The manual (p. 134, Fig. 3.82): "drag
        // the HQ icon onto a new system or right-click, select Move".
        let hq = MenuObject::Headquarters(ManufacturingFacilityKey::default());
        let rows = object_menu_rows(Some(hq), MISSION_GATE);

        assert_eq!(
            labels(&rows),
            ["Move", "Confirmed Move", "Encyclopedia", "Status"]
        );
        assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
        // A0 headquarters/01: Move and Confirmed Move white while the HQ
        // may move (FUN_00578c00).
        let movable = OrderGates {
            headquarters_move: true,
            defenses_move: false,
            ..OrderGates::default()
        };
        assert_eq!(
            enabled(&object_menu_rows(Some(hq), movable)),
            ["Move", "Confirmed Move", "Encyclopedia", "Status"]
        );
        assert_eq!(
            hq.status_object(),
            Some(StatusObject::ManufacturingFacility(
                ManufacturingFacilityKey::default()
            ))
        );
        assert_eq!(hq.scrap_target(), None);
    }

    #[test]
    fn an_unfinished_facility_lists_encyclopedia_status_and_a_grey_scrap() {
        // A0 pending-facility/12: a refinery under construction at Commenor
        // opens Encyclopedia, Status and a grey Scrap (FUN_005399e0's list;
        // its Scrap names no target, so the main.rs gate refuses it).
        let pending = MenuObject::PendingFacility {
            system: SystemKey::default(),
            class: rebellion_core::ids::DatId::new(0x2d00_0002),
            owner: 1,
            arrival: None,
        };
        let rows = object_menu_rows(Some(pending), MISSION_GATE);
        assert_eq!(labels(&rows), ["Encyclopedia", "Status", "Scrap"]);
        assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
        assert_eq!(pending.scrap_target(), None);
        assert_eq!(
            pending.status_object(),
            Some(StatusObject::PendingFacility {
                system: SystemKey::default(),
                class: rebellion_core::ids::DatId::new(0x2d00_0002),
                owner: 1,
                arrival: None,
            })
        );
    }

    #[test]
    fn an_unfinished_squadron_lists_the_squadron_rows_with_a_grey_scrap() {
        // A0 pending-facility/24: Pzob's A-wing under construction opens
        // Move, Confirmed Move, Encyclopedia, Status and a grey Scrap
        // (FUN_00502fe0's list). port: its Move rows stay grey where A0
        // offers them, for the squadron is a queue entry.
        let pending = MenuObject::PendingFighter {
            system: SystemKey::default(),
            class: FighterKey::default(),
        };
        let movable = OrderGates {
            squadron_move: true,
            ..MISSION_GATE
        };
        let rows = object_menu_rows(Some(pending), movable);
        assert_eq!(
            labels(&rows),
            ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"]
        );
        assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
        assert_eq!(pending.scrap_target(), None);
        assert_eq!(pending.mission_member(), None);
        assert_eq!(
            pending.status_object(),
            Some(StatusObject::PendingFighter {
                system: SystemKey::default(),
                class: FighterKey::default(),
            })
        );
    }

    #[test]
    fn a_squadron_in_hyperspace_greys_its_move_rows_and_scrap() {
        // FUN_00502fe0's list, as for a squadron on the planet; an object in
        // hyperspace takes no order (FUN_004f9860), so Move, Confirmed Move
        // and Scrap are grey, as A0 2026-11-11-regiment-en-route/03 shows
        // them on an en route regiment.
        let travelling = MenuObject::TravellingFighter {
            to: rebellion_core::squadrons::SquadronPlace::Planet(SystemKey::default()),
            class: FighterKey::default(),
            arrival: 9,
        };
        let movable = OrderGates {
            squadron_move: true,
            ..MISSION_GATE
        };
        let rows = object_menu_rows(Some(travelling), movable);
        assert_eq!(
            labels(&rows),
            ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"]
        );
        assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
        assert_eq!(travelling.scrap_target(), None);
        assert_eq!(travelling.mission_member(), None);
        assert_eq!(
            travelling.status_object(),
            Some(StatusObject::TravellingFighter {
                to: rebellion_core::squadrons::SquadronPlace::Planet(SystemKey::default()),
                class: FighterKey::default(),
                arrival: 9,
            })
        );
    }

    #[test]
    fn a_regiment_in_training_lists_the_regiment_rows_with_a_grey_scrap() {
        // FUN_00504b30's list: Move, Confirmed Move and Scrap. Its Scrap
        // names no target, so it is grey. port: its Move rows are grey too,
        // for the regiment is a queue entry.
        let pending = MenuObject::PendingTroop {
            system: SystemKey::default(),
            class: rebellion_core::ids::DatId::new(0x1000_0002),
        };
        let movable = OrderGates {
            troop_move: true,
            ..MISSION_GATE
        };
        let rows = object_menu_rows(Some(pending), movable);
        assert_eq!(
            labels(&rows),
            ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"]
        );
        assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
        assert_eq!(pending.scrap_target(), None);
        assert_eq!(
            pending.status_object(),
            Some(StatusObject::PendingTroop {
                system: SystemKey::default(),
                class: rebellion_core::ids::DatId::new(0x1000_0002),
            })
        );
    }

    #[test]
    fn a_facility_lists_scrap_encyclopedia_and_status() {
        // Every facility class keeps the base order list (slot +0x3c,
        // FUN_005399e0): order 0x200, Scrap (FUN_00539c20). FUN_0051d990
        // adds 0x100 and 0x103. Scrap follows its gate
        // (`a_scrap_row_follows_its_gate_for_the_units_it_names`).
        for (selection, status) in [
            (
                MenuObject::DefenseFacility(DefenseFacilityKey::default()),
                StatusObject::DefenseFacility(DefenseFacilityKey::default()),
            ),
            (
                MenuObject::ManufacturingFacility(ManufacturingFacilityKey::default()),
                StatusObject::ManufacturingFacility(ManufacturingFacilityKey::default()),
            ),
            (
                MenuObject::ProductionFacility(ProductionFacilityKey::default()),
                StatusObject::ProductionFacility(ProductionFacilityKey::default()),
            ),
        ] {
            let rows = object_menu_rows(Some(selection), MISSION_GATE);
            assert_eq!(labels(&rows), ["Encyclopedia", "Status", "Scrap"]);
            assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
            assert_eq!(selection.mission_member(), None);
            assert_eq!(selection.status_object(), Some(status));
        }
    }

    #[test]
    fn a_scrap_row_follows_its_gate_for_the_units_it_names() {
        // 0x200 (TEXTSTRA 12295) is enabled by its +0x18 (the object's slot
        // +0x5c, scrap::check). A0 status/11, 20, 22 and 28 show it enabled
        // on the player's regiment, planet squadron, battery and training
        // facility; status/02 shows it disabled on an enemy ship. A0
        // menus/24-fleet1-menu.png shows it enabled on the player's fleet.
        let gates = OrderGates {
            scrap: true,
            ..OrderGates::default()
        };
        for selection in [
            MenuObject::Troop(TroopKey::default()),
            MenuObject::Ship {
                fleet: FleetKey::default(),
                index: 0,
                roster: 0,
            },
            MenuObject::PlanetFighter {
                system: SystemKey::default(),
                class: FighterKey::default(),
            },
            MenuObject::DefenseFacility(DefenseFacilityKey::default()),
            MenuObject::ManufacturingFacility(ManufacturingFacilityKey::default()),
            MenuObject::ProductionFacility(ProductionFacilityKey::default()),
            MenuObject::Fleet(FleetKey::default()),
        ] {
            assert!(selection.scrap_target().is_some(), "{selection:?}");
            assert!(
                enabled(&object_menu_rows(Some(selection), gates)).contains(&"Scrap"),
                "{selection:?}"
            );
            assert!(
                !enabled(&object_menu_rows(Some(selection), OrderGates::default()))
                    .contains(&"Scrap"),
                "{selection:?}"
            );
        }
        assert_eq!(
            MenuObject::Fleet(FleetKey::default()).scrap_target(),
            Some(ScrapTarget::Fleet(FleetKey::default()))
        );
        // A0 menus3/07, 10 and 11: an icon's Scrap is enabled too; its gate
        // checks the icon's kind's objects (MenuObject::scrap_team).
        for quadrant in [Quadrant::System, Quadrant::Defenses, Quadrant::Fleets] {
            let icon = MenuObject::SystemIcon {
                system: SystemKey::default(),
                quadrant,
            };
            assert!(enabled(&object_menu_rows(Some(icon), gates)).contains(&"Scrap"));
            assert!(!enabled(&object_menu_rows(Some(icon), OrderGates::default()))
                .contains(&"Scrap"));
        }
    }

    #[test]
    fn an_icons_scrap_team_is_its_kinds_objects_and_another_selection_its_own() {
        // FUN_00512700 by the item's kind (4, 8, 0x10, 0x40); A0 menus3/08,
        // 12 and 13.
        let mut world = rebellion_core::world::GameWorld::default();
        let system = world.systems.insert(rebellion_core::world::System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
            name: "Coruscant".into(),
            sector: rebellion_core::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
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
            control: rebellion_core::world::ControlKind::Uncontrolled,
        });
        let troop = world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: rebellion_core::ids::DatId::new(0x1000_0001),
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(troop);
        let yard = world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2800_0001),
                side: rebellion_core::dat::Faction::Alliance,
                is_shipyard: false,
            },
        );
        world.systems[system].manufacturing_facilities.push(yard);
        let team = |quadrant| {
            MenuObject::SystemIcon { system, quadrant }
                .scrap_team(
                    &rebellion_core::movement::MovementState::new(),
                    &world,
                    rebellion_core::dat::Faction::Alliance,
                )
        };
        assert_eq!(team(Quadrant::Defenses), [ScrapTarget::Troop(troop)]);
        assert_eq!(
            team(Quadrant::System),
            [ScrapTarget::Facility(
                rebellion_core::world::FacilityRef::Manufacturing(yard)
            )]
        );
        assert!(team(Quadrant::Fleets).is_empty());
        assert!(team(Quadrant::Missions).is_empty());
        assert_eq!(
            MenuObject::Troop(troop).scrap_team(
                    &rebellion_core::movement::MovementState::new(),
                    &world,
                    rebellion_core::dat::Faction::Alliance,
                ),
            [ScrapTarget::Troop(troop)]
        );
        assert_eq!(
            Quadrant::ALL.map(Quadrant::kind),
            [4, 8, 0x10, 0x40]
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
    fn a_submenu_parent_is_enabled_only_while_one_of_its_rows_is() {
        // hyp from A0: Planetary Bombardment is grey with every target
        // disabled (status/05-fleet-menu.png, menus/08-fleet-icon-menu.png)
        // and white beside an enabled one; Command likewise.
        let fleet = Some(MenuObject::Fleet(FleetKey::default()));
        let bombard = |index: usize| OrderGates {
            bombard: std::array::from_fn(|at| at == index),
            ..OrderGates::default()
        };
        for index in 0..4 {
            assert!(
                enabled(&object_menu_rows(fleet, bombard(index)))
                    .contains(&"Planetary Bombardment"),
                "{index}"
            );
        }
        assert!(!enabled(&object_menu_rows(fleet, OrderGates::default()))
            .contains(&"Planetary Bombardment"));
        let character = Some(MenuObject::Character(CharacterKey::default()));
        let command = |index: usize| OrderGates {
            command: std::array::from_fn(|at| at == index),
            ..OrderGates::default()
        };
        for index in 0..4 {
            assert!(
                enabled(&object_menu_rows(character, command(index))).contains(&"Command"),
                "{index}"
            );
        }
        assert!(!enabled(&object_menu_rows(character, OrderGates::default())).contains(&"Command"));
    }

    #[test]
    fn a_row_becomes_a_menu_entry_without_an_icon() {
        // The character records carry no icon (words 7 and 8 are zero).
        let rows = object_menu_rows(
            Some(MenuObject::Character(CharacterKey::default())),
            OrderGates {
                command: [true, false, false, false],
                ..MISSION_GATE
            },
        );
        let command = rows.iter().find(|row| row.item.label == "Command").unwrap();
        assert_eq!(
            command.entry(),
            GameMenuEntry {
                label: "Command",
                icon: None,
                enabled: true,
                submenu: true,
                check: false,
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
        // Each row keeps its gate; past the last row nothing is enabled.
        assert_eq!(
            (0..5).map(|row| menu.submenu_row_enabled(row)).collect::<Vec<_>>(),
            [true, false, true, false, false]
        );
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
