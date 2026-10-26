//! Module 7 of the AI player (`FUN_004d2ef0`, step `FUN_004d3100`): it
//! balances the side's fighter squadrons on planets. It sends a planet's
//! surplus squadrons to a fleet with room or a system short of them, or
//! scraps them; builds squadrons at its shipyard sites for systems short of
//! them; and builds shipyards (`ghidra/notes/ai-production-modules.md`,
//! "Module 7"). Its sites carry the shipyard mark `0x200000`
//! ([`MARK_SHIPYARD`]).

use serde::{Deserialize, Serialize};

use crate::ai::{AIAction, MARK_SHIPYARD};
use crate::ai_module1::{
    adopt_or_request, build, claim, destination, destination_in, scrap_at, scrap_spare_yard,
    yard_class, Inputs, Step, Target, Yard, ANY, SHIPYARD,
};
use crate::ai_module2::{borrow_yard, keep_site};
use crate::ai_module3::choose_target;
use crate::ai_records::{pick, ranked, Mask, Order};
use crate::ids::{DatId, FighterKey, FleetKey, SystemKey};
use crate::manufacturing::{BuildableKind, ProductionArea};
use crate::squadrons::SquadronPlace;

/// `+0x38` while the yard cycle runs (`FUN_004d3100`).
pub const YARD_CYCLE: u32 = 0x3ec;

/// `+0x38` while the surplus cycle runs (`FUN_004d3100`).
pub const SURPLUS_CYCLE: u32 = 0x3f1;

/// `+0x38` while the squadron cycle runs (`FUN_004d3100`).
pub const SQUADRON_CYCLE: u32 = 0x3f2;

/// The yard token module 7 holds the construction yards with (controller
/// `+0xc`, `+8`; `FUN_004d3350` state 5).
pub const YARD_TOKEN: u32 = 8;

/// `+0x24`: the percent of the maintenance capacity module 7 budgets
/// (`FUN_004d2ef0`): 80% of it for squadrons (`FUN_004d66a0`), 20% for
/// shipyards (`FUN_004d6700`).
const PERCENT: i32 = 0x11;

/// The fleet record bits `FUN_004d3610` and `FUN_004d4800` ask for in
/// turn: the side's own (1) with hangar room left (`0x100000`,
/// `FUN_00433620`), besides bit 8 (writer not found), then besides the
/// constructor's 4.
const FLEET_QUERIES: [u32; 2] = [0x10_0009, 0x10_0005];

/// Module 7's own fields (`FUN_004d2ef0`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module7 {
    /// `+0x38`: 0, [`SURPLUS_CYCLE`], [`SQUADRON_CYCLE`] or [`YARD_CYCLE`].
    pub phase: u32,
    /// `+0x3c`: the cycle's state.
    pub state: u32,
    /// `+0x50` in the surplus cycle: the system the squadrons leave.
    pub source: Target,
    /// `+0x50` otherwise: a site, then its manager (shipyard or
    /// construction).
    pub yard: Yard,
    /// `+0x54`: where squadrons go, a fleet or a system.
    pub destination: Option<SquadronPlace>,
    /// `+0x58`: the yard cycle's target.
    pub target: Target,
    /// `+0x5c` in the squadron cycle: the squadron class to build.
    pub fighter: Option<FighterKey>,
    /// `+0x5c` in the yard cycle: the shipyard class to build.
    pub class: Option<DatId>,
    /// `+0x60`: how many.
    pub count: i32,
    /// `+0x64`: the site's shipyards `v[4]`, or the borrowed yard's
    /// construction yards.
    pub capacity: i32,
    /// `+0x68`: the target's free room `v[0x33]`.
    pub room: i32,
    /// `+0x6c`: the sites, marked [`MARK_SHIPYARD`].
    pub sites: Vec<SystemKey>,
    /// `+0x74`: the claimed sites.
    pub claimed: Vec<SystemKey>,
}

impl Module7 {
    /// One step (`FUN_004d3100`). The scheduler steps only a ready module,
    /// so the not-ready branch never runs.
    pub fn step(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.phase {
            YARD_CYCLE => self.yard_cycle(io),
            SURPLUS_CYCLE => self.surplus_cycle(io),
            SQUADRON_CYCLE => self.squadron_cycle(io),
            _ => {
                self.phase = SURPLUS_CYCLE;
                self.state = 0;
                Step::pending()
            }
        }
    }

    /// A refused order (slot `+0x18`).
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

    /// Hand on to `next` without completing the module.
    fn hand_on(&mut self, next: u32) -> Step {
        self.state = 0;
        self.phase = next;
        Step::pending()
    }

