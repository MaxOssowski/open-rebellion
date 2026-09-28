//! Fleet movement system: hyperspace transit between star systems.
//!
//! Fleets travel by issuing a `MovementOrder` which specifies a destination
//! and the total transit duration. Each tick the fleet advances toward the
//! destination; on arrival an `ArrivalEvent` is emitted and the caller applies
//! it through [`apply_fleet_arrival`]. While a fleet is moving, its active
//! order is authoritative and it is absent from every system orbit index.
//!
//! # Speed model
//!
//! Transit time follows the original's per-object rule (`FUN_00514a60` ->
//! `FUN_00556430` -> `FUN_0055d8c0`, `ghidra/notes/build-delivery.md`):
//! ```text
//! transit_ticks = max(1, isqrt(dx^2 + dy^2) / GNPRTB[5120] * speed / 100)
//! ```
//! A fleet travels at its slowest capital ship's speed; a fleet of fighters
//! alone cannot enter hyperspace. Travel is point to point, with no lanes or
//! waypoints.
//!
//! # Usage
//!
//! ```
//! use rebellion_core::movement::{
//!     apply_fleet_arrival, begin_fleet_transit, MovementState, MovementSystem,
//! };
//! use rebellion_core::tick::TickEvent;
//!
//! let mut state = MovementState::new();
//! // Dispatch a fleet to move somewhere:
//! // begin_fleet_transit(&mut state, &mut world, fleet_key, dest_key, transit_ticks);
//!
//! let tick_events = vec![TickEvent { tick: 1 }];
//! let arrivals = MovementSystem::advance(&mut state, &tick_events);
//! // for event in &arrivals { apply_fleet_arrival(&mut world, &mut cargo, event); }
//! ```

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{FleetKey, SystemKey};
use crate::tick::TickEvent;
use crate::troop_transport::TroopTransportState;
use crate::world::{CapitalShipClass, FighterClass, FighterEntry, Fleet, GameWorld};

// ---------------------------------------------------------------------------
// Transit time
// ---------------------------------------------------------------------------

/// GNPRTB parameter dividing distance in the transit formula (`0x1400`,
/// `DAT_006bb6e8`, loaded by `FUN_0055cb60`).
pub const GNPRTB_TRANSIT_DISTANCE_DIVISOR: u16 = 5120;
/// GNPRTB parameter holding the default object speed (`DAT_006b9050`, loaded
/// by `FUN_0053e0b0`); also the percent base of `FUN_0053e190`.
pub const GNPRTB_DEFAULT_SPEED: u16 = 1;
/// Shipped GNPRTB.DAT value of parameter 5120 in every column, used only when
/// no GNPRTB table is loaded.
const SHIPPED_TRANSIT_DISTANCE_DIVISOR: i64 = 5;
/// Shipped GNPRTB.DAT value of parameter 1 in every column, used only when no
/// GNPRTB table is loaded.
const SHIPPED_DEFAULT_SPEED: i64 = 100;

/// A GNPRTB value, or its shipped value when the table is not loaded.
fn gnprtb_or_shipped(world: &GameWorld, id: u16, shipped: i64) -> i64 {
    match world.gnprtb.value(id, world.difficulty_index) {
        0 => shipped,
        value => i64::from(value),
    }
}

/// Integer square root exactly as `FUN_0053e1d0` computes it: a doubling
/// search for an upper bound, then a bisection that settles on the root.
#[must_use]
pub fn original_isqrt(value: i64) -> i64 {
    if value <= 3 {
        return i64::from(value > 0);
    }
    let mut high = 2;
    if value > 4 {
        loop {
            high *= 2;
            if high * high >= value {
                break;
            }
        }
    }
    let mut low = high / 2;
    let mut upper = high;
    let mut previous = high;
    loop {
        let mid = (low + upper) / 2;
        let square = mid * mid;
        let mut next_upper = mid;
        if square <= value {
            next_upper = upper;
            if square < value {
                low = mid;
            }
        }
        upper = next_upper;
        if mid == previous {
            return mid;
        }
        previous = mid;
    }
}

/// Transit days between two positions at `speed`, as `FUN_0055d8c0` computes
/// them: `(isqrt(dx^2 + dy^2) / GNPRTB[5120]) * speed / 100` in integer steps
/// (`FUN_0053e190` -> `FUN_0053e170` -> `FUN_0053e150`), 0 for the same
/// position, and at least 1 otherwise. `speed` 100 is the default; larger is
/// slower.
#[must_use]
pub fn transit_ticks_between(world: &GameWorld, from: (u16, u16), to: (u16, u16), speed: i64) -> u32 {
    let dx = i64::from(to.0) - i64::from(from.0);
    let dy = i64::from(to.1) - i64::from(from.1);
    let distance = original_isqrt(dx * dx + dy * dy);
    if distance == 0 {
        return 0;
    }
    let divisor = gnprtb_or_shipped(world, GNPRTB_TRANSIT_DISTANCE_DIVISOR, SHIPPED_TRANSIT_DISTANCE_DIVISOR);
    let percent = gnprtb_or_shipped(world, GNPRTB_DEFAULT_SPEED, SHIPPED_DEFAULT_SPEED);
    let ticks = (distance / divisor) * speed / percent;
    u32::try_from(ticks.max(1)).unwrap_or(u32::MAX)
}

/// Speed of one capital ship (`FUN_00500820`): the class `hyperdrive`, else
/// `hyperdrive_if_damaged`, else the GNPRTB 1 default.
///
/// The original subtracts `hyperdrive` when a per-ship damage nibble (ship
/// `+0x64` bits 16..19, `FUN_005011f0`) is set; the port does not model that
/// nibble, so every ship reads as undamaged.
#[must_use]
pub fn capital_ship_speed(world: &GameWorld, class: &CapitalShipClass) -> i64 {
    rated_speed(world, class.hyperdrive, class.hyperdrive_if_damaged)
}

