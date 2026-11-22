//! The original Fleet window (window type 4, `FUN_004a2630`): a system's
//! fleets on the left, the selected fleet's contents under four tabs on the
//! right (manual Fig. 3.65, p. 122). Recovery notes:
//! `ghidra/notes/fleet-window.md`.
//!
//! It opens from the fleet icon a sector window shows at the top right of a
//! planet (`FUN_0045ccc0`, `FUN_0045aac0`), and a move released over it
//! targets the fleet under the point (`+0x70`, `FUN_004a3130`), which is how
//! a regiment boards a fleet. A regiment dragged out of the Troops tab moves
//! to where it is dropped (`0x201`, `ghidra/notes/regiment-unload.md`).

use egui_macroquad::egui;
use rebellion_core::dat::Faction;
use rebellion_core::fleet_join;
use rebellion_core::fog::FogState;
use rebellion_core::ids::{CharacterKey, DatId, FleetKey, SpecialForceKey, SystemKey, TroopKey};
use rebellion_core::missions::{MissionMember, MissionTarget, MissionTargetPick};
use rebellion_core::movement::MovementState;
use rebellion_core::squadrons::SquadronPlace;
use rebellion_core::troop_transport::{RegimentLeg, TroopTransportState};
use rebellion_core::world::{CapitalShipClass, ControlKind, GameWorld, ShipInstance};

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::drag_list::{ListSelection, Modifiers};
use crate::object_menu::MenuObject;
use crate::minis::{capital_ship_mini_id, fighter_mini_id};
use crate::quadrant_icons::{quadrant_art, Quadrant};
use crate::system_window::{
    canvas_point, character_picture, clamp_window_to_galaxy, exact_clicked, fleet_label,
    logical_rect, opposing_contents_visible, rect_contains, troop_mini,
};
use crate::targeting::ReleaseTarget;
use crate::window_move::{self, MoveAnchor};

pub const FLEET_WINDOW_WIDTH: f32 = 235.0;
pub const FLEET_WINDOW_HEIGHT: f32 = 304.0;

const BACKGROUND: u32 = 10770;
const CLOSE_NORMAL: u32 = 10108;
const CLOSE_PRESSED: u32 = 10109;
/// 10108's and 10109's key colour, their first DIB pixel.
const CLOSE_KEY: egui::Color32 = egui::Color32::from_rgb(0, 255, 0);
const MINIMIZE_NORMAL: u32 = 10253;
const MINIMIZE_PRESSED: u32 = 10254;
const SECTOR_NORMAL: u32 = 10209;
const SECTOR_PRESSED: u32 = 10208;
/// Alliance art; the Empire's is always 50 higher (`(side != 1) * 0x32`).
const EMPIRE_ART_OFFSET: u32 = 50;
const FLEET_FRAME: u32 = 10400;
const FLEET_SELECTED: u32 = 10401;
const PANE_ART: u32 = 10407;
const TAB_BASE: u32 = 10409;
const RIGHT_ITEM_SELECTED: u32 = 10420;
const FLEET_PICTURE: u32 = 10425;
/// An en route fleet's overlays (`+0x50` bit 4): 10423 over its entry
/// (`FUN_004a37c0`) and 10426 over its picture (`FUN_004a5c00`).
const EN_ROUTE_ENTRY: u32 = 10423;
const EN_ROUTE_PICTURE: u32 = 10426;
/// A damaged fleet's (`+0x50` bit `0x200`) marks: 10424 under its entry
/// (`FUN_004a37c0`) and 10427 under its picture (`FUN_004a5c00`).
const DAMAGED_ENTRY: u32 = 10424;
const DAMAGED_PICTURE: u32 = 10427;

/// Whether a fleet is damaged: `FUN_004feab0` sets its bit `0x200` while
/// any of its capital ships has it (`FUN_00501c00`).
pub(crate) fn fleet_damaged(world: &GameWorld, fleet: FleetKey) -> bool {
    world.fleets.get(fleet).is_some_and(|value| {
        value.capital_ships.iter().filter(|ship| ship.alive).any(|ship| {
            world
                .capital_ship_classes
                .get(ship.class)
                .is_some_and(|class| crate::status_rows::damaged(ship, class))
        })
    })
}

/// `FUN_004a66a0` flag `0x40` (10430 at x 9, `FUN_004a67a0`): the ship's
/// speed, slot `+0x34(1)` (`FUN_00500820`), is 0. That is its hyperdrive
/// less its hits, else its backup's, else the GNPRTB 1 default only while
/// its DeploymentCount (`+0x41`) is 0; `FUN_004fb310` raises the count on
/// each move to a new container. hyp: a ship in a fleet has moved, so it
/// takes no default. A0 `fleet2/umgul-fleet1-damaged`: Medium Transport 1,
/// Hyperdrive Rating 0:100 after the battle at Umgul, carries the mark.
fn stranded(class: &CapitalShipClass, ship: &ShipInstance) -> bool {
    rebellion_core::ship_condition::hyperdrive(class, ship) == 0
}

/// `Fleet_vt34` (`FUN_004fd900`) is 0 once one of its ships' speeds is, so
/// the fleet's entry carries the mark too.
fn fleet_stranded(world: &GameWorld, fleet: FleetKey) -> bool {
    world.fleets.get(fleet).is_some_and(|value| {
        value.capital_ships.iter().filter(|ship| ship.alive).any(|ship| {
            world
                .capital_ship_classes
                .get(ship.class)
                .is_some_and(|class| stranded(class, ship))
        })
    })
}

/// A fleet entry's layers bottom to top (`FUN_004a37c0`): the en route
/// mark, the damage mark, then the entry's frame keyed over them.
fn fleet_entry_layers(side: u8, en_route: bool, damaged: bool) -> Vec<u32> {
    [
        (en_route, EN_ROUTE_ENTRY),
        (damaged, DAMAGED_ENTRY),
        (true, FLEET_FRAME),
    ]
    .into_iter()
    .filter(|&(shown, _)| shown)
    .map(|(_, id)| side_art(id, side))
    .collect()
}

/// `FUN_004a66a0`'s content flags of an object: 1 squadrons, 2 regiments
/// and 4 personnel aboard (`FUN_005039d0`, `FUN_00504c40`,
/// `FUN_00536da0` over the object's contents).
const CONTENT_SQUADRONS: u8 = 1;
const CONTENT_REGIMENTS: u8 = 2;
const CONTENT_PERSONNEL: u8 = 4;
/// The indicators `FUN_004a67a0` blits for those flags: STRATEGY 10404,
/// 10405 and 10406 (`0x28a4`..`0x28a6`, +50 for side 2) at x 25, 41 and
/// 57, and y 33 in the left list or 23 in the right.
const CONTENT_INDICATORS: [(u8, u32, f32); 3] = [
    (CONTENT_SQUADRONS, 10_404, 25.0),
    (CONTENT_REGIMENTS, 10_405, 41.0),
    (CONTENT_PERSONNEL, 10_406, 57.0),
];

/// Each capital ship's content flags, by index in `fleet`.
///
/// port: the port keeps a fleet's regiments and squadrons as the fleet's;
/// hyp: they fill its live ships' capacities (`+0x270`, `+0x26c`) in fleet
/// order, so A0's Fleet 1 shows its two regiments under Medium Transport 1
/// (capacity 2) and none under Corellian Corvette 1 (capacity 0)
/// (`.artifacts/a0-vm/menus/23-fleet1-ships.png`). A character marks the
/// ship it rides on (`Character::ship`), or, unseated, the first living
/// numbered ship, where `missions::seat_characters` would seat it; hyp: a
/// special force, which keeps no seat, marks that first ship too. A0
/// `fleet2/vjun-fleet5-admiral-ships`: Garm Bel Iblis aboard Fleet 5 marks
/// Corellian Corvette 2.
///
/// `FUN_004a66a0` walks the ship's children in mode 3, so the units on
/// their way in mark the ship they were seated on ([`ship_load`]).
fn ship_contents(
    world: &GameWorld,
    transport: &TroopTransportState,
    arrivals: &[(MissionMember, FleetKey)],
    fleet: FleetKey,
) -> Vec<u8> {
    ship_load(world, transport, arrivals, fleet)
        .iter()
        .map(|cargo| {
            let mut flags = 0;
            if cargo.squadrons > 0 || !cargo.incoming_squadrons.is_empty() {
                flags |= CONTENT_SQUADRONS;
            }
            if !cargo.regiments.is_empty() {
                flags |= CONTENT_REGIMENTS;
            }
            if !cargo.characters.is_empty() || !cargo.special_forces.is_empty() {
                flags |= CONTENT_PERSONNEL;
            }
            flags
        })
        .collect()
}

/// What one capital ship carries (see [`ship_contents`] for where the port
/// seats each unit).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ShipCargo {
    pub characters: Vec<CharacterKey>,
    pub special_forces: Vec<SpecialForceKey>,
    pub regiments: Vec<TroopKey>,
    pub squadrons: usize,
    /// The squadrons on their way in seated here, each by its place among
    /// the fleet's incoming squadrons ([`incoming_squadrons`]).
    pub incoming_squadrons: Vec<usize>,
}

/// Each capital ship's cargo, by index in `fleet`, seated as
/// [`ship_contents`] describes.
pub(crate) fn ship_cargo(
    world: &GameWorld,
    transport: &TroopTransportState,
    fleet: FleetKey,
) -> Vec<ShipCargo> {
    let Some(value) = world.fleets.get(fleet) else {
        return Vec::new();
    };
    let mut regiments = transport.cargo(fleet).iter().copied();
    let mut squadrons: usize = value.fighters.iter().map(|entry| entry.count as usize).sum();
    let mut cargo = value
        .capital_ships
        .iter()
        .map(|ship| {
            let Some(class) = world.capital_ship_classes.get(ship.class).filter(|_| ship.alive) else {
                return ShipCargo::default();
            };
            let aboard = squadrons.min(class.fighter_capacity as usize);
            squadrons -= aboard;
            ShipCargo {
                regiments: regiments
                    .by_ref()
                    .take(class.troop_capacity as usize)
                    .collect(),
                squadrons: aboard,
                ..ShipCargo::default()
            }
        })
        .collect::<Vec<_>>();
    let first = first_seat(world, fleet);
    for &key in &value.characters {
        let seat = world
            .characters
            .get(key)
            .and_then(|c| c.ship)
            .and_then(|id| value.capital_ships.iter().position(|ship| ship.id == id))
            .or(first);
        if let Some(each) = seat.and_then(|index| cargo.get_mut(index)) {
            each.characters.push(key);
        }
    }
    for &key in &value.special_forces {
        if let Some(each) = first.and_then(|index| cargo.get_mut(index)) {
            each.special_forces.push(key);
        }
    }
    cargo
}

/// [`ship_cargo`] with the units on their way into `fleet` seated too, as
/// the Fleet window's walks find them (mode 3 counts a ship's children in
/// hyperspace).
///
/// A Move onto a fleet places each member on one of its ships
/// (`FUN_00552300`, fleet destination `0x08..0x0f`): it tries the fleet's
/// usable ships of the side (`FUN_00502e30(fleet, 1, side)`) in child
/// order through `FUN_00552b10`, which keeps a later candidate only when
/// `FUN_0054fbb0` finds it strictly better: not refused, then unlimited
/// room, then more room left after the move (`+8`, `FUN_0054fca0`: the
/// room `FUN_00500b40` less the team's count). So a regiment or squadron
/// boards the ship with the most room left, the first of equals, and the
/// room already counts the units on their way (`FUN_00500b40` subtracts
/// the mode-3 count). Characters and special forces have unlimited room
/// (`0x30..0x40`), so they board the first usable ship. port: the seats are
/// recomputed from the port's own seating each frame, in departure order,
/// and the first ship is [`ship_cargo`]'s first living numbered one.
pub(crate) fn ship_load(
    world: &GameWorld,
    transport: &TroopTransportState,
    arrivals: &[(MissionMember, FleetKey)],
    fleet: FleetKey,
) -> Vec<ShipCargo> {
    let mut cargo = ship_cargo(world, transport, fleet);
    let Some(value) = world.fleets.get(fleet) else {
        return cargo;
    };
    let capacity = |pick: fn(&CapitalShipClass) -> u32| -> Vec<Option<i64>> {
        value
            .capital_ships
            .iter()
            .map(|ship| {
                world
                    .capital_ship_classes
                    .get(ship.class)
                    .filter(|_| ship.alive)
                    .map(|class| i64::from(pick(class)))
            })
            .collect()
    };
    // The ship with the most room left, the first of equals.
    let roomiest = |room: &[Option<i64>], taken: &dyn Fn(&ShipCargo) -> usize, cargo: &[ShipCargo]| {
        room.iter()
            .zip(cargo)
            .enumerate()
            .filter_map(|(index, (room, each))| {
                Some((index, (*room)? - i64::try_from(taken(each)).unwrap_or(i64::MAX)))
            })
            .fold(None, |best: Option<(usize, i64)>, (index, left)| match best {
                Some((_, most)) if most >= left => best,
                _ => Some((index, left)),
            })
            .map(|(index, _)| index)
    };
    let troops = capacity(|class| class.troop_capacity);
    for troop in incoming_regiments(transport, fleet) {
        if let Some(index) = roomiest(&troops, &|each| each.regiments.len(), &cargo) {
            cargo[index].regiments.push(troop);
        }
    }
    let hangars = capacity(|class| class.fighter_capacity);
    for ordinal in 0..incoming_squadrons(transport, fleet).count() {
        let taken = |each: &ShipCargo| each.squadrons + each.incoming_squadrons.len();
        if let Some(index) = roomiest(&hangars, &taken, &cargo) {
            cargo[index].incoming_squadrons.push(ordinal);
        }
    }
    if let Some(each) = first_seat(world, fleet).and_then(|index| cargo.get_mut(index)) {
        for member in bound_members(world, arrivals, fleet) {
            match member {
                MissionMember::Character(key) => each.characters.push(key),
                MissionMember::SpecialForce(key) => each.special_forces.push(key),
            }
        }
    }
    cargo
}

/// The first living numbered capital ship of `fleet`, by index: where the
/// port seats a character with no seat, a special force, and a member on
/// its way in ([`ship_cargo`], [`ship_load`]).
pub(crate) fn first_seat(world: &GameWorld, fleet: FleetKey) -> Option<usize> {
    world
        .fleets
        .get(fleet)?
        .capital_ships
        .iter()
        .position(|ship| ship.id != 0 && ship.alive)
}

/// A unit's container aboard `fleet` as a Status window names it: the
/// ship at `seat`. `FUN_0044a2e0` writes the container's name
/// (`FUN_004f62d0`), and a unit in a fleet is a capital ship's child, never
/// the fleet's (A0 2026-11-14-missions-transfer/36 "Medium Transport 1",
/// 2026-11-14-quadrants-status-known/21, 22, 32, 33 "Alliance Escort
/// Carrier 1" and "3"). port: a unit the port seats on no ship (one past
/// the room, or in a fleet whose ships are not yet numbered) names the
/// fleet's first living ship, and a fleet with none its own name.
pub(crate) fn aboard_label(
    world: &GameWorld,
    fleet: FleetKey,
    seat: Option<usize>,
) -> Option<String> {
    let first_living = || {
        world
            .fleets
            .get(fleet)?
            .capital_ships
            .iter()
            .position(|ship| ship.alive)
    };
    seat.or_else(first_living)
        .and_then(|index| world.ship_name(fleet, index))
        .map(str::to_owned)
        .or_else(|| fleet_label(world, fleet))
}

/// Paint `flags`' content indicators into `item` at height `y`.
#[expect(
    clippy::too_many_arguments,
    reason = "The painter, art cache, the item's place and the indicators' row."
)]
fn paint_contents(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    item: egui::Rect,
    scale: f32,
    side: u8,
    flags: u8,
    y: f32,
) {
    for (flag, id, x) in CONTENT_INDICATORS {
        if flags & flag != 0 {
            paint_native(painter, ctx, cache, DllSource::Strategy, side_art(id, side), item, scale, x, y);
        }
    }
}

/// A fleet picture's layers bottom to top (`FUN_004a5c00`).
fn fleet_picture_layers(side: u8, en_route: bool, damaged: bool) -> Vec<u32> {
    [
        (en_route, EN_ROUTE_PICTURE),
        (damaged, DAMAGED_PICTURE),
        (true, FLEET_PICTURE),
    ]
    .into_iter()
    .filter(|&(shown, _)| shown)
    .map(|(_, id)| side_art(id, side))
    .collect()
}
const NO_HYPERDRIVE: u32 = 10430;
/// STRATEGY 11501 (`0x2ced`): the en route mark of a character's mini.
const EN_ROUTE_PERSONNEL: u32 = 11501;
/// STRATEGY 11515 (`0x2cfb`): the en route mark of a regiment's mini.
const EN_ROUTE_REGIMENT: u32 = 11515;

/// What a mini stands for, as `FUN_0042c3b0` picks its en route mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MiniObject {
    /// A capital ship or squadron.
    Craft,
    Regiment(DatId),
    Character,
    SpecialForce(DatId),
}

/// The mark `FUN_0042c3b0(.., 0, 1)` draws for an en route object's `mini`
/// (`+0x50` bit 4 set, bit 3 clear), at the mini's origin, under the mini. Its default is the
/// object's own GOKRES mark `(class & 0xfff) + 0x5000`, the mini's id plus
/// `0x1000`; it stays for craft, regiment classes `0x10000002` and
/// `0x10000008`, and special force classes `0x3c000003` and `0x3c000005`.
/// Other regiments take 11515, and characters and other special forces
/// 11501. An object is en route while its container is (`FUN_004f8240`), so
/// everything aboard a travelling fleet carries its mark.
pub(crate) fn en_route_mark(object: MiniObject, mini: u32) -> (DllSource, u32) {
    let own = (DllSource::Gokres, mini + 0x1000);
    match object {
        MiniObject::Craft => own,
        MiniObject::Regiment(class) => match class.raw() {
            0x1000_0002 | 0x1000_0008 => own,
            _ => (DllSource::Strategy, EN_ROUTE_REGIMENT),
        },
        MiniObject::Character => (DllSource::Strategy, EN_ROUTE_PERSONNEL),
        MiniObject::SpecialForce(class) => match class.raw() {
            0x3c00_0003 | 0x3c00_0005 => own,
            _ => (DllSource::Strategy, EN_ROUTE_PERSONNEL),
        },
    }
}

/// The mark `FUN_0042c3b0(.., 0, 1)` draws for a damaged object (`+0x50`
/// bit `0x200`): the object's own GOKRES mark `(class & 0xfff) + 0x6000`,
/// the mini's id plus `0x2000`, but STRATEGY 11502 (`0x2cee`) for
/// personnel (`0x30..0x3f`).
pub(crate) fn damage_mark(object: MiniObject, mini: u32) -> (DllSource, u32) {
    match object {
        MiniObject::Character | MiniObject::SpecialForce(_) => (DllSource::Strategy, DAMAGED_PERSONNEL),
        MiniObject::Craft | MiniObject::Regiment(_) => (DllSource::Gokres, mini + 0x2000),
    }
}

/// The mark `FUN_0042c3b0(.., 0, 1)` blits over a captive character's
/// mini (`+0xac` bit 0), after the mini: its GOKRES `(class & 0xfff) +
/// 0x7000`, the mini's id plus `0x3000`.
pub(crate) const fn captive_mark(mini: u32) -> (DllSource, u32) {
    (DllSource::Gokres, mini + 0x3000)
}

/// STRATEGY 11502 (`0x2cee`): the damage mark of a character's mini.
const DAMAGED_PERSONNEL: u32 = 11_502;

/// The backdrop `FUN_0042c3b0(.., 0, ..)` blits into a mini's bitmap
/// first, whatever its last argument: STRATEGY 11500 (`0x2cec`) for
/// personnel (`0x30..0x3f`), 11513 (`0x2cf9`) for a regiment.
pub(crate) const fn mini_backdrop(object: MiniObject) -> Option<(DllSource, u32)> {
    match object {
        MiniObject::Character | MiniObject::SpecialForce(_) => {
            Some((DllSource::Strategy, PERSONNEL_BACKDROP))
        }
        MiniObject::Regiment(_) => Some((DllSource::Strategy, REGIMENT_BACKDROP)),
        MiniObject::Craft => None,
    }
}

/// STRATEGY 11500 (`0x2cec`) and 11513 (`0x2cf9`): the minis' backdrops.
const PERSONNEL_BACKDROP: u32 = 11_500;
const REGIMENT_BACKDROP: u32 = 11_513;

/// What `FUN_0042c3b0`'s draws under a mini, bottom to top: its backdrop,
/// then the marks, en route (`+0x50` bit `0x10`) and damaged (bit
/// `0x200`). The object is blitted keyed over them last, so they show
/// through its matte.
pub(crate) fn mini_marks(
    object: MiniObject,
    mini: u32,
    en_route: bool,
    damaged: bool,
) -> Vec<(DllSource, u32)> {
    let mut marks: Vec<_> = mini_backdrop(object).into_iter().collect();
    if en_route {
        marks.push(en_route_mark(object, mini));
    }
    if damaged {
        marks.push(damage_mark(object, mini));
    }
    marks
}

/// The unfinished mark `FUN_0042c3b0(.., 0, 1)` lays under a mini whose
/// object is not completed (`+0x50` bit 2 clear): STRATEGY `0x2d32`
/// (11570) for side 1, else `0x2d34` (11572).
pub(crate) const fn unfinished_mini_mark(side_one: bool) -> (DllSource, u32) {
    (DllSource::Strategy, if side_one { 11_570 } else { 11_572 })
}

