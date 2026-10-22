//! Maintenance shortfall: the side's autoscrap timer `0x382`
//! (`ghidra/notes/side-resource-monitors.md`, "Autoscrap").
//!
//! `FUN_0052f670` raises the side's over-limit flag (`+0xa8`,
//! `FUN_0052f3d0`) while its maintenance requirement (`+0x70`) or use
//! (`+0x74`) exceeds its capacity (`+0x58`). The flag's hook (side slot
//! `+0x1f8`, `FUN_005329f0`) arms timer `0x382` with the flag on record
//! `+0xcc`, whose minimum `FUN_00532350` sets to GNPRTB 7168 (10): raising
//! the flag starts it, lowering it cancels it, and a fired timer repeats.
//!
//! Each time it fires, `FUN_00530350` checks the limit again, counts the
//! side's objects of types `0x10..0x40` that are existing, completed and
//! carry a class maintenance cost (`FUN_0052e530`), draws one with
//! `FUN_0053e290(count - 1)` and sets its `AutoscrapRequest`
//! (`FUN_004f7950`). The request's hook (slot `+0x140` -> `+0xe0`,
//! `FUN_004f8850`) destroys an existing object at once with reason `0x15`
//! (slot `+0xac`). The side refunds half the class's refined material cost
//! (`FUN_00530270`), and the destruction (`GameObjDestroyedAutoscrap`,
//! `0x304`) posts the Maintenance Shortfall message (TEXTSTRA 28800-28802).

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::ids::{FighterKey, FleetKey, SpecialForceKey, SystemKey, TroopKey};
use crate::manufacturing::ManufacturingState;
use crate::production_cycle::ProductionState;
use crate::side_resources::maintenance_totals;
use crate::tick::TickEvent;
use crate::troop_transport::TroopTransportState;
use crate::world::{FacilityRef, GameWorld};

/// GNPRTB 7168 (`DAT_006bb50c`) = 10: timer `0x382`'s minimum
/// (`FUN_00532350`, side record `+0xcc`).
pub const GNPRTB_AUTOSCRAP_PERIOD: u16 = 7168;

/// The destroyed reason of an autoscrap (`FUN_004f8850`).
pub const AUTOSCRAP_REASON: u8 = 0x15;

/// Each side's timer `0x382`: the tick it next fires, while the side is
/// over its limit (save v64).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoscrapState {
    pub alliance: Option<u64>,
    pub empire: Option<u64>,
}

impl AutoscrapState {
    fn timer(&mut self, side: Faction) -> &mut Option<u64> {
        if side == Faction::Alliance {
            &mut self.alliance
        } else {
            &mut self.empire
        }
    }
}

/// An object `FUN_00530350` may pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoscrapTarget {
    /// A regiment, on a planet, aboard a fleet or travelling.
    Troop(TroopKey),
    /// The capital ship at `index` in `fleet`.
    Ship {
        fleet: FleetKey,
        index: usize,
    },
    /// One squadron of `class` aboard `fleet`.
    FleetFighter {
        fleet: FleetKey,
        class: FighterKey,
    },
    /// One squadron of `class` on `system`'s planet.
    PlanetFighter {
        system: SystemKey,
        class: FighterKey,
    },
    Facility(FacilityRef),
    SpecialForce(SpecialForceKey),
}

/// One autoscrapped object, for the caller to apply with [`apply`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Autoscrap {
    pub side: Faction,
    pub target: AutoscrapTarget,
    /// The class name, for the message's list (TEXTSTRA 28802).
    pub unit: String,
    /// The system it stood at, if any (TEXTSTRA 28801).
    pub system: Option<SystemKey>,
    pub tick: u64,
}

/// One candidate: the target, its class name, refined cost and system.
struct Candidate {
    target: AutoscrapTarget,
    unit: String,
    refined_cost: u32,
    system: Option<SystemKey>,
}

