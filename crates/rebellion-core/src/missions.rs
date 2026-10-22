//! Mission system: the original's mission kinds (MISSNSD records, family
//! `0x40..0x80`) and their lifecycle (`ghidra/notes/mission-lifecycle.md`).
//!
//! # Design
//!
//! - `MissionState` holds every active mission and the members travelling
//!   to a mission target.
//! - `MissionSystem::advance(state, world, uprisings, &ResearchState::default(), tick_events, rolls)`
//!   returns a `MissionAdvance`: member moves and releases, phase 10
//!   results, and the missions that ended.
//! - The caller provides pre-generated random rolls so rebellion-core stays
//!   dependency-free and fully deterministic in tests.
//!
//! # Lifecycle
//!
//! A mission steps through phases `0..=0xb` (`FUN_005227d0`). It waits in
//! phase 4 for its members' transit and in phase 8 for its MISSNSD timer,
//! rolls in phase 10, repeats 10 -> 8 when its record says so, and ends in
//! phase `0xb` once the validator (`FUN_00522480`) or the last step sets an
//! end code.
//!
//! After each phase change the detection run (`mission_detection`,
//! `FUN_00547f60`) lets the decoys draw off defenders, the defenders try to
//! detect the team, and phase 9 looks for a traitor; a detected team is
//! exposed, and its members evade and resign or are captured.
//!
//! # The phase 10 roll
//!
//! Each team member rolls in turn (slots `+0x274`..`+0x284`,
//! `MissionSystem::roll_members`): its chance is the kind's MSTB table at
//! the kind's input (`member_chance`, `MstbTable::step_lookup`,
//! `FUN_00595090`), or the kind's own rule. A missing table, row or target
//! gives 0.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::ids::{CharacterKey, SpecialForceKey, SystemKey};
use crate::research::{ResearchState, TechType};
use crate::tick::TickEvent;
use crate::world::{Character, GameWorld, Skill};

// ---------------------------------------------------------------------------
// MissionKind
// ---------------------------------------------------------------------------

/// The original's mission kinds, each with its MISSNSD record
/// ([`MissionKind::record_id`]); `Autoscrap` has none. Vacation (`0x72`)
/// has no kind: nothing in the original creates one
/// (`ghidra/notes/mission-lifecycle.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionKind {
    // ── Living Galaxy (implemented) ─────────────────────────────────────────
    /// Shift a system's popularity toward the sending faction.
    /// DIPLMSTB.DAT. Skill: diplomacy.
    Diplomacy,

    /// Recruit an available character for the sending faction.
    /// RCRTMSTB.DAT. Skill: leadership.
    Recruitment,

    // ── War Machine phase ────────────────────────────────────────────────────
    /// Destroy an enemy facility (type code 6 in REBEXE.EXE).
    /// SBTGMSTB.DAT. Skill: espionage.
    Sabotage,

    /// Kill an enemy character (type code 7 in REBEXE.EXE).
    /// ASSNMSTB.DAT. Skill: combat.
    Assassination,

    /// Gather intelligence on an enemy system.
    /// ESPIMSTB.DAT. Skill: espionage.
    Espionage,

    /// Free a captured character.
    /// RESCMSTB.DAT. Skill: combat.
    Rescue,

    /// Capture an enemy character.
    /// ABDCMSTB.DAT. Skill: espionage.
    Abduction,

    /// Trigger a planetary uprising.
    /// INCTMSTB.DAT. Skill: diplomacy.
    InciteUprising,

    /// Automatic scrapping of obsolete units (type code 21 / 0x15).
    /// No character required; always succeeds.
    Autoscrap,

    /// Suppress an active uprising on a system.
    /// SUBDMSTB.DAT. Skill: diplomacy.
    SubdueUprising,

    /// Sabotage the Death Star's construction (delay or destroy).
    /// DSSBMSTB.DAT. Skill: espionage.
    DeathStarSabotage,

    /// Observe a system another side holds (MISSNSD `0x54000015`). It
    /// always succeeds (`FUN_0056bea0`) and has no MSTB.
    Reconnaissance,

    /// Research the next ship level at an own system with a shipyard
    /// (MISSNSD `0x53000020`, type 0; `ghidra/notes/mission-lifecycle.md`,
    /// "Research"). Skill: ship design.
    ShipDesignResearch,

    /// Research the next facility level at an own system with a
    /// construction yard (MISSNSD `0x53000021`, type 1). Skill: facility
    /// design.
    FacilityDesignResearch,

    /// Research the next troop level at an own system with a training
    /// facility (MISSNSD `0x53000022`, type 2). Skill: troop training.
    TroopTrainingResearch,

    /// The Move pseudo-mission (MISSNSD `0x41000001`, TEXTSTRA 11265): a
    /// character's or special force's own trip to a system of its side,
    /// which a Move order `0x201` builds as order `0x241` (`move-order.md`).
    /// Its record is hidden, times 0 + rand(0..=0) days, runs detection
    /// without resigning, and takes only own-side targets; the class uses
    /// the base phase handler `FUN_00524b70` (slot `+0x1f4`), so phase 10
    /// rolls no one, and the base rules `FUN_00523450` (slot `+0x1bc`,
    /// `FUN_005215e0`).
    Move,

    /// The Return pseudo-mission (MISSNSD `0x42000002`, TEXTSTRA 11266,
    /// vtable `0x00664388`): the trip home of members leaving an ended
    /// mission (`FUN_00548910` -> `FUN_005475f0`). Its record and slots are
    /// Move's: hidden, timer 0, own-side targets only, the base phase
    /// handler `FUN_00524b70` and the base rules `FUN_005215e0`.
    Return,

    /// The Adrift pseudo-mission (MISSNSD `0x44000004`, TEXTSTRA 11268,
    /// vtable `0x00668d58`): the trip of characters whose container was
    /// destroyed (`FUN_004ef2e0`, `FUN_00549230` -> `FUN_00547880`). Move's
    /// slots, but its own detection mode word `0x4103` (`FUN_00576a20`):
    /// phase 2 mode 3, phase 7 mode 1, phase 9 mode 4
    /// (`ghidra/notes/decoy-roll.md`).
    Adrift,

    /// Jedi Training (MISSNSD `0x58000042`, TEXTSTRA 11330, vtable
    /// `0x006667d8`; `ghidra/notes/force-awareness.md`, "Force growth"): a
    /// teacher who trains Jedi at Force GNPRTB 2581 (100) or more trains the
    /// other members, each gaining `ForceTraining` `+0x90`.
    JediTraining,

    /// Luke's Dagobah mission (MISSNSD `0x71000043`, TEXTSTRA 11331, class
    /// `DagobahMission` vtable `0x00667f38`; `ghidra/notes/dagobah.md`): the
    /// mission manager creates it for Luke when `LukeDagobah` pulses
    /// (`FUN_0054a860`), with the Dagobah abode `0x98000441` as its target.
    Dagobah,

    /// The Bounty mission (MISSNSD `0x65000083`, TEXTSTRA 11395, class
    /// `0x006685c0`; `ghidra/notes/jabba-palace.md`): the Bounty Hunters
    /// timer `0x393` sends the Bounty Hunters unit from Jabba's Palace after
    /// Han (`FUN_00546fa0`). Its hit (`FUN_00575de0`) makes Han roll RLEVADTB
    /// against them; a captured Han is taken to the palace.
    Bounty,

    /// The Palace mission (MISSNSD `0x64000044`, TEXTSTRA 11332, class
    /// `0x006652e8`; `ghidra/notes/jabba-palace.md`): the mission manager
    /// sends Luke, Leia or Chewbacca to Jabba's Palace for the captive Han
    /// (`FUN_0054a910`, `FUN_0054aa60`, `FUN_0054ab10`). A success frees
    /// Han and every prisoner of the palace (`FUN_0056e650`); a failure
    /// leaves the rescuer captive there for good (`FUN_0056e870`).
    Palace,

    /// The Pickup mission (MISSNSD `0x73000082`, TEXTSTRA 11394, class
    /// `0x00665058`; `ghidra/notes/jabba-palace.md`): `DarthPickup` sends
    /// Vader to Jabba's Palace (`FUN_0054abc0`), and the hit
    /// (`FUN_0056da80`) takes the palace's prisoners into the Empire's
    /// hands.
    Pickup,
}

impl MissionKind {
    /// DAT file stem for this kind's MSTB table, ids `0x14..0x1d` of
    /// `FUN_0058b420` (`ghidra/notes/uprising-incident.md`). `None` for
    /// `Autoscrap`, which has none.
    #[must_use]
    pub fn mstb_key(self) -> Option<&'static str> {
        match self {
            MissionKind::Diplomacy => Some("DIPLMSTB"),
            MissionKind::Recruitment => Some("RCRTMSTB"),
            MissionKind::Sabotage => Some("SBTGMSTB"),
            MissionKind::Assassination => Some("ASSNMSTB"),
            MissionKind::Espionage => Some("ESPIMSTB"),
            MissionKind::Rescue => Some("RESCMSTB"),
            MissionKind::Abduction => Some("ABDCMSTB"),
            MissionKind::InciteUprising => Some("INCTMSTB"),
            MissionKind::SubdueUprising => Some("SUBDMSTB"),
            MissionKind::DeathStarSabotage => Some("DSSBMSTB"),
            MissionKind::Autoscrap
            | MissionKind::Reconnaissance
            | MissionKind::ShipDesignResearch
            | MissionKind::FacilityDesignResearch
            | MissionKind::TroopTrainingResearch
            | MissionKind::Move
            | MissionKind::Return
            | MissionKind::Adrift
            | MissionKind::JediTraining
            | MissionKind::Dagobah
            | MissionKind::Bounty
            | MissionKind::Palace
            | MissionKind::Pickup => None,
        }
    }

    /// The tree a research kind advances (`FUN_0056c460`), `None` for the
    /// other kinds.
    #[must_use]
    pub const fn research_tech(self) -> Option<TechType> {
        match self {
            MissionKind::ShipDesignResearch => Some(TechType::Ship),
            MissionKind::FacilityDesignResearch => Some(TechType::Facility),
            MissionKind::TroopTrainingResearch => Some(TechType::Troop),
            _ => None,
        }
    }

    /// Mission duration range in game-days: (`min_ticks`, `max_ticks`).
    ///
    /// These ranges are not the original's: a mission's timer is its MISSNSD
    /// record's minimum plus `rand(0..=spread)` (`FUN_005236e0`; Diplomacy is
    /// 5 and 10). port: they time only a kind with no loaded record.
    #[must_use]
    pub fn tick_range(self) -> (u32, u32) {
        match self {
            MissionKind::Diplomacy | MissionKind::Recruitment => (15, 20),
            MissionKind::Sabotage
            | MissionKind::Rescue
            | MissionKind::InciteUprising
            | MissionKind::SubdueUprising => (20, 30),
            MissionKind::Assassination | MissionKind::Abduction => (25, 35),
            MissionKind::Espionage => (15, 25),
            MissionKind::DeathStarSabotage => (30, 40),
            MissionKind::Autoscrap => (1, 1),
            // The shipped record's 1 + rand(0..=5).
            MissionKind::Reconnaissance => (1, 6),
            // The shipped records' 10 + rand(0..=10).
            MissionKind::ShipDesignResearch
            | MissionKind::FacilityDesignResearch
            | MissionKind::TroopTrainingResearch => (10, 20),
            // The shipped record's 0 + rand(0..=0).
            MissionKind::Move | MissionKind::Return | MissionKind::Adrift => (0, 0),
            // The shipped record's 60 + rand(0..=30).
            MissionKind::JediTraining => (60, 90),
            // The shipped record's 100 + rand(0..=0).
            MissionKind::Dagobah => (100, 100),
            // MISSNSD 0x65000083: 0 + rand(0..=0) days.
            MissionKind::Bounty => (0, 0),
            // MISSNSD 0x64000044: 5 + rand(0..=10) days.
            MissionKind::Palace => (5, 15),
            // MISSNSD 0x73000082: 0 + rand(0..=0) days.
            MissionKind::Pickup => (0, 0),
        }
    }
}

// ---------------------------------------------------------------------------
// MissionFaction
// ---------------------------------------------------------------------------

/// Which faction is sending the mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionFaction {
    Alliance,
    Empire,
}

impl From<MissionFaction> for crate::dat::Faction {
    fn from(faction: MissionFaction) -> Self {
        match faction {
            MissionFaction::Alliance => Self::Alliance,
            MissionFaction::Empire => Self::Empire,
        }
    }
}

// ---------------------------------------------------------------------------
// ActiveMission
// ---------------------------------------------------------------------------

/// One mission member: a character or a special-forces unit. The original
/// accepts DatId families `0x30..0x3f` (`FUN_00522b30`,
/// `ghidra/notes/decoy-roll.md`, "Adding members").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MissionMember {
    Character(CharacterKey),
    SpecialForce(SpecialForceKey),
}

impl MissionMember {
    /// The member's character key, when it is a character.
    #[must_use]
    pub fn character(self) -> Option<CharacterKey> {
        match self {
            Self::Character(key) => Some(key),
            Self::SpecialForce(_) => None,
        }
    }
}

/// A mission currently in progress.
///
/// Members sit in three lists, as in the original mission object: the team
/// (`+0x84`), the decoys (`+0x8c`), and the captured (`+0x94`)
/// (`ghidra/notes/decoy-roll.md`, "Mission members").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveMission {
    /// Unique identifier within `MissionState` (sequential, never reused).
    pub id: u64,
    /// What kind of mission this is.
    pub kind: MissionKind,
    /// Which faction dispatched this mission.
    pub faction: MissionFaction,
    /// Members who act on the mission and roll for success.
    pub team: Vec<MissionMember>,
    /// Members who draw defenders away and never roll for success.
    pub decoys: Vec<MissionMember>,
    /// Prisoners among the requested members (`FUN_0054bb90`).
    pub captured: Vec<MissionMember>,
    /// The target system.
    pub target_system: SystemKey,
    /// The target character (for assassination, abduction, rescue). None for area missions.
    pub target_character: Option<CharacterKey>,
    /// The object a Sabotage or DS Sabotage mission names (order `+0x4c`,
    /// read back by `FUN_00521030`).
    pub target_object: Option<MissionTarget>,
    /// Phase `+0x68`, `0..=0xb`, advanced by the stepper `FUN_005227d0`
    /// (`ghidra/notes/mission-lifecycle.md`, "Phase stepping").
    pub phase: u8,
    /// Ready bit `+0xa4` bit 0: the next step may run.
    pub ready: bool,
    /// End code `+0x64`; 0 while the mission runs. Any other value sends the
    /// next step to phase `0xb`.
    pub end_code: u8,
    /// The day the order was given; the creation steps run from it.
    pub ordered_tick: u64,
    /// Where the lead member stood when phase 4 started (`+0x6c`); phases 5
    /// and 6 act only when it differs from the target (`FUN_00520bb0`).
    pub origin: Option<SystemKey>,
    /// Travel speed phase 2 gave the character members (`+0x9a`,
    /// `FUN_00548370`).
    pub speed: i64,
    /// The day the current wait (transit or timer) began.
    pub wait_start: u64,
    /// The day timer `0x38b` fires while the mission waits in phase 8.
    pub timer_due: Option<u64>,
    /// Whether the destruction check (`FUN_00545240`) has run for the
    /// destroyed container. The original runs it once, on the destruction.
    pub container_loss_seen: bool,
    /// Members with a resign request (role `+0x78` bit 3, `FUN_00534640`),
    /// set when an exposed member may resign (record column 9).
    pub resigning: Vec<MissionMember>,
    /// Members with a remove request (role `+0x78` bit 2,
    /// `Role_SetMissionRemoveRequest` `FUN_005345d0`): a captured member's
    /// status change sets it (`FUN_004f18e0`, save v90).
    pub removing: Vec<MissionMember>,
    /// Jedi Training's teacher `+0xa8` (`MissionJediTrainingTeacherKeyNotif`),
    /// chosen on entering phase 2 (`FUN_00571410` -> `FUN_00570e40`,
    /// `FUN_005703b0`).
    pub teacher: Option<CharacterKey>,
    /// The abode the mission targets (`+0x78` of family `0x98`), in place of
    /// a system. port: `target_system` then keeps the members' starting
    /// system, which no rule reads.
    pub target_abode: Option<crate::ids::AbodeKey>,
    /// The fleet a Move order targets (`0x241`, `FUN_004f4b60`): its
    /// container, which the members board on landing (save v92); a
    /// traveller follows it when it moves (`reroute`, `FUN_004fbc20`).
    #[serde(default)]
    pub target_fleet: Option<crate::ids::FleetKey>,
    /// Dagobah's `FirstTrainingDay` `+0xa8`, set on entering phase 8
    /// (`FUN_005751d0`).
    pub first_training_day: Option<u64>,
    /// Phase 10's result `+0x60` was 3 (`FUN_00521880`).
    pub succeeded: bool,
    /// The abode the members stood in when phase 2 settled the location
    /// (`+0x6c` of family `0x98`), in place of [`Self::origin`]: the Bounty
    /// Hunters leave Jabba's Palace (save v77).
    pub origin_abode: Option<crate::ids::AbodeKey>,
}

/// The phase that starts every member's transit (`FUN_00520b20`).
pub const PHASE_TRANSIT: u8 = 4;
/// The phase that waits for timer `0x38b` (`FUN_00520b30`).
pub const PHASE_TIMER: u8 = 8;
/// The phase whose entry rolls for success (`FUN_00592f50`).
pub const PHASE_OUTCOME: u8 = 10;
/// The last phase (`FUN_005227d0`).
pub const PHASE_END: u8 = 0xb;

impl ActiveMission {
    /// A new mission in phase 0 with its ready bit set, as creation leaves it
    /// (`FUN_0054bac0` -> `FUN_00522130(this, 1)`).
    #[must_use]
    pub fn new(
        id: u64,
        kind: MissionKind,
        faction: MissionFaction,
        team: Vec<MissionMember>,
        target_system: SystemKey,
        ordered_tick: u64,
    ) -> Self {
        ActiveMission {
            id,
            kind,
            faction,
            team,
            decoys: Vec::new(),
            captured: Vec::new(),
            target_system,
            target_character: None,
            target_object: None,
            phase: 0,
            ready: true,
            end_code: 0,
            ordered_tick,
            origin: None,
            speed: 100,
            wait_start: ordered_tick,
            timer_due: None,
            container_loss_seen: false,
            resigning: Vec::new(),
            removing: Vec::new(),
            teacher: None,
            target_abode: None,
            target_fleet: None,
            origin_abode: None,
            first_training_day: None,
            succeeded: false,
        }
    }

    /// A mission whose members have arrived and whose timer fires on
    /// `due_tick` (phase 8, waiting).
    #[must_use]
    pub fn timed(
        id: u64,
        kind: MissionKind,
        faction: MissionFaction,
        team: Vec<MissionMember>,
        target_system: SystemKey,
        due_tick: u64,
    ) -> Self {
        let mut mission = Self::new(id, kind, faction, team, target_system, 0);
        mission.phase = PHASE_TIMER;
        mission.ready = false;
        mission.origin = Some(target_system);
        mission.timer_due = Some(due_tick);
        mission
    }

    /// Every member in list order: team, decoys, captured (`FUN_00525bb0`).
    pub fn members(&self) -> impl Iterator<Item = MissionMember> + '_ {
        self.team
            .iter()
            .chain(&self.decoys)
            .chain(&self.captured)
            .copied()
    }

    /// The first team character, which the mission lists name.
    /// port: the original keeps a leader key (`MissionLeaderKeyNotif`,
    /// `FUN_005251a0`) whose setter is not traced; the phase-10 roll covers
    /// every member (`MissionSystem::roll_members`).
    #[must_use]
    pub fn lead_character(&self) -> Option<CharacterKey> {
        self.team.iter().find_map(|member| member.character())
    }

    /// Whether the mission waits on its members' transit (phase 4).
    #[must_use]
    pub fn in_transit(&self) -> bool {
        self.phase == PHASE_TRANSIT
    }

    /// Days until the current wait ends: the timer in phase 8, the last
    /// arrival in phase 4 (from `arrivals`), else none.
    #[must_use]
    pub fn days_left(&self, now: u64, last_arrival: Option<u64>) -> Option<u64> {
        let due = match self.phase {
            PHASE_TIMER => self.timer_due,
            PHASE_TRANSIT => last_arrival,
            _ => None,
        }?;
        Some(due.saturating_sub(now))
    }

    /// Progress through the current wait, in [0.0, 1.0].
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "a display fraction over day counts far below 2^52"
    )]
    pub fn wait_fraction(&self, now: u64, last_arrival: Option<u64>) -> f32 {
        let due = match self.phase {
            PHASE_TIMER => self.timer_due,
            PHASE_TRANSIT => last_arrival,
            _ => None,
        };
        match due {
            Some(due) if due > self.wait_start => {
                let done = now
                    .saturating_sub(self.wait_start)
                    .min(due - self.wait_start);
                done as f32 / (due - self.wait_start) as f32
            }
            _ => 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Target objects
// ---------------------------------------------------------------------------

/// The object a Sabotage or DS Sabotage mission names. The object rules
/// take families `0x10..0x40` that are completed (`FUN_00593500`); Sabotage
/// refuses the Death Star (families `0x18..0x1c`) and characters
/// (`0x30..0x3c`, `FUN_0056a110`); DS Sabotage needs the Death Star
/// (`FUN_005744c0`).
///
/// port: the port keeps no identity for one capital ship or one fighter
/// squadron, so a ship or squadron target names its class aboard a fleet, as
/// the Death Star target names its fleet; any hull or squadron of that class
/// there stands for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionTarget {
    DefenseFacility(crate::ids::DefenseFacilityKey),
    ManufacturingFacility(crate::ids::ManufacturingFacilityKey),
    ProductionFacility(crate::ids::ProductionFacilityKey),
    Troop(crate::ids::TroopKey),
    SpecialForce(crate::ids::SpecialForceKey),
    /// The Death Star hull (class `0x88`, family `0x18`) in this fleet.
    DeathStar(crate::ids::FleetKey),
    /// A capital ship of `class` other than the Death Star aboard `fleet`
    /// (families `0x14..0x18`).
    CapitalShip {
        fleet: crate::ids::FleetKey,
        class: crate::ids::CapitalShipKey,
    },
    /// A fighter squadron of `class` aboard `fleet` (families `0x1c..0x20`).
    Fighter {
        fleet: crate::ids::FleetKey,
        class: crate::ids::FighterKey,
    },
}

/// What the validator reads about a target object.
struct ObjectState {
    side: crate::dat::Faction,
    location: Option<SystemKey>,
}

impl MissionTarget {
    /// The side holding the object and where it stands, or `None` once it
    /// is gone.
    fn state(self, world: &GameWorld) -> Option<ObjectState> {
        let faction_of = crate::mission_detection::faction_of;
        let at = |listed: &dyn Fn(&crate::world::System) -> bool| {
            world
                .systems
                .iter()
                .find(|(_, sys)| listed(sys))
                .map(|(key, _)| key)
        };
        let (side, location) = match self {
            MissionTarget::DefenseFacility(key) => (
                world.defense_facilities.get(key)?.side,
                at(&|sys| sys.defense_facilities.contains(&key)),
            ),
            MissionTarget::ManufacturingFacility(key) => (
                world.manufacturing_facilities.get(key)?.side,
                at(&|sys| sys.manufacturing_facilities.contains(&key)),
            ),
            MissionTarget::ProductionFacility(key) => (
                world.production_facilities.get(key)?.side,
                at(&|sys| sys.production_facilities.contains(&key)),
            ),
            MissionTarget::Troop(key) => (
                faction_of(world.troops.get(key)?.is_alliance),
                at(&|sys| sys.ground_units.contains(&key)),
            ),
            MissionTarget::SpecialForce(key) => (
                faction_of(world.special_forces.get(key)?.is_alliance),
                at(&|sys| sys.special_forces.contains(&key)),
            ),
            MissionTarget::DeathStar(key) => {
                let fleet = world.fleets.get(key)?;
                fleet.capital_ships.iter().find(|ship| {
                    ship.alive
                        && world
                            .capital_ship_classes
                            .get(ship.class)
                            .is_some_and(crate::world::CapitalShipClass::is_death_star)
                })?;
                (faction_of(fleet.is_alliance), Some(fleet.location))
            }
            MissionTarget::CapitalShip { fleet, class } => {
                let fleet = world.fleets.get(fleet)?;
                fleet
                    .capital_ships
                    .iter()
                    .find(|ship| ship.alive && ship.class == class)?;
                (faction_of(fleet.is_alliance), Some(fleet.location))
            }
            MissionTarget::Fighter { fleet, class } => {
                let fleet = world.fleets.get(fleet)?;
                fleet
                    .fighters
                    .iter()
                    .find(|entry| entry.class == class && entry.count > 0)?;
                (faction_of(fleet.is_alliance), Some(fleet.location))
            }
        };
        Some(ObjectState {
            side,
            location,
        })
    }
}

/// Destroy a sabotaged target (slot `+0xac(6)`, `FUN_005746e0`).
///
/// A capital ship or the Death Star leaves its fleet as any lost ship does
/// ([`crate::scrap::remove_ship`]): the fleet's ship change
/// (`FUN_004fe870` -> `FUN_004fe630`) disbands a fleet left without one. A
/// squadron's fleet follows the port's same rule
/// ([`crate::scrap::disband_if_empty`]).
///
/// port: a regiment aboard a fleet is not at its container, so the
/// validator ends the mission before it can be named here.
pub fn destroy_target(world: &mut GameWorld, target: MissionTarget) {
    match target {
        MissionTarget::DefenseFacility(key) => {
            for (_, sys) in &mut world.systems {
                sys.defense_facilities.retain(|&k| k != key);
            }
            world.defense_facilities.remove(key);
        }
        MissionTarget::ManufacturingFacility(key) => {
            for (_, sys) in &mut world.systems {
                sys.manufacturing_facilities.retain(|&k| k != key);
            }
            world.manufacturing_facilities.remove(key);
        }
        MissionTarget::ProductionFacility(key) => {
            for (_, sys) in &mut world.systems {
                sys.production_facilities.retain(|&k| k != key);
            }
            world.production_facilities.remove(key);
        }
        MissionTarget::Troop(key) => {
            for (_, sys) in &mut world.systems {
                sys.ground_units.retain(|&k| k != key);
            }
            world.troops.remove(key);
        }
        MissionTarget::SpecialForce(key) => destroy_special_force(world, key),
        MissionTarget::DeathStar(key) => {
            let is_death_star = |world: &GameWorld, ship: &crate::world::ShipInstance| {
                world
                    .capital_ship_classes
                    .get(ship.class)
                    .is_some_and(crate::world::CapitalShipClass::is_death_star)
            };
            let Some(index) = world.fleets.get(key).and_then(|fleet| {
                fleet
                    .capital_ships
                    .iter()
                    .position(|ship| ship.alive && is_death_star(world, ship))
            }) else {
                return;
            };
            crate::scrap::remove_ship(world, key, index);
            let still_one = world.fleets.get(key).map(|fleet| {
                fleet
                    .capital_ships
                    .iter()
                    .any(|ship| ship.alive && is_death_star(world, ship))
            });
            if let (Some(still_one), Some(fleet)) = (still_one, world.fleets.get_mut(key)) {
                fleet.has_death_star = still_one;
            }
        }
        MissionTarget::CapitalShip { fleet, class } => {
            if let Some(index) = world.fleets.get(fleet).and_then(|value| {
                value
                    .capital_ships
                    .iter()
                    .position(|ship| ship.alive && ship.class == class)
            }) {
                crate::scrap::remove_ship(world, fleet, index);
            }
        }
        MissionTarget::Fighter { fleet: key, class } => {
            if let Some(fleet) = world.fleets.get_mut(key) {
                if let Some(entry) = fleet
                    .fighters
                    .iter_mut()
                    .find(|entry| entry.class == class && entry.count > 0)
                {
                    entry.count -= 1;
                }
                fleet.fighters.retain(|entry| entry.count > 0);
                crate::scrap::disband_if_empty(world, key);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// MissionState
// ---------------------------------------------------------------------------

/// A request for a new mission, the port's mission order (`0x240`..`0x242`,
/// `ghidra/notes/ai-mission-planning.md`): the chosen team and decoys, the
/// mission kind, and its target.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionRequest {
    pub kind: MissionKind,
    pub faction: MissionFaction,
    pub team: Vec<MissionMember>,
    pub decoys: Vec<MissionMember>,
    pub target_system: SystemKey,
    pub target_character: Option<CharacterKey>,
    /// The object a Sabotage or DS Sabotage order names (`+0x4c`).
    pub target_object: Option<MissionTarget>,
    /// The day the order is given.
    pub tick: u64,
}

impl MissionRequest {
    /// A request with one character on the team and no decoys.
    #[must_use]
    pub fn single(
        kind: MissionKind,
        faction: MissionFaction,
        character: CharacterKey,
        target_system: SystemKey,
        target_character: Option<CharacterKey>,
        tick: u64,
    ) -> Self {
        Self {
            kind,
            faction,
            team: vec![MissionMember::Character(character)],
            decoys: Vec::new(),
            target_system,
            target_character,
            target_object: None,
            tick,
        }
    }
}

/// Why [`MissionState::dispatch_guarded`] refused a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionRefusal {
    /// No free member remained on the team (`FUN_0054bb90`, `0x40`/`0x91`).
    EmptyTeam,
    /// A member is missing, on the other side, already on a mission, or
    /// elsewhere than the other members (`FUN_00522b30`).
    MemberUnavailable(MissionMember),
    /// Sabotage names no object or the Death Star (`FUN_0056a110`, `0x40`/
    /// `0x28`), or DS Sabotage names no Death Star (`FUN_005744c0`,
    /// `0x40`/`0x29`).
    TargetUnavailable,
    /// The record does not admit these members: a special force or
    /// character the record does not take, or a side it does not run for
    /// (`FUN_00583320`, `0x40`/`0x01`).
    MembersNotAllowed,
}

/// What the member checks read about one member.
struct MemberState {
    is_alliance: bool,
    /// A character the side has recruited; special forces always are
    /// (`+0x50 & 4`, `FUN_004f9860`).
    recruited: bool,
    busy: bool,
    prisoner: bool,
    /// A prisoner's captor, `Some(true)` for the Alliance.
    captor_is_alliance: Option<bool>,
    location: MemberLocation,
}

/// Where a member is: a fleet, else a system (the original compares each
/// member's location key, slot `+0xc`, in `FUN_00522b30`).
///
/// port: `FUN_00522b30` also compares the en route bit (`+0x50` bit 5) and
/// the arrival tick (`+0x44`); the port has no en route members yet, so
/// members with no location match each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberLocation {
    Fleet(crate::ids::FleetKey),
    System(SystemKey),
    Nowhere,
}

fn member_state(world: &GameWorld, member: MissionMember) -> Option<MemberState> {
    match member {
        MissionMember::Character(key) => {
            // port: the original deletes a killed character; the port keeps
            // it in the arena (`Character::mark_killed`), so it counts as gone.
            let c = world.characters.get(key).filter(|c| !c.is_killed)?;
            let location = match (c.current_fleet, c.current_system) {
                (Some(fleet), _) => MemberLocation::Fleet(fleet),
                (None, Some(system)) => MemberLocation::System(system),
                (None, None) => MemberLocation::Nowhere,
            };
            Some(MemberState {
                is_alliance: c.is_alliance,
                recruited: c.recruited,
                busy: c.on_mission || c.on_mandatory_mission,
                prisoner: c.is_captive,
                captor_is_alliance: c
                    .captured_by
                    .map(|side| side == crate::dat::Faction::Alliance),
                location,
            })
        }
        MissionMember::SpecialForce(key) => {
            let unit = world.special_forces.get(key)?;
            let aboard = world
                .fleets
                .iter()
                .find(|(_, fleet)| fleet.special_forces.contains(&key))
                .map(|(fleet, _)| MemberLocation::Fleet(fleet));
            let location = aboard.unwrap_or_else(|| {
                world
                    .systems
                    .iter()
                    .find(|(_, system)| system.special_forces.contains(&key))
                    .map_or(MemberLocation::Nowhere, |(system, _)| {
                        MemberLocation::System(system)
                    })
            });
            Some(MemberState {
                is_alliance: unit.is_alliance,
                recruited: true,
                busy: unit.on_mission,
                prisoner: false,
                captor_is_alliance: None,
                location,
            })
        }
    }
}

/// The mission bits a character contributes to the member mask
/// (`FUN_004ed260`).
const CHARACTER_MISSION_BIT: u32 = 0x1_0000;

/// Whether `members` may undertake a mission whose record admits `rules`
/// (`FUN_005830a0` builds the mask, `FUN_00583320` checks it). Every bit of
/// the special forces' SPECFCSD masks and of the characters' `0x10000` must
/// be allowed, and the members must all be of one side that the record
/// admits; no side at all fails too (status `0x16`). Skills and rank play no
/// part.
#[must_use]
pub fn members_allowed(
    world: &GameWorld,
    rules: crate::world::MissionMemberRules,
    members: impl IntoIterator<Item = MissionMember>,
) -> bool {
    let (mut special_forces, mut characters) = (0_u32, 0_u32);
    let (mut alliance, mut empire) = (false, false);
    for member in members {
        let is_alliance = match member {
            MissionMember::Character(key) => {
                let Some(c) = world.characters.get(key) else {
                    continue;
                };
                characters |= CHARACTER_MISSION_BIT;
                // A prisoner's side +0x24 is its captor's while Captured
                // (FUN_004eec60, captive-escape.md).
                match (c.is_captive, c.captured_by) {
                    (true, Some(captor)) => captor == crate::dat::Faction::Alliance,
                    _ => c.is_alliance,
                }
            }
            MissionMember::SpecialForce(key) => {
                let Some(unit) = world.special_forces.get(key) else {
                    continue;
                };
                // port: a unit whose class is missing contributes no bits.
                special_forces |= world
                    .special_force_classes
                    .get(&unit.class_dat_id)
                    .map_or(0, |class| class.mission_mask);
                unit.is_alliance
            }
        };
        if is_alliance {
            alliance = true;
        } else {
            empire = true;
        }
    }
    special_forces & rules.special_force_mask == special_forces
        && characters & rules.character_mask == characters
        && alliance != empire
        && (!alliance || rules.alliance)
        && (!empire || rules.empire)
}

/// The class creation checks of `FUN_0056a110` (Sabotage) and
/// `FUN_005744c0` (Death Star Sabotage), which read the target object's
/// family (slot `+4`): Sabotage refuses the Death Star (families
/// `0x18..0x1c`), Death Star Sabotage every other object. Both the dispatch
/// and the dialog's list run them, since `FUN_0054c590` builds the mission
/// through its class factory. port: a missing object is refused, because
/// every original order names one.
fn creation_target_ok(kind: MissionKind, object: Option<MissionTarget>) -> bool {
    let death_star = object.map(|target| matches!(target, MissionTarget::DeathStar(_)));
    match kind {
        MissionKind::Sabotage => death_star == Some(false),
        MissionKind::DeathStarSabotage => death_star == Some(true),
        _ => true,
    }
}

/// The creation member check of research (`FUN_0056c510` with creation flag
/// `+4`): every character member needs a nonzero effective skill in the
/// tree (`FUN_0056c820` -> `FUN_005337f0`), else the order is refused
/// (`0x40`/4). Special forces take the sibling `FUN_0056c990`; the records'
/// special-force mask 0 keeps them off research. Other kinds pass.
/// GNPRTB 2581 (`DAT_006b90dc`) = 100: the Force at which a Jedi trainer
/// may teach (`FUN_004ed310` -> `FUN_0053eb90`).
const GNPRTB_TEACHING_FORCE: u16 = 2581;
/// GNPRTB 2581's shipped value.
const SHIPPED_TEACHING_FORCE: i64 = 100;
/// GNPRTB 6173 (`DAT_006bb528`) = 50: the share of the Force gap a Jedi
/// Training hit may add (`FUN_0055ca20`).
const GNPRTB_JEDI_TRAINING_SHARE: u16 = 6173;
/// GNPRTB 6173's shipped value.
const SHIPPED_JEDI_TRAINING_SHARE: i64 = 50;
/// The end code a failed Jedi Training rule writes (`FUN_00570690`,
/// `FUN_00570940`: `*end = -1`), as a byte.
pub const JEDI_TRAINING_REFUSED: u8 = 0xff;

/// `FUN_004ed310`: the character trains Jedi (class `+0xb8`) at Force
/// GNPRTB 2581 or more.
#[must_use]
pub fn can_teach_jedi(world: &GameWorld, character: &crate::world::Character) -> bool {
    character.is_jedi_trainer
        && i64::from(character.force_level())
            >= crate::movement::gnprtb_or_shipped(
                world,
                GNPRTB_TEACHING_FORCE,
                SHIPPED_TEACHING_FORCE,
            )
}

/// `FUN_00570e40`: the first character member who may teach.
pub(crate) fn jedi_teacher(world: &GameWorld, team: &[MissionMember]) -> Option<CharacterKey> {
    team.iter()
        .filter_map(|member| member.character())
        .find(|&key| {
            world
                .characters
                .get(key)
                .is_some_and(|c| can_teach_jedi(world, c))
        })
}

/// `FUN_005710b0` -> `FUN_0055c7c0`: a character pupil's chance is the
/// teacher's Force less its own, in percent; 0 without a teacher.
pub(crate) fn jedi_training_chance(
    world: &GameWorld,
    teacher: Option<CharacterKey>,
    member: MissionMember,
) -> i32 {
    let force = |key: CharacterKey| {
        world
            .characters
            .get(key)
            .map(|c| i64::from(c.force_level()))
    };
    match (teacher.and_then(force), member.character().and_then(force)) {
        (Some(teacher), Some(pupil)) => i32::try_from(teacher - pupil).unwrap_or(0),
        _ => 0,
    }
}

/// `FUN_00570690` while a Jedi Training runs (`FUN_00570940` over the member
/// lists): a teacher must be among the members (`0x40`/`0x90`), every other
/// character member needs some Force (`0x40`/5) below the teacher's
/// (`0x40`/`0xd`), and there must be two members or more (`0x40`/`0x94`).
///
/// port: the dialog runs these object checks at creation too, in place of
/// the class-record ones (`FUN_00570bc0`).
fn jedi_training_refused(world: &GameWorld, mission: &ActiveMission) -> bool {
    let members: Vec<MissionMember> = mission.members().collect();
    let Some(teacher) = jedi_teacher(world, &members) else {
        return true;
    };
    let force = |key: CharacterKey| {
        world
            .characters
            .get(key)
            .map_or(0, crate::world::Character::force_level)
    };
    let teacher_force = force(teacher);
    let pupils_fit = members
        .iter()
        .filter_map(|member| member.character())
        .all(|key| {
            let pupil = force(key);
            pupil != 0 && (key == teacher || pupil < teacher_force)
        });
    !pupils_fit || members.len() < 2
}

pub(crate) fn creation_member_ok(
    world: &GameWorld,
    kind: MissionKind,
    member: MissionMember,
) -> bool {
    match kind.research_tech() {
        Some(tech) => member_skill(world, member, tech.skill()).is_some_and(|skill| skill != 0),
        None => true,
    }
}

/// What a Mission order targets: the object the release window answered
/// (`+0x68`, `ghidra/notes/move-order.md`, "Hit tests") and its system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissionTargetPick {
    pub system: SystemKey,
    /// A character under the point.
    pub character: Option<CharacterKey>,
    /// Any other object under the point.
    pub object: Option<MissionTarget>,
}

impl From<SystemKey> for MissionTargetPick {
    fn from(system: SystemKey) -> Self {
        Self {
            system,
            character: None,
            object: None,
        }
    }
}

impl MissionTargetPick {
    /// The rules a kind needs to accept this target: the system's, a
    /// character's (`FUN_00586e20`) or an object's (`FUN_00593500`).
    const fn rules(&self) -> TargetRules {
        if self.character.is_some() {
            TargetRules::Character
        } else if self.object.is_some() {
            TargetRules::Object
        } else {
            TargetRules::System
        }
    }
}

/// The missions the player's dialog lists for a team, decoys, and target,
/// in MISSNSD order (`FUN_004f5380` -> `FUN_005422f0`): every record that is
/// not hidden, whose member rules admit the members, and whose class
/// validator passes on a mission built from them and the target
/// (`FUN_0054c590` runs it on a temporary mission, `FUN_00582b90`). A kind
/// lists only for the target its class takes: a system, a character, or
/// another object.
///
/// port: records outside the port's kinds are skipped (Recon, Research, and
/// Jedi Training are a separate finding).
#[must_use]
pub fn available_kinds(
    world: &GameWorld,
    uprisings: &crate::uprising::UprisingState,
    research: &ResearchState,
    faction: MissionFaction,
    team: &[MissionMember],
    decoys: &[MissionMember],
    target: impl Into<MissionTargetPick>,
) -> Vec<MissionKind> {
    let target = target.into();
    world
        .mission_records
        .iter()
        .filter(|record| !record.hidden)
        .filter_map(|record| {
            // port: the AI's kinds, and Jedi Training, which no AI planner
            // sends (its planner is not read).
            crate::mission_planning::KINDS
                .into_iter()
                .chain([MissionKind::JediTraining])
                .find(|kind| kind.record_id() == Some(record.dat_id))
                .map(|kind| (kind, record))
        })
        .filter(|(kind, _)| kind.target_rules() == target.rules())
        .filter(|(kind, _)| creation_target_ok(*kind, target.object))
        .filter(|(kind, _)| {
            team.iter()
                .chain(decoys)
                .all(|&member| creation_member_ok(world, *kind, member))
        })
        .filter(|(_, record)| {
            members_allowed(world, record.members, team.iter().chain(decoys).copied())
        })
        .filter(|(kind, _)| {
            let mut mission =
                ActiveMission::new(0, *kind, faction, team.to_vec(), target.system, 0);
            mission.decoys = decoys.to_vec();
            mission.target_character = target.character;
            mission.target_object = target.object;
            running_end_code(&mission, world, uprisings, research, &[]) == 0
        })
        .map(|(kind, _)| kind)
        .collect()
}

/// Whether a present member stands in a fleet (`Some(true)`) or at a system
/// (`Some(false)`); `None` without a location.
pub(crate) fn member_location_kind(world: &GameWorld, member: MissionMember) -> Option<bool> {
    match member_state(world, member)?.location {
        MemberLocation::Fleet(_) => Some(true),
        MemberLocation::System(_) => Some(false),
        MemberLocation::Nowhere => None,
    }
}

/// A member's skill: a character's rolled template (base plus half of any
/// unrolled variance, as `MissionKind::skill_score` reads it), or a special
/// force's rolled skill. `None` when the member no longer exists.
#[must_use]
pub fn member_skill(world: &GameWorld, member: MissionMember, skill: Skill) -> Option<u32> {
    match member {
        // The missions read a character's skills through its slots, the
        // effective values: diplomacy +0x7c (slot +0x1dc, FUN_00573ff0),
        // espionage +0x7e (+0x1e0, FUN_00573090) and combat +0x86 (+0x1f0)
        // carry the Force share, combat also the injury.
        // port: seeding rolls every character's variance into its base
        // (FUN_00535e40); half of an unrolled variance matches skill_score.
        // Leadership (+0x88, slot +0x1f4) carries the seat of power's
        // +0x92 (FUN_004eede0).
        MissionMember::Character(key) => world.characters.get(key).map(|c| match skill {
            Skill::Leadership => crate::seat_of_power::effective_leadership(world, c),
            _ => c.effective_skill(skill),
        }),
        MissionMember::SpecialForce(key) => world
            .special_forces
            .get(key)
            .map(|unit| unit.skills[skill as usize]),
    }
}

/// Slot `+0x274`: `member`'s success chance in percent, the MSTB row
/// (`FUN_0053e240` -> `FUN_0055bed0`, a step lookup `FUN_00595090`) for the
/// class's input (`ghidra/notes/decoy-roll.md`, "The success roll"):
///
/// - Diplomacy (`FUN_00573ff0`, `FUN_0055c680`): `(stormtroopers - opposing
///   support) + diplomacy`.
/// - Espionage (`FUN_00573090`): espionage. Rescue (`FUN_0056ae30`): combat.
/// - Sabotage and DS Sabotage (`FUN_0056a2d0`, `FUN_00574600`):
///   `(espionage + combat) / 2`.
/// - Recruitment (`FUN_0056b7b0`): `leadership - opposing support`.
/// - Incite (`FUN_005719d0`): `(leadership - opposing support) -
///   stormtroopers`; Subdue (`FUN_00569b90`): `(stormtroopers - opposing
///   support) + leadership`.
/// - Abduction and Assassination (`FUN_00576e50`, `FUN_005765c0`): `combat -
///   target combat`.
///
/// The opposing support is `FUN_00507270` for side `2 - (side != 1)`, and
/// the stormtroopers are `FUN_005091f0`. A missing table, row, target system
/// (`FUN_00586720`), or target character (`FUN_00586c80`) gives 0.
#[must_use]
pub fn member_chance(
    world: &GameWorld,
    kind: MissionKind,
    faction: MissionFaction,
    target_system: SystemKey,
    target_character: Option<CharacterKey>,
    member: MissionMember,
) -> i32 {
    let skill = |skill| {
        member_skill(world, member, skill).map_or(0, |value| i32::try_from(value).unwrap_or(0))
    };
    let side = crate::dat::Faction::from(faction);
    let opponent = if side == crate::dat::Faction::Alliance {
        crate::dat::Faction::Empire
    } else {
        crate::dat::Faction::Alliance
    };
    let system = world.systems.get(target_system);
    let support = |sys| crate::uprising::support_points(sys, opponent);
    let stormtroopers = |sys| crate::uprising::stormtroopers(world, sys);
    let target_combat = || {
        target_character
            .filter(|key| world.characters.contains_key(*key))
            .map(|key| skill_of(world, key, Skill::Combat))
    };
    let input = match kind {
        MissionKind::Diplomacy => {
            system.map(|sys| stormtroopers(sys) - support(sys) + skill(Skill::Diplomacy))
        }
        MissionKind::Espionage => Some(skill(Skill::Espionage)),
        MissionKind::Rescue => Some(skill(Skill::Combat)),
        MissionKind::Sabotage | MissionKind::DeathStarSabotage => {
            Some((skill(Skill::Espionage) + skill(Skill::Combat)) / 2)
        }
        MissionKind::Recruitment => system.map(|sys| skill(Skill::Leadership) - support(sys)),
        MissionKind::InciteUprising => {
            system.map(|sys| skill(Skill::Leadership) - support(sys) - stormtroopers(sys))
        }
        MissionKind::SubdueUprising => {
            system.map(|sys| stormtroopers(sys) - support(sys) + skill(Skill::Leadership))
        }
        MissionKind::Abduction | MissionKind::Assassination => {
            target_combat().map(|target| skill(Skill::Combat) - target)
        }
        // Autoscrap is no MSTB mission; it always succeeds.
        MissionKind::Autoscrap => return 100,
        // FUN_0056bea0: Reconnaissance always succeeds.
        MissionKind::Reconnaissance => return 100,
        // FUN_0056cb20: 0. Research rolls inside its member slot instead
        // ([`research_chance`]).
        MissionKind::ShipDesignResearch
        | MissionKind::FacilityDesignResearch
        | MissionKind::TroopTrainingResearch => return 0,
        // The base class has no slot +0x274 roll; phase 10 rolls no one.
        MissionKind::Move | MissionKind::Return | MissionKind::Adrift => return 0,
        // FUN_005710b0 reads the mission's teacher +0xa8: phase 10 takes
        // the chance from [`jedi_training_chance`].
        MissionKind::JediTraining => return 0,
        // FUN_00575060: DAT_00661a88, 100 (Dagobah's, Bounty's and
        // Pickup's slot +0x274).
        MissionKind::Dagobah | MissionKind::Bounty | MissionKind::Pickup => return 100,
        // FUN_0056e620 -> FUN_0055c910: espionage (slot +0x1e0) / GNPRTB
        // 6154 + combat (slot +0x1f0) / GNPRTB 6155, in integers.
        MissionKind::Palace => {
            let share = |id, value: i32| {
                let divisor = i32::try_from(crate::movement::gnprtb_or_shipped(
                    world,
                    id,
                    SHIPPED_PALACE_DIVISOR,
                ))
                .unwrap_or(0);
                value.checked_div(divisor).unwrap_or(0)
            };
            return share(GNPRTB_PALACE_ESPIONAGE_DIVISOR, skill(Skill::Espionage))
                + share(GNPRTB_PALACE_COMBAT_DIVISOR, skill(Skill::Combat));
        }
    };
    let (Some(input), Some(table)) = (
        input,
        kind.mstb_key()
            .and_then(|key| world.mission_tables.get(key)),
    ) else {
        return 0;
    };
    i32::try_from(table.step_lookup(input)).unwrap_or(i32::MAX)
}

/// A research member's chance in percent (`FUN_0056cdb0`): its effective
/// skill in the tree (`FUN_005337f0`), through the identity
/// `FUN_0055c6f0`.
#[must_use]
pub fn research_chance(world: &GameWorld, member: MissionMember, tech: TechType) -> i32 {
    member_skill(world, member, tech.skill()).map_or(0, |value| i32::try_from(value).unwrap_or(0))
}

/// A character's skill as [`member_skill`] reads it.
fn skill_of(world: &GameWorld, character: CharacterKey, skill: Skill) -> i32 {
    member_skill(world, MissionMember::Character(character), skill)
        .map_or(0, |value| i32::try_from(value).unwrap_or(0))
}

/// Set or clear a member's on-mission role flag. Clearing it also clears a
/// character's hidden-mission flag.
///
/// The original's leave path `FUN_00536220` recomputes OnMission (bit 7,
/// `FUN_00534950` → `FUN_00534800`) and clears bits 0 and 2..5; where it
/// clears OnHiddenMission (bit 8) is untraced. port: bit 8 clears with bit 7.
pub fn set_on_mission(world: &mut GameWorld, member: MissionMember, on_mission: bool) {
    match member {
        MissionMember::Character(key) => {
            if let Some(c) = world.characters.get_mut(key) {
                c.on_mission = on_mission;
                if !on_mission {
                    c.on_hidden_mission = false;
                }
            }
        }
        MissionMember::SpecialForce(key) => {
            if let Some(unit) = world.special_forces.get_mut(key) {
                unit.on_mission = on_mission;
            }
        }
    }
}

/// Apply a [`MissionEffect::MemberMoved`]: take the member out of its fleet
/// or system, then put it at `to`, or leave it off the map while it travels.
/// A killed character stays where `mark_killed` left it.
pub fn move_member(world: &mut GameWorld, member: MissionMember, to: Option<SystemKey>) {
    match member {
        MissionMember::Character(key) => {
            if world.characters.get(key).is_none_or(|c| c.is_killed) {
                return;
            }
            for (_, fleet) in &mut world.fleets {
                fleet.characters.retain(|&c| c != key);
            }
            let character = &mut world.characters[key];
            character.move_to(None, to);
            character.current_abode = None;
        }
        MissionMember::SpecialForce(key) => {
            if !world.special_forces.contains_key(key) {
                return;
            }
            take_special_force(world, key);
            if let Some(system) = to.and_then(|to| world.systems.get_mut(to)) {
                system.special_forces.push(key);
            }
        }
    }
}

/// A Move's traveller whose target fleet stands elsewhere when it arrives
/// sets out again for the fleet's system, at the mission's speed: the
/// original reroutes a traveller whose target container moves
/// (`FUN_004fbc20`, `GameObjParentReroutedNotif`). port: the port reroutes
/// on arrival, not when the fleet moves.
fn reroute(
    world: &GameWorld,
    mission: &ActiveMission,
    transit: &MemberTransit,
    now: u64,
) -> Option<MemberTransit> {
    let fleet = world.fleets.get(mission.target_fleet?)?;
    let own = crate::mission_detection::faction_of(fleet.is_alliance)
        == crate::dat::Faction::from(mission.faction);
    if !own || fleet.location == transit.to {
        return None;
    }
    let position = |key: SystemKey| world.systems.get(key).map(|s| (s.x, s.y));
    let days = crate::movement::transit_ticks_between(
        world,
        position(transit.to)?,
        position(fleet.location)?,
        mission.speed,
    );
    Some(MemberTransit {
        member: transit.member,
        mission_id: transit.mission_id,
        to: fleet.location,
        arrival: now + u64::from(days.max(1)),
        abode: None,
    })
}

/// The effects of `member` landing at `to` for `mission` (`FUN_004f7640`
/// into the container `+0x74`): into its abode, aboard the fleet a Move
/// targets when that fleet of the mission's side stands there, else at the
/// system.
fn landing(
    world: &GameWorld,
    mission: Option<&ActiveMission>,
    member: MissionMember,
    to: SystemKey,
    abode: Option<crate::ids::AbodeKey>,
) -> Vec<MissionEffect> {
    if let Some(abode) = abode {
        return vec![MissionEffect::MemberEnteredAbode { member, abode }];
    }
    let moved = MissionEffect::MemberMoved {
        member,
        to: Some(to),
    };
    let aboard = mission.and_then(|mission| {
        let fleet = mission.target_fleet?;
        let value = world.fleets.get(fleet)?;
        (value.location == to
            && crate::mission_detection::faction_of(value.is_alliance)
                == crate::dat::Faction::from(mission.faction))
        .then_some(fleet)
    });
    match aboard {
        Some(fleet) => vec![moved, MissionEffect::MemberBoarded { member, fleet }],
        None => vec![moved],
    }
}

/// A battle destroyed `fleet`: its characters stand at its system, and
/// each on no mission goes adrift (`FUN_00556710` -> slot `+0xb4`
/// `FUN_004ef2e0`, `Role_SetAdrift` `FUN_00534790`) for
/// `MissionSystem::send_adrift` to settle or send away. Call it before the
/// fleet is removed.
pub fn cast_adrift(world: &mut GameWorld, fleet: crate::ids::FleetKey) {
    let Some((characters, units, location)) = world.fleets.get_mut(fleet).map(|value| {
        (
            value.characters.clone(),
            std::mem::take(&mut value.special_forces),
            value.location,
        )
    }) else {
        return;
    };
    // A special force's slot +0xb4 (FUN_00503e20) on a destroyed container
    // sets it Autorouting (FUN_004f78e0(1)). port: it stands at the fleet's
    // system; the autoroute away from another side's place
    // (FUN_0054c7d0) is not ported for special forces.
    if let Some(system) = world.systems.get_mut(location) {
        system.special_forces.extend(units);
    }
    for key in characters {
        if let Some(c) = world.characters.get_mut(key) {
            c.move_to(None, Some(location));
            if !c.is_killed && !c.on_mission {
                c.adrift = true;
            }
        }
    }
}

/// Take a special force out of every system, abode and fleet that lists it.
fn take_special_force(world: &mut GameWorld, key: crate::ids::SpecialForceKey) {
    for (_, system) in &mut world.systems {
        system.special_forces.retain(|&unit| unit != key);
    }
    for (_, abode) in &mut world.abodes {
        abode.special_forces.retain(|&unit| unit != key);
    }
    for (_, fleet) in &mut world.fleets {
        fleet.special_forces.retain(|&unit| unit != key);
    }
}

/// Apply [`MissionEffect::MemberBoarded`]: the character or special force
/// joins `fleet` at the fleet's system (`FUN_004f7640` into the container
/// `+0x74`).
pub fn board_fleet(world: &mut GameWorld, member: MissionMember, fleet: crate::ids::FleetKey) {
    let key = match member {
        MissionMember::Character(key) => key,
        MissionMember::SpecialForce(unit) => {
            if world.special_forces.contains_key(unit) && world.fleets.contains_key(fleet) {
                take_special_force(world, unit);
                world.fleets[fleet].special_forces.push(unit);
            }
            return;
        }
    };
    let Some(location) = world.fleets.get(fleet).map(|value| value.location) else {
        return;
    };
    if world.characters.get(key).is_none_or(|c| c.is_killed) {
        return;
    }
    for (_, other) in &mut world.fleets {
        other.characters.retain(|&c| c != key);
    }
    let character = &mut world.characters[key];
    character.move_to(Some(fleet), Some(location));
    character.current_abode = None;
    world.fleets[fleet].characters.push(key);
}

/// Apply [`MissionAdvance::effects`] in order: member moves and releases,
/// and the detection's captures, lost special forces, injuries and Force
/// gains. The native and browser app shares this with the headless
/// integrator.
pub fn apply_advance_effects(world: &mut GameWorld, effects: &[MissionEffect]) {
    for effect in effects {
        match effect {
            MissionEffect::MemberMoved { member, to } => move_member(world, *member, *to),
            MissionEffect::MemberAvailable { member } => set_on_mission(world, *member, false),
            MissionEffect::MemberBoarded { member, fleet } => board_fleet(world, *member, *fleet),
            MissionEffect::AdriftSettled { character } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.adrift = false;
                }
            }
            MissionEffect::MemberOnMission { member, hidden } => {
                set_on_mission(world, *member, true);
                if let Some(c) = member
                    .character()
                    .and_then(|key| world.characters.get_mut(key))
                {
                    c.on_hidden_mission = *hidden;
                }
            }
            MissionEffect::CharacterCaptured {
                character,
                captured_by,
                at_system,
            } => capture_character(world, *character, *captured_by, *at_system),
            MissionEffect::SpecialForceDestroyed { unit } => destroy_special_force(world, *unit),
            MissionEffect::CharacterInjured { character, injury } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.injure(*injury);
                }
            }
            MissionEffect::ForceExperienceGained { character, amount } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.add_force_experience(*amount);
                }
            }
            MissionEffect::MemberEnteredAbode { member, abode } => {
                enter_abode(world, *member, *abode);
            }
            MissionEffect::DagobahCompleted {
                character,
                force_gain,
            } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.add_force_experience(*force_gain);
                    c.dagobah_completed = true;
                }
            }
            MissionEffect::SkillRaised {
                character,
                skill,
                amount,
            } => raise_skill(world, *character, *skill, *amount),
            MissionEffect::SystemIntelligenceGathered { system, .. } => {
                if let Some(sys) = world.systems.get_mut(*system) {
                    sys.exploration_status = crate::dat::ExplorationStatus::Explored;
                }
            }
            MissionEffect::EncounterBegan {
                member,
                defender,
                day,
            } => begin_encounter(world, *member, *defender, *day),
            MissionEffect::EncounterEnded { member, partner } => {
                for key in [member, partner] {
                    if let Some(c) = world.characters.get_mut(*key) {
                        c.encounter = None;
                        c.encounter_day = 0;
                    }
                }
            }
            MissionEffect::HeritageLearned { character } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.heritage_known = true;
                }
            }
            MissionEffect::CapturedByBountyHunters { character } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.escape_barred = true;
                    c.bounty_captured = true;
                }
            }
            MissionEffect::EscapeBarred { character } => {
                if let Some(c) = world.characters.get_mut(*character) {
                    c.escape_barred = true;
                }
            }
            MissionEffect::DiedOfInjury { character } => kill_character(world, *character),
            _ => {}
        }
    }
}

/// `FUN_0055ff60`'s keys: `member` and `defender` name each other in
/// `Encounter` `+0x9c` and record `day` ([`MissionEffect::EncounterBegan`]).
pub fn begin_encounter(
    world: &mut GameWorld,
    member: CharacterKey,
    defender: CharacterKey,
    day: u64,
) {
    let ids = (
        world.characters.get(member).map(|c| c.dat_id),
        world.characters.get(defender).map(|c| c.dat_id),
    );
    let (Some(member_id), Some(defender_id)) = ids else {
        return;
    };
    for (key, partner) in [(member, defender_id), (defender, member_id)] {
        let c = &mut world.characters[key];
        c.encounter = Some(partner);
        c.encounter_day = day;
    }
}

/// Put a character member into `abode`, out of every system and fleet
/// (`FUN_00556390`). port: a special force never goes to an abode.
pub fn enter_abode(world: &mut GameWorld, member: MissionMember, abode: crate::ids::AbodeKey) {
    let MissionMember::Character(key) = member else {
        return;
    };
    if world.characters.get(key).is_none_or(|c| c.is_killed) || !world.abodes.contains_key(abode) {
        return;
    }
    for (_, fleet) in &mut world.fleets {
        fleet.characters.retain(|&c| c != key);
    }
    let character = &mut world.characters[key];
    character.move_to(None, None);
    character.current_abode = Some(abode);
}

/// Raise `character`'s base `skill` by `amount` ([`MissionEffect::SkillRaised`]).
/// The setters (`FUN_00533e20`..`FUN_005341a0`) take the new short as given.
pub fn raise_skill(world: &mut GameWorld, character: CharacterKey, skill: Skill, amount: i32) {
    if let Some(c) = world.characters.get_mut(character) {
        let pair = c.skill_mut(skill);
        pair.base = pair.base.saturating_add_signed(amount);
    }
}

/// `FUN_0055ef30`'s pool: `side`'s living minor characters (families
/// `0x38..0x3c`, MNCHARSD, `FUN_0056f450`) it has not recruited (`+0x50` bit
/// 1 clear). port: the original walks its character container
/// (`FUN_00506e20`); the port walks them in `DatId` order.
pub(crate) fn recruit_pool(world: &GameWorld, side: crate::dat::Faction) -> Vec<CharacterKey> {
    let mut pool: Vec<(crate::ids::DatId, CharacterKey)> = world
        .characters
        .iter()
        .filter(|(_, c)| {
            !c.is_major && !c.recruited && !c.is_killed && character_side(c) == Some(side)
        })
        .map(|(key, c)| (c.dat_id, key))
        .collect();
    pool.sort_by_key(|&(dat_id, _)| dat_id.raw());
    pool.into_iter().map(|(_, key)| key).collect()
}

/// Apply [`MissionEffect::CharacterRecruited`]: `FUN_0055fe70` places the
/// recruit at `system` (slot `+0xa8`) and sets `+0x50` bits 1 and 2
/// (`FUN_004f7480`, `FUN_004f74f0`); taking the last one sets the side's
/// `+0xb8` (`FUN_0052f590`). port: bit 2 is not modelled.
pub fn recruit_character(
    world: &mut GameWorld,
    character: CharacterKey,
    system: SystemKey,
    faction: MissionFaction,
    pool_emptied: bool,
) {
    if let Some(c) = world.characters.get_mut(character) {
        c.recruited = true;
        c.current_system = Some(system);
        c.current_fleet = None;
    }
    if pool_emptied {
        world.set_recruit_pool_empty(faction.into());
    }
}

/// Hold `character` at `at_system` as `captured_by`'s prisoner.
pub fn capture_character(
    world: &mut GameWorld,
    character: CharacterKey,
    captured_by: MissionFaction,
    at_system: SystemKey,
) {
    if let Some(c) = world.characters.get_mut(character) {
        c.is_captive = true;
        c.captured_by = Some(captured_by.into());
        // A character taken in an abode stays there (FUN_0056e870).
        if c.current_abode.is_none() {
            c.current_system = Some(at_system);
        }
        c.current_fleet = None;
        // FUN_004f1a80: a capture sets CanEscape.
        c.escape_barred = false;
    }
    for (_, fleet) in &mut world.fleets {
        fleet.characters.retain(|&k| k != character);
    }
}

/// A character destroyed: off every fleet roster, out of its abode, and
/// marked killed (kept in the arena for lookups, as the integrator's
/// assassination arm does).
pub fn kill_character(world: &mut GameWorld, character: CharacterKey) {
    for (_, fleet) in &mut world.fleets {
        fleet.characters.retain(|&k| k != character);
    }
    if let Some(c) = world.characters.get_mut(character) {
        c.mark_killed();
        c.current_abode = None;
    }
}

/// Remove a special force from its system or fleet and the world.
pub fn destroy_special_force(world: &mut GameWorld, unit: crate::ids::SpecialForceKey) {
    for (_, system) in &mut world.systems {
        system.special_forces.retain(|&key| key != unit);
    }
    for (_, fleet) in &mut world.fleets {
        fleet.special_forces.retain(|&key| key != unit);
    }
    world.special_forces.remove(unit);
}

/// One mission member travelling to its mission's target, as the original's
/// en route member (`+0x50` bit 5, arrival tick `+0x44`; `FUN_00556430`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberTransit {
    pub member: MissionMember,
    /// The mission that sent it.
    pub mission_id: u64,
    /// Its destination, the mission's target container `+0x74`.
    pub to: SystemKey,
    /// The day it arrives.
    pub arrival: u64,
    /// The abode it travels to, in place of `to` (`FUN_00555b30`: 1 day).
    pub abode: Option<crate::ids::AbodeKey>,
}

/// A captive's escape timer `0x38a` (`ghidra/notes/captive-escape.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscapeTimer {
    pub character: CharacterKey,
    /// The day of its next attempt.
    pub due: u64,
}

/// An escaped character autorouting to safety (`FUN_004f18e0`).
///
/// port: the original sends an autorouting group on an Autorouting order
/// `0x43000003` (`FUN_00549230` -> `FUN_00547740`); the port keeps a direct
/// trip. The order adds a mandatory mission flag, which no order meets
/// while the traveller is off the map, and its traveller lands even when
/// the order ends on the way (`FUN_00522fc0`), as this trip does
/// (`ghidra/notes/mission-lifecycle.md`, "The Autorouting order").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscapeeTransit {
    pub character: CharacterKey,
    pub to: SystemKey,
    /// The day it arrives.
    pub arrival: u64,
    /// The abode it travels to, in place of `to` (`FUN_00543040` maps a
    /// Bounty mission destination, family `0x65`, to Jabba's Palace; a trip
    /// to an abode takes 1 day, `FUN_00555b30`; save v77).
    pub abode: Option<crate::ids::AbodeKey>,
}

/// All active missions across the galaxy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MissionState {
    missions: VecDeque<ActiveMission>,
    next_id: u64,
    /// Members on their way to a target. A member keeps travelling when its
    /// mission ends, as its own transit timer does in the original.
    en_route: Vec<MemberTransit>,
    /// Every captive's armed escape timer, in arming order.
    escape_timers: Vec<EscapeTimer>,
    /// Escaped characters travelling to safety.
    escapees: Vec<EscapeeTransit>,
}

impl MissionState {
    #[must_use]
    pub fn new() -> Self {
        MissionState {
            missions: VecDeque::new(),
            next_id: 0,
            en_route: Vec::new(),
            escape_timers: Vec::new(),
            escapees: Vec::new(),
        }
    }

    /// Members travelling to a mission target.
    #[must_use]
    pub fn en_route(&self) -> &[MemberTransit] {
        &self.en_route
    }

    /// Every captive's armed escape timer, in arming order.
    #[must_use]
    pub fn escape_timers(&self) -> &[EscapeTimer] {
        &self.escape_timers
    }

    /// Escaped characters travelling to safety.
    #[must_use]
    pub fn escapees(&self) -> &[EscapeeTransit] {
        &self.escapees
    }

    /// The last arrival day among a mission's travelling members.
    #[must_use]
    pub fn last_arrival(&self, mission_id: u64) -> Option<u64> {
        self.en_route
            .iter()
            .filter(|transit| transit.mission_id == mission_id)
            .map(|transit| transit.arrival)
            .max()
    }

    /// Whether `member` is travelling.
    #[must_use]
    pub fn is_en_route(&self, member: MissionMember) -> bool {
        self.en_route.iter().any(|transit| transit.member == member)
    }

    /// Aim mission `id` at `abode` instead of its system (the Dagobah
    /// mission, `FUN_0054a860`). Returns whether the mission exists.
    pub fn aim_at_abode(&mut self, id: u64, abode: crate::ids::AbodeKey) -> bool {
        match self.missions.iter_mut().find(|mission| mission.id == id) {
            Some(mission) => {
                mission.target_abode = Some(abode);
                true
            }
            None => false,
        }
    }

    /// Whether `character` is autorouting to safety (`+0x50` bit `0x1000`).
    #[must_use]
    pub fn is_autorouting(&self, character: CharacterKey) -> bool {
        self.escapees
            .iter()
            .any(|transit| transit.character == character)
    }

    /// `FUN_00543040` for a character whose destination is an abode: it
    /// autoroutes there from `from`, arriving the next day (`FUN_00555b30`).
    /// The caller takes it off the map.
    pub fn autoroute_to_abode(
        &mut self,
        character: CharacterKey,
        from: SystemKey,
        abode: crate::ids::AbodeKey,
        now: u64,
    ) {
        self.escapees.push(EscapeeTransit {
            character,
            to: from,
            arrival: now + 1,
            abode: Some(abode),
        });
    }

    /// Put `character` on the autorouting road, for other modules' tests.
    #[cfg(test)]
    pub(crate) fn push_escapee(&mut self, transit: EscapeeTransit) {
        self.escapees.push(transit);
    }

    /// Put `transit` on the road, for other modules' tests.
    #[cfg(test)]
    pub(crate) fn push_transit(&mut self, transit: MemberTransit) {
        self.en_route.push(transit);
    }

    /// Dispatch a new mission and return its assigned id. Members are taken
    /// as given; [`MissionState::dispatch_guarded`] applies the original's
    /// member rules. The mission steps from phase 0 on the next advance.
    pub fn dispatch(&mut self, request: MissionRequest) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut mission = ActiveMission::new(
            id,
            request.kind,
            request.faction,
            request.team,
            request.target_system,
            request.tick,
        );
        mission.decoys = request.decoys;
        mission.target_character = request.target_character;
        mission.target_object = request.target_object;
        self.missions.push_back(mission);
        id
    }

    /// Make mission `id`'s Move land its members aboard `fleet`, the order's
    /// target container (`0x241`); `false` when no such mission waits.
    pub fn set_target_fleet(&mut self, id: u64, fleet: crate::ids::FleetKey) -> bool {
        match self.missions.iter_mut().find(|mission| mission.id == id) {
            Some(mission) => {
                mission.target_fleet = Some(fleet);
                true
            }
            None => false,
        }
    }

    /// Dispatch a mission after the original's member checks, and mark every
    /// member on a mission.
    ///
    /// `FUN_0054bb90` moves prisoners from either list to the captured list
    /// and refuses an empty team (message `0x40`/`0x91`). `FUN_00522b30` then
    /// accepts each member only if it is on the mission's side, is not
    /// already a member, has no mission yet (`RoleOnMissionNotif`
    /// `FUN_00536b00`; a mandatory mission counts, `FUN_00536b80`), and
    /// shares the location of the members already accepted
    /// (`ghidra/notes/decoy-roll.md`, "Adding members"). Completion clears
    /// the flags through `MemberAvailable`.
    pub fn dispatch_guarded(
        &mut self,
        request: MissionRequest,
        world: &mut GameWorld,
    ) -> Result<u64, MissionRefusal> {
        let side_is_alliance = request.faction == MissionFaction::Alliance;
        if !creation_target_ok(request.kind, request.target_object) {
            return Err(MissionRefusal::TargetUnavailable);
        }
        if let Some(&member) = request
            .team
            .iter()
            .chain(&request.decoys)
            .find(|&&member| !creation_member_ok(world, request.kind, member))
        {
            return Err(MissionRefusal::MemberUnavailable(member));
        }
        // FUN_0054bb90 moves every prisoner out of the team and decoy lists
        // into the captured list first. FUN_0054c200 then adds the team, the
        // decoys, and the captured, in that order, so the first free team
        // member sets the location the others must share.
        let mut requested: [Vec<(MissionMember, MemberState)>; 3] = Default::default();
        for (member, list) in request
            .team
            .iter()
            .map(|m| (*m, 0))
            .chain(request.decoys.iter().map(|m| (*m, 1)))
        {
            let state =
                member_state(world, member).ok_or(MissionRefusal::MemberUnavailable(member))?;
            requested[if state.prisoner { 2 } else { list }].push((member, state));
        }
        // port: one refused member refuses the whole request. FUN_0054c200
        // keeps adding after a refusal and returns false; what its caller
        // does with that is untraced.
        let mut lists: [Vec<MissionMember>; 3] = Default::default();
        let mut location = None;
        for (index, members) in requested.iter().enumerate() {
            for &(member, ref state) in members {
                if lists.iter().any(|list| list.contains(&member)) {
                    continue;
                }
                // Saves written before dispatch set the flag can hold an
                // active mission for a member whose flag is still false.
                // port: a member still travelling to an ended mission's
                // target is refused; the original compares its en route bit
                // and arrival tick instead (FUN_00522b30).
                // A prisoner's side is its captor's (FUN_004eec60, the
                // side +0x24 while Captured).
                let side_is = if state.prisoner {
                    state.captor_is_alliance.unwrap_or(state.is_alliance)
                } else {
                    state.is_alliance
                };
                if side_is != side_is_alliance || state.busy || self.engaged(member) {
                    return Err(MissionRefusal::MemberUnavailable(member));
                }
                match location {
                    None => location = Some(state.location),
                    Some(here) if here != state.location => {
                        return Err(MissionRefusal::MemberUnavailable(member));
                    }
                    Some(_) => {}
                }
                lists[index].push(member);
            }
        }
        let [team, decoys, captured] = lists;
        if team.is_empty() {
            return Err(MissionRefusal::EmptyTeam);
        }
        // FUN_005420d0 checks the mask of all three lists (FUN_00583320).
        // port: a kind with no MISSNSD record admits any members.
        if let Some(record) = request
            .kind
            .record_id()
            .and_then(|id| world.mission_record(id))
        {
            let members = team.iter().chain(&decoys).chain(&captured).copied();
            if !members_allowed(world, record.members, members) {
                return Err(MissionRefusal::MembersNotAllowed);
            }
        }
        // FUN_00520b70 reads the record's hidden flag (+0x5c) for a
        // member's status: a Move is not shown as a mission.
        let hidden = request
            .kind
            .record_id()
            .and_then(|id| world.mission_record(id))
            .is_some_and(|record| record.hidden);
        for member in team.iter().chain(&decoys).chain(&captured) {
            set_on_mission(world, *member, true);
            if let Some(c) = member
                .character()
                .and_then(|key| world.characters.get_mut(key))
            {
                c.on_hidden_mission = hidden;
            }
        }
        let id = self.dispatch(MissionRequest {
            team,
            decoys,
            ..request
        });
        if let Some(mission) = self.missions.back_mut() {
            mission.captured = captured;
        }
        Ok(id)
    }

    /// Whether `member` belongs to a queued mission or is still travelling
    /// to one.
    fn engaged(&self, member: MissionMember) -> bool {
        self.missions
            .iter()
            .any(|mission| mission.members().any(|x| x == member))
            || self.is_en_route(member)
    }

    /// Whether the object pop-up menu enables Mission (order `0x240`) for
    /// `team`. No target or kind is checked yet. `FUN_0051fe20` builds a
    /// mission-create command from the team (`FUN_004f4a00`), and
    /// `FUN_005429e0` checks it:
    ///
    /// - every member is the player's (`FUN_004f9860`, status 1), recruited
    ///   (status 2), not on a mission (`FUN_00533ce0`), not travelling
    ///   (status 4), and not a prisoner (`FUN_004ed560`);
    /// - all located members share one place (`FUN_0054bf00`, `0x40`/3);
    /// - at least one member has a location (`0x16`). An empty team fails
    ///   the same way (`FUN_0054bb90`).
    ///
    /// port: `FUN_004ed560` also refuses an injured character (`+0x94`), and
    /// `FUN_004f9860` an object with `+0x50 & 8`; the port tracks neither.
    #[must_use]
    pub fn mission_order_enabled(
        &self,
        world: &GameWorld,
        faction: MissionFaction,
        team: &[MissionMember],
    ) -> bool {
        let side_is_alliance = faction == MissionFaction::Alliance;
        let mut place = None;
        for &member in team {
            let Some(state) = member_state(world, member) else {
                return false;
            };
            if state.is_alliance != side_is_alliance
                || !state.recruited
                || state.busy
                || state.prisoner
                || self.engaged(member)
            {
                return false;
            }
            if state.location == MemberLocation::Nowhere {
                continue;
            }
            match place {
                None => place = Some(state.location),
                Some(here) if here != state.location => return false,
                Some(_) => {}
            }
        }
        place.is_some()
    }

    /// Queue a mission built in a test, past the dispatch rules.
    #[cfg(test)]
    pub(crate) fn push_timed(&mut self, mission: ActiveMission) {
        self.next_id = self.next_id.max(mission.id + 1);
        self.missions.push_back(mission);
    }

    /// Whether `faction` may abort mission `id`: the mission's slot `+0x1c0`
    /// (`FUN_00521600`) runs the base check (`FUN_004f9860`: the ordering
    /// side's), then refuses phase 4 (`FUN_00520b20`, code `0x40`) and any
    /// phase but 8 (`FUN_00520b30`, code `0x41`); an ended mission has
    /// nothing to abort.
    #[must_use]
    pub fn abort_allowed(&self, id: u64, faction: MissionFaction) -> bool {
        self.missions
            .iter()
            .find(|m| m.id == id)
            .is_some_and(|m| m.faction == faction && m.phase == 8 && m.end_code == 0)
    }

    /// End a running mission as its owner's abort does: order `0x250`
    /// (`FUN_00580580`, executed by `FUN_0054e310`) calls the mission's slot
    /// `+0x1d4` -> `+0x1d8` (`FUN_005233a0`) -> `+0x1dc(2)`, end code 2 while
    /// the mission runs, so its next step goes to phase `0xb`. Returns
    /// whether the mission ran.
    pub fn abort(&mut self, id: u64) -> bool {
        match self.missions.iter_mut().find(|m| m.id == id) {
            Some(mission) if mission.end_code == 0 => {
                mission.end_code = 2;
                mission.ready = true;
                true
            }
            _ => false,
        }
    }

    /// Cancel a mission by id and free its members for another assignment.
    pub fn release(&mut self, id: u64, world: &mut GameWorld) -> Option<ActiveMission> {
        let mission = self.cancel(id)?;
        for member in mission.members() {
            set_on_mission(world, member, false);
        }
        Some(mission)
    }

    /// Cancel a mission by id. Returns the mission if found, None otherwise.
    pub fn cancel(&mut self, id: u64) -> Option<ActiveMission> {
        if let Some(pos) = self.missions.iter().position(|m| m.id == id) {
            self.missions.remove(pos)
        } else {
            None
        }
    }

    /// All active missions (read-only).
    #[must_use]
    pub fn missions(&self) -> &VecDeque<ActiveMission> {
        &self.missions
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.missions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.missions.is_empty()
    }
}

// ---------------------------------------------------------------------------
// MissionEffect
// ---------------------------------------------------------------------------

/// What actually changed in the game world as a result of a mission.
///
/// The caller in `main.rs` is responsible for applying these effects to
/// `GameWorld`. `MissionSystem` is stateless — it only produces the events.
#[derive(Debug, Clone, PartialEq)]
pub enum MissionEffect {
    // ── Living Galaxy ────────────────────────────────────────────────────────
    /// A Diplomacy success (`FUN_00574120`) changes `side`'s support at
    /// `system` by `points` through `FUN_0050c9f0`
    /// ([`crate::uprising::apply_support_change`]).
    SupportGained {
        system: SystemKey,
        side: crate::dat::Faction,
        points: i32,
    },
    /// A successful member's base skill rises by `amount` (GNPRTB
    /// 6156..6168, the skill setters `FUN_00533e20`..`FUN_005341a0`).
    SkillRaised {
        character: CharacterKey,
        skill: Skill,
        amount: i32,
    },
    /// A research mission's points go to its side's tree (`FUN_0056cf30` ->
    /// `FUN_00530600`); apply with [`ResearchState::add_points`].
    ResearchPoints {
        faction: MissionFaction,
        tech: TechType,
        points: u32,
    },
    /// A pool character joins `faction` at `system` ([`recruit_character`]).
    CharacterRecruited {
        character: CharacterKey,
        system: SystemKey,
        faction: MissionFaction,
        /// The pick took the side's last pool character (`FUN_0055fc80`).
        pool_emptied: bool,
    },

    // ── War Machine ──────────────────────────────────────────────────────────
    /// A Sabotage or DS Sabotage target at `system` is destroyed
    /// ([`destroy_target`]).
    TargetSabotaged {
        target: MissionTarget,
        system: SystemKey,
    },

    /// A character was killed — remove from the game.
    CharacterKilled {
        character: CharacterKey,
        faction: MissionFaction,
    },

    /// A character was captured by the opposing faction.
    ///
    /// The captured character is held at `at_system` until rescued or the game ends.
    CharacterCaptured {
        character: CharacterKey,
        /// Which faction now holds the character.
        captured_by: MissionFaction,
        at_system: SystemKey,
    },

    /// A captured character was successfully rescued.
    CharacterRescued {
        character: CharacterKey,
        /// Faction the character was returned to.
        returned_to: MissionFaction,
        at_system: SystemKey,
    },

    /// Intelligence gathered — reveal fog of war over the target system.
    SystemIntelligenceGathered {
        system: SystemKey,
        /// Faction that gained the intelligence.
        faction: MissionFaction,
    },

    /// An Incite Uprising member's success ran the uprising incident
    /// `FUN_0050d030` at the target (`FUN_00571a60`); apply it with
    /// [`crate::uprising::apply_uprising_event`].
    UprisingIncident(crate::uprising::UprisingEvent),

    // ── Member availability tracking ────────────────────────────────────────
    /// A member has left its mission and may take another
    /// (`RoleOnMissionNotif` `FUN_00536b00` cleared).
    MemberAvailable { member: MissionMember },
    /// A member joined a mission the advance created, a Return order
    /// (`FUN_00522b30`: `RoleOnMissionNotif`, and `OnHiddenMission` from
    /// the record's `+0x5c`, `FUN_00520b70`).
    MemberOnMission { member: MissionMember, hidden: bool },
    /// An adrift character stays where it is: `FUN_0054c7d0` clears its
    /// Adrift (`FUN_00534790(0)`) and Autorouting together.
    AdriftSettled { character: CharacterKey },
    /// A member landed aboard the fleet its Move targets (the container
    /// `+0x74`, `FUN_004f7640`).
    MemberBoarded {
        member: MissionMember,
        fleet: crate::ids::FleetKey,
    },
    /// A member moved for its mission: off the map when it starts a transit
    /// (`to: None`), into the target system when it lands (`FUN_00556390`).
    MemberMoved {
        member: MissionMember,
        to: Option<SystemKey>,
    },

    // ── Escape ──────────────────────────────────────────────────────────────
    /// A captured character escaped on their own.
    CharacterEscaped {
        character: CharacterKey,
        escaped_to_alliance: bool,
    },

    /// A Subdue Uprising succeeded (`FUN_00569c20`): `side` wins
    /// `support_gain` points (`FUN_0055cb10`), then the revolt ends if the
    /// system is garrisoned (`FUN_0050c910`).
    UprisingSubdued {
        system: SystemKey,
        side: crate::dat::Faction,
        support_gain: i32,
    },

    /// A special force captured on a mission is destroyed (slot `+0x20c`,
    /// `FUN_00503eb0` -> slot `+0xac(9)`).
    SpecialForceDestroyed { unit: crate::ids::SpecialForceKey },

    /// A character took the injury roll `FUN_004ef5f0`; slot `+0x2f0`
    /// (`FUN_004ef6d0`) adds it to `+0x94`.
    CharacterInjured {
        character: CharacterKey,
        injury: i32,
    },

    /// A member reached the abode its mission targets (`FUN_00556390`, a
    /// 1-day trip).
    MemberEnteredAbode {
        member: MissionMember,
        abode: crate::ids::AbodeKey,
    },

    /// Dagobah's end (`FUN_005751d0`): Luke's Force rose by `force_gain`
    /// into `ForceExperience` `+0x8e`, and `DagobahCompleted` (`+0xb0` bit
    /// `0x8`) is set.
    DagobahCompleted {
        character: CharacterKey,
        force_gain: i32,
    },

    /// A Jedi Training pupil gained `amount` `ForceTraining` `+0x90`
    /// (`FUN_005712b0` -> `FUN_004ee1d0`).
    ForceTrainingGained {
        character: CharacterKey,
        amount: i32,
    },

    /// Luke or Leia evaded capture: their slot `+0x208` (`FUN_0056f8b0`,
    /// `FUN_0056ffa0`) adds GNPRTB 3081 or 3082 (1) to `ForceExperience`
    /// `+0x8e` (`FUN_004ee150`).
    ForceExperienceGained {
        character: CharacterKey,
        amount: i32,
    },

    /// The Bounty mission attacked Han (`FUN_00572690`, `BountyAttack`,
    /// notification `0x212`); `captured` when he did not evade.
    BountyAttack {
        character: CharacterKey,
        captured: bool,
    },
    /// The Bounty mission captured Han (`FUN_00575de0`): `CanEscape` cleared
    /// (`FUN_004eeb10(0)`) and `CapturedByBountyHunters` set (`FUN_00572700`).
    CapturedByBountyHunters { character: CharacterKey },
    /// A mission member learned through the Force who betrayed its mission
    /// (`FUN_00588e80`; its TraitorDiscovered key `+0xa0`, notification
    /// `0x361`: Character Discovers Traitor).
    TraitorIdentified {
        character: CharacterKey,
        traitor: CharacterKey,
    },
    /// An injury killed a minor character: slot `+0x2f0` (`FUN_004ef6d0`)
    /// destroys it when slot `+0x2e0` says so
    /// ([`crate::mission_detection::dies_of_injury`]).
    DiedOfInjury { character: CharacterKey },
    /// A prisoner loses `CanEscape` (`FUN_004eeb10(0)`): a rescuer the
    /// Palace mission's failure leaves captive (`FUN_0056e870`).
    EscapeBarred { character: CharacterKey },
    /// The Pickup's hit set and cleared the character manager's
    /// `PickupInProgress` (`FUN_0055f6c0`, notification `0x231`: Jabba
    /// Delivers Prisoners).
    PickupInProgress,
    /// A prisoner of Jabba's Palace joined the Pickup mission
    /// (`FUN_0056da80`, slot `+0x1cc`) and took Vader's `+0x70`
    /// (`FUN_00534390`).
    PrisonerPickedUp { character: CharacterKey },

    /// Force detection (`FUN_0058a3f0` -> `FUN_0055ff60`): `member` and
    /// `defender` name each other in `Encounter` `+0x9c` and record `day`.
    EncounterBegan {
        member: CharacterKey,
        defender: CharacterKey,
        day: u64,
    },
    /// The confrontation (`FUN_00560200`) cleared both `Encounter` keys and
    /// days.
    EncounterEnded {
        member: CharacterKey,
        partner: CharacterKey,
    },
    /// Luke met Vader and learned his heritage (`FUN_0056f650`,
    /// `KnowsHeritage`, `+0xb0` bit 0).
    HeritageLearned { character: CharacterKey },
}

// ---------------------------------------------------------------------------
// MissionOutcome / MissionResult
// ---------------------------------------------------------------------------

/// Whether a mission's phase 10 succeeded: its result `+0x60` is 3
/// (`FUN_00521880`), else 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionOutcome {
    Success,
    Failure,
}

/// The result emitted when a mission resolves.
#[derive(Debug, Clone)]
pub struct MissionResult {
    /// The id of the mission that resolved.
    pub mission_id: u64,
    /// The game-day on which the mission resolved.
    pub tick: u64,
    pub kind: MissionKind,
    pub faction: MissionFaction,
    /// The first team character (`ActiveMission::lead_character`), named in
    /// the mission's messages.
    pub character: Option<CharacterKey>,
    pub target_system: SystemKey,
    pub outcome: MissionOutcome,
    /// World-state changes to apply, in the order the members acted.
    pub effects: Vec<MissionEffect>,
}

// ---------------------------------------------------------------------------
// The running validator
// ---------------------------------------------------------------------------

/// Han Solo's DatId (`FUN_00506f50`); his presence speeds a mission's
/// characters (`FUN_00548370`).
const HAN_SOLO: u32 = 0x3300_0243;
/// GNPRTB parameter 3083, the Han Solo mission speed (`FUN_005725a0`).
const GNPRTB_HAN_SOLO_SPEED: u16 = 3083;
/// Shipped GNPRTB.DAT value of parameter 3083, used only when no GNPRTB
/// table is loaded.
const SHIPPED_HAN_SOLO_SPEED: i64 = 50;
/// GNPRTB 2584 (`DAT_006b90ac`, shipped 100): the minimum days of the
/// escape timer `0x38a` (`FUN_004ef750`, record `+0x54`+`0x28`).
const GNPRTB_ESCAPE_MIN: u16 = 2584;
/// GNPRTB 2585 (`DAT_006b9074`, shipped 1000): its random spread.
const GNPRTB_ESCAPE_SPREAD: u16 = 2585;
/// GNPRTB 6156..6168 (`FUN_0055bfd0`), each shipped 1: the base skill a
/// successful member gains.
const GNPRTB_DIPLOMACY_RAISE: u16 = 6156;
/// GNPRTB 6157 (`DAT_006bb534`): Espionage's espionage raise.
const GNPRTB_ESPIONAGE_RAISE: u16 = 6157;
/// GNPRTB 6159 (`DAT_006bb5b0`): Recruitment's leadership raise.
const GNPRTB_RECRUITMENT_RAISE: u16 = 6159;
/// GNPRTB 6160 (`DAT_006bb5a0`): Incite's leadership raise.
const GNPRTB_INCITE_RAISE: u16 = 6160;
/// GNPRTB 6161 (`DAT_006bb538`): Subdue's leadership raise.
const GNPRTB_SUBDUE_RAISE: u16 = 6161;
/// GNPRTB 6162 (`DAT_006bb55c`): Rescue's combat raise.
const GNPRTB_RESCUE_RAISE: u16 = 6162;
/// GNPRTB 6163 (`DAT_006bb574`): Abduction's combat raise.
const GNPRTB_ABDUCTION_RAISE: u16 = 6163;
/// GNPRTB 6164 (`DAT_006bb584`): Assassination's combat raise.
const GNPRTB_ASSASSINATION_RAISE: u16 = 6164;
/// GNPRTB 6165 (`DAT_006bb58c`): Sabotage's espionage raise.
const GNPRTB_SABOTAGE_ESPIONAGE_RAISE: u16 = 6165;
/// GNPRTB 6166 (`DAT_006bb518`): Sabotage's combat raise.
const GNPRTB_SABOTAGE_COMBAT_RAISE: u16 = 6166;
/// GNPRTB 6167 (`DAT_006bb5c0`): DS Sabotage's espionage raise.
const GNPRTB_DS_SABOTAGE_ESPIONAGE_RAISE: u16 = 6167;
/// GNPRTB 6168 (`DAT_006bb57c`): DS Sabotage's combat raise.
const GNPRTB_DS_SABOTAGE_COMBAT_RAISE: u16 = 6168;
/// GNPRTB 6158 (`Gnprtb_6158`): a research member's raise of its skill in
/// the tree (`FUN_0056cdb0`).
const GNPRTB_RESEARCH_RAISE: u16 = 6158;
/// The shipped value of every skill raise, GNPRTB 6156..6170.
const SHIPPED_SKILL_RAISE: i64 = 1;
/// GNPRTB 6154 (`DAT_006bb524`): the Palace chance's espionage divisor
/// (`FUN_0055c910`).
const GNPRTB_PALACE_ESPIONAGE_DIVISOR: u16 = 6154;
/// GNPRTB 6155 (`DAT_006bb5b4`): its combat divisor.
const GNPRTB_PALACE_COMBAT_DIVISOR: u16 = 6155;
/// GNPRTB 6154 and 6155 as shipped.
const SHIPPED_PALACE_DIVISOR: i64 = 3;
/// GNPRTB 6169 (`Gnprtb_6169`): a Palace rescuer's espionage raise
/// (`FUN_0056e650`).
const GNPRTB_PALACE_ESPIONAGE_RAISE: u16 = 6169;
/// GNPRTB 6170 (`Gnprtb_6170`): its combat raise.
const GNPRTB_PALACE_COMBAT_RAISE: u16 = 6170;
/// GNPRTB 6171 (`DAT_006bb5c4`, shipped 1): a research success's minimum
/// points (`FUN_0055c9f0`).
const GNPRTB_RESEARCH_POINTS_MIN: u16 = 6171;
/// The shipped GNPRTB 6171 (`DAT_006bb5c4`).
const SHIPPED_RESEARCH_POINTS_MIN: i64 = 1;
/// GNPRTB 6172 (`DAT_006bb578`, shipped 6): their random spread.
const GNPRTB_RESEARCH_POINTS_SPREAD: u16 = 6172;
/// The shipped GNPRTB 6172 (`DAT_006bb578`).
const SHIPPED_RESEARCH_POINTS_SPREAD: i64 = 6;

/// A skill raise's GNPRTB value, or the shipped one without a table.
fn skill_raise(world: &GameWorld, id: u16) -> i32 {
    i32::try_from(crate::movement::gnprtb_or_shipped(
        world,
        id,
        SHIPPED_SKILL_RAISE,
    ))
    .unwrap_or(0)
}

/// Full support, the Diplomacy end threshold (`DAT_00661a88`, `FUN_00573ee0`).
const FULL_SUPPORT: i32 = 100;

/// Which rules the class validator adds to the base rules
/// (`ghidra/notes/mission-lifecycle.md`, "Validators per class").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetRules {
    /// `FUN_005868c0`: the uprising columns.
    System,
    /// `FUN_00586e20`: the prisoner columns.
    Character,
    /// `FUN_00593500`: the base rules only, while running.
    Object,
    /// `FUN_00523450`: the base rules only, at a system target (Move).
    Place,
}

impl MissionKind {
    /// The kind's MISSNSD.DAT record id (`family << 24 | index`), `None`
    /// for the port-only `Autoscrap`. The families are the class slot `+4`
    /// codes (`ghidra/notes/mission-lifecycle.md`, "Slots per class").
    #[must_use]
    pub fn record_id(self) -> Option<crate::ids::DatId> {
        let raw = match self {
            MissionKind::Diplomacy => 0x5100_0010,
            MissionKind::Rescue => 0x6100_0011,
            MissionKind::Sabotage => 0x6900_0012,
            MissionKind::Espionage => 0x5200_0013,
            MissionKind::Recruitment => 0x5500_0016,
            MissionKind::Abduction => 0x6200_0017,
            MissionKind::InciteUprising => 0x5600_0040,
            MissionKind::DeathStarSabotage => 0x6a00_0041,
            MissionKind::SubdueUprising => 0x5700_0080,
            MissionKind::Assassination => 0x6300_0081,
            MissionKind::Reconnaissance => 0x5400_0015,
            MissionKind::ShipDesignResearch => 0x5300_0020,
            MissionKind::FacilityDesignResearch => 0x5300_0021,
            MissionKind::TroopTrainingResearch => 0x5300_0022,
            MissionKind::Move => 0x4100_0001,
            MissionKind::Return => 0x4200_0002,
            MissionKind::Adrift => 0x4400_0004,
            MissionKind::JediTraining => 0x5800_0042,
            MissionKind::Dagobah => 0x7100_0043,
            MissionKind::Bounty => 0x6500_0083,
            MissionKind::Palace => 0x6400_0044,
            MissionKind::Pickup => 0x7300_0082,
            MissionKind::Autoscrap => return None,
        };
        Some(crate::ids::DatId::new(raw))
    }

    pub(crate) fn target_rules(self) -> TargetRules {
        match self {
            MissionKind::Rescue | MissionKind::Abduction | MissionKind::Assassination => {
                TargetRules::Character
            }
            MissionKind::Sabotage | MissionKind::DeathStarSabotage | MissionKind::Autoscrap => {
                TargetRules::Object
            }
            MissionKind::Diplomacy
            | MissionKind::Espionage
            | MissionKind::Recruitment
            | MissionKind::InciteUprising
            | MissionKind::SubdueUprising
            // FUN_0056be20: the system rules alone.
            | MissionKind::Reconnaissance
            // FUN_0056c510: the system rules and its own.
            | MissionKind::ShipDesignResearch
            | MissionKind::FacilityDesignResearch
            | MissionKind::TroopTrainingResearch => TargetRules::System,
            // FUN_005215e0: the base rules alone, at a system target.
            MissionKind::Move | MissionKind::Return | MissionKind::Adrift => TargetRules::Place,
            // FUN_00570690: the system rules, then its own.
            MissionKind::JediTraining => TargetRules::System,
            // Its target is an abode, a place (FUN_00521070).
            MissionKind::Dagobah => TargetRules::Place,
            // An opponent's free character (MISSNSD 0x65000083).
            // port: the class's own rules (slot +0x1bc FUN_005757f0:
            // FUN_00592aa0, FUN_00575900) are not read; the prisoner columns
            // stand in.
            MissionKind::Bounty => TargetRules::Character,
            // An opponent's prisoner (MISSNSD 0x64000044), held in the
            // palace, its abode target.
            MissionKind::Palace => TargetRules::Character,
            // Its target is the palace, the Empire's own (MISSNSD
            // 0x73000082).
            MissionKind::Pickup => TargetRules::Place,
        }
    }
}

/// What the base rules read about a mission's target `T`.
struct TargetView {
    side: Option<crate::dat::Faction>,
    destroyed: bool,
    en_route: bool,
    location: Option<SystemKey>,
    prisoner: bool,
}

/// A character's location key: its fleet's system, else its own.
fn character_location(world: &GameWorld, character: &Character) -> Option<SystemKey> {
    match character.current_fleet {
        Some(fleet) => world.fleets.get(fleet).map(|f| f.location),
        None => character.current_system,
    }
}

/// A member's location key, as [`character_location`] reads a character.
/// The abode a member stands in: a character's container of family `0x98`,
/// or the abode holding a special force.
fn member_abode(world: &GameWorld, member: MissionMember) -> Option<crate::ids::AbodeKey> {
    match member {
        MissionMember::Character(key) => world.characters.get(key)?.current_abode,
        MissionMember::SpecialForce(key) => world
            .abodes
            .iter()
            .find(|(_, abode)| abode.special_forces.contains(&key))
            .map(|(abode, _)| abode),
    }
}

fn member_location(world: &GameWorld, member: MissionMember) -> Option<SystemKey> {
    match member_state(world, member)?.location {
        MemberLocation::Fleet(fleet) => world.fleets.get(fleet).map(|f| f.location),
        MemberLocation::System(system) => Some(system),
        MemberLocation::Nowhere => None,
    }
}

/// The side a character reads as (`+0x24` bits 6..7). A prisoner carries
/// its captor's side: every Captured change sets it to the other side of
/// the character's home side, or back home on release (`FUN_004eec60`,
/// `ghidra/notes/captive-escape.md`).
#[must_use]
pub fn character_side(character: &Character) -> Option<crate::dat::Faction> {
    use crate::dat::Faction;
    if character.is_captive {
        character.captured_by
    } else if character.is_alliance {
        Some(Faction::Alliance)
    } else if character.is_empire {
        Some(Faction::Empire)
    } else {
        None
    }
}

/// The target the validator reads, `None` when it is missing (rule 1).
///
/// port: the target of a system kind is its target system, which is also
/// its container; an object kind reads its named object (a mission without
/// one reads as missing). A system reads as located in itself (`hyp:` the system
/// slot `+0xc` is untraced) and as its holder's side (inference).
fn target_view(
    mission: &ActiveMission,
    world: &GameWorld,
    en_route: &[MemberTransit],
) -> Option<TargetView> {
    match mission.kind.target_rules() {
        // FUN_00521030 reads the order's object. port: the original keeps a
        // destroyed object (+0x50 bit 3) for its rule 4; the port removes
        // it, so a removed object reads as the opponent's and destroyed.
        TargetRules::Object => {
            let object = mission.target_object?.state(world);
            let opponent = match crate::dat::Faction::from(mission.faction) {
                crate::dat::Faction::Alliance => crate::dat::Faction::Empire,
                _ => crate::dat::Faction::Alliance,
            };
            Some(TargetView {
                side: Some(object.as_ref().map_or(opponent, |object| object.side)),
                destroyed: object.is_none(),
                en_route: false,
                location: object.and_then(|object| object.location),
                prisoner: false,
            })
        }
        // A Move to a fleet reads the fleet: its side and its system.
        TargetRules::Place if mission.target_fleet.is_some() => {
            let fleet = mission.target_fleet.and_then(|key| world.fleets.get(key));
            Some(TargetView {
                side: fleet.map(|fleet| crate::mission_detection::faction_of(fleet.is_alliance)),
                destroyed: fleet.is_none(),
                en_route: false,
                location: fleet.map(|fleet| fleet.location),
                prisoner: false,
            })
        }
        TargetRules::System | TargetRules::Place => {
            let sys = world.systems.get(mission.target_system)?;
            Some(TargetView {
                side: crate::uprising::holder(sys),
                destroyed: sys.is_destroyed,
                en_route: false,
                location: Some(mission.target_system),
                prisoner: false,
            })
        }
        TargetRules::Character => {
            let key = mission.target_character?;
            let character = world.characters.get(key)?;
            Some(TargetView {
                side: character_side(character),
                destroyed: character.is_killed,
                // port: only a mission member's transit is visible here; a
                // target aboard a moving fleet reads its fleet's location.
                en_route: en_route
                    .iter()
                    .any(|transit| transit.member == MissionMember::Character(key)),
                location: character_location(world, character),
                prisoner: character.is_captive,
            })
        }
    }
}

/// The validator `FUN_00522480` over the running checks: the end code the
/// first failing rule sets, or 0 (`ghidra/notes/mission-lifecycle.md`, "The
/// validator and the end codes").
///
/// port: the validator reads the real container and target. Before phase 5
/// with a remote target (`FUN_00520af0` false), the original reads the
/// side's own copy of them instead (`FUN_00521160`, `FUN_005211c0` through
/// `FUN_004f2d10`); the port keeps no per-side copies.
pub(crate) fn running_end_code(
    mission: &ActiveMission,
    world: &GameWorld,
    uprisings: &crate::uprising::UprisingState,
    research: &ResearchState,
    en_route: &[MemberTransit],
) -> u8 {
    use crate::dat::Faction;
    // Rule 1 (FUN_00522480.c:42-52): end 5 unless a team member stands
    // without a remove or resign request (role +0x78 & 0xc); an empty team
    // ends too, and a killed member is deleted. +0xa4 bit 1 (ImpliedTeam,
    // FUN_005221a0), which lifts the rule, is set only on the Autorouting
    // order 0x43 (FUN_00547740), which the port does not run.
    let standing = mission.team.iter().any(|&member| {
        member_state(world, member).is_some()
            && !mission.resigning.contains(&member)
            && !mission.removing.contains(&member)
    });
    if !standing {
        return 5;
    }
    // port: a kind with no MISSNSD record runs no other checks.
    let Some(record) = mission
        .kind
        .record_id()
        .and_then(|id| world.mission_record(id))
    else {
        return 0;
    };
    // FUN_005830a0: members of both sides (status 0x14) or of none (0x16)
    // skip every later check without an end code. Rule 1 has already ended
    // a mission with no standing team member, so some member has a side.
    let sides: Vec<bool> = mission
        .members()
        .filter_map(|member| member_state(world, member).map(|state| state.is_alliance))
        .collect();
    if sides.contains(&true) && sides.contains(&false) {
        return 0;
    }
    // port: an abode target (Dagobah) is a place no rule below can lose: it
    // is never destroyed, never travels and has its own side (3 for
    // Dagobah, "another" for the Dagobah record).
    if let Some(abode) = mission.target_abode {
        // FUN_00523450 rules 5 and 6 for the Palace mission's prisoner,
        // who must stay captive in the abode (MISSNSD 0x64000044, col 20).
        let lost = mission.target_character.is_some_and(|key| {
            world
                .characters
                .get(key)
                .is_none_or(|c| c.is_killed || !c.is_captive || c.current_abode != Some(abode))
        });
        return if lost { 6 } else { 0 };
    }
    let rules = record.rules;
    // FUN_00523450 rule 1: a missing target or container skips the rest.
    let Some(container) = world.systems.get(mission.target_system) else {
        return 0;
    };
    let Some(target) = target_view(mission, world, en_route) else {
        return 0;
    };
    // Rule 2 (col 11): the container is destroyed.
    if rules.container_loss_ends && container.is_destroyed {
        return 7;
    }
    // Rule 3 (cols 15..17): the target's side against the mission's.
    let side = Faction::from(mission.faction);
    let opponent = match side {
        Faction::Alliance => Faction::Empire,
        Faction::Empire | Faction::Neutral => Faction::Alliance,
    };
    let side_allowed = match target.side {
        Some(s) if s == side => rules.own_side_target,
        Some(s) if s == opponent => rules.opponent_target,
        _ => rules.other_side_target,
    };
    if !side_allowed {
        return 8;
    }
    // Rule 4 (col 14): the target is destroyed.
    if rules.target_loss_ends && target.destroyed {
        return 6;
    }
    // Rules 5 and 6: the target is travelling or has left the container.
    if target.en_route || target.location != Some(mission.target_system) {
        return 6;
    }
    // Rule 7 (col 13): the container must be a populated system.
    if rules.needs_populated_container && !container.is_populated {
        return 0xd;
    }
    match mission.kind.target_rules() {
        // FUN_005868c0 (cols 18, 19).
        TargetRules::System => {
            let revolting = uprisings
                .active_uprisings
                .contains_key(&mission.target_system);
            let allowed = if revolting {
                rules.revolting_target
            } else {
                rules.calm_target
            };
            if !allowed {
                return 6;
            }
        }
        // FUN_00586e20 (cols 20, 21).
        TargetRules::Character => {
            let allowed = if target.prisoner {
                rules.prisoner_target
            } else {
                rules.free_target
            };
            if !allowed {
                return 6;
            }
        }
        TargetRules::Object | TargetRules::Place => {}
    }
    // FUN_00573ee0: Diplomacy stops at full support.
    if mission.kind == MissionKind::Diplomacy
        && crate::uprising::support_points(container, side) == FULL_SUPPORT
    {
        return 0xf;
    }
    // FUN_0056b370: Recruitment stops once its side recruited the last pool
    // character (side +0xb8). port: its +0xc test and the FUN_0056b680 and
    // FUN_0056b550 rules are not traced.
    if mission.kind == MissionKind::Recruitment && world.recruit_pool_empty(side) {
        return 0x10;
    }
    // FUN_00570690: Jedi Training's teacher and pupils.
    if mission.kind == MissionKind::JediTraining && jedi_training_refused(world, mission) {
        return JEDI_TRAINING_REFUSED;
    }
    // FUN_0056c510 while running (+0xc): no research site ends with 0xc
    // (0x40/0x2c); the side's Done flag in the tree (FUN_0052eb10) ends with
    // 0xe (0x40/0x32), written after 0xc and so in its place.
    if let Some(tech) = mission.kind.research_tech() {
        if research.progress(side == Faction::Alliance, tech).done {
            return 0xe;
        }
        if !research_site(world, uprisings, side, tech, mission.target_system) {
            return 0xc;
        }
    }
    0
}

/// Whether `system` can host `side`'s research in `tech`: its production
/// manager for the tree's area (`FUN_00509670`) holds a yard of the side
/// that is not Suspended (`FUN_00528100(0)`, yard `+0x60` bit 0).
///
/// port: a yard is Suspended while its system is blockaded or in an
/// uprising, as `crate::production_cycle::suspended_systems` reads it; the
/// blockade is read from the fleets present.
pub(crate) fn research_site(
    world: &GameWorld,
    uprisings: &crate::uprising::UprisingState,
    side: crate::dat::Faction,
    tech: TechType,
    system: SystemKey,
) -> bool {
    let Some(sys) = world.systems.get(system) else {
        return false;
    };
    if uprisings.is_uprising(system)
        || crate::blockade::BlockadeSystem::system_is_blockaded(world, sys)
    {
        return false;
    }
    let family = crate::build_selection::area_family(tech.area());
    sys.manufacturing_facilities
        .iter()
        .filter_map(|&key| world.manufacturing_facilities.get(key))
        .any(|yard| yard.side == side && yard.class_dat_id.family() == family)
}

// ---------------------------------------------------------------------------
// MissionSystem
// ---------------------------------------------------------------------------

/// Stateless system that steps missions through their phases.
///
/// # RNG contract
///
/// The caller provides `rolls`: [`ROLLS_PER_MISSION`] per active mission per
/// tick event. Mission `i` (its position when the event starts) on event `e`
/// draws from the window starting at `(e * n + i) * ROLLS_PER_MISSION`, `n`
/// being the number of missions when the advance starts, so one mission's
/// draws never shift another's. A draw beyond the window, or beyond the
/// slice, reads `0.5`. Each roll is uniform [0, 1). port: the original
/// draws inline; the windows keep the port's draws deterministic per
/// mission.
pub struct MissionSystem;

/// Draws one mission may take on one tick event: the creation chain's timer
/// and the repeat's timer, then the seed of the mission's stream, which the
/// detection run and the phase 10 roll draw from. port: the size of the
/// port's per-mission window.
pub const ROLLS_PER_MISSION: usize = 3;

/// port: the window slot that seeds the mission's stream; the draws before
/// it are taken in order.
const STREAM_SEED_ROLL: usize = 2;

/// One mission's window of rolls for one tick event.
struct MissionRolls<'a> {
    rolls: &'a [f64],
    next: usize,
    stream: crate::mission_detection::MissionRng,
}

impl MissionRolls<'_> {
    /// port: a draw past the window reads 0.5, the port's under-supply
    /// fallback.
    fn next(&mut self) -> f64 {
        let roll = self
            .rolls
            .get(self.next)
            .filter(|_| self.next < STREAM_SEED_ROLL)
            .copied()
            .unwrap_or(0.5);
        self.next += 1;
        roll
    }
}

/// A mission that reached phase `0xb` and was removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionEnded {
    pub mission_id: u64,
    pub kind: MissionKind,
    pub faction: MissionFaction,
    /// The end code (`+0x64`): 1 for a mission that ran its course, else
    /// the validator's reason.
    pub end_code: u8,
    pub tick: u64,
}

/// What one [`MissionSystem::advance`] produced.
#[derive(Debug, Clone, Default)]
pub struct MissionAdvance {
    /// Phase 10 results, one per pass (a repeating mission may resolve
    /// again), in the order they resolved.
    pub results: Vec<MissionResult>,
    /// Member moves and releases in the order they happened. The caller
    /// applies them before `results`.
    pub effects: Vec<MissionEffect>,
    /// Missions that ended, in the order they ended.
    pub ended: Vec<MissionEnded>,
}

/// What a phase step reads besides the mission.
struct StepContext<'a> {
    world: &'a GameWorld,
    uprisings: &'a crate::uprising::UprisingState,
    /// The sides' research, which the research validator reads.
    research: &'a ResearchState,
    /// Every mission as the tick event began, for the uprising incident's
    /// mission terms.
    missions: &'a VecDeque<ActiveMission>,
}

impl MissionSystem {
    /// Step every mission through the ticks in `tick_events`
    /// (`ghidra/notes/mission-lifecycle.md`, "Phase stepping").
    ///
    /// For each tick, members whose transit is due arrive, a mission whose
    /// members have all arrived (phase 4) or whose timer is due (phase 8)
    /// becomes ready, a lost container or target runs the validator
    /// (`FUN_00545240`), and each ready mission steps through phases until
    /// it waits again or ends. A new mission first runs its creation steps
    /// on the day it was ordered.
    ///
    /// The caller applies [`MissionAdvance::effects`], then each result's
    /// effects, to `GameWorld`.
    pub fn advance(
        state: &mut MissionState,
        world: &GameWorld,
        uprisings: &crate::uprising::UprisingState,
        research: &ResearchState,
        tick_events: &[TickEvent],
        rolls: &[f64],
    ) -> MissionAdvance {
        let mut out = MissionAdvance::default();
        let count = state.missions.len();
        for (event_index, event) in tick_events.iter().enumerate() {
            let now = event.tick;
            // FUN_00545820: members whose transit is due arrive.
            let (arrived, travelling): (Vec<_>, Vec<_>) = std::mem::take(&mut state.en_route)
                .into_iter()
                .partition(|transit| transit.arrival <= now);
            state.en_route = travelling;
            for transit in arrived {
                let mission = state.missions.iter().find(|m| m.id == transit.mission_id);
                if let Some(leg) = mission.and_then(|mission| reroute(world, mission, &transit, now)) {
                    // The fleet is the container: the mission's place follows it.
                    if let Some(mission) =
                        state.missions.iter_mut().find(|m| m.id == transit.mission_id)
                    {
                        mission.target_system = leg.to;
                    }
                    state.en_route.push(leg);
                    continue;
                }
                out.effects.extend(landing(world, mission, transit.member, transit.to, transit.abode));
            }

            let mut orders = Self::send_adrift(world, state, &mut out, now);
            let missions = std::mem::take(&mut state.missions);
            let ctx = StepContext {
                world,
                uprisings,
                research,
                missions: &missions,
            };
            for (index, mut mission) in missions.clone().into_iter().enumerate() {
                // port: the mission's own window (see the RNG contract).
                let start = (event_index * count + index) * ROLLS_PER_MISSION;
                let window = rolls
                    .get(start..(start + ROLLS_PER_MISSION).min(rolls.len()))
                    .unwrap_or(&[]);
                let mut draws = MissionRolls {
                    rolls: window,
                    next: 0,
                    stream: crate::mission_detection::MissionRng::seeded(
                        window.get(STREAM_SEED_ROLL).copied().unwrap_or(0.5),
                        mission.id,
                    ),
                };

                // port: dispatch only queues the mission; its creation steps
                // run here, dated the day it was ordered.
                if mission.phase == 0 {
                    let ordered = mission.ordered_tick.min(now);
                    Self::run(&mut mission, &ctx, state, &mut out, ordered, &mut draws);
                }
                if mission.phase < PHASE_END {
                    Self::wake(&mut mission, &ctx, state, now);
                    Self::run(&mut mission, &ctx, state, &mut out, now, &mut draws);
                }
                // port: the original keeps an ended mission one more day:
                // FUN_005229c0 arms timer 0x38c (FUN_005236e0: min 1, spread
                // 0) once the member lists are empty (FUN_00520d60), and its
                // handler FUN_00522a30 deletes the mission in phase 0xb (slot
                // +0xac(0x14)). The members are released here, so the port
                // drops it at once.
                if mission.phase == PHASE_END {
                    orders.extend(Self::finish(&mission, world, state, &mut out, now));
                } else {
                    state.missions.push_back(mission);
                }
            }
            // The Return orders step at once, as the original's new missions
            // do (FUN_00522980 sets the ready bit after each phase change).
            for mut order in orders {
                let mut draws = MissionRolls {
                    rolls: &[],
                    next: 0,
                    stream: crate::mission_detection::MissionRng::seeded(0.5, order.id),
                };
                Self::run(&mut order, &ctx, state, &mut out, now, &mut draws);
                if order.phase == PHASE_END {
                    Self::finish(&order, world, state, &mut out, now);
                } else {
                    state.missions.push_back(order);
                }
            }
        }
        out
    }

    /// Adrift characters (`+0x78` bit 6) settle or set out
    /// (`FUN_0054c7d0`, `FUN_00549230`): one free of any mission whose
    /// system stands and is its side's stays, and its Adrift clears; the
    /// others of one side at one system take the destination the autoroute
    /// search gives (`FUN_00543040`) and leave together on an Adrift order
    /// (`0x44000004`, `FUN_00547880`), whose detection gives the enemy its
    /// chance at them. Returns the orders, which step at once.
    ///
    /// port: a character with nowhere to go stays adrift; the original
    /// sends it to its side's jail (`FUN_00549580`). The order's members
    /// keep Adrift until they settle where it lands.
    fn send_adrift(
        world: &GameWorld,
        state: &mut MissionState,
        out: &mut MissionAdvance,
        now: u64,
    ) -> Vec<ActiveMission> {
        use crate::dat::Faction;
        let hidden = MissionKind::Adrift
            .record_id()
            .and_then(|id| world.mission_record(id))
            .is_some_and(|record| record.hidden);
        let mut groups: Vec<((bool, SystemKey, SystemKey), Vec<MissionMember>)> = Vec::new();
        for (key, c) in &world.characters {
            if !c.adrift || c.on_mission || c.is_killed || c.is_captive || c.current_fleet.is_some() {
                continue;
            }
            let Some(here) = c.current_system else {
                continue;
            };
            let side = if c.is_alliance {
                Faction::Alliance
            } else {
                Faction::Empire
            };
            let Some(sys) = world.systems.get(here) else {
                continue;
            };
            if !sys.is_destroyed && sys.control.faction() == Some(side) {
                out.effects
                    .push(MissionEffect::AdriftSettled { character: key });
                continue;
            }
            let Some(to) = nearest_place(
                world,
                side,
                (i64::from(sys.x), i64::from(sys.y)),
                Some(here),
            ) else {
                continue;
            };
            let member = MissionMember::Character(key);
            let group_key = (c.is_alliance, here, to);
            match groups.iter_mut().find(|(k, _)| *k == group_key) {
                Some((_, team)) => team.push(member),
                None => groups.push((group_key, vec![member])),
            }
        }
        groups
            .into_iter()
            .map(|((is_alliance, _, to), team)| {
                out.effects.extend(
                    team.iter()
                        .map(|&member| MissionEffect::MemberOnMission { member, hidden }),
                );
                let id = state.next_id;
                state.next_id += 1;
                let faction = if is_alliance {
                    MissionFaction::Alliance
                } else {
                    MissionFaction::Empire
                };
                ActiveMission::new(id, MissionKind::Adrift, faction, team, to, now)
            })
            .collect()
    }

    /// An ended mission's members leave (`FUN_00548120`, `FUN_00548910`):
    /// each one [`leaves_for_home`] goes on a Return order (`0x42000002`,
    /// `FUN_005475f0`) with the members sharing its destination, and stays
    /// on a mission; the others are released where they are. Returns the
    /// Return orders.
    ///
    /// port: the original sends a member with nowhere to go to its side's
    /// jail (`FUN_00549580`); the port releases it where it is. The Return
    /// order's Mandatory flag (the mission's `+0xa4` bit 2) is not kept.
    fn finish(
        mission: &ActiveMission,
        world: &GameWorld,
        state: &mut MissionState,
        out: &mut MissionAdvance,
        now: u64,
    ) -> Vec<ActiveMission> {
        // FUN_00548910 groups the members by the first one's destination
        // (FUN_00542ee0), in member order (FUN_00525bb0).
        let mut groups: Vec<(SystemKey, Vec<MissionMember>)> = Vec::new();
        for member in mission.members() {
            let Some(to) = return_destination(mission, world, member) else {
                continue;
            };
            match groups.iter_mut().find(|(dest, _)| *dest == to) {
                Some((_, group)) => group.push(member),
                None => groups.push((to, vec![member])),
            }
        }
        let going_home: Vec<MissionMember> = groups
            .iter()
            .flat_map(|(_, group)| group.iter().copied())
            .collect();
        out.effects.extend(
            mission
                .members()
                .filter(|member| !going_home.contains(member))
                .map(|member| MissionEffect::MemberAvailable { member }),
        );
        // FUN_00546370 -> slot +0xb0 (FUN_004ef260): a member
        // left in an abode of another side autoroutes home.
        if let Some(abode) = mission.target_abode {
            Self::leave_abode(mission, abode, world, state, out, now);
        }
        out.ended.push(MissionEnded {
            mission_id: mission.id,
            kind: mission.kind,
            faction: mission.faction,
            end_code: mission.end_code,
            tick: now,
        });
        let hidden = MissionKind::Return
            .record_id()
            .and_then(|id| world.mission_record(id))
            .is_some_and(|record| record.hidden);
        out.effects.extend(
            going_home
                .iter()
                .map(|&member| MissionEffect::MemberOnMission { member, hidden }),
        );
        groups
            .into_iter()
            .map(|(to, team)| {
                let id = state.next_id;
                state.next_id += 1;
                ActiveMission::new(id, MissionKind::Return, mission.faction, team, to, now)
            })
            .collect()
    }

    /// Set a waiting mission's ready bit or end code for day `now`.
    fn wake(mission: &mut ActiveMission, ctx: &StepContext<'_>, state: &MissionState, now: u64) {
        // FUN_00545240, once per destruction: a destroyed container or
        // target runs the validator; a destroyed container then ends the
        // mission with code 7 when its phase (+0x54 -> +0x1c, FUN_00520ac0)
        // is above 6 (FUN_00520ae0), past the members' landing. A killed
        // target's check may repeat: the validator either ends the mission
        // (a dead target has no location, rule 6) or skips every rule.
        let container_lost = !mission.container_loss_seen
            && mission.target_abode.is_none()
            && ctx
                .world
                .systems
                .get(mission.target_system)
                .is_some_and(|sys| sys.is_destroyed);
        let target_lost = mission.kind.target_rules() == TargetRules::Character
            && mission
                .target_character
                .and_then(|key| ctx.world.characters.get(key))
                .is_some_and(|c| c.is_killed);
        mission.container_loss_seen |= container_lost;
        if mission.end_code == 0 && (container_lost || target_lost) {
            mission.end_code = running_end_code(
                mission,
                ctx.world,
                ctx.uprisings,
                ctx.research,
                &state.en_route,
            );
            if mission.end_code == 0 && container_lost && mission.phase > 6 {
                mission.end_code = 7;
            }
        }
        match mission.phase {
            // FUN_00545820 -> FUN_00522280.
            PHASE_TRANSIT => {
                if mission.end_code == 0 && Self::transit_over(mission, ctx.world, state) {
                    mission.ready = true;
                }
            }
            // FUN_00522a10: timer 0x38b fired.
            PHASE_TIMER => {
                if mission.timer_due.is_some_and(|due| due <= now) {
                    mission.ready = true;
                }
            }
            _ => {}
        }
    }

    /// Whether phase 4's wait is over (`FUN_00522280.c:66-100`): it holds
    /// only while every member still travels, so a mission with no member,
    /// or with any member off the road, is ready. The original deletes a
    /// killed member, so a dead one is skipped: a team killed on the way
    /// counts as arrived.
    fn transit_over(mission: &ActiveMission, world: &GameWorld, state: &MissionState) -> bool {
        let (mut travelling, mut stopped) = (false, false);
        for member in mission.members() {
            if member_state(world, member).is_none() {
                continue;
            }
            if state.is_en_route(member) {
                travelling = true;
            } else {
                stopped = true;
            }
        }
        stopped || !travelling
    }

    /// The stepper `FUN_005227d0`, run until the mission waits or ends.
    fn run(
        mission: &mut ActiveMission,
        ctx: &StepContext<'_>,
        state: &mut MissionState,
        out: &mut MissionAdvance,
        now: u64,
        draws: &mut MissionRolls<'_>,
    ) {
        loop {
            if mission.phase == PHASE_END {
                return;
            }
            let next = if mission.end_code != 0 {
                PHASE_END
            } else if mission.ready {
                mission.ready = false;
                let repeats = mission
                    .kind
                    .record_id()
                    .and_then(|id| ctx.world.mission_record(id))
                    .is_some_and(|record| record.repeats);
                if mission.phase == PHASE_OUTCOME && repeats {
                    PHASE_TIMER
                } else {
                    mission.phase + 1
                }
            } else {
                return;
            };
            let previous = mission.phase;
            mission.phase = next;
            Self::enter(mission, previous, ctx, state, out, now, draws);
            // FUN_00546ea0 runs the detection manager after the change.
            crate::mission_detection::run(
                mission,
                ctx.world,
                ctx.uprisings,
                ctx.research,
                &state.en_route,
                now,
                &mut draws.stream,
                &mut out.effects,
            );
            // FUN_00522980: every phase but 4 and 8 steps on at once.
            if !matches!(mission.phase, PHASE_TRANSIT | PHASE_TIMER) {
                mission.ready = true;
            }
        }
    }

    /// `FUN_00546370` -> slot `+0xb0` (`FUN_004ef260`) when a mission at an
    /// abode ends: each character member in the abode, of another side than
    /// the abode's, autoroutes (`FUN_00543040`) and leaves after 1 day
    /// (`FUN_00555b30`).
    ///
    /// A prisoner the mission freed there (`FUN_004ef510`: state 3, whose
    /// change sets Autorouting, `FUN_004f18e0`) autoroutes out the same way.
    ///
    /// port: the autorouting origin (`FUN_005431e0`'s `+0x74`) is the
    /// system the members left from.
    fn leave_abode(
        mission: &ActiveMission,
        abode: crate::ids::AbodeKey,
        world: &GameWorld,
        state: &mut MissionState,
        out: &mut MissionAdvance,
        now: u64,
    ) {
        let Some(place) = world.abodes.get(abode) else {
            return;
        };
        let origin = mission
            .origin
            .and_then(|key| world.systems.get(key))
            .map_or((0, 0), |s| (i64::from(s.x), i64::from(s.y)));
        let own_effects = || {
            out.results
                .iter()
                .filter(|result| result.mission_id == mission.id)
                .flat_map(|result| &result.effects)
        };
        let freed = own_effects()
            .filter_map(|effect| match effect {
                MissionEffect::CharacterRescued { character, .. } => {
                    Some(MissionMember::Character(*character))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        // A picked-up prisoner is a member of another side than the
        // palace's (its own side +0x24): FUN_004ef260 runs its slot +0x218
        // (FUN_004ef5b0, status 2) and sets Autorouting, and the
        // autorouting's first fitting candidate is its +0x70
        // (FUN_00554ab0), Vader's. port: Vader's +0x70 is the system he
        // left from.
        if let Some(home) = mission
            .origin
            .filter(|_| mission.kind == MissionKind::Pickup)
        {
            for effect in own_effects() {
                if let MissionEffect::PrisonerPickedUp { character } = *effect {
                    out.effects.push(MissionEffect::MemberMoved {
                        member: MissionMember::Character(character),
                        to: None,
                    });
                    state.escapees.push(EscapeeTransit {
                        character,
                        to: home,
                        arrival: now + 1,
                        abode: None,
                    });
                }
            }
        }
        for member in mission.members().chain(freed.iter().copied()) {
            let MissionMember::Character(key) = member else {
                continue;
            };
            let Some(c) = world.characters.get(key) else {
                continue;
            };
            let own = if c.is_alliance {
                crate::dat::Faction::Alliance
            } else if c.is_empire {
                crate::dat::Faction::Empire
            } else {
                crate::dat::Faction::Neutral
            };
            // A prisoner taken this step, or held and not freed this step,
            // stays where its captor's abode holds it. hyp: FUN_004ef260
            // compares its own side +0x24 and autoroutes it (slot +0x218,
            // status 2), but a prisoner's autorouting leads to its captor,
            // the abode it is in; the port reads the captor's side
            // (FUN_004eec60) instead.
            let taken = own_effects().find_map(|effect| match effect {
                MissionEffect::CharacterCaptured {
                    character,
                    captured_by,
                    ..
                } if *character == key => Some(crate::dat::Faction::from(*captured_by)),
                _ => None,
            });
            let held = c
                .captured_by
                .filter(|_| c.is_captive && !freed.contains(&member));
            let side = taken.or(held).unwrap_or(own);
            let arrived = out.effects.iter().any(|effect| {
                matches!(effect, MissionEffect::MemberEnteredAbode { member: m, .. } if *m == member)
            });
            if c.is_killed || place.side == side || (c.current_abode != Some(abode) && !arrived) {
                continue;
            }
            let Some(to) = autoroute_destination(world, key, origin, None) else {
                continue;
            };
            out.effects
                .push(MissionEffect::MemberMoved { member, to: None });
            state.escapees.push(EscapeeTransit {
                character: key,
                to,
                arrival: now + 1,
                abode: None,
            });
        }
    }

    /// The phase handler `FUN_00524b70`: leave `previous`, enter the
    /// mission's new phase.
    fn enter(
        mission: &mut ActiveMission,
        previous: u8,
        ctx: &StepContext<'_>,
        state: &mut MissionState,
        out: &mut MissionAdvance,
        now: u64,
        draws: &mut MissionRolls<'_>,
    ) {
        let world = ctx.world;
        let validate = |mission: &mut ActiveMission, state: &MissionState| {
            if mission.end_code == 0 {
                mission.end_code =
                    running_end_code(mission, world, ctx.uprisings, ctx.research, &state.en_route);
            }
        };
        // Leaving phase 8 runs the validator, then disarms timer 0x38b.
        if previous == PHASE_TIMER {
            validate(mission, state);
            mission.timer_due = None;
        }
        match mission.phase {
            2 => {
                // FUN_00522870 settles the team's location (+0x6c).
                let origin = mission
                    .members()
                    .find_map(|member| member_location(world, member));
                mission.origin = origin;
                mission.origin_abode = if origin.is_none() {
                    mission
                        .members()
                        .find_map(|member| member_abode(world, member))
                } else {
                    None
                };
                mission.speed = Self::character_speed(mission, world);
                // FUN_00571410 (Jedi Training's slot +0x1f4) in phase 2: the
                // teacher +0xa8 is the first member who may teach.
                if mission.kind == MissionKind::JediTraining {
                    mission.teacher = jedi_teacher(world, &mission.team);
                }
            }
            PHASE_TRANSIT => {
                validate(mission, state);
                if mission.end_code == 0 {
                    Self::depart(mission, world, state, out, now);
                }
                mission.wait_start = now;
                // FUN_00522280.
                if mission.end_code == 0 && Self::transit_over(mission, ctx.world, state) {
                    mission.ready = true;
                }
            }
            // Phases 5 and 6 act only away from the origin (FUN_00520bb0);
            // phase 6 lands the members, which the arrivals already did.
            5 | 6 => {
                if mission.origin != Some(mission.target_system) {
                    validate(mission, state);
                }
            }
            PHASE_TIMER => {
                let delay = match mission
                    .kind
                    .record_id()
                    .and_then(|id| world.mission_record(id))
                {
                    Some(record) => crate::uprising::one_roll_timer_delay(
                        i32::try_from(record.timer_min_days).unwrap_or(i32::MAX),
                        i32::try_from(record.timer_spread_days).unwrap_or(i32::MAX),
                        draws.next(),
                    ),
                    // port: a kind with no record keeps the port's range.
                    None => {
                        let (min, max) = mission.kind.tick_range();
                        crate::uprising::one_roll_timer_delay(
                            i32::try_from(min).unwrap_or(i32::MAX),
                            i32::try_from(max - min).unwrap_or(i32::MAX),
                            draws.next(),
                        )
                    }
                };
                mission.wait_start = now;
                mission.timer_due = Some(now + delay);
                // FUN_005751d0 in phase 8: FirstTrainingDay +0xa8 = today.
                if mission.kind == MissionKind::Dagobah {
                    mission.first_training_day = Some(now);
                }
            }
            // The Move class's base handler FUN_00524b70 runs nothing here.
            PHASE_OUTCOME
                if matches!(
                    mission.kind,
                    MissionKind::Move | MissionKind::Return | MissionKind::Adrift
                ) => {}
            PHASE_OUTCOME => {
                let result = Self::roll_members(mission, ctx, now, &mut draws.stream, &out.results);
                mission.succeeded = result.outcome == MissionOutcome::Success;
                out.results.push(result);
            }
            PHASE_END => {
                validate(mission, state);
                if mission.end_code == 0 {
                    mission.end_code = 1;
                }
                // FUN_005751d0 in phase 0xb, after phase 8 (FUN_00520ac0).
                // hyp: FUN_00520ac0 reads the phase the mission leaves.
                if mission.kind == MissionKind::Dagobah && previous >= PHASE_TIMER {
                    out.effects.extend(crate::dagobah::training_end(
                        world,
                        mission.first_training_day,
                        mission.succeeded,
                        now,
                    ));
                }
                // Slot +0x284 (FUN_00592c80) raises observation level 6 at
                // the target for the mission's side. port: left to the
                // detection work of F-019 phase 3.
            }
            _ => {}
        }
    }

    /// The speed `FUN_00548370` gives a mission's characters: GNPRTB 3083
    /// when Han Solo is a free member and no special force is, else the
    /// GNPRTB 1 default.
    fn character_speed(mission: &ActiveMission, world: &GameWorld) -> i64 {
        let han_free = mission.members().any(|member| {
            member
                .character()
                .and_then(|key| world.characters.get(key))
                .is_some_and(|c| c.dat_id.raw() == HAN_SOLO && !c.is_captive)
        });
        let has_special_force = mission
            .members()
            .any(|member| matches!(member, MissionMember::SpecialForce(_)));
        if han_free && !has_special_force {
            crate::movement::gnprtb_or_shipped(world, GNPRTB_HAN_SOLO_SPEED, SHIPPED_HAN_SOLO_SPEED)
        } else {
            crate::movement::default_speed(world)
        }
    }

    /// Phase 4's departures (`FUN_00556430`, `FUN_00556390`): every member
    /// travels from the origin to the target system and stays there. A
    /// member already there lands at once. With no origin, `FUN_00556430`
    /// (`:31-33`) starts no transit and no one moves.
    ///
    /// Every member travels at the speed phase 2 set: a special force
    /// travels at the default (`build-delivery.md`), and any special force
    /// member already holds the characters to it (`FUN_00548370`).
    fn depart(
        mission: &ActiveMission,
        world: &GameWorld,
        state: &mut MissionState,
        out: &mut MissionAdvance,
        now: u64,
    ) {
        let to = mission.target_system;
        let Some(from) = mission.origin else {
            // FUN_00555b30: a trip from an abode takes 1 day.
            if mission.origin_abode.is_some() {
                for member in mission.members() {
                    if member_state(world, member).is_none() {
                        continue;
                    }
                    out.effects
                        .push(MissionEffect::MemberMoved { member, to: None });
                    state.en_route.push(MemberTransit {
                        member,
                        mission_id: mission.id,
                        to,
                        arrival: now + 1,
                        abode: mission.target_abode,
                    });
                }
            }
            return;
        };
        let position = |key: SystemKey| world.systems.get(key).map_or((0, 0), |s| (s.x, s.y));
        for member in mission.members() {
            if member_state(world, member).is_none() {
                continue;
            }
            // FUN_00555b30: a trip to an abode takes 1 day.
            if let Some(abode) = mission.target_abode {
                out.effects
                    .push(MissionEffect::MemberMoved { member, to: None });
                state.en_route.push(MemberTransit {
                    member,
                    mission_id: mission.id,
                    to,
                    arrival: now + 1,
                    abode: Some(abode),
                });
                continue;
            }
            let days = crate::movement::transit_ticks_between(
                world,
                position(from),
                position(to),
                mission.speed,
            );
            if days == 0 {
                out.effects
                    .extend(landing(world, Some(mission), member, to, None));
            } else {
                // port: a travelling member is off the map, as a delivery is.
                out.effects
                    .push(MissionEffect::MemberMoved { member, to: None });
                state.en_route.push(MemberTransit {
                    member,
                    mission_id: mission.id,
                    to,
                    arrival: now + u64::from(days),
                    abode: None,
                });
            }
        }
    }

    /// Phase 10 (`FUN_00592f50`): each team character, then each team
    /// special force (`FUN_00526090`, `FUN_00525e70`), draws `0..99` below
    /// its chance (slot `+0x274`, [`member_chance`]) and acts on a success
    /// (slot `+0x27c`); the class's slot `+0x280` then applies the result
    /// `+0x60` once (`ghidra/notes/mission-lifecycle.md`, "Outcomes").
    /// Decoys and captives never roll. port: the draws come from the
    /// mission's seeded stream, and a recruit in `earlier`, this step's
    /// results so far, has left the pool although the world applies it later.
    fn roll_members(
        mission: &ActiveMission,
        ctx: &StepContext<'_>,
        now: u64,
        stream: &mut crate::mission_detection::MissionRng,
        earlier: &[MissionResult],
    ) -> MissionResult {
        use crate::uprising::Draws;
        let world = ctx.world;
        let side = crate::dat::Faction::from(mission.faction);
        let target = mission.target_system;
        let characters = mission
            .team
            .iter()
            .filter(|member| member.character().is_some());
        let special_forces = mission
            .team
            .iter()
            .filter(|member| member.character().is_none());
        let members: Vec<MissionMember> = characters
            .chain(special_forces)
            .copied()
            .filter(|&member| member_state(world, member).is_some())
            .collect();
        let mut effects = Vec::new();
        // +0x60: 0 unset, 2 failed, 3 succeeded (FUN_00521880).
        let mut result = 0_u8;
        // Research's points this pass (+0xa8, zeroed by FUN_0056cd70).
        let mut research_points = 0_i32;
        // The target of an in-roll action, once it has been acted on.
        let mut target_taken = false;
        // Subdue's view of the target as the successes change it.
        let mut system = world.systems.get(target).cloned();
        let mut revolting = ctx.uprisings.is_uprising(target);
        // FUN_0055ef30: the side's characters Recruitment may still pick.
        let mut pool = if mission.kind == MissionKind::Recruitment {
            let taken: Vec<CharacterKey> = earlier
                .iter()
                .flat_map(|result| &result.effects)
                .filter_map(|effect| match effect {
                    MissionEffect::CharacterRecruited { character, .. } => Some(*character),
                    _ => None,
                })
                .collect();
            recruit_pool(world, side)
                .into_iter()
                .filter(|character| !taken.contains(character))
                .collect()
        } else {
            Vec::new()
        };
        for member in members {
            if mission.kind == MissionKind::SubdueUprising && !(system.is_some() && revolting) {
                // FUN_00569c20 needs the target in an uprising (+0x88 bit 2).
                continue;
            }
            if mission.kind == MissionKind::InciteUprising && system.is_none() {
                continue;
            }
            let chance = match mission.kind.research_tech() {
                Some(tech) => research_chance(world, member, tech),
                None if mission.kind == MissionKind::JediTraining => {
                    jedi_training_chance(world, mission.teacher, member)
                }
                None => member_chance(
                    world,
                    mission.kind,
                    mission.faction,
                    target,
                    mission.target_character,
                    member,
                ),
            };
            if !stream.chance(chance) {
                continue;
            }
            // Slot +0x1d8 holds for a character (FUN_0040f340) and not for a
            // special force (FUN_006158b0).
            let raise = |skill: Skill, id: u16| {
                member
                    .character()
                    .map(|character| MissionEffect::SkillRaised {
                        character,
                        skill,
                        amount: skill_raise(world, id),
                    })
            };
            match mission.kind {
                // FUN_005740a0.
                MissionKind::Diplomacy => {
                    result = 3;
                    effects.extend(raise(Skill::Diplomacy, GNPRTB_DIPLOMACY_RAISE));
                }
                // FUN_00573540: the raise needs a target (FUN_00521070) of
                // another side.
                MissionKind::Espionage => {
                    result = 3;
                    let foreign = world
                        .systems
                        .get(target)
                        .is_some_and(|sys| crate::uprising::holder(sys) != Some(side));
                    if foreign {
                        effects.extend(raise(Skill::Espionage, GNPRTB_ESPIONAGE_RAISE));
                    }
                }
                // FUN_0056b9a0: at the target system (FUN_00586720), a
                // random pool character (FUN_0055fc80, draw FUN_0053e290)
                // joins there (FUN_0055fe70); then 3 and the leadership
                // raise. An empty pool leaves the result alone.
                MissionKind::Recruitment => {
                    let pick = if world.systems.contains_key(target) {
                        stream.pick(pool.len())
                    } else {
                        None
                    };
                    if let Some(index) = pick {
                        let character = pool.remove(index);
                        effects.push(MissionEffect::CharacterRecruited {
                            character,
                            system: target,
                            faction: mission.faction,
                            pool_emptied: pool.is_empty(),
                        });
                        result = 3;
                        effects.extend(raise(Skill::Leadership, GNPRTB_RECRUITMENT_RAISE));
                    }
                }
                // FUN_0056ae50: the target's slot +0x210 frees it.
                MissionKind::Rescue => {
                    result = 3;
                    effects.extend(raise(Skill::Combat, GNPRTB_RESCUE_RAISE));
                    if let Some(character) = mission.target_character.filter(|_| !target_taken) {
                        target_taken = true;
                        effects.push(MissionEffect::CharacterRescued {
                            character,
                            returned_to: mission.faction,
                            at_system: target,
                        });
                    }
                }
                // FUN_00575de0: the target of family 0x33 (Han) is
                // attacked (BountyAttack, notification 0x212) and rolls
                // RLEVADTB against the bounty hunters (FUN_005349e0, captor
                // combat 0): he evades (slot +0x208, the injury roll) or is
                // captured (slot +0x20c, the injury roll first). A captured
                // Han loses CanEscape and gets CapturedByBountyHunters, and
                // the result is 3; otherwise 2.
                // port: the evade's resign request from Han's own mission
                // (FUN_00534c20) is not ported.
                MissionKind::Bounty => {
                    let han = mission.target_character.filter(|&key| {
                        !target_taken
                            && world
                                .characters
                                .get(key)
                                .is_some_and(|c| c.dat_id.index() == HAN_SOLO & 0x00ff_ffff)
                    });
                    if let Some(han) = han {
                        target_taken = true;
                        let combat = skill_of(world, han, Skill::Combat);
                        let evades =
                            crate::mission_detection::rlevadtb_evades(world, combat, stream);
                        let captured = evades == Some(false);
                        if evades.is_some() {
                            if let Some(injury) =
                                crate::mission_detection::injury_roll(world, combat, stream)
                            {
                                effects.push(MissionEffect::CharacterInjured {
                                    character: han,
                                    injury,
                                });
                            }
                        }
                        effects.push(MissionEffect::BountyAttack {
                            character: han,
                            captured,
                        });
                        if captured {
                            effects.push(MissionEffect::CharacterCaptured {
                                character: han,
                                captured_by: mission.faction,
                                at_system: target,
                            });
                            effects.push(MissionEffect::CapturedByBountyHunters { character: han });
                            result = 3;
                        } else {
                            result = 2;
                        }
                    }
                }
                // FUN_0056e650: 3, the member's base combat and espionage
                // rise by GNPRTB 6170 and 6169 (slot +0x1d8), the member
                // frees the target (slot +0x210, FUN_004ef510), then every
                // other prisoner in the target's container (FUN_00521050),
                // the palace. port: a freed prisoner keeps its own side.
                MissionKind::Palace => {
                    result = 3;
                    effects.extend(raise(Skill::Combat, GNPRTB_PALACE_COMBAT_RAISE));
                    effects.extend(raise(Skill::Espionage, GNPRTB_PALACE_ESPIONAGE_RAISE));
                    if !target_taken {
                        target_taken = true;
                        let freed = mission.target_character.into_iter().chain(
                            mission.target_abode.into_iter().flat_map(|abode| {
                                world
                                    .characters
                                    .iter()
                                    .filter(move |(_, c)| {
                                        c.is_captive
                                            && !c.is_killed
                                            && c.current_abode == Some(abode)
                                    })
                                    .map(|(key, _)| key)
                                    .filter(|&key| Some(key) != mission.target_character)
                            }),
                        );
                        for character in freed {
                            let own = if world
                                .characters
                                .get(character)
                                .is_some_and(|c| c.is_empire && !c.is_alliance)
                            {
                                MissionFaction::Empire
                            } else {
                                mission.faction
                            };
                            effects.push(MissionEffect::CharacterRescued {
                                character,
                                returned_to: own,
                                at_system: target,
                            });
                        }
                    }
                }
                // FUN_0056da80: PickupInProgress set; each prisoner of the
                // palace of the mission's side (FUN_004f2640: a prisoner's
                // side is its captor's) on no mission joins the mission
                // (slot +0x1cc) and takes the member's +0x70
                // (FUN_00535760 on the member, FUN_00534390), and the
                // result is 3; PickupInProgress cleared.
                MissionKind::Pickup => {
                    if let Some(abode) = mission.target_abode.filter(|_| !target_taken) {
                        target_taken = true;
                        effects.push(MissionEffect::PickupInProgress);
                        for (character, c) in &world.characters {
                            if c.current_abode == Some(abode)
                                && c.is_captive
                                && !c.is_killed
                                && !c.on_mission
                                && c.captured_by == Some(side)
                            {
                                effects.push(MissionEffect::PrisonerPickedUp { character });
                                result = 3;
                            }
                        }
                    }
                }
                // FUN_00576eb0: the target's slot +0x20c takes it prisoner.
                MissionKind::Abduction => {
                    result = 3;
                    effects.extend(raise(Skill::Combat, GNPRTB_ABDUCTION_RAISE));
                    if let Some(character) = mission.target_character.filter(|_| !target_taken) {
                        target_taken = true;
                        effects.push(MissionEffect::CharacterCaptured {
                            character,
                            captured_by: mission.faction,
                            at_system: target,
                        });
                    }
                }
                // FUN_00576620: each successful member injures the target
                // through its slot +0x2ec (FUN_004ef690: the injury of
                // FUN_0053eae0, then slot +0x2f0 with reason 7, which
                // destroys it when slot +0x2e0 says the injury kills). The
                // result 3 and the raise need it destroyed (+0x50 bit 3),
                // which only a minor character can be.
                MissionKind::Assassination => {
                    if let Some(character) = mission.target_character {
                        if !target_taken {
                            if let Some(injury) =
                                crate::mission_detection::assassination_injury(world, stream)
                            {
                                effects.push(MissionEffect::CharacterInjured { character, injury });
                                if crate::mission_detection::dies_of_injury(
                                    world, character, stream,
                                ) {
                                    target_taken = true;
                                    effects.push(MissionEffect::CharacterKilled {
                                        character,
                                        faction: mission.faction,
                                    });
                                }
                            }
                        }
                        if target_taken {
                            result = 3;
                            effects.extend(raise(Skill::Combat, GNPRTB_ASSASSINATION_RAISE));
                        }
                    }
                }
                // FUN_00571a60: the uprising incident, then 2 while the
                // holder keeps a regiment there (FUN_00509020), else 3.
                // port: its other test, the system's +0x84 bits 2..3 against
                // the opponent, reads a field no recovered function writes.
                MissionKind::InciteUprising => {
                    let event =
                        crate::uprising::incite_incident(world, ctx.missions, target, now, stream);
                    let lost: Vec<crate::ids::TroopKey> = event
                        .iter()
                        .flat_map(|event| match event {
                            crate::uprising::UprisingEvent::UprisingIncident { losses, .. } => {
                                losses.clone()
                            }
                            _ => Vec::new(),
                        })
                        .filter_map(|loss| match loss {
                            crate::uprising::IncidentLoss::Regiment(troop) => Some(troop),
                            _ => None,
                        })
                        .collect();
                    let garrisoned = world.systems.get(target).is_some_and(|sys| {
                        crate::uprising::holder(sys).is_some_and(|holder| {
                            sys.ground_units.iter().any(|key| {
                                !lost.contains(key)
                                    && world.troops.get(*key).is_some_and(|troop| {
                                        crate::mission_detection::faction_of(troop.is_alliance)
                                            == holder
                                    })
                            })
                        })
                    });
                    effects.extend(event.map(MissionEffect::UprisingIncident));
                    result = if garrisoned { 2 } else { 3 };
                    if result == 3 {
                        effects.extend(raise(Skill::Leadership, GNPRTB_INCITE_RAISE));
                    }
                }
                // FUN_00569c20: support (FUN_0055cb10, FUN_0050c9f0), then the
                // end check (FUN_0050c910); 3 once the uprising ended, else 2.
                MissionKind::SubdueUprising => {
                    if let Some(sys) = system.as_mut() {
                        let gain = crate::uprising::mission_support_gain(
                            world,
                            sys,
                            side,
                            crate::uprising::SupportGain::Subdue,
                            stream,
                        );
                        *sys = crate::uprising::with_support_change(world, sys, side, gain);
                        effects.push(MissionEffect::UprisingSubdued {
                            system: target,
                            side,
                            support_gain: gain,
                        });
                        if crate::uprising::garrison_covers(world, sys) {
                            revolting = false;
                        }
                    }
                    result = if revolting { 2 } else { 3 };
                    if result == 3 {
                        effects.extend(raise(Skill::Leadership, GNPRTB_SUBDUE_RAISE));
                    }
                }
                // FUN_0056a300 and FUN_00574630.
                MissionKind::Sabotage => {
                    result = 3;
                    effects.extend(raise(Skill::Espionage, GNPRTB_SABOTAGE_ESPIONAGE_RAISE));
                    effects.extend(raise(Skill::Combat, GNPRTB_SABOTAGE_COMBAT_RAISE));
                }
                MissionKind::DeathStarSabotage => {
                    result = 3;
                    effects.extend(raise(Skill::Espionage, GNPRTB_DS_SABOTAGE_ESPIONAGE_RAISE));
                    effects.extend(raise(Skill::Combat, GNPRTB_DS_SABOTAGE_COMBAT_RAISE));
                }
                MissionKind::Autoscrap => result = 3,
                // Never reached: phase 10 skips Move (FUN_00524b70).
                MissionKind::Move | MissionKind::Return | MissionKind::Adrift => {}
                // FUN_0056bec0: 3, with no skill raise.
                MissionKind::Reconnaissance => result = 3,
                // FUN_00572470: 3.
                MissionKind::Dagobah => result = 3,
                // FUN_005712b0: a character pupil's Force becomes Force +
                // rand(0..=(teacher - pupil) x GNPRTB 6173 / 100)
                // (FUN_0055ca20); the gain goes to ForceTraining +0x90
                // (FUN_004ee1d0) and sets 3 when it is not 0.
                MissionKind::JediTraining => {
                    let pupil = member.character().and_then(|key| world.characters.get(key));
                    let teacher = mission.teacher.and_then(|key| world.characters.get(key));
                    if let (Some(character), Some(pupil), Some(teacher)) =
                        (member.character(), pupil, teacher)
                    {
                        let gap = i64::from(teacher.force_level()) - i64::from(pupil.force_level());
                        if gap > 0 {
                            let share = crate::movement::gnprtb_or_shipped(
                                world,
                                GNPRTB_JEDI_TRAINING_SHARE,
                                SHIPPED_JEDI_TRAINING_SHARE,
                            );
                            let spread = i32::try_from(gap * share / 100).unwrap_or(i32::MAX);
                            let amount = stream.draw(spread);
                            if amount != 0 {
                                result = 3;
                                effects
                                    .push(MissionEffect::ForceTrainingGained { character, amount });
                            }
                        }
                    }
                }
                // FUN_0056cdb0: GNPRTB 6171 + rand(0..=6172) points
                // (FUN_0055c9f0) join the mission's (SetRdPoints +0xa8); then
                // the member's base skill in the tree rises by GNPRTB 6158
                // (FUN_00534990).
                MissionKind::ShipDesignResearch
                | MissionKind::FacilityDesignResearch
                | MissionKind::TroopTrainingResearch => {
                    let gnprtb = |id, shipped| {
                        i32::try_from(crate::movement::gnprtb_or_shipped(world, id, shipped))
                            .unwrap_or(0)
                    };
                    let points = gnprtb(GNPRTB_RESEARCH_POINTS_MIN, SHIPPED_RESEARCH_POINTS_MIN)
                        + stream.draw(gnprtb(
                            GNPRTB_RESEARCH_POINTS_SPREAD,
                            SHIPPED_RESEARCH_POINTS_SPREAD,
                        ));
                    if points != 0 {
                        research_points += points;
                        if let Some(tech) = mission.kind.research_tech() {
                            effects.extend(raise(tech.skill(), GNPRTB_RESEARCH_RAISE));
                        }
                    }
                }
            }
        }
        // Slot +0x280: an unset result fails (FUN_00576700). Research's
        // FUN_0056cf30 adds the mission's points to its side's tree
        // (FUN_00530600) and sets 2 + (points != 0).
        if let Some(tech) = mission.kind.research_tech() {
            result = if research_points == 0 { 2 } else { 3 };
            if let Ok(points @ 1..) = u32::try_from(research_points) {
                effects.push(MissionEffect::ResearchPoints {
                    faction: mission.faction,
                    tech,
                    points,
                });
            }
        }
        // FUN_0056e870 (the Palace's slot +0x280): with no success, every
        // member is taken prisoner with the mission's captor key (slot
        // +0x20c) and loses CanEscape. port: the captor is the palace's
        // side.
        if result == 0 && mission.kind == MissionKind::Palace {
            let captor = mission
                .target_abode
                .and_then(|abode| world.abodes.get(abode))
                .map_or(crate::dat::Faction::Empire, |abode| abode.side);
            let captor = if captor == crate::dat::Faction::Alliance {
                MissionFaction::Alliance
            } else {
                MissionFaction::Empire
            };
            for &member in &mission.team {
                if member_state(world, member).is_none() {
                    continue;
                }
                if let Some(character) = member.character() {
                    effects.push(MissionEffect::CharacterCaptured {
                        character,
                        captured_by: captor,
                        at_system: target,
                    });
                    effects.push(MissionEffect::EscapeBarred { character });
                }
            }
        }
        if result == 0 {
            result = 2;
        }
        if result == 3 {
            match mission.kind {
                // FUN_00574120: FUN_0055cac0 support through FUN_0050c9f0.
                MissionKind::Diplomacy => {
                    if let Some(sys) = world.systems.get(target) {
                        let points = crate::uprising::mission_support_gain(
                            world,
                            sys,
                            side,
                            crate::uprising::SupportGain::Diplomacy,
                            stream,
                        );
                        if points != 0 {
                            effects.push(MissionEffect::SupportGained {
                                system: target,
                                side,
                                points,
                            });
                        }
                    }
                }
                // FUN_00573610: observation level 10 at a system target
                // (FUN_0050d5a0). port: the port reveals the system; the
                // counts of FUN_0055c940 are not ported.
                MissionKind::Espionage => {
                    if world.systems.contains_key(target) {
                        effects.push(MissionEffect::SystemIntelligenceGathered {
                            system: target,
                            faction: mission.faction,
                        });
                    }
                }
                // FUN_0056bee0: observation level 8 for the mission's side
                // at a system target (Event319_Handle(8, side)). port: the
                // port reveals the system, as for Espionage's level 10.
                MissionKind::Reconnaissance => {
                    if world.systems.contains_key(target) {
                        effects.push(MissionEffect::SystemIntelligenceGathered {
                            system: target,
                            faction: mission.faction,
                        });
                    }
                }
                // FUN_005746e0: slot +0xac(6) destroys the order's object
                // (FUN_00521030).
                MissionKind::Sabotage | MissionKind::DeathStarSabotage => {
                    effects.extend(
                        mission
                            .target_object
                            .filter(|object| object.state(world).is_some())
                            .map(|object| MissionEffect::TargetSabotaged {
                                target: object,
                                system: target,
                            }),
                    );
                }
                _ => {}
            }
        }
        MissionResult {
            mission_id: mission.id,
            tick: now,
            kind: mission.kind,
            faction: mission.faction,
            character: mission.lead_character(),
            target_system: target,
            outcome: if result == 3 {
                MissionOutcome::Success
            } else {
                MissionOutcome::Failure
            },
            effects,
        }
    }

    /// Captives escaping on their own (`ghidra/notes/captive-escape.md`).
    ///
    /// Capture sets CanEscape, which arms timer `0x38a` for GNPRTB 2584 +
    /// rand(0..=2585) days (`FUN_004f1a80`, `FUN_004f1ba0`, `FUN_004ef750`).
    /// When it fires the captive rolls (`FUN_0057d330` -> event `0x317` ->
    /// `FUN_00561990`) and the timer re-arms with a fresh draw
    /// (`FUN_005862a0`). A hit frees the captive (`FUN_004ef570`), which
    /// restores its home side and sets Autorouting (`FUN_004f18e0`): it leaves
    /// a location another side holds for [`escape_destination`] and travels
    /// there (`FUN_0054c7d0`). Escapees whose trip ends arrive first.
    ///
    /// Each attempt draws the roll, then the next delay; arming draws the
    /// delay. port: the timer arms on the first advance that sees the captive,
    /// which is the day of its capture when the caller runs missions first, and
    /// a timer whose captive is no longer held is dropped then.
    pub fn advance_escapes(
        state: &mut MissionState,
        world: &GameWorld,
        now: u64,
        rolls: &[f64],
    ) -> Vec<MissionEffect> {
        let mut effects = Vec::new();
        let (arrived, travelling): (Vec<_>, Vec<_>) = std::mem::take(&mut state.escapees)
            .into_iter()
            .partition(|transit| transit.arrival <= now);
        state.escapees = travelling;
        effects.extend(arrived.into_iter().map(|transit| match transit.abode {
            Some(abode) => MissionEffect::MemberEnteredAbode {
                member: MissionMember::Character(transit.character),
                abode,
            },
            None => MissionEffect::MemberMoved {
                member: MissionMember::Character(transit.character),
                to: Some(transit.to),
            },
        }));

        // CanEscape (+0xac bit 0x100) follows Captured (FUN_004f1a80); the
        // Final Battle clears it while the character stays held
        // (FUN_004eeb10(0)).
        let held = |key: CharacterKey| {
            world
                .characters
                .get(key)
                .is_some_and(|c| c.is_captive && !c.is_killed && !c.escape_barred)
        };
        state.escape_timers.retain(|timer| held(timer.character));

        let param = |id| world.gnprtb.value(id, world.difficulty_index);
        let delay = |rolls: &mut crate::uprising::Rolls<'_>| {
            crate::uprising::timer_delay(
                param(GNPRTB_ESCAPE_MIN),
                param(GNPRTB_ESCAPE_SPREAD),
                rolls,
            )
        };
        let empty = crate::world::MstbTable::new(Vec::new());
        let escapetb = world.mission_tables.get("ESCAPETB").unwrap_or(&empty);
        let mut rolls = crate::uprising::Rolls::new(rolls);
        for (key, _) in world.characters.iter().filter(|(key, _)| held(*key)) {
            let Some(index) = state.escape_timers.iter().position(|t| t.character == key) else {
                let due = now + delay(&mut rolls);
                state.escape_timers.push(EscapeTimer {
                    character: key,
                    due,
                });
                continue;
            };
            if state.escape_timers[index].due > now {
                continue;
            }
            let chance = escapetb.step_lookup(escape_input(world, key));
            let hit = crate::uprising::Draws::chance(
                &mut rolls,
                i32::try_from(chance).unwrap_or(i32::MAX),
            );
            state.escape_timers[index].due = now + delay(&mut rolls);
            if !hit {
                continue;
            }
            state.escape_timers.remove(index);
            let character = &world.characters[key];
            effects.push(MissionEffect::CharacterEscaped {
                character: key,
                escaped_to_alliance: character.is_alliance,
            });
            let Some(from) = character.current_system else {
                continue;
            };
            let Some(to) = escape_destination(world, key, from) else {
                continue;
            };
            let position = |key: SystemKey| world.systems.get(key).map_or((0, 0), |s| (s.x, s.y));
            // port: a character travels at the GNPRTB 1 default speed.
            let days = crate::movement::transit_ticks_between(
                world,
                position(from),
                position(to),
                crate::movement::default_speed(world),
            );
            if days == 0 {
                effects.push(MissionEffect::MemberMoved {
                    member: MissionMember::Character(key),
                    to: Some(to),
                });
            } else {
                // port: a travelling character is off the map, as a mission
                // member is.
                effects.push(MissionEffect::MemberMoved {
                    member: MissionMember::Character(key),
                    to: None,
                });
                state.escapees.push(EscapeeTransit {
                    character: key,
                    to,
                    arrival: now + u64::from(days),
                    abode: None,
                });
            }
        }
        effects
    }
}

/// `FUN_00561990` -> `FUN_0055e7e0`: the ESCAPETB input of `captive`,
/// `(espionage + combat) - guards - regiments`. The guards term is the
/// integer average combat of the characters at the captive's system that
/// are usable, of the captor's side and not themselves held (slot `+0x1d4`,
/// Captured); the regiments term counts the captor's regiments there
/// (`FUN_00504cc0`). A captive carries its captor's side (`FUN_004eec60`),
/// so both terms are the captor's. Without a location both are 0.
fn escape_input(world: &GameWorld, captive: CharacterKey) -> i32 {
    let own = skill_of(world, captive, Skill::Espionage) + skill_of(world, captive, Skill::Combat);
    let character = &world.characters[captive];
    let (Some(system), Some(captor)) = (character.current_system, character.captured_by) else {
        return own;
    };
    let Some(sys) = world.systems.get(system) else {
        return own;
    };
    let (sum, count) = world
        .characters
        .iter()
        .filter(|(_, c)| {
            crate::uprising::usable_at(c, system)
                && !c.is_captive
                && crate::uprising::sides_match(c.is_alliance, c.is_empire, captor)
        })
        .fold((0, 0), |(sum, count), (key, _)| {
            (sum + skill_of(world, key, Skill::Combat), count + 1)
        });
    let guards = if count == 0 { 0 } else { sum / count };
    let regiments =
        i32::try_from(crate::uprising::regiments(world, sys, captor).len()).unwrap_or(i32::MAX);
    own - guards - regiments
}

/// Where an escaped `character` autoroutes from `from` (`FUN_0054c7d0`,
/// `FUN_00543040`, `FUN_005545e0`), or `None` when it stays.
///
/// It stays when its own side holds `from` and the system stands. Otherwise
/// it takes the first match, nearest first (`FUN_0058b060` sorts every system
/// by `FUN_0055d860` distance), of: its side's unblockaded system
/// (`FUN_00554e80`), a system holding its side's fleet (`FUN_00554f80`), its
/// side's blockaded system (`FUN_005550e0`), a side-3 system (`FUN_005551e0`),
/// and the other side's system (`FUN_005552e0`). Blockade is
/// [`crate::blockade::BlockadeSystem`]'s rule (`+0x88` bit `0x20`).
///
/// An escapee is on no mission, so its `+0x70` (where it joined one,
/// `Role_vt268`) is clear and `FUN_00554ab0` finds nothing. port: the
/// container candidate (`FUN_00554740`) is skipped, and every candidate
/// passes, as the move validator `FUN_005571c0` is not ported, except
/// `from` itself, which would be no move. A fleet's
/// system is the destination, not the fleet. hyp: side 3 is `Contested`, as
/// in `blockade.rs`; equal distances keep the arena order (the original's
/// `qsort` order is unspecified).
pub(crate) fn escape_destination(
    world: &GameWorld,
    character: CharacterKey,
    from: SystemKey,
) -> Option<SystemKey> {
    use crate::dat::Faction;

    let c = world.characters.get(character)?;
    let side = if c.is_alliance {
        Faction::Alliance
    } else if c.is_empire {
        Faction::Empire
    } else {
        Faction::Neutral
    };
    let here = world.systems.get(from)?;
    if !here.is_destroyed && here.control.faction() == Some(side) {
        return None;
    }
    autoroute_destination(
        world,
        character,
        (i64::from(here.x), i64::from(here.y)),
        Some(from),
    )
}

/// Whether a Move order (`0x241`) of `faction` may target `to`: the mission
/// order's validator `FUN_004f4b60` refuses a movement kind (family
/// `0x40..0x4f`) whose target's side bits (`+0x24` bits 6..7) are not the
/// order's side (`1`/`0x28`). A system's side is its holder's.
#[must_use]
pub fn move_target_allowed(world: &GameWorld, faction: MissionFaction, to: SystemKey) -> bool {
    move_place_allowed(world, faction, to, None)
}

/// [`move_target_allowed`] for a Move whose target may be a fleet at `to`:
/// the fleet's side then decides.
#[must_use]
pub fn move_place_allowed(
    world: &GameWorld,
    faction: MissionFaction,
    to: SystemKey,
    fleet: Option<crate::ids::FleetKey>,
) -> bool {
    if let Some(fleet) = fleet {
        return world.fleets.get(fleet).is_some_and(|value| {
            crate::mission_detection::faction_of(value.is_alliance)
                == crate::dat::Faction::from(faction)
        });
    }
    world
        .systems
        .get(to)
        .is_some_and(|sys| sys.control.faction() == Some(crate::dat::Faction::from(faction)))
}

/// The days a Move of `character` to `to` takes, as the Move confirmation
/// lists it (`FUN_0053c2e0` -> `FUN_004f4850` for `0x241`): the character's
/// system to `to` at the speed phase 2 sets (`FUN_00548370`: GNPRTB 3083
/// for a free Han Solo, else the default). `None` without a location.
#[must_use]
pub fn move_days(world: &GameWorld, character: CharacterKey, to: SystemKey) -> Option<u32> {
    member_move_days(world, MissionMember::Character(character), to)
}

/// [`move_days`] for any member: a special force travels at the default
/// speed (`FUN_00548370` speeds up only a free Han Solo).
#[must_use]
pub fn member_move_days(world: &GameWorld, member: MissionMember, to: SystemKey) -> Option<u32> {
    let from = member_location(world, member)?;
    let position = |key: SystemKey| world.systems.get(key).map(|s| (s.x, s.y));
    let han = match member {
        MissionMember::Character(key) => {
            let c = world.characters.get(key)?;
            c.dat_id.raw() == HAN_SOLO && !c.is_captive
        }
        MissionMember::SpecialForce(key) => {
            world.special_forces.get(key)?;
            false
        }
    };
    let speed = if han {
        crate::movement::gnprtb_or_shipped(world, GNPRTB_HAN_SOLO_SPEED, SHIPPED_HAN_SOLO_SPEED)
    } else {
        crate::movement::default_speed(world)
    };
    Some(crate::movement::transit_ticks_between(
        world,
        position(from)?,
        position(to)?,
        speed,
    ))
}

/// Where `member` of the ended `mission` returns (`FUN_00521220`,
/// `FUN_00542ee0`), or `None` when it is released where it is.
///
/// `FUN_00521220` sends home a member of an ended mission by the record's
/// `+0x68` ([`crate::world::MissionRecord::leave_rule`]): with 1, a member
/// whose container (`FUN_004f6b50`) is not of its side. `FUN_00548910`
/// takes no member with a remove request (`+0x78` bit 2). For an agent
/// mission `FUN_00542ee0` then tries the container (`FUN_00554170`), the
/// member's `+0x70` (`FUN_005543c0`) and the autoroute search
/// `FUN_00543040`.
///
/// `+0x70` is where the member joined the mission: setting RoleOnMission
/// (`FUN_00534800`) runs slot `+0x268` (`Role_vt268` `FUN_00536490`), which
/// stores the member's container there (`FUN_004f6b70` -> `FUN_00534390`)
/// and clears it when the flag clears. It is taken when the move validator
/// passes it (`FUN_005543c0`); a Return (family `0x42`) needs a target of
/// its own side (`FUN_004f4b60`, `1`/`0x28`). The manual calls it the
/// character's base (pp. 110-111).
///
/// port: the container candidate is skipped (it is another side's here);
/// `+0x70` reads the mission's origin system, taken when the member's side
/// holds it, without the route checks; the search runs from the member's
/// system and leaves it out. A prisoner on the captured list stays where
/// it is. Rule 2 (Dagobah, Palace, Pickup, Bounty) keeps the port's abode
/// trips ([`MissionSystem::leave_abode`]).
fn return_destination(
    mission: &ActiveMission,
    world: &GameWorld,
    member: MissionMember,
) -> Option<SystemKey> {
    use crate::dat::Faction;
    if mission.captured.contains(&member) || mission.removing.contains(&member) {
        return None;
    }
    let record = mission
        .kind
        .record_id()
        .and_then(|id| world.mission_record(id))?;
    let state = member_state(world, member)?;
    if record.leave_rule != 1 || state.prisoner {
        return None;
    }
    let side = if state.is_alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    if container_side(world, member) == Some(side) {
        return None;
    }
    let here = member_location(world, member)?;
    if let Some(base) = mission.origin.filter(|&origin| {
        origin != here
            && world
                .systems
                .get(origin)
                .is_some_and(|sys| !sys.is_destroyed && sys.control.faction() == Some(side))
    }) {
        return Some(base);
    }
    let sys = world.systems.get(here)?;
    nearest_place(
        world,
        side,
        (i64::from(sys.x), i64::from(sys.y)),
        Some(here),
    )
}

/// The side of `member`'s container (`FUN_004f6b50`): its abode's, its
/// fleet's, or its system holder's; `None` for a system no single side
/// holds.
fn container_side(world: &GameWorld, member: MissionMember) -> Option<crate::dat::Faction> {
    use crate::dat::Faction;
    if let Some(abode) = member_abode(world, member) {
        return world.abodes.get(abode).map(|abode| abode.side);
    }
    match member_state(world, member)?.location {
        MemberLocation::Fleet(fleet) => world.fleets.get(fleet).map(|fleet| {
            if fleet.is_alliance {
                Faction::Alliance
            } else {
                Faction::Empire
            }
        }),
        MemberLocation::System(system) => world.systems.get(system)?.control.faction(),
        MemberLocation::Nowhere => None,
    }
}

/// [`escape_destination`]'s search from `origin`, leaving out `exclude`:
/// `FUN_005545e0`'s candidates in order, nearest first.
fn autoroute_destination(
    world: &GameWorld,
    character: CharacterKey,
    origin: (i64, i64),
    exclude: Option<SystemKey>,
) -> Option<SystemKey> {
    use crate::dat::Faction;

    let c = world.characters.get(character)?;
    let side = if c.is_alliance {
        Faction::Alliance
    } else if c.is_empire {
        Faction::Empire
    } else {
        Faction::Neutral
    };
    nearest_place(world, side, origin, exclude)
}

/// `FUN_005545e0`'s candidates for `side` from `origin`, leaving out
/// `exclude`: the first rule any system meets, nearest first.
pub(crate) fn nearest_place(
    world: &GameWorld,
    side: crate::dat::Faction,
    origin: (i64, i64),
    exclude: Option<SystemKey>,
) -> Option<SystemKey> {
    use crate::dat::Faction;
    use crate::world::ControlKind;

    let mut nearest: Vec<(i64, SystemKey)> = world
        .systems
        .iter()
        .filter(|(key, s)| Some(*key) != exclude && !s.is_destroyed)
        .map(|(key, s)| {
            let (dx, dy) = (i64::from(s.x) - origin.0, i64::from(s.y) - origin.1);
            (crate::movement::original_isqrt(dx * dx + dy * dy), key)
        })
        .collect();
    nearest.sort_by_key(|(distance, _)| *distance);

    let held_by = |sys: &crate::world::System, faction: Faction| matches!(sys.control, ControlKind::Controlled(f) | ControlKind::Uprising(f) if f == faction);
    let blockaded = |sys: &crate::world::System| {
        crate::blockade::BlockadeSystem::system_is_blockaded(world, sys)
    };
    let has_fleet = |sys: &crate::world::System| {
        sys.fleets
            .iter()
            .filter_map(|&f| world.fleets.get(f))
            .any(|fleet| crate::uprising::sides_match(fleet.is_alliance, !fleet.is_alliance, side))
    };
    let other = match side {
        Faction::Alliance => Some(Faction::Empire),
        Faction::Empire => Some(Faction::Alliance),
        Faction::Neutral => None,
    };
    let rules: [&dyn Fn(&crate::world::System) -> bool; 5] = [
        &|sys| held_by(sys, side) && !blockaded(sys),
        &|sys| has_fleet(sys),
        &|sys| held_by(sys, side) && blockaded(sys),
        &|sys| sys.control == ControlKind::Contested,
        &|sys| other.is_some_and(|other| held_by(sys, other)),
    ];
    rules.iter().find_map(|rule| {
        nearest
            .iter()
            .find(|(_, key)| rule(&world.systems[*key]))
            .map(|(_, key)| *key)
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mission_reads_a_character_s_effective_skills() {
        // Combat goes through slot +0x1f0, the effective combat +0x86
        // (FUN_0053e900): 90 + 90 * 50 / 100 - 10. Diplomacy goes through
        // slot +0x1dc, +0x7c (FUN_00573ff0): 65 + 65 * 50 / 100. Leadership
        // stays the rolled template.
        let mut world = GameWorld::default();
        let key = world.characters.insert(crate::world::Character {
            combat: crate::world::SkillPair {
                base: 90,
                variance: 0,
            },
            diplomacy: crate::world::SkillPair {
                base: 60,
                variance: 10,
            },
            // The Force level +0x8c comes from the Force base +0x8e.
            force_base: 50,
            injury: 10,
            ..crate::world::Character::default()
        });
        let member = MissionMember::Character(key);
        assert_eq!(member_skill(&world, member, Skill::Combat), Some(125));
        assert_eq!(member_skill(&world, member, Skill::Diplomacy), Some(97));
        world.characters[key].leadership = crate::world::SkillPair {
            base: 60,
            variance: 10,
        };
        assert_eq!(member_skill(&world, member, Skill::Leadership), Some(65));
    }
    use crate::ids::{CharacterKey, SystemKey};
    use crate::uprising::UprisingState;
    use crate::world::{Character, GameWorld, SkillPair};

    #[test]
    fn a_kind_with_no_record_times_its_mission_from_the_port_range() {
        // port: without a MISSNSD record the timer draws from tick_range.
        for (roll, due) in [(0.0, 15), (1.0, 20)] {
            let (system, _) = mock_keys();
            let mut world = minimal_world();
            let character = agent_at(&mut world, system, true);
            let mut state = MissionState::new();
            state.dispatch(MissionRequest::single(
                MissionKind::Diplomacy,
                MissionFaction::Alliance,
                character,
                system,
                None,
                0,
            ));
            step(&mut state, &world, 1, &[roll]);
            assert_eq!(state.missions()[0].timer_due, Some(due), "roll {roll}");
        }
    }

    // --- MissionState tests ---

    fn mock_keys() -> (SystemKey, CharacterKey) {
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let mut chr_sm: slotmap::SlotMap<CharacterKey, ()> = slotmap::SlotMap::with_key();
        (sys_sm.insert(()), chr_sm.insert(()))
    }

    #[test]
    fn a_dispatched_mission_starts_in_phase_zero_with_its_ready_bit_set() {
        // FUN_0054bac0 -> FUN_00522130(this, 1).
        let (system, character) = mock_keys();
        let mut state = MissionState::new();
        let id = state.dispatch(MissionRequest::single(
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            character,
            system,
            None,
            7,
        ));
        assert_eq!(state.len(), 1);
        let m = &state.missions()[0];
        assert_eq!(m.id, id);
        assert_eq!((m.phase, m.ready, m.ordered_tick), (0, true, 7));
        assert_eq!(m.timer_due, None);
    }

    #[test]
    fn cancel_removes_mission() {
        let (system, character) = mock_keys();
        let mut state = MissionState::new();
        let id = state.dispatch(MissionRequest::single(
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            character,
            system,
            None,
            0,
        ));
        let removed = state.cancel(id);
        assert!(removed.is_some());
        assert!(state.is_empty());
    }

    // --- MissionSystem::advance tests ---

    fn minimal_world() -> GameWorld {
        GameWorld::default()
    }

    fn skill_pair(base: u32) -> SkillPair {
        SkillPair { base, variance: 0 }
    }

    fn character_with_diplomacy(world: &mut GameWorld, diplomacy: u32) -> CharacterKey {
        world.characters.insert(Character {
            name: "Test".into(),
            is_alliance: true,
            diplomacy: skill_pair(diplomacy),
            can_be_commander: true,
            ..Default::default()
        })
    }

    #[test]
    fn no_tick_events_no_results() {
        let (system, character) = mock_keys();
        let mut state = MissionState::new();
        state.dispatch(MissionRequest::single(
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            character,
            system,
            None,
            0,
        ));
        let world = minimal_world();
        let results = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            &[],
            &[],
        )
        .results;
        assert!(results.is_empty());
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn a_mission_waits_in_phase_eight_until_its_timer_is_due() {
        // FUN_00522a10 sets the ready bit only when timer 0x38b fires.
        let (system, character) = mock_keys();
        let mut state = MissionState::new();
        state.missions.push_back(ActiveMission::timed(
            0,
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            vec![MissionMember::Character(character)],
            system,
            3,
        ));
        let world = minimal_world();
        let advance = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            &[TickEvent { tick: 1 }],
            &[0.0],
        );
        assert!(advance.results.is_empty());
        let mission = &state.missions()[0];
        assert_eq!((mission.phase, mission.ready), (PHASE_TIMER, false));
        assert_eq!(mission.days_left(1, None), Some(2));
    }

    #[test]
    fn mission_resolves_at_zero_ticks() {
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let character = character_with_diplomacy(&mut world, 80);

        let mut state = MissionState::new();
        // Manually insert a mission with 1 tick remaining
        state.missions.push_back(ActiveMission::timed(
            0,
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            vec![MissionMember::Character(character)],
            system,
            1, // will resolve on next tick
        ));
        state.next_id = 1;

        // Roll = 0.01 → 1% of 100 → very likely to succeed with high-skill character
        let results = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            &[TickEvent { tick: 1 }],
            &[0.01],
        )
        .results;
        assert_eq!(results.len(), 1);
        assert!(state.is_empty());
        assert_eq!(results[0].kind, MissionKind::Diplomacy);
    }

    #[test]
    fn guaranteed_failure_with_roll_one() {
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let character = character_with_diplomacy(&mut world, 50);

        let mut state = MissionState::new();
        state.missions.push_back(ActiveMission::timed(
            0,
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            vec![MissionMember::Character(character)],
            system,
            1,
        ));
        state.next_id = 1;

        // roll = 1.0 → 100% of 100 → fails since success_prob <= 100
        let advance = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            &[TickEvent { tick: 1 }],
            &[1.0],
        );
        assert_eq!(advance.results[0].outcome, MissionOutcome::Failure);
        // A failure has no world effect; the release comes with the end.
        assert!(advance.results[0].effects.is_empty());
        assert_eq!(
            advance.effects,
            vec![MissionEffect::MemberAvailable {
                member: MissionMember::Character(character)
            }]
        );
    }

    #[test]
    fn multiple_missions_resolve_independently() {
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let sys_a = sys_sm.insert(());
        let sys_b = sys_sm.insert(());
        let char_a = character_with_diplomacy(&mut world, 70);
        let char_b = character_with_diplomacy(&mut world, 70);

        let mut state = MissionState::new();
        state.missions.push_back(ActiveMission::timed(
            0,
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            vec![MissionMember::Character(char_a)],
            sys_a,
            1,
        ));
        state.missions.push_back(ActiveMission::timed(
            1,
            MissionKind::Diplomacy,
            MissionFaction::Empire,
            vec![MissionMember::Character(char_b)],
            sys_b,
            3,
        ));
        state.next_id = 2;

        let results = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            &[TickEvent { tick: 1 }],
            &[0.0], // first mission succeeds
        )
        .results;
        // Only mission 0 completes (1 tick); mission 1 has 2 ticks left
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].mission_id, 0);
        assert_eq!(state.len(), 1);
        assert_eq!(state.missions()[0].timer_due, Some(3));
    }

    // --- New mission kind tests ---

    #[test]
    fn autoscrap_always_succeeds() {
        let (system, _) = mock_keys();
        let mut world = minimal_world();
        let character = agent_at(&mut world, system, false);
        let mut state = MissionState::new();
        state.missions.push_back(ActiveMission::timed(
            0,
            MissionKind::Autoscrap,
            MissionFaction::Empire,
            vec![MissionMember::Character(character)],
            system,
            1,
        ));
        state.next_id = 1;

        // roll = 1.0 → Autoscrap ignores the roll and always succeeds.
        let results = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            &[TickEvent { tick: 1 }],
            &[1.0],
        )
        .results;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, MissionOutcome::Success);
    }

    #[test]
    fn mstb_key_matches_expected_dat_stems() {
        assert_eq!(MissionKind::Diplomacy.mstb_key(), Some("DIPLMSTB"));
        assert_eq!(MissionKind::Recruitment.mstb_key(), Some("RCRTMSTB"));
        assert_eq!(MissionKind::Sabotage.mstb_key(), Some("SBTGMSTB"));
        assert_eq!(MissionKind::Assassination.mstb_key(), Some("ASSNMSTB"));
        assert_eq!(MissionKind::Espionage.mstb_key(), Some("ESPIMSTB"));
        assert_eq!(MissionKind::Rescue.mstb_key(), Some("RESCMSTB"));
        assert_eq!(MissionKind::Abduction.mstb_key(), Some("ABDCMSTB"));
        assert_eq!(MissionKind::InciteUprising.mstb_key(), Some("INCTMSTB"));
        assert_eq!(MissionKind::Autoscrap.mstb_key(), None);
    }

    // --- Character availability tracking tests ---

    #[test]
    fn every_team_decoy_and_captured_member_is_freed_when_its_mission_completes() {
        // FUN_00525bb0 walks the team, decoy, and captured lists; completion
        // clears each member's RoleOnMissionNotif flag (FUN_00536b00).
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let lead = character_with_diplomacy(&mut world, 80);
        let prisoner = character_with_diplomacy(&mut world, 10);
        let decoy = MissionMember::SpecialForce(world.special_forces.insert(
            crate::world::SpecialForceUnit {
                class_dat_id: crate::ids::DatId::new(0x3c00_0001),
                is_alliance: true,
                skills: [0; 8],
                on_mission: true,
            },
        ));

        let mut state = MissionState::new();
        let mut mission = ActiveMission::timed(
            0,
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            vec![MissionMember::Character(lead)],
            system,
            1,
        );
        mission.decoys = vec![decoy];
        mission.captured = vec![MissionMember::Character(prisoner)];
        state.missions.push_back(mission);
        state.next_id = 1;

        let advance = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            &[TickEvent { tick: 1 }],
            &[0.0],
        );
        assert_eq!(advance.results.len(), 1);
        let freed: Vec<MissionMember> = advance
            .effects
            .iter()
            .filter_map(|e| match e {
                MissionEffect::MemberAvailable { member } => Some(*member),
                _ => None,
            })
            .collect();
        assert_eq!(
            freed,
            vec![
                MissionMember::Character(lead),
                decoy,
                MissionMember::Character(prisoner)
            ]
        );
    }

    // --- Escape tests (ghidra/notes/captive-escape.md) ---

    /// GNPRTB 2584/2585 at their shipped 100 and 1000.
    fn escape_world() -> GameWorld {
        let mut world = minimal_world();
        world.gnprtb = crate::world::GnprtbParams::uniform(&[
            (GNPRTB_ESCAPE_MIN, 100),
            (GNPRTB_ESCAPE_SPREAD, 1000),
        ]);
        world
    }

    fn held_system(world: &mut GameWorld, x: u16, holder: crate::dat::Faction) -> SystemKey {
        let system = system_at(world, x);
        world.systems[system].control = crate::world::ControlKind::Controlled(holder);
        system
    }

    fn escapetb(rows: &[(i32, u32)]) -> crate::world::MstbTable {
        crate::world::MstbTable::new(
            rows.iter()
                .map(|&(threshold, value)| crate::world::MstbEntry { threshold, value })
                .collect(),
        )
    }

    /// An Alliance character the Empire holds at `system`.
    fn captive_at(
        world: &mut GameWorld,
        system: SystemKey,
        espionage: u32,
        combat: u32,
    ) -> CharacterKey {
        world.characters.insert(Character {
            name: "Captive".into(),
            is_alliance: true,
            espionage: skill_pair(espionage),
            combat: skill_pair(combat),
            captured_by: Some(crate::dat::Faction::Empire),
            is_captive: true,
            current_system: Some(system),
            recruited: true,
            ..Default::default()
        })
    }

    fn imperial_guard(world: &mut GameWorld, system: SystemKey, combat: u32) -> CharacterKey {
        let guard = agent_at(world, system, false);
        world.characters[guard].combat = skill_pair(combat);
        guard
    }

    fn add_fleet(world: &mut GameWorld, system: SystemKey, is_alliance: bool) {
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[system].fleets.push(fleet);
    }

    fn timer_of(state: &MissionState, character: CharacterKey) -> Option<u64> {
        state
            .escape_timers
            .iter()
            .find(|timer| timer.character == character)
            .map(|timer| timer.due)
    }

    #[test]
    fn a_captive_first_tries_to_escape_after_the_gnprtb_2584_2585_delay() {
        // FUN_004f1ba0 arms timer 0x38a from the record FUN_004ef750 fills
        // with GNPRTB 2584 (min) and 2585 (spread); FUN_00586130 adds
        // rand(0..=spread). Roll 0.5 draws 500: day 10 + 100 + 500.
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, crate::dat::Faction::Empire);
        let captive = captive_at(&mut world, prison, 100, 100);
        world
            .mission_tables
            .insert("ESCAPETB".into(), escapetb(&[(0, 100)]));
        let mut state = MissionState::new();

        let effects = MissionSystem::advance_escapes(&mut state, &world, 10, &[0.5, 0.0]);
        assert!(effects.is_empty());
        assert_eq!(timer_of(&state, captive), Some(610));
        assert!(MissionSystem::advance_escapes(&mut state, &world, 609, &[0.0, 0.0]).is_empty());
        let effects = MissionSystem::advance_escapes(&mut state, &world, 610, &[0.0, 0.0]);
        assert!(matches!(
            effects.first(),
            Some(MissionEffect::CharacterEscaped { character, escaped_to_alliance: true })
                if *character == captive
        ));
    }

    #[test]
    fn a_missed_attempt_rearms_the_timer_with_a_fresh_delay() {
        // FUN_005862a0: the fire (slot +0x30) runs, then the timer adds
        // rand(0..=spread) + min again. The roll is drawn before the delay.
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, crate::dat::Faction::Empire);
        let captive = captive_at(&mut world, prison, 0, 0);
        world
            .mission_tables
            .insert("ESCAPETB".into(), escapetb(&[(0, 50)]));
        let mut state = MissionState::new();
        MissionSystem::advance_escapes(&mut state, &world, 0, &[0.0]);
        assert_eq!(timer_of(&state, captive), Some(100));

        // 0.50 draws 50, not below 50 percent: a miss; 0.25 adds 250.
        let effects = MissionSystem::advance_escapes(&mut state, &world, 100, &[0.50, 0.25]);
        assert!(effects.is_empty());
        assert_eq!(timer_of(&state, captive), Some(450));
        // 0.49 draws 49, below 50 percent: the captive escapes.
        let effects = MissionSystem::advance_escapes(&mut state, &world, 450, &[0.49, 0.0]);
        assert!(matches!(
            effects.first(),
            Some(MissionEffect::CharacterEscaped { .. })
        ));
        assert_eq!(timer_of(&state, captive), None);
    }

    #[test]
    fn escape_chance_steps_between_escapetb_rows_instead_of_interpolating() {
        // FUN_0053e310 -> FUN_0053e340: the row at or below the input
        // (FUN_00595090) applies unchanged. Input 75 sits between rows 50
        // and 100; interpolating would give 88 percent and escape on 0.85.
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, crate::dat::Faction::Empire);
        let captive = captive_at(&mut world, prison, 40, 35);
        world
            .mission_tables
            .insert("ESCAPETB".into(), escapetb(&[(50, 80), (100, 95)]));
        let mut state = MissionState::new();
        state.escape_timers.push(EscapeTimer {
            character: captive,
            due: 0,
        });

        assert!(MissionSystem::advance_escapes(&mut state, &world, 0, &[0.85, 0.0]).is_empty());
        state.escape_timers[0].due = 0;
        assert!(MissionSystem::advance_escapes(&mut state, &world, 0, &[0.80, 0.0]).is_empty());
        state.escape_timers[0].due = 0;
        assert_eq!(
            MissionSystem::advance_escapes(&mut state, &world, 0, &[0.79, 0.0]).len(),
            1
        );
    }

    #[test]
    fn escape_input_subtracts_the_captors_average_guard_combat_and_regiments() {
        // FUN_00561990 -> FUN_0055e7e0: (espionage +0x1e0 + combat +0x1f0)
        // - avg combat of the captor's usable characters there that are not
        // held (slot +0x1d4) - the captor's regiments there (FUN_00504cc0).
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, crate::dat::Faction::Empire);
        let captive = captive_at(&mut world, prison, 40, 30);
        imperial_guard(&mut world, prison, 20);
        imperial_guard(&mut world, prison, 31);
        // Not guards: another prisoner, a free Rebel, and a guard on a mission.
        captive_at(&mut world, prison, 0, 90);
        let rebel = agent_at(&mut world, prison, true);
        world.characters[rebel].combat = skill_pair(90);
        let busy = imperial_guard(&mut world, prison, 90);
        world.characters[busy].on_mission = true;
        for alliance in [false, false, true] {
            let troop = world.troops.insert(crate::world::TroopUnit {
                class_dat_id: crate::ids::DatId::new(0x1400_0000),
                is_alliance: alliance,
                regiment_strength: 1,
            });
            world.systems[prison].ground_units.push(troop);
        }

        // 70 - (20 + 31) / 2 - 2 = 70 - 25 - 2.
        assert_eq!(escape_input(&world, captive), 43);
    }

    #[test]
    fn escape_input_is_the_captives_own_skills_without_guards() {
        // FUN_00561990: with no guard the average stays 0 (count 0).
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, crate::dat::Faction::Empire);
        let captive = captive_at(&mut world, prison, 40, 30);
        assert_eq!(escape_input(&world, captive), 70);
    }

    #[test]
    fn an_escapee_heads_for_the_nearest_unblockaded_system_of_its_side() {
        // FUN_004f18e0 sets Autorouting; FUN_0054c7d0 leaves a location
        // another side holds; FUN_005545e0 tries FUN_00554e80 (own side, no
        // blockade bit) before FUN_005550e0 (own side, blockaded), each in
        // the distance order of FUN_0058b060.
        use crate::dat::Faction;
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, Faction::Empire);
        let blockaded = held_system(&mut world, 30, Faction::Alliance);
        add_fleet(&mut world, blockaded, false);
        let near = held_system(&mut world, 80, Faction::Alliance);
        let far = held_system(&mut world, 200, Faction::Alliance);
        let captive = captive_at(&mut world, prison, 0, 0);

        assert_eq!(escape_destination(&world, captive, prison), Some(near));
        // A farther clear system still beats the nearer blockaded one.
        world.systems[near].control = crate::world::ControlKind::Controlled(Faction::Empire);
        assert_eq!(escape_destination(&world, captive, prison), Some(far));
        // With no clear system left, the blockaded one is next.
        world.systems[far].control = crate::world::ControlKind::Controlled(Faction::Empire);
        assert_eq!(escape_destination(&world, captive, prison), Some(blockaded));
    }

    #[test]
    fn an_escapee_without_a_friendly_system_falls_back_to_contested_then_enemy() {
        // FUN_005551e0 (side bits 0xc0) before FUN_005552e0 (the other
        // side). hyp: side 3 is ControlKind::Contested, as in blockade.rs.
        use crate::dat::Faction;
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, Faction::Empire);
        let contested = system_at(&mut world, 90);
        world.systems[contested].control = crate::world::ControlKind::Contested;
        let captive = captive_at(&mut world, prison, 0, 0);

        let enemy = held_system(&mut world, 120, Faction::Empire);
        assert_eq!(escape_destination(&world, captive, prison), Some(contested));
        world.systems[contested].control = crate::world::ControlKind::Uncontrolled;
        assert_eq!(escape_destination(&world, captive, prison), Some(enemy));
        // port: the prison itself is never a destination.
        world.systems[enemy].control = crate::world::ControlKind::Uncontrolled;
        assert_eq!(escape_destination(&world, captive, prison), None);
    }

    #[test]
    fn an_escapee_held_where_its_own_side_rules_stays_put() {
        // FUN_0054c7d0: a location of the character's side clears
        // Autorouting (FUN_004f78e0(0)) instead of routing.
        use crate::dat::Faction;
        let mut world = escape_world();
        let home = held_system(&mut world, 0, Faction::Alliance);
        held_system(&mut world, 50, Faction::Alliance);
        let captive = captive_at(&mut world, home, 0, 0);
        assert_eq!(escape_destination(&world, captive, home), None);
    }

    #[test]
    fn an_escapee_leaves_the_map_and_arrives_after_its_transit_days() {
        // build-delivery.md, Travel: FUN_0055d8c0 gives isqrt(100^2) / 5 *
        // 100 / 100 = 20 days at the GNPRTB 1 default speed.
        use crate::dat::Faction;
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, Faction::Empire);
        let haven = held_system(&mut world, 100, Faction::Alliance);
        let captive = captive_at(&mut world, prison, 0, 0);
        world
            .mission_tables
            .insert("ESCAPETB".into(), escapetb(&[(0, 100)]));
        let mut state = MissionState::new();
        state.escape_timers.push(EscapeTimer {
            character: captive,
            due: 5,
        });

        let member = MissionMember::Character(captive);
        let effects = MissionSystem::advance_escapes(&mut state, &world, 5, &[0.0, 0.0]);
        assert_eq!(effects[1], MissionEffect::MemberMoved { member, to: None });
        assert!(MissionSystem::advance_escapes(&mut state, &world, 24, &[]).is_empty());
        assert_eq!(
            MissionSystem::advance_escapes(&mut state, &world, 25, &[]),
            vec![MissionEffect::MemberMoved {
                member,
                to: Some(haven)
            }]
        );
    }

    #[test]
    fn escape_distance_is_the_isqrt_of_the_squared_coordinate_offsets() {
        // FUN_0058b060 ranks systems by FUN_0055d860: isqrt(dx^2 + dy^2).
        // Each case puts a farther system first in the arena.
        use crate::dat::Faction;
        let cases = [
            // (prison, farther, nearer)
            ((100, 0), (30, 0), (150, 0)),
            ((0, 100), (0, 30), (0, 150)),
            ((0, 0), (0, 50), (30, 0)),
        ];
        for (prison_at, farther_at, nearer_at) in cases {
            let mut world = escape_world();
            let mut place = |(x, y): (u16, u16), holder| {
                let system = held_system(&mut world, x, holder);
                world.systems[system].y = y;
                system
            };
            let prison = place(prison_at, Faction::Empire);
            place(farther_at, Faction::Alliance);
            let nearer = place(nearer_at, Faction::Alliance);
            let captive = captive_at(&mut world, prison, 0, 0);
            assert_eq!(escape_destination(&world, captive, prison), Some(nearer));
        }
    }

    #[test]
    fn an_imperial_escapee_with_no_imperial_system_joins_an_imperial_fleets_system() {
        // FUN_00554f80: the nearest system holding a fleet of the
        // character's own side (FUN_004ffef0(system, 3, side)).
        use crate::dat::Faction;
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, Faction::Alliance);
        let rebel_fleet = system_at(&mut world, 20);
        add_fleet(&mut world, rebel_fleet, true);
        let imperial_fleet = system_at(&mut world, 60);
        add_fleet(&mut world, imperial_fleet, false);
        let captive = imperial_guard(&mut world, prison, 0);
        world.characters[captive].is_captive = true;
        world.characters[captive].captured_by = Some(Faction::Alliance);
        assert_eq!(
            escape_destination(&world, captive, prison),
            Some(imperial_fleet)
        );
    }

    #[test]
    fn a_blockaded_system_counts_only_when_the_escapees_side_holds_it() {
        // FUN_005550e0 needs the character's side and the blockade bit; a
        // blockaded contested system waits for FUN_005551e0, nearest first.
        use crate::dat::Faction;
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, Faction::Empire);
        let contested = system_at(&mut world, 40);
        world.systems[contested].control = crate::world::ControlKind::Contested;
        let blockaded = system_at(&mut world, 80);
        world.systems[blockaded].control = crate::world::ControlKind::Contested;
        add_fleet(&mut world, blockaded, false);
        let captive = captive_at(&mut world, prison, 0, 0);
        assert_eq!(escape_destination(&world, captive, prison), Some(contested));
    }

    #[test]
    fn a_killed_captive_has_no_escape_timer() {
        // A destroyed character (+0x50 bit 3) is gone; the port keeps the
        // record but never arms it.
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, crate::dat::Faction::Empire);
        let captive = captive_at(&mut world, prison, 0, 0);
        world.characters[captive].is_killed = true;
        let mut state = MissionState::new();
        MissionSystem::advance_escapes(&mut state, &world, 0, &[0.0]);
        assert_eq!(timer_of(&state, captive), None);
    }

    #[test]
    fn a_captive_freed_another_way_loses_its_escape_timer() {
        // A rescue or uprising release clears Captured, which drops CanEscape
        // and changes the timer record, so timer 0x38a is deleted unfired
        // (FUN_005862a0 compares the record counter).
        let mut world = escape_world();
        let prison = held_system(&mut world, 0, crate::dat::Faction::Empire);
        let captive = captive_at(&mut world, prison, 0, 0);
        let mut state = MissionState::new();
        MissionSystem::advance_escapes(&mut state, &world, 0, &[0.0]);
        assert!(timer_of(&state, captive).is_some());

        world.characters[captive].is_captive = false;
        world.characters[captive].captured_by = None;
        MissionSystem::advance_escapes(&mut state, &world, 1, &[]);
        assert_eq!(timer_of(&state, captive), None);
    }

    #[test]
    fn dispatch_guarded_blocks_mandatory_mission() {
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let character = world.characters.insert(Character {
            name: "Mandatory".into(),
            is_alliance: true,
            is_major: true,
            diplomacy: skill_pair(80),
            can_be_commander: true,
            on_mandatory_mission: true, // <-- mandatory
            ..Default::default()
        });

        let mut state = MissionState::new();
        let result = state.dispatch_guarded(
            MissionRequest::single(
                MissionKind::Diplomacy,
                MissionFaction::Alliance,
                character,
                system,
                None,
                0,
            ),
            &mut world,
        );
        assert_eq!(
            result,
            Err(MissionRefusal::MemberUnavailable(MissionMember::Character(
                character
            ))),
            "mandatory mission character should be blocked"
        );
        assert!(state.is_empty());
    }

    #[test]
    fn a_character_with_an_active_mission_is_refused_even_without_the_flag() {
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let character = world.characters.insert(Character {
            name: "Agent".into(),
            is_alliance: true,
            ..Default::default()
        });
        // An older save: the mission is in flight but on_mission was never set.
        let mut state = MissionState::new();
        state.dispatch(MissionRequest::single(
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            character,
            system,
            None,
            0,
        ));

        let second = state.dispatch_guarded(
            MissionRequest::single(
                MissionKind::Espionage,
                MissionFaction::Alliance,
                character,
                system,
                None,
                0,
            ),
            &mut world,
        );

        assert_eq!(
            second,
            Err(MissionRefusal::MemberUnavailable(MissionMember::Character(
                character
            )))
        );
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn a_dispatched_character_cannot_take_a_second_mission_until_released() {
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let character = world.characters.insert(Character {
            name: "Agent".into(),
            is_alliance: true,
            diplomacy: skill_pair(80),
            ..Default::default()
        });
        let mut state = MissionState::new();
        let send = |state: &mut MissionState, world: &mut GameWorld| {
            state.dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Diplomacy,
                    MissionFaction::Alliance,
                    character,
                    system,
                    None,
                    0,
                ),
                world,
            )
        };

        let first = send(&mut state, &mut world).expect("first dispatch");
        assert!(world.characters[character].on_mission);
        assert!(send(&mut state, &mut world).is_err());
        assert_eq!(state.len(), 1);

        state.release(first, &mut world);
        assert!(!world.characters[character].on_mission);
        assert!(send(&mut state, &mut world).is_ok());
    }

    // --- Member rules (FUN_0054bb90, FUN_00522b30) ---

    /// A loyal agent (loyalty 100, so phase 9 never finds a traitor,
    /// `FUN_00588da0`).
    fn agent_at(world: &mut GameWorld, system: SystemKey, is_alliance: bool) -> CharacterKey {
        world.characters.insert(Character {
            name: "Agent".into(),
            is_alliance,
            is_empire: !is_alliance,
            loyalty: crate::world::SkillPair {
                base: 100,
                variance: 0,
            },
            current_system: Some(system),
            recruited: true,
            ..Default::default()
        })
    }

    fn two_systems() -> (SystemKey, SystemKey) {
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        (sys_sm.insert(()), sys_sm.insert(()))
    }

    fn request(
        team: Vec<MissionMember>,
        decoys: Vec<MissionMember>,
        at: SystemKey,
    ) -> MissionRequest {
        MissionRequest {
            kind: MissionKind::Sabotage,
            faction: MissionFaction::Alliance,
            team,
            decoys,
            target_system: at,
            target_character: None,
            // FUN_0056a110 reads only the object's family at creation.
            target_object: Some(MissionTarget::ManufacturingFacility(
                crate::ids::ManufacturingFacilityKey::default(),
            )),
            tick: 0,
        }
    }

    // --- The pop-up menu's Mission item (FUN_0051fe20, FUN_005429e0) ---

    #[test]
    fn mission_is_enabled_for_a_free_recruited_member_of_the_players_side() {
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let agent = MissionMember::Character(agent_at(&mut world, here, true));
        let state = MissionState::new();
        assert!(state.mission_order_enabled(&world, MissionFaction::Alliance, &[agent]));
        // FUN_004f9860: the other side's player may not order it (status 1).
        assert!(!state.mission_order_enabled(&world, MissionFaction::Empire, &[agent]));
    }

    #[test]
    fn mission_is_disabled_for_a_busy_captive_or_unrecruited_member() {
        // FUN_004f9860 (unrecruited, status 2), FUN_00533ce0 (on a mission),
        // FUN_004ed560 (a prisoner, for order 0x240).
        type Spoil = fn(&mut Character);
        let spoils: [(&str, Spoil); 4] = [
            ("on a mission", |c| c.on_mission = true),
            ("on a mandatory mission", |c| c.on_mandatory_mission = true),
            ("a prisoner", |c| c.is_captive = true),
            ("unrecruited", |c| c.recruited = false),
        ];
        for (why, spoil) in spoils {
            let mut world = minimal_world();
            let (here, _) = two_systems();
            let key = agent_at(&mut world, here, true);
            spoil(&mut world.characters[key]);
            let state = MissionState::new();
            assert!(
                !state.mission_order_enabled(
                    &world,
                    MissionFaction::Alliance,
                    &[MissionMember::Character(key)]
                ),
                "{why}"
            );
        }
    }

    #[test]
    fn mission_is_disabled_while_a_member_travels_or_serves_a_queued_mission() {
        // FUN_004f9860 refuses an object en route (status 4); FUN_00533ce0
        // one already on a mission.
        let mut world = minimal_world();
        let (here, there) = two_systems();
        let agent = MissionMember::Character(agent_at(&mut world, here, true));
        let mut travelling = MissionState::new();
        travelling.push_transit(MemberTransit {
            member: agent,
            mission_id: 7,
            to: there,
            arrival: 10,
            abode: None,
        });
        assert!(!travelling.mission_order_enabled(&world, MissionFaction::Alliance, &[agent]));

        let mut queued = MissionState::new();
        queued.dispatch(request(vec![agent], vec![], here));
        assert!(!queued.mission_order_enabled(&world, MissionFaction::Alliance, &[agent]));
    }

    #[test]
    fn mission_needs_every_located_member_at_one_place_and_one_located_member() {
        // FUN_0054bf00: members apart fail with 0x40/3, a team without a
        // located member with 0x16; FUN_0054bb90 fails an empty team.
        let mut world = minimal_world();
        let (here, there) = two_systems();
        let a = MissionMember::Character(agent_at(&mut world, here, true));
        let b = MissionMember::Character(agent_at(&mut world, here, true));
        let far = MissionMember::Character(agent_at(&mut world, there, true));
        let nowhere_key = agent_at(&mut world, here, true);
        world.characters[nowhere_key].current_system = None;
        let nowhere = MissionMember::Character(nowhere_key);
        let state = MissionState::new();
        let enabled = |team: &[MissionMember]| {
            state.mission_order_enabled(&world, MissionFaction::Alliance, team)
        };

        assert!(enabled(&[a, b]));
        assert!(!enabled(&[a, far]));
        assert!(enabled(&[a, nowhere]));
        assert!(!enabled(&[nowhere]));
        assert!(!enabled(&[]));
    }

    #[test]
    fn mission_is_disabled_for_a_member_that_no_longer_exists() {
        // port: a killed character stays in the arena but counts as gone.
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let key = agent_at(&mut world, here, true);
        world.characters[key].is_killed = true;
        let state = MissionState::new();
        assert!(!state.mission_order_enabled(
            &world,
            MissionFaction::Alliance,
            &[MissionMember::Character(key)]
        ));
    }

    #[test]
    fn a_prisoner_goes_to_the_captured_list_and_a_team_of_prisoners_is_refused() {
        // FUN_0054bb90 moves a prisoner (slot +0x1d4) from either list to the
        // captured list, then refuses an empty team with 0x40/0x91.
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let lead = agent_at(&mut world, here, true);
        let prisoner = agent_at(&mut world, here, true);
        world.characters[prisoner].is_captive = true;
        let mut state = MissionState::new();

        let alone = request(vec![MissionMember::Character(prisoner)], vec![], here);
        assert_eq!(
            state.dispatch_guarded(alone, &mut world),
            Err(MissionRefusal::EmptyTeam)
        );
        assert!(!world.characters[prisoner].on_mission);

        let team = vec![
            MissionMember::Character(lead),
            MissionMember::Character(prisoner),
        ];
        state
            .dispatch_guarded(request(team, vec![], here), &mut world)
            .expect("the free lead keeps the team non-empty");
        let mission = &state.missions()[0];
        assert_eq!(mission.team, vec![MissionMember::Character(lead)]);
        assert_eq!(mission.captured, vec![MissionMember::Character(prisoner)]);
    }

    #[test]
    fn a_member_of_the_other_side_is_refused() {
        // FUN_00522b30: the member's side (+0x24 & 0xc0) must match the mission's.
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let lead = agent_at(&mut world, here, true);
        let imperial = agent_at(&mut world, here, false);
        let mut state = MissionState::new();
        let team = vec![
            MissionMember::Character(lead),
            MissionMember::Character(imperial),
        ];

        assert_eq!(
            state.dispatch_guarded(request(team, vec![], here), &mut world),
            Err(MissionRefusal::MemberUnavailable(MissionMember::Character(
                imperial
            )))
        );
        assert!(state.is_empty());
        assert!(!world.characters[lead].on_mission);
    }

    #[test]
    fn members_at_different_locations_are_refused() {
        // FUN_00522b30: a member must share the location key (slot +0xc) of
        // the members already on the mission.
        let mut world = minimal_world();
        let (here, there) = two_systems();
        let lead = agent_at(&mut world, here, true);
        let far = agent_at(&mut world, there, true);
        let mut state = MissionState::new();
        let team = vec![
            MissionMember::Character(lead),
            MissionMember::Character(far),
        ];

        assert_eq!(
            state.dispatch_guarded(request(team, vec![], here), &mut world),
            Err(MissionRefusal::MemberUnavailable(MissionMember::Character(
                far
            )))
        );
    }

    #[test]
    fn a_member_named_twice_joins_once() {
        // FUN_00522b30 skips a member already in the team, decoy, or captured
        // list (FUN_00520c30).
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let lead = agent_at(&mut world, here, true);
        let mut state = MissionState::new();
        let me = MissionMember::Character(lead);

        let decoy = MissionMember::Character(agent_at(&mut world, here, true));
        let prisoner = agent_at(&mut world, here, true);
        world.characters[prisoner].is_captive = true;
        let prisoner = MissionMember::Character(prisoner);

        state
            .dispatch_guarded(
                request(
                    vec![me, me, prisoner, prisoner],
                    vec![me, decoy, decoy],
                    here,
                ),
                &mut world,
            )
            .expect("one lead");
        let mission = &state.missions()[0];
        assert_eq!(mission.team, vec![me]);
        assert_eq!(mission.decoys, vec![decoy]);
        assert_eq!(mission.captured, vec![prisoner]);
    }

    #[test]
    fn the_lead_is_the_first_team_character_and_a_special_force_has_no_character_key() {
        // Our helper: the lists' named character is the first team
        // character; a special force is never that character.
        let mut sf: slotmap::SlotMap<SpecialForceKey, ()> = slotmap::SlotMap::with_key();
        let unit = MissionMember::SpecialForce(sf.insert(()));
        let mut chr: slotmap::SlotMap<CharacterKey, ()> = slotmap::SlotMap::with_key();
        let (character, second) = (chr.insert(()), chr.insert(()));
        let (system, _) = mock_keys();
        let mission = ActiveMission::timed(
            0,
            MissionKind::Sabotage,
            MissionFaction::Alliance,
            vec![
                unit,
                MissionMember::Character(character),
                MissionMember::Character(second),
            ],
            system,
            1,
        );

        assert_eq!(unit.character(), None);
        assert_eq!(mission.lead_character(), Some(character));
        let empty = ActiveMission::timed(
            1,
            MissionKind::Sabotage,
            MissionFaction::Alliance,
            vec![unit],
            system,
            1,
        );
        assert_eq!(empty.lead_character(), None);
    }

    #[test]
    fn leaving_a_mission_clears_the_hidden_mission_flag_and_joining_keeps_it() {
        // port: leaving clears RoleOnMission (bit 7) and RoleOnHiddenMission
        // (bit 8) together. FUN_00536220 recomputes bit 7 on leave, and where
        // bit 8 clears is untraced. Joining sets only bit 7 here (FUN_00522b30
        // sets bit 8 from the record, F-019 phase 2).
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let lead = agent_at(&mut world, here, true);
        world.characters[lead].on_hidden_mission = true;

        set_on_mission(&mut world, MissionMember::Character(lead), true);
        assert!(world.characters[lead].on_hidden_mission);
        set_on_mission(&mut world, MissionMember::Character(lead), false);
        assert!(!world.characters[lead].on_mission);
        assert!(!world.characters[lead].on_hidden_mission);
    }

    /// A special force on the surface of a new system (special forces live
    /// in a system's list, not in a fleet).
    fn unit_on_surface(world: &mut GameWorld, is_alliance: bool) -> (SpecialForceKey, SystemKey) {
        let unit = world.special_forces.insert(crate::world::SpecialForceUnit {
            class_dat_id: crate::ids::DatId::new(0x3c00_0003),
            is_alliance,
            skills: [0; 8],
            on_mission: false,
        });
        let at = world.systems.insert(crate::world::System {
            last_arrival: None,
            dat_id: crate::ids::DatId::new(0x9000_0001),
            name: "Base".into(),
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
            special_forces: vec![unit],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: crate::world::ControlKind::Uncontrolled,
        });
        (unit, at)
    }

    #[test]
    fn a_killed_character_is_refused() {
        // FUN_00522b30 accepts only a member that exists. port: a killed
        // character stays in the arena (Character::mark_killed), so it counts
        // as gone.
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let dead = agent_at(&mut world, here, true);
        world.characters[dead].mark_killed();
        let mut state = MissionState::new();
        let member = MissionMember::Character(dead);

        assert_eq!(
            state.dispatch_guarded(request(vec![member], vec![], here), &mut world),
            Err(MissionRefusal::MemberUnavailable(member))
        );
        assert!(state.is_empty());
        assert!(!world.characters[dead].on_mission);
    }

    #[test]
    fn a_member_that_no_longer_exists_is_refused_and_no_flag_is_set() {
        // FUN_00522b30 accepts only a member that exists; flags are set only
        // after every member passes.
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let lead = agent_at(&mut world, here, true);
        let gone = agent_at(&mut world, here, true);
        world.characters.remove(gone);
        let mut state = MissionState::new();
        let team = vec![
            MissionMember::Character(lead),
            MissionMember::Character(gone),
        ];

        assert_eq!(
            state.dispatch_guarded(request(team, vec![], here), &mut world),
            Err(MissionRefusal::MemberUnavailable(MissionMember::Character(
                gone
            )))
        );
        assert!(!world.characters[lead].on_mission);
    }

    #[test]
    fn a_special_force_already_on_a_mission_is_refused_and_no_flag_is_set() {
        // FUN_00522b30: a member must have no mission yet (+0x68 key empty).
        let mut world = minimal_world();
        let (unit, at) = unit_on_surface(&mut world, true);
        world.special_forces[unit].on_mission = true;
        let lead = agent_at(&mut world, at, true);
        let mut state = MissionState::new();
        let decoy = MissionMember::SpecialForce(unit);

        assert_eq!(
            state.dispatch_guarded(
                request(vec![MissionMember::Character(lead)], vec![decoy], at),
                &mut world
            ),
            Err(MissionRefusal::MemberUnavailable(decoy))
        );
        assert!(!world.characters[lead].on_mission);
    }

    #[test]
    fn a_member_held_on_another_missions_captured_list_is_busy() {
        // FUN_00522b30: a member must have no mission yet; a captured member
        // has one (FUN_0054c200 adds the captured list too).
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let prisoner = MissionMember::Character(agent_at(&mut world, here, true));
        let mut state = MissionState::new();
        let mut held = ActiveMission::timed(
            0,
            MissionKind::Rescue,
            MissionFaction::Alliance,
            vec![MissionMember::Character(agent_at(&mut world, here, true))],
            here,
            5,
        );
        held.captured = vec![prisoner];
        state.missions.push_back(held);
        state.next_id = 1;

        assert_eq!(
            state.dispatch_guarded(request(vec![prisoner], vec![], here), &mut world),
            Err(MissionRefusal::MemberUnavailable(prisoner))
        );
    }

    #[test]
    fn a_prisoner_named_as_a_decoy_goes_to_the_captured_list_not_the_decoys() {
        // FUN_0054bb90 moves a prisoner out of the decoy list before any add,
        // so FUN_00522b30 never sees a prisoner as a decoy; it records the
        // mission on every member it accepts (FUN_00534230).
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let lead = agent_at(&mut world, here, true);
        let prisoner = agent_at(&mut world, here, true);
        world.characters[prisoner].is_captive = true;
        let mut state = MissionState::new();

        state
            .dispatch_guarded(
                request(
                    vec![MissionMember::Character(lead)],
                    vec![MissionMember::Character(prisoner)],
                    here,
                ),
                &mut world,
            )
            .expect("dispatch");
        let mission = &state.missions()[0];
        assert!(mission.decoys.is_empty());
        assert_eq!(mission.captured, vec![MissionMember::Character(prisoner)]);
        assert!(world.characters[prisoner].on_mission);
    }

    #[test]
    fn the_first_free_team_member_sets_the_location_even_after_a_prisoner() {
        // FUN_0054bb90 moves prisoners out first; FUN_0054c200 then adds the
        // team before the captured list, so a prisoner held elsewhere is the
        // member refused, not the free lead named after it.
        let mut world = minimal_world();
        let (here, there) = two_systems();
        let prisoner = agent_at(&mut world, there, true);
        world.characters[prisoner].is_captive = true;
        let lead = agent_at(&mut world, here, true);
        let mut state = MissionState::new();
        let team = vec![
            MissionMember::Character(prisoner),
            MissionMember::Character(lead),
        ];

        assert_eq!(
            state.dispatch_guarded(request(team, vec![], here), &mut world),
            Err(MissionRefusal::MemberUnavailable(MissionMember::Character(
                prisoner
            )))
        );
    }

    #[test]
    fn a_fleet_character_and_a_surface_character_at_one_system_are_refused() {
        // FUN_00522b30 compares the location key (slot +0xc): a fleet in orbit
        // and the system below it are different locations.
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let mut fleets: slotmap::SlotMap<crate::ids::FleetKey, ()> = slotmap::SlotMap::with_key();
        let aboard = agent_at(&mut world, here, true);
        world.characters[aboard].current_fleet = Some(fleets.insert(()));
        let below = agent_at(&mut world, here, true);
        let mut state = MissionState::new();
        let team = vec![
            MissionMember::Character(aboard),
            MissionMember::Character(below),
        ];

        assert_eq!(
            state.dispatch_guarded(request(team, vec![], here), &mut world),
            Err(MissionRefusal::MemberUnavailable(MissionMember::Character(
                below
            )))
        );
    }

    #[test]
    fn a_special_force_decoy_joins_the_decoys_and_is_freed_on_release() {
        // FUN_0054c200 adds the decoy list with as_decoy = 1 (FUN_00522b30
        // accepts families 0x30..0x3f); releasing clears each member's flag.
        let mut world = minimal_world();
        let (here, _) = two_systems();
        let lead = agent_at(&mut world, here, true);
        let (unit, at) = unit_on_surface(&mut world, true);
        world.characters[lead].current_system = Some(at);
        let mut state = MissionState::new();
        let decoy = MissionMember::SpecialForce(unit);

        let id = state
            .dispatch_guarded(
                request(vec![MissionMember::Character(lead)], vec![decoy], at),
                &mut world,
            )
            .expect("dispatch");
        assert_eq!(state.missions()[0].decoys, vec![decoy]);
        assert!(world.special_forces[unit].on_mission);

        state.release(id, &mut world);
        assert!(!world.special_forces[unit].on_mission);
        assert!(!world.characters[lead].on_mission);
    }

    // --- Phase 10: the per-member roll (FUN_00592f50) ---

    /// A table whose every row is `percent`.
    fn always(world: &mut GameWorld, kind: MissionKind, percent: u32) {
        world.mission_tables.insert(
            kind.mstb_key().unwrap().to_string(),
            crate::world::MstbTable::new(vec![crate::world::MstbEntry {
                threshold: 0,
                value: percent,
            }]),
        );
    }

    /// Run phase 10 of a `kind` mission by `team` at `target` on day 1.
    fn phase_ten(
        world: &GameWorld,
        kind: MissionKind,
        team: Vec<MissionMember>,
        target: SystemKey,
        target_character: Option<CharacterKey>,
    ) -> MissionResult {
        let mut mission = ActiveMission::timed(0, kind, MissionFaction::Alliance, team, target, 1);
        mission.target_character = target_character;
        let mut state = MissionState::new();
        state.push_timed(mission);
        step(&mut state, world, 1, &[]).results.remove(0)
    }

    fn raised(result: &MissionResult) -> Vec<(CharacterKey, Skill, i32)> {
        result
            .effects
            .iter()
            .filter_map(|effect| match effect {
                MissionEffect::SkillRaised {
                    character,
                    skill,
                    amount,
                } => Some((*character, *skill, *amount)),
                _ => None,
            })
            .collect()
    }

    fn held_by(world: &mut GameWorld, system: SystemKey, side: crate::dat::Faction) {
        world.systems[system].control = crate::world::ControlKind::Controlled(side);
    }

    #[test]
    fn every_team_character_rolls_and_a_success_raises_its_skill_by_gnprtb() {
        // FUN_00592f50 rolls each team character, then each team special
        // force; decoys never roll. FUN_005740a0 raises diplomacy by GNPRTB
        // 6156 (shipped 1) through slot +0x1d8, which a special force lacks
        // (FUN_006158b0).
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Diplomacy, 100);
        let first = agent_at(&mut world, here, true);
        let second = agent_at(&mut world, here, true);
        let unit = world.special_forces.insert(crate::world::SpecialForceUnit {
            class_dat_id: crate::ids::DatId::new(0x1400_0001),
            is_alliance: true,
            skills: [0; 8],
            on_mission: true,
        });
        let team = vec![
            MissionMember::SpecialForce(unit),
            MissionMember::Character(first),
            MissionMember::Character(second),
        ];

        let result = phase_ten(&world, MissionKind::Diplomacy, team, here, None);

        assert_eq!(result.outcome, MissionOutcome::Success);
        assert_eq!(
            raised(&result),
            vec![(first, Skill::Diplomacy, 1), (second, Skill::Diplomacy, 1)]
        );
    }

    #[test]
    fn a_failed_roll_leaves_the_result_failed_and_changes_nothing() {
        // Slot +0x280 (FUN_00576700) turns an unset result into 2.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Diplomacy, 0);
        let envoy = MissionMember::Character(agent_at(&mut world, here, true));

        let result = phase_ten(&world, MissionKind::Diplomacy, vec![envoy], here, None);

        assert_eq!(result.outcome, MissionOutcome::Failure);
        assert!(result.effects.is_empty());
    }

    #[test]
    fn diplomacy_wins_support_at_its_side_s_or_a_neutral_system_but_not_the_opponent_s() {
        // FUN_00574120 -> FUN_0055cac0: G6183 + rand(0..=G6184) at home,
        // G6185 + rand(0..=G6186) at a neutral system (side 3,
        // FUN_004f8c60), 0 at a system the opponent holds.
        let mut world = minimal_world();
        world.gnprtb =
            crate::world::GnprtbParams::uniform(&[(6183, 4), (6184, 0), (6185, 2), (6186, 0)]);
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Diplomacy, 100);
        let envoy = MissionMember::Character(agent_at(&mut world, here, true));
        let support = |result: &MissionResult| {
            result
                .effects
                .iter()
                .filter(|effect| matches!(effect, MissionEffect::SupportGained { .. }))
                .cloned()
                .collect::<Vec<_>>()
        };

        held_by(&mut world, here, crate::dat::Faction::Alliance);
        let home = phase_ten(&world, MissionKind::Diplomacy, vec![envoy], here, None);
        held_by(&mut world, here, crate::dat::Faction::Empire);
        let away = phase_ten(&world, MissionKind::Diplomacy, vec![envoy], here, None);
        world.systems[here].control = crate::world::ControlKind::Uncontrolled;
        let neutral = phase_ten(&world, MissionKind::Diplomacy, vec![envoy], here, None);

        assert_eq!(
            support(&home),
            vec![MissionEffect::SupportGained {
                system: here,
                side: crate::dat::Faction::Alliance,
                points: 4,
            }]
        );
        assert!(support(&away).is_empty());
        assert_eq!(
            support(&neutral),
            vec![MissionEffect::SupportGained {
                system: here,
                side: crate::dat::Faction::Alliance,
                points: 2,
            }]
        );
        assert_eq!(away.outcome, MissionOutcome::Success);
    }

    #[test]
    fn espionage_raises_espionage_only_at_a_system_its_side_does_not_hold() {
        // FUN_00573540: the raise needs a target of another side; FUN_00573610
        // raises observation at a system target either way.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Espionage, 100);
        let spy = agent_at(&mut world, here, true);

        held_by(&mut world, here, crate::dat::Faction::Alliance);
        let home = phase_ten(
            &world,
            MissionKind::Espionage,
            vec![MissionMember::Character(spy)],
            here,
            None,
        );
        world.systems[here].control = crate::world::ControlKind::Uncontrolled;
        let away = phase_ten(
            &world,
            MissionKind::Espionage,
            vec![MissionMember::Character(spy)],
            here,
            None,
        );

        assert!(raised(&home).is_empty());
        assert_eq!(raised(&away), vec![(spy, Skill::Espionage, 1)]);
        for result in [&home, &away] {
            assert!(result
                .effects
                .contains(&MissionEffect::SystemIntelligenceGathered {
                    system: here,
                    faction: MissionFaction::Alliance,
                }));
        }
    }

    #[test]
    fn reconnaissance_always_succeeds_and_observes_its_system_without_a_raise() {
        // FUN_0056bea0 gives 100, FUN_0056bec0 sets 3 with no skill raise,
        // and FUN_0056bee0 raises observation (level 8) at a system target.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        held_by(&mut world, here, crate::dat::Faction::Empire);
        let scout = agent_at(&mut world, here, true);

        let result = phase_ten(
            &world,
            MissionKind::Reconnaissance,
            vec![MissionMember::Character(scout)],
            here,
            None,
        );

        assert_eq!(result.outcome, MissionOutcome::Success);
        assert!(raised(&result).is_empty());
        assert!(result
            .effects
            .contains(&MissionEffect::SystemIntelligenceGathered {
                system: here,
                faction: MissionFaction::Alliance,
            }));
    }

    #[test]
    fn the_dialog_lists_reconnaissance_only_at_another_sides_system() {
        // The shipped Recon record (0x54000015) takes other and opponent
        // targets, not its own side's (columns 15..17), under the system
        // rules (FUN_0056be20 -> FUN_005868c0).
        let recon = crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x5400_0015),
            timer_min_days: 1,
            timer_spread_days: 5,
            repeats: false,
            hidden: false,
            detection_phases: true,
            can_resign: true,
            leave_rule: 0,
            rules: crate::world::MissionTargetRules {
                other_side_target: true,
                opponent_target: true,
                revolting_target: true,
                calm_target: true,
                ..Default::default()
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0x104,
                character_mask: 0x1_0000,
            },
        };
        let (mut world, here, envoy) = dialog_world(vec![recon]);
        let list = |world: &GameWorld| {
            available_kinds(
                world,
                &UprisingState::default(),
                &ResearchState::default(),
                MissionFaction::Alliance,
                &[envoy],
                &[],
                here,
            )
        };
        assert!(list(&world).is_empty());
        world.systems[here].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        assert_eq!(list(&world), vec![MissionKind::Reconnaissance]);
    }

    /// The shipped Research record for `kind` (MISSNSD `0x53000020..22`):
    /// an own-side system target, revolting or calm; characters of both
    /// sides (mask `0x10000`), no special forces.
    fn research_record(kind: MissionKind) -> crate::world::MissionRecord {
        crate::world::MissionRecord {
            dat_id: kind.record_id().unwrap(),
            timer_min_days: 10,
            timer_spread_days: 10,
            repeats: true,
            hidden: false,
            detection_phases: false,
            can_resign: false,
            leave_rule: 0,
            rules: crate::world::MissionTargetRules {
                own_side_target: true,
                revolting_target: true,
                calm_target: true,
                ..Default::default()
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0,
                character_mask: 0x1_0000,
            },
        }
    }

    /// Give `system` an Alliance yard of MANFACSD `family`.
    fn yard_at(world: &mut GameWorld, system: SystemKey, family: u32) {
        let yard =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: crate::ids::DatId::new(family << 24 | 1),
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: family == 0x28,
                });
        world.systems[system].manufacturing_facilities.push(yard);
    }

    #[test]
    fn an_aborted_mission_ends_on_its_next_step_and_frees_its_members() {
        // Order 0x250: mission slot +0x1dc(2) sets end code 2, and the
        // stepper FUN_005227d0 sends any end code to phase 0xb.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let agent = MissionMember::Character(agent_at(&mut world, here, true));
        let mut state = MissionState::new();
        state.push_timed(ActiveMission::timed(
            0,
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            vec![agent],
            here,
            5,
        ));
        // FUN_00521600: only in phase 8 and only by the mission's side.
        state.missions[0].phase = 4;
        assert!(!state.abort_allowed(0, MissionFaction::Alliance));
        state.missions[0].phase = 8;
        assert!(!state.abort_allowed(0, MissionFaction::Empire));
        assert!(state.abort_allowed(0, MissionFaction::Alliance));
        assert!(!state.abort_allowed(1, MissionFaction::Alliance));
        assert!(state.abort(0));
        assert!(!state.abort(0));
        assert!(!state.abort_allowed(0, MissionFaction::Alliance));

        let advance = step(&mut state, &world, 1, &[]);

        assert!(state.is_empty());
        assert_eq!(advance.ended.len(), 1);
        assert_eq!(advance.ended[0].end_code, 2);
        assert!(advance
            .effects
            .contains(&MissionEffect::MemberAvailable { member: agent }));
        assert!(!state.abort(0));
    }

    #[test]
    fn each_research_success_adds_points_and_raises_the_tree_skill() {
        // FUN_0056cdb0: a draw below the member's ship design (FUN_005337f0)
        // adds GNPRTB 6171 + rand(0..=6172) points (FUN_0055c9f0) and raises
        // the skill by GNPRTB 6158; FUN_0056cf30 adds the mission's points to
        // the side's tree and sets 2 + (points != 0).
        let mut world = minimal_world();
        // port: a GNPRTB value of 0 reads as shipped, so the spread stays 6.
        world.gnprtb = crate::world::GnprtbParams::uniform(&[(6171, 3), (6158, 2)]);
        let here = system_at(&mut world, 0);
        let first = agent_at(&mut world, here, true);
        let second = agent_at(&mut world, here, true);
        let idle = agent_at(&mut world, here, true);
        for key in [first, second] {
            world.characters[key].ship_design = skill_pair(100);
        }
        world.characters[idle].troop_training = skill_pair(100);
        let team = [first, idle, second].map(MissionMember::Character).to_vec();

        let result = phase_ten(&world, MissionKind::ShipDesignResearch, team, here, None);

        assert_eq!(result.outcome, MissionOutcome::Success);
        assert_eq!(
            raised(&result),
            vec![
                (first, Skill::ShipDesign, 2),
                (second, Skill::ShipDesign, 2)
            ]
        );
        let points: Vec<_> = result
            .effects
            .iter()
            .filter_map(|effect| match effect {
                MissionEffect::ResearchPoints {
                    faction: MissionFaction::Alliance,
                    tech: TechType::Ship,
                    points,
                } => Some(*points),
                _ => None,
            })
            .collect();
        // Two successes of 3 + rand(0..=6) each, in one effect.
        assert_eq!(points.len(), 1);
        assert!((6..=18).contains(&points[0]), "{points:?}");
    }

    #[test]
    fn research_points_are_one_plus_a_draw_up_to_six_by_default() {
        // GNPRTB 6171 = 1 and 6172 = 6 as shipped: 1 + rand(0..=6).
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let designer = agent_at(&mut world, here, true);
        world.characters[designer].facility_design = skill_pair(100);
        let mut seen = std::collections::BTreeSet::new();
        for id in 0..200 {
            let mut mission = ActiveMission::timed(
                id,
                MissionKind::FacilityDesignResearch,
                MissionFaction::Alliance,
                vec![MissionMember::Character(designer)],
                here,
                1,
            );
            mission.target_character = None;
            let mut state = MissionState::new();
            state.push_timed(mission);
            let result = step(&mut state, &world, 1, &[]).results.remove(0);
            for effect in &result.effects {
                if let MissionEffect::ResearchPoints { tech, points, .. } = effect {
                    assert_eq!(*tech, TechType::Facility);
                    seen.insert(*points);
                }
            }
        }
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            (1..=7).collect::<Vec<_>>()
        );
    }

    #[test]
    fn research_with_no_success_fails_and_adds_no_points() {
        // FUN_0056cdb0 draws below a skill of 0 never; FUN_0056cf30 then
        // sets 2 with no points.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let designer = agent_at(&mut world, here, true);

        let result = phase_ten(
            &world,
            MissionKind::TroopTrainingResearch,
            vec![MissionMember::Character(designer)],
            here,
            None,
        );

        assert_eq!(result.outcome, MissionOutcome::Failure);
        assert!(result.effects.is_empty());
    }

    #[test]
    fn research_ends_without_a_working_yard_and_once_its_tree_is_done() {
        // FUN_0056c510 while running: no manager with an active yard of the
        // area (FUN_00509670 -> FUN_00528100(0)) ends with 0xc; the side's
        // Done flag (FUN_0052eb10) ends with 0xe, written after 0xc.
        let (mut world, here, designer) =
            dialog_world(vec![research_record(MissionKind::ShipDesignResearch)]);
        let mission = ActiveMission::new(
            0,
            MissionKind::ShipDesignResearch,
            MissionFaction::Alliance,
            vec![designer],
            here,
            0,
        );
        let mut research = ResearchState::default();
        let mut uprisings = UprisingState::default();
        let code = |world: &GameWorld, uprisings: &UprisingState, research: &ResearchState| {
            running_end_code(&mission, world, uprisings, research, &[])
        };

        assert_eq!(code(&world, &uprisings, &research), 0xc);
        yard_at(&mut world, here, 0x29);
        assert_eq!(code(&world, &uprisings, &research), 0xc);
        yard_at(&mut world, here, 0x28);
        assert_eq!(code(&world, &uprisings, &research), 0);

        uprisings.active_uprisings.insert(
            here,
            crate::uprising::ActiveUprising {
                started_tick: 0,
                next_incident_tick: None,
                next_support_tick: None,
                side: None,
            },
        );
        assert_eq!(code(&world, &uprisings, &research), 0xc);
        research.alliance_progress.ship.done = true;
        assert_eq!(code(&world, &uprisings, &research), 0xe);
        uprisings.active_uprisings.clear();
        assert_eq!(code(&world, &uprisings, &research), 0xe);
        research.alliance_progress.ship.done = false;
        research.empire_progress.ship.done = true;
        research.alliance_progress.troop.done = true;
        assert_eq!(code(&world, &uprisings, &research), 0);
    }

    #[test]
    fn the_dialog_lists_research_for_a_member_with_the_tree_skill_at_its_yard() {
        // FUN_0056c510 at creation: every character needs a nonzero skill in
        // the tree (FUN_0056c820, 0x40/4); while running, a yard of its area.
        let (mut world, here, designer) = dialog_world(vec![
            research_record(MissionKind::ShipDesignResearch),
            research_record(MissionKind::FacilityDesignResearch),
            research_record(MissionKind::TroopTrainingResearch),
        ]);
        let MissionMember::Character(key) = designer else {
            unreachable!()
        };
        let list = |world: &GameWorld| {
            available_kinds(
                world,
                &UprisingState::default(),
                &ResearchState::default(),
                MissionFaction::Alliance,
                &[designer],
                &[],
                here,
            )
        };
        yard_at(&mut world, here, 0x28);
        yard_at(&mut world, here, 0x2a);
        assert!(list(&world).is_empty());

        world.characters[key].ship_design = skill_pair(1);
        world.characters[key].troop_training = skill_pair(40);
        assert_eq!(list(&world), vec![MissionKind::ShipDesignResearch]);
        world.characters[key].facility_design = skill_pair(40);
        assert_eq!(
            list(&world),
            vec![
                MissionKind::ShipDesignResearch,
                MissionKind::FacilityDesignResearch
            ]
        );
    }

    #[test]
    fn a_research_order_with_an_unskilled_member_is_refused() {
        // FUN_0056c820 refuses a character whose skill in the tree is 0.
        let (mut world, here, designer) =
            dialog_world(vec![research_record(MissionKind::ShipDesignResearch)]);
        let MissionMember::Character(key) = designer else {
            unreachable!()
        };
        let order = MissionRequest::single(
            MissionKind::ShipDesignResearch,
            MissionFaction::Alliance,
            key,
            here,
            None,
            0,
        );
        let mut state = MissionState::new();
        assert_eq!(
            state.dispatch_guarded(order.clone(), &mut world),
            Err(MissionRefusal::MemberUnavailable(designer))
        );
        world.characters[key].ship_design = skill_pair(5);
        assert!(state.dispatch_guarded(order, &mut world).is_ok());
    }

    #[test]
    fn a_rescue_frees_its_target_once_and_raises_each_successful_member() {
        // FUN_0056ae50: each success raises combat (G6162) and calls the
        // target's slot +0x210; the port frees it once.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Rescue, 100);
        let (a, b) = (
            agent_at(&mut world, here, true),
            agent_at(&mut world, here, true),
        );
        let prisoner = agent_at(&mut world, here, true);
        world.characters[prisoner].is_captive = true;
        let team = vec![MissionMember::Character(a), MissionMember::Character(b)];

        let result = phase_ten(&world, MissionKind::Rescue, team, here, Some(prisoner));

        let freed = result
            .effects
            .iter()
            .filter(|effect| matches!(effect, MissionEffect::CharacterRescued { .. }))
            .count();
        assert_eq!(freed, 1);
        assert_eq!(
            raised(&result),
            vec![(a, Skill::Combat, 1), (b, Skill::Combat, 1)]
        );
    }

    #[test]
    fn an_abduction_takes_its_target_prisoner_for_the_mission_s_side() {
        // FUN_00576eb0: the target's slot +0x20c with the member as captor.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Abduction, 100);
        let agent = agent_at(&mut world, here, true);
        let target = agent_at(&mut world, here, false);

        let result = phase_ten(
            &world,
            MissionKind::Abduction,
            vec![MissionMember::Character(agent)],
            here,
            Some(target),
        );

        assert_eq!(
            result.effects,
            vec![
                MissionEffect::SkillRaised {
                    character: agent,
                    skill: Skill::Combat,
                    amount: 1
                },
                MissionEffect::CharacterCaptured {
                    character: target,
                    captured_by: MissionFaction::Alliance,
                    at_system: here,
                },
            ]
        );
    }

    #[test]
    fn a_character_dead_of_its_injury_leaves_its_fleet_and_abode() {
        // Slot +0x2f0's destruction: the port marks the character killed
        // and takes it off every roster.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let victim = agent_at(&mut world, here, true);
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: here,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![victim],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.characters[victim].current_fleet = Some(fleet);
        world.characters[victim].current_abode = Some(crate::ids::AbodeKey::default());
        apply_advance_effects(
            &mut world,
            &[MissionEffect::DiedOfInjury { character: victim }],
        );
        let c = &world.characters[victim];
        assert!(c.is_killed);
        assert_eq!(
            (c.current_fleet, c.current_abode, c.current_system),
            (None, None, None)
        );
        assert!(world.fleets[fleet].characters.is_empty());
    }

    #[test]
    fn an_assassination_only_injures_a_major_character() {
        // FUN_00576620: each successful member injures the target (slot
        // +0x2ec, FUN_004ef690); a major character's slot +0x2e0
        // (FUN_006158b0) never lets the injury kill, so the target is not
        // destroyed, the result stays unset (2) and no one is raised.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Assassination, 100);
        let (a, b) = (
            agent_at(&mut world, here, true),
            agent_at(&mut world, here, true),
        );
        let target = agent_at(&mut world, here, false);
        world.characters[target].is_major = true;
        let team = vec![MissionMember::Character(a), MissionMember::Character(b)];

        let result = phase_ten(&world, MissionKind::Assassination, team, here, Some(target));

        let injuries: Vec<i32> = result
            .effects
            .iter()
            .filter_map(|effect| match effect {
                MissionEffect::CharacterInjured { character, injury } if *character == target => {
                    Some(*injury)
                }
                _ => None,
            })
            .collect();
        // FUN_0053eae0: chance 2574 (100), rand(0..=100) + rand(0..=29) + 1.
        assert_eq!(injuries.len(), 2);
        assert!(injuries.iter().all(|injury| (1..=130).contains(injury)));
        assert!(!result
            .effects
            .iter()
            .any(|effect| matches!(effect, MissionEffect::CharacterKilled { .. })));
        assert!(raised(&result).is_empty());
        assert_eq!(result.outcome, MissionOutcome::Failure);
    }

    #[test]
    fn an_assassination_kills_a_minor_character_its_injury_kills() {
        // FUN_0056f160 -> FUN_0053eb30: a minor character dies of an
        // injury with GNPRTB 2577 (100 here); once it is destroyed every
        // success sets 3 and raises combat (G6164), and no one injures it
        // again.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Assassination, 100);
        world.gnprtb = crate::world::GnprtbParams::uniform(&[(2577, 100)]);
        let (a, b) = (
            agent_at(&mut world, here, true),
            agent_at(&mut world, here, true),
        );
        let target = agent_at(&mut world, here, false);
        let team = vec![MissionMember::Character(a), MissionMember::Character(b)];

        let result = phase_ten(&world, MissionKind::Assassination, team, here, Some(target));

        let count = |f: fn(&MissionEffect) -> bool| result.effects.iter().filter(|e| f(e)).count();
        assert_eq!(
            count(|e| matches!(e, MissionEffect::CharacterInjured { .. })),
            1
        );
        assert_eq!(
            count(|e| matches!(e, MissionEffect::CharacterKilled { .. })),
            1
        );
        assert_eq!(
            raised(&result),
            vec![(a, Skill::Combat, 1), (b, Skill::Combat, 1)]
        );
        assert_eq!(result.outcome, MissionOutcome::Success);
    }

    #[test]
    fn an_incite_success_runs_the_incident_and_fails_while_the_holder_keeps_a_regiment() {
        // FUN_00571a60: FUN_0050d030 runs, then the result is 2 while the
        // holder has a regiment there (FUN_00509020), else 3 with a
        // leadership raise (G6160).
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        held_by(&mut world, here, crate::dat::Faction::Empire);
        always(&mut world, MissionKind::InciteUprising, 100);
        let agitator = agent_at(&mut world, here, true);
        let team = || vec![MissionMember::Character(agitator)];
        let incidents = |result: &MissionResult| {
            result
                .effects
                .iter()
                .filter(|effect| matches!(effect, MissionEffect::UprisingIncident(_)))
                .count()
        };

        let open = phase_ten(&world, MissionKind::InciteUprising, team(), here, None);
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: crate::ids::DatId::new(0x1000_0007),
            is_alliance: false,
            regiment_strength: 1,
        });
        world.systems[here].ground_units.push(troop);
        let garrisoned = phase_ten(&world, MissionKind::InciteUprising, team(), here, None);

        assert_eq!(open.outcome, MissionOutcome::Success);
        assert_eq!(raised(&open), vec![(agitator, Skill::Leadership, 1)]);
        assert_eq!(garrisoned.outcome, MissionOutcome::Failure);
        assert!(raised(&garrisoned).is_empty());
        assert_eq!((incidents(&open), incidents(&garrisoned)), (1, 1));
    }

    #[test]
    fn a_subdue_member_rolls_only_during_an_uprising_and_succeeds_once_it_ends() {
        // FUN_00569c20: nothing without the uprising bit (+0x88 bit 2); a
        // success adds FUN_0055cb10 support, runs FUN_0050c910, and sets 3
        // only once the uprising ended.
        let mut world = minimal_world();
        world.gnprtb =
            crate::world::GnprtbParams::uniform(&[(6187, 1), (6188, 0), (7761, 60), (7762, -10)]);
        let here = system_at(&mut world, 0);
        held_by(&mut world, here, crate::dat::Faction::Alliance);
        world.systems[here].popularity_alliance = 0.3;
        world.systems[here].popularity_empire = 0.7;
        always(&mut world, MissionKind::SubdueUprising, 100);
        let governor = agent_at(&mut world, here, true);
        let run = |world: &GameWorld, uprisings: &UprisingState| {
            let mut state = MissionState::new();
            state.push_timed(ActiveMission::timed(
                0,
                MissionKind::SubdueUprising,
                MissionFaction::Alliance,
                vec![MissionMember::Character(governor)],
                here,
                1,
            ));
            MissionSystem::advance(
                &mut state,
                world,
                uprisings,
                &ResearchState::default(),
                &[TickEvent { tick: 1 }],
                &[],
            )
            .results
            .remove(0)
        };
        let calm = UprisingState::default();
        let mut revolt = UprisingState::default();
        revolt.active_uprisings.insert(
            here,
            crate::uprising::ActiveUprising {
                started_tick: 0,
                next_incident_tick: None,
                next_support_tick: None,
                side: None,
            },
        );

        let quiet = run(&world, &calm);
        let unended = run(&world, &revolt);
        for _ in 0..3 {
            let troop = world.troops.insert(crate::world::TroopUnit {
                class_dat_id: crate::ids::DatId::new(0x1000_0001),
                is_alliance: true,
                regiment_strength: 1,
            });
            world.systems[here].ground_units.push(troop);
        }
        let ended = run(&world, &revolt);

        assert!(quiet.effects.is_empty());
        assert_eq!(quiet.outcome, MissionOutcome::Failure);
        assert_eq!(unended.outcome, MissionOutcome::Failure);
        assert_eq!(
            unended.effects,
            vec![MissionEffect::UprisingSubdued {
                system: here,
                side: crate::dat::Faction::Alliance,
                support_gain: 1,
            }]
        );
        assert_eq!(ended.outcome, MissionOutcome::Success);
        assert_eq!(raised(&ended), vec![(governor, Skill::Leadership, 1)]);
    }

    #[test]
    fn an_incite_whose_incident_destroys_the_holder_s_last_regiment_succeeds() {
        // FUN_00571a60 tests the holder's regiments after FUN_0050d030: here
        // UPRIS1TB gives code 2 (FUN_0050d150 destroys a regiment), so none
        // is left and the result is 3.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        held_by(&mut world, here, crate::dat::Faction::Empire);
        always(&mut world, MissionKind::InciteUprising, 100);
        world.mission_tables.insert(
            "UPRIS1TB".into(),
            crate::world::MstbTable::new(vec![crate::world::MstbEntry {
                threshold: i32::MIN,
                value: 2,
            }]),
        );
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: crate::ids::DatId::new(0x1000_0007),
            is_alliance: false,
            regiment_strength: 1,
        });
        world.systems[here].ground_units.push(troop);
        let agitator = agent_at(&mut world, here, true);

        let result = phase_ten(
            &world,
            MissionKind::InciteUprising,
            vec![MissionMember::Character(agitator)],
            here,
            None,
        );

        assert_eq!(result.outcome, MissionOutcome::Success);
        assert!(result.effects.iter().any(|effect| matches!(
            effect,
            MissionEffect::UprisingIncident(crate::uprising::UprisingEvent::UprisingIncident {
                losses,
                ..
            }) if losses == &[crate::uprising::IncidentLoss::Regiment(troop)]
        )));
    }

    #[test]
    fn both_sabotages_raise_espionage_and_combat_by_their_own_gnprtb() {
        // FUN_0056a300: G6165 and G6166; FUN_00574630: G6167 and G6168.
        let mut world = minimal_world();
        world.gnprtb =
            crate::world::GnprtbParams::uniform(&[(6165, 2), (6166, 3), (6167, 4), (6168, 5)]);
        let here = system_at(&mut world, 0);
        let saboteur = agent_at(&mut world, here, true);
        let mut raises = Vec::new();
        for kind in [MissionKind::Sabotage, MissionKind::DeathStarSabotage] {
            always(&mut world, kind, 100);
            let team = vec![MissionMember::Character(saboteur)];
            raises.push(raised(&phase_ten(&world, kind, team, here, None)));
        }

        assert_eq!(
            raises,
            vec![
                vec![
                    (saboteur, Skill::Espionage, 2),
                    (saboteur, Skill::Combat, 3)
                ],
                vec![
                    (saboteur, Skill::Espionage, 4),
                    (saboteur, Skill::Combat, 5)
                ],
            ]
        );
    }

    #[test]
    fn a_raise_reads_its_gnprtb_value_and_adds_to_the_base_skill() {
        // FUN_005740a0 adds GNPRTB 6156 to the base diplomacy (+0x58) through
        // FUN_00533e20.
        let mut world = minimal_world();
        world.gnprtb = crate::world::GnprtbParams::uniform(&[(6156, 3)]);
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Diplomacy, 100);
        let envoy = agent_at(&mut world, here, true);
        world.characters[envoy].diplomacy = skill_pair(40);

        let result = phase_ten(
            &world,
            MissionKind::Diplomacy,
            vec![MissionMember::Character(envoy)],
            here,
            None,
        );
        assert_eq!(raised(&result), vec![(envoy, Skill::Diplomacy, 3)]);
        raise_skill(&mut world, envoy, Skill::Diplomacy, 3);

        assert_eq!(world.characters[envoy].diplomacy.base, 43);
    }

    // --- Slot +0x274, the phase 10 chance ---

    /// A table whose row `t` holds `t + 200`, so the chance reads back its
    /// input.
    fn echo_table(world: &mut GameWorld, name: &str) {
        let rows = (-200..=200)
            .map(|threshold| crate::world::MstbEntry {
                threshold,
                value: u32::try_from(threshold + 200).unwrap(),
            })
            .collect();
        world
            .mission_tables
            .insert(name.to_string(), crate::world::MstbTable::new(rows));
    }

    fn character_with_skills(
        world: &mut GameWorld,
        espionage: u32,
        combat: u32,
        diplomacy: u32,
        leadership: u32,
    ) -> MissionMember {
        MissionMember::Character(world.characters.insert(Character {
            name: "Agent".into(),
            is_alliance: true,
            diplomacy: skill_pair(diplomacy),
            espionage: skill_pair(espionage),
            combat: skill_pair(combat),
            leadership: skill_pair(leadership),
            can_be_commander: true,
            ..Default::default()
        }))
    }

    #[test]
    fn each_class_reads_its_recovered_table_input() {
        // Slot +0x274 per class (decoy-roll.md, "The success roll"): an
        // Empire-held system at 60 Empire support with two Stormtrooper
        // regiments (FUN_005091f0) against an Alliance member.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        world.systems[here].popularity_empire = 0.6;
        world.systems[here].popularity_alliance = 0.4;
        world.systems[here].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        for _ in 0..2 {
            let troop = world.troops.insert(crate::world::TroopUnit {
                class_dat_id: crate::ids::DatId::new(0x1000_0006),
                is_alliance: false,
                regiment_strength: 1,
            });
            world.systems[here].ground_units.push(troop);
        }
        for kind in [
            MissionKind::Diplomacy,
            MissionKind::Espionage,
            MissionKind::Rescue,
            MissionKind::Sabotage,
            MissionKind::DeathStarSabotage,
            MissionKind::Recruitment,
            MissionKind::InciteUprising,
            MissionKind::SubdueUprising,
            MissionKind::Abduction,
            MissionKind::Assassination,
        ] {
            echo_table(&mut world, kind.mstb_key().unwrap());
        }
        let agent = character_with_skills(&mut world, 60, 40, 80, 70);
        let target = character_with_skills(&mut world, 0, 25, 0, 0).character();
        let chance =
            |kind| member_chance(&world, kind, MissionFaction::Alliance, here, target, agent) - 200;

        // FUN_0055c680: (stormtroopers - opposing support) + diplomacy.
        assert_eq!(chance(MissionKind::Diplomacy), 2 - 60 + 80);
        assert_eq!(chance(MissionKind::Espionage), 60);
        assert_eq!(chance(MissionKind::Rescue), 40);
        // FUN_0055c890 / FUN_0055c8d0: (espionage + combat) / 2.
        assert_eq!(chance(MissionKind::Sabotage), 50);
        assert_eq!(chance(MissionKind::DeathStarSabotage), 50);
        // FUN_0055c700: leadership - opposing support.
        assert_eq!(chance(MissionKind::Recruitment), 70 - 60);
        // FUN_0055c740: (leadership - opposing support) - stormtroopers.
        assert_eq!(chance(MissionKind::InciteUprising), 70 - 60 - 2);
        // FUN_0055c780: (stormtroopers - opposing support) + leadership.
        assert_eq!(chance(MissionKind::SubdueUprising), 2 - 60 + 70);
        // FUN_0055c810 / FUN_0055c850: combat - target combat.
        assert_eq!(chance(MissionKind::Abduction), 40 - 25);
        assert_eq!(chance(MissionKind::Assassination), 40 - 25);
    }

    #[test]
    fn a_chance_without_its_table_system_or_target_is_zero() {
        // FUN_0053e240 leaves 0 without a row; FUN_00586720 and FUN_00586c80
        // leave the chance unset without a target.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let elsewhere = system_at(&mut world, 1);
        world.systems.remove(elsewhere);
        let agent = character_with_skills(&mut world, 60, 40, 80, 70);
        let chance = |world: &GameWorld, kind, system| {
            member_chance(world, kind, MissionFaction::Alliance, system, None, agent)
        };
        assert_eq!(chance(&world, MissionKind::Espionage, here), 0);
        echo_table(&mut world, "DIPLMSTB");
        echo_table(&mut world, "ASSNMSTB");
        assert_eq!(chance(&world, MissionKind::Diplomacy, elsewhere), 0);
        assert_eq!(chance(&world, MissionKind::Assassination, here), 0);
        assert_eq!(chance(&world, MissionKind::Diplomacy, here), 200 - 50 + 80);
    }

    #[test]
    fn a_special_force_rolls_on_its_own_skills() {
        // Slots +0x1e0 and +0x1f0 of vtable 0x0065e160 read the unit's base
        // espionage and combat (FUN_00503c40, FUN_00503c80).
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        echo_table(&mut world, "SBTGMSTB");
        let mut skills = [0; 8];
        skills[Skill::Espionage as usize] = 30;
        skills[Skill::Combat as usize] = 50;
        let unit = world.special_forces.insert(crate::world::SpecialForceUnit {
            class_dat_id: crate::ids::DatId::new(0x1400_0001),
            is_alliance: true,
            skills,
            on_mission: false,
        });
        let chance = member_chance(
            &world,
            MissionKind::Sabotage,
            MissionFaction::Alliance,
            here,
            None,
            MissionMember::SpecialForce(unit),
        );
        assert_eq!(chance, 200 + 40);
    }
    // --- Lifecycle (FUN_005227d0, FUN_00524b70, FUN_00522480) ---

    /// A system at `(x, 0)`, populated, held by no one.
    fn system_at(world: &mut GameWorld, x: u16) -> SystemKey {
        world.systems.insert(crate::world::System {
            last_arrival: None,
            dat_id: crate::ids::DatId::new(0x9000_0001),
            name: "System".into(),
            sector: crate::ids::SectorKey::default(),
            x,
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
            control: crate::world::ControlKind::Uncontrolled,
        })
    }

    /// The shipped MISSNSD Diplomacy record `0x51000010`: timer (5, 10),
    /// repeating, detection phases and resign on, columns 11..21 =
    /// 1 1 1 1 1 0 0 1 0 0.
    fn diplomacy_record() -> crate::world::MissionRecord {
        crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x5100_0010),
            timer_min_days: 5,
            timer_spread_days: 10,
            repeats: true,
            hidden: false,
            detection_phases: true,
            can_resign: true,
            leave_rule: 0,
            rules: crate::world::MissionTargetRules {
                container_loss_ends: true,
                needs_populated_container: true,
                target_loss_ends: true,
                own_side_target: true,
                other_side_target: true,
                opponent_target: false,
                revolting_target: false,
                calm_target: true,
                prisoner_target: false,
                free_target: false,
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0,
                character_mask: 0x1_0000,
            },
        }
    }

    /// The shipped MISSNSD Rescue record `0x61000011`: timer (1, 6), not
    /// repeating, detection phases and resign on, columns 11..21 =
    /// 0 0 1 0 0 1 0 0 1 0.
    fn rescue_record() -> crate::world::MissionRecord {
        crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x6100_0011),
            timer_min_days: 1,
            timer_spread_days: 6,
            repeats: false,
            hidden: false,
            detection_phases: true,
            can_resign: true,
            leave_rule: 0,
            rules: crate::world::MissionTargetRules {
                target_loss_ends: true,
                opponent_target: true,
                prisoner_target: true,
                ..Default::default()
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0x802,
                character_mask: 0x1_0000,
            },
        }
    }

    fn step(
        state: &mut MissionState,
        world: &GameWorld,
        tick: u64,
        rolls: &[f64],
    ) -> MissionAdvance {
        MissionSystem::advance(
            state,
            world,
            &UprisingState::default(),
            &ResearchState::default(),
            &[TickEvent { tick }],
            rolls,
        )
    }

    fn diplomacy_from(
        world: &mut GameWorld,
        from: SystemKey,
        to: SystemKey,
    ) -> (MissionState, CharacterKey) {
        world.mission_records = vec![diplomacy_record()];
        let envoy = agent_at(world, from, true);
        let mut state = MissionState::new();
        state
            .dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Diplomacy,
                    MissionFaction::Alliance,
                    envoy,
                    to,
                    None,
                    0,
                ),
                world,
            )
            .expect("a free envoy");
        (state, envoy)
    }

    /// A one-shot Diplomacy that ends on day 5 at `away` (no MSTB table, so
    /// it fails), its record's leave rule `leave_rule`; the envoy's home
    /// side holds `home`.
    fn ending_at(
        away_holder: crate::dat::Faction,
        leave_rule: u32,
    ) -> (GameWorld, MissionState, CharacterKey, SystemKey, SystemKey) {
        use crate::world::ControlKind;
        let mut world = minimal_world();
        let home = system_at(&mut world, 0);
        let away = system_at(&mut world, 100);
        world.systems[home].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        world.systems[away].control = ControlKind::Controlled(away_holder);
        world.mission_records = vec![crate::world::MissionRecord {
            repeats: false,
            leave_rule,
            ..diplomacy_record()
        }];
        let envoy = agent_at(&mut world, away, true);
        world.characters[envoy].on_mission = true;
        let mut state = MissionState::new();
        state.missions.push_back(ActiveMission::timed(
            0,
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            vec![MissionMember::Character(envoy)],
            away,
            5,
        ));
        state.next_id = 1;
        (world, state, envoy, home, away)
    }

    #[test]
    fn an_agent_returns_home_from_another_side_s_system_on_a_return_order() {
        // FUN_00521220: record +0x68 = 1 sends home a member of an ended
        // mission whose container is not of its side; FUN_00548910 gives it
        // a Return order 0x42000002 (FUN_005475f0) to FUN_00542ee0's
        // destination, the nearest system of its side (FUN_00554e80). The
        // order steps at once: phase 4 sends the envoy off the map.
        let (world, mut state, envoy, home, _) = ending_at(crate::dat::Faction::Empire, 1);
        let member = MissionMember::Character(envoy);

        let advance = step(&mut state, &world, 5, &[]);

        assert_eq!(advance.ended.len(), 1);
        assert!(!advance
            .effects
            .contains(&MissionEffect::MemberAvailable { member }));
        assert!(advance
            .effects
            .contains(&MissionEffect::MemberMoved { member, to: None }));
        let order = &state.missions()[0];
        assert_eq!(
            (order.kind, order.team.clone(), order.target_system, order.phase),
            (MissionKind::Return, vec![member], home, PHASE_TRANSIT)
        );
    }

    #[test]
    fn an_agent_returns_to_where_it_joined_the_mission_while_its_side_holds_it() {
        // FUN_00542ee0 tries the member's +0x70, the container it joined the
        // mission from (Role_vt268 FUN_00536490), before the nearest search;
        // a Return needs its own side's target (FUN_004f4b60, 1/0x28). The
        // manual's "base" (pp. 110-111).
        use crate::world::ControlKind;
        for (base_holder, expected_far) in [
            (crate::dat::Faction::Alliance, true),
            (crate::dat::Faction::Empire, false),
        ] {
            let (mut world, mut state, _, home, _) =
                ending_at(crate::dat::Faction::Empire, 1);
            let base = system_at(&mut world, 400);
            world.systems[base].control = ControlKind::Controlled(base_holder);
            state.missions.back_mut().expect("the mission").origin = Some(base);

            step(&mut state, &world, 5, &[]);

            let to = state.missions()[0].target_system;
            assert_eq!(to, if expected_far { base } else { home }, "{base_holder:?}");
        }
    }

    #[test]
    fn an_agent_on_its_own_side_s_system_or_under_rule_zero_stays_where_it_is() {
        // FUN_00521220: rule 1 returns only from another side's container;
        // rule 0 (the movement orders) never returns.
        for (holder, rule) in [
            (crate::dat::Faction::Alliance, 1),
            (crate::dat::Faction::Empire, 0),
        ] {
            let (world, mut state, envoy, _, _) = ending_at(holder, rule);
            let advance = step(&mut state, &world, 5, &[]);
            assert!(advance.effects.contains(&MissionEffect::MemberAvailable {
                member: MissionMember::Character(envoy)
            }));
            assert!(state.missions().is_empty(), "{holder:?} rule {rule}");
        }
    }

    #[test]
    fn members_sharing_a_destination_share_one_return_order() {
        // FUN_00548910 gathers every member whose destination
        // (FUN_00542ee0) is the first one's, one Return order per group; a
        // member with a remove request (+0x78 bit 2) or held on the
        // captured list does not go.
        use crate::world::ControlKind;
        let (mut world, mut state, envoy, home, away) = ending_at(crate::dat::Faction::Empire, 1);
        let far = system_at(&mut world, 250);
        world.systems[far].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let partner = agent_at(&mut world, away, true);
        let other = agent_at(&mut world, far, true);
        world.systems[far].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let near_far = system_at(&mut world, 300);
        world.systems[near_far].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let removed = agent_at(&mut world, away, true);
        let prisoner = agent_at(&mut world, away, true);
        let mission = state.missions.back_mut().expect("the mission");
        let [envoy, partner, other, removed, prisoner] =
            [envoy, partner, other, removed, prisoner].map(MissionMember::Character);
        mission.team.extend([removed, other]);
        mission.decoys.push(partner);
        mission.removing.push(removed);
        mission.captured.push(prisoner);

        let advance = step(&mut state, &world, 5, &[]);

        let orders: Vec<_> = state
            .missions()
            .iter()
            .map(|m| (m.id, m.team.clone(), m.target_system))
            .collect();
        assert_eq!(
            orders,
            vec![(1, vec![envoy, partner], home), (2, vec![other], near_far)]
        );
        for member in [removed, prisoner] {
            assert!(advance
                .effects
                .contains(&MissionEffect::MemberAvailable { member }));
        }
    }

    #[test]
    fn a_return_order_s_members_are_put_on_its_hidden_mission() {
        // FUN_00522b30 sets RoleOnMissionNotif; FUN_00520b70 reads the
        // record's hidden flag +0x5c into OnHiddenMission.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let envoy = agent_at(&mut world, here, true);
        let member = MissionMember::Character(envoy);
        apply_advance_effects(
            &mut world,
            &[MissionEffect::MemberOnMission {
                member,
                hidden: true,
            }],
        );
        let c = &world.characters[envoy];
        assert!(c.on_mission && c.on_hidden_mission);
    }

    #[test]
    fn a_moves_days_follow_the_travel_rule_and_han_solo_s_speed() {
        // FUN_004f4850 lists the trip's days; FUN_00548370 gives a free Han
        // Solo GNPRTB 3083's speed. isqrt(100^2) / GNPRTB 5120 (5) * 100 /
        // 100 = 20 days at the default speed.
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let agent = agent_at(&mut world, from, true);
        assert_eq!(move_days(&world, agent, to), Some(20));
        assert_eq!(move_days(&world, agent, from), Some(0));
        world.characters[agent].dat_id = crate::ids::DatId::new(HAN_SOLO);
        let han = move_days(&world, agent, to).expect("a trip");
        assert!(han < 20, "Han travels faster: {han}");
        world.characters[agent].current_system = None;
        assert_eq!(move_days(&world, agent, to), None);
    }

    #[test]
    fn a_special_force_moves_at_the_default_speed() {
        // FUN_004f4850 for a unit: the same trip, at the default speed
        // (FUN_00548370 speeds up only Han Solo).
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let unit = world.special_forces.insert(crate::world::SpecialForceUnit {
            class_dat_id: crate::ids::DatId::new(0x3c00_0001),
            is_alliance: true,
            skills: [0; 8],
            on_mission: false,
        });
        let member = MissionMember::SpecialForce(unit);
        assert_eq!(member_move_days(&world, member, to), None);
        world.systems[from].special_forces.push(unit);
        assert_eq!(member_move_days(&world, member, to), Some(20));
    }

    #[test]
    fn a_move_order_targets_only_a_system_its_side_holds() {
        // FUN_004f4b60: a movement kind (family 0x40..0x4f) whose target's
        // side bits differ from the order's side is refused (1/0x28).
        use crate::world::ControlKind;
        let mut world = minimal_world();
        let own = system_at(&mut world, 0);
        let enemy = system_at(&mut world, 10);
        let open = system_at(&mut world, 20);
        world.systems[own].control = ControlKind::Uprising(crate::dat::Faction::Alliance);
        world.systems[enemy].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        world.systems[open].control = ControlKind::Contested;
        let allowed = |to| move_target_allowed(&world, MissionFaction::Alliance, to);
        assert!(allowed(own));
        assert!(!allowed(enemy));
        assert!(!allowed(open));
        assert!(move_target_allowed(&world, MissionFaction::Empire, enemy));
    }

    #[test]
    fn the_return_order_ends_on_arrival_and_frees_its_member() {
        // The Return's base phase handler FUN_00524b70 rolls no one; its
        // record times 0 days, so it ends once its member lands.
        let (mut world, mut state, envoy, home, _) =
            ending_at(crate::dat::Faction::Empire, 1);
        world.mission_records.push(crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x4200_0002),
            ..move_record()
        });
        let member = MissionMember::Character(envoy);
        let advance = step(&mut state, &world, 5, &[]);
        // FUN_00520b70: the record is hidden, as Move's.
        assert!(advance
            .effects
            .contains(&MissionEffect::MemberOnMission { member, hidden: true }));
        let arrival = state.en_route()[0].arrival;

        let advance = step(&mut state, &world, arrival, &[]);
        assert!(advance.effects.contains(&MissionEffect::MemberMoved {
            member,
            to: Some(home)
        }));
        // FUN_00586130 makes the 0-day timer at least one tick.
        let advance = step(&mut state, &world, arrival + 1, &[]);
        assert!(advance
            .effects
            .contains(&MissionEffect::MemberAvailable { member }));
        assert_eq!(advance.ended[0].kind, MissionKind::Return);
        assert!(state.missions().is_empty());
    }

    #[test]
    fn a_new_mission_steps_to_the_transit_phase_and_sends_its_members_off_the_map() {
        // FUN_00524b70 phase 4: FUN_00556430 starts each member's transit
        // from the origin +0x6c; the phase waits for the arrivals.
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let (mut state, envoy) = diplomacy_from(&mut world, from, to);

        let advance = step(&mut state, &world, 1, &[]);

        let mission = &state.missions()[0];
        assert_eq!((mission.phase, mission.ready), (PHASE_TRANSIT, false));
        assert_eq!(mission.origin, Some(from));
        let member = MissionMember::Character(envoy);
        assert_eq!(
            advance.effects,
            vec![MissionEffect::MemberMoved { member, to: None }]
        );
        // isqrt(100^2) / GNPRTB 5120 (5) * 100 / 100 = 20 days from day 0.
        assert_eq!(
            state.en_route(),
            &[MemberTransit {
                member,
                mission_id: mission.id,
                to,
                arrival: 20,
                abode: None,
            }]
        );
        assert_eq!(
            mission.days_left(1, state.last_arrival(mission.id)),
            Some(19)
        );
    }

    #[test]
    fn the_mission_waits_for_its_last_arrival_then_arms_the_record_timer() {
        // FUN_00545820 -> FUN_00522280 sets the ready bit once no member is
        // en route; phase 8 arms timer 0x38b for min + rand(0..=spread)
        // (FUN_005236e0, FUN_00586130): 5 + floor(0.3 * 11) = 8 days.
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let (mut state, envoy) = diplomacy_from(&mut world, from, to);
        step(&mut state, &world, 1, &[]);

        assert!(step(&mut state, &world, 19, &[]).effects.is_empty());
        assert_eq!(state.missions()[0].phase, PHASE_TRANSIT);

        let advance = step(&mut state, &world, 20, &[0.3]);
        assert_eq!(
            advance.effects,
            vec![MissionEffect::MemberMoved {
                member: MissionMember::Character(envoy),
                to: Some(to)
            }]
        );
        assert!(state.en_route().is_empty());
        let mission = &state.missions()[0];
        assert_eq!(mission.phase, PHASE_TIMER);
        assert_eq!(mission.timer_due, Some(28));
    }

    /// MISSNSD `0x41000001` (Move) as shipped: both sides, special forces
    /// `0xf0f`, characters `0x10000`, timer 0 + rand(0..=0), hidden,
    /// detection on, no resigning; only an own-side target, ended by the
    /// container's or the target's loss.
    fn move_record() -> crate::world::MissionRecord {
        crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x4100_0001),
            timer_min_days: 0,
            timer_spread_days: 0,
            repeats: false,
            hidden: true,
            detection_phases: true,
            can_resign: false,
            leave_rule: 0,
            rules: crate::world::MissionTargetRules {
                container_loss_ends: true,
                target_loss_ends: true,
                own_side_target: true,
                ..Default::default()
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0xf0f,
                character_mask: 0x1_0000,
            },
        }
    }

    #[test]
    fn a_move_takes_its_member_to_the_target_and_ends_with_no_result() {
        // The Move class (vtable 0x665578, FUN_0056ebc0 = 0x41) steps
        // phases with the base handler FUN_00524b70, which rolls no one in
        // phase 10; its record's timer is 0 days. The member stays where
        // the trip took it (FUN_00556390).
        let mut world = minimal_world();
        world.mission_records = vec![move_record()];
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        world.systems[to].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Alliance);
        let traveller = agent_at(&mut world, from, true);
        let mut state = MissionState::new();
        state
            .dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Move,
                    MissionFaction::Alliance,
                    traveller,
                    to,
                    None,
                    0,
                ),
                &mut world,
            )
            .expect("a free traveller");
        assert!(world.characters[traveller].on_hidden_mission);
        step(&mut state, &world, 1, &[]);

        let advance = step(&mut state, &world, 20, &[0.5]);
        assert_eq!(
            advance.effects,
            vec![MissionEffect::MemberMoved {
                member: MissionMember::Character(traveller),
                to: Some(to)
            }]
        );
        // FUN_00586130 makes the 0-day timer at least one tick.
        assert_eq!(state.missions()[0].timer_due, Some(21));

        let advance = step(&mut state, &world, 21, &[0.0]);
        assert!(advance.results.is_empty());
        assert_eq!(advance.ended.len(), 1);
        assert_eq!(advance.ended[0].end_code, 1);
        assert!(advance.effects.contains(&MissionEffect::MemberAvailable {
            member: MissionMember::Character(traveller)
        }));
    }

    /// A Move of an Alliance agent at x 0 to an Alliance fleet at the
    /// enemy's system at x 100 (`0x241` with the fleet as its target).
    fn move_to_fleet() -> (GameWorld, MissionState, CharacterKey, crate::ids::FleetKey, SystemKey) {
        let mut world = minimal_world();
        world.mission_records = vec![move_record()];
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        world.systems[from].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Alliance);
        world.systems[to].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: to,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        let traveller = agent_at(&mut world, from, true);
        let mut state = MissionState::new();
        let id = state
            .dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Move,
                    MissionFaction::Alliance,
                    traveller,
                    to,
                    None,
                    0,
                ),
                &mut world,
            )
            .expect("a free traveller");
        assert!(state.set_target_fleet(id, fleet));
        (world, state, traveller, fleet, to)
    }

    #[test]
    fn a_move_to_a_fleet_lands_its_member_aboard() {
        // FUN_004f7640 lands the member in the order's container +0x74, the
        // fleet; the validator reads the fleet's side (FUN_00523450 rule 3),
        // so an own fleet at the enemy's system is a fit target.
        let (mut world, mut state, traveller, fleet, to) = move_to_fleet();
        let member = MissionMember::Character(traveller);
        let advance = step(&mut state, &world, 1, &[]);
        apply_advance_effects(&mut world, &advance.effects);

        let advance = step(&mut state, &world, 20, &[0.5]);
        assert_eq!(
            advance.effects,
            vec![
                MissionEffect::MemberMoved {
                    member,
                    to: Some(to)
                },
                MissionEffect::MemberBoarded { member, fleet },
            ]
        );
        apply_advance_effects(&mut world, &advance.effects);
        assert_eq!(world.characters[traveller].current_fleet, Some(fleet));
        assert_eq!(world.fleets[fleet].characters, vec![traveller]);

        let advance = step(&mut state, &world, 21, &[0.0]);
        assert_eq!(advance.ended[0].end_code, 1);
    }

    #[test]
    fn a_destroyed_fleets_free_characters_go_adrift_where_it_stood() {
        // FUN_00556710 -> FUN_004ef2e0: a character on no mission goes
        // adrift; one on a mission (+0x78 bit 7) is left as it is. All stand
        // at the fleet's system.
        let (mut world, _, traveller, fleet, to) = move_to_fleet();
        let busy = agent_at(&mut world, to, true);
        world.characters[busy].on_mission = true;
        let free = agent_at(&mut world, to, true);
        for key in [busy, free] {
            world.characters[key].current_fleet = Some(fleet);
            world.fleets[fleet].characters.push(key);
        }

        cast_adrift(&mut world, fleet);

        assert!(world.characters[free].adrift);
        assert!(!world.characters[busy].adrift);
        for key in [busy, free] {
            let c = &world.characters[key];
            assert_eq!((c.current_fleet, c.current_system), (None, Some(to)));
        }
        assert!(!world.characters[traveller].adrift);
    }

    #[test]
    fn a_special_force_boards_a_fleet_and_goes_where_it_goes() {
        // FUN_004f7640 lands a Move's member in the container +0x74, the
        // fleet, for a special force as for a character; the Fleet window
        // then lists it aboard (FUN_00536da0(ship, 3)).
        let (mut world, _, _, fleet, to) = move_to_fleet();
        let from = world.fleets[fleet].location;
        let unit = world.special_forces.insert(crate::world::SpecialForceUnit {
            class_dat_id: crate::ids::DatId::new(0x3c00_0004),
            is_alliance: true,
            skills: [0; 8],
            on_mission: false,
        });
        world.systems[to].special_forces.push(unit);
        let member = MissionMember::SpecialForce(unit);
        // A fleet gone before the landing takes no one; the unit stays.
        let gone = world.fleets.insert(world.fleets[fleet].clone());
        world.fleets.remove(gone);
        apply_advance_effects(
            &mut world,
            &[MissionEffect::MemberBoarded {
                member,
                fleet: gone,
            }],
        );
        assert_eq!(world.systems[to].special_forces, vec![unit]);

        apply_advance_effects(
            &mut world,
            &[MissionEffect::MemberBoarded { member, fleet }],
        );

        assert_eq!(world.fleets[fleet].special_forces, vec![unit]);
        assert!(world
            .systems
            .values()
            .all(|s| !s.special_forces.contains(&unit)));
        assert_eq!(member_location_kind(&world, member), Some(true));
        assert_eq!(member_location(&world, member), Some(from));
        // The fleet moves, and the unit with it.
        let elsewhere = system_at(&mut world, 300);
        world.fleets[fleet].location = elsewhere;
        assert_eq!(member_location(&world, member), Some(elsewhere));

        // A later move takes it off the fleet.
        move_member(&mut world, member, Some(to));
        assert!(world.fleets[fleet].special_forces.is_empty());
        assert_eq!(world.systems[to].special_forces, vec![unit]);
        assert_eq!(member_location_kind(&world, member), Some(false));
    }

    #[test]
    fn a_destroyed_fleets_special_forces_stand_at_its_system() {
        // FUN_00556710 -> slot +0xb4 FUN_00503e20(0): a special force whose
        // container is destroyed is set Autorouting, not destroyed. port: it
        // stands at the fleet's system.
        let (mut world, _, _, fleet, to) = move_to_fleet();
        let unit = world.special_forces.insert(crate::world::SpecialForceUnit {
            class_dat_id: crate::ids::DatId::new(0x3c00_0004),
            is_alliance: true,
            skills: [0; 8],
            on_mission: false,
        });
        world.fleets[fleet].special_forces.push(unit);

        cast_adrift(&mut world, fleet);

        assert!(world.fleets[fleet].special_forces.is_empty());
        assert_eq!(world.systems[to].special_forces, vec![unit]);
        assert!(world.special_forces.contains_key(unit));
    }

    #[test]
    fn a_destroyed_special_force_leaves_the_fleet_it_was_aboard() {
        let (mut world, _, _, fleet, _) = move_to_fleet();
        let unit = world.special_forces.insert(crate::world::SpecialForceUnit {
            class_dat_id: crate::ids::DatId::new(0x3c00_0004),
            is_alliance: true,
            skills: [0; 8],
            on_mission: false,
        });
        world.fleets[fleet].special_forces.push(unit);

        destroy_special_force(&mut world, unit);

        assert!(world.fleets[fleet].special_forces.is_empty());
        assert!(!world.special_forces.contains_key(unit));
    }

    #[test]
    fn a_character_boarding_a_fleet_leaves_the_fleet_it_was_on() {
        // FUN_004f7640 puts the member in its new container; it leaves the
        // old one's roster.
        let (mut world, _, traveller, fleet, to) = move_to_fleet();
        let old = world.fleets.insert(crate::world::Fleet {
            location: to,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![traveller],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.characters[traveller].current_fleet = Some(old);

        board_fleet(&mut world, MissionMember::Character(traveller), fleet);

        assert!(world.fleets[old].characters.is_empty());
        assert_eq!(world.fleets[fleet].characters, vec![traveller]);
        assert_eq!(
            (
                world.characters[traveller].current_fleet,
                world.characters[traveller].current_system
            ),
            (Some(fleet), Some(to))
        );
    }

    #[test]
    fn a_traveller_follows_its_fleet_to_its_new_system_and_boards_there() {
        // FUN_004fbc20 (GameObjParentReroutedNotif): a traveller whose
        // target container moves is rerouted. port: on arrival, the member
        // sets out again for the fleet's system, 200 units on: 40 days.
        let (mut world, mut state, traveller, fleet, to) = move_to_fleet();
        let member = MissionMember::Character(traveller);
        let advance = step(&mut state, &world, 1, &[]);
        apply_advance_effects(&mut world, &advance.effects);
        let elsewhere = system_at(&mut world, 300);
        world.fleets[fleet].location = elsewhere;

        let advance = step(&mut state, &world, 20, &[0.5]);

        assert!(advance.effects.is_empty(), "{:?}", advance.effects);
        assert_eq!(state.en_route()[0].to, elsewhere);
        assert_eq!(state.en_route()[0].arrival, 60);
        assert_eq!(state.missions()[0].phase, PHASE_TRANSIT);
        assert_eq!(state.missions()[0].target_system, elsewhere);
        assert_ne!(to, elsewhere);

        let advance = step(&mut state, &world, 60, &[0.5]);
        assert_eq!(
            advance.effects,
            vec![
                MissionEffect::MemberMoved {
                    member,
                    to: Some(elsewhere)
                },
                MissionEffect::MemberBoarded { member, fleet },
            ]
        );
    }

    #[test]
    fn a_move_may_target_its_own_sides_fleet_only() {
        // FUN_004f4b60: 1/0x28 for a target whose side is not the order's.
        let (world, _, _, fleet, to) = move_to_fleet();
        assert!(move_place_allowed(&world, MissionFaction::Alliance, to, Some(fleet)));
        assert!(!move_place_allowed(&world, MissionFaction::Empire, to, Some(fleet)));
        assert!(!move_place_allowed(&world, MissionFaction::Alliance, to, None));
    }

    #[test]
    fn adrift_characters_settle_at_their_sides_system_and_leave_others_together() {
        // FUN_0054c7d0: an adrift character that stays (its side's
        // standing system) has Adrift cleared; the others of one side at
        // one place leave together for FUN_00543040's destination on an
        // Adrift order 0x44000004 (FUN_00549230 -> FUN_00547880), hidden as
        // its record says. A prisoner or a member of a mission is left out.
        use crate::world::ControlKind;
        let mut world = minimal_world();
        world.mission_records = vec![crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x4400_0004),
            ..move_record()
        }];
        let home = system_at(&mut world, 0);
        let enemy = system_at(&mut world, 100);
        world.systems[home].control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        world.systems[enemy].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        let adrift = |world: &mut GameWorld, at| {
            let key = agent_at(world, at, true);
            world.characters[key].adrift = true;
            key
        };
        let settled = adrift(&mut world, home);
        let first = adrift(&mut world, enemy);
        let second = adrift(&mut world, enemy);
        let held = adrift(&mut world, enemy);
        world.characters[held].is_captive = true;
        let busy = adrift(&mut world, enemy);
        world.characters[busy].on_mission = true;
        let mut state = MissionState::new();

        let advance = step(&mut state, &world, 1, &[]);

        assert!(advance.effects.contains(&MissionEffect::AdriftSettled { character: settled }));
        let [first, second] = [first, second].map(MissionMember::Character);
        for member in [first, second] {
            assert!(advance
                .effects
                .contains(&MissionEffect::MemberOnMission { member, hidden: true }));
        }
        let orders: Vec<_> = state
            .missions()
            .iter()
            .map(|m| (m.kind, m.team.clone(), m.target_system, m.phase))
            .collect();
        assert_eq!(
            orders,
            [(MissionKind::Adrift, vec![first, second], home, PHASE_TRANSIT)]
        );
        apply_advance_effects(&mut world, &advance.effects);
        assert!(!world.characters[settled].adrift);
        assert!(world.characters[held].adrift && world.characters[busy].adrift);
        // The order took an id of its own.
        let order_id = state.missions()[0].id;
        let next = state.dispatch(MissionRequest::single(
            MissionKind::Move,
            MissionFaction::Alliance,
            settled,
            home,
            None,
            2,
        ));
        assert_ne!(next, order_id);
    }

    #[test]
    fn a_move_to_a_system_of_another_side_ends_with_code_eight() {
        // FUN_00523450 (Move's slot +0x1bc, FUN_005215e0): a target not on
        // the mission's side fails record +0x7c, end 8.
        let mut world = minimal_world();
        world.mission_records = vec![move_record()];
        let here = system_at(&mut world, 0);
        let enemy = system_at(&mut world, 0);
        world.systems[enemy].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        let mission = ActiveMission::new(
            0,
            MissionKind::Move,
            MissionFaction::Alliance,
            vec![MissionMember::Character(agent_at(&mut world, here, true))],
            enemy,
            0,
        );

        let code = running_end_code(
            &mission,
            &world,
            &crate::uprising::UprisingState::default(),
            &ResearchState::new(),
            &[],
        );

        assert_eq!(code, 8);
    }

    #[test]
    fn members_at_the_target_land_at_once_and_the_timer_starts_on_the_order_day() {
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let (mut state, envoy) = diplomacy_from(&mut world, here, here);

        let advance = step(&mut state, &world, 1, &[0.0]);

        assert_eq!(
            advance.effects,
            vec![MissionEffect::MemberMoved {
                member: MissionMember::Character(envoy),
                to: Some(here)
            }]
        );
        assert!(state.en_route().is_empty());
        assert_eq!(state.missions()[0].timer_due, Some(5));
    }

    #[test]
    fn a_repeating_mission_returns_to_the_timer_after_each_outcome() {
        // FUN_005227d0: phase 10 steps to 8 when record +0x58 is set.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let (mut state, _) = diplomacy_from(&mut world, here, here);
        step(&mut state, &world, 1, &[0.0]);

        // Outcome 0.99, then the new timer draw 1.0: 5 + 10 days.
        let advance = step(&mut state, &world, 5, &[0.99, 1.0]);

        assert_eq!(advance.results.len(), 1);
        assert!(advance.ended.is_empty());
        let mission = &state.missions()[0];
        assert_eq!(mission.phase, PHASE_TIMER);
        assert_eq!(mission.timer_due, Some(20));
    }

    #[test]
    fn a_mission_that_runs_its_course_ends_with_code_one_and_frees_its_members() {
        // FUN_00522480 in phase 0xb with no code sets end code 1; the
        // Rescue record does not repeat.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        world.mission_records = vec![rescue_record()];
        let hero = agent_at(&mut world, here, true);
        let prisoner = agent_at(&mut world, here, true);
        world.characters[prisoner].is_captive = true;
        world.characters[prisoner].captured_by = Some(crate::dat::Faction::Empire);
        let mut state = MissionState::new();
        state
            .dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Rescue,
                    MissionFaction::Alliance,
                    hero,
                    here,
                    Some(prisoner),
                    0,
                ),
                &mut world,
            )
            .expect("a free hero");
        // 1 + floor(1.0 * 7).min(6) = 7 days.
        step(&mut state, &world, 1, &[1.0]);
        assert_eq!(state.missions()[0].timer_due, Some(7));

        let advance = step(&mut state, &world, 7, &[0.99]);

        assert_eq!(advance.results.len(), 1);
        assert_eq!(advance.ended.len(), 1);
        assert_eq!(advance.ended[0].end_code, 1);
        assert!(advance.effects.contains(&MissionEffect::MemberAvailable {
            member: MissionMember::Character(hero)
        }));
        assert!(state.is_empty());
    }

    /// A Rescue of an Alliance prisoner held by the Empire, past its
    /// creation steps and waiting for its timer on day 7.
    fn waiting_rescue(world: &mut GameWorld) -> (MissionState, CharacterKey) {
        let here = system_at(world, 0);
        world.mission_records = vec![rescue_record()];
        let hero = agent_at(world, here, true);
        let prisoner = agent_at(world, here, true);
        world.characters[prisoner].is_captive = true;
        world.characters[prisoner].captured_by = Some(crate::dat::Faction::Empire);
        let mut state = MissionState::new();
        state
            .dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Rescue,
                    MissionFaction::Alliance,
                    hero,
                    here,
                    Some(prisoner),
                    0,
                ),
                world,
            )
            .expect("a free hero");
        step(&mut state, world, 1, &[1.0]);
        (state, prisoner)
    }

    #[test]
    fn a_rescue_ends_with_code_eight_once_its_target_is_back_on_its_side() {
        // FUN_00592600 rule 3: a freed prisoner reads as the mission's own
        // side (inference), and Rescue's column 15 is 0.
        let mut world = minimal_world();
        let (mut state, prisoner) = waiting_rescue(&mut world);
        world.characters[prisoner].is_captive = false;
        world.characters[prisoner].captured_by = None;

        // A live target changing sides is seen only on a phase change.
        assert!(step(&mut state, &world, 2, &[]).ended.is_empty());
        let advance = step(&mut state, &world, 7, &[0.0]);

        assert!(advance.results.is_empty());
        assert_eq!(advance.ended[0].end_code, 8);
    }

    #[test]
    fn a_rescue_ends_with_code_six_when_its_opposing_target_is_not_a_prisoner() {
        // FUN_00586e20: a free target needs column 21, which Rescue lacks.
        let mut world = minimal_world();
        let (mut state, target) = waiting_rescue(&mut world);
        let officer = &mut world.characters[target];
        officer.is_captive = false;
        officer.captured_by = None;
        officer.is_alliance = false;
        officer.is_empire = true;

        let advance = step(&mut state, &world, 7, &[0.0]);

        assert!(advance.results.is_empty());
        assert_eq!(advance.ended[0].end_code, 6);
    }

    #[test]
    fn diplomacy_ends_with_code_0xf_once_its_side_holds_full_support() {
        // FUN_00573ee0: FUN_00507270 == 100 ends the mission.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let (mut state, _) = diplomacy_from(&mut world, here, here);
        step(&mut state, &world, 1, &[0.0]);
        world.systems[here].popularity_alliance = 1.0;

        let advance = step(&mut state, &world, 5, &[0.0]);

        assert!(advance.results.is_empty());
        assert_eq!(advance.ended[0].end_code, 0xf);
    }

    /// MISSNSD Recruitment `0x55000016`, columns 11..21 = 1 1 1 1 0 0 1 1
    /// 0 0 (`ghidra/notes/mission-lifecycle.md`). port: the timer is
    /// Diplomacy's, which these tests do not read.
    fn recruitment_record() -> crate::world::MissionRecord {
        crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x5500_0016),
            rules: crate::world::MissionTargetRules {
                other_side_target: false,
                revolting_target: true,
                ..diplomacy_record().rules
            },
            ..diplomacy_record()
        }
    }

    /// An Alliance character outside the side's roster (`+0x50` bit 1 clear).
    fn pool_character(world: &mut GameWorld, dat_id: u32) -> CharacterKey {
        world.characters.insert(Character {
            dat_id: crate::ids::DatId::new(dat_id),
            is_alliance: true,
            ..Default::default()
        })
    }

    fn recruits(result: &MissionResult) -> Vec<(CharacterKey, bool)> {
        result
            .effects
            .iter()
            .filter_map(|effect| match effect {
                MissionEffect::CharacterRecruited {
                    character,
                    pool_emptied,
                    ..
                } => Some((*character, *pool_emptied)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_recruiter_signs_one_of_its_side_s_unrecruited_characters_and_gains_leadership() {
        // FUN_0056b9a0 -> FUN_0055fc80: a random minor character (families
        // 0x38..0x3c) of the side with +0x50 bit 1 clear (FUN_0055ef30); a
        // killed, major, or enemy one is not in the pool. The raise is
        // GNPRTB 6159.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Recruitment, 100);
        let recruiter = agent_at(&mut world, here, true);
        let first = pool_character(&mut world, 2);
        let second = pool_character(&mut world, 3);
        let dead = pool_character(&mut world, 4);
        world.characters[dead].is_killed = true;
        let enemy = pool_character(&mut world, 5);
        world.characters[enemy].is_alliance = false;
        world.characters[enemy].is_empire = true;
        let major = pool_character(&mut world, 6);
        world.characters[major].is_major = true;

        let result = phase_ten(
            &world,
            MissionKind::Recruitment,
            vec![MissionMember::Character(recruiter)],
            here,
            None,
        );

        let signed = recruits(&result);
        assert_eq!(signed.len(), 1);
        assert!([first, second].contains(&signed[0].0));
        assert!(!signed[0].1, "one pool character is left");
        assert_eq!(result.outcome, MissionOutcome::Success);
        assert_eq!(raised(&result), vec![(recruiter, Skill::Leadership, 1)]);
    }

    #[test]
    fn each_successful_recruiter_signs_a_different_character_and_the_last_empties_the_pool() {
        // FUN_0055fc80 sets side +0xb8 (FUN_0052f590) when it takes the
        // last one; a recruiter that finds the pool empty signs no one.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Recruitment, 100);
        let team = (0..3)
            .map(|_| MissionMember::Character(agent_at(&mut world, here, true)))
            .collect();
        let first = pool_character(&mut world, 2);
        let second = pool_character(&mut world, 3);

        let result = phase_ten(&world, MissionKind::Recruitment, team, here, None);

        let signed = recruits(&result);
        assert_eq!(signed.len(), 2);
        assert!(signed.iter().any(|&(c, _)| c == first));
        assert!(signed.iter().any(|&(c, _)| c == second));
        assert_eq!(signed.iter().filter(|&&(_, emptied)| emptied).count(), 1);
        assert!(signed[1].1, "the second pick takes the last one");
        assert_eq!(raised(&result).len(), 2);
    }

    #[test]
    fn a_killed_major_or_enemy_character_is_never_recruited() {
        // FUN_0055ef30 counts only the side's minor characters (families
        // 0x38..0x3c through FUN_0056f450) with +0x50 bit 1 clear.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Recruitment, 100);
        let recruiter = agent_at(&mut world, here, true);
        let dead = pool_character(&mut world, 2);
        world.characters[dead].is_killed = true;
        let major = pool_character(&mut world, 3);
        world.characters[major].is_major = true;
        let enemy = pool_character(&mut world, 4);
        world.characters[enemy].is_alliance = false;
        world.characters[enemy].is_empire = true;
        let signed = pool_character(&mut world, 5);
        world.characters[signed].recruited = true;

        let result = phase_ten(
            &world,
            MissionKind::Recruitment,
            vec![MissionMember::Character(recruiter)],
            here,
            None,
        );

        assert!(recruits(&result).is_empty());
    }

    #[test]
    fn a_recruiter_with_an_empty_pool_fails_without_a_raise() {
        // FUN_0056b9a0 sets 3 only after a recruit; slot +0x280 turns the
        // unset result into 2.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Recruitment, 100);
        let recruiter = agent_at(&mut world, here, true);

        let result = phase_ten(
            &world,
            MissionKind::Recruitment,
            vec![MissionMember::Character(recruiter)],
            here,
            None,
        );

        assert!(recruits(&result).is_empty());
        assert!(raised(&result).is_empty());
        assert_eq!(result.outcome, MissionOutcome::Failure);
    }

    #[test]
    fn two_recruitments_in_one_step_cannot_sign_the_same_character() {
        // port: the world applies recruits after the step, so a later
        // mission skips the pick of an earlier one, as FUN_0055fc80 would
        // skip its +0x50 bit 1.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Recruitment, 100);
        let only = pool_character(&mut world, 2);
        let mut state = MissionState::new();
        for id in 0..2 {
            let recruiter = agent_at(&mut world, here, true);
            state.push_timed(ActiveMission::timed(
                id,
                MissionKind::Recruitment,
                MissionFaction::Alliance,
                vec![MissionMember::Character(recruiter)],
                here,
                1,
            ));
        }

        let advance = step(&mut state, &world, 1, &[]);

        let signed: Vec<_> = advance.results.iter().flat_map(recruits).collect();
        assert_eq!(signed, vec![(only, true)]);
    }

    #[test]
    fn a_recruit_joins_at_the_target_and_an_emptied_pool_marks_its_side() {
        // FUN_0055fe70: slot +0xa8 places it, FUN_004f7480 sets +0x50 bit
        // 1; FUN_0052f590 sets the side's +0xb8.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let recruit = pool_character(&mut world, 2);
        let other = pool_character(&mut world, 3);

        recruit_character(&mut world, recruit, here, MissionFaction::Alliance, false);
        assert!(!world.recruit_pool_empty(crate::dat::Faction::Alliance));
        recruit_character(&mut world, other, here, MissionFaction::Alliance, true);

        let c = &world.characters[recruit];
        assert!(c.recruited);
        assert_eq!(c.current_system, Some(here));
        assert!(world.recruit_pool_empty(crate::dat::Faction::Alliance));
        assert!(!world.recruit_pool_empty(crate::dat::Faction::Empire));
    }

    #[test]
    fn recruitment_ends_with_code_0x10_once_its_side_recruited_the_last_pool_character() {
        // FUN_0056b370: after the system rules, side +0xb8 ends it with 0x10.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        held_by(&mut world, here, crate::dat::Faction::Alliance);
        world.mission_records = vec![recruitment_record()];
        let recruiter = agent_at(&mut world, here, true);
        let mut state = MissionState::new();
        state
            .dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Recruitment,
                    MissionFaction::Alliance,
                    recruiter,
                    here,
                    None,
                    0,
                ),
                &mut world,
            )
            .expect("a free recruiter");
        step(&mut state, &world, 1, &[0.0]);
        assert!(step(&mut state, &world, 2, &[0.0]).ended.is_empty());
        world.set_recruit_pool_empty(crate::dat::Faction::Alliance);

        let advance = step(&mut state, &world, 5, &[0.0]);

        assert!(advance.results.is_empty());
        assert_eq!(advance.ended[0].end_code, 0x10);
    }

    /// MISSNSD Sabotage `0x69000012`, columns 11..21 = 0 0 1 0 1 1 0 0 0 0
    /// (`ghidra/notes/mission-lifecycle.md`). port: the timer is
    /// Diplomacy's, which these tests do not read.
    fn sabotage_record() -> crate::world::MissionRecord {
        crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x6900_0012),
            rules: crate::world::MissionTargetRules {
                container_loss_ends: false,
                needs_populated_container: false,
                target_loss_ends: true,
                own_side_target: false,
                other_side_target: true,
                opponent_target: true,
                revolting_target: false,
                calm_target: false,
                prisoner_target: false,
                free_target: false,
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0x402,
                character_mask: 0x1_0000,
            },
            ..diplomacy_record()
        }
    }

    /// An Empire shipyard at `system`.
    fn enemy_yard(
        world: &mut GameWorld,
        system: SystemKey,
    ) -> crate::ids::ManufacturingFacilityKey {
        let key =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: crate::ids::DatId::new(0x2800_0001),
                    side: crate::dat::Faction::Empire,
                    is_shipyard: true,
                });
        world.systems[system].manufacturing_facilities.push(key);
        key
    }

    /// An Empire fleet at `system` with a Star Destroyer and the Death Star.
    fn death_star_fleet(world: &mut GameWorld, system: SystemKey) -> crate::ids::FleetKey {
        let destroyer = world
            .capital_ship_classes
            .insert(crate::world::CapitalShipClass::default());
        let death_star = world
            .capital_ship_classes
            .insert(crate::world::CapitalShipClass {
                dat_id: crate::ids::DatId::new(crate::world::DEATH_STAR_CLASS_ID),
                ..Default::default()
            });
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: system,
            capital_ships: vec![
                crate::world::ShipInstance::new(destroyer, 100, false),
                crate::world::ShipInstance::new(death_star, 100, false),
            ],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: true,
            special_forces: Vec::new(),
        });
        world.systems[system].fleets.push(fleet);
        fleet
    }

    fn sabotage_request(
        world: &mut GameWorld,
        kind: MissionKind,
        at: SystemKey,
        target: Option<MissionTarget>,
    ) -> MissionRequest {
        let agent = agent_at(world, at, true);
        MissionRequest {
            target_object: target,
            ..MissionRequest::single(kind, MissionFaction::Alliance, agent, at, None, 0)
        }
    }

    #[test]
    fn sabotage_needs_an_object_other_than_the_death_star_and_ds_sabotage_needs_the_death_star() {
        // FUN_0056a110 refuses a family 0x18..0x1c target (0x40/0x28);
        // FUN_005744c0 refuses any other (0x40/0x29). port: every order
        // names an object, so none is refused too.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let yard = MissionTarget::ManufacturingFacility(enemy_yard(&mut world, here));
        let death_star = MissionTarget::DeathStar(death_star_fleet(&mut world, here));
        let mut state = MissionState::new();
        for (kind, target, allowed) in [
            (MissionKind::Sabotage, None, false),
            (MissionKind::Sabotage, Some(death_star), false),
            (MissionKind::Sabotage, Some(yard), true),
            (MissionKind::DeathStarSabotage, None, false),
            (MissionKind::DeathStarSabotage, Some(yard), false),
            (MissionKind::DeathStarSabotage, Some(death_star), true),
        ] {
            let request = sabotage_request(&mut world, kind, here, target);
            let outcome = state.dispatch_guarded(request, &mut world);
            assert_eq!(
                outcome.is_ok(),
                allowed,
                "{kind:?} naming {target:?}: {outcome:?}"
            );
            if !allowed {
                assert_eq!(outcome, Err(MissionRefusal::TargetUnavailable));
            }
        }
    }

    #[test]
    fn a_successful_sabotage_destroys_its_target_object_once() {
        // FUN_005746e0 runs once after every member rolled: on 3, slot
        // +0xac(6) destroys the order's object (FUN_00521030).
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Sabotage, 100);
        let yard = enemy_yard(&mut world, here);
        let team = (0..2)
            .map(|_| MissionMember::Character(agent_at(&mut world, here, true)))
            .collect();
        let mut mission = ActiveMission::timed(
            0,
            MissionKind::Sabotage,
            MissionFaction::Alliance,
            team,
            here,
            1,
        );
        mission.target_object = Some(MissionTarget::ManufacturingFacility(yard));
        let mut state = MissionState::new();
        state.push_timed(mission);

        let result = step(&mut state, &world, 1, &[]).results.remove(0);

        let destroyed: Vec<_> = result
            .effects
            .iter()
            .filter_map(|effect| match effect {
                MissionEffect::TargetSabotaged { target, system } => Some((*target, *system)),
                _ => None,
            })
            .collect();
        assert_eq!(
            destroyed,
            vec![(MissionTarget::ManufacturingFacility(yard), here)]
        );
        assert_eq!(result.outcome, MissionOutcome::Success);
    }

    #[test]
    fn a_failed_sabotage_destroys_nothing() {
        // FUN_005746e0 turns the unset result into 2 and destroys only on 3.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        always(&mut world, MissionKind::Sabotage, 0);
        let yard = enemy_yard(&mut world, here);
        let agent = agent_at(&mut world, here, true);
        let mut mission = ActiveMission::timed(
            0,
            MissionKind::Sabotage,
            MissionFaction::Alliance,
            vec![MissionMember::Character(agent)],
            here,
            1,
        );
        mission.target_object = Some(MissionTarget::ManufacturingFacility(yard));
        let mut state = MissionState::new();
        state.push_timed(mission);

        let result = step(&mut state, &world, 1, &[]).results.remove(0);

        assert!(!result
            .effects
            .iter()
            .any(|effect| matches!(effect, MissionEffect::TargetSabotaged { .. })));
        assert_eq!(result.outcome, MissionOutcome::Failure);
    }

    #[test]
    fn a_destroyed_object_leaves_its_system_and_the_world() {
        // Slot +0xac(6) destroys the object; each kind leaves its system's
        // list and its arena.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let yard = enemy_yard(&mut world, here);
        let turret = world
            .defense_facilities
            .insert(crate::world::DefenseFacilityInstance {
                class_dat_id: crate::ids::DatId::new(0x2200_0001),
                side: crate::dat::Faction::Empire,
            });
        world.systems[here].defense_facilities.push(turret);
        let mine = world
            .production_facilities
            .insert(crate::world::ProductionFacilityInstance {
                class_dat_id: crate::ids::DatId::new(0x2c00_0001),
                side: crate::dat::Faction::Empire,
                is_mine: true,
            });
        world.systems[here].production_facilities.push(mine);
        let regiment = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: crate::ids::DatId::new(0x1000_0001),
            is_alliance: false,
            regiment_strength: 10,
        });
        world.systems[here].ground_units.push(regiment);
        let unit = world.special_forces.insert(crate::world::SpecialForceUnit {
            class_dat_id: crate::ids::DatId::new(0x3c00_0001),
            is_alliance: false,
            skills: [0; 8],
            on_mission: false,
        });
        world.systems[here].special_forces.push(unit);

        for target in [
            MissionTarget::ManufacturingFacility(yard),
            MissionTarget::DefenseFacility(turret),
            MissionTarget::ProductionFacility(mine),
            MissionTarget::Troop(regiment),
            MissionTarget::SpecialForce(unit),
        ] {
            assert!(target.state(&world).is_some(), "{target:?} stands");
            destroy_target(&mut world, target);
            assert!(target.state(&world).is_none(), "{target:?} is gone");
        }
        let sys = &world.systems[here];
        assert!(sys.manufacturing_facilities.is_empty());
        assert!(sys.defense_facilities.is_empty());
        assert!(sys.production_facilities.is_empty());
        assert!(sys.ground_units.is_empty());
        assert!(sys.special_forces.is_empty());
        assert!(world.manufacturing_facilities.is_empty());
        assert!(world.troops.is_empty());
    }

    #[test]
    fn ds_sabotage_destroys_only_the_fleet_s_death_star_hull() {
        // FUN_005746e0 destroys the order's object, the Death Star (class
        // 0x88, family 0x18); the fleet's other hulls stay.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let fleet = death_star_fleet(&mut world, here);
        let target = MissionTarget::DeathStar(fleet);

        destroy_target(&mut world, target);

        let fleet = &world.fleets[fleet];
        assert!(!fleet.has_death_star);
        assert_eq!(fleet.capital_ships.len(), 1);
        assert!(!world.capital_ship_classes[fleet.capital_ships[0].class].is_death_star());
        assert!(target.state(&world).is_none());
    }

    #[test]
    fn a_sabotaged_last_ship_disbands_its_fleet() {
        // FUN_005746e0 destroys the object; the fleet's ship change
        // (FUN_004fe870 -> FUN_004fe630) disbands a fleet left without a
        // capital ship (slot +0xac(0x14)).
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let lone = death_star_fleet(&mut world, here);
        world.fleets[lone].capital_ships.remove(0);
        let escorted = death_star_fleet(&mut world, here);

        destroy_target(&mut world, MissionTarget::DeathStar(lone));
        destroy_target(&mut world, MissionTarget::DeathStar(escorted));

        assert!(!world.fleets.contains_key(lone));
        assert!(!world.systems[here].fleets.contains(&lone));
        assert!(world.systems[here].fleets.contains(&escorted));
        assert!(!world.fleets[escorted].has_death_star);

        // A capital ship's sabotage follows the same rule.
        let destroyer = world.fleets[escorted].capital_ships[0].class;
        destroy_target(
            &mut world,
            MissionTarget::CapitalShip {
                fleet: escorted,
                class: destroyer,
            },
        );
        assert!(!world.fleets.contains_key(escorted));
        assert!(world.systems[here].fleets.is_empty());
    }

    #[test]
    fn a_fleet_keeps_its_death_star_flag_while_another_death_star_remains() {
        // has_death_star follows the fleet's Death Star hulls.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let fleet = death_star_fleet(&mut world, here);
        let second = world.fleets[fleet].capital_ships[1].clone();
        world.fleets[fleet].capital_ships.push(second);

        destroy_target(&mut world, MissionTarget::DeathStar(fleet));

        assert!(world.fleets[fleet].has_death_star);
        assert_eq!(world.fleets[fleet].capital_ships.len(), 2);
    }

    /// A Sabotage of `target` at `here`, waiting in its timer phase.
    fn waiting_sabotage(
        world: &mut GameWorld,
        here: SystemKey,
        target: MissionTarget,
    ) -> MissionState {
        world.mission_records = vec![sabotage_record()];
        let request = sabotage_request(world, MissionKind::Sabotage, here, Some(target));
        let mut state = MissionState::new();
        state
            .dispatch_guarded(request, world)
            .expect("a free saboteur");
        step(&mut state, world, 1, &[0.0]);
        assert!(step(&mut state, world, 2, &[0.0]).ended.is_empty());
        state
    }

    #[test]
    fn a_sabotage_ends_with_code_six_once_its_object_is_destroyed() {
        // FUN_00593500 base rule 4 (col 14): a destroyed target ends 6.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let yard = enemy_yard(&mut world, here);
        let mut state =
            waiting_sabotage(&mut world, here, MissionTarget::ManufacturingFacility(yard));
        destroy_target(&mut world, MissionTarget::ManufacturingFacility(yard));

        let advance = step(&mut state, &world, 5, &[0.0]);

        assert!(advance.results.is_empty());
        assert_eq!(advance.ended[0].end_code, 6);
    }

    #[test]
    fn a_sabotage_ends_with_code_six_once_its_object_leaves_the_target_system() {
        // Base rules 5 and 6: the target has left its container.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let there = system_at(&mut world, 50);
        let regiment = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: crate::ids::DatId::new(0x1000_0001),
            is_alliance: false,
            regiment_strength: 10,
        });
        world.systems[here].ground_units.push(regiment);
        let mut state = waiting_sabotage(&mut world, here, MissionTarget::Troop(regiment));
        world.systems[here].ground_units.clear();
        world.systems[there].ground_units.push(regiment);

        let advance = step(&mut state, &world, 5, &[0.0]);

        assert_eq!(advance.ended[0].end_code, 6);
    }

    #[test]
    fn a_sabotage_ends_with_code_eight_once_its_object_belongs_to_its_own_side() {
        // Base rule 3 (col 15): Sabotage may not target its own side.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let yard = enemy_yard(&mut world, here);
        let mut state =
            waiting_sabotage(&mut world, here, MissionTarget::ManufacturingFacility(yard));
        world.manufacturing_facilities[yard].side = crate::dat::Faction::Alliance;

        let advance = step(&mut state, &world, 5, &[0.0]);

        assert_eq!(advance.ended[0].end_code, 8);
    }

    #[test]
    fn diplomacy_ends_with_code_eight_at_a_system_the_opponent_holds() {
        // FUN_00592600 rule 3: an opponent target reads column 17, 0 for
        // Diplomacy.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let (mut state, _) = diplomacy_from(&mut world, here, here);
        step(&mut state, &world, 1, &[0.0]);
        world.systems[here].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);

        // The validator runs only on a phase change: the mission waits on.
        assert!(step(&mut state, &world, 2, &[]).ended.is_empty());
        let advance = step(&mut state, &world, 5, &[0.0]);

        assert!(advance.results.is_empty());
        assert_eq!(advance.ended[0].end_code, 8);
    }

    #[test]
    fn a_destroyed_target_system_ends_a_waiting_mission_at_once() {
        // FUN_00545240 runs the validator when the container is destroyed;
        // Diplomacy's column 11 ends it with code 7 before the timer fires.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let (mut state, _) = diplomacy_from(&mut world, here, here);
        step(&mut state, &world, 1, &[0.0]);
        world.systems[here].is_destroyed = true;

        let advance = step(&mut state, &world, 2, &[0.0]);

        assert!(advance.results.is_empty());
        assert_eq!(advance.ended[0].end_code, 7);
    }

    #[test]
    fn a_member_still_travelling_when_its_mission_ends_keeps_travelling() {
        // FUN_00592c80 never moves the members; each transit runs on.
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let (mut state, envoy) = diplomacy_from(&mut world, from, to);
        step(&mut state, &world, 1, &[]);
        world.systems[to].is_destroyed = true;

        let advance = step(&mut state, &world, 2, &[]);

        assert_eq!(advance.ended[0].end_code, 7);
        assert!(state.is_en_route(MissionMember::Character(envoy)));
        let arrival = step(&mut state, &world, 20, &[]);
        assert_eq!(
            arrival.effects,
            vec![MissionEffect::MemberMoved {
                member: MissionMember::Character(envoy),
                to: Some(to)
            }]
        );
    }

    #[test]
    fn han_solo_on_the_team_halves_his_mission_s_travel_time() {
        // FUN_00548370: GNPRTB 3083 (50) when Han Solo is a free member
        // and no special force is; 20 days become 10.
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let (mut state, envoy) = diplomacy_from(&mut world, from, to);
        world.characters[envoy].dat_id = crate::ids::DatId::new(0x3300_0243);

        step(&mut state, &world, 1, &[]);

        assert_eq!(state.missions()[0].speed, 50);
        assert_eq!(state.en_route()[0].arrival, 10);
    }

    #[test]
    fn a_special_force_on_han_solo_s_mission_keeps_the_default_speed() {
        // FUN_00525dc0 / FUN_00525a00: any special force member cancels
        // the Han Solo speed.
        let mut world = minimal_world();
        world.mission_records = vec![diplomacy_record()];
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let han = agent_at(&mut world, from, true);
        world.characters[han].dat_id = crate::ids::DatId::new(0x3300_0243);
        let unit = world.special_forces.insert(crate::world::SpecialForceUnit {
            class_dat_id: crate::ids::DatId::new(0x3c00_0003),
            is_alliance: true,
            skills: [0; 8],
            on_mission: false,
        });
        world.systems[from].special_forces.push(unit);
        let mut state = MissionState::new();
        state
            .dispatch_guarded(
                MissionRequest {
                    kind: MissionKind::Diplomacy,
                    faction: MissionFaction::Alliance,
                    team: vec![MissionMember::Character(han)],
                    decoys: vec![MissionMember::SpecialForce(unit)],
                    target_system: to,
                    target_character: None,
                    target_object: None,
                    tick: 0,
                },
                &mut world,
            )
            .expect("both members share a system");

        step(&mut state, &world, 1, &[]);

        assert_eq!(state.missions()[0].speed, 100);
        assert!(state.en_route().iter().all(|transit| transit.arrival == 20));
    }

    #[test]
    fn each_mission_draws_from_its_own_window_of_rolls() {
        // port: ROLLS_PER_MISSION per mission per tick. Each repeating
        // Diplomacy re-arms timer 0x38b (MISSNSD 5 + rand(0..=10)) from the
        // first draw of its own window.
        let (system, _) = mock_keys();
        let mut world = minimal_world();
        world.mission_records = vec![diplomacy_record()];
        let character = agent_at(&mut world, system, true);
        let mut state = MissionState::new();
        for id in 0..3 {
            state.push_timed(ActiveMission::timed(
                id,
                MissionKind::Diplomacy,
                MissionFaction::Alliance,
                vec![MissionMember::Character(character)],
                system,
                1,
            ));
        }
        let mut rolls = vec![0.99; 3 * ROLLS_PER_MISSION];
        rolls[2 * ROLLS_PER_MISSION] = 0.0;

        step(&mut state, &world, 1, &rolls);

        let due: Vec<_> = state.missions().iter().map(|m| m.timer_due).collect();
        assert_eq!(due, vec![Some(16), Some(16), Some(6)]);
    }

    #[test]
    fn a_moved_member_leaves_its_fleet_or_system_and_lands_at_the_target() {
        // FUN_00556390: slot +0xa8 moves the member into the container.
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let pilot = agent_at(&mut world, from, true);
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: from,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![pilot],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.characters[pilot].current_fleet = Some(fleet);
        world.characters[pilot].command = crate::character_command::CommandKind::Admiral;
        let unit = world.special_forces.insert(crate::world::SpecialForceUnit {
            class_dat_id: crate::ids::DatId::new(0x3c00_0003),
            is_alliance: true,
            skills: [0; 8],
            on_mission: true,
        });
        world.systems[from].special_forces.push(unit);

        move_member(&mut world, MissionMember::Character(pilot), None);
        move_member(&mut world, MissionMember::SpecialForce(unit), None);
        assert!(world.fleets[fleet].characters.is_empty());
        assert_eq!(world.characters[pilot].current_fleet, None);
        assert_eq!(world.characters[pilot].current_system, None);
        // FUN_00560d50: the move takes the admiral's command.
        assert_eq!(
            world.characters[pilot].command,
            crate::character_command::CommandKind::None
        );
        assert!(world.systems[from].special_forces.is_empty());

        move_member(&mut world, MissionMember::Character(pilot), Some(to));
        move_member(&mut world, MissionMember::SpecialForce(unit), Some(to));
        assert_eq!(world.characters[pilot].current_system, Some(to));
        assert_eq!(world.systems[to].special_forces, vec![unit]);
    }

    /// An Assassination of an Empire officer, with the shipped Assassination
    /// columns but a 10-day timer, waiting from day 1.
    fn waiting_assassination(
        world: &mut GameWorld,
        with_record: bool,
    ) -> (MissionState, CharacterKey) {
        let here = system_at(world, 0);
        if with_record {
            world.mission_records = vec![crate::world::MissionRecord {
                dat_id: crate::ids::DatId::new(0x6300_0081),
                timer_min_days: 10,
                timer_spread_days: 0,
                repeats: false,
                hidden: false,
                detection_phases: true,
                can_resign: true,
                leave_rule: 0,
                rules: crate::world::MissionTargetRules {
                    target_loss_ends: true,
                    opponent_target: true,
                    free_target: true,
                    ..Default::default()
                },
                // port: the shipped record admits only Empire members; this
                // fixture's assassin is Alliance, so both sides are allowed.
                members: crate::world::MissionMemberRules {
                    alliance: true,
                    empire: true,
                    special_force_mask: 0x800,
                    character_mask: 0x1_0000,
                },
            }];
        }
        let assassin = agent_at(world, here, true);
        let officer = agent_at(world, here, false);
        let mut state = MissionState::new();
        state
            .dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Assassination,
                    MissionFaction::Alliance,
                    assassin,
                    here,
                    Some(officer),
                    0,
                ),
                world,
            )
            .expect("a free assassin");
        step(&mut state, world, 1, &[0.0]);
        (state, officer)
    }

    #[test]
    fn a_killed_target_ends_a_waiting_mission_on_the_day_it_dies() {
        // FUN_00545240 runs the validator when the target (+0x70,
        // FUN_00520cb0) is destroyed; column 14 ends it with code 6.
        let mut world = minimal_world();
        let (mut state, officer) = waiting_assassination(&mut world, true);
        world.characters[officer].mark_killed();

        let advance = step(&mut state, &world, 3, &[]);

        assert!(advance.results.is_empty());
        assert_eq!(
            advance
                .ended
                .iter()
                .map(|e| (e.end_code, e.tick))
                .collect::<Vec<_>>(),
            vec![(6, 3)]
        );
    }

    #[test]
    fn only_a_destroyed_container_ends_a_mission_with_code_seven() {
        // FUN_00545240.c:85-111 sets code 7 for a destroyed container; a
        // killed target that the validator lets pass leaves no code.
        // port: without a record the validator checks nothing.
        let mut world = minimal_world();
        let (mut state, officer) = waiting_assassination(&mut world, false);
        world.characters[officer].mark_killed();

        let advance = step(&mut state, &world, 3, &[]);

        assert!(advance.ended.is_empty());
        assert_eq!(state.missions()[0].end_code, 0);
    }

    #[test]
    fn a_mission_that_lands_at_a_changed_target_ends_before_arming_its_timer() {
        // FUN_00524b70 phases 5 and 6 run the validator when the target
        // differs from the origin (FUN_00520bb0): an envoy landing at a
        // system the opponent took on the way ends with code 8.
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let (mut state, _) = diplomacy_from(&mut world, from, to);
        step(&mut state, &world, 1, &[]);
        world.systems[to].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);

        let advance = step(&mut state, &world, 20, &[0.0]);

        assert_eq!(
            advance
                .ended
                .iter()
                .map(|e| (e.end_code, e.tick))
                .collect::<Vec<_>>(),
            vec![(8, 20)]
        );
    }

    #[test]
    fn the_wait_fraction_follows_the_timer_or_the_last_arrival() {
        let (system, character) = mock_keys();
        let team = vec![MissionMember::Character(character)];
        let mut timed = ActiveMission::timed(
            0,
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            team,
            system,
            10,
        );
        timed.wait_start = 2;
        assert!((timed.wait_fraction(6, None) - 0.5).abs() < 1e-6);
        assert!((timed.wait_fraction(40, None) - 1.0).abs() < 1e-6);
        assert!(!timed.in_transit());

        let mut travelling = timed.clone();
        travelling.phase = PHASE_TRANSIT;
        travelling.timer_due = None;
        assert!(travelling.in_transit());
        assert!((travelling.wait_fraction(4, Some(6)) - 0.5).abs() < 1e-6);
        assert!((travelling.wait_fraction(4, None) - 1.0).abs() < 1e-6);

        let mut due_now = timed;
        due_now.timer_due = Some(2);
        assert!((due_now.wait_fraction(2, None) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_recruitment_at_a_friendly_system_runs_its_course() {
        // FUN_00592600 rule 3: an own-side target reads column 15 (1 for
        // Recruitment), not column 16 (0).
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        world.systems[here].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Alliance);
        world.mission_records = vec![crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x5500_0016),
            timer_min_days: 5,
            timer_spread_days: 0,
            repeats: false,
            hidden: false,
            detection_phases: true,
            can_resign: true,
            leave_rule: 0,
            rules: crate::world::MissionTargetRules {
                container_loss_ends: true,
                needs_populated_container: true,
                target_loss_ends: true,
                own_side_target: true,
                revolting_target: true,
                calm_target: true,
                ..Default::default()
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0,
                character_mask: 0x1_0000,
            },
        }];
        let recruiter = agent_at(&mut world, here, true);
        let mut state = MissionState::new();
        state
            .dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Recruitment,
                    MissionFaction::Alliance,
                    recruiter,
                    here,
                    None,
                    0,
                ),
                &mut world,
            )
            .expect("a free recruiter");
        step(&mut state, &world, 1, &[0.0]);

        let advance = step(&mut state, &world, 5, &[0.99]);

        assert_eq!(advance.results.len(), 1);
        assert_eq!(advance.ended[0].end_code, 1);
    }

    #[test]
    fn a_target_that_leaves_the_system_ends_the_mission_with_code_six() {
        // FUN_00592600 rule 6: the target's location differs from C.
        let mut world = minimal_world();
        let elsewhere = system_at(&mut world, 50);
        let (mut state, officer) = waiting_assassination(&mut world, true);
        world.characters[officer].current_system = Some(elsewhere);

        let advance = step(&mut state, &world, 11, &[0.0]);

        assert!(advance.results.is_empty());
        assert_eq!(advance.ended[0].end_code, 6);
    }

    #[test]
    fn another_member_s_transit_does_not_count_as_the_target_travelling() {
        // FUN_00592600 rule 5 reads the target's own en route bit.
        let mut world = minimal_world();
        let (mut state, _) = waiting_assassination(&mut world, true);
        let bystander = agent_at(&mut world, state.missions()[0].target_system, true);
        state.en_route.push(MemberTransit {
            member: MissionMember::Character(bystander),
            mission_id: 99,
            to: state.missions()[0].target_system,
            arrival: 100,
            abode: None,
        });

        let advance = step(&mut state, &world, 11, &[0.99]);

        assert_eq!(advance.results.len(), 1);
        assert_eq!(advance.ended[0].end_code, 1);
    }

    #[test]
    fn a_mission_whose_team_is_gone_ends_with_code_five() {
        // FUN_00522480.c:42-52: no team member stands without a request once
        // the killed envoy is deleted, so the validator gives 5 before the
        // FUN_005830a0 summary.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let (mut state, envoy) = diplomacy_from(&mut world, here, here);
        step(&mut state, &world, 1, &[0.0]);
        world.characters[envoy].mark_killed();

        let advance = step(&mut state, &world, 5, &[0.99, 0.0]);

        assert!(advance.results.is_empty());
        assert_eq!(advance.ended[0].end_code, 5);
    }

    #[test]
    fn each_tick_of_an_advance_gives_every_mission_its_own_window() {
        // port: mission i on event e draws from (e * n + i) * ROLLS_PER_MISSION.
        let (system, _) = mock_keys();
        let mut world = minimal_world();
        let character = agent_at(&mut world, system, true);
        let mut state = MissionState::new();
        world.mission_records = vec![diplomacy_record()];
        for (id, due) in [(0, 2), (1, 50)] {
            state.push_timed(ActiveMission::timed(
                id,
                MissionKind::Diplomacy,
                MissionFaction::Alliance,
                vec![MissionMember::Character(character)],
                system,
                due,
            ));
        }
        let mut rolls = vec![0.99; 2 * 2 * ROLLS_PER_MISSION];
        rolls[2 * ROLLS_PER_MISSION] = 0.0;

        MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            &[TickEvent { tick: 1 }, TickEvent { tick: 2 }],
            &rolls,
        );

        // Mission 0 repeats on event 1 (tick 2) and re-arms from roll 0.0.
        assert_eq!(state.missions()[0].timer_due, Some(2 + 5));
    }

    #[test]
    fn a_validator_code_stands_when_the_container_is_also_destroyed() {
        // FUN_00545240.c:85-111 sets code 7 only when the validator set
        // none: a freed prisoner still ends the Rescue with code 8.
        let mut world = minimal_world();
        let (mut state, prisoner) = waiting_rescue(&mut world);
        world.characters[prisoner].is_captive = false;
        world.characters[prisoner].captured_by = None;
        let here = state.missions()[0].target_system;
        world.systems[here].is_destroyed = true;

        let advance = step(&mut state, &world, 2, &[]);

        assert_eq!(advance.ended[0].end_code, 8);
    }

    /// A Rescue sent from `(0, 0)` to a prisoner at `(100, 0)`, its hero
    /// travelling after day 1 and landing on day 20.
    fn travelling_rescue(world: &mut GameWorld) -> (MissionState, SystemKey) {
        let from = system_at(world, 0);
        let to = system_at(world, 100);
        world.mission_records = vec![rescue_record()];
        let hero = agent_at(world, from, true);
        let prisoner = agent_at(world, to, true);
        world.characters[prisoner].is_captive = true;
        world.characters[prisoner].captured_by = Some(crate::dat::Faction::Empire);
        let mut state = MissionState::new();
        state
            .dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Rescue,
                    MissionFaction::Alliance,
                    hero,
                    to,
                    Some(prisoner),
                    0,
                ),
                world,
            )
            .expect("a free hero");
        step(&mut state, world, 1, &[]);
        (state, to)
    }

    #[test]
    fn a_container_destroyed_while_the_members_travel_never_gives_code_seven() {
        // FUN_00545240.c:100-104 gives code 7 only when FUN_00520ae0 holds:
        // the phase at +0x54 -> +0x1c (written by FUN_00524b70) is above 6.
        // The check runs once, on the destruction; Rescue's column 11 is 0.
        let mut world = minimal_world();
        let (mut state, to) = travelling_rescue(&mut world);
        world.systems[to].is_destroyed = true;

        assert!(step(&mut state, &world, 2, &[]).ended.is_empty());
        step(&mut state, &world, 20, &[1.0]);
        let advance = step(&mut state, &world, 21, &[]);

        assert!(advance.ended.is_empty());
        let mission = &state.missions()[0];
        assert_eq!((mission.phase, mission.end_code), (PHASE_TIMER, 0));
    }

    #[test]
    fn a_container_destroyed_after_the_members_land_ends_the_mission_with_code_seven() {
        // FUN_00545240.c:100-104: phase 8 is above 6, so code 7 even though
        // Rescue's column 11 does not end it through the validator.
        let mut world = minimal_world();
        let (mut state, _) = waiting_rescue(&mut world);
        let here = state.missions()[0].target_system;
        world.systems[here].is_destroyed = true;

        let advance = step(&mut state, &world, 2, &[]);

        assert_eq!(advance.ended[0].end_code, 7);
    }

    #[test]
    fn a_live_target_s_change_waits_for_the_next_phase_change() {
        // FUN_00545240 runs the validator only for a destroyed container or
        // target; a freed prisoner is seen when the timer fires.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        world.mission_records = vec![rescue_record()];
        let hero = agent_at(&mut world, here, true);
        let prisoner = agent_at(&mut world, here, true);
        let mut rescue = ActiveMission::timed(
            0,
            MissionKind::Rescue,
            MissionFaction::Alliance,
            vec![MissionMember::Character(hero)],
            here,
            7,
        );
        rescue.target_character = Some(prisoner);
        let mut state = MissionState::new();
        state.push_timed(rescue);

        assert!(step(&mut state, &world, 2, &[]).ended.is_empty());
        assert_eq!(step(&mut state, &world, 7, &[]).ended[0].end_code, 8);
    }

    #[test]
    fn a_transit_ends_once_any_member_is_no_longer_travelling() {
        // FUN_00522280.c:66-100: the mission waits only while every member
        // still travels; one member off the road sets the ready bit.
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        world.mission_records = vec![diplomacy_record()];
        let first = agent_at(&mut world, from, true);
        let second = agent_at(&mut world, from, true);
        let mut state = MissionState::new();
        state
            .dispatch_guarded(
                MissionRequest {
                    kind: MissionKind::Diplomacy,
                    faction: MissionFaction::Alliance,
                    team: vec![
                        MissionMember::Character(first),
                        MissionMember::Character(second),
                    ],
                    decoys: vec![],
                    target_system: to,
                    target_character: None,
                    target_object: None,
                    tick: 0,
                },
                &mut world,
            )
            .expect("two free envoys");
        step(&mut state, &world, 1, &[]);
        state
            .en_route
            .retain(|transit| transit.member != MissionMember::Character(first));

        step(&mut state, &world, 2, &[0.0]);

        assert_eq!(state.missions()[0].phase, PHASE_TIMER);
    }

    #[test]
    fn a_team_killed_on_the_way_counts_as_arrived() {
        // FUN_00522280: the original deletes a killed member, so no member
        // is left travelling and the wait ends at once; phase 5's validator
        // then ends the teamless mission with code 5 (FUN_00522480.c:42-52).
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let (mut state, envoy) = diplomacy_from(&mut world, from, to);
        step(&mut state, &world, 1, &[]);
        world.characters[envoy].mark_killed();

        let advance = step(&mut state, &world, 2, &[0.0]);

        assert_eq!(
            advance
                .ended
                .iter()
                .map(|e| (e.end_code, e.tick))
                .collect::<Vec<_>>(),
            vec![(5, 2)]
        );
    }

    #[test]
    fn members_with_no_origin_stay_where_they_are() {
        // FUN_00556430.c:31-33: a null origin (+0x6c) starts no transit and
        // moves no one.
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let (mut state, envoy) = diplomacy_from(&mut world, from, to);
        world.characters[envoy].current_system = None;

        let advance = step(&mut state, &world, 1, &[0.0]);

        assert!(advance.effects.is_empty());
        assert!(state.en_route().is_empty());
        let mission = &state.missions()[0];
        assert_eq!((mission.origin, mission.phase), (None, PHASE_TIMER));
    }

    #[test]
    fn a_cancelled_mission_s_members_travel_on_and_land_at_its_target() {
        // port: release frees the flags, and each member's own transit runs
        // on as it does for a mission that ends (MemberTransit); the
        // original's cancel path is untraced.
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let (mut state, envoy) = diplomacy_from(&mut world, from, to);
        step(&mut state, &world, 1, &[]);
        let id = state.missions()[0].id;

        state.release(id, &mut world).expect("the mission runs");

        let member = MissionMember::Character(envoy);
        assert!(!world.characters[envoy].on_mission);
        assert!(state.is_en_route(member));
        assert_eq!(
            state.dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Diplomacy,
                    MissionFaction::Alliance,
                    envoy,
                    from,
                    None,
                    2,
                ),
                &mut world,
            ),
            Err(MissionRefusal::MemberUnavailable(member))
        );
        let arrival = step(&mut state, &world, 20, &[]);
        assert_eq!(
            arrival.effects,
            vec![MissionEffect::MemberMoved {
                member,
                to: Some(to)
            }]
        );
    }

    #[test]
    fn a_killed_character_stays_where_it_died() {
        // Character::mark_killed takes the dead off the map; a late
        // arrival leaves them there.
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let pilot = agent_at(&mut world, from, true);
        world.characters[pilot].mark_killed();
        let resting = world.characters[pilot].current_system;

        move_member(&mut world, MissionMember::Character(pilot), Some(to));

        assert_eq!(world.characters[pilot].current_system, resting);
        assert_ne!(resting, Some(to));
    }

    #[test]
    fn member_effects_move_then_free_the_member() {
        let mut world = minimal_world();
        let from = system_at(&mut world, 0);
        let to = system_at(&mut world, 100);
        let envoy = agent_at(&mut world, from, true);
        world.characters[envoy].on_mission = true;
        let member = MissionMember::Character(envoy);

        apply_advance_effects(
            &mut world,
            &[
                MissionEffect::MemberMoved {
                    member,
                    to: Some(to),
                },
                MissionEffect::MemberAvailable { member },
                MissionEffect::CharacterKilled {
                    character: envoy,
                    faction: MissionFaction::Empire,
                },
            ],
        );

        let character = &world.characters[envoy];
        assert_eq!(character.current_system, Some(to));
        assert!(!character.on_mission);
        assert!(!character.is_killed);
    }

    #[test]
    fn detection_effects_injure_and_raise_the_force_experience() {
        // The detection's effects reach the character: slot +0x2f0
        // (FUN_004ef6d0) adds the injury, FUN_004ee150 the Force
        // experience, clamped to 0x7fff.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let luke = agent_at(&mut world, here, true);
        world.characters[luke].force_base = 50;
        apply_advance_effects(
            &mut world,
            &[
                MissionEffect::CharacterInjured {
                    character: luke,
                    injury: 7,
                },
                MissionEffect::ForceExperienceGained {
                    character: luke,
                    amount: 1,
                },
            ],
        );
        assert_eq!(world.characters[luke].injury, 7);
        assert_eq!(world.characters[luke].force_base, 51);
        assert_eq!(world.characters[luke].force_level(), 51);
        world.characters[luke].force_base = 0x7fff;
        apply_advance_effects(
            &mut world,
            &[MissionEffect::ForceExperienceGained {
                character: luke,
                amount: 1,
            }],
        );
        assert_eq!(world.characters[luke].force_base, 0x7fff);
    }

    #[test]
    fn detection_effects_take_the_prisoner_and_destroy_the_special_force() {
        // Slot +0x20c holds the captured character at the location for the
        // captor; FUN_00503eb0 destroys a captured special force.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let elsewhere = system_at(&mut world, 100);
        let spy = agent_at(&mut world, elsewhere, true);
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: elsewhere,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![spy],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.characters[spy].current_fleet = Some(fleet);
        let unit = |world: &mut GameWorld| {
            let key = world.special_forces.insert(crate::world::SpecialForceUnit {
                class_dat_id: crate::ids::DatId::new(0x1400_0001),
                is_alliance: true,
                skills: [0; 8],
                on_mission: true,
            });
            world.systems[here].special_forces.push(key);
            key
        };
        let (lost, kept) = (unit(&mut world), unit(&mut world));

        apply_advance_effects(
            &mut world,
            &[
                MissionEffect::CharacterCaptured {
                    character: spy,
                    captured_by: MissionFaction::Empire,
                    at_system: here,
                },
                MissionEffect::SpecialForceDestroyed { unit: lost },
            ],
        );

        let prisoner = &world.characters[spy];
        assert!(prisoner.is_captive);
        assert_eq!(prisoner.captured_by, Some(crate::dat::Faction::Empire));
        assert_eq!(
            (prisoner.current_system, prisoner.current_fleet),
            (Some(here), None)
        );
        assert!(world.fleets[fleet].characters.is_empty());
        assert!(!world.special_forces.contains_key(lost));
        assert_eq!(world.systems[here].special_forces, vec![kept]);
        assert!(world.special_forces.contains_key(kept));
    }

    #[test]
    fn a_mission_with_members_of_both_sides_skips_the_running_checks() {
        // FUN_005830a0 status 0x14: no end code, though Diplomacy at a
        // system the opponent holds would end 8 (column 17).
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        world.systems[here].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        world.mission_records = vec![diplomacy_record()];
        let envoy = MissionMember::Character(agent_at(&mut world, here, true));
        let mut mission = ActiveMission::timed(
            0,
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            vec![envoy],
            here,
            5,
        );
        let uprisings = UprisingState::default();
        assert_eq!(
            running_end_code(&mission, &world, &uprisings, &ResearchState::default(), &[]),
            8
        );

        mission.decoys = vec![MissionMember::Character(agent_at(&mut world, here, false))];

        assert_eq!(
            running_end_code(&mission, &world, &uprisings, &ResearchState::default(), &[]),
            0
        );
    }

    #[test]
    fn a_draw_past_the_timers_reads_the_fallback_not_the_stream_seed() {
        // port: slot STREAM_SEED_ROLL only seeds the mission's stream.
        let window = [0.1, 0.2, 0.9];
        let mut draws = MissionRolls {
            rolls: &window,
            next: 0,
            stream: crate::mission_detection::MissionRng::seeded(0.9, 0),
        };
        let taken: Vec<f64> = (0..3).map(|_| draws.next()).collect();
        assert_eq!(taken, vec![0.1, 0.2, 0.5]);
    }

    // --- Member legality (FUN_005830a0, FUN_00583320) ---

    /// A special force of a class whose SPECFCSD mission mask is `mask`.
    fn unit_of_mask(world: &mut GameWorld, mask: u32, is_alliance: bool) -> MissionMember {
        let class_dat_id = crate::ids::DatId::new(0x3c00_0000 | mask);
        world.special_force_classes.insert(
            class_dat_id,
            crate::world::SpecialForceClassDef {
                skills: [skill_pair(0); 8],
                mission_mask: mask,
            },
        );
        MissionMember::SpecialForce(world.special_forces.insert(crate::world::SpecialForceUnit {
            class_dat_id,
            is_alliance,
            skills: [0; 8],
            on_mission: false,
        }))
    }

    #[test]
    fn a_special_force_is_allowed_only_when_the_record_takes_every_bit_of_its_class() {
        // FUN_00583320: the members' SPECFCSD masks must be a subset of the
        // record's +0x48. The shipped Rescue record takes 0x802.
        let mut world = minimal_world();
        let rescue = rescue_record().members;
        let commandos = unit_of_mask(&mut world, 0x2, true);
        let spies = unit_of_mask(&mut world, 0x8, true);

        assert!(members_allowed(&world, rescue, [commandos]));
        assert!(!members_allowed(&world, rescue, [spies]));
        assert!(!members_allowed(&world, rescue, [commandos, spies]));
    }

    #[test]
    fn a_record_with_no_special_force_bits_takes_only_characters() {
        // The shipped Diplomacy record has +0x48 = 0 and +0x4c = 0x10000.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let diplomacy = diplomacy_record().members;
        let envoy = MissionMember::Character(agent_at(&mut world, here, true));
        let unit = unit_of_mask(&mut world, 0x2, true);

        assert!(members_allowed(&world, diplomacy, [envoy]));
        assert!(!members_allowed(&world, diplomacy, [envoy, unit]));
        let no_characters = crate::world::MissionMemberRules {
            character_mask: 0,
            ..diplomacy
        };
        assert!(!members_allowed(&world, no_characters, [envoy]));
    }

    #[test]
    fn members_must_share_one_side_that_the_record_runs_for() {
        // FUN_00583320: both sides fail (0x14), no side fails (0x16), and a
        // side needs the record's +0x40 (Alliance) or +0x44 (Empire). The
        // shipped Assassination record is Empire-only.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let rules = crate::world::MissionMemberRules {
            alliance: false,
            empire: true,
            special_force_mask: 0x800,
            character_mask: 0x1_0000,
        };
        let rebel = MissionMember::Character(agent_at(&mut world, here, true));
        let imperial = MissionMember::Character(agent_at(&mut world, here, false));

        assert!(members_allowed(&world, rules, [imperial]));
        assert!(!members_allowed(&world, rules, [rebel]));
        assert!(!members_allowed(&world, rules, [imperial, rebel]));
        assert!(!members_allowed(&world, rules, []));
        // The shipped DS Sabotage record is Alliance-only.
        let alliance_only = crate::world::MissionMemberRules {
            alliance: true,
            empire: false,
            ..rules
        };
        assert!(members_allowed(&world, alliance_only, [rebel]));
        assert!(!members_allowed(&world, alliance_only, [imperial]));
    }

    #[test]
    fn a_request_whose_members_the_record_does_not_admit_is_refused() {
        // FUN_005420d0 runs the member check on the team, decoys, and
        // captured before anything is created.
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        world.mission_records = vec![diplomacy_record()];
        let envoy = MissionMember::Character(agent_at(&mut world, here, true));
        let unit = unit_of_mask(&mut world, 0x2, true);
        world.systems[here].special_forces.push(match unit {
            MissionMember::SpecialForce(key) => key,
            MissionMember::Character(_) => unreachable!(),
        });
        let mut state = MissionState::new();

        let refused = state.dispatch_guarded(
            MissionRequest {
                decoys: vec![unit],
                ..MissionRequest::single(
                    MissionKind::Diplomacy,
                    MissionFaction::Alliance,
                    envoy.character().unwrap(),
                    here,
                    None,
                    0,
                )
            },
            &mut world,
        );

        assert_eq!(refused, Err(MissionRefusal::MembersNotAllowed));
        assert!(state.is_empty());
        assert!(!world.characters[envoy.character().unwrap()].on_mission);
    }

    // --- The dialog's mission list (FUN_004f5380, FUN_005422f0) ---

    /// An Alliance envoy at an Alliance system, with records in `records`.
    /// The shipped MISSNSD Dagobah record `0x71000043`: Alliance only,
    /// hidden, 100 days, no resigning, an "other side" target.
    fn dagobah_record() -> crate::world::MissionRecord {
        crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x7100_0043),
            timer_min_days: 100,
            timer_spread_days: 0,
            repeats: false,
            hidden: true,
            detection_phases: true,
            can_resign: false,
            leave_rule: 0,
            rules: crate::world::MissionTargetRules {
                container_loss_ends: true,
                needs_populated_container: false,
                target_loss_ends: true,
                own_side_target: false,
                other_side_target: true,
                opponent_target: false,
                revolting_target: false,
                calm_target: false,
                prisoner_target: false,
                free_target: false,
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: false,
                special_force_mask: 0,
                character_mask: 0x1_0000,
            },
        }
    }

    /// Luke (Force 50) at an Alliance system, a second Alliance system
    /// further off, the Dagobah abode and the Dagobah record.
    fn dagobah_world() -> (
        GameWorld,
        SystemKey,
        SystemKey,
        CharacterKey,
        crate::ids::AbodeKey,
    ) {
        let mut world = minimal_world();
        let home = system_at(&mut world, 0);
        let far = system_at(&mut world, 500);
        for system in [home, far] {
            world.systems[system].control =
                crate::world::ControlKind::Controlled(crate::dat::Faction::Alliance);
        }
        world.mission_records = vec![dagobah_record()];
        let luke = agent_at(&mut world, home, true);
        world.characters[luke].dat_id = crate::ids::DatId::new(0x242);
        world.characters[luke].force_base = 50;
        let dagobah = world.abodes.insert(crate::world::Abode {
            dat_id: crate::ids::DatId::new(0x441),
            name: "Dagobah".into(),
            side: crate::dat::Faction::Neutral,
            special_forces: Vec::new(),
        });
        (world, home, far, luke, dagobah)
    }

    /// MISSNSD `0x65000083` (Bounty): Empire only, hidden, 0 + rand(0..=0)
    /// days, no detection, special forces of mask `0x1000`, an opponent's
    /// free character.
    fn bounty_record() -> crate::world::MissionRecord {
        crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x6500_0083),
            timer_min_days: 0,
            timer_spread_days: 0,
            repeats: false,
            hidden: true,
            detection_phases: false,
            can_resign: false,
            leave_rule: 0,
            rules: crate::world::MissionTargetRules {
                container_loss_ends: false,
                needs_populated_container: false,
                target_loss_ends: true,
                own_side_target: false,
                other_side_target: false,
                opponent_target: true,
                revolting_target: false,
                calm_target: false,
                prisoner_target: false,
                free_target: true,
            },
            members: crate::world::MissionMemberRules {
                alliance: false,
                empire: true,
                special_force_mask: 0x1000,
                character_mask: 0x1_0000,
            },
        }
    }

    /// Han at an Alliance system, Jabba's Palace, the Bounty Hunters class
    /// and record, and an RLEVADTB of `evade` percent.
    fn bounty_world(evade: u32) -> (GameWorld, SystemKey, CharacterKey, crate::ids::AbodeKey) {
        let mut world = minimal_world();
        let home = system_at(&mut world, 0);
        world.systems[home].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Alliance);
        world.mission_records = vec![bounty_record()];
        world.mission_tables.insert(
            "RLEVADTB".into(),
            crate::world::MstbTable::new(vec![crate::world::MstbEntry {
                threshold: 0,
                value: evade,
            }]),
        );
        world.special_force_classes.insert(
            crate::ids::DatId::new(0x3c00_0009),
            crate::world::SpecialForceClassDef {
                skills: [crate::world::SkillPair {
                    base: 0,
                    variance: 0,
                }; 8],
                mission_mask: 0x1000,
            },
        );
        let han = agent_at(&mut world, home, true);
        world.characters[han].dat_id = crate::ids::DatId::new(0x243);
        let palace = world.abodes.insert(crate::world::Abode {
            dat_id: crate::ids::DatId::new(0x481),
            name: "Jabba's Palace".into(),
            side: crate::dat::Faction::Empire,
            special_forces: Vec::new(),
        });
        (world, home, han, palace)
    }

    /// MISSNSD `0x64000044` (Palace): Alliance only, hidden, an opponent's
    /// prisoner as target. port: the test fixes the timer at 5 days.
    fn palace_record() -> crate::world::MissionRecord {
        crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x6400_0044),
            timer_min_days: 5,
            timer_spread_days: 0,
            repeats: false,
            hidden: true,
            detection_phases: true,
            can_resign: false,
            leave_rule: 0,
            rules: crate::world::MissionTargetRules {
                container_loss_ends: false,
                needs_populated_container: false,
                target_loss_ends: true,
                own_side_target: false,
                other_side_target: false,
                opponent_target: true,
                revolting_target: false,
                calm_target: false,
                prisoner_target: true,
                free_target: false,
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: false,
                special_force_mask: 0,
                character_mask: 0x1_0000,
            },
        }
    }

    /// Han held in Jabba's Palace (the Empire's), another prisoner there,
    /// and Luke at an Alliance system with `espionage` and `combat`.
    fn palace_world(
        espionage: u32,
        combat: u32,
    ) -> (
        GameWorld,
        SystemKey,
        CharacterKey,
        CharacterKey,
        CharacterKey,
        crate::ids::AbodeKey,
    ) {
        let mut world = minimal_world();
        let home = system_at(&mut world, 0);
        world.systems[home].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Alliance);
        world.mission_records = vec![palace_record()];
        let palace = world.abodes.insert(crate::world::Abode {
            dat_id: crate::ids::DatId::new(0x481),
            name: "Jabba's Palace".into(),
            side: crate::dat::Faction::Empire,
            special_forces: Vec::new(),
        });
        let held = |world: &mut GameWorld, index: u32| {
            let key = agent_at(world, home, true);
            let c = &mut world.characters[key];
            c.dat_id = crate::ids::DatId::new(index);
            c.is_captive = true;
            c.captured_by = Some(crate::dat::Faction::Empire);
            c.escape_barred = true;
            c.current_system = None;
            c.current_abode = Some(palace);
            key
        };
        let han = held(&mut world, 0x243);
        let other = held(&mut world, 0x343);
        let luke = agent_at(&mut world, home, true);
        let c = &mut world.characters[luke];
        c.dat_id = crate::ids::DatId::new(0x242);
        c.espionage.base = espionage;
        c.combat.base = combat;
        (world, home, han, other, luke, palace)
    }

    /// Run the Palace mission from `day` until it ends, applying its
    /// effects (a rescue as the integrator does); returns its results.
    fn run_palace(state: &mut MissionState, world: &mut GameWorld, day: u64) -> Vec<MissionResult> {
        let mut results = Vec::new();
        for today in day..day + 30 {
            let advance = step(state, world, today, &[]);
            for result in &advance.results {
                apply_advance_effects(world, &result.effects);
                for effect in &result.effects {
                    if let MissionEffect::CharacterRescued { character, .. } = effect {
                        world.characters[*character].is_captive = false;
                        world.characters[*character].captured_by = None;
                    }
                }
            }
            apply_advance_effects(world, &advance.effects);
            results.extend(advance.results);
            if !advance.ended.is_empty() {
                break;
            }
        }
        results
    }

    /// MISSNSD `0x73000082` (Pickup): Empire only, hidden, 0 + rand(0..=0)
    /// days, the Empire's own target.
    fn pickup_record() -> crate::world::MissionRecord {
        let mut record = palace_record();
        record.dat_id = crate::ids::DatId::new(0x7300_0082);
        record.timer_min_days = 0;
        record.rules.opponent_target = false;
        record.rules.own_side_target = true;
        record.members.alliance = false;
        record.members.empire = true;
        record
    }

    #[test]
    fn the_pickup_takes_the_palace_s_prisoners_to_where_vader_came_from() {
        // FUN_0056da80: PickupInProgress (notification 0x231); each prisoner
        // of the palace held for the Empire and on no mission joins the
        // mission and takes Vader's +0x70; FUN_004ef260 then autoroutes it
        // there (FUN_00554ab0). Vader, of the palace's side, stays.
        let (mut world, home, han, other, _, palace) = palace_world(0, 0);
        world.mission_records = vec![pickup_record()];
        world.characters[other].on_mission = true;
        let rescuer = agent_at(&mut world, home, true);
        world.characters[rescuer].is_captive = true;
        world.characters[rescuer].captured_by = Some(crate::dat::Faction::Alliance);
        world.characters[rescuer].current_system = None;
        world.characters[rescuer].current_abode = Some(palace);
        let vader = agent_at(&mut world, home, false);
        world.characters[vader].dat_id = crate::ids::DatId::new(0x281);
        let mut state = MissionState::new();
        let pickup = crate::jabba::JabbaStep::Pickup {
            vader,
            from: home,
            palace,
            tick: 1,
        };
        crate::jabba::perform(&pickup, &mut state, &mut world).expect("dispatched");
        assert_eq!(state.missions()[0].kind, MissionKind::Pickup);
        let mut results = Vec::new();
        let mut ended_on = None;
        for today in 1..10 {
            let advance = step(&mut state, &world, today, &[]);
            for result in &advance.results {
                apply_advance_effects(&mut world, &result.effects);
            }
            apply_advance_effects(&mut world, &advance.effects);
            results.extend(advance.results);
            if !advance.ended.is_empty() {
                ended_on = Some(today);
                break;
            }
        }
        let effects: Vec<&MissionEffect> = results.iter().flat_map(|r| &r.effects).collect();
        assert_eq!(
            effects
                .iter()
                .filter(|e| matches!(e, MissionEffect::PickupInProgress))
                .count(),
            1
        );
        assert!(effects.contains(&&MissionEffect::PrisonerPickedUp { character: han }));
        for absent in [other, rescuer] {
            assert!(!effects.contains(&&MissionEffect::PrisonerPickedUp { character: absent }));
        }
        assert_eq!(
            results.last().map(|r| r.outcome),
            Some(MissionOutcome::Success)
        );
        let day = ended_on.expect("ended");
        assert!(state.is_autorouting(han));
        let transit = state.escapees.iter().find(|t| t.character == han).unwrap();
        assert_eq!((transit.to, transit.arrival), (home, day + 1));
        assert!(!state.is_autorouting(vader));
        assert!(world.characters[han].is_captive);
        let arrived = MissionSystem::advance_escapes(&mut state, &world, day + 1, &[]);
        apply_advance_effects(&mut world, &arrived);
        let c = &world.characters[han];
        assert!(c.is_captive);
        assert_eq!((c.current_system, c.current_abode), (Some(home), None));
    }

    #[test]
    fn the_palace_chance_is_espionage_and_combat_each_over_three() {
        // FUN_0056e620 -> FUN_0055c910: espionage / GNPRTB 6154 (3) +
        // combat / GNPRTB 6155 (3), in integers.
        let (world, home, han, _, luke, _) = palace_world(50, 41);
        assert_eq!(
            member_chance(
                &world,
                MissionKind::Palace,
                MissionFaction::Alliance,
                home,
                Some(han),
                MissionMember::Character(luke),
            ),
            16 + 13
        );
    }

    #[test]
    fn a_palace_rescue_frees_han_and_every_prisoner_there() {
        // FUN_0056e650: on a success the rescuer's base combat and
        // espionage rise by GNPRTB 6170 and 6169 (1), he frees Han (slot
        // +0x210) and every other prisoner of the palace (FUN_00521050);
        // the freed and the rescuer of another side than the palace's
        // autoroute out (FUN_004ef260).
        let (mut world, home, han, other, luke, palace) = palace_world(300, 0);
        // An Empire prisoner keeps its side; the killed, a free guest and
        // a prisoner held elsewhere are not freed.
        world.characters[other].is_alliance = false;
        world.characters[other].is_empire = true;
        let mut extra = |edit: fn(&mut Character)| {
            let key = agent_at(&mut world, home, true);
            let c = &mut world.characters[key];
            c.is_captive = true;
            c.current_system = None;
            c.current_abode = Some(palace);
            edit(c);
            key
        };
        let killed = extra(|c| c.is_killed = true);
        let guest = extra(|c| c.is_captive = false);
        let elsewhere = extra(|c| c.current_abode = None);
        let neutral = extra(|c| c.is_alliance = false);
        let mut state = MissionState::new();
        let rescue = crate::jabba::JabbaStep::Rescue {
            rescuer: luke,
            han,
            from: home,
            palace,
            tick: 1,
        };
        crate::jabba::perform(&rescue, &mut state, &mut world).expect("dispatched");
        assert_eq!(state.missions()[0].kind, MissionKind::Palace);
        assert_eq!(state.missions()[0].target_abode, Some(palace));
        let results = run_palace(&mut state, &mut world, 1);
        let effects: Vec<&MissionEffect> = results.iter().flat_map(|r| &r.effects).collect();
        for (character, side) in [
            (han, MissionFaction::Alliance),
            (other, MissionFaction::Empire),
            (neutral, MissionFaction::Alliance),
        ] {
            assert!(effects.iter().any(|effect| matches!(effect,
                MissionEffect::CharacterRescued { character: c, returned_to, .. }
                    if *c == character && *returned_to == side)));
        }
        for character in [killed, guest, elsewhere] {
            assert!(!effects.iter().any(|effect| matches!(effect,
                MissionEffect::CharacterRescued { character: c, .. } if *c == character)));
        }
        assert!(effects.contains(&&MissionEffect::SkillRaised {
            character: luke,
            skill: Skill::Combat,
            amount: 1,
        }));
        assert!(effects.contains(&&MissionEffect::SkillRaised {
            character: luke,
            skill: Skill::Espionage,
            amount: 1,
        }));
        for character in [han, luke] {
            assert!(state.is_autorouting(character), "{character:?} leaves");
        }
        // Of the palace's own side, the freed Empire prisoner stays.
        assert!(!state.is_autorouting(other));
        assert!(!world.characters[luke].is_captive);
    }

    #[test]
    fn a_failed_palace_rescue_leaves_the_rescuer_captive_there_for_good() {
        // FUN_0056e870: with no success, each member is taken by the
        // mission's captor (slot +0x20c) and loses CanEscape; held for the
        // palace's side, he does not leave it (FUN_004ef260).
        let (mut world, home, han, _, luke, palace) = palace_world(0, 0);
        let mut state = MissionState::new();
        let rescue = crate::jabba::JabbaStep::Rescue {
            rescuer: luke,
            han,
            from: home,
            palace,
            tick: 1,
        };
        crate::jabba::perform(&rescue, &mut state, &mut world).expect("dispatched");
        let results = run_palace(&mut state, &mut world, 1);
        let effects: Vec<&MissionEffect> = results.iter().flat_map(|r| &r.effects).collect();
        assert!(effects.contains(&&MissionEffect::CharacterCaptured {
            character: luke,
            captured_by: MissionFaction::Empire,
            at_system: home,
        }));
        let c = &world.characters[luke];
        assert!(c.is_captive && c.escape_barred);
        assert_eq!(c.captured_by, Some(crate::dat::Faction::Empire));
        assert_eq!(c.current_abode, Some(palace));
        assert_eq!(c.current_system, None);
        assert!(!state.is_autorouting(luke));
        assert!(world.characters[han].is_captive);
    }

    #[test]
    fn the_palace_mission_ends_once_han_is_no_longer_held_there() {
        // FUN_00523450 rules 5 and 6: the target must stay in the
        // mission's container.
        let (mut world, home, han, _, luke, palace) = palace_world(300, 0);
        let mut state = MissionState::new();
        let rescue = crate::jabba::JabbaStep::Rescue {
            rescuer: luke,
            han,
            from: home,
            palace,
            tick: 1,
        };
        crate::jabba::perform(&rescue, &mut state, &mut world).expect("dispatched");
        world.characters[han].is_captive = false;
        let results = run_palace(&mut state, &mut world, 1);
        assert!(results
            .iter()
            .flat_map(|r| &r.effects)
            .all(|effect| !matches!(effect, MissionEffect::CharacterRescued { .. })));
        assert!(!world.characters[luke].is_captive);
    }

    #[test]
    fn the_bounty_hit_acts_only_on_han() {
        // FUN_00575de0 acts on a target of family 0x33 alone.
        let (mut world, home, han, palace) = bounty_world(0);
        world.characters[han].dat_id = crate::ids::DatId::new(0x242);
        let mut state = MissionState::new();
        let hunt = crate::jabba::JabbaStep::BountyHunt {
            han,
            palace,
            at: home,
            tick: 1,
        };
        crate::jabba::perform(&hunt, &mut state, &mut world).expect("dispatched");
        for day in 1..=10 {
            let advance = step(&mut state, &world, day, &[]);
            apply_advance_effects(&mut world, &advance.effects);
            for result in &advance.results {
                assert!(!result
                    .effects
                    .iter()
                    .any(|effect| matches!(effect, MissionEffect::BountyAttack { .. })));
            }
        }
        assert!(!world.characters[han].is_captive);
    }

    #[test]
    fn a_han_who_evades_the_bounty_hunters_stays_free() {
        // FUN_00575de0: an evade (RLEVADTB 100) leaves the result 2 and Han
        // free; BountyAttack still fires (notification 0x212).
        let (mut world, home, han, palace) = bounty_world(100);
        let mut state = MissionState::new();
        let hunt = crate::jabba::JabbaStep::BountyHunt {
            han,
            palace,
            at: home,
            tick: 1,
        };
        crate::jabba::perform(&hunt, &mut state, &mut world).expect("dispatched");
        let mut seen = None;
        for day in 1..=10 {
            let advance = step(&mut state, &world, day, &[]);
            apply_advance_effects(&mut world, &advance.effects);
            for result in &advance.results {
                apply_advance_effects(&mut world, &result.effects);
                seen = Some(result.clone());
            }
        }
        let result = seen.expect("the hunters reached Han");
        assert!(result.effects.contains(&MissionEffect::BountyAttack {
            character: han,
            captured: false,
        }));
        assert_eq!(result.outcome, MissionOutcome::Failure);
        let c = &world.characters[han];
        assert!(!c.is_captive && !c.bounty_captured && !c.escape_barred);
    }

    #[test]
    fn the_bounty_hunters_leave_the_palace_capture_han_and_take_him_there() {
        // FUN_00546fa0 creates the Bounty Hunters (SPECFCSD 0x3c000009) in
        // Jabba's Palace and sends them on mission 0x65000083 after Han; the
        // trip from the abode takes 1 day (FUN_00555b30). The hit
        // FUN_00575de0 makes Han roll RLEVADTB (0 here: captured); captured,
        // he loses CanEscape and gets CapturedByBountyHunters, and
        // FUN_00543040 autoroutes him to the palace, 1 day away.
        let (mut world, home, han, palace) = bounty_world(0);
        let mut state = MissionState::new();
        let hunt = crate::jabba::JabbaStep::BountyHunt {
            han,
            palace,
            at: home,
            tick: 10,
        };
        crate::jabba::perform(&hunt, &mut state, &mut world).expect("dispatched");
        let unit = world.abodes[palace].special_forces[0];
        assert_eq!(state.missions()[0].kind, MissionKind::Bounty);
        assert_eq!(
            state.missions()[0].team,
            [MissionMember::SpecialForce(unit)]
        );

        let mut attacked = None;
        for day in 10..=20 {
            let advance = step(&mut state, &world, day, &[]);
            apply_advance_effects(&mut world, &advance.effects);
            if let Some(transit) = state.en_route.first() {
                assert_eq!(transit.arrival, day + 1, "the trip from the abode");
            }
            for result in &advance.results {
                apply_advance_effects(&mut world, &result.effects);
                if result.effects.contains(&MissionEffect::BountyAttack {
                    character: han,
                    captured: true,
                }) {
                    attacked = Some(day);
                }
            }
            if attacked.is_some() {
                break;
            }
        }
        assert!(attacked.is_some(), "the hunters reached Han");
        assert!(world.abodes[palace].special_forces.is_empty());
        let c = &world.characters[han];
        assert!(c.is_captive && c.escape_barred && c.bounty_captured);

        let mut jabba = crate::jabba::JabbaState {
            armed: true,
            bounty_hunters_active: false,
            bounty_due: None,
        };
        let day = attacked.unwrap() + 1;
        let steps = crate::jabba::JabbaSystem::advance(
            &mut jabba,
            &world,
            &state,
            &[TickEvent { tick: day }],
            &[],
        );
        assert_eq!(
            steps,
            [crate::jabba::JabbaStep::ToPalace {
                captive: han,
                from: home,
                palace,
                tick: day,
            }]
        );
        crate::jabba::perform(&steps[0], &mut state, &mut world);
        assert!(state.is_autorouting(han));
        // FUN_00555b30: the trip to the abode takes exactly 1 day.
        assert_eq!(state.escapees[0].arrival, day + 1);
        assert_eq!(world.characters[han].current_system, None);
        let arrived = MissionSystem::advance_escapes(&mut state, &world, day + 1, &[]);
        assert_eq!(
            arrived,
            [MissionEffect::MemberEnteredAbode {
                member: MissionMember::Character(han),
                abode: palace,
            }]
        );
        apply_advance_effects(&mut world, &arrived);
        assert_eq!(world.characters[han].current_abode, Some(palace));
        assert!(world.characters[han].is_captive);
    }

    #[test]
    fn luke_trains_on_dagobah_for_100_days_and_autoroutes_home() {
        // FUN_0054a860 creates mission 0x71000043 for Luke to the Dagobah
        // abode; the trip takes 1 day (FUN_00555b30); phase 8 records the
        // first training day and waits 100 days; phase 10 succeeds
        // (FUN_00575060 100, FUN_00572470 3); phase 0xb after phase 8 raises
        // Luke's Force by GNPRTB 6174 (60) percent and sets DagobahCompleted
        // (FUN_005751d0); then slot +0xb0 (FUN_004ef260) autoroutes him out
        // of the side-3 abode to his side's nearest system, 1 day away.
        let (mut world, home, _, luke, dagobah) = dagobah_world();
        let mut state = MissionState::new();
        let pulse = crate::dagobah::LukeDagobah {
            luke,
            from: home,
            abode: dagobah,
            tick: 10,
        };
        let id = crate::dagobah::dispatch(&mut state, &mut world, &pulse).expect("dispatched");
        assert!(world.characters[luke].on_mission);
        let mut entered = None;
        let mut completed = None;
        for day in 10..=130 {
            let advance = step(&mut state, &world, day, &[]);
            apply_advance_effects(&mut world, &advance.effects);
            // FUN_00555b30: the trip to the abode takes exactly 1 day.
            if let Some(transit) = state.en_route.first() {
                assert_eq!(transit.arrival, day + 1);
                assert_eq!(transit.abode, Some(dagobah));
            }
            // And the autorouting trip out of it.
            if completed.is_none() {
                if let Some(escape) = state.escapees.first() {
                    assert_eq!(escape.arrival, day + 1);
                    assert_eq!(escape.to, home);
                }
            }
            if entered.is_none() && world.characters[luke].current_abode == Some(dagobah) {
                entered = Some(day);
                assert_eq!(world.characters[luke].current_system, None);
                assert_eq!(state.missions()[0].first_training_day, Some(day));
            }
            if advance.effects.iter().any(|effect| {
                matches!(effect,
                MissionEffect::DagobahCompleted { character, force_gain: 30 } if *character == luke)
            }) {
                completed = Some(day);
            }
            let escapes = MissionSystem::advance_escapes(&mut state, &world, day, &[]);
            apply_advance_effects(&mut world, &escapes);
        }
        let entered = entered.expect("Luke reached Dagobah");
        assert_eq!(entered, 11);
        assert_eq!(completed, Some(entered + 100));
        let c = &world.characters[luke];
        assert!(c.dagobah_completed);
        assert_eq!(c.force_level(), 80);
        assert_eq!(c.current_abode, None);
        assert_eq!(c.current_system, Some(home));
        assert!(!c.on_mission);
        assert!(state.missions().iter().all(|mission| mission.id != id));
    }

    #[test]
    fn only_a_member_of_another_side_than_the_abode_s_autoroutes_out() {
        // FUN_004ef260: Autorouting only when the container's side differs
        // from the character's, and never for a destroyed one.
        let run = |edit: fn(&mut GameWorld, CharacterKey, crate::ids::AbodeKey)| {
            let (mut world, home, _, luke, dagobah) = dagobah_world();
            let mut mission = ActiveMission::timed(
                0,
                MissionKind::Dagobah,
                MissionFaction::Alliance,
                vec![MissionMember::Character(luke)],
                home,
                1,
            );
            mission.target_abode = Some(dagobah);
            mission.origin = Some(home);
            enter_abode(&mut world, MissionMember::Character(luke), dagobah);
            edit(&mut world, luke, dagobah);
            let mut state = MissionState::new();
            state.push_timed(mission);
            let mut leaving = 0;
            for day in 1..=3 {
                let advance = step(&mut state, &world, day, &[]);
                leaving += state.escapees.len();
                apply_advance_effects(&mut world, &advance.effects);
            }
            leaving
        };
        assert!(run(|_, _, _| {}) > 0);
        assert_eq!(
            run(|world, _, abode| world.abodes[abode].side = crate::dat::Faction::Alliance),
            0
        );
        assert_eq!(
            run(|world, luke, _| world.characters[luke].is_killed = true),
            0
        );
        assert_eq!(
            run(|world, luke, _| world.characters[luke].current_abode = None),
            0
        );
    }

    #[test]
    fn only_dagobah_ended_after_phase_8_completes_dagobah() {
        // FUN_005751d0 belongs to the Dagobah class and acts in phase 0xb
        // only after phase 8 (FUN_00520ac0 >= 8).
        let (world, home, _, luke, _) = dagobah_world();
        let completes = |kind, phase: u8| {
            let mut mission = ActiveMission::timed(
                0,
                kind,
                MissionFaction::Alliance,
                vec![MissionMember::Character(luke)],
                home,
                1,
            );
            mission.first_training_day = Some(0);
            mission.phase = phase;
            mission.end_code = 2;
            let mut state = MissionState::new();
            state.push_timed(mission);
            step(&mut state, &world, 1, &[])
                .effects
                .iter()
                .any(|effect| matches!(effect, MissionEffect::DagobahCompleted { .. }))
        };
        assert!(completes(MissionKind::Dagobah, PHASE_TIMER));
        assert!(!completes(MissionKind::Dagobah, 5));
        assert!(!completes(MissionKind::Diplomacy, PHASE_TIMER));
    }

    #[test]
    fn a_killed_character_or_an_unknown_abode_takes_no_one_in() {
        let (mut world, _, _, luke, dagobah) = dagobah_world();
        let member = MissionMember::Character(luke);
        world.characters[luke].is_killed = true;
        enter_abode(&mut world, member, dagobah);
        assert_eq!(world.characters[luke].current_abode, None);
        world.characters[luke].is_killed = false;
        let gone = world.abodes.remove(dagobah).map(|_| dagobah).unwrap();
        enter_abode(&mut world, member, gone);
        assert_eq!(world.characters[luke].current_abode, None);
        assert!(world.characters[luke].current_system.is_some());
        // A character entering from a fleet leaves its roster.
        let (mut world, home, _, luke, dagobah) = dagobah_world();
        let other = agent_at(&mut world, home, true);
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: home,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![luke, other],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.characters[luke].current_fleet = Some(fleet);
        enter_abode(&mut world, MissionMember::Character(luke), dagobah);
        assert_eq!(world.fleets[fleet].characters, vec![other]);
        assert_eq!(world.characters[luke].current_fleet, None);
        assert_eq!(world.characters[luke].current_abode, Some(dagobah));
    }

    #[test]
    fn an_unfinished_dagobah_changes_no_force() {
        // FUN_005751d0 needs phase 8 or later and a first training day.
        let (world, _, _, _, _) = dagobah_world();
        assert_eq!(crate::dagobah::training_end(&world, None, true, 50), []);
        // Without success: Force x ((today - first) / GNPRTB 6175 (2)) / 100:
        // 50 x (41 / 2) / 100 = 10.
        assert_eq!(
            crate::dagobah::training_end(&world, Some(10), false, 51),
            vec![MissionEffect::DagobahCompleted {
                character: world.characters.keys().next().unwrap(),
                force_gain: 10,
            }]
        );
    }

    /// The shipped MISSNSD Jedi Training record `0x58000042`: an own-side
    /// system, timer (60, 30), no repeat, characters of both sides.
    fn jedi_training_record() -> crate::world::MissionRecord {
        crate::world::MissionRecord {
            dat_id: crate::ids::DatId::new(0x5800_0042),
            timer_min_days: 60,
            timer_spread_days: 30,
            repeats: false,
            hidden: false,
            detection_phases: true,
            can_resign: true,
            leave_rule: 0,
            rules: crate::world::MissionTargetRules {
                container_loss_ends: true,
                needs_populated_container: false,
                target_loss_ends: true,
                own_side_target: true,
                other_side_target: false,
                opponent_target: false,
                revolting_target: true,
                calm_target: true,
                prisoner_target: false,
                free_target: false,
            },
            members: crate::world::MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0,
                character_mask: 0x1_0000,
            },
        }
    }

    /// An Alliance teacher (trains Jedi, Force `teacher`) and a pupil
    /// (Force `pupil`) at an Alliance system with the Jedi Training record.
    fn jedi_world(teacher: u16, pupil: u16) -> (GameWorld, SystemKey, CharacterKey, CharacterKey) {
        let (mut world, here, envoy) = dialog_world(vec![jedi_training_record()]);
        let master = envoy.character().unwrap();
        world.characters[master].is_jedi_trainer = true;
        world.characters[master].force_base = teacher;
        let student = agent_at(&mut world, here, true);
        world.characters[student].force_base = pupil;
        (world, here, master, student)
    }

    #[test]
    fn jedi_training_needs_a_teacher_at_force_100_and_weaker_pupils() {
        // FUN_00570690 / FUN_00570940: a member who trains Jedi at Force
        // GNPRTB 2581 (100) or more (FUN_00570e40, FUN_004ed310), every
        // other member with some Force (0x40/5) below the teacher's
        // (0x40/0xd), and two members or more (0x40/0x94).
        let list = |world: &GameWorld, team: &[CharacterKey], here| {
            let team: Vec<MissionMember> =
                team.iter().map(|&c| MissionMember::Character(c)).collect();
            available_kinds(
                world,
                &UprisingState::default(),
                &ResearchState::default(),
                MissionFaction::Alliance,
                &team,
                &[],
                here,
            )
        };
        let (world, here, master, student) = jedi_world(100, 20);
        assert_eq!(
            list(&world, &[master, student], here),
            vec![MissionKind::JediTraining]
        );
        // The teacher may come second.
        assert_eq!(
            list(&world, &[student, master], here),
            vec![MissionKind::JediTraining]
        );
        assert_eq!(list(&world, &[master], here), []);
        let (world, here, master, student) = jedi_world(99, 20);
        assert_eq!(list(&world, &[master, student], here), []);
        let (mut world, here, master, student) = jedi_world(100, 20);
        world.characters[master].is_jedi_trainer = false;
        assert_eq!(list(&world, &[master, student], here), []);
        let (world, here, master, student) = jedi_world(100, 0);
        assert_eq!(list(&world, &[master, student], here), []);
        let (world, here, master, student) = jedi_world(100, 100);
        assert_eq!(list(&world, &[master, student], here), []);
        let (world, here, master, student) = jedi_world(100, 99);
        assert_eq!(
            list(&world, &[master, student], here),
            vec![MissionKind::JediTraining]
        );
        // A running mission whose teacher left ends with -1 (0xff).
        let (world, here, _, student) = jedi_world(100, 20);
        let mission = ActiveMission::new(
            0,
            MissionKind::JediTraining,
            MissionFaction::Alliance,
            vec![MissionMember::Character(student)],
            here,
            0,
        );
        assert_eq!(
            running_end_code(
                &mission,
                &world,
                &UprisingState::default(),
                &ResearchState::default(),
                &[]
            ),
            JEDI_TRAINING_REFUSED
        );
    }

    #[test]
    fn entering_phase_two_chooses_the_first_member_who_may_teach() {
        // FUN_00571410 in phase 2: FUN_00570e40 walks the members in order
        // and FUN_005703b0 stores the first who may teach at +0xa8.
        let (mut world, here, master, student) = jedi_world(120, 20);
        let second = agent_at(&mut world, here, true);
        world.characters[second].is_jedi_trainer = true;
        // Stronger than the later trainer, which then counts as a pupil
        // below the teacher (FUN_00570940's 0x40/0xd).
        world.characters[second].force_base = 130;
        let mut mission = ActiveMission::new(
            0,
            MissionKind::JediTraining,
            MissionFaction::Alliance,
            vec![
                MissionMember::Character(student),
                MissionMember::Character(second),
                MissionMember::Character(master),
            ],
            here,
            0,
        );
        mission.phase = 1;
        let mut state = MissionState::new();
        state.push_timed(mission);
        step(&mut state, &world, 1, &[]);
        let mission = &state.missions()[0];
        assert!(mission.phase >= 2);
        assert_eq!(mission.teacher, Some(second));
    }

    #[test]
    fn a_jedi_training_pupil_gains_up_to_half_the_force_gap() {
        // FUN_005710b0: chance = teacher - pupil (120 - 20 = 100, a sure
        // hit); FUN_005712b0 -> FUN_0055ca20: gain rand(0..=100 x GNPRTB
        // 6173 (50) / 100) into ForceTraining +0x90, result 3 when not 0.
        // The teacher's own chance is 0.
        let (world, here, master, student) = jedi_world(120, 20);
        let mut gains = std::collections::BTreeSet::new();
        for seed in 0..200_u32 {
            let mut mission = ActiveMission::timed(
                0,
                MissionKind::JediTraining,
                MissionFaction::Alliance,
                vec![
                    MissionMember::Character(master),
                    MissionMember::Character(student),
                ],
                here,
                1,
            );
            mission.teacher = Some(master);
            let mut state = MissionState::new();
            state.push_timed(mission);
            let roll = f64::from(seed) / 200.0;
            let result = step(&mut state, &world, 1, &[roll; 8]).results.remove(0);
            let gained: Vec<_> = result
                .effects
                .iter()
                .filter_map(|effect| match effect {
                    MissionEffect::ForceTrainingGained { character, amount } => {
                        Some((*character, *amount))
                    }
                    _ => None,
                })
                .collect();
            match gained.as_slice() {
                [] => assert_eq!(result.outcome, MissionOutcome::Failure),
                [(character, amount)] => {
                    assert_eq!(*character, student);
                    assert_eq!(result.outcome, MissionOutcome::Success);
                    gains.insert(*amount);
                }
                other => panic!("one pupil, {other:?}"),
            }
        }
        assert!(
            gains.iter().all(|gain| (1..=50).contains(gain)),
            "{gains:?}"
        );
        assert!(gains.contains(&50) && gains.len() > 20, "{gains:?}");
        assert_eq!(
            jedi_training_chance(&world, Some(master), MissionMember::Character(master)),
            0
        );
        assert_eq!(
            jedi_training_chance(&world, None, MissionMember::Character(student)),
            0
        );
        assert_eq!(
            jedi_training_chance(&world, Some(master), MissionMember::Character(student)),
            100
        );
    }

    fn dialog_world(
        records: Vec<crate::world::MissionRecord>,
    ) -> (GameWorld, SystemKey, MissionMember) {
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        world.systems[here].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Alliance);
        world.mission_records = records;
        let envoy = MissionMember::Character(agent_at(&mut world, here, true));
        (world, here, envoy)
    }

    #[test]
    fn the_dialog_lists_the_records_the_team_may_undertake_in_missnsd_order() {
        // FUN_005422f0 walks the records in table order. Rescue names a
        // character, so a system target leaves it out.
        let (world, here, envoy) = dialog_world(vec![
            recruitment_record(),
            rescue_record(),
            diplomacy_record(),
        ]);

        let kinds = available_kinds(
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            MissionFaction::Alliance,
            &[envoy],
            &[],
            here,
        );

        assert_eq!(
            kinds,
            vec![MissionKind::Recruitment, MissionKind::Diplomacy]
        );
    }

    #[test]
    fn a_dialog_dropped_on_a_held_prisoner_lists_rescue_alone() {
        // The release asks the window's +0x68, the object under the point;
        // FUN_00586e20 checks Rescue's prisoner and opponent columns. The
        // prisoner reads as the Empire's (FUN_004eec60). System kinds
        // take no character target.
        let (mut world, here, envoy) = dialog_world(vec![
            recruitment_record(),
            rescue_record(),
            diplomacy_record(),
        ]);
        let prisoner = agent_at(&mut world, here, true);
        world.characters[prisoner].is_captive = true;
        world.characters[prisoner].captured_by = Some(crate::dat::Faction::Empire);
        let target = MissionTargetPick {
            system: here,
            character: Some(prisoner),
            object: None,
        };
        let uprisings = UprisingState::default();
        let list = |target| {
            available_kinds(
                &world,
                &uprisings,
                &ResearchState::default(),
                MissionFaction::Alliance,
                &[envoy],
                &[],
                target,
            )
        };

        assert_eq!(list(target), vec![MissionKind::Rescue]);
        // A free friend is no prisoner of the other side.
        world.characters[prisoner].is_captive = false;
        world.characters[prisoner].captured_by = None;
        let list = |target| {
            available_kinds(
                &world,
                &uprisings,
                &ResearchState::default(),
                MissionFaction::Alliance,
                &[envoy],
                &[],
                target,
            )
        };
        assert!(list(target).is_empty());
    }

    #[test]
    fn a_dialog_dropped_on_an_enemy_object_lists_the_object_kinds() {
        // FUN_00593500, the base rules, for Sabotage's object target.
        let (mut world, here, envoy) = dialog_world(vec![diplomacy_record(), sabotage_record()]);
        world.systems[here].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: crate::ids::DatId::new(0x1400_0000),
            is_alliance: false,
            regiment_strength: 1,
        });
        world.systems[here].ground_units.push(troop);
        let target = MissionTargetPick {
            system: here,
            character: None,
            object: Some(MissionTarget::Troop(troop)),
        };

        let kinds = available_kinds(
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            MissionFaction::Alliance,
            &[envoy],
            &[],
            target,
        );

        assert_eq!(kinds, vec![MissionKind::Sabotage]);
    }

    #[test]
    fn a_dialog_dropped_on_the_death_star_lists_ds_sabotage_and_on_a_yard_sabotage() {
        // FUN_0054c590 builds the mission through its class factory, whose
        // creation checks split the two (FUN_0056a110, FUN_005744c0).
        let ds_record = crate::world::MissionRecord {
            dat_id: MissionKind::DeathStarSabotage.record_id().unwrap(),
            ..sabotage_record()
        };
        let (mut world, here, envoy) = dialog_world(vec![sabotage_record(), ds_record]);
        world.systems[here].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        let yard = MissionTarget::ManufacturingFacility(enemy_yard(&mut world, here));
        let death_star = MissionTarget::DeathStar(death_star_fleet(&mut world, here));
        let uprisings = UprisingState::default();
        let list = |object| {
            available_kinds(
                &world,
                &uprisings,
                &ResearchState::default(),
                MissionFaction::Alliance,
                &[envoy],
                &[],
                MissionTargetPick {
                    system: here,
                    character: None,
                    object: Some(object),
                },
            )
        };

        assert_eq!(list(death_star), vec![MissionKind::DeathStarSabotage]);
        assert_eq!(list(yard), vec![MissionKind::Sabotage]);
    }

    #[test]
    fn a_dialog_dropped_on_an_enemy_ship_or_squadron_lists_sabotage_which_destroys_one() {
        // FUN_0056a110 refuses only the Death Star (0x18..0x1c) and
        // characters, so a capital ship (0x14..0x18) or a squadron
        // (0x1c..0x20) is a Sabotage target; the success destroys it
        // (slot +0xac(6), FUN_005746e0).
        let ds_record = crate::world::MissionRecord {
            dat_id: MissionKind::DeathStarSabotage.record_id().unwrap(),
            ..sabotage_record()
        };
        let (mut world, here, envoy) = dialog_world(vec![sabotage_record(), ds_record]);
        world.systems[here].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        let destroyer = world
            .capital_ship_classes
            .insert(crate::world::CapitalShipClass::default());
        let tie = world
            .fighter_classes
            .insert(crate::world::FighterClass::default());
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: here,
            capital_ships: vec![
                crate::world::ShipInstance::new(destroyer, 100, false),
                crate::world::ShipInstance::new(destroyer, 100, false),
            ],
            fighters: vec![crate::world::FighterEntry {
                class: tie,
                count: 1,
            }],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[here].fleets.push(fleet);
        let ship = MissionTarget::CapitalShip {
            fleet,
            class: destroyer,
        };
        let squadron = MissionTarget::Fighter { fleet, class: tie };
        let uprisings = UprisingState::default();
        let list = |world: &GameWorld, object| {
            available_kinds(
                world,
                &uprisings,
                &ResearchState::default(),
                MissionFaction::Alliance,
                &[envoy],
                &[],
                MissionTargetPick {
                    system: here,
                    character: None,
                    object: Some(object),
                },
            )
        };

        assert_eq!(list(&world, ship), vec![MissionKind::Sabotage]);
        assert_eq!(list(&world, squadron), vec![MissionKind::Sabotage]);
        destroy_target(&mut world, ship);
        assert_eq!(world.fleets[fleet].capital_ships.len(), 1);
        destroy_target(&mut world, squadron);
        assert!(world.fleets[fleet].fighters.is_empty());
        // The last squadron gone, the target is missing (rule 1).
        assert!(list(&world, squadron).is_empty());
    }

    #[test]
    fn a_ship_or_squadron_target_is_a_living_hull_or_manned_squadron_of_its_class() {
        // The target names its class aboard a fleet (port:); a dead hull or
        // an empty squadron entry of that class does not stand for it, and
        // the destroy takes that class's, not another's.
        use crate::world::{CapitalShipClass, FighterClass, FighterEntry, ShipInstance};
        let mut world = minimal_world();
        let here = system_at(&mut world, 0);
        let (wanted, other) = (
            world
                .capital_ship_classes
                .insert(CapitalShipClass::default()),
            world
                .capital_ship_classes
                .insert(CapitalShipClass::default()),
        );
        let (tie, bomber) = (
            world.fighter_classes.insert(FighterClass::default()),
            world.fighter_classes.insert(FighterClass::default()),
        );
        let mut dead = ShipInstance::new(wanted, 100, false);
        dead.alive = false;
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: here,
            capital_ships: vec![ShipInstance::new(other, 100, false), dead],
            fighters: vec![
                FighterEntry {
                    class: bomber,
                    count: 1,
                },
                FighterEntry {
                    class: tie,
                    count: 0,
                },
            ],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        let ship = MissionTarget::CapitalShip {
            fleet,
            class: wanted,
        };
        let squadron = MissionTarget::Fighter { fleet, class: tie };
        assert!(ship.state(&world).is_none());
        assert!(squadron.state(&world).is_none());

        world.fleets[fleet].capital_ships[1].alive = true;
        world.fleets[fleet].fighters.push(FighterEntry {
            class: tie,
            count: 1,
        });
        assert!(ship.state(&world).is_some());
        assert!(squadron.state(&world).is_some());
        destroy_target(&mut world, ship);
        destroy_target(&mut world, squadron);
        let value = &world.fleets[fleet];
        assert_eq!(
            value
                .capital_ships
                .iter()
                .map(|s| s.class)
                .collect::<Vec<_>>(),
            vec![other]
        );
        assert_eq!(
            value
                .fighters
                .iter()
                .map(|e| (e.class, e.count))
                .collect::<Vec<_>>(),
            vec![(bomber, 1)]
        );
    }

    #[test]
    fn the_dialog_leaves_out_a_record_whose_validator_refuses_the_target() {
        // FUN_0054c590 runs the class validator: Recruitment takes no target
        // of a third side (column 16), Diplomacy does.
        let (mut world, here, envoy) = dialog_world(vec![recruitment_record(), diplomacy_record()]);
        world.systems[here].control = crate::world::ControlKind::Uncontrolled;

        let kinds = available_kinds(
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            MissionFaction::Alliance,
            &[envoy],
            &[],
            here,
        );

        assert_eq!(kinds, vec![MissionKind::Diplomacy]);
    }

    #[test]
    fn the_dialog_leaves_out_a_record_whose_member_rules_refuse_a_decoy() {
        // FUN_005830a0 masks the decoys too; Diplomacy takes no special force.
        let (mut world, here, envoy) = dialog_world(vec![diplomacy_record()]);
        let unit = unit_of_mask(&mut world, 0x2, true);

        let with_decoy = available_kinds(
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            MissionFaction::Alliance,
            &[envoy],
            &[unit],
            here,
        );
        let alone = available_kinds(
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            MissionFaction::Alliance,
            &[envoy],
            &[],
            here,
        );

        assert!(with_decoy.is_empty());
        assert_eq!(alone, vec![MissionKind::Diplomacy]);
    }

    #[test]
    fn the_dialog_never_lists_a_hidden_record() {
        // FUN_005422f0 skips records with +0x5c set.
        let hidden = crate::world::MissionRecord {
            hidden: true,
            ..diplomacy_record()
        };
        let (world, here, envoy) = dialog_world(vec![hidden]);

        let kinds = available_kinds(
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            MissionFaction::Alliance,
            &[envoy],
            &[],
            here,
        );

        assert!(kinds.is_empty());
    }
}