/// A craft's GOKRES portrait (`FUN_0042c3b0(.., 1, ..)`: `class & 0xfff`),
/// its mini less `0x4000`.
const fn ship_portrait(mini: u32) -> u32 {
    mini - 0x4000
}

/// The en route mark `FUN_0042c3b0(.., 1, 1)` draws over a craft's
/// portrait: its own GOKRES mark, `(class & 0xfff) + 0x1000`.
const fn en_route_portrait_mark(portrait: u32) -> u32 {
    portrait + 0x1000
}

/// The galaxy view rail's icon for a minimized Fleet window (`FUN_004a76e0`).
const RAIL_ICONS: [u32; 3] = [11536, 11537, 11538];

const LEFT_LIST: (f32, f32, f32, f32) = (4.0, 29.0, 91.0, 266.0);
const RIGHT_LIST: (f32, f32, f32, f32) = (101.0, 127.0, 133.0, 164.0);
const PICTURE_PANEL: (f32, f32, f32, f32) = (100.0, 42.0, 125.0, 49.0);
const TAB_STRIP: (f32, f32) = (99.0, 96.0);
/// The right pane's art at (97, 29) (`FUN_004a6980`'s `FUN_0060da10` offset).
const PANE_ORIGIN: (f32, f32) = (97.0, 29.0);
const TAB_X: [f32; 4] = [1.0, 33.0, 65.0, 97.0];
const TAB_SIZE: (f32, f32) = (31.0, 28.0);
const LEFT_ITEM_HEIGHT: f32 = 50.0;
const RIGHT_ITEM_HEIGHT: f32 = 50.0;
/// Each list's scroll bar (`FUN_004a4b10`: `FUN_0060a490(.., 0x29cb, ..)`):
/// STRATEGY RCDATA 10699, `[10, 10657, 10692, 10693, 10694, 10695, 10696,
/// 10697, 10698, 10684]`. A0 fleet-aboard/10 shows it at the right list's
/// right edge, window x 224..233 from y 127, for six squadrons.
const LIST_BAR_ART: crate::list_scroll_bar::BarArt = crate::list_scroll_bar::BarArt {
    width: 10,
    track: 10_657,
    up: [10_692, 10_693],
    down: [10_694, 10_695],
    top: 10_696,
    middle: 10_697,
    bottom: 10_698,
};
/// The lists' arrow step. hyp: `FUN_00607ea0`'s step argument, as the
/// Status window's list takes it (10); not read for this window.
const LIST_STEP: f32 = 10.0;

/// The right list's row under window-local `y` when the list shows from
/// pixel `scroll`.
fn right_row_at(y: f32, scroll: f32) -> usize {
    ((y - RIGHT_LIST.1 + scroll) / RIGHT_ITEM_HEIGHT) as usize
}

/// The right list's rows' height for `rows` rows.
fn right_content(rows: usize) -> f32 {
    RIGHT_ITEM_HEIGHT * rows as f32
}

/// Row `row`'s top when the list shows from pixel `scroll`, or `None` when
/// the row lies wholly outside the list.
fn right_row_top(row: usize, scroll: f32) -> Option<f32> {
    let top = RIGHT_LIST.1 + RIGHT_ITEM_HEIGHT * row as f32 - scroll;
    (top < RIGHT_LIST.1 + RIGHT_LIST.3 && top + RIGHT_ITEM_HEIGHT > RIGHT_LIST.1).then_some(top)
}

/// The layer a list's drag image paints on, over every window: the drag
/// draws on the frame window with its child clipping off (`FUN_0060dce0`).
pub(crate) fn drag_layer() -> egui::LayerId {
    egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("cool-drag-list-image"))
}

/// The right list's items as `CoolDragList` cells in list pixels: 125 by
/// 50, one under another (`FUN_004a4b10`).
fn right_cells(rows: usize) -> Vec<(usize, (f32, f32), (f32, f32))> {
    (0..rows)
        .map(|row| (row, (0.0, RIGHT_ITEM_HEIGHT * row as f32), (125.0, RIGHT_ITEM_HEIGHT)))
        .collect()
}

/// A right list item's unit: its object and how many items before it in
/// the list stand for the same object. The port's squadrons of one class
/// share their object, so the count tells them apart.
type RightKey = (ItemObject, usize);

/// Each right list row's [`RightKey`], in list order.
fn right_keys(items: &[RightItem]) -> Vec<Option<RightKey>> {
    let mut seen: Vec<ItemObject> = Vec::new();
    items
        .iter()
        .map(|item| {
            let object = item.object?;
            let nth = seen.iter().filter(|known| **known == object).count();
            seen.push(object);
            Some((object, nth))
        })
        .collect()
}

/// [`right_cells`] keyed by each row's unit.
fn right_key_cells(keys: &[Option<RightKey>]) -> Vec<(RightKey, (f32, f32), (f32, f32))> {
    right_cells(keys.len())
        .into_iter()
        .filter_map(|(row, origin, size)| Some((keys[row]?, origin, size)))
        .collect()
}

/// The rows of `keys` whose unit `selection` holds, in list order.
fn selected_rows(selection: &ListSelection<RightKey>, keys: &[Option<RightKey>]) -> Vec<usize> {
    keys.iter()
        .enumerate()
        .filter(|(_, key)| key.is_some_and(|key| selection.contains(key)))
        .map(|(row, _)| row)
        .collect()
}

/// The right list's row under list point `at`, among `rows` rows.
fn right_hit(at: (f32, f32), rows: usize) -> Option<usize> {
    (at.0 >= 0.0 && at.0 < RIGHT_LIST.2 && at.1 >= 0.0)
        .then(|| (at.1 / RIGHT_ITEM_HEIGHT) as usize)
        .filter(|row| *row < rows)
}

/// The drop of the left list's dragged `objects` (`0x29a`): the galaxy view
/// issues one `0x201`, or `0x202` when Ctrl is down at the release
/// (`confirmed`, `GetAsyncKeyState(VK_CONTROL)` in `FUN_00422ce0`, window
/// types 4 and 10 alike), for the whole selection. Its fleets move as one
/// fleet move. port: the ships among them move as one ship order per
/// fleet, each its own order.
fn left_drop(objects: &[ItemObject], point: egui::Pos2, confirmed: bool) -> Vec<FleetWindowAction> {
    let fleets: Vec<FleetKey> = objects
        .iter()
        .filter_map(|object| match *object {
            ItemObject::Fleet(fleet) => Some(fleet),
            _ => None,
        })
        .collect();
    let mut actions = Vec::new();
    if !fleets.is_empty() {
        actions.push(FleetWindowAction::DragFleet {
            fleets,
            point,
            confirmed,
        });
    }
    actions.extend(ship_drops(objects, point, confirmed));
    actions.extend(product_drops(objects, point, confirmed));
    actions
}

/// The dragged unfinished units' Move (`0x201`), one order for them all.
fn product_drops(objects: &[ItemObject], point: egui::Pos2, confirmed: bool) -> Vec<FleetWindowAction> {
    let products: Vec<u64> = objects
        .iter()
        .filter_map(|object| match *object {
            ItemObject::Product(product)
            | ItemObject::ProductFighter { product, .. }
            | ItemObject::ProductTroop { product, .. } => Some(product),
            _ => None,
        })
        .collect();
    if products.is_empty() {
        return Vec::new();
    }
    vec![FleetWindowAction::DragProducts {
        products,
        point,
        confirmed,
    }]
}

/// One ship order per fleet among `objects`' ships, in their order.
fn ship_drops(objects: &[ItemObject], point: egui::Pos2, confirmed: bool) -> Vec<FleetWindowAction> {
    let mut actions: Vec<FleetWindowAction> = Vec::new();
    for object in objects {
        let ItemObject::Ship {
            fleet,
            index,
            roster,
        } = *object
        else {
            continue;
        };
        let known = actions.iter_mut().find_map(|action| match action {
            FleetWindowAction::DragShip {
                fleet: of, ships, ..
            } if *of == fleet => Some(ships),
            _ => None,
        });
        match known {
            Some(ships) => ships.push(index),
            None => actions.push(FleetWindowAction::DragShip {
                fleet,
                ships: vec![index],
                roster,
                point,
                confirmed,
            }),
        }
    }
    actions
}

/// The drop of the right list's selected `objects`, all of one tab
/// (`0x29a`): the galaxy view moves them as one order (`FUN_00422ce0`),
/// `0x202` with Ctrl down (`confirmed`). port: ships and squadrons of
/// several fleets (several left entries selected) move as one order per
/// fleet.
fn right_drop(
    objects: &[ItemObject],
    point: egui::Pos2,
    confirmed: bool,
) -> Vec<FleetWindowAction> {
    let Some(first) = objects.first() else {
        return Vec::new();
    };
    match *first {
        ItemObject::Ship { .. } => ship_drops(objects, point, confirmed),
        ItemObject::Fighter { .. } => {
            let mut actions: Vec<FleetWindowAction> = Vec::new();
            for object in objects {
                let ItemObject::Fighter { fleet, index } = *object else {
                    continue;
                };
                let known = actions.iter_mut().find_map(|action| match action {
                    FleetWindowAction::DragSquadron {
                        fleet: of, indices, ..
                    } if *of == fleet => Some(indices),
                    _ => None,
                });
                match known {
                    Some(indices) => indices.push(index),
                    None => actions.push(FleetWindowAction::DragSquadron {
                        fleet,
                        indices: vec![index],
                        point,
                        confirmed,
                    }),
                }
            }
            actions
        }
        ItemObject::Regiment(_) => vec![FleetWindowAction::DragRegiment {
            troops: objects
                .iter()
                .filter_map(|object| match *object {
                    ItemObject::Regiment(troop) => Some(troop),
                    _ => None,
                })
                .collect(),
            point,
            confirmed,
        }],
        ItemObject::Character(_) | ItemObject::SpecialForce(_) => {
            vec![FleetWindowAction::DragMembers {
                members: objects
                    .iter()
                    .filter_map(|object| match *object {
                        ItemObject::Character(key) => Some(MissionMember::Character(key)),
                        ItemObject::SpecialForce(key) => Some(MissionMember::SpecialForce(key)),
                        _ => None,
                    })
                    .collect(),
                point,
                confirmed,
            }]
        }
        // An unfinished unit dragged: its Move (A0 2026-11-15 "m-3": the
        // Carrack dropped on Fleet 3; "m-5": the regiment likewise).
        ItemObject::Product(_)
        | ItemObject::ProductFighter { .. }
        | ItemObject::ProductTroop { .. } => product_drops(objects, point, confirmed),
        ItemObject::Fleet(_) | ItemObject::TravellingFighter { .. } => Vec::new()
    }
}

/// A left list entry's object.
fn entry_object(world: &GameWorld, entry: FleetWindowEntry) -> ItemObject {
    match entry {
        FleetWindowEntry::Fleet(fleet) => ItemObject::Fleet(fleet),
        FleetWindowEntry::Ship { fleet, index } => ItemObject::Ship {
            fleet,
            index,
            roster: fleet_join::roster(world, fleet).unwrap_or_default(),
        },
        FleetWindowEntry::Product { product, .. } => ItemObject::Product(product),
        // port: a copy has no live object; its menu acts on none.
        FleetWindowEntry::Copy { .. } => ItemObject::Fleet(FleetKey::default()),
    }
}

/// The left list's row under window-local `y` when the list shows from
/// pixel `scroll` (the same bar, 266 high: `FUN_004a4b10`).
fn left_row_at(y: f32, scroll: f32) -> usize {
    ((y - LEFT_LIST.1 + scroll) / LEFT_ITEM_HEIGHT) as usize
}

/// The left list's entries' height for `rows` entries.
fn left_content(rows: usize) -> f32 {
    LEFT_ITEM_HEIGHT * rows as f32
}

/// Entry `row`'s top when the left list shows from pixel `scroll`, or
/// `None` when the entry lies wholly outside the list.
fn left_row_top(row: usize, scroll: f32) -> Option<f32> {
    let top = LEFT_LIST.1 + LEFT_ITEM_HEIGHT * row as f32 - scroll;
    (top < LEFT_LIST.1 + LEFT_LIST.3 && top + LEFT_ITEM_HEIGHT > LEFT_LIST.1).then_some(top)
}

/// The scroll state to store once a list's bar hides: back to the top,
/// or `None` when it is there already.
fn reset_when_hidden(
    scroll: f32,
    hold: Option<crate::list_scroll_bar::Hold>,
) -> Option<(f32, Option<crate::list_scroll_bar::Hold>)> {
    (scroll != 0.0 || hold.is_some()).then_some((0.0, None))
}

/// The four tabs (`0x66..0x69`), in strip order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FleetWindowTab {
    CapitalShips,
    Fighters,
    Troops,
    Personnel,
}

impl FleetWindowTab {
    pub const ALL: [Self; 4] = [
        Self::CapitalShips,
        Self::Fighters,
        Self::Troops,
        Self::Personnel,
    ];

    /// The tab's tooltip (`FUN_004a4b10`, TEXTSTRA): 0x3823 "Ships",
    /// 0x3820 "Fighter Squadrons", 0x3821 "Troop Regiments", 0x3822
    /// "Personnel" (A0 tooltip/142..145).
    #[must_use]
    pub const fn tooltip_string(self) -> u16 {
        match self {
            Self::CapitalShips => 0x3823,
            Self::Fighters => 0x3820,
            Self::Troops => 0x3821,
            Self::Personnel => 0x3822,
        }
    }

    const fn index(self) -> u32 {
        match self {
            Self::CapitalShips => 0,
            Self::Fighters => 1,
            Self::Troops => 2,
            Self::Personnel => 3,
        }
    }
}

/// A left-list entry: a fleet, or one of its capital ships while the fleet
/// is expanded (`FUN_004a37c0`, `FUN_004a3d40`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetWindowEntry {
    Fleet(FleetKey),
    Ship { fleet: FleetKey, index: usize },
    /// An unfinished ship of `fleet`, by its product
    /// (`rebellion_core::products`): `FUN_004a3d40` lists the fleet's ships
    /// in mode 3 (Existing).
    Product { fleet: FleetKey, product: u64 },
    /// The `fleet`th other-side fleet of the viewing side's (`alliance`)
    /// frozen copy of `system` (`SystemDetails`, `FUN_004f9a60`): what the
    /// side saw then, though the fleet may have moved or died since.
    Copy {
        alliance: bool,
        system: SystemKey,
        fleet: usize,
    },
}

impl FleetWindowEntry {
    /// The live fleet; none (the null key) for a copy.
    #[must_use]
    pub fn fleet(self) -> FleetKey {
        match self {
            Self::Fleet(fleet) | Self::Ship { fleet, .. } | Self::Product { fleet, .. } => fleet,
            Self::Copy { .. } => FleetKey::default(),
        }
    }

    /// The copied fleet a [`Self::Copy`] names.
    #[must_use]
    pub fn copy(self, world: &GameWorld) -> Option<&rebellion_core::object_views::FleetDetail> {
        let Self::Copy {
            alliance,
            system,
            fleet,
        } = self
        else {
            return None;
        };
        let viewer = Faction::of_alliance(alliance);
        let id = world.systems.get(system)?.dat_id;
        world
            .object_views
            .system_details(viewer, id)?
            .fleets
            .iter()
            .filter(|copy| copy.is_alliance != alliance)
            .nth(fleet)
    }
}

/// The viewing side's frozen copies of `system`'s other-side fleets, as
/// entries (`FUN_004f2d10`, `FUN_004f9a60`); none while the side sees the
/// system live.
pub(crate) fn copy_entries(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> Vec<FleetWindowEntry> {
    if crate::viewed_contents::frozen_extras(world, fog, player, system).is_none() {
        return Vec::new();
    }
    let alliance = player == Faction::Alliance;
    let Some(id) = world.systems.get(system).map(|value| value.dat_id) else {
        return Vec::new();
    };
    let count = world
        .object_views
        .system_details(player, id)
        .map_or(0, |details| {
            details
                .fleets
                .iter()
                .filter(|copy| copy.is_alliance != alliance)
                .count()
        });
    (0..count)
        .map(|fleet| FleetWindowEntry::Copy {
            alliance,
            system,
            fleet,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
struct OpenFleetWindow {
    system: SystemKey,
    logical_position: (i16, i16),
    /// The left list's selected entries, by the `CoolDragList` rules
    /// ([`crate::drag_list`]; `FUN_004a4b10` gives the list no flags, so
    /// several entries may be selected).
    selected: ListSelection<FleetWindowEntry>,
    /// The right list's selected items, by the `CoolDragList` rules
    /// ([`crate::drag_list`]). Each is kept by its unit, not its row: the
    /// original's items are the objects themselves (`+0x6c`), so a unit
    /// that lands, leaves or dies above another moves no selection.
    selected_items: ListSelection<RightKey>,
    expanded: Vec<FleetKey>,
    tab: FleetWindowTab,
    /// The in-place name edit Rename (0x203) opens (`FUN_004ac7a0`).
    rename: Option<RenameEdit>,
    /// Where in the left list, in window-local logical pixels, and when
    /// the last primary click landed; `None` after a click anywhere else
    /// or a double click ([`win32_double_click`]). port: egui counts a
    /// double click across widgets, and a third click as a triple, so the
    /// list keeps its own, as the Fleet Finder's does.
    last_click: Option<((i32, i32), f64)>,
    /// Where the window was when a press on its surface took it
    /// (`crate::window_move`).
    moving: MoveAnchor,
    /// The selected entries' fleets and their ship counts when last drawn:
    /// a change of a count drops the selection (see [`roster_changed`]).
    roster: Vec<(FleetKey, usize)>,
    /// The right list's first visible pixel and a press held on its bar.
    right_scroll: f32,
    right_hold: Option<crate::list_scroll_bar::Hold>,
    /// The left list's first visible pixel and a press held on its bar.
    /// A selection keeps it: a click's `0x29b` only posts `0x467` and
    /// refreshes the right pane (`FUN_004a6390`, `FUN_004a5c00`); a select
    /// from outside (slot `+0x6c`, `FUN_004a73c0`) relays the list out
    /// (`FUN_0060a280`), which only clamps it to the range (`FUN_00609ae0`).
    left_scroll: f32,
    left_hold: Option<crate::list_scroll_bar::Hold>,
}

impl OpenFleetWindow {
    /// Show `tab`. The right list keeps its first visible pixel: the
    /// strip's `0x6a`/`0x6b` (`FUN_004a6390`) refills the list through
    /// `FUN_004a5c00`, which empties its items (`FUN_005f5b20` on `+0x18c`)
    /// and lays it out (`FUN_0060a280` on `+0x164`), and the layout only
    /// clamps `+0xc4` to the range (`FUN_00609ae0`); the draw clamps it.
    fn show_tab(&mut self, tab: FleetWindowTab) {
        self.tab = tab;
        self.selected_items.clear();
    }
}

/// Whether a click at `now` completes a double click begun at `previous`:
/// Win32 posts `WM_LBUTTONDBLCLK` only when the second click lands in the
/// same window within the double-click time, inside the rectangle centred
/// on the first (`SM_CXDOUBLECLK` by `SM_CYDOUBLECLK`, 4 by 4 pixels by
/// default).
fn win32_double_click(
    previous: Option<((i32, i32), f64)>,
    (at, time): ((i32, i32), f64),
    delay: f64,
) -> bool {
    previous.is_some_and(|((x, y), then)| {
        (at.0 - x).abs() <= 2 && (at.1 - y).abs() <= 2 && time - then <= delay
    })
}

/// `point` in `local`'s logical pixels, as the original's window sees it.
#[allow(clippy::cast_possible_truncation)]
fn logical_point(local: egui::Rect, scale: f32, point: egui::Pos2) -> (i32, i32) {
    (
        ((point.x - local.min.x) / scale).floor() as i32,
        ((point.y - local.min.y) / scale).floor() as i32,
    )
}

/// A Rename's edit field (`+0x138`, a CoolStringField) over the entry it
/// renames, with the order pending at `+0x13c`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RenameEdit {
    entry: FleetWindowEntry,
    /// The right list's row the edit stands in, for a ship the selected
    /// fleet lists there; `None` puts it over the left entry.
    item: Option<usize>,
    text: String,
    /// The first frame selects the whole name and takes the focus
    /// (`FUN_00605110(field, 0, -1)`, `SetFocus`).
    fresh: bool,
}

/// What a frame did to an open rename edit.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RenameOutcome {
    Typed(String),
    Commit(String),
    Cancel,
}

/// `CoolDragList` posts `0x29a` only when the release lies more than this
/// squared distance, in list pixels, from the press (`FUN_006083c0`).
const DRAG_DISTANCE_SQUARED: f32 = 24.0;

/// A left press held on a list until its release: the list has captured
/// the mouse (`CoolDragList`, `FUN_006083c0`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct ItemDrag {
    system: SystemKey,
    /// The left list's pressed entry; `None` for a press on the right list,
    /// whose selection a drop carries.
    left: Option<ItemObject>,
    press: egui::Pos2,
}

/// A held drag's image (`FUN_00609960`): the selected items' images, moved
/// by `offset` with the pointer, from the left list or the right.
#[derive(Debug, Clone, Copy, PartialEq)]
struct DragImage {
    offset: egui::Vec2,
    /// The left list entry pressed, which the press selected
    /// (`FUN_006083c0`, case `0x201`); `None` for the right list.
    left: Option<ItemObject>,
}

impl DragImage {
    /// Whether the image holds left list `entry`.
    fn holds(self, entry: FleetWindowEntry) -> bool {
        match (self.left, entry) {
            (Some(ItemObject::Fleet(dragged)), FleetWindowEntry::Fleet(fleet)) => dragged == fleet,
            (
                Some(ItemObject::Ship { fleet: dragged, index: at, .. }),
                FleetWindowEntry::Ship { fleet, index },
            ) => dragged == fleet && at == index,
            _ => false,
        }
    }
}

/// How a held left press ended ([`FleetWindowState::end_drag`]).
#[derive(Debug, Clone, Copy, PartialEq)]
enum DragEnd {
    /// The release posted `0x29a`.
    Drop {
        system: SystemKey,
        left: Option<ItemObject>,
        point: egui::Pos2,
    },
    /// A right list release that is no drop: its held click applies there.
    Release { system: SystemKey, point: egui::Pos2 },
}

/// The object a list item stands for, which a drag carries and a right
/// click opens a menu for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemObject {
    /// A left-list fleet entry.
    Fleet(FleetKey),
    Regiment(TroopKey),
    /// A capital ship, by its index in the fleet's `capital_ships` and the
    /// fleet's roster then (`fleet_join::roster`).
    Ship {
        fleet: FleetKey,
        index: usize,
        roster: u64,
    },
    /// A squadron, by its entry's index in the fleet's `fighters`.
    Fighter { fleet: FleetKey, index: usize },
    /// A squadron of `class` on its way into `fleet` by Move, arriving on
    /// day `arrival`.
    TravellingFighter {
        fleet: FleetKey,
        class: rebellion_core::ids::FighterKey,
        arrival: u64,
    },
    /// A Personnel tab character (manual p. 111: drag a character onto a
    /// system or fleet; p. 56: right-click it for Command).
    Character(rebellion_core::ids::CharacterKey),
    /// A Personnel tab special force (`FUN_00536da0(ship, 3)` lists them
    /// with the characters).
    SpecialForce(rebellion_core::ids::SpecialForceKey),
    /// An unfinished capital ship of the fleet, by its product
    /// (`rebellion_core::products`): `FUN_004a6be0` walks the ships in
    /// mode 3 (Existing), so it lists (A0 2026-11-15 "u-2").
    Product(u64),
    /// An unfinished squadron aboard the fleet at `system`, by its product
    /// (the Fighters tab walks mode 3, `FUN_005039d0(ship, 3)`).
    ProductFighter {
        product: u64,
        system: SystemKey,
        class: rebellion_core::ids::FighterKey,
    },
    /// An unfinished regiment aboard the fleet at `system`, by its product
    /// (`FUN_00504c40(ship, 3)`; A0 2026-11-15 "m-5": the Army Regiment in
    /// Fleet 3's regiments tab, on the grid).
    ProductTroop {
        product: u64,
        system: SystemKey,
        class: DatId,
    },
}

