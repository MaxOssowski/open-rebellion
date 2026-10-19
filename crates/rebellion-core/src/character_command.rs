//! A character's command, `CommandKind` (`+0x96`;
//! `ghidra/notes/character-command.md`).
//!
//! The Command orders `0x260..0x263` (`FUN_0051f4b0`, `FUN_0053ebb0`) give a
//! character no command, or the command of a Commander, an Admiral or a
//! General. [`order_allowed`] is the order's check (`FUN_004ed750`),
//! [`give`] its effect (`FUN_00561bb0`, `FUN_004ef400`), and
//! [`release_unfit`] the loss `FUN_004eeed0` makes through the character's
//! hooks.

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::ids::{CharacterKey, FleetKey, SystemKey};
use crate::world::{Character, GameWorld, Skill};

/// `CommandKind`, character short `+0x96` (`FUN_004ee350`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CommandKind {
    /// 0: no command.
    #[default]
    None = 0,
    /// 1: Commander (order `0x261`; name prefix TEXTSTRA `+0x6800`).
    Commander = 1,
    /// 2: Admiral (order `0x262`; `+0x6c00`).
    Admiral = 2,
    /// 3: General (order `0x263`; `+0x7000`).
    General = 3,
}

impl CommandKind {
    /// The order that gives this command (`FUN_0053ebb0`'s inverse).
    #[must_use]
    pub const fn order(self) -> u16 {
        match self {
            Self::None => 0x260,
            Self::Commander => 0x261,
            Self::Admiral => 0x262,
            Self::General => 0x263,
        }
    }

    /// `FUN_0053ebb0`: the command an order `0x260..0x263` names.
    #[must_use]
    pub const fn of_order(order: u16) -> Option<Self> {
        match order {
            0x260 => Some(Self::None),
            0x261 => Some(Self::Commander),
            0x262 => Some(Self::Admiral),
            0x263 => Some(Self::General),
            _ => None,
        }
    }
}

/// Why a Command order is refused: the base check `FUN_004f9860`, then
/// `FUN_004ed750`'s error codes under `0x30`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// `FUN_004f9860` 1: the character is not the ordering side's (`+0x24`
    /// bits 6..7).
    OtherSide,
    /// `FUN_004f9860` 2: not active (`+0x50` bit 2; for a character, not
    /// recruited).
    Inactive,
    /// `FUN_004f9860` 3: destroyed (`+0x50` bit 3; killed).
    Destroyed,
    /// 1: on a mission (`+0x78` bit 7).
    OnMission,
    /// 2: captured (`+0xac` bit 0).
    Captured,
    /// 3: injured (`+0x94` non-zero).
    Injured,
    /// 4: the class does not allow the command, or None without one.
    NotAllowed,
}

/// `FUN_004ed750`, the character's slot `+0x2d0` for orders `0x260..0x263`,
/// for the side whose order it is (`is_alliance`):
///
/// - the base check `FUN_004f9860`: the character is the ordering side's,
///   active (recruited) and not destroyed;
/// - the order is refused for a character on a mission, injured or
///   captured;
/// - and unless its class allows the command (`FUN_004ed1e0` Commander,
///   `FUN_004ed1c0` Admiral, `FUN_004ed200` General, the class record's
///   `+0xac`, `+0xa8`, `+0xb0`); None needs a command to clear.
///
/// The original reports every refusal that applies, in this order; the
/// port returns the first.
///
/// port: the base check's en route test (`+0x50` bit 4) is not made; the
/// port keeps no such flag on a character.
pub fn order_allowed(
    character: &Character,
    kind: CommandKind,
    is_alliance: bool,
) -> Result<(), Refusal> {
    if character.is_alliance != is_alliance {
        return Err(Refusal::OtherSide);
    }
    if !character.recruited {
        return Err(Refusal::Inactive);
    }
    if character.is_killed {
        return Err(Refusal::Destroyed);
    }
    if character.on_mission || character.on_hidden_mission {
        return Err(Refusal::OnMission);
    }
    if character.injury != 0 {
        return Err(Refusal::Injured);
    }
    if character.is_captive {
        return Err(Refusal::Captured);
    }
    let allowed = match kind {
        CommandKind::None => character.command != CommandKind::None,
        CommandKind::Commander => character.can_be_commander,
        CommandKind::Admiral => character.can_be_admiral,
        CommandKind::General => character.can_be_general,
    };
    if allowed {
        Ok(())
    } else {
        Err(Refusal::NotAllowed)
    }
}

