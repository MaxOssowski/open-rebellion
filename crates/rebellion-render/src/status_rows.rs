//! The Status window's contents for each object family: the title, the
//! picture, the name line and the rows its filler writes (`FUN_00443130`
//! picks the filler by the id's family). Recovery notes:
//! `ghidra/notes/status-window.md`.
//!
//! Every "current:maximum" value joins the two with the executable's ":"
//! (`DAT_006a872c`). Where the port keeps no damage or losses, the current
//! value is the maximum.

use rebellion_core::dat::Faction;
use rebellion_core::ids::{
    CharacterKey, DatId, DefenseFacilityKey, FighterKey, FleetKey, ManufacturingFacilityKey,
    ProductionFacilityKey, SpecialForceKey, SystemKey, TroopKey,
};
use rebellion_core::manufacturing::{BuildableKind, ManufacturingState, ProductionArea};
use rebellion_core::missions::{MissionFaction, MissionMember, MissionState, MissionTarget};
use rebellion_core::movement::MovementState;
use rebellion_core::squadrons::SquadronPlace;
use rebellion_core::troop_transport::{RegimentLeg, TroopTransportState};
use rebellion_core::world::{CapitalShipClass, GameWorld, ShipInstance};

use crate::bmp_cache::DllSource;
use crate::fleet_window::{aboard_label, first_seat, ship_cargo, ship_load};
use crate::mission_dialog::{kind_icon, kind_name};
use crate::status_window::{character_rows, StatusRow};
use crate::minis::{capital_ship_mini_id, fighter_mini_id};
use crate::system_window::{
    character_picture, defense_facility_mini, fleet_label, manufacturing_facility_mini,
    production_facility_mini, special_force_mini, troop_mini,
};

/// The state the fillers read.
#[derive(Clone, Copy)]
pub struct StatusSources<'a> {
    pub world: &'a GameWorld,
    pub missions: &'a MissionState,
    pub movement: &'a MovementState,
    pub transport: &'a TroopTransportState,
    pub manufacturing: &'a ManufacturingState,
    /// Today's day number (`FUN_004fd340`).
    pub today: u64,
    /// What the Objectives read.
    pub objectives: ObjectiveSource,
}

/// What the Objectives filler reads: the viewing side (the galaxy view's
/// `+0x9c`), its copy of the victory conditions word (side `+0xbc`,
/// `rebellion_core::victory::conditions_word`), and whether the game is
/// the standard one (game settings `+8 == 0`, `FUN_004fce50`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObjectiveSource {
    pub alliance: bool,
    pub conditions_word: u32,
    pub standard: bool,
}

/// The object a Status window shows, by the id families `FUN_00443130`
/// dispatches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusObject {
    /// `0x30..0x3b`, `FUN_004486f0`.
    Character(CharacterKey),
    /// `0x3c..0x3f`, `FUN_00445780`.
    SpecialForce(SpecialForceKey),
    /// `0x08..0x0f`, `FUN_00449200`.
    Fleet(FleetKey),
    /// `0x14..0x1b`, `FUN_00446fd0`: a fleet's ship by its index.
    Ship { fleet: FleetKey, index: usize },
    /// `0x1c..0x1f`, `FUN_004444c0`: a fleet's squadron entry by its index.
    /// port: squadrons live on their fleet, not as objects.
    Fighter { fleet: FleetKey, index: usize },
    /// `0x1c..0x1f`, `FUN_004444c0`: a squadron of `class` on `system`'s
    /// planet. port: squadrons are counts per class.
    PlanetFighter {
        system: SystemKey,
        class: FighterKey,
    },
    /// `0x10..0x13`, `FUN_00445280`.
    Troop(TroopKey),
    /// `0x10..0x13`, `FUN_00445280`, filled from the viewing side's copy
    /// of the other side's regiment `id` (`FUN_004f2d10` reads the side's
    /// copy; `FUN_004f9a60` freezes it). `alliance` is the viewing side.
    ViewedTroop {
        alliance: bool,
        id: DatId,
    },
    /// `FUN_00445780`, `FUN_004444c0`, `FUN_00444e20` or `FUN_00446c50` by
    /// family, filled from the player's frozen copy of `system`'s object
    /// (`viewed_contents::ViewedUnit`).
    ViewedUnit {
        system: SystemKey,
        unit: crate::viewed_contents::ViewedUnit,
    },
    /// `0x20..0x27`, `FUN_00444e20`.
    DefenseFacility(DefenseFacilityKey),
    /// `0x28..0x2a`, `FUN_00446c50`.
    ManufacturingFacility(ManufacturingFacilityKey),
    /// `0x2c..0x2d`, `FUN_00446c50`.
    ProductionFacility(ProductionFacilityKey),
    /// `0x20..0x2d`, `FUN_00444e20` or `FUN_00446c50`: a facility page's or
    /// the Defenses window's unfinished unit for `system`, being built or
    /// travelling until `arrival`.
    PendingFacility {
        system: SystemKey,
        class: DatId,
        owner: u8,
        arrival: Option<u64>,
    },
    /// `0x10..0x13`, `FUN_00445280`: a regiment of `class` in training for
    /// `system`.
    PendingTroop { system: SystemKey, class: DatId },
    /// `0x14..0x1b`, `FUN_00446fd0`: an unfinished capital ship, by its
    /// product (`rebellion_core::products`).
    PendingShip { product: u64 },
    /// `0x1c..0x1f`, `FUN_004444c0`: a squadron of `class` being built for
    /// `system`'s planet.
    PendingFighter {
        system: SystemKey,
        class: FighterKey,
    },
    /// `0x1c..0x1f`, `FUN_004444c0`: a squadron of `class` travelling by
    /// Move to `to`, a planet or a fleet, until day `arrival` (`+0x50` bit
    /// 4).
    TravellingFighter {
        to: SquadronPlace,
        class: FighterKey,
        arrival: u64,
    },
    /// `0x50..0x7f`, `FUN_00445c60`: an active mission by its id.
    Mission(u64),
    /// The empty id, `FUN_004ece30` (Agent menu `0x114`): the Objectives,
    /// `FUN_0044b2d0` for side 1, else `FUN_0044bb70`.
    Objectives,
    /// The galaxy, `0xf1000004` (Agent menu `0x113`): the Galaxy Overview,
    /// `FUN_0044a7a0` and `FUN_0044aa20` over fifteen family ranges.
    GalaxyOverview,
    /// `0xa0..0xaf`, `FUN_0044acd0`: a system's manager for one area.
    Producer {
        system: SystemKey,
        area: ProductionArea,
    },
}

/// What the window draws for one object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    pub title: &'static str,
    /// The production manager's title is re-styled to font `0x11`
    /// (`FUN_00403e90(title, 0x11)`).
    pub manager_title: bool,
    /// The background row (`FUN_00443130`): 0 side 1, 1 side 2, 2 other.
    pub side: usize,
    pub picture: Option<(DllSource, u32)>,
    /// What `FUN_0042c3b0(.., 1, 1)` draws under the picture, at its
    /// top-left and within its size, in order: a regiment's backdrop, then
    /// the en route and damage marks.
    pub marks: Vec<(DllSource, u32)>,
    /// `FUN_0044a210`: the object's name.
    pub name: String,
    pub rows: Vec<StatusRow>,
    /// The Objectives' or the Galaxy Overview's rows, in place of `rows`.
    pub icon_rows: Vec<StatusIconRow>,
    pub icon_layout: IconRowLayout,
    /// The Encyclopedia button `0x66`, disabled (STRATEGY 11612) for the
    /// galaxy and the empty id (`FUN_00443020`).
    pub encyclopedia: bool,
}

/// One row of a picture list: an icon and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusIconRow {
    pub source: DllSource,
    pub icon: u32,
    pub text: String,
}

/// How a picture list lays its rows out; both are 35-pixel rows of a cell
/// the list's width (`FUN_006073d0(.., 0xd0, 0x23, ..)`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IconRowLayout {
    /// The Objectives: each icon centred in a 37 by 37 canvas, the text from
    /// x 40, centred on the row's height (`+0xe8 = 0x28`, `+0xe4 = 0x24`).
    #[default]
    Objectives,
    /// The Galaxy Overview: each mini at the cell's top-left, the text
    /// centred across the cell from its top (the list's own format, `+0xe4 =
    /// 9`, set at creation).
    Overview,
}

/// One victory condition of `FUN_0044b2d0` (Alliance) and `FUN_0044bb70`
/// (Empire): its icon, the bits of the conditions word that meet it,
/// whether only the standard game shows it, and per side the list key
/// (`FUN_0060ba00(key)`, inserted in key order) and the TEXTSTRA text while
/// the bits are clear and once one is set.
struct Objective {
    icon: u32,
    bits: u32,
    standard_only: bool,
    alliance: (u8, &'static str, &'static str),
    empire: (u8, &'static str, &'static str),
}

const OBJECTIVES: [Objective; 6] = [
    Objective {
        icon: 10_234,
        bits: 0x300,
        standard_only: false,
        alliance: (1, "Control Coruscant", "Coruscant Taken"),
        empire: (4, "Defend Coruscant", "Coruscant Lost"),
    },
    Objective {
        icon: 11_013,
        bits: 0x2000,
        standard_only: true,
        alliance: (2, "Capture Palpatine", "Emperor Captive"),
        empire: (5, "Defend the Emperor", "Emperor Captured"),
    },
    Objective {
        icon: 11_055,
        bits: 0x1000,
        standard_only: true,
        alliance: (3, "Capture Vader", "Vader Captive"),
        empire: (6, "Defend Vader", "Vader Captured"),
    },
    Objective {
        icon: 904,
        bits: 0x1,
        standard_only: false,
        alliance: (4, "Defend Headquarters", "Headquarters Lost"),
        empire: (1, "Destroy Rebel HQ", "Rebel HQ Destroyed"),
    },
    Objective {
        icon: 11_028,
        bits: 0x10,
        standard_only: true,
        alliance: (5, "Defend Luke", "Luke Captured"),
        empire: (2, "Capture Skywalker", "Skywalker Captive"),
    },
    Objective {
        icon: 11_032,
        bits: 0x20,
        standard_only: true,
        alliance: (6, "Defend Mon Mothma", "Mon Mothma Captured"),
        empire: (3, "Capture Mothma", "Mon Mothma Captive"),
    },
];

