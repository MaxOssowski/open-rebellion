//! The agent's Build orders: the Agent menu's Build Ships, Build Troops and
//! Build Facilities (`0x110..0x112`), run by the agent's module `0x17`
//! (`ghidra/notes/agent-build-orders.md`).
//!
//! An item puts the galaxy view into the target cursor (`FUN_00439d10` →
//! `FUN_0049e360`, `FUN_0041d5e0`). The player clicks a destination
//! ([`accept_target`], `FUN_0049e380`). The agent then finds the nearest
//! idle producer of the kind ([`find_idle_producer`], `FUN_0049e7a0`),
//! points that producer's Destination at the target (`0x214`,
//! `FUN_0049e960`) and opens Build Selection on it (`FUN_0049e6b0` phase 3,
//! `FUN_0041d640`). The player picks the class and quantity there.

use crate::dat::Faction;
use crate::ids::{FleetKey, SectorKey, SystemKey};
use crate::manufacturing::{ManufacturingState, ProductionArea};
use crate::uprising::UprisingState;
use crate::world::GameWorld;

/// What a Build item asks for: module `0x17`'s `+0x34` (`FUN_00439d10`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildKind {
    /// Kind 1, Build Facilities (`0x112`, Alt+F).
    Facilities,
    /// Kind 2, Build Ships (`0x110`, Alt+B).
    Ships,
    /// Kind 3, Build Troops (`0x111`, Alt+T).
    Troops,
}

impl BuildKind {
    /// The Agent menu item's kind (`FUN_00439d10`): `0x110` ships, `0x111`
    /// troops, `0x112` facilities.
    #[must_use]
    pub const fn from_command(command: u16) -> Option<Self> {
        match command {
            0x110 => Some(Self::Ships),
            0x111 => Some(Self::Troops),
            0x112 => Some(Self::Facilities),
            _ => None,
        }
    }

    /// The producer the kind builds with (`FUN_0049ec00` →
    /// `FUN_00509670`: area 0 ships, 2 troops, 1 facilities).
    #[must_use]
    pub const fn area(self) -> ProductionArea {
        match self {
            Self::Ships => ProductionArea::Shipyard,
            Self::Troops => ProductionArea::TrainingFacility,
            Self::Facilities => ProductionArea::ConstructionYard,
        }
    }
}

/// The destination the player clicked: a system, or a fleet (with its
/// system). `FUN_0049e380` takes a system (`0x90..0x97`), a fleet
/// (`0x08..0x0f`) or a capital ship (`0x14..0x1b`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildTarget {
    pub system: SystemKey,
    pub fleet: Option<FleetKey>,
}

/// Why the agent turned a request down: the status words it hands the
/// advisor (`agent->vt+0xc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildRefusal {
    /// Troops to a system the side does not hold and has no fleet at
    /// (`FUN_0049e380`: `{1, 0x24}`).
    UnheldSystem,
    /// No own system has an idle producer of the kind (`FUN_0049e7a0`:
    /// `{0xa0, 3}`).
    NoIdleProducer,
}

impl BuildRefusal {
    /// The status words the advisor reacts to (`agent->vt+0xc`).
    #[must_use]
    pub const fn status(self) -> (u32, u32) {
        match self {
            // FUN_0049e380
            Self::UnheldSystem => (1, 0x24),
            // FUN_0049e7a0
            Self::NoIdleProducer => (0xa0, 3),
        }
    }
}

/// A request the agent took: module `0x17` in state 0 with its kind and
/// target (`+0x34`, `+0x3c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildRequest {
    pub kind: BuildKind,
    pub target: BuildTarget,
}

/// What the agent does with a request (`FUN_0049e6b0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildOutcome {
    /// Point `producer`'s `area` at `destination` (Destination `0x214`,
    /// `FUN_0049e960`), then open Build Selection on it (`FUN_0049ec40` →
    /// `FUN_0041d640`).
    Open {
        producer: SystemKey,
        area: ProductionArea,
        destination: SystemKey,
    },
    /// The advisor's refusal reaction; nothing is built.
    Refused(BuildRefusal),
}

