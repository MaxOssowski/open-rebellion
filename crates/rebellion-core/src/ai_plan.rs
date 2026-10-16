//! A sector planner's plan object (`FUN_004ad8e0`, vtable kinds by slot
//! `+0x14`) and its step (`FUN_004adb70`), which runs as state 5 of the
//! planner's step: the plan's base, the regiments and squadrons of its
//! held fleets, one idle held fleet a cycle with its people and its role,
//! and the roles' orders (`ghidra/notes/ai-sector-plans.md`, "The plan
//! object's step").
//!
//! port: only kinds 1 to 5's slots are ported (vtables `0x65c2c0`,
//! `0x65c268`, `0x65c160`, `0x65c210`, `0x65c108`).

use crate::ai::AIAction;
use crate::ai_module1::ANY;
use crate::ai_module8::SectorPlanner;
use crate::ai_planner::PlannerIo;
use crate::ai_records::{
    pick, picks, ranked, squared_distance, FleetRecord, Mask, Order, SystemRecord,
};
use crate::ids::{CharacterKey, FighterKey, FleetKey, SystemKey, TroopKey};
use crate::squadrons::SquadronPlace;

/// System record `+0x30` bit `0x40000`: a sector plan's base
/// (`FUN_004af8a0`); `0x10000` and `0x20000` mark the bases of other
/// kinds, and a planner's reset clears all three (`FUN_00477190`).
pub const BASE: u32 = 0x4_0000;
/// The base marks of every kind, which a reset clears (`FUN_00477190`,
/// mask `0xfff8ffff`).
pub const BASES: u32 = 0x7_0000;

/// System record `+0x30` bits `0x1f00000`: the roles of the held fleets
/// bound there (`FUN_004af6d0`).
const ROLE_MARKS: u32 = 0x1f0_0000;

/// Fleet record `+0x38` bits `0xf0000000`: the fleet's role in its plan
/// (`FUN_004b1070` tests them, then keeps `& 0x0fffffff`).
const ROLES: u32 = 0xf000_0000;
/// Role `0x10000000`: attack the enemy's fleets at the target
/// (`FUN_004b4560`), marked `0x100000` on the system.
const ROLE_ATTACK: u32 = 0x1000_0000;
/// Role `0x20000000`: assault the target (`FUN_004b8ab0`), marked
/// `0x200000` on the system.
const ROLE_ASSAULT: u32 = 0x2000_0000;
/// Role `0x40000000`: bombard the target (`FUN_004b4730`), marked
/// `0x400000` on the system.
const ROLE_BOMBARD: u32 = 0x4000_0000;
/// Role `0x80000000`: land regiments on a neutral system (kind 5's
/// `FUN_004b2f40`), marked `0x1000000` on the system.
const ROLE_OCCUPY: u32 = 0x8000_0000;
/// Each role and the mark it puts on its target system, in the order
/// `FUN_004af6d0` tests them.
const ROLE_MARK_OF: [(u32, u32); 4] = [
    (ROLE_OCCUPY, 0x100_0000),
    (ROLE_ATTACK, 0x10_0000),
    (ROLE_ASSAULT, 0x20_0000),
    (ROLE_BOMBARD, 0x40_0000),
];

/// Fleet record `+0x38` bits a held fleet must lack to be the step's
/// fleet (`FUN_004b04c0`): a role (`0xf0000000`), damaged or inactive
/// ships (`0x1000`, `0x2000`), en route (`2`, `0x800`).
const BUSY: u32 = 0xf000_3802;

/// Planner `+0x60` bit `0x1000000` read by `FUN_004b06b0`, set with its
/// movement planners (`FUN_00478140`): the flagship's fleet stays.
const FLAGSHIP_STAYS: u32 = 0x100_0000;
/// Planner `+0x60` bit `0x4000000` read by `FUN_004b06b0`: the
/// flagship's fleet goes to `+0x3c`.
const FLAGSHIP_MOVES: u32 = 0x400_0000;

/// The base marks of kinds 4 and 5's slot `+0x34`.
#[derive(Debug, Clone, Copy)]
struct BaseMarks {
    /// Taken by the base.
    set: u32,
    /// Dropped as it takes them.
    clear: u32,
    /// A kept base with this bit is left as it is.
    kept: u32,
    /// Dropped with a lost base.
    release: u32,
}

/// Kind 4's (`FUN_004b8070`): `| 0x50000`, released `& 0xfffaffff`.
const INVASION_MARKS: BaseMarks = BaseMarks {
    set: 0x5_0000,
    clear: 0,
    kept: 0x1_0000,
    release: 0x5_0000,
};

/// Kind 5's (`FUN_004b24b0`): `& 0xfffeffff | 0x60000`, released
/// `& 0xfff8ffff`.
const CONSOLIDATION_MARKS: BaseMarks = BaseMarks {
    set: 0x6_0000,
    clear: 0x1_0000,
    kept: 0x2_0000,
    release: BASES,
};

/// Where state `0xc` sends the step's fleet (`FUN_004b0c90`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Regroup {
    /// Join another held fleet.
    Fleet(FleetKey),
    /// Go to a system (slot `+0x54`).
    System(SystemKey),
}

impl SectorPlanner {
    /// One step of the plan object (`FUN_004adb70`): its order, and true
    /// when the plan's step ended (`*out`, state 0).
    pub fn plan_step(&mut self, io: &mut PlannerIo<'_>) -> (Option<AIAction>, bool) {
        let order = match self.plan_state {
            1 => {
                self.mark_roles(io);
                self.plan_state = 2;
                None
            }
            2 => {
                let based = match self.kind {
                    4 => self.invasion_base(io, INVASION_MARKS),
                    5 => self.invasion_base(io, CONSOLIDATION_MARKS),
                    _ => self.choose_base(io),
                };
                self.plan_state = if based { 3 } else { 5 };
                None
            }
            3 => {
                self.plan_state = 4;
                if matches!(self.kind, 4 | 5) {
                    self.board(io)
                } else {
                    self.unload(io)
                }
            }
            4 => {
                self.plan_state = 5;
                self.base_squadrons(io)
            }
            5 => {
                self.plan_state = if self.next_fleet(io) { 6 } else { 0xe };
                None
            }
            6 => {
                let (order, done) = self.disembark(io);
                self.plan_state = if done { 0xe } else { 7 };
                order
            }
            7 => {
                self.plan_state = if self.attack_target(io) { 0xd } else { 8 };
                None
            }
            // Slot +0x40: kind 4's assault (FUN_004b8ab0); the other ported
            // kinds' are 0x6158b0, which returns 0.
            8 => {
                self.plan_state = if self.kind == 4 && self.assault_target(io) {
                    0xd
                } else {
                    10
                };
                None
            }
            // Slot +0x48: kind 5's occupier (FUN_004b2f40); the other
            // ported kinds' are 0x6158b0.
            10 => {
                self.plan_state = if self.occupy_target(io) { 0xd } else { 9 };
                None
            }
            9 => {
                self.plan_state = if self.bombard_target(io) { 0xd } else { 0xc };
                None
            }
            0xc => {
                self.plan_state = 0xe;
                self.regroup(io)
            }
            0xd => {
                self.plan_state = 0xe;
                self.move_to_target(io)
            }
            0xe => {
                let (order, ended) = self.settle_roles(io);
                if ended {
                    self.plan_state = 0;
                }
                return (order, ended);
            }
            _ => {
                self.plan_state = 1;
                None
            }
        };
        (order, false)
    }

    /// State 1 (`FUN_004af6d0`): the sector's systems lose the role marks
    /// `0x1f00000`; each held fleet with a role marks its target system
    /// ([`ROLE_MARK_OF`], the first role it has).
    fn mark_roles(&self, io: &mut PlannerIo<'_>) {
        let Some(record) = self.sector.and_then(|s| io.galaxy.sector(s)) else {
            return;
        };
        let systems: Vec<SystemKey> = record.systems.iter().map(|r| r.system).collect();
        for system in systems {
            io.set_kind(system, 0, ROLE_MARKS);
        }
        for fleet in &self.fleets {
            let Some(record) = io.galaxy.fleets.get(fleet) else {
                continue;
            };
            let Some(target) = record.target.filter(|&t| io.galaxy.system(t).is_some()) else {
                continue;
            };
            let flags = record.flags;
            if let Some(&(_, mark)) = ROLE_MARK_OF.iter().find(|(role, _)| flags & role != 0) {
                io.set_kind(target, mark, 0);
            }
        }
    }

    /// State 2, slot `+0x34` (`FUN_004af8a0`): keep the base while it is
    /// the side's (`+0x30` 1), not blockaded (`+0x28` 2), and the
    /// headquarters when the sector has one (sector `+0x24` `0x40000000`,
    /// system `+0x28` `0x80000000`), dropping the other kinds' marks
    /// `0x30000`; else its marks `0x70000` go and so does it. Without one,
    /// the sector's own headquarters not blockaded and no base (`+0x30`
    /// forbidding `0x40000`), by key `0x26`, order 1; else such a system
    /// of any kind; else one of the nearest sector that has one
    /// (`FUN_0041b2b0`). The new base takes `0x40000`. True with a base.
    fn choose_base(&mut self, io: &mut PlannerIo<'_>) -> bool {
        if let Some(base) = self.base {
            if let Some(r) = io.galaxy.system(base) {
                let (state, kind) = (r.state, r.kind);
                let at_headquarters = self
                    .sector
                    .and_then(|s| io.galaxy.sector(s))
                    .is_none_or(|s| s.candidates & 0x4000_0000 == 0 || state & 0x8000_0000 != 0);
                if at_headquarters && kind & 1 != 0 && state & 2 == 0 {
                    if kind & 0x3_0000 != 0 {
                        io.set_kind(base, 0, 0x3_0000);
                    }
                    return true;
                }
                io.set_kind(base, 0, BASES);
                self.base = None;
            }
        }
        if self.base.is_some() {
            return false;
        }
        let Some(sector) = self.sector else {
            return false;
        };
        let free = Mask::new(1, BASE);
        for state in [Mask::new(0x8000_0000, 2), Mask::new(0, 2)] {
            let mut coin = || (io.random)(10) < 5;
            let systems = io.galaxy.system_query(
                sector,
                [state, ANY, free],
                0x26,
                Order::Ascending,
                &mut coin,
            );
            if let Some(system) = pick(&[systems]) {
                self.base = Some(system);
                break;
            }
        }
        if self.base.is_none() {
            self.base =
                crate::ai_planner::nearest_sector_system(io, sector, [Mask::new(0, 2), ANY, free]);
        }
        let Some(base) = self.base.filter(|&b| io.galaxy.system(b).is_some()) else {
            return false;
        };
        io.set_kind(base, BASE, 0x3_0000);
        true
    }

    /// `FUN_0041acd0`: the fleets this planner holds (record `+0x30`),
    /// in the world's order, whose `+0x38` passes `mask`, sorted on
    /// `values[key]`.
    fn held_fleets(
        &self,
        io: &mut PlannerIo<'_>,
        mask: Mask,
        key: usize,
        order: Order,
    ) -> Vec<FleetKey> {
        let items: Vec<(FleetKey, i32)> = io
            .world
            .fleets
            .keys()
            .filter_map(|fleet| {
                let r = io.galaxy.fleets.get(&fleet)?;
                (r.owner == self.id && mask.admits(r.flags)).then(|| (fleet, r.values[key]))
            })
            .collect();
        let mut coin = || (io.random)(10) < 5;
        ranked(items, order, &mut coin)
    }

    /// State 3, slot `+0x38` (`FUN_004afce0`): of the held fleets carrying
    /// regiments (`0x40001`, forbidding a role and en route `0xf0000002`;
    /// key `0xc`, order 1), the first at a system of the side's
    /// (`+0x30` 1) unloads there (Move `0x201`) the regiments of its held
    /// ships (`FUN_0041ac80`: `0x4000`, forbidding `0x20800`; key 9, order
    /// 1).
    ///
    /// port: a fleet's regiments are its cargo, which its first ship's
    /// record carries ([`crate::ai_records::FleetRecord::build`]).
    fn unload(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        let fleets = self.held_fleets(io, Mask::new(0x4_0001, 0xf000_0002), 0xc, Order::Ascending);
        for fleet in picks(vec![fleets]) {
            let Some(location) = io.world.fleets.get(fleet).map(|f| f.location) else {
                continue;
            };
            if io.galaxy.system(location).is_none_or(|r| r.kind & 1 == 0) {
                continue;
            }
            let Some(record) = io.galaxy.fleets.get(&fleet) else {
                continue;
            };
            let first = record.ships.first().map(|s| s.id);
            let ships: Vec<(u64, i32)> = record
                .ships
                .iter()
                .filter(|s| Mask::new(0x4000, 0x2_0800).admits(s.flags))
                .map(|s| (s.id, s.values[9]))
                .collect();
            let mut coin = || (io.random)(10) < 5;
            let ships = ranked(ships, Order::Ascending, &mut coin);
            let mut troops: Vec<TroopKey> = Vec::new();
            for ship in picks(vec![ships]) {
                if Some(ship) == first {
                    troops.extend_from_slice(io.transport.cargo(fleet));
                }
            }
            if !troops.is_empty() {
                return Some(AIAction::MoveRegiments {
                    troops,
                    to: location,
                });
            }
        }
        None
    }

    /// State 4 (`FUN_004b0120`): when the base has active squadrons on its
    /// planet (`v[25]`), the held fleet with the most hangar room (`1`,
    /// forbidding en route `2`; key `0x17`, order 1) found at the base
    /// takes as many as it has room for (fleet `values[23]`); a fleet
    /// found there without room ends the search. Move `0x201`.
    ///
    /// hyp: the planet's walk (`FUN_005039d0`, mode 1) gives the side's
    /// squadrons in the planet's order.
    fn base_squadrons(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        let base = self.base?;
        let on_planet = io.galaxy.system(base)?.values[25];
        if on_planet <= 0 {
            return None;
        }
        let fleets = self.held_fleets(io, Mask::new(1, 2), 0x17, Order::Ascending);
        let mut chosen = None;
        for fleet in picks(vec![fleets]) {
            let (Some(record), Some(value)) =
                (io.galaxy.fleets.get(&fleet), io.world.fleets.get(fleet))
            else {
                continue;
            };
            if value.location != base {
                continue;
            }
            if record.values[23] > 0 {
                chosen = Some((fleet, record.values[23], value.is_alliance));
            }
            break;
        }
        let (fleet, room, alliance) = chosen?;
        let mut left = u32::try_from(room.min(on_planet)).unwrap_or(0);
        let mut team: Vec<(FighterKey, u32)> = Vec::new();
        for entry in &io.world.systems.get(base)?.fighters {
            if left == 0 {
                break;
            }
            if crate::squadrons::is_alliance(io.world, entry.class) != Some(alliance) {
                continue;
            }
            let take = entry.count.min(left);
            match team.iter_mut().find(|(class, _)| *class == entry.class) {
                Some((_, count)) => *count += take,
                None => team.push((entry.class, take)),
            }
            left -= take;
        }
        (!team.is_empty()).then_some(AIAction::MoveSquadronTeam {
            squadrons: team,
            from: SquadronPlace::Planet(base),
            to: SquadronPlace::Fleet(fleet),
        })
    }

    /// State 5 (`FUN_004b04c0`): from the held fleet `+0x2c` names (else
    /// the first), once round the list, the first whose record lacks
    /// [`BUSY`] becomes the step's fleet (`+0x28`); `+0x2c` names the one
    /// after it, or nothing once the search came round. True when found.
    fn next_fleet(&mut self, io: &PlannerIo<'_>) -> bool {
        let list = self.fleets.clone();
        let start = self
            .plan_cursor
            .and_then(|c| list.iter().position(|&f| f == c))
            .unwrap_or(0);
        let mut at = (!list.is_empty()).then_some(start);
        let mut found = false;
        while let Some(i) = at {
            if found {
                break;
            }
            let fleet = list[i];
            if io
                .galaxy
                .fleets
                .get(&fleet)
                .is_some_and(|r| r.flags & BUSY == 0)
            {
                self.plan_fleet = Some(fleet);
                found = true;
            }
            let next = (i + 1) % list.len();
            at = (next != start).then_some(next);
        }
        self.plan_cursor = at.filter(|_| found).map(|i| list[i]);
        found
    }

    /// State 6 (`FUN_004b06b0`): when the step's fleet holds the
    /// flagship, it stays (planner `+0x60` `0x1000000`) or goes to the
    /// movement planners' system `+0x3c` when that is not blockaded
    /// (`0x4000000`, Move `0x201`); either ends the plan's step. Else a
    /// fleet carrying the side's people (`0x4000000`), not en route, at a
    /// system of the side's, sets them down there (Move `0x201`): the
    /// people aboard its ships that are not en route and carry people
    /// (`0x600000`), when one of them is the side's (record `+0x30` 2) and
    /// no movement planner holds it (`+0x28`, `FUN_00476e50`). The second
    /// value ends the plan's step.
    ///
    /// port: no movement planner holds anyone, and the port names no
    /// flagship; a fleet's people are its characters, which its first
    /// ship's record carries.
    fn disembark(&mut self, io: &PlannerIo<'_>) -> (Option<AIAction>, bool) {
        let Some(fleet) = self.plan_fleet else {
            return (None, false);
        };
        let Some(record) = io.galaxy.fleets.get(&fleet) else {
            return (None, false);
        };
        let mut done = false;
        if self
            .flagship
            .is_some_and(|id| record.ships.iter().any(|s| s.id == id))
        {
            if self.flags & FLAGSHIP_STAYS != 0 {
                done = true;
            } else if self.flags & FLAGSHIP_MOVES != 0 {
                if let Some(to) = self
                    .station
                    .filter(|&s| io.galaxy.system(s).is_some_and(|r| r.state & 2 == 0))
                {
                    let ships = io.world.fleets.get(fleet).map_or_else(Vec::new, |f| {
                        f.capital_ships
                            .iter()
                            .enumerate()
                            .filter(|(_, s)| s.alive)
                            .map(|(i, _)| i)
                            .collect()
                    });
                    return (Some(AIAction::MoveShips { fleet, ships, to }), true);
                }
            }
        }
        if record.flags & 0x400_0002 != 0x400_0000 {
            return (None, done);
        }
        let Some(location) = io.world.fleets.get(fleet).map(|f| f.location) else {
            return (None, done);
        };
        if io.galaxy.system(location).is_none_or(|r| r.kind & 1 == 0) {
            return (None, done);
        }
        let mut people: Vec<CharacterKey> = Vec::new();
        let mut own = false;
        let carrier = record.ships.first();
        if carrier.is_some_and(|s| s.flags & 0x800 == 0 && s.flags & 0x60_0000 != 0) {
            for &character in io
                .world
                .fleets
                .get(fleet)
                .map_or(&[][..], |f| &f.characters)
            {
                let Some(&flags) = io.people.get(&character) else {
                    continue;
                };
                own |= flags & 2 != 0;
                people.push(character);
            }
        }
        let order = (own && !people.is_empty()).then_some(AIAction::MoveCharacters {
            characters: people,
            to: location,
        });
        (order, done)
    }