/// The Objectives rows (TEXTSTRA `0x8780..0x8797`): each side's own
/// conditions first, the leaders' only in the standard game.
#[must_use]
pub fn objective_rows(source: ObjectiveSource) -> Vec<StatusIconRow> {
    let mut rows: Vec<_> = OBJECTIVES
        .iter()
        .filter(|objective| source.standard || !objective.standard_only)
        .map(|objective| {
            let (key, open, met) = if source.alliance {
                objective.alliance
            } else {
                objective.empire
            };
            let text = if source.conditions_word & objective.bits == 0 {
                open
            } else {
                met
            };
            (key, StatusIconRow {
                source: DllSource::Strategy,
                icon: objective.icon,
                text: text.to_owned(),
            })
        })
        .collect();
    rows.sort_by_key(|&(key, _)| key);
    rows.into_iter().map(|(_, row)| row).collect()
}

/// The fighters' family (FIGHTSD's records, `0x1c`). port: the fighter
/// classes' `DatId`s carry no family byte.
const FIGHTER_FAMILY: u8 = 0x1c;

/// A capital ship class's family: CAPSHPSD gives `0x14` to every record but
/// the Death Star's (`0x88`), which is `0x18`. port: the classes' `DatId`s
/// carry no family byte, so one present is taken as it is.
fn ship_family(dat_id: DatId) -> u8 {
    match (dat_id.family(), dat_id.index()) {
        (0, 0x88) => 0x18,
        (0, _) => 0x14,
        (family, _) => family,
    }
}

/// The Galaxy Overview's rows for the viewing side (`FUN_00443130`, the
/// galaxy's branch). `FUN_0044a7a0` walks fifteen family ranges (troops
/// `0x10..0x13`, capital ships `0x14..0x1b`, fighters `0x1c..0x1f`,
/// headquarters `0x20..0x21`, defenses `0x22..0x25`, yards `0x28..0x2a`,
/// mines and refineries `0x2c..0x2d`, special forces `0x3c..0x3f`) and
/// groups the side's existing objects by class: their count, and their
/// summed maintenance (`FUN_004f2990`) for families `0x10..0x3f`.
/// `FUN_0044aa20` makes a row of each group: the class's GOKRES list mini
/// (`(resource & 0xfff) + 0x4000`), the count in three digits, two spaces
/// (`DAT_006a874c`) and the maintenance in four, keyed by the family so
/// the list sorts by it. hyp: groups of one family keep their class
/// resource order (`FUN_005f5440`'s keyed set); A0 2026-10-29 agrees for
/// the Empire's rows it shows.
///
/// port: an object exists in the world only once built, so the objects
/// under construction (`+0x50` bit 6 clear) are left out as the original
/// leaves them.
#[must_use]
pub fn overview_rows(world: &GameWorld, alliance: bool) -> Vec<StatusIconRow> {
    use std::collections::BTreeMap;
    let side = if alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    let class_cost = |class: DatId| {
        world
            .buildable_classes
            .get(&class)
            .map_or(0, |value| value.maintenance_cost)
    };
    // (family, mini) -> (count, maintenance).
    let mut groups: BTreeMap<(u8, u32), (u32, u32)> = BTreeMap::new();
    let mut add = |family: u8, mini: Option<u32>, count: u32, cost: u32| {
        if let Some(mini) = mini {
            let group = groups.entry((family, mini)).or_default();
            group.0 += count;
            group.1 += cost * count;
        }
    };
    for troop in world.troops.values().filter(|troop| troop.is_alliance == alliance) {
        let class = troop.class_dat_id;
        add(class.family(), troop_mini(class).map(|(mini, _)| mini), 1, class_cost(class));
    }
    for fleet in world.fleets.values().filter(|fleet| fleet.is_alliance == alliance) {
        for ship in fleet.capital_ships.iter().filter(|ship| ship.alive) {
            if let Some(class) = world.capital_ship_classes.get(ship.class) {
                add(
                    ship_family(class.dat_id),
                    capital_ship_mini_id(class.dat_id),
                    1,
                    class.maintenance_cost,
                );
            }
        }
        for entry in &fleet.fighters {
            if let Some(class) = world.fighter_classes.get(entry.class) {
                add(
                    FIGHTER_FAMILY,
                    fighter_mini_id(class.dat_id),
                    entry.count,
                    class.maintenance_cost,
                );
            }
        }
    }
    // The squadrons on the planets (ghidra/notes/planet-squadrons.md).
    for entry in world.systems.values().flat_map(|system| &system.fighters) {
        if let Some(class) = world
            .fighter_classes
            .get(entry.class)
            .filter(|class| class.is_alliance == alliance)
        {
            add(
                FIGHTER_FAMILY,
                fighter_mini_id(class.dat_id),
                entry.count,
                class.maintenance_cost,
            );
        }
    }
    for facility in world.defense_facilities.values().filter(|value| value.side == side) {
        let class = facility.class_dat_id;
        add(class.family(), defense_facility_mini(class).map(|(mini, _)| mini), 1, class_cost(class));
    }
    for facility in world
        .manufacturing_facilities
        .values()
        .filter(|value| value.side == side)
    {
        let class = facility.class_dat_id;
        add(
            class.family(),
            manufacturing_facility_mini(class).map(|(mini, _)| mini),
            1,
            class_cost(class),
        );
    }
    for facility in world
        .production_facilities
        .values()
        .filter(|value| value.side == side)
    {
        let class = facility.class_dat_id;
        add(
            class.family(),
            production_facility_mini(class).map(|(mini, _)| mini),
            1,
            class_cost(class),
        );
    }
    for unit in world
        .special_forces
        .values()
        .filter(|unit| unit.is_alliance == alliance)
    {
        let class = unit.class_dat_id;
        add(class.family(), special_force_mini(class).map(|(mini, _)| mini), 1, class_cost(class));
    }
    groups
        .into_iter()
        .map(|((_, mini), (count, maintenance))| StatusIconRow {
            source: DllSource::Gokres,
            icon: mini,
            text: format!("{count:03}  {maintenance:04}"),
        })
        .collect()
}

fn row(label: &str, value: impl Into<String>) -> StatusRow {
    StatusRow {
        label: label.to_owned(),
        value: value.into(),
    }
}

/// A heading row: the label over an empty value (`0x006b120c`).
fn heading(label: &str) -> StatusRow {
    row(label, "")
}

/// "current:maximum", joined by the executable's ":" (`0x006a872c`).
fn ratio(current: impl std::fmt::Display, maximum: impl std::fmt::Display) -> String {
    format!("{current}:{maximum}")
}

/// TEXTSTRA 34646 "Yes" or 34645 "No".
const fn yes_no(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        "No"
    }
}

/// 34658 "Not Assigned".
const NOT_ASSIGNED: &str = "Not Assigned";

/// A fleet's Admiral, General or Commander row (`FUN_004fd790(fleet,
/// kind)`): the name (`FUN_004f62d0`) of the first character aboard the
/// fleet's ships, of the fleet's side, whose command (`+0x96`) is `kind`
/// (`FUN_005006f0`), else [`NOT_ASSIGNED`].
///
/// port: the characters are taken in the world's order, as the port keeps
/// them on the fleet rather than its ships.
fn commander(
    world: &rebellion_core::world::GameWorld,
    key: FleetKey,
    kind: rebellion_core::character_command::CommandKind,
) -> String {
    rebellion_core::character_command::fleet_holder(world, key, kind)
        .and_then(|c| world.characters.get(c))
        .map_or_else(|| NOT_ASSIGNED.to_owned(), |c| c.name.clone())
}

/// The shared ETA row (`FUN_004fd2b0`): 34662 "ETA Destination:", 14356
/// "Day " and the arrival day.
fn eta(rows: &mut Vec<StatusRow>, arrival: Option<u64>) {
    if let Some(day) = arrival {
        rows.push(row("ETA Destination:", format!("Day {day}")));
    }
}

/// A side's background row.
const fn side_index(is_alliance: bool) -> usize {
    if is_alliance {
        0
    } else {
        1
    }
}

/// A side's background row; a neutral one is "other".
const fn faction_index(side: Faction) -> usize {
    match side {
        Faction::Alliance => 0,
        Faction::Empire => 1,
        Faction::Neutral => 2,
    }
}

/// The system's holder as a background row; neutral and contested are
/// "other".
fn holder_index(world: &GameWorld, system: SystemKey) -> usize {
    world
        .systems
        .get(system)
        .and_then(|value| value.control.faction())
        .map_or(2, faction_index)
}

fn system_name(world: &GameWorld, system: SystemKey) -> String {
    world
        .systems
        .get(system)
        .map(|value| value.name.clone())
        .unwrap_or_default()
}

/// A class record's catalog name (TEXTSTRA).
fn class_name(world: &GameWorld, class: DatId) -> String {
    world
        .buildable_classes
        .get(&class)
        .map(|value| value.name.clone())
        .unwrap_or_default()
}

/// `FUN_0044a620`: class `+0x4c` (`FUN_004f2990`).
fn maintenance(world: &GameWorld, class: DatId) -> String {
    world
        .buildable_classes
        .get(&class)
        .map_or(0, |value| value.maintenance_cost)
        .to_string()
}

