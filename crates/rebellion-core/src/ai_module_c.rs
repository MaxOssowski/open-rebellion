//! Module `0xc` of the AI player (`FUN_004c8260`, vtable `0x65c6b8`, step
//! `FUN_004c84a0` -> `FUN_004c84d0`): the movement planners, which post the
//! side's officers (`ghidra/notes/ai-command-planners.md`).
//!
//! Each planner holds one character and moves it to a target: types 1 and
//! 6 are the strategy's (slot `+0xc` returns 2), asked for by module
//! `0xd`'s state 4 (`FUN_004e57e0`, `FUN_004eaaa0`); types 3, 4 and 5 are a
//! sector planner's (slot `+0xc` returns 1), asked for through its `+0xe0`.
//! Type 1 (`FUN_0047a430`, vtable `0x65a490`) posts a General at a guarded
//! system (record `+0x30` bit `0x10000000`) and gives him its command
//! (`0x263`); type 6 (`FUN_00479a60`, vtable `0x65a3c0`) keeps the Emperor
//! at Coruscant.
//!
//! Types 3, 4 and 5 (`FUN_004bc290`, vtable `0x65c380`: `FUN_0047a3c0`
//! Admiral, `FUN_0047a000` General, `FUN_00479f70` Commander) put an
//! officer aboard a sector planner's flagship (its `+0x38`) and give him
//! its command; states 2 and 3 of the module take the sector planner's
//! requests (`FUN_004c8630`, `FUN_004c86b0`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ai::AIAction;
use crate::ai_module1::{Step, ANY};
use crate::ai_module8::SectorPlanner;
use crate::ai_records::{pick, sector_at, Galaxy, Mask, Order};
use crate::character_command::CommandKind;
use crate::ids::{CharacterKey, SectorKey, SystemKey};
use crate::mission_planning::{standing_system, Planner};
use crate::missions::{MissionKind, MissionMember};
use crate::world::GameWorld;

/// Planner `+0x20` bit `0x80000000`: cancelled (`FUN_0042ef90`); its next
/// step releases its member and leaves the list (`FUN_004bbba0`).
pub const CANCELLED: u32 = 0x8000_0000;
/// Planner `+0x20` bit 1: it holds its member (slot `+0x38`, `FUN_0047a5b0`,
/// `FUN_00479c00`).
pub const HELD: u32 = 1;
/// Planner `+0x20` bit 2: it keeps its target (slot `+0x34`, `FUN_0047a480`,
/// `FUN_00479ad0`).
pub const KEPT: u32 = 2;

/// Planner `+0x20` bit 4 (types 3 to 5): its member holds a command
/// aboard the flagship (`FUN_004bc6d0`), or takes it now (`FUN_004bca20`).
pub const RANKED: u32 = 4;
/// Planner `+0x20` bit 8 (types 3 to 5): its member stands at its target,
/// the flagship (`FUN_004bc6d0`), or is sent there (`FUN_004bca20`).
pub const IN_PLACE: u32 = 8;
/// Planner `+0x20` bit `0x10` (types 3 to 5): its member waits at the
/// station (`FUN_004bca20`).
pub const WAITING: u32 = 0x10;

/// System record `+0x30` bit `0x10000000`: a system the side guards.
///
/// Its headquarters take it (`FUN_004e5870`), and so does a production
/// module's site (`FUN_004dd120`). The sector records count them at `+0x188`
/// (`FUN_00432350`), and the controller sums them at `+0x220`
/// (`FUN_0041b7e0`).
pub const GUARDED: u32 = 0x1000_0000;
/// System record `+0x30` bit `0x20000000`: a type 1 planner's member is
/// posted there (`FUN_0047a5b0`); slot `+0x2c` (`FUN_0047a850`) clears it.
pub const POSTED: u32 = 0x2000_0000;

/// Character record `+0x30` bits a member may not carry for its order
/// (`FUN_004bbc50`): inactive, injured, en route, prisoner, on a mission,
/// a mandatory mission, or holding a command (`0x7000000`).
const BUSY: u32 = 0x0740_00f1;
/// The record's officer ranks, the command it holds (`FUN_00402230`).
const COMMANDING: u32 = 0x700_0000;
/// Character record `+0x30` bits `0x10` and `0x40`: injured or a prisoner,
/// so a planner lets its member go (`FUN_0047a5b0`, `FUN_00479c00`).
const UNFIT: u32 = 0x50;
/// Character record `+0x30` bits `0x20` and `0x80`: en route or on a
/// mission, so the planner is not ready (`FUN_004bba90`) and keeps its
/// member's command at a release (`FUN_004bbba0`).
const AWAY: u32 = 0xa0;

/// One movement planner (`FUN_004bbd60`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandPlanner {
    /// `+0x18`: its id in the list.
    pub id: u32,
    /// Its type (`FUN_0042efe0`): 1 or 6 here.
    pub kind: u32,
    /// `+0x1c`: its step's state (`FUN_004bbb00`).
    pub state: u32,
    /// `+0x20`: its flags ([`CANCELLED`], [`HELD`], [`KEPT`]).
    pub flags: u32,
    /// `+0x28`: the system it posts its member at.
    pub target: Option<SystemKey>,
    /// `+0x2c`: the character it holds.
    pub member: Option<CharacterKey>,
    /// Types 3 to 5, `+0x54`: the sector planner it serves, 0 for none
    /// (`FUN_00476d10`; `FUN_00476de0` clears it). The save keeps it apart
    /// ([`OfficerPost`]).
    #[serde(skip)]
    pub owner: u32,
    /// Types 3 to 5, `+0x28`: the owner's flagship, a capital ship's id
    /// (`FUN_004bc5d0`).
    #[serde(skip)]
    pub flagship: Option<u64>,
    /// Types 3 to 5, `+0x48`: the owner's station (`+0x3c`, `FUN_004bc5d0`).
    #[serde(skip)]
    pub station: Option<SystemKey>,
}

/// The fields of a type 3 to 5 planner that the save keeps apart from
/// [`ModuleC`]'s own encoding (v130): [`CommandPlanner::owner`],
/// [`CommandPlanner::flagship`], [`CommandPlanner::station`], by id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficerPost {
    pub id: u32,
    pub owner: u32,
    pub flagship: Option<u64>,
    pub station: Option<SystemKey>,
}

/// What the save keeps of [`ModuleC`] beyond its own encoding (v130): the
/// sector planner cursor `+0x44`, the sector planner found `+0x40`, and
/// each planner's [`OfficerPost`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectorLinks {
    pub cursor: u32,
    pub found: u32,
    pub posts: Vec<OfficerPost>,
}

/// Module `0xc`'s fields (`FUN_004c8260`: `+0x3c..+0x4c`), its planner
/// list (`+0x50`) and the controller words only it and the strategy use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleC {
    /// `+0x3c`: the cycle's state.
    pub state: u32,
    /// `+0x48`: the planner this cycle steps.
    pub current: u32,
    /// `+0x4c`: the planner the next cycle starts from.
    pub next: u32,
    /// `+0x50`: the planners, in id order.
    pub planners: Vec<CommandPlanner>,
    /// The list's next id.
    pub next_id: u32,
    /// Controller `+0x358`: the type the strategy asks for, 0 for none.
    pub wanted: u32,
    /// Controller `+0x354`: the planner the strategy cancels, 0 for none.
    pub cancel: u32,
    /// Controller `+0x35c`, `+0x360`: the strategy's planners of types 1
    /// and 6 made so far (`FUN_0041b5d0`); nothing takes one off.
    pub made: [i32; 2],
    /// Controller `+0x28`: the Alliance's guarded headquarters
    /// (`FUN_004e5870`).
    pub headquarters: Option<SystemKey>,
    /// `+0x44`: the sector planner state 2 starts from (`FUN_004c8630`).
    /// The save keeps it apart ([`SectorLinks`]).
    #[serde(skip)]
    pub sector_cursor: u32,
    /// `+0x40`: the sector planner with a request that state 3 takes.
    #[serde(skip)]
    pub sector_found: u32,
}

impl Default for ModuleC {
    fn default() -> Self {
        Self {
            state: 0,
            current: 0,
            next: 0,
            planners: Vec::new(),
            // FUN_005f52c0 starts a list's counter at 1.
            next_id: 1,
            wanted: 0,
            cancel: 0,
            made: [0; 2],
            headquarters: None,
            sector_cursor: 0,
            sector_found: 0,
        }
    }
}

/// What module `0xc` reads and changes besides itself.
pub(crate) struct InputsC<'a, 'p> {
    pub(crate) world: &'a GameWorld,
    pub(crate) galaxy: &'a mut Galaxy,
    /// The side's kept record `+0x30` bits ([`crate::ai::AIState::record_kinds`]).
    pub(crate) kinds: &'a mut HashMap<SystemKey, u32>,
    pub(crate) planner: &'a mut Planner<'p>,
    pub(crate) is_alliance: bool,
    /// Module 8's sector planners (controller `+0xd8`).
    pub(crate) sector_planners: &'a mut [SectorPlanner],
}

impl InputsC<'_, '_> {
    fn set_kind(&mut self, system: SystemKey, set: u32, clear: u32) {
        self.galaxy.update_kind(system, set, clear);
        let value = self.kinds.entry(system).or_default();
        *value = (*value & !clear) | set;
        if *value == 0 {
            self.kinds.remove(&system);
        }
    }

    fn kind(&self, system: SystemKey) -> Option<u32> {
        self.galaxy.system(system).map(|r| r.kind)
    }

    fn sectors(&mut self, masks: [Mask; 3], key: usize) -> Vec<SectorKey> {
        let planner = &mut *self.planner;
        self.galaxy
            .sector_query(masks, key, Order::Ascending, &mut || planner.coin())
    }

    fn systems(&mut self, sector: SectorKey, masks: [Mask; 3], key: usize) -> Vec<SystemKey> {
        let planner = &mut *self.planner;
        self.galaxy
            .system_query(sector, masks, key, Order::Ascending, &mut || planner.coin())
    }

    /// `FUN_0041a1d0` (order 1): the sectors passing `masks`, each keyed
    /// on a draw below 20 as it goes into the list, so the key draws and
    /// the tie coins interleave; picked.
    fn random_sector(&mut self, masks: [Mask; 3]) -> Option<SectorKey> {
        let draws = std::cell::RefCell::new(&mut *self.planner);
        let items = self
            .galaxy
            .sectors
            .iter()
            .filter(|s| s.admitted(masks))
            .map(|s| (s.sector, draws.borrow_mut().random(0x14)));
        pick(&[crate::ai_records::ranked(
            items,
            Order::Ascending,
            &mut || draws.borrow_mut().random(10) < 5,
        )])
    }

    /// `FUN_0041a460` (order 1): `sector`'s systems passing `masks`, each
    /// keyed on a draw below 10; picked.
    fn random_system(&mut self, sector: SectorKey, masks: [Mask; 3]) -> Option<SystemKey> {
        let draws = std::cell::RefCell::new(&mut *self.planner);
        let items = self
            .galaxy
            .sector(sector)?
            .systems
            .iter()
            .filter(|r| {
                masks[0].admits(r.state) && masks[1].admits(r.summary) && masks[2].admits(r.kind)
            })
            .map(|r| (r.system, draws.borrow_mut().random(10)));
        pick(&[crate::ai_records::ranked(
            items,
            Order::Ascending,
            &mut || draws.borrow_mut().random(10) < 5,
        )])
    }
}

