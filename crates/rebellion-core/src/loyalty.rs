//! Character loyalty shifts on galaxy events
//! (`ghidra/notes/betrayal-and-loyalty.md`).
//!
//! [`shift`] is `FUN_00561910`: a side's gain moves every character's base
//! loyalty `+0x66`, up for the side's own characters and down for the
//! others. [`settle`] runs the two handlers the port can see from one day
//! to the next: Coruscant changing side (`FUN_005616a0`) and the Alliance
//! losing its headquarters (`FUN_00561780`).
//!
//! port: the original runs each handler on its own notification, as the
//! change happens; the port compares the galaxy at the day's start
//! ([`watch`]) with its end ([`settle`]), so two changes in one day count
//! once.

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::ids::SystemKey;
use crate::world::{Character, ControlKind, GameWorld};

/// Coruscant's record index (`0x109`, `FUN_00561670`).
const CORUSCANT: u32 = 0x109;

/// SDPRTB 2561 (`Sdprtb_2561_*`): the spread of a side's gain when it takes
/// Coruscant, the same for both sides.
const SDPRTB_CORUSCANT_SPREAD: u16 = 2561;
/// SDPRTB 2561 as shipped.
const SHIPPED_CORUSCANT_SPREAD: i64 = 5;
/// SDPRTB 2562 (`Sdprtb_2562_*`): the base of that gain.
const SDPRTB_CORUSCANT_BASE: u16 = 2562;
/// SDPRTB 2562 as shipped.
const SHIPPED_CORUSCANT_BASE: i64 = 0;
/// GNPRTB 2592: the spread of the Alliance's loss of its headquarters.
const GNPRTB_HQ_SPREAD: u16 = 2592;
/// GNPRTB 2592 as shipped.
const SHIPPED_HQ_SPREAD: i64 = -5;
/// GNPRTB 2593: the base of that loss.
const GNPRTB_HQ_BASE: u16 = 2593;
/// GNPRTB 2593 as shipped.
const SHIPPED_HQ_BASE: i64 = 0;

/// `FUN_004edc80`: the side a character's class serves alone, 1 Alliance
/// or 2 Empire, else none.
///
/// port: the class sides are the port's `is_alliance` and `is_empire`.
fn class_side(character: &Character) -> Option<Faction> {
    match (character.is_alliance, character.is_empire) {
        (true, false) => Some(Faction::Alliance),
        (false, true) => Some(Faction::Empire),
        _ => None,
    }
}

/// `FUN_00561910(amount, side)`: each living character not exempt (class
/// `+0xb4`, `is_unable_to_betray`; `FUN_004ed240`) gets `amount` added to
/// its base loyalty when its class serves `side`, else `amount`
/// subtracted (`FUN_004ef1e0`), clamped to `0..=100` (`FUN_0053e0d0`,
/// `FUN_005341a0`). A neutral `side` changes nothing.
pub fn shift(world: &mut GameWorld, side: Faction, amount: i32) {
    if side == Faction::Neutral || amount == 0 {
        return;
    }
    for c in world.characters.values_mut() {
        if c.is_killed || c.is_unable_to_betray {
            continue;
        }
        let delta = if class_side(c) == Some(side) {
            amount
        } else {
            -amount
        };
        let base = i32::try_from(c.loyalty.base).unwrap_or(i32::MAX);
        c.loyalty.base = u32::try_from((base + delta).clamp(0, 100)).unwrap_or(0);
    }
}

/// What [`settle`] compares across a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoyaltyWatch {
    /// Coruscant's holder, when one side holds it.
    coruscant: Option<Faction>,
    /// Whether the Alliance still has its headquarters.
    alliance_hq: bool,
}

/// A shift [`settle`] applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoyaltyShift {
    pub side: Faction,
    pub amount: i32,
}

fn side_of(control: ControlKind) -> Option<Faction> {
    match control {
        ControlKind::Controlled(side) | ControlKind::Uprising(side) => {
            Some(side).filter(|s| *s != Faction::Neutral)
        }
        ControlKind::Uncontrolled | ControlKind::Contested => None,
    }
}

fn hq_standing(world: &GameWorld, alliance_hq: Option<SystemKey>) -> bool {
    alliance_hq
        .and_then(|key| world.systems.get(key))
        .is_some_and(|sys| sys.is_headquarters && !sys.is_destroyed)
}

