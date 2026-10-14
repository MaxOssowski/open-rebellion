//! The AI player's world model: the records its modules read and the
//! queries they choose sectors and systems with
//! (`ghidra/notes/ai-galaxy-records.md`).
//!
//! The original keeps one record per class, system, sector, fleet and
//! person, and refreshes them in the AI tick's phase 4 (`FUN_00418500`).
//! The port builds them from the [`GameWorld`] when a pass needs them.

use std::collections::HashMap;

use crate::ids::DatId;
use crate::world::{GameWorld, Skill};

/// A class record (`FUN_0042dd00`, looked up by `FUN_0042e190`): one per
/// capital ship, fighter, regiment, defense, yard and special-force class.
///
/// `values[i]` is the word at `+0x30 + 4i`; its meaning depends on the
/// family (see the `class_*` constants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClassRecord {
    /// `+0x18`: the family's flags (`CLASS_*`).
    pub flags: u32,
    /// `+0x1c`: the refined material cost (`FUN_0053b860`, class `+0x48`).
    pub refined_cost: i32,
    /// `+0x20`: the maintenance cost (`FUN_0053b870`, class `+0x4c`).
    pub maintenance: i32,
    /// `+0x24`: the research order (class `+0x50`).
    pub research_order: i32,
    /// `+0x28`: 1 for an Alliance-only class, 2 Empire-only, 0 both.
    pub side: u8,
    /// `+0x30..+0x5c`.
    pub values: [i32; 12],
}

/// A capital ship class of families `0x18..0x1b`, which skips the
/// strength figures (`FUN_004739b0`). CAPSHPSD's only such class is the
/// Death Star (`0x88`, `ghidra/notes/mission-lifecycle.md`).
pub const CLASS_SHIP_SPECIAL: u32 = 0x80;
/// A capital ship with a gravity well, class `+0xcc` > 0 (`FUN_004739b0`).
pub const CLASS_INTERDICTOR: u32 = 0x400;
/// Any other capital ship (`FUN_004739b0`).
pub const CLASS_SHIP: u32 = 0x10_0000;
/// A fighter class (`FUN_004736c0`).
pub const CLASS_FIGHTER: u32 = 0x20_0000;
/// A regiment whose defense does not exceed its attack (`FUN_004734d0`).
pub const CLASS_OFFENSIVE: u32 = 0x1000;
/// A regiment whose attack does not exceed its defense (`FUN_004734d0`).
pub const CLASS_DEFENSIVE: u32 = 0x2000;
/// A shipyard class, family `0x28` (`FUN_00473090`).
pub const CLASS_SHIPYARD: u32 = 0x4000;
/// A training facility class, family `0x29` (`FUN_00473090`).
pub const CLASS_TRAINING: u32 = 0x8000;
/// A construction yard class, family `0x2a` (`FUN_00473090`).
pub const CLASS_CONSTRUCTION: u32 = 0x1_0000;
/// A defense of family `0x24` (`FUN_00473290`).
pub const CLASS_DEFENSE_24: u32 = 0x2_0000;
/// A defense of family `0x25` (`FUN_00473290`).
pub const CLASS_DEFENSE_25: u32 = 0x4_0000;
/// A defense of family `0x22` or `0x23` (`FUN_00473290`).
pub const CLASS_DEFENSE_22: u32 = 0x8_0000;

/// The class records: capital ships and fighters by their class keys,
/// which the port's ships and squadrons name, and the rest by class id.
#[derive(Debug, Clone, Default)]
pub struct ClassRecords {
    ships: HashMap<crate::ids::CapitalShipKey, ClassRecord>,
    fighters: HashMap<crate::ids::FighterKey, ClassRecord>,
    by_class: HashMap<DatId, ClassRecord>,
}

fn int(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// `FUN_00473090` and `FUN_00473290`: 1 when the Empire may not build the
/// class, else 2 when the Alliance may not, else 0.
fn either_side(is_alliance: bool, is_empire: bool) -> u8 {
    if !is_empire {
        1
    } else if !is_alliance {
        2
    } else {
        0
    }
}

/// `FUN_004739b0`'s choice of firing arc. Its chain of comparisons picks
/// the first arc, fore to starboard, with the greatest figure (checked on
/// every arc combination of -1..=3 against the decompile's branches).
fn best_arc(a: [i32; 4]) -> usize {
    let top = a.iter().copied().max().unwrap_or(0);
    a.iter().position(|&x| x == top).unwrap_or(0)
}

impl ClassRecords {
    /// Build every class record from the world's class data
    /// (`FUN_0042dd00`).
    #[must_use]
    pub fn build(world: &GameWorld) -> Self {
        let ships = world
            .capital_ship_classes
            .iter()
            .map(|(key, class)| (key, Self::capital_ship(class)))
            .collect();
        let fighters = world
            .fighter_classes
            .iter()
            .map(|(key, class)| (key, Self::fighter_class(class)))
            .collect();
        let mut by_class = HashMap::new();
        for (&id, class) in &world.buildable_classes {
            let record = match id.family() {
                0x10..=0x13 => Self::regiment(world, id, class),
                0x22..=0x27 => Self::defense(id, class),
                0x28..=0x2b => Self::yard(id, class),
                0x3c..=0x3f => Self::special_force(world, id, class),
                _ => continue,
            };
            by_class.insert(id, record);
        }
        Self {
            ships,
            fighters,
            by_class,
        }
    }

    /// The record of a regiment, defense, yard or special-force class
    /// (`FUN_0042e190`).
    #[must_use]
    pub fn get(&self, class: DatId) -> Option<&ClassRecord> {
        self.by_class.get(&class)
    }

    /// The record of a capital ship class.
    #[must_use]
    pub fn ship(&self, class: crate::ids::CapitalShipKey) -> Option<&ClassRecord> {
        self.ships.get(&class)
    }

    /// The record of a fighter class.
    #[must_use]
    pub fn fighter(&self, class: crate::ids::FighterKey) -> Option<&ClassRecord> {
        self.fighters.get(&class)
    }

    /// `FUN_004739b0`. Weapons `W(arc, kind)` are `FUN_0053bb80`: arcs
    /// fore, aft, port, starboard; kinds turbolaser, ion cannon, laser.
    ///
    /// - `values[0]` (`+0x30`): the best arc of (turbolasers + ions) × 100
    ///   plus lasers × 100 / 6, times the weapon recharge rate (`+0xdc`),
    ///   over that arc's weapons.
    /// - `values[1]` (`+0x34`): the best arc of lasers × 10 + turbolasers
    ///   × 10 / 2, times maneuverability (`+0x68`) and the recharge rate,
    ///   over that arc's lasers and turbolasers.
    /// - `values[2]` (`+0x38`): bombardment (`+0xd4`) above 0.
    /// - `values[3]` (`+0x3c`): shield recharge (`+0xe0`) × 100.
    /// - `values[4..=8]` (`+0x40..+0x50`): hull, fighter capacity, troop
    ///   capacity, detection, weapon recharge; `values[9]` (`+0x54`)
    ///   shield recharge.
    /// - `values[10]`, `values[11]` (`+0x58`, `+0x5c`): `values[0]` × 10
    ///   over the maintenance and over the refined cost.
    fn capital_ship(class: &crate::world::CapitalShipClass) -> ClassRecord {
        let weapons = [
            [
                class.turbolaser_fore,
                class.ion_cannon_fore,
                class.laser_cannon_fore,
            ],
            [
                class.turbolaser_aft,
                class.ion_cannon_aft,
                class.laser_cannon_aft,
            ],
            [
                class.turbolaser_port,
                class.ion_cannon_port,
                class.laser_cannon_port,
            ],
            [
                class.turbolaser_starboard,
                class.ion_cannon_starboard,
                class.laser_cannon_starboard,
            ],
        ]
        .map(|arc| arc.map(int));
        let recharge = int(class.weapon_recharge_rate);
        let mut r = ClassRecord {
            refined_cost: int(class.refined_material_cost),
            maintenance: int(class.maintenance_cost),
            research_order: int(class.research_order),
            side: if class.is_empire { 2 } else { 1 },
            ..ClassRecord::default()
        };
        r.values[4] = int(class.hull);
        r.values[5] = int(class.fighter_capacity);
        r.values[6] = int(class.troop_capacity);
        r.values[8] = recharge;
        r.values[9] = int(class.shield_recharge_rate);
        if class.is_death_star() {
            r.flags |= CLASS_SHIP_SPECIAL;
        } else {
            if class.gravity_well_projector > 0 {
                r.flags |= CLASS_INTERDICTOR;
            }
            r.flags |= CLASS_SHIP;
            r.values[3] = int(class.shield_recharge_rate).saturating_mul(100);
            let heavy = weapons.map(|w| w[0] * 100 + w[1] * 100);
            if r.flags & CLASS_INTERDICTOR == 0 && heavy.iter().any(|&a| a > 0) {
                r.flags |= 1;
            }
            let attack: [i32; 4] = std::array::from_fn(|k| heavy[k] + weapons[k][2] * 100 / 6);
            // The original skips an all-zero table; its figure stays 0 then,
            // as the division gives here.
            let k = best_arc(attack);
            let sum: i32 = weapons[k].iter().sum();
            r.values[0] = (recharge * attack[k]).checked_div(sum).unwrap_or(0);
            if r.flags & CLASS_INTERDICTOR == 0
                && r.values[6] == 0
                && weapons.iter().any(|w| w[2] * 10 > 0)
            {
                r.flags |= 2;
            }
            let light: [i32; 4] =
                std::array::from_fn(|k| weapons[k][2] * 10 + weapons[k][0] * 10 / 2);
            let k = best_arc(light);
            let sum = weapons[k][2] + weapons[k][0];
            r.values[1] = (int(class.maneuverability) * recharge * light[k])
                .checked_div(sum)
                .unwrap_or(0);
        }
        r.values[2] = i32::from(class.bombardment_modifier != 0);
        r.values[7] = int(class.detection);
        r.values[10] = (r.values[0] * 10).checked_div(r.maintenance).unwrap_or(0);
        r.values[11] = (r.values[0] * 10).checked_div(r.refined_cost).unwrap_or(0);
        r.flags |= if class.hull < 1100 {
            0x10
        } else if class.hull < 2000 {
            0x20
        } else {
            0x40
        };
        if r.values[6] != 0 {
            let before = r.flags;
            r.flags |= 0x200;
            // `before | 0x204`: 0x200 is already set.
            if before & 0x403 == 0 && before & CLASS_SHIP != 0 {
                r.flags |= 4;
            }
            if r.flags & 1 != 0 {
                r.flags |= 8;
            }
        }
        if r.values[5] != 0 {
            r.flags |= 0x100;
        }
        if r.values[2] != 0 {
            r.flags |= 0x800;
        }
        r
    }

    /// `FUN_004736c0`, from the fore arc only:
    /// - `values[7]` (`+0x4c`): squadron size;
    /// - `values[2]` (`+0x38`): squadron × 25 / 10;
    /// - `values[3]` (`+0x3c`): maneuverability;
    /// - `values[4]` (`+0x40`): bombardment defense;
    /// - `values[6]` (`+0x48`): torpedoes × squadron;
    /// - `values[0]` (`+0x30`): (ions × 100 + lasers × 100 / 6) × squadron,
    ///   over the ions and lasers;
    /// - `values[1]` (`+0x34`): with lasers, maneuverability × squadron ×
    ///   10, plus shields × that / 8;
    /// - `values[5]` (`+0x44`): detection.
    fn fighter_class(class: &crate::world::FighterClass) -> ClassRecord {
        let squad = int(class.squadron_size);
        let (ion, laser) = (int(class.ion_cannon_fore), int(class.laser_cannon_fore));
        let mut r = ClassRecord {
            flags: CLASS_FIGHTER,
            refined_cost: int(class.refined_material_cost),
            maintenance: int(class.maintenance_cost),
            research_order: int(class.research_order),
            side: if class.is_empire { 2 } else { 1 },
            ..ClassRecord::default()
        };
        r.values[7] = squad;
        r.values[2] = squad * 25 / 10;
        r.values[3] = int(class.maneuverability);
        r.values[4] = int(class.bombardment_defense);
        r.values[6] = int(class.torpedoes) * squad;
        let attack = ion * 100 + laser * 100 / 6;
        // The original skips this with no fore guns; the figure stays 0.
        r.values[0] = (attack * squad).checked_div(laser + ion).unwrap_or(0);
        if laser > 0 {
            let light = (laser * r.values[3] * squad * 10) / laser;
            r.values[1] = int(class.shield_strength) * light / 8 + light;
        }
        r.values[5] = int(class.detection);
        r
    }

    /// `FUN_004734d0`: attack (`+0x30`), defense (`+0x34`), bombardment
    /// defense (`+0x38`) and detection (`+0x3c`); offensive when defense
    /// <= attack, defensive when attack <= defense.
    fn regiment(world: &GameWorld, id: DatId, class: &crate::world::BuildableClass) -> ClassRecord {
        let troop = world.troop_classes.get(&id);
        let attack = troop.map_or(0, |t| int(t.attack_strength));
        let defense = troop.map_or(0, |t| int(t.defense_strength));
        let mut r = Self::common(class, if class.is_empire { 2 } else { 1 });
        r.values[0] = attack;
        r.values[1] = defense;
        r.values[2] = int(class.stats.bombardment);
        r.values[3] = troop.map_or(0, |t| int(t.detection));
        if defense <= attack {
            r.flags |= CLASS_OFFENSIVE;
        }
        if attack <= defense {
            r.flags |= CLASS_DEFENSIVE;
        }
        r
    }

    /// `FUN_00473290`: attack (`+0x30`) and shields (`+0x34`).
    fn defense(id: DatId, class: &crate::world::BuildableClass) -> ClassRecord {
        let mut r = Self::common(class, either_side(class.is_alliance, class.is_empire));
        r.values[0] = int(class.stats.attack_strength);
        r.values[1] = int(class.stats.shield_strength);
        r.flags |= match id.family() {
            0x24 => CLASS_DEFENSE_24,
            0x25 => CLASS_DEFENSE_25,
            0x22 | 0x23 => CLASS_DEFENSE_22,
            _ => 0,
        };
        r
    }

    /// `FUN_00473090`: the processing rate (`+0x30`).
    fn yard(id: DatId, class: &crate::world::BuildableClass) -> ClassRecord {
        let mut r = Self::common(class, either_side(class.is_alliance, class.is_empire));
        r.values[0] = int(class.processing_rate);
        r.flags |= match id.family() {
            0x28 => CLASS_SHIPYARD,
            0x29 => CLASS_TRAINING,
            0x2a => CLASS_CONSTRUCTION,
            _ => 0,
        };
        r
    }

    /// `FUN_00472cc0`: the base diplomacy (`+0x30`), espionage (`+0x34`),
    /// combat (`+0x38`) and leadership (`+0x3c`); a bit per unit type
    /// (`0x3c000001`/`5` -> `0x1000000`, `2`/`6` -> `0x2000000`, `3`/`7`
    /// -> `0x4000000`, `4`/`8` -> `0x8000000`).
    fn special_force(
        world: &GameWorld,
        id: DatId,
        class: &crate::world::BuildableClass,
    ) -> ClassRecord {
        let skills = world.special_force_classes.get(&id).map(|c| c.skills);
        let base = |skill: Skill| skills.map_or(0, |s| int(s[skill as usize].base));
        let mut r = Self::common(class, if class.is_empire { 2 } else { 1 });
        r.values[0] = base(Skill::Diplomacy);
        r.values[1] = base(Skill::Espionage);
        r.values[2] = base(Skill::Combat);
        r.values[3] = base(Skill::Leadership);
        r.flags |= match id.raw() {
            0x3c00_0001 | 0x3c00_0005 => 0x100_0000,
            0x3c00_0002 | 0x3c00_0006 => 0x200_0000,
            0x3c00_0003 | 0x3c00_0007 => 0x400_0000,
            0x3c00_0004 | 0x3c00_0008 => 0x800_0000,
            _ => 0,
        };
        r
    }

    /// The fields every family's record takes from its class.
    fn common(class: &crate::world::BuildableClass, side: u8) -> ClassRecord {
        ClassRecord {
            refined_cost: int(class.refined_material_cost),
            maintenance: int(class.maintenance_cost),
            research_order: int(class.research_order),
            side,
            ..ClassRecord::default()
        }
    }
}

/// A fleet record (`FUN_00432e40`, refreshed by `FUN_00433620`): what one
/// fleet holds, summed over its capital ships' records (`FUN_00484630`,
/// added in by `FUN_00433780`).
///
/// `values[i]` is the word at `+0x50 + 4i`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FleetRecord {
    /// `+0x38`.
    pub flags: u32,
    /// `+0x50..+0xc4`.
    pub values: [i32; 30],
}

/// The fleet is the record's side's (`FUN_00433620`).
pub const FLEET_OWN: u32 = 1;
/// The fleet is travelling (object `+0x50` bit 4, `FUN_00433620`).
pub const FLEET_EN_ROUTE: u32 = 2;
/// A ship neither travelling nor inactive (`FUN_00433780`: ship bits
/// `0x800` and `0x2000` clear).
pub const FLEET_STANDING: u32 = 0x4000;

/// A capital ship record's values, `+0x44 + 4i` (`FUN_00484630`).
type ShipValues = [i32; 30];

/// What the first ship of a fleet carries: the regiments aboard and the
/// people's record flags.
type Cargo<'a> = (
    &'a [crate::ids::TroopKey],
    &'a dyn Fn(crate::ids::CharacterKey) -> Option<u32>,
);

/// `FUN_00484630`'s record of a capital ship, with the fleet's fighters,
/// regiments and people aboard it when `cargo` is set.
///
/// port: the port keeps no damage to a ship's weapon and shield recharge
/// (the slots `+0x264 - +0x290` and `+0x268 - +0x294`) or to a squadron
/// (`+0x244 - +0x254`), so each counts at its class's full figure; its
/// fighters and regiments ride the fleet, not one ship, and the first
/// ship carries them; every unit aboard counts as active (object `+0x50`
/// bit 2) and not as bit 5. hyp: the slot `+0x34(1)` test for object bit
/// `0x200` (ship bits `0x1000`/`0x800000`) is untraced and reads as
/// clear.
fn ship_record(
    world: &GameWorld,
    classes: &ClassRecords,
    ship: &crate::world::ShipInstance,
    fleet: &crate::world::Fleet,
    en_route: bool,
    cargo: Option<Cargo<'_>>,
) -> (u32, ShipValues) {
    let mut v: ShipValues = [0; 30];
    let Some(class) = classes.ship(ship.class) else {
        return (0, v);
    };
    // The bits the record keeps from its class (mask 0x1b0204ff). Its
    // 0x4000 (the ship's owner is the record's side) is not summed.
    let mut flags = class.flags & 0x4ff;
    if en_route {
        flags |= 0x800;
    }
    v[0] = class.values[4];
    v[1] = class.values[2];
    // FUN_00473910 with the full shield recharge: class +0x3c. Its guard
    // (+0x54 > 0) and FUN_004738d0's (+0x50 > 0) give 0 when the recharge
    // is 0, as the class figures are then.
    v[3] = class.values[3];
    // FUN_004738d0 / FUN_004738f0 with the full weapon recharge.
    v[14] = class.values[0];
    v[19] = class.values[0];
    if flags & 2 != 0 {
        v[15] = class.values[1];
        v[20] = class.values[1];
    }
    v[7] = class.values[5];
    v[10] = class.values[6];
    v[11] = class.maintenance;
    v[17] += 1;
    if let Some((troops, person)) = cargo {
        for entry in &fleet.fighters {
            let Some(wing) = classes.fighter(entry.class) else {
                continue;
            };
            for _ in 0..entry.count {
                v[13] += wing.maintenance;
                v[5] += 1;
                flags |= 0x100;
                // FUN_00473610 / 620 / 630 at full squadron strength.
                v[19] += wing.values[0];
                v[20] += wing.values[1];
                v[16] += wing.values[2];
                v[14] += wing.values[0];
                v[15] += wing.values[1];
                v[1] += wing.values[4];
                v[17] += 1;
                v[6] += 1;
            }
        }
        for &troop in troops {
            let Some(unit) = world.troops.get(troop) else {
                continue;
            };
            let record = classes.get(unit.class_dat_id);
            v[12] += record.map_or(0, |r| r.maintenance);
            flags |= 0x200;
            v[8] += 1;
            v[2] += record.map_or(0, |r| r.values[0]);
            v[17] += 1;
            v[9] += 1;
        }
        for &character in &fleet.characters {
            let Some(person_flags) = person(character) else {
                continue;
            };
            flags |= person_aboard(person_flags, &mut v);
        }
    }
    (flags, v)
}

/// `FUN_00484630`'s bits for a person aboard, from its record's `+0x30`.
fn person_aboard(person: u32, v: &mut ShipValues) -> u32 {
    const OTHER_SIDE: u32 = 4;
    const PRISONER: u32 = 0x40;
    const EN_ROUTE: u32 = 0x20;
    const MAIN: u32 = 8;
    const LEADER: u32 = 0x4000_0000;
    let mut flags = 0;
    if person & 0x200_0000 != 0 {
        flags |= 0x2000_0000;
    }
    if person & OTHER_SIDE == 0 {
        flags |= if person & PRISONER == 0 {
            0x40_0000
        } else {
            0x8_0000
        };
    } else if person & PRISONER != 0 {
        flags |= 0x20_0000;
    } else {
        if person & MAIN == 0 {
            if person & EN_ROUTE == 0 {
                flags |= 0x10_0000;
            }
        } else {
            if person & EN_ROUTE == 0 {
                flags |= 0x4_0000;
            }
            // While the controller's bit 0 is clear (port: always, as the
            // records' leader bit), a leader counts.
            if person & LEADER != 0 {
                v[18] += 1;
            }
        }
        if person & 0x700_0000 != 0 {
            flags |= 0x400_0000;
        }
    }
    flags
}

