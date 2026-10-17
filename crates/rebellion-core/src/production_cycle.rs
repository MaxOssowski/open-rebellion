//! Mines, refineries and each side's raw and refined material stockpiles
//! (`ghidra/notes/side-resource-monitors.md`).
//!
//! A mine or refinery runs a cycle: a mine ends it with one raw material, a
//! refinery starts it by taking one raw material and ends it with one
//! refined material. The cycle lengthens as the side's upkeep is allocated
//! to the facility.
//!
//! A yard starts its cycle by taking one refined material and ends it with
//! one build point for its production manager, which keeps no more yards
//! running than it has points left to make
//! (`ghidra/notes/manufacturing-manager.md`).
//!
//! Randomness comes from caller-supplied `[0, 1)` rolls consumed in order.
//!
//! port: the original runs on per-facility timers and change events; this
//! advances once per day, in system then facility order, and fixes a
//! cycle's length when it starts.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::blockade::BlockadeState;
use crate::build_selection::area_family;
use crate::dat::Faction;
use crate::ids::{ManufacturingFacilityKey, ProductionFacilityKey, SystemKey};
use crate::manufacturing::{ManufacturingAdvance, ManufacturingState, ProductionArea};
use crate::side_resources::{maintenance_totals, FACILITY_CAPACITY};
use crate::tick::TickEvent;
use crate::uprising::UprisingState;
use crate::world::GameWorld;

/// GNPRTB `parameter_id` `0x1000` (`DAT_006bb700`, loaded by
/// `FUN_0055e290`): the percent of a facility's capacity that one extra day
/// of cycle stands for. It is 20 in every column.
pub const CYCLE_STEP_PERCENT_PARAM: u16 = 0x1000;

/// Facility `+0x6c`, the ProductionModifier `FUN_0055a600` starts at 100.
/// port: nothing changes it yet (`FUN_0055a7a0`'s callers are not traced).
const PRODUCTION_MODIFIER: i32 = 100;

/// One side's stockpiles (side `+0x78`, `+0x7c`), the refineries waiting
/// for raw material (side `+0x88`, counted in `+0x80`) and the yards
/// waiting for refined material (side `+0x8c`, counted in `+0x84`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideStock {
    /// Raw materials on hand, the Raw Materials monitor's value.
    pub raw: i32,
    /// Refined materials on hand, the Refined Materials monitor's value.
    pub refined: i32,
    raw_requests: VecDeque<RawRequest>,
    refined_requests: VecDeque<RefinedRequest>,
}

impl SideStock {
    /// Side `+0x80`: refineries waiting for raw material.
    #[must_use]
    pub fn waiting_for_raw(&self) -> usize {
        self.raw_requests.len()
    }

    /// Side `+0x84`: yards waiting for refined material.
    #[must_use]
    pub fn waiting_for_refined(&self) -> usize {
        self.refined_requests.len()
    }
}

/// A queued request (`FUN_00533610`): the yard and its cycle counter when
/// it asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct RefinedRequest {
    yard: ManufacturingFacilityKey,
    cycle: u32,
}

/// A yard's state and cycle counter, as a mine's or refinery's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
struct YardCycle {
    phase: Phase,
    cycle: u32,
}

/// A queued request (`FUN_005306b0`): the refinery and its cycle counter
/// (`FUN_0053a6e0`) when it asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct RawRequest {
    facility: ProductionFacilityKey,
    cycle: u32,
}

/// `ProcFacilState` (facility `+0x58`): 4, 1 and 2. State 3 lasts only for
/// the cycle-end hook, so it is not stored.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    #[default]
    Inactive,
    Idle,
    Processing {
        done_tick: u64,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
struct FacilityCycle {
    phase: Phase,
    /// Facility `+0x68`: the part of its capacity allocated to upkeep.
    allocated: i32,
    /// `FUN_0053a6e0`: counts entries to and exits from state 1.
    cycle: u32,
    /// Facility `+0x60` bit 4: the next cycle's length is randomised.
    randomise: bool,
}

/// Persistent state of the production cycle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductionState {
    alliance: SideStock,
    empire: SideStock,
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    facilities: HashMap<ProductionFacilityKey, FacilityCycle>,
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    yards: HashMap<ManufacturingFacilityKey, YardCycle>,
    /// World construction has finished (`DAT_006b90e0`, mode `0x1a`).
    started: bool,
}

impl ProductionState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `side`'s stockpiles; a neutral side has none.
    #[must_use]
    pub fn stock(&self, side: Faction) -> Option<&SideStock> {
        match side {
            Faction::Alliance => Some(&self.alliance),
            Faction::Empire => Some(&self.empire),
            Faction::Neutral => None,
        }
    }

    /// `side`'s stockpiles, to change them.
    pub fn stock_mut(&mut self, side: Faction) -> Option<&mut SideStock> {
        match side {
            Faction::Alliance => Some(&mut self.alliance),
            Faction::Empire => Some(&mut self.empire),
            Faction::Neutral => None,
        }
    }

    /// The part of a facility's capacity allocated to upkeep (`+0x68`).
    #[must_use]
    pub fn allocated(&self, facility: ProductionFacilityKey) -> i32 {
        self.facilities
            .get(&facility)
            .map_or(0, |value| value.allocated)
    }

    /// True while the yard is in a cycle.
    #[must_use]
    pub fn yard_is_processing(&self, yard: ManufacturingFacilityKey) -> bool {
        matches!(
            self.yards.get(&yard).map(|value| value.phase),
            Some(Phase::Processing { .. })
        )
    }

    /// True while the yard waits for refined material to start a cycle.
    #[must_use]
    pub fn yard_is_waiting(&self, yard: ManufacturingFacilityKey) -> bool {
        self.yards.get(&yard).map(|value| value.phase) == Some(Phase::Idle)
    }

    /// True while the facility is in a cycle (`+0x60` bit 1).
    #[must_use]
    pub fn is_processing(&self, facility: ProductionFacilityKey) -> bool {
        matches!(
            self.facilities.get(&facility).map(|value| value.phase),
            Some(Phase::Processing { .. })
        )
    }
}

/// What a cycle end added to a side's stockpiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionEvent {
    /// `FUN_00530670`: a mine added one raw material.
    RawMined {
        side: Faction,
        facility: ProductionFacilityKey,
    },
    /// `FUN_005307e0`: a refinery added one refined material.
    MaterialRefined {
        side: Faction,
        facility: ProductionFacilityKey,
    },
}

/// `FUN_0055e2b0`: a cycle's length in days for a facility with `allocated`
/// of its capacity given to upkeep.
///
/// `step` is `step_percent` of the capacity, at least 1; each started step
/// of allocation adds a day to the class `rate`.
#[must_use]
pub fn cycle_period(rate: i32, allocated: i32, step_percent: i32, modifier: i32) -> i32 {
    let step = (FACILITY_CAPACITY * step_percent / 100).max(1);
    let extra = (allocated + step - 1) / step;
    (rate + extra) * modifier / 100
}

/// `FUN_0055e2b0` with facility `+0x60` bit 4 set: `FUN_0053e290(period)`
/// (`0..=period`) plus half the period.
#[must_use]
pub fn randomised_period(period: i32, roll: f64) -> i32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "roll is in [0, 1), so the product is below period + 1"
    )]
    let drawn = (roll * f64::from(period + 1)) as i32;
    drawn.min(period) + period / 2
}

/// `FUN_0052f6b0` / `FUN_0052f8f0`: move the allocation of one pool
/// (`facilities`, in walk order) toward `upkeep`, bounded by the pool's
/// capacity, with `FUN_0055a820` to grow and `FUN_0055a960` to shrink.
fn rebalance(
    cycles: &mut HashMap<ProductionFacilityKey, FacilityCycle>,
    facilities: &[ProductionFacilityKey],
    upkeep: i32,
) {
    let capacity = i32::try_from(facilities.len()).unwrap_or(i32::MAX) * FACILITY_CAPACITY;
    if capacity == 0 {
        return;
    }
    let mut pool: i32 = facilities
        .iter()
        .map(|key| cycles.get(key).map_or(0, |value| value.allocated))
        .sum();
    let delta = (upkeep - pool).min(capacity - pool);
    let shrink = delta < 0;
    let mut remaining = delta.abs();
    while remaining > 0 {
        let before = remaining;
        for key in facilities {
            if remaining == 0 {
                break;
            }
            let cycle = cycles.entry(*key).or_default();
            let moved = if shrink {
                let keep = (pool - 1) * FACILITY_CAPACITY / capacity;
                (cycle.allocated - keep).clamp(0, remaining)
            } else {
                let share = pool * FACILITY_CAPACITY / capacity;
                (share - cycle.allocated + 1)
                    .clamp(0, remaining)
                    .min(FACILITY_CAPACITY - cycle.allocated)
            };
            let signed = if shrink { -moved } else { moved };
            cycle.allocated += signed;
            pool += signed;
            remaining -= moved;
        }
        if remaining == before {
            break;
        }
    }
}

