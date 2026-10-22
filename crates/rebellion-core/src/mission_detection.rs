//! The mission detection manager (`FUN_00547f60`): after each phase change a
//! mission's decoys draw off defenders, the defenders try to detect the team,
//! a disloyal member may betray it, and a detected team is exposed
//! (`ghidra/notes/decoy-roll.md`, "Re-read for the port").
//!
//! The run reads the world and returns its changes as [`MissionEffect`]s;
//! the mission's own end code and resign requests change in place.

use std::collections::{HashMap, HashSet};

use crate::dat::Faction;
use crate::ids::{CharacterKey, DatId, FleetKey, SystemKey, TroopKey};
use crate::missions::{
    member_location_kind, member_skill, running_end_code, ActiveMission, MemberTransit,
    MissionEffect, MissionFaction, MissionMember,
};
use crate::uprising::{
    roll_injury, Draws, UprisingState, GNPRTB_INJURY_BASE, GNPRTB_INJURY_MIN_CHANCE,
    GNPRTB_INJURY_SPREAD,
};
use crate::character_command::{self, CommandKind};
use crate::world::{GameWorld, Skill};

/// GNPRTB 3588 (`DAT_006bb710`): the percent of the detector's officer's
/// espionage the decoy roll subtracts (`FUN_0055e410`).
const GNPRTB_DECOY_OFFICER: u16 = 3588;
/// GNPRTB 3588 as shipped.
const SHIPPED_DECOY_OFFICER: i64 = 35;
/// GNPRTB 3589 (`DAT_006bb70c`): the percent of the detector's officer's
/// espionage the FOILTB roll subtracts (`FUN_0055e470`).
const GNPRTB_FOIL_OFFICER: u16 = 3589;
/// GNPRTB 3589 as shipped.
const SHIPPED_FOIL_OFFICER: i64 = 35;
/// GNPRTB 3584 (`DAT_006bb714`): the detection offset.
const GNPRTB_DETECTION_OFFSET: u16 = 3584;
/// GNPRTB 3584 as shipped.
const SHIPPED_DETECTION_OFFSET: i64 = -1;
/// GNPRTB 2565 as shipped: the injury roll's least chance.
const SHIPPED_INJURY_MIN_CHANCE: i64 = 1;
/// GNPRTB 2566 as shipped: the injury roll's base.
const SHIPPED_INJURY_BASE: i64 = 1;
/// GNPRTB 3081 (`DAT_006bb758`): Luke's Force experience for an evade
/// (`FUN_0056f8b0`).
const GNPRTB_LUKE_EVADE_FORCE: u16 = 3081;
/// GNPRTB 3082 (`DAT_006bb744`): Leia's (`FUN_0056ffa0`).
const GNPRTB_LEIA_EVADE_FORCE: u16 = 3082;
/// Both ship as 1.
const SHIPPED_EVADE_FORCE: i64 = 1;
/// Luke's record (`0x32000242`), whose class `0x00665fd8` overrides slot
/// `+0x208` with `FUN_0056f8b0`.
const LUKE: u32 = 0x242;
/// Leia's record (`0x31000241`), class `0x006663e0`, `FUN_0056ffa0`.
const LEIA: u32 = 0x241;
/// GNPRTB 2567 as shipped: the injury roll's spread.
const SHIPPED_INJURY_SPREAD: i64 = 29;
/// Vader's record (`0x35000281`, `FUN_0058a9e0`).
const VADER: u32 = 0x281;
/// The Emperor's record (`0x34000280`, `FUN_00560200`).
const EMPEROR: u32 = 0x280;

/// GNPRTB 3586 (`DAT_006bb704`): the least Force of a defender in Force
/// detection (`FUN_0055e590` from `FUN_00588840`).
const GNPRTB_FORCE_DETECTOR: u16 = 3586;
/// GNPRTB 3586 as shipped (`FUN_0055e340`).
const SHIPPED_FORCE_DETECTOR: i64 = 60;
/// GNPRTB 3587 (`DAT_006bb718`): the Force detection offset
/// (`FUN_0055e4d0`).
const GNPRTB_FORCE_DETECTION_OFFSET: u16 = 3587;
/// GNPRTB 3587 as shipped (`FUN_0055e340`).
const SHIPPED_FORCE_DETECTION_OFFSET: i64 = -100;
/// GNPRTB 2571 (`DAT_006b90d0`): the duel injury's least chance
/// (`FUN_0053ea70`).
const GNPRTB_DUEL_MIN_CHANCE: u16 = 2571;
/// GNPRTB 2571 as shipped.
const SHIPPED_DUEL_MIN_CHANCE: i64 = 1;
/// GNPRTB 2572 (`DAT_006b90d4`): the duel injury's base (`FUN_0053ea70`).
const GNPRTB_DUEL_BASE: u16 = 2572;
/// GNPRTB 2572 as shipped.
const SHIPPED_DUEL_BASE: i64 = 1;
/// GNPRTB 2573 (`DAT_006b90b8`): the duel injury's spread
/// (`FUN_0053ea70`).
const GNPRTB_DUEL_SPREAD: u16 = 2573;
/// GNPRTB 2573 as shipped.
const SHIPPED_DUEL_SPREAD: i64 = 29;
/// GNPRTB 2578 (`DAT_006b9060`): the base combat a duel's victor gains
/// (`FUN_005340a0` from `FUN_00560200`).
const GNPRTB_DUEL_COMBAT_GAIN: u16 = 2578;
/// GNPRTB 2578 as shipped.
const SHIPPED_DUEL_COMBAT_GAIN: i64 = 1;
/// GNPRTB 3077 (`DAT_006bb74c`): below this Force, Luke's first meeting
/// with Vader wounds him (`FUN_00560200`).
const GNPRTB_LUKE_WOUND_FORCE: u16 = 3077;
/// GNPRTB 3077 as shipped.
const SHIPPED_LUKE_WOUND_FORCE: i64 = 80;
/// GNPRTB 3079 (`DAT_006bb734`): the wound's base (`FUN_0055e810`).
const GNPRTB_LUKE_WOUND_BASE: u16 = 3079;
/// GNPRTB 3079 as shipped.
const SHIPPED_LUKE_WOUND_BASE: i64 = 1;
/// GNPRTB 3080 (`DAT_006bb750`): the wound's spread (`FUN_0055e810`).
const GNPRTB_LUKE_WOUND_SPREAD: u16 = 3080;
/// GNPRTB 3080 as shipped.
const SHIPPED_LUKE_WOUND_SPREAD: i64 = 99;

/// Table ids 10..13 of `FUN_0058b420` (`uprising-incident.md`).
const TDECOYTB: &str = "TDECOYTB";
const FDECOYTB: &str = "FDECOYTB";
const FOILTB: &str = "FOILTB";
const RLEVADTB: &str = "RLEVADTB";

/// The phase the betrayal phase runs in: slot `+0x1c8` is `FUN_00592500`,
/// `+0x68 == 9`, for every agent class.
const PHASE_BETRAYAL: u8 = 9;

/// port: a mission's draws for one tick event, taken by the detection run
/// and the phase 10 roll. The original draws each roll inline from its
/// global generator; the port seeds a SplitMix64 per mission and tick from
/// one caller roll, so the caller's budget stays fixed.
pub(crate) struct MissionRng {
    state: u64,
}

impl MissionRng {
    /// port: the seed spreads the roll over 53 bits and mixes in the
    /// mission id with the SplitMix64 increment.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a roll in [0, 1) scaled to 53 bits fits u64"
    )]
    pub(crate) fn seeded(roll: f64, mission_id: u64) -> Self {
        let bits = (roll.clamp(0.0, 1.0) * f64::from(1u32 << 26) * f64::from(1u32 << 27)) as u64;
        Self {
            state: bits ^ mission_id.wrapping_mul(0x9e37_79b9_7f4a_7c15),
        }
    }

    /// port: one SplitMix64 step.
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

impl Draws for MissionRng {
    /// `FUN_0053e290(n)`: uniform over `0..=n`, or `-(0..=-n)` for negative
    /// `n`.
    fn draw(&mut self, n: i32) -> i32 {
        let span = u64::from(n.unsigned_abs());
        let value = i32::try_from(self.next_u64() % (span + 1)).unwrap_or(i32::MAX);
        if n < 0 {
            -value
        } else {
            value
        }
    }

    /// `FUN_0053e2f0(percent)`: a draw over `0..=99` below `percent`.
    fn chance(&mut self, percent: i32) -> bool {
        self.draw(99) < percent
    }
}

/// Which members and defenders a run counts (manager `+0x18..+0x30`).
///
/// port: the setup `FUN_00589a40` sets the ship and fighter flags
/// (`+0x28`, `+0x2c`) only with the fleet flag and the regiment flag
/// (`+0x30`) only with the system flag, so the port folds each into its walk
/// flag.
#[derive(Debug, Default, Clone, Copy)]
struct Flags {
    /// `+0x18`: members standing at a system.
    at_system: bool,
    /// `+0x1c`: members in a fleet.
    in_fleet: bool,
    /// `+0x20` with `+0x30`: walk the system's regiments.
    system: bool,
    /// `+0x24` with `+0x28` and `+0x2c`: walk the fleets' ships and fighters.
    fleets: bool,
}

/// The kinds one walk asks for (`FUN_00587640` arguments 2..4).
#[derive(Debug, Clone, Copy)]
struct Kinds {
    ships: bool,
    fighters: bool,
    regiments: bool,
}

const ALL: Kinds = Kinds {
    ships: true,
    fighters: true,
    regiments: true,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DefenderId {
    Regiment(TroopKey),
    Ship(FleetKey, usize),
    /// A fleet's fighter entry and the squadron within its count.
    Fighter(FleetKey, usize, u32),
    /// The planet's fighter entry and the squadron within its count.
    PlanetFighter(usize, u32),
}

/// A detector (`ghidra/notes/decoy-roll.md`, "Officer rank and detector
/// slots").
#[derive(Debug, Clone, Copy)]
struct Defender {
    id: DefenderId,
    in_fleet: bool,
    /// Slot `+0x1c4`: the class `detection`.
    detection: i32,
    side: Faction,
}

impl Defender {
    /// Slot `+0x1bc`: the rank of the officer that backs the detector, an
    /// Admiral for a capital ship (`0x0043c1f0`), a Commander for a fighter
    /// squadron (`FUN_0040f340`), a General for a regiment (`FUN_00526f00`).
    const fn rank(&self) -> CommandKind {
        match self.id {
            DefenderId::Ship(..) => CommandKind::Admiral,
            DefenderId::Fighter(..) | DefenderId::PlanetFighter(..) => CommandKind::Commander,
            DefenderId::Regiment(_) => CommandKind::General,
        }
    }

    /// The detector's officer (`FUN_00588b90`, `FUN_00588a90`,
    /// `FUN_005888f0`): its fleet's holder of the rank (`FUN_004fd790`), or,
    /// for one the walk hands no fleet, the system's (`FUN_00509330` on the
    /// pool's system `+8`).
    fn officer(&self, world: &GameWorld, location: SystemKey) -> Option<CharacterKey> {
        match self.id {
            DefenderId::Ship(fleet, _) | DefenderId::Fighter(fleet, ..) => {
                character_command::fleet_holder(world, fleet, self.rank())
            }
            DefenderId::Regiment(_) | DefenderId::PlanetFighter(..) => {
                character_command::system_holder(world, location, self.rank())
            }
        }
    }
}

pub(crate) fn faction_of(is_alliance: bool) -> Faction {
    if is_alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    }
}

/// `FUN_00587640`: the defenders at `location` in walk order.
///
/// The system's squadrons need the system flag and the fighter flag, which
/// the port folds into the fleet flag ([`Flags`]); they come before its
/// regiments.
///
/// port: a fleet's fighters follow all of its ships where the original
/// walks each ship's fighters and regiments after the ship; fleets carry no
/// regiments. A fleet in hyperspace reads its last orbit.
fn walk(world: &GameWorld, location: SystemKey, flags: Flags, kinds: Kinds) -> Vec<Defender> {
    let mut defenders = Vec::new();
    let as_i32 = |value: u32| i32::try_from(value).unwrap_or(i32::MAX);
    if flags.system && flags.fleets && kinds.fighters {
        if let Some(system) = world.systems.get(location) {
            for (index, entry) in system.fighters.iter().enumerate() {
                let Some(class) = world.fighter_classes.get(entry.class) else {
                    continue;
                };
                for squadron in 0..entry.count {
                    defenders.push(Defender {
                        id: DefenderId::PlanetFighter(index, squadron),
                        in_fleet: false,
                        detection: as_i32(class.detection),
                        side: faction_of(class.is_alliance),
                    });
                }
            }
        }
    }
    if flags.system && kinds.regiments {
        if let Some(system) = world.systems.get(location) {
            for &key in &system.ground_units {
                let Some(troop) = world.troops.get(key) else {
                    continue;
                };
                let detection = world
                    .troop_classes
                    .get(&troop.class_dat_id)
                    .map_or(0, |class| class.detection);
                defenders.push(Defender {
                    id: DefenderId::Regiment(key),
                    in_fleet: false,
                    detection: as_i32(detection),
                    side: faction_of(troop.is_alliance),
                });
            }
        }
    }
    if flags.fleets {
        for (fleet_key, fleet) in &world.fleets {
            if fleet.location != location {
                continue;
            }
            let side = faction_of(fleet.is_alliance);
            if kinds.ships {
                for (index, ship) in fleet.capital_ships.iter().enumerate() {
                    if !ship.alive {
                        continue;
                    }
                    let detection = world
                        .capital_ship_classes
                        .get(ship.class)
                        .map_or(0, |class| class.detection);
                    defenders.push(Defender {
                        id: DefenderId::Ship(fleet_key, index),
                        in_fleet: true,
                        detection: as_i32(detection),
                        side,
                    });
                }
            }
            if kinds.fighters {
                for (index, entry) in fleet.fighters.iter().enumerate() {
                    let detection = world
                        .fighter_classes
                        .get(entry.class)
                        .map_or(0, |class| class.detection);
                    for squadron in 0..entry.count {
                        defenders.push(Defender {
                            id: DefenderId::Fighter(fleet_key, index, squadron),
                            in_fleet: true,
                            detection: as_i32(detection),
                            side,
                        });
                    }
                }
            }
        }
    }
    defenders
}

/// `FUN_0053e340` over a whole table: `Some(outcome)` when the table exists.
///
/// port: a present table always yields a row (`MstbTable::step_lookup`); an
/// absent table is data the port was not given, reported as `None`.
fn table_roll(world: &GameWorld, table: &str, x: i32, rng: &mut MissionRng) -> Option<bool> {
    let table = world.mission_tables.get(table)?;
    let value = i32::try_from(table.step_lookup(x)).unwrap_or(i32::MAX);
    Some(rng.chance(value))
}

/// `FUN_005349e0`'s roll: RLEVADTB at `x`, the member's effective combat
/// less its captor's; `Some(true)` evades, `None` without the table.
pub(crate) fn rlevadtb_evades(world: &GameWorld, x: i32, rng: &mut MissionRng) -> Option<bool> {
    table_roll(world, RLEVADTB, x, rng)
}

/// Slot `+0x2e4` `FUN_004ef5f0`: the injury roll `FUN_0053e990` for a
/// character of effective `combat`.
pub(crate) fn injury_roll(world: &GameWorld, combat: i32, rng: &mut MissionRng) -> Option<i32> {
    roll_injury(
        combat,
        param(world, GNPRTB_INJURY_MIN_CHANCE, SHIPPED_INJURY_MIN_CHANCE),
        param(world, GNPRTB_INJURY_SPREAD, SHIPPED_INJURY_SPREAD),
        param(world, GNPRTB_INJURY_BASE, SHIPPED_INJURY_BASE),
        rng,
    )
}