/// The controller's `+0x220`: the guarded systems ([`GUARDED`]) summed
/// over the sector records' `+0x188` (`FUN_0041b7e0`).
#[must_use]
pub fn guarded(galaxy: &Galaxy) -> i32 {
    galaxy
        .sectors
        .iter()
        .map(|s| s.values[sector_at(0x188)])
        .sum()
}

/// Controller `+4` bit `0x80`: a sector record with `+0x2c` bits 4 (a
/// system of the side's) and `0x200` (Coruscant, held) (`FUN_0041b7e0`).
#[must_use]
pub fn holds_coruscant(galaxy: &Galaxy) -> bool {
    galaxy
        .sectors
        .iter()
        .any(|s| s.summary & 4 != 0 && s.summary & 0x200 != 0)
}

/// The type's index in the controller's counts (slot `+0x28`:
/// `FUN_0047a470` 0, `FUN_00479ac0` 1).
const fn made_index(kind: u32) -> usize {
    if kind == 6 {
        1
    } else {
        0
    }
}

impl ModuleC {
    /// Module `0xd`'s state 4 (`FUN_004e57e0` with slot `+0x1c`,
    /// `FUN_004eaaa0`, shared by every strategy class): ask for a type 1
    /// planner while fewer were made than the side guards, else cancel one
    /// while more were; the Empire asks for a type 6 while it holds
    /// Coruscant, has no minor characters (side `+0xa4`) and made none,
    /// and cancels one when it has minors or has lost Coruscant and made
    /// one. A cancellation names the first of the strategy's planners (slot
    /// `+0xc` 2: types 1 and 6) not keeping its target (`+0x20` bit 2
    /// clear), or none.
    pub fn strategy_requests(
        &mut self,
        guarded: i32,
        minors: i32,
        coruscant: bool,
        is_alliance: bool,
    ) {
        let mut wanted = 0;
        let mut cancel = false;
        let [ones, sixes] = self.made;
        if ones < guarded {
            wanted = 1;
        } else if guarded < ones {
            cancel = true;
        }
        if !is_alliance {
            if wanted == 0 && minors == 0 && coruscant && sixes == 0 {
                wanted = 6;
            } else if !cancel && (minors > 0 || !coruscant) && sixes == 1 {
                cancel = true;
            }
        }
        if wanted != 0 {
            self.wanted = wanted;
        }
        if cancel {
            self.cancel = self
                .planners
                .iter()
                .find(|p| matches!(p.kind, 1 | 6) && p.flags & KEPT == 0)
                .map_or(0, |p| p.id);
        }
    }

    /// Module `0xd`'s state 6 (`FUN_004e5870`): system `0x109`
    /// (Coruscant) is guarded while it is the side's; the Alliance also
    /// guards its headquarters (controller `+0x28`), dropping it once the
    /// system is not the side's headquarters, and else finds it: a sector
    /// with `+0x2c` `0x80000000` by `+0x7c` (key 3, order 1), its system
    /// with `+0x30` `0x80000000` (key 0, order 1).
    pub(crate) fn guard(&mut self, io: &mut InputsC<'_, '_>) {
        let coruscant = io
            .world
            .systems
            .iter()
            .find(|(_, s)| s.dat_id.raw() == 0x9000_0109)
            .map(|(k, _)| k);
        if let Some(system) = coruscant {
            if let Some(kind) = io.kind(system) {
                if kind & 1 == 0 {
                    io.set_kind(system, 0, GUARDED);
                } else {
                    io.set_kind(system, GUARDED, 0);
                }
            }
        }
        if !io.is_alliance {
            return;
        }
        if let Some(hq) = self.headquarters {
            if let Some(kind) = io.kind(hq) {
                if kind & 1 == 0 || kind & 0x8000_0000 == 0 {
                    io.set_kind(hq, 0, GUARDED);
                    self.headquarters = None;
                } else {
                    io.set_kind(hq, GUARDED, 0);
                }
            }
        }
        if self.headquarters.is_none() {
            let sector = pick(&[io.sectors([ANY, ANY, Mask::new(0x8000_0000, 0)], 3)]);
            let system = sector.and_then(|sector| {
                pick(&[io.systems(sector, [ANY, ANY, Mask::new(0x8000_0000, 0)], 0)])
            });
            if let Some(system) = system {
                if io.kind(system).is_some_and(|kind| kind & 1 != 0) {
                    io.set_kind(system, GUARDED, 0);
                    self.headquarters = Some(system);
                } else {
                    io.set_kind(system, 0, GUARDED);
                }
            }
        }
    }

    /// One step of the module (`FUN_004c84d0`):
    /// 1. `FUN_004c85b0`: cancel the planner the strategy names
    ///    (`FUN_0042ef90`) and make the type it asks for (`FUN_0042ef50`,
    ///    counted by `FUN_0041b5d0`);
    /// 2. `FUN_004c8630`: from `+0x44`, the first sector planner asking
    ///    (`FUN_00476cf0`: `+0xe0` or `+0xe4` set) into `+0x40`, the one
    ///    after it into `+0x44`; none found goes to 4;
    /// 3. `FUN_004c86b0`: that planner's requests ([`Self::sector_requests`]);
    /// 4. `FUN_004c8740`: from `+0x4c`, the first planner ready (slot
    ///    `+0x10`, `FUN_004bba90`: no member away on a mission or en
    ///    route) becomes `+0x48` and the one after it `+0x4c`; none ends
    ///    the cycle;
    /// 5. `FUN_004c87c0`: one state of that planner's step. An order
    ///    completes the module, raising its priority while the planner's
    ///    cycle goes on; the planner's end ends the module's cycle.
    pub(crate) fn step(&mut self, io: &mut InputsC<'_, '_>) -> Step {
        match self.state {
            1 => {
                self.take_requests();
                self.state = 2;
                Step::pending()
            }
            2 => {
                self.state = if self.find_request(io) { 3 } else { 4 };
                Step::pending()
            }
            3 => {
                self.sector_requests(io);
                self.state = 4;
                Step::pending()
            }
            4 => {
                if self.choose(io) {
                    self.state = 5;
                    Step::pending()
                } else {
                    self.state = 0;
                    Step {
                        order: None,
                        complete: true,
                        raise: false,
                    }
                }
            }
            5 => {
                let Some(at) = self.planners.iter().position(|p| p.id == self.current) else {
                    // FUN_004c87c0: no planner ends the cycle.
                    self.state = 0;
                    return Step {
                        order: None,
                        complete: true,
                        raise: false,
                    };
                };
                let (order, done) = self.step_planner(at, io);
                if done {
                    self.state = 0;
                }
                Step {
                    complete: order.is_some() || done,
                    raise: order.is_some() && !done,
                    order,
                }
            }
            _ => {
                self.state = 1;
                Step::pending()
            }
        }
    }

    /// `FUN_004c85b0`.
    fn take_requests(&mut self) {
        let cancel = std::mem::take(&mut self.cancel);
        if let Some(p) = self
            .planners
            .iter_mut()
            .find(|p| p.id == cancel && cancel != 0)
        {
            p.flags |= CANCELLED;
        }
        let kind = std::mem::take(&mut self.wanted);
        if matches!(kind, 1 | 6) {
            self.make(kind);
            self.made[made_index(kind)] += 1;
        }
    }

    /// `FUN_0042ef50` -> `FUN_0042efe0`: a new planner of `kind` at the
    /// list's end; its id.
    fn make(&mut self, kind: u32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.planners.push(CommandPlanner {
            id,
            kind,
            ..CommandPlanner::default()
        });
        id
    }

    /// `FUN_004c8630`: from the sector planner `+0x44` names (else the
    /// first), the first whose `+0xe0` or `+0xe4` is set (`FUN_00476cf0`)
    /// into `+0x40` and the one after it into `+0x44`; with none both are
    /// cleared, so the next search starts from the first.
    fn find_request(&mut self, io: &InputsC<'_, '_>) -> bool {
        let planners = &*io.sector_planners;
        let start = planners
            .iter()
            .position(|p| p.id == self.sector_cursor)
            .unwrap_or(0);
        let found = planners
            .iter()
            .skip(start)
            .position(|p| p.movement != [0, 0])
            .map(|at| at + start);
        let Some(at) = found else {
            self.sector_found = 0;
            self.sector_cursor = 0;
            return false;
        };
        self.sector_found = planners[at].id;
        self.sector_cursor = planners.get(at + 1).map_or(0, |p| p.id);
        true
    }

    /// `FUN_004c86b0`: the sector planner `+0x40` names cancels the
    /// planner its `+0xe4` names ([`cancel`]) and clears it; the type its
    /// `+0xe0` names is made and, as a sector planner's (slot `+0xc` 1),
    /// attached to it (`FUN_00476d10`: `+0xe0` cleared, the planner
    /// counted by its type and given `+0x54`).
    fn sector_requests(&mut self, io: &mut InputsC<'_, '_>) {
        let found = self.sector_found;
        let Some(sp) = io.sector_planners.iter_mut().find(|p| p.id == found) else {
            return;
        };
        let [wanted, cancelled] = sp.movement;
        if cancelled != 0 {
            cancel(&mut self.planners, cancelled);
            sp.movement[1] = 0;
        }
        if wanted != 0 {
            let id = self.make(wanted);
            if matches!(wanted, 3..=5) {
                sp.movement[0] = 0;
                if let Some(p) = self.planners.iter_mut().find(|p| p.id == id) {
                    p.owner = found;
                }
            }
        }
    }

    /// The fields the save keeps apart ([`SectorLinks`]).
    #[must_use]
    pub fn links(&self) -> SectorLinks {
        SectorLinks {
            cursor: self.sector_cursor,
            found: self.sector_found,
            posts: self
                .planners
                .iter()
                .filter(|p| p.owner != 0 || p.flagship.is_some() || p.station.is_some())
                .map(|p| OfficerPost {
                    id: p.id,
                    owner: p.owner,
                    flagship: p.flagship,
                    station: p.station,
                })
                .collect(),
        }
    }

    /// Put back what [`Self::links`] gave.
    pub fn restore_links(&mut self, links: &SectorLinks) {
        self.sector_cursor = links.cursor;
        self.sector_found = links.found;
        for post in &links.posts {
            if let Some(p) = self.planners.iter_mut().find(|p| p.id == post.id) {
                p.owner = post.owner;
                p.flagship = post.flagship;
                p.station = post.station;
            }
        }
    }

    /// `FUN_004c8740`.
    fn choose(&mut self, io: &InputsC<'_, '_>) -> bool {
        let start = self
            .planners
            .iter()
            .position(|p| p.id == self.next)
            .unwrap_or(0);
        let ready = |p: &CommandPlanner| {
            p.member.is_none_or(|c| {
                io.planner
                    .flags(MissionMember::Character(c))
                    .is_none_or(|flags| flags & AWAY == 0)
            })
        };
        let found = self
            .planners
            .iter()
            .skip(start)
            .position(ready)
            .map(|at| at + start);
        let Some(at) = found else {
            self.current = 0;
            self.next = 0;
            return false;
        };
        self.current = self.planners[at].id;
        self.next = self.planners.get(at + 1).map_or(0, |p| p.id);
        true
    }

