//! Player orders as commands the app and the replay runner share
//! (`docs/plans/2026-11-24-player-orders-in-replay.md`).
//!
//! The app turns a player's order into a `PanelAction`; each state-changing
//! action moves here, one family a stage, as a function over the states it
//! touches. The app calls the function with its keys and maps the outcome to
//! its messages and sounds; a replay names the same objects by ids that
//! survive a fresh run from the same seed ([`PlayerCommand`]) and applies it
//! through [`apply`]. One function, two callers.
//!
//! port: the whole module is the port's own command boundary; the rules it
//! runs are the ones the called core functions cite.

use std::collections::HashSet;

use rebellion_core::blockade::BlockadeState;
use rebellion_core::fleet_join::{join_fleet, FleetMover, FleetOrderError, FleetOrderOutcome};
use rebellion_core::headquarters_move::{move_headquarters, HeadquartersMove, HeadquartersMoveError};
use rebellion_core::ids::{
    CapitalShipKey, CharacterKey, DatId, FighterKey, FleetKey, SpecialForceKey, SystemKey, TroopKey,
};
use rebellion_core::missions::{
    MissionFaction, MissionKind, MissionMember, MissionRefusal, MissionRequest, MissionState,
    MissionTarget,
};
use rebellion_core::manufacturing::{BuildableKind, FacilityBuild, ManufacturingState, ProductionArea};
use rebellion_core::squadrons::{SquadronMove, SquadronMoveError, SquadronPlace};
use rebellion_core::troop_transport::{RegimentMove, RegimentTarget};
use rebellion_core::victory::VictoryState;
use rebellion_core::movement::{
    begin_faction_fleet_transit, validate_fleet_dispatch, AppliedDeparture, FleetDispatchError,
    MovementState,
};
use rebellion_core::troop_transport::{TroopTransportError, TroopTransportState};
use rebellion_core::world::GameWorld;
use serde::{Deserialize, Serialize};

/// A player's order in a replay, its objects named by stable ids: systems
/// and characters by `DatId`, fleets by creation serial
/// ([`GameWorld::fleet_serial`]), regiments by object id
/// (`ObjectViews::regiment_id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "order", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlayerCommand {
    /// A fleet's move to a system, first taking aboard the regiments named
    /// (`PanelAction::DispatchFleet`).
    DispatchFleet {
        fleet: u64,
        destination: DatId,
        troops: Vec<DatId>,
    },
    /// A regiment's move to a planet or a fleet (`PanelAction::MoveRegiment`).
    MoveRegiment { troop: DatId, target: PlaceId },
    /// One squadron of a class moved between a planet and a fleet
    /// (`PanelAction::MoveSquadron`).
    MoveSquadron {
        class: DatId,
        from: PlaceId,
        to: PlaceId,
    },
    /// A fleet, or ships of one, joining another fleet
    /// (`PanelAction::JoinFleet`).
    JoinFleet { mover: MoverId, target: u64 },
    /// The Alliance headquarters moved to a system
    /// (`PanelAction::MoveHeadquarters`).
    MoveHeadquarters { destination: DatId },
    /// A production area's Reserved flag, or the whole system's when no
    /// area is named (`PanelAction::ToggleReserved`).
    ToggleReserved {
        system: DatId,
        area: Option<ProductionArea>,
    },
    /// A production area stopped (`PanelAction::StopProduction`).
    StopProduction { system: DatId, area: ProductionArea },
    /// Units ordered from a production area
    /// (`PanelAction::BuildProduction`).
    BuildProduction {
        system: DatId,
        area: ProductionArea,
        kind: BuildId,
        count: u32,
    },
    /// A production area's Destination, or every area's when none is named
    /// (`PanelAction::SetDestination`).
    SetDestination {
        system: DatId,
        area: Option<ProductionArea>,
        destination: DatId,
    },
    /// A production area's Destination set to a fleet at `at`, or one ship
    /// of it by its id (`PanelAction::SetFleetDestination`).
    SetFleetDestination {
        system: DatId,
        area: ProductionArea,
        fleet: u64,
        at: DatId,
        ship: Option<u64>,
    },
    /// A queued product renamed, by its product id
    /// (`PanelAction::RenameProduct`).
    RenameProduct { product: u64, name: String },
    /// A fleet, or one of its ships by list place, renamed
    /// (`PanelAction::Rename`).
    Rename {
        fleet: u64,
        ship: Option<usize>,
        name: String,
    },
    /// Ships of a fleet made a new fleet (`PanelAction::CreateFleet`).
    CreateFleet {
        fleet: u64,
        ships: Vec<usize>,
        roster: u64,
    },
    /// Ships of a fleet moved into the side's spare fleet at a system
    /// (`PanelAction::MoveShips`).
    MoveShips {
        fleet: u64,
        ships: Vec<usize>,
        roster: u64,
        system: DatId,
    },
    /// Members sent by a Move mission to a system, or to a fleet there
    /// (`PanelAction::MoveCharacter`, `MoveSpecialForce`, `MoveMembers`).
    MoveMembers {
        members: Vec<MemberId>,
        system: DatId,
        fleet: Option<u64>,
    },
    /// Queued or finished products moved to a system or a fleet there, by
    /// product id (`PanelAction::MoveProducts`).
    MoveProducts {
        products: Vec<u64>,
        system: DatId,
        fleet: Option<u64>,
    },
    /// A character's command order `0x260..0x263`
    /// (`PanelAction::OrderCommand`).
    OrderCommand { character: DatId, order_id: u16 },
    /// A member retired (`PanelAction::Retire`).
    Retire { member: MemberId },
    /// Planetary Bombardment of a system by fleets
    /// (`PanelAction::OrderBombardment`).
    OrderBombardment {
        fleets: Vec<u64>,
        system: DatId,
        kind: u32,
    },
    /// Planetary Assault of a system by fleets (`PanelAction::OrderAssault`).
    OrderAssault { fleets: Vec<u64>, system: DatId },
    /// A mission sent (`PanelAction::DispatchMission`).
    DispatchMission {
        kind: MissionKind,
        team: Vec<MemberId>,
        decoys: Vec<MemberId>,
        target: DatId,
        target_character: Option<DatId>,
        target_object: Option<TargetId>,
    },
}

/// Where an object without an id of its own stands: a system, a fleet or
/// an abode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Holder {
    System(DatId),
    Fleet(u64),
    Abode(DatId),
}

/// A special force: its holder and its place in the holder's list (the
/// order the original keeps a container's children in, the same in a
/// fresh run from the same seed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotId {
    pub holder: Holder,
    pub index: usize,
}

/// A mission member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MemberId {
    Character(DatId),
    SpecialForce(SlotId),
}

/// A Sabotage target: a facility by its system and place in that
/// system's list, a regiment by object id, a special force by slot, a
/// fleet's ship or squadron by the fleet's serial and the class's DatId.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TargetId {
    DefenseFacility { system: DatId, index: usize },
    ManufacturingFacility { system: DatId, index: usize },
    ProductionFacility { system: DatId, index: usize },
    Troop(DatId),
    SpecialForce(SlotId),
    DeathStar(u64),
    CapitalShip { fleet: u64, class: DatId },
    Fighter { fleet: u64, class: DatId },
}

/// What a production order builds: a class by its `DatId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BuildId {
    CapitalShip(DatId),
    Fighter(DatId),
    Troop(DatId),
    SpecialForce(DatId),
    DefenseFacility { class: DatId, is_alliance: bool },
    ManufacturingFacility { class: DatId, is_alliance: bool },
    ProductionFacility { class: DatId, is_alliance: bool },
}

/// A planet (its system's `DatId`) or a fleet (its creation serial).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PlaceId {
    System(DatId),
    Fleet(u64),
}

/// A whole fleet, or ships of it by their place in its list (the order is
/// the same in a fresh run from the same seed) with the roster stamp the
/// selection was made on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MoverId {
    Fleet(u64),
    Ships {
        fleet: u64,
        ships: Vec<usize>,
        roster: u64,
    },
}

/// Why a command could not name its objects in this world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandIdError {
    Fleet(u64),
    System(DatId),
    Regiment(DatId),
    FighterClass(DatId),
    ShipClass(DatId),
    Character(DatId),
    Slot,
    Unnamed,
}

impl std::fmt::Display for CommandIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fleet(serial) => write!(f, "no fleet with creation serial {serial}"),
            Self::System(id) => write!(f, "no system {:#010x}", id.0),
            Self::Regiment(id) => write!(f, "no regiment {:#010x}", id.0),
            Self::FighterClass(id) => write!(f, "no fighter class {:#010x}", id.0),
            Self::ShipClass(id) => write!(f, "no capital ship class {:#010x}", id.0),
            Self::Character(id) => write!(f, "no character {:#010x}", id.0),
            Self::Slot => write!(f, "no object in that slot"),
            Self::Unnamed => write!(f, "an object has no stable id"),
        }
    }
}

impl std::error::Error for CommandIdError {}

/// Why a fleet's move was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchRefusal {
    /// The move itself (validation or departure).
    Move(FleetDispatchError),
    /// Taking the named regiments aboard.
    Embark(TroopTransportError),
}

/// A fleet's move with the regiments it takes aboard (the app's
/// `PanelAction::DispatchFleet`): validate the move, embark the regiments,
/// depart; a departure refused after the embarkation puts them back.
///
/// # Errors
/// The refusal the app reports: the move's validation or departure, or the
/// embarkation when regiments were named.
pub fn dispatch_fleet(
    world: &mut GameWorld,
    movement: &mut MovementState,
    troop_transport: &mut TroopTransportState,
    fleet: FleetKey,
    destination: SystemKey,
    troops: &[TroopKey],
    expected_is_alliance: bool,
) -> Result<AppliedDeparture, DispatchRefusal> {
    validate_fleet_dispatch(movement, world, fleet, destination, expected_is_alliance)
        .map_err(DispatchRefusal::Move)?;
    if let Err(error) = troop_transport.embark(world, fleet, troops) {
        if !troops.is_empty() {
            return Err(DispatchRefusal::Embark(error));
        }
    }
    begin_faction_fleet_transit(movement, world, fleet, destination, expected_is_alliance).map_err(
        |error| {
            if !troops.is_empty() {
                let origin = world
                    .fleets
                    .get(fleet)
                    .map(|value| value.location)
                    .unwrap_or_default();
                let _ = troop_transport.disembark_selected(world, fleet, origin, troops);
            }
            DispatchRefusal::Move(error)
        },
    )
}

/// The fleet a creation serial names.
fn fleet_key(world: &GameWorld, serial: u64) -> Result<FleetKey, CommandIdError> {
    world
        .fleets
        .keys()
        .find(|&fleet| world.fleet_serial(fleet) == serial)
        .ok_or(CommandIdError::Fleet(serial))
}

fn system_key(world: &GameWorld, id: DatId) -> Result<SystemKey, CommandIdError> {
    world
        .systems
        .iter()
        .find(|(_, system)| system.dat_id == id)
        .map(|(key, _)| key)
        .ok_or(CommandIdError::System(id))
}

fn troop_key(world: &GameWorld, id: DatId) -> Result<TroopKey, CommandIdError> {
    world
        .object_views
        .regiment_key(world, id)
        .ok_or(CommandIdError::Regiment(id))
}

/// A regiment's move for the side `expected_is_alliance` (the app's
/// `PanelAction::MoveRegiment`): only the side's own regiment moves.
///
/// # Errors
/// The transport's refusal, or `WrongFaction` for the other side's
/// regiment.
#[allow(clippy::too_many_arguments)]
pub fn move_regiment(
    world: &mut GameWorld,
    troop_transport: &mut TroopTransportState,
    movement: &MovementState,
    blockaded: &HashSet<SystemKey>,
    troop: TroopKey,
    target: RegimentTarget,
    tick: u64,
    expected_is_alliance: bool,
) -> Result<RegimentMove, TroopTransportError> {
    if !world
        .troops
        .get(troop)
        .is_some_and(|value| value.is_alliance == expected_is_alliance)
    {
        return Err(TroopTransportError::WrongFaction);
    }
    troop_transport.move_regiment(world, movement, blockaded, troop, target, tick)
}