/// GNPRTB 2574 (`DAT_006b9070`): the assassination injury's chance and its
/// first spread (`FUN_0053eae0`).
const GNPRTB_ASSASSINATION_INJURY_CHANCE: u16 = 2574;
/// GNPRTB 2574 as shipped.
const SHIPPED_ASSASSINATION_INJURY_CHANCE: i64 = 100;
/// GNPRTB 2575 (`DAT_006b9090`): its base.
const GNPRTB_ASSASSINATION_INJURY_BASE: u16 = 2575;
/// GNPRTB 2575 as shipped.
const SHIPPED_ASSASSINATION_INJURY_BASE: i64 = 1;
/// GNPRTB 2576 (`DAT_006b90cc`): its second spread.
const GNPRTB_ASSASSINATION_INJURY_SPREAD: u16 = 2576;
/// GNPRTB 2576 as shipped.
const SHIPPED_ASSASSINATION_INJURY_SPREAD: i64 = 29;
/// GNPRTB 2577 (`DAT_006b90c8`): the chance an injury kills a minor
/// character (`FUN_0053eb30`).
const GNPRTB_MINOR_INJURY_DEATH: u16 = 2577;
/// GNPRTB 2577 as shipped.
const SHIPPED_MINOR_INJURY_DEATH: i64 = 25;

/// The assassination's injury, slot `+0x2ec` (`FUN_004ef690` ->
/// `FUN_0053eae0`): with a chance of GNPRTB 2574, `rand(0..=2574) +
/// rand(0..=2576) + 2575`.
pub(crate) fn assassination_injury(world: &GameWorld, draws: &mut impl Draws) -> Option<i32> {
    let spread = param(
        world,
        GNPRTB_ASSASSINATION_INJURY_CHANCE,
        SHIPPED_ASSASSINATION_INJURY_CHANCE,
    );
    if !draws.chance(spread) {
        return None;
    }
    let first = draws.draw(spread);
    let second = draws.draw(param(
        world,
        GNPRTB_ASSASSINATION_INJURY_SPREAD,
        SHIPPED_ASSASSINATION_INJURY_SPREAD,
    ));
    Some(
        second
            + first
            + param(
                world,
                GNPRTB_ASSASSINATION_INJURY_BASE,
                SHIPPED_ASSASSINATION_INJURY_BASE,
            ),
    )
}

/// Whether an injury kills `character`, slot `+0x2e0` after slot `+0x2f0`
/// (`FUN_004ef6d0`): a minor character (family `0x38`, vtable
/// `0x00665be0`, `FUN_0056f160` -> `FUN_0053eb30`) dies with a chance of
/// GNPRTB 2577; a major one (vtable `0x006657f0`, `FUN_006158b0`) never.
/// Draws nothing for a major character.
pub(crate) fn dies_of_injury(
    world: &GameWorld,
    character: CharacterKey,
    draws: &mut impl Draws,
) -> bool {
    world.characters.get(character).is_some_and(|c| !c.is_major)
        && draws.chance(param(
            world,
            GNPRTB_MINOR_INJURY_DEATH,
            SHIPPED_MINOR_INJURY_DEATH,
        ))
}

fn param(world: &GameWorld, id: u16, shipped: i64) -> i32 {
    i32::try_from(crate::movement::gnprtb_or_shipped(world, id, shipped)).unwrap_or(0)
}

/// A character's record family: `0x30..0x37` for the major characters
/// (MJCHARSD), `0x38..0x3b` for the minor ones (MNCHARSD).
///
/// port: the loader keeps a character's record index without the family
/// byte (`DatId::new(dat.id)`), so a family of 0 reads `is_major`: MJCHARSD
/// ships families `0x30..0x35`, MNCHARSD `0x38`.
fn character_family(c: &crate::world::Character) -> u8 {
    match c.dat_id.family() {
        0 if c.is_major => 0x30,
        0 => 0x38,
        family => family,
    }
}

/// One run's state: the manager and its pool.
struct Run<'a> {
    world: &'a GameWorld,
    location: SystemKey,
    flags: Flags,
    can_resign: bool,
    decoyed: HashSet<DefenderId>,
    /// Members captured or destroyed during the run.
    gone: HashSet<MissionMember>,
    /// Pool counts `+0x34`, `+0x2c`, `+0x30`.
    defenders: i32,
    team: i32,
    decoys: i32,
    detected: bool,
    /// Today, the day an encounter records (`FUN_004fd2f0`).
    day: u64,
    /// port: the run reads the world and applies its effects after, so it
    /// keeps the `Encounter` keys (`+0x9c`) and days it set or cleared, which
    /// the original writes at once.
    encounters: HashMap<CharacterKey, Option<(DatId, u64)>>,
}

/// A confrontation's Force lesson (`FUN_00560200`): the GNPRTB least gain,
/// the GNPRTB percent of the Force gap, and their shipped values.
struct Lesson {
    least: (u16, i64),
    percent: (u16, i64),
}

/// Luke against Vader: GNPRTB 3084 (`DAT_006bb73c`, 1), 3072
/// (`DAT_006bb72c`, 25).
const LUKE_VADER: Lesson = Lesson {
    least: (3084, 1),
    percent: (3072, 25),
};
/// Luke against the Emperor: 3085 (`DAT_006bb71c`, 1), 3073 (`DAT_006bb720`,
/// 20).
const LUKE_EMPEROR: Lesson = Lesson {
    least: (3085, 1),
    percent: (3073, 20),
};
/// Leia against Vader: 3086 (`DAT_006bb724`, 1), 3074 (`DAT_006bb728`, 20).
const LEIA_VADER: Lesson = Lesson {
    least: (3086, 1),
    percent: (3074, 20),
};
/// Leia against the Emperor: 3087 (`DAT_006bb738`, 1), 3075
/// (`DAT_006bb754`, 20).
const LEIA_EMPEROR: Lesson = Lesson {
    least: (3087, 1),
    percent: (3075, 20),
};

/// Which list a walk visits.
#[derive(Debug, Clone, Copy)]
struct Lists {
    chars: bool,
    sforces: bool,
    team: bool,
    decoys: bool,
}