/// Speed of one fighter squadron (`FUN_00502f80`): the same rule as a
/// capital ship over its FIGHTSD ratings, with no damage subtrahend.
#[must_use]
pub fn fighter_speed(world: &GameWorld, class: &FighterClass) -> i64 {
    rated_speed(world, class.hyperdrive, class.hyperdrive_if_damaged)
}

/// The GNPRTB 1 default speed, 100 in every shipped column; regiments and
/// facilities travel at it (`FUN_004f63f0`).
#[must_use]
pub fn default_speed(world: &GameWorld) -> i64 {
    gnprtb_or_shipped(world, GNPRTB_DEFAULT_SPEED, SHIPPED_DEFAULT_SPEED)
}

/// `hyperdrive`, else `hyperdrive_if_damaged`, else the GNPRTB 1 default
/// (`FUN_00500820`, `FUN_00502f80.c:19`).
fn rated_speed(world: &GameWorld, hyperdrive: u32, damaged: u32) -> i64 {
    match (hyperdrive, damaged) {
        (0, 0) => default_speed(world),
        (0, damaged) => i64::from(damaged),
        (hyperdrive, _) => i64::from(hyperdrive),
    }
}

/// Speed of a fleet (`FUN_004fd900`): the slowest, that is the largest, speed
/// among its capital ships; 0 as soon as one member reads 0. `None` when the
/// fleet cannot enter hyperspace because no capital ship carries it
/// (`FUN_004fda10` walks capital ships `0x14..0x1c` only, so fighters alone
/// cannot move).
#[must_use]
pub fn fleet_speed(fleet: &Fleet, world: &GameWorld) -> Option<i64> {
    let mut speeds = fleet
        .capital_ships
        .iter()
        .filter(|ship| ship.alive)
        .filter_map(|ship| world.capital_ship_classes.get(ship.class))
        .map(|class| capital_ship_speed(world, class));
    let first = speeds.next()?;
    Some(speeds.fold(first, |slowest, speed| if slowest == 0 || speed == 0 { 0 } else { slowest.max(speed) }))
}

/// Transit days for a fleet between two systems, or `None` when the fleet
/// cannot enter hyperspace (see [`fleet_speed`]).
#[must_use]
pub fn fleet_transit_ticks(fleet: &Fleet, world: &GameWorld, origin: SystemKey, dest: SystemKey) -> Option<u32> {
    let speed = fleet_speed(fleet, world)?;
    let position = |key: SystemKey| world.systems.get(key).map_or((0, 0), |s| (s.x, s.y));
    Some(transit_ticks_between(world, position(origin), position(dest), speed))
}

// ---------------------------------------------------------------------------
// MovementOrder
// ---------------------------------------------------------------------------

/// An active hyperspace transit order for one fleet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovementOrder {
    /// The fleet making this transit.
    pub fleet: FleetKey,
    /// System the fleet departed from (used for route visualization).
    pub origin: SystemKey,
    /// System the fleet is heading to.
    pub destination: SystemKey,
    /// Ticks needed to complete the transit.
    pub transit_ticks: u32,
    /// Ticks elapsed since departure.
    pub ticks_elapsed: u32,
}

impl MovementOrder {
    /// Create a new movement order.
    #[must_use]
    pub fn new(
        fleet: FleetKey,
        origin: SystemKey,
        destination: SystemKey,
        transit_ticks: u32,
    ) -> Self {
        MovementOrder {
            fleet,
            origin,
            destination,
            transit_ticks,
            ticks_elapsed: 0,
        }
    }

    /// Progress fraction in [0.0, 1.0] — 0.0 = just departed, 1.0 = arrived.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    pub fn progress(&self) -> f32 {
        if self.transit_ticks == 0 {
            return 1.0;
        }
        (self.ticks_elapsed as f32 / self.transit_ticks as f32).min(1.0)
    }

    /// True if the fleet has completed transit.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.ticks_elapsed >= self.transit_ticks
    }

    /// Remaining ticks until arrival.
    #[must_use]
    pub fn ticks_remaining(&self) -> u32 {
        self.transit_ticks.saturating_sub(self.ticks_elapsed)
    }
}

// ---------------------------------------------------------------------------
// MovementState
// ---------------------------------------------------------------------------

/// All active fleet movement orders.
///
/// At most one order per fleet. An active order must arrive or be cancelled
/// explicitly before another can be issued, so travel progress cannot be reset
/// accidentally by repeated player or AI dispatch.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MovementState {
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    orders: HashMap<FleetKey, MovementOrder>,
}

impl MovementState {
    #[must_use]
    pub fn new() -> Self {
        MovementState {
            orders: HashMap::new(),
        }
    }

    /// Issue a movement order if the fleet is not already in transit.
    ///
    /// `transit_ticks` should be computed via `fleet_transit_ticks`.
    /// Returns `true` when the order was accepted. Existing orders are left
    /// unchanged and return `false`.
    pub fn order(
        &mut self,
        fleet: FleetKey,
        origin: SystemKey,
        destination: SystemKey,
        transit_ticks: u32,
    ) -> bool {
        if self.orders.contains_key(&fleet) {
            return false;
        }
        self.orders.insert(
            fleet,
            MovementOrder::new(fleet, origin, destination, transit_ticks),
        );
        true
    }

    /// Cancel a movement order (fleet stays at current location).
    pub fn cancel(&mut self, fleet: FleetKey) -> Option<MovementOrder> {
        self.orders.remove(&fleet)
    }

