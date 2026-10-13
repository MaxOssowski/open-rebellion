//! Manufacturing: production queues and their build points
//! (`ghidra/notes/manufacturing-manager.md`).
//!
//! Each production area of a system keeps a queue. Its first item is the
//! current product; every yard cycle that ends adds one build point, and
//! the product completes when the points reach its class refined cost.
//!
//! # Architecture
//!
//! Production state is kept in a `ManufacturingState` map that lives alongside
//! `GameWorld` rather than inside it. `GameWorld` stores entity class templates
//! (`CapitalShipClass`, `FighterClass`, etc.); `ManufacturingState` stores the
//! per-system work-in-progress queues.
//!
//! Each tick, the caller feeds the `Vec<TickEvent>` from `GameClock::advance`
//! to `crate::production_cycle::advance_yards`, which runs the yards and
//! returns the `CompletionEvent`s for the caller to act on (spawn units,
//! add to fleet, etc.).

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::ids::{CapitalShipKey, DatId, FighterKey, SystemKey};

// ---------------------------------------------------------------------------
// BuildableKind
// ---------------------------------------------------------------------------

/// The class being produced. The order names a class, as Build Selection
/// lists them (`FUN_00537ff0` -> `FUN_0052e580`), and the instance is made
/// when the unit completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildableKind {
    /// A capital ship class (Star Destroyer, Mon Cal Cruiser, etc.)
    CapitalShip(CapitalShipKey),
    /// A fighter squadron class (X-Wing, TIE Fighter, etc.)
    Fighter(FighterKey),
    /// A regiment class (TROOPSD, family `0x10`).
    Troop(DatId),
    /// A special-force class (SPECFCSD, family `0x3c`).
    SpecialForce(DatId),
    /// A planetary defense class (DEFFACSD, families `0x22..0x25`).
    DefenseFacility(FacilityBuild),
    /// A shipyard, training facility or construction yard class (MANFACSD,
    /// families `0x28..0x2a`).
    ManufacturingFacility(FacilityBuild),
    /// A mine or refinery class (PROFACSD, families `0x2c..0x2d`).
    ProductionFacility(FacilityBuild),
}

/// A facility class and the side it is built for: the facility files serve
/// both sides, so the order carries its builder's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacilityBuild {
    pub class: DatId,
    pub is_alliance: bool,
}

impl BuildableKind {
    /// The `DatId` of a regiment, special-force or facility class; ships and
    /// fighters are keyed by their class slots instead.
    #[must_use]
    pub const fn class_dat_id(self) -> Option<DatId> {
        match self {
            Self::Troop(class) | Self::SpecialForce(class) => Some(class),
            Self::DefenseFacility(build)
            | Self::ManufacturingFacility(build)
            | Self::ProductionFacility(build) => Some(build.class),
            Self::CapitalShip(_) | Self::Fighter(_) => None,
        }
    }
}

/// The first day by which `work` units of progress are done when each yard
/// adds one unit every `period` days (`FUN_00528b30`: the sum over the
/// manager's yards of `days / period`; `FUN_00528d30` searches the least
/// such day). Yards with no period add nothing; `None` when none adds any.
#[must_use]
pub fn completion_day(periods: &[u32], work: u32) -> Option<u32> {
    let periods: Vec<u32> = periods.iter().copied().filter(|&p| p > 0).collect();
    if periods.is_empty() {
        return None;
    }
    if work == 0 {
        return Some(0);
    }
    let done = |days: u32| -> u64 { periods.iter().map(|&period| u64::from(days / period)).sum() };
    // The slowest yard alone finishes by `work * max`, so the answer lies
    // in (0, work * max].
    let (mut low, mut high) = (0_u32, work.saturating_mul(*periods.iter().max()?));
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if done(middle) >= u64::from(work) {
            high = middle;
        } else {
            low = middle;
        }
    }
    Some(high)
}

// ---------------------------------------------------------------------------
// QueueItem
// ---------------------------------------------------------------------------

/// One unit ordered at a production area.
///
/// The first unit in the queue is the manager's current product
/// (`FUN_0052aac0`); the rest wait their turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueItem {
    /// What is being built.
    pub kind: BuildableKind,
    /// The build points the unit needs: its class refined material cost
    /// (manager `+0x68` RequiredPointCount, `FUN_0052a8f0`).
    pub cost: u32,
    /// Where the finished object goes; `None` keeps it at the building
    /// system. A different system makes it travel there once completed
    /// (`FUN_0052bee0`, `crate::delivery`).
    pub destination: Option<SystemKey>,
}