    /// State 7, kind 3's slot `+0x3c` (`FUN_004b4560`): a step's fleet
    /// with attack (`values[2]`) takes the first of the sector's systems
    /// with an enemy fleet and no attacker bound there (`+0x30` `0x200`,
    /// forbidding `0x100000`; key `0x27`, order 1) that it is strong
    /// enough for ([`fleet_prevails`]): the system takes `0x100000`, the
    /// fleet the target and [`ROLE_ATTACK`]. Kind 1 (`FUN_004bafe0`), with
    /// none found and the fleet's `values[3]` above 0, then tries the
    /// opponent's systems with `+0x2c` `0x400000` (`+0x30` 4, forbidding
    /// `0x100000`; key `0x1b`, order 1). Kind 2 (`FUN_004b9c90`) does the
    /// same and adds the target to its visited list (`FUN_004f57b0` on
    /// `+0x5c`). True when found.
    fn attack_target(&mut self, io: &mut PlannerIo<'_>) -> bool {
        let found = self.attack(io);
        if found && self.kind == 2 {
            let target = self
                .plan_fleet
                .and_then(|f| io.galaxy.fleets.get(&f)?.target);
            if let Some(target) = target {
                self.visited.push(target);
            }
        }
        found
    }

    /// Slot `+0x3c`'s lists (`FUN_004b4560`, `FUN_004bafe0`,
    /// `FUN_004b9c90`; [`Self::attack_target`]).
    fn attack(&self, io: &mut PlannerIo<'_>) -> bool {
        let (Some(fleet), Some(sector)) = (self.plan_fleet, self.sector) else {
            return false;
        };
        let Some(values) = io.galaxy.fleets.get(&fleet).map(|r| r.values) else {
            return false;
        };
        if values[2] > 0 {
            // Kind 4 (FUN_004b8880) first tries its target when an enemy
            // fleet is there (+0x30 0x200), with no other mark test.
            if let Some(target) = self.plan_target.filter(|_| self.kind == 4) {
                let wins = io.galaxy.system(target).is_some_and(|r| {
                    r.kind & 0x200 != 0
                        && io
                            .galaxy
                            .fleets
                            .get(&fleet)
                            .is_some_and(|f| fleet_prevails(f, r))
                });
                if wins {
                    io.set_kind(target, 0x10_0000, 0);
                    bind(io, fleet, target, ROLE_ATTACK);
                    return true;
                }
            }
            if attack_from(
                io,
                fleet,
                sector,
                [ANY, ANY, Mask::new(0x200, 0x10_0000)],
                0x27,
            ) {
                return true;
            }
        }
        matches!(self.kind, 1 | 2)
            && values[3] > 0
            && attack_from(
                io,
                fleet,
                sector,
                [ANY, Mask::new(0x40_0000, 0), Mask::new(4, 0x10_0000)],
                0x1b,
            )
    }

    /// State 8, kind 4's slot `+0x40` (`FUN_004b8ab0`): a step's fleet with
    /// regiments (`values[12]`) takes the target (`+0x5c`) when it can
    /// assault it ([`fleet_assaults`]): the system takes `0x200000`, the
    /// fleet the target and [`ROLE_ASSAULT`]. True when bound.
    fn assault_target(&mut self, io: &mut PlannerIo<'_>) -> bool {
        let (Some(fleet), Some(target)) = (self.plan_fleet, self.plan_target) else {
            return false;
        };
        if io
            .galaxy
            .fleets
            .get(&fleet)
            .is_none_or(|r| r.values[12] < 1)
            || io.galaxy.system(target).is_none()
            || !fleet_assaults(io, fleet, target)
        {
            return false;
        }
        io.set_kind(target, 0x20_0000, 0);
        bind(io, fleet, target, ROLE_ASSAULT);
        true
    }

    /// State 2, kind 4's slot `+0x34` (`FUN_004b8070`) and kind 5's
    /// (`FUN_004b24b0`), with their [`BaseMarks`]: a base (planner `+0x40`)
    /// whose record is not the side's (`+0x30` 1) or is blockaded (`+0x28`
    /// 2) loses the release marks and goes; else it takes the marks unless
    /// it has the kept bit, and the slot is true. Without a base: the
    /// sector's own headquarters not blockaded and no base (`+0x28`
    /// `0x80000000` forbidding 2, `+0x30` 1 forbidding `0x40000`; key
    /// `0x13`, order 1), else such a system of any kind, else one of the
    /// nearest sector that has one (`FUN_0041b2b0`); it takes the marks. A
    /// base without a record stays and the slot is false.
    fn invasion_base(&mut self, io: &mut PlannerIo<'_>, marks: BaseMarks) -> bool {
        if let Some(base) = self.base {
            let Some(r) = io.galaxy.system(base) else {
                return false;
            };
            let (state, kind) = (r.state, r.kind);
            if kind & 1 == 0 || state & 2 != 0 {
                io.set_kind(base, 0, marks.release);
                self.base = None;
            } else {
                if kind & marks.kept == 0 {
                    io.set_kind(base, marks.set, marks.clear);
                }
                return true;
            }
        }
        let Some(sector) = self.sector else {
            return false;
        };
        let free = Mask::new(1, BASE);
        for state in [Mask::new(0x8000_0000, 2), Mask::new(0, 2)] {
            let mut coin = || (io.random)(10) < 5;
            let systems = io.galaxy.system_query(
                sector,
                [state, ANY, free],
                0x13,
                Order::Ascending,
                &mut coin,
            );
            if let Some(system) = pick(&[systems]) {
                self.base = Some(system);
                break;
            }
        }
        if self.base.is_none() {
            self.base =
                crate::ai_planner::nearest_sector_system(io, sector, [Mask::new(0, 2), ANY, free]);
        }
        let Some(base) = self.base.filter(|&b| io.galaxy.system(b).is_some()) else {
            return false;
        };
        io.set_kind(base, marks.set, marks.clear);
        true
    }

    /// State 10, kind 5's slot `+0x48` (`FUN_004b2f40`): a step's fleet
    /// with regiments (`values[12]`) takes the first of the sector's
    /// neutral systems (`+0x30` 2, forbidding `0x100208`; key `0xf`, order
    /// 1) where it can land ([`fleet_occupies`]): the system takes
    /// `0x1000000`, the fleet the target and [`ROLE_OCCUPY`]. True when
    /// bound.
    fn occupy_target(&mut self, io: &mut PlannerIo<'_>) -> bool {
        if self.kind != 5 {
            return false;
        }
        let (Some(fleet), Some(sector)) = (self.plan_fleet, self.sector) else {
            return false;
        };
        if io
            .galaxy
            .fleets
            .get(&fleet)
            .is_none_or(|r| r.values[12] < 1)
        {
            return false;
        }
        let mut coin = || (io.random)(10) < 5;
        let systems = io.galaxy.system_query(
            sector,
            [ANY, ANY, Mask::new(2, 0x10_0208)],
            0xf,
            Order::Ascending,
            &mut coin,
        );
        for system in picks(vec![systems]) {
            if fleet_occupies(io, fleet, system) {
                io.set_kind(system, 0x100_0000, 0);
                bind(io, fleet, system, ROLE_OCCUPY);
                return true;
            }
        }
        false
    }

    /// State 3, kind 4's slot `+0x38` (`FUN_004b8470`): with the side's
    /// regiments at the base (its record's `v[19]`), the first held fleet
    /// (`1` forbidding en route `2`; key `0x16`, order 1) found at the base
    /// takes, when it has room (`values[22]`), the strongest of them (slot
    /// `+0x1dc`, TROOPSD attack; the first of equals, each once), as many as
    /// the room and `v[19]` allow. Move `0x201` onto the fleet.
    ///
    /// Kind 5's slot (`FUN_004b28c0`) needs the sector record too, and
    /// takes the best defense (slot `+0x1e0`) while the sector holds no
    /// opponent system (`+0x2c` 8).
    ///
    /// hyp: the base's walk (`FUN_00504c40`) gives the side's regiments on
    /// the planet in its order.
    fn board(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        let base = self.base?;
        let present = io.galaxy.system(base)?.values[19];
        let by_defense = if self.kind == 5 {
            let sector = self.sector.and_then(|s| io.galaxy.sector(s))?;
            sector.summary & 8 == 0
        } else {
            false
        };
        if present < 1 {
            return None;
        }
        let fleets = self.held_fleets(io, Mask::new(1, 2), 0x16, Order::Ascending);
        let fleet = picks(vec![fleets])
            .into_iter()
            .find(|&f| io.world.fleets.get(f).is_some_and(|v| v.location == base))?;
        let room = io.galaxy.fleets.get(&fleet)?.values[22];
        let is_alliance = io.world.fleets.get(fleet)?.is_alliance;
        if room < 1 {
            return None;
        }
        let candidates: Vec<(TroopKey, i32)> = io
            .world
            .systems
            .get(base)?
            .ground_units
            .iter()
            .filter_map(|&t| {
                let troop = io.world.troops.get(t)?;
                (troop.is_alliance == is_alliance).then(|| {
                    let strength = io
                        .world
                        .troop_classes
                        .get(&troop.class_dat_id)
                        .map_or(0, |c| {
                            let value = if by_defense {
                                c.defense_strength
                            } else {
                                c.attack_strength
                            };
                            i32::try_from(value).unwrap_or(i32::MAX)
                        });
                    (t, strength)
                })
            })
            .collect();
        let mut chosen: Vec<TroopKey> = Vec::new();
        for _ in 0..room.min(present) {
            let mut best: Option<(TroopKey, i32)> = None;
            // The search starts at 0: a regiment without attack (or
            // defense) is never taken.
            for &(t, attack) in &candidates {
                if !chosen.contains(&t) && attack > best.map_or(0, |(_, top)| top) {
                    best = Some((t, attack));
                }
            }
            if let Some((t, _)) = best {
                chosen.push(t);
            }
        }
        (!chosen.is_empty()).then_some(AIAction::BoardRegiments {
            troops: chosen,
            fleet,
        })
    }

    /// State 9, kind 3's slot `+0x44` (`FUN_004b4730`): a step's fleet
    /// with bombardment (`values[6]`) takes the first of the sector's
    /// systems of the opponent with no role bound there (`+0x30` 4,
    /// forbidding `0x700200`), merged by key 8 (order 2) and key `0x10`
    /// (order 1), that it can bombard ([`bombardment`]): the system
    /// takes `0x400000`, the fleet the target and [`ROLE_BOMBARD`]. True
    /// when found. Kind 1's slot (`FUN_004bb300`) merges three lists:
    /// key 8 (order 2), `0x21` and 0 (order 1). Kind 5's (`FUN_004b3110`)
    /// takes the opponent's held systems (`+0x30` `0x24`) by key 8 (order
    /// 2) and `0x21` (order 1).
    fn bombard_target(&mut self, io: &mut PlannerIo<'_>) -> bool {
        // Kinds 2 and 4's slot +0x44 is 0x6158b0, which returns 0.
        if matches!(self.kind, 2 | 4) {
            return false;
        }
        let (Some(fleet), Some(sector)) = (self.plan_fleet, self.sector) else {
            return false;
        };
        if io.galaxy.fleets.get(&fleet).is_none_or(|r| r.values[6] < 1) {
            return false;
        }
        let held = if self.kind == 5 { 0x24 } else { 4 };
        let masks = [ANY, ANY, Mask::new(held, 0x70_0200)];
        // Kind 1 (FUN_004bb300) merges key 8 (order 2), 0x21 and 0 (order
        // 1); kind 3 key 8 (order 2) and 0x10 (order 1).
        let keys: &[(usize, Order)] = match self.kind {
            1 => &[
                (8, Order::Descending),
                (0x21, Order::Ascending),
                (0, Order::Ascending),
            ],
            5 => &[(8, Order::Descending), (0x21, Order::Ascending)],
            _ => &[(8, Order::Descending), (0x10, Order::Ascending)],
        };
        let mut coin = || (io.random)(10) < 5;
        let lists = keys
            .iter()
            .map(|&(key, order)| io.galaxy.system_query(sector, masks, key, order, &mut coin))
            .collect();
        for system in picks(lists) {
            if io.galaxy.system(system).is_some() && bombardment(io, fleet, system).is_some() {
                io.set_kind(system, 0x40_0000, 0);
                bind(io, fleet, system, ROLE_BOMBARD);
                return true;
            }
        }
        false
    }

    /// State `0xc` (`FUN_004b0c90`): with more than one held fleet, the
    /// step's fleet joins the nearest other one (`FUN_0041ae20`: the
    /// planner's, `1` forbidding `0xf0000002`, by distance, order 2) whose
    /// system has no enemy fleet (`+0x30` `0x200`); else it goes to slot
    /// `+0x54`'s system ([`Self::home`]). Move `0x201`.
    fn regroup(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        let fleet = self.plan_fleet?;
        let mut target = None;
        if self.fleets.len() > 1 {
            for other in picks(vec![self.fleets_near(io, fleet)]) {
                if other == fleet {
                    continue;
                }
                let at = io.world.fleets.get(other).map(|f| f.location);
                if at
                    .and_then(|s| io.galaxy.system(s))
                    .is_some_and(|r| r.kind & 0x200 == 0)
                {
                    target = Some(Regroup::Fleet(other));
                    break;
                }
            }
        }
        match target.or_else(|| self.home(io).map(Regroup::System))? {
            Regroup::Fleet(into) => Some(AIAction::JoinFleet { fleet, into }),
            Regroup::System(to) => move_fleet(io, fleet, to),
        }
    }

    /// `FUN_0041ae20` from `fleet`: the fleets this planner holds
    /// (record `+0x30`) with `1` and without `0xf0000002`, in the world's
    /// order, sorted on the squared distance of their systems from
    /// `fleet`'s (order 2). Empty when `fleet`'s system has no record.
    fn fleets_near(&self, io: &mut PlannerIo<'_>, fleet: FleetKey) -> Vec<FleetKey> {
        let Some(home) = io
            .world
            .fleets
            .get(fleet)
            .and_then(|f| io.galaxy.system(f.location))
            .map(|r| r.position)
        else {
            return Vec::new();
        };
        let mask = Mask::new(1, 0xf000_0002);
        let items: Vec<(FleetKey, i32)> = io
            .world
            .fleets
            .iter()
            .filter_map(|(key, f)| {
                let r = io.galaxy.fleets.get(&key)?;
                if r.owner != self.id || !mask.admits(r.flags) {
                    return None;
                }
                let at = io.galaxy.system(f.location)?.position;
                Some((key, squared_distance(home, at)))
            })
            .collect();
        let mut coin = || (io.random)(10) < 5;
        ranked(items, Order::Descending, &mut coin)
    }

    /// Kinds 1 and 3's slot `+0x54` (`FUN_004b4a20`): the base when it is not
    /// blockaded (`+0x28` 2); else the sector's headquarters of the side's
    /// (`+0x28` `0x80000000` forbidding 2, `+0x30` 1; key 7, order 2);
    /// else such a system with every yard mark (`+0x28` `0x3e00000`; key
    /// `0x21`, order 1); else, when the step's fleet stands at a system of
    /// the opponent (`+0x30` 4), the sector's system nearest it with
    /// neither 4 nor `0x200` (`FUN_0041a400`, order 2).
    ///
    /// Kind 1 (`FUN_004bb630`) first sends a lone held fleet (planner
    /// `+0x2c` holds one) that carries no regiments and is not en route
    /// (record `+0x38` `0xa00`) to the sector's system not the side's,
    /// with no enemy fleet or role bound there (`+0x30` forbidding
    /// `0x700201`), seen longest ago (`FUN_0041a3a0`, order 1). Kind 2
    /// (`FUN_004ba0d0`) sends such a lone fleet, when its `values[2]` and
    /// `values[3]` reach controller `+0x164` and `+0x168` (read as 0), to
    /// the first system of the opponent not yet visited
    /// ([`SectorPlanner::sweep_left`]), which joins the visited list. Kind 5
    /// (`FUN_004b3400`) sends such a lone fleet, when the plan has no base,
    /// to a random system of the sector not yet explored (`+0x2c`
    /// `0x8000000`), not the side's and with no enemy fleet or role there
    /// (`FUN_0041a460`).
    fn home(&mut self, io: &mut PlannerIo<'_>) -> Option<SystemKey> {
        let fleet = self
            .plan_fleet
            .filter(|f| io.galaxy.fleets.contains_key(f))?;
        let lone = self.fleets.len() == 1
            && io
                .galaxy
                .fleets
                .get(&fleet)
                .is_some_and(|r| r.flags & 0xa00 == 0);
        if lone && self.kind == 2 {
            let strong = io
                .galaxy
                .fleets
                .get(&fleet)
                .is_some_and(|r| r.values[2] >= 0 && r.values[3] >= 0);
            if let Some(next) = self
                .sector
                .filter(|_| strong)
                .and_then(|sector| self.sweep_left(io, sector))
            {
                self.visited.push(next);
                return Some(next);
            }
        }
        if lone && self.kind == 5 && self.base.is_none() {
            // FUN_004b3400: a random system of the sector (FUN_0041a460)
            // not explored (+0x2c 0x8000000) and without +0x30 0x700201.
            if let Some(sector) = self.sector {
                let masks = [ANY, Mask::new(0, 0x800_0000), Mask::new(0, 0x70_0201)];
                if let Some(system) = crate::ai_planner::random_system(io, sector, masks) {
                    return Some(system);
                }
            }
        }
        if lone && self.kind == 1 {
            if let Some(sector) = self.sector {
                let mut coin = || (io.random)(10) < 5;
                let systems = io.galaxy.system_query_by_age(
                    sector,
                    [ANY, ANY, Mask::new(0, 0x70_0201)],
                    (io.day, io.seen),
                    Order::Ascending,
                    &mut coin,
                );
                if let Some(system) = pick(&[systems]) {
                    return Some(system);
                }
            }
        }
        // Kind 4 (FUN_004b8ca0) takes its base and the headquarters or a
        // yard without the blockade tests.
        let invasion = self.kind == 4;
        if let Some(base) = self
            .base
            .filter(|&b| invasion || io.galaxy.system(b).is_some_and(|r| r.state & 2 == 0))
        {
            return Some(base);
        }
        let sector = self.sector?;
        let mut coin = || (io.random)(10) < 5;
        let blockade = if invasion { 0 } else { 2 };
        for (state, key, order) in [
            (0x8000_0000, 7, Order::Descending),
            (0x3e0_0000, 0x21, Order::Ascending),
        ] {
            let systems = io.galaxy.system_query(
                sector,
                [Mask::new(state, blockade), ANY, Mask::new(1, 0)],
                key,
                order,
                &mut coin,
            );
            if let Some(system) = pick(&[systems]) {
                return Some(system);
            }
        }
        let location = io.world.fleets.get(fleet)?.location;
        if io.galaxy.system(location).is_none_or(|r| r.kind & 4 == 0) {
            return None;
        }
        let systems = io.galaxy.systems_by_distance(
            sector,
            location,
            [ANY, ANY, Mask::new(0, 0x204)],
            Order::Descending,
            &mut coin,
        );
        pick(&[systems])
    }

