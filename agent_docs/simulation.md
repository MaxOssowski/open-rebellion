---
title: "Simulation Systems"
description: "Seventeen stateless advance-pattern modules implementing the game simulation"
category: "agent-docs"
created: 2026-03-14
updated: 2026-03-16
tags: [simulation, advance-pattern, tick, deterministic]
---

# Simulation Systems

Seventeen modules in `rebellion-core/src/` implement the game simulation. All follow the same stateless advance pattern.

## The Advance Contract

Every simulation system is a pure function:
```rust
System::advance(&mut State, &GameWorld, &[TickEvent], ...) -> Vec<ResultEvent>
```

Rules:
- **Never mutates GameWorld** — returns result events for the caller to apply
- **Caller provides RNG** — random rolls are passed as `&[f64]` or `&[f32]` slices, so tests can control outcomes exactly
- **Deterministic** — same inputs always produce same outputs
- **No IO** — rebellion-core has zero IO deps; stays headless-testable

The `PerceptionIntegrator` (rebellion-data `integrator.rs`) applies effects to `GameWorld` and emits telemetry. `simulation.rs` orchestrates the tick; `integrator.rs` owns all mutation and telemetry.

## System Inventory

| # | System | File | Signature | Key Gotcha |
|---|--------|------|-----------|------------|
| 1 | **Tick** | `tick.rs` | `clock.advance(dt) -> Vec<TickEvent>` | `TICK_DURATION = 1.0s` at Normal speed |
| 2 | **Manufacturing** | `manufacturing.rs`, `production_cycle.rs` | `advance_yards(production, world, manufacturing, blocked, ticks)` | A queue counts build points, one per yard cycle; a cycle start takes one refined material. Yards of a system in uprising or blockaded do not run |
| 3 | **Missions** | `missions.rs` | `advance(state, world, ticks, rolls)` | 9 kinds, MSTB tables, `target_character` for assassination/abduction/rescue |
| 4 | **Events** | `events.rs` | `advance(state, world, ticks, rolls)` | `EventFired` chaining within same call |
| 5 | **AI** | `ai.rs` | `pass(state, world, mfg, missions, movement, economy, troop_transport, tick, ..)` | Per-fleet targeting, deconfliction, battle penalty. `passes_per_day(speed)` passes a day, each applied before the next, one side then the other; at most one order a pass, 10 a day; fourteen modules by priority, then round robin; each pass first rebuilds the side's records (`ai_records::Galaxy`); a module steps until it completes (modules 1, 2, 3, 5, 6, 7 and `0xb`, `ai_module1.rs` to `ai_module7.rs` and `ai_module_b.rs`, run their cycles within one pass); module `0xd` first chooses its posture and names the sectors it plans for (`ai_strategy.rs`), and module 8 keeps a sector planner per planned sector (`ai_module8.rs`), which holds ships and fleets for its plan and steps them (`ai_planner.rs`); the plan object's step (`ai_plan.rs`) chooses its base, unloads its fleets' regiments, boards the base's squadrons, sets down their people, gives one fleet a cycle a role (attack, bombardment) and target or sends it to another held fleet or home, and settles the roles at their targets, a bombarder ordering its bombardment (`AIAction::Bombard`, resolved by `bombardment.rs`); the other kinds' planners are still the port's fleet heuristics, which leave held fleets alone. Before the day's passes the driver numbers new capital ships (`GameWorld::number_ships`), the key of the AI's per-ship records. |
| 6 | **Movement** | `movement.rs` | `advance(state, ticks) -> Vec<ArrivalEvent>` | Caller must update `fleet.location` + `system.fleets` |
| 7 | **Fog** | `fog.rs` | `advance(state, world, movement)` | Fleet presence + sensor radius (detection * 15.0); monotonic |
| 8 | **Combat** | `combat.rs`, `assault.rs` | `resolve_space()`; `assault::resolve` (Planetary Assault only) | Phase gate: `ACTIVE && !PHASES_ENABLED` (inverted) |
| 9 | **Blockade** | `blockade.rs` | `advance(state, world, ticks)` | Hostile fleet + no defender = blockade; timer `0x386` shifts the blockader's support every GNPRTB 7694 days (`FUN_0050cb20`) |
| 10 | **Uprising** | `uprising.rs` | `advance(state, world, economy, missions, ticks, rolls)` | Needs the economy summary; incidents, disasters, the resource incident `0x390` and the revolt's support timer `0x384` (`FUN_0050caa0`) run on recovered timers |
| 10b | **Informants** | `informant.rs` | `advance(state, world, ticks, rolls)` | Each held system's timer `0x38e` (30-1000 days): with `100 - support` percent, INFORMTB tells the other side about the system (`ghidra/notes/informant-incident.md`) |
| 10c | **Healing** | `healing.rs` | `advance(state, world, ticks)` | Each injured character among its own side heals 1 point a day (2 at Force 80+), timer `0x388` (`ghidra/notes/character-injury.md`) |
| 5b | **Dagobah** | `dagobah.rs` | `advance(state, world, missions, ticks, rolls)` then `dispatch` | Timer `0x392` (300-400 days) requires Dagobah; once Luke is free, the mission manager sends him to the Dagobah abode for 100 days (`ghidra/notes/dagobah.md`) |
| 5c | **Final Battle** | `final_battle.rs` | `FinalBattleSystem::advance(state, world, missions, ticks, rolls)` then `perform` each step | FinalBattleReady for a captured Luke who knows his heritage at Force 60+; Move orders take Vader to Luke and both to the Emperor; the battle when the three meet (`ghidra/notes/final-battle.md`) |
| 5d | **Jabba** | `jabba.rs` | `JabbaSystem::advance(state, world, missions, ticks, rolls)` then `perform` each step | BountyHuntersActive arms timer `0x393` (300-600 days); when it fires, the Bounty Hunters unit leaves Jabba's Palace on the Bounty mission against Han, and a captured Han is taken to the palace; while Han is held there, Luke, Leia and Chewbacca go on Palace missions to free him; once Luke, Leia and Han are held, Vader's Pickup mission takes the palace's prisoners to where he came from (`ghidra/notes/jabba-palace.md`) |
| 5e | **SeatOfPower** | `seat_of_power.rs` | `SeatOfPowerSystem::advance(state, world)` | The Emperor free at an Empire-held Coruscant; a change is notification `0x230`, and the app posts Emperor Arrives at Coruscant to the Empire when it is set (`ghidra/notes/story-message-classes.md`, row 16) |
| 10d | **Force discovery** | `force_discovery.rs` | `advance(state, world, ticks, rolls)` | Each trainer with Force 80+ (Vader; Luke later), free, makes the latent characters of its side beside it Force aware, daily, timer `0x389` (`ghidra/notes/force-awareness.md`) |
| 11 | **Death Star** | `death_star.rs` | `advance(state, world, ticks)` | Self-clears construction on completion |
| 12 | **Research** | `research.rs` | `advance_tally(state, world, blocked, ticks, rolls)` after the production cycle; research missions resolve in step 5 | `MissionEffect::ResearchPoints` -> `ResearchState::add_points` (2026-10-14); timer `0x381` adds the side's active yards to each tree every 30-60 days (2026-10-17) |
| 14 | **Victory** | `victory.rs` | `advance_conditions_word(state, world, conditions)`, then `check(state, world, ticks)` | The victory conditions word (`FUN_005180e0`, notification `0x128` on a change: Coruscant Captured, Alliance Headquarters Destroyed); then `check` decides the game by the word's win bits (F-005) |
| 16 | **Economy** | `economy.rs` | `advance(state, world, ticks, difficulty)` | Full 18-function strategic state rebuild (FUN_00508250, which the original runs once at setup, not per tick: `ghidra/notes/timer-scheduler.md`). Runs BEFORE manufacturing (position 0). Resource caps, smuggling percent (FUN_0050b230 → `+0x6c`; support moves only through FUN_0050c9f0, `ghidra/notes/economy-cadence.md`), collection rate, KDY modifier, side resolution (FUN_0050b610: the regiments' side, else the side whose support reaches GNPRTB[7760]; `ghidra/notes/system-control.md`), garrison, troop/fleet summary, incident generation (state-transition-driven, 4 flags). 17 GNPRTB indices. 30 tests. |
| 17 | **Repair** | `repair.rs` | `advance(state, world, ticks)` | Damaged ships at shipyards auto-repair using class `damage_control`. `RepairCheckPerformed` marks the start of a continuous repair episode; `ShipRepaired` records each hull step. Active episodes persist in save v13. |
| 18 | **Production cycle** | `production_cycle.rs` | `advance(state, world, mfg, blocked, ticks, rolls)` | Mines add raw material, refineries turn raw into refined; upkeep allocation lengthens a cycle from 5 to 10 days. Owns the per-side stockpiles the monitors read (`ghidra/notes/side-resource-monitors.md`). Runs after Jedi so earlier rolls keep their positions. A cycle takes its system's ProductionModifier. |
| 18b | **Autoscrap** | `autoscrap.rs` | `advance(state, world, transport, mfg, blocked, ticks, rolls)` | The maintenance shortfall: while a side's upkeep exceeds its capacity, timer `0x382` scraps one random maintained object every GNPRTB 7168 (10) days with reason `0x15` and a half refund (`FUN_00530350`, `FUN_004f8850`). |

Per-system detail docs: `agent_docs/systems/{combat,blockade,uprising,death-star,research,jedi,victory,betrayal,story-events}.md`

## Mission Types and Effects

| Kind | Effect on Success |
|------|-------------------|
| Diplomacy | `PopularityShifted` |
| Recruitment | `CharacterRecruited` |
| Sabotage | `FacilitySabotaged` → remove facility from system + arena |
| Assassination | `CharacterKilled` → remove from fleets + arena |
| Espionage | `SystemIntelligenceGathered` → set explored |
| Rescue | `CharacterRescued` → restore faction, clear captivity |
| Abduction | `CharacterCaptured` → set `is_captive`/`captured_by`/`capture_tick`, flip faction, remove from fleets |
| InciteUprising | `UprisingStarted` → shift popularity |
| Autoscrap | (no world effect) |

Additional per-tick effects (not mission-driven):
- `CharacterEscaped` (from `advance_escapes()`) → restore home faction, clear captivity; the escapee's trip follows as `MemberMoved` effects
- `CharacterBusy` → set `on_mission = true`
- `CharacterAvailable` → clear `on_mission`, `on_hidden_mission`
- `DecoyTriggered` → log message, no world mutation

Probability: MSTB table lookup (a step function, `FUN_00595090`) with quadratic fallback. Combined: `total_prob = agent_prob · (1 − foil_prob)`.

## Integration Order (simulation.rs → integrator.rs)

```
run_simulation_tick():
  integrator = PerceptionIntegrator::new(tick, wall_ms)
  0. economy_events  = EconomySystem::advance()       → integrator.apply_economy_events()
  1. completions     = production_cycle::advance_yards() → integrator.apply_build_completions()
  2. arrivals        = MovementSystem::advance()       → integrator.apply_arrivals()
  3. combat triggers → CombatSystem::resolve_space()   → integrator.apply_space_combat()
  3b. occupation; carried regiments stay aboard until ordered (no ground battle outside Planetary Assault)
  4. reveals         = FogSystem::advance()             → integrator.emit_fog_reveals()
  5. results         = MissionSystem::advance()         → integrator.apply_mission_result()
  5b. escapes        = MissionSystem::advance_escapes() → integrator.apply_escape_effects()
  6. fired_events    = EventSystem::advance()           → integrator.apply_fired_events()
  7. ai_actions      = AISystem::advance()              → integrator.apply_ai_actions()
  8. blockade        = BlockadeSystem::advance()        → integrator.apply_blockade_events()
  9. uprising        = UprisingSystem::advance()        → integrator.apply_uprising_events()
  11. ds_events      = DeathStarSystem::advance()       → integrator.apply_death_star_events()
  12. research       = (none: research missions add points in step 5, apply_mission_result)
  14. victory        = VictorySystem::check()           → integrator.apply_victory()
  15. snapshot       (every 250 ticks)                  → integrator.emit_campaign_snapshot()
  return integrator.finish()
```

Order matters: economy runs before manufacturing (production modifiers); combat needs fleet positions from movement; AI reads mission and manufacturing state that earlier systems may have modified.

## Shared Simulation Tick (v0.8.0) + PerceptionIntegrator (v0.19.0)

The tick orchestration lives in `rebellion-data/src/simulation.rs` (~449 LOC):

```rust
pub fn run_simulation_tick(world, states, tick_events, rolls, wall_ms, config) -> Vec<GameEventRecord>
```

All world mutation and telemetry emission lives in `rebellion-data/src/integrator.rs` (~1,185 LOC):

```rust
pub struct PerceptionIntegrator { events: Vec<GameEventRecord>, tick: u64, wall_ms: u64 }
impl PerceptionIntegrator {
    pub fn apply_economy_events(&mut self, world, events) // ... 17 methods total
    pub fn finish(self) -> Vec<GameEventRecord>
}
```

`rebellion-playtest` calls `run_simulation_tick`; the interactive
`rebellion-app` still duplicates this sequence. That divergence is tracked as
F-012 and must close before native/WASM replay equivalence can pass.
`SimulationStates` bundles all 15 state types plus combat cooldowns and economy
state. `GameEventRecord` (in `rebellion-core/src/game_events.rs`) is structured
telemetry with no IO. The `config` parameter (`&GameConfig` from `tuning.rs`)
controls tunable AI, movement, and production behavior; the playtest binary
accepts `--config <path>` for autoresearch.

**Architecture**: simulation.rs is a thin orchestrator that calls `advance()` on each system and delegates mutation + telemetry to integrator methods. No `events.push()` calls remain in simulation.rs — `integrator.finish()` is the sole return path.

## State Ownership

All simulation states are created in `main.rs` and included in `SaveState` (`rebellion-data/src/save.rs`, version 5) for serialization.

| State | Mutated By |
|-------|------------|
| `GameClock` | `clock.advance(dt)` |
| `ManufacturingState` | `production_cycle::advance_yards`, AI |
| `MissionState` | `MissionSystem::advance`, AI |
| `EventState` | `EventSystem::advance` |
| `AIState` | `AISystem::advance`, mission callbacks |
| `MovementState` | `MovementSystem::advance`, AI fleet orders |
| `FogState` | `FogSystem::advance` |
| `BlockadeState` | `BlockadeSystem::advance` |
| `UprisingState` | `UprisingSystem::advance`, `end_if_garrisoned()` |
| `DeathStarState` | `DeathStarSystem::advance`, `fire()` |
| `ResearchState` | `ResearchState::add_points` from a research mission's `ResearchPoints` effect |
| `VictoryState` | Caller sets `resolved = true` after outcome |
| `GameWorld` | Effect application only (never inside advance()) |

## Adding a New Simulation System

1. Create `crates/rebellion-core/src/{name}.rs`
2. Define `{Name}State` (derive `Serialize, Deserialize`) and `{Name}System` (stateless namespace)
3. Implement `{Name}System::advance(&mut State, ..., &[TickEvent]) -> Vec<ResultEvent>`
4. Accept RNG as caller-provided slices, not internal `rand`
5. Add `pub mod {name};` to `crates/rebellion-core/src/lib.rs`
6. Add `advance()` call to `run_simulation_tick()` in `crates/rebellion-data/src/simulation.rs`
7. Add `apply_{name}_events()` method to `PerceptionIntegrator` in `crates/rebellion-data/src/integrator.rs` — this method applies world mutations AND emits telemetry
8. Add the state to `SimulationStates` in `simulation.rs` and `SaveState` in `save.rs`, bump `SAVE_VERSION`
9. Add unit tests — the advance() pattern makes this trivial (no IO mocks needed)
10. Add doc file at `agent_docs/systems/{name}.md`
11. If the interactive game also needs the system, add `advance()` + mutation to `main.rs` (mirrors the integrator pattern)

## Adding a New Mission Type

1. Add variant to `MissionKind` enum in `missions.rs`
2. Add match arm in `MissionSystem::compute_effects()` — return `Vec<MissionEffect>` for success
3. If new effects are needed, add variant to `MissionEffect` enum
4. Add match arm in `integrator.rs` `apply_mission_effects_inner()` to mutate `GameWorld`
5. Telemetry is automatic via `apply_mission_result()` on PerceptionIntegrator
6. Add to panel UI: `rebellion-render/src/panels/missions.rs`
7. If the mission needs a probability table, add a `*MSTB.DAT` parser and register the table name in `world.mission_tables`
8. Add `target_character` if the mission targets a specific character (assassination, abduction, rescue pattern)
9. Test: create an `ActiveMission`, advance to completion, verify effects

## Adding a New Event Type

1. Add variant to `EventCondition` or `EventAction` in `events.rs`
2. For conditions: add evaluation logic in `EventSystem::evaluate_condition()`
3. For actions: add application logic in `integrator.rs` `apply_event_actions_to_world_inner()`
4. Events are data-driven — defined via `GameEvent` structs with `conditions` and `actions` vectors
5. Event chaining: use `EventCondition::EventFired(id)` to sequence events