/// `FUN_0052e530` over `FUN_00505340`'s walk: `side`'s objects with a class
/// maintenance cost, by type (regiments `0x10`, capital ships `0x14`,
/// squadrons `0x1c`, facilities `0x22..0x2d`, special forces `0x3c`).
///
/// port: within a type the walk follows the world's key order, and a
/// fleet's squadrons come before the planets'. Every object in the world is
/// existing and completed; queued builds are not in it.
#[expect(
    clippy::too_many_lines,
    reason = "One walk over the object types, kept in the original's order."
)]
fn candidates(world: &GameWorld, transport: &TroopTransportState, side: Faction) -> Vec<Candidate> {
    let alliance = side == Faction::Alliance;
    let mut found = Vec::new();
    let system_of = |pred: &dyn Fn(&crate::world::System) -> bool| {
        world
            .systems
            .iter()
            .find(|(_, system)| pred(system))
            .map(|(key, _)| key)
    };
    let buildable = |class| world.buildable_classes.get(&class);

    for (key, troop) in &world.troops {
        let Some(class) = buildable(troop.class_dat_id) else {
            continue;
        };
        if troop.is_alliance == alliance && class.maintenance_cost > 0 {
            let system = system_of(&|s| s.ground_units.contains(&key)).or_else(|| {
                transport
                    .carrier(key)
                    .and_then(|fleet| world.fleets.get(fleet))
                    .map(|fleet| fleet.location)
            });
            found.push(Candidate {
                target: AutoscrapTarget::Troop(key),
                unit: class.name.clone(),
                refined_cost: class.refined_material_cost,
                system,
            });
        }
    }
    for (key, fleet) in &world.fleets {
        if fleet.is_alliance != alliance {
            continue;
        }
        for (index, ship) in fleet.capital_ships.iter().enumerate() {
            let Some(class) = world.capital_ship_classes.get(ship.class) else {
                continue;
            };
            if ship.alive && class.maintenance_cost > 0 {
                found.push(Candidate {
                    target: AutoscrapTarget::Ship { fleet: key, index },
                    unit: class.name.clone(),
                    refined_cost: class.refined_material_cost,
                    system: Some(fleet.location),
                });
            }
        }
    }
    let mut squadrons = |entries: &[crate::world::FighterEntry],
                         target: &dyn Fn(FighterKey) -> AutoscrapTarget,
                         system: Option<SystemKey>| {
        for entry in entries {
            let Some(class) = world.fighter_classes.get(entry.class) else {
                continue;
            };
            if class.is_alliance == alliance && class.maintenance_cost > 0 {
                for _ in 0..entry.count {
                    found.push(Candidate {
                        target: target(entry.class),
                        unit: class.name.clone(),
                        refined_cost: class.refined_material_cost,
                        system,
                    });
                }
            }
        }
    };
    for (key, fleet) in &world.fleets {
        if fleet.is_alliance == alliance {
            squadrons(
                &fleet.fighters,
                &|class| AutoscrapTarget::FleetFighter { fleet: key, class },
                Some(fleet.location),
            );
        }
    }
    for (key, system) in &world.systems {
        squadrons(
            &system.fighters,
            &|class| AutoscrapTarget::PlanetFighter { system: key, class },
            Some(key),
        );
    }
    let mut facility = |target: FacilityRef, owner: Faction, class_id, system| {
        if let Some(class) = buildable(class_id) {
            if owner == side && class.maintenance_cost > 0 {
                found.push(Candidate {
                    target: AutoscrapTarget::Facility(target),
                    unit: class.name.clone(),
                    refined_cost: class.refined_material_cost,
                    system,
                });
            }
        }
    };
    for (key, f) in &world.defense_facilities {
        let system = system_of(&|s| s.defense_facilities.contains(&key));
        facility(FacilityRef::Defense(key), f.side, f.class_dat_id, system);
    }
    for (key, f) in &world.manufacturing_facilities {
        let system = system_of(&|s| s.manufacturing_facilities.contains(&key));
        facility(
            FacilityRef::Manufacturing(key),
            f.side,
            f.class_dat_id,
            system,
        );
    }
    for (key, f) in &world.production_facilities {
        let system = system_of(&|s| s.production_facilities.contains(&key));
        facility(FacilityRef::Production(key), f.side, f.class_dat_id, system);
    }
    for (key, unit) in &world.special_forces {
        let Some(class) = buildable(unit.class_dat_id) else {
            continue;
        };
        if unit.is_alliance == alliance && class.maintenance_cost > 0 {
            found.push(Candidate {
                target: AutoscrapTarget::SpecialForce(key),
                unit: class.name.clone(),
                refined_cost: class.refined_material_cost,
                system: system_of(&|s| s.special_forces.contains(&key)),
            });
        }
    }
    found
}

