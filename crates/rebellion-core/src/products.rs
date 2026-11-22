//! Unfinished products in their destination containers.
//!
//! The original creates every ordered unit at the order, Existing but not
//! Completed (`FUN_0052a140` -> `FUN_004f7d50(.., 1, 0)`), inside the
//! container the build planner picks (`FUN_00552300`): a capital ship in
//! the target system's bit-1 spare fleet, a regiment, squadron or special
//! force in the target system itself (`System_vt78`, `FUN_00507750`).
//! Completion (`FUN_0052bee0`) sets Completed and, when the yard's system
//! differs from the product's, starts its trip; the product never changes
//! container on completion. See `ghidra/notes/unfinished-products.md`.
//!
//! port: an unfinished unit is a [`Product`] record whose
//! [`Container`] names the fleet or system holding it, not a `ShipInstance`
//! or `TroopUnit` with a cleared flag. The walks the original makes in
//! mode 1 (Usable: battle `FUN_0040b0e0`, Retreat `FUN_00516730`, carriers
//! `FUN_00552300` step 3, fleet speed `FUN_004fd900`) read the port's
//! object lists, which keep only finished units; the walks in mode 3
//! (Existing: the Fleet window `FUN_004a3340`, the fleet icon
//! `FUN_004feea0`) read these records too. Completion turns a record into
//! the finished object (the Completed flag flips).
//!
//! A spare fleet that holds only unfinished ships is not Usable
//! (`FUN_004fe540`), so it is no presence for a battle or blockade (A0
//! 2026-11-15 run "u-" run 2: the Alliance took Corellia with no battle).
//! port: such a fleet is *staged*: it exists, numbered (A0 "u-2": "Fleet 4"
//! at once), at its system, but not in the system's fleet list, so every
//! mode-1 walk of `System::fleets` skips it. Its first finished ship puts it
//! in the list.
//!
//! Facilities stay as before (their pages read the queue,
//! `unfinished-facility.md`).

use serde::{Deserialize, Serialize};

use crate::ids::{FleetKey, SystemKey};
use crate::manufacturing::{
    BuildableKind, CompletionEvent, Departure, ManufacturingState, ProductionArea, QueueItem,
};
use crate::world::{Fleet, GameWorld};

/// What holds a product (`+0x1c` parent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Container {
    /// A system (regiments, squadrons, special forces).
    System(SystemKey),
    /// A fleet (capital ships, in the side's spare).
    Fleet(FleetKey),
}

/// One unfinished unit (`+0x50` Created, not Completed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    /// port: the product's identity, in order of creation.
    pub id: u64,
    pub kind: BuildableKind,
    /// The name its order gave it ([`QueueItem::name`]).
    pub name: Option<String>,
    /// Its side (`+0x24` bits 6..7).
    pub is_alliance: bool,
    /// The system whose manager builds it.
    pub yard: SystemKey,
    /// The system its order (or the last Destination) named.
    pub target: SystemKey,
    pub container: Container,
    /// The fleet its order (or the last Destination) named, when that was
    /// a fleet ([`ManufacturingState::destination_fleet`]). Saved by the
    /// save's last field (save v138), so skipped here.
    #[serde(skip)]
    pub target_fleet: Option<FleetKey>,
    /// The ship of [`Self::target_fleet`] its order named, when the
    /// Destination was a ship ([`ManufacturingState::set_ship_destination`]).
    #[serde(skip)]
    pub target_ship: Option<u64>,
    /// The capital ship (`ShipInstance::id`) a squadron or regiment placed
    /// in a fleet waits in ([`seat`]). Saved with the fleet targets.
    #[serde(skip)]
    pub ship: Option<u64>,
}

/// A product's fleet target and seat ([`Product::target_fleet`],
/// [`Product::target_ship`], [`Product::ship`]), for the save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductFleetTarget {
    pub id: u64,
    pub fleet: FleetKey,
    pub target_ship: Option<u64>,
    pub ship: Option<u64>,
}

/// Every unfinished product, in order of creation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Products {
    last_id: u64,
    items: Vec<Product>,
    /// The finished products still travelling from their yard to their
    /// container, by id, with the day each arrives (`+0x50` Completed and
    /// EnrouteActive, `+0x44` DestinationCount; `FUN_0052bee0`). It stays
    /// the last field: a v124 save gains an empty one (save v125).
    trips: Vec<(u64, u64)>,
    /// Unfinished products destroyed by an autoroute that found no place
    /// ([`autoroute_facilities`]), still waiting to leave their queues
    /// ([`destroy_lost`]). port: the world holds no queues, so the caller
    /// that holds them takes these out at its next manufacturing step,
    /// before [`reconcile`]. Not saved (no format change): a side change by
    /// occupation saved before that step keeps its product.
    #[serde(skip)]
    lost: Vec<u64>,
}

impl Products {
    /// Every product, oldest first.
    #[must_use]
    pub fn all(&self) -> &[Product] {
        &self.items
    }

    /// The product `id`.
    #[must_use]
    pub fn get(&self, id: u64) -> Option<&Product> {
        self.items.iter().find(|product| product.id == id)
    }

    /// The unfinished units `fleet` holds, oldest first.
    pub fn in_fleet(&self, fleet: FleetKey) -> impl Iterator<Item = &Product> {
        self.items
            .iter()
            .filter(move |product| product.container == Container::Fleet(fleet))
    }

    /// The unfinished units `system` itself holds, oldest first.
    pub fn in_system(&self, system: SystemKey) -> impl Iterator<Item = &Product> {
        self.items
            .iter()
            .filter(move |product| product.container == Container::System(system))
    }

    /// Whether `fleet` holds an unfinished unit.
    #[must_use]
    pub fn holds(&self, fleet: FleetKey) -> bool {
        self.in_fleet(fleet).next().is_some()
    }

    /// The finished products travelling to their containers, with the day
    /// each arrives.
    pub fn travelling(&self) -> impl Iterator<Item = (&Product, u64)> {
        self.trips
            .iter()
            .filter_map(|&(id, day)| self.get(id).map(|product| (product, day)))
    }

    /// The day product `id` arrives, while it travels finished to its
    /// container; none while it is being built.
    #[must_use]
    pub fn arrival(&self, id: u64) -> Option<u64> {
        self.trips
            .iter()
            .find(|&&(trip, _)| trip == id)
            .map(|&(_, day)| day)
    }

    /// Each product's fleet target ([`Product::target_fleet`]), by id, for
    /// the save.
    #[must_use]
    pub fn fleet_targets(&self) -> Vec<ProductFleetTarget> {
        self.items
            .iter()
            .filter_map(|product| {
                product.target_fleet.map(|fleet| ProductFleetTarget {
                    id: product.id,
                    fleet,
                    target_ship: product.target_ship,
                    ship: product.ship,
                })
            })
            .collect()
    }

    /// Put the saved fleet targets back ([`Self::fleet_targets`]).
    pub fn restore_fleet_targets(&mut self, targets: &[ProductFleetTarget]) {
        for product in &mut self.items {
            let saved = targets.iter().find(|target| target.id == product.id);
            product.target_fleet = saved.map(|target| target.fleet);
            product.target_ship = saved.and_then(|target| target.target_ship);
            product.ship = saved.and_then(|target| target.ship);
        }
    }

