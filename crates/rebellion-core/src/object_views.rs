//! Each side's view of the other side's objects: the original's per-side
//! copies (`ghidra/notes/per-side-object-views.md`).
//!
//! Every object of the original has a true copy and one copy per side
//! (`FUN_005844e0`: group `+0x24 + side * 4`). The windows read the
//! player's side's copy (`FUN_004f2d10` -> `FUN_0053efd0`). A side's copy is
//! live while the object is the side's own, or while the side's seen bit is
//! set, the object is complete and it is not en route (`FUN_004f9a60`);
//! otherwise it is frozen and keeps its old values (`FUN_005404d0`,
//! `FUN_004f9ba0`). A system's seen bit is set while the side has a fleet in
//! it or holds it (`FUN_0050ac80`); an object inside takes its container's
//! (`FUN_004f7bf0`).
//!
//! port: the port keeps a view only of the other side's objects (a side's
//! own copy is always live, so the windows read the live object), and only
//! of regiments so far. Views are keyed by an object id the port gives each
//! regiment (`DatId`, the class family and a running number), never by a
//! slotmap key. The views are refreshed once a day after movement, control
//! and missions ([`compute`], [`apply`]).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::fog::FogState;
use crate::ids::{DatId, SystemKey, TroopKey};
use crate::movement::MovementState;
use crate::troop_transport::{RegimentLeg, TroopTransportState};
use crate::world::GameWorld;

/// Where a regiment's view puts it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewPlace {
    /// On a system's surface (its id).
    Surface(DatId),
    /// In a fleet's cargo at a system: the system's id and the fleet's name
    /// when the view was taken.
    Fleet { system: DatId, fleet_name: String },
}

impl ViewPlace {
    /// The system the place is in.
    #[must_use]
    pub fn system(&self) -> DatId {
        match self {
            Self::Surface(system) | Self::Fleet { system, .. } => *system,
        }
    }
}

/// A side's copy of a regiment (class `0x10..0x13`): what its windows show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegimentView {
    pub class: DatId,
    pub is_alliance: bool,
    pub strength: i16,
    /// Its container. An object in hyperspace is already in its destination
    /// (`FUN_00556390`).
    pub place: ViewPlace,
    /// The arrival day while it was en route (`+0x50` bit 4, `+0x44`).
    pub arrival: Option<u64>,
    /// The copy's sync state (`+0x24` bits 0..3): live (1) or frozen (2).
    pub live: bool,
}

/// One side's view of an object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectView {
    Regiment(RegimentView),
}

impl ObjectView {
    #[must_use]
    pub fn live(&self) -> bool {
        match self {
            Self::Regiment(view) => view.live,
        }
    }

    #[must_use]
    pub fn system(&self) -> DatId {
        match self {
            Self::Regiment(view) => view.place.system(),
        }
    }
}

/// Both sides' views, and the ids that key them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectViews {
    /// Each regiment's object id.
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub regiment_ids: HashMap<TroopKey, DatId>,
    /// The last running number given.
    pub last_id: u32,
    /// The Alliance's views of Imperial objects.
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub alliance: HashMap<DatId, ObjectView>,
    /// The Empire's views of Alliance objects.
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub empire: HashMap<DatId, ObjectView>,
    /// port: the debug "see everything" mode: the windows show every live
    /// object of both sides and ignore the views and the fog. Not saved.
    #[serde(skip)]
    pub see_everything: bool,
    /// Intelligence pushes the day's missions made (`Event319_Handle`),
    /// taken by the next [`advance`] ([`push_system`]). Not saved: the
    /// day's step takes them before a save can be made.
    #[serde(skip)]
    pub pushes: Vec<(Faction, SystemKey)>,
}

impl ObjectViews {
    /// Queues an intelligence push into `side`'s copies at `system`: an
    /// Espionage (level 10, `FUN_00573610`) or Reconnaissance (level 8,
    /// `FUN_0056bee0`) success.
    pub fn queue_push(&mut self, side: Faction, system: SystemKey) {
        self.pushes.push((side, system));
    }

    /// `side`'s views (Alliance or Empire; none for another side).
    #[must_use]
    pub fn side(&self, side: Faction) -> Option<&HashMap<DatId, ObjectView>> {
        match side {
            Faction::Alliance => Some(&self.alliance),
            Faction::Empire => Some(&self.empire),
            Faction::Neutral => None,
        }
    }

    fn side_mut(&mut self, side: Faction) -> Option<&mut HashMap<DatId, ObjectView>> {
        match side {
            Faction::Alliance => Some(&mut self.alliance),
            Faction::Empire => Some(&mut self.empire),
            Faction::Neutral => None,
        }
    }

    /// `side`'s view of the object `id`.
    #[must_use]
    pub fn view(&self, side: Faction, id: DatId) -> Option<&ObjectView> {
        self.side(side)?.get(&id)
    }

    /// A regiment's object id, once numbered.
    #[must_use]
    pub fn regiment_id(&self, key: TroopKey) -> Option<DatId> {
        self.regiment_ids.get(&key).copied()
    }

