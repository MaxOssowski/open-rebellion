//! Fighter squadrons on a planet, and their Move (`0x201`) between a planet
//! and a fleet in orbit (`ghidra/notes/planet-squadrons.md`).
//!
//! A built squadron goes to its destination's system, not to a fleet: the
//! product joins the container its Destination names through slot `+0xa8`
//! (`FUN_0052a140`, `FUN_0052a380`). The system lists it among its own
//! objects (`FUN_005039d0`, families `0x1c..0x1f`), and a squadron there
//! counts toward its side's presence (`FUN_00509710` → `FUN_00503a50`).
//!
//! A squadron's class lists Move, Confirmed Move and Scrap
//! (vtable `0x0065def0` slot `+0x3c`, `FUN_00502fe0` → `FUN_00557ce0`:
//! `0x201`, `0x202`, `0x204`, `0x200`). A move onto a fleet needs the ships'
//! room: each ship's fighter capacity (`+0x26c`) less the squadrons aboard
//! (`FUN_00500b40`, refused with status `0x14`/1).
//!
//! A squadron has a speed (slot `+0x34`, `FUN_00502f80`: the FIGHTSD
//! hyperdrive), so a Move to another system sends it there on its own, as a
//! regiment's does (`FUN_00556390`, `FUN_00556430`).
//!
//! A squadron in a fleet rides on one of its capital ships
//! ([`FighterEntry::carrier`]): a fleet's own room (`FUN_004fe160`) takes
//! only capital ships, so it never holds a squadron itself.
//!
//! port: the room for a move is the whole fleet's, where the original's
//! order names one ship and is refused when that ship has none
//! (`FUN_00550700`, status `0x27`); [`seat_squadrons`] then boards the
//! squadrons onto its ships.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{FighterKey, FleetKey, SystemKey};
use crate::world::{FighterEntry, GameWorld, SquadronDamage};

/// Add `count` whole squadrons of `class` to `entries`, not yet on a ship.
pub fn add(entries: &mut Vec<FighterEntry>, class: FighterKey, count: u32) {
    put(entries, class, count, None);
}

/// Take `count` squadrons of `class` from `entries`, dropping each emptied
/// entry. Nothing changes when there are fewer. port: the port's orders name
/// a class, not a squadron, so it takes from the last entries first.
pub fn take(entries: &mut Vec<FighterEntry>, class: FighterKey, count: u32) -> bool {
    take_entries(entries, class, count).is_some()
}

/// [`take`], giving the squadrons taken as entries: whole ones by their
/// carrier, and each squadron below full strength alone with its losses
/// (`+0x60`, [`crate::world::SquadronDamage`]). None, and nothing
/// changes, when there are fewer.
pub fn take_entries(
    entries: &mut Vec<FighterEntry>,
    class: FighterKey,
    count: u32,
) -> Option<Vec<FighterEntry>> {
    let held = entries
        .iter()
        .filter(|entry| entry.class == class)
        .fold(0_u32, |sum, entry| sum.saturating_add(entry.count));
    if held < count {
        return None;
    }
    let mut taken_entries = Vec::new();
    let mut left = count;
    for index in (0..entries.len()).rev() {
        if left == 0 {
            break;
        }
        if entries[index].class != class {
            continue;
        }
        let taken = entries[index].count.min(left);
        entries[index].count -= taken;
        left -= taken;
        if taken > 0 {
            let mut piece = entries[index].clone();
            piece.count = taken;
            taken_entries.push(piece);
        }
        if entries[index].count == 0 {
            entries.remove(index);
        }
    }
    Some(taken_entries)
}

/// A battle's result for the entry at `index` of `entries`, of a class of
/// `size` craft: `craft_after` holds each surviving squadron's craft
/// (`FUN_0040c780` walks the squadrons, `FUN_0040d260` sets each one's
/// losses `+0x60` to its size less the craft that came back; one with none
/// is destroyed, `FUN_00503370` -> slot `+0xac`). Whole survivors stay in
/// the entry; each one below full strength takes an entry of its own on
/// the same carrier, at the end so the other indices hold. A squadron
/// already damaged keeps its repair timer (the battle leaves it Damaged).
/// With no craft listed, the entry keeps `squads_after` whole squadrons.
pub fn apply_battle_craft(
    entries: &mut Vec<FighterEntry>,
    index: usize,
    size: u32,
    squads_after: u32,
    craft_after: &[u32],
) {
    let Some(entry) = entries.get_mut(index) else {
        return;
    };
    if craft_after.is_empty() {
        entry.count = squads_after;
        if squads_after == 0 {
            entry.damage = None;
        }
        return;
    }
    let timer = entry.damage.and_then(|damage| damage.repair);
    let damaged = |craft: u32, repair| SquadronDamage {
        lost: size.saturating_sub(craft),
        repair,
    };
    let whole = craft_after.iter().filter(|&&craft| craft >= size).count();
    let mut below = craft_after.iter().copied().filter(|&craft| craft < size);
    entry.count = u32::try_from(whole).unwrap_or(u32::MAX);
    entry.damage = None;
    if entry.count == 0 {
        if let Some(craft) = below.next() {
            entry.count = 1;
            entry.damage = Some(damaged(craft, timer));
        }
    }
    let (class, carrier) = (entry.class, entry.carrier);
    for craft in below {
        entries.push(FighterEntry {
            class,
            count: 1,
            carrier,
            damage: Some(damaged(craft, None)),
        });
    }
}

/// Add `entry` to `entries`: whole squadrons join an entry of their class
/// and carrier ([`put`]); a squadron below full strength keeps an entry of
/// its own, with its losses.
pub fn put_entry(entries: &mut Vec<FighterEntry>, entry: FighterEntry) {
    if entry.damage.is_none() {
        put(entries, entry.class, entry.count, entry.carrier);
    } else if entry.count > 0 {
        entries.push(entry);
    }
}

/// The squadrons aboard the ship numbered `ship`.
#[must_use]
pub fn aboard(entries: &[FighterEntry], ship: u64) -> u32 {
    entries
        .iter()
        .filter(|entry| entry.carrier == Some(ship))
        .fold(0_u32, |sum, entry| sum.saturating_add(entry.count))
}

/// Seat `fleet`'s squadrons that ride on none of its living ships: each
/// goes aboard the living ship with the most room left (fighter capacity
/// `+0x26c` less the squadrons aboard, `FUN_00500b40`), the earlier one on
/// a tie. The leg builder (`FUN_00552300`) weighs each ship of the fleet
/// (`FUN_00552b10`) and a later one wins only when strictly better
/// (`FUN_0054fbb0`: no refusal, then more room left, `+8`). Entries of a
/// class on one ship are merged. port: squadrons left over when no ship has
/// room stay in the fleet on no ship, as the port's seeding and sorties can
/// put them there (a Move with no room is refused, `0x14`/1).
pub fn seat_squadrons(world: &mut GameWorld, fleet: FleetKey) {
    let Some(value) = world.fleets.get(fleet) else {
        return;
    };
    let ships = value
        .capital_ships
        .iter()
        .filter(|ship| ship.alive && ship.id != 0)
        .map(|ship| {
            let capacity = world
                .capital_ship_classes
                .get(ship.class)
                .map_or(0, |class| class.fighter_capacity);
            (ship.id, capacity)
        })
        .collect::<Vec<_>>();
    let seated = |entry: &FighterEntry| {
        entry
            .carrier
            .is_some_and(|carrier| ships.iter().any(|(id, _)| *id == carrier))
    };
    if value.fighters.iter().all(&seated) {
        return;
    }
    let entries = std::mem::take(&mut world.fleets[fleet].fighters);
    let (mut kept, loose): (Vec<_>, Vec<_>) = entries.into_iter().partition(|entry| seated(entry));
    for entry in loose {
        let mut left = entry.count;
        while left > 0 {
            let best = ships
                .iter()
                .map(|&(ship, capacity)| (ship, capacity.saturating_sub(aboard(&kept, ship))))
                .filter(|&(_, room)| room > 0)
                .fold(None, |best: Option<(u64, u32)>, candidate| match best {
                    Some(held) if candidate.1 <= held.1 => Some(held),
                    _ => Some(candidate),
                });
            let Some((ship, _)) = best else {
                break;
            };
            left -= 1;
            put_entry(
                &mut kept,
                FighterEntry {
                    count: 1,
                    carrier: Some(ship),
                    ..entry.clone()
                },
            );
        }
        put_entry(
            &mut kept,
            FighterEntry {
                count: left,
                carrier: None,
                ..entry
            },
        );
    }
    world.fleets[fleet].fighters = kept;
}

/// Add `count` whole squadrons of `class` on `carrier` to `entries`: they
/// join an entry of whole squadrons of that class on that carrier.
pub fn put(entries: &mut Vec<FighterEntry>, class: FighterKey, count: u32, carrier: Option<u64>) {
    if count == 0 {
        return;
    }
    if let Some(entry) = entries.iter_mut().find(|entry| {
        entry.class == class && entry.carrier == carrier && entry.damage.is_none()
    }) {
        entry.count = entry.count.saturating_add(count);
    } else {
        entries.push(FighterEntry { class, count, carrier, damage: None });
    }
}