    fn take(&mut self, id: u64) -> Option<Product> {
        let at = self.items.iter().position(|product| product.id == id)?;
        self.trips.retain(|&(trip, _)| trip != id);
        Some(self.items.remove(at))
    }
}

/// Whether `kind` is a unit this module keeps as a product.
#[must_use]
pub const fn is_tracked(kind: BuildableKind) -> bool {
    // A facility too: it stands in its destination's facility list from
    // the order on, not yet completed (`unfinished-facility.md`).
    matches!(
        kind,
        BuildableKind::CapitalShip(_)
            | BuildableKind::Fighter(_)
            | BuildableKind::Troop(_)
            | BuildableKind::SpecialForce(_)
            | BuildableKind::DefenseFacility(_)
            | BuildableKind::ManufacturingFacility(_)
            | BuildableKind::ProductionFacility(_)
    )
}

/// The system a container stands at: a system itself, a fleet's location.
#[must_use]
pub fn container_system(world: &GameWorld, container: Container) -> Option<SystemKey> {
    match container {
        Container::System(system) => world.systems.contains_key(system).then_some(system),
        Container::Fleet(fleet) => world.fleets.get(fleet).map(|value| value.location),
    }
}

/// Whether `fleet` is in no system's fleet list though it stands at one.
fn unlisted(world: &GameWorld, fleet: FleetKey) -> bool {
    world.fleets.get(fleet).is_some_and(|value| {
        world
            .systems
            .get(value.location)
            .is_some_and(|system| !system.fleets.contains(&fleet))
    })
}

/// Whether `fleet` is staged: a spare that holds only unfinished ships and
/// so is in no system's fleet list (module docs).
#[must_use]
pub fn is_staged(world: &GameWorld, fleet: FleetKey) -> bool {
    world.products.holds(fleet) && unlisted(world, fleet)
}

/// The staged fleets at `system`, in creation order: each lists with the
/// system's fleets in the windows that walk in mode 3.
#[must_use]
pub fn staged_fleets_at(world: &GameWorld, system: SystemKey) -> Vec<FleetKey> {
    let mut found: Vec<FleetKey> = Vec::new();
    for product in &world.products.items {
        if let Container::Fleet(fleet) = product.container {
            if !found.contains(&fleet)
                && world
                    .fleets
                    .get(fleet)
                    .is_some_and(|f| f.location == system)
                && is_staged(world, fleet)
            {
                found.push(fleet);
            }
        }
    }
    found.sort_by_key(|&fleet| world.fleet_serial(fleet));
    found
}

fn is_alliance_of(world: &GameWorld, kind: BuildableKind, yard: SystemKey) -> bool {
    match kind {
        BuildableKind::CapitalShip(class) => world
            .capital_ship_classes
            .get(class)
            .is_some_and(|c| c.is_alliance),
        BuildableKind::Fighter(class) => world
            .fighter_classes
            .get(class)
            .is_some_and(|c| c.is_alliance),
        BuildableKind::Troop(class) | BuildableKind::SpecialForce(class) => {
            world.buildable_classes.get(&class).map_or_else(
                || {
                    world
                        .systems
                        .get(yard)
                        .and_then(|system| GameWorld::facility_holder(system.control))
                        == Some(crate::dat::Faction::Alliance)
                },
                |class| class.is_alliance,
            )
        }
        _ => false,
    }
}

/// The side's spare for built ships at `system`, staged or not
/// ([`GameWorld::ship_spare`] for a listed one).
#[must_use]
pub fn spare_at(world: &GameWorld, system: SystemKey, is_alliance: bool) -> Option<FleetKey> {
    world.ship_spare(system, is_alliance).or_else(|| {
        world
            .fleet_names
            .ship_spares
            .iter()
            .find(|&&(at, side, fleet)| {
                at == system
                    && side == is_alliance
                    && world
                        .fleets
                        .get(fleet)
                        .is_some_and(|f| f.location == system && f.is_alliance == is_alliance)
                    && unlisted(world, fleet)
            })
            .map(|&(_, _, fleet)| fleet)
    })
}

/// The container the planner gives a product of `kind` ordered for
/// `target` (`FUN_00552300`): a capital ship goes to the side's bit-1 spare
/// (`FUN_005097d0`), made now when there is none: the `0x300` handler
/// makes it real and numbers it at once (A0 2026-11-15 "u-2": a fleet icon
/// and "Fleet 4" right after the order). Everything else stays in the
/// system, which takes it without limit (`System_vt78`, `FUN_00507750`).
fn place(
    world: &mut GameWorld,
    kind: BuildableKind,
    target: SystemKey,
    is_alliance: bool,
) -> Container {
    if !matches!(kind, BuildableKind::CapitalShip(_)) {
        return Container::System(target);
    }
    if let Some(fleet) = spare_at(world, target, is_alliance) {
        return Container::Fleet(fleet);
    }
    // Unnumbered for now: the spare is numbered when the 0x300 handler
    // runs in the next tick (number_new_spares).
    let fleet = world.insert_fleet_unnamed(Fleet {
        location: target,
        is_alliance,
        ..Default::default()
    });
    world.set_ship_spare(target, is_alliance, fleet);
    Container::Fleet(fleet)
}

/// The container for a product of `kind` whose order named `target` (a
/// fleet, or one ship of it) at `system` (a shipyard's or training
/// manager's Destination, `FUN_00537180`), with the ship it waits in. The
/// planner (`FUN_00552300`) tries the target itself (`FUN_00552b10`),
/// then, for a fleet, its usable ships, keeping the best:
/// - a capital ship of the fleet's side goes into the fleet;
/// - a squadron or regiment goes aboard a ship with room ([`seat`]);
/// - a special force goes into the fleet (people take no room).
///
/// Without a target, or when it takes nothing, [`place`] decides as for
/// the fleet's system.
///
/// port: the original refuses such a Build at Confirm (`FUN_005385f0` on the
/// planner's error, `0x27`); the port places the unit at the fleet's system
/// instead.
fn place_for(
    world: &mut GameWorld,
    kind: BuildableKind,
    system: SystemKey,
    is_alliance: bool,
    target: Option<crate::manufacturing::FleetTarget>,
    carried: &dyn Fn(FleetKey) -> usize,
) -> (Container, Option<u64>) {
    let Some(target) = target else {
        return (place(world, kind, system, is_alliance), None);
    };
    let fleet = target.fleet;
    let own = world
        .fleets
        .get(fleet)
        .is_some_and(|value| value.is_alliance == is_alliance);
    let placed = match kind {
        BuildableKind::CapitalShip(_) if own => Some(None),
        BuildableKind::Fighter(_) | BuildableKind::Troop(_) if own => {
            seat(world, fleet, kind, carried(fleet), target.ship).map(Some)
        }
        BuildableKind::SpecialForce(_) => {
            room_aboard(world, fleet, kind, is_alliance, 0).then_some(None)
        }
        _ => None,
    };
    if let Some(ship) = placed {
        return (Container::Fleet(fleet), ship);
    }
    let at = world
        .fleets
        .get(fleet)
        .map_or(system, |value| value.location);
    (place(world, kind, at, is_alliance), None)
}