/// `FUN_0049e380`: the clicked destination. Troops to a system the side
/// does not hold, with no fleet of its own there, are refused at once;
/// anything else is taken.
///
/// hyp: `FUN_004feea0(system, side, 1)` asks whether the side has a fleet
/// at the system.
pub fn accept_target(
    world: &GameWorld,
    side: Faction,
    kind: BuildKind,
    target: BuildTarget,
) -> Result<BuildRequest, BuildRefusal> {
    if kind == BuildKind::Troops && target.fleet.is_none() {
        let held = world
            .systems
            .get(target.system)
            .is_some_and(|sys| crate::uprising::holder(sys) == Some(side));
        let fleet_there = world.fleets.values().any(|fleet| {
            fleet.location == target.system && Faction::of_alliance(fleet.is_alliance) == side
        });
        if !held && !fleet_there {
            return Err(BuildRefusal::UnheldSystem);
        }
    }
    Ok(BuildRequest { kind, target })
}

/// `FUN_0049e6b0`: find the producer (phase 1), then hand back its
/// Destination and Build Selection (phases 2 and 3).
///
/// port: the phases run at once. The original runs one a work slice of the
/// agent (`FUN_00439a10`), and A0 opens Build Selection in under a second at
/// Very Slow, within the day (`evidence/2026-10-30-agent-build-orders/`).
#[must_use]
pub fn run(
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    uprisings: &UprisingState,
    side: Faction,
    request: BuildRequest,
) -> BuildOutcome {
    let area = request.kind.area();
    match find_idle_producer(
        world,
        manufacturing,
        uprisings,
        side,
        area,
        request.target.system,
    ) {
        Some(producer) => BuildOutcome::Open {
            producer,
            area,
            destination: request.target.system,
        },
        None => BuildOutcome::Refused(BuildRefusal::NoIdleProducer),
    }
}

/// The player record's idle bit for `area` (`FUN_004ebe40`: `0x80`
/// shipyards, `0x100` training facilities, `0x200` construction yards),
/// with the side holding the system (`4`). An explored, populated system
/// with a yard of the area, no uprising (`1`) or blockade (`2`), and its
/// manager building nothing (`RemainingGameObjCount == 0`).
///
/// port: every yard counts as active (`+0x50 & 1`); the port has no yard
/// under construction.
fn idle_producer_at(
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    uprisings: &UprisingState,
    side: Faction,
    area: ProductionArea,
    system: SystemKey,
) -> bool {
    let Some(sys) = world.systems.get(system) else {
        return false;
    };
    crate::uprising::holder(sys) == Some(side)
        && sys.exploration_status == crate::dat::ExplorationStatus::Explored
        && sys.is_populated
        && !uprisings.is_uprising(system)
        && !crate::blockade::BlockadeSystem::system_is_blockaded(world, sys)
        && crate::ai_module1::has_yard(
            world,
            side,
            system,
            crate::build_selection::area_family(area),
        )
        && manufacturing
            .queue(system, area)
            .is_none_or(crate::manufacturing::ProductionQueue::is_empty)
}

/// `FUN_0049e7a0`: the nearest sector to the target's with an own system
/// holding an idle producer of `area` (sector record bits `0x1000`,
/// `0x2000`, `0x4000` with `0x10`; `FUN_0049d770` order 2), then that
/// sector's own system nearest the target with one (`FUN_0049da70`).
///
/// hyp: order 2 lists by distance and `FUN_004357b0` takes the nearest; of
/// equally near ones the first in record order.
#[must_use]
pub fn find_idle_producer(
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    uprisings: &UprisingState,
    side: Faction,
    area: ProductionArea,
    target: SystemKey,
) -> Option<SystemKey> {
    let target_sys = world.systems.get(target)?;
    let idle =
        |system: SystemKey| idle_producer_at(world, manufacturing, uprisings, side, area, system);
    let in_sector = |sector: SectorKey| {
        world
            .systems
            .iter()
            .filter(move |(_, sys)| sys.sector == sector)
            .map(|(key, _)| key)
    };
    let origin = world.sectors.get(target_sys.sector)?;
    let sector = nearest(
        world
            .sectors
            .iter()
            .filter(|&(key, _)| in_sector(key).any(idle))
            .map(|(key, s)| (key, distance((origin.x, origin.y), (s.x, s.y)))),
    )?;
    nearest(in_sector(sector).filter(|&key| idle(key)).map(|key| {
        let sys = &world.systems[key];
        (key, distance((target_sys.x, target_sys.y), (sys.x, sys.y)))
    }))
}

