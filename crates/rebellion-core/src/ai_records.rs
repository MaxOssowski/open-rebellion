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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{BuildableClass, CapitalShipClass, FighterClass};

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
