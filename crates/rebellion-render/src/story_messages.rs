//! Story events as messages in the Message window.
//!
//! The original shows a story event as a message: R2-D2 files it, its
//! category lights on the alert bar, and the Message window's
//! single-message mode (`FUN_00469de0`) draws its picture above its text
//! (`FUN_0046a320`). Each message class builds its own picture in its
//! vtable slot `+0x14` (`ghidra/notes/message-index-rows.md`).

use std::collections::HashMap;

use rebellion_core::events::{
    EventAction, EVT_CHARACTER_FORCE, EVT_JABBA_CAPTURES_CHEWIE, EVT_LEIA_FORCE, EVT_LUKE_DAGOBAH,
};
use rebellion_core::ids::{CharacterKey, DatId};
use rebellion_core::world::GameWorld;

use crate::cockpit::CockpitFaction;
use crate::message_log::{GameMessage, MessageCategory, MessagePicture, MessageRail, RailAudience};
use crate::system_window::character_mini_resource_id;

/// The category story messages are filed under: the Mission rail, bit
/// `0x10`, which the story classes' constructors take from `FUN_0048af30`
/// (Luke Goes to Dagobah `FUN_0048d070`, Force Growth `FUN_0048ed00`,
/// Future Jedi `FUN_0048f0e0`, Jabba `FUN_00490250`).
///
/// port: story events without a recovered message class use the same bit.
pub const STORY_RAIL: MessageRail = MessageRail::Mission;

/// The original message class a port story event is posted as, where the
/// class is recovered and the port's event means the same thing
/// (`ghidra/notes/message-index-rows.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryClass {
    /// Notification `0x221`, `FUN_0048d190`.
    LukeGoesToDagobah,
    /// Notification `0x1e1`, `FUN_0048ed80`.
    ForceGrowth,
    /// Notification `0x362` for Leia, `FUN_0048f1e0`'s `0x71d8` branch.
    LeiaUsesForce,
    /// Notification `0x1e0` case 5, `FUN_00490340`: a character taken at
    /// Jabba's palace.
    JabbaCaptures,
}

impl StoryClass {
    /// The title and body, TEXTSTRA `RT_RCDATA` ids.
    #[must_use]
    pub const fn text_ids(self) -> (u16, u16) {
        match self {
            Self::LukeGoesToDagobah => (0x71b8, 0x71b9),
            Self::ForceGrowth => (0x7180, 0x7181),
            Self::LeiaUsesForce => (0x71d8, 0x71d9),
            Self::JabbaCaptures => (0x7156, 0x7157),
        }
    }

    /// The STRATEGY picture: the class's background, and the subject's
    /// portrait where the class shows one.
    #[must_use]
    pub fn picture(self, side: CockpitFaction, subject: Option<(DatId, bool)>) -> MessagePicture {
        let portrait = subject.and_then(|(dat_id, major)| character_portrait(dat_id, major));
        match self {
            Self::LukeGoesToDagobah => MessagePicture {
                background: 0x421,
                foreground: None,
            },
            Self::ForceGrowth | Self::LeiaUsesForce => MessagePicture {
                background: side_background(side),
                foreground: portrait,
            },
            Self::JabbaCaptures => MessagePicture {
                background: 0x429,
                foreground: None,
            },
        }
    }
}

/// The message class a fired port story event is posted as.
///
/// `0x221` and `0x1e1` share the original's ids. The port's Leia Force
/// discovery (`0x363`) is the original's `0x362` for Leia. Its `0x387`
/// is the original's Jabba capture only where the event captures someone:
/// the port also uses that id for Han's rescue. The port's Final Battle
/// (`0x220`) fires as the battle begins and the original's message
/// (`FUN_0048e3c0`) reports its outcome, so it has no class.
#[must_use]
pub fn story_class(event_id: u32, actions: &[EventAction]) -> Option<StoryClass> {
    match event_id {
        EVT_LUKE_DAGOBAH => Some(StoryClass::LukeGoesToDagobah),
        EVT_CHARACTER_FORCE => Some(StoryClass::ForceGrowth),
        EVT_LEIA_FORCE => Some(StoryClass::LeiaUsesForce),
        EVT_JABBA_CAPTURES_CHEWIE
            if actions
                .iter()
                .any(|action| matches!(action, EventAction::CaptureCharacter { .. })) =>
        {
            Some(StoryClass::JabbaCaptures)
        }
        _ => None,
    }
}

