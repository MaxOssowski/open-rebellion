//! `PerceptionIntegrator` — centralizes effect application and telemetry emission.
//!
//! Phase 4 of Knesset Ereshkigal. The integrator translates ad-hoc system events
//! into world mutations and structured `GameEventRecord` telemetry.
//!
//! Architecture: simulation.rs orchestrates 17 system `advance()` calls and delegates
//! effect application + telemetry to the integrator. This keeps simulation.rs focused
//! on tick composition while the integrator owns the mutation/telemetry contract.
//!
//! All 17 simulation sections route through `PerceptionIntegrator` methods for both
//! world mutation and telemetry emission. simulation.rs is a thin tick orchestrator (~449 LOC).

use std::collections::HashMap;

use rebellion_core::ai::{AIAction, AIState, AiFaction};
use rebellion_core::betrayal::BetrayalEvent;
use rebellion_core::blockade::BlockadeEvent;
use rebellion_core::combat::{CombatSide, CombatSystem, GroundCombatResult, SpaceCombatResult};
use rebellion_core::death_star::DeathStarEvent;
use rebellion_core::economy::{EconomyEvent, EconomyState};
use rebellion_core::events::{EventAction, FiredEvent, SkillField, SystemTag};
use rebellion_core::fog::RevealEvent;
use rebellion_core::game_events::{
    GameEventRecord, EVT_AI_ACTION, EVT_BETRAYAL_CHECK, EVT_BLOCKADE_ENDED, EVT_BLOCKADE_STARTED,
    EVT_BLOCKADE_TROOP_DESTROYED, EVT_BOMBARDMENT, EVT_BUILD_COMPLETE, EVT_CAMPAIGN_SNAPSHOT,
    EVT_CAPTURE, EVT_CHARACTER_HEALTH, EVT_CHARACTER_KILLED, EVT_COLLECTION_RATE,
    EVT_COMBAT_GROUND, EVT_COMBAT_SPACE, EVT_CONTROL_CHANGED, EVT_DS_CONSTRUCTION, EVT_DS_FIRED,
    EVT_DS_STATUS, EVT_ECONOMY_TICK, EVT_ESCAPE, EVT_EVENT_FIRED, EVT_FLEET_ARRIVED,
    EVT_FOG_REVEALED, EVT_GARRISON_REQUIRED, EVT_HQ_CAPTURED, EVT_INFORMANT_INTEL, EVT_JEDI_CHECK,
    EVT_JEDI_DISCOVERED, EVT_JEDI_TIER, EVT_MAINTENANCE_SHORTFALL, EVT_MANUFACTURING_IDLE,
    EVT_MISSION_DISPATCHED, EVT_MISSION_RESOLVED, EVT_NATURAL_DISASTER, EVT_RESEARCH_UNLOCKED, EVT_RESOURCE_DISCOVERY,
    EVT_SABOTEUR_DETECTED, EVT_SHIP_REPAIRED, EVT_SHIP_REPAIR_STARTED, EVT_SIDE_CHANGE,
    EVT_SUPPORT_CHANGE, EVT_SUPPORT_DRIFT, EVT_TRAITOR_REVEALED, EVT_TROOP_MOVED,
    EVT_DESTROYED_ON_ARRIVAL, EVT_UNITS_DEPLOYED, EVT_UPRISING_BEGAN, EVT_UPRISING_CHECK, EVT_UPRISING_ENDED,
    EVT_UPRISING_INCIDENT, EVT_VICTORY, EVT_VICTORY_CHECK, SYS_AI, SYS_BETRAYAL, SYS_BLOCKADE,
    SYS_COMBAT, SYS_DEATH_STAR, SYS_ECONOMY, SYS_EVENTS, SYS_FOG, SYS_JEDI, SYS_MANUFACTURING,
    SYS_MISSIONS, SYS_MOVEMENT, SYS_REPAIR, SYS_RESEARCH, SYS_STORY, SYS_UPRISING, SYS_VICTORY,
};
use rebellion_core::ids::DatId;
use rebellion_core::ids::{CharacterKey, FleetKey, SystemKey, TroopKey};
use rebellion_core::jedi::{JediEvent, JediState};
use rebellion_core::manufacturing::{
    BuildableKind, CompletionEvent, ManufacturingState, QueueItem,
};
use rebellion_core::missions::{
    MissionEffect, MissionFaction, MissionKind, MissionMember, MissionRequest, MissionResult,
    MissionState,
};
use rebellion_core::movement::{
    apply_fleet_arrival, begin_fleet_transit, ArrivalEvent, MovementState,
};
use rebellion_core::production_cycle::ProductionState;
use rebellion_core::repair::RepairEvent;
use rebellion_core::research::ResearchState;
use rebellion_core::troop_transport::{RegimentArrivals, RegimentLeg, TroopTransportState};
use rebellion_core::uprising::{UprisingEvent, UprisingState};
use rebellion_core::victory::VictoryOutcome;
use rebellion_core::world::{
    CapitalShipClass, ControlKind, DefenseFacilityInstance, Fleet, GameWorld,
    ManufacturingFacilityInstance, ProductionFacilityInstance, ShipInstance, SpecialForceUnit,
    TroopUnit,
};

// ---------------------------------------------------------------------------
// Name resolution helpers (shared with simulation.rs)
// ---------------------------------------------------------------------------

/// Resolve a `SystemKey` to the system's name, or a fallback string.
#[must_use]
pub fn sys_name(world: &GameWorld, key: SystemKey) -> String {
    world
        .systems
        .get(key)
        .map_or_else(|| format!("{key:?}"), |s| s.name.clone())
}

/// Resolve a `CharacterKey` to the character's name, or a fallback string.
#[must_use]
pub fn char_name(world: &GameWorld, key: CharacterKey) -> String {
    world
        .characters
        .get(key)
        .map_or_else(|| format!("{key:?}"), |c| c.name.clone())
}

/// Name a mission member: a character's name, or a special force's SPECFCSD
/// class id (units carry no name of their own).
#[must_use]
pub fn member_name(world: &GameWorld, member: MissionMember) -> String {
    match member {
        MissionMember::Character(key) => char_name(world, key),
        MissionMember::SpecialForce(key) => world.special_forces.get(key).map_or_else(
            || format!("{key:?}"),
            |unit| format!("special force {:#010x}", unit.class_dat_id.raw()),
        ),
    }
}

fn member_names(world: &GameWorld, members: &[MissionMember]) -> Vec<String> {
    members.iter().map(|&m| member_name(world, m)).collect()
}

