//! The strategy module of the AI player (id `0xd`, `FUN_004bf0d0`, step
//! `FUN_004bf320`). Each pass its posture (`FUN_004bf490`) picks one of
//! seven strategies (`FUN_004bf7c0`), and the strategy steps through its
//! states (`FUN_004e5440`): it names the sectors the side works on and
//! the kind of plan for each (slot `+0x14`), reviews how the plans go
//! (`FUN_004e5560`), and asks for missions (`FUN_004e5710`). Module 8's
//! sector planners carry the plans out
//! (`ghidra/notes/ai-sector-plans.md`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ai_module1::{Controller, Tallies, ANY};
use crate::ai_module8::{SectorPlanner, CYCLED, REVIEWED};
use crate::ai_records::{pick, picks, Galaxy, Mask, Order};
use crate::ids::{SectorKey, SystemKey};

/// Controller `+4` bit 2: the strategy names its sectors again at its
/// next state 1 (`FUN_004e5530`).
pub const REPLAN: u32 = 2;

/// Controller `+4` bit `0x800000`: the side's fleets are under one and a
/// half times the opponent's, or a module waits for the yards
/// (`FUN_004bf490`). Module 6 then raises the garrison goals.
pub const WEAK: u32 = 0x80_0000;

/// Controller `+4` bit `0x400000`: posture 5, strong with nothing to take
/// (`FUN_004bf490`). Module 6 then lowers the garrison goals.
pub const SECURE: u32 = 0x40_0000;

/// `DAT_0065c49c`: the ratio of the side's fleets' attack to the
/// opponent's below which the side is weak (1.5).
const STRONG_RATIO: f32 = 1.5;

/// One sector the strategy works on (`FUN_004ecac0`, vtable `0x65ca40`):
/// its id (`+0x18`) is its place in the list plus one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectorPlan {
    /// `+0x1c`: the sector.
    pub sector: SectorKey,
    /// `+0x20`: the kind of plan, 1 to 6, which picks the sector planner's
    /// plan object (`FUN_00477630`).
    pub kind: u32,
    /// `+0x24`: kind 4's yard kind (1 shipyards, 2 training facilities, 3
    /// construction yards), else 0.
    pub param: u32,
}

/// Module `0xd`'s posture and strategy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Strategy {
    /// Module `+0x5c`: the posture, 1 to 7; 0 before the first pass.
    pub posture: u32,
    /// Module `+0x60`'s kind (its slot `+0xc`, the posture it was built
    /// for), 0 before the first.
    pub kind: u32,
    /// Strategy `+0x10` bit 1: the last naming found fewer sectors than it
    /// wanted, so the strategy stops growing.
    pub short: bool,
    /// Strategy `+0x14`: name one sector more next time.
    pub grow: bool,
    /// Strategy `+0x18`: name one sector fewer next time.
    pub shrink: bool,
    /// Module `+0x64`: the plans, in id order.
    pub plans: Vec<SectorPlan>,
}

/// What the strategy reads besides the records.
pub struct StrategyInputs<'a> {
    pub galaxy: &'a mut Galaxy,
    pub tallies: Tallies,
    pub controller: &'a mut Controller,
    /// Module `+0x30`, strategy `+4`: the side, 1 the Alliance, 2 the
    /// Empire.
    pub is_alliance: bool,
    /// The day (`FUN_004fcf00`).
    pub day: u64,
    /// System record `+0x44`: the day each system was last seen.
    pub seen: &'a HashMap<SystemKey, u64>,
    /// Controller `+0xd8`: module 8's sector planners.
    pub planners: &'a mut [SectorPlanner],
    pub coin: &'a mut dyn FnMut() -> bool,
}

/// The sector records' fields the plans set, after a refresh
/// (`FUN_004321e0`): each planned sector's `+0x2c` bit `0x8000000`, its
/// kind, parameter and id, then the bits they give.
pub fn mark_plans(galaxy: &mut Galaxy, plans: &[SectorPlan]) {
    for (i, plan) in plans.iter().enumerate() {
        set_plan(galaxy, plan, i);
    }
    for record in &mut galaxy.sectors {
        record.plan_bits();
    }
}

/// `FUN_00431790` and the list id: the plan's fields into its sector's
/// record, with `+0x2c` bit `0x8000000`.
fn set_plan(galaxy: &mut Galaxy, plan: &SectorPlan, at: usize) {
    if let Some(record) = galaxy.sectors.iter_mut().find(|s| s.sector == plan.sector) {
        record.summary = (record.summary | 0x800_0000) & !0x2000_0000;
        record.plan = plan.kind;
        record.plan_param = plan.param;
        record.plan_id = u32::try_from(at + 1).unwrap_or(u32::MAX);
    }
}

/// `FUN_004317b0` and the list id cleared, with `+0x2c` bit `0x8000000`.
fn clear_plan(galaxy: &mut Galaxy, sector: SectorKey) {
    if let Some(record) = galaxy.sectors.iter_mut().find(|s| s.sector == sector) {
        record.summary &= !0x2800_0000;
        record.plan = 0;
        record.plan_param = 0;
        record.plan_id = 0;
    }
}

impl Strategy {
    /// One pass of module `0xd` before its missions: the posture
    /// (`FUN_004bf390` state 1, `FUN_004bf490`), then the strategy's
    /// states 1 and 2 (`FUN_004e5530`, `FUN_004e5560`). State 3 is the
    /// research strategy ([`crate::ai::AISystem`]).
    ///
    /// port: states 4 and 5 (`FUN_004e57e0`, the requests of module `0xc`;
    /// `FUN_004e69d0`, its planners) and 6 (`FUN_004e5870`, the
    /// headquarters' `+0x30` bit `0x10000000`) are not ported; neither
    /// are the Alliance's states after the strategy (`FUN_004bf3e0`).
    pub fn step(&mut self, io: &mut StrategyInputs<'_>) {
        self.choose_posture(&io.tallies, io.controller, io.planners);
        if io.controller.flags & REPLAN != 0 {
            self.short = false;
            self.name_sectors(io);
            io.controller.flags &= !REPLAN;
        }
        self.review(io);
    }

    /// `FUN_004bf490`: keep the posture while its condition holds, else
    /// choose again, and build the posture's strategy when it has none of
    /// that kind.
    fn choose_posture(
        &mut self,
        t: &Tallies,
        controller: &mut Controller,
        planners: &mut [SectorPlanner],
    ) {
        let mut ratio = t.strength as f32;
        if t.enemy_strength > 0 {
            ratio /= t.enemy_strength as f32;
        }
        let weak = ratio < STRONG_RATIO;
        let spare = t.fleet_regiments - t.garrison_goal + t.planet_regiments;
        let takes = spare <= t.weakest_enemy;
        let busy = controller.yard_requests != 0;
        let n = t.own_systems;
        let poor = |energy_floor: i32, raw_floor: i32| {
            t.free_energy < energy_floor || t.free_raw < raw_floor
        };
        let rich = !poor(n / 2 + 1, n / 4 + 1);
        let keep = match self.posture {
            2 => poor(n + 1, n / 2 + 1) && (weak || takes) && !busy && t.open_sectors != 0,
            3 => weak && !busy && spare > 0 && rich,
            4 => busy,
            5 => !weak && !busy && takes && rich,
            6 => !weak && !busy && spare > 0,
            7 => {
                poor(n + 1, n / 2 + 1)
                    && spare > 0
                    && (weak || takes)
                    && !busy
                    && t.open_sectors != 0
            }
            _ => false,
        };
        if !keep {
            controller.flags &= !(WEAK | SECURE);
            let developing = poor(n / 2 + 1, n / 4 + 1) && t.open_sectors > 0;
            self.posture = if weak || busy {
                controller.flags |= WEAK;
                if busy {
                    4
                } else if developing {
                    if takes {
                        2
                    } else {
                        7
                    }
                } else if takes {
                    1
                } else {
                    3
                }
            } else if t.weakest_enemy < spare {
                6
            } else if developing {
                2
            } else {
                controller.flags |= SECURE;
                5
            };
        }
        // A strategy of another kind goes (FUN_004e54f0 clears the
        // planners' 0x8000000); the plans stay. A new one replans
        // (controller +4 bit 2).
        if self.kind != self.posture {
            if self.kind != 0 {
                for p in planners.iter_mut() {
                    p.flags &= !REVIEWED;
                }
            }
            self.kind = self.posture;
            self.short = false;
            self.grow = false;
            self.shrink = false;
            controller.flags |= REPLAN;
        }
    }