/// The character a story message is about: the first one its event's
/// actions name. The port's Force Growth milestone names none and is
/// Luke's (`story_events::define_story_events`).
#[must_use]
pub fn story_subject(
    world: &GameWorld,
    class: StoryClass,
    actions: &[EventAction],
) -> Option<CharacterKey> {
    actions
        .iter()
        .find_map(|action| match action {
            EventAction::ModifyForceTier { character, .. }
            | EventAction::CaptureCharacter { character, .. }
            | EventAction::StartJediTraining { character } => Some(*character),
            _ => None,
        })
        .or_else(|| {
            (class == StoryClass::ForceGrowth)
                .then(|| {
                    world
                        .characters
                        .iter()
                        .find(|(_, character)| character.name.to_lowercase().contains("luke"))
                        .map(|(key, _)| key)
                })
                .flatten()
        })
}

/// The message a fired story event posts: filed under [`STORY_RAIL`] for
/// both sides and, where its class is recovered, titled, worded and
/// pictured as the original's (`port_text` stays otherwise).
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "The event, the world it names, the player's side and the text table are all inputs."
)]
pub fn story_message(
    world: &GameWorld,
    event_id: u32,
    actions: &[EventAction],
    port_text: String,
    tick: u64,
    side: CockpitFaction,
    texts: &HashMap<u16, String>,
    category: MessageCategory,
) -> GameMessage {
    let mut message =
        GameMessage::new(tick, port_text, category).on_rail(STORY_RAIL, RailAudience::Both);
    let Some(class) = story_class(event_id, actions) else {
        return message;
    };
    let subject = story_subject(world, class, actions).and_then(|key| world.characters.get(key));
    let parameters: Vec<MessageParameter<'_>> = subject
        .map(|character| MessageParameter::Character {
            name: &character.name,
            // The Force ranking reads the Force level +0x8c (FUN_004eee40).
            force_ranking: crate::status_window::force_ranking(character.force_level()),
        })
        .into_iter()
        .collect();
    if let Some((title, body)) = story_text(class, texts, &parameters) {
        message.text = body;
        message = message.with_title(title);
    }
    message.with_picture(class.picture(
        side,
        subject.map(|character| (character.dat_id, character.is_major)),
    ))
}

/// Notification `0x221`, Luke Goes to Dagobah (`FUN_0048d190`): built for
/// the Alliance view (`+0x20` == 1) once Luke's mission is the Dagobah one
/// (family `0x71`), titled and worded from TEXTSTRA `0x71b8`/`0x71b9` on
/// picture `0x421`.
#[must_use]
pub fn luke_goes_to_dagobah(tick: u64, texts: &HashMap<u16, String>) -> Option<GameMessage> {
    let (title, body) = story_text(StoryClass::LukeGoesToDagobah, texts, &[])?;
    Some(
        GameMessage::new(tick, body, MessageCategory::Event)
            .on_rail(STORY_RAIL, RailAudience::Alliance)
            .with_title(title)
            .with_picture(StoryClass::LukeGoesToDagobah.picture(CockpitFaction::Alliance, None)),
    )
}

/// Notification `0x210`, Luke Leaves Dagobah (`FUN_0048d0f0`): when Luke's
/// `DagobahCompleted` is set, for the Alliance view only, TEXTSTRA
/// `0x71ba`/`0x71bb` on background `0x412` with Luke's portrait
/// (`FUN_004f3e50(1)`, `FUN_004c5000`).
#[must_use]
pub fn luke_leaves_dagobah(
    world: &GameWorld,
    luke: CharacterKey,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    let luke = world.characters.get(luke)?;
    let title = fill_message_text(texts.get(&0x71ba)?, &[]);
    let body = fill_message_text(texts.get(&0x71bb)?, &[]);
    Some(
        GameMessage::new(tick, body, MessageCategory::Event)
            .on_rail(STORY_RAIL, RailAudience::Alliance)
            .with_title(title)
            .with_picture(MessagePicture {
                background: 0x412,
                foreground: character_portrait(luke.dat_id, luke.is_major),
            }),
    )
}

