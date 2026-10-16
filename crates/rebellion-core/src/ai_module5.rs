//! Module 5 of the AI player (`FUN_004cfb20`, step `FUN_004cfed0`): it
//! trains special forces at its own training facility sites (mark
//! `0x400000`) and, with a construction yard it borrows from module 1,
//! builds more training facilities (`ghidra/notes/ai-production-modules.md`,
//! "Module 5"). Its yard cycle is module 3's with training facilities
//! ([`crate::ai_module1::TRAINING`]).

use serde::{Deserialize, Serialize};

use crate::ai::{AIAction, MARK_TRAINING};
use crate::ai_module1::{
    adopt_or_request, best_class, build, claim, destination, scrap_at, scrap_spare_yard,
    yard_class, ClassKey, Inputs, Step, Target, Yard, ANY, TRAINING,
};
use crate::ai_module2::keep_site;
use crate::ai_module3::choose_target;
use crate::ai_records::{pick, Mask, Order};
use crate::ids::{DatId, SystemKey};
use crate::manufacturing::ProductionArea;

/// `+0x38` while the yard cycle runs (`FUN_004cfed0`).
pub const YARD_CYCLE: u32 = 0x3ec;

/// `+0x38` while the special force cycle runs (`FUN_004cfed0`).
pub const SPECIAL_CYCLE: u32 = 0x3f4;

/// The yard token module 5 holds the construction yards with (controller
/// `+0xc`, `+8`; `FUN_004d0010` state 9).
pub const YARD_TOKEN: u32 = 4;

/// `+0x24`: the percent of the maintenance capacity module 5 budgets
/// (`FUN_004cfb20`): 70% of it for special forces (`FUN_004d1d90`), 30%
/// for training facilities (`FUN_004d1df0`).
const PERCENT: i32 = 4;

/// The first special force kind `+0x40` names (`FUN_004d0480`).
const FIRST_KIND: u16 = 0x5a;

/// Module 5's own fields (`FUN_004cfb20`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module5 {
    /// `+0x38`: 0, [`SPECIAL_CYCLE`] or [`YARD_CYCLE`].
    pub phase: u32,
    /// `+0x3c`: the cycle's state.
    pub state: u32,
    /// `+0x40`: the next special force kind to test, `0x5a..0x5d`.
    pub next_kind: u16,
    /// `+0x44`: the target's free room `v[0x33]`.
    pub room: i32,
    /// `+0x48`: how many.
    pub count: i32,
    /// `+0x4c`: the borrowed yard's construction yards.
    pub capacity: i32,
    /// `+0x50`: the special force class flag (`0x1000000..0x8000000`).
    pub class_flags: u32,
    /// `+0x5c`: the yard cycle's target.
    pub target: Target,
    /// `+0x60`: a site, then its manager.
    pub yard: Yard,
    /// `+0x64`: the class to build.
    pub class: Option<DatId>,
    /// `+0x78`: the sites, marked [`MARK_TRAINING`].
    pub sites: Vec<SystemKey>,
    /// `+0x80`: the claimed sites.
    pub claimed: Vec<SystemKey>,
}

