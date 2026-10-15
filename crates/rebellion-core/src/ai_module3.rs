//! Module 3 of the AI player (`FUN_004d6b20`, step `FUN_004d6d30`): it
//! balances the side's planetary defenses, scrapping a surplus and
//! building where they fall short, and with its own construction yard
//! sites (mark `0x2000000`) builds mines, refineries and more yards
//! (`ghidra/notes/ai-production-modules.md`, "Module 3"). Its cycles share
//! module 1's and 2's steps ([`crate::ai_module1`], [`crate::ai_module2`]).

use serde::{Deserialize, Serialize};

use crate::ai::{AIAction, MARK_BALANCE};
use crate::ai_module1::{
    adopt_or_request, best_class, build, claim, destination, nearest, scrap_at, scrap_spare_yard,
    yard_class, Inputs, Step, Target, Yard, ANY, MINE_CLASS, REFINERY_CLASS,
};
use crate::ai_module2::{keep_site, scrap_in_the_way};
use crate::ai_records::{pick, side_constants::S36C, Mask, Order};
use crate::ids::{DatId, SystemKey};
use crate::scrap::ScrapTarget;

/// `+0x38` while the yard cycle runs (`FUN_004d6d30`).
pub const YARD_CYCLE: u32 = 0x3ec;

/// `+0x38` while the defense surplus cycle runs (`FUN_004d6d30`).
pub const SURPLUS_CYCLE: u32 = 0x3ed;

/// `+0x38` while the defense and mine cycle runs (`FUN_004d6d30`).
pub const DEFENSE_CYCLE: u32 = 0x3ee;

/// The yard token module 3 holds the construction yards with (controller
/// `+0xc`, `+8`; `FUN_004d7060` state 4).
pub const YARD_TOKEN: u32 = 1;

/// `+0x24`: the percent of the maintenance capacity module 3 budgets
/// (`FUN_004d6b20`), 90% of it for defenses (`FUN_004da4d0`) and 10% for
/// construction yards (`FUN_004da530`).
const PERCENT: i32 = 9;

/// The system `FUN_004d76f0` checks first: Coruscant (`0x90000109`).
pub const CORUSCANT: u32 = 0x9000_0109;

/// Module 3's own fields (`FUN_004d6b20`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module3 {
    /// `+0x38`: 0, [`SURPLUS_CYCLE`], [`DEFENSE_CYCLE`] or [`YARD_CYCLE`].
    pub phase: u32,
    /// `+0x3c`: the cycle's state.
    pub state: u32,
    /// `+0x50`: a site, then its construction manager.
    pub yard: Yard,
    /// `+0x54`: the system to scrap at or build at.
    pub site: Target,
    /// `+0x58`: the yard cycle's target.
    pub target: Target,
    /// `+0x5c`: the class to build.
    pub class: Option<DatId>,
    /// `+0x60`: how many.
    pub count: i32,
    /// `+0x64`: the borrowed yard's construction yards.
    pub capacity: i32,
    /// `+0x68`: the target's free room `v[0x33]`.
    pub room: i32,
    /// `+0x6c`: the sites, marked [`MARK_BALANCE`].
    pub sites: Vec<SystemKey>,
    /// `+0x74`: the claimed sites.
    pub claimed: Vec<SystemKey>,
}

