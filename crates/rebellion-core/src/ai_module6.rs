//! Module 6 of the AI player (`FUN_004dd460`, step `FUN_004dd7a0`): it
//! balances the side's regiments. It moves regiments from a surplus to a
//! system short of garrison, trains regiments at its training facility
//! sites and sends them on, and builds training facilities
//! (`ghidra/notes/ai-production-modules.md`, "Module 6"). It shares
//! module 5's mark `0x400000` ([`crate::ai_module1::TRAINING`]).

use serde::{Deserialize, Serialize};

use crate::ai::{AIAction, MARK_TRAINING};
use crate::ai_module1::{
    adopt_or_request, best_class, build, claim, destination, destination_in, scrap_at,
    scrap_spare_yard, yard_class, ClassKey, Inputs, Step, Target, Yard, YardKind, ANY, TRAINING,
};
use crate::ai_module2::keep_site;
use crate::ai_module3::choose_target;
use crate::ai_records::{pick, Mask, Order, CLASS_DEFENSIVE, CLASS_OFFENSIVE};
use crate::ai_strategy::{SECURE, WEAK};
use crate::ids::{DatId, SystemKey, TroopKey};
use crate::manufacturing::ProductionArea;

/// `+0x38` while the yard cycle runs (`FUN_004dd7a0`).
pub const YARD_CYCLE: u32 = 0x3ec;

/// `+0x38` while the move cycle runs (`FUN_004dd7a0`).
pub const MOVE_CYCLE: u32 = 0x3ef;

/// `+0x38` while the regiment cycle runs (`FUN_004dd7a0`).
pub const REGIMENT_CYCLE: u32 = 0x3f0;

/// The yard token module 6 holds the construction yards with (controller
/// `+0xc`, `+8`; `FUN_004dda00` state 5).
pub const YARD_TOKEN: u32 = 0x20;

/// `+0x24`: the percent of the maintenance capacity module 6 budgets
/// (`FUN_004dd460`): 80% of it for regiments (`FUN_004e19c0`), 20% for
/// training facilities (`FUN_004e1a20`).
const PERCENT: i32 = 0x16;

/// Module 5's training facilities, but adopted with any number of sites
/// (`FUN_004e15b0` has no two-site limit).
const KIND: YardKind = YardKind {
    few_sites_only: false,
    ..TRAINING
};

/// Module 6's own fields (`FUN_004dd460`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module6 {
    /// `+0x38`: 0, [`MOVE_CYCLE`], [`REGIMENT_CYCLE`] or [`YARD_CYCLE`].
    pub phase: u32,
    /// `+0x3c`: the cycle's state.
    pub state: u32,
    /// `+0x50` in the move cycle: the system the regiments leave.
    pub source: Target,
    /// `+0x50` otherwise: a site, then its manager (troops or
    /// construction).
    pub yard: Yard,
    /// `+0x54`: the regiments' destination.
    pub destination: Target,
    /// `+0x58`: the yard cycle's target.
    pub target: Target,
    /// `+0x5c`: the class to build.
    pub class: Option<DatId>,
    /// `+0x60`: how many.
    pub count: i32,
    /// `+0x64`: the site's training facilities, or the borrowed yard's
    /// construction yards.
    pub capacity: i32,
    /// `+0x68`: the target's free room `v[0x33]`.
    pub room: i32,
    /// `+0x6c`: the regiment class flags.
    pub class_flags: u32,
    /// `+0x70`: the regiment class key.
    pub class_key: RegimentKey,
    /// `+0x74`: the regiment class order (1 the highest, 2 the lowest).
    pub class_order: u32,
    /// `+0x78`: the destination query to start from, 1 to 4.
    pub next_query: u32,
    /// `+0x28` bits 1, 2, 4 and 8: a destination query found no system.
    pub missed: u32,
    /// `+0x7c`: the sites, marked [`MARK_TRAINING`].
    pub sites: Vec<SystemKey>,
    /// `+0x84`: the claimed sites.
    pub claimed: Vec<SystemKey>,
}

/// The regiment class record value a key names (`FUN_00473490`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegimentKey {
    /// No key yet.
    #[default]
    None,
    /// Key `0x1000`: attack (`+0x30`).
    Attack,
    /// Key `0x2000`: defense (`+0x34`).
    Defense,
    /// Key 1: maintenance (`+0x20`).
    Maintenance,
}

impl Default for Module6 {
    /// `FUN_004dd460`: `+0x78 = 1`.
    fn default() -> Self {
        Self {
            phase: 0,
            state: 0,
            source: Target::None,
            yard: Yard::None,
            destination: Target::None,
            target: Target::None,
            class: None,
            count: 0,
            capacity: 0,
            room: 0,
            class_flags: 0,
            class_key: RegimentKey::None,
            class_order: 0,
            next_query: 1,
            missed: 0,
            sites: Vec::new(),
            claimed: Vec::new(),
        }
    }
}

/// The destination queries of `FUN_004df440`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Query {
    /// `FUN_004e1060`.
    Defended,
    /// `FUN_004e0ef0`.
    Marked,
    /// `FUN_004e1430`.
    Near,
    /// `FUN_004e12e0`.
    Home,
    /// `FUN_004e11a0`.
    Far,
}

impl Module6 {
    /// One step (`FUN_004dd7a0`). The scheduler steps only a ready module,
    /// so the not-ready branch never runs.
    pub fn step(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.phase {
            YARD_CYCLE => self.yard_cycle(io),
            MOVE_CYCLE => self.move_cycle(io),
            REGIMENT_CYCLE => self.regiment_cycle(io),
            _ => {
                self.phase = MOVE_CYCLE;
                self.state = 0;
                Step::pending()
            }
        }
    }

    /// A refused order (slot `+0x18`, `FUN_004dd810`).
    pub fn refused(&mut self) {
        self.phase = 0;
        self.state = 0;
    }

    fn end(&mut self, next: u32, order: Option<AIAction>) -> Step {
        self.state = 0;
        self.phase = next;
        Step {
            order,
            complete: true,
            raise: false,
        }
    }

