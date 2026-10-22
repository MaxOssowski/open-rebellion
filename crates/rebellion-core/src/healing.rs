//! Injury recovery: each character's healing timer `0x388`
//! (`ghidra/notes/character-injury.md`).
//!
//! A character heals while its Healing flag (`+0xac` bit `0x40`) is set:
//! `FUN_004ef030` sets it when the character can heal (`CanHeal`, bit `0x2`),
//! is not en route (`+0x50` bit `0x10`) and is injured (`+0x94`). `CanHeal`
//! (`FUN_004eef90`) holds while the character's container is of its side
//! and it is neither captured nor destroyed. The flag's hook (slot `+0x360`,
//! `FUN_004f1b40`) arms timer `0x388` with it on record `+0x54`+`0x18`, whose
//! minimum is GNPRTB 2561 (1) with `FastHeal` (bit `0x4`) or 2562 (1) without
//! (`FUN_004ef750`, `FUN_004f1af0`). Each fire (`FUN_004ef100`) lowers the
//! injury by GNPRTB 2563 (2) with `FastHeal` or 2564 (1), at least to 0.
//! `FastHeal` is a Force level of GNPRTB 2560 (80) or more (`FUN_004eeff0`,
//! `FUN_0053e970`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::ids::CharacterKey;
use crate::tick::TickEvent;
use crate::world::{Character, GameWorld};

/// GNPRTB 2560 (`DAT_006b9088`) = 80: the Force level that heals fast.
const GNPRTB_FAST_HEAL_FORCE: u16 = 2560;
/// GNPRTB 2561 (`DAT_006b9068`) = 1: timer `0x388`'s minimum with `FastHeal`.
const GNPRTB_FAST_HEAL_PERIOD: u16 = 2561;
/// GNPRTB 2562 (`DAT_006b9064`) = 1: its minimum without.
const GNPRTB_HEAL_PERIOD: u16 = 2562;
/// GNPRTB 2563 (`DAT_006b9084`) = 2: the injury a fire heals with `FastHeal`
/// (`FUN_004ef100`).
const GNPRTB_FAST_HEAL_AMOUNT: u16 = 2563;
/// GNPRTB 2564 (`DAT_006b905c`) = 1: the injury it heals without.
const GNPRTB_HEAL_AMOUNT: u16 = 2564;

/// Each healing character's timer `0x388`: the tick it next fires
/// (save v68).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealingState {
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub timers: HashMap<CharacterKey, u64>,
}

/// A character healed by timer `0x388`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Healed {
    pub character: CharacterKey,
    /// The injury left.
    pub injury: u16,
    pub tick: u64,
}

/// `FUN_004eef90`: the character's container (`+0x1c`, `FUN_004f6b50`) is of
/// its side, and it is neither captured (`+0xac` bit 0) nor destroyed.
///
/// port: the container is the character's fleet, else its system, whose
/// side is its holder. A character with neither, as on a mission journey,
/// stands for the en route bit, which also stops healing.
fn can_heal(world: &GameWorld, c: &Character) -> bool {
    if c.is_captive || c.is_killed {
        return false;
    }
    let side = Faction::of_alliance(c.is_alliance);
    if let Some(fleet) = c.current_fleet.and_then(|key| world.fleets.get(key)) {
        return fleet.is_alliance == c.is_alliance;
    }
    c.current_system
        .and_then(|key| world.systems.get(key))
        .and_then(crate::uprising::holder)
        == Some(side)
}

/// Stateless system for timer `0x388`.
pub struct HealingSystem;

