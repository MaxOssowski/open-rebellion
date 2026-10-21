//! Capital ship repair: each damaged ship's timer `0x380`
//! (`ghidra/notes/ship-repair.md`).
//!
//! A ship is UnderRepair while it is Damaged and Usable (`FUN_005578d0`):
//! anywhere, as long as it does not travel. Turning the flag on arms timer
//! `0x380` on the ship's record (`FUN_005577f0` -> `FUN_00557c00`). Its
//! period is GNPRTB 2052 (2) with FastRepair, else 2053 (30) (slot `+0x1d8`,
//! `FUN_005010a0`). FastRepair (`FUN_0050c580`) holds at a system of the
//! ship's side whose manager has a usable, unsuspended yard (`FUN_0050bb70`,
//! `FUN_00528040(0)`, system `+0x88` bit `0x400`); a change of it rebuilds
//! the timer from that day (`FUN_00557860` -> `FUN_00557c30`). A ship is
//! Damaged with hull damage or any subsystem hit (`FUN_00501c00`). Each
//! fire (`FUN_00501c50`) repairs a tenth of the class's hull
//! (`FUN_00500550`, `FUN_00501b70`), then, on a roll of 0..99 under the
//! class's `damage_control`, one subsystem hit chosen evenly among them
//! ([`repair_hit`], `ghidra/notes/ship-subsystem-damage.md`).
//!
//! port:
//! - a fleet in hyperspace is in no system's list, which stands for the
//!   ship's en route bit;
//! - a ship still unnumbered (id 0) waits for its id;
//! - the port keeps no partial squadron damage, so squadrons have no timer;
//! - a fire's draws come from a `SplitMix64` stream seeded by the day and
//!   the ship's id (the original draws from its game generator,
//!   `FUN_005f5700`);
//! - the delay spread is not set (hyp: 0).
//!
//! # Advance contract
//! `RepairSystem::advance()` never mutates `GameWorld`.
//! It returns `Vec<RepairEvent>` for the caller to apply.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::ids::{FleetKey, SystemKey};
use crate::tick::TickEvent;
use crate::world::GameWorld;

/// GNPRTB 2052 (2): timer `0x380`'s period with FastRepair (`FUN_005010a0`).
const GNPRTB_FAST_REPAIR_PERIOD: u16 = 2052;
/// GNPRTB 2053 (30): its period without.
const GNPRTB_REPAIR_PERIOD: u16 = 2053;

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events produced by the repair system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RepairEvent {
    /// A ship's hull was restored.
    ShipRepaired {
        fleet: FleetKey,
        ship_index: usize,
        hull_before: i32,
        hull_after: i32,
        /// The ship's `+0x64` hit counters after the fire.
        #[serde(default)]
        hits_after: u32,
    },
    /// Repair started for damaged ships in one fleet: their timers were
    /// armed. Never emitted for a healthy fleet.
    RepairCheckPerformed {
        system: SystemKey,
        fleet: FleetKey,
        ships_checked: usize,
    },
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// A ship's armed timer `0x380`: the day it next fires and whether it runs
/// at the FastRepair period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairTimer {
    pub due: u64,
    pub fast: bool,
}

/// Each repairing ship's timer, by ship id, and the fleets holding one
/// (save v98).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepairState {
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_set",
        deserialize_with = "crate::serde_ordered::deserialize_hash_set"
    )]
    active_fleets: HashSet<FleetKey>,
    #[serde(
        default,
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    timers: HashMap<u64, RepairTimer>,
}

impl RepairState {
    /// Whether a fleet holds a ship under repair.
    #[must_use]
    pub fn is_repairing(&self, fleet: FleetKey) -> bool {
        self.active_fleets.contains(&fleet)
    }

    /// A ship's armed timer, by its id.
    #[must_use]
    pub fn timer(&self, ship_id: u64) -> Option<RepairTimer> {
        self.timers.get(&ship_id).copied()
    }
}