/// GNPRTB 2581 (`DAT_006b90dc`) = 100: the Force level at which a trainer
/// may train others (`FUN_004ed310` -> `FUN_0053eb90`), the rank of Jedi
/// Knight.
const GNPRTB_TRAINING_FORCE: u16 = 2581;

/// Leia's record (`0x31000241`), tested by `FUN_0048f1e0`.
const LEIA: u32 = 0x241;

/// Notification `0x362`, `CharacterForceUserDiscoveredKeyNotif`, which the
/// discoverer field `+0xa4` sends when a trainer finds `found`'s Force
/// (timer `0x389`, `FUN_004f0940`). Its builder `FUN_0048f1e0`:
///
/// - shows only to the trainer's side (`FUN_004edc80`), on the side's
///   background (`(side != 1) + 0x412`);
/// - for Leia (`0x31000241`), "Leia Uses Force" (TEXTSTRA `0x71d8` /
///   `0x71d9`) with her portrait;
/// - for anyone else, "Future Jedi Discovered" (`0x7178`) with the
///   trainer's portrait (`FUN_004c5000`), the trainer telling of `found`
///   (`{1:1}`): `0x7179` (train now) when the trainer trains Jedi at Force
///   GNPRTB 2581 (100) or more (`FUN_004ed310`), else `0x717a` (once a
///   Jedi Knight).
///
/// port: the builder's `+0x28`, `+0x30` and `+0x32` (a per-character id and
/// two resources) are not read; the message keeps the Mission rail of the
/// story classes (`FUN_0048f0e0`).
#[must_use]
pub fn force_user_discovered(
    world: &GameWorld,
    trainer: CharacterKey,
    found: CharacterKey,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    let teacher = world.characters.get(trainer)?;
    let pupil = world.characters.get(found)?;
    let (side, audience) = if teacher.is_alliance {
        (CockpitFaction::Alliance, RailAudience::Alliance)
    } else if teacher.is_empire {
        (CockpitFaction::Empire, RailAudience::Empire)
    } else {
        return None;
    };
    let (title, body, speaker) = if pupil.dat_id.index() == LEIA {
        (0x71d8, 0x71d9, pupil)
    } else {
        let knight = i64::from(teacher.force_level())
            >= i64::from(
                world
                    .gnprtb
                    .value(GNPRTB_TRAINING_FORCE, world.difficulty_index),
            );
        let body = if teacher.is_jedi_trainer && knight {
            0x7179
        } else {
            0x717a
        };
        (0x7178, body, teacher)
    };
    let parameters = [
        MessageParameter::Character {
            name: &pupil.name,
            force_ranking: crate::status_window::force_ranking(pupil.force_level()),
        },
        MessageParameter::Character {
            name: &teacher.name,
            force_ranking: crate::status_window::force_ranking(teacher.force_level()),
        },
    ];
    let title = fill_message_text(texts.get(&title)?, &parameters);
    let body = fill_message_text(texts.get(&body)?, &parameters);
    Some(
        GameMessage::new(tick, body, MessageCategory::Event)
            .on_rail(STORY_RAIL, audience)
            .with_title(title)
            .with_picture(MessagePicture {
                background: side_background(side),
                foreground: character_portrait(speaker.dat_id, speaker.is_major),
            }),
    )
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
/// (`FUN_0060b9d0`'s four), or `None` when the texts are missing.
#[must_use]
pub fn story_text(
    class: StoryClass,
    texts: &HashMap<u16, String>,
    parameters: &[MessageParameter<'_>],
) -> Option<(String, String)> {
    let (title, body) = class.text_ids();
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

    fn discovery_texts() -> HashMap<u16, String> {
        HashMap::from([
            (0x7178, "Future Jedi Discovered".to_owned()),
            (0x7179, "I have detected that {1:1} has the ability to use the Force.  We should consider allowing me to conduct training in the way of the Force.".to_owned()),
            (0x717a, "I have detected that {1:1} has the ability to use the Force.  Once I reach the rank of Jedi Knight, I should conduct training in the use of the Force.".to_owned()),
            (0x71d8, "Leia Uses Force".to_owned()),
            (0x71d9, "My heritage as a member of the Skywalker family has given me the ability to use the Force, as it has my brother and father.".to_owned()),
        ])
    }

    fn discovery_world(
        teacher_force: u16,
    ) -> (GameWorld, CharacterKey, CharacterKey, CharacterKey) {
        use rebellion_core::world::{Character, GnprtbEntry, GnprtbParams};
        let mut world = GameWorld {
            gnprtb: GnprtbParams::new(vec![GnprtbEntry {
                parameter_id: 2581,
                development: 100,
                alliance_sp_easy: 100,
                alliance_sp_medium: 100,
                alliance_sp_hard: 100,
                empire_sp_easy: 100,
                empire_sp_medium: 100,
                empire_sp_hard: 100,
                multiplayer: 100,
            }]),
            ..GameWorld::default()
        };
        let vader = world.characters.insert(Character {
            dat_id: DatId::new(0x281),
            name: "Darth Vader".into(),
            is_empire: true,
            is_major: true,
            is_jedi_trainer: true,
            force_base: teacher_force,
            ..Character::default()
        });
        let klev = world.characters.insert(Character {
            dat_id: DatId::new(0x3a0),
            name: "Klev".into(),
            is_empire: true,
            ..Character::default()
        });
        let leia = world.characters.insert(Character {
            dat_id: DatId::new(0x241),
            name: "Leia Organa".into(),
            is_alliance: true,
            is_major: true,
            ..Character::default()
        });
        (world, vader, klev, leia)
    }

    #[test]
    fn luke_s_dagobah_messages_go_to_the_alliance_only() {
        // FUN_0048d190 (0x221): 0x71b8/0x71b9 on picture 0x421; FUN_0048d0f0
        // (0x210): 0x71ba/0x71bb on 0x412 with Luke's portrait; both for
        // the Alliance view (+0x20 == 1).
        let texts = HashMap::from([
            (0x71b8, "Luke Goes to Dagobah".to_owned()),
            (
                0x71b9,
                "Luke has been sent to Dagobah to be trained by Yoda.".to_owned(),
            ),
            (0x71ba, "Luke Leaves Dagobah".to_owned()),
            (0x71bb, "I have finished my training with Yoda.".to_owned()),
        ]);
        let goes = luke_goes_to_dagobah(4, &texts).expect("a message");
        assert_eq!(goes.title.as_deref(), Some("Luke Goes to Dagobah"));
        assert_eq!(goes.audience, RailAudience::Alliance);
        assert_eq!(goes.picture.map(|p| p.background), Some(0x421));
        let mut world = GameWorld::default();
        let luke = world.characters.insert(rebellion_core::world::Character {
            dat_id: DatId::new(0x242),
            name: "Luke Skywalker".into(),
            is_alliance: true,
            is_major: true,
            ..Default::default()
        });
        let leaves = luke_leaves_dagobah(&world, luke, 9, &texts).expect("a message");
        assert_eq!(leaves.title.as_deref(), Some("Luke Leaves Dagobah"));
        assert_eq!(leaves.text, "I have finished my training with Yoda.");
        assert_eq!(leaves.audience, RailAudience::Alliance);
        let picture = leaves.picture.expect("a picture");
        assert_eq!(picture.background, 0x412);
        assert!(picture.foreground.is_some());
        assert_eq!(
            picture.foreground,
            character_portrait(DatId::new(0x242), true)
        );
    }

    #[test]
    fn a_discovery_tells_the_trainers_side_what_it_found() {
        // FUN_0048f1e0: 0x7178 with the trainer's portrait on its side's
        // background (0x413 for the Empire), the found character as {1:1};
        // 0x7179 at Force GNPRTB 2581 (100) or more (FUN_004ed310), shown
        // only to the trainer's side (FUN_004edc80).
        let (world, vader, klev, _) = discovery_world(120);
        let message =
            force_user_discovered(&world, vader, klev, 9, &discovery_texts()).expect("a message");
        assert_eq!(message.title.as_deref(), Some("Future Jedi Discovered"));
        assert_eq!(
            message.text,
            "I have detected that Klev has the ability to use the Force.  We should consider allowing me to conduct training in the way of the Force."
        );
        assert_eq!(message.audience, RailAudience::Empire);
        let picture = message.picture.expect("a picture");
        assert_eq!(picture.background, 0x413);
        assert_eq!(
            picture.foreground,
            character_portrait(DatId::new(0x281), true)
        );
        assert!(picture.foreground.is_some());
        // At exactly 100 FUN_0053eb90 (2581 <= Force) holds.
        let (world, vader, klev, _) = discovery_world(100);
        let message = force_user_discovered(&world, vader, klev, 9, &discovery_texts()).unwrap();
        assert!(message.text.contains("We should consider allowing me"));
        // Below 100: "Once I reach the rank of Jedi Knight".
        let (world, vader, klev, _) = discovery_world(99);
        let message = force_user_discovered(&world, vader, klev, 9, &discovery_texts()).unwrap();
        assert!(message
            .text
            .contains("Once I reach the rank of Jedi Knight"));
    }

    #[test]
    fn leia_found_uses_the_force_in_her_own_words() {
        // FUN_0048f1e0: 0x31000241 takes 0x71d8 / 0x71d9 and her portrait.
        let (mut world, _, _, leia) = discovery_world(80);
        let luke = world.characters.insert(rebellion_core::world::Character {
            dat_id: DatId::new(0x242),
            name: "Luke Skywalker".into(),
            is_alliance: true,
            is_major: true,
            is_jedi_trainer: true,
            force_base: 80,
            ..Default::default()
        });
        let message = force_user_discovered(&world, luke, leia, 9, &discovery_texts()).unwrap();
        assert_eq!(message.title.as_deref(), Some("Leia Uses Force"));
        assert!(message.text.starts_with("My heritage as a member"));
        assert_eq!(message.audience, RailAudience::Alliance);
        let picture = message.picture.unwrap();
        assert_eq!(picture.background, 0x412);
        assert_eq!(
            picture.foreground,
            character_portrait(DatId::new(0x241), true)
        );
    }

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
            story_text(StoryClass::ForceGrowth, &texts, &[luke]),
            Some((
                "Luke Skywalker Force Growth".to_owned(),
                "New insights.  My ranking is Trainee.".to_owned()
            ))
        );
        assert_eq!(
            story_text(StoryClass::LukeGoesToDagobah, &texts, &[]).map(|(title, _)| title),
            Some("Luke Goes to Dagobah".to_owned())
        );
        // No texts (no TEXTSTRA.DLL): none.
        assert_eq!(
            story_text(StoryClass::LukeGoesToDagobah, &HashMap::new(), &[]),
            None
        );
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
    fn each_class_carries_its_recovered_texts_and_picture() {
        // FUN_0048d190, FUN_0048ed80, FUN_0048f1e0 (Leia), FUN_00490340
        // case 5: titles, bodies and backgrounds; the side's background is
        // (side != 1) + 0x412.
        let luke = Some((DatId::new(578), true));
        let leia = Some((DatId::new(577), true));
        let expect = |class: StoryClass, side, subject, ids, background, foreground| {
            assert_eq!(class.text_ids(), ids, "{class:?}");
            assert_eq!(
                class.picture(side, subject),
                MessagePicture {
                    background,
                    foreground
                },
                "{class:?}"
            );
        };
        use CockpitFaction::{Alliance, Empire};
        expect(
            StoryClass::LukeGoesToDagobah,
            Empire,
            luke,
            (0x71b8, 0x71b9),
            1057,
            None,
        );
        expect(
            StoryClass::ForceGrowth,
            Alliance,
            luke,
            (0x7180, 0x7181),
            1042,
            Some(6210),
        );
        expect(
            StoryClass::ForceGrowth,
            Empire,
            luke,
            (0x7180, 0x7181),
            1043,
            Some(6210),
        );
        expect(
            StoryClass::LeiaUsesForce,
            Alliance,
            leia,
            (0x71d8, 0x71d9),
            1042,
            Some(6209),
        );
        expect(
            StoryClass::JabbaCaptures,
            Alliance,
            None,
            (0x7156, 0x7157),
            1065,
            None,
        );
    }

    #[test]
    fn port_story_events_map_to_their_original_classes_where_they_mean_the_same() {
        // ghidra/notes/message-index-rows.md: 0x221 and 0x1e1 share the
        // original's ids; the port's 0x363 is the original's 0x362 for
        // Leia; 0x387 is Jabba's capture only when it captures someone;
        // the Final Battle (0x220) and the gates have no class.
        let (mut world, character) = (GameWorld::default(), CharacterKey::default());
        let capture = [EventAction::CaptureCharacter {
            character,
            captor_faction: rebellion_core::dat::Faction::Empire,
        }];
        assert_eq!(story_class(0x221, &[]), Some(StoryClass::LukeGoesToDagobah));
        assert_eq!(story_class(0x1e1, &[]), Some(StoryClass::ForceGrowth));
        assert_eq!(story_class(0x363, &[]), Some(StoryClass::LeiaUsesForce));
        assert_eq!(
            story_class(0x387, &capture),
            Some(StoryClass::JabbaCaptures)
        );
        assert_eq!(story_class(0x387, &[]), None);
        for unmapped in [0x220, 0x200, 0x212, 0x362, 0x399] {
            assert_eq!(story_class(unmapped, &[]), None, "{unmapped:#x}");
        }

        // The subject is the character the actions name, else Luke for
        // Force Growth.
        assert_eq!(
            story_subject(&world, StoryClass::JabbaCaptures, &capture),
            Some(character)
        );
        assert_eq!(story_subject(&world, StoryClass::ForceGrowth, &[]), None);
        let luke = world.characters.insert(rebellion_core::world::Character {
            name: "Luke Skywalker".into(),
            ..Default::default()
        });
        assert_eq!(
            story_subject(&world, StoryClass::ForceGrowth, &[]),
            Some(luke)
        );
        assert_eq!(story_subject(&world, StoryClass::LeiaUsesForce, &[]), None);
    }

    #[test]
    fn a_story_event_posts_its_classes_message_or_keeps_the_port_text() {
        // FUN_0048f1e0's Leia branch: TEXTSTRA 0x71d8 / 0x71d9 over the
        // side's background with her portrait, filed under bit 0x10.
        let mut world = GameWorld::default();
        let leia = world.characters.insert(rebellion_core::world::Character {
            dat_id: DatId::new(577),
            name: "Leia Organa".into(),
            is_major: true,
            ..Default::default()
        });
        let texts = HashMap::from([
            (0x71d8, "Leia Uses Force".to_owned()),
            (0x71d9, "My heritage gives me the Force.".to_owned()),
        ]);
        let actions = [EventAction::ModifyForceTier {
            character: leia,
            new_tier: rebellion_core::world::ForceTier::Aware,
        }];
        let message = story_message(
            &world,
            0x363,
            &actions,
            "port text".into(),
            40,
            CockpitFaction::Empire,
            &texts,
            MessageCategory::Event,
        );
        assert_eq!(message.title.as_deref(), Some("Leia Uses Force"));
        assert_eq!(message.text, "My heritage gives me the Force.");
        assert_eq!(
            message.picture,
            Some(MessagePicture {
                background: 1043,
                foreground: Some(6209)
            })
        );
        assert_eq!(message.rail, Some(STORY_RAIL));

        let gate = story_message(
            &world,
            0x399,
            &[],
            "port text".into(),
            40,
            CockpitFaction::Empire,
            &texts,
            MessageCategory::Event,
        );
        assert_eq!(
            (gate.text.as_str(), gate.title, gate.picture),
            ("port text", None, None)
        );
        assert_eq!(gate.rail, Some(STORY_RAIL));
    }
}
