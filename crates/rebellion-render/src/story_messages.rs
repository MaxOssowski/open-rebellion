//! Story events as messages in the Message window.
//!
//! The original shows a story event as a message: R2-D2 files it, its
//! category lights on the alert bar, and the Message window's
//! single-message mode (`FUN_00469de0`) draws its picture above its text
//! (`FUN_0046a320`). Each message class builds its own picture in its
//! vtable slot `+0x14` (`ghidra/notes/message-index-rows.md`).

use rebellion_core::events::{EVT_CHARACTER_FORCE, EVT_LUKE_DAGOBAH};
use rebellion_core::ids::DatId;

use crate::cockpit::CockpitFaction;
use crate::message_log::{MessagePicture, MessageRail};
use crate::system_window::character_mini_resource_id;

/// The category story messages are filed under: the Mission rail, bit
/// `0x10`, which the story classes' constructors take from `FUN_0048af30`
/// (Luke Goes to Dagobah `FUN_0048d070`, Force Growth `FUN_0048ed00`,
/// Future Jedi `FUN_0048f0e0`, Jabba `FUN_00490250`).
///
/// port: story events without a recovered message class use the same bit.
pub const STORY_RAIL: MessageRail = MessageRail::Mission;

/// The STRATEGY picture a story message carries, where its class is
/// recovered.
///
/// - `0x221` Luke Goes to Dagobah (`FUN_0048d190`): background `0x421`.
/// - `0x1e1` Force Growth (`FUN_0048ed80`): the side's background
///   (`0x412` Alliance, `0x413` Empire) with the character's portrait.
///
/// `subject` is the event's character (its DAT id and whether it is a
/// major character).
#[must_use]
pub fn story_picture(
    event_id: u32,
    side: CockpitFaction,
    subject: Option<(DatId, bool)>,
) -> Option<MessagePicture> {
    match event_id {
        EVT_LUKE_DAGOBAH => Some(MessagePicture {
            background: 0x421,
            foreground: None,
        }),
        EVT_CHARACTER_FORCE => Some(MessagePicture {
            background: side_background(side),
            foreground: subject.and_then(|(dat_id, major)| character_portrait(dat_id, major)),
        }),
        _ => None,
    }
}

/// The side's story background (`(side != 1) + 0x412`).
const fn side_background(side: CockpitFaction) -> u32 {
    match side {
        CockpitFaction::Alliance => 0x412,
        CockpitFaction::Empire => 0x413,
    }
}

/// A character's 400 by 200 story portrait (`FUN_004c5000`): its class's
/// picture (`+0x30 & 0xfff`, the list mini less `0x4000`) plus `0x1000`.
/// STRATEGY 6208-6273 hold the major characters' and 6720-6811 the
/// minor characters'.
#[must_use]
pub fn character_portrait(dat_id: DatId, major: bool) -> Option<u32> {
    character_mini_resource_id(dat_id, major).map(|mini| mini - 0x4000 + 0x1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_characters_story_portrait_is_its_picture_plus_0x1000() {
        // FUN_004c5000: (+0x30 & 0xfff) + 0x1000. Luke's picture is 0x842
        // (GOKRES 2114), so his portrait is STRATEGY 0x1842 = 6210; the
        // minor characters 832.. and 896.. land on 6720.. and 6784..
        assert_eq!(character_portrait(DatId::new(578), true), Some(6210));
        assert_eq!(character_portrait(DatId::new(592), true), Some(6224));
        assert_eq!(character_portrait(DatId::new(832), false), Some(6720));
        assert_eq!(character_portrait(DatId::new(923), false), Some(6811));
        assert_eq!(character_portrait(DatId::new(700), false), None);
    }

    #[test]
    fn recovered_story_messages_carry_their_classes_pictures() {
        // FUN_0048d190: Luke Goes to Dagobah, background 0x421.
        assert_eq!(
            story_picture(EVT_LUKE_DAGOBAH, CockpitFaction::Empire, None),
            Some(MessagePicture {
                background: 1057,
                foreground: None
            })
        );
        // FUN_0048ed80: Force Growth, the side's background and the
        // character's portrait.
        let luke = Some((DatId::new(578), true));
        assert_eq!(
            story_picture(EVT_CHARACTER_FORCE, CockpitFaction::Alliance, luke),
            Some(MessagePicture {
                background: 1042,
                foreground: Some(6210)
            })
        );
        assert_eq!(
            story_picture(EVT_CHARACTER_FORCE, CockpitFaction::Empire, luke)
                .map(|picture| picture.background),
            Some(1043)
        );
        // No recovered class: no picture.
        assert_eq!(story_picture(0x399, CockpitFaction::Alliance, None), None);
    }
}