/// A GOKRES picture (`FUN_0042c3b0(.., 1, 1)`): the class record's `+0x30`
/// resource `& 0xfff`, the list mini less `0x4000`. port: the regiment
/// mark (STRATEGY 11514) and the status marks are not drawn.
fn picture(mini: Option<u32>) -> Option<(DllSource, u32)> {
    mini.map(|mini| (DllSource::Gokres, mini - 0x4000))
}

/// `FUN_0042c3b0(.., 1, 1)`'s marks for a craft's portrait: GOKRES
/// portrait `+ 0x1000` while en route (`+0x50` bit `0x10`), `+ 0x2000`
/// while damaged (bit `0x200`). port: an object is always built, so the
/// unbuilt mark (bit 4 clear, STRATEGY 11571 / 11573) never shows.
fn craft_marks(
    portrait: Option<(DllSource, u32)>,
    en_route: bool,
    damaged: bool,
) -> Vec<(DllSource, u32)> {
    let Some((_, portrait)) = portrait else {
        return Vec::new();
    };
    [(en_route, 0x1000), (damaged, 0x2000)]
        .into_iter()
        .filter(|&(shown, _)| shown)
        .map(|(_, offset)| (DllSource::Gokres, portrait + offset))
        .collect()
}

/// A regiment's backdrop, STRATEGY 11514 (`0x2cfa`), and its en route
/// mark: STRATEGY 11516 (`0x2cfc`), but the GOKRES default for classes
/// `0x10000002` and `0x10000008` (`FUN_0042c3b0`).
fn regiment_marks(
    portrait: Option<(DllSource, u32)>,
    class: DatId,
    en_route: bool,
) -> Vec<(DllSource, u32)> {
    let mut marks = vec![(DllSource::Strategy, 11_514)];
    if en_route {
        if matches!(class.raw(), 0x1000_0002 | 0x1000_0008) {
            marks.extend(craft_marks(portrait, true, false));
        } else {
            marks.push((DllSource::Strategy, 11_516));
        }
    }
    marks
}

/// A special force's en route mark: STRATEGY 11520 (`0x2d00`), but the
/// GOKRES default for classes `0x3c000003` and `0x3c000005`
/// (`FUN_0042c3b0`). Its damage mark has no portrait (`0x2cee` wraps to 0).
fn special_force_marks(
    portrait: Option<(DllSource, u32)>,
    class: DatId,
    en_route: bool,
) -> Vec<(DllSource, u32)> {
    if !en_route {
        Vec::new()
    } else if matches!(class.raw(), 0x3c00_0003 | 0x3c00_0005) {
        craft_marks(portrait, true, false)
    } else {
        vec![(DllSource::Strategy, 11_520)]
    }
}

/// A fleet's arrival day: today plus the transit left (`move-order.md`).
fn fleet_arrival(sources: StatusSources<'_>, fleet: FleetKey) -> Option<u64> {
    sources.movement.get(fleet).map(|order| {
        sources.today + u64::from(order.transit_ticks.saturating_sub(order.ticks_elapsed))
    })
}

/// The fleet's own Status word: 34628 "Enroute" or 34632 "Awaiting
/// Orders". port: a fleet or ship exists only once complete, so "Under
/// Construction" never shows.
const fn moving_status(en_route: bool) -> &'static str {
    if en_route {
        "Enroute"
    } else {
        "Awaiting Orders"
    }
}

fn alive_ships(
    world: &GameWorld,
    fleet: FleetKey,
) -> impl Iterator<Item = (&ShipInstance, &CapitalShipClass)> + '_ {
    world
        .fleets
        .get(fleet)
        .into_iter()
        .flat_map(|value| value.capital_ships.iter())
        .filter(|ship| ship.alive)
        .filter_map(|ship| Some((ship, world.capital_ship_classes.get(ship.class)?)))
}

/// hyp: the port's nearest field to the damage bit (`+0x50` bit `0x200`):
/// a hull below its class's.
pub fn damaged(ship: &ShipInstance, class: &CapitalShipClass) -> bool {
    ship.subsystem_hits != 0 || u32::try_from(ship.hull_current).unwrap_or(0) < class.hull
}

/// What each of `fleet`'s ships, by index, holds of the units on their way
/// in: squadrons and regiments.
///
/// The ships' slots `+0x23c` (`CapShip_vt23c`, `FUN_00500fe0`) and `+0x240`
/// (`CapShip_vt240`, `FUN_00501040`) count children in mode 3
/// (`FUN_00513180`), and `FUN_00556390` makes a unit moving onto a fleet
/// its ship's child at once, so the travellers count where
/// [`crate::fleet_window::ship_load`] seats them (`FUN_00552300`).
fn travellers(sources: StatusSources<'_>, fleet: FleetKey) -> Vec<[usize; 2]> {
    let world = sources.world;
    let own = crate::fleet_window::ship_cargo(world, sources.transport, fleet);
    crate::fleet_window::ship_load(world, sources.transport, &[], fleet)
        .iter()
        .zip(&own)
        .map(|(load, own)| {
            [
                load.incoming_squadrons.len(),
                load.regiments.len().saturating_sub(own.regiments.len()),
            ]
        })
        .collect()
}

/// The characters and the special forces on their way into `fleet` that
/// its Personnel walks find (mode 3; the Fleet window's
/// [`crate::fleet_window::bound_members`]).
fn incoming_personnel(sources: StatusSources<'_>, fleet: FleetKey) -> [usize; 2] {
    let arrivals = crate::quadrant_icons::fleet_arrivals(sources.missions);
    crate::fleet_window::bound_members(sources.world, &arrivals, fleet).fold(
        [0, 0],
        |[characters, forces], member| match member {
            MissionMember::Character(_) => [characters + 1, forces],
            MissionMember::SpecialForce(_) => [characters, forces + 1],
        },
    )
}

/// The Fleet Status rows (`FUN_00449200`).
///
/// port: the fleet's squadrons, regiments and characters are kept on the
/// fleet, so Embarked and Personnel sum them there, with the units on
/// their way in ([`travellers`]). Personnel sums each ship's
/// `FUN_00536da0(ship, 3)` walk (`FUN_00513180`), families `0x30..0x40`
/// (`FUN_005131d0` keeps `lo <= family < hi`): the characters and the
/// special forces aboard and on their way in. The Capital Ship Status
/// counts the characters only ([`ship_rows`]).
/// hyp: Hyperdrive Rating reads Yes when an alive ship's class has a
/// hyperdrive (`FUN_004fd900` is traced only to its walk).
#[must_use]
pub fn fleet_rows(sources: StatusSources<'_>, key: FleetKey) -> Option<Vec<StatusRow>> {
    use rebellion_core::character_command::CommandKind;
    let world = sources.world;
    let fleet = world.fleets.get(key)?;
    let arrival = fleet_arrival(sources, key);
    let mut rows = vec![row("Status:", moving_status(arrival.is_some()))];
    eta(&mut rows, arrival);
    let ships: Vec<_> = alive_ships(world, key).collect();
    let fighter_capacity: u32 = ships.iter().map(|(_, class)| class.fighter_capacity).sum();
    let troop_capacity: u32 = ships.iter().map(|(_, class)| class.troop_capacity).sum();
    let [squadrons_in, regiments_in] = travellers(sources, key)
        .into_iter()
        .fold([0_usize; 2], |sum, each| [sum[0] + each[0], sum[1] + each[1]]);
    let [characters_in, forces_in] = incoming_personnel(sources, key);
    let incoming = [squadrons_in, regiments_in, characters_in + forces_in];
    let count = |own: usize, kind: usize| own.saturating_add(incoming[kind]).to_string();
    let squadrons = fleet
        .fighters
        .iter()
        .map(|entry| entry.count as usize)
        .sum();
    rows.extend([
        row(
            "Admiral:",
            commander(world, key, CommandKind::Admiral),
        ),
        row(
            "General:",
            commander(world, key, CommandKind::General),
        ),
        row(
            "Commander:",
            commander(world, key, CommandKind::Commander),
        ),
        row("Number Of Ships", fleet.ship_count().to_string()),
        heading("Capacity:"),
        row("Fighter Squadrons:", fighter_capacity.to_string()),
        row("Trooper Regiments:", troop_capacity.to_string()),
        heading("Embarked:"),
        row("Fighter Squadrons:", count(squadrons, 0)),
        row(
            "Trooper Regiments:",
            count(sources.transport.carried_count(key), 1),
        ),
        row(
            "Personnel:",
            count(fleet.characters.len() + fleet.special_forces.len(), 2),
        ),
        row(
            "Damaged Ships:",
            ships
                .iter()
                .filter(|(ship, class)| damaged(ship, class))
                .count()
                .to_string(),
        ),
        row(
            "Hyperdrive Rating:",
            yes_no(ships.iter().any(|(_, class)| class.hyperdrive != 0)),
        ),
    ]);
    Some(rows)
}

/// What one ship carries of its fleet's `total`: the ships fill in order up
/// to each one's capacity. port: the port keeps squadrons and regiments on
/// the fleet, not on a ship (`+0x23c`, `+0x240`).
fn aboard(capacities: &[u32], index: usize, total: u32) -> u32 {
    let before: u32 = capacities[..index].iter().sum();
    total
        .saturating_sub(before)
        .min(capacities.get(index).copied().unwrap_or(0))
}

