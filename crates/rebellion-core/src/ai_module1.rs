//! Module 1 of the AI player (`FUN_004da950`, step `FUN_004dabe0`): it
//! keeps the side's construction yards ("sites") and builds mines,
//! refineries and more yards with them
//! (`ghidra/notes/ai-production-modules.md`, "Module 1").

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ai::{AIAction, MARK_MINES};
use crate::ai_records::{picks, sector_at, Galaxy, Mask, Order, CLASS_CONSTRUCTION};
use crate::dat::Faction;
use crate::ids::{DatId, SectorKey, SystemKey};
use crate::manufacturing::{BuildableKind, FacilityBuild, ProductionArea};
use crate::research::ResearchState;
use crate::scrap::ScrapTarget;
use crate::world::{FacilityRef, GameWorld};

/// `+0x38` while a cycle runs (`FUN_004dabe0`).
pub const CYCLE: u32 = 0x3ec;

/// The yard token module 1 holds the construction yards with (controller
/// `+0xc`, `+8`; `FUN_004db2f0` state 1).
pub const YARD_TOKEN: u32 = 0x80;

/// The refinery class state 6 builds (`FUN_004db2f0`, `0x2d000002`).
pub const REFINERY_CLASS: u32 = 0x2d00_0002;

/// The mine class state 6 builds (`FUN_004db2f0`, `0x2c000001`).
pub const MINE_CLASS: u32 = 0x2c00_0001;

/// Record `+0x30`: a site claimed by a module (`FUN_004dd120`).
pub const CLAIMED: u32 = 0x1000_0000;

/// Controller `+4` bit `0x40`: no site was found (`FUN_004daf10`).
pub const NO_SITE: u32 = 0x40;

/// What `+0x40` holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Target {
    #[default]
    None,
    Sector(SectorKey),
    System(SystemKey),
}

impl Target {
    pub(crate) fn system(self) -> Option<SystemKey> {
        match self {
            Self::System(s) => Some(s),
            _ => None,
        }
    }
}

/// What `+0x44` holds: a site system, then its construction manager
/// (`FUN_00509670(system, 1)`), which the port names by its system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Yard {
    #[default]
    None,
    System(SystemKey),
    Manager(SystemKey),
}

/// Module 1's own fields (`FUN_004da950`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module1 {
    /// `+0x38`: 0, or [`CYCLE`].
    pub phase: u32,
    /// `+0x3c`: the cycle's state.
    pub state: u32,
    /// `+0x40`.
    pub target: Target,
    /// `+0x44`.
    pub yard: Yard,
    /// `+0x48`: the class to build.
    pub class: Option<DatId>,
    /// `+0x4c`: how many.
    pub count: i32,
    /// `+0x50`: the target's free room `v[0x33]`.
    pub room: i32,
    /// `+0x54`: the yard's construction yards `v[6]`.
    pub capacity: i32,
    /// `+0x58`: the sites, marked [`MARK_MINES`].
    pub sites: Vec<SystemKey>,
    /// `+0x60`: the claimed sites, record `+0x30` [`CLAIMED`].
    pub claimed: Vec<SystemKey>,
}

/// The controller words the modules share the construction yards with,
/// and its flags (`FUN_004178f0`: `+8` 0, `+0xc` `0x80`, `+0x10` `0x40`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Controller {
    /// `+8`: the modules asking for the yards, one bit each.
    pub yard_requests: u32,
    /// `+0xc`: the bit that holds the yards.
    pub yard_holder: u32,
    /// `+0x10`: the next free bit.
    pub yard_next: u32,
    /// `+4`, the bits the modules set ([`NO_SITE`]).
    pub flags: u32,
    /// `+0x374`, the shield goal of an own unmarked system's record
    /// ([`crate::ai_records::SHIELD_GOAL`] at setup), which module 3 raises
    /// (`FUN_00418240(4)`).
    pub shield_goal: i32,
}

impl Default for Controller {
    /// `FUN_004178f0`.
    fn default() -> Self {
        Self {
            yard_requests: 0,
            yard_holder: YARD_TOKEN,
            yard_next: 0x40,
            flags: 0,
            shield_goal: crate::ai_records::SHIELD_GOAL,
        }
    }
}

impl Controller {
    /// `FUN_004199b0`: pass the yards on. With no request the next free
    /// bit holds them and the free bit moves right (from `0x80` after 1);
    /// otherwise the holder moves right, wrapping, to the first request,
    /// which is granted and cleared.
    pub fn pass_yards(&mut self) {
        if self.yard_requests == 0 {
            self.yard_holder = self.yard_next;
            self.yard_next >>= 1;
            if self.yard_next == 0 {
                self.yard_next = 0x80;
            }
        } else {
            while self.yard_requests & self.yard_holder == 0 {
                self.yard_holder >>= 1;
                if self.yard_holder == 0 {
                    self.yard_holder = 0x80;
                }
            }
            self.yard_requests &= !self.yard_holder;
        }
    }
}

/// The controller's tallies module 1 reads (phase 4, `FUN_0041b7e0`;
/// `FUN_004182a0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tallies {
    /// `+0x1d0`: the side's mines (own systems' `v[2]`).
    pub mines: i32,
    /// `+0x1d4`: the side's refineries (`v[3]`).
    pub refineries: i32,
    /// `+0x184`: the side's maintenance capacity (side `+0x58`).
    pub capacity: i32,
    /// `+0x188`: the side's maintenance requirement (side `+0x70`).
    pub requirement: i32,
    /// `+0x1dc`: the side's defense maintenance (own systems' `v[31]`).
    pub defenses: i32,
    /// `+0x2c4`: the summed `+0x64` of the side's object records, its
    /// special forces' maintenance (`FUN_0041bc10`, `FUN_00402230`).
    pub special_upkeep: i32,
    /// `+0x2fc`: the most of the sector records' `+0x140`
    /// (`FUN_0041b7e0`).
    pub special_goal: i32,
    /// `+0x2ec..+0x2f8`: the side's special forces of each kind whose
    /// record `+0x34` has `0x800000` (`FUN_0041bc10`). No code sets that
    /// bit (`ai-production-modules.md`, "Module 5"), so these are 0.
    pub special_ready: [i32; 4],
    /// `+0x1d8`: the regiments' maintenance on the side's planets (the
    /// sector records' `+0xfc`, own systems' `v[30]`; `FUN_0041b7e0`).
    pub regiment_upkeep: i32,
    /// `+0x250`: the regiments' maintenance aboard the side's fleets (the
    /// own fleet records' `values[15]`, `FUN_0041ba80`).
    pub cargo_upkeep: i32,
    /// Controller `+4` bit `0x20`: some sector record has `+0x2c` bit
    /// `0x2000000` (`FUN_0041b7e0`).
    pub sector_short: bool,
}

impl Tallies {
    /// The mines, refineries and defense maintenance summed over the
    /// side's sector records (`+0xa8`, `+0xac`, `+0x100`, `FUN_0041b7e0`),
    /// with the side's maintenance.
    #[must_use]
    pub fn of(galaxy: &Galaxy, capacity: i32, requirement: i32) -> Self {
        let sum = |offset| {
            galaxy
                .sectors
                .iter()
                .map(|s| s.values[sector_at(offset)])
                .sum()
        };
        Self {
            mines: sum(0xa8),
            refineries: sum(0xac),
            capacity,
            requirement,
            defenses: sum(0x100),
            special_upkeep: 0,
            special_goal: galaxy
                .sectors
                .iter()
                .map(|s| s.values[sector_at(0x140)])
                .fold(0, i32::max),
            special_ready: [0; 4],
            regiment_upkeep: sum(0xfc),
            cargo_upkeep: galaxy
                .fleets
                .values()
                .filter(|f| f.flags & crate::ai_records::FLEET_OWN != 0)
                .map(|f| f.values[15])
                .sum(),
            sector_short: galaxy.sectors.iter().any(|s| s.summary & 0x200_0000 != 0),
        }
    }
}

/// What a step reads and changes besides the module.
pub struct Inputs<'a> {
    pub world: &'a GameWorld,
    pub research: &'a ResearchState,
    pub galaxy: &'a mut Galaxy,
    pub controller: &'a mut Controller,
    /// The side's yard marks ([`crate::ai::AIState::yard_marks`]).
    pub marks: &'a mut HashMap<SystemKey, u32>,
    /// The side's kept record `+0x30` bits.
    pub kinds: &'a mut HashMap<SystemKey, u32>,
    pub tallies: Tallies,
    pub is_alliance: bool,
    /// `FUN_0041cd80(10) < 5`, the ranked lists' coin.
    pub coin: &'a mut dyn FnMut() -> bool,
}

impl Inputs<'_> {
    pub(crate) fn side(&self) -> Faction {
        Faction::of_alliance(self.is_alliance)
    }

    pub(crate) fn set_mark(&mut self, system: SystemKey, set: u32, clear: u32) {
        self.galaxy.update_state(system, set, clear);
        let value = self.marks.entry(system).or_default();
        *value = (*value & !clear) | set;
        if *value == 0 {
            self.marks.remove(&system);
        }
    }

    pub(crate) fn set_kind(&mut self, system: SystemKey, set: u32, clear: u32) {
        self.galaxy.update_kind(system, set, clear);
        let value = self.kinds.entry(system).or_default();
        *value = (*value & !clear) | set;
        if *value == 0 {
            self.kinds.remove(&system);
        }
    }

    pub(crate) fn sectors(&mut self, masks: [Mask; 3], key: usize, order: Order) -> Vec<SectorKey> {
        self.galaxy.sector_query(masks, key, order, self.coin)
    }

    pub(crate) fn systems(
        &mut self,
        sector: Option<SectorKey>,
        masks: [Mask; 3],
        key: usize,
        order: Order,
    ) -> Vec<SystemKey> {
        sector.map_or_else(Vec::new, |s| {
            self.galaxy.system_query(s, masks, key, order, self.coin)
        })
    }

    /// The side's facilities of `family` at `system`, in the system's
    /// order (`FUN_0052cab0` and its siblings).
    pub(crate) fn facilities(&self, system: SystemKey, family: u8) -> Vec<ScrapTarget> {
        let side = self.side();
        let Some(sys) = self.world.systems.get(system) else {
            return Vec::new();
        };
        let w = self.world;
        let of = |class: DatId, owner: Faction| class.family() == family && owner == side;
        match family {
            0x22..=0x27 => sys
                .defense_facilities
                .iter()
                .filter(|&&k| {
                    w.defense_facilities
                        .get(k)
                        .is_some_and(|f| of(f.class_dat_id, f.side))
                })
                .map(|&k| ScrapTarget::Facility(FacilityRef::Defense(k)))
                .collect(),
            0x28..=0x2a => sys
                .manufacturing_facilities
                .iter()
                .filter(|&&k| {
                    w.manufacturing_facilities
                        .get(k)
                        .is_some_and(|f| of(f.class_dat_id, f.side))
                })
                .map(|&k| ScrapTarget::Facility(FacilityRef::Manufacturing(k)))
                .collect(),
            _ => sys
                .production_facilities
                .iter()
                .filter(|&&k| {
                    w.production_facilities
                        .get(k)
                        .is_some_and(|f| of(f.class_dat_id, f.side))
                })
                .map(|&k| ScrapTarget::Facility(FacilityRef::Production(k)))
                .collect(),
        }
    }

    pub(crate) fn has_manager(&self, system: SystemKey) -> bool {
        has_construction_yard(self.world, self.side(), system)
    }

    /// The side's manager of the yards of `family` at `system`
    /// (`FUN_00509670`; hyp: one stands while the side has such a yard).
    pub(crate) fn has_manager_of(&self, system: SystemKey, family: u8) -> bool {
        has_yard(self.world, self.side(), system, family)
    }
}

