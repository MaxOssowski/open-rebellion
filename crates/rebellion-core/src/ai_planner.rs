//! A sector planner's own work (`FUN_004762b0`, vtable `0x65a3b0`): the
//! capital ships and fleets it holds (`FUN_00478e00`, `FUN_004790d0`,
//! `FUN_00478fd0`, `FUN_004791d0`), and its step (`FUN_004774c0`), which
//! weighs the forces it holds against what its plan wants, takes or gives
//! ships with Create Fleet `0x270`, runs the plan object, and reports
//! (`ghidra/notes/ai-sector-plans.md`, "The planner's step").
//!
//! port: of the plan objects only kind 3's slots are ported here
//! ([`crate::ai_plan`]); the other kinds' planners run the port's fleet
//! heuristics instead of their cycle.

use std::collections::HashMap;

use crate::ai::AIAction;
use crate::ai_module1::ANY;
use crate::ai_module8::{SectorPlanner, CYCLED, REVIEWED};
use crate::ai_records::{
    pick, picks, ranked, sector_at, FleetHold, Galaxy, Mask, Order, ShipHold, ShipRecord,
    FLEET_FREE, FLEET_KEPT, FLEET_OWN, SHIP_FREE, SHIP_KEPT, SHIP_OWN,
};
use crate::ids::{FleetKey, SectorKey, SystemKey};
use crate::world::GameWorld;

/// Ship record `+0x24` bit `0x1000000`: the planner's flagship
/// (`FUN_00479560`).
pub const SHIP_FLAGSHIP: u32 = 0x100_0000;

/// Ship record `+0x24` bit `0x2000000`: a planner passed the ship over
/// (`FUN_00478690`, `FUN_00478aa0`).
pub const SHIP_PASSED: u32 = 0x200_0000;

/// What a planner's step reads and writes.
pub struct PlannerIo<'a> {
    pub world: &'a GameWorld,
    pub galaxy: &'a mut Galaxy,
    /// [`crate::ai::AIState::ship_holds`].
    pub ship_holds: &'a mut HashMap<u64, ShipHold>,
    /// [`crate::ai::AIState::fleet_holds`].
    pub fleet_holds: &'a mut HashMap<FleetKey, FleetHold>,
    /// [`crate::ai::AIState::record_kinds`]: the system records' kept
    /// `+0x30` bits.
    pub kinds: &'a mut HashMap<SystemKey, u32>,
    /// The regiments aboard the fleets.
    pub transport: &'a crate::troop_transport::TroopTransportState,
    /// The people's record flags (`+0x30`), by character.
    pub people: &'a HashMap<crate::ids::CharacterKey, u32>,
    /// Controller `+4`.
    pub controller_flags: u32,
    /// Today (`FUN_004fcf00`).
    pub day: u64,
    /// [`crate::ai::AIState::seen`]: the system records' `+0x44`.
    pub seen: &'a HashMap<SystemKey, u64>,
    /// [`crate::ai::AIState::swept`].
    pub swept: &'a mut Vec<SectorKey>,
    /// `FUN_0041cd80(n)`: a draw below `n`.
    pub random: &'a mut dyn FnMut(i32) -> i32,
}

impl PlannerIo<'_> {
    /// Set and clear bits of `system`'s record `+0x30`, kept across
    /// refreshes (`FUN_00433e30` marks the record dirty).
    pub(crate) fn set_kind(&mut self, system: SystemKey, set: u32, clear: u32) {
        self.galaxy.update_kind(system, set, clear);
        let value = self.kinds.entry(system).or_default();
        *value = (*value & !clear) | set;
        if *value == 0 {
            self.kinds.remove(&system);
        }
    }

    fn ship(&self, id: u64) -> Option<&ShipRecord> {
        self.galaxy
            .fleets
            .values()
            .flat_map(|f| f.ships.iter())
            .find(|s| s.id == id)
    }

    /// Change a ship record and keep the change across refreshes.
    fn edit_ship(&mut self, id: u64, edit: impl FnOnce(&mut ShipRecord)) {
        let Some(record) = self
            .galaxy
            .fleets
            .values_mut()
            .flat_map(|f| f.ships.iter_mut())
            .find(|s| s.id == id)
        else {
            return;
        };
        edit(record);
        self.ship_holds.insert(
            id,
            ShipHold {
                flags: record.flags & SHIP_KEPT,
                owner: record.owner,
                plan: record.plan,
            },
        );
    }

    /// Change a fleet record and keep the change across refreshes.
    pub(crate) fn edit_fleet(
        &mut self,
        fleet: FleetKey,
        edit: impl FnOnce(&mut crate::ai_records::FleetRecord),
    ) {
        let Some(record) = self.galaxy.fleets.get_mut(&fleet) else {
            return;
        };
        edit(record);
        self.fleet_holds.insert(
            fleet,
            FleetHold {
                flags: record.flags & FLEET_KEPT,
                owner: record.owner,
                plan: record.plan,
                target: record.target,
            },
        );
    }
}

/// What a planner's step gives its module.
#[derive(Debug, Clone, PartialEq)]
pub enum PlannerStep {
    /// No order; the cycle goes on.
    Pending,
    /// An order, and whether the cycle ended with it.
    Order(AIAction, bool),
    /// The cycle ended without an order.
    Done,
    /// A planner of a kind not ported: the port's heuristics stand in for
    /// its cycle.
    StandIn,
}

/// The ship record values a held ship adds to the counts
/// (`FUN_00478e00`): count, value `+0x44 + 4i`, and the `+0x64` bit a
/// positive value sets.
const SHIP_ADDS: [(usize, usize, u32); 5] = [
    (0, 14, 0x20_0000),
    (1, 15, 0x40_0000),
    (2, 10, 0x80_0000),
    (3, 7, 0x100_0000),
    (5, 1, 0x200_0000),
];

/// The fleet record values the counts sum (`FUN_00477cb0`): count, value
/// `+0x50 + 4i`, and the `+0x64` bit a positive count sets.
const FLEET_SUMS: [(usize, usize, u32); 8] = [
    (0, 25, 0x20_0000),
    (1, 26, 0x40_0000),
    (2, 13, 0x80_0000),
    (3, 10, 0x100_0000),
    (5, 6, 0x200_0000),
    (7, 11, 0x400_0000),
    (8, 8, 0x800_0000),
    (9, 19, 0x1000_0000),
];

/// The counts `FUN_00477e80` checks for a need or a spare, in order.
const BALANCED: [usize; 8] = [0, 1, 2, 3, 5, 4, 6, 9];

impl SectorPlanner {
    /// `FUN_00478e00`: hold a ship. Its record loses `0x20000` and names
    /// this planner and its plan; its values add to the counts.
    pub(crate) fn take_ship(&mut self, io: &mut PlannerIo<'_>, id: u64) {
        let Some(record) = io.ship(id).cloned() else {
            return;
        };
        let (owner, plan) = (self.id, self.plan_id);
        io.edit_ship(id, |r| {
            r.flags &= !SHIP_FREE;
            r.owner = owner;
            r.plan = plan;
        });
        let v = &record.values;
        for (count, at, bit) in SHIP_ADDS {
            if v[at] > 0 {
                self.counts[count] += v[at];
                self.requests |= bit;
            }
        }
        self.counts[7] += v[8];
        self.counts[8] += v[5];
        for (bit, count) in [(0x80, 4), (0x400, 6), (2, 9)] {
            if record.flags & bit != 0 {
                self.counts[count] += 1;
            }
        }
        self.ships.push(id);
    }

    /// `FUN_004790d0`: give a held ship back. Its record, when this
    /// planner holds it, is free again and passes `0x3000000`; its values
    /// leave the counts.
    fn release_ship(&mut self, io: &mut PlannerIo<'_>, id: u64) {
        let Some(at) = self.ships.iter().position(|&s| s == id) else {
            return;
        };
        if let Some(record) = io.ship(id).cloned() {
            if record.owner == self.id {
                io.edit_ship(id, |r| {
                    r.flags = (r.flags & !0x300_0000) | SHIP_FREE;
                    r.owner = 0;
                    r.plan = 0;
                });
            }
            let v = &record.values;
            for (count, at, _) in SHIP_ADDS {
                self.counts[count] -= v[at];
            }
            self.counts[7] -= v[8];
            self.counts[8] -= v[5];
            for (bit, count) in [(0x80, 4), (0x400, 6), (2, 9)] {
                if record.flags & bit != 0 {
                    self.counts[count] -= 1;
                }
            }
        }
        self.ships.remove(at);
    }

    /// `FUN_00478fd0`: hold a fleet: its record loses bit 4 and names this
    /// planner and its plan.
    pub(crate) fn take_fleet(&mut self, io: &mut PlannerIo<'_>, fleet: FleetKey) {
        if !io.galaxy.fleets.contains_key(&fleet) {
            return;
        }
        let (owner, plan) = (self.id, self.plan_id);
        io.edit_fleet(fleet, |r| {
            r.flags &= !FLEET_FREE;
            r.owner = owner;
            r.plan = plan;
        });
        self.fleets.push(fleet);
    }

    /// `FUN_004791d0`: give a held fleet back: its record is free again.
    fn release_fleet(&mut self, io: &mut PlannerIo<'_>, fleet: FleetKey) {
        let Some(at) = self.fleets.iter().position(|&f| f == fleet) else {
            return;
        };
        io.edit_fleet(fleet, |r| {
            r.flags |= FLEET_FREE;
            r.owner = 0;
            r.plan = 0;
        });
        self.fleets.remove(at);
    }

    /// `FUN_00477190`'s part on the lists: give back every ship and fleet.
    pub fn release_all(&mut self, io: &mut PlannerIo<'_>) {
        for id in self.ships.clone() {
            self.release_ship(io, id);
        }
        for fleet in self.fleets.clone() {
            self.release_fleet(io, fleet);
        }
        self.counts = [0; 10];
    }

    /// `FUN_00477810`: a ship this planner still holds and the side owns
    /// (`0x4000`) keeps its fleet held (a free one is taken); any other is
    /// given back.
    fn sync_ships(&mut self, io: &mut PlannerIo<'_>) {
        for id in self.ships.clone() {
            let record = io.ship(id).cloned();
            match record {
                Some(r) if r.owner == self.id && r.flags & SHIP_OWN != 0 => {
                    let free = io
                        .galaxy
                        .fleets
                        .get(&r.fleet)
                        .is_some_and(|f| f.flags & FLEET_FREE != 0);
                    if !self.fleets.contains(&r.fleet) && free {
                        self.take_fleet(io, r.fleet);
                    }
                }
                _ => self.release_ship(io, id),
            }
        }
    }

    /// `FUN_00477950`: a fleet this planner holds, the side's, with ships,
    /// stays; any other is given back.
    fn sync_fleets(&mut self, io: &mut PlannerIo<'_>) {
        for fleet in self.fleets.clone() {
            let keep = io.galaxy.fleets.get(&fleet).is_some_and(|r| {
                r.owner == self.id && r.flags & FLEET_OWN != 0 && !r.ships.is_empty()
            });
            if !keep {
                self.release_fleet(io, fleet);
            }
        }
    }

    /// One step (`FUN_004774c0`): one state of its cycle.
    pub fn step(&mut self, io: &mut PlannerIo<'_>) -> PlannerStep {
        if self.resync {
            self.sync_ships(io);
            self.sync_fleets(io);
            self.resync = false;
        }
        if self.kind == 0 {
            return PlannerStep::Done;
        }
        // port: only kinds 1 to 4's slots are ported; the other kinds'
        // planners run the port's heuristics for their whole cycle.
        if !ported_kind(self.kind) {
            return PlannerStep::StandIn;
        }
        let mut done = false;
        let order = match self.state {
            1 => {
                self.flags = 0;
                self.requests = 0;
                self.state = 2;
                self.split_damaged(io)
            }
            2 => {
                self.sum_fleets(io);
                self.want(io);
                for (count, wanted) in self.counts.iter_mut().zip(self.wanted) {
                    *count -= wanted;
                }
                self.state = 3;
                None
            }
            3 => {
                self.state = 4;
                self.balance(io)
            }
            4 => {
                self.missions(io);
                self.movement(io);
                self.state = 5;
                None
            }
            5 => {
                let (order, ended) = self.plan_step(io);
                if ended {
                    self.state = 6;
                }
                order
            }
            6 => {
                self.flagship(io);
                // FUN_004799d0 without movement planners (0x2000000).
                self.station = None;
                self.report(io);
                self.state = 1;
                self.resync = true;
                done = true;
                None
            }
            _ => {
                self.state = 1;
                self.resync = true;
                None
            }
        };
        match order {
            Some(order) => {
                self.resync = true;
                PlannerStep::Order(order, done)
            }
            None if done => PlannerStep::Done,
            None => PlannerStep::Pending,
        }
    }

    /// The stand-in for another kind's planner finished: its cycle ends
    /// (port: only kinds 1 to 4's slots are ported).
    pub fn plan_done(&mut self) {
        self.state = 1;
        self.resync = true;
    }