/// One squadron of `class` moved for the side (the app's
/// `PanelAction::MoveSquadron`).
///
/// # Errors
/// The squadrons' refusal.
pub fn move_squadron(
    world: &mut GameWorld,
    troop_transport: &mut TroopTransportState,
    is_alliance: bool,
    class: FighterKey,
    from: SquadronPlace,
    to: SquadronPlace,
    tick: u64,
) -> Result<SquadronMove, SquadronMoveError> {
    troop_transport
        .squadrons
        .move_squadrons(world, is_alliance, class, 1, from, to, tick)
}

fn place_id(world: &GameWorld, place: SquadronPlace) -> Result<PlaceId, CommandIdError> {
    match place {
        SquadronPlace::Planet(system) => Ok(PlaceId::System(
            world.systems.get(system).ok_or(CommandIdError::Unnamed)?.dat_id,
        )),
        SquadronPlace::Fleet(fleet) => fleet_id(world, fleet).map(PlaceId::Fleet),
    }
}

fn fleet_id(world: &GameWorld, fleet: FleetKey) -> Result<u64, CommandIdError> {
    match world.fleet_serial(fleet) {
        0 => Err(CommandIdError::Unnamed),
        serial => Ok(serial),
    }
}

fn place_key(world: &GameWorld, place: PlaceId) -> Result<SquadronPlace, CommandIdError> {
    Ok(match place {
        PlaceId::System(id) => SquadronPlace::Planet(system_key(world, id)?),
        PlaceId::Fleet(serial) => SquadronPlace::Fleet(fleet_key(world, serial)?),
    })
}

fn mover_id(world: &GameWorld, mover: &FleetMover) -> Result<MoverId, CommandIdError> {
    Ok(match mover {
        FleetMover::Fleet(fleet) => MoverId::Fleet(fleet_id(world, *fleet)?),
        FleetMover::Ships {
            fleet,
            ships,
            roster,
        } => MoverId::Ships {
            fleet: fleet_id(world, *fleet)?,
            ships: ships.clone(),
            roster: *roster,
        },
    })
}

fn mover_key(world: &GameWorld, mover: &MoverId) -> Result<FleetMover, CommandIdError> {
    Ok(match mover {
        MoverId::Fleet(serial) => FleetMover::Fleet(fleet_key(world, *serial)?),
        MoverId::Ships {
            fleet,
            ships,
            roster,
        } => FleetMover::Ships {
            fleet: fleet_key(world, *fleet)?,
            ships: ships.clone(),
            roster: *roster,
        },
    })
}

fn fighter_class_key(world: &GameWorld, id: DatId) -> Result<FighterKey, CommandIdError> {
    world
        .fighter_classes
        .iter()
        .find(|(_, class)| class.dat_id == id)
        .map(|(key, _)| key)
        .ok_or(CommandIdError::FighterClass(id))
}

fn system_id(world: &GameWorld, system: SystemKey) -> Result<DatId, CommandIdError> {
    world
        .systems
        .get(system)
        .map(|value| value.dat_id)
        .ok_or(CommandIdError::Unnamed)
}

/// The replay form of a regiment's move.
///
/// # Errors
/// An object without a stable id.
pub fn move_regiment_command(
    world: &GameWorld,
    troop: TroopKey,
    target: RegimentTarget,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::MoveRegiment {
        troop: world.object_views.regiment_id(troop).ok_or(CommandIdError::Unnamed)?,
        target: match target {
            RegimentTarget::System(system) => PlaceId::System(system_id(world, system)?),
            RegimentTarget::Fleet(fleet) => PlaceId::Fleet(fleet_id(world, fleet)?),
        },
    })
}

/// The replay form of a squadron's move.
///
/// # Errors
/// An object without a stable id.
pub fn move_squadron_command(
    world: &GameWorld,
    class: FighterKey,
    from: SquadronPlace,
    to: SquadronPlace,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::MoveSquadron {
        class: world
            .fighter_classes
            .get(class)
            .map(|value| value.dat_id)
            .ok_or(CommandIdError::Unnamed)?,
        from: place_id(world, from)?,
        to: place_id(world, to)?,
    })
}

/// The replay form of a fleet join.
///
/// # Errors
/// An object without a stable id.
pub fn join_fleet_command(
    world: &GameWorld,
    mover: &FleetMover,
    target: FleetKey,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::JoinFleet {
        mover: mover_id(world, mover)?,
        target: fleet_id(world, target)?,
    })
}

/// A production area's Reserved flag flipped, or the system's when `area`
/// is `None` (the app's `PanelAction::ToggleReserved`). Returns the new
/// flag.
pub fn toggle_reserved(
    manufacturing: &mut ManufacturingState,
    system: SystemKey,
    area: Option<ProductionArea>,
) -> bool {
    match area {
        Some(area) => {
            let reserved = !manufacturing.is_reserved(system, area);
            manufacturing.set_reserved(system, area, reserved);
            reserved
        }
        None => manufacturing.toggle_system_reserved(system),
    }
}

fn build_id(world: &GameWorld, kind: BuildableKind) -> Result<BuildId, CommandIdError> {
    Ok(match kind {
        BuildableKind::CapitalShip(class) => BuildId::CapitalShip(
            world.capital_ship_classes.get(class).ok_or(CommandIdError::Unnamed)?.dat_id,
        ),
        BuildableKind::Fighter(class) => BuildId::Fighter(
            world.fighter_classes.get(class).ok_or(CommandIdError::Unnamed)?.dat_id,
        ),
        BuildableKind::Troop(id) => BuildId::Troop(id),
        BuildableKind::SpecialForce(id) => BuildId::SpecialForce(id),
        BuildableKind::DefenseFacility(FacilityBuild { class, is_alliance }) => {
            BuildId::DefenseFacility { class, is_alliance }
        }
        BuildableKind::ManufacturingFacility(FacilityBuild { class, is_alliance }) => {
            BuildId::ManufacturingFacility { class, is_alliance }
        }
        BuildableKind::ProductionFacility(FacilityBuild { class, is_alliance }) => {
            BuildId::ProductionFacility { class, is_alliance }
        }
    })
}

fn build_kind(world: &GameWorld, kind: BuildId) -> Result<BuildableKind, CommandIdError> {
    Ok(match kind {
        BuildId::CapitalShip(id) => BuildableKind::CapitalShip(
            world
                .capital_ship_classes
                .iter()
                .find(|(_, class)| class.dat_id == id)
                .map(|(key, _): (CapitalShipKey, _)| key)
                .ok_or(CommandIdError::ShipClass(id))?,
        ),
        BuildId::Fighter(id) => BuildableKind::Fighter(fighter_class_key(world, id)?),
        BuildId::Troop(id) => BuildableKind::Troop(id),
        BuildId::SpecialForce(id) => BuildableKind::SpecialForce(id),
        BuildId::DefenseFacility { class, is_alliance } => {
            BuildableKind::DefenseFacility(FacilityBuild { class, is_alliance })
        }
        BuildId::ManufacturingFacility { class, is_alliance } => {
            BuildableKind::ManufacturingFacility(FacilityBuild { class, is_alliance })
        }
        BuildId::ProductionFacility { class, is_alliance } => {
            BuildableKind::ProductionFacility(FacilityBuild { class, is_alliance })
        }
    })
}

/// The replay form of a Reserved toggle.
///
/// # Errors
/// A missing system.
pub fn toggle_reserved_command(
    world: &GameWorld,
    system: SystemKey,
    area: Option<ProductionArea>,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::ToggleReserved {
        system: system_id(world, system)?,
        area,
    })
}

/// The replay form of a Stop order.
///
/// # Errors
/// A missing system.
pub fn stop_production_command(
    world: &GameWorld,
    system: SystemKey,
    area: ProductionArea,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::StopProduction {
        system: system_id(world, system)?,
        area,
    })
}

/// The replay form of a production order.
///
/// # Errors
/// A missing system or class.
pub fn build_production_command(
    world: &GameWorld,
    system: SystemKey,
    area: ProductionArea,
    kind: BuildableKind,
    count: u32,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::BuildProduction {
        system: system_id(world, system)?,
        area,
        kind: build_id(world, kind)?,
        count,
    })
}

fn character_id(world: &GameWorld, character: CharacterKey) -> Result<DatId, CommandIdError> {
    world
        .characters
        .get(character)
        .map(|value| value.dat_id)
        .ok_or(CommandIdError::Unnamed)
}

fn character_key(world: &GameWorld, id: DatId) -> Result<CharacterKey, CommandIdError> {
    world
        .characters
        .iter()
        .find(|(_, value)| value.dat_id == id)
        .map(|(key, _)| key)
        .ok_or(CommandIdError::Character(id))
}

fn special_force_slot(world: &GameWorld, unit: SpecialForceKey) -> Result<SlotId, CommandIdError> {
    let systems = world.systems.values().filter_map(|system| {
        let index = system.special_forces.iter().position(|&key| key == unit)?;
        Some(SlotId {
            holder: Holder::System(system.dat_id),
            index,
        })
    });
    let fleets = world.fleets.iter().filter_map(|(fleet, value)| {
        let index = value.special_forces.iter().position(|&key| key == unit)?;
        Some(SlotId {
            holder: Holder::Fleet(world.fleet_serial(fleet)),
            index,
        })
    });
    let abodes = world.abodes.values().filter_map(|abode| {
        let index = abode.special_forces.iter().position(|&key| key == unit)?;
        Some(SlotId {
            holder: Holder::Abode(abode.dat_id),
            index,
        })
    });
    systems
        .chain(fleets)
        .chain(abodes)
        .find(|slot| slot.holder != Holder::Fleet(0))
        .ok_or(CommandIdError::Unnamed)
}

fn special_force_key(world: &GameWorld, slot: SlotId) -> Result<SpecialForceKey, CommandIdError> {
    let list = match slot.holder {
        Holder::System(id) => &world.systems[system_key(world, id)?].special_forces,
        Holder::Fleet(serial) => &world.fleets[fleet_key(world, serial)?].special_forces,
        Holder::Abode(id) => {
            &world
                .abodes
                .values()
                .find(|abode| abode.dat_id == id)
                .ok_or(CommandIdError::Slot)?
                .special_forces
        }
    };
    list.get(slot.index).copied().ok_or(CommandIdError::Slot)
}

fn member_id(world: &GameWorld, member: MissionMember) -> Result<MemberId, CommandIdError> {
    Ok(match member {
        MissionMember::Character(key) => MemberId::Character(character_id(world, key)?),
        MissionMember::SpecialForce(key) => MemberId::SpecialForce(special_force_slot(world, key)?),
    })
}

fn member_key(world: &GameWorld, member: MemberId) -> Result<MissionMember, CommandIdError> {
    Ok(match member {
        MemberId::Character(id) => MissionMember::Character(character_key(world, id)?),
        MemberId::SpecialForce(slot) => MissionMember::SpecialForce(special_force_key(world, slot)?),
    })
}

/// The system holding a facility in one of its lists, and its place there.
fn facility_slot<K: PartialEq + Copy>(
    world: &GameWorld,
    key: K,
    list: impl Fn(&rebellion_core::world::System) -> &[K],
) -> Result<(DatId, usize), CommandIdError> {
    world
        .systems
        .values()
        .find_map(|system| {
            let index = list(system).iter().position(|&k| k == key)?;
            Some((system.dat_id, index))
        })
        .ok_or(CommandIdError::Unnamed)
}

fn facility_key<K: Copy>(
    world: &GameWorld,
    system: DatId,
    index: usize,
    list: impl Fn(&rebellion_core::world::System) -> &[K],
) -> Result<K, CommandIdError> {
    list(&world.systems[system_key(world, system)?])
        .get(index)
        .copied()
        .ok_or(CommandIdError::Slot)
}