    /// The surplus cycle (`FUN_004d3170`).
    fn surplus_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            1 => {
                if self.find_surplus(io) {
                    self.state = 2;
                    Step::pending()
                } else {
                    self.hand_on(SQUADRON_CYCLE)
                }
            }
            2 => {
                self.state = if self.choose_surplus_destination(io) {
                    3
                } else {
                    4
                };
                Step::pending()
            }
            3 => match self.move_surplus(io) {
                Some(order) => self.end(SQUADRON_CYCLE, Some(order)),
                None => {
                    self.state = 4;
                    Step::pending()
                }
            },
            4 => {
                let order = self.scrap_surplus(io);
                self.end(SQUADRON_CYCLE, order)
            }
            _ => {
                self.state = 1;
                Step::pending()
            }
        }
    }

    /// `FUN_004d34c0`: the sector with a squadron surplus (`+0x24`
    /// `0x2000000`) by key `0x22`, its own system not blockaded and not
    /// short of active squadrons (forbidding `+0x30` `0x40000`) by key
    /// `0x35`; found when the system has active squadrons over its goal
    /// (`v[53] > 0`).
    fn find_surplus(&mut self, io: &mut Inputs<'_>) -> bool {
        let sectors = io.sectors([Mask::new(0x200_0000, 0), ANY, ANY], 0x22, Order::Ascending);
        let sector = pick(&[sectors]);
        let masks = [Mask::new(0, 2), ANY, Mask::new(1, 0x4_0000)];
        let system = pick(&[io.systems(sector, masks, 0x35, Order::Ascending)]);
        self.source = system.map_or(Target::None, Target::System);
        system
            .and_then(|s| io.galaxy.system(s))
            .is_some_and(|r| r.values[53] > 0)
    }

    /// `FUN_004d3610`: where the source's surplus goes, into `+0x54`: the
    /// side's fleet with hangar room nearest the source, merged with the
    /// fleets by room (`FUN_0041a600`, `FUN_0041a4c0`), first with bit 8
    /// and then without; failing a fleet, the system nearest the source
    /// short of squadrons (`0x140000`, key `0x26`).
    ///
    /// port: no code sets fleet record bit 8 (hyp), so the first fleet
    /// query finds none; the last, through module 8's sector planners
    /// (`FUN_00419e90`), is not ported and finds nothing.
    fn choose_surplus_destination(&mut self, io: &mut Inputs<'_>) -> bool {
        self.destination = None;
        let Some(source) = self.source.system() else {
            return false;
        };
        for require in FLEET_QUERIES {
            if let Some(fleet) = fleet_with_room(io, source, require) {
                self.destination = Some(SquadronPlace::Fleet(fleet));
                return true;
            }
        }
        let sectors = io.galaxy.sectors_by_distance(
            source,
            [Mask::new(0x1000_0000, 0), ANY, ANY],
            Order::Descending,
            io.coin,
        );
        let sector = pick(&[sectors]);
        let masks = [Mask::new(0x14_0000, 2), ANY, Mask::new(1, 0)];
        let system = pick(&[io.systems(sector, masks, 0x26, Order::Descending)]);
        self.destination = system.map(SquadronPlace::Planet);
        system.is_some()
    }

    /// `FUN_004d3a90`: a Move (`0x201`) of the source's weakest squadron to
    /// the destination, when the destination wants any: a system its
    /// shortfall (`-v[38]`), a fleet its hangar room (fleet record
    /// `+0xac`), at most the source's surplus `v[53]`.
    ///
    /// The loop takes the weakest squadron again on each pass without
    /// leaving out the one taken, so the team holds one squadron.
    fn move_surplus(&mut self, io: &mut Inputs<'_>) -> Option<AIAction> {
        let source = self.source.system()?;
        let wanted = match self.destination? {
            SquadronPlace::Planet(system) => io.galaxy.system(system).map_or(0, |r| -r.values[38]),
            SquadronPlace::Fleet(fleet) => io.galaxy.fleets.get(&fleet).map_or(0, |r| r.values[23]),
        };
        let surplus = io.galaxy.system(source).map_or(0, |r| r.values[53]);
        if wanted.min(surplus) <= 0 {
            return None;
        }
        let class = weakest_squadron(io, source)?;
        Some(AIAction::MoveSquadrons {
            class,
            count: 1,
            from: SquadronPlace::Planet(source),
            to: self.destination?,
        })
    }

    /// `FUN_004d3db0`: a Scrap (`0x200`) of the source's weakest squadron
    /// when it holds more counted squadrons than its goal (`v[38] > 0`);
    /// one squadron, as in [`Self::move_surplus`].
    fn scrap_surplus(&mut self, io: &mut Inputs<'_>) -> Option<AIAction> {
        let source = self.source.system()?;
        if io.galaxy.system(source)?.values[38] <= 0 {
            return None;
        }
        let class = weakest_squadron(io, source)?;
        Some(AIAction::Scrap {
            team: vec![crate::scrap::ScrapTarget::PlanetFighter {
                system: source,
                class,
            }],
        })
    }

    /// The squadron cycle (`FUN_004d3220`).
    fn squadron_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            5 => {
                self.state = if self.keep_shipyard_site(io) { 2 } else { 10 };
                Step::pending()
            }
            2 => {
                self.state = if self.choose_short_system(io) { 6 } else { 10 };
                Step::pending()
            }
            6 => {
                self.state = self.plan_squadrons(io);
                if self.state == 0 {
                    self.phase = YARD_CYCLE;
                    return Step::complete();
                }
                Step::pending()
            }
            10 => {
                if over_budget(io) {
                    self.state = 7;
                    Step::pending()
                } else {
                    self.hand_on(YARD_CYCLE)
                }
            }
            7 => {
                let order = self.relieve(io);
                self.end(YARD_CYCLE, order)
            }
            0xb => {
                let target = match self.destination {
                    Some(SquadronPlace::Planet(system)) => Target::System(system),
                    _ => Target::None,
                };
                match destination_in(ProductionArea::Shipyard, target, self.yard) {
                    Some(order) => {
                        self.state = 0xc;
                        Step {
                            order: Some(order),
                            complete: true,
                            raise: true,
                        }
                    }
                    None => self.end(YARD_CYCLE, None),
                }
            }
            0xc => {
                let order = self.build_squadrons(io);
                self.end(YARD_CYCLE, order)
            }
            _ => {
                self.state = 5;
                Step::pending()
            }
        }
    }

    /// `FUN_004d3ff0`: module 2's site keeping on this module's list
    /// (shipyards, idle bit `0x200`), the site's shipyards `v[4]` into
    /// `+0x64`, and its shipyard manager as the yard.
    ///
    /// port: the fallbacks, controller `+0x20` and `+0x24`, have no
    /// counterpart (no writer is found).
    fn keep_shipyard_site(&mut self, io: &mut Inputs<'_>) -> bool {
        self.capacity = 0;
        let kept = keep_site(io, &mut self.sites, &mut self.yard, MARK_SHIPYARD, SHIPYARD);
        if let Yard::System(site) | Yard::Manager(site) = self.yard {
            self.capacity = io.galaxy.system(site).map_or(0, |r| r.values[4]);
        }
        kept
    }

    /// `FUN_004d4370`: the system short of squadrons into `+0x54`, from the
    /// site: the nearest sector short in a marked system (`+0x24`
    /// `0x4000000`, its systems forbidding `0x4000003`), else the nearest
    /// with a headquarters (`0x40000000`), else the nearest short
    /// (`0x1000000`); systems short of counted squadrons (`+0x28`
    /// `0x40000`, own) by key `0x26`. Found when the system wants squadrons
    /// (`v[38] < 0`). With no system and a squadron budget, the side's
    /// squadron goals rise by one (`FUN_00418240(9, 10, 11)`).
    fn choose_short_system(&mut self, io: &mut Inputs<'_>) -> bool {
        self.destination = None;
        let site = match self.yard {
            Yard::System(site) | Yard::Manager(site) => site,
            Yard::None => return false,
        };
        let mut system = None;
        for (sector_bit, forbid) in [(0x400_0000, 0x400_0003), (0x4000_0000, 3), (0x100_0000, 3)] {
            let sectors = io.galaxy.sectors_by_distance(
                site,
                [Mask::new(sector_bit, 0), ANY, ANY],
                Order::Descending,
                io.coin,
            );
            let sector = pick(&[sectors]);
            let masks = [Mask::new(0x4_0000, forbid), ANY, Mask::new(1, 0)];
            system = pick(&[io.systems(sector, masks, 0x26, Order::Descending)]);
            if system.is_some() {
                break;
            }
        }
        let Some(system) = system else {
            if squadron_budget(io) > 0 {
                for goal in &mut io.controller.squadron_goals {
                    *goal += 1;
                }
            }
            return false;
        };
        self.destination = Some(SquadronPlace::Planet(system));
        io.galaxy.system(system).is_some_and(|r| r.values[38] < 0)
    }

    /// `FUN_004d46e0`: `0xb` to build what the shortfall `-v[38]` asks for,
    /// within the squadron budget and the site's shipyards; 7 when the
    /// side's squadrons cost more than the budget; else 0. The class is
    /// the cheapest (key 1, order 2) for an unmarked system, else the one
    /// with the best `+0x34` (key `0x400`, order 1) (`FUN_0041b230(.., 0x28,
    /// 0x200000, ..)`; `FUN_00473640`).
    fn plan_squadrons(&mut self, io: &mut Inputs<'_>) -> u32 {
        self.count = 0;
        let budget = squadron_budget(io);
        if budget < 1 {
            return if budget < 0 { 7 } else { 0 };
        }
        let record = match self.destination {
            Some(SquadronPlace::Planet(system)) => io.galaxy.system(system).copied(),
            _ => None,
        };
        let (key, order) = match record {
            Some(r) => {
                self.count = -r.values[38];
                if r.state & 0x83e0_0000 == 0 {
                    (FighterRank::Maintenance, Order::Descending)
                } else {
                    (FighterRank::Value(1), Order::Ascending)
                }
            }
            None => (FighterRank::Maintenance, Order::Descending),
        };
        let maintenance;
        (self.fighter, maintenance) = fighter_class(io, key, order);
        if self.fighter.is_some() && maintenance != 0 && maintenance * self.count > budget {
            self.count = budget / maintenance;
        }
        if self.count <= 0 {
            return 0;
        }
        self.count = self.count.min(self.capacity);
        0xb
    }

    /// `FUN_004d5f10`: Build `0x211` of `+0x60` squadrons of `+0x5c` on the
    /// shipyard manager.
    fn build_squadrons(&self, io: &Inputs<'_>) -> Option<AIAction> {
        let Yard::Manager(yard) = self.yard else {
            return None;
        };
        let class = self.fighter?;
        let cost = io
            .world
            .fighter_classes
            .get(class)
            .map_or(0, |c| c.refined_material_cost);
        Some(AIAction::Build {
            system: yard,
            kind: BuildableKind::Fighter(class),
            cost,
            count: self.count,
        })
    }

    /// `FUN_004d4800`: with the side's squadrons over budget, take the
    /// weakest squadron of a system with a surplus, else of an own system
    /// with squadrons (`+0x28` `0x80000`, by `v[23]`), and Move it to a
    /// fleet with hangar room (as [`Self::choose_surplus_destination`]), or
    /// Scrap it when no fleet takes it.
    ///
    /// port: module 8's sector planners (`FUN_00419e90`) are not ported, so
    /// a squadron no fleet takes is scrapped.
    fn relieve(&mut self, io: &mut Inputs<'_>) -> Option<AIAction> {
        self.destination = None;
        let sectors = io.sectors([Mask::new(0x200_0000, 0), ANY, ANY], 0x22, Order::Ascending);
        let sector = pick(&[sectors]);
        let masks = [ANY, ANY, Mask::new(1, 0x4_0000)];
        let mut system = pick(&[io.systems(sector, masks, 0x35, Order::Ascending)]);
        for (sector_forbid, state_forbid) in [(0x4000_0000, 0x8000_0000), (0, 0)] {
            if system.is_some() {
                break;
            }
            let sectors = io.sectors(
                [Mask::new(0x800_0000, sector_forbid), ANY, ANY],
                0x19,
                Order::Descending,
            );
            let sector = pick(&[sectors]);
            let masks = [
                Mask::new(0x8_0000, state_forbid),
                ANY,
                Mask::new(0, 0x4_0000),
            ];
            system = pick(&[io.systems(sector, masks, 0x17, Order::Descending)]);
        }
        let source = system?;
        self.source = Target::System(source);
        let class = weakest_squadron(io, source);
        for require in FLEET_QUERIES {
            if let Some(fleet) = fleet_with_room(io, source, require) {
                self.destination = Some(SquadronPlace::Fleet(fleet));
                break;
            }
        }
        let count = u32::from(class.is_some());
        Some(match self.destination {
            Some(to) => AIAction::MoveSquadrons {
                class: class.unwrap_or_default(),
                count,
                from: SquadronPlace::Planet(source),
                to,
            },
            None => AIAction::Scrap {
                team: class
                    .map(|class| crate::scrap::ScrapTarget::PlanetFighter {
                        system: source,
                        class,
                    })
                    .into_iter()
                    .collect(),
            },
        })
    }

    /// The yard cycle (`FUN_004d3350`): module 6's for shipyards, with the
    /// token 8, module 2's borrow and the states renumbered.
    fn yard_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            2 => {
                if choose_target(
                    io,
                    &mut self.sites,
                    &mut self.target,
                    &mut self.room,
                    MARK_SHIPYARD,
                    SHIPYARD,
                ) {
                    self.state = 5;
                    Step::pending()
                } else {
                    self.end(SURPLUS_CYCLE, None)
                }
            }
            5 => {
                io.controller.yard_requests &= !YARD_TOKEN;
                if io.controller.yard_holder == YARD_TOKEN && self.borrow_yard(io) {
                    io.controller.pass_yards();
                    self.state = 8;
                } else {
                    self.state = 0xd;
                }
                Step::pending()
            }
            8 => {
                let order = scrap_at(io, self.target.system(), SHIPYARD);
                self.state = 6;
                Step::ordered(order)
            }
            6 => {
                self.state = self.plan_yards(io);
                if self.state == 0 {
                    return self.end(SURPLUS_CYCLE, None);
                }
                Step::pending()
            }
            9 => {
                let order = scrap_spare_yard(
                    io,
                    &mut self.sites,
                    &mut self.target,
                    MARK_SHIPYARD,
                    SHIPYARD,
                );
                self.end(SURPLUS_CYCLE, order)
            }
            0xb => match destination(self.target, self.yard) {
                Some(order) => {
                    self.state = 0xc;
                    Step {
                        order: Some(order),
                        complete: true,
                        raise: true,
                    }
                }
                None => self.end(SURPLUS_CYCLE, None),
            },
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
                    MARK_SHIPYARD,
                    YARD_TOKEN,
                    budget,
                    SHIPYARD,
                );
                claim(io, &self.sites, &mut self.claimed, SHIPYARD);
                self.end(SURPLUS_CYCLE, None)
            }
            _ => {
                self.state = 2;
                Step::pending()
            }
        }
    }

    /// `FUN_004d52e0` (module 2's `FUN_004e3810`): borrow the idle
    /// construction yard of module 1's site nearest the target, and make
    /// the target a site.
    fn borrow_yard(&mut self, io: &mut Inputs<'_>) -> bool {
        self.capacity = 0;
        borrow_yard(
            io,
            self.target,
            &mut self.sites,
            &mut self.yard,
            &mut self.capacity,
            MARK_SHIPYARD,
        )
    }

    /// `FUN_004d5a50`: `0xb` to build shipyards, 9 to scrap one when the
    /// sites hold more than the side wants, else 0.
    fn plan_yards(&mut self, io: &mut Inputs<'_>) -> u32 {
        self.count = 0;
        let budget = yard_budget(io, &self.sites);
        if budget < 0 {
            return 9;
        }
        if budget == 0 {
            return 0;
        }
        let maintenance;
        (self.class, maintenance) = yard_class(io, SHIPYARD);
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

/// The squadron class record value a class choice ranks on
/// (`FUN_00473640`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FighterRank {
    /// Key 1: the maintenance (`+0x20`).
    Maintenance,
    /// `values[i]` (`+0x30 + 4i`; key `0x400` is `values[1]`).
    Value(usize),
}