impl Run<'_> {
    /// `FUN_005883b0`: a member counts when it has no request, is not
    /// decoying when asked, and stands where the flags look.
    ///
    /// port: only a decoy decoys, and the lists that skip decoying members
    /// (the team's, and betrayal's before any decoy acts) never meet one, so
    /// the port keeps no decoying mark.
    fn counts(&self, mission: &ActiveMission, member: MissionMember) -> bool {
        if mission.resigning.contains(&member)
            || mission.removing.contains(&member)
            || self.gone.contains(&member)
        {
            return false;
        }
        match member_location_kind(self.world, member) {
            Some(true) => self.flags.in_fleet,
            Some(false) => self.flags.at_system,
            None => false,
        }
    }

    /// `FUN_00587bb0`: team special forces, team characters, decoy special
    /// forces, decoy characters.
    fn members(&self, mission: &ActiveMission, lists: Lists) -> Vec<MissionMember> {
        let mut out = Vec::new();
        let mut visit = |list: &[MissionMember], special: bool| {
            for &member in list {
                let is_special = matches!(member, MissionMember::SpecialForce(_));
                if is_special == special && self.counts(mission, member) {
                    out.push(member);
                }
            }
        };
        if lists.team {
            if lists.sforces {
                visit(&mission.team, true);
            }
            if lists.chars {
                visit(&mission.team, false);
            }
        }
        if lists.decoys {
            if lists.sforces {
                visit(&mission.decoys, true);
            }
            if lists.chars {
                visit(&mission.decoys, false);
            }
        }
        out
    }

    fn skill(&self, member: MissionMember, skill: Skill) -> i32 {
        member_skill(self.world, member, skill)
            .map_or(0, |value| i32::try_from(value).unwrap_or(i32::MAX))
    }

    /// The effective `skill` of `defender`'s officer, 0 without one.
    fn officer_skill(&self, defender: Defender, skill: Skill) -> i32 {
        defender
            .officer(self.world, self.location)
            .map_or(0, |officer| self.skill(MissionMember::Character(officer), skill))
    }

    /// `FUN_005888f0` / `FUN_005349e0`: the member rolls RLEVADTB against the
    /// defender's officer (its effective combat, slot `+0x1f0`), evading or
    /// being captured by it; without an officer, by the defender, at 0.
    fn expose(
        &mut self,
        mission: &mut ActiveMission,
        defender: Defender,
        member: MissionMember,
        rng: &mut MissionRng,
        out: &mut Vec<MissionEffect>,
    ) {
        let (combat, captor) = match defender.officer(self.world, self.location) {
            Some(officer) => (
                self.skill(MissionMember::Character(officer), Skill::Combat),
                faction_of(self.world.characters[officer].is_alliance),
            ),
            None => (0, defender.side),
        };
        self.evade_or_capture(mission, member, combat, captor, rng, out);
        self.request_resign(mission, member);
        // A member with either request leaves the decoy count. port: it also
        // leaves the team count, which nothing reads once the team is
        // exposed; the port drops it.
        let requested =
            mission.resigning.contains(&member) || mission.removing.contains(&member);
        if requested && mission.decoys.contains(&member) {
            self.decoys -= 1;
        }
    }

    /// `FUN_005349e0`: the member rolls RLEVADTB at its effective combat
    /// less the captor's, evading or captured by `captor`'s side. Returns
    /// the injury the roll dealt, or 0.
    fn evade_or_capture(
        &mut self,
        mission: &mut ActiveMission,
        member: MissionMember,
        captor_combat: i32,
        captor: Faction,
        rng: &mut MissionRng,
        out: &mut Vec<MissionEffect>,
    ) -> i32 {
        let combat = self.skill(member, Skill::Combat);
        match (
            table_roll(self.world, RLEVADTB, combat - captor_combat, rng),
            member,
        ) {
            // Slot +0x208: evade. A character also rolls the injury; Luke
            // and Leia then gain Force experience (FUN_0056f8b0,
            // FUN_0056ffa0).
            (Some(true), MissionMember::Character(character)) => {
                self.request_resign(mission, member);
                let injury = self.injure(character, combat, rng, out);
                self.evade_force(character, out);
                injury
            }
            (Some(true), MissionMember::SpecialForce(_)) => {
                self.request_resign(mission, member);
                0
            }
            // Slot +0x20c: captured by the captor's side.
            (Some(false), MissionMember::Character(character)) => {
                let injury = self.injure(character, combat, rng, out);
                out.push(MissionEffect::CharacterCaptured {
                    character,
                    captured_by: match captor {
                        Faction::Alliance => MissionFaction::Alliance,
                        _ => MissionFaction::Empire,
                    },
                    at_system: self.location,
                });
                // FUN_004ef480 sets the status +0x98 to 0, whose handler
                // FUN_004f18e0 sets the member's remove request
                // (FUN_005345d0) while it is on a mission.
                if !mission.removing.contains(&member) {
                    mission.removing.push(member);
                }
                // port: the capture applies after the run, so the run marks
                // the prisoner gone from its lists now.
                self.gone.insert(member);
                injury
            }
            // FUN_00503eb0: a captured special force is destroyed.
            (Some(false), MissionMember::SpecialForce(unit)) => {
                out.push(MissionEffect::SpecialForceDestroyed { unit });
                self.gone.insert(member);
                0
            }
            (None, _) => 0,
        }
    }

    /// `FUN_00534640` when the member may resign (bit 4, record column 9)
    /// and has no remove request (bit 2).
    fn request_resign(&self, mission: &mut ActiveMission, member: MissionMember) {
        if self.can_resign
            && !mission.resigning.contains(&member)
            && !mission.removing.contains(&member)
        {
            mission.resigning.push(member);
        }
    }

    /// Luke's and Leia's slot `+0x208` (`FUN_0056f8b0`, `FUN_0056ffa0`):
    /// after the base evade (`FUN_004ef450`), `ForceExperience` `+0x8e` gains
    /// GNPRTB 3081 (Luke) or 3082 (Leia).
    fn evade_force(&self, character: crate::ids::CharacterKey, out: &mut Vec<MissionEffect>) {
        let Some(c) = self.world.characters.get(character) else {
            return;
        };
        let amount = match c.dat_id.index() {
            LUKE => param(self.world, GNPRTB_LUKE_EVADE_FORCE, SHIPPED_EVADE_FORCE),
            LEIA => param(self.world, GNPRTB_LEIA_EVADE_FORCE, SHIPPED_EVADE_FORCE),
            _ => return,
        };
        out.push(MissionEffect::ForceExperienceGained { character, amount });
    }

    /// Slot `+0x2e4` `FUN_004ef5f0`: the injury roll `FUN_0053e990`, which
    /// returns the injury dealt, or 0.
    fn injure(
        &self,
        character: crate::ids::CharacterKey,
        combat: i32,
        rng: &mut MissionRng,
        out: &mut Vec<MissionEffect>,
    ) -> i32 {
        let Some(injury) = roll_injury(
            combat,
            param(
                self.world,
                GNPRTB_INJURY_MIN_CHANCE,
                SHIPPED_INJURY_MIN_CHANCE,
            ),
            param(self.world, GNPRTB_INJURY_SPREAD, SHIPPED_INJURY_SPREAD),
            param(self.world, GNPRTB_INJURY_BASE, SHIPPED_INJURY_BASE),
            rng,
        ) else {
            return 0;
        };
        out.push(MissionEffect::CharacterInjured { character, injury });
        // Slot +0x2f0 (FUN_004ef6d0, reason 10) then asks slot +0x2e0.
        if dies_of_injury(self.world, character, rng) {
            out.push(MissionEffect::DiedOfInjury { character });
        }
        injury
    }

    /// `FUN_0058a020`, one walk: each defender not decoyed meets a random
    /// decoy (`FUN_00589620`), which draws it off on a TDECOYTB/FDECOYTB roll
    /// or is exposed.
    fn decoy_walk(
        &mut self,
        mission: &mut ActiveMission,
        kinds: Kinds,
        rng: &mut MissionRng,
        out: &mut Vec<MissionEffect>,
    ) {
        let decoy_lists = Lists {
            chars: true,
            sforces: true,
            team: false,
            decoys: true,
        };
        for defender in walk(self.world, self.location, self.flags, kinds) {
            if self.decoyed.contains(&defender.id) {
                continue;
            }
            if self.decoys <= 0 {
                return;
            }
            let pick = rng.draw(self.decoys - 1);
            let Some(decoy) = usize::try_from(pick)
                .ok()
                .and_then(|index| self.members(mission, decoy_lists).get(index).copied())
            else {
                return;
            };
            // FUN_00588b90 -> FUN_0055e410: less the officer's espionage
            // (slot +0x1e0, 0 without one) * G3588 / 100 (DAT_006bb710).
            let officer = self.officer_skill(defender, Skill::Espionage);
            let x = self.skill(decoy, Skill::Espionage)
                - defender.detection
                - officer * param(self.world, GNPRTB_DECOY_OFFICER, SHIPPED_DECOY_OFFICER) / 100;
            let table = if defender.in_fleet {
                FDECOYTB
            } else {
                TDECOYTB
            };
            // port: an absent table leaves the defender in place.
            match table_roll(self.world, table, x, rng) {
                Some(true) => {
                    self.decoyed.insert(defender.id);
                    self.defenders -= 1;
                }
                Some(false) => self.expose(mission, defender, decoy, rng, out),
                None => {}
            }
        }
    }

    /// `FUN_0058a130` with functor `FUN_005896e0`: each defender not decoyed
    /// rolls FOILTB until one detects the team.
    fn detect(&mut self, mission: &ActiveMission, rng: &mut MissionRng) {
        if self.team == 0 {
            return;
        }
        let team_lists = Lists {
            chars: true,
            sforces: true,
            team: true,
            decoys: false,
        };
        let team = self.members(mission, team_lists);
        let espionage: i32 = team
            .iter()
            .map(|&member| self.skill(member, Skill::Espionage))
            .sum::<i32>()
            / self.team;
        let special_forces = i32::try_from(
            team.iter()
                .filter(|member| matches!(member, MissionMember::SpecialForce(_)))
                .count(),
        )
        .unwrap_or(i32::MAX);
        let offset = param(
            self.world,
            GNPRTB_DETECTION_OFFSET,
            SHIPPED_DETECTION_OFFSET,
        );
        for defender in walk(self.world, self.location, self.flags, ALL) {
            if self.decoyed.contains(&defender.id) {
                continue;
            }
            // FUN_00588a90 -> FUN_0055e470: less the officer's espionage
            // (slot +0x1e0, 0 without one) * G3589 / 100 (DAT_006bb70c).
            let officer = self.officer_skill(defender, Skill::Espionage);
            let x = espionage
                - defender.detection
                - special_forces
                - officer * param(self.world, GNPRTB_FOIL_OFFICER, SHIPPED_FOIL_OFFICER) / 100
                - offset;
            // port: an absent FOILTB detects nothing.
            if table_roll(self.world, FOILTB, x, rng) == Some(true) {
                self.detected = true;
                return;
            }
        }
    }

    /// `FUN_00589f10`: the first team or decoy member not decoying that
    /// fails `draw(0..99) < 100 - loyalty` betrays the mission
    /// (`FUN_00588da0`, `FUN_0055e520`). Then each other member
    /// (`FUN_00589490` -> `FUN_00588e80`) learns the traitor with a chance
    /// of its Force `+0x8c` (`FUN_0055e550`): its `TraitorDiscovered` key
    /// `+0xa0` becomes the traitor (`FUN_004ee5c0`, notification `0x361`).
    ///
    /// port: only characters learn (the walk reads `+0x8c`, a character's
    /// Force); the key `+0xa0` is not stored, as no reader of it is traced;
    /// the learning is reported as [`MissionEffect::TraitorIdentified`].
    fn betray(
        &mut self,
        mission: &ActiveMission,
        rng: &mut MissionRng,
        out: &mut Vec<MissionEffect>,
    ) {
        let lists = Lists {
            chars: true,
            sforces: true,
            team: true,
            decoys: true,
        };
        let members = self.members(mission, lists);
        let Some(&traitor) = members
            .iter()
            .find(|&&member| rng.chance(100 - self.skill(member, Skill::Loyalty)))
        else {
            return;
        };
        self.detected = true;
        for &member in &members {
            let MissionMember::Character(learner) = member else {
                continue;
            };
            if member == traitor || !rng.chance(self.force(learner, out)) {
                continue;
            }
            if let MissionMember::Character(traitor) = traitor {
                out.push(MissionEffect::TraitorIdentified {
                    character: learner,
                    traitor,
                });
            }
        }
    }

    /// A character's `Encounter` key and day, as this run left them.
    fn encounter(&self, key: CharacterKey) -> Option<(DatId, u64)> {
        match self.encounters.get(&key) {
            Some(&state) => state,
            None => self
                .world
                .characters
                .get(key)
                .and_then(|c| c.encounter.map(|partner| (partner, c.encounter_day))),
        }
    }

    /// A character's Force `+0x8c` now: the world's, plus the Force
    /// experience this run already gave it, each clamped to `0..=0x7fff` as
    /// `FUN_004ee150` stores it.
    fn force(&self, key: CharacterKey, out: &[MissionEffect]) -> i32 {
        let Some(c) = self.world.characters.get(key) else {
            return 0;
        };
        let gained: i32 = out
            .iter()
            .filter_map(|effect| match effect {
                MissionEffect::ForceExperienceGained { character, amount } if *character == key => {
                    Some(*amount)
                }
                _ => None,
            })
            .sum();
        let base = (i32::from(c.force_base) + gained).clamp(0, 0x7fff);
        (base + i32::from(c.force_training)).min(0x7fff)
    }

    /// `FUN_0058a6c0`: the defenders' Force users, the major characters
    /// (`+0x68`, families `0x30..0x37`) and the minor ones (`+0x60`,
    /// `0x38..0x3b`), at the system with the system flag and aboard its
    /// fleets with the fleet flag (`FUN_0058a7e0`). Each counts
    /// (`FUN_00588840`) with Force at least GNPRTB 3586 (60), free, and of
    /// the other side.
    ///
    /// port: each set is a keyed tree (`FUN_004f44b0` -> `FUN_005f4f10`)
    /// whose in-order list the picks walk; the port reads it in record
    /// order. Without the family byte ([`character_family`]) the index
    /// order is the key order for the shipped records.
    fn force_defenders(&self, side: Faction, out: &[MissionEffect]) -> [Vec<CharacterKey>; 2] {
        let least = param(self.world, GNPRTB_FORCE_DETECTOR, SHIPPED_FORCE_DETECTOR);
        let mut sets: [Vec<(u32, CharacterKey)>; 2] = [Vec::new(), Vec::new()];
        for (key, c) in &self.world.characters {
            let here = match c.current_fleet {
                Some(fleet) => {
                    self.flags.fleets
                        && self
                            .world
                            .fleets
                            .get(fleet)
                            .is_some_and(|f| f.location == self.location)
                }
                None => self.flags.system && c.current_system == Some(self.location),
            };
            let other_side = crate::missions::character_side(c).is_some_and(|s| s != side);
            if !here || c.is_killed || c.is_captive || !other_side {
                continue;
            }
            if self.force(key, out) < least {
                continue;
            }
            match character_family(c) {
                0x30..=0x37 => sets[0].push((c.dat_id.raw(), key)),
                0x38..=0x3b => sets[1].push((c.dat_id.raw(), key)),
                _ => {}
            }
        }
        sets.map(|mut set| {
            set.sort_unstable_by_key(|&(id, _)| id);
            set.into_iter().map(|(_, key)| key).collect()
        })
    }

    /// `FUN_0058a5f0`: the mission's Force users, its major characters
    /// (`+0x50`, `FUN_00587fc0`) and minor ones (`+0x58`, `FUN_00587fe0`),
    /// team and decoys, each counted by `FUN_005883b0`. port: in record
    /// order, as [`Self::force_defenders`].
    fn force_members(&self, mission: &ActiveMission) -> [Vec<CharacterKey>; 2] {
        let mut sets: [Vec<(u32, CharacterKey)>; 2] = [Vec::new(), Vec::new()];
        for &member in mission.team.iter().chain(&mission.decoys) {
            let MissionMember::Character(key) = member else {
                continue;
            };
            let Some(c) = self.world.characters.get(key) else {
                continue;
            };
            if !self.counts(mission, member) {
                continue;
            }
            match character_family(c) {
                0x30..=0x37 => sets[0].push((c.dat_id.raw(), key)),
                0x38..=0x3b => sets[1].push((c.dat_id.raw(), key)),
                _ => {}
            }
        }
        sets.map(|mut set| {
            set.sort_unstable_by_key(|&(id, _)| id);
            set.dedup_by_key(|&mut (id, _)| id);
            set.into_iter().map(|(_, key)| key).collect()
        })
    }

    /// `FUN_0058a9e0` and `FUN_0058ab00`: the next pick from a pair of sets.
    /// Luke (`FUN_0058ac50`), then Vader, when in the first set; else a
    /// random one of the first set (`FUN_0058aca0`, `draw(0..count-1)`),
    /// else of the second. A pick is taken (`+0x1c`) until the sets reset.
    fn pick(
        &self,
        sets: &[Vec<CharacterKey>; 2],
        taken: &HashSet<CharacterKey>,
        rng: &mut MissionRng,
    ) -> Option<CharacterKey> {
        for fixed in [LUKE, VADER] {
            let named = sets[0].iter().copied().find(|&key| {
                !taken.contains(&key)
                    && self
                        .world
                        .characters
                        .get(key)
                        .is_some_and(|c| c.dat_id.index() == fixed)
            });
            if named.is_some() {
                return named;
            }
        }
        for set in sets {
            let open: Vec<CharacterKey> = set
                .iter()
                .copied()
                .filter(|key| !taken.contains(key))
                .collect();
            if open.is_empty() {
                continue;
            }
            let pick = rng.draw(i32::try_from(open.len()).unwrap_or(i32::MAX) - 1);
            return usize::try_from(pick)
                .ok()
                .and_then(|index| open.get(index).copied());
        }
        None
    }

    /// `FUN_00589e40`: Force detection. Each defending Force user in turn
    /// (`FUN_0058a9e0`) meets the mission's Force users (`FUN_0058a2c0`)
    /// until it holds an `Encounter` key: with both Forces above 0, a
    /// chance of their sum plus GNPRTB 3587 (-100) (`FUN_0058a3f0`,
    /// `FUN_0055e4d0`). A hit detects the mission unless the member is a
    /// decoy, starts their encounter (`FUN_0055ff60`), and takes both out of
    /// the sets (`FUN_0058a530`).
    fn force_detect(
        &mut self,
        mission: &ActiveMission,
        rng: &mut MissionRng,
        out: &mut Vec<MissionEffect>,
    ) {
        let side = Faction::from(mission.faction);
        let defenders = self.force_defenders(side, out);
        let members = self.force_members(mission);
        let offset = param(
            self.world,
            GNPRTB_FORCE_DETECTION_OFFSET,
            SHIPPED_FORCE_DETECTION_OFFSET,
        );
        let mut defenders_taken = HashSet::new();
        let mut removed = HashSet::new();
        while let Some(defender) = self.pick(&defenders, &defenders_taken, rng) {
            defenders_taken.insert(defender);
            // FUN_0058a3b0 resets the members' picks, not their removal.
            let mut members_taken = removed.clone();
            while let Some(member) = self.pick(&members, &members_taken, rng) {
                members_taken.insert(member);
                let (f_member, f_defender) = (self.force(member, out), self.force(defender, out));
                let chance = if f_member != 0 && f_defender != 0 {
                    f_member + f_defender + offset
                } else {
                    0
                };
                // FUN_0055e4d0 draws only for a positive chance.
                if chance > 0 && rng.chance(chance) {
                    if !mission.decoys.contains(&MissionMember::Character(member)) {
                        self.detected = true;
                    }
                    self.begin_encounter(member, defender, out);
                    removed.insert(member);
                    removed.insert(defender);
                }
                // The do-while in FUN_0058a2c0 stops once the defender holds
                // an Encounter key.
                if self.encounter(defender).is_some() {
                    break;
                }
            }
        }
    }

    /// `FUN_0055ff60(member, defender)`: when neither holds an `Encounter`
    /// key, both sides observe the system at level 6 (`FUN_0050d5a0(6, 1)`,
    /// `(6, 2)`), each names the other (`FUN_004ee500`) and both record today
    /// (`FUN_004ef1c0`).
    ///
    /// port: the observation reveals the system to both sides, as
    /// Espionage's level 10 does. hyp: `FUN_004f9510` hands the event to
    /// each side's view (`decoy-roll.md`) and sets no rule.
    fn begin_encounter(
        &mut self,
        member: CharacterKey,
        defender: CharacterKey,
        out: &mut Vec<MissionEffect>,
    ) {
        if self.encounter(member).is_some() || self.encounter(defender).is_some() {
            return;
        }
        let (Some(m), Some(d)) = (
            self.world.characters.get(member),
            self.world.characters.get(defender),
        ) else {
            return;
        };
        if member_location_kind(self.world, MissionMember::Character(member)) == Some(false) {
            for faction in [MissionFaction::Alliance, MissionFaction::Empire] {
                out.push(MissionEffect::SystemIntelligenceGathered {
                    system: self.location,
                    faction,
                });
            }
        }
        self.encounters.insert(member, Some((d.dat_id, self.day)));
        self.encounters.insert(defender, Some((m.dat_id, self.day)));
        out.push(MissionEffect::EncounterBegan {
            member,
            defender,
            day: self.day,
        });
    }

    /// `FUN_00588d30` -> `FUN_00560200(member, partner)`: the confrontation
    /// of a detected member holding an `Encounter` key, when each names the
    /// other and both share the encounter day.
    ///
    /// 1. The member rolls RLEVADTB against the partner's effective combat
    ///    (`FUN_005349e0`). A member it did not injure takes the duel roll
    ///    `FUN_0053ea70`; so does the partner, the other way round. The side
    ///    that injured the other raises its base combat `+0x62` by GNPRTB
    ///    2578 (`FUN_005340a0`).
    /// 2. Both keys and days clear (`FUN_004ee500`, `FUN_004ef1c0(0)`).
    /// 3. Luke against Vader learns his heritage the first time
    ///    (`FUN_0056f650`) and, free with Force below GNPRTB 3077, is wounded
    ///    (`FUN_00561b60`); free, he learns Force from the gap. Luke against
    ///    the Emperor, and Leia free and Force aware against either, learn
    ///    the same way ([`Lesson`]).
    fn confront(
        &mut self,
        mission: &mut ActiveMission,
        member: CharacterKey,
        rng: &mut MissionRng,
        out: &mut Vec<MissionEffect>,
    ) {
        let Some((partner_id, day)) = self.encounter(member) else {
            return;
        };
        let Some((partner, p)) = self
            .world
            .characters
            .iter()
            .find(|(_, c)| c.dat_id == partner_id && !c.is_killed)
        else {
            return;
        };
        let Some(m) = self.world.characters.get(member) else {
            return;
        };
        let mutual = self
            .encounter(partner)
            .is_some_and(|(id, partner_day)| id == m.dat_id && partner_day == day);
        if !mutual || day == 0 {
            return;
        }
        let member_combat = self.skill(MissionMember::Character(member), Skill::Combat);
        let partner_combat = self.skill(MissionMember::Character(partner), Skill::Combat);
        let captor = crate::missions::character_side(p).unwrap_or(Faction::Neutral);
        let mut member_injury = self.evade_or_capture(
            mission,
            MissionMember::Character(member),
            partner_combat,
            captor,
            rng,
            out,
        );
        let duel = |rng: &mut MissionRng, own: i32, other: i32| {
            // FUN_0053ea70(own, other): a chance of max(2571, other - own).
            roll_injury(
                100 - (other - own),
                param(self.world, GNPRTB_DUEL_MIN_CHANCE, SHIPPED_DUEL_MIN_CHANCE),
                param(self.world, GNPRTB_DUEL_SPREAD, SHIPPED_DUEL_SPREAD),
                param(self.world, GNPRTB_DUEL_BASE, SHIPPED_DUEL_BASE),
                rng,
            )
            .unwrap_or(0)
        };
        if member_injury == 0 {
            member_injury = duel(rng, member_combat, partner_combat);
            if member_injury != 0 {
                out.push(MissionEffect::CharacterInjured {
                    character: member,
                    injury: member_injury,
                });
                // Slot +0x2f0 asks slot +0x2e0 whether the injury kills.
                if dies_of_injury(self.world, member, rng) {
                    out.push(MissionEffect::DiedOfInjury { character: member });
                }
            }
        }
        let partner_injury = duel(rng, partner_combat, member_combat);
        if partner_injury != 0 {
            out.push(MissionEffect::CharacterInjured {
                character: partner,
                injury: partner_injury,
            });
            if dies_of_injury(self.world, partner, rng) {
                out.push(MissionEffect::DiedOfInjury { character: partner });
            }
        }
        let gain = param(
            self.world,
            GNPRTB_DUEL_COMBAT_GAIN,
            SHIPPED_DUEL_COMBAT_GAIN,
        );
        for (victor, won) in [(member, partner_injury != 0), (partner, member_injury != 0)] {
            if won {
                out.push(MissionEffect::SkillRaised {
                    character: victor,
                    skill: Skill::Combat,
                    amount: gain,
                });
            }
        }
        self.encounters.insert(member, None);
        self.encounters.insert(partner, None);
        out.push(MissionEffect::EncounterEnded { member, partner });
        self.lessons(member, partner, rng, out);
    }

    /// `FUN_00560200`'s Jedi steps, by the pair's records.
    fn lessons(
        &self,
        a: CharacterKey,
        b: CharacterKey,
        rng: &mut MissionRng,
        out: &mut Vec<MissionEffect>,
    ) {
        let role = |index: u32| {
            [a, b].into_iter().find(|&key| {
                self.world
                    .characters
                    .get(key)
                    .is_some_and(|c| c.dat_id.index() == index)
            })
        };
        let (luke, leia, vader, emperor) = (role(LUKE), role(LEIA), role(VADER), role(EMPEROR));
        // +0xac bit 0: held captive, here or by this run's capture.
        let free = |key: CharacterKey| {
            !self.gone.contains(&MissionMember::Character(key))
                && self
                    .world
                    .characters
                    .get(key)
                    .is_some_and(|c| !c.is_captive)
        };
        if let Some(luke) = luke {
            if let Some(vader) = vader {
                let knows = self
                    .world
                    .characters
                    .get(luke)
                    .is_some_and(|c| c.heritage_known);
                if !knows {
                    out.push(MissionEffect::HeritageLearned { character: luke });
                    let least = param(
                        self.world,
                        GNPRTB_LUKE_WOUND_FORCE,
                        SHIPPED_LUKE_WOUND_FORCE,
                    );
                    if free(luke) && self.force(luke, out) < least {
                        // FUN_0055e810: rand(0..=3080) + 3079.
                        let injury =
                            rng.draw(param(
                                self.world,
                                GNPRTB_LUKE_WOUND_SPREAD,
                                SHIPPED_LUKE_WOUND_SPREAD,
                            )) + param(self.world, GNPRTB_LUKE_WOUND_BASE, SHIPPED_LUKE_WOUND_BASE);
                        out.push(MissionEffect::CharacterInjured {
                            character: luke,
                            injury,
                        });
                    }
                }
                if free(luke) {
                    self.learn(luke, vader, &LUKE_VADER, out);
                }
            }
            if let Some(emperor) = emperor {
                if free(luke) {
                    self.learn(luke, emperor, &LUKE_EMPEROR, out);
                }
            }
        }
        if let Some(leia) = leia {
            let aware = self
                .world
                .characters
                .get(leia)
                .is_some_and(|c| c.force_aware);
            if free(leia) && aware {
                if let Some(vader) = vader {
                    self.learn(leia, vader, &LEIA_VADER, out);
                }
                if let Some(emperor) = emperor {
                    self.learn(leia, emperor, &LEIA_EMPEROR, out);
                }
            }
        }
    }

    /// The pupil's `ForceExperience` `+0x8e` gains max(least, (master's
    /// Force - pupil's) x percent / 100) (`FUN_0053e130`, `FUN_0053e190`,
    /// `FUN_004ee150`).
    fn learn(
        &self,
        pupil: CharacterKey,
        master: CharacterKey,
        lesson: &Lesson,
        out: &mut Vec<MissionEffect>,
    ) {
        let mut amount = param(self.world, lesson.least.0, lesson.least.1);
        let gap = self.force(master, out) - self.force(pupil, out);
        if gap > 0 {
            amount = amount.max(gap * param(self.world, lesson.percent.0, lesson.percent.1) / 100);
        }
        out.push(MissionEffect::ForceExperienceGained {
            character: pupil,
            amount,
        });
    }

    /// `FUN_0058a1c0`'s walks: every team character (families `0x30..0x37`,
    /// then `0x38..0x3b`), then every team special force, not decoying, faces
    /// a random defender (`FUN_00588650`) until none is left. A member
    /// holding an `Encounter` key confronts its partner instead
    /// (`FUN_00589360` -> `FUN_005892a0`, [`Self::confront`]).
    fn expose_team(
        &mut self,
        mission: &mut ActiveMission,
        rng: &mut MissionRng,
        out: &mut Vec<MissionEffect>,
    ) {
        let lists = Lists {
            chars: true,
            sforces: false,
            team: true,
            decoys: false,
        };
        let family = |member: &MissionMember| {
            member
                .character()
                .and_then(|key| self.world.characters.get(key))
                .map_or(0, character_family)
        };
        let characters = self.members(mission, lists);
        let (early, late): (Vec<_>, Vec<_>) = characters
            .into_iter()
            .partition(|member| family(member) < 0x38);
        let special_forces = self.members(
            mission,
            Lists {
                chars: false,
                sforces: true,
                ..lists
            },
        );
        for member in early.into_iter().chain(late).chain(special_forces) {
            if let MissionMember::Character(key) = member {
                if self.encounter(key).is_some() {
                    self.confront(mission, key, rng, out);
                    continue;
                }
            }
            if self.defenders <= 0 {
                return;
            }
            let pick = rng.draw(self.defenders - 1);
            let Some(defender) = usize::try_from(pick).ok().and_then(|index| {
                walk(self.world, self.location, self.flags, ALL)
                    .into_iter()
                    .filter(|defender| !self.decoyed.contains(&defender.id))
                    .nth(index)
            }) else {
                return;
            };
            self.expose(mission, defender, member, rng, out);
        }
    }
}