/// `FUN_004f3220` then `FUN_00509670(system, 1)`: `side`'s construction
/// manager at `system`. hyp: a manager stands while the side has a
/// construction yard (`0x2a`) there.
#[must_use]
pub fn has_construction_yard(world: &GameWorld, side: Faction, system: SystemKey) -> bool {
    has_yard(world, side, system, 0x2a)
}

/// Whether `side` has a yard of `family` (`0x28..0x2a`) at `system`.
#[must_use]
pub fn has_yard(world: &GameWorld, side: Faction, system: SystemKey, family: u8) -> bool {
    world.systems.get(system).is_some_and(|sys| {
        sys.manufacturing_facilities.iter().any(|&k| {
            world
                .manufacturing_facilities
                .get(k)
                .is_some_and(|f| f.side == side && f.class_dat_id.family() == family)
        })
    })
}

/// One step's result: the order it gives and whether the module is done
/// for this tick (`+0x20`) or raises its priority (`+0x34`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Step {
    pub order: Option<AIAction>,
    pub complete: bool,
    pub raise: bool,
}

impl Step {
    pub(crate) const fn pending() -> Self {
        Self {
            order: None,
            complete: false,
            raise: false,
        }
    }

    pub(crate) const fn complete() -> Self {
        Self {
            order: None,
            complete: true,
            raise: false,
        }
    }

    /// An order step of states 3 and 7: complete with an order, which
    /// raises the priority; without one the module steps on.
    pub(crate) fn ordered(order: Option<AIAction>) -> Self {
        let has = order.is_some();
        Self {
            order,
            complete: has,
            raise: has,
        }
    }
}

pub(crate) const ANY: Mask = Mask::new(0, 0);

impl Module1 {
    /// One step (`FUN_004dabe0`). The scheduler steps only a ready module,
    /// so the not-ready branch never runs.
    pub fn step(&mut self, io: &mut Inputs<'_>) -> Step {
        if self.phase != 0 {
            if self.phase == CYCLE {
                return self.cycle(io);
            }
            self.phase = 0;
            return Step::pending();
        }
        if self.keep_site(io) {
            self.phase = CYCLE;
            return Step::pending();
        }
        if self.find_site(io) {
            self.phase = CYCLE;
            self.state = 0;
            return Step::pending();
        }
        if io.controller.yard_holder != YARD_TOKEN || io.controller.flags & NO_SITE != 0 {
            let budget = wanted(io, &self.sites, PERCENT);
            self.target = adopt_or_request(
                io,
                &mut self.sites,
                &mut self.class,
                MARK_MINES,
                YARD_TOKEN,
                budget,
                CONSTRUCTION,
            );
        }
        claim(io, &self.sites, &mut self.claimed, CONSTRUCTION);
        self.phase = 0;
        self.state = 0;
        Step::complete()
    }

    /// A refused order (slot `+0x18`, `FUN_004dd810`).
    pub fn refused(&mut self) {
        self.phase = 0;
        self.state = 0;
    }

    /// The cycle (`FUN_004db2f0`).
    fn cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            1 => {
                io.controller.yard_requests &= !YARD_TOKEN;
                if io.controller.yard_holder == YARD_TOKEN {
                    io.controller.pass_yards();
                    self.state = 2;
                } else {
                    self.state = 10;
                }
                Step::pending()
            }
            2 => {
                if self.choose_target(io) {
                    self.state = 3;
                    Step::pending()
                } else {
                    self.end()
                }
            }
            3 => {
                let order = scrap_at(io, self.target.system(), CONSTRUCTION);
                self.state = 4;
                Step::ordered(order)
            }
            4 => {
                self.state = self.plan_yards(io);
                if self.state == 0 {
                    self.phase = 0;
                    return Step::complete();
                }
                Step::pending()
            }
            5 => {
                let order = scrap_spare_yard(
                    io,
                    &mut self.sites,
                    &mut self.target,
                    MARK_MINES,
                    CONSTRUCTION,
                );
                self.state = 0;
                self.phase = 0;
                Step {
                    order,
                    complete: true,
                    raise: false,
                }
            }
            6 => {
                let found = if io.tallies.refineries < io.tallies.mines {
                    self.class = Some(DatId::new(REFINERY_CLASS));
                    self.refinery_site(io)
                } else {
                    self.class = Some(DatId::new(MINE_CLASS));
                    self.mine_site(io)
                };
                if found {
                    self.count = 1;
                    self.state = 7;
                } else {
                    self.state = 0;
                    self.phase = 0;
                }
                // State 6 always runs state 7 (`0x4db44e`).
                self.state_seven(io)
            }
            7 => self.state_seven(io),
            8 => match destination(self.target, self.yard) {
                Some(order) => {
                    self.state = 9;
                    Step {
                        order: Some(order),
                        complete: true,
                        raise: true,
                    }
                }
                None => self.end(),
            },
            9 => {
                let order = build(io, self.yard, self.class, self.count);
                self.state = 0;
                self.phase = 0;
                Step {
                    order,
                    complete: true,
                    raise: false,
                }
            }
            10 => {
                let budget = wanted(io, &self.sites, PERCENT);
                self.target = adopt_or_request(
                    io,
                    &mut self.sites,
                    &mut self.class,
                    MARK_MINES,
                    YARD_TOKEN,
                    budget,
                    CONSTRUCTION,
                );
                self.end()
            }
            _ => {
                self.state = 1;
                Step::pending()
            }
        }
    }

    fn end(&mut self) -> Step {
        self.state = 0;
        self.phase = 0;
        Step::complete()
    }

    /// State 7 then `+0x3c = 8` (`FUN_004db2f0`).
    fn state_seven(&mut self, io: &mut Inputs<'_>) -> Step {
        let order = self.clear_target(io);
        self.state = 8;
        Step::ordered(order)
    }

    /// `FUN_004daca0`: keep a site that has an idle construction yard.
    fn keep_site(&mut self, io: &mut Inputs<'_>) -> bool {
        self.capacity = 0;
        let mut i = 0;
        while i < self.sites.len() {
            let site = self.sites[i];
            let Some(r) = io.galaxy.system(site).copied() else {
                self.sites.remove(i);
                continue;
            };
            if r.kind & 1 == 0 || r.state & MARK_MINES == 0 {
                io.set_mark(site, 0, MARK_MINES);
                self.sites.remove(i);
                continue;
            }
            if r.state & 0x800 == 0 {
                if self.sites.len() == 1 && r.values[6] == 0 {
                    io.set_mark(site, 0, MARK_MINES);
                    self.sites.remove(i);
                    continue;
                }
                i += 1;
                continue;
            }
            if r.state & 3 == 0 {
                self.capacity = r.values[6];
                self.yard = Yard::System(site);
                break;
            }
            i += 1;
        }
        if let Yard::System(site) = self.yard {
            if io.has_manager(site) {
                self.yard = Yard::Manager(site);
                return true;
            }
        }
        false
    }

    /// `FUN_004daf10`: with no site, find one and mark it.
    fn find_site(&mut self, io: &mut Inputs<'_>) -> bool {
        self.capacity = 0;
        if !self.sites.is_empty() {
            return false;
        }
        let bit = [Mask::new(0x2000, 0), ANY, ANY];
        let lists = vec![
            io.sectors(bit, 4, Order::Descending),
            io.sectors(bit, 0x15, Order::Ascending),
            io.sectors(bit, 8, Order::Ascending),
        ];
        let sector = crate::ai_records::pick(&lists);
        self.target = sector.map_or(Target::None, Target::Sector);
        let masks = [Mask::new(0x800, 0x380_0000), ANY, Mask::new(1, 0)];
        let lists = vec![
            io.systems(sector, masks, 6, Order::Ascending),
            io.systems(sector, masks, 0, Order::Ascending),
            io.systems(sector, masks, 0x33, Order::Ascending),
        ];
        if let Some(system) = crate::ai_records::pick(&lists) {
            self.target = Target::System(system);
        }
        if self.target.system().is_none() {
            let lists = vec![io.sectors([ANY, ANY, Mask::new(4, 0)], 0x14, Order::Ascending)];
            let sector = crate::ai_records::pick(&lists);
            self.target = sector.map_or(Target::None, Target::Sector);
            let lists = vec![io.systems(sector, [ANY, ANY, Mask::new(1, 0)], 6, Order::Ascending)];
            if let Some(system) = crate::ai_records::pick(&lists) {
                self.target = Target::System(system);
            }
        }
        let Some(system) = self.target.system() else {
            io.controller.flags |= NO_SITE;
            return false;
        };
        let Some(r) = io.galaxy.system(system).copied() else {
            return false;
        };
        self.sites.push(system);
        io.set_mark(system, MARK_MINES, 0x380_0000);
        if r.state & 0x800 == 0 {
            return false;
        }
        if io.has_manager(system) {
            self.capacity = r.values[6];
            self.yard = Yard::Manager(system);
        }
        true
    }

    /// State 2 (`FUN_004db510`): the target, a site with room and the
    /// fewest construction yards, or a new site.
    fn choose_target(&mut self, io: &mut Inputs<'_>) -> bool {
        self.target = Target::None;
        self.room = 0;
        let mut best = 1000;
        let mut found = false;
        for &site in &self.sites {
            let Some(r) = io.galaxy.system(site) else {
                continue;
            };
            if r.kind & 1 != 0 && r.state & 3 == 0 && r.values[0x33] > 0 && r.values[6] < best {
                self.room = r.values[0x33];
                best = r.values[6];
                self.target = Target::System(site);
                found = true;
            }
        }
        if found {
            return true;
        }
        let bit = [Mask::new(0x80, 0), ANY, ANY];
        let lists = vec![
            io.sectors(bit, 4, Order::Descending),
            io.sectors(bit, 0x15, Order::Ascending),
            io.sectors(bit, 8, Order::Ascending),
        ];
        let sector = crate::ai_records::pick(&lists);
        self.target = sector.map_or(Target::None, Target::Sector);
        let masks = [Mask::new(0, 0x380_0003), ANY, Mask::new(1, 0x4000_0000)];
        let lists = vec![
            io.systems(sector, masks, 6, Order::Ascending),
            io.systems(sector, masks, 0, Order::Ascending),
            io.systems(sector, masks, 0x33, Order::Ascending),
        ];
        let Some(system) = crate::ai_records::pick(&lists) else {
            return false;
        };
        self.target = Target::System(system);
        if io.galaxy.system(system).is_none() {
            return false;
        }
        self.sites.push(system);
        io.set_mark(system, MARK_MINES, 0);
        true
    }

    /// State 4 (`FUN_004dbd60`): 8 to build construction yards, else 6
    /// (5 is stored for a surplus but overwritten, `0x4dbe48`), 0 with no
    /// target system.
    fn plan_yards(&mut self, io: &mut Inputs<'_>) -> u32 {
        self.count = 0;
        let Some(target) = self.target.system() else {
            return 0;
        };
        let wanted = wanted(io, &self.sites, PERCENT);
        let mut next = 0;
        if wanted > 0 {
            let maintenance;
            (self.class, maintenance) = yard_class(io, CONSTRUCTION);
            if self.class.is_some() && maintenance != 0 {
                self.count = wanted / maintenance;
            }
            if self.count <= 0 {
                return if self.count == 0 { 6 } else { 0 };
            }
            if let Some(r) = io.galaxy.system(target) {
                self.count = self.count.min(self.room.min(r.values[15]));
                self.count = self.count.min(self.capacity);
                next = 8;
            }
        }
        if self.count == 0 {
            next = 6;
        }
        next
    }

    /// `FUN_004dc150`: the mine site.
    fn mine_site(&mut self, io: &mut Inputs<'_>) -> bool {
        let bit = [Mask::new(0x100, 0), ANY, ANY];
        let lists = vec![
            io.sectors(bit, 7, Order::Ascending),
            io.sectors(bit, 10, Order::Ascending),
            io.sectors(bit, 9, Order::Ascending),
        ];
        let sector = crate::ai_records::pick(&lists);
        self.target = sector.map_or(Target::None, Target::Sector);
        let masks = [Mask::new(0x80, 0), ANY, ANY];
        let lists = vec![
            io.systems(sector, masks, 0xe, Order::Ascending),
            io.systems(sector, masks, 0, Order::Ascending),
            io.systems(sector, masks, 0xf, Order::Ascending),
        ];
        if let Some(system) = crate::ai_records::pick(&lists) {
            self.target = Target::System(system);
        }
        if self.target.system().is_none() {
            self.target = nearest(io, self.yard, 0x800, [Mask::new(0x40, 0), ANY, ANY]);
        }
        self.target.system().is_some()
    }

    /// `FUN_004dc3f0`: the refinery site.
    fn refinery_site(&mut self, io: &mut Inputs<'_>) -> bool {
        let bit = [Mask::new(0x200, 0), ANY, ANY];
        let lists = vec![
            io.sectors(bit, 7, Order::Ascending),
            io.sectors(bit, 9, Order::Ascending),
        ];
        let sector = crate::ai_records::pick(&lists);
        self.target = sector.map_or(Target::None, Target::Sector);
        let masks = [Mask::new(0x100, 0x3e0_0003), ANY, Mask::new(1, 0)];
        let lists = vec![
            io.systems(sector, masks, 0xf, Order::Ascending),
            io.systems(sector, masks, 0, Order::Ascending),
        ];
        if let Some(system) = crate::ai_records::pick(&lists) {
            self.target = Target::System(system);
        }
        if self.target.system().is_none() {
            self.target = nearest(io, self.yard, 0x400, [Mask::new(0x20, 0), ANY, ANY]);
        }
        self.target.system().is_some()
    }

    /// State 7 (`FUN_004dc650`): clear an unmarked target of its yards.
    fn clear_target(&mut self, io: &mut Inputs<'_>) -> Option<AIAction> {
        let target = self.target.system()?;
        let r = io.galaxy.system(target)?;
        if r.state & crate::ai_records::KEPT_STATE != 0 {
            return None;
        }
        let mut team = io.facilities(target, 0x28);
        team.extend(io.facilities(target, 0x29));
        team.extend(io.facilities(target, 0x2a));
        (!team.is_empty()).then_some(AIAction::Scrap { team })
    }
}