/// `FUN_0041b230(&class, 0x28, 0x200000, key, order)`: the squadron classes
/// the side's shipyards list, ranked on the record value the key selects
/// and picked; with the class's maintenance, 0 with none.
fn fighter_class(io: &mut Inputs<'_>, key: FighterRank, order: Order) -> (Option<FighterKey>, i32) {
    let classes: Vec<_> = crate::build_selection::listed_classes(
        io.world,
        io.research,
        ProductionArea::Shipyard,
        io.is_alliance,
    )
    .into_iter()
    .filter_map(|kind| match kind {
        BuildableKind::Fighter(class) => {
            let r = io.galaxy.classes.fighter(class)?;
            let value = match key {
                FighterRank::Maintenance => r.maintenance,
                FighterRank::Value(i) => r.values[i],
            };
            Some((class, value))
        }
        _ => None,
    })
    .collect();
    let class = pick(&[ranked(classes, order, io.coin)]);
    let maintenance = class
        .and_then(|c| io.galaxy.classes.fighter(c))
        .map_or(0, |r| r.maintenance);
    (class, maintenance)
}

/// The squadron on `system`'s planet with the lowest strength (slot
/// `+0x1f4`, `FUN_005031e0`: fighters left times a constant), the first on
/// a tie, of either side (`FUN_005039d0` mode 1). port: every squadron is
/// at full strength, its class's squadron size.
fn weakest_squadron(io: &Inputs<'_>, system: SystemKey) -> Option<FighterKey> {
    let sys = io.world.systems.get(system)?;
    let mut best: Option<(FighterKey, u32)> = None;
    for entry in sys.fighters.iter().filter(|e| e.count > 0) {
        let size = io
            .world
            .fighter_classes
            .get(entry.class)
            .map_or(0, |c| c.squadron_size);
        if best.is_none_or(|(_, top)| size < top) {
            best = Some((entry.class, size));
        }
    }
    best.map(|(class, _)| class)
}

