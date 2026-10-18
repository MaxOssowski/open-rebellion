//! The AI mission planners' team and decoy selectors
//! (`ghidra/notes/ai-mission-planning.md`).
//!
//! The original keeps one record per character and special force
//! (`FUN_00401d20`, refreshed by `FUN_00402230`): flags at `+0x30`, the
//! missions the object may legally undertake at `+0x34`, and its eight
//! skills at `+0x40`. Each planner class queries those records
//! (`FUN_00403460`), ranks the hits, and takes the best (`FUN_004357b0`)
//! until its team, then its decoy list, is full.
//!
//! port: the original planner runs one state per tick (`FUN_004bced0`) and
//! re-enters its selectors on later cycles. The port plans in one cycle, so
//! the target system (`+0x38`, set in state 6 after the first cycle's
//! selectors) is unset while the selectors run and the nearness query
//! (`FUN_00403750`) never joins.

use std::collections::HashSet;

use crate::ids::{CharacterKey, SystemKey};
use crate::mission_detection::MissionRng;
use crate::missions::{member_skill, members_allowed, MissionKind, MissionMember, MissionState};
use crate::uprising::Draws;
use crate::world::{GameWorld, Skill};

// Record flags `+0x30` (`FUN_00401d20`, `FUN_00402230`).

/// `+0x50` bit 2 is clear: a character the side has not recruited.
const INACTIVE: u32 = 0x1;
/// The object is on the planner's side (`FUN_00401d20`).
const OWN_SIDE: u32 = 0x2;
/// The object is on the other side, or is a prisoner of its own
/// (`FUN_00401d20`).
const OTHER_SIDE: u32 = 0x4;
/// One of the six main characters (`FUN_004025f0`).
const MAIN_CHARACTER: u32 = 0x8;
// 0x10, injured (`+0x94` != 0): the port has no injuries.
/// En route (`+0x50` bit 4, `FUN_00402230`).
const EN_ROUTE: u32 = 0x20;
/// A prisoner (`+0xac` bit 0, `FUN_00401d20`).
const PRISONER: u32 = 0x40;
/// On a mission (role bit 7, `FUN_00402230`).
const ON_MISSION: u32 = 0x80;
/// On a mandatory mission (role bit 9, `FUN_00402230`).
const MANDATORY: u32 = 0x40_0000;
/// Set at construction; cleared when a planner takes the object
/// (`FUN_0047b610`, `FUN_0047b730`).
const UNASSIGNED: u32 = 0x80_0000;
/// Officer rank 1 (`+0x96` = 1, `FUN_00402230`; ranks 2 and 3 are
/// `0x1000000` and `0x2000000`).
const OFFICER_RANK_1: u32 = 0x400_0000;
/// Reserved: a Diplomacy-capable object with diplomacy of 80 or more, or a
/// research-capable one the side needs (`FUN_00402230`).
const RESERVED: u32 = 0x800_0000;
/// A special force (DatId family `0x3c..0x3f`, `FUN_00401d20`).
const SPECIAL_FORCE: u32 = 0x1000_0000;
/// A character (`FUN_00401d20`).
const CHARACTER: u32 = 0x2000_0000;
/// One of the four leaders (`FUN_004024d0`).
const LEADER: u32 = 0x4000_0000;

/// The flags an emitted member may not carry (`FUN_0047b360`).
const NOT_SENT: u32 = 0x0740_00f1;

/// The capability bit of each of the port's mission kinds (`FUN_00402720`).
fn capability(kind: MissionKind) -> Option<u32> {
    Some(match kind {
        MissionKind::Diplomacy => 0x1,
        MissionKind::Rescue => 0x2,
        MissionKind::Sabotage => 0x4,
        MissionKind::Espionage => 0x8,
        MissionKind::Abduction => 0x10,
        MissionKind::SubdueUprising => 0x20,
        MissionKind::Assassination => 0x40,
        MissionKind::DeathStarSabotage => 0x80,
        MissionKind::InciteUprising => 0x100,
        MissionKind::Reconnaissance => 0x200,
        MissionKind::Recruitment => 0x400,
        MissionKind::ShipDesignResearch => 0x800,
        MissionKind::TroopTrainingResearch => 0x1000,
        MissionKind::FacilityDesignResearch => 0x2000,
        // No planner sends these: Autoscrap is the Autoscrap planner's, and
        // Move is the trip an AI planner orders before its mission (FUN_004bd570).
        MissionKind::Autoscrap | MissionKind::Move => return None,
        // port: the AI's Jedi Training planner is not read, so it sends none.
        MissionKind::JediTraining => return None,
        // The mission manager sends Dagobah (FUN_0054a860), not a planner.
        MissionKind::Dagobah => return None,
        // The bounty timer sends the Bounty mission (FUN_00546fa0).
        MissionKind::Bounty => return None,
        // The mission manager sends the Palace missions (FUN_0054a910,
        // FUN_0054aa60, FUN_0054ab10).
        MissionKind::Palace => return None,
        // DarthPickup's hook sends the Pickup (FUN_0054abc0).
        MissionKind::Pickup => return None,
    })
}

/// The port's agent kinds, in capability-bit order.
pub(crate) const KINDS: [MissionKind; 14] = [
    MissionKind::Diplomacy,
    MissionKind::Rescue,
    MissionKind::Sabotage,
    MissionKind::Espionage,
    MissionKind::Abduction,
    MissionKind::SubdueUprising,
    MissionKind::Assassination,
    MissionKind::DeathStarSabotage,
    MissionKind::InciteUprising,
    MissionKind::Reconnaissance,
    MissionKind::Recruitment,
    MissionKind::ShipDesignResearch,
    MissionKind::TroopTrainingResearch,
    MissionKind::FacilityDesignResearch,
];

/// The three Research records (`0x53000020`, `0x53000022`, `0x53000021`;
/// capability bits `0x800`, `0x1000`, `0x2000`; `FUN_00402720`).
#[cfg(test)]
const RESEARCH_RECORDS: [u32; 3] = [0x5300_0020, 0x5300_0022, 0x5300_0021];

/// The six main characters (`FUN_004025f0`), as record ids.
const MAIN_CHARACTERS: [u32; 6] = [0x240, 0x241, 0x242, 0x243, 0x280, 0x281];
/// The four leaders (`FUN_004024d0`): `0x30000240`, `0x32000242`,
/// `0x34000280`, and `0x35000281`.
const LEADERS: [u32; 4] = [0x240, 0x242, 0x280, 0x281];

