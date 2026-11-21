//! Module `0xa` of the AI player (`FUN_004cc6b0`, step `FUN_004cc9c0`): it
//! keeps the squadrons of the side's fleets. It builds squadrons at its
//! shipyard sites for a sector planner's base short of hangar squadrons or
//! for a fleet with hangar room, scraps a fleet's weakest squadron when
//! their upkeep is over budget, and builds shipyards while it holds the
//! yard token `0x10` (`ghidra/notes/ai-production-modules.md`, "Module
//! `0xa`"). Its sites carry the shipyard mark `0x200000`
//! ([`MARK_SHIPYARD`]).

use serde::{Deserialize, Serialize};

use crate::ai::{AIAction, MARK_SHIPYARD};
use crate::ai_module1::{
    adopt_or_request, build, claim, destination, destination_in, scrap_at, scrap_spare_yard,
    yard_class, Inputs, Step, Target, Yard, SHIPYARD,
};
use crate::ai_module2::{borrow_yard, keep_site};
use crate::ai_module3::choose_target_keyed;
use crate::ai_module7::{fifth_less_yards, fighter_class, FighterRank};
use crate::ai_module8::SectorPlanner;
use crate::ai_records::{pick, ranked, Mask, Order, FLEET_OWN, SHIP_FREE, SHIP_OWN};
use crate::ids::{DatId, FighterKey, FleetKey, SystemKey};
use crate::manufacturing::{BuildableKind, ProductionArea};
use crate::scrap::ScrapTarget;
use crate::squadrons::SquadronPlace;

/// `+0x38` while the yard cycle runs (`FUN_004cc9c0`).
pub const YARD_CYCLE: u32 = 0x3ec;

/// `+0x38` while the squadron cycle runs (`FUN_004cc9c0`).
pub const SQUADRON_CYCLE: u32 = 0x3f2;

/// The yard token module `0xa` holds the construction yards with
/// (controller `+0xc`, `+8`; `FUN_004ccb50` state 3).
pub const YARD_TOKEN: u32 = 0x10;

/// `+0x24`: the percent of the maintenance capacity module `0xa` budgets
/// (`FUN_004cc6b0`): 80% of it for the fleets' squadrons
/// (`FUN_004cec20`), 20% for shipyards (`FUN_004cec80`).
const PERCENT: i32 = 8;

/// The fleet records `FUN_004cd040` falls back on: own (1), free (4), with
/// hangar room left (`0x100000`), not en route (2) nor with an inactive
/// ship (`0x400`) nor a travelling one (`0x800`).
const ROOMY_FLEETS: Mask = Mask::new(0x10_0005, 0xc02);

/// Module `0xa`'s own fields (`FUN_004cc6b0`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleA {
    /// `+0x38`: 0, [`SQUADRON_CYCLE`] or [`YARD_CYCLE`].
    pub phase: u32,
    /// `+0x3c`: the cycle's state.
    pub state: u32,
    /// `+0x40`: the id of the sector planner whose base the squadrons go
    /// to, 0 for none.
    pub planner: u32,
    /// `+0x44`: the id of the planner after it, where the next walk starts
    /// (`FUN_005f5500`), 0 for the list's head.
    pub next_planner: u32,
    /// `+0x58`: a site, then its shipyard manager; in the yard cycle the
    /// borrowed construction yard's manager.
    pub yard: Yard,
    /// `+0x5c`: where the squadrons go, a planner's base or a fleet.
    pub destination: Option<SquadronPlace>,
    /// `+0x60`: the yard cycle's target.
    pub target: Target,
    /// `+0x64` in the squadron cycle: the squadron class to build.
    pub fighter: Option<FighterKey>,
    /// `+0x64` in the yard cycle: the shipyard class to build.
    pub class: Option<DatId>,
    /// `+0x68`: how many.
    pub count: i32,
    /// `+0x6c`: the borrowed yard's construction yards (`v[6]`, less one
    /// above 1).
    pub capacity: i32,
    /// `+0x70`: the target's free room `v[0x33]`.
    pub room: i32,
    /// `+0x74`: the sites, marked [`MARK_SHIPYARD`].
    pub sites: Vec<SystemKey>,
    /// `+0x7c`: the claimed sites.
    pub claimed: Vec<SystemKey>,
}

