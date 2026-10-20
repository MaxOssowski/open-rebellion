//! Shared simulation tick function for headless and interactive use.
//!
//! Extracts the per-tick system-advance logic from `rebellion-app/main.rs`
//! into a reusable function that returns structured `GameEventRecord`s
//! instead of pushing to a `MessageLog`.

use std::collections::HashMap;

use crate::integrator::{resolve_system_space_combat, PerceptionIntegrator};
use rebellion_core::ai::{AIState, AISystem};
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
}

/// Run one simulation tick across all 15 systems.
///
/// Advances each system in the canonical order (economy → manufacturing → movement →
/// combat → fog → missions → events → AI → blockade → uprising → betrayal →
/// `death_star` → research → jedi → victory), applies effects to `world`, and
/// returns a `Vec<GameEventRecord>` describing everything that happened.
///
/// `tick_events` comes from `GameClock::advance()`. `rolls` is a pre-generated
/// slice of uniform `[0,1)` f64 values consumed sequentially by systems that
/// need randomness. Pass at least 1024 rolls to cover a typical tick.
///
/// `wall_ms` is the wall-clock milliseconds since session start, used for
/// the `wall_ms` field on each event record.
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
/// The systems a Destroy System order's Death Star destroyed this tick
/// (`FUN_0058d990`), as the Death Star's `PlanetDestroyed` events, which
/// step 11 carries out.
fn planets_destroyed(
    bombard: &rebellion_core::bombardment::BombardEnv,
    tick_events: &[TickEvent],
) -> Vec<rebellion_core::death_star::DeathStarEvent> {
    let Some(last) = tick_events.last() else {
        return Vec::new();
    };
    bombard
        .destroyed_systems()
        .into_iter()
        .map(
            |system| rebellion_core::death_star::DeathStarEvent::PlanetDestroyed {
                system,
                tick: last.tick,
            },
        )
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

pub fn run_simulation_tick(
    world: &mut GameWorld,
    states: &mut SimulationStates,
    tick_events: &[TickEvent],
    rolls: &[f64],
    wall_ms: u64,
    config: &rebellion_core::tuning::GameConfig,
) -> Vec<GameEventRecord> {
    let Some(last_tick_event) = tick_events.last() else {
        return Vec::new();
    };

    let mut integrator = PerceptionIntegrator::new(last_tick_event.tick, wall_ms);
    let mut roll_cursor = 0usize;
    let current_tick = last_tick_event.tick;

    // `MovementState` is authoritative while a fleet is in hyperspace. Repair
    // stale save/index state before economy and manufacturing inspect orbits.
    reconcile_fleet_orbits(&states.movement, world);

    // Helper: consume N rolls from the slice, padding with 1.0 if exhausted.
    let mut take_rolls = |n: usize| -> Vec<f64> {
        let end = (roll_cursor + n).min(rolls.len());
        let slice = &rolls[roll_cursor..end];
        roll_cursor = end;
        let mut v: Vec<f64> = slice.to_vec();
        // Pad with 1.0 (safe default: never fires probability checks)
        v.resize(n, 1.0);
        v
    };

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
    integrator.apply_economy_events(world, &economy_events);

    // ── 1. Manufacturing ─────────────────────────────────────────────────
    // `newly_idle` feeds the K6 EVT_MANUFACTURING_IDLE telemetry for any
    // system whose queue transitioned empty this tick.
    // A manager with no yard left scraps its queue; yards in a system in
    // uprising or under blockade are suspended (FUN_00529dd0, FUN_0050bbb0).
    scrap_yardless_queues(world, &mut states.manufacturing);
    let suspended = suspended_systems(&states.blockade, &states.uprising);
    // Each yard cycle takes one refined material and ends with one build
    // point for its manager (FUN_00530820, FUN_0052a430).
    let mfg_advance = advance_yards(
        &mut states.production,
        world,
        &mut states.manufacturing,
        |system| suspended.contains(&system),
        tick_events,
    );
    integrator.apply_build_completions(world, &mfg_advance.completions);
    integrator.apply_manufacturing_idle(world, &mfg_advance.newly_idle);
    for completion in &mfg_advance.completions {
        states.combat_cooldowns.remove(&completion.system);
    }
    // Remote products travel, then complete at their destination (F-030).
    states.deliveries.depart(world, &mfg_advance.departures);
    let delivered = states.deliveries.advance(world, tick_events);
    integrator.apply_build_completions(world, &delivered.arrivals);
    integrator.apply_deliveries_lost(world, &delivered.lost);
    for completion in &delivered.arrivals {
        states.combat_cooldowns.remove(&completion.system);
    }

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
    // Regiments travelling on their own (FUN_00556430, event 0x387).
    let regiments = states.troop_transport.advance_transit(world, current_tick);
    integrator.apply_regiment_arrivals(world, &regiments);
    // Squadrons travelling on their own (squadrons::SquadronTransits).
    let _ = states
        .troop_transport
        .squadrons
        .advance(world, current_tick);

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
        let combat_rolls = take_rolls(256);
        let sorties = squadrons::scramble(world, sys_key);
        let battle_before = rebellion_core::loyalty::battle_watch(world, sys_key);
        let space_result = resolve_system_space_combat(
            world,
            sys_key,
            world.difficulty_index,
            &combat_rolls,
            current_tick,
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

        integrator.emit_system_space_combat(world, &space_result);
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
            }
        }
    }

    // Observe embarked regiments after arrivals and landings, so a fleet that
    // arrives and is ordered away again this tick still copies the arrival
    // system's withdraw percent. Their rolls happen at the blockade step.
    let mut running_regiments = if tick_events.is_empty() {
        Vec::new()
    } else {
        BlockadeSystem::running_regiments(
            &mut states.blockade,
            world,
            &states.movement,
            &states.troop_transport,
        )
    };

    // ── 4. Fog of war ────────────────────────────────────────────────────
    let reveals = FogSystem::advance(&mut states.fog, world, &states.movement);
    integrator.emit_fog_reveals(&reveals, world);

    // ── 5. Missions ──────────────────────────────────────────────────────
    let mission_rolls = take_rolls(
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
    rebellion_core::missions::apply_advance_effects(world, &mission_advance.effects);
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
    let dagobah_rolls = take_rolls(rebellion_core::dagobah::DagobahSystem::roll_budget());
    if let Some(pulse) = rebellion_core::dagobah::DagobahSystem::advance(
        &mut states.dagobah,
        world,
        &states.missions,
        tick_events,
        &dagobah_rolls,
    ) {
        let mission = rebellion_core::dagobah::dispatch(&mut states.missions, world, &pulse);
        integrator.record_luke_dagobah(world, &pulse, mission.is_some());
    }

    // ── 5c. The Final Battle: FUN_00544da0 .. FUN_00544fc0 ───────────────
    let final_battle_rolls =
        take_rolls(rebellion_core::final_battle::FinalBattleSystem::roll_budget());
    for step in rebellion_core::final_battle::FinalBattleSystem::advance(
        &mut states.final_battle,
        world,
        &states.missions,
        tick_events,
        &final_battle_rolls,
    ) {
        let mission = rebellion_core::final_battle::perform(&step, &mut states.missions, world);
        integrator.record_final_battle_step(world, &step, mission.is_some());
    }

    // ── 5d. The bounty hunters: timer 0x393 (FUN_00546fa0) ───────────────
    let jabba_rolls = take_rolls(rebellion_core::jabba::JabbaSystem::roll_budget());
    for step in rebellion_core::jabba::JabbaSystem::advance(
        &mut states.jabba,
        world,
        &states.missions,
        tick_events,
        &jabba_rolls,
    ) {
        let mission = rebellion_core::jabba::perform(&step, &mut states.missions, world);
        integrator.record_jabba_step(world, &step, mission.is_some());
    }

    // ── 5e. `SeatOfPower` (FUN_005617b0, notification 0x230) ───────────────
    if let Some(held) =
        rebellion_core::seat_of_power::SeatOfPowerSystem::advance(&mut states.seat_of_power, world)
    {
        if let Some(tick) = tick_events.last().map(|event| event.tick) {
            integrator.record_seat_of_power(held, tick);
        }
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
        integrator.apply_mission_result(world, result, &mut states.uprising, &mut states.research);
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

    // ── 5b. Character escapes ────────────────────────────────────────────
    // Each captive draws at most an attempt and its next delay.
    let escape_rolls = take_rolls(2 * world.characters.len());
    let escape_effects =
        MissionSystem::advance_escapes(&mut states.missions, world, current_tick, &escape_rolls);
    integrator.apply_escape_effects(world, &escape_effects);

    // ── 6. Events ────────────────────────────────────────────────────────
    let event_rolls: Vec<f32> = take_rolls(16).iter().map(|&r| r as f32).collect();
    let fired_events = EventSystem::advance(&mut states.events, world, tick_events, &event_rolls);
    integrator.apply_fired_events(world, &fired_events, current_tick, &states.movement);

    // ── 7. AI ────────────────────────────────────────────────────────────
    // Each day, as many passes as the speed has sub-ticks, one side then the
    // other (FUN_0041cf20 -> FUN_004361a0); each pass's orders apply before
    // the next. A new day clears each side's order count (FUN_00485de0).
    let passes = rebellion_core::ai::passes_per_day(states.clock.speed);
    // The orders' bombardments draw from a stream seeded by one roll.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "A roll in 0..1 scaled to 53 bits of seed."
    )]
    let bombard_seed = (take_rolls(1)[0] * 9_007_199_254_740_992.0) as u64;
    let mut bombard = rebellion_core::bombardment::BombardEnv::new(
        bombard_seed,
        Some(states.victory.alliance_hq),
    );
    for event in tick_events {
        states.ai.begin_day();
        states.ai.set_game_type(states.campaign_config.victory_conditions);
        if let Some(ref mut ai2) = states.ai2 {
            ai2.begin_day();
            ai2.set_game_type(states.campaign_config.victory_conditions);
        }
        // The AI follows its ships by id (GameWorld::number_ships).
        world.number_ships();
        for _ in 0..passes {
            let ai_actions = AISystem::pass(
                &mut states.ai,
                world,
                &states.manufacturing,
                &states.missions,
                &states.movement,
                &states.economy,
                &states.troop_transport,
                event.tick,
                config,
                &states.research,
                &states.uprising,
            );
            integrator.apply_ai_actions(
                &ai_actions,
                &mut states.ai,
                &mut states.missions,
                &mut states.manufacturing,
                &mut states.movement,
                &mut states.troop_transport,
                &mut states.production,
                world,
                event.tick,
                false,
                &mut bombard,
            );
            // ── 7b. AI (second faction, dual-AI mode) ────────────────────
            if let Some(ref mut ai2) = states.ai2 {
                let secondary_actions = AISystem::pass(
                    ai2,
                    world,
                    &states.manufacturing,
                    &states.missions,
                    &states.movement,
                    &states.economy,
                    &states.troop_transport,
                    event.tick,
                    config,
                    &states.research,
                    &states.uprising,
                );
                integrator.apply_ai_actions(
                    &secondary_actions,
                    ai2,
                    &mut states.missions,
                    &mut states.manufacturing,
                    &mut states.movement,
                    &mut states.troop_transport,
                    &mut states.production,
                    world,
                    event.tick,
                    true,
                    &mut bombard,
                );
            }
        }
    }

    // ── 8. Blockade ──────────────────────────────────────────────────────
    let mut blockade_events = BlockadeSystem::advance(&mut states.blockade, world, tick_events);
    if let Some(last) = tick_events.last() {
        running_regiments.extend(BlockadeSystem::running_regiments(
            &mut states.blockade,
            world,
            &states.movement,
            &states.troop_transport,
        ));
        let running_rolls = take_rolls(running_regiments.len());
        blockade_events.extend(BlockadeSystem::resolve_running(
            &running_regiments,
            &running_rolls,
            last.tick,
        ));
    }
    integrator.apply_blockade_events(world, &mut states.troop_transport, &blockade_events);
    // Event305_SystemBlockadeChanged_Handle -> FUN_00561280: each ended
    // blockade's loyalty shift for its assault and bombardment losses.
    for (system, blockade) in states.blockade.take_ended() {
        let shift = rebellion_core::loyalty::settle_blockade(world, system, blockade);
        integrator.record_loyalty_shifts(shift.as_slice());
    }

    // ── 9. Uprising ──────────────────────────────────────────────────────
    let uprising_rolls = take_rolls(UprisingSystem::roll_budget(
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
    integrator.apply_uprising_events(world, &uprising_events);

    // ── 9b. Informants: each held system's timer 0x38e ──────────────────
    let informant_rolls = take_rolls(rebellion_core::informant::InformantSystem::roll_budget(
        world,
    ));
    let informant_reports = rebellion_core::informant::InformantSystem::advance(
        &mut states.informant,
        world,
        tick_events,
        &informant_rolls,
    );
    integrator.apply_informant_reports(world, &mut states.fog, &informant_reports);

    // ── 9c. Injury recovery: each healing character's timer 0x388 ────────
    let healed =
        rebellion_core::healing::HealingSystem::advance(&mut states.healing, world, tick_events);
    integrator.apply_healing(world, &healed);
    // A capture's hook (slot +0x34c, FUN_004f1a80) clears the captive's
    // command (FUN_004eeed0).
    rebellion_core::character_command::release_unfit(world);

    // ── 9d. Force discovery: each discovering trainer's timer 0x389 ──────
    let discovery_rolls =
        take_rolls(rebellion_core::force_discovery::ForceDiscoverySystem::roll_budget(world));
    let discovered = rebellion_core::force_discovery::ForceDiscoverySystem::advance(
        &mut states.force_discovery,
        world,
        tick_events,
        &discovery_rolls,
    );
    integrator.apply_force_discoveries(world, &discovered);

    // ── 10. (The port's 50-day defection is gone: the original's only
    // betrayal is the mission phase; ghidra/notes/betrayal-and-loyalty.md.)

    // ── 11. Death Star ───────────────────────────────────────────────────
    for (req, outcome) in &bombard.outcomes {
        integrator.emit_bombardment(world, req, outcome);
    }
    // System_vtd0: the day's bombardment and assault losses count toward
    // each blockade's (DestroyedReason 0xf, 0xe).
    for (system, losses) in &bombard.losses {
        states.blockade.record_losses(*system, *losses);
    }
    for (req, outcome) in &bombard.assaults {
        integrator.emit_assault(world, req, outcome);
    }
    let ds_events = planets_destroyed(&bombard, tick_events);
    integrator.apply_death_star_events(world, &ds_events);
    // Update victory state and clean up destroyed systems.
    //
    // Knesset Shamash-Bet #R11: `cleanup_destroyed_system` now drains an
    // out-parameter `Vec<GameEffect>` — we turn each `GameEffect::CharacterKilled`
    // into an `EVT_CHARACTER_KILLED` telemetry record (DI-H2 payload uses
    // `name` / `dat_id` rather than `CharacterKey`). Character-death story
    // events are strictly next-tick reactive.
    for evt in &ds_events {
        let rebellion_core::death_star::DeathStarEvent::PlanetDestroyed { system, .. } = evt;
        {
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
    }

    // ── 12. Research ─────────────────────────────────────────────────────
    // Research missions add their points when they resolve (step 5).

    // ── 12b. Ship repair ────────────────────────────────────────────────
    let repair_events = RepairSystem::advance(&mut states.repair, world, tick_events);
    integrator.apply_repair_events(world, &repair_events);

    // ── 13. (The port's Jedi tiers are gone: the Force grows through the
    // missions, Dagobah, escapes and the encounter; ghidra/notes/
    // jedi-rs-replacement.md.)

    // ── 13b. Mines, refineries and the side stockpiles ───────────────────
    // After the other consumers so their rolls keep their positions.
    let production_rolls = take_rolls(world.production_facilities.len());
    ProductionSystem::advance(
        &mut states.production,
        world,
        &states.manufacturing,
        |system| states.blockade.is_blockaded(system) || states.uprising.is_uprising(system),
        |system| rebellion_core::economy::production_modifier_at(&states.economy, system),
        tick_events,
        &production_rolls,
    );

    // ── 13c. Maintenance shortfall: each side's autoscrap timer 0x382 ───
    let autoscrap_rolls = take_rolls(2);
    let autoscraps = rebellion_core::autoscrap::AutoscrapSystem::advance(
        &mut states.autoscrap,
        world,
        &states.troop_transport,
        &states.manufacturing,
        |system| states.blockade.is_blockaded(system) || states.uprising.is_uprising(system),
        tick_events,
        &autoscrap_rolls,
    );
    integrator.apply_autoscraps(
        world,
        &mut states.production,
        &mut states.troop_transport,
        &autoscraps,
    );

    // ── 13d. The yards' own research: each side's timer 0x381 ───────────
    let tally_rolls = take_rolls(2);
    let tallies = rebellion_core::research::ResearchSystem::advance_tally(
        &mut states.research,
        world,
        |system| states.blockade.is_blockaded(system) || states.uprising.is_uprising(system),
        tick_events,
        &tally_rolls,
    );
    integrator.record_research_tallies(&tallies);

    // port: a fleet that gained its side's flagship class this tick, by a
    // build or a join on arrival, takes its signature name (fleet_name_bank).
    world.name_flagship_fleets();

    // ── 13e. Loyalty: Coruscant taken, the Alliance HQ lost ─────────────
    // (FUN_005616a0, FUN_00561780; ghidra/notes/betrayal-and-loyalty.md)
    let shifts = rebellion_core::loyalty::settle(
        world,
        loyalty_before,
        Some(states.victory.alliance_hq),
        &mut |m| draw_from_roll(take_rolls(1)[0], m),
    );
    integrator.record_loyalty_shifts(&shifts);

    // ── 14. Victory check ────────────────────────────────────────────────
    // The victory conditions word (FUN_005180e0) and its 0x128.
    if let Some((old, new)) = rebellion_core::victory::advance_conditions_word(
        &mut states.victory,
        world,
        states.campaign_config.victory_conditions,
    ) {
        integrator.record_victory_conditions(old, new, current_tick);
    }
    integrator.emit_victory_check(&states.victory);
    if let Some(outcome) = VictorySystem::check(
        &states.victory,
        world,
        tick_events,
        states.campaign_config.victory_conditions,
    ) {
        integrator.apply_victory(&outcome, &mut states.victory, world);
    }

    // ── 15. Campaign snapshot (every 250 ticks) ────────────────────────
    if current_tick.is_multiple_of(250) && current_tick > 0 {
        integrator.emit_campaign_snapshot(world, states.movement.len(), &states.economy);
    }

    integrator.finish()
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
        let events = planets_destroyed(&env, &[TickEvent { tick: 4 }, TickEvent { tick: 5 }]);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            rebellion_core::death_star::DeathStarEvent::PlanetDestroyed { system, tick: 5 }
                if system == a
        ));
        assert!(planets_destroyed(&env, &[]).is_empty());
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
        run_simulation_tick(
            world,
            states,
            &[TickEvent { tick: 1 }],
            &[0.99; 2048],
            10,
            &rebellion_core::tuning::GameConfig::default(),
        );
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
                &rebellion_core::tuning::GameConfig::default(),
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
                &rebellion_core::tuning::GameConfig::default(),
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
                    &rebellion_core::tuning::GameConfig::default(),
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

        let result = run_simulation_tick(
            &mut world,
            &mut states,
            &[],
            &[],
            0,
            &rebellion_core::tuning::GameConfig::default(),
        );
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
                &rebellion_core::tuning::GameConfig::default(),
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
        let config = rebellion_core::tuning::GameConfig::default();

        let first_events = run_simulation_tick(
            &mut world,
            &mut states,
            &[TickEvent { tick: 1 }],
            &[0.99; 2_048],
            10,
            &config,
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
            &config,
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
            &config,
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
        });
        let empire_fleet = world.fleets.insert(Fleet {
            location: alliance_hq,
            capital_ships: vec![ShipInstance::new(empire_class, 1_000, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[alliance_hq]
            .fleets
            .extend([alliance_fleet, empire_fleet]);

        let mut states = SimulationStates {
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
            &rebellion_core::tuning::GameConfig::default(),
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
        });
        world.systems[origin].fleets.push(fleet);
        let troop = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(20),
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[origin].ground_units.push(troop);

        let mut states = SimulationStates {
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
            &rebellion_core::tuning::GameConfig::default(),
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
            &rebellion_core::tuning::GameConfig::default(),
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
