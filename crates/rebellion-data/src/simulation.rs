//! Shared simulation tick function for headless and interactive use.
//!
//! Extracts the per-tick system-advance logic from `rebellion-app/main.rs`
//! into a reusable function that returns structured `GameEventRecord`s
//! instead of pushing to a `MessageLog`.

use std::collections::HashMap;

use crate::integrator::{resolve_system_space_combat, PerceptionIntegrator};
use rebellion_core::ai::{AIState, AISystem, AiFaction};
use rebellion_core::ai_schedule::{AiDay, AiPassHost, AiSlot};
use rebellion_core::blockade::{BlockadeState, BlockadeSystem};
use rebellion_core::delivery::DeliveryState;
use rebellion_core::economy::{EconomyState, EconomySystem};
use rebellion_core::events::{EventState, EventSystem};
use rebellion_core::fog::{FogState, FogSystem};
use rebellion_core::game_events::GameEventRecord;
use rebellion_core::ids::SystemKey;
use rebellion_core::manufacturing::ManufacturingState;
use rebellion_core::missions::{MissionState, MissionSystem};
use rebellion_core::movement::{reconcile_fleet_orbits, MovementState, MovementSystem};
use rebellion_core::production_cycle::{
    advance_yards, scrap_yardless_queues, suspended_systems, ProductionState, ProductionSystem,
};
use rebellion_core::repair::{RepairState, RepairSystem};
use rebellion_core::research::ResearchState;
use rebellion_core::squadrons;
use rebellion_core::tick::{GameClock, TickEvent};
use rebellion_core::troop_transport::TroopTransportState;
use rebellion_core::uprising::{UprisingState, UprisingSystem};
use rebellion_core::victory::{VictoryState, VictorySystem};
use rebellion_core::world::ControlKind;
use rebellion_core::world::{CampaignConfig, GameWorld};

/// Days before the headless runner tries a system's battle again. hyp: the
/// original's battle trigger is unrecovered (F-031).
const COMBAT_RETRY_TICKS: u64 = 5;
const UNCHANGED_STALEMATE_COOLDOWN: u64 = u64::MAX;

fn combat_is_on_cooldown(last_battle: u64, current_tick: u64) -> bool {
    last_battle == UNCHANGED_STALEMATE_COOLDOWN
        || current_tick < last_battle.saturating_add(COMBAT_RETRY_TICKS)
}

/// Bundles all mutable simulation state needed for a tick.
///
/// Mirrors the set of `*State` locals in `rebellion-app/src/main.rs`.
pub struct SimulationStates {
    pub clock: GameClock,
    pub manufacturing: ManufacturingState,
    pub missions: MissionState,
    pub events: EventState,
    pub ai: AIState,
    /// Optional second AI for dual-AI mode (controls the opposite faction).
    pub ai2: Option<AIState>,
    pub movement: MovementState,
    pub fog: FogState,
    pub blockade: BlockadeState,
    pub uprising: UprisingState,
    pub research: ResearchState,
    pub victory: VictoryState,
    pub economy: EconomyState,
    pub repair: RepairState,
    /// Mines, refineries and each side's stockpiles.
    pub production: ProductionState,
    /// Each side's autoscrap timer `0x382` (the maintenance shortfall).
    pub autoscrap: rebellion_core::autoscrap::AutoscrapState,
    /// Each held system's informant timer `0x38e`.
    pub informant: rebellion_core::informant::InformantState,
    /// Each healing character's timer `0x388` (v68).
    pub healing: rebellion_core::healing::HealingState,
    /// Each discovering trainer's timer `0x389` (v70).
    pub force_discovery: rebellion_core::force_discovery::ForceDiscoveryState,
    /// The mission manager's Dagobah trigger, timer `0x392` (v73).
    pub dagobah: rebellion_core::dagobah::DagobahState,
    /// The mission manager's Final Battle bits (`final_battle.rs`).
    pub final_battle: rebellion_core::final_battle::FinalBattleState,
    /// The bounty hunters (`jabba.rs`).
    pub jabba: rebellion_core::jabba::JabbaState,
    /// `SeatOfPower` (`seat_of_power.rs`).
    pub seat_of_power: rebellion_core::seat_of_power::SeatOfPowerState,
    pub troop_transport: TroopTransportState,
    /// Manufactured objects travelling to their destination (F-030).
    pub deliveries: DeliveryState,
    pub combat_cooldowns: HashMap<SystemKey, u64>,
    /// Original new-game choices that continue to govern this campaign.
    pub campaign_config: CampaignConfig,
    /// The current day's AI passes (`rebellion_core::ai_schedule`, v132).
    pub ai_day: AiDay,
}

/// The systems a Destroy System order's Death Star destroyed this tick
/// (`FUN_0058d990`), as the Death Star's `PlanetDestroyed` events, which
/// step 11 carries out.
fn planets_destroyed(
    bombard: &rebellion_core::bombardment::BombardEnv,
    tick: u64,
) -> Vec<rebellion_core::death_star::DeathStarEvent> {
    bombard
        .destroyed_systems()
        .into_iter()
        .map(|system| rebellion_core::death_star::DeathStarEvent::PlanetDestroyed { system, tick })
        .collect()
}

/// A draw of `0..m` from a uniform roll in `0..1` (`m` at least 1).
fn draw_from_roll(roll: f64, m: i32) -> i32 {
    let m = m.max(1);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the product is floored into 0..m, which fits an i32"
    )]
    let value = (roll * f64::from(m)).floor() as i32;
    value.clamp(0, m - 1)
}

/// `Event305_SystemBlockadeChanged_Handle` -> `FUN_00561280`: each ended
/// blockade's loyalty shift for its assault and bombardment losses.
fn settle_ended_blockades(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    integrator: &mut PerceptionIntegrator,
) {
    for (system, blockade) in states.blockade.take_ended() {
        let shift = rebellion_core::loyalty::settle_blockade(world, system, blockade);
        integrator.record_loyalty_shifts(shift.as_slice());
    }
}

/// After an AI pass whose orders were applied: the state the original's
/// event handlers keep current within the sub-tick, so the next pass reads
/// it (`ghidra/notes/ai-scheduling.md`).
///
/// - The pass's bombardment and assault losses count toward their
///   blockades (`System_vtd0`), then the blockade follows the fleets
///   ([`BlockadeSystem::detect`]; `FUN_00511300` arms timer `0x386` on
///   entering), and an ended blockade settles its loyalty.
/// - The victory conditions word (`FUN_005180e0`) is stored with its
///   `0x128` as the event that changes it is handled.
///
/// Returns whether the word now ends the game: its game-over bit holds
/// the clock before the next sub-tick (`FUN_0041e390` -> `FUN_0051de80`),
/// so the day's remaining AI passes do not run.
fn settle_after_pass(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    integrator: &mut PerceptionIntegrator,
    bombard: &mut rebellion_core::bombardment::BombardEnv,
    presentation: &mut dyn DayPresentation,
    tick: u64,
) -> bool {
    // ── 11. The pass's bombardments and assaults, and the planets a
    // Death Star destroyed (FUN_0058d990), are handled as they happen.
    presentation.pass_settling(world, bombard, tick);
    for (req, outcome) in &bombard.outcomes {
        integrator.emit_bombardment(world, req, outcome);
    }
    for (req, outcome) in &bombard.assaults {
        integrator.emit_assault(world, req, outcome);
    }
    let ds_events = planets_destroyed(bombard, tick);
    integrator.apply_death_star_events(world, &ds_events);
    // Knesset Shamash-Bet #R11: `cleanup_destroyed_system` drains an
    // out-parameter `Vec<GameEffect>`; each `GameEffect::CharacterKilled`
    // becomes an `EVT_CHARACTER_KILLED` telemetry record.
    for evt in &ds_events {
        let rebellion_core::death_star::DeathStarEvent::PlanetDestroyed { system, .. } = evt;
        let mut cleanup_effects: Vec<rebellion_core::effects::GameEffect> = Vec::new();
        rebellion_core::death_star::cleanup_destroyed_system(
            world,
            *system,
            &bombard.killed_at(*system),
            &mut states.manufacturing,
            &mut states.blockade,
            &mut cleanup_effects,
        );
        let dest_sys_name = world
            .systems
            .get(*system)
            .map_or_else(|| String::from("<unknown>"), |s| s.name.clone());
        for effect in cleanup_effects.drain(..) {
            if let rebellion_core::effects::GameEffect::CharacterKilled { character } = effect {
                let (name, dat_id) = world.characters.get(character).map_or_else(
                    || (String::from("<unknown>"), 0),
                    |c| (c.name.clone(), c.dat_id.raw()),
                );
                integrator.emit(
                    rebellion_core::game_events::SYS_MISSIONS,
                    rebellion_core::game_events::EVT_CHARACTER_KILLED,
                    serde_json::json!({
                        "name": name,
                        "dat_id": dat_id,
                        "cause": "death_star",
                        "system": dest_sys_name,
                    }),
                );
            }
        }
    }
    presentation.planets_destroyed(world, &ds_events);
    bombard.outcomes.clear();
    bombard.assaults.clear();
    bombard.reports.clear();
    // System_vtd0: the losses count toward each blockade's (DestroyedReason
    // 0xf, 0xe).
    for (system, losses) in bombard.losses.drain(..) {
        states.blockade.record_losses(system, losses);
    }
    let events = BlockadeSystem::detect(&mut states.blockade, world, tick);
    presentation.blockade(world, &events);
    integrator.apply_blockade_events(world, &mut states.troop_transport, &events);
    settle_ended_blockades(world, states, integrator);
    presentation.pass_settled();
    let conditions = states.campaign_config.victory_conditions;
    if let Some((old, new)) =
        rebellion_core::victory::advance_conditions_word(&mut states.victory, world, conditions)
    {
        integrator.record_victory_conditions(old, new, tick);
        presentation.victory_word(world, new, tick);
    }
    // The word's game-over bit decides the game as the event is handled.
    match VictorySystem::outcome(&states.victory, world, conditions) {
        Some(outcome) => {
            integrator.apply_victory(&outcome, &mut states.victory, world);
            presentation.game_decided(world, &outcome, &states.victory, tick);
            true
        }
        None => states.victory.resolved,
    }
}

/// A headless campaign's first views: each side's setup push
/// (`FUN_0051b990`) and the Empire's push of Yavin, as the app's new game
/// takes them (`object_views::start`, `ghidra/notes/fog-of-war.md`).
pub fn start_views(world: &mut GameWorld, states: &SimulationStates) {
    rebellion_core::object_views::start(
        world,
        &states.movement,
        &states.missions,
        &states.troop_transport,
        &states.uprising,
    );
}

/// Run one simulation tick across all 15 systems.
///
/// Advances each system in the canonical order (economy → manufacturing → movement →
/// combat → fog → missions → events → blockade → the day's timers (uprising,
/// informants, healing, Force discovery, repair, mines and refineries,
/// autoscrap, the yards' research) → running regiments → the day's AI
/// passes in one burst → running regiments → loyalty → views → victory),
/// applies effects to `world`, and returns a `Vec<GameEventRecord>`
/// describing everything that happened. Every timer due on a day fires
/// before the day's first AI tick (`ghidra/notes/ai-scheduling.md`). The
/// three parts are [`begin_simulation_day`], [`run_ai_passes`] and
/// [`close_simulation_day`]; the interactive app spreads the passes over
/// the day's sub-ticks.
///
/// `tick_events` comes from `GameClock::advance()`. `rolls` is a pre-generated
/// slice of uniform `[0,1)` f64 values consumed sequentially by systems that
/// need randomness. Pass at least 1024 rolls to cover a typical tick.
///
/// `wall_ms` is the wall-clock milliseconds since session start, used for
/// the `wall_ms` field on each event record.
pub fn run_simulation_tick(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    tick_events: &[TickEvent],
    rolls: &[f64],
    wall_ms: u64,
) -> Vec<GameEventRecord> {
    let Some(last_tick_event) = tick_events.last() else {
        return Vec::new();
    };
    let mut integrator = PerceptionIntegrator::new(last_tick_event.tick, wall_ms);
    begin_simulation_day(world, states, tick_events, rolls, &mut integrator);
    // The headless runner has no input between sub-ticks: the day's passes
    // run in one burst (rebellion_core::ai_schedule).
    run_ai_passes(
        world,
        states,
        &mut integrator,
        &mut NoPresentation,
        u32::MAX,
    );
    let mut day_rolls = SliceRolls::at(rolls, states.ai_day.roll_cursor as usize);
    close_simulation_day(
        world,
        states,
        &mut day_rolls,
        &mut integrator,
        &mut NoPresentation,
    );
    integrator.finish()
}

/// The headless host of the AI's passes ([`AiPassHost`]): the integrator
/// applies their orders, and [`settle_after_pass`] follows each pass that
/// applied any.
struct HeadlessPasses<'a> {
    world: &'a mut GameWorld,
    states: &'a mut SimulationStates,
    integrator: &'a mut PerceptionIntegrator,
    bombard: rebellion_core::bombardment::BombardEnv,
    presentation: &'a mut dyn DayPresentation,
}

impl AiPassHost for HeadlessPasses<'_> {
    fn faction(&self, slot: AiSlot) -> Option<AiFaction> {
        let ai = match slot {
            AiSlot::Primary => Some(&self.states.ai),
            AiSlot::Secondary => self.states.ai2.as_ref(),
        };
        ai.map(|ai| ai.faction.unwrap_or(AiFaction::Empire))
    }

    fn think(&mut self, slot: AiSlot, day: u64) -> Vec<rebellion_core::ai::AIAction> {
        let states = &mut *self.states;
        let ai = match slot {
            AiSlot::Primary => &mut states.ai,
            AiSlot::Secondary => match states.ai2.as_mut() {
                Some(ai) => ai,
                None => return Vec::new(),
            },
        };
        AISystem::pass(
            ai,
            self.world,
            &states.manufacturing,
            &states.deliveries,
            &states.missions,
            &states.movement,
            &states.economy,
            &states.troop_transport,
            day,
            &states.research,
            &states.uprising,
        )
    }

    fn apply(&mut self, slot: AiSlot, actions: &[rebellion_core::ai::AIAction], day: u64) {
        let states = &mut *self.states;
        let ai = match slot {
            AiSlot::Primary => &mut states.ai,
            AiSlot::Secondary => match states.ai2.as_mut() {
                Some(ai) => ai,
                None => return,
            },
        };
        let side = ai.faction.unwrap_or(AiFaction::Empire);
        let (outcomes, assaults) = (self.bombard.outcomes.len(), self.bombard.assaults.len());
        let applied = self.integrator.apply_ai_actions(
            actions,
            ai,
            &mut states.missions,
            &mut states.manufacturing,
            &mut states.movement,
            &mut states.troop_transport,
            &mut states.production,
            self.world,
            day,
            slot == AiSlot::Secondary,
            &mut self.bombard,
        );
        self.presentation.ai_actions(
            self.world,
            &AiOrders {
                side,
                actions,
                applied: &applied,
                bombardments: &self.bombard.outcomes[outcomes..],
                assaults: &self.bombard.assaults[assaults..],
                day,
            },
        );
    }

    fn settle(&mut self, day: u64) -> bool {
        settle_after_pass(
            self.world,
            self.states,
            self.integrator,
            &mut self.bombard,
            self.presentation,
            day,
        )
    }
}