/// One AI record: `+0x30`, `+0x34`, and the skills at `+0x40`.
#[derive(Debug, Clone)]
struct AgentRecord {
    member: MissionMember,
    flags: u32,
    caps: u32,
    skills: [u32; 8],
    /// port: an earlier plan in this evaluation sent the object, or a
    /// research planner holds it ([`Planner::hold`]).
    sent: bool,
    /// `FUN_00402230`'s Diplomacy reserve: Diplomacy-capable with
    /// diplomacy above 79.
    diplomat: bool,
    /// The object stands, not en route, at a blockaded system: its
    /// location (`+0x38`) is a system whose record has `+0x28` bit `0x2`
    /// (`FUN_00433e40`, system `+0x88` bit `0x20`).
    at_blockade: bool,
}

impl AgentRecord {
    /// `FUN_00402230`: reserved (`0x8000000`) when able for a research tree
    /// (capability `0x800`, `0x1000`, `0x2000`) whose controller bit
    /// (`0x2000000`, `0x4000000`, `0x8000000`) is clear, or when a
    /// diplomat above 79. `done` is the ship, troop and facility trees'
    /// bits, which `FUN_004198a0`, `FUN_00419870` and `FUN_004197b0` set
    /// once the side's research order reaches its target (`+0x140..+0x148`).
    fn reserve(&mut self, done: [bool; 3]) {
        let research = [0x800, 0x1000, 0x2000]
            .into_iter()
            .zip(done)
            .any(|(bit, done)| self.caps & bit != 0 && !done);
        if research || self.diplomat {
            self.flags |= RESERVED;
        } else {
            self.flags &= !RESERVED;
        }
    }
}

/// One candidate query (`FUN_00403460` and `FUN_004035d0`).
#[derive(Debug, Clone, Copy)]
struct Query {
    req_flags: u32,
    req_caps: u32,
    forbid_flags: u32,
    forbid_caps: u32,
    skill: Skill,
    min: u32,
}

/// A planner class's selectors (`FUN_0042f830`).
struct Selectors {
    team: &'static [Query],
    team_size: usize,
    /// The decoy query and its cap; `None` for a class that takes no decoys.
    decoys: Option<(Query, DecoyCap)>,
}

/// Where a decoy selector reads its cap.
#[derive(Clone, Copy)]
enum DecoyCap {
    /// `(record.+0xbc + 2) / 2` of the target system's AI record, whose
    /// `+0xbc` is the mission kind its posture recommends (`FUN_00478000`
    /// via `FUN_004b7150`), or 0.
    Posture,
    /// Planner slot `+0x44` (`FUN_004047d0`).
    Fixed(usize),
}

const fn query(
    req_flags: u32,
    req_caps: u32,
    forbid_flags: u32,
    forbid_caps: u32,
    skill: Skill,
    min: u32,
) -> Query {
    Query {
        req_flags,
        req_caps,
        forbid_flags,
        forbid_caps,
        skill,
        min,
    }
}

/// Every decoy query takes an unassigned special force of the side with
/// espionage of 51 or more (`FUN_0047d720` and siblings).
const fn decoys(req_caps: u32, cap: DecoyCap) -> Option<(Query, DecoyCap)> {
    let q = query(0x1080_0002, req_caps, 0x0740_0051, 0, Skill::Espionage, 51);
    Some((q, cap))
}

/// The selectors of `kind`'s planner (`FUN_0042f830`).
///
/// port: the AI's subtype (side `+0x31c`) is untraced. Espionage takes the
/// class for subtypes other than 6 (the posture `FUN_004b7150` names no
/// subtype for it), and Abduction takes its default class.
fn selectors(kind: MissionKind) -> Option<Selectors> {
    use Skill::{
        Combat, Diplomacy, Espionage, FacilityDesign, Leadership, ShipDesign, TroopTraining,
    };
    const DIPLOMACY: &[Query] = &[query(0x2, 0x1, 0x0740_4051, 0, Diplomacy, 69)];
    const ESPIONAGE: &[Query] = &[query(0x1080_0002, 0x8, 0x0740_0051, 0, Espionage, 51)];
    const RECRUITMENT: &[Query] = &[query(0x2, 0x400, 0x0740_0051, 0, Leadership, 80)];
    const INCITE: &[Query] = &[query(0x2, 0x100, 0x4f40_0051, 0, Leadership, 49)];
    const SUBDUE: &[Query] = &[query(0x2, 0x20, 0x0740_0051, 0, Leadership, 49)];
    const RESCUE: &[Query] = &[query(0x2, 0x2, 0x4f40_0051, 0x700_0000, Combat, 51)];
    const ABDUCTION: &[Query] = &[query(0x2, 0x10, 0x0f40_0051, 0x700_0000, Combat, 51)];
    const ASSASSINATION: &[Query] = &[query(0x1080_0002, 0x40, 0x4740_0051, 0, Combat, 51)];
    const SABOTAGE: &[Query] = &[
        query(0x80_0002, 0x4, 0x4f40_0051, 0, Combat, 51),
        query(0x80_0002, 0x4, 0x4f40_0051, 0, Espionage, 51),
    ];
    const DS_SABOTAGE: &[Query] = &[
        query(0x2, 0x80, 0x4740_0051, 0, Combat, 51),
        query(0x2, 0x80, 0x4740_0051, 0, Espionage, 51),
    ];
    const SHIP_RESEARCH: &[Query] = &[query(0x2, 0x800, 0x0740_0051, 0, ShipDesign, 10)];
    const TROOP_RESEARCH: &[Query] = &[query(0x2, 0x1000, 0x0740_0051, 0, TroopTraining, 10)];
    const FACILITY_RESEARCH: &[Query] = &[query(0x2, 0x2000, 0x0740_0051, 0, FacilityDesign, 10)];
    let (team, team_size, decoys) = match kind {
        // FUN_004815a0; FUN_0047f500 takes no decoys.
        MissionKind::Diplomacy => (DIPLOMACY, 1, None),
        // FUN_004808c0, FUN_00480aa0.
        MissionKind::Espionage => (ESPIONAGE, 2, decoys(0x8, DecoyCap::Posture)),
        // FUN_0047db40; the class has no decoy selector.
        MissionKind::Recruitment => (RECRUITMENT, 1, None),
        // FUN_0047ff90, FUN_00480110.
        MissionKind::InciteUprising => (INCITE, 1, decoys(0x100, DecoyCap::Posture)),
        // FUN_0047cee0; no decoy selector.
        MissionKind::SubdueUprising => (SUBDUE, 1, None),
        // FUN_0047e270, FUN_0047e3f0 (cap slot +0x44 = 4).
        MissionKind::Rescue => (RESCUE, 2, decoys(0x2, DecoyCap::Fixed(4))),
        // FUN_00483290, FUN_00483410 (cap 4).
        MissionKind::Abduction => (ABDUCTION, 2, decoys(0x10, DecoyCap::Fixed(4))),
        // FUN_00482ce0, FUN_00482ec0 with +0x7c = 0x40.
        MissionKind::Assassination => (ASSASSINATION, 2, decoys(0x40, DecoyCap::Posture)),
        // FUN_0047d520, FUN_0047d720.
        MissionKind::Sabotage => (SABOTAGE, 2, decoys(0x4, DecoyCap::Posture)),
        // FUN_00482180, FUN_00482330 (cap 4).
        MissionKind::DeathStarSabotage => (DS_SABOTAGE, 2, decoys(0x80, DecoyCap::Fixed(4))),
        // port: the Recon planners (FUN_0047f790 for subtype 5, else
        // FUN_0047f320 and FUN_0047f500) are not ported, so the AI sends none.
        MissionKind::Autoscrap | MissionKind::Reconnaissance => return None,
        // The Move pseudo-mission has no selectors (FUN_004bd570 builds it).
        MissionKind::Move
        | MissionKind::JediTraining
        | MissionKind::Dagobah
        | MissionKind::Bounty
        | MissionKind::Palace
        | MissionKind::Pickup => return None,
        // FUN_0047bcd0, FUN_0047c2b0, FUN_0047c900: Recruitment's selector
        // shape, with no decoy selector.
        MissionKind::ShipDesignResearch => (SHIP_RESEARCH, 1, None),
        MissionKind::TroopTrainingResearch => (TROOP_RESEARCH, 1, None),
        MissionKind::FacilityDesignResearch => (FACILITY_RESEARCH, 1, None),
    };
    Some(Selectors {
        team,
        team_size,
        decoys,
    })
}