impl FleetRecord {
    /// `FUN_00433620` for `fleet`, seen by the side `is_alliance`: each
    /// capital ship's record added in by `FUN_00433780`.
    #[must_use]
    pub fn build(
        world: &GameWorld,
        classes: &ClassRecords,
        fleet_key: crate::ids::FleetKey,
        is_alliance: bool,
        movement: &crate::movement::MovementState,
        transport: &crate::troop_transport::TroopTransportState,
        person: &dyn Fn(crate::ids::CharacterKey) -> Option<u32>,
    ) -> Option<Self> {
        let fleet = world.fleets.get(fleet_key)?;
        let en_route = movement.is_in_transit(fleet_key);
        // +0x38 keeps 4 from its constructor (mask 0xf000000c).
        let mut r = Self {
            flags: 4,
            values: [0; 30],
        };
        if fleet.is_alliance == is_alliance {
            r.flags |= FLEET_OWN;
        }
        if en_route {
            r.flags |= FLEET_EN_ROUTE;
        }
        let mut cargo = Some((transport.cargo(fleet_key), person));
        for ship in fleet.capital_ships.iter().filter(|s| s.alive) {
            let (flags, v) = ship_record(world, classes, ship, fleet, en_route, cargo.take());
            r.add_ship(flags, &v);
        }
        if r.values[22] > 0 {
            r.flags |= 0x8_0000;
        }
        if r.values[23] > 0 {
            r.flags |= 0x10_0000;
        }
        Some(r)
    }

    /// `FUN_00433780`: one ship record's flags `f` and values `s` into the
    /// fleet's (`values[i]` = `+0x50 + 4i`).
    fn add_ship(&mut self, f: u32, s: &ShipValues) {
        let v = &mut self.values;
        v[0] += 1;
        for (bit, set) in [
            (0x100, 0x2_0000),
            (0x200, 0x4_0000),
            (0x40_0000, 0x400_0000),
            (0x2000_0000, 0x800_0000),
            (0x1_0000, 0x200),
            (0x2000, 0x400),
            (0x8000, 0x100),
            (0x80, 0x10),
            (0x400, 0x20),
            (0x8_0000, 0x100_0000),
            (0x10_0000, 0x40_0000),
            (0x400_0000, 0x200_0000),
            (0x20_0000, 0x80_0000),
        ] {
            if f & bit != 0 {
                self.flags |= set;
            }
        }
        if f & 0x800 != 0 {
            self.flags |= 0x800;
        } else if f & 0x2000 == 0 {
            self.flags |= FLEET_STANDING;
        }
        if f & 0x80_1000 == 0 {
            self.flags |= 0x8000;
            if f & 1 != 0 {
                v[18] += 1;
            }
            if f & 2 != 0 {
                v[19] += 1;
            }
            if f & 4 != 0 {
                v[20] += 1;
            }
        } else {
            if f & 0x80_0000 != 0 {
                self.flags |= 0x2000;
            }
            if f & 0x1000 != 0 {
                self.flags |= 0x1000;
            }
        }
        if f & 0x4_0000 != 0 {
            self.flags |= 0x20_0000;
            v[24] += s[18];
        }
        if f & 0x800_0000 == 0 {
            v[17] += 1;
        }
        if f & 0x2000 == 0 {
            v[2] += s[14];
            v[3] += s[15];
            v[4] += s[3];
            v[6] += s[1];
            v[7] += s[2];
            v[1] += s[0];
            v[5] += s[16];
            v[8] += s[5];
            v[10] += s[7];
            v[9] += s[6];
            v[13] += s[10];
            v[11] += s[8];
            v[12] += s[9];
            v[21] += s[17];
            v[25] += s[19];
            v[26] += s[20];
        }
        v[14] += s[11];
        v[15] += s[12];
        v[16] += s[13];
        if f & 4 != 0 {
            v[27] += s[10];
        }
        if f & 0x2800 == 0 {
            v[22] += s[10] - s[8];
            v[23] += s[7] - s[5];
        }
    }
}

/// The side object's constants (`FUN_0041bd20`, at AI setup): garrison and
/// defense goals by a system's state.
mod side_constants {
    /// Side `+0x364` (`FUN_0041bd20`).
    pub const S364: i32 = 3;
    /// Side `+0x368` (`FUN_0041bd20`).
    pub const S368: i32 = 3;
    /// Side `+0x36c` (`FUN_0041bd20`).
    pub const S36C: i32 = 2;
    /// Side `+0x370` (`FUN_0041bd20`).
    pub const S370: i32 = 1;
    /// Side `+0x374` (`FUN_0041bd20`).
    pub const S374: i32 = 1;
    /// Side `+0x378` (`FUN_0041bd20`).
    pub const S378: i32 = 0;
    /// Side `+0x37c` (`FUN_0041bd20`).
    pub const S37C: i32 = 0;
    /// Side `+0x380` (`FUN_0041bd20`).
    pub const S380: i32 = 3;
    /// Side `+0x384` (`FUN_0041bd20`).
    pub const S384: i32 = 9;
    /// Side `+0x388` (`FUN_0041bd20`).
    pub const S388: i32 = 0;
    /// Side `+0x38c` (`FUN_0041bd20`).
    pub const S38C: i32 = 6;
    /// Side `+0x390` (`FUN_0041bd20`).
    pub const S390: i32 = 12;
}

/// What a system record reads besides the world (`FUN_00433e40`).
#[derive(Clone, Copy)]
pub struct SystemInputs<'a> {
    pub world: &'a GameWorld,
    pub classes: &'a ClassRecords,
    /// The records of the fleets at the system.
    pub fleets: &'a HashMap<crate::ids::FleetKey, FleetRecord>,
    /// A person's record flags (`+0x30`).
    pub person: &'a dyn Fn(crate::ids::CharacterKey) -> Option<u32>,
    pub economy: &'a crate::economy::EconomyState,
    pub uprisings: &'a crate::uprising::UprisingState,
    pub manufacturing: &'a crate::manufacturing::ManufacturingState,
    pub research: &'a crate::research::ResearchState,
    /// The record's side.
    pub is_alliance: bool,
    /// The side's MNCHARSD classes (side `+0xa4`, `FUN_0042dd00`).
    pub minor_classes: i32,
}

/// A system record (`FUN_00433b10`, refreshed by `FUN_00433e40`).
///
/// `values[k]` is `v[k]`, the word at `+0x48 + 4k`
/// (`ghidra/notes/ai-galaxy-records.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemRecord {
    pub system: crate::ids::SystemKey,
    /// `+0x28`: the system's state; the yard marks `0x3e00000` persist.
    pub state: u32,
    /// `+0x2c`: the summaries the queries and the strategy read.
    pub summary: u32,
    /// `+0x30`: side and kind.
    pub kind: u32,
    pub values: [i32; 60],
    /// `+0x34`: the map position, class `+0x4c` (`FUN_00509620`), SYSTEMSD
    /// `x_position`/`y_position`.
    pub position: (i16, i16),
}

/// A map position as the records pack it: two shorts.
fn map_position(x: u16, y: u16) -> (i16, i16) {
    (x.cast_signed(), y.cast_signed())
}

/// `FUN_0041b7b0`: the squared distance of two packed positions, the
/// products wrapping as the original's 32-bit `imul`.
fn squared_distance(a: (i16, i16), b: (i16, i16)) -> i32 {
    let dx = i32::from(a.0) - i32::from(b.0);
    let dy = i32::from(a.1) - i32::from(b.1);
    dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy))
}

/// The side number the original stores (system `+0x24 >> 6 & 3`,
/// `FUN_00433e40`): 1 Alliance, 2 Empire, 0 neither.
fn side_number(side: Option<crate::dat::Faction>) -> u32 {
    match side {
        Some(crate::dat::Faction::Alliance) => 1,
        Some(crate::dat::Faction::Empire) => 2,
        _ => 0,
    }
}

/// The system record bits a refresh keeps (`FUN_00433e40`): `+0x28 &
/// 0x3e00000`, the yard marks, and `+0x30 & 0x35ff0000`, such as a
/// module's claim `0x10000000` (`FUN_004dd120`). Only the modules set them.
pub const KEPT_STATE: u32 = 0x3e0_0000;
/// See [`KEPT_STATE`] (`FUN_00433e40`).
pub const KEPT_KIND: u32 = 0x35ff_0000;

impl SystemRecord {
    /// [`Self::build_kept`] with the yard marks `marks` and no kept `+0x30`
    /// bits.
    #[must_use]
    pub fn build(
        inputs: &SystemInputs<'_>,
        system: crate::ids::SystemKey,
        marks: u32,
    ) -> Option<Self> {
        Self::build_kept(inputs, system, marks, 0)
    }

    /// `FUN_00433e40` for `system`, keeping the yard marks `marks`
    /// (`+0x28 & 0x3e00000`) and the `+0x30` bits `kind & 0x35ff0000`.
    ///
    /// port: every unit and facility in the port is active (object `+0x50`
    /// bit 0); the side's view of the system is the world's (the port's AI
    /// sees the whole galaxy); a system is explored when the world says so;
    /// a yard area is idle when its queue is empty (manager `+0x58`).
    /// hyp: the thresholds the record reads at `+0x40` (`+0x24`, `+0x28`;
    /// the controller's object at `+0x138`) have no writer found and read
    /// as 0; object `+0x50` bit 3 (`+0x30` bit `0x10`) and the controller
    /// bits `0x1000000`/4 (`+0x28` bit `0x40000000`) read as clear.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "One refresh, in the decompile's order."
    )]
    pub fn build_kept(
        inputs: &SystemInputs<'_>,
        system: crate::ids::SystemKey,
        marks: u32,
        kind: u32,
    ) -> Option<Self> {
        use crate::manufacturing::ProductionArea;
        use side_constants::*;
        let world = inputs.world;
        let sys = world.systems.get(system)?;
        let side: u32 = if inputs.is_alliance { 1 } else { 2 };
        let opponent: u32 = 3 - side;
        let holder = side_number(crate::uprising::holder(sys));
        let mut f28 = marks & KEPT_STATE;
        let mut f2c = 0;
        let mut f30 = kind & KEPT_KIND;
        let mut v = [0i32; 60];
        if (0x92..=0x93).contains(&sys.dat_id.family()) {
            f30 |= 0x40;
        }
        if holder == side {
            f30 |= 0x21;
        } else if holder == opponent {
            f30 |= 0x24;
        } else {
            f30 |= 2;
        }
        let position = map_position(sys.x, sys.y);
        let record = |f28, f2c, f30, v| Self {
            system,
            state: f28,
            summary: f2c,
            kind: f30,
            values: v,
            position,
        };
        if sys.exploration_status != crate::dat::ExplorationStatus::Explored {
            // +0x2c is still clear here.
            return Some(record(f28, 0x80, f30, v));
        }
        f2c |= 0x800_0000;
        let economy = inputs.economy.per_system.get(&system);
        let energy = i32::from(sys.total_energy);
        v[1] = energy;
        v[15] = energy - economy.map_or(0, |e| int(e.energy_allocated));
        v[14] = i32::from(sys.raw_materials) - economy.map_or(0, |e| int(e.raw_material_allocated));
        if crate::blockade::BlockadeSystem::system_is_blockaded(world, sys) {
            f28 |= 2;
        }
        let id = sys.dat_id.raw() & 0xff_ffff;
        if id == 0x109 {
            f28 |= 0x8000_0000;
            f30 |= 0x4000_0000;
        }
        if id == 0x121 {
            f30 |= 0x800_0000;
        }
        // FUN_00526a60: the holder's headquarters objects (0x20..0x21).
        if sys.is_headquarters && holder != 0 {
            f28 |= 0x8000_0000;
            f30 |= 0x8000_0000;
            f2c |= 0x200_0000;
        }
        if f30 & 1 != 0 {
            v[0] = 100;
        }
        // FUN_00504c40: the regiments (0x10..0x13).
        for unit in sys.ground_units.iter().filter_map(|&t| world.troops.get(t)) {
            let class = inputs
                .classes
                .get(unit.class_dat_id)
                .copied()
                .unwrap_or_default();
            if f30 & 1 != 0 {
                let counted = if f28 & 0x83e0_0000 == 0 {
                    class.flags & CLASS_OFFENSIVE == 0
                } else {
                    // hyp: the threshold `+0x40 -> +0x24` is 0.
                    class.values[1] >= 0
                };
                // The decompile sets 0x20000000 for a counted regiment of an
                // unmarked system and the class bits for a marked one; a
                // counted unmarked regiment is never offensive, so both read
                // the class. An uncounted one is offensive (threshold 0).
                if counted {
                    v[17] += 1;
                    if class.flags & CLASS_OFFENSIVE != 0 {
                        f28 |= 0x1000_0000;
                    }
                    if class.flags & CLASS_DEFENSIVE != 0 {
                        f28 |= 0x2000_0000;
                    }
                } else {
                    f30 |= 0x4000;
                    if class.flags & CLASS_DEFENSIVE != 0 {
                        f30 |= 0x8000;
                    }
                }
            }
            v[16] += 1;
            v[30] += class.maintenance;
            v[18] += 1;
        }
        // FUN_004ffe70: the fleets (0x08..0x0f).
        for fleet in sys.fleets.iter().filter_map(|f| inputs.fleets.get(f)) {
            let ff = fleet.flags;
            if ff & FLEET_OWN == 0 && ff & FLEET_STANDING != 0 {
                v[39] += fleet.values[25];
                v[40] += fleet.values[26];
                v[41] += fleet.values[4];
                v[43] += fleet.values[8];
                v[29] += fleet.values[21];
                f30 |= 0x200;
                f2c |= 0x80_0000;
                if ff & 0x10 != 0 {
                    f30 |= 0x400;
                    if side == 1 {
                        f2c |= 0x100_0000;
                    }
                }
            }
            if ff & 0x20_0000 != 0 {
                f2c |= 0x2800;
            }
            if ff & 0x40_0000 != 0 {
                f2c |= 0x800;
            }
            if ff & 0x200_0000 != 0 {
                f30 |= 0x1000;
            }
            if ff & 0x100_0000 != 0 {
                f2c |= 0x8000;
            }
        }
        // FUN_00536da0: the people on the planet (0x30..0x3f).
        let people = world.characters.iter().filter(|(_, c)| {
            !c.is_killed && c.current_system == Some(system) && c.current_fleet.is_none()
        });
        for (key, _) in people {
            let Some(pf) = (inputs.person)(key) else {
                continue;
            };
            if pf & 0x2000_0000 == 0 {
                continue;
            }
            if pf & 4 == 0 {
                if pf & 0x40 != 0 {
                    f2c |= 0x4000;
                    v[46] += 1;
                } else if pf & 8 == 0 {
                    v[45] += 1;
                } else {
                    v[44] += 1;
                }
            } else if pf & 0x40 == 0 {
                if pf & 0x20 == 0 {
                    f2c |= 0x400;
                }
                if pf & 0x700_0000 != 0 {
                    f30 |= 0x1000;
                    if pf & 0x200_0000 != 0 {
                        f30 |= 0x3000;
                    }
                }
                if pf & 8 == 0 {
                    v[48] += 1;
                } else {
                    f2c |= 0x1000;
                    v[47] += 1;
                    if pf & 0x4000_0000 != 0 {
                        v[50] += 1;
                    }
                }
            } else {
                f2c |= 0x400_0000;
                v[46] += 1;
            }
        }
        if f30 & 1 != 0 && v[15] > 0 {
            f28 |= 0x20;
            if v[14] > 0 {
                f28 |= 0x40;
            }
        }
        // FUN_0052d370, FUN_0052cf40: the holder's mines and refineries.
        for facility in sys
            .production_facilities
            .iter()
            .filter_map(|&f| world.production_facilities.get(f))
        {
            if side_number(Some(facility.side)) == holder {
                if facility.is_mine {
                    v[2] += 1;
                } else {
                    v[3] += 1;
                }
            }
        }
        // FUN_0052cab0, FUN_0052c740, FUN_0052c420: the yards.
        let idle = |area: ProductionArea| {
            inputs
                .manufacturing
                .queue(system, area)
                .is_none_or(crate::manufacturing::ProductionQueue::is_empty)
        };
        let mut yards = [false; 3];
        for facility in sys
            .manufacturing_facilities
            .iter()
            .filter_map(|&f| world.manufacturing_facilities.get(f))
        {
            let maintenance = inputs
                .classes
                .get(facility.class_dat_id)
                .map_or(0, |c| c.maintenance);
            let (slot, sum) = match facility.class_dat_id.family() {
                0x28 => (0, 34),
                0x29 => (1, 35),
                0x2a => (2, 36),
                _ => continue,
            };
            yards[slot] = true;
            v[4 + slot] += 1;
            v[33] += maintenance;
            v[sum] += maintenance;
        }
        for (slot, area, bit) in [
            (0, ProductionArea::Shipyard, 0x200),
            (1, ProductionArea::TrainingFacility, 0x400),
            (2, ProductionArea::ConstructionYard, 0x800),
        ] {
            if yards[slot] {
                f28 |= 0x1000;
                if f28 & 3 == 0 && idle(area) {
                    f28 |= bit;
                }
            }
        }
        // FUN_005278c0 (0x24), FUN_0051c270 (0x25), FUN_005272e0 and
        // FUN_00527550 (0x23, 0x22): the defenses.
        for facility in sys
            .defense_facilities
            .iter()
            .filter_map(|&f| world.defense_facilities.get(f))
        {
            let class = inputs
                .classes
                .get(facility.class_dat_id)
                .copied()
                .unwrap_or_default();
            match facility.class_dat_id.family() {
                0x24 => {
                    v[7] += 1;
                    v[8] += class.values[1];
                }
                0x25 => {
                    v[10] += 1;
                    f30 |= 0x80;
                }
                0x22 | 0x23 => {
                    v[11] += 1;
                    v[12] += class.values[0];
                }
                _ => continue,
            }
            v[31] += class.maintenance;
        }
        if v[7] > 1 {
            f30 |= 0x100;
        }
        if sys.is_populated {
            if inputs.uprisings.is_uprising(system) {
                f28 |= 1;
            }
            f30 |= 0x28;
            v[0] = crate::uprising::support_points(
                sys,
                if inputs.is_alliance {
                    crate::dat::Faction::Alliance
                } else {
                    crate::dat::Faction::Empire
                },
            );
            if v[0] < 70 {
                f2c |= 0x1000_0000;
            }
            // FUN_005039d0: squadrons on the planet. port: none; the port's
            // fighters ride fleets.
            if f30 & 1 != 0 && f28 & 3 == 0 && f28 & 0x3e0_0000 == 0 {
                f28 |= 0x400_0000;
                let mut used = v[11] + v[10] + v[3] + v[2] + v[7];
                if f30 & 0x8000_0000 != 0 {
                    used += 1;
                }
                if used < v[1] {
                    f28 |= 0x100;
                    if v[14] > 0 {
                        f28 |= 0x80;
                    }
                }
            }
            let support_term = |v: &mut [i32; 60], below: i32, add: i32| {
                if below - v[0] > 0 {
                    v[20] = (v[20] + (add - v[0]) / 10).min(6);
                }
            };
            if f30 & 1 == 0 {
                if f30 & 4 == 0 {
                    if f30 & 2 != 0 {
                        support_term(&mut v, 90, 99);
                        v[52] = v[11] + v[20];
                        if f30 & 0x200 == 0 {
                            f2c |= 0x40;
                        }
                        if f28 & 0x8000_0000 != 0 {
                            v[50] += 1;
                        }
                    }
                } else {
                    support_term(&mut v, 80, 89);
                    v[52] = v[16] + v[20] + v[11];
                    if v[0] > 50 {
                        f2c |= 0x100;
                    }
                    v[29] += v[18] + v[25];
                    if f28 & 0x8000_0000 != 0 {
                        v[50] += 1;
                    }
                    for (present, bit) in [
                        (v[8] > 0, 0x4_0000),
                        (f30 & 0x80 != 0, 0x8_0000),
                        (v[12] > 0, 0x10_0000),
                        (v[18] > 0, 0x20_0000),
                        (v[25] > 0, 0x40_0000),
                    ] {
                        if present {
                            f2c |= bit;
                        }
                    }
                    let built = [2, 3, 4, 5, 6, 10, 7].iter().any(|&k| v[k] > 0);
                    if built && v[11] == 0 {
                        f30 |= 0x800;
                    }
                    if v[4] > 0 || v[5] > 0 || v[6] > 0 {
                        f2c |= 0x2_0000;
                    }
                    if v[2] > 0 || v[3] > 0 {
                        f2c |= 0x1_0000;
                    }
                }
            } else {
                support_term(&mut v, 70, 79);
                let (mut shields, mut batteries) = (0, 0);
                if f30 & 0x4000_0000 == 0 {
                    if f28 & 0x3e0_0000 == 0 {
                        if v[20] > 0 && S378 + S374 < v[1] {
                            shields = S374;
                            batteries = S378;
                        }
                    } else if S370 + S36C < v[1] {
                        shields = S36C;
                        batteries = S370;
                    } else if S374 < v[1] {
                        shields = S374;
                    }
                } else {
                    let empire = i32::from(side == 2);
                    if v[1] < empire + 2 {
                        shields = v[1] - empire;
                    } else {
                        shields = S368;
                        batteries = S364;
                        if v[1] < empire + S364 + S368 {
                            batteries = v[1] - S368 - empire;
                        }
                    }
                }
                v[9] = shields;
                v[13] = batteries;
                let d_shields = v[7] - shields;
                let d_batteries = v[11] - batteries;
                if d_shields < 1 {
                    if d_shields < 0 {
                        f28 |= 0x8000;
                    }
                    if d_batteries < 1 {
                        v[37] = d_shields + d_batteries;
                        if v[37] < 0 {
                            f28 |= 0x4000;
                        }
                    } else {
                        v[37] = d_batteries;
                    }
                } else {
                    v[37] = if d_batteries < 1 {
                        d_shields
                    } else {
                        d_batteries + d_shields
                    };
                }
                if f28 & 1 != 0 {
                    v[20] <<= 1;
                }
                v[22] = if f28 & 0x8000_0000 != 0 {
                    S384
                } else if f28 & 0x3e0_0000 != 0 {
                    S380
                } else {
                    S37C
                }
                .max(v[20]);
                if f28 & 2 == 0 || f30 & 0x3_0000 == 0 {
                    let goal = v[22];
                    let mut checked = true;
                    if v[18] != goal {
                        if v[18] <= goal {
                            f28 |= 0x2000;
                            v[21] = v[22] - v[18];
                            checked = false;
                        } else {
                            f28 |= 0x1_0000;
                            v[19] = v[18] - goal;
                        }
                    }
                    if checked && v[17] < goal {
                        f28 |= 0x2_0000;
                        v[21] = v[22] - v[17];
                    }
                }
                let less = if f28 & 0x8000_0000 != 0 {
                    S390
                } else if f28 & 0x3e0_0000 != 0 {
                    S38C
                } else if v[17] < 1 {
                    -1
                } else {
                    S388
                };
                if less < 0 {
                    v[38] = v[23];
                    v[53] = v[23];
                } else {
                    v[38] = v[24] - less;
                    v[53] = v[25] - less;
                }
                if v[38] < 0 {
                    f28 |= 0x4_0000;
                }
                if v[53] < 0 {
                    f28 |= 0x10_0000;
                }
                let mut marked = 0;
                if f28 & 0x380_0000 != 0 {
                    marked = v[6];
                }
                if f28 & 0x20_0000 != 0 {
                    marked += v[4];
                }
                if f28 & 0x40_0000 != 0 {
                    marked += v[5];
                }
                let mut used = marked + v[7].max(v[9]) + v[11].max(v[13]);
                if f30 & 0x8000_0000 != 0 {
                    used += 1;
                }
                v[51] = v[1] - used;
                if f28 & 3 == 0 {
                    if inputs.minor_classes > 0 && f30 & 0x800_0000 == 0 {
                        f2c |= 2;
                    }
                    let is_alliance = inputs.is_alliance;
                    let done = |tech| {
                        crate::ai::ai_research_done(world, inputs.research, is_alliance, tech)
                    };
                    for (mark, slot, tech, bit) in [
                        (0x20_0000, 0, crate::research::TechType::Ship, 4),
                        (0x40_0000, 1, crate::research::TechType::Troop, 8),
                        (0x380_0000, 2, crate::research::TechType::Facility, 0x10),
                    ] {
                        if f28 & mark != 0 && !done(tech) && yards[slot] && v[18] > 0 {
                            f2c |= bit;
                        }
                    }
                }
                if f28 & 1 != 0 && f30 & 0x200 == 0 {
                    f2c |= 0x20;
                }
                if f2c & 0x1000_0000 != 0 && f30 & 0x200 == 0 && f28 & 1 == 0 {
                    f2c |= 0x40;
                }
            }
        } else if f28 & 0x20 != 0 {
            f28 |= 0x100;
            if f28 & 0x40 != 0 {
                f28 |= 0x80;
            }
        }
        if f30 & 1 == 0 {
            f2c |= 0x80;
            if f30 & 4 != 0 {
                f2c |= 0x200;
            }
        } else {
            f2c |= 0x201;
        }
        Some(record(f28, f2c, f30, v))
    }
}