    /// State `0xd` (`FUN_004b0f90`): the step's fleet goes to its target
    /// (record `+0x2c`). Move `0x201`.
    fn move_to_target(&mut self, io: &mut PlannerIo<'_>) -> Option<AIAction> {
        let fleet = self.plan_fleet?;
        let to = io.galaxy.fleets.get(&fleet)?.target?;
        move_fleet(io, fleet, to)
    }

    /// State `0xe` (`FUN_004b1070`): from the held fleet `+0x30` names
    /// (else the first), each held fleet with a role, not en route (2),
    /// standing at its target (location `+0x28` is `+0x2c`, with a record):
    /// [`ROLE_ATTACK`] keeps its role while [`fleet_prevails`];
    /// [`ROLE_BOMBARD`] bombards (`FUN_004b1330`: Destroy System `0x223`
    /// with fleet `0x10` and system `v[50]` above 0, else [`bombardment`]'s
    /// code) and loses role and target. A fleet whose role fails, or that
    /// stands elsewhere, loses them too. An order stops the walk, `+0x30`
    /// naming the next fleet; the walk's end ends the plan's step (`*out`)
    /// and clears `+0x30`. The order, and true at the end.
    fn settle_roles(&mut self, io: &mut PlannerIo<'_>) -> (Option<AIAction>, bool) {
        let list = self.fleets.clone();
        let mut at = self
            .plan_walk
            .and_then(|w| list.iter().position(|&f| f == w))
            .unwrap_or(0);
        while at < list.len() {
            let fleet = list[at];
            at += 1;
            let order = settle_role(io, fleet);
            if order.is_some() {
                if at < list.len() {
                    self.plan_walk = Some(list[at]);
                    return (order, false);
                }
                self.plan_walk = None;
                return (order, true);
            }
        }
        self.plan_walk = None;
        (None, true)
    }
}

/// `FUN_004b1070`'s body for one held fleet.
fn settle_role(io: &mut PlannerIo<'_>, fleet: FleetKey) -> Option<AIAction> {
    let record = io.galaxy.fleets.get(&fleet)?;
    if record.flags & ROLES == 0 || record.flags & 2 != 0 {
        return None;
    }
    let location = io.world.fleets.get(fleet).map(|f| f.location);
    let there = record
        .target
        .filter(|&t| Some(t) == location)
        .and_then(|t| io.galaxy.system(t).map(|r| (t, r)));
    let mut order = None;
    let mut keep = false;
    if let Some((system, target)) = there {
        if record.flags & ROLE_ATTACK != 0 {
            keep = fleet_prevails(record, target);
        } else if record.flags & ROLE_ASSAULT != 0 {
            // FUN_004b1070: Destroy System with fleet 0x10 and v[50];
            // else Planetary Assault (0x234) while FUN_004ae240 holds;
            // else FUN_004ae320's bombardment. The role stays after an
            // order and goes only when none is given.
            order = if record.flags & 0x10 != 0 && target.values[50] > 0 {
                Some(AIAction::Bombard {
                    fleets: vec![fleet],
                    system,
                    kind: crate::bombardment::DESTROY_SYSTEM,
                })
            } else if fleet_assaults(io, fleet, system) {
                Some(AIAction::Assault {
                    fleets: vec![fleet],
                    system,
                })
            } else {
                bombardment(io, fleet, system).map(|kind| AIAction::Bombard {
                    fleets: vec![fleet],
                    system,
                    kind,
                })
            };
            keep = order.is_some();
        } else if record.flags & ROLE_BOMBARD != 0 {
            let kind = if record.flags & 0x10 != 0 && target.values[50] > 0 {
                Some(crate::bombardment::DESTROY_SYSTEM)
            } else {
                bombardment(io, fleet, system)
            };
            order = kind.map(|kind| AIAction::Bombard {
                fleets: vec![fleet],
                system,
                kind,
            });
        } else if record.flags & ROLE_OCCUPY != 0 {
            // FUN_004b1070: while FUN_004ae440 holds the role stays and
            // FUN_004b1440 lands one regiment.
            keep = fleet_occupies(io, fleet, system);
            if keep {
                order = land_one(io, fleet, system);
            }
        }
    }
    if !keep {
        io.edit_fleet(fleet, |r| {
            r.flags &= !ROLES;
            r.target = None;
        });
    }
    order
}

/// `FUN_004ae240`: whether `fleet` can assault `system`, a system held by
/// a side (`+0x30` `0x20`), not the side's and not fully shielded
/// (`0x101`). At the system both thresholds are 0; elsewhere the fleet's
/// defense (`values[3]`) must reach the system's shields (`v[27]`, or
/// controller `+0x168`), and its troops (`values[9]`) the system's
/// regiments (`v[23]`) unless it has fighters (`values[19]`). Its regiments
/// (`values[12]`) must reach the system's garrison need (`v[52]`).
///
/// hyp: controller `+0x168` reads as 0.
fn fleet_assaults(io: &PlannerIo<'_>, fleet: FleetKey, system: SystemKey) -> bool {
    let (Some(target), Some(record)) = (io.galaxy.system(system), io.galaxy.fleets.get(&fleet))
    else {
        return false;
    };
    if target.kind & 0x101 != 0 || target.kind & 0x20 == 0 {
        return false;
    }
    let at = io.world.fleets.get(fleet).map(|f| f.location) == Some(system);
    let v = &target.values;
    let (shields, troops) = if at { (0, 0) } else { (v[27].max(0), v[23]) };
    record.values[3] >= shields
        && v[52] <= record.values[12]
        && (record.values[19] > 0 || troops <= record.values[9])
}

/// `FUN_004ae440`: whether `fleet` can land on `system`, a neutral system
/// (`+0x30` 2) neither populated (8) nor with an enemy fleet (`0x200`),
/// carrying regiments (`values[12]`).
fn fleet_occupies(io: &PlannerIo<'_>, fleet: FleetKey, system: SystemKey) -> bool {
    let (Some(target), Some(record)) = (io.galaxy.system(system), io.galaxy.fleets.get(&fleet))
    else {
        return false;
    };
    target.kind & 2 != 0 && target.kind & 0x208 == 0 && record.values[12] > 0
}

/// `FUN_004b1440`: the first ship of `fleet` whose record carries
/// regiments (`values[9]`, `+0x68`) gives its first one, Move `0x201` to
/// `system`.
///
/// port: a fleet's regiments are its cargo, which its first ship's record
/// carries ([`crate::ai_records::FleetRecord::build`]).
fn land_one(io: &PlannerIo<'_>, fleet: FleetKey, system: SystemKey) -> Option<AIAction> {
    let record = io.galaxy.fleets.get(&fleet)?;
    if !record.ships.iter().any(|s| s.values[9] > 0) {
        return None;
    }
    let troop = *io.transport.cargo(fleet).first()?;
    Some(AIAction::MoveRegiments {
        troops: vec![troop],
        to: system,
    })
}

/// `FUN_004ae320`: the bombardment `fleet` would give `system`, a system
/// of the opponent (`+0x30` 4) with a sector record (`+0x3c`). The fleet
/// needs, when elsewhere, defense (`values[3]`) of at least the system's
/// shields (`v[27]`, or controller `+0x168` when higher), bombardment
/// (`values[6]`) above the system's `v[8]`, and the system's `v[11]` at 0.
/// Then with `x` half of sector `values[2] - values[4] + 1` above 0,
/// Target Military Facilities (`0x220`) when `v[7] + v[18]` is above `x`
/// and `v[18]` above 0; else General Bombardment (`0x222`) when `v[18]`
/// or `v[33]` is above 0.
///
/// hyp: controller `+0x168` has no writer found and reads as 0, as for
/// kind 3's wants (`FUN_004b4920`).
fn bombardment(io: &PlannerIo<'_>, fleet: FleetKey, system: SystemKey) -> Option<u32> {
    let target = io.galaxy.system(system).filter(|r| r.kind & 4 != 0)?;
    let sector = io
        .galaxy
        .sectors
        .iter()
        .find(|s| s.systems.iter().any(|r| r.system == system))?;
    let record = io.galaxy.fleets.get(&fleet)?;
    let location = io.world.fleets.get(fleet).map(|f| f.location);
    let v = &target.values;
    let shields = if location == Some(system) {
        0
    } else {
        v[27].max(0)
    };
    if shields > record.values[3] || record.values[6] <= v[8] || v[11] != 0 {
        return None;
    }
    let x = (sector.values[2] - sector.values[4] + 1) / 2;
    if x < 1 {
        (v[18] > 0 || v[33] > 0).then_some(0x222)
    } else {
        (v[7] + v[18] > x && v[18] > 0).then_some(0x220)
    }
}

/// A plan object binds `fleet` to `system` in `role` (record `+0x2c`,
/// `+0x38`), kept across refreshes.
/// One attack list of slot `+0x3c` (`FUN_004b4560`, `FUN_004bafe0`): the
/// sector's systems passing `masks`,
/// by `values[key]` (order 1), the first that `fleet` prevails at takes
/// `0x100000` and binds the fleet as its attacker. True when found.
fn attack_from(
    io: &mut PlannerIo<'_>,
    fleet: FleetKey,
    sector: crate::ids::SectorKey,
    masks: [Mask; 3],
    key: usize,
) -> bool {
    let mut coin = || (io.random)(10) < 5;
    let systems = io
        .galaxy
        .system_query(sector, masks, key, Order::Ascending, &mut coin);
    for system in picks(vec![systems]) {
        let (Some(record), Some(target)) = (io.galaxy.fleets.get(&fleet), io.galaxy.system(system))
        else {
            continue;
        };
        if fleet_prevails(record, target) {
            io.set_kind(system, 0x10_0000, 0);
            bind(io, fleet, system, ROLE_ATTACK);
            return true;
        }
    }
    false
}

fn bind(io: &mut PlannerIo<'_>, fleet: FleetKey, system: SystemKey, role: u32) {
    io.edit_fleet(fleet, |r| {
        r.target = Some(system);
        r.flags |= role;
    });
}

/// Move `0x201` of `fleet` to `to`: its living ships, carried out as the
/// fleet's move ([`crate::fleet_join::move_ships_to_system`]).
///
/// port: the port's move refuses the fleet's own system, which the
/// original's accepts and does nothing with (`FUN_00578d00`, step 5), so
/// no order is given there.
fn move_fleet(io: &PlannerIo<'_>, fleet: FleetKey, to: SystemKey) -> Option<AIAction> {
    let value = io.world.fleets.get(fleet)?;
    if value.location == to {
        return None;
    }
    let ships = value
        .capital_ships
        .iter()
        .enumerate()
        .filter(|(_, s)| s.alive)
        .map(|(i, _)| i)
        .collect();
    Some(AIAction::MoveShips { fleet, ships, to })
}