    /// Cancel all movement orders targeting the given system.
    pub fn cancel_orders_to(&mut self, system: crate::ids::SystemKey) {
        self.orders.retain(|_, order| order.destination != system);
    }

    /// Get the active order for a fleet, if any.
    #[must_use]
    pub fn get(&self, fleet: FleetKey) -> Option<&MovementOrder> {
        self.orders.get(&fleet)
    }

    /// Whether a fleet currently has an active hyperspace order.
    #[must_use]
    pub fn is_in_transit(&self, fleet: FleetKey) -> bool {
        self.orders.contains_key(&fleet)
    }

    /// All active orders (immutable).
    #[must_use]
    pub fn orders(&self) -> &HashMap<FleetKey, MovementOrder> {
        &self.orders
    }

    /// All active orders (mutable) — for testing and manual state setup.
    pub fn orders_mut(&mut self) -> &mut HashMap<FleetKey, MovementOrder> {
        &mut self.orders
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.orders.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }
}

// ---------------------------------------------------------------------------
// ArrivalEvent
// ---------------------------------------------------------------------------

/// Emitted when a fleet completes hyperspace transit.
///
/// Apply this event through [`apply_fleet_arrival`] so fleet records and orbit
/// indexes remain canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrivalEvent {
    /// The fleet that arrived.
    pub fleet: FleetKey,
    /// The game-day on which the fleet arrived.
    pub tick: u64,
    /// The system the fleet departed from.
    pub origin: SystemKey,
    /// The system the fleet arrived at.
    pub system: SystemKey,
}

/// Result of applying one arrival to the canonical world fleet indexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedArrival {
    /// Stable fleet identity that remains at the destination.
    pub fleet: FleetKey,
    /// Faction retained before any redundant fleet record is removed.
    pub is_alliance: bool,
    /// Number of compatible fleet records absorbed into `fleet`.
    pub merged_fleets: usize,
}

/// Accepted faction-controlled fleet departure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedDeparture {
    pub fleet: FleetKey,
    pub origin: SystemKey,
    pub destination: SystemKey,
    pub transit_ticks: u32,
    pub is_alliance: bool,
}

/// Reason a player-facing fleet dispatch cannot begin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetDispatchError {
    MissingFleet,
    MissingOrigin,
    MissingDestination,
    DestinationDestroyed,
    WrongFaction,
    AlreadyInTransit,
    AlreadyAtDestination,
    EmptyFleet,
    /// No living capital ship can carry the fleet through hyperspace
    /// (`FUN_004fda10`).
    NoHyperdrive,
}

impl fmt::Display for FleetDispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingFleet => "fleet no longer exists",
            Self::MissingOrigin => "fleet origin is unavailable",
            Self::MissingDestination => "destination is unavailable",
            Self::DestinationDestroyed => "destination has been destroyed",
            Self::WrongFaction => "fleet is not controlled by the player",
            Self::AlreadyInTransit => "fleet is already in transit",
            Self::AlreadyAtDestination => "fleet is already at the destination",
            Self::EmptyFleet => "fleet has no ships or fighter squadrons",
            Self::NoHyperdrive => "fleet has no capital ship to carry it through hyperspace",
        };
        formatter.write_str(message)
    }
}

/// Validate a player-facing fleet dispatch without mutating the campaign.
///
/// # Errors
/// Returns a dispatch error for missing entities, a faction mismatch, an empty
/// fleet, a fleet without a capital ship, an active transit order, or an
/// invalid destination.
pub fn validate_fleet_dispatch(
    state: &MovementState,
    world: &GameWorld,
    fleet: FleetKey,
    destination: SystemKey,
    expected_is_alliance: bool,
) -> Result<(), FleetDispatchError> {
    let value = world
        .fleets
        .get(fleet)
        .ok_or(FleetDispatchError::MissingFleet)?;
    if value.is_alliance != expected_is_alliance {
        return Err(FleetDispatchError::WrongFaction);
    }
    if state.is_in_transit(fleet) {
        return Err(FleetDispatchError::AlreadyInTransit);
    }
    if !world.systems.contains_key(value.location) {
        return Err(FleetDispatchError::MissingOrigin);
    }
    let destination_system = world
        .systems
        .get(destination)
        .ok_or(FleetDispatchError::MissingDestination)?;
    if destination_system.is_destroyed {
        return Err(FleetDispatchError::DestinationDestroyed);
    }
    if value.location == destination {
        return Err(FleetDispatchError::AlreadyAtDestination);
    }
    if value.is_empty() {
        return Err(FleetDispatchError::EmptyFleet);
    }
    if fleet_speed(value, world).is_none() {
        return Err(FleetDispatchError::NoHyperdrive);
    }
    Ok(())
}

/// Validate, time, and begin a faction-controlled fleet departure.
///
/// # Errors
/// Returns a dispatch error when fleet or destination validation fails,
/// or the fleet already has a transit order.
pub fn begin_faction_fleet_transit(
    state: &mut MovementState,
    world: &mut GameWorld,
    fleet: FleetKey,
    destination: SystemKey,
    expected_is_alliance: bool,
) -> Result<AppliedDeparture, FleetDispatchError> {
    validate_fleet_dispatch(state, world, fleet, destination, expected_is_alliance)?;
    let value = world
        .fleets
        .get(fleet)
        .ok_or(FleetDispatchError::MissingFleet)?;
    let origin = value.location;
    let is_alliance = value.is_alliance;
    let transit_ticks = fleet_transit_ticks(value, world, origin, destination)
        .ok_or(FleetDispatchError::NoHyperdrive)?;
    if !begin_fleet_transit(state, world, fleet, destination, transit_ticks) {
        return Err(FleetDispatchError::AlreadyInTransit);
    }
    Ok(AppliedDeparture {
        fleet,
        origin,
        destination,
        transit_ticks,
        is_alliance,
    })
}

