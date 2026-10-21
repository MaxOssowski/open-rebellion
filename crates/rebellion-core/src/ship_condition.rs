//! A capital ship's ratings after its hits and hull damage, as the Capital
//! Ship Status window shows them (`FUN_00446fd0`;
//! `ghidra/notes/status-window.md`, `ghidra/notes/ship-subsystem-damage.md`).
//!
//! Each rating is the class's less a damage term the ship object computes
//! from its subsystem counters (`+0x64`) and its hull damage (`+0x60`).

use crate::world::{CapitalShipClass, GameWorld, ShipInstance, Subsystem};

/// GNPRTB 2048 (`DAT_006bb388`): the hits that take a whole rating.
const GNPRTB_HIT_SHARE: u16 = 2048;

/// `FUN_0053e170`: `a x b / c`, the division truncating.
fn scale(a: i32, b: i32, c: i32) -> i32 {
    a.wrapping_mul(b) / c
}

fn share(world: &GameWorld) -> i32 {
    world.gnprtb.value(GNPRTB_HIT_SHARE, world.difficulty_index)
}

fn int(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// The class hull (slot `+0x248`) and the ship's hull damage (slot
/// `+0x274`, `+0x60`): the class hull less the ship's current hull.
fn hull(class: &CapitalShipClass, ship: &ShipInstance) -> (i32, i32) {
    let full = int(class.hull);
    (full, full - ship.hull_current)
}

/// A recharge rate after `subsystem`'s hits: the class `rate` less
/// `rate x hits / GNPRTB 2048` (`FUN_00500f60` weapons, `FUN_00500fa0`
/// shields), or the class rate when GNPRTB 2048 is 0.
#[must_use]
pub fn recharge(world: &GameWorld, rate: u32, ship: &ShipInstance, subsystem: Subsystem) -> i32 {
    let share = share(world);
    let rate = int(rate);
    if share == 0 {
        return rate;
    }
    rate - scale(rate, i32::from(ship.hits(subsystem)), share)
}

/// A rating after `subsystem`'s hits and the hull damage (`FUN_00500e60`
/// tractor beam, `FUN_005010c0` sub-light engines, `FUN_00501270` a
/// weapon arc): `value x (G - hits) / G x (hull - damage) / hull`, with
/// G = GNPRTB 2048; the class value when G or the hull is 0.
#[must_use]
pub fn rating(
    world: &GameWorld,
    value: u32,
    class: &CapitalShipClass,
    ship: &ShipInstance,
    subsystem: Subsystem,
) -> i32 {
    let share = share(world);
    let value = int(value);
    let (full, damage) = hull(class, ship);
    if share == 0 || full == 0 {
        return value;
    }
    let after_hits = scale(value, share - i32::from(ship.hits(subsystem)), share);
    scale(after_hits, full - damage, full)
}

/// The bombardment modifier after the hull damage (`FUN_00500f00`): the
/// class modifier less `modifier x damage / hull`.
///
/// port: the original takes the whole modifier while a ship is unfinished
/// (`+0x24 & 0x30` clear and `FUN_005006e0`); a port ship is always
/// finished. A hull of 0, which the original would divide by, keeps the
/// class modifier.
#[must_use]
pub fn bombardment(class: &CapitalShipClass, ship: &ShipInstance) -> i32 {
    let modifier = int(class.bombardment_modifier);
    let (full, damage) = hull(class, ship);
    if full == 0 {
        return modifier;
    }
    modifier - scale(modifier, damage, full)
}

/// The Hyperdrive Rating row (`FUN_00446fd0`): the class `hyperdrive`
/// less the primary's hits x it (`FUN_005011f0`); when that is 0, the
/// `hyperdrive_if_damaged` less the backup's (`FUN_00501230`). Unlike the
/// ship's speed (`FUN_00500820`) it takes no default.
#[must_use]
pub fn hyperdrive(class: &CapitalShipClass, ship: &ShipInstance) -> i32 {
    let after = |rating: u32, subsystem| {
        let rating = int(rating);
        rating - scale(rating, i32::from(ship.hits(subsystem)), 1)
    };
    match after(class.hyperdrive, Subsystem::PrimaryHyperdrive) {
        0 => after(class.hyperdrive_if_damaged, Subsystem::BackupHyperdrive),
        rating => rating,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::GnprtbParams;

    fn world(share: i32) -> GameWorld {
        GameWorld {
            gnprtb: GnprtbParams::uniform(&[(GNPRTB_HIT_SHARE, share)]),
            ..GameWorld::default()
        }
    }

    fn class() -> CapitalShipClass {
        CapitalShipClass {
            hull: 200,
            hyperdrive: 3,
            hyperdrive_if_damaged: 7,
            bombardment_modifier: 10,
            ..CapitalShipClass::default()
        }
    }

    fn ship(hull: i32, hits: &[(Subsystem, u8)]) -> ShipInstance {
        let mut ship = ShipInstance::new(crate::ids::CapitalShipKey::default(), 200, false);
        ship.hull_current = hull;
        for &(subsystem, count) in hits {
            ship.set_hits(subsystem, count);
        }
        ship
    }

    #[test]
    fn each_recharge_hit_takes_a_share_of_the_rate() {
        // FUN_00500f60 / FUN_00500fa0: rate x hits / DAT_006bb388 (GNPRTB
        // 2048, 4 as shipped).
        let world = world(4);
        let hit = ship(200, &[(Subsystem::Weapon, 1), (Subsystem::Shield, 3)]);
        assert_eq!(recharge(&world, 10, &hit, Subsystem::Weapon), 8);
        assert_eq!(recharge(&world, 10, &hit, Subsystem::Shield), 3);
        assert_eq!(recharge(&world, 10, &ship(200, &[]), Subsystem::Weapon), 10);
        // A share of 0 takes nothing.
        assert_eq!(
            recharge(&GameWorld::default(), 10, &hit, Subsystem::Shield),
            10
        );
    }

    #[test]
    fn a_rating_falls_with_its_hits_and_the_hull() {
        // FUN_00500e60: value x (G - hits) / G x (hull - damage) / hull.
        let world = world(4);
        let class = class();
        let hit = ship(150, &[(Subsystem::Tractor, 1)]);
        // 12 x 3 / 4 = 9; 9 x 150 / 200 = 6 (6.75 truncated).
        assert_eq!(rating(&world, 12, &class, &hit, Subsystem::Tractor), 6);
        // Another subsystem's hits leave it; the hull still counts.
        assert_eq!(rating(&world, 12, &class, &hit, Subsystem::Speed), 9);
        assert_eq!(
            rating(&world, 12, &class, &ship(200, &[]), Subsystem::Speed),
            12
        );
        // A share or a hull of 0 keeps the class value.
        assert_eq!(
            rating(&GameWorld::default(), 12, &class, &hit, Subsystem::Tractor),
            12
        );
        let hulless = CapitalShipClass { hull: 0, ..class };
        assert_eq!(rating(&world, 12, &hulless, &hit, Subsystem::Tractor), 12);
    }

    #[test]
    fn the_bombardment_modifier_falls_with_the_hull() {
        // FUN_00500f00: modifier - modifier x damage / hull.
        let class = class();
        assert_eq!(bombardment(&class, &ship(200, &[])), 10);
        // 10 x 70 / 200 = 3 (3.5 truncated).
        assert_eq!(bombardment(&class, &ship(130, &[])), 7);
        let hulless = CapitalShipClass { hull: 0, ..class };
        assert_eq!(bombardment(&hulless, &ship(0, &[])), 10);
    }

    #[test]
    fn a_hit_primary_hyperdrive_reads_the_backup() {
        // FUN_00446fd0: hyperdrive - FUN_005011f0, else the backup less
        // FUN_00501230; no default (FUN_004f63f0) here.
        let class = class();
        assert_eq!(hyperdrive(&class, &ship(200, &[])), 3);
        let primary = ship(200, &[(Subsystem::PrimaryHyperdrive, 1)]);
        assert_eq!(hyperdrive(&class, &primary), 7);
        let both = ship(
            200,
            &[
                (Subsystem::PrimaryHyperdrive, 1),
                (Subsystem::BackupHyperdrive, 1),
            ],
        );
        assert_eq!(hyperdrive(&class, &both), 0);
    }
}