/// The squadrons in `entries`.
#[must_use]
pub fn total(entries: &[FighterEntry]) -> u32 {
    entries
        .iter()
        .fold(0_u32, |sum, entry| sum.saturating_add(entry.count))
}

/// Whether a squadron of `class` serves the Alliance.
#[must_use]
pub fn is_alliance(world: &GameWorld, class: FighterKey) -> Option<bool> {
    world
        .fighter_classes
        .get(class)
        .map(|value| value.is_alliance)
}

/// The squadrons of one side on `system`'s planet.
#[must_use]
pub fn on_planet(world: &GameWorld, system: SystemKey, alliance: bool) -> u32 {
    world.systems.get(system).map_or(0, |value| {
        value
            .fighters
            .iter()
            .filter(|entry| is_alliance(world, entry.class) == Some(alliance))
            .fold(0_u32, |sum, entry| sum.saturating_add(entry.count))
    })
}

/// A fleet's room for more squadrons: its living ships' fighter capacity
/// (`+0x26c`, CAPSHPSD `fighter_capacity`) less the squadrons aboard
/// (`FUN_00500b40`).
#[must_use]
pub fn fleet_room(world: &GameWorld, fleet: FleetKey) -> u32 {
    let Some(value) = world.fleets.get(fleet) else {
        return 0;
    };
    let capacity = value
        .capital_ships
        .iter()
        .filter(|ship| ship.alive)
        .filter_map(|ship| world.capital_ship_classes.get(ship.class))
        .fold(0_u32, |sum, class| {
            sum.saturating_add(class.fighter_capacity)
        });
    capacity.saturating_sub(total(&value.fighters))
}

/// Where a squadron is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SquadronPlace {
    /// On a system's planet.
    Planet(SystemKey),
    /// Aboard a fleet.
    Fleet(FleetKey),
}

/// Why a squadron's move is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquadronMoveError {
    MissingFleet,
    MissingSystem,
    /// No squadron of the class is at the place.
    MissingSquadron,
    /// The squadron is not the side's.
    WrongSide,
    /// A fleet in hyperspace takes no orders (manual p. 121).
    InTransit,
    /// The destination fleet is another side's (`FUN_00553410`, `1`/`0x24`).
    OtherSideDestination,
    /// The destination planet is not the squadron's side's: `FUN_0053d430`
    /// (the `0x204` sub-order's check) refuses with `1`/`0x28` a system whose
    /// side bits (`+0x24 >> 6 & 3`) are not the mover's, a neutral or
    /// unheld one included, since a squadron is neither a fleet nor a
    /// regiment.
    OtherSidePlanet,
    /// A move onto the place the squadron is at.
    IntoItself,
    /// The fleet has no room (`FUN_00500b40`, `0x14`/1).
    NoRoom,
    /// The destination planet is blockaded (`FUN_005073d0`, `0x90`/4).
    DestinationBlockaded,
}

impl fmt::Display for SquadronMoveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingFleet => "the fleet no longer exists",
            Self::MissingSystem => "the system no longer exists",
            Self::MissingSquadron => "the squadron is not there",
            Self::WrongSide => "the squadron belongs to another side",
            Self::InTransit => "the fleet is in hyperspace",
            Self::OtherSideDestination => "the destination fleet belongs to another side",
            Self::OtherSidePlanet => "the destination planet belongs to another side",
            Self::IntoItself => "the squadron is already there",
            Self::NoRoom => "the fleet has no room for fighters",
            Self::DestinationBlockaded => "the destination planet is blockaded",
        })
    }
}

/// The system a place is in, and whether the place is usable: a planet
/// always, a fleet only in orbit (in its system's fleet list).
fn place_system(world: &GameWorld, place: SquadronPlace) -> Result<SystemKey, SquadronMoveError> {
    match place {
        SquadronPlace::Planet(system) => {
            if world.systems.contains_key(system) {
                Ok(system)
            } else {
                Err(SquadronMoveError::MissingSystem)
            }
        }
        SquadronPlace::Fleet(fleet) => {
            let value = world
                .fleets
                .get(fleet)
                .ok_or(SquadronMoveError::MissingFleet)?;
            let system = world
                .systems
                .get(value.location)
                .ok_or(SquadronMoveError::MissingSystem)?;
            if system.fleets.contains(&fleet) {
                Ok(value.location)
            } else {
                Err(SquadronMoveError::InTransit)
            }
        }
    }
}

fn entries(world: &GameWorld, place: SquadronPlace) -> &[FighterEntry] {
    match place {
        SquadronPlace::Planet(system) => world
            .systems
            .get(system)
            .map_or(&[], |value| value.fighters.as_slice()),
        SquadronPlace::Fleet(fleet) => world
            .fleets
            .get(fleet)
            .map_or(&[], |value| value.fighters.as_slice()),
    }
}

fn entries_mut(world: &mut GameWorld, place: SquadronPlace) -> Option<&mut Vec<FighterEntry>> {
    match place {
        SquadronPlace::Planet(system) => world
            .systems
            .get_mut(system)
            .map(|value| &mut value.fighters),
        SquadronPlace::Fleet(fleet) => world.fleets.get_mut(fleet).map(|value| &mut value.fighters),
    }
}

/// Set the losses of the `index`th squadron entry at `place` (a repair
/// event, [`crate::repair::RepairEvent::SquadronRepaired`]). Returns whether
/// the entry was there.
pub fn set_damage(
    world: &mut GameWorld,
    place: SquadronPlace,
    index: usize,
    damage: Option<SquadronDamage>,
) -> bool {
    let Some(entry) = entries_mut(world, place).and_then(|entries| entries.get_mut(index)) else {
        return false;
    };
    entry.damage = damage;
    true
}

/// Whether a squadron's pop-up menu enables Move for the side `alliance`:
/// its own squadron of `class` at `place`, on a planet or aboard a fleet in
/// orbit (`FUN_004f9860`'s side and en-route rules). That check is the
/// squadron's only one (its slots `+0x5c`/`+0x6c` are the combat unit's
/// `FUN_005575f0`/`FUN_005576a0`): its order-kind set (`FUN_00503000`,
/// `0x200`..`0x204`, which a Move is in), the side, not complete, destroyed
/// and en route; the port's squadrons are always complete and alive.
#[must_use]
pub fn move_enabled(
    world: &GameWorld,
    alliance: bool,
    class: FighterKey,
    place: SquadronPlace,
) -> bool {
    is_alliance(world, class) == Some(alliance)
        && place_system(world, place).is_ok()
        && entries(world, place)
            .iter()
            .any(|entry| entry.class == class && entry.count > 0)
}

/// The checks of a Move of `count` squadrons of `class` from `from` to
/// `to` for the side `alliance`, but the room.
fn check_route(
    world: &GameWorld,
    alliance: bool,
    class: FighterKey,
    count: u32,
    from: SquadronPlace,
    to: SquadronPlace,
) -> Result<(SystemKey, SystemKey), SquadronMoveError> {
    if from == to {
        return Err(SquadronMoveError::IntoItself);
    }
    let origin = place_system(world, from)?;
    if is_alliance(world, class) != Some(alliance) {
        return Err(SquadronMoveError::WrongSide);
    }
    // Every entry of the class counts: one below full strength has its own.
    let held = entries(world, from)
        .iter()
        .filter(|entry| entry.class == class)
        .fold(0_u32, |sum, entry| sum.saturating_add(entry.count));
    if count == 0 || held < count {
        return Err(SquadronMoveError::MissingSquadron);
    }
    let destination = place_system(world, to)?;
    match to {
        SquadronPlace::Fleet(fleet) => {
            if world.fleets.get(fleet).map(|value| value.is_alliance) != Some(alliance) {
                return Err(SquadronMoveError::OtherSideDestination);
            }
        }
        SquadronPlace::Planet(system) => {
            let side = crate::dat::Faction::of_alliance(alliance);
            let holder = world.systems.get(system).and_then(|value| value.control.faction());
            if holder != Some(side) {
                return Err(SquadronMoveError::OtherSidePlanet);
            }
            // FUN_00550700 asks the system's acceptance (+0x74,
            // FUN_005073d0): a blockaded system gives 0x90/4, lifted for
            // fleets and personnel but not a squadron's 0x204.
            if world.systems.get(system).is_some_and(|value| {
                crate::blockade::BlockadeSystem::system_is_blockaded(world, value)
            }) {
                return Err(SquadronMoveError::DestinationBlockaded);
            }
        }
    }
    Ok((origin, destination))
}

/// A squadron Move's travel to another system (`FUN_00556430`: the
/// in-transit bit, the destination and the arrival day).
///
/// The original moves the squadron into its new container at once and marks
/// it en route. port: as for regiments and deliveries, the port holds it here
/// and places it on arrival, so no walk of the destination sees it early.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquadronTransit {
    pub class: FighterKey,
    pub count: u32,
    /// The system it left.
    pub origin: SystemKey,
    /// Where it goes.
    pub to: SquadronPlace,
    /// The game-day it arrives.
    pub arrival_tick: u64,
    /// The losses (`+0x60`) of each squadron below full strength among
    /// `count`; the others are whole. A squadron en route is not Usable, so
    /// it does not repair (`FUN_005578d0`). The save keeps them in
    /// `SaveState::squadron_damage` (v126); the transit's own encoding
    /// skips them.
    #[serde(skip)]
    pub lost: Vec<u32>,
}

