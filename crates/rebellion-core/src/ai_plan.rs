//! A sector planner's plan object (`FUN_004ad8e0`, vtable kinds by slot
//! `+0x14`) and its step (`FUN_004adb70`), which runs as state 5 of the
//! planner's step: the plan's base, the regiments and squadrons of its
//! held fleets, one idle held fleet a cycle, and that fleet's people
//! (`ghidra/notes/ai-sector-plans.md`, "The plan object's step").
//!
//! port: only kind 3's slots are ported (vtable `0x65c160`), and of the
//! step only states 1 to 6; from state 7 (the fleet's role, slots `+0x3c`
//! to `+0x48`) to `0xe` (the roles' orders, `FUN_004b1070`) the port's
//! fleet heuristics stand in, once a cycle.

use crate::ai::AIAction;
use crate::ai_module1::ANY;
use crate::ai_module8::SectorPlanner;
use crate::ai_planner::PlannerIo;
use crate::ai_records::{pick, picks, ranked, Mask, Order};
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

/// What the plan object's step gives the planner's.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanStep {
    /// A state ran, with its order if it gave one; the step goes on.
    Order(Option<AIAction>),
    /// States 7 to `0xe`: the port's heuristics stand in.
    StandIn,
}

impl SectorPlanner {
    /// One step of the plan object (`FUN_004adb70`).
    pub fn plan_step(&mut self, io: &mut PlannerIo<'_>) -> PlanStep {
        let order = match self.plan_state {
            1 => {
                self.mark_roles(io);
                self.plan_state = 2;
                None
            }
            2 => {
                self.plan_state = if self.choose_base(io) { 3 } else { 5 };
                None
            }
            3 => {
                self.plan_state = 4;
                self.unload(io)
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
            7 | 0xe => return PlanStep::StandIn,
            _ => {
                self.plan_state = 1;
                None
            }
        };
        PlanStep::Order(order)
    }

    /// State 1 (`FUN_004af6d0`): the sector's systems lose the role marks
    /// `0x1f00000`; each held fleet with a role marks its target system.
    ///
    /// port: no held fleet has a role or a target yet (the slots that set
    /// them, `+0x3c` and `+0x44`, are not ported), so only the marks are
    /// cleared.
    fn mark_roles(&self, io: &mut PlannerIo<'_>) {
        let Some(record) = self.sector.and_then(|s| io.galaxy.sector(s)) else {
            return;
        };
        let systems: Vec<SystemKey> = record.systems.iter().map(|r| r.system).collect();
        for system in systems {
            io.set_kind(system, 0, ROLE_MARKS);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_planner::tests::{fixture, fleet, planner, rebuild, ship_id, with, Fx};
    use crate::world::{Character, FighterClass, FighterEntry, TroopUnit};

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

    fn step(fx: &mut Fx, p: &mut SectorPlanner) -> PlanStep {
        with(fx, |io| p.plan_step(io))
    }

    fn kind(fx: &Fx, at: usize) -> u32 {
        fx.galaxy.system(fx.systems[at]).unwrap().kind
    }

    fn edit_system(fx: &mut Fx, at: usize, f: impl FnOnce(&mut crate::ai_records::SystemRecord)) {
        f(fx.galaxy.system_mut(fx.systems[at]).unwrap());
    }

    #[test]
    fn the_plan_object_steps_from_its_base_to_its_idle_fleet_then_stands_in() {
        // FUN_004adb70: 0 -> 1; 1 -> 2; 2 with a base -> 3; 3 -> 4; 4 -> 5;
        // 5 with an idle held fleet -> 6; 6 -> 7, where the port's
        // heuristics stand in.
        let mut fx = fixture();
        let mut p = planner(&fx);
        let f = held(&mut fx, &mut p, 0, 1);
        let mut states = Vec::new();
        for _ in 0..7 {
            assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
            states.push(p.plan_state);
        }
        assert_eq!(states, [1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(p.plan_fleet, Some(f));
        assert_eq!(step(&mut fx, &mut p), PlanStep::StandIn);
        assert_eq!(p.plan_state, 7);

        // 2 without a base -> 5: no system of the side's anywhere.
        let mut fx = fixture();
        let mut p = planner(&fx);
        for at in 0..4 {
            edit_system(&mut fx, at, |r| r.kind &= !1);
        }
        p.plan_state = 2;
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!((p.plan_state, p.base), (5, None));
        // 5 without an idle held fleet -> 0xe, which stands in.
        assert_eq!(step(&mut fx, &mut p), PlanStep::Order(None));
        assert_eq!(p.plan_state, 0xe);
        assert_eq!(step(&mut fx, &mut p), PlanStep::StandIn);
        // Any other state starts over at 1.
        p.plan_state = 8;
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
}