    /// One state of planner `at`'s step (`FUN_004bbb00`): 1 releases a
    /// cancelled planner (`FUN_004bbba0`), else 3 checks the target (slot
    /// `+0x34`), 2 the member (slot `+0x38`), ending without one, and 4
    /// gives the order (`FUN_004bbc50`). Returns the order and whether the
    /// planner's cycle ended.
    fn step_planner(&mut self, at: usize, io: &mut InputsC<'_, '_>) -> (Option<AIAction>, bool) {
        if matches!(self.planners[at].kind, 3..=5) {
            return self.step_officer(at, io);
        }
        let state = self.planners[at].state;
        let (order, done) = match state {
            1 => {
                if self.planners[at].flags & CANCELLED != 0 {
                    let p = self.planners.remove(at);
                    (release(&p, io), true)
                } else {
                    self.planners[at].state = 3;
                    (None, false)
                }
            }
            3 => {
                target(&mut self.planners[at], io);
                self.planners[at].state = 2;
                (None, false)
            }
            2 => {
                if member(&mut self.planners[at], io) {
                    self.planners[at].state = 4;
                    (None, false)
                } else {
                    (None, true)
                }
            }
            4 => (order(&self.planners[at], io), true),
            _ => {
                self.planners[at].state = 1;
                (None, false)
            }
        };
        if done {
            if let Some(p) = self.planners.get_mut(at).filter(|p| p.id == self.current) {
                p.state = 0;
            }
        }
        (order, done)
    }

    /// One state of a type 3 to 5 planner's step (`FUN_004bc480`): 1
    /// releases a cancelled planner (`FUN_004bc520`), else 3 reads the
    /// owner's flagship and station (`FUN_004bc5d0`), 2 the member (slot
    /// `+0x28`), ending without one, and 4 gives the order (`FUN_004bca20`).
    fn step_officer(&mut self, at: usize, io: &mut InputsC<'_, '_>) -> (Option<AIAction>, bool) {
        let state = self.planners[at].state;
        let (order, done) = match state {
            1 => {
                if self.planners[at].flags & CANCELLED != 0 {
                    let p = self.planners.remove(at);
                    (release(&p, io), true)
                } else {
                    self.planners[at].state = 3;
                    (None, false)
                }
            }
            3 => {
                read_owner(&mut self.planners[at], io);
                self.planners[at].state = 2;
                (None, false)
            }
            2 => {
                if officer(&mut self.planners[at], io) {
                    self.planners[at].state = 4;
                    (None, false)
                } else {
                    (None, true)
                }
            }
            4 => (board(&mut self.planners[at], io), true),
            _ => {
                self.planners[at].state = 1;
                (None, false)
            }
        };
        if done {
            if let Some(p) = self.planners.get_mut(at).filter(|p| p.id == self.current) {
                p.state = 0;
            }
        }
        (order, done)
    }

    /// The characters the planners hold, out of every other query
    /// (`FUN_0047b610`'s `0x800000` cleared).
    pub fn members(&self) -> impl Iterator<Item = CharacterKey> + '_ {
        self.planners.iter().filter_map(|p| p.member)
    }

    /// The module's step was refused: the order goes and the cycle ends
    /// (the module's slot `+0x18`, `FUN_004dd810`).
    pub const fn refused(&mut self) {
        self.state = 0;
    }
}

/// Slot `+0x2c`: type 1 (`FUN_0047a850`) clears its target's
/// [`POSTED`]; type 6's (`FUN_00619730`) does nothing.
fn unpost(p: &CommandPlanner, io: &mut InputsC<'_, '_>) {
    if p.kind == 1 {
        if let Some(system) = p.target {
            io.set_kind(system, 0, POSTED);
        }
    }
}

/// `FUN_004bbba0`: a cancelled planner leaves the list (`FUN_0041b600`);
/// its member, still its own, is released, and loses its command (`0x260`)
/// unless away (`0xa0`); its target is given up (slot `+0x2c`).
fn release(p: &CommandPlanner, io: &mut InputsC<'_, '_>) -> Option<AIAction> {
    let order = p.member.and_then(|c| {
        let flags = io.planner.flags(MissionMember::Character(c))?;
        (flags & COMMANDING != 0 && flags & AWAY == 0).then_some(AIAction::Command {
            character: c,
            kind: CommandKind::None,
        })
    });
    unpost(p, io);
    order
}

/// Slot `+0x34` (`FUN_0047a480`, `FUN_00479ad0`): keep the target while
/// its record still qualifies (type 1: [`GUARDED`]; type 6: `+0x30`
/// `0x40000000` and 1), else give it up (slot `+0x2c`) and choose anew
/// (slot `+0x30`); a target found is kept.
fn target(p: &mut CommandPlanner, io: &mut InputsC<'_, '_>) {
    p.flags &= !KEPT;
    if let Some(system) = p.target {
        if let Some(kind) = io.kind(system) {
            let keeps = if p.kind == 6 {
                kind & 0x4000_0000 != 0 && kind & 1 != 0
            } else {
                kind & GUARDED != 0
            };
            if keeps {
                p.flags |= KEPT;
            } else {
                unpost(p, io);
            }
        }
    }
    if p.flags & KEPT == 0 {
        p.target = if p.kind == 6 { capital(io) } else { post(io) };
        if p.target.is_some() {
            p.flags |= KEPT;
        }
    }
}

/// Type 1's slot `+0x30` (`FUN_0047a870`): a sector with `+0x24`
/// `0x40000000` and `+0x2c` [`GUARDED`] by `+0x108` (key `0x26`, order
/// 1), its system with `+0x28` `0x80000000` and `+0x30` [`GUARDED`]
/// without [`POSTED`] by `v[0x21]` (order 1); with none, the same without
/// the headquarters bits.
fn post(io: &mut InputsC<'_, '_>) -> Option<SystemKey> {
    for (candidates, state) in [(0x4000_0000, 0x8000_0000), (0, 0)] {
        let sector =
            pick(&[io.sectors([Mask::new(candidates, 0), ANY, Mask::new(GUARDED, 0)], 0x26)]);
        let system = sector.and_then(|sector| {
            let masks = [Mask::new(state, 0), ANY, Mask::new(GUARDED, POSTED)];
            pick(&[io.systems(sector, masks, 0x21)])
        });
        if system.is_some() {
            return system;
        }
    }
    None
}

/// Type 6's slot `+0x30` (`FUN_00479e30`): a random sector with `+0x24`
/// and `+0x2c` `0x40000000` (`FUN_0041a1d0`, order 1), its random system
/// with `+0x28` `0x80000000` and `+0x30` `0x40000000` (`FUN_0041a460`).
fn capital(io: &mut InputsC<'_, '_>) -> Option<SystemKey> {
    let sector = io.random_sector([Mask::new(0x4000_0000, 0), ANY, Mask::new(0x4000_0000, 0)])?;
    io.random_system(
        sector,
        [Mask::new(0x8000_0000, 0), ANY, Mask::new(0x4000_0000, 0)],
    )
}

/// Slot `+0x38` (`FUN_0047a5b0`, `FUN_00479c00`): keep the member while
/// its record is not injured or a prisoner (type 6 also wants the
/// Emperor's `0x400`), else release it and give up the target. Without
/// one, and with a target no other planner has posted to, take the best
/// character: type 1 a free General-able one of leadership 50 or more,
/// nearest the target (`0x800002`, caps `0x4000000`, without
/// `0x4f4000f1`); type 6 the Emperor (`0x402`, without `0x74000f1`). The
/// pick posts the target ([`POSTED`], type 1 only). Returns whether it
/// holds a member.
fn member(p: &mut CommandPlanner, io: &mut InputsC<'_, '_>) -> bool {
    p.flags &= !HELD;
    if let Some(c) = p.member {
        match io.planner.flags(MissionMember::Character(c)) {
            Some(flags) => {
                let fit = if p.kind == 6 {
                    flags & UNFIT == 0 && flags & 0x400 != 0
                } else {
                    flags & UNFIT == 0
                };
                if fit {
                    p.flags |= HELD;
                } else {
                    p.member = None;
                    unpost(p, io);
                }
            }
            None => p.member = None,
        }
    }
    if p.flags & HELD != 0 {
        return true;
    }
    let Some(target) = p.target else {
        return false;
    };
    let open = io.kind(target).is_some_and(|kind| {
        if p.kind == 6 {
            kind & 1 != 0
        } else {
            kind & POSTED == 0
        }
    });
    if !open {
        p.target = None;
        return false;
    }
    let picked = if p.kind == 6 {
        io.planner.pick_officer((0x402, 0, 0x0740_00f1, 0), None)
    } else {
        let world = io.world;
        let galaxy = &*io.galaxy;
        let here = galaxy.system(target).map(|r| r.position);
        let near = move |m: MissionMember| -> Option<i32> {
            let system = located(world, m)?;
            let there = galaxy.system(system)?.position;
            Some(crate::ai_records::squared_distance(there, here?))
        };
        io.planner
            .pick_officer((0x80_0002, 0x400_0000, 0x4f40_00f1, 50), Some(&near))
    };
    if let Some(c) = picked {
        p.member = Some(c);
        p.flags |= HELD;
        if p.kind == 1 {
            io.set_kind(target, POSTED, 0);
        }
    }
    p.flags & HELD != 0
}

/// The system a member is at: the planet it stands on, else its fleet's
/// system while the fleet stands (`FUN_00403750` walks the location up to
/// a system).
pub(crate) fn located(world: &GameWorld, member: MissionMember) -> Option<SystemKey> {
    standing_system(world, member).or_else(|| {
        let c = world.characters.get(member.character()?)?;
        let fleet = world.fleets.get(c.current_fleet?)?;
        Some(fleet.location)
    })
}

/// `FUN_0042ef90` on planner `id`: a sector planner's planner (slot
/// `+0xc` returns 1) with an owner is first taken off it (`FUN_00476de0`:
/// the owner's count of its type drops, `+0x54` cleared); every one is
/// marked [`CANCELLED`], so its next step releases it.
///
/// port: the owner's counts `+0xc8..+0xd0` are the planners naming it
/// ([`officers_of`]), so clearing `+0x54` drops the count.
pub(crate) fn cancel(planners: &mut [CommandPlanner], id: u32) {
    if let Some(p) = planners.iter_mut().find(|p| p.id == id && id != 0) {
        if matches!(p.kind, 3..=5) {
            p.owner = 0;
        }
        p.flags |= CANCELLED;
    }
}