/// One side's planners for one AI evaluation.
pub(crate) struct Planner<'a> {
    records: Vec<AgentRecord>,
    /// The evaluation's world, which the records were built from.
    world: std::marker::PhantomData<&'a GameWorld>,
    rng: MissionRng,
}

impl<'a> Planner<'a> {
    /// The side's records as `FUN_00402230` would refresh them now.
    ///
    /// port: the tie draws come from a stream seeded by the day and side;
    /// the original draws from its global generator (`FUN_0041cd80`).
    pub(crate) fn new(
        world: &'a GameWorld,
        missions: &MissionState,
        busy: &HashSet<CharacterKey>,
        alliance: bool,
        tick: u64,
    ) -> Self {
        Self {
            records: records(world, missions, busy, alliance),
            world: std::marker::PhantomData,
            rng: MissionRng::seeded(0.0, tick.wrapping_mul(2) + u64::from(alliance)),
        }
    }

    /// The controller's research bits (`FUN_004198a0`, `FUN_00419870`,
    /// `FUN_004197b0`): the ship, troop and facility trees the side has
    /// finished. A record able only for finished trees is no longer
    /// reserved for research (`FUN_00402230`).
    pub(crate) fn research_done(&mut self, done: [bool; 3]) {
        for record in &mut self.records {
            record.reserve(done);
        }
    }

    /// A research planner holds `member` (`FUN_0047b610`: record `+0x24`,
    /// priority `+0x2c` = 0, `0x800000` cleared). No query of this
    /// evaluation takes it: `FUN_004035d0` admits an assigned record only
    /// above its threshold, which is 0 at every call site, and
    /// `FUN_00403460`'s queries require `0x800000`.
    pub(crate) fn hold(&mut self, member: MissionMember) {
        for record in &mut self.records {
            if record.member == member {
                record.sent = true;
                record.flags &= !UNASSIGNED;
            }
        }
    }

    /// `FUN_0047ba40`: a planner keeps its member while the record exists,
    /// is the side's, lacks `0x7400051`, and passes `FUN_0047ba90` (not
    /// standing at a blockaded system). The research classes' slot `+0x3c`
    /// is the shared `FUN_0051ebb0`. port: the record's owner (`+0x24`) is
    /// the planner by construction.
    pub(crate) fn keeps(&self, member: MissionMember) -> bool {
        self.records.iter().any(|r| {
            r.member == member
                && r.flags & OWN_SIDE != 0
                && r.flags & 0x0740_0051 == 0
                && !r.at_blockade
        })
    }

    /// `FUN_0041cd80(10) < 5`: a coin from the planner's draws, for the
    /// ranked lists of the modules.
    pub(crate) fn coin(&mut self) -> bool {
        self.rng.draw(9) < 5
    }

    /// `FUN_0041cd80(n)`: a draw from 0 to `n - 1` from the planner's
    /// draws.
    pub(crate) fn random(&mut self, n: i32) -> i32 {
        self.rng.draw(n - 1)
    }

    /// The flags (`+0x30`) of `member`'s record, which the fleet records
    /// read for the people aboard (`FUN_00484630`).
    pub(crate) fn flags(&self, member: MissionMember) -> Option<u32> {
        self.records
            .iter()
            .find(|r| r.member == member)
            .map(|r| r.flags)
    }

    /// Whether `FUN_0047b360` would send `member`: its record has none of
    /// `0x74000f1` (on a mission, en route, ...).
    pub(crate) fn sendable(&self, member: MissionMember) -> bool {
        self.records
            .iter()
            .any(|r| r.member == member && r.flags & NOT_SENT == 0)
    }

    /// A research planner's team selector (`FUN_0047bcd0` and siblings):
    /// its best candidate, which the planner then holds.
    pub(crate) fn pick(&mut self, kind: MissionKind) -> Option<MissionMember> {
        let selectors = selectors(kind)?;
        let index = *self.select(selectors.team, 1).first()?;
        let member = self.records[index].member;
        self.hold(member);
        Some(member)
    }

    /// `FUN_004357b0` over one sorted list (`FUN_0041be80` with `order`,
    /// `FUN_005f59f0`, `FUN_0041c250`): the last entry wins, so the highest
    /// key for order 1 and the lowest for order 2; equal keys are ordered by
    /// coin flips as they are inserted.
    pub(crate) fn pick_ranked<T: Copy>(
        &mut self,
        items: impl IntoIterator<Item = (T, i64)>,
        ascending: bool,
    ) -> Option<T> {
        let mut list: Vec<(T, i64)> = Vec::new();
        for (item, key) in items {
            let at = list
                .iter()
                .position(|&(_, other)| match key.cmp(&other) {
                    std::cmp::Ordering::Less => ascending,
                    std::cmp::Ordering::Greater => !ascending,
                    std::cmp::Ordering::Equal => self.rng.draw(9) < 5,
                })
                .unwrap_or(list.len());
            list.insert(at, (item, key));
        }
        list.last().map(|&(item, _)| item)
    }

    /// `FUN_0041bc10`: the side's characters (record flags `0x2` and
    /// `0x20000000`) with `kind`'s capability bit, which the strategy wants
    /// as many planners of a research tree as (side `+0x2cc`, `+0x2d0`,
    /// `+0x2c8`).
    pub(crate) fn able(&self, kind: MissionKind) -> usize {
        let Some(bit) = capability(kind) else {
            return 0;
        };
        self.records
            .iter()
            .filter(|r| r.flags & (OWN_SIDE | CHARACTER) == OWN_SIDE | CHARACTER)
            .filter(|r| r.caps & bit != 0)
            .count()
    }

