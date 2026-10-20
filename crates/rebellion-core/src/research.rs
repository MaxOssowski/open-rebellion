//! Research: each side's three research trees, which only research missions
//! advance (`ghidra/notes/mission-lifecycle.md`, "Research").
//!
//! A side holds, per tree, its level (side `+0x9c` ships, `+0xa4`
//! facilities, `+0xa0` troops), its research points (`+0x90`, `+0x98`,
//! `+0x94`), and a Done flag (`+0xac`, `+0xb4`, `+0xb0`). A research
//! mission's successes add points ([`ResearchState::add_points`]); the
//! side's points hook (`FUN_005326f0` and its siblings) then raises the
//! level while the points cover the next level's cost, and sets Done when
//! no class is left to research.
//!
//! A class is buildable once its tree's level reaches its research order
//! (`FUN_0052e580`).
//!
//! The side's yards also research on their own: timer `0x381`, armed when
//! the side comes into play (`FUN_00532350`) and repeating every GNPRTB 7169
//! (30) plus rand(0..=7170 (30)) days, adds to each tree's points the side's
//! usable, unsuspended yards of that tree's area in its systems
//! (`FUN_00530460`, `FUN_00528040`; [`ResearchSystem::advance_tally`]).

use serde::{Deserialize, Serialize};

use crate::build_selection::research_classes;
use crate::ids::{FighterKey, SystemKey};
use crate::manufacturing::ProductionArea;
use crate::world::{GameWorld, Skill};

/// GNPRTB 7169 (`DAT_006bb510`) = 30: timer `0x381`'s minimum
/// (`FUN_00532350`, side record `+0xd4`).
const GNPRTB_TALLY_MIN: u16 = 7169;
/// GNPRTB 7170 (`DAT_006bb508`) = 30: timer `0x381`'s spread.
const GNPRTB_TALLY_SPREAD: u16 = 7170;

/// `DAT_00661a88`: the divisor of a level's cost (`FUN_0053e190`).
const COST_DIVISOR: u32 = 100;

// ---------------------------------------------------------------------------
// TechType
// ---------------------------------------------------------------------------

/// The three research trees, the research record's type (`FUN_0056c460`:
/// `0x53000020` ships 0, `0x53000021` facilities 1, `0x53000022` troops 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TechType {
    /// Ship design: capital ships and fighters (family `0x28`).
    Ship,
    /// Troop training: regiments and special forces (family `0x29`).
    Troop,
    /// Facility design: facilities (family `0x2a`).
    Facility,
}

impl TechType {
    pub const ALL: [TechType; 3] = [TechType::Ship, TechType::Troop, TechType::Facility];

    /// The skill a member researches with (`FUN_0056c820`: type 0 slot
    /// `+0x1e4` ship design, 1 `+0x1ec` facility design, 2 `+0x1e8` troop
    /// training).
    #[must_use]
    pub const fn skill(self) -> Skill {
        match self {
            TechType::Ship => Skill::ShipDesign,
            TechType::Troop => Skill::TroopTraining,
            TechType::Facility => Skill::FacilityDesign,
        }
    }

    /// The production area whose classes the tree unlocks, and which the
    /// research mission's target must run (`FUN_0056c510`).
    #[must_use]
    pub const fn area(self) -> ProductionArea {
        match self {
            TechType::Ship => ProductionArea::Shipyard,
            TechType::Troop => ProductionArea::TrainingFacility,
            TechType::Facility => ProductionArea::ConstructionYard,
        }
    }
}

// ---------------------------------------------------------------------------
// Per-side values
// ---------------------------------------------------------------------------

/// One side's level in each tree.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResearchLevels {
    pub ship: u32,
    pub troop: u32,
    pub facility: u32,
}

impl ResearchLevels {
    /// The level of one tree.
    #[must_use]
    pub fn level(&self, tech: TechType) -> u32 {
        match tech {
            TechType::Ship => self.ship,
            TechType::Troop => self.troop,
            TechType::Facility => self.facility,
        }
    }

