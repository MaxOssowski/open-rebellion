//! Module 8 of the AI player (`FUN_004cf410`, step `FUN_004cf600` ->
//! `FUN_004cf630`): the sector planners. It keeps one planner (controller
//! `+0xd8`, `FUN_004762b0`) per sector the strategy plans for, each claiming
//! a planned sector and binding the plan object of that sector's kind, and
//! steps one planner a cycle (`ghidra/notes/ai-sector-plans.md`, "Module
//! 8").

use serde::{Deserialize, Serialize};

use crate::ai_module1::ANY;
use crate::ai_records::{pick, ranked, Galaxy, Mask, Order};
use crate::ids::SectorKey;

/// Planner `+0x60` bit `0x4000000`: the plan object's cycle has run
/// (kind 3's slot `+0x20`, `FUN_004b3830`). Module 8 adds a planner only
/// while every planner has it.
pub const CYCLED: u32 = 0x400_0000;

/// Planner `+0x60` bit `0x8000000`: the plan's sector is still the
/// side's to work on (`FUN_004b3830`); the strategy's review reads and
/// clears it (`FUN_004e5560`).
pub const REVIEWED: u32 = 0x800_0000;

/// One sector planner (`FUN_004762b0`, vtable `0x65a3b0`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectorPlanner {
    /// `+0x18`: its id in the list.
    pub id: u32,
    /// `+0x1c`: the plan object's step state (`FUN_004774c0`).
    pub state: u32,
    /// `+0x34`: the sector it claimed.
    pub sector: Option<SectorKey>,
    /// `+0x44`: the id of the strategy's plan it works (`FUN_004769f0`).
    pub plan_id: u32,
    /// `+0x5c`'s kind (slot `+0x14`): the plan object it holds, 1 to 6, 0
    /// for none (`FUN_00477630`).
    pub kind: u32,
    /// `+0x60`: its flags ([`CYCLED`], [`REVIEWED`]).
    pub flags: u32,
}

/// What a step of module 8 leaves to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step8 {
    /// Not complete; the scheduler steps the module again.
    Pending,
    /// Complete without an order.
    Done,
    /// State 4 for a planner with a plan object: its step
    /// (`FUN_004774c0`).
    Plan,
}

/// Module 8's own fields (`FUN_004cf410`: `+0x38..+0x44`) and the
/// controller's planner list (`+0xd8`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module8 {
    /// `+0x3c`: the step's state.
    pub state: u32,
    /// `+0x40`: the planner this cycle steps.
    pub current: u32,
    /// `+0x44`: the planner the next cycle steps.
    pub next: u32,
    /// Controller `+0xd8`: the planners, in id order.
    pub planners: Vec<SectorPlanner>,
    /// The list's next id (`FUN_005f52c0` starts it at 1).
    pub next_id: u32,
    /// port: the fleet heuristics standing in for the plan object have
    /// decided this cycle's orders.
    pub decided: bool,
}

impl Default for Module8 {
    fn default() -> Self {
        Self {
            state: 0,
            current: 0,
            next: 0,
            planners: Vec::new(),
            next_id: 1,
            decided: false,
        }
    }
}

/// What a step reads: the records, the controller's planner balance
/// `+0x204` and the AI's generator.
pub struct Inputs8<'a> {
    pub galaxy: &'a mut Galaxy,
    /// Controller `+0x204` ([`crate::ai_module1::Tallies::planner_balance`]).
    pub balance: i32,
    /// `FUN_0041cd80(n)`: a draw below `n`.
    pub random: &'a mut dyn FnMut(i32) -> i32,
}

impl Module8 {
    /// One step (`FUN_004cf630`).
    pub fn step(&mut self, io: &mut Inputs8<'_>) -> Step8 {
        match self.state {
            1 => {
                self.balance(io);
                self.state = 2;
                Step8::Pending
            }
            2 => {
                if self.choose_planner() {
                    self.state = 3;
                    Step8::Pending
                } else {
                    self.end()
                }
            }
            3 => {
                if self.claim(io) {
                    self.state = 4;
                    Step8::Pending
                } else {
                    self.end()
                }
            }
            4 => {
                let bound = self
                    .planners
                    .iter()
                    .any(|p| p.id == self.current && p.kind != 0);
                if bound {
                    Step8::Plan
                } else {
                    // FUN_004774c0 without a plan object: the cycle ends.
                    self.end()
                }
            }
            _ => {
                self.state = 1;
                Step8::Pending
            }
        }
    }