    /// Fill `kind`'s team, then its decoys, as its planner's selectors do,
    /// and keep the members the order takes (`FUN_0047b360`). `None` when
    /// no team member is sent.
    ///
    /// port: a member taken by an earlier plan in this evaluation stays
    /// taken; the original planners run on separate ticks, by which time
    /// the first order has set its members on the mission.
    pub(crate) fn plan(
        &mut self,
        kind: MissionKind,
    ) -> Option<(Vec<MissionMember>, Vec<MissionMember>)> {
        let selectors = selectors(kind)?;
        let team = self.select(selectors.team, selectors.team_size);
        // port: a plan that sends no one keeps no one. The original planner
        // holds its picks (they lost 0x800000) across ticks until it emits.
        if !team
            .iter()
            .any(|&index| self.records[index].flags & NOT_SENT == 0)
        {
            for index in team {
                self.records[index].flags |= UNASSIGNED;
            }
            return None;
        }
        let decoys = match selectors.decoys {
            // hyp: the posture that names +0xbc is not ported; a system
            // with no recommended kind reads 0, which allows one decoy.
            Some((decoy, DecoyCap::Posture)) => self.select(&[decoy], 1),
            Some((decoy, DecoyCap::Fixed(cap))) => self.select(&[decoy], cap),
            None => Vec::new(),
        };
        let sent = |picked: Vec<usize>, records: &[AgentRecord]| -> Vec<MissionMember> {
            picked
                .into_iter()
                .filter(|&index| records[index].flags & NOT_SENT == 0)
                .map(|index| records[index].member)
                .collect()
        };
        // The planner then holds them; the AI's mission planners move them
        // to the target (state 7, FUN_004bd570) and order the mission once
        // all are there (AISystem::mission_planner_cycle).
        let team = sent(team, &self.records);
        let decoys = sent(decoys, &self.records);
        for record in &mut self.records {
            if team.contains(&record.member) || decoys.contains(&record.member) {
                record.sent = true;
            }
        }
        Some((team, decoys))
    }

    /// Run `queries` and take their best common hit until `size` are taken
    /// (`FUN_004357b0` in the selector loop). A taken record loses
    /// `UNASSIGNED`.
    fn select(&mut self, queries: &[Query], size: usize) -> Vec<usize> {
        let mut lists: Vec<Vec<usize>> = queries
            .iter()
            .map(|q| ranked(&self.records, *q, &mut self.rng))
            .collect();
        let mut taken = Vec::new();
        while taken.len() < size {
            let Some(best) = pop_best(&mut lists) else {
                break;
            };
            // FUN_0047ba90 rejects a candidate standing at a blockaded
            // system; the selector loop takes the next.
            if self.records[best].at_blockade {
                continue;
            }
            self.records[best].flags &= !UNASSIGNED;
            taken.push(best);
        }
        taken
    }
}

/// `FUN_00403460`: the records that pass `q`, in the order of the sorted
/// container. Order flag 1 inserts each hit before the first entry with a
/// higher skill; an equal skill goes before it on a coin flip
/// (`FUN_0041c250`, `FUN_0041cd80(10) < 5`).
fn ranked(records: &[AgentRecord], q: Query, rng: &mut MissionRng) -> Vec<usize> {
    let mut list: Vec<usize> = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let skill = record.skills[q.skill as usize];
        let hit = !record.sent
            && record.flags & q.req_flags == q.req_flags
            && record.flags & q.forbid_flags == 0
            && record.caps & q.req_caps == q.req_caps
            && record.caps & q.forbid_caps == 0
            && (q.min..=10_000).contains(&skill);
        if !hit {
            continue;
        }
        // FUN_005f59f0 walks from the head and inserts before the first
        // entry the comparator puts the new one ahead of.
        let at = list
            .iter()
            .position(|&other| {
                let theirs = records[other].skills[q.skill as usize];
                match skill.cmp(&theirs) {
                    std::cmp::Ordering::Less => true,
                    std::cmp::Ordering::Greater => false,
                    std::cmp::Ordering::Equal => rng.draw(9) < 5,
                }
            })
            .unwrap_or(list.len());
        list.insert(at, index);
    }
    list
}

/// `FUN_004357b0`: each entry of the first list scores the sum of its
/// positions in every list; an entry missing from any list leaves the first.
/// The highest score wins, the earliest on a tie, and leaves the first list.
///
/// `FUN_0041c230` numbers positions from 1. Every remaining entry is in
/// every list, so counting from 0 shifts all scores alike.
fn pop_best(lists: &mut [Vec<usize>]) -> Option<usize> {
    let (first, rest) = lists.split_first_mut()?;
    let position = |list: &[usize], index: usize| list.iter().position(|&x| x == index);
    first.retain(|&index| rest.iter().all(|list| list.contains(&index)));
    let mut best: Option<(usize, usize)> = None;
    for (at, &index) in first.iter().enumerate() {
        let score = at
            + rest
                .iter()
                .filter_map(|list| position(list, index))
                .sum::<usize>();
        if best.is_none_or(|(_, top)| top < score) {
            best = Some((index, score));
        }
    }
    let (index, _) = best?;
    first.retain(|&x| x != index);
    Some(index)
}