/// Begin transit and remove the fleet from its origin's orbit index.
///
/// `Fleet.location` remains the last orbiting system while `MovementOrder` is
/// the authoritative in-transit position. Rejected orders do not change the
/// world index.
pub fn begin_fleet_transit(
    state: &mut MovementState,
    world: &mut GameWorld,
    fleet: FleetKey,
    destination: SystemKey,
    transit_ticks: u32,
) -> bool {
    let origin = match world.fleets.get(fleet) {
        Some(value) => value.location,
        None => return false,
    };
    if !state.order(fleet, origin, destination, transit_ticks) {
        return false;
    }
    if let Some(system) = world.systems.get_mut(origin) {
        system.fleets.retain(|&key| key != fleet);
    }
    true
}

/// Rebuild `System.fleets` so it contains each orbiting fleet exactly once and
/// never contains an in-transit fleet.
pub fn reconcile_fleet_orbits(state: &MovementState, world: &mut GameWorld) {
    let mut orbiting: HashMap<FleetKey, SystemKey> = world
        .fleets
        .iter()
        .filter(|(fleet, _)| !state.is_in_transit(*fleet))
        .map(|(fleet, value)| (fleet, value.location))
        .collect();

    for (system_key, system) in &mut world.systems {
        system
            .fleets
            .retain(|fleet| orbiting.get(fleet) == Some(&system_key));
        system.fleets.sort_unstable();
        system.fleets.dedup();
        for fleet in &system.fleets {
            orbiting.remove(fleet);
        }
    }

    let mut missing: Vec<_> = orbiting.into_iter().collect();
    missing.sort_unstable_by_key(|(fleet, _)| *fleet);
    for (fleet, system) in missing {
        if let Some(value) = world.systems.get_mut(system) {
            value.fleets.push(fleet);
            value.fleets.sort_unstable();
        }
    }
}

/// Apply one arrival and consolidate anonymous, same-faction task forces.
///
/// Fleets carrying characters or a Death Star remain separate so explicit
/// player task-force identity is preserved. Production and ordinary AI fleets
/// can merge deterministically instead of accumulating one-ship records.
pub fn apply_fleet_arrival(
    world: &mut GameWorld,
    troop_transport: &mut TroopTransportState,
    arrival: &ArrivalEvent,
) -> Option<AppliedArrival> {
    let arriving = world.fleets.get(arrival.fleet)?;
    let is_alliance = arriving.is_alliance;
    let can_merge = arriving.characters.is_empty() && !arriving.has_death_star;

    if let Some(origin) = world.systems.get_mut(arrival.origin) {
        origin.fleets.retain(|&fleet| fleet != arrival.fleet);
    }
    if let Some(fleet) = world.fleets.get_mut(arrival.fleet) {
        fleet.location = arrival.system;
    }

    let mut compatible = Vec::new();
    if can_merge {
        if let Some(destination) = world.systems.get(arrival.system) {
            compatible.extend(destination.fleets.iter().copied().filter(|&fleet| {
                fleet != arrival.fleet
                    && world.fleets.get(fleet).is_some_and(|value| {
                        value.location == arrival.system
                            && value.is_alliance == is_alliance
                            && value.characters.is_empty()
                            && !value.has_death_star
                    })
            }));
        }
    }
    compatible.push(arrival.fleet);
    compatible.sort_unstable();
    compatible.dedup();

    let survivor = compatible[0];
    let absorbed_keys: Vec<_> = compatible
        .iter()
        .copied()
        .filter(|&fleet| fleet != survivor)
        .collect();
    let absorbed: Vec<_> = absorbed_keys
        .iter()
        .filter_map(|&fleet| world.fleets.remove(fleet))
        .collect();
    for &fleet in &absorbed_keys {
        troop_transport.transfer_fleet(fleet, survivor);
    }

    if let Some(fleet) = world.fleets.get_mut(survivor) {
        fleet.location = arrival.system;
        for other in absorbed {
            fleet.capital_ships.extend(other.capital_ships);
            for fighter in other.fighters {
                if let Some(entry) = fleet
                    .fighters
                    .iter_mut()
                    .find(|entry| entry.class == fighter.class)
                {
                    entry.count = entry.count.saturating_add(fighter.count);
                } else {
                    fleet.fighters.push(FighterEntry {
                        class: fighter.class,
                        count: fighter.count,
                    });
                }
            }
        }
    }

    if let Some(destination) = world.systems.get_mut(arrival.system) {
        destination
            .fleets
            .retain(|fleet| !absorbed_keys.contains(fleet) && *fleet != survivor);
        destination.fleets.push(survivor);
        destination.fleets.sort_unstable();
        destination.fleets.dedup();
    }

    Some(AppliedArrival {
        fleet: survivor,
        is_alliance,
        merged_fleets: absorbed_keys.len(),
    })
}

// ---------------------------------------------------------------------------
// MovementSystem
// ---------------------------------------------------------------------------

/// Stateless system that advances fleet transit orders per tick.
pub struct MovementSystem;