/// A new AI day for each side: the order count clears (`FUN_00485de0`),
/// the game type is set, and new capital ships get the ids the AI follows
/// them by (`GameWorld::number_ships`).
fn begin_ai_day(world: &mut GameWorld, states: &mut SimulationStates) {
    let conditions = states.campaign_config.victory_conditions;
    for ai in std::iter::once(&mut states.ai).chain(states.ai2.as_mut()) {
        ai.begin_day();
        ai.set_game_type(conditions);
    }
    world.number_ships();
}

/// Run the current day's AI passes until `upto` have run
/// ([`AiDay::run_until`]); the headless runner passes `u32::MAX` for the
/// whole day. Returns whether a pass ended the game.
pub fn run_ai_passes(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    integrator: &mut PerceptionIntegrator,
    presentation: &mut dyn DayPresentation,
    upto: u32,
) -> bool {
    let mut day = states.ai_day;
    let bombard = rebellion_core::bombardment::BombardEnv::new(
        day.bombard_state,
        Some(states.victory.alliance_hq),
    );
    let mut host = HeadlessPasses {
        world,
        states,
        integrator,
        bombard,
        presentation,
    };
    let ended = day.run_until(&mut host, upto);
    day.bombard_state = host.bombard.rng_state();
    host.states.ai_day = day;
    ended
}

/// The start of a day up to the day's timers: economy, yards, deliveries,
/// movement, combat, occupation, fog, missions, the story systems, escapes
/// and events. Returns the galaxy the day's loyalty shifts compare against
/// and the regiments that set out since the last day.
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
pub fn day_head(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    tick_events: &[TickEvent],
    rolls: &mut dyn RollSource,
    integrator: &mut PerceptionIntegrator,
    presentation: &mut dyn DayPresentation,
) -> Option<(
    rebellion_core::loyalty::LoyaltyWatch,
    Vec<rebellion_core::blockade::RunningRegiment>,
)> {
    let last_tick_event = tick_events.last()?;
    let current_tick = last_tick_event.tick;

    // `MovementState` is authoritative while a fleet is in hyperspace. Repair
    // stale save/index state before economy and manufacturing inspect orbits.
    reconcile_fleet_orbits(&states.movement, world);


    // The galaxy the day's loyalty shifts compare against (step 13e).
    let loyalty_before =
        rebellion_core::loyalty::watch(world, Some(states.victory.alliance_hq));

    // ── 0. Economy (runs BEFORE manufacturing — affects production) ──────
    let economy_events = EconomySystem::advance_with_uprisings(
        &mut states.economy,
        world,
        tick_events,
        world.difficulty_index,
        &states.uprising,
    );
    let step = DayStep::Economy(&economy_events);
    present(presentation, world, states, &step, StepPhase::Before);
    integrator.apply_economy_events(world, &economy_events);
    present(presentation, world, states, &step, StepPhase::After);

    // ── 1. Manufacturing ─────────────────────────────────────────────────
    // `newly_idle` feeds the K6 EVT_MANUFACTURING_IDLE telemetry for any
    // system whose queue transitioned empty this tick.
    // A manager with no yard left scraps its queue; yards in a system in
    // uprising or under blockade are suspended (FUN_00529dd0, FUN_0050bbb0).
    scrap_yardless_queues(world, &mut states.manufacturing);
    let suspended = suspended_systems(&states.blockade, &states.uprising);
    // Each yard cycle takes one refined material and ends with one build
    // point for its manager (FUN_00530820, FUN_0052a430).
    // An unfinished facility the side change left with no place
    // (FUN_00515480, slot +0xac(2)) leaves its queue.
    rebellion_core::products::destroy_lost(world, &mut states.manufacturing);
    rebellion_core::products::number_new_spares(world);
    rebellion_core::products::reconcile_carrying(world, &states.manufacturing, &|fleet| {
        states.troop_transport.carried_count(fleet)
    });
    let mut mfg_advance = advance_yards(
        &mut states.production,
        world,
        &mut states.manufacturing,
        |system| suspended.contains(&system),
        tick_events,
    );
    // A finished product completes in its container, or travels there
    // from its yard (FUN_0052bee0; rebellion_core::products).
    rebellion_core::products::settle_advance(world, &mut mfg_advance);
    integrator.apply_build_completions(world, &mfg_advance.completions);
    rebellion_core::products::board(world, &mut states.troop_transport, &mfg_advance.completions);
    integrator.apply_manufacturing_idle(world, &mfg_advance.newly_idle);
    for completion in &mfg_advance.completions {
        states.combat_cooldowns.remove(&completion.system);
    }
    let step = DayStep::Manufactured(&mfg_advance);
    present(presentation, world, states, &step, StepPhase::After);
    // Remote products travel, then complete at their destination (F-030).
    // A finished product travelling in its container arrives there
    // (event 0x387; rebellion_core::products::arrive); one whose system is
    // gone goes on as a delivery.
    let arrived = rebellion_core::products::arrive(world, current_tick);
    integrator.apply_build_completions(world, &arrived.completions);
    rebellion_core::products::board(world, &mut states.troop_transport, &arrived.completions);
    for completion in &arrived.completions {
        states.combat_cooldowns.remove(&completion.system);
    }
    let step = DayStep::UnitsArrived(&arrived.completions);
    present(presentation, world, states, &step, StepPhase::After);
    states.deliveries.depart(world, &mfg_advance.departures);
    states.deliveries.depart(world, &arrived.departures);
    let delivered = states.deliveries.advance(world, tick_events);
    integrator.apply_build_completions(world, &delivered.arrivals);
    integrator.apply_deliveries_lost(world, &delivered.lost);
    for completion in &delivered.arrivals {
        states.combat_cooldowns.remove(&completion.system);
    }
    let step = DayStep::UnitsArrived(&delivered.arrivals);
    present(presentation, world, states, &step, StepPhase::After);
    let step = DayStep::Idle(&mfg_advance.newly_idle);
    present(presentation, world, states, &step, StepPhase::After);

    // ── 2. Movement ──────────────────────────────────────────────────────
    let arrivals = MovementSystem::advance(&mut states.movement, tick_events);
    integrator.apply_arrivals(
        world,
        &states.movement,
        &mut states.troop_transport,
        &arrivals,
    );
    for arrival in &arrivals {
        states.combat_cooldowns.remove(&arrival.system);
    }
    present(presentation, world, states, &DayStep::FleetArrivals(&arrivals), StepPhase::After);
    // Regiments travelling on their own (FUN_00556430, event 0x387).
    let regiments = states.troop_transport.advance_transit(world, current_tick);
    integrator.apply_regiment_arrivals(world, &regiments);
    present(presentation, world, states, &DayStep::RegimentsArrived(&regiments), StepPhase::After);
    // Squadrons travelling on their own (squadrons::SquadronTransits).
    let (squadrons_arrived, squadrons_lost) = states
        .troop_transport
        .squadrons
        .advance(world, current_tick);
    let step = DayStep::SquadronsArrived(&squadrons_arrived, &squadrons_lost);
    present(presentation, world, states, &step, StepPhase::After);
    // The headquarters' trip ends (rebellion_core::headquarters_move).
    if let Some(transit) =
        rebellion_core::headquarters_move::advance(&mut states.troop_transport, current_tick)
    {
        let step = DayStep::HeadquartersArrived(&transit);
        present(presentation, world, states, &step, StepPhase::After);
    }

    // ── 3. Combat ────────────────────────────────────────────────────────
    let combat_triggers: Vec<_> = world
        .systems
        .keys()
        .filter(|&sys_key| {
            if let Some(&last_battle) = states.combat_cooldowns.get(&sys_key) {
                if combat_is_on_cooldown(last_battle, current_tick) {
                    return false;
                }
            }
            // FUN_00509710 / FUN_00516d50: both sides present, by a fleet
            // or a squadron on the planet, start a battle.
            let (has_alliance, has_empire) = squadrons::sides_present(world, sys_key);
            has_alliance && has_empire
        })
        .collect();

    for sys_key in combat_triggers {
        let combat_rolls = rolls.take(256);
        let sorties = squadrons::scramble(world, sys_key);
        // A runner may fight the battle itself (the app's tactical battle
        // for the player's side); the day's other battles wait for it.
        if presentation.take_battle(world, states, sys_key, &sorties, combat_rolls[0]) {
            break;
        }
        let battle_before = rebellion_core::loyalty::battle_watch(world, sys_key);
        // FUN_004362b0: each side answers the battle choice. An AI side's
        // strategy gives its retreat order (FUN_00485d50); a side no AI
        // plays gives none.
        let ais: Vec<&AIState> = std::iter::once(&states.ai).chain(&states.ai2).collect();
        let retreat_allowed = rebellion_core::ai_strategy::retreat_orders(
            &ais,
            world,
            &states.movement,
            &states.troop_transport,
            sys_key,
        );
        let space_result = resolve_system_space_combat(
            world,
            sys_key,
            &combat_rolls,
            current_tick,
            retreat_allowed,
        );
        squadrons::recover(world, &sorties);
        // FUN_00517270 -> FUN_00560d00: the winner's loyalty shift for the
        // battle's losses (Alliance attacks, Empire defends).
        let battle_winner = match space_result.winner {
            rebellion_core::combat::CombatSide::Attacker => {
                Some(rebellion_core::dat::Faction::Alliance)
            }
            rebellion_core::combat::CombatSide::Defender => Some(rebellion_core::dat::Faction::Empire),
            rebellion_core::combat::CombatSide::Draw => None,
        };
        let battle_shift = rebellion_core::loyalty::settle_battle(
            world,
            battle_before,
            sys_key,
            battle_winner,
        );
        integrator.record_loyalty_shifts(battle_shift.as_slice());
        // FUN_00516ef0: the loser's fleets and planet squadrons leave on
        // Autorouting.
        let withdrawals = rebellion_core::abstract_battle::withdraw_loser(
            world,
            &mut states.movement,
            &mut states.troop_transport.squadrons,
            sys_key,
            battle_winner,
            current_tick,
        );
        // FUN_0050a0b0 -> FUN_00510b80: the side that withdrew changes, and
        // the garrison takes the system's withdraw percent.
        BlockadeSystem::after_withdrawal(&mut states.blockade, world, sys_key, &withdrawals);

        integrator.emit_system_space_combat(world, &space_result);
        let step = DayStep::BattleFought(&space_result);
        present(presentation, world, states, &step, StepPhase::After);
        states.combat_cooldowns.insert(
            sys_key,
            if space_result.stalemate {
                UNCHANGED_STALEMATE_COOLDOWN
            } else {
                current_tick
            },
        );
        // No bombardment follows a battle: the original bombards only on the
        // Fleet menu's orders 0x220..0x223 (ghidra/notes/bombardment-order.md).
    }

    let destroyed_cargo = states.troop_transport.destroy_untransportable_cargo(world);
    integrator.emit_destroyed_transport_cargo(&destroyed_cargo);

    // ── 3b. Occupation ──────────────────────────────────────────────────
    // Carried regiments stay aboard until an order moves them: nothing in
    // the original lands cargo without one (ghidra/notes/regiment-unload.md,
    // "What happens to the rest of the cargo"). A system's holder follows
    // its regiments as soon as they change (FUN_005109f0 -> FUN_0050b610),
    // so a planet with regiments takes the holder the economy gives it
    // (economy::system_holder): one side's regiments alone take it, two
    // sides' keep the previous side. The original fights on the ground only
    // in a Planetary Assault (ghidra/notes/planetary-assault.md).
    let ground_systems: Vec<_> = world.systems.keys().collect();
    for sys_key in ground_systems {
        let Some(system) = world.systems.get(sys_key) else {
            continue;
        };
        if system.ground_units.is_empty() {
            continue;
        }
        if let ControlKind::Controlled(winner) =
            rebellion_core::economy::system_holder(world, system)
        {
            if system.control != ControlKind::Controlled(winner) {
                integrator.apply_ground_occupation(world, sys_key, winner, current_tick);
                let step = DayStep::Occupied(sys_key, winner);
                present(presentation, world, states, &step, StepPhase::After);
            }
        }
    }

    // Roll the regiments that set out on their own since the last tick
    // (FUN_00504990) before anything else moves them; their rolls happen at
    // the blockade step.
    let running_regiments = if tick_events.is_empty() {
        Vec::new()
    } else {
        BlockadeSystem::running_regiments(&mut states.blockade, world, &states.troop_transport)
    };

    // ── 4. Fog of war ────────────────────────────────────────────────────
    let reveals = FogSystem::advance(&mut states.fog, world, &states.movement);
    integrator.emit_fog_reveals(&reveals, world);
    present(presentation, world, states, &DayStep::FogAdvanced, StepPhase::After);

    run_day_missions(world, states, tick_events, rolls, integrator, presentation);
    Some((loyalty_before, running_regiments))
}