    fn level_mut(&mut self, tech: TechType) -> &mut u32 {
        match tech {
            TechType::Ship => &mut self.ship,
            TechType::Troop => &mut self.troop,
            TechType::Facility => &mut self.facility,
        }
    }
}

/// One tree's research points and Done flag for a side.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TreeProgress {
    /// Points toward the next level.
    pub points: u32,
    /// No class is left to research (`FUN_0052f440`); research missions in
    /// this tree end with code `0xe` (`FUN_0052eb10`).
    pub done: bool,
}

/// One side's progress in each tree.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResearchProgress {
    pub ship: TreeProgress,
    pub troop: TreeProgress,
    pub facility: TreeProgress,
}

impl ResearchProgress {
    /// The progress of one tree.
    #[must_use]
    pub fn tree(&self, tech: TechType) -> TreeProgress {
        match tech {
            TechType::Ship => self.ship,
            TechType::Troop => self.troop,
            TechType::Facility => self.facility,
        }
    }

    fn tree_mut(&mut self, tech: TechType) -> &mut TreeProgress {
        match tech {
            TechType::Ship => &mut self.ship,
            TechType::Troop => &mut self.troop,
            TechType::Facility => &mut self.facility,
        }
    }
}

// ---------------------------------------------------------------------------
// ResearchState
// ---------------------------------------------------------------------------

/// Both sides' research.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResearchState {
    /// The Alliance's levels.
    pub alliance: ResearchLevels,
    /// The Empire's levels.
    pub empire: ResearchLevels,
    /// The Alliance's points and Done flags.
    pub alliance_progress: ResearchProgress,
    /// The Empire's points and Done flags.
    pub empire_progress: ResearchProgress,
    /// The tick the Alliance's timer `0x381` next fires; drawn on the first
    /// advance (v65).
    pub alliance_tally: Option<u64>,
    /// The same for the Empire.
    pub empire_tally: Option<u64>,
}

impl ResearchState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A side's level in one tree.
    #[must_use]
    pub fn level(&self, faction_is_alliance: bool, tech: TechType) -> u32 {
        if faction_is_alliance {
            self.alliance.level(tech)
        } else {
            self.empire.level(tech)
        }
    }

    /// A side's points and Done flag in one tree.
    #[must_use]
    pub fn progress(&self, faction_is_alliance: bool, tech: TechType) -> TreeProgress {
        if faction_is_alliance {
            self.alliance_progress.tree(tech)
        } else {
            self.empire_progress.tree(tech)
        }
    }

    /// True if `class_research_order <= current level` — i.e. the class is
    /// buildable by this faction: `FUN_00567880` (from `FUN_0052e580`)
    /// lists a class whose order is below or equal to the side's level and
    /// skips one above it. Every level starts at 0 (`FUN_00533300`).
    #[must_use]
    pub fn is_unlocked(
        &self,
        faction_is_alliance: bool,
        class_research_order: u32,
        tech: TechType,
    ) -> bool {
        self.level(faction_is_alliance, tech) >= class_research_order
    }

    /// Add a research mission's points to a side's tree (`FUN_0056cf30` ->
    /// `FUN_00530600`), then run the side's points hook (`FUN_005326f0` for
    /// ships, its siblings for the other trees) until it stops: while a next
    /// level exists ([`next_level`]) and the points cover its cost
    /// ([`level_cost`]), the level rises to it and the cost is spent
    /// (`FUN_0052f280`, `FUN_0052f130`, which runs the hook again); with no
    /// next level the tree is Done (`FUN_0052f440`). Returns each level
    /// reached, in order.
    pub fn add_points(
        &mut self,
        world: &GameWorld,
        faction_is_alliance: bool,
        tech: TechType,
        points: u32,
    ) -> Vec<u32> {
        let (levels, progress) = if faction_is_alliance {
            (&mut self.alliance, &mut self.alliance_progress)
        } else {
            (&mut self.empire, &mut self.empire_progress)
        };
        let level = levels.level_mut(tech);
        let tree = progress.tree_mut(tech);
        tree.points = tree.points.saturating_add(points);
        let mut reached = Vec::new();
        loop {
            let Some(next) = next_level(world, faction_is_alliance, tech, *level) else {
                tree.done = true;
                break;
            };
            let cost = level_cost(world, next.difficulty);
            if tree.points < cost {
                break;
            }
            *level = next.order;
            tree.points -= cost;
            reached.push(next.order);
        }
        reached
    }
}

