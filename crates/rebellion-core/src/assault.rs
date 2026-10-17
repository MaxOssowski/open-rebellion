//! Planetary Assault: the Fleet menu's order `0x234`, resolved as the game
//! does (`FUN_00563390` -> `FUN_0058c1e0` -> `FUN_0058c120`;
//! `ghidra/notes/planetary-assault.md`).
//!
//! [`resolve`] reads the world and returns an [`AssaultOutcome`];
//! [`apply`] carries it out. The draws come from the tick's
//! [`crate::bombardment::BombardEnv`] ([`crate::bombardment::BombardEnv::assault`]).
//!
//! port: the port has no officer ranks (`+0x96`), so the generals'
//! leadership terms are 0; a fleet's regiments are its cargo, not each
//! ship's, so the walk takes each fleet's cargo in order.

use serde::{Deserialize, Serialize};

use crate::bombardment::Target;
use crate::dat::Faction;
use crate::ids::{FleetKey, SystemKey, TroopKey};
use crate::movement::MovementState;
use crate::troop_transport::TroopTransportState;
use crate::world::{FacilityRef, GameWorld};

/// Planetary Assault (`0x234`, `FUN_00581450`; STRATEGY 12318).
pub const PLANETARY_ASSAULT: u32 = 0x234;

/// GNPRTB `0x200` (`DAT_006bb788`, 5): a defense's hit chance divisor
/// (`FUN_0055ead0`).
const GNPRTB_DEFENSE_DIVISOR: u16 = 0x200;
/// GNPRTB `0x201` (`DAT_006bb780`, 30): a general's leadership divisor
/// (`FUN_0055eb00`).
const GNPRTB_GENERAL_DIVISOR: u16 = 0x201;
/// GNPRTB `0x202` (`DAT_006bb78c`, 10): the duel's draw, 0 to it.
const GNPRTB_DUEL_DRAW: u16 = 0x202;
/// GNPRTB `0x203` (`DAT_006bb778`, 4): the attacker is lost at or below it.
const GNPRTB_ATTACKER_LOST: u16 = 0x203;
/// GNPRTB `0x204` (`DAT_006bb784`, 6): the defender is lost at or above it.
const GNPRTB_DEFENDER_LOST: u16 = 0x204;
/// GNPRTB `0x205` (`DAT_006bb77c`, 8): a duel's collateral chance, percent
/// (`FUN_0055eb80`).
const GNPRTB_COLLATERAL: u16 = 0x205;
/// GNPRTB `0x1e51` (`DAT_006bb480`, 60): the garrison's support base
/// (`FUN_0055a050`).
const GNPRTB_GARRISON_BASE: u16 = 0x1e51;
/// GNPRTB `0x1e52` (`DAT_006bb3a8`, -10): the garrison's support step.
const GNPRTB_GARRISON_STEP: u16 = 0x1e52;

/// `FUN_0053e2e0`'s range (`DAT_00661a88`): a percent draw, 0 to 99.
const PERCENT: i32 = 100;

/// What one assault asks: the attacking side's fleets (the order's team) at
/// `system`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssaultRequest {
    pub system: SystemKey,
    pub side: Faction,
    pub fleets: Vec<FleetKey>,
    /// The attacking regiments in the walk's order (`FUN_0058ba10`): the
    /// request's fleets of the side, in the system's order, each fleet's
    /// regiments aboard.
    pub troops: Vec<TroopKey>,
}

/// What an assault did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssaultOutcome {
    /// The attackers and defenders counted at the start (`+0x1c`, `+0x20`).
    pub attackers: i32,
    pub defenders: i32,
    /// The attacking regiments lost, to the defenses and in the duels.
    pub lost_attackers: Vec<TroopKey>,
    /// The defending regiments lost in the duels.
    pub lost_defenders: Vec<TroopKey>,
    /// The duels fought (`+0x2c`).
    pub duels: i32,
    /// What the duels' collateral damage destroyed (`FUN_0058c580`).
    pub collateral: Vec<Target>,
    /// The attacking regiments that landed (`FUN_0058c810`).
    pub landed: Vec<TroopKey>,
}

fn gnprtb(world: &GameWorld, id: u16) -> i32 {
    world.gnprtb.value(id, world.difficulty_index)
}