/// Module 1's `+0x24`: the percent of the maintenance capacity it wants in
/// construction yards (`FUN_004da950`).
const PERCENT: i32 = 4;

/// The kind of yard a module's sites keep: what its shared steps count,
/// query, mark and build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct YardKind {
    /// The yards' family (`0x2a`, `0x29`).
    pub family: u8,
    /// The system record's count of them (`v[6]`, `v[5]`).
    pub count: usize,
    /// The system record's upkeep of them (`v[36]`, `v[35]`).
    pub upkeep: usize,
    /// Their class record flag ([`CLASS_CONSTRUCTION`], `CLASS_TRAINING`).
    pub class_flags: u32,
    /// The other yard family a target scrap takes unless marked, with the
    /// mark that spares it.
    pub spared: (u8, u32),
    /// The sector candidate bit (`+0x24`) and key of the adoption query.
    pub sector_bit: u32,
    pub sector_key: usize,
    /// The sector record value of unmarked yards (`+0xc4`, `+0xbc`).
    pub sector_yards: usize,
    /// The marks and states a candidate system may not have (`+0x28`).
    pub forbid: u32,
    /// The system key of the adoption query.
    pub system_key: usize,
    /// The adoption runs only while the module has fewer than two sites.
    pub few_sites_only: bool,
    /// The system record's idle yard bit (`+0x28` `0x800`, `0x400`).
    pub idle: u32,
}

/// Construction yards (modules 1, 2 and 3; `FUN_004db820`, `FUN_004dcc40`,
/// `FUN_004dd120`).
pub(crate) const CONSTRUCTION: YardKind = YardKind {
    family: 0x2a,
    count: 6,
    upkeep: 36,
    class_flags: CLASS_CONSTRUCTION,
    spared: (0x29, 0x40_0000),
    sector_bit: 0x80,
    sector_key: 0x15,
    sector_yards: 0xc4,
    forbid: 0x380_0003,
    system_key: 6,
    few_sites_only: false,
    idle: 0x800,
};

/// Training facilities (module 5; `FUN_004d0d30`, `FUN_004d1970`,
/// `FUN_004d1ed0`).
pub(crate) const TRAINING: YardKind = YardKind {
    family: 0x29,
    count: 5,
    upkeep: 35,
    class_flags: crate::ai_records::CLASS_TRAINING,
    spared: (0x2a, 0x380_0000),
    sector_bit: 0x40,
    sector_key: 0x13,
    sector_yards: 0xbc,
    forbid: 0x40_0003,
    system_key: 5,
    few_sites_only: true,
    idle: 0x400,
};

/// The class record value a class choice ranks on (slot `+4`:
/// `FUN_00473060`, `FUN_00473250`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClassKey {
    /// `values[i]` (`+0x30 + 4i`).
    Value(usize),
    /// Key 1: the maintenance (`+0x20`).
    Maintenance,
}

/// Module 1's state 3 (`FUN_004db820`; module 2's `FUN_004e3a40` on
/// `+0x48`; module 5's `FUN_004d0d30`, sparing construction yards rather
/// than training facilities): scrap what stands in the way at the target.
pub(crate) fn scrap_at(
    io: &mut Inputs<'_>,
    target: Option<SystemKey>,
    kind: YardKind,
) -> Option<AIAction> {
    let target = target?;
    let r = io.galaxy.system(target).copied()?;
    let mut team = Vec::new();
    if r.state & 0x20_0000 == 0 {
        team.extend(io.facilities(target, 0x28));
    }
    let (family, mark) = kind.spared;
    if r.state & mark == 0 {
        team.extend(io.facilities(target, family));
    }
    team.extend(io.facilities(target, 0x25));
    if r.values[15] == 0 && team.is_empty() {
        let t = io.tallies;
        if t.refineries > t.mines || r.values[2] == 0 {
            team.extend(io.facilities(target, 0x2d).into_iter().take(1));
        }
        if team.is_empty() && (t.mines >= t.refineries || r.values[3] == 0) {
            team.extend(io.facilities(target, 0x2c).into_iter().take(1));
        }
    }
    (!team.is_empty()).then_some(AIAction::Scrap { team })
}

/// `FUN_004dd050` (module 2: `FUN_004e4950`): the construction yard
/// maintenance the side wants beyond its sites': `percent` (`+0x24`) of
/// the capacity, 0 when the side's free maintenance falls short of it.
pub(crate) fn wanted(io: &Inputs<'_>, sites: &[SystemKey], percent: i32) -> i32 {
    let held: i32 = sites
        .iter()
        .filter_map(|&s| io.galaxy.system(s))
        .map(|r| r.values[36])
        .sum();
    let t = io.tallies;
    let wanted = percent * t.capacity / 100 - held;
    if t.capacity - t.requirement < wanted {
        0
    } else {
        wanted
    }
}

/// `FUN_0041b230(&class, 0x2a, 0x10000, 0x4000, 2)` → `FUN_0042e3c0`: the
/// side's buildable construction yard class with the lowest processing
/// rate (class record `+0x30`, `FUN_00473060`), and its maintenance
/// (`+0x20`), 0 with none.
pub(crate) fn yard_class(io: &mut Inputs<'_>, kind: YardKind) -> (Option<DatId>, i32) {
    best_class(
        io,
        ProductionArea::ConstructionYard,
        kind.class_flags,
        ClassKey::Value(0),
        Order::Descending,
    )
}

/// `FUN_0041b230(&class, area, flags, key, order)` → `FUN_0042e3c0`: the
/// classes `area`'s manager lists for the side (`FUN_0052e580`: `0x2a` a
/// construction yard, `0x29` a training facility) whose class record has
/// `flags`, ranked on the record value the key selects in `order`, and
/// picked (`FUN_004357b0`); with the class's maintenance (`+0x20`), 0 with
/// none.
pub(crate) fn best_class(
    io: &mut Inputs<'_>,
    area: ProductionArea,
    flags: u32,
    key: ClassKey,
    order: Order,
) -> (Option<DatId>, i32) {
    let classes: Vec<_> =
        crate::build_selection::listed_classes(io.world, io.research, area, io.is_alliance)
            .into_iter()
            .filter_map(BuildableKind::class_dat_id)
            .filter_map(|c| {
                let r = io.galaxy.classes.get(c)?;
                let value = match key {
                    ClassKey::Value(i) => r.values[i],
                    ClassKey::Maintenance => r.maintenance,
                };
                (r.flags & flags == flags).then_some((c, value))
            })
            .collect();
    let list = crate::ai_records::ranked(classes, order, io.coin);
    let class = crate::ai_records::pick(&[list]);
    let maintenance = class
        .and_then(|c| io.galaxy.classes.get(c))
        .map_or(0, |r| r.maintenance);
    (class, maintenance)
}