/// The Capital Ship Status rows (`FUN_00446fd0`), for the fleet's ship at
/// `index`.
///
/// The rows after damage read `rebellion_core::ship_condition`: the ship's
/// hit counters (`+0x64`) and hull. Ship Damaged reads Yes while the hull is
/// below its class's or a counter is set (`+0x50` bit `0x200`,
/// `FUN_00501c00`). The hull is `ShipInstance::hull_current`. Personnel
/// is `FUN_004f25c0(ship, 3)`'s count, families `0x30..0x3c`: the
/// characters only, not the special forces (`0x3c..0x3f`). port:
/// Personnel puts the fleet's characters, and those on their way in,
/// aboard its first alive ship. Embarked counts the squadrons and
/// regiments on their way in seated on the ship too ([`travellers`]).
#[must_use]
pub fn ship_rows(
    sources: StatusSources<'_>,
    fleet_key: FleetKey,
    index: usize,
) -> Option<Vec<StatusRow>> {
    let world = sources.world;
    let fleet = world.fleets.get(fleet_key)?;
    let ship = fleet.capital_ships.get(index).filter(|ship| ship.alive)?;
    let class = world.capital_ship_classes.get(ship.class)?;
    let alive: Vec<usize> = (0..fleet.capital_ships.len())
        .filter(|&i| fleet.capital_ships[i].alive)
        .collect();
    let position = alive.iter().position(|&i| i == index)?;
    let capacities = |field: fn(&CapitalShipClass) -> u32| -> Vec<u32> {
        alive
            .iter()
            .map(|&i| {
                world
                    .capital_ship_classes
                    .get(fleet.capital_ships[i].class)
                    .map_or(0, field)
            })
            .collect()
    };
    let squadrons: u32 = fleet.fighters.iter().map(|entry| entry.count).sum();
    let regiments = u32::try_from(sources.transport.carried_count(fleet_key)).unwrap_or(u32::MAX);
    let [incoming_squadrons, incoming_regiments] = travellers(sources, fleet_key)
        .get(index)
        .copied()
        .unwrap_or_default();
    // The characters on their way in board the first usable ship
    // (FUN_00552300: unlimited room); port: with the fleet's own, the first
    // alive one.
    let personnel = if position == 0 {
        fleet.characters.len() + incoming_personnel(sources, fleet_key)[0]
    } else {
        0
    };
    let arrival = fleet_arrival(sources, fleet_key);
    // A ship on its way to join a fleet is that fleet's member already
    // (FUN_004feca0 reparents it at once; manual p. 122).
    let member_of = crate::fleet_window::join_target(world, sources.movement, fleet_key).unwrap_or(fleet_key);
    let mut rows = vec![
        row("Class:", class.name.clone()),
        row("Fleet:", fleet_label(world, member_of).unwrap_or_default()),
        row("Status:", moving_status(arrival.is_some())),
    ];
    eta(&mut rows, arrival);
    let embarked = [
        aboard(&capacities(|c| c.fighter_capacity), position, squadrons) as usize
            + incoming_squadrons,
        aboard(&capacities(|c| c.troop_capacity), position, regiments) as usize
            + incoming_regiments,
        personnel,
    ];
    class_rows(world, class, ship, embarked, &mut rows);
    Some(rows)
}

/// The Capital Ship Status rows after Status and its ETA (`FUN_00446fd0`):
/// maintenance, capacity, the `embarked` squadrons, regiments and
/// personnel, and `ship`'s ratings against its class.
fn class_rows(
    world: &GameWorld,
    class: &CapitalShipClass,
    ship: &rebellion_core::world::ShipInstance,
    embarked: [usize; 3],
    rows: &mut Vec<StatusRow>,
) {
    use rebellion_core::ship_condition as condition;
    use rebellion_core::world::Subsystem;
    let rating = |value, subsystem| condition::rating(world, value, class, ship, subsystem);
    rows.extend([
        row("Maintenance Cost:", class.maintenance_cost.to_string()),
        heading("Capacity:"),
        row("Fighter Squadrons:", class.fighter_capacity.to_string()),
        row("Trooper Regiments:", class.troop_capacity.to_string()),
        heading("Embarked:"),
        row(
            "Fighter Squadrons:",
            embarked[0].to_string(),
        ),
        row(
            "Trooper Regiments:",
            embarked[1].to_string(),
        ),
        row("Personnel:", embarked[2].to_string()),
        row("Ship Damaged:", yes_no(damaged(ship, class))),
        row(
            "Hyperdrive Rating:",
            ratio(condition::hyperdrive(class, ship), class.hyperdrive),
        ),
        row("Hull Value:", ratio(ship.hull_current, class.hull)),
        row("Damage Control Rating:", class.damage_control.to_string()),
        row(
            "Shield Recharge Rate:",
            ratio(
                condition::recharge(world, class.shield_recharge_rate, ship, Subsystem::Shield),
                class.shield_recharge_rate,
            ),
        ),
        row("Maximum Shield Strength:", class.shield_strength.to_string()),
        row(
            "Tractor Beam Power:",
            ratio(
                rating(class.tractor_beam_power, Subsystem::Tractor),
                class.tractor_beam_power,
            ),
        ),
        row(
            "Sub-Light Engine Rating:",
            ratio(
                rating(class.sub_light_engine, Subsystem::Speed),
                class.sub_light_engine,
            ),
        ),
        row("Maneuverability:", class.maneuverability.to_string()),
        row("Detection Rating:", class.detection.to_string()),
        row(
            "Weapon Recharge Rate:",
            ratio(
                condition::recharge(world, class.weapon_recharge_rate, ship, Subsystem::Weapon),
                class.weapon_recharge_rate,
            ),
        ),
        row(
            "Bombardment Modifier:",
            condition::bombardment(class, ship).to_string(),
        ),
    ]);
    // Fore 0, aft 1, then starboard (arc 3) before port (arc 2), read from
    // the pushes at 0x447f2c..0x448650.
    for (heading_label, [turbo, ion, laser]) in [
        (
            "Forward Weapons Arc Rating:",
            [class.turbolaser_fore, class.ion_cannon_fore, class.laser_cannon_fore],
        ),
        (
            "Aft Weapons Arc Rating:",
            [class.turbolaser_aft, class.ion_cannon_aft, class.laser_cannon_aft],
        ),
        (
            "Starboard Weapons Arc Rating:",
            [
                class.turbolaser_starboard,
                class.ion_cannon_starboard,
                class.laser_cannon_starboard,
            ],
        ),
        (
            "Port Weapons Arc Rating:",
            [class.turbolaser_port, class.ion_cannon_port, class.laser_cannon_port],
        ),
    ] {
        // FUN_00501270: each arc after the weapon hits and the hull.
        rows.extend([
            heading(heading_label),
            row("Turbo Laser:", rating(turbo, Subsystem::Weapon).to_string()),
            row(" Ion Cannon:", rating(ion, Subsystem::Weapon).to_string()),
            row("Laser Cannon:", rating(laser, Subsystem::Weapon).to_string()),
        ]);
    }
}

/// The Fighter Squadron Status rows (`FUN_004444c0`), for the fleet's
/// squadron entry at `index`. It has no "Status:" row; each current value
/// is the maximum less the squadron's losses (`+0x60`).
#[must_use]
pub fn fighter_rows(
    sources: StatusSources<'_>,
    fleet_key: FleetKey,
    index: usize,
) -> Option<Vec<StatusRow>> {
    let world = sources.world;
    let fleet = world.fleets.get(fleet_key)?;
    let entry = fleet.fighters.get(index)?;
    let class = world.fighter_classes.get(entry.class)?;
    // The ship it rides on (FighterEntry::carrier).
    let seat = entry
        .carrier
        .and_then(|id| fleet.capital_ships.iter().position(|ship| ship.id == id));
    let mut rows = vec![row(
        "Attached: ",
        aboard_label(world, fleet_key, seat).unwrap_or_default(),
    )];
    eta(&mut rows, fleet_arrival(sources, fleet_key));
    squadron_class_rows(&mut rows, class, entry.lost());
    Some(rows)
}

/// [`fighter_rows`] for a squadron of `class` on `system`'s planet: it is
/// attached to the system. port: the planet's squadrons are named by
/// class, so the rows show the first one of the class.
#[must_use]
pub fn planet_fighter_rows(
    sources: StatusSources<'_>,
    system: SystemKey,
    class: FighterKey,
) -> Option<Vec<StatusRow>> {
    let world = sources.world;
    let value = world.systems.get(system)?;
    let lost = value
        .fighters
        .iter()
        .find(|entry| entry.class == class && entry.count > 0)
        .map_or(0, rebellion_core::world::FighterEntry::lost);
    let class = world.fighter_classes.get(class)?;
    let mut rows = vec![row("Attached: ", value.name.clone())];
    squadron_class_rows(&mut rows, class, lost);
    Some(rows)
}

/// [`fighter_rows`] for a squadron of `class` on its way to `to`:
/// `FUN_004444c0` names its container, the destination planet's system or
/// fleet (`FUN_00556390` moves it there at once), then `FUN_004fd2b0`'s
/// ETA.
#[must_use]
pub fn travelling_fighter_rows(
    sources: StatusSources<'_>,
    to: SquadronPlace,
    class: FighterKey,
    arrival: u64,
) -> Option<Vec<StatusRow>> {
    let world = sources.world;
    let attached = match to {
        SquadronPlace::Planet(system) => world.systems.get(system)?.name.clone(),
        // The ship the port seats it on among the squadrons coming in
        // (ship_load: FUN_00552300's most room left).
        SquadronPlace::Fleet(fleet) => {
            let ordinal = sources
                .transport
                .squadrons
                .transits()
                .iter()
                .filter(|transit| transit.to == to)
                .take_while(|transit| !(transit.class == class && transit.arrival_tick == arrival))
                .map(|transit| transit.count as usize)
                .sum::<usize>();
            let arrivals = crate::quadrant_icons::fleet_arrivals(sources.missions);
            let seat = ship_load(world, sources.transport, &arrivals, fleet)
                .iter()
                .position(|cargo| cargo.incoming_squadrons.contains(&ordinal));
            aboard_label(world, fleet, seat)?
        }
    };
    let class = world.fighter_classes.get(class)?;
    let mut rows = vec![row("Attached: ", attached)];
    eta(&mut rows, Some(arrival));
    squadron_class_rows(&mut rows, class, 0);
    Some(rows)
}

