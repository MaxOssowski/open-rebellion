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
use rebellion_core::ids::{DatId, FighterKey, FleetKey, SystemKey, TroopKey};
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
    Unnamed,
}

impl std::fmt::Display for CommandIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fleet(serial) => write!(f, "no fleet with creation serial {serial}"),
            Self::System(id) => write!(f, "no system {:#010x}", id.0),
            Self::Regiment(id) => write!(f, "no regiment {:#010x}", id.0),
            Self::FighterClass(id) => write!(f, "no fighter class {:#010x}", id.0),
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
    pub blockade: &'a BlockadeState,
    pub victory: &'a mut VictoryState,
    pub tick: u64,
}

fn refused(refusal: impl std::fmt::Debug) -> CommandError {
    CommandError::Refused(format!("{refusal:?}"))
}

/// Apply a replayed player command for the side `is_alliance`, as the app
/// does for the same order.
///
/// # Errors
/// An id the world does not hold, or the order's refusal (a replay fails
/// closed on either: the recorded session applied it).
pub fn apply(
    world: &mut GameWorld,
    states: CommandStates<'_>,
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
    fn same_as_app(initial: &SaveState, app: impl FnOnce(&mut SaveState), command: PlayerCommand) {
        same_as_app_for(ReplayActor::Alliance, initial, app, command);
    }

    fn same_as_app_for(
        actor: ReplayActor,
        initial: &SaveState,
        app: impl FnOnce(&mut SaveState),
        command: PlayerCommand,
    ) {
        let mut applied = initial.clone();
        app(&mut applied);
        let mut applied = CampaignDayRunner::new(applied);
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

    #[test]
    fn a_command_error_says_what_it_missed() {
        assert_eq!(CommandIdError::Fleet(7).to_string(), "no fleet with creation serial 7");
        assert_eq!(CommandIdError::System(DatId(0x9000_0109)).to_string(), "no system 0x90000109");
        assert_eq!(CommandIdError::Regiment(DatId(0x1000_0002)).to_string(), "no regiment 0x10000002");
        assert_eq!(CommandIdError::FighterClass(DatId(0x1c00_0001)).to_string(), "no fighter class 0x1c000001");
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