/// Module 1's state 5 (`FUN_004dbe70`; module 2's `FUN_004e4020`): leave
/// a site with too few yards, or scrap a construction yard at one with two
/// or more. The site chosen becomes the target.
pub(crate) fn scrap_spare_yard(
    io: &mut Inputs<'_>,
    sites: &mut Vec<SystemKey>,
    target: &mut Target,
    mark: u32,
    kind: YardKind,
) -> Option<AIAction> {
    let mut chosen = None;
    let mut i = 0;
    while i < sites.len() {
        let site = sites[i];
        let Some(r) = io.galaxy.system(site).copied() else {
            i += 1;
            continue;
        };
        let count = sites.len();
        if r.values[kind.count] < 1 {
            sites.remove(i);
            io.set_mark(site, 0, mark);
        } else if r.state & 0x1000 == 0 {
            i += 1;
        } else if r.values[kind.count] < 2 {
            if count > 1 {
                *target = Target::System(site);
                sites.remove(i);
                io.set_mark(site, 0, mark);
                chosen = Some(site);
            }
            break;
        } else {
            *target = Target::System(site);
            chosen = Some(site);
            break;
        }
    }
    let site = chosen?;
    let yard = io.facilities(site, kind.family).into_iter().next()?;
    Some(AIAction::Scrap { team: vec![yard] })
}

/// `FUN_00419b80` then `FUN_0041a400` from the yard, order 2: the nearest
/// sector with `sector_bit` (`+0x24`), then its nearest system whose
/// `+0x28`, `+0x2c` and `+0x30` pass `system`.
pub(crate) fn nearest(
    io: &mut Inputs<'_>,
    yard: Yard,
    sector_bit: u32,
    system: [Mask; 3],
) -> Target {
    let origin = match yard {
        Yard::System(s) | Yard::Manager(s) => s,
        Yard::None => return Target::None,
    };
    let sectors = io.galaxy.sectors_by_distance(
        origin,
        [Mask::new(sector_bit, 0), ANY, ANY],
        Order::Descending,
        io.coin,
    );
    let Some(sector) = crate::ai_records::pick(&[sectors]) else {
        return Target::None;
    };
    let systems = io
        .galaxy
        .systems_by_distance(sector, origin, system, Order::Descending, io.coin);
    crate::ai_records::pick(&[systems]).map_or(Target::Sector(sector), Target::System)
}

/// Module 1's state 8 (`FUN_004dc9f0`; module 2's `FUN_004e4300`):
/// Destination on the yard's manager, to the target system.
pub(crate) fn destination(target: Target, yard: Yard) -> Option<AIAction> {
    destination_in(ProductionArea::ConstructionYard, target, yard)
}

/// Destination on the yard's `area` manager, to the target system
/// (module 6's `FUN_004e0a30` on the troop manager).
pub(crate) fn destination_in(area: ProductionArea, target: Target, yard: Yard) -> Option<AIAction> {
    let target = target.system()?;
    let Yard::Manager(yard) = yard else {
        return None;
    };
    Some(AIAction::SetDestination {
        system: yard,
        area,
        destination: target,
    })
}

/// Module 1's state 9 (`FUN_004dcb30`; module 2's `FUN_004e4440`): Build
/// `count` of the class on the yard's manager.
pub(crate) fn build(
    io: &Inputs<'_>,
    yard: Yard,
    class: Option<DatId>,
    count: i32,
) -> Option<AIAction> {
    let Yard::Manager(yard) = yard else {
        return None;
    };
    let class = class?;
    let build = FacilityBuild {
        class,
        is_alliance: io.is_alliance,
    };
    let kind = match class.family() {
        0x2c | 0x2d => BuildableKind::ProductionFacility(build),
        0x22..=0x27 => BuildableKind::DefenseFacility(build),
        0x10..=0x13 => BuildableKind::Troop(class),
        0x3c..=0x3f => BuildableKind::SpecialForce(class),
        _ => BuildableKind::ManufacturingFacility(build),
    };
    let cost = io
        .world
        .buildable_classes
        .get(&class)
        .map_or(0, |c| c.refined_material_cost);
    Some(AIAction::Build {
        system: yard,
        kind,
        cost,
        count,
    })
}

/// Module 1's state 10 (`FUN_004dcc40`; module 2's `FUN_004e4540`): adopt
/// a system with construction yards as a site (`mark`), or ask for the
/// yards (`token`), writing the yard class into `class` (`+0x48`, module
/// 2's `+0x50`). `wanted` is the module's yard budget (`FUN_004dd050`,
/// `FUN_004e4950`, `FUN_004da530`). Returns the new target.
pub(crate) fn adopt_or_request(
    io: &mut Inputs<'_>,
    sites: &mut Vec<SystemKey>,
    class: &mut Option<DatId>,
    mark: u32,
    token: u32,
    wanted: i32,
    kind: YardKind,
) -> Target {
    if kind.few_sites_only && sites.len() >= 2 {
        return Target::None;
    }
    let mut found = None;
    if wanted > 0 {
        let bit = [Mask::new(kind.sector_bit, 0), ANY, ANY];
        let sectors = io.sectors(bit, kind.sector_key, Order::Ascending);
        'sectors: for sector in picks(vec![sectors]) {
            let yards = io
                .galaxy
                .sector(sector)
                .map_or(0, |s| s.values[sector_at(kind.sector_yards)]);
            if yards <= 0 {
                break;
            }
            let masks = [Mask::new(0, kind.forbid), ANY, Mask::new(1, 0x4000_0000)];
            let systems = io.systems(Some(sector), masks, kind.system_key, Order::Ascending);
            for system in picks(vec![systems]) {
                if io
                    .galaxy
                    .system(system)
                    .is_some_and(|r| r.values[kind.count] > 0 && r.values[kind.upkeep] <= wanted)
                {
                    found = Some(system);
                    break 'sectors;
                }
            }
        }
    }
    if let Some(system) = found {
        sites.push(system);
        io.set_mark(system, mark, 0);
        return Target::System(system);
    }
    if sites.len() < 2 {
        let has_yard = sites
            .first()
            .and_then(|&s| io.galaxy.system(s))
            .is_some_and(|r| r.values[kind.count] > 0);
        if !has_yard {
            let maintenance;
            (*class, maintenance) = yard_class(io, kind);
            if maintenance <= wanted {
                io.controller.yard_requests |= token;
            }
        }
    }
    Target::None
}