/// Stateless system for timer `0x382`.
pub struct AutoscrapSystem;

impl AutoscrapSystem {
    /// Raise or lower each side's flag from its totals (`FUN_0052f670`),
    /// arm or cancel its timer, and fire the timers that are due by the
    /// last tick: each picks one object with the next of `rolls`.
    ///
    /// port: the original raises the flag the moment a total changes; the
    /// port looks once a day, after the day's production.
    pub fn advance(
        state: &mut AutoscrapState,
        world: &GameWorld,
        transport: &TroopTransportState,
        manufacturing: &ManufacturingState,
        blocked: impl Fn(SystemKey) -> bool,
        tick_events: &[TickEvent],
        rolls: &[f64],
    ) -> Vec<Autoscrap> {
        let Some(last) = tick_events.last() else {
            return Vec::new();
        };
        let tick = last.tick;
        // port: a record's spread FUN_00532350 does not set is 0, so the
        // delay is the minimum and takes no roll.
        let period = u64::try_from(
            world
                .gnprtb
                .value(GNPRTB_AUTOSCRAP_PERIOD, world.difficulty_index)
                .max(1),
        )
        .unwrap_or(1);
        let mut rolls = rolls.iter().copied();
        let mut scrapped = Vec::new();
        for side in [Faction::Alliance, Faction::Empire] {
            let over = maintenance_totals(world, manufacturing, side, &blocked).is_over_limit();
            let timer = state.timer(side);
            match (over, *timer) {
                (false, _) => *timer = None,
                (true, None) => *timer = Some(tick + period),
                (true, Some(due)) if due <= tick => {
                    *timer = Some(due + period);
                    let found = candidates(world, transport, side);
                    if found.is_empty() {
                        continue;
                    }
                    let roll = rolls.next().unwrap_or(0.0);
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        clippy::cast_precision_loss,
                        reason = "the roll is in [0, 1), so the index is below the count"
                    )]
                    let index = ((roll * found.len() as f64) as usize).min(found.len() - 1);
                    let chosen = &found[index];
                    scrapped.push(Autoscrap {
                        side,
                        target: chosen.target,
                        unit: chosen.unit.clone(),
                        system: chosen.system,
                        tick,
                    });
                }
                (true, Some(_)) => {}
            }
        }
        scrapped
    }
}

