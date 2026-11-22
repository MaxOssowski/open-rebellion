//! Module 9 of the AI player (`FUN_004cf0a0`, step `FUN_004cf290` ->
//! `FUN_004cf2c0`): one planner (`FUN_00474070`, vtable `0x65a3a0`) for
//! each of the side's headquarters systems that is not blockaded. A
//! planner holds capital ships and one fleet. It gathers heavy ships to
//! its system until the fleet's attack covers the side's best heavy ship
//! class, gives back what is over, lands the regiments and people its
//! fleet carries, and brings the fleet home
//! (`ghidra/notes/ai-module-9.md`).
//!
//! port:
//! - A fleet's regiments and people ride its first ship's record
//!   ([`crate::ai_records::FleetRecord::build`]), so the ship that carries
//!   them is that one.
//! - Module 9 moves ships into its fleet with
//!   [`AIAction::JoinShips`], the ships' Move `0x201` with a fleet for its
//!   target.
//! - Controller `+0x164` is
//!   [`crate::ai_records::ClassFloors::heavy_attack`], computed with the
//!   records.
//! - As for the regiments, one refused person refuses the people's Move
//!   ([`crate::ai`]'s validator), so a fleet carrying a prisoner of the
//!   other side lands no one.

use serde::{Deserialize, Serialize};

use crate::ai::AIAction;
use crate::ai_module1::{Step, ANY};
use crate::ai_planner::{random_system, PlannerIo};
use crate::ai_records::{
    picks, ranked, sector_at, Galaxy, Mask, Order, ShipRecord, FLEET_EN_ROUTE,
    FLEET_FREE, FLEET_OWN, SHIP_FREE, SHIP_OWN,
};
use crate::ids::{FleetKey, SectorKey, SystemKey};

/// Ship record `+0x24` bit `0x10000000`: a module 9 planner holds the ship
/// (`FUN_00475e70` sets it, `FUN_00475f60` clears it).
pub const SHIP_HELD: u32 = 0x1000_0000;

/// Fleet record `+0x38` bit 8: a module 9 planner's fleet (`FUN_00475fe0`
/// sets it, `FUN_00476080` clears it).
pub const FLEET_HELD: u32 = 8;

/// System record `+0x30` bit `0x80000`: a module 9 planner's headquarters
/// (`FUN_004744b0`).
pub const KIND_CLAIMED: u32 = 0x8_0000;

/// Sector record `+0x24` bit `0x40000000`: an own system with `+0x28`
/// `0x80000000` (`FUN_00432350`).
const SECTOR_HEADQUARTERS: u32 = 0x4000_0000;

/// System record `+0x28` bit `0x80000000`: the side's headquarters, or
/// Coruscant (`0x109`) (`FUN_00433e40`).
const STATE_HEADQUARTERS: u32 = 0x8000_0000;

/// System record `+0x28` bit 2: blockaded (`FUN_00433e40`).
const STATE_BLOCKADED: u32 = 2;

/// System record `+0x30` bit 1: the side's own.
const KIND_OWN: u32 = 1;

/// Ship record `+0x24` bits a ship taken for the fleet may not have
/// (`FUN_00475490`): en route `0x800`, damaged `0x1000`/`0x800000`,
/// inactive `0x2000`.
const UNFIT: u32 = 0x80_3800;

/// Ship class flag 1 (`FUN_004739b0`): turbolasers or ion cannons, no
/// gravity well.
const HEAVY: u32 = 1;

/// Controller `+4` bit 0: a Headquarters Only game (`FUN_004182a0`), which
/// doubles what the fleet should hold (`FUN_00474b90`).
const HEADQUARTERS_ONLY: u32 = crate::ai::LEADERS_UNMARKED;

/// Module 9's own fields (`FUN_004cf0a0`: `+0x3c..+0x44`) and the
/// controller's planner list (`+0x11c`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module9 {
    /// `+0x3c`: the step's state.
    pub state: u32,
    /// `+0x40`: the planner this cycle steps.
    pub current: u32,
    /// `+0x44`: the planner the next cycle steps.
    pub next: u32,
    /// Controller `+0x11c`: the planners, in id order.
    pub planners: Vec<HqPlanner>,
    /// The list's next id (`FUN_005f5440`; the list counts from 1).
    pub next_id: u32,
}

impl Default for Module9 {
    fn default() -> Self {
        Self {
            state: 0,
            current: 0,
            next: 0,
            planners: Vec::new(),
            next_id: 1,
        }
    }
}

/// A headquarters planner (`FUN_00474070`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HqPlanner {
    /// `+0x18`.
    pub id: u32,
    /// `+0x1c`: the step's state (`FUN_00474390`).
    pub state: u32,
    /// `+0x24`: the ships it holds, by id.
    pub ships: Vec<u64>,
    /// `+0x2c`: its fleet.
    pub fleet: Option<FleetKey>,
    /// `+0x30`: its headquarters system.
    pub system: Option<SystemKey>,
    /// `+0x34`: the fleet's attack less what it should hold.
    pub balance: i32,
    /// `+0x38`: the lists want checking against the records.
    pub dirty: bool,
}

/// Controller `+0x224` (`FUN_0041b7e0`): the sector records' `+0x1a0`, the
/// side's headquarters systems not blockaded.
#[must_use]
pub fn headquarters_count(galaxy: &Galaxy) -> i32 {
    galaxy
        .sectors
        .iter()
        .map(|s| s.values[sector_at(0x1a0)])
        .sum()
}

impl Module9 {
    /// The strategy's state 5 (`FUN_004e69d0`): with fewer planners than
    /// `wanted` (controller `+0x224`) one more (`FUN_0042ed50`); with more,
    /// the list's head leaves when it has no system, giving everything back
    /// (`FUN_0042edc0`, `FUN_004742b0`).
    pub fn balance(&mut self, io: &mut PlannerIo<'_>, wanted: i32) {
        let count = i32::try_from(self.planners.len()).unwrap_or(i32::MAX);
        if count < wanted {
            let id = self.next_id;
            self.next_id += 1;
            self.planners.push(HqPlanner {
                id,
                ..HqPlanner::default()
            });
        } else if wanted < count && self.planners.first().is_some_and(|p| p.system.is_none()) {
            self.planners[0].release_all(io);
            self.planners.remove(0);
        }
    }

    /// One step (`FUN_004cf2c0`). `need` is controller `+0x164`
    /// ([`crate::ai_records::ClassFloors::heavy_attack`]).
    pub fn step(&mut self, io: &mut PlannerIo<'_>, need: i32) -> Step {
        match self.state {
            2 => {
                if self.choose() {
                    self.state = 3;
                    Step::pending()
                } else {
                    self.state = 0;
                    Step::complete()
                }
            }
            3 => {
                // FUN_004cf3b0: a planner no longer listed is done.
                let Some(i) = self.planners.iter().position(|p| p.id == self.current) else {
                    self.state = 0;
                    return Step::complete();
                };
                let (order, done) = self.planners[i].step(io, need);
                if done {
                    self.state = 0;
                }
                match order {
                    Some(order) => Step {
                        order: Some(order),
                        complete: true,
                        raise: !done,
                    },
                    None if done => Step::complete(),
                    None => Step::pending(),
                }
            }
            _ => {
                self.state = 2;
                Step::pending()
            }
        }
    }

    /// A refused order (slot `+0x18`, `FUN_004dd810`).
    pub fn refused(&mut self) {
        self.state = 0;
    }