impl ItemObject {
    const fn menu_object(self) -> MenuObject {
        match self {
            Self::Fighter { fleet, index } => MenuObject::Fighter { fleet, index },
            Self::TravellingFighter {
                fleet,
                class,
                arrival,
            } => MenuObject::TravellingFighter {
                to: SquadronPlace::Fleet(fleet),
                class,
                arrival,
            },
            Self::Fleet(fleet) => MenuObject::Fleet(fleet),
            Self::Regiment(troop) => MenuObject::Troop(troop),
            Self::Character(character) => MenuObject::Character(character),
            Self::SpecialForce(unit) => MenuObject::SpecialForce(unit),
            Self::Ship {
                fleet,
                index,
                roster,
            } => MenuObject::Ship {
                fleet,
                index,
                roster,
            },
            Self::Product(product) => MenuObject::PendingShip { product },
            Self::ProductFighter {
                product,
                system,
                class,
            } => MenuObject::PendingFighter {
                system,
                class,
                product: Some(product),
            },
            Self::ProductTroop {
                product,
                system,
                class,
            } => MenuObject::PendingTroop {
                system,
                class,
                product: Some(product),
            },
        }
    }
}

/// The open Fleet windows. The last is focused and paints on top.
#[derive(Debug)]
pub struct FleetWindowState {
    faction: CockpitFaction,
    windows: Vec<OpenFleetWindow>,
    /// The kind's claim on the shared window focus.
    focus: crate::window_focus::FocusStamp,
    /// Minimized windows (`0x466`): hidden, not destroyed, so one comes
    /// back with its selection and tab (A0 personnel-states/39, Pzob's
    /// window back with its Arrow selected). port: the rail's thirteenth
    /// window destroys the oldest's window (`FUN_00428b40`); a hidden one
    /// whose rail slot was taken stays here until its system's next open.
    hidden: Vec<OpenFleetWindow>,
    drag: Option<ItemDrag>,
    /// A window was created, not raised, since [`Self::take_created`]
    /// (`FUN_0045aac0` then calls `FUN_0041d2d0`).
    created: bool,
    /// The characters and special forces on their way into a fleet by Move
    /// (`quadrant_icons::fleet_arrivals`), which its Personnel tab lists.
    arrivals: Vec<(MissionMember, FleetKey)>,
}

impl Default for FleetWindowState {
    fn default() -> Self {
        Self {
            faction: CockpitFaction::Alliance,
            windows: Vec::new(),
            focus: crate::window_focus::FocusStamp::default(),
            hidden: Vec::new(),
            drag: None,
            created: false,
            arrivals: Vec::new(),
        }
    }
}

impl FleetWindowState {
    /// The members on their way into fleets this frame
    /// (`quadrant_icons::fleet_arrivals`).
    pub fn set_arrivals(&mut self, arrivals: Vec<(MissionMember, FleetKey)>) {
        self.arrivals = arrivals;
    }

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

    /// Open `system`'s Fleet window at a logical point, clamped into the
    /// galaxy view, or bring its open window to the front: one per system
    /// (`FUN_0045aac0`, id `(system & 0x3ff) << 6 | 4`).
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
        if let Some(index) = self.hidden.iter().position(|window| window.system == system) {
            let mut window = self.hidden.remove(index);
            window.moving = None;
            window.last_click = None;
            self.windows.push(window);
            self.focus.take();
            return true;
        }
        self.windows.push(OpenFleetWindow {
            system,
            logical_position: clamp_window_to_galaxy(
                logical_position,
                layout,
                FLEET_WINDOW_WIDTH,
                FLEET_WINDOW_HEIGHT,
            ),
            selected: ListSelection::default(),
            selected_items: ListSelection::default(),
            expanded: Vec::new(),
            tab: FleetWindowTab::CapitalShips,
            rename: None,
            last_click: None,
            moving: None,
            roster: Vec::new(),
            right_scroll: 0.0,
            right_hold: None,
            left_scroll: 0.0,
            left_hold: None,
        });
        self.focus.take();
        true
    }

    /// Select `entry` in `system`'s open window, expanding its fleet for a
    /// ship (slot `+0x6c`, as the Fleet Finder's open does, `FUN_00429440`).
    pub fn select(&mut self, system: SystemKey, entry: FleetWindowEntry) -> bool {
        let Some(window) = self.window_mut(system) else {
            return false;
        };
        if let FleetWindowEntry::Ship { fleet, .. } = entry {
            if !window.expanded.contains(&fleet) {
                window.expanded.push(fleet);
            }
        }
        window.selected = ListSelection::only(entry);
        window.selected_items.clear();
        window.tab = tab_for(&[entry], window.tab);
        // The right list keeps its place, clamped (`FUN_004a73c0` →
        // `FUN_0060d7e0` → `FUN_004a5c00`; see `OpenFleetWindow::show_tab`).
        true
    }

    /// Expand `fleet`'s entry in `system`'s open window, as a double click
    /// on a collapsed entry does.
    pub fn expand(&mut self, system: SystemKey, fleet: FleetKey) -> bool {
        let Some(window) = self.window_mut(system) else {
            return false;
        };
        if !window.expanded.contains(&fleet) {
            window.expanded.push(fleet);
        }
        true
    }

    /// Whether `fleet`'s entry is expanded in `system`'s open window.
    #[must_use]
    pub fn is_expanded(&self, system: SystemKey, fleet: FleetKey) -> bool {
        self.windows
            .iter()
            .any(|window| window.system == system && window.expanded.contains(&fleet))
    }

    /// Show `tab` in `system`'s open window, as a click on its button does.
    pub fn show_tab(&mut self, system: SystemKey, tab: FleetWindowTab) -> bool {
        let Some(window) = self.window_mut(system) else {
            return false;
        };
        window.tab = tab;
        window.selected_items.clear();
        true
    }

    /// Start Rename (0x203) on `entry` in `system`'s open window
    /// (`FUN_00429350` → `vtable+0x78`, `FUN_004ac7a0`): the entry is
    /// selected and an edit holding its name opens over it, all of it
    /// selected. A rename already open is dropped (`vtable+0x80`).
    pub fn begin_rename(
        &mut self,
        world: &GameWorld,
        system: SystemKey,
        entry: FleetWindowEntry,
    ) -> bool {
        let name = match entry {
            FleetWindowEntry::Fleet(fleet) => world.fleet_name(fleet),
            FleetWindowEntry::Ship { fleet, index } => world.ship_name(fleet, index),
            // FUN_004f6270: the product's own name, else its class's.
            FleetWindowEntry::Product { product, .. } => world.products.get(product).and_then(|value| {
                value.name.as_deref().or_else(|| match value.kind {
                    rebellion_core::manufacturing::BuildableKind::CapitalShip(class) => world
                        .capital_ship_classes
                        .get(class)
                        .map(|class| class.name.as_str()),
                    _ => None,
                })
            }),
            FleetWindowEntry::Copy { .. } => None,
        }
        .map(str::to_owned);
        let Some(text) = name else {
            return false;
        };
        // A0 menus3/41: a ship of the selected fleet is renamed in the
        // right list, under its picture, the fleet staying selected.
        let item = self.window_mut(system).and_then(|window| match entry {
            FleetWindowEntry::Ship { fleet, index }
                if window.selected.single() == Some(FleetWindowEntry::Fleet(fleet))
                    && window.tab == FleetWindowTab::CapitalShips =>
            {
                ship_item_row(world, fleet, index)
            }
            _ => None,
        });
        if item.is_none() && !self.select(system, entry) {
            return false;
        }
        for window in &mut self.windows {
            window.rename = None;
        }
        if let Some(window) = self.window_mut(system) {
            window.rename = Some(RenameEdit {
                entry,
                item,
                text,
                fresh: true,
            });
        }
        true
    }

    /// Whether a rename edit holds the keyboard.
    #[must_use]
    pub fn renaming(&self) -> bool {
        self.windows.iter().any(|window| window.rename.is_some())
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

    /// The screen rect of the `row`th left-list entry of `system`'s window.
    #[must_use]
    pub fn entry_screen_rect(
        &self,
        layout: CockpitLayout,
        system: SystemKey,
        row: usize,
    ) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(logical_rect(
            window_screen_rect(window, layout),
            layout.scale,
            LEFT_LIST.0,
            LEFT_LIST.1 + LEFT_ITEM_HEIGHT * row as f32 - window.left_scroll,
            LEFT_LIST.2,
            LEFT_ITEM_HEIGHT,
        ))
    }

    /// The screen rect of the `row`th right-list item of `system`'s window.
    #[must_use]
    pub fn item_screen_rect(
        &self,
        layout: CockpitLayout,
        system: SystemKey,
        row: usize,
    ) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(logical_rect(
            window_screen_rect(window, layout),
            layout.scale,
            RIGHT_LIST.0,
            RIGHT_LIST.1 + RIGHT_ITEM_HEIGHT * row as f32 - window.right_scroll,
            125.0,
            RIGHT_ITEM_HEIGHT,
        ))
    }

    /// The screen rect of `tab`'s button in `system`'s window.
    #[must_use]
    pub fn tab_screen_rect(
        &self,
        layout: CockpitLayout,
        system: SystemKey,
        tab: FleetWindowTab,
    ) -> Option<egui::Rect> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        Some(tab_rect(
            window_screen_rect(window, layout),
            layout.scale,
            tab,
        ))
    }

    /// The selected entry and tab of `system`'s window.
    #[must_use]
    pub fn selection(
        &self,
        system: SystemKey,
    ) -> Option<(Option<FleetWindowEntry>, FleetWindowTab)> {
        self.windows
            .iter()
            .find(|window| window.system == system)
            .map(|window| (window.selected.single(), window.tab))
    }

    /// What `system`'s window shows, for the interface fixture.
    #[must_use]
    pub fn report(
        &self,
        world: &GameWorld,
        movement: &MovementState,
        fog: &FogState,
        transport: &TroopTransportState,
        system: SystemKey,
    ) -> Option<FleetWindowReport> {
        let window = self.windows.iter().find(|window| window.system == system)?;
        let entries = left_entries(world, movement, fog, cockpit_faction(self.faction), window);
        let picked = picked_entries(window, &entries);
        let selected = match picked.as_slice() {
            [one] => Some(*one),
            _ => None,
        };
        Some(FleetWindowReport {
            origin: window.logical_position,
            entries: entries.len(),
            fleet_rows: entries
                .iter()
                .enumerate()
                .filter_map(|(row, entry)| match entry {
                    FleetWindowEntry::Fleet(fleet) => Some((*fleet, row)),
                    FleetWindowEntry::Ship { .. }
                    | FleetWindowEntry::Product { .. }
                    | FleetWindowEntry::Copy { .. } => None,
                })
                .collect(),
            selected,
            selected_rows: entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| picked.contains(entry))
                .map(|(row, _)| row)
                .collect(),
            tab: window.tab,
            enabled: FleetWindowTab::ALL
                .map(|tab| selection_tab_enabled(world, movement, transport, &self.arrivals, &picked, tab)),
            items: selection_items(world, movement, transport, &self.arrivals, &picked, window.tab)
                .into_iter()
                .map(|item| item.label)
                .collect(),
            selected_items: selected_rows(
                &window.selected_items,
                &right_keys(&selection_items(
                    world,
                    movement,
                    transport,
                    &self.arrivals,
                    &picked,
                    window.tab,
                )),
            ),
            counts: selection_counts(world, movement, transport, &picked, window.tab),
        })
    }

    /// The target a move released at `point` takes from the Fleet window egui
    /// draws as `layer` (`+0x70`, `FUN_004a3130`), or `None` when `layer` is
    /// none of them. The inner `None` is an empty target: the order is
    /// destroyed.
    #[must_use]
    pub fn release_target(
        &self,
        world: &GameWorld,
        movement: &MovementState,
        fog: &FogState,
        layout: CockpitLayout,
        layer: egui::LayerId,
        point: egui::Pos2,
    ) -> Option<Option<ReleaseTarget>> {
        let window = self
            .windows
            .iter()
            .find(|window| area_id(window.system) == layer.id)?;
        let rect = window_screen_rect(window, layout);
        let scale = layout.scale.max(f32::EPSILON);
        let local = (
            (point.x - rect.min.x) / scale,
            (point.y - rect.min.y) / scale,
        );
        let player = cockpit_faction(self.faction);
        let entries = left_entries(world, movement, fog, player, window);
        Some(drop_target(world, player, window, &entries, local))
    }

    /// The capital ship a release at `point` names, when the Fleet window
    /// egui draws as `layer` has a ship's entry under it: a ship of the
    /// left list, or a row of the right list's Capital Ships page, as the
    /// window's `+0x70` answers (`FUN_004a3130`). Its fleet and
    /// `ShipInstance::id`. A0 2026-11-21-build-dest-16/34: a troops band
    /// dropped on Victory Destroyer 2's entry names that ship.
    #[must_use]
    pub fn release_ship(
        &self,
        world: &GameWorld,
        movement: &MovementState,
        fog: &FogState,
        layout: CockpitLayout,
        layer: egui::LayerId,
        point: egui::Pos2,
    ) -> Option<(FleetKey, u64)> {
        let window = self
            .windows
            .iter()
            .find(|window| area_id(window.system) == layer.id)?;
        let rect = window_screen_rect(window, layout);
        let scale = layout.scale.max(f32::EPSILON);
        let local = (
            (point.x - rect.min.x) / scale,
            (point.y - rect.min.y) / scale,
        );
        let player = cockpit_faction(self.faction);
        let entries = left_entries(world, movement, fog, player, window);
        let id = |fleet: FleetKey, index: usize| {
            world
                .fleets
                .get(fleet)
                .and_then(|value| value.capital_ships.get(index))
                .map(|ship| (fleet, ship.id))
        };
        if in_rect(LEFT_LIST, local) {
            return match entries.get(left_row_at(local.1, window.left_scroll)) {
                Some(&FleetWindowEntry::Ship { fleet, index }) => id(fleet, index),
                _ => None,
            };
        }
        if window.tab == FleetWindowTab::CapitalShips && in_rect(RIGHT_LIST, local) {
            let mut row = right_row_at(local.1, window.right_scroll);
            let picked = picked_entries(window, &entries);
            for entry in &picked {
                let ships: Vec<usize> = match *entry {
                    FleetWindowEntry::Fleet(fleet) => {
                        world.fleets.get(fleet).map_or_else(Vec::new, |value| {
                            value
                                .capital_ships
                                .iter()
                                .enumerate()
                                .filter(|(_, ship)| ship.alive)
                                .map(|(index, _)| index)
                                .collect()
                        })
                    }
                    FleetWindowEntry::Ship { index, .. } if picked.len() > 1 => vec![index],
                    _ => Vec::new(),
                };
                if let Some(&index) = ships.get(row) {
                    return id(entry.fleet(), index);
                }
                row -= ships.len();
            }
        }
        None
    }

    /// What a Mission released at `point` takes from the Fleet window egui
    /// draws as `layer` (`+0x68`, `FUN_004a2f80`), or `None` when `layer` is
    /// none of them. The left list (4..95, 29..295) gives the entry under
    /// the point; the right list (101..234, 127..291) the item under the
    /// point, else its selected item; anywhere else, or over the lists'
    /// empty space, the window's subject.
    ///
    /// The inner `None` is an object no mission takes, so the order is
    /// dropped: a fleet (family `0x08..0x10`) fails every class's target
    /// family (the system rules want `0x90..0x98`, the character rules
    /// `0x30..0x3c`, the object rules `0x10..0x40`; `FUN_005868c0`,
    /// `FUN_00586e20`, `FUN_00593500`), so `FUN_005422f0` lists nothing and
    /// no dialog opens. port: the order is dropped at once instead of being
    /// handed back (`FUN_0041ce20`).
    #[expect(
        clippy::too_many_arguments,
        reason = "The lists read the world, the player's view and the fleets' cargo."
    )]
    #[must_use]
    pub fn mission_target(
        &self,
        world: &GameWorld,
        movement: &MovementState,
        fog: &FogState,
        transport: &TroopTransportState,
        layout: CockpitLayout,
        layer: egui::LayerId,
        point: egui::Pos2,
    ) -> Option<Option<MissionTargetPick>> {
        let window = self
            .windows
            .iter()
            .find(|window| area_id(window.system) == layer.id)?;
        let rect = window_screen_rect(window, layout);
        let scale = layout.scale.max(f32::EPSILON);
        let local = (
            (point.x - rect.min.x) / scale,
            (point.y - rect.min.y) / scale,
        );
        let target = if in_rect(LEFT_LIST, local) {
            let row = left_row_at(local.1, window.left_scroll);
            let player = cockpit_faction(self.faction);
            match left_entries(world, movement, fog, player, window).get(row) {
                Some(FleetWindowEntry::Ship { fleet, index }) => {
                    Some(ship_target(world, *fleet, *index).map(ItemTarget::Object))
                }
                Some(
                    FleetWindowEntry::Fleet(_)
                    | FleetWindowEntry::Product { .. }
                    | FleetWindowEntry::Copy { .. },
                ) => Some(None),
                None => None,
            }
        } else if in_rect(RIGHT_LIST, local) {
            let row = right_row_at(local.1, window.right_scroll);
            let player = cockpit_faction(self.faction);
            let picked =
                picked_entries(window, &left_entries(world, movement, fog, player, window));
            let items = selection_items(world, movement, transport, &self.arrivals, &picked, window.tab);
            right_target(world, &items, row).or_else(|| {
                selected_rows(&window.selected_items, &right_keys(&items))
                    .first()
                    .and_then(|&index| right_target(world, &items, index))
            })
        } else {
            None
        };
        let mut pick = MissionTargetPick::from(window.system);
        match target {
            None => {}
            Some(None) => return Some(None),
            Some(Some(ItemTarget::Character(key))) => pick.character = Some(key),
            Some(Some(ItemTarget::Object(object))) => pick.object = Some(object),
        }
        Some(Some(pick))
    }

    pub fn clear(&mut self) {
        self.windows.clear();
        self.hidden.clear();
        self.drag = None;
    }

    /// Minimize `system`'s window (`0x466`): hide it with its state and
    /// return where it stood, for its rail entry.
    pub fn hide(&mut self, system: SystemKey) -> Option<(i16, i16)> {
        // A shown window has no hidden one: `open` takes it back.
        let window = self.close(system)?;
        let at = window.logical_position;
        self.hidden.push(window);
        Some(at)
    }

    /// Whether a left press on a Troops tab regiment is held: the list has
    /// captured the mouse (`FUN_006083c0`), so nothing under the pointer
    /// answers it.
    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// How far the pointer has moved since a list press held on `system`'s
    /// window, and whether the press was on the left list: the list drags
    /// on the first move with the button down (`FUN_006083c0`, case
    /// `0x200`), each `CoolDragList` with its own drag image
    /// (`FUN_00609960`).
    fn drag_offset(&self, ctx: &egui::Context, system: SystemKey) -> Option<DragImage> {
        let drag = self.drag.filter(|drag| drag.system == system)?;
        let offset = ctx.input(|input| input.pointer.latest_pos())? - drag.press;
        (offset != egui::Vec2::ZERO).then_some(DragImage {
            offset,
            left: drag.left,
        })
    }

    /// End a held press on the left release (`FUN_006083c0`, case
    /// `0x202`): more than `sqrt(24)` list pixels from the press it is the
    /// `0x29a` drop, and the galaxy view moves the whole selection against
    /// the window under the point (`FUN_00422ce0`, window type 4: `0x201`).
    /// The lists' flags are 0 (`FUN_004a4b10`), `0x100000` clear, so a
    /// release inside the window's own lists drops too: a ship released on
    /// another fleet's entry joins it.
    ///
    /// With Ctrl down at the release the drop is Confirmed Move (`0x202`,
    /// `FUN_00422ce0`).
    ///
    /// The distance is the release's own point's (`lParam` of
    /// `WM_LBUTTONUP`, [`crate::drag_list::release_point`]): a click whose
    /// pointer moves on within its release frame is no drop.
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
        if moved.length_sq() > DRAG_DISTANCE_SQUARED {
            return Some(DragEnd::Drop {
                system: drag.system,
                left: drag.left,
                point,
            });
        }
        drag.left.is_none().then_some(DragEnd::Release {
            system: drag.system,
            point,
        })
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

    fn close(&mut self, system: SystemKey) -> Option<OpenFleetWindow> {
        let index = self
            .windows
            .iter()
            .position(|window| window.system == system)?;
        Some(self.windows.remove(index))
    }

    fn window_mut(&mut self, system: SystemKey) -> Option<&mut OpenFleetWindow> {
        self.windows
            .iter_mut()
            .find(|window| window.system == system)
    }
}

