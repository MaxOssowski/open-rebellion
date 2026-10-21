//! The Assault Summary report (`ghidra/notes/assault-summary-window.md`):
//! the message class with vtable `0x0065b870` that collects the system's
//! and the attacking fleets' units, the destroyed and the captured, and the
//! outcome the Assault Summary window (`FUN_0044c410`) shows.
//!
//! [`AssaultReport::start`] takes the forces before the assault (the
//! notifications `0x150` and `0x182`); [`AssaultReport::finish`] files the
//! losses (`0x302`) and the outcome (notification 2, `FUN_00496770`).

use crate::assault::AssaultOutcome;
use crate::bombardment::Target;
use crate::dat::Faction;
use crate::ids::{DatId, FleetKey, SystemKey};
use crate::troop_transport::TroopTransportState;
use crate::world::{ControlKind, GameWorld};

/// The Assault Summary's unit categories (`FUN_0044e500`), in the order of
/// their buttons (`0x97..0x9c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportCategory {
    /// `0x97`, "Capital Ships" `0x1928`: families `0x14..0x1b`.
    CapitalShips,
    /// `0x98`, "Fighter Squadrons" `0x1929`: families `0x1c..0x1f`.
    Fighters,
    /// `0x99`, "Manufacturing" `0x1835`: families `0x28..0x2f`.
    Manufacturing,
    /// `0x9a`, "Defense Facilities" `0x1946`: families `0x22..0x27`.
    Defense,
    /// `0x9b`, "Trooper Regiments" `0x1930`: families `0x10..0x13`.
    Troops,
    /// `0x9c`, "Personnel" `0x1931`: families `0x30..0x3f`.
    Personnel,
}

impl ReportCategory {
    /// The buttons left to right.
    pub const ALL: [Self; 6] = [
        Self::CapitalShips,
        Self::Fighters,
        Self::Manufacturing,
        Self::Defense,
        Self::Troops,
        Self::Personnel,
    ];

    /// The category of an object family (`FUN_0044e500`'s ranges), if the
    /// window lists it.
    #[must_use]
    pub const fn of_family(family: u8) -> Option<Self> {
        match family {
            0x14..=0x1b => Some(Self::CapitalShips),
            0x1c..=0x1f => Some(Self::Fighters),
            0x28..=0x2f => Some(Self::Manufacturing),
            0x22..=0x27 => Some(Self::Defense),
            0x10..=0x13 => Some(Self::Troops),
            0x30..=0x3f => Some(Self::Personnel),
            _ => None,
        }
    }
}

/// One unit the report lists: its id (the class's, or the character's),
/// for the row's picture, and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportUnit {
    pub category: ReportCategory,
    pub dat_id: DatId,
    pub name: String,
    /// A major character (its picture's table, `pictures.rs`).
    pub major: bool,
    /// The ground object an assault can destroy; `None` for ships,
    /// squadrons and personnel.
    target: Option<Target>,
}

impl ReportUnit {
    /// A unit no assault destroys (a craft or personnel).
    #[must_use]
    pub fn new(category: ReportCategory, dat_id: DatId, name: String, major: bool) -> Self {
        Self {
            category,
            dat_id,
            name,
            major,
            target: None,
        }
    }
}

/// One side's columns: Operational (Survivors), Captured, Destroyed
/// (Killed).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportForces {
    pub operational: Vec<ReportUnit>,
    pub captured: Vec<ReportUnit>,
    pub destroyed: Vec<ReportUnit>,
}

impl ReportForces {
    /// File the units `lost` names under Destroyed.
    fn file_losses(&mut self, lost: impl Fn(Target) -> bool) {
        let (destroyed, operational) = std::mem::take(&mut self.operational)
            .into_iter()
            .partition(|unit| unit.target.is_some_and(&lost));
        self.operational = operational;
        self.destroyed = destroyed;
    }

    /// The units of `category` in each column.
    #[must_use]
    pub fn columns(&self, category: ReportCategory) -> [Vec<&ReportUnit>; 3] {
        fn of(units: &[ReportUnit], category: ReportCategory) -> Vec<&ReportUnit> {
            units
                .iter()
                .filter(|unit| unit.category == category)
                .collect()
        }
        [
            of(&self.operational, category),
            of(&self.captured, category),
            of(&self.destroyed, category),
        ]
    }
}