/// The next level a side may research in a tree, and its difficulty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextLevel {
    pub order: u32,
    pub difficulty: u32,
}

/// `FUN_00567ba0`: the least research order above `current` that some class
/// of the tree's family holds for the side ([`research_classes`]), with the
/// difficulty (`+0x54`) of the first class at that order (`FUN_00567a90`).
/// `None` past the family's highest order.
#[must_use]
pub fn next_level(
    world: &GameWorld,
    faction_is_alliance: bool,
    tech: TechType,
    current: u32,
) -> Option<NextLevel> {
    research_classes(world, tech.area(), faction_is_alliance)
        .into_iter()
        .filter(|class| class.order > current)
        .min_by_key(|class| class.order)
        .map(|class| NextLevel {
            order: class.order,
            difficulty: class.difficulty,
        })
}

/// A level's cost in points: the class difficulty times the side's `+0xc0`
/// over 100 (`FUN_0055bf80` -> `FUN_0053e170`, an integer division).
/// `+0xc0` is set by `FUN_00519d00` through `FUN_005305c0`. hyp: it holds the
/// galaxy's system count.
#[must_use]
pub fn level_cost(world: &GameWorld, difficulty: u32) -> u32 {
    let systems = u32::try_from(world.systems.len()).unwrap_or(u32::MAX);
    difficulty.saturating_mul(systems) / COST_DIVISOR
}

// ---------------------------------------------------------------------------
// ResearchSystem
// ---------------------------------------------------------------------------

/// One tree's share of a fired timer `0x381`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchTally {
    pub faction_is_alliance: bool,
    pub tech: TechType,
    /// The yards counted.
    pub points: u32,
    /// Each level reached.
    pub reached: Vec<u32>,
}

/// Class availability by research level, and the yards' own research.
pub struct ResearchSystem;

impl ResearchSystem {
    /// `FUN_00528040` over the side's systems (`FUN_00530460`): the side's
    /// yards of `tech`'s area in the systems it holds (side bits `+0x24`),
    /// usable and not suspended (`+0x60` bit 0, which `FUN_0050bbb0` sets
    /// in a system in uprising or blockaded: `blocked`).
    ///
    /// port: every yard in the world is usable, and a system always has
    /// its three managers (`FUN_00530460` adds nothing when one lacks any).
    #[must_use]
    pub fn yard_points(
        world: &GameWorld,
        faction_is_alliance: bool,
        tech: TechType,
        blocked: impl Fn(SystemKey) -> bool,
    ) -> u32 {
        let side = crate::dat::Faction::of_alliance(faction_is_alliance);
        let family = crate::build_selection::area_family(tech.area());
        let mut points = 0;
        for (key, system) in &world.systems {
            if system.control.faction() != Some(side) || blocked(key) {
                continue;
            }
            for yard in system
                .manufacturing_facilities
                .iter()
                .filter_map(|yard| world.manufacturing_facilities.get(*yard))
            {
                if yard.side == side && yard.class_dat_id.family() == family {
                    points += 1;
                }
            }
        }
        points
    }

