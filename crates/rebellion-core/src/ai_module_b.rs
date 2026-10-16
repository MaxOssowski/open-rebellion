//! Module `0xb` of the AI player (`FUN_004c8820`, step `FUN_004c8b30`):
//! it keeps the side's capital ships. It sends damaged ships to a shipyard
//! site and the Death Star home, builds capital ships at its shipyard
//! sites, scraps one when their upkeep is over budget, and builds shipyards
//! (`ghidra/notes/ai-production-modules.md`, "Module `0xb`"). Its sites
//! carry the shipyard mark `0x200000` ([`MARK_SHIPYARD`]).

use serde::{Deserialize, Serialize};

use crate::ai::{AIAction, MARK_SHIPYARD};
use crate::ai_module1::{
    adopt_or_request, build, claim, destination, scrap_at, scrap_spare_yard, yard_class, Inputs,
    Step, Target, Yard, ANY, SHIPYARD,
};
use crate::ai_module2::{borrow_yard, keep_site};
use crate::ai_module3::{choose_target, tenth_less_yards, CORUSCANT};
use crate::ai_records::{
    pick, picks, ranked, ship_damaged, ClassRecord, Mask, Order, SHIP_FREE, SHIP_OWN,
};
use crate::ids::{DatId, FleetKey, SystemKey};
use crate::manufacturing::{BuildableKind, ProductionArea};
use crate::scrap::ScrapTarget;

/// `+0x38` while the yard cycle runs (`FUN_004c8b30`).
pub const YARD_CYCLE: u32 = 0x3ec;

/// `+0x38` while the fleet cycle runs (`FUN_004c8b30`).
pub const FLEET_CYCLE: u32 = 0x3f8;

/// `+0x38` while the ship cycle runs (`FUN_004c8b30`).
pub const SHIP_CYCLE: u32 = 0x3f9;

/// The yard token module `0xb` holds the construction yards with
/// (controller `+0xc`, `+8`; `FUN_004c8d20` state 8).
pub const YARD_TOKEN: u32 = 2;

/// `+0x24`: the percent of the maintenance capacity module `0xb` budgets
/// (`FUN_004c8820`): 90% of it for capital ships (`FUN_004cc220`), 10% for
/// shipyards (`FUN_004cc280`).
const PERCENT: i32 = 0x1e;

/// The role bits `FUN_004ca910` clears from `+0x28` before a plan.
const TRIED: u32 = 0x7f00_0000;

/// A kind of capital ship to build (`FUN_004ca5e0`): the class record
/// flags it needs (`+0x60`), the class record value it ranks on (`+0x64`,
/// slot `+4`) and the bit that marks it tried (`+0x68`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub flags: u32,
    pub key: u32,
    pub tried: u32,
}

/// Troop carriers by troop capacity (`FUN_004ca5e0`).
const TROOP_CARRIERS: Role = Role {
    flags: 0x10_0004,
    key: 0x100,
    tried: 0x400_0000,
};
/// Armed ships by attack per maintenance (`FUN_004ca5e0`).
const ARMED: Role = Role {
    flags: 0x10_0001,
    key: 0x8_0000,
    tried: 0x100_0000,
};
/// Ships with lasers by their laser figure (`FUN_004ca5e0`).
const LASERS: Role = Role {
    flags: 0x10_0002,
    key: 0x400,
    tried: 0x200_0000,
};
/// Carriers by fighter capacity (`FUN_004ca5e0`).
const CARRIERS: Role = Role {
    flags: 0x10_0102,
    key: 0x80,
    tried: 0x800_0000,
};
/// The Death Star by attack (`FUN_004ca5e0`).
const DEATH_STAR: Role = Role {
    flags: 0x80,
    key: 0x200,
    tried: 0x2000_0000,
};

/// Module `0xb`'s own fields (`FUN_004c8820`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleB {
    /// `+0x38`: 0, [`FLEET_CYCLE`], [`SHIP_CYCLE`] or [`YARD_CYCLE`].
    pub phase: u32,
    /// `+0x3c`: the cycle's state.
    pub state: u32,
    /// `+0x28`: the roles tried ([`TRIED`]).
    pub tried: u32,
    /// `+0x50`: a site, then its shipyard manager; in the yard cycle the
    /// borrowed construction yard's manager.
    pub yard: Yard,
    /// `+0x54`: the fleet the fleet cycle works on.
    pub fleet: Option<FleetKey>,
    /// `+0x58`: the yard cycle's target.
    pub target: Target,
    /// `+0x5c` in the yard cycle: the shipyard class to build.
    pub class: Option<DatId>,
    /// `+0x5c` in the ship cycle: the ship or squadron class to build.
    pub ship: Option<BuildableKind>,
    /// `+0x60..+0x68`: the role.
    pub role: Role,
    /// `+0x6c`: how many.
    pub count: i32,
    /// `+0x70`: the borrowed yard's construction yards.
    pub capacity: i32,
    /// `+0x74`: the target's free room `v[0x33]`.
    pub room: i32,
    /// `+0x78`: the sites, marked [`MARK_SHIPYARD`].
    pub sites: Vec<SystemKey>,
    /// `+0x80`: the claimed sites.
    pub claimed: Vec<SystemKey>,
}

impl ModuleB {
    /// One step (`FUN_004c8b30`). The scheduler steps only a ready module,
    /// so the not-ready branch never runs.
    pub fn step(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.phase {
            YARD_CYCLE => self.yard_cycle(io),
            FLEET_CYCLE => self.fleet_cycle(io),
            SHIP_CYCLE => self.ship_cycle(io),
            _ => {
                self.phase = FLEET_CYCLE;
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

    /// The fleet cycle (`FUN_004c8ba0`).
    fn fleet_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            2 => {
                let order = self.repair(io);
                self.state = 3;
                Step::ordered(order)
            }
            3 => {
                let order = self.go_home(io);
                self.state = 4;
                Step::ordered(order)
            }
            4 => {
                let order = self.carry_to_base(io);
                self.end(SHIP_CYCLE, order)
            }
            _ => {
                self.state = 2;
                Step::pending()
            }
        }
    }

    /// `FUN_004c9cb0`: while controller `+4` has `0x30`, the base
    /// (`+0x40`) of the first planned sector's planner (`FUN_00419e90`:
    /// sector `+0x2c` `0x8000000`, order 2; the sector record's `+0x38`
    /// found by `FUN_005f5500`) that asks for regiments (`FUN_00476b70` >
    /// 0) and is not blockaded (`+0x28` bit 2); failing that, the first
    /// pick's base while its planner asks, blockaded or not. Then the
    /// side's fleets (`FUN_0041a4c0`: `+0x38` 5, forbidding 2, key `0xc`,
    /// order 1) in pick order, each through `+0x54`: the first whose own
    /// ships no planner holds (`FUN_0041ac80`: `0x24000`, forbidding
    /// `0x800`) carry regiments (`FUN_00504c40`) gives them all, Move
    /// `0x201` to the base. `+0x54` is left at the next fleet.
    ///
    /// port: a fleet's regiments are its cargo, not a ship's, so a fleet
    /// gives all its cargo when any of its ships passes the mask.
    fn carry_to_base(&mut self, io: &mut Inputs<'_>) -> Option<AIAction> {
        if !(io.tallies.sector_short || io.tallies.sector_invading) {
            return None;
        }
        let sectors = picks(vec![io.galaxy.planned_sectors(
            [ANY, ANY, Mask::new(0x800_0000, 0)],
            Order::Descending,
            io.coin,
        )]);
        let base_of = |io: &Inputs<'_>, sector| {
            let id = io.galaxy.sector(sector)?.planner;
            let planner = io.planners.iter().find(|p| id != 0 && p.id == id)?;
            (planner.request(io.galaxy) > 0).then_some(planner.base?)
        };
        let open = |io: &Inputs<'_>, base| io.galaxy.system(base).is_some_and(|r| r.state & 2 == 0);
        let base = sectors
            .iter()
            .find_map(|&s| base_of(io, s).filter(|&b| open(io, b)))
            .or_else(|| base_of(io, *sectors.first()?))?;
        let list = picks(vec![fleets(io, Mask::new(5, 2), 0xc, Order::Ascending)]);
        let ships = Mask::new(SHIP_FREE | SHIP_OWN, 0x800);
        let mut troops = Vec::new();
        self.fleet = None;
        for (at, &fleet) in list.iter().enumerate() {
            let Some(record) = io.galaxy.fleets.get(&fleet) else {
                continue;
            };
            if record.ships.iter().any(|r| ships.admits(r.flags)) {
                troops.extend_from_slice(io.transport.cargo(fleet));
            }
            if !troops.is_empty() {
                self.fleet = list.get(at + 1).copied();
                break;
            }
        }
        (!troops.is_empty()).then_some(AIAction::MoveRegiments { troops, to: base })
    }