/// The run `FUN_00547f60` after a phase change: one pass of
/// `FUN_00589970`, then the second pass's validator.
///
/// Modes come from the constant `0x4112` (`FUN_005236e0`): phase 2 mode 2,
/// phases 3 and 7 mode 1, phase 9 mode 4, else 0. The second pass
/// (`+0x44` clear) sets skip for modes 1, 2, and 4, so only its validator
/// acts.
#[expect(
    clippy::too_many_arguments,
    reason = "The run reads the mission's world, rules, transits and day."
)]
pub(crate) fn run(
    mission: &mut ActiveMission,
    world: &GameWorld,
    uprisings: &UprisingState,
    research: &crate::research::ResearchState,
    en_route: &[MemberTransit],
    day: u64,
    rng: &mut MissionRng,
    out: &mut Vec<MissionEffect>,
) {
    if mission.end_code != 0 {
        return;
    }
    // FUN_00520ad0 picks the target location +0x78 past phase 4, else the
    // members' location (FUN_00520f40).
    // port: past phase 4 an abode target (the Palace mission's) is the
    // location; the port models no defenders in an abode, so no one
    // detects there.
    let location = if mission.phase > 4 {
        mission
            .target_abode
            .is_none()
            .then_some(mission.target_system)
    } else {
        mission.origin
    };
    let Some(location) = location.filter(|&system| world.systems.contains_key(system)) else {
        return;
    };
    let validate = |mission: &mut ActiveMission| {
        if mission.end_code == 0 {
            mission.end_code = running_end_code(mission, world, uprisings, research, en_route);
        }
    };
    first_pass(mission, world, location, day, rng, out, &validate);
    validate(mission);
    // FUN_004f9510 then hands the ended mission to the opponent's view. hyp:
    // no rule reads it (decoy-roll.md).
}

fn first_pass(
    mission: &mut ActiveMission,
    world: &GameWorld,
    location: SystemKey,
    day: u64,
    rng: &mut MissionRng,
    out: &mut Vec<MissionEffect>,
    validate: &dyn Fn(&mut ActiveMission),
) {
    // FUN_00589a40: the validator, then the skips.
    validate(mission);
    if mission.end_code != 0 {
        return;
    }
    let Some(record) = mission
        .kind
        .record_id()
        .and_then(|id| world.mission_record(id))
    else {
        return;
    };
    if !record.detection_phases {
        return;
    }
    // FUN_005898f0 runs FUN_00589970 twice; for every kind but Adrift the
    // second pass skips (modes 1, 2 and 4), so one pass acts.
    // FUN_00589a40 also runs the validator on the second pass. port: the
    // run's closing validator stands in for it: a member the first pass
    // captured already has its remove request, so the second pass neither
    // counts nor exposes it.
    let mut inherited = Flags::default();
    let mut detected = false;
    for &mode in detection_passes(mission.kind, mission.phase) {
        detected = one_pass(
            mission,
            world,
            location,
            day,
            rng,
            out,
            record.can_resign,
            mode,
            &mut inherited,
            detected,
        );
    }
}

/// The modes `FUN_00589a40` reads from the mission's mode word for `phase`,
/// one per pass that acts: `0x4112` gives phase 2 mode 2, phases 3 and 7
/// mode 1 and phase 9 mode 4; Adrift's `0x4103` gives phase 2 mode 3,
/// which runs mode 1's body on the first pass and mode 2's on the second,
/// phase 7 mode 1 and phase 9 mode 4 (`ghidra/notes/decoy-roll.md`).
fn detection_passes(kind: crate::missions::MissionKind, phase: u8) -> &'static [u8] {
    if kind == crate::missions::MissionKind::Adrift {
        match phase {
            2 => &[1, 2],
            7 => &[1],
            PHASE_BETRAYAL => &[4],
            _ => &[],
        }
    } else {
        match phase {
            2 => &[2],
            3 | 7 => &[1],
            PHASE_BETRAYAL => &[4],
            _ => &[],
        }
    }
}