struct Rolls<'a> {
    rolls: &'a [f64],
    next: usize,
}

impl Rolls<'_> {
    /// The next roll; 0 when the caller supplied too few.
    fn draw(&mut self) -> f64 {
        let roll = self.rolls.get(self.next).copied().unwrap_or(0.0);
        self.next += 1;
        roll
    }
}

/// One day's view of the world for a side.
struct Day<'a, 'r> {
    world: &'a GameWorld,
    tick: u64,
    step_percent: i32,
    /// Refineries start without raw material until setup has finished
    /// (`FUN_0052fb30` with `DAT_006b90e0` clear).
    free_raw: bool,
    rolls: &'a mut Rolls<'r>,
    events: &'a mut Vec<ProductionEvent>,
}

impl Day<'_, '_> {
    /// `FUN_0053a860(1)`: start the cycle and arm timer `0x394`.
    fn begin(&mut self, key: ProductionFacilityKey, cycle: &mut FacilityCycle) {
        let rate = self
            .world
            .production_facilities
            .get(key)
            .and_then(|facility| self.world.buildable_classes.get(&facility.class_dat_id))
            .map_or(0, |class| i32::try_from(class.processing_rate).unwrap_or(0));
        let mut period = cycle_period(
            rate,
            cycle.allocated,
            self.step_percent,
            PRODUCTION_MODIFIER,
        );
        if cycle.randomise {
            period = randomised_period(period, self.rolls.draw());
            cycle.randomise = false;
        }
        // port: a class with no rate would never fire; give it one day.
        let period = u64::try_from(period.max(1)).unwrap_or(1);
        cycle.cycle += 1;
        cycle.phase = Phase::Processing {
            done_tick: self.tick + period,
        };
    }

    /// State 1 and its cycle-start hook: `FUN_00530650` for a mine,
    /// `FUN_005306b0` for a refinery.
    fn enter_idle(
        &mut self,
        key: ProductionFacilityKey,
        is_mine: bool,
        cycle: &mut FacilityCycle,
        stock: &mut SideStock,
    ) {
        cycle.phase = Phase::Idle;
        cycle.cycle += 1;
        if is_mine || self.free_raw {
            self.begin(key, cycle);
        } else if stock.raw > 0 {
            stock.raw -= 1;
            self.begin(key, cycle);
        } else {
            stock.raw_requests.push_back(RawRequest {
                facility: key,
                cycle: cycle.cycle,
            });
        }
    }

    /// `FUN_0052fbf0`: hand raw material to the waiting refineries, first
    /// come first served. A request whose refinery has left the state it
    /// asked in is dropped.
    fn serve_requests(
        &mut self,
        cycles: &mut HashMap<ProductionFacilityKey, FacilityCycle>,
        stock: &mut SideStock,
    ) {
        while stock.raw > 0 {
            let Some(request) = stock.raw_requests.pop_front() else {
                break;
            };
            let Some(cycle) = cycles.get_mut(&request.facility) else {
                continue;
            };
            if cycle.phase == Phase::Idle && cycle.cycle == request.cycle {
                stock.raw -= 1;
                self.begin(request.facility, cycle);
            }
        }
    }
}

/// Stateless system that advances the production cycle.
pub struct ProductionSystem;

impl ProductionSystem {
    /// Advance one day per tick event. `blocked` tells whether a system is
    /// in uprising or blockaded (`FUN_0050bbb0`).
    pub fn advance(
        state: &mut ProductionState,
        world: &GameWorld,
        manufacturing: &ManufacturingState,
        blocked: impl Fn(SystemKey) -> bool,
        tick_events: &[TickEvent],
        rolls: &[f64],
    ) -> Vec<ProductionEvent> {
        let mut events = Vec::new();
        let mut rolls = Rolls { rolls, next: 0 };
        let step_percent = world
            .gnprtb
            .value(CYCLE_STEP_PERCENT_PARAM, world.difficulty_index);
        state
            .facilities
            .retain(|key, _| world.production_facilities.contains_key(*key));

        for tick_event in tick_events {
            for side in [Faction::Alliance, Faction::Empire] {
                // The side's counted facilities, and the rest of its own.
                let (mut mines, mut refineries, mut idle) = (Vec::new(), Vec::new(), Vec::new());
                for (system, value) in &world.systems {
                    let counted = !blocked(system);
                    for &key in &value.production_facilities {
                        let Some(facility) = world.production_facilities.get(key) else {
                            continue;
                        };
                        if facility.side != side {
                            continue;
                        }
                        if !counted {
                            idle.push(key);
                        } else if facility.is_mine {
                            mines.push(key);
                        } else {
                            refineries.push(key);
                        }
                    }
                }
                // `FUN_0055ab60`: a facility that is not counted gives up
                // its allocation and goes to state 4.
                for key in idle {
                    let cycle = state.facilities.entry(key).or_default();
                    if cycle.phase == Phase::Idle {
                        cycle.cycle += 1;
                    }
                    cycle.phase = Phase::Inactive;
                    cycle.allocated = 0;
                }

                let upkeep = maintenance_totals(world, manufacturing, side, &blocked).upkeep;
                rebalance(&mut state.facilities, &mines, upkeep);
                rebalance(&mut state.facilities, &refineries, upkeep);

                let Some(mut stock) = state.stock_mut(side).map(std::mem::take) else {
                    continue;
                };
                let mut day = Day {
                    world,
                    tick: tick_event.tick,
                    step_percent,
                    free_raw: !state.started,
                    rolls: &mut rolls,
                    events: &mut events,
                };
                let walk = mines
                    .iter()
                    .map(|&key| (key, true))
                    .chain(refineries.iter().map(|&key| (key, false)));
                for (key, is_mine) in walk {
                    let mut cycle = state.facilities.get(&key).copied().unwrap_or_default();
                    match cycle.phase {
                        Phase::Inactive => {
                            // `FUN_0053b1d0`: entering state 1 from 4 sets
                            // bit 4.
                            cycle.randomise = true;
                            day.enter_idle(key, is_mine, &mut cycle, &mut stock);
                        }
                        Phase::Processing { done_tick } if day.tick >= done_tick => {
                            if is_mine {
                                stock.raw += 1;
                                day.events.push(ProductionEvent::RawMined {
                                    side,
                                    facility: key,
                                });
                            } else {
                                stock.refined += 1;
                                day.events.push(ProductionEvent::MaterialRefined {
                                    side,
                                    facility: key,
                                });
                            }
                            day.enter_idle(key, is_mine, &mut cycle, &mut stock);
                        }
                        Phase::Idle | Phase::Processing { .. } => {}
                    }
                    state.facilities.insert(key, cycle);
                    if is_mine {
                        day.serve_requests(&mut state.facilities, &mut stock);
                    }
                }
                if let Some(slot) = state.stock_mut(side) {
                    *slot = stock;
                }
            }
            state.started = true;
        }
        events
    }
}

/// The systems whose yards, mines and refineries are Suspended:
/// `FUN_0050bbb0` sets the bit from system `+0x88 & 0x24`, uprising or
/// blockade.
#[must_use]
pub fn suspended_systems(blockade: &BlockadeState, uprising: &UprisingState) -> HashSet<SystemKey> {
    blockade
        .blockaded_systems()
        .iter()
        .chain(uprising.active_uprisings.keys())
        .copied()
        .collect()
}