/// `FUN_004444c0`'s class rows for a squadron that has lost `lost` craft
/// (`+0x254`, `FUN_00503100`): each is the class value times the size
/// (`+0x1e0`, `+0x1f8`, `+0x23c`) less the value times the losses
/// (`+0x204`, `+0x21c`, `+0x24c`), over the first; the bombardment value
/// loses `value x losses / size` (`FUN_00503110`).
fn squadron_class_rows(
    rows: &mut Vec<StatusRow>,
    class: &rebellion_core::world::FighterClass,
    lost: u32,
) {
    let size = class.squadron_size;
    let lost = lost.min(size);
    let full = |per_craft: u32| ratio(per_craft * size - per_craft * lost, per_craft * size);
    let bombardment = class.bombardment_defense;
    let bombardment_lost = if size == 0 { 0 } else { bombardment * lost / size };
    rows.extend([
        row("Maintenance Cost:", class.maintenance_cost.to_string()),
        row("Squadron Size:", ratio(size - lost, size)),
        row("Hyperdrive Rating:", class.hyperdrive.to_string()),
        row("Maximum Shield Strength:", full(class.shield_strength)),
        row("Sub-Light Engine Rating:", class.sub_light_engine.to_string()),
        row("Maneuverability:", class.maneuverability.to_string()),
        row("Detection Rating:", class.detection.to_string()),
        row(
            "Bombardment Value:",
            ratio(bombardment - bombardment_lost, bombardment),
        ),
        heading("Weapons Rating:"),
        row(" Laser Rating:", full(class.laser_cannon_attack_strength)),
        row(" Ion Cannon:", full(class.ion_cannon_attack_strength)),
        row(" Torpedoes:", full(class.torpedoes)),
    ]);
}

/// Where a regiment is, and its arrival day while it travels.
fn regiment_place(sources: StatusSources<'_>, key: TroopKey) -> (String, Option<u64>) {
    let world = sources.world;
    if let Some(transit) = sources
        .transport
        .transits()
        .iter()
        .find(|transit| transit.troop == key)
    {
        // An object in hyperspace is attached to its destination
        // (FUN_00556390).
        // Into a fleet, the ship the port seats it on (ship_load).
        let place = match transit.leg {
            RegimentLeg::Surface(system) => system_name(world, system),
            RegimentLeg::Fleet(fleet) => {
                let arrivals = crate::quadrant_icons::fleet_arrivals(sources.missions);
                let seat = ship_load(world, sources.transport, &arrivals, fleet)
                    .iter()
                    .position(|cargo| cargo.regiments.contains(&key));
                aboard_label(world, fleet, seat).unwrap_or_default()
            }
        };
        return (place, Some(transit.arrival_tick));
    }
    if let Some(fleet) = sources
        .transport
        .fleet_keys()
        .into_iter()
        .find(|&fleet| sources.transport.cargo(fleet).contains(&key))
    {
        // The ship that holds it (ship_cargo).
        let seat = ship_cargo(world, sources.transport, fleet)
            .iter()
            .position(|cargo| cargo.regiments.contains(&key));
        return (aboard_label(world, fleet, seat).unwrap_or_default(), None);
    }
    let system = world
        .systems
        .iter()
        .find(|(_, value)| value.ground_units.contains(&key))
        .map(|(_, value)| value.name.clone())
        .unwrap_or_default();
    (system, None)
}

/// The Trooper Regiment Status rows (`FUN_00445280`).
#[must_use]
pub fn troop_rows(sources: StatusSources<'_>, key: TroopKey) -> Option<Vec<StatusRow>> {
    let troop = sources.world.troops.get(key)?;
    let (attached, arrival) = regiment_place(sources, key);
    Some(regiment_rows(
        sources.world,
        troop.class_dat_id,
        attached,
        moving_status(arrival.is_some()),
        arrival,
    ))
}

/// The Trooper Regiment Status rows (`FUN_00445280`) of a side's copy of a
/// regiment: its place, status and ETA as the copy holds them.
#[must_use]
pub fn viewed_troop_rows(
    world: &GameWorld,
    copy: &rebellion_core::object_views::RegimentView,
) -> Vec<StatusRow> {
    regiment_rows(
        world,
        copy.class,
        crate::viewed_contents::view_attached(world, copy),
        moving_status(copy.arrival.is_some()),
        copy.arrival,
    )
}

/// A Status window with no marks, list of pictures or locked Encyclopedia.
fn plain_view(
    title: &'static str,
    side: usize,
    picture: Option<(DllSource, u32)>,
    name: String,
    rows: Vec<StatusRow>,
) -> StatusView {
    StatusView {
        title,
        manager_title: false,
        side,
        picture,
        marks: Vec::new(),
        name,
        rows,
        icon_rows: Vec::new(),
        icon_layout: IconRowLayout::Objectives,
        encyclopedia: true,
    }
}

/// The Status window of an object of the player's frozen copy of `system`
/// (`FUN_004f2d10` reads the side's copy; `FUN_004f9a60`): a special
/// force's (`FUN_00445780`), squadron's (`FUN_004444c0`) or facility's
/// (`FUN_00444e20`, `FUN_00446c50`) rows from its class, with its ETA
/// while it was seen en route. port: a copied special force's ratings are
/// its class's base skills.
fn viewed_unit_view(
    world: &GameWorld,
    system: SystemKey,
    unit: crate::viewed_contents::ViewedUnit,
) -> Option<StatusView> {
    use crate::viewed_contents::ViewedUnit;
    match unit {
        ViewedUnit::SpecialForce {
            class,
            is_alliance,
            arrival,
        } => {
            let base = world.special_force_classes.get(&class);
            let skill = |index: usize| base.map_or(0, |value| value.skills[index].base);
            let mut rows = vec![
                row("Attached: ", system_name(world, system)),
                row("Status:", moving_status(arrival.is_some())),
            ];
            eta(&mut rows, arrival);
            rows.extend([
                row("Maintenance Cost:", maintenance(world, class)),
                row("Diplomacy Rating:", skill(0).to_string()),
                row("Espionage Rating:", skill(1).to_string()),
                row("Combat Rating:", skill(5).to_string()),
                row("Leadership Rating:", skill(6).to_string()),
            ]);
            let portrait = picture(special_force_mini(class).map(|(mini, _)| mini));
            Some(StatusView {
                marks: special_force_marks(portrait, class, arrival.is_some()),
                ..plain_view(
                    "Spec Forces Status",
                    side_index(is_alliance),
                    portrait,
                    class_name(world, class),
                    rows,
                )
            })
        }
        ViewedUnit::Squadron {
            class,
            is_alliance,
            arrival,
        } => {
            let fighter = world.fighter_classes.values().find(|value| value.dat_id == class)?;
            let mut rows = vec![row("Attached: ", system_name(world, system))];
            eta(&mut rows, arrival);
            squadron_class_rows(&mut rows, fighter, 0);
            let portrait = picture(fighter_mini_id(class));
            Some(StatusView {
                marks: craft_marks(portrait, arrival.is_some(), false),
                ..plain_view(
                    "Fighter Squadron Status",
                    side_index(is_alliance),
                    portrait,
                    fighter.name.clone(),
                    rows,
                )
            })
        }
        ViewedUnit::Facility { class, owner } => {
            let side = if owner == 1 {
                rebellion_core::dat::Faction::Alliance
            } else {
                rebellion_core::dat::Faction::Empire
            };
            let (title, rows, mini) = if (0x20..=0x27).contains(&class.family()) {
                (
                    "Defense Facility Status",
                    defense_rows(world, system, class, None),
                    defense_facility_mini(class).map(|(mini, _)| mini),
                )
            } else {
                (
                    "Manufacturing Status",
                    manufacturing_rows(world, system, class, None),
                    manufacturing_facility_mini(class)
                        .or_else(|| production_facility_mini(class))
                        .map(|(mini, _)| mini),
                )
            };
            Some(plain_view(title, faction_index(side), picture(mini), class_name(world, class), rows))
        }
    }
}

/// `FUN_00445280`'s rows for a regiment of `class`: its place, its Status
/// word (`+0x50` bit 4 34628 "Enroute", else bit 2 34632 "Awaiting
/// Orders", else 34673 "Training") and, en route, its ETA.
fn regiment_rows(
    world: &GameWorld,
    class_id: DatId,
    attached: String,
    status: &str,
    arrival: Option<u64>,
) -> Vec<StatusRow> {
    let class = world.troop_classes.get(&class_id);
    let stats = world
        .buildable_classes
        .get(&class_id)
        .map(|value| value.stats)
        .unwrap_or_default();
    let mut rows = vec![row("Attached: ", attached), row("Status:", status)];
    eta(&mut rows, arrival);
    rows.extend([
        row("Maintenance Cost:", maintenance(world, class_id)),
        row(
            "Attack Strength:",
            class.map_or(0, |value| value.attack_strength).to_string(),
        ),
        row(
            "Defense Strength:",
            class.map_or(0, |value| value.defense_strength).to_string(),
        ),
        row("Bombardment Value:", stats.bombardment.to_string()),
        row(
            "Detection Value:",
            class.map_or(0, |value| value.detection).to_string(),
        ),
    ]);
    rows
}