/// `FUN_0041a600` and `FUN_0041a4c0` merged and picked: the fleet whose
/// record has `require` and is not en route (2), ranked by its squared
/// distance from the source's sector (order 2) and by its hangar room
/// `+0xac` (key `0x17`, order 1).
fn fleet_with_room(io: &mut Inputs<'_>, source: SystemKey, require: u32) -> Option<FleetKey> {
    let home = io.galaxy.sector_position_of(source)?;
    let mask = Mask::new(require, 2);
    let near = io
        .galaxy
        .fleets_near(io.world, home, mask, Order::Descending, io.coin);
    let roomy = ranked(
        io.galaxy
            .fleets_in(io.world, mask)
            .into_iter()
            .map(|key| (key, io.galaxy.fleets[&key].values[23])),
        Order::Ascending,
        io.coin,
    );
    pick(&[near, roomy])
}

/// `FUN_004d66a0`: 80% of `+0x24` percent of the capacity, less the
/// squadrons' upkeep on planets (`+0x1e0`); 0 when the side's free
/// maintenance falls short of it.
fn squadron_budget(io: &Inputs<'_>) -> i32 {
    let t = io.tallies;
    let budget = PERCENT * t.capacity / 100 * 80 / 100 - t.squadron_upkeep;
    if t.capacity - t.requirement < budget {
        0
    } else {
        budget
    }
}

/// `FUN_004d47b0`: 90% of `+0x24` percent of the capacity falls short of
/// the squadrons' upkeep on planets.
fn over_budget(io: &Inputs<'_>) -> bool {
    let t = io.tallies;
    PERCENT * t.capacity / 100 * 90 / 100 - t.squadron_upkeep < 0
}