/// One pass of `FUN_00589970` in `mode`: setup's flags, added to the earlier
/// pass's (`inherited`), then Force detection, betrayal, the decoys and the
/// detection while the team is not yet `detected`, and the detected outcome
/// (`FUN_0058a1c0`) whenever it is. Returns whether the team is detected.
#[expect(
    clippy::too_many_arguments,
    reason = "One pass reads the mission's world, place, day and stream."
)]
fn one_pass(
    mission: &mut ActiveMission,
    world: &GameWorld,
    location: SystemKey,
    day: u64,
    rng: &mut MissionRng,
    out: &mut Vec<MissionEffect>,
    can_resign: bool,
    mode: u8,
    inherited: &mut Flags,
    detected: bool,
) -> bool {
    let side = Faction::from(mission.faction);
    let opponent = if side == Faction::Alliance {
        Faction::Empire
    } else {
        Faction::Alliance
    };
    let fleet_here = |faction: Faction| {
        world
            .fleets
            .values()
            .any(|fleet| fleet.location == location && faction_of(fleet.is_alliance) == faction)
    };
    let opponent_fleet = fleet_here(opponent);
    let own_fleet = fleet_here(side);
    let holder_is_opponent = world
        .systems
        .get(location)
        .and_then(crate::uprising::holder)
        == Some(opponent);
    // Setup only sets flags (FUN_00589a40), so a second pass keeps the
    // first's.
    let mut flags = *inherited;
    match mode {
        1 => {
            flags.in_fleet = true;
            if mission.origin != Some(mission.target_system) {
                flags.at_system = true;
            }
            if opponent_fleet && !own_fleet {
                flags.fleets = true;
            }
        }
        2 => {
            flags.at_system = true;
            if holder_is_opponent {
                flags.system = true;
            }
        }
        _ => {
            (flags.at_system, flags.in_fleet) = (true, true);
            // port: the target of a system or object kind is its system
            // (target_view), read as a system; a character target is not.
            let target_is_system =
                mission.kind.target_rules() != crate::missions::TargetRules::Character;
            if target_is_system {
                if holder_is_opponent {
                    flags.system = true;
                }
            } else if opponent_fleet {
                flags.fleets = true;
            }
        }
    }
    *inherited = flags;
    let mut run = Run {
        world,
        location,
        flags,
        can_resign,
        decoyed: HashSet::new(),
        gone: HashSet::new(),
        defenders: 0,
        team: 0,
        decoys: 0,
        detected,
        day,
        encounters: HashMap::new(),
    };
    // FUN_005885f0.
    run.defenders = i32::try_from(walk(world, location, flags, ALL).len()).unwrap_or(i32::MAX);
    let count =
        |run: &Run<'_>, lists| i32::try_from(run.members(mission, lists).len()).unwrap_or(0);
    run.team = count(
        &run,
        Lists {
            chars: true,
            sforces: true,
            team: true,
            decoys: false,
        },
    );
    run.decoys = count(
        &run,
        Lists {
            chars: true,
            sforces: true,
            team: false,
            decoys: true,
        },
    );
    // FUN_00589e40: Force detection.
    run.force_detect(mission, rng, out);
    if mission.phase == PHASE_BETRAYAL {
        run.betray(mission, rng, out);
    }
    if !run.detected {
        if flags.system {
            run.decoy_walk(mission, ALL, rng, out);
        }
        if flags.fleets {
            run.decoy_walk(
                mission,
                Kinds {
                    ships: true,
                    fighters: false,
                    regiments: false,
                },
                rng,
                out,
            );
            run.decoy_walk(
                mission,
                Kinds {
                    ships: false,
                    fighters: true,
                    regiments: false,
                },
                rng,
                out,
            );
        }
    }
    if !run.detected {
        run.detect(mission, rng);
    }
    if run.detected {
        // Slot +0x1dc: 3, or 4 past phase 4, when members may resign.
        if can_resign {
            mission.end_code = if mission.phase > 4 { 4 } else { 3 };
        }
        run.expose_team(mission, rng, out);
    }
    run.detected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_assassination_injury_and_a_minor_character_s_death_follow_their_rolls() {
        struct Counted(f64, usize);
        impl Draws for Counted {
            fn draw(&mut self, n: i32) -> i32 {
                self.1 += 1;
                let mut rolls = crate::uprising::Rolls::new(std::slice::from_ref(&self.0));
                Draws::draw(&mut rolls, n)
            }
            fn chance(&mut self, percent: i32) -> bool {
                self.draw(99) < percent
            }
        }
        // FUN_0053eae0: chance GNPRTB 2574 (100), then rand(0..=2574) +
        // rand(0..=2576 (29)) + 2575 (1). FUN_0053eb30: a minor character
        // dies with GNPRTB 2577 (25); a major one draws nothing.
        let world = GameWorld::default();
        let roll = |rolls: &[f64]| {
            let mut draws = crate::uprising::Rolls::new(rolls);
            assassination_injury(&world, &mut draws)
        };
        assert_eq!(roll(&[0.0, 0.0, 0.0]), Some(1));
        assert_eq!(roll(&[0.0, 0.999, 0.999]), Some(130));
        let mut world = GameWorld::default();
        let minor = world.characters.insert(crate::world::Character::default());
        let major = world.characters.insert(crate::world::Character {
            is_major: true,
            ..Default::default()
        });
        let dies = |key, roll: f64| {
            let mut draws = Counted(roll, 0);
            let died = dies_of_injury(&world, key, &mut draws);
            (died, draws.1)
        };
        assert_eq!(dies(minor, 0.24), (true, 1));
        assert_eq!(dies(minor, 0.26), (false, 1));
        assert_eq!(dies(major, 0.0), (false, 0));
    }
    use crate::ids::{CharacterKey, DatId, SpecialForceKey};
    use crate::missions::MissionKind;
    use crate::world::{
        Character, ControlKind, Fleet, MissionRecord, MissionTargetRules, MstbEntry, MstbTable,
        ShipInstance, SkillPair, SpecialForceUnit, System, TroopClassDef, TroopUnit,
    };

    /// A populated system held by `control`.
    fn system(world: &mut GameWorld, control: ControlKind) -> SystemKey {
        world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
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
            control,
        })
    }

    fn imperial() -> ControlKind {
        ControlKind::Controlled(Faction::Empire)
    }

    /// The shipped MISSNSD Sabotage record `0x69000012` with detection
    /// phases and resign on; the target rules admit any calm system, so the
    /// validator leaves the run alone.
    fn sabotage_record(can_resign: bool) -> MissionRecord {
        MissionRecord {
            dat_id: DatId::new(0x6900_0012),
            timer_min_days: 1,
            timer_spread_days: 0,
            repeats: false,
            hidden: false,
            detection_phases: true,
            can_resign,
            leave_rule: 0,
            rules: MissionTargetRules {
                own_side_target: true,
                other_side_target: true,
                opponent_target: true,
                calm_target: true,
                ..MissionTargetRules::default()
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0x402,
                character_mask: 0x1_0000,
            },
        }
    }

    /// A table whose every row is `percent`.
    fn table(world: &mut GameWorld, name: &str, percent: u32) {
        world.mission_tables.insert(
            name.to_string(),
            MstbTable::new(vec![MstbEntry {
                threshold: 0,
                value: percent,
            }]),
        );
    }

    fn world_with(control: ControlKind) -> (GameWorld, SystemKey) {
        let mut world = GameWorld::default();
        world.mission_records.push(sabotage_record(true));
        let here = system(&mut world, control);
        (world, here)
    }

    fn agent(world: &mut GameWorld, at: SystemKey, loyalty: u32) -> MissionMember {
        MissionMember::Character(world.characters.insert(Character {
            name: "Agent".into(),
            is_alliance: true,
            loyalty: SkillPair {
                base: loyalty,
                variance: 0,
            },
            current_system: Some(at),
            ..Default::default()
        }))
    }

    fn imperial_regiment(world: &mut GameWorld, at: SystemKey) -> TroopKey {
        regiment(world, at, false, 0)
    }

    /// A regiment whose class detection (slot `+0x1c4`) is `detection`.
    fn regiment(
        world: &mut GameWorld,
        at: SystemKey,
        is_alliance: bool,
        detection: u32,
    ) -> TroopKey {
        let class = DatId::new(0x1000_0000 | detection);
        world.troop_classes.insert(
            class,
            TroopClassDef {
                attack_strength: 0,
                defense_strength: 0,
                detection,
                uprising_defense: 0,
            },
        );
        let key = world.troops.insert(TroopUnit {
            class_dat_id: class,
            is_alliance,
            regiment_strength: 1,
        });
        world.systems[at].ground_units.push(key);
        key
    }

    /// A loyal Alliance agent of family `family` with the given espionage
    /// and combat.
    fn skilled(
        world: &mut GameWorld,
        at: SystemKey,
        family: u32,
        espionage: u32,
        combat: u32,
    ) -> MissionMember {
        let pair = |base| SkillPair { base, variance: 0 };
        MissionMember::Character(world.characters.insert(Character {
            dat_id: DatId::new(family << 24 | 1),
            name: "Agent".into(),
            is_alliance: true,
            loyalty: pair(100),
            espionage: pair(espionage),
            combat: pair(combat),
            current_system: Some(at),
            ..Default::default()
        }))
    }

    /// A table that yields 100 only for `x` in `low..=high`, else 0.
    fn band(world: &mut GameWorld, name: &str, low: i32, high: i32) {
        world.mission_tables.insert(
            name.to_string(),
            MstbTable::new(vec![
                MstbEntry {
                    threshold: i32::MIN,
                    value: 0,
                },
                MstbEntry {
                    threshold: low,
                    value: 100,
                },
                MstbEntry {
                    threshold: high + 1,
                    value: 0,
                },
            ]),
        );
    }

    fn special_force(world: &mut GameWorld, at: SystemKey, espionage: u32) -> MissionMember {
        let mut skills = [0; 8];
        skills[Skill::Espionage as usize] = espionage;
        let unit = world.special_forces.insert(SpecialForceUnit {
            class_dat_id: DatId::new(0x1400_0001),
            is_alliance: true,
            skills,
            on_mission: true,
        });
        world.systems[at].special_forces.push(unit);
        MissionMember::SpecialForce(unit)
    }

    fn imperial_fleet(world: &mut GameWorld, at: SystemKey, is_alliance: bool) -> FleetKey {
        world.fleets.insert(Fleet {
            location: at,
            capital_ships: vec![ShipInstance {
                class: crate::ids::CapitalShipKey::default(),
                hull_current: 1,
                subsystem_hits: 0,
                alive: true,
                name: None,
                id: 0,
            }],
            fighters: vec![],
            characters: vec![],
            is_alliance,
            has_death_star: false,
            special_forces: Vec::new(),
        })
    }

    fn run_seeded(mission: &mut ActiveMission, world: &GameWorld, roll: f64) -> Vec<MissionEffect> {
        let mut out = Vec::new();
        let mut rng = MissionRng::seeded(roll, mission.id);
        run(
            mission,
            world,
            &UprisingState::default(),
            &crate::research::ResearchState::default(),
            &[],
            1,
            &mut rng,
            &mut out,
        );
        out
    }

    fn sabotage(team: Vec<MissionMember>, at: SystemKey, phase: u8) -> ActiveMission {
        let mut mission = ActiveMission::new(
            0,
            MissionKind::Sabotage,
            MissionFaction::Alliance,
            team,
            at,
            0,
        );
        mission.phase = phase;
        mission.origin = Some(at);
        mission
    }

    fn run_once(mission: &mut ActiveMission, world: &GameWorld) -> Vec<MissionEffect> {
        run_seeded(mission, world, 0.25)
    }

    fn character(member: MissionMember) -> CharacterKey {
        member.character().expect("a character member")
    }

    #[test]
    fn an_opposing_regiment_detects_a_team_in_phase_two_and_the_evading_team_resigns() {
        // FUN_005896e0 detects on a FOILTB success; slot +0x1dc ends 3 at or
        // before phase 4 when members may resign; an evading member
        // (RLEVADTB success, slot +0x208) takes a resign request and the
        // injury roll, whose chance max(G2565, 100 - combat) is 100 here.
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 100);
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 3);
        assert_eq!(mission.resigning, vec![lead]);
        assert!(matches!(
            out.as_slice(),
            [MissionEffect::CharacterInjured { character: c, .. }] if *c == character(lead)
        ));
    }

    #[test]
    fn luke_and_leia_gain_force_experience_when_they_evade() {
        // Slot +0x208 of Luke's class (FUN_0056f8b0) and Leia's
        // (FUN_0056ffa0): after the base evade and its injury roll,
        // ForceExperience +0x8e gains GNPRTB 3081 / 3082 (shipped 1).
        for (record, gains) in [(LUKE, true), (LEIA, true), (0x240, false)] {
            let (mut world, here) = world_with(imperial());
            imperial_regiment(&mut world, here);
            table(&mut world, FOILTB, 100);
            table(&mut world, RLEVADTB, 100);
            let lead = agent(&mut world, here, 100);
            world.characters[character(lead)].dat_id = crate::ids::DatId::new(record);
            let mut mission = sabotage(vec![lead], here, 2);

            let out = run_once(&mut mission, &world);

            let gained: Vec<_> = out
                .iter()
                .filter_map(|effect| match effect {
                    MissionEffect::ForceExperienceGained { character, amount } => {
                        Some((*character, *amount))
                    }
                    _ => None,
                })
                .collect();
            let expected = if gains {
                vec![(character(lead), 1)]
            } else {
                vec![]
            };
            assert_eq!(gained, expected, "{record:#x}");
            // The gain follows the injury roll.
            if gains {
                assert!(matches!(
                    out.as_slice(),
                    [
                        MissionEffect::CharacterInjured { .. },
                        MissionEffect::ForceExperienceGained { .. }
                    ]
                ));
            }
        }
    }

    #[test]
    fn a_luke_captured_gains_no_force_experience() {
        // Slot +0x20c (captured) has no override in Luke's class.
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 0);
        let lead = agent(&mut world, here, 100);
        world.characters[character(lead)].dat_id = crate::ids::DatId::new(LUKE);
        let mut mission = sabotage(vec![lead], here, 2);
        let out = run_once(&mut mission, &world);
        assert!(!out
            .iter()
            .any(|effect| matches!(effect, MissionEffect::ForceExperienceGained { .. })));
    }

    #[test]
    fn a_member_who_fails_to_evade_is_injured_and_captured_by_the_defender_s_side() {
        // Slot +0x20c: the captured character takes the injury roll, then
        // becomes the defender side's prisoner at the location.
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 0);
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 3);
        assert!(matches!(
            out.as_slice(),
            [
                MissionEffect::CharacterInjured { .. },
                MissionEffect::CharacterCaptured {
                    captured_by: MissionFaction::Empire,
                    at_system,
                    ..
                },
            ] if *at_system == here
        ));
    }

    #[test]
    fn a_captured_special_force_is_destroyed() {
        // FUN_00503eb0: the special-force capture slot destroys the unit.
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 0);
        let unit: SpecialForceKey = world.special_forces.insert(SpecialForceUnit {
            class_dat_id: DatId::new(0x1400_0001),
            is_alliance: true,
            skills: [0; 8],
            on_mission: true,
        });
        world.systems[here].special_forces.push(unit);
        let mut mission = sabotage(vec![MissionMember::SpecialForce(unit)], here, 2);

        let out = run_once(&mut mission, &world);

        assert!(matches!(
            out.as_slice(),
            [MissionEffect::SpecialForceDestroyed { unit: u }] if *u == unit
        ));
    }

    #[test]
    fn a_system_the_mission_s_own_side_holds_has_no_defenders_in_phase_two() {
        // FUN_00589a40 mode 2 walks the system only when the opponent holds it.
        let (mut world, here) = world_with(ControlKind::Controlled(Faction::Alliance));
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 0);
        assert!(out.is_empty());
    }

    #[test]
    fn a_zero_or_absent_foiltb_detects_nothing() {
        // FUN_005896e0 detects only on a FOILTB success. port: an absent
        // table detects nothing.
        for foil in [Some(0), None] {
            let (mut world, here) = world_with(imperial());
            imperial_regiment(&mut world, here);
            if let Some(percent) = foil {
                table(&mut world, FOILTB, percent);
            }
            let lead = agent(&mut world, here, 100);
            let mut mission = sabotage(vec![lead], here, 2);

            assert!(run_once(&mut mission, &world).is_empty());
            assert_eq!(mission.end_code, 0, "FOILTB {foil:?}");
        }
    }

    #[test]
    fn a_decoy_that_draws_off_the_only_regiment_leaves_the_team_undetected() {
        // FUN_0058a020: a TDECOYTB success decoys the defender, which the
        // detection walk then skips.
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, TDECOYTB, 100);
        let lead = agent(&mut world, here, 100);
        let decoy = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 2);
        mission.decoys = vec![decoy];

        let out = run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 0);
        assert!(out.is_empty());
        assert!(mission.resigning.is_empty());
    }

    #[test]
    fn a_decoy_that_fails_is_exposed_and_resigns_while_the_team_runs_on() {
        // FUN_00589620: a failed decoy roll exposes the decoy (FUN_005888f0).
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 0);
        table(&mut world, TDECOYTB, 0);
        table(&mut world, RLEVADTB, 100);
        let lead = agent(&mut world, here, 100);
        let decoy = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 2);
        mission.decoys = vec![decoy];

        run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 0);
        assert_eq!(mission.resigning, vec![decoy]);
    }

    #[test]
    fn a_disloyal_member_betrays_the_mission_in_phase_nine() {
        // FUN_00589f10 / FUN_00588da0: draw(0..99) < 100 - loyalty betrays;
        // past phase 4 a detected mission ends 4. No table is present, so
        // only the betrayal can detect.
        for (loyalty, end_code) in [(0, 4), (100, 0)] {
            let (mut world, here) = world_with(imperial());
            let lead = agent(&mut world, here, loyalty);
            let mut mission = sabotage(vec![lead], here, PHASE_BETRAYAL);

            run_once(&mut mission, &world);

            assert_eq!(mission.end_code, end_code, "loyalty {loyalty}");
        }
    }

    #[test]
    fn the_other_members_learn_the_traitor_on_their_force() {
        // FUN_00589490 -> FUN_00588e80: after a betrayal, each other member
        // learns the traitor with a chance of its Force +0x8c
        // (FUN_0055e550): Force 100 always, Force 0 never; the traitor
        // itself draws nothing.
        let (mut world, here) = world_with(imperial());
        let traitor = agent(&mut world, here, 0);
        let seer = agent(&mut world, here, 100);
        let blind = agent(&mut world, here, 100);
        world.characters[character(seer)].force_base = 100;
        let mut mission = sabotage(vec![traitor, seer, blind], here, PHASE_BETRAYAL);

        let out = run_once(&mut mission, &world);

        let learned: Vec<_> = out
            .iter()
            .filter_map(|effect| match effect {
                MissionEffect::TraitorIdentified { character, traitor } => {
                    Some((*character, *traitor))
                }
                _ => None,
            })
            .collect();
        assert_eq!(learned, [(character(seer), character(traitor))]);

        // No betrayal, no learning.
        world.characters[character(traitor)].loyalty.base = 100;
        let mut mission = sabotage(vec![traitor, seer, blind], here, PHASE_BETRAYAL);
        let out = run_once(&mut mission, &world);
        assert!(!out
            .iter()
            .any(|effect| matches!(effect, MissionEffect::TraitorIdentified { .. })));
    }

    #[test]
    fn an_opposing_fleet_at_the_origin_detects_a_team_bound_elsewhere_in_phase_three() {
        // Mode 1 counts members at a system only when the origin is not the
        // target, and walks the fleets when only the opponent has one there.
        let (mut world, here) = world_with(imperial());
        let there = system(&mut world, ControlKind::Uncontrolled);
        table(&mut world, FOILTB, 100);
        imperial_fleet(&mut world, here, false);
        for (target, end_code) in [(there, 3), (here, 0)] {
            let lead = agent(&mut world, here, 100);
            let mut mission = sabotage(vec![lead], target, 3);
            mission.origin = Some(here);

            run_once(&mut mission, &world);

            assert_eq!(mission.end_code, end_code, "target {target:?}");
        }
        // An own fleet there as well keeps mode 1 off the fleets.
        imperial_fleet(&mut world, here, true);
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], there, 3);
        mission.origin = Some(here);
        run_once(&mut mission, &world);
        assert_eq!(mission.end_code, 0);
    }

    #[test]
    fn the_adrift_order_s_mode_word_runs_two_passes_in_phase_two() {
        // FUN_00576a20's 0x4103: phase 2 mode 3 (mode 1's body, then mode
        // 2's), phase 7 mode 1, phase 9 mode 4; every other kind 0x4112.
        assert_eq!(detection_passes(MissionKind::Adrift, 2), &[1, 2]);
        assert_eq!(detection_passes(MissionKind::Adrift, 3), &[] as &[u8]);
        assert_eq!(detection_passes(MissionKind::Adrift, 7), &[1]);
        assert_eq!(detection_passes(MissionKind::Adrift, PHASE_BETRAYAL), &[4]);
        assert_eq!(detection_passes(MissionKind::Move, 2), &[2]);
        assert_eq!(detection_passes(MissionKind::Move, 3), &[1]);
        assert_eq!(detection_passes(MissionKind::Move, 5), &[] as &[u8]);
    }

    #[test]
    fn an_adrift_team_meets_the_enemy_fleets_as_it_sets_out() {
        // Mode 3's first pass is mode 1's body: the opponent's fleets at the
        // origin search a team bound elsewhere. A Move's phase 2 is mode 2,
        // the system's regiments only, so a fleet alone finds nothing there.
        // RLEVADTB 0: an exposed member is captured (slot +0x20c).
        let (mut world, here) = world_with(imperial());
        let there = system(&mut world, ControlKind::Uncontrolled);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 0);
        imperial_fleet(&mut world, here, false);
        for raw in [0x4400_0004, 0x4100_0001] {
            world.mission_records.push(MissionRecord {
                dat_id: DatId::new(raw),
                ..sabotage_record(false)
            });
        }
        let captured = |kind| {
            let mut world = world.clone();
            let lead = agent(&mut world, here, 100);
            let mut mission = sabotage(vec![lead], there, 2);
            mission.kind = kind;
            mission.origin = Some(here);
            run_once(&mut mission, &world)
                .iter()
                .any(|effect| matches!(effect, MissionEffect::CharacterCaptured { .. }))
        };
        assert!(captured(MissionKind::Adrift));
        assert!(!captured(MissionKind::Move));
    }

    #[test]
    fn an_adrift_team_the_first_pass_ends_meets_no_second_pass() {
        // FUN_00589a40 runs the validator on each pass: a team the fleets
        // captured on the first pass has every member's remove request
        // (FUN_004f18e0), so rule 1 ends the mission (code 5) and the
        // system's regiments expose no one again. Without an enemy fleet,
        // the second pass (mode 2's body) is the one that finds the team.
        let (mut world, here) = world_with(imperial());
        let there = system(&mut world, ControlKind::Uncontrolled);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 0);
        imperial_regiment(&mut world, here);
        world.mission_records.push(MissionRecord {
            dat_id: DatId::new(0x4400_0004),
            ..sabotage_record(false)
        });
        let captures = |world: &GameWorld| {
            let mut world = world.clone();
            let lead = agent(&mut world, here, 100);
            let mut mission = sabotage(vec![lead], there, 2);
            mission.kind = MissionKind::Adrift;
            mission.origin = Some(here);
            let out = run_once(&mut mission, &world);
            let count = out
                .iter()
                .filter(|effect| matches!(effect, MissionEffect::CharacterCaptured { .. }))
                .count();
            (count, mission.end_code)
        };
        assert_eq!(captures(&world), (1, 5), "the regiments, second pass");
        imperial_fleet(&mut world, here, false);
        assert_eq!(captures(&world), (1, 5), "the fleet, first pass only");
    }

    #[test]
    fn a_detected_team_that_may_not_resign_runs_on() {
        // Slot +0x1dc sets an end code only when the record lets members
        // resign (column 9), and FUN_00534640 requests nothing without it.
        let (mut world, here) = world_with(imperial());
        world.mission_records = vec![sabotage_record(false)];
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 100);
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 2);

        run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 0);
        assert!(mission.resigning.is_empty());
    }

    #[test]
    fn a_team_whose_every_member_resigns_ends_with_code_five() {
        // FUN_00522480.c:42-52: end 5 unless a team member stands without a
        // resign request.
        let (mut world, here) = world_with(imperial());
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 8);
        assert_eq!(
            running_end_code(
                &mission,
                &world,
                &UprisingState::default(),
                &crate::research::ResearchState::default(),
                &[]
            ),
            0
        );
        mission.resigning = vec![lead];
        assert_eq!(
            running_end_code(
                &mission,
                &world,
                &UprisingState::default(),
                &crate::research::ResearchState::default(),
                &[]
            ),
            5
        );
    }

    #[test]
    fn a_team_whose_every_member_has_a_remove_request_ends_with_code_five() {
        // FUN_00522480.c:42-52: a member stands only with role +0x78 & 0xc
        // clear, bit 2 being the remove request (FUN_005345d0).
        let (mut world, here) = world_with(imperial());
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 8);
        mission.removing = vec![lead];
        assert_eq!(
            running_end_code(
                &mission,
                &world,
                &UprisingState::default(),
                &crate::research::ResearchState::default(),
                &[]
            ),
            5
        );
    }

    #[test]
    fn a_captured_member_takes_a_remove_request_and_no_resign_request() {
        // Slot +0x20c FUN_004ef480 sets the status +0x98 to 0; its handler
        // FUN_004f18e0 sets the remove request of a member on a mission
        // (FUN_005345d0). FUN_005888f0.c:61-84 then gives a resign request
        // only to a member without a remove request.
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 0);
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 2);

        let out = run_once(&mut mission, &world);

        assert!(out
            .iter()
            .any(|effect| matches!(effect, MissionEffect::CharacterCaptured { .. })));
        assert_eq!(mission.removing, vec![lead]);
        assert!(mission.resigning.is_empty());
    }

    #[test]
    fn detection_draws_stay_in_range_and_repeat_for_the_same_seed() {
        // Our helper: draw(n) covers 0..=n (FUN_0053e290), 0 for n <= 0.
        let mut rng = MissionRng::seeded(0.5, 7);
        let draws: Vec<i32> = (0..200).map(|_| rng.draw(3)).collect();
        assert!(draws.iter().all(|d| (0..=3).contains(d)));
        assert!((0..=3).all(|n| draws.contains(&n)));
        let mut again = MissionRng::seeded(0.5, 7);
        assert_eq!(draws, (0..200).map(|_| again.draw(3)).collect::<Vec<_>>());
        let negative: Vec<i32> = (0..200).map(|_| rng.draw(-3)).collect();
        assert!((-3..=0).all(|n| negative.contains(&n)));
        assert!(!rng.chance(0));
        assert!(rng.chance(100));
    }

    #[test]
    fn detection_draws_are_splitmix64_seeded_by_the_roll_and_the_mission() {
        // Our helper: the published SplitMix64 first output from state 0,
        // and a mission id that changes the stream.
        assert_eq!(MissionRng::seeded(0.0, 0).next_u64(), 0xe220_a839_7b1d_cdaf);
        assert_eq!(MissionRng::seeded(0.5, 0).state, 1 << 52);
        let first = |roll, id| MissionRng::seeded(roll, id).next_u64();
        assert_ne!(first(0.0, 1), first(0.0, 0));
        assert_ne!(first(0.5, 1), first(0.0, 1));
    }

    #[test]
    fn the_defender_walk_takes_regiments_then_each_fleet_s_live_ships_then_fighters() {
        // FUN_00587640: the system's regiments, then per fleet at the
        // location its ships and fighters (port: fighters after all ships).
        let (mut world, here) = world_with(imperial());
        let there = system(&mut world, imperial());
        let troop = imperial_regiment(&mut world, here);
        let fleet = imperial_fleet(&mut world, here, false);
        world.fleets[fleet].capital_ships.push(ShipInstance {
            class: crate::ids::CapitalShipKey::default(),
            hull_current: 0,
            subsystem_hits: 0,
            alive: false,
            name: None,
            id: 0,
        });
        let tie = world
            .fighter_classes
            .insert(crate::world::FighterClass::default());
        world.fleets[fleet]
            .fighters
            .push(crate::world::FighterEntry {
                class: crate::ids::FighterKey::default(),
                count: 2,
            });
        crate::squadrons::add(&mut world.systems[here].fighters, tie, 1);
        imperial_fleet(&mut world, there, false);
        let ids = |flags, kinds| -> Vec<DefenderId> {
            walk(&world, here, flags, kinds)
                .iter()
                .map(|d| d.id)
                .collect()
        };
        let both = Flags {
            system: true,
            fleets: true,
            ..Flags::default()
        };
        let only = |ships, fighters, regiments| Kinds {
            ships,
            fighters,
            regiments,
        };

        // FUN_00587640: the system's squadrons (FUN_005039d0), then its
        // regiments (FUN_00504c40), then each fleet (FUN_004ffe70).
        assert_eq!(
            ids(both, ALL),
            vec![
                DefenderId::PlanetFighter(0, 0),
                DefenderId::Regiment(troop),
                DefenderId::Ship(fleet, 0),
                DefenderId::Fighter(fleet, 0, 0),
                DefenderId::Fighter(fleet, 0, 1),
            ]
        );
        assert_eq!(
            ids(both, only(true, false, false)),
            vec![DefenderId::Ship(fleet, 0)]
        );
        assert_eq!(
            ids(both, only(false, false, true)),
            vec![DefenderId::Regiment(troop)]
        );
        let fleets = Flags {
            fleets: true,
            ..Flags::default()
        };
        assert_eq!(ids(fleets, only(false, false, true)), vec![]);
        let system_only = Flags {
            system: true,
            ..Flags::default()
        };
        assert_eq!(ids(system_only, only(true, true, false)), vec![]);
    }

    #[test]
    fn a_detection_roll_reads_the_team_s_average_espionage_less_detection_forces_and_offset() {
        // FUN_005896e0: x = avg team espionage - detection - special forces
        // - G3584 (shipped -1). Espionage 30, 10 and 20 average 20; a
        // resigning member's 90 is not counted; detection 5 and one special
        // force give x = 20 - 5 - 1 + 1 = 15.
        let (mut world, here) = world_with(imperial());
        regiment(&mut world, here, false, 5);
        band(&mut world, FOILTB, 15, 15);
        let team = vec![
            skilled(&mut world, here, 0x30, 30, 0),
            skilled(&mut world, here, 0x30, 10, 0),
            special_force(&mut world, here, 20),
            skilled(&mut world, here, 0x30, 90, 0),
        ];
        let mut mission = sabotage(team.clone(), here, 2);
        mission.resigning = vec![team[3]];

        run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 3);
    }

    #[test]
    fn a_decoy_roll_reads_the_decoy_s_espionage_less_the_defender_s_detection() {
        // FUN_00589620: x = decoy espionage - defender detection, with no
        // officer. Espionage 10 against detection 4 gives x = 6.
        let (mut world, here) = world_with(imperial());
        regiment(&mut world, here, false, 4);
        table(&mut world, FOILTB, 100);
        band(&mut world, TDECOYTB, 6, 6);
        let lead = agent(&mut world, here, 100);
        let decoy = skilled(&mut world, here, 0x30, 10, 0);
        let mut mission = sabotage(vec![lead], here, 2);
        mission.decoys = vec![decoy];

        run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 0);
    }

    /// A character of `alliance`'s side on the planet `at` holding `kind`.
    fn officer(
        world: &mut GameWorld,
        at: SystemKey,
        alliance: bool,
        kind: CommandKind,
        espionage: u32,
        combat: u32,
    ) -> CharacterKey {
        let pair = |base| SkillPair { base, variance: 0 };
        world.characters.insert(Character {
            name: "Officer".into(),
            is_alliance: alliance,
            is_empire: !alliance,
            espionage: pair(espionage),
            combat: pair(combat),
            current_system: Some(at),
            command: kind,
            ..Default::default()
        })
    }

    #[test]
    fn a_regiment_s_general_takes_part_of_its_espionage_off_the_decoy_and_foiltb_rolls() {
        // A regiment's slot +0x1bc is 3, a General (FUN_00526f00); with no
        // fleet the officer is the system's (FUN_00509330, the system's
        // side). FUN_0055e410: x = decoy espionage - detection - officer
        // espionage * G3588 (35) / 100; FUN_0055e470 the same with G3589
        // (35). Espionage 20 takes 20 * 35 / 100 = 7 off.
        let decoyed = |officers: &[(bool, CommandKind)]| {
            let (mut world, here) = world_with(imperial());
            regiment(&mut world, here, false, 4);
            table(&mut world, FOILTB, 100);
            band(&mut world, TDECOYTB, -1, -1);
            for &(alliance, kind) in officers {
                officer(&mut world, here, alliance, kind, 20, 0);
            }
            let lead = agent(&mut world, here, 100);
            let decoy = skilled(&mut world, here, 0x30, 10, 0);
            let mut mission = sabotage(vec![lead], here, 2);
            mission.decoys = vec![decoy];
            run_once(&mut mission, &world);
            mission.end_code == 0
        };
        // Without a general x = 10 - 4 = 6 misses the band: detected.
        assert!(!decoyed(&[]));
        // Another rank, or the other side's general, backs nothing.
        assert!(!decoyed(&[(false, CommandKind::Admiral), (true, CommandKind::General)]));
        // The Empire's general: x = 6 - 7 = -1, the regiment is drawn off.
        assert!(decoyed(&[(false, CommandKind::General)]));

        // FOILTB: espionage 20 - detection 5 + 1 = 16 alone; 9 with the
        // general.
        let detected = |general: bool| {
            let (mut world, here) = world_with(imperial());
            regiment(&mut world, here, false, 5);
            band(&mut world, FOILTB, 9, 9);
            if general {
                officer(&mut world, here, false, CommandKind::General, 20, 0);
            }
            let lead = skilled(&mut world, here, 0x30, 20, 0);
            let mut mission = sabotage(vec![lead], here, 2);
            run_once(&mut mission, &world);
            mission.end_code == 3
        };
        assert!(!detected(false));
        assert!(detected(true));
    }

    #[test]
    fn a_ship_is_backed_by_its_fleet_s_admiral_a_squadron_by_a_commander_and_a_regiment_by_a_general() {
        // Slot +0x1bc (decoy-roll.md, "Officer rank and detector slots"):
        // capital ship 2 (0x0043c1f0), fighter squadron 1 (FUN_0040f340),
        // regiment 3 (FUN_00526f00); FUN_004fd790 on the defender's fleet,
        // else FUN_00509330 on the system.
        let (mut world, here) = world_with(imperial());
        let fleet = imperial_fleet(&mut world, here, false);
        let aboard = |world: &mut GameWorld, kind| {
            let c = officer(world, here, false, kind, 0, 0);
            world.characters[c].current_fleet = Some(fleet);
            c
        };
        let admiral = aboard(&mut world, CommandKind::Admiral);
        let commander = aboard(&mut world, CommandKind::Commander);
        aboard(&mut world, CommandKind::General);
        let general = officer(&mut world, here, false, CommandKind::General, 0, 0);
        let planet_commander = officer(&mut world, here, false, CommandKind::Commander, 0, 0);
        let troop = imperial_regiment(&mut world, here);
        let defender = |id| Defender {
            id,
            in_fleet: false,
            detection: 0,
            side: Faction::Empire,
        };
        let officer_of = |id| defender(id).officer(&world, here);
        assert_eq!(officer_of(DefenderId::Ship(fleet, 0)), Some(admiral));
        assert_eq!(officer_of(DefenderId::Fighter(fleet, 0, 0)), Some(commander));
        assert_eq!(officer_of(DefenderId::Regiment(troop)), Some(general));
        assert_eq!(
            officer_of(DefenderId::PlanetFighter(0, 0)),
            Some(planet_commander)
        );
    }

    #[test]
    fn an_exposed_member_rolls_against_and_is_captured_by_the_detector_s_officer() {
        // FUN_005888f0: the officer's key and slot +0x1f0 (effective combat),
        // else the defender's and 0; FUN_005349e0: RLEVADTB at member combat
        // less it. Combat 30 against the general's 30 evades at x = 0.
        let run_with = |general: bool| {
            let (mut world, here) = world_with(imperial());
            imperial_regiment(&mut world, here);
            table(&mut world, FOILTB, 100);
            band(&mut world, RLEVADTB, 0, 0);
            if general {
                officer(&mut world, here, false, CommandKind::General, 0, 30);
            }
            let lead = skilled(&mut world, here, 0x30, 0, 30);
            let mut mission = sabotage(vec![lead], here, 2);
            run_once(&mut mission, &world)
        };
        let captured = |out: &[MissionEffect]| {
            out.iter().any(|e| {
                matches!(
                    e,
                    MissionEffect::CharacterCaptured {
                        captured_by: MissionFaction::Empire,
                        ..
                    }
                )
            })
        };
        assert!(captured(&run_with(false)));
        assert!(!captured(&run_with(true)));
    }

    #[test]
    fn an_injured_minor_member_dies_on_the_death_roll_and_a_major_one_never() {
        // Slot +0x2f0 (FUN_004ef6d0, reason 10) after the evasion injury:
        // slot +0x2e0 kills a minor character with GNPRTB 2577 (100 here);
        // a major character's (FUN_006158b0) never does.
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 100);
        world.gnprtb = crate::world::GnprtbParams::uniform(&[(2565, 100), (2577, 100)]);
        let lead = skilled(&mut world, here, 0x30, 0, 30);
        let mut mission = sabotage(vec![lead], here, 2);
        let out = run_seeded(&mut mission, &world, 0.0);
        assert!(out.contains(&MissionEffect::DiedOfInjury {
            character: character(lead)
        }));

        let MissionMember::Character(key) = lead else {
            unreachable!()
        };
        world.characters[key].is_major = true;
        let mut mission = sabotage(vec![lead], here, 2);
        let out = run_seeded(&mut mission, &world, 0.0);
        assert!(out
            .iter()
            .any(|e| matches!(e, MissionEffect::CharacterInjured { .. })));
        assert!(!out
            .iter()
            .any(|e| matches!(e, MissionEffect::DiedOfInjury { .. })));
    }

    #[test]
    fn an_evading_member_s_injury_draws_after_the_detection_and_evasion_rolls() {
        // FUN_0053e990 with combat 30: chance max(G2565, 70) = 70, injury
        // rand(70) + rand(G2567 = 29) + G2566 = 1. Before it the run draws
        // FOILTB's chance, the defender pick draw(0), and RLEVADTB's chance;
        // after it slot +0x2e0 rolls a minor character's death (G2577, 25).
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 100);
        let lead = skilled(&mut world, here, 0x30, 0, 30);
        let mut mission = sabotage(vec![lead], here, 2);

        let out = run_seeded(&mut mission, &world, 0.0);

        let mut expected = MissionRng::seeded(0.0, 0);
        expected.draw(99);
        expected.draw(0);
        expected.draw(99);
        assert!(expected.chance(70), "the seed should reach the injury");
        let injury = expected.draw(70) + expected.draw(29) + 1;
        let mut effects = vec![MissionEffect::CharacterInjured {
            character: character(lead),
            injury,
        }];
        if expected.chance(25) {
            effects.push(MissionEffect::DiedOfInjury {
                character: character(lead),
            });
        }
        assert_eq!(out, effects);
    }

    #[test]
    fn an_exposed_team_faces_defenders_in_character_family_order_then_special_forces() {
        // FUN_0058a1c0: team characters of families 0x30..0x37, then
        // 0x38..0x3b, then team special forces, whatever the list order.
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 0);
        let unit = special_force(&mut world, here, 0);
        let late = skilled(&mut world, here, 0x38, 0, 0);
        let early = skilled(&mut world, here, 0x37, 0, 0);
        let mut mission = sabotage(vec![unit, late, early], here, 2);

        let out = run_once(&mut mission, &world);

        let order: Vec<MissionMember> = out
            .iter()
            .filter_map(|effect| match effect {
                MissionEffect::CharacterCaptured { character, .. } => {
                    Some(MissionMember::Character(*character))
                }
                MissionEffect::SpecialForceDestroyed { unit } => {
                    Some(MissionMember::SpecialForce(*unit))
                }
                _ => None,
            })
            .collect();
        assert_eq!(order, vec![early, late, unit]);
    }

    #[test]
    fn a_loaded_major_character_is_exposed_before_a_minor_one() {
        // The loader keeps the record index without the family byte
        // (MJCHARSD 0x240, MNCHARSD 0x340), so is_major places the major
        // first (FUN_0058a1c0).
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 0);
        let minor = skilled(&mut world, here, 0, 0, 0);
        world.characters[character(minor)].dat_id = DatId::new(0x340);
        let major = skilled(&mut world, here, 0, 0, 0);
        world.characters[character(major)].dat_id = DatId::new(0x240);
        world.characters[character(major)].is_major = true;
        let mut mission = sabotage(vec![minor, major], here, 2);

        let out = run_once(&mut mission, &world);

        let order: Vec<CharacterKey> = out
            .iter()
            .filter_map(|effect| match effect {
                MissionEffect::CharacterCaptured { character, .. } => Some(*character),
                _ => None,
            })
            .collect();
        assert_eq!(order, [character(major), character(minor)]);
    }

    #[test]
    fn an_alliance_defender_captures_for_the_alliance() {
        // Slot +0x20c: the captor is the defender's side.
        let (mut world, here) = world_with(ControlKind::Controlled(Faction::Alliance));
        regiment(&mut world, here, true, 0);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 0);
        let imperial_agent = world.characters.insert(Character {
            is_empire: true,
            loyalty: SkillPair {
                base: 100,
                variance: 0,
            },
            current_system: Some(here),
            ..Default::default()
        });
        let mut mission = sabotage(vec![MissionMember::Character(imperial_agent)], here, 2);
        mission.faction = MissionFaction::Empire;

        let out = run_once(&mut mission, &world);

        assert!(out.iter().any(|effect| matches!(
            effect,
            MissionEffect::CharacterCaptured {
                captured_by: MissionFaction::Alliance,
                ..
            }
        )));
    }

    #[test]
    fn a_destroyed_decoy_is_not_drawn_again() {
        // FUN_00503eb0 destroys a captured special force; without a resign
        // request (column 9 off) it stays in the decoy count, and the next
        // defender's pick finds no decoy left.
        let (mut world, here) = world_with(imperial());
        world.mission_records = vec![sabotage_record(false)];
        imperial_regiment(&mut world, here);
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 0);
        table(&mut world, TDECOYTB, 0);
        table(&mut world, RLEVADTB, 0);
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 2);
        mission.decoys = vec![special_force(&mut world, here, 0)];

        let out = run_once(&mut mission, &world);

        let destroyed = out
            .iter()
            .filter(|effect| matches!(effect, MissionEffect::SpecialForceDestroyed { .. }))
            .count();
        assert_eq!(destroyed, 1);
    }

    #[test]
    fn a_decoy_that_evades_without_resigning_meets_the_next_defender_too() {
        // FUN_005888f0.c:61-84: without a resign request (column 9 off) an
        // evading decoy stays in the decoy count, so the second regiment's
        // pick lands on it again; each evasion rolls the injury (combat 0,
        // chance 100).
        let (mut world, here) = world_with(imperial());
        world.mission_records = vec![sabotage_record(false)];
        imperial_regiment(&mut world, here);
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 0);
        table(&mut world, TDECOYTB, 0);
        table(&mut world, RLEVADTB, 100);
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 2);
        mission.decoys = vec![agent(&mut world, here, 100)];

        let out = run_once(&mut mission, &world);

        assert_eq!(out.len(), 2);
        assert!(out
            .iter()
            .all(|effect| matches!(effect, MissionEffect::CharacterInjured { .. })));
    }

    #[test]
    fn a_captured_decoy_is_not_drawn_again() {
        // Slot +0x20c takes the decoy prisoner; port: the run marks it gone,
        // so without a resign request the next defender finds no decoy.
        let (mut world, here) = world_with(imperial());
        world.mission_records = vec![sabotage_record(false)];
        imperial_regiment(&mut world, here);
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 0);
        table(&mut world, TDECOYTB, 0);
        table(&mut world, RLEVADTB, 0);
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 2);
        mission.decoys = vec![agent(&mut world, here, 100)];

        let out = run_once(&mut mission, &world);

        let captured = out
            .iter()
            .filter(|effect| matches!(effect, MissionEffect::CharacterCaptured { .. }))
            .count();
        assert_eq!(captured, 1);
    }

    #[test]
    fn each_defender_meets_a_decoy_that_has_not_resigned() {
        // FUN_005888f0.c:61-84: a resigning decoy leaves the decoy count, so
        // the next defender's pick draw(0..decoys-1) lands on the other one.
        for id in 0..16 {
            let (mut world, here) = world_with(imperial());
            imperial_regiment(&mut world, here);
            imperial_regiment(&mut world, here);
            table(&mut world, FOILTB, 0);
            table(&mut world, TDECOYTB, 0);
            table(&mut world, RLEVADTB, 100);
            let lead = agent(&mut world, here, 100);
            let decoys = vec![agent(&mut world, here, 100), agent(&mut world, here, 100)];
            let mut mission = sabotage(vec![lead], here, 2);
            mission.id = id;
            mission.decoys = decoys.clone();

            run_once(&mut mission, &world);

            assert_eq!(mission.resigning.len(), 2, "mission {id}");
            assert!(decoys.iter().all(|d| mission.resigning.contains(d)));
        }
    }

    #[test]
    fn a_decoyed_defender_leaves_the_count_the_exposed_team_draws_from() {
        // FUN_0058a020: a decoyed defender leaves +0x34, so each exposed team
        // member's pick draw(0..defenders-1) lands on the one left. The decoy
        // draws off detection 0 (x = 10) and fails against detection 50.
        for id in 0..16 {
            let (mut world, here) = world_with(imperial());
            regiment(&mut world, here, false, 0);
            regiment(&mut world, here, false, 50);
            table(&mut world, FOILTB, 100);
            band(&mut world, TDECOYTB, 0, 100);
            table(&mut world, RLEVADTB, 100);
            let lead = agent(&mut world, here, 100);
            let mut mission = sabotage(vec![lead], here, 2);
            mission.id = id;
            mission.decoys = vec![skilled(&mut world, here, 0x30, 10, 0)];

            run_once(&mut mission, &world);

            assert!(mission.resigning.contains(&lead), "mission {id}");
        }
    }

    #[test]
    fn past_phase_four_the_run_looks_at_the_target_not_the_origin() {
        // FUN_00520ad0: the target location +0x78 past phase 4. Phase 7 is
        // mode 1; the opponent's fleet waits at the target only.
        let (mut world, here) = world_with(imperial());
        let there = system(&mut world, ControlKind::Uncontrolled);
        table(&mut world, FOILTB, 100);
        imperial_fleet(&mut world, here, false);
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, 7);
        mission.origin = Some(there);

        run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 4);
    }

    #[test]
    fn phase_two_counts_no_member_aboard_a_fleet() {
        // FUN_00589a40 mode 2 sets only the at-system member filter (+0x18).
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        let fleet = imperial_fleet(&mut world, here, true);
        let lead = agent(&mut world, here, 100);
        world.characters[character(lead)].current_fleet = Some(fleet);
        let mut mission = sabotage(vec![lead], here, 2);

        run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 0);
    }

    #[test]
    fn a_phase_nine_system_mission_is_detected_by_the_holder_s_regiments() {
        // FUN_00589a40 mode 4: a system or object target walks the system
        // when the opponent holds it.
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        let lead = agent(&mut world, here, 100);
        let mut mission = sabotage(vec![lead], here, PHASE_BETRAYAL);

        run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 4);
    }

    /// A Force user of record `raw` at `at`: Alliance or Empire, with Force
    /// base `force` and combat `combat`.
    fn force_user(
        world: &mut GameWorld,
        at: SystemKey,
        raw: u32,
        is_alliance: bool,
        force: u16,
        combat: u32,
    ) -> CharacterKey {
        world.characters.insert(Character {
            dat_id: DatId::new(raw),
            name: format!("{raw:#x}"),
            is_alliance,
            is_empire: !is_alliance,
            is_major: true,
            loyalty: SkillPair {
                base: 100,
                variance: 0,
            },
            combat: SkillPair {
                base: combat,
                variance: 0,
            },
            force_base: force,
            current_system: Some(at),
            ..Default::default()
        })
    }

    /// Luke `0x32000242`, Leia `0x31000241`, Vader `0x35000281` and the
    /// Emperor `0x34000280` (TEXTSTRA names, `encounter.md`).
    const LUKE_ID: u32 = 0x3200_0242;
    const LEIA_ID: u32 = 0x3100_0241;
    const VADER_ID: u32 = 0x3500_0281;
    const EMPEROR_ID: u32 = 0x3400_0280;

    /// Give `a` and `b` each other's `Encounter` key on `day`.
    fn encountered(world: &mut GameWorld, a: CharacterKey, b: CharacterKey, day: u64) {
        crate::missions::begin_encounter(world, a, b, day);
    }

    fn encounters_begun(out: &[MissionEffect]) -> Vec<(CharacterKey, CharacterKey)> {
        out.iter()
            .filter_map(|effect| match effect {
                MissionEffect::EncounterBegan {
                    member, defender, ..
                } => Some((*member, *defender)),
                _ => None,
            })
            .collect()
    }

    fn force_gains(out: &[MissionEffect], who: CharacterKey) -> Vec<i32> {
        out.iter()
            .filter_map(|effect| match effect {
                MissionEffect::ForceExperienceGained { character, amount } if *character == who => {
                    Some(*amount)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn vader_detects_luke_by_their_force_and_starts_their_encounter() {
        // FUN_0058a3f0 / FUN_0055e4d0: a chance of 100 + 120 - 100 (GNPRTB
        // 3587) detects the mission; FUN_0055ff60 sets the keys on today and
        // both sides observe the system (FUN_0050d5a0(6, 1), (6, 2)).
        let (mut world, here) = world_with(imperial());
        world.systems[here].exploration_status = crate::dat::ExplorationStatus::Unexplored;
        let luke = force_user(&mut world, here, LUKE_ID, true, 100, 50);
        let vader = force_user(&mut world, here, VADER_ID, false, 120, 50);
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), [(luke, vader)]);
        assert!(out.contains(&MissionEffect::EncounterBegan {
            member: luke,
            defender: vader,
            day: 1,
        }));
        for faction in [MissionFaction::Alliance, MissionFaction::Empire] {
            assert!(out.contains(&MissionEffect::SystemIntelligenceGathered {
                system: here,
                faction,
            }));
        }
        // Detected: slot +0x1dc receives 3 in phase 2.
        assert_eq!(mission.end_code, 3);
        // The detected Luke confronts Vader at once (FUN_00589360).
        assert!(out.contains(&MissionEffect::EncounterEnded {
            member: luke,
            partner: vader,
        }));
        crate::missions::apply_advance_effects(&mut world, &out);
        assert_eq!(world.characters[luke].encounter, None);
        assert_eq!(world.characters[luke].encounter_day, 0);
        assert!(world.characters[luke].heritage_known);
        assert_eq!(
            world.systems[here].exploration_status,
            crate::dat::ExplorationStatus::Explored
        );
    }

    #[test]
    fn loaded_force_users_without_a_family_byte_still_meet() {
        // The loader's Luke is 0x242 and Vader 0x281 (MJCHARSD), both
        // major characters of the sets +0x50 and +0x68 (FUN_0058a5f0,
        // FUN_0058a6c0).
        let (mut world, here) = world_with(imperial());
        let luke = force_user(&mut world, here, 0x242, true, 100, 50);
        let vader = force_user(&mut world, here, 0x281, false, 120, 50);
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), [(luke, vader)]);
        assert!(out.contains(&MissionEffect::HeritageLearned { character: luke }));
    }

    #[test]
    fn a_minor_force_user_defends_after_the_major_ones() {
        // FUN_0058a9e0: a random member of +0x68 (major) before +0x60
        // (minor, MNCHARSD); the minor one never meets Luke once the major
        // one paired with him.
        let (mut world, here) = world_with(imperial());
        let luke = force_user(&mut world, here, LUKE_ID, true, 100, 50);
        let minor = force_user(&mut world, here, 0x340, false, 200, 50);
        world.characters[minor].is_major = false;
        let major = force_user(&mut world, here, 0x3400_0001, false, 120, 50);
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), [(luke, major)]);
    }

    #[test]
    fn a_defender_below_force_sixty_detects_no_force_user() {
        // FUN_00588840: a defender counts with Force >= GNPRTB 3586 (60).
        let (mut world, here) = world_with(imperial());
        let luke = force_user(&mut world, here, LUKE_ID, true, 200, 50);
        force_user(&mut world, here, VADER_ID, false, 59, 50);
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), []);
        assert_eq!(mission.end_code, 0);
    }

    #[test]
    fn force_users_whose_forces_sum_to_100_or_less_never_meet() {
        // FUN_0055e4d0: 40 + 60 + GNPRTB 3587 (-100) is no chance, and no
        // draw; Force exactly GNPRTB 3586 (60) still defends (FUN_0055e590:
        // 60 <= force).
        let (mut world, here) = world_with(imperial());
        let luke = force_user(&mut world, here, LUKE_ID, true, 40, 50);
        force_user(&mut world, here, VADER_ID, false, 60, 50);
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), []);
        world.characters[luke].force_base = 200;
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);
        let out = run_once(&mut mission, &world);
        assert_eq!(encounters_begun(&out).len(), 1);
    }

    #[test]
    fn a_zero_force_chance_draws_nothing() {
        // FUN_0055e4d0 rolls FUN_0053e2f0 only for a positive chance, so a
        // pair whose chance is 0 (40 + 60 - 100) leaves the run's later
        // draws as if Vader stood elsewhere.
        for roll in [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9] {
            let outcome = |vader_here: bool| {
                let (mut world, here) = world_with(imperial());
                imperial_regiment(&mut world, here);
                table(&mut world, FOILTB, 50);
                table(&mut world, RLEVADTB, 50);
                let luke = force_user(&mut world, here, LUKE_ID, true, 40, 50);
                let vader = force_user(&mut world, here, VADER_ID, false, 60, 50);
                if !vader_here {
                    world.characters[vader].current_system = None;
                }
                let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);
                let out = run_seeded(&mut mission, &world, roll);
                (mission.end_code, out)
            };
            assert_eq!(outcome(true), outcome(false), "roll {roll}");
        }
    }

    #[test]
    fn force_training_counts_toward_a_defender_s_force() {
        // Force +0x8c = ForceExperience +0x8e + ForceTraining +0x90: 40 + 30
        // reaches GNPRTB 3586 (60).
        let (mut world, here) = world_with(imperial());
        let luke = force_user(&mut world, here, LUKE_ID, true, 200, 50);
        let vader = force_user(&mut world, here, VADER_ID, false, 40, 50);
        world.characters[vader].force_training = 30;
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), [(luke, vader)]);
    }

    #[test]
    fn a_force_user_aboard_a_fleet_defends_only_with_the_fleet_flag() {
        // FUN_0058a6c0 walks the fleets' ships only with +0x24: mode 1 sets
        // it when only the opponent has a fleet at the origin (phase 3,
        // bound elsewhere); mode 2 (phase 2) never does.
        let (mut world, here) = world_with(imperial());
        let there = system(&mut world, ControlKind::Uncontrolled);
        let fleet = imperial_fleet(&mut world, here, false);
        let vader = force_user(&mut world, here, VADER_ID, false, 120, 50);
        world.characters[vader].current_fleet = Some(fleet);
        world.characters[vader].current_system = None;
        world.fleets[fleet].characters.push(vader);
        let luke = force_user(&mut world, here, LUKE_ID, true, 100, 50);

        let mut bound = sabotage(vec![MissionMember::Character(luke)], there, 3);
        bound.origin = Some(here);
        let out = run_once(&mut bound, &world);
        assert_eq!(encounters_begun(&out), [(luke, vader)]);

        let mut local = sabotage(vec![MissionMember::Character(luke)], here, 2);
        let out = run_once(&mut local, &world);
        assert_eq!(encounters_begun(&out), []);

        // A fleet at another system is not walked.
        world.fleets[fleet].location = there;
        let mut bound = sabotage(vec![MissionMember::Character(luke)], there, 3);
        bound.origin = Some(here);
        let out = run_once(&mut bound, &world);
        assert_eq!(encounters_begun(&out), []);
    }

    #[test]
    fn a_force_user_at_a_system_defends_only_with_the_system_flag() {
        // Mode 2 sets +0x20 only when the opponent holds the system.
        let (mut world, here) = world_with(ControlKind::Uncontrolled);
        let luke = force_user(&mut world, here, LUKE_ID, true, 100, 50);
        force_user(&mut world, here, VADER_ID, false, 120, 50);
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), []);
    }

    #[test]
    fn a_killed_force_user_defends_no_more() {
        // port: the port keeps a killed character in the arena
        // (Character::mark_killed); the original deletes it.
        let (mut world, here) = world_with(imperial());
        let luke = force_user(&mut world, here, LUKE_ID, true, 200, 50);
        let vader = force_user(&mut world, here, VADER_ID, false, 200, 50);
        world.characters[vader].is_killed = true;
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), []);
    }

    #[test]
    fn minor_force_users_defend_and_are_detected() {
        // +0x60 and +0x58 hold the minor characters (MNCHARSD, 0x38..0x3b).
        let (mut world, here) = world_with(imperial());
        let luke = force_user(&mut world, here, LUKE_ID, true, 100, 50);
        let inquisitor = force_user(&mut world, here, 0x340, false, 120, 50);
        world.characters[inquisitor].is_major = false;
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);
        let out = run_once(&mut mission, &world);
        assert_eq!(encounters_begun(&out), [(luke, inquisitor)]);

        let (mut world, here) = world_with(imperial());
        let pupil = force_user(&mut world, here, 0x341, true, 100, 50);
        world.characters[pupil].is_major = false;
        let vader = force_user(&mut world, here, VADER_ID, false, 120, 50);
        let mut mission = sabotage(vec![MissionMember::Character(pupil)], here, 2);
        let out = run_once(&mut mission, &world);
        assert_eq!(encounters_begun(&out), [(pupil, vader)]);
    }

    #[test]
    fn a_captive_or_same_side_force_user_detects_nothing() {
        // FUN_00588840: free (+0xac bit 0 clear) and of the other side.
        let (mut world, here) = world_with(imperial());
        let luke = force_user(&mut world, here, LUKE_ID, true, 200, 50);
        let vader = force_user(&mut world, here, VADER_ID, false, 200, 50);
        world.characters[vader].is_captive = true;
        world.characters[vader].captured_by = Some(Faction::Alliance);
        force_user(&mut world, here, LEIA_ID, true, 200, 50);
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), []);
    }

    #[test]
    fn a_member_without_force_is_never_rolled_against() {
        // FUN_0055e4d0: no chance unless both Forces are non-zero.
        let (mut world, here) = world_with(imperial());
        let agent = force_user(&mut world, here, 0x3000_0001, true, 0, 50);
        force_user(&mut world, here, VADER_ID, false, 200, 50);
        let mut mission = sabotage(vec![MissionMember::Character(agent)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), []);
        assert_eq!(mission.end_code, 0);
    }

    #[test]
    fn a_decoy_detected_by_force_starts_an_encounter_without_detecting_the_mission() {
        // FUN_0058a3f0: a member with +0x78 bit 0 (a decoy) leaves +0x4c.
        let (mut world, here) = world_with(imperial());
        let lead = agent(&mut world, here, 100);
        let luke = force_user(&mut world, here, LUKE_ID, true, 100, 50);
        let vader = force_user(&mut world, here, VADER_ID, false, 120, 50);
        let mut mission = sabotage(vec![lead], here, 2);
        mission.decoys.push(MissionMember::Character(luke));

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), [(luke, vader)]);
        assert_eq!(mission.end_code, 0);
        assert!(!out
            .iter()
            .any(|effect| matches!(effect, MissionEffect::EncounterEnded { .. })));
    }

    #[test]
    fn vader_meets_luke_before_any_other_member() {
        // FUN_0058ab00: Luke (FUN_0058ac50) is picked before the random
        // members of +0x50, whatever the draw.
        for roll in [0.0, 0.3, 0.6, 0.9] {
            let (mut world, here) = world_with(imperial());
            let mut team: Vec<MissionMember> = (1..=4)
                .map(|n| {
                    MissionMember::Character(force_user(
                        &mut world,
                        here,
                        0x3000_0000 | n,
                        true,
                        100,
                        50,
                    ))
                })
                .collect();
            let luke = force_user(&mut world, here, LUKE_ID, true, 100, 50);
            team.push(MissionMember::Character(luke));
            let vader = force_user(&mut world, here, VADER_ID, false, 120, 50);
            let mut mission = sabotage(team, here, 2);

            let out = run_seeded(&mut mission, &world, roll);

            assert_eq!(encounters_begun(&out), [(luke, vader)], "roll {roll}");
        }
    }

    #[test]
    fn a_defender_holding_an_encounter_meets_only_one_member() {
        // FUN_0058a2c0's do-while stops once the defender holds +0x9c; a hit
        // on two holders starts nothing new (FUN_0055ff60) but detects.
        let (mut world, here) = world_with(imperial());
        let luke = force_user(&mut world, here, LUKE_ID, true, 100, 50);
        let other = force_user(&mut world, here, 0x3000_0001, true, 100, 50);
        let vader = force_user(&mut world, here, VADER_ID, false, 120, 50);
        let elsewhere = force_user(&mut world, here, 0x3500_0001, false, 0, 50);
        encountered(&mut world, vader, elsewhere, 3);
        let mut mission = sabotage(
            vec![
                MissionMember::Character(other),
                MissionMember::Character(luke),
            ],
            here,
            2,
        );

        let out = run_once(&mut mission, &world);

        assert_eq!(encounters_begun(&out), []);
        assert_eq!(mission.end_code, 3);
    }

    #[test]
    fn the_confrontation_needs_both_keys_on_the_same_day() {
        // FUN_00560200 returns before the duel unless each names the other
        // and both encounter days (FUN_004ed300) match; the member then
        // faces no random defender either (FUN_00589360).
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        table(&mut world, RLEVADTB, 0);
        let luke = force_user(&mut world, here, LUKE_ID, true, 0, 50);
        let vader = force_user(&mut world, here, VADER_ID, false, 0, 50);
        encountered(&mut world, luke, vader, 4);
        world.characters[vader].encounter_day = 5;
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(mission.end_code, 3);
        assert!(!out.iter().any(|effect| matches!(
            effect,
            MissionEffect::EncounterEnded { .. } | MissionEffect::CharacterCaptured { .. }
        )));
    }

    #[test]
    fn the_confrontation_s_evasion_reads_combat_less_the_partner_s() {
        // FUN_005349e0: RLEVADTB at the member's effective combat less the
        // partner's slot +0x1f0 (+0x86, with the Force share); a band at
        // that difference evades, else the partner captures.
        let (mut world, here) = world_with(imperial());
        let member = force_user(&mut world, here, 0x3000_0001, true, 100, 50);
        let partner = force_user(&mut world, here, 0x3500_0001, false, 120, 80);
        let effective = |key: CharacterKey| {
            i32::try_from(world.characters[key].effective_skill(Skill::Combat)).unwrap()
        };
        let x = effective(member) - effective(partner);
        assert!(x < 0);
        band(&mut world, RLEVADTB, x, x);
        let mut mission = sabotage(vec![MissionMember::Character(member)], here, 2);

        let out = run_once(&mut mission, &world);

        assert!(out.contains(&MissionEffect::EncounterEnded { member, partner }));
        assert!(!out
            .iter()
            .any(|effect| matches!(effect, MissionEffect::CharacterCaptured { .. })));
        assert!(mission.resigning.contains(&MissionMember::Character(member)));
    }

    #[test]
    fn the_stronger_duelist_injures_the_weaker_and_gains_combat() {
        // FUN_0053ea70: the member's chance is max(GNPRTB 2571, partner -
        // member); the injury rand(0..=chance) + rand(0..=2573) + 2572 goes
        // through slot +0x2f0, and the partner's base combat +0x62 rises by
        // GNPRTB 2578 (FUN_005340a0).
        let (mut world, here) = world_with(imperial());
        imperial_regiment(&mut world, here);
        table(&mut world, FOILTB, 100);
        let member = force_user(&mut world, here, 0x3000_0001, true, 0, 0);
        let partner = force_user(&mut world, here, 0x3500_0001, false, 0, 200);
        encountered(&mut world, member, partner, 2);
        let mut mission = sabotage(vec![MissionMember::Character(member)], here, 2);

        let out = run_once(&mut mission, &world);

        let injury = out.iter().find_map(|effect| match effect {
            MissionEffect::CharacterInjured { character, injury } if *character == member => {
                Some(*injury)
            }
            _ => None,
        });
        assert!(injury.is_some_and(|injury| (1..=230).contains(&injury)));
        assert!(out.contains(&MissionEffect::SkillRaised {
            character: partner,
            skill: Skill::Combat,
            amount: 1,
        }));
        assert!(!out.iter().any(|effect| matches!(
            effect,
            MissionEffect::SkillRaised { character, .. } if *character == member
        )));
        assert!(!out.iter().any(|effect| matches!(
            effect,
            MissionEffect::CharacterInjured { character, .. } if *character == partner
        )));
        assert!(out.contains(&MissionEffect::EncounterEnded { member, partner }));
        crate::missions::apply_advance_effects(&mut world, &out);
        assert_eq!(world.characters[partner].combat.base, 201);
        assert_eq!(world.characters[partner].encounter, None);
    }

    #[test]
    fn luke_s_first_meeting_with_vader_wounds_him_and_teaches_him_force() {
        // FUN_00560200: Luke learns KnowsHeritage (FUN_0056f650); free with
        // Force below GNPRTB 3077 (80) he is wounded by rand(0..=3080 (99))
        // + 3079 (1) (FUN_00561b60); then he gains max(3084 (1), (Vader -
        // Luke) x 3072 (25) / 100). His evade (slot +0x208, FUN_0056f8b0)
        // first adds GNPRTB 3081 (1), so the gap is 200 - 61.
        let (mut world, here) = world_with(imperial());
        table(&mut world, RLEVADTB, 100);
        let luke = force_user(&mut world, here, LUKE_ID, true, 60, 100);
        let vader = force_user(&mut world, here, VADER_ID, false, 200, 100);
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert!(out.contains(&MissionEffect::EncounterEnded {
            member: luke,
            partner: vader,
        }));
        assert!(out.contains(&MissionEffect::HeritageLearned { character: luke }));
        assert_eq!(force_gains(&out, luke), [1, 139 * 25 / 100]);
        let heritage = out
            .iter()
            .position(|effect| matches!(effect, MissionEffect::HeritageLearned { .. }))
            .expect("heritage");
        let wound = out[heritage..].iter().find_map(|effect| match effect {
            MissionEffect::CharacterInjured { character, injury } if *character == luke => {
                Some(*injury)
            }
            _ => None,
        });
        assert!(wound.is_some_and(|injury| (1..=100).contains(&injury)));
    }

    #[test]
    fn luke_who_knows_his_heritage_or_has_force_eighty_is_not_wounded() {
        // The wound sits inside the heritage step and needs Force < 80.
        for (knows, force) in [(true, 60), (false, 80)] {
            let (mut world, here) = world_with(imperial());
            table(&mut world, RLEVADTB, 100);
            let luke = force_user(&mut world, here, LUKE_ID, true, force, 100);
            world.characters[luke].heritage_known = knows;
            let vader = force_user(&mut world, here, VADER_ID, false, 200, 100);
            encountered(&mut world, luke, vader, 1);
            let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

            let out = run_once(&mut mission, &world);

            let ended = out
                .iter()
                .position(|effect| matches!(effect, MissionEffect::EncounterEnded { .. }))
                .expect("a confrontation");
            assert!(
                !out[ended..].iter().any(|effect| matches!(
                    effect,
                    MissionEffect::CharacterInjured { character, .. } if *character == luke
                )),
                "knows {knows}, force {force}"
            );
            assert_eq!(
                out.contains(&MissionEffect::HeritageLearned { character: luke }),
                !knows
            );
        }
    }

    #[test]
    fn luke_s_wound_is_rand_3080_plus_3079_and_needs_force_below_80() {
        // FUN_0055e810: rand(0..=GNPRTB 3080) + 3079, here 1 and 50; Force
        // 80 is not below GNPRTB 3077. No RLEVADTB, so no evade adds Force.
        for roll in [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9] {
            for (force, wounded) in [(60, true), (80, false)] {
                let (mut world, here) = world_with(imperial());
                world.gnprtb = crate::world::GnprtbParams::uniform(&[(3080, 1), (3079, 50)]);
                let luke = force_user(&mut world, here, LUKE_ID, true, force, 100);
                let vader = force_user(&mut world, here, VADER_ID, false, 200, 100);
                encountered(&mut world, luke, vader, 1);
                let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

                let out = run_seeded(&mut mission, &world, roll);

                let ended = out
                    .iter()
                    .position(|effect| matches!(effect, MissionEffect::EncounterEnded { .. }))
                    .expect("a confrontation");
                let wound = out[ended..].iter().find_map(|effect| match effect {
                    MissionEffect::CharacterInjured { character, injury } if *character == luke => {
                        Some(*injury)
                    }
                    _ => None,
                });
                if wounded {
                    assert!(matches!(wound, Some(50 | 51)), "roll {roll}: {wound:?}");
                } else {
                    assert_eq!(wound, None, "roll {roll}");
                }
            }
        }
    }

    #[test]
    fn a_captured_luke_learns_his_heritage_but_no_force() {
        // A failed RLEVADTB roll captures Luke (slot +0x20c), so +0xac bit
        // 0 skips the wound and the lesson.
        let (mut world, here) = world_with(imperial());
        table(&mut world, RLEVADTB, 0);
        let luke = force_user(&mut world, here, LUKE_ID, true, 60, 100);
        let vader = force_user(&mut world, here, VADER_ID, false, 200, 100);
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert!(out.contains(&MissionEffect::CharacterCaptured {
            character: luke,
            captured_by: MissionFaction::Empire,
            at_system: here,
        }));
        assert!(out.contains(&MissionEffect::HeritageLearned { character: luke }));
        assert_eq!(force_gains(&out, luke), [0; 0]);
        assert!(out.contains(&MissionEffect::EncounterEnded {
            member: luke,
            partner: vader,
        }));
    }

    #[test]
    fn luke_learns_a_fifth_of_the_gap_from_the_emperor() {
        // GNPRTB 3085 (1) and 3073 (20): (190 - 91) x 20 / 100 after the
        // evade's 1.
        let (mut world, here) = world_with(imperial());
        table(&mut world, RLEVADTB, 100);
        let luke = force_user(&mut world, here, LUKE_ID, true, 90, 100);
        force_user(&mut world, here, EMPEROR_ID, false, 190, 100);
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(force_gains(&out, luke), [1, 99 * 20 / 100]);
        assert!(!out.contains(&MissionEffect::HeritageLearned { character: luke }));
    }

    #[test]
    fn a_force_aware_leia_learns_from_vader_and_an_unaware_one_does_not() {
        // GNPRTB 3086 (1) and 3074 (20), for Leia free and ForceAware
        // (+0xac bit 0x10); the evade gives 3082 (1) first.
        for aware in [true, false] {
            let (mut world, here) = world_with(imperial());
            table(&mut world, RLEVADTB, 100);
            let leia = force_user(&mut world, here, LEIA_ID, true, 60, 100);
            world.characters[leia].force_aware = aware;
            force_user(&mut world, here, VADER_ID, false, 161, 100);
            let mut mission = sabotage(vec![MissionMember::Character(leia)], here, 2);

            let out = run_once(&mut mission, &world);

            let expected: &[i32] = if aware { &[1, 20] } else { &[1] };
            assert_eq!(force_gains(&out, leia), expected, "aware {aware}");
        }
    }

    #[test]
    fn a_weaker_master_still_teaches_the_least_gain() {
        // max(GNPRTB 3084 (1), gap x 25 / 100) for a gap at or below 0.
        let (mut world, here) = world_with(imperial());
        table(&mut world, RLEVADTB, 100);
        let luke = force_user(&mut world, here, LUKE_ID, true, 150, 100);
        world.characters[luke].heritage_known = true;
        force_user(&mut world, here, VADER_ID, false, 120, 100);
        let mut mission = sabotage(vec![MissionMember::Character(luke)], here, 2);

        let out = run_once(&mut mission, &world);

        assert_eq!(force_gains(&out, luke), [1, 1]);
    }

    #[test]
    fn the_encounter_keys_and_heritage_apply_to_the_world() {
        let (mut world, here) = world_with(imperial());
        let luke = force_user(&mut world, here, LUKE_ID, true, 0, 0);
        let vader = force_user(&mut world, here, VADER_ID, false, 0, 0);

        crate::missions::apply_advance_effects(
            &mut world,
            &[MissionEffect::EncounterBegan {
                member: luke,
                defender: vader,
                day: 7,
            }],
        );
        assert_eq!(world.characters[luke].encounter, Some(DatId::new(VADER_ID)));
        assert_eq!(world.characters[vader].encounter, Some(DatId::new(LUKE_ID)));
        assert_eq!(world.characters[vader].encounter_day, 7);

        crate::missions::apply_advance_effects(
            &mut world,
            &[
                MissionEffect::EncounterEnded {
                    member: luke,
                    partner: vader,
                },
                MissionEffect::HeritageLearned { character: luke },
            ],
        );
        assert_eq!(world.characters[vader].encounter, None);
        assert_eq!(world.characters[luke].encounter_day, 0);
        assert!(world.characters[luke].heritage_known);
    }
}
