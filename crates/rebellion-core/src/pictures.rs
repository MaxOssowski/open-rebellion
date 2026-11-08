//! Original picture resources that both the windows and the tactical battle
//! state name: a system's planet picture and a character's compact image.

use crate::ids::DatId;

/// `SYSTEMSD.DAT` records 100 through 299 are contiguous. This table preserves
/// their original `picture_id` values without adding presentation-only state
/// to the serialized simulation world.
pub const SYSTEM_PLANET_PICTURES: [u8; 200] = [
    1, 2, 7, 8, 3, 4, 9, 10, 11, 12, 5, 13, 11, 14, 15, 8, 7, 11, 6, 7, 8, 7, 1, 13, 2, 16, 17, 3,
    4, 18, 9, 13, 15, 7, 14, 18, 19, 20, 8, 13, 5, 6, 1, 2, 3, 7, 4, 5, 6, 1, 2, 3, 11, 4, 11, 5,
    6, 1, 2, 3, 9, 4, 5, 11, 19, 6, 1, 13, 19, 11, 7, 2, 13, 3, 13, 8, 4, 5, 6, 1, 1, 8, 2, 3, 4,
    5, 11, 19, 11, 6, 13, 1, 2, 20, 21, 8, 7, 13, 11, 8, 11, 13, 3, 13, 11, 15, 7, 4, 5, 19, 10,
    20, 20, 11, 19, 6, 9, 1, 20, 2, 19, 14, 22, 9, 11, 13, 3, 11, 8, 18, 4, 20, 15, 8, 13, 8, 8,
    19, 13, 8, 5, 19, 13, 6, 1, 13, 20, 8, 19, 9, 11, 13, 2, 3, 13, 18, 4, 5, 7, 11, 19, 11, 6, 13,
    1, 23, 2, 3, 13, 4, 5, 24, 7, 6, 1, 8, 13, 2, 14, 25, 19, 3, 26, 13, 20, 4, 11, 19, 5, 20, 9,
    20, 9, 9, 19, 6, 8, 1, 19, 8,
];

/// The SYSTEMSD.DAT `picture_id` of a system, from the table above.
#[must_use]
pub fn planet_picture_id(dat_id: DatId) -> u8 {
    dat_id
        .index()
        .checked_sub(100)
        .and_then(|index| SYSTEM_PLANET_PICTURES.get(index as usize))
        .copied()
        .unwrap_or(1)
}

/// A character's picture (its vtable `+0x30`) as its compact image: Luke
/// with `DagobahCompleted` returns `0x850` (`FUN_0056f550`), the Jedi
/// Knight's 18512 (story portrait STRATEGY 6224, as A0
/// future-jedi-alliance/01 shows him after Dagobah); any other character
/// its class's ([`character_mini_resource_id`]).
#[must_use]
pub fn character_picture(character: &crate::world::Character) -> Option<u32> {
    if character.is_major && character.dat_id.index() == LUKE && character.dagobah_completed {
        return Some(LUKE_JEDI_KNIGHT);
    }
    character_mini_resource_id(character.dat_id, character.is_major)
}

/// Luke's record (`0x32000242`), whose vtable `+0x30` is `FUN_0056f550`.
const LUKE: u32 = 0x242;
/// Luke's picture after Dagobah, `0x850` from `FUN_0056f550`, as a compact
/// image (`0x4850`, GOKRES 18512).
const LUKE_JEDI_KNIGHT: u32 = 0x4850;

/// Compact 61x25 GOKRES image of a character, used by the system and the
/// tactical contents windows.
///
/// hyp: GOKRES bands matched by image (`dd1cc803`); the executable's writer is not located.
#[must_use]
pub fn character_mini_resource_id(dat_id: DatId, is_major: bool) -> Option<u32> {
    let index = dat_id.index();
    if is_major {
        matches!(index, 576..=579 | 592 | 640..=641).then_some(index + 17_920)
    } else {
        matches!(index, 832..=857 | 896..=923).then_some(index + 18_176)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Character;

    #[test]
    fn luke_after_dagobah_takes_the_jedi_knights_picture() {
        // FUN_0056f550 (Luke's vtable +0x30): 0x850 with DagobahCompleted,
        // else the class's picture; A0 future-jedi-alliance/01 (day 607,
        // after Dagobah) shows STRATEGY 6224.
        let mut luke = Character {
            dat_id: DatId::new(0x3200_0242),
            is_major: true,
            ..Character::default()
        };
        assert_eq!(character_picture(&luke), Some(18_498));
        luke.dagobah_completed = true;
        assert_eq!(character_picture(&luke), Some(0x4850));
        let vader = Character {
            dat_id: DatId::new(0x3500_0281),
            is_major: true,
            dagobah_completed: true,
            ..Character::default()
        };
        assert_eq!(character_picture(&vader), Some(18_561));
        let minor = Character {
            dat_id: DatId::new(0x242),
            is_major: false,
            dagobah_completed: true,
            ..Character::default()
        };
        assert_eq!(character_picture(&minor), None);
    }
}