    /// `FUN_004c8e90`: the damaged ships of a fleet go to the nearest
    /// shipyard site (`+0x28` `0x200000`) without a blockade, else to the
    /// nearest own system without one. The fleet is the side's with a
    /// damaged ship that can move (`0x1005`), not en route (`0x802`), by
    /// its ships (key 0, order 1): first one at an opponent's system
    /// (`+0x30` 4), else one at a system that is no shipyard site.
    fn repair(&mut self, io: &mut Inputs<'_>) -> Option<AIAction> {
        let masks = Mask::new(0x1005, 0x802);
        let at_war = |io: &Inputs<'_>, fleet| record_at(io, fleet).is_some_and(|r| r.kind & 4 != 0);
        let unrepaired = |io: &Inputs<'_>, fleet| {
            record_at(io, fleet).is_some_and(|r| r.state & MARK_SHIPYARD == 0)
        };
        let first = picks(vec![fleets(io, masks, 0, Order::Ascending)]);
        self.fleet = first.into_iter().find(|&f| at_war(io, f));
        if self.fleet.is_none() {
            let second = picks(vec![fleets(io, masks, 0, Order::Ascending)]);
            self.fleet = second.into_iter().find(|&f| unrepaired(io, f));
        }
        let fleet = self.fleet?;
        let value = io.world.fleets.get(fleet)?;
        let ships: Vec<usize> = value
            .capital_ships
            .iter()
            .enumerate()
            .filter(|(_, s)| ship_damaged(&io.galaxy.classes, s))
            .map(|(i, _)| i)
            .collect();
        if ships.is_empty() {
            return None;
        }
        let here = value.location;
        let mut to = nearest(
            io,
            here,
            Mask::new(0x2_0000, 0),
            [Mask::new(MARK_SHIPYARD, 2), ANY, ANY],
        );
        if to.is_none() {
            to = nearest(
                io,
                here,
                Mask::new(4, 0),
                [Mask::new(0, 2), ANY, Mask::new(1, 0)],
            );
        }
        Some(AIAction::MoveShips {
            fleet,
            ships,
            to: to?,
        })
    }

    /// `FUN_004c94c0`: the side's fleets not en route and with no damaged
    /// ship (`5`, forbidding `0x3802`), fewest ships first (key 0, order
    /// 2), until one gives an order: a fleet with the Death Star (`0x10`)
    /// goes to Coruscant (`0x90000109`) while it is the side's.
    ///
    /// port: the move to a base (`+0x30` `0x40000`) is left out. Of the
    /// plan objects that set the bases only kind 3's (`FUN_004af8a0`) is
    /// ported, not `FUN_004b24b0`, `FUN_004b5b20`, `FUN_004b8070`, and
    /// without a base the original's fallback sends every idle fleet to
    /// another sector at each step. No
    /// ship is inactive (record `0x2000`), so no fleet takes Create Fleet
    /// (`0x270`).
    fn go_home(&mut self, io: &mut Inputs<'_>) -> Option<AIAction> {
        let coruscant = io
            .world
            .systems
            .iter()
            .find(|(_, s)| s.dat_id.raw() == CORUSCANT)
            .map(|(k, _)| k)
            .filter(|&c| io.galaxy.system(c).is_some_and(|r| r.kind & 1 != 0));
        for fleet in picks(vec![fleets(io, Mask::new(5, 0x3802), 0, Order::Descending)]) {
            self.fleet = Some(fleet);
            let has_death_star = io
                .galaxy
                .fleets
                .get(&fleet)
                .is_some_and(|r| r.flags & 0x10 != 0);
            let Some(value) = io.world.fleets.get(fleet) else {
                continue;
            };
            if record_at(io, fleet).is_none() || !has_death_star {
                continue;
            }
            if let Some(home) = coruscant.filter(|&c| c != value.location) {
                let ships = value
                    .capital_ships
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.alive)
                    .map(|(i, _)| i)
                    .collect();
                return Some(AIAction::MoveShips {
                    fleet,
                    ships,
                    to: home,
                });
            }
        }
        self.fleet = None;
        None
    }

    /// The ship cycle (`FUN_004c8c20`).
    fn ship_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            8 => {
                self.state =
                    if keep_site(io, &mut self.sites, &mut self.yard, MARK_SHIPYARD, SHIPYARD) {
                        5
                    } else {
                        0xd
                    };
                Step::pending()
            }
            5 => {
                self.choose_role(io);
                self.state = 9;
                Step::pending()
            }
            9 => {
                self.state = self.plan_ships(io);
                if self.state == 0 {
                    self.phase = YARD_CYCLE;
                    return Step::complete();
                }
                Step::pending()
            }
            0xd => {
                self.state = 0;
                if over_budget(io.tallies) < 0 {
                    self.state = 10;
                } else {
                    self.phase = YARD_CYCLE;
                }
                Step::pending()
            }
            10 => {
                let order = scrap_ship(io);
                self.end(YARD_CYCLE, order)
            }
            0xf => {
                let order = self.build_ships(io);
                self.end(YARD_CYCLE, order)
            }
            _ => {
                self.state = 8;
                Step::pending()
            }
        }
    }

    /// `FUN_004ca5e0`: the role with the most requests of module 8's
    /// sector planners not yet tried; with none, troop carriers while the
    /// fleets' troop room (`+0x260`) is below the most regiments a system
    /// needs (`+0x20c`); the Death Star for the Empire with two sites or
    /// more and none in its fleets (controller `+4` bit 4); armed ships
    /// with none (`+0x268`); ships with lasers with none (`+0x26c`);
    /// carriers while the hangar room (`+0x27c`) is at most controller
    /// `+0x16c`; else no role.
    ///
    /// port: module 8's sector planners (controller `+0xd8`) are not
    /// ported, so no request counts and the bombardment and interdictor
    /// roles never come up. hyp: no writer of controller `+0x16c` is found;
    /// it reads as 0.
    fn choose_role(&mut self, io: &Inputs<'_>) {
        let t = io.tallies;
        let open = |role: Role| self.tried & role.tried == 0;
        self.role = if open(TROOP_CARRIERS) && t.troop_room < t.troops_wanted {
            TROOP_CARRIERS
        } else if !io.is_alliance && self.sites.len() > 1 && !t.death_star && open(DEATH_STAR) {
            DEATH_STAR
        } else if open(ARMED) && t.armed < 1 {
            ARMED
        } else if open(LASERS) && t.lasers < 1 {
            LASERS
        } else if open(CARRIERS) && t.hangars <= 0 {
            CARRIERS
        } else {
            Role {
                tried: self.role.tried,
                ..Role::default()
            }
        };
    }

    /// `FUN_004ca910`: with the ships' budget `w` (`FUN_004cc220`), 10 to
    /// scrap a ship when `w < 0`, 0 when `w == 0`; else the best class of
    /// the role the side's shipyards list (`FUN_0041b270(0x28, +0x60,
    /// +0x64, 1)`) whose maintenance is at most `w`, one of it, `0xf`.
    /// With none the role is tried, 0. The tried roles clear first.
    fn plan_ships(&mut self, io: &mut Inputs<'_>) -> u32 {
        self.tried &= !TRIED;
        self.count = 0;
        let budget = ship_budget(io.tallies);
        if budget < 0 {
            return 10;
        }
        if budget == 0 {
            return 0;
        }
        let role = self.role;
        let classes: Vec<_> = crate::build_selection::listed_classes(
            io.world,
            io.research,
            ProductionArea::Shipyard,
            io.is_alliance,
        )
        .into_iter()
        .filter_map(|kind| {
            let (r, value) = match kind {
                BuildableKind::CapitalShip(c) => {
                    let r = io.galaxy.classes.ship(c)?;
                    (r, ship_value(r, role.key))
                }
                // FUN_00473640 holds none of the roles' keys.
                BuildableKind::Fighter(c) => (io.galaxy.classes.fighter(c)?, 0),
                _ => return None,
            };
            (r.flags & role.flags == role.flags).then_some((kind, value))
        })
        .collect();
        let list = ranked(classes, Order::Ascending, io.coin);
        self.ship = picks(vec![list])
            .into_iter()
            .find(|&kind| maintenance(io, kind) <= budget);
        if self.ship.is_some() {
            self.count = 1;
            return 0xf;
        }
        self.tried |= role.tried;
        0
    }

    /// `FUN_004cba90`: Build `0x211` of the class on the site's shipyard
    /// manager.
    fn build_ships(&self, io: &Inputs<'_>) -> Option<AIAction> {
        let Yard::Manager(site) = self.yard else {
            return None;
        };
        let kind = self.ship?;
        let cost = match kind {
            BuildableKind::CapitalShip(c) => io
                .world
                .capital_ship_classes
                .get(c)
                .map_or(0, |c| c.refined_material_cost),
            BuildableKind::Fighter(c) => io
                .world
                .fighter_classes
                .get(c)
                .map_or(0, |c| c.refined_material_cost),
            _ => 0,
        };
        Some(AIAction::Build {
            system: site,
            kind,
            cost,
            count: self.count,
        })
    }

    /// The yard cycle (`FUN_004c8d20`): module 7's (`FUN_004d3350`) with
    /// the token 2 and module 2's borrow.
    fn yard_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            7 => {
                if choose_target(
                    io,
                    &mut self.sites,
                    &mut self.target,
                    &mut self.room,
                    MARK_SHIPYARD,
                    SHIPYARD,
                ) {
                    self.state = 8;
                    Step::pending()
                } else {
                    self.end(FLEET_CYCLE, None)
                }
            }
            8 => {
                io.controller.yard_requests &= !YARD_TOKEN;
                if io.controller.yard_holder == YARD_TOKEN
                    && borrow_yard(
                        io,
                        self.target,
                        &mut self.sites,
                        &mut self.yard,
                        &mut self.capacity,
                        MARK_SHIPYARD,
                    )
                {
                    io.controller.pass_yards();
                    self.state = 0xb;
                } else {
                    self.state = 0x10;
                }
                Step::pending()
            }
            0xb => {
                let order = scrap_at(io, self.target.system(), SHIPYARD);
                self.state = 9;
                Step::ordered(order)
            }
            9 => {
                self.state = self.plan_yards(io);
                if self.state == 0 {
                    return self.end(FLEET_CYCLE, None);
                }
                Step::pending()
            }
            0xc => {
                let order = scrap_spare_yard(
                    io,
                    &mut self.sites,
                    &mut self.target,
                    MARK_SHIPYARD,
                    SHIPYARD,
                );
                self.end(FLEET_CYCLE, order)
            }
            0xe => match destination(self.target, self.yard) {
                Some(order) => {
                    self.state = 0xf;
                    Step {
                        order: Some(order),
                        complete: true,
                        raise: true,
                    }
                }
                None => self.end(FLEET_CYCLE, None),
            },
            0xf => {
                let order = build(io, self.yard, self.class, self.count);
                self.end(FLEET_CYCLE, order)
            }
            0x10 => {
                let budget = tenth_less_yards(io, &self.sites, PERCENT);
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
                self.end(FLEET_CYCLE, None)
            }
            _ => {
                self.state = 7;
                Step::pending()
            }
        }
    }

    /// `FUN_004cb710` (module 7's `FUN_004d5a50`): `0xe` to build
    /// shipyards, `0xc` to scrap one when the sites hold more than the side
    /// wants (`FUN_004cc280`), else 0.
    fn plan_yards(&mut self, io: &mut Inputs<'_>) -> u32 {
        self.count = 0;
        let budget = tenth_less_yards(io, &self.sites, PERCENT);
        if budget < 0 {
            return 0xc;
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
        0xe
    }
}