impl Module3 {
    /// One step (`FUN_004d6d30`). The scheduler steps only a ready module,
    /// so the not-ready branch never runs.
    pub fn step(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.phase {
            YARD_CYCLE => self.yard_cycle(io),
            SURPLUS_CYCLE => self.surplus_cycle(io),
            DEFENSE_CYCLE => self.defense_cycle(io),
            _ => {
                self.phase = SURPLUS_CYCLE;
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

    /// End a cycle: `+0x3c = 0`, `+0x38` = the next cycle, complete.
    fn end(&mut self, next: u32, order: Option<AIAction>) -> Step {
        self.state = 0;
        self.phase = next;
        Step {
            order,
            complete: true,
            raise: false,
        }
    }

    /// The surplus cycle (`FUN_004d6da0`).
    fn surplus_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            1 => {
                if self.find_surplus(io) {
                    self.state = 2;
                } else {
                    self.state = 0;
                    self.phase = DEFENSE_CYCLE;
                }
                Step::pending()
            }
            2 => {
                let order = self.scrap_surplus(io);
                self.end(DEFENSE_CYCLE, order)
            }
            // Nothing in this cycle sets 3 or 7; state 3 falls into 7
            // whatever `FUN_004d76a0` gives (`FUN_004d6da0`).
            3 | 7 => {
                let order = self.scrap_one_defense(io);
                self.end(YARD_CYCLE, order)
            }
            _ => {
                self.state = 1;
                Step::pending()
            }
        }
    }

    /// The defense and mine cycle (`FUN_004d6e70`).
    fn defense_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            4 => {
                if keep_site(io, &mut self.sites, &mut self.yard, MARK_BALANCE) {
                    self.state = 1;
                } else {
                    self.state = 0;
                    self.phase = YARD_CYCLE;
                }
                Step::pending()
            }
            1 => {
                self.state = if self.find_short(io) { 6 } else { 9 };
                Step::pending()
            }
            6 => {
                let order = self.scrap_for_defenses(io);
                self.state = 5;
                Step::ordered(order)
            }
            5 => {
                self.state = self.plan_defenses(io);
                Step::pending()
            }
            7 => match self.scrap_one_defense(io) {
                Some(order) => self.end(YARD_CYCLE, Some(order)),
                None => {
                    self.state = 9;
                    Step::pending()
                }
            },
            9 => {
                let found = if io.tallies.refineries < io.tallies.mines {
                    self.class = Some(DatId::new(REFINERY_CLASS));
                    self.site = nearest(
                        io,
                        self.yard,
                        0x200,
                        [Mask::new(0x100, 0x3e0_0003), ANY, Mask::new(1, 0)],
                    );
                    self.site.system().is_some()
                } else {
                    self.class = Some(DatId::new(MINE_CLASS));
                    self.site = nearest(
                        io,
                        self.yard,
                        0x100,
                        [Mask::new(0x80, 0x3e0_0003), ANY, ANY],
                    );
                    self.site.system().is_some()
                };
                if found {
                    self.count = 1;
                    self.state = 10;
                } else {
                    self.state = 0;
                    self.phase = YARD_CYCLE;
                }
                // State 9 always runs state 10 (`FUN_004d6e70`).
                self.state_ten(io)
            }
            10 => self.state_ten(io),
            0xb => self.destination_state(self.site, DEFENSE_CYCLE),
            0xc => {
                let order = build(io, self.yard, self.class, self.count);
                self.end(YARD_CYCLE, order)
            }
            _ => {
                self.state = 4;
                Step::pending()
            }
        }
    }

    /// State 10 then `+0x3c = 0xb` (`FUN_004d6e70`).
    fn state_ten(&mut self, io: &mut Inputs<'_>) -> Step {
        let order = self.clear_site(io);
        self.state = 0xb;
        Step::ordered(order)
    }

    /// State `0xb` of either cycle (`FUN_004d9e70`): Destination to
    /// `site`; with none, the cycle `cycle` ends.
    fn destination_state(&mut self, site: Target, cycle: u32) -> Step {
        match destination(site, self.yard) {
            Some(order) => {
                self.state = 0xc;
                Step {
                    order: Some(order),
                    complete: true,
                    raise: true,
                }
            }
            None => self.end(Self::after(cycle), None),
        }
    }

    /// The cycle that follows `cycle`.
    const fn after(cycle: u32) -> u32 {
        match cycle {
            DEFENSE_CYCLE => YARD_CYCLE,
            YARD_CYCLE => SURPLUS_CYCLE,
            _ => DEFENSE_CYCLE,
        }
    }

    /// The yard cycle (`FUN_004d7060`).
    fn yard_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            1 => {
                if self.choose_target(io) {
                    self.state = 4;
                    Step::pending()
                } else {
                    self.end(SURPLUS_CYCLE, None)
                }
            }
            4 => {
                io.controller.yard_requests &= !YARD_TOKEN;
                if io.controller.yard_holder == YARD_TOKEN && self.borrow_yard(io) {
                    io.controller.pass_yards();
                    self.state = 6;
                } else {
                    self.state = 0xd;
                }
                Step::pending()
            }
            6 => {
                let order = scrap_at(io, self.target.system());
                self.state = 5;
                Step::ordered(order)
            }
            5 => {
                self.state = self.plan_yards(io);
                if self.state == 0 {
                    return self.end(SURPLUS_CYCLE, None);
                }
                Step::pending()
            }
            8 => {
                let order = scrap_spare_yard(io, &mut self.sites, &mut self.target, MARK_BALANCE);
                self.end(SURPLUS_CYCLE, order)
            }
            0xb => self.destination_state(self.target, YARD_CYCLE),
            0xc => {
                let order = build(io, self.yard, self.class, self.count);
                self.end(SURPLUS_CYCLE, order)
            }
            0xd => {
                let budget = yard_budget(io, &self.sites);
                self.target = adopt_or_request(
                    io,
                    &mut self.sites,
                    &mut self.class,
                    MARK_BALANCE,
                    YARD_TOKEN,
                    budget,
                );
                claim(io, &self.sites, &mut self.claimed);
                self.end(SURPLUS_CYCLE, None)
            }
            _ => {
                self.state = 1;
                Step::pending()
            }
        }
    }

    /// `FUN_004d71e0`: the system with the greatest defense surplus
    /// (`v[37]`) in the sector with the most, into `+0x54`.
    fn find_surplus(&mut self, io: &mut Inputs<'_>) -> bool {
        self.site = Self::surplus_site(io);
        self.site
            .system()
            .and_then(|s| io.galaxy.system(s))
            .is_some_and(|r| r.values[37] > 0)
    }

    /// The sector with `+0x24` `0x10000` by key `0x1d`, then its system by
    /// `v[37]` (order 1; `FUN_004d71e0`, `FUN_004d8520`).
    fn surplus_site(io: &mut Inputs<'_>) -> Target {
        let lists = vec![io.sectors([Mask::new(0x1_0000, 0), ANY, ANY], 0x1d, Order::Ascending)];
        let sector = pick(&lists);
        let lists = vec![io.systems(sector, [ANY, ANY, Mask::new(1, 0)], 0x25, Order::Ascending)];
        pick(&lists).map_or(Target::None, Target::System)
    }

    /// `FUN_004d7330`: at `+0x54`, the `0x24` defenses beyond the record's
    /// goal (`v[7] - v[9]`), then the batteries beyond theirs (`v[11] -
    /// v[13]`), the `0x22` ones first.
    fn scrap_surplus(&self, io: &mut Inputs<'_>) -> Option<AIAction> {
        let site = self.site.system()?;
        let r = io.galaxy.system(site).copied()?;
        let take = |n: i32| usize::try_from(n).unwrap_or(0);
        let mut team: Vec<ScrapTarget> = io
            .facilities(site, 0x24)
            .into_iter()
            .take(take(r.values[7] - r.values[9]))
            .collect();
        let batteries = take(r.values[11] - r.values[13]);
        let mut rest = io.facilities(site, 0x22);
        rest.extend(io.facilities(site, 0x23));
        team.extend(rest.into_iter().take(batteries));
        (!team.is_empty()).then_some(AIAction::Scrap { team })
    }

    /// `FUN_004da4d0`: 90% of `+0x24` percent of the capacity, less the
    /// side's defense maintenance (`+0x1dc`); 0 when the side's free
    /// maintenance falls short of it.
    fn defense_budget(io: &Inputs<'_>) -> i32 {
        let t = io.tallies;
        let budget = PERCENT * t.capacity / 100 * 90 / 100 - t.defenses;
        if t.capacity - t.requirement < budget {
            0
        } else {
            budget
        }
    }

    /// `FUN_004d76f0`: a system short of defenses into `+0x54`: Coruscant
    /// when it lacks shields, else the sector short of defenses and nearest
    /// the yard, then its system shortest of them (`v[37]` below 0). With
    /// none, the side's shield goal `+0x374` may rise (`FUN_00418240(4)`).
    fn find_short(&mut self, io: &mut Inputs<'_>) -> bool {
        let coruscant = io
            .world
            .systems
            .iter()
            .find(|(_, s)| s.dat_id.raw() == CORUSCANT)
            .map(|(k, _)| k);
        let capital = |io: &Inputs<'_>| {
            coruscant
                .and_then(|c| io.galaxy.system(c).copied())
                .filter(|r| r.kind & 1 != 0 && r.state & 2 == 0)
        };
        self.site = Target::None;
        if let Some(r) = capital(io) {
            self.site = Target::System(r.system);
            if r.state & 0x4000_0000 != 0 || r.values[7] < S36C {
                return true;
            }
        }
        let mut found = false;
        let origin = match self.yard {
            Yard::System(s) | Yard::Manager(s) => Some(s),
            Yard::None => None,
        };
        let bit = [Mask::new(0x4000, 0), ANY, ANY];
        let near = origin.map_or_else(Vec::new, |o| {
            io.galaxy
                .sectors_by_distance(o, bit, Order::Descending, io.coin)
        });
        let lists = vec![io.sectors(bit, 0x1c, Order::Ascending), near];
        let sector = pick(&lists);
        let masks = [Mask::new(0x4000, 0x400_0003), ANY, Mask::new(1, 0)];
        let lists = vec![io.systems(sector, masks, 0x25, Order::Descending)];
        self.site = pick(&lists).map_or(Target::None, Target::System);
        if self.site.system().is_none() {
            if let Some(r) = capital(io).filter(|r| r.values[37] < 0) {
                self.site = Target::System(r.system);
                found = true;
            }
        }
        if self.site.system().is_none() {
            let near = origin.map_or_else(Vec::new, |o| {
                io.galaxy.sectors_by_distance(
                    o,
                    [Mask::new(0x8000, 0), ANY, ANY],
                    Order::Descending,
                    io.coin,
                )
            });
            let sector = pick(&[near]);
            let masks = [Mask::new(0x4000, 3), ANY, Mask::new(1, 0)];
            let lists = vec![io.systems(sector, masks, 0x25, Order::Descending)];
            self.site = pick(&lists).map_or(Target::None, Target::System);
        }
        match self.site.system() {
            Some(site) => {
                if io.galaxy.system(site).is_some_and(|r| r.values[37] < 0) {
                    found = true;
                }
            }
            None => {
                if Self::defense_budget(io) > 0 && io.controller.shield_goal < 2 {
                    io.controller.shield_goal += 1;
                }
            }
        }
        found
    }

    /// `FUN_004d7cf0`: at `+0x54`, what stands in the way of defenses
    /// (`FUN_004e31b0`); with nothing and no free energy, one refinery,
    /// mine, shipyard, training facility or construction yard, the first
    /// the tallies and the system allow.
    fn scrap_for_defenses(&self, io: &mut Inputs<'_>) -> Option<AIAction> {
        let site = self.site.system()?;
        let r = io.galaxy.system(site).copied()?;
        if let Some(order) = scrap_in_the_way(io, self.site) {
            return Some(order);
        }
        if r.values[15] != 0 {
            return None;
        }
        let t = io.tallies;
        let mut families = Vec::new();
        if t.mines < t.refineries || r.values[2] == 0 {
            families.push(0x2d);
        }
        if t.refineries <= t.mines || r.values[3] == 0 {
            families.push(0x2c);
        }
        families.extend([0x28, 0x29, 0x2a]);
        let member = families
            .into_iter()
            .find_map(|family| io.facilities(site, family).into_iter().next())?;
        Some(AIAction::Scrap { team: vec![member] })
    }

    /// `FUN_004d8460`: `0xb` to build one defense at `+0x54`, 7 to scrap
    /// one when defenses cost more than the budget, else 9.
    fn plan_defenses(&mut self, io: &mut Inputs<'_>) -> u32 {
        self.count = 0;
        let budget = Self::defense_budget(io);
        if budget < 0 {
            return 7;
        }
        if budget == 0 {
            return 9;
        }
        let Some(r) = self
            .site
            .system()
            .and_then(|s| io.galaxy.system(s))
            .copied()
        else {
            return 9;
        };
        if r.values[15] <= 0 {
            return 9;
        }
        // FUN_00473250: key 0x10000 is the record's shields (+0x34), key
        // 0x40000 its attack (+0x30).
        let (flags, key) = if r.state & 0x4000_0000 != 0 {
            (crate::ai_records::CLASS_DEFENSE_25, 1)
        } else if r.state & 0x8000 != 0 {
            (crate::ai_records::CLASS_DEFENSE_24, 1)
        } else {
            (crate::ai_records::CLASS_DEFENSE_22, 0)
        };
        let maintenance;
        (self.class, maintenance) = best_class(io, flags, key, Order::Ascending);
        if self.class.is_some() && maintenance != 0 && maintenance <= budget {
            self.count = 1;
            return 0xb;
        }
        9
    }

    /// `FUN_004d8520`: one battery or shield to scrap, at the system with
    /// the greatest surplus, else at one with the most of them.
    fn scrap_one_defense(&mut self, io: &mut Inputs<'_>) -> Option<AIAction> {
        self.site = Self::surplus_site(io);
        for (bits, forbid) in [(0x2_00e0, 0x3e0_0000), (0x2_0000, 0)] {
            if self.site.system().is_some() {
                break;
            }
            let lists = vec![io.sectors([Mask::new(bits, 0), ANY, ANY], 3, Order::Ascending)];
            let sector = pick(&lists);
            let masks = [Mask::new(0, forbid), ANY, Mask::new(1, 0x4000_0000)];
            let lists = vec![
                io.systems(sector, masks, 7, Order::Ascending),
                io.systems(sector, masks, 0xb, Order::Ascending),
            ];
            self.site = pick(&lists).map_or(Target::None, Target::System);
        }
        let site = self.site.system()?;
        let member = [0x22, 0x23, 0x24]
            .into_iter()
            .find_map(|family| io.facilities(site, family).into_iter().next())?;
        Some(AIAction::Scrap { team: vec![member] })
    }

    /// `FUN_004d9b00`: every shipyard, training facility and construction
    /// yard of the side at `+0x54`.
    fn clear_site(&self, io: &mut Inputs<'_>) -> Option<AIAction> {
        let site = self.site.system()?;
        let mut team = io.facilities(site, 0x28);
        team.extend(io.facilities(site, 0x29));
        team.extend(io.facilities(site, 0x2a));
        (!team.is_empty()).then_some(AIAction::Scrap { team })
    }

    /// `FUN_004d8ad0`: the target, a site with room and the fewest
    /// construction yards, or a new site. The second lists join only when
    /// the module has sites.
    fn choose_target(&mut self, io: &mut Inputs<'_>) -> bool {
        self.target = Target::None;
        self.room = 0;
        let mut best = 1000;
        for &site in &self.sites {
            let Some(r) = io.galaxy.system(site) else {
                continue;
            };
            if r.kind & 1 != 0 && r.state & 3 == 0 && r.values[0x33] > 0 && r.values[6] < best {
                self.room = r.values[0x33];
                best = r.values[6];
                self.target = Target::System(site);
            }
        }
        if self.target.system().is_some() {
            return true;
        }
        let listed = !self.sites.is_empty();
        let bit = [Mask::new(0x80, 0), ANY, ANY];
        let mut lists = vec![io.sectors(bit, 0x15, Order::Ascending)];
        if listed {
            lists.push(io.sectors(bit, 4, Order::Descending));
        }
        let sector = pick(&lists);
        let masks = [Mask::new(0, 0x380_0003), ANY, Mask::new(1, 0x4000_0000)];
        let mut lists = vec![io.systems(sector, masks, 6, Order::Ascending)];
        if listed {
            lists.push(io.systems(sector, masks, 0x33, Order::Ascending));
        }
        let Some(system) = pick(&lists) else {
            return false;
        };
        self.target = Target::System(system);
        if io.galaxy.system(system).is_some() {
            self.sites.push(system);
            io.set_mark(system, MARK_BALANCE, 0);
        }
        true
    }

    /// `FUN_004d8db0`: borrow the idle construction yard of module 1's
    /// site nearest the target, and make the target a site.
    fn borrow_yard(&mut self, io: &mut Inputs<'_>) -> bool {
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
        let Some(sector) = pick(&[sectors]) else {
            return false;
        };
        let systems = io.galaxy.systems_by_distance(
            sector,
            target,
            [Mask::new(0x80_0800, 3), ANY, Mask::new(1, 0)],
            Order::Descending,
            io.coin,
        );
        let Some(system) = pick(&[systems]) else {
            return false;
        };
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
        if !self.sites.contains(&target) && io.galaxy.system(target).is_some() {
            self.sites.push(target);
            io.set_mark(target, MARK_BALANCE, 0);
        }
        true
    }

    /// `FUN_004d9520`: `0xb` to build construction yards, 8 to scrap one
    /// when the sites hold more than the side wants, else 0.
    fn plan_yards(&mut self, io: &mut Inputs<'_>) -> u32 {
        self.count = 0;
        let budget = yard_budget(io, &self.sites);
        if budget < 0 {
            return 8;
        }
        if budget == 0 {
            return 0;
        }
        let maintenance;
        (self.class, maintenance) = yard_class(io);
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
        0xb
    }
}