    /// Fire each side's timer `0x381` that is due by the last tick: add its
    /// yards to each tree's points (`FUN_0052f130` ships `+0x90`,
    /// `FUN_0052f1a0` troops `+0x94`, `FUN_0052f210` facilities `+0x98`) and
    /// run the points hook ([`ResearchState::add_points`]). A tree that
    /// gains nothing is unchanged, so its hook does not run. The timer
    /// reschedules with a fresh delay (`FUN_005862a0`); each delay takes the
    /// next of `rolls`.
    pub fn advance_tally(
        state: &mut ResearchState,
        world: &GameWorld,
        blocked: impl Fn(SystemKey) -> bool,
        tick_events: &[crate::tick::TickEvent],
        rolls: &[f64],
    ) -> Vec<ResearchTally> {
        let Some(last) = tick_events.last() else {
            return Vec::new();
        };
        let tick = last.tick;
        let min = world.gnprtb.value(GNPRTB_TALLY_MIN, world.difficulty_index);
        let spread = world
            .gnprtb
            .value(GNPRTB_TALLY_SPREAD, world.difficulty_index);
        let mut rolls = rolls.iter().copied();
        let mut delay =
            || crate::uprising::one_roll_timer_delay(min, spread, rolls.next().unwrap_or(0.0));
        let mut tallies = Vec::new();
        for faction_is_alliance in [true, false] {
            let timer = if faction_is_alliance {
                &mut state.alliance_tally
            } else {
                &mut state.empire_tally
            };
            let due = *timer.get_or_insert_with(|| tick + delay());
            if due > tick {
                continue;
            }
            *timer = Some(due + delay());
            for tech in [TechType::Ship, TechType::Troop, TechType::Facility] {
                let points = Self::yard_points(world, faction_is_alliance, tech, &blocked);
                if points == 0 {
                    continue;
                }
                let reached = state.add_points(world, faction_is_alliance, tech, points);
                tallies.push(ResearchTally {
                    faction_is_alliance,
                    tech,
                    points,
                    reached,
                });
            }
        }
        tallies
    }

    /// True if a capital ship class is available for this faction at the given
    /// research level.
    #[must_use]
    pub fn ship_class_is_available(
        world: &GameWorld,
        state: &ResearchState,
        faction_is_alliance: bool,
        class_key: crate::ids::CapitalShipKey,
    ) -> bool {
        let Some(class) = world.capital_ship_classes.get(class_key) else {
            return false;
        };
        let faction_ok = if faction_is_alliance {
            class.is_alliance
        } else {
            class.is_empire
        };
        if !faction_ok {
            return false;
        }
        state.is_unlocked(faction_is_alliance, class.research_order, TechType::Ship)
    }