/// The day's missions, the story systems (Dagobah, the Final Battle, the
/// bounty hunters, `SeatOfPower`), the missions' results, the captives'
/// escapes and the fired events. Both the headless runner and the
/// interactive app run it; `presentation` sees each step.
#[expect(
    clippy::too_many_lines,
    reason = "Keep the missions' ordered steps together."
)]
pub fn run_day_missions(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    tick_events: &[TickEvent],
    rolls: &mut dyn RollSource,
    integrator: &mut PerceptionIntegrator,
    presentation: &mut dyn DayPresentation,
) {
    let Some(current_tick) = tick_events.last().map(|event| event.tick) else {
        return;
    };
    // ── 5. Missions ──────────────────────────────────────────────────────
    let mission_rolls = rolls.take(
        states.missions.len() * tick_events.len() * rebellion_core::missions::ROLLS_PER_MISSION,
    );
    let mission_advance = MissionSystem::advance(
        &mut states.missions,
        world,
        &states.uprising,
        &states.research,
        tick_events,
        &mission_rolls,
    );
    let step = DayStep::MissionAdvance(&mission_advance);
    present(presentation, world, states, &step, StepPhase::Before);
    rebellion_core::missions::apply_advance_effects(world, &mission_advance.effects);
    present(presentation, world, states, &step, StepPhase::After);
    integrator.record_advance_character_effects(
        world,
        &mission_advance.effects,
        tick_events.last().map_or(0, |event| event.tick),
    );
    integrator.record_return_orders(
        world,
        &states.missions,
        tick_events.last().map_or(0, |event| event.tick),
    );

    // ── 5b. Dagobah: timer 0x392 and the check FUN_00544030 ──────────────
    let dagobah_rolls = rolls.take(rebellion_core::dagobah::DagobahSystem::roll_budget());
    if let Some(pulse) = rebellion_core::dagobah::DagobahSystem::advance(
        &mut states.dagobah,
        world,
        &states.missions,
        tick_events,
        &dagobah_rolls,
    ) {
        let mission = rebellion_core::dagobah::dispatch(&mut states.missions, world, &pulse);
        integrator.record_luke_dagobah(world, &pulse, mission.is_some());
        if mission.is_some() {
            let step = DayStep::LukeToDagobah(pulse.tick);
            present(presentation, world, states, &step, StepPhase::After);
        }
    }
    let step = DayStep::DagobahCompleted(&mission_advance.effects);
    present(presentation, world, states, &step, StepPhase::After);

    // ── 5c. The Final Battle: FUN_00544da0 .. FUN_00544fc0 ───────────────
    let final_battle_rolls =
        rolls.take(rebellion_core::final_battle::FinalBattleSystem::roll_budget());
    for step in rebellion_core::final_battle::FinalBattleSystem::advance(
        &mut states.final_battle,
        world,
        &states.missions,
        tick_events,
        &final_battle_rolls,
    ) {
        let mission = rebellion_core::final_battle::perform(&step, &mut states.missions, world);
        integrator.record_final_battle_step(world, &step, mission.is_some());
        let shown = DayStep::FinalBattle(&step);
        present(presentation, world, states, &shown, StepPhase::After);
    }

    // ── 5d. The bounty hunters: timer 0x393 (FUN_00546fa0) ───────────────
    let jabba_rolls = rolls.take(rebellion_core::jabba::JabbaSystem::roll_budget());
    for step in rebellion_core::jabba::JabbaSystem::advance(
        &mut states.jabba,
        world,
        &states.missions,
        tick_events,
        &jabba_rolls,
    ) {
        let mission = rebellion_core::jabba::perform(&step, &mut states.missions, world);
        integrator.record_jabba_step(world, &step, mission.is_some());
        let shown = DayStep::Jabba(&step, mission.is_some());
        present(presentation, world, states, &shown, StepPhase::After);
    }

    // ── 5e. `SeatOfPower` (FUN_005617b0, notification 0x230) ───────────────
    if let Some(held) =
        rebellion_core::seat_of_power::SeatOfPowerSystem::advance(&mut states.seat_of_power, world)
    {
        if let Some(tick) = tick_events.last().map(|event| event.tick) {
            integrator.record_seat_of_power(held, tick);
        }
        present(presentation, world, states, &DayStep::SeatOfPower(held), StepPhase::After);
    }
    states.ai.free_mission_members(&mission_advance.effects);
    if let Some(ref mut ai2) = states.ai2 {
        ai2.free_mission_members(&mission_advance.effects);
    }
    states.ai.note_mission_results(&mission_advance.results);
    if let Some(ai2) = states.ai2.as_mut() {
        ai2.note_mission_results(&mission_advance.results);
    }
    for result in &mission_advance.results {
        // FUN_00592c80: a mission's end gives its side level 6 of its
        // target system (Event319_Handle).
        let continuing = states
            .missions
            .missions()
            .iter()
            .any(|mission| mission.id == result.mission_id && mission.end_code == 0);
        present(
            presentation,
            world,
            states,
            &DayStep::MissionResult {
                result,
                continuing,
                ended: &[],
            },
            StepPhase::Before,
        );
        let ended = integrator.apply_mission_result(
            world,
            result,
            &mut states.uprising,
            &mut states.research,
        );
        present(
            presentation,
            world,
            states,
            &DayStep::MissionResult {
                result,
                continuing,
                ended: &ended,
            },
            StepPhase::After,
        );
        if !continuing {
            world
                .object_views
                .queue_push(result.faction.into(), result.target_system);
        }
        // Knesset Shamash-Bet #R11: emit `EVT_CHARACTER_KILLED` telemetry for
        // mission-side assassinations. The integrator's `MissionEffect::CharacterKilled`
        // arm marks `is_killed = true` via `mark_killed()` instead of deleting
        // from the arena, so the lookup below ALWAYS succeeds under correct
        // invariants — the `<unknown>` fallback is a real invariant break and
        // panics in debug builds.
        for effect in &result.effects {
            if let rebellion_core::missions::MissionEffect::CharacterKilled { character, .. } =
                effect
            {
                debug_assert!(
                    world.characters.contains_key(*character),
                    "EVT_CHARACTER_KILLED target missing from arena — R11 invariant break"
                );
                let (name, dat_id) = world.characters.get(*character).map_or_else(
                    || (String::from("<unknown>"), 0),
                    |c| (c.name.clone(), c.dat_id.raw()),
                );
                integrator.emit(
                    rebellion_core::game_events::SYS_MISSIONS,
                    rebellion_core::game_events::EVT_CHARACTER_KILLED,
                    serde_json::json!({
                        "name": name,
                        "dat_id": dat_id,
                        "cause": "assassination",
                    }),
                );
            }
        }

        // Knesset Shamash-Bet #R6/#R7/#R8: mission-side notification events.
        //
        // These are state-transition telemetry markers on the outcome of the
        // mission, NOT `Random`-gated EventSystem entries. The CI guard
        // `notification_events_never_use_random` protects the corresponding
        // story-event IDs from being misused.
        match result.outcome {
            rebellion_core::missions::MissionOutcome::Success => {
                // #R6: Successful espionage yields informant intel.
                if matches!(
                    result.kind,
                    rebellion_core::missions::MissionKind::Espionage
                ) {
                    let char_name = result
                        .character
                        .and_then(|key| world.characters.get(key))
                        .map_or_else(|| String::from("<unknown>"), |c| c.name.clone());
                    let sys_name = world
                        .systems
                        .get(result.target_system)
                        .map_or_else(|| String::from("<unknown>"), |s| s.name.clone());
                    integrator.emit(
                        rebellion_core::game_events::SYS_MISSIONS,
                        rebellion_core::game_events::EVT_INFORMANT_INTEL,
                        serde_json::json!({
                            "character": char_name,
                            "system": sys_name,
                        }),
                    );
                }
            }
            rebellion_core::missions::MissionOutcome::Failure => {
                // #R8: Mission failure on combat/assassination → character health hit.
                if matches!(
                    result.kind,
                    rebellion_core::missions::MissionKind::Assassination
                        | rebellion_core::missions::MissionKind::Abduction
                        | rebellion_core::missions::MissionKind::Rescue
                ) {
                    let char_name = result
                        .character
                        .and_then(|key| world.characters.get(key))
                        .map_or_else(|| String::from("<unknown>"), |c| c.name.clone());
                    integrator.emit(
                        rebellion_core::game_events::SYS_MISSIONS,
                        rebellion_core::game_events::EVT_CHARACTER_HEALTH,
                        serde_json::json!({
                            "character": char_name,
                            "mission_kind": format!("{:?}", result.kind),
                        }),
                    );
                }
            }
        }
    }
    // Destroyed 0x302 after the reports and their
    // effects (FUN_0047ae50).
    states.ai.note_missions_ended(&mission_advance.ended, world);
    if let Some(ai2) = states.ai2.as_mut() {
        ai2.note_missions_ended(&mission_advance.ended, world);
    }

    // ── 5b. Character escapes ────────────────────────────────────────────
    // Each captive draws at most an attempt and its next delay.
    let escape_rolls = rolls.take(2 * world.characters.len());
    let escape_effects =
        MissionSystem::advance_escapes(&mut states.missions, world, current_tick, &escape_rolls);
    let step = DayStep::Escapes(&escape_effects);
    present(presentation, world, states, &step, StepPhase::Before);
    integrator.apply_escape_effects(world, &escape_effects);

    // ── 6. Events ────────────────────────────────────────────────────────
    let event_rolls: Vec<f32> = rolls.take(16).iter().map(|&r| r as f32).collect();
    let fired_events = EventSystem::advance(&mut states.events, world, tick_events, &event_rolls);
    // One event at a time, so each one's story effects (its messages and
    // special forces) are shown with it.
    for fired in &fired_events {
        integrator.apply_fired_events(
            world,
            std::slice::from_ref(fired),
            current_tick,
            &states.movement,
        );
        let effects = integrator.drain_story_effects();
        let step = DayStep::EventFired(fired, &effects);
        present(presentation, world, states, &step, StepPhase::After);
    }

}

/// The start of a day: every system up to and including the day's timers,
/// then its AI day begins ([`AiDay::begin`]). With more than one day in
/// `tick_events`, the earlier days' passes run here; the last day's are
/// left to [`run_ai_passes`], and its end to [`close_simulation_day`].
pub fn begin_simulation_day(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    tick_events: &[TickEvent],
    rolls: &[f64],
    integrator: &mut PerceptionIntegrator,
) {
    begin_simulation_day_with(
        world,
        states,
        tick_events,
        rolls,
        integrator,
        &mut NoPresentation,
    );
}

/// [`begin_simulation_day`] shown through `presentation`: the interactive
/// app's start of the day, from the same day's rolls as the headless
/// runner's, so the two give the same campaign (F-012). The place reached
/// in `rolls` is kept for [`close_simulation_day`]
/// (`AiDay::roll_cursor`).
pub fn begin_simulation_day_with(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    tick_events: &[TickEvent],
    rolls: &[f64],
    integrator: &mut PerceptionIntegrator,
    presentation: &mut dyn DayPresentation,
) {
    let mut day_rolls = SliceRolls::at(rolls, 0);
    let Some(loyalty_before) = begin_day(
        world,
        states,
        tick_events,
        &mut day_rolls,
        integrator,
        presentation,
    ) else {
        return;
    };
    start_ai_day(
        world,
        states,
        tick_events,
        loyalty_before,
        &mut day_rolls,
        integrator,
        presentation,
    );
    states.ai_day.roll_cursor = u32::try_from(day_rolls.cursor()).unwrap_or(u32::MAX);
}

/// The day's systems ahead of its AI: [`day_head`] (economy to events,
/// with [`run_day_missions`]) and [`run_day_timers`]. Both the headless
/// runner and the interactive app run it (F-012); `presentation` sees each
/// step. Returns the galaxy the day's loyalty shifts compare against, for
/// [`start_ai_day`]; `None` without a day.
pub fn begin_day(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    tick_events: &[TickEvent],
    rolls: &mut dyn RollSource,
    integrator: &mut PerceptionIntegrator,
    presentation: &mut dyn DayPresentation,
) -> Option<rebellion_core::loyalty::LoyaltyWatch> {
    let (loyalty_before, running) =
        day_head(world, states, tick_events, rolls, integrator, presentation)?;
    run_day_timers(
        world,
        states,
        tick_events,
        &running,
        rolls,
        integrator,
        presentation,
    );
    Some(loyalty_before)
}

/// The day's timers, which fire in its first sub-tick ahead of its first
/// AI pass (`FUN_0041dff0` phase 5 before phase 8;
/// `ghidra/notes/ai-scheduling.md`): the dawn's blockades, uprisings and
/// disasters, informants, healing, Force discovery, repair, mines and
/// refineries, autoscrap and the yards' research; then `running`, the
/// regiments that set out since the last day, run their blockades. Both
/// the headless runner and the interactive app run it; `presentation`
/// sees each step's events before and after they are applied.
#[expect(
    clippy::too_many_lines,
    reason = "Keep the day's ordered timers together."
)]
pub fn run_day_timers(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    tick_events: &[TickEvent],
    running: &[rebellion_core::blockade::RunningRegiment],
    rolls: &mut dyn RollSource,
    integrator: &mut PerceptionIntegrator,
    presentation: &mut dyn DayPresentation,
) {
    let Some(current_tick) = tick_events.last().map(|event| event.tick) else {
        return;
    };
    // ── 8. Blockade: the fleets the dawn brought, and timer 0x386 ────────
    // Every timer due today runs before the day's first AI tick
    // (FUN_0041dff0 phase 5 ahead of phase 8; ghidra/notes/ai-scheduling.md),
    // and the blockade bit follows the arrivals at once.
    let blockade_events = BlockadeSystem::advance(&mut states.blockade, world, tick_events);
    let step = DayStep::DawnBlockade(&blockade_events);
    present(presentation, world, states, &step, StepPhase::Before);
    integrator.apply_blockade_events(world, &mut states.troop_transport, &blockade_events);
    settle_ended_blockades(world, states, integrator);
    present(presentation, world, states, &step, StepPhase::After);

    // ── 9. Uprising ──────────────────────────────────────────────────────
    let uprising_rolls = rolls.take(UprisingSystem::roll_budget(
        &states.uprising,
        world,
        tick_events.last().map_or(0, |event| event.tick),
    ));
    let uprising_events = UprisingSystem::advance(
        &mut states.uprising,
        world,
        &states.economy,
        &states.missions,
        tick_events,
        &uprising_rolls,
    );
    let step = DayStep::Uprisings(&uprising_events);
    present(presentation, world, states, &step, StepPhase::Before);
    integrator.apply_uprising_events(world, &uprising_events);
    // The disaster's unfinished facilities leave their queues
    // (FUN_00511930 walks mode 3; rebellion_core::products::destroy).
    for event in &uprising_events {
        if let rebellion_core::uprising::UprisingEvent::Disaster { lost_products, .. } = event {
            for &id in lost_products {
                rebellion_core::products::destroy(world, &mut states.manufacturing, id);
            }
        }
    }
    present(presentation, world, states, &step, StepPhase::After);

    // ── 9b. Informants: each held system's timer 0x38e ──────────────────
    let informant_rolls = rolls.take(rebellion_core::informant::InformantSystem::roll_budget(
        world,
    ));
    let informant_reports = rebellion_core::informant::InformantSystem::advance(
        &mut states.informant,
        world,
        tick_events,
        &informant_rolls,
    );
    let step = DayStep::Informants(&informant_reports);
    present(presentation, world, states, &step, StepPhase::Before);
    integrator.apply_informant_reports(world, &mut states.fog, &informant_reports);
    // FUN_0050d510: the incident's side gets its push (Event319_Handle).
    for report in &informant_reports {
        world
            .object_views
            .queue_push(report.recipient, report.system);
    }
    present(presentation, world, states, &step, StepPhase::After);

    // ── 9c. Injury recovery: each healing character's timer 0x388 ────────
    let healed =
        rebellion_core::healing::HealingSystem::advance(&mut states.healing, world, tick_events);
    let step = DayStep::Healed(&healed);
    present(presentation, world, states, &step, StepPhase::Before);
    integrator.apply_healing(world, &healed);
    present(presentation, world, states, &step, StepPhase::After);
    // A capture's hook (slot +0x34c, FUN_004f1a80) clears the captive's
    // command (FUN_004eeed0).
    rebellion_core::character_command::release_unfit(world);

    // ── 9d. Force discovery: each discovering trainer's timer 0x389 ──────
    let discovery_rolls =
        rolls.take(rebellion_core::force_discovery::ForceDiscoverySystem::roll_budget(world));
    let discovered = rebellion_core::force_discovery::ForceDiscoverySystem::advance(
        &mut states.force_discovery,
        world,
        tick_events,
        &discovery_rolls,
    );
    let step = DayStep::ForceDiscovered(&discovered);
    present(presentation, world, states, &step, StepPhase::Before);
    integrator.apply_force_discoveries(world, &discovered);
    present(presentation, world, states, &step, StepPhase::After);

    // ── 12b. Ship repair ────────────────────────────────────────────────
    let repair_events = RepairSystem::advance(&mut states.repair, world, tick_events);
    let step = DayStep::Repairs(&repair_events);
    present(presentation, world, states, &step, StepPhase::Before);
    integrator.apply_repair_events(world, &repair_events);
    present(presentation, world, states, &step, StepPhase::After);

    // ── 13b. Mines, refineries and the side stockpiles ───────────────────
    // Timer 0x394, before the AI (ghidra/notes/ai-scheduling.md).
    let production_rolls = rolls.take(world.production_facilities.len());
    ProductionSystem::advance(
        &mut states.production,
        world,
        &states.manufacturing,
        &states.deliveries,
        |system| states.blockade.is_blockaded(system) || states.uprising.is_uprising(system),
        |system| rebellion_core::economy::production_modifier_at(&states.economy, system),
        tick_events,
        &production_rolls,
    );

    // ── 13c. Maintenance shortfall: each side's autoscrap timer 0x382 ───
    let autoscrap_rolls = rolls.take(2);
    let autoscraps = rebellion_core::autoscrap::AutoscrapSystem::advance(
        &mut states.autoscrap,
        world,
        &states.troop_transport,
        &states.manufacturing,
        &states.deliveries,
        |system| states.blockade.is_blockaded(system) || states.uprising.is_uprising(system),
        tick_events,
        &autoscrap_rolls,
    );
    let step = DayStep::Autoscraps(&autoscraps);
    present(presentation, world, states, &step, StepPhase::Before);
    integrator.apply_autoscraps(
        world,
        &mut states.production,
        &mut states.troop_transport,
        &autoscraps,
    );
    present(presentation, world, states, &step, StepPhase::After);

    // ── 13d. The yards' own research: each side's timer 0x381 ───────────
    let tally_rolls = rolls.take(2);
    present(presentation, world, states, &DayStep::ResearchTally(&[]), StepPhase::Before);
    let tallies = rebellion_core::research::ResearchSystem::advance_tally(
        &mut states.research,
        world,
        |system| states.blockade.is_blockaded(system) || states.uprising.is_uprising(system),
        tick_events,
        &tally_rolls,
    );
    integrator.record_research_tallies(&tallies);
    present(presentation, world, states, &DayStep::ResearchTally(&tallies), StepPhase::After);

    // ── 8b. The regiments that set out since the last day run a blockade
    // (FUN_00504990). port: their rolls come before the day's AI passes, so
    // the passes, which a host may spread over the day's sub-ticks, leave no
    // roll owed across them.
    let running_rolls = rolls.take(running.len());
    let running = BlockadeSystem::resolve_running(running, &running_rolls, current_tick);
    present(presentation, world, states, &DayStep::RunningRegiments(&running), StepPhase::Before);
    integrator.apply_blockade_events(world, &mut states.troop_transport, &running);

}