/// The records of every character and special force for the side
/// (`FUN_00401d20` then `FUN_00402230`).
///
/// port: the pool's order is the original's DatId order for characters
/// (majors, then minors, by id); special forces, which the port gives no
/// DatId, follow in arena order.
fn records(
    world: &GameWorld,
    missions: &MissionState,
    busy: &HashSet<CharacterKey>,
    alliance: bool,
) -> Vec<AgentRecord> {
    let mut characters: Vec<_> = world
        .characters
        .iter()
        .filter(|(_, c)| !c.is_killed)
        .collect();
    characters.sort_by_key(|(_, c)| (!c.is_major, c.dat_id.raw()));
    let mut records = Vec::new();
    for (key, c) in characters {
        let member = MissionMember::Character(key);
        let id = c.dat_id.raw();
        let mut flags = UNASSIGNED | CHARACTER;
        // FUN_00401d20: a prisoner counts on the side that holds it.
        flags |= if (c.is_alliance == alliance) != c.is_captive {
            OWN_SIDE
        } else {
            OTHER_SIDE
        };
        let mut caps = 0;
        if c.is_major && MAIN_CHARACTERS.contains(&id) {
            flags |= MAIN_CHARACTER;
        } else {
            // FUN_00401d20: capability bits 0x1000000..0x4000000 mark a
            // minor character that may be an admiral, commander, or general
            // (class record +0xa8, +0xac, +0xb0; FUN_004ed1c0..FUN_004ed200).
            for (can, bit) in [
                (c.can_be_admiral, 0x100_0000),
                (c.can_be_commander, 0x200_0000),
                (c.can_be_general, 0x400_0000),
            ] {
                if can {
                    caps |= bit;
                }
            }
        }
        // FUN_00401d20: the leader bit is set while the side controller's
        // +4 bit 0 is clear. hyp: that bit's only writer (FUN_004182a0, a
        // game setting read through FUN_004fce50) is untraced; it is read
        // as clear, its constructor value.
        if c.is_major && LEADERS.contains(&id) {
            flags |= LEADER;
        }
        if c.is_captive {
            flags |= PRISONER;
        }
        if !c.recruited {
            flags |= INACTIVE;
        }
        // port: the AI's busy set holds characters its applied actions sent
        // on missions or research.
        if c.on_mission || c.on_hidden_mission || busy.contains(&key) {
            flags |= ON_MISSION;
        }
        if c.on_mandatory_mission {
            flags |= MANDATORY;
        }
        if missions.is_en_route(member) {
            flags |= EN_ROUTE;
        }
        // hyp: the port has no officer rank (+0x96); a character in a fleet
        // holds one.
        if c.current_fleet.is_some() {
            flags |= OFFICER_RANK_1;
        }
        records.push(agent_record(world, member, flags, caps));
    }
    for (key, unit) in &world.special_forces {
        let member = MissionMember::SpecialForce(key);
        let mut flags = UNASSIGNED | SPECIAL_FORCE;
        flags |= if unit.is_alliance == alliance {
            OWN_SIDE
        } else {
            OTHER_SIDE
        };
        if unit.on_mission {
            flags |= ON_MISSION;
        }
        if missions.is_en_route(member) {
            flags |= EN_ROUTE;
        }
        records.push(agent_record(world, member, flags, 0));
    }
    for record in &mut records {
        record.at_blockade = record.flags & EN_ROUTE == 0
            && standing_system(world, record.member).is_some_and(|system| {
                crate::blockade::BlockadeSystem::system_is_blockaded(world, &world.systems[system])
            });
    }
    records
}

/// The system an object stands at, when its location (`+0x38`) is a
/// system rather than a fleet.
pub(crate) fn standing_system(world: &GameWorld, member: MissionMember) -> Option<SystemKey> {
    match member {
        MissionMember::Character(key) => {
            let c = world.characters.get(key)?;
            if c.current_fleet.is_some() {
                None
            } else {
                c.current_system.filter(|&s| world.systems.contains_key(s))
            }
        }
        MissionMember::SpecialForce(key) => world
            .systems
            .iter()
            .find(|(_, sys)| sys.special_forces.contains(&key))
            .map(|(system, _)| system),
    }
}

