//! Capital ship and squadron repair: each damaged unit's timer `0x380`
//! (`ghidra/notes/ship-repair.md`, `ghidra/notes/squadron-strength.md`).
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
//! - a squadron's timer is kept on its entry
//!   ([`crate::world::SquadronDamage`]) and changes through
//!   [`RepairEvent::SquadronRepaired`]; squadrons have no ids;
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
use crate::squadrons::SquadronPlace;
use crate::tick::TickEvent;
use crate::world::{FighterEntry, GameWorld, SquadronDamage};

/// GNPRTB 2052 (2): timer `0x380`'s period with FastRepair (`FUN_005010a0`).
const GNPRTB_FAST_REPAIR_PERIOD: u16 = 2052;
/// GNPRTB 2053 (30): its period without.
const GNPRTB_REPAIR_PERIOD: u16 = 2053;
/// GNPRTB 4608 (2): a squadron's period with FastRepair (`FUN_00503140`).
const GNPRTB_SQUADRON_FAST_REPAIR_PERIOD: u16 = 4608;
/// GNPRTB 4609 (30): its period without.
const GNPRTB_SQUADRON_REPAIR_PERIOD: u16 = 4609;

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
    /// A squadron below full strength changed: its timer was armed, rebuilt
    /// or dropped, or a fire gave a craft back. `damage` is its new state,
    /// `None` once it is at full strength again (`FUN_00503530`); the
    /// entry is the `index`th of `place`'s squadrons.
    SquadronRepaired {
        place: SquadronPlace,
        index: usize,
        lost_before: u32,
        damage: Option<SquadronDamage>,
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
        squadron_repairs(world, tick, &mut events);
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

/// The squadrons' timers `0x380`: a squadron is Damaged while it has
/// losses (`FUN_00503340`: `+0x254` non-zero) and Usable in a fleet in a
/// system's list or on a planet that stands; travelling it is in neither,
/// and its timer drops (`FUN_005578d0`). The period is GNPRTB 4608 (2)
/// with FastRepair, else 4609 (30) (slot `+0x1d8`, `FUN_00503140`); each
/// fire gives one craft back (`FUN_00503530` -> `FUN_00503400(1)`). The
/// squadron's side is its class's.
fn squadron_repairs(world: &GameWorld, tick: u64, events: &mut Vec<RepairEvent>) {
    let period = |fast: bool| {
        u64::try_from(
            world
                .gnprtb
                .value(
                    if fast {
                        GNPRTB_SQUADRON_FAST_REPAIR_PERIOD
                    } else {
                        GNPRTB_SQUADRON_REPAIR_PERIOD
                    },
                    world.difficulty_index,
                )
                .max(1),
        )
        .unwrap_or(1)
    };
    let mut places: Vec<(SquadronPlace, Option<SystemKey>, &[FighterEntry])> = Vec::new();
    for (fleet_key, fleet) in &world.fleets {
        let usable = world
            .systems
            .get(fleet.location)
            .is_some_and(|sys| sys.fleets.contains(&fleet_key));
        places.push((
            SquadronPlace::Fleet(fleet_key),
            usable.then_some(fleet.location),
            &fleet.fighters,
        ));
    }
    for (system_key, system) in &world.systems {
        let usable = !system.is_destroyed;
        places.push((
            SquadronPlace::Planet(system_key),
            usable.then_some(system_key),
            &system.fighters,
        ));
    }
    for (place, location, entries) in places {
        for (index, entry) in entries.iter().enumerate() {
            let Some(damage) = entry.damage else {
                continue;
            };
            let changed = |damage: Option<SquadronDamage>| RepairEvent::SquadronRepaired {
                place,
                index,
                lost_before: entry.lost(),
                damage,
            };
            let Some(system) = location.filter(|_| entry.count > 0 && damage.lost > 0) else {
                if damage.repair.is_some() {
                    events.push(changed(Some(SquadronDamage {
                        repair: None,
                        ..damage
                    })));
                }
                continue;
            };
            let alliance = crate::squadrons::is_alliance(world, entry.class).unwrap_or(false);
            let fast = fast_repair(world, system, alliance);
            let timer = match damage.repair {
                // FUN_00557c30: a new period rebuilds the timer from today.
                Some(timer) if timer.fast == fast => timer,
                _ => RepairTimer {
                    due: tick + period(fast),
                    fast,
                },
            };
            if timer.due > tick {
                if damage.repair != Some(timer) {
                    events.push(changed(Some(SquadronDamage {
                        repair: Some(timer),
                        ..damage
                    })));
                }
                continue;
            }
            // FUN_00503530 -> FUN_00503400(1): one craft back; whole, it is
            // no longer Damaged and its timer drops.
            let lost = damage.lost - 1;
            events.push(changed((lost > 0).then_some(SquadronDamage {
                lost,
                repair: Some(RepairTimer {
                    due: timer.due + period(fast),
                    fast,
                }),
            })));
        }
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
            special_forces: Vec::new(),
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
                RepairEvent::RepairCheckPerformed { .. }
                | RepairEvent::SquadronRepaired { .. } => None,
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

    /// GNPRTB 4608 = 2 and 4609 = 30, as shipped (`ship-repair.md`).
    fn squadron_gnprtb() -> GnprtbParams {
        GnprtbParams::uniform(&[(4608, 2), (4609, 30)])
    }

    /// An Empire squadron class of 12 craft, and one squadron of it at
    /// `place` that has lost `lost` craft.
    fn damaged_squadron(world: &mut GameWorld, place: SquadronPlace, lost: u32) {
        let class = world.fighter_classes.insert(FighterClass {
            name: "TIE Fighter".into(),
            is_empire: true,
            squadron_size: 12,
            ..FighterClass::default()
        });
        let entry = FighterEntry {
            class,
            count: 1,
            carrier: None,
            damage: Some(SquadronDamage { lost, repair: None }),
        };
        match place {
            SquadronPlace::Fleet(fleet) => world.fleets[fleet].fighters.push(entry),
            SquadronPlace::Planet(system) => world.systems[system].fighters.push(entry),
        }
    }

    /// Run the repair on `days`, applying its squadron events as the
    /// integrator does; returns each day's squadron events.
    fn run_squadrons(
        world: &mut GameWorld,
        days: std::ops::RangeInclusive<u64>,
    ) -> Vec<(u64, RepairEvent)> {
        let mut state = RepairState::default();
        let mut seen = Vec::new();
        for tick in days {
            for event in RepairSystem::advance(&mut state, world, &day(tick)) {
                if let RepairEvent::SquadronRepaired {
                    place,
                    index,
                    damage,
                    ..
                } = event
                {
                    crate::squadrons::set_damage(world, place, index, damage);
                    seen.push((tick, event));
                }
            }
        }
        seen
    }

    /// The days a craft came back, with the losses left after it.
    fn regained(events: &[(u64, RepairEvent)]) -> Vec<(u64, u32)> {
        events
            .iter()
            .filter_map(|(tick, event)| match event {
                RepairEvent::SquadronRepaired {
                    lost_before,
                    damage,
                    ..
                } => {
                    let after = damage.map_or(0, |value| value.lost);
                    (after < *lost_before).then_some((*tick, after))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_squadron_regains_one_craft_every_two_days_at_its_sides_shipyard() {
        // FUN_00503140: GNPRTB 4608 (2) with FastRepair (FUN_0050c580); each
        // fire FUN_00503530 -> FUN_00503400(1) gives one craft back.
        let mut world = GameWorld {
            gnprtb: squadron_gnprtb(),
            ..GameWorld::default()
        };
        let sys = system(&mut world, true);
        let fleet = fleet(&mut world, sys, 200);
        damaged_squadron(&mut world, SquadronPlace::Fleet(fleet), 3);

        let events = run_squadrons(&mut world, 1..=12);
        assert_eq!(regained(&events), [(3, 2), (5, 1), (7, 0)]);
        // The first day arms the timer.
        assert!(matches!(
            events[0],
            (
                1,
                RepairEvent::SquadronRepaired {
                    lost_before: 3,
                    damage: Some(SquadronDamage {
                        lost: 3,
                        repair: Some(RepairTimer { due: 3, fast: true })
                    }),
                    ..
                }
            )
        ));
        // Whole, it is no longer Damaged (FUN_00503340) and its timer drops:
        // the last event is the one that clears it, once.
        let cleared = events
            .iter()
            .filter(|(_, event)| {
                matches!(event, RepairEvent::SquadronRepaired { lost_before, damage: None, .. } if *lost_before > 0)
            })
            .count();
        assert_eq!(cleared, 1);
        assert_eq!(events.last().map(|(tick, _)| *tick), Some(7));
        assert_eq!(world.fleets[fleet].fighters[0].damage, None);
    }

    #[test]
    fn away_from_a_shipyard_a_planets_squadron_regains_a_craft_every_thirty_days() {
        // FUN_00503140: GNPRTB 4609 (30) without FastRepair. A squadron on
        // a planet is Usable and repairs there (FUN_005578d0).
        let mut world = GameWorld {
            gnprtb: squadron_gnprtb(),
            ..GameWorld::default()
        };
        let sys = system(&mut world, false);
        damaged_squadron(&mut world, SquadronPlace::Planet(sys), 2);

        let events = run_squadrons(&mut world, 1..=70);
        assert_eq!(regained(&events), [(31, 1), (61, 0)]);
        assert_eq!(world.systems[sys].fighters[0].damage, None);
    }

    #[test]
    fn a_new_period_rebuilds_a_squadrons_timer_and_an_emptied_entry_has_none() {
        // FUN_00557860 -> FUN_00557c30: FastRepair changing rebuilds the
        // timer from that day (GNPRTB 4609 -> 4608). An entry with no
        // squadron left is no unit, so its timer drops.
        let mut world = GameWorld {
            gnprtb: squadron_gnprtb(),
            ..GameWorld::default()
        };
        let sys = system(&mut world, false);
        let fleet = fleet(&mut world, sys, 200);
        damaged_squadron(&mut world, SquadronPlace::Fleet(fleet), 2);
        run_squadrons(&mut world, 1..=1);
        assert_eq!(
            world.fleets[fleet].fighters[0].damage.and_then(|d| d.repair),
            Some(RepairTimer { due: 31, fast: false })
        );
        let yard = world.manufacturing_facilities.insert(ManufacturingFacilityInstance {
            class_dat_id: DatId::new(0x2800_0001),
            side: Faction::Empire,
            is_shipyard: true,
        });
        world.systems[sys].manufacturing_facilities.push(yard);
        let events = run_squadrons(&mut world, 5..=7);
        assert_eq!(regained(&events), [(7, 1)]);

        world.fleets[fleet].fighters[0].count = 0;
        run_squadrons(&mut world, 8..=8);
        assert_eq!(
            world.fleets[fleet].fighters[0].damage,
            Some(SquadronDamage { lost: 1, repair: None })
        );
    }

    #[test]
    fn a_travelling_squadron_drops_its_timer_and_a_whole_one_has_none() {
        // FUN_005578d0: not Usable while its fleet travels (in no system's
        // list), so UnderRepair goes off and the timer with it.
        let mut world = GameWorld {
            gnprtb: squadron_gnprtb(),
            ..GameWorld::default()
        };
        let sys = system(&mut world, true);
        let fleet = fleet(&mut world, sys, 200);
        damaged_squadron(&mut world, SquadronPlace::Fleet(fleet), 1);
        let class = world.fleets[fleet].fighters[0].class;
        crate::squadrons::add(&mut world.fleets[fleet].fighters, class, 2);
        run_squadrons(&mut world, 1..=1);
        assert!(world.fleets[fleet].fighters[0].damage.and_then(|d| d.repair).is_some());
        assert_eq!(world.fleets[fleet].fighters[1].damage, None);

        world.systems[sys].fleets.clear();
        let events = run_squadrons(&mut world, 2..=40);
        assert!(regained(&events).is_empty());
        assert_eq!(
            world.fleets[fleet].fighters[0].damage,
            Some(SquadronDamage { lost: 1, repair: None })
        );
    }
}