impl QueueItem {
    /// A unit of `kind` needing `cost` build points.
    #[must_use]
    pub fn new(kind: BuildableKind, cost: u32) -> Self {
        QueueItem {
            kind,
            cost,
            destination: None,
        }
    }

    /// The same item, delivered to `destination` once completed.
    #[must_use]
    pub fn delivered_to(self, destination: SystemKey) -> Self {
        QueueItem {
            destination: Some(destination),
            ..self
        }
    }
}

// ---------------------------------------------------------------------------
// ProductionQueue
// ---------------------------------------------------------------------------

/// One production manager's queue and its build points
/// (`ghidra/notes/manufacturing-manager.md`).
///
/// Every yard cycle that ends adds one point (`FUN_0052a430`). The first
/// item is the current product; it completes when the points done reach
/// its cost (`FUN_0052b960`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductionQueue {
    items: VecDeque<QueueItem>,
    /// Manager `+0x5c` CompletedPointCount: points done on the current
    /// product.
    completed: u32,
    /// Manager `+0x60` OverflowPointCount: points made beyond the current
    /// product's cost.
    overflow: u32,
}

impl ProductionQueue {
    #[must_use]
    pub fn new() -> Self {
        ProductionQueue::default()
    }

    /// Append an item to the back of the queue.
    pub fn enqueue(&mut self, item: QueueItem) {
        self.items.push_back(item);
    }

    /// Remove the item at position `index` (0 = the current product).
    /// Returns `None` if the index is out of range.
    ///
    /// port: the original has no such order; Stop drops the whole queue.
    pub fn cancel(&mut self, index: usize) -> Option<QueueItem> {
        let removed = self.items.remove(index);
        self.clear_points_when_empty();
        removed
    }

    /// Move an item earlier in the queue (swap with the item ahead of it).
    ///
    /// No-ops if `index` is 0 (already at front) or out of range.
    pub fn prioritize(&mut self, index: usize) {
        if index == 0 || index >= self.items.len() {
            return;
        }
        self.items.swap(index - 1, index);
    }

    /// The current product, if any.
    #[must_use]
    pub fn active(&self) -> Option<&QueueItem> {
        self.items.front()
    }

    /// All items in queue order (index 0 = the current product).
    #[must_use]
    pub fn items(&self) -> &VecDeque<QueueItem> {
        &self.items
    }

    /// Points done on the current product (`+0x5c`).
    #[must_use]
    pub fn completed_points(&self) -> u32 {
        self.completed
    }

    /// Points made beyond the current product's cost (`+0x60`).
    #[must_use]
    pub fn overflow_points(&self) -> u32 {
        self.overflow
    }

    /// The points the manager still has to make (`FUN_00529dd0`):
    /// TotalRequiredPointCount less the overflow and the points done.
    #[must_use]
    pub fn points_needed(&self) -> u32 {
        let total: u32 = self.items.iter().map(|item| item.cost).sum();
        total.saturating_sub(self.overflow + self.completed)
    }

    /// The current product's progress, in [0.0, 1.0].
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    pub fn progress_fraction(&self) -> f32 {
        match self.items.front() {
            Some(item) if item.cost > 0 => (self.completed as f32 / item.cost as f32).min(1.0),
            Some(_) => 1.0,
            None => 0.0,
        }
    }

    /// The game day each item completes when yards with these `periods`
    /// keep building from day `today`: item `k` ends once the yards have
    /// made the points of every item up to it (`completion_day`). Empty
    /// when no yard builds. The manual's Best Time to Completion is a day
    /// of the game (Fig. 3.58).
    #[must_use]
    pub fn completion_days(&self, today: u64, periods: &[u32]) -> Vec<u64> {
        let done = self.completed + self.overflow;
        self.items
            .iter()
            .scan(0u32, |total, item| {
                *total += item.cost;
                Some(total.saturating_sub(done))
            })
            .map_while(|work| completion_day(periods, work))
            .map(|days| today + u64::from(days))
            .collect()
    }

    /// Total items, including the current product.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `FUN_0052b8c0`: an empty queue keeps no points.
    fn clear_points_when_empty(&mut self) {
        if self.items.is_empty() {
            self.completed = 0;
            self.overflow = 0;
        }
    }