    /// Slot `+0x14` of the posture's strategy (`FUN_004e62b0`,
    /// `FUN_004e7990`, `FUN_004e8940`, `FUN_004e9690`, `FUN_004ea060`,
    /// `FUN_004ead50`, `FUN_004e6c00`): as many sectors as the plans had,
    /// one more or fewer as state 2 asked, the posture's first choice then
    /// the shared ones. A new list replaces the old; an empty one keeps it.
    fn name_sectors(&mut self, io: &mut StrategyInputs<'_>) {
        let mut wanted = i32::try_from(self.plans.len()).unwrap_or(i32::MAX);
        if self.grow {
            wanted += 1;
        } else if self.shrink {
            wanted -= 1;
        }
        let mut plans: Vec<SectorPlan> = Vec::new();
        if wanted > 0 {
            match self.posture {
                1 => {}
                4 => {
                    let (param, sector) = self.yard_target(io);
                    if let Some(sector) = sector {
                        plans.push(SectorPlan {
                            sector,
                            kind: 4,
                            param,
                        });
                    }
                }
                _ => {
                    if let Some((sector, kind)) = self.first_choice(io) {
                        plans.push(SectorPlan {
                            sector,
                            kind,
                            param: 0,
                        });
                    }
                }
            }
            if self.posture == 7 && plans.len() < count(wanted) {
                self.reach(io, &mut plans);
            }
            self.shared(io, &mut plans, wanted);
        }
        if plans.is_empty() {
            return;
        }
        for old in &self.plans {
            clear_plan(io.galaxy, old.sector);
        }
        for (i, plan) in plans.iter().enumerate() {
            set_plan(io.galaxy, plan, i);
        }
        self.plans = plans;
    }

    /// The sector of the first plan before this naming.
    fn first_sector(&self) -> Option<SectorKey> {
        self.plans.first().map(|p| p.sector)
    }

    /// The lists one choice merges: the query, the sectors nearest the
    /// first plan's (`FUN_00419b80`, order 2) when `near` is given, and
    /// twice the sectors by plan id (`FUN_00419e90`, order 2) when there
    /// were plans.
    fn lists(
        &self,
        io: &mut StrategyInputs<'_>,
        query: Vec<SectorKey>,
        near: Option<[Mask; 3]>,
        planned: Option<[Mask; 3]>,
    ) -> Vec<Vec<SectorKey>> {
        let mut lists = vec![query];
        if let (Some(masks), Some(from)) = (near, self.first_sector()) {
            lists.push(
                io.galaxy
                    .sectors_near(from, masks, Order::Descending, io.coin),
            );
        }
        if let Some(masks) = planned.filter(|_| !self.plans.is_empty()) {
            for _ in 0..2 {
                lists.push(io.galaxy.planned_sectors(masks, Order::Descending, io.coin));
            }
        }
        lists
    }

    /// `FUN_00419a00` merged as [`Self::lists`], near and by plan id on
    /// the same masks.
    fn choice(
        &self,
        io: &mut StrategyInputs<'_>,
        masks: [Mask; 3],
        key: usize,
        order: Order,
        near: bool,
    ) -> Vec<Vec<SectorKey>> {
        let query = io.galaxy.sector_query(masks, key, order, io.coin);
        self.lists(io, query, near.then_some(masks), Some(masks))
    }