    /// A refused order (slot `+0x18`, `FUN_004dd810`).
    pub fn refused(&mut self) {
        self.state = 0;
        self.decided = false;
    }

    /// The cycle's end: `+0x3c = 0`, complete.
    pub fn end(&mut self) -> Step8 {
        self.state = 0;
        self.decided = false;
        Step8::Done
    }

    /// `FUN_004cf700`: with planned sectors short of a planner
    /// (`+0x204 < 0`) and every planner cycled, a new planner
    /// (`FUN_0042f350`); with planners short of a plan (`> 0`), drop one
    /// (`FUN_0042f3d0`).
    fn balance(&mut self, io: &mut Inputs8<'_>) {
        if io.balance < 0 {
            if self.planners.iter().all(|p| p.flags & CYCLED != 0) {
                let id = self.next_id;
                self.next_id += 1;
                self.planners.push(SectorPlanner {
                    id,
                    ..SectorPlanner::default()
                });
            }
        } else if io.balance > 0 {
            self.drop_one(io.galaxy);
        }
    }

    /// `FUN_0042f3d0`: the first planner without a sector, or whose
    /// sector has `+0x18c > 0` (a planner without a plan), is reset
    /// (`FUN_00477190`) and leaves the list.
    fn drop_one(&mut self, galaxy: &mut Galaxy) {
        let at = crate::ai_records::sector_at(0x18c);
        let found = self.planners.iter().position(|p| {
            p.sector
                .is_none_or(|s| galaxy.sector(s).is_some_and(|r| r.values[at] > 0))
        });
        if let Some(i) = found {
            reset(&mut self.planners[i], galaxy);
            self.planners.remove(i);
        }
    }