    /// `FUN_00477a40`: the first held fleet with a damaged ship (`0x3000`,
    /// without `0xf0000802`) gives its damaged ships back (`0x801000`,
    /// not en route) as a fleet of their own (Create Fleet `0x270`).
    ///
    /// hyp: the order's target (slot `+0x2c`) is read from the list node,
    /// not an id; the port leaves the new fleet in its system.
    fn split_damaged(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        for fleet in self.fleets.clone() {
            let Some(record) = io.galaxy.fleets.get(&fleet).cloned() else {
                continue;
            };
            if record.flags & 0x3000 == 0 || record.flags & 0xf000_0802 != 0 {
                continue;
            }
            let damaged: Vec<&ShipRecord> = record
                .ships
                .iter()
                .filter(|s| s.flags & 0x80_1000 != 0 && s.flags & 0x800 == 0)
                .collect();
            if damaged.is_empty() {
                continue;
            }
            let ships = damaged.iter().map(|s| s.index).collect();
            for s in &damaged {
                self.release_ship(io, s.id);
            }
            let to = io.world.fleets.get(fleet)?.location;
            return Some(AIAction::CreateFleet { fleet, ships, to });
        }
        None
    }

    /// `FUN_00477cb0`: the counts from the held fleets' records, the bits
    /// of the positive ones in `+0x64`, and `counts[2]`, `counts[3]` kept
    /// in `+0x94`, `+0x98`.
    fn sum_fleets(&mut self, io: &PlannerIo<'_>) {
        self.counts = [0; 10];
        for fleet in &self.fleets {
            let Some(r) = io.galaxy.fleets.get(fleet) else {
                continue;
            };
            for (count, at, _) in FLEET_SUMS {
                self.counts[count] += r.values[at];
            }
            if r.flags & 0x10 != 0 {
                self.counts[4] += 1;
            }
            if r.flags & 0x20 != 0 {
                self.counts[6] += 1;
            }
        }
        for (count, _, bit) in FLEET_SUMS {
            if self.counts[count] > 0 {
                self.requests |= bit;
            }
        }
        if self.counts[6] > 0 {
            self.requests |= 0x10_0000;
        }
        self.held = [self.counts[2], self.counts[3]];
    }

    /// The plan's slot `+0x50` (`FUN_004adaf0`'s first call). Kind 3,
    /// `FUN_004b4920`: at least 1.2 times the sector's `+0x190` and
    /// `+0x194` (or controller `+0x164`, `+0x168`), and of its `+0x198`
    /// with `+0x138`, never less than before; one ship with lasers; one
    /// Death Star with controller `+4` bit `0x1000000`; the regiments it
    /// has.
    ///
    /// Kind 1, `FUN_004bb510`: the same, but `+0x34` has no controller
    /// floor, and `+0x48` takes 1.1 times the sector's `+0x144` (or
    /// controller `+0x174`). Kind 2, `FUN_004b9fe0`: 1.5 times (`* 3 / 2`)
    /// for 1.2, `+0x38` also adding 1.5 times the sector's `+0x13c`, and
    /// `+0x40` with the floor controller `+0x16c`.
    ///
    /// Kind 4, `FUN_004b8b70`: [`Self::choose_target`] first, then
    /// [`Self::want_invasion`]. Kind 5, `FUN_004b3300`: 1.5 times the
    /// sector's `+0x190` and `+0x194` (the latter with the floor controller
    /// `+0x168` only with an opponent system, `+0x2c` 8), 1.2 times its
    /// `+0x138` with `+0x198`, `+0x3c` at least 1, then kind 4's tail.
    ///
    /// hyp: controller `+0x164`, `+0x168`, `+0x16c` and `+0x174` have no
    /// writer found and read as 0.
    fn want(&mut self, io: &mut PlannerIo<'_>) {
        if !ported_kind(self.kind) {
            return;
        }
        if self.kind == 4 {
            self.choose_target(io);
            self.want_invasion(io);
            return;
        }
        let Some(r) = self.sector.and_then(|s| io.galaxy.sector(s)) else {
            return;
        };
        let v = |offset| r.values[sector_at(offset)];
        let w = &mut self.wanted;
        if self.kind == 5 {
            // FUN_004b3300.
            w[0] = w[0].max(v(0x190) * 3 / 2);
            let mut ships = v(0x194) * 3 / 2;
            if r.summary & 8 != 0 {
                // Controller +0x168 (read as 0) when higher.
                ships = ships.max(0);
            }
            w[1] = w[1].max(ships);
            w[3] = w[3].max((v(0x138) + v(0x198)) * 12 / 10);
            w[2] = w[2].max(1);
            if io.controller_flags & 4 != 0 && v(0x19c) > 0 {
                w[4] = 1;
            }
            if io.controller_flags & 0x100_0000 != 0 {
                w[6] = 1;
            }
            w[7] = self.held[0];
            w[8] = self.held[1];
            w[9] = 1;
            return;
        }
        match self.kind {
            1 => {
                w[0] = w[0].max(v(0x190) * 12 / 10);
                w[5] = w[5].max((v(0x144) * 11 / 10).max(0));
            }
            2 => {
                w[0] = w[0].max((v(0x190) * 3 / 2).max(0));
                w[1] = w[1].max((v(0x13c) * 3 / 2 + v(0x194) * 3 / 2).max(0));
                w[3] = w[3].max(((v(0x138) + v(0x198)) * 3 / 2).max(0));
            }
            _ => w[0] = w[0].max((v(0x190) * 12 / 10).max(0)),
        }
        if self.kind != 2 {
            w[1] = w[1].max((v(0x194) * 12 / 10).max(0));
            w[3] = w[3].max((v(0x198) + v(0x138)) * 12 / 10);
        }
        w[2] = 0;
        w[4] = 0;
        if io.controller_flags & 0x100_0000 != 0 {
            w[6] = 1;
        }
        w[7] = 0;
        w[9] = 1;
        w[8] = self.held[1];
    }

    /// The planner's request (`FUN_00476b70`, the plan object's slot
    /// `+0x0c`): kind 4's (`FUN_004b7100`), with both records, is its
    /// base's garrison term `v[20]` less its `v[16]`, plus the target's
    /// garrison need `v[52]`; kind 5's (`FUN_004b17b0`), with the sector's
    /// and the base's records, the base's `v[20]` less its `v[16]`, less
    /// the sector's own systems (`+0x78`), plus 10; kinds 1 to 3's slot is
    /// `0x6158b0` (0).
    ///
    /// port: kind 6's slot (`FUN_004b4df0`) is not ported and asks for
    /// nothing.
    #[must_use]
    pub fn request(&self, galaxy: &Galaxy) -> i32 {
        if self.kind == 5 {
            let (Some(sector), Some(base)) = (
                self.sector.and_then(|s| galaxy.sector(s)),
                self.base.and_then(|b| galaxy.system(b)),
            ) else {
                return 0;
            };
            return base.values[20] - base.values[16] - sector.values[sector_at(0x78)] + 10;
        }
        if self.kind != 4 {
            return 0;
        }
        let (Some(target), Some(base)) = (
            self.plan_target.and_then(|t| galaxy.system(t)),
            self.base.and_then(|b| galaxy.system(b)),
        ) else {
            return 0;
        };
        base.values[20] - base.values[16] + target.values[52]
    }

    /// Kind 4's target (`FUN_004b8f80`): with the sector record, the yard
    /// key of the plan's parameter (sector `+0x34`: 1 shipyards `v[4]`, 2
    /// training facilities `v[5]`, 3 construction yards `v[6]`, else
    /// `v[0]`), the picks of the sector's systems held by the opponent
    /// (`+0x30` `0x20` forbidding `0x101`) merged by that key and by
    /// `v[0x14]` (both order 2): the first whose record has the key above 0
    /// becomes `+0x5c`, else none. Without the sector record `+0x5c` stays.
    pub(crate) fn choose_target(&mut self, io: &mut PlannerIo<'_>) {
        let Some(sector) = self.sector else {
            return;
        };
        let Some(param) = io.galaxy.sector(sector).map(|r| r.plan_param) else {
            return;
        };
        let key = match param {
            1 => 4,
            2 => 5,
            3 => 6,
            _ => 0,
        };
        let masks = [ANY, ANY, Mask::new(0x20, 0x101)];
        let mut coin = || (io.random)(10) < 5;
        let by_yard = io
            .galaxy
            .system_query(sector, masks, key, Order::Descending, &mut coin);
        let by_support = io
            .galaxy
            .system_query(sector, masks, 0x14, Order::Descending, &mut coin);
        self.plan_target = picks(vec![by_yard, by_support])
            .into_iter()
            .find(|&s| io.galaxy.system(s).is_some_and(|r| r.values[key] > 0));
    }

    /// Kind 4's wants (`FUN_004b8b70`): `+0x34` 1.5 times the sector's
    /// `+0x190` (floor controller `+0x164`); `+0x38` 1.5 times its `+0x194`
    /// plus 1.5 times the target's shields `v[27]` (floor `+0x168`); `+0x40`
    /// 1.1 times its `+0x138` with `+0x198` (floor `+0x16c`); `+0x3c` the
    /// target's garrison need `v[52]`; each never less than before; `+0x44`
    /// 1 with controller `+4` bit 4 and the sector's `+0x19c` above 0;
    /// `+0x4c` 1 with `0x1000000`; `+0x50` and `+0x54` the regiments the
    /// planner has (`+0x94`, `+0x98`); `+0x58` 1.
    ///
    /// hyp: without a target `+0x3c` compares the `+0x40` value left in the
    /// register.
    fn want_invasion(&mut self, io: &PlannerIo<'_>) {
        let Some(r) = self.sector.and_then(|s| io.galaxy.sector(s)) else {
            return;
        };
        let v = |offset| r.values[sector_at(offset)];
        let target = self.plan_target.and_then(|t| io.galaxy.system(t));
        let w = &mut self.wanted;
        w[0] = w[0].max((v(0x190) * 3 / 2).max(0));
        let shields = target.map_or(0, |t| t.values[27] * 3 / 2);
        w[1] = w[1].max((v(0x194) * 3 / 2 + shields).max(0));
        let troops = ((v(0x198) + v(0x138)) * 11 / 10).max(0);
        w[3] = w[3].max(troops);
        w[2] = w[2].max(target.map_or(troops, |t| t.values[52]));
        if io.controller_flags & 4 != 0 && v(0x19c) > 0 {
            w[4] = 1;
        }
        if io.controller_flags & 0x100_0000 != 0 {
            w[6] = 1;
        }
        w[7] = self.held[0];
        w[8] = self.held[1];
        w[9] = 1;
    }

    /// `FUN_00477e80`, state 3: with a count short (below 0), take ships
    /// (`FUN_004782a0`), else take them from a later plan's planner
    /// (`FUN_00478690`), and mark each short count in `+0x64`; with none
    /// taken and a count to spare, give one ship back (`FUN_00478aa0`).
    fn balance(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        let short = |p: &Self| BALANCED.iter().any(|&c| p.counts[c] < 0);
        let mut order = None;
        if short(self) {
            order = self.acquire(io);
            if order.is_none() && short(self) {
                order = self.steal(io);
            }
            for (count, bit) in [
                (0, 1),
                (1, 2),
                (2, 4),
                (3, 8),
                (5, 0x10),
                (7, 0x20),
                (8, 0x40),
                (9, 0x80),
                (6, 0x800),
            ] {
                if self.counts[count] < 0 {
                    self.requests |= bit;
                }
            }
        }
        if order.is_none() && BALANCED.iter().any(|&c| self.counts[c] > 0) {
            order = self.spare(io);
        }
        order
    }

    /// The ship tests `FUN_004782a0` and `FUN_00478690` share: whether
    /// the ship meets a short count, and whether no count is short (which
    /// ends the search).
    fn meets(&self, ship: &ShipRecord) -> (bool, bool) {
        let f = ship.flags;
        let v = &ship.values;
        let mut meets = false;
        let mut none_short = true;
        for (count, gives) in [
            (0, f & 1 != 0),
            (1, f & 0x102 != 0),
            (9, f & 2 != 0),
            (2, f & 0xc != 0),
            (3, v[7] > 0),
            (5, v[1] > 0),
            (4, f & 0x80 != 0),
            (6, f & 0x400 != 0),
        ] {
            if self.counts[count] < 0 && !meets {
                none_short = false;
                meets = gives;
            }
        }
        (meets, none_short)
    }

    /// `FUN_004782a0`: the side's free fleets nearest the sector (`0x8005`,
    /// not en route; `FUN_0041a600`, order 2), in turn, until one gives
    /// ships: its free ships fit to sail (`0x20000`, without `0xc03800`)
    /// that meet a short count are taken, and leave as a fleet for the
    /// planner's system (Create Fleet `0x270`, `FUN_00479220`).
    fn acquire(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        let home = self.sector.and_then(|s| io.galaxy.sector(s))?.position;
        let mut coin = || (io.random)(10) < 5;
        let fleets = io.galaxy.fleets_near(
            io.world,
            home,
            Mask::new(0x8005, 2),
            Order::Descending,
            &mut coin,
        );
        let mut done = false;
        for fleet in picks(vec![fleets]) {
            let ships = io.galaxy.fleets.get(&fleet).map(|r| r.ships.clone())?;
            let mut taken = Vec::new();
            for ship in ships {
                if done {
                    break;
                }
                if ship.flags & SHIP_FREE == 0 || ship.flags & 0xc0_3800 != 0 {
                    continue;
                }
                let (meets, none_short) = self.meets(&ship);
                done = none_short;
                if meets {
                    taken.push(ship.index);
                    self.take_ship(io, ship.id);
                }
            }
            if !taken.is_empty() {
                return Some(self.create_fleet(io, fleet, taken));
            }
        }
        None
    }