    /// The live regiment `id` names, if it still exists.
    #[must_use]
    pub fn regiment_key(&self, world: &GameWorld, id: DatId) -> Option<TroopKey> {
        self.regiment_ids
            .iter()
            .find(|(key, value)| **value == id && world.troops.contains_key(**key))
            .map(|(key, _)| *key)
    }
}

/// Gives each regiment without one an object id: its class family and the
/// next running number, as the original's ids carry their family. Drops the
/// ids of regiments that are gone. port: the running number is the port's;
/// the family byte sits in bits 24..31 as in every `DatId`.
pub fn number_regiments(world: &mut GameWorld) {
    let troops = &world.troops;
    let views = &mut world.object_views;
    views
        .regiment_ids
        .retain(|key, _| troops.contains_key(*key));
    let mut keys: Vec<TroopKey> = troops
        .keys()
        .filter(|key| !views.regiment_ids.contains_key(key))
        .collect();
    // Slotmap order is the insertion order of free slots; sort by the
    // key's index so the numbering is deterministic.
    keys.sort_by_key(|key| slotmap::Key::data(key).as_ffi());
    for key in keys {
        views.last_id += 1;
        let family = u32::from(troops[key].class_dat_id.family());
        views.regiment_ids.insert(
            key,
            DatId::new(family << 24 | (views.last_id & 0x00FF_FFFF)),
        );
    }
}

/// Whether `side` sees `system` (`FUN_0050ac80`): the side holds it, or one
/// of its fleets is in it. hyp: a fleet in hyperspace counts toward
/// neither system.
#[must_use]
pub fn system_seen(
    world: &GameWorld,
    movement: &MovementState,
    side: Faction,
    system: SystemKey,
) -> bool {
    let alliance = side == Faction::Alliance;
    world
        .systems
        .get(system)
        .is_some_and(|value| value.control.is_controlled_by(side))
        || world.fleets.iter().any(|(key, fleet)| {
            fleet.is_alliance == alliance
                && fleet.location == system
                && !movement.is_in_transit(key)
        })
}

/// Where a regiment is now: its place's system key, its view place and its
/// arrival day while en route.
fn regiment_place(
    world: &GameWorld,
    transport: &TroopTransportState,
    key: TroopKey,
) -> Option<(SystemKey, ViewPlace, Option<u64>)> {
    let system_id = |system: SystemKey| world.systems.get(system).map(|value| value.dat_id);
    let fleet_place = |fleet| {
        let value = world.fleets.get(fleet)?;
        Some((
            value.location,
            ViewPlace::Fleet {
                system: system_id(value.location)?,
                fleet_name: world.fleet_name(fleet).unwrap_or_default().to_owned(),
            },
        ))
    };
    if let Some(transit) = transport.transits().iter().find(|value| value.troop == key) {
        let (system, place) = match transit.leg {
            RegimentLeg::Surface(system) => (system, ViewPlace::Surface(system_id(system)?)),
            RegimentLeg::Fleet(fleet) => fleet_place(fleet)?,
        };
        return Some((system, place, Some(transit.arrival_tick)));
    }
    if let Some(fleet) = transport
        .fleet_keys()
        .into_iter()
        .find(|&fleet| transport.cargo(fleet).contains(&key))
    {
        let (system, place) = fleet_place(fleet)?;
        return Some((system, place, None));
    }
    world
        .systems
        .iter()
        .find(|(_, value)| value.ground_units.contains(&key))
        .map(|(system, value)| (system, ViewPlace::Surface(value.dat_id), None))
}

/// A change to one side's views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewUpdate {
    /// The side's copy of `id` takes `view` (a sync or a rebuild).
    Set {
        side: Faction,
        id: DatId,
        view: ObjectView,
    },
    /// The side's copy of `id` is dropped.
    Remove { side: Faction, id: DatId },
}

/// The other side's regiments, with their ids, as `side` would copy them
/// now, and whether that copy is live (`FUN_004f9a60`).
fn regiment_snapshots(
    world: &GameWorld,
    transport: &TroopTransportState,
    side: Faction,
    seen: &dyn Fn(SystemKey) -> bool,
) -> Vec<(DatId, RegimentView)> {
    let alliance = side == Faction::Alliance;
    let mut out = Vec::new();
    for (key, troop) in &world.troops {
        if troop.is_alliance == alliance {
            continue;
        }
        let Some(id) = world.object_views.regiment_id(key) else {
            continue;
        };
        let Some((system, place, arrival)) = regiment_place(world, transport, key) else {
            continue;
        };
        // FUN_004f9a60: live with the seen bit, complete (+0x50 bit 2: a
        // regiment exists only once built) and not en route (bit 4).
        let live = arrival.is_none() && seen(system);
        out.push((
            id,
            RegimentView {
                class: troop.class_dat_id,
                is_alliance: troop.is_alliance,
                strength: troop.regiment_strength,
                place,
                arrival,
                live,
            },
        ));
    }
    out
}