    /// One yard cycle has ended (`FUN_0052a430`): a point for the current
    /// product while it lacks any, otherwise one of overflow. Returns the
    /// items this completes.
    ///
    /// A product completes when its points are non-zero and reach its cost
    /// (`FUN_0052b960`). The next one starts with the overflow, up to its
    /// own cost (`FUN_00529d70`), which may complete it too.
    pub fn add_point(&mut self) -> Vec<QueueItem> {
        let required = self.items.front().map_or(0, |item| item.cost);
        if self.completed < required {
            self.completed += 1;
        } else {
            self.overflow += 1;
        }
        let mut done = Vec::new();
        while let Some(front) = self.items.front() {
            if self.completed == 0 || self.completed < front.cost {
                break;
            }
            done.extend(self.items.pop_front());
            let next = self.items.front().map_or(0, |item| item.cost);
            self.completed = next.min(self.overflow);
            self.overflow -= self.completed;
        }
        self.clear_points_when_empty();
        done
    }
}

// ---------------------------------------------------------------------------
// ManufacturingState
// ---------------------------------------------------------------------------

/// A system's production area: the kind of facility a product comes from.
/// The original keeps one manager object per area at a system (families
/// `0xa0..0xaf`, `FUN_0052c170`): the agent's Destination (`0x214`) and
/// build orders act on construction managers (`0xa0..0xa1`) and training
/// managers (`0xa4..0xa5`; `ghidra/notes/manage-automation.md`), and the
/// manual gives each area its own Destination (p. 84, Fig. 3.75). hyp:
/// shipyards are `0xa2..0xa3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProductionArea {
    Shipyard,
    TrainingFacility,
    ConstructionYard,
}

impl ProductionArea {
    pub const ALL: [Self; 3] = [
        Self::Shipyard,
        Self::TrainingFacility,
        Self::ConstructionYard,
    ];

    /// The area that builds `kind`.
    #[must_use]
    pub const fn of(kind: BuildableKind) -> Self {
        match kind {
            BuildableKind::CapitalShip(_) | BuildableKind::Fighter(_) => Self::Shipyard,
            BuildableKind::Troop(_) | BuildableKind::SpecialForce(_) => Self::TrainingFacility,
            BuildableKind::DefenseFacility(_)
            | BuildableKind::ManufacturingFacility(_)
            | BuildableKind::ProductionFacility(_) => Self::ConstructionYard,
        }
    }
}

/// Every production area's queue in the galaxy.
///
/// The original keeps one manager per area at a system (`FUN_00509670`:
/// ships 0, facilities 1, troops 2), and each builds its own units at once:
/// the Manufacturing window's overview shows the three side by side
/// (`FUN_00455060`, `FUN_00457c90`). Areas with nothing queued are not
/// stored (lazy entry on first enqueue).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManufacturingState {
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    queues: HashMap<(SystemKey, ProductionArea), ProductionQueue>,
    /// Each production area's Destination (`0x214`) where it is not the
    /// area's own system.
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    destinations: HashMap<(SystemKey, ProductionArea), SystemKey>,
}

impl ManufacturingState {
    #[must_use]
    pub fn new() -> Self {
        ManufacturingState {
            queues: HashMap::new(),
            destinations: HashMap::new(),
        }
    }

    /// The queue of `system`'s `area`, created when absent.
    pub fn queue_mut(&mut self, system: SystemKey, area: ProductionArea) -> &mut ProductionQueue {
        self.queues.entry((system, area)).or_default()
    }

    /// Remove every area's queue at a system (used when system is destroyed).
    pub fn clear_queue(&mut self, system: SystemKey) {
        self.queues.retain(|(key, _), _| *key != system);
    }

    /// The queue of `system`'s `area` (read-only); `None` when never used.
    #[must_use]
    pub fn queue(&self, system: SystemKey, area: ProductionArea) -> Option<&ProductionQueue> {
        self.queues.get(&(system, area))
    }

    /// Units queued in every area at `system`.
    #[must_use]
    pub fn queued_at(&self, system: SystemKey) -> usize {
        ProductionArea::ALL
            .iter()
            .filter_map(|&area| self.queue(system, area))
            .map(ProductionQueue::len)
            .sum()
    }