/// The room for `kind` on each living, numbered capital ship of `fleet`,
/// in the fleet's order: its fighter (`+0x26c`) or troop (`+0x270`)
/// capacity less what rides it, counted in mode 3 as `CapShip_vt78`
/// (`FUN_00500b40`) counts, unfinished units too.
///
/// port: the port carries regiments by fleet, so the `carried` regiments
/// are seated as the planner would have seated them one by one
/// ([`seat_regiments`]).
fn ship_rooms(
    world: &GameWorld,
    fleet: FleetKey,
    kind: BuildableKind,
    carried: usize,
) -> Vec<(u64, u32)> {
    ship_rooms_but(world, fleet, kind, carried, None)
}

/// [`ship_rooms`] with the products of `replaced`, the manager (system and
/// area) whose queue an order replaces, giving their room back: Build's
/// check (`FUN_005385f0` -> `FUN_00552210` -> `FUN_00552000` ->
/// `FUN_00552dd0` in mode 0) adds their need back before the new units
/// ask for theirs, as Confirm scraps them (`FUN_0052a2c0`).
#[must_use]
pub fn ship_rooms_but(
    world: &GameWorld,
    fleet: FleetKey,
    kind: BuildableKind,
    carried: usize,
    replaced: Option<(SystemKey, ProductionArea)>,
) -> Vec<(u64, u32)> {
    let Some(value) = world.fleets.get(fleet) else {
        return Vec::new();
    };
    let capacity = |ship: &crate::world::ShipInstance| {
        world
            .capital_ship_classes
            .get(ship.class)
            .map_or(0, |class| match kind {
                BuildableKind::Fighter(_) => class.fighter_capacity,
                _ => class.troop_capacity,
            })
    };
    let regiments = match kind {
        BuildableKind::Troop(_) => seat_regiments(world, fleet, carried),
        _ => Vec::new(),
    };
    value
        .capital_ships
        .iter()
        .filter(|ship| ship.alive && ship.id != 0)
        .map(|ship| {
            let aboard = match kind {
                BuildableKind::Fighter(_) => crate::squadrons::aboard(&value.fighters, ship.id),
                _ => regiments.iter().filter(|&&seat| seat == ship.id).count() as u32,
            };
            let building = world
                .products
                .in_fleet(fleet)
                .filter(|product| product.ship == Some(ship.id))
                .filter(|product| {
                    std::mem::discriminant(&product.kind) == std::mem::discriminant(&kind)
                })
                .filter(|product| {
                    replaced != Some((product.yard, ProductionArea::of(product.kind)))
                })
                .count() as u32;
            (ship.id, capacity(ship).saturating_sub(aboard + building))
        })
        .collect()
}

/// The ship a squadron or regiment of `kind` goes aboard in `fleet`
/// (`FUN_00552300`): the named ship when the Destination names one and it
/// has room, else the ship with the most room left, the earlier one on a
/// tie. Each ship is weighed by `FUN_00552b10` and a later one wins only
/// when strictly better (`FUN_0054fbb0`: no refusal, then more room `+8`,
/// the room `CapShip_vt78` gives, capacity less what is aboard). None has
/// room: `None` (`0x27`).
///
/// A0 2026-11-21-build-dest-16/12, 15, 17: Fleet 1 held an Imperial Star
/// Destroyer (troop capacity 3) and Victory Destroyer 2 (2) and four
/// regiments; the fifth, built for Fleet 1, went aboard Victory Destroyer
/// 2.
#[must_use]
pub fn seat(
    world: &GameWorld,
    fleet: FleetKey,
    kind: BuildableKind,
    carried: usize,
    named: Option<u64>,
) -> Option<u64> {
    let rooms = ship_rooms(world, fleet, kind, carried);
    if let Some(ship) = named {
        if rooms.iter().any(|&(id, room)| id == ship && room > 0) {
            return Some(ship);
        }
    }
    best_room(&rooms)
}

fn best_room(rooms: &[(u64, u32)]) -> Option<u64> {
    rooms
        .iter()
        .filter(|&&(_, room)| room > 0)
        .fold(None, |best: Option<(u64, u32)>, &candidate| match best {
            Some(held) if candidate.1 <= held.1 => Some(held),
            _ => Some(candidate),
        })
        .map(|(ship, _)| ship)
}

/// The ships `carried` regiments of `fleet` ride, in order: each placed as
/// the planner places a regiment, on the ship with the most troop room
/// left (`FUN_00552300`, [`seat`]).
///
/// port: the port keeps a fleet's regiments as one cargo list, not per
/// ship; seating them again in order gives the ships the original's
/// placement gave, as long as every one went aboard by the planner and no
/// ship joined or left since.
#[must_use]
pub fn seat_regiments(world: &GameWorld, fleet: FleetKey, carried: usize) -> Vec<u64> {
    let Some(value) = world.fleets.get(fleet) else {
        return Vec::new();
    };
    let mut rooms: Vec<(u64, u32)> = value
        .capital_ships
        .iter()
        .filter(|ship| ship.alive && ship.id != 0)
        .map(|ship| {
            let capacity = world
                .capital_ship_classes
                .get(ship.class)
                .map_or(0, |class| class.troop_capacity);
            (ship.id, capacity)
        })
        .collect();
    let mut seats = Vec::with_capacity(carried);
    for _ in 0..carried {
        let Some(ship) = best_room(&rooms) else {
            break;
        };
        if let Some(room) = rooms.iter_mut().find(|(id, _)| *id == ship) {
            room.1 -= 1;
        }
        seats.push(ship);
    }
    seats
}

/// The name of the ship numbered `ship` in `fleet`: its own, else its
/// class's (`FUN_004f62d0`).
#[must_use]
pub fn ship_name(world: &GameWorld, fleet: FleetKey, ship: u64) -> Option<&str> {
    let value = world.fleets.get(fleet)?;
    let index = value
        .capital_ships
        .iter()
        .position(|value| value.id == ship)?;
    world.ship_name(fleet, index).or_else(|| {
        world
            .capital_ship_classes
            .get(value.capital_ships[index].class)
            .map(|class| class.name.as_str())
    })
}

/// The system `system`'s `area` delivers to: where its fleet Destination
/// stands now, else its Destination system, else its own system. The
/// transit to a fleet ends at the fleet's system (`FUN_00555b30` takes
/// both ends' systems, `FUN_00555540`).
#[must_use]
pub fn delivery_system(
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    system: SystemKey,
    area: ProductionArea,
) -> SystemKey {
    manufacturing
        .destination_fleet(system, area)
        .and_then(|fleet| world.fleets.get(fleet))
        .map_or_else(
            || manufacturing.destination(system, area).unwrap_or(system),
            |fleet| fleet.location,
        )
}

/// The `0x300` handler (`Event300_GameObjExistingChanged_Handle`,
/// `FUN_005140c0`) runs in the tick after the order: the product's
/// Existing change makes its spare real (`FUN_004fe630`) and numbers it
/// (`FUN_00517760`). Until then the spare bears the record's "Fleet", as
/// the band names it right after the order (A0 2026-11-15 "m-1": "Destination:
/// Fleet" on day 11; "m-2": "Fleet 4" by day 15; A0 66 likewise).
pub fn number_new_spares(world: &mut GameWorld) {
    let mut fleets: Vec<FleetKey> = Vec::new();
    for product in &world.products.items {
        if let Container::Fleet(fleet) = product.container {
            if !fleets.contains(&fleet) && !world.fleet_is_named(fleet) {
                fleets.push(fleet);
            }
        }
    }
    fleets.sort_by_key(|&fleet| world.fleet_serial(fleet));
    for fleet in fleets {
        world.name_new_fleet(fleet);
    }
}

