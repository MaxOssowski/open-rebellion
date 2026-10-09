//! Story events as messages in the Message window.
//!
//! The original shows a story event as a message: R2-D2 files it, its
//! category lights on the alert bar, and the Message window's
//! single-message mode (`FUN_00469de0`) draws its picture above its text
//! (`FUN_0046a320`). Each message class builds its own picture in its
//! vtable slot `+0x14` (`ghidra/notes/message-index-rows.md`).

use std::collections::HashMap;

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

/// A story message's original title and body, TEXTSTRA `RT_RCDATA` ids.
///
/// - `0x221` Luke Goes to Dagobah (`FUN_0048d190`): `0x71b8`, `0x71b9`.
/// - `0x1e1` Force Growth (`FUN_0048ed80`): `0x7180`, `0x7181`, the
///   character's name as parameter 1.
#[must_use]
pub const fn story_text_ids(event_id: u32) -> Option<(u16, u16)> {
    match event_id {
        EVT_LUKE_DAGOBAH => Some((0x71b8, 0x71b9)),
        EVT_CHARACTER_FORCE => Some((0x7180, 0x7181)),
        _ => None,
    }
}

/// One of a message's four parameters (`FUN_0060b9d0`): an object whose
/// property a substitution's kind selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageParameter<'a> {
    /// A character: kind 1 its name, kind 7 its Force ranking (TEXTSTRA
    /// 0x7181 "My ranking is |1:7").
    Character {
        name: &'a str,
        force_ranking: &'a str,
    },
}

impl MessageParameter<'_> {
    /// The text a substitution of `kind` takes from this parameter.
    #[must_use]
    pub fn text(&self, kind: u8) -> Option<&str> {
        match (self, kind) {
            (Self::Character { name, .. }, 1) => Some(name),
            (Self::Character { force_ranking, .. }, 7) => Some(force_ranking),
            _ => None,
        }
    }
}

/// The story message's title and body from the player's TEXTSTRA
/// message texts, its substitutions filled from `parameters`
/// (`FUN_0060b9d0`'s four), or `None` when the class or the texts are
/// missing.
#[must_use]
pub fn story_text(
    event_id: u32,
    texts: &HashMap<u16, String>,
    parameters: &[MessageParameter<'_>],
) -> Option<(String, String)> {
    let (title, body) = story_text_ids(event_id)?;
    Some((
        fill_message_text(texts.get(&title)?, parameters),
        fill_message_text(texts.get(&body)?, parameters),
    ))
}

/// Replace each `{parameter:kind}` with that parameter's text of that kind
/// (empty when the parameter or the kind is missing).
#[must_use]
pub fn fill_message_text(template: &str, parameters: &[MessageParameter<'_>]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let token = &after[..end];
        let parsed = token.split_once(':').and_then(|(index, kind)| {
            Some((index.parse::<usize>().ok()?, kind.parse::<u8>().ok()?))
        });
        match parsed {
            Some((index, kind)) => out.push_str(
                index
                    .checked_sub(1)
                    .and_then(|slot| parameters.get(slot))
                    .and_then(|parameter| parameter.text(kind))
                    .unwrap_or_default(),
            ),
            None => out.push_str(&rest[start..=start + 1 + end]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
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
    fn story_texts_come_from_the_players_message_texts_with_names_filled() {
        // FUN_0048ed80: Force Growth fills 0x7180 / 0x7181 with the
        // character's name; FUN_0048d190: 0x71b8 / 0x71b9 take none.
        let texts = HashMap::from([
            (0x7180, "{1:1} Force Growth".to_owned()),
            (0x7181, "New insights.  My ranking is {1:7}.".to_owned()),
            (0x71b8, "Luke Goes to Dagobah".to_owned()),
            (0x71b9, "Luke has been sent to Dagobah.".to_owned()),
        ]);
        let luke = MessageParameter::Character {
            name: "Luke Skywalker",
            force_ranking: "Trainee",
        };
        assert_eq!(
            story_text(EVT_CHARACTER_FORCE, &texts, &[luke]),
            Some((
                "Luke Skywalker Force Growth".to_owned(),
                "New insights.  My ranking is Trainee.".to_owned()
            ))
        );
        assert_eq!(
            story_text(EVT_LUKE_DAGOBAH, &texts, &[]).map(|(title, _)| title),
            Some("Luke Goes to Dagobah".to_owned())
        );
        // No texts (no TEXTSTRA.DLL) or no recovered class: none.
        assert_eq!(story_text(EVT_LUKE_DAGOBAH, &HashMap::new(), &[]), None);
        assert_eq!(story_text(0x399, &texts, &[]), None);
    }

    #[test]
    fn a_substitution_takes_its_parameters_property_of_its_kind() {
        // TEXTSTRA 0x7181: the same character as name (kind 1) and as Force
        // ranking (kind 7); an unknown kind or parameter fills nothing.
        let leia = MessageParameter::Character {
            name: "Leia",
            force_ranking: "Novice",
        };
        assert_eq!(
            fill_message_text("{1:1} is a {1:7}.", &[leia]),
            "Leia is a Novice."
        );
        assert_eq!(fill_message_text("{1:4}|{2:1}", &[leia]), "|");
        assert_eq!(fill_message_text("odd {brace", &[]), "odd {brace");
        assert_eq!(fill_message_text("keep {x}", &[]), "keep {x}");
    }

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