    /// Enqueue an item on its area's queue at `system`. An item with no
    /// destination of its own takes its area's (hyp: a new product copies
    /// its manager's destination, `+0x74` → `+0x3c`; `build-delivery.md`).
    pub fn enqueue(&mut self, system: SystemKey, item: QueueItem) {
        let area = ProductionArea::of(item.kind);
        let item = match (item.destination, self.destination(system, area)) {
            (None, Some(destination)) => item.delivered_to(destination),
            _ => item,
        };
        self.queue_mut(system, area).enqueue(item);
    }

    /// Build: `count` units of `item` replace whatever `system`'s area of
    /// that kind was building. Manual p. 84: "Starting a new project cancels
    /// the current construction"; the units are built one after another
    /// ("Number to build", Fig. 3.25).
    pub fn build(&mut self, system: SystemKey, item: &QueueItem, count: u32) {
        self.stop(system, ProductionArea::of(item.kind));
        for _ in 0..count {
            self.enqueue(system, item.clone());
        }
    }

    /// Build the units of an order (`build_selection::order_items`): `items` replace
    /// what their area was building, in order.
    pub fn build_units(&mut self, system: SystemKey, items: Vec<QueueItem>) {
        let Some(first) = items.first() else {
            return;
        };
        self.stop(system, ProductionArea::of(first.kind));
        for item in items {
            self.enqueue(system, item);
        }
    }

    /// Stop: `system`'s `area` drops every unit it was building (manual
    /// p. 84, "Stopping Construction").
    pub fn stop(&mut self, system: SystemKey, area: ProductionArea) {
        self.queues.remove(&(system, area));
    }

    /// Where `system`'s `area` delivers what it builds, when not at home.
    #[must_use]
    pub fn destination(&self, system: SystemKey, area: ProductionArea) -> Option<SystemKey> {
        self.destinations.get(&(system, area)).copied()
    }

    /// The Destination order (`0x214`) on `system`'s `area`: what it builds,
    /// queued now or later, goes to `destination`; the area's own system
    /// clears it. hyp: the order rewrites the queued products' destination
    /// (`+0x3c`) too; its handler is untraced (`production-destination.md`).
    pub fn set_destination(
        &mut self,
        system: SystemKey,
        area: ProductionArea,
        destination: SystemKey,
    ) {
        let target = (destination != system).then_some(destination);
        match target {
            Some(destination) => self.destinations.insert((system, area), destination),
            None => self.destinations.remove(&(system, area)),
        };
        if let Some(queue) = self.queues.get_mut(&(system, area)) {
            for item in &mut queue.items {
                item.destination = target;
            }
        }
    }

    /// One yard cycle of `system`'s `area` has ended on day `tick`: add
    /// the point and report what it completes. An item bound elsewhere
    /// departs; the rest complete where they were built.
    pub fn add_point(
        &mut self,
        system: SystemKey,
        area: ProductionArea,
        tick: u64,
    ) -> (Vec<CompletionEvent>, Vec<Departure>) {
        let mut completions = Vec::new();
        let mut departures = Vec::new();
        let Some(queue) = self.queues.get_mut(&(system, area)) else {
            return (completions, departures);
        };
        for item in queue.add_point() {
            match item
                .destination
                .filter(|&destination| destination != system)
            {
                Some(destination) => departures.push(Departure {
                    origin: system,
                    destination,
                    tick,
                    kind: item.kind,
                }),
                None => completions.push(CompletionEvent {
                    system,
                    tick,
                    kind: item.kind,
                }),
            }
        }
        (completions, departures)
    }

    /// All system queues (including empty ones that were created lazily).
    #[must_use]
    pub fn queues(&self) -> &HashMap<(SystemKey, ProductionArea), ProductionQueue> {
        &self.queues
    }
}

// ---------------------------------------------------------------------------
// CompletionEvent + ManufacturingAdvance
// ---------------------------------------------------------------------------

/// Emitted when an item finishes construction.
///
/// The caller is responsible for acting on completions: spawning fleet units,
/// registering facilities in `GameWorld`, sending a message log entry, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionEvent {
    /// The system where construction completed.
    pub system: SystemKey,
    /// The game-day on which the item completed.
    pub tick: u64,
    /// What was built.
    pub kind: BuildableKind,
}