/// What one Fleet window shows ([`FleetWindowState::report`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetWindowReport {
    pub origin: (i16, i16),
    pub entries: usize,
    /// Each listed fleet and its row in the left list.
    pub fleet_rows: Vec<(FleetKey, usize)>,
    /// The one selected entry, `None` for none or several.
    pub selected: Option<FleetWindowEntry>,
    /// The left list's selected rows, in list order.
    pub selected_rows: Vec<usize>,
    pub tab: FleetWindowTab,
    pub enabled: [bool; 4],
    pub items: Vec<String>,
    /// The right list's selected rows, in list order.
    pub selected_items: Vec<usize>,
    pub counts: Option<(u32, u32)>,
}

/// Actions that leave the Fleet window manager.
#[derive(Debug, Clone, PartialEq)]
pub enum FleetWindowAction {
    /// The restore-sector button (`0xca`) opens the subject's sector window
    /// (`FUN_00429ce0`).
    OpenSector(SystemKey),
    SelectSystem(SystemKey),
    /// The minimize button (`100`) posts `0x466`: the window goes to the
    /// galaxy view's rail.
    Minimize {
        system: SystemKey,
        logical_position: (i16, i16),
    },
    /// The left list's selected fleets dragged out of its list (`0x29a`):
    /// the galaxy view hit-tests the screen point and issues `0x201`
    /// (`FUN_00422ce0`) for them all.
    DragFleet {
        fleets: Vec<FleetKey>,
        point: egui::Pos2,
        /// Ctrl was down at the release: `0x202`, Confirmed Move.
        confirmed: bool,
    },
    /// The Troops tab's selected regiments dragged (`0x29a`): the galaxy
    /// view hit-tests the screen point and issues `0x201` (`FUN_00422ce0`).
    DragRegiment {
        troops: Vec<TroopKey>,
        point: egui::Pos2,
        confirmed: bool,
    },
    /// The Personnel tab's selected characters and special forces dragged:
    /// the same `0x201`, which expands to one Move mission order `0x241`
    /// for them all (`FUN_0053c810`).
    DragMembers {
        members: Vec<MissionMember>,
        point: egui::Pos2,
        confirmed: bool,
    },
    /// The Fighters tab's selected squadrons dragged: the same `0x201`, one
    /// squadron of each entry `indices` names in the fleet's `fighters`.
    DragSquadron {
        fleet: FleetKey,
        indices: Vec<usize>,
        point: egui::Pos2,
        confirmed: bool,
    },
    /// Unfinished units (`rebellion_core::products`) dragged: their Move
    /// (`0x201`, `FUN_00556390`), released on a fleet or a system.
    DragProducts {
        products: Vec<u64>,
        point: egui::Pos2,
        confirmed: bool,
    },
    /// The Capital Ships tab's selected ships, or a left list ship entry,
    /// dragged: the same `0x201` for the ships
    /// (`ghidra/notes/fleet-join-split.md`).
    DragShip {
        fleet: FleetKey,
        ships: Vec<usize>,
        roster: u64,
        point: egui::Pos2,
        confirmed: bool,
    },
    /// A right click on a list entry or item opens its object's pop-up menu
    /// (manual p. 120), at a 640 by 480 canvas point.
    OpenObjectMenu {
        system: SystemKey,
        selection: MenuObject,
        /// Every selected item's object, the clicked one first.
        team: Vec<MenuObject>,
        point: (i16, i16),
    },
    /// Enter in a Rename's edit with a name in it: `FUN_004ac950` puts the
    /// text in the order (`+0x44`) and issues it; the setter is
    /// `FUN_004f6e60`.
    Rename {
        entry: FleetWindowEntry,
        name: String,
    },
}

/// The side whose fleets a system's fleet icon and rail icon show
/// (`FUN_0045ccc0`, `FUN_004a76e0`): the player's when it has fleets there,
/// else the other side's, else the system's own side bits. Returns the side
/// (1 Alliance, 2 Empire, 3 contested, 0 none) and its fleet count.
///
/// port: the count is the system's fleets the player can see (the system
/// window's rule), and every fleet in the port is active (`+0x50` bit 6).
#[must_use]
pub fn icon_side(
    world: &GameWorld,
    movement: &MovementState,
    fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> (u8, usize) {
    let fleets = visible_fleets(world, movement, fog, player, system);
    // The other side's copied fleets count too (FUN_004f2d10).
    let copies = copy_entries(world, fog, player, system);
    let count = |side: u8| {
        fleets
            .iter()
            .filter(|&&fleet| {
                world
                    .fleets
                    .get(fleet)
                    .is_some_and(|value| fleet_side(value.is_alliance) == side)
            })
            .count()
            + copies
                .iter()
                .filter_map(|entry| entry.copy(world))
                .filter(|copy| fleet_side(copy.is_alliance) == side)
                .count()
    };
    let own = faction_side(player);
    let other = match own {
        1 => 2,
        2 => 1,
        _ => 0,
    };
    for side in [own, other] {
        let found = count(side);
        if found > 0 {
            return (side, found);
        }
    }
    let side = world
        .systems
        .get(system)
        .map_or(0, |value| control_side(value.control));
    (side, count(side))
}

/// The fleet icon's two bitmaps, or `None` when it is hidden: no fleets, or a
/// side with no art (`FUN_0045d140`, `FUN_0045ca80` kind `0x10`).
#[must_use]
pub fn fleet_icon(
    world: &GameWorld,
    movement: &MovementState,
    fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> Option<(u32, u32)> {
    match icon_side(world, movement, fog, player, system) {
        (_, 0) => None,
        (side, _) => quadrant_art(Quadrant::Fleets, side),
    }
}

/// A minimized Fleet window's rail icon (`FUN_004a76e0`): 11536 for side 1,
/// 11537 for side 2, 11538 otherwise.
#[must_use]
pub fn rail_icon(
    world: &GameWorld,
    movement: &MovementState,
    fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> u32 {
    match icon_side(world, movement, fog, player, system).0 {
        1 => RAIL_ICONS[0],
        2 => RAIL_ICONS[1],
        _ => RAIL_ICONS[2],
    }
}

/// The title strip's side (`+0x16c`, `FUN_004a3340`): the system's side bits
/// with no fleets; otherwise the player's side when one of its fleets is
/// there, else the other side.
fn title_side(world: &GameWorld, fleets: &[FleetKey], player: Faction, system: SystemKey) -> u8 {
    let own = faction_side(player);
    if fleets.is_empty() {
        return world
            .systems
            .get(system)
            .map_or(0, |value| control_side(value.control));
    }
    let own_present = fleets.iter().any(|&fleet| {
        world
            .fleets
            .get(fleet)
            .is_some_and(|value| fleet_side(value.is_alliance) == own)
    });
    if own_present {
        own
    } else if own == 1 {
        2
    } else {
        1
    }
}

/// `FUN_004a3340`: side 1's strip is 10299 active and 10200 inactive, side
/// 2's 10201 and 10302, any other 10303 and 10304. Active is the galaxy
/// view's focused child (`+0xb8`).
fn title_resource(side: u8, focused: bool) -> u32 {
    match (side, focused) {
        (1, true) => 10299,
        (1, false) => 10200,
        (2, true) => 10201,
        (2, false) => 10302,
        (_, true) => 10303,
        (_, false) => 10304,
    }
}

/// The system's fleets in key order that the player can see. hyp: the
/// original's merge loop orders by object id (`+0x6c`); the port's fleets
/// are kept in key order.
fn visible_fleets(
    world: &GameWorld,
    movement: &MovementState,
    fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> Vec<FleetKey> {
    // The player's copy's explored bit (ghidra/notes/fog-of-war.md); a
    // missing system is unexplored.
    if !rebellion_core::object_views::side_explored(world, player, system) {
        return Vec::new();
    }
    let opposing_visible = opposing_contents_visible(world, fog, player, system);
    let player_is_alliance = player == Faction::Alliance;
    // In listed_fleets' creation order (A0 fleet3/07: Fleet 7 after Fleet 6).
    rebellion_core::movement::listed_fleets(movement, world, system)
        .into_iter()
        .filter(|&fleet| {
            world.fleets.get(fleet).is_some_and(|value| {
                opposing_visible || value.is_alliance == player_is_alliance
            })
        })
        .collect()
}

fn left_entries(
    world: &GameWorld,
    movement: &MovementState,
    fog: &FogState,
    player: Faction,
    window: &OpenFleetWindow,
) -> Vec<FleetWindowEntry> {
    let mut entries = Vec::new();
    let fleets = visible_fleets(world, movement, fog, player, window.system);
    // Ships on their way to join a listed fleet are its members already
    // (see `joining_fleets`): their own departing fleet is not listed.
    let joined = |fleet: FleetKey| {
        join_target(world, movement, fleet).is_some_and(|target| fleets.contains(&target))
    };
    // The other side's fleets of the player's frozen copy (FUN_004f2d10,
    // FUN_004f9a60) after the player's own: as seen then.
    let copies = copy_entries(world, fog, player, window.system);
    for &fleet in fleets.iter().filter(|&&fleet| !joined(fleet)) {
        entries.push(FleetWindowEntry::Fleet(fleet));
        if window.expanded.contains(&fleet) {
            for member in std::iter::once(fleet).chain(joining_fleets(world, movement, fleet)) {
                if let Some(value) = world.fleets.get(member) {
                    entries.extend(
                        value
                            .capital_ships
                            .iter()
                            .enumerate()
                            .filter(|(_, ship)| ship.alive)
                            .map(|(index, _)| FleetWindowEntry::Ship { fleet: member, index }),
                    );
                }
            }
            // Its unfinished ships follow (FUN_004a3d40 walks mode 3).
            entries.extend(
                world
                    .products
                    .in_fleet(fleet)
                    .filter(|product| {
                        matches!(
                            product.kind,
                            rebellion_core::manufacturing::BuildableKind::CapitalShip(_)
                        )
                    })
                    .map(|product| FleetWindowEntry::Product {
                        fleet,
                        product: product.id,
                    }),
            );
        }
    }
    entries.extend(copies);
    entries
}

/// The fleets on their way to join `target`, in key order: a Move of ships
/// or a fleet onto a fleet routes each capital ship to it (`FUN_004ffc90`
/// expands a fleet into its ships for a fleet target), and the execute
/// reparents each into the target at once (`FUN_004feca0` → `FUN_004f8630`;
/// `fleet-join-split.md`), so "that ship will immediately be considered a
/// member of the fleet but will still be in hyperspace for several days
/// until it arrives" (manual p. 122). port: the ships travel as a fleet of
/// their own that joins on arrival (`fleet_join::join_fleet`); while it
/// will (`fleet_join::joins_on_arrival`), the Fleet window shows its ships
/// and their cargo as the target's, en route.
fn joining_fleets(world: &GameWorld, movement: &MovementState, target: FleetKey) -> Vec<FleetKey> {
    let Some(value) = world.fleets.get(target) else {
        return Vec::new();
    };
    let mut fleets: Vec<FleetKey> = movement
        .orders()
        .values()
        .filter(|order| order.join == Some(target) && order.fleet != target)
        .filter(|order| {
            fleet_join::joins_on_arrival(world, movement, target, order.destination, value.is_alliance)
                && world.fleets.get(order.fleet).is_some_and(|mover| mover.is_alliance == value.is_alliance)
        })
        .map(|order| order.fleet)
        .collect();
    fleets.sort_unstable();
    fleets
}

/// The fleet whose members `fleet`'s ships are while they travel to join it
/// ([`joining_fleets`]).
pub(crate) fn join_target(world: &GameWorld, movement: &MovementState, fleet: FleetKey) -> Option<FleetKey> {
    movement
        .get(fleet)?
        .join
        .filter(|&target| joining_fleets(world, movement, target).contains(&fleet))
}

/// [`right_items`] of the selection with, for a fleet, the ships on their
/// way to join it and what they carry after its own, each in hyperspace
/// (`FUN_004f8240`: an object is en route while its container is).
/// port: after the fleet's own (the original's list follows object keys).
fn window_items(
    world: &GameWorld,
    movement: &MovementState,
    transport: &TroopTransportState,
    arrivals: &[(MissionMember, FleetKey)],
    selected: Option<FleetWindowEntry>,
    tab: FleetWindowTab,
) -> Vec<RightItem> {
    let mut items = right_items(world, transport, arrivals, selected, tab);
    if let Some(FleetWindowEntry::Fleet(target)) = selected {
        for fleet in joining_fleets(world, movement, target) {
            items.extend(
                right_items(world, transport, arrivals, Some(FleetWindowEntry::Fleet(fleet)), tab)
                    .into_iter()
                    .map(|item| RightItem {
                        en_route: true,
                        ..item
                    }),
            );
        }
    }
    items
}

/// [`tab_enabled`] with the ships on their way to join a selected fleet.
fn window_tab_enabled(
    world: &GameWorld,
    movement: &MovementState,
    transport: &TroopTransportState,
    arrivals: &[(MissionMember, FleetKey)],
    selected: Option<FleetWindowEntry>,
    tab: FleetWindowTab,
) -> bool {
    tab_enabled(world, transport, arrivals, selected, tab)
        || matches!(selected, Some(FleetWindowEntry::Fleet(target))
            if joining_fleets(world, movement, target).into_iter().any(|fleet| {
                tab_enabled(world, transport, arrivals, Some(FleetWindowEntry::Fleet(fleet)), tab)
            }))
}

/// [`tab_counts`] with the ships on their way to join a selected fleet:
/// their slots, aboard and capacity, are the fleet's now (`FUN_004a6be0`
/// walks the fleet's ships in mode 3).
fn window_tab_counts(
    world: &GameWorld,
    movement: &MovementState,
    transport: &TroopTransportState,
    selected: Option<FleetWindowEntry>,
    tab: FleetWindowTab,
) -> Option<(u32, u32)> {
    let (mut aboard, mut capacity) = tab_counts(world, transport, selected, tab)?;
    if let Some(FleetWindowEntry::Fleet(target)) = selected {
        for fleet in joining_fleets(world, movement, target) {
            if let Some((more, room)) = tab_counts(world, transport, Some(FleetWindowEntry::Fleet(fleet)), tab) {
                aboard = aboard.saturating_add(more);
                capacity = capacity.saturating_add(room);
            }
        }
    }
    Some((aboard, capacity))
}

/// `+0x70` (`FUN_004a3130`) at a window-local logical point.
///
/// 1. The default: the selected own-side fleet; none while own-side fleets
///    are listed but none is selected; the subject (the system) when the
///    list holds no own-side fleet.
/// 2. In the left list, the subject, or the entry under the point.
/// 3. In the right list on the Capital ships tab, the ship under the point.
///
/// port: a ship stands for its fleet, which carries the port's cargo.
fn drop_target(
    world: &GameWorld,
    player: Faction,
    window: &OpenFleetWindow,
    entries: &[FleetWindowEntry],
    local: (f32, f32),
) -> Option<ReleaseTarget> {
    let system = window.system;
    let own = |fleet: FleetKey| {
        world
            .fleets
            .get(fleet)
            .is_some_and(|value| fleet_side(value.is_alliance) == faction_side(player))
    };
    let fleet_target = |fleet: FleetKey| ReleaseTarget::Fleet { fleet, system };
    if in_rect(LEFT_LIST, local) {
        let row = left_row_at(local.1, window.left_scroll);
        return Some(
            entries
                .get(row)
                .map_or(ReleaseTarget::System(system), |entry| {
                    fleet_target(entry.fleet())
                }),
        );
    }
    if window.tab == FleetWindowTab::CapitalShips && in_rect(RIGHT_LIST, local) {
        // The ship under the point gives its fleet: each selected entry
        // lists its fleet's living ships, or a ship itself, in list order
        // (FUN_004a6be0).
        let mut row = right_row_at(local.1, window.right_scroll);
        for entry in picked_entries(window, entries) {
            let ships = match entry {
                FleetWindowEntry::Fleet(fleet) => world.fleets.get(fleet).map_or(0, |value| {
                    value.capital_ships.iter().filter(|ship| ship.alive).count()
                }),
                FleetWindowEntry::Ship { .. } if picked_entries(window, entries).len() > 1 => 1,
                FleetWindowEntry::Ship { .. }
                | FleetWindowEntry::Product { .. }
                | FleetWindowEntry::Copy { .. } => 0,
            };
            if row < ships {
                return Some(fleet_target(entry.fleet()));
            }
            row -= ships;
        }
    }
    let own_listed = entries
        .iter()
        .any(|entry| matches!(entry, FleetWindowEntry::Fleet(fleet) if own(*fleet)));
    if !own_listed {
        return Some(ReleaseTarget::System(system));
    }
    match window.selected.single() {
        Some(FleetWindowEntry::Fleet(fleet)) if own(fleet) => Some(fleet_target(fleet)),
        _ => None,
    }
}

/// A tab's state for the selection: Capital ships is disabled for a ship;
/// the others unless the selection carries their contents (`FUN_004a5c00`,
/// flags 1, 2 and 4).
fn tab_enabled(
    world: &GameWorld,
    transport: &TroopTransportState,
    arrivals: &[(MissionMember, FleetKey)],
    selected: Option<FleetWindowEntry>,
    tab: FleetWindowTab,
) -> bool {
    let fleet = match selected {
        Some(FleetWindowEntry::Fleet(fleet)) => fleet,
        // An unfinished ship carries nothing (its rows in the capacity
        // table are refused, unfinished-products.md).
        Some(FleetWindowEntry::Product { .. }) => return false,
        // A0 fleet2/umgul-transport1-selected: Medium Transport 1's
        // regiments light its Troops tab, Corellian Corvette 1's stays
        // dark; a ship's tabs follow its own content flags.
        Some(FleetWindowEntry::Ship { fleet, index }) => {
            let flags = ship_contents(world, transport, arrivals, fleet)
                .get(index)
                .copied()
                .unwrap_or(0);
            return match tab {
                FleetWindowTab::CapitalShips => false,
                FleetWindowTab::Fighters => flags & CONTENT_SQUADRONS != 0,
                FleetWindowTab::Troops => flags & CONTENT_REGIMENTS != 0,
                FleetWindowTab::Personnel => flags & CONTENT_PERSONNEL != 0,
            };
        }
        // A copied fleet's tabs light by what the copy holds. One copied
        // with its name only, made after the side last saw the system, has
        // all four lit though they list nothing: A0 2026-11-14-stale-views/
        // 06-kothlis-fleet16-d374-montage (sha256 prefix 955559b56955015b)
        // selects each of Fleet 16's four tabs, the Fighters and Troops
        // tabs printing 0 and 0. hyp: its ships' copies exist, unfinished
        // (FUN_0053f760), so their content flags reach the entry
        // (FUN_004a66a0) while the lists' mode-3 walks skip them.
        Some(entry @ FleetWindowEntry::Copy { .. }) => {
            let Some(copy) = entry.copy(world) else {
                return false;
            };
            if copy.ships.is_empty() {
                return true;
            }
            return match tab {
                FleetWindowTab::CapitalShips => true,
                FleetWindowTab::Fighters => !copy.fighters.is_empty(),
                FleetWindowTab::Troops => !copy.troops.is_empty(),
                FleetWindowTab::Personnel => !copy.characters.is_empty(),
            };
        }
        None => return false,
    };
    let Some(value) = world.fleets.get(fleet) else {
        return false;
    };
    // What is on its way in by Move is in the fleet already
    // (FUN_00556390), so it lights its tab too.
    match tab {
        FleetWindowTab::CapitalShips => true,
        FleetWindowTab::Fighters => {
            value.fighters.iter().any(|entry| entry.count > 0)
                || transport
                    .squadrons
                    .transits()
                    .iter()
                    .any(|transit| transit.to == SquadronPlace::Fleet(fleet) && transit.count > 0)
        }
        FleetWindowTab::Troops => {
            transport.carried_count(fleet) > 0
                || transport
                    .transits()
                    .iter()
                    .any(|transit| transit.leg == RegimentLeg::Fleet(fleet))
                || world.products.in_fleet(fleet).any(|product| {
                    matches!(product.kind, rebellion_core::manufacturing::BuildableKind::Troop(_))
                })
        }
        FleetWindowTab::Personnel => {
            !value.characters.is_empty()
                || !value.special_forces.is_empty()
                || bound_members(world, arrivals, fleet).next().is_some()
        }
    }
}

/// The members on their way into `fleet` that its walk finds. The walk's
/// mode 3 takes existing children only (`+0x50` bit 6, `FUN_004f6b90`;
/// `FUN_00536da0(ship, 3)`), so a killed member is gone; and the Move's
/// acceptance refuses another side's container (`FUN_00553410`: `1`/`0x24`,
/// `1`/`0x28`), so a member of another side is never aboard. A prisoner
/// carries its captor's side (`FUN_004eec60`, `missions::character_side`).
pub(crate) fn bound_members<'a>(
    world: &'a GameWorld,
    arrivals: &'a [(MissionMember, FleetKey)],
    fleet: FleetKey,
) -> impl Iterator<Item = MissionMember> + 'a {
    let side = world.fleets.get(fleet).map(|value| value.is_alliance);
    arrivals
        .iter()
        .filter(move |(_, to)| *to == fleet)
        .map(|&(member, _)| member)
        .filter(move |&member| {
            let is_alliance = match member {
                MissionMember::Character(key) => world
                    .characters
                    .get(key)
                    .filter(|character| !character.is_killed)
                    .and_then(rebellion_core::missions::character_side)
                    .map(|faction| faction == Faction::Alliance),
                MissionMember::SpecialForce(key) => {
                    world.special_forces.get(key).map(|unit| unit.is_alliance)
                }
            };
            is_alliance.is_some() && is_alliance == side
        })
}

/// The tab a new selection shows: a ship has no Capital Ships tab, so it
/// shows Fighters instead, and keeps it when a fleet is selected again
/// (A0 fleet3/02, 06: Medium Transport 1 selected over Capital Ships shows
/// Fighters with its counters 0 and 0, and Fleet 6 then too).
fn tab_for(selected: &[FleetWindowEntry], tab: FleetWindowTab) -> FleetWindowTab {
    match selected {
        [FleetWindowEntry::Ship { .. }] if tab == FleetWindowTab::CapitalShips => {
            FleetWindowTab::Fighters
        }
        _ => tab,
    }
}

/// Each selected entry's fleet and its live ship count.
fn selected_roster(world: &GameWorld, selected: &[FleetWindowEntry]) -> Vec<(FleetKey, usize)> {
    selected
        .iter()
        .filter_map(|entry| {
            let fleet = entry.fleet();
            let value = world.fleets.get(fleet)?;
            Some((fleet, value.capital_ships.iter().filter(|ship| ship.alive).count()))
        })
        .collect()
}

/// Whether the selected fleet's ships changed since the window last drew:
/// A0 fleet3/03, 05, 10 show no selection after Create Fleet, a ship's
/// Move onto a fleet and a fleet's Move onto another. hyp: the list
/// rebuilds when its fleet's contents change and loses its selection.
fn roster_changed(world: &GameWorld, open: &OpenFleetWindow) -> bool {
    let now = selected_roster(world, open.selected.items());
    open.roster.iter().any(|&(fleet, count)| {
        now.iter()
            .any(|&(listed, live)| listed == fleet && live != count)
    })
}

/// The tabs the strip shows: all four for a fleet; a ship's lacks Capital
/// Ships (A0 fleet2/umgul-corvette1-selected shows three, from x 33). The
/// strip has no disabled art for it: `b + 3` is Personnel's normal art.
fn shown_tabs(selected: &[FleetWindowEntry]) -> impl Iterator<Item = FleetWindowTab> {
    let ship = matches!(selected, [FleetWindowEntry::Ship { .. }]);
    FleetWindowTab::ALL
        .into_iter()
        .filter(move |&tab| !(ship && tab == FleetWindowTab::CapitalShips))
}

/// The window's selected entries among the listed `entries`, in list order,
/// as `FUN_004a5c00` walks the list (`+0x188`).
fn picked_entries(window: &OpenFleetWindow, entries: &[FleetWindowEntry]) -> Vec<FleetWindowEntry> {
    entries
        .iter()
        .copied()
        .filter(|entry| window.selected.contains(*entry))
        .collect()
}

/// The right list for the left list's `selected` entries (`FUN_004a5c00`):
/// the list is emptied, then each selected entry adds its contents for
/// the tab (`FUN_004a6be0`), a fleet its ships' and a ship its own, in list
/// order. On Capital Ships a ship among several adds itself (tab `0x66`).
fn selection_items(
    world: &GameWorld,
    movement: &MovementState,
    transport: &TroopTransportState,
    arrivals: &[(MissionMember, FleetKey)],
    selected: &[FleetWindowEntry],
    tab: FleetWindowTab,
) -> Vec<RightItem> {
    if let [one] = selected {
        return window_items(world, movement, transport, arrivals, Some(*one), tab);
    }
    selected
        .iter()
        .flat_map(|&entry| match (entry, tab) {
            (FleetWindowEntry::Ship { fleet, index }, FleetWindowTab::CapitalShips) => window_items(
                world,
                movement,
                transport,
                arrivals,
                Some(FleetWindowEntry::Fleet(fleet)),
                tab,
            )
            .into_iter()
            .filter(|item| {
                matches!(item.object, Some(ItemObject::Ship { fleet: of, index: at, .. })
                    if of == fleet && at == index)
            })
            .collect(),
            _ => window_items(world, movement, transport, arrivals, Some(entry), tab),
        })
        .collect()
}

/// A tab's state for several entries (`FUN_004a6980`): Capital Ships is
/// disabled only for one ship alone (`+0x14c` bit 8); each other tab
/// lights when any selected entry carries its contents (the items' `+0x54`
/// flags, or'ed).
fn selection_tab_enabled(
    world: &GameWorld,
    movement: &MovementState,
    transport: &TroopTransportState,
    arrivals: &[(MissionMember, FleetKey)],
    selected: &[FleetWindowEntry],
    tab: FleetWindowTab,
) -> bool {
    match selected {
        [one] => window_tab_enabled(world, movement, transport, arrivals, Some(*one), tab),
        [] => false,
        _ if tab == FleetWindowTab::CapitalShips => true,
        _ => selected
            .iter()
            .any(|&entry| window_tab_enabled(world, movement, transport, arrivals, Some(entry), tab)),
    }
}

/// The counts of several entries: `FUN_004a6be0` adds each selected
/// ship's slots into `+0x198` and `+0x19c`.
fn selection_counts(
    world: &GameWorld,
    movement: &MovementState,
    transport: &TroopTransportState,
    selected: &[FleetWindowEntry],
    tab: FleetWindowTab,
) -> Option<(u32, u32)> {
    selected
        .iter()
        .filter_map(|&entry| window_tab_counts(world, movement, transport, Some(entry), tab))
        .reduce(|(aboard, capacity), (more, room)| {
            (aboard.saturating_add(more), capacity.saturating_add(room))
        })
}

/// One right-list item: its GOKRES mini, name and whether it lacks a
/// hyperdrive (indicator 10430, flag `0x40`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct RightItem {
    mini: Option<u32>,
    /// What the mini stands for, for its en route mark.
    kind: MiniObject,
    label: String,
    no_hyperdrive: bool,
    /// The ship's content flags (`FUN_004a66a0`).
    contents: u8,
    /// A damaged ship (`+0x50` bit `0x200`, `FUN_00501c00`).
    damaged: bool,
    /// A captive character (`+0xac` bit 0), marked over its mini.
    captive: bool,
    /// On its way into the fleet by Move (`+0x50` bit 4): `FUN_00556390`
    /// puts it in the fleet at once, so the fleet lists it with its en
    /// route mark although the fleet is in orbit.
    en_route: bool,
    /// The regiment or ship the item stands for.
    object: Option<ItemObject>,
}

/// A Fleet window item a mission can target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemTarget {
    Character(CharacterKey),
    Object(MissionTarget),
}