/// The galaxy as [`settle`] will compare it: Coruscant's holder and the
/// Alliance's headquarters (`alliance_hq`, the victory state's).
#[must_use]
pub fn watch(world: &GameWorld, alliance_hq: Option<SystemKey>) -> LoyaltyWatch {
    LoyaltyWatch {
        coruscant: world
            .systems
            .values()
            .find(|sys| sys.dat_id.index() == CORUSCANT)
            .and_then(|sys| side_of(sys.control)),
        alliance_hq: hq_standing(world, alliance_hq),
    }
}

fn param(world: &GameWorld, id: u16, shipped: i64, sdprtb: Option<Faction>) -> i32 {
    let value = match sdprtb {
        Some(side) if world.sdprtb.contains(id) => {
            i64::from(world.sdprtb.value(id, world.difficulty_index, side))
        }
        Some(_) => shipped,
        None => crate::movement::gnprtb_or_shipped(world, id, shipped),
    };
    i32::try_from(value).unwrap_or(0)
}

/// `FUN_0053e290(n)`: a draw of `0..=n`, or `n..=0` for a negative `n`,
/// from `draw(m)` (`0..m`).
fn upto(draw: &mut dyn FnMut(i32) -> i32, n: i32) -> i32 {
    if n >= 0 {
        draw(n.saturating_add(1))
    } else {
        -draw(1i32.saturating_sub(n))
    }
}

/// The day's loyalty shifts since `before`:
/// - Coruscant taken by a side (`FUN_00561670` -> `FUN_005616a0`):
///   `rand(SDPRTB 2561) + SDPRTB 2562` (`0..=5`) for the new side
///   (`FUN_0053ec90(new, 1)`); the old side's own call adds nothing
///   (`FUN_0053ec90(old, 0)` is 0).
/// - The Alliance's headquarters lost (`FUN_00561470` -> `FUN_00561780`):
///   `rand(GNPRTB 2592) + GNPRTB 2593` (`0..=-5`) for the Alliance.
///
/// Each draw is taken only when its event happens. Returns the shifts.
///
/// hyp: the side words' bits `0x30`, which make `FUN_005616a0` give
/// nothing, are read as clear.
pub fn settle(
    world: &mut GameWorld,
    before: LoyaltyWatch,
    alliance_hq: Option<SystemKey>,
    draw: &mut dyn FnMut(i32) -> i32,
) -> Vec<LoyaltyShift> {
    let after = watch(world, alliance_hq);
    let mut shifts = Vec::new();
    if after.coruscant != before.coruscant {
        if let Some(side) = after.coruscant {
            let spread = param(
                world,
                SDPRTB_CORUSCANT_SPREAD,
                SHIPPED_CORUSCANT_SPREAD,
                Some(side),
            );
            let base = param(
                world,
                SDPRTB_CORUSCANT_BASE,
                SHIPPED_CORUSCANT_BASE,
                Some(side),
            );
            let amount = upto(draw, spread) + base;
            shift(world, side, amount);
            shifts.push(LoyaltyShift { side, amount });
        }
    }
    if before.alliance_hq && !after.alliance_hq {
        let spread = param(world, GNPRTB_HQ_SPREAD, SHIPPED_HQ_SPREAD, None);
        let base = param(world, GNPRTB_HQ_BASE, SHIPPED_HQ_BASE, None);
        let amount = upto(draw, spread) + base;
        shift(world, Faction::Alliance, amount);
        shifts.push(LoyaltyShift {
            side: Faction::Alliance,
            amount,
        });
    }
    shifts
}

/// Each side's losses at a system, by the lost units' class `+0x58`
/// (`System_vtd0`'s counters on the system record).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideLosses {
    pub alliance: i64,
    pub empire: i64,
}

impl SideLosses {
    /// Add `value` to `side`'s losses; a neutral unit counts for neither.
    pub fn add(&mut self, side: Faction, value: i64) {
        match side {
            Faction::Alliance => self.alliance += value,
            Faction::Empire => self.empire += value,
            Faction::Neutral => {}
        }
    }

    /// Add `other`'s losses.
    pub fn merge(&mut self, other: Self) {
        self.alliance += other.alliance;
        self.empire += other.empire;
    }
}

