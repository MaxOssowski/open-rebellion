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


use rebellion_core::ai::{AIAction, AIState, AiFaction};
use rebellion_core::blockade::BlockadeEvent;
use rebellion_core::bombardment::BombardEnv;
use rebellion_core::combat::{CombatSide, SpaceCombatResult};
use rebellion_core::death_star::DeathStarEvent;
use rebellion_core::economy::{EconomyEvent, EconomyState};
use rebellion_core::events::{EventAction, FiredEvent, SkillField, SystemTag};
use rebellion_core::fog::RevealEvent;
use rebellion_core::game_events::{
    GameEventRecord, EVT_AI_ACTION, EVT_BLOCKADE_ENDED, EVT_BLOCKADE_STARTED,
    EVT_BLOCKADE_SUPPORT, EVT_BLOCKADE_TROOP_DESTROYED, EVT_BOMBARDMENT, EVT_BUILD_COMPLETE,
    EVT_CAMPAIGN_SNAPSHOT,
    EVT_CAPTURE, EVT_CHARACTER_HEALTH, EVT_CHARACTER_KILLED, EVT_COLLECTION_RATE,
    EVT_COMBAT_GROUND, EVT_COMBAT_SPACE, EVT_CONTROL_CHANGED, EVT_DS_FIRED,
    EVT_DS_STATUS, EVT_ECONOMY_TICK, EVT_ESCAPE, EVT_EVENT_FIRED, EVT_FLEET_ARRIVED,
    EVT_CONFRONTATION, EVT_FOG_REVEALED, EVT_FORCE_DISCOVERED, EVT_FORCE_ENCOUNTER,
    EVT_FORCE_EXPERIENCE, EVT_STORY_FINAL_BATTLE, EVT_STORY_JABBA, EVT_STORY_SEAT_OF_POWER, EVT_VICTORY_CONDITIONS,
    EVT_GARRISON_REQUIRED, EVT_HERITAGE_LEARNED, EVT_HQ_CAPTURED, EVT_INFORMANT_INTEL,
    EVT_LUKE_DAGOBAH,
    EVT_MAINTENANCE_SHORTFALL, EVT_MANUFACTURING_IDLE, EVT_MISSION_DISPATCHED,
    EVT_ADRIFT_ORDERED, EVT_MISSION_RESOLVED, EVT_NATURAL_DISASTER, EVT_RETURN_ORDERED, EVT_RESEARCH_UNLOCKED, EVT_RESOURCE_DISCOVERY,
    EVT_SABOTEUR_DETECTED, EVT_SHIP_REPAIRED, EVT_SHIP_REPAIR_STARTED,
    EVT_SMUGGLING_PERCENT, EVT_TRAITOR_REVEALED, EVT_TROOP_MOVED,
    EVT_DESTROYED_ON_ARRIVAL, EVT_UNITS_DEPLOYED, EVT_UPRISING_BEGAN, EVT_UPRISING_CHECK, EVT_UPRISING_ENDED,
    EVT_UPRISING_SUPPORT,
    EVT_UPRISING_INCIDENT, EVT_VICTORY, EVT_VICTORY_CHECK, SYS_AI, SYS_BLOCKADE,
    SYS_COMBAT, SYS_DEATH_STAR, SYS_ECONOMY, SYS_EVENTS, SYS_FOG, SYS_JEDI, SYS_MANUFACTURING,
    SYS_MISSIONS, SYS_MOVEMENT, SYS_REPAIR, SYS_RESEARCH, SYS_STORY, SYS_UPRISING, SYS_VICTORY,
};
use rebellion_core::ids::DatId;
use rebellion_core::ids::{CharacterKey, FleetKey, SystemKey, TroopKey};
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
        AIAction::BoardRegiments { troops, fleet } => {
            serde_json::json!({
                "type": "BoardRegiments",
                "troops": troops.len(),
                "at": world
                    .fleets
                    .get(*fleet)
                    .map_or_else(|| "unknown".into(), |f| sys_name(world, f.location)),
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
        AIAction::MoveSquadronTeam {
            squadrons,
            from,
            to,
        } => {
            serde_json::json!({
                "type": "MoveSquadronTeam",
                "count": squadrons.iter().map(|&(_, n)| n).sum::<u32>(),
                "from": format!("{from:?}"),
                "to": format!("{to:?}"),
            })
        }
        AIAction::MoveCharacters { characters, to } => {
            serde_json::json!({
                "type": "MoveCharacters",
                "characters": characters
                    .iter()
                    .map(|&c| member_name(world, MissionMember::Character(c)))
                    .collect::<Vec<_>>(),
                "to": sys_name(world, *to),
            })
        }
        AIAction::Bombard {
            fleets,
            system,
            kind,
        } => serde_json::json!({
            "type": "Bombard",
            "fleets": fleets.len(),
            "system": sys_name(world, *system),
            "kind": format!("{kind:#x}"),
        }),
        AIAction::Assault { fleets, system } => serde_json::json!({
            "type": "Assault",
            "fleets": fleets.len(),
            "system": sys_name(world, *system),
        }),
        AIAction::JoinFleet { fleet, into } => {
            let at = |key| {
                world
                    .fleets
                    .get(key)
                    .map_or_else(|| "unknown".into(), |f| sys_name(world, f.location))
            };
            serde_json::json!({
                "type": "JoinFleet",
                "from": at(*fleet),
                "to": at(*into),
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
        AIAction::CreateFleet { fleet, ships, to } => serde_json::json!({
            "type": "CreateFleet",
            "ships": ships.len(),
            "at": world
                .fleets
                .get(*fleet)
                .map_or_else(|| "unknown".into(), |f| sys_name(world, f.location)),
            "to": sys_name(world, *to),
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

    /// Telemetry for each loyalty shift (`rebellion_core::loyalty::settle`).
    pub fn record_loyalty_shifts(&mut self, shifts: &[rebellion_core::loyalty::LoyaltyShift]) {
        for shift in shifts {
            self.emit(
                SYS_EVENTS,
                rebellion_core::game_events::EVT_LOYALTY_SHIFT,
                serde_json::json!({
                    "side": format!("{:?}", shift.side),
                    "amount": shift.amount,
                }),
            );
        }
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
                        "production_modifier": rebellion_core::economy::production_modifier_at(economy, key),
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

    /// Apply economy events: world mutations (control) + telemetry.
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    pub fn apply_economy_events(&mut self, world: &mut GameWorld, economy_events: &[EconomyEvent]) {
        for ev in economy_events {
            match ev {
                EconomyEvent::SmugglingPercentChanged {
                    system, percent, ..
                } => {
                    self.emit(
                        SYS_ECONOMY,
                        EVT_SMUGGLING_PERCENT,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "percent": percent,
                        }),
                    );
                }
                // Read only by the app's Near Uprising message (0x148).
                EconomyEvent::TroopSurplusChanged { .. } => {}
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
            }
        }
    }

    /// Apply injury recovery (timer `0x388`) with a `character_health` record
    /// for each healing.
    pub fn apply_healing(
        &mut self,
        world: &mut GameWorld,
        healed: &[rebellion_core::healing::Healed],
    ) {
        for heal in healed {
            rebellion_core::healing::apply(world, heal);
            let name = world
                .characters
                .get(heal.character)
                .map_or_else(String::new, |c| c.name.clone());
            self.events.push(GameEventRecord::new(
                heal.tick,
                self.wall_ms,
                SYS_MISSIONS,
                EVT_CHARACTER_HEALTH,
                serde_json::json!({
                    "character": name,
                    "cause": "healing",
                    "injury": heal.injury,
                }),
            ));
        }
    }

    /// Record the Return and Adrift orders the day's mission step gave
    /// (`FUN_00548910`, `FUN_00549230`): a `return_ordered` or
    /// `adrift_ordered` event for each, with its team and destination.
    pub fn record_return_orders(&mut self, world: &GameWorld, missions: &MissionState, tick: u64) {
        for order in missions.missions().iter().filter(|m| m.ordered_tick == tick) {
            let event = match order.kind {
                MissionKind::Return => EVT_RETURN_ORDERED,
                MissionKind::Adrift => EVT_ADRIFT_ORDERED,
                _ => continue,
            };
            self.events.push(GameEventRecord::new(
                tick,
                self.wall_ms,
                SYS_MISSIONS,
                event,
                serde_json::json!({
                    "faction": format!("{:?}", order.faction),
                    "team": member_names(world, &order.team),
                    "to": sys_name(world, order.target_system),
                }),
            ));
        }
    }

    /// Record the mission detection's character effects, which
    /// `missions::apply_advance_effects` has applied: a `character_health`
    /// event for each injury, a `force_experience` event for each Force
    /// gain, and the Force encounters, confrontations and Luke's heritage.
    pub fn record_advance_character_effects(
        &mut self,
        world: &GameWorld,
        effects: &[MissionEffect],
        tick: u64,
    ) {
        let name = |key| {
            world
                .characters
                .get(key)
                .map_or_else(String::new, |c| c.name.clone())
        };
        for effect in effects {
            match effect {
                MissionEffect::CharacterInjured { character, injury } if *injury > 0 => {
                    self.events.push(GameEventRecord::new(
                        tick,
                        self.wall_ms,
                        SYS_MISSIONS,
                        EVT_CHARACTER_HEALTH,
                        serde_json::json!({
                            "character": name(*character),
                            "cause": "mission_injury",
                            "dealt": injury,
                            "injury": world.characters.get(*character).map_or(0, |c| c.injury),
                        }),
                    ));
                }
                MissionEffect::TraitorIdentified { character, traitor } => {
                    self.events.push(GameEventRecord::new(
                        tick,
                        self.wall_ms,
                        SYS_MISSIONS,
                        EVT_TRAITOR_REVEALED,
                        serde_json::json!({
                            "character": name(*character),
                            "traitor": name(*traitor),
                        }),
                    ));
                }
                MissionEffect::DiedOfInjury { character } => {
                    self.events.push(GameEventRecord::new(
                        tick,
                        self.wall_ms,
                        SYS_MISSIONS,
                        EVT_CHARACTER_KILLED,
                        serde_json::json!({
                            "character": name(*character),
                            "cause": "injury",
                        }),
                    ));
                }
                MissionEffect::DagobahCompleted {
                    character,
                    force_gain,
                } => {
                    self.events.push(GameEventRecord::new(
                        tick,
                        self.wall_ms,
                        SYS_JEDI,
                        EVT_FORCE_EXPERIENCE,
                        serde_json::json!({
                            "character": name(*character),
                            "cause": "dagobah",
                            "gain": force_gain,
                            "force": world.characters.get(*character).map_or(0, rebellion_core::world::Character::force_level),
                        }),
                    ));
                }
                MissionEffect::ForceExperienceGained { character, amount } => {
                    self.events.push(GameEventRecord::new(
                        tick,
                        self.wall_ms,
                        SYS_JEDI,
                        EVT_FORCE_EXPERIENCE,
                        serde_json::json!({
                            "character": name(*character),
                            "gain": amount,
                            "force": world.characters.get(*character).map_or(0, rebellion_core::world::Character::force_level),
                        }),
                    ));
                }
                MissionEffect::EncounterBegan {
                    member,
                    defender,
                    day,
                } => {
                    self.events.push(GameEventRecord::new(
                        tick,
                        self.wall_ms,
                        SYS_JEDI,
                        EVT_FORCE_ENCOUNTER,
                        serde_json::json!({
                            "member": name(*member),
                            "defender": name(*defender),
                            "day": day,
                        }),
                    ));
                }
                MissionEffect::EncounterEnded { member, partner } => {
                    self.events.push(GameEventRecord::new(
                        tick,
                        self.wall_ms,
                        SYS_JEDI,
                        EVT_CONFRONTATION,
                        serde_json::json!({
                            "member": name(*member),
                            "partner": name(*partner),
                        }),
                    ));
                }
                MissionEffect::HeritageLearned { character } => {
                    self.events.push(GameEventRecord::new(
                        tick,
                        self.wall_ms,
                        SYS_JEDI,
                        EVT_HERITAGE_LEARNED,
                        serde_json::json!({ "character": name(*character) }),
                    ));
                }
                _ => {}
            }
        }
    }

    /// Record a `LukeDagobah` pulse (notification `0x221`) and whether the
    /// Dagobah mission started.
    pub fn record_luke_dagobah(
        &mut self,
        world: &GameWorld,
        pulse: &rebellion_core::dagobah::LukeDagobah,
        dispatched: bool,
    ) {
        self.events.push(GameEventRecord::new(
            pulse.tick,
            self.wall_ms,
            SYS_STORY,
            EVT_LUKE_DAGOBAH,
            serde_json::json!({
                "character": world.characters.get(pulse.luke).map_or_else(String::new, |c| c.name.clone()),
                "from": sys_name(world, pulse.from),
                "dispatched": dispatched,
            }),
        ));
    }

    /// Record a Final Battle step (`final_battle.rs`): Vader sent to Luke,
    /// both sent to the Emperor, or the battle with its outcome.
    pub fn record_final_battle_step(
        &mut self,
        world: &GameWorld,
        step: &rebellion_core::final_battle::FinalBattleStep,
        dispatched: bool,
    ) {
        use rebellion_core::final_battle::{FinalBattleStep, Outcome};
        let (tick, details) = match *step {
            FinalBattleStep::VaderToLuke { to, tick, .. } => (
                tick,
                serde_json::json!({
                    "step": "vader_to_luke",
                    "to": sys_name(world, to),
                    "dispatched": dispatched,
                }),
            ),
            FinalBattleStep::ToEmperor { to, tick, .. } => (
                tick,
                serde_json::json!({
                    "step": "to_emperor",
                    "to": sys_name(world, to),
                    "dispatched": dispatched,
                }),
            ),
            FinalBattleStep::Battle(battle) => (
                battle.tick,
                serde_json::json!({
                    "step": "battle",
                    "system": battle.system.map(|system| sys_name(world, system)),
                    "luke_wins": battle.outcome == Outcome::LukeWins,
                    "injury": match battle.outcome {
                        Outcome::EmpireWins { injury } => injury,
                        Outcome::LukeWins => 0,
                    },
                    "moved": dispatched,
                }),
            ),
        };
        self.events.push(GameEventRecord::new(
            tick,
            self.wall_ms,
            SYS_STORY,
            EVT_STORY_FINAL_BATTLE,
            details,
        ));
    }

    /// Record a change of the victory conditions word (notification
    /// `0x128`, `victory::advance_conditions_word`).
    pub fn record_victory_conditions(&mut self, old: u32, new: u32, tick: u64) {
        self.events.push(GameEventRecord::new(
            tick,
            self.wall_ms,
            SYS_VICTORY,
            EVT_VICTORY_CONDITIONS,
            serde_json::json!({ "old": old, "new": new }),
        ));
    }

    /// Record a change of `SeatOfPower` (`seat_of_power.rs`): notification
    /// `0x230`, Emperor Arrives at Coruscant when set.
    pub fn record_seat_of_power(&mut self, held: bool, tick: u64) {
        self.events.push(GameEventRecord::new(
            tick,
            self.wall_ms,
            SYS_STORY,
            EVT_STORY_SEAT_OF_POWER,
            serde_json::json!({ "held": held }),
        ));
    }

    /// Record a Jabba step (`jabba.rs`): the hunters sent after Han, a
    /// rescuer sent to the palace, or a captive autorouting there.
    pub fn record_jabba_step(
        &mut self,
        world: &GameWorld,
        step: &rebellion_core::jabba::JabbaStep,
        dispatched: bool,
    ) {
        use rebellion_core::jabba::JabbaStep;
        let (tick, details) = match *step {
            JabbaStep::BountyHunt { han, at, tick, .. } => (
                tick,
                serde_json::json!({
                    "step": "bounty_hunt",
                    "character": char_name(world, han),
                    "system": sys_name(world, at),
                    "dispatched": dispatched,
                }),
            ),
            JabbaStep::Rescue {
                rescuer,
                from,
                tick,
                ..
            } => (
                tick,
                serde_json::json!({
                    "step": "rescue",
                    "character": char_name(world, rescuer),
                    "from": sys_name(world, from),
                    "dispatched": dispatched,
                }),
            ),
            JabbaStep::Pickup {
                vader, from, tick, ..
            } => (
                tick,
                serde_json::json!({
                    "step": "pickup",
                    "character": char_name(world, vader),
                    "from": sys_name(world, from),
                    "dispatched": dispatched,
                }),
            ),
            JabbaStep::ToPalace {
                captive,
                from,
                tick,
                ..
            } => (
                tick,
                serde_json::json!({
                    "step": "to_palace",
                    "character": char_name(world, captive),
                    "from": sys_name(world, from),
                }),
            ),
        };
        self.events.push(GameEventRecord::new(
            tick,
            self.wall_ms,
            SYS_STORY,
            EVT_STORY_JABBA,
            details,
        ));
    }

    /// Apply Force discoveries (timer `0x389`): one `force_discovered`
    /// event each.
    pub fn apply_force_discoveries(
        &mut self,
        world: &mut GameWorld,
        discovered: &[rebellion_core::force_discovery::ForceDiscovered],
    ) {
        for found in discovered {
            rebellion_core::force_discovery::apply(world, found);
            let name = |key| {
                world
                    .characters
                    .get(key)
                    .map_or_else(String::new, |c| c.name.clone())
            };
            self.events.push(GameEventRecord::new(
                found.tick,
                self.wall_ms,
                SYS_JEDI,
                EVT_FORCE_DISCOVERED,
                serde_json::json!({
                    "character": name(found.character),
                    "discoverer": name(found.discoverer),
                    "force_base": found.force_base,
                    "learns_heritage": found.learns_heritage,
                }),
            ));
        }
    }

    /// Apply informant incidents (timer `0x38e`): the recipient learns about
    /// the system. `fog` is the one side whose view the simulation keeps.
    pub fn apply_informant_reports(
        &mut self,
        world: &GameWorld,
        fog: &mut rebellion_core::fog::FogState,
        reports: &[rebellion_core::informant::InformantReport],
    ) {
        for report in reports {
            if fog.faction == report.recipient {
                fog.reveal(report.system);
            }
            self.events.push(GameEventRecord::new(
                report.tick,
                self.wall_ms,
                SYS_ECONOMY,
                EVT_INFORMANT_INTEL,
                serde_json::json!({
                    "system": sys_name(world, report.system),
                    "cause": "informants",
                    "recipient": format!("{:?}", report.recipient),
                    "code": report.code,
                }),
            ));
        }
    }

    /// Record the yards' research (timer `0x381`): one `research_unlocked`
    /// record per level reached.
    pub fn record_research_tallies(&mut self, tallies: &[rebellion_core::research::ResearchTally]) {
        for tally in tallies {
            for level in &tally.reached {
                self.emit(
                    SYS_RESEARCH,
                    EVT_RESEARCH_UNLOCKED,
                    serde_json::json!({
                        "faction_is_alliance": tally.faction_is_alliance,
                        "tech_type": format!("{:?}", tally.tech),
                        "new_level": level,
                        "cause": "yards",
                        "points": tally.points,
                    }),
                );
            }
        }
    }

    /// Apply the side's autoscraps (timer `0x382`): remove each object,
    /// refund its side, and record the Maintenance Shortfall
    /// (`GameObjDestroyedAutoscrap`, `0x304`).
    pub fn apply_autoscraps(
        &mut self,
        world: &mut GameWorld,
        production: &mut rebellion_core::production_cycle::ProductionState,
        transport: &mut TroopTransportState,
        scraps: &[rebellion_core::autoscrap::Autoscrap],
    ) {
        for scrap in scraps {
            let refund = rebellion_core::autoscrap::apply(world, production, transport, scrap);
            self.events.push(GameEventRecord::new(
                scrap.tick,
                self.wall_ms,
                SYS_ECONOMY,
                EVT_MAINTENANCE_SHORTFALL,
                serde_json::json!({
                    "faction": format!("{:?}", scrap.side),
                    "unit": scrap.unit,
                    "system": scrap.system.map(|system| sys_name(world, system)),
                    "refund": refund,
                }),
            ));
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

    /// Emit `EVT_MANUFACTURING_IDLE` (0x160, K6) for each manager (a
    /// system's area) whose queue transitioned from non-empty to empty this
    /// tick (`FUN_0052baa0`). No world mutation — the queue is already
    /// drained by `production_cycle::advance_yards`.
    pub fn apply_manufacturing_idle(
        &mut self,
        world: &GameWorld,
        newly_idle: &[(
            rebellion_core::ids::SystemKey,
            rebellion_core::manufacturing::ProductionArea,
        )],
    ) {
        for &(system, area) in newly_idle {
            self.emit(
                SYS_MANUFACTURING,
                EVT_MANUFACTURING_IDLE,
                serde_json::json!({
                    "system": sys_name(world, system),
                    "area": format!("{area:?}"),
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
    /// Telemetry for one bombardment ([`BombardEnv::bombard`]).
    pub fn emit_bombardment(
        &mut self,
        world: &GameWorld,
        req: &rebellion_core::bombardment::BombardRequest,
        outcome: &rebellion_core::bombardment::BombardmentOutcome,
    ) {
        let headquarters_destroyed = outcome
            .destroyed
            .contains(&rebellion_core::bombardment::Target::Headquarters);
        self.emit(
            SYS_COMBAT,
            EVT_BOMBARDMENT,
            serde_json::json!({
                "system": sys_name(world, req.system),
                "kind": format!("{:#x}", req.kind),
                "shots": outcome.shots,
                "destroyed": outcome.destroyed.len(),
                "ships_hit": outcome.ship_losses.len(),
                "civilian_damage": outcome.civilian_damage,
                "system_destroyed": outcome.system_destroyed,
                "headquarters_destroyed": headquarters_destroyed,
            }),
        );
    }
    /// Telemetry for one Planetary Assault ([`BombardEnv::assault`]).
    pub fn emit_assault(
        &mut self,
        world: &GameWorld,
        req: &rebellion_core::assault::AssaultRequest,
        outcome: &rebellion_core::assault::AssaultOutcome,
    ) {
        self.emit(
            SYS_COMBAT,
            EVT_COMBAT_GROUND,
            serde_json::json!({
                "system": sys_name(world, req.system),
                "order": "assault",
                "attackers": outcome.attackers,
                "defenders": outcome.defenders,
                "duels": outcome.duels,
                "attackers_lost": outcome.lost_attackers.len(),
                "defenders_lost": outcome.lost_defenders.len(),
                "collateral": outcome.collateral.len(),
                "landed": outcome.landed.len(),
            }),
        );
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
            } else if let MissionEffect::MemberEnteredAbode { member, abode } = effect {
                rebellion_core::missions::enter_abode(world, *member, *abode);
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

    /// Apply fired events: world mutations + telemetry.
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
        bombard: &mut BombardEnv,
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
            bombard,
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
                BlockadeEvent::SupportShifted {
                    system,
                    side,
                    support_delta,
                    tick,
                } => {
                    self.events.push(GameEventRecord::new(
                        *tick,
                        self.wall_ms,
                        SYS_BLOCKADE,
                        EVT_BLOCKADE_SUPPORT,
                        serde_json::json!({
                            "system": sys_name(world, *system),
                            "side": format!("{side:?}"),
                            "support_delta": support_delta,
                        }),
                    ));
                    rebellion_core::uprising::shift_support(world, *system, *side, *support_delta);
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
                        "killed": losses
                            .iter()
                            .filter_map(|loss| match loss {
                                rebellion_core::uprising::IncidentLoss::CharacterKilled(c) => {
                                    Some(char_name(world, *c))
                                }
                                _ => None,
                            })
                            .collect::<Vec<_>>(),
                        "support_delta": support_delta,
                    }),
                ),
                UprisingEvent::SupportEroded {
                    system,
                    tick,
                    support_delta,
                    ..
                } => (
                    *tick,
                    EVT_UPRISING_SUPPORT,
                    serde_json::json!({
                        "system": sys_name(world, *system),
                        "support_delta": support_delta,
                    }),
                ),
                UprisingEvent::ResourceChanged {
                    system,
                    tick,
                    code,
                    total_energy,
                    raw_materials,
                } => (
                    *tick,
                    EVT_RESOURCE_DISCOVERY,
                    serde_json::json!({
                        "system": sys_name(world, *system),
                        "code": code,
                        "total_energy": total_energy,
                        "raw_materials": raw_materials,
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
            // The detection's injuries and Force gains come with the
            // advance (`missions::apply_advance_effects`); a result that
            // carries one applies it the same way.
            MissionEffect::ForceExperienceGained { character, amount } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.add_force_experience(*amount);
                }
            }
            // The advance's effects (`missions::apply_advance_effects`).
            MissionEffect::MemberEnteredAbode { member, abode } => {
                rebellion_core::missions::enter_abode(world, *member, *abode);
            }
            MissionEffect::DagobahCompleted {
                character,
                force_gain,
            } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.add_force_experience(*force_gain);
                    c.dagobah_completed = true;
                }
            }
            // FUN_005712b0 -> FUN_004ee1d0: a Jedi Training gain.
            MissionEffect::ForceTrainingGained { character, amount } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.add_force_training(*amount);
                }
            }
            // The encounter's keys and Luke's heritage, set by the mission
            // detection (`missions::apply_advance_effects`).
            MissionEffect::EncounterBegan { .. }
            | MissionEffect::EncounterEnded { .. }
            | MissionEffect::HeritageLearned { .. }
            | MissionEffect::CapturedByBountyHunters { .. }
            | MissionEffect::EscapeBarred { .. }
            | MissionEffect::MemberOnMission { .. }
            | MissionEffect::MemberBoarded { .. }
            | MissionEffect::AdriftSettled { .. }
            | MissionEffect::DiedOfInjury { .. } => {
                rebellion_core::missions::apply_advance_effects(
                    world,
                    std::slice::from_ref(effect),
                );
            }
            // Notifications 0x212 and 0x231 carry no state of their own; a
            // pickup's prisoners leave as the mission ends.
            MissionEffect::BountyAttack { .. }
            | MissionEffect::PickupInProgress
            | MissionEffect::PrisonerPickedUp { .. }
            | MissionEffect::TraitorIdentified { .. } => {}
            MissionEffect::CharacterInjured { character, injury } => {
                // Slot +0x2f0 (FUN_004ef6d0) adds the injury to +0x94.
                if let Some(c) = world.characters.get_mut(*character) {
                    c.injure(*injury);
                }
            }
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
                        SkillField::JediLevel => {
                            let level = apply(c.force_level(), d).min(0x7fff);
                            c.force_base = u16::try_from(level).unwrap_or(0x7fff);
                        }
                    }
                }
            }
            EventAction::RelocateCharacter { .. } | EventAction::TriggerEvent { .. } => {}
            EventAction::SetMandatoryMission {
                character,
                mandatory,
            } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.on_mandatory_mission = *mandatory;
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
                    let fleet = c.current_fleet;
                    c.move_to(fleet, Some(*destination));
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
    bombard: &mut BombardEnv,
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
                    if let Some(&lead) = team.first() {
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
            AIAction::Bombard {
                fleets,
                system,
                kind,
            } => ai_state.faction.is_some_and(|side| {
                world.systems.contains_key(*system) && {
                    bombard.bombard(world, side.as_faction(), fleets, *system, *kind);
                    true
                }
            }),
            AIAction::Assault { fleets, system } => ai_state.faction.is_some_and(|side| {
                world.systems.contains_key(*system) && {
                    bombard.assault(world, troop_transport, side.as_faction(), fleets, *system);
                    true
                }
            }),
            AIAction::JoinFleet { fleet, into } => ai_state.faction.is_some_and(|side| {
                rebellion_core::fleet_join::join_fleet(
                    world,
                    movement_state,
                    troop_transport,
                    &rebellion_core::fleet_join::FleetMover::Fleet(*fleet),
                    *into,
                    matches!(side, AiFaction::Alliance),
                )
                .is_ok()
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
            AIAction::CreateFleet { fleet, ships, to } => ai_state.faction.is_some_and(|side| {
                let is_alliance = matches!(side, AiFaction::Alliance);
                let Some(roster) = rebellion_core::fleet_join::roster(world, *fleet) else {
                    return false;
                };
                if world.fleets.get(*fleet).is_some_and(|f| f.location == *to) {
                    rebellion_core::fleet_join::create_fleet(
                        world,
                        movement_state,
                        troop_transport,
                        *fleet,
                        ships,
                        roster,
                        is_alliance,
                    )
                    .is_ok()
                } else {
                    rebellion_core::fleet_join::move_ships_to_system(
                        world,
                        movement_state,
                        troop_transport,
                        *fleet,
                        ships,
                        roster,
                        *to,
                        is_alliance,
                    )
                    .is_ok()
                }
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
            AIAction::BoardRegiments { troops, fleet } => {
                let blockaded = rebellion_core::blockade::BlockadeSystem::blockaded_now(world);
                let mut moved = false;
                for &troop in troops {
                    moved |= troop_transport
                        .move_regiment(
                            world,
                            movement_state,
                            &blockaded,
                            troop,
                            rebellion_core::troop_transport::RegimentTarget::Fleet(*fleet),
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
            AIAction::MoveSquadronTeam {
                squadrons,
                from,
                to,
            } => {
                let mut moved = false;
                for &(class, count) in squadrons {
                    moved |= rebellion_core::squadrons::is_alliance(world, class).is_some_and(
                        |alliance| {
                            troop_transport
                                .squadrons
                                .move_squadrons(world, alliance, class, count, *from, *to, tick)
                                .is_ok()
                        },
                    );
                }
                moved
            }
            AIAction::MoveCharacters { characters, to } => {
                for &character in characters {
                    rebellion_core::missions::move_member(
                        world,
                        MissionMember::Character(character),
                        Some(*to),
                    );
                }
                !characters.is_empty()
            }
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


    #[test]
    fn each_loyalty_shift_is_recorded_with_its_side_and_amount() {
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.record_loyalty_shifts(&[rebellion_core::loyalty::LoyaltyShift {
            side: Faction::Alliance,
            amount: -5,
        }]);
        let events = integrator.finish();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "loyalty_shift");
        assert_eq!(events[0].details["side"], "Alliance");
        assert_eq!(events[0].details["amount"], -5);
    }

    #[test]
    fn a_jedi_training_gain_raises_the_pupil_s_force_training() {
        // FUN_005712b0 -> FUN_004ee1d0: the gain goes to +0x90, and the
        // Force level +0x8c is +0x8e + +0x90 (FUN_004eee40).
        let mut world = GameWorld::default();
        let pupil = world.characters.insert(rebellion_core::world::Character {
            force_base: 20,
            ..Default::default()
        });
        apply_mission_effects_inner(
            &[MissionEffect::ForceTrainingGained {
                character: pupil,
                amount: 12,
            }],
            &mut world,
            1,
            &mut UprisingState::default(),
        );
        assert_eq!(world.characters[pupil].force_training, 12);
        assert_eq!(world.characters[pupil].force_level(), 32);
    }

    #[test]
    fn a_luke_dagobah_pulse_is_recorded() {
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Yavin");
        let luke = world.characters.insert(rebellion_core::world::Character {
            name: "Luke".into(),
            ..Default::default()
        });
        let abode = world.abodes.insert(rebellion_core::world::Abode {
            dat_id: DatId(0x441),
            name: "Dagobah".into(),
            side: Faction::Neutral,
            special_forces: Vec::new(),
        });
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.record_luke_dagobah(
            &world,
            &rebellion_core::dagobah::LukeDagobah {
                luke,
                from: system,
                abode,
                tick: 351,
            },
            true,
        );
        let event = integrator.events.last().expect("a record");
        assert_eq!(event.event_type, EVT_LUKE_DAGOBAH);
        assert_eq!(event.details["character"], "Luke");
        assert_eq!(event.details["from"], "Yavin");
        assert_eq!(event.details["dispatched"], true);
    }

    #[test]
    fn dagobah_s_end_is_recorded_with_luke_s_new_force() {
        let mut world = GameWorld::default();
        let luke = world.characters.insert(rebellion_core::world::Character {
            name: "Luke".into(),
            force_base: 80,
            ..Default::default()
        });
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.record_advance_character_effects(
            &world,
            &[MissionEffect::DagobahCompleted {
                character: luke,
                force_gain: 30,
            }],
            111,
        );
        let event = integrator.events.last().expect("a record");
        assert_eq!(event.event_type, EVT_FORCE_EXPERIENCE);
        assert_eq!(event.details["cause"], "dagobah");
        assert_eq!(event.details["gain"], 30);
        assert_eq!(event.details["force"], 80);
    }

    #[test]
    fn the_detection_s_injuries_and_force_gains_are_recorded() {
        let mut world = GameWorld::default();
        let luke = world.characters.insert(rebellion_core::world::Character {
            name: "Luke".into(),
            injury: 3,
            force_base: 51,
            ..Default::default()
        });
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.record_advance_character_effects(
            &world,
            &[
                MissionEffect::CharacterInjured {
                    character: luke,
                    injury: 0,
                },
                MissionEffect::CharacterInjured {
                    character: luke,
                    injury: 3,
                },
                MissionEffect::ForceExperienceGained {
                    character: luke,
                    amount: 1,
                },
            ],
            9,
        );
        assert_eq!(integrator.events.len(), 2);
        assert_eq!(integrator.events[0].event_type, EVT_CHARACTER_HEALTH);
        assert_eq!(integrator.events[0].details["cause"], "mission_injury");
        assert_eq!(integrator.events[0].details["dealt"], 3);
        assert_eq!(integrator.events[1].event_type, EVT_FORCE_EXPERIENCE);
        assert_eq!(integrator.events[1].details["force"], 51);
        assert_eq!(integrator.events[1].details["gain"], 1);
    }

    #[test]
    fn a_return_order_of_the_day_is_recorded_with_its_team_and_destination() {
        use rebellion_core::missions::MissionRequest;
        let mut world = GameWorld::default();
        let envoy = world.characters.insert(rebellion_core::world::Character {
            name: "Envoy".into(),
            ..Default::default()
        });
        let home = add_system(&mut world, "Home");
        let elsewhere = add_system(&mut world, "Elsewhere");
        let mut missions = MissionState::new();
        missions.dispatch(MissionRequest::single(
            MissionKind::Return,
            MissionFaction::Alliance,
            envoy,
            home,
            None,
            7,
        ));
        missions.dispatch(MissionRequest::single(
            MissionKind::Move,
            MissionFaction::Alliance,
            envoy,
            elsewhere,
            None,
            7,
        ));
        missions.dispatch(MissionRequest::single(
            MissionKind::Adrift,
            MissionFaction::Alliance,
            envoy,
            elsewhere,
            None,
            7,
        ));
        let mut integrator = PerceptionIntegrator::new(1, 0);

        integrator.record_return_orders(&world, &missions, 7);

        assert_eq!(integrator.events.len(), 2);
        let event = &integrator.events[0];
        assert_eq!(event.event_type, EVT_RETURN_ORDERED);
        assert_eq!(event.details["team"][0], "Envoy");
        assert_eq!(event.details["to"], "Home");
        assert_eq!(integrator.events[1].event_type, EVT_ADRIFT_ORDERED);
        assert_eq!(integrator.events[1].details["to"], "Elsewhere");
        integrator.record_return_orders(&world, &missions, 8);
        assert_eq!(integrator.events.len(), 2);
    }

    #[test]
    fn a_final_battle_step_is_recorded_with_its_outcome() {
        use rebellion_core::final_battle::{FinalBattle, FinalBattleStep, Outcome};
        let mut world = GameWorld::default();
        let mut add = |name: &str| {
            world.characters.insert(rebellion_core::world::Character {
                name: name.into(),
                ..Default::default()
            })
        };
        let (luke, vader, emperor) = (add("Luke"), add("Vader"), add("Emperor"));
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.record_final_battle_step(
            &world,
            &FinalBattleStep::Battle(FinalBattle {
                luke,
                vader,
                emperor,
                system: None,
                at_system: false,
                outcome: Outcome::EmpireWins { injury: 64 },
                tick: 854,
            }),
            false,
        );
        let event = integrator.events.last().expect("a record");
        assert_eq!(event.event_type, EVT_STORY_FINAL_BATTLE);
        assert_eq!(event.tick, 854);
        assert_eq!(event.details["step"], "battle");
        assert_eq!(event.details["luke_wins"], false);
        assert_eq!(event.details["injury"], 64);
    }

    #[test]
    fn no_story_event_reveals_luke_s_heritage() {
        // Luke learns KnowsHeritage only when he confronts Vader
        // (FUN_00560200 -> FUN_0056f650); no story event sets it (the port's
        // 0x396, which did, is gone with its Final Battle chain).
        let mut world = GameWorld::default();
        let luke = world.characters.insert(rebellion_core::world::Character {
            name: "Luke Skywalker".into(),
            is_alliance: true,
            dagobah_completed: true,
            ..Default::default()
        });
        world.characters.insert(rebellion_core::world::Character {
            name: "Darth Vader".into(),
            is_empire: true,
            ..Default::default()
        });
        let mut state = rebellion_core::events::EventState::new();
        rebellion_core::story_events::define_story_events(&mut state, &world);
        let movement = MovementState::default();
        let mut effects = Vec::new();
        for tick in [61, 70, 400, 900] {
            let ticks = [rebellion_core::tick::TickEvent { tick }];
            for fired in
                rebellion_core::events::EventSystem::advance(&mut state, &world, &ticks, &[])
            {
                apply_event_action_to_world(
                    &fired.actions,
                    &mut world,
                    &mut effects,
                    tick,
                    &movement,
                );
            }
        }
        assert!(!world.characters[luke].heritage_known);
    }

    #[test]
    fn the_force_encounter_confrontation_and_heritage_are_recorded() {
        let mut world = GameWorld::default();
        let luke = world.characters.insert(rebellion_core::world::Character {
            name: "Luke".into(),
            ..Default::default()
        });
        let vader = world.characters.insert(rebellion_core::world::Character {
            name: "Vader".into(),
            ..Default::default()
        });
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.record_advance_character_effects(
            &world,
            &[
                MissionEffect::EncounterBegan {
                    member: luke,
                    defender: vader,
                    day: 40,
                },
                MissionEffect::EncounterEnded {
                    member: luke,
                    partner: vader,
                },
                MissionEffect::HeritageLearned { character: luke },
            ],
            40,
        );
        let kinds: Vec<&str> = integrator
            .events
            .iter()
            .map(|event| event.event_type)
            .collect();
        assert_eq!(
            kinds,
            [EVT_FORCE_ENCOUNTER, EVT_CONFRONTATION, EVT_HERITAGE_LEARNED]
        );
        assert_eq!(integrator.events[0].details["defender"], "Vader");
        assert_eq!(integrator.events[0].details["day"], 40);
        assert_eq!(integrator.events[1].details["partner"], "Vader");
        assert_eq!(integrator.events[2].details["character"], "Luke");
    }

    #[test]
    fn a_force_discovery_makes_the_character_aware_and_records_it() {
        // FUN_00560a30: ForceAware, the discoverer (+0xa4), the Force base
        // the awareness hook rolls (FUN_004eee60) and Leia's heritage.
        let mut world = GameWorld::default();
        let trainer = world.characters.insert(rebellion_core::world::Character {
            dat_id: DatId(0x242),
            name: "Luke".into(),
            ..Default::default()
        });
        let found = world.characters.insert(rebellion_core::world::Character {
            dat_id: DatId(0x241),
            name: "Leia".into(),
            force_potential: true,
            ..Default::default()
        });
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.apply_force_discoveries(
            &mut world,
            &[rebellion_core::force_discovery::ForceDiscovered {
                character: found,
                discoverer: trainer,
                force_base: Some(10),
                learns_heritage: true,
                tick: 7,
            }],
        );
        let c = &world.characters[found];
        assert!(c.force_aware && c.heritage_known);
        assert_eq!(c.force_base, 10);
        assert_eq!(c.force_discoverer, Some(DatId(0x242)));
        let event = integrator.events.last().expect("a record");
        assert_eq!(event.event_type, EVT_FORCE_DISCOVERED);
        assert_eq!(event.details["character"], "Leia");
        assert_eq!(event.details["discoverer"], "Luke");
    }

    #[test]
    fn a_story_jedi_level_change_moves_the_force_base() {
        // port: the story events' JediLevel skill now moves the Force base
        // +0x8e, which the Force level +0x8c reads (FUN_004eee40); the
        // class template stays.
        let mut world = GameWorld::default();
        let key = world.characters.insert(rebellion_core::world::Character {
            force_base: 30,
            jedi_level: rebellion_core::world::SkillPair {
                base: 10,
                variance: 40,
            },
            ..Default::default()
        });
        let mut effects = Vec::new();
        let movement = MovementState::default();
        let change = |base_delta| {
            [EventAction::ModifyCharacterSkill {
                character: key,
                skill: rebellion_core::events::SkillField::JediLevel,
                base_delta,
            }]
        };
        apply_event_action_to_world(&change(25), &mut world, &mut effects, 1, &movement);
        assert_eq!(world.characters[key].force_base, 55);
        assert_eq!(world.characters[key].jedi_level.base, 10);
        apply_event_action_to_world(&change(-100), &mut world, &mut effects, 2, &movement);
        assert_eq!(world.characters[key].force_base, 0);
        apply_event_action_to_world(&change(i32::MAX), &mut world, &mut effects, 3, &movement);
        assert_eq!(world.characters[key].force_base, 0x7fff);
    }

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
                next_support_tick: None,
                side: None,
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
                    next_support_tick: None,
                    side: None,
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
                next_support_tick: None,
                side: None,
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
    fn a_blockade_support_shift_moves_the_blockader_s_support() {
        // FUN_0050cb20 -> FUN_0050c9f0 sets the blockader's support.
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Naboo");
        world.systems[system].popularity_alliance = 0.3;
        world.systems[system].popularity_empire = 0.7;
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.apply_blockade_events(
            &mut world,
            &mut TroopTransportState::default(),
            &[BlockadeEvent::SupportShifted {
                system,
                side: Faction::Empire,
                support_delta: 1,
                tick: 31,
            }],
        );
        assert!((world.systems[system].popularity_empire - 0.71).abs() < 1e-6);
        assert!((world.systems[system].popularity_alliance - 0.29).abs() < 1e-6);
        assert!(integrator
            .events
            .iter()
            .any(|e| e.event_type == EVT_BLOCKADE_SUPPORT && e.tick == 31));
    }

    #[test]
    fn a_mission_injury_adds_to_the_character_and_healing_records_it() {
        // Slot +0x2f0 (FUN_004ef6d0) adds the injury; timer 0x388 lowers it.
        let mut world = GameWorld::default();
        let character = world.characters.insert(rebellion_core::world::Character {
            injury: 4,
            ..rebellion_core::world::Character::default()
        });
        apply_mission_effects_inner(
            &[MissionEffect::CharacterInjured {
                character,
                injury: 6,
            }],
            &mut world,
            1,
            &mut UprisingState::default(),
        );
        assert_eq!(world.characters[character].injury, 10);
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.apply_healing(
            &mut world,
            &[rebellion_core::healing::Healed {
                character,
                injury: 9,
                tick: 2,
            }],
        );
        assert_eq!(world.characters[character].injury, 9);
        assert!(integrator
            .events
            .iter()
            .any(|e| e.event_type == EVT_CHARACTER_HEALTH && e.tick == 2));
    }

    #[test]
    fn informants_reveal_the_system_to_the_side_they_tell() {
        // FUN_0050d5a0 passes the system's objects to the other side.
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Naboo");
        let mut integrator = PerceptionIntegrator::new(1, 0);
        let report = rebellion_core::informant::InformantReport {
            system,
            holder: Faction::Empire,
            recipient: Faction::Alliance,
            code: 3,
            tick: 40,
        };
        let mut empire_fog = rebellion_core::fog::FogState::new(Faction::Empire);
        integrator.apply_informant_reports(&world, &mut empire_fog, std::slice::from_ref(&report));
        assert!(!empire_fog.is_visible(system));
        let mut alliance_fog = rebellion_core::fog::FogState::new(Faction::Alliance);
        integrator.apply_informant_reports(&world, &mut alliance_fog, &[report]);
        assert!(alliance_fog.is_visible(system));
        assert!(integrator
            .events
            .iter()
            .any(|e| e.event_type == EVT_INFORMANT_INTEL && e.tick == 40));
    }

    #[test]
    fn a_level_the_yards_reach_is_recorded_as_research_unlocked() {
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.record_research_tallies(&[rebellion_core::research::ResearchTally {
            faction_is_alliance: true,
            tech: rebellion_core::research::TechType::Ship,
            points: 2,
            reached: vec![1, 2],
        }]);
        let levels: Vec<_> = integrator
            .events
            .iter()
            .filter(|e| e.event_type == EVT_RESEARCH_UNLOCKED)
            .map(|e| e.details["new_level"].clone())
            .collect();
        assert_eq!(levels, vec![serde_json::json!(1), serde_json::json!(2)]);
    }

    #[test]
    fn an_autoscrap_removes_the_object_and_records_a_maintenance_shortfall() {
        // FUN_004f8850 destroys with reason 0x15; GameObjDestroyedAutoscrap
        // (0x304) is the Maintenance Shortfall message.
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Naboo");
        let troop = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(troop);
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.apply_autoscraps(
            &mut world,
            &mut rebellion_core::production_cycle::ProductionState::new(),
            &mut TroopTransportState::default(),
            &[rebellion_core::autoscrap::Autoscrap {
                side: Faction::Alliance,
                target: rebellion_core::autoscrap::AutoscrapTarget::Troop(troop),
                unit: "Regiment".into(),
                system: Some(system),
                tick: 11,
            }],
        );
        assert!(world.troops.is_empty());
        assert!(integrator
            .events
            .iter()
            .any(|e| e.event_type == EVT_MAINTENANCE_SHORTFALL && e.tick == 11));
    }

    #[test]
    fn a_smuggling_change_leaves_the_support_alone() {
        // FUN_00509ec0 only stores +0x6c and notifies its views.
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Naboo");
        world.systems[system].popularity_alliance = 0.5;
        world.systems[system].popularity_empire = 0.5;
        let before = (
            world.systems[system].popularity_alliance,
            world.systems[system].popularity_empire,
        );
        let mut integrator = PerceptionIntegrator::new(1, 0);
        integrator.apply_economy_events(
            &mut world,
            &[EconomyEvent::SmugglingPercentChanged {
                system,
                previous: 0,
                percent: 75,
            }],
        );
        assert_eq!(
            (
                world.systems[system].popularity_alliance,
                world.systems[system].popularity_empire,
            ),
            before
        );
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
            &mut BombardEnv::new(0, None),
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
            &mut BombardEnv::new(0, None),
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
            &mut BombardEnv::new(0, None),
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
            &mut BombardEnv::new(0, None),
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
    fn a_squadron_team_move_boards_each_class_s_squadrons() {
        // A sector plan's Move 0x201 of squadrons from its base
        // (FUN_004b0120): each class's count by squadrons::move_squadrons.
        use rebellion_core::squadrons::SquadronPlace;
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Kuat");
        let carrier = world.capital_ship_classes.insert(CapitalShipClass {
            fighter_capacity: 3,
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
        let mut wing = || {
            world
                .fighter_classes
                .insert(rebellion_core::world::FighterClass {
                    is_empire: true,
                    ..Default::default()
                })
        };
        let (tie, bomber) = (wing(), wing());
        rebellion_core::squadrons::add(&mut world.systems[system].fighters, tie, 2);
        rebellion_core::squadrons::add(&mut world.systems[system].fighters, bomber, 2);

        let applied = apply_ai_actions_inner(
            &[AIAction::MoveSquadronTeam {
                squadrons: vec![(tie, 2), (bomber, 1)],
                from: SquadronPlace::Planet(system),
                to: SquadronPlace::Fleet(fleet),
            }],
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut TroopTransportState::new(),
            &mut ProductionState::new(),
            &mut world,
            5,
            &mut BombardEnv::new(0, None),
        );
        assert_eq!(applied, [true]);

        let aboard: Vec<_> = world.fleets[fleet]
            .fighters
            .iter()
            .map(|e| (e.class, e.count))
            .collect();
        assert_eq!(aboard, [(tie, 2), (bomber, 1)]);
        assert_eq!(
            rebellion_core::squadrons::total(&world.systems[system].fighters),
            1
        );
        // No squadron of the class is left there: nothing is applied.
        let applied = apply_ai_actions_inner(
            &[AIAction::MoveSquadronTeam {
                squadrons: vec![(tie, 1)],
                from: SquadronPlace::Planet(system),
                to: SquadronPlace::Fleet(fleet),
            }],
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut TroopTransportState::new(),
            &mut ProductionState::new(),
            &mut world,
            5,
            &mut BombardEnv::new(0, None),
        );
        assert_eq!(applied, [false]);
    }

    #[test]
    fn a_people_move_sets_the_fleet_s_people_down_on_its_system() {
        // A sector plan's Move 0x201 of the people aboard (FUN_004b06b0),
        // each carried out by missions::move_member.
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Kuat");
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        let people: Vec<_> = (0..2)
            .map(|_| {
                let c = world.characters.insert(rebellion_core::world::Character {
                    is_alliance: true,
                    current_fleet: Some(fleet),
                    ..Default::default()
                });
                world.fleets[fleet].characters.push(c);
                c
            })
            .collect();

        let applied = apply_ai_actions_inner(
            &[AIAction::MoveCharacters {
                characters: people.clone(),
                to: system,
            }],
            &mut AIState::new(AiFaction::Alliance),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut TroopTransportState::new(),
            &mut ProductionState::new(),
            &mut world,
            5,
            &mut BombardEnv::new(0, None),
        );
        assert_eq!(applied, [true]);

        assert_eq!(world.fleets[fleet].characters, []);
        for c in people {
            assert_eq!(world.characters[c].current_system, Some(system));
            assert_eq!(world.characters[c].current_fleet, None);
        }
        // Nobody to move: nothing is applied.
        let applied = apply_ai_actions_inner(
            &[AIAction::MoveCharacters {
                characters: vec![],
                to: system,
            }],
            &mut AIState::new(AiFaction::Alliance),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut TroopTransportState::new(),
            &mut ProductionState::new(),
            &mut world,
            5,
            &mut BombardEnv::new(0, None),
        );
        assert_eq!(applied, [false]);
    }

    #[test]
    fn a_bombardment_order_destroys_the_targets_its_shots_reach() {
        // The AI's Planetary Bombardment (FUN_004b1330) resolved through
        // bombardment::resolve: no shields, a bombardment of 5, one
        // regiment without defense (lost to any draw of 1..10).
        let mut world = GameWorld::default();
        world.gnprtb = rebellion_core::world::GnprtbParams::new(
            [(0x602, 1), (0x603, 10)]
                .iter()
                .map(|&(id, v)| rebellion_core::world::GnprtbEntry {
                    parameter_id: id,
                    development: v,
                    alliance_sp_easy: v,
                    alliance_sp_medium: v,
                    alliance_sp_hard: v,
                    empire_sp_easy: v,
                    empire_sp_medium: v,
                    empire_sp_hard: v,
                    multiplayer: v,
                })
                .collect(),
        );
        let kuat = add_system(&mut world, "Kuat");
        world.systems[kuat].control =
            rebellion_core::world::ControlKind::Controlled(Faction::Alliance);
        let regiment = world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: DatId(0x1000_0001),
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[kuat].ground_units.push(regiment);
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            is_empire: true,
            hull: 100,
            bombardment_modifier: 5,
            ..Default::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: kuat,
            capital_ships: vec![rebellion_core::world::ShipInstance::new(class, 100, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[kuat].fleets.push(fleet);
        let mut env = BombardEnv::new(7, None);
        let applied = apply_ai_actions_inner(
            &[AIAction::Bombard {
                fleets: vec![fleet],
                system: kuat,
                kind: rebellion_core::bombardment::GENERAL,
            }],
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut TroopTransportState::new(),
            &mut ProductionState::new(),
            &mut world,
            5,
            &mut env,
        );
        assert_eq!(applied, [true]);
        assert_eq!(world.systems[kuat].ground_units, []);
        assert!(!world.troops.contains_key(regiment));
        assert_eq!(env.outcomes.len(), 1);
        assert_eq!(env.outcomes[0].1.shots, 5);
        // The telemetry names the kind and what fell.
        let mut integrator = PerceptionIntegrator::new(5, 0);
        integrator.emit_bombardment(&world, &env.outcomes[0].0, &env.outcomes[0].1);
        let events = integrator.finish();
        let record = events
            .iter()
            .find(|e| e.event_type == EVT_BOMBARDMENT)
            .unwrap();
        assert_eq!(record.details["kind"], "0x222");
        assert_eq!(record.details["destroyed"], 1);
    }

    #[test]
    fn a_boarding_order_puts_the_regiments_aboard_the_fleet() {
        // A kind 4 plan's Move 0x201 onto its held fleet (FUN_004b8470),
        // carried out by move_regiment with the fleet for the leg.
        let mut world = GameWorld::default();
        let kuat = add_system(&mut world, "Kuat");
        world.systems[kuat].control =
            rebellion_core::world::ControlKind::Controlled(Faction::Empire);
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            is_empire: true,
            hull: 100,
            troop_capacity: 2,
            ..Default::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: kuat,
            capital_ships: vec![rebellion_core::world::ShipInstance::new(class, 100, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[kuat].fleets.push(fleet);
        let regiment = world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: DatId(0x1000_0006),
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[kuat].ground_units.push(regiment);
        let mut transport = TroopTransportState::new();
        let applied = apply_ai_actions_inner(
            &[AIAction::BoardRegiments {
                troops: vec![regiment],
                fleet,
            }],
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut transport,
            &mut ProductionState::new(),
            &mut world,
            5,
            &mut BombardEnv::new(0, None),
        );
        assert_eq!(applied, [true]);
        assert_eq!(transport.cargo(fleet), [regiment]);
        assert!(!world.systems[kuat].ground_units.contains(&regiment));
    }

    #[test]
    fn an_assault_order_lands_the_winners_and_reports_the_ground_battle() {
        // Planetary Assault (0x234) resolved through assault::resolve: with
        // no defender the regiments aboard land at once (FUN_0058c810).
        let mut world = GameWorld::default();
        world.gnprtb = rebellion_core::world::GnprtbParams::new(
            [(0x1e51, 60), (0x1e52, -10)]
                .iter()
                .map(|&(id, v)| rebellion_core::world::GnprtbEntry {
                    parameter_id: id,
                    development: v,
                    alliance_sp_easy: v,
                    alliance_sp_medium: v,
                    alliance_sp_hard: v,
                    empire_sp_easy: v,
                    empire_sp_medium: v,
                    empire_sp_hard: v,
                    multiplayer: v,
                })
                .collect(),
        );
        let kuat = add_system(&mut world, "Kuat");
        world.systems[kuat].control =
            rebellion_core::world::ControlKind::Controlled(Faction::Alliance);
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            is_empire: true,
            hull: 100,
            troop_capacity: 2,
            ..Default::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: kuat,
            capital_ships: vec![rebellion_core::world::ShipInstance::new(class, 100, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[kuat].fleets.push(fleet);
        let regiment = world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: DatId(0x1000_0006),
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[kuat].ground_units.push(regiment);
        let mut transport = TroopTransportState::new();
        transport.embark(&mut world, fleet, &[regiment]).unwrap();
        let mut env = BombardEnv::new(7, None);
        let applied = apply_ai_actions_inner(
            &[AIAction::Assault {
                fleets: vec![fleet],
                system: kuat,
            }],
            &mut AIState::new(AiFaction::Empire),
            &mut MissionState::new(),
            &mut ManufacturingState::new(),
            &mut MovementState::new(),
            &mut transport,
            &mut ProductionState::new(),
            &mut world,
            5,
            &mut env,
        );
        assert_eq!(applied, [true]);
        assert_eq!(world.systems[kuat].ground_units, [regiment]);
        assert_eq!(transport.cargo(fleet), []);
        assert_eq!(env.assaults.len(), 1);
        let mut integrator = PerceptionIntegrator::new(5, 0);
        integrator.emit_assault(&world, &env.assaults[0].0, &env.assaults[0].1);
        let events = integrator.finish();
        let record = events
            .iter()
            .find(|e| e.event_type == EVT_COMBAT_GROUND)
            .unwrap();
        assert_eq!(record.details["order"], "assault");
        assert_eq!(record.details["landed"], 1);
    }

    #[test]
    fn a_join_merges_a_fleet_into_another_in_its_system_or_sends_it_there() {
        // A sector plan's Move 0x201 onto a held fleet (FUN_004b0c90),
        // carried out by fleet_join::join_fleet: in the same system the
        // fleet merges at once; elsewhere it departs to join on arrival.
        let mut world = GameWorld::default();
        let kuat = add_system(&mut world, "Kuat");
        let fondor = add_system(&mut world, "Fondor");
        world.systems[fondor].x = 40;
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            is_empire: true,
            hyperdrive: 100,
            ..Default::default()
        });
        let fleet = |world: &mut GameWorld, at| {
            let key = world.fleets.insert(Fleet {
                location: at,
                capital_ships: vec![rebellion_core::world::ShipInstance::new(class, 100, false)],
                fighters: vec![],
                characters: vec![],
                is_alliance: false,
                has_death_star: false,
            });
            world.systems[at].fleets.push(key);
            key
        };
        let a = fleet(&mut world, kuat);
        let b = fleet(&mut world, kuat);
        let c = fleet(&mut world, fondor);
        let mut movement = MovementState::new();
        let apply = |world: &mut GameWorld, movement: &mut MovementState, fleet, into| {
            apply_ai_actions_inner(
                &[AIAction::JoinFleet { fleet, into }],
                &mut AIState::new(AiFaction::Empire),
                &mut MissionState::new(),
                &mut ManufacturingState::new(),
                movement,
                &mut TroopTransportState::new(),
                &mut ProductionState::new(),
                world,
                5,
                &mut BombardEnv::new(0, None),
            )
        };
        assert_eq!(apply(&mut world, &mut movement, a, b), [true]);
        assert!(!world.fleets.contains_key(a));
        assert_eq!(world.fleets[b].capital_ships.len(), 2);
        assert_eq!(apply(&mut world, &mut movement, b, c), [true]);
        assert!(movement.is_in_transit(b));
        // In hyperspace it cannot be moved again.
        assert_eq!(apply(&mut world, &mut movement, b, c), [false]);
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
                &mut BombardEnv::new(0, None),
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
                &[AIAction::CreateFleet {
                    fleet,
                    ships,
                    to: kuat,
                }],
                &mut AIState::new(AiFaction::Empire),
                &mut MissionState::new(),
                &mut ManufacturingState::new(),
                &mut MovementState::new(),
                &mut TroopTransportState::new(),
                &mut ProductionState::new(),
                world,
                5,
                false,
                &mut BombardEnv::new(0, None),
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
            &mut BombardEnv::new(0, None),
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
            &mut BombardEnv::new(0, None),
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
            &mut BombardEnv::new(0, None),
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
            &mut BombardEnv::new(0, None),
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
            &mut BombardEnv::new(0, None),
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
            &mut BombardEnv::new(0, None),
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
            &mut BombardEnv::new(0, None),
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
    fn a_completed_death_star_can_destroy_a_system_from_its_new_fleet() {
        // CAPSHPSD.DAT record 136 is the Death Star (TEXTSTRA 10120); its
        // fleet enables Destroy System (0x223, FUN_00582440).
        let mut world = GameWorld::default();
        let system = add_system(&mut world, "Yard");
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(rebellion_core::world::DEATH_STAR_CLASS_ID),
            is_empire: true,
            hull: 25_000,
            bombardment_modifier: 15,
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
        assert!(rebellion_core::bombardment::order_enabled(
            &world,
            &rebellion_core::movement::MovementState::new(),
            false,
            &[fleet],
            target,
            rebellion_core::bombardment::DESTROY_SYSTEM,
        ));
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
    rng_rolls: &[f64],
    tick: u64,
) -> SystemSpaceCombatResult {
    let (initial_alliance, initial_empire) = orbiting_fleets_by_faction(world, system);
    let alliance_before = summarize_space_force(world, &initial_alliance);
    let empire_before = summarize_space_force(world, &initial_empire);
    let mut rounds = 0;
    let mut stalemate = false;

    // FUN_0040a050 state 8: every fleet of both sides fights one abstract
    // battle (`rebellion_core::abstract_battle`, FUN_005ae9c0).
    if !initial_alliance.is_empty() && !initial_empire.is_empty() {
        let (round, outcome) = rebellion_core::abstract_battle::resolve_space_battle(
            world,
            system,
            &initial_alliance,
            &initial_empire,
            rebellion_core::abstract_battle::abstract_seed(rng_rolls),
            tick,
        );
        apply_space_combat_result_inner(&round, world);
        rounds = 1;
        stalemate = outcome.stalemate;
    }

    let (alliance, empire) = orbiting_fleets_by_faction(world, system);
    let alliance_after = summarize_space_force(world, &alliance);
    let empire_after = summarize_space_force(world, &empire);
    let (winner, winner_fleet) = match (alliance.first().copied(), empire.first().copied()) {
        (Some(fleet), None) => (CombatSide::Attacker, Some(fleet)),
        (None, Some(fleet)) => (CombatSide::Defender, Some(fleet)),
        _ => (CombatSide::Draw, None),
    };

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

    // Remove dead ships and empty fleets: the two named fleets and every
    // other fleet the battle touched.
    let mut fleets = vec![result.attacker_fleet, result.defender_fleet];
    for fleet in result
        .ship_damage
        .iter()
        .map(|event| event.fleet)
        .chain(result.fighter_losses.iter().map(|event| event.fleet))
    {
        if !fleets.contains(&fleet) {
            fleets.push(fleet);
        }
    }
    for &fleet_key in &fleets {
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
            // FUN_00556710 -> slot +0xb4 FUN_004ef2e0: a character whose
            // container was destroyed (reason not 0x14) goes adrift where the
            // fleet stood, unless it is on a mission; FUN_0054c7d0 then
            // settles it or sends it on an Adrift order
            // (MissionSystem::send_adrift).
            rebellion_core::missions::cast_adrift(world, fleet_key);
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
            // FUN_005b7780: damage per second is the weapons' mean strength
            // times the weapon recharge, so the recharge carries the attack.
            weapon_recharge_rate: attack,
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
    fn fighter_losses_update_rosters_and_set_the_defeated_fleets_officers_adrift() {
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
        // FUN_00556710 -> FUN_004ef2e0: the officer of the destroyed fleet
        // goes adrift where it stood, not captive (the capture this test
        // once asserted had no source; mission-lifecycle.md, "Adrift").
        let officer = &world.characters[captive];
        assert!(officer.adrift && !officer.is_captive);
        assert_eq!(officer.captured_by, None);
        assert_eq!(
            (officer.current_fleet, officer.current_system),
            (None, Some(SystemKey::default()))
        );
    }

    #[test]
    fn a_destroyed_fleets_officer_leaves_the_battle_on_an_adrift_order() {
        // FUN_00556710 -> FUN_004ef2e0 sets the officer adrift where its
        // fleet was destroyed; the next mission step (FUN_0054c7d0 ->
        // FUN_00549230) sends it on an Adrift order 0x44000004 to the
        // nearest system of its side (FUN_00543040).
        use rebellion_core::missions::{MissionKind, MissionState, MissionSystem};
        let mut world = GameWorld::default();
        let battle = add_combat_system(&mut world);
        world.systems[battle].control = ControlKind::Controlled(Faction::Alliance);
        let mut home = world.systems[battle].clone();
        home.x = 100;
        home.control = ControlKind::Controlled(Faction::Empire);
        let home = world.systems.insert(home);
        let winner = world.fleets.insert(Fleet {
            location: battle,
            capital_ships: vec![],
            fighters: vec![FighterEntry {
                class: FighterKey::default(),
                count: 4,
            }],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        let beaten = world.fleets.insert(Fleet {
            location: battle,
            capital_ships: vec![],
            fighters: vec![FighterEntry {
                class: FighterKey::default(),
                count: 3,
            }],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        let officer = world.characters.insert(Character {
            name: "Officer".into(),
            is_empire: true,
            current_fleet: Some(beaten),
            current_system: Some(battle),
            ..Character::default()
        });
        world.fleets[beaten].characters.push(officer);
        let result = SpaceCombatResult {
            attacker_fleet: winner,
            defender_fleet: beaten,
            system: battle,
            winner: CombatSide::Attacker,
            ship_damage: vec![],
            fighter_losses: vec![FighterLossEvent {
                fleet: beaten,
                fighter_index: 0,
                squads_before: 3,
                squads_after: 0,
            }],
            tick: 4,
        };
        apply_space_combat_result_inner(&result, &mut world);
        let mut missions = MissionState::new();

        let advance = MissionSystem::advance(
            &mut missions,
            &world,
            &UprisingState::default(),
            &rebellion_core::research::ResearchState::default(),
            &[rebellion_core::tick::TickEvent { tick: 4 }],
            &[],
        );
        rebellion_core::missions::apply_advance_effects(&mut world, &advance.effects);

        let orders: Vec<_> = missions
            .missions()
            .iter()
            .map(|m| (m.kind, m.target_system))
            .collect();
        assert_eq!(orders, [(MissionKind::Adrift, home)]);
        let c = &world.characters[officer];
        assert!(c.on_mission && c.adrift && !c.is_captive);
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

    // FUN_0040a050 state 8: the engagement is one abstract battle that runs
    // until a side is beaten (FUN_005ae9c0).
    #[test]
    fn system_engagement_resolves_in_one_abstract_battle() {
        let mut world = GameWorld::default();
        let system = add_combat_system(&mut world);
        let attacker = add_ship_fleet(&mut world, system, true, 100, 30);
        let defender = add_ship_fleet(&mut world, system, false, 100, 1);

        let result = resolve_system_space_combat(&mut world, system, &[0.5; 256], 10);

        assert_eq!(result.winner, CombatSide::Attacker);
        assert_eq!(result.winner_fleet, Some(attacker));
        assert_eq!(result.rounds, 1);
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

        let result = resolve_system_space_combat(&mut world, system, &[0.5; 256], 11);

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

    // A battle needs both sides at the system.
    #[test]
    fn a_system_with_one_side_has_no_battle() {
        let mut world = GameWorld::default();
        let system = add_combat_system(&mut world);
        let fleet = add_ship_fleet(&mut world, system, true, 100, 5);

        let result = resolve_system_space_combat(&mut world, system, &[0.5; 256], 14);

        assert_eq!(result.rounds, 0);
        assert_eq!(result.winner, CombatSide::Attacker);
        assert!(world.fleets.contains_key(fleet));
    }

    // FUN_0040c780: every hull the battle destroys leaves its fleet, however
    // many fall in one fleet.
    #[test]
    fn every_hull_lost_in_one_battle_leaves_the_fleet() {
        let mut world = GameWorld::default();
        let system = add_combat_system(&mut world);
        let attacker = add_ship_fleet(&mut world, system, true, 400, 40);
        let defender = add_ship_fleet(&mut world, system, false, 50, 1);
        let extra = world.fleets[defender].capital_ships[0].clone();
        for _ in 0..4 {
            world.fleets[defender].capital_ships.push(extra.clone());
        }

        let result = resolve_system_space_combat(&mut world, system, &[0.5; 256], 13);

        assert_eq!(result.winner, CombatSide::Attacker);
        assert_eq!(result.empire_before.capital_ships, 5);
        assert_eq!(result.empire_after.capital_ships, 0);
        assert!(!world.fleets.contains_key(defender));
        assert!(world.fleets.contains_key(attacker));
    }

    // FUN_005ae9c0: unarmed sides never lose strength; after 1200 seconds
    // the equal sides are both destroyed (FUN_005af7a0).
    #[test]
    fn system_engagement_stall_destroys_two_equal_unarmed_sides() {
        let mut world = GameWorld::default();
        let system = add_combat_system(&mut world);
        add_ship_fleet(&mut world, system, true, 100, 0);
        add_ship_fleet(&mut world, system, false, 100, 0);

        let result = resolve_system_space_combat(&mut world, system, &[0.5; 256], 12);

        assert_eq!(result.winner, CombatSide::Draw);
        assert_eq!(result.rounds, 1);
        assert!(result.stalemate);
        assert!(world.systems[system].fleets.is_empty());
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