/// The Spec Forces Status rows (`FUN_00445780`). The ratings are slots
/// `+0x1dc`, `+0x1e0`, `+0x1f0` and `+0x1f4`: skills 0, 1, 5 and 6.
///
/// port: a special force exists only once built, so "Under Construction"
/// never shows; it is en route while its mission transit lasts, as a
/// character is.
#[must_use]
pub fn special_force_rows(
    sources: StatusSources<'_>,
    key: SpecialForceKey,
) -> Option<Vec<StatusRow>> {
    let world = sources.world;
    let unit = world.special_forces.get(key)?;
    let transit = sources
        .missions
        .en_route()
        .iter()
        .find(|transit| transit.member == MissionMember::SpecialForce(key));
    // Attached names the unit's container: the ship it rides on aboard a
    // fleet (the port seats it on the first, ship_cargo), else its system;
    // in hyperspace the Move's target container (FUN_00556390).
    let attached = transit.map_or_else(
        || {
            world
                .fleets
                .iter()
                .find(|(_, value)| value.special_forces.contains(&key))
                .and_then(|(fleet, _)| aboard_label(world, fleet, first_seat(world, fleet)))
                .or_else(|| {
                    world
                        .systems
                        .iter()
                        .find(|(_, value)| value.special_forces.contains(&key))
                        .map(|(_, value)| value.name.clone())
                })
                .unwrap_or_default()
        },
        |transit| {
            crate::status_window::transit_container(
                world,
                &crate::quadrant_icons::fleet_arrivals(sources.missions),
                transit,
            )
            .unwrap_or_default()
        },
    );
    let status = if transit.is_some() {
        "Enroute"
    } else if unit.on_mission && !unit.on_hidden_mission {
        "On Mission"
    } else {
        "Awaiting Orders"
    };
    let mut rows = vec![row("Attached: ", attached), row("Status:", status)];
    eta(&mut rows, transit.map(|transit| transit.arrival));
    rows.extend([
        row("Maintenance Cost:", maintenance(world, unit.class_dat_id)),
        row("Diplomacy Rating:", unit.skills[0].to_string()),
        row("Espionage Rating:", unit.skills[1].to_string()),
        row("Combat Rating:", unit.skills[5].to_string()),
        row("Leadership Rating:", unit.skills[6].to_string()),
    ]);
    Some(rows)
}

/// The system holding a facility key in one of its lists.
fn facility_system<K: PartialEq>(
    world: &GameWorld,
    key: K,
    list: impl Fn(&rebellion_core::world::System) -> &[K],
) -> Option<SystemKey> {
    world
        .systems
        .iter()
        .find(|(_, value)| list(value).contains(&key))
        .map(|(system, _)| system)
}

/// 34659 "Active". port: a facility exists as a world object only once
/// complete, and sits in a queue or a delivery before that, so "Enroute",
/// "Under Construction" and the ETA row never show (`FUN_0044a470`).
const ACTIVE: &str = "Active";

/// The Defense Facility Status rows (`FUN_00444e20`). Weapons Rating and
/// Shield Strength show only for families `0x22..0x27`.
#[must_use]
pub fn defense_facility_rows(
    sources: StatusSources<'_>,
    key: DefenseFacilityKey,
) -> Option<Vec<StatusRow>> {
    let world = sources.world;
    let facility = world.defense_facilities.get(key)?;
    let system = facility_system(world, key, |value| &value.defense_facilities)?;
    Some(defense_rows(world, system, facility.class_dat_id, None))
}

/// `FUN_00444e20`'s rows for a facility of class `class` at `system`: built,
/// or unfinished (`unfinished`, see [`unfinished_status`]). An object en
/// route leads with its ETA (`FUN_004fd2b0`).
fn defense_rows(
    world: &GameWorld,
    system: SystemKey,
    class: DatId,
    unfinished: Option<Option<u64>>,
) -> Vec<StatusRow> {
    let stats = world
        .buildable_classes
        .get(&class)
        .map(|value| value.stats)
        .unwrap_or_default();
    let mut rows = Vec::new();
    eta(&mut rows, unfinished.flatten());
    rows.extend([
        row("Location:", system_name(world, system)),
        row("Status:", unfinished.map_or(ACTIVE, unfinished_status)),
        row("Maintenance Cost:", maintenance(world, class)),
    ]);
    if (0x22..=0x27).contains(&class.family()) {
        rows.extend([
            row("Weapons Rating:", stats.attack_strength.to_string()),
            row("Shield Strength", stats.shield_strength.to_string()),
        ]);
    }
    rows.push(row(
        "Bombardment Defense Strength:",
        stats.bombardment.to_string(),
    ));
    rows
}

/// A pending unit's facility class.
fn pending_class_id(kind: BuildableKind) -> Option<DatId> {
    match kind {
        BuildableKind::DefenseFacility(build)
        | BuildableKind::ManufacturingFacility(build)
        | BuildableKind::ProductionFacility(build) => Some(build.class),
        _ => None,
    }
}

/// The mark `FUN_0042c3b0(.., 1, 1)` lays under an unfinished object's
/// portrait (`+0x50` bit 2 clear): STRATEGY `0x2d32 + 1` (11571) for side 1,
/// else `0x2d34 + 1` (11573).
const fn unfinished_mark(side_one: bool) -> (DllSource, u32) {
    (DllSource::Strategy, if side_one { 11_571 } else { 11_573 })
}

/// `FUN_0044a470`'s Status word for an unfinished object: 34628 "Enroute"
/// while it travels (`+0x50` bit 4), else 34647 "Under Construction" (bit 2
/// clear; A0 pending-facility/13).
fn unfinished_status(arrival: Option<u64>) -> &'static str {
    if arrival.is_some() {
        "Enroute"
    } else {
        "Under Construction"
    }
}

/// The Manufacturing Status rows (`FUN_00446c50`), for a yard, mine or
/// refinery: built, or unfinished (`unfinished`, see [`unfinished_status`]).
/// An object en route takes its ETA (`FUN_004fd2b0`) after Location.
fn manufacturing_rows(
    world: &GameWorld,
    system: SystemKey,
    class: DatId,
    unfinished: Option<Option<u64>>,
) -> Vec<StatusRow> {
    let value = world.buildable_classes.get(&class);
    let mut rows = vec![row("Location:", system_name(world, system))];
    eta(&mut rows, unfinished.flatten());
    rows.extend([
        row("Status:", unfinished.map_or(ACTIVE, unfinished_status)),
        row("Maintenance Cost:", maintenance(world, class)),
        row(
            "Standard Processing Rate:",
            value.map_or(0, |value| value.processing_rate).to_string(),
        ),
        row(
            "Bombardment Value:",
            value.map_or(0, |value| value.stats.bombardment).to_string(),
        ),
    ]);
    rows
}

/// The name of a mission's target object: its class, or a Death Star's
/// fleet.
#[must_use]
pub fn object_name(world: &GameWorld, target: MissionTarget) -> Option<String> {
    let named = |name: Option<String>| name.filter(|name| !name.is_empty());
    named(match target {
        MissionTarget::DefenseFacility(key) => world
            .defense_facilities
            .get(key)
            .map(|value| class_name(world, value.class_dat_id)),
        MissionTarget::ManufacturingFacility(key) => world
            .manufacturing_facilities
            .get(key)
            .map(|value| class_name(world, value.class_dat_id)),
        MissionTarget::ProductionFacility(key) => world
            .production_facilities
            .get(key)
            .map(|value| class_name(world, value.class_dat_id)),
        MissionTarget::Troop(key) => world
            .troops
            .get(key)
            .map(|value| class_name(world, value.class_dat_id)),
        MissionTarget::SpecialForce(key) => world
            .special_forces
            .get(key)
            .map(|value| class_name(world, value.class_dat_id)),
        MissionTarget::DeathStar(fleet) => fleet_label(world, fleet),
        MissionTarget::CapitalShip { class, .. } => world
            .capital_ship_classes
            .get(class)
            .map(|value| value.name.clone()),
        MissionTarget::Fighter { class, .. } => world
            .fighter_classes
            .get(class)
            .map(|value| value.name.clone()),
    })
}

/// A mission's target by name (`+0x70`), or 34087 "Target Unknown".
fn target_name(world: &GameWorld, mission: &rebellion_core::missions::ActiveMission) -> String {
    let named = |name: Option<String>| name.filter(|name| !name.is_empty());
    let object = mission
        .target_object
        .and_then(|target| object_name(world, target));
    object
        .or_else(|| {
            named(
                mission
                    .target_character
                    .and_then(|key| world.characters.get(key))
                    .map(|value| value.name.clone()),
            )
        })
        .or_else(|| named(Some(system_name(world, mission.target_system))))
        .unwrap_or_else(|| "Target Unknown".to_owned())
}

/// The Mission Status rows (`FUN_00445c60`): the team counts its decoys.
///
/// port: the team is the mission's `team` and `decoys`; whether a captured
/// member still counts is untraced, so the captured are left out.
#[must_use]
pub fn mission_rows(sources: StatusSources<'_>, id: u64) -> Option<Vec<StatusRow>> {
    let mission = sources
        .missions
        .missions()
        .iter()
        .find(|mission| mission.id == id)?;
    let first = mission.team.first().or(mission.decoys.first()).copied();
    let arrival = first.and_then(|member| {
        sources
            .missions
            .en_route()
            .iter()
            .find(|transit| transit.member == member && transit.mission_id == id)
            .map(|transit| transit.arrival)
    });
    let mut rows = vec![row("Target:", target_name(sources.world, mission))];
    eta(&mut rows, arrival);
    rows.extend([
        row(
            "Team Size:",
            (mission.team.len() + mission.decoys.len()).to_string(),
        ),
        row("Decoys:", mission.decoys.len().to_string()),
    ]);
    Some(rows)
}