fn int(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// `FUN_0053e290(n)`: a draw from 0 to `n` (`draw(m)` gives 0 to `m - 1`).
fn upto(draw: &mut dyn FnMut(i32) -> i32, n: i32) -> i32 {
    if n >= 0 {
        draw(n.saturating_add(1))
    } else {
        -draw(1i32.saturating_sub(n))
    }
}

/// `FUN_0053e2f0(p)`: a percent draw (`FUN_0053e2e0`, 0 to 99) below `p`.
fn chance(draw: &mut dyn FnMut(i32) -> i32, percent: i32) -> bool {
    upto(draw, PERCENT - 1) < percent
}

/// The side of `system`'s objects (`+0x24 >> 6 & 3`).
fn system_side(world: &GameWorld, system: SystemKey) -> Option<Faction> {
    world
        .systems
        .get(system)
        .and_then(|s| GameWorld::facility_holder(s.control))
}

/// The attacking regiments of `side`'s `fleets` at `system`
/// (`FUN_0058ba10`): the fleets in the system's order, each one's cargo.
#[must_use]
pub fn attacking_troops(
    world: &GameWorld,
    transport: &TroopTransportState,
    side: Faction,
    fleets: &[FleetKey],
    system: SystemKey,
) -> Vec<TroopKey> {
    let Some(sys) = world.systems.get(system) else {
        return Vec::new();
    };
    sys.fleets
        .iter()
        .filter(|f| fleets.contains(f))
        .filter(|&&f| {
            world
                .fleets
                .get(f)
                .is_some_and(|value| Faction::of_alliance(value.is_alliance) == side)
        })
        .flat_map(|&f| transport.cargo(f).iter().copied())
        .filter(|&t| world.troops.contains_key(t))
        .collect()
}

/// The system's regiments of its own side (`FUN_0058bbd0`, `FUN_00504cc0`).
fn defending_troops(world: &GameWorld, system: SystemKey) -> Vec<TroopKey> {
    let (Some(sys), Some(side)) = (world.systems.get(system), system_side(world, system)) else {
        return Vec::new();
    };
    sys.ground_units
        .iter()
        .copied()
        .filter(|&t| {
            world
                .troops
                .get(t)
                .is_some_and(|u| Faction::of_alliance(u.is_alliance) == side)
        })
        .collect()
}

fn troop_stat(world: &GameWorld, troop: TroopKey, attack: bool) -> i32 {
    world
        .troops
        .get(troop)
        .and_then(|t| world.troop_classes.get(&t.class_dat_id))
        .map_or(0, |c| {
            int(if attack {
                c.attack_strength
            } else {
                c.defense_strength
            })
        })
}

/// The system's facilities the collateral damage draws from
/// (`FUN_00539d70`, families `0x20..0x2f`), without the placeholders
/// (`FUN_00526cf0`, `0x20..0x21`: the Alliance's headquarters).
///
/// hyp: the walk's order is the families' (defenses, then manufacturing,
/// then production facilities), each of any side.
fn collateral_facilities(world: &GameWorld, system: SystemKey) -> Vec<Target> {
    let Some(sys) = world.systems.get(system) else {
        return Vec::new();
    };
    let defenses = sys
        .defense_facilities
        .iter()
        .filter(|&&k| world.defense_facilities.contains_key(k))
        .map(|&k| Target::Defense(k));
    let manufacturing = sys
        .manufacturing_facilities
        .iter()
        .filter(|&&k| world.manufacturing_facilities.contains_key(k))
        .map(|&k| Target::Manufacturing(k));
    let production = sys
        .production_facilities
        .iter()
        .filter(|&&k| world.production_facilities.contains_key(k))
        .map(|&k| Target::Production(k));
    defenses.chain(manufacturing).chain(production).collect()
}

/// `FUN_0055a050(0)` through `FUN_00559fe0(side, 0, 0, 0, 0)`: the
/// garrison a system at support 0 needs, `ceil((G0x1e51 - 0) / -G0x1e52)`
/// (`FUN_0053e160`).
fn landing_limit(world: &GameWorld) -> i32 {
    let base = gnprtb(world, GNPRTB_GARRISON_BASE);
    let step = -gnprtb(world, GNPRTB_GARRISON_STEP);
    if base <= 0 || step == 0 {
        return 0;
    }
    (step - 1 + base) / step
}

/// Resolve one assault (`FUN_0058c120`), drawing with `draw(m)` (0 to
/// `m - 1`).
#[expect(
    clippy::too_many_lines,
    reason = "The six phases in FUN_0058c120's order."
)]
pub fn resolve(
    world: &GameWorld,
    req: &AssaultRequest,
    draw: &mut dyn FnMut(i32) -> i32,
) -> AssaultOutcome {
    let mut out = AssaultOutcome::default();
    let Some(system) = world.systems.get(req.system) else {
        return out;
    };
    // Phase 1 (FUN_0058c2d0): the counts.
    let mut attackers = req.troops.clone();
    let mut defenders = defending_troops(world, req.system);
    out.attackers = i32::try_from(attackers.len()).unwrap_or(i32::MAX);
    out.defenders = i32::try_from(defenders.len()).unwrap_or(i32::MAX);
    // Phase 2 (FUN_0058c3c0): each defense facility fires with chance
    // attack / G0x200 percent at a drawn landing regiment (from those
    // counted at the start; a draw past those left finds none).
    // hyp: every defense facility of the system, the walk's side filter
    // not read.
    let divisor = gnprtb(world, GNPRTB_DEFENSE_DIVISOR).max(1);
    for &d in &system.defense_facilities {
        if attackers.is_empty() {
            break;
        }
        let Some(facility) = world.defense_facilities.get(d) else {
            continue;
        };
        let attack = world
            .buildable_classes
            .get(&facility.class_dat_id)
            .map_or(0, |c| int(c.stats.attack_strength));
        if !chance(draw, attack / divisor) {
            continue;
        }
        let k = upto(draw, out.attackers - 1);
        if let Some(lost) = usize::try_from(k)
            .ok()
            .filter(|&k| k < attackers.len())
            .map(|k| attackers.remove(k))
        {
            out.lost_attackers.push(lost);
        }
    }
    // Phase 3 (FUN_0058c500, FUN_0058bec0): each attacking regiment in
    // order duels a drawn defender while both sides have some.
    if !attackers.is_empty() && !defenders.is_empty() {
        // port: no officer ranks, so no general's leadership (slot +0x1f4).
        let leadership: i32 = 0;
        let general = leadership / gnprtb(world, GNPRTB_GENERAL_DIVISOR).max(1);
        let duel_draw = gnprtb(world, GNPRTB_DUEL_DRAW);
        let attacker_lost = gnprtb(world, GNPRTB_ATTACKER_LOST);
        let defender_lost = gnprtb(world, GNPRTB_DEFENDER_LOST);
        let mut i = 0;
        while i < attackers.len() && !defenders.is_empty() {
            let attacker = attackers[i];
            let r = upto(draw, out.defenders - 1);
            let Some(at) = usize::try_from(r).ok().filter(|&r| r < defenders.len()) else {
                i += 1;
                continue;
            };
            let defender = defenders[at];
            out.duels += 1;
            // FUN_0055eb00: the draw, plus the attacker's TROOPSD attack
            // (class +0x64) and its general, less the defender's defense
            // (+0x68) and its general.
            let x = upto(draw, duel_draw) + (troop_stat(world, attacker, true) + general)
                - (troop_stat(world, defender, false) + general);
            if x >= defender_lost {
                out.lost_defenders.push(defenders.remove(at));
                i += 1;
            } else if x <= attacker_lost {
                out.lost_attackers.push(attackers.remove(i));
            } else {
                i += 1;
            }
        }
    }
    // Phase 4 (FUN_0058c580, FUN_0055eb80, FUN_0055ebc0): each duel with
    // chance G0x205 percent destroys a drawn facility, energy or raw unit.
    let collateral_chance = gnprtb(world, GNPRTB_COLLATERAL);
    let hits = (0..out.duels)
        .filter(|_| chance(draw, collateral_chance))
        .count();
    let mut energy = system.total_energy;
    let mut raw = system.raw_materials;
    for _ in 0..hits {
        let facilities: Vec<Target> = collateral_facilities(world, req.system)
            .into_iter()
            .filter(|t| !out.collateral.contains(t))
            .collect();
        let f = i32::try_from(facilities.len()).unwrap_or(i32::MAX);
        let total = f + i32::from(energy > 0) + i32::from(raw > 0);
        if total == 0 {
            break;
        }
        let r = upto(draw, total - 1);
        let target = if r < f {
            usize::try_from(r)
                .ok()
                .and_then(|r| facilities.get(r).copied())
        } else if r == f && energy > 0 {
            Some(Target::Energy)
        } else if raw > 0 {
            Some(Target::Raw)
        } else {
            None
        };
        // Nothing drawn ends the phase (local_64).
        let Some(t) = target else {
            break;
        };
        match t {
            Target::Energy => energy -= 1,
            Target::Raw => raw -= 1,
            _ => {}
        }
        out.collateral.push(t);
    }
    // Phase 5 (FUN_0058c810): with attackers left and no defender, up to
    // the garrison of support 0 land, in walk order.
    if !attackers.is_empty() && defenders.is_empty() {
        let limit = usize::try_from(landing_limit(world)).unwrap_or(0);
        out.landed = attackers.iter().copied().take(limit).collect();
    }
    // Phase 6 (FUN_0058c8b0) clears the assault marks the port does not keep.
    out
}