/// The day's AI begins (`ai_schedule::AiDay::begin`) for each of
/// `tick_events`: each side's order count clears (`FUN_00485de0`) and new
/// capital ships take their ids. With more than one day, the earlier
/// days' passes run here; the last day's are left to [`run_ai_passes`],
/// and its end to [`close_simulation_day`].
pub fn start_ai_day(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    tick_events: &[TickEvent],
    loyalty_before: rebellion_core::loyalty::LoyaltyWatch,
    rolls: &mut dyn RollSource,
    integrator: &mut PerceptionIntegrator,
    presentation: &mut dyn DayPresentation,
) {
    // ── 7. AI ────────────────────────────────────────────────────────────
    // Each day has as many passes as the speed has sub-ticks, one side then
    // the other (rebellion_core::ai_schedule); every timer above fired in the
    // day's first sub-tick, ahead of its first pass
    // (ghidra/notes/ai-scheduling.md). The orders' bombardments draw from a
    // stream seeded by one roll a call.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "A roll in 0..1 scaled to 53 bits of seed."
    )]
    let bombard_seed = (rolls.take(1)[0] * 9_007_199_254_740_992.0) as u64;
    let days = u32::try_from(tick_events.len()).unwrap_or(u32::MAX);
    let passes = rebellion_core::ai::passes_per_day(states.clock.speed);
    let mut bombard_state = bombard_seed;
    let mut ended = false;
    for (index, event) in tick_events.iter().enumerate() {
        states.ai_day = AiDay::begin(event.tick, passes, bombard_state, loyalty_before, days);
        presentation.ai_day_begun(event.tick, states.clock.speed, passes);
        if ended {
            // A win held the clock: no later day's pass runs.
            states.ai_day.done = passes;
            continue;
        }
        begin_ai_day(world, states);
        if index + 1 < tick_events.len() {
            ended = run_ai_passes(world, states, integrator, presentation, u32::MAX);
            bombard_state = states.ai_day.bombard_state;
        }
    }
}


/// Where a day's rolls come from: uniform values in `0..1`, taken in the
/// order the day's systems ask for them.
pub trait RollSource {
    /// The next `n` rolls.
    fn take(&mut self, n: usize) -> Vec<f64>;
}

/// The headless runner's rolls: a slice generated for the day, padded with
/// 1.0 (which fires no probability check) once it runs out.
pub struct SliceRolls<'a> {
    rolls: &'a [f64],
    cursor: usize,
}

impl<'a> SliceRolls<'a> {
    /// The rolls of `rolls` from `cursor` on.
    #[must_use]
    pub fn at(rolls: &'a [f64], cursor: usize) -> Self {
        Self {
            rolls,
            cursor: cursor.min(rolls.len()),
        }
    }

    /// How many rolls have been taken.
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }
}

impl RollSource for SliceRolls<'_> {
    fn take(&mut self, n: usize) -> Vec<f64> {
        let end = (self.cursor + n).min(self.rolls.len());
        let mut v: Vec<f64> = self.rolls[self.cursor..end].to_vec();
        self.cursor = end;
        v.resize(n, 1.0);
        v
    }
}

/// The interactive game's rolls: drawn from its generator as asked.
pub struct RngRolls<'a, R: rand::Rng>(pub &'a mut R);

impl<R: rand::Rng> RollSource for RngRolls<'_, R> {
    fn take(&mut self, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.0.gen::<f64>()).collect()
    }
}

/// What a runner shows of the day's systems. The shared day functions call
/// it with each step's events **before** they are applied, so the world it
/// reads is the one they change; the headless runner shows nothing
/// ([`NoPresentation`]). The interactive app turns them into its messages,
/// movies and sounds.
pub trait DayPresentation {
    /// The player's agent (`FUN_00439a10`) runs once for each of `days`
    /// days, after the day's AI passes. The headless runner has no player.
    fn player_agent(&mut self, _world: &GameWorld, _states: &mut SimulationStates, _days: u32) {}

    /// A step of the day's start: `Before` with its events not yet applied
    /// (the world as they found it), `After` once they are.
    fn step(
        &mut self,
        _world: &GameWorld,
        _states: &SimulationStates,
        _step: &DayStep<'_>,
        _phase: StepPhase,
    ) {
    }

    /// Whether this runner fights the battle at `system` itself (the app's
    /// tactical battle). When it does, the day's remaining battles wait;
    /// otherwise the abstract battle resolves it. `roll` is the battle's
    /// first roll. The headless runner fights none.
    fn take_battle(
        &mut self,
        _world: &mut GameWorld,
        _states: &mut SimulationStates,
        _system: SystemKey,
        _sorties: &[rebellion_core::squadrons::Sortie],
        _roll: f64,
    ) -> bool {
        false
    }

    /// The AI's day `day` begins with `passes` passes at `speed`.
    fn ai_day_begun(&mut self, _day: u64, _speed: rebellion_core::tick::GameSpeed, _passes: u32) {}

    /// One side's orders of a pass, applied (`applied` says which took),
    /// with the bombardments and assaults they carried out.
    fn ai_actions(&mut self, _world: &GameWorld, _orders: &AiOrders<'_>) {}

    /// A pass that applied orders settles: its bombardments' and assaults'
    /// reports, before the planets a Death Star destroyed go.
    fn pass_settling(
        &mut self,
        _world: &GameWorld,
        _bombard: &rebellion_core::bombardment::BombardEnv,
        _day: u64,
    ) {
    }

    /// The planets a pass's Death Star destroyed, cleaned up.
    fn planets_destroyed(
        &mut self,
        _world: &GameWorld,
        _destroyed: &[rebellion_core::death_star::DeathStarEvent],
    ) {
    }

    /// A pass's blockades have followed its orders.
    fn pass_settled(&mut self) {}

    /// Blockade events about to be applied.
    fn blockade(&mut self, _world: &GameWorld, _events: &[rebellion_core::blockade::BlockadeEvent]) {
    }

    /// The victory conditions word changed to `new` (`0x128`).
    fn victory_word(&mut self, _world: &GameWorld, _new: u32, _day: u64) {}

    /// The game was decided (`victory` already holds it).
    fn game_decided(
        &mut self,
        _world: &GameWorld,
        _outcome: &rebellion_core::victory::VictoryOutcome,
        _victory: &VictoryState,
        _day: u64,
    ) {
    }
}

/// One side's orders of an AI pass, as [`DayPresentation::ai_actions`]
/// sees them.
pub struct AiOrders<'a> {
    pub side: AiFaction,
    pub actions: &'a [rebellion_core::ai::AIAction],
    pub applied: &'a [bool],
    pub bombardments: &'a [(
        rebellion_core::bombardment::BombardRequest,
        rebellion_core::bombardment::BombardmentOutcome,
    )],
    pub assaults: &'a [(
        rebellion_core::assault::AssaultRequest,
        rebellion_core::assault::AssaultOutcome,
    )],
    pub day: u64,
}

/// When [`DayPresentation::step`] is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepPhase {
    /// Before the step's events are applied.
    Before,
    /// After they are.
    After,
}