    /// `FUN_004cf350`: the planner `+0x44` names, else the first, into
    /// `+0x40`, and the one after it into `+0x44`.
    fn choose(&mut self) -> bool {
        let at = self
            .planners
            .iter()
            .position(|p| p.id == self.next)
            .or_else(|| (!self.planners.is_empty()).then_some(0));
        let Some(at) = at else {
            self.current = 0;
            self.next = 0;
            return false;
        };
        self.current = self.planners[at].id;
        self.next = self.planners.get(at + 1).map_or(0, |p| p.id);
        true
    }
}

/// `FUN_00484510`: a ship whose fleet has a role (`+0x38` `0xf0000000`) or
/// stands at a system of the opponent's (`+0x30` 4).
fn busy(io: &PlannerIo<'_>, ship: &ShipRecord) -> bool {
    let Some(fleet) = io.galaxy.fleets.get(&ship.fleet) else {
        return false;
    };
    let at_opponent = io
        .world
        .fleets
        .get(ship.fleet)
        .and_then(|f| io.galaxy.system(f.location))
        .is_some_and(|r| r.kind & 4 != 0);
    at_opponent || fleet.flags & 0xf000_0000 != 0
}

impl HqPlanner {
    /// One step (`FUN_00474390`): the refresh when the lists want it, then
    /// one state. Returns the order and whether the cycle ended.
    pub fn step(&mut self, io: &mut PlannerIo<'_>, need: i32) -> (Option<AIAction>, bool) {
        if self.dirty {
            self.refresh(io);
            self.dirty = false;
        }
        let mut done = false;
        let order = match self.state {
            1 => {
                self.state = if self.keep_system(io) { 2 } else { 8 };
                None
            }
            2 => {
                self.state = 3;
                self.split_damaged(io)
            }
            3 => {
                self.weigh(io, need);
                self.state = 4;
                None
            }
            4 => {
                // The step's flag slot is never 0 (0x47440f): state 4
                // always moves on.
                self.state = 5;
                self.gather(io)
            }
            5 => {
                self.state = 6;
                self.land_regiments(io)
            }
            6 => {
                self.state = 7;
                self.land_people(io)
            }
            7 => {
                self.state = 0;
                done = true;
                self.go_home(io)
            }
            8 => {
                self.release_all(io);
                self.state = 0;
                done = true;
                None
            }
            _ => {
                self.state = 1;
                self.dirty = true;
                None
            }
        };
        if order.is_some() {
            self.dirty = true;
        }
        (order, done)
    }

    /// `FUN_00475e70`: hold a ship. Its record loses `0x20000`, gains
    /// [`SHIP_HELD`] and names no sector planner; an attack (`+0x7c`) above
    /// 0 adds to the balance.
    fn take_ship(&mut self, io: &mut PlannerIo<'_>, id: u64) {
        let Some(attack) = io.ship(id).map(|r| r.values[14]) else {
            return;
        };
        io.edit_ship(id, |r| {
            r.flags = (r.flags & !SHIP_FREE) | SHIP_HELD;
            r.owner = 0;
            r.plan = 0;
        });
        if attack > 0 {
            self.balance += attack;
        }
        self.ships.push(id);
    }

    /// `FUN_00475f60`: give a held ship back. A record no sector planner
    /// names is free again (`& 0xecffffff | 0x20000`); its attack leaves
    /// the balance.
    fn release_ship(&mut self, io: &mut PlannerIo<'_>, id: u64) {
        let Some(at) = self.ships.iter().position(|&s| s == id) else {
            return;
        };
        if let Some(record) = io.ship(id).cloned() {
            if record.owner == 0 {
                io.edit_ship(id, |r| {
                    r.flags = (r.flags & 0xecff_ffff) | SHIP_FREE;
                    r.owner = 0;
                    r.plan = 0;
                });
            }
            self.balance -= record.values[14];
        }
        self.ships.remove(at);
    }

    /// `FUN_00475fe0`: take `fleet`: its record loses [`FLEET_FREE`] and
    /// gains [`FLEET_HELD`].
    fn claim_fleet(&mut self, io: &mut PlannerIo<'_>, fleet: FleetKey) {
        if !io.galaxy.fleets.contains_key(&fleet) {
            return;
        }
        io.edit_fleet(fleet, |r| {
            r.flags = (r.flags & !FLEET_FREE) | FLEET_HELD;
            r.owner = 0;
            r.plan = 0;
        });
        self.fleet = Some(fleet);
    }

    /// `FUN_00476080`: give the fleet back: its record is free again.
    fn release_fleet(&mut self, io: &mut PlannerIo<'_>) {
        let Some(fleet) = self.fleet.take() else {
            return;
        };
        io.edit_fleet(fleet, |r| {
            r.flags = (r.flags & !FLEET_HELD) | FLEET_FREE;
            r.owner = 0;
            r.plan = 0;
        });
    }

    /// `FUN_004742b0`: every ship and the fleet given back; the system's
    /// record loses [`KIND_CLAIMED`] and the planner forgets it.
    pub fn release_all(&mut self, io: &mut PlannerIo<'_>) {
        for id in self.ships.clone() {
            self.release_ship(io, id);
        }
        self.release_fleet(io);
        if let Some(system) = self.system {
            if io.galaxy.system(system).is_some() {
                self.system = None;
                io.set_kind(system, 0, KIND_CLAIMED);
            }
        }
    }

    /// `FUN_004747b0`: the fleet stays while its record is the side's,
    /// held ([`FLEET_HELD`]) and has a ship; each held ship of the side
    /// stays while it is in the fleet, or with no fleet while its own has
    /// a record, a free one of which becomes the planner's. The rest are
    /// given back.
    fn refresh(&mut self, io: &mut PlannerIo<'_>) {
        let keep = self
            .fleet
            .and_then(|f| io.galaxy.fleets.get(&f))
            .is_some_and(|r| {
                r.flags & FLEET_OWN != 0 && r.flags & FLEET_HELD != 0 && !r.ships.is_empty()
            });
        if !keep {
            self.release_fleet(io);
        }
        for id in self.ships.clone() {
            let kept = match io.ship(id).cloned() {
                Some(r) if r.flags & SHIP_OWN != 0 => match self.fleet {
                    Some(fleet) => fleet == r.fleet,
                    None => match io.galaxy.fleets.get(&r.fleet).map(|f| f.flags) {
                        Some(flags) => {
                            if flags & FLEET_FREE != 0 {
                                self.claim_fleet(io, r.fleet);
                            }
                            true
                        }
                        None => false,
                    },
                },
                _ => false,
            };
            if !kept {
                self.release_ship(io, id);
            }
        }
    }

    /// `FUN_004744b0`: keep the system while its record is the side's
    /// headquarters, not blockaded; else forget it and find one: the
    /// sectors with such a system (`FUN_0041a1d0`, random keys, order 1) in
    /// pick order, the first whose own headquarters, not blockaded and not
    /// claimed, is picked (`FUN_0041a460`). The kept or found system's
    /// record gets [`KIND_CLAIMED`].
    fn keep_system(&mut self, io: &mut PlannerIo<'_>) -> bool {
        let mut kept = false;
        if let Some(system) = self.system {
            if let Some(r) = io.galaxy.system(system) {
                if r.kind & KIND_OWN != 0
                    && r.state & STATE_HEADQUARTERS != 0
                    && r.state & STATE_BLOCKADED == 0
                {
                    kept = true;
                    io.set_kind(system, KIND_CLAIMED, 0);
                } else {
                    self.system = None;
                    io.set_kind(system, 0, KIND_CLAIMED);
                }
            }
        }
        if self.system.is_some() {
            return kept;
        }
        let sectors = random_sectors(io, Mask::new(SECTOR_HEADQUARTERS, 0));
        for sector in picks(vec![sectors]) {
            let masks = [
                Mask::new(STATE_HEADQUARTERS, STATE_BLOCKADED),
                ANY,
                Mask::new(KIND_OWN, KIND_CLAIMED),
            ];
            let Some(system) = random_system(io, sector, masks) else {
                continue;
            };
            if io.galaxy.system(system).is_some() {
                self.system = Some(system);
                io.set_kind(system, KIND_CLAIMED, 0);
                return true;
            }
        }
        false
    }

