//! Death Star movement tracking and planet destruction.
//!
//! The Death Star is an ordinary capital ship (CAPSHPSD record 136, TEXTSTRA
//! 10120, research order 0), built by a shipyard like any other; a completed
//! one marks its fleet (`Fleet::has_death_star`). The original has no
//! separate construction timer.
//!
//! 1. **Movement** — Death Star fleet movement reuses `MovementState`
//!    from `movement.rs`.  The Death Star module tracks the active orbital
//!    location for the `VictorySystem` and for the nearby-warning logic.
//!
//! 2. **Planet destruction** — `DeathStarSystem::fire()` checks preconditions:
//!    - Target system is not already destroyed (`!system.is_destroyed`).
//!    - Death Star fleet is present at that system.
//!    - Target is enemy-controlled (Empire Death Star → non-Empire system).
//!    The real superlaser fire path is unrecovered. FUN_005617b0 is the
//!    CharacterMgr SeatOfPower check; FUN_0055f650 stores that flag.
//!    On success emits `PlanetDestroyed`.  Caller sets `system.is_destroyed = true`.
//!
//! # Advance contract
//! `DeathStarSystem::advance()` never mutates `GameWorld`.
//! It returns `Vec<DeathStarEvent>` for the caller to apply.
//!
//! # Source
//! - `ghidra/notes/annotated-functions.md` § `FUN_005617b0`
//! - `ghidra/notes/economy-systems.md` § `SystemDeathStarNearbyNotif`
//! - `ghidra/notes/rust-implementation-guide.md` §3.3, §2.4
//! - `entity-system.md §4.2` — `alive_flag` inverted semantics for systems

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::ids::{FleetKey, SystemKey};
use crate::tick::TickEvent;
use crate::world::ControlKind;
use crate::world::GameWorld;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Threshold distance (in system-coordinate units) for the
/// `SystemDeathStarNearbyNotif` warning.
///
/// Best available approximation. `FUN_00512480` is a 51-byte notification
/// dispatcher without an embedded threshold constant. 300 coordinate units
/// derived from gameplay observation of the warning trigger distance.
pub const NEARBY_WARNING_RADIUS: u32 = 300;

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events produced by `DeathStarSystem::advance()` and `DeathStarSystem::fire()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeathStarEvent {
    /// The Death Star superlaser fired and destroyed `system`.
    ///
    /// Caller must set `world.systems[system].is_destroyed = true` and update
    /// `VictoryState::death_star_location = Some(system)`.
    PlanetDestroyed { system: SystemKey, tick: u64 },

    /// A Death Star fleet is within `NEARBY_WARNING_RADIUS` of `system`.
    ///
    /// Maps to `SystemDeathStarNearbyNotif` (`FUN_00512480`).
    /// Used to trigger Alliance intelligence messages.
    NearbyWarning { system: SystemKey, tick: u64 },
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Persistent Death Star simulation state.
///
/// Held in `main.rs` alongside other simulation states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathStarState {
    /// The fleet key of the active Death Star, if constructed and deployed.
    pub death_star_fleet: Option<FleetKey>,
    /// Whether the Death Star's shield generator (entity family 0x25) is active.
    /// While active, the Death Star takes no hull damage in battle; the shield
    /// does not block planet destruction (the superlaser fire path is unrecovered).
    /// From community disassembly: shield entity managed at FUN_0051c0d0
    /// through FUN_0051c270 (family 0x25..0x26 iterators).
    #[serde(default = "default_shield_active")]
    pub shield_generator_active: bool,
}

fn default_shield_active() -> bool {
    true
}

impl Default for DeathStarState {
    fn default() -> Self {
        DeathStarState {
            death_star_fleet: None,
            shield_generator_active: true,
        }
    }
}

impl DeathStarState {
    /// Destroy the Death Star's shield generator.
    /// After this, the Death Star takes hull damage in battle.
    pub fn destroy_shield(&mut self) {
        self.shield_generator_active = false;
    }
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

/// Stateless Death Star simulation system.
pub struct DeathStarSystem;

impl DeathStarSystem {
    /// Emit the nearby-warning events.
    ///
    /// Called each tick by the main loop.  Returns events for the caller to apply.
    ///
    /// If `death_star_fleet` is set and present in the world, it scans all
    /// Alliance systems within `NEARBY_WARNING_RADIUS` and emits
    /// `NearbyWarning`.
    #[expect(
        clippy::cast_sign_loss,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    pub fn advance(
        state: &mut DeathStarState,
        world: &GameWorld,
        tick_events: &[TickEvent],
    ) -> Vec<DeathStarEvent> {
        let mut events = Vec::new();

        let Some(last_tick_event) = tick_events.last() else {
            return events;
        };

        let last_tick = last_tick_event.tick;

        // Nearby-warning scan.
        if let Some(fleet_key) = state.death_star_fleet {
            if let Some(fleet) = world.fleets.get(fleet_key) {
                let ds_system = fleet.location;
                if let Some(ds_sys) = world.systems.get(ds_system) {
                    let ds_x = i32::from(ds_sys.x);
                    let ds_y = i32::from(ds_sys.y);

                    for (sys_key, sys) in &world.systems {
                        // Only warn about Alliance-controlled systems.
                        if sys.control != ControlKind::Controlled(Faction::Alliance) {
                            continue;
                        }
                        if sys_key == ds_system {
                            continue; // already at this system
                        }
                        let dx = i32::from(sys.x) - ds_x;
                        let dy = i32::from(sys.y) - ds_y;
                        let dist_sq = (dx * dx + dy * dy) as u64;
                        let radius_sq = u64::from(NEARBY_WARNING_RADIUS).pow(2);
                        if dist_sq <= radius_sq {
                            events.push(DeathStarEvent::NearbyWarning {
                                system: sys_key,
                                tick: last_tick,
                            });
                        }
                    }
                }
            }
        }

        events
    }

    /// Attempt to fire the Death Star superlaser at `target_system`.
    ///
    /// Mirrors `FUN_005617b0` + `FUN_0055f650` precondition checks:
    /// - The shield generator does not gate firing: `FUN_005617b0` never
    ///   references it. The shield protects the Death Star in battle instead.
    /// - Target must not already be destroyed (`!system.is_destroyed`).
    /// - An Empire Death Star fleet must be present at `target_system`.
    /// - Target must not be Empire-controlled (no self-destruction).
    ///
    /// Returns `Some(PlanetDestroyed)` on success; `None` if preconditions fail.
    /// The caller must apply `world.systems[target].is_destroyed = true` and
    /// update `VictoryState` after receiving this event.
    #[must_use]
    pub fn fire(
        _state: &DeathStarState,
        world: &GameWorld,
        target_system: SystemKey,
        tick: u64,
    ) -> Option<DeathStarEvent> {
        let sys = world.systems.get(target_system)?;

        // Guard: already destroyed.
        if sys.is_destroyed {
            return None;
        }

        // Guard: no self-destruction — Empire Death Star cannot fire on Empire systems.
        if sys.control.is_controlled_by(Faction::Empire) {
            return None;
        }

        // Guard: a Death Star fleet must be present at this system.
        let has_ds = sys
            .fleets
            .iter()
            .filter_map(|&fk| world.fleets.get(fk))
            .any(|f| !f.is_alliance && f.has_death_star);

        if !has_ds {
            return None;
        }

        Some(DeathStarEvent::PlanetDestroyed {
            system: target_system,
            tick,
        })
    }


}

// ---------------------------------------------------------------------------
// Destroyed system cleanup
// ---------------------------------------------------------------------------

use crate::blockade::BlockadeState;
use crate::effects::GameEffect;
use crate::manufacturing::ManufacturingState;

/// Whether `character` stands in `system` itself: a direct child of the
/// system (`FUN_00513050`'s walk does not descend), not aboard a fleet, in
/// an abode or on a mission, and existing.
#[must_use]
pub fn stands_in(character: &crate::world::Character, system: SystemKey) -> bool {
    character.current_system == Some(system)
        && character.current_fleet.is_none()
        && character.current_abode.is_none()
        && !character.on_mission
        && !character.is_killed
}

/// Carry out a Death Star's destruction of `system`.
///
/// The system's destruction (`FUN_0058ce70` -> slot `+0xac`, `FUN_005080d0`
/// -> `FUN_004f71d0`) sets its destroyed bit (`FUN_004f7560`), whose hook
/// (`FUN_00510950` -> `FUN_004fb520`) calls slot `+0xb4` on each existing
/// direct child:
/// - a ship, squadron, regiment or facility that is completed and not en
///   route is destroyed with the system's reason (`FUN_004f8710` ->
///   `FUN_004f7ed0`); so is a special force (`FUN_00503e20`);
/// - a fleet stays (`FUN_00524fb0` returns 1), with its ships and the
///   characters aboard, and one en route there keeps its trip;
/// - a character on no mission goes adrift (`FUN_004ef2e0`), as the
///   system's reason is not `0x14`.
///
/// Then each of `killed`, the minor characters the destruction hit
/// (`FUN_0058d990`, [`crate::bombardment`]), is killed, with one
/// `GameEffect::CharacterKilled` each in `effects` for the caller's
/// telemetry. A killed character stays in the arena so next-tick story
/// events can still resolve it.
///
/// port: a captive is not set adrift (where the original holds one is not
/// traced); the manufacturing queues and the blockade at the system are
/// cleared.
pub fn cleanup_destroyed_system(
    world: &mut GameWorld,
    system: SystemKey,
    killed: &[crate::ids::CharacterKey],
    manufacturing: &mut ManufacturingState,
    blockade: &mut BlockadeState,
    effects: &mut Vec<GameEffect>,
) {
    let Some(sys) = world.systems.get(system) else {
        return;
    };

    let troop_keys: Vec<_> = sys.ground_units.clone();
    let sf_keys: Vec<_> = sys.special_forces.clone();
    let def_keys: Vec<_> = sys.defense_facilities.clone();
    let mfg_keys: Vec<_> = sys.manufacturing_facilities.clone();
    let prod_keys: Vec<_> = sys.production_facilities.clone();

    for (_, c) in &mut world.characters {
        if stands_in(c, system) && !c.is_captive {
            c.adrift = true;
        }
    }
    for &ck in killed {
        if world.characters.get(ck).is_some_and(|c| !c.is_killed) {
            crate::missions::kill_character(world, ck);
            effects.push(GameEffect::CharacterKilled { character: ck });
        }
    }

    for &tk in &troop_keys {
        world.troops.remove(tk);
    }
    for &sk in &sf_keys {
        world.special_forces.remove(sk);
    }
    for &dk in &def_keys {
        world.defense_facilities.remove(dk);
    }
    for &mk in &mfg_keys {
        world.manufacturing_facilities.remove(mk);
    }
    for &pk in &prod_keys {
        world.production_facilities.remove(pk);
    }

    if let Some(sys) = world.systems.get_mut(system) {
        sys.ground_units.clear();
        sys.fighters.clear();
        sys.special_forces.clear();
        sys.defense_facilities.clear();
        sys.manufacturing_facilities.clear();
        sys.production_facilities.clear();
    }

    // Clear manufacturing queues at the destroyed system.
    manufacturing.clear_queue(system);

    // Lift any blockade at the destroyed system.
    blockade.clear_blockade(system);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{ExplorationStatus, Faction, SectorGroup};
    use crate::ids::DatId;
    use crate::tick::TickEvent;
    use crate::world::{Fleet, GameWorld, Sector, System};

    fn tick(n: u64) -> TickEvent {
        TickEvent { tick: n }
    }

    fn make_world() -> (GameWorld, SystemKey) {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(0x9200_0000),
            name: "Outer Rim".into(),
            group: SectorGroup::RimOuter,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let sys = world.systems.insert(System {
            dat_id: DatId::new(0x9000_0000),
            name: "Alderaan".into(),
            sector,
            x: 100,
            y: 100,
            exploration_status: ExplorationStatus::Explored,
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
            control: ControlKind::Controlled(Faction::Alliance),
        });
        (world, sys)
    }

    fn add_ds_fleet(world: &mut GameWorld, sys: SystemKey) -> FleetKey {
        let fk = world.fleets.insert(Fleet {
            location: sys,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: true,
        });
        world.systems.get_mut(sys).unwrap().fleets.push(fk);
        fk
    }

    // ── Planet destruction tests ─────────────────────────────────────────────

    /// A `DeathStarState` with the shield generator destroyed.
    fn state_shield_down() -> DeathStarState {
        DeathStarState {
            death_star_fleet: None,
            shield_generator_active: false,
        }
    }

    #[test]
    fn fire_succeeds_on_a_valid_target() {
        // No recovered source: FUN_005617b0 is the CharacterMgr SeatOfPower
        // check, not a fire precondition (ghidra/notes/FUN_005617b0.c).
        let (mut world, sys) = make_world();
        add_ds_fleet(&mut world, sys);
        let state = state_shield_down();

        let evt = DeathStarSystem::fire(&state, &world, sys, 42);
        assert!(
            matches!(evt, Some(DeathStarEvent::PlanetDestroyed { system, tick: 42 }) if system == sys),
            "expected PlanetDestroyed"
        );
    }

    #[test]
    fn an_active_shield_generator_does_not_stop_the_superlaser() {
        // FUN_005617b0 never references the shield generator; the shield
        // guards the Death Star in battle.
        let (mut world, sys) = make_world();
        add_ds_fleet(&mut world, sys);
        let state = DeathStarState::default(); // shield_generator_active = true

        assert!(matches!(
            DeathStarSystem::fire(&state, &world, sys, 1),
            Some(DeathStarEvent::PlanetDestroyed { system, .. }) if system == sys
        ));
    }

    #[test]
    fn fire_is_blocked_on_an_already_destroyed_system() {
        // No recovered source: FUN_005617b0 is the CharacterMgr SeatOfPower
        // check, not a fire precondition (ghidra/notes/FUN_005617b0.c).
        let (mut world, sys) = make_world();
        add_ds_fleet(&mut world, sys);
        let state = state_shield_down();
        world.systems.get_mut(sys).unwrap().is_destroyed = true;

        assert!(DeathStarSystem::fire(&state, &world, sys, 1).is_none());
    }

    #[test]
    fn a_destroyed_planet_loses_its_squadrons_with_its_regiments() {
        // port: cleanup_destroyed_system removes every object at the system;
        // the planet's squadrons (FUN_005039d0) stand on it.
        let (mut world, sys) = make_world();
        let tie = world
            .fighter_classes
            .insert(crate::world::FighterClass::default());
        crate::squadrons::add(&mut world.systems[sys].fighters, tie, 2);
        let mut effects = Vec::new();
        cleanup_destroyed_system(
            &mut world,
            sys,
            &[],
            &mut ManufacturingState::new(),
            &mut BlockadeState::new(),
            &mut effects,
        );
        assert!(world.systems[sys].fighters.is_empty());
    }

    #[test]
    fn a_destroyed_systems_fleets_stay_and_the_characters_standing_there_go_adrift() {
        // The system's destroyed hook (FUN_00510950 -> FUN_004fb520) calls
        // slot +0xb4 on each direct child: a fleet's (FUN_00524fb0) does
        // nothing, a regiment's (FUN_004f8710 -> FUN_004f7ed0) destroys it,
        // a character's (FUN_004ef2e0) sets it adrift unless on a mission.
        // FUN_0058d990 then destroys the minor characters it hit.
        let (mut world, sys) = make_world();
        let fleet = add_ds_fleet(&mut world, sys);
        let mut person = |edit: &dyn Fn(&mut crate::world::Character)| {
            let mut c = crate::world::Character {
                current_system: Some(sys),
                ..crate::world::Character::default()
            };
            edit(&mut c);
            world.characters.insert(c)
        };
        let standing = person(&|_| {});
        let hit = person(&|_| {});
        let aboard = person(&|_| {});
        let on_mission = person(&|c| c.on_mission = true);
        let captive = person(&|c| c.is_captive = true);
        world.characters[aboard].current_fleet = Some(fleet);
        world.fleets[fleet].characters.push(aboard);
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance: true,
            regiment_strength: 1,
        });
        world.systems[sys].ground_units.push(troop);

        let mut effects = Vec::new();
        cleanup_destroyed_system(
            &mut world,
            sys,
            &[hit],
            &mut ManufacturingState::new(),
            &mut BlockadeState::new(),
            &mut effects,
        );

        assert_eq!(world.systems[sys].fleets, [fleet]);
        assert_eq!(world.fleets[fleet].characters, [aboard]);
        assert!(!world.characters[aboard].adrift && !world.characters[aboard].is_killed);
        assert!(world.characters[standing].adrift);
        assert!(!world.characters[standing].is_killed);
        assert!(!world.characters[on_mission].adrift);
        assert!(!world.characters[captive].adrift);
        assert!(world.characters[hit].is_killed);
        assert!(matches!(
            effects.as_slice(),
            [GameEffect::CharacterKilled { character }] if *character == hit
        ));
        assert!(!world.troops.contains_key(troop));
        assert!(world.systems[sys].ground_units.is_empty());
    }

    #[test]
    fn fire_is_blocked_without_a_death_star_fleet() {
        // No recovered source: FUN_005617b0 is the CharacterMgr SeatOfPower
        // check, not a fire precondition (ghidra/notes/FUN_005617b0.c).
        let (mut world, sys) = make_world();
        let state = state_shield_down();
        let fk = world.fleets.insert(Fleet {
            location: sys,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems.get_mut(sys).unwrap().fleets.push(fk);

        assert!(DeathStarSystem::fire(&state, &world, sys, 1).is_none());
    }

    #[test]
    fn fire_is_blocked_on_empire_controlled_systems() {
        // No recovered source: FUN_005617b0 is the CharacterMgr SeatOfPower
        // check, not a fire precondition (ghidra/notes/FUN_005617b0.c).
        let (mut world, sys) = make_world();
        let state = state_shield_down();
        world.systems.get_mut(sys).unwrap().control = ControlKind::Controlled(Faction::Empire);
        add_ds_fleet(&mut world, sys);

        assert!(
            DeathStarSystem::fire(&state, &world, sys, 1).is_none(),
            "Death Star must not fire on Empire-controlled systems"
        );
    }

    // ── Nearby-warning tests ─────────────────────────────────────────────────

    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Fixture sizes and coordinates are deliberately small and fit their encoded fields."
    )]
    fn nearby_warning_is_emitted_for_close_alliance_system() {
        // Source: FUN_00512480 sends SystemDeathStarNearbyNotif.
        let (mut world, ds_sys) = make_world();
        // Add a second Alliance system within radius.
        let sector = world.systems[ds_sys].sector;
        let nearby_sys = world.systems.insert(System {
            dat_id: DatId::new(0x9000_0001),
            name: "Tatooine".into(),
            sector,
            x: 100 + NEARBY_WARNING_RADIUS as u16 / 2,
            y: 100,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.6,
            popularity_empire: 0.4,
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
            control: ControlKind::Controlled(Faction::Alliance),
        });
        let _ = nearby_sys;

        let fleet_key = add_ds_fleet(&mut world, ds_sys);
        let mut state = DeathStarState {
            death_star_fleet: Some(fleet_key),
            shield_generator_active: false,
        };

        let events = DeathStarSystem::advance(&mut state, &world, &[tick(1)]);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, DeathStarEvent::NearbyWarning { .. })),
            "expected NearbyWarning for close Alliance system"
        );
    }

    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Fixture sizes and coordinates are deliberately small and fit their encoded fields."
    )]
    fn no_nearby_warning_for_a_distant_system() {
        // Source: FUN_00512480 sends SystemDeathStarNearbyNotif.
        let (mut world, ds_sys) = make_world();
        let sector = world.systems[ds_sys].sector;
        // Place a system far outside the warning radius.
        world.systems.insert(System {
            dat_id: DatId::new(0x9000_0001),
            name: "Distant World".into(),
            sector,
            x: 100 + NEARBY_WARNING_RADIUS as u16 * 2,
            y: 100,
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
            control: ControlKind::Controlled(Faction::Alliance),
        });

        let fleet_key = add_ds_fleet(&mut world, ds_sys);
        let mut state = DeathStarState {
            death_star_fleet: Some(fleet_key),
            shield_generator_active: false,
        };

        let events = DeathStarSystem::advance(&mut state, &world, &[tick(1)]);
        let warning_count = events
            .iter()
            .filter(|e| matches!(e, DeathStarEvent::NearbyWarning { .. }))
            .count();
        assert_eq!(
            warning_count, 0,
            "distant system should not trigger NearbyWarning"
        );
    }
}