impl MovementSystem {
    /// Advance all active movement orders by the ticks in `tick_events`.
    ///
    /// Returns one `ArrivalEvent` per fleet that completes transit this frame.
    /// The caller applies each event through [`apply_fleet_arrival`] so world
    /// fleet records and system orbit indexes remain canonical.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    ///
    /// # Panics
    /// Panics if an order key collected for this batch is absent when its order is advanced.
    pub fn advance(state: &mut MovementState, tick_events: &[TickEvent]) -> Vec<ArrivalEvent> {
        let Some(last_tick_event) = tick_events.last() else {
            return Vec::new();
        };

        let tick_count = tick_events.len() as u32;
        let final_tick = last_tick_event.tick;
        let mut arrivals = Vec::new();

        // HashMap iteration order is randomized per process. Arrival order
        // mutates per-system fleet vectors downstream, so walk by fleet key.
        let mut fleet_keys: Vec<_> = state.orders.keys().copied().collect();
        fleet_keys.sort_unstable();

        // Advance all orders; collect completed ones.
        let mut completed_keys = Vec::new();
        for fleet_key in fleet_keys {
            let order = state
                .orders
                .get_mut(&fleet_key)
                .expect("movement order key collected from the same map");
            order.ticks_elapsed = order
                .ticks_elapsed
                .saturating_add(tick_count)
                .min(order.transit_ticks);

            if order.is_complete() {
                arrivals.push(ArrivalEvent {
                    fleet: fleet_key,
                    tick: final_tick,
                    origin: order.origin,
                    system: order.destination,
                });
                completed_keys.push(fleet_key);
            }
        }

        // Remove completed orders.
        for key in completed_keys {
            state.orders.remove(&key);
        }

        arrivals
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tick::TickEvent;
    use crate::world::ControlKind;

    fn mock_fleet_and_systems() -> (FleetKey, SystemKey, SystemKey) {
        let mut fleet_sm: slotmap::SlotMap<FleetKey, ()> = slotmap::SlotMap::with_key();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let fleet = fleet_sm.insert(());
        let origin = sys_sm.insert(());
        let dest = sys_sm.insert(());
        (fleet, origin, dest)
    }

    fn ticks(n: u64) -> Vec<TickEvent> {
        (1..=n).map(|t| TickEvent { tick: t }).collect()
    }

    // --- MovementOrder ---

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "Transit endpoints are exactly zero and one."
    )]
    fn progress_starts_at_zero() {
        let (fleet, origin, dest) = mock_fleet_and_systems();
        let order = MovementOrder::new(fleet, origin, dest, 10);
        assert_eq!(order.progress(), 0.0);
        assert!(!order.is_complete());
        assert_eq!(order.ticks_remaining(), 10);
    }

    #[test]
    fn progress_reports_half_at_midpoint() {
        let (fleet, origin, dest) = mock_fleet_and_systems();
        let mut order = MovementOrder::new(fleet, origin, dest, 10);
        order.ticks_elapsed = 5;
        assert!((order.progress() - 0.5).abs() < 1e-6);
        assert_eq!(order.ticks_remaining(), 5);
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "Transit endpoints are exactly zero and one."
    )]
    fn progress_clamps_at_one() {
        let (fleet, origin, dest) = mock_fleet_and_systems();
        let mut order = MovementOrder::new(fleet, origin, dest, 5);
        order.ticks_elapsed = 10; // overshoot
        assert_eq!(order.progress(), 1.0);
        assert!(order.is_complete());
    }

    // --- MovementState ---

    #[test]
    fn ordered_movement_is_retrievable_by_fleet() {
        let (fleet, origin, dest) = mock_fleet_and_systems();
        let mut state = MovementState::new();
        state.order(fleet, origin, dest, 10);
        assert_eq!(state.len(), 1);
        assert_eq!(state.get(fleet).unwrap().destination, dest);
    }

    #[test]
    fn cancel_removes_order() {
        let (fleet, origin, dest) = mock_fleet_and_systems();
        let mut state = MovementState::new();
        state.order(fleet, origin, dest, 10);
        let removed = state.cancel(fleet);
        assert!(removed.is_some());
        assert!(state.is_empty());
    }

    #[test]
    fn active_order_rejects_redispatch_without_resetting_progress() {
        let (fleet, origin, dest) = mock_fleet_and_systems();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let dest2 = sys_sm.insert(());

        let mut state = MovementState::new();
        assert!(state.order(fleet, origin, dest, 10));
        MovementSystem::advance(&mut state, &ticks(4));
        let before = state.get(fleet).unwrap().clone();

        assert!(!state.order(fleet, origin, dest2, 20));
        assert_eq!(state.len(), 1);
        assert_eq!(state.get(fleet).unwrap(), &before);
        assert_eq!(state.get(fleet).unwrap().destination, dest);
        assert_eq!(state.get(fleet).unwrap().ticks_elapsed, 4);
    }

    // --- MovementSystem ---

    #[test]
    fn no_ticks_no_arrivals() {
        let (fleet, origin, dest) = mock_fleet_and_systems();
        let mut state = MovementState::new();
        state.order(fleet, origin, dest, 5);
        let arrivals = MovementSystem::advance(&mut state, &[]);
        assert!(arrivals.is_empty());
        assert_eq!(state.len(), 1); // order still active
    }

    #[test]
    fn partial_advance_does_not_arrive() {
        let (fleet, origin, dest) = mock_fleet_and_systems();
        let mut state = MovementState::new();
        state.order(fleet, origin, dest, 10);
        let arrivals = MovementSystem::advance(&mut state, &ticks(5));
        assert!(arrivals.is_empty());
        assert_eq!(state.get(fleet).unwrap().ticks_elapsed, 5);
    }

    #[test]
    fn advance_to_completion_emits_arrival() {
        let (fleet, origin, dest) = mock_fleet_and_systems();
        let mut state = MovementState::new();
        state.order(fleet, origin, dest, 5);
        let arrivals = MovementSystem::advance(&mut state, &ticks(5));
        assert_eq!(arrivals.len(), 1);
        assert_eq!(arrivals[0].fleet, fleet);
        assert_eq!(arrivals[0].system, dest);
        assert_eq!(arrivals[0].origin, origin);
        assert_eq!(arrivals[0].tick, 5);
        // Order removed on arrival
        assert!(state.is_empty());
    }

    #[test]
    fn overshoot_still_arrives_exactly_once() {
        let (fleet, origin, dest) = mock_fleet_and_systems();
        let mut state = MovementState::new();
        state.order(fleet, origin, dest, 3);
        // 10 ticks for a 3-tick journey
        let arrivals = MovementSystem::advance(&mut state, &ticks(10));
        assert_eq!(arrivals.len(), 1);
        assert!(state.is_empty());
    }

    #[test]
    fn multiple_fleets_advance_independently() {
        let mut fleet_sm: slotmap::SlotMap<FleetKey, ()> = slotmap::SlotMap::with_key();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let fleet_a = fleet_sm.insert(());
        let fleet_b = fleet_sm.insert(());
        let origin = sys_sm.insert(());
        let dest_a = sys_sm.insert(());
        let dest_b = sys_sm.insert(());

        let mut state = MovementState::new();
        state.order(fleet_a, origin, dest_a, 5);
        state.order(fleet_b, origin, dest_b, 10);

        // 5 ticks: fleet_a arrives, fleet_b is at 5/10
        let arrivals = MovementSystem::advance(&mut state, &ticks(5));
        assert_eq!(arrivals.len(), 1);
        assert_eq!(arrivals[0].fleet, fleet_a);
        assert_eq!(state.len(), 1);
        assert_eq!(state.get(fleet_b).unwrap().ticks_elapsed, 5);
    }

    #[test]
    fn simultaneous_arrivals_use_stable_fleet_key_order() {
        let mut fleet_sm: slotmap::SlotMap<FleetKey, ()> = slotmap::SlotMap::with_key();
        let fleet_a = fleet_sm.insert(());
        let fleet_b = fleet_sm.insert(());
        let fleet_c = fleet_sm.insert(());
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let origin = sys_sm.insert(());
        let destination = sys_sm.insert(());
        let mut state = MovementState::new();

        for fleet in [fleet_c, fleet_b, fleet_a] {
            state.order(fleet, origin, destination, 1);
        }

        let arrivals = MovementSystem::advance(&mut state, &ticks(1));
        let arrived_fleets: Vec<_> = arrivals.iter().map(|arrival| arrival.fleet).collect();
        assert_eq!(arrived_fleets, vec![fleet_a, fleet_b, fleet_c]);
    }

    // --- Distance-based transit tests ---

    use crate::dat::{ExplorationStatus, SectorGroup};
    use crate::ids::DatId;
    use crate::world::{
        CapitalShipClass, Character, Fleet, GameWorld, Sector, ShipInstance, System, TroopUnit,
    };

    fn test_character(name: &str, hyperdrive_modifier: i16) -> Character {
        Character {
            name: name.into(),
            is_alliance: true,
            hyperdrive_modifier,
            ..Default::default()
        }
    }

    fn test_ship_class(hyperdrive: u32) -> CapitalShipClass {
        CapitalShipClass {
            name: "TestShip".into(),
            is_alliance: true,
            hull: 100,
            shield_strength: 50,
            sub_light_engine: 5,
            maneuverability: 5,
            hyperdrive,
            troop_capacity: 1,
            ..CapitalShipClass::default()
        }
    }

    fn make_system(sector: crate::ids::SectorKey, x: u16, y: u16) -> System {
        System {
            dat_id: DatId::new(0x9000_0000),
            name: format!("Sys@{x},{y}"),
            sector,
            x,
            y,
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
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Uncontrolled,
        }
    }

    fn make_transit_world(x1: u16, y1: u16, x2: u16, y2: u16) -> (GameWorld, SystemKey, SystemKey) {
        let mut world = GameWorld::default();
        let sk = world.sectors.insert(Sector {
            dat_id: DatId::new(0x9200_0000),
            name: "Test".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let s1 = world.systems.insert(make_system(sk, x1, y1));
        let s2 = world.systems.insert(make_system(sk, x2, y2));
        (world, s1, s2)
    }

    fn add_test_fleet(
        world: &mut GameWorld,
        system: SystemKey,
        ship_key: crate::ids::CapitalShipKey,
    ) -> FleetKey {
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![ShipInstance::new(ship_key, 100, true)],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        world.systems[system].fleets.push(fleet);
        fleet
    }

    #[test]
    fn transit_owns_position_until_arrival() {
        let (mut world, origin, destination) = make_transit_world(0, 0, 30, 40);
        let ship_key = world.capital_ship_classes.insert(test_ship_class(80));
        let fleet = add_test_fleet(&mut world, origin, ship_key);
        let mut movement = MovementState::new();

        assert!(begin_fleet_transit(
            &mut movement,
            &mut world,
            fleet,
            destination,
            5,
        ));
        assert!(!world.systems[origin].fleets.contains(&fleet));
        reconcile_fleet_orbits(&movement, &mut world);
        assert!(!world.systems[origin].fleets.contains(&fleet));

        let arrival = MovementSystem::advance(&mut movement, &ticks(5)).remove(0);
        let applied =
            apply_fleet_arrival(&mut world, &mut TroopTransportState::default(), &arrival).unwrap();
        assert_eq!(applied.fleet, fleet);
        assert_eq!(applied.merged_fleets, 0);
        assert_eq!(world.fleets[fleet].location, destination);
        assert_eq!(world.systems[destination].fleets, vec![fleet]);
    }

    #[test]
    fn reconcile_removes_transit_ghosts_and_restores_stationary_fleets() {
        let (mut world, origin, destination) = make_transit_world(0, 0, 30, 40);
        let ship_key = world.capital_ship_classes.insert(test_ship_class(80));
        let transit = add_test_fleet(&mut world, origin, ship_key);
        let stationary = add_test_fleet(&mut world, destination, ship_key);
        world.systems[origin].fleets.push(stationary);
        world.systems[destination].fleets.clear();

        let mut movement = MovementState::new();
        assert!(movement.order(transit, origin, destination, 5));
        reconcile_fleet_orbits(&movement, &mut world);

        assert!(world.systems[origin].fleets.is_empty());
        assert_eq!(world.systems[destination].fleets, vec![stationary]);
    }

    #[test]
    fn compatible_arrival_merges_into_stable_fleet_identity() {
        let (mut world, origin, destination) = make_transit_world(0, 0, 30, 40);
        let ship_key = world.capital_ship_classes.insert(test_ship_class(80));
        let survivor = add_test_fleet(&mut world, destination, ship_key);
        let arriving = add_test_fleet(&mut world, origin, ship_key);
        let arrival = ArrivalEvent {
            fleet: arriving,
            tick: 5,
            origin,
            system: destination,
        };
        let troop = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[origin].ground_units.push(troop);
        let mut transport = TroopTransportState::default();
        transport.embark(&mut world, arriving, &[troop]).unwrap();

        let applied = apply_fleet_arrival(&mut world, &mut transport, &arrival).unwrap();

        assert_eq!(applied.fleet, survivor);
        assert_eq!(applied.merged_fleets, 1);
        assert!(!world.fleets.contains_key(arriving));
        assert_eq!(world.fleets[survivor].ship_count(), 2);
        assert_eq!(world.systems[destination].fleets, vec![survivor]);
        assert_eq!(transport.cargo(survivor), &[troop]);
        assert!(transport.cargo(arriving).is_empty());
    }

    #[test]
    fn character_task_force_remains_separate_on_arrival() {
        let (mut world, origin, destination) = make_transit_world(0, 0, 30, 40);
        let ship_key = world.capital_ship_classes.insert(test_ship_class(80));
        let stationed = add_test_fleet(&mut world, destination, ship_key);
        let arriving = add_test_fleet(&mut world, origin, ship_key);
        let character = world.characters.insert(test_character("Commander", 0));
        world.fleets[arriving].characters.push(character);
        let arrival = ArrivalEvent {
            fleet: arriving,
            tick: 5,
            origin,
            system: destination,
        };

        let applied =
            apply_fleet_arrival(&mut world, &mut TroopTransportState::default(), &arrival).unwrap();

        assert_eq!(applied.fleet, arriving);
        assert_eq!(applied.merged_fleets, 0);
        assert!(world.fleets.contains_key(stationed));
        assert!(world.fleets.contains_key(arriving));
        assert_eq!(world.systems[destination].fleets, vec![stationed, arriving]);
    }

    #[test]
    fn faction_dispatch_validates_and_begins_one_authoritative_order() {
        let (mut world, origin, destination) = make_transit_world(0, 0, 30, 40);
        let ship_key = world.capital_ship_classes.insert(test_ship_class(80));
        let fleet = add_test_fleet(&mut world, origin, ship_key);
        let mut movement = MovementState::new();

        assert_eq!(
            validate_fleet_dispatch(&movement, &world, fleet, destination, false),
            Err(FleetDispatchError::WrongFaction),
        );
        assert_eq!(
            validate_fleet_dispatch(&movement, &world, fleet, origin, true),
            Err(FleetDispatchError::AlreadyAtDestination),
        );

        world.fleets[fleet].capital_ships.clear();
        assert_eq!(
            validate_fleet_dispatch(&movement, &world, fleet, destination, true),
            Err(FleetDispatchError::EmptyFleet),
        );
        world.fleets[fleet]
            .capital_ships
            .push(ShipInstance::new(ship_key, 100, true));

        let sector = world.systems[origin].sector;
        let missing = world.systems.insert(make_system(sector, 60, 80));
        world.systems.remove(missing);
        assert_eq!(
            validate_fleet_dispatch(&movement, &world, fleet, missing, true),
            Err(FleetDispatchError::MissingDestination),
        );

        world.systems[destination].is_destroyed = true;
        assert_eq!(
            validate_fleet_dispatch(&movement, &world, fleet, destination, true),
            Err(FleetDispatchError::DestinationDestroyed),
        );
        world.systems[destination].is_destroyed = false;

        let departure = begin_faction_fleet_transit(
            &mut movement,
            &mut world,
            fleet,
            destination,
            true,
        )
        .unwrap();
        assert_eq!(departure.origin, origin);
        assert_eq!(departure.destination, destination);
        // FUN_0055d8c0: isqrt(30^2 + 40^2) = 50; 50 / 5 * 80 / 100 = 8.
        assert_eq!(departure.transit_ticks, 8);
        assert!(!world.systems[origin].fleets.contains(&fleet));
        assert_eq!(movement.get(fleet).unwrap().destination, destination);
        assert_eq!(
            validate_fleet_dispatch(&movement, &world, fleet, destination, true),
            Err(FleetDispatchError::AlreadyInTransit),
        );
    }

    fn fleet_of(world: &mut GameWorld, location: SystemKey, classes: &[CapitalShipClass]) -> Fleet {
        let capital_ships = classes
            .iter()
            .map(|class| ShipInstance::new(world.capital_ship_classes.insert(class.clone()), 100, true))
            .collect();
        Fleet {
            location,
            capital_ships,
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        }
    }

    // FUN_0053e1d0: doubling search then bisection. It returns floor(sqrt(n))
    // except at powers of four from 4 on, where it returns one less.
    #[test]
    fn the_integer_square_root_is_the_floor_except_one_low_at_powers_of_four() {
        for n in [0_i64, 1, 2, 3, 5, 15, 17, 99, 100, 2_500, 192_400, 299_999] {
            assert_eq!(original_isqrt(n), n.isqrt(), "n = {n}");
        }
        for (n, root) in [(4, 1), (16, 3), (64, 7), (256, 15), (65_536, 255)] {
            assert_eq!(original_isqrt(n), root, "n = {n}");
        }
    }

    // FUN_0055d8c0: (isqrt(d^2) / GNPRTB 5120) * speed / 100, integer steps.
    #[test]
    fn transit_divides_the_integer_distance_by_five_then_scales_by_speed() {
        let (world, _, _) = make_transit_world(0, 0, 0, 0);
        assert_eq!(transit_ticks_between(&world, (0, 0), (30, 40), 100), 10);
        // isqrt(300^2 + 320^2) = 438; 438 / 5 = 87; 87 * 80 / 100 = 69.
        assert_eq!(transit_ticks_between(&world, (0, 0), (300, 320), 80), 69);
        assert_eq!(transit_ticks_between(&world, (300, 320), (0, 0), 80), 69);
    }

    // FUN_0055d8c0: a zero result becomes 1; a zero distance stays 0.
    #[test]
    fn a_short_hop_takes_one_day_and_the_same_position_takes_none() {
        let (world, _, _) = make_transit_world(0, 0, 0, 0);
        assert_eq!(transit_ticks_between(&world, (0, 0), (3, 0), 100), 1);
        assert_eq!(transit_ticks_between(&world, (7, 7), (7, 7), 100), 0);
    }

    // DAT_006bb6e8 (GNPRTB 5120) and DAT_006b9050 (GNPRTB 1) come from the table.
    #[test]
    fn loaded_gnprtb_values_replace_the_shipped_divisor_and_base() {
        let (mut world, _, _) = make_transit_world(0, 0, 0, 0);
        let entry = |id, value| crate::world::GnprtbEntry {
            parameter_id: id,
            development: value,
            alliance_sp_easy: value,
            alliance_sp_medium: value,
            alliance_sp_hard: value,
            empire_sp_easy: value,
            empire_sp_medium: value,
            empire_sp_hard: value,
            multiplayer: value,
        };
        world.gnprtb = crate::world::GnprtbParams::new(vec![entry(5120, 10), entry(1, 50)]);
        // 50 / 10 = 5; 5 * 100 / 50 = 10.
        assert_eq!(transit_ticks_between(&world, (0, 0), (30, 40), 100), 10);
    }

    // FUN_00500820: hyperdrive, else hyperdrive_if_damaged, else GNPRTB 1.
    #[test]
    fn a_ship_without_hyperdrive_uses_its_damaged_rating_then_the_default() {
        let (world, _, _) = make_transit_world(0, 0, 0, 0);
        let class = |hyperdrive, hyperdrive_if_damaged| CapitalShipClass {
            hyperdrive,
            hyperdrive_if_damaged,
            ..test_ship_class(0)
        };
        assert_eq!(capital_ship_speed(&world, &class(80, 120)), 80);
        assert_eq!(capital_ship_speed(&world, &class(0, 120)), 120);
        assert_eq!(capital_ship_speed(&world, &class(0, 0)), 100);
    }

    // FUN_004fd900: the fleet keeps the largest member speed; larger is slower.
    #[test]
    fn the_slowest_ship_sets_the_fleet_speed() {
        let (mut world, origin, dest) = make_transit_world(0, 0, 300, 320);
        let fleet = fleet_of(&mut world, origin, &[test_ship_class(50), test_ship_class(80)]);
        assert_eq!(fleet_speed(&fleet, &world), Some(80));
        assert_eq!(fleet_transit_ticks(&fleet, &world, origin, dest), Some(69));
    }

    // FUN_004fd900 skips members that are not completed; a destroyed ship
    // no longer counts.
    #[test]
    fn a_destroyed_ship_does_not_slow_the_fleet() {
        let (mut world, origin, _) = make_transit_world(0, 0, 300, 320);
        let mut fleet = fleet_of(&mut world, origin, &[test_ship_class(50), test_ship_class(80)]);
        fleet.capital_ships[1].alive = false;
        assert_eq!(fleet_speed(&fleet, &world), Some(50));
    }

    // FUN_004fda10 and FUN_004fd900 walk capital ships (0x14..0x1c) only.
    #[test]
    fn a_fleet_of_fighters_alone_cannot_enter_hyperspace() {
        let (mut world, origin, dest) = make_transit_world(0, 0, 300, 320);
        let mut fleet = fleet_of(&mut world, origin, &[]);
        fleet.fighters.push(FighterEntry { class: world.fighter_classes.insert(Default::default()), count: 1 });
        assert_eq!(fleet_transit_ticks(&fleet, &world, origin, dest), None);
        let key = world.fleets.insert(fleet);
        world.systems[origin].fleets.push(key);
        assert_eq!(
            validate_fleet_dispatch(&MovementState::new(), &world, key, dest, true),
            Err(FleetDispatchError::NoHyperdrive),
        );
    }

    // The Han Solo speed (GNPRTB 3083) is a character's own mission speed
    // (FUN_004ed370, FUN_00542990); fleets take no character into account.
    #[test]
    fn a_character_aboard_does_not_change_fleet_transit() {
        let (mut world, origin, dest) = make_transit_world(0, 0, 300, 320);
        let mut fleet = fleet_of(&mut world, origin, &[test_ship_class(80)]);
        let without = fleet_transit_ticks(&fleet, &world, origin, dest);
        fleet.characters.push(world.characters.insert(test_character("Han Solo", 50)));
        assert_eq!(fleet_transit_ticks(&fleet, &world, origin, dest), without);
    }
}