    /// True if a fighter class is available at the current Ship research level.
    #[must_use]
    pub fn fighter_class_is_available(
        world: &GameWorld,
        state: &ResearchState,
        faction_is_alliance: bool,
        class_key: FighterKey,
    ) -> bool {
        let Some(class) = world.fighter_classes.get(class_key) else {
            return false;
        };
        let faction_ok = if faction_is_alliance {
            class.is_alliance
        } else {
            class.is_empire
        };
        if !faction_ok {
            return false;
        }
        // FIGHTSD.DAT carries its own research_order; fighters come from
        // shipyards, so they advance with the Ship tree (SideShipyardRdOrder).
        state.is_unlocked(faction_is_alliance, class.research_order, TechType::Ship)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DatId;
    use crate::world::{BuildableClass, CapitalShipClass, GameWorld, System};

    fn add_ship_class(
        world: &mut GameWorld,
        research_order: u32,
        research_difficulty: u32,
        is_alliance: bool,
    ) {
        let id = 0x1400_0001 + u32::try_from(world.capital_ship_classes.len()).unwrap();
        world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(id),
            name: "TestShip".into(),
            is_alliance,
            is_empire: !is_alliance,
            refined_material_cost: 100,
            maintenance_cost: 10,
            research_order,
            research_difficulty,
            ..CapitalShipClass::default()
        });
    }

    /// A galaxy of `n` systems, which scales every level's cost.
    fn galaxy(n: usize) -> GameWorld {
        let mut world = GameWorld::default();
        for _ in 0..n {
            world.systems.insert(System {
                dat_id: DatId::new(0x9000_0001),
                name: String::new(),
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
                control: crate::world::ControlKind::Uncontrolled,
            });
        }
        world
    }

    #[test]
    fn a_level_costs_its_class_difficulty_scaled_by_the_galaxy() {
        // FUN_0055bf80 -> FUN_0053e170: difficulty * side +0xc0 / 100.
        let world = galaxy(200);
        assert_eq!(level_cost(&world, 35), 70);
        let world = galaxy(150);
        assert_eq!(level_cost(&world, 35), 52);
    }

    #[test]
    fn the_next_level_is_the_least_order_above_the_current_one() {
        // FUN_00567ba0 walks the orders upward from current + 1 and takes the
        // first that FUN_00567a90 finds a class at, and that class's
        // difficulty: the first in id order (hyp, see research_classes).
        let mut world = galaxy(100);
        add_ship_class(&mut world, 4, 40, true);
        add_ship_class(&mut world, 2, 20, true);
        add_ship_class(&mut world, 2, 25, true);
        add_ship_class(&mut world, 3, 99, false);
        assert_eq!(
            next_level(&world, true, TechType::Ship, 0),
            Some(NextLevel {
                order: 2,
                difficulty: 20,
            })
        );
        assert_eq!(
            next_level(&world, true, TechType::Ship, 2),
            Some(NextLevel {
                order: 4,
                difficulty: 40,
            })
        );
        assert_eq!(next_level(&world, true, TechType::Ship, 4), None);
    }

    #[test]
    fn a_class_without_a_refined_cost_is_not_researched() {
        // FUN_00567880 keeps a class only when FUN_0053b860 is nonzero.
        let mut world = galaxy(100);
        world.buildable_classes.insert(
            DatId::new(0x1000_0001),
            BuildableClass {
                is_alliance: true,
                refined_material_cost: 0,
                research_order: 3,
                ..BuildableClass::default()
            },
        );
        assert_eq!(next_level(&world, true, TechType::Troop, 0), None);
        world
            .buildable_classes
            .get_mut(&DatId::new(0x1000_0001))
            .unwrap()
            .refined_material_cost = 5;
        assert_eq!(
            next_level(&world, true, TechType::Troop, 0).map(|next| next.order),
            Some(3)
        );
        assert_eq!(next_level(&world, true, TechType::Facility, 0), None);
    }

    #[test]
    fn points_raise_the_level_while_they_cover_the_cost() {
        // FUN_005326f0: points >= cost takes the next level and spends the
        // cost (FUN_0052f280, FUN_0052f130), whose hook runs again.
        let mut world = galaxy(100);
        add_ship_class(&mut world, 1, 10, true);
        add_ship_class(&mut world, 2, 15, true);
        add_ship_class(&mut world, 3, 50, true);
        let mut state = ResearchState::new();

        assert!(state.add_points(&world, true, TechType::Ship, 9).is_empty());
        assert_eq!(state.alliance.ship, 0);
        assert_eq!(state.progress(true, TechType::Ship).points, 9);

        assert_eq!(
            state.add_points(&world, true, TechType::Ship, 20),
            vec![1, 2]
        );
        assert_eq!(state.alliance.ship, 2);
        assert_eq!(state.progress(true, TechType::Ship).points, 4);
        assert!(!state.progress(true, TechType::Ship).done);
        assert_eq!(state.empire.ship, 0);
        assert_eq!(state.progress(false, TechType::Ship).points, 0);
    }

    #[test]
    fn points_equal_to_the_cost_reach_the_level() {
        // FUN_005326f0: the level rises when cost <= points.
        let mut world = galaxy(100);
        add_ship_class(&mut world, 1, 10, true);
        add_ship_class(&mut world, 2, 50, true);
        let mut state = ResearchState::new();
        assert_eq!(state.add_points(&world, true, TechType::Ship, 10), vec![1]);
        assert_eq!(state.progress(true, TechType::Ship).points, 0);
    }

    #[test]
    fn a_fighter_class_counts_in_the_ship_tree_for_its_side_with_a_cost() {
        // FUN_00567880 walks every class table of family 0x28, fighters
        // included, keeping a side's classes with a refined cost.
        let mut world = galaxy(100);
        let fighter = |id, is_alliance, cost, order| crate::world::FighterClass {
            dat_id: DatId::new(id),
            is_alliance,
            is_empire: !is_alliance,
            refined_material_cost: cost,
            research_order: order,
            research_difficulty: 30,
            ..Default::default()
        };
        world
            .fighter_classes
            .insert(fighter(0x1c00_0001, true, 0, 1));
        world
            .fighter_classes
            .insert(fighter(0x1c00_0002, false, 5, 2));
        assert_eq!(next_level(&world, true, TechType::Ship, 0), None);
        world
            .fighter_classes
            .insert(fighter(0x1c00_0003, true, 5, 3));
        assert_eq!(
            next_level(&world, true, TechType::Ship, 0),
            Some(NextLevel {
                order: 3,
                difficulty: 30,
            })
        );
    }

    #[test]
    fn the_last_level_marks_the_tree_done() {
        // FUN_005326f0: no next level (FUN_00567ba0's found flag clear) sets
        // the Done flag through FUN_0052f440(1).
        let mut world = galaxy(100);
        add_ship_class(&mut world, 1, 10, true);
        let mut state = ResearchState::new();
        assert_eq!(state.add_points(&world, true, TechType::Ship, 12), vec![1]);
        let tree = state.progress(true, TechType::Ship);
        assert!(tree.done);
        assert_eq!(tree.points, 2);
        assert!(!state.progress(true, TechType::Troop).done);
    }

    #[test]
    fn is_unlocked_respects_current_level() {
        let mut state = ResearchState::new();
        assert!(state.is_unlocked(true, 0, TechType::Ship));
        assert!(!state.is_unlocked(true, 1, TechType::Ship));
        state.alliance.ship = 1;
        assert!(state.is_unlocked(true, 1, TechType::Ship));
        assert!(!state.is_unlocked(true, 2, TechType::Ship));
    }

    #[test]
    fn ship_class_availability_respects_research_order() {
        let mut world = GameWorld::default();
        add_ship_class(&mut world, 3, 20, true); // needs level 3

        let class_key = world.capital_ship_classes.keys().next().unwrap();
        let mut state = ResearchState::new(); // level 0

        assert!(!ResearchSystem::ship_class_is_available(
            &world, &state, true, class_key
        ));

        state.alliance.ship = 3;
        assert!(ResearchSystem::ship_class_is_available(
            &world, &state, true, class_key
        ));
    }

    #[test]
    fn a_fighter_unlocks_when_ship_research_reaches_its_order() {
        // FIGHTSD.DAT: the B-wing carries research_order 5.
        let mut world = GameWorld::default();
        let class_key = world.fighter_classes.insert(crate::world::FighterClass {
            is_alliance: true,
            research_order: 5,
            ..Default::default()
        });
        let mut state = ResearchState::new();
        state.alliance.ship = 4;
        state.alliance.troop = 5;
        state.alliance.facility = 5;

        assert!(!ResearchSystem::fighter_class_is_available(
            &world, &state, true, class_key
        ));
        assert!(!ResearchSystem::fighter_class_is_available(
            &world, &state, false, class_key
        ));

        state.alliance.ship = 5;
        assert!(ResearchSystem::fighter_class_is_available(
            &world, &state, true, class_key
        ));
    }

    /// Four systems: the Alliance's with two shipyards, a training
    /// facility and an Imperial shipyard; the Alliance's in revolt with a
    /// shipyard; the Empire's with a shipyard; a neutral one with an
    /// Alliance shipyard. GNPRTB 7169 = 30, 7170 = 30.
    fn yards_world() -> (GameWorld, Vec<SystemKey>) {
        use crate::dat::Faction::{Alliance, Empire};
        use crate::world::{ControlKind, GnprtbEntry, GnprtbParams, ManufacturingFacilityInstance};
        let mut world = galaxy(4);
        world.gnprtb = GnprtbParams::new(
            [(7169, 30), (7170, 30)]
                .into_iter()
                .map(|(parameter_id, value)| GnprtbEntry {
                    parameter_id,
                    development: value,
                    alliance_sp_easy: value,
                    alliance_sp_medium: value,
                    alliance_sp_hard: value,
                    empire_sp_easy: value,
                    empire_sp_medium: value,
                    empire_sp_hard: value,
                    multiplayer: value,
                })
                .collect(),
        );
        let systems: Vec<SystemKey> = world.systems.keys().collect();
        let holders = [
            ControlKind::Controlled(Alliance),
            ControlKind::Controlled(Alliance),
            ControlKind::Controlled(Empire),
            ControlKind::Uncontrolled,
        ];
        for (system, holder) in systems.iter().zip(holders) {
            world.systems[*system].control = holder;
        }
        let yards = [
            (0, 0x2800_0001, Alliance),
            (0, 0x2800_0001, Alliance),
            (0, 0x2900_0001, Alliance),
            (0, 0x2800_0001, Empire),
            (1, 0x2800_0001, Alliance),
            (2, 0x2800_0001, Empire),
            (3, 0x2800_0001, Alliance),
        ];
        for (at, class, side) in yards {
            let key = world
                .manufacturing_facilities
                .insert(ManufacturingFacilityInstance {
                    class_dat_id: DatId::new(class),
                    side,
                    is_shipyard: class >> 24 == 0x28,
                });
            world.systems[systems[at]]
                .manufacturing_facilities
                .push(key);
        }
        (world, systems)
    }

    #[test]
    fn a_side_s_yards_count_in_the_systems_it_holds_unless_suspended() {
        // FUN_00530460 walks the side's systems (side bits +0x24) and
        // FUN_00528040 counts each manager's yards that FUN_0052bde0 accepts
        // (its area, usable, the side's) and that are not suspended (+0x60
        // bit 0, set in uprising or blockade by FUN_0050bbb0).
        let (world, systems) = yards_world();
        let blocked = |system| system == systems[1];
        let points = |alliance, tech| ResearchSystem::yard_points(&world, alliance, tech, blocked);
        assert_eq!(points(true, TechType::Ship), 2);
        assert_eq!(points(true, TechType::Troop), 1);
        assert_eq!(points(true, TechType::Facility), 0);
        assert_eq!(points(false, TechType::Ship), 1);
        assert_eq!(
            ResearchSystem::yard_points(&world, true, TechType::Ship, |_| false),
            3
        );
    }

    #[test]
    fn the_yards_research_every_thirty_to_sixty_days() {
        // FUN_00532350 arms timer 0x381 with record +0xd4: GNPRTB 7169 (30)
        // + rand(0..=7170 (30)) (FUN_00586130), drawn again on each fire.
        // Ship class order 1 at difficulty 50 costs 50 * 4 / 100 = 2.
        let (mut world, systems) = yards_world();
        add_ship_class(&mut world, 1, 50, true);
        let mut state = ResearchState::new();
        let tick = |state: &mut ResearchState, tick, roll| {
            ResearchSystem::advance_tally(
                state,
                &world,
                |system| system == systems[1],
                &[crate::tick::TickEvent { tick }],
                &[roll, roll],
            )
        };
        assert_eq!(tick(&mut state, 1, 0.0), vec![]);
        assert_eq!(state.alliance_tally, Some(31));
        for day in 2..=30 {
            assert_eq!(tick(&mut state, day, 0.0), vec![]);
        }
        let fired = tick(&mut state, 31, 0.999_999);
        assert_eq!(
            fired,
            vec![
                ResearchTally {
                    faction_is_alliance: true,
                    tech: TechType::Ship,
                    points: 2,
                    reached: vec![1],
                },
                ResearchTally {
                    faction_is_alliance: true,
                    tech: TechType::Troop,
                    points: 1,
                    reached: vec![],
                },
                ResearchTally {
                    faction_is_alliance: false,
                    tech: TechType::Ship,
                    points: 1,
                    reached: vec![],
                },
            ]
        );
        assert_eq!(state.level(true, TechType::Ship), 1);
        assert_eq!(state.progress(true, TechType::Troop).points, 1);
        assert_eq!(state.alliance_tally, Some(91));
    }
}