/// What a squadron Move did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquadronMove {
    /// Within one system the squadrons changed place at once.
    Placed,
    /// The squadrons left for another system and arrive on this day.
    Departed(u64),
}

/// The squadrons travelling on their own, in departure order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquadronTransits {
    transits: Vec<SquadronTransit>,
}

impl SquadronTransits {
    /// The squadrons travelling, in departure order.
    #[must_use]
    pub fn transits(&self) -> &[SquadronTransit] {
        &self.transits
    }

    /// Puts squadrons already off their place on the road (tests and
    /// fixtures).
    #[doc(hidden)]
    pub fn push_transit_for_test(&mut self, transit: SquadronTransit) {
        self.transits.push(transit);
    }

    /// The squadrons on their way to `fleet`.
    fn incoming(&self, fleet: FleetKey) -> u32 {
        self.transits
            .iter()
            .filter(|transit| transit.to == SquadronPlace::Fleet(fleet))
            .fold(0_u32, |sum, transit| sum.saturating_add(transit.count))
    }

    /// Check a Move of `count` squadrons of `class` from `from` to `to` for
    /// the side `alliance`, as [`Self::move_squadrons`] would, without moving
    /// them: the move command's validation (`FUN_00578d00`). A fleet's room
    /// counts the squadrons on their way to it.
    ///
    /// # Errors
    /// Returns why the move is refused.
    pub fn check(
        &self,
        world: &GameWorld,
        alliance: bool,
        class: FighterKey,
        count: u32,
        from: SquadronPlace,
        to: SquadronPlace,
    ) -> Result<(), SquadronMoveError> {
        self.checked(world, alliance, class, count, from, to)
            .map(|_| ())
    }

    /// [`Self::check`], giving the systems the squadrons leave and enter.
    fn checked(
        &self,
        world: &GameWorld,
        alliance: bool,
        class: FighterKey,
        count: u32,
        from: SquadronPlace,
        to: SquadronPlace,
    ) -> Result<(SystemKey, SystemKey), SquadronMoveError> {
        let systems = check_route(world, alliance, class, count, from, to)?;
        if let SquadronPlace::Fleet(fleet) = to {
            if fleet_room(world, fleet).saturating_sub(self.incoming(fleet)) < count {
                return Err(SquadronMoveError::NoRoom);
            }
        }
        Ok(systems)
    }

    /// The days [`Self::move_squadrons`] would take the squadrons: none
    /// within one system. The Confirmed Move's line (`FUN_0053c2e0`).
    ///
    /// # Errors
    /// Returns [`Self::check`]'s refusal.
    pub fn move_days(
        &self,
        world: &GameWorld,
        alliance: bool,
        class: FighterKey,
        count: u32,
        from: SquadronPlace,
        to: SquadronPlace,
    ) -> Result<u32, SquadronMoveError> {
        let (origin, destination) = self.checked(world, alliance, class, count, from, to)?;
        Ok(if origin == destination {
            0
        } else {
            squadron_transit_ticks(world, class, origin, destination)
        })
    }

    /// Move `count` squadrons of `class` from `from` to `to` for the side
    /// `alliance` on day `now`: within one system at once, to another
    /// system at the class's speed (`FUN_00502f80`, `FUN_0055d8c0`). A
    /// fleet left with no ship, squadron or character disbands
    /// (`FUN_004fe630`).
    ///
    /// # Errors
    /// Returns [`Self::check`]'s refusal, and then nothing changes.
    #[expect(
        clippy::too_many_arguments,
        reason = "A Move names its side, squadrons, places and day."
    )]
    pub fn move_squadrons(
        &mut self,
        world: &mut GameWorld,
        alliance: bool,
        class: FighterKey,
        count: u32,
        from: SquadronPlace,
        to: SquadronPlace,
        now: u64,
    ) -> Result<SquadronMove, SquadronMoveError> {
        let (origin, destination) = self.checked(world, alliance, class, count, from, to)?;
        let taken = entries_mut(world, from)
            .and_then(|source| take_entries(source, class, count))
            .unwrap_or_default();
        // FUN_00556390 puts the squadron in its destination at once; a
        // planet's system records its side (slot +0xd0, FUN_00508660).
        if let SquadronPlace::Planet(system) = to {
            if let Some(value) = world.systems.get_mut(system) {
                value.unit_joined(alliance);
            }
        }
        let moved = if origin == destination {
            // A squadron moved within its system stays Usable and keeps its
            // repair timer (FUN_005578d0).
            if let Some(target) = entries_mut(world, to) {
                for entry in taken {
                    put_entry(target, FighterEntry { carrier: None, ..entry });
                }
            }
            SquadronMove::Placed
        } else {
            let ticks = squadron_transit_ticks(world, class, origin, destination);
            let arrival_tick = now + u64::from(ticks);
            let lost = taken
                .iter()
                .filter_map(|entry| entry.damage.map(|damage| damage.lost))
                .collect();
            self.transits.push(SquadronTransit {
                class,
                count,
                origin,
                to,
                arrival_tick,
                lost,
            });
            SquadronMove::Departed(arrival_tick)
        };
        if let SquadronPlace::Fleet(fleet) = from {
            disband_if_empty(world, fleet);
        }
        Ok(moved)
    }

    /// Place every squadron whose day has come in its destination: a
    /// planet, or a fleet wherever it is. One whose fleet is gone or whose
    /// planet is destroyed is lost (`FUN_004fc080`). Returns the arrivals
    /// and the losses.
    pub fn advance(
        &mut self,
        world: &mut GameWorld,
        now: u64,
    ) -> (Vec<SquadronTransit>, Vec<SquadronTransit>) {
        let (mut arrived, mut lost) = (Vec::new(), Vec::new());
        let mut remaining = Vec::with_capacity(self.transits.len());
        for transit in std::mem::take(&mut self.transits) {
            if transit.arrival_tick > now {
                remaining.push(transit);
                continue;
            }
            let entries = match transit.to {
                SquadronPlace::Planet(system) => world
                    .systems
                    .get_mut(system)
                    .filter(|value| !value.is_destroyed)
                    .map(|value| &mut value.fighters),
                SquadronPlace::Fleet(fleet) => {
                    world.fleets.get_mut(fleet).map(|value| &mut value.fighters)
                }
            };
            match entries {
                Some(entries) => {
                    land(entries, transit.class, transit.count, &transit.lost);
                    arrived.push(transit);
                }
                None => lost.push(transit),
            }
        }
        self.transits = remaining;
        (arrived, lost)
    }
}

/// Add `count` squadrons of `class`, not yet on a ship, to `entries`:
/// one below full strength for each of `lost`, the rest whole. A squadron
/// arriving has no repair timer: travelling cancelled it (`FUN_005578d0`).
fn land(entries: &mut Vec<FighterEntry>, class: FighterKey, count: u32, lost: &[u32]) {
    let damaged = lost.iter().take(count as usize);
    let mut whole = count;
    for &craft in damaged {
        whole -= 1;
        put_entry(
            entries,
            FighterEntry {
                class,
                count: 1,
                carrier: None,
                damage: Some(SquadronDamage {
                    lost: craft,
                    repair: None,
                }),
            },
        );
    }
    add(entries, class, whole);
}

/// Where a squadron below full strength is, for the save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamagedSquadronPlace {
    /// The `index`th entry of a place's squadrons.
    Entry(SquadronPlace, usize),
    /// Among the squadrons of the `index`th transit.
    Transit(usize),
}

/// A squadron below full strength as the save keeps it
/// (`SaveState::squadron_damage`, v126): the world's and the transits' own
/// encodings skip the losses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamagedSquadron {
    pub place: DamagedSquadronPlace,
    pub damage: SquadronDamage,
}

/// The squadrons below full strength, in the fleets', the planets' and the
/// transits' order.
#[must_use]
pub fn damaged_squadrons(world: &GameWorld, transits: &SquadronTransits) -> Vec<DamagedSquadron> {
    let fleets = world
        .fleets
        .iter()
        .map(|(key, fleet)| (SquadronPlace::Fleet(key), &fleet.fighters));
    let planets = world
        .systems
        .iter()
        .map(|(key, system)| (SquadronPlace::Planet(key), &system.fighters));
    let mut records: Vec<DamagedSquadron> = fleets
        .chain(planets)
        .flat_map(|(place, entries)| {
            entries.iter().enumerate().filter_map(move |(index, entry)| {
                entry.damage.map(|damage| DamagedSquadron {
                    place: DamagedSquadronPlace::Entry(place, index),
                    damage,
                })
            })
        })
        .collect();
    for (index, transit) in transits.transits.iter().enumerate() {
        records.extend(transit.lost.iter().map(|&lost| DamagedSquadron {
            place: DamagedSquadronPlace::Transit(index),
            damage: SquadronDamage { lost, repair: None },
        }));
    }
    records
}