/// Wraps the outputs of `crate::production_cycle::advance_yards` so the
/// integrator can emit both `EVT_BUILD_COMPLETE`/`EVT_UNITS_DEPLOYED` (K5)
/// and `EVT_MANUFACTURING_IDLE` (K6) without extra world-state plumbing.
///
/// `newly_idle` is the set of system keys whose production queue
/// transitioned from non-empty to empty during this advance call. Detection
/// is intra-tick (pre/post length compare — no persistent "`was_empty`" bit).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManufacturingAdvance {
    pub completions: Vec<CompletionEvent>,
    pub newly_idle: Vec<SystemKey>,
    /// Completed items bound for another system; `crate::delivery` times
    /// their travel and completes them there on arrival.
    pub departures: Vec<Departure>,
}

/// A completed item leaving its building system for its destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Departure {
    /// The building system.
    pub origin: SystemKey,
    /// Where the item goes.
    pub destination: SystemKey,
    /// The game-day on which the item completed.
    pub tick: u64,
    /// What was built.
    pub kind: BuildableKind,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::SystemKey;

    // Helper: fabricate distinct SystemKeys from one shared slotmap.
    fn mock_system_keys(n: usize) -> Vec<SystemKey> {
        let mut sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        (0..n).map(|_| sm.insert(())).collect()
    }

    fn mock_system_key() -> SystemKey {
        mock_system_keys(1).into_iter().next().unwrap()
    }

    fn cap_ship_item(cost: u32) -> QueueItem {
        // We need a CapitalShipKey to build the kind. Use a slotmap.
        let mut sm: slotmap::SlotMap<CapitalShipKey, ()> = slotmap::SlotMap::with_key();
        let key = sm.insert(());
        QueueItem::new(BuildableKind::CapitalShip(key), cost)
    }

    fn fighter_item(cost: u32) -> QueueItem {
        let mut sm: slotmap::SlotMap<FighterKey, ()> = slotmap::SlotMap::with_key();
        let key = sm.insert(());
        QueueItem::new(BuildableKind::Fighter(key), cost)
    }

    fn troop_item(cost: u32) -> QueueItem {
        QueueItem::new(BuildableKind::Troop(DatId::new(0x1000_0001)), cost)
    }

    /// Add `points` points and return every item they complete.
    fn add_points(queue: &mut ProductionQueue, points: u32) -> Vec<QueueItem> {
        (0..points).flat_map(|_| queue.add_point()).collect()
    }

    // FUN_00528b30: each yard adds days / period units, and FUN_00528d30
    // finds the least day whose sum reaches the work. MANFACSD gives a
    // standard yard period 4 and an advanced one 2.
    #[test]
    fn a_build_ends_on_the_first_day_the_yards_progress_covers_its_work() {
        assert_eq!(completion_day(&[4], 8), Some(32));
        assert_eq!(completion_day(&[4, 2], 6), Some(8));
        assert_eq!(completion_day(&[4, 4], 3), Some(8));
        assert_eq!(completion_day(&[4, 0], 1), Some(4));
        assert_eq!(completion_day(&[], 5), None);
        assert_eq!(completion_day(&[0], 5), None);
        assert_eq!(completion_day(&[4], 0), Some(0));
    }

    // FUN_0052a430: a cycle's point goes to CompletedPointCount while that
    // is below RequiredPointCount, the current product's cost.
    #[test]
    fn a_point_counts_toward_the_current_product() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(cap_ship_item(3));
        queue.enqueue(fighter_item(2));

        assert!(add_points(&mut queue, 2).is_empty());

        assert_eq!(queue.completed_points(), 2);
        assert_eq!(queue.overflow_points(), 0);
        assert_eq!(queue.len(), 2);
    }

    // FUN_0052b960: the current product completes when its points reach
    // its cost; the next one becomes current with none (FUN_00529d70).
    #[test]
    fn a_product_completes_when_its_points_reach_its_cost() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(cap_ship_item(3));
        queue.enqueue(fighter_item(2));

        let done = add_points(&mut queue, 3);

        assert_eq!(done, [cap_ship_item(3)]);
        assert_eq!(queue.active(), Some(&fighter_item(2)));
        assert_eq!(queue.completed_points(), 0);
        assert_eq!(add_points(&mut queue, 1), []);
        assert_eq!(add_points(&mut queue, 1), [fighter_item(2)]);
    }

    // FUN_0052b960 asks for a non-zero CompletedPointCount, and
    // FUN_0052a430 sends a point to the overflow once the cost is met, so
    // a product that costs nothing never completes.
    #[test]
    fn a_product_that_costs_nothing_only_gathers_overflow() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(cap_ship_item(0));

        assert!(add_points(&mut queue, 3).is_empty());

        assert_eq!(queue.completed_points(), 0);
        assert_eq!(queue.overflow_points(), 3);
    }

    // FUN_00529d70: the next product starts with min(its cost, overflow)
    // and the overflow loses that; a product this pays for completes too.
    #[test]
    fn overflow_gives_the_following_products_a_head_start() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(cap_ship_item(0));
        queue.enqueue(fighter_item(1));
        queue.enqueue(troop_item(1));
        queue.enqueue(cap_ship_item(3));
        add_points(&mut queue, 3);
        queue.cancel(0);
        assert_eq!(queue.overflow_points(), 3);

        // One point pays the fighter; the overflow pays the regiment and
        // two of the ship's three.
        let done = queue.add_point();

        assert_eq!(done, [fighter_item(1), troop_item(1)]);
        assert_eq!(queue.active(), Some(&cap_ship_item(3)));
        assert_eq!(queue.completed_points(), 2);
        assert_eq!(queue.overflow_points(), 0);
    }

    // FUN_0052a430: with the cost already met, the point is overflow.
    #[test]
    fn a_point_beyond_the_current_products_cost_is_overflow() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(cap_ship_item(5));
        queue.enqueue(fighter_item(2));
        queue.enqueue(troop_item(4));
        add_points(&mut queue, 3);
        // The fighter becomes current with three points done.
        queue.prioritize(1);

        let done = queue.add_point();

        assert_eq!(done, [fighter_item(2)]);
        assert_eq!(queue.active(), Some(&cap_ship_item(5)));
        assert_eq!(queue.completed_points(), 1);
        assert_eq!(queue.overflow_points(), 0);
    }

    // FUN_0052b8c0: an empty queue clears both counts, so overflow is lost.
    #[test]
    fn an_emptied_queue_keeps_no_points() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(cap_ship_item(5));
        queue.enqueue(fighter_item(2));
        add_points(&mut queue, 3);

        queue.cancel(1);
        assert_eq!(queue.completed_points(), 3);
        queue.cancel(0);

        assert_eq!(queue.completed_points(), 0);
        assert_eq!(queue.overflow_points(), 0);

        // A point with nothing queued is not kept either.
        assert!(queue.add_point().is_empty());
        assert_eq!(queue.overflow_points(), 0);
    }

    // FUN_00529dd0: needed = TotalRequiredPointCount - OverflowPointCount
    // - CompletedPointCount.
    #[test]
    fn the_points_needed_are_the_queues_cost_less_the_points_made() {
        let mut queue = ProductionQueue::new();
        assert_eq!(queue.points_needed(), 0);
        queue.enqueue(cap_ship_item(5));
        queue.enqueue(fighter_item(2));
        assert_eq!(queue.points_needed(), 7);

        add_points(&mut queue, 3);
        assert_eq!(queue.points_needed(), 4);
        // Completing a product leaves the rest's need as it was.
        add_points(&mut queue, 2);
        assert_eq!(queue.points_needed(), 2);

        // Overflow counts as made, and never below nothing.
        let mut free = ProductionQueue::new();
        free.enqueue(cap_ship_item(0));
        free.enqueue(fighter_item(2));
        add_points(&mut free, 1);
        assert_eq!(free.points_needed(), 1);
        add_points(&mut free, 2);
        assert_eq!(free.points_needed(), 0);
    }

    #[test]
    fn progress_is_the_share_of_the_current_products_cost_done() {
        let mut queue = ProductionQueue::new();
        assert!(queue.progress_fraction().abs() < f32::EPSILON);
        queue.enqueue(cap_ship_item(10));
        add_points(&mut queue, 5);
        assert!((queue.progress_fraction() - 0.5).abs() < 0.001);

        let mut free = ProductionQueue::new();
        free.enqueue(cap_ship_item(0));
        assert!((free.progress_fraction() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn each_queued_item_completes_after_those_ahead_of_it() {
        // Manual Fig. 3.58: completion is a game day. FUN_00528d30: the
        // day the yards have made the points still owed.
        let mut queue = ProductionQueue::new();
        queue.enqueue(cap_ship_item(10));
        queue.enqueue(cap_ship_item(4));
        add_points(&mut queue, 3);

        // 7 and 11 points left: one yard of period 4, then with one of 2.
        assert_eq!(queue.completion_days(100, &[4]), [128, 144]);
        assert_eq!(queue.completion_days(100, &[4, 2]), [110, 116]);
        assert!(queue.completion_days(100, &[]).is_empty());
        assert!(ProductionQueue::new().completion_days(5, &[4]).is_empty());

        // Overflow counts as points made: three of the ship's four.
        let mut ahead = ProductionQueue::new();
        ahead.enqueue(cap_ship_item(0));
        ahead.enqueue(cap_ship_item(4));
        add_points(&mut ahead, 3);
        assert_eq!(ahead.overflow_points(), 3);
        assert_eq!(ahead.completion_days(100, &[4]), [100, 104]);
    }

    #[test]
    fn build_units_replaces_the_areas_queue_with_the_units_in_order() {
        let system = mock_system_key();
        let mut state = ManufacturingState::new();
        state.enqueue(system, cap_ship_item(9));
        state.enqueue(system, fighter_item(9));
        state.build_units(system, vec![cap_ship_item(8), cap_ship_item(4)]);
        let queue = state.queue(system, ProductionArea::Shipyard).unwrap();
        let costs: Vec<u32> = queue.items().iter().map(|item| item.cost).collect();
        assert_eq!(costs, [8, 4]);
        state.build_units(system, Vec::new());
        assert_eq!(
            state.queue(system, ProductionArea::Shipyard).unwrap().len(),
            2
        );
    }

    // FUN_0052bee0: a product built for another system leaves its facility
    // en route on completion; one built for its own system completes there.
    #[test]
    fn a_build_for_another_system_departs_and_one_for_its_own_completes_there() {
        let systems = mock_system_keys(3);
        let mut state = ManufacturingState::new();
        state.enqueue(systems[0], cap_ship_item(1).delivered_to(systems[1]));
        state.enqueue(systems[2], fighter_item(1).delivered_to(systems[2]));

        let (completions, departures) = state.add_point(systems[0], ProductionArea::Shipyard, 9);
        assert!(completions.is_empty());
        assert_eq!(
            departures,
            vec![Departure {
                origin: systems[0],
                destination: systems[1],
                tick: 9,
                kind: cap_ship_item(1).kind,
            }]
        );

        let (completions, departures) = state.add_point(systems[2], ProductionArea::Shipyard, 9);
        assert!(departures.is_empty());
        assert_eq!(
            completions,
            vec![CompletionEvent {
                system: systems[2],
                tick: 9,
                kind: fighter_item(1).kind,
            }]
        );
    }

    // FUN_00509670: a system has one manager per area (ships 0, facilities
    // 1, troops 2), each with its own points.
    #[test]
    fn a_point_reaches_only_its_own_areas_queue() {
        let system = mock_system_key();
        let mut state = ManufacturingState::new();
        state.enqueue(system, cap_ship_item(2));
        state.enqueue(system, troop_item(2));

        state.add_point(system, ProductionArea::TrainingFacility, 1);

        let points = |area| state.queue(system, area).unwrap().completed_points();
        assert_eq!(points(ProductionArea::TrainingFacility), 1);
        assert_eq!(points(ProductionArea::Shipyard), 0);
        // An area with no queue takes none.
        let (completions, departures) =
            state.add_point(system, ProductionArea::ConstructionYard, 1);
        assert!(completions.is_empty() && departures.is_empty());
    }

    // Manual p. 84: a new project cancels the current construction, and its
    // Number to build units are built one after another (Fig. 3.25).
    #[test]
    fn building_replaces_only_that_areas_units() {
        let system = mock_system_key();
        let mut state = ManufacturingState::new();
        state.enqueue(system, cap_ship_item(9));
        state.enqueue(system, troop_item(4));

        state.build(system, &cap_ship_item(2), 3);

        let ships = state.queue(system, ProductionArea::Shipyard).unwrap();
        assert_eq!(ships.len(), 3);
        assert!(ships.items().iter().all(|item| item.cost == 2));
        assert_eq!(
            state
                .queue(system, ProductionArea::TrainingFacility)
                .unwrap()
                .len(),
            1
        );
    }

    // FUN_00529ab0 scraps the queue before it creates the ordered units,
    // and the emptied queue clears its points (FUN_0052b8c0).
    #[test]
    fn building_drops_the_points_made_on_the_replaced_units() {
        let system = mock_system_key();
        let mut state = ManufacturingState::new();
        state.enqueue(system, cap_ship_item(9));
        state.add_point(system, ProductionArea::Shipyard, 1);

        state.build(system, &cap_ship_item(2), 1);

        let ships = state.queue(system, ProductionArea::Shipyard).unwrap();
        assert_eq!(ships.completed_points(), 0);
    }

    // Manual p. 84, "Stopping Construction".
    #[test]
    fn stopping_an_area_drops_its_units_and_keeps_the_others() {
        let system = mock_system_key();
        let mut state = ManufacturingState::new();
        state.build(system, &cap_ship_item(2), 2);
        state.enqueue(system, troop_item(4));

        state.stop(system, ProductionArea::Shipyard);

        assert!(state.queue(system, ProductionArea::Shipyard).is_none());
        assert_eq!(state.queued_at(system), 1);
    }

    #[test]
    fn clearing_a_system_drops_every_area() {
        let keys = mock_system_keys(2);
        let mut state = ManufacturingState::new();
        state.enqueue(keys[0], cap_ship_item(2));
        state.enqueue(keys[0], troop_item(2));
        state.enqueue(keys[1], troop_item(2));

        state.clear_queue(keys[0]);

        assert_eq!(state.queued_at(keys[0]), 0);
        assert_eq!(state.queued_at(keys[1]), 1);
    }

    #[test]
    fn enqueued_item_becomes_the_active_production() {
        let mut q = ProductionQueue::new();
        assert!(q.active().is_none());

        q.enqueue(cap_ship_item(10));
        assert_eq!(q.active().unwrap().cost, 10);
    }

    #[test]
    fn each_production_area_keeps_its_own_destination() {
        // Manual p. 84, Fig. 3.75: Destination per production area; the
        // agent's 0x214 acts on one area's manager (manage-automation.md).
        let mut systems: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let (home, away) = (systems.insert(()), systems.insert(()));
        let mut state = ManufacturingState::new();
        state.enqueue(home, cap_ship_item(10));
        state.enqueue(home, troop_item(5));

        state.set_destination(home, ProductionArea::Shipyard, away);
        let destinations = |state: &ManufacturingState| {
            ProductionArea::ALL
                .iter()
                .filter_map(|&area| state.queue(home, area))
                .flat_map(|queue| queue.items().iter().map(|item| item.destination))
                .collect::<Vec<_>>()
        };
        assert_eq!(destinations(&state), [Some(away), None]);
        assert_eq!(
            state.destination(home, ProductionArea::Shipyard),
            Some(away)
        );
        assert_eq!(
            state.destination(home, ProductionArea::TrainingFacility),
            None
        );

        // A later product of that area takes it; another area's does not.
        state.enqueue(home, cap_ship_item(3));
        state.enqueue(home, troop_item(5));
        assert_eq!(destinations(&state), [Some(away), Some(away), None, None]);

        // Back at home clears it.
        state.set_destination(home, ProductionArea::Shipyard, home);
        assert_eq!(destinations(&state), [None, None, None, None]);
        assert_eq!(state.destination(home, ProductionArea::Shipyard), None);
    }

    #[test]
    fn canceling_the_active_item_promotes_the_next() {
        let mut q = ProductionQueue::new();
        q.enqueue(cap_ship_item(10));
        q.enqueue(fighter_item(5));

        q.cancel(0);
        assert_eq!(q.len(), 1);
        assert_eq!(q.active().unwrap().cost, 5);
    }

    #[test]
    fn prioritize_moves_item_forward() {
        let mut q = ProductionQueue::new();
        q.enqueue(cap_ship_item(10)); // index 0
        q.enqueue(fighter_item(5)); // index 1

        q.prioritize(1);
        // Fighter should now be at index 0
        match q.active().unwrap().kind {
            BuildableKind::Fighter(_) => {}
            _ => panic!("expected fighter at front after prioritize"),
        }
    }

    #[test]
    fn prioritize_noop_on_front() {
        let mut q = ProductionQueue::new();
        q.enqueue(cap_ship_item(10));
        q.prioritize(0); // no-op
        assert_eq!(q.active().unwrap().cost, 10);
    }
}
