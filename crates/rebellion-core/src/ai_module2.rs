//! Module 2 of the AI player (`FUN_004e1e50`, step `FUN_004e2120`): it
//! keeps its own construction yard sites and builds a mine or a refinery
//! at a time with them, or more construction yards with a yard it borrows
//! from module 1 (`ghidra/notes/ai-production-modules.md`, "Module 2").
//! Its yard cycle shares module 1's steps ([`crate::ai_module1`]).

use serde::{Deserialize, Serialize};

use crate::ai::{AIAction, MARK_FACILITIES};
use crate::ai_module1::{
    adopt_or_request, build, claim, destination, nearest, scrap_at, scrap_spare_yard, wanted,
    yard_class, Inputs, Step, Target, Yard, ANY, MINE_CLASS, REFINERY_CLASS,
};
use crate::ai_records::{pick, Mask, Order};
use crate::ids::{DatId, SystemKey};

/// `+0x38` while the mine cycle runs (`FUN_004e2120`).
pub const MINE_CYCLE: u32 = 0x3ea;

/// `+0x38` while the refinery cycle runs (`FUN_004e2120`).
pub const REFINERY_CYCLE: u32 = 0x3eb;

/// `+0x38` while the yard cycle runs (`FUN_004e2120`).
pub const YARD_CYCLE: u32 = 0x3ec;

/// The yard token module 2 holds the construction yards with (controller
/// `+0xc`, `+8`; `FUN_004e2400` state 7).
pub const YARD_TOKEN: u32 = 0x40;

/// `+0x24`: the percent of the maintenance capacity module 2 wants in
/// construction yards (`FUN_004e1e50`).
const PERCENT: i32 = 3;

/// Module 2's own fields (`FUN_004e1e50`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module2 {
    /// `+0x38`: 0, [`MINE_CYCLE`], [`REFINERY_CYCLE`] or [`YARD_CYCLE`].
    pub phase: u32,
    /// `+0x3c`: the cycle's state.
    pub state: u32,
    /// `+0x40`: the mine site.
    pub mine_site: Target,
    /// `+0x44`: the refinery site.
    pub refinery_site: Target,
    /// `+0x48`: the target.
    pub target: Target,
    /// `+0x4c`: a site, then its construction manager.
    pub yard: Yard,
    /// `+0x50`: the class to build.
    pub class: Option<DatId>,
    /// `+0x54`: how many.
    pub count: i32,
    /// `+0x58`: the borrowed yard's construction yards.
    pub capacity: i32,
    /// `+0x5c`: the target's free room `v[0x33]`.
    pub room: i32,
    /// `+0x60`: the sites, marked [`MARK_FACILITIES`].
    pub sites: Vec<SystemKey>,
    /// `+0x68`: the claimed sites.
    pub claimed: Vec<SystemKey>,
}

impl Module2 {
    /// One step (`FUN_004e2120`). The scheduler steps only a ready module,
    /// so the not-ready branch never runs. Choosing a cycle does not
    /// complete the module.
    pub fn step(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.phase {
            0 => {
                self.phase = if !self.keep_site(io) {
                    YARD_CYCLE
                } else if io.tallies.refineries <= io.tallies.mines {
                    REFINERY_CYCLE
                } else {
                    MINE_CYCLE
                };
                Step::pending()
            }
            MINE_CYCLE | REFINERY_CYCLE => self.facility_cycle(io),
            YARD_CYCLE => self.yard_cycle(io),
            _ => {
                self.phase = 0;
                Step::pending()
            }
        }
    }

    /// A refused order (slot `+0x18`, `FUN_004dd810`).
    pub fn refused(&mut self) {
        self.phase = 0;
        self.state = 0;
    }

    fn end(&mut self, order: Option<AIAction>) -> Step {
        self.state = 0;
        self.phase = 0;
        Step {
            order,
            complete: true,
            raise: false,
        }
    }

    /// States 1 and 2 of both cycles (`FUN_004e25c0`, `FUN_004e2400`):
    /// 2 after a system over budget, then 3.
    fn budget_states(&mut self, io: &mut Inputs<'_>) -> Step {
        if self.state == 1 {
            self.state = if self.over_budget(io) { 2 } else { 3 };
            return Step::pending();
        }
        let order = self.scrap_one(io);
        self.state = 3;
        Step::ordered(order)
    }

    /// States 8 of both cycles (`FUN_004e4300`): Destination to `site`.
    fn destination_state(&mut self, site: Target) -> Step {
        match destination(site, self.yard) {
            Some(order) => {
                self.state = 9;
                Step {
                    order: Some(order),
                    complete: true,
                    raise: true,
                }
            }
            None => self.end(None),
        }
    }