/// A living capital ship as a mission target: the Death Star hull
/// (`CapitalShipClass::is_death_star`) by its fleet, any other by its class
/// aboard the fleet.
fn ship_target(world: &GameWorld, fleet: FleetKey, index: usize) -> Option<MissionTarget> {
    let ship = world.fleets.get(fleet)?.capital_ships.get(index)?;
    let class = world.capital_ship_classes.get(ship.class)?;
    Some(if class.is_death_star() {
        MissionTarget::DeathStar(fleet)
    } else {
        MissionTarget::CapitalShip {
            fleet,
            class: ship.class,
        }
    })
}

/// The right list's `index`th item as a mission target: `None` when there
/// is no such item, `Some(None)` for an item no mission takes.
fn right_target(world: &GameWorld, items: &[RightItem], index: usize) -> Option<Option<ItemTarget>> {
    let item = items.get(index)?;
    Some(match item.object {
        Some(ItemObject::Regiment(key)) => Some(ItemTarget::Object(MissionTarget::Troop(key))),
        Some(ItemObject::Ship { fleet, index, .. }) => {
            ship_target(world, fleet, index).map(ItemTarget::Object)
        }
        Some(ItemObject::Fighter { fleet, index }) => world
            .fleets
            .get(fleet)
            .and_then(|value| value.fighters.get(index))
            .map(|entry| {
                ItemTarget::Object(MissionTarget::Fighter {
                    fleet,
                    class: entry.class,
                })
            }),
        Some(ItemObject::Character(key)) => Some(ItemTarget::Character(key)),
        Some(ItemObject::SpecialForce(key)) => {
            Some(ItemTarget::Object(MissionTarget::SpecialForce(key)))
        }
        Some(
            ItemObject::Fleet(_)
            | ItemObject::TravellingFighter { .. }
            | ItemObject::Product(_)
            | ItemObject::ProductFighter { .. }
            | ItemObject::ProductTroop { .. },
        )
        | None => None,
    })
}

/// The current tab's objects aboard the selected fleet (`FUN_004a6be0`).
/// The right list's row of `fleet`'s ship `index` on the Capital Ships
/// tab: its place among the fleet's living ships.
fn ship_item_row(world: &GameWorld, fleet: FleetKey, index: usize) -> Option<usize> {
    let ships = &world.fleets.get(fleet)?.capital_ships;
    ships.get(index).filter(|ship| ship.alive)?;
    Some(ships[..index].iter().filter(|ship| ship.alive).count())
}

/// A copied fleet's right list (`FleetDetail`): its ships with their
/// damage, its squadrons, its regiments and its characters as the side saw
/// them (`FUN_004f9a60`). port: the items open no menu.
fn copy_items(
    world: &GameWorld,
    copy: &rebellion_core::object_views::FleetDetail,
    tab: FleetWindowTab,
) -> Vec<RightItem> {
    let item = |mini, kind, label: String, damaged| RightItem {
        mini,
        kind,
        label,
        no_hyperdrive: false,
        contents: 0,
        damaged,
        captive: false,
        en_route: false,
        object: None,
    };
    match tab {
        FleetWindowTab::CapitalShips => copy
            .ships
            .iter()
            .filter_map(|ship| {
                let class = world
                    .capital_ship_classes
                    .values()
                    .find(|class| class.dat_id == ship.class)?;
                Some(item(
                    capital_ship_mini_id(ship.class),
                    MiniObject::Craft,
                    ship.name.clone().unwrap_or_else(|| class.name.clone()),
                    ship.hull < i32::try_from(class.hull).unwrap_or(i32::MAX),
                ))
            })
            .collect(),
        FleetWindowTab::Fighters => copy
            .fighters
            .iter()
            .filter_map(|(class, count)| {
                let fighter = world.fighter_classes.values().find(|value| value.dat_id == *class)?;
                Some((0..*count).map(move |_| {
                    item(fighter_mini_id(*class), MiniObject::Craft, fighter.name.clone(), false)
                }))
            })
            .flatten()
            .collect(),
        FleetWindowTab::Troops => copy
            .troops
            .iter()
            .filter_map(|class| {
                let (mini, label) = troop_mini(*class)?;
                Some(item(Some(mini), MiniObject::Regiment(*class), label.into(), false))
            })
            .collect(),
        FleetWindowTab::Personnel => copy
            .characters
            .iter()
            .filter_map(|id| {
                let key = crate::viewed_contents::character_key(world, *id)?;
                let character = &world.characters[key];
                Some(item(
                    character_picture(character),
                    MiniObject::Character,
                    character.name.clone(),
                    false,
                ))
            })
            .collect(),
    }
}

fn right_items(
    world: &GameWorld,
    transport: &TroopTransportState,
    arrivals: &[(MissionMember, FleetKey)],
    selected: Option<FleetWindowEntry>,
    tab: FleetWindowTab,
) -> Vec<RightItem> {
    // A selected ship lists what it carries (FUN_00536da0(ship, ..), its
    // own containers): A0 fleet-aboard/10..12 shows Imperial Star Destroyer
    // 1's six TIE Fighter squadrons, three Stormtrooper Regiments and
    // Veers. The port seats the fleet's units as `ship_cargo` does, and the
    // units on their way in as `ship_load` does.
    if let Some(entry @ FleetWindowEntry::Copy { .. }) = selected {
        return entry.copy(world).map_or_else(Vec::new, |copy| copy_items(world, copy, tab));
    }
    if let Some(FleetWindowEntry::Ship { fleet, index }) = selected {
        let cargo = ship_load(world, transport, arrivals, fleet);
        let Some(own) = cargo.get(index) else {
            return Vec::new();
        };
        let items = right_items(
            world,
            transport,
            arrivals,
            Some(FleetWindowEntry::Fleet(fleet)),
            tab,
        );
        return match tab {
            FleetWindowTab::CapitalShips => Vec::new(),
            FleetWindowTab::Fighters => {
                // The fleet's own squadrons first, then those on their way
                // in, each listed under the ship it was seated on.
                let own_total = items
                    .iter()
                    .filter(|item| !matches!(item.object, Some(ItemObject::TravellingFighter { .. })))
                    .count();
                let before: usize = cargo[..index].iter().map(|each| each.squadrons).sum();
                let mut items = items.into_iter();
                let mut listed: Vec<RightItem> =
                    items.by_ref().take(own_total).skip(before).take(own.squadrons).collect();
                listed.extend(
                    items
                        .enumerate()
                        .filter(|(ordinal, _)| own.incoming_squadrons.contains(ordinal))
                        .map(|(_, item)| item),
                );
                listed
            }
            FleetWindowTab::Troops | FleetWindowTab::Personnel => items
                .into_iter()
                .filter(|item| match item.object {
                    Some(ItemObject::Regiment(key)) => own.regiments.contains(&key),
                    Some(ItemObject::Character(key)) => own.characters.contains(&key),
                    Some(ItemObject::SpecialForce(key)) => own.special_forces.contains(&key),
                    _ => false,
                })
                .collect(),
        };
    }
    let Some(FleetWindowEntry::Fleet(fleet)) = selected else {
        return Vec::new();
    };
    let Some(value) = world.fleets.get(fleet) else {
        return Vec::new();
    };
    let roster = fleet_join::roster(world, fleet).unwrap_or_default();
    let contents = ship_contents(world, transport, arrivals, fleet);
    match tab {
        FleetWindowTab::CapitalShips => value
            .capital_ships
            .iter()
            .enumerate()
            .filter(|(_, ship)| ship.alive)
            .filter_map(|(index, ship)| {
                Some((index, ship, world.capital_ship_classes.get(ship.class)?))
            })
            .map(|(index, ship, class)| RightItem {
                mini: capital_ship_mini_id(class.dat_id),
                kind: MiniObject::Craft,
                // FUN_004f6270: the ship's own name, else its class's.
                label: world
                    .ship_name(fleet, index)
                    .unwrap_or(&class.name)
                    .to_owned(),
                no_hyperdrive: stranded(class, ship),
                contents: contents.get(index).copied().unwrap_or(0),
                damaged: crate::status_rows::damaged(ship, class),
                captive: false,
                en_route: false,
                object: Some(ItemObject::Ship {
                    fleet,
                    index,
                    roster,
                }),
            })
            .chain(world.products.in_fleet(fleet).filter_map(|product| {
                let rebellion_core::manufacturing::BuildableKind::CapitalShip(class) =
                    product.kind
                else {
                    return None;
                };
                let class = world.capital_ship_classes.get(class)?;
                Some(RightItem {
                    mini: capital_ship_mini_id(class.dat_id),
                    kind: MiniObject::Craft,
                    label: product.name.clone().unwrap_or_else(|| class.name.clone()),
                    no_hyperdrive: false,
                    contents: 0,
                    damaged: false,
                    captive: false,
                    // Finished, it travels to the fleet (bit 4).
                    en_route: world.products.arrival(product.id).is_some(),
                    object: Some(ItemObject::Product(product.id)),
                })
            }))
            .collect(),
        // port: the port keeps squadrons as counts per class. Squadrons on
        // their way into the fleet by Move follow its own: FUN_00556390
        // puts them in the fleet at once, in hyperspace.
        FleetWindowTab::Fighters => {
            let travelling = transport
                .squadrons
                .transits()
                .iter()
                .filter(|transit| transit.to == SquadronPlace::Fleet(fleet))
                .map(|transit| {
                    (
                        transit.class,
                        transit.count,
                        ItemObject::TravellingFighter {
                            fleet,
                            class: transit.class,
                            arrival: transit.arrival_tick,
                        },
                        0,
                    )
                });
            value
                .fighters
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    (
                        entry.class,
                        entry.count,
                        ItemObject::Fighter { fleet, index },
                        // A squadron with losses (+0x60) is damaged
                        // (FUN_00503340 sets +0x50 bit 0x200).
                        if entry.damage.is_some() { entry.count } else { 0 },
                    )
                })
                .chain(travelling)
                // The unfinished squadrons aboard follow (mode 3).
                .chain(world.products.in_fleet(fleet).filter_map(|product| {
                    let rebellion_core::manufacturing::BuildableKind::Fighter(class) = product.kind
                    else {
                        return None;
                    };
                    Some((
                        class,
                        1,
                        ItemObject::ProductFighter {
                            product: product.id,
                            system: value.location,
                            class,
                        },
                        0,
                    ))
                }))
                .filter_map(|(class, count, object, damaged)| {
                    Some((world.fighter_classes.get(class)?, count, object, damaged))
                })
                .flat_map(|(class, count, object, damaged)| {
                    (0..count).map(move |row| RightItem {
                        mini: fighter_mini_id(class.dat_id),
                        kind: MiniObject::Craft,
                        label: class.name.clone(),
                        no_hyperdrive: false,
                        contents: 0,
                        damaged: row < damaged,
                        captive: false,
                        en_route: matches!(object, ItemObject::TravellingFighter { .. }),
                        object: Some(object),
                    })
                })
                .collect()
        }
        // Regiments on their way into the fleet by Move follow its cargo
        // (FUN_00556390, `regiment-unload.md`).
        FleetWindowTab::Troops => transport
            .cargo(fleet)
            .iter()
            .map(|&key| (key, false))
            .chain(
                transport
                    .transits()
                    .iter()
                    .filter(|transit| transit.leg == RegimentLeg::Fleet(fleet))
                    .map(|transit| (transit.troop, true)),
            )
            .filter_map(|(key, en_route)| Some((key, en_route, world.troops.get(key)?)))
            .filter_map(|(key, en_route, troop)| {
                Some((
                    key,
                    en_route,
                    troop.class_dat_id,
                    troop_mini(troop.class_dat_id)?,
                ))
            })
            .map(|(key, en_route, class, (mini, label))| RightItem {
                mini: Some(mini),
                kind: MiniObject::Regiment(class),
                label: label.to_owned(),
                no_hyperdrive: false,
                contents: 0,
                damaged: false,
                captive: false,
                en_route,
                object: Some(ItemObject::Regiment(key)),
            })
            // The unfinished regiments aboard follow (mode 3; A0 2026-11-15
            // "m-5": on the grid).
            .chain(world.products.in_fleet(fleet).filter_map(|product| {
                let rebellion_core::manufacturing::BuildableKind::Troop(class) = product.kind else {
                    return None;
                };
                let (mini, label) = troop_mini(class)?;
                Some(RightItem {
                    mini: Some(mini),
                    kind: MiniObject::Regiment(class),
                    label: label.to_owned(),
                    no_hyperdrive: false,
                    contents: 0,
                    damaged: false,
                    captive: false,
                    en_route: world.products.arrival(product.id).is_some(),
                    object: Some(ItemObject::ProductTroop {
                        product: product.id,
                        system: value.location,
                        class,
                    }),
                })
            }))
            .collect(),
        FleetWindowTab::Personnel => value
            .characters
            .iter()
            .filter_map(|&key| Some((key, world.characters.get(key)?)))
            .map(|(key, character)| RightItem {
                mini: character_picture(character),
                kind: MiniObject::Character,
                label: character.name.clone(),
                no_hyperdrive: false,
                contents: 0,
                // FUN_004f7800: an injured character is damaged.
                damaged: character.injury > 0,
                captive: character.is_captive,
                en_route: false,
                object: Some(ItemObject::Character(key)),
            })
            // hyp: the special forces after the characters.
            .chain(value.special_forces.iter().filter_map(|&key| {
                let unit = world.special_forces.get(key)?;
                let (mini, label) = crate::system_window::special_force_mini(unit.class_dat_id)?;
                Some(RightItem {
                    mini: Some(mini),
                    kind: MiniObject::SpecialForce(unit.class_dat_id),
                    label: label.to_owned(),
                    no_hyperdrive: false,
                contents: 0,
                    damaged: false,
                    captive: false,
                    en_route: false,
                    object: Some(ItemObject::SpecialForce(key)),
                })
            }))
            // A member on its way aboard by Move is in the fleet already
            // (FUN_00556390 into the Move's target container +0x74), with
            // its en route mark (FUN_0042c3b0 +0x50 bit 4).
            .chain(
                bound_members(world, arrivals, fleet)
                    .filter_map(|member| travelling_member_item(world, member)),
            )
            .collect(),
    }
}