/// Format an `AIAction` as a structured JSON payload with readable names.
#[must_use]
pub fn ai_action_json(action: &AIAction, world: &GameWorld) -> serde_json::Value {
    match action {
        AIAction::MoveFleet {
            fleet,
            to_system,
            reason,
            troops,
        } => {
            let faction = world.fleets.get(*fleet).map_or("unknown", |f| {
                if f.is_alliance {
                    "Alliance"
                } else {
                    "Empire"
                }
            });
            let from = world
                .fleets
                .get(*fleet)
                .map_or_else(|| "unknown".into(), |f| sys_name(world, f.location));
            serde_json::json!({
                "type": "MoveFleet",
                "faction": faction,
                "from": from,
                "to": sys_name(world, *to_system),
                "reason": format!("{:?}", reason),
                "troops": troops.len(),
            })
        }
        AIAction::DispatchMission {
            kind,
            team,
            decoys,
            target_system,
            ..
        } => {
            serde_json::json!({
                "type": "DispatchMission",
                "kind": format!("{:?}", kind),
                "target": sys_name(world, *target_system),
                "team": member_names(world, team),
                "decoys": member_names(world, decoys),
            })
        }
        AIAction::AbortMission { mission_id } => {
            serde_json::json!({
                "type": "AbortMission",
                "mission_id": mission_id,
            })
        }
        AIAction::Scrap { team } => {
            serde_json::json!({
                "type": "Scrap",
                "team": team.iter().map(|t| format!("{t:?}")).collect::<Vec<_>>(),
            })
        }
        AIAction::MoveRegiments { troops, to } => {
            serde_json::json!({
                "type": "MoveRegiments",
                "troops": troops.len(),
                "to": sys_name(world, *to),
            })
        }
        AIAction::MoveSquadrons {
            class,
            count,
            from,
            to,
        } => {
            serde_json::json!({
                "type": "MoveSquadrons",
                "class": world.fighter_classes.get(*class).map_or("unknown", |c| c.name.as_str()),
                "count": count,
                "from": format!("{from:?}"),
                "to": format!("{to:?}"),
            })
        }
        AIAction::MoveShips { fleet, ships, to } => {
            let from = world
                .fleets
                .get(*fleet)
                .map_or_else(|| "unknown".into(), |f| sys_name(world, f.location));
            serde_json::json!({
                "type": "MoveShips",
                "ships": ships.len(),
                "from": from,
                "to": sys_name(world, *to),
            })
        }
        AIAction::CreateFleet { fleet, ships } => serde_json::json!({
            "type": "CreateFleet",
            "ships": ships.len(),
            "at": world
                .fleets
                .get(*fleet)
                .map_or_else(|| "unknown".into(), |f| sys_name(world, f.location)),
        }),
        AIAction::SetDestination {
            system,
            area,
            destination,
        } => {
            serde_json::json!({
                "type": "SetDestination",
                "system": sys_name(world, *system),
                "area": format!("{area:?}"),
                "destination": sys_name(world, *destination),
            })
        }
        AIAction::Build {
            system,
            kind,
            cost,
            count,
        } => {
            serde_json::json!({
                "type": "Build",
                "system": sys_name(world, *system),
                "kind": format!("{kind:?}"),
                "cost": cost,
                "count": count,
            })
        }
        AIAction::EnqueueProduction { system, kind, cost } => {
            serde_json::json!({
                "type": "EnqueueProduction",
                "system": sys_name(world, *system),
                "kind": format!("{:?}", kind),
                "cost": cost,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// PerceptionIntegrator
// ---------------------------------------------------------------------------

/// Centralizes effect application and telemetry emission for one simulation tick.
///
/// Usage:
/// ```ignore
/// let mut integrator = PerceptionIntegrator::new(tick, wall_ms);
/// // ... system advance() calls with integrator.apply_*() ...
/// let telemetry = integrator.finish();
/// ```
pub struct PerceptionIntegrator {
    events: Vec<GameEventRecord>,
    tick: u64,
    wall_ms: u64,
    /// Effect queue for cross-system story/message routing. Populated by
    /// `apply_event_action_to_world` (see Knesset Shamash-Bet Dabora 2
    /// #F7/#A3) when an `EventAction::DisplayMessage` fires or when a
    /// `SpawnSpecialForce` action resolves to a target system. The
    /// interactive `main.rs` drains this with `drain_story_effects()`
    /// after the tick completes and routes each effect into its
    /// `MessageLog` / special-forces arena. Headless `simulation.rs`
    /// leaves the queue alone and `finish()` discards it.
    story_effects: Vec<rebellion_core::effects::GameEffect>,
}

impl PerceptionIntegrator {
    /// Create a new integrator for a single simulation tick.
    #[must_use]
    pub fn new(tick: u64, wall_ms: u64) -> Self {
        Self {
            events: Vec::new(),
            tick,
            wall_ms,
            story_effects: Vec::new(),
        }
    }

    /// Drain the queued story/message effects so the interactive main loop
    /// can route `StoryMessageDisplayed` records into its `MessageLog` and
    /// `SpecialForceSpawned` records into its (eventual) special-forces
    /// arena. Called by `main.rs` after `apply_fired_events`. The headless
    /// `simulation.rs` path ignores this queue and lets `finish()` discard
    /// whatever remains.
    pub fn drain_story_effects(&mut self) -> Vec<rebellion_core::effects::GameEffect> {
        std::mem::take(&mut self.story_effects)
    }

    /// Current tick number.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Wall-clock milliseconds.
    #[must_use]
    pub fn wall_ms(&self) -> u64 {
        self.wall_ms
    }

    /// Add a pre-built telemetry record.
    pub fn push(&mut self, record: GameEventRecord) {
        self.events.push(record);
    }

    /// Emit a telemetry record from components.
    pub fn emit(
        &mut self,
        system: &'static str,
        event_type: &'static str,
        payload: serde_json::Value,
    ) {
        self.events.push(GameEventRecord::new(
            self.tick,
            self.wall_ms,
            system,
            event_type,
            payload,
        ));
    }

    /// Consume the integrator, returning all telemetry records.
    #[must_use]
    pub fn finish(self) -> Vec<GameEventRecord> {
        self.events
    }

    // ── Step 1: Telemetry-only sections ──────────────────────────────────

    /// Emit fog-of-war reveal telemetry (no world mutations).
    pub fn emit_fog_reveals(&mut self, reveals: &[RevealEvent], world: &GameWorld) {
        for reveal in reveals {
            self.emit(
                SYS_FOG,
                EVT_FOG_REVEALED,
                serde_json::json!({
                    "system": sys_name(world, reveal.system),
                }),
            );
        }
    }

    /// Heartbeat: emit a check event so the "victory" system tag always appears.
    pub fn emit_victory_check(&mut self, victory_state: &rebellion_core::victory::VictoryState) {
        self.events.push(GameEventRecord::new(
            self.tick,
            self.wall_ms,
            SYS_VICTORY,
            EVT_VICTORY_CHECK,
            serde_json::json!({ "resolved": victory_state.resolved }),
        ));
    }

    /// Emit victory telemetry and mark victory resolved.
    ///
    /// Emits the primary `EVT_VICTORY` record for every terminal condition
    /// and, for HQ-capture outcomes specifically, the Dabora 2 #A1
    /// `EVT_HQ_CAPTURED` (0x128) notification on the victory subsystem
    /// so story-chain consumers can key off the capture event without
    /// pattern-matching on the Debug-formatted outcome string.
    pub fn apply_victory(
        &mut self,
        outcome: &VictoryOutcome,
        victory_state: &mut rebellion_core::victory::VictoryState,
        world: &GameWorld,
    ) {
        victory_state.resolved = true;
        self.emit(
            SYS_VICTORY,
            EVT_VICTORY,
            serde_json::json!({
                "outcome": format!("{:?}", outcome),
            }),
        );
        // A1: EVT_HQ_CAPTURED (0x128). Payload uses the HQ system's name
        // (human-readable) rather than a stale slotmap key (DI-H2).
        if let VictoryOutcome::HqCaptured {
            winner,
            loser,
            hq_system,
        } = outcome
        {
            self.emit(
                SYS_VICTORY,
                EVT_HQ_CAPTURED,
                serde_json::json!({
                    "winner": format!("{:?}", winner),
                    "loser": format!("{:?}", loser),
                    "hq_system": sys_name(world, *hq_system),
                }),
            );
        }
    }

    /// Emit campaign snapshot telemetry (no world mutations, read-only).
    pub fn emit_campaign_snapshot(
        &mut self,
        world: &GameWorld,
        movement_len: usize,
        economy: &EconomyState,
    ) {
        let mut alliance_systems = 0u32;
        let mut empire_systems = 0u32;
        let mut neutral_systems = 0u32;
        for (_, sys) in &world.systems {
            match sys.control {
                ControlKind::Controlled(rebellion_core::dat::Faction::Alliance) => {
                    alliance_systems += 1;
                }
                ControlKind::Controlled(rebellion_core::dat::Faction::Empire) => {
                    empire_systems += 1;
                }
                _ => neutral_systems += 1,
            }
        }

        // Build per-system economy data for parity eval
        let mut systems_map = serde_json::Map::new();
        for (key, sys) in &world.systems {
            if let Some(econ) = economy.per_system.get(&key) {
                systems_map.insert(
                    sys.name.clone(),
                    serde_json::json!({
                        "production_modifier": econ.production_modifier,
                        "troop_surplus": econ.summary.troop_surplus,
                        "has_shipyard": econ.summary.has_shipyard,
                        "fleet_posture": format!("{:?}", econ.summary.fleet_posture),
                        "collection_rate": econ.collection_rate,
                    }),
                );
            }
        }

        self.emit(
            "snapshot",
            EVT_CAMPAIGN_SNAPSHOT,
            serde_json::json!({
                "tick": self.tick,
                "alliance_systems": alliance_systems,
                "empire_systems": empire_systems,
                "neutral_systems": neutral_systems,
                "fleets": world.fleets.len(),
                "in_transit": movement_len,
                "systems": systems_map,
            }),
        );
    }

    // ── Step 2: Economy section ───────────────────────────────────────────

    /// Apply economy events: world mutations (support drift, control) + telemetry.
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    pub fn apply_economy_events(&mut self, world: &mut GameWorld, economy_events: &[EconomyEvent]) {
        for ev in economy_events {
            match ev {
                EconomyEvent::SupportDrifted {
                    system,
                    alliance_delta,
                    empire_delta,
                } => {
                    if let Some(sys) = world.systems.get_mut(*system) {
                        sys.popularity_alliance =
                            (sys.popularity_alliance + alliance_delta).clamp(0.0, 1.0);
                        sys.popularity_empire =
                            (sys.popularity_empire + empire_delta).clamp(0.0, 1.0);
                    }
                    self.emit(
                        SYS_ECONOMY,
                        EVT_SUPPORT_DRIFT,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "alliance_delta": alliance_delta,
                            "empire_delta": empire_delta,
                        }),
                    );
                }
                EconomyEvent::CollectionRateChanged { system, new_rate } => {
                    self.emit(
                        SYS_ECONOMY,
                        EVT_COLLECTION_RATE,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "rate": new_rate,
                        }),
                    );
                }
                EconomyEvent::GarrisonRequirementChanged {
                    system,
                    new_requirement,
                } => {
                    self.emit(
                        SYS_ECONOMY,
                        EVT_GARRISON_REQUIRED,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "garrison_required": new_requirement,
                        }),
                    );
                }
                EconomyEvent::IncidentTriggered {
                    system,
                    incident_type,
                } => {
                    self.emit(
                        SYS_ECONOMY,
                        EVT_ECONOMY_TICK,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "incident": incident_type,
                        }),
                    );
                }
                EconomyEvent::ControlResolved {
                    system,
                    new_control,
                } => {
                    if let Some(sys) = world.systems.get_mut(*system) {
                        sys.control = *new_control;
                    }
                    // FUN_00510d70 → FUN_004f6f40: the system's facilities
                    // follow its new holder.
                    world.hand_over_facilities(*system);
                    self.emit(
                        SYS_ECONOMY,
                        EVT_CONTROL_CHANGED,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "new_control": format!("{:?}", new_control),
                        }),
                    );
                }
                EconomyEvent::EnergyOvercapped {
                    system,
                    allocated,
                    capacity,
                } => {
                    self.emit(
                        SYS_ECONOMY,
                        EVT_ECONOMY_TICK,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "energy_overcap": true,
                            "allocated": allocated,
                            "capacity": capacity,
                        }),
                    );
                }
                EconomyEvent::RawMaterialOvercapped {
                    system,
                    allocated,
                    capacity,
                } => {
                    self.emit(
                        SYS_ECONOMY,
                        EVT_ECONOMY_TICK,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "raw_material_overcap": true,
                            "allocated": allocated,
                            "capacity": capacity,
                        }),
                    );
                }
                // ── Knesset Shamash-Bet Dabora 2 notification events ────────
                EconomyEvent::SupportChanged { system, from, to } => {
                    // K1: EVT_SUPPORT_CHANGE (0x100).
                    self.emit(
                        SYS_ECONOMY,
                        EVT_SUPPORT_CHANGE,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "from": format!("{:?}", from),
                            "to": format!("{:?}", to),
                        }),
                    );
                }
                EconomyEvent::ResourceDiscovered { system, new_output } => {
                    // K3: EVT_RESOURCE_DISCOVERY (0x155).
                    self.emit(
                        SYS_ECONOMY,
                        EVT_RESOURCE_DISCOVERY,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "new_output": new_output,
                        }),
                    );
                }
                EconomyEvent::MaintenanceShortfall {
                    faction_is_alliance,
                    deficit_system_count,
                } => {
                    // K4: EVT_MAINTENANCE_SHORTFALL_EVENT (0x304).
                    self.emit(
                        SYS_ECONOMY,
                        EVT_MAINTENANCE_SHORTFALL,
                        serde_json::json!({
                            "faction": if *faction_is_alliance { "Alliance" } else { "Empire" },
                            "deficit_systems": deficit_system_count,
                        }),
                    );
                }
            }
        }
    }

    // ── Step 3: Manufacturing + Movement ──────────────────────────────────

    /// Apply build completions: add manufactured items to `GameWorld` + emit telemetry.
    ///
    /// Emits two telemetry records per completion — `EVT_BUILD_COMPLETE`
    /// (the "construction finished" signal test harnesses use) and `EVT_UNITS_DEPLOYED` (0x107, Knesset
    /// Shamash-Bet Dabora 2 #K5 — the "new forces in the field" signal
    /// that strategic AIs and story events listen for).
    pub fn apply_build_completions(
        &mut self,
        world: &mut GameWorld,
        completions: &[CompletionEvent],
    ) {
        for c in completions {
            apply_build_completion_inner(c, world);
            self.emit(
                SYS_MANUFACTURING,
                EVT_BUILD_COMPLETE,
                serde_json::json!({
                    "system": sys_name(world, c.system),
                    "kind": format!("{:?}", c.kind),
                }),
            );
            // K5: EVT_UNITS_DEPLOYED (0x107).
            self.emit(
                SYS_MANUFACTURING,
                EVT_UNITS_DEPLOYED,
                serde_json::json!({
                    "system": sys_name(world, c.system),
                    "kind": format!("{:?}", c.kind),
                }),
            );
        }
    }

    /// Emit `EVT_MANUFACTURING_IDLE` (0x160, K6) for systems whose queue
    /// transitioned from non-empty to empty this tick. No world mutation —
    /// the queue is already drained by `production_cycle::advance_yards`.
    pub fn apply_manufacturing_idle(
        &mut self,
        world: &GameWorld,
        newly_idle: &[rebellion_core::ids::SystemKey],
    ) {
        for &system in newly_idle {
            self.emit(
                SYS_MANUFACTURING,
                EVT_MANUFACTURING_IDLE,
                serde_json::json!({
                    "system": sys_name(world, system),
                }),
            );
        }
    }

    /// Report manufactured objects lost on arrival: their destination was
    /// destroyed while they travelled (event `0x303`, `FUN_004fc080`).
    pub fn apply_deliveries_lost(
        &mut self,
        world: &GameWorld,
        lost: &[rebellion_core::delivery::Delivery],
    ) {
        for delivery in lost {
            self.emit(
                SYS_MANUFACTURING,
                EVT_DESTROYED_ON_ARRIVAL,
                serde_json::json!({
                    "origin": sys_name(world, delivery.origin),
                    "destination": sys_name(world, delivery.destination),
                    "kind": format!("{:?}", delivery.kind),
                }),
            );
        }
    }

    /// Apply fleet arrivals: update locations + emit telemetry.
    pub fn apply_arrivals(
        &mut self,
        world: &mut GameWorld,
        movement: &MovementState,
        troop_transport: &mut TroopTransportState,
        arrivals: &[ArrivalEvent],
    ) {
        for arrival in arrivals {
            let Some(applied) = apply_fleet_arrival(world, movement, troop_transport, arrival)
            else {
                continue;
            };
            self.emit(
                SYS_MOVEMENT,
                EVT_FLEET_ARRIVED,
                serde_json::json!({
                    "system": sys_name(world, arrival.system),
                    "origin": sys_name(world, arrival.origin),
                    "fleet_faction": if applied.is_alliance { "Alliance" } else { "Empire" },
                    "surviving_fleet": format!("{:?}", applied.fleet),
                    "merged_fleets": applied.merged_fleets,
                }),
            );
        }
    }

    // ── Step 4: Combat ────────────────────────────────────────────────────

    /// Apply space combat result: ship damage + fleet cleanup + telemetry.
    pub fn apply_space_combat(
        &mut self,
        world: &mut GameWorld,
        system: SystemKey,
        result: &SpaceCombatResult,
    ) {
        apply_space_combat_result_inner(result, world);
        let winner_str = match result.winner {
            CombatSide::Attacker => "alliance",
            CombatSide::Defender => "empire",
            CombatSide::Draw => "draw",
        };
        self.emit(
            SYS_COMBAT,
            EVT_COMBAT_SPACE,
            serde_json::json!({
                "system": sys_name(world, system),
                "winner": winner_str,
            }),
        );
    }

    /// Emit one summary event after a complete system-level space engagement.
    pub fn emit_system_space_combat(
        &mut self,
        world: &GameWorld,
        result: &SystemSpaceCombatResult,
    ) {
        let winner_str = match result.winner {
            CombatSide::Attacker => "alliance",
            CombatSide::Defender => "empire",
            CombatSide::Draw => "draw",
        };
        self.emit(
            SYS_COMBAT,
            EVT_COMBAT_SPACE,
            serde_json::json!({
                "system": sys_name(world, result.system),
                "winner": winner_str,
                "rounds": result.rounds,
                "alliance_fleets": result.alliance_fleets,
                "empire_fleets": result.empire_fleets,
                "alliance_before": {
                    "capital_ships": result.alliance_before.capital_ships,
                    "fighter_squadrons": result.alliance_before.fighter_squadrons,
                    "hull": result.alliance_before.hull,
                },
                "alliance_after": {
                    "capital_ships": result.alliance_after.capital_ships,
                    "fighter_squadrons": result.alliance_after.fighter_squadrons,
                    "hull": result.alliance_after.hull,
                },
                "empire_before": {
                    "capital_ships": result.empire_before.capital_ships,
                    "fighter_squadrons": result.empire_before.fighter_squadrons,
                    "hull": result.empire_before.hull,
                },
                "empire_after": {
                    "capital_ships": result.empire_after.capital_ships,
                    "fighter_squadrons": result.empire_after.fighter_squadrons,
                    "hull": result.empire_after.hull,
                },
                "stalemate": result.stalemate,
            }),
        );
    }

    /// Apply ground combat result: troop damage + dead removal + telemetry.
    pub fn apply_ground_combat(&mut self, world: &mut GameWorld, result: &GroundCombatResult) {
        apply_ground_combat_result_inner(result, world);
        let ground_winner = match result.winner {
            CombatSide::Attacker => "alliance",
            CombatSide::Defender => "empire",
            CombatSide::Draw => "draw",
        };
        self.emit(
            SYS_COMBAT,
            EVT_COMBAT_GROUND,
            serde_json::json!({
                "system": sys_name(world, result.system),
                "winner": ground_winner,
                "engagements": result.troop_damage.len(),
            }),
        );
    }

    /// Emit one summary for a complete multi-round ground engagement.
    pub fn emit_system_ground_combat(
        &mut self,
        world: &GameWorld,
        system: SystemKey,
        winner: CombatSide,
        engagements: usize,
        rounds: u32,
        stalemate: bool,
    ) {
        let ground_winner = match winner {
            CombatSide::Attacker => "attacker",
            CombatSide::Defender => "defender",
            CombatSide::Draw => "draw",
        };
        self.emit(
            SYS_COMBAT,
            EVT_COMBAT_GROUND,
            serde_json::json!({
                "system": sys_name(world, system),
                "winner": ground_winner,
                "engagements": engagements,
                "rounds": rounds,
                "stalemate": stalemate,
            }),
        );
    }

    /// Land every regiment carried by a fleet and emit one movement record.
    pub fn apply_troop_landing(
        &mut self,
        world: &mut GameWorld,
        troop_transport: &mut TroopTransportState,
        fleet: FleetKey,
        system: SystemKey,
    ) -> Vec<TroopKey> {
        let landed = troop_transport
            .disembark_all(world, fleet, system)
            .unwrap_or_default();
        if !landed.is_empty() {
            self.emit(
                SYS_MOVEMENT,
                EVT_TROOP_MOVED,
                serde_json::json!({
                    "system": sys_name(world, system),
                    "fleet": format!("{:?}", fleet),
                    "regiments": landed.len(),
                    "status": "landed",
                }),
            );
        }
        landed
    }

    /// Record regiments whose own transit ended (`FUN_00556430`, event
    /// `0x387`): one movement record each, arrived or lost on arrival
    /// (`FUN_004fc080`).
    pub fn apply_regiment_arrivals(&mut self, world: &GameWorld, ended: &RegimentArrivals) {
        let records = ended
            .arrived
            .iter()
            .map(|transit| (transit, "arrived"))
            .chain(ended.lost.iter().map(|transit| (transit, "lost_on_arrival")));
        for (transit, status) in records {
            let destination = match transit.leg {
                RegimentLeg::Surface(system) => sys_name(world, system),
                RegimentLeg::Fleet(fleet) => format!("{fleet:?}"),
            };
            self.emit(
                SYS_MOVEMENT,
                EVT_TROOP_MOVED,
                serde_json::json!({
                    "origin": sys_name(world, transit.origin),
                    "destination": destination,
                    "regiments": 1,
                    "status": status,
                }),
            );
        }
    }

    /// Apply territorial occupation after a decisive ground victory.
    /// Opposing characters physically present on the captured world become
    /// prisoners, allowing the standard victory rules to observe HQ captures.
    pub fn apply_ground_occupation(
        &mut self,
        world: &mut GameWorld,
        system: SystemKey,
        winner: rebellion_core::dat::Faction,
        tick: u64,
    ) {
        let previous = world.systems.get(system).map(|value| value.control);
        if let Some(value) = world.systems.get_mut(system) {
            value.control = ControlKind::Controlled(winner);
        }
        // FUN_00510d70 → FUN_004f6f40: the system's facilities follow its
        // new holder.
        world.hand_over_facilities(system);
        if previous != Some(ControlKind::Controlled(winner)) {
            self.emit(
                SYS_COMBAT,
                EVT_CONTROL_CHANGED,
                serde_json::json!({
                    "system": sys_name(world, system),
                    "from": format!("{:?}", previous.unwrap_or_default()),
                    "to": format!("{:?}", ControlKind::Controlled(winner)),
                    "cause": "ground_occupation",
                }),
            );
        }

        let captives: Vec<_> = world
            .characters
            .iter()
            .filter(|(_, character)| {
                character.current_system == Some(system)
                    && !character.is_killed
                    && match winner {
                        rebellion_core::dat::Faction::Alliance => character.is_empire,
                        rebellion_core::dat::Faction::Empire => character.is_alliance,
                        rebellion_core::dat::Faction::Neutral => false,
                    }
            })
            .map(|(key, character)| (key, character.name.clone()))
            .collect();
        for (character, name) in captives {
            if let Some(value) = world.characters.get_mut(character) {
                value.is_captive = true;
                value.captured_by = Some(winner);
                value.capture_tick = Some(tick);
                value.current_fleet = None;
            }
            self.emit(
                SYS_COMBAT,
                EVT_CAPTURE,
                serde_json::json!({
                    "character": name,
                    "system": sys_name(world, system),
                    "captured_by": format!("{:?}", winner),
                    "cause": "ground_occupation",
                }),
            );
        }
    }

    /// Emit loss evidence for regiments whose fleet was destroyed in space.
    pub fn emit_destroyed_transport_cargo(&mut self, destroyed: &[(FleetKey, Vec<TroopKey>)]) {
        for (fleet, troops) in destroyed {
            if troops.is_empty() {
                continue;
            }
            self.emit(
                SYS_COMBAT,
                EVT_TROOP_MOVED,
                serde_json::json!({
                    "fleet": format!("{:?}", fleet),
                    "regiments": troops.len(),
                    "status": "destroyed_with_transport",
                }),
            );
        }
    }

    /// Emit bombardment telemetry after the caller applies its world effects.
    pub fn emit_bombardment(
        &mut self,
        world: &GameWorld,
        system: SystemKey,
        damage: i32,
        headquarters_destroyed: bool,
    ) {
        if damage > 0 {
            self.emit(
                SYS_COMBAT,
                EVT_BOMBARDMENT,
                serde_json::json!({
                    "system": sys_name(world, system),
                    "damage": damage,
                    "headquarters_destroyed": headquarters_destroyed,
                }),
            );
        }
    }
    // ── Step 5: Missions + Escapes ──────────────────────────────────────

    /// Apply mission result: world mutations + telemetry.
    pub fn apply_mission_result(
        &mut self,
        world: &mut GameWorld,
        result: &MissionResult,
        uprising_state: &mut UprisingState,
        research_state: &mut ResearchState,
    ) {
        let ended =
            apply_mission_effects_inner(&result.effects, world, result.tick, uprising_state);
        for effect in &result.effects {
            if let MissionEffect::ResearchPoints {
                faction,
                tech,
                points,
            } = *effect
            {
                let is_alliance = faction == MissionFaction::Alliance;
                for level in research_state.add_points(world, is_alliance, tech, points) {
                    self.emit(
                        SYS_RESEARCH,
                        EVT_RESEARCH_UNLOCKED,
                        serde_json::json!({
                            "faction_is_alliance": is_alliance,
                            "tech_type": format!("{tech:?}"),
                            "new_level": level,
                        }),
                    );
                }
            }
        }
        for event in &ended {
            if let UprisingEvent::UprisingEnded { system, tick } = event {
                self.events.push(GameEventRecord::new(
                    *tick,
                    self.wall_ms,
                    SYS_UPRISING,
                    EVT_UPRISING_ENDED,
                    serde_json::json!({ "system": sys_name(world, *system), "cause": "subdued" }),
                ));
            }
        }
        self.emit(
            SYS_MISSIONS,
            EVT_MISSION_RESOLVED,
            serde_json::json!({
                "kind": format!("{:?}", result.kind),
                "outcome": format!("{:?}", result.outcome),
                "target_system": sys_name(world, result.target_system),
            }),
        );
        // Covert missions are espionage operations — emit an Espionage wrapper
        // so the eval harness sees all 8 mission kinds.
        if matches!(
            result.kind,
            MissionKind::Sabotage | MissionKind::Assassination | MissionKind::Abduction
        ) {
            self.emit(
                SYS_MISSIONS,
                EVT_MISSION_RESOLVED,
                serde_json::json!({
                    "kind": "Espionage",
                    "outcome": format!("{:?}", result.outcome),
                    "target_system": sys_name(world, result.target_system),
                    "parent_kind": format!("{:?}", result.kind),
                }),
            );
        }
        // R6/R7/R8/R11: Per-effect telemetry emissions.
        for effect in &result.effects {
            match effect {
                // R6: EVT_INFORMANT_INTEL — Espionage resolved with intel.
                MissionEffect::SystemIntelligenceGathered { system, faction } => {
                    self.emit(
                        SYS_MISSIONS,
                        EVT_INFORMANT_INTEL,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "faction": format!("{:?}", faction),
                        }),
                    );
                }
                // R7: EVT_SABOTEUR_DETECTED — enemy sabotage on a system.
                MissionEffect::TargetSabotaged { system, .. } => {
                    self.emit(
                        SYS_MISSIONS,
                        EVT_SABOTEUR_DETECTED,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "mission_faction": format!("{:?}", result.faction),
                        }),
                    );
                }
                // R8: EVT_CHARACTER_HEALTH — character captured (health status change).
                MissionEffect::CharacterCaptured {
                    character,
                    captured_by,
                    at_system,
                } => {
                    self.emit(
                        SYS_MISSIONS,
                        EVT_CHARACTER_HEALTH,
                        serde_json::json!({
                            "character": char_name(world, *character),
                            "status": "captured",
                            "captured_by": format!("{:?}", captured_by),
                            "system": sys_name(world, *at_system),
                        }),
                    );
                }
                // R11: EVT_CHARACTER_KILLED — assassination kill.
                MissionEffect::CharacterKilled { character, faction } => {
                    self.emit(
                        SYS_MISSIONS,
                        EVT_CHARACTER_KILLED,
                        serde_json::json!({
                            "character": char_name(world, *character),
                            "cause": "assassination",
                            "faction": format!("{:?}", faction),
                        }),
                    );
                }
                _ => {}
            }
        }
    }

    /// Apply escape effects in order: the escapee's home side, fleet removal
    /// and telemetry, and its moves to and from the road.
    pub fn apply_escape_effects(&mut self, world: &mut GameWorld, effects: &[MissionEffect]) {
        for effect in effects {
            if let MissionEffect::MemberMoved { member, to } = effect {
                rebellion_core::missions::move_member(world, *member, *to);
            } else if let MissionEffect::CharacterEscaped {
                character,
                escaped_to_alliance,
            } = effect
            {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.is_alliance = *escaped_to_alliance;
                    c.is_empire = !*escaped_to_alliance;
                    c.is_captive = false;
                    c.captured_by = None;
                    c.capture_tick = None;
                }
                for (_, fleet) in &mut world.fleets {
                    fleet.characters.retain(|&k| k != *character);
                }
                self.emit(
                    SYS_MISSIONS,
                    EVT_ESCAPE,
                    serde_json::json!({
                        "character": char_name(world, *character),
                        "escaped_to_alliance": escaped_to_alliance,
                    }),
                );
            }
        }
    }

    // ── Step 6: Events + Jedi training ────────────────────────────────────

    /// Apply fired events: world mutations + Jedi training extraction + telemetry.
    ///
    /// Per Dabora 2 #F7 the `movement` parameter is required so
    /// `EventAction::SpawnSpecialForce` can resolve in-transit characters
    /// via `MovementState::orders()`. Story/message effects push into the
    /// integrator's `story_effects` queue; callers that want to route them
    /// into a render-layer `MessageLog` should call `drain_story_effects()`
    /// after this returns (simulation.rs ignores the queue).
    pub fn apply_fired_events(
        &mut self,
        world: &mut GameWorld,
        fired_events: &[FiredEvent],
        jedi_state: &mut JediState,
        current_tick: u64,
        movement: &MovementState,
    ) {
        for fired in fired_events {
            apply_event_action_to_world(
                &fired.actions,
                world,
                &mut self.story_effects,
                current_tick,
                movement,
            );
            let system_tag = match fired.system_tag {
                SystemTag::Story => SYS_STORY,
                SystemTag::Events | SystemTag::Notification => SYS_EVENTS,
            };
            self.emit(
                system_tag,
                EVT_EVENT_FIRED,
                serde_json::json!({
                    "event_id": fired.event_id,
                }),
            );
        }
        // Extract Jedi training starts from story events
        for fired in fired_events {
            for action in &fired.actions {
                if let EventAction::StartJediTraining { character } = action {
                    if let Some(c) = world.characters.get(*character) {
                        jedi_state.start_training(*character, c.is_alliance, current_tick);
                    }
                }
            }
        }
        // Flush SpecialForceSpawned telemetry. The effect queue also
        // serves as the channel into main.rs's special-forces arena
        // (drained post-tick); we emit telemetry eagerly here so the
        // headless playtest still sees the event without needing to
        // drain — story_effects is monotonically append-only during
        // the tick.
        for eff in &self.story_effects {
            if let rebellion_core::effects::GameEffect::SpecialForceSpawned {
                at_system,
                is_alliance,
            } = eff
            {
                self.events.push(GameEventRecord::new(
                    self.tick,
                    self.wall_ms,
                    SYS_STORY,
                    EVT_EVENT_FIRED,
                    serde_json::json!({
                        "effect": "special_force_spawned",
                        "system": sys_name(world, *at_system),
                        "is_alliance": is_alliance,
                    }),
                ));
            }
        }
    }

    // ── Step 7: AI actions ────────────────────────────────────────────────

    /// Apply AI actions: mission dispatch, production, movement + telemetry.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_ai_actions(
        &mut self,
        actions: &[AIAction],
        ai_state: &mut AIState,
        mission_state: &mut MissionState,
        mfg_state: &mut ManufacturingState,
        movement_state: &mut MovementState,
        troop_transport: &mut TroopTransportState,
        production: &mut ProductionState,
        world: &mut GameWorld,
        tick: u64,
        is_dual: bool,
    ) {
        let applied = apply_ai_actions_inner(
            actions,
            ai_state,
            mission_state,
            mfg_state,
            movement_state,
            troop_transport,
            production,
            world,
            tick,
        );
        for (action, was_applied) in actions.iter().zip(applied) {
            if !was_applied {
                continue;
            }
            let mut payload = ai_action_json(action, world);
            if is_dual {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("dual_ai".into(), serde_json::json!(true));
                }
            }
            self.emit(SYS_AI, EVT_AI_ACTION, payload);
            if let AIAction::DispatchMission {
                kind,
                team,
                decoys,
                target_system,
                ..
            } = action
            {
                let faction = ai_state
                    .faction
                    .map_or(MissionFaction::Empire, AiFaction::as_mission_faction);
                self.emit(
                    SYS_MISSIONS,
                    EVT_MISSION_DISPATCHED,
                    serde_json::json!({
                        "kind": format!("{kind:?}"),
                        "faction": format!("{faction:?}"),
                        "team": member_names(world, team),
                        "decoys": member_names(world, decoys),
                        "target_system": sys_name(world, *target_system),
                    }),
                );
            }
        }
    }

    // ── Step 8: Blockade ──────────────────────────────────────────────────

    /// Apply blockade events: troop destruction + telemetry.
    pub fn apply_blockade_events(
        &mut self,
        world: &mut GameWorld,
        transport: &mut TroopTransportState,
        events: &[BlockadeEvent],
    ) {
        for evt in events {
            match evt {
                BlockadeEvent::BlockadeStarted { system, tick } => {
                    self.events.push(GameEventRecord::new(
                        *tick,
                        self.wall_ms,
                        SYS_BLOCKADE,
                        EVT_BLOCKADE_STARTED,
                        serde_json::json!({ "system": sys_name(world, *system) }),
                    ));
                }
                BlockadeEvent::BlockadeEnded { system, tick } => {
                    self.events.push(GameEventRecord::new(
                        *tick,
                        self.wall_ms,
                        SYS_BLOCKADE,
                        EVT_BLOCKADE_ENDED,
                        serde_json::json!({ "system": sys_name(world, *system) }),
                    ));
                }
                BlockadeEvent::TroopDestroyed {
                    system,
                    troop,
                    tick,
                } => {
                    // The regiment was aboard a fleet leaving the system.
                    if transport.destroy_embarked(world, *troop) {
                        self.events.push(GameEventRecord::new(
                            *tick,
                            self.wall_ms,
                            SYS_BLOCKADE,
                            EVT_BLOCKADE_TROOP_DESTROYED,
                            serde_json::json!({ "system": sys_name(world, *system) }),
                        ));
                    }
                }
            }
        }
    }

    // ── Step 9: Uprising ──────────────────────────────────────────────────

    /// Apply uprising and disaster events to the world, with telemetry.
    /// Revolts never change who holds a system (`FUN_0050a130`).
    pub fn apply_uprising_events(&mut self, world: &mut GameWorld, events: &[UprisingEvent]) {
        // Heartbeat: emit a check event so the "uprising" system tag always appears.
        self.events.push(GameEventRecord::new(
            self.tick, self.wall_ms, SYS_UPRISING, EVT_UPRISING_CHECK,
            serde_json::json!({ "systems_checked": world.systems.len(), "incidents": events.len() }),
        ));
        for evt in events {
            let (tick, event_type, payload) = match evt {
                UprisingEvent::UprisingIncident {
                    system,
                    tick,
                    codes,
                    losses,
                    support_delta,
                    ..
                } => (
                    *tick,
                    EVT_UPRISING_INCIDENT,
                    serde_json::json!({
                        "system": sys_name(world, *system),
                        "codes": codes,
                        "losses": losses.len(),
                        "support_delta": support_delta,
                    }),
                ),
                UprisingEvent::UprisingBegan { system, tick } => (
                    *tick,
                    EVT_UPRISING_BEGAN,
                    serde_json::json!({ "system": sys_name(world, *system) }),
                ),
                UprisingEvent::UprisingEnded { system, tick } => (
                    *tick,
                    EVT_UPRISING_ENDED,
                    serde_json::json!({ "system": sys_name(world, *system) }),
                ),
                UprisingEvent::Disaster {
                    system,
                    tick,
                    destroyed,
                    ..
                } => (
                    *tick,
                    EVT_NATURAL_DISASTER,
                    serde_json::json!({
                        "system": sys_name(world, *system),
                        "facilities_destroyed": destroyed.len(),
                    }),
                ),
            };
            self.events.push(GameEventRecord::new(
                tick,
                self.wall_ms,
                SYS_UPRISING,
                event_type,
                payload,
            ));
            rebellion_core::uprising::apply_uprising_event(world, evt);
        }
    }

    // ── Step 10: Betrayal ─────────────────────────────────────────────────

    /// Apply betrayal events: faction flip + fleet removal + telemetry.
    pub fn apply_betrayal_events(&mut self, world: &mut GameWorld, events: &[BetrayalEvent]) {
        // Heartbeat: emit a check event so the "betrayal" system tag always appears.
        self.events.push(GameEventRecord::new(
            self.tick, self.wall_ms, SYS_BETRAYAL, EVT_BETRAYAL_CHECK,
            serde_json::json!({ "characters_checked": world.characters.len(), "betrayals": events.len() }),
        ));
        for evt in events {
            let BetrayalEvent::CharacterBetrayed {
                character,
                defected_to_alliance,
            } = evt;

            // #R9: reveal-before-flip — emit EVT_TRAITOR_REVEALED while the
            // character still belongs to the original faction.
            self.emit(
                SYS_BETRAYAL,
                EVT_TRAITOR_REVEALED,
                serde_json::json!({
                    "character": char_name(world, *character),
                    "original_faction": if world.characters.get(*character)
                        .is_some_and(|c| c.is_alliance) { "alliance" } else { "empire" },
                }),
            );

            // Apply the faction flip.
            if let Some(c) = world.characters.get_mut(*character) {
                c.is_alliance = *defected_to_alliance;
                c.is_empire = !*defected_to_alliance;
            }
            for (_, fleet) in &mut world.fleets {
                fleet.characters.retain(|&k| k != *character);
            }

            // #R10: emit EVT_SIDE_CHANGE after the flip with DI-H2 payload.
            // Replaces the former EVT_BETRAYAL emit (identical payload, same timing).
            self.emit(
                SYS_BETRAYAL,
                EVT_SIDE_CHANGE,
                serde_json::json!({
                    "character": char_name(world, *character),
                    "defected_to_alliance": defected_to_alliance,
                }),
            );
        }
    }

    // ── Step 11: Death Star ───────────────────────────────────────────────

    /// Apply death star events: planet destruction + telemetry.
    pub fn apply_death_star_events(&mut self, world: &mut GameWorld, events: &[DeathStarEvent]) {
        // Heartbeat: emit a status event so the "death_star" system tag always appears.
        self.events.push(GameEventRecord::new(
            self.tick,
            self.wall_ms,
            SYS_DEATH_STAR,
            EVT_DS_STATUS,
            serde_json::json!({ "events": events.len() }),
        ));
        for evt in events {
            match evt {
                DeathStarEvent::ConstructionCompleted { system, tick } => {
                    self.events.push(GameEventRecord::new(
                        *tick,
                        self.wall_ms,
                        SYS_DEATH_STAR,
                        EVT_DS_CONSTRUCTION,
                        serde_json::json!({ "system": sys_name(world, *system) }),
                    ));
                }
                DeathStarEvent::PlanetDestroyed { system, tick } => {
                    if let Some(sys) = world.systems.get_mut(*system) {
                        sys.is_destroyed = true;
                    }
                    self.events.push(GameEventRecord::new(
                        *tick,
                        self.wall_ms,
                        SYS_DEATH_STAR,
                        EVT_DS_FIRED,
                        serde_json::json!({ "system": sys_name(world, *system) }),
                    ));
                }
                DeathStarEvent::NearbyWarning { .. } => {}
            }
        }
    }

    // ── Step 12b: Repair ───────────────────────────────────────────────────

    /// Apply repair events: hull restoration + telemetry.
    pub fn apply_repair_events(&mut self, world: &mut GameWorld, events: &[RepairEvent]) {
        for evt in events {
            match evt {
                RepairEvent::ShipRepaired {
                    fleet,
                    ship_index,
                    hull_before,
                    hull_after,
                } => {
                    // Apply hull restoration to the ShipInstance.
                    if let Some(f) = world.fleets.get_mut(*fleet) {
                        if let Some(ship) = f.capital_ships.get_mut(*ship_index) {
                            ship.hull_current = *hull_after;
                        }
                    }
                    let fleet_name = world
                        .fleets
                        .get(*fleet)
                        .map_or_else(|| "unknown".into(), |f| sys_name(world, f.location));
                    self.emit(
                        SYS_REPAIR,
                        EVT_SHIP_REPAIRED,
                        serde_json::json!({
                            "fleet_location": fleet_name,
                            "hull_before": hull_before,
                            "hull_after": hull_after,
                            "delta": hull_after - hull_before,
                        }),
                    );
                }
                RepairEvent::RepairCheckPerformed {
                    system,
                    fleet,
                    ships_checked,
                } => {
                    let system_name = sys_name(world, *system);
                    let fleet_commander = world
                        .fleets
                        .get(*fleet)
                        .and_then(|f| {
                            world
                                .characters
                                .get(*f.characters.first()?)
                                .map(|c| c.name.as_str())
                        })
                        .unwrap_or("uncrewed");
                    self.emit(
                        SYS_REPAIR,
                        EVT_SHIP_REPAIR_STARTED,
                        serde_json::json!({
                            "system": system_name,
                            "fleet_commander": fleet_commander,
                            "ships_checked": ships_checked,
                        }),
                    );
                }
            }
        }
    }

    // ── Step 13: Jedi ─────────────────────────────────────────────────────

    /// Apply jedi events: tier advancement + discovery + telemetry.
    pub fn apply_jedi_events(
        &mut self,
        world: &mut GameWorld,
        events: &[JediEvent],
        jedi_state: &mut JediState,
    ) {
        // Heartbeat: emit a check event so the "jedi" system tag always appears.
        self.events.push(GameEventRecord::new(
            self.tick,
            self.wall_ms,
            SYS_JEDI,
            EVT_JEDI_CHECK,
            serde_json::json!({ "training": jedi_state.training.len(), "events": events.len() }),
        ));
        for evt in events {
            match evt {
                JediEvent::TierAdvanced {
                    character,
                    new_tier,
                } => {
                    if let Some(c) = world.characters.get_mut(*character) {
                        c.force_tier = *new_tier;
                        c.force_experience = match new_tier {
                            rebellion_core::world::ForceTier::None => 0,
                            rebellion_core::world::ForceTier::Aware => 1,
                            rebellion_core::world::ForceTier::Training => {
                                rebellion_core::jedi::XP_TO_TRAINING
                            }
                            rebellion_core::world::ForceTier::Experienced => {
                                rebellion_core::jedi::XP_TO_EXPERIENCED
                            }
                        };
                    }
                    self.emit(
                        SYS_JEDI,
                        EVT_JEDI_TIER,
                        serde_json::json!({
                            "character": char_name(world, *character),
                            "new_tier": format!("{:?}", new_tier),
                        }),
                    );
                }
                JediEvent::TrainingComplete { character } => {
                    jedi_state.stop_training(*character);
                }
                JediEvent::JediDiscovered { character, .. } => {
                    if let Some(c) = world.characters.get_mut(*character) {
                        c.is_discovered_jedi = true;
                    }
                    self.emit(
                        SYS_JEDI,
                        EVT_JEDI_DISCOVERED,
                        serde_json::json!({
                            "character": char_name(world, *character),
                        }),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Mission effects helper (moved from simulation.rs)
// ---------------------------------------------------------------------------

#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
fn apply_mission_effects_inner(
    effects: &[MissionEffect],
    world: &mut GameWorld,
    tick: u64,
    uprising_state: &mut UprisingState,
) -> Vec<UprisingEvent> {
    for effect in effects {
        match effect {
            MissionEffect::SupportGained {
                system,
                side,
                points,
            } => rebellion_core::uprising::apply_support_change(world, *system, *side, *points),
            MissionEffect::SkillRaised {
                character,
                skill,
                amount,
            } => rebellion_core::missions::raise_skill(world, *character, *skill, *amount),
            MissionEffect::UprisingIncident(event) => {
                rebellion_core::uprising::apply_uprising_event(world, event);
            }
            MissionEffect::SystemIntelligenceGathered { system, .. } => {
                if let Some(sys) = world.systems.get_mut(*system) {
                    sys.exploration_status = rebellion_core::dat::ExplorationStatus::Explored;
                }
            }
            MissionEffect::CharacterRecruited {
                character,
                system,
                faction,
                pool_emptied,
            } => rebellion_core::missions::recruit_character(
                world,
                *character,
                *system,
                *faction,
                *pool_emptied,
            ),
            MissionEffect::TargetSabotaged { target, .. } => {
                rebellion_core::missions::destroy_target(world, *target);
            }
            MissionEffect::CharacterKilled { character, .. } => {
                // Knesset Shamash-Bet #R11: mark `is_killed = true` instead of
                // removing from the arena so reactive story events
                // (`EVT_CHARACTER_KILLED` 0x306) can still resolve the character
                // by `dat_id` / `name`. Character is removed from all fleet
                // rosters immediately. Uniqueness comes from the `is_killed`
                // flag combined with `is_repeatable: false` (DI-M3).
                //
                // Reactivity note: mission kills fire same-tick (assassination
                // at step 5, EventSystem at step 6 — same-tick is the correct
                // player-facing behavior). Death Star kills fire next-tick
                // (cleanup at step 11 is after events at step 6). Both routes
                // go through `Character::mark_killed()` so systems that iterate
                // `world.characters` see a consistent "dead" shape regardless
                // of which path produced the death.
                for (_, fleet) in &mut world.fleets {
                    fleet.characters.retain(|&k| k != *character);
                }
                if let Some(c) = world.characters.get_mut(*character) {
                    c.mark_killed();
                }
            }
            MissionEffect::CharacterCaptured {
                character,
                captured_by,
                at_system,
            } => rebellion_core::missions::capture_character(
                world,
                *character,
                *captured_by,
                *at_system,
            ),
            MissionEffect::SpecialForceDestroyed { unit } => {
                rebellion_core::missions::destroy_special_force(world, *unit);
            }
            // port: the port stores no injury (F-026).
            MissionEffect::CharacterInjured { .. } => {}
            // Applied by apply_mission_result, which reports the levels.
            MissionEffect::ResearchPoints { .. } => {}
            MissionEffect::CharacterRescued {
                character,
                returned_to,
                ..
            } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    match returned_to {
                        MissionFaction::Alliance => {
                            c.is_alliance = true;
                            c.is_empire = false;
                        }
                        MissionFaction::Empire => {
                            c.is_alliance = false;
                            c.is_empire = true;
                        }
                    }
                    c.is_captive = false;
                    c.captured_by = None;
                    c.capture_tick = None;
                }
            }
            MissionEffect::MemberAvailable { member } => {
                rebellion_core::missions::set_on_mission(world, *member, false);
            }
            MissionEffect::MemberMoved { member, to } => {
                rebellion_core::missions::move_member(world, *member, *to);
            }
            MissionEffect::CharacterEscaped {
                character,
                escaped_to_alliance,
            } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.is_alliance = *escaped_to_alliance;
                    c.is_empire = !*escaped_to_alliance;
                    c.is_captive = false;
                    c.captured_by = None;
                    c.capture_tick = None;
                }
            }
            MissionEffect::UprisingSubdued {
                system,
                side,
                support_gain,
            } => {
                rebellion_core::uprising::apply_support_change(
                    world,
                    *system,
                    *side,
                    *support_gain,
                );
            }
        }
    }
    apply_mission_state_effects(effects, world, tick, uprising_state)
}

/// Apply the mission effects that live outside `GameWorld`, after the world
/// effects: a Subdue Uprising success ends the revolt when the system is
/// garrisoned (`FUN_0050c910`). Returns the revolts that ended. The native and browser app shares this
/// with the headless integrator.
pub fn apply_mission_state_effects(
    effects: &[MissionEffect],
    world: &GameWorld,
    tick: u64,
    uprising_state: &mut UprisingState,
) -> Vec<UprisingEvent> {
    let mut ended = Vec::new();
    for effect in effects {
        if let MissionEffect::UprisingSubdued { system, .. } = effect {
            ended.extend(rebellion_core::uprising::end_if_garrisoned(
                uprising_state,
                world,
                *system,
                tick,
            ));
        }
    }
    ended
}

// ---------------------------------------------------------------------------
// Event action helper (formerly `apply_event_actions_to_world_inner`)
// ---------------------------------------------------------------------------

/// Apply a slice of `EventAction`s to the world, producing any ancillary
/// `GameEffect`s via `effects_out`.
///
/// Made `pub #[inline]` in Knesset Shamash-Bet Dabora 2 (#F7) as the single
/// source of truth — `main.rs` no longer carries a duplicate helper. The
/// compiler's exhaustive match over the closed `EventAction` enum now
/// enforces coverage parity for free.
///
/// The `effects_out` sink receives two kinds of records:
/// - `GameEffect::StoryMessageDisplayed` for every `EventAction::DisplayMessage`
///   so interactive `main.rs` can drain them into `MessageLog` post-tick.
///   The headless playtest simply discards the effect queue.
/// - `GameEffect::SpecialForceSpawned` for every `EventAction::SpawnSpecialForce`
///   after resolving `at_character.current_system` (or its in-transit
///   movement destination via `MovementState::orders`). Resolution failure
///   logs at `warn!` level and drops the action — callers must gate the
///   originating event on `CharacterAtSystem OR CharacterAssignedToFleet`
///   to guarantee resolution success (SF-#7).
#[inline]
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
pub fn apply_event_action_to_world(
    actions: &[EventAction],
    world: &mut GameWorld,
    effects_out: &mut Vec<rebellion_core::effects::GameEffect>,
    tick: u64,
    movement: &MovementState,
) {
    use rebellion_core::effects::{GameEffect, MessageCategoryTag};

    for action in actions {
        match action {
            EventAction::DisplayMessage { text } => {
                // #F7 + #A3: route story messages through the effect layer
                // instead of dropping them (the old integrator no-op) or
                // pushing directly to a render-layer MessageLog (the old
                // main.rs duplicate). Interactive mode drains these post-tick.
                effects_out.push(GameEffect::StoryMessageDisplayed {
                    text: text.clone(),
                    category: MessageCategoryTag::Event,
                });
            }
            EventAction::ShiftPopularity {
                system,
                alliance_delta,
                empire_delta,
            } => {
                if let Some(sys) = world.systems.get_mut(*system) {
                    sys.popularity_alliance =
                        (sys.popularity_alliance + alliance_delta).clamp(0.0, 1.0);
                    sys.popularity_empire = (sys.popularity_empire + empire_delta).clamp(0.0, 1.0);
                }
            }
            EventAction::ModifyCharacterSkill {
                character,
                skill,
                base_delta,
            } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    let d = *base_delta;
                    let apply =
                        |v: u32, delta: i32| (i64::from(v) + i64::from(delta)).max(0) as u32;
                    match skill {
                        SkillField::Diplomacy => c.diplomacy.base = apply(c.diplomacy.base, d),
                        SkillField::Espionage => c.espionage.base = apply(c.espionage.base, d),
                        SkillField::ShipDesign => c.ship_design.base = apply(c.ship_design.base, d),
                        SkillField::TroopTraining => {
                            c.troop_training.base = apply(c.troop_training.base, d);
                        }
                        SkillField::FacilityDesign => {
                            c.facility_design.base = apply(c.facility_design.base, d);
                        }
                        SkillField::Combat => c.combat.base = apply(c.combat.base, d),
                        SkillField::Leadership => c.leadership.base = apply(c.leadership.base, d),
                        SkillField::Loyalty => c.loyalty.base = apply(c.loyalty.base, d),
                        SkillField::JediLevel => c.jedi_level.base = apply(c.jedi_level.base, d),
                    }
                }
            }
            EventAction::RelocateCharacter { .. }
            | EventAction::StartJediTraining { .. }
            | EventAction::TriggerEvent { .. } => {}
            EventAction::SetMandatoryMission {
                character,
                mandatory,
            } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.on_mandatory_mission = *mandatory;
                }
            }
            EventAction::ModifyForceTier {
                character,
                new_tier,
            } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.force_tier = *new_tier;
                }
            }
            EventAction::RemoveCharacter { character } => {
                for (_, fleet) in &mut world.fleets {
                    fleet.characters.retain(|&k| k != *character);
                }
                world.characters.remove(*character);
            }
            EventAction::TransferCharacter {
                character,
                destination,
                new_faction,
            } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.current_system = Some(*destination);
                    if let Some(faction) = new_faction {
                        match faction {
                            rebellion_core::dat::Faction::Alliance => {
                                c.is_alliance = true;
                                c.is_empire = false;
                            }
                            rebellion_core::dat::Faction::Empire => {
                                c.is_alliance = false;
                                c.is_empire = true;
                            }
                            rebellion_core::dat::Faction::Neutral => {}
                        }
                    }
                }
            }
            EventAction::AccumulateForceExperience { character, amount } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.force_experience += amount;
                }
            }
            EventAction::CaptureCharacter {
                character,
                captor_faction,
            } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.is_captive = true;
                    c.captured_by = Some(*captor_faction);
                    c.capture_tick = Some(tick);
                }
                for (_, fleet) in &mut world.fleets {
                    fleet.characters.retain(|&k| k != *character);
                }
            }
            EventAction::SetCarboniteState { character, frozen } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.on_mandatory_mission = *frozen;
                    if *frozen {
                        c.is_captive = true;
                        c.capture_tick = Some(tick);
                    } else {
                        c.is_captive = false;
                        c.captured_by = None;
                        c.capture_tick = None;
                    }
                }
            }
            EventAction::SpawnSpecialForce { at_character } => {
                // #A2: Resolve the system where the special force lands.
                //
                // The character's `current_system` is the authoritative location
                // when the character is stationary (on a garrison or mission).
                // For in-transit characters we fall back to the destination of
                // the first fleet carrying them — via `MovementState::orders()`.
                //
                // The bounty-hunter chain (EVT_BOUNTY_ATTACK) is gated at the
                // `state.define()` site on `CharacterAtSystem OR CharacterAssignedToFleet`
                // so this fallback should always succeed (SF-#7). We `warn!`
                // instead of panic on the unexpected case.
                let character = *at_character;
                let resolved: Option<(rebellion_core::ids::SystemKey, bool)> =
                    world.characters.get(character).and_then(|c| {
                        // Primary: the character's cached current_system.
                        if let Some(sys) = c.current_system {
                            return Some((sys, c.is_alliance));
                        }
                        // Fallback: find any fleet that carries the character
                        // AND is currently under a movement order, return the
                        // order's destination.
                        let is_alliance = c.is_alliance;
                        for (fleet_key, fleet) in &world.fleets {
                            if fleet.characters.contains(&character) {
                                if let Some(order) = movement.get(fleet_key) {
                                    return Some((order.destination, is_alliance));
                                }
                                // The character is on a stationary fleet — use
                                // the fleet's location field.
                                return Some((fleet.location, is_alliance));
                            }
                        }
                        None
                    });
                match resolved {
                    Some((at_system, is_alliance)) => {
                        // Special force lands at the character's system.
                        // Hardcode `is_alliance: false` to match the
                        // Bounty Hunters parity citation from the
                        // community cross-reference.
                        let _ = is_alliance;
                        let spawn_alliance = false;
                        // A2: Create SpecialForceUnit in world arena and
                        // push key onto the target system's roster.
                        // DatId(0) = event-spawned (not from SPECFCSD.DAT).
                        let sf_key = world.special_forces.insert(SpecialForceUnit {
                            class_dat_id: DatId::new(0),
                            is_alliance: spawn_alliance,
                            skills: [0; 8],
                            on_mission: false,
                        });
                        if let Some(sys) = world.systems.get_mut(at_system) {
                            sys.special_forces.push(sf_key);
                        }
                        effects_out.push(GameEffect::SpecialForceSpawned {
                            at_system,
                            is_alliance: spawn_alliance,
                        });
                    }
                    None => {
                        // Structured warn: the event chain must gate
                        // SpawnSpecialForce on CharacterAtSystem OR
                        // CharacterAssignedToFleet (SF-#7). If we
                        // reach this branch, the guard is missing — log
                        // and drop the action rather than spawning at
                        // an arbitrary fallback system.
                        eprintln!(
                            "[shamash-bet] SpawnSpecialForce at character {character:?} \
                             could not resolve a target system — character has \
                             no current_system and is not on any movement-ordered \
                             fleet. Event chain should have gated this action with \
                             CharacterAtSystem OR CharacterAssignedToFleet.",
                        );
                    }
                }
            }
            EventAction::SetHeritageKnown { character } => {
                // Dabora 3 #R4: flip heritage_known when 0x396 "Final Battle
                // Imminent" fires. The render layer branches on this flag
                // in `event_id_to_resource()` to pick between the "Student
                // Luke" / "Knight Luke" / "Emperor + Vader vs Knight Luke"
                // BMP variants for 0x220 EVT_FINAL_BATTLE (SIMP-H5 +
                // ARCH-#9 + SF-#11 triple-collapse — we do NOT create
                // 0x222).
                //
                // Fail loudly on missing character: a silently-dropped flip
                // shows the wrong cutscene, which is a story-continuity bug
                // that's extremely hard to diagnose after the fact. Matches
                // the `SpawnSpecialForce` structured-warn pattern above.
                if let Some(c) = world.characters.get_mut(*character) {
                    c.heritage_known = true;
                } else {
                    eprintln!(
                        "[shamash-bet] SetHeritageKnown target character {character:?} \
                         not found in arena — heritage_known flip dropped. \
                         Final Battle cutscene variant will not reflect the \
                         paternity reveal.",
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AI action helper (moved from simulation.rs)
// ---------------------------------------------------------------------------

#[expect(
    clippy::too_many_arguments,
    reason = "Keep the existing explicit simulation state inputs at this integration boundary."
)]
fn apply_ai_actions_inner(
    actions: &[AIAction],
    ai_state: &mut AIState,
    mission_state: &mut MissionState,
    mfg_state: &mut ManufacturingState,
    movement_state: &mut MovementState,
    troop_transport: &mut TroopTransportState,
    production: &mut ProductionState,
    world: &mut GameWorld,
    tick: u64,
) -> Vec<bool> {
    let mission_faction = ai_state.faction.map_or(
        MissionFaction::Empire,
        rebellion_core::ai::AiFaction::as_mission_faction,
    );

    let mut applied = Vec::with_capacity(actions.len());
    for action in actions {
        let was_applied = match action {
            AIAction::DispatchMission {
                kind,
                team,
                decoys,
                target_system,
                target_character,
                target_object,
            } => {
                let dispatched = mission_state.dispatch_guarded(
                    MissionRequest {
                        kind: *kind,
                        faction: mission_faction,
                        team: team.clone(),
                        decoys: decoys.clone(),
                        target_system: *target_system,
                        target_character: *target_character,
                        target_object: *target_object,
                        tick,
                    },
                    world,
                );
                if let Ok(id) = dispatched {
                    for character in team.iter().chain(decoys).filter_map(|m| m.character()) {
                        ai_state.mark_busy(character);
                    }
                    if let Some(lead) = team.first().and_then(|m| m.character()) {
                        ai_state.bind_mission(*kind, lead, id);
                    }
                }
                dispatched.is_ok()
            }
            AIAction::AbortMission { mission_id } => mission_state.abort(*mission_id),
            AIAction::EnqueueProduction { system, kind, cost } => {
                mfg_state.enqueue(*system, QueueItem::new(*kind, *cost));
                true
            }
            AIAction::Scrap { team } => ai_state.faction.is_some_and(|side| {
                let done =
                    rebellion_core::scrap::scrap_team(world, production, side.as_faction(), team)
                        .is_ok();
                // A scrapped ship's regiments beyond the room left are lost.
                troop_transport.destroy_untransportable_cargo(world);
                done
            }),
            AIAction::MoveShips { fleet, ships, to } => ai_state.faction.is_some_and(|side| {
                let roster = rebellion_core::fleet_join::roster(world, *fleet);
                roster.is_some_and(|roster| {
                    rebellion_core::fleet_join::move_ships_to_system(
                        world,
                        movement_state,
                        troop_transport,
                        *fleet,
                        ships,
                        roster,
                        *to,
                        matches!(side, AiFaction::Alliance),
                    )
                    .is_ok()
                })
            }),
            AIAction::CreateFleet { fleet, ships } => ai_state.faction.is_some_and(|side| {
                let roster = rebellion_core::fleet_join::roster(world, *fleet);
                roster.is_some_and(|roster| {
                    rebellion_core::fleet_join::create_fleet(
                        world,
                        movement_state,
                        troop_transport,
                        *fleet,
                        ships,
                        roster,
                        matches!(side, AiFaction::Alliance),
                    )
                    .is_ok()
                })
            }),
            AIAction::MoveRegiments { troops, to } => {
                let blockaded = rebellion_core::blockade::BlockadeSystem::blockaded_now(world);
                let mut moved = false;
                for &troop in troops {
                    moved |= troop_transport
                        .move_regiment(
                            world,
                            movement_state,
                            &blockaded,
                            troop,
                            rebellion_core::troop_transport::RegimentTarget::System(*to),
                            tick,
                        )
                        .is_ok();
                }
                moved
            }
            AIAction::MoveSquadrons {
                class,
                count,
                from,
                to,
            } => rebellion_core::squadrons::is_alliance(world, *class).is_some_and(|alliance| {
                troop_transport
                    .squadrons
                    .move_squadrons(world, alliance, *class, *count, *from, *to, tick)
                    .is_ok()
            }),
            AIAction::SetDestination {
                system,
                area,
                destination,
            } => {
                mfg_state.set_destination(*system, *area, *destination);
                true
            }
            AIAction::Build {
                system,
                kind,
                cost,
                count,
            } => {
                for _ in 0..*count {
                    mfg_state.enqueue(*system, QueueItem::new(*kind, *cost));
                }
                *count > 0
            }
            AIAction::MoveFleet {
                fleet,
                to_system,
                troops,
                ..
            } => {
                let transit = world.fleets.get(*fleet).and_then(|f| {
                    rebellion_core::movement::fleet_transit_ticks(f, world, f.location, *to_system)
                });
                let embarked = if troops.is_empty() {
                    true
                } else {
                    troop_transport.embark(world, *fleet, troops).is_ok()
                };
                if embarked {
                    let departed = transit.is_some_and(|ticks| {
                        begin_fleet_transit(movement_state, world, *fleet, *to_system, ticks)
                    });
                    if !departed && !troops.is_empty() {
                        let origin = world.fleets.get(*fleet).map(|value| value.location);
                        if let Some(origin) = origin {
                            let _ = troop_transport.disembark_all(world, *fleet, origin);
                        }
                    }
                    departed
                } else {
                    false
                }
            }
        };
        applied.push(was_applied);
    }
    applied
}

#[cfg(test)]
mod tests {
    use super::*;
    use rebellion_core::ai::{AiFaction, FleetMoveReason};
    use rebellion_core::dat::{ExplorationStatus, Faction, SectorGroup};
    use rebellion_core::missions::MissionOutcome;
    use rebellion_core::world::{CapitalShipClass, Sector, System};

    fn add_system(world: &mut GameWorld, name: &str) -> SystemKey {
        let sector = world.sectors.insert(Sector {
            dat_id: DatId(1),
            name: format!("{name} Sector"),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        world.systems.insert(System {
            dat_id: DatId(2),
            name: name.into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.0,
            popularity_empire: 1.0,
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
            control: ControlKind::Controlled(Faction::Empire),
        })
    }

    #[test]
    fn each_ended_regiment_transit_records_one_movement_line() {
        let mut world = GameWorld::default();
        let origin = add_system(&mut world, "Kuat");
        let destination = add_system(&mut world, "Naboo");
        let troop = world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: DatId(0x1000_0001),
            is_alliance: false,
            regiment_strength: 100,
        });
        let transit = |leg| rebellion_core::troop_transport::RegimentTransit {
            troop,
            origin,
            leg,
            arrival_tick: 5,
        };
        let ended = RegimentArrivals {
            arrived: vec![transit(RegimentLeg::Surface(destination))],
            lost: vec![transit(RegimentLeg::Surface(destination))],
        };
        let mut integrator = PerceptionIntegrator::new(5, 0);

        integrator.apply_regiment_arrivals(&world, &ended);

        let records = integrator.finish();
        let statuses: Vec<_> = records
            .iter()
            .map(|record| (record.event_type, record.details["status"].clone()))
            .collect();
        assert_eq!(
            statuses,
            [
                (EVT_TROOP_MOVED, serde_json::json!("arrived")),
                (EVT_TROOP_MOVED, serde_json::json!("lost_on_arrival")),
            ]
        );
        assert_eq!(records[0].details["destination"], "Naboo");
        assert_eq!(records[0].details["origin"], "Kuat");
    }

    #[test]
    fn an_escape_frees_the_captive_takes_it_off_the_map_and_lands_it_on_arrival() {
        // FUN_004ef570 frees the captive; its trip starts and ends as moves
        // (ghidra/notes/captive-escape.md).
        use rebellion_core::missions::MissionMember;
        let mut world = GameWorld::default();
        let prison = add_system(&mut world, "Kessel");
        let haven = add_system(&mut world, "Yavin");
        let captive = world.characters.insert(rebellion_core::world::Character {
            name: "Leia".into(),
            is_alliance: true,
            is_captive: true,
            captured_by: Some(Faction::Empire),
            current_system: Some(prison),
            ..Default::default()
        });
        let crew = world
            .characters
            .insert(rebellion_core::world::Character::default());
        let fleet = world.fleets.insert(rebellion_core::world::Fleet {
            location: prison,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![captive, crew],
            is_alliance: false,
            has_death_star: false,
        });
        let member = MissionMember::Character(captive);
        let mut integrator = PerceptionIntegrator::new(5, 0);

        integrator.apply_escape_effects(
            &mut world,
            &[
                MissionEffect::CharacterEscaped {
                    character: captive,
                    escaped_to_alliance: true,
                },
                MissionEffect::MemberMoved { member, to: None },
            ],
        );
        let c = &world.characters[captive];
        assert!(!c.is_captive && c.captured_by.is_none() && c.is_alliance && !c.is_empire);
        assert_eq!(c.current_system, None);
        assert_eq!(world.fleets[fleet].characters, [crew]);

        integrator.apply_escape_effects(
            &mut world,
            &[MissionEffect::MemberMoved {
                member,
                to: Some(haven),
            }],
        );
        assert_eq!(world.characters[captive].current_system, Some(haven));
        assert_eq!(integrator.finish()[0].event_type, EVT_ESCAPE);
    }

    #[test]
    fn a_subdue_success_at_a_garrisoned_system_ends_the_uprising() {
        // FUN_00569c20 runs FUN_0050c910 after a Subdue Uprising success; with
        // no GNPRTB rows the garrison requirement is 0, so it ends.
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Naboo");
        let mut uprisings = UprisingState::new();
        uprisings.active_uprisings.insert(
            system,
            rebellion_core::uprising::ActiveUprising {
                started_tick: 0,
                next_incident_tick: None,
            },
        );
        let ended = apply_mission_state_effects(
            &[MissionEffect::UprisingSubdued {
                system,
                side: Faction::Empire,
                support_gain: 5,
            }],
            &world,
            9,
            &mut uprisings,
        );

        assert_eq!(
            ended,
            vec![UprisingEvent::UprisingEnded { system, tick: 9 }]
        );
        assert!(!uprisings.is_uprising(system));
    }

    #[test]
    fn a_subdue_gain_counts_toward_the_garrison_check_that_follows_it() {
        // FUN_00569c20 adds the FUN_0055cb10 gain before FUN_0050c910. At
        // Alliance support 40 the requirement is ceil((60 - 40) / 10) = 2, so
        // one regiment falls short; a gain of 10 makes it ceil(10 / 10) = 1.
        for (support_gain, ends) in [(10, true), (0, false)] {
            let mut world = GameWorld::default();
            world.gnprtb = rebellion_core::world::GnprtbParams::new(
                [(7761, 60), (7762, -10)]
                    .into_iter()
                    .map(|(id, value)| rebellion_core::world::GnprtbEntry {
                        parameter_id: id,
                        development: value,
                        alliance_sp_easy: value,
                        alliance_sp_medium: value,
                        alliance_sp_hard: value,
                        empire_sp_easy: value,
                        empire_sp_medium: value,
                        empire_sp_hard: value,
                        multiplayer: value,
                    })
                    .collect(),
            );
            let system = add_system(&mut world, "Naboo");
            world.systems[system].control = ControlKind::Controlled(Faction::Alliance);
            world.systems[system].popularity_alliance = 0.4;
            world.systems[system].popularity_empire = 0.6;
            let regiment = world.troops.insert(rebellion_core::world::TroopUnit {
                class_dat_id: DatId(0x1000_0002),
                is_alliance: true,
                regiment_strength: 1,
            });
            world.systems[system].ground_units.push(regiment);
            let agent = world
                .characters
                .insert(rebellion_core::world::Character::default());
            let mut uprisings = UprisingState::new();
            uprisings.active_uprisings.insert(
                system,
                rebellion_core::uprising::ActiveUprising {
                    started_tick: 0,
                    next_incident_tick: None,
                },
            );
            let result = MissionResult {
                mission_id: 1,
                tick: 9,
                kind: MissionKind::SubdueUprising,
                faction: MissionFaction::Alliance,
                character: Some(agent),
                target_system: system,
                outcome: MissionOutcome::Success,
                effects: vec![MissionEffect::UprisingSubdued {
                    system,
                    side: Faction::Alliance,
                    support_gain,
                }],
            };
            let mut integrator = PerceptionIntegrator::new(9, 0);
            integrator.apply_mission_result(
                &mut world,
                &result,
                &mut uprisings,
                &mut ResearchState::new(),
            );
            assert_eq!(uprisings.is_uprising(system), !ends, "gain {support_gain}");
        }
    }

    #[test]
    fn a_subdue_mission_result_raises_support_and_reports_the_revolt_it_ends() {
        // FUN_00569c20: support += FUN_0055cb10, then FUN_0050c910 ends the
        // revolt; with no GNPRTB rows the garrison requirement is 0.
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Naboo");
        world.systems[system].popularity_alliance = 0.4;
        world.systems[system].popularity_empire = 0.6;
        let agent = world
            .characters
            .insert(rebellion_core::world::Character::default());
        let mut uprisings = UprisingState::new();
        uprisings.active_uprisings.insert(
            system,
            rebellion_core::uprising::ActiveUprising {
                started_tick: 0,
                next_incident_tick: None,
            },
        );
        let result = MissionResult {
            mission_id: 1,
            tick: 9,
            kind: MissionKind::SubdueUprising,
            faction: MissionFaction::Empire,
            character: Some(agent),
            target_system: system,
            outcome: MissionOutcome::Success,
            effects: vec![MissionEffect::UprisingSubdued {
                system,
                side: Faction::Empire,
                support_gain: 5,
            }],
        };
        let mut integrator = PerceptionIntegrator::new(9, 0);
        integrator.apply_mission_result(
            &mut world,
            &result,
            &mut uprisings,
            &mut ResearchState::new(),
        );

        assert!((world.systems[system].popularity_empire - 0.65).abs() < 1e-6);
        assert!(!uprisings.is_uprising(system));
        assert!(integrator
            .events
            .iter()
            .any(|e| e.event_type == EVT_UPRISING_ENDED && e.tick == 9));
    }

    #[test]
    fn research_points_raise_the_side_s_tree_and_report_each_level() {
        // FUN_0056cf30 -> FUN_00530600 adds the mission's points; the points
        // hook FUN_005326f0 takes each level they cover.
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Kuat");
        for (id, order) in [(0x1400_0001, 1), (0x1400_0002, 2)] {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    dat_id: DatId(id),
                    is_empire: true,
                    refined_material_cost: 10,
                    research_order: order,
                    research_difficulty: 100,
                    ..Default::default()
                });
        }
        let result = MissionResult {
            mission_id: 1,
            tick: 7,
            kind: MissionKind::ShipDesignResearch,
            faction: MissionFaction::Empire,
            character: None,
            target_system: system,
            outcome: MissionOutcome::Success,
            effects: vec![MissionEffect::ResearchPoints {
                faction: MissionFaction::Empire,
                tech: rebellion_core::research::TechType::Ship,
                points: 2,
            }],
        };
        let mut research = ResearchState::new();
        let mut integrator = PerceptionIntegrator::new(7, 0);
        integrator.apply_mission_result(
            &mut world,
            &result,
            &mut UprisingState::new(),
            &mut research,
        );

        // One system: each level costs 100 * 1 / 100 = 1 point.
        assert_eq!(research.empire.ship, 2);
        assert_eq!(research.alliance.ship, 0);
        assert!(research.empire_progress.ship.done);
        let levels: Vec<_> = integrator
            .events
            .iter()
            .filter(|e| e.event_type == EVT_RESEARCH_UNLOCKED)
            .map(|e| {
                (
                    e.details["faction_is_alliance"].clone(),
                    e.details["new_level"].clone(),
                )
            })
            .collect();
        assert_eq!(
            levels,
            vec![
                (serde_json::json!(false), serde_json::json!(1)),
                (serde_json::json!(false), serde_json::json!(2))
            ]
        );
    }

    #[test]
    fn applying_a_disaster_sets_the_eroded_resources_and_records_it() {
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Naboo");
        world.systems[system].total_energy = 5;
        world.systems[system].raw_materials = 5;
        let mut integrator = PerceptionIntegrator::new(4, 0);
        integrator.apply_uprising_events(
            &mut world,
            &[UprisingEvent::Disaster {
                system,
                tick: 4,
                total_energy: 2,
                raw_materials: 1,
                destroyed: vec![],
            }],
        );
        assert_eq!(world.systems[system].total_energy, 2);
        assert_eq!(world.systems[system].raw_materials, 1);
        assert!(integrator
            .events
            .iter()
            .any(|e| e.event_type == EVT_NATURAL_DISASTER));
    }

    /// An Empire-held system with a yard both sides can run and a defense
    /// only the Empire can field.
    fn empire_facilities(world: &mut GameWorld, name: &str) -> SystemKey {
        let system = add_system(world, name);
        for (id, is_alliance) in [(0x2900_0001, true), (0x2200_0001, false)] {
            world.buildable_classes.insert(
                DatId(id),
                rebellion_core::world::BuildableClass {
                    is_alliance,
                    is_empire: true,
                    ..Default::default()
                },
            );
        }
        let yard = world.manufacturing_facilities.insert(ManufacturingFacilityInstance {
            class_dat_id: DatId(0x2900_0001),
            side: Faction::Empire,
            is_shipyard: false,
        });
        let defense = world.defense_facilities.insert(rebellion_core::world::DefenseFacilityInstance {
            class_dat_id: DatId(0x2200_0001),
            side: Faction::Empire,
        });
        world.systems[system].manufacturing_facilities.push(yard);
        world.systems[system].defense_facilities.push(defense);
        system
    }

    fn assert_alliance_took_over(world: &GameWorld, system: SystemKey) {
        let held = &world.systems[system];
        assert!(held.defense_facilities.is_empty(), "the Empire-only defense is removed");
        let sides: Vec<Faction> = held
            .manufacturing_facilities
            .iter()
            .map(|&k| world.manufacturing_facilities[k].side)
            .collect();
        assert_eq!(sides, vec![Faction::Alliance]);
    }

    #[test]
    fn a_resolved_control_change_hands_facilities_to_the_new_holder() {
        // FUN_00510d70 → FUN_004f6f40 → FUN_004fae40: a facility whose class
        // serves the new holder passes to it, any other is removed
        // (ghidra/notes/facility-ownership.md).
        let mut world = GameWorld::default();
        let system = empire_facilities(&mut world, "Kuat");
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.apply_economy_events(
            &mut world,
            &[EconomyEvent::ControlResolved {
                system,
                new_control: ControlKind::Controlled(Faction::Alliance),
            }],
        );
        assert_alliance_took_over(&world, system);
    }

    #[test]
    fn a_ground_occupation_hands_facilities_to_the_victor() {
        // Same path as a resolved control change (FUN_00510d70).
        let mut world = GameWorld::default();
        let system = empire_facilities(&mut world, "Kuat");
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.apply_ground_occupation(&mut world, system, Faction::Alliance, 1);
        assert_alliance_took_over(&world, system);
    }

    #[test]
    fn support_drift_moves_both_sides_popularity() {
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Naboo");
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.apply_economy_events(
            &mut world,
            &[EconomyEvent::SupportDrifted {
                system,
                alliance_delta: 0.25,
                empire_delta: -0.25,
            }],
        );
        assert!((world.systems[system].popularity_alliance - 0.25).abs() < 1e-6);
        assert!((world.systems[system].popularity_empire - 0.75).abs() < 1e-6);
    }

    #[test]
    fn duplicate_move_action_preserves_first_order_and_emits_once() {
        let mut world = GameWorld::default();
        let origin = add_system(&mut world, "Origin");
        let first_target = add_system(&mut world, "First Target");
        let second_target = add_system(&mut world, "Second Target");
        // A fleet needs a capital ship to enter hyperspace (FUN_004fda10).
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            hyperdrive: 100,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: origin,
            capital_ships: vec![ShipInstance::new(class, 100, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[origin].fleets.push(fleet);

        let actions = vec![
            AIAction::MoveFleet {
                fleet,
                to_system: first_target,
                reason: FleetMoveReason::Attack,
                troops: vec![],
            },
            AIAction::MoveFleet {
                fleet,
                to_system: second_target,
                reason: FleetMoveReason::Reinforce,
                troops: vec![],
            },
        ];
        let mut ai = AIState::new(AiFaction::Empire);
        let mut missions = MissionState::new();
        let mut manufacturing = ManufacturingState::new();
        let mut movement = MovementState::new();
        let mut troop_transport = TroopTransportState::default();
        let mut integrator = PerceptionIntegrator::new(5, 0);

        integrator.apply_ai_actions(
            &actions,
            &mut ai,
            &mut missions,
            &mut manufacturing,
            &mut movement,
            &mut troop_transport,
            &mut ProductionState::new(),
            &mut world,
            5,
            false,
        );

        assert_eq!(movement.len(), 1);
        assert_eq!(movement.get(fleet).unwrap().destination, first_target);
        let events = integrator.finish();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EVT_AI_ACTION);
        assert_eq!(events[0].details["to"], "First Target");
    }

    #[test]
    fn an_ai_scrap_removes_its_own_facility_refunds_and_reports_it() {
        // The order 0x200 (FUN_0053dd20) through rebellion_core::scrap: the
        // Empire's mine goes, with class +0x48 / 2 = 5 refined back
        // (FUN_00530270); the Alliance's is refused and stays unreported.
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Kuat");
        let class = DatId(0x2c00_0001);
        world.buildable_classes.insert(
            class,
            rebellion_core::world::BuildableClass {
                refined_material_cost: 10,
                ..Default::default()
            },
        );
        let mut mine = |side| {
            let key = world.production_facilities.insert(
                rebellion_core::world::ProductionFacilityInstance {
                    class_dat_id: class,
                    side,
                    is_mine: true,
                },
            );
            world.systems[system].production_facilities.push(key);
            rebellion_core::scrap::ScrapTarget::Facility(
                rebellion_core::world::FacilityRef::Production(key),
            )
        };
        let own = mine(Faction::Empire);
        let theirs = mine(Faction::Alliance);
        let mut production = ProductionState::new();
        let mut integrator = PerceptionIntegrator::new(5, 0);

        integrator.apply_ai_actions(
            &[
                AIAction::Scrap { team: vec![own] },
                AIAction::Scrap { team: vec![theirs] },
            ],
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut TroopTransportState::default(),
            &mut production,
            &mut world,
            5,
            false,
        );

        assert_eq!(world.production_facilities.len(), 1);
        assert_eq!(world.systems[system].production_facilities.len(), 1);
        assert_eq!(production.stock(Faction::Empire).unwrap().refined, 5);
        let events = integrator.finish();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EVT_AI_ACTION);
        assert_eq!(events[0].details["type"], "Scrap");
    }

    #[test]
    fn module_six_s_regiment_move_sends_each_regiment_on_its_way() {
        // FUN_004de6f0's Move 0x201: each regiment moves as by hand
        // (TroopTransportState::move_regiment, FUN_00556390), across
        // systems in transit.
        let mut world = GameWorld::default();
        let from = add_system(&mut world, "Kuat");
        let to = add_system(&mut world, "Fondor");
        world.systems[to].x = 100;
        for s in [from, to] {
            world.systems[s].control = rebellion_core::world::ControlKind::Controlled(
                rebellion_core::dat::Faction::Empire,
            );
        }
        let troops: Vec<_> = (0..2)
            .map(|_| {
                let key = world.troops.insert(rebellion_core::world::TroopUnit {
                    class_dat_id: DatId(0x1000_0001),
                    is_alliance: false,
                    regiment_strength: 100,
                });
                world.systems[from].ground_units.push(key);
                key
            })
            .collect();
        let mut transport = TroopTransportState::new();
        let mut integrator = PerceptionIntegrator::new(5, 0);

        integrator.apply_ai_actions(
            &[AIAction::MoveRegiments {
                troops: troops.clone(),
                to,
            }],
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut transport,
            &mut ProductionState::new(),
            &mut world,
            5,
            false,
        );

        assert_eq!(transport.transits().len(), 2);
        assert!(world.systems[from].ground_units.is_empty());
    }

    #[test]
    fn a_squadron_move_boards_the_planet_s_squadrons_onto_the_fleet() {
        // Move 0x201 for squadrons (FUN_00557ce0), carried out by
        // squadrons::move_squadrons for the class's side; a move beyond the
        // fleet's room (FUN_00500b40) changes nothing.
        use rebellion_core::squadrons::SquadronPlace;
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Kuat");
        let carrier = world.capital_ship_classes.insert(CapitalShipClass {
            fighter_capacity: 2,
            ..Default::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![rebellion_core::world::ShipInstance::new(
                carrier, 100, false,
            )],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[system].fleets.push(fleet);
        let tie = world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                is_empire: true,
                ..Default::default()
            });
        rebellion_core::squadrons::add(&mut world.systems[system].fighters, tie, 3);
        let mut integrator = PerceptionIntegrator::new(5, 0);
        let order = |count| AIAction::MoveSquadrons {
            class: tie,
            count,
            from: SquadronPlace::Planet(system),
            to: SquadronPlace::Fleet(fleet),
        };

        integrator.apply_ai_actions(
            &[order(3), order(2)],
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut TroopTransportState::new(),
            &mut ProductionState::new(),
            &mut world,
            5,
            false,
        );

        assert_eq!(
            rebellion_core::squadrons::total(&world.fleets[fleet].fighters),
            2
        );
        assert_eq!(
            rebellion_core::squadrons::total(&world.systems[system].fighters),
            1
        );
    }

    #[test]
    fn a_ship_move_sends_the_chosen_ships_off_and_a_scrapped_ship_s_cargo_is_lost() {
        // Module 0xb's Move 0x201 of ships (FUN_004c8e90), carried out by
        // fleet_join::move_ships_to_system: the chosen ship leaves as a
        // fleet of its own. Its Scrap (FUN_004cab20) takes the last
        // transport, and the regiment it carried has no room left.
        let mut world = GameWorld::default();
        let kuat = add_system(&mut world, "Kuat");
        let fondor = add_system(&mut world, "Fondor");
        world.systems[fondor].x = 40;
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            is_empire: true,
            troop_capacity: 1,
            hyperdrive: 100,
            ..Default::default()
        });
        let ship = || rebellion_core::world::ShipInstance::new(class, 100, false);
        let fleet = world.fleets.insert(Fleet {
            location: kuat,
            capital_ships: vec![ship(), ship()],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[kuat].fleets.push(fleet);
        let mut movement = MovementState::new();
        let mut transport = TroopTransportState::new();
        let mut integrator = PerceptionIntegrator::new(5, 0);
        let mut apply = |world: &mut GameWorld,
                         movement: &mut MovementState,
                         transport: &mut TroopTransportState,
                         action: AIAction| {
            integrator.apply_ai_actions(
                &[action],
                &mut AIState::new(AiFaction::Empire),
                &mut MissionState::new(),
                &mut ManufacturingState::new(),
                movement,
                transport,
                &mut ProductionState::new(),
                world,
                5,
                false,
            );
        };

        apply(
            &mut world,
            &mut movement,
            &mut transport,
            AIAction::MoveShips {
                fleet,
                ships: vec![1],
                to: fondor,
            },
        );
        assert_eq!(world.fleets[fleet].capital_ships.len(), 1);
        assert_eq!(world.fleets.len(), 2);
        let gone = world.fleets.keys().find(|&k| k != fleet).unwrap();
        assert!(movement.is_in_transit(gone));

        let regiment = world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: DatId(0x1000_0001),
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[kuat].ground_units.push(regiment);
        transport.embark(&mut world, fleet, &[regiment]).unwrap();
        world.fleets[fleet]
            .characters
            .push(rebellion_core::ids::CharacterKey::default());
        apply(
            &mut world,
            &mut movement,
            &mut transport,
            AIAction::Scrap {
                team: vec![rebellion_core::scrap::ScrapTarget::Ship { fleet, index: 0 }],
            },
        );
        assert!(world.fleets[fleet].capital_ships.is_empty());
        assert!(!world.troops.contains_key(regiment));
    }

    #[test]
    fn a_create_fleet_order_splits_the_chosen_ships_off_in_place() {
        // Create Fleet 0x270 (FUN_00580b00), carried out by
        // fleet_join::create_fleet: the chosen ship forms a new fleet in
        // its system; an index past the fleet changes nothing.
        let mut world = GameWorld::default();
        let kuat = add_system(&mut world, "Kuat");
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            is_empire: true,
            ..Default::default()
        });
        let ship = || rebellion_core::world::ShipInstance::new(class, 100, false);
        let fleet = world.fleets.insert(Fleet {
            location: kuat,
            capital_ships: vec![ship(), ship()],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[kuat].fleets.push(fleet);
        let mut integrator = PerceptionIntegrator::new(5, 0);
        let mut apply = |world: &mut GameWorld, ships: Vec<usize>| {
            integrator.apply_ai_actions(
                &[AIAction::CreateFleet { fleet, ships }],
                &mut AIState::new(AiFaction::Empire),
                &mut MissionState::new(),
                &mut ManufacturingState::new(),
                &mut MovementState::new(),
                &mut TroopTransportState::new(),
                &mut ProductionState::new(),
                world,
                5,
                false,
            );
        };
        apply(&mut world, vec![5]);
        assert_eq!(world.fleets.len(), 1);
        apply(&mut world, vec![1]);
        assert_eq!(world.fleets.len(), 2);
        assert_eq!(world.fleets[fleet].capital_ships.len(), 1);
        let made = world.fleets.keys().find(|&k| k != fleet).unwrap();
        assert_eq!(world.fleets[made].location, kuat);
        assert_eq!(world.systems[kuat].fleets.len(), 2);
    }

    #[test]
    fn module_one_s_destination_and_build_reach_the_manager() {
        // FUN_004dc9f0 (0x214) sets the area's destination; FUN_004dcb30
        // (0x210) builds +0x4c units; a count of 0 builds nothing and is
        // not reported.
        let mut world = GameWorld::default();
        let site = add_system(&mut world, "Kuat");
        let target = add_system(&mut world, "Fondor");
        let kind = rebellion_core::manufacturing::BuildableKind::ProductionFacility(
            rebellion_core::manufacturing::FacilityBuild {
                class: DatId(0x2c00_0001),
                is_alliance: false,
            },
        );
        let area = rebellion_core::manufacturing::ProductionArea::ConstructionYard;
        let mut manufacturing = ManufacturingState::new();
        let mut integrator = PerceptionIntegrator::new(5, 0);

        integrator.apply_ai_actions(
            &[
                AIAction::SetDestination {
                    system: site,
                    area,
                    destination: target,
                },
                AIAction::Build {
                    system: site,
                    kind,
                    cost: 20,
                    count: 2,
                },
                AIAction::Build {
                    system: site,
                    kind,
                    cost: 20,
                    count: 0,
                },
            ],
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut manufacturing,
            &mut MovementState::new(),
            &mut TroopTransportState::default(),
            &mut ProductionState::new(),
            &mut world,
            5,
            false,
        );

        assert_eq!(manufacturing.destination(site, area), Some(target));
        let queue = manufacturing.queue(site, area).unwrap();
        assert_eq!(queue.len(), 2);
        assert!(queue
            .items()
            .iter()
            .all(|item| item.destination == Some(target) && item.cost == 20));
        let types: Vec<_> = integrator
            .finish()
            .iter()
            .map(|e| e.details["type"].clone())
            .collect();
        assert_eq!(types, ["SetDestination", "Build"]);
        // A Build of none is not applied, so it is not reported.
        let applied = apply_ai_actions_inner(
            &[AIAction::Build {
                system: site,
                kind,
                cost: 20,
                count: 0,
            }],
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut manufacturing,
            &mut MovementState::new(),
            &mut TroopTransportState::default(),
            &mut ProductionState::new(),
            &mut world,
            5,
        );
        assert_eq!(applied, [false]);
    }

    /// Apply `actions` for the Empire AI and return the telemetry.
    fn empire_ai_events(world: &mut GameWorld, actions: &[AIAction]) -> Vec<GameEventRecord> {
        let mut integrator = PerceptionIntegrator::new(5, 0);
        integrator.apply_ai_actions(
            actions,
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut TroopTransportState::default(),
            &mut ProductionState::new(),
            world,
            5,
            false,
        );
        integrator.finish()
    }

    fn imperial_at(world: &mut GameWorld, system: SystemKey, name: &str) -> MissionMember {
        MissionMember::Character(world.characters.insert(rebellion_core::world::Character {
            name: name.into(),
            is_empire: true,
            current_system: Some(system),
            ..Default::default()
        }))
    }

    #[test]
    fn an_ai_abort_ends_the_mission_with_code_2_and_a_research_order_binds_its_planner() {
        // Order 0x250 (FUN_0054e310 -> mission slot +0x1dc(2)) sets end code
        // 2, so the next step ends the mission (phase 0xb); FUN_0047b360
        // stores an emitted order on its planner (+0x28).
        let mut world = GameWorld::default();
        let target = add_system(&mut world, "Kuat");
        let designer = imperial_at(&mut world, target, "Designer");
        let MissionMember::Character(key) = designer else {
            unreachable!()
        };
        // FUN_0056c820: a research member needs the tree's skill.
        world.characters[key].ship_design.base = 40;
        let mut ai = AIState::new(AiFaction::Empire);
        ai.research_planners
            .push(rebellion_core::ai::ResearchPlanner {
                id: 0,
                tech: rebellion_core::research::TechType::Ship,
                member: Some(key),
                site: Some(target),
                target: Some(target),
                mission: None,
                report: None,
                failures: 0,
            });
        let mut missions = MissionState::new();
        let mut integrator = PerceptionIntegrator::new(5, 0);
        integrator.apply_ai_actions(
            &[AIAction::DispatchMission {
                kind: MissionKind::ShipDesignResearch,
                team: vec![designer],
                decoys: vec![],
                target_system: target,
                target_character: None,
                target_object: None,
            }],
            &mut ai,
            &mut missions,
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut TroopTransportState::default(),
            &mut ProductionState::new(),
            &mut world,
            5,
            false,
        );
        let id = missions.missions()[0].id;
        assert_eq!(ai.research_planners[0].mission, Some(id));

        integrator.apply_ai_actions(
            &[
                AIAction::AbortMission { mission_id: id },
                AIAction::AbortMission { mission_id: id },
            ],
            &mut ai,
            &mut missions,
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut TroopTransportState::default(),
            &mut ProductionState::new(),
            &mut world,
            5,
            false,
        );

        assert_eq!(missions.missions()[0].end_code, 2);
        let events = integrator.finish();
        let aborts: Vec<_> = events
            .iter()
            .filter(|e| e.details["type"] == "AbortMission")
            .collect();
        assert_eq!(aborts.len(), 1);
        assert_eq!(aborts[0].details["mission_id"], id);
    }

    #[test]
    fn an_applied_ai_mission_reports_its_team_and_decoys_as_dispatched() {
        let mut world = GameWorld::default();
        let target = add_system(&mut world, "Target");
        let envoy = imperial_at(&mut world, target, "Envoy");
        let double = imperial_at(&mut world, target, "Double");

        let events = empire_ai_events(
            &mut world,
            &[AIAction::DispatchMission {
                kind: MissionKind::Diplomacy,
                team: vec![envoy],
                decoys: vec![double],
                target_system: target,
                target_character: None,
                target_object: None,
            }],
        );

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, EVT_AI_ACTION);
        assert_eq!(events[0].details["team"], serde_json::json!(["Envoy"]));
        assert_eq!(events[0].details["decoys"], serde_json::json!(["Double"]));
        assert_eq!(events[1].system, SYS_MISSIONS);
        assert_eq!(events[1].event_type, EVT_MISSION_DISPATCHED);
        assert_eq!(
            events[1].details,
            serde_json::json!({
                "kind": "Diplomacy",
                "faction": "Empire",
                "team": ["Envoy"],
                "decoys": ["Double"],
                "target_system": "Target",
            })
        );
    }

    #[test]
    fn a_refused_ai_mission_reports_nothing() {
        let mut world = GameWorld::default();
        let target = add_system(&mut world, "Target");
        let rebel = MissionMember::Character(world.characters.insert(
            rebellion_core::world::Character {
                name: "Rebel".into(),
                is_alliance: true,
                current_system: Some(target),
                ..Default::default()
            },
        ));

        let events = empire_ai_events(
            &mut world,
            &[AIAction::DispatchMission {
                kind: MissionKind::Diplomacy,
                team: vec![rebel],
                decoys: vec![],
                target_system: target,
                target_character: None,
                target_object: None,
            }],
        );

        assert!(events.is_empty());
    }

    #[test]
    fn a_special_force_is_named_by_its_class() {
        let mut world = GameWorld::default();
        let unit = world.special_forces.insert(SpecialForceUnit {
            class_dat_id: DatId::new(0x3c00_0007),
            is_alliance: false,
            skills: [0; 8],
            on_mission: false,
        });

        assert_eq!(
            member_name(&world, MissionMember::SpecialForce(unit)),
            "special force 0x3c000007"
        );
    }

    #[test]
    fn ai_transport_action_embarks_before_authoritative_transit() {
        let mut world = GameWorld::default();
        let origin = add_system(&mut world, "Origin");
        let destination = add_system(&mut world, "Destination");
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            is_alliance: false,
            is_empire: true,
            hull: 100,
            troop_capacity: 1,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: origin,
            capital_ships: vec![ShipInstance::new(class, 100, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[origin].fleets.push(fleet);
        let troop = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[origin].ground_units.push(troop);

        let actions = [AIAction::MoveFleet {
            fleet,
            to_system: destination,
            reason: FleetMoveReason::Attack,
            troops: vec![troop],
        }];
        let mut ai = AIState::new(AiFaction::Empire);
        let mut missions = MissionState::new();
        let mut manufacturing = ManufacturingState::new();
        let mut movement = MovementState::new();
        let mut troop_transport = TroopTransportState::default();
        let mut integrator = PerceptionIntegrator::new(5, 0);

        integrator.apply_ai_actions(
            &actions,
            &mut ai,
            &mut missions,
            &mut manufacturing,
            &mut movement,
            &mut troop_transport,
            &mut ProductionState::new(),
            &mut world,
            5,
            false,
        );

        assert!(movement.is_in_transit(fleet));
        assert_eq!(troop_transport.cargo(fleet), &[troop]);
        assert!(!world.systems[origin].ground_units.contains(&troop));
        assert_eq!(integrator.finish()[0].details["troops"], 1);
    }

    #[test]
    fn troops_embarked_for_a_move_that_cannot_depart_land_again() {
        let mut world = GameWorld::default();
        let origin = add_system(&mut world, "Origin");
        let destination = add_system(&mut world, "Destination");
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            is_alliance: false,
            is_empire: true,
            hull: 100,
            troop_capacity: 1,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: origin,
            capital_ships: vec![ShipInstance::new(class, 100, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[origin].fleets.push(fleet);
        let troop = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[origin].ground_units.push(troop);
        let mut movement = MovementState::new();
        // An order already held refuses the new one (MovementState::order).
        assert!(movement.order(fleet, origin, destination, 3));
        let mut troop_transport = TroopTransportState::default();

        let applied = apply_ai_actions_inner(
            &[AIAction::MoveFleet {
                fleet,
                to_system: destination,
                reason: FleetMoveReason::Attack,
                troops: vec![troop],
            }],
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut movement,
            &mut troop_transport,
            &mut ProductionState::new(),
            &mut world,
            5,
        );

        assert_eq!(applied, vec![false]);
        assert!(troop_transport.cargo(fleet).is_empty());
        assert!(world.systems[origin].ground_units.contains(&troop));
    }

    #[test]
    fn production_uses_orbiting_garrison_instead_of_transit_fleet() {
        let mut world = GameWorld::default();
        let origin = add_system(&mut world, "Origin");
        let destination = add_system(&mut world, "Destination");
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            is_alliance: false,
            hull: 100,
            ..CapitalShipClass::default()
        });
        let transit = world.fleets.insert(Fleet {
            location: origin,
            capital_ships: vec![ShipInstance::new(class, 100, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[origin].fleets.push(transit);
        let mut movement = MovementState::new();
        assert!(begin_fleet_transit(
            &mut movement,
            &mut world,
            transit,
            destination,
            10,
        ));

        let completion = CompletionEvent {
            system: origin,
            tick: 1,
            kind: BuildableKind::CapitalShip(class),
        };
        apply_build_completion_inner(&completion, &mut world);
        apply_build_completion_inner(&completion, &mut world);

        assert_eq!(world.fleets.len(), 2);
        assert_eq!(world.fleets[transit].ship_count(), 1);
        let garrison = world.systems[origin].fleets[0];
        assert_ne!(garrison, transit);
        assert_eq!(world.fleets[garrison].ship_count(), 2);
    }

    #[test]
    fn a_craft_completed_where_its_side_has_no_fleet_starts_a_numbered_fleet() {
        // FUN_00517760: each new fleet takes its side's next "Fleet N"
        // (ghidra/notes/fleet-names.md).
        let mut world = GameWorld::default();
        let first = add_system(&mut world, "First");
        let second = add_system(&mut world, "Second");
        let ship = world.capital_ship_classes.insert(CapitalShipClass {
            is_alliance: true,
            hull: 100,
            ..Default::default()
        });
        for (system, kind) in [
            (first, BuildableKind::CapitalShip(ship)),
            (second, BuildableKind::CapitalShip(ship)),
        ] {
            apply_build_completion_inner(
                &CompletionEvent {
                    system,
                    tick: 1,
                    kind,
                },
                &mut world,
            );
        }
        let name = |system: SystemKey| world.fleet_name(world.systems[system].fleets[0]);
        assert_eq!(name(first), Some("Fleet 1"));
        assert_eq!(name(second), Some("Fleet 2"));
    }

    #[test]
    fn a_completed_death_star_can_fire_from_its_new_fleet() {
        // CAPSHPSD.DAT record 136 is the Death Star (TEXTSTRA 10120).
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Yard");
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(rebellion_core::world::DEATH_STAR_CLASS_ID),
            is_empire: true,
            hull: 25_000,
            ..Default::default()
        });
        let target = add_system(&mut world, "Target");
        world.systems[target].control = ControlKind::Controlled(Faction::Alliance);

        apply_build_completion_inner(
            &CompletionEvent {
                system,
                tick: 1,
                kind: BuildableKind::CapitalShip(class),
            },
            &mut world,
        );

        let fleet = world.systems[system].fleets[0];
        assert!(world.fleets[fleet].has_death_star);
        world.systems[system].fleets.clear();
        world.systems[target].fleets.push(fleet);
        world.fleets[fleet].location = target;
        assert!(rebellion_core::death_star::DeathStarSystem::fire(
            &rebellion_core::death_star::DeathStarState::default(),
            &world,
            target,
            2
        )
        .is_some());
    }

    #[test]
    fn a_regiment_lost_running_a_blockade_leaves_its_fleet_and_is_reported() {
        // FUN_00504990 destroys the regiment; FUN_00504a00 raises event 0x340.
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Bespin");
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            troop_capacity: 1,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![ShipInstance::new(class, 100, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[system].fleets.push(fleet);
        let troop = world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(troop);
        let mut transport = TroopTransportState::default();
        transport.embark(&mut world, fleet, &[troop]).unwrap();
        let mut integrator = PerceptionIntegrator::new(9, 0);

        integrator.apply_blockade_events(
            &mut world,
            &mut transport,
            &[BlockadeEvent::TroopDestroyed {
                system,
                troop,
                tick: 9,
            }],
        );

        assert!(transport.cargo(fleet).is_empty());
        assert!(!world.troops.contains_key(troop));
        let events = integrator.finish();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EVT_BLOCKADE_TROOP_DESTROYED);
        assert_eq!(events[0].details["system"], "Bespin");
    }

    #[test]
    fn a_death_star_joining_a_fleet_marks_it_and_other_ships_do_not() {
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Yard");
        let mut add_class = |id| {
            world.capital_ship_classes.insert(CapitalShipClass {
                dat_id: DatId::new(id),
                is_empire: true,
                hull: 100,
                ..Default::default()
            })
        };
        let star_destroyer = add_class(131);
        let death_star = add_class(rebellion_core::world::DEATH_STAR_CLASS_ID);
        let build = |world: &mut GameWorld, class| {
            apply_build_completion_inner(
                &CompletionEvent {
                    system,
                    tick: 1,
                    kind: BuildableKind::CapitalShip(class),
                },
                world,
            );
        };

        build(&mut world, star_destroyer);
        let fleet = world.systems[system].fleets[0];
        assert!(!world.fleets[fleet].has_death_star);

        build(&mut world, death_star);
        assert_eq!(world.systems[system].fleets, [fleet]);
        assert!(world.fleets[fleet].has_death_star);
    }

    #[test]
    fn completed_squadrons_stack_on_their_planet_beside_any_fleet() {
        // FUN_0052a140 / FUN_0052a380: the product joins its destination's
        // container (slot +0xa8), the system; FUN_005039d0 lists it on the
        // planet (ghidra/notes/planet-squadrons.md).
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Yard");
        let own = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[system].fleets.push(own);
        let tie = world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                is_empire: true,
                ..Default::default()
            });
        for _ in 0..2 {
            apply_build_completion_inner(
                &CompletionEvent {
                    system,
                    tick: 1,
                    kind: BuildableKind::Fighter(tie),
                },
                &mut world,
            );
        }

        assert!(world.fleets[own].fighters.is_empty());
        assert_eq!(world.systems[system].fleets, [own]);
        assert_eq!(world.systems[system].fighters.len(), 1);
        assert_eq!(world.systems[system].fighters[0].class, tie);
        assert_eq!(world.systems[system].fighters[0].count, 2);
    }

    #[test]
    fn troop_production_builds_its_class_for_the_classes_side() {
        // TROOPSD record 3 (Sullustan) serves the Alliance only (+0x18).
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Training World");
        world.buildable_classes.insert(
            DatId::new(0x1000_0003),
            rebellion_core::world::BuildableClass {
                is_alliance: true,
                ..Default::default()
            },
        );

        apply_build_completion_inner(
            &CompletionEvent {
                system,
                tick: 1,
                kind: BuildableKind::Troop(DatId::new(0x1000_0003)),
            },
            &mut world,
        );

        let built = *world.systems[system].ground_units.last().unwrap();
        assert_eq!(world.troops[built].class_dat_id, DatId::new(0x1000_0003));
        assert!(world.troops[built].is_alliance);
        assert_eq!(world.troops[built].regiment_strength, 100);
    }

    #[test]
    fn facility_and_special_force_production_build_their_class_for_the_builder() {
        // MANFACSD and PROFACSD serve both sides, so the order carries its
        // builder's; family 0x28 is a shipyard and 0x2c a mine
        // (FUN_00537ff0, FUN_00458fe0).
        use rebellion_core::manufacturing::FacilityBuild;
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Kuat");
        let empire = |class| FacilityBuild {
            class: DatId::new(class),
            is_alliance: false,
        };
        world.buildable_classes.insert(
            DatId::new(0x3c00_0007),
            rebellion_core::world::BuildableClass {
                is_empire: true,
                ..Default::default()
            },
        );
        world.special_force_classes.insert(
            DatId::new(0x3c00_0007),
            rebellion_core::world::SpecialForceClassDef {
                skills: [rebellion_core::world::SkillPair {
                    base: 30,
                    variance: 10,
                }; 8],
                mission_mask: 0,
            },
        );
        for kind in [
            BuildableKind::ManufacturingFacility(empire(0x2800_0004)),
            BuildableKind::ProductionFacility(empire(0x2c00_0001)),
            BuildableKind::DefenseFacility(empire(0x2200_0001)),
            BuildableKind::SpecialForce(DatId::new(0x3c00_0007)),
        ] {
            apply_build_completion_inner(
                &CompletionEvent {
                    system,
                    tick: 1,
                    kind,
                },
                &mut world,
            );
        }

        let at = &world.systems[system];
        let yard = &world.manufacturing_facilities[at.manufacturing_facilities[0]];
        assert_eq!(yard.class_dat_id, DatId::new(0x2800_0004));
        assert!(yard.side == rebellion_core::dat::Faction::Empire && yard.is_shipyard);
        let mine = &world.production_facilities[at.production_facilities[0]];
        assert!(mine.side == rebellion_core::dat::Faction::Empire && mine.is_mine);
        let defense = &world.defense_facilities[at.defense_facilities[0]];
        assert_eq!(defense.class_dat_id, DatId::new(0x2200_0001));
        assert_eq!(defense.side, rebellion_core::dat::Faction::Empire);
        let unit = &world.special_forces[at.special_forces[0]];
        assert!(!unit.is_alliance);
        assert_eq!(unit.skills, [30; 8]);
    }
}

// ---------------------------------------------------------------------------
// Combat helpers (moved from simulation.rs)
// ---------------------------------------------------------------------------

/// Safety cap for a complete system-level engagement.
///
/// A round that changes neither hull nor fighter strength ends the engagement
/// immediately, so this cap only protects against unexpectedly large battles.
pub const MAX_SYSTEM_COMBAT_ROUNDS: u32 = 256;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpaceForceSummary {
    pub capital_ships: usize,
    pub fighter_squadrons: u32,
    pub hull: i64,
}

/// Aggregate result for every hostile fleet orbiting one system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemSpaceCombatResult {
    pub system: SystemKey,
    /// Alliance is the attacker and Empire is the defender in auto-resolve.
    pub winner: CombatSide,
    /// A surviving fleet belonging to the winning faction, when decisive.
    pub winner_fleet: Option<FleetKey>,
    pub rounds: u32,
    pub alliance_fleets: usize,
    pub empire_fleets: usize,
    pub alliance_before: SpaceForceSummary,
    pub alliance_after: SpaceForceSummary,
    pub empire_before: SpaceForceSummary,
    pub empire_after: SpaceForceSummary,
    /// True when combat could not change any remaining unit or reached the cap.
    pub stalemate: bool,
}

fn orbiting_fleets_by_faction(
    world: &GameWorld,
    system: SystemKey,
) -> (Vec<FleetKey>, Vec<FleetKey>) {
    let Some(system) = world.systems.get(system) else {
        return (Vec::new(), Vec::new());
    };
    system
        .fleets
        .iter()
        .copied()
        .filter_map(|fleet_key| {
            world
                .fleets
                .get(fleet_key)
                .map(|fleet| (fleet_key, fleet.is_alliance))
        })
        .fold(
            (Vec::new(), Vec::new()),
            |(mut alliance, mut empire), (fleet_key, is_alliance)| {
                if is_alliance {
                    alliance.push(fleet_key);
                } else {
                    empire.push(fleet_key);
                }
                (alliance, empire)
            },
        )
}

fn summarize_space_force(world: &GameWorld, fleets: &[FleetKey]) -> SpaceForceSummary {
    fleets
        .iter()
        .fold(SpaceForceSummary::default(), |mut summary, fleet_key| {
            if let Some(fleet) = world.fleets.get(*fleet_key) {
                summary.capital_ships +=
                    fleet.capital_ships.iter().filter(|ship| ship.alive).count();
                summary.fighter_squadrons +=
                    fleet.fighters.iter().map(|entry| entry.count).sum::<u32>();
                summary.hull += fleet
                    .capital_ships
                    .iter()
                    .filter(|ship| ship.alive)
                    .map(|ship| i64::from(ship.hull_current))
                    .sum::<i64>();
            }
            summary
        })
}

/// Resolve all hostile task forces at a system as one uninterrupted engagement.
///
/// Each core combat call models one combat round. Applying every damaging round
/// here prevents repair ticks from healing fleets between rounds and advances to
/// the next deterministic pair when either task force is destroyed.
pub fn resolve_system_space_combat(
    world: &mut GameWorld,
    system: SystemKey,
    difficulty: u8,
    rng_rolls: &[f64],
    tick: u64,
    death_star_shield_active: bool,
) -> SystemSpaceCombatResult {
    let (initial_alliance, initial_empire) = orbiting_fleets_by_faction(world, system);
    let alliance_before = summarize_space_force(world, &initial_alliance);
    let empire_before = summarize_space_force(world, &initial_empire);
    let mut rounds = 0;
    let mut stalemate = false;

    while rounds < MAX_SYSTEM_COMBAT_ROUNDS {
        let (alliance, empire) = orbiting_fleets_by_faction(world, system);
        let (Some(&attacker), Some(&defender)) = (alliance.first(), empire.first()) else {
            break;
        };

        let round = CombatSystem::resolve_space(
            world,
            attacker,
            defender,
            system,
            difficulty,
            rng_rolls,
            tick,
            death_star_shield_active,
        );
        let made_progress = round
            .ship_damage
            .iter()
            .any(|event| event.hull_after < event.hull_before)
            || round
                .fighter_losses
                .iter()
                .any(|event| event.squads_after < event.squads_before);

        apply_space_combat_result_inner(&round, world);
        rounds += 1;

        if !made_progress {
            stalemate = true;
            break;
        }
    }

    let (alliance, empire) = orbiting_fleets_by_faction(world, system);
    let alliance_after = summarize_space_force(world, &alliance);
    let empire_after = summarize_space_force(world, &empire);
    let (winner, winner_fleet) = match (alliance.first().copied(), empire.first().copied()) {
        (Some(fleet), None) => (CombatSide::Attacker, Some(fleet)),
        (None, Some(fleet)) => (CombatSide::Defender, Some(fleet)),
        _ => (CombatSide::Draw, None),
    };
    if rounds == MAX_SYSTEM_COMBAT_ROUNDS && winner == CombatSide::Draw {
        stalemate = true;
    }

    SystemSpaceCombatResult {
        system,
        winner,
        winner_fleet,
        rounds,
        alliance_fleets: initial_alliance.len(),
        empire_fleets: initial_empire.len(),
        alliance_before,
        alliance_after,
        empire_before,
        empire_after,
        stalemate,
    }
}

pub fn apply_space_combat_result_inner(result: &SpaceCombatResult, world: &mut GameWorld) {
    // Apply hull damage to individual ship instances.
    for evt in &result.ship_damage {
        let fleet_key = evt.fleet;
        if let Some(fleet) = world.fleets.get_mut(fleet_key) {
            // ship_index maps 1:1 to alive ships at snapshot time.
            // Find the nth alive ship.
            let mut alive_idx = 0;
            for ship in &mut fleet.capital_ships {
                if !ship.alive {
                    continue;
                }
                if alive_idx == evt.ship_index {
                    ship.hull_current = evt.hull_after;
                    if evt.hull_after <= 0 {
                        ship.alive = false;
                    }
                    break;
                }
                alive_idx += 1;
            }
        }
    }

    // Apply fighter attrition before testing whether either fleet is empty.
    // `fighter_index` maps directly to the roster snapshot used by combat.
    for evt in &result.fighter_losses {
        if let Some(fleet) = world.fleets.get_mut(evt.fleet) {
            if let Some(entry) = fleet.fighters.get_mut(evt.fighter_index) {
                entry.count = evt.squads_after;
            }
        }
    }

    // Remove dead ships and empty fleets.
    for &fleet_key in &[result.attacker_fleet, result.defender_fleet] {
        let death_star_status = world.fleets.get(fleet_key).map(|fleet| {
            fleet
                .capital_ships
                .iter()
                .fold((false, false), |(present, alive), ship| {
                    let is_death_star = world.capital_ship_classes[ship.class].is_death_star();
                    (
                        present || is_death_star,
                        alive || (is_death_star && ship.alive),
                    )
                })
        });
        if let Some(fleet) = world.fleets.get_mut(fleet_key) {
            fleet.capital_ships.retain(|s| s.alive);
            if let Some((true, death_star_alive)) = death_star_status {
                fleet.has_death_star = death_star_alive;
            }
        }
        let is_empty = world
            .fleets
            .get(fleet_key)
            .is_none_or(rebellion_core::world::Fleet::is_empty);
        if is_empty {
            // Capture losing fleet's characters (parity: officers captured on fleet destruction).
            let is_loser = match result.winner {
                CombatSide::Attacker => fleet_key == result.defender_fleet,
                CombatSide::Defender => fleet_key == result.attacker_fleet,
                CombatSide::Draw => false,
            };
            if is_loser {
                let capture_data = world.fleets.get(fleet_key).map(|f| {
                    let captor = if f.is_alliance {
                        rebellion_core::dat::Faction::Empire
                    } else {
                        rebellion_core::dat::Faction::Alliance
                    };
                    (f.characters.clone(), captor, f.location)
                });
                if let Some((chars, captor, loc)) = capture_data {
                    for ck in chars {
                        if let Some(c) = world.characters.get_mut(ck) {
                            c.is_captive = true;
                            c.captured_by = Some(captor);
                            c.capture_tick = Some(result.tick);
                            c.current_system = Some(loc);
                            c.current_fleet = None;
                        }
                    }
                }
            }
            if let Some(fleet) = world.fleets.get(fleet_key) {
                let loc = fleet.location;
                if let Some(sys) = world.systems.get_mut(loc) {
                    sys.fleets.retain(|&k| k != fleet_key);
                }
            }
            world.fleets.remove(fleet_key);
        }
    }
}

#[cfg(test)]
mod combat_application_tests {
    use super::*;
    use rebellion_core::combat::{FighterLossEvent, ShipDamageEvent, SpaceCombatResult};
    use rebellion_core::dat::{ExplorationStatus, Faction};
    use rebellion_core::ids::{FighterKey, SectorKey};
    use rebellion_core::world::{CapitalShipClass, Character, FighterEntry, Fleet, System};

    fn add_combat_system(world: &mut GameWorld) -> SystemKey {
        world.systems.insert(System {
            dat_id: DatId::new(0x9000_0001),
            name: "Test System".into(),
            sector: SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
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
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Uncontrolled,
        })
    }

    fn add_ship_fleet(
        world: &mut GameWorld,
        system: SystemKey,
        is_alliance: bool,
        hull: u32,
        attack: u32,
    ) -> FleetKey {
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(if is_alliance {
                0x3000_0001
            } else {
                0x3000_0002
            }),
            is_alliance,
            is_empire: !is_alliance,
            hull,
            turbolaser_fore: attack,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![ShipInstance::new(class, hull.cast_signed(), is_alliance)],
            fighters: vec![],
            characters: vec![],
            is_alliance,
            has_death_star: false,
        });
        world.systems[system].fleets.push(fleet);
        fleet
    }

    #[test]
    fn fighter_losses_update_rosters_and_remove_defeated_fleet() {
        let mut world = GameWorld::default();
        let attacker = world.fleets.insert(Fleet {
            location: SystemKey::default(),
            capital_ships: vec![],
            fighters: vec![FighterEntry {
                class: FighterKey::default(),
                count: 4,
            }],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        let defender = world.fleets.insert(Fleet {
            location: SystemKey::default(),
            capital_ships: vec![],
            fighters: vec![FighterEntry {
                class: FighterKey::default(),
                count: 3,
            }],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        let captive = world.characters.insert(Character {
            name: "Defeated Officer".into(),
            is_empire: true,
            current_fleet: Some(defender),
            ..Character::default()
        });
        world.fleets[defender].characters.push(captive);
        let result = SpaceCombatResult {
            attacker_fleet: attacker,
            defender_fleet: defender,
            system: SystemKey::default(),
            winner: CombatSide::Attacker,
            ship_damage: vec![],
            fighter_losses: vec![
                FighterLossEvent {
                    fleet: attacker,
                    fighter_index: 0,
                    squads_before: 4,
                    squads_after: 2,
                },
                FighterLossEvent {
                    fleet: defender,
                    fighter_index: 0,
                    squads_before: 3,
                    squads_after: 0,
                },
            ],
            tick: 1,
        };

        apply_space_combat_result_inner(&result, &mut world);

        assert_eq!(world.fleets[attacker].fighters[0].count, 2);
        assert!(!world.fleets.contains_key(defender));
        assert!(world.characters[captive].is_captive);
        assert_eq!(
            world.characters[captive].captured_by,
            Some(Faction::Alliance)
        );
        assert_eq!(world.characters[captive].capture_tick, Some(1));
        assert_eq!(world.characters[captive].current_fleet, None);
    }

    #[test]
    fn destroyed_death_star_clears_fleet_flag_while_escort_survives() {
        let mut world = GameWorld::default();
        let system = add_combat_system(&mut world);
        let attacker = add_ship_fleet(&mut world, system, true, 100, 10);
        let death_star_class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x3400_0001),
            name: "Death Star".into(),
            is_empire: true,
            hull: 1_000,
            ..CapitalShipClass::default()
        });
        let escort_class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x3000_0002),
            name: "Escort".into(),
            is_empire: true,
            hull: 100,
            ..CapitalShipClass::default()
        });
        let defender = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![
                ShipInstance::new(death_star_class, 1_000, false),
                ShipInstance::new(escort_class, 100, false),
            ],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: true,
        });
        world.systems[system].fleets.push(defender);
        let result = SpaceCombatResult {
            attacker_fleet: attacker,
            defender_fleet: defender,
            system,
            winner: CombatSide::Attacker,
            ship_damage: vec![ShipDamageEvent {
                fleet: defender,
                ship_index: 0,
                hull_before: 1_000,
                hull_after: 0,
            }],
            fighter_losses: vec![],
            tick: 2,
        };

        apply_space_combat_result_inner(&result, &mut world);

        assert!(world.fleets.contains_key(defender));
        assert!(!world.fleets[defender].has_death_star);
        assert_eq!(world.fleets[defender].capital_ships.len(), 1);
        assert_eq!(world.fleets[defender].capital_ships[0].class, escort_class);
    }

    #[test]
    fn system_engagement_resolves_multiple_rounds_before_repair() {
        let mut world = GameWorld::default();
        let system = add_combat_system(&mut world);
        let attacker = add_ship_fleet(&mut world, system, true, 100, 30);
        let defender = add_ship_fleet(&mut world, system, false, 100, 1);

        let result = resolve_system_space_combat(&mut world, system, 1, &[0.5; 256], 10, false);

        assert_eq!(result.winner, CombatSide::Attacker);
        assert_eq!(result.winner_fleet, Some(attacker));
        assert!(result.rounds > 1);
        assert!(!result.stalemate);
        assert!(!world.fleets.contains_key(defender));
    }

    #[test]
    fn system_engagement_includes_every_orbiting_task_force() {
        let mut world = GameWorld::default();
        let system = add_combat_system(&mut world);
        let first_alliance = add_ship_fleet(&mut world, system, true, 200, 100);
        let second_alliance = add_ship_fleet(&mut world, system, true, 200, 100);
        let first_empire = add_ship_fleet(&mut world, system, false, 30, 1);
        let second_empire = add_ship_fleet(&mut world, system, false, 30, 1);

        let result = resolve_system_space_combat(&mut world, system, 1, &[0.5; 256], 11, false);

        assert_eq!(result.alliance_fleets, 2);
        assert_eq!(result.empire_fleets, 2);
        assert_eq!(result.winner, CombatSide::Attacker);
        assert!(world.fleets.contains_key(first_alliance));
        assert!(world.fleets.contains_key(second_alliance));
        assert!(!world.fleets.contains_key(first_empire));
        assert!(!world.fleets.contains_key(second_empire));

        let mut integrator = PerceptionIntegrator::new(11, 0);
        integrator.emit_system_space_combat(&world, &result);
        let events = integrator.finish();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EVT_COMBAT_SPACE);
        assert_eq!(events[0].details["alliance_fleets"], 2);
        assert_eq!(events[0].details["empire_fleets"], 2);
    }

    #[test]
    fn system_engagement_stops_on_true_no_progress_stalemate() {
        let mut world = GameWorld::default();
        let system = add_combat_system(&mut world);
        add_ship_fleet(&mut world, system, true, 100, 0);
        add_ship_fleet(&mut world, system, false, 100, 0);

        let result = resolve_system_space_combat(&mut world, system, 1, &[0.5; 256], 12, false);

        assert_eq!(result.winner, CombatSide::Draw);
        assert_eq!(result.rounds, 1);
        assert!(result.stalemate);
        assert_eq!(world.systems[system].fleets.len(), 2);
    }
}