impl Module5 {
    /// One step (`FUN_004cfed0`). The scheduler steps only a ready module,
    /// so the not-ready branch never runs.
    pub fn step(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.phase {
            YARD_CYCLE => self.yard_cycle(io),
            SPECIAL_CYCLE => self.special_cycle(io),
            _ => {
                self.phase = SPECIAL_CYCLE;
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

    /// The special force cycle (`FUN_004cff20`).
    fn special_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            9 => {
                self.state =
                    if keep_site(io, &mut self.sites, &mut self.yard, MARK_TRAINING, TRAINING) {
                        1
                    } else {
                        6
                    };
                Step::pending()
            }
            1 => {
                self.state = if self.choose_kind(io) { 5 } else { 6 };
                Step::pending()
            }
            5 => {
                self.state = self.plan_special_forces(io);
                if self.state == 0 {
                    return self.end(YARD_CYCLE, None);
                }
                Step::pending()
            }
            6 => {
                if special_budget(io) < 0 {
                    self.state = 2;
                    Step::pending()
                } else {
                    self.end(YARD_CYCLE, None)
                }
            }
            // FUN_004d0610: the Retire query needs record flag 0x2000,
            // which no code sets and FUN_00402230 clears, so it finds
            // nothing and no order is given.
            2 => self.end(YARD_CYCLE, None),
            0xb => {
                let order = build(io, self.yard, self.class, self.count);
                self.end(YARD_CYCLE, order)
            }
            _ => {
                self.state = 9;
                Step::pending()
            }
        }
    }

    /// `FUN_004d0480`: from `+0x40`, the first special force kind with
    /// fewer ready units (`+0x2ec..+0x2f8`) than `max(+0x2fc, 8)`, one at
    /// a time; `+0x40` moves on past each kind tested.
    fn choose_kind(&mut self, io: &Inputs<'_>) -> bool {
        if self.next_kind == 0 {
            self.next_kind = FIRST_KIND;
        }
        let start = self.next_kind;
        let goal = io.tallies.special_goal.max(8);
        loop {
            let Some(k) = self.next_kind.checked_sub(FIRST_KIND).filter(|&k| k < 4) else {
                return false;
            };
            let k = usize::from(k);
            self.count = i32::from(io.tallies.special_ready[k] < goal);
            self.class_flags = 0x100_0000 << k;
            self.next_kind = if k == 3 {
                FIRST_KIND
            } else {
                self.next_kind + 1
            };
            if self.count > 0 {
                return true;
            }
            if self.next_kind == start {
                return false;
            }
        }
    }

    /// `FUN_004d0590`: `0xb` to train the kind's dearest class (at most
    /// what the budget pays for), 2 when special forces cost more than the
    /// budget, else 0.
    fn plan_special_forces(&mut self, io: &mut Inputs<'_>) -> u32 {
        let budget = special_budget(io);
        if budget < 0 {
            return 2;
        }
        if budget == 0 {
            return 0;
        }
        let maintenance;
        (self.class, maintenance) = best_class(
            io,
            ProductionArea::TrainingFacility,
            self.class_flags,
            ClassKey::Maintenance,
            Order::Ascending,
        );
        if self.class.is_some() && maintenance != 0 && maintenance * self.count > budget {
            self.count = budget / maintenance;
        }
        // port: the original also clears controller +0x18, which has no
        // port counterpart.
        if self.count > 0 {
            0xb
        } else {
            0
        }
    }

    /// The yard cycle (`FUN_004d0010`).
    fn yard_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            3 => {
                self.state = if choose_target(
                    io,
                    &mut self.sites,
                    &mut self.target,
                    &mut self.room,
                    MARK_TRAINING,
                    TRAINING,
                ) {
                    9
                } else {
                    7
                };
                Step::pending()
            }
            9 => {
                io.controller.yard_requests &= !YARD_TOKEN;
                if io.controller.yard_holder == YARD_TOKEN && self.borrow_yard(io) {
                    io.controller.pass_yards();
                    self.state = 4;
                } else {
                    self.state = 0xc;
                }
                Step::pending()
            }
            4 => {
                let order = scrap_at(io, self.target.system(), TRAINING);
                self.state = 5;
                Step::ordered(order)
            }
            5 => {
                self.state = self.plan_yards(io);
                if self.state == 0 {
                    return self.end(SPECIAL_CYCLE, None);
                }
                Step::pending()
            }
            7 => {
                if training_budget(io, &self.sites) < 0 {
                    self.state = 8;
                } else {
                    self.state = 0;
                    self.phase = SPECIAL_CYCLE;
                }
                Step::pending()
            }
            8 => {
                let order = scrap_spare_yard(
                    io,
                    &mut self.sites,
                    &mut self.target,
                    MARK_TRAINING,
                    TRAINING,
                );
                self.end(SPECIAL_CYCLE, order)
            }
            10 => match destination(self.target, self.yard) {
                Some(order) => {
                    self.state = 0xb;
                    Step {
                        order: Some(order),
                        complete: true,
                        raise: true,
                    }
                }
                None => self.end(SPECIAL_CYCLE, None),
            },
            0xb => {
                let order = build(io, self.yard, self.class, self.count);
                self.end(SPECIAL_CYCLE, order)
            }
            0xc => {
                let budget = training_budget(io, &self.sites);
                self.target = adopt_or_request(
                    io,
                    &mut self.sites,
                    &mut self.class,
                    MARK_TRAINING,
                    YARD_TOKEN,
                    budget,
                    TRAINING,
                );
                claim(io, &self.sites, &mut self.claimed, TRAINING);
                self.end(SPECIAL_CYCLE, None)
            }
            _ => {
                self.state = 3;
                Step::pending()
            }
        }
    }

    /// `FUN_004d0b40`: borrow the idle construction yard of module 1's
    /// site in the sector with `0x1000` by key `0x14`, its system by
    /// construction yards (order 1).
    fn borrow_yard(&mut self, io: &mut Inputs<'_>) -> bool {
        self.capacity = 0;
        let lists = vec![io.sectors([Mask::new(0x1000, 0), ANY, ANY], 0x14, Order::Ascending)];
        let sector = pick(&lists);
        let masks = [Mask::new(0x80_0800, 3), ANY, Mask::new(1, 0)];
        let lists = vec![io.systems(sector, masks, 6, Order::Ascending)];
        self.yard = Yard::None;
        let Some(system) = pick(&lists) else {
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

    /// `FUN_004d1270`: 10 to build training facilities, 8 to scrap one
    /// when the sites hold more than the side wants, else 0.
    fn plan_yards(&mut self, io: &mut Inputs<'_>) -> u32 {
        self.count = 0;
        let budget = training_budget(io, &self.sites);
        if budget < 0 {
            return 8;
        }
        if budget == 0 {
            return 0;
        }
        let maintenance;
        (self.class, maintenance) = yard_class(io, TRAINING);
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
        10
    }
}

/// `FUN_004d1d90`: 70% of `+0x24` percent of the capacity, less the
/// special forces' upkeep (`+0x2c4`); 0 when the side's free maintenance
/// falls short of it.
fn special_budget(io: &Inputs<'_>) -> i32 {
    let t = io.tallies;
    let budget = PERCENT * t.capacity / 100 * 70 / 100 - t.special_upkeep;
    if t.capacity - t.requirement < budget {
        0
    } else {
        budget
    }
}

/// `FUN_004d1df0`: 30% of `+0x24` percent of the capacity, less the sites'
/// training facility upkeep (`v[35]`); 0 when the side's free maintenance
/// falls short of it.
fn training_budget(io: &Inputs<'_>, sites: &[SystemKey]) -> i32 {
    let held: i32 = sites
        .iter()
        .filter_map(|&s| io.galaxy.system(s))
        .map(|r| r.values[35])
        .sum();
    let t = io.tallies;
    let budget = PERCENT * t.capacity / 100 * 30 / 100 - held;
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
    use crate::scrap::ScrapTarget;
    use crate::world::FacilityRef;

    const TRAINING_CLASS: u32 = 0x2900_0002;

    fn with_io<R>(
        f: &Fixture,
        state: &mut AIState,
        galaxy: &mut Galaxy,
        tallies: Tallies,
        run: impl FnOnce(&mut Module5, &mut Inputs<'_>) -> R,
    ) -> R {
        let research = ResearchState::new();
        let transport = crate::troop_transport::TroopTransportState::default();
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
            planners: &[],
            transport: &transport,
            coin: &mut coin,
        };
        run(&mut state.module5, &mut io)
    }

    fn edited<R>(
        f: &Fixture,
        state: &mut AIState,
        tallies: Tallies,
        edit: impl FnOnce(&mut Galaxy),
        run: impl FnOnce(&mut Module5, &mut Inputs<'_>) -> R,
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

    /// The fixture with a training facility at `yard`, the training
    /// facility class, and the four basic special force classes with
    /// maintenance 1 to 4 and an elite first kind (`0x3c000005`) at 5.
    fn training_fixture() -> Fixture {
        let mut f = fixture();
        let training = class(&mut f.world, TRAINING_CLASS, 10, 2, 0);
        add_yard(&mut f.world, f.yard, training, Faction::Alliance);
        for (raw, upkeep) in [
            (0x3c00_0001, 1),
            (0x3c00_0002, 2),
            (0x3c00_0003, 3),
            (0x3c00_0004, 4),
            (0x3c00_0005, 5),
        ] {
            class(&mut f.world, raw, upkeep, 0, 0);
        }
        f
    }

    /// `yard` as module 5's site.
    fn with_site(f: &Fixture) -> AIState {
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.sites.push(f.yard);
        state.yard_marks.insert(f.yard, MARK_TRAINING);
        state
    }

    fn idle(g: &mut Galaxy, s: SystemKey) {
        rec(g, s).state |= 0x400;
    }

    #[test]
    fn the_special_force_cycle_trains_the_first_kind_at_an_idle_site() {
        // FUN_004cff20: 0 -> 9 (FUN_004d01a0 keeps the marked site with a
        // training facility v[5], idle +0x28 0x400, and its troop
        // manager) -> 1 (FUN_004d0480: kind 0x5a, none ready) -> 5
        // (FUN_004d0590: 0x3c000005, the dearest of the kind, 5 within
        // 4% * 1000 * 70% = 28) -> 0xb (FUN_004d15f0: Build 0x212) -> end
        // with 0x3ec.
        let f = training_fixture();
        let mut state = with_site(&f);
        state.module5.phase = SPECIAL_CYCLE;
        let mut trail = Vec::new();
        let mut order = None;
        for _ in 0..8 {
            let s = edited(
                &f,
                &mut state,
                ROOMY,
                |g| idle(g, f.yard),
                |m, io| m.step(io),
            );
            trail.push((state.module5.state, s.complete));
            if s.complete {
                order = s.order;
                break;
            }
        }
        assert_eq!(
            trail,
            [(9, false), (1, false), (5, false), (0xb, false), (0, true)]
        );
        assert_eq!(
            order,
            Some(AIAction::Build {
                system: f.yard,
                kind: BuildableKind::SpecialForce(DatId::new(0x3c00_0005)),
                cost: 10,
                count: 1,
            })
        );
        assert_eq!(state.module5.phase, YARD_CYCLE);
        assert_eq!(state.module5.next_kind, 0x5b);
        assert_eq!(state.module5.class_flags, 0x100_0000);
    }

    #[test]
    fn a_stray_phase_starts_the_special_force_cycle_and_states_reset() {
        // FUN_004cfed0: any other +0x38 gives 0x3f4, +0x3c kept.
        // FUN_004cff20 default: 9; FUN_004d0010 default: 3. A refusal
        // clears both (FUN_004dd810).
        let f = training_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.state = 4;
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(
            (state.module5.phase, state.module5.state),
            (SPECIAL_CYCLE, 4)
        );
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module5.state, 9);
        state.module5.phase = YARD_CYCLE;
        state.module5.state = 1;
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module5.state, 3);
        state.module5.refused();
        assert_eq!((state.module5.phase, state.module5.state), (0, 0));
    }

    #[test]
    fn without_a_site_or_a_short_kind_the_cycle_ends_or_waits_on_its_budget() {
        // FUN_004cff20: 9 or 1 failing go to 6; 6 ends the cycle (0x3ec)
        // unless the budget is below 0, then 2: the Retire query
        // (FUN_004d0610) finds nothing, the cycle ends.
        let f = training_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.phase = SPECIAL_CYCLE;
        state.module5.state = 9;
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module5.state, 6);
        assert_eq!(step(&f, &mut state, ROOMY), Step::complete());
        assert_eq!((state.module5.phase, state.module5.state), (YARD_CYCLE, 0));

        let over = Tallies {
            special_upkeep: 29,
            ..ROOMY
        };
        state.module5.phase = SPECIAL_CYCLE;
        state.module5.state = 6;
        assert_eq!(step(&f, &mut state, over), Step::pending());
        assert_eq!(state.module5.state, 2);
        assert_eq!(step(&f, &mut state, over), Step::complete());
        assert_eq!((state.module5.phase, state.module5.state), (YARD_CYCLE, 0));

        let mut state = with_site(&f);
        state.module5.phase = SPECIAL_CYCLE;
        state.module5.state = 1;
        let full = Tallies {
            special_ready: [8; 4],
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, full), Step::pending());
        assert_eq!(state.module5.state, 6);
    }

    #[test]
    fn the_kind_turns_round_the_four_and_stops_where_it_began() {
        // FUN_004d0480: +0x40 from 0x5a; a kind with fewer ready units
        // (+0x2ec + 4k) than max(+0x2fc, 8) gives +0x48 = 1; +0x50 is its
        // flag; +0x40 moves on past each kind tested, 0x5d to 0x5a; with
        // none short the walk ends where it began.
        let f = training_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        let choose = |state: &mut AIState, t: Tallies| {
            edited(&f, state, t, |_| {}, |m, io| m.choose_kind(io))
        };
        let t = |ready: [i32; 4], goal: i32| Tallies {
            special_ready: ready,
            special_goal: goal,
            ..ROOMY
        };
        assert!(choose(&mut state, t([0; 4], 0)));
        assert_eq!(
            (
                state.module5.next_kind,
                state.module5.class_flags,
                state.module5.count
            ),
            (0x5b, 0x100_0000, 1)
        );
        // Kinds 2 and 3 ready (at the floor of 8), kind 4 short.
        assert!(choose(&mut state, t([0, 8, 8, 7], 0)));
        assert_eq!(
            (state.module5.next_kind, state.module5.class_flags),
            (0x5a, 0x800_0000)
        );
        // The goal above 8 rules.
        assert!(choose(&mut state, t([9, 0, 0, 0], 10)));
        assert_eq!(state.module5.class_flags, 0x100_0000);
        assert!(!choose(&mut state, t([10; 4], 10)));
        assert_eq!((state.module5.next_kind, state.module5.count), (0x5b, 0));
        assert_eq!(state.module5.class_flags, 0x100_0000);
        state.module5.next_kind = 0x60;
        assert!(!choose(&mut state, t([0; 4], 0)));
    }

    #[test]
    fn the_plan_trains_what_the_budget_pays_for() {
        // FUN_004d0590: w = FUN_004d1d90; w < 0 gives 2, w == 0 gives 0;
        // the kind's dearest class (key 1, the maintenance, order 1);
        // when m * +0x48 > w, +0x48 = w / m; then 0xb, or 0 with none.
        let f = training_fixture();
        let plan = |count: i32, flags: u32, upkeep: i32| {
            let mut state = AIState::new(AiFaction::Alliance);
            state.module5.count = count;
            state.module5.class_flags = flags;
            let t = Tallies {
                special_upkeep: upkeep,
                ..ROOMY
            };
            let next = edited(&f, &mut state, t, |_| {}, |m, io| m.plan_special_forces(io));
            (next, state.module5.count, state.module5.class)
        };
        let id = |raw| Some(DatId::new(raw));
        // Budget 28.
        assert_eq!(plan(1, 0x100_0000, 0), (0xb, 1, id(0x3c00_0005)));
        assert_eq!(plan(9, 0x100_0000, 0), (0xb, 5, id(0x3c00_0005)));
        assert_eq!(plan(1, 0x200_0000, 26), (0xb, 1, id(0x3c00_0002)));
        assert_eq!(plan(1, 0x800_0000, 25), (0, 0, id(0x3c00_0004)));
        // A budget of 0 plans nothing and leaves the class alone.
        assert_eq!(plan(1, 0x100_0000, 28), (0, 1, None));
        assert_eq!(plan(1, 0x100_0000, 29).0, 2);
        // No class of the kind: the count stands.
        assert_eq!(plan(1, 0x40_0000, 0), (0xb, 1, None));
    }

    #[test]
    fn the_budgets_are_seventy_and_thirty_percent_of_four() {
        // FUN_004d1d90: 4 * capacity / 100 * 70 / 100 - +0x2c4.
        // FUN_004d1df0: the same with 30, less the sites' v[35]. Both 0
        // when capacity - requirement falls below them.
        let f = training_fixture();
        let mut state = with_site(&f);
        let t = |requirement, upkeep| Tallies {
            requirement,
            special_upkeep: upkeep,
            ..ROOMY
        };
        let run = |state: &mut AIState, t: Tallies| {
            edited(
                &f,
                state,
                t,
                |_| {},
                |m, io| (special_budget(io), training_budget(io, &m.sites)),
            )
        };
        // 4% of 1000 is 40: 28 and 12, less the site's training upkeep 10.
        assert_eq!(run(&mut state, t(0, 0)), (28, 2));
        assert_eq!(run(&mut state, t(0, 30)), (-2, 2));
        assert_eq!(run(&mut state, t(972, 0)), (28, 2));
        assert_eq!(run(&mut state, t(973, 0)), (0, 2));
        assert_eq!(run(&mut state, t(998, 0)), (0, 2));
        assert_eq!(run(&mut state, t(999, 0)), (0, 0));
    }

    #[test]
    fn a_training_site_needs_a_facility_an_idle_one_and_calm() {
        // FUN_004d01a0 (FUN_004e21d0 with v[5], +0x28 0x400 and the troop
        // manager): a site without them stays listed but is not kept; one
        // that lost its mark leaves.
        let f = training_fixture();
        let changes: [fn(&mut SystemRecord); 4] = [
            |r| r.state &= !0x400,
            |r| r.values[5] = 0,
            |r| r.state |= 1,
            |r| r.state |= 2,
        ];
        for change in changes {
            let mut state = with_site(&f);
            let kept = edited(
                &f,
                &mut state,
                ROOMY,
                |g| {
                    idle(g, f.yard);
                    change(rec(g, f.yard));
                },
                |m, io| keep_site(io, &mut m.sites, &mut m.yard, MARK_TRAINING, TRAINING),
            );
            assert!(!kept);
            assert_eq!(state.module5.sites, vec![f.yard]);
        }
        // An idle site's yard without a training facility has no manager.
        let mut f = training_fixture();
        let k = f.world.systems[f.yard]
            .manufacturing_facilities
            .pop()
            .unwrap();
        f.world.manufacturing_facilities.remove(k);
        let mut state = with_site(&f);
        let kept = edited(
            &f,
            &mut state,
            ROOMY,
            |g| {
                idle(g, f.yard);
                rec(g, f.yard).values[5] = 1;
            },
            |m, io| keep_site(io, &mut m.sites, &mut m.yard, MARK_TRAINING, TRAINING),
        );
        assert!(!kept);
        assert_eq!(state.module5.yard, Yard::System(f.yard));

        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.sites.push(f.yard);
        state.yard_marks.insert(f.yard, MARK_MINES);
        let kept = edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| keep_site(io, &mut m.sites, &mut m.yard, MARK_TRAINING, TRAINING),
        );
        assert!(!kept);
        assert!(state.module5.sites.is_empty());
    }

    #[test]
    fn the_yard_cycle_targets_a_new_site_and_borrows_module_one_s_yard() {
        // FUN_004d0010: 3 (FUN_004d0860: sector +0x24 0x40 by 0x13, a
        // system forbidding 0x400003 by key 5, marked 0x400000: the yard,
        // with the most training facilities, as module 1's mark 0x800000
        // is not forbidden) -> 9 (bit
        // 4 cleared; +0xc == 4 and FUN_004d0b40: the sector with 0x1000
        // by 0x14, its system with 0x800800 by key 6; +0x4c = v[6]) ->
        // FUN_004199b0, 4.
        let mut f = training_fixture();
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
        state.module5.phase = YARD_CYCLE;
        state.module5.state = 3;
        let pass = |g: &mut Galaxy| g.sectors[0].candidates |= 0x40;
        assert_eq!(
            edited(&f, &mut state, ROOMY, pass, |m, io| m.step(io)),
            Step::pending()
        );
        assert_eq!(state.module5.state, 9);
        assert_eq!(state.module5.target, Target::System(f.yard));
        assert_eq!(state.module5.sites, vec![f.yard]);
        assert_eq!(
            state.yard_marks.get(&f.yard),
            Some(&(MARK_MINES | MARK_TRAINING))
        );
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module5.state, 4);
        assert_eq!(state.module5.yard, Yard::Manager(f.yard));
        assert_eq!(state.module5.capacity, 1);
        assert_eq!(state.controller.yard_holder, 0x80);
        assert_eq!(state.controller.yard_requests, 0);
        // The borrowed yard adds no site (FUN_004d0b40).
        assert_eq!(state.module5.sites, vec![f.yard]);

        // One yard is lent whole; no sector or manager: none.
        let f2 = training_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f2.yard, MARK_MINES);
        assert!(edited(
            &f2,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| m.borrow_yard(io)
        ));
        assert_eq!(state.module5.capacity, 1);
        let none = |g: &mut Galaxy| g.sectors[0].candidates &= !0x1000;
        assert!(!edited(&f2, &mut state, ROOMY, none, |m, io| m.borrow_yard(io)));
        assert_eq!(
            (state.module5.yard, state.module5.capacity),
            (Yard::None, 0)
        );
        let mut f3 = training_fixture();
        let k = f3.world.systems[f3.yard].manufacturing_facilities.remove(0);
        f3.world.manufacturing_facilities.remove(k);
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f3.yard, MARK_MINES);
        let lent = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x1000;
            let r = rec(g, f3.yard);
            r.state |= 0x800;
            r.values[6] = 1;
        };
        assert!(!edited(&f3, &mut state, ROOMY, lent, |m, io| m.borrow_yard(io)));
        assert_eq!(state.module5.yard, Yard::System(f3.yard));

        // Without the token: 0xc.
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.phase = YARD_CYCLE;
        state.module5.state = 9;
        state.controller.yard_requests = YARD_TOKEN;
        step(&f, &mut state, ROOMY);
        assert_eq!(state.module5.state, 0xc);
        assert_eq!(state.controller.yard_requests, 0);
    }

    #[test]
    fn the_yard_cycle_scraps_unmarked_construction_yards_and_builds_training_facilities() {
        // FUN_004d0010 4: FUN_004d0d30 takes the target's construction
        // yards unless marked 0x3800000 -> 5: FUN_004d1270 (30% of 4%,
        // the training class) gives 10 -> Destination -> 0xb: Build ->
        // end with 0x3f4.
        let f = training_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.phase = YARD_CYCLE;
        state.module5.state = 4;
        state.module5.target = Target::System(f.yard);
        state.module5.yard = Yard::Manager(f.yard);
        state.module5.room = 9;
        state.module5.capacity = 9;
        let yards = |f: &Fixture| -> Vec<ScrapTarget> {
            f.world.systems[f.yard]
                .manufacturing_facilities
                .iter()
                .map(|&k| ScrapTarget::Facility(FacilityRef::Manufacturing(k)))
                .collect()
        };
        let s = step(&f, &mut state, ROOMY);
        assert_eq!(
            s.order,
            Some(AIAction::Scrap {
                team: vec![yards(&f)[0]]
            })
        );
        assert_eq!(state.module5.state, 5);
        let rich = Tallies {
            capacity: 4000,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, rich), Step::pending());
        assert_eq!((state.module5.state, state.module5.count), (10, 4));
        let s = step(&f, &mut state, rich);
        assert!(s.complete && s.raise);
        let s = step(&f, &mut state, rich);
        let Some(AIAction::Build { kind, count, .. }) = s.order else {
            panic!("{s:?}");
        };
        assert_eq!(
            (kind.class_dat_id(), count),
            (Some(DatId::new(TRAINING_CLASS)), 4)
        );
        assert_eq!(
            (state.module5.phase, state.module5.state),
            (SPECIAL_CYCLE, 0)
        );

        // Marked for construction: spared.
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        let order = edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |_, io| scrap_at(io, Some(f.yard), TRAINING),
        );
        assert_eq!(order, None);
    }

    #[test]
    fn the_training_plan_and_the_end_of_the_yard_cycle() {
        // FUN_004d1270: w < 0 gives 8, w == 0 gives 0, w / m capped by
        // +0x4c and min(v[15], +0x44) gives 10. State 5 at 0 ends with
        // 0x3f4; state 7 (no target) goes to 8 below budget, else back to
        // 0x3f4; 8 (FUN_004d1310) scraps a spare training facility; 10
        // without a manager ends.
        let f = training_fixture();
        let plan = |capacity: i32, sites: Vec<SystemKey>, room: i32, lent: i32| {
            let mut state = AIState::new(AiFaction::Alliance);
            state.module5.sites = sites;
            state.module5.target = Target::System(f.empty);
            state.module5.room = room;
            state.module5.capacity = lent;
            let t = Tallies { capacity, ..ROOMY };
            let next = edited(&f, &mut state, t, |_| {}, |m, io| m.plan_yards(io));
            (next, state.module5.count)
        };
        // 30% of 4% of 4000 is 48: 4 of 10.
        assert_eq!(plan(4000, vec![], 9, 9), (10, 4));
        assert_eq!(plan(4000, vec![], 9, 2), (10, 2));
        assert_eq!(plan(4000, vec![], 1, 9), (10, 1));
        // 30% of 4% of 1000 is 12, less the site's 10: 2, below a yard.
        assert_eq!(plan(1000, vec![f.yard], 9, 9), (0, 0));
        // 6 less 10 is below 0.
        assert_eq!(plan(500, vec![f.yard], 9, 9), (8, 0));
        assert_eq!(plan(1000, vec![], 9, 9), (10, 1));
        assert_eq!(plan(800, vec![], 9, 9), (0, 0));
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.room = 9;
        state.module5.capacity = 9;
        let t = Tallies {
            capacity: 4000,
            ..ROOMY
        };
        assert_eq!(
            edited(&f, &mut state, t, |_| {}, |m, io| m.plan_yards(io)),
            0
        );

        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.phase = YARD_CYCLE;
        state.module5.state = 5;
        let small = Tallies {
            capacity: 800,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, small), Step::complete());
        assert_eq!(
            (state.module5.phase, state.module5.state),
            (SPECIAL_CYCLE, 0)
        );

        state.module5.phase = YARD_CYCLE;
        state.module5.state = 7;
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(
            (state.module5.phase, state.module5.state),
            (SPECIAL_CYCLE, 0)
        );

        let mut f = training_fixture();
        add_yard(
            &mut f.world,
            f.yard,
            DatId::new(TRAINING_CLASS),
            Faction::Alliance,
        );
        let mut state = with_site(&f);
        state.module5.phase = YARD_CYCLE;
        state.module5.state = 7;
        let poor = Tallies {
            capacity: 500,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, poor), Step::pending());
        assert_eq!(state.module5.state, 8);
        let s = edited(
            &f,
            &mut state,
            poor,
            |g| rec(g, f.yard).state |= 0x1000,
            |m, io| m.step(io),
        );
        let training: Vec<_> = f.world.systems[f.yard]
            .manufacturing_facilities
            .iter()
            .filter(|&&k| f.world.manufacturing_facilities[k].class_dat_id.family() == 0x29)
            .map(|&k| ScrapTarget::Facility(FacilityRef::Manufacturing(k)))
            .collect();
        assert_eq!(
            s.order,
            Some(AIAction::Scrap {
                team: vec![training[0]]
            })
        );
        assert_eq!(
            (state.module5.phase, state.module5.state),
            (SPECIAL_CYCLE, 0)
        );

        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.phase = YARD_CYCLE;
        state.module5.state = 10;
        state.module5.target = Target::System(f.empty);
        state.module5.yard = Yard::System(f.yard);
        assert_eq!(step(&f, &mut state, ROOMY), Step::complete());
        assert_eq!(
            (state.module5.phase, state.module5.state),
            (SPECIAL_CYCLE, 0)
        );
    }

    #[test]
    fn state_c_adopts_a_training_site_only_with_fewer_than_two_sites() {
        // FUN_004d1970 (with fewer than two sites): sectors 0x40 by 0x13
        // with unmarked training facilities (+0xbc), a system with v[5] >
        // 0 and v[35] within the budget, marked 0x400000; else, the first
        // site without one, the request bit 4 when the training class
        // fits. FUN_004d1ed0 claims by v[5].
        let f = training_fixture();
        let rich = Tallies {
            capacity: 4000,
            ..ROOMY
        };
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.phase = YARD_CYCLE;
        state.module5.state = 0xc;
        let s = edited(
            &f,
            &mut state,
            rich,
            |g| rec(g, f.yard).values[18] = 1,
            |m, io| m.step(io),
        );
        assert_eq!(s, Step::complete());
        assert_eq!(state.module5.sites, vec![f.yard]);
        assert_eq!(state.yard_marks.get(&f.yard), Some(&MARK_TRAINING));
        assert_eq!(state.module5.claimed, vec![f.yard]);

        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.sites = vec![f.empty, f.empty];
        state.module5.phase = YARD_CYCLE;
        state.module5.state = 0xc;
        step(&f, &mut state, rich);
        assert_eq!(state.module5.sites, vec![f.empty, f.empty]);
        assert_eq!(state.controller.yard_requests, 0);

        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.sites = vec![f.empty];
        state.module5.phase = YARD_CYCLE;
        state.module5.state = 0xc;
        let none = |g: &mut Galaxy| rec(g, f.yard).values[5] = 0;
        edited(&f, &mut state, rich, none, |m, io| m.step(io));
        assert_eq!(state.controller.yard_requests, YARD_TOKEN);
        assert_eq!(state.module5.class, Some(DatId::new(TRAINING_CLASS)));
    }

    #[test]
    fn the_special_force_goal_is_the_most_of_the_sectors() {
        // FUN_0041b7e0: controller +0x2fc keeps the largest sector +0x140.
        let f = training_fixture();
        let state = AIState::new(AiFaction::Alliance);
        let mut g = records_of(&f, &state);
        g.sectors[0].values[crate::ai_records::sector_at(0x140)] = 11;
        g.sectors.push(g.sectors[0].clone());
        g.sectors[1].values[crate::ai_records::sector_at(0x140)] = 3;
        assert_eq!(Tallies::of(&g, 0, 0).special_goal, 11);
        g.sectors[0].values[crate::ai_records::sector_at(0x140)] = -2;
        assert_eq!(Tallies::of(&g, 0, 0).special_goal, 3);
    }

    #[test]
    fn a_budget_of_exactly_zero_neither_scraps_nor_plans() {
        // FUN_004d0570 and FUN_004d0d10 test the budgets below 0;
        // FUN_004d1270 gives 8 only below 0. Free maintenance 1 caps the
        // budgets at 0.
        let f = training_fixture();
        let zero = Tallies {
            requirement: 999,
            ..ROOMY
        };
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.phase = SPECIAL_CYCLE;
        state.module5.state = 6;
        assert_eq!(step(&f, &mut state, zero), Step::complete());
        assert_eq!((state.module5.phase, state.module5.state), (YARD_CYCLE, 0));
        state.module5.state = 7;
        assert_eq!(step(&f, &mut state, zero), Step::pending());
        assert_eq!(
            (state.module5.phase, state.module5.state),
            (SPECIAL_CYCLE, 0)
        );
        let next = edited(&f, &mut state, zero, |_| {}, |m, io| m.plan_yards(io));
        assert_eq!(next, 0);
    }

    #[test]
    fn a_kind_past_the_fourth_ends_the_walk() {
        // FUN_004d0480's switch covers 0x5a..0x5d only.
        let f = training_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.next_kind = 0x5e;
        let found = edited(&f, &mut state, ROOMY, |_| {}, |m, io| m.choose_kind(io));
        assert!(!found);
        assert_eq!(state.module5.next_kind, 0x5e);
    }

    #[test]
    fn a_training_class_without_maintenance_plans_no_facilities() {
        // FUN_004d1270: +0x48 = w / m only with a class and m != 0.
        let mut f = training_fixture();
        f.world
            .buildable_classes
            .get_mut(&DatId::new(TRAINING_CLASS))
            .unwrap()
            .maintenance_cost = 0;
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.target = Target::System(f.empty);
        state.module5.room = 9;
        state.module5.capacity = 9;
        let rich = Tallies {
            capacity: 4000,
            ..ROOMY
        };
        let next = edited(&f, &mut state, rich, |_| {}, |m, io| m.plan_yards(io));
        assert_eq!((next, state.module5.count), (0, 0));
        assert_eq!(state.module5.class, Some(DatId::new(TRAINING_CLASS)));
    }

    #[test]
    fn only_classes_with_the_kind_s_flag_are_chosen() {
        // FUN_0042e3c0 keeps a class record only when it has all of the
        // flags: a special force class of no known kind (flags 0) is
        // never one of them, however dear.
        let mut f = training_fixture();
        class(&mut f.world, 0x3c00_0009, 20, 0, 0);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module5.count = 1;
        state.module5.class_flags = 0x100_0000;
        let next = edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| m.plan_special_forces(io),
        );
        assert_eq!(next, 0xb);
        assert_eq!(state.module5.class, Some(DatId::new(0x3c00_0005)));
    }
}