/// `FUN_004ae150`: whether `fleet` prevails at `system` over the enemy
/// there: with its fleets (`+0x30` `0x200`) their attack `v[39]`, defense
/// `v[40]` and `v[43]`; with its forces (4) adding `v[26]`, `v[27]` and
/// `v[23]`. The enemy's attack must be above 0 and the fleet's
/// (`values[2]`) above 1.2 times it; when the third is above 0, the
/// fleet's defense (`values[3]`) must be above the second, and its
/// `values[9]` above the third with fleet `0x10` or system `0x400`, else
/// at least the third unless its `values[19]` is above 0.
fn fleet_prevails(fleet: &FleetRecord, system: &SystemRecord) -> bool {
    let (mut attack, mut defense, mut third) = (0, 0, 0);
    let v = &system.values;
    if system.kind & 0x200 != 0 {
        (attack, defense, third) = (v[39], v[40], v[43]);
    }
    if system.kind & 4 != 0 {
        attack += v[26];
        defense += v[27];
        third += v[23];
    }
    if attack < 1 || attack.wrapping_mul(12) / 10 >= fleet.values[2] {
        return false;
    }
    if third < 1 {
        return true;
    }
    if fleet.values[3] <= defense {
        return false;
    }
    if fleet.flags & 0x10 != 0 || system.kind & 0x400 != 0 {
        fleet.values[9] > third
    } else {
        fleet.values[19] >= 1 || fleet.values[9] >= third
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_planner::tests::{fixture, fleet, planner, rebuild, ship_id, with, Fx};
    use crate::world::{Character, FighterClass, FighterEntry, TroopUnit};
    use std::collections::HashMap;

    /// A held fleet of `n` ships at system `at`, every ship held.
    fn held(fx: &mut Fx, p: &mut SectorPlanner, at: usize, n: u32) -> FleetKey {
        let f = fleet(fx, at, n);
        let ids: Vec<u64> = (0..n as usize).map(|i| ship_id(fx, f, i)).collect();
        with(fx, |io| {
            p.take_fleet(io, f);
            for id in ids {
                p.take_ship(io, id);
            }
        });
        f
    }

    /// What one plan step gives: its order while the step goes on, or with
    /// the step's end (state `0xe`).
    #[derive(Debug, PartialEq)]
    enum PlanStep {
        Order(Option<AIAction>),
        Ended(Option<AIAction>),
    }

    fn step(fx: &mut Fx, p: &mut SectorPlanner) -> PlanStep {
        match with(fx, |io| p.plan_step(io)) {
            (order, false) => PlanStep::Order(order),
            (order, true) => PlanStep::Ended(order),
        }
    }

    fn kind(fx: &Fx, at: usize) -> u32 {
        fx.galaxy.system(fx.systems[at]).unwrap().kind
    }

    fn edit_system(fx: &mut Fx, at: usize, f: impl FnOnce(&mut crate::ai_records::SystemRecord)) {
        f(fx.galaxy.system_mut(fx.systems[at]).unwrap());
    }

    #[test]
    fn the_plan_object_steps_from_its_base_through_its_fleet_s_roles_to_its_end() {
        // FUN_004adb70: 0 -> 1; 1 -> 2; 2 with a base -> 3; 3 -> 4; 4 -> 5;
        // 5 with an idle held fleet -> 6; 6 -> 7; 7 without a target -> 8;
        // 8 -> 10 and 10 -> 9 (slots 0x6158b0 return 0); 9 without a
        // target -> 0xc; 0xc -> 0xe; 0xe at its walk's end -> 0 with *out.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        let mut states = Vec::new();
        for _ in 0..12 {
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
            states.push(p.plan_state);
        }
        assert_eq!(states, [1, 2, 3, 4, 5, 6, 7, 8, 10, 9, 0xc, 0xe]);
        assert_eq!(p.plan_fleet, Some(f));
        assert_eq!(step(&mut fx, &mut p), PlanStep::Ended(None));
        assert_eq!(p.plan_state, 0);

        // 2 without a base -> 5: no system of the side's anywhere.
        let mut fx = fixture();
        let mut p = planner(&fx);
        for at in 0..4 {
            edit_system(&mut fx, at, |r| r.kind &= !1);
        }
        p.plan_state = 2;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!((p.plan_state, p.base), (5, None));
        // 5 without an idle held fleet -> 0xe, which ends the step.
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xe);
        assert_eq!(step(&mut fx, &mut p), PlanStep::Ended(None));
        // Any other state (0xb has no case) starts over at 1.
        p.plan_state = 0xb;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 1);
    }

    #[test]
    fn state_six_ends_the_step_when_the_flagship_s_fleet_stays() {
        // FUN_004adb70 case 6: *out from FUN_004b06b0 -> 0xe, else 7.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        p.plan_fleet = Some(f);
        p.flagship = Some(ship_id(&fx, f, 0));
        p.flags = FLAGSHIP_STAYS;
        p.plan_state = 6;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xe);
    }

    #[test]
    fn state_one_clears_only_the_role_marks_of_its_sector() {
        // FUN_004af6d0: the sector record's systems lose +0x30 & 0x1f00000.
        let mut fx = fixture();
        for at in [0, 2] {
            fx.kinds.insert(fx.systems[at], ROLE_MARKS | BASE);
        }
        rebuild(&mut fx);
        let mut p = planner(&fx);
        p.plan_state = 1;
        step(&mut fx, &mut p);
        assert_eq!(kind(&fx, 0) & (ROLE_MARKS | BASE), BASE);
        assert_eq!(fx.kinds.get(&fx.systems[0]), Some(&BASE));
        assert_eq!(kind(&fx, 2) & ROLE_MARKS, ROLE_MARKS);
    }

    /// Run state 2 (`FUN_004af8a0`) and give its base.
    fn base(fx: &mut Fx, p: &mut SectorPlanner) -> Option<SystemKey> {
        p.plan_state = 2;
        step(fx, p);
        p.base
    }

    #[test]
    fn the_base_is_the_sector_s_headquarters_else_its_own_system_by_key() {
        // FUN_004af8a0: (+0x28 0x80000000, forbidding 2; +0x30 1,
        // forbidding 0x40000; key 0x26, order 1), else the same without
        // the headquarters. The base takes 0x40000.
        let mut fx = fixture();
        let mut p = planner(&fx);
        edit_system(&mut fx, 1, |r| r.state |= 0x8000_0000);
        assert_eq!(base(&mut fx, &mut p), Some(fx.systems[1]));
        assert_eq!(p.plan_state, 3);
        assert_eq!(kind(&fx, 1) & BASES, BASE);
        assert_eq!(fx.kinds.get(&fx.systems[1]), Some(&BASE));

        // By key 0x26 (v[38]), order 1: the last of the list, the highest.
        let mut fx = fixture();
        let mut p = planner(&fx);
        edit_system(&mut fx, 0, |r| r.values[0x26] = 1);
        edit_system(&mut fx, 1, |r| r.values[0x26] = 5);
        assert_eq!(base(&mut fx, &mut p), Some(fx.systems[1]));

        // Another plan's base and a blockaded system are passed over.
        for skip in [
            |r: &mut crate::ai_records::SystemRecord| r.kind |= BASE,
            |r: &mut _| {
                let r: &mut crate::ai_records::SystemRecord = r;
                r.state |= 2;
            },
        ] {
            let mut fx = fixture();
            let mut p = planner(&fx);
            edit_system(&mut fx, 0, |r| r.values[0x26] = 1);
            edit_system(&mut fx, 1, |r| r.values[0x26] = 5);
            edit_system(&mut fx, 1, skip);
            assert_eq!(base(&mut fx, &mut p), Some(fx.systems[0]));
        }
    }

    #[test]
    fn equal_keys_go_by_the_coin_for_the_base_and_the_unloading_fleet() {
        // FUN_005f59f0: an equal key goes before the other on
        // FUN_0041cd80(10) < 5; FUN_004357b0 takes the list's last.
        for (draw, first) in [(0, true), (4, true), (5, false), (7, false)] {
            let mut fx = fixture();
            let mut p = planner(&fx);
            for at in 0..2 {
                edit_system(&mut fx, at, |r| r.values[0x26] = 1);
            }
            fx.draws = vec![draw];
            let want = if first { fx.systems[0] } else { fx.systems[1] };
            assert_eq!(base(&mut fx, &mut p), Some(want), "draw {draw}");

            // FUN_0041acd0's list of held fleets carrying regiments.
            let mut fx = fixture();
            let mut p = planner(&fx);
            let a = held(&mut fx, &mut p, 0, 1);
            let b = held(&mut fx, &mut p, 1, 1);
            let ta = embark(&mut fx, a);
            let tb = embark(&mut fx, b);
            fx.draws = vec![draw];
            p.plan_state = 3;
            let (troop, to) = if first { (ta, 0) } else { (tb, 1) };
            assert_eq!(
                step(&mut fx, &mut p),
                PlanStep::Order(Some(AIAction::MoveRegiments {
                    troops: vec![troop],
                    to: fx.systems[to],
                })),
                "draw {draw}"
            );
        }
    }

    #[test]
    fn a_kept_mark_stays_when_set_again() {
        // FUN_00433e30's record keeps +0x30 bits set twice.
        let mut fx = fixture();
        let s0 = fx.systems[0];
        with(&mut fx, |io| {
            io.set_kind(s0, BASE, 0);
            io.set_kind(s0, BASE, 0x3_0000);
        });
        assert_eq!(kind(&fx, 0) & BASE, BASE);
        assert_eq!(fx.kinds.get(&s0), Some(&BASE));
    }

    #[test]
    fn without_an_own_system_the_nearest_other_sector_gives_the_base() {
        // FUN_004af8a0 -> FUN_0041b2b0 (+0x28 forbidding 2, +0x30 1
        // forbidding 0x40000).
        let mut fx = fixture();
        let mut p = planner(&fx);
        for at in 0..2 {
            edit_system(&mut fx, at, |r| r.kind &= !1);
        }
        let b = base(&mut fx, &mut p).unwrap();
        assert!(fx.systems[2..].contains(&b));
        assert_eq!(fx.galaxy.system(b).unwrap().kind & BASE, BASE);
        // Without a sector, no base.
        let mut p = SectorPlanner {
            sector: None,
            ..planner(&fx)
        };
        assert_eq!(base(&mut fx, &mut p), None);
        assert_eq!(p.plan_state, 5);
    }

    #[test]
    fn a_kept_base_drops_other_kinds_marks_and_a_lost_one_its_own() {
        // FUN_004af8a0: a base still the side's, not blockaded, the
        // headquarters when the sector has one, loses 0x30000; any other
        // loses 0x70000 and is chosen anew.
        let mut fx = fixture();
        let mut p = planner(&fx);
        p.base = Some(fx.systems[0]);
        edit_system(&mut fx, 0, |r| r.kind |= 0x3_0000);
        assert_eq!(base(&mut fx, &mut p), Some(fx.systems[0]));
        assert_eq!(p.plan_state, 3);
        assert_eq!(kind(&fx, 0) & BASES, 0);

        let lose: [fn(&mut Fx); 3] = [
            |fx| edit_system(fx, 0, |r| r.state |= 2),
            |fx| edit_system(fx, 0, |r| r.kind &= !1),
            |fx| {
                fx.galaxy.sectors[0].candidates |= 0x4000_0000;
                edit_system(fx, 1, |r| r.state |= 0x8000_0000);
            },
        ];
        for lost in lose {
            let mut fx = fixture();
            let mut p = planner(&fx);
            p.base = Some(fx.systems[0]);
            edit_system(&mut fx, 0, |r| r.kind |= BASES);
            fx.kinds.insert(fx.systems[0], BASES);
            lost(&mut fx);
            assert_eq!(base(&mut fx, &mut p), Some(fx.systems[1]));
            assert_eq!(kind(&fx, 0) & BASES, 0);
            assert!(!fx.kinds.contains_key(&fx.systems[0]));
        }
        // The headquarters stays the base of a sector that has one.
        let mut fx = fixture();
        let mut p = planner(&fx);
        fx.galaxy.sectors[0].candidates |= 0x4000_0000;
        edit_system(&mut fx, 0, |r| r.state |= 0x8000_0000);
        p.base = Some(fx.systems[0]);
        assert_eq!(base(&mut fx, &mut p), Some(fx.systems[0]));
        assert_eq!(p.plan_state, 3);
    }

    #[test]
    fn a_base_without_a_record_is_kept_and_gives_no_step() {
        // FUN_004af8a0: a base whose system has no record is neither kept
        // nor replaced; the step goes to 5.
        let mut fx = fixture();
        let mut p = planner(&fx);
        p.base = Some(SystemKey::default());
        assert_eq!(base(&mut fx, &mut p), Some(SystemKey::default()));
        assert_eq!(p.plan_state, 5);
    }

    #[test]
    fn a_reset_planner_gives_up_its_base_and_the_base_s_marks() {
        // FUN_00477190: a base whose system has a record loses +0x30
        // 0x70000 and the planner its base (+0x40); one without a record
        // stays named.
        let mut fx = fixture();
        let mut p = planner(&fx);
        p.base = Some(fx.systems[0]);
        edit_system(&mut fx, 0, |r| r.kind |= BASES | ROLE_MARKS);
        fx.kinds.insert(fx.systems[0], BASES);
        let reset = |fx: &mut Fx, p: &mut SectorPlanner| {
            let mut random = |_| 0;
            let mut io = crate::ai_module8::Inputs8 {
                world: &fx.world,
                galaxy: &mut fx.galaxy,
                ship_holds: &mut fx.ship_holds,
                fleet_holds: &mut fx.fleet_holds,
                kinds: &mut fx.kinds,
                transport: &fx.transport,
                people: &fx.people,
                controller_flags: 0,
                day: fx.day,
                seen: &fx.seen,
                swept: &mut fx.swept,
                balance: 0,
                random: &mut random,
            };
            crate::ai_module8::reset(p, &mut io);
        };
        reset(&mut fx, &mut p);
        assert_eq!(p.base, None);
        assert_eq!(kind(&fx, 0) & (BASES | ROLE_MARKS), ROLE_MARKS);
        assert!(!fx.kinds.contains_key(&fx.systems[0]));
        p.base = Some(SystemKey::default());
        reset(&mut fx, &mut p);
        assert_eq!(p.base, Some(SystemKey::default()));
    }

    /// A regiment of the Alliance aboard `fleet`, whose ships carry two.
    fn embark(fx: &mut Fx, f: FleetKey) -> TroopKey {
        let class = fx.world.fleets[f].capital_ships[0].class;
        fx.world.capital_ship_classes[class].troop_capacity = 2;
        let at = fx.world.fleets[f].location;
        let troop = fx.world.troops.insert(TroopUnit {
            class_dat_id: crate::ids::DatId::new(0x1000_0001),
            is_alliance: true,
            regiment_strength: 100,
        });
        fx.world.systems[at].ground_units.push(troop);
        fx.transport.embark(&mut fx.world, f, &[troop]).unwrap();
        rebuild(fx);
        troop
    }

    #[test]
    fn a_held_fleet_at_an_own_system_unloads_its_held_ships_regiments() {
        // FUN_004afce0: held fleets with 0x40001 and no 0xf0000002, at a
        // system with +0x30 1, Move (0x201) the regiments of their held
        // ships (0x4000, forbidding 0x20800) to that system.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 2);
        let troop = embark(&mut fx, f);
        p.plan_state = 3;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(Some(AIAction::MoveRegiments {
                troops: vec![troop],
                to: fx.systems[0],
            }))
        );
        assert_eq!(p.plan_state, 4);

        let stay: [fn(&mut Fx, FleetKey); 4] = [
            // Not at a system of the side's.
            |fx, _| edit_system(fx, 0, |r| r.kind &= !1),
            // A role.
            |fx, f| fx.galaxy.fleets.get_mut(&f).unwrap().flags |= 0x1000_0000,
            // Not held by this planner.
            |fx, f| fx.galaxy.fleets.get_mut(&f).unwrap().owner = 9,
            // The ship carrying them is not held.
            |fx, f| fx.galaxy.fleets.get_mut(&f).unwrap().ships[0].flags |= SHIP_FREE,
        ];
        for edit in stay {
            let mut fx = fixture();
            let mut p = planner(&fx);
            let f = held(&mut fx, &mut p, 0, 2);
            embark(&mut fx, f);
            edit(&mut fx, f);
            p.plan_state = 3;
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        }
    }

    use crate::ai_records::SHIP_FREE;

    /// Fighter classes of the Empire and the Alliance (two), and `count`
    /// squadrons of each on system 0's planet, the Empire's first.
    fn squadrons(fx: &mut Fx, count: [u32; 3]) -> [crate::ids::FighterKey; 3] {
        let mut wing = |alliance| {
            fx.world.fighter_classes.insert(FighterClass {
                is_alliance: alliance,
                is_empire: !alliance,
                squadron_size: 12,
                ..FighterClass::default()
            })
        };
        let classes = [wing(false), wing(true), wing(true)];
        let at = fx.systems[0];
        fx.world.systems[at].fighters = classes
            .iter()
            .zip(count)
            .map(|(&class, count)| FighterEntry { class, count })
            .collect();
        classes
    }

    /// A held fleet at system 0 whose `n` ships hold two squadrons each.
    fn carrier(fx: &mut Fx, p: &mut SectorPlanner, n: u32) -> FleetKey {
        let f = held(fx, p, 0, n);
        let class = fx.world.fleets[f].capital_ships[0].class;
        fx.world.capital_ship_classes[class].fighter_capacity = 2;
        rebuild(fx);
        f
    }

    #[test]
    fn the_base_s_squadrons_board_a_held_fleet_there_up_to_its_room() {
        // FUN_004b0120: base v[25] > 0; the held fleet (1, forbidding 2;
        // key 0x17, order 1) at the base with room (fleet +0xac) takes
        // min(room, v[25]) squadrons of the planet (Move 0x201).
        let mut fx = fixture();
        let [_, a, b] = squadrons(&mut fx, [1, 2, 3]);
        let mut p = planner(&fx);
        let f = carrier(&mut fx, &mut p, 2);
        p.base = Some(fx.systems[0]);
        p.plan_state = 4;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(Some(AIAction::MoveSquadronTeam {
                squadrons: vec![(a, 2), (b, 2)],
                from: SquadronPlace::Planet(fx.systems[0]),
                to: SquadronPlace::Fleet(f),
            }))
        );
        assert_eq!(p.plan_state, 5);
        // Room beyond the side's squadrons takes those there are. (The cap
        // v[25] counts every squadron of the planet, so in the port it
        // never falls below the side's.)
        let mut fx = fixture();
        let [_, a, b] = squadrons(&mut fx, [1, 1, 1]);
        let mut p = planner(&fx);
        let f = carrier(&mut fx, &mut p, 2);
        p.base = Some(fx.systems[0]);
        p.plan_state = 4;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(Some(AIAction::MoveSquadronTeam {
                squadrons: vec![(a, 1), (b, 1)],
                from: SquadronPlace::Planet(fx.systems[0]),
                to: SquadronPlace::Fleet(f),
            }))
        );
    }

    #[test]
    fn the_squadrons_taken_stop_at_the_room_and_add_up_by_class() {
        // FUN_004b0120's loop: one squadron at a time while the count
        // lasts, in the planet's order.
        let mut fx = fixture();
        let [_, a, b] = squadrons(&mut fx, [0, 2, 4]);
        let mut p = planner(&fx);
        let class = {
            let f = carrier(&mut fx, &mut p, 1);
            fx.world.fleets[f].capital_ships[0].class
        };
        fx.world.capital_ship_classes[class].fighter_capacity = 5;
        let at = fx.systems[0];
        fx.world.systems[at]
            .fighters
            .push(FighterEntry { class: a, count: 3 });
        rebuild(&mut fx);
        p.base = Some(at);
        p.plan_state = 4;
        let PlanStep::Order(Some(AIAction::MoveSquadronTeam { squadrons, .. })) =
            step(&mut fx, &mut p)
        else {
            panic!("no squadron move");
        };
        assert_eq!(squadrons, [(a, 2), (b, 3)]);
        // Room for more than the first entries: a class met again adds up.
        fx.world.capital_ship_classes[class].fighter_capacity = 9;
        rebuild(&mut fx);
        p.plan_state = 4;
        let PlanStep::Order(Some(AIAction::MoveSquadronTeam { squadrons, .. })) =
            step(&mut fx, &mut p)
        else {
            panic!("no squadron move");
        };
        assert_eq!(squadrons, [(a, 5), (b, 4)]);
    }

    #[test]
    fn no_squadron_boards_without_room_or_a_fleet_at_the_base() {
        // FUN_004b0120: the first held fleet at the base ends the search;
        // without room it takes nothing. No base squadrons, no search.
        let none: [fn(&mut Fx, &mut SectorPlanner); 4] = [
            |fx, p| {
                carrier(fx, p, 1);
                let at = fx.systems[0];
                fx.world.systems[at].fighters.clear();
                rebuild(fx);
            },
            |fx, p| {
                held(fx, p, 0, 1);
            },
            |fx, p| {
                carrier(fx, p, 1);
                let (here, there) = (fx.systems[0], fx.systems[1]);
                fx.world.systems[there].fighters = fx.world.systems[here].fighters.clone();
                rebuild(fx);
                p.base = Some(there);
            },
            |fx, p| {
                carrier(fx, p, 1);
                p.base = None;
            },
        ];
        for setup in none {
            let mut fx = fixture();
            squadrons(&mut fx, [0, 2, 0]);
            let mut p = planner(&fx);
            p.base = Some(fx.systems[0]);
            setup(&mut fx, &mut p);
            p.plan_state = 4;
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
            assert_eq!(p.plan_state, 5);
        }
    }

    #[test]
    fn the_idle_fleet_search_goes_round_the_held_fleets_once() {
        // FUN_004b04c0: from +0x2c (else the first), the first fleet
        // without 0xf0003802 becomes +0x28; +0x2c the one after it, cleared
        // once the search comes round.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let a = held(&mut fx, &mut p, 0, 1);
        let b = held(&mut fx, &mut p, 0, 1);
        let c = held(&mut fx, &mut p, 0, 1);
        fx.galaxy.fleets.get_mut(&b).unwrap().flags |= 0x1000;
        let search = |fx: &mut Fx, p: &mut SectorPlanner| {
            p.plan_state = 5;
            step(fx, p);
            (p.plan_state, p.plan_fleet, p.plan_cursor)
        };
        assert_eq!(search(&mut fx, &mut p), (6, Some(a), Some(b)));
        assert_eq!(search(&mut fx, &mut p), (6, Some(c), Some(a)));
        assert_eq!(search(&mut fx, &mut p), (6, Some(a), Some(b)));
        // Each busy bit keeps a fleet out; none idle: 0xe, +0x2c cleared.
        for bit in [2, 0x800, 0x1000, 0x2000, 0x1000_0000, 0x8000_0000] {
            for f in [a, c] {
                fx.galaxy.fleets.get_mut(&f).unwrap().flags |= bit;
            }
            assert_eq!(search(&mut fx, &mut p), (0xe, Some(a), None));
            for f in [a, c] {
                fx.galaxy.fleets.get_mut(&f).unwrap().flags &= !bit;
            }
        }
        // A lone fleet: found, and the search came round.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let a = held(&mut fx, &mut p, 0, 1);
        assert_eq!(search(&mut fx, &mut p), (6, Some(a), None));
    }

    /// A character aboard `f` whose record has the flags `flags`.
    fn aboard(fx: &mut Fx, f: FleetKey, flags: u32) -> CharacterKey {
        let c = fx.world.characters.insert(Character::default());
        fx.world.fleets[f].characters.push(c);
        fx.people.insert(c, flags);
        rebuild(fx);
        c
    }

    /// Person record flags: the side's (`2`), the other side's prisoner
    /// (`4 | 0x40`).
    const OWN: u32 = 2;
    const PRISONER: u32 = 4 | 0x40;

    #[test]
    fn a_fleet_at_an_own_system_sets_down_its_people_there() {
        // FUN_004b06b0: fleet 0x4000000 without 2, at +0x30 1: the people
        // aboard its ships (0x600000, not 0x800) Move (0x201) there when
        // one of them is the side's (+0x30 2).
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        let own = aboard(&mut fx, f, OWN);
        let prisoner = aboard(&mut fx, f, PRISONER);
        p.plan_fleet = Some(f);
        p.plan_state = 6;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(Some(AIAction::MoveCharacters {
                characters: vec![own, prisoner],
                to: fx.systems[0],
            }))
        );
        assert_eq!(p.plan_state, 7);

        let stay: [fn(&mut Fx, FleetKey); 5] = [
            |fx, _| edit_system(fx, 0, |r| r.kind &= !1),
            |fx, f| fx.galaxy.fleets.get_mut(&f).unwrap().flags |= 2,
            |fx, f| fx.galaxy.fleets.get_mut(&f).unwrap().flags &= !0x400_0000,
            |fx, f| fx.galaxy.fleets.get_mut(&f).unwrap().ships[0].flags |= 0x800,
            |fx, f| fx.galaxy.fleets.get_mut(&f).unwrap().ships[0].flags &= !0x60_0000,
        ];
        for edit in stay {
            let mut fx = fixture();
            let mut p = planner(&fx);
            let f = held(&mut fx, &mut p, 0, 1);
            aboard(&mut fx, f, OWN);
            edit(&mut fx, f);
            p.plan_fleet = Some(f);
            p.plan_state = 6;
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        }
        // Only a prisoner aboard: the fleet lacks 0x4000000; and with the
        // bit but no person of the side's, nobody moves.
        for force in [false, true] {
            let mut fx = fixture();
            let mut p = planner(&fx);
            let f = held(&mut fx, &mut p, 0, 1);
            aboard(&mut fx, f, PRISONER);
            if force {
                fx.galaxy.fleets.get_mut(&f).unwrap().flags |= 0x400_0000;
                fx.galaxy.fleets.get_mut(&f).unwrap().ships[0].flags |= 0x40_0000;
            }
            p.plan_fleet = Some(f);
            p.plan_state = 6;
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        }
    }

    #[test]
    fn the_flagship_s_fleet_goes_to_the_movement_system() {
        // FUN_004b06b0: the fleet holding the flagship (+0x38) Moves to
        // +0x3c while +0x60 has 0x4000000 and +0x3c is not blockaded,
        // which ends the step.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 2);
        aboard(&mut fx, f, OWN);
        p.plan_fleet = Some(f);
        p.flagship = Some(ship_id(&fx, f, 1));
        p.flags = FLAGSHIP_MOVES;
        p.station = Some(fx.systems[2]);
        p.plan_state = 6;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(Some(AIAction::MoveShips {
                fleet: f,
                ships: vec![0, 1],
                to: fx.systems[2],
            }))
        );
        assert_eq!(p.plan_state, 0xe);
        // Blockaded, no station, another fleet's flagship or no 0x4000000:
        // the people go down instead and the step goes on.
        let other: [fn(&mut Fx, &mut SectorPlanner); 4] = [
            |fx, _| edit_system(fx, 2, |r| r.state |= 2),
            |_, p| p.station = None,
            |_, p| p.flagship = Some(u64::MAX),
            |_, p| p.flags = 0,
        ];
        for edit in other {
            let mut fx = fixture();
            let mut p = planner(&fx);
            let f = held(&mut fx, &mut p, 0, 2);
            aboard(&mut fx, f, OWN);
            p.plan_fleet = Some(f);
            p.flagship = Some(ship_id(&fx, f, 1));
            p.flags = FLAGSHIP_MOVES;
            p.station = Some(fx.systems[2]);
            edit(&mut fx, &mut p);
            p.plan_state = 6;
            assert!(matches!(
                step(&mut fx, &mut p),
                PlanStep::Order(Some(AIAction::MoveCharacters { .. }))
            ));
            assert_eq!(p.plan_state, 7);
        }
    }

    fn record(fx: &mut Fx, f: FleetKey) -> &mut FleetRecord {
        fx.galaxy.fleets.get_mut(&f).unwrap()
    }

    /// An enemy fleet at system `at` of attack `a`, defense `d` and third
    /// value `t` (`v[39]`, `v[40]`, `v[43]`).
    fn enemy_fleet(fx: &mut Fx, at: usize, (a, d, t): (i32, i32, i32)) {
        edit_system(fx, at, |r| {
            r.kind |= 0x200;
            (r.values[39], r.values[40], r.values[43]) = (a, d, t);
        });
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "One table of FUN_004ae150's branches."
    )]
    fn a_fleet_prevails_over_more_than_a_fifth_less_attack_and_the_defenses() {
        // FUN_004ae150: (v[39], v[40], v[43]) with +0x30 0x200, plus
        // (v[26], v[27], v[23]) with 4; attack above 0 and 12/10 of it
        // below values[2]; with a third value, values[3] above the second
        // and values[9] above it (fleet 0x10 or system 0x400), else at
        // least it unless values[19] is above 0.
        let fleet = |flags, v2, v3, v9, v19| {
            let mut r = FleetRecord {
                flags,
                ..FleetRecord::default()
            };
            (r.values[2], r.values[3], r.values[9], r.values[19]) = (v2, v3, v9, v19);
            r
        };
        let system = |kind, fleets: (i32, i32, i32), forces: (i32, i32, i32)| {
            let mut r = SystemRecord {
                system: SystemKey::default(),
                state: 0,
                summary: 0,
                kind,
                values: [0; 60],
                position: (0, 0),
            };
            (r.values[39], r.values[40], r.values[43]) = fleets;
            (r.values[26], r.values[27], r.values[23]) = forces;
            r
        };
        let cases = [
            // 12/10 of 10 is 12: 13 prevails, 12 does not.
            (
                fleet(0, 13, 0, 0, 0),
                system(0x200, (10, 0, 0), (0, 0, 0)),
                true,
            ),
            (
                fleet(0, 12, 0, 0, 0),
                system(0x200, (10, 0, 0), (0, 0, 0)),
                false,
            ),
            // 12/10 of 9 truncates to 10.
            (
                fleet(0, 11, 0, 0, 0),
                system(0x200, (9, 0, 0), (0, 0, 0)),
                true,
            ),
            // No attack there.
            (
                fleet(0, 50, 0, 0, 0),
                system(0x200, (0, 0, 0), (0, 0, 0)),
                false,
            ),
            // The forces count with 4 only; the fleets with 0x200 only.
            (
                fleet(0, 13, 0, 0, 0),
                system(4, (50, 0, 0), (10, 0, 0)),
                true,
            ),
            (
                fleet(0, 13, 0, 0, 0),
                system(0x204, (5, 0, 0), (5, 0, 0)),
                true,
            ),
            (
                fleet(0, 13, 0, 0, 0),
                system(0x204, (6, 0, 0), (5, 0, 0)),
                false,
            ),
            (
                fleet(0, 13, 0, 0, 0),
                system(0, (10, 0, 0), (10, 0, 0)),
                false,
            ),
            // A third value: defense above the second (summed), ...
            (
                fleet(0, 13, 7, 3, 0),
                system(0x204, (5, 3, 1), (5, 4, 2)),
                false,
            ),
            (
                fleet(0, 13, 7, 3, 0),
                system(0x204, (5, 3, 1), (5, 3, 0)),
                true,
            ),
            // ... values[9] at least it, or values[19] above 0 ...
            (
                fleet(0, 13, 7, 2, 0),
                system(0x200, (10, 3, 3), (0, 0, 0)),
                false,
            ),
            (
                fleet(0, 13, 7, 2, 1),
                system(0x200, (10, 3, 3), (0, 0, 0)),
                true,
            ),
            (
                fleet(0, 13, 7, 3, 0),
                system(0x200, (10, 3, 3), (0, 0, 0)),
                true,
            ),
            // ... or above it with fleet 0x10 or system 0x400.
            (
                fleet(0x10, 13, 7, 3, 1),
                system(0x200, (10, 3, 3), (0, 0, 0)),
                false,
            ),
            (
                fleet(0x10, 13, 7, 4, 0),
                system(0x200, (10, 3, 3), (0, 0, 0)),
                true,
            ),
            (
                fleet(0, 13, 7, 3, 1),
                system(0x600, (10, 3, 3), (0, 0, 0)),
                false,
            ),
            (
                fleet(0, 13, 7, 4, 0),
                system(0x600, (10, 3, 3), (0, 0, 0)),
                true,
            ),
            // An attack of 1 counts (12/10 of it is 1).
            (
                fleet(0, 2, 0, 0, 0),
                system(0x200, (1, 0, 0), (0, 0, 0)),
                true,
            ),
            // A third value of 1 already asks for the defense.
            (
                fleet(0, 13, 3, 9, 0),
                system(0x200, (10, 3, 1), (0, 0, 0)),
                false,
            ),
            // The third values add too: 2 is below 1 + 2.
            (
                fleet(0, 13, 7, 2, 0),
                system(0x204, (5, 3, 1), (5, 0, 2)),
                false,
            ),
            // The two defenses add: 6 is above 3 + 2.
            (
                fleet(0, 13, 6, 9, 0),
                system(0x204, (5, 3, 1), (5, 2, 0)),
                true,
            ),
        ];
        for (i, (f, s, expected)) in cases.iter().enumerate() {
            assert_eq!(fleet_prevails(f, s), *expected, "case {i}");
        }
    }

    #[test]
    fn state_seven_binds_the_fleet_to_the_first_enemy_fleet_it_prevails_over() {
        // FUN_004b4560: values[2] above 0; the sector's systems with +0x30
        // 0x200 and without 0x100000, key 0x27 order 1 (the highest first),
        // the first FUN_004ae150 passes takes 0x100000, the fleet +0x2c
        // and 0x10000000; then 0xd.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        record(&mut fx, f).values[2] = 13;
        enemy_fleet(&mut fx, 0, (10, 0, 0));
        enemy_fleet(&mut fx, 1, (20, 0, 0));
        // System 1 comes first (key 0x27 is v[39], the enemy's attack)
        // but is too strong.
        p.plan_fleet = Some(f);
        p.plan_state = 7;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xd);
        assert_eq!(kind(&fx, 0) & 0x10_0000, 0x10_0000);
        assert_eq!(kind(&fx, 1) & 0x10_0000, 0);
        let r = &fx.galaxy.fleets[&f];
        assert_eq!(
            (r.target, r.flags & ROLES),
            (Some(fx.systems[0]), ROLE_ATTACK)
        );
        assert_eq!(
            (fx.fleet_holds[&f].target, fx.fleet_holds[&f].flags & ROLES),
            (Some(fx.systems[0]), ROLE_ATTACK)
        );
        assert_eq!(fx.kinds[&fx.systems[0]] & 0x10_0000, 0x10_0000);

        // A system an attacker is bound to (0x100000) is passed over, and
        // with none left the step goes to 8.
        let mut p2 = planner(&fx);
        let g = held(&mut fx, &mut p2, 0, 1);
        record(&mut fx, g).values[2] = 13;
        // The refresh kept the mark (+0x30 & 0x35ff0000); the enemy is
        // still there.
        enemy_fleet(&mut fx, 0, (10, 0, 0));
        enemy_fleet(&mut fx, 1, (20, 0, 0));
        assert_eq!(kind(&fx, 0) & 0x10_0000, 0x10_0000);
        p2.plan_fleet = Some(g);
        p2.plan_state = 7;
        assert_eq!(step(&mut fx, &mut p2), PlanStep::Order(None));
        assert_eq!(p2.plan_state, 8);
        assert_eq!(fx.galaxy.fleets[&g].target, None);

        // No attack: 8 at once, without the query (whose tie on key 0x27
        // would draw).
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        enemy_fleet(&mut fx, 0, (1, 0, 0));
        enemy_fleet(&mut fx, 1, (1, 0, 0));
        fx.draws = vec![7, 7];
        p.plan_fleet = Some(f);
        p.plan_state = 7;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 8);
        assert_eq!(fx.draws.len(), 2);
        record(&mut fx, f).values[2] = 1;
        p.plan_state = 7;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!((p.plan_state, fx.draws.len()), (8, 1));
    }

    /// Make system `at` the opponent's with what `FUN_004ae320` reads, and
    /// its sector's `values[2]` and `values[4]`.
    fn bombardable(fx: &mut Fx, at: usize, (v7, v18, v33): (i32, i32, i32), sector: (i32, i32)) {
        edit_system(fx, at, |r| {
            r.kind = (r.kind & !1) | 4;
            (r.values[7], r.values[18], r.values[33]) = (v7, v18, v33);
        });
        let s = fx
            .galaxy
            .sectors
            .iter_mut()
            .find(|s| s.sector == fx.sectors[0])
            .unwrap();
        (s.values[2], s.values[4]) = sector;
    }

    #[test]
    fn a_bombardment_targets_the_facilities_when_the_sector_is_held_else_is_general() {
        // FUN_004ae320: +0x30 4; values[3] at least max(v[27], controller
        // +0x168) when elsewhere, 0 at it; values[6] above v[8]; v[11] 0.
        // x = (sector values[2] - values[4] + 1) / 2: above 0, 0x220 with
        // v[7] + v[18] above x and v[18] above 0; else 0x222 with v[18] or
        // v[33] above 0.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        record(&mut fx, f).values[6] = 5;
        let code = |fx: &mut Fx| with(fx, |io| bombardment(io, f, fx_system(io, 1)));
        // x = (9 - 4 + 1) / 2 = 3: v[7] + v[18] = 4 > 3 with v[18] 1.
        bombardable(&mut fx, 1, (3, 1, 0), (9, 4));
        assert_eq!(code(&mut fx), Some(0x220));
        // v[7] + v[18] = 3, not above 3; v[18] 0 neither.
        bombardable(&mut fx, 1, (2, 1, 0), (9, 4));
        assert_eq!(code(&mut fx), None);
        bombardable(&mut fx, 1, (9, 0, 5), (9, 4));
        assert_eq!(code(&mut fx), None);
        // x = (4 - 4 + 1) / 2 = 0: v[18] or v[33] above 0.
        bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
        assert_eq!(code(&mut fx), Some(0x222));
        bombardable(&mut fx, 1, (0, 0, 1), (4, 4));
        assert_eq!(code(&mut fx), Some(0x222));
        bombardable(&mut fx, 1, (9, 0, 0), (4, 4));
        assert_eq!(code(&mut fx), None);
        // x = (5 - 4 + 1) / 2 = 1 > 0.
        bombardable(&mut fx, 1, (0, 2, 0), (5, 4));
        assert_eq!(code(&mut fx), Some(0x220));
        // values[6] must be above v[8]; v[11] must be 0.
        bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
        edit_system(&mut fx, 1, |r| r.values[8] = 5);
        assert_eq!(code(&mut fx), None);
        edit_system(&mut fx, 1, |r| (r.values[8], r.values[11]) = (4, 1));
        assert_eq!(code(&mut fx), None);
        // Shields elsewhere: values[3] at least v[27].
        edit_system(&mut fx, 1, |r| (r.values[11], r.values[27]) = (0, 3));
        record(&mut fx, f).values[3] = 2;
        assert_eq!(code(&mut fx), None);
        record(&mut fx, f).values[3] = 3;
        assert_eq!(code(&mut fx), Some(0x222));
        // Not the opponent's (+0x30 without 4).
        edit_system(&mut fx, 1, |r| r.kind &= !4);
        assert_eq!(code(&mut fx), None);
        // The system's own sector's values count: sector 1 (x 0, General
        // Bombardment) while sector 0's would give Target Military
        // Facilities.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        record(&mut fx, f).values[6] = 5;
        bombardable(&mut fx, 0, (0, 1, 0), (9, 4));
        edit_system(&mut fx, 2, |r| {
            r.kind = 4;
            r.values[18] = 1;
        });
        let s1 = fx
            .galaxy
            .sectors
            .iter_mut()
            .find(|s| s.sector == fx.sectors[1])
            .unwrap();
        (s1.values[2], s1.values[4]) = (4, 4);
        let system = fx.systems[2];
        assert_eq!(with(&mut fx, |io| bombardment(io, f, system)), Some(0x222));
        // At the system its shields do not count.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 1, 1);
        record(&mut fx, f).values[6] = 5;
        bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
        edit_system(&mut fx, 1, |r| r.values[27] = 3);
        assert_eq!(
            with(&mut fx, |io| bombardment(io, f, fx_system(io, 1))),
            Some(0x222)
        );
    }

    fn fx_system(io: &PlannerIo<'_>, at: usize) -> SystemKey {
        io.galaxy.sectors[0].systems[at].system
    }

    #[test]
    fn state_nine_binds_a_bombarder_to_a_system_it_can_bombard() {
        // FUN_004b4730: values[6] above 0; the sector's systems with +0x30
        // 4 and without 0x700200, merged by key 8 (order 2) and key 0x10
        // (order 1); the first FUN_004ae320 passes takes 0x400000, the
        // fleet +0x2c and 0x40000000; then 0xd, else 0xc.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        // A bombardment of 1 is enough (above 0 and v[8]).
        record(&mut fx, f).values[6] = 1;
        bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
        p.plan_fleet = Some(f);
        p.plan_state = 9;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xd);
        assert_eq!(kind(&fx, 1) & 0x40_0000, 0x40_0000);
        let r = &fx.galaxy.fleets[&f];
        assert_eq!(
            (r.target, r.flags & ROLES),
            (Some(fx.systems[1]), ROLE_BOMBARD)
        );
        // A role bound there (0x700200 holds 0x400000) passes it over.
        let mut p2 = planner(&fx);
        let g = held(&mut fx, &mut p2, 0, 1);
        record(&mut fx, g).values[6] = 5;
        bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
        assert_eq!(kind(&fx, 1) & 0x40_0000, 0x40_0000);
        p2.plan_fleet = Some(g);
        p2.plan_state = 9;
        assert_eq!(step(&mut fx, &mut p2), PlanStep::Order(None));
        assert_eq!(p2.plan_state, 0xc);
        // So does an enemy fleet there (0x200), and no bombardment at all.
        for edit in [
            |fx: &mut Fx, _: FleetKey| edit_system(fx, 1, |r| r.kind |= 0x200),
            // No bombardment, even where FUN_004ae320 would pass (v[8]
            // below it).
            |fx: &mut Fx, f: FleetKey| {
                record(fx, f).values[6] = 0;
                edit_system(fx, 1, |r| r.values[8] = -1);
            },
            // Listed, but FUN_004ae320 fails (v[11]).
            |fx: &mut Fx, _: FleetKey| edit_system(fx, 1, |r| r.values[11] = 1),
        ] {
            let mut fx = fixture();
            let mut p = planner(&fx);
            let f = held(&mut fx, &mut p, 0, 1);
            record(&mut fx, f).values[6] = 5;
            bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
            edit(&mut fx, f);
            p.plan_fleet = Some(f);
            p.plan_state = 9;
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
            assert_eq!(p.plan_state, 0xc);
            assert_eq!(fx.galaxy.fleets[&f].target, None);
        }
    }

    #[test]
    fn state_nine_tries_the_merged_lists_in_their_best_order() {
        // FUN_004b4730 merges key 8 (order 2) and key 0x10 (order 1); the
        // pick (FUN_004357b0) takes the highest sum of places, the earliest
        // of the first list on a tie. By key 8 descending [1, 0], by key
        // 0x10 ascending [0, 1]: both sum to 1, so system 1; the first list
        // alone would give system 0.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 2, 1);
        record(&mut fx, f).values[6] = 5;
        bombardable(&mut fx, 0, (0, 1, 0), (4, 4));
        bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
        edit_system(&mut fx, 0, |r| (r.values[8], r.values[0x10]) = (1, 1));
        edit_system(&mut fx, 1, |r| (r.values[8], r.values[0x10]) = (2, 9));
        p.plan_fleet = Some(f);
        p.plan_state = 9;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(fx.galaxy.fleets[&f].target, Some(fx.systems[1]));
    }

    #[test]
    fn state_c_joins_the_nearest_held_fleet_without_an_enemy_there() {
        // FUN_004b0c90: more than one held fleet; FUN_0041ae20 (the
        // planner's, 1 forbidding 0xf0000002, by distance, order 2): the
        // first other one at a system without +0x30 0x200 is the target.
        // Move 0x201 onto it; then 0xe.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        let near = held(&mut fx, &mut p, 1, 1);
        let far = held(&mut fx, &mut p, 3, 1);
        p.plan_fleet = Some(f);
        p.plan_state = 0xc;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(Some(AIAction::JoinFleet {
                fleet: f,
                into: near
            }))
        );
        assert_eq!(p.plan_state, 0xe);
        // An enemy fleet at the nearest: the next.
        enemy_fleet(&mut fx, 1, (1, 0, 0));
        p.plan_state = 0xc;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(Some(AIAction::JoinFleet {
                fleet: f,
                into: far
            }))
        );
        // A fleet with a role or en route is not listed.
        record(&mut fx, far).flags |= ROLE_ATTACK;
        record(&mut fx, near).flags &= !0x200;
        edit_system(&mut fx, 1, |r| r.kind &= !0x200);
        record(&mut fx, near).flags |= 2;
        p.plan_state = 0xc;
        // Neither: slot +0x54, the base at the fleet's own system, so no
        // move (port: the original's move there does nothing).
        p.base = Some(fx.systems[0]);
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xe);
        // With one held fleet slot +0x54 at once: the base elsewhere.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        p.plan_fleet = Some(f);
        p.base = Some(fx.systems[1]);
        p.plan_state = 0xc;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(Some(AIAction::MoveShips {
                fleet: f,
                ships: vec![0],
                to: fx.systems[1],
            }))
        );
    }

    #[test]
    fn slot_54_prefers_the_base_then_headquarters_then_a_yard_then_the_nearest_safe_system() {
        // FUN_004b4a20: the base not blockaded; else +0x28 0x80000000
        // forbidding 2 with +0x30 1 (key 7, order 2); else +0x28
        // 0x3e00000 the same (key 0x21, order 1); else, at a system of the
        // opponent, the sector's system nearest the fleet without +0x30
        // 0x204 (FUN_0041a400, order 2).
        let home = |fx: &mut Fx, p: &mut SectorPlanner| with(fx, |io| p.home(io));
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        p.plan_fleet = Some(f);
        p.base = Some(fx.systems[1]);
        assert_eq!(home(&mut fx, &mut p), Some(fx.systems[1]));
        // Blockaded: no headquarters, no yard, the fleet at its own system.
        edit_system(&mut fx, 1, |r| r.state |= 2);
        assert_eq!(home(&mut fx, &mut p), None);
        // The headquarters.
        edit_system(&mut fx, 0, |r| r.state |= 0x8000_0000);
        assert_eq!(home(&mut fx, &mut p), Some(fx.systems[0]));
        // Blockaded: a yard with every mark (no base now).
        p.base = None;
        edit_system(&mut fx, 0, |r| r.state |= 2);
        edit_system(&mut fx, 1, |r| r.state = 0x3e0_0000);
        assert_eq!(home(&mut fx, &mut p), Some(fx.systems[1]));
        edit_system(&mut fx, 1, |r| r.state = 0x1e0_0000);
        assert_eq!(home(&mut fx, &mut p), None);
        // At a system of the opponent: the nearest without 4 or 0x200.
        edit_system(&mut fx, 0, |r| r.kind = 4);
        assert_eq!(home(&mut fx, &mut p), Some(fx.systems[1]));
        edit_system(&mut fx, 1, |r| r.kind |= 0x200);
        assert_eq!(home(&mut fx, &mut p), None);
        // No step fleet: nothing, not even an open base.
        p.plan_fleet = None;
        p.base = Some(fx.systems[1]);
        edit_system(&mut fx, 1, |r| r.state = 0);
        assert_eq!(home(&mut fx, &mut p), None);
    }

    #[test]
    fn kind_one_attacks_a_marked_system_of_the_opponent_when_no_enemy_fleet_is_beaten() {
        // FUN_004bafe0: kind 3's list first (values[2] above 0); with no
        // target and values[3] above 0, the systems with +0x2c 0x400000,
        // +0x30 4, without 0x100000 (key 0x1b, order 1), each through
        // FUN_004ae150. Kind 3 (FUN_004b4560) has no second list.
        let setup = |kind: u32, v3: i32, summary: u32| {
            let mut fx = fixture();
            let mut p = SectorPlanner {
                kind,
                ..planner(&fx)
            };
            let f = held(&mut fx, &mut p, 0, 1);
            (record(&mut fx, f).values[2], record(&mut fx, f).values[3]) = (13, v3);
            // The opponent's forces there: attack 10 (v[26]).
            edit_system(&mut fx, 1, |r| {
                r.kind = (r.kind & !1) | 4;
                r.summary |= summary;
                r.values[26] = 10;
            });
            p.plan_fleet = Some(f);
            p.plan_state = 7;
            (fx, p, f)
        };
        let (mut fx, mut p, f) = setup(1, 1, 0x40_0000);
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xd);
        assert_eq!(kind(&fx, 1) & 0x10_0000, 0x10_0000);
        // Only kind 2 keeps a visited list.
        assert_eq!(p.visited, []);
        let r = &fx.galaxy.fleets[&f];
        assert_eq!(
            (r.target, r.flags & ROLES),
            (Some(fx.systems[1]), ROLE_ATTACK)
        );
        // Kind 3, no defense (values[3]) or no 0x400000: no target.
        for (kind_, v3, summary) in [(3, 1, 0x40_0000), (1, 0, 0x40_0000), (1, 1, 0)] {
            let (mut fx, mut p, f) = setup(kind_, v3, summary);
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
            assert_eq!(p.plan_state, 8, "{kind_} {v3} {summary:#x}");
            assert_eq!(fx.galaxy.fleets[&f].target, None);
        }
        // Too strong (12/10 of 11 is 13): FUN_004ae150 fails.
        let (mut fx, mut p, _) = setup(1, 1, 0x40_0000);
        edit_system(&mut fx, 1, |r| r.values[26] = 11);
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 8);
    }

    #[test]
    fn kind_one_merges_three_bombardment_lists() {
        // FUN_004bb300 merges key 8 (order 2), key 0x21 and key 0 (order
        // 1); FUN_004b4730 key 8 (order 2) and key 0x10 (order 1). With
        // system 0 first by key 8 and by key 0x10, and last by keys 0x21
        // and 0, kind 3 picks system 1 and kind 1 system 0.
        let choose = |kind: u32| {
            let mut fx = fixture();
            let mut p = SectorPlanner {
                kind,
                ..planner(&fx)
            };
            let f = held(&mut fx, &mut p, 2, 1);
            record(&mut fx, f).values[6] = 5;
            bombardable(&mut fx, 0, (0, 1, 0), (4, 4));
            bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
            edit_system(&mut fx, 0, |r| {
                (r.values[8], r.values[0x10], r.values[0x21], r.values[0]) = (1, 0, 1, 1);
            });
            // No equal keys, so no coin.
            edit_system(&mut fx, 1, |r| {
                (r.values[8], r.values[0x10], r.values[0x21], r.values[0]) = (0, 1, 0, 0);
            });
            p.plan_fleet = Some(f);
            p.plan_state = 9;
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
            fx.galaxy.fleets[&f].target
        };
        let fx = fixture();
        assert_eq!(choose(3), Some(fx.systems[1]));
        assert_eq!(choose(1), Some(fx.systems[0]));
    }

    #[test]
    fn kind_one_sends_a_lone_fleet_to_the_system_seen_longest_ago() {
        // FUN_004bb630: one held fleet (FUN_005f50e0 on planner +0x2c),
        // record without +0x38 0xa00: the pick of FUN_0041a3a0(+0x30
        // forbidding 0x700201, order 1), keyed by FUN_00433de0 (today less
        // +0x44); then kind 3's chain (FUN_004b4a20).
        let setup = || {
            let mut fx = fixture();
            let far = fx.systems[1];
            let other = fx.world.systems.insert(fx.world.systems[far].clone());
            fx.world.systems[other].x = 60;
            rebuild(&mut fx);
            let mut p = SectorPlanner {
                kind: 1,
                ..planner(&fx)
            };
            let f = held(&mut fx, &mut p, 0, 1);
            p.plan_fleet = Some(f);
            for at in [far, other] {
                fx.galaxy.update_kind(at, 2, 1);
            }
            fx.day = 10;
            (fx, p, far, other)
        };
        let home = |fx: &mut Fx, p: &mut SectorPlanner| with(fx, |io| p.home(io));
        let (mut fx, mut p, far, other) = setup();
        fx.seen = HashMap::from([(far, 5), (other, 1)]);
        assert_eq!(home(&mut fx, &mut p), Some(other));
        fx.seen = HashMap::from([(far, 1), (other, 5)]);
        assert_eq!(home(&mut fx, &mut p), Some(far));
        // Equal ages go by the coin (FUN_0041c250: ahead on a draw below
        // 5), and the pick takes the last.
        fx.seen = HashMap::from([(far, 3), (other, 3)]);
        for (draw, chosen) in [(0, far), (4, far), (5, other), (9, other)] {
            fx.draws = vec![draw];
            assert_eq!(home(&mut fx, &mut p), Some(chosen), "draw {draw}");
        }
        // An enemy fleet or a role there passes it over.
        fx.galaxy.update_kind(far, 0x200, 0);
        assert_eq!(home(&mut fx, &mut p), Some(other));
        fx.galaxy.update_kind(other, 0x40_0000, 0);
        assert_eq!(home(&mut fx, &mut p), None);
        // Kind 3, two held fleets, regiments aboard (0x200) or en route
        // (0x800): no patrol (the chain finds nothing at an own system).
        let edits: [fn(&mut Fx, &mut SectorPlanner); 4] = [
            |_, p| p.kind = 3,
            |fx, p| {
                held(fx, p, 0, 1);
            },
            |fx, p| record(fx, p.plan_fleet.unwrap()).flags |= 0x200,
            |fx, p| record(fx, p.plan_fleet.unwrap()).flags |= 0x800,
        ];
        for edit in edits {
            let (mut fx, mut p, far, other) = setup();
            fx.seen = HashMap::from([(far, 5), (other, 1)]);
            edit(&mut fx, &mut p);
            for at in [far, other] {
                fx.galaxy.update_kind(at, 2, 1);
            }
            assert_eq!(home(&mut fx, &mut p), None);
        }
    }

    #[test]
    fn kind_two_remembers_its_attack_target_and_does_not_bombard() {
        // FUN_004b9c90: kind 1's lists, then FUN_004f57b0 adds the target
        // to +0x5c; slot +0x44 is 0x6158b0 (0), so state 9 goes to 0xc.
        let mut fx = fixture();
        let mut p = SectorPlanner {
            kind: 2,
            ..planner(&fx)
        };
        let f = held(&mut fx, &mut p, 0, 1);
        (record(&mut fx, f).values[2], record(&mut fx, f).values[3]) = (13, 1);
        edit_system(&mut fx, 1, |r| {
            r.kind = (r.kind & !1) | 4;
            r.summary |= 0x40_0000;
            r.values[26] = 10;
        });
        p.plan_fleet = Some(f);
        p.plan_state = 7;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xd);
        assert_eq!(p.visited, vec![fx.systems[1]]);
        // No visit without a target.
        let mut q = SectorPlanner {
            kind: 2,
            ..planner(&fx)
        };
        let g = held(&mut fx, &mut q, 0, 1);
        q.plan_fleet = Some(g);
        q.plan_state = 7;
        assert_eq!(step(&mut fx, &mut q), PlanStep::Order(None));
        assert_eq!((q.plan_state, q.visited.len()), (8, 0));
        // A bombarder that kind 3 would bind.
        let mut fx = fixture();
        let mut p = SectorPlanner {
            kind: 2,
            ..planner(&fx)
        };
        let f = held(&mut fx, &mut p, 2, 1);
        record(&mut fx, f).values[6] = 5;
        bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
        p.plan_fleet = Some(f);
        p.plan_state = 9;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xc);
        assert_eq!(fx.galaxy.fleets[&f].target, None);
    }

    #[test]
    fn kind_two_sends_a_lone_fleet_to_each_system_of_the_opponent_once() {
        // FUN_004ba0d0: one held fleet, without 0xa00, values[2] and [3]
        // at least controller +0x164/+0x168 (0): the first of
        // FUN_0041a3a0(+0x30 0x20 forbidding 0x700201) not on +0x5c, which
        // joins it; with none, kind 3's chain.
        let setup = || {
            let mut fx = fixture();
            let far = fx.systems[1];
            let other = fx.world.systems.insert(fx.world.systems[far].clone());
            fx.world.systems[other].x = 60;
            rebuild(&mut fx);
            let mut p = SectorPlanner {
                kind: 2,
                ..planner(&fx)
            };
            let f = held(&mut fx, &mut p, 0, 1);
            p.plan_fleet = Some(f);
            for at in [far, other] {
                fx.galaxy.update_kind(at, 0x24, 1);
            }
            fx.day = 10;
            fx.seen = HashMap::from([(far, 5), (other, 1)]);
            (fx, p, f, far, other)
        };
        let home = |fx: &mut Fx, p: &mut SectorPlanner| with(fx, |io| p.home(io));
        let (mut fx, mut p, _, far, other) = setup();
        assert_eq!(home(&mut fx, &mut p), Some(other));
        assert_eq!(home(&mut fx, &mut p), Some(far));
        assert_eq!(p.visited, vec![other, far]);
        assert_eq!(home(&mut fx, &mut p), None);
        // Equal ages go by the coin (ahead on a draw below 5); the pick
        // takes the last.
        let (mut fx, mut p, _, far, other) = setup();
        fx.seen = HashMap::from([(far, 3), (other, 3)]);
        for (draw, chosen) in [(4, far), (5, other)] {
            p.visited.clear();
            fx.draws = vec![draw];
            assert_eq!(home(&mut fx, &mut p), Some(chosen), "draw {draw}");
        }
        // A neutral system (+0x30 2, no 0x20) is not on the list.
        let (mut fx, mut p, _, far, other) = setup();
        fx.galaxy.update_kind(other, 2, 0x24);
        assert_eq!(home(&mut fx, &mut p), Some(far));
        // Two held fleets, regiments aboard, en route, or a weak fleet:
        // no sweep (the chain finds nothing at an own system).
        let edits: [fn(&mut Fx, &mut SectorPlanner, FleetKey); 5] = [
            |fx, p, _| {
                held(fx, p, 0, 1);
            },
            |fx, _, f| record(fx, f).flags |= 0x200,
            |fx, _, f| record(fx, f).flags |= 0x800,
            |fx, _, f| record(fx, f).values[2] = -1,
            |fx, _, f| record(fx, f).values[3] = -1,
        ];
        for edit in edits {
            let (mut fx, mut p, f, far, other) = setup();
            edit(&mut fx, &mut p, f);
            for at in [far, other] {
                fx.galaxy.update_kind(at, 0x24, 1);
            }
            assert_eq!(home(&mut fx, &mut p), None);
            assert_eq!(p.visited, []);
        }
    }

    #[test]
    fn slot_54_takes_the_nearest_safe_system_of_several() {
        // FUN_0041a400 order 2 with the pick: the nearest system.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        p.plan_fleet = Some(f);
        edit_system(&mut fx, 0, |r| r.kind = 4);
        let far = fx.systems[1];
        let near = fx.world.systems.insert(fx.world.systems[far].clone());
        fx.world.systems[near].x = 10;
        fx.world.systems[near].fleets.clear();
        rebuild(&mut fx);
        edit_system(&mut fx, 0, |r| r.kind = 4);
        assert_eq!(with(&mut fx, |io| p.home(io)), Some(near));
    }

    #[test]
    fn state_d_moves_the_fleet_to_its_target() {
        // FUN_004b0f90: Move 0x201 of the fleet to its record's +0x2c;
        // then 0xe.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 2);
        p.plan_fleet = Some(f);
        p.plan_state = 0xd;
        // No target: no order.
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xe);
        record(&mut fx, f).target = Some(fx.systems[1]);
        p.plan_state = 0xd;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(Some(AIAction::MoveShips {
                fleet: f,
                ships: vec![0, 1],
                to: fx.systems[1],
            }))
        );
        // A lost ship stays behind out of the move.
        fx.world.fleets[f].capital_ships[0].alive = false;
        p.plan_state = 0xd;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(Some(AIAction::MoveShips {
                fleet: f,
                ships: vec![1],
                to: fx.systems[1],
            }))
        );
    }

    #[test]
    fn state_e_keeps_an_attacker_while_it_prevails_and_frees_the_others() {
        // FUN_004b1070: each held fleet with a role, not en route; at its
        // target 0x10000000 stays while FUN_004ae150 holds; else, and
        // elsewhere, role and target go (0x0fffffff). 0x40000000 goes once
        // there. The walk's end ends the step (0, *out).
        let mut fx = fixture();
        let mut p = planner(&fx);
        let stays = held(&mut fx, &mut p, 1, 1);
        let weak = held(&mut fx, &mut p, 1, 1);
        let away = held(&mut fx, &mut p, 0, 1);
        let bomber = held(&mut fx, &mut p, 1, 1);
        let moving = held(&mut fx, &mut p, 0, 1);
        enemy_fleet(&mut fx, 1, (10, 0, 0));
        let target = Some(fx.systems[1]);
        for (f, role, attack) in [
            (stays, ROLE_ATTACK, 13),
            (weak, ROLE_ATTACK, 12),
            (away, ROLE_ATTACK, 13),
            (bomber, ROLE_BOMBARD, 13),
            (moving, ROLE_ATTACK | 2, 13),
        ] {
            with(&mut fx, |io| {
                io.edit_fleet(f, |r| {
                    r.flags |= role;
                    r.target = target;
                    r.values[2] = attack;
                });
            });
        }
        p.plan_state = 0xe;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Ended(None));
        assert_eq!(p.plan_state, 0);
        let bound = |fx: &Fx, f| {
            let r = &fx.galaxy.fleets[&f];
            (r.flags & ROLES, r.target)
        };
        assert_eq!(bound(&fx, stays), (ROLE_ATTACK, target));
        assert_eq!(bound(&fx, moving), (ROLE_ATTACK, target));
        for f in [weak, away, bomber] {
            assert_eq!(bound(&fx, f), (0, None));
            assert_eq!(fx.fleet_holds[&f].target, None);
            assert_eq!(fx.fleet_holds[&f].flags & ROLES, 0);
        }
        // A target without a record frees the fleet too.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        with(&mut fx, |io| {
            io.edit_fleet(f, |r| {
                r.flags |= ROLE_ATTACK;
                r.target = Some(SystemKey::default());
            });
        });
        p.plan_state = 0xe;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Ended(None));
        assert_eq!(bound(&fx, f), (0, None));
    }

    #[test]
    fn state_one_marks_each_held_fleet_s_target_by_its_role() {
        // FUN_004af6d0: the sector's systems lose 0x1f00000; a held fleet
        // whose +0x2c is a system marks it 0x1000000 for 0x80000000,
        // 0x100000 for 0x10000000, 0x200000 for 0x20000000, 0x400000 for
        // 0x40000000, the first it has.
        for (role, mark) in [
            (0x8000_0000 | ROLE_ATTACK, 0x100_0000),
            (ROLE_ATTACK | ROLE_BOMBARD, 0x10_0000),
            (0x2000_0000 | ROLE_BOMBARD, 0x20_0000),
            (ROLE_BOMBARD, 0x40_0000),
            (0, 0),
        ] {
            let mut fx = fixture();
            let mut p = planner(&fx);
            let f = held(&mut fx, &mut p, 0, 1);
            with(&mut fx, |io| io.set_kind(fx_system(io, 0), 0x1f0_0000, 0));
            let target = fx.systems[1];
            with(&mut fx, |io| {
                io.edit_fleet(f, |r| {
                    r.flags |= role;
                    r.target = Some(target);
                });
            });
            p.plan_state = 1;
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
            assert_eq!(kind(&fx, 0) & ROLE_MARKS, 0, "role {role:#x}");
            assert_eq!(kind(&fx, 1) & ROLE_MARKS, mark, "role {role:#x}");
        }
    }

    #[test]
    fn a_fleet_keeps_its_plan_s_role_and_target_across_refreshes() {
        // The record's +0x2c is written only by the plan objects, and the
        // refresh (FUN_00433620) keeps +0x38 & 0xf000000c.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        let target = fx.systems[1];
        with(&mut fx, |io| bind(io, f, target, ROLE_BOMBARD));
        rebuild(&mut fx);
        let r = &fx.galaxy.fleets[&f];
        assert_eq!((r.target, r.flags & ROLES), (Some(target), ROLE_BOMBARD));
    }

    #[test]
    fn equal_keys_go_by_the_coin_in_every_list() {
        // FUN_005f59f0 puts an entry with an equal key ahead on
        // FUN_0041cd80(10) < 5, and the pick (FUN_004357b0) takes the last
        // of a single list first: a draw of 4 and one of 5 choose apart.
        let draw = |d: i32| {
            // State 7: two enemy fleets of equal attack (key 0x27).
            let mut fx = fixture();
            let mut p = planner(&fx);
            let f = held(&mut fx, &mut p, 2, 1);
            record(&mut fx, f).values[2] = 13;
            enemy_fleet(&mut fx, 0, (10, 0, 0));
            enemy_fleet(&mut fx, 1, (10, 0, 0));
            fx.draws = vec![d];
            p.plan_fleet = Some(f);
            p.plan_state = 7;
            step(&mut fx, &mut p);
            let attack = fx.galaxy.fleets[&f].target;

            // State 9: two systems equal on keys 8 and 0x10, one draw each.
            let mut fx = fixture();
            let mut p = planner(&fx);
            let f = held(&mut fx, &mut p, 2, 1);
            record(&mut fx, f).values[6] = 5;
            bombardable(&mut fx, 0, (0, 1, 0), (4, 4));
            bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
            fx.draws = vec![d, d];
            p.plan_fleet = Some(f);
            p.plan_state = 9;
            step(&mut fx, &mut p);
            let bombard = fx.galaxy.fleets[&f].target;

            // State 0xc: two held fleets at one distance.
            let mut fx = fixture();
            let mut p = planner(&fx);
            let f = held(&mut fx, &mut p, 0, 1);
            let a = held(&mut fx, &mut p, 1, 1);
            let b = held(&mut fx, &mut p, 1, 1);
            fx.draws = vec![d];
            p.plan_fleet = Some(f);
            p.plan_state = 0xc;
            let join = match step(&mut fx, &mut p) {
                PlanStep::Order(Some(AIAction::JoinFleet { into, .. })) => {
                    usize::from(into == b) + 2 * usize::from(into == a)
                }
                other => panic!("{other:?}"),
            };

            // Slot +0x54: two safe systems at one distance.
            let mut fx = fixture();
            let mut p = planner(&fx);
            let f = held(&mut fx, &mut p, 0, 1);
            let twin = fx
                .world
                .systems
                .insert(fx.world.systems[fx.systems[1]].clone());
            fx.world.systems[twin].fleets.clear();
            rebuild(&mut fx);
            edit_system(&mut fx, 0, |r| r.kind = 4);
            fx.draws = vec![d];
            p.plan_fleet = Some(f);
            let home = with(&mut fx, |io| p.home(io));
            (attack, bombard, join, home == Some(twin), fx.systems)
        };
        let (attack, bombard, join, twin, systems) = draw(4);
        assert_eq!(attack, Some(systems[0]));
        assert_eq!(bombard, Some(systems[0]));
        assert_eq!(join, 2);
        assert!(!twin);
        let (attack, bombard, join, twin, systems) = draw(5);
        assert_eq!(attack, Some(systems[1]));
        assert_eq!(bombard, Some(systems[1]));
        assert_eq!(join, 1);
        assert!(twin);
    }

    #[test]
    fn a_bombarder_at_its_target_bombards_and_the_walk_goes_on_after_it() {
        // FUN_004b1070: a 0x40000000 fleet at its target gives
        // FUN_004b1330 with FUN_004ae320's code and loses role and target;
        // the order stops the walk, +0x30 naming the next fleet, from which
        // the next step goes on; an order on the last fleet ends the step.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let first = held(&mut fx, &mut p, 1, 1);
        let second = held(&mut fx, &mut p, 1, 1);
        bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
        let target = fx.systems[1];
        for f in [first, second] {
            record(&mut fx, f).values[6] = 5;
            with(&mut fx, |io| bind(io, f, target, ROLE_BOMBARD));
        }
        p.plan_state = 0xe;
        let order = |f| AIAction::Bombard {
            fleets: vec![f],
            system: target,
            kind: crate::bombardment::GENERAL,
        };
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(Some(order(first))));
        assert_eq!((p.plan_state, p.plan_walk), (0xe, Some(second)));
        assert_eq!(fx.galaxy.fleets[&first].flags & ROLES, 0);
        assert_eq!(fx.galaxy.fleets[&first].target, None);
        assert_eq!(fx.galaxy.fleets[&second].flags & ROLES, ROLE_BOMBARD);
        assert_eq!(step(&mut fx, &mut p), PlanStep::Ended(Some(order(second))));
        assert_eq!((p.plan_state, p.plan_walk), (0, None));
        // The walk goes on from +0x30, not from the first: bound again, the
        // first waits while the walk starts at the second.
        for f in [first, second] {
            with(&mut fx, |io| bind(io, f, target, ROLE_BOMBARD));
        }
        p.plan_state = 0xe;
        p.plan_walk = Some(second);
        assert_eq!(step(&mut fx, &mut p), PlanStep::Ended(Some(order(second))));
        assert_eq!(fx.galaxy.fleets[&first].flags & ROLES, ROLE_BOMBARD);
    }

    fn kind_four(fx: &Fx) -> SectorPlanner {
        SectorPlanner {
            kind: 4,
            ..planner(fx)
        }
    }

    #[test]
    fn kind_four_keeps_an_own_base_and_marks_it_else_picks_one_by_key_13() {
        // FUN_004b8070: a base not the side's or blockaded loses 0x50000 and
        // goes; else it takes 0x50000 (unless it has 0x10000) and the slot is
        // true; without one, the sector's own headquarters not blockaded
        // without 0x40000 by key 0x13 (order 1), else any such system.
        let mut fx = fixture();
        let mut p = kind_four(&fx);
        p.base = Some(fx.systems[1]);
        assert!(with(&mut fx, |io| p.invasion_base(io, INVASION_MARKS)));
        assert_eq!(kind(&fx, 1) & 0x5_0000, 0x5_0000);
        // With 0x10000 already, nothing is added.
        let mut fx = fixture();
        let mut p = kind_four(&fx);
        p.base = Some(fx.systems[1]);
        edit_system(&mut fx, 1, |r| r.kind |= 0x1_0000);
        assert!(with(&mut fx, |io| p.invasion_base(io, INVASION_MARKS)));
        assert_eq!(kind(&fx, 1) & 0x5_0000, 0x1_0000);
        // Blockaded: dropped with its marks; a new one by key 0x13.
        let mut fx = fixture();
        let mut p = kind_four(&fx);
        p.base = Some(fx.systems[1]);
        edit_system(&mut fx, 1, |r| {
            r.state |= 2;
            r.kind |= 0x5_0000;
        });
        edit_system(&mut fx, 0, |r| r.values[0x13] = 1);
        assert!(with(&mut fx, |io| p.invasion_base(io, INVASION_MARKS)));
        assert_eq!(p.base, Some(fx.systems[0]));
        assert_eq!(kind(&fx, 1) & 0x5_0000, 0);
        assert_eq!(kind(&fx, 0) & 0x5_0000, 0x5_0000);
        // Equal keys go by the coin (ahead on a draw below 5).
        let mut chosen = Vec::new();
        for draw in [4, 5] {
            let mut fx = fixture();
            let mut p = kind_four(&fx);
            fx.draws = vec![draw];
            with(&mut fx, |io| p.invasion_base(io, INVASION_MARKS));
            chosen.push(p.base);
        }
        let fx = fixture();
        assert_eq!(chosen, [Some(fx.systems[0]), Some(fx.systems[1])]);
        // Not the side's: dropped too.
        let mut fx = fixture();
        let mut p = kind_four(&fx);
        p.base = Some(fx.systems[1]);
        edit_system(&mut fx, 1, |r| r.kind &= !1);
        with(&mut fx, |io| p.invasion_base(io, INVASION_MARKS));
        assert_ne!(p.base, Some(fx.systems[1]));
    }

    #[test]
    fn kind_four_boards_the_strongest_regiments_at_its_base_onto_a_fleet_there() {
        // FUN_004b8470: with regiments at the base (v[19]), the first held
        // fleet there with room (values[22]) takes the strongest (slot
        // +0x1dc, TROOPSD attack, above 0), as many as the room and v[19].
        let mut fx = fixture();
        let mut p = kind_four(&fx);
        let f = held(&mut fx, &mut p, 1, 1);
        p.base = Some(fx.systems[1]);
        let mut troop = |attack: u32| {
            let class = crate::ids::DatId::new(0x1000_0100 + attack);
            fx.world.troop_classes.insert(
                class,
                crate::world::TroopClassDef {
                    attack_strength: attack,
                    defense_strength: 1,
                    detection: 0,
                },
            );
            let t = fx.world.troops.insert(TroopUnit {
                class_dat_id: class,
                is_alliance: true,
                regiment_strength: 100,
            });
            fx.world.systems[fx.systems[1]].ground_units.push(t);
            t
        };
        let (weak, none, strong) = (troop(5), troop(0), troop(7));
        let _ = none;
        edit_system(&mut fx, 1, |r| r.values[19] = 3);
        record(&mut fx, f).values[22] = 3;
        let order = with(&mut fx, |io| p.board(io));
        assert_eq!(
            order,
            Some(AIAction::BoardRegiments {
                troops: vec![strong, weak],
                fleet: f,
            })
        );
        // The room or v[19] limits the count.
        record(&mut fx, f).values[22] = 1;
        assert_eq!(
            with(&mut fx, |io| p.board(io)),
            Some(AIAction::BoardRegiments {
                troops: vec![strong],
                fleet: f,
            })
        );
        record(&mut fx, f).values[22] = 3;
        edit_system(&mut fx, 1, |r| r.values[19] = 1);
        assert_eq!(
            with(&mut fx, |io| p.board(io)),
            Some(AIAction::BoardRegiments {
                troops: vec![strong],
                fleet: f,
            })
        );
        // No room, no regiments counted, or no fleet at the base: nothing.
        for edit in [
            |fx: &mut Fx, f: FleetKey| record(fx, f).values[22] = 0,
            |fx: &mut Fx, _: FleetKey| edit_system(fx, 1, |r| r.values[19] = 0),
        ] {
            edit_system(&mut fx, 1, |r| r.values[19] = 3);
            record(&mut fx, f).values[22] = 3;
            edit(&mut fx, f);
            assert_eq!(with(&mut fx, |io| p.board(io)), None);
        }
        edit_system(&mut fx, 1, |r| r.values[19] = 3);
        record(&mut fx, f).values[22] = 3;
        p.base = Some(fx.systems[0]);
        edit_system(&mut fx, 0, |r| r.values[19] = 3);
        assert_eq!(with(&mut fx, |io| p.board(io)), None);
    }

    #[test]
    fn kind_four_attacks_its_target_first_then_assaults_it() {
        // FUN_004b8880: with an enemy fleet at the target that the fleet
        // prevails over, the target; else kind 3's list. FUN_004b8ab0: a
        // fleet with regiments (values[12]) passing FUN_004ae240 takes the
        // target as its assault (0x200000, 0x20000000); slot +0x44 is 0.
        let mut fx = fixture();
        let mut p = kind_four(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        let target = fx.systems[1];
        p.plan_target = Some(target);
        p.plan_fleet = Some(f);
        record(&mut fx, f).values[2] = 13;
        // An attacker already bound there (0x100000) keeps kind 3's list
        // off it; the target is tried without that test.
        edit_system(&mut fx, 1, |r| {
            r.kind = 0x10_0224;
            r.values[39] = 10;
        });
        p.plan_state = 7;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xd);
        let r = &fx.galaxy.fleets[&f];
        assert_eq!((r.target, r.flags & ROLES), (Some(target), ROLE_ATTACK));
        // Without +0x30 0x200 the target is not tried, though the fleet
        // would prevail over its forces (v[26], counted with +0x30 4).
        let mut fx = fixture();
        let mut p = kind_four(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        p.plan_target = Some(target);
        p.plan_fleet = Some(f);
        record(&mut fx, f).values[2] = 13;
        edit_system(&mut fx, 1, |r| {
            r.kind = 0x24;
            r.values[26] = 10;
        });
        p.plan_state = 7;
        step(&mut fx, &mut p);
        assert_eq!(p.plan_state, 8);
        // No enemy fleet there: state 8 binds the assault.
        let mut fx = fixture();
        let mut p = kind_four(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        p.plan_target = Some(target);
        p.plan_fleet = Some(f);
        edit_system(&mut fx, 1, |r| r.kind = 0x24);
        record(&mut fx, f).values[12] = 1;
        p.plan_state = 7;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 8);
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xd);
        assert_eq!(kind(&fx, 1) & 0x20_0000, 0x20_0000);
        let r = &fx.galaxy.fleets[&f];
        assert_eq!((r.target, r.flags & ROLES), (Some(target), ROLE_ASSAULT));
        // Without regiments, or with no target: 10, then 9 gives nothing
        // (no bombarder) and 0xc regroups.
        for edit in [
            |fx: &mut Fx, f: FleetKey, _: &mut SectorPlanner| record(fx, f).values[12] = 0,
            |_: &mut Fx, _: FleetKey, p: &mut SectorPlanner| p.plan_target = None,
        ] {
            let mut fx = fixture();
            let mut p = kind_four(&fx);
            let f = held(&mut fx, &mut p, 0, 1);
            p.plan_target = Some(target);
            p.plan_fleet = Some(f);
            edit_system(&mut fx, 1, |r| r.kind = 0x24);
            record(&mut fx, f).values[12] = 1;
            record(&mut fx, f).values[6] = 5;
            bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
            edit(&mut fx, f, &mut p);
            p.plan_state = 8;
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
            assert_eq!(p.plan_state, 10);
            step(&mut fx, &mut p);
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
            assert_eq!(p.plan_state, 0xc);
        }
    }

    #[test]
    fn kind_four_s_step_takes_its_own_base_and_boards_there() {
        // FUN_004adb70 with kind 4's slots: state 2 is FUN_004b8070 (marks
        // 0x50000), state 3 FUN_004b8470 (Move onto the fleet).
        let mut fx = fixture();
        let mut p = kind_four(&fx);
        let f = held(&mut fx, &mut p, 1, 1);
        p.base = Some(fx.systems[1]);
        let class = crate::ids::DatId::new(0x1000_0105);
        fx.world.troop_classes.insert(
            class,
            crate::world::TroopClassDef {
                attack_strength: 5,
                defense_strength: 1,
                detection: 0,
            },
        );
        let t = fx.world.troops.insert(TroopUnit {
            class_dat_id: class,
            is_alliance: true,
            regiment_strength: 100,
        });
        fx.world.systems[fx.systems[1]].ground_units.push(t);
        edit_system(&mut fx, 1, |r| r.values[19] = 1);
        record(&mut fx, f).values[22] = 1;
        p.plan_state = 2;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 3);
        assert_eq!(kind(&fx, 1) & 0x5_0000, 0x5_0000);
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(Some(AIAction::BoardRegiments {
                troops: vec![t],
                fleet: f,
            }))
        );
    }

    #[test]
    fn kind_four_retreats_to_its_base_even_when_blockaded() {
        // FUN_004b8ca0: the base without the blockade test; the
        // headquarters and a yard without it either.
        let mut fx = fixture();
        let mut p = kind_four(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        p.plan_fleet = Some(f);
        p.base = Some(fx.systems[1]);
        edit_system(&mut fx, 1, |r| r.state |= 2);
        assert_eq!(with(&mut fx, |io| p.home(io)), Some(fx.systems[1]));
        p.base = None;
        edit_system(&mut fx, 1, |r| r.state |= 0x8000_0000);
        assert_eq!(with(&mut fx, |io| p.home(io)), Some(fx.systems[1]));
        edit_system(&mut fx, 1, |r| r.state = 0x3e0_0002);
        assert_eq!(with(&mut fx, |io| p.home(io)), Some(fx.systems[1]));
        // Kind 3 passes the blockaded ones over.
        p.kind = 3;
        assert_eq!(with(&mut fx, |io| p.home(io)), None);
    }

    #[test]
    fn a_fleet_assaults_a_held_system_its_defense_and_troops_can_take() {
        // FUN_004ae240: +0x30 0x20 without 0x101; elsewhere values[3] at
        // least v[27] (shields) and values[9] at least v[23] unless
        // values[19] is above 0; values[12] at least v[52]; at the system
        // both thresholds are 0.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        let target = fx.systems[1];
        edit_system(&mut fx, 1, |r| {
            r.kind = 0x24;
            (r.values[27], r.values[23], r.values[52]) = (3, 2, 1);
        });
        let ok = |fx: &mut Fx| with(fx, |io| fleet_assaults(io, f, target));
        (
            record(&mut fx, f).values[3],
            record(&mut fx, f).values[9],
            record(&mut fx, f).values[12],
        ) = (3, 2, 1);
        assert!(ok(&mut fx));
        record(&mut fx, f).values[3] = 2;
        assert!(!ok(&mut fx));
        record(&mut fx, f).values[3] = 3;
        record(&mut fx, f).values[9] = 1;
        assert!(!ok(&mut fx));
        record(&mut fx, f).values[19] = 1;
        assert!(ok(&mut fx));
        record(&mut fx, f).values[12] = 0;
        assert!(!ok(&mut fx));
        record(&mut fx, f).values[12] = 1;
        // Own (1), fully shielded (0x100) or held by no side (no 0x20).
        for kind in [0x21, 0x124, 0x4] {
            edit_system(&mut fx, 1, |r| r.kind = kind);
            assert!(!ok(&mut fx), "{kind:#x}");
        }
        // At the system: no shield or troop threshold.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 1, 1);
        edit_system(&mut fx, 1, |r| {
            r.kind = 0x24;
            (r.values[27], r.values[23]) = (9, 9);
        });
        assert!(with(&mut fx, |io| fleet_assaults(io, f, fx_system(io, 1))));
    }

    #[test]
    fn an_assaulter_at_its_target_assaults_and_keeps_its_role() {
        // FUN_004b1070: a 0x20000000 fleet at its target gives Planetary
        // Assault 0x234 while FUN_004ae240 holds, else FUN_004ae320's
        // bombardment, else loses role and target; Destroy System with
        // fleet 0x10 and v[50]. After an order the role stays.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 1, 1);
        let target = fx.systems[1];
        edit_system(&mut fx, 1, |r| r.kind = 0x24);
        with(&mut fx, |io| bind(io, f, target, ROLE_ASSAULT));
        p.plan_state = 0xe;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Ended(Some(AIAction::Assault {
                fleets: vec![f],
                system: target,
            }))
        );
        let r = &fx.galaxy.fleets[&f];
        assert_eq!((r.flags & ROLES, r.target), (ROLE_ASSAULT, Some(target)));
        // FUN_004ae240 fails (v[52] above values[12]): a bombardment.
        edit_system(&mut fx, 1, |r| r.values[52] = 1);
        bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
        edit_system(&mut fx, 1, |r| r.values[52] = 1);
        record(&mut fx, f).values[6] = 5;
        p.plan_state = 0xe;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Ended(Some(AIAction::Bombard {
                fleets: vec![f],
                system: target,
                kind: crate::bombardment::GENERAL,
            }))
        );
        assert_eq!(fx.galaxy.fleets[&f].flags & ROLES, ROLE_ASSAULT);
        // Neither: the role and the target go.
        record(&mut fx, f).values[6] = 0;
        p.plan_state = 0xe;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Ended(None));
        let r = &fx.galaxy.fleets[&f];
        assert_eq!((r.flags & ROLES, r.target), (0, None));
        // A Death Star fleet needs v[50] above 0, and v[50] needs the
        // Death Star (fleet 0x10): else the assault.
        let assault = AIAction::Assault {
            fleets: vec![f],
            system: target,
        };
        edit_system(&mut fx, 1, |r| r.values[52] = 0);
        for (death_star, v50) in [(true, 0), (false, 1)] {
            with(&mut fx, |io| bind(io, f, target, ROLE_ASSAULT));
            if death_star {
                record(&mut fx, f).flags |= 0x10;
            } else {
                record(&mut fx, f).flags &= !0x10;
            }
            edit_system(&mut fx, 1, |r| r.values[50] = v50);
            p.plan_state = 0xe;
            assert_eq!(
                step(&mut fx, &mut p),
                PlanStep::Ended(Some(assault.clone()))
            );
        }
        // A Death Star fleet with v[50]: Destroy System.
        with(&mut fx, |io| bind(io, f, target, ROLE_ASSAULT));
        record(&mut fx, f).flags |= 0x10;
        edit_system(&mut fx, 1, |r| r.values[50] = 1);
        p.plan_state = 0xe;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Ended(Some(AIAction::Bombard {
                fleets: vec![f],
                system: target,
                kind: crate::bombardment::DESTROY_SYSTEM,
            }))
        );
    }

    #[test]
    fn a_death_star_bombarder_destroys_a_system_with_v50() {
        // FUN_004b1070: with fleet 0x10 and the system's v[50] (+0x110)
        // above 0, FUN_004b1330 gives Destroy System 0x223.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 1, 1);
        bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
        edit_system(&mut fx, 1, |r| r.values[50] = 1);
        let target = fx.systems[1];
        record(&mut fx, f).values[6] = 5;
        record(&mut fx, f).flags |= 0x10;
        with(&mut fx, |io| bind(io, f, target, ROLE_BOMBARD));
        p.plan_state = 0xe;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Ended(Some(AIAction::Bombard {
                fleets: vec![f],
                system: target,
                kind: crate::bombardment::DESTROY_SYSTEM,
            }))
        );
        // v[50] without the fleet's 0x10: FUN_004ae320's code.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 1, 1);
        bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
        edit_system(&mut fx, 1, |r| r.values[50] = 1);
        record(&mut fx, f).values[6] = 5;
        with(&mut fx, |io| bind(io, f, target, ROLE_BOMBARD));
        p.plan_state = 0xe;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Ended(Some(AIAction::Bombard {
                fleets: vec![f],
                system: target,
                kind: crate::bombardment::GENERAL,
            }))
        );
        // Without v[50]: FUN_004ae320's code; without it, no order.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 1, 1);
        bombardable(&mut fx, 1, (0, 0, 0), (4, 4));
        record(&mut fx, f).flags |= 0x10;
        record(&mut fx, f).values[6] = 5;
        with(&mut fx, |io| bind(io, f, target, ROLE_BOMBARD));
        p.plan_state = 0xe;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Ended(None));
        assert_eq!(fx.galaxy.fleets[&f].flags & ROLES, 0);
    }

    fn kind_five(fx: &Fx) -> SectorPlanner {
        SectorPlanner {
            kind: 5,
            ..planner(fx)
        }
    }

    fn sector_mut(fx: &mut Fx) -> &mut crate::ai_records::SectorRecord {
        let sector = fx.sectors[0];
        fx.galaxy
            .sectors
            .iter_mut()
            .find(|s| s.sector == sector)
            .unwrap()
    }

    #[test]
    fn kind_five_marks_its_base_0x60000_and_drops_the_invasion_mark() {
        // FUN_004b24b0: a kept base without 0x20000 takes
        // `& 0xfffeffff | 0x60000`; with 0x20000 it is left; a lost base
        // drops 0x70000 (`& 0xfff8ffff`); a new one takes the same marks.
        let mut fx = fixture();
        let mut p = kind_five(&fx);
        p.base = Some(fx.systems[1]);
        edit_system(&mut fx, 1, |r| r.kind |= 0x1_0000);
        assert!(with(&mut fx, |io| p.invasion_base(io, CONSOLIDATION_MARKS)));
        assert_eq!(kind(&fx, 1) & BASES, 0x6_0000);
        let mut fx = fixture();
        edit_system(&mut fx, 1, |r| r.kind |= 0x3_0000);
        assert!(with(&mut fx, |io| p.invasion_base(io, CONSOLIDATION_MARKS)));
        assert_eq!(kind(&fx, 1) & BASES, 0x3_0000);
        let mut fx = fixture();
        edit_system(&mut fx, 1, |r| {
            r.state |= 2;
            r.kind |= BASES;
        });
        edit_system(&mut fx, 0, |r| r.values[0x13] = 1);
        edit_system(&mut fx, 0, |r| r.kind |= 0x1_0000);
        // Kind 5's step takes this base in state 2.
        p.plan_state = 2;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!((p.base, p.plan_state), (Some(fx.systems[0]), 3));
        assert_eq!(kind(&fx, 1) & BASES, 0);
        assert_eq!(kind(&fx, 0) & BASES, 0x6_0000);
    }

    #[test]
    fn kind_five_boards_by_defense_unless_an_opponent_holds_a_system_there() {
        // FUN_004b28c0: kind 4's boarding with the sector record; the best
        // slot +0x1e0 (TROOPSD defense) while the sector's +0x2c lacks 8,
        // else +0x1dc (attack).
        let mut fx = fixture();
        let mut p = kind_five(&fx);
        let f = held(&mut fx, &mut p, 1, 1);
        p.base = Some(fx.systems[1]);
        let mut troop = |n: u32, attack: u32, defense: u32| {
            let class = crate::ids::DatId::new(0x1000_0100 + n);
            fx.world.troop_classes.insert(
                class,
                crate::world::TroopClassDef {
                    attack_strength: attack,
                    defense_strength: defense,
                    detection: 0,
                },
            );
            let t = fx.world.troops.insert(TroopUnit {
                class_dat_id: class,
                is_alliance: true,
                regiment_strength: 100,
            });
            fx.world.systems[fx.systems[1]].ground_units.push(t);
            t
        };
        let (raider, guard, mixed) = (troop(1, 7, 1), troop(2, 1, 6), troop(3, 3, 3));
        edit_system(&mut fx, 1, |r| r.values[19] = 2);
        record(&mut fx, f).values[22] = 2;
        sector_mut(&mut fx).summary &= !8;
        let board = |fx: &mut Fx, p: &mut SectorPlanner| with(fx, |io| p.board(io));
        let onto = |troops| Some(AIAction::BoardRegiments { troops, fleet: f });
        assert_eq!(board(&mut fx, &mut p), onto(vec![guard, mixed]));
        // State 3 boards for kind 5 (slot +0x38).
        p.plan_state = 3;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Order(onto(vec![guard, mixed]))
        );
        sector_mut(&mut fx).summary |= 8;
        assert_eq!(board(&mut fx, &mut p), onto(vec![raider, mixed]));
        // Kind 4 goes by attack either way.
        sector_mut(&mut fx).summary &= !8;
        p.kind = 4;
        assert_eq!(board(&mut fx, &mut p), onto(vec![raider, mixed]));
        // Kind 5 without the sector record boards nothing.
        p.kind = 5;
        p.sector = None;
        assert_eq!(board(&mut fx, &mut p), None);
    }

    /// Systems 0 and 1 neutral (`+0x30` 2) with their key `0xf`.
    fn neutral(fx: &mut Fx, keys: [i32; 2]) {
        for (at, key) in keys.into_iter().enumerate() {
            edit_system(fx, at, |r| {
                r.kind = 2;
                r.values[0xf] = key;
            });
        }
    }

    #[test]
    fn kind_five_sends_a_fleet_with_regiments_to_occupy_a_neutral_system() {
        // FUN_004b2f40 (state 10, slot +0x48): values[12] above 0; the
        // sector's systems with +0x30 2 forbidding 0x100208, key 0xf order
        // 1 (picked from the last); the first passing FUN_004ae440 takes
        // 0x1000000, the fleet the target and role 0x80000000: state 0xd.
        // Else state 9.
        let setup = |keys| {
            let mut fx = fixture();
            let mut p = kind_five(&fx);
            let f = held(&mut fx, &mut p, 0, 1);
            record(&mut fx, f).values[12] = 1;
            neutral(&mut fx, keys);
            p.plan_fleet = Some(f);
            p.plan_state = 10;
            (fx, p, f)
        };
        let (mut fx, mut p, f) = setup([1, 2]);
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xd);
        assert_eq!(fx.galaxy.fleets[&f].target, Some(fx.systems[1]));
        assert_eq!(fx.galaxy.fleets[&f].flags & ROLES, ROLE_OCCUPY);
        assert_eq!(kind(&fx, 1) & 0x100_0000, 0x100_0000);
        let (mut fx, mut p, f) = setup([2, 1]);
        record(&mut fx, f).values[12] = 2;
        step(&mut fx, &mut p);
        assert_eq!(fx.galaxy.fleets[&f].target, Some(fx.systems[0]));
        // Equal keys go by the coin (ahead on a draw below 5).
        for (draw, at) in [(4, 0), (5, 1)] {
            let (mut fx, mut p, f) = setup([1, 1]);
            fx.draws = vec![draw];
            step(&mut fx, &mut p);
            assert_eq!(fx.galaxy.fleets[&f].target, Some(fx.systems[at]));
        }
        // A populated system, an enemy fleet, a mark, or not neutral: the
        // other one.
        for bits in [8, 0x200, 0x10_0000] {
            let (mut fx, mut p, f) = setup([1, 2]);
            edit_system(&mut fx, 1, |r| r.kind |= bits);
            step(&mut fx, &mut p);
            assert_eq!(fx.galaxy.fleets[&f].target, Some(fx.systems[0]));
        }
        let (mut fx, mut p, f) = setup([1, 2]);
        edit_system(&mut fx, 1, |r| r.kind = 1);
        step(&mut fx, &mut p);
        assert_eq!(fx.galaxy.fleets[&f].target, Some(fx.systems[0]));
        // No regiments, or another kind: state 9 and no role.
        for edit in [
            |fx: &mut Fx, p: &mut SectorPlanner, f| {
                let _ = p;
                record(fx, f).values[12] = 0;
            },
            |_: &mut Fx, p: &mut SectorPlanner, _| p.kind = 3,
        ] {
            let (mut fx, mut p, f) = setup([1, 2]);
            edit(&mut fx, &mut p, f);
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
            assert_eq!(p.plan_state, 9);
            assert_eq!(fx.galaxy.fleets[&f].flags & ROLES, 0);
        }
    }

    #[test]
    fn an_occupier_at_its_target_lands_one_regiment_while_it_can() {
        // FUN_004b1070 with role 0x80000000 at the target: while
        // FUN_004ae440 holds, FUN_004b1440 gives the first regiment of the
        // first ship carrying any (+0x68), Move 0x201 to the target; the
        // role stays. When it fails the role goes.
        let mut fx = fixture();
        let mut p = kind_five(&fx);
        let f = fleet(&mut fx, 1, 1);
        let class = fx.world.fleets[f].capital_ships[0].class;
        fx.world.capital_ship_classes[class].troop_capacity = 5;
        let troops: Vec<TroopKey> = (0..2)
            .map(|_| {
                let t = fx.world.troops.insert(TroopUnit {
                    class_dat_id: crate::ids::DatId::new(0x1000_0001),
                    is_alliance: true,
                    regiment_strength: 100,
                });
                fx.world.systems[fx.systems[1]].ground_units.push(t);
                t
            })
            .collect();
        fx.transport.load(&mut fx.world, f, &troops).unwrap();
        crate::ai_planner::tests::rebuild(&mut fx);
        let id = ship_id(&fx, f, 0);
        with(&mut fx, |io| {
            p.take_fleet(io, f);
            p.take_ship(io, id);
        });
        record(&mut fx, f).values[12] = 2;
        neutral(&mut fx, [0, 0]);
        let target = fx.systems[1];
        with(&mut fx, |io| bind(io, f, target, ROLE_OCCUPY));
        p.plan_state = 0xe;
        assert_eq!(
            step(&mut fx, &mut p),
            PlanStep::Ended(Some(AIAction::MoveRegiments {
                troops: vec![troops[0]],
                to: target,
            }))
        );
        assert_eq!(fx.galaxy.fleets[&f].flags & ROLES, ROLE_OCCUPY);
        // No ship record carrying regiments: no order, the role stays.
        record(&mut fx, f).ships[0].values[9] = 0;
        p.plan_state = 0xe;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Ended(None));
        assert_eq!(fx.galaxy.fleets[&f].flags & ROLES, ROLE_OCCUPY);
        // No regiments left (values[12]): the role and target go.
        record(&mut fx, f).values[12] = 0;
        p.plan_state = 0xe;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Ended(None));
        assert_eq!(fx.galaxy.fleets[&f].flags & ROLES, 0);
        record(&mut fx, f).values[12] = 2;
        // The side's now, or populated: the role and target go.
        record(&mut fx, f).ships[0].values[9] = 2;
        for bits in [0x21, 0x2 | 8] {
            with(&mut fx, |io| bind(io, f, target, ROLE_OCCUPY));
            edit_system(&mut fx, 1, |r| r.kind = bits);
            p.plan_state = 0xe;
            assert_eq!(step(&mut fx, &mut p), PlanStep::Ended(None));
            assert_eq!(fx.galaxy.fleets[&f].flags & ROLES, 0);
            assert_eq!(fx.galaxy.fleets[&f].target, None);
        }
    }

    #[test]
    fn kind_five_bombards_the_opponent_s_held_systems_by_keys_8_and_0x21() {
        // FUN_004b3110: kind 3's FUN_004b4730 on +0x30 0x24 (forbidding
        // 0x700200), merging key 8 (order 2) and key 0x21 (order 1). With
        // system 0 first by key 8 and by key 0x10 and last by key 0x21,
        // kind 3 picks system 1 and kind 5 system 0; without 0x20, kind 5
        // passes a system over.
        let choose = |kind: u32, held_bits: [u32; 2]| {
            let mut fx = fixture();
            let mut p = SectorPlanner {
                kind,
                ..planner(&fx)
            };
            let f = held(&mut fx, &mut p, 2, 1);
            record(&mut fx, f).values[6] = 5;
            bombardable(&mut fx, 0, (0, 1, 0), (4, 4));
            bombardable(&mut fx, 1, (0, 1, 0), (4, 4));
            for (at, bits) in held_bits.into_iter().enumerate() {
                edit_system(&mut fx, at, |r| r.kind = (r.kind & !0x20) | bits);
            }
            edit_system(&mut fx, 0, |r| {
                (r.values[8], r.values[0x10], r.values[0x21]) = (1, 0, 1);
            });
            edit_system(&mut fx, 1, |r| {
                (r.values[8], r.values[0x10], r.values[0x21]) = (0, 1, 0);
            });
            p.plan_fleet = Some(f);
            p.plan_state = 9;
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
            fx.galaxy.fleets[&f].target
        };
        let fx = fixture();
        assert_eq!(choose(3, [0x20, 0x20]), Some(fx.systems[1]));
        assert_eq!(choose(5, [0x20, 0x20]), Some(fx.systems[0]));
        assert_eq!(choose(5, [0, 0x20]), Some(fx.systems[1]));
        assert_eq!(choose(3, [0, 0]), Some(fx.systems[1]));
    }

    #[test]
    fn kind_five_s_lone_fleet_without_a_base_goes_to_a_system_not_the_side_s() {
        // FUN_004b3400: one held fleet, no base, record without +0x38
        // 0xa00: a random system of the sector (FUN_0041a460) without +0x2c
        // 0x8000000 (explored) and +0x30 0x700201; then kind 3's chain.
        let mut fx = fixture();
        let mut p = kind_five(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        p.plan_fleet = Some(f);
        edit_system(&mut fx, 1, |r| {
            r.kind &= !1;
            r.summary &= !0x800_0000;
        });
        let home = |fx: &mut Fx, p: &mut SectorPlanner| with(fx, |io| p.home(io));
        assert_eq!(home(&mut fx, &mut p), Some(fx.systems[1]));
        // Kind 3 has no such step.
        p.kind = 3;
        assert_eq!(home(&mut fx, &mut p), None);
        p.kind = 5;
        // Explored, a fleet en route, or two fleets: none.
        edit_system(&mut fx, 1, |r| r.summary |= 0x800_0000);
        assert_eq!(home(&mut fx, &mut p), None);
        edit_system(&mut fx, 1, |r| r.summary &= !0x800_0000);
        record(&mut fx, f).flags |= 0x800;
        assert_eq!(home(&mut fx, &mut p), None);
        record(&mut fx, f).flags &= !0x800;
        let other = held(&mut fx, &mut p, 0, 1);
        let _ = other;
        assert_eq!(home(&mut fx, &mut p), None);
        // With a base, the base.
        let mut fx = fixture();
        let mut p = kind_five(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        p.plan_fleet = Some(f);
        edit_system(&mut fx, 1, |r| {
            r.kind &= !1;
            r.summary &= !0x800_0000;
        });
        p.base = Some(fx.systems[0]);
        assert_eq!(home(&mut fx, &mut p), Some(fx.systems[0]));
    }
}