/// `FUN_004da530`: 10% of `+0x24` percent of the capacity, less the
/// sites' construction yard maintenance (`v[36]`); 0 when the side's free
/// maintenance falls short of it.
fn yard_budget(io: &Inputs<'_>, sites: &[SystemKey]) -> i32 {
    let held: i32 = sites
        .iter()
        .filter_map(|&s| io.galaxy.system(s))
        .map(|r| r.values[36])
        .sum();
    let t = io.tallies;
    let budget = PERCENT * t.capacity / 100 * 10 / 100 - held;
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
    use crate::ai_module1::tests::{
        add_yard, class, fixture, record, records_of, system, Fixture, ROOMY,
    };
    use crate::ai_module1::Tallies;
    use crate::ai_records::{Galaxy, SystemRecord};
    use crate::dat::Faction;
    use crate::manufacturing::{BuildableKind, ProductionArea};
    use crate::research::ResearchState;
    use crate::world::{DefenseFacilityInstance, FacilityRef};

    fn with_io<R>(
        f: &Fixture,
        state: &mut AIState,
        galaxy: &mut Galaxy,
        tallies: Tallies,
        run: impl FnOnce(&mut Module3, &mut Inputs<'_>) -> R,
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
        run(&mut state.module3, &mut io)
    }

    fn step(f: &Fixture, state: &mut AIState, tallies: Tallies) -> Step {
        let mut galaxy = records_of(f, state);
        with_io(f, state, &mut galaxy, tallies, |m, io| m.step(io))
    }

    /// Runs `run` on records that `edit` changed first.
    fn edited<R>(
        f: &Fixture,
        state: &mut AIState,
        tallies: Tallies,
        edit: impl FnOnce(&mut Galaxy),
        run: impl FnOnce(&mut Module3, &mut Inputs<'_>) -> R,
    ) -> R {
        let mut galaxy = records_of(f, state);
        edit(&mut galaxy);
        with_io(f, state, &mut galaxy, tallies, run)
    }

    fn defense_class(f: &mut Fixture, raw: u32, attack: u32, shields: u32, upkeep: u32) -> DatId {
        let id = class(&mut f.world, raw, upkeep, 0, 0);
        let c = f.world.buildable_classes.get_mut(&id).unwrap();
        c.stats.attack_strength = attack;
        c.stats.shield_strength = shields;
        id
    }

    fn add_defense(
        world: &mut crate::world::GameWorld,
        at: SystemKey,
        class: DatId,
    ) -> ScrapTarget {
        let key = world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: class,
            side: Faction::Alliance,
        });
        world.systems[at].defense_facilities.push(key);
        ScrapTarget::Facility(FacilityRef::Defense(key))
    }

    fn manufacturing(f: &Fixture, at: SystemKey) -> Vec<ScrapTarget> {
        f.world.systems[at]
            .manufacturing_facilities
            .iter()
            .map(|&k| ScrapTarget::Facility(FacilityRef::Manufacturing(k)))
            .collect()
    }

    fn rec(galaxy: &mut Galaxy, s: SystemKey) -> &mut SystemRecord {
        record(galaxy, s)
    }

    #[test]
    fn the_cycles_hand_on_from_surplus_to_defenses_to_yards() {
        // FUN_004d6d30: any other +0x38 starts 0x3ed at state 0.
        // FUN_004d6da0: 0 -> 1, no surplus (FUN_004d71e0): 0x3ee.
        // FUN_004d6e70: 0 -> 4, no site (FUN_004d7ac0): 0x3ec.
        // FUN_004d7060: 0 -> 1, a target (FUN_004d8ad0) -> 4, the yards
        // are not module 3's: 0xd, which ends with 0x3ed, complete.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        let mut trail = Vec::new();
        for _ in 0..12 {
            let s = step(&f, &mut state, ROOMY);
            trail.push((state.module3.phase, state.module3.state, s.complete));
            if s.complete {
                break;
            }
        }
        assert_eq!(
            trail,
            [
                (SURPLUS_CYCLE, 0, false),
                (SURPLUS_CYCLE, 1, false),
                (DEFENSE_CYCLE, 0, false),
                (DEFENSE_CYCLE, 4, false),
                (YARD_CYCLE, 0, false),
                (YARD_CYCLE, 1, false),
                (YARD_CYCLE, 4, false),
                (YARD_CYCLE, 0xd, false),
                (SURPLUS_CYCLE, 0, true),
            ]
        );
    }

    #[test]
    fn a_defense_surplus_is_found_and_scrapped_beyond_the_goals() {
        // FUN_004d71e0: sector +0x24 0x10000 by 0x1d, its own system by
        // v[37] (order 1); success when v[37] > 0. FUN_004d7330: v[7] -
        // v[9] of 0x24, then v[11] - v[13] batteries, 0x22 before 0x23;
        // the cycle then ends with 0x3ee, complete without raising.
        let mut f = fixture();
        let shield = defense_class(&mut f, 0x2400_0001, 0, 40, 12);
        let gun = defense_class(&mut f, 0x2200_0001, 2000, 0, 4);
        let ion = defense_class(&mut f, 0x2300_0001, 800, 0, 7);
        let s1 = add_defense(&mut f.world, f.empty, shield);
        let s2 = add_defense(&mut f.world, f.empty, shield);
        add_defense(&mut f.world, f.empty, shield);
        let i1 = add_defense(&mut f.world, f.empty, ion);
        add_defense(&mut f.world, f.empty, ion);
        let g1 = add_defense(&mut f.world, f.empty, gun);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.phase = SURPLUS_CYCLE;
        state.module3.state = 1;
        let surplus = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x1_0000;
            let r = rec(g, f.empty);
            r.values[37] = 4;
            r.values[7] = 3;
            r.values[9] = 1;
            r.values[11] = 3;
            r.values[13] = 1;
        };
        let s = edited(&f, &mut state, ROOMY, surplus, |m, io| m.step(io));
        assert_eq!(s, Step::pending());
        assert_eq!(state.module3.state, 2);
        assert_eq!(state.module3.site, Target::System(f.empty));

        let s = edited(&f, &mut state, ROOMY, surplus, |m, io| m.step(io));
        assert_eq!(
            s,
            Step {
                order: Some(AIAction::Scrap {
                    team: vec![s1, s2, g1, i1]
                }),
                complete: true,
                raise: false,
            }
        );
        assert_eq!(
            (state.module3.phase, state.module3.state),
            (DEFENSE_CYCLE, 0)
        );

        // No surplus at the system found: not taken.
        state.module3.phase = SURPLUS_CYCLE;
        state.module3.state = 1;
        let none = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x1_0000;
            rec(g, f.empty).values[37] = 0;
            rec(g, f.yard).values[37] = 0;
        };
        edited(&f, &mut state, ROOMY, none, |m, io| m.step(io));
        assert_eq!(
            (state.module3.phase, state.module3.state),
            (DEFENSE_CYCLE, 0)
        );
    }

    #[test]
    fn the_defense_budget_is_nine_tenths_of_nine_percent_less_the_upkeep() {
        // FUN_004da4d0: 9 * capacity / 100 * 90 / 100 - +0x1dc; 0 when
        // capacity - requirement falls below it. FUN_004da530: the same
        // with 10 / 100, less the sites' v[36].
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.sites.push(f.yard);
        let t = |capacity, requirement, defenses| Tallies {
            capacity,
            requirement,
            defenses,
            ..ROOMY
        };
        let run = |state: &mut AIState, tallies| {
            edited(
                &f,
                state,
                tallies,
                |_| {},
                |m, io| (Module3::defense_budget(io), yard_budget(io, &m.sites)),
            )
        };
        // 9% of 1000 = 90, 90% = 81; 10% of 90 = 9, less the yard's 10.
        assert_eq!(run(&mut state, t(1000, 0, 30)), (51, -1));
        assert_eq!(run(&mut state, t(1000, 0, 90)), (-9, -1));
        // 9% of 2000 = 180: 162 and 18 - 10.
        assert_eq!(run(&mut state, t(2000, 0, 2)), (160, 8));
        // Free maintenance 100 is below 160, but not below 8.
        assert_eq!(run(&mut state, t(2000, 1900, 2)), (0, 8));
        assert_eq!(run(&mut state, t(2000, 1995, 2)), (0, 0));
        // 9% of 111 = 9 (integer), 90% = 8.
        assert_eq!(run(&mut state, t(111, 0, 0)).0, 8);
    }

    #[test]
    fn coruscant_short_of_shields_comes_first() {
        // FUN_004d76f0: Coruscant (0x90000109), own and not blockaded,
        // with fewer 0x24 defenses than side +0x36c (2), or +0x28
        // 0x40000000.
        // Its record (FUN_00433e40, id 0x109) is short of defenses too;
        // the edits clear that so only the first check can find it.
        let mut f = fixture();
        f.world.systems[f.empty].dat_id = DatId::new(CORUSCANT);
        let mut state = AIState::new(AiFaction::Alliance);
        let calm = |g: &mut Galaxy, v7: i32, bits: u32| {
            g.sectors[0].candidates &= !(0x4000 | 0x8000);
            let r = rec(g, f.empty);
            r.values[7] = v7;
            r.values[37] = 0;
            r.state = (r.state & !0xc000) | bits;
        };
        let found = edited(
            &f,
            &mut state,
            ROOMY,
            |g| calm(g, 1, 0),
            |m, io| m.find_short(io),
        );
        assert!(found);
        assert_eq!(state.module3.site, Target::System(f.empty));

        let found = edited(
            &f,
            &mut state,
            ROOMY,
            |g| calm(g, 2, 0),
            |m, io| m.find_short(io),
        );
        assert!(!found);

        let marked = |g: &mut Galaxy| calm(g, 2, 0x4000_0000);
        assert!(edited(&f, &mut state, ROOMY, marked, |m, io| m.find_short(io)));

        for change in [
            (|r: &mut SystemRecord| r.state |= 2) as fn(&mut SystemRecord),
            |r| r.kind &= !1,
        ] {
            let edit = |g: &mut Galaxy| {
                calm(g, 1, 0);
                change(rec(g, f.empty));
            };
            let found = edited(&f, &mut state, ROOMY, edit, |m, io| m.find_short(io));
            assert!(!found);
            assert_eq!(state.module3.site, Target::None);
        }
    }

    #[test]
    fn a_system_short_of_defenses_is_found_near_the_yard() {
        // FUN_004d76f0: sectors +0x24 0x4000 by 0x1c merged with the
        // nearest to +0x50; their systems with +0x28 0x4000 and +0x30 1,
        // forbidding 0x4000003, by v[37] (order 2); found when v[37] < 0.
        // With none there, the sector nearest with 0x8000 and its system
        // with 0x4000, forbidding 3.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.yard = Yard::Manager(f.yard);
        // Both records carry +0x28 0x4000000 (FUN_00433e40); the first
        // query forbids it.
        let short = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x4000;
            let r = rec(g, f.empty);
            r.state = (r.state | 0x4000) & !0x400_0000;
            r.values[37] = -1;
        };
        assert!(edited(&f, &mut state, ROOMY, short, |m, io| m.find_short(io)));
        assert_eq!(state.module3.site, Target::System(f.empty));

        // With 0x4000000 only the second query (forbidding 3) finds it.
        let second = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x4000 | 0x8000;
            let r = rec(g, f.empty);
            r.state |= 0x4000 | 0x400_0000;
            r.values[37] = -2;
        };
        assert!(edited(&f, &mut state, ROOMY, second, |m, io| m.find_short(io)));
        assert_eq!(state.module3.site, Target::System(f.empty));

        // Found but not short: no.
        let even = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x4000;
            let r = rec(g, f.empty);
            r.state = (r.state | 0x4000) & !0x400_0000;
            r.values[37] = 0;
        };
        assert!(!edited(&f, &mut state, ROOMY, even, |m, io| m.find_short(io)));
        assert_eq!(state.module3.site, Target::System(f.empty));
    }

    #[test]
    fn with_nothing_short_the_shield_goal_rises_once() {
        // FUN_004d76f0: no system, FUN_004da4d0 > 0 and side +0x374 < 2:
        // FUN_00418240(4) raises +0x374.
        let mut f = fixture();
        f.world.systems[f.yard].dat_id = DatId::new(CORUSCANT);
        let mut state = AIState::new(AiFaction::Alliance);
        // Coruscant is the yard, with its shields full and nothing short.
        let full = |g: &mut Galaxy| {
            g.sectors[0].candidates &= !(0x4000 | 0x8000);
            let r = rec(g, f.yard);
            r.values[7] = 2;
            r.values[37] = 0;
        };
        let t = |defenses| Tallies { defenses, ..ROOMY };
        for (defenses, goal) in [(81, 1), (80, 2), (0, 2)] {
            edited(&f, &mut state, t(defenses), full, |m, io| m.find_short(io));
            assert_eq!(state.controller.shield_goal, goal, "upkeep {defenses}");
        }
        // Coruscant short by v[37] is the second choice.
        let short = |g: &mut Galaxy| {
            g.sectors[0].candidates &= !(0x4000 | 0x8000);
            let r = rec(g, f.yard);
            r.values[7] = 2;
            r.values[37] = -1;
        };
        assert!(edited(&f, &mut state, ROOMY, short, |m, io| m.find_short(io)));
        assert_eq!(state.module3.site, Target::System(f.yard));
    }

    #[test]
    fn the_raised_goal_reaches_the_records() {
        // FUN_00433e40: a marked own system whose energy v[1] is at most
        // +0x36c + +0x370 (3) and above +0x374 takes +0x374 as its shield
        // goal v[9].
        let mut f = fixture();
        f.world.systems[f.empty].total_energy = 3;
        let classes = crate::ai_records::ClassRecords::build(&f.world);
        let fleets = std::collections::HashMap::new();
        let economy = crate::economy::EconomyState::default();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = crate::manufacturing::ManufacturingState::new();
        let research = ResearchState::new();
        let goal = |shield_goal| {
            let inputs = crate::ai_records::SystemInputs {
                world: &f.world,
                classes: &classes,
                fleets: &fleets,
                person: &|_| None,
                economy: &economy,
                uprisings: &uprisings,
                manufacturing: &manufacturing,
                research: &research,
                is_alliance: true,
                minor_classes: 0,
                shield_goal,
            };
            let r = SystemRecord::build(&inputs, f.empty, MARK_MINES).unwrap();
            (r.values[1], r.values[9])
        };
        assert_eq!(goal(1), (3, 1));
        assert_eq!(goal(2), (3, 2));
    }

    #[test]
    fn what_blocks_the_defenses_is_scrapped_or_one_facility_when_energy_is_out() {
        // FUN_004d7cf0: the FUN_004e31b0 team; with none and v[15] == 0,
        // one refinery when +0x1d0 < +0x1d4 or v[2] == 0, else one mine
        // when +0x1d4 <= +0x1d0 or v[3] == 0, else one shipyard, training
        // facility or construction yard.
        let mut f = fixture();
        let refinery = class(&mut f.world, REFINERY_CLASS, 0, 0, 0);
        let mine = class(&mut f.world, MINE_CLASS, 0, 0, 0);
        let add = |f: &mut Fixture, c: DatId| {
            let key =
                f.world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: c,
                        side: Faction::Alliance,
                        is_mine: c.family() == 0x2c,
                    });
            f.world.systems[f.yard].production_facilities.push(key);
            ScrapTarget::Facility(FacilityRef::Production(key))
        };
        let r1 = add(&mut f, refinery);
        let m1 = add(&mut f, mine);
        let mut state = AIState::new(AiFaction::Alliance);
        // The yard's own construction yard, unmarked: in the way.
        state.module3.site = Target::System(f.yard);
        let order = edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| m.scrap_for_defenses(io),
        );
        assert_eq!(
            order,
            Some(AIAction::Scrap {
                team: manufacturing(&f, f.yard)
            })
        );

        state.yard_marks.insert(f.yard, MARK_MINES);
        let run = |state: &mut AIState, t: Tallies, v2: i32, v3: i32, v15: i32| {
            edited(
                &f,
                state,
                t,
                |g| {
                    let r = rec(g, f.yard);
                    r.values[2] = v2;
                    r.values[3] = v3;
                    r.values[15] = v15;
                },
                |m, io| m.scrap_for_defenses(io),
            )
        };
        let t = |mines, refineries| Tallies {
            mines,
            refineries,
            ..ROOMY
        };
        let one = |x| Some(AIAction::Scrap { team: vec![x] });
        assert_eq!(run(&mut state, t(1, 1), 1, 1, 1), None);
        assert_eq!(run(&mut state, t(1, 2), 1, 1, 0), one(r1));
        assert_eq!(run(&mut state, t(1, 1), 0, 1, 0), one(r1));
        assert_eq!(run(&mut state, t(1, 1), 1, 1, 0), one(m1));
        assert_eq!(run(&mut state, t(2, 1), 1, 1, 0), one(m1));
        // Without a refinery to take, the mine needs +0x1d4 <= +0x1d0 or
        // v[3] == 0; failing both, the construction yard goes.
        f.world.systems[f.yard]
            .production_facilities
            .retain(|&k| ScrapTarget::Facility(FacilityRef::Production(k)) != r1);
        let run = |state: &mut AIState, t: Tallies, v3: i32| {
            edited(
                &f,
                state,
                t,
                |g| {
                    let r = rec(g, f.yard);
                    r.values[2] = 1;
                    r.values[3] = v3;
                    r.values[15] = 0;
                },
                |m, io| m.scrap_for_defenses(io),
            )
        };
        assert_eq!(run(&mut state, t(1, 2), 0), one(m1));
        assert_eq!(
            run(&mut state, t(1, 2), 1),
            one(manufacturing(&f, f.yard)[0])
        );
        state.module3.site = Target::None;
        assert_eq!(run(&mut state, t(1, 1), 0), None);
    }

    #[test]
    fn the_defense_class_follows_what_the_site_lacks() {
        // FUN_004d8460: 0x40000 by shields when +0x28 has 0x40000000;
        // 0x20000 by shields with 0x8000; else 0x80000 by attack
        // (FUN_00473250), order 1: the strongest. One, when its upkeep is
        // at most the budget: 0xb.
        let mut f = fixture();
        let weak_gun = defense_class(&mut f, 0x2200_0001, 800, 0, 4);
        let gun = defense_class(&mut f, 0x2300_0001, 5000, 0, 4);
        let shield = defense_class(&mut f, 0x2400_0001, 0, 80, 7);
        defense_class(&mut f, 0x2400_0002, 0, 40, 7);
        let ion = defense_class(&mut f, 0x2500_0001, 9000, 30, 20);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.site = Target::System(f.empty);
        for (bits, class) in [(0, gun), (0x8000, shield), (0x4000_0000, ion)] {
            let next = edited(
                &f,
                &mut state,
                ROOMY,
                |g| rec(g, f.empty).state |= bits,
                |m, io| m.plan_defenses(io),
            );
            assert_eq!((next, state.module3.count), (0xb, 1), "{bits:#x}");
            assert_eq!(state.module3.class, Some(class));
        }
        let _ = weak_gun;
        // Upkeep 20 over a budget of 11: 9.
        let tight = Tallies {
            defenses: 70,
            ..ROOMY
        };
        let next = edited(
            &f,
            &mut state,
            tight,
            |g| rec(g, f.empty).state |= 0x4000_0000,
            |m, io| m.plan_defenses(io),
        );
        assert_eq!((next, state.module3.count), (9, 0));
        // At the budget: built.
        let exact = Tallies {
            defenses: 61,
            ..ROOMY
        };
        let next = edited(
            &f,
            &mut state,
            exact,
            |g| rec(g, f.empty).state |= 0x4000_0000,
            |m, io| m.plan_defenses(io),
        );
        assert_eq!(next, 0xb);
    }

    #[test]
    fn the_defense_plan_scraps_over_budget_and_waits_without_energy_or_class() {
        // FUN_004d8460: w < 0 gives 7; w == 0, no record, no free energy
        // (v[15]) or no class gives 9.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.site = Target::System(f.empty);
        let t = |defenses| Tallies { defenses, ..ROOMY };
        let plan = |state: &mut AIState, t, v15: i32| {
            edited(
                &f,
                state,
                t,
                |g| rec(g, f.empty).values[15] = v15,
                |m, io| m.plan_defenses(io),
            )
        };
        assert_eq!(plan(&mut state, t(82), 5), 7);
        assert_eq!(plan(&mut state, t(81), 5), 9);
        assert_eq!(plan(&mut state, t(0), 0), 9);
        // No defense class at all.
        assert_eq!(plan(&mut state, t(0), 5), 9);
        assert_eq!(state.module3.class, None);
        state.module3.site = Target::None;
        assert_eq!(plan(&mut state, t(0), 5), 9);
    }

    #[test]
    fn one_defense_is_scrapped_where_there_are_most() {
        // FUN_004d8520: the surplus site, else sectors 0x200e0 then
        // 0x20000 by key 3 and their systems by v[7] and v[0xb]; the
        // side's first 0x22, else 0x23, else 0x24 there.
        let mut f = fixture();
        let gun = defense_class(&mut f, 0x2200_0001, 2000, 0, 4);
        let ion = defense_class(&mut f, 0x2300_0001, 800, 0, 7);
        let shield = defense_class(&mut f, 0x2400_0001, 0, 40, 12);
        let s = add_defense(&mut f.world, f.empty, shield);
        let i = add_defense(&mut f.world, f.empty, ion);
        let g = add_defense(&mut f.world, f.empty, gun);
        let mut state = AIState::new(AiFaction::Alliance);
        let surplus = |gx: &mut Galaxy| {
            gx.sectors[0].candidates |= 0x1_0000;
            rec(gx, f.empty).values[37] = 1;
            rec(gx, f.yard).values[37] = -1;
        };
        let order = edited(&f, &mut state, ROOMY, surplus, |m, io| {
            m.scrap_one_defense(io)
        });
        assert_eq!(order, Some(AIAction::Scrap { team: vec![g] }));
        assert_eq!(state.module3.site, Target::System(f.empty));

        for bits in [0x2_00e0, 0x2_0000] {
            let edit = |gx: &mut Galaxy| {
                gx.sectors[0].candidates = bits;
                rec(gx, f.yard).kind &= !1;
                rec(gx, f.empty).values[7] = 3;
            };
            let order = edited(&f, &mut state, ROOMY, edit, |m, io| m.scrap_one_defense(io));
            assert_eq!(order, Some(AIAction::Scrap { team: vec![g] }), "{bits:#x}");
        }
        // The 0x200e0 query forbids a marked system (0x3e00000): the yard,
        // marked and holding more, is passed over for the empty system.
        let gy = add_defense(&mut f.world, f.yard, gun);
        state.yard_marks.insert(f.yard, crate::ai::MARK_SHIPYARD);
        let marked = |gx: &mut Galaxy| {
            gx.sectors[0].candidates = 0x2_00e0;
            for (sys, n) in [(f.yard, 5), (f.empty, 3)] {
                let r = rec(gx, sys);
                r.values[7] = n;
                r.values[0xb] = n;
            }
        };
        let order = edited(&f, &mut state, ROOMY, marked, |m, io| {
            m.scrap_one_defense(io)
        });
        assert_eq!(order, Some(AIAction::Scrap { team: vec![g] }));
        // The 0x20000 query does not: the yard.
        let unmarked_query = |gx: &mut Galaxy| {
            marked(gx);
            gx.sectors[0].candidates = 0x2_0000;
        };
        let order = edited(&f, &mut state, ROOMY, unmarked_query, |m, io| {
            m.scrap_one_defense(io)
        });
        assert_eq!(order, Some(AIAction::Scrap { team: vec![gy] }));

        f.world.systems[f.empty]
            .defense_facilities
            .retain(|&k| ScrapTarget::Facility(FacilityRef::Defense(k)) != g);
        let mut state = AIState::new(AiFaction::Alliance);
        let order = edited(&f, &mut state, ROOMY, surplus, |m, io| {
            m.scrap_one_defense(io)
        });
        assert_eq!(order, Some(AIAction::Scrap { team: vec![i] }));
        f.world.systems[f.empty]
            .defense_facilities
            .retain(|&k| ScrapTarget::Facility(FacilityRef::Defense(k)) != i);
        let order = edited(&f, &mut state, ROOMY, surplus, |m, io| {
            m.scrap_one_defense(io)
        });
        assert_eq!(order, Some(AIAction::Scrap { team: vec![s] }));
    }

    /// The fixture with `yard` as module 3's site with an idle yard, and
    /// a defense class to build.
    fn defense_site() -> (Fixture, AIState, DatId) {
        let mut f = fixture();
        let gun = defense_class(&mut f, 0x2200_0001, 2000, 0, 4);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.sites.push(f.yard);
        state.yard_marks.insert(f.yard, MARK_BALANCE);
        (f, state, gun)
    }

    #[test]
    fn the_defense_cycle_builds_one_defense_where_it_falls_short() {
        // FUN_004d6e70: 0 -> 4 (FUN_004d7ac0 keeps the site, its manager
        // into +0x50) -> 1 (FUN_004d76f0: the empty system, v[37] < 0) ->
        // 6 (FUN_004d7cf0: nothing in the way) -> 5 (FUN_004d8460: one
        // battery) -> 0xb (Destination, complete, raise) -> 0xc (Build) ->
        // end with 0x3ec.
        let (f, mut state, gun) = defense_site();
        state.module3.phase = DEFENSE_CYCLE;
        // The site's own record, marked, is short too (FUN_00433e40); the
        // edit leaves only the empty system short.
        let short = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x4000;
            let r = rec(g, f.empty);
            r.state = (r.state | 0x4000) & !(0x400_0000 | 0x8000);
            r.values[37] = -1;
            let y = rec(g, f.yard);
            y.state &= !0x4000;
            y.values[37] = 0;
        };
        let mut trail = Vec::new();
        let mut order = None;
        for _ in 0..8 {
            let s = edited(&f, &mut state, ROOMY, short, |m, io| m.step(io));
            trail.push((state.module3.state, s.complete, s.raise));
            if s.order.is_some() {
                order = s.order;
                break;
            }
        }
        assert_eq!(
            trail,
            [
                (4, false, false),
                (1, false, false),
                (6, false, false),
                (5, false, false),
                (0xb, false, false),
                (0xc, true, true),
            ]
        );
        assert_eq!(
            order,
            Some(AIAction::SetDestination {
                system: f.yard,
                area: ProductionArea::ConstructionYard,
                destination: f.empty,
            })
        );
        let s = step(&f, &mut state, ROOMY);
        assert!(s.complete && !s.raise);
        let Some(AIAction::Build {
            system,
            kind,
            count,
            ..
        }) = s.order
        else {
            panic!("{s:?}");
        };
        assert_eq!((system, count), (f.yard, 1));
        assert!(matches!(kind, BuildableKind::DefenseFacility(b) if b.class == gun));
        assert_eq!((state.module3.phase, state.module3.state), (YARD_CYCLE, 0));
    }

    #[test]
    fn without_a_short_system_the_cycle_builds_a_mine_or_refinery() {
        // FUN_004d6e70 state 1 -> 9: the refinery when +0x1d4 < +0x1d0
        // (FUN_004d99d0: sector 0x200, system 0x100 with +0x30 1), else
        // the mine (FUN_004d98a0: sector 0x100, system 0x80), nearest the
        // yard, forbidding 0x3e00003; +0x60 = 1; state 10 (FUN_004d9b00)
        // scraps nothing at the empty system; 0xb, 0xc.
        for (mines, refineries, class) in [(2, 1, REFINERY_CLASS), (1, 1, MINE_CLASS)] {
            let (f, mut state, _) = defense_site();
            state.module3.phase = DEFENSE_CYCLE;
            state.module3.state = 9;
            state.module3.yard = Yard::Manager(f.yard);
            let t = Tallies {
                mines,
                refineries,
                ..ROOMY
            };
            let s = step(&f, &mut state, t);
            assert_eq!(s, Step::pending());
            assert_eq!(state.module3.state, 0xb);
            assert_eq!(state.module3.site, Target::System(f.empty));
            assert_eq!(state.module3.class, Some(DatId::new(class)));
            assert_eq!(state.module3.count, 1);
            let s = step(&f, &mut state, t);
            assert!(s.complete && s.raise);
            let s = step(&f, &mut state, t);
            let Some(AIAction::Build { kind, .. }) = s.order else {
                panic!("{s:?}");
            };
            assert_eq!(kind.class_dat_id(), Some(DatId::new(class)));
        }
    }

    #[test]
    fn a_failed_mine_site_still_runs_state_ten_into_the_yard_cycle() {
        // FUN_004d6e70 state 9 without a site: +0x38 = 0x3ec, +0x3c = 0,
        // then state 10 stores 0xb, so the yard cycle resumes at 0xb with
        // the yard's target +0x58 (none: the cycle ends with 0x3ed).
        let (f, mut state, _) = defense_site();
        state.module3.phase = DEFENSE_CYCLE;
        state.module3.state = 9;
        state.module3.yard = Yard::None;
        let s = step(&f, &mut state, ROOMY);
        assert_eq!(s, Step::pending());
        assert_eq!(
            (state.module3.phase, state.module3.state),
            (YARD_CYCLE, 0xb)
        );
        let s = step(&f, &mut state, ROOMY);
        assert_eq!(s, Step::complete());
        assert_eq!(
            (state.module3.phase, state.module3.state),
            (SURPLUS_CYCLE, 0)
        );
    }

    #[test]
    fn state_ten_clears_every_yard_at_the_site() {
        // FUN_004d9b00: every 0x28, 0x29 and 0x2a of the side at +0x54,
        // marked or not, complete and raise.
        let (mut f, mut state, _) = defense_site();
        let shipyard = class(&mut f.world, 0x2800_0001, 10, 3, 0);
        add_yard(&mut f.world, f.yard, shipyard, Faction::Alliance);
        state.module3.phase = DEFENSE_CYCLE;
        state.module3.state = 10;
        state.module3.site = Target::System(f.yard);
        let s = step(&f, &mut state, ROOMY);
        assert_eq!(
            s,
            Step {
                order: Some(AIAction::Scrap {
                    team: vec![manufacturing(&f, f.yard)[1], manufacturing(&f, f.yard)[0]]
                }),
                complete: true,
                raise: true,
            }
        );
        assert_eq!(state.module3.state, 0xb);
    }

    #[test]
    fn over_budget_the_cycle_scraps_one_defense_or_goes_on_to_mines() {
        // FUN_004d6e70 state 5 -> 7 for w < 0; state 7 (FUN_004d8520)
        // ends the cycle with an order, or goes to 9 without one.
        let (mut f, mut state, gun) = defense_site();
        let g = add_defense(&mut f.world, f.empty, gun);
        state.module3.phase = DEFENSE_CYCLE;
        state.module3.state = 5;
        state.module3.site = Target::System(f.empty);
        let over = Tallies {
            defenses: 100,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, over), Step::pending());
        assert_eq!(state.module3.state, 7);
        let surplus = |gx: &mut Galaxy| {
            gx.sectors[0].candidates |= 0x1_0000;
            rec(gx, f.empty).values[37] = 1;
            rec(gx, f.yard).values[37] = -1;
        };
        let s = edited(&f, &mut state, over, surplus, |m, io| m.step(io));
        assert_eq!(
            s,
            Step {
                order: Some(AIAction::Scrap { team: vec![g] }),
                complete: true,
                raise: false,
            }
        );
        assert_eq!((state.module3.phase, state.module3.state), (YARD_CYCLE, 0));

        let (f, mut state, _) = defense_site();
        state.module3.phase = DEFENSE_CYCLE;
        state.module3.state = 7;
        let s = edited(
            &f,
            &mut state,
            over,
            |g| g.sectors[0].candidates = 0,
            |m, io| m.step(io),
        );
        assert_eq!(s, Step::pending());
        assert_eq!(state.module3.state, 9);
    }

    #[test]
    fn the_target_s_second_lists_join_only_with_sites() {
        // FUN_004d8ad0: the sector list by key 4 (order 2) and the system
        // list by 0x33 join only when +0x6c is not empty (FUN_005f50e0).
        // With A (v[6] 1, room 5) and B (v[6] 0, room 9), key 6 alone
        // picks A; with key 0x33 the places tie and B, first, wins.
        let mut f = fixture();
        let b = system(&mut f.world, f.sector, 9);
        let pick_with = |state: &mut AIState| {
            let edit = |g: &mut Galaxy| {
                let a = rec(g, f.empty);
                a.values[6] = 1;
                a.values[0x33] = 5;
                let rb = rec(g, b);
                rb.values[6] = 0;
                rb.values[0x33] = 9;
                rec(g, f.yard).values[0x33] = 0;
            };
            assert!(edited(&f, state, ROOMY, edit, |m, io| m.choose_target(io)));
            state.module3.target
        };
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        assert_eq!(pick_with(&mut state), Target::System(f.empty));
        assert_eq!(state.module3.sites, vec![f.empty]);
        assert_eq!(state.yard_marks.get(&f.empty), Some(&MARK_BALANCE));
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.sites.push(f.yard);
        state.yard_marks.insert(f.yard, MARK_BALANCE);
        assert_eq!(pick_with(&mut state), Target::System(b));
    }

    #[test]
    fn the_target_is_a_calm_own_site_with_room_and_fewest_yards() {
        // FUN_004d8ad0's walk over +0x6c: own, calm, v[0x33] > 0, v[6]
        // below the best; +0x68 = v[0x33]. No site and no query: none,
        // and the yard cycle ends.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.sites = vec![f.yard, f.empty];
        let edit = |g: &mut Galaxy| {
            rec(g, f.yard).values[0x33] = 3;
            let e = rec(g, f.empty);
            e.values[0x33] = 4;
            e.values[6] = 0;
        };
        assert!(edited(&f, &mut state, ROOMY, edit, |m, io| m.choose_target(io)));
        assert_eq!(
            (state.module3.target, state.module3.room),
            (Target::System(f.empty), 4)
        );
        let changes: [fn(&mut SystemRecord); 5] = [
            |r| r.kind &= !1,
            |r| r.state |= 1,
            |r| r.state |= 2,
            |r| r.values[0x33] = 0,
            |r| r.values[6] = 1,
        ];
        for change in changes {
            let edit = |g: &mut Galaxy| {
                rec(g, f.yard).values[0x33] = 3;
                let e = rec(g, f.empty);
                e.values[0x33] = 4;
                e.values[6] = 0;
                change(e);
            };
            edited(&f, &mut state, ROOMY, edit, |m, io| m.choose_target(io));
            assert_eq!(
                (state.module3.target, state.module3.room),
                (Target::System(f.yard), 3)
            );
        }

        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.yard_marks.insert(f.empty, MARK_MINES);
        state.module3.phase = YARD_CYCLE;
        state.module3.state = 1;
        assert_eq!(step(&f, &mut state, ROOMY), Step::complete());
        assert_eq!(
            (state.module3.phase, state.module3.state),
            (SURPLUS_CYCLE, 0)
        );
    }

    #[test]
    fn with_the_token_module_three_borrows_module_one_s_nearest_yard() {
        // FUN_004d7060 state 4: clear +8 bit 1; with +0xc == 1,
        // FUN_004d8db0 takes the sector nearest the target with 0x1000
        // and its system nearest the target with 0x800800 (FUN_0041a400):
        // +0x64 = v[6] less one when above 1; FUN_004199b0; state 6.
        let mut f = fixture();
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
        state.module3.phase = YARD_CYCLE;
        state.module3.state = 4;
        state.module3.target = Target::System(f.empty);
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module3.state, 6);
        assert_eq!(state.module3.yard, Yard::Manager(f.yard));
        assert_eq!(state.module3.capacity, 1);
        assert_eq!(state.module3.sites, vec![f.empty]);
        assert_eq!(state.yard_marks.get(&f.empty), Some(&MARK_BALANCE));
        assert_eq!(state.controller.yard_holder, 0x80);
        assert_eq!(state.controller.yard_requests, 0);

        // One yard is lent whole; a listed target is not added twice.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.module3.target = Target::System(f.empty);
        state.module3.sites.push(f.empty);
        assert!(edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| m.borrow_yard(io)
        ));
        assert_eq!(state.module3.capacity, 1);
        assert_eq!(state.module3.sites, vec![f.empty]);

        // Without the token: 0xd.
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.phase = YARD_CYCLE;
        state.module3.state = 4;
        state.controller.yard_requests = YARD_TOKEN;
        step(&f, &mut state, ROOMY);
        assert_eq!(state.module3.state, 0xd);
        assert_eq!(state.controller.yard_requests, 0);
    }

    #[test]
    fn a_borrowed_yard_needs_a_marked_idle_own_yard_and_a_manager() {
        // FUN_004d8db0 fails without a target, a sector with 0x1000, a
        // system with 0x800800 or its manager.
        let mut f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        assert!(!edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| m.borrow_yard(io)
        ));
        state.module3.target = Target::System(f.empty);
        let edits: [fn(&mut Galaxy); 2] = [
            |g| g.sectors[0].candidates &= !0x1000,
            |g| {
                g.sectors[0]
                    .systems
                    .iter_mut()
                    .for_each(|r| r.state &= !0x800)
            },
        ];
        for edit in edits {
            assert!(!edited(&f, &mut state, ROOMY, edit, |m, io| m.borrow_yard(io)));
        }
        let yard = f.world.systems[f.yard]
            .manufacturing_facilities
            .pop()
            .unwrap();
        f.world.manufacturing_facilities.remove(yard);
        let edit = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x1000;
            let r = rec(g, f.yard);
            r.state |= 0x800;
            r.values[6] = 1;
        };
        assert!(!edited(&f, &mut state, ROOMY, edit, |m, io| m.borrow_yard(io)));
        assert_eq!(state.module3.yard, Yard::None);
    }

    #[test]
    fn the_yard_plan_uses_a_tenth_of_the_budget() {
        // FUN_004d9520: w = FUN_004da530; w < 0 gives 8, w == 0 gives 0;
        // w / m yards capped by +0x64 and min(v[15], +0x68): 0xb.
        let f = fixture();
        let plan = |capacity: i32, sites: Vec<SystemKey>, room: i32, lent: i32| {
            let mut state = AIState::new(AiFaction::Alliance);
            state.module3.sites = sites;
            state.module3.target = Target::System(f.empty);
            state.module3.room = room;
            state.module3.capacity = lent;
            let t = Tallies { capacity, ..ROOMY };
            let next = edited(&f, &mut state, t, |_| {}, |m, io| m.plan_yards(io));
            (next, state.module3.count)
        };
        // 9% of 3000 = 270, a tenth 27: 2 yards of 10.
        assert_eq!(plan(3000, vec![], 9, 9), (0xb, 2));
        assert_eq!(plan(3000, vec![], 9, 1), (0xb, 1));
        assert_eq!(plan(3000, vec![], 0, 9), (0xb, 0));
        // 27 less the site's 10 is 17: 1.
        assert_eq!(plan(3000, vec![f.yard], 9, 9), (0xb, 1));
        // 9 less 10 is -1.
        assert_eq!(plan(1000, vec![f.yard], 9, 9), (8, 0));
        // 9 alone is below one yard.
        assert_eq!(plan(1000, vec![], 9, 9), (0, 0));
        // 9% of 1112 = 100, a tenth 10: none left beyond the site.
        assert_eq!(plan(1112, vec![f.yard], 9, 9), (0, 0));
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.room = 9;
        state.module3.capacity = 9;
        let t = Tallies {
            capacity: 3000,
            ..ROOMY
        };
        assert_eq!(
            edited(&f, &mut state, t, |_| {}, |m, io| m.plan_yards(io)),
            0
        );
        state.module3.target = Target::System(f.empty);
        let next = edited(
            &f,
            &mut state,
            t,
            |g| rec(g, f.empty).values[15] = 1,
            |m, io| m.plan_yards(io),
        );
        assert_eq!((next, state.module3.count), (0xb, 1));
    }

    #[test]
    fn the_yard_cycle_orders_and_builds_its_yards() {
        // FUN_004d7060: 6 (FUN_004d8fe0, nothing to scrap) -> 5 -> 0xb
        // (Destination from the borrowed manager) -> 0xc (Build) -> end
        // with 0x3ed. A plan of 0 ends the cycle at once; 8 scraps a
        // spare yard (FUN_004d95c0) and ends.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.phase = YARD_CYCLE;
        state.module3.state = 6;
        state.module3.target = Target::System(f.empty);
        state.module3.yard = Yard::Manager(f.yard);
        state.module3.room = 9;
        state.module3.capacity = 2;
        let rich = Tallies {
            capacity: 3000,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, rich), Step::pending());
        assert_eq!(state.module3.state, 5);
        assert_eq!(step(&f, &mut state, rich), Step::pending());
        assert_eq!(state.module3.state, 0xb);
        let s = step(&f, &mut state, rich);
        assert!(s.complete && s.raise);
        assert_eq!(
            s.order,
            Some(AIAction::SetDestination {
                system: f.yard,
                area: ProductionArea::ConstructionYard,
                destination: f.empty,
            })
        );
        let s = step(&f, &mut state, rich);
        let Some(AIAction::Build { count, kind, .. }) = s.order else {
            panic!("{s:?}");
        };
        assert_eq!(
            (count, kind.class_dat_id()),
            (2, Some(DatId::new(0x2a00_0003)))
        );
        assert_eq!(
            (state.module3.phase, state.module3.state),
            (SURPLUS_CYCLE, 0)
        );

        state.module3.phase = YARD_CYCLE;
        state.module3.state = 5;
        assert_eq!(step(&f, &mut state, ROOMY), Step::complete());
        assert_eq!(
            (state.module3.phase, state.module3.state),
            (SURPLUS_CYCLE, 0)
        );

        let mut f = fixture();
        add_yard(
            &mut f.world,
            f.yard,
            DatId::new(0x2a00_0003),
            Faction::Alliance,
        );
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.sites.push(f.yard);
        state.module3.phase = YARD_CYCLE;
        state.module3.state = 8;
        let s = edited(
            &f,
            &mut state,
            ROOMY,
            |g| rec(g, f.yard).state |= 0x1000,
            |m, io| m.step(io),
        );
        assert_eq!(
            s.order,
            Some(AIAction::Scrap {
                team: vec![manufacturing(&f, f.yard)[0]]
            })
        );
        assert_eq!(
            (state.module3.phase, state.module3.state),
            (SURPLUS_CYCLE, 0)
        );
    }

    #[test]
    fn state_d_adopts_a_yard_system_or_asks_with_bit_one() {
        // FUN_004da0d0 (FUN_004dcc40 with mark 0x2000000, request bit 1,
        // budget FUN_004da530) and FUN_004da610 (the claim).
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.phase = YARD_CYCLE;
        state.module3.state = 0xd;
        let rich = Tallies {
            capacity: 3000,
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
        assert_eq!(state.module3.sites, vec![f.yard]);
        assert_eq!(state.yard_marks.get(&f.yard), Some(&MARK_BALANCE));
        assert_eq!(state.module3.claimed, vec![f.yard]);

        let mut f = fixture();
        let yard = f.world.systems[f.yard]
            .manufacturing_facilities
            .pop()
            .unwrap();
        f.world.manufacturing_facilities.remove(yard);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.phase = YARD_CYCLE;
        state.module3.state = 0xd;
        step(&f, &mut state, rich);
        assert_eq!(state.controller.yard_requests, YARD_TOKEN);
    }

    #[test]
    fn a_stray_phase_starts_the_surplus_cycle_and_states_reset() {
        // FUN_004d6d30: any other +0x38 gives 0x3ed, +0x3c = 0. Each
        // cycle's default: 0x3ed -> 1, 0x3ee -> 4, 0x3ec -> 1. A refusal
        // clears both (FUN_004dd810).
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.phase = 0x3eb;
        state.module3.state = 5;
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(
            (state.module3.phase, state.module3.state),
            (SURPLUS_CYCLE, 0)
        );
        for (phase, s, next) in [
            (SURPLUS_CYCLE, 9, 1),
            (DEFENSE_CYCLE, 2, 4),
            (YARD_CYCLE, 3, 1),
        ] {
            state.module3.phase = phase;
            state.module3.state = s;
            assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
            assert_eq!(state.module3.state, next);
        }
        state.module3.refused();
        assert_eq!((state.module3.phase, state.module3.state), (0, 0));
    }

    #[test]
    fn the_unreachable_surplus_states_scrap_one_defense_and_hand_on() {
        // FUN_004d6da0 states 3 and 7: FUN_004d8520, then 0x3ec.
        let f = fixture();
        for s in [3, 7] {
            let mut state = AIState::new(AiFaction::Alliance);
            state.module3.phase = SURPLUS_CYCLE;
            state.module3.state = s;
            assert_eq!(step(&f, &mut state, ROOMY), Step::complete());
            assert_eq!((state.module3.phase, state.module3.state), (YARD_CYCLE, 0));
        }
    }

    #[test]
    fn the_tallies_sum_the_defense_upkeep() {
        // FUN_0041b7e0: controller +0x1dc += sector +0x100, the own
        // systems' v[31].
        let mut f = fixture();
        let gun = defense_class(&mut f, 0x2200_0001, 2000, 0, 4);
        add_defense(&mut f.world, f.empty, gun);
        add_defense(&mut f.world, f.yard, gun);
        let state = AIState::new(AiFaction::Alliance);
        let g = records_of(&f, &state);
        assert_eq!(Tallies::of(&g, 0, 0).defenses, 8);
    }

    #[test]
    fn the_budgets_hold_while_free_maintenance_equals_them() {
        // FUN_004da4d0 and FUN_004da530 give 0 only when capacity -
        // requirement is below the budget.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        let t = |requirement| Tallies {
            capacity: 1000,
            requirement,
            ..ROOMY
        };
        let run = |state: &mut AIState, tallies| {
            edited(
                &f,
                state,
                tallies,
                |_| {},
                |m, io| (Module3::defense_budget(io), yard_budget(io, &m.sites)),
            )
        };
        // Budgets 81 and 9.
        assert_eq!(run(&mut state, t(919)), (81, 9));
        assert_eq!(run(&mut state, t(920)), (0, 9));
        assert_eq!(run(&mut state, t(991)), (0, 9));
        assert_eq!(run(&mut state, t(992)), (0, 0));
    }

    #[test]
    fn a_defense_cycle_without_a_destination_hands_on_to_the_yards() {
        // FUN_004d6e70 state 0xb: no Destination (no manager) ends the
        // cycle with +0x38 = 0x3ec.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.phase = DEFENSE_CYCLE;
        state.module3.state = 0xb;
        state.module3.site = Target::System(f.empty);
        state.module3.yard = Yard::System(f.yard);
        assert_eq!(step(&f, &mut state, ROOMY), Step::complete());
        assert_eq!((state.module3.phase, state.module3.state), (YARD_CYCLE, 0));
    }

    #[test]
    fn a_yard_class_without_maintenance_plans_no_yards() {
        // FUN_004d9520: +0x60 = w / m only with a class and m != 0.
        let mut f = fixture();
        f.world
            .buildable_classes
            .get_mut(&DatId::new(0x2a00_0003))
            .unwrap()
            .maintenance_cost = 0;
        let mut state = AIState::new(AiFaction::Alliance);
        state.module3.target = Target::System(f.empty);
        state.module3.room = 9;
        state.module3.capacity = 9;
        let t = Tallies {
            capacity: 3000,
            ..ROOMY
        };
        let next = edited(&f, &mut state, t, |_| {}, |m, io| m.plan_yards(io));
        assert_eq!((next, state.module3.count), (0, 0));
        assert_eq!(state.module3.class, Some(DatId::new(0x2a00_0003)));
    }
}