fn target_id(world: &GameWorld, target: MissionTarget) -> Result<TargetId, CommandIdError> {
    Ok(match target {
        MissionTarget::DefenseFacility(key) => {
            let (system, index) = facility_slot(world, key, |s| &s.defense_facilities)?;
            TargetId::DefenseFacility { system, index }
        }
        MissionTarget::ManufacturingFacility(key) => {
            let (system, index) = facility_slot(world, key, |s| &s.manufacturing_facilities)?;
            TargetId::ManufacturingFacility { system, index }
        }
        MissionTarget::ProductionFacility(key) => {
            let (system, index) = facility_slot(world, key, |s| &s.production_facilities)?;
            TargetId::ProductionFacility { system, index }
        }
        MissionTarget::Troop(key) => {
            TargetId::Troop(world.object_views.regiment_id(key).ok_or(CommandIdError::Unnamed)?)
        }
        MissionTarget::SpecialForce(key) => TargetId::SpecialForce(special_force_slot(world, key)?),
        MissionTarget::DeathStar(fleet) => TargetId::DeathStar(fleet_id(world, fleet)?),
        MissionTarget::CapitalShip { fleet, class } => TargetId::CapitalShip {
            fleet: fleet_id(world, fleet)?,
            class: world
                .capital_ship_classes
                .get(class)
                .ok_or(CommandIdError::Unnamed)?
                .dat_id,
        },
        MissionTarget::Fighter { fleet, class } => TargetId::Fighter {
            fleet: fleet_id(world, fleet)?,
            class: world.fighter_classes.get(class).ok_or(CommandIdError::Unnamed)?.dat_id,
        },
    })
}

fn ship_class_key(world: &GameWorld, id: DatId) -> Result<CapitalShipKey, CommandIdError> {
    world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| class.dat_id == id)
        .map(|(key, _)| key)
        .ok_or(CommandIdError::ShipClass(id))
}

fn target_key(world: &GameWorld, target: TargetId) -> Result<MissionTarget, CommandIdError> {
    Ok(match target {
        TargetId::DefenseFacility { system, index } => MissionTarget::DefenseFacility(
            facility_key(world, system, index, |s| &s.defense_facilities)?,
        ),
        TargetId::ManufacturingFacility { system, index } => MissionTarget::ManufacturingFacility(
            facility_key(world, system, index, |s| &s.manufacturing_facilities)?,
        ),
        TargetId::ProductionFacility { system, index } => MissionTarget::ProductionFacility(
            facility_key(world, system, index, |s| &s.production_facilities)?,
        ),
        TargetId::Troop(id) => MissionTarget::Troop(troop_key(world, id)?),
        TargetId::SpecialForce(slot) => MissionTarget::SpecialForce(special_force_key(world, slot)?),
        TargetId::DeathStar(serial) => MissionTarget::DeathStar(fleet_key(world, serial)?),
        TargetId::CapitalShip { fleet, class } => MissionTarget::CapitalShip {
            fleet: fleet_key(world, fleet)?,
            class: ship_class_key(world, class)?,
        },
        TargetId::Fighter { fleet, class } => MissionTarget::Fighter {
            fleet: fleet_key(world, fleet)?,
            class: fighter_class_key(world, class)?,
        },
    })
}

/// The replay form of a mission order (its side goes with the actor, its
/// day with the command's tick).
///
/// # Errors
/// An object without a stable id.
pub fn dispatch_mission_command(
    world: &GameWorld,
    request: &MissionRequest,
) -> Result<PlayerCommand, CommandIdError> {
    let members = |list: &[MissionMember]| {
        list.iter()
            .map(|&member| member_id(world, member))
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(PlayerCommand::DispatchMission {
        kind: request.kind,
        team: members(&request.team)?,
        decoys: members(&request.decoys)?,
        target: system_id(world, request.target_system)?,
        target_character: request
            .target_character
            .map(|key| character_id(world, key))
            .transpose()?,
        target_object: request
            .target_object
            .map(|target| target_id(world, target))
            .transpose()?,
    })
}

/// A mission order (the app's `PanelAction::DispatchMission`), through the
/// missions' own guard.
///
/// # Errors
/// The missions' refusal.
pub fn dispatch_mission(
    world: &mut GameWorld,
    missions: &mut MissionState,
    request: MissionRequest,
) -> Result<u64, MissionRefusal> {
    missions.dispatch_guarded(request, world)
}

/// A production Destination for one area, or for every area when `area`
/// is `None` (the app's `PanelAction::SetDestination`).
pub fn set_destination(
    manufacturing: &mut ManufacturingState,
    system: SystemKey,
    area: Option<ProductionArea>,
    destination: SystemKey,
) -> usize {
    let areas = area.map_or(ProductionArea::ALL.to_vec(), |area| vec![area]);
    for &area in &areas {
        manufacturing.set_destination(system, area, destination);
    }
    areas.len()
}

/// A production Destination that is a fleet, or one ship of it (the app's
/// `PanelAction::SetFleetDestination`). False when refused.
pub fn set_fleet_destination(
    manufacturing: &mut ManufacturingState,
    system: SystemKey,
    area: ProductionArea,
    fleet: FleetKey,
    at: SystemKey,
    ship: Option<u64>,
) -> bool {
    match ship {
        Some(ship) => manufacturing.set_ship_destination(system, area, fleet, ship, at),
        None => manufacturing.set_fleet_destination(system, area, fleet, at),
    }
}

/// A fleet, or one of its ships, renamed (the app's `PanelAction::Rename`).
/// False when refused.
pub fn rename(world: &mut GameWorld, fleet: FleetKey, ship: Option<usize>, name: &str) -> bool {
    match ship {
        Some(index) => world.rename_ship(fleet, index, name),
        None => world.rename_fleet(fleet, name),
    }
}

/// The replay form of a production Destination.
///
/// # Errors
/// A missing system.
pub fn set_destination_command(
    world: &GameWorld,
    system: SystemKey,
    area: Option<ProductionArea>,
    destination: SystemKey,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::SetDestination {
        system: system_id(world, system)?,
        area,
        destination: system_id(world, destination)?,
    })
}

/// The replay form of a fleet Destination.
///
/// # Errors
/// A missing system or an unnamed fleet.
pub fn set_fleet_destination_command(
    world: &GameWorld,
    system: SystemKey,
    area: ProductionArea,
    fleet: FleetKey,
    at: SystemKey,
    ship: Option<u64>,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::SetFleetDestination {
        system: system_id(world, system)?,
        area,
        fleet: fleet_id(world, fleet)?,
        at: system_id(world, at)?,
        ship,
    })
}

/// The replay form of a fleet or ship rename.
///
/// # Errors
/// An unnamed fleet.
pub fn rename_command(
    world: &GameWorld,
    fleet: FleetKey,
    ship: Option<usize>,
    name: &str,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::Rename {
        fleet: fleet_id(world, fleet)?,
        ship,
        name: name.to_owned(),
    })
}

/// The replay form of a Create Fleet order.
///
/// # Errors
/// An unnamed fleet.
pub fn create_fleet_command(
    world: &GameWorld,
    fleet: FleetKey,
    ships: &[usize],
    roster: u64,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::CreateFleet {
        fleet: fleet_id(world, fleet)?,
        ships: ships.to_vec(),
        roster,
    })
}

/// The replay form of ships moved into the side's spare fleet.
///
/// # Errors
/// An unnamed fleet or a missing system.
pub fn move_ships_command(
    world: &GameWorld,
    fleet: FleetKey,
    ships: &[usize],
    roster: u64,
    system: SystemKey,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::MoveShips {
        fleet: fleet_id(world, fleet)?,
        ships: ships.to_vec(),
        roster,
        system: system_id(world, system)?,
    })
}

/// Why a Move order was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveRefusal {
    /// The place is the other side's (`FUN_004f4b60`, `1`/`0x28`).
    OtherSide,
    /// The mission's own refusal.
    Mission(MissionRefusal),
}

/// Members sent by a Move mission to `system`, or to `fleet` there (the
/// app's `MoveCharacter`, `MoveSpecialForce` and `MoveMembers`): the place
/// must be open to the side (`missions::move_place_allowed`), then the
/// Move mission goes, aimed at the fleet when one is named. Returns the
/// mission's id.
///
/// # Errors
/// The place's or the mission's refusal.
pub fn move_members(
    world: &mut GameWorld,
    missions: &mut MissionState,
    faction: MissionFaction,
    members: Vec<MissionMember>,
    system: SystemKey,
    fleet: Option<FleetKey>,
    tick: u64,
) -> Result<u64, MoveRefusal> {
    if !rebellion_core::missions::move_place_allowed(world, faction, system, fleet) {
        return Err(MoveRefusal::OtherSide);
    }
    let id = missions
        .dispatch_guarded(
            MissionRequest {
                kind: MissionKind::Move,
                faction,
                team: members,
                decoys: Vec::new(),
                target_system: system,
                target_character: None,
                target_object: None,
                tick,
            },
            world,
        )
        .map_err(MoveRefusal::Mission)?;
    if let Some(fleet) = fleet {
        missions.set_target_fleet(id, fleet);
    }
    Ok(id)
}

/// Products moved to a system or a fleet there, each on its own (the
/// app's `MoveProducts`); the result of each move.
pub fn move_products(
    world: &mut GameWorld,
    troop_transport: &TroopTransportState,
    products: &[u64],
    system: SystemKey,
    fleet: Option<FleetKey>,
) -> Vec<bool> {
    products
        .iter()
        .map(|&product| {
            rebellion_core::products::move_to_target(
                world,
                product,
                system,
                fleet,
                fleet.map_or(0, |fleet| troop_transport.carried_count(fleet)),
            )
        })
        .collect()
}

/// A character's command order (the app's `OrderCommand`); `None` for an
/// order that names no command.
pub fn order_command(
    world: &mut GameWorld,
    missions: &MissionState,
    character: CharacterKey,
    order: u16,
    is_alliance: bool,
) -> Option<Result<Vec<CharacterKey>, rebellion_core::character_command::Refusal>> {
    let kind = rebellion_core::character_command::CommandKind::of_order(order)?;
    let en_route = missions.is_en_route(MissionMember::Character(character));
    Some(rebellion_core::character_command::give(
        world, character, kind, is_alliance, en_route,
    ))
}

/// A member retired (the app's `Retire`).
///
/// # Errors
/// The retirement's refusal.
pub fn retire(
    world: &mut GameWorld,
    missions: &MissionState,
    member: MissionMember,
    is_alliance: bool,
) -> Result<(), rebellion_core::character_retire::Refusal> {
    let en_route = missions.is_en_route(member);
    match member {
        MissionMember::Character(key) => {
            rebellion_core::character_retire::retire(world, key, is_alliance, en_route)
        }
        MissionMember::SpecialForce(key) => {
            rebellion_core::character_retire::retire_special_force(world, key, is_alliance, en_route)
        }
    }
}

/// The replay form of a Move of members.
///
/// # Errors
/// An object without a stable id.
pub fn move_members_command(
    world: &GameWorld,
    members: &[MissionMember],
    system: SystemKey,
    fleet: Option<FleetKey>,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::MoveMembers {
        members: members
            .iter()
            .map(|&member| member_id(world, member))
            .collect::<Result<_, _>>()?,
        system: system_id(world, system)?,
        fleet: fleet.map(|fleet| fleet_id(world, fleet)).transpose()?,
    })
}

/// The replay form of products' moves.
///
/// # Errors
/// A missing system or an unnamed fleet.
pub fn move_products_command(
    world: &GameWorld,
    products: &[u64],
    system: SystemKey,
    fleet: Option<FleetKey>,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::MoveProducts {
        products: products.to_vec(),
        system: system_id(world, system)?,
        fleet: fleet.map(|fleet| fleet_id(world, fleet)).transpose()?,
    })
}

/// The replay form of a command order.
///
/// # Errors
/// A missing character.
pub fn order_command_command(
    world: &GameWorld,
    character: CharacterKey,
    order: u16,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::OrderCommand {
        character: character_id(world, character)?,
        order_id: order,
    })
}