    /// The posture's first plan, one pick ([`pick`] takes a list's last
    /// entry: the least key for order 2, the greatest for order 1):
    /// - 7 and 2 (`FUN_004e6c00`, `FUN_004e7990`): kind 5 on a sector
    ///   without `+0x2c` 2, the fewest opponent systems (`+0x80`);
    /// - 3 (`FUN_004e8940`): kind 6 on a sector with `0x408` without
    ///   `0x40000`, the least `+0xa0`; else any `0x408` by the least
    ///   `+0x190`; else kind 5 on a sector without 2, by the most own
    ///   systems (`+0x7c`);
    /// - 5 (`FUN_004ea060`): kind 2 on a sector with `0x40000` by the most
    ///   `+0x190`, else on one with 8 without `0x20000000` whose opponent
    ///   systems were seen the longest ago ([`stale_sectors`]);
    /// - 6 (`FUN_004ead50`): kind 6 on the Empire's sector with its
    ///   headquarters and `0x408` (controller `+4` bit `0x400`), by the
    ///   most `+0x19c`; else a `0x408` sector by the most `+0x19c`, then
    ///   by the most `+0x130`; else kind 1 on a sector with `0x80100` by
    ///   the most opponent systems; else kind 5 as posture 3's.
    ///
    /// Each merges the sectors nearest the first plan's and the planned
    /// ones as [`Self::lists`] says, except where noted in the code.
    fn first_choice(&self, io: &mut StrategyInputs<'_>) -> Option<(SectorKey, u32)> {
        use Order::{Ascending, Descending};
        let opponent = [ANY, ANY, Mask::new(0, 2)];
        let contested = [ANY, ANY, Mask::new(0x408, 0x4_0000)];
        let held = [ANY, ANY, Mask::new(0x408, 0)];
        let first = |me: &Self, io: &mut StrategyInputs<'_>, masks, key, order, near| {
            let lists = me.choice(io, masks, key, order, near);
            pick(&lists)
        };
        match self.posture {
            2 | 7 => first(self, io, opponent, 4, Descending, true).map(|s| (s, 5)),
            3 => {
                let six = first(self, io, contested, 0xc, Descending, true).or_else(|| {
                    let query = io.galaxy.sector_query(held, 0x48, Descending, io.coin);
                    pick(&self.lists(io, query, None, Some(held)))
                });
                six.map(|s| (s, 6))
                    .or_else(|| first(self, io, opponent, 3, Ascending, true).map(|s| (s, 5)))
            }
            5 => {
                let open = [ANY, ANY, Mask::new(0x4_0000, 0)];
                first(self, io, open, 0x48, Ascending, true)
                    .or_else(|| {
                        let masks = [ANY, ANY, Mask::new(8, 0x2000_0000)];
                        let query = stale_sectors(io, masks, Ascending);
                        pick(&self.lists(io, query, Some(masks), Some(masks)))
                    })
                    .map(|s| (s, 2))
            }
            6 => {
                let mut found = None;
                // Controller +4 bit 0x400 is set when a sector has +0x2c
                // 0x80000000 (FUN_0041b7e0), which this query asks for
                // too, so the query alone gives the same pick.
                if !io.is_alliance {
                    let masks = [ANY, ANY, Mask::new(0x8000_0408, 0)];
                    let query = io.galaxy.sector_query(masks, 0x4b, Ascending, io.coin);
                    found = pick(&[query]);
                }
                let found = found
                    .or_else(|| first(self, io, held, 0x4b, Ascending, true))
                    .or_else(|| {
                        let query = io.galaxy.sector_query(held, 0x30, Ascending, io.coin);
                        let near = [ANY, ANY, Mask::new(8, 0)];
                        let mut lists = self.lists(io, query, Some(near), None);
                        if !self.plans.is_empty() {
                            for _ in 0..2 {
                                lists.push(io.galaxy.planned_sectors(held, Descending, io.coin));
                            }
                        }
                        pick(&lists)
                    });
                found.map(|s| (s, 6)).or_else(|| {
                    let attack = [ANY, ANY, Mask::new(0x8_0100, 0)];
                    first(self, io, attack, 4, Ascending, true)
                        .map(|s| (s, 1))
                        .or_else(|| first(self, io, opponent, 3, Ascending, true).map(|s| (s, 5)))
                })
            }
            _ => None,
        }
    }

    /// Posture 7's second plan (`FUN_004e6c00`): kind 6 on the first sector
    /// not yet named with `+0x2c` `0x408` without `0x40000`, by the least
    /// `+0xa0`; with none, any `0x408` sector by the least `+0x190`.
    fn reach(&self, io: &mut StrategyInputs<'_>, plans: &mut Vec<SectorPlan>) {
        let contested = [ANY, ANY, Mask::new(0x408, 0x4_0000)];
        let held = [ANY, ANY, Mask::new(0x408, 0)];
        let mut lists = self.choice(io, contested, 0xc, Order::Descending, false);
        if pick(&lists).is_none() {
            lists = self.choice(io, held, 0x48, Order::Descending, false);
        }
        if let Some(sector) = picks(lists).into_iter().find(|&s| !named(plans, s)) {
            plans.push(SectorPlan {
                sector,
                kind: 6,
                param: 0,
            });
        }
    }

    /// The choices every strategy ends with (`FUN_004e62b0` and the
    /// others' tails), each taking sectors not yet
    /// named until it has `wanted`: kind 3 on sectors with `+0x24`
    /// `0x40000000` and `+0x2c` 4, then `+0x2c` `0x16` without 8, then
    /// `+0x2c` 6, each by the most `+0x108`; then kind 1 on sectors with
    /// `0x80000` (without 4 in postures 1, 2 and 7), by the most opponent
    /// systems (`+0x80`). Short of `wanted`, the strategy stops growing
    /// (`+0x10` bit 1).
    fn shared(&mut self, io: &mut StrategyInputs<'_>, plans: &mut Vec<SectorPlan>, wanted: i32) {
        let attack_forbids = if matches!(self.posture, 1 | 2 | 7) {
            4
        } else {
            0
        };
        let steps = [
            ([Mask::new(0x4000_0000, 0), ANY, Mask::new(4, 0)], 0x26, 3),
            ([ANY, ANY, Mask::new(0x16, 8)], 0x26, 3),
            ([ANY, ANY, Mask::new(6, 0)], 0x26, 3),
            ([ANY, ANY, Mask::new(0x8_0000, attack_forbids)], 4, 1),
        ];
        for (masks, key, kind) in steps {
            if plans.len() >= count(wanted) {
                return;
            }
            let lists = self.choice(io, masks, key, Order::Ascending, false);
            for sector in picks(lists) {
                if plans.len() >= count(wanted) {
                    break;
                }
                if !named(plans, sector) {
                    plans.push(SectorPlan {
                        sector,
                        kind,
                        param: 0,
                    });
                }
            }
        }
        if plans.len() < count(wanted) {
            self.short = true;
        }
    }

    /// `FUN_004e5b10`, posture 4's first plan: for the highest module bit
    /// waiting for the yards (controller `+8`), a sector with `+0x2c` 2
    /// (by the least `+0xa0`) whose systems not the side's hold yards of
    /// the kind that module builds with (`+0x11c`, `+0x120`, `+0x124`),
    /// and a system there with them (`+0x30` `0x20` without `0x101`, by
    /// that count and by `v[20]`, order 2). The kind: 1 shipyards (bits 2, 8, `0x10`), 2
    /// training facilities (4, `0x20`), 3 construction yards (1, `0x40`,
    /// `0x80`); it is the last bit's even when no sector has them.
    fn yard_target(&self, io: &mut StrategyInputs<'_>) -> (u32, Option<SectorKey>) {
        let requests = io.controller.yard_requests;
        let mut param = 0;
        let mut found = None;
        let mut bit = 0x80;
        while requests != 0 && found.is_none() && bit != 0 {
            let asked = bit & requests;
            bit >>= 1;
            if asked == 0 {
                continue;
            }
            let (sector_key, system_key);
            (sector_key, system_key, param) = match asked {
                1 | 0x40 | 0x80 => (0x2d, 6, 3),
                2 | 8 | 0x10 => (0x2b, 4, 1),
                _ => (0x2c, 5, 2),
            };
            let held = [ANY, ANY, Mask::new(2, 0)];
            for sector in picks(self.choice(io, held, 0xc, Order::Descending, true)) {
                let holds = io
                    .galaxy
                    .sector(sector)
                    .is_some_and(|r| r.values[sector_key] > 0);
                if holds && self.has_yard_system(io, sector, system_key) {
                    found = Some(sector);
                    break;
                }
            }
        }
        (param, found)
    }

    /// The system half of [`Self::yard_target`] (`FUN_004e5b10`'s
    /// `FUN_0041a340` queries).
    fn has_yard_system(&self, io: &mut StrategyInputs<'_>, sector: SectorKey, key: usize) -> bool {
        let masks = [ANY, ANY, Mask::new(0x20, 0x101)];
        let lists = vec![
            io.galaxy
                .system_query(sector, masks, key, Order::Descending, io.coin),
            io.galaxy
                .system_query(sector, masks, 0x14, Order::Descending, io.coin),
        ];
        picks(lists)
            .into_iter()
            .any(|s| io.galaxy.system(s).is_some_and(|r| r.values[key] >= 1))
    }

    /// `FUN_004e5560`, state 2: each plan's planner (its sector's `+0x38`)
    /// takes the plan's id (`FUN_004769f0`). Once every plan's planner has
    /// `+0x60` bits `0x8000000` and `0x4000000`: a planner without
    /// `0x20000` asks for a replan, each loses `0x8000000`; more than one
    /// that gave up (`0x10000`, with the last one's) shrinks the list, else
    /// with all held (`0x80000`) and the last naming not short it grows;
    /// either names the sectors again (controller `+4` bit 2).
    fn review(&mut self, io: &mut StrategyInputs<'_>) {
        self.grow = false;
        self.shrink = false;
        let mut done = true;
        let mut owners = Vec::new();
        for (i, plan) in self.plans.iter().enumerate() {
            let Some(record) = io.galaxy.sector(plan.sector) else {
                continue;
            };
            let owner = io.planners.iter().position(|p| p.id == record.planner);
            if let Some(at) = owner {
                let p = &mut io.planners[at];
                p.plan_id = u32::try_from(i + 1).unwrap_or(u32::MAX);
                owners.push(at);
                if p.flags & REVIEWED != 0 && p.flags & CYCLED != 0 {
                    continue;
                }
            }
            done = false;
        }
        if !done {
            return;
        }
        let (mut unheld, mut gave_up, mut last, mut count) = (false, false, false, 0);
        for at in owners {
            let p = &mut io.planners[at];
            if p.flags & 0x8_0000 == 0 {
                unheld = true;
            }
            last = p.flags & 0x1_0000 != 0;
            if last {
                gave_up = true;
                count += 1;
            }
            if p.flags & 0x2_0000 == 0 {
                io.controller.flags |= REPLAN;
            }
            p.flags &= !REVIEWED;
        }
        if gave_up {
            if last && count > 1 {
                io.controller.flags |= REPLAN;
                self.shrink = true;
            }
        } else if !unheld && !self.short {
            io.controller.flags |= REPLAN;
            self.grow = true;
        }
    }
}