    /// `FUN_00478690`: the side's fleets held for a later plan (`+0x34`
    /// above this one's; `0x8001` without `0xf0000006`; `FUN_0041ab40`,
    /// order 1, the latest first), in turn, until one gives ships: its
    /// held ships fit to sail (without `0xc23800`) that meet a short count
    /// are taken, a flagship only marked (`0x2000000`).
    fn steal(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        let items: Vec<(FleetKey, i32)> = io
            .world
            .fleets
            .keys()
            .filter_map(|key| {
                let r = io.galaxy.fleets.get(&key)?;
                (r.plan > self.plan_id && Mask::new(0x8001, 0xf000_0006).admits(r.flags))
                    .then(|| (key, i32::try_from(r.plan).unwrap_or(i32::MAX)))
            })
            .collect();
        let mut coin = || (io.random)(10) < 5;
        let fleets = ranked(items, Order::Ascending, &mut coin);
        let mut done = false;
        for fleet in picks(vec![fleets]) {
            let ships = io.galaxy.fleets.get(&fleet).map(|r| r.ships.clone())?;
            let mut taken = Vec::new();
            for ship in ships {
                if done {
                    break;
                }
                if ship.flags & 0xc2_3800 != 0 {
                    continue;
                }
                let (meets, none_short) = self.meets(&ship);
                done = none_short;
                if meets {
                    if ship.flags & SHIP_FLAGSHIP == 0 {
                        taken.push(ship.index);
                        self.take_ship(io, ship.id);
                    } else {
                        io.edit_ship(ship.id, |r| r.flags |= SHIP_PASSED);
                    }
                }
            }
            if !taken.is_empty() {
                return Some(self.create_fleet(io, fleet, taken));
            }
        }
        None
    }

    /// `FUN_00478aa0`: the first held ship able to leave (without
    /// `0x402800`, `FUN_00484510`: its fleet has no role and is not at
    /// the opponent's system) whose values the spare counts cover is given
    /// back as a fleet of its own; the flagship of several ships is only
    /// marked (`0x2000000`).
    fn spare(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        for id in self.ships.clone() {
            let Some(ship) = io.ship(id).cloned() else {
                continue;
            };
            if ship.flags & 0x40_2800 != 0 || self.busy(io, &ship) {
                continue;
            }
            let c = &self.counts;
            let v = &ship.values;
            let f = ship.flags;
            let spare = v[14] <= c[0]
                && v[15] <= c[1]
                && (f & 2 == 0 || c[9] >= 1)
                && (f & 0xc == 0 || v[10] <= c[2])
                && (v[7] <= 0 || v[7] <= c[3])
                && (v[1] <= 0 || v[1] <= c[5])
                && (f & 0x80 == 0 || c[4] >= 1)
                && (f & 0x400 == 0 || c[6] >= 1);
            if !spare {
                continue;
            }
            if self.flagship == Some(id) && self.ships.len() > 1 {
                io.edit_ship(id, |r| r.flags |= SHIP_PASSED);
                continue;
            }
            self.release_ship(io, id);
            return Some(self.create_fleet(io, ship.fleet, vec![ship.index]));
        }
        None
    }

    /// `FUN_00484510`: a ship whose fleet has a role (`+0x38`
    /// `0xf0000000`) or stands at a system of the opponent's (`+0x30` 4).
    fn busy(&self, io: &PlannerIo<'_>, ship: &ShipRecord) -> bool {
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

    /// Create Fleet `0x270` of `ships` of `fleet` for the system
    /// `FUN_00479220` gives.
    fn create_fleet(&self, io: &mut PlannerIo<'_>, fleet: FleetKey, ships: Vec<usize>) -> AIAction {
        let here = io.world.fleets.get(fleet).map(|f| f.location);
        let to = self.destination(io, here).or(here).unwrap_or_default();
        AIAction::CreateFleet { fleet, ships, to }
    }

    /// `FUN_00479220`: the planner's base unless blockaded (`+0x28` 2);
    /// else its sector's headquarters (`+0x28` `0x80000000`, own, not
    /// blockaded; key 7, order 2); else the own system of its sector
    /// nearest `origin` (`FUN_0041a400`); else an own system of the nearest
    /// sector that has one (`FUN_0041b2b0`).
    fn destination(&self, io: &mut PlannerIo<'_>, origin: Option<SystemKey>) -> Option<SystemKey> {
        if let Some(base) = self
            .base
            .filter(|&b| io.galaxy.system(b).is_some_and(|r| r.state & 2 == 0))
        {
            return Some(base);
        }
        let sector = self.sector?;
        let own = [Mask::new(0, 2), ANY, Mask::new(1, 0)];
        let mut coin = || (io.random)(10) < 5;
        let headquarters = io.galaxy.system_query(
            sector,
            [Mask::new(0x8000_0000, 2), ANY, Mask::new(1, 0)],
            7,
            Order::Descending,
            &mut coin,
        );
        if let Some(s) = pick(&[headquarters]) {
            return Some(s);
        }
        if let Some(origin) = origin {
            let near =
                io.galaxy
                    .systems_by_distance(sector, origin, own, Order::Descending, &mut coin);
            if let Some(s) = pick(&[near]) {
                return Some(s);
            }
        }
        nearest_sector_system(io, sector, own)
    }

    /// `FUN_00478000`, state 4: the plan's mission request (slot `+0x18`)
    /// into `+0xbc`/`+0xc0`, `+0x64` bit `0x100` with one; the mission
    /// planners it holds (`+0xb0`) set more bits.
    ///
    /// port: the sector planners' mission planners are not ported, so it
    /// holds none and every count of them (`+0xa0..+0xac`) is 0. Kinds 1
    /// and 3's slot (`FUN_004b7150`) then asks for Recon (`0x54`) in a sector with
    /// a system not the side's (`+0x2c` `0x18`), else Espionage (`0x52`).
    fn missions(&mut self, io: &PlannerIo<'_>) {
        self.mission = [0; 3];
        if !ported_kind(self.kind) {
            return;
        }
        if let Some(r) = self.sector.and_then(|s| io.galaxy.sector(s)) {
            self.mission[0] = if r.summary & 0x18 != 0 { 0x54 } else { 0x52 };
        }
        if self.mission[0] != 0 {
            self.requests |= 0x100;
        }
    }

    /// `FUN_00478140`: the plan's movement request (slot `+0x1c`) into
    /// `+0xe0`, `+0x64` bit `0x200` with one.
    ///
    /// port: the sector planners' movement planners (module `0xc`, list
    /// `+0xd4`) are not ported, so it holds none. Kind 3's slot
    /// (`FUN_004b1800`, the same code as kind 1's `FUN_004b9290`) asks for kind 3 while the planner holds a fleet,
    /// else kind 5 while it holds regiments (`+0x98`) in a sector with
    /// `+0x2c` `0x40008`.
    fn movement(&mut self, io: &PlannerIo<'_>) {
        self.movement = [0; 2];
        self.flags &= !0x700_0000;
        if !ported_kind(self.kind) {
            return;
        }
        if let Some(r) = self.sector.and_then(|s| io.galaxy.sector(s)) {
            // Kind 4 (FUN_004b72d0) first asks for kind 4 with regiments
            // (+0x94) in a sector with +0x2c 8 and no movement planner of
            // that kind (+0xcc).
            if self.kind == 4 && self.held[0] > 0 && r.summary & 8 != 0 {
                self.movement[0] = 4;
            } else if !self.fleets.is_empty() {
                self.movement[0] = 3;
            } else if self.held[1] > 0 && r.summary & 0x4_0008 != 0 {
                self.movement[0] = 5;
            }
        }
        if self.movement[0] != 0 {
            self.requests |= 0x200;
        }
    }

    /// `FUN_00479560` without movement planners (`+0x60` `0x2000000`
    /// clear): the flagship, when this planner holds it, loses its mark.
    fn flagship(&mut self, io: &mut PlannerIo<'_>) {
        if let Some(id) = self.flagship.take() {
            if io.ship(id).is_some_and(|r| r.owner == self.id) {
                io.edit_ship(id, |r| r.flags &= !SHIP_FLAGSHIP);
            }
        }
    }

    /// The plan's slot `+0x20` and `FUN_00479a00`: kind 3
    /// (`FUN_004b3830`) sets `0x4000000`, with a sector record
    /// `0x8000000`, and `0x20000` when the sector has a system of the
    /// side's (`+0x2c` 4); kind 1 (`FUN_004ba5e0`) the same with the
    /// sector's `+0x2c` `0x80000`; kind 2 (`FUN_004b9310`) with
    /// `0x40000`, else ([`Self::sweep_left`]) while a system of the
    /// opponent is left to visit, else it marks the sector swept. Then from `+0x64`: nothing short (`0x4ff`;
    /// the mission and movement requests `0x100`, `0x200` are outside it)
    /// gives `0x1c0000`; short and nothing held (`0x9fe00000`), `0x10000`
    /// (gave up); else `0x100000`, and `0x80000` when none of `0x1f` is
    /// short.
    fn report(&mut self, io: &mut PlannerIo<'_>) {
        if ported_kind(self.kind) {
            self.flags |= CYCLED;
            let bit = match self.kind {
                1 => 0x8_0000,
                2 => 0x4_0000,
                _ => 4,
            };
            if let Some(summary) = self
                .sector
                .and_then(|s| io.galaxy.sector(s))
                .map(|r| r.summary)
            {
                self.flags |= REVIEWED;
                if self.kind == 4 {
                    // FUN_004b70a0: while the target is not the side's.
                    if self
                        .plan_target
                        .and_then(|t| io.galaxy.system(t))
                        .is_some_and(|r| r.kind & 1 == 0)
                    {
                        self.flags |= 0x2_0000;
                    }
                } else if self.kind == 5 {
                    // FUN_004b1760: while the sector is not wholly the
                    // side's (+0x2c 2) or holds an opponent's system (8).
                    if summary & 2 == 0 || summary & 8 != 0 {
                        self.flags |= 0x2_0000;
                    }
                } else if summary & bit != 0 {
                    self.flags |= 0x2_0000;
                } else if self.kind == 2 {
                    self.sweep(io);
                }
            }
        }
        if self.requests & 0x4ff == 0 {
            self.flags |= 0x1c_0000;
        } else if self.requests & 0x9fe0_0000 == 0 {
            self.flags |= 0x1_0000;
        } else {
            self.flags |= 0x10_0000;
            if self.requests & 0x1f == 0 {
                self.flags |= 0x8_0000;
            }
        }
    }
}

impl SectorPlanner {
    /// Kind 2's report without the sector's `0x40000` (`FUN_004b9310`):
    /// over the systems of the opponent with no enemy fleet or role there
    /// (`FUN_0041a3a0`: `+0x30` `0x20` forbidding `0x700201`, order 1),
    /// one not on the visited list gives `0x20000`; with none the sector
    /// record takes `0x20000000` ([`crate::ai::AIState::swept`]).
    fn sweep(&mut self, io: &mut PlannerIo<'_>) {
        let Some(sector) = self.sector else {
            return;
        };
        if self.sweep_left(io, sector).is_some() {
            self.flags |= 0x2_0000;
        } else {
            if let Some(r) = io.galaxy.sectors.iter_mut().find(|r| r.sector == sector) {
                r.summary |= 0x2000_0000;
            }
            if !io.swept.contains(&sector) {
                io.swept.push(sector);
            }
        }
    }

