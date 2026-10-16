//! Planetary bombardment: the Fleet menu's orders `0x220` Target Military
//! Facilities, `0x221` Target Civilian Facilities, `0x222` General
//! Bombardment and `0x223` Destroy System, resolved as the game does
//! (`FUN_005637c0` -> `FUN_0058d800` -> `FUN_0058d770`;
//! `ghidra/notes/bombardment-order.md`).
//!
//! [`resolve`] reads the world and returns a [`BombardmentOutcome`];
//! [`apply`] carries it out. A Death Star's destruction of the system is
//! left to the caller's Death Star cleanup.
//!
//! port: the port has no officer ranks (`+0x96`), so the general's and the
//! admiral's leadership factors are 1; a ship keeps no shield damage, so
//! its shields are recomputed each time (`FUN_00501160`).

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::ids::ProductionFacilityKey;
use crate::ids::{
    DatId, DefenseFacilityKey, FleetKey, ManufacturingFacilityKey, SystemKey, TroopKey,
};
use crate::world::{FacilityRef, GameWorld};

/// Target Military Facilities (`0x220`, `FUN_00582980`; `FUN_0058d800`).
pub const TARGET_MILITARY: u32 = 0x220;
/// Target Civilian Facilities (`0x221`, `FUN_00582710`; `FUN_0058d800`).
pub const TARGET_CIVILIAN: u32 = 0x221;
/// General Bombardment (`0x222`, `FUN_005828b0`; `FUN_0058d800`).
pub const GENERAL: u32 = 0x222;
/// Destroy System (`0x223`, `FUN_005827e0`; `FUN_0058d800`).
pub const DESTROY_SYSTEM: u32 = 0x223;

/// GNPRTB `0x600` (`DAT_006bb768`, 40): the general's leadership divisor.
const GNPRTB_GENERAL_DIVISOR: u16 = 0x600;
/// GNPRTB `0x601` (`DAT_006bb774`, 40): the admiral's leadership divisor.
const GNPRTB_ADMIRAL_DIVISOR: u16 = 0x601;
/// GNPRTB `0x602` (`DAT_006bb75c`, 1): the destruction draw's low end
/// (`FUN_0055e970`).
const GNPRTB_DRAW_LOW: u16 = 0x602;
/// GNPRTB `0x603` (`DAT_006bb764`, 10): the destruction draw's high end
/// (`FUN_0055e970`).
const GNPRTB_DRAW_HIGH: u16 = 0x603;
/// GNPRTB `0x604` (`DAT_006bb760`, 9): an energy unit's defense.
const GNPRTB_ENERGY_DEFENSE: u16 = 0x604;
/// GNPRTB `0x605` (`DAT_006bb770`, 9): a raw material unit's defense.
const GNPRTB_RAW_DEFENSE: u16 = 0x605;
/// GNPRTB `0x800` (`DAT_006bb388`, 4): the shield allocation scale
/// (`FUN_00501160`).
const GNPRTB_SHIELD_SCALE: u16 = 0x800;
/// GNPRTB `0x1e19` (`DAT_006bb47c`, -20): the attacker's support change
/// after civilian damage (`FUN_0058e320`).
const GNPRTB_CIVILIAN_SUPPORT: u16 = 0x1e19;

/// The ion cannons' family (DEFFACSD 1, KDY v-150): their hits drain a
/// ship's shields and stop it bombarding (`FUN_00527550`).
const ION_CANNONS: u8 = 0x22;
/// The planetary batteries' family (DEFFACSD 2 and 5): their hits past the
/// shields damage the hull (`FUN_005272e0`).
const BATTERIES: u8 = 0x23;
/// The planetary shields' family (DEFFACSD 3 and 6, `FUN_00527940`).
const SHIELDS: u8 = 0x24;

/// One target on the ground, in the order `FUN_0058cba0` walks them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    /// A regiment (families `0x10..0x13`, `FUN_00504cc0`).
    Troop(TroopKey),
    /// A defense facility (`0x22..0x27`, `FUN_00527050`).
    Defense(DefenseFacilityKey),
    /// The Alliance's headquarters (ALLFACSD, `0x20`, `FUN_00526d70`).
    Headquarters,
    /// A manufacturing facility (`0x28..0x2b`, `FUN_0053b760`).
    Manufacturing(ManufacturingFacilityKey),
    /// A production facility (`0x2c..0x2f`, `FUN_0053b760`).
    Production(ProductionFacilityKey),
    /// An energy unit (system `+0x5c`).
    Energy,
    /// A raw material unit (system `+0x64`).
    Raw,
}

impl Target {
    /// Whether its loss is civilian damage (`+0x28`): a manufacturing or
    /// production facility (`FUN_0058ce70`), an energy or raw unit
    /// (`FUN_0058cf20`, `FUN_0058cf50`).
    #[must_use]
    pub const fn is_civilian(self) -> bool {
        matches!(
            self,
            Self::Manufacturing(_) | Self::Production(_) | Self::Energy | Self::Raw
        )
    }
}

/// What one bombardment asks: the attacking side's fleets (the order's
/// team) at `system`, the order's kind, and the Alliance's headquarters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BombardRequest {
    pub system: SystemKey,
    pub side: Faction,
    pub kind: u32,
    pub fleets: Vec<FleetKey>,
    /// The system holding the Alliance's headquarters object, if any.
    pub alliance_hq: Option<SystemKey>,
}

/// A bombarding ship's losses: hull damage, and whether ion hits stopped it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShipLoss {
    pub fleet: FleetKey,
    /// The ship's place in its fleet.
    pub index: usize,
    pub hull_damage: i32,
}

/// What a bombardment did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BombardmentOutcome {
    /// A Death Star destroyed the system (`0x223`, `FUN_0058d990`).
    pub system_destroyed: bool,
    /// The bombarding ships the batteries hurt.
    pub ship_losses: Vec<ShipLoss>,
    /// Firepower less the shields (`FUN_0058df60`).
    pub shots: i32,
    /// The targets destroyed, in order.
    pub destroyed: Vec<Target>,
    /// Civilian damage (`+0x28`): the attacker loses support.
    pub civilian_damage: bool,
}

/// `FUN_0053e290(n)`: a draw from 0 to `n`, from `-n` to 0 when negative
/// (`FUN_005f5700(n + 1)`). `draw(m)` gives 0 to `m - 1`.
fn upto(draw: &mut dyn FnMut(i32) -> i32, n: i32) -> i32 {
    if n >= 0 {
        draw(n.saturating_add(1))
    } else {
        -draw(1i32.saturating_sub(n))
    }
}

/// The side of `system`'s objects (`+0x24 >> 6 & 3`).
fn system_side(world: &GameWorld, system: SystemKey) -> Option<Faction> {
    world
        .systems
        .get(system)
        .and_then(|s| GameWorld::facility_holder(s.control))
}

