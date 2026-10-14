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

use serde::{Deserialize, Serialize};

use crate::build_selection::research_classes;
use crate::ids::FighterKey;
use crate::manufacturing::ProductionArea;
use crate::world::{GameWorld, Skill};

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
    /// buildable by this faction.
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

/// Class availability by research level.
pub struct ResearchSystem;

impl ResearchSystem {
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
}
