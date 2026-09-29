//! Mission system: all 11 mission types with MSTB probability lookup.
//!
//! Missions are assigned to characters and execute over multiple game-days.
//! When a mission completes, success is determined either by a quadratic
//! probability formula (fallback) or by a piecewise-linear MSTB table lookup
//! when `world.mission_tables` is populated by rebellion-data.
//!
//! # Design
//!
//! - `MissionState` holds every active mission and the members travelling
//!   to a mission target.
//! - `MissionSystem::advance(state, world, uprisings, tick_events, rolls)`
//!   returns a `MissionAdvance`: member moves and releases, phase 10
//!   results, and the missions that ended.
//! - The caller provides pre-generated random rolls so rebellion-core stays
//!   dependency-free and fully deterministic in tests.
//!
//! # Probability
//!
//! Two paths, in priority order:
//! 1. **MSTB lookup**: if `world.mission_tables` contains an entry for this mission kind,
//!    use `MstbTable::lookup(skill_score)` (piecewise-linear over `IntTableEntry` thresholds).
//! 2. **Quadratic fallback**: `clamp(a·score² + b·score + c, min%, max%)` — same as
//!    rebellion2's Mission.cs coefficients. Used when MSTB tables are not yet loaded.
//!
//! Combined: `total_prob = agent_prob · (1 − foil_prob)`.
//!
//! # Mission Types
//!
//! Nine types ported from REBEXE.EXE's 13-case dispatch (`FUN_0050d5a0`):
//! Diplomacy, Recruitment, Sabotage, Assassination, Espionage, Rescue, Abduction,
//! `InciteUprising`, Autoscrap.
//!
//! # Lifecycle
//!
//! A mission steps through phases `0..=0xb` (`FUN_005227d0`,
//! `ghidra/notes/mission-lifecycle.md`). It waits in phase 4 for its
//! members' transit and in phase 8 for its MISSNSD timer, rolls in phase 10,
//! repeats 10 -> 8 when its record says so, and ends in phase `0xb` once the
//! validator (`FUN_00522480`) or the last step sets an end code.
//!
//! After each phase change the detection run (`mission_detection`,
//! `FUN_00547f60`) lets the decoys draw off defenders, the defenders try to
//! detect the team, and phase 9 looks for a traitor; a detected team is
//! exposed, and its members evade and resign or are captured.
//!
//! The phase 10 roll and the probability paths above are interim until
//! F-019 phase 4 ports the per-member roll.

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::ids::{CharacterKey, SpecialForceKey, SystemKey};
use crate::tick::TickEvent;
use crate::world::{Character, GameWorld, MstbTable, Skill};

// ---------------------------------------------------------------------------
// MissionKind
// ---------------------------------------------------------------------------

/// All nine mission types recognised by the REBEXE.EXE dispatch (`FUN_0050d5a0`).
///
/// Type codes match the original binary's 13-case switch where applicable.
/// Coefficients are ported from rebellion2's Mission.cs; they serve as
/// fallback when `GameWorld::mission_tables` is not yet populated.
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
}

impl MissionKind {
    /// Whether this mission kind is covert and subject to enemy counter-intelligence.
    ///
    /// Covert missions (espionage, sabotage, assassination, abduction, rescue,
    /// Death Star sabotage) can be foiled by enemy operatives at the target system.
    /// Non-covert missions (diplomacy, recruitment, uprising) are visible operations
    /// that succeed or fail on their own merit without foil risk.
    #[must_use]
    pub fn is_covert(self) -> bool {
        matches!(
            self,
            MissionKind::Sabotage
                | MissionKind::Assassination
                | MissionKind::Espionage
                | MissionKind::Rescue
                | MissionKind::Abduction
                | MissionKind::DeathStarSabotage
        )
    }