/// The container holding `system`'s current ship product: the ship
/// manager's DeploymentKey (`+0x78`, `FUN_0052ac40`: the product's
/// container), which the band names (`FUN_004591b0`). It follows the
/// product: A0 2026-11-15 "m-2": "Destination: Fleet 3" once the Carrack
/// was dropped on Fleet 3.
#[must_use]
pub fn deployment_fleet(world: &GameWorld, system: SystemKey) -> Option<FleetKey> {
    match current_ship_product(world, system)?.container {
        Container::Fleet(fleet) => Some(fleet),
        Container::System(_) => None,
    }
}

/// A staged fleet left with nothing goes (A0 2026-11-15 "u-3": the
/// replaced Carrack's Fleet 4 and its icon disappeared).
fn drop_if_emptied(world: &mut GameWorld, container: Container) {
    let Container::Fleet(fleet) = container else {
        return;
    };
    if world.products.holds(fleet) || !unlisted(world, fleet) {
        return;
    }
    let Some(value) = world.fleets.get(fleet) else {
        return;
    };
    if value.capital_ships.is_empty()
        && value.fighters.is_empty()
        && value.characters.is_empty()
        && value.special_forces.is_empty()
        && !value.has_death_star
    {
        world.fleets.remove(fleet);
        world.forget_ship_spare(fleet);
    }
}

fn remove(world: &mut GameWorld, id: u64) -> Option<Product> {
    let product = world.products.take(id)?;
    drop_if_emptied(world, product.container);
    Some(product)
}

/// Make the products match the queues: a queued unit with no product gets
/// one, placed now (`FUN_0052a140`); a product whose unit left its queue
/// (replaced by a new order, stopped, scrapped: `FUN_00529ab0` ->
/// `FUN_0052a2c0`) goes, with a staged spare it empties; a product whose
/// Destination changed (`0x214`, `FUN_0052a380`) moves to the new target's
/// container. A product whose fleet is gone is placed again.
///
/// Idempotent; queues are taken in key order so the ids are deterministic.
pub fn reconcile(world: &mut GameWorld, manufacturing: &ManufacturingState) {
    reconcile_carrying(world, manufacturing, &|_| 0);
}

/// [`reconcile`] with each fleet's `carried` regiments, which seat a
/// regiment built for the fleet ([`seat`]).
pub fn reconcile_carrying(
    world: &mut GameWorld,
    manufacturing: &ManufacturingState,
    carried: &dyn Fn(FleetKey) -> usize,
) {
    let mut keys: Vec<(SystemKey, ProductionArea)> =
        manufacturing.queues().keys().copied().collect();
    keys.sort_by_key(|&(system, area)| (system, area as u8));

    // Products of a queue that no longer exists. A product on its trip
    // has left its queue (FUN_0052bee0 completed it).
    let orphans: Vec<u64> = world
        .products
        .items
        .iter()
        .filter(|product| world.products.arrival(product.id).is_none())
        .filter(|product| !keys.contains(&(product.yard, ProductionArea::of(product.kind))))
        .map(|product| product.id)
        .collect();
    for id in orphans {
        remove(world, id);
    }

    for (yard, area) in keys {
        let items: Vec<QueueItem> = manufacturing
            .queue(yard, area)
            .map(|queue| {
                queue
                    .items()
                    .iter()
                    .filter(|item| is_tracked(item.kind))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let held: Vec<u64> = world
            .products
            .items
            .iter()
            .filter(|p| p.yard == yard && ProductionArea::of(p.kind) == area)
            .filter(|p| world.products.arrival(p.id).is_none())
            .map(|p| p.id)
            .collect();
        let same = |product: &Product, item: &QueueItem| {
            product.kind == item.kind && product.name == item.name
        };
        let mut kept = 0;
        while kept < held.len()
            && kept < items.len()
            && world
                .products
                .get(held[kept])
                .is_some_and(|product| same(product, &items[kept]))
        {
            kept += 1;
        }
        for &id in &held[kept..] {
            remove(world, id);
        }
        // The manager's fleet Destination applies to the units that take
        // its Destination system (set_destination rewrites them all).
        let fleet_target = |item: &QueueItem| {
            manufacturing
                .destination_target(yard, area)
                .filter(|_| item.destination == manufacturing.destination(yard, area))
        };
        // Destination changes and lost fleets.
        for (&id, item) in held[..kept].iter().zip(&items) {
            let target = item.destination.unwrap_or(yard);
            let fleet_target = fleet_target(item);
            let target_fleet = fleet_target.map(|value| value.fleet);
            let target_ship = fleet_target.and_then(|value| value.ship);
            let Some(product) = world.products.get(id).cloned() else {
                continue;
            };
            let lost = container_system(world, product.container).is_none();
            if product.target == target
                && product.target_fleet == target_fleet
                && product.target_ship == target_ship
                && !lost
            {
                continue;
            }
            let Some(mut product) = world.products.take(id) else {
                continue;
            };
            if !lost {
                drop_if_emptied(world, product.container);
            }
            product.target = target;
            product.target_fleet = target_fleet;
            product.target_ship = target_ship;
            (product.container, product.ship) = place_for(
                world,
                product.kind,
                target,
                product.is_alliance,
                fleet_target,
                carried,
            );
            let at = world.products.items.partition_point(|p| p.id < id);
            world.products.items.insert(at, product);
        }
        for item in &items[kept..] {
            if !world.systems.contains_key(yard) {
                break;
            }
            let target = item.destination.unwrap_or(yard);
            let fleet_target = fleet_target(item);
            let is_alliance = is_alliance_of(world, item.kind, yard);
            let (container, ship) =
                place_for(world, item.kind, target, is_alliance, fleet_target, carried);
            world.products.last_id += 1;
            let id = world.products.last_id;
            world.products.items.push(Product {
                id,
                kind: item.kind,
                name: item.name.clone(),
                is_alliance,
                yard,
                target,
                container,
                target_fleet: fleet_target.map(|value| value.fleet),
                target_ship: fleet_target.and_then(|value| value.ship),
                ship,
            });
        }
    }
}

/// Where a finished product goes, for the caller to apply.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settled {
    /// Products finished where they stand.
    pub completions: Vec<CompletionEvent>,
    /// Units with no product, finished away from their destination,
    /// which travel there as deliveries.
    pub departures: Vec<Departure>,
    /// Products finished away from their container: each stays in it,
    /// travelling until its arrival ([`Products::arrival`]); listed here
    /// for the departure's message.
    pub trips: Vec<Departure>,
}

/// The oldest product of `yard` matching a finished `kind` and `name`.
fn finished(
    world: &GameWorld,
    yard: SystemKey,
    kind: BuildableKind,
    name: &Option<String>,
) -> Option<u64> {
    world
        .products
        .items
        .iter()
        .find(|p| {
            p.yard == yard
                && p.kind == kind
                && &p.name == name
                && world.products.arrival(p.id).is_none()
        })
        .map(|p| p.id)
}

/// Finish the products a manufacturing advance completed (`FUN_0052bee0`).
/// A product finishes in its container: where that stands at the yard's
/// system it completes there, in that container (a ship in its fleet);
/// elsewhere it travels from the yard to its container's system (EnrouteActive).
/// The product was placed when it was ordered, and may have moved since
/// (its fleet moved, or a Move carried it). A unit this module does not
/// keep passes through unchanged.
pub fn settle(
    world: &mut GameWorld,
    completions: Vec<CompletionEvent>,
    departures: Vec<Departure>,
) -> Settled {
    let mut out = Settled::default();
    let finish = |world: &mut GameWorld,
                  yard: SystemKey,
                  tick: u64,
                  kind: BuildableKind,
                  name: Option<String>,
                  fallback: SystemKey,
                  out: &mut Settled| {
        let id = finished(world, yard, kind, &name);
        // FUN_0052bee0: a product whose container stands away from its
        // yard is Completed and EnrouteActive in that container; it
        // completes there on arrival (FUN_004fba10 -> FUN_00556620).
        if let Some(id) = id {
            let product = world.products.get(id).cloned();
            if let Some(product) = product {
                // A facility too: its container is its destination system
                // (unfinished-facility.md, bit 4 "Enroute").
                if let Some(at) = container_system(world, product.container) {
                    if at != yard {
                        let days = crate::delivery::transit_days(world, kind, yard, at);
                        world.products.trips.push((id, tick + u64::from(days)));
                        out.trips.push(Departure {
                            origin: yard,
                            destination: at,
                            tick,
                            kind,
                            name,
                        });
                        return;
                    }
                }
            }
        }
        let product = id.and_then(|id| world.products.take(id));
        let (at, fleet) = match &product {
            Some(product) => (
                container_system(world, product.container).unwrap_or(fallback),
                match product.container {
                    Container::Fleet(fleet) => Some(fleet),
                    Container::System(_) => None,
                },
            ),
            None => (fallback, None),
        };
        if at == yard {
            out.completions.push(CompletionEvent {
                system: at,
                tick,
                kind,
                name,
                fleet,
            });
        } else {
            out.departures.push(Departure {
                origin: yard,
                destination: at,
                tick,
                kind,
                name,
            });
        }
    };
    for completion in completions {
        let yard = completion.system;
        finish(
            world,
            yard,
            completion.tick,
            completion.kind,
            completion.name,
            yard,
            &mut out,
        );
    }
    for departure in departures {
        finish(
            world,
            departure.origin,
            departure.tick,
            departure.kind,
            departure.name,
            departure.destination,
            &mut out,
        );
    }
    out
}

/// [`settle`] on an advance's completions and departures, in place.
pub fn settle_advance(
    world: &mut GameWorld,
    advance: &mut crate::manufacturing::ManufacturingAdvance,
) {
    let settled = settle(
        world,
        std::mem::take(&mut advance.completions),
        std::mem::take(&mut advance.departures),
    );
    advance.completions = settled.completions;
    advance.departures = settled.departures;
    advance.trips = settled.trips;
}

/// What the day's arrivals of finished products give the caller.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Arrived {
    /// Products that reached their container, as completions there.
    pub completions: Vec<CompletionEvent>,
    /// Products whose container's system is gone: they go on as
    /// deliveries from there, which reroute (`crate::delivery`,
    /// `FUN_004f8710` sets Autorouting).
    pub departures: Vec<Departure>,
}