/// The record of the system `fleet` stands at (`FUN_00403d30` on the fleet
/// record's `+0x28`).
fn record_at<'a>(
    io: &'a Inputs<'_>,
    fleet: FleetKey,
) -> Option<&'a crate::ai_records::SystemRecord> {
    let location = io.world.fleets.get(fleet)?.location;
    io.galaxy.system(location)
}

/// `FUN_0041a4c0`: the fleets whose record passes `masks` (`+0x38`), keyed
/// by `values[key]`, in the world's fleet order.
///
/// port: a fleet of squadrons alone (the port's seeding) holds no capital
/// ship, which no fleet of the original lacks; the queries pass over it.
fn fleets(io: &mut Inputs<'_>, masks: Mask, key: usize, order: Order) -> Vec<FleetKey> {
    let keyed: Vec<_> = io
        .world
        .fleets
        .iter()
        .filter(|(_, f)| !f.capital_ships.is_empty())
        .filter_map(|(fleet, _)| {
            let r = io.galaxy.fleets.get(&fleet)?;
            masks.admits(r.flags).then_some((fleet, r.values[key]))
        })
        .collect();
    ranked(keyed, order, io.coin)
}

/// The system nearest `here` in the nearest sector whose summary
/// (`+0x2c`) passes `sector` and with no blockade (`+0x24` 8), passing
/// `masks` (`FUN_00419b80`, `FUN_0041a400`, both order 2).
fn nearest(
    io: &mut Inputs<'_>,
    here: SystemKey,
    sector: Mask,
    masks: [Mask; 3],
) -> Option<SystemKey> {
    let sectors = io.galaxy.sectors_by_distance(
        here,
        [Mask::new(8, 0), ANY, sector],
        Order::Descending,
        io.coin,
    );
    let sector = pick(&[sectors])?;
    let systems = io
        .galaxy
        .systems_by_distance(sector, here, masks, Order::Descending, io.coin);
    pick(&[systems])
}