/// Put back the losses [`damaged_squadrons`] recorded; every other
/// squadron is whole. A save from before v126 has none, so all its
/// squadrons load at full strength.
pub fn restore_damaged_squadrons(
    world: &mut GameWorld,
    transits: &mut SquadronTransits,
    records: &[DamagedSquadron],
) {
    for transit in &mut transits.transits {
        transit.lost.clear();
    }
    for fleet in world.fleets.values_mut() {
        for entry in &mut fleet.fighters {
            entry.damage = None;
        }
    }
    for system in world.systems.values_mut() {
        for entry in &mut system.fighters {
            entry.damage = None;
        }
    }
    for record in records {
        match record.place {
            DamagedSquadronPlace::Entry(place, index) => {
                set_damage(world, place, index, Some(record.damage));
            }
            DamagedSquadronPlace::Transit(index) => {
                if let Some(transit) = transits.transits.get_mut(index) {
                    transit.lost.push(record.damage.lost);
                }
            }
        }
    }
}

/// Remove `fleet` when it holds nothing: no ship, squadron or character.
///
/// port: the original's fleet holds only capital ships, and disbands when
/// the last leaves (`FUN_004fe630`); the port's seeding makes fleets of
/// squadrons alone, which disband when their last squadron leaves.
fn disband_if_empty(world: &mut GameWorld, fleet: FleetKey) {
    let empty = world.fleets.get(fleet).is_some_and(|value| {
        value.capital_ships.is_empty()
            && value.fighters.is_empty()
            && value.characters.is_empty()
            && value.special_forces.is_empty()
    });
    if !empty {
        return;
    }
    if let Some(value) = world.fleets.remove(fleet) {
        if let Some(system) = world.systems.get_mut(value.location) {
            system.fleets.retain(|&key| key != fleet);
        }
    }
}

/// Squadrons of `class` travel between systems at the class's speed
/// (`FUN_00502f80`, `FUN_0055d8c0`).
fn squadron_transit_ticks(
    world: &GameWorld,
    class: FighterKey,
    origin: SystemKey,
    destination: SystemKey,
) -> u32 {
    let position = |key: SystemKey| world.systems.get(key).map_or((0, 0), |s| (s.x, s.y));
    let speed = world.fighter_classes.get(class).map_or_else(
        || crate::movement::default_speed(world),
        |value| crate::movement::fighter_speed(world, value),
    );
    crate::movement::transit_ticks_between(world, position(origin), position(destination), speed)
}

/// Whether each side, Alliance then Empire, is present at `system`: an
/// active fleet in orbit or an active squadron on the planet
/// (`FUN_00509710`). Both present make a battle pending (`FUN_0050a900`),
/// and the battle manager starts a battle there (`FUN_00516d50`).
#[must_use]
pub fn sides_present(world: &GameWorld, system: SystemKey) -> (bool, bool) {
    let Some(value) = world.systems.get(system) else {
        return (false, false);
    };
    let fleets = value
        .fleets
        .iter()
        .filter(|&&key| crate::products::is_usable(world, key))
        .filter_map(|&key| world.fleets.get(key))
        .map(|fleet| fleet.is_alliance);
    let squadrons = value
        .fighters
        .iter()
        .filter(|entry| entry.count > 0)
        .filter_map(|entry| is_alliance(world, entry.class));
    fleets
        .chain(squadrons)
        .fold((false, false), |(alliance, empire), side| {
            (alliance || side, empire || !side)
        })
}

/// A side's planet squadrons sent up for one battle: into its first fleet
/// in orbit, or, when it has none, into a group of their own.
///
/// port: the original's battle takes a planet's squadrons from the planet
/// (manual p. 129, "they can engage an enemy fleet when it enters your
/// system"; `FUN_00516ef0` counts them with each side's fleets). The port's
/// combat is fleet against fleet, so they join a fleet for the battle's
/// length and the survivors land again (`recover`). The squadrons aboard
/// keep no identity, so the planet takes back its own first.
#[derive(Debug, Clone)]
pub struct Sortie {
    pub system: SystemKey,
    pub fleet: FleetKey,
    /// True when the fleet is a group made for the battle.
    pub group: bool,
    /// The squadrons that left the planet.
    pub launched: Vec<FighterEntry>,
}

/// Send each side's squadrons on `system`'s planet into the battle there
/// ([`Sortie`]). The planet keeps none.
pub fn scramble(world: &mut GameWorld, system: SystemKey) -> Vec<Sortie> {
    let mut sorties = Vec::new();
    for alliance in [true, false] {
        let Some(value) = world.systems.get(system) else {
            return sorties;
        };
        let launched: Vec<FighterEntry> = value
            .fighters
            .iter()
            .filter(|entry| entry.count > 0 && is_alliance(world, entry.class) == Some(alliance))
            .cloned()
            .collect();
        if launched.is_empty() {
            continue;
        }
        let carrier = value.fleets.iter().copied().find(|&key| {
            world
                .fleets
                .get(key)
                .is_some_and(|f| f.is_alliance == alliance)
        });
        if let Some(planet) = world.systems.get_mut(system) {
            planet
                .fighters
                .retain(|entry| !launched.iter().any(|sent| sent.class == entry.class));
        }
        let (fleet, group) = if let Some(fleet) = carrier {
            for entry in &launched {
                put_entry(&mut world.fleets[fleet].fighters, entry.clone());
            }
            (fleet, false)
        } else {
            let fleet = world.fleets.insert(crate::world::Fleet {
                location: system,
                fighters: launched.clone(),
                is_alliance: alliance,
                ..Default::default()
            });
            world.systems[system].fleets.push(fleet);
            (fleet, true)
        };
        sorties.push(Sortie {
            system,
            fleet,
            group,
            launched,
        });
    }
    sorties
}