/// Carry out `outcome`: the lost regiments leave their fleets and the
/// planet, the collateral damage is done, and the landed regiments go
/// down to the planet (`FUN_004f6fd0`). The system's control is left to the
/// caller's occupation.
pub fn apply(
    world: &mut GameWorld,
    transport: &mut TroopTransportState,
    req: &AssaultRequest,
    outcome: &AssaultOutcome,
) {
    for &troop in &outcome.lost_attackers {
        transport.destroy_embarked(world, troop);
        world.troops.remove(troop);
    }
    for &troop in &outcome.lost_defenders {
        if let Some(sys) = world.systems.get_mut(req.system) {
            sys.ground_units.retain(|&k| k != troop);
        }
        world.troops.remove(troop);
    }
    for &target in &outcome.collateral {
        match target {
            Target::Defense(k) => world.remove_facility(req.system, FacilityRef::Defense(k)),
            Target::Manufacturing(k) => {
                world.remove_facility(req.system, FacilityRef::Manufacturing(k));
            }
            Target::Production(k) => {
                world.remove_facility(req.system, FacilityRef::Production(k));
            }
            Target::Energy => {
                if let Some(sys) = world.systems.get_mut(req.system) {
                    sys.total_energy = sys.total_energy.saturating_sub(1);
                }
            }
            Target::Raw => {
                if let Some(sys) = world.systems.get_mut(req.system) {
                    sys.raw_materials = sys.raw_materials.saturating_sub(1);
                }
            }
            Target::Troop(_) | Target::Headquarters => {}
        }
    }
    for &fleet in &req.fleets {
        let landing: Vec<TroopKey> = transport
            .cargo(fleet)
            .iter()
            .copied()
            .filter(|t| outcome.landed.contains(t))
            .collect();
        if !landing.is_empty() {
            let _ = transport.disembark_selected(world, fleet, req.system, &landing);
        }
    }
}

