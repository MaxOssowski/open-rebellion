//! Victory condition detection.
//!
//! Checks the asymmetric original-game objectives each tick:
//! - **Alliance**: capture Coruscant and, in Standard mode, hold Emperor
//!   Palpatine and Darth Vader captive.
//! - **Empire**: destroy the mobile Alliance headquarters, then occupy its
//!   system, and, in Standard mode, hold Mon Mothma and Luke Skywalker captive.
//! - **Death Star route**: destroying the entire Alliance-HQ system satisfies
//!   the Empire's headquarters objective, but not Standard's leader objective.
//! - **Headquarters Only**: omits only the principal-leader requirements.
//!
//! # Architecture
//!
//! Follows the stateless advance pattern:
//! ```text
//! VictorySystem::check(&VictoryState, &world, &[TickEvent], VictoryConditions)
//!     -> Option<VictoryOutcome>
//! ```
//! Returns `None` every tick until a terminal condition is met, then returns
//! `Some(VictoryOutcome)`. The caller sets `VictoryState::resolved = true` to
//! suppress repeated checks after the first outcome.
//!
//! # Source
//!
//! Ghidra RE: `entity-system.md §4.2` — `SideVictoryConditionsNotif`,
//! `FinalBattle` (`FUN_0054ba00`), event IDs `0x12c`/`0x180`.
//! `IsHeadquarters` flag → `System::is_headquarters`.
//! Death Star: family `0x34`; fires when `alive_flag` bit0 == 0 (INVERTED).

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::ids::SystemKey;
use crate::tick::TickEvent;
use crate::world::{GameWorld, VictoryConditions};

// ---------------------------------------------------------------------------
// VictoryOutcome
// ---------------------------------------------------------------------------

/// Terminal game states returned by `VictorySystem::check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VictoryOutcome {
    /// The Alliance captured and controls Coruscant.
    ///
    /// `winner` controls `hq_system`; `loser` has lost their command center.
    HqCaptured {
        winner: Faction,
        loser: Faction,
        hq_system: SystemKey,
    },

    /// The Empire destroyed the mobile Alliance headquarters by bombardment
    /// and subsequently took control of its system.
    HqDestroyed {
        winner: Faction,
        loser: Faction,
        hq_system: SystemKey,
    },

    /// The Death Star destroyed the planet containing the Alliance HQ.
    ///
    /// The Empire wins: `target_system` has been destroyed.
    DeathStarVictory { target_system: SystemKey },
}

// ---------------------------------------------------------------------------
// VictoryState
// ---------------------------------------------------------------------------

/// Configuration for victory detection — set at game start.
///
/// Stores the HQ system for each faction and tracks Death Star status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VictoryState {
    /// The Alliance headquarters system key.
    pub alliance_hq: SystemKey,
    /// The Empire headquarters system key.
    pub empire_hq: SystemKey,
    /// When `true`, Death Star win-condition checks are active.
    pub death_star_active: bool,
    /// The system the Death Star is currently orbiting (if active).
    pub death_star_location: Option<SystemKey>,
    /// Set `true` once a `VictoryOutcome` has been returned to suppress
    /// re-firing on subsequent ticks.
    pub resolved: bool,
    /// The victory conditions word, galaxy `+0x74` (`FUN_005180e0`), that
    /// both sides copy to `+0xbc` (`FUN_00518460`; save v79).
    #[serde(default)]
    pub conditions_word: u32,
}