impl HealingSystem {
    /// Arm the timers of the characters that are healing, drop the others,
    /// and fire the ones due by the last tick. Returns each healing, which
    /// the caller applies with [`apply`].
    pub fn advance(
        state: &mut HealingState,
        world: &GameWorld,
        tick_events: &[TickEvent],
    ) -> Vec<Healed> {
        let Some(last) = tick_events.last() else {
            return Vec::new();
        };
        let tick = last.tick;
        let param = |id| world.gnprtb.value(id, world.difficulty_index);
        let mut healed = Vec::new();
        for (key, c) in &world.characters {
            // FUN_004ef030: Healing = `CanHeal`, not en route, injured.
            if c.injury == 0 || !can_heal(world, c) {
                state.timers.remove(&key);
                continue;
            }
            let force = c.force_level();
            let fast = i64::from(force) >= i64::from(param(GNPRTB_FAST_HEAL_FORCE));
            let period = u64::try_from(
                param(if fast {
                    GNPRTB_FAST_HEAL_PERIOD
                } else {
                    GNPRTB_HEAL_PERIOD
                })
                .max(1),
            )
            .unwrap_or(1);
            let due = *state.timers.entry(key).or_insert(tick + period);
            if due > tick {
                continue;
            }
            state.timers.insert(key, due + period);
            let amount = param(if fast {
                GNPRTB_FAST_HEAL_AMOUNT
            } else {
                GNPRTB_HEAL_AMOUNT
            });
            let left = (i32::from(c.injury) - amount).max(0);
            healed.push(Healed {
                character: key,
                injury: u16::try_from(left).unwrap_or(0),
                tick,
            });
        }
        healed
    }
}