/// Whether the side's `fleets` may order Planetary Assault of `system`: the
/// order's enabled check, a team of one side (`FUN_00553770`) with an active
/// fleet carrying a regiment (`FUN_00581ae0`; else `8`/`7` without such a
/// fleet, `8`/`5` without a regiment).
///
/// port: the members' target test (`FUN_0053f150`) is not read; in its
/// place each fleet must orbit `system`, out of hyperspace, and `system`
/// must stand, not be the side's, and not be fully shielded (two
/// shields: the manual, "Offensive play"; `FUN_004ae240`'s `+0x30`
/// `0x100`).
#[must_use]
pub fn order_enabled(
    world: &GameWorld,
    movement: &MovementState,
    transport: &TroopTransportState,
    is_alliance: bool,
    fleets: &[FleetKey],
    system: SystemKey,
) -> bool {
    let at = |f: &FleetKey| {
        world.fleets.get(*f).is_some_and(|value| {
            value.is_alliance == is_alliance
                && value.location == system
                && !movement.is_in_transit(*f)
        })
    };
    let carrying = |f: &FleetKey| {
        transport
            .cargo(*f)
            .iter()
            .any(|&t| world.troops.contains_key(t))
    };
    !fleets.is_empty()
        && fleets.iter().all(at)
        && fleets.iter().any(carrying)
        && world.systems.get(system).is_some_and(|sys| {
            !sys.is_destroyed
                && GameWorld::facility_holder(sys.control)
                    != Some(Faction::of_alliance(is_alliance))
                && shields(world, system) <= 1
        })
}

/// The system's shields (family `0x24`, DEFFACSD 3 and 6).
fn shields(world: &GameWorld, system: SystemKey) -> usize {
    world.systems.get(system).map_or(0, |sys| {
        sys.defense_facilities
            .iter()
            .filter_map(|&d| world.defense_facilities.get(d))
            .filter(|f| f.class_dat_id.family() == SHIELDS)
            .count()
    })
}

/// The planetary shields' family (`0x24`, DEFFACSD 3 and 6, `FUN_00527940`).
const SHIELDS: u8 = 0x24;