/// The index of a sector record's word at `offset` (from `+0x70`, the
/// keys of `FUN_00419a00`).
pub(crate) const fn sector_at(offset: usize) -> usize {
    (offset - 0x70) / 4
}

/// A sector record (`FUN_00431450`, refreshed by `FUN_004321e0`): the sum
/// of its systems' records (`FUN_00432350`).
///
/// `values[k]` is the word at `+0x70 + 4k`, the key a sector query sorts on
/// (`FUN_00419a00`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectorRecord {
    pub sector: crate::ids::SectorKey,
    /// `+0x24`, cleared at each refresh (`FUN_004fcdd0`).
    pub candidates: u32,
    /// `+0x28`.
    pub state: u32,
    /// `+0x2c`.
    pub summary: u32,
    /// `+0x70..+0x1d4`.
    pub values: [i32; 90],
    /// The system records, in the sector's order.
    pub systems: Vec<SystemRecord>,
    /// `+0x40`: the map position, class `+0x48` (`FUN_00431450` ->
    /// `FUN_00526560`), SECTORSD `x_position`/`y_position`.
    pub position: (i16, i16),
}

impl SectorRecord {
    /// `FUN_004321e0` over `systems`, the sector's system records, for the
    /// sector at map `position`.
    ///
    /// port: a sector holds as many systems as the galaxy gives it; the
    /// original's tests for all 10 (`+0x70`, `+0x74`, `+0x78`) read "all
    /// of them". hyp: the fields `+0x30`/`+0x38` behind `+0x2c` bits
    /// `0x4000000`/`0x10000` and `+0x18c` are untraced and stay clear.
    #[must_use]
    pub fn build(
        sector: crate::ids::SectorKey,
        position: (i16, i16),
        systems: Vec<SystemRecord>,
    ) -> Self {
        let mut r = Self {
            sector,
            candidates: 0,
            state: 0,
            summary: 0,
            values: [0; 90],
            systems: Vec::new(),
            position,
        };
        r.values[sector_at(0x144)] = 100_000;
        for system in &systems {
            r.add_system(system);
        }
        let count = i32::try_from(systems.len()).unwrap_or(i32::MAX);
        let v = &mut r.values;
        if r.summary & 0x100 == 0 {
            v[sector_at(0x144)] = 0;
        }
        if v[sector_at(0x7c)] > 0 {
            v[sector_at(0x8c)] /= v[sector_at(0x7c)];
        }
        if count > 0 && v[sector_at(0x70)] == count {
            r.state |= 0x4000_0000;
        }
        if count > 0 && v[sector_at(0x74)] == count {
            r.summary |= 1;
        }
        if count > 0 && v[sector_at(0x78)] == count {
            r.summary |= 2;
        }
        if v[sector_at(0x19c)] > 0 {
            r.summary |= 0x80;
        }
        r.systems = systems;
        r
    }

    /// `FUN_00432350`: one system record into the sector's.
    #[expect(
        clippy::too_many_lines,
        reason = "One accumulation, in the decompile's order."
    )]
    fn add_system(&mut self, system: &SystemRecord) {
        let (f28, f30, f2c) = (system.state, system.kind, system.summary);
        let p = &system.values;
        let s = &mut self.values;
        let add = |s: &mut [i32; 90], offset: usize, value: i32| s[sector_at(offset)] += value;
        if f30 & 1 == 0 {
            if f30 & 4 == 0 {
                self.summary |= 0x10;
                add(s, 0x84, 1);
                add(s, 0x190, p[0x27]);
                add(s, 0x194, p[0x28]);
                add(s, 0x198, p[0x2b]);
                add(s, 0x11c, p[4]);
                add(s, 0x120, p[5]);
                add(s, 0x124, p[6]);
                if f28 & 0x8000_0000 != 0 {
                    self.candidates |= 0x4000_0000;
                }
                if f30 & 0x20 != 0 {
                    self.summary |= 0x40;
                }
            } else {
                self.summary |= 8;
                add(s, 0x80, 1);
                add(s, 0x114, p[2]);
                add(s, 0x118, p[3]);
                add(s, 0x128, p[7]);
                add(s, 0x12c, p[10]);
                add(s, 0x110, p[1] - p[0xf]);
                add(s, 0x190, p[0x27]);
                add(s, 0x194, p[0x28]);
                add(s, 0x198, p[0x2b]);
                add(s, 0xa0, p[0x34]);
                add(s, 0x130, p[0x21]);
                add(s, 0x11c, p[4]);
                add(s, 0x120, p[5]);
                add(s, 0x124, p[6]);
                for (offset, k) in [(0x134, 0x34), (0x138, 0x17), (0x13c, 0x1b), (0x140, 0x1d)] {
                    let at = sector_at(offset);
                    s[at] = s[at].max(p[k]);
                }
                if f28 & 0x8000_0000 != 0 {
                    self.candidates |= 0x8000_0000;
                }
                if f30 & 0x800 != 0 {
                    self.summary |= 0x100;
                    let at = sector_at(0x144);
                    s[at] = s[at].min(p[8]);
                }
                if f30 & 0x100 == 0 {
                    self.summary |= 0x400;
                }
                if f30 & 0x2000 == 0 {
                    self.summary |= 0x40_0000;
                }
                if f2c & 0x2f7_0000 != 0 {
                    self.summary |= 0x8_0000;
                }
            }
        } else {
            self.summary |= 4;
            add(s, 0x7c, 1);
            for (offset, k) in [
                (0x8c, 0),
                (0x90, 1),
                (0xa8, 2),
                (0xac, 3),
                (0xc8, 7),
                (0xcc, 10),
                (0xd0, 0xb),
                (0xd4, 0x17),
                (0x98, 0xe),
                (0x94, 0xf),
                (0x9c, 0x16),
                (0xf0, 0x10),
                (0xfc, 0x1e),
                (0x100, 0x1f),
                (0x104, 0x20),
                (0x108, 0x21),
                (0xdc, 0x1a),
                (0x190, 0x27),
                (0x194, 0x28),
                (0x198, 0x2b),
            ] {
                add(s, offset, p[k]);
            }
            if f30 & 0x4_0000 != 0 {
                self.summary |= 0x80_0000;
                if f30 & 0x1_0000 != 0 {
                    self.summary |= 0x100_0000;
                } else if f30 & 0x2_0000 != 0 {
                    self.summary |= 0x200_0000;
                }
            }
            if f28 & 2 == 0 {
                self.candidates |= 8;
            }
            let calm = f28 & 3 == 0;
            if calm && f30 & 0x800_0000 == 0 && p[0x33] > 0 {
                for (mark, bit) in [(0x20_0000, 0x20), (0x40_0000, 0x40), (0x380_0000, 0x80)] {
                    if f28 & mark == 0 {
                        self.candidates |= bit;
                    }
                }
            }
            if calm {
                for (state, bit) in [(0x80, 0x100), (0x100, 0x200), (0x20, 0x400), (0x40, 0x800)] {
                    if f28 & state != 0 {
                        self.candidates |= bit;
                    }
                }
                if f28 & 0x80_0000 != 0 && f28 & 0x800 != 0 {
                    self.candidates |= 0x1000;
                }
            }
            if f28 & 0x800 != 0 && f28 & 0x380_0000 == 0 {
                self.candidates |= 0x2000;
            }
            if f28 & 0x3e0_0000 != 0 {
                self.candidates |= 0x2000_0000;
                if f28 & 0x20_0000 != 0 {
                    if f28 & 2 == 0 {
                        self.summary |= 0x2_0000;
                    }
                    add(s, 0xb0, p[4]);
                }
                if f28 & 0x40_0000 != 0 {
                    add(s, 0xb8, p[5]);
                }
            }
            if f28 & 0x380_0000 != 0 {
                add(s, 0xc0, p[6]);
            } else {
                add(s, 0xc4, p[6]);
            }
            if f28 & 0x40_0000 == 0 {
                add(s, 0xbc, p[5]);
            }
            if f28 & 0x20_0000 == 0 {
                add(s, 0xb4, p[4]);
            }
            if f30 & 0x1000_0000 != 0 {
                add(s, 0x188, 1);
                if f30 & 0x2000_0000 == 0 {
                    self.summary |= 0x1000_0000;
                }
            }
            if p[0x25] > 0 {
                self.candidates |= 0x1_0000;
                add(s, 0xe4, 1);
            } else if p[0x25] < 0 {
                self.candidates |= 0x8000;
                add(s, 0xe0, 1);
                if f28 & 0x3e0_0000 != 0 {
                    self.candidates |= 0x4000;
                }
            }
            if f28 & 0x8000_0000 == 0 && p[0xb] > 0 && p[7] > 0 {
                self.candidates |= 0x2_0000;
            }
            if f28 & 0x1_0000 != 0 {
                self.candidates |= 0x8_0000;
                add(s, 0xec, p[0x13]);
                if f28 & 0x1000_0000 != 0 {
                    self.candidates |= 0x10_0000;
                }
                if f28 & 0x2000_0000 != 0 {
                    self.candidates |= 0x20_0000;
                }
                if f30 & 0x4000 != 0 {
                    self.summary |= 0x800;
                }
                if f30 & 0x8000 != 0 {
                    self.summary |= 0x1000;
                }
            }
            if f28 & 0x2000 != 0 {
                self.candidates |= 0x4_0000;
                add(s, 0xe8, p[0x15]);
                if f28 & 0x3e0_0000 != 0 {
                    self.candidates |= 0x80_0000;
                }
            }
            if f28 & 0x2_0000 != 0 {
                self.candidates |= 0x40_0000;
            }
            if p[0x35] > 0 && f30 & 0x4_0000 == 0 && f28 & 2 == 0 {
                self.candidates |= 0x200_0000;
                add(s, 0xf8, p[0x35]);
            }
            if p[0x26] < 0 {
                self.candidates |= 0x100_0000;
                add(s, 0xf4, 1);
                if f28 & 0x3e0_0000 != 0 {
                    self.candidates |= 0x400_0000;
                }
                if f28 & 0x10_0000 != 0 {
                    self.candidates |= 0x1000_0000;
                }
            }
            if f28 & 0x8_0000 != 0 {
                self.candidates |= 0x800_0000;
            }
            if f28 & 1 != 0 {
                self.candidates |= 0x10;
            }
            if f28 & 0x8000_0000 != 0 {
                self.candidates |= 0x4000_0000;
                if f30 & 0x4000_0000 != 0 {
                    self.summary |= 0x200;
                }
                if f28 & 2 == 0 {
                    add(s, 0x1a0, 1);
                }
            }
            if f2c & 0x1000_0000 != 0 {
                self.state |= 0x800_0000;
            }
        }
        add(s, 0x19c, p[0x32]);
        if f2c & 0x800_0000 != 0 {
            add(s, 0x70, 1);
        }
        for (bit, offset) in [(8, 0x74), (0x20, 0x78), (0x10, 0x88)] {
            if f30 & bit != 0 {
                add(s, offset, 1);
            }
        }
        for (bit, summary) in [
            (0x40, 0x20),
            (0x4000_0000, 0x4000_0000),
            (0x8000_0000, 0x8000_0000),
            (0x200, 0x4_0000),
        ] {
            if f30 & bit != 0 {
                self.summary |= summary;
            }
        }
        for (mask, count, state) in [
            (1, Some(0x150), 0),
            (2, Some(0x160), 8),
            (4, Some(0x154), 1),
            (8, Some(0x158), 2),
            (0x10, Some(0x15c), 4),
            (0x20, Some(0x164), 0x10),
            (0x40, Some(0x168), 0x20),
            (0x80, Some(0x16c), 0x40),
            (0xc00, Some(0x170), 0x80),
            (0x3000, Some(0x174), 0x100),
            (0x100, Some(0x178), 0x200),
            (0x200, Some(0x17c), 0x400),
            (0x1_0000, None, 0x800),
            (0x2_0000, None, 0x1000),
            (0x80_0000, None, 0x2000),
            (0xc000, Some(0x180), 0x4000),
            (0x100_0000, Some(0x184), 0x8000),
        ] {
            if f2c & mask != 0 {
                if let Some(offset) = count {
                    add(s, offset, 1);
                }
                self.state |= state;
                if mask == 0x40 && f2c & 0x2000_0000 == 0 {
                    self.state |= 0x400_0000;
                }
            }
        }
    }
}

/// The bits a record's flag word must hold and must not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mask {
    pub require: u32,
    pub forbid: u32,
}

impl Mask {
    /// The query mask `require`/`forbid`.
    #[must_use]
    pub const fn new(require: u32, forbid: u32) -> Self {
        Self { require, forbid }
    }

    const fn admits(self, flags: u32) -> bool {
        flags & self.require == self.require && flags & self.forbid == 0
    }
}

/// A list's order (`FUN_0041be80`, compared by `FUN_0041c250`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Order 1: the lower key first.
    Ascending,
    /// Order 2: the higher key first.
    Descending,
}

/// One sorted list (`FUN_005f59f0`): each entry goes before the first one
/// the comparator puts it ahead of, an equal key on a coin flip
/// (`FUN_0041cd80(10) < 5`).
pub fn ranked<T: Copy>(
    items: impl IntoIterator<Item = (T, i32)>,
    order: Order,
    coin: &mut dyn FnMut() -> bool,
) -> Vec<T> {
    let mut list: Vec<(T, i32)> = Vec::new();
    for (item, key) in items {
        let at = list
            .iter()
            .position(|&(_, other)| match key.cmp(&other) {
                std::cmp::Ordering::Less => order == Order::Ascending,
                std::cmp::Ordering::Greater => order == Order::Descending,
                std::cmp::Ordering::Equal => coin(),
            })
            .unwrap_or(list.len());
        list.insert(at, (item, key));
    }
    list.into_iter().map(|(item, _)| item).collect()
}

/// `FUN_004357b0` called until it gives nothing: each call takes the best
/// entry ([`pick`]) and removes it from the first list, so this is the
/// order the calls give.
pub fn picks<T: Copy + PartialEq>(mut lists: Vec<Vec<T>>) -> Vec<T> {
    let mut out = Vec::new();
    while let Some(best) = pick(&lists) {
        lists[0].retain(|&x| x != best);
        out.push(best);
    }
    out
}

/// `FUN_004357b0` over lists merged by `FUN_00435790`: an entry of the
/// first list that every list holds, with the highest sum of its places
/// (`FUN_0041c230` ranks from 1), the earliest on a tie.
pub fn pick<T: Copy + PartialEq>(lists: &[Vec<T>]) -> Option<T> {
    let (first, rest) = lists.split_first()?;
    let mut best: Option<(T, usize)> = None;
    for (at, &item) in first.iter().enumerate() {
        let mut score = at;
        let mut everywhere = true;
        for list in rest {
            match list.iter().position(|&x| x == item) {
                Some(place) => score += place,
                None => everywhere = false,
            }
        }
        if everywhere && best.is_none_or(|(_, top)| top < score) {
            best = Some((item, score));
        }
    }
    best.map(|(item, _)| item)
}

/// The AI's records of the galaxy for one side, as phase 4 refreshes them
/// (`FUN_00418500`): class, fleet, system and sector records.
#[derive(Debug, Clone, Default)]
pub struct Galaxy {
    pub classes: ClassRecords,
    pub fleets: HashMap<crate::ids::FleetKey, FleetRecord>,
    /// One per sector, in the world's sector order.
    pub sectors: Vec<SectorRecord>,
}

impl Galaxy {
    /// Build every record. `marks` holds each system's yard marks
    /// (`+0x28 & 0x3e00000`) and `kinds` its kept `+0x30` bits
    /// ([`KEPT_KIND`]), which only the production modules change.
    ///
    /// port: the original refreshes the sectors (with their systems)
    /// before the fleets, so a system reads the previous refresh's fleet
    /// records; the port builds the fleets first.
    #[must_use]
    pub fn build(
        inputs: &SystemInputs<'_>,
        movement: &crate::movement::MovementState,
        transport: &crate::troop_transport::TroopTransportState,
        marks: &HashMap<crate::ids::SystemKey, u32>,
        kinds: &HashMap<crate::ids::SystemKey, u32>,
    ) -> Self {
        let world = inputs.world;
        let classes = inputs.classes.clone();
        let fleets = world
            .fleets
            .keys()
            .filter_map(|key| {
                FleetRecord::build(
                    world,
                    &classes,
                    key,
                    inputs.is_alliance,
                    movement,
                    transport,
                    inputs.person,
                )
                .map(|r| (key, r))
            })
            .collect();
        let inputs = SystemInputs {
            fleets: &fleets,
            ..*inputs
        };
        let sectors = world
            .sectors
            .keys()
            .map(|sector| {
                let systems: Vec<SystemRecord> = world
                    .systems
                    .iter()
                    .filter(|(_, sys)| sys.sector == sector)
                    .filter_map(|(key, _)| {
                        let kept = |map: &HashMap<_, u32>| map.get(&key).copied().unwrap_or(0);
                        SystemRecord::build_kept(&inputs, key, kept(marks), kept(kinds))
                    })
                    .collect();
                let position = world
                    .sectors
                    .get(sector)
                    .map_or((0, 0), |s| map_position(s.x, s.y));
                SectorRecord::build(sector, position, systems)
            })
            .collect();
        Self {
            classes,
            fleets,
            sectors,
        }
    }

    /// `FUN_00419a00`: the sectors whose `+0x24`, `+0x28`, `+0x2c` pass
    /// `masks`, sorted on `values[key]` (`+0x70 + 4·key`).
    pub fn sector_query(
        &self,
        masks: [Mask; 3],
        key: usize,
        order: Order,
        coin: &mut dyn FnMut() -> bool,
    ) -> Vec<crate::ids::SectorKey> {
        ranked(
            self.sectors
                .iter()
                .filter(|s| {
                    masks[0].admits(s.candidates)
                        && masks[1].admits(s.state)
                        && masks[2].admits(s.summary)
                })
                .map(|s| (s.sector, s.values.get(key).copied().unwrap_or(0))),
            order,
            coin,
        )
    }