    /// `FUN_004749b0`: with damaged ships in the fleet (record `0x3000`,
    /// none of `0xf0000802`), those not en route (`0x801000` without
    /// `0x800`) leave as a fleet in its system (Create Fleet `0x270`).
    fn split_damaged(&self, io: &PlannerIo<'_>) -> Option<AIAction> {
        let fleet = self.fleet?;
        let record = io.galaxy.fleets.get(&fleet)?;
        if record.flags & 0x3000 == 0 || record.flags & 0xf000_0802 != 0 {
            return None;
        }
        let ships: Vec<usize> = record
            .ships
            .iter()
            .filter(|s| s.flags & 0x80_1000 != 0 && s.flags & 0x800 == 0)
            .map(|s| s.index)
            .collect();
        if ships.is_empty() {
            return None;
        }
        let to = io.world.fleets.get(fleet)?.location;
        Some(AIAction::CreateFleet { fleet, ships, to })
    }

    /// `FUN_00474b90`: the fleet's attack (record `+0xb4`, `values[25]`),
    /// 0 without one, less `need`, twice it in a Headquarters Only game.
    fn weigh(&mut self, io: &PlannerIo<'_>, need: i32) {
        let attack = self
            .fleet
            .and_then(|f| io.galaxy.fleets.get(&f))
            .map_or(0, |r| r.values[25]);
        let times = if io.controller_flags & HEADQUARTERS_ONLY == 0 {
            1
        } else {
            2
        };
        self.balance = attack - need * times;
    }

    /// `FUN_00474c00`: nothing while the fleet is en route; short, heavy
    /// ships from the nearest free fleets ([`Self::take_near`]), else from
    /// the sector plans' fleets ([`Self::take_planned`]); over, one ship
    /// given back ([`Self::spare`]).
    fn gather(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        if self
            .fleet
            .and_then(|f| io.galaxy.fleets.get(&f))
            .is_some_and(|r| r.flags & FLEET_EN_ROUTE != 0)
        {
            return None;
        }
        if self.balance < 0 {
            if let Some(order) = self.take_near(io) {
                return Some(order);
            }
            if let Some(order) = self.take_planned(io) {
                return Some(order);
            }
        }
        if self.balance > 0 {
            return self.spare(io);
        }
        None
    }

    /// `FUN_00475490`: the side's free fleets not en route nor held here,
    /// with a sound ship (`0x8005` without `0xa`), nearest the system
    /// first (`FUN_0041a8a0`, order 2).
    fn take_near(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        let home = self.system.and_then(|s| io.galaxy.system(s))?.position;
        let fleets = {
            let random = &mut *io.random;
            let mut coin = || random(10) < 5;
            io.galaxy.fleets_near(
                io.world,
                home,
                Mask::new(0x8005, 0xa),
                Order::Descending,
                &mut coin,
            )
        };
        self.take_from(io, picks(vec![fleets]))
    }

    /// `FUN_00475890`: the side's fleets held for a sector plan (`+0x34`
    /// above 0), with a sound ship and none of en route, free, held here
    /// or a role (`0x8001` without `0xf000000e`), by plan id, order 1
    /// (`FUN_0041ab40`).
    fn take_planned(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        self.system?;
        let items: Vec<(FleetKey, i32)> = io
            .world
            .fleets
            .keys()
            .filter_map(|key| {
                let r = io.galaxy.fleets.get(&key)?;
                (r.plan > 0 && Mask::new(0x8001, 0xf000_000e).admits(r.flags))
                    .then(|| (key, i32::try_from(r.plan).unwrap_or(i32::MAX)))
            })
            .collect();
        let fleets = {
            let random = &mut *io.random;
            let mut coin = || random(10) < 5;
            ranked(items, Order::Ascending, &mut coin)
        };
        self.take_from(io, picks(vec![fleets]))
    }

    /// The walk `FUN_00475490` and `FUN_00475890` share: the fleets in
    /// turn until one gives ships. A free ship fit to sail (`0x20000`
    /// without [`UNFIT`]) ends the walk unless the balance is short; then a
    /// heavy one is taken. The taken ships move into the planner's fleet
    /// (Move `0x201`), or with none leave for its system as a fleet
    /// (Create Fleet `0x270`).
    fn take_from(&mut self, io: &mut PlannerIo<'_>, fleets: Vec<FleetKey>) -> Option<AIAction> {
        let mut stop = false;
        for fleet in fleets {
            let Some(ships) = io.galaxy.fleets.get(&fleet).map(|r| r.ships.clone()) else {
                continue;
            };
            let mut taken = Vec::new();
            for ship in ships {
                if stop {
                    break;
                }
                if ship.flags & SHIP_FREE == 0 || ship.flags & UNFIT != 0 {
                    continue;
                }
                stop = true;
                if self.balance < 0 {
                    stop = false;
                    if ship.flags & HEAVY != 0 {
                        taken.push(ship.index);
                        self.take_ship(io, ship.id);
                    }
                }
            }
            if taken.is_empty() {
                continue;
            }
            return Some(match self.fleet {
                Some(into) => AIAction::JoinShips {
                    fleet,
                    ships: taken,
                    into,
                },
                None => AIAction::CreateFleet {
                    fleet,
                    ships: taken,
                    to: self.system?,
                },
            });
        }
        None
    }

    /// `FUN_00475c70`: the first held ship fit to leave (without `0x2800`,
    /// its fleet not busy, `FUN_00484510`) whose attack the balance covers
    /// is given back and leaves for the system as a fleet of its own
    /// (Create Fleet `0x270`).
    fn spare(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        let to = self.system?;
        for id in self.ships.clone() {
            let Some(ship) = io.ship(id).cloned() else {
                continue;
            };
            if ship.flags & 0x2800 != 0 || busy(io, &ship) || self.balance < ship.values[14] {
                continue;
            }
            self.release_ship(io, id);
            return Some(AIAction::CreateFleet {
                fleet: ship.fleet,
                ships: vec![ship.index],
                to,
            });
        }
        None
    }

    /// The fleet's system when its record is the side's (`+0x30` 1).
    fn own_station(io: &PlannerIo<'_>, fleet: FleetKey) -> Option<SystemKey> {
        let at = io.world.fleets.get(fleet)?.location;
        io.galaxy
            .system(at)
            .is_some_and(|r| r.kind & KIND_OWN != 0)
            .then_some(at)
    }

    /// `FUN_00474ce0`: with regiments aboard (record `+0x80`,
    /// `values[12]`) at a system of the side, those aboard the held ships
    /// not en route (`FUN_0041ac80`: `0x4000` without `0x20800`) move to
    /// that system (`0x201`).
    fn land_regiments(&self, io: &PlannerIo<'_>) -> Option<AIAction> {
        let fleet = self.fleet?;
        let record = io.galaxy.fleets.get(&fleet)?;
        if record.values[12] <= 0 {
            return None;
        }
        let to = Self::own_station(io, fleet)?;
        let carries = record
            .ships
            .iter()
            .any(|s| Mask::new(SHIP_OWN, SHIP_FREE | 0x800).admits(s.flags) && s.values[9] > 0);
        let troops = io.transport.cargo(fleet).to_vec();
        (carries && !troops.is_empty()).then_some(AIAction::MoveRegiments { troops, to })
    }

