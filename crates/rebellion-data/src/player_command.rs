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

use rebellion_core::ids::{DatId, FleetKey, SystemKey, TroopKey};
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
}

/// Why a command could not name its objects in this world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandIdError {
    Fleet(u64),
    System(DatId),
    Regiment(DatId),
    Unnamed,
}

impl std::fmt::Display for CommandIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fleet(serial) => write!(f, "no fleet with creation serial {serial}"),
            Self::System(id) => write!(f, "no system {:#010x}", id.0),
            Self::Regiment(id) => write!(f, "no regiment {:#010x}", id.0),
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

/// The states a player command may change.
pub struct CommandStates<'a> {
    pub movement: &'a mut MovementState,
    pub troop_transport: &'a mut TroopTransportState,
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
            .map_err(|refusal| CommandError::Refused(format!("{refusal:?}")))
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

    #[test]
    fn a_command_error_says_what_it_missed() {
        assert_eq!(CommandIdError::Fleet(7).to_string(), "no fleet with creation serial 7");
        assert_eq!(CommandIdError::System(DatId(0x9000_0109)).to_string(), "no system 0x90000109");
        assert_eq!(CommandIdError::Regiment(DatId(0x1000_0002)).to_string(), "no regiment 0x10000002");
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