impl VictoryState {
    #[must_use]
    pub fn new(alliance_hq: SystemKey, empire_hq: SystemKey) -> Self {
        VictoryState {
            alliance_hq,
            empire_hq,
            death_star_active: false,
            death_star_location: None,
            resolved: false,
            conditions_word: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// The victory conditions word (`ghidra/notes/victory-conditions.md`)
// ---------------------------------------------------------------------------

/// Bit `0x1`: the Alliance headquarters is gone or destroyed (`FUN_005180e0`).
pub const WORD_HQ_LOST: u32 = 0x1;
/// Bit `0x10`: Luke captured (`FUN_005180e0`).
pub const WORD_LUKE_CAPTURED: u32 = 0x10;
/// Bit `0x20`: Mon Mothma captured (`FUN_005180e0`).
pub const WORD_MON_MOTHMA_CAPTURED: u32 = 0x20;
/// Bit `0x100`: Coruscant is the Alliance's (`FUN_005180e0`).
pub const WORD_CORUSCANT_ALLIANCE: u32 = 0x100;
/// Bit `0x200`: Coruscant destroyed (`FUN_005180e0`).
pub const WORD_CORUSCANT_DESTROYED: u32 = 0x200;
/// Bit `0x1000`: Vader captured (`FUN_005180e0`).
pub const WORD_VADER_CAPTURED: u32 = 0x1000;
/// Bit `0x2000`: the Emperor captured (`FUN_005180e0`).
pub const WORD_EMPEROR_CAPTURED: u32 = 0x2000;
/// Bit `0x100000`: the Empire wins (`FUN_005180e0`).
pub const WORD_EMPIRE_WINS: u32 = 0x10_0000;
/// Bit `0x1000000`: the Alliance wins (`FUN_005180e0`).
pub const WORD_ALLIANCE_WINS: u32 = 0x100_0000;
/// Bit `0x10000000`: the game is over (`FUN_005180e0`).
pub const WORD_GAME_OVER: u32 = 0x1000_0000;

/// Luke `0x32000242` (`FUN_005180e0`).
const LUKE: u32 = 0x242;
/// Mon Mothma `0x30000240` (`FUN_005180e0`).
const MON_MOTHMA: u32 = 0x240;
/// Vader `0x35000281` (`FUN_005180e0`).
const VADER: u32 = 0x281;
/// The Emperor `0x34000280` (`FUN_005180e0`).
const EMPEROR: u32 = 0x280;
/// Coruscant `0x90000109` (`FUN_005180e0`).
const CORUSCANT: u32 = 0x109;

/// `FUN_005180e0`: the word, or `None` while one of its characters or
/// Coruscant is missing. `old` is the stored word, which settles a day on
/// which both sides would win.
///
/// port: the headquarters (`FUN_00506110`) is the Alliance HQ system; it
/// is lost when the system is destroyed or no longer holds the HQ. A
/// captive is any captured character (`+0xac` bit 0), as in the original.
#[must_use]
pub fn conditions_word(
    state: &VictoryState,
    world: &GameWorld,
    victory_conditions: VictoryConditions,
) -> Option<u32> {
    let captured = |index: u32| {
        world
            .characters
            .values()
            .find(|c| c.dat_id.index() == index)
            .map(|c| c.is_captive)
    };
    let (luke, mon_mothma, vader, emperor) = (
        captured(LUKE)?,
        captured(MON_MOTHMA)?,
        captured(VADER)?,
        captured(EMPEROR)?,
    );
    let coruscant = world
        .systems
        .values()
        .find(|s| s.dat_id.index() == CORUSCANT)?;
    let hq_lost = world
        .systems
        .get(state.alliance_hq)
        .is_none_or(|sys| sys.is_destroyed || !sys.is_headquarters);
    let coruscant_alliance = coruscant.control.is_controlled_by(Faction::Alliance);
    let coruscant_destroyed = coruscant.is_destroyed;
    let coruscant_taken = coruscant_alliance || coruscant_destroyed;
    let (mut alliance_wins, mut empire_wins) = match victory_conditions {
        VictoryConditions::Standard => (
            vader && emperor && coruscant_taken,
            luke && mon_mothma && hq_lost,
        ),
        VictoryConditions::HeadquartersOnly => (coruscant_taken, hq_lost),
    };
    if alliance_wins && empire_wins {
        // The win already recorded stands; with none, the Alliance's.
        let old = state.conditions_word;
        let empire_first = old & WORD_ALLIANCE_WINS == 0 && old & WORD_EMPIRE_WINS != 0;
        alliance_wins = !empire_first;
        empire_wins = empire_first;
    }
    let bits = [
        (hq_lost, WORD_HQ_LOST),
        (luke, WORD_LUKE_CAPTURED),
        (mon_mothma, WORD_MON_MOTHMA_CAPTURED),
        (coruscant_alliance, WORD_CORUSCANT_ALLIANCE),
        (coruscant_destroyed, WORD_CORUSCANT_DESTROYED),
        (vader, WORD_VADER_CAPTURED),
        (emperor, WORD_EMPEROR_CAPTURED),
        (empire_wins, WORD_EMPIRE_WINS),
        (alliance_wins, WORD_ALLIANCE_WINS),
        (alliance_wins || empire_wins, WORD_GAME_OVER),
    ];
    Some(
        bits.iter()
            .filter(|(set, _)| *set)
            .fold(0, |word, (_, bit)| word | bit),
    )
}

/// The day's check of the word (`FUN_005180e0`, copied to both sides by
/// `FUN_00518460`). Returns `(old, new)` when it changed: notification
/// `0x128` (`FUN_00532f90`).
///
/// port: the original runs the check from its galaxy handlers; the port
/// runs it each day.
pub fn advance_conditions_word(
    state: &mut VictoryState,
    world: &GameWorld,
    victory_conditions: VictoryConditions,
) -> Option<(u32, u32)> {
    let new = conditions_word(state, world, victory_conditions)?;
    let old = state.conditions_word;
    if new == old {
        return None;
    }
    state.conditions_word = new;
    Some((old, new))
}

// ---------------------------------------------------------------------------
// VictorySystem
// ---------------------------------------------------------------------------

/// Stateless victory-condition evaluator.
pub struct VictorySystem;

impl VictorySystem {
    /// The game's end, read from the victory conditions word
    /// ([`conditions_word`], `FUN_005180e0`): the game is over once its bit
    /// `0x10000000` is set (`FUN_0051de80`), and the win bits name the
    /// winner. An Alliance win is Coruscant held or destroyed; an Empire
    /// win is the Alliance headquarters gone, by the Death Star when its
    /// system is destroyed.
    ///
    /// Returns `Some(VictoryOutcome)` the first tick the word ends the game;
    /// `None` otherwise. Skips frames without a simulation tick and
    /// already-resolved games. The caller must set `state.resolved = true`
    /// after acting on a result.
    ///
    /// port: the word is computed here, as not every caller runs
    /// [`advance_conditions_word`] first; the stored word only settles a day
    /// on which both would win.
    #[must_use]
    pub fn check(
        state: &VictoryState,
        world: &GameWorld,
        tick_events: &[TickEvent],
        victory_conditions: VictoryConditions,
    ) -> Option<VictoryOutcome> {
        if tick_events.is_empty() || state.resolved {
            return None;
        }
        let word = conditions_word(state, world, victory_conditions)?;
        if word & WORD_GAME_OVER == 0 {
            return None;
        }
        if word & WORD_ALLIANCE_WINS != 0 {
            return Some(VictoryOutcome::HqCaptured {
                winner: Faction::Alliance,
                loser: Faction::Empire,
                hq_system: state.empire_hq,
            });
        }
        let destroyed = world
            .systems
            .get(state.alliance_hq)
            .is_some_and(|sys| sys.is_destroyed);
        Some(if destroyed {
            VictoryOutcome::DeathStarVictory {
                target_system: state.alliance_hq,
            }
        } else {
            VictoryOutcome::HqDestroyed {
                winner: Faction::Empire,
                loser: Faction::Alliance,
                hq_system: state.alliance_hq,
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::ExplorationStatus;
    use crate::dat::SectorGroup;
    use crate::ids::DatId;
    use crate::tick::TickEvent;
    use crate::world::ControlKind;
    use crate::world::{Character, GameWorld, Sector, System};

    fn tick(n: u64) -> TickEvent {
        TickEvent { tick: n }
    }

    /// Build a minimal world with one sector and two systems.
    fn make_world() -> (GameWorld, SystemKey, SystemKey) {
        let mut world = GameWorld::default();

        let sector_key = world.sectors.insert(Sector {
            dat_id: DatId::new(0x9200_0000),
            name: "Test Sector".into(),
            group: SectorGroup::RimOuter,
            x: 0,
            y: 0,
            systems: vec![],
        });

        let a_hq = world.systems.insert(System {
            dat_id: DatId::new(0x9000_0000),
            name: "Yavin IV".into(),
            sector: sector_key,
            x: 100,
            y: 200,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.8,
            popularity_empire: 0.2,
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
            is_headquarters: true,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Alliance),
        });

        let e_hq = world.systems.insert(System {
            dat_id: DatId::new(0x9000_0001),
            name: "Coruscant".into(),
            sector: sector_key,
            x: 500,
            y: 500,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.1,
            popularity_empire: 0.9,
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
            is_headquarters: true,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Empire),
        });

        (world, a_hq, e_hq)
    }

    /// `make_world` with Coruscant as `0x90000109` and the four victory
    /// characters, free.
    fn word_world() -> (GameWorld, VictoryState, [crate::ids::CharacterKey; 4]) {
        let (mut world, a_hq, e_hq) = make_world();
        world.systems[e_hq].dat_id = DatId::new(CORUSCANT);
        let keys = [LUKE, MON_MOTHMA, VADER, EMPEROR].map(|index| {
            world.characters.insert(Character {
                dat_id: DatId::new(index),
                ..Character::default()
            })
        });
        (world, VictoryState::new(a_hq, e_hq), keys)
    }

    #[test]
    fn the_victory_conditions_word_carries_its_bits() {
        // FUN_005180e0: 0x1 HQ lost, 0x10 Luke, 0x20 Mon Mothma, 0x100
        // Coruscant the Alliance's, 0x200 destroyed, 0x1000 Vader, 0x2000
        // the Emperor.
        let (mut world, state, [luke, mon, vader, emperor]) = word_world();
        let word = |w: &GameWorld| conditions_word(&state, w, VictoryConditions::Standard);
        assert_eq!(word(&world), Some(0));
        let cases: [(
            fn(&mut GameWorld, &VictoryState, [crate::ids::CharacterKey; 4]),
            u32,
        ); 7] = [
            (
                |w, s, _| w.systems[s.alliance_hq].is_headquarters = false,
                WORD_HQ_LOST,
            ),
            (
                |w, _, k| w.characters[k[0]].is_captive = true,
                WORD_LUKE_CAPTURED,
            ),
            (
                |w, _, k| w.characters[k[1]].is_captive = true,
                WORD_MON_MOTHMA_CAPTURED,
            ),
            (
                |w, s, _| {
                    w.systems[s.empire_hq].control = ControlKind::Controlled(Faction::Alliance)
                },
                WORD_CORUSCANT_ALLIANCE,
            ),
            (
                |w, s, _| w.systems[s.empire_hq].is_destroyed = true,
                WORD_CORUSCANT_DESTROYED,
            ),
            (
                |w, _, k| w.characters[k[2]].is_captive = true,
                WORD_VADER_CAPTURED,
            ),
            (
                |w, _, k| w.characters[k[3]].is_captive = true,
                WORD_EMPEROR_CAPTURED,
            ),
        ];
        for (edit, bit) in cases {
            let mut w = world.clone();
            edit(&mut w, &state, [luke, mon, vader, emperor]);
            assert_eq!(word(&w), Some(bit), "{bit:#x}");
        }
        world.systems[state.alliance_hq].is_destroyed = true;
        assert_eq!(word(&world), Some(WORD_HQ_LOST));
        world.characters.remove(vader);
        assert_eq!(word(&world), None, "a missing character builds no word");
    }

    #[test]
    fn the_word_s_wins_follow_the_game_type() {
        // FUN_005180e0, galaxy +0x60: 0 needs the leaders captured, 1 only
        // the HQ or Coruscant.
        let (mut world, state, [luke, mon, vader, emperor]) = word_world();
        world.systems[state.alliance_hq].is_headquarters = false;
        let word = |w: &GameWorld, c| conditions_word(&state, w, c).unwrap();
        let over = WORD_EMPIRE_WINS | WORD_GAME_OVER;
        assert_eq!(
            word(&world, VictoryConditions::HeadquartersOnly) & over,
            over
        );
        assert_eq!(word(&world, VictoryConditions::Standard) & over, 0);
        world.characters[luke].is_captive = true;
        assert_eq!(word(&world, VictoryConditions::Standard) & over, 0);
        world.characters[mon].is_captive = true;
        assert_eq!(word(&world, VictoryConditions::Standard) & over, over);

        let (mut world, state, _) = word_world();
        world.systems[state.empire_hq].is_destroyed = true;
        let over = WORD_ALLIANCE_WINS | WORD_GAME_OVER;
        assert_eq!(
            conditions_word(&state, &world, VictoryConditions::HeadquartersOnly).unwrap() & over,
            over
        );
        world.characters[vader].is_captive = true;
        world.characters[emperor].is_captive = true;
        assert_eq!(
            conditions_word(&state, &world, VictoryConditions::Standard).unwrap() & over,
            over
        );
    }

    #[test]
    fn when_both_would_win_the_recorded_win_stands() {
        // FUN_005180e0: with both wins, 0x1000000 in the old word keeps the
        // Alliance's, else 0x100000 the Empire's, else the Alliance's.
        let (mut world, mut state, _) = word_world();
        world.systems[state.alliance_hq].is_headquarters = false;
        world.systems[state.empire_hq].is_destroyed = true;
        let wins = |state: &VictoryState| {
            conditions_word(state, &world, VictoryConditions::HeadquartersOnly).unwrap()
                & (WORD_ALLIANCE_WINS | WORD_EMPIRE_WINS)
        };
        assert_eq!(wins(&state), WORD_ALLIANCE_WINS);
        state.conditions_word = WORD_EMPIRE_WINS;
        assert_eq!(wins(&state), WORD_EMPIRE_WINS);
        state.conditions_word = WORD_EMPIRE_WINS | WORD_ALLIANCE_WINS;
        assert_eq!(wins(&state), WORD_ALLIANCE_WINS);
    }

    #[test]
    fn a_change_of_the_word_is_reported_once() {
        // FUN_00532f90 posts 0x128 on a change of +0xbc (old, new).
        let (mut world, mut state, [luke, ..]) = word_world();
        let advance = |state: &mut VictoryState, w: &GameWorld| {
            advance_conditions_word(state, w, VictoryConditions::Standard)
        };
        assert_eq!(advance(&mut state, &world), None);
        world.characters[luke].is_captive = true;
        assert_eq!(advance(&mut state, &world), Some((0, WORD_LUKE_CAPTURED)));
        assert_eq!(state.conditions_word, WORD_LUKE_CAPTURED);
        assert_eq!(advance(&mut state, &world), None);
    }

    #[test]
    fn no_outcome_without_ticks() {
        let (world, a, e) = make_world();
        let state = VictoryState::new(a, e);
        assert!(VictorySystem::check(&state, &world, &[], VictoryConditions::Standard).is_none());
    }

    #[test]
    fn no_outcome_when_resolved() {
        let (world, a, e) = make_world();
        let mut state = VictoryState::new(a, e);
        state.resolved = true;
        assert!(
            VictorySystem::check(&state, &world, &[tick(1)], VictoryConditions::Standard).is_none()
        );
    }

    /// A bombardment that destroyed the Alliance's headquarters object
    /// (ALLFACSD, `bombardment-order.md`), carried out.
    fn headquarters_bombarded(world: &mut GameWorld, alliance_hq: SystemKey) {
        let req = crate::bombardment::BombardRequest {
            system: alliance_hq,
            side: Faction::Empire,
            kind: crate::bombardment::GENERAL,
            fleets: Vec::new(),
            alliance_hq: Some(alliance_hq),
        };
        let outcome = crate::bombardment::BombardmentOutcome {
            destroyed: vec![crate::bombardment::Target::Headquarters],
            ..crate::bombardment::BombardmentOutcome::default()
        };
        crate::bombardment::apply(world, &req, &outcome);
    }

    #[test]
    fn a_bombardment_destroying_the_headquarters_clears_the_alliance_hq_facility() {
        let (mut world, alliance_hq, _) = make_world();
        headquarters_bombarded(&mut world, alliance_hq);
        assert!(!world.systems[alliance_hq].is_headquarters);
    }

    #[test]
    fn the_headquarters_game_ends_on_coruscant_held_or_destroyed_or_the_hq_lost() {
        // FUN_005180e0, game type 1: the Alliance wins with Coruscant held
        // or destroyed; the Empire with the headquarters gone or
        // destroyed, whoever holds its system; FUN_0051de80 reads the
        // game-over bit.
        let hq = VictoryConditions::HeadquartersOnly;
        let (mut world, state, _) = word_world();
        assert!(VictorySystem::check(&state, &world, &[tick(1)], hq).is_none());
        world.systems[state.empire_hq].control = ControlKind::Controlled(Faction::Alliance);
        assert!(matches!(
            VictorySystem::check(&state, &world, &[tick(1)], hq),
            Some(VictoryOutcome::HqCaptured {
                winner: Faction::Alliance,
                ..
            })
        ));

        let (mut world, state, _) = word_world();
        world.systems[state.empire_hq].is_destroyed = true;
        assert!(matches!(
            VictorySystem::check(&state, &world, &[tick(1)], hq),
            Some(VictoryOutcome::HqCaptured {
                winner: Faction::Alliance,
                ..
            })
        ));

        let (mut world, state, _) = word_world();
        headquarters_bombarded(&mut world, state.alliance_hq);
        assert!(matches!(
            VictorySystem::check(&state, &world, &[tick(1)], hq),
            Some(VictoryOutcome::HqDestroyed { winner: Faction::Empire, hq_system, .. })
                if hq_system == state.alliance_hq
        ));

        let (mut world, state, _) = word_world();
        world.systems[state.alliance_hq].is_destroyed = true;
        assert!(matches!(
            VictorySystem::check(&state, &world, &[tick(1)], hq),
            Some(VictoryOutcome::DeathStarVictory { .. })
        ));
    }

    #[test]
    fn the_standard_game_also_needs_the_opposing_leaders_captured() {
        // FUN_005180e0, game type 0: the Empire needs Luke and Mon Mothma
        // captured besides the headquarters; the Alliance Vader and the
        // Emperor besides Coruscant. Any capture counts (+0xac bit 0).
        let standard = VictoryConditions::Standard;
        let (mut world, state, [luke, mon, vader, emperor]) = word_world();
        world.systems[state.alliance_hq].is_headquarters = false;
        world.characters[luke].is_captive = true;
        assert!(VictorySystem::check(&state, &world, &[tick(1)], standard).is_none());
        world.characters[mon].is_captive = true;
        assert!(matches!(
            VictorySystem::check(&state, &world, &[tick(1)], standard),
            Some(VictoryOutcome::HqDestroyed {
                winner: Faction::Empire,
                ..
            })
        ));

        let (mut world, state, _) = word_world();
        world.systems[state.empire_hq].control = ControlKind::Controlled(Faction::Alliance);
        world.characters[vader].is_captive = true;
        assert!(VictorySystem::check(&state, &world, &[tick(1)], standard).is_none());
        world.characters[emperor].is_captive = true;
        assert!(matches!(
            VictorySystem::check(&state, &world, &[tick(1)], standard),
            Some(VictoryOutcome::HqCaptured {
                winner: Faction::Alliance,
                ..
            })
        ));
    }

    #[test]
    fn no_game_ends_without_its_four_characters_and_coruscant() {
        // FUN_005180e0 builds no word while one of them is missing.
        let (mut world, state, [luke, ..]) = word_world();
        world.systems[state.empire_hq].control = ControlKind::Controlled(Faction::Alliance);
        world.characters.remove(luke);
        assert!(VictorySystem::check(
            &state,
            &world,
            &[tick(1)],
            VictoryConditions::HeadquartersOnly
        )
        .is_none());
    }
}