/// The ship class record's slot `+4` (`FUN_00473930`) for the roles' keys.
fn ship_value(r: &ClassRecord, key: u32) -> i32 {
    match key {
        0x200 => r.values[0],
        0x8_0000 => r.values[10],
        0x400 => r.values[1],
        0x80 => r.values[5],
        0x100 => r.values[6],
        _ => 0,
    }
}

/// A class's maintenance (record `+0x20`, `FUN_0042e190`).
fn maintenance(io: &Inputs<'_>, kind: BuildableKind) -> i32 {
    match kind {
        BuildableKind::CapitalShip(c) => io.galaxy.classes.ship(c).map_or(0, |r| r.maintenance),
        BuildableKind::Fighter(c) => io.galaxy.classes.fighter(c).map_or(0, |r| r.maintenance),
        _ => 0,
    }
}

/// `FUN_004caad0`: 90% of `+0x24` percent of the capacity, less the
/// capital ships' upkeep (`+0x24c`); the ships are over budget below 0.
fn over_budget(t: crate::ai_module1::Tallies) -> i32 {
    PERCENT * t.capacity / 100 * 90 / 100 - t.ship_upkeep
}

/// `FUN_004cc220`: [`over_budget`]'s figure, 0 when the side's free
/// maintenance falls short of it.
fn ship_budget(t: crate::ai_module1::Tallies) -> i32 {
    let budget = over_budget(t);
    if t.capacity - t.requirement < budget {
        0
    } else {
        budget
    }
}