fn gnprtb(world: &GameWorld, id: u16) -> i32 {
    world.gnprtb.value(id, world.difficulty_index)
}

fn class_stats(world: &GameWorld, class: DatId) -> crate::world::ClassStats {
    world
        .buildable_classes
        .get(&class)
        .map(|c| c.stats)
        .unwrap_or_default()
}

fn int(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// The bombarding ships (`FUN_0058ca70`): the request's fleets of the side
/// in the system's order, each fleet's living ships in its order.
fn bombarding_ships(world: &GameWorld, req: &BombardRequest) -> Vec<(FleetKey, usize)> {
    let Some(system) = world.systems.get(req.system) else {
        return Vec::new();
    };
    system
        .fleets
        .iter()
        .filter(|f| req.fleets.contains(f))
        .filter_map(|&f| world.fleets.get(f).map(|fleet| (f, fleet)))
        .filter(|(_, fleet)| Faction::of_alliance(fleet.is_alliance) == req.side)
        .flat_map(|(f, fleet)| {
            fleet
                .capital_ships
                .iter()
                .enumerate()
                .filter(|(_, s)| s.alive)
                .map(move |(i, _)| (f, i))
        })
        .collect()
}

/// The targets of `kind` at the system (`FUN_0058cba0`), each of the
/// system's side: with `0x220`, `0x222`, `0x223` its regiments, defense
/// facilities and the headquarters; with `0x221`, `0x222`, `0x223` its
/// manufacturing and production facilities; with `0x222`, `0x223` an energy
/// and a raw material unit.
///
/// hyp: the system's list holds the manufacturing facilities before the
/// production ones (`FUN_0053b760` walks families `0x28..0x2f`).
fn targets(world: &GameWorld, req: &BombardRequest) -> Vec<Target> {
    let mut out = Vec::new();
    let (Some(system), Some(side)) = (
        world.systems.get(req.system),
        system_side(world, req.system),
    ) else {
        return out;
    };
    let ground = matches!(req.kind, TARGET_MILITARY | GENERAL | DESTROY_SYSTEM);
    let facilities = matches!(req.kind, TARGET_CIVILIAN | GENERAL | DESTROY_SYSTEM);
    let resources = matches!(req.kind, GENERAL | DESTROY_SYSTEM);
    if ground {
        for &t in &system.ground_units {
            if world
                .troops
                .get(t)
                .is_some_and(|u| Faction::of_alliance(u.is_alliance) == side)
            {
                out.push(Target::Troop(t));
            }
        }
        for &d in &system.defense_facilities {
            if world
                .defense_facilities
                .get(d)
                .is_some_and(|f| f.side == side)
            {
                out.push(Target::Defense(d));
            }
        }
        if system.is_headquarters
            && req.alliance_hq == Some(req.system)
            && side == Faction::Alliance
        {
            out.push(Target::Headquarters);
        }
    }
    if facilities {
        out.extend(civilian_facilities(world, req.system, side));
    }
    if resources {
        if system.total_energy > 0 {
            out.push(Target::Energy);
        }
        if system.raw_materials > 0 {
            out.push(Target::Raw);
        }
    }
    out
}

/// The system's manufacturing and production facilities of `side`
/// (`FUN_0053b760`).
fn civilian_facilities(world: &GameWorld, system: SystemKey, side: Faction) -> Vec<Target> {
    let Some(sys) = world.systems.get(system) else {
        return Vec::new();
    };
    let manufacturing = sys
        .manufacturing_facilities
        .iter()
        .filter(|&&k| {
            world
                .manufacturing_facilities
                .get(k)
                .is_some_and(|f| f.side == side)
        })
        .map(|&k| Target::Manufacturing(k));
    let production = sys
        .production_facilities
        .iter()
        .filter(|&&k| {
            world
                .production_facilities
                .get(k)
                .is_some_and(|f| f.side == side)
        })
        .map(|&k| Target::Production(k));
    manufacturing.chain(production).collect()
}

/// `FUN_0058cf80`: a target's defense: a facility's or the headquarters'
/// class `bombardment_defense` (`+0x58`), a regiment's TROOPSD
/// `bombardment_defense` (`+0x60`), GNPRTB `0x604` for energy and `0x605`
/// for raw materials.
fn defense(world: &GameWorld, target: Target) -> i32 {
    let class = match target {
        Target::Troop(k) => world.troops.get(k).map(|t| t.class_dat_id),
        Target::Defense(k) => world.defense_facilities.get(k).map(|f| f.class_dat_id),
        Target::Headquarters => Some(DatId::new(HEADQUARTERS_CLASS)),
        Target::Manufacturing(k) => world
            .manufacturing_facilities
            .get(k)
            .map(|f| f.class_dat_id),
        Target::Production(k) => world.production_facilities.get(k).map(|f| f.class_dat_id),
        Target::Energy => return gnprtb(world, GNPRTB_ENERGY_DEFENSE),
        Target::Raw => return gnprtb(world, GNPRTB_RAW_DEFENSE),
    };
    class.map_or(0, |c| int(class_stats(world, c).bombardment))
}

/// The Alliance headquarters' class (ALLFACSD 1, family `0x20`,
/// `FUN_00526d70`).
const HEADQUARTERS_CLASS: u32 = 0x2000_0001;

/// `FUN_0055e970` with `FUN_0053e2c0`: destroyed when `defense` is below a
/// draw from GNPRTB `0x602` to `0x603`.
fn destroyed(world: &GameWorld, defense: i32, draw: &mut dyn FnMut(i32) -> i32) -> bool {
    let low = gnprtb(world, GNPRTB_DRAW_LOW);
    let high = gnprtb(world, GNPRTB_DRAW_HIGH);
    defense < upto(draw, high - low) + low
}

/// A ship's state during the bombardment.
#[derive(Debug, Clone, Copy)]
struct Bombarder {
    fleet: FleetKey,
    index: usize,
    /// Ion hits (`+0x54` -> `+0x20`).
    ion: i32,
    /// Battery hits (`+0x54` -> `+0x24`).
    battery: i32,
    hull_damage: i32,
}

/// `FUN_00501160`: a ship's shields: the class's, scaled by GNPRTB `0x800`
/// less the `+0x64` low nibble over GNPRTB `0x800`, then by its hull left.
///
/// hyp: the port's unset packed word (0) reads as a nibble of 0.
fn shields(world: &GameWorld, ship: &crate::world::ShipInstance, damage: i32) -> i32 {
    let Some(class) = world.capital_ship_classes.get(ship.class) else {
        return 0;
    };
    let full = int(class.shield_strength);
    let scale = gnprtb(world, GNPRTB_SHIELD_SCALE);
    let hull = int(class.hull);
    if scale == 0 || hull == 0 {
        return full;
    }
    let allocated = full.wrapping_mul(scale - i32::from(ship.shield_nibble())) / scale;
    allocated.wrapping_mul(hull - damage) / hull
}

/// `FUN_0055e9a0(a, b)`: the excess of one over the other, `(a - b, 0)` or
/// `(0, b - a)`.
const fn excess(a: i32, b: i32) -> (i32, i32) {
    if a > b {
        (a - b, 0)
    } else {
        (0, b - a)
    }
}

/// Resolve one bombardment (`FUN_0058d770`), drawing with `draw(m)` (0 to
/// `m - 1`).
#[expect(
    clippy::too_many_lines,
    reason = "The four phases in FUN_0058d770's order."
)]
pub fn resolve(
    world: &GameWorld,
    req: &BombardRequest,
    draw: &mut dyn FnMut(i32) -> i32,
) -> BombardmentOutcome {
    let mut out = BombardmentOutcome::default();
    let ships = bombarding_ships(world, req);
    // Phase 1 (FUN_0058d990): a Death Star among the ships destroys the
    // system; hyp: its objects go with it, so no battery fires and nothing
    // is left to bombard.
    if req.kind == DESTROY_SYSTEM
        && ships.iter().any(|&(f, i)| {
            world.capital_ship_classes[world.fleets[f].capital_ships[i].class].is_death_star()
        })
    {
        out.system_destroyed = true;
        out.civilian_damage = true;
        return out;
    }
    let targets_at_start = i32::try_from(targets(world, req).len()).unwrap_or(i32::MAX);
    let mut bombarders: Vec<Bombarder> = ships
        .iter()
        .map(|&(fleet, index)| {
            let ship = &world.fleets[fleet].capital_ships[index];
            let hull = world
                .capital_ship_classes
                .get(ship.class)
                .map_or(0, |c| int(c.hull));
            Bombarder {
                fleet,
                index,
                ion: 0,
                battery: 0,
                hull_damage: hull - ship.hull_current,
            }
        })
        .collect();
    let count = i32::try_from(bombarders.len()).unwrap_or(i32::MAX);
    // Phase 2 (FUN_0058dc10): each ion cannon, then each battery, of any
    // side hits a drawn ship with (general / G0x600 + 1) * its attack.
    let general = 0;
    let factor = general / gnprtb(world, GNPRTB_GENERAL_DIVISOR).max(1) + 1;
    let Some(system) = world.systems.get(req.system) else {
        return out;
    };
    for family in [ION_CANNONS, BATTERIES] {
        for &d in &system.defense_facilities {
            let Some(facility) = world.defense_facilities.get(d) else {
                continue;
            };
            if facility.class_dat_id.family() != family {
                continue;
            }
            let k = upto(draw, count - 1);
            let Some(ship) = usize::try_from(k).ok().and_then(|k| bombarders.get_mut(k)) else {
                continue;
            };
            let hit = factor.wrapping_mul(int(
                class_stats(world, facility.class_dat_id).attack_strength
            ));
            if family == ION_CANNONS {
                ship.ion = ship.ion.wrapping_add(hit);
            } else {
                ship.battery = ship.battery.wrapping_add(hit);
            }
        }
    }
    // FUN_0058d540: the shields take the ion hits, then the battery hits;
    // the battery hits past them damage the hull up to what it has left. A
    // ship with ion hits past its shields cannot bombard (FUN_00500f00).
    for b in &mut bombarders {
        let ship = &world.fleets[b.fleet].capital_ships[b.index];
        let hull = world
            .capital_ship_classes
            .get(ship.class)
            .map_or(0, |c| int(c.hull));
        let (ion, left) = excess(b.ion, shields(world, ship, b.hull_damage));
        let (battery, _) = excess(b.battery, left);
        let hull_left = hull - b.hull_damage;
        let (_, after) = excess(battery, hull_left);
        let dealt = hull_left - after;
        b.ion = ion;
        if dealt != 0 {
            b.hull_damage += dealt;
            out.ship_losses.push(ShipLoss {
                fleet: b.fleet,
                index: b.index,
                hull_damage: dealt,
            });
        }
    }
    // No bombarding ship left: the bombardment stops (+0x30).
    let alive = |b: &Bombarder| {
        world
            .capital_ship_classes
            .get(world.fleets[b.fleet].capital_ships[b.index].class)
            .is_some_and(|c| b.hull_damage < int(c.hull))
    };
    if !bombarders.iter().any(alive) {
        return out;
    }
    // Phase 3 (FUN_0058df60): firepower less the shields of the system's
    // side gives the shots.
    let admiral = 0;
    let factor = admiral / gnprtb(world, GNPRTB_ADMIRAL_DIVISOR).max(1) + 1;
    let mut firepower = 0i32;
    for &fleet in req.fleets.iter().filter(|f| system.fleets.contains(f)) {
        let Some(value) = world.fleets.get(fleet) else {
            continue;
        };
        if Faction::of_alliance(value.is_alliance) != req.side {
            continue;
        }
        let mut sum = 0i32;
        for b in bombarders.iter().filter(|b| b.fleet == fleet && alive(b)) {
            sum = sum.wrapping_add(ship_bombardment(world, b));
        }
        for entry in &value.fighters {
            let per = world
                .fighter_classes
                .get(entry.class)
                .map_or(0, |c| int(c.bombardment_defense));
            sum = sum.wrapping_add(per.wrapping_mul(int(entry.count)));
        }
        firepower = firepower.wrapping_add(factor.wrapping_mul(sum));
    }
    let side = system_side(world, req.system);
    let shields: i32 = system
        .defense_facilities
        .iter()
        .filter_map(|&d| world.defense_facilities.get(d))
        .filter(|f| f.class_dat_id.family() == SHIELDS && Some(f.side) == side)
        .map(|f| int(class_stats(world, f.class_dat_id).shield_strength))
        .fold(0i32, i32::wrapping_add);
    out.shots = if firepower > shields {
        firepower - shields
    } else {
        0
    };
    let mut gone: Vec<Target> = Vec::new();
    let live = |gone: &[Target]| -> Vec<Target> {
        targets(world, req)
            .into_iter()
            .filter(|t| !gone.contains(t))
            .collect()
    };
    // Target Military Facilities: collateral damage with probability
    // 1 / (targets + 1); the decompile takes the first civilian facility
    // (FUN_0058d080, its walk never advancing).
    if out.shots > 0 && req.kind == TARGET_MILITARY && upto(draw, targets_at_start) < 1 {
        let civilian = side.map_or_else(Vec::new, |s| civilian_facilities(world, req.system, s));
        if !civilian.is_empty() {
            let n = i32::try_from(civilian.len()).unwrap_or(i32::MAX);
            let _ = upto(draw, n - 1);
        }
        let first = civilian.first().copied();
        let d = first.map_or(0, |t| defense(world, t));
        if destroyed(world, d, draw) {
            if let Some(t) = first {
                gone.push(t);
            }
        }
    }
    // Each shot draws from the targets counted at the start (+0x20); a
    // draw past the targets left finds none. The shots stop once a
    // destruction leaves no target (+0x30).
    let mut left = live(&gone);
    for _ in 0..out.shots {
        let n = upto(draw, targets_at_start - 1);
        let Some(&t) = usize::try_from(n).ok().and_then(|n| left.get(n)) else {
            continue;
        };
        if destroyed(world, defense(world, t), draw) {
            gone.push(t);
            left = live(&gone);
            if left.is_empty() {
                break;
            }
        }
    }
    out.civilian_damage = gone.iter().any(|t| t.is_civilian());
    out.destroyed = gone;
    out
}