/// `FUN_00529dd0`: a manager with no yard of its area and side scraps its
/// queue (`FUN_0052a2c0`). A Suspended yard still counts as one. Returns
/// the areas scrapped.
///
/// port: the queued units are dropped; the original destroys each product
/// object with cause `0x16`.
pub fn scrap_yardless_queues(
    world: &GameWorld,
    manufacturing: &mut ManufacturingState,
) -> Vec<(SystemKey, ProductionArea)> {
    let mut areas: Vec<_> = manufacturing
        .queues()
        .iter()
        .filter(|(_, queue)| !queue.is_empty())
        .map(|(&key, _)| key)
        .collect();
    areas.sort_unstable();
    areas.retain(|&(system, area)| {
        let Some(value) = world.systems.get(system) else {
            return true;
        };
        // While contested the yards keep their last holder's side, and so
        // does the manager.
        let Some(side) = GameWorld::facility_holder(value.control) else {
            return false;
        };
        !value
            .manufacturing_facilities
            .iter()
            .filter_map(|&key| world.manufacturing_facilities.get(key))
            .any(|yard| yard.side == side && yard.class_dat_id.family() == area_family(area))
    });
    for &(system, area) in &areas {
        manufacturing.stop(system, area);
    }
    areas
}

/// One manager's yard: its key and class rate (`FUN_0052cc90`).
#[derive(Clone, Copy)]
struct Yard {
    key: ManufacturingFacilityKey,
    rate: u32,
}

/// The yards of a side and their refined material, for one day.
struct YardDay<'a> {
    yards: &'a mut HashMap<ManufacturingFacilityKey, YardCycle>,
    stock: &'a mut SideStock,
    tick: u64,
}

impl YardDay<'_> {
    fn phase(&self, key: ManufacturingFacilityKey) -> Phase {
        self.yards
            .get(&key)
            .map_or(Phase::Inactive, |yard| yard.phase)
    }

    /// `FUN_0053a860(1)`: start a cycle of the class rate from day `from`.
    fn begin(yard: &mut YardCycle, rate: u32, from: u64) {
        yard.cycle += 1;
        yard.phase = Phase::Processing {
            done_tick: from + u64::from(rate),
        };
    }

    /// State 1 and its cycle-start hook (`FUN_00530820`): take one refined
    /// material (`FUN_0052fb80`) or queue a request (`FUN_00533610`).
    fn enter_idle(&mut self, yard: Yard, from: u64) {
        let cycle = self.yards.entry(yard.key).or_default();
        cycle.phase = Phase::Idle;
        cycle.cycle += 1;
        if self.stock.refined > 0 {
            self.stock.refined -= 1;
            Self::begin(cycle, yard.rate, from);
        } else {
            self.stock.refined_requests.push_back(RefinedRequest {
                yard: yard.key,
                cycle: cycle.cycle,
            });
        }
    }

    /// State 4. A yard put there in a cycle does not get its refined
    /// material back.
    fn sleep(&mut self, key: ManufacturingFacilityKey) {
        if let Some(yard) = self.yards.get_mut(&key) {
            if yard.phase == Phase::Idle {
                yard.cycle += 1;
            }
            yard.phase = Phase::Inactive;
        }
    }

    /// `FUN_00529dd0`: keep as many yards out of state 4 as the manager
    /// has points left to make. Inactive yards wake fastest first
    /// (`FUN_0053aac0`); running ones stop slowest first, a waiting yard
    /// before one in a cycle before one of `ending`, whose cycle has just
    /// ended (`FUN_0053ab10`). A woken yard counts its cycle from day
    /// `from`.
    fn regulate(
        &mut self,
        yards: &[Yard],
        needed: u32,
        ending: &[ManufacturingFacilityKey],
        from: u64,
    ) {
        let needed = usize::try_from(needed).unwrap_or(usize::MAX);
        loop {
            let active = yards
                .iter()
                .filter(|yard| self.phase(yard.key) != Phase::Inactive)
                .count();
            if active < needed {
                let Some(&fastest) = yards
                    .iter()
                    .filter(|yard| self.phase(yard.key) == Phase::Inactive)
                    .min_by_key(|yard| yard.rate)
                else {
                    return;
                };
                self.enter_idle(fastest, from);
            } else if active > needed {
                let Some(&slowest) = yards
                    .iter()
                    .filter(|yard| self.phase(yard.key) != Phase::Inactive)
                    .min_by_key(|yard| {
                        let state = if ending.contains(&yard.key) {
                            2
                        } else {
                            u8::from(self.phase(yard.key) != Phase::Idle)
                        };
                        (state, std::cmp::Reverse(yard.rate))
                    })
                else {
                    return;
                };
                self.sleep(slowest.key);
            } else {
                return;
            }
        }
    }

    /// `FUN_0052fd30`: hand refined material to the waiting yards, first
    /// come first served. A request whose yard has left the state it asked
    /// in is dropped.
    fn serve_requests(&mut self, world: &GameWorld) {
        while self.stock.refined > 0 {
            let Some(request) = self.stock.refined_requests.pop_front() else {
                break;
            };
            let Some(cycle) = self.yards.get_mut(&request.yard) else {
                continue;
            };
            if cycle.phase == Phase::Idle && cycle.cycle == request.cycle {
                self.stock.refined -= 1;
                Self::begin(cycle, yard_rate(world, request.yard), self.tick);
            }
        }
    }
}

/// A yard's class rate (`FUN_00520b70`); 0 without a class record.
fn yard_rate(world: &GameWorld, key: ManufacturingFacilityKey) -> u32 {
    world
        .manufacturing_facilities
        .get(key)
        .and_then(|yard| world.buildable_classes.get(&yard.class_dat_id))
        .map_or(0, |class| class.processing_rate)
}

