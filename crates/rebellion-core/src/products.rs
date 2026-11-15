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

    fn take(&mut self, id: u64) -> Option<Product> {
        let at = self.items.iter().position(|product| product.id == id)?;
        self.trips.retain(|&(trip, _)| trip != id);
        Some(self.items.remove(at))
    }
}

/// Whether `kind` is a unit this module keeps as a product.
#[must_use]
pub const fn is_tracked(kind: BuildableKind) -> bool {
    matches!(
        kind,
        BuildableKind::CapitalShip(_)
            | BuildableKind::Fighter(_)
            | BuildableKind::Troop(_)
            | BuildableKind::SpecialForce(_)
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
    let fleet = world.insert_fleet(Fleet {
        location: target,
        capital_ships: Vec::new(),
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    world.set_ship_spare(target, is_alliance, fleet);
    Container::Fleet(fleet)
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
        // Destination changes and lost fleets.
        for (&id, item) in held[..kept].iter().zip(&items) {
            let target = item.destination.unwrap_or(yard);
            let Some(product) = world.products.get(id).cloned() else {
                continue;
            };
            let lost = container_system(world, product.container).is_none();
            if product.target == target && !lost {
                continue;
            }
            let Some(mut product) = world.products.take(id) else {
                continue;
            };
            if !lost {
                drop_if_emptied(world, product.container);
            }
            product.target = target;
            product.container = place(world, product.kind, target, product.is_alliance);
            let at = world.products.items.partition_point(|p| p.id < id);
            world.products.items.insert(at, product);
        }
        for item in &items[kept..] {
            if !world.systems.contains_key(yard) {
                break;
            }
            let target = item.destination.unwrap_or(yard);
            let is_alliance = is_alliance_of(world, item.kind, yard);
            let container = place(world, item.kind, target, is_alliance);
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
    if world.products.arrival(id).is_some() {
        return false;
    }
    let Some(product) = world.products.get(id).cloned() else {
        return false;
    };
    let area = ProductionArea::of(product.kind);
    // Its place among the queue's units this module keeps.
    let Some(ordinal) = world
        .products
        .items
        .iter()
        .filter(|p| p.yard == product.yard && ProductionArea::of(p.kind) == area)
        .filter(|p| world.products.arrival(p.id).is_none())
        .position(|p| p.id == id)
    else {
        return false;
    };
    let Some(index) = manufacturing.queue(product.yard, area).and_then(|queue| {
        queue
            .items()
            .iter()
            .enumerate()
            .filter(|(_, item)| is_tracked(item.kind))
            .nth(ordinal)
            .map(|(index, _)| index)
    }) else {
        return false;
    };
    if !manufacturing
        .queue_mut(product.yard, area)
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

/// A Move released on `system`, or on `fleet` there: a regiment or
/// squadron goes to the system ([`move_to`]); a fleet target refuses it
/// (port: the carrier pick for a product is not traced). hyp: an
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
        (_, Some(_)) => return false,
        (_, None) => Container::System(system),
    };
    move_to(world, id, container)
}

#[cfg(test)]
#[path = "products_tests.rs"]
mod tests;