/// A regiment's loss value, its class `+0x58` (`uprising_defense`), and
/// its side.
#[must_use]
pub fn troop_loss(world: &GameWorld, troop: crate::ids::TroopKey) -> Option<(Faction, i64)> {
    let unit = world.troops.get(troop)?;
    let value = world
        .troop_classes
        .get(&unit.class_dat_id)
        .map_or(0, |class| i64::from(class.uprising_defense));
    Some((Faction::of_alliance(unit.is_alliance), value))
}

/// SDPRTB 2560 (`Sdprtb_2560_*`): the divisor of a battle's or a
/// blockade's losses (`FUN_0053ec10`).
const SDPRTB_LOSS_DIVISOR: u16 = 2560;
/// SDPRTB 2560 as shipped (50 for both sides).
const SHIPPED_LOSS_DIVISOR: i64 = 50;

/// Each side's capital ships at a system, by the value a lost one adds to
/// its side's battle losses (class `+0x58`, `uprising_defense`;
/// `System_vtd0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BattleWatch {
    alliance: i64,
    empire: i64,
}

/// The value of each side's living capital ships orbiting `system`.
#[must_use]
pub fn battle_watch(world: &GameWorld, system: SystemKey) -> BattleWatch {
    let mut watch = BattleWatch::default();
    let Some(sys) = world.systems.get(system) else {
        return watch;
    };
    for fleet in sys.fleets.iter().filter_map(|&key| world.fleets.get(key)) {
        let value: i64 = fleet
            .capital_ships
            .iter()
            .filter(|ship| ship.alive)
            .filter_map(|ship| world.capital_ship_classes.get(ship.class))
            .map(|class| i64::from(class.uprising_defense))
            .sum();
        if fleet.is_alliance {
            watch.alliance += value;
        } else {
            watch.empire += value;
        }
    }
    watch
}

/// `FUN_0053ec10(side, alliance losses, empire losses)`:
/// `(other - own) / SDPRTB 2560 + other / SDPRTB 2560`, read for `side`,
/// with C's truncating division.
fn loss_shift(world: &GameWorld, side: Faction, alliance: i64, empire: i64) -> i32 {
    let (own, other) = match side {
        Faction::Alliance => (alliance, empire),
        Faction::Empire => (empire, alliance),
        Faction::Neutral => return 0,
    };
    let divisor =
        i64::from(param(world, SDPRTB_LOSS_DIVISOR, SHIPPED_LOSS_DIVISOR, Some(side)).max(1));
    i32::try_from((other - own) / divisor + other / divisor).unwrap_or(0)
}

/// A battle's end at `system` (`FUN_00517270` -> `FUN_00560d00`): the
/// winning side's loyalty shift, from each side's losses since `before`
/// (`FUN_0053ec10`). A battle without a winner shifts nothing.
///
/// port: the losses are the capital ships the port tracks; a regiment
/// lost aboard (class `+0x58` too) is not counted, and the port's
/// fighters are worth 0 as FIGHTSD ships them.
pub fn settle_battle(
    world: &mut GameWorld,
    before: BattleWatch,
    system: SystemKey,
    winner: Option<Faction>,
) -> Option<LoyaltyShift> {
    let side = winner.filter(|side| *side != Faction::Neutral)?;
    let after = battle_watch(world, system);
    let alliance = (before.alliance - after.alliance).max(0);
    let empire = (before.empire - after.empire).max(0);
    let amount = loss_shift(world, side, alliance, empire);
    shift(world, side, amount);
    (amount != 0).then_some(LoyaltyShift { side, amount })
}