    /// The mine and refinery cycle (`FUN_004e25c0`).
    fn facility_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        let refinery = self.phase == REFINERY_CYCLE;
        let site = if refinery {
            self.refinery_site
        } else {
            self.mine_site
        };
        match self.state {
            1 | 2 => self.budget_states(io),
            3 => {
                let found = if refinery {
                    self.find_refinery_site(io)
                } else {
                    self.find_mine_site(io)
                };
                if found {
                    self.state = 4;
                    Step::pending()
                } else {
                    self.end(None)
                }
            }
            4 => {
                let order = Self::scrap_in_the_way(io, site);
                self.state = 8;
                Step::ordered(order)
            }
            8 => self.destination_state(site),
            9 => {
                let class = if refinery { REFINERY_CLASS } else { MINE_CLASS };
                self.class = Some(DatId::new(class));
                self.count = 1;
                let order = build(io, self.yard, self.class, self.count);
                self.end(order)
            }
            _ => {
                self.state = 1;
                Step::pending()
            }
        }
    }

    /// The yard cycle (`FUN_004e2400`).
    fn yard_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            1 | 2 => self.budget_states(io),
            3 => {
                if self.choose_target(io) {
                    self.state = 7;
                    Step::pending()
                } else {
                    self.end(None)
                }
            }
            4 => {
                let order = scrap_at(io, self.target.system());
                self.state = 5;
                Step::ordered(order)
            }
            5 => {
                self.state = self.plan_yards(io);
                if self.state == 0 {
                    return self.end(None);
                }
                Step::pending()
            }
            6 => {
                let order =
                    scrap_spare_yard(io, &mut self.sites, &mut self.target, MARK_FACILITIES);
                self.end(order)
            }
            7 => {
                io.controller.yard_requests &= !YARD_TOKEN;
                if io.controller.yard_holder == YARD_TOKEN && self.borrow_yard(io) {
                    io.controller.pass_yards();
                    self.state = 4;
                } else {
                    self.state = 10;
                }
                Step::pending()
            }
            8 => self.destination_state(self.target),
            9 => {
                let order = build(io, self.yard, self.class, self.count);
                self.end(order)
            }
            10 => {
                let budget = wanted(io, &self.sites, PERCENT);
                self.target = adopt_or_request(
                    io,
                    &mut self.sites,
                    &mut self.class,
                    MARK_FACILITIES,
                    YARD_TOKEN,
                    budget,
                );
                claim(io, &self.sites, &mut self.claimed);
                self.end(None)
            }
            _ => {
                self.state = 1;
                Step::pending()
            }
        }
    }

    /// `FUN_004e21d0`: keep the first site with an idle construction yard,
    /// and take its manager as the yard.
    fn keep_site(&mut self, io: &mut Inputs<'_>) -> bool {
        keep_site(io, &mut self.sites, &mut self.yard, MARK_FACILITIES)
    }

    /// `FUN_004e2760`: while the side's maintenance requirement exceeds
    /// its capacity, a system to scrap at, into the target.
    fn over_budget(&mut self, io: &mut Inputs<'_>) -> bool {
        if io.tallies.capacity >= io.tallies.requirement {
            return false;
        }
        let bits = [Mask::new(0xe0, 0), ANY, ANY];
        let lists = vec![
            io.sectors(bits, 0x15, Order::Ascending),
            io.sectors(bits, 0x11, Order::Ascending),
            io.sectors(bits, 0x13, Order::Ascending),
        ];
        let sector = pick(&lists);
        let masks = [Mask::new(0, 0x3e0_0000), ANY, Mask::new(1, 0)];
        let lists = vec![
            io.systems(sector, masks, 6, Order::Ascending),
            io.systems(sector, masks, 4, Order::Ascending),
            io.systems(sector, masks, 5, Order::Ascending),
        ];
        self.target = pick(&lists).map_or(Target::None, Target::System);
        self.target.system().is_some()
    }

    /// `FUN_004e2940`: the side's first shipyard at the target, else its
    /// first training facility, else its first construction yard.
    fn scrap_one(&mut self, io: &mut Inputs<'_>) -> Option<AIAction> {
        let target = self.target.system()?;
        io.galaxy.system(target)?;
        let member = [0x28, 0x29, 0x2a]
            .into_iter()
            .find_map(|family| io.facilities(target, family).into_iter().next())?;
        Some(AIAction::Scrap { team: vec![member] })
    }

    /// `FUN_004e2c70`: keep the mine site, or take the one nearest the
    /// yard.
    fn find_mine_site(&mut self, io: &mut Inputs<'_>) -> bool {
        self.mine_site = Self::site_near(
            io,
            self.mine_site,
            self.yard,
            0x80,
            [(0x100, 0x80), (0x800, 0x40)],
        );
        self.mine_site.system().is_some()
    }

    /// `FUN_004e2f10`: keep the refinery site, or take the one nearest the
    /// yard.
    fn find_refinery_site(&mut self, io: &mut Inputs<'_>) -> bool {
        self.refinery_site = Self::site_near(
            io,
            self.refinery_site,
            self.yard,
            0x100,
            [(0x200, 0x100), (0x400, 0x20)],
        );
        self.refinery_site.system().is_some()
    }

    /// The site kept when it is a calm system with `keep` (`+0x28`), else
    /// the nearest by each (sector `+0x24`, system `+0x28`) pair in turn,
    /// the system calm (`FUN_004e2c70`, `FUN_004e2f10`).
    fn site_near(
        io: &mut Inputs<'_>,
        site: Target,
        yard: Yard,
        keep: u32,
        pairs: [(u32, u32); 2],
    ) -> Target {
        if let Some(system) = site.system() {
            if io
                .galaxy
                .system(system)
                .is_some_and(|r| r.state & 3 == 0 && r.state & keep != 0)
            {
                return site;
            }
        }
        let mut site = Target::None;
        for (sector_bit, system_bit) in pairs {
            site = nearest(io, yard, sector_bit, [Mask::new(system_bit, 3), ANY, ANY]);
            if site.system().is_some() {
                break;
            }
        }
        site
    }

    /// `FUN_004e31b0`.
    fn scrap_in_the_way(io: &mut Inputs<'_>, site: Target) -> Option<AIAction> {
        scrap_in_the_way(io, site)
    }

    /// `FUN_004e3530`: the target, a site with room and the fewest
    /// construction yards, or a new site.
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
        let bit = [Mask::new(0x80, 0), ANY, ANY];
        let lists = vec![
            io.sectors(bit, 0x15, Order::Ascending),
            io.sectors(bit, 4, Order::Descending),
        ];
        let sector = pick(&lists);
        let masks = [Mask::new(0, 0x380_0003), ANY, Mask::new(1, 0x4000_0000)];
        let mut lists = vec![io.systems(sector, masks, 6, Order::Ascending)];
        if !self.sites.is_empty() {
            lists.push(io.systems(sector, masks, 0x33, Order::Ascending));
        }
        let Some(system) = pick(&lists) else {
            return false;
        };
        self.target = Target::System(system);
        if io.galaxy.system(system).is_some() {
            self.sites.push(system);
            io.set_mark(system, MARK_FACILITIES, 0);
        }
        true
    }

    /// `FUN_004e3810`: borrow the idle construction yard of module 1's
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
        let sector = pick(&[sectors]);
        let masks = [Mask::new(0x80_0800, 3), ANY, Mask::new(1, 0)];
        let systems = io.systems(sector, masks, 6, Order::Ascending);
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
            io.set_mark(target, MARK_FACILITIES, 0);
        }
        true
    }

    /// `FUN_004e3f80`: 8 to build construction yards, 6 to scrap one when
    /// the sites hold more than the side wants, else 0.
    fn plan_yards(&mut self, io: &mut Inputs<'_>) -> u32 {
        self.count = 0;
        let wanted = wanted(io, &self.sites, PERCENT);
        if wanted < 0 {
            return 6;
        }
        if wanted == 0 {
            return 0;
        }
        let maintenance;
        (self.class, maintenance) = yard_class(io);
        if self.class.is_some() && maintenance != 0 {
            self.count = wanted / maintenance;
        }
        if self.count <= 0 {
            return 0;
        }
        self.count = self.count.min(self.capacity);
        let Some(r) = self.target.system().and_then(|t| io.galaxy.system(t)) else {
            return 0;
        };
        self.count = self.count.min(r.values[15].min(self.room));
        8
    }
}