/// The replay form of a Retire order.
///
/// # Errors
/// An object without a stable id.
pub fn retire_command(world: &GameWorld, member: MissionMember) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::Retire {
        member: member_id(world, member)?,
    })
}

/// The galaxy's part of a system's destruction (the app's
/// `destroy_planet` before its messages): the system is destroyed and
/// what stood there is cleared (`death_star::cleanup_destroyed_system`).
/// Returns the cleanup's effects (characters killed) for the messages.
pub fn destroy_system(
    world: &mut GameWorld,
    system: SystemKey,
    killed: &[CharacterKey],
    manufacturing: &mut ManufacturingState,
    blockade: &mut BlockadeState,
) -> Vec<rebellion_core::effects::GameEffect> {
    if let Some(value) = world.systems.get_mut(system) {
        value.is_destroyed = true;
    }
    let mut effects = Vec::new();
    rebellion_core::death_star::cleanup_destroyed_system(
        world,
        system,
        killed,
        manufacturing,
        blockade,
        &mut effects,
    );
    effects
}

/// The galaxy's part of a side taking a system (the app's
/// `apply_system_occupation` before its message; `FUN_00510d70` ->
/// `FUN_004f6f40`): the system and its facilities go to `winner`, and the
/// other side's characters there are taken captive. Returns whether the
/// holder changed.
pub fn occupy(world: &mut GameWorld, system: SystemKey, winner: rebellion_core::dat::Faction, tick: u64) -> bool {
    use rebellion_core::dat::Faction;
    use rebellion_core::world::ControlKind;
    let previous = world.systems.get(system).map(|value| value.control);
    if let Some(value) = world.systems.get_mut(system) {
        value.control = ControlKind::Controlled(winner);
    }
    world.hand_over_facilities(system);
    for (_, character) in &mut world.characters {
        let is_enemy = match winner {
            Faction::Alliance => character.is_empire,
            Faction::Empire => character.is_alliance,
            Faction::Neutral => false,
        };
        if character.current_system == Some(system) && is_enemy && !character.is_killed {
            character.is_captive = true;
            character.captured_by = Some(winner);
            character.capture_tick = Some(tick);
            character.current_fleet = None;
        }
    }
    previous != Some(ControlKind::Controlled(winner))
}

/// A Planetary Assault's landing settles the system at once (the app's
/// `occupy_system`): when its regiments give it a holder other than its
/// own, that side takes it. Returns the new holder when it changed.
pub fn occupy_after_landing(
    world: &mut GameWorld,
    system: SystemKey,
    tick: u64,
) -> Option<rebellion_core::dat::Faction> {
    use rebellion_core::world::ControlKind;
    let value = world.systems.get(system)?;
    if value.ground_units.is_empty() {
        return None;
    }
    match rebellion_core::economy::system_holder(world, value) {
        ControlKind::Controlled(winner) if value.control != ControlKind::Controlled(winner) => {
            occupy(world, system, winner, tick).then_some(winner)
        }
        _ => None,
    }
}

/// What a Planetary Bombardment did, for the app's messages.
pub struct BombardmentDone {
    pub outcome: rebellion_core::bombardment::BombardmentOutcome,
    pub reports: Vec<rebellion_core::assault_report::AssaultReport>,
    /// The system's name and holder before a destruction, and the
    /// destruction's effects (characters killed); `None` when it stands.
    pub destroyed: Option<(String, Option<rebellion_core::dat::Faction>, Vec<rebellion_core::effects::GameEffect>)>,
}

/// Planetary Bombardment (the app's `PanelAction::OrderBombardment`):
/// when the order is enabled, the bombardment (a stream seeded by the
/// tick), its losses recorded at the systems, and a destroyed system's
/// end. `None` when the order is refused.
#[allow(clippy::too_many_arguments)]
pub fn order_bombardment(
    world: &mut GameWorld,
    movement: &MovementState,
    troop_transport: &mut TroopTransportState,
    blockade: &mut BlockadeState,
    manufacturing: &mut ManufacturingState,
    alliance_hq: SystemKey,
    tick: u64,
    is_alliance: bool,
    fleets: &[FleetKey],
    system: SystemKey,
    kind: u32,
) -> Option<BombardmentDone> {
    if !rebellion_core::bombardment::order_enabled(world, movement, is_alliance, fleets, system, kind) {
        return None;
    }
    let mut bombard = rebellion_core::bombardment::BombardEnv::new(tick, Some(alliance_hq));
    let outcome = bombard
        .bombard_reported(
            world,
            troop_transport,
            rebellion_core::dat::Faction::of_alliance(is_alliance),
            fleets,
            system,
            kind,
        )
        .clone();
    for (system, losses) in &bombard.losses {
        blockade.record_losses(*system, *losses);
    }
    let destroyed = outcome.system_destroyed.then(|| {
        let name = world
            .systems
            .get(system)
            .map_or_else(|| "Unknown".to_string(), |s| s.name.clone());
        let holder = world.systems.get(system).and_then(|s| s.control.faction());
        let effects = destroy_system(world, system, &outcome.killed, manufacturing, blockade);
        (name, holder, effects)
    });
    Some(BombardmentDone {
        outcome,
        reports: std::mem::take(&mut bombard.reports),
        destroyed,
    })
}

/// What a Planetary Assault did, for the app's messages.
pub struct AssaultDone {
    pub outcome: rebellion_core::assault::AssaultOutcome,
    pub reports: Vec<rebellion_core::assault_report::AssaultReport>,
    /// The side that took the system on the assault's day, if one did.
    pub occupied: Option<rebellion_core::dat::Faction>,
}

/// Planetary Assault (the app's `PanelAction::OrderAssault`): when the
/// order is enabled, the assault (a stream seeded by the tick), its losses
/// recorded, and the landing's occupation in the same pass. `None` when
/// the order is refused.
#[allow(clippy::too_many_arguments)]
pub fn order_assault(
    world: &mut GameWorld,
    movement: &MovementState,
    troop_transport: &mut TroopTransportState,
    blockade: &mut BlockadeState,
    alliance_hq: SystemKey,
    tick: u64,
    is_alliance: bool,
    fleets: &[FleetKey],
    system: SystemKey,
) -> Option<AssaultDone> {
    if !rebellion_core::assault::order_enabled(world, movement, troop_transport, is_alliance, fleets, system) {
        return None;
    }
    let mut env = rebellion_core::bombardment::BombardEnv::new(tick, Some(alliance_hq));
    let outcome = env
        .assault(
            world,
            troop_transport,
            rebellion_core::dat::Faction::of_alliance(is_alliance),
            fleets,
            system,
        )
        .clone();
    for (system, losses) in &env.losses {
        blockade.record_losses(*system, *losses);
    }
    let occupied = occupy_after_landing(world, system, tick);
    Some(AssaultDone {
        outcome,
        reports: std::mem::take(&mut env.reports),
        occupied,
    })
}

fn fleet_ids(world: &GameWorld, fleets: &[FleetKey]) -> Result<Vec<u64>, CommandIdError> {
    fleets.iter().map(|&fleet| fleet_id(world, fleet)).collect()
}

fn fleet_keys(world: &GameWorld, fleets: &[u64]) -> Result<Vec<FleetKey>, CommandIdError> {
    fleets.iter().map(|&serial| fleet_key(world, serial)).collect()
}

/// The replay form of a Planetary Bombardment.
///
/// # Errors
/// An unnamed fleet or a missing system.
pub fn order_bombardment_command(
    world: &GameWorld,
    fleets: &[FleetKey],
    system: SystemKey,
    kind: u32,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::OrderBombardment {
        fleets: fleet_ids(world, fleets)?,
        system: system_id(world, system)?,
        kind,
    })
}

/// The replay form of a Planetary Assault.
///
/// # Errors
/// An unnamed fleet or a missing system.
pub fn order_assault_command(
    world: &GameWorld,
    fleets: &[FleetKey],
    system: SystemKey,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::OrderAssault {
        fleets: fleet_ids(world, fleets)?,
        system: system_id(world, system)?,
    })
}

/// The replay form of the headquarters' move.
///
/// # Errors
/// A missing system.
pub fn move_headquarters_command(
    world: &GameWorld,
    destination: SystemKey,
) -> Result<PlayerCommand, CommandIdError> {
    Ok(PlayerCommand::MoveHeadquarters {
        destination: system_id(world, destination)?,
    })
}

/// The replay form of the app's fleet move, from its keys.
///
/// # Errors
/// [`CommandIdError::Unnamed`] when the fleet has no creation serial or a
/// regiment no object id, or a missing system.
pub fn dispatch_fleet_command(
    world: &GameWorld,
    fleet: FleetKey,
    destination: SystemKey,
    troops: &[TroopKey],
) -> Result<PlayerCommand, CommandIdError> {
    let serial = world.fleet_serial(fleet);
    if serial == 0 {
        return Err(CommandIdError::Unnamed);
    }
    let destination = world
        .systems
        .get(destination)
        .map(|system| system.dat_id)
        .ok_or(CommandIdError::Unnamed)?;
    let troops = troops
        .iter()
        .map(|&troop| world.object_views.regiment_id(troop).ok_or(CommandIdError::Unnamed))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PlayerCommand::DispatchFleet {
        fleet: serial,
        destination,
        troops,
    })
}

/// Why a replayed command did not apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    Id(CommandIdError),
    Refused(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(error) => error.fmt(f),
            Self::Refused(why) => write!(f, "the order was refused: {why}"),
        }
    }
}

impl std::error::Error for CommandError {}

impl From<CommandIdError> for CommandError {
    fn from(error: CommandIdError) -> Self {
        Self::Id(error)
    }
}

/// The states a player command may read or change, and the tick it is
/// given on.
pub struct CommandStates<'a> {
    pub movement: &'a mut MovementState,
    pub troop_transport: &'a mut TroopTransportState,
    pub blockade: &'a mut BlockadeState,
    pub victory: &'a mut VictoryState,
    pub manufacturing: &'a mut ManufacturingState,
    pub missions: &'a mut MissionState,
    pub tick: u64,
}

fn faction(is_alliance: bool) -> MissionFaction {
    if is_alliance {
        MissionFaction::Alliance
    } else {
        MissionFaction::Empire
    }
}

fn refused(refusal: impl std::fmt::Debug) -> CommandError {
    CommandError::Refused(format!("{refusal:?}"))
}

/// What follows every player order in the app: an order's units are
/// objects from its fire (`FUN_0052a140`), placed now in their containers
/// (`products::reconcile_carrying`).
pub fn after_order(
    world: &mut GameWorld,
    manufacturing: &ManufacturingState,
    troop_transport: &TroopTransportState,
) {
    rebellion_core::products::reconcile_carrying(world, manufacturing, &|fleet| {
        troop_transport.carried_count(fleet)
    });
}

/// Apply a replayed player command for the side `is_alliance`, as the app
/// does for the same order, then [`after_order`].
///
/// # Errors
/// An id the world does not hold, or the order's refusal (a replay fails
/// closed on either: the recorded session applied it).
pub fn apply(
    world: &mut GameWorld,
    mut states: CommandStates<'_>,
    is_alliance: bool,
    command: &PlayerCommand,
) -> Result<(), CommandError> {
    let result = apply_order(world, &mut states, is_alliance, command);
    after_order(world, states.manufacturing, states.troop_transport);
    result
}