/// `FUN_00561bb0`, the Command order of the side `is_alliance` on
/// `character`:
///
/// 1. The command the order names (`FUN_0053ebb0`); the character's own
///    command again names none.
/// 2. With a command, its holder among the side's characters where the
///    character stands (`FUN_0055ed30`) loses it: aboard a fleet, the
///    characters of that fleet (`FUN_004fd790` over its ships); on a
///    planet, those of the system not aboard a fleet (`FUN_00509330`).
/// 3. Slot `+0x2f4` (`FUN_004ef340`) re-checks the order ([`order_allowed`])
///    and runs `FUN_004ef400`: the character's own command again clears it,
///    any other is set (`FUN_004ee350`).
///
/// Returns the characters whose command changed, the previous holder first.
/// Nothing changes when the check fails.
pub fn give(
    world: &mut GameWorld,
    character: CharacterKey,
    kind: CommandKind,
    is_alliance: bool,
) -> Result<Vec<CharacterKey>, Refusal> {
    let Some(c) = world.characters.get(character) else {
        return Err(Refusal::Destroyed);
    };
    order_allowed(c, kind, is_alliance)?;
    let kind = if c.command == kind {
        CommandKind::None
    } else {
        kind
    };
    let (fleet, system, side) = (c.current_fleet, c.current_system, c.is_alliance);
    let mut changed = Vec::new();
    if kind != CommandKind::None {
        let holder = world.characters.iter().find_map(|(key, other)| {
            let there = match fleet {
                Some(f) => other.current_fleet == Some(f),
                None => {
                    system.is_some()
                        && other.current_system == system
                        && other.current_fleet.is_none()
                }
            };
            (key != character
                && there
                && other.is_alliance == side
                && !other.is_killed
                && other.command == kind)
                .then_some(key)
        });
        if let Some(holder) = holder {
            world.characters[holder].set_command(CommandKind::None);
            changed.push(holder);
        }
    }
    let c = &mut world.characters[character];
    if c.command != kind {
        c.set_command(kind);
        changed.push(character);
    }
    Ok(changed)
}

/// `FUN_004eeed0`: a character keeps its command only while uninjured and
/// not captured. The character's injury (slot `+0x32c`, `FUN_004f1720`) and
/// capture (slot `+0x34c`, `FUN_004f1a80`) hooks run it.
///
/// hyp: its other two tests, `+0x50` bit 6 and a commanding key `+0x68` of
/// family `0x50..0x7f`, hold while a character commands, since the order
/// sets the key (`SetCharacterCommandingKey`); neither is read yet.
#[must_use]
pub fn keeps(character: &Character) -> bool {
    character.injury == 0 && !character.is_captive && !character.is_killed
}

/// The characters whose command `FUN_004eeed0` clears: each one holding a
/// command it no longer [`keeps`]. Returns them.
///
/// port: the original clears it in the hook as the injury or capture
/// happens; the port's injuries clear it at once ([`Character::injure`]) and
/// every capture by the day's pass that calls this.
pub fn release_unfit(world: &mut GameWorld) -> Vec<CharacterKey> {
    let mut released = Vec::new();
    for (key, c) in &mut world.characters {
        if c.command != CommandKind::None && !keeps(c) {
            c.set_command(CommandKind::None);
            released.push(key);
        }
    }
    released
}

