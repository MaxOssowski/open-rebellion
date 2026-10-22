//! The Retire order (`0x242`) on a character (`ghidra/notes/character-retire.md`).
//!
//! The order (`FUN_004f45b0`, vtable `0x0065d068`) builds event `0x230`
//! (`FUN_00540c40`, vtable `0x00661af0`). The event checks the character
//! through its slots `+0x1c0`/`+0x1c4` and, when the player confirms,
//! fires slot `+0x200`, which destroys it with reason `0x14`
//! (`FUN_00534c00` -> slot `+0xac`). The Personnel Finder then lists it as
//! "Retired" (`FUN_00465bb0`, TEXTSTRA `0x8799`).

use crate::ids::CharacterKey;
use crate::world::{Character, GameWorld};

/// Why a Retire order is refused: the base check `FUN_004f9860`, then the
/// codes under `0x30` of `FUN_00533ba0` and `FUN_004ed460`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// `FUN_004f9860` 1: not the ordering side's (`+0x24` bits 6..7).
    OtherSide,
    /// `FUN_004f9860` 2: not active (`+0x50` bit 2; for a character, not
    /// recruited).
    Inactive,
    /// `FUN_004f9860` 3: destroyed (`+0x50` bit 3).
    Destroyed,
    /// `FUN_004f9860` 4: en route (`+0x50` bit 4).
    EnRoute,
    /// `FUN_00533ba0` 1: on a mission (`+0x78` bit 7).
    OnMission,
    /// `FUN_004ed460` 1: it holds a command (`+0x96`).
    Commanding,
    /// `FUN_004ed460` 2: captured (`+0xac` bit 0).
    Captured,
}

/// `FUN_004ed460`, the character's slot `+0x1c0`, for the side whose order
/// it is (`is_alliance`): the base check (`FUN_004f9860`: the side's,
/// recruited, not destroyed, not `en_route`), then a character on a
/// mission, holding a command or captured is refused. An injured one may
/// retire.
///
/// The original reports every refusal that applies, in this order; the
/// port returns the first. port: `en_route` is the port's travel
/// (`MissionState::is_en_route`), which the caller knows.
pub fn order_allowed(
    character: &Character,
    is_alliance: bool,
    en_route: bool,
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
    if en_route {
        return Err(Refusal::EnRoute);
    }
    if character.on_mission || character.on_hidden_mission {
        return Err(Refusal::OnMission);
    }
    if character.command != crate::character_command::CommandKind::None {
        return Err(Refusal::Commanding);
    }
    if character.is_captive {
        return Err(Refusal::Captured);
    }
    Ok(())
}

/// Event `0x230`'s fire (`FUN_00540f40` -> slot `+0x200`, `FUN_00534b70`):
/// the check again, then the character is destroyed with reason `0x14`
/// (`FUN_00534c00`). Nothing changes when the check fails.
pub fn retire(
    world: &mut GameWorld,
    character: CharacterKey,
    is_alliance: bool,
    en_route: bool,
) -> Result<(), Refusal> {
    let Some(c) = world.characters.get_mut(character) else {
        return Err(Refusal::Destroyed);
    };
    order_allowed(c, is_alliance, en_route)?;
    c.mark_killed();
    c.is_retired = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character_command::CommandKind;

    fn rebel() -> Character {
        Character {
            is_alliance: true,
            recruited: true,
            ..Character::default()
        }
    }

    #[test]
    fn retire_refuses_as_the_original_checks() {
        // FUN_004f9860, then FUN_00533ba0 (on a mission) and FUN_004ed460
        // (commanding, captured); an injured character may retire.
        let with = |change: fn(&mut Character), en_route| {
            let mut c = rebel();
            change(&mut c);
            order_allowed(&c, true, en_route)
        };
        assert_eq!(with(|_| {}, false), Ok(()));
        assert_eq!(with(|c| c.injury = 4, false), Ok(()));
        assert_eq!(
            with(|c| c.is_alliance = false, false),
            Err(Refusal::OtherSide)
        );
        assert_eq!(with(|c| c.recruited = false, false), Err(Refusal::Inactive));
        assert_eq!(with(|c| c.is_killed = true, false), Err(Refusal::Destroyed));
        assert_eq!(with(|_| {}, true), Err(Refusal::EnRoute));
        assert_eq!(
            with(|c| c.on_mission = true, false),
            Err(Refusal::OnMission)
        );
        assert_eq!(
            with(|c| c.on_hidden_mission = true, false),
            Err(Refusal::OnMission)
        );
        assert_eq!(
            with(|c| c.command = CommandKind::Admiral, false),
            Err(Refusal::Commanding)
        );
        assert_eq!(with(|c| c.is_captive = true, false), Err(Refusal::Captured));
    }

    #[test]
    fn a_retired_character_leaves_the_game_as_retired() {
        // FUN_00534c00: destroyed with reason 0x14, which the Personnel
        // Finder reads as "Retired".
        let mut world = GameWorld::default();
        let key = world.characters.insert(rebel());
        world.characters[key].current_system = Some(crate::ids::SystemKey::default());
        assert_eq!(
            retire(&mut world, key, false, false),
            Err(Refusal::OtherSide)
        );
        assert!(!world.characters[key].is_killed);
        assert_eq!(retire(&mut world, key, true, false), Ok(()));
        let c = &world.characters[key];
        assert!(c.is_killed && c.is_retired);
        assert_eq!(c.current_system, None);
        assert_eq!(
            retire(&mut world, key, true, false),
            Err(Refusal::Destroyed)
        );
    }
}