/// The arrival (event `0x387` -> `FUN_004fba10` -> `FUN_00556620`): each
/// finished product whose trip ends by `now` clears EnrouteActive and is
/// usable in its container. port: one that stands in a fleet completes
/// wherever the fleet is that day.
pub fn arrive(world: &mut GameWorld, now: u64) -> Arrived {
    let mut out = Arrived::default();
    let due: Vec<(u64, u64)> = world
        .products
        .trips
        .iter()
        .copied()
        .filter(|&(_, day)| day <= now)
        .collect();
    for (id, day) in due {
        let Some(product) = world.products.take(id) else {
            continue;
        };
        let at = container_system(world, product.container);
        let fleet = match product.container {
            Container::Fleet(fleet) => Some(fleet),
            Container::System(_) => None,
        };
        match at {
            Some(at) if world.systems.get(at).is_some_and(|s| !s.is_destroyed) => {
                out.completions.push(CompletionEvent {
                    system: at,
                    tick: day,
                    kind: product.kind,
                    name: product.name,
                    fleet,
                });
            }
            _ => {
                let from = at.unwrap_or(product.target);
                out.departures.push(Departure {
                    origin: from,
                    destination: from,
                    tick: day,
                    kind: product.kind,
                    name: product.name,
                });
                if let Some(fleet) = fleet {
                    drop_if_emptied(world, Container::Fleet(fleet));
                }
            }
        }
    }
    out
}

/// A finished regiment, squadron or special force of the side
/// `is_alliance` travelling into `system` (Completed, EnrouteActive, its
/// container the system): the defenses icon's walks hold it, and its own
/// Move refuses while it travels (`FUN_004f9860` code 4;
/// `DeliveryState::surface_arrival_pending` for one with no product).
#[must_use]
pub fn surface_arrival_pending(world: &GameWorld, system: SystemKey, is_alliance: bool) -> bool {
    world.products.travelling().any(|(product, _)| {
        product.container == Container::System(system)
            && product.is_alliance == is_alliance
            && matches!(
                product.kind,
                BuildableKind::Troop(_)
                    | BuildableKind::Fighter(_)
                    | BuildableKind::SpecialForce(_)
            )
    })
}

/// A regiment, squadron or defense facility of the side `is_alliance` in
/// `system` that is unfinished or travelling in. The defenses icon's Scrap
/// team (`FUN_00512700` kind 8, order `0x200`: `FUN_00504c40`,
/// `FUN_00503a50`, `FUN_00527050`, each in mode 3) holds it, and its scrap
/// check `FUN_004f9860` refuses status 2 (not completed) and 4 (en route)
/// (`ghidra/notes/scrap-order.md`), so the icon's Scrap is refused. A0
/// 2026-11-19-edited-saves-10 c-01 (queued) and c-05 (en route).
#[must_use]
pub fn defenses_scrap_refused(world: &GameWorld, system: SystemKey, is_alliance: bool) -> bool {
    world.products.in_system(system).any(|product| {
        product.is_alliance == is_alliance
            && matches!(
                product.kind,
                BuildableKind::Troop(_)
                    | BuildableKind::Fighter(_)
                    | BuildableKind::DefenseFacility(_)
            )
    })
}

/// The current product of `system`'s ship manager (`FUN_00509670(system,
/// 0)`, its ProductKey `+0x74`): the oldest of its products still being
/// built, wherever it stands.
#[must_use]
pub fn current_ship_product(world: &GameWorld, system: SystemKey) -> Option<&Product> {
    current_product(world, system, ProductionArea::Shipyard)
}

/// The current product of `system`'s `area` manager (its ProductKey
/// `+0x74`): the oldest of its products still being built.
#[must_use]
pub fn current_product(
    world: &GameWorld,
    system: SystemKey,
    area: ProductionArea,
) -> Option<&Product> {
    world.products.items.iter().find(|product| {
        product.yard == system
            && ProductionArea::of(product.kind) == area
            && world.products.arrival(product.id).is_none()
    })
}