fn apply_order(
    world: &mut GameWorld,
    states: &mut CommandStates<'_>,
    is_alliance: bool,
    command: &PlayerCommand,
) -> Result<(), CommandError> {
    match command {
        PlayerCommand::DispatchFleet {
            fleet,
            destination,
            troops,
        } => {
            let fleet = fleet_key(world, *fleet)?;
            let destination = system_key(world, *destination)?;
            let troops = troops
                .iter()
                .map(|&id| troop_key(world, id))
                .collect::<Result<Vec<_>, _>>()?;
            dispatch_fleet(
                world,
                states.movement,
                states.troop_transport,
                fleet,
                destination,
                &troops,
                is_alliance,
            )
            .map(|_| ())
            .map_err(refused)
        }
        PlayerCommand::MoveRegiment { troop, target } => {
            let troop = troop_key(world, *troop)?;
            let target = match *target {
                PlaceId::System(id) => RegimentTarget::System(system_key(world, id)?),
                PlaceId::Fleet(serial) => RegimentTarget::Fleet(fleet_key(world, serial)?),
            };
            move_regiment(
                world,
                states.troop_transport,
                states.movement,
                states.blockade.blockaded_systems(),
                troop,
                target,
                states.tick,
                is_alliance,
            )
            .map(|_| ())
            .map_err(refused)
        }
        PlayerCommand::MoveSquadron { class, from, to } => {
            let class = fighter_class_key(world, *class)?;
            let from = place_key(world, *from)?;
            let to = place_key(world, *to)?;
            move_squadron(world, states.troop_transport, is_alliance, class, from, to, states.tick)
                .map(|_| ())
                .map_err(refused)
        }
        PlayerCommand::JoinFleet { mover, target } => {
            let mover = mover_key(world, mover)?;
            let target = fleet_key(world, *target)?;
            join_fleet(
                world,
                states.movement,
                states.troop_transport,
                &mover,
                target,
                is_alliance,
            )
            .map(|_: FleetOrderOutcome| ())
            .map_err(|error: FleetOrderError| refused(error))
        }
        PlayerCommand::MoveHeadquarters { destination } => {
            let destination = system_key(world, *destination)?;
            move_headquarters(
                world,
                states.troop_transport,
                states.victory,
                is_alliance,
                destination,
                states.tick,
            )
            .map(|_: HeadquartersMove| ())
            .map_err(|error: HeadquartersMoveError| refused(error))
        }
        PlayerCommand::ToggleReserved { system, area } => {
            let system = system_key(world, *system)?;
            toggle_reserved(states.manufacturing, system, *area);
            Ok(())
        }
        PlayerCommand::StopProduction { system, area } => {
            let system = system_key(world, *system)?;
            states.manufacturing.stop(system, *area);
            Ok(())
        }
        PlayerCommand::BuildProduction {
            system,
            area,
            kind,
            count,
        } => {
            let system = system_key(world, *system)?;
            let kind = build_kind(world, *kind)?;
            if rebellion_core::build_selection::confirm_order(
                world,
                states.manufacturing,
                system,
                *area,
                kind,
                *count,
                is_alliance,
            ) {
                Ok(())
            } else {
                Err(CommandError::Refused("the production order".into()))
            }
        }
        PlayerCommand::SetDestination {
            system,
            area,
            destination,
        } => {
            let system = system_key(world, *system)?;
            let destination = system_key(world, *destination)?;
            set_destination(states.manufacturing, system, *area, destination);
            Ok(())
        }
        PlayerCommand::SetFleetDestination {
            system,
            area,
            fleet,
            at,
            ship,
        } => {
            let system = system_key(world, *system)?;
            let fleet = fleet_key(world, *fleet)?;
            let at = system_key(world, *at)?;
            if set_fleet_destination(states.manufacturing, system, *area, fleet, at, *ship) {
                Ok(())
            } else {
                Err(CommandError::Refused("the fleet Destination".into()))
            }
        }
        PlayerCommand::RenameProduct { product, name } => {
            if rebellion_core::products::rename(world, states.manufacturing, *product, name.clone()) {
                Ok(())
            } else {
                Err(CommandError::Refused("the product's name".into()))
            }
        }
        PlayerCommand::Rename { fleet, ship, name } => {
            let fleet = fleet_key(world, *fleet)?;
            if rename(world, fleet, *ship, name) {
                Ok(())
            } else {
                Err(CommandError::Refused("the name".into()))
            }
        }
        PlayerCommand::CreateFleet {
            fleet,
            ships,
            roster,
        } => {
            let fleet = fleet_key(world, *fleet)?;
            rebellion_core::fleet_join::create_fleet(
                world,
                states.movement,
                states.troop_transport,
                fleet,
                ships,
                *roster,
                is_alliance,
            )
            .map(|_| ())
            .map_err(refused)
        }
        PlayerCommand::MoveShips {
            fleet,
            ships,
            roster,
            system,
        } => {
            let fleet = fleet_key(world, *fleet)?;
            let system = system_key(world, *system)?;
            rebellion_core::fleet_join::move_ships_into_spare(
                world,
                states.movement,
                states.troop_transport,
                fleet,
                ships,
                *roster,
                system,
                is_alliance,
            )
            .map(|_| ())
            .map_err(refused)
        }
        PlayerCommand::OrderBombardment { fleets, system, kind } => {
            let fleets = fleet_keys(world, fleets)?;
            let system = system_key(world, *system)?;
            let alliance_hq = states.victory.alliance_hq;
            order_bombardment(
                world,
                states.movement,
                states.troop_transport,
                states.blockade,
                states.manufacturing,
                alliance_hq,
                states.tick,
                is_alliance,
                &fleets,
                system,
                *kind,
            )
            .map(|_| ())
            .ok_or_else(|| CommandError::Refused("Planetary Bombardment".into()))
        }
        PlayerCommand::OrderAssault { fleets, system } => {
            let fleets = fleet_keys(world, fleets)?;
            let system = system_key(world, *system)?;
            let alliance_hq = states.victory.alliance_hq;
            order_assault(
                world,
                states.movement,
                states.troop_transport,
                states.blockade,
                alliance_hq,
                states.tick,
                is_alliance,
                &fleets,
                system,
            )
            .map(|_| ())
            .ok_or_else(|| CommandError::Refused("Planetary Assault".into()))
        }
        PlayerCommand::MoveMembers {
            members,
            system,
            fleet,
        } => {
            let members = members
                .iter()
                .map(|&member| member_key(world, member))
                .collect::<Result<Vec<_>, _>>()?;
            let system = system_key(world, *system)?;
            let fleet = fleet.map(|serial| fleet_key(world, serial)).transpose()?;
            move_members(
                world,
                states.missions,
                faction(is_alliance),
                members,
                system,
                fleet,
                states.tick,
            )
            .map(|_| ())
            .map_err(refused)
        }
        PlayerCommand::MoveProducts {
            products,
            system,
            fleet,
        } => {
            let system = system_key(world, *system)?;
            let fleet = fleet.map(|serial| fleet_key(world, serial)).transpose()?;
            // The app moves each product on its own and reports nothing.
            move_products(world, states.troop_transport, products, system, fleet);
            Ok(())
        }
        PlayerCommand::OrderCommand { character, order_id } => {
            let character = character_key(world, *character)?;
            let order = *order_id;
            match order_command(world, states.missions, character, order, is_alliance) {
                Some(Ok(_)) => Ok(()),
                Some(Err(refusal)) => Err(refused(refusal)),
                None => Err(CommandError::Refused(format!("order {order:#x} names no command"))),
            }
        }
        PlayerCommand::Retire { member } => {
            let member = member_key(world, *member)?;
            retire(world, states.missions, member, is_alliance).map_err(refused)
        }
        PlayerCommand::DispatchMission {
            kind,
            team,
            decoys,
            target,
            target_character,
            target_object,
        } => {
            let members = |list: &[MemberId]| {
                list.iter()
                    .map(|&member| member_key(world, member))
                    .collect::<Result<Vec<_>, _>>()
            };
            let request = MissionRequest {
                kind: *kind,
                faction: faction(is_alliance),
                team: members(team)?,
                decoys: members(decoys)?,
                target_system: system_key(world, *target)?,
                target_character: target_character
                    .map(|id| character_key(world, id))
                    .transpose()?,
                target_object: target_object.map(|id| target_key(world, id)).transpose()?,
                tick: states.tick,
            };
            dispatch_mission(world, states.missions, request)
                .map(|_| ())
                .map_err(refused)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::{CampaignDayRunner, ReplayActor, ReplayCommand};
    use crate::save::SaveState;

    fn seed42() -> SaveState {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
        let world = crate::load_game_data_with_options(
            &data,
            &crate::replay_fixture::seed42_seed_options(),
        )
        .expect("load original campaign data");
        crate::replay_fixture::seed42_initial_state(world).unwrap()
    }

    /// The first Alliance fleet and a system it may move to.
    fn alliance_move(state: &SaveState) -> (FleetKey, SystemKey) {
        let world = &state.world;
        let mut fleets: Vec<FleetKey> = world
            .fleets
            .iter()
            .filter(|(_, fleet)| fleet.is_alliance && fleet.ship_count() > 0)
            .map(|(key, _)| key)
            .collect();
        fleets.sort_by_key(|&fleet| world.fleet_serial(fleet));
        for fleet in fleets {
            let mut systems: Vec<(DatId, SystemKey)> =
                world.systems.iter().map(|(key, system)| (system.dat_id, key)).collect();
            systems.sort_by_key(|(id, _)| id.0);
            if let Some((_, to)) = systems.into_iter().find(|&(_, to)| {
                validate_fleet_dispatch(&state.movement, world, fleet, to, true).is_ok()
            }) {
                return (fleet, to);
            }
        }
        panic!("no Alliance fleet can move");
    }

    fn advance(runner: &mut CampaignDayRunner) {
        let start = runner.tick();
        runner.apply(&ReplayCommand::AdvanceTicks { count: 10 }).unwrap();
        assert_eq!(runner.tick(), start + 10, "ten days passed");
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_fleet_move_ends_where_the_app_s_move_does() {
        let initial = seed42();
        let (fleet, to) = alliance_move(&initial);

        // The app's path: its keys straight into the shared order.
        let mut app = initial.clone();
        let departure = dispatch_fleet(
            &mut app.world,
            &mut app.movement,
            &mut app.troop_transport,
            fleet,
            to,
            &[],
            true,
        )
        .expect("the move departs");
        assert_eq!(departure.destination, to);
        let command = dispatch_fleet_command(&initial.world, fleet, to, &[]).unwrap();
        let mut app = CampaignDayRunner::new(app);
        advance(&mut app);

        // The replay's path: stable ids, the Alliance as actor.
        let mut replay = CampaignDayRunner::new(initial.clone());
        replay
            .apply_as(ReplayActor::Alliance, &ReplayCommand::Player { command: command.clone() })
            .unwrap();
        advance(&mut replay);

        let mut idle = CampaignDayRunner::new(initial);
        advance(&mut idle);

        assert_eq!(replay.fingerprint().unwrap(), app.fingerprint().unwrap());
        assert_ne!(idle.fingerprint().unwrap(), app.fingerprint().unwrap(), "the move changed the run");
        // The command survives JSON, as a replay file keeps it.
        let json = serde_json::to_string(&ReplayCommand::Player { command: command.clone() }).unwrap();
        assert_eq!(
            serde_json::from_str::<ReplayCommand>(&json).unwrap(),
            ReplayCommand::Player { command }
        );
    }

    /// The first fleet of a side that can move with one of its system's
    /// regiments aboard, and a system it may move to.
    fn move_with_troop(state: &SaveState, is_alliance: bool) -> Option<(FleetKey, SystemKey, TroopKey)> {
        let world = &state.world;
        let mut fleets: Vec<FleetKey> = world
            .fleets
            .iter()
            .filter(|(_, fleet)| fleet.is_alliance == is_alliance && fleet.ship_count() > 0)
            .map(|(key, _)| key)
            .collect();
        fleets.sort_by_key(|&fleet| world.fleet_serial(fleet));
        let mut systems: Vec<(DatId, SystemKey)> =
            world.systems.iter().map(|(key, system)| (system.dat_id, key)).collect();
        systems.sort_by_key(|(id, _)| id.0);
        fleets.into_iter().find_map(|fleet| {
            let at = world.fleets[fleet].location;
            let troop = world.systems[at].ground_units.iter().copied().find(|&troop| {
                world.troops[troop].is_alliance == is_alliance
                    && world.object_views.regiment_id(troop).is_some()
            })?;
            let mut probe = state.clone();
            probe.troop_transport.embark(&mut probe.world, fleet, &[troop]).ok()?;
            let to = systems.iter().map(|&(_, to)| to).find(|&to| {
                validate_fleet_dispatch(&state.movement, world, fleet, to, is_alliance).is_ok()
            })?;
            Some((fleet, to, troop))
        })
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_move_takes_the_same_regiment_aboard() {
        let initial = seed42();
        let (fleet, to, troop) =
            move_with_troop(&initial, true).expect("an Alliance fleet with a regiment to carry");
        let command = dispatch_fleet_command(&initial.world, fleet, to, &[troop]).unwrap();

        let mut app = initial.clone();
        dispatch_fleet(
            &mut app.world,
            &mut app.movement,
            &mut app.troop_transport,
            fleet,
            to,
            &[troop],
            true,
        )
        .expect("the move departs");
        assert_eq!(app.troop_transport.cargo(fleet), &[troop]);
        let mut app = CampaignDayRunner::new(app);

        let mut replay = CampaignDayRunner::new(initial.clone());
        replay
            .apply_as(ReplayActor::Alliance, &ReplayCommand::Player { command })
            .unwrap();
        assert_eq!(replay.fingerprint().unwrap(), app.fingerprint().unwrap());
        let mut bare = CampaignDayRunner::new(initial.clone());
        bare.apply_as(
            ReplayActor::Alliance,
            &ReplayCommand::Player {
                command: dispatch_fleet_command(&initial.world, fleet, to, &[]).unwrap(),
            },
        )
        .unwrap();
        assert_ne!(bare.fingerprint().unwrap(), app.fingerprint().unwrap(), "the regiment went aboard");
        advance(&mut replay);
        advance(&mut app);
        assert_eq!(replay.fingerprint().unwrap(), app.fingerprint().unwrap());
        assert_eq!(app.world().fleets.len(), replay.world().fleets.len());
    }

    /// The order applied the app's way (`app`) and replayed (`command`,
    /// Alliance actor) give one state, then the same state ten days on, and
    /// it differs from the run without the order.
    fn same_as_app(initial: &SaveState, app: impl FnOnce(&mut Live<'_>), command: PlayerCommand) {
        same_as_app_for(ReplayActor::Alliance, initial, app, command);
    }

    /// The app's states as the order code sees them, borrowed from a live
    /// runner.
    struct Live<'a> {
        world: &'a mut GameWorld,
        clock: &'a mut rebellion_core::tick::GameClock,
        manufacturing: &'a mut ManufacturingState,
        missions: &'a mut MissionState,
        movement: &'a mut MovementState,
        troop_transport: &'a mut TroopTransportState,
        blockade: &'a mut BlockadeState,
        victory: &'a mut VictoryState,
    }

    /// Run `app` on a live runner's states, as the app's loop does, then
    /// the order's aftermath. Both paths stay in one live runtime: a slot
    /// a live world reuses differs in version from one a fresh load gives
    /// (a key, not the game).
    fn applied_live(initial: &SaveState, app: impl FnOnce(&mut Live<'_>)) -> CampaignDayRunner {
        let mut runner = CampaignDayRunner::new(initial.clone());
        let (world, states) = runner.parts_mut();
        let mut live = Live {
            world,
            clock: &mut states.clock,
            manufacturing: &mut states.manufacturing,
            missions: &mut states.missions,
            movement: &mut states.movement,
            troop_transport: &mut states.troop_transport,
            blockade: &mut states.blockade,
            victory: &mut states.victory,
        };
        app(&mut live);
        after_order(live.world, live.manufacturing, live.troop_transport);
        runner
    }

    fn same_as_app_for(
        actor: ReplayActor,
        initial: &SaveState,
        app: impl FnOnce(&mut Live<'_>),
        command: PlayerCommand,
    ) {
        let mut applied = applied_live(initial, app);
        let mut replay = CampaignDayRunner::new(initial.clone());
        replay
            .apply_as(actor, &ReplayCommand::Player { command })
            .unwrap();
        let idle = CampaignDayRunner::new(initial.clone());
        assert_eq!(replay.fingerprint().unwrap(), applied.fingerprint().unwrap());
        assert_ne!(idle.fingerprint().unwrap(), applied.fingerprint().unwrap(), "the order did something");
        advance(&mut replay);
        advance(&mut applied);
        assert_eq!(replay.fingerprint().unwrap(), applied.fingerprint().unwrap());
    }

    fn sorted_systems(world: &GameWorld) -> Vec<SystemKey> {
        let mut systems: Vec<(DatId, SystemKey)> =
            world.systems.iter().map(|(key, system)| (system.dat_id, key)).collect();
        systems.sort_by_key(|(id, _)| id.0);
        systems.into_iter().map(|(_, key)| key).collect()
    }

    fn sorted_alliance_fleets(world: &GameWorld) -> Vec<FleetKey> {
        sorted_fleets(world, true)
    }

    fn sorted_fleets(world: &GameWorld, is_alliance: bool) -> Vec<FleetKey> {
        let mut fleets: Vec<FleetKey> = world
            .fleets
            .iter()
            .filter(|(_, fleet)| fleet.is_alliance == is_alliance)
            .map(|(key, _)| key)
            .collect();
        fleets.sort_by_key(|&fleet| world.fleet_serial(fleet));
        fleets
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_regiment_move_matches_the_app_s() {
        let initial = seed42();
        let world = &initial.world;
        let found = sorted_alliance_fleets(world).into_iter().find_map(|fleet| {
            let at = world.fleets[fleet].location;
            world.systems[at].ground_units.iter().copied().find_map(|troop| {
                let mut probe = initial.clone();
                move_regiment(
                    &mut probe.world,
                    &mut probe.troop_transport,
                    &probe.movement,
                    probe.blockade.blockaded_systems(),
                    troop,
                    RegimentTarget::Fleet(fleet),
                    probe.clock.tick,
                    true,
                )
                .ok()?;
                world.object_views.regiment_id(troop)?;
                Some((troop, fleet))
            })
        });
        let (troop, fleet) = found.expect("an Alliance regiment that can board a fleet");
        let command = move_regiment_command(world, troop, RegimentTarget::Fleet(fleet)).unwrap();
        same_as_app(
            &initial,
            |state| {
                move_regiment(
                    &mut state.world,
                    &mut state.troop_transport,
                    &state.movement,
                    state.blockade.blockaded_systems(),
                    troop,
                    RegimentTarget::Fleet(fleet),
                    state.clock.tick,
                    true,
                )
                .unwrap();
            },
            command,
        );
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_squadron_move_matches_the_app_s() {
        // The Empire: at the start only its ships carry squadrons.
        let initial = seed42();
        let world = &initial.world;
        let mut classes: Vec<(DatId, FighterKey)> = world
            .fighter_classes
            .iter()
            .map(|(key, class)| (class.dat_id, key))
            .collect();
        classes.sort_by_key(|(id, _)| id.0);
        let found = sorted_fleets(world, false).into_iter().find_map(|fleet| {
            let at = world.fleets[fleet].location;
            let ways = [
                (SquadronPlace::Planet(at), SquadronPlace::Fleet(fleet)),
                (SquadronPlace::Fleet(fleet), SquadronPlace::Planet(at)),
            ];
            ways.into_iter().find_map(|(from, to)| {
                classes.iter().find_map(|&(_, class)| {
                    let mut probe = initial.clone();
                    move_squadron(&mut probe.world, &mut probe.troop_transport, false, class, from, to, 0)
                        .ok()?;
                    Some((class, from, to))
                })
            })
        });
        let (class, from, to) = found.expect("an Imperial squadron that can move");
        let command = move_squadron_command(world, class, from, to).unwrap();
        same_as_app_for(
            ReplayActor::Empire,
            &initial,
            |state| {
                let tick = state.clock.tick;
                move_squadron(&mut state.world, &mut state.troop_transport, false, class, from, to, tick)
                    .unwrap();
            },
            command,
        );
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_fleet_join_matches_the_app_s() {
        let initial = seed42();
        let world = &initial.world;
        let fleets = sorted_alliance_fleets(world);
        let found = fleets.iter().find_map(|&mover| {
            fleets.iter().copied().filter(|&target| target != mover).find_map(|target| {
                let mut probe = initial.clone();
                join_fleet(
                    &mut probe.world,
                    &mut probe.movement,
                    &mut probe.troop_transport,
                    &FleetMover::Fleet(mover),
                    target,
                    true,
                )
                .ok()?;
                Some((mover, target))
            })
        });
        let (mover, target) = found.expect("two Alliance fleets that can join");
        let command = join_fleet_command(world, &FleetMover::Fleet(mover), target).unwrap();
        same_as_app(
            &initial,
            |state| {
                join_fleet(
                    &mut state.world,
                    &mut state.movement,
                    &mut state.troop_transport,
                    &FleetMover::Fleet(mover),
                    target,
                    true,
                )
                .unwrap();
            },
            command,
        );
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_headquarters_move_matches_the_app_s() {
        let initial = seed42();
        let world = &initial.world;
        let found = sorted_systems(world).into_iter().find(|&to| {
            let mut probe = initial.clone();
            matches!(
                move_headquarters(
                    &mut probe.world,
                    &mut probe.troop_transport,
                    &mut probe.victory,
                    true,
                    to,
                    probe.clock.tick,
                ),
                Ok(HeadquartersMove::Departed(_))
            )
        });
        let to = found.expect("a system the Alliance headquarters can move to");
        let command = move_headquarters_command(world, to).unwrap();
        same_as_app(
            &initial,
            |state| {
                let tick = state.clock.tick;
                move_headquarters(
                    &mut state.world,
                    &mut state.troop_transport,
                    &mut state.victory,
                    true,
                    to,
                    tick,
                )
                .unwrap();
            },
            command,
        );
    }

    /// An Alliance capital ship order that queues.
    fn alliance_ship_build(initial: &SaveState) -> (SystemKey, ProductionArea, BuildableKind) {
        alliance_build_of(initial, |kind| matches!(kind, BuildableKind::CapitalShip(_)))
    }

    /// An Alliance production order that queues: system, area, kind.
    fn alliance_build(initial: &SaveState) -> (SystemKey, ProductionArea, BuildableKind) {
        alliance_build_of(initial, |_| true)
    }

    fn alliance_build_of(
        initial: &SaveState,
        wanted: impl Fn(BuildableKind) -> bool,
    ) -> (SystemKey, ProductionArea, BuildableKind) {
        let world = &initial.world;
        let mut kinds: Vec<(u32, BuildableKind)> = world
            .capital_ship_classes
            .iter()
            .map(|(key, class)| (class.dat_id.0, BuildableKind::CapitalShip(key)))
            .chain(
                world
                    .fighter_classes
                    .iter()
                    .map(|(key, class)| (class.dat_id.0, BuildableKind::Fighter(key))),
            )
            .collect();
        kinds.sort_by_key(|(id, _)| *id);
        kinds.retain(|&(_, kind)| wanted(kind));
        let areas = [
            ProductionArea::Shipyard,
            ProductionArea::TrainingFacility,
            ProductionArea::ConstructionYard,
        ];
        sorted_systems(world)
            .into_iter()
            .find_map(|system| {
                areas.iter().find_map(|&area| {
                    kinds.iter().find_map(|&(_, kind)| {
                        let mut probe = initial.clone();
                        rebellion_core::build_selection::confirm_order(
                            &mut probe.world,
                            &mut probe.manufacturing,
                            system,
                            area,
                            kind,
                            1,
                            true,
                        )
                        .then_some((system, area, kind))
                    })
                })
            })
            .expect("an Alliance production order that queues")
    }

    fn build(
        world: &mut GameWorld,
        manufacturing: &mut ManufacturingState,
        (system, area, kind): (SystemKey, ProductionArea, BuildableKind),
    ) {
        assert!(rebellion_core::build_selection::confirm_order(
            world,
            manufacturing,
            system,
            area,
            kind,
            1,
            true,
        ));
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_production_order_matches_the_app_s() {
        let initial = seed42();
        let order = alliance_build(&initial);
        let command =
            build_production_command(&initial.world, order.0, order.1, order.2, 1).unwrap();
        same_as_app(&initial, |state| build(state.world, state.manufacturing, order), command);
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_reserved_toggle_and_stop_match_the_app_s() {
        let initial = seed42();
        let (system, area, kind) = alliance_build(&initial);
        same_as_app(
            &initial,
            |state| {
                assert!(toggle_reserved(&mut state.manufacturing, system, Some(area)));
            },
            toggle_reserved_command(&initial.world, system, Some(area)).unwrap(),
        );
        same_as_app(
            &initial,
            |state| {
                toggle_reserved(&mut state.manufacturing, system, None);
            },
            toggle_reserved_command(&initial.world, system, None).unwrap(),
        );
        // Stop what an order queued.
        let mut queued = initial.clone();
        build(&mut queued.world, &mut queued.manufacturing, (system, area, kind));
        same_as_app(
            &queued,
            |state| state.manufacturing.stop(system, area),
            stop_production_command(&initial.world, system, area).unwrap(),
        );
    }

    /// The first Alliance character (by DatId) that a mission of `kind`
    /// to some system (by DatId) accepts.
    fn alliance_mission(initial: &SaveState, kind: MissionKind) -> Option<MissionRequest> {
        let world = &initial.world;
        let mut characters: Vec<(DatId, CharacterKey)> = world
            .characters
            .iter()
            .filter(|(_, c)| c.is_alliance)
            .map(|(key, c)| (c.dat_id, key))
            .collect();
        characters.sort_by_key(|(id, _)| id.0);
        let systems = sorted_systems(world);
        characters.iter().find_map(|&(_, character)| {
            systems.iter().find_map(|&system| {
                let request = MissionRequest::single(
                    kind,
                    MissionFaction::Alliance,
                    character,
                    system,
                    None,
                    initial.clock.tick,
                );
                let mut probe = initial.clone();
                dispatch_mission(&mut probe.world, &mut probe.missions, request.clone()).ok()?;
                Some(request)
            })
        })
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_mission_matches_the_app_s() {
        let initial = seed42();
        for kind in [MissionKind::Diplomacy, MissionKind::Espionage] {
            let request = alliance_mission(&initial, kind).expect("an Alliance mission that goes");
            let command = dispatch_mission_command(&initial.world, &request).unwrap();
            same_as_app(
                &initial,
                |state| {
                    dispatch_mission(&mut state.world, &mut state.missions, request).unwrap();
                },
                command,
            );
        }
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn mission_targets_and_members_keep_their_ids() {
        let initial = seed42();
        let world = &initial.world;
        // Every facility, regiment and special force at a system names
        // itself back.
        for system in world.systems.values() {
            for &key in &system.defense_facilities {
                let id = target_id(world, MissionTarget::DefenseFacility(key)).unwrap();
                assert_eq!(target_key(world, id).unwrap(), MissionTarget::DefenseFacility(key));
            }
            for &key in &system.manufacturing_facilities {
                let id = target_id(world, MissionTarget::ManufacturingFacility(key)).unwrap();
                assert_eq!(target_key(world, id).unwrap(), MissionTarget::ManufacturingFacility(key));
            }
            for &key in &system.production_facilities {
                let id = target_id(world, MissionTarget::ProductionFacility(key)).unwrap();
                assert_eq!(target_key(world, id).unwrap(), MissionTarget::ProductionFacility(key));
            }
            for &key in &system.special_forces {
                let member = member_id(world, MissionMember::SpecialForce(key)).unwrap();
                assert_eq!(member_key(world, member).unwrap(), MissionMember::SpecialForce(key));
                let id = target_id(world, MissionTarget::SpecialForce(key)).unwrap();
                assert_eq!(target_key(world, id).unwrap(), MissionTarget::SpecialForce(key));
            }
            for &key in &system.ground_units {
                if world.object_views.regiment_id(key).is_some() {
                    let id = target_id(world, MissionTarget::Troop(key)).unwrap();
                    assert_eq!(target_key(world, id).unwrap(), MissionTarget::Troop(key));
                }
            }
        }
        for (fleet, value) in &world.fleets {
            for &key in &value.special_forces {
                let member = member_id(world, MissionMember::SpecialForce(key)).unwrap();
                assert_eq!(member_key(world, member).unwrap(), MissionMember::SpecialForce(key));
            }
            for ship in &value.capital_ships {
                let target = MissionTarget::CapitalShip { fleet, class: ship.class };
                assert_eq!(target_key(world, target_id(world, target).unwrap()).unwrap(), target);
            }
            for entry in &value.fighters {
                let target = MissionTarget::Fighter { fleet, class: entry.class };
                assert_eq!(target_key(world, target_id(world, target).unwrap()).unwrap(), target);
            }
            let target = MissionTarget::DeathStar(fleet);
            assert_eq!(target_key(world, target_id(world, target).unwrap()).unwrap(), target);
        }
        for (key, character) in &world.characters {
            let member = member_id(world, MissionMember::Character(key)).unwrap();
            assert_eq!(member, MemberId::Character(character.dat_id));
            assert_eq!(member_key(world, member).unwrap(), MissionMember::Character(key));
        }
        // A slot past the list's end names nothing.
        let system = world.systems.values().next().unwrap().dat_id;
        assert_eq!(
            target_key(world, TargetId::DefenseFacility { system, index: usize::MAX }),
            Err(CommandIdError::Slot)
        );
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn replayed_destinations_match_the_app_s() {
        let initial = seed42();
        let world = &initial.world;
        let (system, area, _) = alliance_build(&initial);
        let other = sorted_systems(world)
            .into_iter()
            .find(|&to| to != system)
            .unwrap();
        same_as_app(
            &initial,
            |state| {
                set_destination(&mut state.manufacturing, system, Some(area), other);
            },
            set_destination_command(world, system, Some(area), other).unwrap(),
        );
        same_as_app(
            &initial,
            |state| {
                assert_eq!(set_destination(&mut state.manufacturing, system, None, other), 3);
            },
            set_destination_command(world, system, None, other).unwrap(),
        );
        // A fleet as the Destination of an area that takes one.
        let found = sorted_systems(world).into_iter().find_map(|yard| {
            [ProductionArea::Shipyard, ProductionArea::TrainingFacility]
                .into_iter()
                .find_map(|area| {
                    sorted_alliance_fleets(world).into_iter().find_map(|fleet| {
                        let at = world.fleets[fleet].location;
                        let mut probe = initial.clone();
                        set_fleet_destination(&mut probe.manufacturing, yard, area, fleet, at, None)
                            .then_some((yard, area, fleet, at))
                    })
                })
        });
        let (yard, area, fleet, at) = found.expect("an area that takes an Alliance fleet");

        same_as_app(
            &initial,
            |state| {
                assert!(set_fleet_destination(&mut state.manufacturing, yard, area, fleet, at, None));
            },
            set_fleet_destination_command(world, yard, area, fleet, at, None).unwrap(),
        );
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn replayed_renames_match_the_app_s() {
        let initial = seed42();
        let world = &initial.world;
        let fleet = sorted_alliance_fleets(world)
            .into_iter()
            .find(|&fleet| world.fleets[fleet].ship_count() > 0)
            .unwrap();
        same_as_app(
            &initial,
            |state| assert!(rename(&mut state.world, fleet, None, "Red Squadron")),
            rename_command(world, fleet, None, "Red Squadron").unwrap(),
        );
        same_as_app(
            &initial,
            |state| assert!(rename(&mut state.world, fleet, Some(0), "Home One")),
            rename_command(world, fleet, Some(0), "Home One").unwrap(),
        );
        // A product renamed by its id: products appear once production
        // runs, so the replay orders a ship, runs a day, then renames it
        // through products::rename, the function the app calls.
        let (system, area, kind) = alliance_ship_build(&initial);
        let mut runner = CampaignDayRunner::new(initial.clone());
        let order = build_production_command(world, system, area, kind, 1).unwrap();
        runner
            .apply_as(ReplayActor::Alliance, &ReplayCommand::Player { command: order })
            .unwrap();
        runner.apply(&ReplayCommand::AdvanceTicks { count: 1 }).unwrap();
        let product = runner
            .world()
            .products
            .all()
            .iter()
            .map(|p| p.id)
            .max()
            .expect("the ship order made a product");
        let rename_it = ReplayCommand::Player {
            command: PlayerCommand::RenameProduct { product, name: "Liberty".into() },
        };
        runner.apply_as(ReplayActor::Alliance, &rename_it).unwrap();
        assert_eq!(
            runner.world().products.get(product).and_then(|p| p.name.as_deref()),
            Some("Liberty")
        );
        let missing = ReplayCommand::Player {
            command: PlayerCommand::RenameProduct { product: u64::MAX, name: "x".into() },
        };
        assert!(runner.apply_as(ReplayActor::Alliance, &missing).is_err());
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn replayed_ship_orders_match_the_app_s() {
        // No fleet opens with two ships: join two Alliance fleets first
        // (a replayed join matches the app's, above).
        let start = seed42();
        let fleets = sorted_alliance_fleets(&start.world);
        let (mover, fleet) = fleets
            .iter()
            .find_map(|&mover| {
                fleets.iter().copied().filter(|&t| t != mover).find_map(|target| {
                    let mut probe = start.clone();
                    join_fleet(
                        &mut probe.world,
                        &mut probe.movement,
                        &mut probe.troop_transport,
                        &FleetMover::Fleet(mover),
                        target,
                        true,
                    )
                    .ok()
                    .map(|_| (mover, target))
                })
            })
            .expect("two Alliance fleets that can join");
        // The mover flies to its target and joins it on arrival.
        let mut runner = CampaignDayRunner::new(start.clone());
        runner
            .apply_as(
                ReplayActor::Alliance,
                &ReplayCommand::Player {
                    command: join_fleet_command(&start.world, &FleetMover::Fleet(mover), fleet)
                        .unwrap(),
                },
            )
            .unwrap();
        for _ in 0..120 {
            if runner.world().fleets.get(fleet).is_some_and(|f| f.ship_count() > 1) {
                break;
            }
            runner.apply(&ReplayCommand::AdvanceTicks { count: 1 }).unwrap();
        }
        let initial = runner.state();
        let (side, actor) = (true, ReplayActor::Alliance);
        let world = &initial.world;
        assert!(world.fleets[fleet].ship_count() > 1);
        let roster = rebellion_core::fleet_join::roster(world, fleet).unwrap();
        same_as_app_for(
            actor,
            &initial,
            |state| {
                rebellion_core::fleet_join::create_fleet(
                    &mut state.world,
                    &state.movement,
                    &mut state.troop_transport,
                    fleet,
                    &[0],
                    roster,
                    side,
                )
                .unwrap();
            },
            create_fleet_command(world, fleet, &[0], roster).unwrap(),
        );
        // Ships moved on into the spare fleet at a system they may reach.
        let found = sorted_systems(world).into_iter().find(|&to| {
            let mut probe = initial.clone();
            rebellion_core::fleet_join::move_ships_into_spare(
                &mut probe.world,
                &mut probe.movement,
                &mut probe.troop_transport,
                fleet,
                &[0],
                roster,
                to,
                side,
            )
            .is_ok()
        });
        let to = found.expect("a system the ships may move to");
        same_as_app_for(
            actor,
            &initial,
            |state| {
                rebellion_core::fleet_join::move_ships_into_spare(
                    &mut state.world,
                    &mut state.movement,
                    &mut state.troop_transport,
                    fleet,
                    &[0],
                    roster,
                    to,
                    side,
                )
                .unwrap();
            },
            move_ships_command(world, fleet, &[0], roster, to).unwrap(),
        );
    }

    fn sorted_alliance_characters(world: &GameWorld) -> Vec<CharacterKey> {
        let mut characters: Vec<(DatId, CharacterKey)> = world
            .characters
            .iter()
            .filter(|(_, c)| c.is_alliance)
            .map(|(key, c)| (c.dat_id, key))
            .collect();
        characters.sort_by_key(|(id, _)| id.0);
        characters.into_iter().map(|(_, key)| key).collect()
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_move_of_members_matches_the_app_s() {
        let initial = seed42();
        let world = &initial.world;
        let systems = sorted_systems(world);
        let found = sorted_alliance_characters(world).into_iter().find_map(|character| {
            systems.iter().find_map(|&to| {
                let mut probe = initial.clone();
                move_members(
                    &mut probe.world,
                    &mut probe.missions,
                    MissionFaction::Alliance,
                    vec![MissionMember::Character(character)],
                    to,
                    None,
                    probe.clock.tick,
                )
                .ok()?;
                Some((character, to))
            })
        });
        let (character, to) = found.expect("an Alliance character that can move");
        let members = [MissionMember::Character(character)];
        same_as_app(
            &initial,
            |state| {
                let tick = state.clock.tick;
                move_members(
                    &mut state.world,
                    &mut state.missions,
                    MissionFaction::Alliance,
                    members.to_vec(),
                    to,
                    None,
                    tick,
                )
                .unwrap();
            },
            move_members_command(world, &members, to, None).unwrap(),
        );
        // A place of the other side is refused before any mission.
        let imperial = systems
            .iter()
            .copied()
            .find(|&to| {
                !rebellion_core::missions::move_place_allowed(world, MissionFaction::Alliance, to, None)
            })
            .expect("a system closed to the Alliance");
        let mut probe = initial.clone();
        assert_eq!(
            move_members(
                &mut probe.world,
                &mut probe.missions,
                MissionFaction::Alliance,
                members.to_vec(),
                imperial,
                None,
                0,
            ),
            Err(MoveRefusal::OtherSide)
        );
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn replayed_command_and_retire_orders_match_the_app_s() {
        let initial = seed42();
        let world = &initial.world;
        let characters = sorted_alliance_characters(world);
        // FUN_0053ebb0: orders 0x260..0x263 name the commands.
        let found = characters.iter().find_map(|&character| {
            (0x261..=0x263).find_map(|order| {
                let mut probe = initial.clone();
                order_command(&mut probe.world, &probe.missions, character, order, true)?.ok()?;
                Some((character, order))
            })
        });
        let (character, order) = found.expect("an Alliance character who takes a command");
        same_as_app(
            &initial,
            |state| {
                order_command(&mut state.world, &state.missions, character, order, true)
                    .unwrap()
                    .unwrap();
            },
            order_command_command(world, character, order).unwrap(),
        );
        let mut probe = initial.clone();
        assert!(order_command(&mut probe.world, &probe.missions, character, 0x100, true).is_none());
        let found = characters.iter().copied().find(|&character| {
            let mut probe = initial.clone();
            retire(&mut probe.world, &probe.missions, MissionMember::Character(character), true).is_ok()
        });
        let retiring = found.expect("an Alliance character who may retire");
        same_as_app(
            &initial,
            |state| {
                retire(&mut state.world, &state.missions, MissionMember::Character(retiring), true)
                    .unwrap();
            },
            retire_command(world, MissionMember::Character(retiring)).unwrap(),
        );
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_product_move_takes_the_app_s_function() {
        // A product appears once production runs: the replay orders a ship,
        // runs a day and moves the product to another of its side's
        // systems, as products::move_to_target does for the app.
        let initial = seed42();
        let world = &initial.world;
        let (system, area, kind) = alliance_ship_build(&initial);
        let mut runner = CampaignDayRunner::new(initial.clone());
        runner
            .apply_as(
                ReplayActor::Alliance,
                &ReplayCommand::Player {
                    command: build_production_command(world, system, area, kind, 1).unwrap(),
                },
            )
            .unwrap();
        runner.apply(&ReplayCommand::AdvanceTicks { count: 1 }).unwrap();
        let state = runner.state();
        // The side's own product: the order made it (the AI orders too).
        let product = state
            .world
            .products
            .all()
            .iter()
            .filter(|p| p.is_alliance)
            .map(|p| p.id)
            .max()
            .expect("the Alliance's product");
        let found = sorted_systems(&state.world).into_iter().find(|&to| {
            let mut probe = state.clone();
            move_products(&mut probe.world, &probe.troop_transport, &[product], to, None)[0]
        });
        let to = found.expect("a system the product may go to");
        let command = move_products_command(&state.world, &[product], to, None).unwrap();
        // Both paths in one live runner: the move stages and drops a fleet,
        // and a slot a live world reuses differs in version from the slot a
        // state loaded afresh gives (a key, not the game).
        let mut app = CampaignDayRunner::new(state.clone());
        assert_eq!(
            move_products(app.world_mut(), &TroopTransportState::default(), &[product], to, None),
            [true]
        );
        let mut replay = CampaignDayRunner::new(state.clone());
        replay
            .apply_as(ReplayActor::Alliance, &ReplayCommand::Player { command })
            .unwrap();
        let idle = CampaignDayRunner::new(state);
        assert_eq!(replay.fingerprint().unwrap(), app.fingerprint().unwrap());
        assert_ne!(idle.fingerprint().unwrap(), app.fingerprint().unwrap());
        advance(&mut replay);
        advance(&mut app);
        assert_eq!(replay.fingerprint().unwrap(), app.fingerprint().unwrap());
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_bombardment_matches_the_app_s() {
        // A0 menus/13, 15: a side may bombard its own system, so a fleet at
        // home will do; the strike's stream is seeded by the day.
        let initial = seed42();
        let world = &initial.world;
        let found = [false, true].into_iter().find_map(|side| {
            sorted_fleets(world, side).into_iter().find_map(|fleet| {
                let at = world.fleets[fleet].location;
                [rebellion_core::bombardment::TARGET_MILITARY, rebellion_core::bombardment::TARGET_CIVILIAN]
                    .into_iter()
                    .find(|&kind| {
                        // One that changes the galaxy (a strike can miss
                        // every target).
                        let mut probe = initial.clone();
                        let before = CampaignDayRunner::new(probe.clone()).fingerprint().unwrap();
                        let hq = probe.victory.alliance_hq;
                        let tick = probe.clock.tick;
                        order_bombardment(
                            &mut probe.world,
                            &probe.movement,
                            &mut probe.troop_transport,
                            &mut probe.blockade,
                            &mut probe.manufacturing,
                            hq,
                            tick,
                            side,
                            &[fleet],
                            at,
                            kind,
                        )
                        .is_some()
                            && CampaignDayRunner::new(probe).fingerprint().unwrap() != before
                    })
                    .map(|kind| (side, fleet, at, kind))
            })
        });
        let (side, fleet, at, kind) = found.expect("a fleet that may bombard its system");
        let actor = if side { ReplayActor::Alliance } else { ReplayActor::Empire };
        let command = order_bombardment_command(world, &[fleet], at, kind).unwrap();
        same_as_app_for(
            actor,
            &initial,
            |state| {
                let hq = state.victory.alliance_hq;
                let tick = state.clock.tick;
                order_bombardment(
                    state.world,
                    state.movement,
                    state.troop_transport,
                    state.blockade,
                    state.manufacturing,
                    hq,
                    tick,
                    side,
                    &[fleet],
                    at,
                    kind,
                )
                .expect("the bombardment goes");
            },
            command,
        );
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_replayed_assault_matches_the_app_s() {
        // An Alliance fleet takes a regiment aboard and flies to a neutral
        // system (replayed orders that match the app's, above), then
        // assaults it.
        let start = seed42();
        let world = &start.world;
        let side = true;
        let systems = sorted_systems(world);
        let plan = sorted_alliance_fleets(world).into_iter().find_map(|fleet| {
            let at = world.fleets[fleet].location;
            let troop = world.systems[at].ground_units.iter().copied().find(|&troop| {
                if !world.troops[troop].is_alliance || world.object_views.regiment_id(troop).is_none() {
                    return false;
                }
                let mut probe = start.clone();
                move_regiment(
                    &mut probe.world,
                    &mut probe.troop_transport,
                    &probe.movement,
                    probe.blockade.blockaded_systems(),
                    troop,
                    RegimentTarget::Fleet(fleet),
                    probe.clock.tick,
                    true,
                )
                .is_ok()
            })?;
            let to = systems.iter().copied().find(|&to| {
                world.systems[to].control == rebellion_core::world::ControlKind::Uncontrolled
                    && validate_fleet_dispatch(&start.movement, world, fleet, to, true).is_ok()
            })?;
            Some((fleet, troop, to))
        });
        let (fleet, troop, at) = plan.expect("an Alliance fleet, a regiment and a neutral system");
        let mut runner = CampaignDayRunner::new(start.clone());
        for command in [
            move_regiment_command(world, troop, RegimentTarget::Fleet(fleet)).unwrap(),
            dispatch_fleet_command(world, fleet, at, &[]).unwrap(),
        ] {
            runner
                .apply_as(ReplayActor::Alliance, &ReplayCommand::Player { command })
                .unwrap();
        }
        for _ in 0..120 {
            let (world, states) = runner.parts_mut();
            if rebellion_core::assault::order_enabled(
                world,
                &states.movement,
                &states.troop_transport,
                true,
                &[fleet],
                at,
            ) {
                break;
            }
            runner.apply(&ReplayCommand::AdvanceTicks { count: 1 }).unwrap();
        }
        let initial = runner.state();
        let actor = if side { ReplayActor::Alliance } else { ReplayActor::Empire };
        let command = order_assault_command(&initial.world, &[fleet], at).unwrap();
        same_as_app_for(
            actor,
            &initial,
            |state| {
                let hq = state.victory.alliance_hq;
                let tick = state.clock.tick;
                order_assault(
                    state.world,
                    state.movement,
                    state.troop_transport,
                    state.blockade,
                    hq,
                    tick,
                    side,
                    &[fleet],
                    at,
                )
                .expect("the assault goes");
            },
            command,
        );
    }

    #[test]
    fn a_command_error_says_what_it_missed() {
        assert_eq!(CommandIdError::Fleet(7).to_string(), "no fleet with creation serial 7");
        assert_eq!(CommandIdError::System(DatId(0x9000_0109)).to_string(), "no system 0x90000109");
        assert_eq!(CommandIdError::Regiment(DatId(0x1000_0002)).to_string(), "no regiment 0x10000002");
        assert_eq!(CommandIdError::FighterClass(DatId(0x1c00_0001)).to_string(), "no fighter class 0x1c000001");
        assert_eq!(CommandIdError::ShipClass(DatId(0x1400_0040)).to_string(), "no capital ship class 0x14000040");
        assert_eq!(CommandIdError::Character(DatId(0x3000_0240)).to_string(), "no character 0x30000240");
        assert_eq!(CommandIdError::Slot.to_string(), "no object in that slot");
        assert_eq!(CommandIdError::Unnamed.to_string(), "an object has no stable id");
        assert_eq!(CommandError::from(CommandIdError::Unnamed).to_string(), "an object has no stable id");
        assert_eq!(CommandError::Refused("x".into()).to_string(), "the order was refused: x");
    }

    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_player_order_needs_a_side_and_a_known_fleet() {
        let initial = seed42();
        let (fleet, to) = alliance_move(&initial);
        let command = dispatch_fleet_command(&initial.world, fleet, to, &[]).unwrap();
        // An Empire fleet's move: the engine may not give it, the Empire may.
        let (empire_fleet, empire_to, _) =
            move_with_troop(&initial, false).expect("an Empire fleet that can move");
        let empire_order = ReplayCommand::Player {
            command: dispatch_fleet_command(&initial.world, empire_fleet, empire_to, &[]).unwrap(),
        };
        let mut runner = CampaignDayRunner::new(initial);
        assert!(
            runner.apply_as(ReplayActor::Engine, &empire_order).is_err(),
            "the engine gives no orders"
        );
        let order = ReplayCommand::Player { command };
        // The Empire may not move an Alliance fleet.
        assert!(runner.apply_as(ReplayActor::Empire, &order).is_err());
        let unknown = ReplayCommand::Player {
            command: PlayerCommand::DispatchFleet {
                fleet: u64::MAX,
                destination: DatId(0),
                troops: Vec::new(),
            },
        };
        assert!(runner.apply_as(ReplayActor::Alliance, &unknown).is_err());
    }
}