/// `FUN_004cab20`: Scrap `0x200` of the side's fleet not en route with the
/// least upkeep (`5`, forbidding `0x402`, key `0xe`, order 2), its living
/// ship with the least maintenance (`FUN_0041ac80`: `0x24000`, forbidding
/// `0x2800`, key `0xb`, order 2).
fn scrap_ship(io: &mut Inputs<'_>) -> Option<AIAction> {
    let fleet = pick(&[fleets(io, Mask::new(5, 0x402), 0xe, Order::Descending)])?;
    let value = io.world.fleets.get(fleet)?;
    let ships: Vec<_> = value
        .capital_ships
        .iter()
        .enumerate()
        .filter(|(_, s)| s.alive)
        .map(|(i, s)| {
            let upkeep = io.galaxy.classes.ship(s.class).map_or(0, |r| r.maintenance);
            (i, upkeep)
        })
        .collect();
    let index = pick(&[ranked(ships, Order::Descending, io.coin)])?;
    Some(AIAction::Scrap {
        team: vec![ScrapTarget::Ship { fleet, index }],
    })
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
    use crate::ids::CapitalShipKey;
    use crate::research::ResearchState;
    use crate::world::{CapitalShipClass, ControlKind, Fleet, ShipInstance};

    const SHIPYARD_CLASS: u32 = 0x2800_0001;

    fn with_io<R>(
        f: &Fixture,
        state: &mut AIState,
        galaxy: &mut Galaxy,
        tallies: Tallies,
        is_alliance: bool,
        transport: &crate::troop_transport::TroopTransportState,
        run: impl FnOnce(&mut ModuleB, &mut Inputs<'_>) -> R,
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
            is_alliance,
            planners: &state.module8.planners,
            transport,
            coin: &mut coin,
        };
        run(&mut state.module_b, &mut io)
    }

    fn edited<R>(
        f: &Fixture,
        state: &mut AIState,
        tallies: Tallies,
        edit: impl FnOnce(&mut Galaxy),
        run: impl FnOnce(&mut ModuleB, &mut Inputs<'_>) -> R,
    ) -> R {
        let mut galaxy = records_of(f, state);
        edit(&mut galaxy);
        let transport = crate::troop_transport::TroopTransportState::default();
        with_io(f, state, &mut galaxy, tallies, true, &transport, run)
    }

    fn step(f: &Fixture, state: &mut AIState, tallies: Tallies) -> Step {
        edited(f, state, tallies, |_| {}, |m, io| m.step(io))
    }

    fn rec(g: &mut Galaxy, s: SystemKey) -> &mut SystemRecord {
        record(g, s)
    }

    struct Hulls {
        /// Troop capacity 6 (flags `0x100204`), maintenance 4, cost 20.
        trooper: CapitalShipKey,
        /// Troop capacity 10, maintenance 500.
        big_trooper: CapitalShipKey,
        /// Turbolasers (flags `0x100001`), maintenance 9, cost 30.
        gunship: CapitalShipKey,
    }

    /// The fixture with an Alliance shipyard at `yard` and three Alliance
    /// capital ship classes of hull 100.
    fn ship_fixture() -> (Fixture, Hulls) {
        let mut f = fixture();
        let shipyard = class(&mut f.world, SHIPYARD_CLASS, 10, 2, 0);
        add_yard(&mut f.world, f.yard, shipyard, Faction::Alliance);
        let mut hull = |c: CapitalShipClass| {
            f.world.capital_ship_classes.insert(CapitalShipClass {
                is_alliance: true,
                hull: 100,
                ..c
            })
        };
        let trooper = hull(CapitalShipClass {
            troop_capacity: 6,
            maintenance_cost: 4,
            refined_material_cost: 20,
            ..CapitalShipClass::default()
        });
        let big_trooper = hull(CapitalShipClass {
            troop_capacity: 10,
            maintenance_cost: 500,
            ..CapitalShipClass::default()
        });
        let gunship = hull(CapitalShipClass {
            turbolaser_fore: 3,
            weapon_recharge_rate: 2,
            maintenance_cost: 9,
            refined_material_cost: 30,
            ..CapitalShipClass::default()
        });
        (
            f,
            Hulls {
                trooper,
                big_trooper,
                gunship,
            },
        )
    }

    /// An Alliance fleet at `at` of ships with these hulls left.
    fn fleet(f: &mut Fixture, at: SystemKey, ships: &[(CapitalShipKey, i32)]) -> FleetKey {
        let fleet = f.world.fleets.insert(Fleet {
            location: at,
            capital_ships: ships
                .iter()
                .map(|&(class, hull)| ShipInstance::new(class, hull, true))
                .collect(),
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        f.world.systems[at].fleets.push(fleet);
        fleet
    }

    fn at(state: &mut AIState, phase: u32, step: u32) {
        state.module_b.phase = phase;
        state.module_b.state = step;
    }

    #[test]
    fn the_cycles_hand_on_from_fleets_to_ships_to_yards() {
        // FUN_004c8b30: any other +0x38 starts 0x3f8. FUN_004c8ba0 2 and 3
        // find no fleet; 4 ends with 0x3f9, complete. FUN_004c8c20 8 keeps
        // no site: 0xd (FUN_004caad0, within budget) hands on to 0x3ec
        // without completing. FUN_004c8d20 7 takes the yard as the target
        // (module 1's mark is not forbidden, 0x200003); 8 without the token
        // goes to 0x10, which ends with 0x3f8, complete.
        let (f, _) = ship_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.yard_marks.insert(f.empty, MARK_MINES);
        let mut trail = Vec::new();
        for _ in 0..14 {
            let s = step(&f, &mut state, ROOMY);
            trail.push((state.module_b.phase, state.module_b.state, s.complete));
        }
        assert_eq!(
            trail,
            [
                (FLEET_CYCLE, 0, false),
                (FLEET_CYCLE, 2, false),
                (FLEET_CYCLE, 3, false),
                (FLEET_CYCLE, 4, false),
                (SHIP_CYCLE, 0, true),
                (SHIP_CYCLE, 8, false),
                (SHIP_CYCLE, 0xd, false),
                (YARD_CYCLE, 0, false),
                (YARD_CYCLE, 7, false),
                (YARD_CYCLE, 8, false),
                (YARD_CYCLE, 0x10, false),
                (FLEET_CYCLE, 0, true),
                (FLEET_CYCLE, 2, false),
                (FLEET_CYCLE, 3, false),
            ]
        );
        state.module_b.refused();
        assert_eq!((state.module_b.phase, state.module_b.state), (0, 0));
    }

    #[test]
    fn damaged_ships_leave_their_fleet_for_the_nearest_shipyard_site() {
        // FUN_004c8e90: the fleet with a damaged ship that can move
        // (record 0x1005: FUN_00484630 sets 0x1000 for object +0x50 bit
        // 0x200 with a speed) at a system that is no site; its ships with
        // 0x1000 go to the site nearest it (+0x28 0x200000, no blockade),
        // Move 0x201, complete and raise.
        let (mut f, h) = ship_fixture();
        let here = f.empty;
        let damaged = fleet(&mut f, here, &[(h.gunship, 100), (h.gunship, 40)]);
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_SHIPYARD);
        at(&mut state, FLEET_CYCLE, 2);
        let s = step(&f, &mut state, ROOMY);
        assert_eq!(
            s.order,
            Some(AIAction::MoveShips {
                fleet: damaged,
                ships: vec![1],
                to: f.yard,
            })
        );
        assert!(s.complete && s.raise);
        assert_eq!(
            (state.module_b.state, state.module_b.fleet),
            (3, Some(damaged))
        );

        // A fleet already at a site stays to be repaired.
        let (mut f, h) = ship_fixture();
        let site = f.yard;
        fleet(&mut f, site, &[(h.gunship, 40)]);
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_SHIPYARD);
        at(&mut state, FLEET_CYCLE, 2);
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module_b.fleet, None);
    }

    #[test]
    fn a_damaged_fleet_at_an_opponent_s_system_goes_first_to_the_nearest_own_system() {
        // FUN_004c8e90 walks the fleets most ships first (key 0, order 1)
        // for one at an opponent's system (+0x30 4) before one at a system
        // that is no site. With no site the ships go to the nearest own
        // system without a blockade (sector +0x2c 4, system +0x30 1).
        let (mut f, h) = ship_fixture();
        let (sector, here) = (f.sector, f.empty);
        let enemy = system(&mut f.world, sector, 4);
        f.world.systems[enemy].control = ControlKind::Controlled(Faction::Empire);
        fleet(&mut f, here, &[(h.gunship, 40), (h.gunship, 40)]);
        let raider = fleet(&mut f, enemy, &[(h.gunship, 40)]);
        let mut state = AIState::new(AiFaction::Alliance);
        at(&mut state, FLEET_CYCLE, 2);
        assert_eq!(
            step(&f, &mut state, ROOMY).order,
            Some(AIAction::MoveShips {
                fleet: raider,
                ships: vec![0],
                to: f.empty,
            })
        );
    }

    #[test]
    fn the_death_star_goes_home_to_coruscant_while_it_is_the_side_s() {
        // FUN_004c94c0: a fleet whose record has the Death Star bit 0x10,
        // not at Coruscant (0x90000109), Moves there while Coruscant's
        // record is the side's (+0x30 1). The fleets are walked fewest
        // ships first; a fleet without the Death Star stays.
        let (mut f, h) = ship_fixture();
        f.world.systems[f.yard].dat_id = DatId::new(CORUSCANT);
        let death_star = f.world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(crate::world::DEATH_STAR_CLASS_ID),
            is_alliance: true,
            hull: 100,
            ..CapitalShipClass::default()
        });
        let here = f.empty;
        fleet(&mut f, here, &[(h.gunship, 100)]);
        let ds = fleet(&mut f, here, &[(death_star, 100), (h.gunship, 100)]);
        let mut state = AIState::new(AiFaction::Alliance);
        at(&mut state, FLEET_CYCLE, 3);
        let s = step(&f, &mut state, ROOMY);
        assert_eq!(
            s.order,
            Some(AIAction::MoveShips {
                fleet: ds,
                ships: vec![0, 1],
                to: f.yard,
            })
        );
        assert!(s.complete && s.raise);
        assert_eq!(state.module_b.state, 4);

        // Not while Coruscant is another side's.
        let mut state = AIState::new(AiFaction::Alliance);
        at(&mut state, FLEET_CYCLE, 3);
        let lost = |g: &mut Galaxy| rec(g, f.yard).kind &= !1;
        let s = edited(&f, &mut state, ROOMY, lost, |m, io| m.step(io));
        assert_eq!(s, Step::pending());
        assert_eq!(state.module_b.fleet, None);
    }

    /// A regiment at `at` loaded onto `fleet`.
    fn carried(
        f: &mut Fixture,
        transport: &mut crate::troop_transport::TroopTransportState,
        fleet: FleetKey,
    ) -> crate::ids::TroopKey {
        let at = f.world.fleets[fleet].location;
        let troop = f.world.troops.insert(crate::world::TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance: true,
            regiment_strength: 100,
        });
        f.world.systems[at].ground_units.push(troop);
        transport.embark(&mut f.world, fleet, &[troop]).unwrap();
        troop
    }

    fn base_planner(id: u32, base: SystemKey) -> crate::ai_module8::SectorPlanner {
        crate::ai_module8::SectorPlanner {
            id,
            kind: 4,
            base: Some(base),
            plan_target: Some(base),
            ..Default::default()
        }
    }

    /// The sector record at `i` planned (`+0x2c` `0x8000000`) by planner
    /// `id`, ranked by `plan_id`, and `base` asking for 3 (`v[20]`).
    fn planned(g: &mut Galaxy, i: usize, id: u32, base: SystemKey) {
        let s = &mut g.sectors[i];
        s.summary |= 0x800_0000;
        (s.planner, s.plan_id) = (id, id);
        rec(g, base).values[20] = 3;
    }

    fn run_carry(
        f: &Fixture,
        state: &mut AIState,
        transport: &crate::troop_transport::TroopTransportState,
        tallies: Tallies,
        edit: impl FnOnce(&mut Galaxy),
    ) -> Option<AIAction> {
        let mut galaxy = records_of(f, state);
        edit(&mut galaxy);
        with_io(f, state, &mut galaxy, tallies, true, transport, |m, io| {
            m.carry_to_base(io)
        })
    }

    const SHORT: Tallies = Tallies {
        sector_short: true,
        ..ROOMY
    };

    #[test]
    fn carried_regiments_go_to_the_base_of_a_planner_that_asks() {
        // FUN_004c9cb0 (fleet cycle state 4): with controller +4 0x30, the
        // planned sector (+0x2c 0x8000000) whose planner (+0x38) asks
        // (FUN_00476b70 > 0) names its base +0x40; the first fleet whose
        // free own ships (0x24000, forbidding 0x800) carry regiments gives
        // them all: Move 0x201 to the base. The cycle then ends with
        // 0x3f9.
        let (mut f, h) = ship_fixture();
        let mut transport = crate::troop_transport::TroopTransportState::default();
        let yard = f.yard;
        let ship = fleet(&mut f, yard, &[(h.trooper, 100)]);
        let first = carried(&mut f, &mut transport, ship);
        let second = carried(&mut f, &mut transport, ship);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module8.planners.push(base_planner(1, f.empty));
        at(&mut state, FLEET_CYCLE, 4);
        let mut galaxy = records_of(&f, &state);
        planned(&mut galaxy, 0, 1, f.empty);
        let s = with_io(
            &f,
            &mut state,
            &mut galaxy,
            SHORT,
            true,
            &transport,
            |m, io| m.step(io),
        );
        let both = Some(AIAction::MoveRegiments {
            troops: vec![first, second],
            to: f.empty,
        });
        assert_eq!(s.order, both);
        assert!(s.complete);
        assert_eq!(
            (state.module_b.phase, state.module_b.state),
            (SHIP_CYCLE, 0)
        );
        assert_eq!(state.module_b.fleet, None);

        // Either controller bit serves.
        let invading = Tallies {
            sector_invading: true,
            ..ROOMY
        };
        let got = run_carry(&f, &mut state, &transport, invading, |g| {
            planned(g, 0, 1, f.empty);
        });
        assert_eq!(got, both);

        // Nothing without the bits, without the sector bit, with a planner
        // asking for nothing, or with the ships held by a planner.
        let got = run_carry(&f, &mut state, &transport, ROOMY, |g| {
            planned(g, 0, 1, f.empty);
        });
        assert_eq!(got, None);
        let got = run_carry(&f, &mut state, &transport, SHORT, |g| {
            planned(g, 0, 1, f.empty);
            g.sectors[0].summary &= !0x800_0000;
        });
        assert_eq!(got, None);
        let got = run_carry(&f, &mut state, &transport, SHORT, |g| {
            planned(g, 0, 1, f.empty);
            rec(g, f.empty).values[20] = 0;
        });
        assert_eq!(got, None);
        let got = run_carry(&f, &mut state, &transport, SHORT, |g| {
            planned(g, 0, 2, f.empty);
        });
        assert_eq!(got, None);
        let got = run_carry(&f, &mut state, &transport, SHORT, |g| {
            planned(g, 0, 1, f.empty);
            for r in &mut g.fleets.get_mut(&ship).unwrap().ships {
                r.flags &= !crate::ai_records::SHIP_FREE;
            }
        });
        assert_eq!(got, None);
        // A fleet without regiments gives nothing.
        let got = run_carry(&f, &mut state, &Default::default(), SHORT, |g| {
            planned(g, 0, 1, f.empty);
        });
        assert_eq!(got, None);
    }

    #[test]
    fn the_first_unblockaded_base_wins_and_else_the_first_pick_s() {
        // FUN_004c9cb0 walks the picks (order 2: the least plan id first)
        // until a base's record lacks +0x28 bit 2; without one it takes
        // the first pick's base (0x4c9ef0) while its planner asks.
        let (mut f, h) = ship_fixture();
        let far_sector = f.world.sectors.insert(crate::world::Sector {
            dat_id: DatId::new(0x8000_0002),
            name: "Far".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let far = system(&mut f.world, far_sector, 9);
        f.world.sectors[far_sector].systems.push(far);
        let mut transport = crate::troop_transport::TroopTransportState::default();
        let yard = f.yard;
        let ship = fleet(&mut f, yard, &[(h.trooper, 100)]);
        let troop = carried(&mut f, &mut transport, ship);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module8.planners.push(base_planner(1, f.empty));
        state.module8.planners.push(base_planner(2, far));
        let to = |to| {
            Some(AIAction::MoveRegiments {
                troops: vec![troop],
                to,
            })
        };
        let two = |g: &mut Galaxy| {
            planned(g, 0, 1, f.empty);
            let i = g
                .sectors
                .iter()
                .position(|s| s.sector == far_sector)
                .unwrap();
            planned(g, i, 2, far);
        };
        let got = run_carry(&f, &mut state, &transport, SHORT, two);
        assert_eq!(got, to(f.empty));
        let got = run_carry(&f, &mut state, &transport, SHORT, |g| {
            two(g);
            rec(g, f.empty).state |= 2;
        });
        assert_eq!(got, to(far));
        let got = run_carry(&f, &mut state, &transport, SHORT, |g| {
            two(g);
            rec(g, f.empty).state |= 2;
            rec(g, far).state |= 2;
        });
        assert_eq!(got, to(f.empty));
        // The fallback asks the first pick's planner again.
        let got = run_carry(&f, &mut state, &transport, SHORT, |g| {
            two(g);
            rec(g, f.empty).values[20] = 0;
            rec(g, far).state |= 2;
        });
        assert_eq!(got, None);
    }

    #[test]
    fn the_fleets_go_by_their_key_and_the_cycle_keeps_the_next() {
        // FUN_004c9cb0: FUN_0041a4c0(5, 2, 0xc) order 1, picked from the
        // last (the highest values[12] first); the first fleet with
        // regiments gives them, and +0x54 holds the next pick.
        let (mut f, h) = ship_fixture();
        let mut transport = crate::troop_transport::TroopTransportState::default();
        let yard = f.yard;
        let low = fleet(&mut f, yard, &[(h.trooper, 100)]);
        let high = fleet(&mut f, yard, &[(h.trooper, 100)]);
        let empty = fleet(&mut f, yard, &[(h.trooper, 100)]);
        let low_troop = carried(&mut f, &mut transport, low);
        let high_troop = carried(&mut f, &mut transport, high);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module8.planners.push(base_planner(1, f.empty));
        let keys = |g: &mut Galaxy, k: [i32; 3]| {
            planned(g, 0, 1, f.empty);
            for (fleet, k) in [low, high, empty].into_iter().zip(k) {
                g.fleets.get_mut(&fleet).unwrap().values[0xc] = k;
            }
        };
        let got = run_carry(&f, &mut state, &transport, SHORT, |g| keys(g, [1, 3, 2]));
        assert_eq!(
            got,
            Some(AIAction::MoveRegiments {
                troops: vec![high_troop],
                to: f.empty,
            })
        );
        assert_eq!(state.module_b.fleet, Some(empty));
        let got = run_carry(&f, &mut state, &transport, SHORT, |g| keys(g, [3, 1, 2]));
        assert_eq!(
            got,
            Some(AIAction::MoveRegiments {
                troops: vec![low_troop],
                to: f.empty,
            })
        );
        assert_eq!(state.module_b.fleet, Some(empty));
        // A fleet of the other side (record +0x38 without 5) is passed
        // over.
        let got = run_carry(&f, &mut state, &transport, SHORT, |g| {
            keys(g, [1, 3, 2]);
            g.fleets.get_mut(&high).unwrap().flags &= !4;
        });
        assert_eq!(
            got,
            Some(AIAction::MoveRegiments {
                troops: vec![low_troop],
                to: f.empty,
            })
        );
        assert_eq!(state.module_b.fleet, None);
    }

    #[test]
    fn the_ship_cycle_keeps_a_shipyard_site_and_builds_the_role_s_best_affordable_class() {
        // FUN_004c8c20: 8 (FUN_004ca300 keeps the site, v[4], idle 0x200)
        // -> 5 (FUN_004ca5e0: troop room 0 below the 3 a system needs) -> 9
        // (FUN_004ca910: the troop carriers by troop capacity, the best
        // whose maintenance 500 is over the budget passed over) -> 0xf
        // (FUN_004cba90: Build 0x211 of one on the shipyard manager) ->
        // end with 0x3ec.
        let (f, h) = ship_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_b.sites.push(f.yard);
        state.yard_marks.insert(f.yard, MARK_SHIPYARD);
        at(&mut state, SHIP_CYCLE, 8);
        let t = Tallies {
            troops_wanted: 3,
            ..ROOMY
        };
        let mut trail = Vec::new();
        for _ in 0..4 {
            let s = step(&f, &mut state, t);
            trail.push((state.module_b.state, s));
        }
        assert_eq!(state.module_b.role, TROOP_CARRIERS);
        assert_eq!(
            trail.iter().map(|(s, _)| *s).collect::<Vec<_>>(),
            [5, 9, 0xf, 0]
        );
        assert_eq!(
            trail[3].1.order,
            Some(AIAction::Build {
                system: f.yard,
                kind: BuildableKind::CapitalShip(h.trooper),
                cost: 20,
                count: 1,
            })
        );
        assert!(trail[3].1.complete);
        assert_eq!(state.module_b.phase, YARD_CYCLE);
        let _ = h.big_trooper;
    }

    #[test]
    fn the_role_falls_back_from_troops_to_the_death_star_armed_lasers_and_carriers() {
        // FUN_004ca5e0 with no planner requests: troop carriers while
        // +0x260 < +0x20c; the Death Star for the Empire (+0x30 == 2) with
        // two sites and none in its fleets (+4 bit 4); armed ships with
        // +0x268 < 1; lasers with +0x26c < 1; carriers with +0x27c <= 0.
        // A tried role is passed over; with none the tried bit stays.
        let (f, _) = ship_fixture();
        let mut state = AIState::new(AiFaction::Empire);
        state.module_b.sites = vec![f.yard, f.empty];
        let role = |state: &mut AIState, t: Tallies, alliance: bool| {
            let mut galaxy = records_of(&f, state);
            with_io(
                &f,
                state,
                &mut galaxy,
                t,
                alliance,
                &Default::default(),
                |m, io| {
                    m.choose_role(io);
                    m.role
                },
            )
        };
        let short = Tallies {
            troops_wanted: 1,
            hangars: 1,
            armed: 1,
            lasers: 1,
            ..ROOMY
        };
        assert_eq!(role(&mut state, short, false), TROOP_CARRIERS);
        let enough = Tallies {
            troop_room: 1,
            ..short
        };
        assert_ne!(role(&mut state, enough, true), TROOP_CARRIERS);
        state.module_b.tried = TROOP_CARRIERS.tried;
        assert_eq!(role(&mut state, short, false), DEATH_STAR);
        // The Alliance has no Death Star role; +0x68 keeps the last one's.
        assert_eq!(
            role(&mut state, short, true),
            Role {
                tried: DEATH_STAR.tried,
                ..Role::default()
            }
        );
        let built = Tallies {
            death_star: true,
            ..short
        };
        assert_eq!(role(&mut state, built, true).flags, 0);
        state.module_b.sites.pop();
        assert_eq!(role(&mut state, short, false).flags, 0);
        let unarmed = Tallies { armed: 0, ..short };
        assert_eq!(role(&mut state, unarmed, true), ARMED);
        let no_lasers = Tallies { lasers: 0, ..short };
        assert_eq!(role(&mut state, no_lasers, true), LASERS);
        let no_hangars = Tallies {
            hangars: 0,
            ..short
        };
        assert_eq!(role(&mut state, no_hangars, true), CARRIERS);
        for tried in [ARMED, LASERS, CARRIERS] {
            state.module_b.tried = tried.tried;
            let all = Tallies {
                armed: 0,
                lasers: 0,
                hangars: 0,
                ..short
            };
            assert_ne!(role(&mut state, all, true), tried);
        }
    }

    #[test]
    fn the_ship_plan_scraps_over_budget_ends_at_zero_and_marks_a_role_it_cannot_afford() {
        // FUN_004ca910: the tried bits clear; FUN_004cc220 (90% of 30% of
        // 1000 = 270, less +0x24c) below 0 gives 10, 0 gives 0; a role
        // with no class within the budget is tried (+0x28 |= +0x68).
        let (f, _) = ship_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_b.role = TROOP_CARRIERS;
        state.module_b.tried = 0xff00_0001;
        let plan = |state: &mut AIState, t| {
            edited(&f, state, t, |_| {}, |m, io| (m.plan_ships(io), m.tried))
        };
        let over = Tallies {
            ship_upkeep: 271,
            ..ROOMY
        };
        assert_eq!(plan(&mut state, over), (10, 0x8000_0001));
        let spent = Tallies {
            ship_upkeep: 270,
            ..ROOMY
        };
        assert_eq!(plan(&mut state, spent).0, 0);
        let tight = Tallies {
            ship_upkeep: 267,
            ..ROOMY
        };
        assert_eq!(
            plan(&mut state, tight),
            (0, 0x8000_0001 | TROOP_CARRIERS.tried)
        );
        assert_eq!(state.module_b.count, 0);
        let roomy = Tallies {
            ship_upkeep: 266,
            ..ROOMY
        };
        assert_eq!(plan(&mut state, roomy), (0xf, 0x8000_0001));
        assert_eq!(state.module_b.count, 1);
    }

    #[test]
    fn with_no_role_any_class_the_shipyards_list_may_be_built() {
        // FUN_004ca910 with +0x60 0: every shipyard class passes the flags,
        // ships and squadrons alike, at key 0.
        let (f, _) = ship_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        let kind = edited(
            &f,
            &mut state,
            ROOMY,
            |_| {},
            |m, io| {
                assert_eq!(m.plan_ships(io), 0xf);
                m.ship
            },
        );
        assert!(matches!(kind, Some(BuildableKind::CapitalShip(_))));
    }

    #[test]
    fn a_squadron_class_is_built_on_its_own_upkeep_and_cost() {
        // FUN_004ca910 with no role: a squadron class qualifies (key 0,
        // FUN_00473640) when its maintenance (+0x20) fits the budget;
        // FUN_004cba90 builds it on the manager at its refined cost.
        let (mut f, _) = ship_fixture();
        f.world.capital_ship_classes.clear();
        let wing = |f: &mut Fixture, maintenance| {
            f.world.fighter_classes.clear();
            f.world.fighter_classes.insert(crate::world::FighterClass {
                is_alliance: true,
                maintenance_cost: maintenance,
                refined_material_cost: 7,
                ..crate::world::FighterClass::default()
            })
        };
        let cheap = wing(&mut f, 5);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_b.yard = Yard::Manager(f.yard);
        let order = edited(&f, &mut state, ROOMY, |_| {}, |m, io| {
            assert_eq!(m.plan_ships(io), 0xf);
            m.build_ships(io)
        });
        assert_eq!(
            order,
            Some(AIAction::Build {
                system: f.yard,
                kind: BuildableKind::Fighter(cheap),
                cost: 7,
                count: 1,
            })
        );
        wing(&mut f, 271);
        let plan = edited(&f, &mut state, ROOMY, |_| {}, |m, io| m.plan_ships(io));
        assert_eq!(plan, 0);
    }

    #[test]
    fn the_roles_rank_ship_classes_on_their_class_record_values() {
        // FUN_00473930 (ship class record slot +4): 0x200 +0x30, 0x80000
        // +0x58, 0x400 +0x34, 0x80 +0x44, 0x100 +0x48; others 0.
        let r = ClassRecord {
            values: std::array::from_fn(|i| 10 + i32::try_from(i).unwrap()),
            ..ClassRecord::default()
        };
        let keys = [0x200, 0x8_0000, 0x400, 0x80, 0x100, 0];
        assert_eq!(keys.map(|k| ship_value(&r, k)), [10, 20, 11, 15, 16, 0]);
    }

    #[test]
    fn the_shipyard_plan_divides_its_budget_by_the_class_s_upkeep() {
        // FUN_004cb710: count = w / m, 30 / 10 = 3 under the yard's 4 and
        // the room 9; a class with no maintenance builds none.
        let (mut f, _) = ship_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_b.capacity = 4;
        state.module_b.room = 9;
        state.module_b.target = Target::System(f.empty);
        let plan = |f: &Fixture, state: &mut AIState| {
            edited(f, state, ROOMY, |_| {}, |m, io| (m.plan_yards(io), m.count))
        };
        assert_eq!(plan(&f, &mut state), (0xe, 3));
        f.world
            .buildable_classes
            .get_mut(&DatId::new(SHIPYARD_CLASS))
            .unwrap()
            .maintenance_cost = 0;
        assert_eq!(plan(&f, &mut state), (0, 0));
    }

    #[test]
    fn the_budgets_hold_at_their_edges() {
        // FUN_004cc220: 0 only when the free maintenance is below the
        // budget; FUN_004caad0: over only below 0.
        let edge = |requirement| Tallies {
            requirement,
            ..ROOMY
        };
        assert_eq!(ship_budget(edge(1000 - 270)), 270);
        assert_eq!(ship_budget(edge(1000 - 269)), 0);
        assert_eq!(over_budget(ROOMY), 270);
    }

    #[test]
    fn over_budget_the_cheapest_ship_of_the_cheapest_fleet_is_scrapped() {
        // FUN_004c8c20 0xd: FUN_004caad0 below 0 gives 10; FUN_004cab20:
        // the fleet with the least upkeep (key 0xe, order 2), its ship with
        // the least maintenance (key 0xb, order 2), Scrap 0x200. A fleet of
        // squadrons alone is passed over (port).
        let (mut f, h) = ship_fixture();
        let here = f.empty;
        fleet(&mut f, here, &[(h.gunship, 100), (h.gunship, 100)]);
        let cheap = fleet(&mut f, here, &[(h.gunship, 100), (h.trooper, 100)]);
        let wings = f.world.fleets.insert(Fleet {
            location: here,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        f.world.systems[here].fleets.push(wings);
        let mut state = AIState::new(AiFaction::Alliance);
        at(&mut state, SHIP_CYCLE, 0xd);
        let t = |ship_upkeep| Tallies {
            ship_upkeep,
            ..ROOMY
        };
        assert_eq!(step(&f, &mut state, t(270)), Step::pending());
        assert_eq!(
            (state.module_b.phase, state.module_b.state),
            (YARD_CYCLE, 0)
        );
        at(&mut state, SHIP_CYCLE, 0xd);
        step(&f, &mut state, t(271));
        assert_eq!(state.module_b.state, 10);
        let s = step(&f, &mut state, t(271));
        assert_eq!(
            s.order,
            Some(AIAction::Scrap {
                team: vec![ScrapTarget::Ship {
                    fleet: cheap,
                    index: 1,
                }],
            })
        );
        assert!(s.complete);
        assert_eq!(state.module_b.phase, YARD_CYCLE);
    }

    #[test]
    fn the_yard_cycle_borrows_with_token_two_and_builds_shipyards_on_a_tenth_of_its_share() {
        // FUN_004c8d20 8: bit 2 cleared; with +0xc == 2, FUN_004cafa0 (module
        // 2's FUN_004e3810) takes the yard nearest the target and makes the
        // target a site marked 0x200000; FUN_004199b0; 0xb (FUN_004cb1d0)
        // -> 9 (FUN_004cb710: 10% of 30% of 1000 = 30, over the class's 10:
        // 3, under the yard's 1) gives 0xe -> Destination -> Build.
        let (f, _) = ship_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.controller.yard_holder = YARD_TOKEN;
        state.controller.yard_requests = YARD_TOKEN | 0x80;
        at(&mut state, YARD_CYCLE, 8);
        state.module_b.target = Target::System(f.empty);
        state.module_b.room = 9;
        let lent = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x1000;
            let r = rec(g, f.yard);
            r.state |= 0x800;
            r.values[6] = 2;
        };
        let s = edited(&f, &mut state, ROOMY, lent, |m, io| m.step(io));
        assert_eq!(s, Step::pending());
        assert_eq!(state.module_b.state, 0xb);
        assert_eq!(
            (state.module_b.yard, state.module_b.capacity),
            (Yard::Manager(f.yard), 1)
        );
        assert_eq!(state.module_b.sites, [f.empty]);
        assert_eq!(state.yard_marks.get(&f.empty), Some(&MARK_SHIPYARD));
        assert_eq!(state.controller.yard_requests, 0);
        assert_eq!(state.controller.yard_holder, 0x80);
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!(state.module_b.state, 9);
        assert_eq!(step(&f, &mut state, ROOMY), Step::pending());
        assert_eq!((state.module_b.state, state.module_b.count), (0xe, 1));
        let s = step(&f, &mut state, ROOMY);
        assert!(s.complete && s.raise);
        assert_eq!(state.module_b.state, 0xf);
        let s = step(&f, &mut state, ROOMY);
        let Some(AIAction::Build { kind, count, .. }) = s.order else {
            panic!("{s:?}");
        };
        assert_eq!(
            (kind.class_dat_id(), count),
            (Some(DatId::new(SHIPYARD_CLASS)), 1)
        );
        assert_eq!(
            (state.module_b.phase, state.module_b.state),
            (FLEET_CYCLE, 0)
        );
    }

    #[test]
    fn the_shipyard_plan_scraps_a_surplus_and_ends_otherwise() {
        // FUN_004cb710: the budget below 0 (the sites' v[36] over 30) gives
        // 0xc, which scraps a spare shipyard (FUN_004cb7b0) and ends; 0
        // gives 0, which ends with 0x3f8.
        let (f, _) = ship_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_b.sites.push(f.yard);
        state.yard_marks.insert(f.yard, MARK_SHIPYARD);
        at(&mut state, YARD_CYCLE, 9);
        let heavy = |g: &mut Galaxy| rec(g, f.yard).values[36] = 31;
        edited(&f, &mut state, ROOMY, heavy, |m, io| m.step(io));
        assert_eq!(state.module_b.state, 0xc);
        let s = edited(&f, &mut state, ROOMY, heavy, |m, io| m.step(io));
        assert!(s.complete);
        assert_eq!(
            (state.module_b.phase, state.module_b.state),
            (FLEET_CYCLE, 0)
        );
        at(&mut state, YARD_CYCLE, 9);
        let full = |g: &mut Galaxy| rec(g, f.yard).values[36] = 30;
        let s = edited(&f, &mut state, ROOMY, full, |m, io| m.step(io));
        assert!(s.complete);
        assert_eq!(
            (state.module_b.phase, state.module_b.state),
            (FLEET_CYCLE, 0)
        );
    }
}