/// A plan count as a length.
fn count(wanted: i32) -> usize {
    usize::try_from(wanted).unwrap_or(0)
}

/// `FUN_004e5260`: whether the new list names `sector`.
fn named(plans: &[SectorPlan], sector: SectorKey) -> bool {
    plans.iter().any(|p| p.sector == sector)
}

/// `FUN_0041a060`: the sectors passing `masks`, keyed on how long ago
/// their opponent systems were seen (`FUN_004317e0`).
fn stale_sectors(io: &mut StrategyInputs<'_>, masks: [Mask; 3], order: Order) -> Vec<SectorKey> {
    let items: Vec<(SectorKey, i32)> = io
        .galaxy
        .sectors
        .iter()
        .filter(|s| s.admitted(masks))
        .map(|s| (s.sector, staleness(s, io.day, io.seen)))
        .collect();
    crate::ai_records::ranked(items, order, io.coin)
}

/// `FUN_004317e0`: the days since each system held by the opponent (record
/// `+0x30` `0x20` without 1) was last seen (`FUN_00433de0`: the day less
/// `+0x44`), summed and divided by the sector's systems not its side's
/// (`+0x84` + `+0x80`).
fn staleness(
    record: &crate::ai_records::SectorRecord,
    day: u64,
    seen: &HashMap<SystemKey, u64>,
) -> i32 {
    let total: i64 = record
        .systems
        .iter()
        .filter(|r| r.kind & 1 == 0 && r.kind & 0x20 != 0)
        .map(|r| {
            let last = seen.get(&r.system).copied().unwrap_or(0);
            i64::try_from(day).unwrap_or(i64::MAX) - i64::try_from(last).unwrap_or(0)
        })
        .sum();
    let total = i32::try_from(total).unwrap_or(i32::MAX);
    let others = record.values[crate::ai_records::sector_at(0x84)]
        + record.values[crate::ai_records::sector_at(0x80)];
    if total > 0 && others != 0 {
        total / others
    } else {
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_records::{sector_at, SectorRecord, SystemRecord};

    fn record(system: SystemKey, kind: u32) -> SystemRecord {
        SystemRecord {
            system,
            state: 0,
            summary: 0,
            kind,
            values: [0; 60],
            position: (0, 0),
        }
    }

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

    struct Run {
        galaxy: Galaxy,
        controller: Controller,
        tallies: Tallies,
        seen: HashMap<SystemKey, u64>,
        day: u64,
        is_alliance: bool,
        planners: Vec<SectorPlanner>,
    }

    impl Run {
        fn new(galaxy: Galaxy) -> Self {
            Self {
                galaxy,
                controller: Controller::default(),
                tallies: Tallies::default(),
                seen: HashMap::new(),
                day: 0,
                is_alliance: false,
                planners: Vec::new(),
            }
        }

        fn with<R>(&mut self, f: impl FnOnce(&mut StrategyInputs<'_>) -> R) -> R {
            let mut coin = || false;
            let mut io = StrategyInputs {
                galaxy: &mut self.galaxy,
                tallies: self.tallies,
                controller: &mut self.controller,
                is_alliance: self.is_alliance,
                day: self.day,
                seen: &self.seen,
                planners: &mut self.planners,
                coin: &mut coin,
            };
            f(&mut io)
        }
    }

    fn posture_of(t: Tallies, yard_requests: u32, before: u32) -> (u32, u32) {
        let mut strategy = Strategy {
            posture: before,
            ..Strategy::default()
        };
        let mut controller = Controller {
            yard_requests,
            ..Controller::default()
        };
        strategy.choose_posture(&t, &mut controller, &mut []);
        (strategy.posture, controller.flags & (WEAK | SECURE))
    }

    #[test]
    fn the_posture_follows_strength_spare_troops_and_the_economy() {
        // FUN_004bf490's choice with no posture to keep: weak below 1.5
        // times the opponent's attack (DAT_0065c49c) or with a module
        // waiting for the yards (controller +8); spare regiments
        // (+0x25c - +0x1fc + +0x200) against the weakest opponent sector
        // (+0x210); poor below half the systems' energy plus one or a
        // quarter's raw materials plus one, with an open sector (+0x1c8).
        let base = Tallies {
            own_systems: 4,
            free_energy: 3,
            free_raw: 2,
            open_sectors: 1,
            ..Tallies::default()
        };
        let weak = Tallies {
            strength: 2,
            enemy_strength: 2,
            ..base
        };
        let strong = Tallies {
            strength: 3,
            enemy_strength: 2,
            ..base
        };
        let poor = |t: Tallies| Tallies {
            free_energy: 2,
            ..t
        };
        let spare = |t: Tallies| Tallies {
            fleet_regiments: 5,
            weakest_enemy: 4,
            ..t
        };
        assert_eq!(posture_of(weak, 0, 0), (1, WEAK));
        assert_eq!(posture_of(poor(weak), 0, 0), (2, WEAK));
        assert_eq!(posture_of(spare(weak), 0, 0), (3, WEAK));
        assert_eq!(posture_of(strong, 4, 0), (4, WEAK));
        assert_eq!(posture_of(strong, 0, 0), (5, SECURE));
        assert_eq!(posture_of(spare(strong), 0, 0), (6, 0));
        assert_eq!(posture_of(spare(poor(weak)), 0, 0), (7, WEAK));
        assert_eq!(posture_of(poor(strong), 0, 0), (2, 0));
        // Without an open sector the economy does not count as poor.
        let closed = Tallies {
            open_sectors: 0,
            ..poor(weak)
        };
        assert_eq!(posture_of(closed, 0, 0), (1, WEAK));
        // No opponent fleets: the ratio is the side's own attack.
        let alone = Tallies {
            strength: 1,
            enemy_strength: 0,
            ..base
        };
        assert_eq!(posture_of(alone, 0, 0), (1, WEAK));
    }

    /// `FUN_004bf490` transcribed from the decompile, its locals named as
    /// there: `fVar6` the ratio, `iVar8` the spare regiments, `iVar1`
    /// `+0x210`, `iVar2` `+0x1ec`, `iVar3` `+0x1e8`, `iVar4` the switch's
    /// operand; `+8` the yard requests. Returns `(+0x5c, +4 bits)`.
    #[expect(
        clippy::int_plus_one,
        reason = "The decompile's comparisons, kept as written."
    )]
    fn decompiled_posture(t: &Tallies, yards: u32, posture: u32, flags: u32) -> (u32, u32) {
        let mut f_var6 = t.strength as f32;
        let i_var8 = t.fleet_regiments - t.garrison_goal + t.planet_regiments;
        if 0 < t.enemy_strength {
            f_var6 /= t.enemy_strength as f32;
        }
        let (i_var1, i_var2, i_var3) = (t.weakest_enemy, t.free_raw, t.free_energy);
        let s1ac = t.own_systems;
        let s1c8 = t.open_sectors;
        let war = yards != 0;
        let lab_57d = |n: i32| n / 2 + 1 <= i_var3 && ((n + (n >> 31 & 3)) >> 2) + 1 <= i_var2;
        let kept = match posture {
            2 => {
                (i_var3 < s1ac + 1 || i_var2 < s1ac / 2 + 1)
                    && (f_var6 < 1.5 || i_var8 <= i_var1)
                    && !war
                    && s1c8 != 0
            }
            3 => f_var6 < 1.5 && !war && 0 < i_var8 && lab_57d(s1ac),
            4 => war,
            5 => 1.5 <= f_var6 && !war && i_var8 <= i_var1 && lab_57d(s1ac),
            6 => 1.5 <= f_var6 && !war && 0 < i_var8,
            7 => {
                (i_var3 < s1ac + 1 || i_var2 < s1ac / 2 + 1)
                    && 0 < i_var8
                    && (f_var6 < 1.5 || i_var8 <= i_var1)
                    && !war
                    && s1c8 != 0
            }
            _ => false,
        };
        if kept {
            return (posture, flags);
        }
        let mut flags = flags & 0xff3f_ffff;
        let n = s1ac;
        let short = (i_var3 < n / 2 + 1 || i_var2 < ((n + (n >> 31 & 3)) >> 2) + 1) && 0 < s1c8;
        let posture = if f_var6 < 1.5 || war {
            flags |= 0x80_0000;
            if war {
                4
            } else if short {
                (u32::from(i_var8 <= i_var1).wrapping_sub(1) & 5) + 2
            } else {
                (u32::from(i_var8 <= i_var1).wrapping_sub(1) & 2) + 1
            }
        } else if i_var1 < i_var8 {
            6
        } else if short {
            2
        } else {
            flags |= 0x40_0000;
            5
        };
        (posture, flags)
    }

    #[test]
    fn the_posture_matches_the_decompiled_rule_at_every_boundary() {
        // FUN_004bf490 over a grid around each threshold: the ratio at,
        // below and above 1.5; the spare regiments around 0 and around
        // +0x210; the energy and raw materials around n/2+1, n/4+1, n+1;
        // no open sector or one; a yard request or none; each posture to
        // keep. The controller starts with both bits set, so a kept
        // posture keeps them.
        let mut cases = 0;
        for (strength, enemy) in [(2, 2), (3, 2), (4, 2), (5, 0), (1, 0)] {
            for spare in -1..=4 {
                for weakest in 0..=3 {
                    for n in [4, 5, 7] {
                        for energy in [n / 2, n / 2 + 1, n, n + 1] {
                            for raw in [n / 4, n / 4 + 1, n / 2, n / 2 + 1] {
                                for open in [0, 1] {
                                    for yards in [0, 2] {
                                        for before in 0..=7 {
                                            let t = Tallies {
                                                strength,
                                                enemy_strength: enemy,
                                                fleet_regiments: spare + 3 - 2,
                                                garrison_goal: 3,
                                                planet_regiments: 2,
                                                weakest_enemy: weakest,
                                                own_systems: n,
                                                free_energy: energy,
                                                free_raw: raw,
                                                open_sectors: open,
                                                ..Tallies::default()
                                            };
                                            let flags = WEAK | SECURE | 1;
                                            let mut strategy = Strategy {
                                                posture: before,
                                                ..Strategy::default()
                                            };
                                            let mut controller = Controller {
                                                yard_requests: yards,
                                                flags,
                                                ..Controller::default()
                                            };
                                            strategy.choose_posture(&t, &mut controller, &mut []);
                                            let want = decompiled_posture(&t, yards, before, flags);
                                            assert_eq!(
                                                (strategy.posture, controller.flags & !REPLAN),
                                                want,
                                                "{t:?} yards {yards} before {before}"
                                            );
                                            cases += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(cases > 100_000);
    }

    #[test]
    fn without_the_replan_bit_the_sectors_stay_named() {
        // FUN_004e5530 names the sectors only with controller +4 bit 2.
        let (mut g, keys) = galaxy(2);
        g.sectors[1].summary = 6;
        let mut run = Run::new(g);
        let old = SectorPlan {
            sector: keys[0],
            kind: 1,
            param: 0,
        };
        let mut strategy = Strategy {
            posture: 1,
            kind: 1,
            plans: vec![old],
            ..Strategy::default()
        };
        run.controller.flags = 0;
        run.with(|io| strategy.step(io));
        assert_eq!(strategy.plans, vec![old]);
        run.controller.flags = REPLAN;
        run.with(|io| strategy.step(io));
        assert_eq!(strategy.plans[0].sector, keys[1]);
    }

    #[test]
    fn a_shrinking_strategy_names_one_sector_fewer() {
        // Slot +0x14 takes one from the count with +0x18.
        let (mut g, keys) = galaxy(3);
        for i in 0..3 {
            g.sectors[i].summary = 6;
        }
        let mut run = Run::new(g);
        let mut strategy = Strategy {
            posture: 1,
            kind: 1,
            shrink: true,
            plans: keys
                .iter()
                .map(|&sector| SectorPlan {
                    sector,
                    kind: 3,
                    param: 0,
                })
                .collect(),
            ..Strategy::default()
        };
        run.with(|io| strategy.name_sectors(io));
        assert_eq!(strategy.plans.len(), 2);
    }

    #[test]
    fn posture_four_names_its_yard_sector_first() {
        // FUN_004e9690: kind 4 with FUN_004e5b10's parameter, then the
        // shared tail.
        let mut keys = slotmap::SlotMap::<SystemKey, ()>::with_key();
        let mut rec = record(keys.insert(()), 0x24);
        rec.values[4] = 1;
        let (mut g, sectors) = galaxy(2);
        g.sectors[1] = SectorRecord::build(sectors[1], (10, 0), vec![rec]);
        g.sectors[1].summary = 2;
        g.sectors[1].values[sector_at(0x11c)] = 1;
        g.sectors[0].summary = 6;
        let mut run = Run::new(g);
        run.controller.yard_requests = 2;
        let mut four = Strategy {
            posture: 4,
            kind: 4,
            grow: true,
            plans: vec![SectorPlan {
                sector: sectors[0],
                kind: 3,
                param: 0,
            }],
            ..Strategy::default()
        };
        run.with(|io| four.name_sectors(io));
        let named: Vec<(SectorKey, u32, u32)> = four
            .plans
            .iter()
            .map(|p| (p.sector, p.kind, p.param))
            .collect();
        assert_eq!(named, vec![(sectors[1], 4, 1), (sectors[0], 3, 0)]);
        // A sector without the yards (+0x11c 0) is passed over.
        run.galaxy.sectors[1].values[sector_at(0x11c)] = 0;
        assert_eq!(run.with(|io| four.yard_target(io)), (1, None));
    }

    #[test]
    fn posture_seven_s_second_sector_needs_room_and_posture_seven() {
        // FUN_004e6c00 adds kind 6 only while short of the count; the
        // other classes have no such step (FUN_004e7990).
        let (mut g, keys) = galaxy(3);
        g.sectors[0].summary = 0x408;
        let mut run = Run::new(g);
        let one = |posture| Strategy {
            posture,
            kind: posture,
            plans: vec![SectorPlan {
                sector: keys[2],
                kind: 3,
                param: 0,
            }],
            ..Strategy::default()
        };
        let mut seven = one(7);
        run.with(|io| seven.name_sectors(io));
        assert_eq!(
            seven.plans.iter().map(|p| p.kind).collect::<Vec<_>>(),
            vec![5]
        );
        let mut two = Strategy {
            grow: true,
            ..one(2)
        };
        run.with(|io| two.name_sectors(io));
        assert!(two.plans.iter().all(|p| p.kind != 6));
    }

    #[test]
    fn the_first_choice_merges_the_sectors_nearest_the_first_plan() {
        // FUN_004e7990: the query (+0x80, order 2), FUN_00419b80 from the
        // first old plan's sector (order 2) and twice FUN_00419e90: the
        // summed places pick the middle sector, which the query and the
        // planned lists alone would not.
        let (mut g, keys) = galaxy(4);
        g.sectors[3].summary = 2;
        g.sectors[3].position = (0, 0);
        for (i, x) in [10, 20, 40].into_iter().enumerate() {
            g.sectors[i].position = (x, 0);
        }
        g.sectors[2].values[sector_at(0x80)] = 1;
        let mut run = Run::new(g);
        let two = Strategy {
            posture: 2,
            plans: vec![SectorPlan {
                sector: keys[3],
                kind: 3,
                param: 0,
            }],
            ..Strategy::default()
        };
        mark_plans(&mut run.galaxy, &two.plans);
        assert_eq!(run.with(|io| two.first_choice(io)), Some((keys[1], 5)));
        let masks = [ANY, ANY, Mask::new(0, 2)];
        let without_near = run.with(|io| {
            let query = io.galaxy.sector_query(masks, 4, Order::Descending, io.coin);
            two.lists(io, query, None, Some(masks))
        });
        assert_eq!(pick(&without_near), Some(keys[2]));
    }

    #[test]
    fn posture_five_falls_back_on_the_sector_seen_longest_ago() {
        // FUN_004ea060: with no +0x2c 0x40000 sector, FUN_0041a060 over
        // +0x2c 8 without 0x20000000 keyed on FUN_004317e0, order 1.
        let mut keys = slotmap::SlotMap::<SystemKey, ()>::with_key();
        let a = record(keys.insert(()), 0x24);
        let b = record(keys.insert(()), 0x24);
        let (_, sectors) = galaxy(2);
        let mut g = Galaxy {
            sectors: vec![
                SectorRecord::build(sectors[0], (0, 0), vec![a]),
                SectorRecord::build(sectors[1], (10, 0), vec![b]),
            ],
            ..Galaxy::default()
        };
        g.sectors[0].summary = 8;
        g.sectors[1].summary = 8;
        let mut run = Run::new(g);
        run.day = 50;
        run.seen.insert(a.system, 40);
        run.seen.insert(b.system, 5);
        let five = Strategy {
            posture: 5,
            ..Strategy::default()
        };
        assert_eq!(run.with(|io| five.first_choice(io)), Some((sectors[1], 2)));
        run.galaxy.sectors[1].summary = 0x2000_0008;
        assert_eq!(run.with(|io| five.first_choice(io)), Some((sectors[0], 2)));
    }

    /// Two plans, each sector held by a planner with `flags`.
    fn reviewed(flags: [u32; 2], short: bool) -> (Strategy, Run) {
        let (mut g, keys) = galaxy(2);
        g.sectors[0].planner = 1;
        g.sectors[1].planner = 2;
        let mut run = Run::new(g);
        run.planners = (0..2)
            .map(|i| SectorPlanner {
                id: u32::try_from(i).unwrap() + 1,
                sector: Some(keys[i]),
                flags: flags[i],
                ..SectorPlanner::default()
            })
            .collect();
        run.controller.flags = 0;
        let mut strategy = Strategy {
            posture: 1,
            kind: 1,
            short,
            plans: keys
                .iter()
                .map(|&sector| SectorPlan {
                    sector,
                    kind: 3,
                    param: 0,
                })
                .collect(),
            ..Strategy::default()
        };
        run.with(|io| strategy.review(io));
        (strategy, run)
    }

    #[test]
    fn the_review_waits_for_every_plan_s_planner_to_cycle() {
        // FUN_004e5560: each plan's planner takes its id (FUN_004769f0);
        // a planner without both +0x60 0x8000000 and 0x4000000 holds the
        // review back.
        let done = REVIEWED | CYCLED | 0xa_0000;
        let (s, run) = reviewed([done, CYCLED | 0xa_0000], false);
        assert!(!s.grow && !s.shrink);
        assert_eq!(run.controller.flags, 0);
        assert_eq!(
            run.planners.iter().map(|p| p.plan_id).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(run.planners[0].flags, done);
        // Either bit alone holds it back.
        for half in [REVIEWED, CYCLED] {
            let (s, run) = reviewed([done, half | 0xa_0000], false);
            assert!(!s.grow);
            assert_eq!(run.controller.flags, 0);
        }
    }

    #[test]
    fn with_every_plan_held_the_strategy_grows_unless_short() {
        // All done, all held (0x80000), none gave up: grow (+0x14) and
        // replan; each planner loses 0x8000000. Short (+0x10 bit 1): no.
        let done = REVIEWED | CYCLED | 0xa_0000;
        let (s, run) = reviewed([done, done], false);
        assert!(s.grow && !s.shrink);
        assert_eq!(run.controller.flags, REPLAN);
        assert!(run.planners.iter().all(|p| p.flags == CYCLED | 0xa_0000));
        let (s, run) = reviewed([done, done], true);
        assert!(!s.grow);
        assert_eq!(run.controller.flags, 0);
        // A sector not held (no 0x80000): no growth.
        let (s, _) = reviewed([done, REVIEWED | CYCLED | 0x2_0000], false);
        assert!(!s.grow);
    }

    #[test]
    fn two_planners_giving_up_shrink_the_list() {
        // More than one 0x10000, the last planner's among them: shrink
        // (+0x18) and replan; one alone does neither, and blocks growth.
        let gave_up = REVIEWED | CYCLED | 0xb_0000;
        let done = REVIEWED | CYCLED | 0xa_0000;
        let (s, run) = reviewed([gave_up, gave_up], false);
        assert!(s.shrink && !s.grow);
        assert_eq!(run.controller.flags, REPLAN);
        let (s, run) = reviewed([gave_up, done], false);
        assert!(!s.shrink && !s.grow);
        assert_eq!(run.controller.flags, 0);
        // A planner without 0x20000 asks for a replan by itself.
        let (s, run) = reviewed([REVIEWED | CYCLED | 0x8_0000, gave_up], false);
        assert!(!s.shrink && !s.grow);
        assert_eq!(run.controller.flags, REPLAN);
    }

    #[test]
    fn a_new_posture_clears_the_planners_review_bit() {
        // FUN_004e54f0 on the old strategy: every planner on controller
        // +0xd8 loses +0x60 0x8000000; the first strategy has no old one.
        let mut planners = vec![SectorPlanner {
            flags: REVIEWED | CYCLED,
            ..SectorPlanner::default()
        }];
        let mut strategy = Strategy::default();
        let mut controller = Controller::default();
        strategy.choose_posture(&Tallies::default(), &mut controller, &mut planners);
        assert_eq!(planners[0].flags, REVIEWED | CYCLED);
        controller.yard_requests = 1;
        strategy.choose_posture(&Tallies::default(), &mut controller, &mut planners);
        assert_eq!(planners[0].flags, CYCLED);
    }

    #[test]
    fn a_posture_is_kept_while_its_own_condition_holds() {
        // FUN_004bf490's switch on +0x5c: posture 6 needs only strength,
        // no yard requests and spare regiments, though the choice would now
        // give 5; posture 2 keeps below the systems' energy plus one; a
        // kept posture leaves controller +4's bits.
        let strong = Tallies {
            strength: 3,
            enemy_strength: 1,
            own_systems: 4,
            free_energy: 3,
            free_raw: 2,
            open_sectors: 1,
            fleet_regiments: 5,
            weakest_enemy: 9,
            ..Tallies::default()
        };
        assert_eq!(posture_of(strong, 0, 6), (6, 0));
        assert_eq!(posture_of(strong, 0, 0), (5, SECURE));
        let fair = Tallies {
            fleet_regiments: 0,
            free_energy: 4,
            ..strong
        };
        assert_eq!(posture_of(fair, 0, 2), (2, 0));
        assert_eq!(posture_of(fair, 0, 0), (5, SECURE));
        // Posture 4 holds while a module waits; posture 1 is never kept.
        assert_eq!(posture_of(strong, 1, 4), (4, 0));
        assert_eq!(posture_of(fair, 0, 1), (5, SECURE));
    }

    #[test]
    fn a_new_strategy_replans_and_a_kept_one_does_not() {
        // FUN_004bf490: a strategy whose kind (slot +0xc) is not the
        // posture goes; a new one sets controller +4 bit 2.
        let mut strategy = Strategy::default();
        let mut controller = Controller::default();
        let weak = Tallies::default();
        strategy.choose_posture(&weak, &mut controller, &mut []);
        assert_eq!((strategy.kind, controller.flags & REPLAN), (1, REPLAN));
        controller.flags = 0;
        strategy.grow = true;
        strategy.choose_posture(&weak, &mut controller, &mut []);
        assert_eq!((controller.flags & REPLAN, strategy.grow), (0, true));
        controller.yard_requests = 1;
        strategy.choose_posture(&weak, &mut controller, &mut []);
        assert_eq!((strategy.kind, controller.flags & REPLAN), (4, REPLAN));
        assert!(!strategy.grow);
    }

    #[test]
    fn with_no_plan_the_strategy_grows_to_one_sector() {
        // FUN_004e5560 with an empty list: every plan is done, none gave
        // up, so it grows (+0x14) and asks for a replan; the next state 1
        // (FUN_004e5530 -> FUN_004e62b0) names one sector: kind 3 on the
        // sector with +0x24 0x40000000 and +0x2c 4 with the most +0x108.
        let (mut g, keys) = galaxy(3);
        for (i, v) in [(0, 5), (1, 9)] {
            g.sectors[i].candidates = 0x4000_0000;
            g.sectors[i].summary = 4;
            g.sectors[i].values[sector_at(0x108)] = v;
        }
        let mut run = Run::new(g);
        run.tallies.strength = 0;
        let mut strategy = Strategy::default();
        run.with(|io| strategy.step(io));
        assert_eq!(strategy.posture, 1);
        assert!(strategy.plans.is_empty());
        assert!(strategy.grow);
        assert_eq!(run.controller.flags & REPLAN, REPLAN);
        run.with(|io| strategy.step(io));
        assert_eq!(
            strategy.plans,
            vec![SectorPlan {
                sector: keys[1],
                kind: 3,
                param: 0
            }]
        );
        assert_eq!(run.controller.flags & REPLAN, 0);
        let record = run.galaxy.sector(keys[1]).unwrap();
        assert_eq!(
            (record.summary & 0x800_0000, record.plan, record.plan_id),
            (0x800_0000, 3, 1)
        );
        // With a plan and no planner the review stays (port: module 8's
        // planners are not ported).
        assert!(!strategy.grow);
    }

    #[test]
    fn the_shared_choices_fill_the_list_in_their_order() {
        // FUN_004e62b0's four queries: kind 3 on +0x24 0x40000000 with
        // +0x2c 4, then 0x16 without 8, then 6 (each by the most +0x108),
        // then kind 1 on 0x80000 without 4 by the most +0x80; a sector
        // already named is skipped (FUN_004e5260).
        let (mut g, keys) = galaxy(5);
        g.sectors[0].candidates = 0x4000_0000;
        g.sectors[0].summary = 6;
        g.sectors[1].summary = 6;
        g.sectors[1].values[sector_at(0x108)] = 2;
        g.sectors[2].summary = 0x8_0000;
        g.sectors[2].values[sector_at(0x80)] = 1;
        g.sectors[3].summary = 0x8_0000;
        g.sectors[3].values[sector_at(0x80)] = 3;
        g.sectors[4].summary = 0x8_0004;
        let mut run = Run::new(g);
        let mut strategy = Strategy {
            posture: 1,
            kind: 1,
            ..Strategy::default()
        };
        strategy.plans = (0..3)
            .map(|i| SectorPlan {
                sector: keys[i],
                kind: 3,
                param: 0,
            })
            .collect();
        strategy.grow = true;
        run.with(|io| strategy.name_sectors(io));
        let named: Vec<(SectorKey, u32)> =
            strategy.plans.iter().map(|p| (p.sector, p.kind)).collect();
        assert_eq!(
            named,
            vec![(keys[0], 3), (keys[1], 3), (keys[3], 1), (keys[2], 1)]
        );
        assert!(!strategy.short);
        // One more than there are sectors to name: short (+0x10 bit 1).
        strategy.grow = true;
        run.with(|io| strategy.name_sectors(io));
        assert!(strategy.short);
        assert_eq!(strategy.plans.len(), 4);
        // Posture 3 does not forbid +0x2c 4 for kind 1.
        let mut third = Strategy {
            posture: 3,
            kind: 3,
            grow: true,
            plans: strategy.plans.clone(),
            ..Strategy::default()
        };
        run.with(|io| third.name_sectors(io));
        assert!(third
            .plans
            .iter()
            .any(|p| p.sector == keys[4] && p.kind == 1));
    }

    #[test]
    fn a_naming_that_finds_nothing_keeps_the_old_plans() {
        // FUN_004e62b0 replaces the module's list only when the new one has
        // an entry; a new list clears the old sectors' fields first.
        let (g, keys) = galaxy(2);
        let mut run = Run::new(g);
        let old = SectorPlan {
            sector: keys[0],
            kind: 5,
            param: 0,
        };
        let mut strategy = Strategy {
            posture: 1,
            kind: 1,
            plans: vec![old],
            ..Strategy::default()
        };
        mark_plans(&mut run.galaxy, &strategy.plans);
        run.with(|io| strategy.name_sectors(io));
        assert_eq!(strategy.plans, vec![old]);
        assert!(strategy.short);
        run.galaxy.sectors[1].summary = 6;
        run.with(|io| strategy.name_sectors(io));
        assert_eq!(strategy.plans[0].sector, keys[1]);
        let cleared = run.galaxy.sector(keys[0]).unwrap();
        assert_eq!(
            (cleared.summary & 0x800_0000, cleared.plan, cleared.plan_id),
            (0, 0, 0)
        );
    }

    #[test]
    fn a_planned_sector_without_a_planner_counts_minus_one() {
        // The end of FUN_004321e0: +0x30 set gives +0x2c 0x4000000 and,
        // with no planner (+0x38), 0x10000 and +0x18c = -1; a planner
        // without a plan gives +0x18c = 1.
        let (mut g, keys) = galaxy(3);
        g.sectors[2].planner = 7;
        let plan = SectorPlan {
            sector: keys[0],
            kind: 3,
            param: 0,
        };
        mark_plans(&mut g, &[plan]);
        let at = sector_at(0x18c);
        let s = &g.sectors;
        assert_eq!((s[0].summary, s[0].values[at]), (0xc01_0000, -1));
        assert_eq!((s[1].summary, s[1].values[at]), (0, 0));
        assert_eq!((s[2].summary, s[2].values[at]), (0, 1));
        g.sectors[0].planner = 1;
        g.sectors[0].summary = 0;
        g.sectors[0].values[at] = 0;
        g.sectors[0].plan_bits();
        assert_eq!(
            (g.sectors[0].summary, g.sectors[0].values[at]),
            (0x400_0000, 0)
        );
        let t = Tallies::of(&g, 0, 0);
        assert_eq!(t.planner_balance, 1);
    }

    #[test]
    fn the_planned_query_keys_on_the_plan_id_and_100_without_one() {
        // FUN_00419e90: FUN_0041c360(entry, +0x3c), or 100 when +0x3c is 0.
        let (mut g, keys) = galaxy(3);
        g.sectors[0].plan_id = 2;
        g.sectors[1].plan_id = 1;
        let order = g.planned_sectors([ANY; 3], Order::Descending, &mut || false);
        assert_eq!(order, vec![keys[2], keys[0], keys[1]]);
    }

    #[test]
    fn the_first_choice_follows_the_posture() {
        // FUN_004e7990 (posture 2): kind 5 on a sector without +0x2c 2 by
        // the fewest +0x80 (order 2, the list's last); FUN_004ea060
        // (posture 5): kind 2 on +0x2c 0x40000 by the most +0x190.
        let (mut g, keys) = galaxy(3);
        g.sectors[0].summary = 2;
        g.sectors[1].values[sector_at(0x80)] = 4;
        g.sectors[2].values[sector_at(0x80)] = 1;
        g.sectors[1].summary = 0x4_0000;
        let mut run = Run::new(g);
        let two = Strategy {
            posture: 2,
            ..Strategy::default()
        };
        assert_eq!(run.with(|io| two.first_choice(io)), Some((keys[2], 5)));
        // Wanting no sector (no plan, no growth), FUN_004e7990 names none.
        let mut idle = Strategy {
            posture: 2,
            kind: 2,
            ..Strategy::default()
        };
        run.with(|io| idle.name_sectors(io));
        assert!(idle.plans.is_empty());
        assert!(!idle.short);
        let five = Strategy {
            posture: 5,
            ..Strategy::default()
        };
        assert_eq!(run.with(|io| five.first_choice(io)), Some((keys[1], 2)));
        // Posture 6: kind 6 on a 0x408 sector, else kind 1 on 0x80100,
        // else kind 5 on a sector without 2.
        let six = Strategy {
            posture: 6,
            ..Strategy::default()
        };
        // Kind 5 by the most +0x7c: a tie keeps the query's order (the coin
        // never goes ahead), so the last sector.
        assert_eq!(run.with(|io| six.first_choice(io)), Some((keys[2], 5)));
        run.galaxy.sectors[0].summary = 0x8_0100;
        assert_eq!(run.with(|io| six.first_choice(io)), Some((keys[0], 1)));
        run.galaxy.sectors[2].summary = 0x408;
        assert_eq!(run.with(|io| six.first_choice(io)), Some((keys[2], 6)));
        // Posture 3: kind 6 on 0x408 without 0x40000, else kind 5.
        let three = Strategy {
            posture: 3,
            ..Strategy::default()
        };
        assert_eq!(run.with(|io| three.first_choice(io)), Some((keys[2], 6)));
        run.galaxy.sectors[2].summary = 0x4_0408;
        assert_eq!(run.with(|io| three.first_choice(io)), Some((keys[2], 6)));
        run.galaxy.sectors[2].summary = 2;
        assert_eq!(run.with(|io| three.first_choice(io)), Some((keys[1], 5)));
    }

    #[test]
    fn posture_six_takes_the_empire_s_headquarters_sector_first() {
        // FUN_004ead50: with controller +4 bit 0x400 (a sector's +0x2c
        // 0x80000000, FUN_0041b7e0) and the Empire (strategy +4 == 2), a
        // sector with 0x80000408 by the most +0x19c, merged with nothing.
        let (mut g, keys) = galaxy(2);
        g.sectors[0].summary = 0x408;
        g.sectors[0].values[sector_at(0x19c)] = 9;
        g.sectors[1].summary = 0x8000_0408;
        let mut run = Run::new(g);
        let six = Strategy {
            posture: 6,
            ..Strategy::default()
        };
        assert_eq!(run.with(|io| six.first_choice(io)), Some((keys[1], 6)));
        run.is_alliance = true;
        assert_eq!(run.with(|io| six.first_choice(io)), Some((keys[0], 6)));
    }

    #[test]
    fn posture_seven_names_a_second_sector_of_kind_six() {
        // FUN_004e6c00: after kind 5 (without +0x2c 2, a tie on +0x80 to
        // the last), kind 6 on a 0x408 sector without 0x40000 by the
        // least +0xa0, once; then the shared choices (6: kind 3).
        let (mut g, keys) = galaxy(3);
        g.sectors[0].summary = 0x40e;
        g.sectors[0].values[sector_at(0xa0)] = 5;
        g.sectors[1].summary = 0x408;
        g.sectors[1].values[sector_at(0xa0)] = 2;
        let mut run = Run::new(g);
        let mut seven = Strategy {
            posture: 7,
            kind: 7,
            grow: true,
            plans: vec![
                SectorPlan {
                    sector: keys[2],
                    kind: 3,
                    param: 0,
                };
                2
            ],
            ..Strategy::default()
        };
        run.with(|io| seven.name_sectors(io));
        let named: Vec<(SectorKey, u32)> = seven.plans.iter().map(|p| (p.sector, p.kind)).collect();
        assert_eq!(named, vec![(keys[2], 5), (keys[1], 6), (keys[0], 3)]);
    }

    #[test]
    fn posture_four_targets_yards_of_the_waiting_module_s_kind() {
        // FUN_004e5b10: the highest waiting bit picks the yard kind (0x80
        // construction yards, +0x124, v[6], plan parameter 3; 2 shipyards,
        // +0x11c, v[4], 1); a +0x2c 2 sector holding them, and a system
        // with +0x30 0x20 without 0x101 that has one.
        let mut keys = slotmap::SlotMap::<SystemKey, ()>::with_key();
        let mut rec = record(keys.insert(()), 0x24);
        rec.values[4] = 1;
        let (mut g, sectors) = galaxy(2);
        g.sectors[1] = SectorRecord::build(sectors[1], (10, 0), vec![rec]);
        g.sectors[1].summary = 2;
        g.sectors[1].values[sector_at(0x11c)] = 1;
        g.sectors[0].summary = 2;
        g.sectors[0].values[sector_at(0x124)] = 1;
        let mut run = Run::new(g);
        let four = Strategy {
            posture: 4,
            ..Strategy::default()
        };
        run.controller.yard_requests = 0x82;
        // 0x80 first: sector 0 has construction yards but no system with
        // one, so 2 (shipyards) decides.
        assert_eq!(run.with(|io| four.yard_target(io)), (1, Some(sectors[1])));
        run.controller.yard_requests = 0x80;
        assert_eq!(run.with(|io| four.yard_target(io)), (3, None));
        run.controller.yard_requests = 0;
        assert_eq!(run.with(|io| four.yard_target(io)), (0, None));
        run.controller.yard_requests = 0x24;
        assert_eq!(run.with(|io| four.yard_target(io)), (2, None));
    }

    #[test]
    fn a_sector_s_staleness_is_the_days_since_its_opponent_systems_were_seen() {
        // FUN_004317e0 over FUN_00433de0: (day - +0x44) for each system
        // with +0x30 0x20 and without 1, over +0x84 + +0x80.
        let mut keys = slotmap::SlotMap::<SystemKey, ()>::with_key();
        let mut system = |kind| record(keys.insert(()), kind);
        let records = vec![system(0x24), system(0x24), system(0x21), system(2)];
        let seen: HashMap<SystemKey, u64> = [(records[0].system, 90)].into_iter().collect();
        let (_, sectors) = galaxy(1);
        let mut sector = SectorRecord::build(sectors[0], (0, 0), records);
        assert_eq!(
            sector.values[sector_at(0x80)] + sector.values[sector_at(0x84)],
            3
        );
        // (100 - 90) + (100 - 0) = 110, over 3.
        assert_eq!(staleness(&sector, 100, &seen), 36);
        sector.values[sector_at(0x80)] = 0;
        sector.values[sector_at(0x84)] = 0;
        assert_eq!(staleness(&sector, 100, &seen), 110);
    }
}