/// A Personnel tab row for `member` on its way into the fleet.
fn travelling_member_item(world: &GameWorld, member: MissionMember) -> Option<RightItem> {
    Some(match member {
        MissionMember::Character(key) => {
            let character = world.characters.get(key).filter(|value| !value.is_killed)?;
            RightItem {
                mini: character_picture(character),
                kind: MiniObject::Character,
                label: character.name.clone(),
                no_hyperdrive: false,
                contents: 0,
                damaged: character.injury > 0,
                captive: character.is_captive,
                en_route: true,
                object: Some(ItemObject::Character(key)),
            }
        }
        MissionMember::SpecialForce(key) => {
            let unit = world.special_forces.get(key)?;
            let (mini, label) = crate::system_window::special_force_mini(unit.class_dat_id)?;
            RightItem {
                mini: Some(mini),
                kind: MiniObject::SpecialForce(unit.class_dat_id),
                label: label.to_owned(),
                no_hyperdrive: false,
                contents: 0,
                damaged: false,
                captive: false,
                en_route: true,
                object: Some(ItemObject::SpecialForce(key)),
            }
        }
    })
}

/// The two numbers tabs `0x67` and `0x68` print into the picture panel: the
/// squadrons or regiments aboard and the capacity (`FUN_004a5c00`, ship
/// slots `+0x23c`/`+0x240` and `+0x26c`/`+0x270`, summed).
fn tab_counts(
    world: &GameWorld,
    transport: &TroopTransportState,
    selected: Option<FleetWindowEntry>,
    tab: FleetWindowTab,
) -> Option<(u32, u32)> {
    // A selected ship prints its own slots (A0 fleet-aboard/10, 11:
    // Imperial Star Destroyer 1's 6 and 6 squadrons, 3 and 3 regiments).
    if let Some(FleetWindowEntry::Ship { fleet, index }) = selected {
        let value = world.fleets.get(fleet)?;
        let ship = value.capital_ships.get(index)?;
        let class = world.capital_ship_classes.get(ship.class)?;
        // Its slots count the units on their way in seated on it too (mode
        // 3, CapShip_vt23c / CapShip_vt240).
        let own = ship_load(world, transport, &[], fleet).into_iter().nth(index)?;
        return match tab {
            FleetWindowTab::Fighters => Some((
                u32::try_from(own.squadrons + own.incoming_squadrons.len()).unwrap_or(u32::MAX),
                class.fighter_capacity,
            )),
            FleetWindowTab::Troops => Some((
                u32::try_from(own.regiments.len()).unwrap_or(u32::MAX),
                class.troop_capacity,
            )),
            FleetWindowTab::CapitalShips | FleetWindowTab::Personnel => None,
        };
    }
    // A copied fleet prints what its copy holds: its squadrons or
    // regiments and its copied ships' room (0 and 0 for a fleet copied with
    // its name only, A0 06-kothlis-fleet16-d374-montage).
    if let Some(entry @ FleetWindowEntry::Copy { .. }) = selected {
        let copy = entry.copy(world)?;
        let room = |capacity: fn(&rebellion_core::world::CapitalShipClass) -> u32| {
            copy.ships
                .iter()
                .filter_map(|ship| {
                    world
                        .capital_ship_classes
                        .values()
                        .find(|class| class.dat_id == ship.class)
                })
                .map(capacity)
                .sum::<u32>()
        };
        return match tab {
            FleetWindowTab::Fighters => Some((
                copy.fighters.iter().map(|(_, count)| count).sum(),
                room(|class| class.fighter_capacity),
            )),
            FleetWindowTab::Troops => Some((
                u32::try_from(copy.troops.len()).unwrap_or(u32::MAX),
                room(|class| class.troop_capacity),
            )),
            FleetWindowTab::CapitalShips | FleetWindowTab::Personnel => None,
        };
    }
    let Some(FleetWindowEntry::Fleet(fleet)) = selected else {
        return None;
    };
    let value = world.fleets.get(fleet)?;
    // The ships' slots +0x23c / +0x240 count their children by the list's
    // own walk, mode 3 (CapShip_vt23c FUN_00500fe0, CapShip_vt240
    // FUN_00501040: FUN_005039d0 / FUN_00504c40 and FUN_00513180), so units
    // on their way in, already the ships' children (FUN_00556390), count.
    match tab {
        FleetWindowTab::Fighters => {
            let own: u32 = value.fighters.iter().map(|entry| entry.count).sum();
            let incoming = incoming_squadrons(transport, fleet).count();
            let aboard = own.saturating_add(u32::try_from(incoming).unwrap_or(u32::MAX));
            let capacity = value
                .capital_ships
                .iter()
                .filter(|ship| ship.alive)
                .filter_map(|ship| world.capital_ship_classes.get(ship.class))
                .map(|class| class.fighter_capacity)
                .fold(0_u32, u32::saturating_add);
            Some((aboard, capacity))
        }
        FleetWindowTab::Troops => Some((
            u32::try_from(transport.carried_count(fleet) + incoming_regiments(transport, fleet).count())
                .unwrap_or(u32::MAX),
            TroopTransportState::fleet_capacity(world, fleet)?,
        )),
        FleetWindowTab::CapitalShips | FleetWindowTab::Personnel => None,
    }
}

/// The squadrons on their way into `fleet` by Move, one class per
/// squadron, in departure order.
fn incoming_squadrons(
    transport: &TroopTransportState,
    fleet: FleetKey,
) -> impl Iterator<Item = &rebellion_core::squadrons::SquadronTransit> + '_ {
    transport
        .squadrons
        .transits()
        .iter()
        .filter(move |transit| transit.to == SquadronPlace::Fleet(fleet))
        .flat_map(|transit| (0..transit.count).map(move |_| transit))
}

/// The regiments on their way into `fleet` by Move, in departure order.
fn incoming_regiments(
    transport: &TroopTransportState,
    fleet: FleetKey,
) -> impl Iterator<Item = TroopKey> + '_ {
    transport
        .transits()
        .iter()
        .filter(move |transit| transit.leg == RegimentLeg::Fleet(fleet))
        .map(|transit| transit.troop)
}

/// The left edge of a picture `width` wide, centred on whole pixels in the
/// 125-wide picture panel (`FUN_004a5c00`).
fn picture_x(width: f32) -> f32 {
    PICTURE_PANEL.0 + ((PICTURE_PANEL.2 - width) / 2.0).floor()
}

/// Tab `k`'s bitmap from a strip's base: normal `b + k`, disabled
/// `b + 3 + k`, selected `b + 7 + k` (`FUN_004a4b10`).
/// A tab's art: the shown tab's selected art even while disabled (A0
/// menus/23-fleet1-fighters: Fighters 10417 over a fleet with none aboard),
/// else the disabled or the normal art.
fn tab_resource(base: u32, tab: FleetWindowTab, selected: bool, enabled: bool) -> u32 {
    let k = tab.index();
    if selected {
        base + 7 + k
    } else if !enabled {
        base + 3 + k
    } else {
        base + k
    }
}

fn side_art(base: u32, side: u8) -> u32 {
    if side == 2 {
        base + EMPIRE_ART_OFFSET
    } else {
        base
    }
}

pub(crate) const fn fleet_side(is_alliance: bool) -> u8 {
    if is_alliance {
        1
    } else {
        2
    }
}

pub(crate) fn faction_side(faction: Faction) -> u8 {
    match faction {
        Faction::Alliance => 1,
        Faction::Empire => 2,
        Faction::Neutral => 0,
    }
}

/// hyp: `Contested` is side 3, and an uprising keeps its holder's side
/// (`ghidra/notes/blockade-bit.md`, "Port").
pub(crate) fn control_side(control: ControlKind) -> u8 {
    match control {
        ControlKind::Contested => 3,
        control => control.faction().map_or(0, faction_side),
    }
}

fn cockpit_faction(faction: CockpitFaction) -> Faction {
    match faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    }
}

fn in_rect(rect: (f32, f32, f32, f32), point: (f32, f32)) -> bool {
    point.0 >= rect.0 && point.0 < rect.0 + rect.2 && point.1 >= rect.1 && point.1 < rect.1 + rect.3
}

fn area_id(system: SystemKey) -> egui::Id {
    egui::Id::new(("original-fleet-window", system))
}

fn window_screen_rect(window: &OpenFleetWindow, layout: CockpitLayout) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            layout.canvas.x + f32::from(window.logical_position.0) * layout.scale,
            layout.canvas.y + f32::from(window.logical_position.1) * layout.scale,
        ),
        egui::vec2(
            FLEET_WINDOW_WIDTH * layout.scale,
            FLEET_WINDOW_HEIGHT * layout.scale,
        ),
    )
}

/// Where the pane art 10407 (or its side's) is placed so `tab`'s control
/// shows it as its backdrop. `FUN_004a6980` hands the strip the pane through
/// `FUN_0060da10(strip, pane, 99 - 97, 96 - 29)`: the strip's surface
/// becomes the pane from (2, 67), and each tab's backdrop (`+0xa4`) that
/// surface read at (0, 0) (`+0x10c` = `+0x110` = 0), whatever the tab's x.
/// So a tab's column c shows the pane at window x 99 + c: the pane's origin
/// moves by the tab's x in the strip. `FUN_00602d30` composes the art over
/// that backdrop, and only a 30-wide art (10409, 10412, ...) leaves its
/// column 30 showing: A0 fleet2/vjun-fleet5-admiral-* show the pane's
/// window (129, 96) black at (130, 96) and (129, 122) at (226, 122).
fn tab_backdrop_origin(tab: FleetWindowTab) -> (f32, f32) {
    let x = TAB_X[tab.index() as usize];
    (PANE_ORIGIN.0 + x, PANE_ORIGIN.1)
}

fn tab_rect(window: egui::Rect, scale: f32, tab: FleetWindowTab) -> egui::Rect {
    logical_rect(
        window,
        scale,
        TAB_STRIP.0 + TAB_X[tab.index() as usize],
        TAB_STRIP.1,
        TAB_SIZE.0,
        TAB_SIZE.1,
    )
}

#[derive(Default)]
struct WindowDrawResult {
    focus: bool,
    close: bool,
    minimize: bool,
    open_sector: bool,
    /// A left or right press on a left list entry, applied at once with
    /// its release for a click (`FUN_006083c0`).
    left_press: Option<LeftPress>,
    toggle: Option<FleetKey>,
    tab: Option<FleetWindowTab>,
    /// A left or right press on the right list: the row under it, the list
    /// point, the keys held and the rows listed (`FUN_006083c0`).
    item_press: Option<(Option<usize>, (f32, f32), Modifiers, Vec<Option<RightKey>>)>,
    drag: Option<ItemDrag>,
    /// The object, release point and right list row of a right release.
    object_menu: Option<(MenuObject, egui::Pos2, Option<usize>)>,
    rename: Option<RenameOutcome>,
    /// A primary click this frame: where and when it landed in the left
    /// list, or `None` outside it or for a double click.
    last_click: Option<Option<((i32, i32), f64)>>,
    /// The pointer's travel since a press on the window's surface.
    travel: Option<egui::Vec2>,
    /// The right list's scroll and bar hold after this frame.
    right_scroll: Option<(f32, Option<crate::list_scroll_bar::Hold>)>,
    /// The left list's scroll and bar hold after this frame.
    left_scroll: Option<(f32, Option<crate::list_scroll_bar::Hold>)>,
}

/// A press or release on a left list entry: the entry, the list point, the
/// keys held, whether the press and the release came this frame, and the
/// list's cells.
#[derive(Debug, Clone, PartialEq)]
struct LeftPress {
    entry: FleetWindowEntry,
    point: (f32, f32),
    modifiers: Modifiers,
    press: bool,
    release: bool,
    cells: Vec<(FleetWindowEntry, (f32, f32), (f32, f32))>,
}