/// What an Assault Summary reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssaultReport {
    /// `+0x5c`.
    pub system: SystemKey,
    /// The attacking side (`+0x6c`).
    pub attacker: Faction,
    /// The system's side before the assault (`+0x68`); `None` neutral.
    pub defender: Option<Faction>,
    /// Whether the system is the attacker's afterwards (notification 2
    /// reads the system's side bits).
    pub taken: bool,
    /// `+0x78`, `+0x90`, `+0x88`.
    pub attackers: ReportForces,
    /// `+0x70`, `+0x98`, `+0x80`.
    pub system_forces: ReportForces,
}

/// TEXTSTRA RCDATA `0x7090` "Assault on {1:1}", the title `+0x44`
/// (`FUN_00496770`).
pub const TITLE: u16 = 0x7090;
/// STRATEGY `0x2b98` / `0x2b99`: the Alliance's / the Empire's assault
/// picture (`+0x2e`, `FUN_00496770`).
const PICTURES: [u32; 2] = [0x2b98, 0x2b99];

impl AssaultReport {
    /// The forces before `side`'s `fleets` assault `system`: the system's
    /// units (`0x150`, `FUN_00496f50`) and the fleets' ships with their
    /// squadrons, regiments and personnel (`0x182`, `FUN_00496770`).
    ///
    /// port: the system's units are its regiments, special forces,
    /// facilities, squadrons and the characters on its planet; the
    /// fleets' cargo is their regiments (the port keeps them per fleet).
    #[must_use]
    pub fn start(
        world: &GameWorld,
        transport: &TroopTransportState,
        side: Faction,
        fleets: &[FleetKey],
        system: SystemKey,
    ) -> Option<Self> {
        let sys = world.systems.get(system)?;
        let mut attackers = Vec::new();
        for &key in fleets {
            let Some(fleet) = world.fleets.get(key) else {
                continue;
            };
            for (index, ship) in fleet.capital_ships.iter().enumerate() {
                if let Some(class) = world.capital_ship_classes.get(ship.class) {
                    let name = world.ship_name(key, index).unwrap_or(&class.name);
                    push(&mut attackers, class.dat_id, name.to_owned(), None);
                }
            }
            for entry in &fleet.fighters {
                squadrons(world, entry, &mut attackers);
            }
            for &troop in transport.cargo(key) {
                troop_unit(world, troop, &mut attackers);
            }
            for &character in &fleet.characters {
                if let Some(value) = world.characters.get(character) {
                    push_character(&mut attackers, value);
                }
            }
        }

        let mut forces = Vec::new();
        for &troop in &sys.ground_units {
            troop_unit(world, troop, &mut forces);
        }
        for &key in &sys.special_forces {
            if let Some(force) = world.special_forces.get(key) {
                let name = class_name(world, force.class_dat_id);
                push(&mut forces, force.class_dat_id, name, None);
            }
        }
        for &key in &sys.defense_facilities {
            if let Some(facility) = world.defense_facilities.get(key) {
                let name = class_name(world, facility.class_dat_id);
                push(
                    &mut forces,
                    facility.class_dat_id,
                    name,
                    Some(Target::Defense(key)),
                );
            }
        }
        for &key in &sys.manufacturing_facilities {
            if let Some(facility) = world.manufacturing_facilities.get(key) {
                let name = class_name(world, facility.class_dat_id);
                let target = Some(Target::Manufacturing(key));
                push(&mut forces, facility.class_dat_id, name, target);
            }
        }
        for &key in &sys.production_facilities {
            if let Some(facility) = world.production_facilities.get(key) {
                let name = class_name(world, facility.class_dat_id);
                let target = Some(Target::Production(key));
                push(&mut forces, facility.class_dat_id, name, target);
            }
        }
        for entry in &sys.fighters {
            squadrons(world, entry, &mut forces);
        }
        let aboard = |key| {
            world
                .fleets
                .values()
                .any(|fleet| fleet.characters.contains(&key))
        };
        for (key, character) in &world.characters {
            if !character.is_killed && character.current_system == Some(system) && !aboard(key) {
                push_character(&mut forces, character);
            }
        }

        Some(Self {
            system,
            attacker: side,
            defender: GameWorld::facility_holder(sys.control)
                .filter(|&side| side != Faction::Neutral),
            taken: false,
            attackers: ReportForces {
                operational: attackers,
                ..ReportForces::default()
            },
            system_forces: ReportForces {
                operational: forces,
                ..ReportForces::default()
            },
        })
    }