impl ModuleA {
    /// One step (`FUN_004cc9c0`). The scheduler steps only a ready module,
    /// so the not-ready branch never runs.
    pub fn step(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.phase {
            YARD_CYCLE => self.yard_cycle(io),
            SQUADRON_CYCLE => self.squadron_cycle(io),
            _ => {
                self.phase = SQUADRON_CYCLE;
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

    /// The squadron cycle (`FUN_004cca20`).
    fn squadron_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            3 => {
                self.state =
                    if keep_site(io, &mut self.sites, &mut self.yard, MARK_SHIPYARD, SHIPYARD) {
                        2
                    } else {
                        8
                    };
                Step::pending()
            }
            2 => {
                self.state = if self.choose_destination(io) { 4 } else { 8 };
                Step::pending()
            }
            4 => {
                self.state = self.plan_squadrons(io);
                if self.state == 0 {
                    self.phase = YARD_CYCLE;
                    return Step::complete();
                }
                Step::pending()
            }
            8 => {
                if squadron_budget(io) < 0 {
                    self.state = 5;
                } else {
                    self.state = 0;
                    self.phase = YARD_CYCLE;
                }
                Step::pending()
            }
            5 => {
                let order = relieve(io);
                self.end(YARD_CYCLE, order)
            }
            9 => {
                let to = self.destination_system(io);
                match destination_in(ProductionArea::Shipyard, to, self.yard) {
                    Some(order) => {
                        self.state = 10;
                        Step {
                            order: Some(order),
                            complete: true,
                            raise: true,
                        }
                    }
                    None => self.end(YARD_CYCLE, None),
                }
            }
            10 => match self.build_squadrons(io) {
                Some(order) => self.end(YARD_CYCLE, Some(order)),
                // FUN_004ce490 gives Build 0x211 with no class, which the
                // validation refuses: slot +0x18 resets the module.
                None if self.fighter.is_none() && matches!(self.yard, Yard::Manager(_)) => {
                    self.refused();
                    Step::complete()
                }
                None => self.end(YARD_CYCLE, None),
            },
            _ => {
                self.state = 3;
                Step::pending()
            }
        }
    }

    /// `FUN_004cd040`: where the squadrons go, into `+0x5c`, and how many
    /// into `+0x68`. The walk of module 8's sector planners (controller
    /// `+0xd8`) starts at the one `+0x44` names, else at the head, and does
    /// not wrap: the first planner whose base is short of squadrons for its
    /// fleets' hangars (`FUN_00476f90`) gives its base and the shortfall
    /// (planner `+0x9c`); `+0x40` takes the planner and `+0x44` the next.
    /// With none both clear, and the side's fleet with the least hangar
    /// room left (`FUN_0041a4c0(0x100005, 0xc02, 0x17)`, order 2) takes one
    /// squadron.
    fn choose_destination(&mut self, io: &mut Inputs<'_>) -> bool {
        self.count = 0;
        self.destination = None;
        let planners = io.planners;
        let from = planners
            .iter()
            .position(|p| p.id == self.next_planner)
            .unwrap_or(0);
        for (at, planner) in planners.iter().enumerate().skip(from) {
            if let Some((base, short)) = short_base(io, planner) {
                self.destination = Some(SquadronPlace::Planet(base));
                self.count = short;
                self.planner = planner.id;
                self.next_planner = planners.get(at + 1).map_or(0, |p| p.id);
                return true;
            }
        }
        self.planner = 0;
        self.next_planner = 0;
        let fleets = crate::ai_module_b::fleets(io, ROOMY_FLEETS, 0x17, Order::Descending);
        let Some(fleet) = pick(&[fleets]) else {
            return false;
        };
        self.destination = Some(SquadronPlace::Fleet(fleet));
        self.count = 1;
        true
    }

    /// The system the Destination names (`FUN_004ce350(+0x5c)`).
    ///
    /// port: a production manager's destination is a system in the port
    /// ([`crate::production_cycle::ProductionState::set_destination`]); a
    /// fleet's (the original's `0x214` takes one, `FUN_00537180`) is its
    /// system when the order is given.
    fn destination_system(&self, io: &Inputs<'_>) -> Target {
        match self.destination {
            Some(SquadronPlace::Planet(system)) => Target::System(system),
            Some(SquadronPlace::Fleet(fleet)) => io
                .world
                .fleets
                .get(fleet)
                .map_or(Target::None, |f| Target::System(f.location)),
            None => Target::None,
        }
    }

    /// `FUN_004cd230`: 9 to build the squadrons within the budget
    /// `w` (`FUN_004cec20`) and the site's shipyards, 5 when the fleets'
    /// squadrons cost more than it, else 0. The class is the side's best
    /// listed squadron (`FUN_0041b230(.., 0x28, 0x200000, key, 1)`) by its
    /// torpedoes (key `0x8000`, `values[6]`) on a draw of 6 or more of 10
    /// (`FUN_0041cd80(10)`, `0x4cd253`), else by its `+0x34` (key `0x400`,
    /// `values[1]`); when `m * count > w`, `count = w / m`.
    ///
    /// port: it then clears controller `+0x20`, a site fallback with no
    /// counterpart (module 7's `FUN_004d3ff0`).
    fn plan_squadrons(&mut self, io: &mut Inputs<'_>) -> u32 {
        let budget = squadron_budget(io);
        if budget < 1 {
            return if budget < 0 { 5 } else { 0 };
        }
        let key = if (io.random)(10) >= 6 {
            FighterRank::Value(6)
        } else {
            FighterRank::Value(1)
        };
        let maintenance;
        (self.fighter, maintenance) = fighter_class(io, key, Order::Ascending);
        if self.fighter.is_some() && maintenance != 0 && maintenance * self.count > budget {
            self.count = budget / maintenance;
        }
        if self.count <= 0 {
            return 0;
        }
        // FUN_004f2f60 and FUN_00528040(0): the shipyard manager's yards.
        if let Yard::Manager(site) = self.yard {
            self.count = self.count.min(shipyards(io, site));
        }
        9
    }

    /// `FUN_004ce490`: Build `0x211` of `+0x68` squadrons of `+0x64` on the
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

    /// The yard cycle (`FUN_004ccb50`): module 7's (`FUN_004d3350`) with
    /// the token `0x10` and the states renumbered.
    fn yard_cycle(&mut self, io: &mut Inputs<'_>) -> Step {
        match self.state {
            2 => {
                // FUN_004cd580: module 7's FUN_004d5000, its second system
                // list ranked on key 1.
                if choose_target_keyed(
                    io,
                    &mut self.sites,
                    &mut self.target,
                    &mut self.room,
                    MARK_SHIPYARD,
                    SHIPYARD,
                    1,
                ) {
                    self.state = 3;
                    Step::pending()
                } else {
                    self.end(SQUADRON_CYCLE, None)
                }
            }
            3 => {
                io.controller.yard_requests &= !YARD_TOKEN;
                // FUN_004cd860: module 2's FUN_004e3810 on +0x60.
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
                    self.state = 6;
                } else {
                    self.state = 0xb;
                }
                Step::pending()
            }
            6 => {
                let order = scrap_at(io, self.target.system(), SHIPYARD);
                self.state = 4;
                Step::ordered(order)
            }
            4 => {
                self.state = self.plan_yards(io);
                if self.state == 0 {
                    return self.end(SQUADRON_CYCLE, None);
                }
                Step::pending()
            }
            7 => {
                let order = scrap_spare_yard(
                    io,
                    &mut self.sites,
                    &mut self.target,
                    MARK_SHIPYARD,
                    SHIPYARD,
                );
                self.end(SQUADRON_CYCLE, order)
            }
            9 => match destination(self.target, self.yard) {
                Some(order) => {
                    self.state = 10;
                    Step {
                        order: Some(order),
                        complete: true,
                        raise: true,
                    }
                }
                None => self.end(SQUADRON_CYCLE, None),
            },
            10 => {
                let order = build(io, self.yard, self.class, self.count);
                self.end(SQUADRON_CYCLE, order)
            }
            0xb => {
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
                self.end(SQUADRON_CYCLE, None)
            }
            _ => {
                self.state = 2;
                Step::pending()
            }
        }
    }