impl crate::bombardment::BombardEnv {
    /// Resolve and carry out the assault of `system` by `side`'s `fleets`;
    /// the outcome is kept in [`Self::assaults`].
    pub fn assault(
        &mut self,
        world: &mut GameWorld,
        transport: &mut TroopTransportState,
        side: Faction,
        fleets: &[FleetKey],
        system: SystemKey,
    ) -> &AssaultOutcome {
        let req = AssaultRequest {
            system,
            side,
            fleets: fleets.to_vec(),
            troops: attacking_troops(world, transport, side, fleets, system),
        };
        let outcome = resolve(world, &req, &mut |m| self.draw(m));
        apply(world, transport, &req, &outcome);
        self.assaults.push((req, outcome));
        &self.assaults[self.assaults.len() - 1].1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{ExplorationStatus, SectorGroup};
    use crate::ids::{DatId, DefenseFacilityKey};
    use crate::world::{
        BuildableClass, CapitalShipClass, ClassStats, ControlKind, DefenseFacilityInstance, Fleet,
        GnprtbParams, ManufacturingFacilityInstance, Sector, ShipInstance, System, TroopClassDef,
        TroopUnit,
    };

    /// GNPRTB as shipped (planetary-assault.md): 0x200..0x205, 0x1e51,
    /// 0x1e52.
    fn gnprtb() -> GnprtbParams {
        GnprtbParams::uniform(&[
            (0x200, 5),
            (0x201, 30),
            (0x202, 10),
            (0x203, 4),
            (0x204, 6),
            (0x205, 8),
            (0x1e51, 60),
            (0x1e52, -10),
        ])
    }

    struct Fx {
        world: GameWorld,
        transport: TroopTransportState,
        system: SystemKey,
        fleet: FleetKey,
    }

    const ATTACKERS: u32 = 0x1000_0006;
    const DEFENDERS: u32 = 0x1000_0001;

    /// An Alliance system with an Imperial fleet in orbit; the Empire's
    /// regiments attack 5, the Alliance's defend 5.
    fn fixture() -> Fx {
        let mut world = GameWorld {
            gnprtb: gnprtb(),
            ..GameWorld::default()
        };
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(0x8000_0001),
            name: "Sector".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let system = world.systems.insert(System {
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
            control: ControlKind::Controlled(Faction::Alliance),
        });
        let ship_class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x1400_0001),
            is_empire: true,
            hull: 100,
            troop_capacity: 20,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: ShipInstance::make(ship_class, 100, false, 1),
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[system].fleets.push(fleet);
        for (class, attack, defense) in [(ATTACKERS, 5, 0), (DEFENDERS, 0, 5)] {
            world.troop_classes.insert(
                DatId::new(class),
                TroopClassDef {
                    attack_strength: attack,
                    defense_strength: defense,
                    detection: 0,
                },
            );
        }
        Fx {
            world,
            transport: TroopTransportState::new(),
            system,
            fleet,
        }
    }

    /// `n` Imperial regiments aboard the fleet.
    fn aboard(fx: &mut Fx, n: usize) -> Vec<TroopKey> {
        let troops: Vec<TroopKey> = (0..n)
            .map(|_| {
                let t = fx.world.troops.insert(TroopUnit {
                    class_dat_id: DatId::new(ATTACKERS),
                    is_alliance: false,
                    regiment_strength: 100,
                });
                fx.world.systems[fx.system].ground_units.push(t);
                t
            })
            .collect();
        fx.transport
            .embark(&mut fx.world, fx.fleet, &troops)
            .unwrap();
        troops
    }

    /// `n` Alliance regiments on the planet.
    fn defending(fx: &mut Fx, n: usize) -> Vec<TroopKey> {
        (0..n)
            .map(|_| {
                let t = fx.world.troops.insert(TroopUnit {
                    class_dat_id: DatId::new(DEFENDERS),
                    is_alliance: true,
                    regiment_strength: 100,
                });
                fx.world.systems[fx.system].ground_units.push(t);
                t
            })
            .collect()
    }

