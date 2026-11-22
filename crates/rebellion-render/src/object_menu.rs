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
    /// (`0x1c..0x1f`, `+0x50` bit 2 clear), by its class, and its product
    /// (`rebellion_core::products`).
    PendingFighter {
        system: SystemKey,
        class: FighterKey,
        product: Option<u64>,
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
    /// `+0x50` bit 2 clear), by its class, and its product
    /// (`rebellion_core::products`).
    PendingTroop {
        system: SystemKey,
        class: rebellion_core::ids::DatId,
        product: Option<u64>,
    },
    /// An unfinished capital ship in a Fleet window, by its product
    /// (`rebellion_core::products`; `0x14..0x1b`, `+0x50` bit 2 clear).
    PendingShip { product: u64 },
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
            | Self::PendingShip { .. }
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
            | Self::PendingShip { .. }
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
            // A spare holding only unfinished ships takes no Scrap (A0
            // 2026-11-15 "u-2": Fleet 4's Scrap grey).
            Self::Fleet(fleet) if rebellion_core::products::is_staged(world, fleet) => {
                Vec::new()
            }
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
            Self::PendingFighter { system, class, .. } => {
                Some(StatusObject::PendingFighter { system, class })
            }
            Self::TravellingFighter { to, class, arrival } => {
                Some(StatusObject::TravellingFighter { to, class, arrival })
            }
            Self::PendingTroop { system, class, .. } => {
                Some(StatusObject::PendingTroop { system, class })
            }
            Self::PendingShip { product } => Some(StatusObject::PendingShip { product }),
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
    // An unfinished regiment or squadron keeps its Move (A0 2026-11-15
    // "u-3": the unfinished TIE's Move white; its vt74 asks nothing of
    // Completed, unfinished-products.md).
    let troop = matches!(
        selection,
        Some(
            MenuObject::Troop(_)
                | MenuObject::PendingTroop {
                    product: Some(_),
                    ..
                }
        )
    );
    let squadron = matches!(
        selection,
        Some(
            MenuObject::Fighter { .. }
                | MenuObject::PlanetFighter { .. }
                | MenuObject::PendingFighter {
                    product: Some(_),
                    ..
                }
        )
    );
    let fleet_entry = matches!(selection, Some(MenuObject::Fleet(_)));
    let character = matches!(selection, Some(MenuObject::Character(_)));
    let special_force = matches!(selection, Some(MenuObject::SpecialForce(_)));
    let ship = matches!(selection, Some(MenuObject::Ship { .. }));
    // An unfinished ship keeps Move, Confirmed Move and Rename (A0
    // 2026-11-15 "u-2": white), not Create Fleet or Scrap.
    let pending_ship = matches!(selection, Some(MenuObject::PendingShip { .. }));
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
        // target; its Move follows its gate (A0 2026-11-15 "u-3").
        Some(
            MenuObject::Troop(_)
            | MenuObject::PendingTroop { .. }
            | MenuObject::ViewedTroop { .. },
        ) => &[MOVE, CONFIRMED_MOVE, SCRAP],
        Some(MenuObject::Ship { .. } | MenuObject::PendingShip { .. }) => {
            &[MOVE, CONFIRMED_MOVE, SCRAP, RENAME, CREATE_FLEET]
        }
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
        // Scrap names no target, so it is grey, and its Move follows its
        // gate (A0 2026-11-15 "u-3": white).
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
                        || ((ship || pending_ship) && gates.ship_move)
                        || (squadron && gates.squadron_move)
                        || (headquarters && gates.headquarters_move)
                        || (defenses && gates.defenses_move)
                }
                ObjectMenuCommand::CreateFleet => ship && gates.ship_move,
                ObjectMenuCommand::Rename => (fleet_entry || ship || pending_ship) && gates.rename,
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
                        || ((ship || pending_ship) && gates.ship_move)
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
#[path = "object_menu_tests.rs"]
mod tests;