/// Run the yards for one day per tick event and return what they
/// complete.
///
/// A yard takes one refined material at each cycle start (`FUN_00530820`)
/// and adds one build point to its manager at the end (`FUN_00530950`,
/// `FUN_0052a430`). Its cycle is the class rate (`FUN_0052cc90`). After
/// every point and every order the manager sets how many yards run
/// (`FUN_00529dd0`). The yards of a `blocked` system are Suspended and do
/// not run (`FUN_0050bbb0`).
///
/// port: the original acts on each event as it happens; this walks the
/// managers once a day, in system then area order. Every cycle of a
/// manager that ends on a day adds its point before the manager sets its
/// yards, which keeps Build Selection's completion day (`FUN_00528d30`);
/// the original's order among timers of one day is not traced. A yard
/// woken by an order counts its cycle from the day before, when the order
/// was given. A yard whose class has no rate never ends a cycle, so it is
/// left out.
pub fn advance_yards(
    state: &mut ProductionState,
    world: &GameWorld,
    manufacturing: &mut ManufacturingState,
    blocked: impl Fn(SystemKey) -> bool,
    tick_events: &[TickEvent],
) -> ManufacturingAdvance {
    let mut advance = ManufacturingAdvance::default();
    state
        .yards
        .retain(|key, _| world.manufacturing_facilities.contains_key(*key));
    let mut busy: Vec<SystemKey> = manufacturing
        .queues()
        .iter()
        .filter(|(_, queue)| !queue.is_empty())
        .map(|(&(system, _), _)| system)
        .collect();
    busy.sort_unstable();
    busy.dedup();

    for tick_event in tick_events {
        let tick = tick_event.tick;
        for (side, stock) in [
            (Faction::Alliance, &mut state.alliance),
            (Faction::Empire, &mut state.empire),
        ] {
            let mut day = YardDay {
                yards: &mut state.yards,
                stock,
                tick,
            };
            day.serve_requests(world);

            for (system, value) in &world.systems {
                for area in ProductionArea::ALL {
                    // While contested the yards keep their last holder's
                    // side, and so does the manager.
                    let yards: Vec<Yard> = value
                        .manufacturing_facilities
                        .iter()
                        .filter_map(|&key| Some((key, world.manufacturing_facilities.get(key)?)))
                        .filter(|(_, yard)| {
                            yard.side == side && yard.class_dat_id.family() == area_family(area)
                        })
                        .map(|(key, _)| Yard {
                            key,
                            rate: yard_rate(world, key),
                        })
                        .filter(|yard| yard.rate > 0)
                        .collect();
                    if GameWorld::facility_holder(value.control)
                        .is_some_and(|holder| holder != side)
                        || blocked(system)
                    {
                        for yard in &yards {
                            day.sleep(yard.key);
                        }
                        continue;
                    }
                    let ending: Vec<Yard> = yards
                        .iter()
                        .copied()
                        .filter(|yard| {
                            matches!(
                                day.phase(yard.key),
                                Phase::Processing { done_tick } if tick >= done_tick
                            )
                        })
                        .collect();
                    for _ in &ending {
                        let (completions, departures) = manufacturing.add_point(system, area, tick);
                        advance.completions.extend(completions);
                        advance.departures.extend(departures);
                    }
                    let needed = manufacturing
                        .queue(system, area)
                        .map_or(0, crate::manufacturing::ProductionQueue::points_needed);
                    let ended: Vec<_> = ending.iter().map(|yard| yard.key).collect();
                    day.regulate(&yards, needed, &ended, tick.saturating_sub(1));
                    for yard in ending {
                        if day.phase(yard.key) != Phase::Inactive {
                            day.enter_idle(yard, tick);
                        }
                    }
                }
            }
        }
    }
    advance.newly_idle = busy
        .into_iter()
        .filter(|&system| manufacturing.queued_at(system) == 0)
        .collect();
    advance
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::ExplorationStatus;
    use crate::ids::DatId;
    use crate::world::{
        BuildableClass, ControlKind, GnprtbEntry, GnprtbParams, ProductionFacilityInstance, System,
        TroopUnit,
    };

    const MINE: u32 = 0x2c00_0001;
    const REFINERY: u32 = 0x2d00_0002;

    /// A world with GNPRTB 0x1000 = 20 and the PROFACSD processing rate 5.
    fn world() -> GameWorld {
        let gnprtb = GnprtbParams::new(vec![GnprtbEntry {
            parameter_id: u32::from(CYCLE_STEP_PERCENT_PARAM),
            development: 20,
            alliance_sp_easy: 20,
            alliance_sp_medium: 20,
            alliance_sp_hard: 20,
            empire_sp_easy: 20,
            empire_sp_medium: 20,
            empire_sp_hard: 20,
            multiplayer: 20,
        }]);
        let mut world = GameWorld {
            gnprtb,
            ..GameWorld::default()
        };
        for id in [MINE, REFINERY] {
            world.buildable_classes.insert(
                DatId::new(id),
                BuildableClass {
                    processing_rate: 5,
                    ..BuildableClass::default()
                },
            );
        }
        world
    }

    fn system(world: &mut GameWorld) -> SystemKey {
        world.systems.insert(System {
            dat_id: DatId::new(0x9000_0001),
            name: "Here".into(),
            sector: crate::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 10,
            raw_materials: 5,
            espionage_rating: 0.0,
            fleets: Vec::new(),
            ground_units: Vec::new(),
            special_forces: Vec::new(),
            defense_facilities: Vec::new(),
            manufacturing_facilities: Vec::new(),
            production_facilities: Vec::new(),
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Alliance),
        })
    }

    fn facility(world: &mut GameWorld, at: SystemKey, id: u32) -> ProductionFacilityKey {
        let key = world
            .production_facilities
            .insert(ProductionFacilityInstance {
                class_dat_id: DatId::new(id),
                side: Faction::Alliance,
                is_mine: id == MINE,
            });
        world.systems[at].production_facilities.push(key);
        key
    }

    /// Give the Alliance `upkeep` points of regiment upkeep.
    fn upkeep(world: &mut GameWorld, upkeep: u32) {
        let class = DatId::new(0x1000_0001);
        world.buildable_classes.insert(
            class,
            BuildableClass {
                maintenance_cost: upkeep,
                ..BuildableClass::default()
            },
        );
        world.troops.insert(TroopUnit {
            class_dat_id: class,
            is_alliance: true,
            regiment_strength: 1,
        });
    }

    /// Advance through `days` (inclusive) with every roll at `roll`.
    fn run(
        state: &mut ProductionState,
        world: &GameWorld,
        days: std::ops::RangeInclusive<u64>,
        roll: f64,
    ) -> Vec<ProductionEvent> {
        let mut events = Vec::new();
        for tick in days {
            events.extend(ProductionSystem::advance(
                state,
                world,
                &ManufacturingState::new(),
                |_| false,
                &[TickEvent { tick }],
                &[roll; 16],
            ));
        }
        events
    }

    fn alliance(state: &ProductionState) -> (i32, i32) {
        let stock = state.stock(Faction::Alliance).expect("alliance stock");
        (stock.raw, stock.refined)
    }

    #[test]
    fn a_cycle_lasts_the_class_rate_plus_a_day_per_ten_points_allocated() {
        // FUN_0055e2b0 with GNPRTB 0x1000 = 20 and capacity 50: step 10.
        assert_eq!(cycle_period(5, 0, 20, 100), 5);
        assert_eq!(cycle_period(5, 1, 20, 100), 6);
        assert_eq!(cycle_period(5, 10, 20, 100), 6);
        assert_eq!(cycle_period(5, 11, 20, 100), 7);
        assert_eq!(cycle_period(5, 50, 20, 100), 10);
        // The modifier is a percentage; a step never falls below 1.
        assert_eq!(cycle_period(5, 50, 20, 50), 5);
        assert_eq!(cycle_period(4, 3, 0, 100), 7);
    }

    #[test]
    fn a_randomised_cycle_lasts_half_to_one_and_a_half_periods() {
        // FUN_0055e2b0: FUN_0053e290(period) + period / 2, and
        // FUN_005f5700(period + 1) yields 0..=period.
        assert_eq!(randomised_period(5, 0.0), 2);
        assert_eq!(randomised_period(5, 0.5), 5);
        assert_eq!(randomised_period(5, 0.999_999), 7);
        assert_eq!(randomised_period(10, 0.999_999), 15);
    }

    #[test]
    fn both_stockpiles_open_at_zero() {
        // FUN_00533300 zeroes side +0x78 and +0x7c, and no seeder sets them.
        assert_eq!(alliance(&ProductionState::new()), (0, 0));
        assert_eq!(
            ProductionState::new().stock(Faction::Empire),
            Some(&SideStock::default())
        );
        assert_eq!(ProductionState::new().stock(Faction::Neutral), None);
    }

    #[test]
    fn a_mine_adds_one_raw_material_at_the_end_of_each_cycle() {
        // FUN_00530670: raw +0x78 += 1. The first cycle after activation is
        // randomised; a roll of 0.5 makes it the plain 5 days.
        let mut world = world();
        let here = system(&mut world);
        let mine = facility(&mut world, here, MINE);
        let mut state = ProductionState::new();

        assert!(run(&mut state, &world, 1..=5, 0.5).is_empty());
        assert!(state.is_processing(mine));
        assert_eq!(alliance(&state), (0, 0));

        let events = run(&mut state, &world, 6..=6, 0.5);
        assert_eq!(
            events,
            vec![ProductionEvent::RawMined {
                side: Faction::Alliance,
                facility: mine
            }]
        );
        assert_eq!(alliance(&state), (1, 0));

        // The next cycles are not randomised: a low roll changes nothing.
        run(&mut state, &world, 7..=16, 0.0);
        assert_eq!(alliance(&state), (3, 0));
    }

    #[test]
    fn the_first_cycle_after_activation_is_randomised() {
        // FUN_0053b1d0 sets +0x60 bit 4 on entering state 1 from state 4.
        let mut world = world();
        let here = system(&mut world);
        facility(&mut world, here, MINE);

        let mut early = ProductionState::new();
        run(&mut early, &world, 1..=3, 0.0);
        assert_eq!(alliance(&early), (1, 0), "2-day first cycle");

        let mut late = ProductionState::new();
        run(&mut late, &world, 1..=7, 0.999_999);
        assert_eq!(alliance(&late), (0, 0));
        run(&mut late, &world, 8..=8, 0.999_999);
        assert_eq!(alliance(&late), (1, 0), "7-day first cycle");
    }

    #[test]
    fn a_refinery_turns_one_raw_material_into_one_refined() {
        // FUN_005306b0 takes one raw at the cycle start (FUN_0052fb30);
        // FUN_005307e0 adds one refined at its end. Setup starts every
        // refinery without raw material (DAT_006b90e0 clear).
        let mut world = world();
        let here = system(&mut world);
        facility(&mut world, here, MINE);
        let refinery = facility(&mut world, here, REFINERY);
        let mut state = ProductionState::new();

        // Day 6: the mine's raw goes straight to the refinery, whose free
        // first cycle has just ended.
        let events = run(&mut state, &world, 1..=6, 0.5);
        assert_eq!(events.len(), 2);
        assert_eq!(alliance(&state), (0, 1));
        assert!(state.is_processing(refinery));

        run(&mut state, &world, 7..=11, 0.5);
        assert_eq!(alliance(&state), (0, 2));
    }

    #[test]
    fn a_refinery_without_raw_material_waits_and_is_served_in_order() {
        // FUN_005306b0 queues a request; FUN_0052fbf0 pops the head when
        // raw arrives. Two refineries and one mine: after the free first
        // cycles the refineries take turns, one per unit of raw.
        let mut world = world();
        let here = system(&mut world);
        facility(&mut world, here, MINE);
        let first = facility(&mut world, here, REFINERY);
        let second = facility(&mut world, here, REFINERY);
        let third = facility(&mut world, here, REFINERY);
        let mut state = ProductionState::new();
        let waiting = |state: &ProductionState| {
            state
                .stock(Faction::Alliance)
                .map(SideStock::waiting_for_raw)
        };
        assert_eq!(waiting(&state), Some(0));

        run(&mut state, &world, 1..=6, 0.5);
        assert_eq!(alliance(&state), (0, 3), "three free first cycles");
        assert!(state.is_processing(first), "the day's one unit is taken");
        assert!(!state.is_processing(second));
        assert!(!state.is_processing(third));
        assert_eq!(waiting(&state), Some(2));

        // Day 11: the mine's second unit goes to the second refinery, which
        // has waited longest; the first finishes and queues behind it.
        run(&mut state, &world, 7..=11, 0.5);
        assert_eq!(alliance(&state), (0, 4));
        assert!(state.is_processing(second));
        assert!(!state.is_processing(first));
        assert!(!state.is_processing(third), "one unit serves one refinery");
        assert_eq!(waiting(&state), Some(2));
    }

    #[test]
    fn upkeep_is_spread_across_a_pool_in_proportion() {
        // FUN_0052f6b0 with FUN_0055a820: each pass gives every mine up to
        // its share plus one, so 70 points over three mines settle at
        // 24, 23 and 23.
        let mut world = world();
        let here = system(&mut world);
        let mines = [
            facility(&mut world, here, MINE),
            facility(&mut world, here, MINE),
            facility(&mut world, here, MINE),
        ];
        let refinery = facility(&mut world, here, REFINERY);
        upkeep(&mut world, 70);
        let mut state = ProductionState::new();

        run(&mut state, &world, 1..=1, 0.5);
        assert_eq!(mines.map(|key| state.allocated(key)), [24, 23, 23]);
        // The refinery pool holds 50, so it is filled and no more.
        assert_eq!(state.allocated(refinery), 50);
    }

    #[test]
    fn allocation_shrinks_when_upkeep_falls() {
        // FUN_0055a960: n = allocated - (pool - 1) * capacity / pool
        // capacity, taken facility by facility until the pool matches.
        let mut world = world();
        let here = system(&mut world);
        let mines = [
            facility(&mut world, here, MINE),
            facility(&mut world, here, MINE),
        ];
        upkeep(&mut world, 60);
        let mut state = ProductionState::new();
        run(&mut state, &world, 1..=1, 0.5);
        assert_eq!(mines.map(|key| state.allocated(key)), [30, 30]);

        world.troops.clear();
        upkeep(&mut world, 21);
        run(&mut state, &world, 2..=2, 0.5);
        // 39 points leave one at a time, the first mine first.
        assert_eq!(mines.map(|key| state.allocated(key)), [10, 11]);
    }

    #[test]
    fn a_fully_allocated_mine_takes_ten_days() {
        // FUN_0055e2b0: rate 5 plus ceil(50 / 10).
        let mut world = world();
        let here = system(&mut world);
        facility(&mut world, here, MINE);
        upkeep(&mut world, 50);
        let mut state = ProductionState::new();

        // The first cycle is randomised around 10: 10 at a roll of 0.5
        // (5 + 10 / 2).
        run(&mut state, &world, 1..=10, 0.5);
        assert_eq!(alliance(&state), (0, 0));
        run(&mut state, &world, 11..=11, 0.5);
        assert_eq!(alliance(&state), (1, 0));
        run(&mut state, &world, 12..=21, 0.5);
        assert_eq!(alliance(&state), (2, 0));
    }

    #[test]
    fn a_blocked_facility_stops_and_gives_up_its_allocation() {
        // FUN_0050bbb0 sets +0x60 bit 0 in a system in uprising or
        // blockaded; FUN_0055ab60 zeroes the allocated half and
        // FUN_0053aa40 moves the facility to state 4.
        let mut world = world();
        let here = system(&mut world);
        let mine = facility(&mut world, here, MINE);
        upkeep(&mut world, 30);
        let mut state = ProductionState::new();
        run(&mut state, &world, 1..=2, 0.5);
        assert_eq!(state.allocated(mine), 30);
        assert!(state.is_processing(mine));

        let blocked = |state: &mut ProductionState, tick| {
            ProductionSystem::advance(
                state,
                &world,
                &ManufacturingState::new(),
                |system| system == here,
                &[TickEvent { tick }],
                &[0.5; 4],
            )
        };
        for tick in 3..=20 {
            assert!(blocked(&mut state, tick).is_empty());
        }
        assert_eq!(state.allocated(mine), 0);
        assert!(!state.is_processing(mine));
        assert_eq!(alliance(&state), (0, 0));

        // Released, it is allocated 30 again (period 8) and starts with a
        // randomised first cycle: 8 / 2 days at a roll of 0.
        run(&mut state, &world, 21..=24, 0.0);
        assert_eq!(state.allocated(mine), 30);
        assert_eq!(alliance(&state), (0, 0));
        run(&mut state, &world, 25..=25, 0.0);
        assert_eq!(alliance(&state), (1, 0));
    }

    #[test]
    fn a_removed_facility_is_forgotten_and_its_request_dropped() {
        // FUN_0052fbf0 drops a request whose facility no longer exists.
        let mut world = world();
        let here = system(&mut world);
        facility(&mut world, here, MINE);
        let refinery = facility(&mut world, here, REFINERY);
        let doomed = facility(&mut world, here, REFINERY);
        let mut state = ProductionState::new();
        run(&mut state, &world, 1..=6, 0.5);
        assert!(!state.is_processing(doomed), "waiting for raw");

        world.production_facilities.remove(doomed);
        world.systems[here]
            .production_facilities
            .retain(|&key| key != doomed);
        run(&mut state, &world, 7..=11, 0.5);
        assert_eq!(state.allocated(doomed), 0);
        // The surviving refinery got the raw the dead one had asked for.
        assert!(state.is_processing(refinery));
        assert_eq!(alliance(&state), (0, 3));
    }

    #[test]
    fn each_activation_draws_its_own_roll() {
        // FUN_0055e2b0 calls FUN_0053e290 once per randomised period.
        let mut world = world();
        let here = system(&mut world);
        let early = facility(&mut world, here, MINE);
        let late = facility(&mut world, here, MINE);
        let mut state = ProductionState::new();
        let day = |state: &mut ProductionState, tick| {
            ProductionSystem::advance(
                state,
                &world,
                &ManufacturingState::new(),
                |_| false,
                &[TickEvent { tick }],
                &[0.0, 0.999_999],
            )
        };
        let mut mined = Vec::new();
        for tick in 1..=8 {
            for event in day(&mut state, tick) {
                if let ProductionEvent::RawMined { facility, .. } = event {
                    mined.push((tick, facility));
                }
            }
        }
        // 2 and 7 days after day 1; the early mine's second cycle is the
        // plain 5 days and ends on day 8 too.
        assert_eq!(mined, vec![(3, early), (8, early), (8, late)]);
    }

    #[test]
    fn a_request_from_before_a_block_is_dropped() {
        // FUN_0052fbf0 serves a request only while its refinery is in
        // state 1 with the cycle counter it asked with (FUN_0053a6e0).
        let mut world = world();
        let mines = system(&mut world);
        let refineries = system(&mut world);
        facility(&mut world, mines, MINE);
        let first = facility(&mut world, refineries, REFINERY);
        let second = facility(&mut world, refineries, REFINERY);
        let mut state = ProductionState::new();
        let day = |state: &mut ProductionState, tick, blocked: bool| {
            ProductionSystem::advance(
                state,
                &world,
                &ManufacturingState::new(),
                |system| blocked && system == refineries,
                &[TickEvent { tick }],
                &[0.5; 8],
            );
        };
        for tick in 1..=6 {
            day(&mut state, tick, false);
        }
        assert!(state.is_processing(first));
        assert!(!state.is_processing(second), "second has a request queued");

        // A block stops both; released, each asks again, first then second.
        day(&mut state, 7, true);
        assert!(!state.is_processing(first));
        day(&mut state, 8, false);
        for tick in 9..=11 {
            day(&mut state, tick, false);
        }
        // Day 11's unit skips the second refinery's old request and goes to
        // the first, which is now ahead of it.
        assert!(state.is_processing(first));
        assert!(!state.is_processing(second));
        assert_eq!(alliance(&state), (0, 2));
        assert_eq!(
            state
                .stock(Faction::Alliance)
                .map(SideStock::waiting_for_raw),
            Some(1)
        );
    }

    #[test]
    fn a_shrinking_pool_takes_first_from_the_facility_above_its_share() {
        // FUN_0055a960: n = allocated - (pool - 1) * capacity / pool
        // capacity. A mine built beside a full one starts at 0 and stays
        // there while the pool only shrinks.
        let mut world = world();
        let here = system(&mut world);
        let full = facility(&mut world, here, MINE);
        upkeep(&mut world, 50);
        let mut state = ProductionState::new();
        run(&mut state, &world, 1..=1, 0.5);
        assert_eq!(state.allocated(full), 50);

        let fresh = facility(&mut world, here, MINE);
        run(&mut state, &world, 2..=2, 0.5);
        assert_eq!(
            [state.allocated(full), state.allocated(fresh)],
            [50, 0],
            "upkeep 50 is already met"
        );

        world.troops.clear();
        upkeep(&mut world, 10);
        run(&mut state, &world, 3..=3, 0.5);
        assert_eq!([state.allocated(full), state.allocated(fresh)], [10, 0]);
    }

    const SHIPYARD: u32 = 0x2800_0001;
    const ADVANCED_SHIPYARD: u32 = 0x2800_0004;
    const TRAINING_FACILITY: u32 = 0x2900_0001;

    /// Add an Alliance yard of class `id` and cycle `rate` at `here`.
    fn yard(
        world: &mut GameWorld,
        here: SystemKey,
        id: u32,
        rate: u32,
    ) -> ManufacturingFacilityKey {
        use crate::world::ManufacturingFacilityInstance;
        world.buildable_classes.insert(
            DatId::new(id),
            BuildableClass {
                processing_rate: rate,
                ..BuildableClass::default()
            },
        );
        let key = world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: DatId::new(id),
                side: Faction::Alliance,
                is_shipyard: id >> 24 == 0x28,
            });
        world.systems[here].manufacturing_facilities.push(key);
        key
    }

    /// A ship needing `cost` build points.
    fn ship(world: &mut GameWorld, cost: u32) -> crate::manufacturing::QueueItem {
        use crate::manufacturing::{BuildableKind, QueueItem};
        use crate::world::CapitalShipClass;
        let class = world
            .capital_ship_classes
            .insert(CapitalShipClass::default());
        QueueItem::new(BuildableKind::CapitalShip(class), cost)
    }

    /// A new Alliance system with a shipyard (MANFACSD rate 4) and an
    /// advanced shipyard (rate 2), and a ship of `cost` points ordered
    /// there. Returns the system, the queues, and the slow and fast yards.
    fn shipyard_system(
        world: &mut GameWorld,
        cost: u32,
    ) -> (
        SystemKey,
        ManufacturingState,
        ManufacturingFacilityKey,
        ManufacturingFacilityKey,
    ) {
        let here = system(world);
        let slow = yard(world, here, SHIPYARD, 4);
        let fast = yard(world, here, ADVANCED_SHIPYARD, 2);
        let mut manufacturing = ManufacturingState::new();
        manufacturing.enqueue(here, ship(world, cost));
        (here, manufacturing, slow, fast)
    }

    fn yards_day(
        state: &mut ProductionState,
        world: &GameWorld,
        manufacturing: &mut ManufacturingState,
        tick: u64,
    ) -> ManufacturingAdvance {
        advance_yards(
            state,
            world,
            manufacturing,
            |_| false,
            &[TickEvent { tick }],
        )
    }

    fn stocked(refined: i32) -> ProductionState {
        let mut state = ProductionState::new();
        state.alliance.refined = refined;
        state
    }

    #[test]
    fn a_yard_takes_one_refined_material_each_time_its_cycle_starts() {
        // FUN_00530820 takes one refined (FUN_0052fb80) at the cycle start;
        // FUN_0052cc90 sets the cycle to the class rate, here 4 and 2 days;
        // FUN_00530950 ends it with one point for the manager.
        let mut world = world();
        let (here, mut manufacturing, _, _) = shipyard_system(&mut world, 20);
        let mut state = stocked(10);

        let mut left = Vec::new();
        for tick in 1..=8 {
            let advance = yards_day(&mut state, &world, &mut manufacturing, tick);
            assert_eq!(advance, ManufacturingAdvance::default());
            left.push(alliance(&state).1);
        }
        // Both start with the order; the fast yard starts again on days 2,
        // 4, 6 and 8, the slow one on days 4 and 8.
        assert_eq!(left, [8, 7, 7, 5, 5, 4, 4, 2]);
        // Cycles ended: the fast yard's on days 2, 4, 6 and 8, the slow
        // one's on days 4 and 8.
        let queue = manufacturing.queue(here, ProductionArea::Shipyard);
        assert_eq!(queue.map(|queue| queue.completed_points()), Some(6));
    }

    #[test]
    fn an_order_completes_on_its_best_day_for_its_cost_in_refined_material() {
        // FUN_00528d30: two yards of periods 4 and 2 make 6 points by day
        // 8. FUN_00529dd0 stops the yards once no point is needed, so the
        // order draws its refined cost and no more.
        use crate::manufacturing::completion_day;
        let mut world = world();
        let (here, mut manufacturing, slow, fast) = shipyard_system(&mut world, 6);
        let mut state = stocked(10);
        assert_eq!(completion_day(&[4, 2], 6), Some(8));

        for tick in 1..=7 {
            let advance = yards_day(&mut state, &world, &mut manufacturing, tick);
            assert_eq!(advance, ManufacturingAdvance::default(), "day {tick}");
        }
        let advance = yards_day(&mut state, &world, &mut manufacturing, 8);

        assert_eq!(advance.completions.len(), 1);
        assert_eq!(advance.completions[0].system, here);
        assert_eq!(advance.completions[0].tick, 8);
        assert_eq!(advance.newly_idle, [here]);
        assert_eq!(alliance(&state), (0, 4));
        for key in [slow, fast] {
            assert!(!state.yard_is_processing(key) && !state.yard_is_waiting(key));
        }
        // Nothing queued: nothing starts, and the system is not idle anew.
        let after = yards_day(&mut state, &world, &mut manufacturing, 9);
        assert_eq!(after, ManufacturingAdvance::default());
        assert_eq!(alliance(&state), (0, 4));
    }

    #[test]
    fn a_manager_wakes_no_more_yards_than_it_needs_points_fastest_first() {
        // FUN_00529dd0 wakes yards while fewer run than points are needed;
        // FUN_0053aac0 picks the fastest class rate.
        let mut world = world();
        let (here, mut manufacturing, slow, fast) = shipyard_system(&mut world, 1);
        let mut state = stocked(10);

        let advance = advance_yards(
            &mut state,
            &world,
            &mut manufacturing,
            |_| false,
            &[TickEvent { tick: 1 }],
        );
        assert!(advance.completions.is_empty());
        assert!(state.yard_is_processing(fast));
        assert!(!state.yard_is_processing(slow) && !state.yard_is_waiting(slow));
        assert_eq!(alliance(&state), (0, 9));

        // The order was given on day 0, so the 2-day cycle ends on day 2.
        let advance = yards_day(&mut state, &world, &mut manufacturing, 2);
        assert_eq!(advance.completions.len(), 1);
        assert_eq!(advance.completions[0].tick, 2);
        assert_eq!(manufacturing.queued_at(here), 0);
    }

    #[test]
    fn several_days_in_one_call_run_one_after_another() {
        let mut world = world();
        let (_, mut manufacturing, _, _) = shipyard_system(&mut world, 1);
        let mut state = stocked(10);

        let advance = advance_yards(
            &mut state,
            &world,
            &mut manufacturing,
            |_| false,
            &[TickEvent { tick: 1 }, TickEvent { tick: 2 }],
        );

        assert_eq!(advance.completions.len(), 1);
        assert_eq!(advance.completions[0].tick, 2);
    }

    #[test]
    fn a_surplus_yard_stops_slowest_first() {
        // FUN_00529dd0 and FUN_0053ab10: with more yards running than
        // points needed, the slowest stops. Four points: the fast yard
        // makes one on day 2, both make one on day 4, and one is left.
        let mut world = world();
        let (_, mut manufacturing, slow, fast) = shipyard_system(&mut world, 4);
        let mut state = stocked(10);

        for tick in 1..=4 {
            yards_day(&mut state, &world, &mut manufacturing, tick);
        }
        assert!(state.yard_is_processing(fast));
        assert!(!state.yard_is_processing(slow) && !state.yard_is_waiting(slow));

        assert!(yards_day(&mut state, &world, &mut manufacturing, 5)
            .completions
            .is_empty());
        let advance = yards_day(&mut state, &world, &mut manufacturing, 6);
        assert_eq!(advance.completions.len(), 1);
        assert_eq!(alliance(&state), (0, 6));
    }

    #[test]
    fn a_waiting_yard_stops_before_one_that_has_just_ended_its_cycle() {
        // FUN_0053ab10 stops a yard in state 1 before one in state 2 or 3.
        // FUN_00533610 queues a yard that finds no refined material, and
        // FUN_0052fd30 serves the queue, dropping a request whose yard has
        // left the state it asked in.
        let mut world = world();
        let (_, mut manufacturing, slow, fast) = shipyard_system(&mut world, 2);
        let mut state = stocked(1);

        yards_day(&mut state, &world, &mut manufacturing, 1);
        assert!(state.yard_is_processing(fast));
        assert!(state.yard_is_waiting(slow));
        assert_eq!(state.alliance.waiting_for_refined(), 1);
        assert_eq!(alliance(&state), (0, 0));

        // Day 2: one point made, one needed, two yards awake.
        yards_day(&mut state, &world, &mut manufacturing, 2);
        assert!(!state.yard_is_waiting(slow) && !state.yard_is_processing(slow));
        assert!(state.yard_is_waiting(fast));

        // One refined material arrives: the slow yard's request is dropped
        // and the fast yard's is served.
        state.alliance.refined = 1;
        yards_day(&mut state, &world, &mut manufacturing, 3);
        assert!(state.yard_is_processing(fast));
        assert!(!state.yard_is_processing(slow));
        assert_eq!(alliance(&state), (0, 0));
        assert_eq!(state.alliance.waiting_for_refined(), 0);
        assert!(yards_day(&mut state, &world, &mut manufacturing, 4)
            .completions
            .is_empty());
        let advance = yards_day(&mut state, &world, &mut manufacturing, 5);
        assert_eq!(advance.completions.len(), 1);
    }

    #[test]
    fn a_waiting_yard_stops_before_one_in_its_cycle() {
        // FUN_0053ab10 stops a yard in state 1 before one in state 2,
        // whatever their rates.
        let mut world = world();
        let (here, mut manufacturing, slow, fast) = shipyard_system(&mut world, 2);
        manufacturing.enqueue(here, ship(&mut world, 1));
        let mut state = stocked(2);

        // Both start; on day 2 the fast yard ends its cycle and finds no
        // refined material for the next, while the slow one runs to day 4.
        yards_day(&mut state, &world, &mut manufacturing, 1);
        yards_day(&mut state, &world, &mut manufacturing, 2);
        assert!(state.yard_is_waiting(fast));
        assert!(state.yard_is_processing(slow));

        // The second ship is dropped: one point is needed, two yards are
        // awake.
        manufacturing
            .queue_mut(here, ProductionArea::Shipyard)
            .cancel(1);
        yards_day(&mut state, &world, &mut manufacturing, 3);

        assert!(!state.yard_is_waiting(fast) && !state.yard_is_processing(fast));
        assert!(state.yard_is_processing(slow));
        let advance = yards_day(&mut state, &world, &mut manufacturing, 4);
        assert_eq!(advance.completions.len(), 1);
    }

    #[test]
    fn a_running_yard_stops_before_one_that_has_just_ended_its_cycle() {
        // FUN_0053ab10 stops a yard in state 2 before one in state 3. Two
        // points: the fast yard makes one on day 2 while the slow one is
        // in its cycle; the fast yard goes on and the slow one stops.
        let mut world = world();
        let (_, mut manufacturing, slow, fast) = shipyard_system(&mut world, 2);
        let mut state = stocked(10);

        yards_day(&mut state, &world, &mut manufacturing, 1);
        assert!(state.yard_is_processing(slow) && state.yard_is_processing(fast));
        yards_day(&mut state, &world, &mut manufacturing, 2);

        assert!(!state.yard_is_processing(slow));
        assert!(state.yard_is_processing(fast));
        // The slow yard's refined material is spent.
        assert_eq!(alliance(&state), (0, 7));
        let advance = yards_day(&mut state, &world, &mut manufacturing, 4);
        assert_eq!(advance.completions.len(), 1);
    }

    #[test]
    fn a_request_from_an_earlier_wait_is_dropped() {
        // FUN_0052fd30 serves a request only while the yard's cycle counter
        // (FUN_0053a6e0) is the one it asked with.
        let mut world = world();
        let (here, mut manufacturing, _, fast) = shipyard_system(&mut world, 1);
        let mut state = stocked(0);

        yards_day(&mut state, &world, &mut manufacturing, 1);
        assert!(state.yard_is_waiting(fast));
        // Stop: the yard goes to sleep. A new order wakes it to wait again.
        manufacturing.stop(here, ProductionArea::Shipyard);
        yards_day(&mut state, &world, &mut manufacturing, 2);
        assert!(!state.yard_is_waiting(fast));
        manufacturing.enqueue(here, ship(&mut world, 1));
        yards_day(&mut state, &world, &mut manufacturing, 3);
        assert!(state.yard_is_waiting(fast));
        assert_eq!(state.alliance.waiting_for_refined(), 2);

        state.alliance.refined = 1;
        yards_day(&mut state, &world, &mut manufacturing, 4);

        // The first request is dropped without taking the unit, and the
        // second takes it.
        assert!(state.yard_is_processing(fast));
        assert_eq!(alliance(&state), (0, 0));
        assert_eq!(state.alliance.waiting_for_refined(), 0);
    }

    #[test]
    fn yards_with_nothing_queued_or_in_a_blocked_system_draw_nothing() {
        // FUN_00529dd0 keeps no more yards running than the manager has
        // points to make; FUN_0050bbb0 suspends a blocked system's yards.
        let mut world = world();
        let (here, mut manufacturing, slow, fast) = shipyard_system(&mut world, 20);
        let mut state = stocked(10);

        let mut idle = ManufacturingState::new();
        yards_day(&mut state, &world, &mut idle, 1);
        assert_eq!(alliance(&state), (0, 10));

        let blocked =
            |state: &mut ProductionState, manufacturing: &mut ManufacturingState, tick| {
                advance_yards(
                    state,
                    &world,
                    manufacturing,
                    |system| system == here,
                    &[TickEvent { tick }],
                )
            };
        blocked(&mut state, &mut manufacturing, 1);
        assert_eq!(alliance(&state), (0, 10));

        // Blocked in mid-cycle, the yards stop and their cycles make no
        // point.
        yards_day(&mut state, &world, &mut manufacturing, 2);
        assert!(state.yard_is_processing(slow) && state.yard_is_processing(fast));
        assert_eq!(alliance(&state), (0, 8));
        for tick in 3..=6 {
            let advance = blocked(&mut state, &mut manufacturing, tick);
            assert_eq!(advance, ManufacturingAdvance::default());
        }
        assert!(!state.yard_is_processing(slow) && !state.yard_is_processing(fast));
        assert_eq!(alliance(&state), (0, 8));
        let queue = manufacturing.queue(here, ProductionArea::Shipyard);
        assert_eq!(queue.map(|queue| queue.completed_points()), Some(0));

        // Free again, they start over.
        yards_day(&mut state, &world, &mut manufacturing, 7);
        assert_eq!(alliance(&state), (0, 6));
    }

    #[test]
    fn the_holders_stockpile_pays_for_the_area() {
        // FUN_00530820 runs on the yard's side object (FUN_00516360).
        let mut world = world();
        let (here, mut manufacturing, _, _) = shipyard_system(&mut world, 20);
        world.systems[here].control = ControlKind::Controlled(Faction::Empire);
        for yard in world.manufacturing_facilities.values_mut() {
            yard.side = Faction::Empire;
        }
        let mut state = stocked(10);
        state.empire.refined = 10;

        yards_day(&mut state, &world, &mut manufacturing, 1);
        assert_eq!(alliance(&state), (0, 10));
        assert_eq!(
            state.stock(Faction::Empire).map(|stock| stock.refined),
            Some(8)
        );
    }

    #[test]
    fn a_contested_systems_yards_build_for_their_last_holder() {
        // facility-ownership.md: while contested the yards keep their last
        // holder's side; a yard that is not its system's holder's does not
        // run.
        let mut world = world();
        let (here, mut manufacturing, _, _) = shipyard_system(&mut world, 20);
        let mut state = stocked(10);
        state.empire.refined = 10;

        world.systems[here].control = ControlKind::Controlled(Faction::Empire);
        yards_day(&mut state, &world, &mut manufacturing, 1);
        assert_eq!(alliance(&state), (0, 10));

        world.systems[here].control = ControlKind::Contested;
        yards_day(&mut state, &world, &mut manufacturing, 2);
        assert_eq!(alliance(&state), (0, 8));
        assert_eq!(
            state.stock(Faction::Empire).map(|stock| stock.refined),
            Some(10)
        );
    }

    #[test]
    fn a_yard_whose_class_has_no_rate_draws_nothing() {
        // FUN_00520b70 returns 0 for a yard without a class record; such a
        // yard never fires its cycle timer.
        let mut world = world();
        let (_, mut manufacturing, _, _) = shipyard_system(&mut world, 20);
        for class in world.buildable_classes.values_mut() {
            class.processing_rate = 0;
        }
        let mut state = stocked(10);

        for tick in 1..=8 {
            let advance = yards_day(&mut state, &world, &mut manufacturing, tick);
            assert_eq!(advance, ManufacturingAdvance::default());
        }
        assert_eq!(alliance(&state), (0, 10));
    }

    #[test]
    fn a_system_goes_idle_once_when_its_last_area_finishes() {
        // FUN_00509670: one manager per area, each with its own yards. K6
        // waits for the last busy area and fires once.
        use crate::manufacturing::{BuildableKind, QueueItem};
        let mut world = world();
        let (here, mut manufacturing, _, _) = shipyard_system(&mut world, 2);
        yard(&mut world, here, TRAINING_FACILITY, 2);
        let regiment = BuildableKind::Troop(DatId::new(0x1000_0001));
        manufacturing.enqueue(here, QueueItem::new(regiment, 1));
        let mut state = stocked(10);

        // Day 1 starts both shipyards and the training facility.
        yards_day(&mut state, &world, &mut manufacturing, 1);
        assert_eq!(alliance(&state), (0, 7));

        let second = yards_day(&mut state, &world, &mut manufacturing, 2);
        assert_eq!(second.completions.len(), 1);
        assert_eq!(second.completions[0].kind, regiment);
        assert!(second.newly_idle.is_empty());

        yards_day(&mut state, &world, &mut manufacturing, 3);
        let fourth = yards_day(&mut state, &world, &mut manufacturing, 4);
        assert_eq!(fourth.completions.len(), 1);
        assert_eq!(fourth.newly_idle, [here]);

        let fifth = yards_day(&mut state, &world, &mut manufacturing, 5);
        assert!(fifth.newly_idle.is_empty());
    }

    #[test]
    fn a_product_for_another_system_departs_when_its_points_are_made() {
        // FUN_0052bee0: EnrouteActive when the deployment location is not
        // where the product stands.
        let mut world = world();
        let (here, mut manufacturing, _, _) = shipyard_system(&mut world, 20);
        let away = system(&mut world);
        manufacturing.stop(here, ProductionArea::Shipyard);
        let bound = ship(&mut world, 1).delivered_to(away);
        manufacturing.enqueue(here, bound.clone());
        let mut state = stocked(10);

        yards_day(&mut state, &world, &mut manufacturing, 1);
        let advance = yards_day(&mut state, &world, &mut manufacturing, 2);

        assert!(advance.completions.is_empty());
        assert_eq!(
            advance.departures,
            vec![crate::manufacturing::Departure {
                origin: here,
                destination: away,
                tick: 2,
                kind: bound.kind,
            }]
        );
        assert_eq!(advance.newly_idle, [here]);
    }

    #[test]
    fn completions_come_in_system_order() {
        let mut world = world();
        let (first, mut manufacturing, _, _) = shipyard_system(&mut world, 1);
        let second = system(&mut world);
        yard(&mut world, second, ADVANCED_SHIPYARD, 2);
        manufacturing.enqueue(second, ship(&mut world, 1));
        let mut state = stocked(10);

        yards_day(&mut state, &world, &mut manufacturing, 1);
        let advance = yards_day(&mut state, &world, &mut manufacturing, 2);

        let systems: Vec<_> = advance.completions.iter().map(|done| done.system).collect();
        assert_eq!(systems, [first, second]);
        assert_eq!(advance.newly_idle, [first, second]);
    }

    #[test]
    fn a_removed_yard_is_forgotten() {
        let mut world = world();
        let (here, mut manufacturing, _, fast) = shipyard_system(&mut world, 20);
        let mut state = stocked(10);
        yards_day(&mut state, &world, &mut manufacturing, 1);
        assert!(state.yard_is_processing(fast));

        world.manufacturing_facilities.remove(fast);
        world.systems[here]
            .manufacturing_facilities
            .retain(|&key| key != fast);
        yards_day(&mut state, &world, &mut manufacturing, 2);

        assert!(!state.yards.contains_key(&fast));
    }

    #[test]
    fn a_manager_with_no_yard_scraps_its_queue() {
        // FUN_00529dd0: with no yard of the manager's area and side,
        // FUN_0052a2c0 scraps everything queued.
        use crate::manufacturing::{BuildableKind, QueueItem};
        let mut world = world();
        let (here, mut manufacturing, _, _) = shipyard_system(&mut world, 20);
        let regiment = BuildableKind::Troop(DatId::new(0x1000_0001));
        manufacturing.enqueue(here, QueueItem::new(regiment, 5));

        // The system has shipyards and no training facility.
        assert_eq!(
            scrap_yardless_queues(&world, &mut manufacturing),
            vec![(here, ProductionArea::TrainingFacility)]
        );
        assert!(manufacturing
            .queue(here, ProductionArea::TrainingFacility)
            .is_none());
        assert_eq!(
            manufacturing
                .queue(here, ProductionArea::Shipyard)
                .map(|queue| queue.len()),
            Some(1)
        );

        // A yard of the other side is not the manager's.
        for yard in world.manufacturing_facilities.values_mut() {
            yard.side = Faction::Empire;
        }
        assert_eq!(
            scrap_yardless_queues(&world, &mut manufacturing),
            vec![(here, ProductionArea::Shipyard)]
        );
    }

    #[test]
    fn a_contested_systems_queue_is_kept() {
        // facility-ownership.md: while contested the yards keep their last
        // holder's side, so the manager still has them.
        let mut world = world();
        let (here, mut manufacturing, _, _) = shipyard_system(&mut world, 20);
        world.systems[here].control = ControlKind::Contested;
        for yard in world.manufacturing_facilities.values_mut() {
            yard.side = Faction::Empire;
        }
        assert!(scrap_yardless_queues(&world, &mut manufacturing).is_empty());
        assert!(manufacturing
            .queue(here, ProductionArea::Shipyard)
            .is_some());
    }

    #[test]
    fn uprising_and_blockade_both_suspend_a_system() {
        // FUN_0050bbb0: Suspended = system +0x88 & 0x24 != 0.
        use crate::uprising::ActiveUprising;
        let mut world = world();
        let restless = system(&mut world);
        let calm = system(&mut world);
        let mut uprising = UprisingState::new();
        uprising.active_uprisings.insert(
            restless,
            ActiveUprising {
                started_tick: 1,
                next_incident_tick: None,
                next_support_tick: None,
                side: None,
            },
        );
        let suspended = suspended_systems(&BlockadeState::new(), &uprising);
        assert!(suspended.contains(&restless));
        assert!(!suspended.contains(&calm));
    }
}