/// FastRepair for a ship of `is_alliance` at `system` (`FUN_0050c580`): the
/// system is of the ship's side and holds a usable, unsuspended yard
/// (`FUN_0050bb70`, `FUN_00528040(0)`).
///
/// port: a yard is a shipyard of the ship's side at the system; a system
/// in revolt or blockaded suspends its yards (`FUN_0050bbb0`).
fn fast_repair(world: &GameWorld, system: SystemKey, is_alliance: bool) -> bool {
    let Some(sys) = world.systems.get(system) else {
        return false;
    };
    let side = Faction::of_alliance(is_alliance);
    if sys.is_destroyed || crate::uprising::holder(sys) != Some(side) {
        return false;
    }
    if matches!(sys.control, crate::world::ControlKind::Uprising(_))
        || crate::blockade::BlockadeSystem::system_is_blockaded(world, sys)
    {
        return false;
    }
    sys.manufacturing_facilities.iter().any(|&key| {
        world
            .manufacturing_facilities
            .get(key)
            .is_some_and(|yard| yard.is_shipyard && yard.side == side)
    })
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

pub struct RepairSystem;

impl RepairSystem {
    /// Arm the timers of the ships under repair, drop the others, and fire
    /// the ones due by the last tick. Returns the repairs for the caller to
    /// apply to `GameWorld`.
    pub fn advance(
        state: &mut RepairState,
        world: &GameWorld,
        tick_events: &[TickEvent],
    ) -> Vec<RepairEvent> {
        let Some(last) = tick_events.last() else {
            return Vec::new();
        };
        let tick = last.tick;
        let period = |fast: bool| {
            u64::try_from(
                world
                    .gnprtb
                    .value(
                        if fast {
                            GNPRTB_FAST_REPAIR_PERIOD
                        } else {
                            GNPRTB_REPAIR_PERIOD
                        },
                        world.difficulty_index,
                    )
                    .max(1),
            )
            .unwrap_or(1)
        };

        let mut events = Vec::new();
        let mut under_repair = HashSet::new();
        let mut repairing_now = HashSet::new();
        for (fleet_key, fleet) in &world.fleets {
            // Usable: a fleet in hyperspace is in no system's list.
            let Some(sys) = world.systems.get(fleet.location) else {
                continue;
            };
            if !sys.fleets.contains(&fleet_key) {
                continue;
            }
            let fast = fast_repair(world, fleet.location, fleet.is_alliance);
            let mut armed = 0;
            for (ship_index, ship) in fleet.capital_ships.iter().enumerate() {
                if !ship.alive || ship.id == 0 {
                    continue;
                }
                let Some(class) = world.capital_ship_classes.get(ship.class) else {
                    continue;
                };
                let hull = i32::try_from(class.hull).unwrap_or(i32::MAX);
                // Damaged (FUN_00501c00): hull damage or a subsystem hit.
                if ship.hull_current >= hull && ship.subsystem_hits == 0 {
                    continue;
                }
                under_repair.insert(ship.id);
                repairing_now.insert(fleet_key);
                let timer = state.timers.entry(ship.id).or_insert_with(|| {
                    armed += 1;
                    RepairTimer {
                        due: tick + period(fast),
                        fast,
                    }
                });
                // FUN_00557c30: a new period rebuilds the timer from today.
                if timer.fast != fast {
                    *timer = RepairTimer {
                        due: tick + period(fast),
                        fast,
                    };
                }
                if timer.due > tick {
                    continue;
                }
                timer.due += period(fast);
                // FUN_00501c50 -> FUN_00501b70: damage less a tenth of the
                // class hull (a whole hull stays whole).
                let hull_after = (ship.hull_current + hull / 10).min(hull);
                let mut draws = FireDraws::new(tick, ship.id);
                let hits_after =
                    repair_hit(ship.subsystem_hits, class.damage_control, |m| draws.draw(m));
                events.push(RepairEvent::ShipRepaired {
                    fleet: fleet_key,
                    ship_index,
                    hull_before: ship.hull_current,
                    hull_after,
                    hits_after,
                });
            }
            if armed > 0 {
                events.push(RepairEvent::RepairCheckPerformed {
                    system: fleet.location,
                    fleet: fleet_key,
                    ships_checked: armed,
                });
            }
        }
        // A ship no longer Damaged and Usable drops its timer
        // (FUN_005578d0 -> FUN_005577f0 off).
        state.timers.retain(|id, _| under_repair.contains(id));
        state
            .active_fleets
            .retain(|fleet| repairing_now.contains(fleet));
        state.active_fleets.extend(repairing_now);
        events
    }
}

/// `FUN_00501c50`'s subsystem step on the `+0x64` counters `hits`. With `n`
/// hits (`FUN_005006a0`), a roll of 0..99 (`FUN_0053e2e0`,
/// `DAT_00661a88` = 100) under `damage_control` (class `+0xd8`,
/// `FUN_00558680`) draws `r` in 0..`n`-1 (`FUN_0053e290(n - 1)`). The
/// counters, in bit order, give `r` up while they do not exceed it; the
/// first above it drops by one. `draw(m)` gives 0..`m`-1.
pub fn repair_hit(hits: u32, damage_control: u32, mut draw: impl FnMut(u32) -> u32) -> u32 {
    use crate::world::{ShipInstance, Subsystem};
    let mut ship = ShipInstance::new(crate::ids::CapitalShipKey::default(), 0, false);
    ship.subsystem_hits = hits;
    let n: u32 = Subsystem::ALL
        .iter()
        .map(|&s| u32::from(ship.hits(s)))
        .sum();
    if n == 0 || draw(DAMAGE_CONTROL_ROLL) >= damage_control {
        return hits;
    }
    let mut r = draw(n);
    for subsystem in Subsystem::ALL {
        let count = u32::from(ship.hits(subsystem));
        if count == 0 || count <= r {
            r -= count;
            continue;
        }
        ship.set_hits(subsystem, ship.hits(subsystem) - 1);
        break;
    }
    ship.subsystem_hits
}

/// `DAT_00661a88` (100): the damage-control roll's range (`FUN_0053e2e0`).
const DAMAGE_CONTROL_ROLL: u32 = 100;

/// port: one repair fire's draws, a `SplitMix64` stream seeded by the day
/// and the ship's id, in place of the game generator (`FUN_005f5700`).
struct FireDraws(u64);

impl FireDraws {
    /// port: the seed mixes the day with the ship's id (golden-ratio step).
    fn new(tick: u64, ship_id: u64) -> Self {
        Self(tick.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ ship_id)
    }

    /// port: `SplitMix64`'s step and mix, as `bombardment::BombardEnv::draw`;
    /// a draw from 0 to `m - 1`.
    fn draw(&mut self, m: u32) -> u32 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        u32::try_from(z % u64::from(m.max(1))).unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{ExplorationStatus, Faction};
    use crate::ids::DatId;
    use crate::world::*;

    /// GNPRTB 2052 = 2 and 2053 = 30, as shipped (`ship-repair.md`).
    fn gnprtb() -> GnprtbParams {
        GnprtbParams::uniform(&[(2052, 2), (2053, 30)])
    }

    fn system(world: &mut GameWorld, shipyard: bool) -> SystemKey {
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(0x9200_0000),
            name: "Test Sector".into(),
            group: crate::dat::SectorGroup::Core,
            x: 100,
            y: 100,
            systems: vec![],
        });
        let yards = if shipyard {
            vec![world
                .manufacturing_facilities
                .insert(ManufacturingFacilityInstance {
                    class_dat_id: DatId::new(0x2800_0001),
                    side: Faction::Empire,
                    is_shipyard: true,
                })]
        } else {
            vec![]
        };
        world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0000),
            name: "Kuat".into(),
            sector,
            x: 100,
            y: 100,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.3,
            popularity_empire: 0.7,
            is_populated: true,
            total_energy: 10,
            raw_materials: 8,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: yards,
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Empire),
        })
    }

    /// An Empire fleet at `sys` with one numbered ship of hull 200 at
    /// `hull`.
    fn fleet(world: &mut GameWorld, sys: SystemKey, hull: i32) -> FleetKey {
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            name: "Star Destroyer".into(),
            is_empire: true,
            hull: 200,
            damage_control: 5,
            ..CapitalShipClass::default()
        });
        let mut ship = ShipInstance::new(class, hull, false);
        world.last_ship_id += 1;
        ship.id = world.last_ship_id;
        let key = world.fleets.insert(Fleet {
            location: sys,
            capital_ships: vec![ship],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[sys].fleets.push(key);
        key
    }

    fn day(n: u64) -> [TickEvent; 1] {
        [TickEvent { tick: n }]
    }

    fn repaired(events: &[RepairEvent]) -> Vec<(i32, i32)> {
        events
            .iter()
            .filter_map(|event| match event {
                RepairEvent::ShipRepaired {
                    hull_before,
                    hull_after,
                    ..
                } => Some((*hull_before, *hull_after)),
                RepairEvent::RepairCheckPerformed { .. } => None,
            })
            .collect()
    }

    // port: a fire's draws are SplitMix64 (the reference outputs from seed
    // 0 are 0xe220a8397b1dcdaf, 0x6e789e6aa1b965f4, 0x06c45d188009454f),
    // seeded by the day times the golden-ratio step, xor the ship's id.
    #[test]
    fn a_fires_draws_follow_splitmix64() {
        let mut draws = FireDraws::new(0, 0);
        assert_eq!(
            [draws.draw(1000), draws.draw(1000), draws.draw(1000)],
            [535, 700, 679]
        );
        let mut draws = FireDraws::new(0, 0);
        assert_eq!([draws.draw(7), draws.draw(7), draws.draw(7)], [2, 1, 2]);
        assert_eq!(FireDraws::new(0, 0).draw(0), 0);
        assert_eq!(FireDraws::new(2, 5).0, 2u64.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 5);
    }

    // FUN_00501c50: with n hits, a roll of 0..99 under damage_control draws
    // r in 0..n-1; the counters, in bit order, give r up while they do not
    // exceed it, and the first above it drops by one.
    #[test]
    fn a_fire_mends_the_hit_its_draw_falls_on() {
        use crate::world::Subsystem;
        // Shield 1, weapon 2, speed 1: n = 4.
        let mut ship = ShipInstance::new(crate::ids::CapitalShipKey::default(), 0, false);
        ship.set_hits(Subsystem::Shield, 1);
        ship.set_hits(Subsystem::Weapon, 2);
        ship.set_hits(Subsystem::Speed, 1);
        let hits = ship.subsystem_hits;
        let mend = |roll: u32, r: u32| {
            let mut draws = vec![roll, r].into_iter();
            let mut bounds = Vec::new();
            let after = repair_hit(hits, 30, |m| {
                bounds.push(m);
                draws.next().unwrap()
            });
            (after, bounds)
        };
        let (after, bounds) = mend(0, 0);
        assert_eq!(bounds, [100, 4]);
        assert_eq!(after, 0x1020);
        assert_eq!(mend(29, 1).0, 0x1011);
        assert_eq!(mend(29, 2).0, 0x1011);
        assert_eq!(mend(29, 3).0, 0x0021);
        // A roll at or over damage_control mends nothing.
        let (after, bounds) = mend(30, 0);
        assert_eq!((after, bounds), (hits, vec![100]));
        // With no hit nothing is drawn.
        let mut drawn = 0;
        assert_eq!(
            repair_hit(0, 100, |_| {
                drawn += 1;
                0
            }),
            0
        );
        assert_eq!(drawn, 0);
    }

    // FUN_00501c00: a ship whole of hull but with a subsystem hit is Damaged,
    // goes under repair, and its fires skip the hull.
    #[test]
    fn a_ship_with_only_subsystem_hits_is_repaired() {
        let mut world = GameWorld {
            gnprtb: gnprtb(),
            ..GameWorld::default()
        };
        let sys = system(&mut world, true);
        let fleet = fleet(&mut world, sys, 200);
        world.capital_ship_classes[world.fleets[fleet].capital_ships[0].class].damage_control = 100;
        world.fleets[fleet].capital_ships[0].set_hits(crate::world::Subsystem::Tractor, 1);
        let mut state = RepairState::default();
        let _ = RepairSystem::advance(&mut state, &world, &day(10));
        assert!(state.is_repairing(fleet));
        let fired = RepairSystem::advance(&mut state, &world, &day(12));
        assert!(fired.iter().any(|event| matches!(
            event,
            RepairEvent::ShipRepaired {
                hull_before: 200,
                hull_after: 200,
                hits_after: 0,
                ..
            }
        )));
    }

    #[test]
    fn a_damaged_ship_repairs_a_tenth_of_its_hull_every_two_days_at_its_shipyard() {
        // FUN_005010a0: GNPRTB 2052 (2) with FastRepair; FUN_00501c50 ->
        // FUN_00501b70: damage less class hull / 10 (FUN_00500550).
        let mut world = GameWorld {
            gnprtb: gnprtb(),
            ..GameWorld::default()
        };
        let sys = system(&mut world, true);
        let fleet = fleet(&mut world, sys, 150);
        let mut state = RepairState::default();

        let armed = RepairSystem::advance(&mut state, &world, &day(10));
        assert!(repaired(&armed).is_empty());
        assert!(armed.iter().any(|event| matches!(
            event,
            RepairEvent::RepairCheckPerformed { fleet: f, ships_checked: 1, .. } if *f == fleet
        )));
        let id = world.fleets[fleet].capital_ships[0].id;
        assert_eq!(state.timer(id), Some(RepairTimer { due: 12, fast: true }));
        assert!(repaired(&RepairSystem::advance(&mut state, &world, &day(11))).is_empty());
        assert_eq!(
            repaired(&RepairSystem::advance(&mut state, &world, &day(12))),
            [(150, 170)]
        );
        assert_eq!(state.timer(id).map(|t| t.due), Some(14));
        assert!(state.is_repairing(fleet));
    }

    #[test]
    fn away_from_a_shipyard_a_ship_repairs_every_thirty_days() {
        // FUN_0050c580: no usable yard of the side, so GNPRTB 2053 (30).
        // There is no shipyard condition on repair itself (FUN_005578d0).
        let mut world = GameWorld {
            gnprtb: gnprtb(),
            ..GameWorld::default()
        };
        let sys = system(&mut world, false);
        let fleet = fleet(&mut world, sys, 100);
        let mut state = RepairState::default();
        RepairSystem::advance(&mut state, &world, &day(1));
        let id = world.fleets[fleet].capital_ships[0].id;
        assert_eq!(state.timer(id), Some(RepairTimer { due: 31, fast: false }));
        assert!(repaired(&RepairSystem::advance(&mut state, &world, &day(30))).is_empty());
        assert_eq!(
            repaired(&RepairSystem::advance(&mut state, &world, &day(31))),
            [(100, 120)]
        );
    }

    #[test]
    fn a_new_period_rebuilds_the_timer_and_a_healthy_or_travelling_ship_has_none() {
        // FUN_00557860 -> FUN_00557c30: FastRepair changes the delay and
        // the timer counts again from that day. FUN_005578d0: no timer
        // without damage, or while not usable (en route).
        let mut world = GameWorld {
            gnprtb: gnprtb(),
            ..GameWorld::default()
        };
        let sys = system(&mut world, false);
        let fleet = fleet(&mut world, sys, 100);
        let id = world.fleets[fleet].capital_ships[0].id;
        let mut state = RepairState::default();
        RepairSystem::advance(&mut state, &world, &day(1));
        // A shipyard of the side opens at the system.
        let yard = world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: DatId::new(0x2800_0001),
                side: Faction::Empire,
                is_shipyard: true,
            });
        world.systems[sys].manufacturing_facilities.push(yard);
        RepairSystem::advance(&mut state, &world, &day(5));
        assert_eq!(state.timer(id), Some(RepairTimer { due: 7, fast: true }));
        // An enemy-held system gives no FastRepair.
        world.systems[sys].control = ControlKind::Controlled(Faction::Alliance);
        RepairSystem::advance(&mut state, &world, &day(6));
        assert_eq!(state.timer(id), Some(RepairTimer { due: 36, fast: false }));
        // In hyperspace (out of every system's list) the timer drops.
        world.systems[sys].fleets.clear();
        RepairSystem::advance(&mut state, &world, &day(7));
        assert_eq!(state.timer(id), None);
        assert!(!state.is_repairing(fleet));
        // Healthy: no timer, no event.
        world.systems[sys].fleets.push(fleet);
        world.fleets[fleet].capital_ships[0].hull_current = 200;
        assert!(RepairSystem::advance(&mut state, &world, &day(8)).is_empty());
        assert_eq!(state.timer(id), None);
    }

    #[test]
    fn an_unnumbered_or_lost_ship_and_an_empty_day_repair_nothing() {
        let mut world = GameWorld {
            gnprtb: gnprtb(),
            ..GameWorld::default()
        };
        let sys = system(&mut world, true);
        let fleet = fleet(&mut world, sys, 100);
        let mut state = RepairState::default();
        assert!(RepairSystem::advance(&mut state, &world, &[]).is_empty());
        world.fleets[fleet].capital_ships[0].id = 0;
        assert!(RepairSystem::advance(&mut state, &world, &day(1)).is_empty());
        world.fleets[fleet].capital_ships[0].id = 9;
        world.fleets[fleet].capital_ships[0].alive = false;
        assert!(RepairSystem::advance(&mut state, &world, &day(2)).is_empty());
    }

    #[test]
    fn a_system_in_revolt_or_destroyed_suspends_fast_repair() {
        // FUN_0050bbb0: uprising or blockade suspends the yards, and
        // FUN_00528040(0) counts only unsuspended ones.
        let mut world = GameWorld {
            gnprtb: gnprtb(),
            ..GameWorld::default()
        };
        let sys = system(&mut world, true);
        assert!(fast_repair(&world, sys, false));
        assert!(!fast_repair(&world, sys, true));
        // A yard must be a shipyard of the unit's side.
        let yard = world.systems[sys].manufacturing_facilities[0];
        world.manufacturing_facilities[yard].is_shipyard = false;
        assert!(!fast_repair(&world, sys, false));
        world.manufacturing_facilities[yard].is_shipyard = true;
        world.manufacturing_facilities[yard].side = Faction::Alliance;
        assert!(!fast_repair(&world, sys, false));
        world.manufacturing_facilities[yard].side = Faction::Empire;
        world.systems[sys].control = ControlKind::Uprising(Faction::Empire);
        assert!(!fast_repair(&world, sys, false));
        world.systems[sys].control = ControlKind::Controlled(Faction::Empire);
        world.systems[sys].is_destroyed = true;
        assert!(!fast_repair(&world, sys, false));
    }
}