/// Draw every open Fleet window.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit state and rendering inputs at this UI boundary, as the system window does."
)]
pub fn draw_fleet_windows(
    ctx: &egui::Context,
    world: &GameWorld,
    movement: &MovementState,
    fog: &FogState,
    transport: &TroopTransportState,
    state: &mut FleetWindowState,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Vec<FleetWindowAction> {
    state.prepare_faction(faction);
    let windows = state.windows.clone();
    let arrivals = state.arrivals.clone();
    let focused_system = windows
        .last()
        .filter(|_| state.focus.holds())
        .map(|window| window.system);
    let ended = state.end_drag(ctx, layout.scale);
    let mut actions = Vec::new();
    let player = cockpit_faction(faction);
    for window in &windows {
        let entries = left_entries(world, movement, fog, player, window);
        let picked = picked_entries(window, &entries);
        match ended {
            Some(DragEnd::Drop {
                system,
                left: Some(object),
                point,
            }) if system == window.system => {
                // A press on a selected entry holds its click, so the drag
                // carries the whole selection (flag 0x10).
                let objects: Vec<ItemObject> =
                    if picked.iter().any(|&entry| entry_object(world, entry) == object) {
                        picked.iter().map(|&entry| entry_object(world, entry)).collect()
                    } else {
                        vec![object]
                    };
                actions.extend(left_drop(
                    &objects,
                    point,
                    crate::drag_list::modifiers(ctx).ctrl,
                ));
                if let Some(open) = state.window_mut(system) {
                    open.selected.dropped();
                }
            }
            Some(DragEnd::Drop {
                system,
                left: None,
                point,
            }) if system == window.system => {
                let items = selection_items(world, movement, transport, &arrivals, &picked, window.tab);
                // An object in hyperspace takes no order (FUN_004f9860).
                let objects: Vec<ItemObject> = selected_rows(&window.selected_items, &right_keys(&items))
                    .into_iter()
                    .filter(|row| !items[*row].en_route)
                    .filter_map(|row| items[row].object)
                    .filter(|object| !matches!(object, ItemObject::TravellingFighter { .. }))
                    .collect();
                actions.extend(right_drop(
                    &objects,
                    point,
                    crate::drag_list::modifiers(ctx).ctrl,
                ));
                if let Some(open) = state.window_mut(system) {
                    open.selected_items.dropped();
                }
            }
            Some(DragEnd::Release { system, point }) if system == window.system => {
                let keys = right_keys(&selection_items(
                    world,
                    movement,
                    transport,
                    &arrivals,
                    &picked,
                    window.tab,
                ));
                let local = window_screen_rect(window, layout);
                let at = (
                    (point.x - local.min.x) / layout.scale - RIGHT_LIST.0,
                    (point.y - local.min.y) / layout.scale - RIGHT_LIST.1 + window.right_scroll,
                );
                let hit = right_hit(at, keys.len()).and_then(|row| keys[row]);
                let modifiers = crate::drag_list::modifiers(ctx);
                if let Some(open) = state.window_mut(system) {
                    open.selected_items
                        .release(hit, at, modifiers, &right_key_cells(&keys));
                }
            }
            _ => {}
        }
        let mut result = draw_fleet_window(
            ctx,
            world,
            movement,
            fog,
            transport,
            &arrivals,
            window,
            focused_system == Some(window.system),
            faction,
            layout,
            cache,
            state.drag_offset(ctx, window.system),
        );
        let system = window.system;
        if let Some(drag) = result.drag {
            state.drag = Some(drag);
        }
        if let Some((selection, point, row)) = result.object_menu {
            // The menu takes the hit list's selection (slot +0x58,
            // FUN_004a2c80), the clicked item first.
            // Squadrons of one class share their object, so each selected
            // item stays in the team.
            let items = selection_items(world, movement, transport, &arrivals, &picked, window.tab);
            let rows = selected_rows(&window.selected_items, &right_keys(&items));
            let left: Vec<MenuObject> = picked
                .iter()
                .map(|&entry| entry_object(world, entry).menu_object())
                .collect();
            let team = if row.is_none() && left.len() > 1 && left.contains(&selection) {
                // A left entry's menu takes the left list's selection.
                crate::drag_list::clicked_first(selection, left)
            } else if row.is_some_and(|row| rows.contains(&row)) {
                crate::drag_list::clicked_first(
                    selection,
                    rows.iter()
                        .filter_map(|&index| items[index].object.map(ItemObject::menu_object)),
                )
            } else {
                vec![selection]
            };
            actions.push(FleetWindowAction::OpenObjectMenu {
                system: window.system,
                selection,
                team,
                point: canvas_point(layout, point),
            });
        }
        if result.close || !world.systems.contains_key(system) {
            state.close(system);
            continue;
        }
        if result.minimize {
            if let Some(logical_position) = state.hide(system) {
                actions.push(FleetWindowAction::Minimize {
                    system,
                    logical_position,
                });
            }
            continue;
        }
        if result.open_sector {
            actions.push(FleetWindowAction::OpenSector(system));
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
                        FLEET_WINDOW_WIDTH,
                        FLEET_WINDOW_HEIGHT,
                    )
                },
            );
            if let Some((first, hold)) = result.right_scroll {
                open.right_scroll = first;
                open.right_hold = hold;
            }
            if let Some((first, hold)) = result.left_scroll {
                open.left_scroll = first;
                open.left_hold = hold;
            }
            if let Some(press) = result.left_press {
                let before = open.selected.clone();
                let hit = Some(press.entry);
                if press.press {
                    open.selected.press(hit, press.point, press.modifiers, &press.cells);
                }
                if press.release {
                    open.selected.release(hit, press.point, press.modifiers, &press.cells);
                }
                // FUN_004a5c00: while one of the viewer's entries is
                // selected, the other side's are not (item `+0x68` bit 8,
                // set for the viewer's side: without one, the picture takes
                // the other side's art).
                let own = |entry: &FleetWindowEntry| {
                    world
                        .fleets
                        .get(entry.fleet())
                        .is_some_and(|value| fleet_side(value.is_alliance) == faction_side(player))
                };
                if open.selected.items().iter().any(own) {
                    open.selected.retain(own);
                }
                if open.selected != before {
                    open.selected_items.clear();
                }
                // The right list keeps its place, clamped (`0x29b`,
                // `FUN_004a6390` → `FUN_004a5c00`).
                let now = picked_entries(open, &entries);
                open.tab = tab_for(&now, open.tab);
            }
            if roster_changed(world, open) {
                open.selected.clear();
                open.selected_items.clear();
            }
            open.roster = selected_roster(world, open.selected.items());
            if let Some(click) = result.last_click {
                open.last_click = click;
            }
            if let Some(fleet) = result.toggle {
                if let Some(index) = open.expanded.iter().position(|&key| key == fleet) {
                    open.expanded.remove(index);
                } else {
                    open.expanded.push(fleet);
                }
            }
            if let Some(tab) = result.tab {
                open.show_tab(tab);
            }
            if let Some((hit, point, modifiers, keys)) = result.item_press {
                open.selected_items
                    .press(hit.and_then(|row| keys[row]), point, modifiers, &right_key_cells(&keys));
            }
            // A unit no longer listed is no longer selected.
            let now = picked_entries(open, &entries);
            let keys = right_keys(&selection_items(world, movement, transport, &arrivals, &now, open.tab));
            open.selected_items.retain(|key| keys.contains(&Some(*key)));
            match result.rename {
                Some(RenameOutcome::Typed(text)) => {
                    if let Some(edit) = &mut open.rename {
                        edit.text = text;
                        edit.fresh = false;
                    }
                }
                Some(RenameOutcome::Commit(name)) => {
                    if let Some(edit) = open.rename.take() {
                        actions.push(FleetWindowAction::Rename {
                            entry: edit.entry,
                            name,
                        });
                    }
                }
                Some(RenameOutcome::Cancel) => open.rename = None,
                None => {}
            }
        }
        if result.focus {
            state.focus(system);
            actions.push(FleetWindowAction::SelectSystem(system));
        }
    }
    actions
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Keep the window's ordered paint and input pass together, as the system window does."
)]
fn draw_fleet_window(
    ctx: &egui::Context,
    world: &GameWorld,
    movement: &MovementState,
    fog: &FogState,
    transport: &TroopTransportState,
    arrivals: &[(MissionMember, FleetKey)],
    window: &OpenFleetWindow,
    focused: bool,
    faction: CockpitFaction,
    layout: CockpitLayout,
    cache: &mut BmpCache,
    drag_image: Option<DragImage>,
) -> WindowDrawResult {
    let mut result = WindowDrawResult::default();
    let Some(system) = world.systems.get(window.system) else {
        result.close = true;
        return result;
    };
    let player = cockpit_faction(faction);
    let viewer = faction_side(player);
    let other = if viewer == 1 { 2 } else { 1 };
    let entries = left_entries(world, movement, fog, player, window);
    let fleets: Vec<FleetKey> = entries
        .iter()
        .filter_map(|entry| match entry {
            FleetWindowEntry::Fleet(fleet) => Some(*fleet),
            FleetWindowEntry::Ship { .. }
            | FleetWindowEntry::Product { .. }
            | FleetWindowEntry::Copy { .. } => None,
        })
        .collect();
    let picked = picked_entries(window, &entries);
    // One entry alone selected; several show a fleet picture with no name
    // (`FUN_004a5c00`: `FUN_00601aa0(0)`, `+0x14c` bit 8 clear).
    let selected = match picked.as_slice() {
        [one] => Some(*one),
        _ => None,
    };
    // The picture's subject: the last selected entry in list order.
    let subject = picked.last().copied();
    let picture_fleet = match picked.as_slice() {
        [FleetWindowEntry::Fleet(fleet)] => Some(*fleet),
        [_, _, ..] => subject.map(FleetWindowEntry::fleet),
        _ => None,
    };
    let selected_side = subject.and_then(|entry| {
        world
            .fleets
            .get(entry.fleet())
            .map(|value| fleet_side(value.is_alliance))
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
            paint(
                cache,
                title_resource(title_side(world, &fleets, player, window.system), focused),
                2.0,
                2.0,
            );
            // The title label starts after the restore-sector button: its
            // width plus 5, at y 2, 16 high, in font 5, black, format 0x24
            // (one line, left-aligned, vertically centred; FUN_004a4b10).
            crate::theme::paint_gdi_text(
                &painter,
                logical_rect(local, scale, 19.0, 2.0, 180.0, 16.0),
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
                // The Fleet window blits its buttons plain: A0
                // fleet2/umgul-empty and system-windows/09-fleet show each
                // one's green key pixel (0, 255, 0) at its (x, 16), the
                // close box's too, which other windows key (FUN_004a4b10
                // makes them over the surface `+0xa4`). 10108/10109's first
                // DIB pixel is that green.
                if normal == CLOSE_NORMAL {
                    paint_key_pixel(
                        &painter,
                        ctx,
                        cache,
                        local,
                        scale,
                        x,
                        16.0,
                        KeyPixel::Colour(CLOSE_KEY),
                    );
                }
                clicked[index] = exact_clicked(&response, rect);
            }
            result.open_sector = clicked[0];
            result.minimize = clicked[1];
            result.close = clicked[2];

            // The right pane shows only while something is selected: the
            // player's art for its own side, the other art otherwise.
            if let Some(side) = selected_side {
                let art_side = if side == viewer { viewer } else { other };
                paint(
                    cache,
                    side_art(PANE_ART, art_side),
                    PANE_ORIGIN.0,
                    PANE_ORIGIN.1,
                );
            }

            // The left list: fleet entries, and ships while expanded.
            let list = logical_rect(
                local,
                scale,
                LEFT_LIST.0,
                LEFT_LIST.1,
                LEFT_LIST.2,
                LEFT_LIST.3,
            );
            let list_painter = painter.with_clip_rect(list);
            // Both lists write in font 10, white (FUN_004a4b10: the list's
            // slot +0x18 with 10, FUN_00609940/FUN_00609950 with 0xffffff).
            let font = crate::theme::gdi_font(10, scale);
            let click = ctx.input(|input| {
                input
                    .pointer
                    .primary_clicked()
                    .then(|| input.pointer.interact_pos())
                    .flatten()
                    .map(|point| (point, input.time))
            });
            let delay = ctx.options(|options| options.input_options.max_double_click_delay);
            let mut double = false;
            if let Some((point, time)) = click {
                let here =
                    rect_contains(list, point).then(|| (logical_point(local, scale, point), time));
                double =
                    here.is_some_and(|here| win32_double_click(window.last_click, here, delay));
                result.last_click = Some(here.filter(|_| !double));
            }
            // The left list's bar (FUN_004a4b10: FUN_0060a490 with 0x29cb,
            // 266 high), as the right list's.
            let left_content = left_content(entries.len());
            let left_bar = crate::list_scroll_bar::ScrollBar::new(
                LIST_BAR_ART,
                LEFT_LIST.0,
                LEFT_LIST.1,
                LEFT_LIST.2,
                LEFT_LIST.3,
                left_content,
                window.left_scroll,
            );
            let left_scroll = if left_bar.visible() {
                window.left_scroll.clamp(
                    0.0,
                    crate::list_scroll_bar::max_first(left_content, LEFT_LIST.3),
                )
            } else {
                0.0
            };
            let left_bar_rect = {
                let (bx, by, bw, bh) = left_bar.rect();
                logical_rect(local, scale, bx, by, bw, bh)
            };
            // While a left list drag is held, the selected entry's image
            // follows the pointer (FUN_00609960 copies each selected item's
            // selected image, FUN_0060bd00, into the list-sized drag bitmap;
            // FUN_0060df00 moves it), its text not included.
            for (row, entry) in entries.iter().enumerate() {
                let Some(top) = left_row_top(row, left_scroll) else {
                    continue;
                };
                let item = logical_rect(
                    local,
                    scale,
                    LEFT_LIST.0,
                    top,
                    LEFT_LIST.2,
                    LEFT_ITEM_HEIGHT,
                );
                // The image and whether it takes the selected frame.
                let mut targets = vec![(list_painter.clone(), item, picked.contains(entry))];
                if let Some(image) = drag_image.filter(|image| image.holds(*entry)) {
                    targets.push((
                        ctx.layer_painter(drag_layer())
                            .with_clip_rect(list.translate(image.offset)),
                        item.translate(image.offset),
                        true,
                    ));
                }
                let side = world
                    .fleets
                    .get(entry.fleet())
                    .map_or(1, |value| fleet_side(value.is_alliance));
                let expanded = window.expanded.contains(&entry.fleet());
                match *entry {
                    FleetWindowEntry::Fleet(fleet) => {
                        // The lines go first: A0 menus/23 draws the
                        // selection frame over the branch at (7, 35).
                        paint_line(&list_painter, item, scale, (2.0, 6.0), (5.0, 6.0));
                        if expanded {
                            paint_line(&list_painter, item, scale, (2.0, 6.0), (2.0, 50.0));
                        }
                        // hyp: the entry shows its ships' indicators, as
                        // A0 menus/23 marks Fleet 1's regiments under its
                        // frame at (41, 33) (FUN_004a3d40 draws the ships').
                        let flags = ship_contents(world, transport, arrivals, fleet)
                            .into_iter()
                            .fold(0, |all, flags| all | flags);
                        for (target, item, chosen) in &targets {
                            let item = *item;
                            if *chosen {
                                paint_native(
                                    target,
                                    ctx,
                                    cache,
                                    DllSource::Strategy,
                                    side_art(FLEET_SELECTED, side),
                                    item,
                                    scale,
                                    0.0,
                                    0.0,
                                );
                            }
                            for id in fleet_entry_layers(
                                side,
                                movement.is_in_transit(fleet),
                                fleet_damaged(world, fleet),
                            ) {
                                paint_native(
                                    target,
                                    ctx,
                                    cache,
                                    DllSource::Strategy,
                                    id,
                                    item,
                                    scale,
                                    5.0,
                                    5.0,
                                );
                            }
                            paint_contents(target, ctx, cache, item, scale, side, flags, 33.0);
                            if fleet_stranded(world, fleet) {
                                paint_native(
                                    target,
                                    ctx,
                                    cache,
                                    DllSource::Strategy,
                                    side_art(NO_HYPERDRIVE, side),
                                    item,
                                    scale,
                                    9.0,
                                    33.0,
                                );
                            }
                        }
                        if let Some(label) = fleet_label(world, fleet) {
                            // port: FUN_004c7e10's item draws the name;
                            // its rectangle is untraced. A0
                            // system-windows/09-fleet sets "Fleet 4" with its
                            // pen 7 into the row and its line's top 3 down
                            // (the window's x 11 and baseline 43, tmAscent
                            // 11, from the list at (4, 29)).
                            crate::theme::paint_gdi_text(
                                &list_painter,
                                logical_rect(item, scale, 7.0, 3.0, 84.0, 14.0),
                                scale,
                                10,
                                &label,
                                egui::Color32::WHITE,
                                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
                            );
                        }
                    }
                    FleetWindowEntry::Copy { .. } => {
                        // A copied fleet's row, as the side saw it
                        // (FUN_004f9a60): its side's art and its name.
                        let copy = entry.copy(world);
                        let side = copy.map_or(side, |copy| fleet_side(copy.is_alliance));
                        paint_line(&list_painter, item, scale, (2.0, 6.0), (5.0, 6.0));
                        for (target, item, chosen) in &targets {
                            let item = *item;
                            if *chosen {
                                paint_native(
                                    target,
                                    ctx,
                                    cache,
                                    DllSource::Strategy,
                                    side_art(FLEET_SELECTED, side),
                                    item,
                                    scale,
                                    0.0,
                                    0.0,
                                );
                            }
                            for id in fleet_entry_layers(side, false, false) {
                                paint_native(
                                    target,
                                    ctx,
                                    cache,
                                    DllSource::Strategy,
                                    id,
                                    item,
                                    scale,
                                    5.0,
                                    5.0,
                                );
                            }
                        }
                        if let Some(copy) = copy {
                            crate::theme::paint_gdi_text(
                                &list_painter,
                                logical_rect(item, scale, 7.0, 3.0, 84.0, 14.0),
                                scale,
                                10,
                                &copy.name,
                                egui::Color32::WHITE,
                                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
                            );
                        }
                    }
                    FleetWindowEntry::Product { fleet, product } => {
                        let last = entries
                            .get(row + 1)
                            .is_none_or(|next| next.fleet() != fleet);
                        if last {
                            paint_line(&list_painter, item, scale, (2.0, 0.0), (2.0, 6.0));
                            paint_line(&list_painter, item, scale, (2.0, 6.0), (5.0, 6.0));
                        } else {
                            paint_line(&list_painter, item, scale, (2.0, 0.0), (2.0, 50.0));
                        }
                        let value = world.products.get(product);
                        let class = value.and_then(|value| match value.kind {
                            rebellion_core::manufacturing::BuildableKind::CapitalShip(class) => {
                                world.capital_ship_classes.get(class)
                            }
                            _ => None,
                        });
                        if let (Some(value), Some(class)) = (value, class) {
                            let name = value.name.as_deref().unwrap_or(&class.name);
                            crate::theme::paint_gdi_text(
                                &list_painter,
                                logical_rect(item, scale, 7.0, 3.0, 84.0, 14.0),
                                scale,
                                10,
                                name,
                                egui::Color32::WHITE,
                                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
                            );
                        }
                        for (target, item, _) in targets.iter().filter(|(_, _, chosen)| *chosen) {
                            paint_native(
                                target,
                                ctx,
                                cache,
                                DllSource::Strategy,
                                side_art(FLEET_SELECTED, side),
                                *item,
                                scale,
                                0.0,
                                0.0,
                            );
                        }
                        if let (Some(value), Some(mini)) =
                            (value, class.and_then(|class| capital_ship_mini_id(class.dat_id)))
                        {
                            // Under construction: the unfinished mark last
                            // (FUN_0042c3b0); on its trip: the en route one.
                            let travelling = world.products.arrival(product).is_some();
                            let mut marks = mini_marks(MiniObject::Craft, mini, travelling, false);
                            if !travelling {
                                marks.push(unfinished_mini_mark(value.is_alliance));
                            }
                            for (target, item, _) in &targets {
                                for &(source, id) in marks
                                    .iter()
                                    .chain(std::iter::once(&(DllSource::Gokres, mini)))
                                {
                                    paint_native(
                                        target, ctx, cache, source, id, *item, scale, 5.0, 15.0,
                                    );
                                }
                            }
                        }
                    }
                    FleetWindowEntry::Ship { fleet, index } => {
                        let last = entries
                            .get(row + 1)
                            .is_none_or(|next| next.fleet() != fleet);
                        if last {
                            paint_line(&list_painter, item, scale, (2.0, 0.0), (2.0, 6.0));
                            paint_line(&list_painter, item, scale, (2.0, 6.0), (5.0, 6.0));
                        } else {
                            paint_line(&list_painter, item, scale, (2.0, 0.0), (2.0, 50.0));
                        }
                        let class = world
                            .fleets
                            .get(fleet)
                            .and_then(|value| value.capital_ships.get(index))
                            .and_then(|ship| world.capital_ship_classes.get(ship.class));
                        if let Some(class) = class {
                            // A0 fleet3/01: a ship's entry names it where a
                            // fleet's does (cap height at the row's y 6,
                            // from x 12), clipped by the list ("Medium
                            // Transpor"); the name is FUN_004f6270's.
                            let name = world.ship_name(fleet, index).unwrap_or(&class.name);
                            crate::theme::paint_gdi_text(
                                &list_painter,
                                logical_rect(item, scale, 7.0, 3.0, 84.0, 14.0),
                                scale,
                                10,
                                name,
                                egui::Color32::WHITE,
                                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
                            );
                        }
                        // A0 fleet2/umgul-corvette1-selected: a selected
                        // ship's entry takes the fleet's frame 10401 at its
                        // origin, over the tree line; A0 2026-11-13-drag/25
                        // keeps its right edge over a long name.
                        for (target, item, _) in targets.iter().filter(|(_, _, chosen)| *chosen) {
                            paint_native(
                                target,
                                ctx,
                                cache,
                                DllSource::Strategy,
                                side_art(FLEET_SELECTED, side),
                                *item,
                                scale,
                                0.0,
                                0.0,
                            );
                        }
                        if let Some(class) = class {
                            if let Some(mini) = capital_ship_mini_id(class.dat_id) {
                                let damaged = world
                                    .fleets
                                    .get(fleet)
                                    .and_then(|value| value.capital_ships.get(index))
                                    .is_some_and(|ship| crate::status_rows::damaged(ship, class));
                                let marks = mini_marks(
                                    MiniObject::Craft,
                                    mini,
                                    movement.is_in_transit(fleet),
                                    damaged,
                                );
                                for (target, item, _) in &targets {
                                    for &(source, id) in marks
                                        .iter()
                                        .chain(std::iter::once(&(DllSource::Gokres, mini)))
                                    {
                                        paint_native(
                                            target,
                                            ctx,
                                            cache,
                                            source,
                                            id,
                                            *item,
                                            scale,
                                            5.0,
                                            15.0,
                                        );
                                    }
                                }
                            }
                            let flags = ship_contents(world, transport, arrivals, fleet)
                                .get(index)
                                .copied()
                                .unwrap_or(0);
                            let stranded = world
                                .fleets
                                .get(fleet)
                                .and_then(|value| value.capital_ships.get(index))
                                .is_some_and(|ship| stranded(class, ship));
                            for (target, item, _) in &targets {
                                let item = *item;
                                paint_contents(target, ctx, cache, item, scale, side, flags, 33.0);
                                if stranded {
                                    paint_native(
                                        target,
                                        ctx,
                                        cache,
                                        DllSource::Strategy,
                                        side_art(NO_HYPERDRIVE, side),
                                        item,
                                        scale,
                                        9.0,
                                        33.0,
                                    );
                                }
                            }
                        }
                    }
                }
                // The bar takes its own presses, over the entry's edge.
                let entry_area = if left_bar.visible() {
                    egui::Rect::from_min_max(
                        item.min,
                        egui::pos2(item.max.x.min(left_bar_rect.min.x), item.max.y),
                    )
                } else {
                    item
                };
                let response = ui.interact(
                    entry_area.intersect(list),
                    ui.id().with((window.system, "entry", row)),
                    egui::Sense::click(),
                );
                // A press on an entry, in list pixels (FUN_00609d80), with
                // the list's cells for a Shift range (FUN_00609590).
                let left_press = |point: egui::Pos2, press, release| LeftPress {
                    entry: *entry,
                    point: (
                        (point.x - local.min.x) / scale - LEFT_LIST.0,
                        (point.y - local.min.y) / scale - LEFT_LIST.1 + left_scroll,
                    ),
                    modifiers: crate::drag_list::modifiers(ui.ctx()),
                    press,
                    release,
                    cells: entries
                        .iter()
                        .enumerate()
                        .map(|(row, entry)| {
                            (
                                *entry,
                                (0.0, LEFT_ITEM_HEIGHT * row as f32),
                                (LEFT_LIST.2, LEFT_ITEM_HEIGHT),
                            )
                        })
                        .collect(),
                };
                // A left or right press applies at once (FUN_006083c0, case
                // 0x201/0x204: an unselected item is selected alone, with
                // Ctrl toggled, with Shift added; A0 2026-11-13-drag/25
                // frames the pressed row while the button is held); the
                // left release applies a click a press on a selected item
                // held (flag 0x10).
                let pressed_at = ui.ctx().input(|input| {
                    (input.pointer.button_pressed(egui::PointerButton::Primary)
                        || input.pointer.button_pressed(egui::PointerButton::Secondary))
                    .then(|| input.pointer.press_origin())
                    .flatten()
                    .filter(|point| rect_contains(entry_area, *point))
                });
                let released = exact_clicked(&response, entry_area);
                let pressed = pressed_at.filter(|_| {
                    response.is_pointer_button_down_on() || released || response.secondary_clicked()
                });
                if let Some(point) = pressed {
                    result.left_press = Some(left_press(point, true, released));
                    result.focus = true;
                } else if released {
                    let point = response
                        .interact_pointer_pos()
                        .or_else(|| click.map(|(point, _)| point))
                        .unwrap_or_else(|| entry_area.center());
                    result.left_press = Some(left_press(point, false, true));
                    result.focus = true;
                }
                if response.clicked() && double {
                    if let FleetWindowEntry::Fleet(fleet) = entry {
                        result.toggle = Some(*fleet);
                    }
                }
                let object = entry_object(world, *entry);
                if let (true, Some(point)) = (
                    response.secondary_clicked(),
                    response.interact_pointer_pos(),
                ) {
                    // A0 fleet3/02, 08: the right click selects the entry
                    // it opens the menu for (Medium Transport 1, Fleet 7);
                    // on a selected entry it keeps the selection (0x204).
                    // The press above applied it.
                    result.object_menu = Some((object.menu_object(), point, None));
                }
                // A drag out of either list posts 0x29a (FUN_006083c0).
                if let Some(press) = ui.ctx().input(|input| {
                    input
                        .pointer
                        .button_pressed(egui::PointerButton::Primary)
                        .then(|| input.pointer.press_origin())
                        .flatten()
                }) {
                    if response.is_pointer_button_down_on() {
                        result.drag = Some(ItemDrag {
                            system: window.system,
                            left: Some(object),
                            press,
                        });
                    }
                }
            }
            // The bar (FUN_0060a490) shows once the entries pass the list's
            // height; its arrows, track and thumb scroll it.
            if left_bar.visible() {
                let mut hold = window.left_hold;
                let first = crate::list_scroll_bar::ui_bar(
                    ui,
                    cache,
                    local,
                    scale,
                    LIST_BAR_ART,
                    LEFT_LIST,
                    left_content,
                    left_scroll,
                    &mut hold,
                    LIST_STEP,
                );
                result.left_scroll = Some((first, hold));
            } else {
                result.left_scroll = reset_when_hidden(window.left_scroll, window.left_hold);
            }
            if let Some(edit) = &window.rename {
                result.rename = draw_rename_edit(
                    ui,
                    window,
                    edit,
                    &entries,
                    local,
                    list,
                    scale,
                    &font,
                    left_scroll,
                );
            }

            if let Some(side) = selected_side {
                // The picture: one fleet's 10425 centered, keyed over the
                // en route mark 10426 drawn first at the same left edge; or
                // one ship's portrait at the panel's origin (`FUN_004a5c00`).
                if let Some(fleet) = picture_fleet {
                    let picture = side_art(FLEET_PICTURE, side);
                    let width = cache
                        .original_resource_size(DllSource::Strategy, picture)
                        .map_or(PICTURE_PANEL.2, |size| size[0] as f32);
                    for id in fleet_picture_layers(
                        side,
                        movement.is_in_transit(fleet),
                        fleet_damaged(world, fleet),
                    ) {
                        paint(cache, id, picture_x(width), PICTURE_PANEL.1);
                    }
                    // The label +0x190 (FUN_004a4b10): (100, 29), 128 by 25,
                    // format 0 (left, top), font 5. The A0 capture shows
                    // "Fleet 2" red, bold, from x 101.
                    if let Some(label) = fleet_label(world, fleet).filter(|_| selected.is_some()) {
                        // DrawTextA without DT_NOCLIP clips to the label: A0
                        // fleet-aboard/10 cuts "Imperial Star Destroyer 1"
                        // at x 228.
                        let label_rect = logical_rect(local, scale, 100.0, 29.0, 128.0, 25.0);
                        crate::theme::paint_gdi_text(
                            &painter.with_clip_rect(label_rect.intersect(painter.clip_rect())),
                            label_rect,
                            scale,
                            5,
                            &label,
                            label_color(viewer),
                            (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
                        );
                    }
                }
                if let Some(FleetWindowEntry::Ship { fleet, index }) = selected {
                    // A0 fleet2/umgul-corvette1-selected: the label +0x190
                    // names the ship as it does a fleet ("Corellian
                    // Corvette 1", FUN_004f6270).
                    let name = world.fleets.get(fleet).and_then(|value| {
                        let ship = value.capital_ships.get(index)?;
                        let class = world.capital_ship_classes.get(ship.class)?;
                        Some(world.ship_name(fleet, index).unwrap_or(&class.name).to_owned())
                    });
                    if let Some(name) = name {
                        // DrawTextA without DT_NOCLIP clips to the label: A0
                        // fleet-aboard/10 cuts "Imperial Star Destroyer 1"
                        // at x 228.
                        let label_rect = logical_rect(local, scale, 100.0, 29.0, 128.0, 25.0);
                        crate::theme::paint_gdi_text(
                            &painter.with_clip_rect(label_rect.intersect(painter.clip_rect())),
                            label_rect,
                            scale,
                            5,
                            &name,
                            label_color(viewer),
                            (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
                        );
                    }
                    let ship = world
                        .fleets
                        .get(fleet)
                        .and_then(|value| value.capital_ships.get(index))
                        .and_then(|ship| Some((ship, world.capital_ship_classes.get(ship.class)?)));
                    let mini = ship.and_then(|(_, class)| capital_ship_mini_id(class.dat_id));
                    if let (Some(mini), Some((ship, class))) = (mini, ship) {
                        // FUN_0042c3b0(.., 1, 1): the marks under the
                        // portrait, which is blitted keyed over them.
                        let portrait = ship_portrait(mini);
                        let mut layers = Vec::new();
                        if movement.is_in_transit(fleet) {
                            layers.push(en_route_portrait_mark(portrait));
                        }
                        if crate::status_rows::damaged(ship, class) {
                            layers.push(portrait + 0x2000);
                        }
                        layers.push(portrait);
                        // The panel is a 125 by 49 bitmap; the 50-row
                        // portrait loses its last row.
                        let panel = painter.with_clip_rect(logical_rect(
                            local,
                            scale,
                            PICTURE_PANEL.0,
                            PICTURE_PANEL.1,
                            PICTURE_PANEL.2,
                            PICTURE_PANEL.3,
                        ));
                        for id in layers {
                            paint_native(
                                &panel,
                                ctx,
                                cache,
                                DllSource::Gokres,
                                id,
                                local,
                                scale,
                                PICTURE_PANEL.0,
                                PICTURE_PANEL.1,
                            );
                        }
                    }
                }
                if let Some((aboard, capacity)) = selection_counts(world, movement, transport, &picked, window.tab)
                {
                    // The count fonts +0x1a4 and +0x1a8 are font 10
                    // (FUN_006002b0(10), FUN_004a4b10).
                    // The aboard count from the panel's left, the capacity
                    // ending 2 short of its right (DT_RIGHT).
                    crate::theme::paint_gdi_text(
                        &painter,
                        logical_rect(
                            local,
                            scale,
                            PICTURE_PANEL.0 + 1.0,
                            PICTURE_PANEL.1 + 2.0,
                            61.0,
                            14.0,
                        ),
                        scale,
                        10,
                        &aboard.to_string(),
                        egui::Color32::WHITE,
                        (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
                    );
                    crate::theme::paint_gdi_text(
                        &painter,
                        logical_rect(
                            local,
                            scale,
                            PICTURE_PANEL.0 + 62.0,
                            PICTURE_PANEL.1 + 2.0,
                            61.0,
                            14.0,
                        ),
                        scale,
                        10,
                        &capacity.to_string(),
                        egui::Color32::WHITE,
                        (crate::theme::GdiAlign::Right, crate::theme::GdiVAlign::Top),
                    );
                }

                // The strip: the player's base for its own side, the other
                // base otherwise.
                let strip_side = if side == viewer { viewer } else { other };
                let base = side_art(TAB_BASE, strip_side);
                for tab in shown_tabs(&picked) {
                    let enabled = selection_tab_enabled(world, movement, transport, arrivals, &picked, tab);
                    let rect = tab_rect(local, scale, tab);
                    let id = tab_resource(base, tab, tab == window.tab, enabled);
                    // The control clips the 29-row art to 28 (`FUN_00602d30`).
                    let tab_painter = painter.with_clip_rect(rect);
                    // Its backdrop first: what a 30-wide art leaves of the
                    // 31-wide control (column 30) shows it.
                    let (pane_x, pane_y) = tab_backdrop_origin(tab);
                    paint_native(
                        &tab_painter,
                        ctx,
                        cache,
                        DllSource::Strategy,
                        side_art(PANE_ART, strip_side),
                        local,
                        scale,
                        pane_x,
                        pane_y,
                    );
                    paint_native(
                        &tab_painter,
                        ctx,
                        cache,
                        DllSource::Strategy,
                        id,
                        rect,
                        scale,
                        0.0,
                        0.0,
                    );
                    let response =
                        ui.interact(rect, ui.id().with((window.system, tab)), egui::Sense::click());
                    // A disabled tab (FUN_004a6980) shows no tip.
                    if enabled {
                        crate::win_tooltip::response_tool(&response, rect, tab.tooltip_string());
                    }
                    // A disabled tab (state 0x40) still takes the click:
                    // A0 menus/23-fleet1-fighters and -personnel show the
                    // empty Fighters and Personnel lists of a fleet with
                    // neither aboard.
                    if exact_clicked(&response, rect) {
                        result.tab = Some(tab);
                        result.focus = true;
                    }
                }

                let right = logical_rect(
                    local,
                    scale,
                    RIGHT_LIST.0,
                    RIGHT_LIST.1,
                    RIGHT_LIST.2,
                    RIGHT_LIST.3,
                );
                let right_painter = painter.with_clip_rect(right);
                // Everything aboard a travelling fleet is en route
                // (`FUN_004f8240`).
                let travelling = !picked.is_empty()
                    && picked.iter().all(|entry| {
                        matches!(entry, FleetWindowEntry::Fleet(fleet) if movement.is_in_transit(*fleet))
                    });
                let shown_items = selection_items(world, movement, transport, arrivals, &picked, window.tab);
                let shown_keys = right_keys(&shown_items);
                let content = right_content(shown_items.len());
                let bar = crate::list_scroll_bar::ScrollBar::new(
                    LIST_BAR_ART,
                    RIGHT_LIST.0,
                    RIGHT_LIST.1,
                    RIGHT_LIST.2,
                    RIGHT_LIST.3,
                    content,
                    window.right_scroll,
                );
                let scroll = if bar.visible() { window.right_scroll.clamp(0.0, crate::list_scroll_bar::max_first(content, RIGHT_LIST.3)) } else { 0.0 };
                let (bx, by, bw, bh) = bar.rect();
                let bar_rect = logical_rect(local, scale, bx, by, bw, bh);
                for (row, item) in shown_items.iter().enumerate() {
                    let Some(top) = right_row_top(row, scroll) else {
                        continue;
                    };
                    let cell =
                        logical_rect(local, scale, RIGHT_LIST.0, top, 125.0, RIGHT_ITEM_HEIGHT);
                    // The item's image (FUN_004a6e70): the frame on the
                    // selected image only, the mini and the indicators. While
                    // a drag is held, the selected items' images follow the
                    // pointer (FUN_00609960 copies them into the list-sized
                    // drag bitmap, FUN_0060dbb0 / FUN_0060df00).
                    let chosen = shown_keys[row].is_some_and(|key| window.selected_items.contains(key));
                    let mut targets = vec![(right_painter.clone(), cell)];
                    let right_drag = drag_image.filter(|image| image.left.is_none()).map(|image| image.offset);
                    if let (true, Some(offset)) = (chosen, right_drag) {
                        targets.push((
                            ctx.layer_painter(drag_layer())
                                .with_clip_rect(right.translate(offset)),
                            cell.translate(offset),
                        ));
                    }
                    for (target, cell) in targets {
                        if chosen {
                            paint_native(
                                &target,
                                ctx,
                                cache,
                                DllSource::Strategy,
                                side_art(RIGHT_ITEM_SELECTED, side),
                                cell,
                                scale,
                                1.0,
                                1.0,
                            );
                        }
                        if let Some(mini) = item.mini {
                            let marks = mini_marks(
                                item.kind,
                                mini,
                                travelling || item.en_route,
                                item.damaged,
                            );
                            let captive = item.captive.then(|| captive_mark(mini));
                            // FUN_0042c3b0 lays the unfinished mark last
                            // under the mini: 0x2d32 (11570) for side 1,
                            // else 0x2d34 (11572) (A0 2026-11-15 "u-2": the
                            // Carrack on the Empire's green grid).
                            let unfinished = match item.object {
                                Some(
                                    ItemObject::Product(product)
                                    | ItemObject::ProductFighter { product, .. }
                                    | ItemObject::ProductTroop { product, .. },
                                ) if world.products.arrival(product).is_none() =>
                                {
                                    world
                                        .products
                                        .get(product)
                                        .map(|value| unfinished_mini_mark(value.is_alliance))
                                }
                                _ => None,
                            };
                            for (source, id) in marks
                                .into_iter()
                                .chain(unfinished)
                                .chain(std::iter::once((DllSource::Gokres, mini)))
                                .chain(captive)
                            {
                                paint_native(&target, ctx, cache, source, id, cell, scale, 28.0, 4.0);
                            }
                        }
                        paint_contents(&target, ctx, cache, cell, scale, side, item.contents, 23.0);
                        if item.no_hyperdrive {
                            paint_native(
                                &target,
                                ctx,
                                cache,
                                DllSource::Strategy,
                                side_art(NO_HYPERDRIVE, side),
                                cell,
                                scale,
                                9.0,
                                23.0,
                            );
                        }
                    }
                    // The edit stands in for the name being renamed.
                    let editing = window
                        .rename
                        .as_ref()
                        .is_some_and(|edit| edit.item == Some(row));
                    if !editing {
                        crate::theme::paint_gdi_text(
                            &right_painter,
                            logical_rect(cell, scale, 0.0, 37.0, 125.0, 14.0),
                            scale,
                            10,
                            &item.label,
                            egui::Color32::WHITE,
                            (crate::theme::GdiAlign::Center, crate::theme::GdiVAlign::Top),
                        );
                    }
                    // The bar takes its own presses, over the cell's edge.
                    let cell_area = if bar.visible() {
                        egui::Rect::from_min_max(
                            cell.min,
                            egui::pos2(cell.max.x.min(bar_rect.min.x), cell.max.y),
                        )
                    } else {
                        cell
                    };
                    let response = ui.interact(
                        cell_area.intersect(right),
                        ui.id().with((window.system, "item", row)),
                        egui::Sense::click(),
                    );
                    if response.secondary_clicked() {
                        if let (Some(object), Some(point)) =
                            (item.object, response.interact_pointer_pos())
                        {
                            result.object_menu = Some((object.menu_object(), point, Some(row)));
                        }
                    }
                }
                // A left or right press on the list selects by the
                // CoolDragList rules (FUN_006083c0), and a left press
                // captures the mouse for a drag.
                let (left_pressed, any_pressed, press_origin) = ui.ctx().input(|input| {
                    (
                        input.pointer.button_pressed(egui::PointerButton::Primary),
                        input.pointer.button_pressed(egui::PointerButton::Primary)
                            || input.pointer.button_pressed(egui::PointerButton::Secondary),
                        input.pointer.press_origin(),
                    )
                });
                let items_area = if bar.visible() {
                    egui::Rect::from_min_max(
                        right.min,
                        egui::pos2(right.max.x.min(bar_rect.min.x), right.max.y),
                    )
                } else {
                    right
                };
                if let Some(press) = press_origin.filter(|point| {
                    any_pressed
                        && rect_contains(items_area, *point)
                        && ui.ctx().layer_id_at(*point) == Some(ui.layer_id())
                }) {
                    let at = (
                        (press.x - local.min.x) / scale - RIGHT_LIST.0,
                        (press.y - local.min.y) / scale - RIGHT_LIST.1 + scroll,
                    );
                    result.item_press = Some((
                        right_hit(at, shown_items.len()),
                        at,
                        crate::drag_list::modifiers(ui.ctx()),
                        right_keys(&shown_items),
                    ));
                    result.focus = true;
                    if left_pressed {
                        result.drag = Some(ItemDrag {
                            system: window.system,
                            left: None,
                            press,
                        });
                    }
                }
                // The bar (FUN_0060a490) shows once the items pass the
                // list's height; its arrows, track and thumb scroll it.
                if bar.visible() {
                    let mut hold = window.right_hold;
                    let first = crate::list_scroll_bar::ui_bar(
                        ui,
                        cache,
                        local,
                        scale,
                        LIST_BAR_ART,
                        RIGHT_LIST,
                        content,
                        scroll,
                        &mut hold,
                        LIST_STEP,
                    );
                    result.right_scroll = Some((first, hold));
                } else {
                    result.right_scroll = reset_when_hidden(window.right_scroll, window.right_hold);
                }
            }

            if window_response.clicked() || clicked.iter().any(|&value| value) {
                result.focus = true;
            }
            // The child controls: the title buttons, the left list and,
            // while something is selected, the tabs and the right list.
            let mut controls: Vec<egui::Rect> = [3.0, 204.0, 218.0]
                .into_iter()
                .map(|x| logical_rect(local, scale, x, 3.0, 14.0, 14.0))
                .collect();
            controls.push(list);
            if selected_side.is_some() {
                controls.extend(shown_tabs(&picked).map(|tab| tab_rect(local, scale, tab)));
                controls.push(logical_rect(
                    local,
                    scale,
                    RIGHT_LIST.0,
                    RIGHT_LIST.1,
                    RIGHT_LIST.2,
                    RIGHT_LIST.3,
                ));
            }
            result.travel =
                window_move::held_travel(ui, &window_response, &controls, window.moving, scale);
        });
    if area.response.clicked() {
        result.focus = true;
    }
    result
}

/// The selected name's color: `0x20000ff` (red) for a side 1 viewer,
/// `0x200ff00` (green) otherwise (`FUN_004a4b10`).
fn label_color(viewer: u8) -> egui::Color32 {
    if viewer == 1 {
        egui::Color32::from_rgb(255, 0, 0)
    } else {
        egui::Color32::from_rgb(0, 255, 0)
    }
}

/// The rename edit over its entry: the entry's name line, white on the
/// list. hyp: the selection rectangle (`vtable+0x88`) is the entry's name
/// line; the field's font is font 10 (`vtable+0x18(10)`, unmapped).
#[expect(
    clippy::too_many_arguments,
    reason = "The edit needs the window's frame, list clip and font."
)]
fn draw_rename_edit(
    ui: &mut egui::Ui,
    window: &OpenFleetWindow,
    edit: &RenameEdit,
    entries: &[FleetWindowEntry],
    local: egui::Rect,
    list: egui::Rect,
    scale: f32,
    font: &egui::FontId,
    left_scroll: f32,
) -> Option<RenameOutcome> {
    let rect = if let Some(item) = edit.item {
        // The item's name line (+37, 125 wide) in the right list, where
        // the list shows it: FUN_004a71d0 takes the item's rect, which
        // FUN_00609e20 keeps less the first visible pixel (FUN_00609dd0).
        let top = RIGHT_LIST.1 + RIGHT_ITEM_HEIGHT * item as f32 - window.right_scroll + 37.0;
        let right = logical_rect(local, scale, RIGHT_LIST.0, RIGHT_LIST.1, RIGHT_LIST.2, RIGHT_LIST.3);
        logical_rect(local, scale, RIGHT_LIST.0, top, 125.0, 14.0).intersect(right)
    } else {
        let Some(row) = entries.iter().position(|entry| *entry == edit.entry) else {
            // The entry left the list: the edit goes with it.
            return Some(RenameOutcome::Cancel);
        };
        let top = LEFT_LIST.1 + LEFT_ITEM_HEIGHT * row as f32 - left_scroll;
        logical_rect(local, scale, LEFT_LIST.0 + 4.0, top + 3.0, 85.0, 15.0).intersect(list)
    };
    let id = ui.id().with((window.system, "rename"));
    let mut text = edit.text.clone();
    ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
    // The selected name shows black on white, the edit's system colours
    // (A0 menus3/41, 46); typed text is white on the list's black.
    let all_selected = edit.fresh
        || egui::TextEdit::load_state(ui.ctx(), id)
            .and_then(|state| state.cursor.char_range())
            .is_some_and(|range| {
                let (from, to) = (
                    range.primary.index.min(range.secondary.index),
                    range.primary.index.max(range.secondary.index),
                );
                from == 0 && to == text.chars().count() && to > 0
            });
    ui.visuals_mut().selection.bg_fill = egui::Color32::WHITE;
    // The right list centres its names; the left list starts them at the
    // entry's left.
    let align = if edit.item.is_some() {
        egui::Align::Center
    } else {
        egui::Align::Min
    };
    let response = ui.put(
        rect,
        egui::TextEdit::singleline(&mut text)
            .id(id)
            .frame(false)
            .font(font.clone())
            .text_color(if all_selected {
                egui::Color32::BLACK
            } else {
                egui::Color32::WHITE
            })
            .horizontal_align(align)
            .desired_width(rect.width()),
    );
    if edit.fresh {
        response.request_focus();
        let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(text.chars().count()),
            )));
        state.store(ui.ctx(), id);
        return Some(RenameOutcome::Typed(text));
    }
    let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
    let escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
    if enter {
        // FUN_004ac950: an empty name keeps the field open.
        if text.is_empty() {
            response.request_focus();
            return Some(RenameOutcome::Typed(text));
        }
        return Some(RenameOutcome::Commit(text));
    }
    // A press elsewhere ends the edit unissued (`FUN_004aca40`): the
    // window's own 0x201/0x204 and a child's 0x40e (`FUN_004ac3a0`), which
    // a list (`FUN_006083c0`) and its scroll bar (`FUN_0060aca0`) post on
    // a press. hyp: Escape too; the field's own key handling is untraced.
    if escape || response.lost_focus() {
        return Some(RenameOutcome::Cancel);
    }
    response.changed().then_some(RenameOutcome::Typed(text))
}