/// A blockade's end at `system` (`Event305_SystemBlockadeChanged_Handle`
/// -> `FUN_00561280`): the shift for `blockade`'s losses
/// (`FUN_0053ec10`). The side is the blockader (`+0x60`), switched to the
/// other side when the system stands and its holder is not the blockader.
///
/// port: side 3's losses, which `FUN_00509a20` adds for the side that
/// does not blockade, are not kept; a destroyed system's blockade is
/// dropped with it (`BlockadeState::clear_blockade`).
pub fn settle_blockade(
    world: &mut GameWorld,
    system: SystemKey,
    blockade: crate::blockade::BlockadeLosses,
) -> Option<LoyaltyShift> {
    let sys = world.systems.get(system)?;
    let mut side = blockade.blockader;
    if side == Faction::Neutral {
        return None;
    }
    let holder = GameWorld::facility_holder(sys.control);
    if !sys.is_destroyed && holder != Some(side) {
        side = match side {
            Faction::Alliance => Faction::Empire,
            _ => Faction::Alliance,
        };
    }
    let amount = loss_shift(
        world,
        side,
        blockade.losses.alliance,
        blockade.losses.empire,
    );
    shift(world, side, amount);
    (amount != 0).then_some(LoyaltyShift { side, amount })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::ExplorationStatus;
    use crate::ids::{DatId, SectorKey};
    use crate::world::{SkillPair, System};

    fn system(world: &mut GameWorld, index: u32, control: ControlKind) -> SystemKey {
        world.systems.insert(System {
            dat_id: DatId::new(0x9000_0000 | index),
            name: format!("{index:#x}"),
            sector: SectorKey::default(),
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
            fighters: vec![],
            is_headquarters: false,
            is_destroyed: false,
            control,
        })
    }

    fn person(
        world: &mut GameWorld,
        alliance: bool,
        empire: bool,
        loyalty: u32,
    ) -> crate::ids::CharacterKey {
        world.characters.insert(Character {
            is_alliance: alliance,
            is_empire: empire,
            loyalty: SkillPair {
                base: loyalty,
                variance: 0,
            },
            ..Character::default()
        })
    }

    fn loyalty(world: &GameWorld, key: crate::ids::CharacterKey) -> u32 {
        world.characters[key].loyalty.base
    }

    #[test]
    fn a_side_s_gain_raises_its_characters_and_lowers_the_others() {
        // FUN_00561910 -> FUN_004ef1e0: the amount, negated unless the class
        // serves the side (FUN_004edc80), clamped to 0..=100 (FUN_0053e0d0);
        // a class with +0xb4 (is_unable_to_betray) does not move.
        let mut world = GameWorld::default();
        let rebel = person(&mut world, true, false, 50);
        let imperial = person(&mut world, false, true, 50);
        let both = person(&mut world, true, true, 50);
        let high = person(&mut world, true, false, 98);
        let low = person(&mut world, false, true, 2);
        let key = person(&mut world, true, false, 50);
        world.characters[key].is_unable_to_betray = true;
        shift(&mut world, Faction::Alliance, 4);
        assert_eq!(
            [rebel, imperial, both, high, low, key].map(|k| loyalty(&world, k)),
            [54, 46, 46, 100, 0, 50]
        );
        shift(&mut world, Faction::Neutral, 4);
        assert_eq!(loyalty(&world, rebel), 54);
    }

    #[test]
    fn a_battle_s_winner_gains_its_enemy_s_losses_less_its_own_over_fifty() {
        // FUN_00560d00 -> FUN_0053ec10: for the winner w,
        // (L(o) - L(w)) / SDPRTB 2560 + L(o) / SDPRTB 2560, SDPRTB 2560 = 50;
        // the losses are the lost ships' class +0x58 (System_vtd0).
        use crate::world::{CapitalShipClass, Fleet, ShipInstance};
        let mut world = GameWorld::default();
        let here = system(&mut world, 0x101, ControlKind::Controlled(Faction::Empire));
        let star_destroyer = world.capital_ship_classes.insert(CapitalShipClass {
            uprising_defense: 42,
            ..CapitalShipClass::default()
        });
        let frigate = world.capital_ship_classes.insert(CapitalShipClass {
            uprising_defense: 15,
            ..CapitalShipClass::default()
        });
        let fleet = |world: &mut GameWorld, alliance: bool, class, n| {
            let key = world.fleets.insert(Fleet {
                location: here,
                capital_ships: ShipInstance::make(class, 100, alliance, n),
                fighters: vec![],
                characters: vec![],
                is_alliance: alliance,
                has_death_star: false,
            });
            world.systems[here].fleets.push(key);
            key
        };
        let rebels = fleet(&mut world, true, frigate, 2);
        let escort = fleet(&mut world, true, frigate, 1);
        let imperials = fleet(&mut world, false, star_destroyer, 4);
        let rebel = person(&mut world, true, false, 50);
        let imperial = person(&mut world, false, true, 50);
        let before = battle_watch(&world, here);
        // The Alliance loses two frigates (30), the Empire three of its
        // four destroyers (126): (126 - 30) / 50 + 126 / 50 = 1 + 2 = 3.
        world.fleets[rebels].capital_ships[0].alive = false;
        world.fleets[escort].capital_ships[0].alive = false;
        for ship in world.fleets[imperials].capital_ships.iter_mut().take(3) {
            ship.alive = false;
        }
        let mut world_draw = world.clone();
        assert_eq!(
            settle_battle(&mut world, before, here, Some(Faction::Alliance)),
            Some(LoyaltyShift {
                side: Faction::Alliance,
                amount: 3
            })
        );
        assert_eq!(
            (loyalty(&world, rebel), loyalty(&world, imperial)),
            (53, 47)
        );
        // The Empire winning the same losses: (30 - 126) / 50 + 30 / 50 = -1
        // (C truncates).
        let mut world_lost = world_draw.clone();
        assert_eq!(
            settle_battle(&mut world_lost, before, here, Some(Faction::Empire)),
            Some(LoyaltyShift {
                side: Faction::Empire,
                amount: -1
            })
        );
        // No winner, no shift.
        assert_eq!(settle_battle(&mut world_draw, before, here, None), None);
        assert_eq!(loyalty(&world_draw, rebel), 50);
    }

    #[test]
    fn a_blockade_s_end_credits_the_holder_against_its_blockader() {
        // FUN_00561280: the blockader (+0x60), switched to the other side
        // when the standing system is held by the other; FUN_0053ec10 over
        // the assault and bombardment losses (FUN_00509a20), / 50.
        use crate::blockade::BlockadeLosses;
        let mut world = GameWorld::default();
        let here = system(
            &mut world,
            0x101,
            ControlKind::Controlled(Faction::Alliance),
        );
        let rebel = person(&mut world, true, false, 50);
        let blockade = BlockadeLosses {
            blockader: Faction::Empire,
            losses: SideLosses {
                alliance: 20,
                empire: 130,
            },
        };
        // An Imperial blockade of a Rebel world: the Alliance is credited,
        // (130 - 20) / 50 + 130 / 50 = 2 + 2 = 4.
        assert_eq!(
            settle_blockade(&mut world, here, blockade),
            Some(LoyaltyShift {
                side: Faction::Alliance,
                amount: 4
            })
        );
        assert_eq!(loyalty(&world, rebel), 54);
        // Held by the blockader: the Empire, (20 - 130) / 50 + 20 / 50 = -2.
        world.systems[here].control = ControlKind::Controlled(Faction::Empire);
        assert_eq!(
            settle_blockade(&mut world, here, blockade),
            Some(LoyaltyShift {
                side: Faction::Empire,
                amount: -2
            })
        );
        // A destroyed system keeps the blockader.
        world.systems[here].control = ControlKind::Controlled(Faction::Alliance);
        world.systems[here].is_destroyed = true;
        assert_eq!(
            settle_blockade(&mut world, here, blockade).map(|s| s.side),
            Some(Faction::Empire)
        );
        // A Rebel blockade of an Imperial world credits the Empire.
        world.systems[here].is_destroyed = false;
        world.systems[here].control = ControlKind::Controlled(Faction::Empire);
        let rebel_blockade = BlockadeLosses {
            blockader: Faction::Alliance,
            ..blockade
        };
        assert_eq!(
            settle_blockade(&mut world, here, rebel_blockade).map(|s| s.side),
            Some(Faction::Empire)
        );
        // No losses, no shift.
        let quiet = BlockadeLosses {
            blockader: Faction::Empire,
            losses: SideLosses::default(),
        };
        assert_eq!(settle_blockade(&mut world, here, quiet), None);
    }

    #[test]
    fn side_losses_add_by_side_and_merge() {
        let mut losses = SideLosses::default();
        losses.add(Faction::Alliance, 3);
        losses.add(Faction::Empire, 5);
        losses.add(Faction::Neutral, 7);
        losses.merge(SideLosses {
            alliance: 1,
            empire: 2,
        });
        assert_eq!(
            losses,
            SideLosses {
                alliance: 4,
                empire: 7
            }
        );
    }

    #[test]
    fn the_empire_s_gain_raises_its_characters() {
        let mut world = GameWorld::default();
        let rebel = person(&mut world, true, false, 50);
        let imperial = person(&mut world, false, true, 50);
        shift(&mut world, Faction::Empire, 3);
        assert_eq!(
            (loyalty(&world, rebel), loyalty(&world, imperial)),
            (47, 53)
        );
    }

    #[test]
    fn the_shifts_read_the_side_s_sdprtb_and_the_gnprtb() {
        // FUN_0053ec90 reads Sdprtb_2561/2562 for the new side;
        // FUN_0053ed00 reads Gnprtb_2592/2593.
        use crate::world::{GnprtbParams, SdprtbEntry, SdprtbParams};
        let row = |parameter_id, alliance, empire| SdprtbEntry {
            parameter_id,
            dev_alliance: alliance,
            dev_empire: empire,
            alliance_sp_easy_alliance: alliance,
            alliance_sp_easy_empire: empire,
            alliance_sp_medium_alliance: alliance,
            alliance_sp_medium_empire: empire,
            alliance_sp_hard_alliance: alliance,
            alliance_sp_hard_empire: empire,
            empire_sp_easy_alliance: alliance,
            empire_sp_easy_empire: empire,
            empire_sp_medium_alliance: alliance,
            empire_sp_medium_empire: empire,
            empire_sp_hard_alliance: alliance,
            empire_sp_hard_empire: empire,
            multiplayer_alliance: alliance,
            multiplayer_empire: empire,
        };
        let only_base = SdprtbParams::new(vec![row(2562, 3, 7)]);
        assert!(only_base.contains(2562) && !only_base.contains(2561));
        let mut world = GameWorld::default();
        world.sdprtb = SdprtbParams::new(vec![row(2561, 9, 2), row(2562, 3, 7)]);
        world.gnprtb = GnprtbParams::uniform(&[(2592, -2), (2593, -1)]);
        let coruscant = system(&mut world, 0x109, ControlKind::Uncontrolled);
        let hq = system(
            &mut world,
            0x10a,
            ControlKind::Controlled(Faction::Alliance),
        );
        world.systems[hq].is_headquarters = true;
        let mut sizes = Vec::new();
        let mut draw = |m: i32| {
            sizes.push(m);
            1
        };
        let before = watch(&world, Some(hq));
        world.systems[coruscant].control = ControlKind::Controlled(Faction::Empire);
        world.systems[hq].is_destroyed = true;
        let shifts = settle(&mut world, before, Some(hq), &mut draw);
        // Empire: rand(2) + 7 = 1 + 7; Alliance: -rand(2) - 1 = -1 - 1.
        assert_eq!(
            shifts,
            [
                LoyaltyShift {
                    side: Faction::Empire,
                    amount: 8
                },
                LoyaltyShift {
                    side: Faction::Alliance,
                    amount: -2
                }
            ]
        );
        assert_eq!(sizes, [3, 3]);
    }

    #[test]
    fn coruscant_taken_or_the_rebel_headquarters_lost_shift_loyalty() {
        // FUN_005616a0 for Coruscant (0x109): rand(SDPRTB 2561 = 5) + 2562
        // (0) for the new side; FUN_00561780 for the Alliance HQ (family
        // 0x20..0x21): rand(GNPRTB 2592 = -5) + 2593 (0).
        let mut world = GameWorld::default();
        let coruscant = system(&mut world, 0x109, ControlKind::Controlled(Faction::Empire));
        let hq = system(
            &mut world,
            0x10a,
            ControlKind::Controlled(Faction::Alliance),
        );
        world.systems[hq].is_headquarters = true;
        let rebel = person(&mut world, true, false, 50);
        let imperial = person(&mut world, false, true, 50);
        let mut sizes = Vec::new();
        let mut draw = |m: i32| {
            sizes.push(m);
            m - 1
        };

        let before = watch(&world, Some(hq));
        assert!(settle(&mut world, before, Some(hq), &mut draw).is_empty());

        world.systems[coruscant].control = ControlKind::Controlled(Faction::Alliance);
        let shifts = settle(&mut world, before, Some(hq), &mut draw);
        assert_eq!(
            shifts,
            [LoyaltyShift {
                side: Faction::Alliance,
                amount: 5
            }]
        );
        assert_eq!(
            (loyalty(&world, rebel), loyalty(&world, imperial)),
            (55, 45)
        );

        // Coruscant left neutral gives nothing; the HQ lost costs the
        // Alliance 0..=5 (here 5).
        let before = watch(&world, Some(hq));
        world.systems[coruscant].control = ControlKind::Uncontrolled;
        world.systems[hq].is_headquarters = false;
        let shifts = settle(&mut world, before, Some(hq), &mut draw);
        assert_eq!(
            shifts,
            [LoyaltyShift {
                side: Faction::Alliance,
                amount: -5
            }]
        );
        assert_eq!(
            (loyalty(&world, rebel), loyalty(&world, imperial)),
            (50, 50)
        );
        assert_eq!(sizes, [6, 6]);
    }
}