    fn defense(fx: &mut Fx, raw: u32, attack: u32) -> DefenseFacilityKey {
        let class_dat_id = DatId::new(raw);
        fx.world.buildable_classes.insert(
            class_dat_id,
            BuildableClass {
                stats: ClassStats {
                    attack_strength: attack,
                    ..ClassStats::default()
                },
                ..BuildableClass::default()
            },
        );
        let key = fx.world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id,
            side: Faction::Alliance,
        });
        fx.world.systems[fx.system].defense_facilities.push(key);
        key
    }

    fn request(fx: &Fx) -> AssaultRequest {
        AssaultRequest {
            system: fx.system,
            side: Faction::Empire,
            fleets: vec![fx.fleet],
            troops: attacking_troops(
                &fx.world,
                &fx.transport,
                Faction::Empire,
                &[fx.fleet],
                fx.system,
            ),
        }
    }

    /// Draws from a script, then `fill`, each clamped to `0..m`.
    fn script(draws: &[i32], fill: i32) -> impl FnMut(i32) -> i32 + '_ {
        let mut at = 0;
        move |m| {
            let d = draws.get(at).copied().unwrap_or(fill);
            at += 1;
            d.clamp(0, m - 1)
        }
    }

    #[test]
    fn each_attacker_duels_a_drawn_defender_and_the_draw_decides_who_falls() {
        // FUN_0058bec0: a draw of 0..+0x20 - 1 below the defenders left picks
        // one, else no duel; FUN_0055eb00: x = 0..10 + attack - defense, the
        // defender lost at or above G0x204 (6), the attacker at or below
        // G0x203 (4). Collateral draws (percent) of 99 hit nothing.
        let mut fx = fixture();
        let a = aboard(&mut fx, 3);
        let d = defending(&mut fx, 2);
        let req = request(&fx);
        assert_eq!(req.troops, a);
        // a0: defender 1, x 6 -> d1 falls; a1: draw 1, only one left: no
        // duel; a2: defender 0, x 4 -> a2 falls. Then 2 collateral draws.
        let out = resolve(&fx.world, &req, &mut script(&[1, 6, 1, 0, 4, 99, 99], 99));
        assert_eq!((out.attackers, out.defenders, out.duels), (3, 2, 2));
        assert_eq!(out.lost_defenders, vec![d[1]]);
        assert_eq!(out.lost_attackers, vec![a[2]]);
        assert_eq!(out.landed, vec![]);
        // x 5 neither falls.
        let out = resolve(&fx.world, &req, &mut script(&[0, 5, 0, 5, 0, 5], 99));
        assert_eq!(
            (
                out.duels,
                out.lost_attackers.len(),
                out.lost_defenders.len()
            ),
            (3, 0, 0)
        );
        // The defense and the attack count: attack 5 against defense 7.
        fx.world
            .troop_classes
            .get_mut(&DatId::new(DEFENDERS))
            .unwrap()
            .defense_strength = 7;
        let out = resolve(&fx.world, &req, &mut script(&[0, 6], 99));
        assert_eq!(out.lost_attackers, vec![a[0]]);
    }

    #[test]
    fn with_no_defender_left_the_winners_land_up_to_the_garrison_of_support_zero() {
        // FUN_0058c810: attackers left and no defender: up to
        // ceil(G0x1e51 / -G0x1e52) = 6 land, in walk order (FUN_004f6fd0).
        let mut fx = fixture();
        let a = aboard(&mut fx, 8);
        let d = defending(&mut fx, 1);
        let req = request(&fx);
        let out = resolve(&fx.world, &req, &mut script(&[0, 6], 99));
        assert_eq!(out.lost_defenders, d);
        assert_eq!(out.landed, a[..6].to_vec());
        apply(&mut fx.world, &mut fx.transport, &req, &out);
        let ground = &fx.world.systems[fx.system].ground_units;
        assert!(a[..6].iter().all(|t| ground.contains(t)));
        assert!(!ground.contains(&d[0]));
        assert!(!fx.world.troops.contains_key(d[0]));
        assert_eq!(fx.transport.cargo(fx.fleet), &a[6..]);
        // Without any defender the attackers land at once, no duel drawn.
        let mut fx = fixture();
        let a = aboard(&mut fx, 2);
        let out = resolve(&fx.world, &request(&fx), &mut script(&[], 0));
        assert_eq!((out.duels, out.landed.clone()), (0, a));
        // A defender left: none land.
        let mut fx = fixture();
        aboard(&mut fx, 2);
        defending(&mut fx, 1);
        let out = resolve(&fx.world, &request(&fx), &mut script(&[0, 5, 0, 5], 99));
        assert_eq!(out.landed, vec![]);
    }

    #[test]
    fn the_defenses_fire_at_drawn_landing_regiments_with_their_attack_over_five() {
        // FUN_0058c3c0: each defense facility (0x22..0x27) hits with chance
        // class +0x5c / G0x200 percent (FUN_0055ead0), a hit taking the
        // drawn attacker (0..+0x1c - 1); a shield (attack 0) never hits but
        // draws. A draw past those left finds none.
        let mut fx = fixture();
        let a = aboard(&mut fx, 2);
        defense(&mut fx, 0x2400_0003, 0);
        defense(&mut fx, 0x2300_0002, 50);
        defense(&mut fx, 0x2300_0002, 50);
        defense(&mut fx, 0x2300_0002, 50);
        // Shield 0 (no hit); battery 9 < 10: hit a1; battery 9: draw 1 is
        // past the one left; battery 10: miss.
        let out = resolve(
            &fx.world,
            &request(&fx),
            &mut script(&[0, 9, 1, 9, 1, 10], 0),
        );
        assert_eq!(out.lost_attackers, vec![a[1]]);
        assert_eq!(out.landed, vec![a[0]]);
        // Every attacker lost ends the phase: no more draws.
        let mut fx = fixture();
        let a = aboard(&mut fx, 1);
        defense(&mut fx, 0x2300_0002, 50);
        defense(&mut fx, 0x2300_0002, 50);
        let mut draws = vec![0, 0];
        let mut used = 0;
        let out = resolve(&fx.world, &request(&fx), &mut |m| {
            used += 1;
            draws.pop().unwrap_or(0).clamp(0, m - 1)
        });
        assert_eq!((out.lost_attackers, out.landed, used), (a, vec![], 2));
    }

    #[test]
    fn each_duel_may_destroy_a_drawn_facility_or_resource_unit() {
        // FUN_0055eb80: each duel hits with chance G0x205 (8) percent;
        // FUN_0055ebc0: a draw over the facilities, then an energy unit,
        // then a raw unit; nothing to draw ends the phase.
        let mut fx = fixture();
        aboard(&mut fx, 2);
        defending(&mut fx, 2);
        let class_dat_id = DatId::new(0x2800_0001);
        let shipyard = fx
            .world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id,
                side: Faction::Alliance,
                is_shipyard: true,
            });
        fx.world.systems[fx.system]
            .manufacturing_facilities
            .push(shipyard);
        fx.world.systems[fx.system].total_energy = 1;
        fx.world.systems[fx.system].raw_materials = 1;
        // Two duels (x 5); collateral 7 (hit), 8 (miss); the hit draws 1 of
        // 0..2: the energy unit.
        let req = request(&fx);
        let out = resolve(&fx.world, &req, &mut script(&[0, 5, 0, 5, 7, 8, 1], 0));
        assert_eq!(out.collateral, vec![Target::Energy]);
        // Draw 0: the shipyard; draw 2: the raw unit.
        let out = resolve(&fx.world, &req, &mut script(&[0, 5, 0, 5, 7, 7, 0, 2], 0));
        assert_eq!(
            out.collateral,
            vec![Target::Manufacturing(shipyard), Target::Raw]
        );
        apply(&mut fx.world, &mut fx.transport, &req, &out);
        let sys = &fx.world.systems[fx.system];
        assert_eq!((sys.raw_materials, sys.total_energy), (0, 1));
        assert_eq!(sys.manufacturing_facilities.len(), 0);
        // Without energy, a draw equal to the facilities takes the raw unit.
        fx.world.systems[fx.system].total_energy = 0;
        fx.world.systems[fx.system].raw_materials = 1;
        let out = resolve(&fx.world, &req, &mut script(&[0, 5, 0, 5, 0, 99, 0], 0));
        assert_eq!(out.collateral, vec![Target::Raw]);
        // Nothing to draw: the phase ends.
        fx.world.systems[fx.system].raw_materials = 0;
        let out = resolve(&fx.world, &req, &mut script(&[0, 5, 0, 5, 0, 0], 0));
        assert_eq!(out.collateral, vec![]);
    }

    #[test]
    fn each_draw_takes_its_range_from_the_counts_at_the_start() {
        // FUN_0055ead0 and FUN_0055eb80 draw a percent (0..99,
        // FUN_0053e2e0); a hit's attacker 0..+0x1c - 1, a duel's defender
        // 0..+0x20 - 1, its roll 0..G0x202; the collateral target 0..(F +
        // [energy] + [raw]) - 1 (FUN_0055ebc0). `draw(m)` gives 0..m - 1.
        let mut fx = fixture();
        aboard(&mut fx, 3);
        defending(&mut fx, 2);
        defense(&mut fx, 0x2300_0002, 50);
        fx.world.systems[fx.system].total_energy = 1;
        let req = request(&fx);
        let draws = [0, 0, 0, 6, 0, 6, 0, 50];
        let mut seen = Vec::new();
        let mut at = 0;
        let out = resolve(&fx.world, &req, &mut |m| {
            seen.push(m);
            let d = draws.get(at).copied().unwrap_or(99);
            at += 1;
            d.clamp(0, m - 1)
        });
        // Battery: percent, then the attacker of 3. Duels of the two left:
        // defender of 2, roll of 11 (each wins). One collateral hit of the
        // two duels' percents (the first), drawn over the battery and the
        // energy unit (draw 99: the energy).
        assert_eq!(seen, [100, 3, 2, 11, 2, 11, 100, 100, 2]);
        assert_eq!(out.collateral, vec![Target::Energy]);
        assert_eq!(out.duels, 2);
        // Without energy, a raw unit alone is drawn over 1.
        let mut fx = fixture();
        aboard(&mut fx, 1);
        defending(&mut fx, 1);
        fx.world.systems[fx.system].raw_materials = 1;
        let mut seen = Vec::new();
        let draws = [0, 5, 0, 0];
        let mut at = 0;
        let out = resolve(&fx.world, &request(&fx), &mut |m| {
            seen.push(m);
            let d = draws.get(at).copied().unwrap_or(0);
            at += 1;
            d.clamp(0, m - 1)
        });
        assert_eq!(seen, [1, 11, 100, 1]);
        assert_eq!(out.collateral, vec![Target::Raw]);
    }

    #[test]
    fn an_energy_or_raw_unit_goes_once_and_nothing_left_ends_the_damage() {
        // FUN_00509cc0 lowers +0x5c at once, so a second hit finds none.
        let mut fx = fixture();
        aboard(&mut fx, 2);
        defending(&mut fx, 2);
        fx.world.systems[fx.system].total_energy = 1;
        let out = resolve(
            &fx.world,
            &request(&fx),
            &mut script(&[0, 5, 0, 5, 0, 0, 0], 0),
        );
        assert_eq!(out.collateral, vec![Target::Energy]);
        fx.world.systems[fx.system].total_energy = 0;
        fx.world.systems[fx.system].raw_materials = 1;
        let out = resolve(
            &fx.world,
            &request(&fx),
            &mut script(&[0, 5, 0, 5, 0, 0, 0], 0),
        );
        assert_eq!(out.collateral, vec![Target::Raw]);
    }

    #[test]
    fn no_garrison_base_lands_no_one() {
        // FUN_0055a050: support 0 at or above G0x1e51 needs no garrison.
        let mut fx = fixture();
        fx.world.gnprtb = GnprtbParams::uniform(&[(0x1e51, 0), (0x1e52, -10)]);
        aboard(&mut fx, 2);
        let out = resolve(&fx.world, &request(&fx), &mut script(&[], 0));
        assert_eq!(out.landed, vec![]);
        // A step of 0 divides nothing.
        fx.world.gnprtb = GnprtbParams::uniform(&[(0x1e51, 60), (0x1e52, 0)]);
        let out = resolve(&fx.world, &request(&fx), &mut script(&[], 0));
        assert_eq!(out.landed, vec![]);
    }

    #[test]
    fn the_order_needs_a_fleet_in_orbit_carrying_a_regiment_at_a_system_not_its_own() {
        // FUN_00581ae0: a team fleet carrying a regiment (else 8/5); port:
        // in orbit, out of hyperspace, a standing system not the side's,
        // not fully shielded (two shields; the manual).
        let mut fx = fixture();
        let movement = MovementState::new();
        let enabled = |fx: &Fx| {
            order_enabled(
                &fx.world,
                &movement,
                &fx.transport,
                false,
                &[fx.fleet],
                fx.system,
            )
        };
        assert!(!enabled(&fx));
        aboard(&mut fx, 1);
        assert!(enabled(&fx));
        defense(&mut fx, 0x2400_0003, 0);
        assert!(enabled(&fx));
        defense(&mut fx, 0x2400_0006, 0);
        assert!(!enabled(&fx));
        let mut fx = fixture();
        aboard(&mut fx, 1);
        fx.world.systems[fx.system].control = ControlKind::Controlled(Faction::Empire);
        assert!(!enabled(&fx));
        fx.world.systems[fx.system].control = ControlKind::Uncontrolled;
        assert!(enabled(&fx));
        fx.world.systems[fx.system].is_destroyed = true;
        assert!(!enabled(&fx));
        fx.world.systems[fx.system].is_destroyed = false;
        assert!(!order_enabled(
            &fx.world,
            &movement,
            &fx.transport,
            true,
            &[fx.fleet],
            fx.system
        ));
        assert!(!order_enabled(
            &fx.world,
            &movement,
            &fx.transport,
            false,
            &[],
            fx.system
        ));
    }

    #[test]
    fn the_env_carries_an_assault_out_and_keeps_it() {
        // The tick's draws (port: SplitMix64) resolve and apply the assault.
        let mut fx = fixture();
        let a = aboard(&mut fx, 1);
        let mut env = crate::bombardment::BombardEnv::new(5, None);
        let landed = env
            .assault(
                &mut fx.world,
                &mut fx.transport,
                Faction::Empire,
                &[fx.fleet],
                fx.system,
            )
            .landed
            .clone();
        assert_eq!(landed, a);
        assert_eq!(env.assaults.len(), 1);
        assert!(fx.world.systems[fx.system].ground_units.contains(&a[0]));
    }
}