/// `FUN_004fd790(fleet, kind)`: the first character of the fleet's own side
/// (`+0x24 >> 6 & 3`) aboard it holding `kind`.
#[must_use]
pub fn fleet_holder(world: &GameWorld, fleet: FleetKey, kind: CommandKind) -> Option<CharacterKey> {
    let side = world.fleets.get(fleet)?.is_alliance;
    world.characters.iter().find_map(|(key, c)| {
        (c.current_fleet == Some(fleet) && c.is_alliance == side && !c.is_killed && c.command == kind)
            .then_some(key)
    })
}

/// `FUN_00509330(system, kind)`: the first character of the system's own
/// side (`+0x24 >> 6 & 3`) held by the system, not aboard a fleet, holding
/// `kind`. A neutral or contested system has none.
#[must_use]
pub fn system_holder(
    world: &GameWorld,
    system: SystemKey,
    kind: CommandKind,
) -> Option<CharacterKey> {
    let side = match GameWorld::facility_holder(world.systems.get(system)?.control)? {
        Faction::Alliance => true,
        Faction::Empire => false,
        Faction::Neutral => return None,
    };
    world.characters.iter().find_map(|(key, c)| {
        (c.current_system == Some(system)
            && c.current_fleet.is_none()
            && c.is_alliance == side
            && !c.is_killed
            && c.command == kind)
            .then_some(key)
    })
}