/// Blit a bitmap at its native size at a logical offset in `parent`
/// (`FUN_00602d30`).
#[expect(
    clippy::too_many_arguments,
    reason = "A blit names its painter, source, resource and position."
)]
#[expect(
    clippy::cast_precision_loss,
    reason = "Bitmap sizes are small integers."
)]
pub(crate) fn paint_native(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    source: DllSource,
    resource_id: u32,
    parent: egui::Rect,
    scale: f32,
    x: f32,
    y: f32,
) {
    #[cfg(test)]
    tests::PAINTED.with(|painted| {
        painted
            .borrow_mut()
            .push((resource_id, logical_rect(parent, scale, x, y, 0.0, 0.0).min));
    });
    let Some([width, height]) = cache.original_resource_size(source, resource_id) else {
        return;
    };
    let Some(texture) = cache
        .get(ctx, source, resource_id)
        .map(egui::TextureHandle::id)
    else {
        return;
    };
    painter.image(
        texture,
        logical_rect(parent, scale, x, y, width as f32, height as f32),
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

/// What a title button's key pixel shows (its first DIB pixel, the bottom-left
/// corner, the one pixel `FUN_005fd170` keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyPixel {
    /// The window's own background bitmap at that point: the button is a
    /// child blitted keyed over its parent (`FUN_00602d30` ->
    /// `FUN_005fd0f0`), the title strip a sibling that does not show through.
    Background(u32),
    /// The window's background bitmap at another point (`x`, `y`): the
    /// button composes its art over the background surface it was given,
    /// read at the button's own position in its parent (`FUN_00602d30`
    /// copies `+0xa4` from (`+0x10c`, `+0x110`)), and that parent is not
    /// the window.
    BackgroundAt(u32, u16, u16),
    /// The key colour itself: the button is blitted plain.
    Colour(egui::Color32),
}

/// Repaint the one key pixel at window point (`x`, `y`) as `key` says.
#[expect(
    clippy::too_many_arguments,
    reason = "The pixel needs paint_native's whole context."
)]
pub(crate) fn paint_key_pixel(
    painter: &egui::Painter,
    ctx: &egui::Context,
    cache: &mut BmpCache,
    parent: egui::Rect,
    scale: f32,
    x: f32,
    y: f32,
    key: KeyPixel,
) {
    #[cfg(test)]
    tests::KEY_PIXELS.with(|pixels| {
        pixels
            .borrow_mut()
            .push((key, logical_rect(parent, scale, x, y, 0.0, 0.0).min));
    });
    let pixel = logical_rect(parent, scale, x, y, 1.0, 1.0);
    match key {
        KeyPixel::Background(id) => paint_native(
            &painter.with_clip_rect(pixel),
            ctx,
            cache,
            DllSource::Strategy,
            id,
            parent,
            scale,
            0.0,
            0.0,
        ),
        KeyPixel::BackgroundAt(id, from_x, from_y) => paint_native(
            &painter.with_clip_rect(pixel),
            ctx,
            cache,
            DllSource::Strategy,
            id,
            parent,
            scale,
            x - f32::from(from_x),
            y - f32::from(from_y),
        ),
        // A mesh, which egui does not feather: a filled 1 by 1 rect
        // shows nothing at scale 1 (fleet-window.mjs a0-empty).
        KeyPixel::Colour(colour) => {
            let mut mesh = egui::Mesh::default();
            mesh.add_colored_rect(pixel, colour);
            painter.add(mesh);
        }
    }
}

/// A white tree line (`CreatePen(PS_DOT, 1, 0x2ffffff)`, `MoveTo`/`LineTo`)
/// from `from` up to but not including `to`, as GDI's `LineTo` leaves its
/// end point. A0 shows it solid: `fleet2/vjun-fleet5-admiral-ships` runs
/// x 6 from y 35 to 84 unbroken, and `menus/23-fleet1-ships` the branch at
/// y 85 over x 6..8. A mesh, which egui does not feather, so a one-pixel
/// line keeps its pixels at scale 1.
fn paint_line(
    painter: &egui::Painter,
    parent: egui::Rect,
    scale: f32,
    from: (f32, f32),
    to: (f32, f32),
) {
    let (x, y) = (from.0.min(to.0), from.1.min(to.1));
    let (width, height) = ((to.0 - from.0).abs().max(1.0), (to.1 - from.1).abs().max(1.0));
    let mut mesh = egui::Mesh::default();
    mesh.add_colored_rect(logical_rect(parent, scale, x, y, width, height), egui::Color32::WHITE);
    painter.add(mesh);
}

#[cfg(test)]
#[path = "fleet_window_tests.rs"]
pub(crate) mod tests;
