//! Fog of war and intel system.
//!
//! Each faction starts with knowledge of only the systems they physically
//! occupy (fleet or character present). As fleets move and characters are
//! dispatched, new systems are revealed and added to that faction's
//! `FogState`.
//!
//! # Visibility rules (simplified Phase 0)
//!
//! A system is **visible** to a faction if:
//! 1. The faction has at least one orbiting fleet at that system, OR
//! 2. The faction has at least one character assigned to that system via a fleet.
//!
//! Once revealed a system stays revealed. port: this set only records where
//! a side's fleets have stood; what a side sees is its copy of each system
//! (`object_views`, `ghidra/notes/fog-of-war.md`). The original has no
//! sensor range and no reveal ahead of a fleet's arrival.
//!
//! # Usage
//!
//! ```ignore
//! use rebellion_core::fog::{FogState, FogSystem};
//! use rebellion_core::dat::Faction;
//!
//! let mut fog = FogState::new(Faction::Alliance);
//! // Seed with all systems containing Alliance fleets at game start:
//! FogSystem::seed(&mut fog, &world);
//!
//! // Each tick after fleet movement resolves:
//! let reveals = FogSystem::advance(&mut fog, &world, &movement_state);
//! ```

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::ids::SystemKey;
use crate::movement::MovementState;
use crate::world::GameWorld;

// ---------------------------------------------------------------------------
// FogState
// ---------------------------------------------------------------------------

/// Per-faction visibility state: the set of systems currently known to
/// this faction.
///
/// Systems are added monotonically — once revealed they stay revealed.
/// A fleet arriving at a new system via `MovementState` will reveal it
/// on the next `FogSystem::advance` call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FogState {
    /// Which faction owns this fog view.
    pub faction: Faction,
    /// All systems currently visible to this faction.
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_set",
        deserialize_with = "crate::serde_ordered::deserialize_hash_set"
    )]
    pub visible: HashSet<SystemKey>,
}

impl FogState {
    #[must_use]
    pub fn new(faction: Faction) -> Self {
        FogState {
            faction,
            visible: HashSet::new(),
        }
    }

    /// Returns true if `system` is currently visible to this faction.
    #[inline]
    #[must_use]
    pub fn is_visible(&self, system: SystemKey) -> bool {
        self.visible.contains(&system)
    }

    /// Mark a system as revealed (idempotent).
    pub fn reveal(&mut self, system: SystemKey) {
        self.visible.insert(system);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.visible.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.visible.is_empty()
    }
}

// ---------------------------------------------------------------------------
// RevealEvent
// ---------------------------------------------------------------------------

/// Emitted when a system becomes newly visible to a faction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealEvent {
    /// The faction that gained visibility.
    pub faction: Faction,
    /// The system that was revealed.
    pub system: SystemKey,
}

// ---------------------------------------------------------------------------
// FogSystem
// ---------------------------------------------------------------------------

/// Stateless system that updates fog-of-war visibility.
pub struct FogSystem;

impl FogSystem {
    /// Seed initial visibility from the current world state.
    ///
    /// Call once at game start (or after loading a save) to populate
    /// `FogState` with all systems the faction already occupies.
    /// Idempotent — safe to call multiple times.
    pub fn seed(fog: &mut FogState, world: &GameWorld) {
        let is_alliance = fog.faction == Faction::Alliance;
        for (_fleet_key, fleet) in &world.fleets {
            if fleet.is_alliance == is_alliance {
                fog.reveal(fleet.location);
            }
        }
    }