/// `FUN_0041b7b0` on two map positions.
fn distance(a: (u16, u16), b: (u16, u16)) -> i32 {
    crate::ai_records::squared_distance(
        (a.0.cast_signed(), a.1.cast_signed()),
        (b.0.cast_signed(), b.1.cast_signed()),
    )
}

/// The first of the smallest distances.
fn nearest<K: Copy>(candidates: impl Iterator<Item = (K, i32)>) -> Option<K> {
    candidates
        .fold(None, |best: Option<(K, i32)>, (key, d)| match best {
            Some((_, b)) if b <= d => best,
            _ => Some((key, d)),
        })
        .map(|(key, _)| key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_module1::tests::{add_yard, system};
    use crate::ids::DatId;
    use crate::manufacturing::{BuildableKind, QueueItem};
    use crate::world::{ControlKind, Fleet, Sector};

    const SHIPYARD: u32 = 0x2800_0001;
    const TRAINING: u32 = 0x2900_0001;

    fn sector(world: &mut GameWorld, x: u16) -> SectorKey {
        world.sectors.insert(Sector {
            dat_id: DatId::new(0x8000_0001),
            name: "Sector".into(),
            group: crate::dat::SectorGroup::Core,
            x,
            y: 0,
            systems: vec![],
        })
    }

    fn yard(world: &mut GameWorld, at: SystemKey, raw: u32) {
        add_yard(world, at, DatId::new(raw), Faction::Alliance);
    }

    fn find(world: &GameWorld, mfg: &ManufacturingState, target: SystemKey) -> Option<SystemKey> {
        find_idle_producer(
            world,
            mfg,
            &UprisingState::default(),
            Faction::Alliance,
            ProductionArea::Shipyard,
            target,
        )
    }

    #[test]
    fn the_menu_items_ask_for_ships_troops_and_facilities_from_their_yards() {
        // FUN_00439d10: 0x110 -> 2, 0x111 -> 3, 0x112 -> 1; FUN_0049ec00
        // asks FUN_00509670 for area 0, 2 and 1.
        assert_eq!(BuildKind::from_command(0x110), Some(BuildKind::Ships));
        assert_eq!(BuildKind::from_command(0x111), Some(BuildKind::Troops));
        assert_eq!(BuildKind::from_command(0x112), Some(BuildKind::Facilities));
        assert_eq!(BuildKind::from_command(0x113), None);
        assert_eq!(BuildKind::Ships.area(), ProductionArea::Shipyard);
        assert_eq!(BuildKind::Troops.area(), ProductionArea::TrainingFacility);
        assert_eq!(
            BuildKind::Facilities.area(),
            ProductionArea::ConstructionYard
        );
    }

    #[test]
    fn the_nearest_sector_wins_then_its_system_nearest_the_target() {
        // FUN_0049e7a0: sectors by distance from the target's sector, then
        // that sector's systems by distance from the target.
        let mut world = GameWorld::default();
        let home = sector(&mut world, 0);
        let near = sector(&mut world, 10);
        let far = sector(&mut world, 50);
        let target = system(&mut world, home, 0);
        let far_yard = system(&mut world, far, 1);
        let near_a = system(&mut world, near, 40);
        let near_b = system(&mut world, near, 20);
        for at in [far_yard, near_a, near_b] {
            yard(&mut world, at, SHIPYARD);
        }
        let mfg = ManufacturingState::new();
        assert_eq!(find(&world, &mfg, target), Some(near_b));
    }

    #[test]
    fn a_busy_held_away_or_troubled_yard_is_passed_over() {
        // FUN_004ebe40: 0x80 needs the side holding the system, no uprising
        // (1) or blockade (2), and an empty manager.
        let mut world = GameWorld::default();
        let home = sector(&mut world, 0);
        let target = system(&mut world, home, 0);
        let busy = system(&mut world, home, 1);
        let enemy = system(&mut world, home, 2);
        let rising = system(&mut world, home, 3);
        let idle = system(&mut world, home, 9);
        for at in [busy, enemy, rising, idle] {
            yard(&mut world, at, SHIPYARD);
        }
        world.systems[enemy].control = ControlKind::Controlled(Faction::Empire);
        let mut mfg = ManufacturingState::new();
        mfg.enqueue(
            busy,
            QueueItem::new(BuildableKind::CapitalShip(Default::default()), 10),
        );
        let mut uprisings = UprisingState::default();
        uprisings.active_uprisings.insert(
            rising,
            crate::uprising::ActiveUprising {
                started_tick: 0,
                next_incident_tick: None,
                next_support_tick: None,
                side: None,
            },
        );
        let found = find_idle_producer(
            &world,
            &mfg,
            &uprisings,
            Faction::Alliance,
            ProductionArea::Shipyard,
            target,
        );
        assert_eq!(found, Some(idle));
    }

    #[test]
    fn only_a_yard_of_the_kinds_area_counts() {
        // FUN_0049eb80 / FUN_0049ebc0: the area's own bits.
        let mut world = GameWorld::default();
        let home = sector(&mut world, 0);
        let target = system(&mut world, home, 0);
        let barracks = system(&mut world, home, 1);
        yard(&mut world, barracks, TRAINING);
        let mfg = ManufacturingState::new();
        assert_eq!(find(&world, &mfg, target), None);
        let request = BuildRequest {
            kind: BuildKind::Troops,
            target: BuildTarget {
                system: target,
                fleet: None,
            },
        };
        assert_eq!(
            run(
                &world,
                &mfg,
                &UprisingState::default(),
                Faction::Alliance,
                request
            ),
            BuildOutcome::Open {
                producer: barracks,
                area: ProductionArea::TrainingFacility,
                destination: target,
            }
        );
        let ships = BuildRequest {
            kind: BuildKind::Ships,
            ..request
        };
        assert_eq!(
            run(
                &world,
                &mfg,
                &UprisingState::default(),
                Faction::Alliance,
                ships
            ),
            BuildOutcome::Refused(BuildRefusal::NoIdleProducer)
        );
        assert_eq!(BuildRefusal::NoIdleProducer.status(), (0xa0, 3));
    }

    #[test]
    fn troops_to_an_unheld_system_without_a_fleet_there_are_refused() {
        // FUN_0049e380: kind 3, a system not held by the side, and
        // FUN_004feea0 finds no fleet of the side: {1, 0x24}.
        let mut world = GameWorld::default();
        let home = sector(&mut world, 0);
        let enemy = system(&mut world, home, 0);
        world.systems[enemy].control = ControlKind::Controlled(Faction::Empire);
        let at = BuildTarget {
            system: enemy,
            fleet: None,
        };
        assert_eq!(
            accept_target(&world, Faction::Alliance, BuildKind::Troops, at),
            Err(BuildRefusal::UnheldSystem)
        );
        assert_eq!(BuildRefusal::UnheldSystem.status(), (1, 0x24));
        // Ships may go there, and troops may once a fleet of the side is;
        // the other side's fleet there, or the side's fleet elsewhere, does
        // not count.
        assert!(accept_target(&world, Faction::Alliance, BuildKind::Ships, at).is_ok());
        let elsewhere = system(&mut world, home, 9);
        let add_fleet = |world: &mut GameWorld, location, is_alliance| {
            world.fleets.insert(Fleet {
                location,
                capital_ships: vec![],
                fighters: vec![],
                characters: vec![],
                is_alliance,
                has_death_star: false,
                special_forces: vec![],
            })
        };
        add_fleet(&mut world, enemy, false);
        add_fleet(&mut world, elsewhere, true);
        assert_eq!(
            accept_target(&world, Faction::Alliance, BuildKind::Troops, at),
            Err(BuildRefusal::UnheldSystem)
        );
        let fleet = add_fleet(&mut world, enemy, true);
        assert!(accept_target(&world, Faction::Alliance, BuildKind::Troops, at).is_ok());
        let aboard = BuildTarget {
            system: enemy,
            fleet: Some(fleet),
        };
        assert!(accept_target(&world, Faction::Alliance, BuildKind::Troops, aboard).is_ok());
    }
}