    /// File `outcome`'s losses under Destroyed (`0x302`, `FUN_00497380`)
    /// and read whether the attacker now holds the system (notification
    /// 2), from `world` after the assault.
    ///
    /// hyp: Operational lists the units that survive the assault.
    /// port: the assault captures no personnel, so Captured stays empty.
    pub fn finish(&mut self, world: &GameWorld, outcome: &AssaultOutcome) {
        self.attackers.file_losses(|target| {
            matches!(target, Target::Troop(troop) if outcome.lost_attackers.contains(&troop))
        });
        self.system_forces.file_losses(|target| match target {
            Target::Troop(troop) => outcome.lost_defenders.contains(&troop),
            _ => outcome.collateral.contains(&target),
        });
        self.taken = world.systems.get(self.system).is_some_and(|sys| {
            crate::economy::system_holder(world, sys) == ControlKind::Controlled(self.attacker)
        });
    }

    /// `FUN_00497530`: kind 7 (the window opens at once) when the attacker
    /// is the viewer's side, else kind 3 (a message).
    #[must_use]
    pub fn opens_at_once(&self, viewer: Faction) -> bool {
        self.attacker == viewer
    }

    /// The picture `+0x2e`, by the attacker.
    #[must_use]
    pub fn picture(&self) -> u32 {
        PICTURES[usize::from(self.attacker != Faction::Alliance)]
    }

    /// The body `+0x50` (`FUN_00496770`): `0x7091` taken or `0x7092`
    /// defended for a held system, `0x7093` seized or `0x7094` repulsed
    /// for a neutral one. Its parameters are the attacker, the defender
    /// (when held) and the system.
    #[must_use]
    pub fn body(&self) -> u16 {
        match (self.defender.is_some(), self.taken) {
            (true, true) => 0x7091,
            (true, false) => 0x7092,
            (false, true) => 0x7093,
            (false, false) => 0x7094,
        }
    }
}

fn push(units: &mut Vec<ReportUnit>, dat_id: DatId, name: String, target: Option<Target>) {
    if let Some(category) = ReportCategory::of_family(dat_id.family()) {
        units.push(ReportUnit {
            category,
            dat_id,
            name,
            major: false,
            target,
        });
    }
}

fn push_character(units: &mut Vec<ReportUnit>, character: &crate::world::Character) {
    if let Some(category) = ReportCategory::of_family(character.dat_id.family()) {
        units.push(ReportUnit::new(
            category,
            character.dat_id,
            character.name.clone(),
            character.is_major,
        ));
    }
}

fn class_name(world: &GameWorld, class: DatId) -> String {
    world
        .buildable_classes
        .get(&class)
        .map(|value| value.name.clone())
        .unwrap_or_default()
}

fn troop_unit(world: &GameWorld, troop: crate::ids::TroopKey, units: &mut Vec<ReportUnit>) {
    if let Some(value) = world.troops.get(troop) {
        let name = class_name(world, value.class_dat_id);
        push(units, value.class_dat_id, name, Some(Target::Troop(troop)));
    }
}