    /// Advance fog-of-war after fleet movement has resolved for this tick.
    ///
    /// Scans orbiting fleets and approaching in-transit destinations belonging
    /// to this faction, revealing any system not yet visible.
    ///
    /// Returns one `RevealEvent` per newly-revealed system.
    pub fn advance(
        fog: &mut FogState,
        world: &GameWorld,
        movement_state: &MovementState,
    ) -> Vec<RevealEvent> {
        // FUN_0050ac80: a side sees a system only through one of its usable
        // fleets in it (or by holding it). A fleet in hyperspace sees
        // neither end of its trip, and no ship sees past its own system
        // (ghidra/notes/fog-of-war.md: the original has no sensor reach and
        // no half-way reveal).
        let is_alliance = fog.faction == Faction::Alliance;
        let mut events = Vec::new();

        for (fleet_key, fleet) in &world.fleets {
            if fleet.is_alliance != is_alliance {
                continue;
            }
            if movement_state.is_in_transit(fleet_key) {
                continue;
            }
            if !fog.is_visible(fleet.location) {
                fog.reveal(fleet.location);
                events.push(RevealEvent {
                    faction: fog.faction,
                    system: fleet.location,
                });
            }
        }

        events
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{FleetKey, SystemKey};
    use crate::movement::MovementState;
    use crate::world::ControlKind;
    use crate::world::{Fleet, GameWorld};

    // Build a minimal GameWorld with N systems and M alliance fleets.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Fixture sizes and coordinates are deliberately small and fit their encoded fields."
    )]
    fn make_world_with_fleets(
        n_systems: usize,
        fleet_locations: &[usize], // indices into systems vec
        is_alliance: bool,
    ) -> (GameWorld, Vec<SystemKey>, Vec<FleetKey>) {
        let mut world = GameWorld::default();

        // Create a dummy sector key for systems.
        let sector = world.sectors.insert(crate::world::Sector {
            dat_id: crate::ids::DatId(0),
            name: "Test Sector".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });

        let sys_keys: Vec<SystemKey> = (0..n_systems)
            .map(|i| {
                world.systems.insert(crate::world::System {
                    last_arrival: None,
                    dat_id: crate::ids::DatId(i as u32),
                    name: format!("System {i}"),
                    sector,
                    x: i as u16 * 10,
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
                    manufacturing_facilities: vec![],
                    production_facilities: vec![],
                    fighters: Vec::new(),
                    is_headquarters: false,
                    is_destroyed: false,
                    control: ControlKind::Uncontrolled,
                })
            })
            .collect();

        let fleet_keys: Vec<FleetKey> = fleet_locations
            .iter()
            .map(|&idx| {
                world.fleets.insert(Fleet {
                    location: sys_keys[idx],
                    capital_ships: vec![],
                    fighters: vec![],
                    characters: vec![],
                    is_alliance,
                    has_death_star: false,
                    special_forces: Vec::new(),
                })
            })
            .collect();

        (world, sys_keys, fleet_keys)
    }

    #[test]
    fn new_fog_is_empty() {
        let fog = FogState::new(Faction::Alliance);
        assert!(fog.is_empty());
    }

    #[test]
    fn reveal_is_idempotent() {
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let key = sys_sm.insert(());
        let mut fog = FogState::new(Faction::Alliance);
        fog.reveal(key);
        fog.reveal(key);
        assert_eq!(fog.len(), 1);
    }

    #[test]
    fn a_fog_counts_each_revealed_system_once() {
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let keys = [sys_sm.insert(()), sys_sm.insert(())];
        let mut fog = FogState::new(Faction::Alliance);
        fog.reveal(keys[0]);
        fog.reveal(keys[1]);
        fog.reveal(keys[0]);
        assert_eq!(fog.len(), 2);
        assert!(!fog.is_empty());
    }

    #[test]
    fn seed_reveals_faction_fleet_locations() {
        let (world, sys_keys, _) = make_world_with_fleets(3, &[0, 2], true);
        let mut fog = FogState::new(Faction::Alliance);
        FogSystem::seed(&mut fog, &world);
        assert!(fog.is_visible(sys_keys[0]));
        assert!(!fog.is_visible(sys_keys[1]));
        assert!(fog.is_visible(sys_keys[2]));
    }

    #[test]
    fn seed_ignores_enemy_fleets() {
        // Empire fleets should not reveal for Alliance fog.
        let (world, sys_keys, _) = make_world_with_fleets(2, &[0], false); // empire fleet at sys 0
        let mut fog = FogState::new(Faction::Alliance);
        FogSystem::seed(&mut fog, &world);
        assert!(!fog.is_visible(sys_keys[0]));
    }

    #[test]
    fn advance_reveals_new_fleet_location() {
        let (world, sys_keys, _) = make_world_with_fleets(2, &[0, 1], true);
        let mut fog = FogState::new(Faction::Alliance);
        // Pre-reveal sys 0 only.
        fog.reveal(sys_keys[0]);
        let ms = MovementState::new();
        let events = FogSystem::advance(&mut fog, &world, &ms);
        // sys 1 should now be revealed.
        assert!(fog.is_visible(sys_keys[1]));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].system, sys_keys[1]);
        assert_eq!(events[0].faction, Faction::Alliance);
    }

    #[test]
    fn advance_no_new_systems_no_events() {
        let (world, sys_keys, _) = make_world_with_fleets(1, &[0], true);
        let mut fog = FogState::new(Faction::Alliance);
        fog.reveal(sys_keys[0]);
        let ms = MovementState::new();
        let events = FogSystem::advance(&mut fog, &world, &ms);
        assert!(events.is_empty());
    }

    #[test]
    fn a_moving_fleet_reveals_no_system_before_it_arrives() {
        // FUN_0050ac80 counts only usable fleets (+0x50 bit 0), and a fleet
        // in hyperspace is not usable (ghidra/notes/fog-of-war.md).
        let (world, sys_keys, fleet_keys) = make_world_with_fleets(2, &[0], true);
        let mut fog = FogState::new(Faction::Alliance);
        fog.reveal(sys_keys[0]);

        let mut ms = MovementState::new();
        ms.order(fleet_keys[0], sys_keys[0], sys_keys[1], 10);
        for elapsed in [4, 5, 9] {
            ms.orders_mut()
                .get_mut(&fleet_keys[0])
                .unwrap()
                .ticks_elapsed = elapsed;
            let events = FogSystem::advance(&mut fog, &world, &ms);
            assert!(!fog.is_visible(sys_keys[1]), "revealed at {elapsed} of 10");
            assert!(events.is_empty());
        }
    }

    #[test]
    fn in_transit_fleet_does_not_reveal_or_scan_from_stale_origin() {
        let (world, sys_keys, fleet_keys) = make_world_with_fleets(2, &[0], true);
        let mut fog = FogState::new(Faction::Alliance);
        let mut movement = MovementState::new();
        assert!(movement.order(fleet_keys[0], sys_keys[0], sys_keys[1], 10,));

        let events = FogSystem::advance(&mut fog, &world, &movement);

        assert!(events.is_empty());
        assert!(!fog.is_visible(sys_keys[0]));
        assert!(!fog.is_visible(sys_keys[1]));
    }

    #[test]
    fn advance_ignores_enemy_transit_orders() {
        let (world, sys_keys, fleet_keys) = make_world_with_fleets(2, &[0], false); // empire fleet
        let mut fog = FogState::new(Faction::Alliance);

        let mut ms = MovementState::new();
        ms.order(fleet_keys[0], sys_keys[0], sys_keys[1], 10);
        ms.orders_mut()
            .get_mut(&fleet_keys[0])
            .unwrap()
            .ticks_elapsed = 8;

        let events = FogSystem::advance(&mut fog, &world, &ms);
        assert!(events.is_empty());
        assert!(!fog.is_visible(sys_keys[1]));
    }

    #[test]
    fn empire_fog_tracks_empire_fleets() {
        let (world, sys_keys, _) = make_world_with_fleets(2, &[1], false); // empire at sys 1
        let mut fog = FogState::new(Faction::Empire);
        FogSystem::seed(&mut fog, &world);
        assert!(!fog.is_visible(sys_keys[0]));
        assert!(fog.is_visible(sys_keys[1]));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    fn a_fleet_with_sensors_reveals_no_neighbouring_system() {
        // FUN_0050ac80: a side sees a system only from a fleet in it; the
        // original has no sensor reach (ghidra/notes/fog-of-war.md).
        use crate::ids::DatId;
        use crate::world::{CapitalShipClass, ShipInstance};

        let mut world = GameWorld::default();
        let sector = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "Test".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });

        // Fleet at (0, 0), nearby system at (100, 0), far system at (1000, 0).
        // detection=10 → radius = 10 * 15.0 = 150 units → (100,0) within range, (1000,0) not.
        let sys0 = world.systems.insert(crate::world::System {
            last_arrival: None,
            dat_id: DatId(0),
            name: "Base".into(),
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
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Uncontrolled,
        });
        let sys_near = world.systems.insert(crate::world::System {
            last_arrival: None,
            dat_id: DatId(1),
            name: "Near".into(),
            sector,
            x: 100,
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
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Uncontrolled,
        });
        let sys_far = world.systems.insert(crate::world::System {
            last_arrival: None,
            dat_id: DatId(2),
            name: "Far".into(),
            sector,
            x: 1000,
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
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Uncontrolled,
        });

        let ship_class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId(0),
            name: "Sensor Ship".into(),
            is_alliance: true,
            is_empire: false,
            hull: 100,
            shield_strength: 50,
            sub_light_engine: 5,
            maneuverability: 5,
            hyperdrive: 2,
            detection: 10,
            ..CapitalShipClass::default()
        });

        world.fleets.insert(Fleet {
            location: sys0,
            capital_ships: vec![ShipInstance::new(ship_class, 100, true)],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });

        let mut fog = FogState::new(Faction::Alliance);
        fog.reveal(sys0); // fleet's own system already visible
        let ms = MovementState::new();
        let events = FogSystem::advance(&mut fog, &world, &ms);

        assert!(!fog.is_visible(sys_near));
        assert!(!fog.is_visible(sys_far));
        assert!(events.is_empty());
    }
}