    /// DAT file stem for this kind's *MSTB probability table.
    ///
    /// This is the key used to look up `world.mission_tables`. `None` for
    /// `Autoscrap` (no probability table — it always succeeds).
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
            MissionKind::Autoscrap => None,
        }
    }

    /// Quadratic success-probability fallback coefficients: (a, b, c).
    ///
    /// Formula: `prob% = a·score² + b·score + c`, clamped to [min%, max%].
    ///
    /// Diplomacy and Recruitment are from rebellion2 Mission.cs.
    /// Other types use placeholder coefficients fit to approximate MSTB curves.
    /// Once `world.mission_tables` is populated, these are never used.
    #[must_use]
    pub fn coefficients(self) -> (f64, f64, f64) {
        match self {
            MissionKind::Diplomacy => (0.005_558, 0.7656, 20.15),
            MissionKind::Recruitment => (-0.001_748, 0.8657, 11.923),
            MissionKind::Sabotage => (-0.002, 0.75, 15.0),
            MissionKind::Assassination => (-0.003, 0.80, 10.0),
            MissionKind::Espionage => (-0.002, 0.78, 12.0),
            MissionKind::Rescue => (-0.002, 0.72, 10.0),
            MissionKind::Abduction => (-0.002, 0.70, 8.0),
            MissionKind::InciteUprising => (-0.003, 0.65, 18.0),
            MissionKind::SubdueUprising => (-0.002, 0.70, 20.0),
            MissionKind::DeathStarSabotage => (-0.003, 0.60, 10.0),
            MissionKind::Autoscrap => (0.0, 0.0, 100.0), // always succeeds
        }
    }

    /// Extract the relevant skill score from a character for this mission type.
    #[must_use]
    pub fn skill_score(self, character: &Character) -> u32 {
        let pair = match self {
            MissionKind::Recruitment => character.leadership,
            MissionKind::Sabotage
            | MissionKind::Espionage
            | MissionKind::Abduction
            | MissionKind::DeathStarSabotage => character.espionage,
            MissionKind::Assassination | MissionKind::Rescue => character.combat,
            MissionKind::Diplomacy | MissionKind::InciteUprising | MissionKind::SubdueUprising => {
                character.diplomacy
            }
            // Autoscrap has no character; callers guard against passing None.
            MissionKind::Autoscrap => return 100,
        };
        // Expected value: base + half variance (variance resolved at scenario start).
        pair.base + pair.variance / 2
    }

    /// Compute the composite input value for MSTB table lookup.
    ///
    /// The original game (per Ghidra RE + `TheArchitect2018` wiki) computes a
    /// composite input from character skill + game-state context before looking
    /// up the probability table. This replaces our previous approach of passing
    /// raw `skill_score` directly to `MstbTable::lookup()`.
    ///
    /// Source functions from REBEXE.EXE:
    /// - Diplomacy:     `sub_55ae50` → `(enemy_pop - our_pop) + diplomacy_rating`
    /// - Recruitment:   `sub_55aed0` → `leadership - target_resistance`
    /// - Espionage:     `sub_55ae90` → `espionage_skill` (direct lookup)
    /// - Subdue:        `sub_55af50` → `(enemy_pop - our_pop) + diplomacy_rating`
    /// - DS Sabotage:   `sub_55b0a0` → `(espionage + combat) / 2`
    /// - Escape:        `sub_55cfb0` → `((p3 + p2) - p4) - p5` (see `check_escapes`)
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    #[expect(
        clippy::manual_midpoint,
        reason = "Preserve the existing signed sum and division semantics, including overflow behavior."
    )]
    pub fn compute_table_input(
        self,
        character: &Character,
        system: Option<&crate::world::System>,
        faction: MissionFaction,
        target_character: Option<&Character>,
    ) -> i32 {
        let skill = self.skill_score(character).cast_signed();

        match self {
            // sub_55ae50: input = (enemy_popularity - our_popularity) + diplomacy_rating
            // Original: INCTMS_TABLE[(diplomacy - pop_support) - espionage_rating]
            MissionKind::InciteUprising => {
                if let Some(sys) = system {
                    let (our_pop, enemy_pop) = match faction {
                        MissionFaction::Alliance => {
                            (sys.popularity_alliance, sys.popularity_empire)
                        }
                        MissionFaction::Empire => (sys.popularity_empire, sys.popularity_alliance),
                    };
                    let pop_delta = ((enemy_pop - our_pop) * 100.0) as i32;
                    let counter_intel = (sys.espionage_rating * 100.0) as i32;
                    pop_delta + skill - counter_intel
                } else {
                    skill
                }
            }

            // sub_55af50: same formula as diplomacy
            MissionKind::Diplomacy | MissionKind::SubdueUprising => {
                if let Some(sys) = system {
                    let (our_pop, enemy_pop) = match faction {
                        MissionFaction::Alliance => {
                            (sys.popularity_alliance, sys.popularity_empire)
                        }
                        MissionFaction::Empire => (sys.popularity_empire, sys.popularity_alliance),
                    };
                    let pop_delta = ((enemy_pop - our_pop) * 100.0) as i32;
                    pop_delta + skill
                } else {
                    skill
                }
            }

            // FIX #5: Recruitment uses target character's loyalty as resistance.
            // Original: RCRTMS_TABLE[leadership - target_resistance]
            MissionKind::Recruitment => {
                let resistance = if let Some(target) = target_character {
                    (target.loyalty.base + target.loyalty.variance / 2).cast_signed()
                } else if let Some(sys) = system {
                    let our_pop = match faction {
                        MissionFaction::Alliance => sys.popularity_alliance,
                        MissionFaction::Empire => sys.popularity_empire,
                    };
                    (50.0 - our_pop * 100.0).max(0.0) as i32
                } else {
                    0
                };
                skill - resistance
            }

            // sub_55b0a0 / SBTGMS_TABLE: both sabotage kinds use (espionage + combat) / 2
            MissionKind::DeathStarSabotage | MissionKind::Sabotage => {
                let espionage =
                    (character.espionage.base + character.espionage.variance / 2).cast_signed();
                let combat = (character.combat.base + character.combat.variance / 2).cast_signed();
                (espionage + combat) / 2
            }

            // FIX #4: Assassination subtracts target defense (combat rating).
            // Original: ASSNMS_TABLE[combat - target_defense]
            MissionKind::Assassination => {
                let target_defense = target_character
                    .map_or(0, |t| (t.combat.base + t.combat.variance / 2).cast_signed());
                skill - target_defense
            }

            // FIX #3: Abduction subtracts target defense (combat rating).
            // Original: ABDCMS_TABLE[espionage - target_defense]
            MissionKind::Abduction => {
                let target_defense = target_character
                    .map_or(0, |t| (t.combat.base + t.combat.variance / 2).cast_signed());
                skill - target_defense
            }

            // sub_55ae90: direct skill lookup (no composite)
            MissionKind::Espionage | MissionKind::Rescue => skill,

            MissionKind::Autoscrap => 100,
        }
    }

    /// Minimum success probability (percent, 1–100).
    #[must_use]
    pub fn min_success_prob(self) -> f64 {
        1.0
    }

    /// Maximum success probability (percent, 1–100).
    #[must_use]
    pub fn max_success_prob(self) -> f64 {
        100.0
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

    /// The first team character. The resolver rolls for this one member
    /// until the per-member roll lands (port: interim, F-019 phase 4).
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
}

/// What the member checks read about one member.
struct MemberState {
    is_alliance: bool,
    busy: bool,
    prisoner: bool,
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
                busy: c.on_mission || c.on_mandatory_mission,
                prisoner: c.is_captive,
                location,
            })
        }
        MissionMember::SpecialForce(key) => {
            let unit = world.special_forces.get(key)?;
            let location = world
                .systems
                .iter()
                .find(|(_, system)| system.special_forces.contains(&key))
                .map_or(MemberLocation::Nowhere, |(system, _)| {
                    MemberLocation::System(system)
                });
            Some(MemberState {
                is_alliance: unit.is_alliance,
                busy: unit.on_mission,
                prisoner: false,
                location,
            })
        }
    }
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
        MissionMember::Character(key) => world.characters.get(key).map(|c| {
            let pair = c.skill(skill);
            // port: seeding rolls every character's variance into its base
            // (FUN_00535e40); half of an unrolled variance matches skill_score.
            pair.base + pair.variance / 2
        }),
        MissionMember::SpecialForce(key) => world
            .special_forces
            .get(key)
            .map(|unit| unit.skills[skill as usize]),
    }
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
            character.current_fleet = None;
            character.current_system = to;
        }
        MissionMember::SpecialForce(key) => {
            if !world.special_forces.contains_key(key) {
                return;
            }
            for (_, system) in &mut world.systems {
                system.special_forces.retain(|&unit| unit != key);
            }
            if let Some(system) = to.and_then(|to| world.systems.get_mut(to)) {
                system.special_forces.push(key);
            }
        }
    }
}

/// Apply [`MissionAdvance::effects`] in order: member moves and releases,
/// and the detection's captures, lost special forces, and injuries. The
/// native and browser app shares this with the headless integrator.
pub fn apply_advance_effects(world: &mut GameWorld, effects: &[MissionEffect]) {
    for effect in effects {
        match effect {
            MissionEffect::MemberMoved { member, to } => move_member(world, *member, *to),
            MissionEffect::MemberAvailable { member } => set_on_mission(world, *member, false),
            MissionEffect::CharacterCaptured {
                character,
                captured_by,
                at_system,
            } => capture_character(world, *character, *captured_by, *at_system),
            MissionEffect::SpecialForceDestroyed { unit } => destroy_special_force(world, *unit),
            _ => {}
        }
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
        c.current_system = Some(at_system);
        c.current_fleet = None;
    }
    for (_, fleet) in &mut world.fleets {
        fleet.characters.retain(|&k| k != character);
    }
}

/// Remove a special force from its system and the world.
pub fn destroy_special_force(world: &mut GameWorld, unit: crate::ids::SpecialForceKey) {
    for (_, system) in &mut world.systems {
        system.special_forces.retain(|&key| key != unit);
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
}

/// All active missions across the galaxy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MissionState {
    missions: VecDeque<ActiveMission>,
    next_id: u64,
    /// Members on their way to a target. A member keeps travelling when its
    /// mission ends, as its own transit timer does in the original.
    en_route: Vec<MemberTransit>,
}

impl MissionState {
    #[must_use]
    pub fn new() -> Self {
        MissionState {
            missions: VecDeque::new(),
            next_id: 0,
            en_route: Vec::new(),
        }
    }