/// The ship a regiment completed aboard `completion`'s fleet boarded, as
/// [`board`] put it there: the last of the fleet's regiments, seated as
/// the planner seats them ([`seat_regiments`]). None for any other unit.
#[must_use]
pub fn boarded_ship<'w>(
    world: &'w GameWorld,
    transport: &crate::troop_transport::TroopTransportState,
    completion: &CompletionEvent,
) -> Option<&'w str> {
    let fleet = completion.fleet?;
    if !matches!(completion.kind, BuildableKind::Troop(_)) {
        return None;
    }
    let ship = *seat_regiments(world, fleet, transport.carried_count(fleet)).last()?;
    ship_name(world, fleet, ship)
}

/// The ship a finished squadron or regiment travelling to its fleet waits
/// in ([`Product::ship`]), for its departure's message.
#[must_use]
pub fn travelling_ship<'w>(world: &'w GameWorld, departure: &Departure) -> Option<&'w str> {
    let (product, _) = world.products.travelling().find(|(product, _)| {
        product.yard == departure.origin
            && product.kind == departure.kind
            && product.name == departure.name
            && container_system(world, product.container) == Some(departure.destination)
    })?;
    let Container::Fleet(fleet) = product.container else {
        return None;
    };
    ship_name(world, fleet, product.ship?)
}

/// The ship `system`'s `area` names while its current product waits
/// seated in a fleet: the manager's DeploymentKey (`+0x78`) is the
/// product's container (`FUN_0052ac40`), a capital ship for a regiment or
/// squadron (A0 2026-11-21-build-dest-16/12: "Destination: Victory
/// Destroyer 2" once the regiment for Fleet 1 was confirmed).
#[must_use]
pub fn deployment_ship(world: &GameWorld, system: SystemKey, area: ProductionArea) -> Option<&str> {
    let product = current_product(world, system, area)?;
    let Container::Fleet(fleet) = product.container else {
        return None;
    };
    ship_name(world, fleet, product.ship?)
}

/// `FUN_00487cc0` for an unfinished unit's Move (`0x201`): it asks when
/// the first member's container is a system blockaded (`+0x88` bit
/// `0x20`) and held by the member's side. An unfinished ship stands in a
/// fleet, so its Move never asks. (A Confirmed Move, `0x202`, always asks.)
#[must_use]
pub fn move_asks(world: &GameWorld, products: &[u64]) -> bool {
    let Some(first) = products.first().and_then(|&id| world.products.get(id)) else {
        return false;
    };
    let Container::System(system) = first.container else {
        return false;
    };
    world.systems.get(system).is_some_and(|value| {
        crate::blockade::BlockadeSystem::system_is_blockaded(world, value)
            && GameWorld::facility_holder(value.control)
                == Some(crate::dat::Faction::of_alliance(first.is_alliance))
    })
}

/// Whether `kind` is a facility (families `0x20..0x2f`).
#[must_use]
pub const fn is_facility(kind: BuildableKind) -> bool {
    matches!(
        kind,
        BuildableKind::DefenseFacility(_)
            | BuildableKind::ManufacturingFacility(_)
            | BuildableKind::ProductionFacility(_)
    )
}

/// What a holder change did to the facility products of a system
/// ([`autoroute_facilities`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Autorouted {
    /// Products put in another system of their side, with that system.
    pub moved: Vec<(u64, SystemKey)>,
    /// Products en route that found no place: destroyed, with no message.
    pub removed: Vec<u64>,
    /// Unfinished products that found no place: destroyed, with no message,
    /// once [`destroy_lost`] takes them from their queues.
    pub destroyed: Vec<u64>,
}

/// The facility side a product of `kind` is built for ([`FacilityBuild`]).
const fn facility_side(kind: BuildableKind) -> Option<bool> {
    match kind {
        BuildableKind::DefenseFacility(build)
        | BuildableKind::ManufacturingFacility(build)
        | BuildableKind::ProductionFacility(build) => Some(build.is_alliance),
        _ => None,
    }
}

/// The facilities `site` holds or will hold, as (taken, mines taken):
/// built ones and facility products, unfinished or on their way
/// (`System_vt78`, `FUN_00507750`, walks mode 3: `FUN_00539d70` and
/// `FUN_0052d610`).
fn facility_slots_taken(world: &GameWorld, site: SystemKey) -> (u32, u32) {
    let Some(system) = world.systems.get(site) else {
        return (0, 0);
    };
    let built = system
        .defense_facilities
        .iter()
        .filter(|key| world.defense_facilities.contains_key(**key))
        .map(|_| false)
        .chain(
            system
                .manufacturing_facilities
                .iter()
                .filter(|key| world.manufacturing_facilities.contains_key(**key))
                .map(|_| false),
        )
        .chain(
            system
                .production_facilities
                .iter()
                .filter_map(|key| world.production_facilities.get(*key))
                .map(|facility| facility.is_mine),
        );
    let products = world
        .products
        .in_system(site)
        .filter(|product| is_facility(product.kind))
        .map(|product| is_mine(product.kind));
    built.chain(products).fold((0, 0), |(all, mines), mine| {
        (all + 1, mines + u32::from(mine))
    })
}

/// Whether `kind` is a mine (PROFACSD family `0x2c`), which also takes a
/// raw-material slot (`FUN_0054f5b0` row `0x2c..0x2d`).
fn is_mine(kind: BuildableKind) -> bool {
    matches!(kind, BuildableKind::ProductionFacility(build) if build.class.family() == 0x2c)
}

/// The autoroute search (`FUN_005545e0`) for a facility of the side
/// `is_alliance` from `origin`: the nearest system that takes it.
///
/// Every candidate passes the move validator (`FUN_005571c0`), where the
/// system's `vt74` (`System_vt74`, `FUN_005073d0`) refuses a system of
/// another side (`FUN_00553410`, `{1, 0x24}`) or a blockaded one
/// (`{0x90, 4}`); it clears those refusals only for a fleet (`0x08..0x10`)
/// or a character (order `0x240`), never for a facility (`0x20..0x30`).
/// Its `vt78` (`System_vt78`, `FUN_00507750`) gives the room: energy
/// (`+0x5c`) less the facilities there, and for a mine raw materials
/// (`+0x64`) less the mines there. So the neutral (`FUN_005551e0`) and
/// enemy (`FUN_005552e0`) steps never place a facility, nor does the step
/// of own fleets (`FUN_00554f80`; a fleet takes no facility). What is left
/// is the nearest unblockaded system of its side with room
/// (`FUN_00554e80`), nearest by the `FUN_0055d860` distance.
fn facility_place(
    world: &GameWorld,
    is_alliance: bool,
    origin: (i64, i64),
    kind: BuildableKind,
) -> Option<SystemKey> {
    use crate::world::ControlKind;
    let side = crate::dat::Faction::of_alliance(is_alliance);
    let mut nearest: Vec<(i64, SystemKey)> = world
        .systems
        .iter()
        .filter(|(_, s)| !s.is_destroyed)
        .filter(|(_, s)| {
            matches!(s.control, ControlKind::Controlled(f) | ControlKind::Uprising(f) if f == side)
        })
        .filter(|(_, s)| !crate::blockade::BlockadeSystem::system_is_blockaded(world, s))
        .map(|(key, s)| {
            let (dx, dy) = (i64::from(s.x) - origin.0, i64::from(s.y) - origin.1);
            (crate::movement::original_isqrt(dx * dx + dy * dy), key)
        })
        .collect();
    nearest.sort_by_key(|(distance, _)| *distance);
    nearest.into_iter().map(|(_, key)| key).find(|&key| {
        let system = &world.systems[key];
        let (taken, mines) = facility_slots_taken(world, key);
        u32::from(system.total_energy) > taken
            && (!is_mine(kind) || u32::from(system.raw_materials) > mines)
    })
}