/// Fill in the skills (slots `+0x1dc..+0x1f8`), the mission capabilities
/// (`FUN_00402720`), and the reserved bit.
fn agent_record(
    world: &GameWorld,
    member: MissionMember,
    flags: u32,
    officer_caps: u32,
) -> AgentRecord {
    let mut skills = [0; 8];
    for skill in Skill::ALL {
        skills[skill as usize] = member_skill(world, member, skill).unwrap_or(0);
    }
    let mut caps = officer_caps;
    for kind in KINDS {
        let Some(bit) = capability(kind) else {
            continue;
        };
        // port: a kind with no MISSNSD record admits any members, as at
        // creation. FUN_0054c440 also runs the class's creation check, so a
        // character with no skill in a research tree lacks its bit
        // (FUN_0056c820).
        let legal = kind
            .record_id()
            .and_then(|id| world.mission_record(id))
            .is_none_or(|record| members_allowed(world, record.members, [member]))
            && crate::missions::creation_member_ok(world, kind, member);
        if legal {
            caps |= bit;
        }
    }
    // FUN_00402230: a Diplomacy-capable object with diplomacy above 79 is
    // reserved, and so is one able for a research tree the side has not
    // finished ([`Planner::research_done`]); until told, no tree is done.
    let diplomat = caps & 0x1 != 0 && skills[Skill::Diplomacy as usize] > 79;
    let mut record = AgentRecord {
        member,
        flags,
        caps,
        skills,
        sent: false,
        diplomat,
        at_blockade: false,
    };
    record.reserve([false; 3]);
    record
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ranged_draw_covers_zero_to_one_below_its_bound() {
        // FUN_0041cd80(n): 0 to n - 1.
        let world = GameWorld::default();
        let mut planner = Planner::new(&world, &MissionState::new(), &HashSet::new(), true, 3);
        let draws: Vec<i32> = (0..400).map(|_| planner.random(20)).collect();
        assert!(draws.iter().all(|d| (0..20).contains(d)));
        assert_eq!(draws.iter().max(), Some(&19));
        assert_eq!(draws.iter().min(), Some(&0));
    }

    use crate::world::{Character, MissionMemberRules, MissionRecord, SkillPair, SpecialForceUnit};

    fn pair(base: u32) -> SkillPair {
        SkillPair { base, variance: 0 }
    }

    /// A world with no MISSNSD records: every member is legal, and nobody is
    /// reserved for research.
    fn world() -> GameWorld {
        GameWorld::default()
    }

    /// The shipped Research records: characters of both sides, no special
    /// forces.
    fn research_records() -> Vec<MissionRecord> {
        RESEARCH_RECORDS
            .iter()
            .map(|&raw| MissionRecord {
                dat_id: crate::ids::DatId::new(raw),
                timer_min_days: 10,
                timer_spread_days: 10,
                repeats: true,
                hidden: false,
                detection_phases: false,
                can_resign: false,
                rules: crate::world::MissionTargetRules::default(),
                members: MissionMemberRules {
                    alliance: true,
                    empire: true,
                    special_force_mask: 0,
                    character_mask: 0x1_0000,
                },
            })
            .collect()
    }

    /// A recruited Alliance minor character.
    fn agent(
        world: &mut GameWorld,
        dat_id: u32,
        edit: impl FnOnce(&mut Character),
    ) -> MissionMember {
        let mut c = Character {
            dat_id: crate::ids::DatId::new(dat_id),
            name: "Agent".into(),
            is_alliance: true,
            recruited: true,
            ..Default::default()
        };
        edit(&mut c);
        MissionMember::Character(world.characters.insert(c))
    }

    fn unit(world: &mut GameWorld, espionage: u32, combat: u32) -> MissionMember {
        let mut skills = [0; 8];
        skills[Skill::Espionage as usize] = espionage;
        skills[Skill::Combat as usize] = combat;
        MissionMember::SpecialForce(world.special_forces.insert(SpecialForceUnit {
            class_dat_id: crate::ids::DatId::new(0x3c00_0002),
            is_alliance: true,
            skills,
            on_mission: false,
        }))
    }

    fn planner(world: &GameWorld) -> Planner<'_> {
        Planner::new(world, &MissionState::new(), &HashSet::new(), true, 1)
    }

    #[test]
    fn the_planner_s_coin_is_a_draw_below_five_of_ten() {
        // FUN_0041cd80(10) < 5, from the planner's generator (seeded by the
        // tick and side): heads and tails both come up, in its order.
        let world = GameWorld::default();
        let mut p = planner(&world);
        let mut rng = MissionRng::seeded(0.0, 2 + 1);
        let coins: Vec<bool> = (0..40).map(|_| p.coin()).collect();
        let draws: Vec<bool> = (0..40).map(|_| rng.draw(9) < 5).collect();
        assert_eq!(coins, draws);
        assert!(coins.contains(&true) && coins.contains(&false));
    }

    #[test]
    fn a_planner_keeps_only_a_free_member_of_its_side_away_from_a_blockade() {
        // FUN_0047ba40: the record must lack 0x7400051 (here 0x1, a
        // character the side has not recruited) and pass FUN_0047ba90;
        // a record on the other side is not the side's to keep.
        let mut world = world();
        let own = agent(&mut world, 1, |_| {});
        let enemy = agent(&mut world, 2, |c| {
            c.is_alliance = false;
            c.is_empire = true;
        });
        let unrecruited = agent(&mut world, 3, |c| c.recruited = false);
        let mut planner = planner(&world);
        assert!(planner.keeps(own));
        assert!(!planner.keeps(enemy));
        assert!(!planner.keeps(unrecruited));
        planner.records[0].at_blockade = true;
        assert!(!planner.keeps(own));
    }

    #[test]
    fn a_candidate_standing_at_a_blockaded_system_is_passed_over() {
        // FUN_0047ba90 rejects a candidate whose location is a system whose
        // record has +0x28 bit 0x2, the blockade (FUN_00433e40, system
        // +0x88 bit 0x20); the selector loop takes the next.
        use crate::world::{ControlKind, Fleet, System};
        let mut world = world();
        let system = |world: &mut GameWorld| {
            world.systems.insert(System {
                dat_id: crate::ids::DatId::new(0x9000_0001),
                name: "System".into(),
                sector: crate::ids::SectorKey::default(),
                x: 0,
                y: 0,
                exploration_status: crate::dat::ExplorationStatus::Explored,
                popularity_alliance: 0.5,
                popularity_empire: 0.5,
                is_populated: true,
                total_energy: 0,
                raw_materials: 0,
                espionage_rating: 0.0,
                fleets: vec![],
                ground_units: vec![],
                special_forces: vec![],
                defense_facilities: vec![],
                manufacturing_facilities: vec![],
                production_facilities: vec![],
                fighters: Vec::new(),
                is_headquarters: false,
                is_destroyed: false,
                control: ControlKind::Controlled(crate::dat::Faction::Alliance),
            })
        };
        let blockaded = system(&mut world);
        let free = system(&mut world);
        let fleet = world.fleets.insert(Fleet {
            location: blockaded,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[blockaded].fleets.push(fleet);
        let best = agent(&mut world, 1, |c| {
            c.diplomacy = pair(90);
            c.current_system = Some(blockaded);
        });
        let next = agent(&mut world, 2, |c| {
            c.diplomacy = pair(80);
            c.current_system = Some(free);
        });
        let _ = best;

        assert_eq!(
            planner(&world)
                .plan(MissionKind::Diplomacy)
                .map(|(team, _)| team),
            Some(vec![next])
        );
    }

    #[test]
    fn a_diplomat_is_the_one_with_the_highest_diplomacy() {
        // FUN_00403460 sorts ascending (order 1, FUN_0041c250) and
        // FUN_004357b0 takes the highest position.
        let mut world = world();
        let _ = agent(&mut world, 1, |c| c.diplomacy = pair(70));
        let best = agent(&mut world, 2, |c| c.diplomacy = pair(79));
        let _ = agent(&mut world, 3, |c| c.diplomacy = pair(75));

        let plan = planner(&world).plan(MissionKind::Diplomacy);

        assert_eq!(plan, Some((vec![best], vec![])));
    }

    #[test]
    fn a_candidate_missing_from_any_query_is_dropped_and_positions_add_up() {
        // FUN_004357b0 sums each first-list entry's positions in every list
        // and drops entries some list lacks. Combat ranks b 1, c 2, a 3 and
        // espionage a 1, c 2, b 3, so all three score 4; the earliest in the
        // combat list, b, wins, then c. d lacks espionage 51.
        let mut world = world();
        let a = unit(&mut world, 60, 90);
        let b = unit(&mut world, 90, 60);
        let c = unit(&mut world, 80, 80);
        let _d = unit(&mut world, 50, 99);

        let (team, _) = planner(&world).plan(MissionKind::Sabotage).unwrap();

        assert_eq!(team, vec![b, c]);
        assert!(!team.contains(&a));
    }

    #[test]
    fn sabotage_decoys_are_special_forces_left_after_the_team() {
        // FUN_0047d720 queries 0x10800002: special forces still unassigned
        // (a team member lost 0x800000 in FUN_0047b610). hyp: one decoy,
        // the cap for a system whose posture recommends nothing.
        let mut world = world();
        let first = unit(&mut world, 90, 90);
        let second = unit(&mut world, 80, 80);
        let third = unit(&mut world, 70, 70);
        let _ = unit(&mut world, 60, 60);

        let plan = planner(&world).plan(MissionKind::Sabotage);

        assert_eq!(plan, Some((vec![first, second], vec![third])));
    }

    #[test]
    fn a_rescue_takes_up_to_four_decoys() {
        // FUN_0047e3f0 caps the decoys at slot +0x44, FUN_004047d0 = 4.
        let mut world = world();
        let units: Vec<_> = (0..7).map(|i| unit(&mut world, 90 - i, 90 - i)).collect();

        let (team, decoys) = planner(&world).plan(MissionKind::Rescue).unwrap();

        assert_eq!(team, units[..2].to_vec());
        assert_eq!(decoys, units[2..6].to_vec());
    }

    #[test]
    fn a_best_candidate_already_on_a_mission_is_picked_but_not_sent() {
        // The Diplomacy query does not forbid 0x80, but FUN_0047b360 sends
        // no member with any of 0x74000f1, so the order has no team.
        let mut world = world();
        let _ = agent(&mut world, 1, |c| {
            c.diplomacy = pair(90);
            c.on_mission = true;
        });
        let _ = agent(&mut world, 2, |c| c.diplomacy = pair(70));

        assert_eq!(planner(&world).plan(MissionKind::Diplomacy), None);
    }

    #[test]
    fn a_character_the_side_has_not_recruited_is_never_picked() {
        // FUN_00402230 sets 0x1 while +0x50 bit 2 is clear; every query
        // forbids it.
        let mut world = world();
        let _ = agent(&mut world, 1, |c| {
            c.diplomacy = pair(90);
            c.recruited = false;
        });

        assert_eq!(planner(&world).plan(MissionKind::Diplomacy), None);
    }

    #[test]
    fn a_minor_character_fit_to_command_is_kept_off_rescues() {
        // FUN_0047e270 forbids capability bits 0x7000000, which
        // FUN_00401d20 sets for a minor character that may be an admiral,
        // commander, or general.
        let mut world = world();
        let _ = agent(&mut world, 0x300, |c| {
            c.combat = pair(90);
            c.can_be_general = true;
        });
        let fighter = agent(&mut world, 0x301, |c| c.combat = pair(60));

        let plan = planner(&world).plan(MissionKind::Rescue);

        assert_eq!(plan, Some((vec![fighter], vec![])));
    }

    #[test]
    fn a_character_able_for_an_unfinished_research_tree_is_reserved_from_sabotage() {
        // FUN_00402230 (lines 72-93): an object with a research capability
        // (+0x34 bit 0x800, 0x1000, 0x2000) is reserved while the
        // controller's bit for that tree is clear; FUN_004198a0 and its
        // siblings set it once the tree is finished.
        let mut world = world();
        let _ = agent(&mut world, 1, |c| {
            c.combat = pair(90);
            c.espionage = pair(90);
            c.ship_design = pair(10);
        });
        world.mission_records = research_records();
        assert_eq!(planner(&world).plan(MissionKind::Sabotage), None);

        // Another tree done keeps it; its own tree done frees it.
        let mut p = planner(&world);
        p.research_done([false, true, true]);
        assert_eq!(p.plan(MissionKind::Sabotage), None);
        let mut p = planner(&world);
        p.research_done([true, false, false]);
        assert!(p.plan(MissionKind::Sabotage).is_some());
    }

    #[test]
    fn a_character_with_no_research_skill_is_not_reserved_for_research() {
        // FUN_00402720 builds the capability through FUN_0054c440, whose
        // research check needs a nonzero skill in the tree (FUN_0056c820),
        // so such a character has no research bit to reserve it.
        let mut world = world();
        let _ = agent(&mut world, 1, |c| {
            c.combat = pair(90);
            c.espionage = pair(90);
        });
        world.mission_records = research_records();
        assert!(planner(&world).plan(MissionKind::Sabotage).is_some());
    }

    #[test]
    fn a_plan_keeps_both_picks_wherever_they_stand() {
        // FUN_0047e270 picks a team of two; the planner holds both and
        // gathers them at the target (state 7, FUN_004bd570) before its
        // order, so a pick standing elsewhere stays on the team.
        let mut world = world();
        let mut places: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let (here, there) = (places.insert(()), places.insert(()));
        let first = agent(&mut world, 1, |c| {
            c.combat = pair(90);
            c.current_system = Some(here);
        });
        let away = agent(&mut world, 2, |c| {
            c.combat = pair(80);
            c.current_system = Some(there);
        });
        let _third = agent(&mut world, 3, |c| {
            c.combat = pair(70);
            c.current_system = Some(here);
        });

        let plan = planner(&world).plan(MissionKind::Rescue);

        assert_eq!(plan, Some((vec![first, away], vec![])));
    }

    #[test]
    fn a_record_s_flags_are_found_by_its_member() {
        // FUN_00401d20: an own unrecruited character carries 0x2 and 0x1.
        let mut world = world();
        let key = agent(&mut world, 1, |c| c.recruited = false);
        let planner = planner(&world);

        let flags = planner.flags(key).unwrap();

        assert_eq!(flags & (OWN_SIDE | INACTIVE), OWN_SIDE | INACTIVE);
        assert_eq!(
            planner.flags(MissionMember::Character(CharacterKey::default())),
            None
        );
    }

    #[test]
    fn a_busy_best_candidate_still_takes_a_team_slot() {
        // No query forbids 0x80, so the Recruitment planner picks the busy
        // leader and FUN_0047b360 drops him; the failed plan frees him, and
        // the DS Sabotage planner picks him again before the next best.
        let mut world = world();
        let skilled = |c: &mut Character| {
            c.combat = pair(95);
            c.espionage = pair(95);
            c.leadership = pair(90);
            c.on_mission = true;
        };
        let _busy = agent(&mut world, 1, skilled);
        let second = agent(&mut world, 2, |c| {
            c.combat = pair(80);
            c.espionage = pair(80);
        });
        let _third = agent(&mut world, 3, |c| {
            c.combat = pair(70);
            c.espionage = pair(70);
        });
        let mut planner = planner(&world);

        assert_eq!(planner.plan(MissionKind::Recruitment), None);
        assert_eq!(
            planner.plan(MissionKind::DeathStarSabotage),
            Some((vec![second], vec![]))
        );
    }

    #[test]
    fn a_character_en_route_takes_a_team_slot_but_is_not_sent() {
        // 0x20 (FUN_00402230) is in no query's forbid mask, only in
        // FUN_0047b360's.
        let mut world = world();
        let travelling = agent(&mut world, 1, |c| {
            c.combat = pair(95);
            c.espionage = pair(95);
        });
        let second = agent(&mut world, 2, |c| {
            c.combat = pair(80);
            c.espionage = pair(80);
        });
        let _third = agent(&mut world, 3, |c| {
            c.combat = pair(70);
            c.espionage = pair(70);
        });
        let mut missions = MissionState::new();
        missions.push_transit(crate::missions::MemberTransit {
            member: travelling,
            mission_id: 1,
            to: SystemKey::default(),
            arrival: 9,
            abode: None,
        });
        let mut planner = Planner::new(&world, &missions, &HashSet::new(), true, 1);

        assert_eq!(
            planner.plan(MissionKind::DeathStarSabotage),
            Some((vec![second], vec![]))
        );
    }

    #[test]
    fn a_busy_or_travelling_special_force_takes_a_slot_but_is_not_sent() {
        let mut world = world();
        let busy = unit(&mut world, 95, 95);
        if let MissionMember::SpecialForce(key) = busy {
            world.special_forces[key].on_mission = true;
        }
        let second = unit(&mut world, 80, 80);
        let third = unit(&mut world, 70, 70);

        assert_eq!(
            planner(&world).plan(MissionKind::Sabotage),
            Some((vec![second], vec![third]))
        );

        if let MissionMember::SpecialForce(key) = busy {
            world.special_forces[key].on_mission = false;
        }
        let mut missions = MissionState::new();
        missions.push_transit(crate::missions::MemberTransit {
            member: busy,
            mission_id: 1,
            to: SystemKey::default(),
            arrival: 9,
            abode: None,
        });
        let mut planner = Planner::new(&world, &missions, &HashSet::new(), true, 1);

        assert_eq!(
            planner.plan(MissionKind::Sabotage),
            Some((vec![second], vec![third]))
        );
    }

    #[test]
    fn equal_skills_are_ordered_by_a_coin_flip() {
        // FUN_0041c250: an equal entry goes ahead on FUN_0041cd80(10) < 5,
        // half the time. Over 1000 seeded evaluations the first-listed of
        // two equal diplomats is picked about 500 times.
        let mut world = world();
        let first = agent(&mut world, 1, |c| c.diplomacy = pair(80));
        let _ = agent(&mut world, 2, |c| c.diplomacy = pair(80));

        let picked = (0..1000)
            .filter(|&tick| {
                let mut planner =
                    Planner::new(&world, &MissionState::new(), &HashSet::new(), true, tick);
                planner.plan(MissionKind::Diplomacy) == Some((vec![first], vec![]))
            })
            .count();

        assert!((450..=550).contains(&picked), "picked {picked} of 1000");
    }

    #[test]
    fn positions_are_added_not_multiplied() {
        // FUN_00435980 adds positions. Combat ranks a, b, c, d and espionage
        // d, c, b, a give every candidate the same sum, so the earliest in
        // the combat list, a, wins; a product would pick b.
        let mut world = world();
        let a = unit(&mut world, 90, 60);
        let _b = unit(&mut world, 80, 70);
        let _c = unit(&mut world, 70, 80);
        let _d = unit(&mut world, 60, 90);

        let (team, _) = planner(&world).plan(MissionKind::Sabotage).unwrap();

        assert_eq!(team[0], a);
    }

    #[test]
    fn a_major_other_than_the_six_mains_fit_to_command_is_kept_off_rescues() {
        // FUN_00401d20 gives officer capability bits to any character but
        // the six mains (FUN_004025f0); FUN_0047e270 forbids them.
        let mut world = world();
        let _general = agent(&mut world, 0x2c0, |c| {
            c.is_major = true;
            c.combat = pair(90);
            c.can_be_general = true;
        });
        let main = agent(&mut world, 0x241, |c| {
            c.is_major = true;
            c.combat = pair(80);
            c.can_be_general = true;
        });

        assert_eq!(
            planner(&world).plan(MissionKind::Rescue),
            Some((vec![main], vec![]))
        );
    }

    #[test]
    fn a_main_character_who_is_not_a_leader_may_sabotage() {
        // 0x31000241 is main (FUN_004025f0) but not a leader (FUN_004024d0).
        let mut world = world();
        let main = agent(&mut world, 0x241, |c| {
            c.is_major = true;
            c.combat = pair(90);
            c.espionage = pair(90);
        });

        assert_eq!(
            planner(&world).plan(MissionKind::Sabotage),
            Some((vec![main], vec![]))
        );
    }

    /// The shipped Diplomacy record: characters of both sides only.
    fn diplomacy_record() -> MissionRecord {
        MissionRecord {
            dat_id: crate::ids::DatId::new(0x5100_0010),
            timer_min_days: 5,
            timer_spread_days: 10,
            repeats: true,
            hidden: false,
            detection_phases: true,
            can_resign: true,
            rules: crate::world::MissionTargetRules::default(),
            members: MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0,
                character_mask: 0x1_0000,
            },
        }
    }

    #[test]
    fn a_diplomat_of_80_is_reserved_from_sabotage() {
        // FUN_00402230 reserves a Diplomacy-capable object with diplomacy
        // above 79, and FUN_0047d520 forbids reserved objects.
        let saboteur = |diplomacy| {
            move |c: &mut Character| {
                c.diplomacy = pair(diplomacy);
                c.combat = pair(90);
                c.espionage = pair(90);
            }
        };
        let mut world = world();
        world.mission_records = vec![diplomacy_record()];
        let _reserved = agent(&mut world, 1, saboteur(80));
        assert_eq!(planner(&world).plan(MissionKind::Sabotage), None);

        let free = agent(&mut world, 2, saboteur(79));
        assert_eq!(
            planner(&world).plan(MissionKind::Sabotage),
            Some((vec![free], vec![]))
        );
    }

    #[test]
    fn a_special_force_the_diplomacy_record_refuses_is_never_reserved_for_diplomacy() {
        // The shipped Diplomacy record takes no special force (+0x48 = 0),
        // so a SPECFCSD class of mask 0x2 is not Diplomacy-capable, whatever
        // its diplomacy.
        let mut world = world();
        world.mission_records = vec![diplomacy_record()];
        world.special_force_classes.insert(
            crate::ids::DatId::new(0x3c00_0002),
            crate::world::SpecialForceClassDef {
                skills: [pair(0); 8],
                mission_mask: 0x2,
            },
        );
        let commando = unit(&mut world, 90, 90);
        if let MissionMember::SpecialForce(key) = commando {
            world.special_forces[key].skills[Skill::Diplomacy as usize] = 90;
        }

        assert_eq!(
            planner(&world).plan(MissionKind::Sabotage),
            Some((vec![commando], vec![]))
        );
    }

    #[test]
    fn a_plan_with_no_team_takes_no_decoys() {
        // port: an Incite planner with no leader of 49 sends nothing, so its
        // would-be decoy is still free for the Sabotage team.
        let mut world = world();
        let commando = unit(&mut world, 80, 80);
        let mut planner = planner(&world);

        assert_eq!(planner.plan(MissionKind::InciteUprising), None);
        assert_eq!(
            planner.plan(MissionKind::Sabotage),
            Some((vec![commando], vec![]))
        );
    }

    #[test]
    fn a_member_one_plan_sends_is_not_sent_by_the_next() {
        // port: the original's second planner would run after the first
        // order set its member on the mission.
        let mut world = world();
        let first = agent(&mut world, 1, |c| c.diplomacy = pair(90));
        let second = agent(&mut world, 2, |c| c.diplomacy = pair(80));
        let mut planner = planner(&world);

        assert_eq!(
            planner.plan(MissionKind::Diplomacy),
            Some((vec![first], vec![]))
        );
        assert_eq!(
            planner.plan(MissionKind::Diplomacy),
            Some((vec![second], vec![]))
        );
        assert_eq!(planner.plan(MissionKind::Diplomacy), None);
    }

    #[test]
    fn a_leader_is_kept_off_sabotage_but_may_negotiate() {
        // FUN_00401d20 sets 0x40000000 for the four leaders
        // (FUN_004024d0) while the side's +4 bit 0 is clear; FUN_0047d520
        // forbids it and FUN_004815a0 does not.
        let mut world = world();
        let leader = agent(&mut world, 0x240, |c| {
            c.is_major = true;
            c.diplomacy = pair(90);
            c.combat = pair(90);
            c.espionage = pair(90);
        });
        let mut planner = planner(&world);

        assert_eq!(planner.plan(MissionKind::Sabotage), None);
        assert_eq!(
            planner.plan(MissionKind::Diplomacy),
            Some((vec![leader], vec![]))
        );
    }
}