/// `FUN_004dd120` (module 2: `FUN_004e4a20`; module 5: `FUN_004d1ed0`,
/// by training facilities): claim the site with the most yards of `kind`
/// and active regiments that no other module claims.
pub(crate) fn claim(
    io: &mut Inputs<'_>,
    sites: &[SystemKey],
    claimed: &mut Vec<SystemKey>,
    kind: YardKind,
) {
    let mut best = 0;
    let mut chosen = None;
    for &site in sites {
        let Some(r) = io.galaxy.system(site) else {
            continue;
        };
        if r.values[kind.count] <= best {
            continue;
        }
        if r.kind & CLAIMED != 0 && !claimed.contains(&site) {
            continue;
        }
        if r.values[18] > 0 {
            best = r.values[kind.count];
            chosen = Some(site);
        }
    }
    for site in std::mem::take(claimed) {
        io.set_kind(site, 0, CLAIMED);
    }
    if let Some(site) = chosen {
        io.set_kind(site, CLAIMED, 0);
        claimed.push(site);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ai::AIState;
    use crate::ai_records::KEPT_STATE as KEPT;
    use crate::ai_records::{ClassRecords, SystemInputs, SHIELD_GOAL};
    use crate::world::{BuildableClass, ManufacturingFacilityInstance, Sector, System};

    /// Module 1's view of the shared steps, as the tests name them.
    impl Module1 {
        fn scrap_at_target(&mut self, io: &mut Inputs<'_>) -> Option<AIAction> {
            scrap_at(io, self.target.system(), CONSTRUCTION)
        }

        fn wanted(&self, io: &Inputs<'_>) -> i32 {
            wanted(io, &self.sites, PERCENT)
        }

        fn yard_class(&mut self, io: &mut Inputs<'_>) -> i32 {
            let maintenance;
            (self.class, maintenance) = yard_class(io, CONSTRUCTION);
            maintenance
        }

        fn scrap_spare_yard(&mut self, io: &mut Inputs<'_>) -> Option<AIAction> {
            scrap_spare_yard(
                io,
                &mut self.sites,
                &mut self.target,
                MARK_MINES,
                CONSTRUCTION,
            )
        }

        fn build(&self, io: &Inputs<'_>) -> Option<AIAction> {
            build(io, self.yard, self.class, self.count)
        }

        fn adopt_or_request(&mut self, io: &mut Inputs<'_>) {
            let budget = wanted(io, &self.sites, PERCENT);
            self.target = adopt_or_request(
                io,
                &mut self.sites,
                &mut self.class,
                MARK_MINES,
                YARD_TOKEN,
                budget,
                CONSTRUCTION,
            );
        }

        fn claim(&mut self, io: &mut Inputs<'_>) {
            claim(io, &self.sites, &mut self.claimed, CONSTRUCTION);
        }
    }

    pub(crate) struct Fixture {
        pub(crate) world: GameWorld,
        pub(crate) sector: SectorKey,
        /// An Alliance system with an idle construction yard.
        pub(crate) yard: SystemKey,
        /// An Alliance system with nothing built.
        pub(crate) empty: SystemKey,
    }

    pub(crate) fn system(world: &mut GameWorld, sector: SectorKey, x: u16) -> SystemKey {
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
            total_energy: 10,
            raw_materials: 5,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            is_headquarters: false,
            is_destroyed: false,
            control: crate::world::ControlKind::Controlled(Faction::Alliance),
        })
    }

    pub(crate) fn class(
        world: &mut GameWorld,
        raw: u32,
        maintenance: u32,
        rate: u32,
        order: u32,
    ) -> DatId {
        let id = DatId::new(raw);
        world.buildable_classes.insert(
            id,
            BuildableClass {
                is_alliance: true,
                is_empire: true,
                maintenance_cost: maintenance,
                refined_material_cost: 10,
                processing_rate: rate,
                research_order: order,
                ..BuildableClass::default()
            },
        );
        id
    }

    pub(crate) fn add_yard(world: &mut GameWorld, at: SystemKey, class: DatId, side: Faction) {
        let key = world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: class,
                side,
                is_shipyard: class.family() == 0x28,
            });
        world.systems[at].manufacturing_facilities.push(key);
    }

    pub(crate) fn fixture() -> Fixture {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(0x8000_0001),
            name: "Sector".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let yard = system(&mut world, sector, 0);
        let empty = system(&mut world, sector, 5);
        let construction = class(&mut world, 0x2a00_0003, 10, 3, 0);
        class(&mut world, MINE_CLASS, 0, 0, 0);
        class(&mut world, REFINERY_CLASS, 0, 0, 0);
        add_yard(&mut world, yard, construction, Faction::Alliance);
        Fixture {
            world,
            sector,
            yard,
            empty,
        }
    }

    pub(crate) fn records_of(f: &Fixture, state: &AIState) -> Galaxy {
        let classes = ClassRecords::build(&f.world);
        let fleets = HashMap::new();
        let economy = crate::economy::EconomyState::default();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = crate::manufacturing::ManufacturingState::new();
        let research = ResearchState::new();
        let inputs = SystemInputs {
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
            shield_goal: SHIELD_GOAL,
        };
        Galaxy::build(
            &inputs,
            &crate::movement::MovementState::new(),
            &crate::troop_transport::TroopTransportState::new(),
            &state.yard_marks,
            &state.record_kinds,
        )
    }

    /// One step of the side's module 1 over freshly built records.
    fn step(f: &Fixture, state: &mut AIState, tallies: Tallies) -> Step {
        let mut galaxy = records_of(f, state);
        let research = ResearchState::new();
        let mut coin = || false;
        let mut io = Inputs {
            world: &f.world,
            research: &research,
            galaxy: &mut galaxy,
            controller: &mut state.controller,
            marks: &mut state.yard_marks,
            kinds: &mut state.record_kinds,
            tallies,
            is_alliance: true,
            coin: &mut coin,
        };
        state.module1.step(&mut io)
    }

    /// Capacity 1000, requirement 0: 4% of it is 40, so 30 beyond the
    /// yard's own 10 (`FUN_004dd050`).
    pub(crate) const ROOMY: Tallies = Tallies {
        mines: 0,
        refineries: 0,
        capacity: 1000,
        requirement: 0,
        defenses: 0,
        special_upkeep: 0,
        special_goal: 0,
        special_ready: [0; 4],
        regiment_upkeep: 0,
        cargo_upkeep: 0,
        sector_short: false,
    };

    #[test]
    fn the_yards_pass_to_each_module_in_turn_or_to_the_first_that_asks() {
        // FUN_004199b0 from FUN_004178f0's 0x80/0x40.
        let mut c = Controller::default();
        let mut holders = Vec::new();
        for _ in 0..9 {
            c.pass_yards();
            holders.push(c.yard_holder);
        }
        assert_eq!(holders, [0x40, 0x20, 0x10, 8, 4, 2, 1, 0x80, 0x40]);
        // A request is granted from the holder rightward, wrapping.
        c.yard_requests = 0x80 | 2;
        c.pass_yards();
        assert_eq!((c.yard_holder, c.yard_requests), (2, 0x80));
        c.pass_yards();
        assert_eq!((c.yard_holder, c.yard_requests), (0x80, 0));
    }

    #[test]
    fn the_first_step_marks_a_site_with_an_idle_yard_and_starts_a_cycle() {
        // FUN_004daf10: the sector with an idle construction yard
        // (+0x24 0x2000), its system with one (+0x28 0x800), marked
        // 0x800000 and kept at +0x58; the manager goes to +0x44 and v[6]
        // to +0x54. The step does not complete (FUN_004dabe0).
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);

        let s = step(&f, &mut state, ROOMY);

        assert!(!s.complete && s.order.is_none());
        let m = &state.module1;
        assert_eq!((m.phase, m.state), (CYCLE, 0));
        assert_eq!(m.sites, vec![f.yard]);
        assert_eq!(m.yard, Yard::Manager(f.yard));
        assert_eq!(m.capacity, 1);
        assert_eq!(state.yard_marks.get(&f.yard), Some(&MARK_MINES));
    }

    #[test]
    fn with_maintenance_to_spare_the_cycle_orders_more_yards_at_the_site() {
        // FUN_004db2f0: states 0 -> 1 (the yards are module 1's; passed on)
        // -> 2 (target: the site, room v[0x33]) -> 3 (nothing to scrap)
        // -> 4 (30 / 10 = 3, capped by +0x54 = 1: state 8) -> 8
        // (Destination, complete) -> 9 (Build 1 of 0x2a000003).
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let mut steps = Vec::new();
        for _ in 0..8 {
            let s = step(&f, &mut state, ROOMY);
            steps.push((state.module1.state, s.complete, s.order.is_some()));
            if s.order.is_some() {
                break;
            }
        }
        assert_eq!(
            steps,
            [
                (0, false, false),
                (1, false, false),
                (2, false, false),
                (3, false, false),
                (4, false, false),
                (8, false, false),
                (9, true, true),
            ]
        );
        assert_eq!(state.controller.yard_holder, 0x40);
        assert_eq!(state.module1.target, Target::System(f.yard));

        let s = step(&f, &mut state, ROOMY);

        assert_eq!(
            s.order,
            Some(AIAction::Build {
                system: f.yard,
                kind: BuildableKind::ManufacturingFacility(FacilityBuild {
                    class: DatId::new(0x2a00_0003),
                    is_alliance: true,
                }),
                cost: 10,
                count: 1,
            })
        );
        assert!(s.complete);
        assert_eq!((state.module1.phase, state.module1.state), (0, 0));
    }

    #[test]
    fn the_destination_order_sends_the_yard_s_work_to_the_target() {
        // FUN_004dc9f0: 0x214 on the manager at +0x44, target +0x40.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let order = (0..8)
            .find_map(|_| step(&f, &mut state, ROOMY).order)
            .unwrap();
        assert_eq!(
            order,
            AIAction::SetDestination {
                system: f.yard,
                area: ProductionArea::ConstructionYard,
                destination: f.yard,
            }
        );
    }

    #[test]
    fn state_four_never_reaches_state_five() {
        // FUN_004dbd60 stores 5 for a surplus (w < 0) but falls into the
        // +0x4c == 0 test and stores 6 (0x4dbe48 -> 0x4dbe50).
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let short = Tallies {
            capacity: 100,
            ..ROOMY
        };
        for _ in 0..5 {
            step(&f, &mut state, short);
        }
        assert_eq!(state.module1.state, 4);
        // 4 * 100 / 100 - 10 = -6.
        step(&f, &mut state, short);
        assert_eq!(state.module1.state, 6);
    }

    #[test]
    fn state_six_takes_the_refinery_when_refineries_trail_mines() {
        // FUN_004db2f0 state 6: the refinery 0x2d000002 when +0x1d4 <
        // +0x1d0 (FUN_004dc3f0 finds the empty system, +0x28 0x100), one of
        // it (+0x4c = 1), then state 7 runs at once and leaves 8.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let tallies = Tallies {
            mines: 2,
            refineries: 1,
            capacity: 100,
            requirement: 0,
            ..ROOMY
        };
        for _ in 0..6 {
            step(&f, &mut state, tallies);
        }
        assert_eq!(state.module1.state, 6);

        let s = step(&f, &mut state, tallies);

        let m = &state.module1;
        assert_eq!(m.class, Some(DatId::new(REFINERY_CLASS)));
        assert_eq!((m.phase, m.state, m.count), (CYCLE, 8, 1));
        assert_eq!(m.target, Target::System(f.empty));
        assert!(!s.complete && s.order.is_none());
    }

    #[test]
    fn a_failed_mine_site_still_runs_state_seven() {
        // FUN_004db2f0 state 6 with no room anywhere (no free energy or raw)
        // and no yard to measure from: the mine (0 >= 0) finds no site, so
        // +0x3c = +0x38 = 0, yet state 7 runs at once and leaves +0x3c = 8
        // (0x4db448 -> 0x4db44e).
        let mut f = fixture();
        for sys in [f.yard, f.empty] {
            f.world.systems[sys].total_energy = 0;
            f.world.systems[sys].raw_materials = 0;
        }
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.phase = CYCLE;
        state.module1.state = 6;

        let s = step(&f, &mut state, Tallies::default());

        let m = &state.module1;
        assert_eq!(m.class, Some(DatId::new(MINE_CLASS)));
        assert_eq!((m.phase, m.state), (0, 8));
        assert_eq!(m.target, Target::None);
        assert!(!s.complete && s.order.is_none());
    }

    #[test]
    fn state_three_scraps_the_unmarked_shipyards_at_the_target() {
        // FUN_004db820: every 0x28 and 0x29 yard unless the target is
        // marked for it, and every 0x25 defense, in one Scrap order.
        let mut f = fixture();
        let shipyard = class(&mut f.world, 0x2800_0001, 13, 3, 0);
        add_yard(&mut f.world, f.yard, shipyard, Faction::Alliance);
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let order = (0..6)
            .find_map(|_| step(&f, &mut state, ROOMY).order)
            .unwrap();
        let key = f.world.systems[f.yard].manufacturing_facilities[1];
        assert_eq!(
            order,
            AIAction::Scrap {
                team: vec![ScrapTarget::Facility(FacilityRef::Manufacturing(key))]
            }
        );
        assert_eq!(state.module1.state, 4);
    }

    #[test]
    fn with_no_yard_the_fallback_marks_an_own_system() {
        // FUN_004daf10: no sector with an idle construction yard, so sector
        // +0x2c bit 4 (key 0x14) and an own system by v[6]: marked, kept,
        // but without an idle yard the search fails and, with the yards
        // module 1's and +4 0x40 clear, FUN_004dcc40 does not run.
        let mut f = fixture();
        let yard = f.world.systems[f.yard]
            .manufacturing_facilities
            .pop()
            .unwrap();
        f.world.manufacturing_facilities.remove(yard);
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);

        let s = step(&f, &mut state, ROOMY);

        assert!(s.complete && s.order.is_none());
        assert_eq!(state.module1.sites.len(), 1);
        assert_eq!(
            (state.controller.flags, state.controller.yard_requests),
            (0, 0)
        );
    }

    #[test]
    fn without_the_yards_and_a_yard_to_adopt_module_one_asks_for_them() {
        // FUN_004dcc40 when another module holds the yards (+0xc 0x40):
        // w = 40 > 0 but no sector has unmarked construction yards
        // (+0xc4), the only site has none (v[6] = 0) and the yard class
        // costs 10 <= 40: controller +8 |= 0x80.
        let mut f = fixture();
        let yard = f.world.systems[f.yard]
            .manufacturing_facilities
            .pop()
            .unwrap();
        f.world.manufacturing_facilities.remove(yard);
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.controller.yard_holder = 0x40;

        let s = step(&f, &mut state, ROOMY);

        assert!(s.complete && s.order.is_none());
        assert_eq!(state.controller.yard_requests, YARD_TOKEN);
        assert_eq!(state.module1.class, Some(DatId::new(0x2a00_0003)));
        let _ = f.sector;
    }

    #[test]
    fn a_site_s_unclaimed_yards_with_regiments_are_claimed() {
        // FUN_004dd120: the site with the most construction yards and
        // active regiments (v[18] > 0) gets +0x30 0x10000000.
        let mut f = fixture();
        let regiment = class(&mut f.world, 0x1000_0001, 1, 0, 0);
        let troop = f.world.troops.insert(crate::world::TroopUnit {
            class_dat_id: regiment,
            is_alliance: true,
            regiment_strength: 100,
        });
        f.world.systems[f.yard].ground_units.push(troop);
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.sites = vec![f.yard];
        state.set_yard_marks(f.yard, MARK_MINES);
        // A busy yard: no site is kept, so the step claims and completes.
        state.module1.sites.push(f.empty);
        let mut galaxy = records_of(&f, &state);
        galaxy.update_state(f.yard, 0, 0x800);
        let research = ResearchState::new();
        let mut coin = || false;
        let mut io = Inputs {
            world: &f.world,
            research: &research,
            galaxy: &mut galaxy,
            controller: &mut state.controller,
            marks: &mut state.yard_marks,
            kinds: &mut state.record_kinds,
            tallies: ROOMY,
            is_alliance: true,
            coin: &mut coin,
        };
        state.module1.claim(&mut io);

        assert_eq!(state.module1.claimed, vec![f.yard]);
        assert_eq!(state.record_kinds.get(&f.yard), Some(&CLAIMED));
        assert_eq!(galaxy.system(f.yard).unwrap().kind & CLAIMED, CLAIMED);
    }

    /// Run `run` on module 1 with `galaxy`.
    fn with_io<R>(
        f: &Fixture,
        state: &mut AIState,
        galaxy: &mut Galaxy,
        tallies: Tallies,
        run: impl FnOnce(&mut Module1, &mut Inputs<'_>) -> R,
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
        run(&mut state.module1, &mut io)
    }

    pub(crate) fn record(
        galaxy: &mut Galaxy,
        system: SystemKey,
    ) -> &mut crate::ai_records::SystemRecord {
        galaxy
            .sectors
            .iter_mut()
            .flat_map(|s| &mut s.systems)
            .find(|r| r.system == system)
            .unwrap()
    }

    #[test]
    fn a_kept_site_drops_lost_and_bare_sites_and_takes_the_first_calm_idle_one() {
        // FUN_004daca0 over +0x58: a site without a record leaves; one not
        // the side's or without its mark drops the mark and leaves; a busy
        // one stays unless it is the only site and has no yard; the first
        // calm one with an idle yard is taken (+0x54 = v[6], +0x44).
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let mut nowhere = slotmap::SlotMap::<SystemKey, ()>::with_key();
        let missing = (0..9).map(|_| nowhere.insert(())).last().unwrap();
        state.module1.sites = vec![missing, f.empty, f.yard];
        state.set_yard_marks(f.yard, MARK_MINES | 0x20_0000);
        let mut galaxy = records_of(&f, &state);
        // The empty system is the side's but unmarked.
        assert!(with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.keep_site(io)));
        assert_eq!(state.module1.sites, vec![f.yard]);
        assert_eq!(state.module1.yard, Yard::Manager(f.yard));
        assert_eq!(state.module1.capacity, 1);
        // Both kept marks survive; only a dropped site loses 0x800000.
        assert_eq!(
            state.yard_marks.get(&f.yard),
            Some(&(MARK_MINES | 0x20_0000))
        );

        // Not the side's: the mark goes, other marks stay.
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.yard).kind &= !1;
        state.module1.yard = Yard::None;
        assert!(!with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.keep_site(io)));
        assert!(state.module1.sites.is_empty());
        assert_eq!(state.yard_marks.get(&f.yard), Some(&0x20_0000));
        assert_eq!(record(&mut galaxy, f.yard).state & MARK_MINES, 0);
    }

    #[test]
    fn a_busy_or_unsettled_site_is_kept_in_the_list_but_not_taken() {
        // FUN_004daca0: no idle yard (0x800) keeps the site unless it is
        // the only one with no yard; an idle yard in an uprising or blockade
        // (+0x28 & 3) is passed over.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.sites = vec![f.yard, f.empty];
        state.set_yard_marks(f.yard, MARK_MINES);
        state.set_yard_marks(f.empty, MARK_MINES);
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.yard).state |= 2;
        assert!(!with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.keep_site(io)));
        assert_eq!(state.module1.sites, vec![f.yard, f.empty]);
        assert_eq!(state.module1.capacity, 0);

        // Alone, busy and bare: it goes.
        state.module1.sites = vec![f.empty];
        let mut galaxy = records_of(&f, &state);
        assert!(!with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.keep_site(io)));
        assert!(state.module1.sites.is_empty());
        assert!(!state.yard_marks.contains_key(&f.empty));

        // Alone, busy, with a yard: it stays.
        state.module1.sites = vec![f.yard];
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.yard).state &= !0x800;
        assert!(!with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.keep_site(io)));
        assert_eq!(state.module1.sites, vec![f.yard]);
    }

    #[test]
    fn the_target_is_the_calm_own_site_with_room_and_the_fewest_yards() {
        // FUN_004db510: own (+0x30 1), calm, v[0x33] > 0, v[6] below the
        // best so far (from 1000, strictly).
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.sites = vec![f.yard, f.empty];
        let mut galaxy = records_of(&f, &state);
        for (sys, yards, room) in [(f.yard, 2, 5), (f.empty, 2, 7)] {
            let r = record(&mut galaxy, sys);
            r.values[6] = yards;
            r.values[0x33] = room;
        }
        assert!(with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m
            .choose_target(io)));
        assert_eq!(
            (state.module1.target, state.module1.room),
            (Target::System(f.yard), 5)
        );

        record(&mut galaxy, f.empty).values[6] = 1;
        with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.choose_target(io)
        });
        assert_eq!(state.module1.target, Target::System(f.empty));

        for (bits, value) in [(2, 7), (0, 0)] {
            let r = record(&mut galaxy, f.empty);
            r.state |= bits;
            r.values[0x33] = value;
            with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
                m.choose_target(io)
            });
            assert_eq!(state.module1.target, Target::System(f.yard), "{bits}");
            let r = record(&mut galaxy, f.empty);
            r.state &= !2;
            r.values[0x33] = 7;
        }
        record(&mut galaxy, f.empty).kind &= !1;
        with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.choose_target(io)
        });
        assert_eq!(state.module1.target, Target::System(f.yard));
    }

    #[test]
    fn state_three_spares_marked_yards_and_takes_one_mine_or_refinery_when_full() {
        // FUN_004db820: no 0x28 with a ship mark, no 0x29 with a troop mark;
        // with no free energy (v[15] == 0) and nothing else, one refinery
        // when +0x1d4 > +0x1d0 or no mine there (v[2] == 0), else one mine
        // when +0x1d0 >= +0x1d4 or no refinery (v[3] == 0).
        let mut f = fixture();
        let shipyard = class(&mut f.world, 0x2800_0001, 13, 3, 0);
        let training = class(&mut f.world, 0x2900_0002, 10, 3, 0);
        add_yard(&mut f.world, f.yard, shipyard, Faction::Alliance);
        add_yard(&mut f.world, f.yard, training, Faction::Alliance);
        let gun = class(&mut f.world, 0x2500_0001, 1, 0, 0);
        let gun = f
            .world
            .defense_facilities
            .insert(crate::world::DefenseFacilityInstance {
                class_dat_id: gun,
                side: Faction::Alliance,
            });
        f.world.systems[f.yard].defense_facilities.push(gun);
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.target = Target::System(f.yard);
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.yard).state |= 0x20_0000 | 0x40_0000;
        let order = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.scrap_at_target(io)
        });
        assert_eq!(
            order,
            Some(AIAction::Scrap {
                team: vec![ScrapTarget::Facility(FacilityRef::Defense(gun))]
            })
        );

        // Full, nothing else: a refinery or a mine by the balance.
        f.world.systems[f.yard].defense_facilities.clear();
        let mut put = |raw: u32, is_mine: bool| {
            let class_dat_id = DatId::new(raw);
            let key =
                f.world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id,
                        side: Faction::Alliance,
                        is_mine,
                    });
            f.world.systems[f.yard].production_facilities.push(key);
            ScrapTarget::Facility(FacilityRef::Production(key))
        };
        let mine = put(MINE_CLASS, true);
        put(MINE_CLASS, true);
        let refinery = put(REFINERY_CLASS, false);
        put(REFINERY_CLASS, false);
        let cases = [
            ((1, 2), (1, 1), Some(refinery)),
            ((2, 1), (0, 1), Some(refinery)),
            ((2, 2), (1, 1), Some(mine)),
            ((1, 2), (1, 0), Some(refinery)),
            ((1, 2), (0, 0), Some(refinery)),
            ((2, 1), (1, 0), Some(mine)),
            ((2, 1), (1, 1), Some(mine)),
            ((1, 1), (1, 1), Some(mine)),
        ];
        for ((mines, refineries), (v2, v3), expected) in cases {
            let tallies = Tallies {
                mines,
                refineries,
                ..ROOMY
            };
            let mut galaxy = records_of(&f, &state);
            let r = record(&mut galaxy, f.yard);
            r.state |= 0x20_0000 | 0x40_0000;
            r.values[15] = 0;
            r.values[2] = v2;
            r.values[3] = v3;
            let order = with_io(&f, &mut state, &mut galaxy, tallies, |m, io| {
                m.scrap_at_target(io)
            });
            let case = (mines, refineries, v2, v3);
            assert_eq!(
                order,
                expected.map(|t| AIAction::Scrap { team: vec![t] }),
                "{case:?}"
            );
        }
        // No refinery there: the mine needs +0x1d0 >= +0x1d4 or v[3] == 0.
        let refineries: Vec<_> = f.world.systems[f.yard]
            .production_facilities
            .iter()
            .copied()
            .filter(|&k| !f.world.production_facilities[k].is_mine)
            .collect();
        f.world.systems[f.yard]
            .production_facilities
            .retain(|k| !refineries.contains(k));
        for (v3, expected) in [(0, Some(mine)), (1, None)] {
            let tallies = Tallies {
                mines: 1,
                refineries: 2,
                ..ROOMY
            };
            let mut galaxy = records_of(&f, &state);
            let r = record(&mut galaxy, f.yard);
            r.state |= 0x20_0000 | 0x40_0000;
            r.values[15] = 0;
            r.values[2] = 1;
            r.values[3] = v3;
            let order = with_io(&f, &mut state, &mut galaxy, tallies, |m, io| {
                m.scrap_at_target(io)
            });
            assert_eq!(
                order,
                expected.map(|t| AIAction::Scrap { team: vec![t] }),
                "v[3] {v3}"
            );
        }
        // With free energy nothing is scrapped.
        let mut galaxy = records_of(&f, &state);
        let r = record(&mut galaxy, f.yard);
        r.state |= 0x20_0000 | 0x40_0000;
        r.values[15] = 1;
        assert_eq!(
            with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m
                .scrap_at_target(io)),
            None
        );
    }

    #[test]
    fn the_wanted_yard_upkeep_is_zero_when_the_side_cannot_afford_it() {
        // FUN_004dd050: 4 * capacity / 100 less the sites' v[36]; 0 when
        // capacity - requirement falls below it.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.sites = vec![f.yard];
        let mut galaxy = records_of(&f, &state);
        let wanted = |state: &mut AIState, galaxy: &mut Galaxy, requirement| {
            let t = Tallies {
                requirement,
                ..ROOMY
            };
            with_io(&f, state, galaxy, t, |m, io| m.wanted(io))
        };
        assert_eq!(wanted(&mut state, &mut galaxy, 0), 30);
        assert_eq!(wanted(&mut state, &mut galaxy, 970), 30);
        assert_eq!(wanted(&mut state, &mut galaxy, 971), 0);
    }

    #[test]
    fn a_small_yard_budget_builds_no_yard_and_goes_to_state_six() {
        // FUN_004dbd60: w = 5 > 0 but below the yard's maintenance 10, so
        // +0x4c = 0 and the result is 6; with no target system it is 0.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.sites = vec![f.yard];
        state.module1.target = Target::System(f.yard);
        state.module1.room = 9;
        state.module1.capacity = 9;
        let mut galaxy = records_of(&f, &state);
        let t = Tallies {
            capacity: 375,
            ..ROOMY
        };
        assert_eq!(
            with_io(&f, &mut state, &mut galaxy, t, |m, io| m.plan_yards(io)),
            6
        );
        assert_eq!(state.module1.count, 0);
        // w = 4 * 1000 / 100 - 10 = 30: 30 / 10 = 3, capped by v[15] = 10,
        // room 9 and capacity 9 to 3.
        assert_eq!(
            with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.plan_yards(io)),
            8
        );
        assert_eq!(state.module1.count, 3);
        state.module1.target = Target::None;
        assert_eq!(
            with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.plan_yards(io)),
            0
        );
    }

    #[test]
    fn the_fastest_buildable_yard_class_is_chosen() {
        // FUN_0042e3c0 with key 0x4000 (FUN_00473060: +0x30, the
        // processing rate) in order 2: the lowest rate; the advanced yard
        // needs research order 2, so it is not listed yet (FUN_0052e580).
        let mut f = fixture();
        class(&mut f.world, 0x2a00_0006, 12, 1, 2);
        class(&mut f.world, 0x2a00_0007, 14, 2, 0);
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let mut galaxy = records_of(&f, &state);
        let m = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.yard_class(io));
        assert_eq!(
            (state.module1.class, m),
            (Some(DatId::new(0x2a00_0007)), 14)
        );
    }

    #[test]
    fn state_five_leaves_a_thin_site_and_scraps_a_spare_yard() {
        // FUN_004dbe70 over +0x58: no yard leaves (mark dropped); no active
        // yard (0x1000) is passed over; one yard stops the walk, leaving
        // (and scrapping) only when more sites remain; two or more scrap the
        // first construction yard.
        let mut f = fixture();
        let construction = DatId::new(0x2a00_0003);
        add_yard(&mut f.world, f.yard, construction, Faction::Alliance);
        let first = ScrapTarget::Facility(FacilityRef::Manufacturing(
            f.world.systems[f.yard].manufacturing_facilities[0],
        ));
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.sites = vec![f.empty, f.yard];
        state.set_yard_marks(f.empty, MARK_MINES);
        let mut galaxy = records_of(&f, &state);
        let order = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.scrap_spare_yard(io)
        });
        assert_eq!(order, Some(AIAction::Scrap { team: vec![first] }));
        assert_eq!(state.module1.sites, vec![f.yard]);
        assert!(!state.yard_marks.contains_key(&f.empty));
        assert_eq!(state.module1.target, Target::System(f.yard));

        // One yard, other sites left: the site goes and its yard too.
        f.world.systems[f.yard].manufacturing_facilities.truncate(1);
        state.module1.sites = vec![f.yard, f.empty];
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.empty).values[6] = 1;
        let order = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.scrap_spare_yard(io)
        });
        assert_eq!(order, Some(AIAction::Scrap { team: vec![first] }));
        assert_eq!(state.module1.sites, vec![f.empty]);

        // One yard, alone: it stays and nothing is scrapped.
        state.module1.sites = vec![f.yard];
        let mut galaxy = records_of(&f, &state);
        assert_eq!(
            with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m
                .scrap_spare_yard(io)),
            None
        );
        assert_eq!(state.module1.sites, vec![f.yard]);

        // No active yard: passed over.
        let mut galaxy = records_of(&f, &state);
        let r = record(&mut galaxy, f.yard);
        r.state &= !0x1000;
        r.values[6] = 3;
        assert_eq!(
            with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m
                .scrap_spare_yard(io)),
            None
        );
    }

    #[test]
    fn state_seven_clears_an_unmarked_target_of_its_yards() {
        // FUN_004dc650: every 0x28, 0x29 and 0x2a yard at an unmarked
        // (+0x28 & 0x3e00000 == 0) target; a marked one is left.
        let mut f = fixture();
        let shipyard = class(&mut f.world, 0x2800_0001, 13, 3, 0);
        add_yard(&mut f.world, f.yard, shipyard, Faction::Alliance);
        let yards = &f.world.systems[f.yard].manufacturing_facilities;
        let team: Vec<_> = [yards[1], yards[0]]
            .map(|k| ScrapTarget::Facility(FacilityRef::Manufacturing(k)))
            .to_vec();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.target = Target::System(f.yard);
        let mut galaxy = records_of(&f, &state);
        let order = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.clear_target(io)
        });
        assert_eq!(order, Some(AIAction::Scrap { team }));
        record(&mut galaxy, f.yard).state |= 0x200_0000;
        assert_eq!(
            with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m
                .clear_target(io)),
            None
        );
    }

    #[test]
    fn a_mine_is_built_as_a_production_facility() {
        // FUN_004dcb30: Build 0x210 with +0x48 = 0x2c000001 on the manager.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.yard = Yard::Manager(f.yard);
        state.module1.class = Some(DatId::new(MINE_CLASS));
        state.module1.count = 1;
        let mut galaxy = records_of(&f, &state);
        let order = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.build(io));
        assert_eq!(
            order,
            Some(AIAction::Build {
                system: f.yard,
                kind: BuildableKind::ProductionFacility(FacilityBuild {
                    class: DatId::new(MINE_CLASS),
                    is_alliance: true,
                }),
                cost: 10,
                count: 1,
            })
        );
    }

    #[test]
    fn with_no_room_the_mine_goes_to_the_nearest_sector_s_nearest_system() {
        // FUN_004dc150: no sector with mine room (+0x24 0x100), so from the
        // yard the nearest sector with +0x24 0x800 (FUN_00419b80, order 2)
        // and its nearest system with free raw (+0x28 0x40, FUN_0041a400).
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.yard = Yard::Manager(f.yard);
        let mut galaxy = records_of(&f, &state);
        for s in &mut galaxy.sectors {
            s.candidates = (s.candidates & !0x100) | 0x800;
        }
        assert!(with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.mine_site(io)));
        // Both systems have free raw; the yard's own system is nearest.
        assert_eq!(state.module1.target, Target::System(f.yard));

        // Without a yard to measure from there is no target.
        state.module1.yard = Yard::None;
        assert!(!with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.mine_site(io)));
        assert_eq!(state.module1.target, Target::None);
    }

    #[test]
    fn state_ten_adopts_a_system_whose_yards_fit_the_budget() {
        // FUN_004dcc40: with w > 0, the sectors by +0xc4 (unmarked yards)
        // and their own systems by v[6]: the first with yards (v[6] > 0)
        // whose upkeep fits (v[36] <= w) becomes a marked site.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let mut galaxy = records_of(&f, &state);
        with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.adopt_or_request(io)
        });
        assert_eq!(state.module1.sites, vec![f.yard]);
        assert_eq!(state.module1.target, Target::System(f.yard));
        assert_eq!(state.yard_marks.get(&f.yard), Some(&MARK_MINES));
        assert_eq!(state.controller.yard_requests, 0);

        // With no site w = 40: yard upkeep 41 does not fit, 40 does.
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.yard).values[36] = 41;
        with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.adopt_or_request(io)
        });
        assert!(state.module1.sites.is_empty());
        // At the budget it fits.
        record(&mut galaxy, f.yard).values[36] = 40;
        with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.adopt_or_request(io)
        });
        assert_eq!(state.module1.sites, vec![f.yard]);
    }

    #[test]
    fn state_ten_asks_for_yards_only_with_few_bare_sites() {
        // FUN_004dcc40 with nothing to adopt (no sector with unmarked yards,
        // +0xc4): fewer than two sites, the first without a yard (v[6]),
        // and the yard class costing at most w ask (+8 |= 0x80).
        let f = fixture();
        let poor = Tallies {
            capacity: 100,
            ..ROOMY
        };
        for (sites, yards, tallies, asks) in [
            (vec![f.empty], 0, ROOMY, true),
            (vec![f.empty], 1, ROOMY, false),
            (vec![f.empty, f.empty], 0, ROOMY, false),
            (vec![], 0, ROOMY, true),
            (vec![f.empty], 0, poor, false),
        ] {
            let mut state = AIState::new(crate::ai::AiFaction::Alliance);
            state.module1.sites = sites.clone();
            let mut galaxy = records_of(&f, &state);
            for s in &mut galaxy.sectors {
                s.values[sector_at(0xc4)] = 0;
            }
            record(&mut galaxy, f.empty).values[6] = yards;
            with_io(&f, &mut state, &mut galaxy, tallies, |m, io| {
                m.adopt_or_request(io)
            });
            let asked = state.controller.yard_requests == YARD_TOKEN;
            let case = (sites.len(), yards, tallies.capacity);
            assert_eq!(asked, asks, "{case:?}");
            assert_eq!(state.module1.sites, sites);
        }
    }

    #[test]
    fn a_claim_needs_regiments_more_yards_and_no_other_module_s_claim() {
        // FUN_004dd120: v[6] above the best so far (from 0), v[18] > 0, and
        // +0x30 0x10000000 only when it is this module's claim.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.sites = vec![f.yard, f.empty];
        let mut galaxy = records_of(&f, &state);
        for (sys, yards) in [(f.yard, 2), (f.empty, 2)] {
            let r = record(&mut galaxy, sys);
            r.values[6] = yards;
            r.values[18] = 1;
        }
        with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.claim(io));
        assert_eq!(state.module1.claimed, vec![f.yard]);

        // Another module's claim on the yard: the empty system instead.
        state.module1.claimed.clear();
        record(&mut galaxy, f.yard).kind |= CLAIMED;
        with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.claim(io));
        assert_eq!(state.module1.claimed, vec![f.empty]);
        // The old claim is this module's now: it keeps the yard.
        state.module1.claimed = vec![f.yard];
        record(&mut galaxy, f.empty).kind &= !CLAIMED;
        with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.claim(io));
        assert_eq!(state.module1.claimed, vec![f.yard]);
        assert_eq!(record(&mut galaxy, f.empty).kind & CLAIMED, 0);
        // No regiments: no claim, and the old one is cleared.
        for sys in [f.yard, f.empty] {
            record(&mut galaxy, sys).values[18] = 0;
        }
        with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.claim(io));
        assert!(state.module1.claimed.is_empty());
        assert!(!state.record_kinds.contains_key(&f.yard));
    }

    #[test]
    fn the_cycle_runs_states_one_five_seven_and_ten() {
        // FUN_004db2f0: state 1 clears module 1's request (+8 & ~0x80);
        // state 5 ends the cycle; state 7 steps on to 8 without an order;
        // state 10 runs FUN_004dcc40 and ends it.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.controller.yard_requests = YARD_TOKEN | 1;
        state.controller.yard_holder = 1;
        state.module1.phase = CYCLE;
        state.module1.state = 1;
        let s = step(&f, &mut state, ROOMY);
        assert_eq!(state.controller.yard_requests, 1);
        assert_eq!(state.module1.state, 10);
        assert!(!s.complete);
        let s = step(&f, &mut state, ROOMY);
        assert!(s.complete);
        assert_eq!((state.module1.phase, state.module1.state), (0, 0));
        assert_eq!(state.module1.sites, vec![f.yard]);

        state.module1.phase = CYCLE;
        state.module1.state = 5;
        let s = step(&f, &mut state, ROOMY);
        assert!(s.complete);
        assert_eq!((state.module1.phase, state.module1.state), (0, 0));

        state.module1.phase = CYCLE;
        state.module1.state = 7;
        state.module1.target = Target::None;
        let s = step(&f, &mut state, ROOMY);
        assert!(!s.complete);
        assert_eq!(state.module1.state, 8);
        // State 8 with no target system ends the cycle.
        let s = step(&f, &mut state, ROOMY);
        assert!(s.complete && s.order.is_none());
        assert_eq!((state.module1.phase, state.module1.state), (0, 0));
        // Another phase value is cleared without completing.
        state.module1.phase = 7;
        let s = step(&f, &mut state, ROOMY);
        assert!(!s.complete);
        assert_eq!(state.module1.phase, 0);
    }

    #[test]
    fn marks_and_kept_bits_change_records_and_the_side_s_maps() {
        // FUN_00433e30 marks a record dirty; the bits a module sets show at
        // once and stay for the next refresh (AIState maps).
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let mut galaxy = records_of(&f, &state);
        with_io(&f, &mut state, &mut galaxy, ROOMY, |_, io| {
            io.set_mark(f.yard, MARK_MINES | 0x20_0000, 0);
            io.set_mark(f.yard, 0, 0x20_0000);
            io.set_kind(f.yard, CLAIMED | 0x4_0000, 0);
            io.set_kind(f.yard, 0, 0x4_0000);
        });
        assert_eq!(state.yard_marks.get(&f.yard), Some(&MARK_MINES));
        assert_eq!(state.record_kinds.get(&f.yard), Some(&CLAIMED));
        let r = record(&mut galaxy, f.yard);
        assert_eq!(r.state & KEPT, MARK_MINES);
        assert_eq!(r.kind & crate::ai_records::KEPT_KIND, CLAIMED);
        with_io(&f, &mut state, &mut galaxy, ROOMY, |_, io| {
            io.set_mark(f.yard, 0, MARK_MINES);
            io.set_kind(f.yard, 0, CLAIMED);
        });
        assert!(state.yard_marks.is_empty() && state.record_kinds.is_empty());
        // A rebuilt galaxy keeps what the maps hold.
        state.set_yard_marks(f.yard, MARK_MINES);
        state.record_kinds.insert(f.yard, CLAIMED);
        let mut galaxy = records_of(&f, &state);
        let r = record(&mut galaxy, f.yard);
        assert_eq!(
            (r.state & MARK_MINES, r.kind & CLAIMED),
            (MARK_MINES, CLAIMED)
        );
    }

    #[test]
    fn a_construction_manager_needs_the_side_s_construction_yard() {
        // FUN_00509670(system, 1): the 0x2a manager of the side (hyp).
        let mut f = fixture();
        assert!(has_construction_yard(&f.world, Faction::Alliance, f.yard));
        assert!(!has_construction_yard(&f.world, Faction::Empire, f.yard));
        assert!(!has_construction_yard(&f.world, Faction::Alliance, f.empty));
        let shipyard = class(&mut f.world, 0x2800_0001, 13, 3, 0);
        add_yard(&mut f.world, f.empty, shipyard, Faction::Alliance);
        assert!(!has_construction_yard(&f.world, Faction::Alliance, f.empty));
    }

    #[test]
    fn the_tallies_sum_the_side_s_mines_and_refineries() {
        // FUN_0041b7e0: +0x1d0 += sector +0xa8, +0x1d4 += +0xac.
        let mut f = fixture();
        for (raw, is_mine, at) in [
            (MINE_CLASS, true, f.yard),
            (MINE_CLASS, true, f.empty),
            (REFINERY_CLASS, false, f.empty),
        ] {
            let key =
                f.world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId::new(raw),
                        side: Faction::Alliance,
                        is_mine,
                    });
            f.world.systems[at].production_facilities.push(key);
        }
        let state = AIState::new(crate::ai::AiFaction::Alliance);
        let t = Tallies::of(&records_of(&f, &state), 7, 3);
        assert_eq!(
            t,
            Tallies {
                mines: 2,
                refineries: 1,
                capacity: 7,
                requirement: 3,
                defenses: 0,
                special_upkeep: 0,
                special_goal: 0,
                special_ready: [0; 4],
                regiment_upkeep: 0,
                cargo_upkeep: 0,
                sector_short: false,
            }
        );
    }

    #[test]
    fn setting_a_bit_already_set_keeps_it() {
        // FUN_004daf10 and FUN_004db510 OR the mark in (`| 0x800000`).
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let mut galaxy = records_of(&f, &state);
        with_io(&f, &mut state, &mut galaxy, ROOMY, |_, io| {
            for _ in 0..2 {
                io.set_mark(f.yard, MARK_MINES, 0);
                io.set_kind(f.yard, CLAIMED, 0);
            }
        });
        assert_eq!(state.yard_marks.get(&f.yard), Some(&MARK_MINES));
        assert_eq!(state.record_kinds.get(&f.yard), Some(&CLAIMED));
        let r = record(&mut galaxy, f.yard);
        assert_eq!(
            (r.state & MARK_MINES, r.kind & CLAIMED),
            (MARK_MINES, CLAIMED)
        );
    }

    #[test]
    fn an_idle_site_without_a_manager_is_not_kept() {
        // FUN_004daca0 then FUN_00509670(system, 1): no manager, no site.
        let mut f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.sites = vec![f.yard];
        state.set_yard_marks(f.yard, MARK_MINES);
        let mut galaxy = records_of(&f, &state);
        f.world.systems[f.yard].manufacturing_facilities.clear();
        assert!(!with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.keep_site(io)));
        assert_eq!(state.module1.yard, Yard::System(f.yard));
    }

    #[test]
    fn with_no_own_system_no_site_is_found_and_the_flag_is_set() {
        // FUN_004daf10: no system at all sets controller +4 |= 0x40, keeping
        // its other bits.
        let mut f = fixture();
        for sys in [f.yard, f.empty] {
            f.world.systems[sys].control = crate::world::ControlKind::Uncontrolled;
        }
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.controller.flags = 1;
        let mut galaxy = records_of(&f, &state);
        assert!(!with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.find_site(io)));
        assert_eq!(state.controller.flags, 1 | NO_SITE);
        assert!(state.module1.sites.is_empty());
    }

    #[test]
    fn state_three_takes_an_unmarked_target_s_training_facility() {
        // FUN_004db820: the 0x29 list unless +0x28 0x400000.
        let mut f = fixture();
        let training = class(&mut f.world, 0x2900_0002, 10, 3, 0);
        add_yard(&mut f.world, f.yard, training, Faction::Alliance);
        let key = f.world.systems[f.yard].manufacturing_facilities[1];
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.target = Target::System(f.yard);
        let mut galaxy = records_of(&f, &state);
        let order = with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| {
            m.scrap_at_target(io)
        });
        assert_eq!(
            order,
            Some(AIAction::Scrap {
                team: vec![ScrapTarget::Facility(FacilityRef::Manufacturing(key))]
            })
        );
    }

    #[test]
    fn with_no_yard_budget_the_class_is_left_alone() {
        // FUN_004dbd60 calls FUN_0041b230 only for w > 0; a free yard class
        // (maintenance 0) builds none rather than dividing by it.
        let mut f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        state.module1.target = Target::System(f.yard);
        state.module1.class = Some(DatId::new(MINE_CLASS));
        state.module1.sites = vec![f.yard];
        let mut galaxy = records_of(&f, &state);
        let even = Tallies {
            capacity: 250,
            ..ROOMY
        };
        assert_eq!(
            with_io(&f, &mut state, &mut galaxy, even, |m, io| m.plan_yards(io)),
            6
        );
        assert_eq!(state.module1.class, Some(DatId::new(MINE_CLASS)));

        f.world
            .buildable_classes
            .get_mut(&DatId::new(0x2a00_0003))
            .unwrap()
            .maintenance_cost = 0;
        let mut galaxy = records_of(&f, &state);
        assert_eq!(
            with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m.plan_yards(io)),
            6
        );
        assert_eq!(state.module1.count, 0);
    }

    #[test]
    fn state_ten_searches_only_with_a_yard_budget() {
        // FUN_004dcc40: the adoption search needs w > 0; with w == 0 a
        // free yard (v[36] = 0) is not adopted.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let mut galaxy = records_of(&f, &state);
        record(&mut galaxy, f.yard).values[36] = 0;
        let none = Tallies {
            capacity: 0,
            ..ROOMY
        };
        with_io(&f, &mut state, &mut galaxy, none, |m, io| {
            m.adopt_or_request(io)
        });
        assert!(state.module1.sites.is_empty());
    }

    #[test]
    fn state_five_passes_over_a_site_without_a_record() {
        // FUN_004dbe70: a site with no record is skipped.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let mut nowhere = slotmap::SlotMap::<SystemKey, ()>::with_key();
        let missing = (0..9).map(|_| nowhere.insert(())).last().unwrap();
        state.module1.sites = vec![missing];
        let mut galaxy = records_of(&f, &state);
        assert_eq!(
            with_io(&f, &mut state, &mut galaxy, ROOMY, |m, io| m
                .scrap_spare_yard(io)),
            None
        );
        assert_eq!(state.module1.sites, vec![missing]);
    }

    #[test]
    fn a_refused_order_ends_the_cycle() {
        // FUN_004dd810: +0x38 = +0x3c = 0.
        let mut m = Module1 {
            phase: CYCLE,
            state: 9,
            ..Module1::default()
        };
        m.refused();
        assert_eq!((m.phase, m.state), (0, 0));
    }

    #[test]
    fn a_build_names_the_kind_of_its_class_family() {
        // The Build order's class (0x210 facilities and defenses, 0x212
        // regiments and special forces) goes to the matching kind.
        let f = fixture();
        let mut state = AIState::new(crate::ai::AiFaction::Alliance);
        let mut galaxy = records_of(&f, &state);
        let kinds = with_io(&f, &mut state, &mut galaxy, ROOMY, |_, io| {
            [
                0x1000_0001,
                0x3c00_0001,
                0x2200_0001,
                0x2c00_0001,
                0x2a00_0003,
            ]
            .map(|raw| {
                match build(io, Yard::Manager(f.yard), Some(DatId::new(raw)), 1) {
                    Some(AIAction::Build { kind, .. }) => kind,
                    other => panic!("{other:?}"),
                }
            })
        });
        let facility = |raw| FacilityBuild {
            class: DatId::new(raw),
            is_alliance: true,
        };
        assert_eq!(
            kinds,
            [
                BuildableKind::Troop(DatId::new(0x1000_0001)),
                BuildableKind::SpecialForce(DatId::new(0x3c00_0001)),
                BuildableKind::DefenseFacility(facility(0x2200_0001)),
                BuildableKind::ProductionFacility(facility(0x2c00_0001)),
                BuildableKind::ManufacturingFacility(facility(0x2a00_0003)),
            ]
        );
    }
}