/// Store a healing's injury (`FUN_004ee2d0`).
pub fn apply(world: &mut GameWorld, healed: &Healed) {
    if let Some(c) = world.characters.get_mut(healed.character) {
        c.injury = healed.injury;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{DatId, SystemKey};
    use crate::world::{ControlKind, Fleet, GnprtbEntry, GnprtbParams, System};

    /// An Alliance-held system, an Empire-held one, the shipped GNPRTB
    /// 2560..2564, and an Alliance character with `injury` at the first,
    /// Force level `force`.
    fn setup(injury: u16, force: u32) -> (GameWorld, CharacterKey, SystemKey, SystemKey) {
        let mut world = GameWorld {
            gnprtb: GnprtbParams::new(
                [(2560, 80), (2561, 1), (2562, 1), (2563, 2), (2564, 1)]
                    .into_iter()
                    .map(|(parameter_id, value)| GnprtbEntry {
                        parameter_id,
                        development: value,
                        alliance_sp_easy: value,
                        alliance_sp_medium: value,
                        alliance_sp_hard: value,
                        empire_sp_easy: value,
                        empire_sp_medium: value,
                        empire_sp_hard: value,
                        multiplayer: value,
                    })
                    .collect(),
            ),
            ..GameWorld::default()
        };
        let mut system = |holder| {
            world.systems.insert(System {
                last_arrival: None,
                dat_id: DatId::new(0x9000_0001),
                name: "S".into(),
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
                fighters: vec![],
                is_headquarters: false,
                is_destroyed: false,
                control: holder,
            })
        };
        let own = system(ControlKind::Controlled(Faction::Alliance));
        let enemy = system(ControlKind::Controlled(Faction::Empire));
        let character = world.characters.insert(Character {
            is_alliance: true,
            force_base: u16::try_from(force).unwrap(),
            injury,
            current_system: Some(own),
            ..Character::default()
        });
        (world, character, own, enemy)
    }

    fn days(state: &mut HealingState, world: &mut GameWorld, range: std::ops::RangeInclusive<u64>) {
        for tick in range {
            for heal in HealingSystem::advance(state, world, &[TickEvent { tick }]) {
                apply(world, &heal);
            }
        }
    }

    #[test]
    fn an_injured_character_at_home_heals_a_point_a_day() {
        // FUN_004f1b40 arms 0x388 with Healing; its record's minimum is
        // GNPRTB 2562 (1); FUN_004ef100 lowers +0x94 by 2564 (1).
        let (mut world, character, ..) = setup(3, 0);
        let mut state = HealingState::default();
        days(&mut state, &mut world, 1..=1);
        assert_eq!(world.characters[character].injury, 3);
        assert_eq!(state.timers[&character], 2);
        days(&mut state, &mut world, 2..=3);
        assert_eq!(world.characters[character].injury, 1);
        days(&mut state, &mut world, 4..=4);
        assert_eq!(world.characters[character].injury, 0);
        // Healed: the flag falls and the timer goes.
        days(&mut state, &mut world, 5..=5);
        assert!(!state.timers.contains_key(&character));
    }

    #[test]
    fn a_force_level_of_eighty_heals_two_points_a_day() {
        // FUN_004eeff0: FastHeal at +0x8c >= GNPRTB 2560 (80); FUN_004ef100
        // then heals GNPRTB 2563 (2), never below 0.
        let (mut world, character, ..) = setup(3, 80);
        let mut state = HealingState::default();
        days(&mut state, &mut world, 1..=2);
        assert_eq!(world.characters[character].injury, 1);
        days(&mut state, &mut world, 3..=3);
        assert_eq!(world.characters[character].injury, 0);
        let (mut world, character, ..) = setup(3, 79);
        let mut state = HealingState::default();
        days(&mut state, &mut world, 1..=2);
        assert_eq!(world.characters[character].injury, 2);
        // The class Force template is not the Force level (+0x8c,
        // FUN_004eee40): a template of 90 with no Force base heals slowly.
        let (mut world, character, ..) = setup(3, 0);
        world.characters[character].jedi_level.base = 90;
        let mut state = HealingState::default();
        days(&mut state, &mut world, 1..=2);
        assert_eq!(world.characters[character].injury, 2);
    }

    #[test]
    fn the_timer_waits_its_record_s_minimum_between_healings() {
        // FUN_004ef750 sets record +0x54+0x18's minimum to GNPRTB 2562; with
        // 3 the healings fall on days 4 and 7 (FUN_005862a0 repeats).
        let (mut world, character, ..) = setup(5, 0);
        world.gnprtb = GnprtbParams::new(
            [(2560, 80), (2561, 1), (2562, 3), (2563, 2), (2564, 1)]
                .into_iter()
                .map(|(parameter_id, value)| GnprtbEntry {
                    parameter_id,
                    development: value,
                    alliance_sp_easy: value,
                    alliance_sp_medium: value,
                    alliance_sp_hard: value,
                    empire_sp_easy: value,
                    empire_sp_medium: value,
                    empire_sp_hard: value,
                    multiplayer: value,
                })
                .collect(),
        );
        let mut state = HealingState::default();
        days(&mut state, &mut world, 1..=3);
        assert_eq!(world.characters[character].injury, 5);
        days(&mut state, &mut world, 4..=4);
        assert_eq!(world.characters[character].injury, 4);
        assert_eq!(state.timers[&character], 7);
        days(&mut state, &mut world, 5..=7);
        assert_eq!(world.characters[character].injury, 3);
    }

    #[test]
    fn only_a_free_character_among_its_own_side_heals() {
        // FUN_004eef90: CanHeal needs the container of the character's side,
        // and neither capture nor destruction.
        let still = |arrange: &dyn Fn(&mut GameWorld, CharacterKey, SystemKey)| {
            let (mut world, character, _, enemy) = setup(3, 0);
            arrange(&mut world, character, enemy);
            let mut state = HealingState::default();
            days(&mut state, &mut world, 1..=3);
            world.characters[character].injury
        };
        assert_eq!(
            still(&|w, c, enemy| w.characters[c].current_system = Some(enemy)),
            3
        );
        assert_eq!(still(&|w, c, _| w.characters[c].current_system = None), 3);
        assert_eq!(still(&|w, c, _| w.characters[c].is_captive = true), 3);
        // A fleet of the character's side heals it even at an enemy system;
        // the other side's fleet does not.
        let fleet = |alliance: bool| {
            move |w: &mut GameWorld, c: CharacterKey, enemy: SystemKey| {
                let key = w.fleets.insert(Fleet {
                    location: enemy,
                    capital_ships: vec![],
                    fighters: vec![],
                    characters: vec![c],
                    is_alliance: alliance,
                    has_death_star: false,
                    special_forces: Vec::new(),
                });
                w.characters[c].current_fleet = Some(key);
                w.characters[c].current_system = Some(enemy);
            }
        };
        assert_eq!(still(&fleet(true)), 1);
        assert_eq!(still(&fleet(false)), 3);
    }
}