/// The sector planner `owner`'s movement planners of `kind`, its count
/// `+0xc8 + 4 * FUN_00479520(kind)` (type 3 index 0, 4 index 1, 5 index 2):
/// `FUN_00476d10` raises it as it attaches one and `FUN_00476de0` lowers it
/// as it takes one off.
#[must_use]
pub fn officers_of(planners: &[CommandPlanner], owner: u32, kind: u32) -> i32 {
    let n = planners
        .iter()
        .filter(|p| owner != 0 && p.owner == owner && p.kind == kind)
        .count();
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// The command a type 3 to 5 planner gives (slot `+0x1c`): type 3
/// `FUN_0047a400` `0x262` Admiral, 4 `FUN_0047a040` `0x263` General, 5
/// `FUN_00479fd0` `0x261` Commander.
const fn command_of(kind: u32) -> CommandKind {
    match kind {
        3 => CommandKind::Admiral,
        4 => CommandKind::General,
        _ => CommandKind::Commander,
    }
}

/// The capability a type 3 to 5 planner's member needs (`+0x50`, set by
/// `FUN_0047a3c0`, `FUN_0047a000`, `FUN_00479f70`): record `+0x34`
/// `0x1000000` Admiral, `0x4000000` General, `0x2000000` Commander
/// (`FUN_00401d20`).
const fn caps_of(kind: u32) -> u32 {
    match kind {
        3 => 0x100_0000,
        4 => 0x400_0000,
        _ => 0x200_0000,
    }
}

/// The fleet carrying the capital ship `id`.
fn ship_fleet(world: &GameWorld, id: u64) -> Option<crate::ids::FleetKey> {
    world
        .fleets
        .iter()
        .find(|(_, f)| f.capital_ships.iter().any(|s| s.id == id && s.alive))
        .map(|(k, _)| k)
}

/// Whether character `c` stands at the flagship `flagship` (record `+0x38`
/// equal to the planner's `+0x28`).
///
/// port: the port seats a fleet's characters on a ship of its own choice
/// ([`crate::missions::seat_characters`]), and a Move takes a character to
/// a fleet, not to one of its ships; a member aboard the flagship's fleet
/// stands at the flagship.
fn at_flagship(world: &GameWorld, c: CharacterKey, flagship: Option<u64>) -> bool {
    let fleet = world.characters.get(c).and_then(|c| c.current_fleet);
    fleet.is_some() && fleet == flagship.and_then(|id| ship_fleet(world, id))
}

/// `FUN_004bc5d0`: bit 2 cleared; from the owner (`+0x54`), the flagship
/// (`+0x38`) into `+0x28` and the station (`+0x3c`) into `+0x48`; bit 2
/// again when the first is a capital ship and the second a system.
fn read_owner(p: &mut CommandPlanner, io: &InputsC<'_, '_>) {
    p.flags &= !KEPT;
    let owner = p.owner;
    let Some(sp) = io
        .sector_planners
        .iter()
        .find(|s| s.id == owner && owner != 0)
    else {
        return;
    };
    p.station = sp.station;
    p.flagship = sp.flagship;
    if p.flagship.is_some() && p.station.is_some() {
        p.flags |= KEPT;
    }
}

/// Slot `+0x28` of types 3 and 5 (`FUN_004bc6d0`) and 4 (`FUN_0047a050`):
/// bits 1, 4, 8 and `0x10` cleared; a member neither injured nor a prisoner
/// is kept (bit 1), with bit 8 when it stands at the flagship and 4 as well
/// when it holds a command; else it is let go. Without one, and with an
/// owner, the best free officer of the planner's capability and leadership
/// 50 or more (`FUN_00403460(0x800002, +0x50, 0x4f4000f1, 0, 6, 50, 10000,
/// 1)`; type 4 forbids `0x474000f1`, admitting a General reserved for
/// research or diplomacy), merged with the nearest to the station or the
/// sector's first system (`FUN_00403750`, `FUN_00432120(.., 0)`, order 2).
/// Returns whether it holds a member.
///
/// port: the member's record `+0x2c` (the owner's plan id, slot `+0x20`)
/// has no port counterpart.
fn officer(p: &mut CommandPlanner, io: &mut InputsC<'_, '_>) -> bool {
    p.flags &= !(HELD | RANKED | IN_PLACE | WAITING);
    if let Some(c) = p.member {
        match io.planner.flags(MissionMember::Character(c)) {
            Some(flags) if flags & UNFIT == 0 => {
                p.flags |= HELD;
                if at_flagship(io.world, c, p.flagship) {
                    p.flags |= IN_PLACE;
                    if flags & COMMANDING != 0 {
                        p.flags |= RANKED;
                    }
                }
            }
            _ => p.member = None,
        }
    }
    if p.flags & HELD != 0 {
        return true;
    }
    let owner = p.owner;
    let Some(sp) = io
        .sector_planners
        .iter()
        .find(|s| s.id == owner && owner != 0)
    else {
        return false;
    };
    let first = sp
        .sector
        .and_then(|s| io.galaxy.sector(s))
        .and_then(|r| r.systems.first())
        .map(|r| r.system);
    let origin = p.station.or(first);
    let forbid = if p.kind == 4 {
        0x4740_00f1
    } else {
        0x4f40_00f1
    };
    let world = io.world;
    let galaxy = &*io.galaxy;
    let here = origin.and_then(|o| galaxy.system(o)).map(|r| r.position);
    let near = move |m: MissionMember| -> Option<i32> {
        let system = located(world, m)?;
        let there = galaxy.system(system)?.position;
        Some(crate::ai_records::squared_distance(there, here?))
    };
    let picked = io
        .planner
        .pick_officer((0x80_0002, caps_of(p.kind), forbid, 50), Some(&near));
    if let Some(c) = picked {
        p.member = Some(c);
        p.flags |= HELD;
    }
    p.flags & HELD != 0
}

/// `FUN_004bca20`: a member free for orders (none of `0x74000f1`):
/// - at the flagship and without a command, it takes slot `+0x1c`'s
///   (bit 4);
/// - with no flagship, it moves to the station unless there (bit `0x10`);
/// - aboard another fleet, not en route: where the flagship is, not en
///   route, it boards it (bit 8, a Move `0x201` to the ship); elsewhere it
///   moves to the station;
/// - on a planet: in the flagship's system, not en route, it boards it
///   (bit 8); elsewhere it moves to the station unless there (bit `0x10`).
fn board(p: &mut CommandPlanner, io: &InputsC<'_, '_>) -> Option<AIAction> {
    let c = p.member?;
    let member = MissionMember::Character(c);
    let flags = io.planner.flags(member)?;
    if flags & BUSY != 0 {
        return None;
    }
    let move_to = |to: SystemKey| AIAction::DispatchMission {
        kind: MissionKind::Move,
        team: vec![member],
        decoys: Vec::new(),
        target_system: to,
        target_character: None,
        target_object: None,
    };
    let Some(flagship) = p.flagship else {
        let station = p.station?;
        if standing_system(io.world, member) == Some(station) {
            p.flags |= WAITING;
            return None;
        }
        return Some(move_to(station));
    };
    if at_flagship(io.world, c, Some(flagship)) {
        if flags & COMMANDING != 0 {
            return None;
        }
        p.flags |= RANKED;
        return Some(AIAction::Command {
            character: c,
            kind: command_of(p.kind),
        });
    }
    let target = ship_fleet(io.world, flagship)?;
    let record = io.galaxy.fleets.get(&target)?;
    let ship_moving = record
        .ships
        .iter()
        .find(|s| s.id == flagship)
        .is_none_or(|s| s.flags & 0x800 != 0);
    let there = io.world.fleets.get(target)?.location;
    let character = io.world.characters.get(c)?;
    if let Some(fleet) = character.current_fleet {
        let own = io.galaxy.fleets.get(&fleet)?;
        let seat = own
            .ships
            .iter()
            .find(|s| Some(s.id) == character.ship)
            .or_else(|| own.ships.first())?;
        if seat.flags & 0x800 != 0 {
            return None;
        }
        if io.world.fleets.get(fleet)?.location != there {
            return p.station.map(move_to);
        }
    } else {
        let here = standing_system(io.world, member);
        if here != Some(there) {
            let station = p.station?;
            if here == Some(station) {
                p.flags |= WAITING;
                return None;
            }
            return Some(move_to(station));
        }
    }
    if ship_moving {
        return None;
    }
    p.flags |= IN_PLACE;
    Some(AIAction::BoardFleet {
        character: c,
        fleet: target,
    })
}

/// `FUN_004bbc50`: a member free for orders (none of `0x74000f1`) standing
/// at the target takes its command (slot `+0x1c`: `0x263` General; type 6,
/// slot `+0x18` 6, takes none); elsewhere it moves there (`0x201`,
/// `FUN_004bc040`), the Move pseudo-mission.
fn order(p: &CommandPlanner, io: &InputsC<'_, '_>) -> Option<AIAction> {
    let c = p.member?;
    let member = MissionMember::Character(c);
    let flags = io.planner.flags(member)?;
    if flags & BUSY != 0 {
        return None;
    }
    let target = p.target?;
    if standing_system(io.world, member) == Some(target) {
        (flags & COMMANDING == 0 && p.kind != 6).then_some(AIAction::Command {
            character: c,
            kind: CommandKind::General,
        })
    } else {
        Some(AIAction::DispatchMission {
            kind: MissionKind::Move,
            team: vec![member],
            decoys: Vec::new(),
            target_system: target,
            target_character: None,
            target_object: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_planner::tests::{fixture, rebuild, Fx};
    use crate::missions::MissionState;
    use crate::world::{Character, SkillPair};
    use std::collections::HashSet;

    /// A recruited Alliance minor of `leadership` at `at` who may be a
    /// General.
    fn general(fx: &mut Fx, at: usize, leadership: u32) -> CharacterKey {
        fx.world.characters.insert(Character {
            dat_id: crate::ids::DatId::new(0x3200_0300),
            name: "General".into(),
            is_alliance: true,
            recruited: true,
            can_be_general: true,
            leadership: SkillPair {
                base: leadership,
                variance: 0,
            },
            current_system: Some(fx.systems[at]),
            ..Default::default()
        })
    }

    /// Run `f` with module `0xc`'s inputs over the fixture.
    fn with<R>(fx: &mut Fx, f: impl FnOnce(&mut InputsC<'_, '_>) -> R) -> R {
        let missions = MissionState::new();
        let busy = HashSet::new();
        let mut planner = Planner::new(&fx.world, &missions, &busy, true, 7);
        let mut io = InputsC {
            world: &fx.world,
            galaxy: &mut fx.galaxy,
            kinds: &mut fx.kinds,
            planner: &mut planner,
            is_alliance: true,
            sector_planners: &mut [],
        };
        f(&mut io)
    }

    fn guard_system(fx: &mut Fx, at: usize) {
        fx.kinds.insert(fx.systems[at], GUARDED);
        rebuild(fx);
    }

    #[test]
    fn the_strategy_asks_for_a_general_post_per_guarded_system_and_never_counts_one_off() {
        // FUN_004eaaa0: a type 1 while controller +0x35c < +0x220, a
        // cancellation while above; FUN_0041b5d0 is the only writer of
        // +0x35c, so a cancelled planner still counts.
        let mut m = ModuleC::default();
        m.strategy_requests(2, 1, false, true);
        assert_eq!((m.wanted, m.cancel), (1, 0));
        m.take_requests();
        m.strategy_requests(2, 1, false, true);
        m.take_requests();
        assert_eq!(m.made, [2, 0]);
        let made: Vec<_> = m.planners.iter().map(|p| (p.id, p.kind)).collect();
        assert_eq!(made, [(1, 1), (2, 1)]);
        m.strategy_requests(2, 1, false, true);
        assert_eq!((m.wanted, m.cancel), (0, 0));
        // FUN_004e57e0: the first of the strategy's planners not keeping its
        // target (+0x20 bit 2) is named.
        m.planners[0].flags |= KEPT;
        m.strategy_requests(1, 1, false, true);
        assert_eq!((m.wanted, m.cancel), (0, 2));
        m.take_requests();
        assert_eq!(m.planners[1].flags & CANCELLED, CANCELLED);
        m.planners.remove(1);
        m.strategy_requests(1, 1, false, true);
        assert_eq!((m.made, m.wanted, m.cancel), ([2, 0], 0, 0));
    }

    #[test]
    fn the_empire_asks_for_the_emperor_s_post_without_minors_while_it_holds_coruscant() {
        // FUN_004eaaa0, side 2: type 6 with side +0xa4 0, controller +4
        // bit 0x80 and +0x360 0; cancelled with minors or without
        // Coruscant once +0x360 is 1.
        let mut m = ModuleC {
            made: [1, 0],
            ..ModuleC::default()
        };
        m.strategy_requests(1, 0, true, false);
        assert_eq!(m.wanted, 6);
        m.wanted = 0;
        m.strategy_requests(1, 1, true, false);
        assert_eq!(m.wanted, 0);
        m.strategy_requests(1, 0, true, true);
        assert_eq!(m.wanted, 0, "the Alliance asks for none");
        m.made = [1, 1];
        m.planners.push(CommandPlanner {
            id: 9,
            kind: 6,
            ..CommandPlanner::default()
        });
        m.strategy_requests(1, 0, true, false);
        assert_eq!(m.cancel, 0);
        m.strategy_requests(1, 1, true, false);
        assert_eq!(m.cancel, 9, "minors cancel it");
        m.cancel = 0;
        m.strategy_requests(1, 0, false, false);
        assert_eq!(m.cancel, 9);
        // FUN_0041b5d0: a type 6 counts at +0x360 (slot +0x28 returns 1).
        let mut m = ModuleC {
            wanted: 6,
            ..ModuleC::default()
        };
        m.take_requests();
        assert_eq!(m.made, [0, 1]);
    }

    #[test]
    fn coruscant_and_the_alliance_headquarters_are_guarded_while_the_side_holds_them() {
        // FUN_004e5870: system 0x90000109's record +0x30 takes 0x10000000
        // while its bit 1 is set; the Alliance's controller +0x28 finds its
        // headquarters (sector +0x2c 0x80000000, system +0x30 0x80000000)
        // and drops it once the system is not its headquarters.
        let mut fx = fixture();
        fx.world.systems[fx.systems[0]].dat_id = crate::ids::DatId::new(0x9000_0109);
        fx.world.systems[fx.systems[3]].is_headquarters = true;
        rebuild(&mut fx);
        let mut m = ModuleC::default();
        with(&mut fx, |io| m.guard(io));
        assert_eq!(m.headquarters, Some(fx.systems[3]));
        assert_eq!(fx.kinds.get(&fx.systems[0]), Some(&GUARDED));
        assert_eq!(fx.kinds.get(&fx.systems[3]), Some(&GUARDED));
        rebuild(&mut fx);
        assert_eq!(guarded(&fx.galaxy), 2);
        // FUN_0041b7e0: controller +4 0x80 while Coruscant is held.
        assert!(holds_coruscant(&fx.galaxy));
        // Kept while it holds; the bits are set once.
        with(&mut fx, |io| m.guard(io));
        assert_eq!(fx.kinds.get(&fx.systems[3]), Some(&GUARDED));
        assert_eq!(fx.kinds.get(&fx.systems[0]), Some(&GUARDED));
        // A headquarters lost to the other side is dropped.
        fx.world.systems[fx.systems[3]].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        rebuild(&mut fx);
        with(&mut fx, |io| m.guard(io));
        assert_eq!(m.headquarters, None);
        assert_eq!(fx.kinds.get(&fx.systems[3]), None);
        fx.world.systems[fx.systems[3]].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Alliance);
        fx.world.systems[fx.systems[3]].is_headquarters = false;
        fx.world.systems[fx.systems[0]].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        rebuild(&mut fx);
        with(&mut fx, |io| m.guard(io));
        assert_eq!(m.headquarters, None);
        assert!(fx.kinds.is_empty());
        rebuild(&mut fx);
        assert!(!holds_coruscant(&fx.galaxy));
    }

    #[test]
    fn a_general_post_takes_the_best_leader_near_the_guarded_system() {
        // FUN_0047a870: the guarded system not yet posted (+0x30
        // 0x10000000 without 0x20000000); FUN_0047a5b0: a free General-able
        // character (0x800002, caps 0x4000000) of leadership 50 or more,
        // ranked by leadership (FUN_00403460, order 1) and nearness to the
        // target (FUN_00403750, order 2); the pick posts the target.
        let mut fx = fixture();
        guard_system(&mut fx, 1);
        let weak = general(&mut fx, 1, 40);
        let near = general(&mut fx, 1, 60);
        let far = general(&mut fx, 3, 70);
        let mut p = CommandPlanner {
            id: 1,
            kind: 1,
            ..CommandPlanner::default()
        };
        with(&mut fx, |io| {
            target(&mut p, io);
            assert!(member(&mut p, io));
        });
        assert_eq!(p.target, Some(fx.systems[1]));
        assert_eq!(p.flags & (KEPT | HELD), KEPT | HELD);
        // Leadership places near 0th and far 1st, nearness far 0th and near
        // 1st: they tie, and the first on a tie, near, wins
        // (FUN_004357b0).
        assert_eq!(p.member, Some(near));
        assert_ne!(p.member, Some(weak));
        assert_ne!(p.member, Some(far));
        assert_eq!(fx.kinds[&fx.systems[1]], GUARDED | POSTED);
        // A second planner finds no open post.
        rebuild(&mut fx);
        let mut q = CommandPlanner {
            id: 2,
            kind: 1,
            ..CommandPlanner::default()
        };
        with(&mut fx, |io| {
            target(&mut q, io);
            assert!(!member(&mut q, io));
        });
        assert_eq!(q.target, None);
    }

    #[test]
    fn the_posted_general_moves_to_the_system_then_takes_its_command() {
        // FUN_004bbc50: a free member away from the target moves there
        // (0x201, FUN_004bc040); there, without a command, it takes slot
        // +0x1c's 0x263; holding one (0x74000f1) it is given nothing.
        let mut fx = fixture();
        let c = general(&mut fx, 0, 60);
        let p = CommandPlanner {
            id: 1,
            kind: 1,
            target: Some(fx.systems[1]),
            member: Some(c),
            ..CommandPlanner::default()
        };
        let moved = with(&mut fx, |io| order(&p, io));
        assert_eq!(
            moved,
            Some(AIAction::DispatchMission {
                kind: MissionKind::Move,
                team: vec![MissionMember::Character(c)],
                decoys: Vec::new(),
                target_system: fx.systems[1],
                target_character: None,
                target_object: None,
            })
        );
        fx.world.characters[c].current_system = Some(fx.systems[1]);
        let command = with(&mut fx, |io| order(&p, io));
        assert_eq!(
            command,
            Some(AIAction::Command {
                character: c,
                kind: CommandKind::General
            })
        );
        fx.world.characters[c].command = CommandKind::General;
        assert_eq!(with(&mut fx, |io| order(&p, io)), None);
        // Type 6 (slot +0x18 returns 6) takes no command.
        fx.world.characters[c].command = CommandKind::None;
        let six = CommandPlanner { kind: 6, ..p };
        assert_eq!(with(&mut fx, |io| order(&six, io)), None);
    }

    #[test]
    fn a_cancelled_post_leaves_and_its_general_gives_up_the_command() {
        // FUN_004bbb00 state 1 with +0x20 0x80000000: FUN_004bbba0 orders
        // 0x260 for a member holding a command and not away (0xa0), gives
        // up the target (FUN_0047a850) and leaves the list (FUN_0041b600).
        let mut fx = fixture();
        fx.kinds.insert(fx.systems[1], GUARDED | POSTED);
        rebuild(&mut fx);
        let c = general(&mut fx, 1, 60);
        fx.world.characters[c].command = CommandKind::General;
        let mut m = ModuleC::default();
        m.planners.push(CommandPlanner {
            id: 4,
            kind: 1,
            state: 1,
            flags: CANCELLED | KEPT | HELD,
            target: Some(fx.systems[1]),
            member: Some(c),
            ..CommandPlanner::default()
        });
        m.current = 4;
        let (order, done) = with(&mut fx, |io| m.step_planner(0, io));
        assert!(done);
        assert_eq!(
            order,
            Some(AIAction::Command {
                character: c,
                kind: CommandKind::None
            })
        );
        assert!(m.planners.is_empty());
        assert_eq!(fx.kinds[&fx.systems[1]], GUARDED);
    }

    #[test]
    fn a_cancelled_post_orders_nothing_for_a_general_without_a_command_or_away() {
        // FUN_004bbba0: 0x260 only for +0x30 0x7000000 without 0xa0.
        let mut fx = fixture();
        let c = general(&mut fx, 1, 60);
        let p = CommandPlanner {
            id: 4,
            kind: 1,
            flags: CANCELLED,
            target: Some(fx.systems[1]),
            member: Some(c),
            ..CommandPlanner::default()
        };
        assert_eq!(with(&mut fx, |io| release(&p, io)), None);
        fx.world.characters[c].command = CommandKind::General;
        fx.world.characters[c].on_mission = true;
        assert_eq!(with(&mut fx, |io| release(&p, io)), None);
    }

    #[test]
    fn a_post_keeps_its_target_while_the_target_still_qualifies() {
        // FUN_0047a480: kept (bit 2) while the record has 0x10000000, even
        // when this planner posted it; FUN_00479ad0: kept while it has
        // 0x40000000 and is the side's.
        let mut fx = fixture();
        fx.kinds.insert(fx.systems[1], GUARDED | POSTED);
        fx.world.systems[fx.systems[2]].dat_id = crate::ids::DatId::new(0x9000_0109);
        rebuild(&mut fx);
        let mut one = CommandPlanner {
            id: 1,
            kind: 1,
            target: Some(fx.systems[1]),
            ..CommandPlanner::default()
        };
        let mut six = CommandPlanner {
            id: 2,
            kind: 6,
            target: Some(fx.systems[2]),
            ..CommandPlanner::default()
        };
        with(&mut fx, |io| {
            target(&mut one, io);
            target(&mut six, io);
        });
        assert_eq!((one.target, one.flags), (Some(fx.systems[1]), KEPT));
        assert_eq!((six.target, six.flags), (Some(fx.systems[2]), KEPT));
        assert_eq!(fx.kinds[&fx.systems[1]], GUARDED | POSTED);
        // A type 6 target that is not Coruscant is given up.
        let mut wrong = CommandPlanner {
            id: 3,
            kind: 6,
            target: Some(fx.systems[1]),
            ..CommandPlanner::default()
        };
        with(&mut fx, |io| target(&mut wrong, io));
        assert_eq!(wrong.target, Some(fx.systems[2]));
        // A type 1 target that lost the guard bit is given up (slot +0x2c
        // clears its 0x20000000) and none other is found.
        fx.kinds.insert(fx.systems[1], POSTED);
        rebuild(&mut fx);
        with(&mut fx, |io| target(&mut one, io));
        assert_eq!((one.target, one.flags), (None, 0));
        assert_eq!(fx.kinds.get(&fx.systems[1]), None);
    }

    #[test]
    fn a_fit_member_is_kept_even_holding_its_command() {
        // FUN_0047a5b0: a member whose record lacks 0x50 stays held; the
        // query (forbidding 0x7000000) would not take him again.
        let mut fx = fixture();
        fx.kinds.insert(fx.systems[1], GUARDED | POSTED);
        rebuild(&mut fx);
        let c = general(&mut fx, 1, 60);
        fx.world.characters[c].command = CommandKind::General;
        let mut p = CommandPlanner {
            id: 1,
            kind: 1,
            target: Some(fx.systems[1]),
            member: Some(c),
            ..CommandPlanner::default()
        };
        assert!(with(&mut fx, |io| member(&mut p, io)));
        assert_eq!((p.member, p.flags), (Some(c), HELD));
        assert_eq!(fx.kinds[&fx.systems[1]], GUARDED | POSTED);
    }

    #[test]
    fn the_emperor_s_post_holds_only_the_emperor_at_a_coruscant_of_the_side() {
        // FUN_00479c00: a member without 0x400 is let go; a target that is
        // not the side's (+0x30 bit 1) is dropped before any pick.
        let mut fx = fixture();
        fx.world.systems[fx.systems[2]].dat_id = crate::ids::DatId::new(0x9000_0109);
        rebuild(&mut fx);
        let c = general(&mut fx, 2, 90);
        let mut p = CommandPlanner {
            id: 1,
            kind: 6,
            target: Some(fx.systems[2]),
            member: Some(c),
            ..CommandPlanner::default()
        };
        assert!(!with(&mut fx, |io| member(&mut p, io)));
        assert_eq!((p.member, p.target), (None, Some(fx.systems[2])));
        // The Emperor himself is let go once a prisoner (0x40).
        let emperor = fx.world.characters.insert(Character {
            dat_id: crate::ids::DatId::new(0x280),
            is_major: true,
            is_alliance: true,
            recruited: true,
            is_captive: true,
            current_system: Some(fx.systems[2]),
            ..Default::default()
        });
        p.member = Some(emperor);
        assert!(!with(&mut fx, |io| member(&mut p, io)));
        assert_eq!(p.member, None);
        // Free, he is kept even where the query (0x74000f1) would not
        // take him again.
        fx.world.characters[emperor].is_captive = false;
        fx.world.characters[emperor].command = CommandKind::General;
        p.member = Some(emperor);
        assert!(with(&mut fx, |io| member(&mut p, io)));
        assert_eq!(p.member, Some(emperor));
        fx.world.characters.remove(emperor);
        fx.world.characters.insert(Character {
            dat_id: crate::ids::DatId::new(0x280),
            is_major: true,
            is_alliance: true,
            recruited: true,
            current_system: Some(fx.systems[0]),
            ..Default::default()
        });
        fx.world.systems[fx.systems[2]].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        rebuild(&mut fx);
        assert!(!with(&mut fx, |io| member(&mut p, io)));
        assert_eq!((p.member, p.target), (None, None));
    }

    #[test]
    fn a_refused_order_ends_the_cycle() {
        // The module's slot +0x18 (FUN_004dd810) after a refused order.
        let mut m = ModuleC {
            state: 5,
            ..ModuleC::default()
        };
        m.refused();
        assert_eq!(m.state, 0);
    }

    #[test]
    fn a_member_captured_is_let_go_and_its_post_reopened() {
        // FUN_0047a5b0: a held member whose record has 0x50 (here the
        // prisoner's 0x40) is released and slot +0x2c clears the target's
        // 0x20000000.
        let mut fx = fixture();
        fx.kinds.insert(fx.systems[1], GUARDED | POSTED);
        rebuild(&mut fx);
        let c = general(&mut fx, 1, 60);
        fx.world.characters[c].is_captive = true;
        let mut p = CommandPlanner {
            id: 1,
            kind: 1,
            target: Some(fx.systems[1]),
            member: Some(c),
            ..CommandPlanner::default()
        };
        let held = with(&mut fx, |io| member(&mut p, io));
        // The post reopens and no other General is at hand.
        assert!(!held);
        assert_eq!((p.member, p.flags), (None, 0));
        assert_eq!(fx.kinds[&fx.systems[1]], GUARDED);
    }

    #[test]
    fn the_module_steps_one_planner_a_cycle_and_its_order_ends_the_step() {
        // FUN_004c84d0: 0 -> 1 (requests) -> 2 -> 4 (choose) -> 5, the
        // planner's states 1, 3, 2, 4 (FUN_004bbb00) one a step, none
        // completing the module until the order, which ends the planner's
        // cycle and the module's (state 0, no raise).
        let mut fx = fixture();
        guard_system(&mut fx, 1);
        let c = general(&mut fx, 0, 60);
        let mut m = ModuleC {
            wanted: 1,
            ..ModuleC::default()
        };
        let mut steps = Vec::new();
        for _ in 0..12 {
            let s = with(&mut fx, |io| m.step(io));
            steps.push((m.state, s.complete, s.raise, s.order.is_some()));
            if s.complete {
                break;
            }
        }
        assert_eq!(
            steps,
            [
                (1, false, false, false),
                (2, false, false, false),
                (4, false, false, false),
                (5, false, false, false),
                (5, false, false, false),
                (5, false, false, false),
                (5, false, false, false),
                (5, false, false, false),
                (0, true, false, true),
            ]
        );
        assert_eq!(m.planners[0].member, Some(c));
        assert_eq!(m.made, [1, 0]);
        // FUN_004c8740 left +0x4c at 0, so the next cycle takes the list's
        // first again.
        assert_eq!((m.current, m.next), (1, 0));
    }

    #[test]
    fn the_emperor_s_post_is_coruscant_and_only_the_emperor_holds_it() {
        // FUN_00479e30: a random sector with +0x24 and +0x2c 0x40000000 and
        // its random system with +0x28 0x80000000 and +0x30 0x40000000
        // (system 0x109); FUN_00479c00: the side's record with the
        // Emperor's 0x400 (query 0x402, FUN_00403460), kept while it has
        // 0x400 and no 0x50. FUN_004bbc50 then moves him there, but gives
        // no command (slot +0x18 returns 6).
        let mut fx = fixture();
        fx.world.systems[fx.systems[2]].dat_id = crate::ids::DatId::new(0x9000_0109);
        rebuild(&mut fx);
        let general = general(&mut fx, 0, 90);
        let emperor = fx.world.characters.insert(Character {
            dat_id: crate::ids::DatId::new(0x280),
            name: "Emperor".into(),
            is_major: true,
            is_alliance: true,
            recruited: true,
            current_system: Some(fx.systems[0]),
            ..Default::default()
        });
        let mut p = CommandPlanner {
            id: 1,
            kind: 6,
            ..CommandPlanner::default()
        };
        with(&mut fx, |io| {
            target(&mut p, io);
            assert!(member(&mut p, io));
        });
        assert_eq!(p.target, Some(fx.systems[2]));
        assert_eq!(p.member, Some(emperor));
        assert_ne!(p.member, Some(general));
        // Type 6 posts no claim.
        assert!(fx.kinds.is_empty());
        let moved = with(&mut fx, |io| order(&p, io));
        assert!(matches!(
            moved,
            Some(AIAction::DispatchMission {
                kind: MissionKind::Move,
                target_system,
                ..
            }) if target_system == fx.systems[2]
        ));
        // The sector's headquarters (+0x28 0x80000000 without +0x30
        // 0x40000000) never passes FUN_00479e30's masks, whatever the draws.
        fx.world.systems[fx.systems[3]].is_headquarters = true;
        rebuild(&mut fx);
        let missions = MissionState::new();
        for tick in 0..16 {
            let mut planner = Planner::new(&fx.world, &missions, &HashSet::new(), true, tick);
            let mut io = InputsC {
                world: &fx.world,
                galaxy: &mut fx.galaxy,
                kinds: &mut fx.kinds,
                planner: &mut planner,
                is_alliance: true,
                sector_planners: &mut [],
            };
            assert_eq!(capital(&mut io), Some(fx.systems[2]), "tick {tick}");
        }
        // Lost Coruscant, in a sector with nothing else of the side's: the
        // target goes and no other is found.
        fx.world.systems[fx.systems[3]].is_headquarters = false;
        fx.world.systems[fx.systems[2]].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        rebuild(&mut fx);
        with(&mut fx, |io| target(&mut p, io));
        assert_eq!((p.target, p.flags & KEPT), (None, 0));
    }

    #[test]
    fn a_planner_whose_member_is_away_waits_its_turn() {
        // FUN_004bba90: a member whose record has 0x20 or 0x80 (en route,
        // on a mission) makes the planner not ready; FUN_004c8740 then
        // ends the cycle without one.
        let mut fx = fixture();
        let c = general(&mut fx, 0, 60);
        fx.world.characters[c].on_mission = true;
        let mut m = ModuleC::default();
        m.planners.push(CommandPlanner {
            id: 1,
            kind: 1,
            member: Some(c),
            ..CommandPlanner::default()
        });
        assert!(!with(&mut fx, |io| m.choose(io)));
        fx.world.characters[c].on_mission = false;
        assert!(with(&mut fx, |io| m.choose(io)));
        assert_eq!(m.current, 1);
    }

    #[test]
    fn the_planners_members_are_the_ones_held_out_of_other_queries() {
        // FUN_0047a5b0 clears a pick's 0x800000, so no query requiring it
        // (the mission planners', FUN_00403460) takes a member; the pass
        // holds what this lists.
        let mut fx = fixture();
        let c = general(&mut fx, 0, 60);
        let m = ModuleC {
            planners: vec![
                CommandPlanner {
                    id: 1,
                    member: Some(c),
                    ..CommandPlanner::default()
                },
                CommandPlanner {
                    id: 2,
                    ..CommandPlanner::default()
                },
            ],
            ..ModuleC::default()
        };
        assert_eq!(m.members().collect::<Vec<_>>(), [c]);
    }

    #[test]
    fn the_cycle_starts_from_the_planner_after_the_last_one_run() {
        // FUN_004c8740: the walk starts at the planner +0x4c names, and
        // +0x4c takes the one after the planner found.
        let mut fx = fixture();
        let mut m = ModuleC::default();
        for id in 1..=3 {
            m.planners.push(CommandPlanner {
                id,
                kind: 1,
                ..CommandPlanner::default()
            });
        }
        m.next = 2;
        assert!(with(&mut fx, |io| m.choose(io)));
        assert_eq!((m.current, m.next), (2, 3));
        assert!(with(&mut fx, |io| m.choose(io)));
        assert_eq!((m.current, m.next), (3, 0));
    }

    #[test]
    fn a_planner_with_no_post_to_fill_ends_the_cycle_without_an_order() {
        // FUN_004bbb00 state 2: slot +0x38 finds no member, the step ends
        // (out 1) and FUN_004c84d0 completes the module with no order.
        let mut fx = fixture();
        let mut m = ModuleC {
            wanted: 1,
            ..ModuleC::default()
        };
        let mut last = None;
        for _ in 0..12 {
            let s = with(&mut fx, |io| m.step(io));
            if s.complete {
                last = Some(s);
                break;
            }
        }
        let last = last.expect("the cycle ends");
        assert_eq!((last.order, last.raise, m.state), (None, false, 0));
        assert_eq!(m.planners[0].state, 0);
    }

    /// Run `f` with module `0xc`'s inputs and `sector_planners`.
    fn with_owners<R>(
        fx: &mut Fx,
        sector_planners: &mut [SectorPlanner],
        f: impl FnOnce(&mut InputsC<'_, '_>) -> R,
    ) -> R {
        let missions = MissionState::new();
        let busy = HashSet::new();
        let mut planner = Planner::new(&fx.world, &missions, &busy, true, 7);
        let mut io = InputsC {
            world: &fx.world,
            galaxy: &mut fx.galaxy,
            kinds: &mut fx.kinds,
            planner: &mut planner,
            is_alliance: true,
            sector_planners,
        };
        f(&mut io)
    }

    /// A sector planner `id` of the fixture's first sector.
    fn owner(fx: &Fx, id: u32) -> SectorPlanner {
        SectorPlanner {
            id,
            sector: Some(fx.sectors[0]),
            kind: 6,
            ..SectorPlanner::default()
        }
    }

    #[test]
    fn a_sector_planner_s_request_makes_an_officer_post_attached_to_it() {
        // FUN_004c8630: from +0x44, the first sector planner with +0xe0 or
        // +0xe4 (FUN_00476cf0) into +0x40, the next into +0x44. FUN_004c86b0:
        // +0xe4 cancelled (FUN_0042ef90 -> FUN_00476de0: +0x54 cleared);
        // +0xe0's type made (FUN_0042ef50) and attached (FUN_00476d10:
        // +0xe0 cleared, +0x54 the sector planner). Only the strategy's
        // types count in controller +0x35c/+0x360 (FUN_0041b5d0).
        let mut fx = fixture();
        let mut owners = [owner(&fx, 2), owner(&fx, 3), owner(&fx, 4)];
        owners[1].movement = [4, 0];
        let mut m = ModuleC {
            state: 2,
            ..ModuleC::default()
        };
        with_owners(&mut fx, &mut owners, |io| {
            m.step(io);
            assert_eq!((m.state, m.sector_found, m.sector_cursor), (3, 3, 4));
            m.step(io);
        });
        assert_eq!(m.state, 4);
        assert_eq!(m.planners.len(), 1);
        let made = m.planners[0].clone();
        assert_eq!((made.kind, made.owner), (4, 3));
        assert_eq!(owners[1].movement, [0, 0]);
        assert_eq!(m.made, [0, 0]);
        assert_eq!(officers_of(&m.planners, 3, 4), 1);

        // From +0x44 (planner 4) no request is left: both are cleared, so
        // the next search starts from the first.
        m.state = 2;
        with_owners(&mut fx, &mut owners, |io| m.step(io));
        assert_eq!((m.state, m.sector_found, m.sector_cursor), (4, 0, 0));

        // A spare (+0xe4) is taken off its owner and cancelled.
        owners[1].movement = [0, made.id];
        m.state = 2;
        with_owners(&mut fx, &mut owners, |io| {
            m.step(io);
            m.step(io);
        });
        assert_eq!(owners[1].movement, [0, 0]);
        assert_eq!(m.planners[0].owner, 0);
        assert_ne!(m.planners[0].flags & CANCELLED, 0);
        assert_eq!(officers_of(&m.planners, 3, 4), 0);

        // The search starts at +0x44: with requests on the first and the
        // last planner and +0x44 naming the last, the last is found.
        owners[0].movement = [3, 0];
        owners[2].movement = [5, 0];
        m.sector_cursor = 4;
        m.state = 2;
        with_owners(&mut fx, &mut owners, |io| m.step(io));
        assert_eq!((m.state, m.sector_found, m.sector_cursor), (3, 4, 0));
    }

    #[test]
    fn the_general_post_takes_a_general_near_the_station_and_boards_the_flagship() {
        // FUN_004bc480: 3 reads the owner's flagship (+0x38) and station
        // (+0x3c) (FUN_004bc5d0, bit 2); 2 picks a free General of
        // leadership 50+ near the station (FUN_0047a050); 4
        // (FUN_004bca20): on a planet of the flagship's system, a Move to
        // the flagship (bit 8); aboard it, its command 0x263 (bit 4).
        let mut fx = fixture();
        let f = crate::ai_planner::tests::fleet(&mut fx, 1, 1);
        let ship = fx.world.fleets[f].capital_ships[0].id;
        let mut owners = [SectorPlanner {
            flagship: Some(ship),
            station: Some(fx.systems[0]),
            ..owner(&fx, 2)
        }];
        let far = general(&mut fx, 3, 55);
        let near = general(&mut fx, 1, 60);
        let mut m = ModuleC {
            planners: vec![CommandPlanner {
                id: 1,
                kind: 4,
                owner: 2,
                ..CommandPlanner::default()
            }],
            current: 1,
            ..ModuleC::default()
        };
        let mut run = |fx: &mut Fx, m: &mut ModuleC| {
            with_owners(fx, &mut owners, |io| {
                let mut last = (None, false);
                for _ in 0..6 {
                    last = m.step_planner(0, io);
                    if last.1 {
                        break;
                    }
                }
                last
            })
        };
        let (order, done) = run(&mut fx, &mut m);
        assert!(done);
        let p = &m.planners[0];
        assert_eq!((p.flagship, p.station), (Some(ship), Some(fx.systems[0])));
        assert_ne!(p.flags & KEPT, 0);
        // The merged pick (FUN_004357b0): the nearer General leads both
        // the leadership list and the distance list (FUN_00403750).
        let member = p.member.expect("a General is held");
        assert_eq!(member, near);
        let _ = far;
        fx.world.characters[member].current_system = Some(fx.systems[1]);
        m.planners[0].member = Some(member);
        let _ = order;
        let (order, _) = run(&mut fx, &mut m);
        assert_eq!(
            order,
            Some(AIAction::BoardFleet {
                character: member,
                fleet: f
            })
        );
        assert_ne!(m.planners[0].flags & IN_PLACE, 0);

        // Aboard the flagship's fleet: the command, then nothing.
        fx.world.characters[member].current_fleet = Some(f);
        fx.world.fleets[f].characters.push(member);
        let (order, _) = run(&mut fx, &mut m);
        assert_eq!(
            order,
            Some(AIAction::Command {
                character: member,
                kind: CommandKind::General
            })
        );
        assert_eq!(m.planners[0].flags & (IN_PLACE | RANKED), IN_PLACE | RANKED);
        fx.world.characters[member].command = CommandKind::General;
        let (order, _) = run(&mut fx, &mut m);
        assert_eq!(order, None);
        assert_eq!(m.planners[0].flags & (IN_PLACE | RANKED), IN_PLACE | RANKED);
    }

    #[test]
    fn an_officer_away_from_the_flagship_s_system_goes_to_the_station_and_waits_there() {
        // FUN_004bca20: on a planet of another system, a Move to the station
        // (+0x48), or waiting there (bit 0x10); with no flagship, the same.
        let mut fx = fixture();
        let f = crate::ai_planner::tests::fleet(&mut fx, 1, 1);
        let ship = fx.world.fleets[f].capital_ships[0].id;
        let c = general(&mut fx, 3, 60);
        let mut p = CommandPlanner {
            id: 1,
            kind: 4,
            owner: 2,
            member: Some(c),
            flagship: Some(ship),
            station: Some(fx.systems[0]),
            ..CommandPlanner::default()
        };
        let moved = with(&mut fx, |io| board(&mut p, io));
        assert!(matches!(
            moved,
            Some(AIAction::DispatchMission {
                kind: MissionKind::Move,
                target_system,
                ..
            }) if target_system == fx.systems[0]
        ));
        fx.world.characters[c].current_system = Some(fx.systems[0]);
        assert_eq!(with(&mut fx, |io| board(&mut p, io)), None);
        assert_ne!(p.flags & WAITING, 0);
        p.flags = 0;
        p.flagship = None;
        assert_eq!(with(&mut fx, |io| board(&mut p, io)), None);
        assert_ne!(p.flags & WAITING, 0);
    }

    #[test]
    fn the_strategy_cancels_only_its_own_posts() {
        // FUN_004eaaa0: a cancellation names the first planner whose slot
        // +0xc returns 2 (the strategy's types 1 and 6, FUN_0040f340 gives
        // 1 for types 3 to 5) and whose +0x20 lacks bit 2.
        let mut m = ModuleC {
            planners: vec![
                CommandPlanner {
                    id: 1,
                    kind: 4,
                    owner: 2,
                    ..CommandPlanner::default()
                },
                CommandPlanner {
                    id: 2,
                    kind: 1,
                    ..CommandPlanner::default()
                },
            ],
            made: [1, 0],
            ..ModuleC::default()
        };
        m.strategy_requests(0, 0, false, true);
        assert_eq!(m.cancel, 2);
        m.planners.truncate(1);
        m.strategy_requests(0, 0, false, true);
        assert_eq!(m.cancel, 0);
    }

    #[test]
    fn the_sector_links_carry_each_officer_post_and_the_cursor_back() {
        // ModuleC::links / restore_links: what the save keeps apart (v130)
        // of +0x54, +0x28 and +0x48 per planner and the module's +0x40 and
        // +0x44; a planner with none of them is left out.
        let station = Some(SystemKey::default());
        let post = |id, owner, flagship, station| CommandPlanner {
            id,
            kind: 4,
            owner,
            flagship,
            station,
            ..CommandPlanner::default()
        };
        let m = ModuleC {
            planners: vec![
                post(1, 2, None, None),
                post(2, 0, Some(9), None),
                post(3, 0, None, station),
                post(4, 0, None, None),
            ],
            sector_cursor: 5,
            sector_found: 6,
            ..ModuleC::default()
        };
        let links = m.links();
        let ids: Vec<u32> = links.posts.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
        assert_eq!((links.cursor, links.found), (5, 6));
        let mut bare = m.clone();
        for p in &mut bare.planners {
            *p = post(p.id, 0, None, None);
        }
        bare.sector_cursor = 0;
        bare.sector_found = 0;
        bare.restore_links(&links);
        assert_eq!(bare, m);
    }

    #[test]
    fn the_admiral_and_commander_posts_take_their_own_officers_and_commands() {
        // FUN_0047a3c0 (+0x50 0x1000000, slot +0x1c 0x262 Admiral) and
        // FUN_00479f70 (0x2000000, 0x261 Commander): the member query asks
        // for the type's capability (FUN_00401d20), and aboard the
        // flagship the member takes the type's command (FUN_004bca20).
        let mut fx = fixture();
        let f = crate::ai_planner::tests::fleet(&mut fx, 1, 1);
        let ship = fx.world.fleets[f].capital_ships[0].id;
        let leader = general(&mut fx, 1, 90);
        let minor = |fx: &mut Fx, admiral: bool| {
            let c = general(fx, 1, 60);
            let ch = &mut fx.world.characters[c];
            ch.can_be_general = false;
            ch.can_be_admiral = admiral;
            ch.can_be_commander = !admiral;
            c
        };
        let admiral = minor(&mut fx, true);
        let commander = minor(&mut fx, false);
        let mut owners = [SectorPlanner {
            flagship: Some(ship),
            station: Some(fx.systems[1]),
            ..owner(&fx, 2)
        }];
        for (kind, member, command) in [
            (3, admiral, CommandKind::Admiral),
            (5, commander, CommandKind::Commander),
        ] {
            let mut p = CommandPlanner {
                id: 1,
                kind,
                owner: 2,
                flagship: Some(ship),
                station: Some(fx.systems[1]),
                ..CommandPlanner::default()
            };
            assert!(
                with_owners(&mut fx, &mut owners, |io| officer(&mut p, io)),
                "type {kind}"
            );
            assert_eq!(p.member, Some(member), "type {kind}");
            fx.world.characters[member].current_fleet = Some(f);
            let order = with_owners(&mut fx, &mut owners, |io| board(&mut p, io));
            assert_eq!(
                order,
                Some(AIAction::Command {
                    character: member,
                    kind: command
                })
            );
        }
        // The better leader, a General only, is passed over by both.
        let _ = leader;
    }

    #[test]
    fn a_lost_flagship_sends_no_officer_and_is_no_place_to_stand() {
        // FUN_004bca20: the target (+0x28) is a capital ship; once it is
        // gone (FUN_004f5940 finds no record) no order is given, and a
        // member on a planet does not stand at it.
        let mut fx = fixture();
        let f = crate::ai_planner::tests::fleet(&mut fx, 1, 1);
        let ship = fx.world.fleets[f].capital_ships[0].id;
        fx.world.fleets[f].capital_ships[0].alive = false;
        let c = general(&mut fx, 1, 60);
        let mut p = CommandPlanner {
            id: 1,
            kind: 4,
            owner: 2,
            member: Some(c),
            flagship: Some(ship),
            station: Some(fx.systems[1]),
            ..CommandPlanner::default()
        };
        assert_eq!(with(&mut fx, |io| board(&mut p, io)), None);
        assert_eq!(p.flags & (IN_PLACE | RANKED), 0);
    }

    #[test]
    fn reading_the_owner_drops_the_kept_bit_and_keeps_the_others() {
        // FUN_004bc5d0: +0x20 bit 2 cleared first (& 0xfffffffd), set again
        // only with both a flagship and a station; the other bits stay.
        let mut fx = fixture();
        let mut owners = [owner(&fx, 2)];
        let mut p = CommandPlanner {
            id: 1,
            kind: 4,
            owner: 2,
            flags: HELD | KEPT | IN_PLACE,
            ..CommandPlanner::default()
        };
        with_owners(&mut fx, &mut owners, |io| read_owner(&mut p, io));
        assert_eq!(p.flags, HELD | IN_PLACE);
        // An owner gone: the same.
        p.flags = HELD | KEPT;
        p.owner = 9;
        with_owners(&mut fx, &mut owners, |io| read_owner(&mut p, io));
        assert_eq!(p.flags, HELD);
    }

    #[test]
    fn a_cancellation_names_one_planner_and_none_for_id_zero() {
        // FUN_0042ef90 on an id: that planner only; FUN_005f5500(0) finds
        // none.
        let post = |id| CommandPlanner {
            id,
            kind: 3,
            owner: 2,
            ..CommandPlanner::default()
        };
        let mut planners = vec![post(1), post(2)];
        cancel(&mut planners, 0);
        assert_eq!((planners[0].flags, planners[1].flags), (0, 0));
        cancel(&mut planners, 2);
        assert_eq!((planners[0].flags, planners[0].owner), (0, 2));
        assert_eq!((planners[1].flags, planners[1].owner), (CANCELLED, 0));
    }

    #[test]
    fn only_the_general_post_admits_an_officer_reserved_for_diplomacy() {
        // FUN_0047a050 forbids 0x474000f1, FUN_004bc6d0 0x4f4000f1: a
        // record reserved for research or diplomacy (0x8000000,
        // FUN_00402230: diplomacy above 79) may be a type 4's General but
        // not a type 3's Admiral; one holding a command (0x7000000) is
        // neither's.
        let mut fx = fixture();
        let c = general(&mut fx, 1, 60);
        fx.world.characters[c].can_be_admiral = true;
        fx.world.characters[c].diplomacy = SkillPair {
            base: 90,
            variance: 0,
        };
        let mut owners = [SectorPlanner {
            station: Some(fx.systems[1]),
            ..owner(&fx, 2)
        }];
        let mut pick = |fx: &mut Fx, kind| {
            let mut p = CommandPlanner {
                id: 1,
                kind,
                owner: 2,
                station: Some(fx.systems[1]),
                ..CommandPlanner::default()
            };
            with_owners(fx, &mut owners, |io| officer(&mut p, io));
            p.member
        };
        assert_eq!(pick(&mut fx, 4), Some(c));
        assert_eq!(pick(&mut fx, 3), None);
        fx.world.characters[c].diplomacy.base = 0;
        assert_eq!(pick(&mut fx, 3), Some(c));
        fx.world.characters[c].command = CommandKind::General;
        assert_eq!(pick(&mut fx, 3), None);
        assert_eq!(pick(&mut fx, 4), None);
    }

    #[test]
    fn an_officer_aboard_another_fleet_boards_the_flagship_only_beside_it() {
        // FUN_004bca20, a member aboard another ship: neither ship en route
        // (0x800), the fleets in one system: a Move to the flagship (bit
        // 8); in different systems: a Move to the station (+0x48); its own
        // ship en route: nothing.
        let mut fx = fixture();
        let target = crate::ai_planner::tests::fleet(&mut fx, 1, 1);
        let other = crate::ai_planner::tests::fleet(&mut fx, 1, 1);
        let ship = fx.world.fleets[target].capital_ships[0].id;
        let c = general(&mut fx, 1, 60);
        fx.world.characters[c].current_fleet = Some(other);
        let station = Some(fx.systems[0]);
        let post = |c| CommandPlanner {
            id: 1,
            kind: 4,
            owner: 2,
            member: Some(c),
            flagship: Some(ship),
            station,
            ..CommandPlanner::default()
        };
        let mut p = post(c);
        assert_eq!(
            with(&mut fx, |io| board(&mut p, io)),
            Some(AIAction::BoardFleet {
                character: c,
                fleet: target
            })
        );
        assert_ne!(p.flags & IN_PLACE, 0);

        // The flagship en route: nothing.
        let mut p = post(c);
        fx.galaxy.fleets.get_mut(&target).unwrap().ships[0].flags |= 0x800;
        assert_eq!(with(&mut fx, |io| board(&mut p, io)), None);
        assert_eq!(p.flags & IN_PLACE, 0);
        fx.galaxy.fleets.get_mut(&target).unwrap().ships[0].flags &= !0x800;

        // Its own ship en route: nothing.
        fx.galaxy.fleets.get_mut(&other).unwrap().ships[0].flags |= 0x800;
        assert_eq!(with(&mut fx, |io| board(&mut p, io)), None);
        fx.galaxy.fleets.get_mut(&other).unwrap().ships[0].flags &= !0x800;

        // In another system: to the station.
        fx.world.fleets[other].location = fx.systems[3];
        let order = with(&mut fx, |io| board(&mut p, io));
        assert!(matches!(
            order,
            Some(AIAction::DispatchMission { kind: MissionKind::Move, target_system, .. })
                if target_system == fx.systems[0]
        ));
    }

    #[test]
    fn an_officer_post_keeps_a_fit_member_and_lets_an_injured_one_go() {
        // FUN_004bc6d0: +0x20 & ~0x1d first (bit 2 stays); a member without
        // 0x50 is kept (1), at the flagship 8, with a rank 4 as well; an
        // injured one (0x10) is let go, and with no other candidate the
        // step ends (0). An owner not in the list picks nobody.
        let mut fx = fixture();
        let f = crate::ai_planner::tests::fleet(&mut fx, 1, 1);
        let ship = fx.world.fleets[f].capital_ships[0].id;
        let c = general(&mut fx, 1, 60);
        let mut owners = [owner(&fx, 2)];
        let mut p = CommandPlanner {
            id: 1,
            kind: 4,
            owner: 2,
            member: Some(c),
            flagship: Some(ship),
            flags: KEPT | RANKED | IN_PLACE | WAITING,
            ..CommandPlanner::default()
        };
        assert!(with_owners(&mut fx, &mut owners, |io| officer(&mut p, io)));
        assert_eq!(p.flags, KEPT | HELD);
        fx.world.characters[c].current_fleet = Some(f);
        assert!(with_owners(&mut fx, &mut owners, |io| officer(&mut p, io)));
        assert_eq!(p.flags, KEPT | HELD | IN_PLACE);
        fx.world.characters[c].command = CommandKind::General;
        assert!(with_owners(&mut fx, &mut owners, |io| officer(&mut p, io)));
        assert_eq!(p.flags, KEPT | HELD | IN_PLACE | RANKED);

        fx.world.characters[c].injury = 10;
        assert!(!with_owners(&mut fx, &mut owners, |io| officer(&mut p, io)));
        assert_eq!((p.member, p.flags), (None, KEPT));

        // A fit candidate, but the planner's owner is gone.
        let other = general(&mut fx, 0, 70);
        p.owner = 9;
        assert!(!with_owners(&mut fx, &mut owners, |io| officer(&mut p, io)));
        p.owner = 2;
        assert!(with_owners(&mut fx, &mut owners, |io| officer(&mut p, io)));
        assert_eq!(p.member, Some(other));
    }

    #[test]
    fn the_owner_s_flagship_and_station_both_keep_the_target() {
        // FUN_004bc5d0: bit 2 needs both the flagship (a capital ship) and
        // the station (a system) of the owner +0x54 itself.
        let mut fx = fixture();
        let mut owners = [SectorPlanner {
            flagship: Some(7),
            station: Some(fx.systems[0]),
            ..owner(&fx, 2)
        }];
        let read = |fx: &mut Fx, owners: &mut [SectorPlanner], owner| {
            let mut p = CommandPlanner {
                id: 1,
                kind: 3,
                owner,
                ..CommandPlanner::default()
            };
            with_owners(fx, owners, |io| read_owner(&mut p, io));
            (p.flags & KEPT, p.flagship)
        };
        assert_eq!(read(&mut fx, &mut owners, 2), (KEPT, Some(7)));
        assert_eq!(read(&mut fx, &mut owners, 9), (0, None));
        owners[0].flagship = None;
        assert_eq!(read(&mut fx, &mut owners, 2), (0, None));
    }

    #[test]
    fn an_officer_s_step_ends_back_at_state_zero() {
        // FUN_004bc480: an ended step (*out set) resets +0x1c to 0.
        let mut fx = fixture();
        let mut owners = [owner(&fx, 2)];
        let mut m = ModuleC {
            planners: vec![CommandPlanner {
                id: 1,
                kind: 3,
                owner: 2,
                state: 2,
                ..CommandPlanner::default()
            }],
            current: 1,
            ..ModuleC::default()
        };
        let (_, done) = with_owners(&mut fx, &mut owners, |io| m.step_planner(0, io));
        assert!(done);
        assert_eq!(m.planners[0].state, 0);
    }

    #[test]
    fn an_officer_rides_the_ship_he_is_seated_on() {
        // FUN_004bca20 reads the member's own ship (record +0x38): seated
        // on a ship en route (0x800), he is given nothing even when the
        // fleet's first ship is at rest.
        let mut fx = fixture();
        let target = crate::ai_planner::tests::fleet(&mut fx, 1, 1);
        let other = crate::ai_planner::tests::fleet(&mut fx, 1, 2);
        let ship = fx.world.fleets[target].capital_ships[0].id;
        let seat = fx.world.fleets[other].capital_ships[1].id;
        let c = general(&mut fx, 1, 60);
        fx.world.characters[c].current_fleet = Some(other);
        fx.world.characters[c].ship = Some(seat);
        fx.galaxy.fleets.get_mut(&other).unwrap().ships[1].flags |= 0x800;
        let mut p = CommandPlanner {
            id: 1,
            kind: 4,
            owner: 2,
            member: Some(c),
            flagship: Some(ship),
            station: Some(fx.systems[0]),
            ..CommandPlanner::default()
        };
        assert_eq!(with(&mut fx, |io| board(&mut p, io)), None);
    }
}