    /// Members travelling to a mission target.
    #[must_use]
    pub fn en_route(&self) -> &[MemberTransit] {
        &self.en_route
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
        self.missions.push_back(mission);
        id
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
                let busy_elsewhere = self
                    .missions
                    .iter()
                    .any(|m| m.members().any(|x| x == member))
                    || self.is_en_route(member);
                if state.is_alliance != side_is_alliance || state.busy || busy_elsewhere {
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
        for member in team.iter().chain(&decoys).chain(&captured) {
            set_on_mission(world, *member, true);
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

    /// Queue a mission built in a test, past the dispatch rules.
    #[cfg(test)]
    pub(crate) fn push_timed(&mut self, mission: ActiveMission) {
        self.next_id = self.next_id.max(mission.id + 1);
        self.missions.push_back(mission);
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
    /// System popularity shifted toward the faction by `delta` (0.0–1.0 scale).
    PopularityShifted {
        system: SystemKey,
        faction: MissionFaction,
        /// Amount added to the faction's popularity (rebellion2 uses +1/100 per success).
        delta: f32,
    },
    /// A character was recruited to the faction (placed at the target system).
    CharacterRecruited {
        system: SystemKey,
        /// The character who conducted the recruitment.
        recruiter: CharacterKey,
        faction: MissionFaction,
    },

    // ── War Machine ──────────────────────────────────────────────────────────
    /// A facility was sabotaged — reduce its remaining production ticks.
    ///
    /// The caller applies `ticks_lost` to the appropriate facility at
    /// `system.manufacturing_facilities[facility_index]` or
    /// `system.defense_facilities[facility_index]`.
    FacilitySabotaged {
        system: SystemKey,
        /// Index into the system's facility list (manufacturing or defense).
        facility_index: usize,
        /// Production ticks lost due to sabotage.
        ticks_lost: u32,
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

    /// An uprising was incited — shift popularity against the controlling faction.
    UprisingStarted {
        system: SystemKey,
        /// Popularity delta applied against the controlling faction (positive = shift toward other side).
        popularity_delta: f32,
    },

    // ── Member availability tracking ────────────────────────────────────────
    /// A member has left its mission and may take another
    /// (`RoleOnMissionNotif` `FUN_00536b00` cleared).
    MemberAvailable { member: MissionMember },
    /// A member moved for its mission: off the map when it starts a transit
    /// (`to: None`), into the target system when it lands (`FUN_00556390`).
    MemberMoved {
        member: MissionMember,
        to: Option<SystemKey>,
    },

    // ── Decoy / Escape ──────────────────────────────────────────────────────
    /// A decoy drew a defender away from the mission. Nothing raises it until
    /// the recovered decoy phase (`FUN_0058a020`) is ported (F-019).
    DecoyTriggered {
        system: SystemKey,
        decoy_character: CharacterKey,
    },
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

    /// Death Star construction was sabotaged — delay by `ticks_delayed`.
    DeathStarSabotaged {
        /// Number of construction ticks added to the countdown.
        ticks_delayed: u32,
    },

    /// A special force captured on a mission is destroyed (slot `+0x20c`,
    /// `FUN_00503eb0` -> slot `+0xac(9)`).
    SpecialForceDestroyed { unit: crate::ids::SpecialForceKey },

    /// A character took the injury roll `FUN_004ef5f0` (health `+0x94`).
    /// port: the port stores no injury (F-026), so applying it changes
    /// nothing.
    CharacterInjured {
        character: CharacterKey,
        injury: i32,
    },
}

// ---------------------------------------------------------------------------
// MissionOutcome / MissionResult
// ---------------------------------------------------------------------------

/// Whether a mission succeeded, failed, or was foiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionOutcome {
    Success,
    Failure,
    /// Enemy counter-intelligence detected and blocked the mission.
    Foiled,
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
    /// The team character the interim resolver rolled for
    /// (`ActiveMission::lead_character`).
    pub character: Option<CharacterKey>,
    pub target_system: SystemKey,
    pub outcome: MissionOutcome,
    /// World-state changes to apply (only non-empty on Success).
    pub effects: Vec<MissionEffect>,
}

// ---------------------------------------------------------------------------
// Probability helpers (pure functions, testable)
// ---------------------------------------------------------------------------

/// Evaluate the quadratic success-probability formula.
///
/// Returns a probability in percent [0, 100].
#[must_use]
pub fn quadratic_prob(score: f64, a: f64, b: f64, c: f64) -> f64 {
    a * score * score + b * score + c
}

/// Clamp a probability to the mission's configured [min%, max%] range.
#[must_use]
pub fn clamp_prob(p: f64, min: f64, max: f64) -> f64 {
    p.max(min).min(max)
}

/// Combined success probability: `agent_prob% * (1 - foil_prob%)`.
///
/// Both inputs are in percent [0, 100]; output is in percent [0, 100].
#[must_use]
pub fn total_success_prob(agent_prob_pct: f64, foil_prob_pct: f64) -> f64 {
    let agent = agent_prob_pct / 100.0;
    let foil = foil_prob_pct / 100.0;
    agent * (1.0 - foil) * 100.0
}

/// Foil probability from defense score.
///
/// Coefficients from Mission.cs `FoilProbability`: -0.001999·d² + 0.8879·d + 84.61.
/// Returns 0.0 if the mission is in a friendly system (no counter-intel threat).
#[expect(
    clippy::manual_clamp,
    reason = "min/max map NaN to the lower bound; clamp would propagate NaN."
)]
#[must_use]
pub fn foil_prob(defense_score: f64, own_system: bool) -> f64 {
    if own_system {
        return 0.0;
    }
    quadratic_prob(defense_score, -0.001_999, 0.8879, 84.61)
        .max(0.0)
        .min(100.0)
}

/// Compute the counter-intelligence defense score at a target system.
///
/// Sums the espionage skill of all enemy characters present at the system.
/// The enemy faction is the opposite of whoever dispatched the mission.
/// A higher score means stronger counter-intelligence, raising the foil probability.
///
/// From the original game: enemy operatives stationed at a system can detect
/// and block covert missions. The system's `espionage_rating` provides a
/// baseline; stationed characters add their espionage skill on top.
#[must_use]
pub fn compute_defense_score(
    world: &GameWorld,
    target_system: SystemKey,
    mission_faction: MissionFaction,
) -> f64 {
    let Some(sys) = world.systems.get(target_system) else {
        return 0.0;
    };

    // Baseline from the system's own counter-intelligence rating.
    let mut score = f64::from(sys.espionage_rating);

    // Add espionage skill of enemy characters stationed at this system.
    for (_key, character) in &world.characters {
        if character.current_system != Some(target_system) {
            continue;
        }
        // Only count characters belonging to the opposing faction.
        let is_enemy = match mission_faction {
            MissionFaction::Alliance => character.is_empire,
            MissionFaction::Empire => character.is_alliance,
        };
        if is_enemy {
            // Use base espionage skill (expected value of the pair).
            score += f64::from(character.espionage.base);
        }
    }

    score
}

/// Whether the target system is controlled by the mission's faction.
///
/// Missions in friendly systems face no counter-intelligence threat.
#[must_use]
pub fn is_own_system(world: &GameWorld, target_system: SystemKey, faction: MissionFaction) -> bool {
    let Some(sys) = world.systems.get(target_system) else {
        return false;
    };
    match sys.control.faction() {
        Some(crate::dat::Faction::Alliance) => faction == MissionFaction::Alliance,
        Some(crate::dat::Faction::Empire) => faction == MissionFaction::Empire,
        _ => false,
    }
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
            | MissionKind::SubdueUprising => TargetRules::System,
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
fn member_location(world: &GameWorld, member: MissionMember) -> Option<SystemKey> {
    match member_state(world, member)?.location {
        MemberLocation::Fleet(fleet) => world.fleets.get(fleet).map(|f| f.location),
        MemberLocation::System(system) => Some(system),
        MemberLocation::Nowhere => None,
    }
}

/// The side a character reads as. Inference: a prisoner reads as its
/// captor's side (`ghidra/notes/mission-lifecycle.md`, "The running checks").
fn character_side(character: &Character) -> Option<crate::dat::Faction> {
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
/// its container, and so is a Sabotage target until F-019 phase 4 names
/// target objects. A system reads as located in itself (`hyp:` the system
/// slot `+0xc` is untraced) and as its holder's side (inference).
fn target_view(
    mission: &ActiveMission,
    world: &GameWorld,
    en_route: &[MemberTransit],
) -> Option<TargetView> {
    match mission.kind.target_rules() {
        TargetRules::System | TargetRules::Object => {
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
    en_route: &[MemberTransit],
) -> u8 {
    use crate::dat::Faction;
    // Rule 1 (FUN_00522480.c:42-52): end 5 unless a team member stands
    // without a remove or resign request; an empty team ends too, and a
    // killed member is deleted. port: the port has no remove requests, and
    // +0xa4 bit 1, which lifts the rule, is untraced and taken as clear.
    let standing = mission.team.iter().any(|&member| {
        member_state(world, member).is_some() && !mission.resigning.contains(&member)
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
        TargetRules::Object => {}
    }
    // FUN_00573ee0: Diplomacy stops at full support. Recruitment's end 0x10
    // (FUN_0056b370, the side's recruit pool is empty) waits for the recruit
    // pick of F-019 phase 4.
    if mission.kind == MissionKind::Diplomacy
        && crate::uprising::support_points(container, side) == FULL_SUPPORT
    {
        return 0xf;
    }
    0
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

/// Draws one mission may take on one tick event: the creation chain's timer,
/// then the outcome roll, the Subdue Uprising support draw
/// (`FUN_0055cb10`), and the repeat's timer, then the seed of the detection
/// draws (`mission_detection`). port: the size of the port's per-mission
/// window.
pub const ROLLS_PER_MISSION: usize = 5;

/// port: the window slot that seeds the detection draws; the draws before
/// it are taken in order.
const DETECTION_SEED_ROLL: usize = 4;

/// One mission's window of rolls for one tick event.
struct MissionRolls<'a> {
    rolls: &'a [f64],
    next: usize,
    detection: crate::mission_detection::DetectionRng,
}

impl MissionRolls<'_> {
    /// port: a draw past the window reads 0.5, the port's under-supply
    /// fallback.
    fn next(&mut self) -> f64 {
        let roll = self
            .rolls
            .get(self.next)
            .filter(|_| self.next < DETECTION_SEED_ROLL)
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
        tick_events: &[TickEvent],
        rolls: &[f64],
    ) -> MissionAdvance {
        let ctx = StepContext { world, uprisings };
        let mut out = MissionAdvance::default();
        let count = state.missions.len();
        for (event_index, event) in tick_events.iter().enumerate() {
            let now = event.tick;
            // FUN_00545820: members whose transit is due arrive.
            let (arrived, travelling): (Vec<_>, Vec<_>) = std::mem::take(&mut state.en_route)
                .into_iter()
                .partition(|transit| transit.arrival <= now);
            state.en_route = travelling;
            out.effects.extend(
                arrived
                    .into_iter()
                    .map(|transit| MissionEffect::MemberMoved {
                        member: transit.member,
                        to: Some(transit.to),
                    }),
            );

            let missions = std::mem::take(&mut state.missions);
            for (index, mut mission) in missions.into_iter().enumerate() {
                // port: the mission's own window (see the RNG contract).
                let start = (event_index * count + index) * ROLLS_PER_MISSION;
                let window = rolls
                    .get(start..(start + ROLLS_PER_MISSION).min(rolls.len()))
                    .unwrap_or(&[]);
                let mut draws = MissionRolls {
                    rolls: window,
                    next: 0,
                    detection: crate::mission_detection::DetectionRng::seeded(
                        window.get(DETECTION_SEED_ROLL).copied().unwrap_or(0.5),
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
                if mission.phase == PHASE_END {
                    out.effects.extend(
                        mission
                            .members()
                            .map(|member| MissionEffect::MemberAvailable { member }),
                    );
                    out.ended.push(MissionEnded {
                        mission_id: mission.id,
                        kind: mission.kind,
                        faction: mission.faction,
                        end_code: mission.end_code,
                        tick: now,
                    });
                } else {
                    state.missions.push_back(mission);
                }
            }
        }
        out
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
            mission.end_code = running_end_code(mission, ctx.world, ctx.uprisings, &state.en_route);
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
                &state.en_route,
                &mut draws.detection,
                &mut out.effects,
            );
            // FUN_00522980: every phase but 4 and 8 steps on at once.
            if !matches!(mission.phase, PHASE_TRANSIT | PHASE_TIMER) {
                mission.ready = true;
            }
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
                mission.end_code = running_end_code(mission, world, ctx.uprisings, &state.en_route);
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
                mission.speed = Self::character_speed(mission, world);
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
            }
            PHASE_OUTCOME => {
                let roll = draws.next();
                let mut result = Self::resolve_mission(mission, world, now, roll);
                for effect in &mut result.effects {
                    if let MissionEffect::UprisingSubdued {
                        system,
                        side,
                        support_gain,
                    } = effect
                    {
                        *support_gain = crate::uprising::subdue_support_gain(
                            world,
                            *system,
                            *side,
                            draws.next(),
                        );
                    }
                }
                out.results.push(result);
            }
            PHASE_END => {
                validate(mission, state);
                if mission.end_code == 0 {
                    mission.end_code = 1;
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
        let Some(from) = mission.origin else {
            return;
        };
        let position = |key: SystemKey| world.systems.get(key).map_or((0, 0), |s| (s.x, s.y));
        let to = mission.target_system;
        for member in mission.members() {
            if member_state(world, member).is_none() {
                continue;
            }
            let days = crate::movement::transit_ticks_between(
                world,
                position(from),
                position(to),
                mission.speed,
            );
            if days == 0 {
                out.effects.push(MissionEffect::MemberMoved {
                    member,
                    to: Some(to),
                });
            } else {
                // port: a travelling member is off the map, as a delivery is.
                out.effects
                    .push(MissionEffect::MemberMoved { member, to: None });
                state.en_route.push(MemberTransit {
                    member,
                    mission_id: mission.id,
                    to,
                    arrival: now + u64::from(days),
                });
            }
        }
    }

    /// Resolve a single completed mission into a `MissionResult`.
    fn resolve_mission(
        mission: &ActiveMission,
        world: &GameWorld,
        tick: u64,
        roll: f64,
    ) -> MissionResult {
        let lead = mission.lead_character();
        let character = lead.and_then(|key| world.characters.get(key));
        let target_system = world.systems.get(mission.target_system);

        // Compute composite table input per original game formulas (TheArchitect2018 wiki).
        let target_char = mission
            .target_character
            .and_then(|k| world.characters.get(k));
        let table_input = character.map_or(0, |c| {
            mission
                .kind
                .compute_table_input(c, target_system, mission.faction, target_char)
        });

        // Compute counter-intelligence inputs for covert missions.
        let defense_score = compute_defense_score(world, mission.target_system, mission.faction);
        let own_system = is_own_system(world, mission.target_system, mission.faction);

        let (outcome, effects) = Self::determine_outcome(
            mission,
            character,
            table_input,
            tick,
            roll,
            &world.mission_tables,
            defense_score,
            own_system,
        );

        MissionResult {
            mission_id: mission.id,
            tick,
            kind: mission.kind,
            faction: mission.faction,
            character: lead,
            target_system: mission.target_system,
            outcome,
            effects,
        }
    }

    /// Compute outcome and effects given a pre-rolled value.
    ///
    /// Uses MSTB table lookup when available in `world.mission_tables`,
    /// falls back to quadratic coefficients otherwise. The `table_input` is
    /// a composite value computed from character skill + game context per the
    /// original REBEXE.EXE formulas (see `MissionKind::compute_table_input`).
    ///
    /// For covert missions, `defense_score` and `own_system` determine the
    /// foil probability from enemy counter-intelligence. Non-covert missions
    /// ignore the foil path entirely.
    #[expect(
        clippy::too_many_arguments,
        reason = "Keep the existing explicit simulation inputs; grouping them changes the API."
    )]
    fn determine_outcome(
        mission: &ActiveMission,
        character: Option<&Character>,
        table_input: i32,
        _tick: u64,
        roll: f64,
        mission_tables: &HashMap<String, MstbTable>,
        defense_score: f64,
        own_system: bool,
    ) -> (MissionOutcome, Vec<MissionEffect>) {
        // Autoscrap always succeeds — no character or probability check needed.
        if mission.kind == MissionKind::Autoscrap {
            return (MissionOutcome::Success, Self::build_effects(mission));
        }

        let skill_score: u32 = character.map_or(0, |c| mission.kind.skill_score(c));

        // Priority 1: MSTB table lookup using composite input (per original game formulas).
        // Priority 2: quadratic fallback using raw skill_score (rebellion2 Mission.cs).
        let agent_prob = if let Some(key) = mission.kind.mstb_key() {
            if let Some(table) = mission_tables.get(key) {
                let raw = f64::from(table.lookup(table_input));
                clamp_prob(
                    raw,
                    mission.kind.min_success_prob(),
                    mission.kind.max_success_prob(),
                )
            } else {
                let (a, b, c) = mission.kind.coefficients();
                let raw = quadratic_prob(f64::from(skill_score), a, b, c);
                clamp_prob(
                    raw,
                    mission.kind.min_success_prob(),
                    mission.kind.max_success_prob(),
                )
            }
        } else {
            100.0 // No MSTB key → always succeeds (only Autoscrap, handled above)
        };

        // Counter-intelligence foil check for covert missions.
        // From Mission.cs FoilProbability: quadratic formula on defense_score.
        // Non-covert missions (diplomacy, recruitment, uprising) are never foiled.
        let foil_pct = if mission.kind.is_covert() {
            foil_prob(defense_score, own_system)
        } else {
            0.0
        };
        let success_prob = total_success_prob(agent_prob, foil_pct);

        if roll * 100.0 <= success_prob {
            let effects = Self::build_effects(mission);
            (MissionOutcome::Success, effects)
        } else if mission.kind.is_covert() && foil_pct > 0.0 {
            // The mission was foiled by enemy counter-intelligence.
            // Distinguish from plain failure: if the agent would have succeeded
            // without foil interference (roll <= agent_prob), it was specifically
            // the counter-intel that blocked it.
            if roll * 100.0 <= agent_prob {
                (MissionOutcome::Foiled, Vec::new())
            } else {
                (MissionOutcome::Failure, Vec::new())
            }
        } else {
            (MissionOutcome::Failure, Vec::new())
        }
    }

    /// Build the world-state effects for a successful mission.
    ///
    /// Conservative defaults for War Machine types: the caller is expected to
    /// apply these effects to `GameWorld` and may override the values based on
    /// additional game state (e.g. facility selection for Sabotage).
    fn build_effects(mission: &ActiveMission) -> Vec<MissionEffect> {
        match mission.kind {
            MissionKind::Diplomacy => {
                vec![MissionEffect::PopularityShifted {
                    system: mission.target_system,
                    faction: mission.faction,
                    // rebellion2 uses +1 on a 0–100 integer scale → +0.01 on our f32 [0,1]
                    delta: 0.01,
                }]
            }
            MissionKind::Recruitment => mission
                .lead_character()
                .map(|recruiter| MissionEffect::CharacterRecruited {
                    system: mission.target_system,
                    recruiter,
                    faction: mission.faction,
                })
                .into_iter()
                .collect(),
            MissionKind::Sabotage => {
                // Target facility index 0 as default — callers should select the
                // specific facility based on game state before dispatching.
                vec![MissionEffect::FacilitySabotaged {
                    system: mission.target_system,
                    facility_index: 0,
                    // ~10 ticks of production lost per successful sabotage.
                    ticks_lost: 10,
                }]
            }
            MissionKind::Assassination => {
                if let Some(target) = mission.target_character {
                    vec![MissionEffect::CharacterKilled {
                        character: target,
                        faction: mission.faction,
                    }]
                } else {
                    vec![] // No target specified — no effect
                }
            }
            MissionKind::Espionage => {
                vec![MissionEffect::SystemIntelligenceGathered {
                    system: mission.target_system,
                    faction: mission.faction,
                }]
            }
            MissionKind::Rescue => {
                if let Some(target) = mission.target_character {
                    vec![MissionEffect::CharacterRescued {
                        character: target,
                        returned_to: mission.faction,
                        at_system: mission.target_system,
                    }]
                } else {
                    vec![]
                }
            }
            MissionKind::Abduction => {
                if let Some(target) = mission.target_character {
                    let captured_by = mission.faction;
                    vec![MissionEffect::CharacterCaptured {
                        character: target,
                        captured_by,
                        at_system: mission.target_system,
                    }]
                } else {
                    vec![]
                }
            }
            MissionKind::InciteUprising => {
                vec![MissionEffect::UprisingStarted {
                    system: mission.target_system,
                    // +0.05 popularity delta: more impactful than diplomacy (+0.01).
                    popularity_delta: 0.05,
                }]
            }
            MissionKind::SubdueUprising => {
                vec![MissionEffect::UprisingSubdued {
                    system: mission.target_system,
                    side: mission.faction.into(),
                    // MissionSystem::advance draws the gain.
                    support_gain: 0,
                }]
            }
            MissionKind::DeathStarSabotage => {
                vec![MissionEffect::DeathStarSabotaged {
                    // Delay construction by ~50 ticks (significant setback).
                    ticks_delayed: 50,
                }]
            }
            MissionKind::Autoscrap => {
                // Autoscrap has no world-state effects — the actual unit removal
                // is handled outside the mission system.
                Vec::new()
            }
        }
    }

    /// Check for captured characters escaping on their own.
    ///
    /// For each character held by the opposing faction, look up ESCAPETB
    /// and roll against the escape probability. Returns one `CharacterEscaped`
    /// effect per successful escape.
    #[must_use]
    pub fn check_escapes(world: &GameWorld, rolls: &[f64]) -> Vec<MissionEffect> {
        let Some(table) = world.mission_tables.get("ESCAPETB") else {
            return Vec::new();
        };

        let mut effects = Vec::new();
        let mut roll_iter = rolls.iter().copied();

        for (char_key, character) in &world.characters {
            if !character.is_captive {
                continue;
            }
            let roll = roll_iter.next().unwrap_or(1.0); // 1.0 = no escape (safe default)
                                                        // Use loyalty as the skill score for escape probability
            let skill_score = character.loyalty.base.cast_signed();
            let escape_prob = f64::from(table.lookup(skill_score)) / 100.0;
            if roll < escape_prob {
                // Character escapes back to their home faction.
                // is_alliance reflects original allegiance — capture tracks captor,
                // not the character's identity.
                let escaped_to_alliance = character.is_alliance;
                effects.push(MissionEffect::CharacterEscaped {
                    character: char_key,
                    escaped_to_alliance,
                });
            }
        }

        effects
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CharacterKey, SectorKey, SystemKey};
    use crate::uprising::UprisingState;
    use crate::world::{Character, GameWorld, SkillPair};

    // --- Probability formula tests ---

    #[test]
    fn quadratic_prob_at_zero_score_equals_constant() {
        let (a, b, c) = MissionKind::Diplomacy.coefficients();
        let p = quadratic_prob(0.0, a, b, c);
        assert!((p - c).abs() < 1e-9, "expected {c}, got {p}");
    }

    #[test]
    fn quadratic_prob_diplomacy_at_max_skill() {
        // Max skill score is 100 (u32 base) — should be near 100%.
        let (a, b, c) = MissionKind::Diplomacy.coefficients();
        let p = quadratic_prob(100.0, a, b, c);
        let clamped = clamp_prob(p, 1.0, 100.0);
        assert!(
            clamped >= 90.0,
            "expected high probability at max skill, got {clamped}"
        );
    }

    #[test]
    fn total_success_prob_no_foil() {
        // With 0% foil, total == agent prob.
        let total = total_success_prob(75.0, 0.0);
        assert!((total - 75.0).abs() < 1e-9);
    }

    #[test]
    fn total_success_prob_full_foil() {
        // With 100% foil, total == 0%.
        let total = total_success_prob(75.0, 100.0);
        assert!(total.abs() < 1e-9);
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "Friendly systems must return exactly zero, not an approximation."
    )]
    fn foil_prob_own_system_is_zero() {
        assert_eq!(foil_prob(99.0, true), 0.0);
    }

    #[test]
    fn foil_prob_enemy_system_increases_with_defense() {
        let low = foil_prob(0.0, false);
        let high = foil_prob(50.0, false);
        assert!(high > low, "expected foil to increase with defense score");
    }

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

    // --- is_covert classification ---

    #[test]
    fn covert_missions_classified_correctly() {
        assert!(MissionKind::Sabotage.is_covert());
        assert!(MissionKind::Assassination.is_covert());
        assert!(MissionKind::Espionage.is_covert());
        assert!(MissionKind::Rescue.is_covert());
        assert!(MissionKind::Abduction.is_covert());
        assert!(MissionKind::DeathStarSabotage.is_covert());
    }

    #[test]
    fn non_covert_missions_classified_correctly() {
        assert!(!MissionKind::Diplomacy.is_covert());
        assert!(!MissionKind::Recruitment.is_covert());
        assert!(!MissionKind::InciteUprising.is_covert());
        assert!(!MissionKind::SubdueUprising.is_covert());
        assert!(!MissionKind::Autoscrap.is_covert());
    }

    // --- Counter-intelligence integration ---

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "An empty defense must produce exactly zero."
    )]
    fn defense_score_zero_with_no_enemies() {
        let world = GameWorld::default();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let sys_key = sys_sm.insert(());
        // System not in world → score is 0.
        let score = compute_defense_score(&world, sys_key, MissionFaction::Alliance);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn own_system_returns_false_for_missing_system() {
        let world = GameWorld::default();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let sys_key = sys_sm.insert(());
        assert!(!is_own_system(&world, sys_key, MissionFaction::Alliance));
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
        let results =
            MissionSystem::advance(&mut state, &world, &UprisingState::default(), &[], &[]).results;
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
    fn diplomacy_success_emits_popularity_effect() {
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let character = character_with_diplomacy(&mut world, 90);

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

        let results = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &[TickEvent { tick: 1 }],
            &[0.0], // guaranteed success
        )
        .results;
        assert_eq!(results[0].outcome, MissionOutcome::Success);
        // PopularityShifted only; the release comes with the mission's end.
        assert_eq!(results[0].effects.len(), 1);
        match &results[0].effects[0] {
            MissionEffect::PopularityShifted { faction, delta, .. } => {
                assert_eq!(*faction, MissionFaction::Alliance);
                assert!((delta - 0.01).abs() < 1e-6);
            }
            _ => panic!("expected PopularityShifted"),
        }
    }

    #[test]
    fn recruitment_success_emits_recruit_effect() {
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let character = world.characters.insert(Character {
            name: "Leader".into(),
            is_alliance: true,
            leadership: skill_pair(80),
            can_be_commander: true,
            ..Default::default()
        });

        let mut state = MissionState::new();
        state.missions.push_back(ActiveMission::timed(
            0,
            MissionKind::Recruitment,
            MissionFaction::Empire,
            vec![MissionMember::Character(character)],
            system,
            1,
        ));
        state.next_id = 1;

        let results = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &[TickEvent { tick: 5 }],
            &[0.0], // guaranteed success
        )
        .results;
        assert_eq!(results[0].outcome, MissionOutcome::Success);
        match &results[0].effects[0] {
            MissionEffect::CharacterRecruited { faction, .. } => {
                assert_eq!(*faction, MissionFaction::Empire);
            }
            _ => panic!("expected CharacterRecruited"),
        }
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
            &[TickEvent { tick: 1 }],
            &[1.0],
        )
        .results;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, MissionOutcome::Success);
    }

    #[test]
    fn sabotage_success_emits_facility_sabotaged() {
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let character = world.characters.insert(Character {
            name: "Spy".into(),
            is_alliance: true,
            espionage: skill_pair(80), // high espionage
            can_be_commander: true,
            ..Default::default()
        });

        let mut state = MissionState::new();
        state.missions.push_back(ActiveMission::timed(
            0,
            MissionKind::Sabotage,
            MissionFaction::Alliance,
            vec![MissionMember::Character(character)],
            system,
            1,
        ));
        state.next_id = 1;

        let results = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &[TickEvent { tick: 1 }],
            &[0.0], // guaranteed success
        )
        .results;
        assert_eq!(results[0].outcome, MissionOutcome::Success);
        assert!(results[0]
            .effects
            .iter()
            .any(|e| matches!(e, MissionEffect::FacilitySabotaged { .. })));
    }