    /// The move cycle (`FUN_004dd840`): four ways to move regiments, the
    /// first that gives an order ending the cycle.
    fn move_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        let order = match self.state {
            1 | 2 => self.planned_move(),
            3 => self.move_to_short(io, Mask::new(0x4_0010, 0), 0x2001, false),
            4 => self.move_to_short(io, Mask::new(0x4_0000, 0), 0x2000, true),
            _ => {
                self.state = 1;
                return Step::pending();
            }
        };
        if order.is_some() || self.state == 4 {
            return self.end(REGIMENT_CYCLE, order);
        }
        self.state += 1;
        Step::pending()
    }

    /// `FUN_004ddb70` and `FUN_004de130`: the count to move comes from the
    /// fleet planner module 8 keeps for the destination's sector
    /// (`FUN_0042f310`, `FUN_00476b70`).
    ///
    /// port: module 8 is not ported, so no planner asks and nothing moves.
    fn planned_move(&mut self) -> Option<AIAction> {
        self.source = Target::None;
        self.destination = Target::None;
        None
    }

    /// `FUN_004de6f0` (and `FUN_004deba0` with `defensive`): the system
    /// short of garrison in the sector with the most, then the nearest
    /// sector's system with a surplus, and up to the shortfall `v[21]` (at
    /// most the source's surplus `v[19]`) of its weakest regiments.
    fn move_to_short(
        &mut self,
        io: &mut Inputs<'_>,
        sector_bits: Mask,
        state_bits: u32,
        defensive: bool,
    ) -> Option<AIAction> {
        self.source = Target::None;
        let sector = pick(&[io.sectors([sector_bits, ANY, ANY], 0x1e, Order::Descending)]);
        let masks = [Mask::new(state_bits, 2), ANY, Mask::new(1, 0)];
        let destination = pick(&[io.systems(sector, masks, 0x15, Order::Descending)]);
        self.destination = destination.map_or(Target::None, Target::System);
        let destination = destination?;
        let mut count = io.galaxy.system(destination).map_or(0, |r| r.values[21]);
        let near = |io: &mut Inputs<'_>, bits: [Mask; 3]| {
            let sectors =
                io.galaxy
                    .sectors_by_distance(destination, bits, Order::Descending, io.coin);
            pick(&[sectors])
        };
        let source = if defensive {
            let sector = near(io, [Mask::new(0x8_0000, 0), ANY, Mask::new(0x1000, 0)]);
            let masks = [Mask::new(0x1_0000, 2), ANY, Mask::new(0x8001, 0x3_0000)];
            let first = pick(&[io.systems(sector, masks, 0x13, Order::Descending)]);
            first.or_else(|| {
                let sector = near(io, [Mask::new(0x28_0000, 0), ANY, ANY]);
                let masks = [Mask::new(0x2001_0000, 2), ANY, Mask::new(1, 0x3_0000)];
                pick(&[io.systems(sector, masks, 0x13, Order::Descending)])
            })
        } else {
            let sector = near(io, [Mask::new(0x8_0000, 0), ANY, ANY]);
            let masks = [Mask::new(0x1_0000, 2), ANY, Mask::new(1, 0x3_0000)];
            pick(&[io.systems(sector, masks, 0x13, Order::Descending)])
        };
        self.source = source.map_or(Target::None, Target::System);
        let source = source?;
        let surplus = io.galaxy.system(source)?.values[19];
        count = count.min(surplus);
        let mut team: Vec<TroopKey> = Vec::new();
        let candidates = Self::regiments(io, source, defensive);
        while count > 0 {
            let Some(&(troop, _)) = candidates
                .iter()
                .filter(|(t, _)| !team.contains(t))
                .min_by_key(|&&(_, defense)| defense)
            else {
                break;
            };
            team.push(troop);
            count -= 1;
        }
        (!team.is_empty()).then_some(AIAction::MoveRegiments {
            troops: team,
            to: destination,
        })
    }

    /// The side's regiments on `system`'s surface (`FUN_00504c40`) with
    /// their slot `+0x1e0`, the class's defense (`FUN_00520ba0`, class
    /// `+0x68`), in the system's order; only defensive classes (record
    /// flag `0x2000`) when `defensive`. The first of the lowest wins.
    fn regiments(io: &Inputs<'_>, system: SystemKey, defensive: bool) -> Vec<(TroopKey, i32)> {
        let side = io.side();
        let Some(sys) = io.world.systems.get(system) else {
            return Vec::new();
        };
        sys.ground_units
            .iter()
            .filter_map(|&k| {
                let unit = io.world.troops.get(k)?;
                if crate::dat::Faction::of_alliance(unit.is_alliance) != side {
                    return None;
                }
                if defensive
                    && io
                        .galaxy
                        .classes
                        .get(unit.class_dat_id)
                        .is_none_or(|r| r.flags & CLASS_DEFENSIVE == 0)
                {
                    return None;
                }
                let defense = io
                    .world
                    .troop_classes
                    .get(&unit.class_dat_id)
                    .map_or(0, |c| i32::try_from(c.defense_strength).unwrap_or(i32::MAX));
                Some((k, defense))
            })
            .collect()
    }

    /// The regiment cycle (`FUN_004dd8d0`).
    fn regiment_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            5 => {
                self.state = if self.keep_training_site(io) { 6 } else { 0xb };
                Step::pending()
            }
            6 => {
                self.state = if self.choose_destination(io) { 7 } else { 0xb };
                Step::pending()
            }
            7 => {
                self.state = self.plan_regiments(io);
                if self.state == 0 {
                    return self.end(YARD_CYCLE, None);
                }
                Step::pending()
            }
            // FUN_004dfa70 gives 0, so 8 (FUN_004dfa80) is never reached.
            0xb => {
                self.count = 0;
                self.state = 0;
                self.phase = YARD_CYCLE;
                Step::pending()
            }
            0xc => match destination_in(
                ProductionArea::TrainingFacility,
                self.destination,
                self.yard,
            ) {
                Some(order) => {
                    self.state = 0xd;
                    Step {
                        order: Some(order),
                        complete: true,
                        raise: true,
                    }
                }
                None => self.end(YARD_CYCLE, None),
            },
            0xd => {
                let order = build(io, self.yard, self.class, self.count);
                self.end(YARD_CYCLE, order)
            }
            _ => {
                self.state = 5;
                Step::pending()
            }
        }
    }

    /// `FUN_004df160`: module 5's site keeping on this module's list, with
    /// the site's training facilities into `+0x64`.
    ///
    /// port: the fallback, controller `+0x18`, has no counterpart.
    fn keep_training_site(&mut self, io: &mut Inputs<'_>) -> bool {
        self.capacity = 0;
        let kept = keep_site(io, &mut self.sites, &mut self.yard, MARK_TRAINING, KIND);
        if let Yard::System(site) | Yard::Manager(site) = self.yard {
            self.capacity = io.galaxy.system(site).map_or(0, |r| r.values[5]);
        }
        kept
    }

    /// `FUN_004df440`: the destination, by the five queries in the order
    /// `+0x78` sets, each while none has found a system. When every query
    /// but the home one found nothing, the strategy's posture is weak
    /// (controller `+4` [`WEAK`]) and regiments are budgeted, the side's
    /// garrison goals rise by one: `+0x384` below 12, `+0x380` below 6,
    /// `+0x37c` below 4 (`FUN_00418240(8, 7, 6)`).
    fn choose_destination(&mut self, io: &mut Inputs<'_>) -> bool {
        use Query::{Defended, Far, Home, Marked, Near};
        self.count = 0;
        self.class_flags = 0;
        self.class_key = RegimentKey::None;
        self.class_order = 0;
        self.missed &= !0xf;
        let (order, next): (&[Query], u32) = match self.next_query {
            1 => (&[Defended, Marked, Near, Home, Far], 2),
            2 => (&[Marked, Near, Near, Home, Far, Defended], 3),
            3 => (&[Home, Far, Defended, Marked, Near], 4),
            4 => (&[Far, Defended, Marked, Near, Home], 1),
            _ => (&[], 1),
        };
        for (i, &query) in order.iter().enumerate() {
            if i > 0 && self.destination.system().is_some() {
                continue;
            }
            self.query(io, query);
        }
        self.next_query = next;
        let found = self
            .destination
            .system()
            .is_some_and(|d| io.galaxy.system(d).is_some());
        if self.missed & 0xf == 0xf && io.controller.flags & WEAK != 0 && regiment_budget(io) > 0 {
            for (goal, ceiling) in io.controller.garrison_goals.iter_mut().zip([4, 6, 12]) {
                if *goal < ceiling {
                    *goal += 1;
                }
            }
        }
        found
    }

    /// One destination query (`FUN_004e1060`, `FUN_004e0ef0`,
    /// `FUN_004e1430`, `FUN_004e12e0`, `FUN_004e11a0`).
    fn query(&mut self, io: &mut Inputs<'_>, query: Query) {
        let origin = match self.yard {
            Yard::System(s) | Yard::Manager(s) => Some(s),
            Yard::None => None,
        };
        if query == Query::Home {
            if let Some(home) = origin {
                self.destination = Target::System(home);
                self.count = self.capacity;
                self.set_class(CLASS_OFFENSIVE, RegimentKey::Attack, 1);
            }
            return;
        }
        let sector = match query {
            Query::Far => pick(&[io.sectors(
                [Mask::new(0x4000_0000, 0), ANY, ANY],
                0x1e,
                Order::Descending,
            )]),
            _ => {
                let bit = match query {
                    Query::Defended => 0x80_0000,
                    Query::Marked => 0x40_0000,
                    _ => 0x4_0000,
                };
                origin.and_then(|o| {
                    let sectors = io.galaxy.sectors_by_distance(
                        o,
                        [Mask::new(bit, 0), ANY, ANY],
                        Order::Descending,
                        io.coin,
                    );
                    pick(&[sectors])
                })
            }
        };
        let state = match query {
            Query::Defended => Mask::new(0x2000, 0x400_0003),
            Query::Marked => Mask::new(0x2_0000, 3),
            _ => Mask::new(0x2000, 3),
        };
        let found = pick(&[io.systems(
            sector,
            [state, ANY, Mask::new(1, 0)],
            0x15,
            Order::Descending,
        )]);
        self.destination = found.map_or(Target::None, Target::System);
        let record = found.and_then(|s| io.galaxy.system(s)).copied();
        // FUN_004e1430 sets its bit only when +0x54 holds no system; a
        // system the query gives always has a record, so a missing record
        // means no system for every query.
        let Some(r) = record else {
            self.missed |= match query {
                Query::Defended => 1,
                Query::Far => 2,
                Query::Near => 4,
                Query::Marked => 8,
                Query::Home => 0,
            };
            return;
        };
        self.count = r.values[21];
        match query {
            Query::Marked if r.state & 0x83e0_0000 == 0 => {
                self.set_class(CLASS_DEFENSIVE, RegimentKey::Maintenance, 2);
            }
            Query::Near => self.set_class(CLASS_DEFENSIVE, RegimentKey::Maintenance, 2),
            _ => self.set_class(CLASS_DEFENSIVE, RegimentKey::Defense, 1),
        }
    }

    fn set_class(&mut self, flags: u32, key: RegimentKey, order: u32) {
        self.class_flags = flags;
        self.class_key = key;
        self.class_order = order;
    }

    /// `FUN_004df9a0`: `0xc` to train regiments of the chosen class (at
    /// most what the budget pays for and the site's training facilities),
    /// 8 when regiments cost more than the budget, else 0. With no budget
    /// and a secure posture (controller `+4` [`SECURE`]) the side's
    /// garrison goals fall by one: `+0x384` above 9, `+0x380` above 3,
    /// `+0x37c` above 0 (`FUN_00418270(8, 7, 6)`).
    ///
    /// port: controller `+0x14`, which it clears, has no counterpart.
    fn plan_regiments(&mut self, io: &mut Inputs<'_>) -> u32 {
        let budget = regiment_budget(io);
        if io.controller.flags & SECURE != 0 && budget < 1 {
            for (goal, floor) in io.controller.garrison_goals.iter_mut().zip([0, 3, 9]) {
                if *goal > floor {
                    *goal -= 1;
                }
            }
        }
        if budget < 0 {
            return 8;
        }
        if budget == 0 {
            return 0;
        }
        let key = match self.class_key {
            RegimentKey::Attack => ClassKey::Value(0),
            RegimentKey::Defense => ClassKey::Value(1),
            RegimentKey::Maintenance | RegimentKey::None => ClassKey::Maintenance,
        };
        let order = if self.class_order == 2 {
            Order::Descending
        } else {
            Order::Ascending
        };
        let maintenance;
        (self.class, maintenance) = best_class(
            io,
            ProductionArea::TrainingFacility,
            self.class_flags,
            key,
            order,
        );
        if self.class.is_some() && maintenance != 0 && maintenance * self.count > budget {
            self.count = budget / maintenance;
        }
        if self.count > 0 {
            self.count = self.count.min(self.capacity);
            return 0xc;
        }
        0
    }

    /// The yard cycle (`FUN_004dda00`).
    fn yard_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            6 => {
                if choose_target(
                    io,
                    &mut self.sites,
                    &mut self.target,
                    &mut self.room,
                    MARK_TRAINING,
                    KIND,
                ) {
                    self.state = 5;
                    Step::pending()
                } else {
                    self.end(MOVE_CYCLE, None)
                }
            }
            5 => {
                io.controller.yard_requests &= !YARD_TOKEN;
                if io.controller.yard_holder == YARD_TOKEN && self.borrow_yard(io) {
                    io.controller.pass_yards();
                    self.state = 9;
                } else {
                    self.state = 0xe;
                }
                Step::pending()
            }
            9 => {
                let order = scrap_at(io, self.target.system(), KIND);
                self.state = 7;
                Step::ordered(order)
            }
            7 => {
                self.state = self.plan_yards(io);
                if self.state == 0 {
                    return self.end(MOVE_CYCLE, None);
                }
                Step::pending()
            }
            10 => {
                let order =
                    scrap_spare_yard(io, &mut self.sites, &mut self.target, MARK_TRAINING, KIND);
                self.end(MOVE_CYCLE, order)
            }
            0xc => match destination(self.target, self.yard) {
                Some(order) => {
                    self.state = 0xd;
                    Step {
                        order: Some(order),
                        complete: true,
                        raise: true,
                    }
                }
                None => self.end(MOVE_CYCLE, None),
            },
            0xd => {
                let order = build(io, self.yard, self.class, self.count);
                self.end(MOVE_CYCLE, order)
            }
            0xe => {
                let budget = training_budget(io, &self.sites);
                self.target = adopt_or_request(
                    io,
                    &mut self.sites,
                    &mut self.class,
                    MARK_TRAINING,
                    YARD_TOKEN,
                    budget,
                    KIND,
                );
                claim(io, &self.sites, &mut self.claimed, KIND);
                self.end(MOVE_CYCLE, None)
            }
            _ => {
                self.state = 6;
                Step::pending()
            }
        }
    }

    /// `FUN_004dffd0`: borrow the idle construction yard of module 1's
    /// site in the sector nearest the target with `0x1000`, its system by
    /// construction yards (order 1).
    fn borrow_yard(&mut self, io: &mut Inputs<'_>) -> bool {
        self.capacity = 0;
        self.yard = Yard::None;
        let Some(target) = self.target.system() else {
            return false;
        };
        let sectors = io.galaxy.sectors_by_distance(
            target,
            [Mask::new(0x1000, 0), ANY, ANY],
            Order::Descending,
            io.coin,
        );
        let sector = pick(&[sectors]);
        let masks = [Mask::new(0x80_0800, 3), ANY, Mask::new(1, 0)];
        let Some(system) = pick(&[io.systems(sector, masks, 6, Order::Ascending)]) else {
            return false;
        };
        self.yard = Yard::System(system);
        let Some(r) = io.galaxy.system(system).copied() else {
            return false;
        };
        if !io.has_manager(system) {
            return false;
        }
        self.capacity = if r.values[6] > 1 {
            r.values[6] - 1
        } else {
            r.values[6]
        };
        self.yard = Yard::Manager(system);
        true
    }

    /// `FUN_004e06b0`: `0xc` to build training facilities, 10 to scrap one
    /// when the sites hold more than the side wants, else 0.
    fn plan_yards(&mut self, io: &mut Inputs<'_>) -> u32 {
        self.count = 0;
        let budget = training_budget(io, &self.sites);
        if budget < 0 {
            return 10;
        }
        if budget == 0 {
            return 0;
        }
        let maintenance;
        (self.class, maintenance) = yard_class(io, KIND);
        if self.class.is_some() && maintenance != 0 {
            self.count = budget / maintenance;
        }
        if self.count <= 0 {
            return 0;
        }
        self.count = self.count.min(self.capacity);
        let Some(r) = self.target.system().and_then(|t| io.galaxy.system(t)) else {
            return 0;
        };
        self.count = self.count.min(r.values[15].min(self.room));
        0xc
    }
}