    /// `FUN_00474ff0`: with an own person aboard (record `0x4000000`) and
    /// not en route, at a system of the side, the people with a record
    /// aboard its ships not en route with people (`0x600000`) move to that
    /// system (`0x201`) when one is the side's own (record `+0x30` bit 2).
    fn land_people(&self, io: &PlannerIo<'_>) -> Option<AIAction> {
        let fleet = self.fleet?;
        let record = io.galaxy.fleets.get(&fleet)?;
        if record.flags & 0x400_0000 == 0 || record.flags & FLEET_EN_ROUTE != 0 {
            return None;
        }
        let to = Self::own_station(io, fleet)?;
        if !record
            .ships
            .iter()
            .any(|s| s.flags & 0x800 == 0 && s.flags & 0x60_0000 != 0)
        {
            return None;
        }
        let mut own = false;
        let mut characters = Vec::new();
        for &c in &io.world.fleets.get(fleet)?.characters {
            if let Some(&flags) = io.people.get(&c) {
                own |= flags & 2 != 0;
                characters.push(c);
            }
        }
        own.then_some(AIAction::MoveCharacters { characters, to })
    }

    /// `FUN_00475330`: with the system not blockaded, a fleet not en route
    /// at another system moves to it (`0x201` of the fleet).
    fn go_home(&self, io: &PlannerIo<'_>) -> Option<AIAction> {
        let home = self.system?;
        if io.galaxy.system(home)?.state & STATE_BLOCKADED != 0 {
            return None;
        }
        let fleet = self.fleet?;
        let record = io.galaxy.fleets.get(&fleet)?;
        let value = io.world.fleets.get(fleet)?;
        if value.location == home || record.flags & FLEET_EN_ROUTE != 0 {
            return None;
        }
        let ships = value
            .capital_ships
            .iter()
            .enumerate()
            .filter(|(_, s)| s.alive)
            .map(|(i, _)| i)
            .collect();
        Some(AIAction::MoveShips {
            fleet,
            ships,
            to: home,
        })
    }
}