/// The holder's slot `+0x1f4`, its leadership (`+0x88`,
/// [`Character::effective_skill`]); 0 without a holder, as the callers
/// (`FUN_0058dc10`, `FUN_0058df60`, `FUN_0058bec0`) start it.
#[must_use]
pub fn leadership(world: &GameWorld, holder: Option<CharacterKey>) -> i32 {
    holder
        .and_then(|key| world.characters.get(key))
        .map_or(0, |c| {
            i32::try_from(c.effective_skill(Skill::Leadership)).unwrap_or(i32::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Command order of `c`'s own side.
    fn give_as(
        world: &mut GameWorld,
        c: CharacterKey,
        kind: CommandKind,
    ) -> Result<Vec<CharacterKey>, Refusal> {
        let side = world.characters[c].is_alliance;
        give(world, c, kind, side)
    }

    fn officer(world: &mut GameWorld, alliance: bool) -> CharacterKey {
        world.characters.insert(Character {
            is_alliance: alliance,
            is_empire: !alliance,
            recruited: true,
            can_be_admiral: true,
            can_be_commander: true,
            can_be_general: true,
            ..Default::default()
        })
    }

    #[test]
    fn the_command_orders_name_none_commander_admiral_and_general() {
        // FUN_0053ebb0: 0x260 -> 0, 0x261 -> 1, 0x262 -> 2, 0x263 -> 3.
        let kinds = [0x260, 0x261, 0x262, 0x263, 0x264].map(CommandKind::of_order);
        assert_eq!(
            kinds,
            [
                Some(CommandKind::None),
                Some(CommandKind::Commander),
                Some(CommandKind::Admiral),
                Some(CommandKind::General),
                None
            ]
        );
        for kind in kinds.into_iter().flatten() {
            assert_eq!(CommandKind::of_order(kind.order()), Some(kind));
        }
    }

    #[test]
    fn a_command_order_needs_a_free_uninjured_character_its_class_allows() {
        // FUN_004ed750: refused on a mission (+0x78 bit 7), injured (+0x94),
        // captured (+0xac bit 0), or when the class lacks the kind
        // (FUN_004ed1e0, FUN_004ed1c0, FUN_004ed200); None needs a command.
        let base = Character {
            is_alliance: true,
            recruited: true,
            can_be_admiral: true,
            ..Default::default()
        };
        assert_eq!(order_allowed(&base, CommandKind::Admiral, true), Ok(()));
        // FUN_004f9860: the ordering side's, active (recruited), not destroyed.
        assert_eq!(
            order_allowed(&base, CommandKind::Admiral, false),
            Err(Refusal::OtherSide)
        );
        let with = |edit: fn(&mut Character)| {
            let mut c = base.clone();
            edit(&mut c);
            order_allowed(&c, CommandKind::Admiral, true)
        };
        assert_eq!(with(|c| c.recruited = false), Err(Refusal::Inactive));
        assert_eq!(with(|c| c.is_killed = true), Err(Refusal::Destroyed));
        assert_eq!(with(|c| c.on_mission = true), Err(Refusal::OnMission));
        assert_eq!(
            with(|c| c.on_hidden_mission = true),
            Err(Refusal::OnMission)
        );
        assert_eq!(with(|c| c.injury = 1), Err(Refusal::Injured));
        assert_eq!(with(|c| c.is_captive = true), Err(Refusal::Captured));
        assert_eq!(with(|c| c.can_be_admiral = false), Err(Refusal::NotAllowed));
        assert_eq!(
            order_allowed(&base, CommandKind::General, true),
            Err(Refusal::NotAllowed)
        );
        assert_eq!(
            order_allowed(&base, CommandKind::Commander, true),
            Err(Refusal::NotAllowed)
        );
        let general = Character {
            is_alliance: true,
            recruited: true,
            can_be_general: true,
            can_be_commander: true,
            ..Default::default()
        };
        assert_eq!(order_allowed(&general, CommandKind::General, true), Ok(()));
        assert_eq!(
            order_allowed(&general, CommandKind::Commander, true),
            Ok(())
        );
        assert_eq!(
            order_allowed(&base, CommandKind::None, true),
            Err(Refusal::NotAllowed)
        );
        let commanding = Character {
            command: CommandKind::Admiral,
            ..base
        };
        assert_eq!(order_allowed(&commanding, CommandKind::None, true), Ok(()));
    }

    #[test]
    fn a_fleet_or_a_planet_has_one_holder_of_each_command() {
        // FUN_00561bb0: in a fleet (FUN_004fd790) or on a planet
        // (FUN_00509330) the side's holder of the kind loses it; the same
        // kind again clears it (FUN_004ef400).
        let mut keys = slotmap::SlotMap::<FleetKey, ()>::with_key();
        let (fleet, other_fleet) = (keys.insert(()), keys.insert(()));
        let mut systems = slotmap::SlotMap::<SystemKey, ()>::with_key();
        let system = systems.insert(());
        let mut world = GameWorld::default();
        let place = |world: &mut GameWorld, alliance, fleet: Option<FleetKey>| {
            let c = officer(world, alliance);
            world.characters[c].current_system = Some(system);
            world.characters[c].current_fleet = fleet;
            c
        };
        let a = place(&mut world, true, Some(fleet));
        let b = place(&mut world, true, Some(fleet));
        let elsewhere = place(&mut world, true, Some(other_fleet));
        let enemy = place(&mut world, false, Some(fleet));
        let planet = place(&mut world, true, None);
        for c in [elsewhere, enemy, planet] {
            assert_eq!(give_as(&mut world, c, CommandKind::Admiral), Ok(vec![c]));
        }
        assert_eq!(give_as(&mut world, a, CommandKind::Admiral), Ok(vec![a]));
        assert_eq!(give_as(&mut world, b, CommandKind::Admiral), Ok(vec![a, b]));
        assert_eq!(world.characters[a].command, CommandKind::None);
        assert_eq!(world.characters[b].command, CommandKind::Admiral);
        // The other fleet's, the enemy's and the planet's admirals stay.
        for c in [elsewhere, enemy, planet] {
            assert_eq!(world.characters[c].command, CommandKind::Admiral);
        }
        // Another kind takes no one's.
        assert_eq!(give_as(&mut world, a, CommandKind::General), Ok(vec![a]));
        assert_eq!(world.characters[b].command, CommandKind::Admiral);
        // On the planet: the system's holder not aboard a fleet.
        let second = place(&mut world, true, None);
        assert_eq!(
            give_as(&mut world, second, CommandKind::Admiral),
            Ok(vec![planet, second])
        );
        assert_eq!(world.characters[b].command, CommandKind::Admiral);
        // The same kind again clears it, as None does.
        assert_eq!(give_as(&mut world, b, CommandKind::Admiral), Ok(vec![b]));
        assert_eq!(world.characters[b].command, CommandKind::None);
        assert_eq!(give_as(&mut world, a, CommandKind::None), Ok(vec![a]));
        assert_eq!(world.characters[a].command, CommandKind::None);
        // A refused order changes nothing.
        world.characters[a].injury = 3;
        assert_eq!(
            give_as(&mut world, a, CommandKind::Admiral),
            Err(Refusal::Injured)
        );
        assert_eq!(world.characters[a].command, CommandKind::None);
    }

    #[test]
    fn a_fleet_s_or_a_planet_s_officer_is_its_own_side_s_holder_of_the_rank() {
        // FUN_004fd790: the fleet's side (+0x24 >> 6 & 3) aboard it;
        // FUN_00509330: the system's side held by the system, not aboard a
        // fleet. Slot +0x1f4 is the holder's leadership (+0x88).
        use crate::dat::ExplorationStatus;
        use crate::world::{ControlKind, Fleet, SkillPair, System};
        let mut world = GameWorld::default();
        let system = world.systems.insert(System {
            dat_id: crate::ids::DatId::new(0x9000_0000),
            name: "Yavin".into(),
            sector: crate::ids::SectorKey::default(),
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
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });
        let fleet = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        let place = |world: &mut GameWorld, alliance, fleet, kind, leadership| {
            let c = officer(world, alliance);
            let c_mut = &mut world.characters[c];
            c_mut.current_system = Some(system);
            c_mut.current_fleet = fleet;
            c_mut.command = kind;
            c_mut.leadership = SkillPair {
                base: leadership,
                variance: 0,
            };
            c
        };
        assert_eq!(fleet_holder(&world, fleet, CommandKind::Admiral), None);
        assert_eq!(leadership(&world, None), 0);
        place(&mut world, false, Some(fleet), CommandKind::Admiral, 90);
        place(&mut world, true, Some(fleet), CommandKind::General, 80);
        let admiral = place(&mut world, true, Some(fleet), CommandKind::Admiral, 70);
        let general = place(&mut world, true, None, CommandKind::General, 60);
        place(&mut world, false, None, CommandKind::General, 50);
        assert_eq!(
            fleet_holder(&world, fleet, CommandKind::Admiral),
            Some(admiral)
        );
        assert_eq!(
            system_holder(&world, system, CommandKind::General),
            Some(general)
        );
        assert_eq!(system_holder(&world, system, CommandKind::Admiral), None);
        assert_eq!(
            leadership(&world, fleet_holder(&world, fleet, CommandKind::Admiral)),
            70
        );
        // A killed holder holds nothing; a neutral system has no officer.
        world.characters[admiral].is_killed = true;
        assert_eq!(fleet_holder(&world, fleet, CommandKind::Admiral), None);
        world.systems[system].control = ControlKind::Uncontrolled;
        assert_eq!(system_holder(&world, system, CommandKind::General), None);
    }

    #[test]
    fn a_character_that_moves_loses_its_command() {
        // FUN_00560d50 from the move event FUN_00514a60: a character
        // (0x30..0x3b) holding a command that moves loses it
        // (FUN_004ee350(0)); one that stays keeps it.
        let mut fleets = slotmap::SlotMap::<FleetKey, ()>::with_key();
        let fleet = fleets.insert(());
        let mut systems = slotmap::SlotMap::<SystemKey, ()>::with_key();
        let (here, there) = (systems.insert(()), systems.insert(()));
        let mut world = GameWorld::default();
        let c = officer(&mut world, true);
        let held = |world: &mut GameWorld, fleet, system| {
            let value = &mut world.characters[c];
            value.current_fleet = fleet;
            value.current_system = system;
            value.command = CommandKind::General;
        };
        let moved = |world: &mut GameWorld, fleet, system| {
            world.characters[c].move_to(fleet, system);
            world.characters[c].command
        };
        held(&mut world, None, Some(here));
        assert_eq!(moved(&mut world, None, Some(here)), CommandKind::General);
        // To another planet, aboard a fleet, or off the map on a mission.
        for (fleet, system) in [(None, Some(there)), (Some(fleet), Some(here)), (None, None)] {
            held(&mut world, None, Some(here));
            assert_eq!(moved(&mut world, fleet, system), CommandKind::None);
            assert_eq!(world.characters[c].current_fleet, fleet);
            assert_eq!(world.characters[c].current_system, system);
        }
        // Off the fleet onto its planet.
        held(&mut world, Some(fleet), Some(here));
        assert_eq!(moved(&mut world, None, Some(here)), CommandKind::None);
    }

    #[test]
    fn a_command_renames_the_character_and_its_loss_renames_it_back() {
        // FUN_004ee350's hook, slot +0x330 (FUN_004f17f0): the name becomes
        // TEXTSTRA of the class name +0x6800, +0x6c00 or +0x7000, and with
        // no command the name itself (FUN_004f6e60).
        let mut world = GameWorld::default();
        let titled = |name: &str| -> [String; 4] {
            [
                name.to_owned(),
                format!("Commander {name}"),
                format!("Admiral {name}"),
                format!("General {name}"),
            ]
        };
        let ackbar = officer(&mut world, true);
        let rieekan = officer(&mut world, true);
        let mut fleets = slotmap::SlotMap::<FleetKey, ()>::with_key();
        let fleet = fleets.insert(());
        for (c, name) in [(ackbar, "Ackbar"), (rieekan, "Rieekan")] {
            world.characters[c].name = name.into();
            world.characters[c].command_names = titled(name);
            world.characters[c].current_fleet = Some(fleet);
        }
        give_as(&mut world, ackbar, CommandKind::Admiral).unwrap();
        assert_eq!(world.characters[ackbar].name, "Admiral Ackbar");
        give_as(&mut world, ackbar, CommandKind::General).unwrap();
        assert_eq!(world.characters[ackbar].name, "General Ackbar");
        // Another takes the kind: the holder's name goes back.
        give_as(&mut world, rieekan, CommandKind::General).unwrap();
        assert_eq!(world.characters[ackbar].name, "Ackbar");
        assert_eq!(world.characters[rieekan].name, "General Rieekan");
        // An injury takes the command and the name.
        world.characters[rieekan].injure(4);
        assert_eq!(world.characters[rieekan].name, "Rieekan");
        // A capture too.
        world.characters[ackbar].injury = 0;
        give_as(&mut world, ackbar, CommandKind::Commander).unwrap();
        assert_eq!(world.characters[ackbar].name, "Commander Ackbar");
        world.characters[ackbar].is_captive = true;
        release_unfit(&mut world);
        assert_eq!(world.characters[ackbar].name, "Ackbar");
        // An empty entry keeps the name.
        world.characters[ackbar].is_captive = false;
        world.characters[ackbar].command_names[2] = String::new();
        give_as(&mut world, ackbar, CommandKind::Admiral).unwrap();
        assert_eq!(world.characters[ackbar].name, "Ackbar");
    }

    #[test]
    fn an_injury_or_a_capture_takes_the_command() {
        // FUN_004eeed0 from the injury hook (slot +0x32c, FUN_004f1720) and
        // the capture hook (slot +0x34c, FUN_004f1a80).
        let mut world = GameWorld::default();
        let [a, b, c] = [0; 3].map(|_| officer(&mut world, true));
        for k in [a, b, c] {
            world.characters[k].command = CommandKind::General;
        }
        world.characters[a].injure(0);
        assert_eq!(world.characters[a].command, CommandKind::General);
        world.characters[a].injure(5);
        assert_eq!(world.characters[a].command, CommandKind::None);
        world.characters[b].is_captive = true;
        assert_eq!(release_unfit(&mut world), vec![b]);
        assert_eq!(world.characters[b].command, CommandKind::None);
        assert_eq!(world.characters[c].command, CommandKind::General);
        assert!(release_unfit(&mut world).is_empty());
        assert!(keeps(&world.characters[c]));
        world.characters[c].mark_killed();
        assert!(!keeps(&world.characters[c]));
    }
}