/// An area's manager name: `FUN_004f62d0` gives the object's own name
/// (`+0x34`), else its class's, the class names TEXTSTRA 11524
/// "Construction", 11525 "Shipyard" and 11526 "Training" (families
/// `0xa0..0xa1`, `0xa2..0xa3`, `0xa4..`, in the order of the pictures
/// 263, 262, 264). A0 pending-facility/34: Pzob's Ship Construction Status
/// names it "Shipyard".
const fn manager_name(area: ProductionArea) -> &'static str {
    match area {
        ProductionArea::ConstructionYard => "Construction",
        ProductionArea::Shipyard => "Shipyard",
        ProductionArea::TrainingFacility => "Training",
    }
}

/// An area's manager title (6195, 6185, 6193), the yard family it counts
/// and its busy word (34672 "Building", 34673 "Training").
const fn manager(area: ProductionArea) -> (&'static str, u8, &'static str) {
    match area {
        ProductionArea::ConstructionYard => ("Facilities Under Construction", 0x2a, "Building"),
        ProductionArea::Shipyard => ("Ship Construction", 0x28, "Building"),
        ProductionArea::TrainingFacility => ("Troops in Training", 0x29, "Training"),
    }
}

/// The Production Manager Status rows (`FUN_0044acd0`).
///
/// port: the area's queue stands in for the manager, and a system's yards
/// serve its holder. hyp: the completion day is the queue's last unit's.
#[must_use]
pub fn producer_rows(
    sources: StatusSources<'_>,
    system: SystemKey,
    area: ProductionArea,
) -> Option<Vec<StatusRow>> {
    let world = sources.world;
    let value = world.systems.get(system)?;
    let (_, family, busy) = manager(area);
    let facilities: Vec<_> = value
        .manufacturing_facilities
        .iter()
        .filter_map(|key| world.manufacturing_facilities.get(*key))
        .filter(|facility| facility.class_dat_id.family() == family)
        .collect();
    let yards = facilities.len();
    let queue = sources
        .manufacturing
        .queue(system, area)
        .filter(|queue| !queue.is_empty());
    let status = match (yards, queue) {
        (0, _) => "No Facilities",
        (_, None) => "Idle",
        (_, Some(_)) => busy,
    };
    let mut rows = vec![
        row("Location:", value.name.clone()),
        row("Status:", status),
    ];
    if let (true, Some(queue)) = (yards > 0, queue) {
        rows.push(row("Items to Build:", queue.len().to_string()));
        let periods = facilities.first().map_or_else(Vec::new, |facility| {
            rebellion_core::build_selection::yard_periods(
                world,
                system,
                area,
                facility.side == rebellion_core::dat::Faction::Alliance,
            )
        });
        if let Some(day) = queue.completion_days(sources.today, &periods).last() {
            rows.push(row("Estimated Day of Completion:", day.to_string()));
        }
    }
    Some(rows)
}