/// Land each sortie's surviving squadrons on its planet after the battle,
/// and remove a group made for it.
pub fn recover(world: &mut GameWorld, sorties: &[Sortie]) {
    for sortie in sorties {
        let mut landing = Vec::new();
        if sortie.group {
            if let Some(fleet) = world.fleets.remove(sortie.fleet) {
                landing = fleet.fighters;
            }
            if let Some(system) = world.systems.get_mut(sortie.system) {
                system.fleets.retain(|&key| key != sortie.fleet);
            }
        } else if let Some(fleet) = world.fleets.get_mut(sortie.fleet) {
            // Each class takes back as many as it sent, the ones on no ship
            // (the planet's) first, those below full strength with their
            // losses.
            for sent in &sortie.launched {
                let (mut entries, unseated): (Vec<_>, Vec<_>) = std::mem::take(&mut fleet.fighters)
                    .into_iter()
                    .partition(|entry| entry.class != sent.class || entry.carrier.is_some());
                // The unseated ones go last, so they are taken first.
                entries.extend(unseated);
                let held = entries
                    .iter()
                    .filter(|entry| entry.class == sent.class)
                    .fold(0_u32, |sum, entry| sum.saturating_add(entry.count));
                if let Some(taken) = take_entries(&mut entries, sent.class, held.min(sent.count)) {
                    landing.extend(taken);
                }
                fleet.fighters = entries;
            }
        }
        if let Some(system) = world.systems.get_mut(sortie.system) {
            for entry in landing {
                put_entry(&mut system.fighters, FighterEntry { carrier: None, ..entry });
            }
        }
        // The planet's squadrons never joined the fleet in the original
        // (FUN_00516ef0 counts them apart): a fleet whose last ship fell
        // is empty once they land, and disbands (FUN_004fe630).
        if !sortie.group {
            crate::scrap::disband_if_empty(world, sortie.fleet);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{ExplorationStatus, Faction, SectorGroup};
    use crate::ids::DatId;
    use crate::repair::RepairTimer;
    use crate::world::{
        CapitalShipClass, ControlKind, FighterClass, Fleet, Sector, ShipInstance, System,
    };

    struct Setup {
        world: GameWorld,
        here: SystemKey,
        there: SystemKey,
        fleet: FleetKey,
        x_wing: FighterKey,
        tie: FighterKey,
    }

    /// An Alliance carrier fleet of `capacity` squadrons at `here`, both
    /// systems held by the Alliance.
    fn setup(capacity: u32) -> Setup {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(1),
            name: "Test".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let system = |name: &str| System {
            dat_id: DatId::new(0x9000_0001),
            name: name.into(),
            sector,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            control: ControlKind::Controlled(Faction::Alliance),
            ..Default::default()
        };
        let here = world.systems.insert(system("Here"));
        let there = world.systems.insert(system("There"));
        let carrier = world.capital_ship_classes.insert(CapitalShipClass {
            name: "Carrier".into(),
            is_alliance: true,
            fighter_capacity: capacity,
            hull: 100,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: here,
            capital_ships: vec![ShipInstance::new(carrier, 100, true)],
            is_alliance: true,
            ..Default::default()
        });
        world.systems[here].fleets.push(fleet);
        let x_wing = world.fighter_classes.insert(FighterClass {
            name: "X-wing".into(),
            is_alliance: true,
            ..FighterClass::default()
        });
        let tie = world.fighter_classes.insert(FighterClass {
            name: "TIE".into(),
            is_empire: true,
            ..FighterClass::default()
        });
        Setup {
            world,
            here,
            there,
            fleet,
            x_wing,
            tie,
        }
    }

    // FUN_00500b40: a ship holds up to its fighter capacity (+0x26c), and
    // a fleet's squadrons ride on its ships (FUN_004fe160 takes only
    // capital ships). Each boards the ship with the most room left, the
    // earlier on a tie (FUN_00552b10 keeps a candidate unless FUN_0054fbb0
    // finds a later one strictly better, +8 the room left).
    #[test]
    fn squadrons_board_the_ship_with_the_most_room_left() {
        let mut setup = setup(2);
        let carrier = setup.world.fleets[setup.fleet].capital_ships[0].class;
        let fleet = &mut setup.world.fleets[setup.fleet];
        fleet.capital_ships.push(ShipInstance::new(carrier, 100, true));
        add(&mut fleet.fighters, setup.x_wing, 5);
        setup.world.number_ships();
        let fleet = &setup.world.fleets[setup.fleet];
        let first = fleet.capital_ships[0].id;
        let second = fleet.capital_ships[1].id;
        assert_eq!(aboard(&fleet.fighters, first), 2);
        assert_eq!(aboard(&fleet.fighters, second), 2);
        // The one left over has no ship.
        assert_eq!(
            fleet.fighters.iter().filter(|entry| entry.carrier.is_none()).map(|entry| entry.count).sum::<u32>(),
            1
        );
        assert_eq!(total(&fleet.fighters), 5);
        // Two squadrons for two ships of room 3 spread one a ship.
        let mut roomy = self::setup(3);
        let carrier = roomy.world.fleets[roomy.fleet].capital_ships[0].class;
        let fleet = &mut roomy.world.fleets[roomy.fleet];
        fleet.capital_ships.push(ShipInstance::new(carrier, 100, true));
        add(&mut fleet.fighters, roomy.x_wing, 2);
        roomy.world.number_ships();
        let fleet = &roomy.world.fleets[roomy.fleet];
        assert_eq!(aboard(&fleet.fighters, fleet.capital_ships[0].id), 1);
        assert_eq!(aboard(&fleet.fighters, fleet.capital_ships[1].id), 1);
    }

    // Two loose entries of one class boarding one ship make one entry.
    #[test]
    fn squadrons_of_a_class_on_one_ship_share_an_entry() {
        let mut setup = setup(4);
        let fleet = &mut setup.world.fleets[setup.fleet];
        fleet.fighters.push(FighterEntry { class: setup.x_wing, count: 1, carrier: None, damage: None });
        fleet.fighters.push(FighterEntry { class: setup.x_wing, count: 2, carrier: Some(99), damage: None });
        fleet.fighters.push(FighterEntry { class: setup.tie, count: 1, carrier: None, damage: None });
        setup.world.number_ships();
        let fleet = &setup.world.fleets[setup.fleet];
        let ship = fleet.capital_ships[0].id;
        assert_eq!(
            fleet.fighters.iter().map(|entry| (entry.class, entry.count, entry.carrier)).collect::<Vec<_>>(),
            [(setup.x_wing, 3, Some(ship)), (setup.tie, 1, Some(ship))]
        );
    }

    // A squadron whose ship is gone boards another ship with room. A ship
    // scrapped or destroyed outside a battle takes its docked squadrons
    // (crate::scrap::destroy_ship, GameObj_vt11c); in a battle the lost
    // ship's hangar dies with it (FUN_005af7a0 in the abstract battle, the
    // tactical commit), and the tactical commit puts its launched ones on
    // the system (FUN_0040d320). port: any squadron still in a fleet on no
    // living ship is seated here.
    #[test]
    fn a_lost_ships_squadrons_board_another_ship() {
        let mut setup = setup(2);
        let carrier = setup.world.fleets[setup.fleet].capital_ships[0].class;
        let fleet = &mut setup.world.fleets[setup.fleet];
        fleet.capital_ships.push(ShipInstance::new(carrier, 100, true));
        add(&mut fleet.fighters, setup.x_wing, 2);
        setup.world.number_ships();
        let first = setup.world.fleets[setup.fleet].capital_ships[0].id;
        assert_eq!(aboard(&setup.world.fleets[setup.fleet].fighters, first), 1);
        setup.world.fleets[setup.fleet].capital_ships[0].alive = false;
        setup.world.number_ships();
        let fleet = &setup.world.fleets[setup.fleet];
        let second = fleet.capital_ships[1].id;
        assert_eq!(aboard(&fleet.fighters, second), 2);
        assert_eq!(fleet.fighters.len(), 1);
    }

    // port: an order names a class, so the squadrons come from the last
    // entries first, across ships.
    #[test]
    fn taking_squadrons_draws_on_the_last_entries_first() {
        let mut entries = vec![
            FighterEntry { class: FighterKey::default(), count: 2, carrier: Some(1), damage: None },
            FighterEntry { class: FighterKey::default(), count: 1, carrier: Some(2), damage: None },
        ];
        assert!(take(&mut entries, FighterKey::default(), 2));
        assert_eq!(entries.len(), 1);
        assert_eq!((entries[0].count, entries[0].carrier), (1, Some(1)));
        assert!(!take(&mut entries, FighterKey::default(), 2));
        assert_eq!(entries[0].count, 1);
    }

    /// A Move on day 0 with no squadron travelling.
    fn move_squadrons(
        world: &mut GameWorld,
        alliance: bool,
        class: FighterKey,
        count: u32,
        from: SquadronPlace,
        to: SquadronPlace,
    ) -> Result<SquadronMove, SquadronMoveError> {
        SquadronTransits::default().move_squadrons(world, alliance, class, count, from, to, 0)
    }

    fn check_move(
        world: &GameWorld,
        alliance: bool,
        class: FighterKey,
        count: u32,
        from: SquadronPlace,
        to: SquadronPlace,
    ) -> Result<(), SquadronMoveError> {
        SquadronTransits::default().check(world, alliance, class, count, from, to)
    }

    #[test]
    fn squadrons_add_to_their_class_entry_and_leave_an_emptied_one() {
        let s = setup(0);
        let mut entries = Vec::new();
        add(&mut entries, s.x_wing, 2);
        add(&mut entries, s.tie, 1);
        add(&mut entries, s.x_wing, 1);
        add(&mut entries, s.tie, 0);
        assert_eq!(total(&entries), 4);
        assert_eq!(entries.len(), 2);
        assert!(!take(&mut entries, s.tie, 2));
        assert!(take(&mut entries, s.tie, 1));
        assert_eq!(entries.len(), 1);
        assert!(!take(&mut entries, s.tie, 1));
        assert!(take(&mut entries, s.x_wing, 3));
        assert!(entries.is_empty());
    }

    #[test]
    fn a_fleet_s_room_is_its_living_ships_capacity_less_the_squadrons_aboard() {
        // FUN_00500b40: +0x26c (CAPSHPSD fighter_capacity) less the
        // squadrons aboard.
        let mut s = setup(3);
        s.world.fleets[s.fleet].fighters.push(FighterEntry {
            class: s.x_wing,
            count: 1,
            carrier: None,
            damage: None,
        });
        assert_eq!(fleet_room(&s.world, s.fleet), 2);
        s.world.fleets[s.fleet].capital_ships[0].alive = false;
        assert_eq!(fleet_room(&s.world, s.fleet), 0);
        assert_eq!(fleet_room(&s.world, FleetKey::default()), 0);
    }

    #[test]
    fn a_side_counts_only_its_own_squadrons_on_a_planet() {
        let mut s = setup(0);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 2);
        add(&mut s.world.systems[s.here].fighters, s.tie, 1);
        assert_eq!(on_planet(&s.world, s.here, true), 2);
        assert_eq!(on_planet(&s.world, s.here, false), 1);
        assert_eq!(on_planet(&s.world, s.there, true), 0);
    }

    #[test]
    fn a_planet_s_squadrons_board_a_fleet_in_orbit_up_to_its_room() {
        // FUN_00557ce0: a squadron's Move (0x201); FUN_00500b40 refuses a
        // fleet without room (0x14/1).
        let mut s = setup(2);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 3);
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        assert_eq!(
            move_squadrons(&mut s.world, true, s.x_wing, 3, planet, fleet),
            Err(SquadronMoveError::NoRoom)
        );
        assert_eq!(total(&s.world.systems[s.here].fighters), 3);
        move_squadrons(&mut s.world, true, s.x_wing, 2, planet, fleet).unwrap();
        assert_eq!(total(&s.world.fleets[s.fleet].fighters), 2);
        assert_eq!(total(&s.world.systems[s.here].fighters), 1);
        assert_eq!(fleet_room(&s.world, s.fleet), 0);
        // Boarding a fleet joins no system (FUN_00508660 is the system's).
        assert_eq!(s.world.systems[s.here].last_arrival, None);
    }

    #[test]
    fn a_fleet_s_squadrons_land_on_its_own_planet() {
        let mut s = setup(2);
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 2);
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        move_squadrons(&mut s.world, true, s.x_wing, 1, fleet, planet).unwrap();
        assert_eq!(total(&s.world.fleets[s.fleet].fighters), 1);
        assert_eq!(on_planet(&s.world, s.here, true), 1);
        // FUN_00508660 -> FUN_0050a1b0: the planet's system records the
        // squadron's side.
        assert_eq!(
            s.world.systems[s.here].last_arrival,
            Some(crate::dat::Faction::Alliance)
        );
        // The carrier stays: a fleet with a ship does not disband.
        move_squadrons(&mut s.world, true, s.x_wing, 1, fleet, planet).unwrap();
        assert!(s.world.fleets.contains_key(s.fleet));
    }

    #[test]
    fn a_fleet_of_squadrons_alone_disbands_when_the_last_lands() {
        let mut s = setup(0);
        s.world.fleets[s.fleet].capital_ships.clear();
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 2);
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        move_squadrons(&mut s.world, true, s.x_wing, 1, fleet, planet).unwrap();
        assert!(s.world.fleets.contains_key(s.fleet));
        move_squadrons(&mut s.world, true, s.x_wing, 1, fleet, planet).unwrap();
        assert!(!s.world.fleets.contains_key(s.fleet));
        assert!(s.world.systems[s.here].fleets.is_empty());
        assert_eq!(on_planet(&s.world, s.here, true), 2);
    }

    #[test]
    fn a_squadron_move_is_refused_for_the_wrong_side_count_or_place() {
        let mut s = setup(4);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 1);
        add(&mut s.world.systems[s.here].fighters, s.tie, 1);
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        let check = |world: &GameWorld, alliance, class, count, from, to| {
            check_move(world, alliance, class, count, from, to)
        };
        assert_eq!(
            check(&s.world, true, s.tie, 1, planet, fleet),
            Err(SquadronMoveError::WrongSide)
        );
        // FUN_00553410 (1/0x24): an Empire squadron onto an Alliance fleet.
        assert_eq!(
            check(&s.world, false, s.tie, 1, planet, fleet),
            Err(SquadronMoveError::OtherSideDestination)
        );
        assert_eq!(
            check(&s.world, true, s.x_wing, 2, planet, fleet),
            Err(SquadronMoveError::MissingSquadron)
        );
        assert_eq!(
            check(&s.world, true, s.x_wing, 0, planet, fleet),
            Err(SquadronMoveError::MissingSquadron)
        );
        assert_eq!(
            check(&s.world, true, s.x_wing, 1, planet, planet),
            Err(SquadronMoveError::IntoItself)
        );
        // FUN_00556390: a squadron travels to another system on its own.
        assert_eq!(
            check(
                &s.world,
                true,
                s.x_wing,
                1,
                planet,
                SquadronPlace::Planet(s.there)
            ),
            Ok(())
        );
        // FUN_0053d430's 1/0x28 refuses a planet whose side bits are not the
        // mover's: another side's, and a neutral one too.
        s.world.systems[s.here].control = ControlKind::Controlled(Faction::Empire);
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 1);
        assert_eq!(
            check(&s.world, true, s.x_wing, 1, fleet, planet),
            Err(SquadronMoveError::OtherSidePlanet)
        );
        s.world.systems[s.here].control = ControlKind::Uncontrolled;
        assert_eq!(
            check(&s.world, true, s.x_wing, 1, fleet, planet),
            Err(SquadronMoveError::OtherSidePlanet)
        );
        s.world.systems[s.here].control = ControlKind::Controlled(Faction::Alliance);
        assert_eq!(check(&s.world, true, s.x_wing, 1, fleet, planet), Ok(()));
        assert_eq!(
            check(
                &s.world,
                true,
                s.x_wing,
                1,
                SquadronPlace::Planet(SystemKey::default()),
                fleet
            ),
            Err(SquadronMoveError::MissingSystem)
        );
        assert_eq!(
            check(
                &s.world,
                true,
                s.x_wing,
                1,
                SquadronPlace::Fleet(FleetKey::default()),
                planet
            ),
            Err(SquadronMoveError::MissingFleet)
        );
    }

    #[test]
    fn a_fleet_in_hyperspace_neither_gives_nor_takes_squadrons() {
        // Manual p. 121: a fleet in hyperspace takes no orders.
        let mut s = setup(2);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 1);
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 1);
        s.world.systems[s.here].fleets.clear();
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        assert_eq!(
            check_move(&s.world, true, s.x_wing, 1, planet, fleet),
            Err(SquadronMoveError::InTransit)
        );
        assert_eq!(
            check_move(&s.world, true, s.x_wing, 1, fleet, planet),
            Err(SquadronMoveError::InTransit)
        );
    }

    #[test]
    fn move_is_enabled_for_the_side_s_own_squadron_on_a_planet_or_fleet_in_orbit() {
        // FUN_004f9860: the object is the order's side and not en route.
        let mut s = setup(1);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 1);
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 1);
        let (planet, fleet) = (SquadronPlace::Planet(s.here), SquadronPlace::Fleet(s.fleet));
        assert!(move_enabled(&s.world, true, s.x_wing, planet));
        assert!(move_enabled(&s.world, true, s.x_wing, fleet));
        assert!(!move_enabled(&s.world, false, s.x_wing, planet));
        assert!(!move_enabled(&s.world, true, s.tie, planet));
        assert!(!move_enabled(
            &s.world,
            true,
            s.x_wing,
            SquadronPlace::Planet(s.there)
        ));
        // Another class of the side, or an emptied entry, has no squadron
        // to move.
        let a_wing = s.world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            ..FighterClass::default()
        });
        assert!(!move_enabled(&s.world, true, a_wing, planet));
        s.world.systems[s.there].fighters.push(FighterEntry {
            class: s.x_wing,
            count: 0,
            carrier: None,
            damage: None,
        });
        assert!(!move_enabled(
            &s.world,
            true,
            s.x_wing,
            SquadronPlace::Planet(s.there)
        ));
        s.world.systems[s.here].fleets.clear();
        assert!(!move_enabled(&s.world, true, s.x_wing, fleet));
    }

    #[test]
    fn a_squadron_may_not_move_onto_its_sides_blockaded_planet() {
        // FUN_00552d10 -> FUN_00550700 asks the destination system's
        // acceptance (slot +0x74, System_vt74 FUN_005073d0) with the
        // sub-order kind and the mover's family: a blockaded system (+0x88
        // bit 0x20) gives 0x90/4, lifted only for 0x204 fleets (0x08..0x10),
        // 0x240/0x241 personnel and kind 0x3f0, so not for a squadron's
        // 0x204 (0x1c..0x1f).
        let mut s = setup(2);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 1);
        let (here, there) = (
            SquadronPlace::Planet(s.here),
            SquadronPlace::Planet(s.there),
        );
        let blockader = s.world.fleets.insert(Fleet {
            location: s.there,
            ..Default::default()
        });
        s.world.systems[s.there].fleets.push(blockader);
        assert_eq!(
            check_move(&s.world, true, s.x_wing, 1, here, there),
            Err(SquadronMoveError::DestinationBlockaded)
        );
        // The same planet without the blockade takes it.
        s.world.systems[s.there].fleets.clear();
        assert_eq!(check_move(&s.world, true, s.x_wing, 1, here, there), Ok(()));
        assert_eq!(
            SquadronMoveError::DestinationBlockaded.to_string(),
            "the destination planet is blockaded"
        );
    }

    #[test]
    fn a_refusal_reads_as_the_player_s_message() {
        // Our own wording for the advisor's line the app shows.
        assert_eq!(
            SquadronMoveError::NoRoom.to_string(),
            "the fleet has no room for fighters"
        );
        assert_eq!(
            SquadronMoveError::OtherSidePlanet.to_string(),
            "the destination planet belongs to another side"
        );
    }

    #[test]
    fn a_side_is_present_by_a_fleet_or_a_squadron_on_the_planet() {
        // FUN_00509710: an active fleet (FUN_004ffef0) or squadron
        // (FUN_00503a50) of the side.
        let mut s = setup(1);
        assert_eq!(sides_present(&s.world, s.here), (true, false));
        add(&mut s.world.systems[s.here].fighters, s.tie, 1);
        assert_eq!(sides_present(&s.world, s.here), (true, true));
        assert_eq!(sides_present(&s.world, s.there), (false, false));
        add(&mut s.world.systems[s.there].fighters, s.x_wing, 1);
        assert_eq!(sides_present(&s.world, s.there), (true, false));
        s.world.systems[s.there].fighters[0].count = 0;
        assert_eq!(sides_present(&s.world, s.there), (false, false));
        assert_eq!(
            sides_present(&s.world, SystemKey::default()),
            (false, false)
        );
    }

    #[test]
    fn a_battle_sends_each_side_s_planet_squadrons_up_and_lands_the_survivors() {
        // port: a planet's squadrons fight beside their side's fleet, or as
        // a group of their own (Sortie); the survivors land again.
        let mut s = setup(1);
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 1);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 2);
        add(&mut s.world.systems[s.here].fighters, s.tie, 3);
        let sorties = scramble(&mut s.world, s.here);

        assert_eq!(sorties.len(), 2);
        assert!(s.world.systems[s.here].fighters.is_empty());
        assert_eq!((sorties[0].fleet, sorties[0].group), (s.fleet, false));
        assert_eq!(total(&s.world.fleets[s.fleet].fighters), 3);
        let group = sorties[1].fleet;
        assert!(sorties[1].group);
        assert!(!s.world.fleets[group].is_alliance);
        assert_eq!(total(&s.world.fleets[group].fighters), 3);
        assert!(s.world.systems[s.here].fleets.contains(&group));

        // The battle costs the Alliance two squadrons and the Empire one.
        take(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 2);
        take(&mut s.world.fleets[group].fighters, s.tie, 1);
        recover(&mut s.world, &sorties);

        assert!(!s.world.fleets.contains_key(group));
        assert!(!s.world.systems[s.here].fleets.contains(&group));
        assert!(s.world.fleets[s.fleet].fighters.is_empty());
        assert_eq!(on_planet(&s.world, s.here, true), 1);
        assert_eq!(on_planet(&s.world, s.here, false), 2);
    }

    #[test]
    fn a_fleet_whose_last_ship_fell_disbands_once_the_planet_s_squadrons_land() {
        // The planet's squadrons fight beside the fleet (port: Sortie) but
        // never belong to it (FUN_00516ef0 counts them apart). When the
        // fleet's last ship is destroyed, the survivors land and the fleet,
        // holding nothing, disbands (FUN_004fe630). Seed 1's 5,000-day
        // campaign, day 2093: the fleet stayed, empty.
        let mut s = setup(1);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 2);
        let sorties = scramble(&mut s.world, s.here);
        assert_eq!((sorties[0].fleet, sorties[0].group), (s.fleet, false));
        crate::scrap::destroy_ship(&mut s.world, s.fleet, 0);
        assert!(s.world.fleets.contains_key(s.fleet));
        recover(&mut s.world, &sorties);

        assert!(!s.world.fleets.contains_key(s.fleet));
        assert!(!s.world.systems[s.here].fleets.contains(&s.fleet));
        assert_eq!(on_planet(&s.world, s.here, true), 2);
    }

    /// A squadron of `class` at `carrier` that has lost `lost` craft.
    fn damaged(class: FighterKey, carrier: Option<u64>, lost: u32) -> FighterEntry {
        FighterEntry {
            class,
            count: 1,
            carrier,
            damage: Some(SquadronDamage { lost, repair: None }),
        }
    }

    #[test]
    fn a_battle_sets_each_survivors_losses_from_the_craft_that_came_back() {
        // FUN_0040d260: losses +0x60 = size less the craft the result word
        // gives (FUN_00503370 adds them); a squadron with none is destroyed
        // (FUN_00503370 -> slot +0xac). The others keep their entry's index.
        let class = FighterKey::default();
        let timer = Some(RepairTimer { due: 9, fast: false });
        let mut entries = vec![
            FighterEntry { class, count: 3, carrier: Some(1), damage: None },
            FighterEntry {
                damage: Some(SquadronDamage { lost: 2, repair: timer }),
                ..damaged(class, None, 2)
            },
            FighterEntry { class, count: 2, carrier: None, damage: None },
            FighterEntry { class, count: 2, carrier: None, damage: None },
        ];
        apply_battle_craft(&mut entries, 0, 12, 2, &[12, 7]);
        // Already damaged, it keeps its timer (still Damaged).
        apply_battle_craft(&mut entries, 1, 12, 1, &[4]);
        apply_battle_craft(&mut entries, 2, 12, 2, &[3, 11]);
        apply_battle_craft(&mut entries, 3, 12, 0, &[]);

        assert_eq!(
            entries[..4],
            [
                FighterEntry { class, count: 1, carrier: Some(1), damage: None },
                FighterEntry {
                    damage: Some(SquadronDamage { lost: 8, repair: timer }),
                    ..damaged(class, None, 8)
                },
                damaged(class, None, 9),
                FighterEntry { class, count: 0, carrier: None, damage: None },
            ]
        );
        assert_eq!(entries[4..], [damaged(class, Some(1), 5), damaged(class, None, 1)]);
        // An event with no craft listed keeps whole squadrons; a lost one
        // keeps no losses.
        let mut old = vec![
            FighterEntry { class, count: 3, carrier: None, damage: None },
            damaged(class, None, 4),
            damaged(class, None, 4),
        ];
        apply_battle_craft(&mut old, 0, 12, 2, &[]);
        apply_battle_craft(&mut old, 1, 12, 0, &[]);
        apply_battle_craft(&mut old, 2, 12, 1, &[]);
        assert_eq!(
            old,
            [
                FighterEntry { class, count: 2, carrier: None, damage: None },
                FighterEntry { class, count: 0, carrier: None, damage: None },
                damaged(class, None, 4),
            ]
        );
        // An emptied damaged entry adds nothing.
        put_entry(&mut old, FighterEntry { count: 0, ..damaged(class, None, 1) });
        assert_eq!(old.len(), 3);
    }

    #[test]
    fn a_damaged_squadron_keeps_its_own_entry_and_its_losses_when_it_moves() {
        // A squadron's losses are its own (+0x60): it never merges with
        // whole ones. Moved within its system it stays Usable and keeps its
        // timer; across systems it travels (not Usable, FUN_005578d0) and
        // lands with its losses and no timer.
        let mut s = setup(4);
        s.world.systems[s.there].x = 300;
        let timer = Some(RepairTimer { due: 40, fast: false });
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 1);
        s.world.systems[s.here].fighters.push(FighterEntry {
            damage: Some(SquadronDamage { lost: 4, repair: timer }),
            ..damaged(s.x_wing, None, 4)
        });
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 1);
        assert_eq!(s.world.systems[s.here].fighters.len(), 2);

        let mut transits = SquadronTransits::default();
        let here = SquadronPlace::Planet(s.here);
        let fleet = SquadronPlace::Fleet(s.fleet);
        // Both go, the damaged one in its own entry.
        transits.move_squadrons(&mut s.world, true, s.x_wing, 2, here, fleet, 1).unwrap();
        let aboard = &s.world.fleets[s.fleet].fighters;
        assert_eq!(total(aboard), 2);
        assert!(aboard.iter().any(|entry| entry.damage
            == Some(SquadronDamage { lost: 4, repair: timer })));

        // Within the system, off a ship onto the planet: on no ship.
        for entry in &mut s.world.fleets[s.fleet].fighters {
            entry.carrier = Some(7);
        }
        transits.move_squadrons(&mut s.world, true, s.x_wing, 2, fleet, here, 2).unwrap();
        assert!(s.world.systems[s.here].fighters.iter().all(|entry| entry.carrier.is_none()));
        transits.move_squadrons(&mut s.world, true, s.x_wing, 2, here, fleet, 2).unwrap();
        let moved = transits
            .move_squadrons(&mut s.world, true, s.x_wing, 2, fleet, SquadronPlace::Planet(s.there), 2)
            .unwrap();
        let SquadronMove::Departed(day) = moved else {
            panic!("{moved:?}");
        };
        assert_eq!(transits.transits()[0].lost, [4]);
        transits.advance(&mut s.world, day);
        let landed = &s.world.systems[s.there].fighters;
        assert_eq!(total(landed), 2);
        assert_eq!(
            landed.iter().filter_map(|entry| entry.damage).collect::<Vec<_>>(),
            [SquadronDamage { lost: 4, repair: None }]
        );
    }

    #[test]
    fn taken_squadrons_come_as_their_entries_without_empty_ones() {
        let class = FighterKey::default();
        let mut entries = vec![
            FighterEntry { class, count: 2, carrier: Some(1), damage: None },
            damaged(class, None, 3),
            FighterEntry { class, count: 0, carrier: None, damage: None },
        ];
        assert_eq!(
            take_entries(&mut entries, class, 2),
            Some(vec![
                damaged(class, None, 3),
                FighterEntry { class, count: 1, carrier: Some(1), damage: None },
            ])
        );
        assert_eq!(take_entries(&mut entries, class, 2), None);
    }

    #[test]
    fn a_squadron_no_ship_has_room_for_rides_on_none() {
        // seat_squadrons: a squadron whose ship is gone and finds no room
        // stays in the fleet on no ship, its losses kept.
        let mut s = setup(0);
        s.world.fleets[s.fleet].fighters = vec![damaged(s.x_wing, Some(99), 3)];
        s.world.number_ships();
        assert_eq!(s.world.fleets[s.fleet].fighters, [damaged(s.x_wing, None, 3)]);
    }

    #[test]
    fn a_planets_damaged_squadron_lands_again_with_its_losses() {
        let mut s = setup(1);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 1);
        s.world.systems[s.here].fighters.push(damaged(s.x_wing, None, 6));
        let sorties = scramble(&mut s.world, s.here);
        assert_eq!(total(&s.world.fleets[s.fleet].fighters), 2);
        // The fleet's own squadron aboard its ship, listed after them: the
        // planet takes its unseated ones first.
        s.world.fleets[s.fleet].fighters.push(FighterEntry {
            class: s.x_wing,
            count: 1,
            carrier: Some(1),
            damage: None,
        });
        recover(&mut s.world, &sorties);
        let landed = &s.world.systems[s.here].fighters;
        assert_eq!(total(landed), 2);
        assert!(landed.contains(&damaged(s.x_wing, None, 6)), "{landed:?}");
        assert_eq!(
            s.world.fleets[s.fleet].fighters,
            [FighterEntry { class: s.x_wing, count: 1, carrier: Some(1), damage: None }]
        );

        // One seated meanwhile lands on no ship.
        let sorties = scramble(&mut s.world, s.here);
        for entry in &mut s.world.fleets[s.fleet].fighters {
            entry.carrier = Some(1);
        }
        recover(&mut s.world, &sorties);
        assert!(s.world.systems[s.here].fighters.iter().all(|entry| entry.carrier.is_none()));
    }

    #[test]
    fn the_save_records_put_back_each_squadrons_losses_and_an_old_save_is_whole() {
        // SaveState::squadron_damage (v126): the world's and the transits'
        // encodings skip the losses; the records carry them.
        let mut s = setup(2);
        s.world.systems[s.there].x = 300;
        let timer = Some(RepairTimer { due: 12, fast: true });
        s.world.fleets[s.fleet].fighters.push(FighterEntry {
            damage: Some(SquadronDamage { lost: 3, repair: timer }),
            ..damaged(s.x_wing, None, 3)
        });
        s.world.systems[s.here].fighters.push(damaged(s.x_wing, None, 2));
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 1);
        let mut transits = SquadronTransits::default();
        transits
            .move_squadrons(
                &mut s.world,
                true,
                s.x_wing,
                2,
                SquadronPlace::Planet(s.here),
                SquadronPlace::Planet(s.there),
                1,
            )
            .unwrap();
        let records = damaged_squadrons(&s.world, &transits);
        assert_eq!(records.len(), 2);

        let encoded = serde_json::to_string(&(&s.world.fleets[s.fleet].fighters, &transits)).unwrap();
        let (fighters, mut loaded): (Vec<FighterEntry>, SquadronTransits) =
            serde_json::from_str(&encoded).unwrap();
        assert!(fighters.iter().all(|entry| entry.damage.is_none()));
        assert!(loaded.transits()[0].lost.is_empty());
        let mut world = s.world.clone();
        world.fleets[s.fleet].fighters = fighters;
        restore_damaged_squadrons(&mut world, &mut loaded, &records);
        assert_eq!(world.fleets[s.fleet].fighters, s.world.fleets[s.fleet].fighters);
        assert_eq!(loaded, transits);

        restore_damaged_squadrons(&mut world, &mut loaded, &[]);
        assert!(damaged_squadrons(&world, &loaded).is_empty());
    }

    #[test]
    fn a_destroyed_fleet_or_group_lands_nothing() {
        let mut s = setup(1);
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 2);
        add(&mut s.world.systems[s.here].fighters, s.tie, 1);
        let sorties = scramble(&mut s.world, s.here);
        for sortie in &sorties {
            s.world.fleets.remove(sortie.fleet);
        }
        recover(&mut s.world, &sorties);
        assert!(s.world.systems[s.here].fighters.is_empty());
        assert_eq!(scramble(&mut s.world, s.there).len(), 0);
        // An emptied entry sends nothing up.
        s.world.systems[s.there].fighters.push(FighterEntry {
            class: s.tie,
            count: 0,
            carrier: None,
            damage: None,
        });
        assert!(scramble(&mut s.world, s.there).is_empty());
        assert!(scramble(&mut s.world, SystemKey::default()).is_empty());
    }

    #[test]
    fn a_squadrons_move_days_are_the_trips_and_nothing_moves() {
        // FUN_0053c2e0: the Confirmed Move's line gives the days the move
        // itself would take at the class's speed (FUN_00502f80). port: none
        // within one system, as the move places it at once.
        let mut s = setup(1);
        s.world.systems[s.there].x = 300;
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 2);
        let transits = SquadronTransits::default();
        let here = SquadronPlace::Planet(s.here);
        let days = |count, to| transits.move_days(&s.world, true, s.x_wing, count, here, to);
        let class = &s.world.fighter_classes[s.x_wing];
        let ticks = crate::movement::transit_ticks_between(
            &s.world,
            (0, 0),
            (300, 0),
            crate::movement::fighter_speed(&s.world, class),
        );
        assert!(ticks > 1);

        assert_eq!(days(1, SquadronPlace::Planet(s.there)), Ok(ticks));
        assert_eq!(days(1, SquadronPlace::Fleet(s.fleet)), Ok(0));
        for (count, to) in [(3, SquadronPlace::Planet(s.there)), (1, here)] {
            let refusal = transits.check(&s.world, true, s.x_wing, count, here, to);
            assert!(refusal.is_err());
            assert_eq!(days(count, to), refusal.map(|()| 0));
        }
        assert_eq!(on_planet(&s.world, s.here, true), 2);
        assert!(transits.transits().is_empty());
    }

    #[test]
    fn a_squadron_sent_to_another_system_travels_and_lands_there() {
        // FUN_00556390 / FUN_00556430: across systems the squadron leaves at
        // once and arrives after the transit at its speed (FUN_00502f80).
        let mut s = setup(1);
        s.world.systems[s.there].x = 300;
        add(&mut s.world.systems[s.here].fighters, s.x_wing, 2);
        let mut transits = SquadronTransits::default();
        let moved = transits
            .move_squadrons(
                &mut s.world,
                true,
                s.x_wing,
                1,
                SquadronPlace::Planet(s.here),
                SquadronPlace::Planet(s.there),
                10,
            )
            .unwrap();
        let SquadronMove::Departed(day) = moved else {
            panic!("{moved:?}");
        };
        let class = &s.world.fighter_classes[s.x_wing];
        let ticks = crate::movement::transit_ticks_between(
            &s.world,
            (0, 0),
            (300, 0),
            crate::movement::fighter_speed(&s.world, class),
        );
        assert!(ticks > 1);
        assert_eq!(day, 10 + u64::from(ticks));
        assert_eq!(on_planet(&s.world, s.here, true), 1);
        assert_eq!(on_planet(&s.world, s.there, true), 0);
        assert_eq!(transits.transits().len(), 1);

        assert_eq!(transits.advance(&mut s.world, day - 1), (vec![], vec![]));
        let (arrived, lost) = transits.advance(&mut s.world, day);
        assert_eq!((arrived.len(), lost.len()), (1, 0));
        assert_eq!(on_planet(&s.world, s.there, true), 1);
        assert!(transits.transits().is_empty());
    }

    #[test]
    fn a_squadron_on_its_way_to_a_fleet_takes_its_room_and_boards_on_arrival() {
        // FUN_00500b40 at the order: the room counts the squadrons on their
        // way; the original puts the squadron in its fleet at once.
        let mut s = setup(1);
        s.world.systems[s.there].x = 300;
        add(&mut s.world.systems[s.there].fighters, s.x_wing, 2);
        let mut transits = SquadronTransits::default();
        let (from, to) = (
            SquadronPlace::Planet(s.there),
            SquadronPlace::Fleet(s.fleet),
        );
        let SquadronMove::Departed(day) = transits
            .move_squadrons(&mut s.world, true, s.x_wing, 1, from, to, 0)
            .unwrap()
        else {
            panic!("no transit");
        };
        assert_eq!(
            transits.check(&s.world, true, s.x_wing, 1, from, to),
            Err(SquadronMoveError::NoRoom)
        );
        transits.advance(&mut s.world, day);
        assert_eq!(total(&s.world.fleets[s.fleet].fighters), 1);

        // A planet destroyed before the arrival loses it (FUN_004fc080).
        transits
            .move_squadrons(
                &mut s.world,
                true,
                s.x_wing,
                1,
                from,
                SquadronPlace::Planet(s.here),
                0,
            )
            .unwrap();
        s.world.systems[s.here].is_destroyed = true;
        let (arrived, lost) = transits.advance(&mut s.world, u64::MAX);
        assert_eq!((arrived.len(), lost.len()), (0, 1));
    }

    #[test]
    fn squadrons_pass_between_two_fleets_in_one_system() {
        let mut s = setup(2);
        let other = s.world.fleets.insert(Fleet {
            location: s.here,
            ..s.world.fleets[s.fleet].clone()
        });
        s.world.systems[s.here].fleets.push(other);
        add(&mut s.world.fleets[s.fleet].fighters, s.x_wing, 2);
        move_squadrons(
            &mut s.world,
            true,
            s.x_wing,
            2,
            SquadronPlace::Fleet(s.fleet),
            SquadronPlace::Fleet(other),
        )
        .unwrap();
        assert_eq!(total(&s.world.fleets[other].fighters), 2);
        assert!(s.world.fleets[s.fleet].fighters.is_empty());
    }
}