    /// The first system of `FUN_0041a3a0(sector, +0x30 0x20 forbidding
    /// 0x700201, order 1)`, in pick order, not on kind 2's visited list
    /// (`FUN_004f5940` on `+0x5c`).
    pub(crate) fn sweep_left(
        &self,
        io: &mut PlannerIo<'_>,
        sector: SectorKey,
    ) -> Option<SystemKey> {
        let mut coin = || (io.random)(10) < 5;
        let systems = io.galaxy.system_query_by_age(
            sector,
            [ANY, ANY, Mask::new(0x20, 0x70_0201)],
            (io.day, io.seen),
            Order::Ascending,
            &mut coin,
        );
        picks(vec![systems])
            .into_iter()
            .find(|s| !self.visited.contains(s))
    }
}

/// The plan kinds whose plan object's slots are ported: 1 (vtable
/// `0x65c2c0`), 2 (`0x65c268`), 3 (`0x65c160`), 4 (`0x65c210`) and 5
/// (`0x65c108`).
pub(crate) fn ported_kind(kind: u32) -> bool {
    matches!(kind, 1..=5)
}

/// `FUN_0041b2b0`: of the sectors but `from`, by squared distance (the
/// first of equals), the nearest whose systems include one passing
/// `masks`, picked by a random key (`FUN_0041a460`, order 1).
pub(crate) fn nearest_sector_system(
    io: &mut PlannerIo<'_>,
    from: SectorKey,
    masks: [Mask; 3],
) -> Option<SystemKey> {
    let home = io.galaxy.sector(from)?.position;
    let mut best: Option<(i32, SystemKey)> = None;
    let sectors: Vec<(SectorKey, (i16, i16))> = io
        .galaxy
        .sectors
        .iter()
        .filter(|s| s.sector != from)
        .map(|s| (s.sector, s.position))
        .collect();
    for (sector, at) in sectors {
        let d = crate::ai_records::squared_distance(home, at);
        if best.is_some_and(|(top, _)| d >= top) {
            continue;
        }
        if let Some(system) = random_system(io, sector, masks) {
            best = Some((d, system));
        }
    }
    best.map(|(_, s)| s)
}

/// `FUN_0041a460` -> `FUN_00431f40`: `sector`'s systems passing `masks`,
/// each keyed on a draw below 10 as it goes into the list (order 1),
/// picked.
pub(crate) fn random_system(
    io: &mut PlannerIo<'_>,
    sector: SectorKey,
    masks: [Mask; 3],
) -> Option<SystemKey> {
    let systems: Vec<SystemKey> = io
        .galaxy
        .sector(sector)?
        .systems
        .iter()
        .filter(|r| {
            masks[0].admits(r.state) && masks[1].admits(r.summary) && masks[2].admits(r.kind)
        })
        .map(|r| r.system)
        .collect();
    let draws = std::cell::RefCell::new(&mut *io.random);
    let items = systems.into_iter().map(|s| (s, (draws.borrow_mut())(10)));
    pick(&[ranked(items, Order::Ascending, &mut || {
        (draws.borrow_mut())(10) < 5
    })])
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ai_records::{
        ClassRecords, SystemInputs, GARRISON_GOALS, SHIELD_GOAL, SQUADRON_GOALS,
    };
    use crate::dat::Faction;
    use crate::ids::DatId;
    use crate::world::{CapitalShipClass, ControlKind, Fleet, Sector, ShipInstance, System};

    /// Two sectors (at 0 and 100), each with two Alliance systems; the
    /// first sector's second system is far from its first.
    pub(crate) struct Fx {
        pub(crate) world: GameWorld,
        pub(crate) galaxy: Galaxy,
        pub(crate) sectors: [SectorKey; 2],
        pub(crate) systems: [SystemKey; 4],
        pub(crate) ship_holds: HashMap<u64, ShipHold>,
        pub(crate) fleet_holds: HashMap<FleetKey, FleetHold>,
        pub(crate) kinds: HashMap<SystemKey, u32>,
        pub(crate) transport: crate::troop_transport::TroopTransportState,
        pub(crate) people: HashMap<crate::ids::CharacterKey, u32>,
        pub(crate) draws: Vec<i32>,
        pub(crate) day: u64,
        pub(crate) seen: HashMap<SystemKey, u64>,
        pub(crate) swept: Vec<SectorKey>,
    }

    pub(crate) fn fixture() -> Fx {
        let mut world = GameWorld::default();
        let mut sector = |x| {
            world.sectors.insert(Sector {
                dat_id: DatId::new(0x8000_0001),
                name: "Sector".into(),
                group: crate::dat::SectorGroup::Core,
                x,
                y: 0,
                systems: vec![],
            })
        };
        let sectors = [sector(0), sector(100)];
        let mut system = |sector, x| {
            world.systems.insert(System {
                dat_id: DatId::new(0x9000_0001),
                name: "System".into(),
                sector,
                x,
                y: 0,
                exploration_status: crate::dat::ExplorationStatus::Explored,
                popularity_alliance: 0.9,
                popularity_empire: 0.1,
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
                fighters: Vec::new(),
                is_headquarters: false,
                is_destroyed: false,
                control: ControlKind::Controlled(Faction::Alliance),
            })
        };
        let systems = [
            system(sectors[0], 0),
            system(sectors[0], 30),
            system(sectors[1], 100),
            system(sectors[1], 130),
        ];
        let mut fx = Fx {
            world,
            galaxy: Galaxy::default(),
            sectors,
            systems,
            ship_holds: HashMap::new(),
            fleet_holds: HashMap::new(),
            kinds: HashMap::new(),
            transport: crate::troop_transport::TroopTransportState::new(),
            people: HashMap::new(),
            draws: Vec::new(),
            day: 0,
            seen: HashMap::new(),
            swept: Vec::new(),
        };
        rebuild(&mut fx);
        fx
    }

    /// An Alliance fleet of `n` ships at `at`.
    pub(crate) fn fleet(fx: &mut Fx, at: usize, n: u32) -> FleetKey {
        let class = fx.world.capital_ship_classes.insert(CapitalShipClass {
            is_alliance: true,
            hull: 10,
            ..CapitalShipClass::default()
        });
        let at = fx.systems[at];
        let fleet = fx.world.fleets.insert(Fleet {
            location: at,
            capital_ships: ShipInstance::make(class, 10, true, n),
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        fx.world.systems[at].fleets.push(fleet);
        fx.world.number_ships();
        rebuild(fx);
        fleet
    }

    pub(crate) fn rebuild(fx: &mut Fx) {
        let classes = ClassRecords::build(&fx.world);
        let fleets = HashMap::new();
        let economy = crate::economy::EconomyState::default();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = crate::manufacturing::ManufacturingState::new();
        let research = crate::research::ResearchState::new();
        let people = &fx.people;
        let person = |c| people.get(&c).copied();
        let inputs = SystemInputs {
            world: &fx.world,
            classes: &classes,
            fleets: &fleets,
            person: &person,
            economy: &economy,
            uprisings: &uprisings,
            manufacturing: &manufacturing,
            research: &research,
            is_alliance: true,
            minor_classes: 0,
            shield_goal: SHIELD_GOAL,
            squadron_goals: SQUADRON_GOALS,
            garrison_goals: GARRISON_GOALS,
        };
        fx.galaxy = Galaxy::build(
            &inputs,
            &crate::movement::MovementState::new(),
            &fx.transport,
            &HashMap::new(),
            &fx.kinds,
        );
        crate::ai_records::apply_ship_holds(&mut fx.galaxy, &fx.ship_holds);
        crate::ai_records::apply_fleet_holds(&mut fx.galaxy, &fx.fleet_holds);
    }

    pub(crate) fn with<R>(fx: &mut Fx, f: impl FnOnce(&mut PlannerIo<'_>) -> R) -> R {
        let draws = &mut fx.draws;
        let mut random = |n: i32| {
            let d = if draws.is_empty() { 0 } else { draws.remove(0) };
            d.rem_euclid(n)
        };
        let mut io = PlannerIo {
            world: &fx.world,
            galaxy: &mut fx.galaxy,
            ship_holds: &mut fx.ship_holds,
            fleet_holds: &mut fx.fleet_holds,
            kinds: &mut fx.kinds,
            transport: &fx.transport,
            people: &fx.people,
            controller_flags: 0,
            day: fx.day,
            seen: &fx.seen,
            swept: &mut fx.swept,
            random: &mut random,
        };
        f(&mut io)
    }

    pub(crate) fn planner(fx: &Fx) -> SectorPlanner {
        SectorPlanner {
            id: 1,
            sector: Some(fx.sectors[0]),
            plan_id: 1,
            kind: 3,
            ..SectorPlanner::default()
        }
    }

    pub(crate) fn ship_mut(fx: &mut Fx, fleet: FleetKey, i: usize) -> &mut ShipRecord {
        &mut fx.galaxy.fleets.get_mut(&fleet).unwrap().ships[i]
    }

    pub(crate) fn ship_id(fx: &Fx, fleet: FleetKey, i: usize) -> u64 {
        fx.world.fleets[fleet].capital_ships[i].id
    }

    #[test]
    fn a_held_ship_names_its_planner_and_adds_its_values() {
        // FUN_00478e00: +0x24 loses 0x20000, +0x30/+0x34 the planner and
        // its plan; +0x7c, +0x80, +0x6c, +0x60, +0x48 (when positive) add
        // to +0x6c..+0x80 with their +0x64 bits, +0x64 and +0x58 to +0x88,
        // +0x8c, and bits 0x80, 0x400, 2 count. FUN_004790d0 takes them
        // off again and frees the ship.
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 1);
        let id = ship_id(&fx, f, 0);
        {
            let s = ship_mut(&mut fx, f, 0);
            for (at, v) in [(14, 5), (15, 3), (10, 2), (7, 1), (1, 4), (8, 6), (5, 7)] {
                s.values[at] = v;
            }
            s.flags |= 0x80 | 0x400 | 2;
        }
        let mut p = planner(&fx);
        with(&mut fx, |io| p.take_ship(io, id));
        assert_eq!(p.counts, [5, 3, 2, 1, 1, 4, 1, 6, 7, 1]);
        assert_eq!(p.requests, 0x3e0_0000);
        assert_eq!(p.ships, vec![id]);
        let r = &fx.galaxy.fleets[&f].ships[0];
        assert_eq!((r.flags & SHIP_FREE, r.owner, r.plan), (0, 1, 1));
        assert_eq!(
            fx.ship_holds[&id],
            ShipHold {
                flags: 0,
                owner: 1,
                plan: 1
            }
        );
        with(&mut fx, |io| p.release_ship(io, id));
        assert_eq!(p.counts, [0; 10]);
        assert!(p.ships.is_empty());
        let r = &fx.galaxy.fleets[&f].ships[0];
        assert_eq!((r.flags & SHIP_FREE, r.owner), (SHIP_FREE, 0));
    }

    #[test]
    fn a_ship_another_planner_took_is_dropped_without_freeing_it() {
        // FUN_004790d0 frees the record only when +0x30 is this planner;
        // the values still leave the counts and the id the list.
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 1);
        let id = ship_id(&fx, f, 0);
        ship_mut(&mut fx, f, 0).values[14] = 5;
        let mut p = planner(&fx);
        with(&mut fx, |io| p.take_ship(io, id));
        ship_mut(&mut fx, f, 0).owner = 9;
        with(&mut fx, |io| p.release_ship(io, id));
        assert_eq!(p.counts[0], 0);
        assert!(p.ships.is_empty());
        let r = &fx.galaxy.fleets[&f].ships[0];
        assert_eq!((r.flags & SHIP_FREE, r.owner), (0, 9));
    }

    #[test]
    fn a_held_fleet_names_its_planner_until_given_back() {
        // FUN_00478fd0 clears +0x38 bit 4 and sets +0x30/+0x34;
        // FUN_004791d0 sets bit 4 and clears them.
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 1);
        let mut p = planner(&fx);
        with(&mut fx, |io| p.take_fleet(io, f));
        let r = &fx.galaxy.fleets[&f];
        assert_eq!((r.flags & FLEET_FREE, r.owner, r.plan), (0, 1, 1));
        assert_eq!(p.fleets, vec![f]);
        assert_eq!(fx.fleet_holds[&f].owner, 1);
        with(&mut fx, |io| p.release_fleet(io, f));
        let r = &fx.galaxy.fleets[&f];
        assert_eq!((r.flags & FLEET_FREE, r.owner), (FLEET_FREE, 0));
        assert!(p.fleets.is_empty());
    }

    #[test]
    fn the_lists_follow_the_records_after_an_order() {
        // FUN_00477810: a held ship still this planner's and the side's
        // keeps its fleet held (a free one is taken); one another planner
        // took is given back. FUN_00477950: a held fleet with no ships or
        // another holder is given back.
        let mut fx = fixture();
        let a = fleet(&mut fx, 0, 2);
        let b = fleet(&mut fx, 0, 1);
        let mut p = planner(&fx);
        let (a0, a1, b0) = (ship_id(&fx, a, 0), ship_id(&fx, a, 1), ship_id(&fx, b, 0));
        with(&mut fx, |io| {
            p.take_ship(io, a0);
            p.take_ship(io, a1);
            p.take_ship(io, b0);
            p.take_fleet(io, b);
        });
        ship_mut(&mut fx, a, 1).owner = 7;
        fx.galaxy.fleets.get_mut(&b).unwrap().owner = 7;
        p.resync = true;
        p.kind = 0;
        assert_eq!(with(&mut fx, |io| p.step(io)), PlannerStep::Done);
        assert!(!p.resync);
        assert_eq!(p.ships, vec![a0, b0]);
        assert_eq!(p.fleets, vec![a]);
    }

    #[test]
    fn a_cycle_runs_state_by_state_and_reports() {
        // FUN_004774c0: 0 -> 1 (resync), 1 (clears +0x60/+0x64), 2, 3, 4,
        // 5 (the plan object), 6 (report) -> 1, done. The plan object
        // (FUN_004adb70) steps 0 -> 1, 1, 2 (a base), 3, 4, 5 (no held
        // fleet: 0xe), 0xe (its walk ends: 0, *out), and state 5 goes on
        // to 6.
        let mut fx = fixture();
        let mut p = planner(&fx);
        p.flags = 0xff;
        let mut steps = Vec::new();
        for _ in 0..12 {
            steps.push(with(&mut fx, |io| p.step(io)));
        }
        assert!(steps.iter().all(|s| *s == PlannerStep::Pending));
        assert_eq!((p.state, p.plan_state), (6, 0));
        assert!(p.base.is_some());
        assert_eq!(with(&mut fx, |io| p.step(io)), PlannerStep::Done);
        assert_eq!((p.state, p.resync), (1, true));
        // Kind 3's slot +0x20 and FUN_00479a00: cycled and reviewed, the
        // sector's own system (0x20000), something asked and nothing
        // held: gave up (0x10000).
        assert_eq!(p.flags, CYCLED | REVIEWED | 0x2_0000 | 0x1_0000);
        // Another kind's planner stands in for its whole cycle.
        let mut other = SectorPlanner {
            kind: 6,
            ..planner(&fx)
        };
        assert_eq!(with(&mut fx, |io| other.step(io)), PlannerStep::StandIn);
        other.state = 5;
        other.plan_done();
        assert_eq!((other.state, other.resync), (1, true));
    }

    #[test]
    fn damaged_ships_of_a_held_fleet_leave_as_their_own_fleet() {
        // FUN_00477a40: the first held fleet with 0x3000 and without
        // 0xf0000802 gives back its ships with 0x801000 (not en route) and
        // orders Create Fleet 0x270 for them.
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 3);
        let mut p = planner(&fx);
        let ids: Vec<u64> = (0..3).map(|i| ship_id(&fx, f, i)).collect();
        with(&mut fx, |io| {
            for &id in &ids {
                p.take_ship(io, id);
            }
            p.take_fleet(io, f);
        });
        fx.galaxy.fleets.get_mut(&f).unwrap().flags |= 0x1000;
        ship_mut(&mut fx, f, 0).flags |= 0x1000;
        ship_mut(&mut fx, f, 2).flags |= 0x80_0000;
        let order = with(&mut fx, |io| p.split_damaged(io));
        assert_eq!(
            order,
            Some(AIAction::CreateFleet {
                fleet: f,
                ships: vec![0, 2],
                to: fx.systems[0]
            })
        );
        assert_eq!(p.ships, vec![ids[1]]);
        // A fleet with a role (0x10000000) keeps them.
        let mut q = planner(&fx);
        with(&mut fx, |io| q.take_fleet(io, f));
        fx.galaxy.fleets.get_mut(&f).unwrap().flags |= 0x1000_0000;
        assert_eq!(with(&mut fx, |io| q.split_damaged(io)), None);
    }