/// One row per squadron. port: squadrons are counts per class.
fn squadrons(world: &GameWorld, entry: &crate::world::FighterEntry, units: &mut Vec<ReportUnit>) {
    if let Some(class) = world.fighter_classes.get(entry.class) {
        for _ in 0..entry.count {
            push(units, class.dat_id, class.name.clone(), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{ExplorationStatus, SectorGroup};
    use crate::ids::{DefenseFacilityKey, TroopKey};
    use crate::world::{
        BuildableClass, CapitalShipClass, Character, DefenseFacilityInstance, FighterClass,
        FighterEntry, Fleet, Sector, ShipInstance, System, TroopUnit,
    };

    struct Fx {
        world: GameWorld,
        transport: TroopTransportState,
        system: SystemKey,
        fleet: FleetKey,
        aboard: TroopKey,
        garrison: TroopKey,
        battery: DefenseFacilityKey,
    }

    fn named(world: &mut GameWorld, raw: u32, name: &str) -> DatId {
        let id = DatId::new(raw);
        world.buildable_classes.insert(
            id,
            BuildableClass {
                name: name.into(),
                ..BuildableClass::default()
            },
        );
        id
    }

    /// An Alliance system with a regiment and a battery; an Imperial fleet
    /// of one ship, two squadrons and a regiment in orbit.
    fn fixture(control: ControlKind) -> Fx {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(0x8000_0001),
            name: "Sector".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let system = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "Hoth".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
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
        });
        let ship_class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x1400_0001),
            name: "Star Destroyer".into(),
            is_empire: true,
            hull: 100,
            troop_capacity: 20,
            ..CapitalShipClass::default()
        });
        let fighter = world.fighter_classes.insert(FighterClass {
            dat_id: DatId::new(0x1c00_0005),
            name: "TIE Fighter".into(),
            ..FighterClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: ShipInstance::make(ship_class, 100, false, 1),
            fighters: vec![FighterEntry {
                class: fighter,
                count: 2,
            }],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[system].fleets.push(fleet);
        let stormtroopers = named(&mut world, 0x1000_0006, "Stormtroopers");
        let infantry = named(&mut world, 0x1000_0001, "Rebel Infantry");
        let aboard = world.troops.insert(TroopUnit {
            class_dat_id: stormtroopers,
            is_alliance: false,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(aboard);
        let mut transport = TroopTransportState::new();
        transport.embark(&mut world, fleet, &[aboard]).unwrap();
        let garrison = world.troops.insert(TroopUnit {
            class_dat_id: infantry,
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(garrison);
        let ion = named(&mut world, 0x2300_0002, "Ion Cannon");
        let battery = world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id: ion,
            side: Faction::Alliance,
        });
        world.systems[system].defense_facilities.push(battery);
        Fx {
            world,
            transport,
            system,
            fleet,
            aboard,
            garrison,
            battery,
        }
    }

    fn names(units: &[&ReportUnit]) -> Vec<String> {
        units.iter().map(|unit| unit.name.clone()).collect()
    }

    fn start(fx: &Fx) -> AssaultReport {
        AssaultReport::start(
            &fx.world,
            &fx.transport,
            Faction::Empire,
            &[fx.fleet],
            fx.system,
        )
        .unwrap()
    }

    #[test]
    fn the_report_lists_the_fleets_ships_squadrons_and_cargo_as_the_attackers() {
        // FUN_00496770 (0x182): the fleet's ships and their contents go to
        // +0x78; FUN_0044e500 sorts them by family.
        let fx = fixture(ControlKind::Controlled(Faction::Alliance));
        let report = start(&fx);
        let columns = |category| names(&report.attackers.columns(category)[0]);
        assert_eq!(columns(ReportCategory::CapitalShips), ["Star Destroyer"]);
        assert_eq!(
            columns(ReportCategory::Fighters),
            ["TIE Fighter", "TIE Fighter"]
        );
        assert_eq!(columns(ReportCategory::Troops), ["Stormtroopers"]);
        assert_eq!(report.defender, Some(Faction::Alliance));
    }

    #[test]
    fn the_report_lists_the_planets_regiments_and_facilities_as_the_system_forces() {
        // FUN_00496f50 (0x150): the system's contents go to +0x70.
        let fx = fixture(ControlKind::Controlled(Faction::Alliance));
        let report = start(&fx);
        let columns = |category| names(&report.system_forces.columns(category)[0]);
        assert_eq!(columns(ReportCategory::Troops), ["Rebel Infantry"]);
        assert_eq!(columns(ReportCategory::Defense), ["Ion Cannon"]);
        assert!(columns(ReportCategory::CapitalShips).is_empty());
    }

    #[test]
    fn the_system_forces_take_only_the_living_characters_on_the_planet() {
        // FUN_00496f50 (0x150): the system's contents, not a fleet's;
        // FUN_00496770 (0x182): the fleet's personnel go with the attackers.
        let mut fx = fixture(ControlKind::Controlled(Faction::Alliance));
        let mut character = |name: &str, at: Option<SystemKey>, killed: bool| {
            fx.world.characters.insert(Character {
                dat_id: DatId::new(0x3000_0001),
                name: name.into(),
                current_system: at,
                is_killed: killed,
                is_major: name == "Leia",
                ..Character::default()
            })
        };
        let here = Some(fx.system);
        character("Leia", here, false);
        character("Biggs", here, true);
        character("Wedge", None, false);
        let vader = character("Vader", here, false);
        fx.world.fleets[fx.fleet].characters.push(vader);
        let report = start(&fx);
        let personnel =
            |forces: &ReportForces| names(&forces.columns(ReportCategory::Personnel)[0]);
        assert_eq!(personnel(&report.system_forces), ["Leia"]);
        assert_eq!(personnel(&report.attackers), ["Vader"]);
        // pictures.rs: a major character's picture comes from its own table.
        let [leia, ..] = report.system_forces.columns(ReportCategory::Personnel);
        assert!(leia[0].major);
        let [vader, ..] = report.attackers.columns(ReportCategory::Personnel);
        assert!(!vader[0].major);
    }

    #[test]
    fn the_losses_move_to_destroyed_on_each_side() {
        // FUN_00497380 (0x302): a lost unit is filed under +0x80 (on the
        // system) or +0x88 (the attacker's).
        let mut fx = fixture(ControlKind::Controlled(Faction::Alliance));
        let mut report = start(&fx);
        let outcome = AssaultOutcome {
            attackers: 1,
            defenders: 1,
            lost_attackers: vec![fx.aboard],
            lost_defenders: vec![fx.garrison],
            collateral: vec![Target::Defense(fx.battery)],
            ..AssaultOutcome::default()
        };
        fx.world.troops.remove(fx.aboard);
        report.finish(&fx.world, &outcome);
        let [operational, _, destroyed] = report.attackers.columns(ReportCategory::Troops);
        assert!(operational.is_empty());
        assert_eq!(names(&destroyed), ["Stormtroopers"]);
        let [_, _, destroyed] = report.system_forces.columns(ReportCategory::Troops);
        assert_eq!(names(&destroyed), ["Rebel Infantry"]);
        let [operational, _, destroyed] = report.system_forces.columns(ReportCategory::Defense);
        assert!(operational.is_empty());
        assert_eq!(names(&destroyed), ["Ion Cannon"]);
    }

    #[test]
    fn a_held_system_left_to_the_attackers_regiments_is_taken() {
        // FUN_00496770 (notification 2): 0x7091 when the system's side is
        // now the attacker's, 0x7092 when not.
        let mut fx = fixture(ControlKind::Controlled(Faction::Alliance));
        let mut report = start(&fx);
        let outcome = AssaultOutcome {
            lost_defenders: vec![fx.garrison],
            landed: vec![fx.aboard],
            ..AssaultOutcome::default()
        };
        fx.world.systems[fx.system]
            .ground_units
            .retain(|&k| k != fx.garrison);
        fx.world.troops.remove(fx.garrison);
        fx.transport
            .disembark_selected(&mut fx.world, fx.fleet, fx.system, &[fx.aboard])
            .unwrap();
        report.finish(&fx.world, &outcome);
        assert!(report.taken);
        assert_eq!(report.body(), 0x7091);

        let fx = fixture(ControlKind::Controlled(Faction::Alliance));
        let mut report = start(&fx);
        report.finish(&fx.world, &AssaultOutcome::default());
        assert!(!report.taken);
        assert_eq!(report.body(), 0x7092);
    }

    #[test]
    fn a_neutral_system_reads_seized_or_repulsed() {
        // FUN_00496770: +0x68 0 (neutral) takes 0x7093 or 0x7094.
        let fx = fixture(ControlKind::Uncontrolled);
        let mut report = start(&fx);
        assert_eq!(report.defender, None);
        report.taken = true;
        assert_eq!(report.body(), 0x7093);
        report.taken = false;
        assert_eq!(report.body(), 0x7094);
    }

    #[test]
    fn the_window_opens_at_once_only_for_the_attackers_side() {
        // FUN_00497530: kind 7 when +0x6c is the viewer's side, else 3.
        let fx = fixture(ControlKind::Controlled(Faction::Alliance));
        let report = start(&fx);
        assert!(report.opens_at_once(Faction::Empire));
        assert!(!report.opens_at_once(Faction::Alliance));
        // FUN_00496770: +0x2e 0x2b98 for side 1, 0x2b99 otherwise.
        assert_eq!(report.picture(), 0x2b99);
    }

    #[test]
    fn each_category_takes_its_families() {
        // FUN_0044e500's ranges for buttons 0x97..0x9c.
        for (family, category) in [
            (0x14, ReportCategory::CapitalShips),
            (0x1f, ReportCategory::Fighters),
            (0x28, ReportCategory::Manufacturing),
            (0x2f, ReportCategory::Manufacturing),
            (0x22, ReportCategory::Defense),
            (0x13, ReportCategory::Troops),
            (0x30, ReportCategory::Personnel),
            (0x3f, ReportCategory::Personnel),
        ] {
            assert_eq!(ReportCategory::of_family(family), Some(category));
        }
        for family in [0x08, 0x20, 0x21, 0x40, 0x90] {
            assert_eq!(ReportCategory::of_family(family), None);
        }
    }
}