/// A change of `system`'s holder to `holder` for the facility products in
/// it (`FUN_004fae40` walks the system in mode 3 and calls slot `+0xb0`,
/// `FUN_004f8680`, on each object). A facility that is not Completed, or
/// is EnrouteActive, is not handed over: `FUN_004f8680` sets Autorouting
/// (`FUN_004f78e0(1)`), and the `0x307` handler
/// (`Event307_GameObjAutoroutingChanged_Handle`, `FUN_00515480`) runs the
/// autoroute search (`FUN_005545e0`, [`facility_place`]):
/// - found: `FUN_00556390` moves it there. An unfinished one is not
///   Deployed (slot `+0x38`), so it changes container at once and its
///   completion starts the trip from its yard. One en route keeps
///   travelling (EnrouteActive stays set); its pending arrival `0x387`
///   still fires at the old day, and `FUN_00556620` then starts the leg on
///   to its new container. port: the new arrival day is the old one plus
///   the transit from `system` to the new place.
/// - not found: slot `+0xac(2)` (`FUN_004f8660`), destroyed with reason 2.
///   No Facility Lost: `FUN_004fc080` raises `0x303` only for an object
///   EnrouteActive whose destination-change count (`+0x42`, raised by a
///   container change while en route, `GameObj_vtf4`, `FUN_004faf50`;
///   cleared on arrival, `FUN_004fb760`) is not 0, so a first trip raises
///   none. A trip an earlier autoroute moved has the count at 1 (the move
///   is slot `+0xa8`, `FUN_004f6fd0`, which calls slot `+0xf4`), yet no
///   `0x303` ever becomes a message: its UI record (class `0x00659778`)
///   has `ret 4` at slot `+0x1c`, so Facility Lost (`FUN_00497690`) is
///   never fed. A0 2026-11-18 edited saves part 8 (f-05..f-09) agrees.
///
///   An unfinished one is not EnrouteActive, so it raises no `0x303`
///   either; its manager no longer takes it as its current product
///   (`FUN_00528890` wants `+0x50` bits `0x1c` clear, Destroyed `0x8`
///   among them). port: it waits in the products' lost list until
///   [`destroy_lost`] takes its queued unit and it out together.
///
/// A product of the holder's side stays (the search's first step,
/// `FUN_00554740`, accepts its own container).
pub fn autoroute_facilities(
    world: &mut GameWorld,
    system: SystemKey,
    holder: crate::dat::Faction,
) -> Autorouted {
    let mut out = Autorouted::default();
    let Some(origin) = world
        .systems
        .get(system)
        .map(|s| (i64::from(s.x), i64::from(s.y)))
    else {
        return out;
    };
    let ids: Vec<u64> = world
        .products
        .in_system(system)
        .filter(|product| {
            facility_side(product.kind)
                .is_some_and(|is_alliance| crate::dat::Faction::of_alliance(is_alliance) != holder)
        })
        .map(|product| product.id)
        .collect();
    for id in ids {
        let Some(product) = world.products.get(id).cloned() else {
            continue;
        };
        let Some(is_alliance) = facility_side(product.kind) else {
            continue;
        };
        let arrival = world.products.arrival(id);
        match facility_place(world, is_alliance, origin, product.kind) {
            Some(place) => {
                if let Some(day) = arrival {
                    let days = crate::delivery::transit_days(world, product.kind, system, place);
                    for trip in &mut world.products.trips {
                        if trip.0 == id {
                            trip.1 = day + u64::from(days);
                        }
                    }
                }
                if let Some(value) = world.products.items.iter_mut().find(|p| p.id == id) {
                    value.container = Container::System(place);
                }
                out.moved.push((id, place));
            }
            None if arrival.is_some() => {
                world.products.take(id);
                out.removed.push(id);
            }
            None => {
                if !world.products.lost.contains(&id) {
                    world.products.lost.push(id);
                }
                out.destroyed.push(id);
            }
        }
    }
    out
}

/// The unfinished products an autoroute destroyed ([`autoroute_facilities`],
/// slot `+0xac(2)` in `FUN_00515480`) leave their manager's queue and the
/// world ([`destroy`]). Returns the ids taken out.
pub fn destroy_lost(world: &mut GameWorld, manufacturing: &mut ManufacturingState) -> Vec<u64> {
    let lost = std::mem::take(&mut world.products.lost);
    lost.into_iter()
        .filter(|&id| destroy(world, manufacturing, id))
        .collect()
}

/// The queued unit product `id` stands for: its yard, area and index.
fn queue_index(
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    id: u64,
) -> Option<(SystemKey, ProductionArea, usize)> {
    if world.products.arrival(id).is_some() {
        return None;
    }
    let product = world.products.get(id)?;
    let area = ProductionArea::of(product.kind);
    // Its place among the queue's units this module keeps.
    let ordinal = world
        .products
        .items
        .iter()
        .filter(|p| p.yard == product.yard && ProductionArea::of(p.kind) == area)
        .filter(|p| world.products.arrival(p.id).is_none())
        .position(|p| p.id == id)?;
    let index = manufacturing.queue(product.yard, area).and_then(|queue| {
        queue
            .items()
            .iter()
            .enumerate()
            .filter(|(_, item)| is_tracked(item.kind))
            .nth(ordinal)
            .map(|(index, _)| index)
    })?;
    Some((product.yard, area, index))
}

/// An unfinished unit destroyed where it stands (the disaster
/// `FUN_00511930` walks a system's facilities in mode 3, unfinished ones
/// too): it leaves its manager's queue and the world.
pub fn destroy(world: &mut GameWorld, manufacturing: &mut ManufacturingState, id: u64) -> bool {
    let Some((yard, area, index)) = queue_index(world, manufacturing, id) else {
        return false;
    };
    if manufacturing.queue_mut(yard, area).cancel(index).is_none() {
        return false;
    }
    remove(world, id).is_some()
}

/// Rename (`0x203`, `FUN_00429350`) an unfinished unit: its name
/// (`+0x34`) changes, and with it its queued unit's, which names the
/// finished object (A0 2026-11-15 "u-2": the unfinished Carrack's Rename
/// white). An object en route takes no order (`FUN_004f9860`).
pub fn rename(
    world: &mut GameWorld,
    manufacturing: &mut ManufacturingState,
    id: u64,
    name: String,
) -> bool {
    let Some((yard, area, index)) = queue_index(world, manufacturing, id) else {
        return false;
    };
    if !manufacturing
        .queue_mut(yard, area)
        .rename(index, name.clone())
    {
        return false;
    }
    if let Some(value) = world.products.items.iter_mut().find(|p| p.id == id) {
        value.name = Some(name);
    }
    true
}