/// A ship's bombardment (slot `+0x1dc`, `FUN_00501470`): its class's
/// `bombardment_modifier` less its share of hull damage (`FUN_00500f00`),
/// none with ion hits past its shields.
///
/// hyp: the ship's `+0x24 & 0x30` is 0, so the ion hits count.
fn ship_bombardment(world: &GameWorld, b: &Bombarder) -> i32 {
    let ship = &world.fleets[b.fleet].capital_ships[b.index];
    let Some(class) = world.capital_ship_classes.get(ship.class) else {
        return 0;
    };
    let modifier = int(class.bombardment_modifier);
    let hull = int(class.hull);
    if b.ion != 0 || hull == 0 {
        return 0;
    }
    modifier - modifier.wrapping_mul(b.hull_damage) / hull
}

/// `FUN_004fd6b0`: a fleet's bombardment, each living ship's (its class's
/// `bombardment_modifier` less its share of hull damage, `FUN_00501470`)
/// and each squadron's aboard (FIGHTSD `+0xcc`). The Fleet menu's orders
/// need a fleet with some (`FUN_00582440`).
#[must_use]
pub fn fleet_bombardment(world: &GameWorld, fleet: FleetKey) -> i32 {
    let Some(value) = world.fleets.get(fleet) else {
        return 0;
    };
    let ships = value
        .capital_ships
        .iter()
        .enumerate()
        .filter(|(_, s)| s.alive)
        .map(|(index, s)| {
            let hull = world
                .capital_ship_classes
                .get(s.class)
                .map_or(0, |c| int(c.hull));
            ship_bombardment(
                world,
                &Bombarder {
                    fleet,
                    index,
                    ion: 0,
                    battery: 0,
                    hull_damage: hull - s.hull_current,
                },
            )
        })
        .fold(0i32, i32::wrapping_add);
    value.fighters.iter().fold(ships, |sum, entry| {
        let per = world
            .fighter_classes
            .get(entry.class)
            .map_or(0, |c| int(c.bombardment_defense));
        sum.wrapping_add(per.wrapping_mul(int(entry.count)))
    })
}