    /// `FUN_0041a340` -> `FUN_00431830`: `sector`'s systems whose `+0x28`,
    /// `+0x2c`, `+0x30` pass `masks`, sorted on `v[key]` (`+0x48 + 4·key`).
    pub fn system_query(
        &self,
        sector: crate::ids::SectorKey,
        masks: [Mask; 3],
        key: usize,
        order: Order,
        coin: &mut dyn FnMut() -> bool,
    ) -> Vec<crate::ids::SystemKey> {
        let Some(record) = self.sectors.iter().find(|s| s.sector == sector) else {
            return Vec::new();
        };
        ranked(
            record
                .systems
                .iter()
                .filter(|r| {
                    masks[0].admits(r.state)
                        && masks[1].admits(r.summary)
                        && masks[2].admits(r.kind)
                })
                .map(|r| (r.system, r.values.get(key).copied().unwrap_or(0))),
            order,
            coin,
        )
    }

    /// `FUN_00419b80`: the sectors whose `+0x24`, `+0x28`, `+0x2c` pass
    /// `masks`, sorted on their squared distance (`FUN_0041b7b0`) from the
    /// sector of `origin`, the object the walk up its containers starts at
    /// (here its system). Empty when `origin` has no record.
    pub fn sectors_by_distance(
        &self,
        origin: crate::ids::SystemKey,
        masks: [Mask; 3],
        order: Order,
        coin: &mut dyn FnMut() -> bool,
    ) -> Vec<crate::ids::SectorKey> {
        let Some(home) = self
            .sectors
            .iter()
            .find(|s| s.systems.iter().any(|r| r.system == origin))
        else {
            return Vec::new();
        };
        ranked(
            self.sectors
                .iter()
                .filter(|s| {
                    masks[0].admits(s.candidates)
                        && masks[1].admits(s.state)
                        && masks[2].admits(s.summary)
                })
                .map(|s| (s.sector, squared_distance(home.position, s.position))),
            order,
            coin,
        )
    }

    /// `FUN_0041a400` -> `FUN_00431c00`: `sector`'s systems whose `+0x28`,
    /// `+0x2c`, `+0x30` pass `masks`, sorted on their squared distance from
    /// `origin`'s system record (`+0x34`). Empty when either has no record.
    pub fn systems_by_distance(
        &self,
        sector: crate::ids::SectorKey,
        origin: crate::ids::SystemKey,
        masks: [Mask; 3],
        order: Order,
        coin: &mut dyn FnMut() -> bool,
    ) -> Vec<crate::ids::SystemKey> {
        let (Some(record), Some(home)) = (
            self.sectors.iter().find(|s| s.sector == sector),
            self.system(origin),
        ) else {
            return Vec::new();
        };
        ranked(
            record
                .systems
                .iter()
                .filter(|r| {
                    masks[0].admits(r.state)
                        && masks[1].admits(r.summary)
                        && masks[2].admits(r.kind)
                })
                .map(|r| (r.system, squared_distance(home.position, r.position))),
            order,
            coin,
        )
    }

    /// Set and clear bits of `system`'s record state `+0x28` (a module's
    /// marks, seen at once; the sector records wait for the next refresh,
    /// `FUN_00433e30` only marks the record dirty).
    pub fn update_state(&mut self, system: crate::ids::SystemKey, set: u32, clear: u32) {
        if let Some(r) = self.system_mut(system) {
            r.state = (r.state & !clear) | set;
        }
    }

    /// Set and clear bits of `system`'s record `+0x30`.
    pub fn update_kind(&mut self, system: crate::ids::SystemKey, set: u32, clear: u32) {
        if let Some(r) = self.system_mut(system) {
            r.kind = (r.kind & !clear) | set;
        }
    }

    fn system_mut(&mut self, system: crate::ids::SystemKey) -> Option<&mut SystemRecord> {
        self.sectors
            .iter_mut()
            .flat_map(|s| &mut s.systems)
            .find(|r| r.system == system)
    }

    /// A sector's record.
    #[must_use]
    pub fn sector(&self, sector: crate::ids::SectorKey) -> Option<&SectorRecord> {
        self.sectors.iter().find(|s| s.sector == sector)
    }