/// The window's contents for `object`, or `None` once it is gone.
#[must_use]
pub fn status_view(sources: StatusSources<'_>, object: StatusObject) -> Option<StatusView> {
    let world = sources.world;
    let view = |title, side, picture, name, rows| StatusView {
        title,
        manager_title: false,
        side,
        picture,
        marks: Vec::new(),
        name,
        rows,
        icon_rows: Vec::new(),
        icon_layout: IconRowLayout::Objectives,
        encyclopedia: true,
    };
    Some(match object {
        // FUN_00443130: the background as for the galaxy (11554 for a side-1
        // viewer, 11558 otherwise), STRATEGY 11590, the name TEXTSTRA 12564.
        StatusObject::Objectives => StatusView {
            icon_rows: objective_rows(sources.objectives),
            encyclopedia: false,
            ..view(
                "",
                usize::from(!sources.objectives.alliance),
                Some((DllSource::Strategy, 11_590)),
                "Objectives".to_owned(),
                Vec::new(),
            )
        },
        // The same frame for the galaxy, the name TEXTSTRA 12563.
        StatusObject::GalaxyOverview => StatusView {
            icon_rows: overview_rows(world, sources.objectives.alliance),
            icon_layout: IconRowLayout::Overview,
            encyclopedia: false,
            ..view(
                "",
                usize::from(!sources.objectives.alliance),
                Some((DllSource::Strategy, 11_590)),
                "Galaxy Overview".to_owned(),
                Vec::new(),
            )
        },
        StatusObject::Character(key) => {
            let character = world.characters.get(key)?;
            let side = if character.is_alliance {
                0
            } else if character.is_empire {
                1
            } else {
                2
            };
            view(
                "Character Status",
                side,
                character_picture(character).map(|mini| (DllSource::Gokres, mini - 0x4000)),
                character.name.clone(),
                character_rows(
                    world,
                    sources.missions.en_route(),
                    &crate::quadrant_icons::fleet_arrivals(sources.missions),
                    key,
                )?,
            )
        }
        StatusObject::SpecialForce(key) => {
            let unit = world.special_forces.get(key)?;
            let portrait = picture(special_force_mini(unit.class_dat_id).map(|(mini, _)| mini));
            let en_route = sources
                .missions
                .en_route()
                .iter()
                .any(|transit| transit.member == MissionMember::SpecialForce(key));
            let marks = special_force_marks(portrait, unit.class_dat_id, en_route);
            StatusView {
                marks,
                ..view(
                    "Spec Forces Status",
                    side_index(unit.is_alliance),
                    portrait,
                    class_name(world, unit.class_dat_id),
                    special_force_rows(sources, key)?,
                )
            }
        }
        StatusObject::Fleet(key) => {
            let fleet = world.fleets.get(key)?;
            // FUN_0042c3b0: STRATEGY 0x28b9 for side 1, 0x28eb otherwise.
            let picture = if fleet.is_alliance { 10_425 } else { 10_475 };
            // 10426 / 10476 en route (+0x50 bit 0x10); 10427 / 10477
            // damaged (bit 0x200, which FUN_004feab0 sets while a ship of
            // the fleet is damaged).
            let en_route = fleet_arrival(sources, key).is_some();
            let damaged = crate::fleet_window::fleet_damaged(world, key);
            let marks = [(en_route, picture + 1), (damaged, picture + 2)]
                .into_iter()
                .filter(|&(shown, _)| shown)
                .map(|(_, id)| (DllSource::Strategy, id))
                .collect();
            StatusView {
                marks,
                ..view(
                    "Fleet Status",
                    side_index(fleet.is_alliance),
                    Some((DllSource::Strategy, picture)),
                    fleet_label(world, key).unwrap_or_default(),
                    fleet_rows(sources, key)?,
                )
            }
        }
        StatusObject::Ship { fleet, index } => {
            let value = world.fleets.get(fleet)?;
            let ship = value.capital_ships.get(index)?;
            let class = world.capital_ship_classes.get(ship.class)?;
            let portrait = picture(capital_ship_mini_id(class.dat_id));
            let marks = craft_marks(
                portrait,
                fleet_arrival(sources, fleet).is_some(),
                damaged(ship, class),
            );
            StatusView {
                marks,
                ..view(
                    "Capital Ship Status",
                    side_index(value.is_alliance),
                    portrait,
                    ship.name.clone().unwrap_or_else(|| class.name.clone()),
                    ship_rows(sources, fleet, index)?,
                )
            }
        }
        StatusObject::Fighter { fleet, index } => {
            let value = world.fleets.get(fleet)?;
            let entry = value.fighters.get(index)?;
            let class = world.fighter_classes.get(entry.class)?;
            let portrait = picture(fighter_mini_id(class.dat_id));
            // A squadron with losses is damaged (FUN_00503340).
            let marks = craft_marks(
                portrait,
                fleet_arrival(sources, fleet).is_some(),
                entry.damage.is_some(),
            );
            StatusView {
                marks,
                ..view(
                    "Fighter Squadron Status",
                    side_index(value.is_alliance),
                    portrait,
                    class.name.clone(),
                    fighter_rows(sources, fleet, index)?,
                )
            }
        }
        StatusObject::PlanetFighter { system, class } => {
            let fighter = world.fighter_classes.get(class)?;
            view(
                "Fighter Squadron Status",
                side_index(fighter.is_alliance),
                picture(fighter_mini_id(fighter.dat_id)),
                fighter.name.clone(),
                planet_fighter_rows(sources, system, class)?,
            )
        }
        StatusObject::Troop(key) => {
            let troop = world.troops.get(key)?;
            let portrait = picture(troop_mini(troop.class_dat_id).map(|(mini, _)| mini));
            let en_route = regiment_place(sources, key).1.is_some();
            let marks = regiment_marks(portrait, troop.class_dat_id, en_route);
            StatusView {
                marks,
                ..view(
                    "Trooper Regiment Status",
                    side_index(troop.is_alliance),
                    portrait,
                    class_name(world, troop.class_dat_id),
                    troop_rows(sources, key)?,
                )
            }
        }
        StatusObject::ViewedTroop { alliance, id } => {
            let viewer = if alliance {
                rebellion_core::dat::Faction::Alliance
            } else {
                rebellion_core::dat::Faction::Empire
            };
            let copy = crate::viewed_contents::regiment_view(world, viewer, id)?;
            let portrait = picture(troop_mini(copy.class).map(|(mini, _)| mini));
            let marks = regiment_marks(portrait, copy.class, copy.arrival.is_some());
            StatusView {
                marks,
                ..view(
                    "Trooper Regiment Status",
                    side_index(copy.is_alliance),
                    portrait,
                    class_name(world, copy.class),
                    viewed_troop_rows(world, copy),
                )
            }
        }
        StatusObject::ViewedUnit { system, unit } => viewed_unit_view(world, system, unit)?,
        StatusObject::DefenseFacility(key) => {
            let facility = world.defense_facilities.get(key)?;
            view(
                "Defense Facility Status",
                faction_index(facility.side),
                picture(defense_facility_mini(facility.class_dat_id).map(|(mini, _)| mini)),
                class_name(world, facility.class_dat_id),
                defense_facility_rows(sources, key)?,
            )
        }
        StatusObject::ManufacturingFacility(key) => {
            let facility = world.manufacturing_facilities.get(key)?;
            let system = facility_system(world, key, |value| &value.manufacturing_facilities)?;
            let class = facility.class_dat_id;
            // FUN_00443130 fills families 0x20..0x27 with FUN_00444e20, so
            // the headquarters (ALLFACSD, 0x20) shows Defense Facility
            // Status, though the port keeps it with the yards.
            // The headquarters' trip (headquarters_move).
            let arrival = sources
                .transport
                .headquarters
                .filter(|transit| class.family() == 0x20 && transit.destination == system)
                .map(|transit| transit.arrival_tick);
            let (title, rows) = if (0x20..=0x27).contains(&class.family()) {
                (
                    "Defense Facility Status",
                    defense_rows(world, system, class, arrival.map(Some)),
                )
            } else {
                (
                    "Manufacturing Status",
                    manufacturing_rows(world, system, class, None),
                )
            };
            StatusView {
                // FUN_0042c3b0: STRATEGY 11506 under a facility's portrait
                // (0x20..0x2f outside 0x22..0x27) while it is en route.
                marks: arrival
                    .map(|_| (DllSource::Strategy, 11_506))
                    .into_iter()
                    .collect(),
                ..view(
                    title,
                    faction_index(facility.side),
                    picture(manufacturing_facility_mini(class).map(|(mini, _)| mini)),
                    class_name(world, class),
                    rows,
                )
            }
        }
        StatusObject::ProductionFacility(key) => {
            let facility = world.production_facilities.get(key)?;
            let system = facility_system(world, key, |value| &value.production_facilities)?;
            view(
                "Manufacturing Status",
                faction_index(facility.side),
                picture(production_facility_mini(facility.class_dat_id).map(|(mini, _)| mini)),
                class_name(world, facility.class_dat_id),
                manufacturing_rows(world, system, facility.class_dat_id, None),
            )
        }
        // FUN_00446c50 on an object not yet completed or en route;
        // FUN_0042c3b0(.., 1, 1) lays STRATEGY 11571 (side 1) / 11573 under
        // an unfinished portrait (bit 2 clear) and 11506 under a facility
        // en route (bit 4; 11510 for 0x22..0x27).
        StatusObject::PendingFacility {
            system,
            class,
            owner,
            arrival,
        } => {
            let still_there = match arrival {
                Some(day) => sources.today <= day,
                None => crate::manufacturing_window::pending_units(sources.manufacturing, system)
                    .any(|kind| pending_class_id(kind) == Some(class)),
            };
            if !still_there {
                return None;
            }
            // FUN_00443130 fills families 0x20..0x27 with FUN_00444e20.
            let defense = (0x20..=0x27).contains(&class.family());
            let (title, mini, rows) = if defense {
                (
                    "Defense Facility Status",
                    defense_facility_mini(class),
                    defense_rows(world, system, class, Some(arrival)),
                )
            } else {
                (
                    "Manufacturing Status",
                    manufacturing_facility_mini(class).or_else(|| production_facility_mini(class)),
                    manufacturing_rows(world, system, class, Some(arrival)),
                )
            };
            let marks = match arrival {
                Some(_) if (0x22..=0x27).contains(&class.family()) => {
                    vec![(DllSource::Strategy, 11_510)]
                }
                Some(_) => vec![(DllSource::Strategy, 11_506)],
                None => vec![unfinished_mark(owner == 1)],
            };
            StatusView {
                marks,
                ..view(
                    title,
                    usize::from(owner != 1),
                    picture(mini.map(|(mini, _)| mini)),
                    class_name(world, class),
                    rows,
                )
            }
        }
        // FUN_00445280 on a regiment not yet completed: 34673 "Training",
        // its backdrop and 11571 / 11573 (FUN_0042c3b0).
        // FUN_00446fd0 on an unfinished ship: Status 34647 "Under
        // Construction" (bit 2 clear), its picture over the side's grid
        // (A0 2026-11-15 "u-2-carrack-status": Fleet 4, Under Construction,
        // Maintenance Cost 24, Hull 1000:1000).
        StatusObject::PendingShip { product } => {
            let product = world.products.get(product)?;
            let BuildableKind::CapitalShip(class_key) = product.kind else {
                return None;
            };
            let rebellion_core::products::Container::Fleet(fleet) = product.container else {
                return None;
            };
            let class = world.capital_ship_classes.get(class_key)?;
            let portrait = picture(capital_ship_mini_id(class.dat_id));
            let ship = rebellion_core::world::ShipInstance::new(
                class_key,
                class.hull.cast_signed(),
                product.is_alliance,
            );
            // Finished and travelling to its fleet: "Enroute" and its ETA
            // (FUN_004fd2b0), with the en route mark instead.
            let arrival = world.products.arrival(product.id);
            let mut rows = vec![
                row("Class:", class.name.clone()),
                row("Fleet:", fleet_label(world, fleet).unwrap_or_default()),
                row("Status:", unfinished_status(arrival).to_owned()),
            ];
            eta(&mut rows, arrival);
            class_rows(world, class, &ship, [0, 0, 0], &mut rows);
            StatusView {
                marks: if arrival.is_some() {
                    craft_marks(portrait, true, false)
                } else {
                    vec![unfinished_mark(product.is_alliance)]
                },
                ..view(
                    "Capital Ship Status",
                    side_index(product.is_alliance),
                    portrait,
                    product.name.clone().unwrap_or_else(|| class.name.clone()),
                    rows,
                )
            }
        }
        StatusObject::PendingTroop { system, class } => {
            // The regiment stands where its product does (a Move may
            // have carried it).
            // On the planet or aboard a fleet there (A0 2026-11-15 "m-5").
            let product = world.products.all().iter().find(|product| {
                product.kind == BuildableKind::Troop(class)
                    && rebellion_core::products::container_system(world, product.container)
                        == Some(system)
            })?;
            // Finished and on its way (bits 2 and 4): FUN_00445280 writes
            // "Enroute" and the ETA, over the en route mark.
            let arrival = world.products.arrival(product.id);
            let alliance = world.buildable_classes.get(&class)?.is_alliance;
            let portrait = picture(troop_mini(class).map(|(mini, _)| mini));
            let mut marks = regiment_marks(portrait, class, arrival.is_some());
            if arrival.is_none() {
                marks.push(unfinished_mark(alliance));
            }
            let status = if arrival.is_some() { "Enroute" } else { "Training" };
            StatusView {
                marks,
                ..view(
                    "Trooper Regiment Status",
                    side_index(alliance),
                    portrait,
                    class_name(world, class),
                    regiment_rows(world, class, system_name(world, system), status, arrival),
                )
            }
        }
        // FUN_004444c0 on a squadron not yet completed: the planet
        // squadron's rows (A0 pending-facility/25) over 11571 / 11573.
        StatusObject::PendingFighter { system, class } => {
            let queued = world.products.all().iter().any(|product| {
                product.kind == BuildableKind::Fighter(class)
                    && rebellion_core::products::container_system(world, product.container)
                        == Some(system)
            });
            if !queued {
                return None;
            }
            let fighter = world.fighter_classes.get(class)?;
            StatusView {
                marks: vec![unfinished_mark(fighter.is_alliance)],
                ..view(
                    "Fighter Squadron Status",
                    side_index(fighter.is_alliance),
                    picture(fighter_mini_id(fighter.dat_id)),
                    fighter.name.clone(),
                    planet_fighter_rows(sources, system, class)?,
                )
            }
        }
        // FUN_004444c0 on a squadron in hyperspace: FUN_0042c3b0(.., 1, 1)
        // lays its own en route mark (portrait + 0x1000) under the
        // portrait. The window stays only while the squadron travels.
        StatusObject::TravellingFighter { to, class, arrival } => {
            let travelling = sources
                .transport
                .squadrons
                .transits()
                .iter()
                .any(|transit| {
                    transit.class == class && transit.arrival_tick == arrival && transit.to == to
                });
            if !travelling {
                return None;
            }
            let fighter = world.fighter_classes.get(class)?;
            let portrait = picture(fighter_mini_id(fighter.dat_id));
            StatusView {
                marks: craft_marks(portrait, true, false),
                ..view(
                    "Fighter Squadron Status",
                    side_index(fighter.is_alliance),
                    portrait,
                    fighter.name.clone(),
                    travelling_fighter_rows(sources, to, class, arrival)?,
                )
            }
        }
        StatusObject::Mission(id) => {
            let mission = sources
                .missions
                .missions()
                .iter()
                .find(|mission| mission.id == id)?;
            let side = match mission.faction {
                MissionFaction::Alliance => 0,
                MissionFaction::Empire => 1,
            };
            view(
                "Mission Status",
                side,
                Some((DllSource::Gokres, kind_icon(mission.kind, mission.faction))),
                kind_name(mission.kind).to_owned(),
                mission_rows(sources, id)?,
            )
        }
        StatusObject::Producer { system, area } => {
            let (title, _, _) = manager(area);
            // FUN_00443130: GOKRES 263 (0xa0..0xa1), 262 (0xa2..0xa3), 264.
            let picture = match area {
                ProductionArea::ConstructionYard => 263,
                ProductionArea::Shipyard => 262,
                ProductionArea::TrainingFacility => 264,
            };
            StatusView {
                manager_title: true,
                ..view(
                    title,
                    holder_index(world, system),
                    Some((DllSource::Gokres, picture)),
                    manager_name(area).into(),
                    producer_rows(sources, system, area)?,
                )
            }
        }
    })
}

#[cfg(test)]
#[path = "status_rows_tests.rs"]
mod tests;