/// `FUN_004d6700`: 20% of `+0x24` percent of the capacity, less the sites'
/// shipyard upkeep (`v[34]`); 0 when the side's free maintenance falls
/// short of it.
fn yard_budget(io: &Inputs<'_>, sites: &[SystemKey]) -> i32 {
    let held: i32 = sites
        .iter()
        .filter_map(|&s| io.galaxy.system(s))
        .map(|r| r.values[34])
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
    use crate::ai_records::{FleetRecord, Galaxy, SystemRecord};
    use crate::dat::Faction;
    use crate::research::ResearchState;
    use crate::world::{CapitalShipClass, FighterClass, Fleet, ShipInstance};

    const SHIPYARD_CLASS: u32 = 0x2800_0001;

    fn with_io<R>(
        f: &Fixture,
        state: &mut AIState,
        galaxy: &mut Galaxy,
        tallies: Tallies,
        run: impl FnOnce(&mut Module7, &mut Inputs<'_>) -> R,
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
        run(&mut state.module7, &mut io)
    }

    fn edited<R>(
        f: &Fixture,
        state: &mut AIState,
        tallies: Tallies,
        edit: impl FnOnce(&mut Galaxy),
        run: impl FnOnce(&mut Module7, &mut Inputs<'_>) -> R,
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

    struct Wings {
        /// Squadron size 12, maintenance 2, `values[1]` 60.
        cheap: FighterKey,
        /// Squadron size 6, maintenance 5, `values[1]` 300.
        strong: FighterKey,
    }

    /// The fixture with a shipyard at `yard` and two Alliance squadron
    /// classes.
    fn squadron_fixture() -> (Fixture, Wings) {
        let mut f = fixture();
        let shipyard = class(&mut f.world, SHIPYARD_CLASS, 10, 2, 0);
        add_yard(&mut f.world, f.yard, shipyard, Faction::Alliance);
        let mut wing = |size, maintenance, maneuver| {
            f.world.fighter_classes.insert(FighterClass {
                is_alliance: true,
                squadron_size: size,
                maintenance_cost: maintenance,
                refined_material_cost: 7,
                laser_cannon_fore: 1,
                maneuverability: maneuver,
                ..FighterClass::default()
            })
        };
        let cheap = wing(12, 2, 1);
        let strong = wing(6, 5, 10);
        (f, Wings { cheap, strong })
    }

    fn with_site(f: &Fixture) -> AIState {
        let mut state = AIState::new(AiFaction::Alliance);
        state.module7.sites.push(f.yard);
        state.yard_marks.insert(f.yard, MARK_SHIPYARD);
        state
    }

    /// An Alliance carrier fleet with `room` hangar room at `at`, and its
    /// record (own, hangar room left: `0x100005`).
    fn carrier(f: &mut Fixture, at: SystemKey, room: u32) -> FleetKey {
        let class = f.world.capital_ship_classes.insert(CapitalShipClass {
            is_alliance: true,
            fighter_capacity: room,
            hull: 100,
            ..CapitalShipClass::default()
        });
        let fleet = f.world.fleets.insert(Fleet {
            location: at,
            capital_ships: vec![ShipInstance::new(class, 100, true)],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        f.world.systems[at].fleets.push(fleet);
        fleet
    }

    fn fleet_record(g: &mut Galaxy, fleet: FleetKey, room: i32) {
        let mut values = [0; 30];
        values[23] = room;
        g.fleets.insert(
            fleet,
            FleetRecord {
                flags: 0x10_0005,
                values,
                ships: Vec::new(),
                owner: 0,
                plan: 0,
                target: None,
            },
        );
    }

    #[test]
    fn the_cycles_hand_on_from_surplus_to_squadrons_to_yards() {
        // FUN_004d3100: any other +0x38 starts 0x3f1. FUN_004d3170 1 finds
        // no surplus and hands on to 0x3f2 without completing. FUN_004d3220
        // 5 keeps no site: 10 (FUN_004d47b0, within budget) hands on to
        // 0x3ec. FUN_004d3350 2 takes the yard as the target (module 1's
        // mark is not forbidden, 0x200003); 5 without the token goes to
        // 0xd, which ends with 0x3f1, complete.
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.yard_marks.insert(f.empty, MARK_MINES);
        let mut trail = Vec::new();
        for _ in 0..12 {
            let s = step(&f, &mut state, ROOMY);
            trail.push((state.module7.phase, state.module7.state, s.complete));
        }
        assert_eq!(
            trail,
            [
                (SURPLUS_CYCLE, 0, false),
                (SURPLUS_CYCLE, 1, false),
                (SQUADRON_CYCLE, 0, false),
                (SQUADRON_CYCLE, 5, false),
                (SQUADRON_CYCLE, 10, false),
                (YARD_CYCLE, 0, false),
                (YARD_CYCLE, 2, false),
                (YARD_CYCLE, 5, false),
                (YARD_CYCLE, 0xd, false),
                (SURPLUS_CYCLE, 0, true),
                (SURPLUS_CYCLE, 1, false),
                (SQUADRON_CYCLE, 0, false),
            ]
        );
        state.module7.refused();
        assert_eq!((state.module7.phase, state.module7.state), (0, 0));
    }

    /// Records with a squadron surplus at `yard` (`v[53]` 2, `v[38]`
    /// `counted`) in a sector flagged `0x2000000`.
    fn surplus(g: &mut Galaxy, f: &Fixture, counted: i32) {
        g.sectors[0].candidates |= 0x200_0000;
        let s = rec(g, f.yard);
        s.values[53] = 2;
        s.values[38] = counted;
    }

    #[test]
    fn the_surplus_s_weakest_squadron_goes_to_a_fleet_with_hangar_room() {
        // FUN_004d34c0 finds the source (v[53] > 0); FUN_004d3610 takes the
        // own fleet with hangar room (0x100005, FUN_0041a600 merged with
        // FUN_0041a4c0); FUN_004d3a90 moves the weakest squadron (slot
        // +0x1f4: the smallest) as Move 0x201, one squadron, ending the
        // cycle.
        let (mut f, w) = squadron_fixture();
        crate::squadrons::add(&mut f.world.systems[f.yard].fighters, w.cheap, 1);
        crate::squadrons::add(&mut f.world.systems[f.yard].fighters, w.strong, 1);
        let at = f.empty;
        let fleet = carrier(&mut f, at, 3);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module7.phase = SURPLUS_CYCLE;
        state.module7.state = 1;
        let edit = |g: &mut Galaxy| {
            surplus(g, &f, 1);
            fleet_record(g, fleet, 3);
        };
        let mut trail = Vec::new();
        for _ in 0..3 {
            let s = edited(&f, &mut state, ROOMY, edit, |m, io| m.step(io));
            trail.push((state.module7.state, s));
        }
        assert_eq!((trail[0].0, trail[1].0), (2, 3));
        assert_eq!(
            trail[2].1.order,
            Some(AIAction::MoveSquadrons {
                class: w.strong,
                count: 1,
                from: SquadronPlace::Planet(f.yard),
                to: SquadronPlace::Fleet(fleet),
            })
        );
        assert!(trail[2].1.complete);
        assert_eq!(state.module7.phase, SQUADRON_CYCLE);
    }

    #[test]
    fn without_a_fleet_the_surplus_goes_to_the_nearest_short_system() {
        // FUN_004d3610: the sector nearest the source with 0x10000000, its
        // system with +0x28 0x140000 by key 0x26; FUN_004d3a90 takes at
        // most the shortfall -v[38] and the surplus v[53].
        let (mut f, w) = squadron_fixture();
        crate::squadrons::add(&mut f.world.systems[f.yard].fighters, w.cheap, 2);
        let mut state = AIState::new(AiFaction::Alliance);
        let edit = |g: &mut Galaxy| {
            surplus(g, &f, 1);
            g.sectors[0].candidates |= 0x1000_0000;
            let d = rec(g, f.empty);
            d.state |= 0x14_0000;
            d.values[38] = -2;
        };
        let order = edited(&f, &mut state, ROOMY, edit, |m, io| {
            assert!(m.find_surplus(io));
            assert!(m.choose_surplus_destination(io));
            m.move_surplus(io)
        });
        assert_eq!(
            order,
            Some(AIAction::MoveSquadrons {
                class: w.cheap,
                count: 1,
                from: SquadronPlace::Planet(f.yard),
                to: SquadronPlace::Planet(f.empty),
            })
        );
        // A destination that wants none gives no Move.
        let edit = |g: &mut Galaxy| {
            surplus(g, &f, 1);
            g.sectors[0].candidates |= 0x1000_0000;
            rec(g, f.empty).state |= 0x14_0000;
        };
        let order = edited(&f, &mut state, ROOMY, edit, |m, io| {
            m.find_surplus(io);
            m.choose_surplus_destination(io);
            m.move_surplus(io)
        });
        assert_eq!(order, None);
    }

    #[test]
    fn a_surplus_with_nowhere_to_go_is_scrapped_one_squadron_at_a_time() {
        // FUN_004d3170 2 -> 4 (no destination); FUN_004d3db0 scraps the
        // weakest squadron while the source's v[38] > 0, Scrap 0x200.
        let (mut f, w) = squadron_fixture();
        crate::squadrons::add(&mut f.world.systems[f.yard].fighters, w.cheap, 2);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module7.phase = SURPLUS_CYCLE;
        state.module7.state = 1;
        let mut last = Step::pending();
        for _ in 0..3 {
            last = edited(
                &f,
                &mut state,
                ROOMY,
                |g| surplus(g, &f, 1),
                |m, io| m.step(io),
            );
        }
        assert_eq!(
            last.order,
            Some(AIAction::Scrap {
                team: vec![crate::scrap::ScrapTarget::PlanetFighter {
                    system: f.yard,
                    class: w.cheap,
                }],
            })
        );
        // Counted squadrons at the goal: nothing to scrap.
        state.module7.phase = SURPLUS_CYCLE;
        state.module7.state = 1;
        for _ in 0..3 {
            last = edited(
                &f,
                &mut state,
                ROOMY,
                |g| surplus(g, &f, 0),
                |m, io| m.step(io),
            );
        }
        assert_eq!((last.order, last.complete), (None, true));
    }

    /// Records with the site idle and `empty` short of squadrons (`v[38]`
    /// `-short`, `+0x28` 0x40000 and `marks`) in a sector flagged `sector`.
    fn short(g: &mut Galaxy, f: &Fixture, sector: u32, marks: u32, short: i32) {
        g.sectors[0].candidates |= sector;
        let y = rec(g, f.yard);
        y.state |= 0x200;
        let d = rec(g, f.empty);
        d.state |= 0x4_0000 | marks;
        d.values[38] = -short;
    }

    #[test]
    fn the_squadron_cycle_builds_the_shortfall_at_the_site_and_sends_it() {
        // FUN_004d3220: 5 keeps the site (FUN_004d3ff0: v[4] shipyards,
        // idle 0x200), 2 finds the short system (FUN_004d4370, sector
        // 0x1000000), 6 plans (FUN_004d46e0: -v[38] = 3, at most the site's
        // shipyards v[4] = 1; the cheapest class for an unmarked system),
        // 0xb gives Destination on the shipyard manager (FUN_004d5dd0) and
        // 0xc Build 0x211 (FUN_004d5f10).
        let (f, w) = squadron_fixture();
        let mut state = with_site(&f);
        state.module7.phase = SQUADRON_CYCLE;
        // The marked site wants S38C (6) squadrons itself and would come
        // first (sector 0x4000000); here it is satisfied.
        let edit = |g: &mut Galaxy| {
            short(g, &f, 0x100_0000, 0, 3);
            let y = rec(g, f.yard);
            y.state &= !0x4_0000;
            y.values[38] = 0;
        };
        let mut steps = Vec::new();
        for _ in 0..6 {
            let s = edited(&f, &mut state, ROOMY, edit, |m, io| m.step(io));
            steps.push((state.module7.state, s));
        }
        let states: Vec<_> = steps.iter().map(|(state, _)| *state).collect();
        assert_eq!(states, [5, 2, 6, 0xb, 0xc, 0]);
        assert_eq!(state.module7.capacity, 1);
        assert_eq!(state.module7.fighter, Some(w.cheap));
        assert_eq!(
            steps[4].1.order,
            Some(AIAction::SetDestination {
                system: f.yard,
                area: ProductionArea::Shipyard,
                destination: f.empty,
            })
        );
        assert!(steps[4].1.complete && steps[4].1.raise);
        assert_eq!(
            steps[5].1.order,
            Some(AIAction::Build {
                system: f.yard,
                kind: BuildableKind::Fighter(w.cheap),
                cost: 7,
                count: 1,
            })
        );
        assert_eq!(state.module7.phase, YARD_CYCLE);
    }

    #[test]
    fn the_plan_pays_within_the_budget_and_picks_the_class_by_the_mark() {
        // FUN_004d46e0: w = FUN_004d66a0; with m * count > w, count = w / m;
        // a marked system (0x83e00000) ranks by +0x34 (key 0x400, order 1).
        let (f, w) = squadron_fixture();
        let mut state = with_site(&f);
        state.module7.capacity = 10;
        state.module7.destination = Some(SquadronPlace::Planet(f.empty));
        // 17% of 100 is 17, 80% of it 13; strong costs 5: 13 / 5 = 2.
        let tight = Tallies {
            capacity: 100,
            ..ROOMY
        };
        let plan = |state: &mut AIState, tallies, marks| {
            edited(
                &f,
                state,
                tallies,
                |g| short(g, &f, 0, marks, 6),
                |m, io| (m.plan_squadrons(io), m.count, m.fighter),
            )
        };
        assert_eq!(
            plan(&mut state, tight, MARK_SHIPYARD),
            (0xb, 2, Some(w.strong))
        );
        assert_eq!(plan(&mut state, tight, 0), (0xb, 6, Some(w.cheap)));
        // No budget: 0; over it: 7 (FUN_004d4800 next).
        let none = Tallies {
            capacity: 0,
            ..ROOMY
        };
        assert_eq!(plan(&mut state, none, 0).0, 0);
        let over = Tallies {
            squadron_upkeep: 500,
            ..ROOMY
        };
        assert_eq!(plan(&mut state, over, 0).0, 7);
    }

    #[test]
    fn with_no_short_system_and_a_budget_the_squadron_goals_rise() {
        // FUN_004d4370: no system and FUN_004d66a0 > 0 raise side +0x388,
        // +0x38c and +0x390 by one (FUN_00418240(9, 10, 11)).
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module7.yard = Yard::Manager(f.yard);
        let found = edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| m.choose_short_system(io),
        );
        assert!(!found);
        assert_eq!(state.controller.squadron_goals, [1, 7, 13]);
        let none = Tallies {
            capacity: 0,
            ..ROOMY
        };
        edited(
            &f,
            &mut state,
            none,
            |_| {},
            |m, io| m.choose_short_system(io),
        );
        assert_eq!(state.controller.squadron_goals, [1, 7, 13]);
        // A system not short is found but refused, and raises nothing.
        let edit = |g: &mut Galaxy| {
            short(g, &f, 0x100_0000, 0, 0);
        };
        let found = edited(&f, &mut state, ROOMY, edit, |m, io| {
            m.choose_short_system(io)
        });
        assert!(!found);
        assert_eq!(
            state.module7.destination,
            Some(SquadronPlace::Planet(f.empty))
        );
        assert_eq!(state.controller.squadron_goals, [1, 7, 13]);
    }

    #[test]
    fn the_raised_goals_reach_the_records() {
        // FUN_00433e40: v[38] = counted squadrons less the side's goal
        // (+0x38c for an own marked system).
        let (mut f, w) = squadron_fixture();
        crate::squadrons::add(&mut f.world.systems[f.empty].fighters, w.cheap, 2);
        let mut state = AIState::new(AiFaction::Alliance);
        // records_of reads the setup goals; build with raised ones.
        let classes = crate::ai_records::ClassRecords::build(&f.world);
        let fleets = std::collections::HashMap::new();
        let economy = crate::economy::EconomyState::default();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = crate::manufacturing::ManufacturingState::new();
        let research = ResearchState::new();
        state.controller.squadron_goals = [5, 9, 12];
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
            shield_goal: crate::ai_records::SHIELD_GOAL,
            squadron_goals: state.controller.squadron_goals,
            garrison_goals: state.controller.garrison_goals,
        };
        // A marked system (a yard mark) takes the marked goal +0x38c.
        let r = SystemRecord::build(&inputs, f.empty, MARK_SHIPYARD).unwrap();
        assert_eq!((r.values[24], r.values[38]), (2, 2 - 9));
    }

    #[test]
    fn over_budget_a_planet_s_squadron_goes_to_a_fleet_or_is_scrapped() {
        // FUN_004d47b0: 90% of 17% of the capacity below the upkeep gives 7;
        // FUN_004d4800 takes the surplus source (sector 0x2000000, key 0x35)
        // and moves its weakest squadron to a fleet with room, else scraps
        // it.
        let (mut f, w) = squadron_fixture();
        crate::squadrons::add(&mut f.world.systems[f.yard].fighters, w.cheap, 1);
        let at = f.empty;
        let fleet = carrier(&mut f, at, 2);
        let over = Tallies {
            squadron_upkeep: 200,
            ..ROOMY
        };
        let mut state = AIState::new(AiFaction::Alliance);
        state.module7.phase = SQUADRON_CYCLE;
        state.module7.state = 10;
        let s = edited(&f, &mut state, over, |_| {}, |m, io| m.step(io));
        assert_eq!((state.module7.state, s.complete), (7, false));
        let edit = |g: &mut Galaxy| {
            surplus(g, &f, 1);
            fleet_record(g, fleet, 2);
        };
        let s = edited(&f, &mut state, over, edit, |m, io| m.step(io));
        assert_eq!(
            s.order,
            Some(AIAction::MoveSquadrons {
                class: w.cheap,
                count: 1,
                from: SquadronPlace::Planet(f.yard),
                to: SquadronPlace::Fleet(fleet),
            })
        );
        assert_eq!((state.module7.phase, state.module7.state), (YARD_CYCLE, 0));
        // No fleet: Scrap. The second source query (sector 0x8000000, own
        // squadrons 0x80000 by v[23]) finds the system without a surplus.
        let (mut f, w) = squadron_fixture();
        crate::squadrons::add(&mut f.world.systems[f.yard].fighters, w.cheap, 1);
        let edit = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x800_0000;
            rec(g, f.yard).state |= 0x8_0000;
        };
        let order = edited(&f, &mut state, over, edit, |m, io| m.relieve(io));
        assert_eq!(
            order,
            Some(AIAction::Scrap {
                team: vec![crate::scrap::ScrapTarget::PlanetFighter {
                    system: f.yard,
                    class: w.cheap,
                }],
            })
        );
        // No planet with squadrons: no order.
        let (f, _) = squadron_fixture();
        let order = edited(&f, &mut state, over, |_| {}, |m, io| m.relieve(io));
        assert_eq!(order, None);
        // Within budget, 10 hands on to the yards.
        state.module7.phase = SQUADRON_CYCLE;
        state.module7.state = 10;
        let s = step(&f, &mut state, ROOMY);
        assert_eq!(
            (state.module7.phase, state.module7.state, s.complete),
            (YARD_CYCLE, 0, false)
        );
    }

    #[test]
    fn the_yard_cycle_borrows_a_yard_makes_the_target_a_site_and_builds_shipyards() {
        // FUN_004d3350 5: bit 8 cleared; with +0xc == 8, FUN_004d52e0 takes
        // the yard nearest the target (module 2's FUN_004e3810) and makes
        // the target a site marked 0x200000; FUN_004199b0; 8 (FUN_004d5510)
        // -> 6 (FUN_004d5a50, 20% of 17%, the shipyard class) gives 0xb ->
        // Destination -> Build.
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.controller.yard_holder = YARD_TOKEN;
        state.controller.yard_requests = YARD_TOKEN | 0x80;
        state.module7.phase = YARD_CYCLE;
        state.module7.state = 5;
        state.module7.target = Target::System(f.empty);
        state.module7.room = 9;
        let lent = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x1000;
            let r = rec(g, f.yard);
            r.state |= 0x800;
            r.values[6] = 2;
        };
        let s = edited(&f, &mut state, ROOMY, lent, |m, io| m.step(io));
        assert_eq!(s, Step::pending());
        assert_eq!(state.module7.state, 8);
        assert_eq!(
            (state.module7.yard, state.module7.capacity),
            (Yard::Manager(f.yard), 1)
        );
        assert_eq!(state.module7.sites, [f.empty]);
        assert_eq!(state.yard_marks.get(&f.empty), Some(&MARK_SHIPYARD));
        assert_eq!(state.controller.yard_requests, 0);
        assert_eq!(state.controller.yard_holder, 0x80);
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module7.state, 6);
        let rich = Tallies {
            capacity: 4000,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, rich), Step::pending());
        assert_eq!((state.module7.state, state.module7.count), (0xb, 1));
        let s = step(&f, &mut state, rich);
        assert!(s.complete && s.raise);
        let s = step(&f, &mut state, rich);
        let Some(AIAction::Build { kind, .. }) = s.order else {
            panic!("{s:?}");
        };
        assert_eq!(kind.class_dat_id(), Some(DatId::new(SHIPYARD_CLASS)));
        assert_eq!(
            (state.module7.phase, state.module7.state),
            (SURPLUS_CYCLE, 0)
        );
    }

    #[test]
    fn the_shipyard_plan_scraps_a_surplus_and_ends_otherwise() {
        // FUN_004d5a50: the budget below 0 gives 9, which scraps a spare
        // shipyard (FUN_004d5af0); 0 gives 0.
        let (f, _) = squadron_fixture();
        let mut state = with_site(&f);
        let heavy = |g: &mut Galaxy| rec(g, f.yard).values[34] = 500;
        let plan = edited(&f, &mut state, ROOMY, heavy, |m, io| m.plan_yards(io));
        assert_eq!(plan, 9);
        let none = Tallies {
            capacity: 0,
            ..ROOMY
        };
        let mut bare = AIState::new(AiFaction::Alliance);
        let plan = edited(&f, &mut bare, none, |_| {}, |m, io| m.plan_yards(io));
        assert_eq!(plan, 0);
    }

    #[test]
    fn the_budgets_are_eighty_twenty_and_ninety_percent_of_seventeen() {
        // FUN_004d66a0, FUN_004d6700 and FUN_004d47b0 over +0x24 = 0x11.
        let (f, _) = squadron_fixture();
        let mut state = with_site(&f);
        let t = Tallies {
            capacity: 1000,
            squadron_upkeep: 30,
            ..ROOMY
        };
        let held = |g: &mut Galaxy| rec(g, f.yard).values[34] = 4;
        let (squadrons, yards, over) = edited(&f, &mut state, t, held, |m, io| {
            (
                squadron_budget(io),
                yard_budget(io, &m.sites),
                over_budget(io),
            )
        });
        assert_eq!(
            (squadrons, yards, over),
            (170 * 80 / 100 - 30, 170 * 20 / 100 - 4, false)
        );
        let short = Tallies {
            requirement: 990,
            ..t
        };
        let (squadrons, yards) = edited(
            &f,
            &mut state,
            short,
            |_| {},
            |m, io| (squadron_budget(io), yard_budget(io, &m.sites)),
        );
        assert_eq!((squadrons, yards), (0, 0));
        let over = Tallies {
            squadron_upkeep: 154,
            ..t
        };
        assert!(edited(
            &f,
            &mut state,
            over,
            |_| {},
            |_, io| over_budget(io)
        ));
    }

    #[test]
    fn a_shipyard_target_s_scrap_spares_marked_training_and_construction_yards() {
        // FUN_004d5510: training facilities unless 0x400000, construction
        // yards unless 0x3800000, never shipyards.
        let (mut f, _) = squadron_fixture();
        let training = class(&mut f.world, 0x2900_0001, 1, 1, 0);
        add_yard(&mut f.world, f.yard, training, Faction::Alliance);
        let state = &mut AIState::new(AiFaction::Alliance);
        let team = |state: &mut AIState, marks| {
            edited(
                &f,
                state,
                ROOMY,
                |g| rec(g, f.yard).state |= marks,
                |_, io| match scrap_at(io, Some(f.yard), SHIPYARD) {
                    Some(AIAction::Scrap { team }) => team.len(),
                    _ => 0,
                },
            )
        };
        assert_eq!(team(state, 0), 2);
        assert_eq!(team(state, 0x40_0000), 1);
        assert_eq!(team(state, 0x40_0000 | 0x80_0000), 0);
    }

    #[test]
    fn a_source_with_no_active_surplus_is_not_found() {
        // FUN_004d34c0: found only with v[53] > 0.
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        let edit = |g: &mut Galaxy| {
            surplus(g, &f, 1);
            rec(g, f.yard).values[53] = 0;
        };
        assert!(!edited(&f, &mut state, ROOMY, edit, |m, io| m.find_surplus(io)));
    }

    #[test]
    fn only_the_side_s_fleet_with_hangar_room_in_orbit_takes_a_surplus() {
        // FUN_0041a600 / FUN_0041a4c0: the record must have 0x100005 (own,
        // hangar room) and not 2 (en route); an enemy fleet, a full one or
        // one in hyperspace is no destination.
        let (mut f, _) = squadron_fixture();
        let at = f.empty;
        let fleet = carrier(&mut f, at, 2);
        let mut state = AIState::new(AiFaction::Alliance);
        for flags in [0x10_0004, 0x4_0005, 0x10_0007] {
            let edit = |g: &mut Galaxy| {
                surplus(g, &f, 1);
                g.fleets.insert(
                    fleet,
                    FleetRecord {
                        flags,
                        values: [0; 30],
                        ships: Vec::new(),
                        owner: 0,
                        plan: 0,
                        target: None,
                    },
                );
            };
            let found = edited(&f, &mut state, ROOMY, edit, |m, io| {
                m.find_surplus(io);
                m.choose_surplus_destination(io)
            });
            assert!(!found, "{flags:#x}");
        }
    }

    #[test]
    fn the_nearest_fleet_with_room_wins_over_a_far_one() {
        // FUN_0041a600: the fleets by squared distance from the source's
        // sector (order 2), merged with FUN_0041a4c0 by room (order 1): the
        // near fleet with more room ranks first in both.
        let (mut f, _) = squadron_fixture();
        let far_sector = f.world.sectors.insert(crate::world::Sector {
            dat_id: DatId::new(0x8000_0002),
            name: "Far".into(),
            group: crate::dat::SectorGroup::Core,
            x: 900,
            y: 900,
            systems: vec![],
        });
        let far = crate::ai_module1::tests::system(&mut f.world, far_sector, 900);
        f.world.sectors[far_sector].systems.push(far);
        f.world.systems[far].y = 900;
        let at = f.empty;
        let near_fleet = carrier(&mut f, at, 2);
        let far_fleet = carrier(&mut f, far, 2);
        let mut state = AIState::new(AiFaction::Alliance);
        let edit = |g: &mut Galaxy| {
            surplus(g, &f, 1);
            fleet_record(g, far_fleet, 2);
            fleet_record(g, near_fleet, 3);
        };
        let destination = edited(&f, &mut state, ROOMY, edit, |m, io| {
            m.find_surplus(io);
            m.choose_surplus_destination(io);
            m.destination
        });
        assert_eq!(destination, Some(SquadronPlace::Fleet(near_fleet)));
    }

    #[test]
    fn the_weakest_squadron_is_the_first_of_the_smallest_with_any_left() {
        // Slot +0x1f4 (FUN_005031e0), compared with < from 1000000: the
        // first squadron of the lowest strength; an emptied entry has none.
        let (mut f, w) = squadron_fixture();
        let twin = f.world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            squadron_size: 6,
            ..FighterClass::default()
        });
        f.world.systems[f.yard].fighters = vec![
            crate::world::FighterEntry {
                class: w.cheap,
                count: 0,
                carrier: None,
            },
            crate::world::FighterEntry {
                class: w.strong,
                count: 1,
                carrier: None,
            },
            crate::world::FighterEntry {
                class: twin,
                count: 1,
                carrier: None,
            },
        ];
        let mut state = AIState::new(AiFaction::Alliance);
        let weakest = edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |_, io| weakest_squadron(io, f.yard),
        );
        assert_eq!(weakest, Some(w.strong));
        f.world.systems[f.yard].fighters.truncate(1);
        let weakest = edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |_, io| weakest_squadron(io, f.yard),
        );
        assert_eq!(weakest, None);
    }

    #[test]
    fn the_plan_s_edges_follow_the_budget() {
        // FUN_004d46e0: w = 1 still plans; m * count == w keeps the count; a
        // class with no maintenance keeps it too.
        let (mut f, w) = squadron_fixture();
        let mut state = with_site(&f);
        state.module7.capacity = 10;
        state.module7.destination = Some(SquadronPlace::Planet(f.empty));
        let plan = |f: &Fixture, state: &mut AIState, tallies, need| {
            edited(
                f,
                state,
                tallies,
                |g| short(g, f, 0, 0, need),
                |m, io| (m.plan_squadrons(io), m.count),
            )
        };
        // Capacity 100: 17%, 80% of it 13, less the upkeep 12: budget 1.
        let one = Tallies {
            capacity: 100,
            squadron_upkeep: 12,
            ..ROOMY
        };
        // Budget 1 with a class of upkeep 1 plans one.
        f.world.fighter_classes[w.cheap].maintenance_cost = 1;
        assert_eq!(plan(&f, &mut state, one, 1), (0xb, 1));
        f.world.fighter_classes[w.cheap].maintenance_cost = 2;
        // Within a large budget the shortfall stands (136 >= 2 * 2).
        assert_eq!(plan(&f, &mut state, ROOMY, 2), (0xb, 2));
        let exact = Tallies {
            capacity: 100,
            squadron_upkeep: 9,
            ..ROOMY
        };
        // Budget 4: two cheap squadrons cost exactly 4.
        assert_eq!(plan(&f, &mut state, exact, 2), (0xb, 2));
        f.world.fighter_classes[w.cheap].maintenance_cost = 0;
        f.world.fighter_classes[w.strong].maintenance_cost = 0;
        assert_eq!(plan(&f, &mut state, exact, 3), (0xb, 3));
    }

    #[test]
    fn a_relief_source_without_squadrons_scraps_nothing() {
        // FUN_004d4800: a source with a surplus record but no squadron left
        // gives an empty team.
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        let order = edited(
            &f,
            &mut state,
            ROOMY,
            |g| surplus(g, &f, 1),
            |m, io| m.relieve(io),
        );
        assert_eq!(order, Some(AIAction::Scrap { team: vec![] }));
    }

    #[test]
    fn the_yard_cycle_s_scrap_state_and_borrow_edges() {
        // FUN_004d3350: the plan's 9 runs FUN_004d5af0; FUN_004d52e0 keeps
        // v[6] when it is 1 and adds a site only once.
        let (f, _) = squadron_fixture();
        let mut state = with_site(&f);
        state.module7.phase = YARD_CYCLE;
        state.module7.state = 6;
        let heavy = |g: &mut Galaxy| rec(g, f.yard).values[34] = 500;
        edited(&f, &mut state, ROOMY, heavy, |m, io| m.step(io));
        assert_eq!(state.module7.state, 9);
        let s = edited(&f, &mut state, ROOMY, heavy, |m, io| m.step(io));
        assert!(s.complete);
        assert_eq!(
            (state.module7.phase, state.module7.state),
            (SURPLUS_CYCLE, 0)
        );

        let mut state = with_site(&f);
        state.module7.target = Target::System(f.yard);
        let lent = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x1000;
            let r = rec(g, f.yard);
            r.state |= 0x80_0800;
            r.values[6] = 1;
        };
        assert!(edited(&f, &mut state, ROOMY, lent, |m, io| m.borrow_yard(io)));
        assert_eq!(state.module7.capacity, 1);
        assert_eq!(state.module7.sites, [f.yard]);
    }

    #[test]
    fn the_shipyard_plan_divides_the_budget_by_the_class_s_upkeep() {
        // FUN_004d5a50: count = w / m, here 13 / 10 = 1 under the yard's 4;
        // a class with no maintenance builds none.
        let (mut f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module7.capacity = 4;
        state.module7.room = 9;
        state.module7.target = Target::System(f.empty);
        let modest = Tallies {
            capacity: 400,
            ..ROOMY
        };
        let plan = |f: &Fixture, state: &mut AIState| {
            edited(
                f,
                state,
                modest,
                |_| {},
                |m, io| (m.plan_yards(io), m.count),
            )
        };
        assert_eq!(plan(&f, &mut state), (0xb, 1));
        f.world
            .buildable_classes
            .get_mut(&DatId::new(SHIPYARD_CLASS))
            .unwrap()
            .maintenance_cost = 0;
        assert_eq!(plan(&f, &mut state), (0, 0));
    }

    #[test]
    fn the_budgets_hold_at_their_edges() {
        // FUN_004d66a0 / FUN_004d6700: 0 only when the free maintenance is
        // below the budget; FUN_004d47b0: over only below 0.
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        // Capacity 1000: squadrons 136, yards 34.
        let edge = |requirement| Tallies {
            requirement,
            ..ROOMY
        };
        let budgets = |state: &mut AIState, t| {
            edited(
                &f,
                state,
                t,
                |_| {},
                |m, io| (squadron_budget(io), yard_budget(io, &m.sites)),
            )
        };
        assert_eq!(budgets(&mut state, edge(1000 - 136)), (136, 34));
        assert_eq!(budgets(&mut state, edge(1000 - 34)).1, 34);
        let at = Tallies {
            squadron_upkeep: 153,
            ..ROOMY
        };
        assert!(!edited(&f, &mut state, at, |_| {}, |_, io| over_budget(io)));
    }
}