    /// A system's record.
    #[must_use]
    pub fn system(&self, system: crate::ids::SystemKey) -> Option<&SystemRecord> {
        self.sectors
            .iter()
            .flat_map(|s| &s.systems)
            .find(|r| r.system == system)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{BuildableClass, CapitalShipClass, FighterClass};

    /// An Alliance system at the map's origin.
    fn system(world: &mut GameWorld) -> crate::ids::SystemKey {
        world.systems.insert(crate::world::System {
            dat_id: DatId::new(0x9000_0001),
            name: "System".into(),
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
            control: crate::world::ControlKind::Controlled(crate::dat::Faction::Alliance),
        })
    }

    #[test]
    fn a_fleet_record_sums_its_ships_and_what_they_carry() {
        // FUN_00433620 over FUN_00484630 and FUN_00433780: a gunship that
        // carries 2 of 3 squadrons and 1 of 2 regiments, and a bare hull.
        let mut world = GameWorld::default();
        let here = system(&mut world);
        let gunship = world.capital_ship_classes.insert(CapitalShipClass {
            turbolaser_fore: 2,
            fighter_capacity: 3,
            troop_capacity: 2,
            ..ship(0x40)
        });
        let hull = world.capital_ship_classes.insert(CapitalShipClass {
            hull: 1200,
            maintenance_cost: 2,
            ..ship(0x41)
        });
        let wing = world.fighter_classes.insert(FighterClass {
            squadron_size: 3,
            ion_cannon_fore: 1,
            maintenance_cost: 1,
            bombardment_defense: 2,
            ..FighterClass::default()
        });
        let regiment_class = DatId::new(0x1000_0001);
        world.buildable_classes.insert(
            regiment_class,
            BuildableClass {
                maintenance_cost: 3,
                ..BuildableClass::default()
            },
        );
        world.troop_classes.insert(
            regiment_class,
            crate::world::TroopClassDef {
                attack_strength: 5,
                defense_strength: 1,
                detection: 0,
            },
        );
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: regiment_class,
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[here].ground_units.push(troop);
        let ships = [gunship, hull].map(|class| crate::world::ShipInstance::new(class, 500, true));
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: here,
            capital_ships: ships.to_vec(),
            fighters: vec![crate::world::FighterEntry {
                class: wing,
                count: 2,
            }],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        world.systems[here].fleets.push(fleet);
        let mut transport = crate::troop_transport::TroopTransportState::new();
        transport.embark(&mut world, fleet, &[troop]).unwrap();
        let classes = ClassRecords::build(&world);
        let movement = crate::movement::MovementState::new();
        let build = |is_alliance| {
            FleetRecord::build(
                &world,
                &classes,
                fleet,
                is_alliance,
                &movement,
                &transport,
                &|_| None,
            )
            .unwrap()
        };

        let r = build(true);

        // Ships, hulls 500 + 1200; attack 200 × recharge 2 / 2 + 2 × 300;
        // bombardment defense 2 × 2; regiment attack 5.
        assert_eq!(r.values[..3], [2, 1700, 800]);
        assert_eq!(r.values[25], 800);
        assert_eq!((r.values[6], r.values[7]), (4, 5));
        // Squadrons 2, active 2, room 3; regiments 1, active 1, room 2.
        assert_eq!(r.values[8..14], [2, 2, 3, 1, 1, 2]);
        // Maintenance: ships 5 + 2, regiments 3, squadrons 2.
        assert_eq!(r.values[14..17], [7, 3, 2]);
        // Active ships 2, gunships 1, units 4 + 1, free room 1 and 1.
        assert_eq!(r.values[17..24], [2, 1, 0, 0, 5, 1, 1]);
        assert_eq!(
            r.flags,
            4 | FLEET_OWN | FLEET_STANDING | 0x8000 | 0x2_0000 | 0x4_0000 | 0x8_0000 | 0x10_0000
        );
        assert_eq!(build(false).flags & FLEET_OWN, 0);
    }

    /// A world for the system records: an Alliance system with energy 10
    /// (2 used) and raw materials 5 (1 used), support 50, two defensive
    /// regiments, an idle shipyard, a 0x24 shield and a 0x22 battery.
    struct SystemFixture {
        world: GameWorld,
        system: crate::ids::SystemKey,
        economy: crate::economy::EconomyState,
    }

    fn system_fixture() -> SystemFixture {
        let mut world = GameWorld::default();
        let system = system(&mut world);
        world.systems[system].total_energy = 10;
        world.systems[system].raw_materials = 5;
        let mut economy = crate::economy::EconomyState::default();
        economy.per_system.insert(
            system,
            crate::economy::SystemEconomy {
                energy_allocated: 2,
                raw_material_allocated: 1,
                ..crate::economy::SystemEconomy::default()
            },
        );
        let class = |maintenance| BuildableClass {
            is_alliance: true,
            is_empire: true,
            maintenance_cost: maintenance,
            ..BuildableClass::default()
        };
        let regiment = DatId::new(0x1000_0001);
        world.buildable_classes.insert(regiment, class(3));
        world.troop_classes.insert(
            regiment,
            crate::world::TroopClassDef {
                attack_strength: 1,
                defense_strength: 5,
                detection: 0,
            },
        );
        for _ in 0..2 {
            let troop = world.troops.insert(crate::world::TroopUnit {
                class_dat_id: regiment,
                is_alliance: true,
                regiment_strength: 100,
            });
            world.systems[system].ground_units.push(troop);
        }
        let shipyard = DatId::new(0x2800_0001);
        world.buildable_classes.insert(shipyard, class(4));
        let yard =
            world
                .manufacturing_facilities
                .insert(crate::world::ManufacturingFacilityInstance {
                    class_dat_id: shipyard,
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: true,
                });
        world.systems[system].manufacturing_facilities.push(yard);
        for (raw, stats) in [
            (
                0x2400_0001,
                crate::world::ClassStats {
                    shield_strength: 7,
                    ..crate::world::ClassStats::default()
                },
            ),
            (
                0x2200_0001,
                crate::world::ClassStats {
                    attack_strength: 9,
                    ..crate::world::ClassStats::default()
                },
            ),
        ] {
            let id = DatId::new(raw);
            world
                .buildable_classes
                .insert(id, BuildableClass { stats, ..class(1) });
            let defense = world
                .defense_facilities
                .insert(crate::world::DefenseFacilityInstance {
                    class_dat_id: id,
                    side: crate::dat::Faction::Alliance,
                });
            world.systems[system].defense_facilities.push(defense);
        }
        SystemFixture {
            world,
            system,
            economy,
        }
    }

    fn system_record(fixture: &SystemFixture, is_alliance: bool, marks: u32) -> SystemRecord {
        system_record_with(fixture, is_alliance, marks, 1)
    }

    fn system_record_with(
        fixture: &SystemFixture,
        is_alliance: bool,
        marks: u32,
        minor_classes: i32,
    ) -> SystemRecord {
        let classes = ClassRecords::build(&fixture.world);
        let fleets = HashMap::new();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = crate::manufacturing::ManufacturingState::new();
        let research = crate::research::ResearchState::new();
        let inputs = SystemInputs {
            world: &fixture.world,
            classes: &classes,
            fleets: &fleets,
            person: &|_| None,
            economy: &fixture.economy,
            uprisings: &uprisings,
            manufacturing: &manufacturing,
            research: &research,
            is_alliance,
            minor_classes,
        };
        SystemRecord::build(&inputs, fixture.system, marks).unwrap()
    }

    #[test]
    fn an_own_system_record_counts_its_garrison_defenses_and_room() {
        // FUN_00433e40 for an own, populated, unmarked system.
        let fixture = system_fixture();

        let r = system_record(&fixture, true, 0);

        // +0x30: own (0x21), populated (0x28).
        assert_eq!(r.kind, 0x29);
        // +0x28: defensive regiments counted (0x20000000); a yard
        // (0x1000), idle (0x200); free energy and raw (0x20, 0x40); unmarked
        // and safe (0x4000000) with room for a mine (0x100, 0x80).
        assert_eq!(
            r.state,
            0x2000_0000 | 0x1000 | 0x200 | 0x20 | 0x40 | 0x400_0000 | 0x100 | 0x80
        );
        // +0x2c: explored, support below 70, minor classes (2), safe from
        // fleets (0x40), own (0x201).
        assert_eq!(r.summary, 0x800_0000 | 0x1000_0000 | 2 | 0x40 | 0x201);
        let v = &r.values;
        // Support 50, energy 10, free raw 4 and energy 8.
        assert_eq!((v[0], v[1], v[14], v[15]), (50, 10, 4, 8));
        // A shipyard, maintenance 4 in all and in shipyards.
        assert_eq!((v[4], v[33], v[34]), (1, 4, 4));
        // The shield (7) and the battery (9), maintenance 1 each.
        assert_eq!((v[7], v[8], v[11], v[12], v[31]), (1, 7, 1, 9, 2));
        // Regiments 2, counted 2, active 2, maintenance 6.
        assert_eq!((v[16], v[17], v[18], v[30]), (2, 2, 2, 6));
        // Support term (79 - 50) / 10 = 2; shield goal 1, battery goal 0;
        // balance 0 + 1; garrison goal 2; room 10 - (1 + 1) = 8.
        assert_eq!(
            (v[20], v[9], v[13], v[37], v[22], v[51]),
            (2, 1, 0, 1, 2, 8)
        );
    }

    #[test]
    fn an_opponent_s_system_record_lists_its_assets() {
        // FUN_00433e40 seen by the Empire: +0x30 4 | 0x20 | 0x28; support
        // term (89 - 50) / 10 = 3; +0x118 = regiments 2 + 3 + batteries 1;
        // +0x2c 0x40000 (a shield of strength), 0x100000 (battery attack),
        // 0x200000 (active regiments), 0x20000 (a yard), and 0x80 | 0x200.
        let fixture = system_fixture();

        let r = system_record(&fixture, false, 0);

        assert_eq!(r.kind, 0x24 | 0x28);
        assert_eq!(r.values[20], 3);
        assert_eq!(r.values[52], 6);
        assert_eq!(r.values[29], 2);
        assert_eq!(
            r.summary,
            0x800_0000 | 0x1000_0000 | 0x4_0000 | 0x10_0000 | 0x20_0000 | 0x2_0000 | 0x280
        );
        // No goals for another side's system.
        assert_eq!((r.values[9], r.values[22], r.values[51]), (0, 0, 0));
    }

    #[test]
    fn a_neutral_or_unexplored_system_record_says_little() {
        // FUN_00433e40: neutral: +0x30 2, support term (99 - 50) / 10 = 4,
        // +0x118 = batteries + term, +0x2c 0x40 with no foreign fleet;
        // unexplored: +0x2c 0x80 only.
        let mut fixture = system_fixture();
        fixture.world.systems[fixture.system].control = crate::world::ControlKind::Uncontrolled;

        let r = system_record(&fixture, true, 0);

        assert_eq!(r.kind, 2 | 0x28);
        assert_eq!((r.values[20], r.values[52]), (4, 5));
        assert_eq!(r.summary, 0x800_0000 | 0x1000_0000 | 0x40 | 0x80);

        fixture.world.systems[fixture.system].exploration_status =
            crate::dat::ExplorationStatus::Unexplored;
        let r = system_record(&fixture, true, 0);
        assert_eq!((r.kind, r.state, r.summary), (2, 0, 0x80));
        assert_eq!(r.values, [0; 60]);
    }

    #[test]
    fn a_marked_system_sets_its_goals_from_the_side_constants() {
        // FUN_00433e40 with a ship mark (0x200000): shield goal +0x36c 2,
        // battery goal +0x370 1 (1 + 2 < 10); garrison goal +0x380 3;
        // regiments against the goal: 0 - +0x38c 6; room 10 - (shipyard 1
        // + max(1, 2) + max(1, 1)) = 6; the marked yard with regiments
        // makes a research site for ships (+0x2c 4).
        let fixture = system_fixture();

        let r = system_record(&fixture, true, 0x20_0000);

        let v = &r.values;
        assert_eq!((v[9], v[13], v[22]), (2, 1, 3));
        assert_eq!((v[37], v[38], v[53], v[51]), (-1, -6, -6, 6));
        assert_ne!(r.state & 0x20_0000, 0);
        // Shields short (0x8000), the balance short (0x4000), the garrison
        // short (0x20000; active 2 of 3, 0x2000), fighters short (0x40000,
        // 0x100000); no 0x4000000 with a mark.
        for bit in [0x8000, 0x4000, 0x2000, 0x4_0000, 0x10_0000] {
            assert_ne!(r.state & bit, 0, "{bit:#x}");
        }
        assert_eq!(r.state & 0x400_0000, 0);
        assert_eq!(v[21], 1);
        assert_ne!(r.summary & 4, 0);
    }

    #[test]
    fn the_headquarters_and_an_uprising_raise_the_goals() {
        // FUN_00433e40: the headquarters (0x80000000 on +0x28 and +0x30,
        // 0x2000000 on +0x2c) take the garrison goal +0x384 9; an
        // uprising (+0x28 bit 1) doubles the support term and sets +0x2c
        // 0x20 with no foreign fleet.
        let mut fixture = system_fixture();
        fixture.world.systems[fixture.system].is_headquarters = true;

        let r = system_record(&fixture, true, 0);

        assert_ne!(r.state & 0x8000_0000, 0);
        assert_ne!(r.kind & 0x8000_0000, 0);
        assert_ne!(r.summary & 0x200_0000, 0);
        assert_eq!(r.values[22], 9);
        // Regiments against the goal: 0 - +0x390 12.
        assert_eq!(r.values[38], -12);

        let fixture = system_fixture();
        let classes = ClassRecords::build(&fixture.world);
        let fleets = HashMap::new();
        let mut uprisings = crate::uprising::UprisingState::default();
        uprisings.active_uprisings.insert(
            fixture.system,
            crate::uprising::ActiveUprising {
                started_tick: 0,
                next_incident_tick: None,
            },
        );
        let manufacturing = crate::manufacturing::ManufacturingState::new();
        let research = crate::research::ResearchState::new();
        let r = SystemRecord::build(
            &SystemInputs {
                world: &fixture.world,
                classes: &classes,
                fleets: &fleets,
                person: &|_| None,
                economy: &fixture.economy,
                uprisings: &uprisings,
                manufacturing: &manufacturing,
                research: &research,
                is_alliance: true,
                minor_classes: 1,
            },
            fixture.system,
            0,
        )
        .unwrap();
        assert_ne!(r.state & 1, 0);
        assert_eq!(r.values[20], 4);
        assert_eq!(r.summary & (0x20 | 0x40 | 2), 0x20);
        // The uprising bit comes after the yards, so the idle bit stays;
        // the safe-room bit needs no uprising.
        assert_eq!(r.state & (0x400_0000 | 0x200), 0x200);
    }

    #[test]
    fn people_aboard_mark_the_ship_by_side_and_standing() {
        // FUN_00484630 over the person records' +0x30.
        let cases = [
            (0, 0x40_0000, 0),
            (0x40, 0x8_0000, 0),
            (4 | 0x40, 0x20_0000, 0),
            (4, 0x10_0000, 0),
            (4 | 0x20, 0, 0),
            (4 | 8, 0x4_0000, 0),
            (4 | 8 | 0x20 | 0x4000_0000, 0, 1),
            (4 | 0x100_0000, 0x10_0000 | 0x400_0000, 0),
            (4 | 0x200_0000, 0x10_0000 | 0x400_0000 | 0x2000_0000, 0),
            (0x200_0000, 0x40_0000 | 0x2000_0000, 0),
        ];
        for (person, flags, leaders) in cases {
            let mut v: ShipValues = [0; 30];
            assert_eq!(person_aboard(person, &mut v), flags, "{person:#x}");
            assert_eq!(v[18], leaders, "{person:#x}");
        }
    }

    #[test]
    fn each_ship_bit_maps_to_its_fleet_bit() {
        // FUN_00433780.
        let s: ShipValues = std::array::from_fn(|i| i32::try_from(i).unwrap() + 1);
        for (ship, fleet) in [
            (0x100, 0x2_0000),
            (0x200, 0x4_0000),
            (0x40_0000, 0x400_0000),
            (0x2000_0000, 0x800_0000),
            (0x1_0000, 0x200),
            (0x8000, 0x100),
            (0x80, 0x10),
            (0x400, 0x20),
            (0x8_0000, 0x100_0000),
            (0x10_0000, 0x40_0000),
            (0x400_0000, 0x200_0000),
            (0x20_0000, 0x80_0000),
        ] {
            let mut r = FleetRecord::default();
            r.add_ship(ship, &s);
            assert_ne!(r.flags & fleet, 0, "{ship:#x}");
        }
        // Travelling: 0x800, no standing; inactive: neither.
        let mut r = FleetRecord::default();
        r.add_ship(0x800, &s);
        assert_eq!(r.flags & (0x800 | FLEET_STANDING), 0x800);
        let mut r = FleetRecord::default();
        r.add_ship(0x2000, &s);
        assert_eq!(r.flags & (0x800 | FLEET_STANDING | 0x400), 0x400);
        // An inactive ship adds no strength; its rooms still count.
        assert_eq!(r.values[1..14], [0; 13]);
        assert_eq!(r.values[14..17], [12, 13, 14]);
        assert_eq!(r.values[22..24], [0, 0]);
        // 0x801000 clear: 0x8000 and the type counts; set: 0x2000, 0x1000.
        let mut r = FleetRecord::default();
        r.add_ship(1 | 2 | 4, &s);
        assert_eq!(r.flags & 0x8000, 0x8000);
        assert_eq!(r.values[18..21], [1, 1, 1]);
        assert_eq!(r.values[27], 11);
        assert_eq!((r.values[22], r.values[23]), (11 - 9, 8 - 6));
        let mut r = FleetRecord::default();
        r.add_ship(0x80_1000 | 1, &s);
        assert_eq!(r.flags & (0x8000 | 0x2000 | 0x1000), 0x2000 | 0x1000);
        assert_eq!(r.values[18], 0);
        // Leaders aboard (0x40000) add +0xb0; a kept 0x8000000 is not active.
        let mut r = FleetRecord::default();
        r.add_ship(0x4_0000 | 0x800_0000, &s);
        assert_eq!((r.values[24], r.values[17]), (19, 0));
        assert_ne!(r.flags & 0x20_0000, 0);
        // Standing and active: every figure adds in.
        let mut r = FleetRecord::default();
        r.add_ship(0, &s);
        assert_eq!(
            r.values[..17],
            [1, 1, 15, 16, 4, 17, 2, 3, 6, 7, 8, 9, 10, 11, 12, 13, 14]
        );
        assert_eq!((r.values[21], r.values[25], r.values[26]), (18, 20, 21));
        assert_eq!(r.flags, FLEET_STANDING | 0x8000);
    }

    #[test]
    fn a_travelling_fleet_is_neither_standing_nor_roomy() {
        // FUN_00433620: object +0x50 bit 4 sets +0x38 bit 2 and each ship's
        // 0x800, which bars 0x4000 and the free room (0x2800).
        let mut world = GameWorld::default();
        let here = system(&mut world);
        let carrier = world.capital_ship_classes.insert(CapitalShipClass {
            fighter_capacity: 2,
            troop_capacity: 2,
            ..ship(0x40)
        });
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: here,
            capital_ships: vec![crate::world::ShipInstance::new(carrier, 500, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        let classes = ClassRecords::build(&world);
        let mut movement = crate::movement::MovementState::new();
        movement.order(fleet, here, here, 3);
        let transport = crate::troop_transport::TroopTransportState::new();

        let r = FleetRecord::build(
            &world,
            &classes,
            fleet,
            true,
            &movement,
            &transport,
            &|_| None,
        )
        .unwrap();

        assert_eq!(r.flags, 4 | FLEET_EN_ROUTE | 0x800 | 0x8000);
        assert_eq!((r.values[22], r.values[23]), (0, 0));
        assert!(FleetRecord::build(
            &world,
            &classes,
            crate::ids::FleetKey::default(),
            true,
            &movement,
            &transport,
            &|_| None
        )
        .is_none());
    }

    fn record_with(
        fixture: &SystemFixture,
        is_alliance: bool,
        fleets: &HashMap<crate::ids::FleetKey, FleetRecord>,
        person: &dyn Fn(crate::ids::CharacterKey) -> Option<u32>,
        manufacturing: &crate::manufacturing::ManufacturingState,
    ) -> SystemRecord {
        let classes = ClassRecords::build(&fixture.world);
        let uprisings = crate::uprising::UprisingState::default();
        let research = crate::research::ResearchState::new();
        SystemRecord::build(
            &SystemInputs {
                world: &fixture.world,
                classes: &classes,
                fleets,
                person,
                economy: &fixture.economy,
                uprisings: &uprisings,
                manufacturing,
                research: &research,
                is_alliance,
                minor_classes: 0,
            },
            fixture.system,
            0,
        )
        .unwrap()
    }

    #[test]
    fn foreign_fleets_and_people_on_the_planet_mark_the_system() {
        // FUN_00433e40: a standing foreign fleet adds its figures and sets
        // +0x30 0x200 (0x400 with fleet bit 0x10) and +0x2c 0x800000
        // (0x1000000 for an Alliance record); its other bits pass on.
        let mut fixture = system_fixture();
        let fleet = fixture.world.fleets.insert(crate::world::Fleet {
            location: fixture.system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        let own = fixture.world.fleets.insert(crate::world::Fleet {
            location: fixture.system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        fixture.world.systems[fixture.system].fleets = vec![fleet, own];
        let mut values = [0; 30];
        for (i, value) in [(25, 1), (26, 2), (4, 3), (8, 4), (21, 5)] {
            values[i] = value;
        }
        let fleets = HashMap::from([
            (
                fleet,
                FleetRecord {
                    flags: FLEET_STANDING | 0x10 | 0x20_0000 | 0x40_0000 | 0x200_0000 | 0x100_0000,
                    values,
                },
            ),
            (
                own,
                FleetRecord {
                    flags: FLEET_OWN | FLEET_STANDING,
                    values,
                },
            ),
        ]);
        let people: Vec<_> = [
            0x2000_0000,
            0x2000_0000 | 8,
            0x2000_0000 | 0x40,
            0x2000_0000 | 4,
            0x2000_0000 | 4 | 8 | 0x4000_0000,
            0x2000_0000 | 4 | 0x40,
            0x2000_0000 | 4 | 0x20 | 0x200_0000,
            4,
        ]
        .into_iter()
        .map(|flags| {
            let key = fixture.world.characters.insert(crate::world::Character {
                current_system: Some(fixture.system),
                ..crate::world::Character::default()
            });
            (key, flags)
        })
        .collect();
        let person = |key| people.iter().find(|(k, _)| *k == key).map(|(_, f)| *f);
        let manufacturing = crate::manufacturing::ManufacturingState::new();

        let r = record_with(&fixture, true, &fleets, &person, &manufacturing);

        let v = &r.values;
        assert_eq!((v[39], v[40], v[41], v[43], v[29]), (1, 2, 3, 4, 5));
        for bit in [0x200, 0x400, 0x1000, 0x3000] {
            assert_ne!(r.kind & bit, 0, "kind {bit:#x}");
        }
        for bit in [
            0x80_0000, 0x100_0000, 0x2800, 0x8000, 0x4000, 0x400, 0x1000, 0x400_0000,
        ] {
            assert_ne!(r.summary & bit, 0, "summary {bit:#x}");
        }
        // Own: 1 minor, 1 main, 1 prisoner; other: 1 minor, 1 main leader,
        // 1 prisoner, 1 minor en route.
        assert_eq!(
            (v[44], v[45], v[46], v[47], v[48], v[50]),
            (1, 1, 2, 1, 2, 1)
        );
        // A foreign fleet bars +0x2c 0x40.
        assert_eq!(r.summary & 0x40, 0);
    }

    #[test]
    fn a_loyal_system_wants_no_defenses_and_counts_them_all_as_spare() {
        // FUN_00433e40 for an own system at support 100: no support term
        // (70 - 100 <= 0), so no shield or battery goal; the shield and
        // the battery are both spare (+0x9c = 1 + 1).
        let mut fixture = system_fixture();
        fixture.world.systems[fixture.system].popularity_alliance = 1.0;

        let r = system_record(&fixture, true, 0);

        let v = &r.values;
        assert_eq!((v[0], v[20], v[9], v[13], v[37]), (100, 0, 0, 0, 2));
        // Support 70 or more: neither +0x2c 0x10000000 nor 0x40.
        assert_eq!(r.summary & (0x1000_0000 | 0x40), 0);
    }

    #[test]
    fn a_side_without_minor_classes_marks_no_research_site() {
        // FUN_00433e40: +0x2c 2 needs the side's minor class count above 0.
        let fixture = system_fixture();
        assert_eq!(system_record_with(&fixture, true, 0, 1).summary & 2, 2);
        assert_eq!(system_record_with(&fixture, true, 0, 0).summary & 2, 0);
    }

    #[test]
    fn an_offensive_regiment_leaves_the_garrison_short_of_counted_troops() {
        // FUN_00433e40: an unmarked own system counts only regiments that
        // are not offensive (FUN_0042dd00: defense <= attack). Two
        // regiments meet the goal 2 (+0x48 v[18]) but one counts, so +0x28
        // 0x20000 and +0x5c = 2 - 1.
        let mut fixture = system_fixture();
        let offensive = DatId::new(0x1000_0002);
        fixture.world.buildable_classes.insert(
            offensive,
            BuildableClass {
                is_alliance: true,
                ..BuildableClass::default()
            },
        );
        fixture.world.troop_classes.insert(
            offensive,
            crate::world::TroopClassDef {
                attack_strength: 5,
                defense_strength: 1,
                detection: 0,
            },
        );
        let sys = fixture.system;
        let first = fixture.world.systems[sys].ground_units[0];
        fixture.world.troops[first].class_dat_id = offensive;

        let r = system_record(&fixture, true, 0);

        let v = &r.values;
        assert_eq!((v[17], v[18], v[22], v[21]), (1, 2, 2, 1));
        assert_ne!(r.state & 0x2_0000, 0);
        assert_eq!(r.state & 0x2000, 0);
    }

    #[test]
    fn an_opponent_s_headquarters_counts_as_a_target_but_not_a_mine_site() {
        // FUN_00433e40 seen by the Empire: no +0x28 0x4000000 (room for a
        // mine) outside its own systems; the holder's headquarters
        // (FUN_00526a60, +0x28 0x80000000) raises +0x110.
        let mut fixture = system_fixture();
        let r = system_record(&fixture, false, 0);
        assert_eq!(r.state & (0x400_0000 | 0x100 | 0x80), 0);
        assert_eq!(r.values[50], 0);

        fixture.world.systems[fixture.system].is_headquarters = true;
        let r = system_record(&fixture, false, 0);
        assert_eq!(r.values[50], 1);
    }

    #[test]
    fn the_capital_s_defense_goals_follow_its_energy() {
        // FUN_00433e40 for id 0x109 (+0x30 0x40000000): shields +0x368 3
        // and batteries +0x364 3, the batteries cut to the energy left
        // below 0 + 3 + 3; with energy 1 (below 0 + 2) one shield only.
        let mut fixture = system_fixture();
        fixture.world.systems[fixture.system].dat_id = DatId::new(0x9000_0109);
        for (energy, shields, batteries) in [(8, 3, 3), (5, 3, 2), (1, 1, 0)] {
            fixture.world.systems[fixture.system].total_energy = energy;

            let r = system_record(&fixture, true, 0);

            assert_eq!(
                (r.values[9], r.values[13]),
                (shields, batteries),
                "{energy}"
            );
        }
    }

    #[test]
    fn a_system_s_ids_mines_and_yards_set_their_bits() {
        // FUN_00433e40: id 0x121 (+0x30 0x8000000, no +0x2c 2), family 0x92
        // (+0x30 0x40), the holder's mines and refineries, a 0x25 defense
        // (+0x30 0x80) and two 0x24 (+0x30 0x100), a busy shipyard (no
        // 0x200), idle training and construction yards (0x400, 0x800).
        let mut fixture = system_fixture();
        let sys = fixture.system;
        fixture.world.systems[sys].dat_id = DatId::new(0x9200_0121);
        for (is_mine, side) in [
            (true, crate::dat::Faction::Alliance),
            (false, crate::dat::Faction::Alliance),
            (true, crate::dat::Faction::Empire),
        ] {
            let mine = fixture.world.production_facilities.insert(
                crate::world::ProductionFacilityInstance {
                    class_dat_id: DatId::new(0x2c00_0001),
                    side,
                    is_mine,
                },
            );
            fixture.world.systems[sys].production_facilities.push(mine);
        }
        for raw in [0x2500_0001, 0x2400_0001] {
            let id = DatId::new(raw);
            fixture.world.buildable_classes.entry(id).or_default();
            let defense =
                fixture
                    .world
                    .defense_facilities
                    .insert(crate::world::DefenseFacilityInstance {
                        class_dat_id: id,
                        side: crate::dat::Faction::Alliance,
                    });
            fixture.world.systems[sys].defense_facilities.push(defense);
        }
        for raw in [0x2900_0001, 0x2a00_0001] {
            let id = DatId::new(raw);
            fixture.world.buildable_classes.insert(
                id,
                BuildableClass {
                    maintenance_cost: 5,
                    ..BuildableClass::default()
                },
            );
            let yard = fixture.world.manufacturing_facilities.insert(
                crate::world::ManufacturingFacilityInstance {
                    class_dat_id: id,
                    side: crate::dat::Faction::Alliance,
                    is_shipyard: false,
                },
            );
            fixture.world.systems[sys]
                .manufacturing_facilities
                .push(yard);
        }
        let mut manufacturing = crate::manufacturing::ManufacturingState::new();
        manufacturing.enqueue(
            sys,
            crate::manufacturing::QueueItem::new(
                crate::manufacturing::BuildableKind::Troop(DatId::new(0x1000_0001)),
                1,
            ),
        );
        let classes = ClassRecords::build(&fixture.world);
        let fleets = HashMap::new();
        let uprisings = crate::uprising::UprisingState::default();
        let research = crate::research::ResearchState::new();
        let r = SystemRecord::build(
            &SystemInputs {
                world: &fixture.world,
                classes: &classes,
                fleets: &fleets,
                person: &|_| None,
                economy: &fixture.economy,
                uprisings: &uprisings,
                manufacturing: &manufacturing,
                research: &research,
                is_alliance: true,
                minor_classes: 1,
            },
            sys,
            0,
        )
        .unwrap();

        assert_eq!(
            r.kind & (0x800_0000 | 0x40 | 0x80 | 0x100),
            0x800_0000 | 0x40 | 0x80 | 0x100
        );
        assert_eq!(r.summary & 2, 0);
        let v = &r.values;
        assert_eq!((v[2], v[3]), (1, 1));
        assert_eq!((v[4], v[5], v[6], v[10], v[7]), (1, 1, 1, 1, 2));
        assert_eq!((v[33], v[35], v[36]), (14, 5, 5));
        assert_eq!(r.state & (0x200 | 0x400 | 0x800), 0x200 | 0x800);
    }

    #[test]
    fn the_empire_s_capital_takes_its_own_defense_goals() {
        // FUN_00433e40 for system 0x109 on the Empire's side: +0x28
        // 0x80000000, +0x30 0x40000000; energy 10 >= 1 + 3 + 3: shields
        // +0x368 3, batteries +0x364 3; with energy 5: batteries 5 - 3 - 1.
        let mut fixture = system_fixture();
        let sys = fixture.system;
        fixture.world.systems[sys].dat_id = DatId::new(0x9000_0109);
        fixture.world.systems[sys].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);

        let r = system_record(&fixture, false, 0);

        assert_ne!(r.state & 0x8000_0000, 0);
        assert_ne!(r.kind & 0x4000_0000, 0);
        assert_eq!((r.values[9], r.values[13]), (3, 3));
        fixture.world.systems[sys].total_energy = 5;
        let r = system_record(&fixture, false, 0);
        assert_eq!((r.values[9], r.values[13]), (3, 1));
        fixture.world.systems[sys].total_energy = 2;
        let r = system_record(&fixture, false, 0);
        assert_eq!((r.values[9], r.values[13]), (1, 0));
        // The Alliance's 0x109: with energy 2 (not below 0 + 2) shields 3
        // and batteries 2 - 3 - 0; with energy 1, shields 1 - 0.
        fixture.world.systems[sys].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Alliance);
        let r = system_record(&fixture, true, 0);
        assert_eq!((r.values[9], r.values[13]), (3, -1));
        fixture.world.systems[sys].total_energy = 1;
        let r = system_record(&fixture, true, 0);
        assert_eq!((r.values[9], r.values[13]), (1, 0));
    }

    #[test]
    fn an_unpopulated_own_system_with_free_energy_has_room() {
        // FUN_00433e40: unpopulated, free energy (0x20) gives 0x100, and
        // free raw (0x40) 0x180; support stays 100; no goals.
        let mut fixture = system_fixture();
        fixture.world.systems[fixture.system].is_populated = false;

        let r = system_record(&fixture, true, 0);

        assert_eq!(
            r.state & (0x20 | 0x40 | 0x100 | 0x80),
            0x20 | 0x40 | 0x100 | 0x80
        );
        assert_eq!(r.values[0], 100);
        assert_eq!(r.kind & 0x28, 0x20);
        assert_eq!(r.values[22], 0);

        fixture.economy.per_system.clear();
        fixture.world.systems[fixture.system].raw_materials = 0;
        let r = system_record(&fixture, true, 0);
        assert_eq!(r.state & (0x40 | 0x80 | 0x100), 0x100);
    }

    #[test]
    fn a_sector_record_sums_an_own_system() {
        // FUN_004321e0 and FUN_00432350 over the own system of
        // an_own_system_record_counts_its_garrison_defenses_and_room.
        let fixture = system_fixture();
        let system = system_record(&fixture, true, 0);

        let r = SectorRecord::build(crate::ids::SectorKey::default(), (0, 0), vec![system]);

        assert_eq!(
            r.candidates,
            8 | 0x20 | 0x40 | 0x80 | 0x100 | 0x200 | 0x400 | 0x800 | 0x1_0000 | 0x2_0000
        );
        assert_eq!(
            r.state,
            0x800_0000 | 8 | 0x20 | 0x400_0000 | 0x400 | 0x4000_0000
        );
        assert_eq!(r.summary, 4 | 1 | 2);
        let at = |offset| r.values[sector_at(offset)];
        for (offset, value) in [
            (0x7c, 1),
            (0x8c, 50),
            (0x90, 10),
            (0x94, 8),
            (0x98, 4),
            (0x9c, 2),
            (0xb4, 1),
            (0xc8, 1),
            (0xd0, 1),
            (0xe4, 1),
            (0xf0, 2),
            (0x70, 1),
            (0x74, 1),
            (0x78, 1),
            (0x150, 1),
            (0x160, 1),
            (0x168, 1),
            (0x17c, 1),
            (0x144, 0),
        ] {
            assert_eq!(at(offset), value, "{offset:#x}");
        }
    }

    #[test]
    fn a_sector_record_sums_opponent_and_neutral_systems() {
        // FUN_00432350's other branches: the Empire sees the fixture as an
        // opponent's system (+0x2c 8, +0x80, maxima, +0x2c 0x400 without
        // two shields, 0x400000 without officers, 0x80000 for its assets)
        // and a neutral copy (+0x2c 0x10, +0x84).
        let mut fixture = system_fixture();
        let opponent = system_record(&fixture, false, 0);
        fixture.world.systems[fixture.system].control = crate::world::ControlKind::Uncontrolled;
        let neutral = system_record(&fixture, false, 0);

        let r = SectorRecord::build(
            crate::ids::SectorKey::default(),
            (0, 0),
            vec![opponent, neutral],
        );

        let at = |offset| r.values[sector_at(offset)];
        assert_eq!((at(0x80), at(0x84)), (1, 1));
        assert_eq!(at(0x110), 10 - 8);
        assert_eq!((at(0x128), at(0x11c)), (1, 2));
        assert_eq!(at(0x134), opponent.values[0x34]);
        for bit in [8, 0x10, 0x400, 0x40_0000, 0x8_0000] {
            assert_ne!(r.summary & bit, 0, "{bit:#x}");
        }
        assert_eq!(r.summary & 0x100, 0);
    }

    #[test]
    fn repeated_picks_give_the_best_then_the_next() {
        // FUN_004357b0 removes what it takes from the first list, so
        // calling it again gives the next best; an entry missing from
        // another list is never taken.
        assert_eq!(picks(vec![vec![1, 2, 3, 4], vec![2, 3, 4]]), vec![4, 3, 2]);
        assert_eq!(picks(vec![vec![7, 7]]), vec![7]);
        assert!(picks::<u8>(vec![vec![]]).is_empty());
    }

    #[test]
    fn a_module_s_bits_change_one_record_and_its_sector_is_found() {
        // FUN_004daf10 sets 0x800000 and clears 0x3800000 on the record;
        // FUN_004dd120 sets and clears +0x30 0x10000000.
        let (mut galaxy, sectors, systems) = spread_galaxy(&[(0, 0), (9, 9)], &[(1, (1, 1), true)]);
        let before = *galaxy.system(systems[1]).unwrap();
        galaxy.update_state(systems[0], 0x80_0000, 0x380_0000);
        galaxy.update_state(systems[0], 0x100_0000, 0);
        galaxy.update_state(systems[0], 0, 0x100_0000);
        galaxy.update_kind(systems[0], 0x1000_0000 | 0x4_0000, 0);
        galaxy.update_kind(systems[0], 0, 0x4_0000);
        let r = galaxy.system(systems[0]).unwrap();
        assert_eq!(r.state & KEPT_STATE, 0x80_0000);
        assert_eq!(r.kind & KEPT_KIND, 0x1000_0000);
        assert_ne!(r.kind & 1, 0);
        assert_eq!(*galaxy.system(systems[1]).unwrap(), before);
        assert_eq!(galaxy.sector(sectors[1]).unwrap().sector, sectors[1]);
        assert_eq!(galaxy.sector(sectors[1]).unwrap().systems.len(), 1);
        let mut nowhere = slotmap::SlotMap::<crate::ids::SectorKey, ()>::with_key();
        let missing = (0..5).map(|_| nowhere.insert(())).last().unwrap();
        assert!(galaxy.sector(missing).is_none());
    }

    #[test]
    fn a_ranked_list_breaks_ties_by_coin_and_the_pick_needs_every_list() {
        // FUN_005f59f0 / FUN_0041c250 and FUN_004357b0.
        let mut heads = || true;
        let mut tails = || false;
        let items = [(1, 5), (2, 3), (3, 5), (4, 9)];
        assert_eq!(
            ranked(items, Order::Ascending, &mut tails),
            vec![2, 1, 3, 4]
        );
        assert_eq!(
            ranked(items, Order::Ascending, &mut heads),
            vec![2, 3, 1, 4]
        );
        assert_eq!(
            ranked(items, Order::Descending, &mut tails),
            vec![4, 1, 3, 2]
        );
        // The highest place sum among entries in every list, earliest on a
        // tie: 3 is at 2 + 1, 1 at 1 + 2; 4 is missing from the second.
        assert_eq!(pick(&[vec![2, 1, 3, 4], vec![2, 3, 1]]), Some(1));
        assert_eq!(pick(&[vec![2, 3, 1, 4], vec![2, 3, 1]]), Some(1));
        assert_eq!(pick(&[vec![2, 1], vec![2]]), Some(2));
        assert_eq!(pick::<u8>(&[]), None);
        // Places add: 1, 2 and 3 all sum to 2; the earliest wins.
        assert_eq!(pick(&[vec![1, 2, 3], vec![3, 2, 1]]), Some(1));
        assert_eq!(pick(&[vec![1], vec![2]]), None);
    }

    /// The fixture's world with sectors at `sectors` (map positions), one
    /// fixture system in the first, and a copy of it in each sector at each
    /// of `systems` (sector index, position, controlled).
    fn spread_galaxy(
        sectors: &[(u16, u16)],
        systems: &[(usize, (u16, u16), bool)],
    ) -> (
        Galaxy,
        Vec<crate::ids::SectorKey>,
        Vec<crate::ids::SystemKey>,
    ) {
        let mut fixture = system_fixture();
        let keys: Vec<_> = sectors
            .iter()
            .map(|&(x, y)| {
                fixture.world.sectors.insert(crate::world::Sector {
                    dat_id: DatId::new(0x8000_0001),
                    name: "Sector".into(),
                    group: crate::dat::SectorGroup::Core,
                    x,
                    y,
                    systems: vec![],
                })
            })
            .collect();
        fixture.world.systems[fixture.system].sector = keys[0];
        let mut placed = vec![fixture.system];
        for &(at, (x, y), held) in systems {
            let key = system(&mut fixture.world);
            let sys = &mut fixture.world.systems[key];
            sys.sector = keys[at];
            (sys.x, sys.y) = (x, y);
            if !held {
                sys.control = crate::world::ControlKind::Uncontrolled;
            }
            placed.push(key);
        }
        let classes = ClassRecords::build(&fixture.world);
        let fleets = HashMap::new();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = crate::manufacturing::ManufacturingState::new();
        let research = crate::research::ResearchState::new();
        let inputs = SystemInputs {
            world: &fixture.world,
            classes: &classes,
            fleets: &fleets,
            person: &|_| None,
            economy: &fixture.economy,
            uprisings: &uprisings,
            manufacturing: &manufacturing,
            research: &research,
            is_alliance: true,
            minor_classes: 0,
        };
        let galaxy = Galaxy::build(
            &inputs,
            &crate::movement::MovementState::new(),
            &crate::troop_transport::TroopTransportState::new(),
            &HashMap::new(),
            &HashMap::new(),
        );
        (galaxy, keys, placed)
    }

    #[test]
    fn sectors_rank_by_squared_distance_from_the_origin_s_sector() {
        // FUN_00419b80: sector records passing the masks, keyed on
        // FUN_0041b7b0 (dx² + dy²) from the origin's sector's +0x40. From
        // (0, 0): (3, 4) is 25 away, (10, 0) 100. Order 2 puts the farthest
        // first, so FUN_004357b0's pick (the last place) is the nearest.
        let (galaxy, sectors, systems) = spread_galaxy(
            &[(0, 0), (10, 0), (3, 4)],
            &[(1, (0, 0), true), (2, (0, 0), false)],
        );
        let any = Mask::default();
        let mut coin = || false;

        let far_first =
            galaxy.sectors_by_distance(systems[0], [any; 3], Order::Descending, &mut coin);
        assert_eq!(far_first, vec![sectors[1], sectors[2], sectors[0]]);
        assert_eq!(pick(&[far_first]), Some(sectors[0]));
        assert_eq!(
            galaxy.sectors_by_distance(systems[2], [any; 3], Order::Ascending, &mut coin),
            vec![sectors[2], sectors[0], sectors[1]]
        );
        // Sector +0x2c bit 4 (an own system) leaves out the neutral one.
        assert_eq!(
            galaxy.sectors_by_distance(
                systems[0],
                [any, any, Mask::new(4, 0)],
                Order::Descending,
                &mut coin
            ),
            vec![sectors[1], sectors[0]]
        );
        // A key past every system in the galaxy.
        let mut nowhere = slotmap::SlotMap::<crate::ids::SystemKey, ()>::with_key();
        let missing = (0..10).map(|_| nowhere.insert(())).last().unwrap();
        assert!(galaxy
            .sectors_by_distance(missing, [any; 3], Order::Ascending, &mut coin)
            .is_empty());
    }

    #[test]
    fn a_sector_s_systems_rank_by_squared_distance_from_the_origin_system() {
        // FUN_0041a400 -> FUN_00431c00: the sector's system records passing
        // the masks, keyed on the squared distance of their +0x34 from the
        // origin system's. From (1, 1): (4, 5) is 25 away, (1, 3) 4, and
        // the origin 0.
        let (galaxy, sectors, systems) = spread_galaxy(
            &[(0, 0), (50, 50)],
            &[(0, (4, 5), true), (0, (1, 3), false), (1, (2, 2), true)],
        );
        let mut origin_moved = galaxy.clone();
        for r in origin_moved.sectors.iter_mut().flat_map(|s| &mut s.systems) {
            if r.system == systems[0] {
                r.position = (1, 1);
            }
        }
        let any = Mask::default();
        let mut coin = || false;

        assert_eq!(
            origin_moved.systems_by_distance(
                sectors[0],
                systems[0],
                [any; 3],
                Order::Ascending,
                &mut coin
            ),
            vec![systems[0], systems[2], systems[1]]
        );
        // +0x30 bit 1 (own) leaves out the neutral system; the origin may
        // lie in another sector: from (2, 2), (4, 5) is 13 away, (1, 1) 2.
        assert_eq!(
            origin_moved.systems_by_distance(
                sectors[0],
                systems[3],
                [any, any, Mask::new(1, 0)],
                Order::Descending,
                &mut coin
            ),
            vec![systems[1], systems[0]]
        );
        // A key past every system in the galaxy.
        let mut nowhere = slotmap::SlotMap::<crate::ids::SystemKey, ()>::with_key();
        let missing = (0..10).map(|_| nowhere.insert(())).last().unwrap();
        assert!(origin_moved
            .systems_by_distance(sectors[0], missing, [any; 3], Order::Ascending, &mut coin)
            .is_empty());
    }

    #[test]
    fn a_squared_distance_wraps_like_the_original_s_imul() {
        // FUN_0041b7b0 multiplies 32-bit ints: (dx, dy) = (32767, 32767)
        // gives 2 * 1073676289, which wraps.
        assert_eq!(squared_distance((3, 4), (0, 0)), 25);
        assert_eq!(squared_distance((0, 0), (-3, -4)), 25);
        assert_eq!(squared_distance((32767, 32767), (0, 0)), 2_147_352_578);
        assert_eq!(
            squared_distance((32767, 32767), (-1, 0)),
            1_073_741_824_i32.wrapping_add(1_073_676_289)
        );
        assert_eq!(map_position(65535, 2), (-1, 2));
    }

    #[test]
    fn the_galaxy_queries_sectors_and_their_systems_by_mask_and_key() {
        // FUN_00419a00 and FUN_0041a340 over a built galaxy.
        let mut fixture = system_fixture();
        let sector = fixture.world.sectors.insert(crate::world::Sector {
            dat_id: DatId::new(0x8000_0001),
            name: "Sector".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![fixture.system],
        });
        fixture.world.systems[fixture.system].sector = sector;
        let other = system(&mut fixture.world);
        fixture.world.systems[other].sector = sector;
        fixture.world.systems[other].total_energy = 3;
        let hull = fixture.world.capital_ship_classes.insert(ship(0x40));
        let raider = fixture.world.fleets.insert(crate::world::Fleet {
            location: other,
            capital_ships: vec![crate::world::ShipInstance::new(hull, 1, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        fixture.world.systems[other].fleets.push(raider);
        let classes = ClassRecords::build(&fixture.world);
        let fleets = HashMap::new();
        let uprisings = crate::uprising::UprisingState::default();
        let manufacturing = crate::manufacturing::ManufacturingState::new();
        let research = crate::research::ResearchState::new();
        let inputs = SystemInputs {
            world: &fixture.world,
            classes: &classes,
            fleets: &fleets,
            person: &|_| None,
            economy: &fixture.economy,
            uprisings: &uprisings,
            manufacturing: &manufacturing,
            research: &research,
            is_alliance: true,
            minor_classes: 0,
        };
        let marks = HashMap::from([(other, 0x20_0000)]);
        let galaxy = Galaxy::build(
            &inputs,
            &crate::movement::MovementState::new(),
            &crate::troop_transport::TroopTransportState::new(),
            &marks,
            &HashMap::new(),
        );
        let mut coin = || false;

        assert_eq!(galaxy.sectors.len(), 1);
        assert_eq!(galaxy.system(other).unwrap().state & 0x3e0_0000, 0x20_0000);
        // The raider's record reaches its system (+0x30 0x200).
        assert_eq!(galaxy.fleets.len(), 1);
        assert_ne!(galaxy.system(other).unwrap().kind & 0x200, 0);
        // Sectors with own systems (+0x2c bit 4), keyed by +0x7c.
        let any = Mask::default();
        assert_eq!(
            galaxy.sector_query([any, any, Mask::new(4, 0)], 3, Order::Ascending, &mut coin),
            vec![sector]
        );
        assert!(galaxy
            .sector_query([any, any, Mask::new(0, 4)], 3, Order::Ascending, &mut coin)
            .is_empty());
        // Own systems by energy v[1], ascending, then without a ship mark.
        assert_eq!(
            galaxy.system_query(
                sector,
                [any, any, Mask::new(1, 0)],
                1,
                Order::Ascending,
                &mut coin
            ),
            vec![other, fixture.system]
        );
        assert_eq!(
            galaxy.system_query(
                sector,
                [Mask::new(0, 0x20_0000), any, any],
                1,
                Order::Ascending,
                &mut coin
            ),
            vec![fixture.system]
        );
        assert!(galaxy
            .system_query(
                crate::ids::SectorKey::default(),
                [any; 3],
                1,
                Order::Ascending,
                &mut coin
            )
            .is_empty());
        assert!(galaxy.system(crate::ids::SystemKey::default()).is_none());
    }

    #[test]
    fn a_ship_record_scales_its_figures_by_what_it_can_recharge() {
        // FUN_00484630: no shield recharge (class +0x54 0) leaves +0x50 0,
        // no weapon recharge (+0x50 0) leaves the attack 0; the light
        // attack needs class bit 2.
        let mut world = GameWorld::default();
        let here = system(&mut world);
        let dry = world.capital_ship_classes.insert(CapitalShipClass {
            turbolaser_fore: 3,
            weapon_recharge_rate: 0,
            ..ship(0x40)
        });
        let laser = world.capital_ship_classes.insert(CapitalShipClass {
            laser_cannon_fore: 3,
            shield_recharge_rate: 2,
            maneuverability: 1,
            ..ship(0x41)
        });
        let carrier = world.capital_ship_classes.insert(CapitalShipClass {
            laser_cannon_fore: 3,
            troop_capacity: 1,
            maneuverability: 1,
            ..ship(0x42)
        });
        let fleet = crate::world::Fleet {
            location: here,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        };
        let classes = ClassRecords::build(&world);
        let record = |class| {
            ship_record(
                &world,
                &classes,
                &crate::world::ShipInstance::new(class, 1, true),
                &fleet,
                false,
                None,
            )
        };

        let (_, v) = record(dry);
        assert_eq!((v[3], v[14], v[19]), (0, 0, 0));
        let (flags, v) = record(laser);
        assert_ne!(flags & 2, 0);
        assert_eq!(v[3], 200);
        // Light: (1 × 2 × 30) / 3.
        assert_eq!((v[15], v[20]), (20, 20));
        let (flags, v) = record(carrier);
        assert_eq!(flags & 2, 0);
        assert_eq!((v[15], v[20]), (0, 0));
    }

    #[test]
    fn squadrons_and_people_aboard_add_to_the_first_ship() {
        // FUN_00484630: a squadron with lasers adds its light attack and
        // bombardment figure; a person's bits pass to the ship.
        let mut world = GameWorld::default();
        let here = system(&mut world);
        let hull = world.capital_ship_classes.insert(ship(0x40));
        let wing = world.fighter_classes.insert(FighterClass {
            squadron_size: 2,
            laser_cannon_fore: 3,
            maneuverability: 2,
            ..FighterClass::default()
        });
        let officer = world.characters.insert(crate::world::Character::default());
        let fleet = world.fleets.insert(crate::world::Fleet {
            location: here,
            capital_ships: vec![crate::world::ShipInstance::new(hull, 1, true)],
            fighters: vec![crate::world::FighterEntry {
                class: wing,
                count: 1,
            }],
            characters: vec![officer],
            is_alliance: true,
            has_death_star: false,
        });
        let classes = ClassRecords::build(&world);

        let r = FleetRecord::build(
            &world,
            &classes,
            fleet,
            true,
            &crate::movement::MovementState::new(),
            &crate::troop_transport::TroopTransportState::new(),
            &|key| (key == officer).then_some(0),
        )
        .unwrap();

        // Attack 50 × 2 / 3 = 33, light 3 × 2 × 2 × 10 / 3 = 40, +0x38 5.
        assert_eq!((r.values[2], r.values[3], r.values[5]), (33, 40, 5));
        assert_eq!((r.values[25], r.values[26]), (33, 40));
        // An own free person aboard (0x400000) gives the fleet 0x4000000.
        assert_ne!(r.flags & 0x400_0000, 0);
    }

    #[test]
    fn single_ship_bits_set_only_their_fleet_bits() {
        // FUN_00433780 with one of 0x1000 / 0x800000 / bit 2 at a time.
        let s: ShipValues = [3; 30];
        let mut r = FleetRecord::default();
        r.add_ship(0x1000, &s);
        assert_eq!(r.flags & (0x2000 | 0x1000 | 0x8000), 0x1000);
        let mut r = FleetRecord::default();
        r.add_ship(0x80_0000, &s);
        assert_eq!(r.flags & (0x2000 | 0x1000 | 0x8000), 0x2000);
        let mut r = FleetRecord::default();
        r.add_ship(2, &s);
        assert_eq!((r.values[19], r.values[27]), (1, 0));
    }

    #[test]
    fn the_alliance_sees_an_empire_system_as_the_opponent_s() {
        // FUN_00433e40: the opponent of side 1 is 2 (+0x30 4 | 0x20);
        // an unpopulated opponent's system keeps support 0 and free
        // energy sets no +0x28 bit.
        let mut fixture = system_fixture();
        fixture.world.systems[fixture.system].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        fixture.world.systems[fixture.system].is_populated = false;

        let r = system_record(&fixture, true, 0);

        assert_eq!(r.kind, 0x24);
        assert_eq!(r.values[0], 0);
        assert_eq!(r.state & (0x20 | 0x40 | 0x100), 0);
    }

    #[test]
    fn an_enemy_fleet_alone_blockades_the_system() {
        // FUN_00433e40: system +0x88 bit 0x20 sets +0x28 bit 2, which bars
        // the idle and safe-room bits.
        let mut fixture = system_fixture();
        let fleet = fixture.world.fleets.insert(crate::world::Fleet {
            location: fixture.system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        fixture.world.systems[fixture.system].fleets.push(fleet);

        let r = system_record(&fixture, true, 0);

        assert_eq!(r.state & (2 | 0x200 | 0x400_0000), 2);
    }

    #[test]
    fn regiments_count_by_mark_and_class() {
        // FUN_00433e40 (FUN_00504c40): unmarked, an offensive class does
        // not count (+0x30 0x4000, 0x8000 if also defensive) and a
        // defensive one does (+0x28 0x20000000); marked, every regiment
        // counts (threshold 0), with +0x28 0x10000000 / 0x20000000 by class.
        let cases = [
            (0, (5, 1), 0, 0x4000, 0),
            (0, (1, 5), 2, 0, 0x2000_0000),
            (0, (3, 3), 0, 0x4000 | 0x8000, 0),
            (0x20_0000, (5, 1), 2, 0, 0x1000_0000),
            (0x20_0000, (1, 5), 2, 0, 0x2000_0000),
        ];
        for (marks, (attack, defense), counted, kind, state) in cases {
            let mut fixture = system_fixture();
            fixture
                .world
                .troop_classes
                .get_mut(&DatId::new(0x1000_0001))
                .unwrap()
                .attack_strength = attack;
            fixture
                .world
                .troop_classes
                .get_mut(&DatId::new(0x1000_0001))
                .unwrap()
                .defense_strength = defense;

            let r = system_record(&fixture, true, marks);

            assert_eq!(r.values[17], counted, "{marks:#x} {attack} {defense}");
            assert_eq!(
                r.kind & (0x4000 | 0x8000),
                kind,
                "{marks:#x} {attack} {defense}"
            );
            assert_eq!(
                r.state & (0x1000_0000 | 0x2000_0000),
                state,
                "{marks:#x} {attack} {defense}"
            );
        }
        // Another side's system counts nobody.
        let r = system_record(&system_fixture(), false, 0);
        assert_eq!((r.values[16], r.values[17]), (2, 0));
    }

    #[test]
    fn each_fleet_bit_marks_the_system_on_its_own() {
        // FUN_00433e40 (FUN_004ffe70), one fleet record at a time.
        let cases: [(u32, bool, u32, u32); 9] = [
            (FLEET_OWN | FLEET_STANDING, true, 0, 0),
            (0, true, 0, 0),
            (FLEET_STANDING, true, 0x200, 0x80_0000),
            (FLEET_STANDING | 0x10, true, 0x600, 0x180_0000),
            (FLEET_STANDING | 0x10, false, 0x600, 0x80_0000),
            (0x20_0000, true, 0, 0x2800),
            (0x40_0000, true, 0, 0x800),
            (0x200_0000, true, 0x1000, 0),
            (0x100_0000, true, 0, 0x8000),
        ];
        for (flags, is_alliance, kind, summary) in cases {
            let mut fixture = system_fixture();
            let key = fixture.world.fleets.insert(crate::world::Fleet {
                location: fixture.system,
                capital_ships: vec![],
                fighters: vec![],
                characters: vec![],
                is_alliance: !is_alliance,
                has_death_star: false,
            });
            fixture.world.systems[fixture.system].fleets.push(key);
            let fleets = HashMap::from([(
                key,
                FleetRecord {
                    flags,
                    values: [1; 30],
                },
            )]);
            let manufacturing = crate::manufacturing::ManufacturingState::new();

            let r = record_with(&fixture, is_alliance, &fleets, &|_| None, &manufacturing);

            let mask_kind = 0x200 | 0x400 | 0x1000;
            let mask_summary = 0x80_0000 | 0x100_0000 | 0x2800 | 0x8000;
            assert_eq!(r.kind & mask_kind, kind, "{flags:#x}");
            assert_eq!(r.summary & mask_summary, summary, "{flags:#x}");
            let added = if kind & 0x200 == 0 { 0 } else { 1 };
            assert_eq!(r.values[39], added, "{flags:#x}");
        }
    }

    #[test]
    fn each_person_on_the_planet_counts_on_its_own() {
        // FUN_00433e40 (FUN_00536da0), one person record at a time:
        // (flags, +0x30, +0x2c, v[44..=48], v[50]).
        let c = 0x2000_0000;
        let cases: [(u32, u32, u32, [i32; 5], i32); 11] = [
            (c, 0, 0, [0, 1, 0, 0, 0], 0),
            (c | 8, 0, 0, [1, 0, 0, 0, 0], 0),
            (c | 0x40, 0, 0x4000, [0, 0, 1, 0, 0], 0),
            (c | 4, 0, 0x400, [0, 0, 0, 0, 1], 0),
            (c | 4 | 0x20, 0, 0, [0, 0, 0, 0, 1], 0),
            (c | 4 | 8, 0, 0x1400, [0, 0, 0, 1, 0], 0),
            (c | 4 | 8 | 0x4000_0000, 0, 0x1400, [0, 0, 0, 1, 0], 1),
            (c | 4 | 0x100_0000, 0x1000, 0x400, [0, 0, 0, 0, 1], 0),
            (c | 4 | 0x200_0000, 0x3000, 0x400, [0, 0, 0, 0, 1], 0),
            (c | 4 | 0x40, 0, 0x400_0000, [0, 0, 1, 0, 0], 0),
            (4, 0, 0, [0, 0, 0, 0, 0], 0),
        ];
        for (flags, kind, summary, counts, leaders) in cases {
            let mut fixture = system_fixture();
            let key = fixture.world.characters.insert(crate::world::Character {
                current_system: Some(fixture.system),
                ..crate::world::Character::default()
            });
            let manufacturing = crate::manufacturing::ManufacturingState::new();
            let person = |k| (k == key).then_some(flags);

            let r = record_with(&fixture, true, &HashMap::new(), &person, &manufacturing);

            assert_eq!(r.kind & 0x3000, kind, "{flags:#x}");
            assert_eq!(
                r.summary & (0x4000 | 0x400 | 0x1000 | 0x400_0000),
                summary,
                "{flags:#x}"
            );
            assert_eq!(r.values[44..49], counts, "{flags:#x}");
            assert_eq!(r.values[50], leaders, "{flags:#x}");
        }
        // The killed and those aboard a fleet are not on the planet.
        let mut fixture = system_fixture();
        let fleet = fixture.world.fleets.insert(crate::world::Fleet {
            location: fixture.system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        for (is_killed, current_fleet) in [(true, None), (false, Some(fleet))] {
            fixture.world.characters.insert(crate::world::Character {
                current_system: Some(fixture.system),
                is_killed,
                current_fleet,
                ..crate::world::Character::default()
            });
        }
        let manufacturing = crate::manufacturing::ManufacturingState::new();
        let r = record_with(
            &fixture,
            true,
            &HashMap::new(),
            &|_| Some(c),
            &manufacturing,
        );
        assert_eq!(r.values[45], 0);
    }

    #[test]
    fn free_energy_counts_only_for_an_own_system() {
        // FUN_00433e40: +0x28 0x20 needs +0x30 bit 1 and v[15] > 0.
        let fixture = system_fixture();
        assert_eq!(system_record(&fixture, false, 0).state & 0x60, 0);
        let mut fixture = system_fixture();
        fixture
            .economy
            .per_system
            .get_mut(&fixture.system)
            .unwrap()
            .energy_allocated = 10;
        let r = system_record(&fixture, true, 0);
        assert_eq!(r.values[15], 0);
        assert_eq!(r.state & 0x60, 0);
    }

    /// A system for the goal tests: its holder, energy (all free), raw
    /// materials free, support, and how many of each family it holds.
    #[derive(Clone, Copy, Default)]
    struct Build {
        empire: bool,
        energy: u8,
        raw: u8,
        support: f32,
        mines: u8,
        refineries: u8,
        yards: [u8; 3],
        shields: u8,
        shield_strength: u32,
        defense_25: u8,
        batteries: u8,
        battery_attack: u32,
        regiments: u8,
        headquarters: bool,
        id: u32,
    }

    fn built(b: Build) -> SystemFixture {
        let mut world = GameWorld::default();
        let system = system(&mut world);
        let side = if b.empire {
            crate::dat::Faction::Empire
        } else {
            crate::dat::Faction::Alliance
        };
        {
            let sys = &mut world.systems[system];
            sys.control = crate::world::ControlKind::Controlled(side);
            sys.total_energy = b.energy;
            sys.raw_materials = b.raw;
            sys.popularity_alliance = b.support;
            sys.popularity_empire = b.support;
            sys.is_headquarters = b.headquarters;
            if b.id != 0 {
                sys.dat_id = DatId::new(0x9000_0000 | b.id);
            }
        }
        let both = BuildableClass {
            is_alliance: true,
            is_empire: true,
            ..BuildableClass::default()
        };
        for (is_mine, n) in [(true, b.mines), (false, b.refineries)] {
            for _ in 0..n {
                let f =
                    world
                        .production_facilities
                        .insert(crate::world::ProductionFacilityInstance {
                            class_dat_id: DatId::new(0x2c00_0001),
                            side,
                            is_mine,
                        });
                world.systems[system].production_facilities.push(f);
            }
        }
        for (raw, n) in [
            (0x2800_0001, b.yards[0]),
            (0x2900_0001, b.yards[1]),
            (0x2a00_0001, b.yards[2]),
        ] {
            world
                .buildable_classes
                .insert(DatId::new(raw), both.clone());
            for _ in 0..n {
                let f = world.manufacturing_facilities.insert(
                    crate::world::ManufacturingFacilityInstance {
                        class_dat_id: DatId::new(raw),
                        side,
                        is_shipyard: raw == 0x2800_0001,
                    },
                );
                world.systems[system].manufacturing_facilities.push(f);
            }
        }
        for (raw, n, stats) in [
            (
                0x2400_0001,
                b.shields,
                crate::world::ClassStats {
                    shield_strength: b.shield_strength,
                    ..crate::world::ClassStats::default()
                },
            ),
            (
                0x2500_0001,
                b.defense_25,
                crate::world::ClassStats::default(),
            ),
            (
                0x2200_0001,
                b.batteries,
                crate::world::ClassStats {
                    attack_strength: b.battery_attack,
                    ..crate::world::ClassStats::default()
                },
            ),
        ] {
            world.buildable_classes.insert(
                DatId::new(raw),
                BuildableClass {
                    stats,
                    ..both.clone()
                },
            );
            for _ in 0..n {
                let f = world
                    .defense_facilities
                    .insert(crate::world::DefenseFacilityInstance {
                        class_dat_id: DatId::new(raw),
                        side,
                    });
                world.systems[system].defense_facilities.push(f);
            }
        }
        let regiment = DatId::new(0x1000_0001);
        world.buildable_classes.insert(regiment, both);
        world.troop_classes.insert(
            regiment,
            crate::world::TroopClassDef {
                attack_strength: 1,
                defense_strength: 5,
                detection: 0,
            },
        );
        for _ in 0..b.regiments {
            let t = world.troops.insert(crate::world::TroopUnit {
                class_dat_id: regiment,
                is_alliance: !b.empire,
                regiment_strength: 100,
            });
            world.systems[system].ground_units.push(t);
        }
        let mut economy = crate::economy::EconomyState::default();
        economy.per_system.insert(
            system,
            crate::economy::SystemEconomy {
                raw_material_allocated: 0,
                ..crate::economy::SystemEconomy::default()
            },
        );
        SystemFixture {
            world,
            system,
            economy,
        }
    }

    #[test]
    fn support_below_70_and_its_term() {
        // FUN_00433e40: +0x2c 0x10000000 below 70; an own system's term
        // (79 - support) / 10 while support < 70.
        let at = |support| {
            system_record(
                &built(Build {
                    support,
                    energy: 3,
                    ..Build::default()
                }),
                true,
                0,
            )
        };
        let r = at(0.70);
        assert_eq!((r.summary & 0x1000_0000, r.values[20]), (0, 0));
        let r = at(0.69);
        assert_eq!((r.summary & 0x1000_0000, r.values[20]), (0x1000_0000, 1));
        assert_eq!(at(0.0).values[20], 6);
    }

    #[test]
    fn the_safe_room_counts_each_occupant() {
        // FUN_00433e40: an own, calm, unmarked, populated system has room
        // for a mine or refinery (+0x28 0x100, 0x80 with free raw) while
        // its batteries, 0x25 defenses, refineries, mines, shields and the
        // headquarters (+1) leave energy.
        let room = |b: Build| system_record(&built(b), true, 0).state & 0x180;
        let base = Build {
            support: 0.8,
            energy: 1,
            ..Build::default()
        };
        assert_eq!(room(base), 0x100);
        assert_eq!(room(Build { raw: 1, ..base }), 0x180);
        for b in [
            Build {
                batteries: 1,
                ..base
            },
            Build {
                defense_25: 1,
                ..base
            },
            Build {
                refineries: 1,
                ..base
            },
            Build { mines: 1, ..base },
            Build { shields: 1, ..base },
            Build {
                headquarters: true,
                ..base
            },
        ] {
            assert_eq!(room(b), 0);
            assert_eq!(room(Build { energy: 2, ..b }), 0x100);
        }
    }

    #[test]
    fn the_defense_goals_follow_energy_and_marks() {
        // FUN_00433e40's goals: (shields v[9], batteries v[13]).
        let goals = |b: Build, marks| {
            let r = system_record(&built(b), !b.empire, marks);
            (r.values[9], r.values[13])
        };
        let low = Build {
            support: 0.6,
            ..Build::default()
        };
        // Unmarked with a support term: +0x374 1 once 0 + 1 < energy.
        assert_eq!(goals(Build { energy: 1, ..low }, 0), (0, 0));
        assert_eq!(goals(Build { energy: 2, ..low }, 0), (1, 0));
        assert_eq!(
            goals(
                Build {
                    energy: 2,
                    support: 0.8,
                    ..low
                },
                0
            ),
            (0, 0)
        );
        // Marked: (2, 1) once 1 + 2 < energy, else 1 once 1 < energy.
        assert_eq!(goals(Build { energy: 4, ..low }, 0x20_0000), (2, 1));
        assert_eq!(goals(Build { energy: 3, ..low }, 0x20_0000), (1, 0));
        assert_eq!(goals(Build { energy: 1, ..low }, 0x20_0000), (0, 0));
        // System 0x109 for the Alliance: (3, 3) from 0 + 3 + 3 on.
        let capital = Build { id: 0x109, ..low };
        assert_eq!(
            goals(
                Build {
                    energy: 6,
                    ..capital
                },
                0
            ),
            (3, 3)
        );
        assert_eq!(
            goals(
                Build {
                    energy: 5,
                    ..capital
                },
                0
            ),
            (3, 2)
        );
        assert_eq!(
            goals(
                Build {
                    energy: 1,
                    ..capital
                },
                0
            ),
            (1, 0)
        );
        // For the Empire the floor is 1 + 2.
        let empire = Build {
            empire: true,
            ..capital
        };
        assert_eq!(
            goals(
                Build {
                    energy: 2,
                    ..empire
                },
                0
            ),
            (1, 0)
        );
        assert_eq!(
            goals(
                Build {
                    energy: 3,
                    ..empire
                },
                0
            ),
            (3, -1)
        );
        assert_eq!(
            goals(
                Build {
                    energy: 7,
                    ..empire
                },
                0
            ),
            (3, 3)
        );
    }

    #[test]
    fn the_defense_balance_adds_the_shortfalls_and_the_surplus() {
        // FUN_00433e40: d = shields - goal, e = batteries - goal; v[37] is
        // d + e when both are below 1 (+0x28 0x8000 when d < 0, 0x4000 when
        // the sum < 0), e when only e >= 1, d when only d >= 1, d + e when
        // both are. Marked, energy 4: goals (2, 1).
        let balance = |shields, batteries| {
            let r = system_record(
                &built(Build {
                    support: 0.6,
                    energy: 4,
                    shields,
                    batteries,
                    ..Build::default()
                }),
                true,
                0x20_0000,
            );
            (r.values[37], r.state & 0xc000)
        };
        assert_eq!(balance(0, 0), (-3, 0xc000));
        assert_eq!(balance(2, 1), (0, 0));
        assert_eq!(balance(1, 2), (1, 0x8000));
        assert_eq!(balance(3, 0), (1, 0));
        assert_eq!(balance(3, 3), (3, 0));
        assert_eq!(balance(2, 0), (-1, 0x4000));
    }

    #[test]
    fn an_opponent_s_system_lists_each_asset_on_its_own() {
        // FUN_00433e40's opponent branch, one asset at a time:
        // (build, +0x30 bits, +0x2c bits).
        let base = Build {
            empire: true,
            support: 0.5,
            energy: 9,
            ..Build::default()
        };
        let cases = [
            (base, 0, 0),
            (Build { shields: 1, ..base }, 0x800, 0),
            (
                Build {
                    shields: 1,
                    shield_strength: 1,
                    ..base
                },
                0x800,
                0x4_0000,
            ),
            (
                Build {
                    defense_25: 1,
                    ..base
                },
                0x880,
                0x8_0000,
            ),
            (
                Build {
                    batteries: 1,
                    ..base
                },
                0,
                0,
            ),
            (
                Build {
                    batteries: 1,
                    battery_attack: 1,
                    ..base
                },
                0,
                0x10_0000,
            ),
            (
                Build {
                    regiments: 1,
                    ..base
                },
                0,
                0x20_0000,
            ),
            (Build { mines: 1, ..base }, 0x800, 0x1_0000),
            (
                Build {
                    refineries: 1,
                    ..base
                },
                0x800,
                0x1_0000,
            ),
            (
                Build {
                    yards: [1, 0, 0],
                    ..base
                },
                0x800,
                0x2_0000,
            ),
            (
                Build {
                    yards: [0, 1, 0],
                    ..base
                },
                0x800,
                0x2_0000,
            ),
            (
                Build {
                    yards: [0, 0, 1],
                    ..base
                },
                0x800,
                0x2_0000,
            ),
            (
                Build {
                    mines: 1,
                    batteries: 1,
                    ..base
                },
                0,
                0x1_0000,
            ),
        ];
        for (i, (b, kind, summary)) in cases.into_iter().enumerate() {
            let r = system_record(&built(b), true, 0);
            assert_eq!(r.kind & (0x800 | 0x80), kind, "case {i}");
            assert_eq!(
                r.summary & (0x4_0000 | 0x8_0000 | 0x10_0000 | 0x20_0000 | 0x40_0000 | 0x3_0000),
                summary,
                "case {i}"
            );
        }
        // Support above 50 sets 0x100; the regiments count into v[29];
        // the headquarters count once.
        let r = system_record(
            &built(Build {
                support: 0.51,
                regiments: 2,
                ..base
            }),
            true,
            0,
        );
        assert_eq!((r.summary & 0x100, r.values[29]), (0x100, 2));
        let r = system_record(&built(base), true, 0);
        assert_eq!(r.summary & 0x100, 0);
        let r = system_record(
            &built(Build {
                headquarters: true,
                ..base
            }),
            true,
            0,
        );
        assert_eq!(r.values[50], 1);
        // A neutral headquarters counts too; a neutral system without one
        // does not.
        let mut neutral = built(Build {
            headquarters: true,
            ..base
        });
        neutral.world.systems[neutral.system].control = crate::world::ControlKind::Uncontrolled;
        assert_eq!(system_record(&neutral, true, 0).values[50], 0);
        let mut neutral = built(Build { id: 0x109, ..base });
        neutral.world.systems[neutral.system].control = crate::world::ControlKind::Uncontrolled;
        assert_eq!(system_record(&neutral, true, 0).values[50], 1);
    }

    fn synthetic(state: u32, kind: u32, summary: u32, values: &[(usize, i32)]) -> SystemRecord {
        let mut v = [0; 60];
        for &(k, x) in values {
            v[k] = x;
        }
        SystemRecord {
            system: crate::ids::SystemKey::default(),
            state,
            summary,
            kind,
            values: v,
            position: (0, 0),
        }
    }

    fn sector_of(records: Vec<SystemRecord>) -> SectorRecord {
        SectorRecord::build(crate::ids::SectorKey::default(), (0, 0), records)
    }

    #[test]
    fn each_own_system_condition_sets_its_sector_bit() {
        // FUN_00432350's own branch, one condition at a time:
        // (+0x28, +0x30, +0x2c, values, sector +0x24, +0x28, +0x2c).
        let own = 1;
        let room = [(0x33, 1)];
        type Case = (u32, u32, u32, Vec<(usize, i32)>, u32, u32, u32);
        let cases: Vec<Case> = vec![
            (2, own, 0, vec![], 0, 0, 4),
            (0, own, 0, vec![], 8, 0, 4),
            (2, own, 0, room.to_vec(), 0, 0, 4),
            (0, own | 0x800_0000, 0, room.to_vec(), 8, 0, 4),
            (0x20_0000 | 2, own, 0, room.to_vec(), 0x2000_0000, 0, 4),
            (0x80 | 1, own, 0, vec![], 8 | 0x10, 0, 4),
            (0x80, own, 0, vec![], 8 | 0x100, 0, 4),
            (0x100, own, 0, vec![], 8 | 0x200, 0, 4),
            (0x20, own, 0, vec![], 8 | 0x400, 0, 4),
            (0x40, own, 0, vec![], 8 | 0x800, 0, 4),
            (0x80_0800 | 2, own, 0, vec![], 0x2000_0000, 0, 4),
            (0x80_0800, own, 0, vec![], 8 | 0x2000_0000 | 0x1000, 0, 4),
            (0x800, own, 0, vec![], 8 | 0x2000, 0, 4),
            (0x40_0000, own, 0, vec![], 8 | 0x2000_0000, 0, 4),
            (
                0x1_0000 | 2,
                own | 0x4000,
                0,
                vec![],
                0x8_0000,
                0,
                4 | 0x800,
            ),
            (
                0x1_0000 | 0x1000_0000 | 2,
                own | 0x8000,
                0,
                vec![],
                0x18_0000,
                0,
                4 | 0x1000,
            ),
            (0x1_0000 | 0x2000_0000 | 2, own, 0, vec![], 0x28_0000, 0, 4),
            (0x2000 | 2, own, 0, vec![], 0x4_0000, 0, 4),
            (
                0x2000 | 0x20_0000 | 2,
                own,
                0,
                vec![],
                0x84_0000 | 0x2000_0000,
                0,
                4,
            ),
            (0x2_0000 | 2, own, 0, vec![], 0x40_0000, 0, 4),
            (0x8_0000 | 2, own, 0, vec![], 0x800_0000, 0, 4),
            (0x8000_0000 | 2, own, 0, vec![], 0x4000_0000, 0, 4),
            (
                0x8000_0000 | 2,
                own | 0x4000_0000,
                0,
                vec![],
                0x4000_0000,
                0,
                4 | 0x200 | 0x4000_0000,
            ),
            (2, own | 0x4_0000, 0, vec![], 0, 0, 4 | 0x80_0000),
            (2, own | 0x5_0000, 0, vec![], 0, 0, 4 | 0x180_0000),
            (2, own | 0x6_0000, 0, vec![], 0, 0, 4 | 0x280_0000),
            (2, own | 0x1000_0000, 0, vec![], 0, 0, 4 | 0x1000_0000),
            (2, own | 0x3000_0000, 0, vec![], 0, 0, 4),
            (2, own, 0x1000_0000, vec![], 0, 0x800_0000, 4),
            (2, own, 0, vec![(0x25, 1)], 0x1_0000, 0, 4),
            (2, own, 0, vec![(0x25, -1)], 0x8000, 0, 4),
            (
                2 | 0x20_0000,
                own,
                0,
                vec![(0x25, -1)],
                0x8000 | 0x4000 | 0x2000_0000,
                0,
                4,
            ),
            (2, own, 0, vec![(0xb, 1), (7, 1)], 0x2_0000, 0, 4),
            (2, own, 0, vec![(0xb, 1)], 0, 0, 4),
            (2, own, 0, vec![(7, 1)], 0, 0, 4),
            (0x80_0000, own, 0, vec![], 8 | 0x2000_0000, 0, 4),
            (0x20_0000, own, 0, vec![], 8 | 0x2000_0000, 0, 4 | 0x2_0000),
            (
                2 | 0x8000_0000,
                own,
                0,
                vec![(0xb, 1), (7, 1)],
                0x4000_0000,
                0,
                4,
            ),
            (0, own, 0, vec![(0x35, 1)], 8 | 0x200_0000, 0, 4),
            (0, own | 0x4_0000, 0, vec![(0x35, 1)], 8, 0, 4 | 0x80_0000),
            (2, own, 0, vec![(0x26, -1)], 0x100_0000, 0, 4),
            (
                2 | 0x10_0000 | 0x20_0000,
                own,
                0,
                vec![(0x26, -1)],
                0x1500_0000 | 0x2000_0000,
                0,
                4,
            ),
        ];
        for (i, (state, kind, summary, values, candidates, sector_state, sector_summary)) in
            cases.into_iter().enumerate()
        {
            let r = sector_of(vec![synthetic(state, kind, summary, &values)]);
            assert_eq!(r.candidates, candidates, "case {i}");
            assert_eq!(r.state & !0x4000_0000, sector_state, "case {i}");
            assert_eq!(r.summary & !3, sector_summary, "case {i}");
        }
    }

    #[test]
    fn an_own_system_s_yards_count_as_marked_or_free_by_its_marks() {
        // FUN_00432350: shipyards (v[4]) add to +0xb0 under a ship mark
        // (0x200000), else +0xb4; training facilities (v[5]) to +0xb8
        // under 0x400000, else +0xbc; construction yards (v[6]) to +0xc0
        // under 0x3800000, else +0xc4.
        let yards = [(4, 1), (5, 2), (6, 4)];
        for (marks, expected) in [
            (0, [0, 1, 0, 2, 0, 4]),
            (0x20_0000, [1, 0, 0, 2, 0, 4]),
            (0x40_0000, [0, 1, 2, 0, 0, 4]),
            (0x80_0000, [0, 1, 0, 2, 4, 0]),
        ] {
            let r = sector_of(vec![synthetic(marks, 1, 0, &yards)]);
            let got = [0xb0, 0xb4, 0xb8, 0xbc, 0xc0, 0xc4].map(|o| r.values[sector_at(o)]);
            assert_eq!(got, expected, "{marks:#x}");
        }
    }

    #[test]
    fn a_calm_own_headquarters_counts_in_its_sector() {
        // FUN_00432350: an own system with +0x28 0x80000000 adds 1 to
        // +0x1a0 unless blockaded (+0x28 2).
        let at = |state| sector_of(vec![synthetic(state, 1, 0, &[])]).values[sector_at(0x1a0)];
        assert_eq!(at(0x8000_0000), 1);
        assert_eq!(at(0x8000_0000 | 2), 0);
    }

    #[test]
    fn opponent_and_neutral_systems_set_their_sector_bits() {
        // FUN_00432350's other branches and the tail.
        let opponent = 4;
        let r = sector_of(vec![synthetic(
            0x8000_0000,
            opponent | 0x800 | 0x100 | 0x2000,
            0x1_0000,
            &[(8, 7)],
        )]);
        assert_eq!(r.candidates, 0x8000_0000);
        assert_eq!(r.summary & !3, 8 | 0x100 | 0x8_0000);
        assert_eq!(r.values[sector_at(0x144)], 7);
        let r = sector_of(vec![synthetic(0, opponent, 0, &[])]);
        assert_eq!(r.candidates, 0);
        assert_eq!(r.summary & !3, 8 | 0x400 | 0x40_0000);
        assert_eq!(r.values[sector_at(0x144)], 0);
        let r = sector_of(vec![synthetic(0x8000_0000, 0x20, 0, &[])]);
        assert_eq!((r.candidates, r.summary & !3), (0x4000_0000, 0x10 | 0x40));
        let r = sector_of(vec![synthetic(0, 2, 0, &[])]);
        assert_eq!((r.candidates, r.summary & !3), (0, 0x10));
        // The tail: kind bits 8, 0x20 and 0x10 count; 0x40, 0x40000000,
        // 0x80000000, 0x200 pass to +0x2c; v[0x32] adds to +0x19c.
        let r = sector_of(vec![synthetic(
            0,
            2 | 8 | 0x10 | 0x20 | 0x40 | 0x200 | 0x4000_0000 | 0x8000_0000,
            0x800_0000,
            &[(0x32, 2)],
        )]);
        for offset in [0x70, 0x74, 0x78, 0x88] {
            assert_eq!(r.values[sector_at(offset)], 1, "{offset:#x}");
        }
        assert_eq!(r.values[sector_at(0x19c)], 2);
        assert_eq!(
            r.summary,
            0x10 | 0x40 | 0x20 | 0x4000_0000 | 0x8000_0000 | 0x4_0000 | 0x80 | 1 | 2
        );
        assert_eq!(r.state, 0x4000_0000);
    }

    #[test]
    fn the_sector_tail_divides_the_support_and_checks_all_systems() {
        // FUN_004321e0: +0x8c / +0x7c; +0x28 0x40000000, +0x2c 1 and 2
        // when every system is explored, populated (8), or kind 0x20.
        let explored = synthetic(0, 1 | 8 | 0x20, 0x800_0000, &[(0, 60)]);
        let bare = synthetic(0, 1, 0, &[(0, 20)]);
        let r = sector_of(vec![explored, explored]);
        assert_eq!(r.values[sector_at(0x8c)], 60);
        assert_eq!((r.state & 0x4000_0000, r.summary & 3), (0x4000_0000, 3));
        let r = sector_of(vec![explored, bare]);
        assert_eq!(r.values[sector_at(0x8c)], 40);
        assert_eq!((r.state & 0x4000_0000, r.summary & 3), (0, 0));
        let r = sector_of(vec![]);
        assert_eq!((r.state, r.summary, r.values[sector_at(0x8c)]), (0, 0, 0));
    }

    #[test]
    fn the_sector_counts_each_system_summary_bit() {
        // FUN_00432350: each +0x2c bit counts and sets its sector +0x28 bit.
        for (bit, offset, state) in [
            (1, Some(0x150), 0),
            (2, Some(0x160), 8),
            (4, Some(0x154), 1),
            (8, Some(0x158), 2),
            (0x10, Some(0x15c), 4),
            (0x20, Some(0x164), 0x10),
            (0x40, Some(0x168), 0x20 | 0x400_0000),
            (0x80, Some(0x16c), 0x40),
            (0x400, Some(0x170), 0x80),
            (0x800, Some(0x170), 0x80),
            (0x1000, Some(0x174), 0x100),
            (0x2000, Some(0x174), 0x100),
            (0x100, Some(0x178), 0x200),
            (0x200, Some(0x17c), 0x400),
            (0x1_0000, None, 0x800),
            (0x2_0000, None, 0x1000),
            (0x80_0000, None, 0x2000),
            (0x4000, Some(0x180), 0x4000),
            (0x8000, Some(0x180), 0x4000),
            (0x100_0000, Some(0x184), 0x8000),
        ] {
            let r = sector_of(vec![synthetic(0, 2, bit, &[])]);
            assert_eq!(r.state & !0x4000_0000, state, "{bit:#x}");
            if let Some(offset) = offset {
                assert_eq!(r.values[sector_at(offset)], 1, "{bit:#x}");
            }
        }
        let r = sector_of(vec![synthetic(0, 2, 0x40 | 0x2000_0000, &[])]);
        assert_eq!(r.state & !0x4000_0000, 0x20);
    }

    #[test]
    fn the_garrison_and_squadron_goals_compare_counts() {
        // FUN_00433e40: active regiments v[18] against the goal v[22]: fewer
        // set 0x2000 and v[21]; more set 0x10000 and v[19]; then counted
        // regiments v[17] below the goal set 0x20000 and v[21]. Squadrons:
        // an unmarked system with no counted regiment takes v[23].
        let goal = |regiments, marks| {
            let r = system_record(
                &built(Build {
                    support: 0.8,
                    energy: 9,
                    regiments,
                    ..Build::default()
                }),
                true,
                marks,
            );
            (r.state & 0x3_2000, r.values[19], r.values[21], r.values[38])
        };
        // Marked: goal 3.
        assert_eq!(goal(1, 0x20_0000), (0x2000, 0, 2, -6));
        assert_eq!(goal(3, 0x20_0000), (0, 0, 0, -6));
        assert_eq!(goal(4, 0x20_0000), (0x1_0000, 1, 0, -6));
        // Unmarked: goal 0; no counted regiment takes v[23] = 0.
        assert_eq!(goal(0, 0), (0, 0, 0, 0));
        assert_eq!(goal(2, 0), (0x1_0000, 2, 0, 0));
        // A blockade with officers' bits 0x30000 skips the check.
        let mut fixture = built(Build {
            support: 0.8,
            energy: 9,
            regiments: 1,
            ..Build::default()
        });
        let fleet = fixture.world.fleets.insert(crate::world::Fleet {
            location: fixture.system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        fixture.world.systems[fixture.system].fleets.push(fleet);
        let r = system_record(&fixture, true, 0x20_0000);
        assert_eq!(r.state & 0x2000, 0x2000);
    }

    #[test]
    fn marked_yards_take_their_room() {
        // FUN_00433e40: v[51] = energy - (marked yards + max(shields, goal)
        // + max(batteries, goal) + HQ).
        let room = |yards, marks| {
            system_record(
                &built(Build {
                    support: 0.8,
                    energy: 9,
                    yards,
                    ..Build::default()
                }),
                true,
                marks,
            )
            .values[51]
        };
        // Marked (goals 2 and 1): 9 - 3 = 6, less each marked area's yards.
        assert_eq!(room([0, 0, 0], 0x20_0000), 6);
        assert_eq!(room([2, 0, 0], 0x20_0000), 4);
        assert_eq!(room([0, 2, 0], 0x40_0000), 4);
        assert_eq!(room([0, 0, 2], 0x80_0000), 4);
        assert_eq!(room([2, 2, 2], 0x20_0000), 4);
        // The headquarters take one more.
        let r = system_record(
            &built(Build {
                support: 0.8,
                energy: 9,
                headquarters: true,
                ..Build::default()
            }),
            true,
            0,
        );
        assert_eq!(r.values[51], 9 - 1);
    }

    #[test]
    fn research_sites_need_a_mark_a_yard_regiments_and_an_open_tree() {
        // FUN_00433e40 (+0x2c 4, 8, 0x10): the tree's mark, its yard,
        // active regiments, and the tree not Done for the AI.
        let site = |yards: [u8; 3], marks, regiments| {
            system_record(
                &built(Build {
                    support: 0.8,
                    energy: 9,
                    yards,
                    regiments,
                    ..Build::default()
                }),
                true,
                marks,
            )
            .summary
                & 0x1c
        };
        assert_eq!(site([1, 0, 0], 0x20_0000, 1), 4);
        assert_eq!(site([0, 1, 0], 0x40_0000, 1), 8);
        assert_eq!(site([0, 0, 1], 0x80_0000, 1), 0x10);
        assert_eq!(site([1, 1, 1], 0x20_0000, 0), 0);
        assert_eq!(site([0, 1, 1], 0x20_0000, 1), 0);
        assert_eq!(site([1, 1, 1], 0, 1), 0);
        // The minor classes set +0x2c 2 unless the system is 0x121.
        let r = system_record(
            &built(Build {
                energy: 9,
                ..Build::default()
            }),
            true,
            0,
        );
        assert_eq!(r.summary & 2, 2);
        let r = system_record(
            &built(Build {
                energy: 9,
                id: 0x121,
                ..Build::default()
            }),
            true,
            0,
        );
        assert_eq!(r.summary & 2, 0);
    }

    #[test]
    fn the_best_arc_is_the_first_with_the_greatest_figure() {
        // FUN_004739b0's comparisons, fore to starboard.
        assert_eq!(best_arc([5, 1, 1, 1]), 0);
        assert_eq!(best_arc([1, 5, 1, 1]), 1);
        assert_eq!(best_arc([1, 1, 5, 1]), 2);
        assert_eq!(best_arc([1, 1, 1, 5]), 3);
        assert_eq!(best_arc([1, 3, 3, 1]), 1);
        assert_eq!(best_arc([3, 3, 4, 4]), 2);
        assert_eq!(best_arc([0, 0, 0, 0]), 0);
    }

    fn ship(id: u32) -> CapitalShipClass {
        CapitalShipClass {
            dat_id: DatId::new(id),
            refined_material_cost: 10,
            maintenance_cost: 5,
            research_order: 3,
            weapon_recharge_rate: 2,
            hull: 500,
            ..CapitalShipClass::default()
        }
    }

    #[test]
    fn a_ship_with_lasers_only_and_no_troops_gets_bit_2() {
        // FUN_004739b0: no interdictor, troop capacity +0x48 == 0, and a
        // laser in some arc set bit 2; no turbolaser or ion, no bit 1.
        // Fighters (+0x44) set 0x100, bombardment (+0xd4) 0x800 and +0x38.
        let class = CapitalShipClass {
            laser_cannon_port: 6,
            fighter_capacity: 2,
            bombardment_modifier: 1,
            maneuverability: 3,
            ..ship(0x1400_0002)
        };

        let r = ClassRecords::capital_ship(&class);

        assert_eq!(r.flags, CLASS_SHIP | 2 | 0x10 | 0x100 | 0x800);
        // Port arc: 600 / 6 = 100 × 2 / 6 = 33; light 60 × 3 × 2 / 6 = 60.
        assert_eq!((r.values[0], r.values[1]), (33, 60));
        assert_eq!((r.values[2], r.values[5]), (1, 2));
        assert_eq!((r.values[10], r.values[11]), (66, 33));
        assert_eq!(
            (r.refined_cost, r.maintenance, r.research_order, r.side),
            (10, 5, 3, 1)
        );
    }

    #[test]
    fn an_interdictor_never_counts_as_a_gunship_or_an_assault_carrier() {
        // FUN_004739b0: a gravity well (+0xcc) sets 0x400, which bars bit
        // 1, bit 2 and the 0x204 transport bits.
        let class = CapitalShipClass {
            gravity_well_projector: 1,
            turbolaser_fore: 2,
            laser_cannon_fore: 2,
            troop_capacity: 1,
            hull: 2000,
            ..ship(0x1400_0003)
        };

        let r = ClassRecords::capital_ship(&class);

        assert_eq!(r.flags, CLASS_SHIP | CLASS_INTERDICTOR | 0x40 | 0x200);
    }

    #[test]
    fn a_transport_without_guns_is_marked_0x204() {
        // FUN_004739b0: troops, and none of 0x403 with 0x100000, give
        // 0x204; hull 1100 is the middle size.
        let class = CapitalShipClass {
            troop_capacity: 4,
            hull: 1100,
            ..ship(0x1400_0004)
        };

        let r = ClassRecords::capital_ship(&class);

        assert_eq!(r.flags, CLASS_SHIP | 0x204 | 0x20);
    }

    #[test]
    fn the_light_figure_counts_half_the_turbolasers() {
        // FUN_004739b0: fore lasers 1 and turbolasers 4: 10 + 40 / 2 = 30,
        // × maneuverability 2 × recharge 2 / 5 = 24.
        let class = CapitalShipClass {
            turbolaser_fore: 4,
            laser_cannon_fore: 1,
            maneuverability: 2,
            ..ship(0x1400_0006)
        };

        assert_eq!(ClassRecords::capital_ship(&class).values[1], 24);
    }

    #[test]
    fn a_special_family_transport_gets_no_0x204() {
        // FUN_004739b0: the 0x204 bits need 0x100000, which the families
        // 0x18..0x1b never set.
        let class = CapitalShipClass {
            troop_capacity: 2,
            ..ship(crate::world::DEATH_STAR_CLASS_ID)
        };

        let r = ClassRecords::capital_ship(&class);

        assert_eq!(r.flags, CLASS_SHIP_SPECIAL | 0x10 | 0x200);
    }

    #[test]
    fn a_gunship_without_lasers_or_troops_sets_only_bit_1() {
        let class = CapitalShipClass {
            ion_cannon_aft: 3,
            hull: 1999,
            ..ship(0x1400_0005)
        };

        let r = ClassRecords::capital_ship(&class);

        assert_eq!(r.flags, CLASS_SHIP | 1 | 0x20);
        // Aft: 300 × 2 / 3 = 200; turbolasers and lasers 0.
        assert_eq!((r.values[0], r.values[1]), (200, 0));
    }

    #[test]
    fn a_capital_ship_record_rates_its_best_arcs() {
        // FUN_004739b0 with W(fore) = (4, 2, 6), W(aft) = (0, 0, 12):
        // heavy arcs 600 and 0; attack 600 + 100 = 700 and 1200 / 6 = 200;
        // fore wins: 700 × recharge 3 / 12 = 175. Light: fore 60 + 20 = 80,
        // aft 120: aft × maneuverability 5 × 3 / 12 = 150.
        let class = CapitalShipClass {
            dat_id: DatId::new(0x1400_0001),
            is_empire: true,
            refined_material_cost: 35,
            maintenance_cost: 7,
            research_order: 2,
            turbolaser_fore: 4,
            ion_cannon_fore: 2,
            laser_cannon_fore: 6,
            laser_cannon_aft: 12,
            weapon_recharge_rate: 3,
            maneuverability: 5,
            shield_recharge_rate: 4,
            hull: 1500,
            troop_capacity: 2,
            detection: 9,
            ..CapitalShipClass::default()
        };

        let r = ClassRecords::capital_ship(&class);

        assert_eq!(r.values[0], 175);
        assert_eq!(r.values[1], 150);
        assert_eq!(r.values[3], 400);
        assert_eq!((r.values[4], r.values[6], r.values[7]), (1500, 2, 9));
        assert_eq!(r.values[10], 1750 / 7);
        assert_eq!(r.values[11], 1750 / 35);
        assert_eq!(r.side, 2);
        // CLASS_SHIP, heavy guns (1), lasers but troops (no 2), size
        // 0x20, troop carrier 0x200, and 8 with bit 1.
        assert_eq!(r.flags, CLASS_SHIP | 1 | 0x20 | 0x200 | 8);
    }

    #[test]
    fn a_ship_of_the_special_families_skips_the_strengths() {
        // FUN_004739b0: families 0x18..0x1b set 0x80 and jump to the tail.
        let class = CapitalShipClass {
            dat_id: DatId::new(crate::world::DEATH_STAR_CLASS_ID),
            turbolaser_fore: 9,
            weapon_recharge_rate: 3,
            hull: 2500,
            ..CapitalShipClass::default()
        };

        let r = ClassRecords::capital_ship(&class);

        assert_eq!(r.values[0], 0);
        assert_eq!(r.flags, CLASS_SHIP_SPECIAL | 0x40);
        assert_eq!(r.side, 1);
    }

    #[test]
    fn a_fighter_record_rates_its_fore_guns_by_squadron() {
        // FUN_004736c0: squadron 6, ions 2, lasers 3: (200 + 50) × 6 / 5 =
        // 300; maneuverability 4: 3 × 4 × 6 × 10 / 3 = 240, shields 2:
        // 2 × 240 / 8 + 240 = 300.
        let class = FighterClass {
            dat_id: DatId::new(0x1c00_0001),
            squadron_size: 6,
            ion_cannon_fore: 2,
            laser_cannon_fore: 3,
            maneuverability: 4,
            shield_strength: 2,
            torpedoes: 2,
            ..FighterClass::default()
        };

        let r = ClassRecords::fighter_class(&class);

        assert_eq!(r.values[0], 300);
        assert_eq!(r.values[1], 300);
        assert_eq!((r.values[2], r.values[6], r.values[7]), (15, 12, 6));
        assert_eq!(r.flags, CLASS_FIGHTER);
        assert_eq!(r.side, 1);

        // Costs, research and side as the class has them; no guns, no
        // figures.
        let bare = ClassRecords::fighter_class(&FighterClass {
            refined_material_cost: 4,
            maintenance_cost: 1,
            research_order: 2,
            is_empire: true,
            squadron_size: 6,
            ..FighterClass::default()
        });
        assert_eq!(
            (
                bare.refined_cost,
                bare.maintenance,
                bare.research_order,
                bare.side
            ),
            (4, 1, 2, 2)
        );
        assert_eq!((bare.values[0], bare.values[1]), (0, 0));
    }

    #[test]
    fn regiment_defense_and_yard_records_carry_their_family_bits() {
        // FUN_004734d0, FUN_00473290, FUN_00473090.
        let mut world = GameWorld::default();
        let class = |alliance, empire| BuildableClass {
            is_alliance: alliance,
            is_empire: empire,
            refined_material_cost: 10,
            maintenance_cost: 2,
            research_order: 1,
            processing_rate: 5,
            ..BuildableClass::default()
        };
        let regiment = DatId::new(0x1000_0001);
        world.troop_classes.insert(
            regiment,
            crate::world::TroopClassDef {
                attack_strength: 7,
                defense_strength: 7,
                detection: 3,
            },
        );
        world.buildable_classes.insert(regiment, class(true, false));
        world
            .buildable_classes
            .insert(DatId::new(0x2400_0001), class(true, true));
        world
            .buildable_classes
            .insert(DatId::new(0x2300_0001), class(false, true));
        world
            .buildable_classes
            .insert(DatId::new(0x2900_0001), class(true, true));
        world
            .buildable_classes
            .insert(DatId::new(0x2c00_0001), class(true, true));

        let records = ClassRecords::build(&world);

        let troop = records.get(regiment).unwrap();
        assert_eq!(
            (troop.refined_cost, troop.maintenance, troop.research_order),
            (10, 2, 1)
        );
        assert_eq!(troop.flags, CLASS_OFFENSIVE | CLASS_DEFENSIVE);
        assert_eq!(
            (troop.values[0], troop.values[1], troop.values[3]),
            (7, 7, 3)
        );
        assert_eq!(troop.side, 1);
        let shield = records.get(DatId::new(0x2400_0001)).unwrap();
        assert_eq!((shield.flags, shield.side), (CLASS_DEFENSE_24, 0));
        let battery = records.get(DatId::new(0x2300_0001)).unwrap();
        assert_eq!((battery.flags, battery.side), (CLASS_DEFENSE_22, 2));
        let training = records.get(DatId::new(0x2900_0001)).unwrap();
        assert_eq!((training.flags, training.values[0]), (CLASS_TRAINING, 5));
        // FUN_0042dd00 builds no record for mines and refineries.
        assert!(records.get(DatId::new(0x2c00_0001)).is_none());
    }

    #[test]
    fn ships_and_fighters_are_found_by_their_class_keys() {
        // FUN_0042dd00 lists every capital ship and fighter class.
        let mut world = GameWorld::default();
        let carrier = world.capital_ship_classes.insert(CapitalShipClass {
            fighter_capacity: 4,
            ..ship(0x40)
        });
        let wing = world.fighter_classes.insert(FighterClass {
            squadron_size: 3,
            ..FighterClass::default()
        });

        let records = ClassRecords::build(&world);

        assert_eq!(records.ship(carrier).unwrap().values[5], 4);
        assert_eq!(records.fighter(wing).unwrap().values[7], 3);
        assert!(records.get(DatId::new(0x40)).is_none());
    }

    #[test]
    fn every_defense_yard_and_special_force_family_has_its_bit() {
        // FUN_00473290, FUN_00473090, FUN_00472cc0.
        let mut world = GameWorld::default();
        let both = BuildableClass {
            is_alliance: true,
            is_empire: true,
            ..BuildableClass::default()
        };
        let ids = [
            (0x2200_0001, CLASS_DEFENSE_22),
            (0x2500_0001, CLASS_DEFENSE_25),
            (0x2600_0001, 0),
            (0x2800_0001, CLASS_SHIPYARD),
            (0x2a00_0001, CLASS_CONSTRUCTION),
            (0x2b00_0001, 0),
            (0x3c00_0001, 0x100_0000),
            (0x3c00_0006, 0x200_0000),
            (0x3c00_0003, 0x400_0000),
            (0x3c00_0008, 0x800_0000),
            (0x3c00_0009, 0),
        ];
        for (raw, _) in ids {
            world
                .buildable_classes
                .insert(DatId::new(raw), both.clone());
        }
        let mut skills = [crate::world::SkillPair {
            base: 0,
            variance: 0,
        }; 8];
        for (skill, base) in [
            (Skill::Diplomacy, 11),
            (Skill::Espionage, 22),
            (Skill::Combat, 33),
            (Skill::Leadership, 44),
        ] {
            skills[skill as usize].base = base;
        }
        world.special_force_classes.insert(
            DatId::new(0x3c00_0001),
            crate::world::SpecialForceClassDef {
                skills,
                mission_mask: 0,
            },
        );

        let records = ClassRecords::build(&world);

        for (raw, bit) in ids {
            assert_eq!(records.get(DatId::new(raw)).unwrap().flags, bit, "{raw:#x}");
        }
        let force = records.get(DatId::new(0x3c00_0001)).unwrap();
        assert_eq!(force.values[..4], [11, 22, 33, 44]);
        assert_eq!(force.side, 2);
    }
}