    /// `FUN_004cdfd0` (module 7's `FUN_004d5a50`): 9 to build shipyards,
    /// 7 to scrap one when the sites hold more than the side wants, else 0.
    fn plan_yards(&mut self, io: &mut Inputs<'_>) -> u32 {
        self.count = 0;
        let budget = yard_budget(io, &self.sites);
        if budget < 0 {
            return 7;
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
        9
    }
}

/// `FUN_00476f90` on `planner`: its base (`+0x40`), a system with a record
/// and no blockade (`+0x28` bit 2), and the hangar room left in the fleets
/// it holds (`FUN_0041acd0(id, 1, 0, 0x17)`, fleet record `+0xac`) less the
/// squadrons on the base's planet (`v[23]`), when that is above 0.
fn short_base(io: &Inputs<'_>, planner: &SectorPlanner) -> Option<(SystemKey, i32)> {
    let base = planner.base?;
    let record = io.galaxy.system(base)?;
    if record.state & 2 != 0 {
        return None;
    }
    let room: i32 = io
        .galaxy
        .fleets
        .values()
        .filter(|f| f.owner == planner.id && f.flags & FLEET_OWN != 0)
        .map(|f| f.values[23])
        .sum();
    let short = room - record.values[23];
    (short > 0).then_some((base, short))
}

/// `FUN_00528040(0)` on the site's shipyard manager: the side's shipyards
/// there not suspended (`+0x60` bit 0). The kept site is calm (`+0x28 & 3`
/// clear), so none is.
fn shipyards(io: &Inputs<'_>, site: SystemKey) -> i32 {
    let side = io.side();
    let Some(system) = io.world.systems.get(site) else {
        return 0;
    };
    let count = system
        .manufacturing_facilities
        .iter()
        .filter_map(|&key| io.world.manufacturing_facilities.get(key))
        .filter(|yard| yard.side == side && yard.class_dat_id.family() == SHIPYARD.family)
        .count();
    i32::try_from(count).unwrap_or(i32::MAX)
}

/// `FUN_004cd310`: of the side's free fleets not en route
/// (`FUN_0041a4c0(5, 2, 9, 1)`), the one with the most squadrons; of its
/// own free ships not travelling (`FUN_0041ac80(0x24000, 0x800, 6, 1)`),
/// the one with the most squadrons aboard; of those squadrons the one with
/// the lowest slot `+0x1f4` (`FUN_005031e0`): Scrap `0x200`.
///
/// port: a fleet's squadrons ride on its first ship's record
/// ([`crate::ai_records`]); the squadron is the fleet's weakest, the
/// smallest class, the first on a tie (module 7's).
fn relieve(io: &mut Inputs<'_>) -> Option<AIAction> {
    let fleets = crate::ai_module_b::fleets(io, Mask::new(5, 2), 9, Order::Ascending);
    let fleet = pick(&[fleets])?;
    let record = io.galaxy.fleets.get(&fleet)?;
    let ships = Mask::new(SHIP_FREE | SHIP_OWN, 0x800);
    let aboard: Vec<_> = record
        .ships
        .iter()
        .filter(|s| ships.admits(s.flags))
        .map(|s| (s.index, s.values[6]))
        .collect();
    let squadrons: std::collections::HashMap<usize, i32> = aboard.iter().copied().collect();
    let ship = pick(&[ranked(aboard, Order::Ascending, io.coin)])?;
    if squadrons.get(&ship).copied().unwrap_or(0) < 1 {
        return None;
    }
    let class = weakest_squadron(io, fleet)?;
    Some(AIAction::Scrap {
        team: vec![ScrapTarget::FleetFighter { fleet, class }],
    })
}

/// The squadron of `fleet` with the lowest strength (slot `+0x1f4`: the
/// class's squadron size, losses not counted), the first on a tie.
fn weakest_squadron(io: &Inputs<'_>, fleet: FleetKey) -> Option<FighterKey> {
    let fleet = io.world.fleets.get(fleet)?;
    let mut best: Option<(FighterKey, u32)> = None;
    for entry in fleet.fighters.iter().filter(|e| e.count > 0) {
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

/// `FUN_004cec20`: 80% of `+0x24` percent of the capacity, less the
/// squadrons' upkeep aboard the fleets (`+0x254`); 0 when the side's free
/// maintenance falls short of it. `FUN_004cd2f0` asks whether it is below
/// 0.
fn squadron_budget(io: &Inputs<'_>) -> i32 {
    let t = io.tallies;
    let budget = PERCENT * t.capacity / 100 * 80 / 100 - t.wing_upkeep;
    if t.capacity - t.requirement < budget {
        0
    } else {
        budget
    }
}

/// `FUN_004cec80` (module 7's `FUN_004d6700`): 20% of `+0x24` percent of
/// the capacity, less the sites' shipyard upkeep (`v[34]`).
fn yard_budget(io: &Inputs<'_>, sites: &[SystemKey]) -> i32 {
    fifth_less_yards(io, sites, PERCENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{AIState, AiFaction, MARK_MINES};
    use crate::ai_module1::tests::{add_yard, class, fixture, record, records_of, Fixture, ROOMY};
    use crate::ai_module1::Tallies;
    use crate::ai_records::{FleetRecord, Galaxy, ShipRecord, SystemRecord};
    use crate::dat::Faction;
    use crate::research::ResearchState;
    use crate::world::{CapitalShipClass, FighterClass, Fleet, ShipInstance};

    const SHIPYARD_CLASS: u32 = 0x2800_0001;

    /// What a step sees besides the fixture: module 8's planners and the
    /// draw `FUN_0041cd80(10)` gives.
    #[derive(Clone, Copy)]
    struct Around<'p> {
        planners: &'p [SectorPlanner],
        draw: i32,
    }

    const ALONE: Around<'static> = Around {
        planners: &[],
        draw: 0,
    };

    fn edited<R>(
        f: &Fixture,
        state: &mut AIState,
        tallies: Tallies,
        around: Around<'_>,
        edit: impl FnOnce(&mut Galaxy),
        run: impl FnOnce(&mut ModuleA, &mut Inputs<'_>) -> R,
    ) -> R {
        let mut galaxy = records_of(f, state);
        edit(&mut galaxy);
        let research = ResearchState::new();
        let transport = crate::troop_transport::TroopTransportState::default();
        let mut coin = || false;
        let mut random = |n| {
            assert_eq!(n, 10);
            around.draw
        };
        let mut io = Inputs {
            world: &f.world,
            research: &research,
            galaxy: &mut galaxy,
            controller: &mut state.controller,
            marks: &mut state.yard_marks,
            kinds: &mut state.record_kinds,
            tallies,
            is_alliance: true,
            planners: around.planners,
            transport: &transport,
            coin: &mut coin,
            random: &mut random,
        };
        run(&mut state.module_a, &mut io)
    }

    fn rec(g: &mut Galaxy, s: SystemKey) -> &mut SystemRecord {
        record(g, s)
    }

    struct Wings {
        /// Squadron size 12, maintenance 2, `values[1]` 120, torpedoes 3
        /// (`values[6]` 36).
        cheap: FighterKey,
        /// Squadron size 6, maintenance 5, `values[1]` 600, no torpedoes.
        strong: FighterKey,
    }

    /// The fixture with two shipyards at `yard` and two Alliance squadron
    /// classes.
    fn squadron_fixture() -> (Fixture, Wings) {
        let mut f = fixture();
        let shipyard = class(&mut f.world, SHIPYARD_CLASS, 10, 2, 0);
        add_yard(&mut f.world, f.yard, shipyard, Faction::Alliance);
        add_yard(&mut f.world, f.yard, shipyard, Faction::Alliance);
        let mut wing = |size, maintenance, maneuver, torpedoes| {
            f.world.fighter_classes.insert(FighterClass {
                is_alliance: true,
                squadron_size: size,
                maintenance_cost: maintenance,
                refined_material_cost: 7,
                laser_cannon_fore: 1,
                maneuverability: maneuver,
                torpedoes,
                ..FighterClass::default()
            })
        };
        let cheap = wing(12, 2, 1, 3);
        let strong = wing(6, 5, 10, 0);
        (f, Wings { cheap, strong })
    }

    /// Module `0xa` with `yard` as its site.
    fn with_site(f: &Fixture) -> AIState {
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_a.sites.push(f.yard);
        state.yard_marks.insert(f.yard, MARK_SHIPYARD);
        state
    }

    fn idle(g: &mut Galaxy, f: &Fixture) {
        rec(g, f.yard).state |= 0x200;
    }

    /// An Alliance carrier fleet at `at`.
    fn carrier(f: &mut Fixture, at: SystemKey) -> FleetKey {
        let class = f.world.capital_ship_classes.insert(CapitalShipClass {
            is_alliance: true,
            fighter_capacity: 4,
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

    /// A fleet record with `flags`, hangar room `room` (`values[23]`), held
    /// by planner `owner`, its one ship carrying `wings` squadrons
    /// (`values[6]`).
    fn fleet_record(
        g: &mut Galaxy,
        fleet: FleetKey,
        flags: u32,
        room: i32,
        owner: u32,
        wings: i32,
    ) {
        let mut values = [0; 30];
        values[23] = room;
        values[9] = wings;
        let mut ship = [0; 30];
        ship[6] = wings;
        g.fleets.insert(
            fleet,
            FleetRecord {
                flags,
                values,
                ships: vec![ShipRecord {
                    index: 0,
                    id: 0,
                    fleet,
                    flags: SHIP_FREE | SHIP_OWN,
                    owner: 0,
                    plan: 0,
                    values: ship,
                }],
                owner,
                plan: 0,
                target: None,
            },
        );
    }

    fn planner(id: u32, base: SystemKey) -> SectorPlanner {
        SectorPlanner {
            id,
            base: Some(base),
            ..SectorPlanner::default()
        }
    }

    #[test]
    fn the_cycles_hand_on_from_squadrons_to_yards_and_back() {
        // FUN_004cc9c0: any other +0x38 starts 0x3f2. FUN_004cca20 3 keeps
        // no site: 8 (FUN_004cd2f0, within budget) hands on to 0x3ec
        // without completing. FUN_004ccb50 2 takes a target
        // (FUN_004cd580; module 1's mark is not forbidden, 0x200003); 3
        // without the token 0x10 goes to 0xb (FUN_004ce810, FUN_004ced60),
        // which adopts the yard with its shipyards as a site and ends with
        // 0x3f2, complete. The next squadron cycle keeps that site (3 -> 2).
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.yard_marks.insert(f.empty, MARK_MINES);
        let mut trail = Vec::new();
        for _ in 0..10 {
            let s = edited(&f, &mut state, ROOMY, ALONE, |_| {}, |m, io| m.step(io));
            trail.push((state.module_a.phase, state.module_a.state, s.complete));
        }
        assert_eq!(
            trail,
            [
                (SQUADRON_CYCLE, 0, false),
                (SQUADRON_CYCLE, 3, false),
                (SQUADRON_CYCLE, 8, false),
                (YARD_CYCLE, 0, false),
                (YARD_CYCLE, 2, false),
                (YARD_CYCLE, 3, false),
                (YARD_CYCLE, 0xb, false),
                (SQUADRON_CYCLE, 0, true),
                (SQUADRON_CYCLE, 3, false),
                (SQUADRON_CYCLE, 2, false),
            ]
        );
        assert_eq!(state.module_a.sites, [f.yard]);
        state.module_a.refused();
        assert_eq!((state.module_a.phase, state.module_a.state), (0, 0));
    }

    #[test]
    fn a_planner_s_base_short_of_squadrons_has_them_built_at_the_site() {
        // FUN_004cca20: 3 keeps the site (FUN_004cccc0: shipyards v[4],
        // idle 0x200); 2 finds planner 7's base short (FUN_004cd040,
        // FUN_00476f90: its fleets' hangar room 5 less the base's 1
        // squadron); 4 plans (FUN_004cd230: a draw below 6 ranks on +0x34,
        // the count capped by the manager's 2 shipyards, FUN_00528040); 9
        // gives Destination 0x214 on the shipyard manager (FUN_004ce350)
        // and 10 Build 0x211 (FUN_004ce490).
        let (mut f, w) = squadron_fixture();
        crate::squadrons::add(&mut f.world.systems[f.empty].fighters, w.cheap, 1);
        let at = f.empty;
        let held = carrier(&mut f, at);
        let mut state = with_site(&f);
        state.module_a.phase = SQUADRON_CYCLE;
        let planners = [planner(7, f.empty), planner(9, f.yard)];
        let around = Around {
            planners: &planners,
            draw: 5,
        };
        let edit = |g: &mut Galaxy| {
            idle(g, &f);
            fleet_record(g, held, 5, 5, 7, 0);
        };
        let mut steps = Vec::new();
        for _ in 0..6 {
            let s = edited(&f, &mut state, ROOMY, around, edit, |m, io| m.step(io));
            steps.push((state.module_a.state, s));
        }
        let states: Vec<_> = steps.iter().map(|(state, _)| *state).collect();
        assert_eq!(states, [3, 2, 4, 9, 10, 0]);
        assert_eq!(
            (state.module_a.planner, state.module_a.next_planner),
            (7, 9)
        );
        assert_eq!(state.module_a.fighter, Some(w.strong));
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
                kind: BuildableKind::Fighter(w.strong),
                cost: 7,
                count: 2,
            })
        );
        assert!(steps[5].1.complete);
        assert_eq!(state.module_a.phase, YARD_CYCLE);
    }

    #[test]
    fn the_planner_walk_resumes_after_its_last_pick_and_does_not_wrap() {
        // FUN_004cd040 starts at the planner +0x44 names (FUN_005f5500),
        // else the head; a found planner leaves +0x44 at the next one. A
        // base not short or blockaded (+0x28 2) is passed over; past the
        // list's end the walk falls back on the fleets.
        let (mut f, _) = squadron_fixture();
        let at = f.empty;
        let fleet = carrier(&mut f, at);
        let third = crate::ai_module1::tests::system(&mut f.world, f.sector, 9);
        let planners = [planner(1, f.empty), planner(2, f.yard), planner(3, third)];
        let around = Around {
            planners: &planners,
            draw: 0,
        };
        let mut state = AIState::new(AiFaction::Alliance);
        let walk = |state: &mut AIState, owner: u32, edit: &dyn Fn(&mut Galaxy)| {
            edited(
                &f,
                state,
                ROOMY,
                around,
                |g| {
                    fleet_record(g, fleet, 0x10_0005, 2, owner, 0);
                    edit(g);
                },
                |m, io| {
                    (
                        m.choose_destination(io),
                        m.destination,
                        m.count,
                        m.planner,
                        m.next_planner,
                    )
                },
            )
        };
        // Planner 3 holds the fleet: its base is short by 2.
        let found = walk(&mut state, 3, &|_| {});
        assert_eq!(found, (true, Some(SquadronPlace::Planet(third)), 2, 3, 0));
        // From the head again; planner 3's base now holds 2 squadrons: the
        // fallback, the fleet with hangar room (key 0x17, order 2), one
        // squadron.
        let found = walk(&mut state, 3, &|g| rec(g, third).values[23] = 2);
        assert_eq!(found, (true, Some(SquadronPlace::Fleet(fleet)), 1, 0, 0));
        // A blockaded base is passed over.
        let found = walk(&mut state, 3, &|g| rec(g, third).state |= 2);
        assert_eq!(found.1, Some(SquadronPlace::Fleet(fleet)));
        // A walk that starts past a short base does not wrap to it.
        state.module_a.next_planner = 3;
        let found = walk(&mut state, 1, &|_| {});
        assert_eq!(found, (true, Some(SquadronPlace::Fleet(fleet)), 1, 0, 0));
        let found = walk(&mut state, 1, &|_| {});
        assert_eq!(found, (true, Some(SquadronPlace::Planet(f.empty)), 2, 1, 2));
    }

    #[test]
    fn with_no_fleet_with_hangar_room_no_destination_is_found() {
        // FUN_004cd040's fallback FUN_0041a4c0(0x100005, 0xc02, 0x17): a
        // fleet en route (2) or without room (0x100000) is not one.
        let (mut f, _) = squadron_fixture();
        let at = f.empty;
        let fleet = carrier(&mut f, at);
        let mut state = AIState::new(AiFaction::Alliance);
        for flags in [0x10_0007, 5] {
            let found = edited(
                &f,
                &mut state,
                ROOMY,
                ALONE,
                |g| fleet_record(g, fleet, flags, 1, 0, 0),
                |m, io| (m.choose_destination(io), m.destination, m.count),
            );
            assert_eq!(found, (false, None, 0));
        }
        let found = edited(
            &f,
            &mut state,
            ROOMY,
            ALONE,
            |g| fleet_record(g, fleet, 0x10_0005, 1, 0, 0),
            |m, io| (m.choose_destination(io), m.destination_system(io)),
        );
        // port: the Destination names the fleet's system.
        assert_eq!(found, (true, Target::System(f.empty)));
    }

    #[test]
    fn the_squadron_plan_pays_within_the_budget_and_draws_its_key() {
        // FUN_004cd230: w = FUN_004cec20 (80% of 8% of 1000, 64); the class
        // by torpedoes (key 0x8000, values[6]) on a draw of 6 or more
        // (0x4cd253 cmp eax,6; setge), else by +0x34 (key 0x400); with
        // m * count > w, count = w / m.
        let (f, w) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        let mut plan = |tallies, draw, yard| {
            state.module_a.count = 20;
            state.module_a.yard = yard;
            let around = Around {
                planners: &[],
                draw,
            };
            edited(
                &f,
                &mut state,
                tallies,
                around,
                |_| {},
                |m, io| (m.plan_squadrons(io), m.count, m.fighter),
            )
        };
        assert_eq!(plan(ROOMY, 6, Yard::None), (9, 20, Some(w.cheap)));
        assert_eq!(plan(ROOMY, 5, Yard::None), (9, 12, Some(w.strong)));
        // FUN_00528040(0): the manager's two shipyards cap the count.
        assert_eq!(plan(ROOMY, 9, Yard::Manager(f.yard)), (9, 2, Some(w.cheap)));
        // No budget: 0; over it: 5 (FUN_004cd310 next).
        let none = Tallies {
            capacity: 0,
            ..ROOMY
        };
        assert_eq!(plan(none, 0, Yard::None).0, 0);
        let over = Tallies {
            wing_upkeep: 65,
            ..ROOMY
        };
        assert_eq!(plan(over, 0, Yard::None).0, 5);
        // A budget below one squadron plans nothing.
        let poor = Tallies {
            wing_upkeep: 62,
            ..ROOMY
        };
        assert_eq!(plan(poor, 0, Yard::None), (0, 0, Some(w.strong)));
    }

    #[test]
    fn over_budget_the_fleet_with_most_squadrons_scraps_its_weakest() {
        // FUN_004cca20 8 (FUN_004cd2f0: w < 0) -> 5 (FUN_004cd310): the
        // free fleet with the most squadrons (key 9, order 1), its ship
        // with the most aboard (key 6), the squadron with the lowest slot
        // +0x1f4: Scrap 0x200. The cycle ends with 0x3ec.
        let (mut f, w) = squadron_fixture();
        let (empty, yard) = (f.empty, f.yard);
        let bare = carrier(&mut f, empty);
        let full = carrier(&mut f, yard);
        crate::squadrons::add(&mut f.world.fleets[full].fighters, w.cheap, 1);
        crate::squadrons::add(&mut f.world.fleets[full].fighters, w.strong, 1);
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_a.phase = SQUADRON_CYCLE;
        state.module_a.state = 8;
        let over = Tallies {
            wing_upkeep: 100,
            ..ROOMY
        };
        let edit = |g: &mut Galaxy| {
            fleet_record(g, bare, 5, 0, 0, 0);
            fleet_record(g, full, 5, 0, 0, 2);
        };
        let s = edited(&f, &mut state, over, ALONE, edit, |m, io| m.step(io));
        assert_eq!((s, state.module_a.state), (Step::pending(), 5));
        let s = edited(&f, &mut state, over, ALONE, edit, |m, io| m.step(io));
        assert_eq!(
            s.order,
            Some(AIAction::Scrap {
                team: vec![ScrapTarget::FleetFighter {
                    fleet: full,
                    class: w.strong,
                }],
            })
        );
        assert!(s.complete);
        assert_eq!(
            (state.module_a.phase, state.module_a.state),
            (YARD_CYCLE, 0)
        );
        // A fleet en route is not taken, and a ship with none aboard gives
        // no Scrap.
        let edit = |g: &mut Galaxy| {
            fleet_record(g, bare, 5, 0, 0, 0);
            fleet_record(g, full, 7, 0, 0, 2);
        };
        let order = edited(&f, &mut state, over, ALONE, edit, |_, io| relieve(io));
        assert_eq!(order, None);
    }

    #[test]
    fn the_yard_cycle_borrows_a_yard_under_token_0x10_and_builds_shipyards() {
        // FUN_004ccb50 3: bit 0x10 cleared; with +0xc == 0x10, FUN_004cd860
        // takes the yard nearest the target (module 2's FUN_004e3810) and
        // makes the target a site marked 0x200000; FUN_004199b0; 6
        // (FUN_004cda90) -> 4 (FUN_004cdfd0, 20% of 8%, the shipyard
        // class) gives 9 -> Destination (FUN_004ce5b0) -> Build
        // (FUN_004ce6f0).
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.yard_marks.insert(f.yard, MARK_MINES);
        state.controller.yard_holder = YARD_TOKEN;
        state.controller.yard_requests = YARD_TOKEN | 0x80;
        state.module_a.phase = YARD_CYCLE;
        state.module_a.state = 3;
        state.module_a.target = Target::System(f.empty);
        state.module_a.room = 9;
        let lent = |g: &mut Galaxy| {
            g.sectors[0].candidates |= 0x1000;
            let r = rec(g, f.yard);
            r.state |= 0x800;
            r.values[6] = 2;
        };
        let s = edited(&f, &mut state, ROOMY, ALONE, lent, |m, io| m.step(io));
        assert_eq!(s, Step::pending());
        assert_eq!(state.module_a.state, 6);
        assert_eq!(
            (state.module_a.yard, state.module_a.capacity),
            (Yard::Manager(f.yard), 1)
        );
        assert_eq!(state.module_a.sites, [f.empty]);
        assert_eq!(state.yard_marks.get(&f.empty), Some(&MARK_SHIPYARD));
        assert_eq!(state.controller.yard_requests, 0);
        assert_eq!(state.controller.yard_holder, 0x80);
        let step = |state: &mut AIState, tallies| {
            edited(&f, state, tallies, ALONE, |_| {}, |m, io| m.step(io))
        };
        assert_eq!(step(&mut state, ROOMY), Step::pending());
        assert_eq!(state.module_a.state, 4);
        let rich = Tallies {
            capacity: 10_000,
            ..ROOMY
        };
        assert_eq!(step(&mut state, rich), Step::pending());
        assert_eq!((state.module_a.state, state.module_a.count), (9, 1));
        let s = step(&mut state, rich);
        assert!(s.complete && s.raise);
        assert_eq!(
            s.order,
            Some(AIAction::SetDestination {
                system: f.yard,
                area: ProductionArea::ConstructionYard,
                destination: f.empty,
            })
        );
        let s = step(&mut state, rich);
        let Some(AIAction::Build { kind, count, .. }) = s.order else {
            panic!("{s:?}");
        };
        assert_eq!(
            (kind.class_dat_id(), count),
            (Some(DatId::new(SHIPYARD_CLASS)), 1)
        );
        assert_eq!(
            (state.module_a.phase, state.module_a.state),
            (SQUADRON_CYCLE, 0)
        );
    }

    #[test]
    fn the_shipyard_plan_scraps_a_surplus_and_ends_otherwise() {
        // FUN_004cdfd0: the budget below 0 gives 7, which scraps a spare
        // shipyard (FUN_004ce070); 0 gives 0.
        let (f, _) = squadron_fixture();
        let mut state = with_site(&f);
        let heavy = |g: &mut Galaxy| rec(g, f.yard).values[34] = 500;
        let plan = edited(&f, &mut state, ROOMY, ALONE, heavy, |m, io| {
            m.plan_yards(io)
        });
        assert_eq!(plan, 7);
        let none = Tallies {
            capacity: 0,
            ..ROOMY
        };
        let mut bare = AIState::new(AiFaction::Alliance);
        let plan = edited(&f, &mut bare, none, ALONE, |_| {}, |m, io| m.plan_yards(io));
        assert_eq!(plan, 0);
    }

    #[test]
    fn the_budgets_are_eighty_and_twenty_percent_of_eight() {
        // FUN_004cec20 and FUN_004cec80 over +0x24 = 8 (FUN_004cc6b0).
        let (f, _) = squadron_fixture();
        let mut state = with_site(&f);
        let t = Tallies {
            capacity: 1000,
            wing_upkeep: 30,
            ..ROOMY
        };
        let held = |g: &mut Galaxy| rec(g, f.yard).values[34] = 4;
        let budgets = edited(&f, &mut state, t, ALONE, held, |m, io| {
            (squadron_budget(io), yard_budget(io, &m.sites))
        });
        assert_eq!(budgets, (80 * 80 / 100 - 30, 80 * 20 / 100 - 4));
        let short = Tallies {
            requirement: 990,
            ..t
        };
        let budgets = edited(&f, &mut state, short, ALONE, held, |m, io| {
            (squadron_budget(io), yard_budget(io, &m.sites))
        });
        assert_eq!(budgets, (0, 0));
    }

    #[test]
    fn a_build_without_a_squadron_class_resets_the_module() {
        // FUN_004ce490 gives Build 0x211 with no class; the validation
        // refuses it and slot +0x18 (FUN_004dd810) clears +0x38 and +0x3c.
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_a.phase = SQUADRON_CYCLE;
        state.module_a.state = 10;
        state.module_a.yard = Yard::Manager(f.yard);
        let s = edited(&f, &mut state, ROOMY, ALONE, |_| {}, |m, io| m.step(io));
        assert_eq!(s, Step::complete());
        assert_eq!((state.module_a.phase, state.module_a.state), (0, 0));
        // Without a manager no Build is made: the cycle ends.
        state.module_a.phase = SQUADRON_CYCLE;
        state.module_a.state = 10;
        state.module_a.yard = Yard::None;
        let s = edited(&f, &mut state, ROOMY, ALONE, |_| {}, |m, io| m.step(io));
        assert!(s.complete && s.order.is_none());
        assert_eq!(state.module_a.phase, YARD_CYCLE);
    }

    #[test]
    fn the_squadron_budget_holds_at_its_edges() {
        // FUN_004cec20: the budget stands while the free maintenance equals
        // it (`free < w` gives 0); FUN_004cd2f0 asks for below 0, so a budget
        // of 0 hands on to the yards (FUN_004cca20 state 8).
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        let even = Tallies {
            requirement: 1000 - 64,
            ..ROOMY
        };
        let budget = edited(
            &f,
            &mut state,
            even,
            ALONE,
            |_| {},
            |_, io| squadron_budget(io),
        );
        assert_eq!(budget, 64);
        let none = Tallies {
            wing_upkeep: 64,
            ..ROOMY
        };
        state.module_a.phase = SQUADRON_CYCLE;
        state.module_a.state = 8;
        let s = edited(&f, &mut state, none, ALONE, |_| {}, |m, io| m.step(io));
        assert_eq!(s, Step::pending());
        assert_eq!(
            (state.module_a.phase, state.module_a.state),
            (YARD_CYCLE, 0)
        );
    }

    #[test]
    fn a_budget_of_one_pays_for_one_squadron_of_upkeep_one() {
        // FUN_004cd230: only w < 1 stops the plan; w = 1 buys one squadron
        // of maintenance 1 (64 less 63).
        let (mut f, _) = squadron_fixture();
        let thrifty = f.world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            squadron_size: 1,
            maintenance_cost: 1,
            refined_material_cost: 7,
            laser_cannon_fore: 1,
            maneuverability: 100,
            ..FighterClass::default()
        });
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_a.count = 3;
        let one = Tallies {
            wing_upkeep: 63,
            ..ROOMY
        };
        let plan = edited(
            &f,
            &mut state,
            one,
            ALONE,
            |_| {},
            |m, io| (m.plan_squadrons(io), m.count, m.fighter),
        );
        assert_eq!(plan, (9, 1, Some(thrifty)));
    }

    #[test]
    fn the_shipyard_plan_divides_the_budget_by_the_class_s_upkeep() {
        // FUN_004cdfd0: count = w / m (160 / 10), capped by +0x6c, the
        // target's v[15] and +0x70.
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_a.target = Target::System(f.empty);
        state.module_a.capacity = 100;
        state.module_a.room = 100;
        let rich = Tallies {
            capacity: 10_000,
            ..ROOMY
        };
        let wide = |g: &mut Galaxy| rec(g, f.empty).values[15] = 100;
        let plan = edited(&f, &mut state, rich, ALONE, wide, |m, io| {
            (m.plan_yards(io), m.count)
        });
        assert_eq!(plan, (9, 16));
    }

    #[test]
    fn the_spare_shipyard_state_ends_the_yard_cycle() {
        // FUN_004ccb50 7 (FUN_004ce070) ends with 0x3f2, complete, with no
        // shipyard to spare here.
        let (f, _) = squadron_fixture();
        let mut state = AIState::new(AiFaction::Alliance);
        state.module_a.phase = YARD_CYCLE;
        state.module_a.state = 7;
        let s = edited(&f, &mut state, ROOMY, ALONE, |_| {}, |m, io| m.step(io));
        assert!(s.complete);
        assert_eq!(
            (state.module_a.phase, state.module_a.state),
            (SQUADRON_CYCLE, 0)
        );
    }

    #[test]
    fn only_the_planner_s_own_fleets_count_toward_its_base() {
        // FUN_0041acd0(id, 1, ..): a held fleet record without the own bit
        // 1 adds no hangar room.
        let (mut f, _) = squadron_fixture();
        let at = f.empty;
        let fleet = carrier(&mut f, at);
        let planners = [planner(4, f.empty)];
        let around = Around {
            planners: &planners,
            draw: 0,
        };
        let mut state = AIState::new(AiFaction::Alliance);
        let walk = |state: &mut AIState, flags| {
            edited(
                &f,
                state,
                ROOMY,
                around,
                |g| fleet_record(g, fleet, flags, 3, 4, 0),
                |m, io| (m.choose_destination(io), m.destination),
            )
        };
        assert_eq!(
            walk(&mut state, 5),
            (true, Some(SquadronPlace::Planet(f.empty)))
        );
        assert_eq!(walk(&mut state, 4), (false, None));
    }

    #[test]
    fn a_relief_needs_an_own_free_ship_with_a_squadron_aboard() {
        // FUN_004cd310: FUN_0041ac80(0x24000, 0x800, 6, 1) takes only own
        // free ships; one squadron aboard is enough.
        let (mut f, w) = squadron_fixture();
        let at = f.yard;
        let fleet = carrier(&mut f, at);
        crate::squadrons::add(&mut f.world.fleets[fleet].fighters, w.cheap, 1);
        let mut state = AIState::new(AiFaction::Alliance);
        let relief = |state: &mut AIState, ship_flags| {
            edited(
                &f,
                state,
                ROOMY,
                ALONE,
                |g| {
                    fleet_record(g, fleet, 5, 0, 0, 1);
                    g.fleets.get_mut(&fleet).unwrap().ships[0].flags = ship_flags;
                },
                |_, io| relieve(io),
            )
        };
        assert_eq!(
            relief(&mut state, SHIP_FREE | SHIP_OWN),
            Some(AIAction::Scrap {
                team: vec![ScrapTarget::FleetFighter {
                    fleet,
                    class: w.cheap,
                }],
            })
        );
        assert_eq!(relief(&mut state, SHIP_OWN), None);
    }

    #[test]
    fn the_weakest_squadron_is_the_first_smallest_with_any_left() {
        // Slot +0x1f4 (FUN_005031e0): the smallest class, the first on a
        // tie; an emptied entry is no squadron.
        let (mut f, w) = squadron_fixture();
        let twin = f.world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            squadron_size: 6,
            ..FighterClass::default()
        });
        let tiny = f.world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            squadron_size: 1,
            ..FighterClass::default()
        });
        let at = f.yard;
        let fleet = carrier(&mut f, at);
        let wings = &mut f.world.fleets[fleet].fighters;
        crate::squadrons::add(wings, w.cheap, 1);
        crate::squadrons::add(wings, w.strong, 1);
        crate::squadrons::add(wings, twin, 1);
        crate::squadrons::add(wings, tiny, 1);
        wings.last_mut().unwrap().count = 0;
        let mut state = AIState::new(AiFaction::Alliance);
        let weakest = edited(
            &f,
            &mut state,
            ROOMY,
            ALONE,
            |_| {},
            |_, io| weakest_squadron(io, fleet),
        );
        assert_eq!(weakest, Some(w.strong));
    }
}