    /// `FUN_004cf760`: the planner `+0x44` names, else the first, into
    /// `+0x40`, and the one after it into `+0x44`.
    fn choose_planner(&mut self) -> bool {
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

    /// `FUN_004cf7c0`: the planner keeps its sector while it holds it and
    /// the sector's `+0x18c` is 0; else it claims the planned sector
    /// without a planner (`+0x2c` `0x10000`) nearest it (`FUN_00419b80`,
    /// order 2), else one the strategy planned (`0x8010000`), else any
    /// (`0x10000`), each by a random key (`FUN_0041a1d0`, order 2). Then
    /// it binds the plan object (`FUN_004773f0`).
    fn claim(&mut self, io: &mut Inputs8<'_>) -> bool {
        let Some(i) = self.planners.iter().position(|p| p.id == self.current) else {
            return false;
        };
        let id = self.planners[i].id;
        let mut held = false;
        if let Some(sector) = self.planners[i].sector {
            let at = crate::ai_records::sector_at(0x18c);
            held = io
                .galaxy
                .sector(sector)
                .is_some_and(|r| r.values[at] == 0 && r.planner == id);
            if !held {
                let random = &mut *io.random;
                let near = io.galaxy.sectors_near(
                    sector,
                    [ANY, ANY, Mask::new(0x1_0000, 0)],
                    Order::Descending,
                    &mut || random(10) < 5,
                );
                if let Some(found) = pick(&[near]) {
                    take(&mut self.planners[i], io.galaxy, found);
                    held = true;
                }
            }
        }
        if !held {
            for bits in [0x801_0000, 0x1_0000] {
                let sectors = random_sectors(io, Mask::new(bits, 0));
                if let Some(found) = pick(&[sectors]) {
                    take(&mut self.planners[i], io.galaxy, found);
                    held = true;
                    break;
                }
            }
            if !held {
                return false;
            }
        }
        bind(&mut self.planners[i], io.galaxy)
    }
}

/// `FUN_0041a1d0`: the sectors whose `+0x2c` passes `summary`, each keyed
/// on a draw below 20 (`FUN_0041cd80(0x14)`) as it goes into the list
/// (order 2), so the key draws and the tie coins interleave.
fn random_sectors(io: &mut Inputs8<'_>, summary: Mask) -> Vec<SectorKey> {
    let sectors: Vec<SectorKey> = io
        .galaxy
        .sectors
        .iter()
        .filter(|s| s.admitted([ANY, ANY, summary]))
        .map(|s| s.sector)
        .collect();
    let draws = std::cell::RefCell::new(&mut *io.random);
    let items = sectors.into_iter().map(|s| (s, (draws.borrow_mut())(0x14)));
    ranked(items, Order::Descending, &mut || {
        (draws.borrow_mut())(10) < 5
    })
}

/// `FUN_004768b0`: release the planner's sector (its `+0x38` when it is
/// this planner's) and claim `sector` when no planner holds it, taking its
/// plan's id (`FUN_004769f0`).
fn take(planner: &mut SectorPlanner, galaxy: &mut Galaxy, sector: SectorKey) {
    if let Some(old) = planner.sector.take() {
        if let Some(r) = galaxy.sectors.iter_mut().find(|r| r.sector == old) {
            if r.planner == planner.id {
                r.planner = 0;
            }
        }
    }
    if let Some(r) = galaxy.sectors.iter_mut().find(|r| r.sector == sector) {
        if r.planner == 0 {
            r.planner = planner.id;
            planner.sector = Some(sector);
            planner.plan_id = r.plan_id;
        }
    }
    if planner.sector.is_none() {
        planner.plan_id = 0;
    }
}

/// `FUN_004773f0`: keep the plan object while its kind is the sector's
/// plan (`+0x30`), else build the one of that kind (`FUN_00477630`), the
/// step state starting over; follow the plan's id. Without one the
/// planner resets (`FUN_00477190`).
fn bind(planner: &mut SectorPlanner, galaxy: &mut Galaxy) -> bool {
    let Some(record) = planner.sector.and_then(|s| galaxy.sector(s)) else {
        reset(planner, galaxy);
        return false;
    };
    let (plan, plan_id) = (record.plan, record.plan_id);
    if planner.kind != plan {
        planner.kind = 0;
        planner.state = 0;
        if plan != 0 {
            planner.kind = plan;
        }
    }
    if plan_id != planner.plan_id {
        planner.plan_id = plan_id;
    }
    if planner.kind == 0 {
        reset(planner, galaxy);
        return false;
    }
    true
}

/// `FUN_00477190`: drop the plan object and the step state, and release
/// the sector (its `+0x38`) and the plan's id.
///
/// port: the planner holds no fleets, units or missions yet
/// (`FUN_004790d0`, `FUN_004791d0`, `FUN_0042f690`, `FUN_0042ef90`), nor a
/// system whose `+0x30` bits `0x70000` it would clear.
pub fn reset(planner: &mut SectorPlanner, galaxy: &mut Galaxy) {
    planner.kind = 0;
    planner.state = 0;
    if let Some(sector) = planner.sector.take() {
        if let Some(r) = galaxy.sectors.iter_mut().find(|r| r.sector == sector) {
            r.planner = 0;
        }
    }
    planner.plan_id = 0;
}

/// The refresh's `+0x38`: each planner's sector names it.
pub fn mark_planners(galaxy: &mut Galaxy, planners: &[SectorPlanner]) {
    for p in planners {
        if let Some(r) = p
            .sector
            .and_then(|s| galaxy.sectors.iter_mut().find(|r| r.sector == s))
        {
            r.planner = p.id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_records::{sector_at, SectorRecord};

    fn galaxy(n: usize) -> (Galaxy, Vec<SectorKey>) {
        let mut keys = slotmap::SlotMap::<SectorKey, ()>::with_key();
        let sectors: Vec<SectorRecord> = (0..n)
            .map(|i| {
                let x = i16::try_from(i).unwrap() * 10;
                SectorRecord::build(keys.insert(()), (x, 0), vec![])
            })
            .collect();
        let keys = sectors.iter().map(|s| s.sector).collect();
        (
            Galaxy {
                sectors,
                ..Galaxy::default()
            },
            keys,
        )
    }

    /// A planned sector without a planner, as the refresh leaves it.
    fn planned(g: &mut Galaxy, at: usize, kind: u32, id: u32) {
        let r = &mut g.sectors[at];
        r.plan = kind;
        r.plan_id = id;
        r.summary |= 0x800_0000;
        r.plan_bits();
    }

    fn run<R>(
        m: &mut Module8,
        g: &mut Galaxy,
        balance: i32,
        draws: &mut Vec<i32>,
        f: impl FnOnce(&mut Module8, &mut Inputs8<'_>) -> R,
    ) -> R {
        let mut random = |n: i32| {
            let d = if draws.is_empty() { 0 } else { draws.remove(0) };
            d.rem_euclid(n)
        };
        let mut io = Inputs8 {
            galaxy: g,
            balance,
            random: &mut random,
        };
        f(m, &mut io)
    }

    fn planner(id: u32, sector: Option<SectorKey>, kind: u32) -> SectorPlanner {
        SectorPlanner {
            id,
            sector,
            kind,
            ..SectorPlanner::default()
        }
    }

    #[test]
    fn a_planned_sector_short_of_a_planner_adds_one_once_all_have_cycled() {
        // FUN_004cf700: controller +0x204 < 0 and no planner without +0x60
        // 0x4000000: FUN_0042f350 adds a planner, its id from the list's
        // counter (FUN_005f5440, from 1).
        let (mut g, _) = galaxy(1);
        let mut m = Module8::default();
        run(&mut m, &mut g, -1, &mut vec![], |m, io| m.balance(io));
        assert_eq!(m.planners, vec![planner(1, None, 0)]);
        run(&mut m, &mut g, -1, &mut vec![], |m, io| m.balance(io));
        assert_eq!(m.planners.len(), 1);
        m.planners[0].flags = CYCLED;
        run(&mut m, &mut g, -1, &mut vec![], |m, io| m.balance(io));
        assert_eq!(
            m.planners.iter().map(|p| p.id).collect::<Vec<_>>(),
            vec![1, 2]
        );
        run(&mut m, &mut g, 0, &mut vec![], |m, io| m.balance(io));
        assert_eq!(m.planners.len(), 2);
    }

    #[test]
    fn a_planner_without_a_plan_is_dropped_when_planners_outnumber_plans() {
        // FUN_0042f3d0 (+0x204 > 0): the first planner with no sector, or
        // whose sector has +0x18c > 0, is reset (FUN_00477190: its sector's
        // +0x38 cleared) and leaves the list.
        let (mut g, keys) = galaxy(2);
        g.sectors[0].planner = 1;
        g.sectors[0].values[sector_at(0x18c)] = 0;
        g.sectors[1].planner = 2;
        g.sectors[1].values[sector_at(0x18c)] = 1;
        let mut m = Module8 {
            planners: vec![planner(1, Some(keys[0]), 3), planner(2, Some(keys[1]), 3)],
            ..Module8::default()
        };
        run(&mut m, &mut g, 1, &mut vec![], |m, io| m.balance(io));
        assert_eq!(m.planners, vec![planner(1, Some(keys[0]), 3)]);
        assert_eq!(g.sectors[1].planner, 0);
        m.planners.push(planner(3, None, 0));
        run(&mut m, &mut g, 1, &mut vec![], |m, io| m.balance(io));
        assert_eq!(m.planners, vec![planner(1, Some(keys[0]), 3)]);
    }

    #[test]
    fn the_cycles_take_the_planners_in_turn() {
        // FUN_004cf760: the planner +0x44 names, else the first; +0x44
        // then names the one after it, or none.
        let mut m = Module8 {
            planners: vec![
                planner(1, None, 0),
                planner(2, None, 0),
                planner(4, None, 0),
            ],
            ..Module8::default()
        };
        let mut order = Vec::new();
        for _ in 0..4 {
            assert!(m.choose_planner());
            order.push((m.current, m.next));
        }
        assert_eq!(order, vec![(1, 2), (2, 4), (4, 0), (1, 2)]);
        m.next = 3;
        assert!(m.choose_planner());
        assert_eq!(m.current, 1);
        m.planners.clear();
        assert!(!m.choose_planner());
        assert_eq!((m.current, m.next), (0, 0));
    }

    #[test]
    fn a_planner_keeps_its_sector_or_claims_the_nearest_planned_one() {
        // FUN_004cf7c0: kept while +0x18c == 0 and +0x38 is the planner;
        // else the sector with +0x2c 0x10000 nearest it (FUN_00419b80,
        // order 2), claimed by FUN_004768b0, then bound (FUN_004773f0).
        let (mut g, keys) = galaxy(4);
        g.sectors[0].planner = 1;
        planned(&mut g, 0, 3, 1);
        planned(&mut g, 2, 5, 2);
        planned(&mut g, 3, 1, 3);
        let mut m = Module8 {
            current: 1,
            planners: vec![planner(1, Some(keys[0]), 0)],
            ..Module8::default()
        };
        assert!(run(&mut m, &mut g, 0, &mut vec![], |m, io| m.claim(io)));
        assert_eq!(
            m.planners[0],
            SectorPlanner {
                plan_id: 1,
                ..planner(1, Some(keys[0]), 3)
            }
        );
        // Its plan went: the nearest planned sector without a planner.
        g.sectors[0].values[sector_at(0x18c)] = 1;
        assert!(run(&mut m, &mut g, 0, &mut vec![], |m, io| m.claim(io)));
        assert_eq!(
            m.planners[0],
            SectorPlanner {
                plan_id: 2,
                ..planner(1, Some(keys[2]), 5)
            }
        );
        assert_eq!((g.sectors[0].planner, g.sectors[2].planner), (0, 1));
        // Two planned sectors as near: the tie coin (below 5 ahead) decides.
        for (coin, picked) in [(4, keys[1]), (5, keys[3])] {
            let (mut g, keys) = galaxy(4);
            g.sectors[1].position = (-20, 0);
            g.sectors[3].position = (20, 0);
            g.sectors[0].planner = 1;
            g.sectors[0].values[sector_at(0x18c)] = 1;
            planned(&mut g, 1, 3, 1);
            planned(&mut g, 3, 3, 2);
            let mut m = Module8 {
                current: 1,
                planners: vec![planner(1, Some(keys[0]), 0)],
                ..Module8::default()
            };
            let mut draws = vec![coin];
            assert!(run(&mut m, &mut g, 0, &mut draws, |m, io| m.claim(io)));
            assert_eq!(m.planners[0].sector, Some(picked));
        }
    }

    #[test]
    fn a_planner_without_a_sector_claims_a_planned_one_by_a_random_key() {
        // FUN_004cf7c0 then FUN_0041a1d0: +0x2c 0x8010000 keyed on
        // FUN_0041cd80(0x14), order 2 (the least key), drawn sector by
        // sector with the tie coins between.
        let (mut g, keys) = galaxy(3);
        planned(&mut g, 0, 3, 1);
        planned(&mut g, 1, 3, 2);
        planned(&mut g, 2, 1, 3);
        let mut m = Module8 {
            current: 7,
            planners: vec![planner(7, None, 0)],
            ..Module8::default()
        };
        // Keys 9, 4, 12 give the list 12, 9, 4 with no tie; the pick is
        // the least, 4.
        let mut draws = vec![9, 4, 12];
        assert!(run(&mut m, &mut g, 0, &mut draws, |m, io| m.claim(io)));
        assert_eq!(m.planners[0].sector, Some(keys[1]));
        assert!(draws.is_empty());
        // A tie draws a coin: 3 then 3; the coin (draw 7, not below 5)
        // keeps the second behind, so it is picked.
        let mut m = Module8 {
            current: 8,
            planners: vec![planner(8, None, 0)],
            ..Module8::default()
        };
        g.sectors[1].planner = 0;
        let mut draws = vec![3, 3, 7, 15];
        assert!(run(&mut m, &mut g, 0, &mut draws, |m, io| m.claim(io)));
        assert_eq!(m.planners[0].sector, Some(keys[1]));
        assert!(draws.is_empty());
        // A tie coin below 5 (FUN_0041cd80(10) < 5) puts the later sector
        // ahead, so the earlier one is picked; 5 does not.
        for (coin, picked) in [(4, keys[0]), (5, keys[1])] {
            let mut m = Module8 {
                current: 6,
                planners: vec![planner(6, None, 0)],
                ..Module8::default()
            };
            g.sectors.iter_mut().for_each(|r| r.planner = 0);
            let mut draws = vec![3, 3, coin, 15];
            assert!(run(&mut m, &mut g, 0, &mut draws, |m, io| m.claim(io)));
            assert_eq!(m.planners[0].sector, Some(picked));
        }
        g.sectors.iter_mut().for_each(|r| r.planner = 0);
        // A sector the strategy did not plan (no +0x2c 0x8000000) waits
        // for the second query, whatever its key.
        let mut m = Module8 {
            current: 9,
            planners: vec![planner(9, None, 0)],
            ..Module8::default()
        };
        g.sectors[1].planner = 0;
        g.sectors[0].summary &= !0x800_0000;
        let mut draws = vec![9, 0];
        assert!(run(&mut m, &mut g, 0, &mut draws, |m, io| m.claim(io)));
        assert_eq!(m.planners[0].sector, Some(keys[2]));
    }

    #[test]
    fn with_no_planned_sector_the_cycle_ends() {
        // FUN_004cf630: state 2 with no planner, or state 3 with no sector
        // to claim, ends the cycle (+0x3c = 0, complete).
        let (mut g, _) = galaxy(2);
        let mut m = Module8::default();
        let steps: Vec<Step8> = (0..3)
            .map(|_| run(&mut m, &mut g, 0, &mut vec![], |m, io| m.step(io)))
            .collect();
        assert_eq!(steps, vec![Step8::Pending, Step8::Pending, Step8::Done]);
        m.planners.push(planner(1, None, 0));
        let steps: Vec<Step8> = (0..4)
            .map(|_| run(&mut m, &mut g, 0, &mut vec![], |m, io| m.step(io)))
            .collect();
        assert_eq!(
            steps,
            vec![Step8::Pending, Step8::Pending, Step8::Pending, Step8::Done]
        );
        assert_eq!(m.state, 0);
    }

    #[test]
    fn a_planner_holding_a_plan_reaches_its_plan_step() {
        // FUN_004cf630 states 1 to 3 within the cycle, then state 4
        // (FUN_004cfac0 -> FUN_004774c0) for a planner with a plan object.
        let (mut g, keys) = galaxy(1);
        planned(&mut g, 0, 6, 1);
        let mut m = Module8::default();
        let steps: Vec<Step8> = (0..5)
            .map(|_| run(&mut m, &mut g, -1, &mut vec![], |m, io| m.step(io)))
            .collect();
        assert_eq!(steps[..4], [Step8::Pending; 4]);
        assert_eq!(steps[4], Step8::Plan);
        assert_eq!(
            m.planners,
            vec![SectorPlanner {
                plan_id: 1,
                ..planner(1, Some(keys[0]), 6)
            }]
        );
        assert_eq!(g.sectors[0].planner, 1);
        assert_eq!(m.end(), Step8::Done);
        assert_eq!((m.state, m.decided), (0, false));
        // Another planner's plan object does not count for this one.
        let mut two = Module8 {
            state: 4,
            current: 2,
            planners: vec![planner(1, None, 3), planner(2, None, 0)],
            ..Module8::default()
        };
        let step = run(&mut two, &mut g, 0, &mut vec![], |m, io| m.step(io));
        assert_eq!(step, Step8::Done);
    }

    #[test]
    fn a_planner_whose_sector_lost_its_plan_resets() {
        // FUN_004773f0: no plan (+0x30 0) gives no plan object, so
        // FUN_00477190 drops the kind and the step state and releases the
        // sector (+0x38) and the plan's id.
        let (mut g, keys) = galaxy(1);
        g.sectors[0].planner = 1;
        let mut p = SectorPlanner {
            state: 3,
            plan_id: 1,
            ..planner(1, Some(keys[0]), 3)
        };
        assert!(!bind(&mut p, &mut g));
        assert_eq!(p, planner(1, None, 0));
        assert_eq!(g.sectors[0].planner, 0);
        // A new kind drops the old object and its step state.
        g.sectors[0].plan = 5;
        g.sectors[0].plan_id = 2;
        let mut p = SectorPlanner {
            state: 3,
            ..planner(1, Some(keys[0]), 3)
        };
        assert!(bind(&mut p, &mut g));
        assert_eq!((p.kind, p.state, p.plan_id), (5, 0, 2));
        let mut same = SectorPlanner {
            state: 3,
            ..planner(1, Some(keys[0]), 5)
        };
        assert!(bind(&mut same, &mut g));
        assert_eq!(same.state, 3);
    }

    #[test]
    fn a_sector_another_planner_holds_is_not_taken() {
        // FUN_004768b0 claims only a sector whose +0x38 is 0, and releases
        // the old sector only when it is this planner's.
        let (mut g, keys) = galaxy(2);
        g.sectors[0].planner = 5;
        g.sectors[1].planner = 2;
        let mut p = planner(1, Some(keys[0]), 0);
        take(&mut p, &mut g, keys[1]);
        assert_eq!((p.sector, p.plan_id), (None, 0));
        assert_eq!((g.sectors[0].planner, g.sectors[1].planner), (5, 2));
    }

    #[test]
    fn the_refresh_names_each_planner_s_sector() {
        // FUN_004321e0 keeps +0x38 (FUN_004317d0), so a planner's sector
        // carries its id and, without a plan, +0x18c = 1.
        let (mut g, keys) = galaxy(2);
        mark_planners(&mut g, &[planner(4, Some(keys[1]), 0), planner(5, None, 0)]);
        crate::ai_strategy::mark_plans(&mut g, &[]);
        assert_eq!((g.sectors[0].planner, g.sectors[1].planner), (0, 4));
        assert_eq!(g.sectors[1].values[sector_at(0x18c)], 1);
    }
}