/// `FUN_004e19c0`: 80% of `+0x24` percent of the capacity, less the
/// regiments' upkeep aboard fleets (`+0x250`) and on planets (`+0x1d8`); 0
/// when the side's free maintenance falls short of it.
fn regiment_budget(io: &Inputs<'_>) -> i32 {
    let t = io.tallies;
    let budget = PERCENT * t.capacity / 100 * 80 / 100 - t.cargo_upkeep - t.regiment_upkeep;
    if t.capacity - t.requirement < budget {
        0
    } else {
        budget
    }
}

/// `FUN_004e1a20`: 20% of `+0x24` percent of the capacity, less the sites'
/// training facility upkeep (`v[35]`); 0 when the side's free maintenance
/// falls short of it.
fn training_budget(io: &Inputs<'_>, sites: &[SystemKey]) -> i32 {
    let held: i32 = sites
        .iter()
        .filter_map(|&s| io.galaxy.system(s))
        .map(|r| r.values[35])
        .sum();
    let t = io.tallies;
    let budget = PERCENT * t.capacity / 100 * 20 / 100 - held;
    if t.capacity - t.requirement < budget {
        0
    } else {
        budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{AIState, AiFaction, MARK_MINES};
    use crate::ai_module1::tests::{add_yard, class, fixture, record, records_of, Fixture, ROOMY};
    use crate::ai_module1::Tallies;
    use crate::ai_records::{Galaxy, SystemRecord};
    use crate::dat::Faction;
    use crate::manufacturing::BuildableKind;
    use crate::research::ResearchState;
    use crate::world::{TroopClassDef, TroopUnit};

    const TRAINING_CLASS: u32 = 0x2900_0002;
    /// A defensive regiment class (attack 2, defense 6, maintenance 2).
    const GUARD: u32 = 0x1000_0001;
    /// An offensive regiment class (attack 8, defense 3, maintenance 3).
    const RAIDER: u32 = 0x1000_0002;
    /// A cheaper defensive class (attack 1, defense 4, maintenance 1).
    const MILITIA: u32 = 0x1000_0003;

    fn with_io<R>(
        f: &Fixture,
        state: &mut AIState,
        galaxy: &mut Galaxy,
        tallies: Tallies,
        run: impl FnOnce(&mut Module6, &mut Inputs<'_>) -> R,
    ) -> R {
        let research = ResearchState::new();
        let mut coin = || false;
        let mut io = Inputs {
            world: &f.world,
            research: &research,
            galaxy,
            controller: &mut state.controller,
            marks: &mut state.yard_marks,
            kinds: &mut state.record_kinds,
            tallies,
            is_alliance: true,
            coin: &mut coin,
        };
        run(&mut state.module6, &mut io)
    }

    fn edited<R>(
        f: &Fixture,
        state: &mut AIState,
        tallies: Tallies,
        edit: impl FnOnce(&mut Galaxy),
        run: impl FnOnce(&mut Module6, &mut Inputs<'_>) -> R,
    ) -> R {
        let mut galaxy = records_of(f, state);
        edit(&mut galaxy);
        with_io(f, state, &mut galaxy, tallies, run)
    }

    fn step(f: &Fixture, state: &mut AIState, tallies: Tallies) -> Step {
        edited(f, state, tallies, |_| {}, |m, io| m.step(io))
    }

    fn rec(g: &mut Galaxy, s: SystemKey) -> &mut SystemRecord {
        record(g, s)
    }

    /// The fixture with a training facility at `yard` and three regiment
    /// classes.
    fn regiment_fixture() -> Fixture {
        let mut f = fixture();
        let training = class(&mut f.world, TRAINING_CLASS, 10, 2, 0);
        add_yard(&mut f.world, f.yard, training, Faction::Alliance);
        for (raw, attack, defense, upkeep) in
            [(GUARD, 2, 6, 2), (RAIDER, 8, 3, 3), (MILITIA, 1, 4, 1)]
        {
            class(&mut f.world, raw, upkeep, 0, 0);
            f.world.troop_classes.insert(
                DatId::new(raw),
                TroopClassDef {
                    attack_strength: attack,
                    defense_strength: defense,
                    detection: 0,
                },
            );
        }
        f
    }

    fn add_regiment(
        world: &mut crate::world::GameWorld,
        at: SystemKey,
        raw: u32,
        is_alliance: bool,
    ) -> TroopKey {
        let key = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(raw),
            is_alliance,
            regiment_strength: 100,
        });
        world.systems[at].ground_units.push(key);
        key
    }

    fn with_site(f: &Fixture) -> AIState {
        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.sites.push(f.yard);
        state.yard_marks.insert(f.yard, MARK_TRAINING);
        state
    }

    #[test]
    fn the_cycles_hand_on_from_moves_to_regiments_to_yards() {
        // FUN_004dd7a0: any other +0x38 starts 0x3ef. FUN_004dd840 tries
        // its four moves (none here) and hands on to 0x3f0, complete.
        // FUN_004dd8d0: 5 without a site gives 0xb, which hands on to
        // 0x3ec (FUN_004dfa70 gives 0). FUN_004dda00: 6 takes the yard as
        // the target (module 1's mark is not forbidden, 0x400003), 5
        // without the token goes to 0xe, which ends with 0x3ef.
        let f = regiment_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.yard_marks.insert(f.empty, MARK_MINES);
        let mut trail = Vec::new();
        for _ in 0..14 {
            let s = step(&f, &mut state, ROOMY);
            trail.push((state.module6.phase, state.module6.state, s.complete));
        }
        assert_eq!(
            trail,
            [
                (MOVE_CYCLE, 0, false),
                (MOVE_CYCLE, 1, false),
                (MOVE_CYCLE, 2, false),
                (MOVE_CYCLE, 3, false),
                (MOVE_CYCLE, 4, false),
                (REGIMENT_CYCLE, 0, true),
                (REGIMENT_CYCLE, 5, false),
                (REGIMENT_CYCLE, 0xb, false),
                (YARD_CYCLE, 0, false),
                (YARD_CYCLE, 6, false),
                (YARD_CYCLE, 5, false),
                (YARD_CYCLE, 0xe, false),
                (MOVE_CYCLE, 0, true),
                (MOVE_CYCLE, 1, false),
            ]
        );
        assert_eq!(Module6::default().next_query, 1);
    }

    /// Records with `empty` short of garrison (`v[21]` 3, `+0x28`
    /// `bits`) and `yard` with a surplus (`v[19]` `surplus`, `+0x28`
    /// `0x10000`), the sector flagged for both.
    fn short_and_surplus(g: &mut Galaxy, f: &Fixture, sector: u32, bits: u32, surplus: i32) {
        g.sectors[0].candidates |= sector | 0x8_0000;
        let d = rec(g, f.empty);
        d.state |= bits;
        d.values[21] = 3;
        let s = rec(g, f.yard);
        s.state |= 0x1_0000;
        s.values[19] = surplus;
    }

    #[test]
    fn the_short_system_gets_the_surplus_s_weakest_regiments() {
        // FUN_004de6f0 (state 3): the system with +0x28 0x2001 in the
        // sector with +0x24 0x40010; the source nearest it with 0x80000
        // and +0x28 0x10000; up to v[21] (3), at most the source's v[19]
        // (2), lowest defense (slot +0x1e0, class +0x68) first. The order
        // ends the cycle (FUN_004dd840).
        let mut f = regiment_fixture();
        let guard = add_regiment(&mut f.world, f.yard, GUARD, true);
        let raider = add_regiment(&mut f.world, f.yard, RAIDER, true);
        let militia = add_regiment(&mut f.world, f.yard, MILITIA, true);
        add_regiment(&mut f.world, f.yard, MILITIA, false);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.phase = MOVE_CYCLE;
        state.module6.state = 3;
        let s = edited(
            &f,
            &mut state,
            ROOMY,
            |g| short_and_surplus(g, &f, 0x4_0010, 0x2001, 2),
            |m, io| m.step(io),
        );
        assert_eq!(
            s,
            Step {
                order: Some(AIAction::MoveRegiments {
                    troops: vec![raider, militia],
                    to: f.empty,
                }),
                complete: true,
                raise: false,
            }
        );
        assert_eq!(
            (state.module6.phase, state.module6.state),
            (REGIMENT_CYCLE, 0)
        );
        assert_eq!(state.module6.source, Target::System(f.yard));
        let _ = guard;

        // A shortfall of 1 takes one; no surplus record, none.
        let mut state = AIState::new(AiFaction::Alliance);
        let order = edited(
            &f,
            &mut state,
            ROOMY,
            |g| short_and_surplus(g, &f, 0x4_0010, 0x2001, 1),
            |m, io| m.move_to_short(io, Mask::new(0x4_0010, 0), 0x2001, false),
        );
        assert_eq!(
            order,
            Some(AIAction::MoveRegiments {
                troops: vec![raider],
                to: f.empty
            })
        );
        let order = edited(
            &f,
            &mut state,
            ROOMY,
            |g| short_and_surplus(g, &f, 0x4_0010, 0x2001, 0),
            |m, io| m.move_to_short(io, Mask::new(0x4_0010, 0), 0x2001, false),
        );
        assert_eq!(order, None);
    }

    #[test]
    fn the_defensive_move_takes_defensive_classes_and_falls_back() {
        // FUN_004deba0 (state 4): sector 0x40000, system 0x2000; the
        // source with sector +0x2c 0x1000 and +0x30 0x8001, else the
        // sector with 0x280000 and a system with 0x20010000; regiments
        // whose class record has 0x2000 only. State 4 ends the cycle
        // either way.
        let mut f = regiment_fixture();
        let guard = add_regiment(&mut f.world, f.yard, GUARD, true);
        add_regiment(&mut f.world, f.yard, RAIDER, true);
        let militia = add_regiment(&mut f.world, f.yard, MILITIA, true);
        let mut state = AIState::new(AiFaction::Alliance);
        let first = |g: &mut Galaxy| {
            short_and_surplus(g, &f, 0x4_0000, 0x2000, 5);
            g.sectors[0].summary |= 0x1000;
            rec(g, f.yard).kind |= 0x8001;
        };
        let order = edited(&f, &mut state, ROOMY, first, |m, io| {
            m.move_to_short(io, Mask::new(0x4_0000, 0), 0x2000, true)
        });
        assert_eq!(
            order,
            Some(AIAction::MoveRegiments {
                troops: vec![militia, guard],
                to: f.empty
            })
        );
        let fallback = |g: &mut Galaxy| {
            short_and_surplus(g, &f, 0x4_0000, 0x2000, 5);
            g.sectors[0].candidates |= 0x28_0000;
            rec(g, f.yard).state |= 0x2000_0000;
        };
        let order = edited(&f, &mut state, ROOMY, fallback, |m, io| {
            m.move_to_short(io, Mask::new(0x4_0000, 0), 0x2000, true)
        });
        assert_eq!(
            order,
            Some(AIAction::MoveRegiments {
                troops: vec![militia, guard],
                to: f.empty
            })
        );
        state.module6.phase = MOVE_CYCLE;
        state.module6.state = 4;
        assert_eq!(step(&f, &mut state, ROOMY), Step::complete());
        assert_eq!(
            (state.module6.phase, state.module6.state),
            (REGIMENT_CYCLE, 0)
        );
    }

    #[test]
    fn the_planned_moves_find_no_planner_and_clear_their_systems() {
        // FUN_004ddb70 and FUN_004de130 take their count from module 8's
        // sector planners (FUN_0042f310), which the port has not.
        let f = regiment_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.source = Target::System(f.yard);
        state.module6.destination = Target::System(f.empty);
        state.module6.phase = MOVE_CYCLE;
        state.module6.state = 1;
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module6.state, 2);
        assert_eq!(
            (state.module6.source, state.module6.destination),
            (Target::None, Target::None)
        );
    }

    /// `yard` as the training site, idle.
    fn idle(g: &mut Galaxy, f: &Fixture) {
        rec(g, f.yard).state |= 0x400;
    }

    #[test]
    fn the_regiment_cycle_trains_the_home_guard_and_sends_it() {
        // FUN_004dd8d0: 5 (FUN_004df160: the site, +0x64 = v[5]) -> 6
        // (FUN_004df440 from +0x78 = 3: FUN_004e12e0 first, the site's
        // own system, +0x60 = +0x64, offensive by attack) -> 7
        // (FUN_004df9a0: the raider, within 80% of 22% of 1000 = 176)
        // -> 0xc (Destination on the troop manager) -> 0xd (Build 0x212)
        // -> end with 0x3ec.
        let f = regiment_fixture();
        let mut state = with_site(&f);
        state.module6.phase = REGIMENT_CYCLE;
        state.module6.next_query = 3;
        let mut trail = Vec::new();
        let mut order = None;
        for _ in 0..6 {
            let s = edited(&f, &mut state, ROOMY, |g| idle(g, &f), |m, io| m.step(io));
            trail.push((state.module6.state, s.complete, s.raise));
            if s.order.is_some() {
                order = s.order;
                break;
            }
        }
        assert_eq!(
            trail,
            [
                (5, false, false),
                (6, false, false),
                (7, false, false),
                (0xc, false, false),
                (0xd, true, true),
            ]
        );
        assert_eq!(
            order,
            Some(AIAction::SetDestination {
                system: f.yard,
                area: ProductionArea::TrainingFacility,
                destination: f.yard,
            })
        );
        assert_eq!(state.module6.next_query, 4);
        let s = step(&f, &mut state, ROOMY);
        assert_eq!(
            s.order,
            Some(AIAction::Build {
                system: f.yard,
                kind: BuildableKind::Troop(DatId::new(RAIDER)),
                cost: 10,
                count: 1,
            })
        );
        assert_eq!((state.module6.phase, state.module6.state), (YARD_CYCLE, 0));
    }

    #[test]
    fn the_destination_queries_set_the_class_their_system_needs() {
        // FUN_004e1060 (near 0x800000, 0x2000 forbidding 0x4000003):
        // defensive by defense, order 1. FUN_004e0ef0 (near 0x400000,
        // 0x20000): the same when +0x28 has 0x83e00000, else by
        // maintenance, order 2. FUN_004e1430 (near 0x40000, 0x2000): by
        // maintenance, order 2. FUN_004e11a0 (sector 0x40000000 by 0x1e):
        // as FUN_004e1060. Each takes v[21]; a miss sets +0x28 1, 8, 4, 2.
        let f = regiment_fixture();
        let cases: [(Query, u32, u32, u32, RegimentKey, u32); 5] = [
            (
                Query::Defended,
                0x80_0000,
                0x2000,
                0,
                RegimentKey::Defense,
                1,
            ),
            (
                Query::Marked,
                0x40_0000,
                0x2_0000,
                0,
                RegimentKey::Maintenance,
                2,
            ),
            (
                Query::Marked,
                0x40_0000,
                0x2_0000,
                MARK_MINES,
                RegimentKey::Defense,
                1,
            ),
            (
                Query::Near,
                0x4_0000,
                0x2000,
                0,
                RegimentKey::Maintenance,
                2,
            ),
            (Query::Far, 0x4000_0000, 0x2000, 0, RegimentKey::Defense, 1),
        ];
        for (query, sector, state_bit, mark, key, order) in cases {
            let mut state = AIState::new(AiFaction::Alliance);
            state.module6.yard = Yard::Manager(f.yard);
            if mark != 0 {
                state.yard_marks.insert(f.empty, mark);
            }
            edited(
                &f,
                &mut state,
                ROOMY,
                |g| {
                    g.sectors[0].candidates |= sector;
                    let r = rec(g, f.empty);
                    r.state = (r.state | state_bit) & !0x400_0000;
                    r.values[21] = 4;
                },
                |m, io| m.query(io, query),
            );
            let m = &state.module6;
            assert_eq!(m.destination, Target::System(f.empty), "{query:?}");
            assert_eq!(
                (m.count, m.class_flags, m.class_key, m.class_order),
                (4, CLASS_DEFENSIVE, key, order),
                "{query:?}"
            );
        }
        for (query, bit) in [
            (Query::Defended, 1),
            (Query::Far, 2),
            (Query::Near, 4),
            (Query::Marked, 8),
        ] {
            let mut state = AIState::new(AiFaction::Alliance);
            state.module6.yard = Yard::Manager(f.yard);
            edited(
                &f,
                &mut state,
                ROOMY,
                |g| g.sectors[0].candidates = 0,
                |m, io| m.query(io, query),
            );
            assert_eq!(state.module6.missed, bit, "{query:?}");
            assert_eq!(state.module6.destination, Target::None);
        }
    }

    #[test]
    fn the_destination_queries_run_in_turn_until_one_finds_a_system() {
        // FUN_004df440: +0x78 picks the order and moves on (4 back to 1;
        // any other value gives 1 and no query); a query runs only while
        // +0x54 holds no system; success needs its record.
        let f = regiment_fixture();
        let mut state = with_site(&f);
        state.module6.yard = Yard::Manager(f.yard);
        state.module6.capacity = 2;
        // From 1: FUN_004e1060 finds the short system first. The marked
        // site's own record is short too (FUN_00433e40); the edit clears
        // that.
        let short = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x80_0000;
            let r = rec(g, f.empty);
            r.state = (r.state | 0x2000) & !0x400_0000;
            r.values[21] = 4;
            rec(g, f.yard).state &= !0x2000;
        };
        let found = edited(&f, &mut state, ROOMY, short, |m, io| {
            m.choose_destination(io)
        });
        assert!(found);
        assert_eq!(state.module6.destination, Target::System(f.empty));
        assert_eq!((state.module6.next_query, state.module6.count), (2, 4));
        // From 4: FUN_004e11a0 and the rest miss; FUN_004e12e0 last gives
        // the site's own system.
        state.module6.next_query = 4;
        let found = edited(
            &f,
            &mut state,
            ROOMY,
            |g| g.sectors[0].candidates = 0,
            |m, io| m.choose_destination(io),
        );
        assert!(found);
        assert_eq!(state.module6.destination, Target::System(f.yard));
        assert_eq!(state.module6.missed, 0xf);
        assert_eq!(
            (state.module6.count, state.module6.class_flags),
            (2, CLASS_OFFENSIVE)
        );
        assert_eq!(state.module6.next_query, 1);
        state.module6.next_query = 7;
        state.module6.destination = Target::None;
        let found = edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| m.choose_destination(io),
        );
        assert!(!found);
        assert_eq!(state.module6.next_query, 1);
    }

    #[test]
    fn the_regiment_plan_pays_within_the_budget_and_the_site() {
        // FUN_004df9a0: w = FUN_004e19c0 (80% of 22% of the capacity less
        // +0x250 and +0x1d8): w < 0 gives 8, w == 0 gives 0; the class by
        // the chosen flags, key and order; m * count > w gives w / m;
        // then at most +0x64; 0xc with a count.
        let f = regiment_fixture();
        let plan =
            |count: i32, capacity: i32, key: RegimentKey, order: u32, flags: u32, upkeep: i32| {
                let mut state = AIState::new(AiFaction::Alliance);
                state.module6.count = count;
                state.module6.capacity = capacity;
                state.module6.class_flags = flags;
                state.module6.class_key = key;
                state.module6.class_order = order;
                let t = Tallies {
                    regiment_upkeep: upkeep,
                    cargo_upkeep: 1,
                    ..ROOMY
                };
                let next = edited(&f, &mut state, t, |_| {}, |m, io| m.plan_regiments(io));
                (next, state.module6.count, state.module6.class)
            };
        let id = |raw| Some(DatId::new(raw));
        // Budget 176 less 1 and the upkeep.
        assert_eq!(
            plan(3, 9, RegimentKey::Defense, 1, CLASS_DEFENSIVE, 0),
            (0xc, 3, id(GUARD))
        );
        assert_eq!(
            plan(3, 9, RegimentKey::Maintenance, 2, CLASS_DEFENSIVE, 0),
            (0xc, 3, id(MILITIA))
        );
        assert_eq!(
            plan(3, 2, RegimentKey::Attack, 1, CLASS_OFFENSIVE, 0),
            (0xc, 2, id(RAIDER))
        );
        // 3 guards cost 6 against a budget of 5.
        assert_eq!(
            plan(3, 9, RegimentKey::Defense, 1, CLASS_DEFENSIVE, 170),
            (0xc, 2, id(GUARD))
        );
        assert_eq!(
            plan(3, 9, RegimentKey::Defense, 1, CLASS_DEFENSIVE, 174),
            (0, 0, id(GUARD))
        );
        assert_eq!(
            plan(3, 9, RegimentKey::Defense, 1, CLASS_DEFENSIVE, 175).0,
            0
        );
        assert_eq!(
            plan(3, 9, RegimentKey::Defense, 1, CLASS_DEFENSIVE, 176).0,
            8
        );
    }

    #[test]
    fn the_budgets_are_eighty_and_twenty_percent_of_twenty_two() {
        // FUN_004e19c0 and FUN_004e1a20; 0 when capacity - requirement is
        // below them.
        let f = regiment_fixture();
        let mut state = with_site(&f);
        let t = |requirement| Tallies {
            requirement,
            ..ROOMY
        };
        let run = |state: &mut AIState, t: Tallies| {
            edited(
                &f,
                state,
                t,
                |_| {},
                |m, io| (regiment_budget(io), training_budget(io, &m.sites)),
            )
        };
        // 22% of 1000 is 220: 176 and 44, less the site's 10.
        assert_eq!(run(&mut state, t(0)), (176, 34));
        assert_eq!(run(&mut state, t(824)), (176, 34));
        assert_eq!(run(&mut state, t(825)), (0, 34));
        assert_eq!(run(&mut state, t(966)), (0, 34));
        assert_eq!(run(&mut state, t(967)), (0, 0));
    }

    #[test]
    fn a_kept_site_gives_its_training_facilities() {
        // FUN_004df160: +0x64 = the kept site's v[5]; 0 without one.
        let f = regiment_fixture();
        let mut state = with_site(&f);
        let kept = edited(
            &f,
            &mut state,
            ROOMY,
            |g| idle(g, &f),
            |m, io| m.keep_training_site(io),
        );
        assert!(kept);
        assert_eq!(
            (state.module6.capacity, state.module6.yard),
            (1, Yard::Manager(f.yard))
        );
        let kept = edited(
            &f,
            &mut state,
            ROOMY,
            |g| rec(g, f.yard).state &= !0x400,
            |m, io| m.keep_training_site(io),
        );
        assert!(!kept);
        assert_eq!(state.module6.capacity, 0);
    }

    #[test]
    fn the_yard_cycle_borrows_near_the_target_and_builds_training_facilities() {
        // FUN_004dda00 5: bit 0x20 cleared; with +0xc == 0x20, FUN_004dffd0
        // takes the sector nearest the target with 0x1000 and its system
        // with 0x800800 by key 6: +0x64 = v[6] (less one above 1), no new
        // site; FUN_004199b0; 9 (FUN_004e0170) -> 7 (FUN_004e06b0, 20% of
        // 22%, the training class) gives 0xc -> Destination -> Build.
        let mut f = regiment_fixture();
        add_yard(
            &mut f.world,
            f.yard,
            DatId::new(0x2a00_0003),
            Faction::Alliance,
        );
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.controller.yard_holder = YARD_TOKEN;
        state.controller.yard_requests = YARD_TOKEN | 0x80;
        state.module6.phase = YARD_CYCLE;
        state.module6.state = 5;
        state.module6.target = Target::System(f.empty);
        state.module6.room = 9;
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module6.state, 9);
        assert_eq!(
            (state.module6.yard, state.module6.capacity),
            (Yard::Manager(f.yard), 1)
        );
        assert_eq!(state.controller.yard_holder, 0x80);
        assert_eq!(state.controller.yard_requests, 0);
        assert!(state.module6.sites.is_empty());
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module6.state, 7);
        let rich = Tallies {
            capacity: 4000,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, rich), Step::pending());
        assert_eq!((state.module6.state, state.module6.count), (0xc, 1));
        let s = step(&f, &mut state, rich);
        assert!(s.complete && s.raise);
        let s = step(&f, &mut state, rich);
        let Some(AIAction::Build { kind, .. }) = s.order else {
            panic!("{s:?}");
        };
        assert_eq!(kind.class_dat_id(), Some(DatId::new(TRAINING_CLASS)));
        assert_eq!((state.module6.phase, state.module6.state), (MOVE_CYCLE, 0));

        // No sector, no system, or no manager: no borrow.
        let f2 = regiment_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.target = Target::System(f2.empty);
        assert!(!edited(
            &f2,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| m.borrow_yard(io)
        ));
        assert_eq!(state.module6.yard, Yard::None);
        state.module6.target = Target::None;
        assert!(!edited(
            &f2,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| m.borrow_yard(io)
        ));
        let mut f3 = regiment_fixture();
        let k = f3.world.systems[f3.yard].manufacturing_facilities.remove(0);
        f3.world.manufacturing_facilities.remove(k);
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f3.yard, MARK_MINES);
        state.module6.target = Target::System(f3.empty);
        let lent = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x1000;
            let r = rec(g, f3.yard);
            r.state |= 0x800;
            r.values[6] = 1;
        };
        assert!(!edited(&f3, &mut state, ROOMY, lent, |m, io| m.borrow_yard(io)));
        assert_eq!(state.module6.yard, Yard::System(f3.yard));
    }

    #[test]
    fn the_training_plan_scraps_a_surplus_and_ends_otherwise() {
        // FUN_004e06b0: w < 0 gives 10 (FUN_004e0750: a spare training
        // facility), w == 0 or no yard's worth gives 0 (end with 0x3ef);
        // the count is capped by +0x64 and min(v[15], +0x68).
        let f = regiment_fixture();
        let plan = |capacity: i32, sites: Vec<SystemKey>, room: i32, lent: i32| {
            let mut state = AIState::new(AiFaction::Alliance);
            state.module6.sites = sites;
            state.module6.target = Target::System(f.empty);
            state.module6.room = room;
            state.module6.capacity = lent;
            let t = Tallies { capacity, ..ROOMY };
            let next = edited(&f, &mut state, t, |_| {}, |m, io| m.plan_yards(io));
            (next, state.module6.count)
        };
        // 20% of 22% of 4000 is 176: 17 of 10.
        assert_eq!(plan(4000, vec![], 9, 9), (0xc, 9));
        assert_eq!(plan(4000, vec![], 2, 9), (0xc, 2));
        assert_eq!(plan(4000, vec![], 9, 3), (0xc, 3));
        // 44 less the site's 10 is 34.
        assert_eq!(plan(1000, vec![f.yard], 9, 9), (0xc, 3));
        // 22% of 200 is 44, a fifth 8, less 10.
        assert_eq!(plan(200, vec![f.yard], 9, 9), (10, 0));
        assert_eq!(plan(200, vec![], 9, 9), (0, 0));
        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.room = 9;
        state.module6.capacity = 9;
        assert_eq!(
            edited(&f, &mut state, ROOMY, |_| {}, |m, io| m.plan_yards(io)),
            0
        );

        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.phase = YARD_CYCLE;
        state.module6.state = 7;
        let small = Tallies {
            capacity: 200,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, small), Step::complete());
        assert_eq!((state.module6.phase, state.module6.state), (MOVE_CYCLE, 0));

        let mut f = regiment_fixture();
        add_yard(
            &mut f.world,
            f.yard,
            DatId::new(TRAINING_CLASS),
            Faction::Alliance,
        );
        let mut state = with_site(&f);
        state.module6.phase = YARD_CYCLE;
        state.module6.state = 10;
        let s = edited(
            &f,
            &mut state,
            small,
            |g| rec(g, f.yard).state |= 0x1000,
            |m, io| m.step(io),
        );
        assert!(matches!(s.order, Some(AIAction::Scrap { ref team }) if team.len() == 1));
        assert_eq!((state.module6.phase, state.module6.state), (MOVE_CYCLE, 0));

        // 0xc without a manager ends.
        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.phase = YARD_CYCLE;
        state.module6.state = 0xc;
        state.module6.target = Target::System(f.empty);
        state.module6.yard = Yard::System(f.yard);
        assert_eq!(step(&f, &mut state, ROOMY), Step::complete());
        assert_eq!((state.module6.phase, state.module6.state), (MOVE_CYCLE, 0));
    }

    #[test]
    fn state_e_adopts_with_any_number_of_sites_and_claims() {
        // FUN_004e15b0 has no two-site limit (FUN_004d1970 has); it adopts
        // the training facility system and FUN_004e1b00 claims it.
        let f = regiment_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.sites = vec![f.empty, f.empty];
        state.module6.phase = YARD_CYCLE;
        state.module6.state = 0xe;
        let rich = Tallies {
            capacity: 4000,
            ..ROOMY
        };
        let s = edited(
            &f,
            &mut state,
            rich,
            |g| rec(g, f.yard).values[18] = 1,
            |m, io| m.step(io),
        );
        assert_eq!(s, Step::complete());
        assert_eq!(state.module6.sites, vec![f.empty, f.empty, f.yard]);
        assert_eq!(state.module6.claimed, vec![f.yard]);
        assert_eq!(state.yard_marks.get(&f.yard), Some(&MARK_TRAINING));
    }

    #[test]
    fn the_regiment_cycle_without_a_destination_or_manager_hands_on() {
        // FUN_004dd8d0: 6 without a destination gives 0xb; 7 at 0 ends
        // with 0x3ec; 0xc without a manager ends.
        let f = regiment_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.phase = REGIMENT_CYCLE;
        state.module6.state = 6;
        state.module6.next_query = 9;
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module6.state, 0xb);
        state.module6.state = 7;
        let zero = Tallies {
            requirement: 1000,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, zero), Step::complete());
        assert_eq!((state.module6.phase, state.module6.state), (YARD_CYCLE, 0));
        state.module6.phase = REGIMENT_CYCLE;
        state.module6.state = 0xc;
        state.module6.destination = Target::System(f.empty);
        state.module6.yard = Yard::System(f.yard);
        assert_eq!(step(&f, &mut state, ROOMY), Step::complete());
        assert_eq!((state.module6.phase, state.module6.state), (YARD_CYCLE, 0));
        state.module6.refused();
        assert_eq!((state.module6.phase, state.module6.state), (0, 0));
    }

    #[test]
    fn the_tallies_sum_the_regiments_upkeep_and_flag_short_sectors() {
        // FUN_0041b7e0: +0x1d8 sums sector +0xfc (own systems' v[30]);
        // +4 bit 0x20 for a sector with +0x2c 0x2000000. FUN_0041ba80:
        // +0x250 sums own fleets' values[15].
        let f = regiment_fixture();
        let state = AIState::new(AiFaction::Alliance);
        let mut g = records_of(&f, &state);
        g.sectors[0].values[crate::ai_records::sector_at(0xfc)] = 7;
        g.sectors[0].summary |= 0x200_0000;
        let fleet = |flags, upkeep| {
            let mut r = crate::ai_records::FleetRecord {
                flags,
                values: [0; 30],
                ships: Vec::new(),
            };
            r.values[15] = upkeep;
            r
        };
        let mut keys = slotmap::SlotMap::<crate::ids::FleetKey, ()>::with_key();
        g.fleets
            .insert(keys.insert(()), fleet(crate::ai_records::FLEET_OWN, 3));
        g.fleets.insert(keys.insert(()), fleet(4, 5));
        let t = Tallies::of(&g, 0, 0);
        assert_eq!(
            (t.regiment_upkeep, t.cargo_upkeep, t.sector_short),
            (7, 3, true)
        );
        g.sectors[0].summary &= !0x200_0000;
        assert!(!Tallies::of(&g, 0, 0).sector_short);
    }

    #[test]
    fn the_misses_clear_and_the_second_order_runs_the_marked_query_first() {
        // FUN_004df440 clears +0x28 bits 0xf before its queries; from +0x78
        // = 2 it starts with FUN_004e0ef0 and moves on to 3.
        let f = regiment_fixture();
        let mut state = with_site(&f);
        state.module6.yard = Yard::Manager(f.yard);
        state.module6.missed = 0xf;
        state.module6.next_query = 2;
        let marked = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x40_0000;
            let r = rec(g, f.empty);
            r.state = (r.state | 0x2_0000) & !0x400_0000;
            r.values[21] = 2;
        };
        let found = edited(&f, &mut state, ROOMY, marked, |m, io| {
            m.choose_destination(io)
        });
        assert!(found);
        assert_eq!(state.module6.destination, Target::System(f.empty));
        assert_eq!(state.module6.missed, 0);
        assert_eq!(state.module6.next_query, 3);
        assert_eq!(state.module6.class_key, RegimentKey::Maintenance);
    }

    #[test]
    fn the_defended_query_forbids_a_system_marked_0x4000000() {
        // FUN_004e1060's systems forbid +0x28 0x4000003; the others forbid
        // only 3.
        let f = regiment_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.yard = Yard::Manager(f.yard);
        let marked = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x80_0000;
            let r = rec(g, f.empty);
            r.state |= 0x2000 | 0x400_0000;
            rec(g, f.yard).state &= !0x2000;
        };
        edited(&f, &mut state, ROOMY, marked, |m, io| {
            m.query(io, Query::Defended)
        });
        assert_eq!(state.module6.destination, Target::None);
        assert_eq!(state.module6.missed, 1);
    }

    #[test]
    fn a_single_borrowed_yard_is_lent_whole_and_a_yard_plan_needs_a_budget() {
        // FUN_004dffd0: +0x64 = v[6], less one only above 1.
        // FUN_004e06b0: 10 only below 0; with a class and m != 0.
        let f = regiment_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.module6.target = Target::System(f.empty);
        assert!(edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| m.borrow_yard(io)
        ));
        assert_eq!(state.module6.capacity, 1);
        let zero = Tallies {
            requirement: 1000,
            ..ROOMY
        };
        assert_eq!(
            edited(&f, &mut state, zero, |_| {}, |m, io| m.plan_yards(io)),
            0
        );
        let mut f = regiment_fixture();
        f.world
            .buildable_classes
            .get_mut(&DatId::new(TRAINING_CLASS))
            .unwrap()
            .maintenance_cost = 0;
        let mut state = AIState::new(AiFaction::Alliance);
        state.module6.target = Target::System(f.empty);
        state.module6.room = 9;
        state.module6.capacity = 9;
        let next = edited(&f, &mut state, ROOMY, |_| {}, |m, io| m.plan_yards(io));
        assert_eq!((next, state.module6.count), (0, 0));
    }

    #[test]
    fn a_weak_posture_raises_the_garrison_goals_when_every_query_fails() {
        // FUN_004df440's end: +0x28 & 0xf == 0xf, controller +4 0x800000
        // and FUN_004e19c0 > 0 raise +0x384 below 12, +0x380 below 6 and
        // +0x37c below 4 (FUN_00418240(8, 7, 6)).
        let f = regiment_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        let choose = |state: &mut AIState, tallies| {
            state.module6.next_query = 1;
            edited(&f, state, tallies, |_| {}, |m, io| m.choose_destination(io))
        };
        assert!(!choose(&mut state, ROOMY));
        assert_eq!(state.module6.missed & 0xf, 0xf);
        assert_eq!(state.controller.garrison_goals, [0, 3, 9]);
        state.controller.flags |= WEAK;
        choose(&mut state, ROOMY);
        assert_eq!(state.controller.garrison_goals, [1, 4, 10]);
        // No regiment budget: no raise.
        let poor = Tallies {
            capacity: 0,
            ..ROOMY
        };
        choose(&mut state, poor);
        assert_eq!(state.controller.garrison_goals, [1, 4, 10]);
        state.controller.garrison_goals = [3, 6, 11];
        choose(&mut state, ROOMY);
        assert_eq!(state.controller.garrison_goals, [4, 6, 12]);
        choose(&mut state, ROOMY);
        assert_eq!(state.controller.garrison_goals, [4, 6, 12]);
        // A query that finds a system leaves the goals: order 3 asks the
        // site first (FUN_004e12e0), and the others do not run.
        state.controller.garrison_goals = [0, 3, 9];
        state.module6.yard = Yard::System(f.yard);
        state.module6.next_query = 3;
        let found = edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| m.choose_destination(io),
        );
        assert!(found);
        assert_eq!(state.module6.missed & 0xf, 0);
        assert_eq!(state.controller.garrison_goals, [0, 3, 9]);
    }

    #[test]
    fn a_secure_posture_lowers_the_garrison_goals_without_a_budget() {
        // FUN_004df9a0's end: controller +4 0x400000 and FUN_004e19c0 < 1
        // lower +0x384 above 9, +0x380 above 3 and +0x37c above 0
        // (FUN_00418270).
        let f = regiment_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.controller.garrison_goals = [1, 5, 10];
        let poor = Tallies {
            capacity: 0,
            ..ROOMY
        };
        edited(&f, &mut state, poor, |_| {}, |m, io| m.plan_regiments(io));
        assert_eq!(state.controller.garrison_goals, [1, 5, 10]);
        state.controller.flags |= SECURE;
        edited(&f, &mut state, ROOMY, |_| {}, |m, io| m.plan_regiments(io));
        assert_eq!(state.controller.garrison_goals, [1, 5, 10]);
        // A budget of 1 (22% of 10 is 2, 80% of that 1) is not below 1.
        let one = Tallies {
            capacity: 10,
            ..ROOMY
        };
        edited(&f, &mut state, one, |_| {}, |m, io| m.plan_regiments(io));
        assert_eq!(state.controller.garrison_goals, [1, 5, 10]);
        edited(&f, &mut state, poor, |_| {}, |m, io| m.plan_regiments(io));
        assert_eq!(state.controller.garrison_goals, [0, 4, 9]);
        edited(&f, &mut state, poor, |_| {}, |m, io| m.plan_regiments(io));
        assert_eq!(state.controller.garrison_goals, [0, 3, 9]);
    }
}