/// Move: an unfinished unit is not Deployed, so not mobile (slot `+0x38`),
/// and `FUN_00556390` puts it in its new container at once, with no trip
/// (`unfinished-products.md`, "Moving an unfinished squadron or
/// regiment"). Its completion then starts the trip from its yard.
pub fn move_to(world: &mut GameWorld, id: u64, container: Container) -> bool {
    // An object en route takes no order (FUN_004f9860).
    if container_system(world, container).is_none() || world.products.arrival(id).is_some() {
        return false;
    }
    let Some(index) = world.products.items.iter().position(|p| p.id == id) else {
        return false;
    };
    let old = world.products.items[index].container;
    world.products.items[index].container = container;
    drop_if_emptied(world, old);
    true
}

/// The Move of a staged spare (its only members unfinished): it is not
/// mobile (`FUN_004fda10`), so `FUN_00556390` changes its container at
/// once, with no trip, carrying its products; leaving, it stops being its
/// system's spare (`FUN_004ff570`). A0 2026-11-15 "m-2": the spare holding
/// the unfinished Carrack moved to Talus; the build stayed at Corfai,
/// naming it.
pub fn relocate_staged(world: &mut GameWorld, fleet: FleetKey, destination: SystemKey) -> bool {
    if !is_staged(world, fleet) || !world.systems.contains_key(destination) {
        return false;
    }
    if let Some(value) = world.fleets.get_mut(fleet) {
        value.location = destination;
    }
    world.forget_ship_spare(fleet);
    true
}

/// A Move released on `system`, or on `fleet` there: a regiment, squadron
/// or special force goes to the system ([`move_to`]), or aboard the fleet
/// when it has room ([`room_aboard`]). hyp: an
/// unfinished capital ship goes into `fleet` when it is of its side, else
/// into the side's spare at `system`, as an order for that system would
/// place it (`FUN_00552300`); `FUN_00556390` moves it at once, not being
/// Deployed. A0 2026-11-15 "u-2" shows its Move white; where it goes was
/// not captured.
pub fn move_to_target(
    world: &mut GameWorld,
    id: u64,
    system: SystemKey,
    fleet: Option<FleetKey>,
    carried_regiments: usize,
) -> bool {
    let Some(product) = world.products.get(id).cloned() else {
        return false;
    };
    if !world.systems.contains_key(system) || world.products.arrival(id).is_some() {
        return false;
    }
    let container = match (product.kind, fleet) {
        (BuildableKind::CapitalShip(_), Some(fleet))
            if world
                .fleets
                .get(fleet)
                .is_some_and(|f| f.is_alliance == product.is_alliance) =>
        {
            Container::Fleet(fleet)
        }
        (BuildableKind::CapitalShip(_), _) => {
            if let Some(spare) = spare_at(world, system, product.is_alliance) {
                Container::Fleet(spare)
            } else {
                // The old spare goes first when this empties it.
                let Some(mut moving) = world.products.take(id) else {
                    return false;
                };
                drop_if_emptied(world, moving.container);
                moving.container = place(world, moving.kind, system, moving.is_alliance);
                let at = world.products.items.partition_point(|p| p.id < id);
                world.products.items.insert(at, moving);
                return true;
            }
        }
        (kind, Some(fleet)) => {
            if !room_aboard(world, fleet, kind, product.is_alliance, carried_regiments) {
                return false;
            }
            Container::Fleet(fleet)
        }
        (_, None) => Container::System(system),
    };
    move_to(world, id, container)
}

/// Whether `fleet` takes one more unit of `kind` of the side `is_alliance`
/// aboard. The planner (`FUN_00552300`) weighs a fleet target's usable
/// ships (mode 1, `FUN_00502e30(fleet, 1)`) in its capacity table
/// (`FUN_00550700`: each ship's `vt74`, then its room `vt78`), keeping the
/// first with no refusal (`FUN_0054fbb0`); with none it refuses (`0x27`).
/// A ship's room counts what is aboard in mode 3, unfinished units too
/// (`abstract-battle.md`: slot `+0x23c`). So:
/// - a squadron needs the fleet's fighter capacity (`+0x26c`) above the
///   squadrons aboard and those being built aboard
///   ([`crate::squadrons::fleet_room`]);
/// - a regiment needs its troop capacity above the `carried_regiments`
///   and those being built aboard;
/// - a special force needs no room (people take none, `FUN_00552300`);
/// - the fleet must be of the unit's side and have a usable ship: a
///   staged spare has none.
#[must_use]
pub fn room_aboard(
    world: &GameWorld,
    fleet: FleetKey,
    kind: BuildableKind,
    is_alliance: bool,
    carried_regiments: usize,
) -> bool {
    let Some(value) = world.fleets.get(fleet) else {
        return false;
    };
    if value.is_alliance != is_alliance || !value.capital_ships.iter().any(|ship| ship.alive) {
        return false;
    }
    let building = |family: fn(&BuildableKind) -> bool| {
        world
            .products
            .in_fleet(fleet)
            .filter(|product| family(&product.kind))
            .count()
    };
    match kind {
        BuildableKind::Fighter(_) => {
            let room = crate::squadrons::fleet_room(world, fleet) as usize;
            room > building(|kind| matches!(kind, BuildableKind::Fighter(_)))
        }
        BuildableKind::Troop(_) => {
            let capacity = crate::troop_transport::TroopTransportState::fleet_capacity(world, fleet)
                .unwrap_or(0) as usize;
            capacity > carried_regiments + building(|kind| matches!(kind, BuildableKind::Troop(_)))
        }
        BuildableKind::SpecialForce(_) => true,
        _ => false,
    }
}

/// A regiment or special force finished aboard a fleet boards it: the
/// completion put it on the fleet's planet (`crate::world`'s lists, the
/// integrator), and it goes aboard as its product was
/// (`TroopTransportState::embark`; a special force onto
/// `Fleet::special_forces`). A squadron the integrator seats itself.
pub fn board(
    world: &mut GameWorld,
    transport: &mut crate::troop_transport::TroopTransportState,
    completions: &[CompletionEvent],
) {
    for completion in completions {
        let Some(fleet) = completion.fleet else {
            continue;
        };
        if !world
            .fleets
            .get(fleet)
            .is_some_and(|value| value.location == completion.system)
        {
            continue;
        }
        match completion.kind {
            BuildableKind::Troop(class) => {
                let troop = world.systems.get(completion.system).and_then(|system| {
                    system.ground_units.iter().rev().copied().find(|&key| {
                        world
                            .troops
                            .get(key)
                            .is_some_and(|t| t.class_dat_id == class)
                    })
                });
                if let Some(troop) = troop {
                    let _ = transport.embark(world, fleet, &[troop]);
                }
            }
            BuildableKind::SpecialForce(class) => {
                let unit = world.systems.get(completion.system).and_then(|system| {
                    system.special_forces.iter().rev().copied().find(|&key| {
                        world
                            .special_forces
                            .get(key)
                            .is_some_and(|unit| unit.class_dat_id == class)
                    })
                });
                if let Some(unit) = unit {
                    if let Some(system) = world.systems.get_mut(completion.system) {
                        system.special_forces.retain(|&key| key != unit);
                    }
                    if let Some(value) = world.fleets.get_mut(fleet) {
                        value.special_forces.push(unit);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "products_tests.rs"]
mod tests;
