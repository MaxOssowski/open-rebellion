//! Rule-based AI manager for the computer-controlled faction.
//!
//! The AI makes [`passes_per_day`] passes a game-day and each pass produces
//! a list of `AIAction` recommendations. The caller applies those actions to
//! `ManufacturingState`, `MissionState`, and `GameWorld` before the next
//! pass.
//!
//! # Design
//!
//! Follows the same stateless pattern as manufacturing and mission systems:
//! - `AIState` holds per-faction persistent data (cooldowns, assignments)
//! - `AISystem::pass(state, world, mfg, missions, ..) -> Vec<AIAction>`
//! - Actions are recommendations only — the caller decides whether to apply them
//!
//! # Heuristics (ported from rebellion2's AIManager.cs)
//!
//! 1. **Officer assignment**: for each available (unassigned) character —
//!    - If major character AND unrecruited officers exist → recruitment mission
//!    - Else if major OR diplomacy > 60 → diplomacy mission targeting lowest-popularity system
//! 2. **Production — fighters**: if a system has idle manufacturing capacity,
//!    enqueue the best available fighter class for fleets with excess carrier slots
//! 3. **Production — facilities**: if a system has fewer than `MAX_CONSTRUCTION_YARDS`
//!    construction yards and has free slots, enqueue another
//!
//! # Cadence
//!
//! The AI player's tick (`FUN_00485b40`) runs once a sub-tick, and a day has
//! as many sub-ticks as the game speed's rate (`FUN_0051dea0`, side
//! `+0xc4`, set by the speed order `0x280`): 600 Very Slow, 60 Slow, 12
//! Medium, 4 Fast. Each pass refreshes the records, readies every module,
//! and runs the modules in turn until one issues orders, no module is ready,
//! or the day's [`ORDERS_PER_DAY`] are spent ([`AISystem::pass`],
//! `ghidra/notes/ai-mission-planning.md`, "Cadence").

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::ids::{
    CapitalShipKey, CharacterKey, DatId, FighterKey, FleetKey, ManufacturingFacilityKey, SystemKey,
    TroopKey,
};
use crate::manufacturing::{BuildableKind, FacilityBuild, ManufacturingState, ProductionArea};
use crate::mission_planning::Planner;
use crate::missions::{MissionFaction, MissionKind, MissionState};
use crate::research::{ResearchState, ResearchSystem, TechType};
use crate::tuning::GameConfig;
use crate::world::{Character, GameWorld};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The orders an AI may issue a day (`+0x428`, set to 10 by
/// `FUN_00485a50`; `FUN_00485b40` counts every order but a Rename `0x203`).
pub const ORDERS_PER_DAY: u32 = 10;

/// The AI player's modules (`FUN_004bea20`), in the order `FUN_00485a50`
/// adds them to the scheduler, which the round robin and the priority scan
/// walk (`ghidra/notes/ai-mission-planning.md`, "The fourteen modules").
pub const MODULE_IDS: [u8; MODULES] = [0xd, 2, 1, 3, 6, 7, 4, 5, 8, 9, 0xa, 0xb, 0xc, 0xe];

/// How many modules the AI player has (`FUN_004bea20` ids 1 to `0xe`).
const MODULES: usize = 14;

/// port: the most module steps one pass takes (`FUN_00485b40` has no
/// bound; a module that never completes would hang the original).
const MAX_STEPS: usize = 256;

/// The controller's tallies for module 1 (`FUN_0041b7e0`, `FUN_004182a0`):
/// the side's mines and refineries from its records, and its maintenance
/// capacity and requirement, without the mines and refineries of a
/// suspended system (uprising or blockade, `FUN_0050bbb0`).
fn side_tallies(
    world: &GameWorld,
    mfg_state: &ManufacturingState,
    uprisings: &crate::uprising::UprisingState,
    faction: AiFaction,
    galaxy: &crate::ai_records::Galaxy,
) -> crate::ai_module1::Tallies {
    let blocked = |s| {
        uprisings.is_uprising(s)
            || world
                .systems
                .get(s)
                .is_some_and(|sys| crate::blockade::BlockadeSystem::system_is_blockaded(world, sys))
    };
    let totals =
        crate::side_resources::maintenance_totals(world, mfg_state, faction.as_faction(), blocked);
    let mut tallies = crate::ai_module1::Tallies::of(galaxy, totals.capacity(), totals.upkeep);
    tallies.special_upkeep = special_upkeep(world, faction);
    tallies
}

/// Controller `+0x2c4` (`FUN_0041bc10`): the summed record `+0x64` of the
/// side's own objects. `FUN_00402230` sets it only for a non-character,
/// to its class maintenance (`FUN_004f2990` -> `FUN_0053b870`), so it is
/// the side's special forces' upkeep.
fn special_upkeep(world: &GameWorld, faction: AiFaction) -> i32 {
    let is_alliance = matches!(faction, AiFaction::Alliance);
    world
        .special_forces
        .values()
        .filter(|u| u.is_alliance == is_alliance)
        .filter_map(|u| world.buildable_classes.get(&u.class_dat_id))
        .map(|c| i32::try_from(c.maintenance_cost).unwrap_or(i32::MAX))
        .sum()
}

/// The order's validation (slot `+0x1c`) for the production modules'
/// orders: every Scrap member passes [`crate::scrap::check`]; Destination
/// needs the side's manager of its area there, Build the manager of its
/// class's area (hyp: a yard of that area's family, `FUN_00509670`), a
/// class the manager lists ([`crate::build_selection::listed_classes`])
/// and a count.
///
/// port: the heuristics' other orders are checked when applied.
fn accepts(
    order: &AIAction,
    world: &GameWorld,
    research: &ResearchState,
    faction: AiFaction,
    transport: &crate::troop_transport::TroopTransportState,
    movement: &crate::movement::MovementState,
) -> bool {
    let side = faction.as_faction();
    let is_alliance = matches!(faction, AiFaction::Alliance);
    match order {
        AIAction::Scrap { team } => {
            !team.is_empty()
                && team
                    .iter()
                    .all(|&t| crate::scrap::check(world, side, t).is_ok())
        }
        AIAction::SetDestination {
            system,
            area,
            destination,
        } => {
            crate::ai_module1::has_yard(
                world,
                side,
                *system,
                crate::build_selection::area_family(*area),
            ) && world.systems.contains_key(*destination)
        }
        AIAction::Build {
            system,
            kind,
            count,
            ..
        } => {
            *count > 0
                && crate::ai_module1::has_yard(
                    world,
                    side,
                    *system,
                    crate::build_selection::area_family(ProductionArea::of(*kind)),
                )
                && crate::build_selection::listed_classes(
                    world,
                    research,
                    ProductionArea::of(*kind),
                    is_alliance,
                )
                .contains(kind)
        }
        // The move command's validation (FUN_00578d00) for each regiment:
        // the side's own (FUN_004f9860), then the regiment move's checks.
        AIAction::MoveRegiments { troops, to } => {
            let blockaded = crate::blockade::BlockadeSystem::blockaded_now(world);
            !troops.is_empty()
                && troops.iter().all(|&troop| {
                    world
                        .troops
                        .get(troop)
                        .is_some_and(|t| t.is_alliance == is_alliance)
                        && transport
                            .check_move_regiment(
                                world,
                                movement,
                                &blockaded,
                                troop,
                                crate::troop_transport::RegimentTarget::System(*to),
                            )
                            .is_ok()
                })
        }
        // The same validation with the fleet for the destination
        // (FUN_00578d00, FUN_00553410).
        AIAction::BoardRegiments { troops, fleet } => {
            let blockaded = crate::blockade::BlockadeSystem::blockaded_now(world);
            !troops.is_empty()
                && troops.iter().all(|&troop| {
                    world
                        .troops
                        .get(troop)
                        .is_some_and(|t| t.is_alliance == is_alliance)
                        && transport
                            .check_move_regiment(
                                world,
                                movement,
                                &blockaded,
                                troop,
                                crate::troop_transport::RegimentTarget::Fleet(*fleet),
                            )
                            .is_ok()
                })
        }
        // Create Fleet's validation: the side's own fleet, out of
        // hyperspace, each ship a living one of it named once
        // (fleet_join::check_create_fleet), and a target elsewhere it can
        // reach (FUN_00578d00).
        AIAction::CreateFleet { fleet, ships, to } => {
            crate::fleet_join::roster(world, *fleet).is_some_and(|roster| {
                crate::fleet_join::check_create_fleet(
                    world,
                    movement,
                    *fleet,
                    ships,
                    roster,
                    is_alliance,
                )
                .is_ok()
            }) && world.fleets.get(*fleet).is_some_and(|f| {
                f.location == *to
                    || crate::movement::validate_fleet_dispatch(
                        movement,
                        world,
                        *fleet,
                        *to,
                        is_alliance,
                    )
                    .is_ok()
            })
        }
        // The move command's validation (FUN_00578d00): the side's own
        // fleet, out of hyperspace, with living ships, to a system it can
        // reach. port: a part of a fleet is checked as the whole fleet, so
        // a move to the fleet's own system is refused.
        AIAction::MoveShips { fleet, ships, to } => {
            !ships.is_empty()
                && world.fleets.get(*fleet).is_some_and(|f| {
                    ships
                        .iter()
                        .all(|&i| f.capital_ships.get(i).is_some_and(|s| s.alive))
                })
                && crate::movement::validate_fleet_dispatch(
                    movement,
                    world,
                    *fleet,
                    *to,
                    is_alliance,
                )
                .is_ok()
        }
        // The bombardment orders' enabled check (bombardment::order_enabled).
        AIAction::Bombard {
            fleets,
            system,
            kind,
        } => {
            crate::bombardment::order_enabled(world, movement, is_alliance, fleets, *system, *kind)
        }
        // Planetary Assault's enabled check (assault::order_enabled).
        AIAction::Assault { fleets, system } => {
            crate::assault::order_enabled(world, movement, transport, is_alliance, fleets, *system)
        }
        // The move command's validation (FUN_00578d00) with a fleet for
        // the destination: both the side's, out of hyperspace, and the
        // departure's checks when the target lies elsewhere.
        AIAction::JoinFleet { fleet, into } => crate::fleet_join::validate_join(
            world,
            movement,
            &crate::fleet_join::FleetMover::Fleet(*fleet),
            *into,
            is_alliance,
        )
        .is_ok(),
        // The move command's validation (FUN_00578d00): the side's own
        // squadrons, then the squadron move's checks.
        AIAction::MoveSquadrons {
            class,
            count,
            from,
            to,
        } => transport
            .squadrons
            .check(world, is_alliance, *class, *count, *from, *to)
            .is_ok(),
        AIAction::MoveSquadronTeam {
            squadrons,
            from,
            to,
        } => {
            !squadrons.is_empty()
                && squadrons.iter().all(|&(class, count)| {
                    transport
                        .squadrons
                        .check(world, is_alliance, class, count, *from, *to)
                        .is_ok()
                })
        }
        // The move command's validation (FUN_00578d00) for each person:
        // the side's own, free, aboard a fleet out of hyperspace at the
        // target. port: as for the regiments, one refusal refuses the
        // order.
        AIAction::MoveCharacters { characters, to } => {
            !characters.is_empty()
                && characters.iter().all(|&key| {
                    world
                        .characters
                        .get(key)
                        .is_some_and(|c| c.is_alliance == is_alliance && !c.is_captive)
                        && world.fleets.iter().any(|(k, f)| {
                            f.characters.contains(&key)
                                && f.location == *to
                                && !movement.is_in_transit(k)
                        })
                })
        }
        _ => true,
    }
}

/// The mission module's groups of port heuristics, after its research
/// planners: officers, espionage, rescue, reconnaissance, uprisings.
///
/// port: the original's list holds one planner per request (`FUN_004d2730`).
const MISSION_GROUPS: u8 = 1;

/// The module whose step issues the production heuristic's order for
/// `kind`: `0xa` a manufacturing facility; none for the rest.
///
/// port: a first read of the modules' orders and marks (`0x210` with marks
/// `0x1000000` and `0x2000000`, `0x212` with `0x400000`, `0x211` with
/// `0x200000`; `ghidra/notes/ai-mission-planning.md`). Module 2, now
/// ported, builds only construction yards, mines and refineries
/// (`FUN_004e2400`, `FUN_004e25c0`), so the heuristic's facilities, mostly
/// shipyards and training facilities, stand in for module `0xa`'s
/// shipyard placement. Module 3, ported, builds the defenses
/// (`FUN_004d8460`), so the heuristic's defenses stand in for no module.
/// Modules 5 and 6, ported, train special forces (`FUN_004d0590`) and
/// regiments (`FUN_004df9a0`), so the heuristic's regiments stand in for
/// no module. Modules 7 and `0xb`, ported, build squadrons
/// (`FUN_004d46e0`) and capital ships (`FUN_004ca910`), so the heuristic's
/// fighters and ships stand in for no module.
fn production_module(kind: &BuildableKind) -> Option<u8> {
    match kind {
        BuildableKind::ManufacturingFacility(_) => Some(0xa),
        _ => None,
    }
}

/// One entry of the mission module's planner list (`FUN_004d2730`).
///
/// port: the original lists one planner per request; the port lists its
/// research planners, then one entry per group of mission heuristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionEntry {
    /// The research planner with this id.
    Research(u64),
    /// A group of mission heuristics, below [`MISSION_GROUPS`].
    Group(u8),
}

/// The AI passes in a game-day at `speed`: the speed's rate, one AI tick
/// per sub-tick (`FUN_00485b40`, `FUN_0051dea0`).
///
/// port: a Paused clock runs days only for the play-test commands; they
/// take Fast's 4.
#[must_use]
pub fn passes_per_day(speed: crate::tick::GameSpeed) -> u32 {
    speed.original_rate().map_or(4, u32::from)
}

/// Diplomacy skill threshold above which a minor character is considered a
/// viable diplomat (rebellion2: `officer.GetSkillValue(Diplomacy) > 60`).
pub const DIPLOMACY_SKILL_THRESHOLD: u32 = 60;

/// Maximum construction yards the AI will build per system before stopping.
pub const MAX_CONSTRUCTION_YARDS: usize = 5;

/// Minimum popularity fraction below which the AI considers a system a
/// diplomacy target (systems already at high popularity are deprioritized).
pub const DIPLOMACY_TARGET_POPULARITY_CAP: f32 = 0.8;

/// Maximum number of new covert missions the AI will queue per evaluation pass.
/// Prevents spamming every available operative on espionage each tick interval.
pub const MAX_COVERT_OPS_PER_EVAL: usize = 3;

// ---------------------------------------------------------------------------
// GalaxyState — strategic categorization of all systems
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// AiFaction
// ---------------------------------------------------------------------------

/// The faction the AI controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiFaction {
    Alliance,
    Empire,
}

impl AiFaction {
    /// The side as a [`crate::dat::Faction`].
    #[must_use]
    pub const fn as_faction(self) -> crate::dat::Faction {
        match self {
            Self::Alliance => crate::dat::Faction::Alliance,
            Self::Empire => crate::dat::Faction::Empire,
        }
    }

    /// Convert to the `MissionFaction` used by the mission system.
    #[must_use]
    pub fn as_mission_faction(self) -> MissionFaction {
        match self {
            AiFaction::Alliance => MissionFaction::Alliance,
            AiFaction::Empire => MissionFaction::Empire,
        }
    }

    /// Returns true if the given character belongs to this faction.
    #[must_use]
    pub fn owns_character(self, c: &Character) -> bool {
        match self {
            AiFaction::Alliance => c.is_alliance,
            AiFaction::Empire => c.is_empire,
        }
    }

    /// Returns true if a system favors this faction (above neutral popularity).
    #[must_use]
    pub fn system_popularity(self, system: &crate::world::System) -> f32 {
        match self {
            AiFaction::Alliance => system.popularity_alliance,
            AiFaction::Empire => system.popularity_empire,
        }
    }
}

// ---------------------------------------------------------------------------
// AIState
// ---------------------------------------------------------------------------

/// Persistent AI state for one controlled faction (the AI player, vtable
/// `0x0065abf0`).
///
/// Tracks which characters are currently on missions, the day's order
/// count, and where the module and research planner rounds stand.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AIState {
    /// Which faction this AI controls.
    pub faction: Option<AiFaction>,
    /// Orders issued today (`+0x420`), against [`ORDERS_PER_DAY`].
    pub orders_today: u32,
    /// The module the round robin took last (`+0x84 -> +0x10`,
    /// `FUN_004be980`), an index into [`MODULE_IDS`].
    pub module_cursor: Option<usize>,
    /// Where the priority scan began last (`+0x84 -> +0x14`, `FUN_004bede0`).
    pub priority_cursor: Option<usize>,
    /// Each module's priority (`+0x34`): raised when a step issues an order
    /// and the module has more to do, zeroed before its next step.
    pub module_priority: [u32; MODULES],
    /// The orders each module has decided and not yet issued, one a step.
    pub module_pending: [Vec<AIAction>; MODULES],
    /// The entry the mission module's next step runs (`+0x4c`,
    /// `FUN_004d2730`): the one after the entry last run.
    pub mission_next: Option<MissionEntry>,
    /// Characters currently dispatched on missions (not available for re-dispatch).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_set",
        deserialize_with = "crate::serde_ordered::deserialize_hash_set"
    )]
    pub busy_characters: HashSet<CharacterKey>,
    /// The research planners the strategy keeps (side `+0xa8`, built by
    /// `FUN_0042f830` from subtypes 1 to 3), in creation order.
    pub research_planners: Vec<ResearchPlanner>,
    /// The id the next research planner takes.
    pub next_planner_id: u64,
    /// The mission planners of the port's mission heuristics (side
    /// `+0xa8`): each gathers its team at the target before it orders the
    /// mission (`FUN_004bced0` states 7 and 8; save v80).
    #[serde(default)]
    pub mission_planners: Vec<MissionPlanner>,
    /// The yard marks the modules keep on the side's system records
    /// (`+0x28 & 0x3e00000`, [`YARD_MARKS`]). The record refresh
    /// (`FUN_00433e40`) keeps them; only the modules set and clear them.
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub yard_marks: HashMap<SystemKey, u32>,
    /// The record `+0x30` bits the modules keep on the side's system
    /// records ([`crate::ai_records::KEPT_KIND`]), such as module 1's claim.
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub record_kinds: HashMap<SystemKey, u32>,
    /// The controller words the modules share (`FUN_004178f0`).
    pub controller: crate::ai_module1::Controller,
    /// Module 1's fields (`FUN_004da950`).
    pub module1: crate::ai_module1::Module1,
    /// Module 2's fields (`FUN_004e1e50`).
    pub module2: crate::ai_module2::Module2,
    /// Module 3's fields (`FUN_004d6b20`).
    pub module3: crate::ai_module3::Module3,
    /// Module 5's fields (`FUN_004cfb20`).
    pub module5: crate::ai_module5::Module5,
    /// Module 6's fields (`FUN_004dd460`).
    pub module6: crate::ai_module6::Module6,
    /// Module 7's fields (`FUN_004d2ef0`).
    #[serde(default)]
    pub module7: crate::ai_module7::Module7,
    /// Module `0xb`'s fields (`FUN_004c8820`).
    #[serde(default)]
    pub module_b: crate::ai_module_b::ModuleB,
    /// Module `0xd`'s posture and strategy (`FUN_004bf0d0`).
    #[serde(default)]
    pub strategy: crate::ai_strategy::Strategy,
    /// Module 8's fields and the sector planners (`FUN_004cf410`,
    /// controller `+0xd8`).
    #[serde(default)]
    pub module8: crate::ai_module8::Module8,
    /// What the AI keeps on its ship records (`+0x24` bits, `+0x30`,
    /// `+0x34`), by [`crate::world::ShipInstance::id`]; a ship without an
    /// entry has the constructor's ([`crate::ai_records::ShipHold`]).
    #[serde(
        default,
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub ship_holds: HashMap<u64, crate::ai_records::ShipHold>,
    /// What the AI keeps on its fleet records (`+0x38` kept bits, `+0x30`,
    /// `+0x34`); a fleet without an entry has the constructor's
    /// ([`crate::ai_records::FleetHold`]).
    #[serde(
        default,
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub fleet_holds: HashMap<FleetKey, crate::ai_records::FleetHold>,
    /// System record `+0x44`: the day each system not the side's was last
    /// seen blockaded (`FUN_00433e40`), which the records keep.
    #[serde(
        default,
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub seen: HashMap<SystemKey, u64>,
    /// The sectors whose record `+0x2c` has `0x20000000`: kind 2's plan
    /// has visited every system of the opponent there (`FUN_004b9310`).
    /// The refresh keeps the bit (`FUN_004321e0`, `+0x2c & 0x28000000`)
    /// until the strategy names its plans again (`FUN_00431790`,
    /// `FUN_004317b0`).
    #[serde(default)]
    pub swept: Vec<crate::ids::SectorKey>,
    /// The system record `+0x2c` claims the mission planners keep
    /// ([`crate::ai_records::KEPT_CLAIMS`]; save v81).
    #[serde(
        default,
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub record_claims: HashMap<SystemKey, u32>,
    /// The sector record `+0x28` claims the mission planners keep
    /// ([`crate::ai_records::KEPT_SECTOR_CLAIMS`]; save v82).
    #[serde(
        default,
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub sector_claims: HashMap<crate::ids::SectorKey, u32>,
}

/// A system record's yard marks (`+0x28 & 0x3e00000`), which
/// `FUN_00433e40` keeps across refreshes.
pub const YARD_MARKS: u32 = 0x3e0_0000;
/// Mark `0x200000`: a shipyard site (modules 7, `0xa` and `0xb`;
/// `FUN_004c8d20`, `FUN_004cacc0`).
pub const MARK_SHIPYARD: u32 = 0x20_0000;
/// Mark `0x400000`: a training facility site (modules 5 and 6;
/// `FUN_004d0860`).
pub const MARK_TRAINING: u32 = 0x40_0000;
/// Mark `0x800000`: a construction yard site for mines and refineries
/// (module 1; `FUN_004daf10`, `FUN_004db510`).
pub const MARK_MINES: u32 = 0x80_0000;
/// Mark `0x1000000`: a construction yard site for facilities (module 2,
/// step `FUN_004e2120`).
pub const MARK_FACILITIES: u32 = 0x100_0000;
/// Mark `0x2000000`: a construction yard site for the facility balance
/// (module 3, step `FUN_004d6d30`).
pub const MARK_BALANCE: u32 = 0x200_0000;

/// A research planner (`FUN_0047bba0`, `FUN_0047c170`, `FUN_0047c750`) and
/// what it keeps between cycles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchPlanner {
    /// Its id (planner `+0x18`), which a held record names (`+0x24`).
    pub id: u64,
    pub tech: TechType,
    /// The designer it holds (team list `+0x3c`).
    pub member: Option<CharacterKey>,
    /// The research site it remembers (`+0x34`).
    pub site: Option<SystemKey>,
    /// The previous cycle's target (`+0x38`).
    pub target: Option<SystemKey>,
    /// The mission its last order created (`+0x28`).
    pub mission: Option<u64>,
    /// The result of that mission's last Mission Report (`0x1f0`) not yet
    /// read (flags `0x40000000`/`0x20000000` from `FUN_0047c890`).
    pub report: Option<crate::missions::MissionOutcome>,
    /// Failed results counted against slot `+0x24` (`FUN_0047ae20`,
    /// planner `+0x6c`).
    pub failures: u32,
}

/// A mission planner (`FUN_0042f830`, run by `FUN_004bced0`): the members
/// its selectors picked and hold (`FUN_0047b610`), and the order it waits
/// to give once they all stand at the target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionPlanner {
    /// Its id (planner `+0x18`).
    pub id: u64,
    pub kind: MissionKind,
    /// The team (`+0x3c`) and decoys (`+0x44`) it holds.
    pub team: Vec<crate::missions::MissionMember>,
    pub decoys: Vec<crate::missions::MissionMember>,
    /// The target (`+0x2c`; a standing planner's `+0x34`): system,
    /// character and object. A standing planner may have none yet.
    pub target_system: Option<SystemKey>,
    pub target_character: Option<CharacterKey>,
    pub target_object: Option<crate::missions::MissionTarget>,
    /// The day it was made.
    pub created: u64,
    /// The last day it ran a cycle: the AI ticks several times a day, and
    /// a Move order is not applied before the day's end.
    #[serde(default)]
    pub last_cycle: u64,
    /// Where it gathers its team (`+0x38`, state 6 `FUN_004bd0a0`).
    #[serde(default)]
    pub staging: Option<SystemKey>,
    /// Made by the strategy's request (`FUN_004d2580`): it lives until the
    /// strategy cancels it, picks its target each cycle (state 5, slot
    /// `+0x50`), and orders its mission again once the team is free.
    #[serde(default)]
    pub standing: bool,
    /// The system whose record it claims (`+0x30`; record `+0x2c` bit
    /// `0x20000000` for Diplomacy).
    #[serde(default)]
    pub claimed: Option<SystemKey>,
    /// The sector whose record it claims (`+0x70`; Reconnaissance, sector
    /// record `+0x28` bit `0x10000000`; save v82).
    #[serde(default)]
    pub sector: Option<crate::ids::SectorKey>,
    /// The mission its last order created (`+0x28`; save v83).
    #[serde(default)]
    pub mission: Option<u64>,
    /// That mission's last Mission Report (`0x1f0`) not yet read.
    #[serde(default)]
    pub report: Option<crate::missions::MissionOutcome>,
    /// Failed results counted against slot `+0x24` (`FUN_0047ae20`,
    /// `+0x6c`).
    #[serde(default)]
    pub failures: u32,
    /// `+0x20` bits `0x40000000` (check the target) and `0x20000000`
    /// (abort) that the report set (`FUN_0047c890`) and state 1 reads
    /// (`FUN_0047b250`).
    #[serde(default)]
    pub pending: u32,
}

/// State 6 (`FUN_004bd0a0`): the planner's gathering place `+0x38`. It
/// keeps its previous one while that is still the side's (system record
/// `+0x30` bit 1, `+0x28` bit 2 clear); else it takes the target `+0x34`
/// when that is the side's; else a system of the side picked by the
/// nearness query from the target (`FUN_0041a460`); else the system the
/// first team member stands in.
///
/// hyp: the nearness query's arguments (`1, 2`) read as the side's
/// systems, nearest first; the port takes the side's unblockaded system
/// nearest the target by map distance.
fn staging_point(
    world: &GameWorld,
    is_alliance: bool,
    previous: Option<SystemKey>,
    target: SystemKey,
    team: &[crate::missions::MissionMember],
) -> Option<SystemKey> {
    let side = crate::dat::Faction::of_alliance(is_alliance);
    let own = |system: SystemKey| {
        world.systems.get(system).is_some_and(|sys| {
            crate::uprising::holder(sys) == Some(side)
                && !sys.is_destroyed
                && !crate::blockade::BlockadeSystem::system_is_blockaded(world, sys)
        })
    };
    if let Some(previous) = previous.filter(|&p| own(p)) {
        return Some(previous);
    }
    if own(target) {
        return Some(target);
    }
    let at = world
        .systems
        .get(target)
        .map(|sys| (i64::from(sys.x), i64::from(sys.y)));
    let nearest = at.and_then(|(x, y)| {
        world
            .systems
            .iter()
            .filter(|&(key, _)| own(key))
            .min_by_key(|(_, sys)| {
                let (dx, dy) = (i64::from(sys.x) - x, i64::from(sys.y) - y);
                dx * dx + dy * dy
            })
            .map(|(key, _)| key)
    });
    nearest.or_else(|| {
        team.first()
            .and_then(|&m| crate::mission_planning::standing_system(world, m))
    })
}

/// port: a mission planner that has not ordered its mission within this
/// many days is dropped and frees its members. The original's strategy
/// cancels a planner (`FUN_0042f690`) on rules not traced.
pub const MISSION_PLANNER_DAYS: u64 = 60;

impl AIState {
    #[must_use]
    pub fn new(faction: AiFaction) -> Self {
        AIState {
            faction: Some(faction),
            orders_today: 0,
            module_cursor: None,
            priority_cursor: None,
            module_priority: [0; MODULES],
            module_pending: Default::default(),
            mission_next: None,
            busy_characters: HashSet::new(),
            research_planners: Vec::new(),
            next_planner_id: 0,
            mission_planners: Vec::new(),
            yard_marks: HashMap::new(),
            record_kinds: HashMap::new(),
            controller: crate::ai_module1::Controller::default(),
            module1: crate::ai_module1::Module1::default(),
            module2: crate::ai_module2::Module2::default(),
            module3: crate::ai_module3::Module3::default(),
            module5: crate::ai_module5::Module5::default(),
            module6: crate::ai_module6::Module6::default(),
            module7: crate::ai_module7::Module7::default(),
            module_b: crate::ai_module_b::ModuleB::default(),
            strategy: crate::ai_strategy::Strategy::default(),
            module8: crate::ai_module8::Module8::default(),
            ship_holds: HashMap::new(),
            fleet_holds: HashMap::new(),
            seen: HashMap::new(),
            swept: Vec::new(),
            record_claims: HashMap::new(),
            sector_claims: HashMap::new(),
        }
    }

    /// `system`'s yard marks (record `+0x28 & 0x3e00000`).
    #[must_use]
    pub fn yard_marks(&self, system: SystemKey) -> u32 {
        self.yard_marks.get(&system).copied().unwrap_or(0)
    }

    /// Set `marks` on `system`'s record; bits outside [`YARD_MARKS`] are
    /// not marks and are ignored.
    pub fn set_yard_marks(&mut self, system: SystemKey, marks: u32) {
        if marks & YARD_MARKS != 0 {
            *self.yard_marks.entry(system).or_default() |= marks & YARD_MARKS;
        }
    }

    /// Clear `marks` from `system`'s record (`FUN_004daca0` drops a lost
    /// site's `0x800000`); a system left with none is forgotten.
    pub fn clear_yard_marks(&mut self, system: SystemKey, marks: u32) {
        if let Some(value) = self.yard_marks.get_mut(&system) {
            *value &= !marks;
            if *value == 0 {
                self.yard_marks.remove(&system);
            }
        }
    }

    /// Mission Report `0x1f0` (`FUN_00525040`, from `FUN_00523e70` on a
    /// change of result) reaches the planner whose mission it is
    /// (`FUN_00485f60` -> `FUN_004195f0` -> `FUN_0042f7f0` -> slot `+0x30`).
    /// The port hears each phase 10 result.
    pub fn note_mission_results(&mut self, results: &[crate::missions::MissionResult]) {
        for result in results {
            for p in &mut self.research_planners {
                if p.mission == Some(result.mission_id) {
                    p.report = Some(result.outcome);
                }
            }
            for p in &mut self.mission_planners {
                if p.mission == Some(result.mission_id) {
                    p.report = Some(result.outcome);
                }
            }
        }
    }

    /// An applied order binds its mission to the planner that sent it
    /// (`FUN_0047b360` stores the order in planner `+0x28`): the standing
    /// planner of its kind whose team `lead` leads, or the research planner
    /// of its tree whose designer `lead` is.
    pub fn bind_mission(
        &mut self,
        kind: MissionKind,
        lead: crate::missions::MissionMember,
        mission_id: u64,
    ) {
        if let Some(p) = self
            .mission_planners
            .iter_mut()
            .find(|p| p.standing && p.kind == kind && p.team.first() == Some(&lead))
        {
            p.mission = Some(mission_id);
            p.report = None;
        }
        let (Some(tech), Some(member)) = (kind.research_tech(), lead.character()) else {
            return;
        };
        if let Some(p) = self
            .research_planners
            .iter_mut()
            .find(|p| p.tech == tech && p.member == Some(member))
        {
            p.mission = Some(mission_id);
            p.report = None;
        }
    }

    /// A new day clears the order count (`FUN_00485de0`, slot `+0x54`, from
    /// `FUN_0041e500` at the day boundary: its window `+0x424` stays 0, so
    /// every call clears `+0x420`).
    pub fn begin_day(&mut self) {
        self.orders_today = 0;
    }

    /// Mark a character as busy (on a mission).
    pub fn mark_busy(&mut self, character: CharacterKey) {
        self.busy_characters.insert(character);
    }

    /// Release a character back to available pool.
    pub fn mark_available(&mut self, character: CharacterKey) {
        self.busy_characters.remove(&character);
    }

    /// Returns true if a character is currently busy.
    #[must_use]
    pub fn is_busy(&self, character: CharacterKey) -> bool {
        self.busy_characters.contains(&character)
    }

    /// Release every character a resolved mission frees. Special forces
    /// are not in the busy set.
    pub fn free_mission_members(&mut self, effects: &[crate::missions::MissionEffect]) {
        for effect in effects {
            if let crate::missions::MissionEffect::MemberAvailable { member } = effect {
                if let Some(character) = member.character() {
                    self.mark_available(character);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AIAction
// ---------------------------------------------------------------------------

/// A recommended action for the caller to apply to the game state.
///
/// Actions are pure recommendations — the AI system never mutates state
/// directly. The caller decides whether to apply each action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AIAction {
    /// Dispatch a mission: the order `0x240` a planner emits
    /// (`FUN_0047b360`), with its team (`+0x2c`) and decoys (`+0x58`).
    DispatchMission {
        kind: MissionKind,
        team: Vec<crate::missions::MissionMember>,
        decoys: Vec<crate::missions::MissionMember>,
        target_system: SystemKey,
        /// Target character for character-targeted missions (Assassination, Abduction, Recruitment).
        target_character: Option<CharacterKey>,
        /// The object a Sabotage or DS Sabotage mission names.
        target_object: Option<crate::missions::MissionTarget>,
    },

    /// Abort one of the side's missions: a planner's order `0x250` on its
    /// mission (`FUN_0047b250`), which ends it with code 2
    /// ([`MissionState::abort`]).
    AbortMission { mission_id: u64 },

    /// Enqueue a unit or facility for construction at a system.
    EnqueueProduction {
        system: SystemKey,
        kind: BuildableKind,
        /// The build points the unit needs: its class refined material
        /// cost.
        cost: u32,
    },

    /// Move a fleet to a target system (attack or reinforce).
    MoveFleet {
        fleet: FleetKey,
        to_system: SystemKey,
        reason: FleetMoveReason,
        /// Surface regiments to embark before departure. Empty for ordinary
        /// fleet movement. Capacity and co-location are revalidated when the
        /// action is applied.
        troops: Vec<TroopKey>,
    },

    /// Scrap a team of the side's facilities or regiments: the order
    /// `0x200` (`FUN_0053dd20`) that modules 1, 3, 6, 7 and `0xa` build
    /// (`ghidra/notes/ai-production-modules.md`).
    Scrap {
        team: Vec<crate::scrap::ScrapTarget>,
    },

    /// Move regiments of the side to a system: the order `0x201` module 6
    /// builds for regiments (`FUN_004de6f0`), each regiment carried out by
    /// [`crate::troop_transport::TroopTransportState::move_regiment`].
    MoveRegiments {
        troops: Vec<TroopKey>,
        to: SystemKey,
    },

    /// Move regiments of the side onto one of its fleets (`0x201`): a kind
    /// 4 plan boards the base's regiments onto its held fleet there
    /// (`FUN_004b8470`), each carried out by
    /// [`crate::troop_transport::TroopTransportState::move_regiment`].
    BoardRegiments {
        troops: Vec<TroopKey>,
        fleet: FleetKey,
    },

    /// Move `count` squadrons of `class` (`0x201`), carried out by
    /// [`crate::squadrons::move_squadrons`] for the class's side.
    MoveSquadrons {
        class: crate::ids::FighterKey,
        count: u32,
        from: crate::squadrons::SquadronPlace,
        to: crate::squadrons::SquadronPlace,
    },

    /// Move squadrons of several classes (`0x201`): a sector plan object's
    /// squadrons from its base to a held fleet (`FUN_004b0120`), each
    /// class's count carried out by [`crate::squadrons::move_squadrons`].
    MoveSquadronTeam {
        squadrons: Vec<(crate::ids::FighterKey, u32)>,
        from: crate::squadrons::SquadronPlace,
        to: crate::squadrons::SquadronPlace,
    },

    /// Move the people aboard a fleet to the system it orbits (`0x201`,
    /// `FUN_004b06b0`), each carried out by
    /// [`crate::missions::move_member`].
    MoveCharacters {
        characters: Vec<CharacterKey>,
        to: SystemKey,
    },

    /// Move capital ships of one of the side's fleets (`0x201`) to a
    /// system: module `0xb`'s damaged ships (`FUN_004c8e90`) or whole
    /// fleet (`FUN_004c94c0`), carried out by
    /// [`crate::fleet_join::move_ships_to_system`]. The ships are their
    /// places in the fleet.
    MoveShips {
        fleet: FleetKey,
        ships: Vec<usize>,
        to: SystemKey,
    },

    /// Move one of the side's fleets onto another (`0x201` whose target is
    /// a fleet): a sector plan object's step fleet joining a held fleet
    /// (`FUN_004b0c90`), carried out by [`crate::fleet_join::join_fleet`].
    JoinFleet { fleet: FleetKey, into: FleetKey },

    /// Planetary Bombardment (`kind` `0x220..0x223`) of `system` by the
    /// side's `fleets`: a sector plan object's bombarder at its target
    /// (`FUN_004b1330`), resolved by [`crate::bombardment::resolve`].
    Bombard {
        fleets: Vec<FleetKey>,
        system: SystemKey,
        kind: u32,
    },

    /// Planetary Assault (`0x234`) of `system` by the side's `fleets`,
    /// resolved by [`crate::assault::resolve`]
    /// (`ghidra/notes/planetary-assault.md`).
    Assault {
        fleets: Vec<FleetKey>,
        system: SystemKey,
    },

    /// Create Fleet (`0x270`): capital ships of one of the side's fleets
    /// leave it as a new fleet for the order's target (`+0x34`,
    /// `FUN_0051fbb0`): in their system through
    /// [`crate::fleet_join::create_fleet`], elsewhere through
    /// [`crate::fleet_join::move_ships_to_system`]. The ships are their
    /// places in the fleet.
    ///
    /// hyp: the order's move command (`FUN_00580a90`) takes the new fleet
    /// to the target; the player's Create Fleet targets the ships' own
    /// system.
    CreateFleet {
        fleet: FleetKey,
        ships: Vec<usize>,
        to: SystemKey,
    },

    /// Destination (`0x214`) on `system`'s `area` manager: what it builds
    /// goes to `destination` (`FUN_004dc9f0`).
    SetDestination {
        system: SystemKey,
        area: crate::manufacturing::ProductionArea,
        destination: SystemKey,
    },

    /// Build (`0x210`) `count` units of `kind` on `system`'s manager
    /// (`FUN_004dcb30`, count `+0x4c`), each `cost` build points.
    Build {
        system: SystemKey,
        kind: BuildableKind,
        cost: u32,
        count: i32,
    },
}

/// Why the AI is moving a fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FleetMoveReason {
    /// Attacking a weak or neutral enemy system.
    Attack,
    /// Reinforcing a friendly system under threat.
    Reinforce,
}

// ---------------------------------------------------------------------------
// AISystem
// ---------------------------------------------------------------------------

/// What a module step reads.
#[derive(Clone, Copy)]
struct ModuleInputs<'a> {
    world: &'a GameWorld,
    mfg_state: &'a ManufacturingState,
    mission_state: &'a MissionState,
    transport: &'a crate::troop_transport::TroopTransportState,
    tick: u64,
    config: &'a GameConfig,
    research_state: &'a ResearchState,
    uprisings: &'a crate::uprising::UprisingState,
    faction: AiFaction,
}

/// What phase 4 reads to refresh the side's records ([`AISystem::records`]).
struct RecordInputs<'a> {
    world: &'a GameWorld,
    economy: &'a crate::economy::EconomyState,
    uprisings: &'a crate::uprising::UprisingState,
    manufacturing: &'a ManufacturingState,
    research: &'a ResearchState,
    movement: &'a crate::movement::MovementState,
    transport: &'a crate::troop_transport::TroopTransportState,
}

/// Side `+0xa4`: the side's MNCHARSD classes (`FUN_0042dd00`, family
/// `0x38`, counted at setup).
///
/// hyp: each MNCHARSD record is one of the world's minor characters
/// (`rebellion-data` loads one per record), and a class serves the sides
/// its record names. The loader keeps a character's record index without
/// its family byte, so a minor character is one that is not major.
fn minor_classes(world: &GameWorld, is_alliance: bool) -> i32 {
    let count = world
        .characters
        .values()
        .filter(|c| !c.is_major)
        .filter(|c| {
            if is_alliance {
                c.is_alliance
            } else {
                c.is_empire
            }
        })
        .count();
    i32::try_from(count).unwrap_or(i32::MAX)
}

/// Stateless AI evaluation system.
pub struct AISystem;

impl AISystem {
    /// One AI pass, the AI player's tick for one sub-tick (`FUN_00485b40`):
    /// phase 4 refreshes the records ([`Planner::new`], `FUN_00418500`, and
    /// the research planners' holds, `FUN_0047b610`), phase 5 readies every
    /// module (slot `+0x10`, `FUN_004cc990`), and phase 3 runs module steps
    /// ([`Self::next_module`], [`Self::step_module`]) until a step issues an
    /// order, which ends the tick, or no module is ready. Nothing runs once
    /// the day's [`ORDERS_PER_DAY`] are spent. So a pass issues at most one
    /// order. The caller applies it before the next pass, and calls
    /// [`AIState::begin_day`] before a day's first.
    ///
    /// After applying `DispatchMission` actions, the caller must also call
    /// `state.mark_busy(character)` for each dispatched character member, and
    /// `state.mark_available(character)` when the corresponding
    /// `MissionResult` arrives.
    #[expect(
        clippy::too_many_arguments,
        reason = "Keep the existing explicit simulation inputs; grouping them changes the API."
    )]
    pub fn pass(
        state: &mut AIState,
        world: &GameWorld,
        mfg_state: &ManufacturingState,
        mission_state: &MissionState,
        movement: &crate::movement::MovementState,
        economy: &crate::economy::EconomyState,
        troop_transport: &crate::troop_transport::TroopTransportState,
        tick: u64,
        config: &GameConfig,
        research_state: &ResearchState,
        uprisings: &crate::uprising::UprisingState,
    ) -> Vec<AIAction> {
        let Some(faction) = state.faction else {
            return Vec::new();
        };
        if state.orders_today >= ORDERS_PER_DAY {
            return Vec::new();
        }
        let mut planner = Planner::new(
            world,
            mission_state,
            &state.busy_characters,
            matches!(faction, AiFaction::Alliance),
            tick,
        );
        // FUN_004198a0, FUN_00419870, FUN_004197b0: the controller's bit for
        // each tree the side has finished.
        planner.research_done(
            [TechType::Ship, TechType::Troop, TechType::Facility].map(|tech| {
                ai_research_done(
                    world,
                    research_state,
                    matches!(faction, AiFaction::Alliance),
                    tech,
                )
            }),
        );
        // FUN_0047b610: a held designer, or a mission planner's member, is
        // out of every other query.
        for p in &state.research_planners {
            if let Some(c) = p.member {
                planner.hold(crate::missions::MissionMember::Character(c));
            }
        }
        for p in &state.mission_planners {
            for &member in p.team.iter().chain(&p.decoys) {
                planner.hold(member);
            }
        }
        let mut galaxy = Self::records(
            state,
            &planner,
            &RecordInputs {
                world,
                economy,
                uprisings,
                manufacturing: mfg_state,
                research: research_state,
                movement,
                transport: troop_transport,
            },
        );
        // A record goes with its ship: holds of ships no longer among the
        // side's records are dropped.
        let alive: HashSet<u64> = galaxy
            .fleets
            .values()
            .flat_map(|f| f.ships.iter().map(|s| s.id))
            .collect();
        state.ship_holds.retain(|id, _| alive.contains(id));
        state
            .fleet_holds
            .retain(|key, _| galaxy.fleets.contains_key(key));
        // FUN_00433e40: a blockaded system not the side's is seen today
        // (record +0x44).
        for r in galaxy.sectors.iter().flat_map(|s| &s.systems) {
            if r.state & 2 != 0 && r.kind & 6 != 0 {
                state.seen.insert(r.system, tick);
            }
        }
        let ctx = ModuleInputs {
            world,
            mfg_state,
            mission_state,
            transport: troop_transport,
            tick,
            config,
            research_state,
            uprisings,
            faction,
        };
        let tallies = side_tallies(world, mfg_state, uprisings, faction, &galaxy);
        // port: eight modules share the construction yards through the
        // controller's token (FUN_004199b0: 0x80 module 1, 0x40 module 2,
        // 0x20 module 6, 0x10 module 0xa, 8 module 7, 4 module 5, 2 module
        // 0xb, 1 module 3), each passing it on once its own work allows.
        // Only modules 1, 2, 3, 5, 6, 7 and 0xb are ported, so a pass hands
        // on the yards that an unported module holds.
        if ![
            crate::ai_module1::YARD_TOKEN,
            crate::ai_module2::YARD_TOKEN,
            crate::ai_module3::YARD_TOKEN,
            crate::ai_module5::YARD_TOKEN,
            crate::ai_module6::YARD_TOKEN,
            crate::ai_module7::YARD_TOKEN,
            crate::ai_module_b::YARD_TOKEN,
        ]
        .contains(&state.controller.yard_holder)
        {
            state.controller.pass_yards();
        }
        let mut ready = [true; MODULES];
        let mut current = None;
        let mut orders = Vec::new();
        // port: a guard against a module that never completes; module 1
        // takes at most a dozen steps a tick.
        for _ in 0..MAX_STEPS {
            if state.orders_today >= ORDERS_PER_DAY {
                break;
            }
            let module = match current {
                Some(module) => module,
                None => match Self::next_module(state, &ready) {
                    Some(module) => module,
                    None => break,
                },
            };
            current = Some(module);
            // FUN_00485b40 zeroes the priority before the step.
            state.module_priority[module] = 0;
            let step = Self::step_module(module, state, &mut planner, &ctx, &mut galaxy, tallies);
            if step.raise {
                state.module_priority[module] += 1;
            }
            let gave = step.order.is_some();
            let mut complete = step.complete;
            if let Some(order) = step.order {
                if accepts(
                    &order,
                    world,
                    research_state,
                    faction,
                    troop_transport,
                    movement,
                ) {
                    state.orders_today += 1;
                    orders.push(order);
                } else {
                    // The module's slot +0x18 (FUN_004dd810).
                    match MODULE_IDS[module] {
                        1 => state.module1.refused(),
                        2 => state.module2.refused(),
                        3 => state.module3.refused(),
                        5 => state.module5.refused(),
                        6 => state.module6.refused(),
                        7 => state.module7.refused(),
                        8 => state.module8.refused(),
                        0xb => state.module_b.refused(),
                        _ => {}
                    }
                    state.module_priority[module] = 0;
                    complete = true;
                }
            }
            if !complete {
                continue;
            }
            ready[module] = false;
            current = None;
            if gave {
                break;
            }
        }
        orders
    }

    /// Phase 4's refresh of the side's records (`FUN_00418500`): the class,
    /// fleet, system and sector records ([`crate::ai_records::Galaxy`]),
    /// with the people aboard read from the planner's records
    /// ([`Planner::flags`], `FUN_00401d20`) and the yard marks the modules
    /// keep ([`AIState::yard_marks`]).
    ///
    /// port: the original refreshes a record when its dirty flag is set;
    /// the port rebuilds them all each pass (about 30 µs natively on the
    /// seed-42 galaxy).
    fn records(
        state: &AIState,
        planner: &Planner<'_>,
        inputs: &RecordInputs<'_>,
    ) -> crate::ai_records::Galaxy {
        let is_alliance = matches!(state.faction, Some(AiFaction::Alliance));
        let classes = crate::ai_records::ClassRecords::build(inputs.world);
        let fleets = HashMap::new();
        let person = |c| planner.flags(crate::missions::MissionMember::Character(c));
        let system_inputs = crate::ai_records::SystemInputs {
            world: inputs.world,
            classes: &classes,
            fleets: &fleets,
            person: &person,
            economy: inputs.economy,
            uprisings: inputs.uprisings,
            manufacturing: inputs.manufacturing,
            research: inputs.research,
            is_alliance,
            minor_classes: minor_classes(inputs.world, is_alliance),
            shield_goal: state.controller.shield_goal,
            squadron_goals: state.controller.squadron_goals,
            garrison_goals: state.controller.garrison_goals,
        };
        let mut galaxy = crate::ai_records::Galaxy::build(
            &system_inputs,
            inputs.movement,
            inputs.transport,
            &state.yard_marks,
            &state.record_kinds,
        );
        crate::ai_records::apply_ship_holds(&mut galaxy, &state.ship_holds);
        crate::ai_records::apply_fleet_holds(&mut galaxy, &state.fleet_holds);
        crate::ai_records::apply_claims(&mut galaxy, &state.record_claims);
        crate::ai_records::apply_sector_claims(&mut galaxy, &state.sector_claims);
        crate::ai_module8::mark_planners(&mut galaxy, &state.module8.planners);
        crate::ai_strategy::mark_plans(&mut galaxy, &state.strategy.plans);
        crate::ai_strategy::mark_swept(&mut galaxy, &state.swept);
        galaxy
    }

    /// The module the next step runs (`FUN_004be980`): the ready module
    /// with the highest priority above 0, scanning from a cursor that moves
    /// on one module each call, the first on a tie (`FUN_004bede0`); else
    /// the next ready module after the one the round robin took last.
    fn next_module(state: &mut AIState, ready: &[bool; MODULES]) -> Option<usize> {
        let scan = state.priority_cursor.map_or(0, |last| last + 1) % MODULES;
        state.priority_cursor = Some(scan);
        let mut best: Option<usize> = None;
        for module in (scan..scan + MODULES).map(|m| m % MODULES) {
            let top = best.map_or(0, |b| state.module_priority[b]);
            if ready[module] && state.module_priority[module] > top {
                best = Some(module);
            }
        }
        if best.is_some() {
            return best;
        }
        let next = state.module_cursor.map_or(0, |last| last + 1);
        let module = (next..next + MODULES)
            .map(|m| m % MODULES)
            .find(|&m| ready[m]);
        // With none ready, +0x10 has gone round to where it began.
        state.module_cursor = Some(module.unwrap_or(next % MODULES));
        module
    }

    /// One step of module `module`: the next order it has decided, else the
    /// first of a new decision ([`Self::run_module`]). A step with an order
    /// completes the module, and one with more to issue raises its
    /// priority, as the build modules' states (`FUN_004db2f0`,
    /// `FUN_004d0010`, `FUN_004c8ba0`, ...) and the planner modules
    /// (`FUN_004d2490` state 6) do.
    ///
    /// port: the port's heuristics decide several orders at once; the
    /// original's state machines decide one a step.
    fn step_module(
        module: usize,
        state: &mut AIState,
        planner: &mut Planner<'_>,
        ctx: &ModuleInputs<'_>,
        galaxy: &mut crate::ai_records::Galaxy,
        tallies: crate::ai_module1::Tallies,
    ) -> crate::ai_module1::Step {
        let id = MODULE_IDS[module];
        if matches!(id, 1 | 2 | 3 | 5 | 6 | 7 | 0xb) {
            let mut coin = || planner.coin();
            let mut io = crate::ai_module1::Inputs {
                world: ctx.world,
                research: ctx.research_state,
                galaxy,
                controller: &mut state.controller,
                marks: &mut state.yard_marks,
                kinds: &mut state.record_kinds,
                tallies,
                is_alliance: matches!(ctx.faction, AiFaction::Alliance),
                planners: &state.module8.planners,
                transport: ctx.transport,
                coin: &mut coin,
            };
            return match id {
                1 => state.module1.step(&mut io),
                2 => state.module2.step(&mut io),
                3 => state.module3.step(&mut io),
                5 => state.module5.step(&mut io),
                6 => state.module6.step(&mut io),
                7 => state.module7.step(&mut io),
                _ => state.module_b.step(&mut io),
            };
        }
        if id == 0xd && state.module_pending[module].is_empty() {
            let mut coin = || planner.coin();
            let mut io = crate::ai_strategy::StrategyInputs {
                galaxy,
                tallies,
                controller: &mut state.controller,
                is_alliance: matches!(ctx.faction, AiFaction::Alliance),
                day: ctx.tick,
                seen: &state.seen,
                swept: &mut state.swept,
                planners: &mut state.module8.planners,
                coin: &mut coin,
            };
            state.strategy.step(&mut io);
        }
        if id == 8 {
            return Self::module8_step(state, planner, ctx, galaxy, tallies);
        }
        if state.module_pending[module].is_empty() {
            let mut decided = Vec::new();
            Self::run_module(
                MODULE_IDS[module],
                state,
                planner,
                ctx,
                galaxy,
                &mut decided,
            );
            state.module_pending[module] = decided;
        }
        let order = (!state.module_pending[module].is_empty())
            .then(|| state.module_pending[module].remove(0));
        crate::ai_module1::Step {
            raise: !state.module_pending[module].is_empty(),
            order,
            complete: true,
        }
    }

    /// One step of module 8 ([`crate::ai_module8::Module8::step`]): its
    /// planners and their plan objects (`FUN_00477630`, kinds 1 to 6).
    fn module8_step(
        state: &mut AIState,
        planner: &mut Planner<'_>,
        ctx: &ModuleInputs<'_>,
        galaxy: &mut crate::ai_records::Galaxy,
        tallies: crate::ai_module1::Tallies,
    ) -> crate::ai_module1::Step {
        use crate::ai_module8::Step8;
        let step = {
            // The records of the people aboard the side's fleets
            // (FUN_004b06b0 reads +0x30).
            let people: HashMap<CharacterKey, u32> = ctx
                .world
                .fleets
                .values()
                .flat_map(|f| f.characters.iter().copied())
                .filter_map(|c| {
                    planner
                        .flags(crate::missions::MissionMember::Character(c))
                        .map(|flags| (c, flags))
                })
                .collect();
            let mut random = |n| planner.random(n);
            let mut io = crate::ai_module8::Inputs8 {
                world: ctx.world,
                galaxy,
                ship_holds: &mut state.ship_holds,
                fleet_holds: &mut state.fleet_holds,
                kinds: &mut state.record_kinds,
                transport: ctx.transport,
                people: &people,
                controller_flags: state.controller.flags,
                day: ctx.tick,
                seen: &state.seen,
                swept: &mut state.swept,
                balance: tallies.planner_balance,
                random: &mut random,
            };
            state.module8.step(&mut io)
        };
        let done = crate::ai_module1::Step {
            order: None,
            complete: true,
            raise: false,
        };
        match step {
            Step8::Pending => crate::ai_module1::Step::pending(),
            Step8::Done => done,
            Step8::Order(order, raise) => crate::ai_module1::Step {
                order: Some(order),
                complete: true,
                raise,
            },
        }
    }

    /// The port's work for the original module `id`.
    ///
    /// - `0xd`, the strategy: its posture and sector plans run first in
    ///   [`Self::step_module`] ([`crate::ai_strategy`]), then the research
    ///   strategy ([`Self::research_strategy`]).
    /// - 4, the missions: one entry of its list ([`Self::mission_step`]).
    /// - `0xa`: the production heuristic's manufacturing facilities.
    /// - 8: Death Star escort and fleet deployment; `0xc`: troop movement.
    ///
    /// port: the assignment of the port's heuristics to modules is a first
    /// read (the modules' roles in the note above); modules 9 and `0xe`
    /// have no port counterpart and order nothing. Modules 1, 2, 3, 5, 6, 7
    /// and `0xb` are ported ([`crate::ai_module1`], [`crate::ai_module2`],
    /// [`crate::ai_module3`], [`crate::ai_module5`], [`crate::ai_module6`],
    /// [`crate::ai_module7`], [`crate::ai_module_b`]).
    fn run_module(
        id: u8,
        state: &mut AIState,
        planner: &mut Planner<'_>,
        ctx: &ModuleInputs<'_>,
        galaxy: &mut crate::ai_records::Galaxy,
        actions: &mut Vec<AIAction>,
    ) {
        let ModuleInputs {
            world,
            mfg_state,
            mission_state,
            config,
            research_state,
            uprisings,
            faction,
            ..
        } = *ctx;
        let production = |actions: &mut Vec<AIAction>| {
            let mut all = Vec::new();
            Self::evaluate_production(world, mfg_state, research_state, faction, config, &mut all);
            actions.extend(all.into_iter().filter(|a| match a {
                AIAction::EnqueueProduction { kind, .. } => production_module(kind) == Some(id),
                _ => false,
            }));
        };
        match id {
            0xd => {
                Self::research_strategy(
                    state,
                    planner,
                    world,
                    mission_state,
                    research_state,
                    uprisings,
                    faction,
                    actions,
                );
                Self::mission_strategy(state, world, galaxy);
            }
            4 => Self::mission_step(state, planner, ctx, galaxy, actions),
            0xa => production(actions),
            0xc => Self::evaluate_troop_deployment(world, faction, config, actions),
            _ => {}
        }
    }

    /// One step of the mission module (`FUN_004d2490` states 5 and 6): the
    /// entry `+0x4c` names, else the first, runs, and `+0x4c` takes the
    /// entry after it (`FUN_004d2730`). Its list holds the research
    /// planners, then the port's groups of mission heuristics.
    fn mission_step(
        state: &mut AIState,
        planner: &mut Planner<'_>,
        ctx: &ModuleInputs<'_>,
        galaxy: &mut crate::ai_records::Galaxy,
        actions: &mut Vec<AIAction>,
    ) {
        let ModuleInputs {
            world,
            research_state,
            uprisings,
            faction,
            config,
            ..
        } = *ctx;
        let entries: Vec<MissionEntry> = state
            .research_planners
            .iter()
            .map(|p| MissionEntry::Research(p.id))
            .chain((0..MISSION_GROUPS).map(MissionEntry::Group))
            .collect();
        let at = entries
            .iter()
            .position(|&e| Some(e) == state.mission_next)
            .unwrap_or(0);
        state.mission_next = entries.get(at + 1).copied();
        let start = actions.len();
        match entries[at] {
            MissionEntry::Research(id) => {
                let is_alliance = matches!(faction, AiFaction::Alliance);
                if let Some(p) = state.research_planners.iter_mut().find(|p| p.id == id) {
                    Self::research_cycle(
                        p,
                        planner,
                        world,
                        research_state,
                        uprisings,
                        is_alliance,
                        actions,
                    );
                }
            }
            MissionEntry::Group(0) => {
                Self::evaluate_espionage(planner, world, faction, config, actions);
            }
            MissionEntry::Group(_) => {}
        }
        Self::adopt_mission_orders(state, ctx.tick, start, actions);
        Self::mission_planner_reports(state, galaxy, (ctx.mission_state, ctx.tick), actions);
        Self::mission_planner_cycle(state, planner, world, galaxy, ctx.tick, actions);
    }

    /// The heuristics' orders become mission planners (`FUN_0042f830`) that
    /// hold the members their selectors picked. A planner for the same kind
    /// and target already running takes the place of a new one, which
    /// frees the new one's members.
    ///
    /// port: the side's strategy (`+0x318`, `+0x31c`) is untraced, so the
    /// port's heuristics stand in for it and name the kind and target.
    fn adopt_mission_orders(
        state: &mut AIState,
        tick: u64,
        start: usize,
        actions: &mut Vec<AIAction>,
    ) {
        let mut kept = Vec::new();
        for action in actions.drain(start..) {
            match action {
                AIAction::DispatchMission {
                    kind,
                    team,
                    decoys,
                    target_system,
                    target_character,
                    target_object,
                } if !matches!(kind, MissionKind::Move) && kind.research_tech().is_none() => {
                    let running = state.mission_planners.iter().any(|p| {
                        p.kind == kind
                            && p.target_system == Some(target_system)
                            && p.target_character == target_character
                            && p.target_object == target_object
                    });
                    // FUN_0047b610: a member another planner holds (record
                    // +0x24) is not this one's.
                    let held: Vec<crate::missions::MissionMember> = state
                        .mission_planners
                        .iter()
                        .flat_map(|p| p.team.iter().chain(&p.decoys).copied())
                        .collect();
                    let team: Vec<_> = team.into_iter().filter(|m| !held.contains(m)).collect();
                    let decoys: Vec<_> = decoys.into_iter().filter(|m| !held.contains(m)).collect();
                    if !running && !team.is_empty() {
                        let id = state.next_planner_id;
                        state.next_planner_id += 1;
                        state.mission_planners.push(MissionPlanner {
                            id,
                            kind,
                            team,
                            decoys,
                            target_system: Some(target_system),
                            target_character,
                            target_object,
                            created: tick,
                            last_cycle: 0,
                            staging: None,
                            standing: false,
                            claimed: None,
                            sector: None,
                            mission: None,
                            report: None,
                            failures: 0,
                            pending: 0,
                        });
                    }
                }
                other => kept.push(other),
            }
        }
        actions.extend(kept);
    }

    /// One cycle of every mission planner (`FUN_004bced0`):
    ///
    /// - state 2 (`FUN_0047b110` -> `FUN_0047ba40`): a member the planner
    ///   can no longer keep is dropped, and a diplomat a better free one
    ///   outranks ([`Planner::outranked`]); a planner with no team left ends
    ///   (`FUN_0047aff0`);
    /// - state 5 (slot `+0x50`): a standing planner's target
    ///   ([`recruitment_target`], [`diplomacy_target`],
    ///   [`incite_target`], [`subdue_target`], [`recon_target`],
    ///   [`espionage_target`], [`abduction_target`], [`rescue_target`]);
    /// - state 6 ([`staging_point`]): the gathering place;
    /// - state 7 (`FUN_004bd570`): each free member away from it gets a
    ///   Move order there (`0x201`, the Move pseudo-mission);
    /// - state 8 (`FUN_0047b360`): once the team and the decoys all stand
    ///   there, the mission order on the target. A standing planner goes
    ///   on; another ends.
    ///
    /// States 3 and 4 ([`Planner::refill`]) add members up to the class's
    /// sizes after state 2.
    ///
    /// port: a planner of the heuristics whose team is still empty after
    /// state 3 ends, and one older than [`MISSION_PLANNER_DAYS`] ends. A
    /// standing planner lives until the strategy cancels it.
    fn mission_planner_cycle(
        state: &mut AIState,
        planner: &mut Planner<'_>,
        world: &GameWorld,
        galaxy: &mut crate::ai_records::Galaxy,
        tick: u64,
        actions: &mut Vec<AIAction>,
    ) {
        use crate::mission_planning::standing_system;
        let is_alliance = matches!(state.faction, Some(AiFaction::Alliance));
        let claims = &mut state.record_claims;
        let sector_claims = &mut state.sector_claims;
        let seen = &state.seen;
        state.mission_planners.retain_mut(|p| {
            if !p.standing && tick.saturating_sub(p.created) > MISSION_PLANNER_DAYS {
                return false;
            }
            if p.last_cycle == tick {
                return p.standing || !p.team.is_empty();
            }
            p.last_cycle = tick;
            let kind = p.kind;
            p.team
                .retain(|&m| planner.keeps(m) && !planner.outranked(kind, m));
            p.decoys.retain(|&m| planner.keeps(m));
            for &member in p.team.iter().chain(&p.decoys) {
                planner.hold(member);
            }
            let (team, decoys) = planner.refill(p.kind, p.team.len(), p.decoys.len());
            p.team.extend(team);
            p.decoys.extend(decoys);
            if p.standing {
                p.target_system = match p.kind {
                    MissionKind::Recruitment => recruitment_target(p, planner, world, galaxy),
                    MissionKind::InciteUprising => incite_target(p, planner, galaxy),
                    MissionKind::SubdueUprising => subdue_target(p, planner, galaxy),
                    MissionKind::Reconnaissance => {
                        recon_target(p, planner, galaxy, sector_claims, (tick, seen))
                    }
                    MissionKind::Espionage => {
                        espionage_target(p, planner, galaxy, sector_claims, (tick, seen))
                    }
                    MissionKind::Abduction => abduction_target(p, planner, world, galaxy),
                    MissionKind::Rescue => rescue_target(p, planner, world, galaxy),
                    _ => diplomacy_target(p, planner, galaxy, claims),
                };
            }
            if p.team.is_empty() {
                return p.standing;
            }
            let Some(target) = p.target_system else {
                return true;
            };
            p.staging = staging_point(world, is_alliance, p.staging, target, &p.team);
            let Some(staging) = p.staging else {
                return true;
            };
            let mut gathered = true;
            for &member in p.team.iter().chain(&p.decoys) {
                if standing_system(world, member) == Some(staging) {
                    continue;
                }
                gathered = false;
                if planner.sendable(member) {
                    actions.push(AIAction::DispatchMission {
                        kind: MissionKind::Move,
                        team: vec![member],
                        decoys: Vec::new(),
                        target_system: staging,
                        target_character: None,
                        target_object: None,
                    });
                }
            }
            let free = p.team.iter().chain(&p.decoys).all(|&m| planner.sendable(m));
            // port: a Recruitment ordered while the side's pool is empty
            // ends at once (code 0x10, FUN_0056b370); the planner waits.
            let side = crate::dat::Faction::of_alliance(is_alliance);
            let futile = p.kind == MissionKind::Recruitment && world.recruit_pool_empty(side);
            if !(gathered && free) || futile {
                return true;
            }
            actions.push(AIAction::DispatchMission {
                kind: p.kind,
                team: p.team.clone(),
                decoys: p.decoys.clone(),
                target_system: target,
                target_character: p.target_character,
                target_object: p.target_object,
            });
            p.standing
        });
    }

    /// The Mission Reports of the standing Diplomacy, Incite and Subdue
    /// planners (slot `+0x30`, `FUN_0047c890`) and state 1's abort
    /// (`FUN_0047b250`):
    ///
    /// - a failure counts against slot `+0x24` (`FUN_0047ae20`: 5 for
    ///   Diplomacy, `FUN_00404c20`, and Subdue; 10 for Incite,
    ///   `FUN_004820c0`); past it the planner aborts (`0x20000000`), else it
    ///   checks its target (`0x40000000`);
    /// - a success clears the count and checks the target;
    /// - while the planner's mission runs, a target its slot `+0x34` no
    ///   longer admits turns the check into an abort, and the abort orders
    ///   `0x250` on the mission (end code 2). The bits then clear; with no
    ///   running mission they wait.
    ///
    /// port: slot `+0x34` reads the target record's bit: `0x40`
    /// (Diplomacy, `FUN_00481eb0`), `0x100` (Incite, `FUN_004806d0`), `0x20`
    /// (Subdue, `FUN_0047d320`).
    ///
    /// The other classes read their reports in their own slots `+0x30`:
    ///
    /// - Recruitment (`FUN_0047dad0`): a success clears the count and,
    ///   when the target is still the side's with no enemy fleet, makes it
    ///   the gathering place `+0x38`; a failure counts against slot `+0x24`
    ///   (`FUN_00480df0`, 1000) and sets nothing else;
    /// - Reconnaissance (`FUN_0047f720`) and Espionage (`FUN_00480eb0`): a
    ///   success marks the target seen today (record `+0x44`,
    ///   `FUN_00433dc0`) and clears the count; either result clears the
    ///   target `+0x34`, and Espionage also its gathering place `+0x38`.
    fn mission_planner_reports(
        state: &mut AIState,
        galaxy: &crate::ai_records::Galaxy,
        (missions, day): (&MissionState, u64),
        actions: &mut Vec<AIAction>,
    ) {
        use crate::missions::MissionOutcome;
        const CHECK: u32 = 0x4000_0000;
        const ABORT: u32 = 0x2000_0000;
        let seen = &mut state.seen;
        for p in state.mission_planners.iter_mut().filter(|p| p.standing) {
            match (p.kind, p.report) {
                (MissionKind::Recruitment, Some(MissionOutcome::Success)) => {
                    p.report = None;
                    p.failures = 0;
                    let own = p
                        .target_system
                        .and_then(|t| galaxy.system(t))
                        .is_some_and(|r| r.kind & 1 != 0 && r.kind & 0x200 == 0);
                    if own {
                        p.staging = p.target_system;
                    }
                }
                (MissionKind::Recruitment, Some(MissionOutcome::Failure)) => {
                    p.report = None;
                    p.failures = p.failures.saturating_add(1);
                    if p.failures > 1000 {
                        p.failures = 0;
                    }
                }
                (MissionKind::Reconnaissance | MissionKind::Espionage, Some(outcome)) => {
                    p.report = None;
                    if outcome == MissionOutcome::Success {
                        if let Some(t) = p.target_system.filter(|&t| galaxy.system(t).is_some()) {
                            seen.insert(t, day);
                        }
                        p.failures = 0;
                    }
                    p.target_system = None;
                    if p.kind == MissionKind::Espionage {
                        p.staging = None;
                    }
                }
                _ => {}
            }
            let (limit, bit) = match p.kind {
                MissionKind::Diplomacy => (5, 0x40),
                MissionKind::SubdueUprising => (5, 0x20),
                MissionKind::InciteUprising => (10, 0x100),
                _ => continue,
            };
            match p.report.take() {
                Some(crate::missions::MissionOutcome::Failure) => {
                    p.failures += 1;
                    if p.failures > limit {
                        p.failures = 0;
                        p.pending |= ABORT;
                    } else {
                        p.pending |= CHECK;
                    }
                }
                Some(crate::missions::MissionOutcome::Success) => {
                    p.failures = 0;
                    p.pending |= CHECK;
                }
                None => {}
            }
            if p.pending & (CHECK | ABORT) == 0 {
                continue;
            }
            let Some(id) = p.mission.filter(|&id| {
                missions
                    .missions()
                    .iter()
                    .any(|m| m.id == id && m.end_code == 0)
            }) else {
                continue;
            };
            if p.pending & CHECK != 0 {
                let admitted = p
                    .target_system
                    .and_then(|t| galaxy.system(t))
                    .is_some_and(|r| r.summary & bit != 0);
                if !admitted {
                    p.pending = (p.pending & !CHECK) | ABORT;
                }
            }
            if p.pending & ABORT != 0 {
                actions.push(AIAction::AbortMission { mission_id: id });
            }
            p.pending &= !(CHECK | ABORT);
        }
    }

    /// The strategy's rows for the standing planners (`FUN_004e8290`), each
    /// kept at its want by [`Self::keep_planners`]:
    ///
    /// - row 1, Recruitment (count `+0x320`, slot `+0x40` 0,
    ///   `FUN_0047a470`): two while the side has MNCHARSD classes (side
    ///   `+0xa4`), else none; the weakest by leadership goes first (slot
    ///   `+0x18`, `FUN_0047da30`: the team's record `+0x58`);
    /// - row 5, Diplomacy (count `+0x34c`, slot `+0x40` 0xb,
    ///   `FUN_004814f0`): [`diplomacy_wanted`]; the weakest by diplomacy
    ///   (`FUN_00481500`: record `+0x40`);
    /// - row 6, Subdue (count `+0x330`, slot `+0x40` 4, `FUN_0047ce30`):
    ///   [`subdue_wanted`]; the weakest by leadership (`FUN_0047ce40`:
    ///   record `+0x58`);
    /// - row 7, Incite (count `+0x350`, slot `+0x40` 0xc, `FUN_0047fee0`):
    ///   [`incite_wanted`]; the weakest by leadership (`FUN_0047fef0`:
    ///   record `+0x58`).
    ///
    /// - row 9, Abduction (count `+0x338`, slot `+0x40` 6, `FUN_004831e0`):
    ///   one; the weakest by combat (`FUN_004831f0`: record `+0x54`);
    /// - row 10, Rescue (count `+0x340`, slot `+0x40` 8, `FUN_0047e1c0`):
    ///   one while a character of the side is a prisoner, else none; the
    ///   weakest by combat (`FUN_0047e1d0`: record `+0x54`);
    /// - row 11, Reconnaissance (count `+0x344`, slot `+0x40` 9,
    ///   `FUN_0047f670`): as many as the side's sector planners (controller
    ///   `+0xd8`, module 8); the weakest by espionage (`FUN_0047f680`:
    ///   record `+0x44`).
    ///
    /// - row 12, Espionage subtype 6 (count `+0x348`, slot `+0x40` 10,
    ///   `FUN_00480e00`): the sectors holding opponent systems (side
    ///   `+0x1bc`, sector `+0x2c` 8) less the sector planners; the weakest
    ///   by espionage (`FUN_00480e10`: record `+0x44`).
    ///
    /// port: rows 2 to 4 are the research planners
    /// ([`Self::research_strategy`]) and row 8 is not ported (it asks only
    /// for an Alliance with a Death Star fleet),
    /// so the rows are not ordered behind each other, and each count
    /// reaches its want within one step (the original asks once per
    /// strategy pass).
    fn mission_strategy(
        state: &mut AIState,
        world: &GameWorld,
        galaxy: &mut crate::ai_records::Galaxy,
    ) {
        use crate::world::Skill;
        let is_alliance = matches!(state.faction, Some(AiFaction::Alliance));
        let recruiters = if minor_classes(world, is_alliance) > 0 {
            2
        } else {
            0
        };
        Self::keep_planners(
            state,
            world,
            galaxy,
            (MissionKind::Recruitment, recruiters, Skill::Leadership),
        );
        let diplomats = diplomacy_wanted(galaxy);
        Self::keep_planners(
            state,
            world,
            galaxy,
            (MissionKind::Diplomacy, diplomats, Skill::Diplomacy),
        );
        let subduers = subdue_wanted(galaxy);
        Self::keep_planners(
            state,
            world,
            galaxy,
            (MissionKind::SubdueUprising, subduers, Skill::Leadership),
        );
        let inciters = incite_wanted(galaxy);
        Self::keep_planners(
            state,
            world,
            galaxy,
            (MissionKind::InciteUprising, inciters, Skill::Leadership),
        );
        let scouts = state.module8.planners.len();
        Self::keep_planners(
            state,
            world,
            galaxy,
            (MissionKind::Reconnaissance, scouts, Skill::Espionage),
        );
        Self::keep_planners(
            state,
            world,
            galaxy,
            (MissionKind::Abduction, 1, Skill::Combat),
        );
        // FUN_0041bc10 sets controller bit 0x200 for a record of the side's
        // character that is a prisoner (+0x30 2, 0x20000000, 0x40): a
        // prisoner counts on its home side (FUN_00401d20).
        let prisoner = world.characters.values().any(|c| {
            !c.is_killed
                && c.is_captive
                && c.is_alliance == is_alliance
                && c.is_empire != is_alliance
        });
        Self::keep_planners(
            state,
            world,
            galaxy,
            (MissionKind::Rescue, usize::from(prisoner), Skill::Combat),
        );
        let held = galaxy.sectors.iter().filter(|s| s.summary & 8 != 0).count();
        let spies = held.saturating_sub(scouts);
        Self::keep_planners(
            state,
            world,
            galaxy,
            (MissionKind::Espionage, spies, Skill::Espionage),
        );
    }

    /// One row of the strategy (`FUN_004e8290`): while the side has fewer
    /// standing planners of `kind` than `wanted`, ask for one more
    /// (`FUN_004d2580` builds it); while more, cancel the one whose team
    /// has the least `skill` (slot `+0x18`; `FUN_004e5710`), the earlier
    /// on a tie. A cancelled planner frees its members and its claim
    /// (`FUN_0047aff0`, slot `+0x38`, `FUN_00481fe0` for Diplomacy); its
    /// mission runs on.
    fn keep_planners(
        state: &mut AIState,
        world: &GameWorld,
        galaxy: &mut crate::ai_records::Galaxy,
        (kind, wanted, skill): (MissionKind, usize, crate::world::Skill),
    ) {
        let count = |state: &AIState| {
            state
                .mission_planners
                .iter()
                .filter(|p| p.standing && p.kind == kind)
                .count()
        };
        while count(state) > wanted {
            let team_skill = |p: &MissionPlanner| -> u32 {
                p.team
                    .iter()
                    .filter_map(|&m| crate::missions::member_skill(world, m, skill))
                    .sum()
            };
            let Some(at) = state
                .mission_planners
                .iter()
                .enumerate()
                .filter(|(_, p)| p.standing && p.kind == kind)
                .min_by_key(|&(at, p)| (team_skill(p), at))
                .map(|(at, _)| at)
            else {
                break;
            };
            let p = state.mission_planners.remove(at);
            if let Some(system) = p.claimed {
                release_claim(&mut state.record_claims, galaxy, system);
            }
            if let Some(sector) = p.sector {
                release_sector(
                    &mut state.sector_claims,
                    galaxy,
                    sector,
                    sector_claim_bit(p.kind),
                );
            }
        }
        while count(state) < wanted {
            let id = state.next_planner_id;
            state.next_planner_id += 1;
            state.mission_planners.push(MissionPlanner {
                id,
                kind,
                team: Vec::new(),
                decoys: Vec::new(),
                target_system: None,
                target_character: None,
                target_object: None,
                created: 0,
                last_cycle: 0,
                staging: None,
                standing: true,
                claimed: None,
                sector: None,
                mission: None,
                report: None,
                failures: 0,
                pending: 0,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Officer heuristics
    // -----------------------------------------------------------------------

    /// Plan `kind`'s team and decoys (`crate::mission_planning`) and push
    /// the order when the planner sends anyone. Returns whether it did.
    fn dispatch_planned(
        planner: &mut Planner<'_>,
        kind: MissionKind,
        target: (SystemKey, Option<CharacterKey>),
        target_object: Option<crate::missions::MissionTarget>,
        actions: &mut Vec<AIAction>,
    ) -> bool {
        let Some((team, decoys)) = planner.plan(kind) else {
            return false;
        };
        actions.push(AIAction::DispatchMission {
            kind,
            team,
            decoys,
            target_system: target.0,
            target_character: target.1,
            target_object,
        });
        true
    }

    // -----------------------------------------------------------------------
    // Espionage heuristics
    // -----------------------------------------------------------------------

    /// Dispatch covert operatives on Sabotage and Assassination missions
    /// against the enemy faction. Espionage and Abduction are the
    /// strategy's ([`Self::mission_strategy`]).
    ///
    /// port: the targets are the port's (side `+0x318` is untraced); the
    /// planner of each kind picks the members (`crate::mission_planning`).
    /// 1. **Sabotage**: enemy systems with manufacturing facilities, the most
    ///    first.
    /// 2. **Assassination**: enemy characters.
    ///
    /// At most `max_covert_ops_per_eval` covert missions are queued per pass,
    /// and a kind stops once its planner finds no team.
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    fn evaluate_espionage(
        planner: &mut Planner<'_>,
        world: &GameWorld,
        faction: AiFaction,
        config: &GameConfig,
        actions: &mut Vec<AIAction>,
    ) {
        let mut ops_queued = 0;

        // ── Priority 1: Sabotage enemy manufacturing systems ─────────────────
        // Score each enemy system by number of mfg facilities (proxy for value).
        // port: the planner's object choice is untraced (F-019 phase 5); the
        // AI names the system's first enemy manufacturing facility.
        let mut sabotage_targets: Vec<(SystemKey, usize, ManufacturingFacilityKey)> = world
            .systems
            .iter()
            .filter_map(|(sys_key, system)| {
                // Target systems where the enemy faction has manufacturing presence.
                let enemy_mfg: Vec<ManufacturingFacilityKey> = system
                    .manufacturing_facilities
                    .iter()
                    .copied()
                    .filter(|mfk| {
                        world
                            .manufacturing_facilities
                            .get(*mfk)
                            .is_some_and(|f| match faction {
                                AiFaction::Alliance => f.side == crate::dat::Faction::Empire,
                                AiFaction::Empire => f.side == crate::dat::Faction::Alliance,
                            })
                    })
                    .collect();
                enemy_mfg
                    .first()
                    .map(|&first| (sys_key, enemy_mfg.len(), first))
            })
            .collect();

        // Highest facility count first.
        sabotage_targets.sort_by_key(|a| std::cmp::Reverse(a.1));

        for &(target_sys, _, facility) in &sabotage_targets {
            if ops_queued >= config.ai.max_covert_ops_per_eval {
                break;
            }
            let target = Some(crate::missions::MissionTarget::ManufacturingFacility(
                facility,
            ));
            if !Self::dispatch_planned(
                planner,
                MissionKind::Sabotage,
                (target_sys, None),
                target,
                actions,
            ) {
                break;
            }
            ops_queued += 1;
        }

        // ── Priority 2: Assassination of enemy characters ──
        // The target chooser (FUN_004bd0a0) sends the order to the target's
        // system; a character in a fleet stands at the fleet's system.
        let whereabouts = |c: &Character| {
            c.current_system.or_else(|| {
                c.current_fleet
                    .and_then(|fleet| world.fleets.get(fleet))
                    .map(|fleet| fleet.location)
            })
        };
        let enemy_major_chars: Vec<(CharacterKey, SystemKey)> = world
            .characters
            .iter()
            .filter(|(_, c)| c.is_major && !faction.owns_character(c) && !c.is_killed)
            .filter(|(_, c)| !c.is_captive)
            .filter_map(|(key, c)| whereabouts(c).map(|system| (key, system)))
            .collect();

        for &(target_char, target_sys) in &enemy_major_chars {
            if ops_queued >= config.ai.max_covert_ops_per_eval {
                break;
            }
            let target = (target_sys, Some(target_char));
            if !Self::dispatch_planned(planner, MissionKind::Assassination, target, None, actions) {
                break;
            }
            ops_queued += 1;
        }
    }

    // -----------------------------------------------------------------------
    // Research heuristics
    // -----------------------------------------------------------------------

    /// The research part of the strategy (`FUN_004e8290`), module `0xd`.
    ///
    /// Per tree (ships, troops, facilities): first each planner reads its
    /// Mission Report (`FUN_0047c890`, `FUN_0047b250`): a success clears its
    /// failures, a failure counts one, past slot `+0x24` (1000) the planner
    /// aborts its mission, and either result aborts the mission when the
    /// remembered site lost its site bit (`FUN_0047c110` and siblings).
    /// Then the strategy cancels the weakest surplus planner while there
    /// are more than characters able to research the tree, or every one
    /// once the tree is Done for the AI (`FUN_004e5710` -> `FUN_0042f690` ->
    /// `FUN_0047aff0`: the planner and its hold go, its mission runs on),
    /// and adds planners while there are fewer (`FUN_004d2580`). The
    /// mission module runs the planners ([`Self::mission_step`]).
    ///
    /// port: a tree's cancellations and additions all happen within one
    /// step; the original makes one per strategy step. The reports' aborts
    /// are the planners' orders (`FUN_0047b250`) in the original.
    #[expect(
        clippy::too_many_arguments,
        reason = "The planners read the same state the AI evaluation does."
    )]
    fn research_strategy(
        state: &mut AIState,
        planner: &mut Planner<'_>,
        world: &GameWorld,
        mission_state: &MissionState,
        research_state: &ResearchState,
        uprisings: &crate::uprising::UprisingState,
        faction: AiFaction,
        actions: &mut Vec<AIAction>,
    ) {
        let is_alliance = matches!(faction, AiFaction::Alliance);
        let running = |id: Option<u64>| {
            id.and_then(|id| mission_state.missions().iter().find(|m| m.id == id))
                .filter(|m| m.end_code == 0)
        };
        // A planner whose mission has ended no longer tracks it.
        for p in &mut state.research_planners {
            if running(p.mission).is_none() {
                p.mission = None;
                p.report = None;
            }
        }
        for tech in TechType::ALL {
            let kind = match tech {
                TechType::Ship => MissionKind::ShipDesignResearch,
                TechType::Troop => MissionKind::TroopTrainingResearch,
                TechType::Facility => MissionKind::FacilityDesignResearch,
            };
            let site = |system: SystemKey| {
                ai_research_site(world, uprisings, research_state, is_alliance, tech, system)
            };
            for p in state
                .research_planners
                .iter_mut()
                .filter(|p| p.tech == tech)
            {
                let Some(outcome) = p.report.take() else {
                    continue;
                };
                let give_up = match outcome {
                    crate::missions::MissionOutcome::Success => {
                        p.failures = 0;
                        false
                    }
                    crate::missions::MissionOutcome::Failure => {
                        p.failures += 1;
                        p.failures > RESEARCH_PLANNER_FAILURES
                    }
                };
                if give_up {
                    p.failures = 0;
                }
                if let Some(mission) = running(p.mission) {
                    if give_up || !p.site.is_some_and(site) {
                        actions.push(AIAction::AbortMission {
                            mission_id: mission.id,
                        });
                    }
                }
            }
            let wanted = planner.able(kind);
            let done = ai_research_done(world, research_state, is_alliance, tech);
            let limit = if done { 0 } else { wanted };
            while state
                .research_planners
                .iter()
                .filter(|p| p.tech == tech)
                .count()
                > limit
            {
                // FUN_004e5710: the least team skill (slot +0x18,
                // FUN_0047bc30), the earlier planner on a tie.
                let skill = |p: &ResearchPlanner| {
                    p.member
                        .and_then(|c| {
                            crate::missions::member_skill(
                                world,
                                crate::missions::MissionMember::Character(c),
                                tech.skill(),
                            )
                        })
                        .unwrap_or(0)
                };
                let weakest = state
                    .research_planners
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.tech == tech)
                    .min_by_key(|&(at, p)| (skill(p), at))
                    .map(|(at, _)| at);
                if let Some(at) = weakest {
                    state.research_planners.remove(at);
                }
            }
            while state
                .research_planners
                .iter()
                .filter(|p| p.tech == tech)
                .count()
                < limit
            {
                let id = state.next_planner_id;
                state.next_planner_id += 1;
                state.research_planners.push(ResearchPlanner {
                    id,
                    tech,
                    member: None,
                    site: None,
                    target: None,
                    mission: None,
                    report: None,
                    failures: 0,
                });
            }
        }
    }

    /// One cycle of a research planner (`FUN_004bced0`).
    ///
    /// - State 2 (`FUN_0047b110` -> `FUN_0047ba40`): the planner drops a
    ///   member it can no longer keep.
    /// - State 3 (`FUN_0047bcd0` and siblings): without one it takes the
    ///   best candidate.
    /// - State 5 ([`research_site_choice`]): the site (`+0x34`, copied to
    ///   `+0x30`); with none the cycle ends.
    /// - State 6 (`FUN_004bd0a0`): the gathering place `+0x38`, the
    ///   previous one while it is the side's and not blockaded, else the
    ///   site when it is.
    /// - State 7 (`FUN_004bd570`): a free member away from the gathering
    ///   place (record `+0x38`, a fleet or another system) gets a Move
    ///   order `0x201` to it, the Move pseudo-mission `0x241`.
    /// - State 8 (`FUN_0047b360`): once the member stands there, the
    ///   research order on the site (`+0x30`). A member on a mission or
    ///   travelling gets neither.
    fn research_cycle(
        p: &mut ResearchPlanner,
        planner: &mut Planner<'_>,
        world: &GameWorld,
        research: &ResearchState,
        uprisings: &crate::uprising::UprisingState,
        is_alliance: bool,
        actions: &mut Vec<AIAction>,
    ) {
        use crate::missions::MissionMember;
        let kind = match p.tech {
            TechType::Ship => MissionKind::ShipDesignResearch,
            TechType::Troop => MissionKind::TroopTrainingResearch,
            TechType::Facility => MissionKind::FacilityDesignResearch,
        };
        if let Some(c) = p.member {
            if !planner.keeps(MissionMember::Character(c)) {
                p.member = None;
            }
        }
        if p.member.is_none() {
            p.member = planner.pick(kind).and_then(MissionMember::character);
        }
        let Some(member) = p.member else {
            return;
        };
        p.site = research_site_choice(planner, world, uprisings, research, is_alliance, p);
        // State 5 without a site ends the cycle (slot +0x50 returns 0).
        let Some(site) = p.site else {
            return;
        };
        let usable = |system: SystemKey| {
            let side = crate::dat::Faction::of_alliance(is_alliance);
            world.systems.get(system).is_some_and(|sys| {
                GameWorld::facility_holder(sys.control) == Some(side)
                    && !crate::blockade::BlockadeSystem::system_is_blockaded(world, sys)
            })
        };
        let target = p
            .target
            .filter(|&t| usable(t))
            .or(Some(site).filter(|&s| usable(s)));
        p.target = target;
        let Some(target) = target else {
            return;
        };
        let member = MissionMember::Character(member);
        if planner.sendable(member) {
            // FUN_0047b360 orders the mission on +0x30, the site the slot
            // copied from +0x34; +0x38 is only where the team gathers.
            let there = crate::mission_planning::standing_system(world, member) == Some(target);
            actions.push(AIAction::DispatchMission {
                kind: if there { kind } else { MissionKind::Move },
                team: vec![member],
                decoys: Vec::new(),
                target_system: if there { site } else { target },
                target_character: None,
                target_object: None,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Production heuristics
    // -----------------------------------------------------------------------

    /// For systems with idle manufacturing capacity, enqueue appropriate units.
    ///
    /// - Systems with no active queue → enqueue best available fighter class
    /// - Systems with few construction yards → enqueue a manufacturing facility
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    fn evaluate_production(
        world: &GameWorld,
        mfg_state: &ManufacturingState,
        research_state: &ResearchState,
        faction: AiFaction,
        config: &GameConfig,
        actions: &mut Vec<AIAction>,
    ) {
        let best_fighter = Self::best_fighter_class(world, research_state, faction);
        let best_capship = Self::best_capital_ship_class(world, research_state, faction);

        // Count existing fleet assets to decide what to build next.
        let is_alliance = matches!(faction, AiFaction::Alliance);
        let our_capship_count: usize = world
            .fleets
            .values()
            .filter(|f| f.is_alliance == is_alliance)
            .map(|f| f.ship_count() as usize)
            .sum();
        let our_fighter_count: usize = world
            .fleets
            .values()
            .filter(|f| f.is_alliance == is_alliance)
            .flat_map(|f| &f.fighters)
            .map(|e| e.count as usize)
            .sum::<usize>()
            + world
                .systems
                .keys()
                .map(|system| crate::squadrons::on_planet(world, system, is_alliance) as usize)
                .sum::<usize>();

        let our_faction = if is_alliance {
            crate::dat::Faction::Alliance
        } else {
            crate::dat::Faction::Empire
        };

        for (sys_key, system) in &world.systems {
            // A faction cannot use an isolated facility after losing control of
            // its system. Without this ownership gate, seeded facilities on
            // neutral worlds produced one-ship fleets across the galaxy.
            if !system.control.is_controlled_by(our_faction) {
                continue;
            }

            // Only act on systems where this faction has manufacturing facilities.
            let has_mfg = system.manufacturing_facilities.iter().any(|mfk| {
                world
                    .manufacturing_facilities
                    .get(*mfk)
                    .is_some_and(|f| match faction {
                        AiFaction::Alliance => f.side == crate::dat::Faction::Alliance,
                        AiFaction::Empire => f.side == crate::dat::Faction::Empire,
                    })
            });

            if !has_mfg {
                continue;
            }

            // A Build order needs a yard of its own area: FUN_00527d00
            // refuses one whose manager has none, and FUN_00529dd0 scraps a
            // queue left without one.
            let has_yard = |area: ProductionArea| {
                system.manufacturing_facilities.iter().any(|mfk| {
                    world.manufacturing_facilities.get(*mfk).is_some_and(|f| {
                        f.side == crate::dat::Faction::of_alliance(is_alliance)
                            && f.class_dat_id.family() == crate::build_selection::area_family(area)
                    })
                })
            };
            let ships = has_yard(ProductionArea::Shipyard);
            let training = has_yard(ProductionArea::TrainingFacility);
            let construction = has_yard(ProductionArea::ConstructionYard);

            // A unit needs its class refined cost in build points
            // (FUN_0052a8f0).
            let class_cost = |kind: BuildableKind| {
                crate::build_selection::unit_costs(world, kind).map_or(0, |(refined, _)| refined)
            };

            let queue_len = mfg_state.queued_at(sys_key);

            // Allow up to 3 items in queue (don't just wait for empty).
            if queue_len >= 3 {
                continue;
            }

            // Production priority: capital ships first (they create fleets),
            // then fighters (fill carrier slots), then construction yards.
            // Capital ships are the bottleneck — without them, no fleets.
            if ships && our_capship_count < config.production.capship_threshold {
                if let Some((capship_key, capship_class)) = best_capship {
                    let cost = capship_class.refined_material_cost;
                    actions.push(AIAction::EnqueueProduction {
                        system: sys_key,
                        kind: BuildableKind::CapitalShip(capship_key),
                        cost,
                    });
                    continue;
                }
            }

            // Build fighters: either to fill carrier capacity, or as primary
            // combat units when no capital ship class is available.
            let needs_fighters = if best_capship.is_some() {
                our_fighter_count < our_capship_count * config.production.fighter_ratio
            } else {
                true // no capships available, fighters are our only option
            };
            if ships && needs_fighters {
                if let Some((fighter_key, fighter_class)) = best_fighter {
                    let cost = fighter_class.refined_material_cost;
                    actions.push(AIAction::EnqueueProduction {
                        system: sys_key,
                        kind: BuildableKind::Fighter(fighter_key),
                        cost,
                    });
                    continue;
                }
            }

            // Build troops: controlled systems with < 2 friendly regiments get ground forces.
            let friendly_troops = system
                .ground_units
                .iter()
                .filter(|tk| {
                    world
                        .troops
                        .get(**tk)
                        .is_some_and(|t| t.is_alliance == is_alliance)
                })
                .count();
            if training && friendly_troops < 2 {
                if let Some(troop_key) = Self::find_troop_class(world, research_state, faction) {
                    actions.push(AIAction::EnqueueProduction {
                        system: sys_key,
                        kind: BuildableKind::Troop(troop_key),
                        cost: class_cost(BuildableKind::Troop(troop_key)),
                    });
                    continue;
                }
            }

            // Build defense facilities: controlled systems with < 2 defenses.
            let friendly_defenses = system
                .defense_facilities
                .iter()
                .filter(|dk| {
                    world
                        .defense_facilities
                        .get(**dk)
                        .is_some_and(|d| d.side == crate::dat::Faction::of_alliance(is_alliance))
                })
                .count();
            if construction && friendly_defenses < 2 {
                if let Some(def_key) =
                    Self::find_defense_facility_class(world, research_state, faction)
                {
                    actions.push(AIAction::EnqueueProduction {
                        system: sys_key,
                        kind: BuildableKind::DefenseFacility(def_key),
                        cost: class_cost(BuildableKind::DefenseFacility(def_key)),
                    });
                    continue;
                }
            }

            // Build more construction yards if below cap.
            let yard_count = system.manufacturing_facilities.len();
            if construction && yard_count < config.ai.max_construction_yards {
                if let Some(mfg_key) =
                    Self::find_manufacturing_facility_class(world, research_state, faction)
                {
                    actions.push(AIAction::EnqueueProduction {
                        system: sys_key,
                        kind: BuildableKind::ManufacturingFacility(mfg_key),
                        cost: class_cost(BuildableKind::ManufacturingFacility(mfg_key)),
                    });
                    continue;
                }
            }

            // Default: build more capital ships.
            if !ships {
                continue;
            }
            if let Some((capship_key, capship_class)) = best_capship {
                let cost = capship_class.refined_material_cost;
                actions.push(AIAction::EnqueueProduction {
                    system: sys_key,
                    kind: BuildableKind::CapitalShip(capship_key),
                    cost,
                });
            }
        }
    }

    /// Select the most advanced fighter class this faction has researched.
    fn best_fighter_class<'w>(
        world: &'w GameWorld,
        research_state: &ResearchState,
        faction: AiFaction,
    ) -> Option<(FighterKey, &'w crate::world::FighterClass)> {
        let is_alliance = matches!(faction, AiFaction::Alliance);
        world
            .fighter_classes
            .iter()
            .filter(|(key, _)| {
                ResearchSystem::fighter_class_is_available(world, research_state, is_alliance, *key)
            })
            .max_by_key(|(_, fc)| fc.refined_material_cost)
    }

    /// Select the most advanced capital ship class this faction has researched.
    /// Prefers higher hull (stronger ships) as the tiebreaker.
    fn best_capital_ship_class<'w>(
        world: &'w GameWorld,
        research_state: &ResearchState,
        faction: AiFaction,
    ) -> Option<(CapitalShipKey, &'w crate::world::CapitalShipClass)> {
        let is_alliance = matches!(faction, AiFaction::Alliance);
        world
            .capital_ship_classes
            .iter()
            .filter(|(key, _)| {
                ResearchSystem::ship_class_is_available(world, research_state, is_alliance, *key)
            })
            .max_by_key(|(_, cs)| cs.hull)
    }

    /// Whether a side has researched catalog class `class` on tree `tech`:
    /// the same test Build Selection applies to a player's list
    /// (`build_selection::listed_classes`), so the AI cannot order a regiment
    /// or facility class the player could not.
    ///
    /// hyp: the research-order gate itself rests on the DAT `research_order`
    /// field and the `rebellion2` prototype; recovered by the function that
    /// fills the manager lists `FUN_0052e580` reads (F-018).
    /// ASSUMPTION: a class the catalog does not hold has no order to wait on.
    fn catalog_class_is_researched(
        world: &GameWorld,
        research_state: &ResearchState,
        is_alliance: bool,
        class: DatId,
        tech: TechType,
    ) -> bool {
        world
            .buildable_classes
            .get(&class)
            .is_none_or(|c| research_state.is_unlocked(is_alliance, c.research_order, tech))
    }

    /// Find a manufacturing facility key to use as a class reference for facility construction.
    ///
    /// In Living Galaxy scope we just pick any existing faction-owned facility
    /// of a researched class as the template for "build another like this".
    fn find_manufacturing_facility_class(
        world: &GameWorld,
        research_state: &ResearchState,
        faction: AiFaction,
    ) -> Option<FacilityBuild> {
        let is_alliance = matches!(faction, AiFaction::Alliance);
        world
            .manufacturing_facilities
            .iter()
            .filter(|(_, f)| {
                Self::catalog_class_is_researched(
                    world,
                    research_state,
                    is_alliance,
                    f.class_dat_id,
                    TechType::Facility,
                )
            })
            .find(|(_, f)| match faction {
                AiFaction::Alliance => f.side == crate::dat::Faction::Alliance,
                AiFaction::Empire => f.side == crate::dat::Faction::Empire,
            })
            .map(|(_, f)| FacilityBuild {
                class: f.class_dat_id,
                is_alliance: f.side == crate::dat::Faction::Alliance,
            })
    }

    /// Find a troop unit key to use as a class reference for troop production.
    fn find_troop_class(
        world: &GameWorld,
        research_state: &ResearchState,
        faction: AiFaction,
    ) -> Option<DatId> {
        let is_alliance = matches!(faction, AiFaction::Alliance);
        world
            .troops
            .iter()
            .filter(|(_, t)| {
                Self::catalog_class_is_researched(
                    world,
                    research_state,
                    is_alliance,
                    t.class_dat_id,
                    TechType::Troop,
                )
            })
            .find(|(_, t)| match faction {
                AiFaction::Alliance => t.is_alliance,
                AiFaction::Empire => !t.is_alliance,
            })
            .map(|(_, t)| t.class_dat_id)
    }

    /// Find a defense facility key to use as a class reference for defense construction.
    fn find_defense_facility_class(
        world: &GameWorld,
        research_state: &ResearchState,
        faction: AiFaction,
    ) -> Option<FacilityBuild> {
        let is_alliance = matches!(faction, AiFaction::Alliance);
        world
            .defense_facilities
            .iter()
            .filter(|(_, d)| {
                Self::catalog_class_is_researched(
                    world,
                    research_state,
                    is_alliance,
                    d.class_dat_id,
                    TechType::Facility,
                )
            })
            .find(|(_, d)| match faction {
                AiFaction::Alliance => d.side == crate::dat::Faction::Alliance,
                AiFaction::Empire => d.side == crate::dat::Faction::Empire,
            })
            .map(|(_, d)| FacilityBuild {
                class: d.class_dat_id,
                is_alliance: d.side == crate::dat::Faction::Alliance,
            })
    }

    // -----------------------------------------------------------------------
    // Fleet deployment heuristics
    // -----------------------------------------------------------------------

    /// Redistribute troops from oversupplied to undersupplied friendly systems.
    ///
    /// Two-pass approach:
    /// 1. **Frontline reinforcement**: controlled systems adjacent to enemy
    ///    territory (any enemy-controlled neighbor) with below-minimum garrisons
    ///    receive priority reinforcements.
    /// 2. **General redistribution**: remaining undersupplied systems (HQ first)
    ///    receive troops from donor systems.
    ///
    /// Donor threshold and receiver minimum are configurable via `AiConfig`.
    /// Each move is one regiment's own Move to the receiver.
    fn evaluate_troop_deployment(
        world: &GameWorld,
        faction: AiFaction,
        config: &GameConfig,
        actions: &mut Vec<AIAction>,
    ) {
        let is_alliance = matches!(faction, AiFaction::Alliance);
        let our_faction = if is_alliance {
            crate::dat::Faction::Alliance
        } else {
            crate::dat::Faction::Empire
        };
        let enemy_faction = if is_alliance {
            crate::dat::Faction::Empire
        } else {
            crate::dat::Faction::Alliance
        };

        // Build set of enemy-controlled system positions for adjacency detection.
        let enemy_positions: Vec<(u16, u16)> = world
            .systems
            .iter()
            .filter(|(_, s)| s.control.is_controlled_by(enemy_faction))
            .map(|(_, s)| (s.x, s.y))
            .collect();

        // Collect garrison counts per controlled system.
        let mut donors: Vec<(SystemKey, Vec<TroopKey>)> = Vec::new();
        // (system, priority: 0=frontline HQ, 1=frontline, 2=HQ, 3=other)
        let mut receivers: Vec<(SystemKey, u8)> = Vec::new();

        for (sys_key, system) in &world.systems {
            if !system.control.is_controlled_by(our_faction) {
                continue;
            }

            let friendly_troops: Vec<TroopKey> = system
                .ground_units
                .iter()
                .filter(|tk| {
                    world
                        .troops
                        .get(**tk)
                        .is_some_and(|t| t.is_alliance == is_alliance)
                })
                .copied()
                .collect();

            let count = friendly_troops.len();

            // Check if this system is a frontline system (near enemy territory).
            // "Near" = within 150 coordinate units of any enemy system.
            let is_frontline = enemy_positions.iter().any(|&(ex, ey)| {
                let dx = i32::from(system.x) - i32::from(ex);
                let dy = i32::from(system.y) - i32::from(ey);
                (dx * dx + dy * dy) < 150 * 150
            });

            if count > config.ai.troop_garrison_donor_threshold {
                donors.push((sys_key, friendly_troops));
            } else if count < config.ai.troop_garrison_min {
                let priority = match (is_frontline, system.is_headquarters) {
                    (true, true) => 0,   // frontline HQ — absolute priority
                    (true, false) => 1,  // frontline — high priority
                    (false, true) => 2,  // HQ — medium priority
                    (false, false) => 3, // other — low priority
                };
                receivers.push((sys_key, priority));
            }
        }

        // Sort receivers by priority (lowest number = highest priority).
        receivers.sort_by_key(|(_, priority)| *priority);

        // Each weak system takes one regiment from an oversupplied donor,
        // which travels on its own (Move 0x201 of the regiment, as module
        // 6's moves give; port: the heuristic stands in for module 0xc).
        let mut reserved_troops = HashSet::new();
        for (receiver, _) in receivers {
            let Some(troop) = donors.iter().find_map(|(_, troops)| {
                troops
                    .iter()
                    .rev()
                    .copied()
                    .find(|troop| !reserved_troops.contains(troop))
            }) else {
                continue;
            };
            actions.push(AIAction::MoveRegiments {
                troops: vec![troop],
                to: receiver,
            });
            reserved_troops.insert(troop);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Slot `+0x24` of the research planners (`FUN_00480df0`): the failed
/// results a planner bears before it aborts its mission.
const RESEARCH_PLANNER_FAILURES: u32 = 1000;

/// `FUN_0042dd00`: the research level at which the AI counts a tree Done
/// (controller `+0x140` ships, `+0x144` troops, `+0x148` facilities): the
/// highest research order of the side's own capital ship and fighter
/// classes (families `0x14..0x1f`), of its own regiment classes
/// (`0x10..0x13`), or of every facility class (`0x22..0x2b`, any side). A
/// class counts as one side's when the other side's flag is clear
/// (`FUN_00473290`: `+0x28` 1 Alliance, 2 Empire, 0 both).
fn ai_research_goal(world: &GameWorld, is_alliance: bool, tech: TechType) -> u32 {
    let own = |alliance: bool, empire: bool| {
        if empire {
            !alliance && !is_alliance
        } else {
            is_alliance
        }
    };
    match tech {
        TechType::Ship => world
            .capital_ship_classes
            .values()
            .filter(|c| own(c.is_alliance, c.is_empire))
            .map(|c| c.research_order)
            .chain(
                world
                    .fighter_classes
                    .values()
                    .filter(|c| own(c.is_alliance, c.is_empire))
                    .map(|c| c.research_order),
            )
            .max()
            .unwrap_or(0),
        TechType::Troop => world
            .buildable_classes
            .iter()
            .filter(|(id, c)| {
                (0x10..=0x13).contains(&id.family()) && own(c.is_alliance, c.is_empire)
            })
            .map(|(_, c)| c.research_order)
            .max()
            .unwrap_or(0),
        TechType::Facility => world
            .buildable_classes
            .iter()
            .filter(|(id, _)| (0x22..=0x2b).contains(&id.family()))
            .map(|(_, c)| c.research_order)
            .max()
            .unwrap_or(0),
    }
}

/// Whether the AI counts a tree Done: the side's research-order
/// notification (`FUN_004198a0`, `FUN_00419870`, `FUN_004197b0`) found the
/// new level equal to [`ai_research_goal`] and set the controller's bit
/// `0x2000000`, `0x4000000` or `0x8000000`.
///
/// port: the bit is read as the level having reached a nonzero goal; the
/// notification fires only on a change of level, so a goal of 0 never sets
/// it.
pub(crate) fn ai_research_done(
    world: &GameWorld,
    research: &ResearchState,
    is_alliance: bool,
    tech: TechType,
) -> bool {
    let goal = ai_research_goal(world, is_alliance, tech);
    goal != 0 && research.level(is_alliance, tech) >= goal
}

/// A research site of the tree (`FUN_00433e40`, system record `+0x2c` bit
/// 4, 8 or `0x10`): the side's system (`+0x30` bit 0) in no uprising or
/// blockade (`+0x28 & 3`) with an active yard of the tree's area
/// ([`crate::missions::research_site`]) and an active regiment of the side
/// (`+0x90`), while the tree is not Done for the AI.
///
/// port: the original also needs the production AI's mark for a yard of
/// the area there (`+0x28` bits `0x200000`, `0x400000`, `0x3800000`, set by
/// `FUN_004cacc0` and its siblings when they order one). The port keeps the
/// marks ([`AIState::yard_marks`]) but no module sets them yet, so every
/// yard of the side counts.
fn ai_research_site(
    world: &GameWorld,
    uprisings: &crate::uprising::UprisingState,
    research: &ResearchState,
    is_alliance: bool,
    tech: TechType,
    system: SystemKey,
) -> bool {
    let side = crate::dat::Faction::of_alliance(is_alliance);
    let Some(sys) = world.systems.get(system) else {
        return false;
    };
    GameWorld::facility_holder(sys.control) == Some(side)
        && crate::missions::research_site(world, uprisings, side, tech, system)
        && sys.ground_units.iter().any(|&key| {
            world
                .troops
                .get(key)
                .is_some_and(|t| t.is_alliance == is_alliance)
        })
        && !ai_research_done(world, research, is_alliance, tech)
}

/// Slot `+0x50` of a research planner (`FUN_0047be50` ships,
/// `FUN_0047c430` troops, `FUN_0047ca80` facilities): the research site the
/// planner works towards (`+0x34`).
///
/// 1. The remembered site while it still has the tree's site bit.
/// 2. With a previous target (`+0x38`): the nearest sector holding a site
///    (`FUN_00419b80`, sector `+0x28` bit 1, 2 or 4, by the squared distance
///    from the target's sector, order 2), then its site nearest the target
///    (`FUN_0041a400` -> `FUN_00431c00`, order 2).
/// 3. Else the sector with the most sites (`FUN_00419a00`, sector `+0x154`,
///    `+0x158`, `+0x15c`, order 1), then its site with the highest support
///    (`FUN_0041a340` -> `FUN_00431830`, system record `+0x48`: the side's
///    support, `FUN_00507270`, or 100 unpopulated; order 1).
///
/// Ties fall to [`Planner::pick_ranked`]. hyp: the records' `+0x40` place
/// that the distance keys read is the sector's and system's map position.
fn research_site_choice(
    planner: &mut Planner<'_>,
    world: &GameWorld,
    uprisings: &crate::uprising::UprisingState,
    research: &ResearchState,
    is_alliance: bool,
    p: &ResearchPlanner,
) -> Option<SystemKey> {
    let tech = p.tech;
    let site =
        |system: SystemKey| ai_research_site(world, uprisings, research, is_alliance, tech, system);
    if let Some(s) = p.site.filter(|&s| site(s)) {
        return Some(s);
    }
    let sites_in = |sector: crate::ids::SectorKey| -> Vec<SystemKey> {
        world
            .systems
            .iter()
            .filter(|(key, sys)| sys.sector == sector && site(*key))
            .map(|(key, _)| key)
            .collect()
    };
    let squared = |a: (u16, u16), b: (u16, u16)| {
        let dx = i64::from(a.0) - i64::from(b.0);
        let dy = i64::from(a.1) - i64::from(b.1);
        dx * dx + dy * dy
    };
    let sectors: Vec<(crate::ids::SectorKey, Vec<SystemKey>)> = world
        .sectors
        .keys()
        .map(|sector| (sector, sites_in(sector)))
        .filter(|(_, sites)| !sites.is_empty())
        .collect();
    if let Some(from) = p.target.and_then(|t| world.systems.get(t)) {
        let here = world.sectors.get(from.sector).map(|s| (s.x, s.y))?;
        let sector = planner.pick_ranked(
            sectors.iter().map(|(sector, _)| {
                let at = world.sectors.get(*sector).map_or((0, 0), |s| (s.x, s.y));
                (*sector, squared(here, at))
            }),
            false,
        )?;
        let sites = sectors.into_iter().find(|(s, _)| *s == sector)?.1;
        planner.pick_ranked(
            sites.into_iter().map(|system| {
                let sys = &world.systems[system];
                (system, squared((from.x, from.y), (sys.x, sys.y)))
            }),
            false,
        )
    } else {
        let sector = planner.pick_ranked(
            sectors
                .iter()
                .map(|(sector, sites)| (*sector, i64::try_from(sites.len()).unwrap_or(0))),
            true,
        )?;
        let side = crate::dat::Faction::of_alliance(is_alliance);
        let sites = sectors.into_iter().find(|(s, _)| *s == sector)?.1;
        planner.pick_ranked(
            sites.into_iter().map(|system| {
                let sys = &world.systems[system];
                let support = if sys.is_populated {
                    crate::uprising::support_points(sys, side)
                } else {
                    100
                };
                (system, i64::from(support))
            }),
            true,
        )
    }
}

/// Side `+0x214` (`FUN_0041b7e0`, `FUN_00418500`): the Diplomacy planners
/// the strategy wants. The sector records' `+0x168` (their systems with
/// record `+0x2c` bit `0x40`: own below 70 support, or neutral, with no
/// fleet of the side and no uprising) summed, then `/ 5 + 1` when above 0.
fn diplomacy_wanted(galaxy: &crate::ai_records::Galaxy) -> usize {
    let sum: i32 = galaxy
        .sectors
        .iter()
        .map(|s| s.values[crate::ai_records::sector_at(0x168)])
        .sum();
    let wanted = if sum > 0 { sum / 5 + 1 } else { sum };
    usize::try_from(wanted).unwrap_or(0)
}

/// Slot `+0x38` (`FUN_00481fe0`): clear `system`'s Diplomacy claim (record
/// `+0x2c` bit `0x20000000`).
fn release_claim(
    claims: &mut HashMap<SystemKey, u32>,
    galaxy: &mut crate::ai_records::Galaxy,
    system: SystemKey,
) {
    use crate::ai_records::CLAIM_DIPLOMACY;
    if let Some(bits) = claims.get_mut(&system) {
        *bits &= !CLAIM_DIPLOMACY;
        if *bits == 0 {
            claims.remove(&system);
        }
    }
    galaxy.update_summary(system, 0, CLAIM_DIPLOMACY);
}

/// Claim `system` for Diplomacy (record `+0x2c` `|= 0x20000000`).
fn take_claim(
    claims: &mut HashMap<SystemKey, u32>,
    galaxy: &mut crate::ai_records::Galaxy,
    system: SystemKey,
) {
    use crate::ai_records::CLAIM_DIPLOMACY;
    *claims.entry(system).or_insert(0) |= CLAIM_DIPLOMACY;
    galaxy.update_summary(system, CLAIM_DIPLOMACY, 0);
}

/// `FUN_0041a1d0`: the sectors whose `+0x24`, `+0x28`, `+0x2c` pass
/// `masks`, each keyed on a draw below 20 (`FUN_0041cd80(0x14)`) as it goes
/// into the list, so the key draws and the tie coins interleave.
fn random_sectors(
    planner: &mut Planner<'_>,
    galaxy: &crate::ai_records::Galaxy,
    masks: [crate::ai_records::Mask; 3],
    order: crate::ai_records::Order,
) -> Vec<crate::ids::SectorKey> {
    let sectors: Vec<crate::ids::SectorKey> = galaxy
        .sectors
        .iter()
        .filter(|s| s.admitted(masks))
        .map(|s| s.sector)
        .collect();
    let draws = std::cell::RefCell::new(planner);
    let items = sectors
        .into_iter()
        .map(|s| (s, draws.borrow_mut().random(0x14)));
    crate::ai_records::ranked(items, order, &mut || draws.borrow_mut().coin())
}

/// `FUN_0041a460` -> `FUN_00431f40`: of `sector`'s systems whose `+0x28`,
/// `+0x2c`, `+0x30` pass `masks`, each keyed on a draw below 10 as it goes
/// into the list, the one picked (`FUN_004357b0`).
fn random_system(
    planner: &mut Planner<'_>,
    galaxy: &crate::ai_records::Galaxy,
    sector: Option<crate::ids::SectorKey>,
    masks: [crate::ai_records::Mask; 3],
    order: crate::ai_records::Order,
) -> Option<SystemKey> {
    let systems: Vec<SystemKey> = galaxy
        .sector(sector?)?
        .systems
        .iter()
        .filter(|r| {
            masks[0].admits(r.state) && masks[1].admits(r.summary) && masks[2].admits(r.kind)
        })
        .map(|r| r.system)
        .collect();
    let draws = std::cell::RefCell::new(planner);
    let items = systems
        .into_iter()
        .map(|s| (s, draws.borrow_mut().random(10)));
    let list = crate::ai_records::ranked(items, order, &mut || draws.borrow_mut().coin());
    crate::ai_records::pick(&[list])
}

/// State 5 of a Diplomacy planner (slot `+0x50`, `FUN_00481720`): its
/// target `+0x34`, claimed in its record (`+0x2c` bit `0x20000000`).
///
/// Without a team it releases its target's claim and picks none. With
/// one it keeps its target while the record still has `+0x2c` bit `0x40`
/// and no enemy fleet (`+0x30` bit `0x200`). Otherwise, the first that
/// gives a system:
///
/// 1. the sector with own systems and an unclaimed candidate (`+0x28`
///    `0x4000020`, `+0x2c` 4) of the lowest average own support (key
///    `+0x8c`, order 2); below 70, a random own unclaimed candidate there
///    (`+0x2c` `0x40`, `+0x30` 1, order 2);
/// 2. from the previous gathering place `+0x38`: the nearest sector with
///    neutral systems and an unclaimed candidate (`+0x2c` `0x10`, order
///    2), then a random neutral unclaimed candidate there (`+0x30` 2);
/// 3. from `+0x38`: the unclaimed candidate nearest it in the sector last
///    picked (`FUN_0041a400`, order 2);
/// 4. the sector ranked by its own and its neutral systems and a draw
///    (keys `+0x7c`, `+0x84`, `FUN_0041a1d0`, order 1), then a random
///    neutral unclaimed candidate (order 1);
/// 5. the sector ranked by its own systems and a draw over every sector
///    with a candidate (`+0x28` `0x20`), then a random unclaimed candidate
///    of any side (order 2).
fn diplomacy_target(
    p: &mut MissionPlanner,
    planner: &mut Planner<'_>,
    galaxy: &mut crate::ai_records::Galaxy,
    claims: &mut HashMap<SystemKey, u32>,
) -> Option<SystemKey> {
    use crate::ai_records::{pick, sector_at, Mask, Order, CLAIM_DIPLOMACY};
    let any = Mask::new(0, 0);
    let current = p.target_system;
    if p.team.is_empty() {
        if let Some(t) = current {
            release_claim(claims, galaxy, t);
            if p.claimed == Some(t) {
                p.claimed = None;
            }
        }
        return current;
    }
    let claim = |p: &mut MissionPlanner,
                 galaxy: &mut crate::ai_records::Galaxy,
                 claims: &mut HashMap<SystemKey, u32>,
                 t: SystemKey| {
        take_claim(claims, galaxy, t);
        p.claimed = Some(t);
        Some(t)
    };
    if let Some(t) = current {
        if let Some(r) = galaxy.system(t) {
            if r.summary & 0x40 != 0 && r.kind & 0x200 == 0 {
                return claim(p, galaxy, claims, t);
            }
        }
        release_claim(claims, galaxy, t);
        if p.claimed == Some(t) {
            p.claimed = None;
        }
    }
    let candidate = Mask::new(0x40, CLAIM_DIPLOMACY);
    let open = Mask::new(0x400_0020, 0);
    let previous = p.staging;
    let coin_of = |planner: &mut Planner<'_>| planner.coin();
    let sectors = galaxy.sector_query(
        [any, open, Mask::new(4, 0)],
        7,
        Order::Descending,
        &mut || coin_of(planner),
    );
    let mut last = pick(&[sectors]);
    let low = last
        .and_then(|s| galaxy.sector(s))
        .is_some_and(|r| r.values[sector_at(0x8c)] < 70);
    let mut target = None;
    if low {
        target = random_system(
            planner,
            galaxy,
            last,
            [any, candidate, Mask::new(1, 0)],
            Order::Descending,
        );
    }
    if let (None, Some(prev)) = (target, previous) {
        let sectors = galaxy.sectors_by_distance(
            prev,
            [any, open, Mask::new(0x10, 0)],
            Order::Descending,
            &mut || coin_of(planner),
        );
        last = pick(&[sectors]);
        target = random_system(
            planner,
            galaxy,
            last,
            [any, candidate, Mask::new(2, 0)],
            Order::Descending,
        );
    }
    if let (None, Some(prev), Some(sector)) = (target, previous, last) {
        let systems = galaxy.systems_by_distance(
            sector,
            prev,
            [any, candidate, any],
            Order::Descending,
            &mut || coin_of(planner),
        );
        target = pick(&[systems]);
    }
    if target.is_none() {
        let masks = [any, open, Mask::new(0x10, 0)];
        let own = galaxy.sector_query(masks, 3, Order::Ascending, &mut || coin_of(planner));
        let neutral = galaxy.sector_query(masks, 5, Order::Ascending, &mut || coin_of(planner));
        let drawn = random_sectors(planner, galaxy, masks, Order::Ascending);
        let sector = pick(&[own, neutral, drawn]);
        target = random_system(
            planner,
            galaxy,
            sector,
            [any, candidate, Mask::new(2, 0)],
            Order::Ascending,
        );
    }
    if target.is_none() {
        let own = galaxy.sector_query([any, open, any], 3, Order::Ascending, &mut || {
            coin_of(planner)
        });
        let drawn = random_sectors(
            planner,
            galaxy,
            [any, Mask::new(0x20, 0), any],
            Order::Ascending,
        );
        let sector = pick(&[own, drawn]);
        target = random_system(
            planner,
            galaxy,
            sector,
            [any, candidate, any],
            Order::Descending,
        );
    }
    target.and_then(|t| claim(p, galaxy, claims, t))
}

/// Slot `+0x38` of a Reconnaissance or Espionage planner (`FUN_0047fe10`,
/// `FUN_00481420`): clear `bit`, its claim on its sector (record `+0x28`
/// bit `0x10000000` or `0x20000000`).
fn release_sector(
    claims: &mut HashMap<crate::ids::SectorKey, u32>,
    galaxy: &mut crate::ai_records::Galaxy,
    sector: crate::ids::SectorKey,
    bit: u32,
) {
    if let Some(bits) = claims.get_mut(&sector) {
        *bits &= !bit;
        if *bits == 0 {
            claims.remove(&sector);
        }
    }
    galaxy.update_sector_state(sector, 0, bit);
}

/// Claim `sector` with `bit` (record `+0x28`).
fn take_sector(
    claims: &mut HashMap<crate::ids::SectorKey, u32>,
    galaxy: &mut crate::ai_records::Galaxy,
    sector: crate::ids::SectorKey,
    bit: u32,
) {
    *claims.entry(sector).or_insert(0) |= bit;
    galaxy.update_sector_state(sector, bit, 0);
}

/// A planner's sector claim bit: `0x20000000` for Espionage, else
/// `0x10000000` (Reconnaissance).
fn sector_claim_bit(kind: MissionKind) -> u32 {
    if kind == MissionKind::Espionage {
        crate::ai_records::CLAIM_ESPIONAGE
    } else {
        crate::ai_records::CLAIM_RECON
    }
}

/// State 5 of a Reconnaissance planner (slot `+0x50`, `FUN_0047f970`): its
/// target `+0x34`, a system not the side's (record `+0x2c` bit `0x80`), and
/// the sector `+0x70` it claims (sector record `+0x28` bit `0x10000000`).
///
/// It keeps its target while the record has `0x80`. Otherwise it releases
/// its sector and picks a sector with such a system (`+0x28` `0x40`) and
/// no claim, the near one from the previous gathering place `+0x38`
/// (`FUN_00419b80`, order 2) or a drawn one (`FUN_0041a1d0`):
///
/// - while the side counts a sector without `+0x24` bit `0x40000000`
///   (controller `+0x1c4`, `FUN_0041b7e0`): the sector must also lack
///   `+0x28` `0x40000000` and `+0x2c` `0x4000000`; the drawn one is order
///   2; then a random unexplored such system (`FUN_0041a460`, forbidding
///   `+0x2c` `0x8000000`; order 1 without `+0x38`, 2 with it);
/// - else the sector must lack `+0x2c` `0x4000000` (the drawn one order
///   1), then the such system seen longest ago (`FUN_0041a3a0`, order 1).
///
/// A planner without a team still picks: `FUN_0047f970` does not test
/// `+0x20` bit 1.
fn recon_target(
    p: &mut MissionPlanner,
    planner: &mut Planner<'_>,
    galaxy: &mut crate::ai_records::Galaxy,
    claims: &mut HashMap<crate::ids::SectorKey, u32>,
    (day, seen): (u64, &HashMap<SystemKey, u64>),
) -> Option<SystemKey> {
    use crate::ai_records::{pick, Mask, Order, CLAIM_RECON};
    let any = Mask::new(0, 0);
    if let Some(t) = p
        .target_system
        .filter(|&t| galaxy.system(t).is_some_and(|r| r.summary & 0x80 != 0))
    {
        return Some(t);
    }
    if let Some(sector) = p.sector.take() {
        release_sector(claims, galaxy, sector, CLAIM_RECON);
    }
    let foreign = Mask::new(0x80, 0);
    let open_sectors = galaxy
        .sectors
        .iter()
        .filter(|s| s.candidates & 0x4000_0000 == 0)
        .count();
    let (sector, target) = if open_sectors >= 1 {
        let masks = [any, Mask::new(0x40, 0x5000_0000), Mask::new(0, 0x400_0000)];
        let (sectors, order) = match p.staging {
            Some(prev) => (
                galaxy.sectors_by_distance(prev, masks, Order::Descending, &mut || planner.coin()),
                Order::Descending,
            ),
            None => (
                random_sectors(planner, galaxy, masks, Order::Descending),
                Order::Ascending,
            ),
        };
        let sector = pick(&[sectors]);
        let unexplored = [any, Mask::new(0x80, 0x800_0000), any];
        (
            sector,
            random_system(planner, galaxy, sector, unexplored, order),
        )
    } else {
        let masks = [any, Mask::new(0x40, 0x1000_0000), Mask::new(0, 0x400_0000)];
        let sectors = match p.staging {
            Some(prev) => {
                galaxy.sectors_by_distance(prev, masks, Order::Descending, &mut || planner.coin())
            }
            None => random_sectors(planner, galaxy, masks, Order::Ascending),
        };
        let sector = pick(&[sectors]);
        let target = sector.and_then(|sector| {
            pick(&[galaxy.system_query_by_age(
                sector,
                [any, foreign, any],
                (day, seen),
                Order::Ascending,
                &mut || planner.coin(),
            )])
        });
        (sector, target)
    };
    let (Some(sector), Some(target)) = (sector, target) else {
        return None;
    };
    take_sector(claims, galaxy, sector, CLAIM_RECON);
    p.sector = Some(sector);
    Some(target)
}

/// State 5 of an Espionage planner of subtype 6 (slot `+0x50`,
/// `FUN_00481110`): its target `+0x34`, an opponent's system (record `+0x30`
/// bit 4, `+0x2c` bit `0x200`), and the sector `+0x70` it claims (sector
/// record `+0x28` bit `0x20000000`).
///
/// It keeps its target while that is still the opponent's. Otherwise it
/// releases its sector and picks a sector with an opponent's system
/// (`+0x28` `0x400`, `+0x2c` 8) and neither its claim nor `+0x2c`
/// `0x4000000`: near the previous gathering place `+0x38` (`FUN_00419b80`,
/// order 2) or drawn (`FUN_0041a1d0`, order 1). The target is its
/// opponent's system seen longest ago (`FUN_0041a3a0`, order 1).
///
/// Like Reconnaissance it does not test for a team.
fn espionage_target(
    p: &mut MissionPlanner,
    planner: &mut Planner<'_>,
    galaxy: &mut crate::ai_records::Galaxy,
    claims: &mut HashMap<crate::ids::SectorKey, u32>,
    (day, seen): (u64, &HashMap<SystemKey, u64>),
) -> Option<SystemKey> {
    use crate::ai_records::{pick, Mask, Order, CLAIM_ESPIONAGE};
    let any = Mask::new(0, 0);
    if let Some(t) = p.target_system.filter(|&t| {
        galaxy
            .system(t)
            .is_some_and(|r| r.kind & 4 != 0 && r.summary & 0x200 != 0)
    }) {
        return Some(t);
    }
    if let Some(sector) = p.sector.take() {
        release_sector(claims, galaxy, sector, CLAIM_ESPIONAGE);
    }
    let masks = [any, Mask::new(0x400, 0x2000_0000), Mask::new(8, 0x400_0000)];
    let sectors = match p.staging {
        Some(prev) => {
            galaxy.sectors_by_distance(prev, masks, Order::Descending, &mut || planner.coin())
        }
        None => random_sectors(planner, galaxy, masks, Order::Ascending),
    };
    let sector = pick(&[sectors])?;
    let target = pick(&[galaxy.system_query_by_age(
        sector,
        [any, Mask::new(0x200, 0), Mask::new(4, 0)],
        (day, seen),
        Order::Ascending,
        &mut || planner.coin(),
    )])?;
    take_sector(claims, galaxy, sector, CLAIM_ESPIONAGE);
    p.sector = Some(sector);
    Some(target)
}

/// Whether `character` is an opponent's main character free in the field
/// for the side's records (`+0x30` bits 8 and 4, `FUN_00401d20`).
fn enemy_main(planner: &Planner<'_>, character: CharacterKey) -> bool {
    planner
        .flags(crate::missions::MissionMember::Character(character))
        .is_some_and(|f| f & 0xc == 0xc)
}

/// `FUN_00483d10`: a random opponent's main character (record `+0x30` 8
/// and 4) at `system`: on the planet when its record has `+0x2c` `0x1000`,
/// and aboard a fleet there not the side's carrying one (fleet record
/// `+0x38` `0x200000`) when it has `0x2000`; the draw is
/// `FUN_0041cd80(count)`.
///
/// port: the candidates are taken in the world's character order.
fn abduction_character(
    planner: &mut Planner<'_>,
    world: &GameWorld,
    galaxy: &crate::ai_records::Galaxy,
    system: SystemKey,
) -> Option<CharacterKey> {
    let record = galaxy.system(system)?;
    let sys = world.systems.get(system)?;
    let mut candidates: Vec<CharacterKey> = Vec::new();
    if record.summary & 0x1000 != 0 {
        candidates.extend(world.characters.iter().filter_map(|(key, c)| {
            (!c.is_killed
                && c.current_system == Some(system)
                && c.current_fleet.is_none()
                && enemy_main(planner, key))
            .then_some(key)
        }));
    }
    if record.summary & 0x2000 != 0 {
        for &fleet in &sys.fleets {
            let carrying = galaxy.fleets.get(&fleet).is_some_and(|r| {
                r.flags & crate::ai_records::FLEET_OWN == 0 && r.flags & 0x20_0000 != 0
            });
            if !carrying {
                continue;
            }
            candidates.extend(world.characters.iter().filter_map(|(key, c)| {
                (!c.is_killed && c.current_fleet == Some(fleet) && enemy_main(planner, key))
                    .then_some(key)
            }));
        }
    }
    if candidates.is_empty() {
        return None;
    }
    let count = i32::try_from(candidates.len()).unwrap_or(i32::MAX);
    let at = usize::try_from(planner.random(count)).unwrap_or(0);
    candidates.get(at).copied()
}

/// State 5 of an Abduction planner (slot `+0x50`, `FUN_00483640`): its
/// target character `+0x30` and system `+0x34`.
///
/// It keeps its character while that still stands at `+0x34` (the walk up
/// its containers) as an opponent's main character, neither travelling
/// nor a prisoner (record `+0x30` 8 and 4, not `0x60`). Otherwise it lists
/// the systems holding one: the sectors with any (`+0x28` `0x100`, key
/// `+0x174`, order 1) in turn, and in each the systems with one on the
/// planet (`+0x2c` `0x1000`, key `v[47]`, order 1), then those with one
/// aboard a fleet (`0x2000`, order 2). It takes the first where the decoys
/// gathered at `+0x38` (`FUN_004bd000`, list `+0x44`) fill the decoy cap
/// (slot `+0x44`, `FUN_004047d0`: 4) or reach half its defense (record
/// `+0xbc`), then a character there ([`abduction_character`]).
fn abduction_target(
    p: &mut MissionPlanner,
    planner: &mut Planner<'_>,
    world: &GameWorld,
    galaxy: &crate::ai_records::Galaxy,
) -> Option<SystemKey> {
    use crate::ai_records::{picks, sector_at, Mask, Order};
    use crate::mission_planning::standing_system;
    let any = Mask::new(0, 0);
    let gathered = p
        .decoys
        .iter()
        .filter(|&&m| p.staging.is_some() && standing_system(world, m) == p.staging)
        .filter(|&&m| planner.sendable(m))
        .count();
    let gathered = i32::try_from(gathered).unwrap_or(i32::MAX);
    if let (Some(system), Some(target)) = (p.target_system, p.target_character) {
        let kept = world.characters.get(target).is_some_and(|c| {
            let here = c.current_system.or_else(|| {
                c.current_fleet
                    .and_then(|f| world.fleets.get(f))
                    .map(|f| f.location)
            });
            here == Some(system)
        }) && planner
            .flags(crate::missions::MissionMember::Character(target))
            .is_some_and(|f| f & 0xc == 0xc && f & 0x60 == 0);
        if kept {
            return Some(system);
        }
    }
    p.target_character = None;
    let sectors = picks(vec![galaxy.sector_query(
        [any, Mask::new(0x100, 0), any],
        sector_at(0x174),
        Order::Ascending,
        &mut || planner.coin(),
    )]);
    let mut systems = Vec::new();
    for sector in sectors {
        let planet = galaxy.system_query(
            sector,
            [any, Mask::new(0x1000, 0), any],
            47,
            Order::Ascending,
            &mut || planner.coin(),
        );
        systems.extend(picks(vec![planet]));
        let aboard = galaxy.system_query(
            sector,
            [any, Mask::new(0x2000, 0), any],
            47,
            Order::Descending,
            &mut || planner.coin(),
        );
        systems.extend(picks(vec![aboard]));
    }
    let system = systems.into_iter().find(|&s| {
        galaxy
            .system(s)
            .is_some_and(|r| gathered == 4 || r.values[29] / 2 <= gathered)
    })?;
    p.target_character = abduction_character(planner, world, galaxy, system);
    p.target_character.map(|_| system)
}

/// Whether `character` is a prisoner of the side (record `+0x30` 2 and
/// `0x40`, `FUN_00401d20`).
fn own_prisoner(planner: &Planner<'_>, character: CharacterKey) -> bool {
    planner
        .flags(crate::missions::MissionMember::Character(character))
        .is_some_and(|f| f & 0x42 == 0x42)
}

/// `FUN_0047ecf0`: a random prisoner of the side (record `+0x30` 0x40 and
/// 2) at `system`: on the planet when its record has `+0x2c` `0x4000`,
/// and aboard a fleet there not the side's carrying one (fleet record
/// `+0x38` `0x1000000`) when it has `0x8000`; the draw is
/// `FUN_0041cd80(count)`.
///
/// port: the candidates are taken in the world's character order.
fn rescue_character(
    planner: &mut Planner<'_>,
    world: &GameWorld,
    galaxy: &crate::ai_records::Galaxy,
    system: SystemKey,
) -> Option<CharacterKey> {
    let record = galaxy.system(system)?;
    let sys = world.systems.get(system)?;
    let mut candidates: Vec<CharacterKey> = Vec::new();
    if record.summary & 0x4000 != 0 {
        candidates.extend(world.characters.iter().filter_map(|(key, c)| {
            (!c.is_killed
                && c.current_system == Some(system)
                && c.current_fleet.is_none()
                && own_prisoner(planner, key))
            .then_some(key)
        }));
    }
    if record.summary & 0x8000 != 0 {
        for &fleet in &sys.fleets {
            let carrying = galaxy.fleets.get(&fleet).is_some_and(|r| {
                r.flags & crate::ai_records::FLEET_OWN == 0 && r.flags & 0x100_0000 != 0
            });
            if !carrying {
                continue;
            }
            candidates.extend(world.characters.iter().filter_map(|(key, c)| {
                (!c.is_killed && c.current_fleet == Some(fleet) && own_prisoner(planner, key))
                    .then_some(key)
            }));
        }
    }
    if candidates.is_empty() {
        return None;
    }
    let count = i32::try_from(candidates.len()).unwrap_or(i32::MAX);
    let at = usize::try_from(planner.random(count)).unwrap_or(0);
    candidates.get(at).copied()
}

/// State 5 of a Rescue planner (slot `+0x50`, `FUN_0047e620`): its target
/// character `+0x30` and system `+0x34`, chosen anew each cycle.
///
/// The test that would keep the character reads `(flags | 0x20) == 0`
/// (`or eax, 0x20` at `0x47e800`), which never holds, so it never keeps.
/// It lists the systems holding a prisoner of the side: the sectors with
/// one (`+0x28` `0x4000`, key `+0x180`, order 1) in turn, and in each the
/// systems with one on the planet (`+0x2c` `0x4000`, key `v[29]`, order 1),
/// then those with one aboard a fleet (`0x8000`, order 2). It takes the
/// first where the decoys gathered at `+0x38` (`FUN_004bd000`) fill the
/// decoy cap (slot `+0x44`, `FUN_004047d0`: 4) or reach half its defense
/// (record `+0xbc`), then a prisoner there ([`rescue_character`]).
fn rescue_target(
    p: &mut MissionPlanner,
    planner: &mut Planner<'_>,
    world: &GameWorld,
    galaxy: &crate::ai_records::Galaxy,
) -> Option<SystemKey> {
    use crate::ai_records::{picks, sector_at, Mask, Order};
    use crate::mission_planning::standing_system;
    let any = Mask::new(0, 0);
    let gathered = p
        .decoys
        .iter()
        .filter(|&&m| p.staging.is_some() && standing_system(world, m) == p.staging)
        .filter(|&&m| planner.sendable(m))
        .count();
    let gathered = i32::try_from(gathered).unwrap_or(i32::MAX);
    p.target_character = None;
    let sectors = picks(vec![galaxy.sector_query(
        [any, Mask::new(0x4000, 0), any],
        sector_at(0x180),
        Order::Ascending,
        &mut || planner.coin(),
    )]);
    let mut systems = Vec::new();
    for sector in sectors {
        let planet = galaxy.system_query(
            sector,
            [any, Mask::new(0x4000, 0), any],
            29,
            Order::Ascending,
            &mut || planner.coin(),
        );
        systems.extend(picks(vec![planet]));
        let aboard = galaxy.system_query(
            sector,
            [any, Mask::new(0x8000, 0), any],
            29,
            Order::Descending,
            &mut || planner.coin(),
        );
        systems.extend(picks(vec![aboard]));
    }
    let system = systems.into_iter().find(|&s| {
        galaxy
            .system(s)
            .is_some_and(|r| gathered == 4 || r.values[29] / 2 <= gathered)
    })?;
    p.target_character = rescue_character(planner, world, galaxy, system);
    p.target_character.map(|_| system)
}

/// Row 6's want (`FUN_004e8290`): the side's uprisings plus one (side
/// `+0x21c`, the sector records' `+0x164`: systems with record `+0x2c` bit
/// `0x20`, an own uprising with no own fleet; `FUN_0041b7e0`) while
/// controller bit `0x100` is set, else none. The bit is set when a sector
/// record has `+0x28` bit `0x8000000`, an own system below 70 support.
fn subdue_wanted(galaxy: &crate::ai_records::Galaxy) -> usize {
    let flagged = galaxy.sectors.iter().any(|s| s.state & 0x800_0000 != 0);
    if !flagged {
        return 0;
    }
    let uprisings: i32 = galaxy
        .sectors
        .iter()
        .map(|s| s.values[crate::ai_records::sector_at(0x164)])
        .sum();
    usize::try_from(uprisings + 1).unwrap_or(0)
}

/// State 5 of a Subdue planner (slot `+0x50`, `FUN_0047d060`): its target
/// `+0x34`, an own system in an uprising with no own fleet (record `+0x2c`
/// bit `0x20`).
///
/// Without a team it has none. With one it keeps its target while the
/// record has `0x20`. Otherwise:
///
/// - from the previous gathering place `+0x38`: the nearest sector with
///   such a system (`FUN_00419b80`, sector `+0x28` `0x10`, order 2), then
///   its such system nearest `+0x38` (`FUN_0041a400`, order 2);
/// - with no `+0x38`: the sector with the most of them (key `+0x164`,
///   order 1), then its such system with the highest support
///   (`FUN_0041a340`, key `v[0]`, order 1).
///
/// A Subdue planner claims nothing.
fn subdue_target(
    p: &MissionPlanner,
    planner: &mut Planner<'_>,
    galaxy: &crate::ai_records::Galaxy,
) -> Option<SystemKey> {
    use crate::ai_records::{pick, sector_at, Mask, Order};
    p.team.first()?;
    let any = Mask::new(0, 0);
    let revolt = [any, Mask::new(0x20, 0), any];
    if let Some(t) = p
        .target_system
        .filter(|&t| galaxy.system(t).is_some_and(|r| r.summary & 0x20 != 0))
    {
        return Some(t);
    }
    let sectors = [any, Mask::new(0x10, 0), any];
    if let Some(prev) = p.staging {
        let near =
            galaxy.sectors_by_distance(prev, sectors, Order::Descending, &mut || planner.coin());
        let sector = pick(&[near])?;
        let systems =
            galaxy.systems_by_distance(sector, prev, revolt, Order::Descending, &mut || {
                planner.coin()
            });
        return pick(&[systems]);
    }
    let most = galaxy.sector_query(sectors, sector_at(0x164), Order::Ascending, &mut || {
        planner.coin()
    });
    let sector = pick(&[most])?;
    let systems = galaxy.system_query(sector, revolt, 0, Order::Ascending, &mut || planner.coin());
    pick(&[systems])
}

/// Side `+0x218` (`FUN_0041b7e0`, `FUN_00418500`): the Incite planners the
/// strategy wants. The sector records' `+0x178` (their systems with record
/// `+0x2c` bit `0x100`: the opponent's, where the side's support is above
/// 50) summed, then `/ 6 + 1` when above 0.
fn incite_wanted(galaxy: &crate::ai_records::Galaxy) -> usize {
    let sum: i32 = galaxy
        .sectors
        .iter()
        .map(|s| s.values[crate::ai_records::sector_at(0x178)])
        .sum();
    let wanted = if sum > 0 { sum / 6 + 1 } else { sum };
    usize::try_from(wanted).unwrap_or(0)
}

/// State 5 of an Incite planner (slot `+0x50`, `FUN_00480370`): its target
/// `+0x34`, an opponent's system whose record has `+0x2c` bit `0x100`.
///
/// Without a team it has none. With one it keeps its target while the
/// record still has `0x100` and no enemy fleet (`+0x30` bit `0x200`).
/// Otherwise, the first that gives one:
///
/// 1. from the previous gathering place `+0x38`: the nearest sector with
///    such a system and opponent systems (`FUN_00419b80`, `+0x28`
///    `0x200`, `+0x2c` 8, order 2), then its such system nearest `+0x38`
///    (`FUN_0041a400`, `+0x2c` `0x100`, `+0x30` 4, order 2);
/// 2. the sector ranked by its opponent systems (key `+0x80`) and a draw
///    (`FUN_0041a1d0`), both order 1, then its such system with the
///    highest support of the side (`FUN_0041a340`, key `v[0]`, order 1).
///
/// An Incite planner claims nothing.
fn incite_target(
    p: &MissionPlanner,
    planner: &mut Planner<'_>,
    galaxy: &crate::ai_records::Galaxy,
) -> Option<SystemKey> {
    use crate::ai_records::{pick, sector_at, Mask, Order};
    p.team.first()?;
    let any = Mask::new(0, 0);
    if let Some(t) = p.target_system.filter(|&t| {
        galaxy
            .system(t)
            .is_some_and(|r| r.summary & 0x100 != 0 && r.kind & 0x200 == 0)
    }) {
        return Some(t);
    }
    let sectors = [any, Mask::new(0x200, 0), Mask::new(8, 0)];
    let systems = [any, Mask::new(0x100, 0), Mask::new(4, 0)];
    let mut target = None;
    if let Some(prev) = p.staging {
        let near =
            galaxy.sectors_by_distance(prev, sectors, Order::Descending, &mut || planner.coin());
        let sector = pick(&[near]);
        target = sector.and_then(|sector| {
            pick(&[galaxy.systems_by_distance(
                sector,
                prev,
                systems,
                Order::Descending,
                &mut || planner.coin(),
            )])
        });
    }
    if target.is_none() {
        let most = galaxy.sector_query(sectors, sector_at(0x80), Order::Ascending, &mut || {
            planner.coin()
        });
        let drawn = random_sectors(planner, galaxy, sectors, Order::Ascending);
        let sector = pick(&[most, drawn]);
        target = sector.and_then(|sector| {
            pick(&[
                galaxy.system_query(sector, systems, 0, Order::Ascending, &mut || planner.coin())
            ])
        });
    }
    target
}

/// State 5 of a Recruitment planner (slot `+0x50`, `FUN_0047dcc0`): its
/// target `+0x34`, a system whose record has `+0x2c` bit 2 (the side's,
/// with no uprising or blockade, while the side has MNCHARSD classes).
///
/// Without a team it has none. With one it keeps its target while that
/// still has bit 2. Otherwise it takes, the first that gives one:
///
/// 1. the system where the first team member stands (the walk up its
///    containers, `FUN_004f2d10`), when it has bit 2;
/// 2. from the previous gathering place `+0x38`: the nearest sector with
///    such a system (`FUN_00419b80`, sector `+0x28` 8, order 2), then its
///    one nearest `+0x38` (`FUN_0041a400`, order 2);
/// 3. with no `+0x38`: the sector with the most such systems (key
///    `+0x160`, `FUN_00419a00` order 1), then its one with the highest
///    support (`FUN_0041a340`, key `v[0]`, order 1).
///
/// A Recruitment planner claims nothing.
fn recruitment_target(
    p: &MissionPlanner,
    planner: &mut Planner<'_>,
    world: &GameWorld,
    galaxy: &crate::ai_records::Galaxy,
) -> Option<SystemKey> {
    use crate::ai_records::{pick, sector_at, Mask, Order};
    let first = *p.team.first()?;
    let any = Mask::new(0, 0);
    let calm = |system: SystemKey| galaxy.system(system).is_some_and(|r| r.summary & 2 != 0);
    if let Some(t) = p.target_system.filter(|&t| calm(t)) {
        return Some(t);
    }
    if let Some(here) = crate::mission_planning::standing_system(world, first).filter(|&s| calm(s))
    {
        return Some(here);
    }
    let mut coin = || planner.coin();
    let systems = Mask::new(2, 0);
    if let Some(prev) = p.staging {
        let sectors = galaxy.sectors_by_distance(
            prev,
            [any, Mask::new(8, 0), any],
            Order::Descending,
            &mut coin,
        );
        let sector = pick(&[sectors])?;
        let near = galaxy.systems_by_distance(
            sector,
            prev,
            [any, systems, any],
            Order::Descending,
            &mut coin,
        );
        return pick(&[near]);
    }
    let sectors = galaxy.sector_query(
        [any, Mask::new(8, 0), any],
        sector_at(0x160),
        Order::Ascending,
        &mut coin,
    );
    let sector = pick(&[sectors])?;
    let best = galaxy.system_query(sector, [any, systems, any], 0, Order::Ascending, &mut coin);
    pick(&[best])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::SectorGroup;
    use crate::ids::{DatId, SectorKey};
    use crate::manufacturing::ManufacturingState;
    use crate::missions::MissionState;
    use crate::tick::TickEvent;
    use crate::tuning::GameConfig;
    use crate::world::{
        CapitalShipClass, Character, ControlKind, FighterClass, Fleet, GameWorld, Sector,
        ShipInstance, SkillPair, System,
    };

    #[test]
    fn every_character_a_mission_frees_leaves_the_busy_set() {
        use crate::missions::{MissionEffect, MissionMember};
        let mut keys: slotmap::SlotMap<CharacterKey, ()> = slotmap::SlotMap::with_key();
        let (lead, second, idle) = (keys.insert(()), keys.insert(()), keys.insert(()));
        let mut units: slotmap::SlotMap<crate::ids::SpecialForceKey, ()> =
            slotmap::SlotMap::with_key();
        let mut ai = AIState::new(AiFaction::Empire);
        for character in [lead, second, idle] {
            ai.mark_busy(character);
        }

        ai.free_mission_members(&[
            MissionEffect::MemberAvailable {
                member: MissionMember::Character(lead),
            },
            MissionEffect::MemberAvailable {
                member: MissionMember::SpecialForce(units.insert(())),
            },
            MissionEffect::MemberAvailable {
                member: MissionMember::Character(second),
            },
        ]);

        assert!(!ai.is_busy(lead));
        assert!(!ai.is_busy(second));
        assert!(ai.is_busy(idle));
    }

    // -----------------------------------------------------------------------
    // World builder helpers
    // -----------------------------------------------------------------------

    fn empty_world() -> GameWorld {
        GameWorld::default()
    }

    fn add_sector(world: &mut GameWorld) -> SectorKey {
        world.sectors.insert(Sector {
            dat_id: DatId(0),
            name: "Test Sector".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        })
    }

    fn add_system(
        world: &mut GameWorld,
        sector: SectorKey,
        pop_alliance: f32,
        pop_empire: f32,
    ) -> SystemKey {
        world.systems.insert(System {
            dat_id: DatId(0),
            name: "Test System".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: pop_alliance,
            popularity_empire: pop_empire,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Uncontrolled,
        })
    }

    fn add_character(
        world: &mut GameWorld,
        is_alliance: bool,
        is_major: bool,
        diplomacy_base: u32,
    ) -> CharacterKey {
        world.characters.insert(Character {
            name: "TestChar".into(),
            is_alliance,
            is_empire: !is_alliance,
            is_major,
            diplomacy: SkillPair {
                base: diplomacy_base,
                variance: 0,
            },
            leadership: SkillPair {
                base: 50,
                variance: 0,
            },
            can_be_commander: true,
            // A placed character belongs to its side's roster (+0x50 bit 2).
            recruited: true,
            ..Default::default()
        })
    }

    /// port: with no ship class to build, the AI orders regiments, then
    /// defenses, then construction yards, naming the class of one it owns.
    #[test]
    fn the_ai_orders_ground_and_facility_classes_it_already_fields() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let yard =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId(0x2a00_0003),
                    side: crate::dat::Faction::Empire,
                    is_shipyard: false,
                });
        // Regiments are a training facility's product (FUN_00527d00 refuses
        // a Build with no yard of its area).
        let training =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId(0x2900_0002),
                    side: crate::dat::Faction::Empire,
                    is_shipyard: false,
                });
        let sys_key = world.systems.insert(System {
            dat_id: DatId(0),
            name: "Coruscant".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.1,
            popularity_empire: 0.9,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![yard, training],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Empire),
        });
        let trooper = |class| crate::world::TroopUnit {
            class_dat_id: DatId(class),
            is_alliance: false,
            regiment_strength: 100,
        };
        // An Alliance regiment first, so the Empire's must be found by side.
        world.troops.insert(crate::world::TroopUnit {
            is_alliance: true,
            ..trooper(0x1000_0001)
        });
        world.troops.insert(trooper(0x1000_0006));
        world
            .defense_facilities
            .insert(crate::world::DefenseFacilityInstance {
                class_dat_id: DatId(0x2200_0001),
                side: crate::dat::Faction::Empire,
            });
        let ordered = |world: &GameWorld| {
            evaluate(
                &mut AIState::new(AiFaction::Empire),
                world,
                &ManufacturingState::new(),
                &MissionState::new(),
                &ticks(7),
                &GameConfig::default(),
                &crate::research::ResearchState::new(),
                &crate::uprising::UprisingState::default(),
            )
            .into_iter()
            .find_map(|action| match action {
                AIAction::EnqueueProduction { system, kind, .. } if system == sys_key => Some(kind),
                _ => None,
            })
        };
        let empire = |class| FacilityBuild {
            class: DatId(class),
            is_alliance: false,
        };

        assert_eq!(
            ordered(&world),
            Some(BuildableKind::Troop(DatId(0x1000_0006)))
        );
        for _ in 0..2 {
            let key = world.troops.insert(trooper(0x1000_0006));
            world.systems[sys_key].ground_units.push(key);
        }
        assert_eq!(
            ordered(&world),
            Some(BuildableKind::DefenseFacility(empire(0x2200_0001)))
        );
        for _ in 0..2 {
            let key = world
                .defense_facilities
                .insert(crate::world::DefenseFacilityInstance {
                    class_dat_id: DatId(0x2200_0001),
                    side: crate::dat::Faction::Empire,
                });
            world.systems[sys_key].defense_facilities.push(key);
        }
        assert_eq!(
            ordered(&world),
            Some(BuildableKind::ManufacturingFacility(empire(0x2a00_0003)))
        );
    }

    /// What a side's AI orders at its one system, which has a construction
    /// yard and a training facility (both research order 1 on the Facility
    /// tree), a regiment of a Troop-order-2 class and a defense of a
    /// Facility-order-3 class, at the given research levels. The other side
    /// has researched everything.
    fn ground_order_at_levels(
        is_alliance: bool,
        troop: u32,
        facility: u32,
    ) -> Option<BuildableKind> {
        let (world, sys_key, research, faction) = ground_world(is_alliance, troop, facility);
        evaluate(
            &mut AIState::new(faction),
            &world,
            &ManufacturingState::new(),
            &MissionState::new(),
            &ticks(7),
            &GameConfig::default(),
            &research,
            &crate::uprising::UprisingState::default(),
        )
        .into_iter()
        .find_map(|action| match action {
            AIAction::EnqueueProduction { system, kind, .. } if system == sys_key => Some(kind),
            _ => None,
        })
    }

    /// The world of [`ground_order_at_levels`], its system, research and
    /// side.
    fn ground_world(
        is_alliance: bool,
        troop: u32,
        facility: u32,
    ) -> (GameWorld, SystemKey, ResearchState, AiFaction) {
        use crate::world::BuildableClass;
        let side = crate::dat::Faction::of_alliance(is_alliance);
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let mut catalog = |id: u32, order: u32| {
            world.buildable_classes.insert(
                DatId(id),
                BuildableClass {
                    is_alliance: true,
                    is_empire: true,
                    research_order: order,
                    ..BuildableClass::default()
                },
            );
        };
        catalog(0x2a00_0003, 1);
        catalog(0x2900_0002, 1);
        catalog(0x1000_0002, 2);
        catalog(0x2200_0001, 3);
        let facility_of = |world: &mut GameWorld, class| {
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId(class),
                    side,
                    is_shipyard: false,
                })
        };
        let yard = facility_of(&mut world, 0x2a00_0003);
        let training = facility_of(&mut world, 0x2900_0002);
        let sys_key = world.systems.insert(System {
            dat_id: DatId(0),
            name: "Home".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![yard, training],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(side),
        });
        world.troops.insert(crate::world::TroopUnit {
            class_dat_id: DatId(0x1000_0002),
            is_alliance,
            regiment_strength: 100,
        });
        world
            .defense_facilities
            .insert(crate::world::DefenseFacilityInstance {
                class_dat_id: DatId(0x2200_0001),
                side,
            });
        let mut research = crate::research::ResearchState::new();
        let all = crate::research::ResearchLevels {
            ship: 9,
            troop: 9,
            facility: 9,
        };
        let own = crate::research::ResearchLevels {
            ship: 0,
            troop,
            facility,
        };
        (research.alliance, research.empire) = if is_alliance { (own, all) } else { (all, own) };
        let faction = if is_alliance {
            AiFaction::Alliance
        } else {
            AiFaction::Empire
        };
        (world, sys_key, research, faction)
    }

    #[test]
    fn module_a_issues_the_heuristic_s_manufacturing_facilities() {
        // port: the production heuristic's facilities stand in for module
        // 0xa's shipyard placement (FUN_004cc9c0); its other modules' kinds
        // are not 0xa's.
        let (world, sys_key, research, faction) = ground_world(true, 0, 2);
        let (missions, mfg, config, uprisings) = (
            MissionState::new(),
            ManufacturingState::new(),
            GameConfig::default(),
            crate::uprising::UprisingState::default(),
        );
        let ctx = ModuleInputs {
            faction,
            ..inputs(&world, &missions, &research, &mfg, &config, &uprisings)
        };
        let busy = HashSet::new();
        let mut planner = Planner::new(&world, &missions, &busy, true, 7);
        let mut actions = Vec::new();
        AISystem::run_module(
            0xa,
            &mut AIState::new(faction),
            &mut planner,
            &ctx,
            &mut crate::ai_records::Galaxy::default(),
            &mut actions,
        );
        assert!(matches!(
            actions.as_slice(),
            [AIAction::EnqueueProduction {
                system,
                kind: BuildableKind::ManufacturingFacility(_),
                ..
            }] if *system == sys_key
        ));
    }

    /// hyp: the AI waits on the same DAT `research_order` gate Build Selection
    /// applies to the player's lists (F-018), each class on its own tree.
    #[test]
    fn the_ai_orders_no_regiment_or_facility_class_its_side_has_not_researched() {
        for is_alliance in [true, false] {
            let build = |class| FacilityBuild {
                class: DatId(class),
                is_alliance,
            };
            // Nothing researched: the other side's levels do not count.
            assert_eq!(ground_order_at_levels(is_alliance, 0, 0), None);
            // The regiment class waits on the Troop tree alone.
            assert_eq!(ground_order_at_levels(is_alliance, 1, 0), None);
            assert_eq!(
                ground_order_at_levels(is_alliance, 2, 0),
                Some(BuildableKind::Troop(DatId(0x1000_0002)))
            );
            // The defense class waits on the Facility tree alone; below its
            // order the yard class, already researched, is ordered instead.
            assert_eq!(
                ground_order_at_levels(is_alliance, 0, 3),
                Some(BuildableKind::DefenseFacility(build(0x2200_0001)))
            );
            assert_eq!(
                ground_order_at_levels(is_alliance, 0, 2),
                Some(BuildableKind::ManufacturingFacility(build(0x2a00_0003)))
            );
            assert_eq!(
                ground_order_at_levels(is_alliance, 0, 1),
                Some(BuildableKind::ManufacturingFacility(build(0x2a00_0003)))
            );
        }
    }

    fn ticks(n: u64) -> Vec<TickEvent> {
        (1..=n).map(|t| TickEvent { tick: t }).collect()
    }

    /// Every module once and every research planner, with no order budget:
    /// the heuristics as one evaluation on the last of `tick_events`.
    #[expect(clippy::too_many_arguments, reason = "The inputs of a pass.")]
    fn evaluate(
        state: &mut AIState,
        world: &GameWorld,
        mfg_state: &ManufacturingState,
        mission_state: &MissionState,
        tick_events: &[TickEvent],
        config: &GameConfig,
        research_state: &ResearchState,
        uprisings: &crate::uprising::UprisingState,
    ) -> Vec<AIAction> {
        let (Some(last), Some(faction)) = (tick_events.last(), state.faction) else {
            return Vec::new();
        };
        let busy = state.busy_characters.clone();
        let mut planner = Planner::new(
            world,
            mission_state,
            &busy,
            matches!(faction, AiFaction::Alliance),
            last.tick,
        );
        let ctx = ModuleInputs {
            world,
            mfg_state,
            mission_state,
            transport: Box::leak(Box::default()),
            tick: last.tick,
            config,
            research_state,
            uprisings,
            faction,
        };
        let mut actions = Vec::new();
        research_all(state, &mut planner, &ctx, &mut actions);
        AISystem::evaluate_espionage(&mut planner, world, faction, config, &mut actions);
        AISystem::evaluate_production(
            world,
            mfg_state,
            research_state,
            faction,
            config,
            &mut actions,
        );
        AISystem::run_module(
            8,
            state,
            &mut planner,
            &ctx,
            &mut crate::ai_records::Galaxy::default(),
            &mut actions,
        );
        AISystem::run_module(
            0xc,
            state,
            &mut planner,
            &ctx,
            &mut crate::ai_records::Galaxy::default(),
            &mut actions,
        );
        actions
    }

    /// The research strategy, the planners' holds, and every planner's
    /// cycle at once.
    fn research_all(
        state: &mut AIState,
        planner: &mut Planner<'_>,
        ctx: &ModuleInputs<'_>,
        actions: &mut Vec<AIAction>,
    ) {
        AISystem::research_strategy(
            state,
            planner,
            ctx.world,
            ctx.mission_state,
            ctx.research_state,
            ctx.uprisings,
            ctx.faction,
            actions,
        );
        for p in &state.research_planners {
            if let Some(c) = p.member {
                planner.hold(MissionMember::Character(c));
            }
        }
        let is_alliance = matches!(ctx.faction, AiFaction::Alliance);
        for p in &mut state.research_planners {
            AISystem::research_cycle(
                p,
                planner,
                ctx.world,
                ctx.research_state,
                ctx.uprisings,
                is_alliance,
                actions,
            );
        }
    }

    fn inputs<'a>(
        world: &'a GameWorld,
        missions: &'a MissionState,
        research: &'a ResearchState,
        mfg: &'a ManufacturingState,
        config: &'a GameConfig,
        uprisings: &'a crate::uprising::UprisingState,
    ) -> ModuleInputs<'a> {
        ModuleInputs {
            world,
            mfg_state: mfg,
            mission_state: missions,
            transport: Box::leak(Box::default()),
            tick: 7,
            config,
            research_state: research,
            uprisings,
            faction: AiFaction::Alliance,
        }
    }

    // -----------------------------------------------------------------------
    // AIState tests
    // -----------------------------------------------------------------------

    #[test]
    fn busy_character_can_be_marked_and_cleared() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let _ = add_system(&mut world, sector, 0.5, 0.5);
        let char_key = add_character(&mut world, true, false, 30);

        let mut state = AIState::new(AiFaction::Alliance);
        assert!(!state.is_busy(char_key));
        state.mark_busy(char_key);
        assert!(state.is_busy(char_key));
        state.mark_available(char_key);
        assert!(!state.is_busy(char_key));
    }

    // -----------------------------------------------------------------------
    // No-op conditions
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Officer heuristics
    // -----------------------------------------------------------------------

    /// The characters of the first `DispatchMission` of `kind`'s team.
    fn team_of(
        actions: &[AIAction],
        kind: MissionKind,
    ) -> Option<Vec<crate::missions::MissionMember>> {
        actions.iter().find_map(|a| match a {
            AIAction::DispatchMission { kind: k, team, .. } if *k == kind => Some(team.clone()),
            _ => None,
        })
    }

    #[test]
    fn a_recruiter_needs_leadership_of_80_and_someone_left_to_recruit() {
        // FUN_0047db40: one member, leadership 80..=10000. FUN_004e8290
        // row 1 wants Recruitment planners only while the side has
        // MNCHARSD classes (side +0xa4, FUN_0042dd00).
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let sys = add_system(&mut world, sector, 0.1, 0.9);
        world.systems[sys].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let recruiter = add_character(&mut world, false, true, 30);
        world.characters[recruiter].leadership.base = 80;
        let weaker = add_character(&mut world, false, true, 30);
        world.characters[weaker].leadership.base = 79;
        for c in [recruiter, weaker] {
            world.characters[c].current_system = Some(sys);
        }
        let mut state = AIState::new(AiFaction::Empire);

        assert_eq!(
            team_of(
                &strategy_pass(&mut state, &world, 1),
                MissionKind::Recruitment
            ),
            None
        );

        let recruit = add_character(&mut world, false, false, 30);
        world.characters[recruit].recruited = false;
        world.characters[recruit].dat_id = DatId(0x3800_0001);
        let mut state = AIState::new(AiFaction::Empire);
        let team = team_of(
            &strategy_pass(&mut state, &world, 1),
            MissionKind::Recruitment,
        );

        assert_eq!(
            team,
            Some(vec![crate::missions::MissionMember::Character(recruiter)])
        );
    }

    /// The planners of `kind`, in order.
    fn only(state: &AIState, kind: MissionKind) -> Vec<MissionPlanner> {
        state
            .mission_planners
            .iter()
            .filter(|p| p.kind == kind)
            .cloned()
            .collect()
    }

    /// One day of the standing planners: the side's records (phase 4), the
    /// strategy's rows 1 and 5 and every mission planner's cycle.
    fn strategy_pass(state: &mut AIState, world: &GameWorld, tick: u64) -> Vec<AIAction> {
        strategy_pass_in(
            state,
            world,
            tick,
            &crate::uprising::UprisingState::default(),
        )
    }

    /// [`strategy_pass`] with `uprisings` in progress.
    fn strategy_pass_in(
        state: &mut AIState,
        world: &GameWorld,
        tick: u64,
        uprisings: &crate::uprising::UprisingState,
    ) -> Vec<AIAction> {
        let missions = MissionState::new();
        let busy = HashSet::new();
        let economy = crate::economy::EconomyState::default();
        let manufacturing = ManufacturingState::new();
        let research = ResearchState::new();
        let movement = crate::movement::MovementState::new();
        let transport = crate::troop_transport::TroopTransportState::new();
        let is_alliance = matches!(state.faction, Some(AiFaction::Alliance));
        let mut planner = Planner::new(world, &missions, &busy, is_alliance, 7);
        for p in &state.mission_planners {
            for &member in p.team.iter().chain(&p.decoys) {
                planner.hold(member);
            }
        }
        let inputs = RecordInputs {
            world,
            economy: &economy,
            uprisings,
            manufacturing: &manufacturing,
            research: &research,
            movement: &movement,
            transport: &transport,
        };
        let mut galaxy = AISystem::records(state, &planner, &inputs);
        AISystem::mission_strategy(state, world, &mut galaxy);
        let mut actions = Vec::new();
        AISystem::mission_planner_cycle(
            state,
            &mut planner,
            world,
            &mut galaxy,
            tick,
            &mut actions,
        );
        actions
    }

    /// An Empire system at `support` (percent) where `diplomats` (by
    /// diplomacy) stand, and a neutral system, both explored, in one
    /// sector.
    fn diplomacy_world(
        support: f32,
        diplomats: &[u32],
    ) -> (GameWorld, SystemKey, SystemKey, Vec<CharacterKey>) {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let home = add_system(&mut world, sector, 0.1, support / 100.0);
        world.systems[home].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let neutral = add_system(&mut world, sector, 0.4, 0.4);
        let keys = diplomats
            .iter()
            .map(|&d| {
                // Major, so the side has no MNCHARSD class and no
                // Recruitment planner (side +0xa4 = 0).
                let c = add_character(&mut world, false, true, d);
                world.characters[c].current_system = Some(home);
                c
            })
            .collect();
        (world, home, neutral, keys)
    }

    #[test]
    fn a_diplomat_needs_diplomacy_of_69() {
        // FUN_004815a0: one member, diplomacy 69..=10000.
        let (world, _, _, _) = diplomacy_world(90.0, &[68]);
        let mut state = AIState::new(AiFaction::Empire);

        assert_eq!(
            team_of(
                &strategy_pass(&mut state, &world, 1),
                MissionKind::Diplomacy
            ),
            None
        );

        let (world, _, _, keys) = diplomacy_world(90.0, &[69]);
        let mut state = AIState::new(AiFaction::Empire);
        let team = team_of(
            &strategy_pass(&mut state, &world, 1),
            MissionKind::Diplomacy,
        );

        assert_eq!(
            team,
            Some(vec![crate::missions::MissionMember::Character(keys[0])])
        );
    }

    #[test]
    fn a_diplomat_is_sent_only_where_the_diplomacy_record_admits_the_target() {
        // MISSNSD Diplomacy (cols 15..17): own and third-side targets, no
        // opponent's (FUN_00523450 rule 3, end 8). FUN_00481720 takes only
        // a system with record +0x2c bit 0x40, own or neutral
        // (FUN_00433e40). The opponent's system is the least popular, yet
        // the neutral one is the target.
        let (mut world, _, neutral, _) = diplomacy_world(90.0, &[90]);
        let sector = world.systems[neutral].sector;
        let rebel = add_system(&mut world, sector, 0.9, 0.1);
        world.systems[rebel].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let mut state = AIState::new(AiFaction::Empire);

        let actions = strategy_pass(&mut state, &world, 1);

        let targets: Vec<_> = orders(&actions, MissionKind::Diplomacy)
            .into_iter()
            .map(|(_, target, _)| target)
            .collect();
        assert_eq!(targets, vec![neutral]);
    }

    #[test]
    fn the_strategy_keeps_one_diplomacy_planner_and_one_more_per_five_candidates() {
        // FUN_004e8290 row 5: a request while +0x34c < +0x214, a
        // cancellation while above; +0x214 is the candidates (record +0x2c
        // 0x40) / 5 + 1 when any (FUN_0041b7e0, FUN_00418500).
        let count = |candidates: usize| {
            let (mut world, home, neutral, _) = diplomacy_world(90.0, &[]);
            let sector = world.systems[home].sector;
            world.systems.remove(neutral);
            for _ in 0..candidates {
                add_system(&mut world, sector, 0.4, 0.4);
            }
            let mut state = AIState::new(AiFaction::Empire);
            strategy_pass(&mut state, &world, 1);
            state
                .mission_planners
                .iter()
                .filter(|p| p.standing && p.kind == MissionKind::Diplomacy)
                .count()
        };
        assert_eq!([0, 1, 4, 5, 9, 10].map(count), [0, 1, 1, 2, 2, 3]);
    }

    #[test]
    fn a_surplus_diplomacy_planner_with_the_weakest_team_is_cancelled_and_frees_its_claim() {
        // FUN_004e5710: the least slot +0x18 (FUN_00481500, the team's
        // diplomacy); FUN_00481fe0 releases the record's claim.
        let (world, _, neutral, keys) = diplomacy_world(90.0, &[90]);
        let mut state = AIState::new(AiFaction::Empire);
        strategy_pass(&mut state, &world, 1);
        assert_eq!(state.record_claims.get(&neutral), Some(&0x2000_0000));
        let weak = MissionPlanner {
            id: 99,
            team: Vec::new(),
            ..state.mission_planners[0].clone()
        };
        state.mission_planners.insert(0, weak);
        let strong = state.mission_planners[1].clone();
        strategy_pass(&mut state, &world, 2);
        let diplomats = only(&state, MissionKind::Diplomacy);
        assert_eq!(diplomats.len(), 1);
        assert_eq!(diplomats[0].id, strong.id);
        assert_eq!(diplomats[0].team, vec![MissionMember::Character(keys[0])]);

        // With no candidate left, the last planner goes and frees its claim.
        let mut world = world;
        world.systems[neutral].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        world.systems[neutral].popularity_empire = 0.9;
        strategy_pass(&mut state, &world, 3);
        assert!(only(&state, MissionKind::Diplomacy).is_empty());
        assert!(state.record_claims.is_empty());
    }

    #[test]
    fn a_diplomacy_planner_trades_its_diplomat_for_a_better_free_one() {
        // FUN_0047ba40 asks slot +0x3c (FUN_00481ef0) in state 2: a free
        // diplomat (69..) with more diplomacy (record +0x40) makes the
        // planner let its own go, and state 3 takes the better one.
        let (mut world, home, _, keys) = diplomacy_world(90.0, &[80]);
        let mut state = AIState::new(AiFaction::Empire);
        strategy_pass(&mut state, &world, 1);
        let team = |state: &AIState| only(state, MissionKind::Diplomacy)[0].team.clone();
        assert_eq!(team(&state), vec![MissionMember::Character(keys[0])]);
        // An equal one does not outrank it.
        let equal = add_character(&mut world, false, true, 80);
        world.characters[equal].current_system = Some(home);
        strategy_pass(&mut state, &world, 2);
        assert_eq!(team(&state), vec![MissionMember::Character(keys[0])]);
        let better = add_character(&mut world, false, true, 95);
        world.characters[better].current_system = Some(home);
        strategy_pass(&mut state, &world, 3);
        assert_eq!(team(&state), vec![MissionMember::Character(better)]);
    }

    #[test]
    fn a_diplomacy_planner_takes_an_own_system_below_70_before_a_neutral_one() {
        // FUN_00481720: first the sector of the lowest average own support
        // (key +0x8c) when it is below 70, and an own candidate there.
        let (world, home, neutral, _) = diplomacy_world(50.0, &[90]);
        let mut state = AIState::new(AiFaction::Empire);
        let targets = |actions: &[AIAction]| -> Vec<SystemKey> {
            orders(actions, MissionKind::Diplomacy)
                .into_iter()
                .map(|(_, target, _)| target)
                .collect()
        };
        assert_eq!(targets(&strategy_pass(&mut state, &world, 1)), vec![home]);

        let (world, _, _, _) = diplomacy_world(70.0, &[90]);
        let mut state = AIState::new(AiFaction::Empire);
        assert_eq!(
            targets(&strategy_pass(&mut state, &world, 1)),
            vec![neutral]
        );
    }

    #[test]
    fn a_diplomacy_planner_keeps_its_claimed_target_and_another_takes_the_next() {
        // FUN_00481720 keeps +0x34 while its record has +0x2c 0x40 and no
        // enemy fleet; the claim (+0x2c 0x20000000) keeps the second
        // planner's queries off it at once. The home system is the only
        // own candidate (support 30), so without the claim both would take
        // it; four neutral candidates in another sector make five, two
        // planners.
        let (mut world, home, neutral, _) = diplomacy_world(30.0, &[90, 80]);
        let far = add_sector(&mut world);
        world.sectors[far].x = 500;
        world.systems[neutral].sector = far;
        let neutrals: Vec<SystemKey> = std::iter::once(neutral)
            .chain((0..3).map(|_| add_system(&mut world, far, 0.4, 0.4)))
            .collect();
        let mut state = AIState::new(AiFaction::Empire);
        let first = strategy_pass(&mut state, &world, 1);
        let targets: Vec<SystemKey> = state
            .mission_planners
            .iter()
            .filter_map(|p| p.target_system)
            .collect();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0], home);
        assert!(neutrals.contains(&targets[1]));
        assert_eq!(orders(&first, MissionKind::Diplomacy).len(), 2);
        for t in &targets {
            assert_eq!(state.record_claims.get(t), Some(&0x2000_0000));
            assert_eq!(
                state
                    .mission_planners
                    .iter()
                    .filter(|p| p.claimed == Some(*t))
                    .count(),
                1
            );
        }

        // The next day each keeps its own.
        strategy_pass(&mut state, &world, 2);
        let kept: Vec<SystemKey> = state
            .mission_planners
            .iter()
            .filter_map(|p| p.target_system)
            .collect();
        assert_eq!(kept, targets);

        // Once its target is no candidate (own at 90), the planner takes
        // another and frees the old claim.
        world.systems[home].popularity_empire = 0.9;
        strategy_pass(&mut state, &world, 3);
        assert_ne!(state.mission_planners[0].target_system, Some(home));
        assert!(!state.record_claims.contains_key(&home));
    }

    #[test]
    fn a_diplomacy_planner_without_a_team_frees_its_claim() {
        // FUN_00481720 with +0x20 bit 1 clear: the record of +0x34 loses
        // its claim, and no target is picked.
        let (mut world, _, neutral, keys) = diplomacy_world(90.0, &[90]);
        let mut state = AIState::new(AiFaction::Empire);
        strategy_pass(&mut state, &world, 1);
        assert_eq!(state.mission_planners[0].claimed, Some(neutral));
        world.characters[keys[0]].mark_killed();
        strategy_pass(&mut state, &world, 2);
        let p = &state.mission_planners[0];
        assert!(p.team.is_empty());
        assert_eq!(p.claimed, None);
        assert_eq!(p.target_system, Some(neutral));
        assert!(state.record_claims.is_empty());
    }

    #[test]
    fn the_own_sector_counts_by_its_average_support_below_70() {
        // FUN_00481720: the sector record's +0x8c (its own systems' support
        // over their count, FUN_004321e0) must be below 70. Own systems at
        // 40 and 100 average 70, so the own candidate waits and the
        // neutral one is taken.
        let (mut world, home, neutral, _) = diplomacy_world(40.0, &[90]);
        let sector = world.systems[home].sector;
        let rich = add_system(&mut world, sector, 0.0, 1.0);
        world.systems[rich].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let mut state = AIState::new(AiFaction::Empire);
        strategy_pass(&mut state, &world, 1);
        assert_eq!(state.mission_planners[0].target_system, Some(neutral));
    }

    #[test]
    fn the_refresh_keeps_the_planners_claims_and_a_sector_with_none_open_loses_0x4000000() {
        // FUN_00433e40 keeps +0x2c & 0x60000000; FUN_00432350 sets sector
        // +0x28 0x4000000 for a system with +0x2c 0x40 and no
        // 0x20000000.
        let (world, home, neutral, _) = diplomacy_world(90.0, &[]);
        let sector = world.systems[home].sector;
        let galaxy_with = |claims: &[(SystemKey, u32)]| {
            let mut state = AIState::new(AiFaction::Empire);
            state.record_claims = claims.iter().copied().collect();
            let missions = MissionState::new();
            let busy = HashSet::new();
            let economy = crate::economy::EconomyState::default();
            let uprisings = crate::uprising::UprisingState::default();
            let manufacturing = ManufacturingState::new();
            let research = ResearchState::new();
            let movement = crate::movement::MovementState::new();
            let transport = crate::troop_transport::TroopTransportState::new();
            let planner = Planner::new(&world, &missions, &busy, false, 7);
            let inputs = RecordInputs {
                world: &world,
                economy: &economy,
                uprisings: &uprisings,
                manufacturing: &manufacturing,
                research: &research,
                movement: &movement,
                transport: &transport,
            };
            AISystem::records(&state, &planner, &inputs)
        };
        let open = |g: &crate::ai_records::Galaxy| g.sector(sector).unwrap().state & 0x400_0000;

        let g = galaxy_with(&[]);
        assert_eq!(open(&g), 0x400_0000);
        assert_eq!(g.system(neutral).unwrap().summary & 0xe000_0000, 0);

        let g = galaxy_with(&[(neutral, 0xffff_ffff)]);
        assert_eq!(
            g.system(neutral).unwrap().summary & 0xe000_0000,
            0x6000_0000
        );
        assert_eq!(open(&g), 0);

        // Another claim (0x40000000) leaves the system open.
        let g = galaxy_with(&[(neutral, 0x4000_0000)]);
        assert_eq!(open(&g), 0x400_0000);
    }

    #[test]
    fn a_claim_is_seen_at_once_and_a_planner_left_with_no_candidate_holds_no_claim() {
        // FUN_00481720 sets and clears record +0x2c 0x20000000 in place
        // (seen by the next query), and a planner whose target lost +0x2c
        // 0x40 with nothing else to take keeps no claim (+0x30).
        let (world, home, neutral, _) = diplomacy_world(90.0, &[90]);
        let mut state = AIState::new(AiFaction::Empire);
        strategy_pass(&mut state, &world, 1);
        let missions = MissionState::new();
        let busy = HashSet::new();
        let economy = crate::economy::EconomyState::default();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = ManufacturingState::new();
        let research = ResearchState::new();
        let movement = crate::movement::MovementState::new();
        let transport = crate::troop_transport::TroopTransportState::new();
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let inputs = RecordInputs {
            world: &world,
            economy: &economy,
            uprisings: &uprisings,
            manufacturing: &manufacturing,
            research: &research,
            movement: &movement,
            transport: &transport,
        };
        let mut galaxy = AISystem::records(&state, &planner, &inputs);
        let before = galaxy.system(neutral).unwrap().summary;
        assert_eq!(before & 0x2000_0000, 0x2000_0000);
        galaxy.update_summary(neutral, 0, 0x2000_0000);
        assert_eq!(
            galaxy.system(neutral).unwrap().summary,
            before & !0x2000_0000
        );
        galaxy.update_summary(neutral, 0x2000_0000, 0);
        assert_eq!(galaxy.system(neutral).unwrap().summary, before);
        galaxy.update_summary(neutral, 0x2000_0000, 0);
        assert_eq!(galaxy.system(neutral).unwrap().summary, before);

        for system in [home, neutral] {
            galaxy.update_summary(system, 0, 0x40);
        }
        let mut p = state.mission_planners[0].clone();
        let target = diplomacy_target(&mut p, &mut planner, &mut galaxy, &mut state.record_claims);
        assert_eq!(target, None);
        assert_eq!(p.claimed, None);
        assert!(state.record_claims.is_empty());
    }

    #[test]
    fn a_diplomacy_planner_aborts_its_mission_once_the_target_is_no_candidate_or_after_five_failures(
    ) {
        // FUN_0047c890: a result sets the target check (0x40000000), and a
        // failure past slot +0x24 (FUN_00404c20, 5; FUN_0047ae20) the abort
        // (0x20000000). FUN_0047b250 turns a failed check (FUN_00481eb0:
        // record +0x2c 0x40) into the abort and orders 0x250 while the
        // mission runs.
        let (mut world, _, neutral, keys) = diplomacy_world(90.0, &[90]);
        let mut state = AIState::new(AiFaction::Empire);
        strategy_pass(&mut state, &world, 1);
        let mut missions = MissionState::new();
        let id = missions.dispatch(crate::missions::MissionRequest {
            kind: MissionKind::Diplomacy,
            faction: MissionFaction::Empire,
            team: vec![MissionMember::Character(keys[0])],
            decoys: Vec::new(),
            target_system: neutral,
            target_character: None,
            target_object: None,
            tick: 1,
        });
        state.bind_mission(
            MissionKind::Diplomacy,
            MissionMember::Character(keys[0]),
            id,
        );
        assert_eq!(state.mission_planners[0].mission, Some(id));
        let result = |outcome| result(id, outcome);
        let reports = |state: &mut AIState, world: &GameWorld| {
            let galaxy = galaxy_of(state, world);
            let mut actions = Vec::new();
            AISystem::mission_planner_reports(state, &galaxy, (&missions, 1), &mut actions);
            aborted(&actions)
        };

        // A success with the target still a candidate: nothing.
        state.note_mission_results(&[result(crate::missions::MissionOutcome::Success)]);
        assert!(reports(&mut state, &world).is_empty());
        // Five failures are borne; the sixth aborts.
        for _ in 0..5 {
            state.note_mission_results(&[result(crate::missions::MissionOutcome::Failure)]);
            assert!(reports(&mut state, &world).is_empty());
        }
        state.note_mission_results(&[result(crate::missions::MissionOutcome::Failure)]);
        assert_eq!(reports(&mut state, &world), vec![id]);
        assert_eq!(state.mission_planners[0].failures, 0);

        // The target taken by the side at 90 support: the next result
        // aborts.
        world.systems[neutral].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        world.systems[neutral].popularity_empire = 0.9;
        state.note_mission_results(&[result(crate::missions::MissionOutcome::Success)]);
        assert_eq!(reports(&mut state, &world), vec![id]);
        // No report, no check.
        assert!(reports(&mut state, &world).is_empty());
    }

    #[test]
    fn a_report_binds_to_its_standing_planner_and_a_failure_also_checks_the_target() {
        // bind_mission takes the standing planner of the order's kind whose
        // lead the order names; FUN_0047c890 sets the check on a failure
        // within the count too, and FUN_0047b250 acts only on a running
        // mission, keeping the bits until then.
        let (mut world, _, neutral, keys) = diplomacy_world(90.0, &[90, 80]);
        let mut state = AIState::new(AiFaction::Empire);
        strategy_pass(&mut state, &world, 1);
        // The test reads the Diplomacy planner alone.
        state
            .mission_planners
            .retain(|p| p.kind == MissionKind::Diplomacy);
        let lead = state.mission_planners[0].team[0].character().unwrap();
        let other = keys.iter().copied().find(|&k| k != lead).unwrap();
        let mut heuristic = state.mission_planners[0].clone();
        heuristic.id = 77;
        heuristic.standing = false;
        state.mission_planners.insert(0, heuristic);
        state.bind_mission(MissionKind::Diplomacy, MissionMember::Character(other), 5);
        state.bind_mission(
            MissionKind::InciteUprising,
            MissionMember::Character(lead),
            6,
        );
        assert!(state.mission_planners.iter().all(|p| p.mission.is_none()));
        state.bind_mission(MissionKind::Diplomacy, MissionMember::Character(lead), 7);
        assert_eq!(
            state
                .mission_planners
                .iter()
                .map(|p| p.mission)
                .collect::<Vec<_>>(),
            vec![None, Some(7)]
        );

        let mut missions = MissionState::new();
        let id = missions.dispatch(crate::missions::MissionRequest {
            kind: MissionKind::Diplomacy,
            faction: MissionFaction::Empire,
            team: vec![MissionMember::Character(lead)],
            decoys: Vec::new(),
            target_system: neutral,
            target_character: None,
            target_object: None,
            tick: 1,
        });
        // The heuristic planner names the same mission: it is not read.
        state.mission_planners[0].mission = Some(id);
        state.mission_planners[0].report = Some(crate::missions::MissionOutcome::Failure);
        state.mission_planners[0].failures = 9;
        state.bind_mission(MissionKind::Diplomacy, MissionMember::Character(lead), id);
        world.systems[neutral].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        world.systems[neutral].popularity_empire = 0.9;
        let galaxy = galaxy_of(&state, &world);
        // A failure within the count, the target no candidate: abort.
        state.note_mission_results(&[result(id, crate::missions::MissionOutcome::Failure)]);
        state.mission_planners[0].report = Some(crate::missions::MissionOutcome::Failure);
        let mut actions = Vec::new();
        AISystem::mission_planner_reports(&mut state, &galaxy, (&missions, 1), &mut actions);
        assert_eq!(aborted(&actions), vec![id]);
        assert_eq!(state.mission_planners[0].failures, 9);

        // A report for a mission that has ended waits for a running one.
        state.note_mission_results(&[result(id, crate::missions::MissionOutcome::Success)]);
        let mut ended = missions.clone();
        assert!(ended.abort(id));
        let mut actions = Vec::new();
        AISystem::mission_planner_reports(&mut state, &galaxy, (&ended, 1), &mut actions);
        assert!(actions.is_empty());
        assert_eq!(state.mission_planners[1].pending, 0x4000_0000);
        let mut actions = Vec::new();
        AISystem::mission_planner_reports(&mut state, &galaxy, (&missions, 1), &mut actions);
        assert_eq!(aborted(&actions), vec![id]);
        assert_eq!(state.mission_planners[1].pending, 0);

        // An abort waiting with no running mission, then a check: both
        // bits wait, and the running mission is aborted once.
        state.mission_planners[1].failures = 5;
        state.note_mission_results(&[result(id, crate::missions::MissionOutcome::Failure)]);
        AISystem::mission_planner_reports(&mut state, &galaxy, (&ended, 1), &mut Vec::new());
        state.note_mission_results(&[result(id, crate::missions::MissionOutcome::Success)]);
        AISystem::mission_planner_reports(&mut state, &galaxy, (&ended, 1), &mut Vec::new());
        assert_eq!(state.mission_planners[1].pending, 0x6000_0000);
        let mut actions = Vec::new();
        AISystem::mission_planner_reports(&mut state, &galaxy, (&missions, 1), &mut actions);
        assert_eq!(aborted(&actions), vec![id]);
    }

    #[test]
    fn recon_espionage_and_recruitment_read_their_own_reports() {
        // FUN_0047f720 / FUN_00480eb0: a success marks the target seen
        // today (FUN_00433dc0, record +0x44); either result clears +0x34,
        // and Espionage +0x38 too. FUN_0047dad0: a success makes an own
        // calm target the gathering place +0x38.
        use crate::missions::MissionOutcome::{Failure, Success};
        let (world, home, rebels, _) = recon_world(0, true);
        let spy = add_commando(&mut GameWorld::default(), false, 60, 0);
        let mut state = AIState::new(AiFaction::Empire);
        let planner = |id, kind, lead| MissionPlanner {
            id,
            kind,
            team: vec![lead],
            decoys: Vec::new(),
            target_system: Some(rebels[0]),
            target_character: None,
            target_object: None,
            created: 0,
            last_cycle: 0,
            staging: Some(home),
            standing: true,
            claimed: None,
            sector: None,
            mission: None,
            report: None,
            failures: 3,
            pending: 0,
        };
        let scout =
            MissionMember::Character(add_character(&mut GameWorld::default(), false, true, 0));
        state.mission_planners = vec![
            planner(1, MissionKind::Reconnaissance, scout),
            planner(2, MissionKind::Espionage, spy),
            MissionPlanner {
                target_system: Some(home),
                staging: None,
                ..planner(3, MissionKind::Recruitment, scout)
            },
        ];
        state.bind_mission(MissionKind::Reconnaissance, scout, 10);
        state.bind_mission(MissionKind::Espionage, spy, 11);
        state.bind_mission(MissionKind::Recruitment, scout, 12);
        assert_eq!(
            state
                .mission_planners
                .iter()
                .map(|p| p.mission)
                .collect::<Vec<_>>(),
            vec![Some(10), Some(11), Some(12)]
        );
        let galaxy = galaxy_of(&state, &world);
        state.note_mission_results(&[
            result(10, Success),
            result(11, Failure),
            result(12, Success),
        ]);
        AISystem::mission_planner_reports(
            &mut state,
            &galaxy,
            (&MissionState::new(), 40),
            &mut Vec::new(),
        );
        let [recon, espionage, recruit] = [0, 1, 2].map(|i| state.mission_planners[i].clone());
        assert_eq!(
            (recon.target_system, recon.staging, recon.failures),
            (None, Some(home), 0)
        );
        assert_eq!(state.seen.get(&rebels[0]), Some(&40));
        assert_eq!(
            (
                espionage.target_system,
                espionage.staging,
                espionage.failures
            ),
            (None, None, 3)
        );
        assert_eq!((recruit.staging, recruit.failures), (Some(home), 0));
        assert!(state.mission_planners.iter().all(|p| p.report.is_none()));

        // A Recruitment target not the side's does not become the
        // gathering place; failures count against 1000 (FUN_0047ae20 with
        // FUN_00480df0) and wrap past it.
        let read = |state: &mut AIState, outcome| {
            state.note_mission_results(&[result(12, outcome)]);
            AISystem::mission_planner_reports(
                state,
                &galaxy,
                (&MissionState::new(), 41),
                &mut Vec::new(),
            );
            (
                state.mission_planners[2].staging,
                state.mission_planners[2].failures,
            )
        };
        state.mission_planners[2].staging = None;
        state.mission_planners[2].target_system = Some(rebels[1]);
        assert_eq!(read(&mut state, Success), (None, 0));
        assert_eq!(read(&mut state, Failure), (None, 1));
        state.mission_planners[2].failures = 999;
        assert_eq!(read(&mut state, Failure), (None, 1000));
        assert_eq!(read(&mut state, Failure), (None, 0));
    }

    #[test]
    fn incite_bears_ten_failures_and_subdue_checks_its_revolt() {
        // Slot +0x24: 10 for Incite (FUN_004820c0), 5 for Subdue
        // (FUN_00404c20); slot +0x34: +0x2c 0x20 for Subdue (FUN_0047d320).
        let run = |state: &mut AIState,
                   world: &GameWorld,
                   uprisings: &crate::uprising::UprisingState,
                   kind: MissionKind,
                   outcomes: &[crate::missions::MissionOutcome]| {
            let p = state
                .mission_planners
                .iter()
                .find(|p| p.kind == kind)
                .unwrap();
            let (lead, target) = (p.team[0], p.target_system.unwrap());
            let mut missions = MissionState::new();
            let id = missions.dispatch(crate::missions::MissionRequest {
                kind,
                faction: MissionFaction::Empire,
                team: vec![lead],
                decoys: Vec::new(),
                target_system: target,
                target_character: None,
                target_object: None,
                tick: 1,
            });
            state.bind_mission(kind, lead, id);
            let missions_ref = &missions;
            outcomes
                .iter()
                .map(|&o| {
                    state.note_mission_results(&[result(id, o)]);
                    let missions_state = MissionState::new();
                    let economy = crate::economy::EconomyState::default();
                    let manufacturing = ManufacturingState::new();
                    let research = ResearchState::new();
                    let movement = crate::movement::MovementState::new();
                    let transport = crate::troop_transport::TroopTransportState::new();
                    let busy = HashSet::new();
                    let planner = Planner::new(world, &missions_state, &busy, false, 7);
                    let inputs = RecordInputs {
                        world,
                        economy: &economy,
                        uprisings,
                        manufacturing: &manufacturing,
                        research: &research,
                        movement: &movement,
                        transport: &transport,
                    };
                    let galaxy = AISystem::records(state, &planner, &inputs);
                    let mut actions = Vec::new();
                    AISystem::mission_planner_reports(
                        state,
                        &galaxy,
                        (missions_ref, 1),
                        &mut actions,
                    );
                    !aborted(&actions).is_empty()
                })
                .collect::<Vec<bool>>()
        };
        use crate::missions::MissionOutcome::{Failure, Success};
        let (world, _, _, _, _) = incite_world(&[60], &[60.0], &[]);
        let mut state = AIState::new(AiFaction::Empire);
        strategy_pass(&mut state, &world, 1);
        let calm = crate::uprising::UprisingState::default();
        let mut expected = vec![false; 10];
        expected.push(true);
        assert_eq!(
            run(
                &mut state,
                &world,
                &calm,
                MissionKind::InciteUprising,
                &[Failure; 11]
            ),
            expected
        );

        let (world, uprisings, _, _) =
            subdue_world(&[(90.0, false, false), (30.0, true, false)], &[90]);
        let mut state = AIState::new(AiFaction::Empire);
        strategy_pass_in(&mut state, &world, 1, &uprisings);
        assert_eq!(
            run(
                &mut state,
                &world,
                &uprisings,
                MissionKind::SubdueUprising,
                &[Success]
            ),
            vec![false]
        );
        assert_eq!(
            run(
                &mut state,
                &world,
                &calm,
                MissionKind::SubdueUprising,
                &[Success]
            ),
            vec![true]
        );
    }

    #[test]
    fn the_strategy_adds_and_cancels_only_standing_diplomacy_planners() {
        // FUN_004e5710 picks among the planners of the cancelled kind;
        // the heuristic planner holds a minor of diplomacy 50 and the
        // standing one takes the diplomat of 90, so the heuristic one is the
        // weaker;
        // FUN_004e8290 row 5 counts only +0x34c.
        let heuristic = |id, member| MissionPlanner {
            id,
            kind: MissionKind::Diplomacy,
            team: vec![member],
            decoys: Vec::new(),
            target_system: None,
            target_character: None,
            target_object: None,
            created: 1,
            last_cycle: 0,
            staging: None,
            standing: false,
            claimed: None,
            sector: None,
            mission: None,
            report: None,
            failures: 0,
            pending: 0,
        };
        let (world, _, _, keys) = diplomacy_world(90.0, &[50, 90]);
        let mut state = AIState::new(AiFaction::Empire);
        state
            .mission_planners
            .push(heuristic(50, MissionMember::Character(keys[0])));
        strategy_pass(&mut state, &world, 1);
        assert_eq!(
            only(&state, MissionKind::Diplomacy)
                .iter()
                .map(|p| p.standing)
                .collect::<Vec<_>>(),
            vec![false, true]
        );

        // No candidate: the standing one goes, the other stays.
        let mut world = world;
        for sys in world.systems.values_mut() {
            sys.control = ControlKind::Controlled(crate::dat::Faction::Empire);
            sys.popularity_empire = 0.9;
        }
        strategy_pass(&mut state, &world, 2);
        assert_eq!(
            only(&state, MissionKind::Diplomacy)
                .iter()
                .map(|p| p.id)
                .collect::<Vec<_>>(),
            vec![50]
        );
    }

    #[test]
    fn only_standing_diplomacy_planners_count_against_the_want_and_each_has_its_own_id() {
        // FUN_004e8290 row 5 counts +0x34c, the live Diplomacy planners
        // (slot +0x40 = 0xb, FUN_004814f0).
        let (world, home, neutral, _) = diplomacy_world(90.0, &[]);
        let mut state = AIState::new(AiFaction::Empire);
        let other = |kind, standing| MissionPlanner {
            id: 50,
            kind,
            team: Vec::new(),
            decoys: Vec::new(),
            target_system: Some(home),
            target_character: None,
            target_object: None,
            created: 1,
            last_cycle: 0,
            staging: None,
            standing,
            claimed: None,
            sector: None,
            mission: None,
            report: None,
            failures: 0,
            pending: 0,
        };
        state
            .mission_planners
            .push(other(MissionKind::Diplomacy, false));
        // Row 8 (Death Star Sabotage) is not ported, so its planner stays.
        state
            .mission_planners
            .push(other(MissionKind::DeathStarSabotage, true));
        state.next_planner_id = 3;
        let mut galaxy = crate::ai_records::Galaxy::default();
        AISystem::mission_strategy(&mut state, &world, &mut galaxy);
        // Only row 9's Abduction planner joins (FUN_004e8290 asks for one
        // always).
        assert_eq!(state.mission_planners.len(), 3);
        assert_eq!(only(&state, MissionKind::Abduction).len(), 1);
        let _ = neutral;

        // Five candidates want two.
        let (mut world, home, _, _) = diplomacy_world(90.0, &[]);
        let sector = world.systems[home].sector;
        for _ in 0..4 {
            add_system(&mut world, sector, 0.4, 0.4);
        }
        let mut state = AIState::new(AiFaction::Empire);
        state.next_planner_id = 3;
        strategy_pass(&mut state, &world, 1);
        let ids: Vec<u64> = only(&state, MissionKind::Diplomacy)
            .iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(ids, vec![3, 4]);
        // The Abduction planner takes the next id.
        assert_eq!(state.next_planner_id, 6);
    }

    #[test]
    fn low_diplomacy_minor_not_dispatched() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let _ = add_system(&mut world, sector, 0.1, 0.2);

        // Minor with low diplomacy — below threshold
        let _ = add_character(&mut world, false, false, 30);

        let mut state = AIState::new(AiFaction::Empire);
        let mfg = ManufacturingState::new();
        let missions = MissionState::new();

        let actions = evaluate(
            &mut state,
            &world,
            &mfg,
            &missions,
            &ticks(7),
            &GameConfig::default(),
            &crate::research::ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        );
        let mission_count = actions
            .iter()
            .filter(|a| matches!(a, AIAction::DispatchMission { .. }))
            .count();
        assert_eq!(mission_count, 0);
    }

    #[test]
    fn busy_character_not_redispatched() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let _ = add_system(&mut world, sector, 0.1, 0.2);

        let diplomat = add_character(&mut world, false, false, 80);

        let mut state = AIState::new(AiFaction::Empire);
        state.mark_busy(diplomat);

        let mfg = ManufacturingState::new();
        let missions = MissionState::new();

        let actions = evaluate(
            &mut state,
            &world,
            &mfg,
            &missions,
            &ticks(7),
            &GameConfig::default(),
            &crate::research::ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        );
        let mission_count = actions
            .iter()
            .filter(|a| matches!(a, AIAction::DispatchMission { .. }))
            .count();
        assert_eq!(mission_count, 0);
    }

    // -----------------------------------------------------------------------
    // Production heuristics
    // -----------------------------------------------------------------------

    /// An Empire system with a shipyard and a TIE Fighter class.
    fn fighter_world() -> (GameWorld, SystemKey) {
        let mut world = empty_world();
        let sector = add_sector(&mut world);

        // Add an empire manufacturing facility
        let mfg_key =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    // A shipyard: fighters are a shipyard's product, and
                    // FUN_00527d00 refuses a Build with no yard of its area.
                    class_dat_id: DatId(0x2800_0001),
                    side: crate::dat::Faction::Empire, // empire
                    is_shipyard: true,
                });

        let sys_key = world.systems.insert(System {
            dat_id: DatId(0),
            name: "Coruscant".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.1,
            popularity_empire: 0.9,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![mfg_key],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Empire),
        });

        // Add a TIE fighter class
        let _ = world.fighter_classes.insert(FighterClass {
            dat_id: DatId(10),
            name: "TIE Fighter".into(),
            is_alliance: false,
            is_empire: true,
            refined_material_cost: 20,
            maintenance_cost: 2,
            squadron_size: 6,
            torpedoes: 0,
            overall_attack_strength: 10,
            bombardment_defense: 0,
            ..FighterClass::default()
        });

        (world, sys_key)
    }

    #[test]
    fn the_production_heuristic_counts_the_squadrons_on_its_planets() {
        // Our own heuristic (module 0xb's stand-in): fighters until the side
        // holds fighter_ratio per capital ship, squadrons on planets
        // included now that a built squadron stays there.
        let (mut world, sys_key) = fighter_world();
        let destroyer = world.capital_ship_classes.insert(CapitalShipClass {
            name: "Star Destroyer".into(),
            is_empire: true,
            hull: 100,
            refined_material_cost: 100,
            ..CapitalShipClass::default()
        });
        let config = GameConfig::default();
        let ships = config.production.capship_threshold;
        let fleet = world.fleets.insert(Fleet {
            location: sys_key,
            capital_ships: ShipInstance::make(destroyer, 100, false, u32::try_from(ships).unwrap()),
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[sys_key].fleets.push(fleet);
        let tie = world.fighter_classes.keys().next().unwrap();
        let wanted = u32::try_from(ships * config.production.fighter_ratio).unwrap();
        let fighters = |world: &GameWorld| {
            let mut actions = Vec::new();
            AISystem::evaluate_production(
                world,
                &ManufacturingState::new(),
                &crate::research::ResearchState::new(),
                AiFaction::Empire,
                &config,
                &mut actions,
            );
            actions.iter().any(|a| {
                matches!(
                    a,
                    AIAction::EnqueueProduction {
                        kind: BuildableKind::Fighter(_),
                        ..
                    }
                )
            })
        };
        crate::squadrons::add(&mut world.systems[sys_key].fighters, tie, wanted - 1);
        assert!(fighters(&world));
        crate::squadrons::add(&mut world.systems[sys_key].fighters, tie, 1);
        assert!(!fighters(&world));
    }

    #[test]
    fn idle_system_with_mfg_gets_fighter_production() {
        let (world, sys_key) = fighter_world();
        let mut state = AIState::new(AiFaction::Empire);
        let mfg = ManufacturingState::new(); // empty queue
        let missions = MissionState::new();

        let actions = evaluate(
            &mut state,
            &world,
            &mfg,
            &missions,
            &ticks(7),
            &GameConfig::default(),
            &crate::research::ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        );

        let has_fighter_enqueue = actions.iter().any(|a| {
            matches!(
                a,
                AIAction::EnqueueProduction { system, kind: BuildableKind::Fighter(_), .. }
                if *system == sys_key
            )
        });
        assert!(
            has_fighter_enqueue,
            "expected fighter production at empire system"
        );
    }

    #[test]
    fn a_system_without_a_shipyard_is_given_no_ship_or_fighter_order() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);

        // Add an empire manufacturing facility
        let mfg_key =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    // A construction yard only: FUN_00527d00 refuses a Build
                    // whose manager has no yard of its area.
                    class_dat_id: DatId(0x2a00_0003),
                    side: crate::dat::Faction::Empire, // empire
                    is_shipyard: false,
                });

        let sys_key = world.systems.insert(System {
            dat_id: DatId(0),
            name: "Coruscant".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.1,
            popularity_empire: 0.9,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![mfg_key],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Empire),
        });

        // Add a TIE fighter class
        let _ = world.fighter_classes.insert(FighterClass {
            dat_id: DatId(10),
            name: "TIE Fighter".into(),
            is_alliance: false,
            is_empire: true,
            refined_material_cost: 20,
            maintenance_cost: 2,
            squadron_size: 6,
            torpedoes: 0,
            overall_attack_strength: 10,
            bombardment_defense: 0,
            ..FighterClass::default()
        });

        let mut state = AIState::new(AiFaction::Empire);
        let mfg = ManufacturingState::new(); // empty queue
        let missions = MissionState::new();

        let actions = evaluate(
            &mut state,
            &world,
            &mfg,
            &missions,
            &ticks(7),
            &GameConfig::default(),
            &crate::research::ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        );

        let ship_orders = actions
            .iter()
            .filter(|a| {
                matches!(
                    a,
                    AIAction::EnqueueProduction {
                        system,
                        kind: BuildableKind::Fighter(_) | BuildableKind::CapitalShip(_),
                        ..
                    } if *system == sys_key
                )
            })
            .count();
        assert_eq!(ship_orders, 0);
    }

    /// Fighter the Empire AI enqueues at an idle shipyard system holding a
    /// researched TIE Fighter (order 0) and a TIE Defender (FIGHTSD.DAT
    /// research_order 8) at the given Ship research level.
    fn empire_fighter_choice_at_ship_level(ship_level: u32) -> DatId {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let mfg_key =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId(0x2800_0001),
                    side: crate::dat::Faction::Empire,
                    is_shipyard: true,
                });
        world.systems.insert(System {
            dat_id: DatId(0),
            name: "Coruscant".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.1,
            popularity_empire: 0.9,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![mfg_key],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Empire),
        });
        for (id, name, cost, order) in [(5, "TIE Fighter", 20, 0), (8, "TIE Defender", 60, 8)] {
            world.fighter_classes.insert(FighterClass {
                dat_id: DatId(id),
                name: name.into(),
                is_empire: true,
                refined_material_cost: cost,
                squadron_size: 6,
                research_order: order,
                ..FighterClass::default()
            });
        }
        let mut research = crate::research::ResearchState::new();
        research.empire.ship = ship_level;

        let actions = evaluate(
            &mut AIState::new(AiFaction::Empire),
            &world,
            &ManufacturingState::new(),
            &MissionState::new(),
            &ticks(7),
            &GameConfig::default(),
            &research,
            &crate::uprising::UprisingState::default(),
        );
        actions
            .iter()
            .find_map(|a| match a {
                AIAction::EnqueueProduction {
                    kind: BuildableKind::Fighter(key),
                    ..
                } => Some(world.fighter_classes[*key].dat_id),
                _ => None,
            })
            .expect("fighter production")
    }

    #[test]
    fn ai_picks_its_strongest_researched_capital_ship() {
        let mut world = empty_world();
        // CAPSHPSD.DAT: order-0 classes start buildable; 9 is late-game.
        for (id, hull, order) in [(131, 100, 0), (128, 900, 9)] {
            world
                .capital_ship_classes
                .insert(crate::world::CapitalShipClass {
                    dat_id: DatId(id),
                    is_empire: true,
                    hull,
                    research_order: order,
                    ..Default::default()
                });
        }
        let mut research = crate::research::ResearchState::new();
        let pick = |research: &crate::research::ResearchState| {
            AISystem::best_capital_ship_class(&world, research, AiFaction::Empire)
                .map(|(_, c)| c.dat_id)
        };

        assert_eq!(pick(&research), Some(DatId(131)));
        research.empire.ship = 9;
        assert_eq!(pick(&research), Some(DatId(128)));
    }

    #[test]
    fn ai_builds_only_fighters_its_ship_research_has_reached() {
        assert_eq!(empire_fighter_choice_at_ship_level(7), DatId(5));
        assert_eq!(empire_fighter_choice_at_ship_level(8), DatId(8));
    }

    #[test]
    fn uncontrolled_system_facility_cannot_produce_for_ai() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let mfg_key =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId(1),
                    side: crate::dat::Faction::Empire,
                    is_shipyard: false,
                });
        let sys_key = add_system(&mut world, sector, 0.5, 0.5);
        world.systems[sys_key]
            .manufacturing_facilities
            .push(mfg_key);
        world.systems[sys_key].control = ControlKind::Uncontrolled;
        world.fighter_classes.insert(FighterClass {
            dat_id: DatId(10),
            name: "TIE Fighter".into(),
            is_empire: true,
            refined_material_cost: 20,
            overall_attack_strength: 10,
            ..FighterClass::default()
        });

        let mut state = AIState::new(AiFaction::Empire);
        let actions = evaluate(
            &mut state,
            &world,
            &ManufacturingState::new(),
            &MissionState::new(),
            &ticks(7),
            &GameConfig::default(),
            &crate::research::ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        );

        assert!(!actions.iter().any(|action| matches!(
            action,
            AIAction::EnqueueProduction { system, .. } if *system == sys_key
        )));
    }

    // -----------------------------------------------------------------------
    // Fleet deployment heuristics
    // -----------------------------------------------------------------------

    #[test]
    fn blockading_fleet_holds_enemy_system() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let hq_sys = add_system(&mut world, sector, 0.1, 0.9);
        world.systems[hq_sys].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        world.systems[hq_sys].is_headquarters = true;
        let blockaded_sys = add_system(&mut world, sector, 0.8, 0.2);
        world.systems[blockaded_sys].control =
            ControlKind::Controlled(crate::dat::Faction::Alliance);
        let other_target = add_system(&mut world, sector, 0.7, 0.3);
        world.systems[other_target].control =
            ControlKind::Controlled(crate::dat::Faction::Alliance);

        let class_key = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        let blockader = world.fleets.insert(Fleet {
            location: blockaded_sys,
            capital_ships: ShipInstance::make(class_key, 100, false, 1),
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[blockaded_sys].fleets.push(blockader);

        let mut state = AIState::new(AiFaction::Empire);
        let actions = evaluate(
            &mut state,
            &world,
            &ManufacturingState::new(),
            &MissionState::new(),
            &ticks(7),
            &GameConfig::default(),
            &ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        );

        assert!(!actions.iter().any(
            |action| matches!(action, AIAction::MoveFleet { fleet, .. } if *fleet == blockader)
        ));
    }

    #[test]
    fn fleet_in_transit_is_not_redispatched() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let home_sys = add_system(&mut world, sector, 0.1, 0.9);
        world.systems[home_sys].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let target_sys = add_system(&mut world, sector, 0.8, 0.1);
        world.systems[target_sys].control = ControlKind::Controlled(crate::dat::Faction::Alliance);

        let class_key = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        let fleet_key = world.fleets.insert(Fleet {
            location: home_sys,
            capital_ships: ShipInstance::make(class_key, 100, false, 1),
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[home_sys].fleets.push(fleet_key);

        let mut movement = crate::movement::MovementState::new();
        assert!(movement.order(fleet_key, home_sys, target_sys, 10));
        crate::movement::MovementSystem::advance(&mut movement, &ticks(3));
        let before = movement.get(fleet_key).unwrap().clone();

        let mut state = AIState::new(AiFaction::Empire);
        let actions = evaluate(
            &mut state,
            &world,
            &ManufacturingState::new(),
            &MissionState::new(),
            &ticks(7),
            &GameConfig::default(),
            &ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        );

        assert!(!actions.iter().any(
            |action| matches!(action, AIAction::MoveFleet { fleet, .. } if *fleet == fleet_key)
        ));
        assert_eq!(movement.get(fleet_key), Some(&before));
    }

    #[test]
    fn fleet_receives_at_most_one_move_action_per_evaluation() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let home_sys = add_system(&mut world, sector, 0.1, 0.9);
        world.systems[home_sys].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let staging_sys = add_system(&mut world, sector, 0.1, 0.9);
        world.systems[staging_sys].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        world.systems[staging_sys].x = 25;
        let target_sys = add_system(&mut world, sector, 0.8, 0.1);
        world.systems[target_sys].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        world.systems[target_sys].x = 100;

        let class_key = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        let death_star = world.fleets.insert(Fleet {
            location: home_sys,
            capital_ships: ShipInstance::make(class_key, 100, false, 1),
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: true,
        });
        world.systems[home_sys].fleets.push(death_star);
        let escort = world.fleets.insert(Fleet {
            location: staging_sys,
            capital_ships: ShipInstance::make(class_key, 100, false, 1),
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[staging_sys].fleets.push(escort);

        let mut state = AIState::new(AiFaction::Empire);
        let actions = evaluate(
            &mut state,
            &world,
            &ManufacturingState::new(),
            &MissionState::new(),
            &ticks(7),
            &GameConfig::default(),
            &ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        );

        for fleet in [death_star, escort] {
            let move_count = actions
                .iter()
                .filter(|action| {
                    matches!(action, AIAction::MoveFleet { fleet: moved, .. } if *moved == fleet)
                })
                .count();
            assert!(move_count <= 1, "fleet received {move_count} move actions");
        }
    }

    // -----------------------------------------------------------------------
    // Espionage heuristics
    // -----------------------------------------------------------------------

    /// Helper: insert a character with custom espionage + combat scores.
    /// A covert operative. The covert tables read their input as the chance
    /// (row `t` holds `t`), so the chance follows the operative's skills
    /// through `member_chance`.
    fn add_spy(
        world: &mut GameWorld,
        is_alliance: bool,
        is_major: bool,
        espionage_base: u32,
        combat_base: u32,
    ) -> CharacterKey {
        for key in ["SBTGMSTB", "ESPIMSTB", "ASSNMSTB", "ABDCMSTB"] {
            let rows = (0..=100)
                .map(|threshold| crate::world::MstbEntry {
                    threshold,
                    value: threshold.unsigned_abs(),
                })
                .collect();
            world
                .mission_tables
                .insert(key.to_string(), crate::world::MstbTable::new(rows));
        }
        world.characters.insert(Character {
            name: "TestSpy".into(),
            is_alliance,
            is_empire: !is_alliance,
            is_major,
            espionage: SkillPair {
                base: espionage_base,
                variance: 0,
            },
            combat: SkillPair {
                base: combat_base,
                variance: 0,
            },
            leadership: SkillPair {
                base: 50,
                variance: 0,
            },
            can_be_commander: true,
            recruited: true,
            ..Default::default()
        })
    }

    /// A special force of `espionage` and `combat`, standing nowhere.
    fn add_commando(
        world: &mut GameWorld,
        is_alliance: bool,
        espionage: u32,
        combat: u32,
    ) -> crate::missions::MissionMember {
        let mut skills = [0; 8];
        skills[crate::world::Skill::Espionage as usize] = espionage;
        skills[crate::world::Skill::Combat as usize] = combat;
        crate::missions::MissionMember::SpecialForce(world.special_forces.insert(
            crate::world::SpecialForceUnit {
                class_dat_id: DatId(0x3c00_0002),
                is_alliance,
                skills,
                on_mission: false,
            },
        ))
    }

    /// Whether the Empire sends a special force of `espionage` and `combat`
    /// to sabotage an Alliance manufacturing system.
    fn sabotage_dispatched(espionage: u32, combat: u32) -> bool {
        sabotage_dispatched_by(AiFaction::Empire, |world, alliance| {
            add_commando(world, alliance, espionage, combat)
        })
    }

    /// Whether `side`'s AI sends the agent `add` makes to sabotage the enemy
    /// shipyard, naming the shipyard as the target object.
    fn sabotage_dispatched_by(
        side: AiFaction,
        add: impl FnOnce(&mut GameWorld, bool) -> crate::missions::MissionMember,
    ) -> bool {
        let alliance_ai = side == AiFaction::Alliance;
        let mut world = empty_world();
        let sector = add_sector(&mut world);

        // Enemy (alliance) system with a manufacturing facility — a sabotage target.
        let mfg_key =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId(1),
                    side: crate::dat::Faction::of_alliance(!alliance_ai), // owned by the AI's enemy
                    is_shipyard: false,
                });
        let enemy_sys = world.systems.insert(System {
            dat_id: DatId(0),
            name: "Enemy Shipyard".into(),
            sector,
            x: 10,
            y: 10,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.8,
            popularity_empire: 0.1,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![mfg_key],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Uncontrolled,
        });

        let spy = add(&mut world, alliance_ai);

        let mut state = AIState::new(side);
        let mfg = ManufacturingState::new();
        let missions = MissionState::new();

        let actions = evaluate(
            &mut state,
            &world,
            &mfg,
            &missions,
            &ticks(7),
            &GameConfig::default(),
            &crate::research::ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        );

        let sabotage = actions.iter().find(|a| {
            matches!(
                a,
                AIAction::DispatchMission {
                    kind: MissionKind::Sabotage,
                    team,
                    target_system,
                    target_object: Some(crate::missions::MissionTarget::ManufacturingFacility(yard)),
                    ..
                }
                if *team == [spy] && *target_system == enemy_sys && *yard == mfg_key
            )
        });
        sabotage.is_some()
    }

    #[test]
    fn a_sabotage_team_needs_combat_and_espionage_of_51() {
        // FUN_0047d520 queries combat 51..=10000 and espionage 51..=10000
        // and takes only hits of both (FUN_004357b0).
        assert!(sabotage_dispatched(51, 51));
        assert!(!sabotage_dispatched(50, 80));
        assert!(!sabotage_dispatched(80, 50));
    }

    #[test]
    fn an_alliance_commando_sabotages_an_empire_shipyard() {
        assert!(sabotage_dispatched_by(
            AiFaction::Alliance,
            |world, alliance| { add_commando(world, alliance, 80, 60) }
        ));
    }

    #[test]
    fn low_espionage_character_not_dispatched_on_covert_ops() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);

        let mfg_key =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId(1),
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: false,
                });
        let _ = world.systems.insert(System {
            dat_id: DatId(0),
            name: "Enemy Shipyard".into(),
            sector,
            x: 10,
            y: 10,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.8,
            popularity_empire: 0.1,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![mfg_key],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Uncontrolled,
        });

        // Empire spy with low espionage — below threshold.
        let _ = add_spy(&mut world, false, false, 20, 20);

        let mut state = AIState::new(AiFaction::Empire);
        let mfg = ManufacturingState::new();
        let missions = MissionState::new();

        let actions = evaluate(
            &mut state,
            &world,
            &mfg,
            &missions,
            &ticks(7),
            &GameConfig::default(),
            &crate::research::ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        );

        let covert = actions
            .iter()
            .filter(|a| {
                matches!(
                    a,
                    AIAction::DispatchMission {
                        kind: MissionKind::Sabotage
                            | MissionKind::Assassination
                            | MissionKind::Espionage,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(
            covert, 0,
            "low-espionage character should not be dispatched on covert ops"
        );
    }

    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    fn covert_ops_capped_at_max_per_eval() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);

        // Create 5 enemy manufacturing systems — more than MAX_COVERT_OPS_PER_EVAL.
        for i in 0..5u32 {
            let mfg_key = world.manufacturing_facilities.insert(
                crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId(i),
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: false,
                },
            );
            world.systems.insert(System {
                dat_id: DatId(i),
                name: format!("Enemy System {i}"),
                sector,
                x: (i * 10) as u16,
                y: 0,
                exploration_status: crate::dat::ExplorationStatus::Explored,
                popularity_alliance: 0.7,
                popularity_empire: 0.1,
                is_populated: true,
                total_energy: 0,
                raw_materials: 0,
                espionage_rating: 0.0,
                fleets: vec![],
                ground_units: vec![],
                special_forces: vec![],
                defense_facilities: vec![],
                manufacturing_facilities: vec![mfg_key],
                production_facilities: vec![],
                fighters: Vec::new(),
                is_headquarters: false,
                is_destroyed: false,
                control: ControlKind::Uncontrolled,
            });
        }

        // 10 Empire special forces — all highly skilled.
        for _ in 0..10 {
            add_commando(&mut world, false, 80, 60);
        }

        let mut state = AIState::new(AiFaction::Empire);
        let mfg = ManufacturingState::new();
        let missions = MissionState::new();

        let actions = evaluate(
            &mut state,
            &world,
            &mfg,
            &missions,
            &ticks(7),
            &GameConfig::default(),
            &crate::research::ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        );

        let covert_count = actions
            .iter()
            .filter(|a| {
                matches!(
                    a,
                    AIAction::DispatchMission {
                        kind: MissionKind::Sabotage
                            | MissionKind::Assassination
                            | MissionKind::Espionage,
                        ..
                    }
                )
            })
            .count();

        assert!(
            covert_count <= MAX_COVERT_OPS_PER_EVAL,
            "expected at most {MAX_COVERT_OPS_PER_EVAL} covert ops, got {covert_count}",
        );
    }

    // -----------------------------------------------------------------------
    // Knesset Shamash-Bet #R11 — killed characters are invisible to dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn a_killed_character_is_never_planned() {
        // FUN_00402230 builds records only for the side's live objects.
        let mut world = empty_world();
        let ck = add_character(&mut world, /*alliance*/ false, /*major*/ true, 90);
        assert!(planner_for(&world, AiFaction::Empire)
            .plan(MissionKind::Diplomacy)
            .is_some());

        world.characters.get_mut(ck).unwrap().mark_killed();
        assert_eq!(
            planner_for(&world, AiFaction::Empire).plan(MissionKind::Diplomacy),
            None,
            "killed character must not pass dispatch gate"
        );
    }

    // -----------------------------------------------------------------------
    // D1: Enhanced dispatch validator tests
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // D2: Troop deployment tests
    // -----------------------------------------------------------------------

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    fn troop_deployment_prioritizes_frontline_systems() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let config = GameConfig::default();

        // Donor system: 5 empire troops.
        let donor_sys = world.systems.insert(System {
            dat_id: DatId(0),
            name: "Donor".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.1,
            popularity_empire: 0.9,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Empire),
        });
        for _ in 0..5 {
            let tk = world.troops.insert(crate::world::TroopUnit {
                class_dat_id: DatId(0),
                is_alliance: false,
                regiment_strength: 100,
            });
            world.systems[donor_sys].ground_units.push(tk);
        }

        // Frontline receiver: 0 troops, near enemy territory.
        let frontline_sys = world.systems.insert(System {
            dat_id: DatId(1),
            name: "Frontline".into(),
            sector,
            x: 100,
            y: 0, // close to enemy
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.1,
            popularity_empire: 0.9,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Empire),
        });

        // Interior receiver: 0 troops, far from enemy.
        let _interior_sys = world.systems.insert(System {
            dat_id: DatId(2),
            name: "Interior".into(),
            sector,
            x: 0,
            y: 500, // far from enemy
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.1,
            popularity_empire: 0.9,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Empire),
        });

        // Enemy system at x=120 (near frontline_sys).
        let _ = world.systems.insert(System {
            dat_id: DatId(3),
            name: "Enemy".into(),
            sector,
            x: 120,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.9,
            popularity_empire: 0.1,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });

        let mut actions = Vec::new();
        AISystem::evaluate_troop_deployment(&world, AiFaction::Empire, &config, &mut actions);

        // Should dispatch at least one troop move.
        assert!(!actions.is_empty(), "expected troop deployment actions");

        // Module 0xc, the movement planners, issues the troop moves.
        let (missions, research, mfg, uprisings) = (
            MissionState::new(),
            ResearchState::new(),
            ManufacturingState::new(),
            crate::uprising::UprisingState::default(),
        );
        let ctx = ModuleInputs {
            faction: AiFaction::Empire,
            ..inputs(&world, &missions, &research, &mfg, &config, &uprisings)
        };
        let busy = HashSet::new();
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let mut by_module = Vec::new();
        AISystem::run_module(
            0xc,
            &mut AIState::new(AiFaction::Empire),
            &mut planner,
            &ctx,
            &mut crate::ai_records::Galaxy::default(),
            &mut by_module,
        );
        assert_eq!(by_module, actions);

        // The first troop move goes to the frontline system (higher
        // priority): one donor regiment's own Move (0x201). It rode a
        // transport fleet until the port's automatic landing was retired
        // (2026-10-17): carried regiments now stay aboard without an order
        // (ghidra/notes/regiment-unload.md).
        let AIAction::MoveRegiments { troops, to } = &actions[0] else {
            panic!("expected a regiment's own Move: {actions:?}");
        };
        assert_eq!(
            *to, frontline_sys,
            "first troop deployment should target frontline system"
        );
        assert_eq!(troops.len(), 1);
        assert!(world.systems[donor_sys].ground_units.contains(&troops[0]));
    }

    // -----------------------------------------------------------------------
    // D5: Reconnaissance test
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Planned missions (F-019 phase 5a)
    // -----------------------------------------------------------------------

    use crate::missions::MissionMember;

    /// An Empire world with two characters at `here` and `there`, and a
    /// target system.
    fn gathering_world() -> (GameWorld, [CharacterKey; 2], [SystemKey; 3]) {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let places = [0, 1, 2].map(|_| add_system(&mut world, sector, 0.2, 0.8));
        // The target is the Empire's, so the team gathers there (state 6).
        world.systems[places[2]].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let chars = [places[0], places[1]].map(|at| {
            let c = add_character(&mut world, false, false, 60);
            world.characters[c].current_system = Some(at);
            c
        });
        (world, chars, places)
    }

    fn rescue_order(team: Vec<MissionMember>, target: SystemKey) -> AIAction {
        AIAction::DispatchMission {
            kind: MissionKind::Rescue,
            team,
            decoys: Vec::new(),
            target_system: target,
            target_character: None,
            target_object: None,
        }
    }

    #[test]
    fn a_mission_planner_gathers_its_team_before_the_order() {
        // FUN_004bced0: state 7 (FUN_004bd570) moves each member away from
        // the target there (Move 0x201); state 8 (FUN_0047b360) orders the
        // mission once all stand there.
        let (mut world, [a, b], [_, _, target]) = gathering_world();
        let team = vec![MissionMember::Character(a), MissionMember::Character(b)];
        let mut state = AIState::new(AiFaction::Empire);
        let mut actions = vec![rescue_order(team.clone(), target)];
        AISystem::adopt_mission_orders(&mut state, 1, 0, &mut actions);
        assert!(actions.is_empty());
        assert_eq!(state.mission_planners.len(), 1);
        // The same order again is the running planner's.
        let mut again = vec![rescue_order(team.clone(), target)];
        AISystem::adopt_mission_orders(&mut state, 1, 0, &mut again);
        assert!(again.is_empty());
        assert_eq!(state.mission_planners.len(), 1);

        let mut actions = Vec::new();
        AISystem::mission_planner_cycle(
            &mut state,
            &mut planner_for(&world, AiFaction::Empire),
            &world,
            &mut crate::ai_records::Galaxy::default(),
            2,
            &mut actions,
        );
        let moves: Vec<_> = orders(&actions, MissionKind::Move)
            .into_iter()
            .map(|(team, to, _)| (team, to))
            .collect();
        assert_eq!(moves, [(vec![team[0]], target), (vec![team[1]], target)]);
        assert_eq!(state.mission_planners.len(), 1);
        // A second AI tick the same day leaves the planner as it is.
        let mut again = Vec::new();
        AISystem::mission_planner_cycle(
            &mut state,
            &mut planner_for(&world, AiFaction::Empire),
            &world,
            &mut crate::ai_records::Galaxy::default(),
            2,
            &mut again,
        );
        assert!(again.is_empty());
        assert_eq!(state.mission_planners.len(), 1);

        // One there, one still away: no order yet.
        world.characters[a].current_system = Some(target);
        let mut actions = Vec::new();
        AISystem::mission_planner_cycle(
            &mut state,
            &mut planner_for(&world, AiFaction::Empire),
            &world,
            &mut crate::ai_records::Galaxy::default(),
            3,
            &mut actions,
        );
        assert!(orders(&actions, MissionKind::Rescue).is_empty());
        assert_eq!(orders(&actions, MissionKind::Move).len(), 1);

        world.characters[b].current_system = Some(target);
        let mut actions = Vec::new();
        AISystem::mission_planner_cycle(
            &mut state,
            &mut planner_for(&world, AiFaction::Empire),
            &world,
            &mut crate::ai_records::Galaxy::default(),
            4,
            &mut actions,
        );
        assert_eq!(
            orders(&actions, MissionKind::Rescue),
            [(team, target, None)]
        );
        assert!(state.mission_planners.is_empty());
    }

    #[test]
    fn a_mission_planner_refills_its_team_to_the_class_s_size() {
        // FUN_004bced0 state 3 (slot +0x48): the Rescue selector
        // (FUN_0047e270, size 2) runs each cycle and replaces a member
        // state 2 dropped; a member the planner holds is not picked twice.
        let (mut world, [a, b], [_, _, target]) = gathering_world();
        // FUN_0047e270 forbids the officer capabilities (0x7000000).
        let fighter = |world: &mut GameWorld, key: CharacterKey| {
            world.characters[key].combat = SkillPair {
                base: 90,
                variance: 0,
            };
            world.characters[key].can_be_commander = false;
        };
        fighter(&mut world, a);
        fighter(&mut world, b);
        let c = add_character(&mut world, false, false, 60);
        world.characters[c].current_system = Some(target);
        fighter(&mut world, c);
        let team = vec![MissionMember::Character(a), MissionMember::Character(b)];
        let mut state = AIState::new(AiFaction::Empire);
        AISystem::adopt_mission_orders(&mut state, 1, 0, &mut vec![rescue_order(team, target)]);
        let mut actions = Vec::new();
        AISystem::mission_planner_cycle(
            &mut state,
            &mut planner_for(&world, AiFaction::Empire),
            &world,
            &mut crate::ai_records::Galaxy::default(),
            2,
            &mut actions,
        );
        assert_eq!(
            state.mission_planners[0].team,
            [MissionMember::Character(a), MissionMember::Character(b)],
            "a full team takes no one"
        );
        world.characters[b].mark_killed();
        AISystem::mission_planner_cycle(
            &mut state,
            &mut planner_for(&world, AiFaction::Empire),
            &world,
            &mut crate::ai_records::Galaxy::default(),
            3,
            &mut actions,
        );
        assert_eq!(
            state.mission_planners[0].team,
            [MissionMember::Character(a), MissionMember::Character(c)]
        );
    }

    #[test]
    fn a_mission_planner_gathers_at_the_side_s_system_nearest_an_enemy_target() {
        // FUN_004bd0a0: the previous place while it is the side's; else the
        // target when it is; else the side's system nearest the target
        // (FUN_0041a460); else the first member's system.
        let (mut world, [a, _], [here, there, target]) = gathering_world();
        let team = [MissionMember::Character(a)];
        world.systems[target].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        world.systems[target].x = 100;
        world.systems[here].x = 10;
        world.systems[there].x = 90;
        assert_eq!(
            staging_point(&world, false, None, target, &team),
            Some(here),
            "none own yet"
        );
        world.systems[here].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        world.systems[there].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        assert_eq!(
            staging_point(&world, false, None, target, &team),
            Some(there)
        );
        assert_eq!(
            staging_point(&world, false, Some(here), target, &team),
            Some(here)
        );
        world.systems[target].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        assert_eq!(
            staging_point(&world, false, None, target, &team),
            Some(target)
        );
        // The distance counts both axes (squared).
        world.systems[target].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        world.systems[target].y = 100;
        world.systems[here].x = 100;
        world.systems[here].y = 10;
        world.systems[there].x = 100;
        world.systems[there].y = 120;
        assert_eq!(
            staging_point(&world, false, None, target, &team),
            Some(there)
        );
        world.systems[here].y = 95;
        assert_eq!(
            staging_point(&world, false, None, target, &team),
            Some(here)
        );
        world.systems[here].x = 140;
        world.systems[here].y = 100;
        world.systems[there].x = 100;
        world.systems[there].y = 70;
        assert_eq!(
            staging_point(&world, false, None, target, &team),
            Some(there)
        );
    }

    #[test]
    fn each_order_is_its_own_planner_unless_one_runs_for_it_and_held_members_stay_put() {
        // FUN_0042f830 makes one planner per order; the port's heuristics
        // repeat their orders, so one already running for the same kind
        // and target takes the new one's place. FUN_0047b610: a member
        // another planner holds is not taken again.
        let (_, [a, b], [here, there, target]) = gathering_world();
        let (ma, mb) = (MissionMember::Character(a), MissionMember::Character(b));
        let order =
            |kind, team: Vec<MissionMember>, system, character, object| AIAction::DispatchMission {
                kind,
                team,
                decoys: Vec::new(),
                target_system: system,
                target_character: character,
                target_object: object,
            };
        let mut state = AIState::new(AiFaction::Empire);
        let mut actions = vec![
            order(MissionKind::Rescue, vec![ma], target, None, None),
            // The same member again: held, so no planner.
            order(MissionKind::Sabotage, vec![ma], here, None, None),
            order(MissionKind::Rescue, vec![mb], here, None, None),
        ];
        AISystem::adopt_mission_orders(&mut state, 1, 0, &mut actions);
        assert_eq!(state.mission_planners.len(), 2);
        assert_eq!(state.mission_planners[0].team, [ma]);
        assert_eq!(state.mission_planners[1].team, [mb]);
        assert_ne!(state.mission_planners[0].id, state.mission_planners[1].id);
        // A held decoy is dropped, a free one kept.
        let free = MissionMember::SpecialForce(crate::ids::SpecialForceKey::default());
        let mut actions = vec![AIAction::DispatchMission {
            kind: MissionKind::Espionage,
            team: vec![free],
            decoys: vec![ma, MissionMember::Character(CharacterKey::default())],
            target_system: there,
            target_character: None,
            target_object: None,
        }];
        AISystem::adopt_mission_orders(&mut state, 1, 0, &mut actions);
        assert_eq!(
            state.mission_planners[2].decoys,
            [MissionMember::Character(CharacterKey::default())]
        );
        // A planner per differing kind, system, character or object.
        let object = Some(crate::missions::MissionTarget::DeathStar(
            crate::ids::FleetKey::default(),
        ));
        let mut state = AIState::new(AiFaction::Empire);
        for (kind, system, character, object) in [
            (MissionKind::Rescue, target, None, None),
            (MissionKind::Abduction, target, None, None),
            (MissionKind::Rescue, there, None, None),
            (MissionKind::Rescue, target, Some(b), None),
            (MissionKind::Rescue, target, None, object),
        ] {
            AISystem::adopt_mission_orders(
                &mut state,
                1,
                0,
                &mut vec![order(kind, vec![ma], system, character, object)],
            );
            // Free the member for the next variant.
            if let Some(p) = state.mission_planners.last_mut() {
                p.team.clear();
            }
        }
        assert_eq!(state.mission_planners.len(), 5);
        let ids: HashSet<u64> = state.mission_planners.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), 5);
        // An order for a running planner's kind and target adds none.
        let mut state = AIState::new(AiFaction::Empire);
        for member in [ma, mb] {
            AISystem::adopt_mission_orders(
                &mut state,
                1,
                0,
                &mut vec![order(MissionKind::Rescue, vec![member], target, None, None)],
            );
        }
        assert_eq!(state.mission_planners.len(), 1);
        // Other actions pass through untouched.
        let mut passing = vec![order(MissionKind::Move, vec![mb], here, None, None)];
        AISystem::adopt_mission_orders(&mut state, 1, 0, &mut passing);
        assert_eq!(passing.len(), 1);
    }

    #[test]
    fn a_mission_planner_drops_members_it_cannot_keep_and_ends_when_empty_or_old() {
        // State 2 (FUN_0047b110 -> FUN_0047ba40) drops a member that is no
        // longer the side's live record; an empty team ends the planner
        // (FUN_0047aff0). port: so does an age past MISSION_PLANNER_DAYS.
        let (mut world, [a, b], [_, _, target]) = gathering_world();
        let team = vec![MissionMember::Character(a), MissionMember::Character(b)];
        let mut state = AIState::new(AiFaction::Empire);
        let mut actions = vec![rescue_order(team, target)];
        AISystem::adopt_mission_orders(&mut state, 1, 0, &mut actions);
        world.characters[a].mark_killed();
        let cycle = |state: &mut AIState, world: &GameWorld, tick| {
            let mut actions = Vec::new();
            AISystem::mission_planner_cycle(
                state,
                &mut planner_for(world, AiFaction::Empire),
                world,
                &mut crate::ai_records::Galaxy::default(),
                tick,
                &mut actions,
            );
            actions
        };
        cycle(&mut state, &world, 2);
        assert_eq!(
            state.mission_planners[0].team,
            [MissionMember::Character(b)]
        );
        world.characters[b].mark_killed();
        cycle(&mut state, &world, 3);
        assert!(state.mission_planners.is_empty());

        let (world, [a, _], [_, _, target]) = gathering_world();
        let mut actions = vec![rescue_order(vec![MissionMember::Character(a)], target)];
        AISystem::adopt_mission_orders(&mut state, 10, 0, &mut actions);
        cycle(&mut state, &world, 10 + MISSION_PLANNER_DAYS);
        assert_eq!(state.mission_planners.len(), 1);
        cycle(&mut state, &world, 11 + MISSION_PLANNER_DAYS);
        assert!(state.mission_planners.is_empty());
    }

    fn planner_for(world: &GameWorld, faction: AiFaction) -> Planner<'_> {
        Planner::new(
            world,
            &MissionState::new(),
            &HashSet::new(),
            faction == AiFaction::Alliance,
            7,
        )
    }

    /// Each `kind` order as (team, target system, target character).
    fn orders(
        actions: &[AIAction],
        kind: MissionKind,
    ) -> Vec<(Vec<MissionMember>, SystemKey, Option<CharacterKey>)> {
        actions
            .iter()
            .filter_map(|a| match a {
                AIAction::DispatchMission {
                    kind: k,
                    team,
                    target_system,
                    target_character,
                    ..
                } if *k == kind => Some((team.clone(), *target_system, *target_character)),
                _ => None,
            })
            .collect()
    }

    /// `n` special forces of `combat` with no espionage, so none is a decoy.
    fn fighters(world: &mut GameWorld, is_alliance: bool, n: usize) -> Vec<MissionMember> {
        (0..n)
            .map(|_| add_commando(world, is_alliance, 0, 90))
            .collect()
    }

    fn at(world: &mut GameWorld, character: CharacterKey, system: SystemKey) {
        world.characters[character].current_system = Some(system);
    }

    #[test]
    fn the_strategy_keeps_two_recruiters_and_none_go_once_the_pool_is_empty() {
        // FUN_004e8290 row 1: two Recruitment planners while side +0xa4 >
        // 0. FUN_0047dcc0 targets the system the recruiter stands in when
        // its record has +0x2c bit 2. Once the side has recruited its last
        // (FUN_0052f590), a Recruitment would end at once (0x10,
        // FUN_0056b370), and port: none goes out.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let sys = add_system(&mut world, sector, 0.1, 0.9);
        world.systems[sys].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        for _ in 0..3 {
            let recruiter = add_character(&mut world, false, true, 30);
            world.characters[recruiter].leadership.base = 90;
            world.characters[recruiter].current_system = Some(sys);
        }
        for _ in 0..2 {
            let recruit = add_character(&mut world, false, false, 30);
            world.characters[recruit].recruited = false;
            world.characters[recruit].dat_id = DatId(0x3800_0001);
        }
        let run = |world: &GameWorld| {
            let mut state = AIState::new(AiFaction::Empire);
            let actions = strategy_pass(&mut state, world, 1);
            let planners = state
                .mission_planners
                .iter()
                .filter(|p| p.standing && p.kind == MissionKind::Recruitment)
                .count();
            (planners, orders(&actions, MissionKind::Recruitment))
        };

        let (planners, sent) = run(&world);
        assert_eq!(planners, 2);
        assert_eq!(sent.len(), 2);
        assert!(sent.iter().all(|(_, target, _)| *target == sys));

        world.set_recruit_pool_empty(crate::dat::Faction::Empire);

        let (planners, sent) = run(&world);
        assert_eq!(planners, 2);
        assert!(sent.is_empty());
    }

    /// An Empire home system where `agents` (major, by leadership) stand,
    /// and Alliance systems in two sectors at the given Empire support
    /// (percent): `near` at x 0, `far` at x 900.
    fn incite_world(
        agents: &[u32],
        near: &[f32],
        far: &[f32],
    ) -> (
        GameWorld,
        SystemKey,
        Vec<SystemKey>,
        Vec<SystemKey>,
        Vec<CharacterKey>,
    ) {
        let mut world = empty_world();
        let home_sector = add_sector(&mut world);
        let home = add_system(&mut world, home_sector, 0.1, 0.9);
        world.systems[home].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let far_sector = add_sector(&mut world);
        world.sectors[far_sector].x = 900;
        let rebel = |world: &mut GameWorld, sector, support: &f32, x| {
            let s = add_system(world, sector, 0.2, support / 100.0);
            world.systems[s].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
            world.systems[s].x = x;
            s
        };
        let near: Vec<SystemKey> = near
            .iter()
            .enumerate()
            .map(|(i, sup)| rebel(&mut world, home_sector, sup, 10 * (i as u16 + 1)))
            .collect();
        let far: Vec<SystemKey> = far
            .iter()
            .enumerate()
            .map(|(i, sup)| rebel(&mut world, far_sector, sup, 900 + 10 * i as u16))
            .collect();
        let keys = agents
            .iter()
            .map(|&lead| {
                let c = add_character(&mut world, false, true, 30);
                world.characters[c].leadership.base = lead;
                world.characters[c].current_system = Some(home);
                c
            })
            .collect();
        (world, home, near, far, keys)
    }

    #[test]
    fn the_strategy_keeps_one_incite_planner_and_one_more_per_six_candidates() {
        // FUN_004e8290 row 7: +0x350 against +0x218, the opponent systems
        // where the side's support is above 50 (record +0x2c 0x100,
        // sector +0x178) / 6 + 1 when any (FUN_00418500).
        let count = |n: usize| {
            let (world, _, _, _, _) = incite_world(&[], &vec![60.0; n], &[]);
            let mut state = AIState::new(AiFaction::Empire);
            strategy_pass(&mut state, &world, 1);
            state
                .mission_planners
                .iter()
                .filter(|p| p.standing && p.kind == MissionKind::InciteUprising)
                .count()
        };
        assert_eq!([0, 1, 5, 6, 12].map(count), [0, 1, 1, 2, 3]);
        // The planner's cycle orders the Incite on its target.
        let (world, _, near, _, keys) = incite_world(&[60], &[60.0], &[]);
        let mut state = AIState::new(AiFaction::Empire);
        let actions = strategy_pass(&mut state, &world, 1);
        assert_eq!(
            orders(&actions, MissionKind::InciteUprising),
            vec![(vec![MissionMember::Character(keys[0])], near[0], None)]
        );
        // At 50 the side's support is no candidate.
        let (world, _, _, _, _) = incite_world(&[], &[50.0], &[]);
        let mut state = AIState::new(AiFaction::Empire);
        strategy_pass(&mut state, &world, 1);
        assert!(!state
            .mission_planners
            .iter()
            .any(|p| p.kind == MissionKind::InciteUprising));
    }

    #[test]
    fn an_incite_target_is_kept_else_the_nearest_else_the_best_supported() {
        // FUN_00480370. Only the far sector holds candidates, so the
        // sector draws (FUN_0041a1d0) cannot change the sector.
        let (world, home, _, far, keys) = incite_world(&[60], &[], &[60.0, 90.0]);
        let state = AIState::new(AiFaction::Empire);
        let galaxy = galaxy_of(&state, &world);
        let busy = HashSet::new();
        let missions = MissionState::new();
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let mut p = MissionPlanner {
            id: 1,
            kind: MissionKind::InciteUprising,
            team: vec![MissionMember::Character(keys[0])],
            decoys: Vec::new(),
            target_system: None,
            target_character: None,
            target_object: None,
            created: 0,
            last_cycle: 0,
            staging: None,
            standing: true,
            claimed: None,
            sector: None,
            mission: None,
            report: None,
            failures: 0,
            pending: 0,
        };
        let mut target = |p: &MissionPlanner| incite_target(p, &mut planner, &galaxy);
        // Without +0x38: the best supported (v[0], order 1).
        assert_eq!(target(&p), Some(far[1]));
        // From the home system: the nearest to it.
        p.staging = Some(home);
        assert_eq!(target(&p), Some(far[0]));
        // A kept target stays.
        p.target_system = Some(far[1]);
        assert_eq!(target(&p), Some(far[1]));
        // Not a candidate (own): replaced.
        p.target_system = Some(home);
        assert_eq!(target(&p), Some(far[0]));
        p.team.clear();
        assert_eq!(target(&p), None);
    }

    /// Empire systems at the given support (percent), the first `revolts`
    /// of them in an uprising, in sectors `near` (x 0) and `far` (x 900)
    /// by `in_far`; `agents` (major, leadership) stand at a calm one.
    fn subdue_world(
        systems: &[(f32, bool, bool)],
        agents: &[u32],
    ) -> (
        GameWorld,
        crate::uprising::UprisingState,
        Vec<SystemKey>,
        Vec<CharacterKey>,
    ) {
        let mut world = empty_world();
        let near = add_sector(&mut world);
        let far = add_sector(&mut world);
        world.sectors[far].x = 900;
        let mut uprisings = crate::uprising::UprisingState::default();
        let keys: Vec<SystemKey> = systems
            .iter()
            .enumerate()
            .map(|(i, &(support, revolt, in_far))| {
                let sector = if in_far { far } else { near };
                let s = add_system(&mut world, sector, 0.1, support / 100.0);
                world.systems[s].control = ControlKind::Controlled(crate::dat::Faction::Empire);
                world.systems[s].x = if in_far { 900 } else { 0 } + 10 * i as u16;
                if revolt {
                    uprisings.active_uprisings.insert(
                        s,
                        crate::uprising::ActiveUprising {
                            started_tick: 0,
                            next_incident_tick: None,
                            next_support_tick: None,
                            side: None,
                        },
                    );
                }
                s
            })
            .collect();
        let calm = systems.iter().position(|&(_, revolt, _)| !revolt);
        let people = agents
            .iter()
            .map(|&lead| {
                let c = add_character(&mut world, false, true, 30);
                world.characters[c].leadership.base = lead;
                world.characters[c].current_system = calm.map(|i| keys[i]);
                c
            })
            .collect();
        (world, uprisings, keys, people)
    }

    #[test]
    fn the_strategy_keeps_a_subdue_planner_per_uprising_and_one_while_support_is_low() {
        // FUN_004e8290 row 6: while controller bit 0x100 (a sector with
        // +0x28 0x8000000, an own system below 70) is set, +0x330 against
        // +0x21c + 1 (own uprisings, sector +0x164); without the bit, none.
        let count = |systems: &[(f32, bool, bool)]| {
            let (world, uprisings, _, _) = subdue_world(systems, &[]);
            let mut state = AIState::new(AiFaction::Empire);
            strategy_pass_in(&mut state, &world, 1, &uprisings);
            state
                .mission_planners
                .iter()
                .filter(|p| p.standing && p.kind == MissionKind::SubdueUprising)
                .count()
        };
        assert_eq!(count(&[(90.0, false, false)]), 0);
        assert_eq!(count(&[(60.0, false, false)]), 1);
        assert_eq!(count(&[(60.0, true, false), (40.0, true, true)]), 3);

        // The planner's cycle gathers its team at the target (the side's
        // system, FUN_004bd0a0), then orders the Subdue there.
        let (mut world, uprisings, keys, people) =
            subdue_world(&[(90.0, false, false), (30.0, true, false)], &[90]);
        let mut state = AIState::new(AiFaction::Empire);
        let agent = vec![MissionMember::Character(people[0])];
        let actions = strategy_pass_in(&mut state, &world, 1, &uprisings);
        assert_eq!(
            orders(&actions, MissionKind::Move),
            vec![(agent.clone(), keys[1], None)]
        );
        world.characters[people[0]].current_system = Some(keys[1]);
        let actions = strategy_pass_in(&mut state, &world, 2, &uprisings);
        assert_eq!(
            orders(&actions, MissionKind::SubdueUprising),
            vec![(agent, keys[1], None)]
        );
    }

    #[test]
    fn a_subdue_target_is_kept_else_the_nearest_else_the_best_supported_in_the_busiest_sector() {
        // FUN_0047d060: an own uprising with no own fleet (+0x2c 0x20).
        let (world, uprisings, keys, people) = subdue_world(
            &[
                (90.0, false, false),
                (30.0, true, false),
                (40.0, true, true),
                (60.0, true, true),
            ],
            &[60],
        );
        let state = AIState::new(AiFaction::Empire);
        let missions = MissionState::new();
        let busy = HashSet::new();
        let economy = crate::economy::EconomyState::default();
        let manufacturing = ManufacturingState::new();
        let research = ResearchState::new();
        let movement = crate::movement::MovementState::new();
        let transport = crate::troop_transport::TroopTransportState::new();
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let inputs = RecordInputs {
            world: &world,
            economy: &economy,
            uprisings: &uprisings,
            manufacturing: &manufacturing,
            research: &research,
            movement: &movement,
            transport: &transport,
        };
        let galaxy = AISystem::records(&state, &planner, &inputs);
        let mut p = MissionPlanner {
            id: 1,
            kind: MissionKind::SubdueUprising,
            team: vec![MissionMember::Character(people[0])],
            decoys: Vec::new(),
            target_system: None,
            target_character: None,
            target_object: None,
            created: 0,
            last_cycle: 0,
            staging: None,
            standing: true,
            claimed: None,
            sector: None,
            mission: None,
            report: None,
            failures: 0,
            pending: 0,
        };
        let mut target = |p: &MissionPlanner| subdue_target(p, &mut planner, &galaxy);
        // The far sector has two uprisings; its best supported.
        assert_eq!(target(&p), Some(keys[3]));
        // From the calm home: the nearest sector, its revolt.
        p.staging = Some(keys[0]);
        assert_eq!(target(&p), Some(keys[1]));
        p.target_system = Some(keys[2]);
        assert_eq!(target(&p), Some(keys[2]));
        p.target_system = Some(keys[0]);
        assert_eq!(target(&p), Some(keys[1]));
        p.team.clear();
        assert_eq!(target(&p), None);
    }

    /// An Empire home with `scouts` (major, espionage 90) standing there,
    /// and two Alliance systems in a far sector, `explored` or not.
    fn recon_world(
        scouts: usize,
        explored: bool,
    ) -> (GameWorld, SystemKey, [SystemKey; 2], Vec<CharacterKey>) {
        let mut world = empty_world();
        let home_sector = add_sector(&mut world);
        let home = add_system(&mut world, home_sector, 0.1, 0.9);
        world.systems[home].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let far = add_sector(&mut world);
        world.sectors[far].x = 900;
        let rebels = [0, 1].map(|i| {
            let s = add_system(&mut world, far, 0.9, 0.1);
            world.systems[s].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
            world.systems[s].x = 900 + 10 * i;
            if !explored {
                world.systems[s].exploration_status = crate::dat::ExplorationStatus::Unexplored;
            }
            s
        });
        let keys = (0..scouts)
            .map(|_| {
                let c = add_character(&mut world, false, true, 30);
                world.characters[c].espionage.base = 90;
                world.characters[c].current_system = Some(home);
                c
            })
            .collect();
        (world, home, rebels, keys)
    }

    #[test]
    fn the_strategy_keeps_a_recon_planner_per_sector_planner() {
        // FUN_004e8290 row 11: +0x344 against the count of controller
        // +0xd8, the module 8 sector planners.
        let (world, _, _, _) = recon_world(0, false);
        let count = |planners: usize| {
            let mut state = AIState::new(AiFaction::Empire);
            state.module8.planners = vec![crate::ai_module8::SectorPlanner::default(); planners];
            strategy_pass(&mut state, &world, 1);
            state
                .mission_planners
                .iter()
                .filter(|p| p.standing && p.kind == MissionKind::Reconnaissance)
                .count()
        };
        assert_eq!([0, 1, 3].map(count), [0, 1, 3]);
    }

    #[test]
    fn a_recon_planner_claims_its_sector_and_a_second_finds_none_there() {
        // FUN_0047f970: a random unexplored system not the side's
        // (+0x2c 0x80, not 0x8000000) in a sector with one (+0x28 0x40)
        // and no claim (0x10000000), which it then claims. The claim keeps
        // the second planner out; FUN_0047fe10 releases it.
        let (world, _, rebels, keys) = recon_world(2, false);
        let mut state = AIState::new(AiFaction::Empire);
        state.module8.planners = vec![crate::ai_module8::SectorPlanner::default(); 2];
        let actions = strategy_pass(&mut state, &world, 1);
        let scouts = only(&state, MissionKind::Reconnaissance);
        let targets: Vec<Option<SystemKey>> = scouts.iter().map(|p| p.target_system).collect();
        assert_eq!(targets.len(), 2);
        assert!(targets[0].is_some_and(|t| rebels.contains(&t)));
        assert_eq!(targets[1], None);
        let far = world.systems[rebels[0]].sector;
        assert_eq!(scouts[0].sector, Some(far));
        assert_eq!(state.sector_claims.get(&far), Some(&0x1000_0000));
        // The scout stands at home, the side's system nearest the target,
        // so the order goes out at once.
        let sent = orders(&actions, MissionKind::Reconnaissance);
        assert_eq!(sent.len(), 1);
        assert_eq!(Some(sent[0].1), targets[0]);
        assert!(keys.contains(&sent[0].0[0].character().unwrap()));

        // The next day the first keeps its target and the claim holds.
        let kept = targets[0];
        strategy_pass(&mut state, &world, 2);
        let scouts = only(&state, MissionKind::Reconnaissance);
        assert_eq!(scouts[0].target_system, kept);
        assert_eq!(scouts[1].target_system, None);

        // With no sector planner left, both go and the claim with them.
        state.module8.planners.clear();
        strategy_pass(&mut state, &world, 3);
        assert!(!state
            .mission_planners
            .iter()
            .any(|p| p.kind == MissionKind::Reconnaissance));
        assert!(state.sector_claims.values().all(|b| b & 0x1000_0000 == 0));
    }

    #[test]
    fn while_a_sector_lacks_a_capital_recon_takes_only_unexplored_systems() {
        // FUN_0047f970 with controller +0x1c4 >= 1: FUN_0041a460 forbids
        // +0x2c 0x8000000 (explored), so explored enemy systems give none.
        let (world, _, _, _) = recon_world(1, true);
        let mut state = AIState::new(AiFaction::Empire);
        state.module8.planners = vec![crate::ai_module8::SectorPlanner::default()];
        strategy_pass(&mut state, &world, 1);
        assert_eq!(state.mission_planners[0].target_system, None);
        assert!(state.sector_claims.is_empty());
    }

    #[test]
    fn with_every_sector_holding_a_capital_recon_takes_the_system_seen_longest_ago() {
        // FUN_0047f970 with controller +0x1c4 < 1 (no sector record without
        // +0x24 0x40000000, set by a headquarters or Coruscant,
        // FUN_00432350): the sector may lack only the claim (0x10000000),
        // and the system is the one not seen for longest (FUN_0041a3a0,
        // order 1), explored or not.
        let (mut world, home, rebels, keys) = recon_world(1, true);
        world.systems[home].is_headquarters = true;
        let far = world.systems[rebels[0]].sector;
        let capital = add_system(&mut world, far, 0.1, 0.9);
        world.systems[capital].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        world.systems[capital].is_headquarters = true;
        let mut state = AIState::new(AiFaction::Empire);
        state.seen.insert(rebels[0], 5);
        let galaxy = galaxy_of(&state, &world);
        assert!(galaxy
            .sectors
            .iter()
            .all(|s| s.candidates & 0x4000_0000 != 0));
        let mut galaxy = galaxy;
        let missions = MissionState::new();
        let busy = HashSet::new();
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let mut p = MissionPlanner {
            id: 1,
            kind: MissionKind::Reconnaissance,
            team: vec![MissionMember::Character(keys[0])],
            decoys: Vec::new(),
            target_system: None,
            target_character: None,
            target_object: None,
            created: 0,
            last_cycle: 0,
            staging: Some(home),
            standing: true,
            claimed: None,
            sector: None,
            mission: None,
            report: None,
            failures: 0,
            pending: 0,
        };
        let mut claims = HashMap::new();
        let mut seen = state.seen.clone();
        let target = recon_target(&mut p, &mut planner, &mut galaxy, &mut claims, (10, &seen));
        assert_eq!(target, Some(rebels[1]));
        seen.insert(rebels[1], 9);
        release_sector(&mut claims, &mut galaxy, far, 0x1000_0000);
        p.target_system = None;
        let target = recon_target(&mut p, &mut planner, &mut galaxy, &mut claims, (10, &seen));
        assert_eq!(target, Some(rebels[0]));
    }

    #[test]
    fn the_strategy_keeps_an_espionage_planner_per_enemy_sector_beyond_the_sector_planners() {
        // FUN_004e8290 row 12: +0x348 against +0x1bc (sectors with an
        // opponent's system, sector +0x2c 8; FUN_0041b7e0) less the
        // length of controller +0xd8.
        let (mut world, _, _, _) = recon_world(0, true);
        let third = add_sector(&mut world);
        let rebel = add_system(&mut world, third, 0.9, 0.1);
        world.systems[rebel].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let count = |planners: usize| {
            let mut state = AIState::new(AiFaction::Empire);
            state.module8.planners = vec![crate::ai_module8::SectorPlanner::default(); planners];
            strategy_pass(&mut state, &world, 1);
            state
                .mission_planners
                .iter()
                .filter(|p| p.standing && p.kind == MissionKind::Espionage)
                .count()
        };
        assert_eq!([0, 1, 2, 3].map(count), [2, 1, 0, 0]);
    }

    #[test]
    fn an_espionage_planner_takes_the_enemy_system_seen_longest_ago_and_claims_its_sector() {
        // FUN_00481110: an opponent's system (+0x30 4, +0x2c 0x200) seen
        // longest ago (FUN_0041a3a0, order 1) in an unclaimed sector, which
        // it claims (+0x28 0x20000000); kept while still the opponent's,
        // then released (FUN_00481420).
        // FUN_00480f30 takes a special force (record +0x30 0x10000000) of
        // espionage 51 or more.
        let (mut world, home, rebels, _) = recon_world(0, true);
        let spy = add_commando(&mut world, false, 60, 0);
        let MissionMember::SpecialForce(key) = spy else {
            unreachable!()
        };
        world.systems[home].special_forces.push(key);
        // The Abduction planner, listed first, takes up to four decoys
        // (FUN_00483410, best espionage first) even without a team; four
        // better special forces fill it.
        for _ in 0..4 {
            let MissionMember::SpecialForce(other) = add_commando(&mut world, false, 90, 0) else {
                unreachable!()
            };
            world.systems[home].special_forces.push(other);
        }
        let mut state = AIState::new(AiFaction::Empire);
        state.seen.insert(rebels[1], 1);
        let actions = strategy_pass(&mut state, &world, 10);
        let p = state
            .mission_planners
            .iter()
            .find(|p| p.kind == MissionKind::Espionage)
            .unwrap()
            .clone();
        assert_eq!(p.target_system, Some(rebels[0]));
        let far = world.systems[rebels[0]].sector;
        assert_eq!(p.sector, Some(far));
        assert_eq!(state.sector_claims.get(&far), Some(&0x2000_0000));
        assert_eq!(
            orders(&actions, MissionKind::Espionage),
            vec![(vec![spy], rebels[0], None)]
        );

        // Seen since: the target stays while it is the opponent's.
        state.seen.insert(rebels[0], 9);
        strategy_pass(&mut state, &world, 11);
        let target = |state: &AIState| {
            state
                .mission_planners
                .iter()
                .find(|p| p.kind == MissionKind::Espionage)
                .and_then(|p| p.target_system)
        };
        assert_eq!(target(&state), Some(rebels[0]));

        // Taken by the side: the next opponent's system, in the same sector.
        world.systems[rebels[0]].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        strategy_pass(&mut state, &world, 12);
        assert_eq!(target(&state), Some(rebels[1]));
        assert_eq!(state.sector_claims.get(&far), Some(&0x2000_0000));

        // None left: the claim goes.
        world.systems[rebels[1]].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        strategy_pass(&mut state, &world, 13);
        assert!(state.sector_claims.is_empty());
    }

    #[test]
    fn an_unexplored_enemy_system_is_no_espionage_target() {
        // FUN_00481110 keeps +0x34 only with record +0x30 4 and +0x2c
        // 0x200; an unexplored system's record stops at +0x2c 0x80
        // (FUN_00433e40), so even a kept one is dropped.
        let (world, _, rebels, _) = recon_world(0, false);
        let state = AIState::new(AiFaction::Empire);
        let mut galaxy = galaxy_of(&state, &world);
        let missions = MissionState::new();
        let busy = HashSet::new();
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let mut p = MissionPlanner {
            id: 1,
            kind: MissionKind::Espionage,
            team: Vec::new(),
            decoys: Vec::new(),
            target_system: Some(rebels[0]),
            target_character: None,
            target_object: None,
            created: 0,
            last_cycle: 0,
            staging: None,
            standing: true,
            claimed: None,
            sector: None,
            mission: None,
            report: None,
            failures: 0,
            pending: 0,
        };
        let mut claims = HashMap::new();
        let seen = HashMap::new();
        let target = espionage_target(&mut p, &mut planner, &mut galaxy, &mut claims, (5, &seen));
        assert_eq!(target, None);
    }

    #[test]
    fn the_refresh_keeps_the_sector_claims_and_a_claim_is_seen_at_once() {
        // FUN_004321e0 keeps sector +0x28 & 0x30000000; the claim and its
        // release (FUN_0047f970, FUN_0047fe10) change the record in place.
        let (world, _, rebels, _) = recon_world(0, false);
        let far = world.systems[rebels[0]].sector;
        let plain = galaxy_of(&AIState::new(AiFaction::Empire), &world);
        let before = plain.sector(far).unwrap().state;
        let mut state = AIState::new(AiFaction::Empire);
        state.sector_claims.insert(far, 0xffff_ffff);
        let claimed = galaxy_of(&state, &world);
        assert_eq!(claimed.sector(far).unwrap().state, before | 0x3000_0000);
        let mut galaxy = plain;
        galaxy.update_sector_state(far, 0x1000_0000, 0);
        galaxy.update_sector_state(far, 0x1000_0000, 0);
        assert_eq!(galaxy.sector(far).unwrap().state, before | 0x1000_0000);
        galaxy.update_sector_state(far, 0, 0x1000_0000);
        assert_eq!(galaxy.sector(far).unwrap().state, before);
    }

    #[test]
    fn a_recon_target_that_becomes_the_side_s_is_dropped_and_its_sector_freed() {
        // FUN_0047f970 keeps +0x34 only while its record has +0x2c 0x80;
        // otherwise it releases +0x70 first.
        let (mut world, _, rebels, _) = recon_world(1, false);
        let mut state = AIState::new(AiFaction::Empire);
        state.module8.planners = vec![crate::ai_module8::SectorPlanner::default()];
        strategy_pass(&mut state, &world, 1);
        let first = state.mission_planners[0].target_system.unwrap();
        for s in rebels {
            world.systems[s].control = ControlKind::Controlled(crate::dat::Faction::Empire);
            world.systems[s].exploration_status = crate::dat::ExplorationStatus::Explored;
        }
        strategy_pass(&mut state, &world, 2);
        assert_ne!(state.mission_planners[0].target_system, Some(first));
        assert_eq!(state.mission_planners[0].target_system, None);
        assert_eq!(state.mission_planners[0].sector, None);
        assert!(state.sector_claims.is_empty());
    }

    /// An Empire home with an abductor (combat 80) and `decoys` special
    /// forces there, and two Alliance systems in a far sector: `guarded`
    /// with Leia (`0x241`), Luke (`0x242`) and two Alliance regiments,
    /// `open` with Han (`0x243`) and none.
    fn abduction_world(
        decoys: usize,
    ) -> (
        GameWorld,
        SystemKey,
        SystemKey,
        SystemKey,
        [CharacterKey; 2],
        CharacterKey,
    ) {
        let mut world = empty_world();
        let home_sector = add_sector(&mut world);
        let home = add_system(&mut world, home_sector, 0.1, 0.9);
        world.systems[home].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let far = add_sector(&mut world);
        world.sectors[far].x = 900;
        let rebel = |world: &mut GameWorld, x| {
            let s = add_system(world, far, 0.9, 0.1);
            world.systems[s].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
            world.systems[s].x = x;
            s
        };
        let guarded = rebel(&mut world, 900);
        let open = rebel(&mut world, 910);
        for _ in 0..2 {
            let troop = world.troops.insert(crate::world::TroopUnit {
                class_dat_id: DatId(0),
                is_alliance: true,
                regiment_strength: 50,
            });
            world.systems[guarded].ground_units.push(troop);
        }
        let main = |world: &mut GameWorld, id, at| {
            let c = add_character(world, true, true, 30);
            world.characters[c].dat_id = DatId(id);
            world.characters[c].current_system = Some(at);
            c
        };
        let leia = main(&mut world, 0x241, guarded);
        // Luke too, so the guarded system's v[47] (2) ranks it first.
        main(&mut world, 0x242, guarded);
        let han = main(&mut world, 0x243, open);
        // Not a commander: FUN_00483290 forbids capability bits 0x7000000.
        let abductor = add_character(&mut world, false, true, 30);
        world.characters[abductor].combat.base = 80;
        world.characters[abductor].can_be_commander = false;
        world.characters[abductor].current_system = Some(home);
        for _ in 0..decoys {
            let MissionMember::SpecialForce(key) = add_commando(&mut world, false, 60, 0) else {
                unreachable!()
            };
            world.systems[home].special_forces.push(key);
        }
        (world, home, guarded, open, [leia, han], abductor)
    }

    #[test]
    fn an_abduction_goes_where_the_gathered_decoys_match_half_the_defense() {
        // FUN_00483640: the first system holding an opponent's main
        // character (+0x2c 0x1000) where the decoys gathered at +0x38
        // (FUN_004bd000, list +0x44) fill the cap (4) or reach half the
        // defense (record +0xbc); then that character (FUN_00483d10).
        let (world, home, _, open, [_, han], abductor) = abduction_world(0);
        let mut state = AIState::new(AiFaction::Empire);
        let actions = strategy_pass(&mut state, &world, 1);
        let p = only(&state, MissionKind::Abduction).remove(0);
        assert_eq!(
            (p.target_system, p.target_character),
            (Some(open), Some(han))
        );
        assert_eq!(p.team, vec![MissionMember::Character(abductor)]);
        assert_eq!(p.staging, Some(home));
        assert_eq!(
            orders(&actions, MissionKind::Abduction),
            vec![(vec![MissionMember::Character(abductor)], open, Some(han))]
        );

        // One decoy gathered at home covers the guarded system's two
        // regiments (2 / 2 <= 1), which comes first by key v[47].
        let (world, home, guarded, _, [leia, _], _) = abduction_world(1);
        let galaxy = galaxy_of(&AIState::new(AiFaction::Empire), &world);
        assert_eq!(galaxy.system(guarded).unwrap().values[29], 2);
        let missions = MissionState::new();
        let busy = HashSet::new();
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let decoy = world.systems[home].special_forces[0];
        let mut p = MissionPlanner {
            decoys: vec![MissionMember::SpecialForce(decoy)],
            target_system: None,
            target_character: None,
            ..p
        };
        assert_eq!(
            abduction_target(&mut p, &mut planner, &world, &galaxy),
            Some(guarded)
        );
        let first = p.target_character.unwrap();
        assert_eq!(world.characters[first].current_system, Some(guarded));
        // Kept while the character stands there.
        assert_eq!(
            abduction_target(&mut p, &mut planner, &world, &galaxy),
            Some(guarded)
        );
        assert_eq!(p.target_character, Some(first));
        // A target system the character is not in: chosen again.
        p.target_system = Some(home);
        assert_eq!(
            abduction_target(&mut p, &mut planner, &world, &galaxy),
            Some(guarded)
        );
        // The decoy counts only where the planner gathers (+0x38).
        p.target_system = None;
        p.target_character = None;
        p.staging = Some(open);
        assert_eq!(
            abduction_target(&mut p, &mut planner, &world, &galaxy),
            Some(open)
        );
        let _ = (leia, han);
    }

    /// An Alliance home with a rescuer (combat 80), and an Empire prison
    /// system in a far sector holding Luke (`0x242`) and, with `guards`
    /// Empire regiments, Leia (`0x241`) in a second prison; both captive.
    fn rescue_world(
        guards: usize,
    ) -> (
        GameWorld,
        SystemKey,
        [SystemKey; 2],
        [CharacterKey; 2],
        CharacterKey,
    ) {
        let mut world = empty_world();
        let home_sector = add_sector(&mut world);
        let home = add_system(&mut world, home_sector, 0.9, 0.1);
        world.systems[home].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let far = add_sector(&mut world);
        world.sectors[far].x = 900;
        let prison = |world: &mut GameWorld, x| {
            let s = add_system(world, far, 0.1, 0.9);
            world.systems[s].control = ControlKind::Controlled(crate::dat::Faction::Empire);
            world.systems[s].x = x;
            s
        };
        let open = prison(&mut world, 900);
        let guarded = prison(&mut world, 910);
        for _ in 0..guards {
            let troop = world.troops.insert(crate::world::TroopUnit {
                class_dat_id: DatId(0),
                is_alliance: false,
                regiment_strength: 50,
            });
            world.systems[guarded].ground_units.push(troop);
        }
        let captive = |world: &mut GameWorld, id, at| {
            let c = add_character(world, true, true, 30);
            world.characters[c].dat_id = DatId(id);
            world.characters[c].current_system = Some(at);
            world.characters[c].is_captive = true;
            world.characters[c].captured_by = Some(crate::dat::Faction::Empire);
            c
        };
        let luke = captive(&mut world, 0x242, open);
        let leia = captive(&mut world, 0x241, guarded);
        let rescuer = add_character(&mut world, true, true, 30);
        world.characters[rescuer].combat.base = 80;
        world.characters[rescuer].can_be_commander = false;
        world.characters[rescuer].current_system = Some(home);
        // MISSNSD Abduction admitting no one, so the Abduction planner
        // (listed first, combat 51..) does not take the rescuer.
        world.mission_records = vec![crate::world::MissionRecord {
            dat_id: MissionKind::Abduction.record_id().unwrap(),
            timer_min_days: 5,
            timer_spread_days: 10,
            repeats: false,
            hidden: false,
            detection_phases: true,
            can_resign: true,
            rules: crate::world::MissionTargetRules::default(),
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0,
                character_mask: 0,
            },
        }];
        (world, home, [open, guarded], [luke, leia], rescuer)
    }

    #[test]
    fn the_strategy_keeps_a_rescue_planner_while_a_character_of_the_side_is_a_prisoner() {
        // FUN_004e8290 row 10: one while controller bit 0x200 (FUN_0041bc10:
        // a record of the side's character with 0x40) is set, else none.
        let (mut world, home, [open, guarded], [luke, leia], rescuer) = rescue_world(0);
        let mut state = AIState::new(AiFaction::Alliance);
        let actions = strategy_pass(&mut state, &world, 1);
        let p = only(&state, MissionKind::Rescue);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].staging, Some(home));
        // Both prisons are open; the planet list ranks by defense (v[29],
        // order 1), equal here, so either prisoner may come first.
        let sent = orders(&actions, MissionKind::Rescue);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, vec![MissionMember::Character(rescuer)]);
        assert!([(open, Some(luke)), (guarded, Some(leia))].contains(&(sent[0].1, sent[0].2)));

        for c in [luke, leia] {
            world.characters[c].is_captive = false;
            world.characters[c].captured_by = None;
        }
        strategy_pass(&mut state, &world, 2);
        assert!(only(&state, MissionKind::Rescue).is_empty());
    }

    #[test]
    fn a_rescue_target_is_chosen_anew_where_the_decoys_cover_half_the_defense() {
        // FUN_0047e620 never keeps its character ((flags | 0x20) == 0 at
        // 0x47e800 never holds) and takes the first prison whose defense
        // (record +0xbc) the gathered decoys halve, or that fills the cap;
        // FUN_0047ecf0 then draws a prisoner of the side there.
        let (world, home, [open, guarded], [luke, leia], rescuer) = rescue_world(2);
        let galaxy = galaxy_of(&AIState::new(AiFaction::Alliance), &world);
        assert_eq!(galaxy.system(guarded).unwrap().values[29], 2);
        assert_eq!(galaxy.system(open).unwrap().summary & 0x4000, 0x4000);
        let missions = MissionState::new();
        let busy = HashSet::new();
        let mut planner = Planner::new(&world, &missions, &busy, true, 7);
        let mut p = MissionPlanner {
            id: 1,
            kind: MissionKind::Rescue,
            team: vec![MissionMember::Character(rescuer)],
            decoys: Vec::new(),
            target_system: Some(guarded),
            target_character: Some(leia),
            target_object: None,
            created: 0,
            last_cycle: 0,
            staging: Some(home),
            standing: true,
            claimed: None,
            sector: None,
            mission: None,
            report: None,
            failures: 0,
            pending: 0,
        };
        // The guarded prison ranks first (v[29] 2, order 1) but needs a
        // decoy; with none, the open one, and Luke.
        assert_eq!(
            rescue_target(&mut p, &mut planner, &world, &galaxy),
            Some(open)
        );
        assert_eq!(p.target_character, Some(luke));
        let picks: HashSet<CharacterKey> = (0..20)
            .filter_map(|_| rescue_character(&mut planner, &world, &galaxy, open))
            .collect();
        assert_eq!(picks, HashSet::from([luke]));
        // A free character of the side or an Empire one is no prisoner of
        // the side to rescue.
        assert!(!own_prisoner(&planner, rescuer));
    }

    #[test]
    fn a_rescue_counts_the_decoys_where_it_gathers_and_reaches_prisoners_aboard_enemy_fleets() {
        // FUN_004bd000 counts the decoys at +0x38; one covers two guards
        // (2 / 2 <= 1). FUN_0047ecf0 takes a prisoner aboard an opponent's
        // fleet carrying one (fleet record +0x38 0x1000000, system +0x2c
        // 0x8000).
        let (mut world, home, [open, guarded], [luke, leia], rescuer) = rescue_world(2);
        let MissionMember::SpecialForce(decoy) = add_commando(&mut world, true, 60, 0) else {
            unreachable!()
        };
        world.systems[home].special_forces.push(decoy);
        let galaxy = galaxy_of(&AIState::new(AiFaction::Alliance), &world);
        let missions = MissionState::new();
        let busy = HashSet::new();
        let mut planner = Planner::new(&world, &missions, &busy, true, 7);
        let mut p = MissionPlanner {
            id: 1,
            kind: MissionKind::Rescue,
            team: vec![MissionMember::Character(rescuer)],
            decoys: vec![MissionMember::SpecialForce(decoy)],
            target_system: None,
            target_character: None,
            target_object: None,
            created: 0,
            last_cycle: 0,
            staging: Some(home),
            standing: true,
            claimed: None,
            sector: None,
            mission: None,
            report: None,
            failures: 0,
            pending: 0,
        };
        assert_eq!(
            rescue_target(&mut p, &mut planner, &world, &galaxy),
            Some(guarded)
        );
        assert_eq!(p.target_character, Some(leia));
        p.staging = Some(open);
        assert_eq!(
            rescue_target(&mut p, &mut planner, &world, &galaxy),
            Some(open)
        );
        assert_eq!(p.target_character, Some(luke));

        // Luke moved aboard an Empire fleet at his prison.
        let class = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        let fleet = world.fleets.insert(Fleet {
            location: open,
            capital_ships: ShipInstance::make(class, 100, false, 1),
            fighters: vec![],
            characters: vec![luke],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[open].fleets.push(fleet);
        world.characters[luke].current_system = None;
        world.characters[luke].current_fleet = Some(fleet);
        let galaxy = galaxy_of(&AIState::new(AiFaction::Alliance), &world);
        assert_eq!(galaxy.system(open).unwrap().summary & 0xc000, 0x8000);
        let mut planner = Planner::new(&world, &missions, &busy, true, 7);
        let picks: HashSet<CharacterKey> = (0..20)
            .filter_map(|_| rescue_character(&mut planner, &world, &galaxy, open))
            .collect();
        assert_eq!(picks, HashSet::from([luke]));
        assert_eq!(rescue_character(&mut planner, &world, &galaxy, home), None);
    }

    #[test]
    fn an_abduction_takes_only_opponents_main_characters_on_the_planet_or_aboard_their_fleets() {
        // FUN_00483d10: record +0x30 8 and 4; on the planet with +0x2c
        // 0x1000, aboard a fleet not the side's carrying one (fleet record
        // +0x38 0x200000) with 0x2000. FUN_00483640 keeps its character
        // only while not a prisoner or travelling (not 0x60).
        let mut world = empty_world();
        let home_sector = add_sector(&mut world);
        let home = add_system(&mut world, home_sector, 0.1, 0.9);
        world.systems[home].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let target = add_system(&mut world, home_sector, 0.9, 0.1);
        world.systems[target].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let person = |world: &mut GameWorld, alliance: bool, id: u32| {
            let c = add_character(world, alliance, true, 30);
            world.characters[c].dat_id = DatId(id);
            world.characters[c].current_system = Some(target);
            c
        };
        let luke = person(&mut world, true, 0x242);
        let _minor = person(&mut world, true, 0x2c7);
        let vader = person(&mut world, false, 0x281);
        let leia = person(&mut world, true, 0x241);
        let class = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        let fleet = world.fleets.insert(Fleet {
            location: target,
            capital_ships: ShipInstance::make(class, 100, true, 1),
            fighters: vec![],
            characters: vec![leia],
            is_alliance: true,
            has_death_star: false,
        });
        world.systems[target].fleets.push(fleet);
        world.characters[leia].current_system = None;
        world.characters[leia].current_fleet = Some(fleet);
        // An Empire fleet with Vader aboard does not count.
        let own = world.fleets.insert(Fleet {
            location: target,
            capital_ships: ShipInstance::make(class, 100, false, 1),
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[target].fleets.push(own);
        let state = AIState::new(AiFaction::Empire);
        let galaxy = galaxy_of(&state, &world);
        assert_eq!(galaxy.system(target).unwrap().summary & 0x3000, 0x3000);
        let missions = MissionState::new();
        let busy = HashSet::new();
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let picks: HashSet<CharacterKey> = (0..40)
            .filter_map(|_| abduction_character(&mut planner, &world, &galaxy, target))
            .collect();
        assert_eq!(picks, HashSet::from([luke, leia]));

        // Luke gone, and Vader a prisoner of the Alliance there: his
        // record reads 8, 2 and 0x40 (a prisoner counts on its home side,
        // FUN_00401d20), so no opponent's main character is free on the
        // planet (no 0x1000) and only Leia aboard is left. A planner aimed
        // at Vader picks again.
        let mut world = world;
        world.characters[luke].current_system = None;
        world.characters[vader].is_captive = true;
        world.characters[vader].captured_by = Some(crate::dat::Faction::Alliance);
        let galaxy = galaxy_of(&state, &world);
        assert_eq!(galaxy.system(target).unwrap().summary & 0x3000, 0x2000);
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let picks: HashSet<CharacterKey> = (0..40)
            .filter_map(|_| abduction_character(&mut planner, &world, &galaxy, target))
            .collect();
        assert_eq!(picks, HashSet::from([leia]));
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let mut p = MissionPlanner {
            id: 1,
            kind: MissionKind::Abduction,
            team: Vec::new(),
            decoys: Vec::new(),
            target_system: Some(target),
            target_character: Some(vader),
            target_object: None,
            created: 0,
            last_cycle: 0,
            staging: Some(home),
            standing: true,
            claimed: None,
            sector: None,
            mission: None,
            report: None,
            failures: 0,
            pending: 0,
        };
        assert_eq!(
            abduction_target(&mut p, &mut planner, &world, &galaxy),
            Some(target)
        );
        assert_eq!(p.target_character, Some(leia));

        // Two regiments now guard the system, so no new choice would take
        // it with no decoy gathered; Leia, free aboard, is kept all the
        // same (record 8 and 4, not 0x60).
        for _ in 0..2 {
            let troop = world.troops.insert(crate::world::TroopUnit {
                class_dat_id: DatId(0),
                is_alliance: true,
                regiment_strength: 50,
            });
            world.systems[target].ground_units.push(troop);
        }
        let galaxy = galaxy_of(&state, &world);
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        assert_eq!(
            abduction_target(&mut p, &mut planner, &world, &galaxy),
            Some(target)
        );
        assert_eq!(p.target_character, Some(leia));
        p.target_character = None;
        assert_eq!(
            abduction_target(&mut p, &mut planner, &world, &galaxy),
            None
        );
    }

    /// The side's records (phase 4) for `world`, as `state` keeps them.
    fn galaxy_of(state: &AIState, world: &GameWorld) -> crate::ai_records::Galaxy {
        let missions = MissionState::new();
        let busy = HashSet::new();
        let economy = crate::economy::EconomyState::default();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = ManufacturingState::new();
        let research = ResearchState::new();
        let movement = crate::movement::MovementState::new();
        let transport = crate::troop_transport::TroopTransportState::new();
        let is_alliance = matches!(state.faction, Some(AiFaction::Alliance));
        let planner = Planner::new(world, &missions, &busy, is_alliance, 7);
        let inputs = RecordInputs {
            world,
            economy: &economy,
            uprisings: &uprisings,
            manufacturing: &manufacturing,
            research: &research,
            movement: &movement,
            transport: &transport,
        };
        AISystem::records(state, &planner, &inputs)
    }

    #[test]
    fn a_recruitment_target_is_the_recruiter_s_calm_system_else_by_count_and_support_else_by_distance(
    ) {
        // FUN_0047dcc0: a system with record +0x2c bit 2. The recruiter
        // stands in a neutral system. Without +0x38: the sector with the
        // most such systems (key +0x160, order 1), then its best supported
        // (v[0], order 1). With +0x38: the nearest sector with one (order
        // 2), then its system nearest +0x38.
        let mut world = empty_world();
        let near = add_sector(&mut world);
        let far = add_sector(&mut world);
        world.sectors[far].x = 900;
        let own = |world: &mut GameWorld, sector, support: f32, x: u16| {
            let s = add_system(world, sector, 0.0, support);
            world.systems[s].control = ControlKind::Controlled(crate::dat::Faction::Empire);
            world.systems[s].x = x;
            s
        };
        let lone = own(&mut world, near, 0.95, 10);
        let low = own(&mut world, far, 0.6, 900);
        let high = own(&mut world, far, 0.8, 990);
        let neutral = add_system(&mut world, near, 0.3, 0.3);
        let recruiter = add_character(&mut world, false, true, 30);
        world.characters[recruiter].current_system = Some(neutral);
        let minor = add_character(&mut world, false, false, 30);
        world.characters[minor].dat_id = DatId(0x3800_0001);
        world.characters[minor].current_system = Some(lone);
        let state = AIState::new(AiFaction::Empire);
        let galaxy = galaxy_of(&state, &world);
        let mut p = MissionPlanner {
            id: 1,
            kind: MissionKind::Recruitment,
            team: vec![MissionMember::Character(recruiter)],
            decoys: Vec::new(),
            target_system: None,
            target_character: None,
            target_object: None,
            created: 0,
            last_cycle: 0,
            staging: None,
            standing: true,
            claimed: None,
            sector: None,
            mission: None,
            report: None,
            failures: 0,
            pending: 0,
        };
        let busy = HashSet::new();
        let missions = MissionState::new();
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let mut target = |p: &MissionPlanner| recruitment_target(p, &mut planner, &world, &galaxy);

        assert_eq!(target(&p), Some(high));
        p.staging = Some(neutral);
        assert_eq!(target(&p), Some(lone));
        p.staging = Some(low);
        assert_eq!(target(&p), Some(low));
        // A kept target and the recruiter's own calm system come first.
        p.target_system = Some(high);
        assert_eq!(target(&p), Some(high));
        p.target_system = Some(neutral);
        p.staging = None;
        p.team = vec![MissionMember::Character(minor)];
        assert_eq!(target(&p), Some(lone));
        p.team.clear();
        assert_eq!(target(&p), None);
    }

    #[test]
    fn a_sabotage_goes_to_each_enemy_yard() {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let mut yards = Vec::new();
        for _ in 0..2 {
            let system = add_system(&mut world, sector, 0.8, 0.1);
            let yard = world.manufacturing_facilities.insert(
                crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId(1),
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: false,
                },
            );
            world.systems[system].manufacturing_facilities.push(yard);
            yards.push(system);
        }
        for _ in 0..4 {
            add_commando(&mut world, false, 0, 90);
        }
        for member in world.special_forces.values_mut() {
            member.skills[crate::world::Skill::Espionage as usize] = 90;
        }
        let mut planner = planner_for(&world, AiFaction::Empire);
        let mut actions = Vec::new();

        AISystem::evaluate_espionage(
            &mut planner,
            &world,
            AiFaction::Empire,
            &GameConfig::default(),
            &mut actions,
        );

        let targets: Vec<SystemKey> = orders(&actions, MissionKind::Sabotage)
            .into_iter()
            .map(|(_, system, _)| system)
            .collect();
        assert_eq!(targets.len(), 2);
        assert!(yards.iter().all(|yard| targets.contains(yard)));
    }

    #[test]
    fn an_assassination_goes_where_a_free_enemy_major_stands_up_to_the_covert_cap() {
        // FUN_004bd0a0 sends the order to the target's system; a character
        // in a fleet stands at the fleet's system.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let here = add_system(&mut world, sector, 0.8, 0.1);
        let docked = add_system(&mut world, sector, 0.8, 0.1);
        let fleet = world.fleets.insert(Fleet {
            location: docked,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        let target = add_character(&mut world, true, true, 30);
        at(&mut world, target, here);
        let aboard = add_character(&mut world, true, true, 30);
        world.characters[aboard].current_fleet = Some(fleet);
        for (captive, killed) in [(true, false), (false, true)] {
            let spared = add_character(&mut world, true, true, 30);
            at(&mut world, spared, here);
            world.characters[spared].is_captive = captive;
            world.characters[spared].is_killed = killed;
        }
        let own = add_character(&mut world, false, true, 30);
        at(&mut world, own, here);
        for _ in 0..2 {
            let other = add_character(&mut world, true, true, 30);
            at(&mut world, other, here);
        }
        fighters(&mut world, false, 10);
        let mut planner = planner_for(&world, AiFaction::Empire);
        let mut actions = Vec::new();

        AISystem::evaluate_espionage(
            &mut planner,
            &world,
            AiFaction::Empire,
            &GameConfig::default(),
            &mut actions,
        );

        let sent = orders(&actions, MissionKind::Assassination);
        assert_eq!(sent.len(), GameConfig::default().ai.max_covert_ops_per_eval);
        assert!(sent.iter().any(|(_, _, c)| *c == Some(target)));
        assert!(sent.iter().any(|(_, _, c)| *c == Some(aboard)));
        for (_, system, character) in &sent {
            let character = character.unwrap();
            assert!(character != own && !world.characters[character].is_captive);
            assert!(!world.characters[character].is_killed);
            let expected = if character == aboard { docked } else { here };
            assert_eq!(*system, expected);
        }
    }

    /// An Alliance system holding a yard of `family` (MANFACSD `0x28`
    /// shipyard, `0x29` training facility, `0x2a` construction yard) and an
    /// Alliance regiment: a research site of the yard's tree.
    fn yard_system(world: &mut GameWorld, sector: SectorKey, family: u32) -> SystemKey {
        let system = add_system(world, sector, 0.5, 0.5);
        world.systems[system].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let yard =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId::new(family << 24 | 1),
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: family == 0x28,
                });
        world.systems[system].manufacturing_facilities.push(yard);
        garrison(world, system, true);
        system
    }

    fn garrison(world: &mut GameWorld, system: SystemKey, is_alliance: bool) {
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance,
            regiment_strength: 50,
        });
        world.systems[system].ground_units.push(troop);
    }

    fn designer(world: &mut GameWorld, tech: TechType, base: u32) -> MissionMember {
        let key = add_character(world, true, false, 0);
        let pair = SkillPair { base, variance: 0 };
        let c = &mut world.characters[key];
        match tech {
            TechType::Ship => c.ship_design = pair,
            TechType::Troop => c.troop_training = pair,
            TechType::Facility => c.facility_design = pair,
        }
        MissionMember::Character(key)
    }

    /// One AI evaluation's research part for the Alliance at `tick`.
    fn research_pass(
        state: &mut AIState,
        world: &GameWorld,
        missions: &MissionState,
        research: &ResearchState,
        tick: u64,
    ) -> Vec<AIAction> {
        let mut planner = Planner::new(world, missions, &state.busy_characters.clone(), true, tick);
        let mut actions = Vec::new();
        let (mfg, config, uprisings) = (
            ManufacturingState::new(),
            GameConfig::default(),
            crate::uprising::UprisingState::default(),
        );
        let ctx = ModuleInputs {
            tick,
            ..inputs(world, missions, research, &mfg, &config, &uprisings)
        };
        research_all(state, &mut planner, &ctx, &mut actions);
        actions
    }

    fn research_orders(
        world: &GameWorld,
        missions: &MissionState,
        research: &ResearchState,
    ) -> Vec<AIAction> {
        research_pass(
            &mut AIState::new(AiFaction::Alliance),
            world,
            missions,
            research,
            7,
        )
    }

    /// Dispatch `member`'s research the way the integrator applies an
    /// order, binding it to its planner.
    fn apply_order(
        state: &mut AIState,
        world: &mut GameWorld,
        missions: &mut MissionState,
        kind: MissionKind,
        member: MissionMember,
        system: SystemKey,
    ) -> u64 {
        let id = missions
            .dispatch_guarded(
                crate::missions::MissionRequest {
                    kind,
                    faction: crate::missions::MissionFaction::Alliance,
                    team: vec![member],
                    decoys: vec![],
                    target_system: system,
                    target_character: None,
                    target_object: None,
                    tick: 1,
                },
                world,
            )
            .unwrap();
        state.bind_mission(kind, member, id);
        id
    }

    fn aborted(actions: &[AIAction]) -> Vec<u64> {
        actions
            .iter()
            .filter_map(|a| match a {
                AIAction::AbortMission { mission_id } => Some(*mission_id),
                _ => None,
            })
            .collect()
    }

    fn result(id: u64, outcome: crate::missions::MissionOutcome) -> crate::missions::MissionResult {
        crate::missions::MissionResult {
            mission_id: id,
            tick: 9,
            kind: MissionKind::ShipDesignResearch,
            faction: crate::missions::MissionFaction::Alliance,
            character: None,
            target_system: SystemKey::default(),
            outcome,
            effects: vec![],
        }
    }

    #[test]
    fn each_tree_keeps_a_planner_for_every_character_able_to_research_it() {
        // FUN_004e8290 requests a research subtype while its live planners
        // (+0x324/+0x328/+0x32c) are fewer than the side's characters with
        // the tree's capability (FUN_0041bc10: +0x2cc, +0x2d0, +0x2c8); each
        // planner takes the best free member of skill 10 or more
        // (FUN_0047bcd0, FUN_0047c2b0, FUN_0047c900).
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let _bare = add_system(&mut world, sector, 0.5, 0.5);
        let shipyard = yard_system(&mut world, sector, 0x28);
        let training = yard_system(&mut world, sector, 0x29);
        let weak = designer(&mut world, TechType::Ship, 20);
        let ship = designer(&mut world, TechType::Ship, 60);
        let troop = designer(&mut world, TechType::Troop, 30);
        let _facility = designer(&mut world, TechType::Facility, 70);
        let mut state = AIState::new(AiFaction::Alliance);

        let actions = research_pass(
            &mut state,
            &world,
            &MissionState::new(),
            &ResearchState::new(),
            7,
        );

        // FUN_004bd570: each designer stands away from its site, so its
        // first order is the Move there.
        assert_eq!(
            orders(&actions, MissionKind::Move),
            vec![
                (vec![ship], shipyard, None),
                (vec![weak], shipyard, None),
                (vec![troop], training, None)
            ]
        );
        let techs: Vec<_> = state
            .research_planners
            .iter()
            .map(|p| (p.id, p.tech))
            .collect();
        assert_eq!(
            techs,
            vec![
                (0, TechType::Ship),
                (1, TechType::Ship),
                (2, TechType::Troop),
                (3, TechType::Facility)
            ]
        );
        assert_eq!(state.next_planner_id, 4);
    }

    #[test]
    fn a_designer_below_skill_10_is_never_sent() {
        // FUN_0047bcd0: FUN_004035d0(.., skill 2, minimum 10, ..). The
        // weaker designer still counts as able (FUN_0041bc10).
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let _ = yard_system(&mut world, sector, 0x28);
        let _ = designer(&mut world, TechType::Ship, 9);
        let actions = research_orders(&world, &MissionState::new(), &ResearchState::new());
        assert!(orders(&actions, MissionKind::Move).is_empty());

        let ten = designer(&mut world, TechType::Ship, 10);
        let actions = research_orders(&world, &MissionState::new(), &ResearchState::new());
        // FUN_004bd570: away from the site, the order is the Move there.
        let sent: Vec<_> = orders(&actions, MissionKind::Move)
            .into_iter()
            .map(|(team, _, _)| team)
            .collect();
        assert_eq!(sent, vec![vec![ten]]);
    }

    #[test]
    fn a_planner_holds_its_designer_while_he_researches() {
        // FUN_0047b610 marks the record held (+0x24, priority +0x2c = 0)
        // and FUN_004035d0 (threshold 0) skips it for every other planner;
        // FUN_0047ba40 keeps a member on a mission (0x80 is not in
        // 0x7400051). So a second designer, not the busy first, is sent.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let shipyard = yard_system(&mut world, sector, 0x28);
        let best = designer(&mut world, TechType::Ship, 60);
        let mut state = AIState::new(AiFaction::Alliance);
        let mut missions = MissionState::new();
        research_pass(&mut state, &world, &missions, &ResearchState::new(), 7);
        apply_order(
            &mut state,
            &mut world,
            &mut missions,
            MissionKind::ShipDesignResearch,
            best,
            shipyard,
        );
        let second = designer(&mut world, TechType::Ship, 40);

        let actions = research_pass(&mut state, &world, &missions, &ResearchState::new(), 14);

        // FUN_004bd570: the second stands away from the site.
        assert_eq!(
            orders(&actions, MissionKind::Move),
            vec![(vec![second], shipyard, None)]
        );
        assert_eq!(state.research_planners[0].member, best.character());
        assert_eq!(state.research_planners[1].member, second.character());
    }

    #[test]
    fn a_research_site_needs_the_side_s_garrison_and_no_uprising() {
        // FUN_00433e40: +0x2c bit 4 needs +0x90 > 0 (an active regiment)
        // and +0x28 & 3 clear (bit 1 an uprising, bit 2 a blockade).
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let system = add_system(&mut world, sector, 0.5, 0.5);
        world.systems[system].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let yard =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId::new(0x2800_0001),
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: true,
                });
        world.systems[system].manufacturing_facilities.push(yard);
        let research = ResearchState::new();
        let no_uprising = crate::uprising::UprisingState::default();
        let site = |world: &GameWorld, uprisings| {
            ai_research_site(world, uprisings, &research, true, TechType::Ship, system)
        };

        assert!(!site(&world, &no_uprising));
        garrison(&mut world, system, false);
        assert!(!site(&world, &no_uprising));
        garrison(&mut world, system, true);
        assert!(site(&world, &no_uprising));
        assert!(!ai_research_site(
            &world,
            &no_uprising,
            &research,
            true,
            TechType::Troop,
            system
        ));
        let mut revolt = crate::uprising::UprisingState::default();
        revolt.active_uprisings.insert(
            system,
            crate::uprising::ActiveUprising {
                started_tick: 0,
                next_incident_tick: None,
                next_support_tick: None,
                side: None,
            },
        );
        assert!(!site(&world, &revolt));
    }

    fn fresh_planner(tech: TechType) -> ResearchPlanner {
        ResearchPlanner {
            id: 0,
            tech,
            member: None,
            site: None,
            target: None,
            mission: None,
            report: None,
            failures: 0,
        }
    }

    fn choose(world: &GameWorld, p: &ResearchPlanner) -> Option<SystemKey> {
        let mut planner = Planner::new(world, &MissionState::new(), &HashSet::new(), true, 7);
        research_site_choice(
            &mut planner,
            world,
            &crate::uprising::UprisingState::default(),
            &ResearchState::new(),
            true,
            p,
        )
    }

    #[test]
    fn a_new_planner_takes_the_best_supported_site_of_the_sector_with_most_sites() {
        // FUN_0047be50 with no previous target: FUN_00419a00 keys the
        // sectors by their site count (+0x154, order 1), then FUN_00431830
        // keys the sector's sites by the side's support (system record
        // +0x48, FUN_00507270, order 1); FUN_004357b0 takes the last.
        let mut world = empty_world();
        let lone = add_sector(&mut world);
        let busy = add_sector(&mut world);
        let favourite = yard_system(&mut world, lone, 0x28);
        world.systems[favourite].popularity_alliance = 0.9;
        let middling = yard_system(&mut world, busy, 0x28);
        world.systems[middling].popularity_alliance = 0.6;
        let best = yard_system(&mut world, busy, 0x28);
        world.systems[best].popularity_alliance = 0.8;

        assert_eq!(choose(&world, &fresh_planner(TechType::Ship)), Some(best));
    }

    #[test]
    fn a_later_cycle_takes_the_site_nearest_its_previous_target() {
        // FUN_0047be50 with +0x38 a system: FUN_00419b80 keys the sectors by
        // the squared distance from its sector (order 2), then
        // FUN_00431c00 the sector's sites by the distance from it (order 2).
        let mut world = empty_world();
        let near = add_sector(&mut world);
        let far = add_sector(&mut world);
        world.sectors[far].x = 500;
        let crowded_a = yard_system(&mut world, far, 0x28);
        let _crowded_b = yard_system(&mut world, far, 0x28);
        world.systems[crowded_a].x = 500;
        let close = yard_system(&mut world, near, 0x28);
        world.systems[close].x = 40;
        let closer = yard_system(&mut world, near, 0x28);
        world.systems[closer].x = 12;
        let from = add_system(&mut world, near, 0.5, 0.5);
        world.systems[from].x = 10;
        let mut p = fresh_planner(TechType::Ship);
        p.target = Some(from);

        assert_eq!(choose(&world, &p), Some(closer));
        p.site = Some(close);
        assert_eq!(choose(&world, &p), Some(close));
    }

    #[test]
    fn the_nearest_site_is_measured_by_squared_map_distance() {
        // FUN_00431c00 keys a sector's sites by the squared difference of
        // the records' places (FUN_0041b7b0) from the previous target.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let place = |world: &mut GameWorld, x, y| {
            let system = yard_system(world, sector, 0x28);
            world.systems[system].x = x;
            world.systems[system].y = y;
            system
        };
        let east = place(&mut world, 120, 100);
        let _south = place(&mut world, 100, 70);
        let _west = place(&mut world, 10, 100);
        let from = add_system(&mut world, sector, 0.5, 0.5);
        world.systems[from].x = 100;
        world.systems[from].y = 100;
        let mut p = fresh_planner(TechType::Ship);
        p.target = Some(from);

        assert_eq!(choose(&world, &p), Some(east));
    }

    #[test]
    fn a_research_order_binds_to_the_planner_of_its_tree_and_designer() {
        // FUN_0047b360 stores the emitted order on the planner that built
        // it (+0x28); FUN_0042f7f0 then routes the mission's reports there.
        let mut world = empty_world();
        let a = add_character(&mut world, true, false, 0);
        let b = add_character(&mut world, true, false, 0);
        let mut state = AIState::new(AiFaction::Alliance);
        let mut troop = fresh_planner(TechType::Troop);
        troop.member = Some(a);
        let mut ship = fresh_planner(TechType::Ship);
        ship.member = Some(b);
        state.research_planners = vec![troop, ship];

        state.bind_mission(
            MissionKind::ShipDesignResearch,
            MissionMember::Character(a),
            7,
        );
        assert!(state.research_planners.iter().all(|p| p.mission.is_none()));
        state.bind_mission(MissionKind::Diplomacy, MissionMember::Character(b), 7);
        assert!(state.research_planners.iter().all(|p| p.mission.is_none()));
        state.bind_mission(
            MissionKind::ShipDesignResearch,
            MissionMember::Character(b),
            7,
        );
        assert_eq!(state.research_planners[1].mission, Some(7));
        assert_eq!(state.research_planners[0].mission, None);

        state.note_mission_results(&[result(7, crate::missions::MissionOutcome::Failure)]);
        assert_eq!(
            state.research_planners[1].report,
            Some(crate::missions::MissionOutcome::Failure)
        );
        assert_eq!(state.research_planners[0].report, None);
    }

    #[test]
    fn equal_keys_are_ordered_by_coin_flips_and_the_last_wins() {
        // FUN_005f59f0 inserts before the first entry FUN_0041c250 puts the
        // new one ahead of; equal keys go ahead on FUN_0041cd80(10) < 5.
        // FUN_004357b0 takes the last of the list (rank FUN_0041c230).
        let world = empty_world();
        let mut wins = [0; 2];
        for tick in 0..200 {
            let mut planner =
                Planner::new(&world, &MissionState::new(), &HashSet::new(), true, tick);
            assert_eq!(planner.pick_ranked([(0, 1), (1, 5), (2, 3)], true), Some(1));
            assert_eq!(
                planner.pick_ranked([(0, 1), (1, 5), (2, 3)], false),
                Some(0)
            );
        }
        // Two equal keys: the second goes ahead of the first on a draw
        // below 5 of 0..=9, so either wins half the time.
        for tick in 0..4000 {
            let mut planner =
                Planner::new(&world, &MissionState::new(), &HashSet::new(), true, tick);
            wins[planner.pick_ranked([(0, 4), (1, 4)], true).unwrap()] += 1;
        }
        assert!((1800..=2200).contains(&wins[0]), "{wins:?}");
    }

    #[test]
    fn the_ai_counts_a_tree_done_at_its_own_classes_highest_order() {
        // FUN_0042dd00: +0x140 is the highest research order of the side's
        // own ship classes (wrapper +0x28 1 or 2, FUN_00473290); +0x148
        // takes every facility class. FUN_004198a0 sets the Done bit when
        // the level reaches it.
        let mut world = empty_world();
        let class = |alliance, empire, order| crate::world::CapitalShipClass {
            is_alliance: alliance,
            is_empire: empire,
            research_order: order,
            ..Default::default()
        };
        world.capital_ship_classes.insert(class(true, false, 3));
        world.capital_ship_classes.insert(class(true, true, 7));
        world.capital_ship_classes.insert(class(false, true, 5));
        world.buildable_classes.insert(
            DatId::new(0x2200_0001),
            crate::world::BuildableClass {
                is_empire: true,
                research_order: 4,
                ..Default::default()
            },
        );
        assert_eq!(ai_research_goal(&world, true, TechType::Ship), 3);
        assert_eq!(ai_research_goal(&world, false, TechType::Ship), 5);
        assert_eq!(ai_research_goal(&world, true, TechType::Facility), 4);
        assert_eq!(ai_research_goal(&world, true, TechType::Troop), 0);

        let mut research = ResearchState::new();
        research.alliance.ship = 2;
        assert!(!ai_research_done(&world, &research, true, TechType::Ship));
        research.alliance.ship = 3;
        assert!(ai_research_done(&world, &research, true, TechType::Ship));
        assert!(!ai_research_done(&world, &research, true, TechType::Troop));
    }

    #[test]
    fn the_troop_goal_takes_only_the_side_s_own_regiment_classes() {
        // FUN_0042dd00: +0x144 walks families 0x10..0x13 and keeps a class
        // whose wrapper side (+0x28, FUN_00473290) is the AI's.
        let mut world = empty_world();
        let class = |alliance, empire, order| crate::world::BuildableClass {
            is_alliance: alliance,
            is_empire: empire,
            research_order: order,
            ..Default::default()
        };
        world
            .buildable_classes
            .insert(DatId::new(0x1000_0001), class(true, false, 2));
        world
            .buildable_classes
            .insert(DatId::new(0x1000_0002), class(false, true, 6));
        world
            .buildable_classes
            .insert(DatId::new(0x2200_0001), class(true, false, 4));
        assert_eq!(ai_research_goal(&world, true, TechType::Troop), 2);
        assert_eq!(ai_research_goal(&world, false, TechType::Troop), 6);
    }

    #[test]
    fn a_cancelled_planner_leaves_its_mission_running() {
        // FUN_004e5710 picks the planner with the least team skill
        // (FUN_0047bc30); FUN_0047aff0 releases its hold and removes it, but
        // issues no order on its mission.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let shipyard = yard_system(&mut world, sector, 0x28);
        let strong = designer(&mut world, TechType::Ship, 60);
        let weak = designer(&mut world, TechType::Ship, 30);
        let mut state = AIState::new(AiFaction::Alliance);
        let mut missions = MissionState::new();
        research_pass(&mut state, &world, &missions, &ResearchState::new(), 7);
        apply_order(
            &mut state,
            &mut world,
            &mut missions,
            MissionKind::ShipDesignResearch,
            strong,
            shipyard,
        );
        apply_order(
            &mut state,
            &mut world,
            &mut missions,
            MissionKind::ShipDesignResearch,
            weak,
            shipyard,
        );
        // The weaker designer can no longer research ships.
        world.characters[weak.character().unwrap()].ship_design = SkillPair {
            base: 0,
            variance: 0,
        };

        let actions = research_pass(&mut state, &world, &missions, &ResearchState::new(), 14);

        assert!(aborted(&actions).is_empty());
        let members: Vec<_> = state.research_planners.iter().map(|p| p.member).collect();
        assert_eq!(members, vec![strong.character()]);
        assert_eq!(missions.len(), 2);
    }

    #[test]
    fn a_done_tree_drops_its_planners_and_orders_none() {
        // FUN_004e8290: with the Done bit set the subtype takes no request
        // and its planners are cancelled; their missions run on.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let shipyard = yard_system(&mut world, sector, 0x28);
        world
            .capital_ship_classes
            .insert(crate::world::CapitalShipClass {
                is_alliance: true,
                research_order: 2,
                ..Default::default()
            });
        let running = designer(&mut world, TechType::Ship, 50);
        let mut state = AIState::new(AiFaction::Alliance);
        let mut missions = MissionState::new();
        research_pass(&mut state, &world, &missions, &ResearchState::new(), 7);
        apply_order(
            &mut state,
            &mut world,
            &mut missions,
            MissionKind::ShipDesignResearch,
            running,
            shipyard,
        );
        let mut research = ResearchState::new();
        research.alliance.ship = 2;

        let actions = research_pass(&mut state, &world, &missions, &research, 14);

        assert!(orders(&actions, MissionKind::ShipDesignResearch).is_empty());
        assert!(aborted(&actions).is_empty());
        assert!(state
            .research_planners
            .iter()
            .all(|p| p.tech != TechType::Ship));
    }

    #[test]
    fn a_reported_result_aborts_research_where_the_site_is_lost() {
        // FUN_0047c890: a result of 2 or 3 flags the planner; at its next
        // state 1 FUN_0047b250 asks FUN_0047c110 for the site bit and,
        // without it, orders 0x250 on the mission. No report, no check.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let shipyard = yard_system(&mut world, sector, 0x28);
        let ship = designer(&mut world, TechType::Ship, 50);
        let mut state = AIState::new(AiFaction::Alliance);
        let mut missions = MissionState::new();
        research_pass(&mut state, &world, &missions, &ResearchState::new(), 7);
        let id = apply_order(
            &mut state,
            &mut world,
            &mut missions,
            MissionKind::ShipDesignResearch,
            ship,
            shipyard,
        );
        world.systems[shipyard].ground_units.clear();

        let quiet = research_pass(&mut state, &world, &missions, &ResearchState::new(), 14);
        assert!(aborted(&quiet).is_empty());

        state.note_mission_results(&[result(id, crate::missions::MissionOutcome::Success)]);
        let actions = research_pass(&mut state, &world, &missions, &ResearchState::new(), 21);
        assert_eq!(aborted(&actions), vec![id]);
        assert!(state.research_planners[0].report.is_none());
    }

    #[test]
    fn a_planner_aborts_after_more_than_a_thousand_failures() {
        // FUN_0047ae20 counts failed results (+0x6c) against slot +0x24
        // (FUN_00480df0, 1000); past it the planner sets 0x20000000 and
        // FUN_0047b250 orders 0x250. A success clears the count.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let shipyard = yard_system(&mut world, sector, 0x28);
        let ship = designer(&mut world, TechType::Ship, 50);
        let mut state = AIState::new(AiFaction::Alliance);
        let mut missions = MissionState::new();
        research_pass(&mut state, &world, &missions, &ResearchState::new(), 7);
        let id = apply_order(
            &mut state,
            &mut world,
            &mut missions,
            MissionKind::ShipDesignResearch,
            ship,
            shipyard,
        );
        state.research_planners[0].failures = 1000;

        state.note_mission_results(&[result(id, crate::missions::MissionOutcome::Success)]);
        assert!(aborted(&research_pass(
            &mut state,
            &world,
            &missions,
            &ResearchState::new(),
            14
        ))
        .is_empty());
        assert_eq!(state.research_planners[0].failures, 0);

        state.research_planners[0].failures = 999;
        state.note_mission_results(&[result(id, crate::missions::MissionOutcome::Failure)]);
        assert!(aborted(&research_pass(
            &mut state,
            &world,
            &missions,
            &ResearchState::new(),
            21
        ))
        .is_empty());
        state.note_mission_results(&[result(id, crate::missions::MissionOutcome::Failure)]);
        assert_eq!(
            aborted(&research_pass(
                &mut state,
                &world,
                &missions,
                &ResearchState::new(),
                28
            )),
            vec![id]
        );
        assert_eq!(state.research_planners[0].failures, 0);
    }

    #[test]
    fn the_research_order_goes_to_the_site_from_wherever_the_team_gathered() {
        // FUN_0047b360 orders on +0x30, the site the slot copied from +0x34
        // (FUN_0047be50 at 0x47c0cd); +0x38 (FUN_004bd0a0) is only where
        // the designer gathers. A designer gathered at a system with no
        // shipyard is sent to research at the shipyard.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let shipyard = yard_system(&mut world, sector, 0x28);
        let old = add_system(&mut world, sector, 0.5, 0.5);
        world.systems[old].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let ship = designer(&mut world, TechType::Ship, 50);
        let MissionMember::Character(key) = ship else {
            unreachable!()
        };
        world.characters[key].current_system = Some(old);
        let mut state = AIState::new(AiFaction::Alliance);
        research_pass(
            &mut state,
            &world,
            &MissionState::new(),
            &ResearchState::new(),
            7,
        );
        state.research_planners[0].target = Some(old);

        let actions = research_pass(
            &mut state,
            &world,
            &MissionState::new(),
            &ResearchState::new(),
            14,
        );
        assert_eq!(
            orders(&actions, MissionKind::ShipDesignResearch),
            vec![(vec![ship], shipyard, None)]
        );
        assert!(orders(&actions, MissionKind::Move).is_empty());

        // With no site at all, the cycle ends before any order.
        world.systems[shipyard].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let actions = research_pass(
            &mut state,
            &world,
            &MissionState::new(),
            &ResearchState::new(),
            21,
        );
        assert!(orders(&actions, MissionKind::ShipDesignResearch).is_empty());
        assert!(orders(&actions, MissionKind::Move).is_empty());
    }

    #[test]
    fn the_previous_target_stays_while_it_is_the_side_s_and_unblockaded() {
        // FUN_004bd0a0 keeps +0x38 when its record has +0x30 bit 0 and not
        // +0x28 bit 0x2, before it looks at the site +0x34.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let shipyard = yard_system(&mut world, sector, 0x28);
        let old = add_system(&mut world, sector, 0.5, 0.5);
        world.systems[old].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let ship = designer(&mut world, TechType::Ship, 50);
        let mut state = AIState::new(AiFaction::Alliance);
        research_pass(
            &mut state,
            &world,
            &MissionState::new(),
            &ResearchState::new(),
            7,
        );
        state.research_planners[0].target = Some(old);

        let actions = research_pass(
            &mut state,
            &world,
            &MissionState::new(),
            &ResearchState::new(),
            14,
        );
        // FUN_004bd570: the designer stands at neither, so it moves first.
        assert_eq!(
            orders(&actions, MissionKind::Move),
            vec![(vec![ship], old, None)]
        );

        world.systems[old].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let actions = research_pass(
            &mut state,
            &world,
            &MissionState::new(),
            &ResearchState::new(),
            21,
        );
        assert_eq!(
            orders(&actions, MissionKind::Move),
            vec![(vec![ship], shipyard, None)]
        );
    }

    #[test]
    fn a_designer_away_from_the_site_is_moved_there_before_its_research() {
        // FUN_004bd570 (planner state 7): a free member whose location
        // (+0x38) is not the target gets a Move order 0x201 to it (the Move
        // pseudo-mission 0x241, MISSNSD 0x41000001); state 8
        // (FUN_0047b360) orders the research once it stands there.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let shipyard = yard_system(&mut world, sector, 0x28);
        let elsewhere = add_system(&mut world, sector, 0.5, 0.5);
        let ship = designer(&mut world, TechType::Ship, 50);
        let key = ship.character().unwrap();
        world.characters[key].current_system = Some(elsewhere);
        let mut state = AIState::new(AiFaction::Alliance);

        let actions = research_pass(
            &mut state,
            &world,
            &MissionState::new(),
            &ResearchState::new(),
            7,
        );
        assert_eq!(
            orders(&actions, MissionKind::Move),
            vec![(vec![ship], shipyard, None)]
        );
        assert!(orders(&actions, MissionKind::ShipDesignResearch).is_empty());

        world.characters[key].current_system = Some(shipyard);
        let actions = research_pass(
            &mut state,
            &world,
            &MissionState::new(),
            &ResearchState::new(),
            14,
        );
        assert!(orders(&actions, MissionKind::Move).is_empty());
        assert_eq!(
            orders(&actions, MissionKind::ShipDesignResearch),
            vec![(vec![ship], shipyard, None)]
        );
    }

    /// An Alliance shipyard site with a designer away from it, and
    /// `fleets` Alliance fleets there facing an Empire system: the research
    /// module (0) orders a Move, the fleet module (8) attacks.
    fn cadence_world(fleets: usize) -> (GameWorld, MissionMember) {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let shipyard = yard_system(&mut world, sector, 0x28);
        let target = add_system(&mut world, sector, 0.15, 0.6);
        world.systems[target].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let class_key = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        for _ in 0..fleets {
            let _ = world.fleets.insert(Fleet {
                location: shipyard,
                capital_ships: ShipInstance::make(class_key, 100, true, 1),
                fighters: vec![],
                characters: vec![],
                is_alliance: true,
                has_death_star: false,
            });
        }
        let ship = designer(&mut world, TechType::Ship, 50);
        (world, ship)
    }

    fn cadence_pass(state: &mut AIState, world: &GameWorld) -> Vec<AIAction> {
        AISystem::pass(
            state,
            world,
            &ManufacturingState::new(),
            &MissionState::new(),
            &crate::movement::MovementState::new(),
            &crate::economy::EconomyState::default(),
            &crate::troop_transport::TroopTransportState::new(),
            7,
            &GameConfig::default(),
            &ResearchState::new(),
            &crate::uprising::UprisingState::default(),
        )
    }

    #[test]
    fn the_strategy_module_chooses_a_posture_and_plans_a_sector_on_the_next_pass() {
        // Module 0xd (FUN_004bf390): the posture (FUN_004bf490) builds a
        // strategy, which replans (controller +4 bit 2) with no plan to
        // keep; the review (FUN_004e5560) grows the empty list, and the
        // next pass names one sector (FUN_004e62b0's third query: +0x2c 6,
        // kind 3). A blockaded system not the side's is seen that day
        // (FUN_00433e40, record +0x44).
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let own = add_system(&mut world, sector, 0.9, 0.1);
        world.systems[own].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let held = add_system(&mut world, sector, 0.1, 0.9);
        world.systems[held].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let blockaded = add_system(&mut world, sector, 0.1, 0.9);
        world.systems[blockaded].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let fleet = world.fleets.insert(Fleet {
            location: blockaded,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        world.systems[blockaded].fleets.push(fleet);
        // An own system under an Empire fleet is blockaded too, but the
        // side's own (+0x30 & 6 clear).
        let enemy = world.fleets.insert(Fleet {
            location: own,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[own].fleets.push(enemy);
        let mut state = AIState::new(AiFaction::Alliance);
        cadence_pass(&mut state, &world);
        assert_ne!(state.strategy.posture, 0);
        assert_eq!(state.strategy.kind, state.strategy.posture);
        assert!(state.strategy.plans.is_empty());
        assert!(state.strategy.grow);
        assert_eq!(state.seen, HashMap::from([(blockaded, 7)]));
        cadence_pass(&mut state, &world);
        assert_eq!(
            state.strategy.plans,
            vec![crate::ai_strategy::SectorPlan {
                sector,
                kind: 3,
                param: 0
            }]
        );
        // The third pass's refresh counts the plan short of a planner
        // (+0x204 = -1): module 8 adds one, which claims the sector and
        // binds kind 3 (FUN_004cf700, FUN_004cf7c0, FUN_004773f0).
        assert!(state.module8.planners.is_empty());
        cadence_pass(&mut state, &world);
        let planners: Vec<_> = state
            .module8
            .planners
            .iter()
            .map(|p| (p.id, p.sector, p.plan_id, p.kind))
            .collect();
        assert_eq!(planners, vec![(1, Some(sector), 1, 3)]);
        // Its first cycle ran (FUN_004774c0 states 1 to 6, back to 1),
        // reporting through kind 3's slot +0x20 (0x4000000).
        let p = &state.module8.planners[0];
        assert_eq!(p.state, 1);
        assert_ne!(p.flags & crate::ai_module8::CYCLED, 0);
    }

    /// A sector planner already holding the world's sector, planned kind
    /// 3, so module 8 reaches state 4 in its first cycle (`FUN_004cf630`
    /// states 1 to 3: no planner to add, it keeps its sector, binds).
    fn with_sector_planner(state: &mut AIState, world: &GameWorld) {
        let sector = world.sectors.keys().next().unwrap();
        state.strategy.plans = vec![crate::ai_strategy::SectorPlan {
            sector,
            kind: 3,
            param: 0,
        }];
        state.module8.planners = vec![crate::ai_module8::SectorPlanner {
            id: 1,
            sector: Some(sector),
            plan_id: 1,
            kind: 3,
            ..crate::ai_module8::SectorPlanner::default()
        }];
        state.module8.next_id = 2;
    }

    fn kinds(actions: &[AIAction]) -> Vec<&'static str> {
        actions
            .iter()
            .map(|a| match a {
                AIAction::DispatchMission {
                    kind: MissionKind::Move,
                    ..
                } => "move",
                AIAction::MoveFleet { .. } => "fleet",
                _ => "other",
            })
            .collect()
    }

    /// Module indices in [`MODULE_IDS`]: the strategy, the missions, the
    /// sector fleet planners, the troop module `0xc`.
    const STRATEGY: usize = 0;
    const MISSIONS: usize = 6;
    const FLEETS: usize = 8;
    const TROOPS: usize = 12;

    /// An order a module keeps in its queue (port: module `0xc`'s
    /// heuristic decides several at once and issues one a step).
    fn queued(id: u64) -> AIAction {
        AIAction::AbortMission { mission_id: id }
    }

    #[test]
    fn each_production_order_belongs_to_one_module() {
        let facility = FacilityBuild {
            class: DatId::new(0x2200_0001),
            is_alliance: true,
        };
        // Module 2 builds only 0x2a, 0x2c and 0x2d (FUN_004e2400,
        // FUN_004e25c0); the heuristic's facilities stand in for 0xa.
        assert_eq!(
            production_module(&BuildableKind::ManufacturingFacility(facility)),
            Some(0xa)
        );
        // Module 3 builds the defenses itself (FUN_004d8460).
        assert_eq!(
            production_module(&BuildableKind::DefenseFacility(facility)),
            None
        );
        // Module 6 trains the regiments itself (FUN_004df9a0).
        assert_eq!(
            production_module(&BuildableKind::Troop(DatId::new(0x1000_0001))),
            None
        );
        // Modules 7 and 0xb build the squadrons and capital ships
        // themselves (FUN_004d46e0, FUN_004ca910).
        assert_eq!(
            production_module(&BuildableKind::CapitalShip(CapitalShipKey::default())),
            None
        );
        assert_eq!(
            production_module(&BuildableKind::Fighter(FighterKey::default())),
            None
        );
        assert_eq!(
            production_module(&BuildableKind::SpecialForce(DatId::new(0x3c00_0001))),
            None
        );
    }

    #[test]
    fn a_production_module_issues_only_its_own_kind() {
        // The world of idle_system_with_mfg_gets_fighter_production: the
        // heuristic decides fighters there, which no module issues now that
        // modules 7 and 0xb build squadrons and ships (FUN_004d46e0,
        // FUN_004ca910); modules 2, 3, 5, 6, 0xa and 0xb issue nothing.
        let (world, _) = fighter_world();
        let (missions, research, mfg, config, uprisings) = (
            MissionState::new(),
            ResearchState::new(),
            ManufacturingState::new(),
            GameConfig::default(),
            crate::uprising::UprisingState::default(),
        );
        let ctx = ModuleInputs {
            faction: AiFaction::Empire,
            ..inputs(&world, &missions, &research, &mfg, &config, &uprisings)
        };
        let busy = HashSet::new();
        let mut planner = Planner::new(&world, &missions, &busy, false, 7);
        let mut state = AIState::new(AiFaction::Empire);
        let mut run = |id| {
            let mut actions = Vec::new();
            AISystem::run_module(
                id,
                &mut state,
                &mut planner,
                &ctx,
                &mut crate::ai_records::Galaxy::default(),
                &mut actions,
            );
            actions
        };

        let mut decided = Vec::new();
        AISystem::evaluate_production(
            &world,
            &mfg,
            &research,
            AiFaction::Empire,
            &config,
            &mut decided,
        );
        assert!(decided.iter().any(|a| matches!(
            a,
            AIAction::EnqueueProduction {
                kind: BuildableKind::Fighter(_),
                ..
            }
        )));
        for id in [2, 3, 5, 6, 0xa, 0xb] {
            assert!(run(id).is_empty(), "module {id}");
        }
    }

    #[test]
    fn the_modules_run_in_the_order_the_ai_player_adds_them() {
        // FUN_00485a50: FUN_004be930 with 0xd, 2, 1, 3, 6, 7, 4, 5, 8, 9,
        // 0xa, 0xb, 0xc, 0xe.
        assert_eq!(
            MODULE_IDS,
            [0xd, 2, 1, 3, 6, 7, 4, 5, 8, 9, 0xa, 0xb, 0xc, 0xe]
        );
        assert_eq!(MODULE_IDS[STRATEGY], 0xd);
        assert_eq!(MODULE_IDS[MISSIONS], 4);
        assert_eq!(MODULE_IDS[FLEETS], 8);
    }

    #[test]
    fn a_pass_issues_one_order_and_the_next_goes_on_after_its_module() {
        // FUN_00485b40 phase 3: a step that completes its module with an
        // order ends the tick (phase 4, return 1); FUN_004be980 then takes
        // the next ready module after the one last run (+0x10), wrapping.
        let (world, ship) = cadence_world(1);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_pending[TROOPS] = vec![queued(1)];

        assert_eq!(kinds(&cadence_pass(&mut state, &world)), vec!["move"]);
        assert_eq!(state.module_cursor, Some(MISSIONS));
        // The caller applies the Move: the designer is busy.
        state.mark_busy(ship.character().unwrap());
        assert_eq!(cadence_pass(&mut state, &world), vec![queued(1)]);
        assert_eq!(state.module_cursor, Some(TROOPS));
        // Round from module 0xe to 0xc again: the rest order nothing.
        state.module_pending[TROOPS] = vec![queued(2)];
        assert_eq!(cadence_pass(&mut state, &world), vec![queued(2)]);
        assert_eq!(state.module_cursor, Some(TROOPS));
        assert_eq!(state.orders_today, 3);
    }

    #[test]
    fn a_module_with_orders_left_raises_its_priority_and_runs_first() {
        // FUN_004d2490 state 6 and the build modules raise +0x34 after an
        // order while their cycle goes on; FUN_004bede0 takes the ready
        // module with the highest +0x34 before the round robin, and
        // FUN_00485b40 zeroes it before the step.
        let (world, _) = cadence_world(1);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_pending[TROOPS] = vec![queued(1), queued(2)];
        assert_eq!(kinds(&cadence_pass(&mut state, &world)), vec!["move"]);

        assert_eq!(cadence_pass(&mut state, &world), vec![queued(1)]);
        assert_eq!(state.module_priority[TROOPS], 1);
        assert_eq!(state.module_pending[TROOPS].len(), 1);
        // The round robin would go on from module 0xe and come round to the
        // mission module first, whose designer is still free; the priority
        // sends the queued order.
        assert_eq!(cadence_pass(&mut state, &world), vec![queued(2)]);
        assert_eq!(state.module_priority[TROOPS], 0);
        assert!(state.module_pending[TROOPS].is_empty());
        assert_eq!(state.module_cursor, Some(TROOPS));
    }

    #[test]
    fn equal_priorities_go_to_the_first_from_a_scan_that_moves_on_each_call() {
        // FUN_004bede0 steps +0x14 on one module per call and keeps the
        // first strictly higher +0x34 from there.
        let world = empty_world();
        let order = queued;
        let mut state = AIState::new(AiFaction::Alliance);
        for (module, id) in [(MISSIONS, 1), (TROOPS, 2)] {
            state.module_priority[module] = 1;
            state.module_pending[module] = vec![order(id)];
        }

        assert_eq!(cadence_pass(&mut state, &world), vec![order(1)]);
        assert_eq!(state.priority_cursor, Some(0));
        assert_eq!(cadence_pass(&mut state, &world), vec![order(2)]);

        let mut state = AIState::new(AiFaction::Alliance);
        for (module, id) in [(MISSIONS, 1), (TROOPS, 2)] {
            state.module_priority[module] = 1;
            state.module_pending[module] = vec![order(id)];
        }
        state.priority_cursor = Some(MISSIONS);
        assert_eq!(cadence_pass(&mut state, &world), vec![order(2)]);
        assert_eq!(state.priority_cursor, Some(MISSIONS + 1));
        // The round robin did not move.
        assert_eq!(state.module_cursor, None);
    }

    #[test]
    fn a_module_that_has_not_completed_steps_again_in_the_same_pass() {
        // FUN_00485b40: a module that leaves +0x20 clear stays current, so
        // module 1 runs its site search and cycle states 1 to 8 within one
        // tick; the Destination it gives (FUN_004dc9f0) passes the
        // validator (a construction yard of the side) and ends the tick.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let own = |world: &mut GameWorld| {
            let sys = add_system(world, sector, 0.9, 0.1);
            world.systems[sys].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
            world.systems[sys].total_energy = 10;
            world.systems[sys].raw_materials = 5;
            sys
        };
        let site = own(&mut world);
        let empty = own(&mut world);
        let class = |world: &mut GameWorld, raw: u32| {
            let id = DatId(raw);
            world.buildable_classes.insert(
                id,
                crate::world::BuildableClass {
                    is_alliance: true,
                    is_empire: true,
                    maintenance_cost: 10,
                    refined_material_cost: 10,
                    ..Default::default()
                },
            );
            id
        };
        let yard = class(&mut world, 0x2a00_0003);
        let key =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: yard,
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: false,
                });
        world.systems[site].manufacturing_facilities.push(key);
        for (raw, is_mine) in [(0x2c00_0001, true), (0x2d00_0002, false)] {
            let class_dat_id = class(&mut world, raw);
            let key =
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id,
                        side: crate::dat::Faction::Alliance,
                        is_mine,
                    });
            world.systems[site].production_facilities.push(key);
        }
        let mut state = AIState::new(AiFaction::Alliance);
        // Module 1 (index 2) first, by priority (FUN_004bede0).
        state.module_priority[2] = 1;

        let orders = cadence_pass(&mut state, &world);

        assert_eq!(
            orders,
            vec![AIAction::SetDestination {
                system: site,
                area: ProductionArea::ConstructionYard,
                destination: empty,
            }]
        );
        assert_eq!(state.module1.state, 9);
        assert_eq!(state.orders_today, 1);
        // The Destination step raised module 1's priority (+0x34).
        assert_eq!(state.module_priority[2], 1);
    }

    #[test]
    fn the_validator_checks_module_one_s_orders() {
        // Slot +0x1c: every Scrap member's FUN_004f9860; Destination and
        // Build on the side's construction manager (hyp: a yard of the
        // side); Build of a listed class (FUN_0052e580) with a count.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let site = add_system(&mut world, sector, 0.9, 0.1);
        let elsewhere = add_system(&mut world, sector, 0.9, 0.1);
        let yard = DatId(0x2a00_0003);
        world.buildable_classes.insert(
            yard,
            crate::world::BuildableClass {
                is_alliance: true,
                is_empire: true,
                ..Default::default()
            },
        );
        let key =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: yard,
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: false,
                });
        world.systems[site].manufacturing_facilities.push(key);
        let research = ResearchState::new();
        let ok = |order: &AIAction| {
            accepts(
                order,
                &world,
                &research,
                AiFaction::Alliance,
                &crate::troop_transport::TroopTransportState::new(),
                &crate::movement::MovementState::new(),
            )
        };
        let own =
            crate::scrap::ScrapTarget::Facility(crate::world::FacilityRef::Manufacturing(key));
        assert!(ok(&AIAction::Scrap { team: vec![own] }));
        assert!(!ok(&AIAction::Scrap { team: vec![] }));
        assert!(!accepts(
            &AIAction::Scrap { team: vec![own] },
            &world,
            &research,
            AiFaction::Empire,
            &crate::troop_transport::TroopTransportState::new(),
            &crate::movement::MovementState::new()
        ));
        let destination = |system, destination| AIAction::SetDestination {
            system,
            area: ProductionArea::ConstructionYard,
            destination,
        };
        assert!(ok(&destination(site, elsewhere)));
        assert!(!ok(&destination(elsewhere, site)));
        let mut gone = world.clone();
        gone.systems.remove(elsewhere);
        assert!(!accepts(
            &destination(site, elsewhere),
            &gone,
            &research,
            AiFaction::Alliance,
            &crate::troop_transport::TroopTransportState::new(),
            &crate::movement::MovementState::new()
        ));
        let build = |system, class: u32, count| AIAction::Build {
            system,
            kind: BuildableKind::ManufacturingFacility(FacilityBuild {
                class: DatId(class),
                is_alliance: true,
            }),
            cost: 10,
            count,
        };
        assert!(ok(&build(site, 0x2a00_0003, 1)));
        assert!(!ok(&build(site, 0x2a00_0003, 0)));
        assert!(!ok(&build(elsewhere, 0x2a00_0003, 1)));
        assert!(!ok(&build(site, 0x2a00_0009, 1)));
        // Other orders are checked when applied.
        assert!(ok(&AIAction::AbortMission { mission_id: 1 }));
    }

    #[test]
    fn the_side_s_maintenance_leaves_out_suspended_systems() {
        // FUN_004182a0 (+0x184 = side +0x58): a mine and a refinery give
        // 50 (FUN_005323c0) unless their system is in an uprising or
        // blockaded (FUN_0050bbb0 suspends them).
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let site = add_system(&mut world, sector, 0.9, 0.1);
        world.systems[site].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        for (raw, is_mine) in [(0x2c00_0001, true), (0x2d00_0002, false)] {
            let key =
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId(raw),
                        side: crate::dat::Faction::Alliance,
                        is_mine,
                    });
            world.systems[site].production_facilities.push(key);
        }
        let galaxy = crate::ai_records::Galaxy::default();
        let mfg = ManufacturingState::new();
        let mut uprisings = crate::uprising::UprisingState::default();
        let capacity = |world: &GameWorld, uprisings: &crate::uprising::UprisingState| {
            side_tallies(world, &mfg, uprisings, AiFaction::Alliance, &galaxy).capacity
        };
        assert_eq!(capacity(&world, &uprisings), 50);
        uprisings.active_uprisings.insert(
            site,
            crate::uprising::ActiveUprising {
                started_tick: 0,
                next_incident_tick: None,
                next_support_tick: None,
                side: None,
            },
        );
        assert_eq!(capacity(&world, &uprisings), 0);
        uprisings.active_uprisings.clear();
        let raider = world.fleets.insert(crate::world::Fleet {
            location: site,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[site].fleets.push(raider);
        assert_eq!(capacity(&world, &uprisings), 0);
    }

    #[test]
    fn a_refused_module_one_order_ends_its_cycle_and_the_tick() {
        // FUN_00485b40: a refused order (slot +0x1c) goes to slot +0x18,
        // FUN_004dd810 (complete, +0x38 = +0x3c = +0x34 = 0), and since
        // the step gave an order the tick ends.
        // State 8's Destination on a manager the side lacks: refused.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let site = add_system(&mut world, sector, 0.9, 0.1);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module1.phase = crate::ai_module1::CYCLE;
        state.module1.state = 8;
        state.module1.target = crate::ai_module1::Target::System(site);
        state.module1.yard = crate::ai_module1::Yard::Manager(site);
        state.module_priority[2] = 5;

        let orders = cadence_pass(&mut state, &world);

        assert!(orders.is_empty());
        assert_eq!((state.module1.phase, state.module1.state), (0, 0));
        assert_eq!(state.module_priority[2], 0);
        assert_eq!(state.orders_today, 0);
        // Only module 1 was stepped: the round robin did not move.
        assert_eq!(state.module_cursor, None);
    }

    #[test]
    fn a_refused_module_two_order_ends_its_cycle_and_the_tick() {
        // FUN_00485b40 sends module 2's refused order to its slot +0x18,
        // FUN_004dd810, as for module 1. Module 2's yard cycle state 8
        // (FUN_004e4300): Destination on a manager the side lacks.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let site = add_system(&mut world, sector, 0.9, 0.1);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module2.phase = crate::ai_module2::YARD_CYCLE;
        state.module2.state = 8;
        state.module2.target = crate::ai_module1::Target::System(site);
        state.module2.yard = crate::ai_module1::Yard::Manager(site);
        state.module_priority[1] = 5;

        let orders = cadence_pass(&mut state, &world);

        assert!(orders.is_empty());
        assert_eq!((state.module2.phase, state.module2.state), (0, 0));
        assert_eq!(state.module_priority[1], 0);
        assert_eq!(state.orders_today, 0);
        assert_eq!(state.module_cursor, None);
    }

    #[test]
    fn a_refused_module_three_order_ends_its_cycle_and_the_tick() {
        // FUN_00485b40 sends module 3's refused order to its slot +0x18,
        // FUN_004dd810. Module 3's yard cycle state 0xb (FUN_004d9e70):
        // Destination on a manager the side lacks.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let site = add_system(&mut world, sector, 0.9, 0.1);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.phase = crate::ai_module3::YARD_CYCLE;
        state.module3.state = 0xb;
        state.module3.target = crate::ai_module1::Target::System(site);
        state.module3.yard = crate::ai_module1::Yard::Manager(site);
        state.module_priority[3] = 5;

        let orders = cadence_pass(&mut state, &world);

        assert!(orders.is_empty());
        assert_eq!((state.module3.phase, state.module3.state), (0, 0));
        assert_eq!(state.module_priority[3], 0);
        assert_eq!(state.orders_today, 0);
        assert_eq!(state.module_cursor, None);
    }

    #[test]
    fn a_refused_module_five_order_ends_its_cycle_and_the_tick() {
        // FUN_00485b40 sends module 5's refused order to its slot +0x18,
        // FUN_004dd810. Module 5's yard cycle state 10 (FUN_004d1710):
        // Destination on a manager the side lacks.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let site = add_system(&mut world, sector, 0.9, 0.1);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.phase = crate::ai_module5::YARD_CYCLE;
        state.module5.state = 10;
        state.module5.target = crate::ai_module1::Target::System(site);
        state.module5.yard = crate::ai_module1::Yard::Manager(site);
        state.module_priority[7] = 5;

        let orders = cadence_pass(&mut state, &world);

        assert!(orders.is_empty());
        assert_eq!((state.module5.phase, state.module5.state), (0, 0));
        assert_eq!(state.module_priority[7], 0);
        assert_eq!(state.orders_today, 0);
        assert_eq!(state.module_cursor, None);
    }

    #[test]
    fn a_special_force_build_needs_the_side_s_training_facility() {
        // Slot +0x1c of Build 0x212 (FUN_004d15f0): the troop manager
        // (FUN_00509670(system, 2); hyp: a training facility of the side)
        // listing the class.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let site = add_system(&mut world, sector, 0.9, 0.1);
        let spy = DatId(0x3c00_0001);
        for class in [spy, DatId(0x2900_0001), DatId(0x2a00_0003)] {
            world.buildable_classes.insert(
                class,
                crate::world::BuildableClass {
                    is_alliance: true,
                    is_empire: true,
                    ..Default::default()
                },
            );
        }
        let research = ResearchState::new();
        let build = AIAction::Build {
            system: site,
            kind: BuildableKind::SpecialForce(spy),
            cost: 1,
            count: 1,
        };
        let add = |world: &mut GameWorld, class: u32| {
            let key = world.manufacturing_facilities.insert(
                crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId(class),
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: false,
                },
            );
            world.systems[site].manufacturing_facilities.push(key);
        };
        add(&mut world, 0x2a00_0003);
        assert!(!accepts(
            &build,
            &world,
            &research,
            AiFaction::Alliance,
            &crate::troop_transport::TroopTransportState::new(),
            &crate::movement::MovementState::new()
        ));
        add(&mut world, 0x2900_0001);
        assert!(accepts(
            &build,
            &world,
            &research,
            AiFaction::Alliance,
            &crate::troop_transport::TroopTransportState::new(),
            &crate::movement::MovementState::new()
        ));
    }

    #[test]
    fn the_special_force_upkeep_counts_the_side_s_own_units() {
        // FUN_0041bc10: controller +0x2c4 += record +0x64 for the side's
        // own records; FUN_00402230 sets +0x64 for a non-character to its
        // class maintenance (FUN_004f2990).
        let mut world = empty_world();
        for (raw, upkeep) in [(0x3c00_0001, 2), (0x3c00_0002, 5)] {
            world.buildable_classes.insert(
                DatId(raw),
                crate::world::BuildableClass {
                    maintenance_cost: upkeep,
                    ..Default::default()
                },
            );
        }
        for (raw, is_alliance) in [
            (0x3c00_0001, true),
            (0x3c00_0002, true),
            (0x3c00_0002, false),
        ] {
            world.special_forces.insert(crate::world::SpecialForceUnit {
                class_dat_id: DatId(raw),
                is_alliance,
                skills: [0; 8],
                on_mission: false,
            });
        }
        assert_eq!(special_upkeep(&world, AiFaction::Alliance), 7);
        assert_eq!(special_upkeep(&world, AiFaction::Empire), 5);
    }

    #[test]
    fn a_refused_module_six_order_ends_its_cycle_and_the_tick() {
        // FUN_00485b40 sends module 6's refused order to its slot +0x18,
        // FUN_004dd810. Module 6's regiment cycle state 0xc (FUN_004e0a30):
        // Destination on a troop manager the side lacks.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let site = add_system(&mut world, sector, 0.9, 0.1);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.phase = crate::ai_module6::REGIMENT_CYCLE;
        state.module6.state = 0xc;
        state.module6.destination = crate::ai_module1::Target::System(site);
        state.module6.yard = crate::ai_module1::Yard::Manager(site);
        state.module_priority[4] = 5;

        let orders = cadence_pass(&mut state, &world);

        assert!(orders.is_empty());
        assert_eq!((state.module6.phase, state.module6.state), (0, 0));
        assert_eq!(state.module_priority[4], 0);
        assert_eq!(state.orders_today, 0);
        assert_eq!(state.module_cursor, None);
    }

    #[test]
    fn a_refused_module_seven_order_ends_its_cycle_and_the_tick() {
        // FUN_00485b40 sends module 7's refused order to its slot +0x18.
        // Module 7's squadron cycle state 0xb (FUN_004d5dd0): Destination
        // on a shipyard manager the side lacks.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let site = add_system(&mut world, sector, 0.9, 0.1);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module7.phase = crate::ai_module7::SQUADRON_CYCLE;
        state.module7.state = 0xb;
        state.module7.destination = Some(crate::squadrons::SquadronPlace::Planet(site));
        state.module7.yard = crate::ai_module1::Yard::Manager(site);
        state.module_priority[5] = 5;

        let orders = cadence_pass(&mut state, &world);

        assert!(orders.is_empty());
        assert_eq!((state.module7.phase, state.module7.state), (0, 0));
        assert_eq!(state.module_priority[5], 0);
        assert_eq!(state.orders_today, 0);
        assert_eq!(state.module_cursor, None);
    }

    #[test]
    fn a_refused_module_b_order_ends_its_cycle_and_the_tick() {
        // FUN_00485b40 sends module 0xb's refused order to its slot +0x18.
        // Its yard cycle state 0xe (FUN_004cbbb0): Destination on a
        // construction manager the side lacks.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let site = add_system(&mut world, sector, 0.9, 0.1);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_b.phase = crate::ai_module_b::YARD_CYCLE;
        state.module_b.state = 0xe;
        state.module_b.target = crate::ai_module1::Target::System(site);
        state.module_b.yard = crate::ai_module1::Yard::Manager(site);
        state.module_priority[11] = 5;

        let orders = cadence_pass(&mut state, &world);

        assert!(orders.is_empty());
        assert_eq!((state.module_b.phase, state.module_b.state), (0, 0));
        assert_eq!(state.module_priority[11], 0);
        assert_eq!(state.orders_today, 0);
    }

    #[test]
    fn create_fleet_is_accepted_for_living_ships_of_an_own_fleet_at_rest() {
        // Create Fleet's validation (fleet_join::check_create_fleet): the
        // side's fleet, out of hyperspace, each named ship living.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let here = add_system(&mut world, sector, 0.5, 0.5);
        let class = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        let fleet = world.fleets.insert(Fleet {
            location: here,
            capital_ships: ShipInstance::make(class, 100, true, 2),
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        let far = add_system(&mut world, sector, 0.5, 0.5);
        world.systems[far].x = 40;
        let transport = crate::troop_transport::TroopTransportState::new();
        let mut movement = crate::movement::MovementState::new();
        let to_far = |to| {
            accepts(
                &AIAction::CreateFleet {
                    fleet,
                    ships: vec![1],
                    to,
                },
                &world,
                &ResearchState::new(),
                AiFaction::Alliance,
                &transport,
                &crate::movement::MovementState::new(),
            )
        };
        // A target elsewhere needs a move the fleet can make: not to a
        // system that does not exist.
        assert!(!to_far(SystemKey::default()));
        assert!(to_far(here));
        assert!(to_far(far));
        let ok = |ships: Vec<usize>, side, movement: &crate::movement::MovementState| {
            accepts(
                &AIAction::CreateFleet {
                    fleet,
                    ships,
                    to: here,
                },
                &world,
                &ResearchState::new(),
                side,
                &transport,
                movement,
            )
        };
        assert!(ok(vec![1], AiFaction::Alliance, &movement));
        assert!(!ok(vec![1], AiFaction::Empire, &movement));
        assert!(!ok(vec![2], AiFaction::Alliance, &movement));
        assert!(!ok(vec![], AiFaction::Alliance, &movement));
        movement.order(fleet, here, here, 3);
        assert!(!ok(vec![1], AiFaction::Alliance, &movement));
    }

    #[test]
    fn the_ship_holds_follow_the_side_s_ships_and_reach_their_records() {
        // FUN_00484630 keeps a ship record's AI bits and +0x30/+0x34 (by
        // ship id); a hold whose ship is gone is dropped at the refresh.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let here = add_system(&mut world, sector, 0.5, 0.5);
        world.systems[here].sector = sector;
        let class = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        let fleet = world.fleets.insert(Fleet {
            location: here,
            capital_ships: ShipInstance::make(class, 100, true, 1),
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        world.systems[here].fleets.push(fleet);
        world.number_ships();
        let id = world.fleets[fleet].capital_ships[0].id;
        let mut state = AIState::new(AiFaction::Alliance);
        let hold = crate::ai_records::ShipHold {
            flags: 0,
            owner: 3,
            plan: 1,
        };
        state.ship_holds.insert(id, hold);
        state.ship_holds.insert(id + 50, hold);
        cadence_pass(&mut state, &world);
        assert_eq!(state.ship_holds, HashMap::from([(id, hold)]));
        let missions = MissionState::new();
        let busy = HashSet::new();
        let economy = crate::economy::EconomyState::default();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = ManufacturingState::new();
        let research = ResearchState::new();
        let movement = crate::movement::MovementState::new();
        let transport = crate::troop_transport::TroopTransportState::new();
        let inputs = RecordInputs {
            world: &world,
            economy: &economy,
            uprisings: &uprisings,
            manufacturing: &manufacturing,
            research: &research,
            movement: &movement,
            transport: &transport,
        };
        let planner = Planner::new(&world, &missions, &busy, true, 7);
        let galaxy = AISystem::records(&state, &planner, &inputs);
        let r = &galaxy.fleets[&fleet].ships[0];
        assert_eq!(
            (r.owner, r.plan, r.flags & crate::ai_records::SHIP_FREE),
            (3, 1, 0)
        );
    }

    #[test]
    fn a_planner_sends_its_damaged_ships_off_as_a_fleet() {
        // FUN_004774c0 state 1 (FUN_00477a40): the held fleet with a
        // damaged ship (0x3000) gives it back as a fleet of its own,
        // Create Fleet 0x270; the order completes module 8 and, the cycle
        // going on, raises its priority (FUN_004cf630).
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let here = add_system(&mut world, sector, 0.9, 0.1);
        world.systems[here].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            hull: 100,
            hyperdrive: 100,
            is_alliance: true,
            ..CapitalShipClass::default()
        });
        let mut ships = ShipInstance::make(class, 100, true, 2);
        ships[1].hull_current = 40;
        let fleet = world.fleets.insert(Fleet {
            location: here,
            capital_ships: ships,
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        world.systems[here].fleets.push(fleet);
        world.number_ships();
        let ids: Vec<u64> = world.fleets[fleet]
            .capital_ships
            .iter()
            .map(|s| s.id)
            .collect();
        let mut state = AIState::new(AiFaction::Alliance);
        with_sector_planner(&mut state, &world);
        let held = crate::ai_records::ShipHold {
            flags: 0,
            owner: 1,
            plan: 1,
        };
        for &id in &ids {
            state.ship_holds.insert(id, held);
        }
        state.fleet_holds.insert(
            fleet,
            crate::ai_records::FleetHold {
                flags: 0,
                owner: 1,
                plan: 1,
                target: None,
            },
        );
        let p = &mut state.module8.planners[0];
        p.ships = ids.clone();
        p.fleets = vec![fleet];
        state.module_cursor = Some(FLEETS - 1);

        let orders = cadence_pass(&mut state, &world);

        assert_eq!(
            orders,
            vec![AIAction::CreateFleet {
                fleet,
                ships: vec![1],
                to: here
            }]
        );
        assert_eq!(state.module_priority[FLEETS], 1);
        assert_eq!(state.module8.planners[0].ships, vec![ids[0]]);
        assert_eq!(state.ship_holds[&ids[1]].owner, 0);
    }

    #[test]
    fn a_planner_without_a_plan_object_ends_its_cycle() {
        // FUN_004774c0 for a planner without a plan object (+0x5c): its
        // cycle ends (FUN_004cf630: +0x3c = 0, complete) in the same pass.
        // FUN_00477630 makes kinds 1 to 6 only, so a kind 7 planner has
        // none; the port's stand-in fleet heuristics ran it until they were
        // removed (2026-10-17), and then the planner's state became 1.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let _ = add_system(&mut world, sector, 0.9, 0.1);
        let mut state = AIState::new(AiFaction::Alliance);
        with_sector_planner(&mut state, &world);
        state.strategy.plans[0].kind = 7;
        state.module8.planners[0].kind = 7;
        state.module_cursor = Some(FLEETS - 1);
        assert!(cadence_pass(&mut state, &world).is_empty());
        assert_eq!(state.module8.state, 0);
        assert_eq!(state.module8.planners[0].state, 0);
        assert_eq!(state.module_priority[FLEETS], 0);
    }

    #[test]
    fn a_refused_module_eight_order_ends_its_cycle() {
        // FUN_00485b40 sends module 8's refused order to its slot +0x18
        // (FUN_004dd810: +0x3c = 0). Here a kind 3 plan's state 0xd
        // (FUN_004b0f90) moves its fleet to its target, a destroyed system,
        // which the move's validation refuses.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let home = add_system(&mut world, sector, 0.9, 0.1);
        world.systems[home].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let gone = add_system(&mut world, sector, 0.9, 0.1);
        world.systems[gone].is_destroyed = true;
        let class = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        let fleet = world.fleets.insert(Fleet {
            location: home,
            capital_ships: ShipInstance::make(class, 100, true, 1),
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        world.systems[home].fleets.push(fleet);
        let mut state = AIState::new(AiFaction::Alliance);
        with_sector_planner(&mut state, &world);
        // Posture 1 kept from the start, so the strategy keeps its plan.
        state.strategy.posture = 1;
        state.strategy.kind = 1;
        state.fleet_holds.insert(
            fleet,
            crate::ai_records::FleetHold {
                owner: 1,
                plan: 1,
                target: Some(gone),
                ..crate::ai_records::FleetHold::default()
            },
        );
        {
            let p = &mut state.module8.planners[0];
            (p.state, p.plan_state) = (5, 0xd);
            p.plan_fleet = Some(fleet);
            p.fleets = vec![fleet];
        }
        (state.module8.state, state.module8.current) = (4, 1);
        state.module_priority[FLEETS] = 5;

        let orders = cadence_pass(&mut state, &world);

        assert!(orders.is_empty());
        assert_eq!(state.module8.state, 0);
        assert_eq!(state.module_priority[FLEETS], 0);
        assert_eq!(state.module8.planners[0].plan_state, 0xe);
    }

    /// An Alliance system holding, on its planet, 3 X-wings and 2 TIEs, with
    /// an Alliance carrier fleet of room `room` and an Empire carrier.
    fn squadron_world(room: u32) -> (GameWorld, SystemKey, FleetKey, FleetKey, FighterKey) {
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let system = add_system(&mut world, sector, 0.9, 0.1);
        world.systems[system].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let mut fleet = |is_alliance| {
            let carrier = world.capital_ship_classes.insert(CapitalShipClass {
                is_alliance,
                fighter_capacity: room,
                ..CapitalShipClass::default()
            });
            let key = world.fleets.insert(Fleet {
                location: system,
                capital_ships: vec![ShipInstance::new(carrier, 100, is_alliance)],
                fighters: vec![],
                characters: vec![],
                is_alliance,
                has_death_star: false,
            });
            world.systems[system].fleets.push(key);
            key
        };
        let empire = fleet(false);
        let ours = fleet(true);
        let x_wing = world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            ..FighterClass::default()
        });
        let tie = world.fighter_classes.insert(FighterClass {
            is_empire: true,
            ..FighterClass::default()
        });
        crate::squadrons::add(&mut world.systems[system].fighters, x_wing, 3);
        crate::squadrons::add(&mut world.systems[system].fighters, tie, 2);
        (world, system, ours, empire, x_wing)
    }

    #[test]
    fn the_ai_s_squadron_move_passes_the_squadron_move_s_checks() {
        // FUN_00578d00: the move command's validation for each member, here
        // squadrons::check_move.
        use crate::squadrons::SquadronPlace;
        let (world, system, ours, empire, x_wing) = squadron_world(2);
        let transport = crate::troop_transport::TroopTransportState::new();
        let movement = crate::movement::MovementState::new();
        let research = ResearchState::new();
        let ok = |count, to| {
            accepts(
                &AIAction::MoveSquadrons {
                    class: x_wing,
                    count,
                    from: SquadronPlace::Planet(system),
                    to: SquadronPlace::Fleet(to),
                },
                &world,
                &research,
                AiFaction::Alliance,
                &transport,
                &movement,
            )
        };
        assert!(ok(2, ours));
        assert!(!ok(3, ours));
        assert!(!ok(1, empire));
    }

    #[test]
    fn a_squadron_team_move_needs_every_class_s_squadron_move_to_pass() {
        // FUN_00578d00 for FUN_004b0120's Move: each squadron passes the
        // squadron move's checks (squadrons::check_move).
        use crate::squadrons::SquadronPlace;
        let (mut world, system, ours, empire, x_wing) = squadron_world(4);
        let y_wing = world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            ..FighterClass::default()
        });
        crate::squadrons::add(&mut world.systems[system].fighters, y_wing, 1);
        let transport = crate::troop_transport::TroopTransportState::new();
        let movement = crate::movement::MovementState::new();
        let research = ResearchState::new();
        let ok = |squadrons: Vec<(FighterKey, u32)>, to| {
            accepts(
                &AIAction::MoveSquadronTeam {
                    squadrons,
                    from: SquadronPlace::Planet(system),
                    to: SquadronPlace::Fleet(to),
                },
                &world,
                &research,
                AiFaction::Alliance,
                &transport,
                &movement,
            )
        };
        assert!(ok(vec![(x_wing, 3), (y_wing, 1)], ours));
        assert!(!ok(vec![], ours));
        assert!(!ok(vec![(x_wing, 3), (y_wing, 2)], ours));
        assert!(!ok(vec![(x_wing, 1)], empire));
    }

    #[test]
    fn a_people_move_needs_the_side_s_free_people_aboard_a_fleet_there() {
        // FUN_00578d00 for FUN_004b06b0's Move: each person the side's,
        // not a captive, aboard a fleet out of hyperspace at the target.
        let (mut world, system, ours, empire, _) = squadron_world(2);
        let sector = world.systems[system].sector;
        let there = add_system(&mut world, sector, 0.9, 0.1);
        let mut person = |fleet, is_alliance| {
            let c = world.characters.insert(Character {
                is_alliance,
                is_empire: !is_alliance,
                ..Character::default()
            });
            world.fleets[fleet].characters.push(c);
            c
        };
        let a = person(ours, true);
        let b = person(ours, true);
        let enemy = person(empire, false);
        let ashore = world.characters.insert(Character {
            is_alliance: true,
            ..Character::default()
        });
        let transport = crate::troop_transport::TroopTransportState::new();
        let mut movement = crate::movement::MovementState::new();
        let research = ResearchState::new();
        let ok = |world: &GameWorld,
                  movement: &crate::movement::MovementState,
                  characters: Vec<CharacterKey>,
                  to| {
            accepts(
                &AIAction::MoveCharacters { characters, to },
                world,
                &research,
                AiFaction::Alliance,
                &transport,
                movement,
            )
        };
        assert!(ok(&world, &movement, vec![a, b], system));
        assert!(!ok(&world, &movement, vec![], system));
        assert!(!ok(&world, &movement, vec![a], there));
        assert!(!ok(&world, &movement, vec![a, enemy], system));
        assert!(!ok(&world, &movement, vec![a, ashore], system));
        world.characters[b].is_captive = true;
        assert!(!ok(&world, &movement, vec![a, b], system));
        crate::movement::begin_faction_fleet_transit(&mut movement, &mut world, ours, there, true)
            .unwrap();
        assert!(!ok(&world, &movement, vec![a], system));
    }

    #[test]
    fn a_join_needs_the_side_s_fleet_and_another_of_its_fleets_to_reach() {
        // FUN_00578d00 for FUN_004b0c90's Move onto a fleet: both the
        // side's and out of hyperspace, not the fleet itself, and the
        // departure's checks when the target lies elsewhere
        // (fleet_join::validate_join).
        let (mut world, system, ours, empire, _) = squadron_world(2);
        let sector = world.systems[system].sector;
        let there = add_system(&mut world, sector, 0.9, 0.1);
        let class = world.fleets[ours].capital_ships[0].class;
        let fleet = |world: &mut GameWorld, at| {
            let key = world.fleets.insert(Fleet {
                location: at,
                capital_ships: vec![ShipInstance::new(class, 100, true)],
                fighters: vec![],
                characters: vec![],
                is_alliance: true,
                has_death_star: false,
            });
            world.systems[at].fleets.push(key);
            key
        };
        let here = fleet(&mut world, system);
        let away = fleet(&mut world, there);
        let transport = crate::troop_transport::TroopTransportState::new();
        let mut movement = crate::movement::MovementState::new();
        let research = ResearchState::new();
        let ok = |world: &GameWorld, movement: &crate::movement::MovementState, fleet, into| {
            accepts(
                &AIAction::JoinFleet { fleet, into },
                world,
                &research,
                AiFaction::Alliance,
                &transport,
                movement,
            )
        };
        assert!(ok(&world, &movement, ours, here));
        assert!(!ok(&world, &movement, ours, ours));
        assert!(!ok(&world, &movement, ours, empire));
        assert!(!ok(&world, &movement, empire, ours));
        // Elsewhere: the departure's checks too, which a destroyed system
        // fails.
        assert!(ok(&world, &movement, ours, away));
        world.systems[there].is_destroyed = true;
        assert!(!ok(&world, &movement, ours, away));
        world.systems[there].is_destroyed = false;
        crate::movement::begin_faction_fleet_transit(&mut movement, &mut world, here, there, true)
            .unwrap();
        assert!(!ok(&world, &movement, ours, here));
        assert!(!ok(&world, &movement, here, ours));
    }

    #[test]
    fn a_bombardment_needs_the_side_s_fleets_in_orbit_with_bombardment() {
        // The orders' enabled check: a team of one side (FUN_00553770), a
        // fleet with bombardment and, for 0x223, a Death Star in it
        // (FUN_00582440); port: in orbit, out of hyperspace, at a system
        // not the side's (the manual's "enemy or neutral system").
        let (mut world, system, ours, empire, _) = squadron_world(2);
        let class = world.fleets[empire].capital_ships[0].class;
        world.capital_ship_classes[class].bombardment_modifier = 5;
        world.capital_ship_classes[class].hull = 100;
        let sector = world.systems[system].sector;
        let there = add_system(&mut world, sector, 0.9, 0.1);
        let transport = crate::troop_transport::TroopTransportState::new();
        let mut movement = crate::movement::MovementState::new();
        let research = ResearchState::new();
        let ok = |world: &GameWorld,
                  movement: &crate::movement::MovementState,
                  fleets: Vec<FleetKey>,
                  kind| {
            accepts(
                &AIAction::Bombard {
                    fleets,
                    system,
                    kind,
                },
                world,
                &research,
                AiFaction::Empire,
                &transport,
                movement,
            )
        };
        let general = crate::bombardment::GENERAL;
        assert!(ok(&world, &movement, vec![empire], general));
        assert!(ok(
            &world,
            &movement,
            vec![empire],
            crate::bombardment::TARGET_MILITARY
        ));
        assert!(!ok(&world, &movement, vec![empire], 0x224));
        assert!(!ok(&world, &movement, vec![empire], 0x21f));
        assert!(!ok(&world, &movement, vec![], general));
        assert!(!ok(&world, &movement, vec![ours], general));
        assert!(!ok(&world, &movement, vec![empire, ours], general));
        // No Death Star: no Destroy System.
        assert!(!ok(
            &world,
            &movement,
            vec![empire],
            crate::bombardment::DESTROY_SYSTEM
        ));
        // The side's own system, or no bombardment: refused.
        world.systems[system].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        assert!(!ok(&world, &movement, vec![empire], general));
        world.systems[system].control = ControlKind::Uncontrolled;
        assert!(ok(&world, &movement, vec![empire], general));
        world.capital_ship_classes[class].bombardment_modifier = 0;
        assert!(!ok(&world, &movement, vec![empire], general));
        world.capital_ship_classes[class].bombardment_modifier = 5;
        // In hyperspace: refused.
        crate::movement::begin_faction_fleet_transit(
            &mut movement,
            &mut world,
            empire,
            there,
            false,
        )
        .unwrap();
        assert!(!ok(&world, &movement, vec![empire], general));
    }

    #[test]
    fn boarding_needs_the_side_s_regiments_and_a_fleet_with_room_there() {
        // FUN_00578d00 with a fleet for the destination (FUN_00553410):
        // the side's regiment at the fleet's system, the fleet the side's,
        // out of hyperspace, with room (FUN_00500b40).
        let (mut world, system, ours, empire, _) = squadron_world(2);
        let class = world.fleets[empire].capital_ships[0].class;
        world.capital_ship_classes[class].troop_capacity = 1;
        let transport = crate::troop_transport::TroopTransportState::new();
        let movement = crate::movement::MovementState::new();
        let research = ResearchState::new();
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: crate::ids::DatId::new(0x1000_0006),
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(troop);
        let ok = |world: &GameWorld, troops: Vec<TroopKey>, fleet| {
            accepts(
                &AIAction::BoardRegiments { troops, fleet },
                world,
                &research,
                AiFaction::Empire,
                &transport,
                &movement,
            )
        };
        assert!(ok(&world, vec![troop], empire));
        assert!(!ok(&world, vec![], empire));
        assert!(!ok(&world, vec![troop], ours));
        world.capital_ship_classes[class].troop_capacity = 0;
        assert!(!ok(&world, vec![troop], empire));
    }

    #[test]
    fn an_assault_needs_the_side_s_fleet_in_orbit_carrying_a_regiment() {
        // Planetary Assault's enabled check (FUN_00581ae0): a team fleet
        // carrying a regiment; port: assault::order_enabled's orbit and
        // system tests.
        let (mut world, system, ours, empire, _) = squadron_world(2);
        let class = world.fleets[empire].capital_ships[0].class;
        world.capital_ship_classes[class].troop_capacity = 2;
        let mut transport = crate::troop_transport::TroopTransportState::new();
        let movement = crate::movement::MovementState::new();
        let research = ResearchState::new();
        let ok = |world: &GameWorld,
                  transport: &crate::troop_transport::TroopTransportState,
                  fleets: Vec<FleetKey>| {
            accepts(
                &AIAction::Assault { fleets, system },
                world,
                &research,
                AiFaction::Empire,
                transport,
                &movement,
            )
        };
        assert!(!ok(&world, &transport, vec![empire]));
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: crate::ids::DatId::new(0x1000_0006),
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(troop);
        transport.embark(&mut world, empire, &[troop]).unwrap();
        assert!(ok(&world, &transport, vec![empire]));
        assert!(!ok(&world, &transport, vec![ours]));
        assert!(!ok(&world, &transport, vec![]));
        world.systems[system].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        assert!(!ok(&world, &transport, vec![empire]));
    }

    #[test]
    fn the_ai_s_ship_move_needs_its_own_fleet_s_living_ships_and_another_system() {
        // FUN_00578d00 for module 0xb's Move (FUN_004c8e90, FUN_004c94c0):
        // the side's own fleet out of hyperspace, ships of it that live,
        // and a destination it can enter; port: checked as the whole
        // fleet, so the fleet's own system is refused.
        let (mut world, system, ours, empire, _) = squadron_world(2);
        let sector = world.systems[system].sector;
        let there = add_system(&mut world, sector, 0.9, 0.1);
        let transport = crate::troop_transport::TroopTransportState::new();
        let mut movement = crate::movement::MovementState::new();
        let research = ResearchState::new();
        let ok = |world: &GameWorld,
                  movement: &crate::movement::MovementState,
                  fleet,
                  ships: Vec<usize>,
                  to| {
            accepts(
                &AIAction::MoveShips { fleet, ships, to },
                world,
                &research,
                AiFaction::Alliance,
                &transport,
                movement,
            )
        };
        assert!(ok(&world, &movement, ours, vec![0], there));
        assert!(!ok(&world, &movement, ours, vec![], there));
        assert!(!ok(&world, &movement, ours, vec![1], there));
        assert!(!ok(&world, &movement, ours, vec![0], system));
        assert!(!ok(&world, &movement, empire, vec![0], there));
        world.fleets[ours].capital_ships[0].alive = false;
        assert!(!ok(&world, &movement, ours, vec![0], there));
        world.fleets[ours].capital_ships[0].alive = true;
        crate::movement::begin_faction_fleet_transit(&mut movement, &mut world, ours, there, true)
            .unwrap();
        assert!(!ok(&world, &movement, ours, vec![0], system));
    }

    #[test]
    fn a_regiment_move_needs_the_side_s_regiments_each_able_to_move() {
        // The move command's validation (FUN_00578d00) for each member:
        // the side's own (FUN_004f9860), then the regiment move's checks
        // (TroopTransportState::check_move_regiment); a Destination needs
        // the manager of its area.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let from = add_system(&mut world, sector, 0.9, 0.1);
        let to = add_system(&mut world, sector, 0.9, 0.1);
        let enemy = add_system(&mut world, sector, 0.1, 0.9);
        for s in [from, to] {
            world.systems[s].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        }
        world.systems[enemy].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        world.systems[enemy].is_populated = true;
        let mut add = |is_alliance| {
            let key = world.troops.insert(crate::world::TroopUnit {
                class_dat_id: DatId(0x1000_0001),
                is_alliance,
                regiment_strength: 100,
            });
            world.systems[from].ground_units.push(key);
            key
        };
        let ours = add(true);
        let theirs = add(false);
        let transport = crate::troop_transport::TroopTransportState::new();
        let movement = crate::movement::MovementState::new();
        let research = ResearchState::new();
        let ok = |troops: Vec<TroopKey>, to| {
            accepts(
                &AIAction::MoveRegiments { troops, to },
                &world,
                &research,
                AiFaction::Alliance,
                &transport,
                &movement,
            )
        };
        assert!(ok(vec![ours], to));
        assert!(!ok(vec![], to));
        assert!(!ok(vec![ours, theirs], to));
        assert!(!ok(vec![ours], enemy));
        let destination = AIAction::SetDestination {
            system: from,
            area: ProductionArea::TrainingFacility,
            destination: to,
        };
        let check = |world: &GameWorld| {
            accepts(
                &destination,
                world,
                &research,
                AiFaction::Alliance,
                &transport,
                &movement,
            )
        };
        let mut world = world.clone();
        let yard = |world: &mut GameWorld, class: u32| {
            let key = world.manufacturing_facilities.insert(
                crate::world::ManufacturingFacilityInstance {
                    class_dat_id: DatId(class),
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: false,
                },
            );
            world.systems[from].manufacturing_facilities.push(key);
        };
        yard(&mut world, 0x2a00_0003);
        assert!(!check(&world));
        yard(&mut world, 0x2900_0001);
        assert!(check(&world));
    }

    #[test]
    fn the_round_robin_wraps_round_the_list_to_a_module_with_orders() {
        // FUN_004be980: +0x10 steps to the next module and goes round the
        // list back to where it began.
        let world = empty_world();
        // Index 6 (module 4): indices 1 to 5 are modules 2, 1, 3, 6 and 7,
        // which run their own cycles rather than lists of decided orders.
        let order = AIAction::AbortMission { mission_id: 7 };
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_pending[6] = vec![order.clone()];
        state.module_cursor = Some(MODULES - 2);

        assert_eq!(cadence_pass(&mut state, &world), vec![order]);
        assert_eq!(state.module_cursor, Some(6));
        // Every module runs and orders nothing; +0x10 stops where the last
        // search began, one past module 4.
        assert!(cadence_pass(&mut state, &world).is_empty());
        assert_eq!(state.module_cursor, Some(7));
    }

    #[test]
    fn phase_four_builds_the_side_s_records_with_its_marks_and_minor_classes() {
        // FUN_00418500 refreshes the side's records each AI tick:
        // FUN_00433e40 keeps the yard marks (+0x28 & 0x3e00000) and, for an
        // own calm system, sets +0x2c 2 while the side has MNCHARSD classes
        // (side +0xa4, FUN_0042dd00).
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let sys = add_system(&mut world, sector, 0.5, 0.5);
        world.systems[sys].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let minor = add_character(&mut world, true, false, 0);
        world.characters[minor].dat_id = DatId(0x3800_0001);
        let missions = MissionState::new();
        let busy = HashSet::new();
        let economy = crate::economy::EconomyState::default();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = ManufacturingState::new();
        let research = ResearchState::new();
        let movement = crate::movement::MovementState::new();
        let transport = crate::troop_transport::TroopTransportState::new();
        let inputs = RecordInputs {
            world: &world,
            economy: &economy,
            uprisings: &uprisings,
            manufacturing: &manufacturing,
            research: &research,
            movement: &movement,
            transport: &transport,
        };
        let mut state = AIState::new(AiFaction::Alliance);
        state.set_yard_marks(sys, MARK_SHIPYARD);
        let planner = Planner::new(&world, &missions, &busy, true, 7);

        let galaxy = AISystem::records(&state, &planner, &inputs);

        let r = galaxy.system(sys).unwrap();
        assert_eq!(r.state & YARD_MARKS, MARK_SHIPYARD);
        assert_eq!(r.summary & 2, 2);
        assert_eq!(r.kind & 1, 1);
        // The Empire sees the system as the opponent's (+0x30 4).
        let planner = Planner::new(&world, &missions, &busy, false, 7);
        let galaxy = AISystem::records(&AIState::new(AiFaction::Empire), &planner, &inputs);
        assert_eq!(galaxy.system(sys).unwrap().kind & 5, 4);
    }

    #[test]
    fn the_refreshed_sector_records_carry_the_strategy_s_plans() {
        // FUN_004321e0 keeps +0x2c 0x8000000 and reads +0x30/+0x38: the
        // planned sector, with no planner, has 0x4000000, 0x10000 and
        // +0x18c = -1, and its +0x3c is the plan's id.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let _ = add_system(&mut world, sector, 0.5, 0.5);
        let missions = MissionState::new();
        let busy = HashSet::new();
        let economy = crate::economy::EconomyState::default();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = ManufacturingState::new();
        let research = ResearchState::new();
        let movement = crate::movement::MovementState::new();
        let transport = crate::troop_transport::TroopTransportState::new();
        let inputs = RecordInputs {
            world: &world,
            economy: &economy,
            uprisings: &uprisings,
            manufacturing: &manufacturing,
            research: &research,
            movement: &movement,
            transport: &transport,
        };
        let mut state = AIState::new(AiFaction::Alliance);
        state.strategy.plans = vec![crate::ai_strategy::SectorPlan {
            sector,
            kind: 3,
            param: 0,
        }];
        let planner = Planner::new(&world, &missions, &busy, true, 7);

        let galaxy = AISystem::records(&state, &planner, &inputs);

        let r = galaxy.sector(sector).unwrap();
        assert_eq!(r.summary & 0xc01_0000, 0xc01_0000);
        assert_eq!((r.plan, r.plan_id), (3, 1));
        assert_eq!(r.values[crate::ai_records::sector_at(0x18c)], -1);
        // With its planner (+0x38) the sector is no longer short of one.
        state.module8.planners = vec![crate::ai_module8::SectorPlanner {
            id: 4,
            sector: Some(sector),
            ..crate::ai_module8::SectorPlanner::default()
        }];
        let galaxy = AISystem::records(&state, &planner, &inputs);
        let r = galaxy.sector(sector).unwrap();
        assert_eq!((r.planner, r.summary & 0x1_0000), (4, 0));
        assert_eq!(r.values[crate::ai_records::sector_at(0x18c)], 0);
        // A swept sector keeps +0x2c 0x20000000 through the refresh
        // (FUN_004321e0 keeps 0x28000000), after the plan's bits.
        assert_eq!(r.summary & 0x2000_0000, 0);
        state.swept = vec![sector];
        let galaxy = AISystem::records(&state, &planner, &inputs);
        let r = galaxy.sector(sector).unwrap();
        assert_eq!(r.summary & 0x2800_0000, 0x2800_0000);
    }

    #[test]
    fn a_side_s_minor_classes_are_its_minor_characters() {
        // Side +0xa4 counts the side's MNCHARSD (family 0x38) classes.
        // hyp: one minor character per MNCHARSD record. The loader's minor
        // characters carry no family byte, so the last counts too.
        let mut world = empty_world();
        for (raw, alliance) in [
            (0x3800_0001, true),
            (0x3800_0002, true),
            (0x3000_0001, true),
        ] {
            let c = add_character(&mut world, alliance, raw >> 24 == 0x30, 0);
            world.characters[c].dat_id = DatId(raw);
        }
        let both = add_character(&mut world, true, false, 0);
        world.characters[both].dat_id = DatId(0x3800_0003);
        world.characters[both].is_empire = true;
        let loaded = add_character(&mut world, false, false, 0);
        world.characters[loaded].dat_id = DatId(0x2c7);

        assert_eq!(minor_classes(&world, true), 3);
        assert_eq!(minor_classes(&world, false), 2);
    }

    #[test]
    fn yard_marks_stay_until_a_module_clears_them() {
        // FUN_00433e40 keeps +0x28 & 0x3e00000 across refreshes; module 1
        // sets 0x800000 (FUN_004daf10) and drops it (FUN_004daca0).
        let mut systems = slotmap::SlotMap::<SystemKey, ()>::with_key();
        let (a, b) = (systems.insert(()), systems.insert(()));
        let mut state = AIState::new(AiFaction::Empire);

        state.set_yard_marks(a, MARK_MINES);
        state.set_yard_marks(a, MARK_SHIPYARD | 0x4000);
        state.set_yard_marks(b, 0x4000);

        assert_eq!(state.yard_marks(a), MARK_MINES | MARK_SHIPYARD);
        assert_eq!(state.yard_marks(b), 0);
        assert!(!state.yard_marks.contains_key(&b));
        state.begin_day();
        assert_eq!(state.yard_marks(a), MARK_MINES | MARK_SHIPYARD);

        state.clear_yard_marks(a, MARK_MINES);
        assert_eq!(state.yard_marks(a), MARK_SHIPYARD);
        state.clear_yard_marks(a, MARK_SHIPYARD);
        assert!(state.yard_marks.is_empty());
        state.clear_yard_marks(b, MARK_MINES);
        assert!(state.yard_marks.is_empty());
    }

    #[test]
    fn the_five_yard_marks_fill_the_kept_mask() {
        // FUN_00433e40's kept mask 0x3e00000 is the five marks:
        // 0x200000 ships, 0x400000 troops, 0x800000/0x1000000/0x2000000
        // construction yards (FUN_00432350 reads 0x3800000 as one).
        let all = [
            MARK_SHIPYARD,
            MARK_TRAINING,
            MARK_MINES,
            MARK_FACILITIES,
            MARK_BALANCE,
        ];
        assert_eq!(all.iter().fold(0, |m, b| m | b), YARD_MARKS);
        assert_eq!(MARK_MINES | MARK_FACILITIES | MARK_BALANCE, 0x380_0000);
        assert_eq!(all.iter().map(|b| b.count_ones()).sum::<u32>(), 5);
    }

    #[test]
    fn the_day_s_tenth_order_stops_the_ai_until_the_next_day() {
        // FUN_00485a50 sets the budget +0x428 to 10; phase 3 runs no module
        // once the count +0x420 reaches it, and FUN_00485de0 clears the
        // count at the day boundary. A module's orders left wait for it.
        let (world, ship) = cadence_world(1);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_pending[TROOPS] = vec![queued(1), queued(2)];
        state.mark_busy(ship.character().unwrap());
        state.orders_today = ORDERS_PER_DAY - 1;

        assert_eq!(cadence_pass(&mut state, &world), vec![queued(1)]);
        assert_eq!(state.orders_today, ORDERS_PER_DAY);
        assert!(cadence_pass(&mut state, &world).is_empty());
        assert_eq!(state.module_pending[TROOPS].len(), 1);

        state.begin_day();
        assert_eq!(state.orders_today, 0);
        assert_eq!(cadence_pass(&mut state, &world), vec![queued(2)]);
        assert!(state.module_pending[TROOPS].is_empty());
    }

    #[test]
    fn a_mission_step_runs_one_entry_and_the_next_step_the_one_after() {
        // FUN_004d2490 state 5 (FUN_004d2730) takes the entry +0x4c names,
        // else the first, and stores the one after it; state 6 runs it.
        // port: the research planners come first, then the groups.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let shipyard = yard_system(&mut world, sector, 0x28);
        let first = designer(&mut world, TechType::Ship, 60);
        let second = designer(&mut world, TechType::Ship, 40);
        let mut state = AIState::new(AiFaction::Alliance);
        let (missions, research, mfg, config, uprisings) = (
            MissionState::new(),
            ResearchState::new(),
            ManufacturingState::new(),
            GameConfig::default(),
            crate::uprising::UprisingState::default(),
        );
        let ctx = inputs(&world, &missions, &research, &mfg, &config, &uprisings);
        let busy = state.busy_characters.clone();
        let mut planner = Planner::new(&world, &missions, &busy, true, 7);
        AISystem::research_strategy(
            &mut state,
            &mut planner,
            &world,
            &missions,
            &research,
            &uprisings,
            AiFaction::Alliance,
            &mut Vec::new(),
        );
        let ids: Vec<u64> = state.research_planners.iter().map(|p| p.id).collect();
        let mut step = |state: &mut AIState| {
            let mut actions = Vec::new();
            AISystem::mission_step(
                state,
                &mut planner,
                &ctx,
                &mut crate::ai_records::Galaxy::default(),
                &mut actions,
            );
            orders(&actions, MissionKind::Move)
        };

        assert_eq!(step(&mut state), vec![(vec![first], shipyard, None)]);
        assert_eq!(state.mission_next, Some(MissionEntry::Research(ids[1])));
        assert_eq!(step(&mut state), vec![(vec![second], shipyard, None)]);
        assert_eq!(state.mission_next, Some(MissionEntry::Group(0)));
        for group in 0..MISSION_GROUPS {
            assert!(step(&mut state).is_empty());
            let after = (group + 1 < MISSION_GROUPS).then_some(MissionEntry::Group(group + 1));
            assert_eq!(state.mission_next, after);
        }
        assert_eq!(step(&mut state), vec![(vec![first], shipyard, None)]);
    }

    #[test]
    fn a_day_has_as_many_ai_passes_as_the_speed_s_rate() {
        // FUN_0051dea0: sub-ticks a day = side +0xc4, which the speed order
        // 0x280 sets to the rate (FUN_00628ff0): 600, 60, 12, 4.
        use crate::tick::GameSpeed;
        assert_eq!(passes_per_day(GameSpeed::VerySlow), 600);
        assert_eq!(passes_per_day(GameSpeed::Slow), 60);
        assert_eq!(passes_per_day(GameSpeed::Medium), 12);
        assert_eq!(passes_per_day(GameSpeed::Fast), 4);
        // port: play-test days while Paused.
        assert_eq!(passes_per_day(GameSpeed::Paused), 4);
    }

    #[test]
    fn another_side_s_or_another_tree_s_research_blocks_no_order() {
        // FUN_004e5710 and the counts +0x324.. look at the side's own
        // planners of the subtype only.
        let mut world = empty_world();
        let sector = add_sector(&mut world);
        let shipyard = yard_system(&mut world, sector, 0x28);
        let ship = designer(&mut world, TechType::Ship, 50);
        let enemy = add_character(&mut world, false, false, 0);
        let mut missions = MissionState::new();
        missions.dispatch(crate::missions::MissionRequest {
            kind: MissionKind::ShipDesignResearch,
            faction: crate::missions::MissionFaction::Empire,
            team: vec![MissionMember::Character(enemy)],
            decoys: vec![],
            target_system: shipyard,
            target_character: None,
            target_object: None,
            tick: 1,
        });

        world.characters[ship.character().unwrap()].current_system = Some(shipyard);

        let actions = research_orders(&world, &missions, &ResearchState::new());

        assert_eq!(
            orders(&actions, MissionKind::ShipDesignResearch),
            vec![(vec![ship], shipyard, None)]
        );
    }
}