    #[test]
    fn the_counts_sum_the_held_fleets_and_less_what_the_plan_wants() {
        // FUN_00477cb0: +0xb4, +0xb8, +0x84, +0x78, +0x68, +0x7c, +0x70,
        // +0x9c into the counts, 0x10 and 0x20 counted, the positive ones'
        // bits; then FUN_004adaf0 less kind 3's wants (FUN_004b4920: 1.2
        // times the sector's +0x190, +0x194 and +0x198 + +0x138, one ship
        // with lasers, the regiments held).
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 1);
        {
            let r = fx.galaxy.fleets.get_mut(&f).unwrap();
            for (at, v) in [
                (25, 50),
                (26, 9),
                (13, 2),
                (10, 4),
                (6, 1),
                (11, 3),
                (8, 6),
                (19, 1),
            ] {
                r.values[at] = v;
            }
            r.flags |= 0x10;
        }
        {
            let s = &mut fx.galaxy.sectors[0];
            s.values[sector_at(0x190)] = 10;
            s.values[sector_at(0x194)] = 20;
            s.values[sector_at(0x198)] = 3;
            s.values[sector_at(0x138)] = 2;
        }
        let mut p = planner(&fx);
        p.wanted[0] = 40;
        with(&mut fx, |io| p.take_fleet(io, f));
        p.state = 2;
        assert_eq!(with(&mut fx, |io| p.step(io)), PlannerStep::Pending);
        assert_eq!(p.held, [2, 4]);
        assert_eq!(p.wanted, [40, 24, 0, 6, 0, 0, 0, 0, 4, 1]);
        assert_eq!(p.counts, [10, -15, 2, -2, 1, 1, 0, 3, 2, 0]);
        assert_eq!(p.requests, 0x1fe0_0000 & !0x10_0000);
    }

    #[test]
    fn a_short_count_takes_free_ships_of_the_nearest_fleet() {
        // FUN_004782a0: the own free fleets (0x8005, not en route) nearest
        // the sector; their free ships fit to sail that meet a short count
        // are taken and leave for the planner's system (FUN_00479220) by
        // Create Fleet; FUN_00477e80 marks the short counts.
        let mut fx = fixture();
        let far = fleet(&mut fx, 2, 1);
        let near = fleet(&mut fx, 1, 2);
        let mut p = planner(&fx);
        for f in [far, near] {
            for i in 0..fx.galaxy.fleets[&f].ships.len() {
                ship_mut(&mut fx, f, i).flags |= 1;
                ship_mut(&mut fx, f, i).values[14] = 2;
            }
        }
        // The near fleet's first ship is held already (no 0x20000).
        ship_mut(&mut fx, near, 0).flags &= !SHIP_FREE;
        p.counts[0] = -1;
        let order = with(&mut fx, |io| p.balance(io));
        assert_eq!(
            order,
            Some(AIAction::CreateFleet {
                fleet: near,
                ships: vec![1],
                to: fx.systems[1]
            })
        );
        assert_eq!(p.ships, vec![ship_id(&fx, near, 1)]);
        assert_eq!(p.requests & 1, 0);
        assert_eq!(fx.galaxy.fleets[&near].ships[0].owner, 0);
    }

    #[test]
    fn with_no_free_ship_a_later_plan_s_ships_are_taken() {
        // FUN_00478690: the fleets held for a later plan (+0x34 above
        // this one's), the latest first; a flagship is only marked.
        let mut fx = fixture();
        let f = fleet(&mut fx, 1, 2);
        let mut later = SectorPlanner {
            id: 2,
            plan_id: 5,
            ..planner(&fx)
        };
        let (s0, s1) = (ship_id(&fx, f, 0), ship_id(&fx, f, 1));
        with(&mut fx, |io| {
            later.take_ship(io, s0);
            later.take_ship(io, s1);
            later.take_fleet(io, f);
        });
        ship_mut(&mut fx, f, 0).flags |= 1 | SHIP_FLAGSHIP;
        ship_mut(&mut fx, f, 1).flags |= 1;
        let mut p = planner(&fx);
        p.counts[0] = -2;
        let order = with(&mut fx, |io| p.balance(io));
        assert_eq!(
            order,
            Some(AIAction::CreateFleet {
                fleet: f,
                ships: vec![1],
                to: fx.systems[1]
            })
        );
        assert_ne!(fx.galaxy.fleets[&f].ships[0].flags & SHIP_PASSED, 0);
        assert_eq!(fx.galaxy.fleets[&f].ships[1].owner, 1);
        // An earlier plan's fleet is left alone.
        let mut first = SectorPlanner {
            plan_id: 9,
            ..planner(&fx)
        };
        first.counts[0] = -1;
        let mut earlier = SectorPlanner {
            id: 3,
            plan_id: 2,
            ..planner(&fx)
        };
        let g = fleet(&mut fx, 1, 1);
        let g0 = ship_id(&fx, g, 0);
        with(&mut fx, |io| {
            earlier.take_ship(io, g0);
            earlier.take_fleet(io, g);
        });
        ship_mut(&mut fx, g, 0).flags |= 1;
        assert_eq!(with(&mut fx, |io| first.steal(io)), None);
    }

    #[test]
    fn a_spare_ship_is_given_back_one_at_a_time() {
        // FUN_00478aa0: the first held ship whose values the spare counts
        // cover leaves by Create Fleet; a flagship of several is marked;
        // a ship whose fleet has a role (FUN_00484510) stays.
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 3);
        let ids: Vec<u64> = (0..3).map(|i| ship_id(&fx, f, i)).collect();
        let mut p = planner(&fx);
        with(&mut fx, |io| {
            for &id in &ids {
                p.take_ship(io, id);
            }
        });
        for i in 0..3 {
            ship_mut(&mut fx, f, i).values[14] = 5;
        }
        p.flagship = Some(ids[0]);
        p.counts = [5, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let order = with(&mut fx, |io| p.balance(io));
        assert_eq!(
            order,
            Some(AIAction::CreateFleet {
                fleet: f,
                ships: vec![1],
                to: fx.systems[0]
            })
        );
        assert_ne!(fx.galaxy.fleets[&f].ships[0].flags & SHIP_PASSED, 0);
        assert_eq!(p.ships, vec![ids[0], ids[2]]);
        // Short of the ship's values: none leaves.
        p.counts = [4, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(with(&mut fx, |io| p.spare(io)), None);
        p.counts = [5, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        fx.galaxy.fleets.get_mut(&f).unwrap().flags |= 0x2000_0000;
        assert_eq!(with(&mut fx, |io| p.spare(io)), None);
    }

    #[test]
    fn create_fleet_goes_to_the_base_else_the_sector_s_systems() {
        // FUN_00479220: the base unless blockaded; else the sector's
        // headquarters; else its own system nearest the origin; else an
        // own system of the nearest other sector.
        let mut fx = fixture();
        let mut p = planner(&fx);
        p.base = Some(fx.systems[1]);
        let to = |fx: &mut Fx, p: &SectorPlanner, origin| with(fx, |io| p.destination(io, origin));
        let (s0, s1) = (fx.systems[0], fx.systems[1]);
        assert_eq!(to(&mut fx, &p, Some(s0)), Some(s1));
        fx.galaxy.system_mut(fx.systems[1]).unwrap().state |= 2;
        assert_eq!(to(&mut fx, &p, Some(s0)), Some(s0));
        fx.galaxy.system_mut(fx.systems[1]).unwrap().state |= 0x8000_0000;
        fx.galaxy.system_mut(fx.systems[1]).unwrap().state &= !2;
        p.base = None;
        assert_eq!(to(&mut fx, &p, Some(s0)), Some(s1));
        // No own system in the sector: the nearest other sector's.
        for i in 0..2 {
            fx.galaxy.system_mut(fx.systems[i]).unwrap().kind &= !1;
        }
        let other = to(&mut fx, &p, Some(s0));
        assert!(other == Some(fx.systems[2]) || other == Some(fx.systems[3]));
    }

    #[test]
    fn the_plan_asks_for_recon_or_espionage_and_movement() {
        // Kind 3's slot +0x18 (FUN_004b7150) with no mission planners:
        // Recon 0x54 with a system not the side's (+0x2c 0x18), else
        // Espionage 0x52; slot +0x1c (FUN_004b1800): 3 with a fleet held,
        // else 5 with regiments held in a sector with 0x40008.
        let mut fx = fixture();
        let mut p = planner(&fx);
        with(&mut fx, |io| {
            p.missions(io);
            p.movement(io);
        });
        assert_eq!((p.mission[0], p.movement[0], p.requests), (0x52, 0, 0x100));
        fx.galaxy.sectors[0].summary |= 0x10;
        p.held[1] = 2;
        fx.galaxy.sectors[0].summary |= 8;
        with(&mut fx, |io| {
            p.missions(io);
            p.movement(io);
        });
        assert_eq!((p.mission[0], p.movement[0]), (0x54, 5));
        let f = fleet(&mut fx, 0, 1);
        with(&mut fx, |io| p.take_fleet(io, f));
        with(&mut fx, |io| p.movement(io));
        assert_eq!(p.movement[0], 3);
        assert_eq!(p.requests & 0x300, 0x300);
    }

    #[test]
    fn the_report_reads_what_was_asked_and_held() {
        // FUN_00479a00: nothing short (0x4ff clear; the mission and
        // movement requests 0x100, 0x200 are outside it): 0x1c0000; short,
        // nothing held (0x9fe00000): 0x10000; held: 0x100000, and 0x80000
        // when none of 0x1f is short.
        let fx = fixture();
        let flags = |requests| {
            let mut p = SectorPlanner {
                kind: 6,
                requests,
                ..planner(&fx)
            };
            let mut fx = fixture();
            with(&mut fx, |io| p.report(io));
            p.flags
        };
        assert_eq!(flags(0), 0x1c_0000);
        assert_eq!(flags(0x300), 0x1c_0000);
        assert_eq!(flags(1), 0x1_0000);
        assert_eq!(flags(0x400 | 0x20_0000), 0x18_0000);
        assert_eq!(flags(0x80 | 0x8000_0000), 0x18_0000);
        assert_eq!(flags(0x2 | 0x20_0000), 0x10_0000);
    }

    #[test]
    fn the_flagship_s_mark_goes_when_no_movement_planner_holds_ships() {
        // FUN_00479560 with +0x60 0x2000000 clear: the flagship this
        // planner holds loses 0x1000000.
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 1);
        let id = ship_id(&fx, f, 0);
        let mut p = planner(&fx);
        with(&mut fx, |io| p.take_ship(io, id));
        ship_mut(&mut fx, f, 0).flags |= SHIP_FLAGSHIP;
        p.flagship = Some(id);
        with(&mut fx, |io| p.flagship(io));
        assert_eq!(p.flagship, None);
        assert_eq!(fx.galaxy.fleets[&f].ships[0].flags & SHIP_FLAGSHIP, 0);
    }

    #[test]
    fn a_reset_gives_back_every_ship_and_fleet() {
        // FUN_00477190 -> FUN_004790d0, FUN_004791d0 over the lists.
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 1);
        let id = ship_id(&fx, f, 0);
        let mut p = planner(&fx);
        with(&mut fx, |io| {
            p.take_ship(io, id);
            p.take_fleet(io, f);
            p.release_all(io);
        });
        assert!(p.ships.is_empty() && p.fleets.is_empty());
        assert_eq!(fx.galaxy.fleets[&f].owner, 0);
        assert_eq!(fx.galaxy.fleets[&f].ships[0].owner, 0);
    }

    /// A ship record with `flags` and values `(i, v)`.
    fn record(flags: u32, values: &[(usize, i32)]) -> ShipRecord {
        let mut v = [0; 30];
        for &(i, x) in values {
            v[i] = x;
        }
        ShipRecord {
            index: 0,
            id: 1,
            fleet: FleetKey::default(),
            flags,
            owner: 0,
            plan: 0,
            values: v,
        }
    }

    #[test]
    fn each_short_count_is_met_by_its_own_kind_of_ship() {
        // FUN_004782a0 / FUN_00478690: count 0 by bit 1, 1 by 0x102, 9 by
        // 2, 2 by 0xc, 3 by +0x60 > 0, 5 by +0x48 > 0, 4 by 0x80, 6 by
        // 0x400; only the first short count not yet met is tried.
        let fx = fixture();
        let cases: [(usize, ShipRecord); 9] = [
            (0, record(1, &[])),
            (1, record(0x100, &[])),
            (1, record(2, &[])),
            (9, record(2, &[])),
            (2, record(8, &[])),
            (3, record(0, &[(7, 1)])),
            (5, record(0, &[(1, 1)])),
            (4, record(0x80, &[])),
            (6, record(0x400, &[])),
        ];
        for (count, ship) in cases {
            let mut p = planner(&fx);
            p.counts[count] = -1;
            assert_eq!(p.meets(&ship), (true, false), "count {count}");
            let bare = record(0, &[(7, 0), (1, -1)]);
            assert_eq!(p.meets(&bare), (false, false), "count {count}");
            p.counts[count] = 0;
            assert_eq!(p.meets(&ship), (false, true), "count {count}");
        }
        // A ship meeting a later short count is taken too.
        let mut p = planner(&fx);
        p.counts[0] = -1;
        p.counts[6] = -1;
        assert_eq!(p.meets(&record(0x400, &[])), (true, false));
    }

    #[test]
    fn a_ship_is_spare_only_when_every_count_covers_it() {
        // FUN_00478aa0: +0x7c <= count 0, +0x80 <= 1, bit 2 needs count
        // 9 >= 1, 0xc needs +0x6c <= 2, +0x60 (when positive) <= 3, +0x48
        // (when positive) <= 5, 0x80 needs count 4 >= 1, 0x400 count 6 >= 1.
        type Case<'a> = (u32, &'a [(usize, i32)], usize, i32);
        let cases: [Case<'_>; 8] = [
            (0, &[(14, 3)], 0, 3),
            (0, &[(15, 3)], 1, 3),
            (2, &[], 9, 1),
            (8, &[(10, 3)], 2, 3),
            (0, &[(7, 3)], 3, 3),
            (0, &[(1, 3)], 5, 3),
            (0x80, &[], 4, 1),
            (0x400, &[], 6, 1),
        ];
        for (flags, values, count, enough) in cases {
            let mut fx = fixture();
            let f = fleet(&mut fx, 0, 1);
            let id = ship_id(&fx, f, 0);
            let mut p = planner(&fx);
            with(&mut fx, |io| p.take_ship(io, id));
            {
                let r = ship_mut(&mut fx, f, 0);
                r.flags |= flags;
                for &(i, v) in values {
                    r.values[i] = v;
                }
            }
            p.counts = [0; 10];
            p.counts[count] = enough - 1;
            assert_eq!(with(&mut fx, |io| p.spare(io)), None, "count {count}");
            p.counts[count] = enough;
            assert!(with(&mut fx, |io| p.spare(io)).is_some(), "count {count}");
        }
    }

    #[test]
    fn a_lone_flagship_is_spare_and_a_ship_with_nothing_is_too() {
        // FUN_00478aa0: the flagship is kept only among several ships; a
        // ship of no values is covered by any counts.
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 1);
        let id = ship_id(&fx, f, 0);
        let mut p = planner(&fx);
        with(&mut fx, |io| p.take_ship(io, id));
        p.flagship = Some(id);
        assert!(with(&mut fx, |io| p.spare(io)).is_some());
    }

    #[test]
    fn a_count_at_zero_is_neither_short_nor_spare() {
        // FUN_00477e80: short below 0, spare above 0; a short count met
        // by the first fleet asks no later plan's ships; the marks are
        // for the counts short after the takes.
        let mut fx = fixture();
        let near = fleet(&mut fx, 1, 1);
        let later = fleet(&mut fx, 1, 1);
        ship_mut(&mut fx, near, 0).flags |= 1;
        ship_mut(&mut fx, near, 0).values[14] = 1;
        let mut other = SectorPlanner {
            id: 2,
            plan_id: 5,
            ..planner(&fx)
        };
        let l0 = ship_id(&fx, later, 0);
        with(&mut fx, |io| {
            other.take_ship(io, l0);
            other.take_fleet(io, later);
        });
        ship_mut(&mut fx, later, 0).flags |= 1;
        let mut p = planner(&fx);
        assert_eq!(with(&mut fx, |io| p.balance(io)), None);
        assert_eq!(p.requests, 0);
        p.counts[0] = -1;
        assert!(with(&mut fx, |io| p.balance(io)).is_some());
        assert_eq!(fx.galaxy.fleets[&later].ships[0].owner, 2);
        assert_eq!(p.counts[0], 0);
        assert_eq!(p.requests & 1, 0);
        p.counts[1] = -1;
        p.counts[7] = -1;
        p.counts[8] = -1;
        p.counts[6] = -1;
        with(&mut fx, |io| p.balance(io));
        assert_eq!(p.requests & 0x862, 0x862);
    }

    #[test]
    fn a_plan_s_own_fleets_and_earlier_ones_are_not_taken_from() {
        // FUN_0041ab40 asks +0x34 above this plan's id: an equal id is
        // not taken from.
        let mut fx = fixture();
        let f = fleet(&mut fx, 1, 1);
        let mut same = SectorPlanner {
            id: 2,
            plan_id: 1,
            ..planner(&fx)
        };
        let id = ship_id(&fx, f, 0);
        with(&mut fx, |io| {
            same.take_ship(io, id);
            same.take_fleet(io, f);
        });
        ship_mut(&mut fx, f, 0).flags |= 1;
        let mut p = planner(&fx);
        p.counts[0] = -1;
        assert_eq!(with(&mut fx, |io| p.steal(io)), None);
    }

    #[test]
    fn a_held_ship_s_values_and_bits_count_only_when_present() {
        // FUN_00478e00: a value of 0 adds no +0x64 bit; a ship without
        // 0x80, 0x400, 2 adds none of those counts. FUN_004790d0 clears
        // 0x3000000 when it frees the ship.
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 1);
        let id = ship_id(&fx, f, 0);
        let mut p = planner(&fx);
        with(&mut fx, |io| p.take_ship(io, id));
        assert_eq!((p.counts, p.requests), ([0; 10], 0));
        ship_mut(&mut fx, f, 0).flags |= SHIP_FLAGSHIP | SHIP_PASSED | 0x10;
        with(&mut fx, |io| p.release_ship(io, id));
        let r = fx.galaxy.fleets[&f].ships[0].flags;
        assert_eq!(
            r & (SHIP_FLAGSHIP | SHIP_PASSED | 0x10 | SHIP_FREE),
            0x10 | SHIP_FREE
        );
        assert_eq!(p.counts, [0; 10]);
    }

    #[test]
    fn the_lists_drop_what_is_not_the_side_s_or_not_free_to_take() {
        // FUN_00477810: a ship without 0x4000 is given back; a held ship's
        // fleet is taken only while free (bit 4). FUN_00477950: a fleet
        // without 1 (the side's) is given back.
        let mut fx = fixture();
        let a = fleet(&mut fx, 0, 1);
        let b = fleet(&mut fx, 0, 1);
        let (a0, b0) = (ship_id(&fx, a, 0), ship_id(&fx, b, 0));
        let mut p = planner(&fx);
        let mut other = SectorPlanner {
            id: 9,
            ..planner(&fx)
        };
        with(&mut fx, |io| {
            p.take_ship(io, a0);
            p.take_ship(io, b0);
            other.take_fleet(io, b);
        });
        ship_mut(&mut fx, a, 0).flags &= !SHIP_OWN;
        with(&mut fx, |io| p.sync_ships(io));
        assert_eq!(p.ships, vec![b0]);
        assert!(p.fleets.is_empty());
        let mut q = planner(&fx);
        with(&mut fx, |io| q.take_fleet(io, a));
        fx.galaxy.fleets.get_mut(&a).unwrap().flags &= !FLEET_OWN;
        with(&mut fx, |io| q.sync_fleets(io));
        assert!(q.fleets.is_empty());
    }

    #[test]
    fn an_en_route_damaged_ship_stays_and_a_carrier_fleet_counts() {
        // FUN_00477a40 leaves a damaged ship en route (0x800);
        // FUN_00477cb0 counts a fleet with 0x20 in count 6 and sets
        // 0x100000.
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 1);
        let id = ship_id(&fx, f, 0);
        let mut p = planner(&fx);
        with(&mut fx, |io| {
            p.take_ship(io, id);
            p.take_fleet(io, f);
        });
        fx.galaxy.fleets.get_mut(&f).unwrap().flags |= 0x1000 | 0x20;
        ship_mut(&mut fx, f, 0).flags |= 0x1000 | 0x800;
        assert_eq!(with(&mut fx, |io| p.split_damaged(io)), None);
        with(&mut fx, |io| p.sum_fleets(io));
        assert_eq!((p.counts[6], p.counts[4]), (1, 0));
        assert_ne!(p.requests & 0x10_0000, 0);
    }

    #[test]
    fn kind_three_wants_more_than_its_sector_s_threat() {
        // FUN_004b4920: 1.2 times +0x190 (as an integer), and of +0x198 +
        // +0x138.
        let mut fx = fixture();
        {
            let s = &mut fx.galaxy.sectors[0];
            s.values[sector_at(0x190)] = 15;
            s.values[sector_at(0x198)] = 7;
            s.values[sector_at(0x138)] = 8;
        }
        let mut p = planner(&fx);
        with(&mut fx, |io| p.want(io));
        assert_eq!((p.wanted[0], p.wanted[3]), (18, 18));
        let mut other = SectorPlanner {
            kind: 6,
            ..planner(&fx)
        };
        with(&mut fx, |io| other.want(io));
        assert_eq!(other.wanted, [0; 10]);
    }

    #[test]
    fn kind_one_wants_kind_three_s_navy_without_its_floor_and_a_tenth_more_troops() {
        // FUN_004bb510: +0x34 1.2 x +0x190 with no controller floor (a
        // negative threat stays negative), +0x38 and +0x40 as kind 3's,
        // +0x48 1.1 x +0x144 (floor controller +0x174, read as 0); kind 3
        // (FUN_004b4920) leaves +0x48.
        let mut fx = fixture();
        {
            let s = &mut fx.galaxy.sectors[0];
            s.values[sector_at(0x190)] = -10;
            s.values[sector_at(0x194)] = 5;
            s.values[sector_at(0x198)] = 7;
            s.values[sector_at(0x138)] = 8;
            s.values[sector_at(0x144)] = 20;
        }
        let mut one = SectorPlanner {
            kind: 1,
            wanted: [-20; 10],
            ..planner(&fx)
        };
        with(&mut fx, |io| one.want(io));
        assert_eq!(
            (one.wanted[0], one.wanted[1], one.wanted[3], one.wanted[5]),
            (-12, 6, 18, 22)
        );
        let mut three = SectorPlanner {
            wanted: [-20; 10],
            ..planner(&fx)
        };
        with(&mut fx, |io| three.want(io));
        assert_eq!((three.wanted[0], three.wanted[5]), (0, -20));
        // +0x48's floor is 0 (controller +0x174): a negative +0x144 gives 0.
        fx.galaxy.sectors[0].values[sector_at(0x144)] = -20;
        with(&mut fx, |io| one.want(io));
        assert_eq!(one.wanted[5], 22);
        let mut low = SectorPlanner {
            kind: 1,
            wanted: [-20; 10],
            ..planner(&fx)
        };
        with(&mut fx, |io| low.want(io));
        assert_eq!(low.wanted[5], 0);
    }

    #[test]
    fn kind_two_wants_half_again_its_sector_s_threat() {
        // FUN_004b9fe0: +0x34 3/2 x +0x190, +0x38 3/2 x +0x13c + 3/2 x
        // +0x194, +0x40 3/2 x (+0x138 + +0x198), each at least the
        // controller's +0x164, +0x168, +0x16c (read as 0).
        let mut fx = fixture();
        {
            let s = &mut fx.galaxy.sectors[0];
            s.values[sector_at(0x190)] = 11;
            s.values[sector_at(0x13c)] = 5;
            s.values[sector_at(0x194)] = 5;
            s.values[sector_at(0x138)] = 4;
            s.values[sector_at(0x198)] = 7;
            s.values[sector_at(0x144)] = 20;
        }
        let mut two = SectorPlanner {
            kind: 2,
            wanted: [-20; 10],
            ..planner(&fx)
        };
        with(&mut fx, |io| two.want(io));
        // 5 * 3 / 2 + 5 * 3 / 2 = 7 + 7; (4 + 7) * 3 / 2 = 16.
        assert_eq!(
            (two.wanted[0], two.wanted[1], two.wanted[3], two.wanted[5]),
            (16, 14, 16, -20)
        );
        for at in [0x190, 0x13c, 0x194, 0x138, 0x198] {
            fx.galaxy.sectors[0].values[sector_at(at)] = -9;
        }
        let mut low = SectorPlanner {
            kind: 2,
            wanted: [-20; 10],
            ..planner(&fx)
        };
        with(&mut fx, |io| low.want(io));
        assert_eq!((low.wanted[0], low.wanted[1], low.wanted[3]), (0, 0, 0));
    }

    #[test]
    fn kind_two_reports_a_system_left_to_visit_else_marks_its_sector_swept() {
        // FUN_004b9310: 0x20000 with the sector's +0x2c 0x40000; else over
        // FUN_0041a3a0(+0x30 0x20 forbidding 0x700201) a system not on the
        // visited list (+0x5c) gives 0x20000, and with none the sector
        // record takes 0x20000000.
        let report = |fx: &mut Fx, visited: Vec<SystemKey>| {
            let mut p = SectorPlanner {
                kind: 2,
                visited,
                ..planner(fx)
            };
            with(fx, |io| p.report(io));
            p.flags & 0x2_0000
        };
        let mut fx = fixture();
        let enemy = fx.systems[1];
        fx.galaxy.update_kind(enemy, 0x24, 1);
        fx.galaxy.sectors[0].summary &= !0x4_0000;
        assert_eq!(report(&mut fx, vec![]), 0x2_0000);
        assert_eq!(fx.swept, []);
        assert_eq!(report(&mut fx, vec![enemy]), 0);
        assert_eq!(fx.swept, vec![fx.sectors[0]]);
        assert_eq!(fx.galaxy.sectors[0].summary & 0x2000_0000, 0x2000_0000);
        // Once only on the list.
        assert_eq!(report(&mut fx, vec![enemy]), 0);
        assert_eq!(fx.swept, vec![fx.sectors[0]]);
        // An enemy fleet or a role there leaves nothing to visit.
        let mut fx = fixture();
        fx.galaxy.update_kind(enemy, 0x224, 1);
        assert_eq!(report(&mut fx, vec![]), 0);
        assert_eq!(fx.swept, vec![fx.sectors[0]]);
        // 0x40000 gives 0x20000 without the list.
        let mut fx = fixture();
        fx.galaxy.sectors[0].summary |= 0x4_0000;
        assert_eq!(report(&mut fx, vec![]), 0x2_0000);
        assert_eq!(fx.swept, []);
    }

    /// Make `fx.systems[at]` the opponent's (`+0x30` `0x24`) with values.
    fn opponent(fx: &mut Fx, at: usize, values: &[(usize, i32)]) {
        let r = fx.galaxy.system_mut(fx.systems[at]).unwrap();
        r.kind = (r.kind & !1) | 0x24;
        for &(i, v) in values {
            r.values[i] = v;
        }
    }

    #[test]
    fn kind_four_targets_the_first_system_of_the_opponent_with_its_yard_kind() {
        // FUN_004b8f80: the sector's plan parameter (+0x34) names the yard
        // key (1 v[4], 2 v[5], 3 v[6]); the picks of the systems with +0x30
        // 0x20 without 0x101, by that key and by v[0x14] (order 2), are
        // tried in turn; the first with the key above 0 becomes +0x5c, each
        // cycle anew.
        let mut fx = fixture();
        let mut p = SectorPlanner {
            kind: 4,
            ..planner(&fx)
        };
        fx.galaxy.sectors[0].plan_param = 1;
        opponent(&mut fx, 0, &[(4, 0), (0x14, 9)]);
        opponent(&mut fx, 1, &[(4, 2), (0x14, 1)]);
        with(&mut fx, |io| p.choose_target(io));
        assert_eq!(p.plan_target, Some(fx.systems[1]));
        // Training facilities (v[5]): none has one.
        fx.galaxy.sectors[0].plan_param = 2;
        with(&mut fx, |io| p.choose_target(io));
        assert_eq!(p.plan_target, None);
        // Construction yards (v[6]) at both: the merged lists' best pick
        // (system 0 first by v[0x14] and equal by v[6]: the coin keeps it).
        fx.galaxy.sectors[0].plan_param = 3;
        opponent(&mut fx, 0, &[(6, 0), (0, 1)]);
        opponent(&mut fx, 1, &[(6, 1), (0, 0)]);
        with(&mut fx, |io| p.choose_target(io));
        assert_eq!(p.plan_target, Some(fx.systems[1]));
        // Equal keys go by the coin (ahead on a draw below 5).
        opponent(&mut fx, 0, &[(6, 1), (0x14, 1)]);
        opponent(&mut fx, 1, &[(6, 1), (0x14, 1)]);
        let mut chosen = Vec::new();
        for draw in [4, 5] {
            fx.draws = vec![draw, draw];
            with(&mut fx, |io| p.choose_target(io));
            chosen.push(p.plan_target);
        }
        assert_eq!(chosen, [Some(fx.systems[0]), Some(fx.systems[1])]);
        // The side's own system (1) is not a target.
        let r = fx.galaxy.system_mut(fx.systems[1]).unwrap();
        r.kind |= 1;
        let r = fx.galaxy.system_mut(fx.systems[0]).unwrap();
        r.kind |= 1;
        with(&mut fx, |io| p.choose_target(io));
        assert_eq!(p.plan_target, None);
        // Without the sector the target stays.
        p.plan_target = Some(fx.systems[0]);
        p.sector = None;
        with(&mut fx, |io| p.choose_target(io));
        assert_eq!(p.plan_target, Some(fx.systems[0]));
    }

    #[test]
    fn kind_four_wants_half_again_the_threat_and_the_target_s_shields_and_garrison() {
        // FUN_004b8b70: +0x34 3/2 x +0x190; +0x38 3/2 x +0x194 plus 3/2 x
        // the target's v[27]; +0x40 11/10 x (+0x138 + +0x198); +0x3c the
        // target's v[52]; +0x44 1 with controller +4 bit 4 and +0x19c above
        // 0; +0x50, +0x54 the planner's +0x94, +0x98; +0x58 1.
        let mut fx = fixture();
        {
            let s = &mut fx.galaxy.sectors[0];
            s.plan_param = 1;
            s.values[sector_at(0x190)] = 10;
            s.values[sector_at(0x194)] = 5;
            s.values[sector_at(0x138)] = 8;
            s.values[sector_at(0x198)] = 12;
            s.values[sector_at(0x19c)] = 1;
        }
        opponent(&mut fx, 1, &[(4, 1), (27, 4), (52, 7)]);
        let mut p = SectorPlanner {
            kind: 4,
            held: [2, 3],
            wanted: [-1; 10],
            ..planner(&fx)
        };
        with(&mut fx, |io| p.want(io));
        assert_eq!(p.plan_target, Some(fx.systems[1]));
        // 5 * 3 / 2 + 4 * 3 / 2 = 7 + 6; (8 + 12) * 11 / 10 = 22.
        assert_eq!(p.wanted, [15, 13, 7, 22, -1, -1, -1, 2, 3, 1]);
        let mut flagged = SectorPlanner {
            kind: 4,
            ..planner(&fx)
        };
        with(&mut fx, |io| {
            io.controller_flags = 0x100_0004;
            flagged.want(io);
        });
        assert_eq!((flagged.wanted[4], flagged.wanted[6]), (1, 1));
        // +0x19c at 0: no +0x44 even with bit 4.
        fx.galaxy.sectors[0].values[sector_at(0x19c)] = 0;
        let mut zero = SectorPlanner {
            kind: 4,
            ..planner(&fx)
        };
        with(&mut fx, |io| {
            io.controller_flags = 4;
            zero.want(io);
        });
        assert_eq!(zero.wanted[4], 0);
        // Without a target, +0x3c takes the +0x40 value.
        opponent(&mut fx, 1, &[(4, 0)]);
        let mut none = SectorPlanner {
            kind: 4,
            ..planner(&fx)
        };
        with(&mut fx, |io| none.want(io));
        assert_eq!(
            (none.plan_target, none.wanted[2], none.wanted[1]),
            (None, 22, 7)
        );
    }

    #[test]
    fn kind_four_asks_for_movement_four_and_reports_while_its_target_stands() {
        // FUN_004b72d0: movement 4 with regiments (+0x94) in a sector with
        // +0x2c 8, else kind 3's; FUN_004b70a0: 0x20000 while the target's
        // record lacks +0x30 1.
        let mut fx = fixture();
        let mut p = SectorPlanner {
            kind: 4,
            held: [1, 0],
            ..planner(&fx)
        };
        fx.galaxy.sectors[0].summary |= 8;
        with(&mut fx, |io| p.movement(io));
        assert_eq!(p.movement[0], 4);
        p.held[0] = 0;
        with(&mut fx, |io| p.movement(io));
        assert_eq!(p.movement[0], 0);
        p.held[0] = 1;
        fx.galaxy.sectors[0].summary &= !8;
        with(&mut fx, |io| p.movement(io));
        assert_eq!(p.movement[0], 0);
        p.fleets = vec![FleetKey::default()];
        with(&mut fx, |io| p.movement(io));
        assert_eq!(p.movement[0], 3);
        opponent(&mut fx, 1, &[]);
        p.plan_target = Some(fx.systems[1]);
        p.flags = 0;
        with(&mut fx, |io| p.report(io));
        assert_eq!(p.flags & 0x2_0000, 0x2_0000);
        fx.galaxy.system_mut(fx.systems[1]).unwrap().kind |= 1;
        p.flags = 0;
        with(&mut fx, |io| p.report(io));
        assert_eq!(p.flags & 0x2_0000, 0);
        p.plan_target = None;
        with(&mut fx, |io| p.report(io));
        assert_eq!(p.flags & 0x2_0000, 0);
    }

    #[test]
    fn kind_one_runs_its_own_cycle_with_kind_three_s_requests() {
        // The planner steps kinds 1 and 3 itself; kind 1 shares
        // FUN_004b7150 (missions) and FUN_004b9290 (movement, the code of
        // FUN_004b1800). Its report (FUN_004ba5e0) sets 0x20000 with the
        // sector's +0x2c 0x80000, not 4.
        let mut fx = fixture();
        let mut p = SectorPlanner {
            kind: 1,
            ..planner(&fx)
        };
        assert_eq!(with(&mut fx, |io| p.step(io)), PlannerStep::Pending);
        let mut three = planner(&fx);
        with(&mut fx, |io| {
            p.missions(io);
            p.movement(io);
            three.missions(io);
            three.movement(io);
        });
        assert_eq!((p.mission, p.movement), (three.mission, three.movement));
        assert_ne!(p.mission[0], 0);
        fx.galaxy.sectors[0].summary = (fx.galaxy.sectors[0].summary | 4) & !0x8_0000;
        p.flags = 0;
        with(&mut fx, |io| p.report(io));
        assert_eq!(p.flags & 0x2_0000, 0);
        assert_ne!(p.flags & REVIEWED, 0);
        fx.galaxy.sectors[0].summary |= 0x8_0000;
        p.flags = 0;
        with(&mut fx, |io| p.report(io));
        assert_eq!(p.flags & 0x2_0000, 0x2_0000);
        // Kind 6's slots are not ported: it hands over (kind 4 until stage
        // 5g and kind 5 until stage 6b of module 8).
        let mut six = SectorPlanner {
            kind: 6,
            ..planner(&fx)
        };
        six.resync = false;
        assert_eq!(with(&mut fx, |io| six.step(io)), PlannerStep::StandIn);
    }

    #[test]
    fn movement_keeps_the_other_flags_and_needs_regiments_and_the_sector() {
        // FUN_00478140 clears +0x60 0x7000000 only; kind 3's slot +0x1c
        // asks for 5 with regiments held (+0x98 > 0) and +0x2c 0x40008.
        let mut fx = fixture();
        let mut p = planner(&fx);
        p.flags = 0x700_0001;
        p.held[1] = 1;
        with(&mut fx, |io| p.movement(io));
        assert_eq!((p.flags, p.movement[0]), (1, 0));
        fx.galaxy.sectors[0].summary |= 0x4_0000;
        with(&mut fx, |io| p.movement(io));
        assert_eq!(p.movement[0], 5);
        p.held[1] = 0;
        with(&mut fx, |io| p.movement(io));
        assert_eq!(p.movement[0], 0);
    }

    #[test]
    fn the_report_s_own_system_bit_needs_one_in_the_sector() {
        // FUN_004b3830: 0x20000 only with +0x2c 4.
        let mut fx = fixture();
        fx.galaxy.sectors[0].summary &= !4;
        let mut p = planner(&fx);
        with(&mut fx, |io| p.report(io));
        assert_eq!(p.flags & 0x2_0000, 0);
        assert_ne!(p.flags & REVIEWED, 0);
    }

    #[test]
    fn ties_go_by_the_coin() {
        // FUN_0041c250: an equal key goes ahead on FUN_0041cd80(10) < 5;
        // a pick takes the list's last entry.
        // Two free fleets as near (FUN_004782a0).
        for (coin, second) in [(4, false), (5, true)] {
            let mut fx = fixture();
            let a = fleet(&mut fx, 1, 1);
            let b = fleet(&mut fx, 1, 1);
            for f in [a, b] {
                ship_mut(&mut fx, f, 0).flags |= 1;
                ship_mut(&mut fx, f, 0).values[14] = 1;
            }
            let mut p = planner(&fx);
            p.counts[0] = -1;
            fx.draws = vec![coin];
            let Some(AIAction::CreateFleet { fleet, .. }) = with(&mut fx, |io| p.acquire(io))
            else {
                panic!("no order");
            };
            assert_eq!(fleet, if second { b } else { a });
        }
        // Two fleets of the same later plan (FUN_0041ab40, order 1).
        for (coin, second) in [(4, false), (5, true)] {
            let mut fx = fixture();
            let a = fleet(&mut fx, 1, 1);
            let b = fleet(&mut fx, 1, 1);
            let mut other = SectorPlanner {
                id: 2,
                plan_id: 5,
                ..planner(&fx)
            };
            for f in [a, b] {
                let id = ship_id(&fx, f, 0);
                with(&mut fx, |io| {
                    other.take_ship(io, id);
                    other.take_fleet(io, f);
                });
                ship_mut(&mut fx, f, 0).flags |= 1;
            }
            let mut p = planner(&fx);
            p.counts[0] = -1;
            fx.draws = vec![coin];
            let Some(AIAction::CreateFleet { fleet, .. }) = with(&mut fx, |io| p.steal(io)) else {
                panic!("no order");
            };
            assert_eq!(fleet, if second { b } else { a });
        }
        // Two headquarters (FUN_00479220, key 7, order 2).
        for (coin, second) in [(4, false), (5, true)] {
            let mut fx = fixture();
            for i in 0..2 {
                fx.galaxy.system_mut(fx.systems[i]).unwrap().state |= 0x8000_0000;
            }
            let p = planner(&fx);
            fx.draws = vec![coin];
            let s0 = fx.systems[0];
            let s1 = fx.systems[1];
            let to = with(&mut fx, |io| p.destination(io, Some(s0)));
            assert_eq!(to, Some(if second { s1 } else { s0 }));
        }
    }

    #[test]
    fn the_nearest_other_sector_gives_a_system_by_a_random_key() {
        // FUN_0041b2b0 over FUN_0041a460: the nearer sector wins; its
        // systems by a draw below 10, order 1 (the greatest key picked).
        let mut fx = fixture();
        // A third, farther sector with its own system.
        let far = fx.world.sectors.insert(Sector {
            dat_id: DatId::new(0x8000_0001),
            name: "Sector".into(),
            group: crate::dat::SectorGroup::Core,
            x: 300,
            y: 0,
            systems: vec![],
        });
        let mut sys = fx.world.systems[fx.systems[2]].clone();
        sys.sector = far;
        sys.x = 300;
        let farther = fx.world.systems.insert(sys);
        rebuild(&mut fx);
        let own = [Mask::new(0, 2), ANY, Mask::new(1, 0)];
        fx.draws = vec![3, 8, 5];
        let from = fx.sectors[0];
        let got = with(&mut fx, |io| nearest_sector_system(io, from, own));
        assert_eq!(got, Some(fx.systems[3]));
        assert_ne!(got, Some(farther));
        // A blockaded system (+0x28 2) does not pass.
        fx.galaxy.system_mut(fx.systems[3]).unwrap().state |= 2;
        fx.draws = vec![3];
        let got = with(&mut fx, |io| nearest_sector_system(io, from, own));
        assert_eq!(got, Some(fx.systems[2]));
    }

    #[test]
    fn with_nothing_short_no_fleet_is_searched_and_none_given() {
        // FUN_00477e80: no count below 0 searches no fleet (no draws for
        // ties), and with none above 0 nothing is given back.
        let mut fx = fixture();
        let a = fleet(&mut fx, 1, 1);
        let b = fleet(&mut fx, 1, 1);
        let mut p = planner(&fx);
        let id = ship_id(&fx, a, 0);
        with(&mut fx, |io| p.take_ship(io, id));
        let _ = b;
        p.counts = [0; 10];
        fx.draws = vec![4, 4, 4];
        assert_eq!(with(&mut fx, |io| p.balance(io)), None);
        assert_eq!(fx.draws.len(), 3);
        assert_eq!(p.ships, vec![id]);
    }

    #[test]
    fn a_take_that_leaves_a_count_short_asks_no_later_plan() {
        // FUN_00477e80: an order from FUN_004782a0 ends the search, even
        // while a count is still short.
        let mut fx = fixture();
        let near = fleet(&mut fx, 1, 1);
        let later = fleet(&mut fx, 1, 1);
        let mut other = SectorPlanner {
            id: 2,
            plan_id: 5,
            ..planner(&fx)
        };
        let l0 = ship_id(&fx, later, 0);
        with(&mut fx, |io| {
            other.take_ship(io, l0);
            other.take_fleet(io, later);
        });
        for f in [near, later] {
            ship_mut(&mut fx, f, 0).flags |= 1;
            ship_mut(&mut fx, f, 0).values[14] = 1;
        }
        let mut p = planner(&fx);
        p.counts[0] = -5;
        assert!(with(&mut fx, |io| p.balance(io)).is_some());
        assert_eq!(p.counts[0], -4);
        assert_eq!(fx.galaxy.fleets[&later].ships[0].owner, 2);
    }

    #[test]
    fn spare_and_short_tests_read_only_what_the_ship_has() {
        // FUN_004782a0: +0x48 of 0 meets no short count 5. FUN_00478aa0:
        // a ship without 0xc is spare whatever its +0x6c.
        let fx = fixture();
        let mut p = planner(&fx);
        p.counts[5] = -1;
        assert!(!p.meets(&record(0, &[(1, 0)])).0);
        let mut fx = fixture();
        let f = fleet(&mut fx, 0, 1);
        let id = ship_id(&fx, f, 0);
        let mut p = planner(&fx);
        with(&mut fx, |io| p.take_ship(io, id));
        ship_mut(&mut fx, f, 0).values[10] = 9;
        p.counts = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(with(&mut fx, |io| p.spare(io)).is_some());
    }

    #[test]
    fn a_sector_s_random_system_must_pass_every_mask() {
        // FUN_00431f40: +0x28, +0x2c and +0x30 each pass; the key is a
        // draw below 10 per system, ties going ahead on a coin below 5.
        let mut fx = fixture();
        let sector = fx.sectors[1];
        let own = [Mask::new(0, 2), ANY, Mask::new(1, 0)];
        fx.galaxy.system_mut(fx.systems[2]).unwrap().state |= 2;
        fx.draws = vec![9, 0];
        let got = with(&mut fx, |io| random_system(io, sector, own));
        assert_eq!(got, Some(fx.systems[3]));
        fx.galaxy.system_mut(fx.systems[2]).unwrap().state &= !2;
        fx.galaxy.system_mut(fx.systems[2]).unwrap().summary |= 0x4000_0000;
        let picky = [Mask::new(0, 2), Mask::new(0, 0x4000_0000), Mask::new(1, 0)];
        fx.draws = vec![9, 0];
        let got = with(&mut fx, |io| random_system(io, sector, picky));
        assert_eq!(got, Some(fx.systems[3]));
        fx.galaxy.system_mut(fx.systems[2]).unwrap().summary &= !0x4000_0000;
        fx.galaxy.system_mut(fx.systems[2]).unwrap().kind &= !1;
        fx.draws = vec![9, 0];
        let got = with(&mut fx, |io| random_system(io, sector, own));
        assert_eq!(got, Some(fx.systems[3]));
        fx.galaxy.system_mut(fx.systems[2]).unwrap().kind |= 1;
        for (coin, second) in [(4, false), (5, true)] {
            fx.draws = vec![3, 3, coin];
            let got = with(&mut fx, |io| random_system(io, sector, own));
            assert_eq!(got, Some(fx.systems[if second { 3 } else { 2 }]));
        }
    }

    #[test]
    fn kind_four_asks_for_its_base_s_garrison_term_plus_the_target_s_need() {
        // FUN_00476b70 calls the plan object's slot +0x0c; kind 4's
        // (FUN_004b7100) gives base +0x98 - +0x88 + target +0x118, that is
        // v[20] - v[16] + v[52], and 0 without either record. Kinds 1 to 3
        // have 0x6158b0 there.
        let mut fx = fixture();
        let (base, target) = (fx.systems[0], fx.systems[2]);
        let r = fx.galaxy.system_mut(base).unwrap();
        (r.values[20], r.values[16]) = (7, 2);
        fx.galaxy.system_mut(target).unwrap().values[52] = 4;
        let mut p = planner(&fx);
        (p.kind, p.base, p.plan_target) = (4, Some(base), Some(target));
        assert_eq!(p.request(&fx.galaxy), 9);
        p.kind = 3;
        assert_eq!(p.request(&fx.galaxy), 0);
        p.kind = 4;
        p.plan_target = None;
        assert_eq!(p.request(&fx.galaxy), 0);
        (p.plan_target, p.base) = (Some(target), None);
        assert_eq!(p.request(&fx.galaxy), 0);
    }

    #[test]
    fn kind_five_asks_for_a_regiment_per_system_not_yet_the_side_s() {
        // FUN_004b17b0: with the sector record (+0x34) and the base's,
        // base +0x98 - +0x88 - sector +0x78 + 10; else 0.
        let mut fx = fixture();
        let base = fx.systems[0];
        let r = fx.galaxy.system_mut(base).unwrap();
        (r.values[20], r.values[16]) = (5, 1);
        fx.galaxy.sectors[0].values[sector_at(0x78)] = 7;
        let mut p = planner(&fx);
        (p.kind, p.base) = (5, Some(base));
        assert_eq!(p.request(&fx.galaxy), 7);
        p.sector = None;
        assert_eq!(p.request(&fx.galaxy), 0);
        p.sector = Some(fx.sectors[0]);
        p.base = None;
        assert_eq!(p.request(&fx.galaxy), 0);
    }

    #[test]
    fn kind_five_wants_half_again_its_sector_s_threat_and_one_regiment() {
        // FUN_004b3300: +0x34 1.5 x +0x190; +0x38 1.5 x +0x194 (or
        // controller +0x168, read as 0, with +0x2c 8); +0x40 1.2 x (+0x138
        // + +0x198); +0x3c at least 1; +0x44 1 with controller +4 bit 4 and
        // +0x19c above 0; +0x4c 1 with 0x1000000; +0x50, +0x54 the
        // planner's +0x94, +0x98; +0x58 1. Each never less than before.
        let mut fx = fixture();
        {
            let s = &mut fx.galaxy.sectors[0];
            s.values[sector_at(0x190)] = 10;
            s.values[sector_at(0x194)] = -4;
            s.values[sector_at(0x138)] = 3;
            s.values[sector_at(0x198)] = 7;
            s.values[sector_at(0x19c)] = 1;
            s.summary &= !8;
        }
        let mut p = SectorPlanner {
            kind: 5,
            held: [2, 3],
            ..planner(&fx)
        };
        p.wanted[1] = -9;
        with(&mut fx, |io| p.want(io));
        assert_eq!(p.wanted, [15, -6, 1, 12, 0, 0, 0, 2, 3, 1]);
        fx.galaxy.sectors[0].summary |= 8;
        p.wanted[1] = -9;
        with(&mut fx, |io| {
            io.controller_flags = 0x100_0004;
            p.want(io);
        });
        assert_eq!(p.wanted, [15, 0, 1, 12, 1, 0, 1, 2, 3, 1]);
        // +0x44 needs +0x19c above 0.
        fx.galaxy.sectors[0].values[sector_at(0x19c)] = 0;
        p.wanted[4] = 0;
        with(&mut fx, |io| {
            io.controller_flags = 4;
            p.want(io);
        });
        assert_eq!(p.wanted[4], 0);
        // Without the sector record nothing changes.
        let mut q = SectorPlanner {
            kind: 5,
            sector: None,
            ..planner(&fx)
        };
        with(&mut fx, |io| q.want(io));
        assert_eq!(q.wanted, [0; 10]);
    }

    #[test]
    fn kind_five_reports_work_while_its_sector_is_not_wholly_the_side_s() {
        // FUN_004b1760: 0x4000000, with the sector record 0x8000000, and
        // 0x20000 while +0x2c lacks 2 or has 8.
        let mut fx = fixture();
        let mut p = SectorPlanner {
            kind: 5,
            ..planner(&fx)
        };
        let mut report = |summary: u32| {
            let s = &mut fx.galaxy.sectors[0];
            s.summary = (s.summary & !0xa) | summary;
            p.flags = 0;
            with(&mut fx, |io| p.report(io));
            p.flags & (CYCLED | REVIEWED | 0x2_0000)
        };
        assert_eq!(report(2), CYCLED | REVIEWED);
        assert_eq!(report(0), CYCLED | REVIEWED | 0x2_0000);
        assert_eq!(report(0xa), CYCLED | REVIEWED | 0x2_0000);
    }
}