/// `FUN_0041a1d0(.., order 1)`: the sectors whose `+0x24` passes `mask`,
/// each keyed on a draw below 20 as it goes into the list.
fn random_sectors(io: &mut PlannerIo<'_>, mask: Mask) -> Vec<SectorKey> {
    let sectors: Vec<SectorKey> = io
        .galaxy
        .sectors
        .iter()
        .filter(|s| s.admitted([mask, ANY, ANY]))
        .map(|s| s.sector)
        .collect();
    let draws = std::cell::RefCell::new(&mut *io.random);
    let items = sectors.into_iter().map(|s| (s, (draws.borrow_mut())(0x14)));
    ranked(items, Order::Ascending, &mut || {
        (draws.borrow_mut())(10) < 5
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_planner::tests::{fixture, rebuild, with, Fx};
    use crate::ai_records::ClassRecords;
    use crate::world::GameWorld;
    use crate::ids::{CapitalShipKey, CharacterKey, TroopKey};
    use crate::world::{CapitalShipClass, Character, Fleet, ShipInstance, TroopUnit};

    /// A capital ship class of the Alliance: heavy (a turbolaser, class
    /// flag 1, `FUN_004739b0`) or light (six lasers), whose attack
    /// `values[0]` is 100 times `recharge` heavy, 100 / 6 times light.
    fn class(fx: &mut Fx, heavy: bool, recharge: u32) -> CapitalShipKey {
        fx.world.capital_ship_classes.insert(CapitalShipClass {
            is_alliance: true,
            hull: 10,
            turbolaser_fore: u32::from(heavy),
            laser_cannon_fore: if heavy { 0 } else { 6 },
            weapon_recharge_rate: recharge,
            ..CapitalShipClass::default()
        })
    }

    /// An Alliance fleet at `fx.systems[at]` of one ship of each class.
    fn fleet_of(fx: &mut Fx, at: usize, classes: &[CapitalShipKey]) -> FleetKey {
        let at = fx.systems[at];
        let fleet = fx.world.fleets.insert(Fleet {
            location: at,
            capital_ships: classes
                .iter()
                .map(|&c| ShipInstance::new(c, 10, true))
                .collect(),
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        fx.world.systems[at].fleets.push(fleet);
        fx.world.number_ships();
        rebuild(fx);
        fleet
    }

    /// The Alliance headquarters at `fx.systems[at]`.
    fn headquarters(fx: &mut Fx, at: usize) -> SystemKey {
        let system = fx.systems[at];
        fx.world.systems[system].is_headquarters = true;
        rebuild(fx);
        system
    }

    fn ship_id(fx: &Fx, fleet: FleetKey, i: usize) -> u64 {
        fx.world.fleets[fleet].capital_ships[i].id
    }

    fn ship(fx: &Fx, fleet: FleetKey, i: usize) -> &ShipRecord {
        &fx.galaxy.fleets[&fleet].ships[i]
    }

    /// A planner at `state` with the headquarters `system`.
    fn planner(state: u32, system: Option<SystemKey>) -> HqPlanner {
        HqPlanner {
            id: 1,
            state,
            system,
            ..HqPlanner::default()
        }
    }

    #[test]
    fn a_planner_claims_the_side_s_headquarters_and_marks_its_record() {
        // FUN_004744b0: with no system, the sectors with +0x24 0x40000000
        // give the own headquarters, not blockaded nor claimed
        // (FUN_0041a460(.., 0x80000000, 0, 1, 2, 0, 0x80000, 1)); its
        // record takes +0x30 0x80000 and the step goes to 2.
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 2);
        let mut p = planner(1, None);
        with(&mut fx, |io| p.step(io, 0));
        assert_eq!((p.state, p.system), (2, Some(hq)));
        assert_eq!(fx.kinds[&hq], KIND_CLAIMED);
        assert_ne!(fx.galaxy.system(hq).unwrap().kind & KIND_CLAIMED, 0);
        // A second planner finds the claimed one taken: state 8.
        let mut q = planner(1, None);
        with(&mut fx, |io| q.step(io, 0));
        assert_eq!((q.state, q.system), (8, None));
    }

    #[test]
    fn a_headquarters_blockaded_is_forgotten_and_unmarked() {
        // FUN_004744b0: a kept system must stay the side's (+0x30 1), its
        // headquarters (+0x28 0x80000000) and not blockaded (+0x28 2);
        // else it loses 0x80000 (0x47457b) and a new one is searched.
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let mut p = planner(1, Some(hq));
        with(&mut fx, |io| p.step(io, 0));
        assert_eq!((p.state, p.system), (2, Some(hq)));
        fx.galaxy.update_state(hq, STATE_BLOCKADED, 0);
        p.state = 1;
        with(&mut fx, |io| p.step(io, 0));
        assert_eq!((p.state, p.system), (8, None));
        assert_eq!(fx.galaxy.system(hq).unwrap().kind & KIND_CLAIMED, 0);
        assert!(!fx.kinds.contains_key(&hq));
    }

    #[test]
    fn a_short_balance_takes_the_nearest_fleet_s_heavy_ships_for_the_headquarters() {
        // FUN_00474c00 -> FUN_00475490: the free fleets nearest the system
        // (FUN_0041a8a0(.., 0x8005, 0xa, 2)) in turn; while short, each
        // free sound ship goes on and a heavy one (bit 1) is taken
        // (FUN_00475e70: 0x20000 off, 0x10000000 on, its +0x7c added);
        // once covered the next such ship ends the walk. With no fleet the
        // ships leave for the system (Create Fleet 0x270, +0x30).
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let (heavy, light) = (class(&mut fx, true, 3), class(&mut fx, false, 3));
        fleet_of(&mut fx, 3, &[heavy]);
        let near = fleet_of(&mut fx, 1, &[light, heavy, heavy, heavy]);
        let mut p = planner(4, Some(hq));
        p.balance = -500;
        let (order, done) = with(&mut fx, |io| p.step(io, 0));
        assert_eq!(
            order,
            Some(AIAction::CreateFleet {
                fleet: near,
                ships: vec![1, 2],
                to: hq,
            })
        );
        assert!(!done);
        assert_eq!(p.state, 5);
        assert_eq!(p.balance, 100);
        assert_eq!(p.ships, vec![ship_id(&fx, near, 1), ship_id(&fx, near, 2)]);
        assert_eq!(
            ship(&fx, near, 1).flags & (SHIP_FREE | SHIP_HELD),
            SHIP_HELD
        );
        assert_eq!(ship(&fx, near, 3).flags & SHIP_FREE, SHIP_FREE);
        assert_eq!(
            fx.ship_holds[&ship_id(&fx, near, 2)].flags & SHIP_HELD,
            SHIP_HELD
        );
        assert!(p.dirty);
    }

    #[test]
    fn with_a_fleet_the_taken_ships_move_into_it() {
        // FUN_00475490 with +0x2c a fleet: Move 0x201 of the ships with the
        // fleet for its target. A held fleet (bit 8) is not searched.
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let heavy = class(&mut fx, true, 3);
        let home = fleet_of(&mut fx, 0, &[heavy]);
        let far = fleet_of(&mut fx, 2, &[heavy]);
        let mut p = planner(4, Some(hq));
        with(&mut fx, |io| p.claim_fleet(io, home));
        p.balance = -1;
        let (order, _) = with(&mut fx, |io| p.step(io, 0));
        assert_eq!(
            order,
            Some(AIAction::JoinShips {
                fleet: far,
                ships: vec![0],
                into: home,
            })
        );
    }

    #[test]
    fn the_sector_plans_fleets_give_ships_when_no_free_fleet_does() {
        // FUN_00475890 after FUN_00475490 found none: the fleets held for a
        // plan (+0x34 above 0; 0x8001 without 0xf000000e,
        // FUN_0041ab40(0, .., 1)), the latest plan first.
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let heavy = class(&mut fx, true, 3);
        let early = fleet_of(&mut fx, 1, &[heavy]);
        let late = fleet_of(&mut fx, 2, &[heavy]);
        for (f, plan) in [(early, 1), (late, 2)] {
            with(&mut fx, |io| {
                io.edit_fleet(f, |r| {
                    r.flags &= !FLEET_FREE;
                    r.owner = 7;
                    r.plan = plan;
                });
            });
        }
        let mut p = planner(4, Some(hq));
        p.balance = -1;
        let (order, _) = with(&mut fx, |io| p.step(io, 0));
        assert_eq!(
            order,
            Some(AIAction::CreateFleet {
                fleet: late,
                ships: vec![0],
                to: hq,
            })
        );
    }

    #[test]
    fn a_surplus_gives_back_the_first_ship_it_covers_as_a_fleet_of_its_own() {
        // FUN_00475c70: the first held ship without 0x2800, its fleet not
        // busy (FUN_00484510), whose +0x7c the balance covers, given back
        // (FUN_00475f60: free again, its +0x7c off the balance) and sent to
        // the system as a fleet (Create Fleet 0x270). One a cycle.
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let (big, small) = (class(&mut fx, true, 5), class(&mut fx, true, 2));
        let home = fleet_of(&mut fx, 0, &[big, small, small]);
        let mut p = planner(4, Some(hq));
        with(&mut fx, |io| {
            p.claim_fleet(io, home);
            for i in 0..3 {
                let id = io.galaxy.fleets[&home].ships[i].id;
                p.take_ship(io, id);
            }
        });
        p.balance = 300;
        let (order, _) = with(&mut fx, |io| p.step(io, 0));
        assert_eq!(
            order,
            Some(AIAction::CreateFleet {
                fleet: home,
                ships: vec![1],
                to: hq,
            })
        );
        assert_eq!(p.balance, 100);
        assert_eq!(p.ships.len(), 2);
        assert_eq!(
            ship(&fx, home, 1).flags & (SHIP_FREE | SHIP_HELD),
            SHIP_FREE
        );
    }

    #[test]
    fn the_balance_is_the_fleet_s_attack_less_the_heaviest_class_doubled_for_headquarters_only() {
        // FUN_00474b90: +0x34 = the fleet record's +0xb4 (values[25]), 0
        // with none, less controller +0x164; controller +4 bit 0 doubles it.
        let mut fx = fixture();
        let heavy = class(&mut fx, true, 3);
        let home = fleet_of(&mut fx, 0, &[heavy, heavy]);
        let mut p = planner(3, None);
        with(&mut fx, |io| p.step(io, 250));
        assert_eq!((p.state, p.balance), (4, -250));
        with(&mut fx, |io| p.claim_fleet(io, home));
        p.state = 3;
        with(&mut fx, |io| p.step(io, 250));
        assert_eq!(p.balance, 350);
        p.state = 3;
        with(&mut fx, |io| {
            io.controller_flags = HEADQUARTERS_ONLY;
            p.step(io, 250)
        });
        assert_eq!(p.balance, 100);
    }

    #[test]
    fn the_heaviest_attack_is_the_side_s_best_researched_heavy_class() {
        // FUN_0042e210 -> FUN_0042e960(side, level, 1, 0x200, 1): the
        // class records of the side with flag 1 and a research order at
        // most the level, the largest values[0] (key 0x200).
        let mut fx = fixture();
        class(&mut fx, true, 3);
        // A light class of more attack (30 x 100 / 6) does not count.
        class(&mut fx, false, 30);
        let later = class(&mut fx, true, 7);
        fx.world.capital_ship_classes[later].research_order = 2;
        let empire = class(&mut fx, true, 8);
        fx.world.capital_ship_classes[empire].is_alliance = false;
        fx.world.capital_ship_classes[empire].is_empire = true;
        let classes = ClassRecords::build(&fx.world);
        let floors = |w: &GameWorld, c: &ClassRecords, r: &crate::research::ResearchState, a| {
            crate::ai_records::ClassFloors::of(w, c, r, a).heavy_attack
        };
        let mut research = crate::research::ResearchState::new();
        assert_eq!(floors(&fx.world, &classes, &research, true), 300);
        assert_eq!(floors(&fx.world, &classes, &research, false), 800);
        research.alliance.ship = 2;
        assert_eq!(floors(&fx.world, &classes, &research, true), 700);
        assert_eq!(floors(&GameWorld::default(), &classes, &research, true), 0);
    }

    #[test]
    fn the_refresh_takes_a_held_ship_s_free_fleet_and_lets_others_go() {
        // FUN_004747b0: with no fleet, a held ship of the side keeps its
        // own fleet's record, which becomes the planner's when free
        // (FUN_00475fe0: bit 4 off, 8 on); then a ship in another fleet is
        // given back (FUN_00475f60), and a fleet with no ships left goes
        // (FUN_00476080: 4 on, 8 off).
        let mut fx = fixture();
        let heavy = class(&mut fx, true, 3);
        let a = fleet_of(&mut fx, 0, &[heavy, heavy]);
        let b = fleet_of(&mut fx, 1, &[heavy]);
        let mut p = planner(1, None);
        let ids = [ship_id(&fx, a, 0), ship_id(&fx, b, 0)];
        with(&mut fx, |io| {
            for id in ids {
                p.take_ship(io, id);
            }
        });
        assert_eq!(p.balance, 600);
        with(&mut fx, |io| p.refresh(io));
        assert_eq!(p.fleet, Some(a));
        assert_eq!(p.ships, vec![ids[0]]);
        assert_eq!(p.balance, 300);
        assert_eq!(
            fx.galaxy.fleets[&a].flags & (FLEET_FREE | FLEET_HELD),
            FLEET_HELD
        );
        assert_eq!(fx.fleet_holds[&a].flags & FLEET_HELD, FLEET_HELD);
        assert_eq!(ship(&fx, b, 0).flags & (SHIP_FREE | SHIP_HELD), SHIP_FREE);
        // A fleet no longer the side's (+0x38 bit 1) is given back (with no
        // held ship left to take it again).
        fx.galaxy.fleets.get_mut(&a).unwrap().flags &= !FLEET_OWN;
        let held = std::mem::take(&mut p.ships);
        with(&mut fx, |io| p.refresh(io));
        assert_eq!(p.fleet, None);
        p.ships = held;
        with(&mut fx, |io| p.claim_fleet(io, a));
        fx.galaxy.fleets.get_mut(&a).unwrap().flags |= FLEET_OWN;
        with(&mut fx, |io| p.refresh(io));
        assert_eq!(p.fleet, Some(a));
        // The fleet's ships gone: it is given back.
        fx.galaxy.fleets.get_mut(&a).unwrap().ships.clear();
        with(&mut fx, |io| p.refresh(io));
        assert_eq!(p.fleet, None);
        assert_eq!(
            fx.galaxy.fleets[&a].flags & (FLEET_FREE | FLEET_HELD),
            FLEET_FREE
        );
        assert!(p.ships.is_empty());
    }

    #[test]
    fn the_refresh_checks_the_marks_on_the_fleet_and_its_ships() {
        // FUN_004747b0: the fleet must keep bit 8 (FUN_00475fe0); a ship
        // must be the side's (0x4000); with no fleet, a held ship's fleet
        // that is not free (bit 4) stays the ship's but is not taken.
        let mut fx = fixture();
        let heavy = class(&mut fx, true, 3);
        let a = fleet_of(&mut fx, 0, &[heavy, heavy]);
        let ids = [ship_id(&fx, a, 0), ship_id(&fx, a, 1)];
        let mut p = planner(1, None);
        with(&mut fx, |io| p.claim_fleet(io, a));
        fx.galaxy.fleets.get_mut(&a).unwrap().flags &= !FLEET_HELD;
        with(&mut fx, |io| p.refresh(io));
        assert_eq!(p.fleet, None);
        // The fleet is free again; a sector planner takes it.
        with(&mut fx, |io| io.edit_fleet(a, |r| r.flags &= !FLEET_FREE));
        with(&mut fx, |io| {
            for id in ids {
                p.take_ship(io, id);
            }
        });
        fx.galaxy.fleets.get_mut(&a).unwrap().ships[1].flags &= !SHIP_OWN;
        with(&mut fx, |io| p.refresh(io));
        assert_eq!(p.fleet, None);
        assert_eq!(p.ships, vec![ids[0]]);
        assert_eq!(fx.galaxy.fleets[&a].flags & FLEET_HELD, 0);
    }

    #[test]
    fn a_ship_a_sector_planner_names_is_dropped_without_freeing_it() {
        // FUN_00475f60 frees the record only while +0x30 is 0.
        let mut fx = fixture();
        let heavy = class(&mut fx, true, 3);
        let a = fleet_of(&mut fx, 0, &[heavy]);
        let id = ship_id(&fx, a, 0);
        let mut p = planner(1, None);
        with(&mut fx, |io| {
            p.take_ship(io, id);
            io.edit_ship(id, |r| r.owner = 4);
            p.release_ship(io, id);
        });
        assert!(p.ships.is_empty());
        assert_eq!(p.balance, 0);
        assert_eq!(ship(&fx, a, 0).flags & SHIP_FREE, 0);
    }

    #[test]
    fn damaged_ships_of_the_fleet_leave_as_their_own_fleet_in_its_system() {
        // FUN_004749b0: a fleet record with 0x3000 and none of 0xf0000802:
        // its ships with 0x801000 and without 0x800, Create Fleet 0x270
        // whose target is the fleet's system (record +0x28, 0x474a2f).
        let mut fx = fixture();
        let heavy = class(&mut fx, true, 3);
        let home = fleet_of(&mut fx, 1, &[heavy, heavy, heavy]);
        let mut p = planner(2, None);
        with(&mut fx, |io| p.claim_fleet(io, home));
        let r = fx.galaxy.fleets.get_mut(&home).unwrap();
        r.flags |= 0x1000;
        r.ships[0].flags |= 0x1000;
        r.ships[2].flags |= 0x80_0000;
        let (order, done) = with(&mut fx, |io| p.step(io, 0));
        assert_eq!(
            order,
            Some(AIAction::CreateFleet {
                fleet: home,
                ships: vec![0, 2],
                to: fx.systems[1],
            })
        );
        assert!(!done);
        assert_eq!(p.state, 3);
        // En route the fleet keeps them.
        fx.galaxy.fleets.get_mut(&home).unwrap().flags |= 0x800;
        p.state = 2;
        assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
    }

    /// A regiment of the Alliance aboard `fleet`, whose ships carry two.
    fn embark(fx: &mut Fx, f: FleetKey) -> TroopKey {
        let class = fx.world.fleets[f].capital_ships[0].class;
        fx.world.capital_ship_classes[class].troop_capacity = 2;
        let at = fx.world.fleets[f].location;
        let troop = fx.world.troops.insert(TroopUnit {
            class_dat_id: crate::ids::DatId::new(0x1000_0001),
            is_alliance: true,
            regiment_strength: 100,
        });
        fx.world.systems[at].ground_units.push(troop);
        fx.transport.embark(&mut fx.world, f, &[troop]).unwrap();
        rebuild(fx);
        troop
    }

    #[test]
    fn regiments_aboard_land_at_a_system_of_the_side() {
        // FUN_00474ce0: a fleet record with +0x80 above 0 at a system of
        // the side (+0x30 1): the regiments aboard its held ships not en
        // route (FUN_0041ac80(fleet, 0x4000, 0x20800, 9, 1)) move to the
        // fleet's system (0x201, 0x474f5e).
        let mut fx = fixture();
        let heavy = class(&mut fx, true, 3);
        let home = fleet_of(&mut fx, 1, &[heavy]);
        let troop = embark(&mut fx, home);
        let mut p = planner(5, None);
        with(&mut fx, |io| p.claim_fleet(io, home));
        // A free (unheld) carrier lands nothing.
        assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
        let id = ship_id(&fx, home, 0);
        with(&mut fx, |io| p.take_ship(io, id));
        p.state = 5;
        assert_eq!(
            with(&mut fx, |io| p.step(io, 0)).0,
            Some(AIAction::MoveRegiments {
                troops: vec![troop],
                to: fx.systems[1],
            })
        );
        // At a system not the side's, none.
        let at = fx.systems[1];
        fx.galaxy.update_kind(at, 4, 1);
        p.state = 5;
        assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
    }

    /// A character aboard `f` whose record has the flags `flags`.
    fn aboard(fx: &mut Fx, f: FleetKey, flags: u32) -> CharacterKey {
        let c = fx.world.characters.insert(Character::default());
        fx.world.fleets[f].characters.push(c);
        fx.people.insert(c, flags);
        rebuild(fx);
        c
    }

    #[test]
    fn people_aboard_land_when_one_is_the_side_s_own() {
        // FUN_00474ff0: a fleet record with 0x4000000, not en route, at a
        // system of the side: everyone with a record aboard its ships not
        // en route with 0x600000 moves to that system (0x201, 0x4752d2),
        // when one record has +0x30 bit 2 (0x475212).
        let mut fx = fixture();
        let heavy = class(&mut fx, true, 3);
        let home = fleet_of(&mut fx, 1, &[heavy]);
        let prisoner = aboard(&mut fx, home, 4 | 0x40);
        let mut p = planner(6, None);
        with(&mut fx, |io| p.claim_fleet(io, home));
        assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
        let own = aboard(&mut fx, home, 2);
        with(&mut fx, |io| p.claim_fleet(io, home));
        p.state = 6;
        let (order, done) = with(&mut fx, |io| p.step(io, 0));
        assert_eq!(
            order,
            Some(AIAction::MoveCharacters {
                characters: vec![prisoner, own],
                to: fx.systems[1],
            })
        );
        assert!(!done);
        assert_eq!(p.state, 7);
    }

    #[test]
    fn the_fleet_goes_home_unless_the_headquarters_is_blockaded() {
        // FUN_00475330: the system not blockaded (+0x28 2), the fleet's
        // record not en route at another system: Move 0x201 of the fleet
        // to it; the cycle ends (done, state 0).
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let heavy = class(&mut fx, true, 3);
        let away = fleet_of(&mut fx, 2, &[heavy, heavy]);
        let mut p = planner(7, Some(hq));
        with(&mut fx, |io| p.claim_fleet(io, away));
        let (order, done) = with(&mut fx, |io| p.step(io, 0));
        assert_eq!(
            order,
            Some(AIAction::MoveShips {
                fleet: away,
                ships: vec![0, 1],
                to: hq,
            })
        );
        assert!(done);
        assert_eq!(p.state, 0);
        fx.galaxy.update_state(hq, STATE_BLOCKADED, 0);
        p.state = 7;
        assert_eq!(with(&mut fx, |io| p.step(io, 0)), (None, true));
    }

    #[test]
    fn a_cycle_runs_state_by_state_and_state_four_always_moves_on() {
        // FUN_00474390: other states set 1 and +0x38; 1 to 7 in turn, 8
        // when no system is kept. State 4 moves on even with its fleet en
        // route (the flag slot holds the caller's pointer, 0x47440f).
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let heavy = class(&mut fx, true, 3);
        let home = fleet_of(&mut fx, 0, &[heavy]);
        let mut p = planner(0, None);
        let id = ship_id(&fx, home, 0);
        with(&mut fx, |io| {
            p.claim_fleet(io, home);
            p.take_ship(io, id);
        });
        fx.galaxy.fleets.get_mut(&home).unwrap().flags |= FLEET_EN_ROUTE;
        let mut states = Vec::new();
        for _ in 0..8 {
            let (order, done) = with(&mut fx, |io| p.step(io, 1000));
            assert_eq!(order, None);
            states.push((p.state, done));
        }
        assert_eq!(
            states,
            [
                (1, false),
                (2, false),
                (3, false),
                (4, false),
                (5, false),
                (6, false),
                (7, false),
                (0, true),
            ]
        );
        assert_eq!(p.system, Some(hq));
        assert_eq!(p.balance, -700);
        // No headquarters: 8 gives everything back.
        let mut fx = fixture();
        let heavy = class(&mut fx, true, 3);
        let a = fleet_of(&mut fx, 0, &[heavy]);
        let mut p = planner(1, None);
        let id = ship_id(&fx, a, 0);
        with(&mut fx, |io| p.take_ship(io, id));
        with(&mut fx, |io| p.step(io, 0));
        assert_eq!(p.state, 8);
        assert_eq!(with(&mut fx, |io| p.step(io, 0)), (None, true));
        assert!(p.ships.is_empty() && p.fleet.is_none());
        assert_eq!(ship(&fx, a, 0).flags & SHIP_FREE, SHIP_FREE);
    }

    #[test]
    fn the_strategy_keeps_a_planner_per_headquarters_and_drops_a_head_without_one() {
        // FUN_004e69d0: fewer planners than controller +0x224 adds one
        // (FUN_0042ed50); more drops the head only when it has no system
        // (FUN_0042edc0, FUN_004742b0).
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        assert_eq!(headquarters_count(&fx.galaxy), 1);
        let mut m = Module9::default();
        with(&mut fx, |io| m.balance(io, 1));
        with(&mut fx, |io| m.balance(io, 1));
        assert_eq!(m.planners.len(), 1);
        assert_eq!((m.planners[0].id, m.next_id), (1, 2));
        m.planners[0].system = Some(hq);
        with(&mut fx, |io| m.balance(io, 0));
        assert_eq!(m.planners.len(), 1);
        m.planners[0].system = None;
        with(&mut fx, |io| m.balance(io, 0));
        assert!(m.planners.is_empty());
        // With no headquarters the count is 0 (FUN_00432350, +0x1a0).
        fx.world.systems[hq].is_headquarters = false;
        rebuild(&mut fx);
        assert_eq!(headquarters_count(&fx.galaxy), 0);
    }

    #[test]
    fn the_module_steps_one_planner_a_cycle_and_raises_after_an_order() {
        // FUN_004cf2c0: state 2 takes the planner +0x44 names, else the
        // head (FUN_004cf350); state 3 steps it, an order completing the
        // module and, the planner not done, raising +0x34; a done planner
        // ends the cycle. No planner: complete at once.
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let heavy = class(&mut fx, true, 3);
        let away = fleet_of(&mut fx, 2, &[heavy]);
        let mut m = Module9::default();
        assert_eq!(with(&mut fx, |io| m.step(io, 0)), Step::pending());
        assert_eq!(with(&mut fx, |io| m.step(io, 0)), Step::complete());
        assert_eq!(m.state, 0);
        m.planners = vec![
            HqPlanner {
                id: 1,
                state: 7,
                system: Some(hq),
                ..HqPlanner::default()
            },
            HqPlanner {
                id: 2,
                ..HqPlanner::default()
            },
        ];
        with(&mut fx, |io| m.planners[0].claim_fleet(io, away));
        m.state = 2;
        assert_eq!(with(&mut fx, |io| m.step(io, 0)), Step::pending());
        assert_eq!((m.current, m.next), (1, 2));
        let s = with(&mut fx, |io| m.step(io, 0));
        assert!(matches!(s.order, Some(AIAction::MoveShips { .. })));
        assert!(s.complete && !s.raise);
        assert_eq!(m.state, 0);
        // The next cycle takes planner 2; its first state is pending.
        m.state = 2;
        with(&mut fx, |io| m.step(io, 0));
        assert_eq!((m.current, m.next), (2, 0));
        assert_eq!(with(&mut fx, |io| m.step(io, 0)), Step::pending());
        assert_eq!(m.state, 3);
        // An order before the cycle's end raises the priority.
        with(&mut fx, |io| m.planners[1].claim_fleet(io, away));
        m.planners[1].state = 2;
        let r = fx.galaxy.fleets.get_mut(&away).unwrap();
        r.flags |= 0x1000;
        r.ships[0].flags |= 0x1000;
        let s = with(&mut fx, |io| m.step(io, 0));
        assert!(matches!(s.order, Some(AIAction::CreateFleet { .. })));
        assert!(s.complete && s.raise);
        assert_eq!(m.state, 3);
        m.refused();
        assert_eq!(m.state, 0);
        // A planner done without an order (state 8) completes the module.
        m.planners[1].state = 8;
        m.state = 3;
        assert_eq!(with(&mut fx, |io| m.step(io, 0)), Step::complete());
        assert_eq!(m.state, 0);
    }

    #[test]
    fn a_held_ship_whose_fleet_has_a_role_or_stands_at_the_opponent_s_is_not_spared() {
        // FUN_00475c70 skips a ship FUN_00484510 calls busy: its fleet's
        // record has a role (+0x38 0xf0000000) or its system is the
        // opponent's (+0x30 4); and one with 0x2800.
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let heavy = class(&mut fx, true, 2);
        let home = fleet_of(&mut fx, 0, &[heavy]);
        let spare = |fx: &mut Fx| {
            let mut p = planner(4, Some(hq));
            with(fx, |io| {
                p.claim_fleet(io, home);
                let id = io.galaxy.fleets[&home].ships[0].id;
                p.take_ship(io, id);
            });
            p.balance = 200;
            with(fx, |io| p.step(io, 0)).0
        };
        fx.galaxy.fleets.get_mut(&home).unwrap().flags |= 0x1000_0000;
        assert_eq!(spare(&mut fx), None);
        fx.galaxy.fleets.get_mut(&home).unwrap().flags &= !0x1000_0000;
        fx.galaxy.update_kind(hq, 4, 0);
        assert_eq!(spare(&mut fx), None);
        fx.galaxy.update_kind(hq, 0, 4);
        fx.galaxy.fleets.get_mut(&home).unwrap().ships[0].flags |= 0x2000;
        assert_eq!(spare(&mut fx), None);
        fx.galaxy.fleets.get_mut(&home).unwrap().ships[0].flags &= !0x2000;
        assert!(spare(&mut fx).is_some());
    }

    #[test]
    fn an_en_route_fleet_gathers_nothing_and_a_fleet_at_home_stays() {
        // FUN_00474c00 does nothing while the fleet's record is en route
        // (+0x38 2); FUN_00475330 moves no fleet already at the system or
        // en route.
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let heavy = class(&mut fx, true, 3);
        let home = fleet_of(&mut fx, 0, &[heavy]);
        fleet_of(&mut fx, 1, &[heavy]);
        let mut p = planner(4, Some(hq));
        with(&mut fx, |io| p.claim_fleet(io, home));
        fx.galaxy.fleets.get_mut(&home).unwrap().flags |= FLEET_EN_ROUTE;
        p.balance = -1;
        assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
        p.state = 7;
        assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
        fx.galaxy.fleets.get_mut(&home).unwrap().flags &= !FLEET_EN_ROUTE;
        p.state = 7;
        assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
        p.state = 4;
        p.balance = -1;
        assert!(with(&mut fx, |io| p.step(io, 0)).0.is_some());
    }

    #[test]
    fn a_kept_headquarters_must_stay_the_side_s_and_draws_nothing() {
        // FUN_004744b0: a kept system's record needs +0x30 1 and +0x28
        // 0x80000000; kept, no sector search runs (no FUN_0041a1d0 draws).
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let mut p = planner(1, Some(hq));
        fx.draws = vec![7; 4];
        with(&mut fx, |io| p.step(io, 0));
        assert_eq!((p.state, p.system), (2, Some(hq)));
        assert_eq!(fx.draws.len(), 4);
        for (set_state, clear_state, clear_kind) in [(0, STATE_HEADQUARTERS, 0), (0, 0, KIND_OWN)] {
            let mut fx = fixture();
            let hq = headquarters(&mut fx, 0);
            fx.galaxy.update_state(hq, set_state, clear_state);
            fx.galaxy.update_kind(hq, 0, clear_kind);
            let mut p = planner(1, Some(hq));
            with(&mut fx, |io| p.step(io, 0));
            assert_eq!((p.state, p.system), (8, None));
        }
    }

    #[test]
    fn only_a_fleet_record_with_damaged_ships_splits_them() {
        // FUN_004749b0 needs the fleet record's 0x3000.
        let mut fx = fixture();
        let heavy = class(&mut fx, true, 3);
        let home = fleet_of(&mut fx, 1, &[heavy, heavy]);
        let mut p = planner(2, None);
        with(&mut fx, |io| p.claim_fleet(io, home));
        fx.galaxy.fleets.get_mut(&home).unwrap().ships[0].flags |= 0x1000;
        assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
    }

    #[test]
    fn an_even_balance_neither_takes_nor_gives() {
        // FUN_00474c00: below 0 takes, above 0 gives; at 0 neither, even
        // with free heavy ships near and a held ship of no attack.
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let (heavy, unarmed) = (class(&mut fx, true, 3), class(&mut fx, true, 0));
        fleet_of(&mut fx, 1, &[heavy]);
        let home = fleet_of(&mut fx, 0, &[unarmed]);
        let mut p = planner(4, Some(hq));
        with(&mut fx, |io| {
            p.claim_fleet(io, home);
            let id = io.galaxy.fleets[&home].ships[0].id;
            p.take_ship(io, id);
        });
        p.balance = 0;
        assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
    }

    #[test]
    fn only_free_ships_are_taken_and_only_planned_fleets_fit_to_sail() {
        // FUN_00475490 takes a free ship (0x20000) only; FUN_00475890 only
        // fleets of a plan (+0x34 above 0) passing 0x8001 without
        // 0xf000000e.
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let heavy = class(&mut fx, true, 3);
        let near = fleet_of(&mut fx, 1, &[heavy]);
        let id = ship_id(&fx, near, 0);
        with(&mut fx, |io| io.edit_ship(id, |r| r.flags &= !SHIP_FREE));
        let mut p = planner(4, Some(hq));
        p.balance = -1;
        assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
        // Held for no plan, or for a plan but en route: none.
        let mut fx = fixture();
        let hq = headquarters(&mut fx, 0);
        let heavy = class(&mut fx, true, 3);
        let f = fleet_of(&mut fx, 1, &[heavy]);
        for (plan, flags) in [(0, 0), (2, FLEET_EN_ROUTE)] {
            with(&mut fx, |io| {
                io.edit_fleet(f, |r| {
                    r.flags = (r.flags & !FLEET_FREE) | flags;
                    r.owner = 7;
                    r.plan = plan;
                });
            });
            let mut p = planner(4, Some(hq));
            p.balance = -1;
            assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
        }
    }

    #[test]
    fn people_land_only_from_a_standing_fleet_with_its_own_aboard() {
        // FUN_00474ff0: the record's 0x4000000 and no en route (2); the
        // ship's 0x600000; a person of the side's (+0x30 2) among them.
        let mut fx = fixture();
        let heavy = class(&mut fx, true, 3);
        let home = fleet_of(&mut fx, 1, &[heavy]);
        aboard(&mut fx, home, 2);
        let mut p = planner(6, None);
        let run = |fx: &mut Fx, p: &mut HqPlanner| {
            with(fx, |io| p.claim_fleet(io, home));
            p.state = 6;
            with(fx, |io| p.step(io, 0)).0
        };
        assert!(run(&mut fx, &mut p).is_some());
        let r = fx.galaxy.fleets.get_mut(&home).unwrap();
        r.flags |= FLEET_EN_ROUTE;
        assert_eq!(run(&mut fx, &mut p), None);
        let r = fx.galaxy.fleets.get_mut(&home).unwrap();
        r.flags &= !(FLEET_EN_ROUTE | 0x400_0000);
        assert_eq!(run(&mut fx, &mut p), None);
        let r = fx.galaxy.fleets.get_mut(&home).unwrap();
        r.flags |= 0x400_0000;
        r.ships[0].flags &= !0x60_0000;
        assert_eq!(run(&mut fx, &mut p), None);
        // Only a prisoner aboard, the record's bits forced: none.
        let mut fx = fixture();
        let heavy = class(&mut fx, true, 3);
        let home = fleet_of(&mut fx, 1, &[heavy]);
        aboard(&mut fx, home, 4 | 0x40);
        assert_ne!(ship(&fx, home, 0).flags & 0x20_0000, 0);
        fx.galaxy.fleets.get_mut(&home).unwrap().flags |= 0x400_0000;
        let mut p = planner(6, None);
        with(&mut fx, |io| p.claim_fleet(io, home));
        assert_eq!(with(&mut fx, |io| p.step(io, 0)).0, None);
    }
}