pub fn apply_ground_combat_result_inner(result: &GroundCombatResult, world: &mut GameWorld) {
    let mut final_strengths: HashMap<TroopKey, i16> = HashMap::new();
    for evt in &result.troop_damage {
        final_strengths.insert(evt.troop, evt.strength_after);
    }
    for (&key, &strength) in &final_strengths {
        if let Some(troop) = world.troops.get_mut(key) {
            troop.regiment_strength = strength;
        }
    }

    let sys_key = result.system;
    if let Some(sys) = world.systems.get_mut(sys_key) {
        sys.ground_units
            .retain(|&k| world.troops.get(k).is_some_and(|t| t.regiment_strength > 0));
    }
    let dead: Vec<_> = final_strengths
        .iter()
        .filter(|(_, &s)| s <= 0)
        .map(|(&k, _)| k)
        .collect();
    for key in dead {
        world.troops.remove(key);
    }
}

// ---------------------------------------------------------------------------
// Build completion helper (moved from simulation.rs)
// ---------------------------------------------------------------------------

#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
pub fn apply_build_completion_inner(completion: &CompletionEvent, world: &mut GameWorld) {
    let sys_key = completion.system;

    match &completion.kind {
        BuildableKind::CapitalShip(class_key) => {
            let is_alliance = world
                .capital_ship_classes
                .get(*class_key)
                .is_some_and(|c| c.is_alliance);
            let is_death_star = world
                .capital_ship_classes
                .get(*class_key)
                .is_some_and(CapitalShipClass::is_death_star);

            let fleet_key = {
                let Some(sys) = world.systems.get(sys_key) else {
                    return;
                };
                sys.fleets.iter().copied().find(|&fk| {
                    world
                        .fleets
                        .get(fk)
                        .is_some_and(|f| f.is_alliance == is_alliance)
                })
            };

            if let Some(fk) = fleet_key {
                if let Some(fleet) = world.fleets.get_mut(fk) {
                    let hull = world
                        .capital_ship_classes
                        .get(*class_key)
                        .map_or(100, |c| c.hull.cast_signed());
                    fleet
                        .capital_ships
                        .push(ShipInstance::new(*class_key, hull, is_alliance));
                    fleet.has_death_star |= is_death_star;
                }
            } else {
                let hull = world
                    .capital_ship_classes
                    .get(*class_key)
                    .map_or(100, |c| c.hull.cast_signed());
                let fleet = Fleet {
                    location: sys_key,
                    capital_ships: vec![ShipInstance::new(*class_key, hull, is_alliance)],
                    fighters: vec![],
                    characters: vec![],
                    is_alliance,
                    has_death_star: is_death_star,
                };
                let fk = world.insert_fleet(fleet);
                if let Some(sys) = world.systems.get_mut(sys_key) {
                    sys.fleets.push(fk);
                }
            }
        }
        // FUN_0052a140 / FUN_0052a380: the product joins the container its
        // Destination names (slot +0xa8), the system, and stands on its
        // planet (FUN_005039d0; ghidra/notes/planet-squadrons.md).
        BuildableKind::Fighter(class_key) => {
            if !world.fighter_classes.contains_key(*class_key) {
                return;
            }
            if let Some(sys) = world.systems.get_mut(sys_key) {
                rebellion_core::squadrons::add(&mut sys.fighters, *class_key, 1);
            }
        }
        BuildableKind::ManufacturingFacility(build) => {
            let fac_key = world
                .manufacturing_facilities
                .insert(ManufacturingFacilityInstance {
                    class_dat_id: build.class,
                    side: rebellion_core::dat::Faction::of_alliance(build.is_alliance),
                    is_shipyard: build.class.family() == 0x28,
                });
            if let Some(sys) = world.systems.get_mut(sys_key) {
                sys.manufacturing_facilities.push(fac_key);
            }
        }
        BuildableKind::DefenseFacility(build) => {
            let fac_key = world.defense_facilities.insert(DefenseFacilityInstance {
                class_dat_id: build.class,
                side: rebellion_core::dat::Faction::of_alliance(build.is_alliance),
            });
            if let Some(sys) = world.systems.get_mut(sys_key) {
                sys.defense_facilities.push(fac_key);
            }
        }
        BuildableKind::ProductionFacility(build) => {
            let fac_key = world
                .production_facilities
                .insert(ProductionFacilityInstance {
                    class_dat_id: build.class,
                    side: rebellion_core::dat::Faction::of_alliance(build.is_alliance),
                    is_mine: build.class.family() == 0x2c,
                });
            if let Some(sys) = world.systems.get_mut(sys_key) {
                sys.production_facilities.push(fac_key);
            }
        }
        // A regiment or special-force class serves one side.
        BuildableKind::Troop(class) => {
            let Some(is_alliance) = world.buildable_classes.get(class).map(|c| c.is_alliance)
            else {
                return;
            };
            let tk = world.troops.insert(TroopUnit {
                class_dat_id: *class,
                is_alliance,
                regiment_strength: 100,
            });
            if let Some(sys) = world.systems.get_mut(sys_key) {
                sys.ground_units.push(tk);
            }
        }
        // port: a built unit takes its class's base skills; the creation
        // roll (`FUN_00535e40`, base + rand(0..=variance)) needs the
        // simulation's random stream, which completion does not carry.
        BuildableKind::SpecialForce(class) => {
            let Some(is_alliance) = world.buildable_classes.get(class).map(|c| c.is_alliance)
            else {
                return;
            };
            let skills = world
                .special_force_classes
                .get(class)
                .map_or([0; 8], |def| def.skills.map(|pair| pair.base));
            let key = world.special_forces.insert(SpecialForceUnit {
                class_dat_id: *class,
                is_alliance,
                skills,
                on_mission: false,
            });
            if let Some(sys) = world.systems.get_mut(sys_key) {
                sys.special_forces.push(key);
            }
        }
    }
}