    #[test]
    fn incite_uprising_success_emits_uprising_started() {
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let character = world.characters.insert(Character {
            name: "Agitator".into(),
            is_alliance: true,
            diplomacy: skill_pair(70),
            can_be_commander: true,
            ..Default::default()
        });

        let mut state = MissionState::new();
        state.missions.push_back(ActiveMission::timed(
            0,
            MissionKind::InciteUprising,
            MissionFaction::Alliance,
            vec![MissionMember::Character(character)],
            system,
            1,
        ));
        state.next_id = 1;

        let results = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &[TickEvent { tick: 1 }],
            &[0.0],
        )
        .results;
        assert_eq!(results[0].outcome, MissionOutcome::Success);
        assert!(results[0]
            .effects
            .iter()
            .any(|e| matches!(e, MissionEffect::UprisingStarted { .. })));
    }

    #[test]
    fn mstb_table_lookup_used_when_populated() {
        use crate::world::{MstbEntry, MstbTable};

        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let character = world.characters.insert(Character {
            name: "Diplomat".into(),
            is_alliance: true,
            diplomacy: skill_pair(50), // score = 50 → threshold delta = 0 → table midpoint
            can_be_commander: true,
            ..Default::default()
        });

        // Install a minimal DIPLMSTB table: two entries, threshold 0 = value 99.
        // With score=50 → delta=0 → table lookup returns 99 → guaranteed success.
        let table = MstbTable::new(vec![
            MstbEntry {
                threshold: -50,
                value: 1,
            },
            MstbEntry {
                threshold: 0,
                value: 99,
            },
            MstbEntry {
                threshold: 50,
                value: 100,
            },
        ]);
        world.mission_tables.insert("DIPLMSTB".to_string(), table);

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

        // roll = 0.98 → 98% < 99% success → should succeed via MSTB table
        let results = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &[TickEvent { tick: 1 }],
            &[0.98],
        )
        .results;
        assert_eq!(
            results[0].outcome,
            MissionOutcome::Success,
            "MSTB table lookup should yield ~99% success at skill=50"
        );
    }

    #[test]
    fn espionage_success_emits_intelligence_gathered() {
        let mut world = minimal_world();
        let mut sys_sm: slotmap::SlotMap<SystemKey, ()> = slotmap::SlotMap::with_key();
        let system = sys_sm.insert(());
        let character = world.characters.insert(Character {
            name: "Intel".into(),
            is_alliance: true,
            espionage: skill_pair(75),
            can_be_commander: true,
            ..Default::default()
        });

        let mut state = MissionState::new();
        state.missions.push_back(ActiveMission::timed(
            0,
            MissionKind::Espionage,
            MissionFaction::Alliance,
            vec![MissionMember::Character(character)],
            system,
            1,
        ));
        state.next_id = 1;

        let results = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &[TickEvent { tick: 1 }],
            &[0.0],
        )
        .results;
        assert_eq!(results[0].outcome, MissionOutcome::Success);
        assert!(results[0]
            .effects
            .iter()
            .any(|e| matches!(e, MissionEffect::SystemIntelligenceGathered { .. })));
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

    // --- Escape tests ---

    #[test]
    fn check_escapes_returns_escaped_when_roll_below_prob() {
        use crate::dat::Faction;
        use crate::world::{MstbEntry, MstbTable};

        let mut world = minimal_world();
        let _char_key = world.characters.insert(Character {
            name: "Captive".into(),
            is_alliance: true,
            loyalty: skill_pair(50), // loyalty base = 50
            captured_by: Some(Faction::Empire),
            capture_tick: Some(10),
            is_captive: true,
            ..Default::default()
        });

        // ESCAPETB: loyalty 50 → 80% escape probability
        let table = MstbTable::new(vec![
            MstbEntry {
                threshold: 0,
                value: 50,
            },
            MstbEntry {
                threshold: 50,
                value: 80,
            },
            MstbEntry {
                threshold: 100,
                value: 95,
            },
        ]);
        world.mission_tables.insert("ESCAPETB".to_string(), table);

        // Roll 0.5 < 0.80 → escapes
        let effects = MissionSystem::check_escapes(&world, &[0.5]);
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            MissionEffect::CharacterEscaped {
                escaped_to_alliance,
                ..
            } => {
                // Captured by Empire → escapes TO Alliance
                assert!(*escaped_to_alliance);
            }
            other => panic!("expected CharacterEscaped, got {other:?}"),
        }
    }

    #[test]
    fn check_escapes_skips_non_captive() {
        use crate::world::{MstbEntry, MstbTable};

        let mut world = minimal_world();
        world.characters.insert(Character {
            name: "Free".into(),
            is_alliance: true,
            loyalty: skill_pair(90),
            // is_captive defaults to false
            ..Default::default()
        });

        let table = MstbTable::new(vec![
            MstbEntry {
                threshold: 0,
                value: 100,
            }, // would always escape if checked
        ]);
        world.mission_tables.insert("ESCAPETB".to_string(), table);

        // Roll that would succeed — but character is not captive
        let effects = MissionSystem::check_escapes(&world, &[0.0]);
        assert!(effects.is_empty());
    }

    #[test]
    fn check_escapes_returns_empty_without_escapetb() {
        let world = minimal_world();
        // No ESCAPETB in mission_tables
        let effects = MissionSystem::check_escapes(&world, &[0.0, 0.0, 0.0]);
        assert!(effects.is_empty());
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
            tick: 0,
        }
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
        // port: the interim resolver rolls one character until F-019 phase 4
        // ports the per-member roll; a special force is never that character.
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

    // --- P0 formula correction tests ---

    fn character_with_skills(
        world: &mut GameWorld,
        espionage: u32,
        combat: u32,
        diplomacy: u32,
        leadership: u32,
        loyalty: u32,
    ) -> CharacterKey {
        world.characters.insert(Character {
            name: "Agent".into(),
            is_alliance: true,
            diplomacy: skill_pair(diplomacy),
            espionage: skill_pair(espionage),
            combat: skill_pair(combat),
            leadership: skill_pair(leadership),
            loyalty: skill_pair(loyalty),
            can_be_commander: true,
            ..Default::default()
        })
    }

    #[test]
    fn sabotage_uses_composite_espionage_and_combat() {
        let mut world = GameWorld::default();
        let key = character_with_skills(&mut world, 60, 40, 0, 0, 0);
        let character = world.characters.get(key).unwrap();
        let input = MissionKind::Sabotage.compute_table_input(
            character,
            None,
            MissionFaction::Alliance,
            None,
        );
        // (60 + 40) / 2 = 50
        assert_eq!(input, 50, "sabotage should use (espionage + combat) / 2");
    }

    #[test]
    fn incite_uprising_equals_diplomacy_with_zero_espionage() {
        let mut world = GameWorld::default();
        let key = character_with_skills(&mut world, 0, 0, 80, 0, 0);
        let character = world.characters.get(key).unwrap();

        let sys = crate::world::System {
            dat_id: crate::ids::DatId(0),
            name: "Test".into(),
            sector: SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.3,
            popularity_empire: 0.6,
            is_populated: false,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            is_headquarters: false,
            is_destroyed: false,
            control: crate::world::ControlKind::Uncontrolled,
        };

        let diplomacy_input = MissionKind::Diplomacy.compute_table_input(
            character,
            Some(&sys),
            MissionFaction::Alliance,
            None,
        );
        let incite_input = MissionKind::InciteUprising.compute_table_input(
            character,
            Some(&sys),
            MissionFaction::Alliance,
            None,
        );
        assert_eq!(
            incite_input, diplomacy_input,
            "with espionage_rating=0, incite should equal diplomacy"
        );
    }

    #[test]
    fn incite_uprising_reduced_by_espionage_rating() {
        let mut world = GameWorld::default();
        let key = character_with_skills(&mut world, 0, 0, 80, 0, 0);
        let character = world.characters.get(key).unwrap();

        let sys = crate::world::System {
            dat_id: crate::ids::DatId(0),
            name: "Test".into(),
            sector: SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.3,
            popularity_empire: 0.6,
            is_populated: false,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.25,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            is_headquarters: false,
            is_destroyed: false,
            control: crate::world::ControlKind::Uncontrolled,
        };

        let diplomacy_input = MissionKind::Diplomacy.compute_table_input(
            character,
            Some(&sys),
            MissionFaction::Alliance,
            None,
        );
        let incite_input = MissionKind::InciteUprising.compute_table_input(
            character,
            Some(&sys),
            MissionFaction::Alliance,
            None,
        );
        assert!(
            incite_input < diplomacy_input,
            "espionage_rating=0.25 should reduce incite input below diplomacy: {incite_input} vs {diplomacy_input}"
        );
        // 0.25 * 100 = 25 reduction
        assert_eq!(diplomacy_input - incite_input, 25);
    }

    #[test]
    fn abduction_subtracts_target_defense() {
        let mut world = GameWorld::default();
        let agent_key = character_with_skills(&mut world, 70, 0, 0, 0, 0);
        let target_key = character_with_skills(&mut world, 0, 40, 0, 0, 0);
        let agent = world.characters.get(agent_key).unwrap();
        let target = world.characters.get(target_key).unwrap();
        let input = MissionKind::Abduction.compute_table_input(
            agent,
            None,
            MissionFaction::Alliance,
            Some(target),
        );
        // 70 - 40 = 30
        assert_eq!(input, 30, "abduction should subtract target combat defense");
    }

    #[test]
    fn assassination_subtracts_target_defense() {
        let mut world = GameWorld::default();
        let agent_key = character_with_skills(&mut world, 0, 80, 0, 0, 0);
        let target_key = character_with_skills(&mut world, 0, 50, 0, 0, 0);
        let agent = world.characters.get(agent_key).unwrap();
        let target = world.characters.get(target_key).unwrap();
        let input = MissionKind::Assassination.compute_table_input(
            agent,
            None,
            MissionFaction::Alliance,
            Some(target),
        );
        // 80 - 50 = 30
        assert_eq!(
            input, 30,
            "assassination should subtract target combat defense"
        );
    }

    #[test]
    fn recruitment_uses_target_character_loyalty() {
        let mut world = GameWorld::default();
        let agent_key = character_with_skills(&mut world, 0, 0, 0, 60, 0);
        let target_key = character_with_skills(&mut world, 0, 0, 0, 0, 45);
        let agent = world.characters.get(agent_key).unwrap();
        let target = world.characters.get(target_key).unwrap();
        let input = MissionKind::Recruitment.compute_table_input(
            agent,
            None,
            MissionFaction::Alliance,
            Some(target),
        );
        // 60 - 45 = 15
        assert_eq!(
            input, 15,
            "recruitment should use target loyalty as resistance"
        );
    }
    // --- Lifecycle (FUN_005227d0, FUN_00524b70, FUN_00522480) ---

    /// A system at `(x, 0)`, populated, held by no one.
    fn system_at(world: &mut GameWorld, x: u16) -> SystemKey {
        world.systems.insert(crate::world::System {
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
            rules: crate::world::MissionTargetRules {
                target_loss_ends: true,
                opponent_target: true,
                prisoner_target: true,
                ..Default::default()
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
                arrival: 20
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
        // port: ROLLS_PER_MISSION per mission per tick; the first mission's
        // second draw never reaches the second mission's outcome roll.
        let (system, _) = mock_keys();
        let mut world = minimal_world();
        let character = agent_at(&mut world, system, true);
        let mut state = MissionState::new();
        for id in 0..2 {
            state.push_timed(ActiveMission::timed(
                id,
                MissionKind::Autoscrap,
                MissionFaction::Empire,
                vec![MissionMember::Character(character)],
                system,
                1,
            ));
        }
        state.push_timed(ActiveMission::timed(
            2,
            MissionKind::Diplomacy,
            MissionFaction::Alliance,
            vec![MissionMember::Character(character)],
            system,
            1,
        ));
        let mut rolls = vec![1.0; 3 * ROLLS_PER_MISSION];
        rolls[2 * ROLLS_PER_MISSION] = 0.0;

        let advance = step(&mut state, &world, 1, &rolls);

        assert_eq!(advance.results[2].outcome, MissionOutcome::Success);
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
        });
        world.characters[pilot].current_fleet = Some(fleet);
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
                rules: crate::world::MissionTargetRules {
                    target_loss_ends: true,
                    opponent_target: true,
                    free_target: true,
                    ..Default::default()
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
            rules: crate::world::MissionTargetRules {
                container_loss_ends: true,
                needs_populated_container: true,
                target_loss_ends: true,
                own_side_target: true,
                revolting_target: true,
                calm_target: true,
                ..Default::default()
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

        let advance = MissionSystem::advance(
            &mut state,
            &world,
            &UprisingState::default(),
            &[TickEvent { tick: 1 }, TickEvent { tick: 2 }],
            &rolls,
        );

        assert_eq!(advance.results[0].outcome, MissionOutcome::Success);
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
        assert_eq!(running_end_code(&mission, &world, &uprisings, &[]), 8);

        mission.decoys = vec![MissionMember::Character(agent_at(&mut world, here, false))];

        assert_eq!(running_end_code(&mission, &world, &uprisings, &[]), 0);
    }

    #[test]
    fn a_fifth_sequential_draw_reads_the_fallback_not_the_detection_seed() {
        // port: slot DETECTION_SEED_ROLL only seeds the detection stream.
        let window = [0.1, 0.2, 0.3, 0.4, 0.9];
        let mut draws = MissionRolls {
            rolls: &window,
            next: 0,
            detection: crate::mission_detection::DetectionRng::seeded(0.9, 0),
        };
        let taken: Vec<f64> = (0..5).map(|_| draws.next()).collect();
        assert_eq!(taken, vec![0.1, 0.2, 0.3, 0.4, 0.5]);
    }
}