/// Carry out `outcome` of `req`: the ships' hull damage (a ship at its
/// hull is lost, `FUN_00501ae0`; a fleet left with nothing disbands), the
/// targets destroyed, and the attacker's support after civilian damage
/// (GNPRTB `0x1e19`, `FUN_0050c9f0`). The system's destruction is the
/// caller's.
///
/// port: the sector's and the galaxy's support events (`FUN_00518b20`) are
/// not ported.
pub fn apply(world: &mut GameWorld, req: &BombardRequest, outcome: &BombardmentOutcome) {
    let mut lost: Vec<(FleetKey, usize)> = Vec::new();
    for loss in &outcome.ship_losses {
        let Some(ship) = world
            .fleets
            .get_mut(loss.fleet)
            .and_then(|f| f.capital_ships.get_mut(loss.index))
        else {
            continue;
        };
        ship.hull_current = (ship.hull_current - loss.hull_damage).max(0);
        if ship.hull_current == 0 {
            ship.alive = false;
            lost.push((loss.fleet, loss.index));
        }
    }
    lost.sort_by_key(|&(_, index)| std::cmp::Reverse(index));
    for (fleet, index) in lost {
        crate::scrap::remove_ship(world, fleet, index);
    }
    for &target in &outcome.destroyed {
        match target {
            Target::Troop(key) => {
                if let Some(sys) = world.systems.get_mut(req.system) {
                    sys.ground_units.retain(|&k| k != key);
                }
                world.troops.remove(key);
            }
            Target::Defense(key) => world.remove_facility(req.system, FacilityRef::Defense(key)),
            Target::Manufacturing(key) => {
                world.remove_facility(req.system, FacilityRef::Manufacturing(key));
            }
            Target::Production(key) => {
                world.remove_facility(req.system, FacilityRef::Production(key));
            }
            Target::Headquarters => {
                if let Some(sys) = world.systems.get_mut(req.system) {
                    sys.is_headquarters = false;
                }
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
        }
    }
    if outcome.civilian_damage && !outcome.system_destroyed {
        let delta = gnprtb(world, GNPRTB_CIVILIAN_SUPPORT);
        crate::uprising::apply_support_change(world, req.system, req.side, delta);
    }
}

/// One tick's bombardments: their draws, the Alliance's headquarters, and
/// what they did, for the caller to report and to destroy the systems a
/// Death Star destroyed.
///
/// port: the draws come from a `SplitMix64` stream seeded by the tick's
/// generator (the original draws from its one game generator,
/// `FUN_005f5700`).
#[derive(Debug, Clone)]
pub struct BombardEnv {
    state: u64,
    /// The system holding the Alliance's headquarters object.
    pub alliance_hq: Option<SystemKey>,
    /// Each bombardment carried out, in order.
    pub outcomes: Vec<(BombardRequest, BombardmentOutcome)>,
}

impl BombardEnv {
    #[must_use]
    pub fn new(seed: u64, alliance_hq: Option<SystemKey>) -> Self {
        Self {
            state: seed,
            alliance_hq,
            outcomes: Vec::new(),
        }
    }

    /// A draw from 0 to `m - 1` (`m` at least 1). port: `SplitMix64` in
    /// place of `FUN_005f5700`.
    pub fn draw(&mut self, m: i32) -> i32 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        let m = u64::try_from(m.max(1)).unwrap_or(1);
        i32::try_from(z % m).unwrap_or(0)
    }

    /// Resolve and carry out the bombardment of `system` by `side`'s
    /// `fleets` with `kind`; the outcome is kept.
    pub fn bombard(
        &mut self,
        world: &mut GameWorld,
        side: Faction,
        fleets: &[FleetKey],
        system: SystemKey,
        kind: u32,
    ) -> &BombardmentOutcome {
        let req = BombardRequest {
            system,
            side,
            kind,
            fleets: fleets.to_vec(),
            alliance_hq: self.alliance_hq,
        };
        let outcome = resolve(world, &req, &mut |m| self.draw(m));
        apply(world, &req, &outcome);
        self.outcomes.push((req, outcome));
        &self.outcomes[self.outcomes.len() - 1].1
    }

    /// The systems a Death Star destroyed.
    #[must_use]
    pub fn destroyed_systems(&self) -> Vec<SystemKey> {
        self.outcomes
            .iter()
            .filter(|(_, o)| o.system_destroyed)
            .map(|(r, _)| r.system)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{ExplorationStatus, SectorGroup};
    use crate::ids::CapitalShipKey;
    use crate::world::{
        BuildableClass, CapitalShipClass, ClassStats, ControlKind, DefenseFacilityInstance,
        FighterClass, FighterEntry, Fleet, GnprtbParams, ManufacturingFacilityInstance,
        ProductionFacilityInstance, Sector, ShipInstance, System, TroopUnit,
    };

    /// GNPRTB as shipped: 0x600..0x605, 0x800, 0x1e19 (bombardment-order.md).
    fn gnprtb() -> GnprtbParams {
        GnprtbParams::uniform(&[
            (0x600, 40),
            (0x601, 40),
            (0x602, 1),
            (0x603, 10),
            (0x604, 9),
            (0x605, 9),
            (0x800, 4),
            (0x1e19, -20),
        ])
    }

    struct Fx {
        world: GameWorld,
        system: SystemKey,
        fleet: FleetKey,
        ship_class: CapitalShipKey,
    }

    fn class(world: &mut GameWorld, raw: u32, stats: ClassStats) -> DatId {
        let id = DatId::new(raw);
        world.buildable_classes.insert(
            id,
            BuildableClass {
                stats,
                ..BuildableClass::default()
            },
        );
        id
    }

    fn defense(
        world: &mut GameWorld,
        system: SystemKey,
        raw: u32,
        stats: ClassStats,
    ) -> DefenseFacilityKey {
        let class_dat_id = class(world, raw, stats);
        let key = world.defense_facilities.insert(DefenseFacilityInstance {
            class_dat_id,
            side: Faction::Alliance,
        });
        world.systems[system].defense_facilities.push(key);
        key
    }

    fn troop(world: &mut GameWorld, system: SystemKey, bombardment: u32) -> TroopKey {
        let class_dat_id = class(
            world,
            0x1000_0001,
            ClassStats {
                bombardment,
                ..ClassStats::default()
            },
        );
        let key = world.troops.insert(TroopUnit {
            class_dat_id,
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(key);
        key
    }

    /// An Alliance system (support 50) with an Imperial fleet of `ships`
    /// ships of hull 100, shields 10, bombardment 20 in orbit.
    fn fixture(ships: u32) -> Fx {
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
            shield_strength: 10,
            bombardment_modifier: 20,
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: ShipInstance::make(ship_class, 100, false, ships),
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[system].fleets.push(fleet);
        Fx {
            world,
            system,
            fleet,
            ship_class,
        }
    }

    fn request(fx: &Fx, kind: u32) -> BombardRequest {
        BombardRequest {
            system: fx.system,
            side: Faction::Empire,
            kind,
            fleets: vec![fx.fleet],
            alliance_hq: None,
        }
    }

    /// Draws from a script, then `fill`.
    fn script(draws: &[i32], fill: i32) -> impl FnMut(i32) -> i32 + '_ {
        let mut at = 0;
        move |m| {
            let d = draws.get(at).copied().unwrap_or(fill);
            at += 1;
            d.clamp(0, m - 1)
        }
    }

    const ION: u32 = 0x2200_0001;
    const BATTERY: u32 = 0x2300_0002;
    const SHIELD: u32 = 0x2400_0003;

    fn attack(a: u32) -> ClassStats {
        ClassStats {
            attack_strength: a,
            ..ClassStats::default()
        }
    }

    #[test]
    fn the_order_s_kind_chooses_the_target_classes_in_their_walk_order() {
        use Target::{Defense, Energy, Headquarters, Manufacturing, Production, Raw, Troop};
        // FUN_0058d800: 0x220 ground (+0x34); 0x221 facilities (+0x38);
        // 0x222 and 0x223 all three with the resources (+0x3c).
        // FUN_0058cba0 walks regiments, defense facilities, the
        // headquarters, then the manufacturing and production facilities,
        // then energy (+0x5c) and raw materials (+0x64); each of the
        // system's side.
        let mut fx = fixture(1);
        let t = troop(&mut fx.world, fx.system, 3);
        let d = defense(&mut fx.world, fx.system, BATTERY, attack(10));
        let yard_class = class(&mut fx.world, 0x2800_0001, ClassStats::default());
        let yard = fx
            .world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: yard_class,
                side: Faction::Alliance,
                is_shipyard: true,
            });
        fx.world.systems[fx.system]
            .manufacturing_facilities
            .push(yard);
        let mine_class = class(&mut fx.world, 0x2c00_0001, ClassStats::default());
        let mine = fx
            .world
            .production_facilities
            .insert(ProductionFacilityInstance {
                class_dat_id: mine_class,
                side: Faction::Alliance,
                is_mine: true,
            });
        fx.world.systems[fx.system].production_facilities.push(mine);
        // Another side's regiment is no target.
        let theirs = troop(&mut fx.world, fx.system, 3);
        fx.world.troops[theirs].is_alliance = false;
        fx.world.systems[fx.system].total_energy = 2;
        fx.world.systems[fx.system].raw_materials = 1;
        fx.world.systems[fx.system].is_headquarters = true;
        let at = |kind| {
            let mut req = request(&fx, kind);
            req.alliance_hq = Some(fx.system);
            targets(&fx.world, &req)
        };
        assert_eq!(at(TARGET_MILITARY), [Troop(t), Defense(d), Headquarters]);
        assert_eq!(at(TARGET_CIVILIAN), [Manufacturing(yard), Production(mine)]);
        let all = [
            Troop(t),
            Defense(d),
            Headquarters,
            Manufacturing(yard),
            Production(mine),
            Energy,
            Raw,
        ];
        assert_eq!(at(GENERAL), all);
        assert_eq!(at(DESTROY_SYSTEM), all);
        // The headquarters only at the Alliance's HQ system, held by it.
        assert_eq!(
            targets(&fx.world, &request(&fx, TARGET_MILITARY)),
            [Troop(t), Defense(d)]
        );
        fx.world.systems[fx.system].control = ControlKind::Controlled(Faction::Empire);
        let mut req = request(&fx, TARGET_MILITARY);
        req.alliance_hq = Some(fx.system);
        assert_eq!(targets(&fx.world, &req), [Troop(theirs)]);
    }

    #[test]
    fn firepower_less_the_planet_s_shields_gives_the_shots() {
        // FUN_0058df60: each ship's bombardment modifier (FUN_00501470) and
        // each squadron's FIGHTSD +0xcc, times (admiral / G0x601 + 1),
        // less the system's family 0x24 shields (class +0x60).
        let mut fx = fixture(2);
        let squadron = fx.world.fighter_classes.insert(FighterClass {
            bombardment_defense: 3,
            ..FighterClass::default()
        });
        fx.world.fleets[fx.fleet].fighters.push(FighterEntry {
            class: squadron,
            count: 4,
        });
        defense(
            &mut fx.world,
            fx.system,
            SHIELD,
            ClassStats {
                shield_strength: 15,
                ..ClassStats::default()
            },
        );
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[], 9));
        assert_eq!(out.shots, 20 + 20 + 12 - 15);
        assert_eq!(fleet_bombardment(&fx.world, fx.fleet), 52);
        // A ship's share of hull damage comes off its modifier: hull 75 of
        // 100 leaves 20 - 20 * 25 / 100 = 15.
        fx.world.fleets[fx.fleet].capital_ships[0].hull_current = 75;
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[], 9));
        assert_eq!(fleet_bombardment(&fx.world, fx.fleet), 47);
        assert_eq!(out.shots, 15 + 20 + 12 - 15);
        // Another side's shield does not count (FUN_00527940, side +0x18).
        let theirs = defense(
            &mut fx.world,
            fx.system,
            0x2400_0006,
            ClassStats {
                shield_strength: 30,
                ..ClassStats::default()
            },
        );
        fx.world.defense_facilities[theirs].side = Faction::Empire;
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[], 9));
        assert_eq!(out.shots, 15 + 20 + 12 - 15);
        // Shields at or above the firepower: no shot.
        fx.world
            .buildable_classes
            .get_mut(&DatId::new(SHIELD))
            .unwrap()
            .stats
            .shield_strength = 47;
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[], 9));
        assert_eq!(out.shots, 0);
    }

    #[test]
    fn ion_cannons_drain_shields_and_stop_a_ship_bombarding() {
        // FUN_0058dc10: each family 0x22 facility hits a drawn ship with
        // (general / G0x600 + 1) * attack into its first count;
        // FUN_0058d540: the shields absorb it, the excess stays, and
        // FUN_00500f00 then leaves the ship no bombardment.
        let mut fx = fixture(2);
        // The first ship at half hull: shields 5, bombardment 10.
        fx.world.fleets[fx.fleet].capital_ships[0].hull_current = 50;
        defense(&mut fx.world, fx.system, ION, attack(10));
        // Shields 10 absorb 10: the second ship still bombards.
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[1], 9));
        assert_eq!((out.shots, out.ship_losses.len()), (30, 0));
        // 11 is one past them: the drawn ship (the second) is stopped.
        fx.world
            .buildable_classes
            .get_mut(&DatId::new(ION))
            .unwrap()
            .stats
            .attack_strength = 11;
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[1], 9));
        assert_eq!((out.shots, out.ship_losses.len()), (10, 0));
        // The first drawn instead.
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[0], 9));
        assert_eq!(out.shots, 20);
        // The cannon draws among the two ships (FUN_0053e290(+0x1c - 1));
        // then each shot draws among no target (FUN_0053e290(-1)).
        let mut sizes = Vec::new();
        resolve(&fx.world, &request(&fx, TARGET_CIVILIAN), &mut |m| {
            sizes.push(m);
            0
        });
        assert_eq!(sizes[0], 2);
    }

    #[test]
    fn batteries_past_the_shields_damage_the_hull_up_to_what_is_left() {
        // FUN_0058d540: battery hits (family 0x23) after the shields take
        // the ion hits; the excess hits the hull, at most its hull left;
        // the ship's shields are its class's scaled by the +0x64 nibble
        // over G0x800 and by its hull left (FUN_00501160).
        let mut fx = fixture(1);
        let ion = defense(&mut fx.world, fx.system, ION, attack(4));
        defense(&mut fx.world, fx.system, BATTERY, attack(30));
        let out = resolve(
            &fx.world,
            &request(&fx, TARGET_CIVILIAN),
            &mut script(&[0, 0], 9),
        );
        // Shields 10 take the 4 ion and 6 of the 30: 24 on the hull.
        assert_eq!(
            out.ship_losses,
            [ShipLoss {
                fleet: fx.fleet,
                index: 0,
                hull_damage: 24
            }]
        );
        // The damaged ship bombards with its share: 20 - 20 * 24 / 100.
        assert_eq!(out.shots, 16);
        // Hull 50 of 100 halves the shields (5) and caps the damage at 50:
        // 4 ion, 1 more absorbed, 29 to the hull.
        fx.world.fleets[fx.fleet].capital_ships[0].hull_current = 50;
        let out = resolve(
            &fx.world,
            &request(&fx, TARGET_CIVILIAN),
            &mut script(&[0, 0], 9),
        );
        assert_eq!(out.ship_losses[0].hull_damage, 29);
        // A nibble of 2: 10 * (4 - 2) / 4 = 5, * 50 / 100 = 2, all taken by
        // the ion hits: the whole 30 on the hull.
        fx.world.fleets[fx.fleet].capital_ships[0].set_shield_nibble(2);
        let out = resolve(
            &fx.world,
            &request(&fx, TARGET_CIVILIAN),
            &mut script(&[0, 0], 9),
        );
        assert_eq!(out.ship_losses[0].hull_damage, 30);
        // More than the hull left: the hull left, and the ship is lost.
        fx.world
            .buildable_classes
            .get_mut(&DatId::new(BATTERY))
            .unwrap()
            .stats
            .attack_strength = 500;
        let mut fx2 = fx;
        let out = resolve(
            &fx2.world,
            &request(&fx2, TARGET_CIVILIAN),
            &mut script(&[0, 0], 9),
        );
        assert_eq!(out.ship_losses[0].hull_damage, 50);
        // With no bombarding ship left the bombardment stops (+0x30), the
        // fleet's squadrons too (they are aboard its ships).
        assert_eq!(out.shots, 0);
        let squadron = fx2.world.fighter_classes.insert(FighterClass {
            bombardment_defense: 3,
            ..FighterClass::default()
        });
        fx2.world.fleets[fx2.fleet].fighters.push(FighterEntry {
            class: squadron,
            count: 4,
        });
        let with_squadrons = resolve(
            &fx2.world,
            &request(&fx2, TARGET_CIVILIAN),
            &mut script(&[0, 0], 9),
        );
        assert_eq!(with_squadrons.shots, 0);
        fx2.world.fleets[fx2.fleet].fighters.clear();
        let req = request(&fx2, TARGET_CIVILIAN);
        apply(&mut fx2.world, &req, &out);
        assert!(!fx2.world.fleets.contains_key(fx2.fleet));
        let _ = ion;
    }

    #[test]
    fn a_shot_destroys_its_drawn_target_when_its_defense_loses_the_draw() {
        // FUN_0058df60: a target drawn from those counted at the start; its
        // bombardment defense (FUN_0058cf80) against a draw of G0x602..G0x603
        // (FUN_0055e970: destroyed when below it).
        let mut fx = fixture(1);
        fx.world.capital_ship_classes[fx.ship_class].bombardment_modifier = 1;
        let a = troop(&mut fx.world, fx.system, 5);
        let b = troop(&mut fx.world, fx.system, 5);
        // One shot at the second (draw 1); draw 4 gives 5, not above 5.
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[1, 4], 9));
        assert_eq!((out.shots, out.destroyed.len()), (1, 0));
        // Draw 5 gives 6: destroyed.
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[1, 5], 9));
        assert_eq!(out.destroyed, [Target::Troop(b)]);
        let req = request(&fx, GENERAL);
        apply(&mut fx.world, &req, &out);
        assert_eq!(fx.world.systems[fx.system].ground_units, [a]);
        assert!(!fx.world.troops.contains_key(b));
        assert!(!out.civilian_damage);
    }

    #[test]
    fn shots_draw_from_the_targets_counted_at_the_start_until_none_are_left() {
        // FUN_0058df60 draws each shot from +0x20, the count before any
        // shot; a draw past the targets left finds none and rolls nothing.
        // A destruction that leaves none stops the shots.
        let mut fx = fixture(1);
        fx.world.capital_ship_classes[fx.ship_class].bombardment_modifier = 3;
        let a = troop(&mut fx.world, fx.system, 0);
        let b = troop(&mut fx.world, fx.system, 0);
        // Shot 1 takes b (draws 1, 0); shot 2 draws 1, past the one left;
        // shot 3 takes a (draws 0, 0).
        let mut draws = Vec::new();
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut |m| {
            draws.push(m);
            [1, 0, 1, 0, 0][draws.len() - 1].min(m - 1)
        });
        assert_eq!(out.destroyed, [Target::Troop(b), Target::Troop(a)]);
        assert_eq!(draws, [2, 10, 2, 2, 10]);
        // With no target at all every shot still draws.
        let mut fx = fixture(1);
        fx.world.capital_ship_classes[fx.ship_class].bombardment_modifier = 3;
        let mut n = 0;
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut |_| {
            n += 1;
            0
        });
        assert_eq!((out.shots, n), (3, 3));
        // The last target's destruction ends the shots: 2 draws, not 5.
        let mut fx = fixture(1);
        fx.world.capital_ship_classes[fx.ship_class].bombardment_modifier = 4;
        troop(&mut fx.world, fx.system, 0);
        let mut n = 0;
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut |_| {
            n += 1;
            0
        });
        assert_eq!((out.shots, out.destroyed.len(), n), (4, 1, 2));
    }

    #[test]
    fn military_targeting_risks_the_first_civilian_facility() {
        // FUN_0058df60 with 0x220: a draw of 0..=targets below 1 sends a
        // shot-free roll at the first civilian facility (FUN_0058d080,
        // whose walk never advances); its loss is civilian damage, which
        // costs the attacker G0x1e19 (-20) support (FUN_0050c9f0).
        let mut fx = fixture(1);
        fx.world.capital_ship_classes[fx.ship_class].bombardment_modifier = 1;
        let _ = troop(&mut fx.world, fx.system, 10);
        let yard_class = class(&mut fx.world, 0x2800_0001, ClassStats::default());
        let first = fx
            .world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: yard_class,
                side: Faction::Alliance,
                is_shipyard: true,
            });
        let second = fx
            .world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: yard_class,
                side: Faction::Alliance,
                is_shipyard: true,
            });
        fx.world.systems[fx.system].manufacturing_facilities = vec![first, second];
        // Collateral draw 0 of 0..=1; the facility draw 1 is ignored; the
        // roll 0 (1) beats defense 0; the shot draws 0, the roll fails.
        let out = resolve(
            &fx.world,
            &request(&fx, TARGET_MILITARY),
            &mut script(&[0, 1, 0, 0, 0], 0),
        );
        assert_eq!(out.destroyed, [Target::Manufacturing(first)]);
        assert!(out.civilian_damage);
        let req = request(&fx, TARGET_MILITARY);
        apply(&mut fx.world, &req, &out);
        assert_eq!(
            fx.world.systems[fx.system].manufacturing_facilities,
            [second]
        );
        assert!((fx.world.systems[fx.system].popularity_empire - 0.3).abs() < 1e-6);
        assert!((fx.world.systems[fx.system].popularity_alliance - 0.7).abs() < 1e-6);
        // Collateral draw 1: none.
        let out = resolve(
            &fx.world,
            &request(&fx, TARGET_MILITARY),
            &mut script(&[1, 0, 9], 9),
        );
        assert_eq!(out.destroyed, []);
        // The draws' sizes: 0..=1 for the collateral, the one facility
        // left, the roll, the one target, its roll.
        let mut sizes = Vec::new();
        resolve(&fx.world, &request(&fx, TARGET_MILITARY), &mut |m| {
            sizes.push(m);
            0
        });
        assert_eq!(sizes, [2, 1, 10, 1, 10]);
        // Without a shot no collateral damage either.
        fx.world.capital_ship_classes[fx.ship_class].bombardment_modifier = 0;
        let out = resolve(&fx.world, &request(&fx, TARGET_MILITARY), &mut script(&[], 0));
        assert_eq!((out.shots, out.destroyed.len()), (0, 0));
        fx.world.capital_ship_classes[fx.ship_class].bombardment_modifier = 1;
        // The facility draw comes before the roll: with defense 5, the
        // roll is the third draw (0, giving 1), not the second (9).
        fx.world
            .buildable_classes
            .get_mut(&yard_class)
            .unwrap()
            .stats
            .bombardment = 5;
        let out = resolve(
            &fx.world,
            &request(&fx, TARGET_MILITARY),
            &mut script(&[0, 9, 0, 0, 0], 0),
        );
        assert_eq!(out.destroyed, []);
    }

    #[test]
    fn energy_and_raw_units_have_a_fixed_defense_and_count_as_civilian() {
        // FUN_0058cf80: G0x604 and G0x605 (9); FUN_0058cf20 and FUN_0058cf50
        // lower +0x5c and +0x64 by one and set +0x28.
        let mut fx = fixture(1);
        fx.world.capital_ship_classes[fx.ship_class].bombardment_modifier = 1;
        fx.world.systems[fx.system].total_energy = 3;
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[0, 8], 9));
        assert_eq!(out.destroyed, []);
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[0, 9], 9));
        assert_eq!(out.destroyed, [Target::Energy]);
        assert!(out.civilian_damage);
        let req = request(&fx, GENERAL);
        apply(&mut fx.world, &req, &out);
        assert_eq!(fx.world.systems[fx.system].total_energy, 2);
        // A raw material unit likewise (G0x605).
        fx.world.systems[fx.system].total_energy = 0;
        fx.world.systems[fx.system].raw_materials = 1;
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[0, 8], 9));
        assert_eq!(out.destroyed, []);
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[0, 9], 9));
        assert_eq!(out.destroyed, [Target::Raw]);
        let req = request(&fx, GENERAL);
        apply(&mut fx.world, &req, &out);
        assert_eq!(fx.world.systems[fx.system].raw_materials, 0);
    }

    #[test]
    fn a_death_star_destroys_the_system_only_on_destroy_system() {
        // FUN_0058d990: with +0x44 (0x223) a bombarding ship of family
        // 0x18..0x1b destroys the system; the other kinds bombard.
        let mut fx = fixture(1);
        let ds = fx.world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x3400_0001),
            hull: 1000,
            bombardment_modifier: 15,
            ..CapitalShipClass::default()
        });
        fx.world.fleets[fx.fleet]
            .capital_ships
            .push(ShipInstance::new(ds, 1000, false));
        let out = resolve(
            &fx.world,
            &request(&fx, DESTROY_SYSTEM),
            &mut script(&[], 9),
        );
        assert!(out.system_destroyed);
        // The destruction is the caller's; apply leaves the support alone.
        let mut world = fx.world.clone();
        apply(&mut world, &request(&fx, DESTROY_SYSTEM), &out);
        assert!((world.systems[fx.system].popularity_empire - 0.5).abs() < 1e-6);
        let out = resolve(&fx.world, &request(&fx, GENERAL), &mut script(&[], 9));
        assert!(!out.system_destroyed);
        assert_eq!(out.shots, 35);
        // Without the Death Star Destroy System bombards like General.
        fx.world.fleets[fx.fleet].capital_ships.pop();
        let out = resolve(
            &fx.world,
            &request(&fx, DESTROY_SYSTEM),
            &mut script(&[], 9),
        );
        assert!(!out.system_destroyed);
        assert_eq!(out.shots, 20);
    }

    #[test]
    fn the_headquarters_falls_to_a_shot_its_defense_loses() {
        // The headquarters object (ALLFACSD 1) has bombardment defense 9:
        // only a draw of 10 destroys it; apply clears the HQ facility.
        let mut fx = fixture(1);
        fx.world.capital_ship_classes[fx.ship_class].bombardment_modifier = 1;
        class(
            &mut fx.world,
            HEADQUARTERS_CLASS,
            ClassStats {
                bombardment: 9,
                ..ClassStats::default()
            },
        );
        fx.world.systems[fx.system].is_headquarters = true;
        let mut req = request(&fx, GENERAL);
        req.alliance_hq = Some(fx.system);
        let out = resolve(&fx.world, &req, &mut script(&[0, 8], 9));
        assert_eq!(out.destroyed, []);
        let out = resolve(&fx.world, &req, &mut script(&[0, 9], 9));
        assert_eq!(out.destroyed, [Target::Headquarters]);
        apply(&mut fx.world, &req, &out);
        assert!(!fx.world.systems[fx.system].is_headquarters);
    }

    #[test]
    fn the_tick_s_draws_are_a_fixed_stream_within_range() {
        // port: SplitMix64 from the tick's seed.
        let mut a = BombardEnv::new(42, None);
        let mut b = BombardEnv::new(42, None);
        let xs: Vec<i32> = (0..50).map(|_| a.draw(7)).collect();
        let ys: Vec<i32> = (0..50).map(|_| b.draw(7)).collect();
        assert_eq!(xs, ys);
        assert!(xs.iter().all(|x| (0..7).contains(x)));
        assert!((0..7).all(|v| xs.contains(&v)));
        assert_eq!(BombardEnv::new(1, None).draw(0), 0);
        // SplitMix64 (Steele, Lea, Flood 2014) from 42, mod 1000.
        let mut env = BombardEnv::new(42, None);
        let first: Vec<i32> = (0..5).map(|_| env.draw(1000)).collect();
        assert_eq!(first, [413, 291, 858, 764, 250]);
    }

    #[test]
    fn a_tick_s_bombardment_is_carried_out_and_kept() {
        // BombardEnv::bombard resolves, applies and keeps the outcome;
        // destroyed_systems names those a Death Star destroyed.
        let mut fx = fixture(1);
        fx.world.capital_ship_classes[fx.ship_class].bombardment_modifier = 2;
        let t = troop(&mut fx.world, fx.system, 0);
        let mut env = BombardEnv::new(3, None);
        let shots = env
            .bombard(&mut fx.world, Faction::Empire, &[fx.fleet], fx.system, GENERAL)
            .shots;
        assert_eq!(shots, 2);
        assert_eq!(env.outcomes.len(), 1);
        assert_eq!(env.outcomes[0].1.destroyed, [Target::Troop(t)]);
        assert!(!fx.world.troops.contains_key(t));
        assert_eq!(env.destroyed_systems(), []);
        let ds = fx.world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x3400_0001),
            hull: 1000,
            ..CapitalShipClass::default()
        });
        fx.world.fleets[fx.fleet].capital_ships.push(ShipInstance::new(ds, 1000, false));
        assert!(
            env.bombard(&mut fx.world, Faction::Empire, &[fx.fleet], fx.system, DESTROY_SYSTEM)
                .system_destroyed
        );
        assert_eq!(env.destroyed_systems(), [fx.system]);
    }

    #[test]
    fn without_a_shield_scale_a_ship_keeps_its_class_shields() {
        // FUN_00501160 scales only when GNPRTB 0x800 and the hull are not 0.
        let mut fx = fixture(1);
        fx.world.gnprtb = GnprtbParams::default();
        fx.world.fleets[fx.fleet].capital_ships[0].hull_current = 50;
        let ship = fx.world.fleets[fx.fleet].capital_ships[0].clone();
        assert_eq!(shields(&fx.world, &ship, 50), 10);
        fx.world.gnprtb = gnprtb();
        assert_eq!(shields(&fx.world, &ship, 50), 5);
    }
}