/// The day's view changes for both sides (`FUN_004f9a60` over every
/// object).
///
/// - A live copy takes the object's state.
/// - A copy that was live and is frozen now takes the change that froze it
///   (the master setter syncs the copy before the copy's state changes:
///   hyp, from A0 OBJ-02-C010, where a regiment's frozen copy reads
///   "Enroute, ETA Day 171") and is marked frozen.
/// - A frozen copy keeps its values. A copy that was never live is not
///   made.
/// - A copy of an object that is gone is dropped while the copy is live
///   (the side saw it go); a frozen one stays (hyp).
#[must_use]
pub fn compute(
    world: &GameWorld,
    transport: &TroopTransportState,
    movement: &MovementState,
) -> Vec<ViewUpdate> {
    let mut updates = Vec::new();
    for side in [Faction::Alliance, Faction::Empire] {
        let Some(views) = world.object_views.side(side) else {
            continue;
        };
        let seen = |system| system_seen(world, movement, side, system);
        let snapshots = regiment_snapshots(world, transport, side, &seen);
        let present: std::collections::HashSet<DatId> =
            snapshots.iter().map(|(id, _)| *id).collect();
        for (id, view) in snapshots {
            let was_live = views.get(&id).is_some_and(ObjectView::live);
            let view = ObjectView::Regiment(view);
            if (view.live() || was_live) && views.get(&id) != Some(&view) {
                updates.push(ViewUpdate::Set { side, id, view });
            }
        }
        let mut gone: Vec<DatId> = views
            .iter()
            .filter(|(id, view)| view.live() && !present.contains(id))
            .map(|(id, _)| *id)
            .collect();
        gone.sort_by_key(|id| id.raw());
        updates.extend(gone.into_iter().map(|id| ViewUpdate::Remove { side, id }));
    }
    updates
}

/// An intelligence push into `side`'s copies at `system`
/// (`Event319_Handle` `FUN_0050d5a0`): Espionage's level 10 and
/// Reconnaissance's level 8 both copy the system's regiments (child types
/// `0x01..0xfe`, `0x10..0x1f`). The copies take the object's state and keep
/// their live or frozen state.
#[must_use]
pub fn push_system(
    world: &GameWorld,
    transport: &TroopTransportState,
    movement: &MovementState,
    side: Faction,
    system: SystemKey,
) -> Vec<ViewUpdate> {
    let seen = |at| system_seen(world, movement, side, at);
    let Some(system_id) = world.systems.get(system).map(|value| value.dat_id) else {
        return Vec::new();
    };
    regiment_snapshots(world, transport, side, &seen)
        .into_iter()
        .filter(|(_, view)| view.place.system() == system_id)
        .map(|(id, view)| ViewUpdate::Set {
            side,
            id,
            view: ObjectView::Regiment(view),
        })
        .collect()
}

/// Applies `updates` in order.
pub fn apply(world: &mut GameWorld, updates: &[ViewUpdate]) {
    for update in updates {
        match update {
            ViewUpdate::Set { side, id, view } => {
                if let Some(views) = world.object_views.side_mut(*side) {
                    views.insert(*id, view.clone());
                }
            }
            ViewUpdate::Remove { side, id } => {
                if let Some(views) = world.object_views.side_mut(*side) {
                    views.remove(id);
                }
            }
        }
    }
}

/// The daily step, after movement, control and missions: number new
/// regiments, refresh both sides' views, then take the day's intelligence
/// pushes.
pub fn advance(world: &mut GameWorld, transport: &TroopTransportState, movement: &MovementState) {
    number_regiments(world);
    let mut updates = compute(world, transport, movement);
    for (side, system) in std::mem::take(&mut world.object_views.pushes) {
        updates.extend(push_system(world, transport, movement, side, system));
    }
    apply(world, &updates);
}

/// Seeds the views from the current state for the systems each side's fog
/// shows (the save v116 migration and a fixture's start): each other-side
/// regiment there takes a copy, live or frozen by `FUN_004f9a60`.
/// port: a v116 save kept no copies, so the frozen ones start from today.
pub fn seed_from_fog(
    world: &mut GameWorld,
    transport: &TroopTransportState,
    movement: &MovementState,
    fogs: [&FogState; 2],
) {
    number_regiments(world);
    let mut updates = Vec::new();
    for fog in fogs {
        let side = fog.faction;
        let seen = |at| system_seen(world, movement, side, at);
        for (id, view) in regiment_snapshots(world, transport, side, &seen) {
            let known = world
                .systems
                .iter()
                .find(|(_, value)| value.dat_id == view.place.system())
                .is_some_and(|(system, _)| fog.is_visible(system) || seen(system));
            if known {
                updates.push(ViewUpdate::Set {
                    side,
                    id,
                    view: ObjectView::Regiment(view),
                });
            }
        }
    }
    apply(world, &updates);
}

#[cfg(test)]
#[path = "object_views_tests.rs"]
mod tests;