/// A step of the day's start, with its events.
#[derive(Debug, Clone, Copy)]
pub enum DayStep<'a> {
    /// The fleets the dawn brought and timer `0x386`.
    DawnBlockade(&'a [rebellion_core::blockade::BlockadeEvent]),
    /// Uprisings, disasters, incidents and resource changes
    /// (`0x38d`, `0x38f`, `0x390`, `0x384`).
    Uprisings(&'a [rebellion_core::uprising::UprisingEvent]),
    /// Informant reports (`0x38e`).
    Informants(&'a [rebellion_core::informant::InformantReport]),
    /// Healed characters (`0x388`).
    Healed(&'a [rebellion_core::healing::Healed]),
    /// Characters made Force aware (`0x389`).
    ForceDiscovered(&'a [rebellion_core::force_discovery::ForceDiscovered]),
    /// Repairs (`0x380`).
    Repairs(&'a [rebellion_core::repair::RepairEvent]),
    /// The maintenance shortfall's scrapped units (`0x382`).
    Autoscraps(&'a [rebellion_core::autoscrap::Autoscrap]),
    /// The yards' research tally (`0x381`): `Before` comes ahead of the
    /// tally itself, with no tallies.
    ResearchTally(&'a [rebellion_core::research::ResearchTally]),
    /// The regiments that set out since the last day, running a blockade.
    RunningRegiments(&'a [rebellion_core::blockade::BlockadeEvent]),
    /// The economy's events (control, smuggling, troop surplus...).
    Economy(&'a [rebellion_core::economy::EconomyEvent]),
    /// The yards' day: completions, departures and trips.
    Manufactured(&'a rebellion_core::manufacturing::ManufacturingAdvance),
    /// Products and deliveries completed at their destination.
    UnitsArrived(&'a [rebellion_core::manufacturing::CompletionEvent]),
    /// Production areas whose queues just emptied (`0x160`).
    Idle(&'a [(SystemKey, rebellion_core::manufacturing::ProductionArea)]),
    /// Fleets that arrived.
    FleetArrivals(&'a [rebellion_core::movement::ArrivalEvent]),
    /// Regiments travelling on their own that arrived or were lost.
    RegimentsArrived(&'a rebellion_core::troop_transport::RegimentArrivals),
    /// Squadrons travelling on their own that arrived, and those lost.
    SquadronsArrived(
        &'a [rebellion_core::squadrons::SquadronTransit],
        &'a [rebellion_core::squadrons::SquadronTransit],
    ),
    /// The headquarters' trip ended.
    HeadquartersArrived(&'a rebellion_core::headquarters_move::HeadquartersTransit),
    /// An abstract battle was fought.
    BattleFought(&'a crate::integrator::SystemSpaceCombatResult),
    /// One side's regiments took the system.
    Occupied(SystemKey, rebellion_core::dat::Faction),
    /// The runner's fog advanced.
    FogAdvanced,
    /// The missions' day: member moves, releases, arrivals and detections.
    MissionAdvance(&'a rebellion_core::missions::MissionAdvance),
    /// Luke sent to Dagobah (`FUN_00544030`), on this day.
    LukeToDagobah(u64),
    /// The missions' effects, for the Dagobah missions that completed.
    DagobahCompleted(&'a [rebellion_core::missions::MissionEffect]),
    /// A Final Battle step, performed.
    FinalBattle(&'a rebellion_core::final_battle::FinalBattleStep),
    /// A bounty hunters' step, performed, and whether it sent a mission.
    Jabba(&'a rebellion_core::jabba::JabbaStep, bool),
    /// `SeatOfPower` changed to this.
    SeatOfPower(bool),
    /// A mission's result: whether the mission goes on, and (`After`) the
    /// uprisings its effects ended.
    MissionResult {
        result: &'a rebellion_core::missions::MissionResult,
        continuing: bool,
        ended: &'a [rebellion_core::uprising::UprisingEvent],
    },
    /// The captives' escapes and their trips.
    Escapes(&'a [rebellion_core::missions::MissionEffect]),
    /// A story or notification event fired, with its story effects.
    EventFired(
        &'a rebellion_core::events::FiredEvent,
        &'a [rebellion_core::effects::GameEffect],
    ),
}

fn present(
    presentation: &mut dyn DayPresentation,
    world: &GameWorld,
    states: &SimulationStates,
    step: &DayStep<'_>,
    phase: StepPhase,
) {
    presentation.step(world, states, step, phase);
}

/// A runner that shows nothing: the headless playtest and replays.
pub struct NoPresentation;

impl DayPresentation for NoPresentation {}

/// The end of a day whose AI passes have all run: the player's agent, the
/// regiments the passes sent through a blockade, the flagships' names,
/// loyalty, the products, each side's views, and the victory check. Does
/// nothing while passes are still to run or once the day is closed. Both
/// the headless runner and the interactive app run it
/// (`ghidra/notes/ai-scheduling.md`).
pub fn close_simulation_day(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    rolls: &mut dyn RollSource,
    integrator: &mut PerceptionIntegrator,
    presentation: &mut dyn DayPresentation,
) {
    let Some((loyalty_before, days)) = states.ai_day.close() else {
        return;
    };
    let current_tick = states.ai_day.day;
    let tick_events = [TickEvent { tick: current_tick }];

    // The player's agent: its running modules' Destination and build
    // orders, once a day (FUN_00439a10).
    presentation.player_agent(world, states, days);

    // ── 8b. Regiments the day's orders sent running a blockade ───────────
    let running_regiments =
        BlockadeSystem::running_regiments(&mut states.blockade, world, &states.troop_transport);
    let running_rolls = rolls.take(running_regiments.len());
    let running = BlockadeSystem::resolve_running(&running_regiments, &running_rolls, current_tick);
    presentation.blockade(world, &running);
    integrator.apply_blockade_events(world, &mut states.troop_transport, &running);

    // ── 10. (The port's 50-day defection is gone: the original's only
    // betrayal is the mission phase; ghidra/notes/betrayal-and-loyalty.md.)

    // ── 11. Death Star: each pass's bombardments, assaults and destroyed
    // planets settle after it (settle_after_pass).

    // ── 12. Research ─────────────────────────────────────────────────────
    // Research missions add their points when they resolve (step 5).

    // port: a fleet that gained its side's flagship class this tick, by a
    // build or a join on arrival, takes its signature name (fleet_name_bank).
    world.name_flagship_fleets();

    // ── 13e. Loyalty: Coruscant taken, the Alliance HQ lost ─────────────
    // (FUN_005616a0, FUN_00561780; ghidra/notes/betrayal-and-loyalty.md)
    let shifts = rebellion_core::loyalty::settle(
        world,
        loyalty_before,
        Some(states.victory.alliance_hq),
        &mut |m| draw_from_roll(rolls.take(1)[0], m),
    );
    integrator.record_loyalty_shifts(&shifts);

    // The day's orders placed their units in their containers
    // (FUN_0052a140; rebellion_core::products).
    rebellion_core::products::reconcile_carrying(world, &states.manufacturing, &|fleet| {
        states.troop_transport.carried_count(fleet)
    });

    // ── 13f. Each side's views of the other side's objects ──────────────
    // After movement, control and missions (FUN_004f9a60 per object;
    // ghidra/notes/per-side-object-views.md).
    // The fleet, mission and traveller copies first: they read the day
    // before's system copies for who was seen leaving.
    rebellion_core::object_views::advance_extras(
        world,
        &states.movement,
        &states.missions,
        &states.troop_transport,
    );
    rebellion_core::object_views::advance_with_uprisings(
        world,
        &states.troop_transport,
        &states.movement,
        &states.uprising,
    );

    // ── 14. Victory check ────────────────────────────────────────────────
    // The victory conditions word (FUN_005180e0) and its 0x128.
    if let Some((old, new)) = rebellion_core::victory::advance_conditions_word(
        &mut states.victory,
        world,
        states.campaign_config.victory_conditions,
    ) {
        integrator.record_victory_conditions(old, new, current_tick);
        presentation.victory_word(world, new, current_tick);
    }
    integrator.emit_victory_check(&states.victory);
    if let Some(outcome) = VictorySystem::check(
        &states.victory,
        world,
        &tick_events,
        states.campaign_config.victory_conditions,
    ) {
        integrator.apply_victory(&outcome, &mut states.victory, world);
        presentation.game_decided(world, &outcome, &states.victory, current_tick);
    }

    // ── 15. Campaign snapshot (every 250 ticks) ────────────────────────
    if current_tick.is_multiple_of(250) && current_tick > 0 {
        integrator.emit_campaign_snapshot(world, states.movement.len(), &states.economy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_roll_draws_evenly_below_the_bound() {
        assert_eq!(draw_from_roll(0.0, 6), 0);
        assert_eq!(draw_from_roll(0.5, 6), 3);
        assert_eq!(draw_from_roll(0.99, 6), 5);
        assert_eq!(draw_from_roll(1.0, 6), 5);
        assert_eq!(draw_from_roll(0.7, 1), 0);
        assert_eq!(draw_from_roll(0.7, 0), 0);
    }

    #[test]
    fn a_destroy_system_order_s_planet_goes_through_the_death_star_s_destruction() {
        // FUN_0058d990: a Death Star bombarding with 0x223 destroys the
        // system; the tick hands it to step 11 as PlanetDestroyed.
        let mut systems = slotmap::SlotMap::<rebellion_core::ids::SystemKey, ()>::with_key();
        let (a, b) = (systems.insert(()), systems.insert(()));
        let mut env = rebellion_core::bombardment::BombardEnv::new(0, None);
        for (system, destroyed) in [(a, true), (b, false)] {
            env.outcomes.push((
                rebellion_core::bombardment::BombardRequest {
                    system,
                    side: Faction::Empire,
                    kind: rebellion_core::bombardment::DESTROY_SYSTEM,
                    fleets: Vec::new(),
                    alliance_hq: None,
                },
                rebellion_core::bombardment::BombardmentOutcome {
                    system_destroyed: destroyed,
                    ..Default::default()
                },
            ));
        }
        let events = planets_destroyed(&env, 5);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            rebellion_core::death_star::DeathStarEvent::PlanetDestroyed { system, tick: 5 }
                if system == a
        ));
    }
    use rebellion_core::ai::AiFaction;
    use rebellion_core::dat::Faction;
    use rebellion_core::fog::FogState;
    use rebellion_core::game_events::{
        EVT_BOMBARDMENT, EVT_CAPTURE, EVT_COMBAT_GROUND, EVT_CONTROL_CHANGED, EVT_FLEET_ARRIVED,
        EVT_TROOP_MOVED, EVT_VICTORY,
    };
    use rebellion_core::ids::SectorKey;
    use rebellion_core::ids::{DatId, FleetKey, TroopKey};
    use rebellion_core::movement::begin_fleet_transit;
    use rebellion_core::world::{
        CapitalShipClass, Character, ControlKind, Fleet, ShipInstance, System, TroopClassDef,
        TroopUnit, VictoryConditions,
    };

    fn make_test_states() -> (GameWorld, SimulationStates) {
        let mut world = GameWorld::default();
        // Need at least 2 systems for VictoryState
        let s1 = world.systems.insert(rebellion_core::world::System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(1),
            name: "System A".into(),
            sector: SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
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
            espionage_rating: 0.0,
        });
        let s2 = world.systems.insert(rebellion_core::world::System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(2),
            name: "System B".into(),
            sector: SectorKey::default(),
            x: 100,
            y: 100,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
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
            espionage_rating: 0.0,
        });

        let states = SimulationStates {
            ai_day: rebellion_core::ai_schedule::AiDay::default(),
            clock: GameClock::new(),
            manufacturing: ManufacturingState::new(),
            missions: MissionState::new(),
            events: EventState::new(),
            ai: AIState::new(AiFaction::Empire),
            ai2: None,
            movement: MovementState::new(),
            fog: FogState::new(Faction::Alliance),
            blockade: BlockadeState::new(),
            uprising: UprisingState::new(),
            research: ResearchState::new(),
            victory: VictoryState::new(s1, s2),
            economy: EconomyState::default(),
            repair: RepairState::default(),
            production: ProductionState::new(),
            autoscrap: rebellion_core::autoscrap::AutoscrapState::default(),
            informant: rebellion_core::informant::InformantState::default(),
            healing: rebellion_core::healing::HealingState::default(),
            force_discovery: rebellion_core::force_discovery::ForceDiscoveryState::default(),
            dagobah: rebellion_core::dagobah::DagobahState::default(),
            final_battle: rebellion_core::final_battle::FinalBattleState::default(),
            jabba: rebellion_core::jabba::JabbaState::default(),
            seat_of_power: rebellion_core::seat_of_power::SeatOfPowerState::default(),
            troop_transport: TroopTransportState::default(),
            deliveries: DeliveryState::default(),
            combat_cooldowns: HashMap::new(),
            campaign_config: CampaignConfig::default(),
        };
        (world, states)
    }

    // FUN_0051b990 (setup stage 0x18): each side takes a full copy of every
    // core system it does not hold, and the headless runners take it as the
    // app does (ghidra/notes/fog-of-war.md).
    #[test]
    fn a_headless_campaign_starts_with_each_sides_setup_copies() {
        let (mut world, states) = make_test_states();
        let core = world.sectors.insert(rebellion_core::world::Sector {
            dat_id: DatId::new(0x8000_0001),
            name: "Core".into(),
            group: rebellion_core::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: world.systems.keys().collect(),
        });
        let keys: Vec<_> = world.systems.keys().collect();
        for key in &keys {
            world.systems[*key].sector = core;
        }
        world.systems[keys[0]].control = ControlKind::Controlled(Faction::Empire);
        world.systems[keys[0]].total_energy = 7;
        start_views(&mut world, &states);
        let id = world.systems[keys[0]].dat_id;
        let copy = world
            .object_views
            .system(Faction::Alliance, id)
            .expect("the Alliance's copy of the Empire's core system");
        assert!(!copy.live);
        assert_eq!(copy.total_energy, 7);
        assert!(rebellion_core::object_views::side_explored(
            &world,
            Faction::Alliance,
            keys[0]
        ));
        for key in &keys {
            let id = world.systems[*key].dat_id;
            assert!(world.object_views.system(Faction::Empire, id).is_some());
        }
    }

    /// An Alliance fleet with room for one regiment and one Alliance
    /// regiment on the surface, both at the first system, which the Alliance
    /// holds.
    fn world_with_loading_fleet() -> (GameWorld, SimulationStates, FleetKey, TroopKey) {
        let (mut world, states) = make_test_states();
        let system = world.systems.keys().next().unwrap();
        world.systems[system].control = ControlKind::Controlled(Faction::Alliance);
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            name: "Troop Transport".into(),
            is_alliance: true,
            hull: 100,
            troop_capacity: 1,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![ShipInstance::new(class, 100, true)],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[system].fleets.push(fleet);
        let troop = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(20),
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(troop);
        (world, states, fleet, troop)
    }

    fn run_one_tick(world: &mut GameWorld, states: &mut SimulationStates) {
        run_simulation_tick(world, states, &[TickEvent { tick: 1 }], &[0.99; 2048], 10);
    }

    #[test]
    fn a_ship_autoscrapped_at_dawn_is_gone_from_the_ai_s_first_pass() {
        // Every timer due on a day fires in FUN_0041dff0 phase 5 of the
        // day's first sub-tick, before that sub-tick's AI tick (phase 8);
        // timer 0x382 (FUN_00530350) scraps a maintained object while the
        // side is over its limit (ghidra/notes/ai-scheduling.md). The AI's
        // refresh (FUN_00418500) then has no record for the gone ship, and
        // a hold on it lapses. The Alliance has no AI here, so only the
        // timer can scrap its ship; the Empire's AI watches every fleet.
        let held_after_a_day = |scrap: bool| {
            let (mut world, mut states, fleet, troop) = world_with_loading_fleet();
            world.troops.remove(troop);
            let system = world.systems.keys().next().unwrap();
            world.systems[system].ground_units.clear();
            let class = world.fleets[fleet].capital_ships[0].class;
            world.capital_ship_classes[class].maintenance_cost = 1;
            world.number_ships();
            let ship = world.fleets[fleet].capital_ships[0].id;
            assert_eq!(states.ai.faction, Some(AiFaction::Empire));
            states.ai.ship_holds.insert(
                ship,
                rebellion_core::ai_records::ShipHold {
                    flags: 0,
                    owner: 0,
                    plan: 0,
                },
            );
            // Over its limit, the Alliance's timer is due today, or not yet.
            states.autoscrap.alliance = Some(if scrap { 1 } else { 2 });
            run_one_tick(&mut world, &mut states);
            let alive = world
                .fleets
                .get(fleet)
                .is_some_and(|f| f.capital_ships.iter().any(|s| s.id == ship && s.alive));
            assert_eq!(alive, !scrap);
            states.ai.ship_holds.contains_key(&ship)
        };
        assert!(held_after_a_day(false));
        assert!(!held_after_a_day(true));
    }

    #[test]
    fn after_an_ai_pass_s_orders_the_blockade_follows_the_fleets_without_its_timer() {
        // The blockade bit follows the fleets within the sub-tick, and
        // FUN_00511300 arms timer 0x386 as it starts; FUN_0050cb20 fires
        // only on its day. The pass's bombardment losses count first
        // (System_vtd0) (ghidra/notes/ai-scheduling.md).
        let (mut world, mut states) = make_test_states();
        let system = world.systems.keys().next().unwrap();
        world.systems[system].control = ControlKind::Controlled(Faction::Alliance);
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[system].fleets.push(fleet);
        let mut integrator = PerceptionIntegrator::new(4, 0);
        let mut bombard = rebellion_core::bombardment::BombardEnv::new(0, None);
        assert!(!settle_after_pass(
            &mut world,
            &mut states,
            &mut integrator,
            &mut bombard,
            &mut NoPresentation,
            4
        ));
        assert!(states.blockade.is_blockaded(system));
        assert_eq!(states.blockade.support_timer(system), Some(5));
        // A later pass's bombardment losses count toward the blockade and
        // are spent; on the timer's day a pass shifts no support.
        let losses = rebellion_core::loyalty::SideLosses {
            alliance: 3,
            empire: 0,
        };
        bombard.losses.push((system, losses));
        assert!(!settle_after_pass(
            &mut world,
            &mut states,
            &mut integrator,
            &mut bombard,
            &mut NoPresentation,
            5
        ));
        assert!(bombard.losses.is_empty());
        assert_eq!(
            states.blockade.blockade_losses(system).map(|b| b.losses),
            Some(losses)
        );
        assert_eq!(states.blockade.support_timer(system), Some(5));
        // The fleet leaves: the blockade ends, and its losses settle.
        world.systems[system].fleets.clear();
        world.fleets.remove(fleet);
        settle_after_pass(
            &mut world,
            &mut states,
            &mut integrator,
            &mut bombard,
            &mut NoPresentation,
            5,
        );
        assert!(!states.blockade.is_blockaded(system));
        assert_eq!(states.blockade.blockade_losses(system), None);
    }

    #[test]
    fn a_pass_that_wins_the_game_holds_the_day_s_remaining_passes() {
        // FUN_005180e0 stores the word as the event that changes it is
        // handled, and its game-over bit holds the clock before the next
        // sub-tick (FUN_0041e390 -> FUN_0051de80).
        let (mut world, mut states) = make_test_states();
        states.campaign_config.victory_conditions =
            rebellion_core::world::VictoryConditions::HeadquartersOnly;
        let (alliance_hq, coruscant) = (states.victory.alliance_hq, states.victory.empire_hq);
        world.systems[alliance_hq].is_headquarters = true;
        world.systems[coruscant].dat_id = DatId::new(0x109);
        for index in [0x242, 0x240, 0x281, 0x280] {
            world.characters.insert(rebellion_core::world::Character {
                dat_id: DatId::new(index),
                ..rebellion_core::world::Character::default()
            });
        }
        let mut integrator = PerceptionIntegrator::new(4, 0);
        let mut bombard = rebellion_core::bombardment::BombardEnv::new(0, None);
        assert!(!settle_after_pass(
            &mut world,
            &mut states,
            &mut integrator,
            &mut bombard,
            &mut NoPresentation,
            4
        ));
        world.systems[coruscant].control = ControlKind::Controlled(Faction::Alliance);
        assert!(settle_after_pass(
            &mut world,
            &mut states,
            &mut integrator,
            &mut bombard,
            &mut NoPresentation,
            4
        ));
        assert_ne!(
            states.victory.conditions_word & rebellion_core::victory::WORD_GAME_OVER,
            0
        );
        assert!(integrator
            .finish()
            .iter()
            .any(|e| e.event_type == rebellion_core::game_events::EVT_VICTORY_CONDITIONS));
    }

    /// A presentation that records the steps it is shown.
    #[derive(Default)]
    struct StepLog(Vec<String>);

    impl DayPresentation for StepLog {
        fn step(
            &mut self,
            _world: &GameWorld,
            _states: &SimulationStates,
            step: &DayStep<'_>,
            phase: StepPhase,
        ) {
            let name = format!("{step:?}");
            let name = name.split('(').next().unwrap_or_default().to_string();
            self.0.push(format!("{name} {phase:?}"));
        }
    }

    #[test]
    fn the_day_s_timers_show_each_step_before_and_after_it_is_applied() {
        // The timers' order: every timer due on a day fires in FUN_0041dff0
        // phase 5 of its first sub-tick (ghidra/notes/ai-scheduling.md).
        let (mut world, mut states) = make_test_states();
        let mut log = StepLog::default();
        run_day_timers(
            &mut world,
            &mut states,
            &[TickEvent { tick: 1 }],
            &[],
            &mut SliceRolls::at(&[], 0),
            &mut PerceptionIntegrator::new(1, 0),
            &mut log,
        );
        assert_eq!(
            log.0,
            [
                "DawnBlockade Before",
                "DawnBlockade After",
                "Uprisings Before",
                "Uprisings After",
                "Informants Before",
                "Informants After",
                "Healed Before",
                "Healed After",
                "ForceDiscovered Before",
                "ForceDiscovered After",
                "Repairs Before",
                "Repairs After",
                "Autoscraps Before",
                "Autoscraps After",
                "ResearchTally Before",
                "ResearchTally After",
                "RunningRegiments Before",
            ]
        );
        // No day, no timers.
        let mut none = StepLog::default();
        run_day_timers(
            &mut world,
            &mut states,
            &[],
            &[],
            &mut SliceRolls::at(&[], 0),
            &mut PerceptionIntegrator::new(1, 0),
            &mut none,
        );
        assert!(none.0.is_empty());
    }

    #[test]
    fn the_day_s_missions_show_the_advance_before_and_after_it_is_applied() {
        // port: the shared missions step (F-012); the runner's presentation
        // sees the advance, the Dagobah check and the escapes in order.
        let (mut world, mut states) = make_test_states();
        let mut log = StepLog::default();
        run_day_missions(
            &mut world,
            &mut states,
            &[TickEvent { tick: 1 }],
            &mut SliceRolls::at(&[], 0),
            &mut PerceptionIntegrator::new(1, 0),
            &mut log,
        );
        let shown: Vec<&str> = log
            .0
            .iter()
            .map(String::as_str)
            .filter(|step| !step.starts_with("SeatOfPower") && !step.starts_with("EventFired"))
            .collect();
        assert_eq!(
            shown,
            [
                "MissionAdvance Before",
                "MissionAdvance After",
                "DagobahCompleted After",
                "Escapes Before",
            ]
        );
        let mut none = StepLog::default();
        run_day_missions(
            &mut world,
            &mut states,
            &[],
            &mut SliceRolls::at(&[], 0),
            &mut PerceptionIntegrator::new(1, 0),
            &mut none,
        );
        assert!(none.0.is_empty());
    }

    /// A presentation that takes every battle, as the app takes the
    /// player's.
    #[derive(Default)]
    struct BattleTaker {
        steps: StepLog,
        taken: Vec<SystemKey>,
    }

    impl DayPresentation for BattleTaker {
        fn step(
            &mut self,
            world: &GameWorld,
            states: &SimulationStates,
            step: &DayStep<'_>,
            phase: StepPhase,
        ) {
            self.steps.step(world, states, step, phase);
        }

        fn take_battle(
            &mut self,
            _world: &mut GameWorld,
            _states: &mut SimulationStates,
            system: SystemKey,
            _sorties: &[rebellion_core::squadrons::Sortie],
            _roll: f64,
        ) -> bool {
            self.taken.push(system);
            true
        }
    }

    /// A presentation that records the passes' hooks.
    #[derive(Default)]
    struct PassLog(Vec<String>);

    impl DayPresentation for PassLog {
        fn ai_actions(&mut self, _world: &GameWorld, orders: &AiOrders<'_>) {
            self.0.push(format!("orders {:?} {:?}", orders.side, orders.applied));
        }
        fn pass_settling(
            &mut self,
            _world: &GameWorld,
            _bombard: &rebellion_core::bombardment::BombardEnv,
            _day: u64,
        ) {
            self.0.push("settling".into());
        }
        fn blockade(
            &mut self,
            _world: &GameWorld,
            _events: &[rebellion_core::blockade::BlockadeEvent],
        ) {
            self.0.push("blockade".into());
        }
        fn pass_settled(&mut self) {
            self.0.push("settled".into());
        }
    }

    #[test]
    fn a_pass_shows_each_side_s_applied_orders_then_its_settling() {
        // FUN_004361a0's orders run in a later sub-tick, and the state their
        // events keep up settles before the next (ghidra/notes/ai-scheduling.md).
        let (mut world, mut states) = make_test_states();
        let system = world.systems.keys().next().unwrap();
        let class = world.capital_ship_classes.insert(Default::default());
        let mut fleet = || {
            let key = world.fleets.insert(Fleet {
                location: system,
                capital_ships: vec![rebellion_core::world::ShipInstance::new(class, 100, false)],
                fighters: vec![],
                characters: vec![],
                is_alliance: false,
                has_death_star: false,
                special_forces: Vec::new(),
            });
            world.systems[system].fleets.push(key);
            key
        };
        let (a, b) = (fleet(), fleet());
        let mut integrator = PerceptionIntegrator::new(4, 0);
        let mut log = PassLog::default();
        let mut host = HeadlessPasses {
            world: &mut world,
            states: &mut states,
            integrator: &mut integrator,
            bombard: rebellion_core::bombardment::BombardEnv::new(0, None),
            presentation: &mut log,
        };
        host.apply(
            AiSlot::Primary,
            &[
                rebellion_core::ai::AIAction::JoinFleet { fleet: a, into: b },
                rebellion_core::ai::AIAction::JoinFleet { fleet: a, into: b },
            ],
            4,
        );
        assert!(!host.settle(4));
        assert_eq!(
            log.0,
            [
                "orders Empire [true, false]",
                "settling",
                "blockade",
                "settled"
            ]
        );
    }

    #[test]
    fn the_day_s_head_shows_its_steps_in_the_day_s_order() {
        // Economy before manufacturing (FUN_00508250's state feeds the
        // yards), then movement, fog and the missions (F-012).
        let (mut world, mut states) = make_test_states();
        let mut log = StepLog::default();
        day_head(
            &mut world,
            &mut states,
            &[TickEvent { tick: 1 }],
            &mut SliceRolls::at(&[], 0),
            &mut PerceptionIntegrator::new(1, 0),
            &mut log,
        );
        let head: Vec<&str> = log
            .0
            .iter()
            .map(String::as_str)
            .take_while(|step| !step.starts_with("MissionAdvance"))
            .collect();
        assert_eq!(
            head,
            [
                "Economy Before",
                "Economy After",
                "Manufactured After",
                "UnitsArrived After",
                "UnitsArrived After",
                "Idle After",
                "FleetArrivals After",
                "RegimentsArrived After",
                "SquadronsArrived After",
                "FogAdvanced After",
            ]
        );
    }

    #[test]
    fn a_runner_that_fights_a_battle_itself_leaves_it_unresolved() {
        // port: the app's tactical battle (F-012); the abstract battle
        // (FUN_0040a050 state 8) resolves only the battles a runner leaves.
        let opposed = || {
            let (mut world, states) = make_test_states();
            let system = world.systems.keys().next().unwrap();
            let class = world.capital_ship_classes.insert(Default::default());
            for is_alliance in [true, false] {
                let key = world.fleets.insert(Fleet {
                    location: system,
                    capital_ships: vec![rebellion_core::world::ShipInstance::new(
                        class, 100, false,
                    )],
                    fighters: vec![],
                    characters: vec![],
                    is_alliance,
                    has_death_star: false,
                    special_forces: Vec::new(),
                });
                world.systems[system].fleets.push(key);
            }
            (world, states, system)
        };
        let fought = |presentation: &mut dyn DayPresentation| {
            let (mut world, mut states, system) = opposed();
            day_head(
                &mut world,
                &mut states,
                &[TickEvent { tick: 1 }],
                &mut SliceRolls::at(&[0.5; 2048], 0),
                &mut PerceptionIntegrator::new(1, 0),
                presentation,
            );
            (system, states.combat_cooldowns.contains_key(&system))
        };
        let mut log = StepLog::default();
        let (_, cooled) = fought(&mut log);
        assert!(cooled);
        assert!(log.0.iter().any(|step| step == "BattleFought After"));
        let mut taker = BattleTaker::default();
        let (system, cooled) = fought(&mut taker);
        assert_eq!(taker.taken, [system]);
        assert!(!cooled);
        assert!(!taker.steps.0.iter().any(|step| step == "BattleFought After"));
    }

    #[test]
    fn slice_rolls_go_on_from_their_cursor_and_pad_with_one_once_spent() {
        let rolls = [0.1, 0.2, 0.3];
        let mut source = SliceRolls::at(&rolls, 1);
        assert_eq!(source.take(1), [0.2]);
        assert_eq!(source.cursor(), 2);
        assert_eq!(source.take(3), [0.3, 1.0, 1.0]);
        assert_eq!(source.cursor(), 3);
        assert_eq!(SliceRolls::at(&rolls, 9).cursor(), 3);
    }

    #[test]
    fn the_headless_host_applies_each_side_s_orders_and_tags_the_second_side_s() {
        // FUN_004361a0 ticks both players and their stored orders run in a
        // later sub-tick (FUN_0048aa90; ghidra/notes/ai-scheduling.md); the
        // second AI's telemetry carries its dual-AI mark.
        let (mut world, mut states) = make_test_states();
        states.ai2 = Some(AIState::new(AiFaction::Alliance));
        let system = world.systems.keys().next().unwrap();
        let class = world.capital_ship_classes.insert(Default::default());
        let fleet = |world: &mut GameWorld, is_alliance: bool| {
            let key = world.fleets.insert(Fleet {
                location: system,
                capital_ships: vec![rebellion_core::world::ShipInstance::new(class, 100, false)],
                fighters: vec![],
                characters: vec![],
                is_alliance,
                has_death_star: false,
                special_forces: Vec::new(),
            });
            world.systems[system].fleets.push(key);
            key
        };
        let (empire_a, empire_b) = (fleet(&mut world, false), fleet(&mut world, false));
        let (alliance_a, alliance_b) = (fleet(&mut world, true), fleet(&mut world, true));
        let mut integrator = PerceptionIntegrator::new(4, 0);
        let mut host = HeadlessPasses {
            world: &mut world,
            states: &mut states,
            integrator: &mut integrator,
            bombard: rebellion_core::bombardment::BombardEnv::new(0, None),
            presentation: &mut NoPresentation,
        };
        assert_eq!(host.faction(AiSlot::Primary), Some(AiFaction::Empire));
        assert_eq!(host.faction(AiSlot::Secondary), Some(AiFaction::Alliance));
        host.apply(
            AiSlot::Primary,
            &[rebellion_core::ai::AIAction::JoinFleet {
                fleet: empire_a,
                into: empire_b,
            }],
            4,
        );
        host.apply(
            AiSlot::Secondary,
            &[rebellion_core::ai::AIAction::JoinFleet {
                fleet: alliance_a,
                into: alliance_b,
            }],
            4,
        );
        assert!(!world.fleets.contains_key(empire_a));
        assert!(!world.fleets.contains_key(alliance_a));
        let dual: Vec<bool> = integrator
            .finish()
            .iter()
            .filter(|e| e.event_type == rebellion_core::game_events::EVT_AI_ACTION)
            .map(|e| e.details.get("dual_ai").is_some())
            .collect();
        assert_eq!(dual, [false, true]);
    }

    #[test]
    fn the_headless_host_s_settle_reports_a_won_game() {
        // FUN_005180e0's game-over bit holds the clock before the next
        // sub-tick (FUN_0041e390 -> FUN_0051de80).
        let (mut world, mut states) = make_test_states();
        states.campaign_config.victory_conditions =
            rebellion_core::world::VictoryConditions::HeadquartersOnly;
        let (alliance_hq, coruscant) = (states.victory.alliance_hq, states.victory.empire_hq);
        world.systems[alliance_hq].is_headquarters = true;
        world.systems[coruscant].dat_id = DatId::new(0x109);
        for index in [0x242, 0x240, 0x281, 0x280] {
            world.characters.insert(rebellion_core::world::Character {
                dat_id: DatId::new(index),
                ..rebellion_core::world::Character::default()
            });
        }
        let mut integrator = PerceptionIntegrator::new(4, 0);
        let mut settled = |world: &mut GameWorld, states: &mut SimulationStates| {
            HeadlessPasses {
                world,
                states,
                integrator: &mut integrator,
                bombard: rebellion_core::bombardment::BombardEnv::new(0, None),
                presentation: &mut NoPresentation,
            }
            .settle(4)
        };
        assert!(!settled(&mut world, &mut states));
        world.systems[coruscant].control = ControlKind::Controlled(Faction::Alliance);
        assert!(settled(&mut world, &mut states));
    }

    #[test]
    fn a_day_numbers_the_ships_before_the_ai_passes() {
        // port: GameWorld::number_ships gives the AI's per-ship records
        // their key (FUN_00484230 keeps one record per ship object).
        let (mut world, mut states, fleet, _) = world_with_loading_fleet();
        assert_eq!(world.fleets[fleet].capital_ships[0].id, 0);
        run_one_tick(&mut world, &mut states);
        assert_eq!(world.fleets[fleet].capital_ships[0].id, 1);
    }

    /// GNPRTB 7760 (60, `DAT_006bb4c0`): a side holds a system without
    /// regiments on its support (`FUN_0055a080`); the fixtures give each
    /// system its holder's support.
    fn control_threshold() -> rebellion_core::world::GnprtbParams {
        rebellion_core::world::GnprtbParams::new(vec![rebellion_core::world::GnprtbEntry {
            parameter_id: 7760,
            development: 60,
            alliance_sp_easy: 60,
            alliance_sp_medium: 60,
            alliance_sp_hard: 60,
            empire_sp_easy: 60,
            empire_sp_medium: 60,
            empire_sp_hard: 60,
            multiplayer: 60,
        }])
    }

    #[test]
    fn a_regiment_arriving_takes_its_planet_the_same_day() {
        // FUN_005109f0 -> FUN_0050b610 as the system's regiments change:
        // the holder follows them at once (step 3b, economy::system_holder),
        // before the economy's next run. Here a regiment's own Move reaches
        // an explored, unpopulated, neutral planet (FUN_005073d0).
        let (mut world, mut states, _, troop) = world_with_loading_fleet();
        world.gnprtb = control_threshold();
        let mut keys = world.systems.keys();
        let (home, target) = (keys.next().unwrap(), keys.next().unwrap());
        world.systems[home].popularity_alliance = 0.9;
        world.systems[target].is_populated = false;
        world.systems[target].control = ControlKind::Uncontrolled;
        let moved = states.troop_transport.move_regiment(
            &mut world,
            &states.movement,
            &std::collections::HashSet::new(),
            troop,
            rebellion_core::troop_transport::RegimentTarget::System(target),
            0,
        );
        assert!(moved.is_ok(), "{moved:?}");
        let mut landed_at = None;
        for tick in 1..200 {
            run_simulation_tick(
                &mut world,
                &mut states,
                &[TickEvent { tick }],
                &[0.99; 2048],
                10,
            );
            if world.systems[target].ground_units.contains(&troop) {
                landed_at = Some(tick);
                break;
            }
        }
        assert!(landed_at.is_some());
        assert_eq!(
            world.systems[target].control,
            ControlKind::Controlled(Faction::Alliance)
        );
    }

    #[test]
    fn a_carried_regiment_stays_aboard_at_its_system() {
        // Nothing in the original lands cargo without an order
        // (ghidra/notes/regiment-unload.md). Until 2026-10-17 the port's
        // automatic landing put it down unless a hold kept it aboard.
        let (mut world, mut states, fleet, troop) = world_with_loading_fleet();
        let system = world.fleets[fleet].location;
        states
            .troop_transport
            .embark(&mut world, fleet, &[troop])
            .unwrap();

        run_one_tick(&mut world, &mut states);

        assert_eq!(states.troop_transport.cargo(fleet), [troop]);
        assert!(!world.systems[system].ground_units.contains(&troop));
    }

    /// The first system, Alliance-held, with one yard of `yard_class`
    /// (cycle 2 days, as MANFACSD's advanced yards) and a three-point ship
    /// ordered; the points done on it after `days`, or `None` once
    /// nothing is queued. The Alliance starts with `refined` material, and the
    /// system is in uprising when asked.
    fn ship_points_after(yard_class: u32, refined: i32, uprising: bool, days: u64) -> Option<u32> {
        use rebellion_core::manufacturing::{BuildableKind, ProductionArea, QueueItem};
        use rebellion_core::world::{BuildableClass, ManufacturingFacilityInstance};

        let (mut world, mut states, _, _) = world_with_loading_fleet();
        let system = world.systems.keys().next().unwrap();
        world.buildable_classes.insert(
            DatId::new(yard_class),
            BuildableClass {
                processing_rate: 2,
                ..BuildableClass::default()
            },
        );
        let yard = world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: DatId::new(yard_class),
                side: Faction::Alliance,
                is_shipyard: yard_class >> 24 == 0x28,
            });
        world.systems[system].manufacturing_facilities.push(yard);
        let ship = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        states
            .manufacturing
            .enqueue(system, QueueItem::new(BuildableKind::CapitalShip(ship), 3));
        states
            .production
            .stock_mut(Faction::Alliance)
            .expect("the Alliance keeps a stockpile")
            .refined = refined;
        if uprising {
            states.uprising.active_uprisings.insert(
                system,
                rebellion_core::uprising::ActiveUprising {
                    started_tick: 0,
                    next_incident_tick: Some(u64::MAX),
                    next_support_tick: Some(u64::MAX),
                    side: None,
                },
            );
        }
        for tick in 1..=days {
            run_simulation_tick(
                &mut world,
                &mut states,
                &[TickEvent { tick }],
                &[0.99; 2048],
                10,
            );
        }
        states
            .manufacturing
            .queue(system, ProductionArea::Shipyard)
            .filter(|queue| !queue.is_empty())
            .map(rebellion_core::manufacturing::ProductionQueue::completed_points)
    }

    #[test]
    fn mines_in_a_system_in_revolt_give_no_capacity_and_start_the_autoscrap_timer() {
        // FUN_0050bbb0 suspends the facilities of a system in uprising, so
        // FUN_0052fff0 leaves them out of the capacity; FUN_0052f670 then
        // raises the over-limit flag and FUN_005329f0 arms timer 0x382
        // (GNPRTB 7168; this world's empty table gives the 1-day floor).
        use rebellion_core::world::{BuildableClass, ProductionFacilityInstance};
        let timer_after_a_day = |uprising: bool| {
            let (mut world, mut states, _, troop) = world_with_loading_fleet();
            let system = world.systems.keys().next().unwrap();
            world.buildable_classes.insert(
                world.troops[troop].class_dat_id,
                BuildableClass {
                    maintenance_cost: 1,
                    ..BuildableClass::default()
                },
            );
            for (id, is_mine) in [(0x2c00_0001, true), (0x2d00_0001, false)] {
                let key = world
                    .production_facilities
                    .insert(ProductionFacilityInstance {
                        class_dat_id: DatId::new(id),
                        side: Faction::Alliance,
                        is_mine,
                    });
                world.systems[system].production_facilities.push(key);
            }
            if uprising {
                states.uprising.active_uprisings.insert(
                    system,
                    rebellion_core::uprising::ActiveUprising {
                        started_tick: 0,
                        next_incident_tick: Some(u64::MAX),
                        next_support_tick: Some(u64::MAX),
                        side: Some(Faction::Alliance),
                    },
                );
            }
            run_one_tick(&mut world, &mut states);
            states.autoscrap.alliance
        };
        assert_eq!(timer_after_a_day(false), None);
        assert_eq!(timer_after_a_day(true), Some(2));
    }

    #[test]
    fn a_shipyard_in_a_system_in_revolt_does_no_research() {
        // FUN_00528040 skips a yard with +0x60 bit 0, which FUN_0050bbb0
        // sets in a system in uprising; timer 0x381 (FUN_00530460) adds the
        // rest to the Ship points. This world's empty GNPRTB gives the
        // timer its 1-day floor.
        use rebellion_core::research::TechType;
        use rebellion_core::world::ManufacturingFacilityInstance;
        let ship_points_after_two_days = |uprising: bool| {
            let (mut world, mut states, _, _) = world_with_loading_fleet();
            let system = world.systems.keys().next().unwrap();
            let yard = world
                .manufacturing_facilities
                .insert(ManufacturingFacilityInstance {
                    class_dat_id: DatId::new(0x2800_0001),
                    side: Faction::Alliance,
                    is_shipyard: true,
                });
            world.systems[system].manufacturing_facilities.push(yard);
            if uprising {
                states.uprising.active_uprisings.insert(
                    system,
                    rebellion_core::uprising::ActiveUprising {
                        started_tick: 0,
                        next_incident_tick: Some(u64::MAX),
                        next_support_tick: Some(u64::MAX),
                        side: Some(Faction::Alliance),
                    },
                );
            }
            for tick in 1..=2 {
                run_simulation_tick(
                    &mut world,
                    &mut states,
                    &[TickEvent { tick }],
                    &[0.0; 2048],
                    10,
                );
            }
            states.research.progress(true, TechType::Ship).points
        };
        assert_eq!(ship_points_after_two_days(false), 1);
        assert_eq!(ship_points_after_two_days(true), 0);
    }

    #[test]
    fn a_shipyard_without_refined_material_does_not_advance_its_build() {
        // FUN_00530820: a yard takes one refined material to start a cycle
        // and waits when the side has none.
        assert_eq!(ship_points_after(0x2800_0004, 0, false, 5), Some(0));
        // The control: supplied, its 2-day cycles end on days 2 and 4
        // (FUN_0052cc90), each with a point (FUN_0052a430).
        assert_eq!(ship_points_after(0x2800_0004, 5, false, 5), Some(2));
    }

    #[test]
    fn a_ship_completes_when_its_yard_has_made_its_cost_in_points() {
        // FUN_0052b960: three points for a three-point ship, on day 6.
        assert_eq!(ship_points_after(0x2800_0004, 5, false, 5), Some(2));
        assert_eq!(ship_points_after(0x2800_0004, 5, false, 6), None);
    }

    #[test]
    fn a_shipyard_in_an_uprising_does_not_advance_its_build() {
        // FUN_0050bbb0 suspends a system's yards while it is in uprising
        // (+0x88 bit 2), and FUN_00529dd0 counts no suspended yard.
        assert_eq!(ship_points_after(0x2800_0004, 5, false, 2), Some(1));
        assert_eq!(ship_points_after(0x2800_0004, 5, true, 2), Some(0));
    }

    #[test]
    fn a_ship_queue_with_no_shipyard_is_scrapped() {
        // FUN_00529dd0: a manager with no yard of its area scraps its queue
        // (FUN_0052a2c0). A construction yard is not a shipyard.
        assert_eq!(ship_points_after(0x2800_0004, 5, false, 1), Some(0));
        assert_eq!(ship_points_after(0x2a00_0006, 5, false, 1), None);
    }

    #[test]
    fn a_day_names_a_fleet_that_gained_its_sides_flagship_class() {
        // port: fleet_name_bank's signature names, under Canonical naming.
        let (mut world, mut states, fleet, _) = world_with_loading_fleet();
        world.start_fleet_naming(rebellion_core::world::FleetNaming::Canonical);
        let class = world.fleets[fleet].capital_ships[0].class;
        world.capital_ship_classes[class].dat_id =
            rebellion_core::fleet_name_bank::MON_CALAMARI_CRUISER;

        run_one_tick(&mut world, &mut states);

        assert_eq!(world.fleet_name(fleet), Some("Rebel Command Fleet"));
    }

    #[test]
    fn empty_tick_events_returns_empty() {
        let (mut world, mut states) = make_test_states();

        let result = run_simulation_tick(&mut world, &mut states, &[], &[], 0);
        assert!(result.is_empty());
    }

    #[test]
    fn an_enemy_planet_s_squadrons_meet_an_arriving_fleet_in_battle() {
        // FUN_00509710 / FUN_00516d50: a squadron on the planet makes its side
        // present, so the Alliance fleet at the Empire's planet starts a
        // battle; port: the squadrons fight as a group and land after it.
        let (mut world, mut states, fleet, _) = world_with_loading_fleet();
        let system = world.fleets[fleet].location;
        world.systems[system].control = ControlKind::Controlled(Faction::Empire);
        let tie = world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                is_empire: true,
                squadron_size: 12,
                laser_cannon_fore: 2,
                laser_cannon_attack_strength: 2,
                ..Default::default()
            });
        squadrons::add(&mut world.systems[system].fighters, tie, 3);

        run_one_tick(&mut world, &mut states);

        assert!(states.combat_cooldowns.contains_key(&system));
        assert!(world
            .fleets
            .iter()
            .all(|(key, value)| key == fleet || value.is_alliance));
        assert!(world.systems[system].fleets.iter().all(|&key| key == fleet));
        // The armed squadrons beat the unarmed transport and land again.
        assert!(!world.fleets.contains_key(fleet));
        assert!((1..=3).contains(&squadrons::on_planet(&world, system, false)));
    }

    #[test]
    fn unchanged_stalemate_does_not_reopen_on_five_tick_cadence() {
        assert!(combat_is_on_cooldown(UNCHANGED_STALEMATE_COOLDOWN, 5));
        assert!(combat_is_on_cooldown(UNCHANGED_STALEMATE_COOLDOWN, 50_000));
        assert!(combat_is_on_cooldown(10, 14));
        assert!(!combat_is_on_cooldown(10, 15));
    }

    #[test]
    fn carried_regiments_stay_aboard_over_any_planet_without_an_order() {
        // Nothing in the original lands cargo without an order
        // (ghidra/notes/regiment-unload.md, "What happens to the rest of the
        // cargo"). Until the port's automatic landing was retired
        // (2026-10-17) a regiment landed on its side's standing planet.
        let run = |control: ControlKind, destroyed: bool| {
            let mut world = GameWorld::default();
            let make = |dat_id, control| System {
                last_arrival: None,
                dat_id: DatId::new(dat_id),
                name: "System".into(),
                sector: SectorKey::default(),
                x: 0,
                y: 0,
                exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
                popularity_alliance: 0.9,
                popularity_empire: 0.1,
                is_populated: true,
                total_energy: 0,
                raw_materials: 0,
                fleets: vec![],
                ground_units: vec![],
                special_forces: vec![],
                defense_facilities: vec![],
                manufacturing_facilities: vec![],
                production_facilities: vec![],
                fighters: Vec::new(),
                is_headquarters: false,
                is_destroyed: false,
                control,
                espionage_rating: 0.0,
            };
            let target = world.systems.insert(make(1, control));
            let other = world
                .systems
                .insert(make(2, ControlKind::Controlled(Faction::Empire)));
            let class = world.capital_ship_classes.insert(CapitalShipClass {
                is_alliance: true,
                hull: 100,
                troop_capacity: 2,
                ..CapitalShipClass::default()
            });
            let fleet = world.fleets.insert(Fleet {
                location: target,
                capital_ships: vec![ShipInstance::new(class, 100, true)],
                fighters: vec![],
                characters: vec![],
                is_alliance: true,
                has_death_star: false,
                special_forces: Vec::new(),
            });
            world.systems[target].fleets.push(fleet);
            world.systems[target].is_destroyed = destroyed;
            let troop = world.troops.insert(TroopUnit {
                class_dat_id: DatId::new(20),
                is_alliance: true,
                regiment_strength: 100,
            });
            world.systems[target].ground_units.push(troop);
            let mut states = SimulationStates {
                ai_day: rebellion_core::ai_schedule::AiDay::default(),
                clock: GameClock::new(),
                manufacturing: ManufacturingState::new(),
                missions: MissionState::new(),
                events: EventState::new(),
                ai: AIState::new(AiFaction::Empire),
                ai2: None,
                movement: MovementState::new(),
                fog: FogState::new(Faction::Alliance),
                blockade: BlockadeState::new(),
                uprising: UprisingState::new(),
                research: ResearchState::new(),
                victory: VictoryState::new(target, other),
                economy: EconomyState::default(),
                repair: RepairState::default(),
                production: ProductionState::new(),
                autoscrap: rebellion_core::autoscrap::AutoscrapState::default(),
                informant: rebellion_core::informant::InformantState::default(),
                healing: rebellion_core::healing::HealingState::default(),
                force_discovery: rebellion_core::force_discovery::ForceDiscoveryState::default(),
                dagobah: rebellion_core::dagobah::DagobahState::default(),
                final_battle: rebellion_core::final_battle::FinalBattleState::default(),
                jabba: rebellion_core::jabba::JabbaState::default(),
                seat_of_power: rebellion_core::seat_of_power::SeatOfPowerState::default(),
                troop_transport: TroopTransportState::default(),
                deliveries: DeliveryState::default(),
                combat_cooldowns: HashMap::new(),
                campaign_config: CampaignConfig::default(),
            };
            states
                .troop_transport
                .embark(&mut world, fleet, &[troop])
                .unwrap();
            run_simulation_tick(
                &mut world,
                &mut states,
                &[TickEvent { tick: 1 }],
                &[0.99; 2_048],
                10,
            );
            states.troop_transport.cargo(fleet).is_empty()
                && world.systems[target].ground_units.contains(&troop)
        };
        // The side's planet, standing or destroyed, another side's, or a
        // neutral one: it stays aboard.
        assert!(!run(ControlKind::Controlled(Faction::Alliance), false));
        assert!(!run(ControlKind::Controlled(Faction::Alliance), true));
        assert!(!run(ControlKind::Controlled(Faction::Empire), false));
        assert!(!run(ControlKind::Uncontrolled, false));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    fn regiments_of_both_sides_stand_together_without_an_assault() {
        // The original fights on the ground only in a Planetary Assault
        // (ghidra/notes/planetary-assault.md, FUN_00563390): two sides'
        // regiments on one planet stand together, and neither side takes
        // it. This test asserted the port's automatic surface battle.
        let mut world = GameWorld::default();
        let make_system = |dat_id, name: &str, control| System {
            last_arrival: None,
            dat_id: DatId::new(dat_id),
            name: name.into(),
            sector: SectorKey::default(),
            x: dat_id as u16 * 100,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control,
            espionage_rating: 0.0,
        };
        let target = world.systems.insert(make_system(
            1,
            "Contested Surface",
            ControlKind::Controlled(Faction::Empire),
        ));
        let other = world.systems.insert(make_system(
            2,
            "Other System",
            ControlKind::Controlled(Faction::Alliance),
        ));
        let fleet = world.fleets.insert(Fleet {
            location: target,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[target].fleets.push(fleet);
        let troop_class = DatId::new(20);
        world.troop_classes.insert(
            troop_class,
            TroopClassDef {
                attack_strength: 0,
                defense_strength: 10,
                detection: 0,
                uprising_defense: 0,
            },
        );
        let alliance_troop = world.troops.insert(TroopUnit {
            class_dat_id: troop_class,
            is_alliance: true,
            regiment_strength: 1_000,
        });
        let empire_troop = world.troops.insert(TroopUnit {
            class_dat_id: troop_class,
            is_alliance: false,
            regiment_strength: 1_000,
        });
        world.systems[target]
            .ground_units
            .extend([alliance_troop, empire_troop]);

        let mut states = SimulationStates {
            ai_day: rebellion_core::ai_schedule::AiDay::default(),
            clock: GameClock::new(),
            manufacturing: ManufacturingState::new(),
            missions: MissionState::new(),
            events: EventState::new(),
            ai: AIState::new(AiFaction::Empire),
            ai2: None,
            movement: MovementState::new(),
            fog: FogState::new(Faction::Alliance),
            blockade: BlockadeState::new(),
            uprising: UprisingState::new(),
            research: ResearchState::new(),
            victory: VictoryState::new(other, target),
            economy: EconomyState::default(),
            repair: RepairState::default(),
            production: ProductionState::new(),
            autoscrap: rebellion_core::autoscrap::AutoscrapState::default(),
            informant: rebellion_core::informant::InformantState::default(),
            healing: rebellion_core::healing::HealingState::default(),
            force_discovery: rebellion_core::force_discovery::ForceDiscoveryState::default(),
            dagobah: rebellion_core::dagobah::DagobahState::default(),
            final_battle: rebellion_core::final_battle::FinalBattleState::default(),
            jabba: rebellion_core::jabba::JabbaState::default(),
            seat_of_power: rebellion_core::seat_of_power::SeatOfPowerState::default(),
            troop_transport: TroopTransportState::default(),
            deliveries: DeliveryState::default(),
            combat_cooldowns: HashMap::new(),
            campaign_config: CampaignConfig::default(),
        };

        let first_events = run_simulation_tick(
            &mut world,
            &mut states,
            &[TickEvent { tick: 1 }],
            &[0.99; 2_048],
            10,
        );
        let first_strength = world.troops[alliance_troop].regiment_strength;
        assert_eq!(first_strength, 1_000);
        assert!(states.troop_transport.is_empty());
        assert!(!first_events
            .iter()
            .any(|event| event.event_type == EVT_COMBAT_GROUND));

        let second_events = run_simulation_tick(
            &mut world,
            &mut states,
            &[TickEvent { tick: 2 }],
            &[0.99; 2_048],
            20,
        );

        assert_eq!(
            world.troops[alliance_troop].regiment_strength,
            first_strength
        );
        assert_eq!(world.troops[empire_troop].regiment_strength, 1_000);
        assert!(states.troop_transport.is_empty());
        assert!(!second_events
            .iter()
            .any(|event| event.event_type == EVT_COMBAT_GROUND));
        assert_ne!(
            world.systems[target].control,
            ControlKind::Controlled(Faction::Alliance)
        );
        // The Empire's regiment alone takes the planet back (port: the
        // occupation).
        world.systems[target]
            .ground_units
            .retain(|&t| t != alliance_troop);
        world.systems[target].fleets.clear();
        world.fleets.remove(fleet);
        run_simulation_tick(
            &mut world,
            &mut states,
            &[TickEvent { tick: 3 }],
            &[0.99; 2_048],
            30,
        );
        assert_eq!(
            world.systems[target].control,
            ControlKind::Controlled(Faction::Empire)
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    fn a_decisive_imperial_victory_leaves_the_alliance_hq_facility_standing() {
        // No bombardment follows a battle: the original bombards only on the
        // Fleet menu's orders 0x220..0x223 (FUN_005637c0,
        // ghidra/notes/bombardment-order.md); the port's automatic one was
        // invented (F-032).
        let mut world = GameWorld::default();
        world.gnprtb = control_threshold();
        let make_system = |dat_id, name: &str, control, is_headquarters| System {
            last_arrival: None,
            dat_id: DatId::new(dat_id),
            name: name.into(),
            sector: SectorKey::default(),
            x: dat_id as u16 * 100,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
            popularity_alliance: if control == ControlKind::Controlled(Faction::Alliance) {
                0.9
            } else {
                0.1
            },
            popularity_empire: if control == ControlKind::Controlled(Faction::Alliance) {
                0.1
            } else {
                0.9
            },
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters,
            is_destroyed: false,
            control,
            espionage_rating: 0.0,
        };
        let alliance_hq = world.systems.insert(make_system(
            1,
            "Alliance HQ",
            ControlKind::Controlled(Faction::Alliance),
            true,
        ));
        let coruscant = world.systems.insert(make_system(
            2,
            "Coruscant",
            ControlKind::Controlled(Faction::Empire),
            true,
        ));
        let alliance_class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(61),
            name: "Alliance Scout".into(),
            is_alliance: true,
            is_empire: false,
            hull: 1,
            ..CapitalShipClass::default()
        });
        let empire_class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(62),
            name: "Imperial Bombardment Ship".into(),
            is_alliance: false,
            is_empire: true,
            hull: 1_000,
            turbolaser_fore: 1_000,
            bombardment_modifier: 100,
            ..CapitalShipClass::default()
        });
        let alliance_fleet = world.fleets.insert(Fleet {
            location: alliance_hq,
            capital_ships: vec![ShipInstance::new(alliance_class, 1, true)],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        let empire_fleet = world.fleets.insert(Fleet {
            location: alliance_hq,
            capital_ships: vec![ShipInstance::new(empire_class, 1_000, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[alliance_hq]
            .fleets
            .extend([alliance_fleet, empire_fleet]);

        let mut states = SimulationStates {
            ai_day: rebellion_core::ai_schedule::AiDay::default(),
            clock: GameClock::new(),
            manufacturing: ManufacturingState::new(),
            missions: MissionState::new(),
            events: EventState::new(),
            ai: AIState::new(AiFaction::Empire),
            ai2: None,
            movement: MovementState::new(),
            fog: FogState::new(Faction::Alliance),
            blockade: BlockadeState::new(),
            uprising: UprisingState::new(),
            research: ResearchState::new(),
            victory: VictoryState::new(alliance_hq, coruscant),
            economy: EconomyState::default(),
            repair: RepairState::default(),
            production: ProductionState::new(),
            autoscrap: rebellion_core::autoscrap::AutoscrapState::default(),
            informant: rebellion_core::informant::InformantState::default(),
            healing: rebellion_core::healing::HealingState::default(),
            force_discovery: rebellion_core::force_discovery::ForceDiscoveryState::default(),
            dagobah: rebellion_core::dagobah::DagobahState::default(),
            final_battle: rebellion_core::final_battle::FinalBattleState::default(),
            jabba: rebellion_core::jabba::JabbaState::default(),
            seat_of_power: rebellion_core::seat_of_power::SeatOfPowerState::default(),
            troop_transport: TroopTransportState::default(),
            deliveries: DeliveryState::default(),
            combat_cooldowns: HashMap::new(),
            campaign_config: CampaignConfig {
                victory_conditions: VictoryConditions::HeadquartersOnly,
                ..CampaignConfig::default()
            },
        };

        let events = run_simulation_tick(
            &mut world,
            &mut states,
            &[TickEvent { tick: 1 }],
            &[0.5; 2_048],
            10,
        );

        assert!(world.systems[alliance_hq].is_headquarters);
        assert!(!states.victory.resolved);
        assert!(!events
            .iter()
            .any(|event| event.event_type == EVT_BOMBARDMENT));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    fn an_imperial_transport_at_the_alliance_hq_keeps_its_regiment_aboard() {
        // A regiment carried to a system lands only on its side's planet
        // (FUN_005073d0); another side's is taken only by a Planetary
        // Assault (ghidra/notes/planetary-assault.md). This test asserted
        // the port's automatic landing and occupation of the HQ; on the same
        // campaign the regiment stays aboard, the HQ stays the Alliance's
        // and nothing bombards (ghidra/notes/bombardment-order.md).
        let mut world = GameWorld::default();
        world.gnprtb = control_threshold();
        let add_system = |world: &mut GameWorld, dat_id, name: &str, control, is_headquarters| {
            world.systems.insert(System {
                last_arrival: None,
                dat_id: DatId::new(dat_id),
                name: name.into(),
                sector: SectorKey::default(),
                x: dat_id as u16 * 100,
                y: 0,
                exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
                popularity_alliance: if control == ControlKind::Controlled(Faction::Alliance) {
                    0.9
                } else {
                    0.1
                },
                popularity_empire: if control == ControlKind::Controlled(Faction::Alliance) {
                    0.1
                } else {
                    0.9
                },
                is_populated: true,
                total_energy: 0,
                raw_materials: 0,
                fleets: vec![],
                ground_units: vec![],
                special_forces: vec![],
                defense_facilities: vec![],
                manufacturing_facilities: vec![],
                production_facilities: vec![],
                fighters: Vec::new(),
                is_headquarters,
                is_destroyed: false,
                control,
                espionage_rating: 0.0,
            })
        };
        let alliance_hq = add_system(
            &mut world,
            1,
            "Alliance HQ",
            ControlKind::Controlled(Faction::Alliance),
            true,
        );
        let coruscant = add_system(
            &mut world,
            2,
            "Coruscant",
            ControlKind::Controlled(Faction::Empire),
            true,
        );
        let origin = add_system(
            &mut world,
            3,
            "Imperial Staging",
            ControlKind::Controlled(Faction::Empire),
            false,
        );
        let transport_class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(63),
            name: "Imperial Assault Transport".into(),
            is_alliance: false,
            is_empire: true,
            hull: 100,
            troop_capacity: 1,
            bombardment_modifier: 100,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: origin,
            capital_ships: vec![ShipInstance::new(transport_class, 100, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[origin].fleets.push(fleet);
        let troop = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(20),
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[origin].ground_units.push(troop);

        let mut states = SimulationStates {
            ai_day: rebellion_core::ai_schedule::AiDay::default(),
            clock: GameClock::new(),
            manufacturing: ManufacturingState::new(),
            missions: MissionState::new(),
            events: EventState::new(),
            ai: AIState::new(AiFaction::Empire),
            ai2: None,
            movement: MovementState::new(),
            fog: FogState::new(Faction::Alliance),
            blockade: BlockadeState::new(),
            uprising: UprisingState::new(),
            research: ResearchState::new(),
            victory: VictoryState::new(alliance_hq, coruscant),
            economy: EconomyState::default(),
            repair: RepairState::default(),
            production: ProductionState::new(),
            autoscrap: rebellion_core::autoscrap::AutoscrapState::default(),
            informant: rebellion_core::informant::InformantState::default(),
            healing: rebellion_core::healing::HealingState::default(),
            force_discovery: rebellion_core::force_discovery::ForceDiscoveryState::default(),
            dagobah: rebellion_core::dagobah::DagobahState::default(),
            final_battle: rebellion_core::final_battle::FinalBattleState::default(),
            jabba: rebellion_core::jabba::JabbaState::default(),
            seat_of_power: rebellion_core::seat_of_power::SeatOfPowerState::default(),
            troop_transport: TroopTransportState::default(),
            deliveries: DeliveryState::default(),
            combat_cooldowns: HashMap::new(),
            campaign_config: CampaignConfig {
                victory_conditions: VictoryConditions::HeadquartersOnly,
                ..CampaignConfig::default()
            },
        };
        states
            .troop_transport
            .embark(&mut world, fleet, &[troop])
            .unwrap();
        assert!(begin_fleet_transit(
            &mut states.movement,
            &mut world,
            fleet,
            alliance_hq,
            1,
        ));

        let events = run_simulation_tick(
            &mut world,
            &mut states,
            &[TickEvent { tick: 1 }],
            &[0.99; 2_048],
            10,
        );

        assert!(world.systems[alliance_hq].is_headquarters);
        assert_eq!(states.troop_transport.cargo(fleet), [troop]);
        assert!(!world.systems[alliance_hq].ground_units.contains(&troop));
        assert_eq!(
            world.systems[alliance_hq].control,
            ControlKind::Controlled(Faction::Alliance)
        );
        assert!(!states.victory.resolved);
        assert!(!events
            .iter()
            .any(|event| event.event_type == EVT_BOMBARDMENT || event.event_type == EVT_VICTORY));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    fn a_transport_at_the_other_side_s_system_keeps_its_regiment_aboard() {
        // FUN_005073d0: the regiment does not land on the Empire's planet;
        // only a Planetary Assault takes it (ghidra/notes/planetary-assault.md).
        // This test asserted the port's automatic landing, occupation,
        // capture and victory; on the same campaign the fleet arrives and
        // nothing else happens.
        let mut world = GameWorld::default();
        world.gnprtb = control_threshold();
        let add_system = |world: &mut GameWorld, dat_id, name: &str, control, is_headquarters| {
            world.systems.insert(System {
                last_arrival: None,
                dat_id: DatId::new(dat_id),
                name: name.into(),
                sector: SectorKey::default(),
                x: dat_id as u16 * 100,
                y: 0,
                exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
                popularity_alliance: if control == ControlKind::Controlled(Faction::Alliance) {
                    0.9
                } else {
                    0.1
                },
                popularity_empire: if control == ControlKind::Controlled(Faction::Empire) {
                    0.9
                } else {
                    0.1
                },
                is_populated: true,
                total_energy: 0,
                raw_materials: 0,
                fleets: vec![],
                ground_units: vec![],
                special_forces: vec![],
                defense_facilities: vec![],
                manufacturing_facilities: vec![],
                production_facilities: vec![],
                fighters: Vec::new(),
                is_headquarters,
                is_destroyed: false,
                control,
                espionage_rating: 0.0,
            })
        };
        let origin = add_system(
            &mut world,
            1,
            "Alliance Base",
            ControlKind::Controlled(Faction::Alliance),
            true,
        );
        let target = add_system(
            &mut world,
            2,
            "Imperial HQ",
            ControlKind::Controlled(Faction::Empire),
            true,
        );
        let transport_class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(64),
            name: "Troop Transport".into(),
            is_alliance: true,
            is_empire: false,
            hull: 100,
            troop_capacity: 1,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: origin,
            capital_ships: vec![ShipInstance::new(transport_class, 100, true)],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[origin].fleets.push(fleet);
        let troop = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(20),
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[origin].ground_units.push(troop);
        let prisoner = world.characters.insert(Character {
            dat_id: DatId::new(4),
            name: "Imperial Leader".into(),
            is_empire: true,
            current_system: Some(target),
            ..Character::default()
        });

        let mut states = SimulationStates {
            ai_day: rebellion_core::ai_schedule::AiDay::default(),
            clock: GameClock::new(),
            manufacturing: ManufacturingState::new(),
            missions: MissionState::new(),
            events: EventState::new(),
            ai: AIState::new(AiFaction::Empire),
            ai2: None,
            movement: MovementState::new(),
            fog: FogState::new(Faction::Alliance),
            blockade: BlockadeState::new(),
            uprising: UprisingState::new(),
            research: ResearchState::new(),
            victory: VictoryState::new(origin, target),
            economy: EconomyState::default(),
            repair: RepairState::default(),
            production: ProductionState::new(),
            autoscrap: rebellion_core::autoscrap::AutoscrapState::default(),
            informant: rebellion_core::informant::InformantState::default(),
            healing: rebellion_core::healing::HealingState::default(),
            force_discovery: rebellion_core::force_discovery::ForceDiscoveryState::default(),
            dagobah: rebellion_core::dagobah::DagobahState::default(),
            final_battle: rebellion_core::final_battle::FinalBattleState::default(),
            jabba: rebellion_core::jabba::JabbaState::default(),
            seat_of_power: rebellion_core::seat_of_power::SeatOfPowerState::default(),
            troop_transport: TroopTransportState::default(),
            deliveries: DeliveryState::default(),
            combat_cooldowns: HashMap::new(),
            campaign_config: CampaignConfig::default(),
        };
        states
            .troop_transport
            .embark(&mut world, fleet, &[troop])
            .unwrap();
        states.campaign_config.victory_conditions = VictoryConditions::HeadquartersOnly;
        assert!(begin_fleet_transit(
            &mut states.movement,
            &mut world,
            fleet,
            target,
            1,
        ));

        let events = run_simulation_tick(
            &mut world,
            &mut states,
            &[TickEvent { tick: 1 }],
            &[0.99; 2048],
            10,
        );

        assert_eq!(states.troop_transport.cargo(fleet), [troop]);
        assert!(!world.systems[target].ground_units.contains(&troop));
        assert_eq!(
            world.systems[target].control,
            ControlKind::Controlled(Faction::Empire)
        );
        assert!(!world.characters[prisoner].is_captive);
        assert!(!states.victory.resolved);
        assert!(events
            .iter()
            .any(|event| event.event_type == EVT_FLEET_ARRIVED));
        for event_type in [
            EVT_TROOP_MOVED,
            EVT_CONTROL_CHANGED,
            EVT_CAPTURE,
            EVT_VICTORY,
        ] {
            assert!(
                !events.iter().any(|event| event.event_type == event_type),
                "unexpected {event_type} telemetry"
            );
        }
    }
}