/// `FUN_004e21d0` (module 3: `FUN_004d7ac0`): keep the first site with an
/// idle construction yard, and take its manager as the yard. A site that
/// is not the side's or lost its `mark` leaves the list.
pub(crate) fn keep_site(
    io: &mut Inputs<'_>,
    sites: &mut Vec<SystemKey>,
    yard: &mut Yard,
    mark: u32,
) -> bool {
    *yard = Yard::None;
    let mut i = 0;
    while i < sites.len() {
        let site = sites[i];
        let Some(r) = io.galaxy.system(site).copied() else {
            sites.remove(i);
            continue;
        };
        if r.kind & 1 == 0 || r.state & mark == 0 {
            io.set_mark(site, 0, mark);
            sites.remove(i);
            continue;
        }
        i += 1;
        if r.values[6] > 0 && r.state & 0x800 != 0 && r.state & 3 == 0 {
            *yard = Yard::System(site);
            break;
        }
    }
    match *yard {
        Yard::System(site) if io.has_manager(site) => {
            *yard = Yard::Manager(site);
            true
        }
        _ => false,
    }
}

/// `FUN_004e31b0` (the first part of module 3's `FUN_004d7cf0`): at the
/// site, every shipyard, training facility and construction yard not kept
/// by another module's mark.
pub(crate) fn scrap_in_the_way(io: &mut Inputs<'_>, site: Target) -> Option<AIAction> {
    let site = site.system()?;
    let r = io.galaxy.system(site).copied()?;
    let mut team = Vec::new();
    for (family, mark) in [(0x28, 0x20_0000), (0x29, 0x40_0000), (0x2a, 0x380_0000)] {
        if r.state & mark == 0 {
            team.extend(io.facilities(site, family));
        }
    }
    (!team.is_empty()).then_some(AIAction::Scrap { team })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{AIState, AiFaction, MARK_MINES};
    use crate::ai_module1::tests::{
        add_yard, class, fixture, record, records_of, system, Fixture, ROOMY,
    };
    use crate::ai_module1::{Controller, Tallies};
    use crate::ai_records::Galaxy;
    use crate::dat::Faction;
    use crate::research::ResearchState;
    use crate::scrap::ScrapTarget;
    use crate::world::FacilityRef;

    /// Runs `run` on the side's module 2 over `galaxy`.
    fn with_io<R>(
        f: &Fixture,
        state: &mut AIState,
        galaxy: &mut Galaxy,
        tallies: Tallies,
        run: impl FnOnce(&mut Module2, &mut Inputs<'_>) -> R,
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
        run(&mut state.module2, &mut io)
    }

    /// One step of the side's module 2 over freshly built records.
    fn step(f: &Fixture, state: &mut AIState, tallies: Tallies) -> Step {
        let mut galaxy = records_of(f, state);
        with_io(f, state, &mut galaxy, tallies, |m, io| m.step(io))
    }

    /// `f.yard` as module 2's site.
    fn with_site(f: &Fixture) -> AIState {
        let mut state = AIState::new(AiFaction::Alliance);
        state.module2.sites.push(f.yard);
        state.yard_marks.insert(f.yard, MARK_FACILITIES);
        state
    }

    fn yards(f: &Fixture, at: crate::ids::SystemKey) -> Vec<ScrapTarget> {
        f.world.systems[at]
            .manufacturing_facilities
            .iter()
            .map(|&k| ScrapTarget::Facility(FacilityRef::Manufacturing(k)))
            .collect()
    }

    /// A shipyard, a training facility and a construction yard at `empty`.
    fn three_yards(f: &mut Fixture) -> Vec<ScrapTarget> {
        let shipyard = class(&mut f.world, 0x2800_0001, 10, 3, 0);
        let training = class(&mut f.world, 0x2900_0001, 10, 3, 0);
        let construction = DatId::new(0x2a00_0003);
        for c in [shipyard, training, construction] {
            add_yard(&mut f.world, f.empty, c, Faction::Alliance);
        }
        yards(f, f.empty)
    }

    #[test]
    fn an_idle_site_starts_the_refinery_cycle_unless_refineries_lead() {
        // FUN_004e2120: FUN_004e21d0 keeps the marked own site with v[6] >
        // 0, an idle yard (0x800) and calm, and takes its manager into
        // +0x4c; +0x38 = 0x3eb when +0x1d4 <= +0x1d0, else 0x3ea. Choosing
        // a cycle does not complete the module.
        let f = fixture();
        for (mines, refineries, phase) in [(0, 0, REFINERY_CYCLE), (1, 2, MINE_CYCLE)] {
            let mut state = with_site(&f);
            let tallies = Tallies {
                mines,
                refineries,
                ..ROOMY
            };

            let s = step(&f, &mut state, tallies);

            assert_eq!(s, Step::pending());
            assert_eq!(state.module2.phase, phase);
            assert_eq!(state.module2.yard, Yard::Manager(f.yard));
        }
    }

    #[test]
    fn without_a_site_to_keep_the_yard_cycle_starts() {
        // FUN_004e21d0: a site that lost its mark 0x1000000 drops it and
        // leaves; one not the side's leaves too. FUN_004e2120 then sets
        // +0x38 = 0x3ec.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module2.sites.push(f.yard);
        state.yard_marks.insert(f.yard, MARK_MINES);

        let s = step(&f, &mut state, ROOMY);

        assert_eq!(s, Step::pending());
        assert_eq!(state.module2.phase, YARD_CYCLE);
        assert!(state.module2.sites.is_empty());
        assert_eq!(state.module2.yard, Yard::None);

        let mut state = with_site(&f);
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.yard).kind &= !1;
        let kept = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.keep_site(io));
        assert!(!kept);
        assert!(state.module2.sites.is_empty());
        assert_eq!(state.yard_marks.get(&f.yard), None);
    }

    #[test]
    fn a_busy_bare_or_unsettled_site_stays_listed_but_is_not_kept() {
        // FUN_004e21d0 takes a site only with v[6] > 0, +0x28 0x800 and
        // +0x28 & 3 == 0; any other own marked site stays in +0x60. A site
        // without a record leaves.
        let f = fixture();
        let changes: [fn(&mut crate::ai_records::SystemRecord); 4] = [
            |r| r.state &= !0x800,
            |r| r.values[6] = 0,
            |r| r.state |= 1,
            |r| r.state |= 2,
        ];
        for change in changes {
            let mut state = with_site(&f);
            let mut galaxy = records_of(&f, &state);
            change(record(&mut galaxy, f.yard));
            let kept = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.keep_site(io));
            assert!(!kept);
            assert_eq!(state.module2.sites, vec![f.yard]);
            assert_eq!(state.module2.yard, Yard::None);
        }

        let mut state = with_site(&f);
        state.module2.sites.insert(0, f.empty);
        let mut galaxy = records_of(&f, &state);
        galaxy.sectors[0].systems.retain(|r| r.system != f.empty);
        let kept = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.keep_site(io));
        assert!(kept);
        assert_eq!(state.module2.sites, vec![f.yard]);
    }

    #[test]
    fn a_kept_site_without_a_manager_keeps_its_system_but_fails() {
        // FUN_004e21d0: the site stays in +0x4c when FUN_00509670 finds no
        // construction manager; the step fails.
        let mut f = fixture();
        let yard = f.world.systems[f.yard]
            .manufacturing_facilities
            .pop()
            .unwrap();
        f.world.manufacturing_facilities.remove(yard);
        let mut state = with_site(&f);
        let mut galaxy = records_of(&f, &state);
        let r = record(&mut galaxy, f.yard);
        r.state |= 0x800;
        r.values[6] = 1;

        let kept = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.keep_site(io));

        assert!(!kept);
        assert_eq!(state.module2.yard, Yard::System(f.yard));
    }

    #[test]
    fn the_refinery_cycle_orders_one_refinery_at_the_nearest_site() {
        // FUN_004e25c0: 0 -> 1 -> 3 (FUN_004e2760: no shortfall) -> 4
        // (FUN_004e2f10: the nearest sector with +0x24 0x200, its nearest
        // calm system with +0x28 0x100: the empty one, as the marked site's
        // record has no room bits, FUN_00433e40) -> 8 (FUN_004e31b0: nothing
        // there) -> 9 (FUN_004e4300: Destination, complete, raise) -> end
        // (FUN_004e4440: Build 1 of 0x2d000002 on the site's manager).
        let f = fixture();
        let mut state = with_site(&f);
        let mut steps = Vec::new();
        for _ in 0..8 {
            let s = step(&f, &mut state, ROOMY);
            steps.push((state.module2.state, s.complete, s.raise));
            if let Some(order) = s.order {
                assert_eq!(
                    order,
                    AIAction::SetDestination {
                        system: f.yard,
                        area: crate::manufacturing::ProductionArea::ConstructionYard,
                        destination: f.empty,
                    }
                );
                break;
            }
        }
        assert_eq!(
            steps,
            [
                (0, false, false),
                (1, false, false),
                (3, false, false),
                (4, false, false),
                (8, false, false),
                (9, true, true),
            ]
        );
        assert_eq!(state.module2.refinery_site, Target::System(f.empty));

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
        assert_eq!(kind.class_dat_id(), Some(DatId::new(REFINERY_CLASS)));
        assert_eq!((state.module2.phase, state.module2.state), (0, 0));
    }

    #[test]
    fn the_mine_cycle_builds_the_mine_class() {
        // FUN_004e25c0 state 9 with +0x38 = 0x3ea: +0x50 = 0x2c000001,
        // +0x54 = 1.
        let f = fixture();
        let mut state = with_site(&f);
        state.module2.phase = MINE_CYCLE;
        state.module2.state = 9;
        state.module2.yard = Yard::Manager(f.yard);

        let s = step(&f, &mut state, ROOMY);

        let Some(AIAction::Build { kind, count, .. }) = s.order else {
            panic!("{s:?}");
        };
        assert_eq!(count, 1);
        assert_eq!(kind.class_dat_id(), Some(DatId::new(MINE_CLASS)));
        assert!(s.complete && !s.raise);
    }

    #[test]
    fn a_mine_site_is_kept_while_calm_with_room_else_the_nearest_is_taken() {
        // FUN_004e2c70: +0x40 stays when its record is calm with +0x28
        // 0x80; else FUN_00419b80/FUN_0041a400 from +0x4c (sector 0x100,
        // system 0x80 forbidding 3; then 0x800 and 0x40). FUN_004e2f10 the
        // same with 0x100, (0x200, 0x100), (0x400, 0x20).
        let f = fixture();
        let mut state = with_site(&f);
        state.module2.yard = Yard::Manager(f.yard);
        state.module2.mine_site = Target::System(f.empty);
        state.module2.refinery_site = Target::System(f.empty);
        let mut galaxy = records_of(&f, &state);
        let found = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            (m.find_mine_site(io), m.find_refinery_site(io))
        });
        assert_eq!(found, (true, true));
        assert_eq!(state.module2.mine_site, Target::System(f.empty));
        assert_eq!(state.module2.refinery_site, Target::System(f.empty));

        for (bit, mine) in [(0x80, true), (0x100, false), (2, true), (2, false)] {
            let mut galaxy = records_of(&f, &state);
            let r = record(&mut galaxy, f.empty);
            if bit == 2 {
                r.state |= 2;
            } else {
                r.state &= !bit;
            }
            with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
                if mine {
                    m.find_mine_site(io);
                } else {
                    m.find_refinery_site(io);
                }
            });
            let site = if mine {
                state.module2.mine_site
            } else {
                state.module2.refinery_site
            };
            assert_eq!(site, Target::System(f.yard), "bit {bit:#x} mine {mine}");
            state.module2.mine_site = Target::System(f.empty);
            state.module2.refinery_site = Target::System(f.empty);
        }
    }

    #[test]
    fn the_fallback_site_pairs_apply_when_the_first_finds_no_system() {
        // FUN_004e2c70: without a system by sector 0x100 / system 0x80,
        // sector 0x800 / system 0x40. FUN_004e2f10: 0x400 / 0x20.
        let f = fixture();
        let mut state = with_site(&f);
        state.module2.yard = Yard::Manager(f.yard);
        for mine in [true, false] {
            let mut galaxy = records_of(&f, &state);
            for system in [f.yard, f.empty] {
                let r = record(&mut galaxy, system);
                r.state &= !(0x80 | 0x100);
                r.state |= 0x20 | 0x40;
            }
            galaxy.sectors[0].candidates = 0x800 | 0x400;
            let found = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
                if mine {
                    m.find_mine_site(io)
                } else {
                    m.find_refinery_site(io)
                }
            });
            assert!(found, "mine {mine}");

            let mut galaxy = records_of(&f, &state);
            galaxy.sectors[0].candidates = 0;
            state.module2.mine_site = Target::None;
            state.module2.refinery_site = Target::None;
            let found = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
                if mine {
                    m.find_mine_site(io)
                } else {
                    m.find_refinery_site(io)
                }
            });
            assert!(!found, "mine {mine}");
        }
    }

    #[test]
    fn with_no_site_the_facility_cycle_ends() {
        // FUN_004e25c0 state 3: no site, so +0x3c = 0, +0x38 = 0, complete.
        let f = fixture();
        let mut state = with_site(&f);
        state.module2.phase = MINE_CYCLE;
        state.module2.state = 3;
        state.module2.yard = Yard::None;
        let s = step(&f, &mut state, ROOMY);
        assert_eq!(s, Step::complete());
        assert_eq!((state.module2.phase, state.module2.state), (0, 0));
    }

    #[test]
    fn over_budget_the_cycle_scraps_one_yard_at_a_system() {
        // FUN_004e2760 runs only while +0x184 < +0x188: sectors by 0xe0,
        // systems with +0x30 1 and no 0x3e00000 mark, picked into +0x48;
        // state 2 (FUN_004e2940) then scraps one member, completing and
        // raising.
        let mut f = fixture();
        let team = three_yards(&mut f);
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        let short = Tallies {
            capacity: 10,
            requirement: 20,
            ..ROOMY
        };
        state.module2.phase = YARD_CYCLE;
        state.module2.state = 1;

        assert_eq!(step(&f, &mut state, short), Step::pending());
        assert_eq!(state.module2.state, 2);
        assert_eq!(state.module2.target, Target::System(f.empty));

        let s = step(&f, &mut state, short);
        assert_eq!(
            s,
            Step {
                order: Some(AIAction::Scrap {
                    team: vec![team[0]]
                }),
                complete: true,
                raise: true,
            }
        );
        assert_eq!(state.module2.state, 3);

        // At capacity the check leaves the target alone and goes to 3.
        state.module2.state = 1;
        state.module2.target = Target::None;
        let even = Tallies {
            capacity: 20,
            requirement: 20,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, even), Step::pending());
        assert_eq!(state.module2.state, 3);
        assert_eq!(state.module2.target, Target::None);
    }

    #[test]
    fn the_budget_scrap_takes_a_shipyard_then_a_training_facility_then_a_yard() {
        // FUN_004e2940: the first of FUN_0052cab0, else FUN_0052c740, else
        // FUN_0052c420; nothing without a target record. State 2 without
        // an order steps on.
        let mut f = fixture();
        let team = three_yards(&mut f);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module2.target = Target::System(f.empty);
        for expected in [team[0], team[1], team[2]] {
            let mut galaxy = records_of(&f, &state);
            let order = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.scrap_one(io));
            assert_eq!(
                order,
                Some(AIAction::Scrap {
                    team: vec![expected]
                })
            );
            let ScrapTarget::Facility(FacilityRef::Manufacturing(k)) = expected else {
                unreachable!()
            };
            f.world.systems[f.empty]
                .manufacturing_facilities
                .retain(|&m| m != k);
        }
        let mut galaxy = records_of(&f, &state);
        state.module2.state = 2;
        let s = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.budget_states(io)
        });
        assert_eq!(s, Step::pending());
        assert_eq!(state.module2.state, 3);

        let mut galaxy = records_of(&f, &state);
        galaxy.sectors[0].systems.retain(|r| r.system != f.yard);
        state.module2.target = Target::System(f.yard);
        let order = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.scrap_one(io));
        assert_eq!(order, None);
    }

    #[test]
    fn the_site_scrap_spares_each_kind_its_mark_keeps() {
        // FUN_004e31b0: every 0x28 unless 0x200000, every 0x29 unless
        // 0x400000, every 0x2a unless 0x3800000.
        let mut f = fixture();
        let team = three_yards(&mut f);
        for (mark, kept) in [
            (0, None),
            (0x20_0000, Some(0)),
            (0x40_0000, Some(1)),
            (MARK_MINES, Some(2)),
            (MARK_FACILITIES, Some(2)),
            (crate::ai::MARK_BALANCE, Some(2)),
        ] {
            let mut state = AIState::new(AiFaction::Alliance);
            if mark != 0 {
                state.yard_marks.insert(f.empty, mark);
            }
            let mut galaxy = records_of(&f, &state);
            let order = with_io(&f, &mut state, &mut galaxy, ROOMY, |_, io| {
                Module2::scrap_in_the_way(io, Target::System(f.empty))
            });
            let mut expected = team.clone();
            if let Some(i) = kept {
                expected.remove(i);
            }
            assert_eq!(order, Some(AIAction::Scrap { team: expected }), "{mark:#x}");
        }
        let mut state = AIState::new(AiFaction::Alliance);
        let mut galaxy = records_of(&f, &state);
        let none = with_io(&f, &mut state, &mut galaxy, ROOMY, |_, io| {
            Module2::scrap_in_the_way(io, Target::System(f.yard))
        });
        assert_eq!(
            none,
            Some(AIAction::Scrap {
                team: yards(&f, f.yard)
            })
        );
        state.yard_marks.insert(f.yard, MARK_MINES);
        let mut galaxy = records_of(&f, &state);
        let none = with_io(&f, &mut state, &mut galaxy, ROOMY, |_, io| {
            Module2::scrap_in_the_way(io, Target::System(f.yard))
        });
        assert_eq!(none, None);
    }

    #[test]
    fn the_target_is_the_calm_own_site_with_room_and_the_fewest_yards() {
        // FUN_004e3530: from +0x60, own, calm, v[0x33] > 0, v[6] below the
        // best (from 1000); +0x5c = v[0x33].
        let f = fixture();
        let mut state = with_site(&f);
        state.module2.sites.push(f.empty);
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.empty).values[0x33] = 4;
        record(&mut galaxy, f.empty).values[6] = 0;
        record(&mut galaxy, f.yard).values[0x33] = 7;
        let found = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.choose_target(io)
        });
        assert!(found);
        assert_eq!(state.module2.target, Target::System(f.empty));
        assert_eq!(state.module2.room, 4);

        let changes: [fn(&mut crate::ai_records::SystemRecord); 5] = [
            |r| r.kind &= !1,
            |r| r.state |= 1,
            |r| r.state |= 2,
            |r| r.values[0x33] = 0,
            |r| r.values[6] = 1,
        ];
        for change in changes {
            let mut galaxy = records_of(&f, &state);
            let r = record(&mut galaxy, f.empty);
            r.values[0x33] = 4;
            r.values[6] = 0;
            change(r);
            record(&mut galaxy, f.yard).values[0x33] = 7;
            with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
                m.choose_target(io)
            });
            assert_eq!(state.module2.target, Target::System(f.yard));
            assert_eq!(state.module2.room, 7);
        }
    }

    #[test]
    fn without_a_site_the_target_comes_from_the_query_and_becomes_a_site() {
        // FUN_004e3530: sectors by +0x24 0x80; systems with +0x30 1,
        // forbidding +0x28 0x3800003; the system found joins +0x60 with
        // the mark 0x1000000 and +0x5c stays 0. No system: failure, and
        // the yard cycle ends.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        let mut galaxy = records_of(&f, &state);
        let found = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.choose_target(io)
        });
        assert!(found);
        assert_eq!(state.module2.target, Target::System(f.empty));
        assert_eq!(state.module2.sites, vec![f.empty]);
        assert_eq!(state.module2.room, 0);
        assert_eq!(state.yard_marks.get(&f.empty), Some(&MARK_FACILITIES));

        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.yard_marks.insert(f.empty, MARK_MINES);
        state.module2.phase = YARD_CYCLE;
        state.module2.state = 3;
        let s = step(&f, &mut state, ROOMY);
        assert_eq!(s, Step::complete());
        assert_eq!((state.module2.phase, state.module2.state), (0, 0));
        assert_eq!(state.module2.target, Target::None);
    }

    #[test]
    fn with_the_token_module_two_borrows_module_one_s_nearest_yard() {
        // FUN_004e2400 state 7: clear +8 bit 0x40; with +0xc == 0x40,
        // FUN_004e3810 takes the sector nearest the target with +0x24
        // 0x1000, its system with +0x28 0x800800 (calm, own): +0x58 = v[6]
        // less one, +0x4c its manager; the target joins +0x60. Then
        // FUN_004199b0 and state 4.
        let mut f = fixture();
        let construction = DatId::new(0x2a00_0003);
        add_yard(&mut f.world, f.yard, construction, Faction::Alliance);
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.controller.yard_holder = YARD_TOKEN;
        state.controller.yard_requests = YARD_TOKEN | 0x80;
        state.module2.phase = YARD_CYCLE;
        state.module2.state = 7;
        state.module2.target = Target::System(f.empty);

        let s = step(&f, &mut state, ROOMY);

        assert_eq!(s, Step::pending());
        assert_eq!(state.module2.state, 4);
        assert_eq!(state.module2.yard, Yard::Manager(f.yard));
        assert_eq!(state.module2.capacity, 1);
        assert_eq!(state.module2.sites, vec![f.empty]);
        assert_eq!(state.yard_marks.get(&f.empty), Some(&MARK_FACILITIES));
        assert_eq!(state.controller.yard_holder, 0x80);
        assert_eq!(state.controller.yard_requests, 0);

        // A single yard is lent whole; a listed target is not added again.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.module2.target = Target::System(f.empty);
        state.module2.sites.push(f.empty);
        let mut galaxy = records_of(&f, &state);
        let ok = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.borrow_yard(io)
        });
        assert!(ok);
        assert_eq!(state.module2.capacity, 1);
        assert_eq!(state.module2.sites, vec![f.empty]);
    }

    #[test]
    fn without_the_token_or_a_yard_to_borrow_module_two_adopts_or_asks() {
        // FUN_004e2400 state 7 -> 10 when +0xc != 0x40 or FUN_004e3810
        // fails; state 10 runs FUN_004e4540 (module 1's FUN_004dcc40 with
        // the request 0x40) and FUN_004e4a20, and ends.
        let f = fixture();
        for (holder, mark) in [(0x80, MARK_MINES), (YARD_TOKEN, 0)] {
            let mut state = AIState::new(AiFaction::Alliance);
            if mark != 0 {
                state.yard_marks.insert(f.yard, mark);
            }
            state.controller.yard_holder = holder;
            state.controller.yard_requests = YARD_TOKEN;
            state.module2.phase = YARD_CYCLE;
            state.module2.state = 7;
            state.module2.target = Target::System(f.empty);

            assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
            assert_eq!(state.module2.state, 10, "holder {holder:#x}");
            assert_eq!(state.controller.yard_requests, 0);
            assert_eq!(state.controller.yard_holder, holder);

            let s = step(&f, &mut state, ROOMY);
            assert_eq!(s, Step::complete());
            assert_eq!((state.module2.phase, state.module2.state), (0, 0));
        }

        // No sector has unmarked yards and no site has one: ask with 0x40.
        let mut f = fixture();
        let yard = f.world.systems[f.yard]
            .manufacturing_facilities
            .pop()
            .unwrap();
        f.world.manufacturing_facilities.remove(yard);
        let mut state = AIState::new(AiFaction::Alliance);
        state.controller.yard_holder = 0x80;
        state.module2.phase = YARD_CYCLE;
        state.module2.state = 10;
        step(&f, &mut state, ROOMY);
        assert_eq!(state.controller.yard_requests, YARD_TOKEN);
        assert_eq!(state.module2.class, Some(DatId::new(0x2a00_0003)));
    }

    #[test]
    fn state_ten_adopts_a_yard_system_and_claims_a_site() {
        // FUN_004e4540 adopts the own system with unmarked construction
        // yards (mark 0x1000000), and FUN_004e4a20 claims the site with
        // the most yards and active regiments (record +0x30 0x10000000).
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.controller.yard_holder = 0x80;
        state.module2.phase = YARD_CYCLE;
        state.module2.state = 10;
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.yard).values[18] = 1;
        let s = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.step(io));
        assert_eq!(s, Step::complete());
        assert_eq!(state.module2.sites, vec![f.yard]);
        assert_eq!(state.module2.target, Target::System(f.yard));
        assert_eq!(state.yard_marks.get(&f.yard), Some(&MARK_FACILITIES));
        assert_eq!(state.module2.claimed, vec![f.yard]);
        assert_eq!(
            state.record_kinds.get(&f.yard),
            Some(&crate::ai_module1::CLAIMED)
        );
    }

    #[test]
    fn the_yard_plan_scraps_a_surplus_and_caps_the_count() {
        // FUN_004e3f80: w = 3% of +0x184 less the sites' v[36]
        // (FUN_004e4950): w < 0 gives 6, w == 0 gives 0; w > 0 builds w / m
        // of the class (FUN_0041b230), capped by +0x58 and by min(v[15],
        // +0x5c), and gives 8 even when the cap leaves 0.
        let f = fixture();
        let plan = |capacity: i32, room: i32, borrowed: i32, sites: Vec<_>| {
            let mut state = AIState::new(AiFaction::Alliance);
            state.module2.sites = sites;
            state.module2.target = Target::System(f.empty);
            state.module2.room = room;
            state.module2.capacity = borrowed;
            let mut galaxy = records_of(&f, &state);
            let tallies = Tallies { capacity, ..ROOMY };
            let next = with_io(&f, &mut state, &mut galaxy, tallies, |m, io| {
                m.plan_yards(io)
            });
            (next, state.module2.count)
        };
        // 3% of 100 is 3, less the site's 10.
        assert_eq!(plan(100, 9, 9, vec![f.yard]), (6, 0));
        // 3% of 1000 is 30, less 10 is 20; 20 / 10 = 2.
        assert_eq!(plan(1000, 9, 9, vec![f.yard]), (8, 2));
        assert_eq!(plan(1000, 9, 1, vec![f.yard]), (8, 1));
        assert_eq!(plan(1000, 1, 9, vec![f.yard]), (8, 1));
        assert_eq!(plan(1000, 0, 9, vec![f.yard]), (8, 0));
        // 30 / 10 = 3 without a site.
        assert_eq!(plan(1000, 9, 9, vec![]), (8, 3));
        // 3% of 333 is 9, below one yard.
        assert_eq!(plan(333, 9, 9, vec![]), (0, 0));
        // 3% of 334 is 10, less 10 is 0.
        assert_eq!(plan(334, 9, 9, vec![f.yard]), (0, 0));
        // Unaffordable: capacity - requirement below w.
        let mut state = AIState::new(AiFaction::Alliance);
        state.module2.target = Target::System(f.empty);
        state.module2.room = 9;
        state.module2.capacity = 9;
        let mut galaxy = records_of(&f, &state);
        let poor = Tallies {
            capacity: 1000,
            requirement: 980,
            ..ROOMY
        };
        let next = with_io(&f, &mut state, &mut galaxy, poor, |m, io| m.plan_yards(io));
        assert_eq!(next, 0);
        // No target record: 0.
        let mut galaxy = records_of(&f, &state);
        state.module2.target = Target::None;
        let next = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.plan_yards(io));
        assert_eq!(next, 0);
    }

    #[test]
    fn the_free_energy_caps_the_count_too() {
        // FUN_004e3f80: min(v[15], +0x5c) of the target's record.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module2.target = Target::System(f.empty);
        state.module2.room = 9;
        state.module2.capacity = 9;
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.empty).values[15] = 2;
        let next = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.plan_yards(io));
        assert_eq!((next, state.module2.count), (8, 2));
    }

    #[test]
    fn the_yard_cycle_orders_destination_and_build_at_its_target() {
        // FUN_004e2400: 4 (FUN_004e3a40, nothing to scrap) -> 5
        // (FUN_004e3f80: 8) -> 8 (Destination from the borrowed manager,
        // complete, raise) -> 9 (Build the yards) -> end.
        let f = fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module2.phase = YARD_CYCLE;
        state.module2.state = 4;
        state.module2.target = Target::System(f.empty);
        state.module2.yard = Yard::Manager(f.yard);
        state.module2.room = 9;
        state.module2.capacity = 2;

        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module2.state, 5);
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module2.state, 8);
        let s = step(&f, &mut state, ROOMY);
        assert!(s.complete && s.raise);
        assert_eq!(
            s.order,
            Some(AIAction::SetDestination {
                system: f.yard,
                area: crate::manufacturing::ProductionArea::ConstructionYard,
                destination: f.empty,
            })
        );
        let s = step(&f, &mut state, ROOMY);
        assert!(s.complete && !s.raise);
        let Some(AIAction::Build {
            system,
            count,
            kind,
            ..
        }) = s.order
        else {
            panic!("{s:?}");
        };
        assert_eq!((system, count), (f.yard, 2));
        assert_eq!(kind.class_dat_id(), Some(DatId::new(0x2a00_0003)));
        assert_eq!((state.module2.phase, state.module2.state), (0, 0));

        // Without a manager state 8 ends the cycle.
        state.module2.phase = YARD_CYCLE;
        state.module2.state = 8;
        state.module2.yard = Yard::System(f.yard);
        assert_eq!(step(&f, &mut state, ROOMY), Step::complete());
        assert_eq!((state.module2.phase, state.module2.state), (0, 0));
    }

    #[test]
    fn a_surplus_scraps_a_spare_yard_and_ends() {
        // FUN_004e2400 state 5 -> 6 for w < 0; state 6 (FUN_004e4020, module
        // 1's FUN_004dbe70 on +0x60) scraps the first yard of a site with
        // two or more, and ends.
        let mut f = fixture();
        add_yard(
            &mut f.world,
            f.yard,
            DatId::new(0x2a00_0003),
            Faction::Alliance,
        );
        let mut state = with_site(&f);
        state.module2.phase = YARD_CYCLE;
        state.module2.state = 5;
        let small = Tallies {
            capacity: 100,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, small), Step::pending());
        assert_eq!(state.module2.state, 6);

        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.yard).state |= 0x1000;
        let s = with_io(&f, &mut state, &mut galaxy, small, |m, io| m.step(io));

        assert_eq!(
            s,
            Step {
                order: Some(AIAction::Scrap {
                    team: vec![yards(&f, f.yard)[0]]
                }),
                complete: true,
                raise: false,
            }
        );
        assert_eq!(state.module2.target, Target::System(f.yard));
        assert_eq!((state.module2.phase, state.module2.state), (0, 0));
    }

    #[test]
    fn a_new_cycle_starts_at_state_one_and_a_stray_phase_resets() {
        // FUN_004e25c0 and FUN_004e2400: any other +0x3c gives 1;
        // FUN_004e2120: any other +0x38 is cleared. A refusal clears both
        // (FUN_004dd810).
        let f = fixture();
        for (phase, state_in) in [(MINE_CYCLE, 5), (REFINERY_CYCLE, 0), (YARD_CYCLE, 11)] {
            let mut state = with_site(&f);
            state.module2.phase = phase;
            state.module2.state = state_in;
            assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
            assert_eq!(state.module2.state, 1);
            assert_eq!(state.module2.phase, phase);
        }
        let mut state = with_site(&f);
        state.module2.phase = 0x3e9;
        state.module2.state = 4;
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!((state.module2.phase, state.module2.state), (0, 4));

        state.module2.phase = YARD_CYCLE;
        state.module2.refused();
        assert_eq!((state.module2.phase, state.module2.state), (0, 0));
        let _ = Controller::default();
    }
    #[test]
    fn a_kept_mine_site_wins_over_a_nearer_one() {
        // FUN_004e2c70: a calm +0x40 with +0x28 0x80 stays even when the
        // nearest query would give another system.
        let mut f = fixture();
        let far = system(&mut f.world, f.sector, 100);
        let mut state = with_site(&f);
        state.module2.yard = Yard::Manager(f.yard);
        state.module2.mine_site = Target::System(far);
        let mut galaxy = records_of(&f, &state);
        let found = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.find_mine_site(io)
        });
        assert!(found);
        assert_eq!(state.module2.mine_site, Target::System(far));

        // Unsettled, it gives way to the nearest.
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, far).state |= 1;
        with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.find_mine_site(io)
        });
        assert_eq!(state.module2.mine_site, Target::System(f.empty));
    }

    #[test]
    fn the_room_list_joins_the_query_only_when_sites_are_listed() {
        // FUN_004e3530: the key 0x33 list is merged only when +0x60 is not
        // empty (FUN_005f50e0). With A (v[6] 1, room 5) and B (v[6] 0,
        // room 9), key 6 alone picks A; with key 0x33 the places tie and
        // the first list's earlier entry, B, wins (FUN_004357b0).
        let mut f = fixture();
        let b = system(&mut f.world, f.sector, 9);
        let pick_with = |state: &mut AIState| {
            let mut galaxy = records_of(&f, state);
            let a = record(&mut galaxy, f.empty);
            a.values[6] = 1;
            a.values[0x33] = 5;
            let rb = record(&mut galaxy, b);
            rb.values[6] = 0;
            rb.values[0x33] = 9;
            record(&mut galaxy, f.yard).values[0x33] = 0;
            assert!(with_io(&f, state, &mut galaxy, ROOMY, |m, io| m.choose_target(io)));
            state.module2.target
        };
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        assert_eq!(pick_with(&mut state), Target::System(f.empty));
        let mut state = with_site(&f);
        assert_eq!(pick_with(&mut state), Target::System(b));
    }

    #[test]
    fn a_yard_class_without_maintenance_builds_nothing() {
        // FUN_004e3f80: +0x54 = w / m only with a class and m != 0;
        // otherwise it stays 0 and the plan gives 0.
        let mut f = fixture();
        f.world
            .buildable_classes
            .get_mut(&DatId::new(0x2a00_0003))
            .unwrap()
            .maintenance_cost = 0;
        let mut state = AIState::new(AiFaction::Alliance);
        state.module2.target = Target::System(f.empty);
        state.module2.room = 9;
        state.module2.capacity = 9;
        let mut galaxy = records_of(&f, &state);
        let next = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.plan_yards(io));
        assert_eq!((next, state.module2.count), (0, 0));
        assert_eq!(state.module2.class, Some(DatId::new(0x2a00_0003)));
    }
}