/// Remove the autoscrapped object and refund half its class's refined
/// material cost to its side (`FUN_00530270`, reason `0x15`, completed).
/// Returns the refund.
pub fn apply(
    world: &mut GameWorld,
    production: &mut ProductionState,
    transport: &mut TroopTransportState,
    scrap: &Autoscrap,
) -> i32 {
    let refined_cost = candidates(world, transport, scrap.side)
        .into_iter()
        .find(|candidate| candidate.target == scrap.target)
        .map_or(0, |candidate| candidate.refined_cost);
    let refund = i32::try_from(refined_cost / 2).unwrap_or(i32::MAX);
    match scrap.target {
        AutoscrapTarget::Troop(key) => {
            for system in world.systems.values_mut() {
                system.ground_units.retain(|&troop| troop != key);
            }
            transport.forget_transit(key);
            if !transport.destroy_embarked(world, key) {
                world.troops.remove(key);
            }
        }
        AutoscrapTarget::Ship { fleet, index } => {
            if world
                .fleets
                .get(fleet)
                .is_some_and(|value| index < value.capital_ships.len())
            {
                crate::scrap::remove_ship(world, fleet, index);
            }
        }
        AutoscrapTarget::FleetFighter { fleet, class } => {
            if let Some(value) = world.fleets.get_mut(fleet) {
                crate::squadrons::take(&mut value.fighters, class, 1);
            }
            crate::scrap::disband_if_empty(world, fleet);
        }
        AutoscrapTarget::PlanetFighter { system, class } => {
            if let Some(value) = world.systems.get_mut(system) {
                crate::squadrons::take(&mut value.fighters, class, 1);
            }
        }
        AutoscrapTarget::Facility(facility) => {
            if let Some(system) = scrap.system {
                world.remove_facility(system, facility);
            }
        }
        AutoscrapTarget::SpecialForce(key) => crate::missions::destroy_special_force(world, key),
    }
    if let Some(stock) = production.stock_mut(scrap.side) {
        stock.refined += refund;
    }
    refund
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DatId;
    use crate::world::{
        BuildableClass, CapitalShipClass, ControlKind, FighterClass, FighterEntry, Fleet,
        GnprtbEntry, GnprtbParams, ProductionFacilityInstance, ShipInstance, SpecialForceUnit,
        System, TroopUnit,
    };

    const REGIMENT: u32 = 0x1000_0001;
    const MINE: u32 = 0x2c00_0001;
    const REFINERY: u32 = 0x2d00_0001;
    const COMMANDOS: u32 = 0x3c00_0001;

    struct Fixture {
        world: GameWorld,
        here: SystemKey,
    }

    fn fixture() -> Fixture {
        let mut world = GameWorld {
            gnprtb: GnprtbParams::new(vec![GnprtbEntry {
                parameter_id: u32::from(GNPRTB_AUTOSCRAP_PERIOD),
                development: 10,
                alliance_sp_easy: 10,
                alliance_sp_medium: 10,
                alliance_sp_hard: 10,
                empire_sp_easy: 10,
                empire_sp_medium: 10,
                empire_sp_hard: 10,
                multiplayer: 10,
            }]),
            ..GameWorld::default()
        };
        for (id, name, upkeep, refined) in [
            (REGIMENT, "Regiment", 2, 7),
            (MINE, "Mine", 0, 4),
            (REFINERY, "Refinery", 0, 4),
            (COMMANDOS, "Commandos", 1, 9),
        ] {
            world.buildable_classes.insert(
                DatId::new(id),
                BuildableClass {
                    name: name.into(),
                    maintenance_cost: upkeep,
                    refined_material_cost: refined,
                    ..BuildableClass::default()
                },
            );
        }
        let here = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "Here".into(),
            sector: crate::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 10,
            raw_materials: 10,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: vec![],
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Alliance),
        });
        Fixture { world, here }
    }

    fn regiment(f: &mut Fixture, alliance: bool) -> TroopKey {
        let key = f.world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(REGIMENT),
            is_alliance: alliance,
            regiment_strength: 100,
        });
        f.world.systems[f.here].ground_units.push(key);
        key
    }

    /// One mine and one refinery give the Alliance 50 capacity.
    fn capacity(f: &mut Fixture) {
        for (id, is_mine) in [(MINE, true), (REFINERY, false)] {
            let key = f
                .world
                .production_facilities
                .insert(ProductionFacilityInstance {
                    class_dat_id: DatId::new(id),
                    side: Faction::Alliance,
                    is_mine,
                });
            f.world.systems[f.here].production_facilities.push(key);
        }
    }

    fn day(state: &mut AutoscrapState, f: &Fixture, tick: u64, roll: f64) -> Vec<Autoscrap> {
        AutoscrapSystem::advance(
            state,
            &f.world,
            &TroopTransportState::default(),
            &ManufacturingState::new(),
            |_| false,
            &[TickEvent { tick }],
            &[roll, roll],
        )
    }

    #[test]
    fn a_side_over_its_limit_loses_one_object_every_ten_days() {
        // FUN_0052f670 raises the flag (upkeep 2 > capacity 0); its hook
        // FUN_005329f0 arms timer 0x382 (GNPRTB 7168 = 10), which repeats.
        let mut f = fixture();
        let troop = regiment(&mut f, true);
        let mut state = AutoscrapState::default();
        for tick in 1..=10 {
            assert_eq!(day(&mut state, &f, tick, 0.0), vec![]);
        }
        assert_eq!(state.alliance, Some(11));
        let scrapped = day(&mut state, &f, 11, 0.0);
        assert_eq!(
            scrapped,
            vec![Autoscrap {
                side: Faction::Alliance,
                target: AutoscrapTarget::Troop(troop),
                unit: "Regiment".into(),
                system: Some(f.here),
                tick: 11,
            }]
        );
        assert_eq!(state.alliance, Some(21));
        assert_eq!(state.empire, None);
    }

    #[test]
    fn a_side_back_within_its_limit_cancels_the_timer() {
        // FUN_005329f0 arms 0x382 with the lowered flag, cancelling it.
        let mut f = fixture();
        regiment(&mut f, true);
        let mut state = AutoscrapState::default();
        day(&mut state, &f, 1, 0.0);
        assert_eq!(state.alliance, Some(11));
        capacity(&mut f);
        assert_eq!(day(&mut state, &f, 11, 0.0), vec![]);
        assert_eq!(state.alliance, None);
    }

    #[test]
    fn the_pick_runs_over_the_side_s_maintained_objects_by_type() {
        // FUN_00530350 counts what FUN_0052e530 accepts over FUN_00505340's
        // walk (types 0x10..0x40) and draws FUN_0053e290(count - 1).
        let mut f = fixture();
        regiment(&mut f, false);
        let troop = regiment(&mut f, true);
        let ship_class = f.world.capital_ship_classes.insert(CapitalShipClass {
            name: "Corvette".into(),
            is_alliance: true,
            maintenance_cost: 3,
            ..CapitalShipClass::default()
        });
        let wing = f.world.fighter_classes.insert(FighterClass {
            name: "X-wing".into(),
            is_alliance: true,
            maintenance_cost: 1,
            ..FighterClass::default()
        });
        let fleet = f.world.insert_fleet(Fleet {
            location: f.here,
            capital_ships: vec![ShipInstance::new(ship_class, 10, true)],
            fighters: vec![FighterEntry {
                class: wing,
                count: 2,
            }],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        f.world.systems[f.here].fleets.push(fleet);
        // Upkeep-free facilities are not candidates.
        capacity(&mut f);
        let unit = f.world.special_forces.insert(SpecialForceUnit {
            class_dat_id: DatId::new(COMMANDOS),
            is_alliance: true,
            skills: [0; 8],
            on_mission: false,
        });
        f.world.systems[f.here].special_forces.push(unit);
        let defense = f
            .world
            .defense_facilities
            .insert(crate::world::DefenseFacilityInstance {
                class_dat_id: DatId::new(REGIMENT),
                side: Faction::Alliance,
            });
        f.world.systems[f.here].defense_facilities.push(defense);
        f.world.systems[f.here].fighters.push(FighterEntry {
            class: wing,
            count: 1,
        });

        // Neither the other side's objects nor those without upkeep count
        // (FUN_0052e530: same side bits, FUN_004f2990 not 0).
        let free_ship = f.world.capital_ship_classes.insert(CapitalShipClass {
            is_alliance: true,
            ..CapitalShipClass::default()
        });
        f.world.fleets[fleet]
            .capital_ships
            .push(ShipInstance::new(free_ship, 10, true));
        let free_wing = f.world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            ..FighterClass::default()
        });
        let enemy_wing = f.world.fighter_classes.insert(FighterClass {
            is_alliance: false,
            maintenance_cost: 1,
            ..FighterClass::default()
        });
        for class in [free_wing, enemy_wing] {
            f.world.fleets[fleet]
                .fighters
                .push(FighterEntry { class, count: 1 });
            f.world.systems[f.here]
                .fighters
                .push(FighterEntry { class, count: 1 });
        }
        let enemy_ship = f.world.capital_ship_classes.insert(CapitalShipClass {
            is_alliance: false,
            maintenance_cost: 3,
            ..CapitalShipClass::default()
        });
        let enemy_fleet = f.world.insert_fleet(Fleet {
            location: f.here,
            capital_ships: vec![ShipInstance::new(enemy_ship, 10, false)],
            fighters: vec![FighterEntry {
                class: enemy_wing,
                count: 1,
            }],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        f.world.systems[f.here].fleets.push(enemy_fleet);
        let free_troop = f.world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(MINE),
            is_alliance: true,
            regiment_strength: 100,
        });
        f.world.systems[f.here].ground_units.push(free_troop);
        for (class, is_alliance) in [(MINE, true), (COMMANDOS, false)] {
            let other = f.world.special_forces.insert(SpecialForceUnit {
                class_dat_id: DatId::new(class),
                is_alliance,
                skills: [0; 8],
                on_mission: false,
            });
            f.world.systems[f.here].special_forces.push(other);
        }
        let enemy_defense =
            f.world
                .defense_facilities
                .insert(crate::world::DefenseFacilityInstance {
                    class_dat_id: DatId::new(REGIMENT),
                    side: Faction::Empire,
                });
        f.world.systems[f.here]
            .defense_facilities
            .push(enemy_defense);

        let order: Vec<AutoscrapTarget> =
            candidates(&f.world, &TroopTransportState::default(), Faction::Alliance)
                .into_iter()
                .map(|candidate| candidate.target)
                .collect();
        assert_eq!(
            order,
            vec![
                AutoscrapTarget::Troop(troop),
                AutoscrapTarget::Ship { fleet, index: 0 },
                AutoscrapTarget::FleetFighter { fleet, class: wing },
                AutoscrapTarget::FleetFighter { fleet, class: wing },
                AutoscrapTarget::PlanetFighter {
                    system: f.here,
                    class: wing
                },
                AutoscrapTarget::Facility(FacilityRef::Defense(defense)),
                AutoscrapTarget::SpecialForce(unit),
            ]
        );
        // Without the mine and refinery the side has no capacity.
        f.world.production_facilities.clear();
        f.world.systems[f.here].production_facilities.clear();
        let mut state = AutoscrapState {
            alliance: Some(1),
            empire: None,
        };
        let last = day(&mut state, &f, 1, 0.999_999);
        assert_eq!(last[0].target, AutoscrapTarget::SpecialForce(unit));
        // A roll of 1 still picks the last.
        let mut state = AutoscrapState {
            alliance: Some(1),
            empire: None,
        };
        assert_eq!(
            day(&mut state, &f, 1, 1.0)[0].target,
            AutoscrapTarget::SpecialForce(unit)
        );
        let mut state = AutoscrapState {
            alliance: Some(1),
            empire: None,
        };
        let middle = day(&mut state, &f, 1, 0.5);
        assert_eq!(
            middle[0].target,
            AutoscrapTarget::FleetFighter { fleet, class: wing }
        );
    }

    #[test]
    fn an_autoscrap_removes_the_object_and_refunds_half_its_refined_cost() {
        // FUN_004f8850 destroys with reason 0x15; FUN_00530270 refunds class
        // +0x48 / 2 for reasons 0x14..0x16: 7 / 2 = 3.
        let mut f = fixture();
        let troop = regiment(&mut f, true);
        let mut production = ProductionState::new();
        let mut transport = TroopTransportState::default();
        let scrap = Autoscrap {
            side: Faction::Alliance,
            target: AutoscrapTarget::Troop(troop),
            unit: "Regiment".into(),
            system: Some(f.here),
            tick: 11,
        };
        assert_eq!(
            apply(&mut f.world, &mut production, &mut transport, &scrap),
            3
        );
        assert!(f.world.troops.is_empty());
        assert_eq!(f.world.systems[f.here].ground_units, vec![]);
        assert_eq!(production.stock(Faction::Alliance).unwrap().refined, 3);

        // A squadron aboard: the fleet left empty disbands (FUN_004fe630).
        let wing = f.world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            maintenance_cost: 1,
            refined_material_cost: 5,
            ..FighterClass::default()
        });
        let fleet = f.world.insert_fleet(Fleet {
            location: f.here,
            capital_ships: vec![],
            fighters: vec![FighterEntry {
                class: wing,
                count: 1,
            }],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        f.world.systems[f.here].fleets.push(fleet);
        let scrap = Autoscrap {
            target: AutoscrapTarget::FleetFighter { fleet, class: wing },
            ..scrap
        };
        assert_eq!(
            apply(&mut f.world, &mut production, &mut transport, &scrap),
            2
        );
        assert!(!f.world.fleets.contains_key(fleet));
        assert_eq!(f.world.systems[f.here].fleets, vec![]);
        assert_eq!(production.stock(Faction::Alliance).unwrap().refined, 5);
    }

    #[test]
    fn an_autoscrapped_ship_or_carried_regiment_leaves_its_fleet() {
        // FUN_004f8850 destroys the object itself: a ship leaves its fleet,
        // a regiment its fleet's cargo, each refunded half its cost.
        let mut f = fixture();
        let transport_class = f.world.capital_ship_classes.insert(CapitalShipClass {
            name: "Transport".into(),
            is_alliance: true,
            maintenance_cost: 3,
            refined_material_cost: 9,
            troop_capacity: 2,
            ..CapitalShipClass::default()
        });
        let fleet = f.world.insert_fleet(Fleet {
            location: f.here,
            capital_ships: vec![
                ShipInstance::new(transport_class, 10, true),
                ShipInstance::new(transport_class, 10, true),
            ],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        f.world.systems[f.here].fleets.push(fleet);
        let troop = regiment(&mut f, true);
        let mut transport = TroopTransportState::default();
        transport.embark(&mut f.world, fleet, &[troop]).unwrap();
        let found = candidates(&f.world, &transport, Faction::Alliance);
        assert_eq!(found[0].system, Some(f.here));

        let mut production = ProductionState::new();
        let base = Autoscrap {
            side: Faction::Alliance,
            target: AutoscrapTarget::Troop(troop),
            unit: "Regiment".into(),
            system: Some(f.here),
            tick: 11,
        };
        assert_eq!(
            apply(&mut f.world, &mut production, &mut transport, &base),
            3
        );
        assert_eq!(transport.cargo(fleet), &[]);
        assert!(!f.world.troops.contains_key(troop));

        let ship = Autoscrap {
            target: AutoscrapTarget::Ship { fleet, index: 1 },
            ..base
        };
        assert_eq!(
            apply(&mut f.world, &mut production, &mut transport, &ship),
            4
        );
        assert_eq!(f.world.fleets[fleet].capital_ships.len(), 1);
        // A stale index removes nothing.
        apply(&mut f.world, &mut production, &mut transport, &ship);
        assert_eq!(f.world.fleets[fleet].capital_ships.len(), 1);
        assert_eq!(production.stock(Faction::Alliance).unwrap().refined, 7);
    }
}
