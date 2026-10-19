//! Story events as messages in the Message window.
//!
//! The original shows a story event as a message: R2-D2 files it, its
//! category lights on the alert bar, and the Message window's
//! single-message mode (`FUN_00469de0`) draws its picture above its text
//! (`FUN_0046a320`). Each message class builds its own picture in its
//! vtable slot `+0x14` (`ghidra/notes/message-index-rows.md`).

use std::collections::HashMap;

use rebellion_core::events::{EventAction, EVT_CHARACTER_FORCE, EVT_LUKE_DAGOBAH};
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
/// `0x221` and `0x1e1` share the original's ids. Leia's Force discovery is
/// the trainer's `0x362` (`force_user_discovered`). Jabba's
/// capture is posted by [`jabba_captures_message`] since the port's own
/// `0x387` went (save v77). The port's Final Battle
/// (`0x220`) fires as the battle begins and the original's message
/// (`FUN_0048e3c0`) reports its outcome, so it has no class.
#[must_use]
pub fn story_class(event_id: u32) -> Option<StoryClass> {
    match event_id {
        EVT_LUKE_DAGOBAH => Some(StoryClass::LukeGoesToDagobah),
        EVT_CHARACTER_FORCE => Some(StoryClass::ForceGrowth),
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
            EventAction::CaptureCharacter { character, .. } => Some(*character),
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
    let Some(class) = story_class(event_id) else {
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

/// Luke's (`0x32000242`), Vader's (`0x35000281`) and the Emperor's
/// (`0x34000280`) records, which `FUN_0048d7f0` tests with Leia's.
const LUKE: u32 = 0x242;
const VADER: u32 = 0x281;
const EMPEROR: u32 = 0x280;

/// What the encounter class (vtable `0x65b408`) gathered over a day: each
/// notification `0x360` (`CharacterEncounterKeyNotif`, an `Encounter` key
/// set or cleared) files its character by side, the Alliance's at `+0x68`
/// and the Empire's at `+0x6c` (`FUN_0048d9a0`), the last one winning; and
/// `0x211` (`LukeKnowsHeritageNotif`) sets bit 0 (`FUN_0048d7f0`).
///
/// port: the class builds on notification 2, once a day; the port builds
/// once per mission advance, which runs the day's tick events together.
/// The Final Battle bit (`0x220`, `FUN_0048e3c0`) is not gathered: the
/// port's own `0x220` story event is not the original's notification.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EncounterNotes {
    alliance: Option<CharacterKey>,
    empire: Option<CharacterKey>,
    /// `+0x64` bit `0x10000000`: a notified object was no character of
    /// either side, so the class shows nothing.
    suppressed: bool,
    /// `+0x64` bit 0.
    heritage: bool,
}

impl EncounterNotes {
    /// Gather a mission advance's encounter notifications: the member's
    /// key, then the defender's (`FUN_0055ff60`), or the partner's
    /// (`FUN_00560200`), and Luke's `KnowsHeritage` (`FUN_0056f650`).
    #[must_use]
    pub fn gather(world: &GameWorld, effects: &[rebellion_core::missions::MissionEffect]) -> Self {
        use rebellion_core::missions::MissionEffect;
        let mut notes = Self::default();
        for effect in effects {
            match effect {
                MissionEffect::EncounterBegan {
                    member, defender, ..
                } => {
                    notes.file(world, *member);
                    notes.file(world, *defender);
                }
                MissionEffect::EncounterEnded { member, partner } => {
                    notes.file(world, *member);
                    notes.file(world, *partner);
                }
                MissionEffect::HeritageLearned { .. } => notes.heritage = true,
                _ => {}
            }
        }
        notes
    }

    /// `FUN_0048d9a0`: side 1 at `+0x68`, side 2 at `+0x6c`
    /// (`FUN_004edc80`); anything else suppresses the message.
    fn file(&mut self, world: &GameWorld, character: CharacterKey) {
        match world.characters.get(character) {
            Some(c) if c.is_alliance => self.alliance = Some(character),
            Some(c) if c.is_empire => self.empire = Some(character),
            _ => self.suppressed = true,
        }
    }

    /// `FUN_0048d7f0` on notification 2: for an Alliance Luke or Leia and
    /// an Empire Vader or Emperor, Luke Discovers His Heritage
    /// (`FUN_0048dc50`) when bit 0 is set, else Character Confronts
    /// character (`FUN_0048df20`); for any other pair, Force user Detects
    /// Enemy (`FUN_0048e1d0`), one per side. Each side's view builds its
    /// own message (`+0x40` = 1).
    #[must_use]
    pub fn messages(
        &self,
        world: &GameWorld,
        missions: &rebellion_core::missions::MissionState,
        tick: u64,
        texts: &HashMap<u16, String>,
    ) -> Vec<GameMessage> {
        if self.suppressed {
            return Vec::new();
        }
        let (Some(alliance), Some(empire)) = (self.alliance, self.empire) else {
            return Vec::new();
        };
        let (Some(a), Some(e)) = (world.characters.get(alliance), world.characters.get(empire))
        else {
            return Vec::new();
        };
        let jedi_pair =
            matches!(a.dat_id.index(), LUKE | LEIA) && matches!(e.dat_id.index(), VADER | EMPEROR);
        if jedi_pair {
            let built = if self.heritage {
                heritage_message(
                    a,
                    e,
                    taken_to_emperor(world, missions, alliance, empire),
                    tick,
                    texts,
                )
            } else {
                confrontation_message(a, e, tick, texts)
            };
            return built.into_iter().collect();
        }
        [
            (CockpitFaction::Alliance, RailAudience::Alliance, a, e),
            (CockpitFaction::Empire, RailAudience::Empire, e, a),
        ]
        .into_iter()
        .filter_map(|(side, audience, own, enemy)| {
            detects_enemy_message(side, audience, own, enemy, tick, texts)
        })
        .collect()
    }
}

/// The Final Battle (`FUN_0048e3c0`), which the encounter class builds
/// when `0x220` (`MissionMgrFinalBattleNotif`) set its bit `0x10`, for each
/// side: title `0x71e0`, body `0x71e1` with Luke free (he overcame the
/// Emperor) or `0x71e2` with him captured, on background `0x428` once
/// Luke's `DagobahCompleted` (`+0xb0` bit 8) is set, else `0x427`.
///
/// port: the media ids `+0x30` and `+0x32` are not read.
#[must_use]
pub fn final_battle_message(
    world: &GameWorld,
    luke: CharacterKey,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    let luke = world.characters.get(luke)?;
    let title = fill_message_text(texts.get(&0x71e0)?, &[]);
    let body_id = if luke.is_captive { 0x71e2 } else { 0x71e1 };
    let body = fill_message_text(texts.get(&body_id)?, &[]);
    Some(
        GameMessage::new(tick, body, MessageCategory::Event)
            .on_rail(STORY_RAIL, RailAudience::Both)
            .with_title(title)
            .with_picture(MessagePicture {
                background: if luke.dagobah_completed { 0x428 } else { 0x427 },
                foreground: None,
            }),
    )
}

/// Jabba Captures (`FUN_00490340` case 5): notification `0x1e0` for a
/// character captured with Jabba's Palace (`0x98000481`) as its captor,
/// the Palace mission's failed rescuer (`FUN_0056e870`). TEXTSTRA
/// `0x7156` / `0x7157` name it (`+0x30`) on background `0x429`.
///
/// port: posted to both sides, as the class's case 5 reads no side; the
/// media ids `+0x30`, `+0x32` and the speaker `+0x28` are not read.
#[must_use]
pub fn jabba_captures_message(
    character: &rebellion_core::world::Character,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    let class = StoryClass::JabbaCaptures;
    let (title, body) = class.text_ids();
    let parameters = [MessageParameter::Named {
        name: &character.name,
    }];
    Some(
        GameMessage::new(
            tick,
            fill_message_text(texts.get(&body)?, &parameters),
            MessageCategory::Event,
        )
        .on_rail(STORY_RAIL, RailAudience::Both)
        .with_title(fill_message_text(texts.get(&title)?, &parameters))
        .with_picture(class.picture(CockpitFaction::Alliance, None)),
    )
}

/// Character Discovers Traitor (`FUN_0048c9c0`): notification `0x361`,
/// a character's TraitorDiscovered key set to a traitor (`FUN_00588e80`).
/// The discoverer's side reads TEXTSTRA `0x71e8` "{1:1} Discovers Traitor"
/// / `0x71e9` with the traitor (2) and the other side's name (3,
/// `FUN_004f3dd0`), on its side background with the discoverer's portrait.
///
/// port: the side names are "Alliance" and "Empire" (the side records'
/// `+0x30` names are not loaded); the speakers and media are not read.
#[must_use]
pub fn discovers_traitor_message(
    discoverer: &rebellion_core::world::Character,
    traitor: &rebellion_core::world::Character,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    let (side, audience, other) = if discoverer.is_alliance {
        (CockpitFaction::Alliance, RailAudience::Alliance, "Empire")
    } else {
        (CockpitFaction::Empire, RailAudience::Empire, "Alliance")
    };
    let parameters = [
        MessageParameter::Named {
            name: &discoverer.name,
        },
        MessageParameter::Named {
            name: &traitor.name,
        },
        MessageParameter::Named { name: other },
    ];
    Some(
        GameMessage::new(
            tick,
            fill_message_text(texts.get(&0x71e9)?, &parameters),
            MessageCategory::Event,
        )
        .on_rail(STORY_RAIL, audience)
        .with_title(fill_message_text(texts.get(&0x71e8)?, &parameters))
        .with_picture(MessagePicture {
            background: side_background(side),
            foreground: character_portrait(discoverer.dat_id, discoverer.is_major),
        }),
    )
}

/// Rescue Attempt (`FUN_0048c4c0`): notification `0x200`
/// (`RoleEnrouteActiveNotif`), a character setting out (EnrouteActive)
/// whose second key, at record `+0x2c`, is a Palace mission (family
/// `0x64`, checked at `0x48c5d9`). The Alliance alone reads TEXTSTRA
/// `0x71ea` / `0x71eb` with the rescuer's name, on its side background
/// (`0x412`) with the rescuer's portrait (`FUN_004c5000`).
///
/// port: the speaker (`+0x28` by character) and media are not read.
#[must_use]
pub fn rescue_attempt_message(
    rescuer: &rebellion_core::world::Character,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    let parameters = [MessageParameter::Named {
        name: &rescuer.name,
    }];
    Some(
        GameMessage::new(
            tick,
            fill_message_text(texts.get(&0x71eb)?, &parameters),
            MessageCategory::Event,
        )
        .on_rail(STORY_RAIL, RailAudience::Alliance)
        .with_title(fill_message_text(texts.get(&0x71ea)?, &parameters))
        .with_picture(MessagePicture {
            background: side_background(CockpitFaction::Alliance),
            foreground: character_portrait(rescuer.dat_id, rescuer.is_major),
        }),
    )
}

/// Notification `0x128`, a change of the victory conditions word
/// (`ghidra/notes/victory-conditions.md`), for both sides:
///
/// - Alliance Headquarters Destroyed (`FUN_0048e5a0`) when the new word
///   has bit `0x1`: TEXTSTRA `0x7188` / `0x7189`;
/// - Coruscant Captured (`FUN_0048f790`) while Coruscant is the
///   Alliance's (bit `0x100`): `0x7170` / `0x7171`.
///
/// Neither has a once flag. port: their `+0x5c` (`0x73`, `0x74`; hyp: a
/// film) and speakers are not read, and they set no background.
#[must_use]
pub fn victory_conditions_messages(
    word: u32,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Vec<GameMessage> {
    let message = |title: u16, body: u16| {
        Some(
            GameMessage::new(tick, texts.get(&body)?.clone(), MessageCategory::Event)
                .on_rail(STORY_RAIL, RailAudience::Both)
                .with_title(texts.get(&title)?.clone()),
        )
    };
    let mut out = Vec::new();
    if word & rebellion_core::victory::WORD_HQ_LOST != 0 {
        out.extend(message(0x7188, 0x7189));
    }
    if word & rebellion_core::victory::WORD_CORUSCANT_ALLIANCE != 0 {
        out.extend(message(0x7170, 0x7171));
    }
    out
}

/// Emperor Arrives at Coruscant (`FUN_0048ce80`): notification `0x230`
/// with its argument set, as `SeatOfPower` is set. The Empire alone reads
/// TEXTSTRA `0x71c8` / `0x71c9` on background `0x42c`.
///
/// port: the speaker `0x1a`, the media `0x47b` and `FUN_004c4990(0x130)`
/// are not read.
#[must_use]
pub fn emperor_arrives_message(tick: u64, texts: &HashMap<u16, String>) -> Option<GameMessage> {
    Some(
        GameMessage::new(tick, texts.get(&0x71c9)?.clone(), MessageCategory::Event)
            .on_rail(STORY_RAIL, RailAudience::Empire)
            .with_title(texts.get(&0x71c8)?.clone())
            .with_picture(MessagePicture {
                background: 0x42c,
                foreground: None,
            }),
    )
}

/// Killed by Imperial Assassins (`FUN_0048b050`): notification `0x306`,
/// which `FUN_004fc080` posts for an object destroyed with reason 7 (the
/// assassination's injury), with the object and its location. TEXTSTRA
/// `0x7208` / `0x7209` name the character (1) and the system around its
/// location (3), on background `0x42e`, for the Alliance only.
///
/// port: the location is the character's system; the media id `0x474`
/// and the speaker `0x15` are not read.
#[must_use]
pub fn assassins_message(
    character: &rebellion_core::world::Character,
    system: &str,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    let parameters = [
        MessageParameter::Named {
            name: &character.name,
        },
        MessageParameter::Named { name: system },
        MessageParameter::Named { name: system },
    ];
    Some(
        GameMessage::new(
            tick,
            fill_message_text(texts.get(&0x7209)?, &parameters),
            MessageCategory::Event,
        )
        .on_rail(STORY_RAIL, RailAudience::Alliance)
        .with_title(fill_message_text(texts.get(&0x7208)?, &parameters))
        .with_picture(MessagePicture {
            background: 0x42e,
            foreground: None,
        }),
    )
}

/// Death Star Sabotaged (`FUN_0048d470`): notification `0x305`, which
/// `FUN_004fc080` posts for an object destroyed with reason 6 (sabotage),
/// when the object is of family `0x18`..`0x1b` (the Death Star) and its
/// location stands at a system. TEXTSTRA `0x71bd` / `0x71be` name the
/// system (1); both sides are shown it.
///
/// port: the builder sets no background but writes `+0x5c` = `0x72`
/// (hyp: a film, not traced); the message has no picture.
#[must_use]
pub fn death_star_sabotaged_message(
    system: &str,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    let parameters = [MessageParameter::Named { name: system }];
    Some(
        GameMessage::new(
            tick,
            fill_message_text(texts.get(&0x71be)?, &parameters),
            MessageCategory::Event,
        )
        .on_rail(STORY_RAIL, RailAudience::Both)
        .with_title(fill_message_text(texts.get(&0x71bd)?, &parameters)),
    )
}

/// Jabba Delivers Prisoners (`FUN_0048b6e0`): notification `0x231` with
/// its argument set, as the Pickup's hit sets `PickupInProgress`. TEXTSTRA
/// `0x7200` / `0x7201` take no arguments; the background is `0x42a`, or
/// `0x42b` when the side record's `+0xb0` bit 8 is set.
///
/// port: posted to both sides (`story-message-classes.md` row 15) on
/// `0x42a`, as the side record's bit is not traced; the media ids and the
/// speaker `0x1b` are not read.
#[must_use]
pub fn jabba_delivers_message(tick: u64, texts: &HashMap<u16, String>) -> Option<GameMessage> {
    Some(
        GameMessage::new(tick, texts.get(&0x7201)?.clone(), MessageCategory::Event)
            .on_rail(STORY_RAIL, RailAudience::Both)
            .with_title(texts.get(&0x7200)?.clone())
            .with_picture(MessagePicture {
                background: 0x42a,
                foreground: None,
            }),
    )
}

/// Notification `0x212` (`FUN_0048fa00`), Han's `BountyAttack`, for each
/// side:
///
/// - Han captured (`+0xac` bit 0): Han Solo Captured by Bounty Hunters
///   (`0x716c` / `0x716d`, background `0x41d`) for both;
/// - otherwise the Alliance reads Han Solo Attacked by Bounty Hunters
///   (`0x7168` / `0x7169`, `0x41c`), and the Empire Bounty Hunters Locate
///   Solo (`0x716a` / `0x716b` with the system Han stands at, `0x41e`) when
///   his containers reach a system.
///
/// port: the media ids `+0x30`, `+0x32` and the speaker `+0x28` are not
/// read.
#[must_use]
pub fn bounty_messages(
    world: &GameWorld,
    han: CharacterKey,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Vec<GameMessage> {
    let Some(c) = world.characters.get(han) else {
        return Vec::new();
    };
    let message =
        |title: u16, body: u16, parameters: &[MessageParameter<'_>], background, audience| {
            Some(
                GameMessage::new(
                    tick,
                    fill_message_text(texts.get(&body)?, parameters),
                    MessageCategory::Event,
                )
                .on_rail(STORY_RAIL, audience)
                .with_title(fill_message_text(texts.get(&title)?, parameters))
                .with_picture(MessagePicture {
                    background,
                    foreground: None,
                }),
            )
        };
    if c.is_captive {
        return message(0x716c, 0x716d, &[], 0x41d, RailAudience::Both)
            .into_iter()
            .collect();
    }
    let system = c
        .current_fleet
        .and_then(|fleet| world.fleets.get(fleet))
        .map(|fleet| fleet.location)
        .or(c.current_system)
        .and_then(|key| world.systems.get(key));
    let mut out: Vec<GameMessage> = message(0x7168, 0x7169, &[], 0x41c, RailAudience::Alliance)
        .into_iter()
        .collect();
    if let Some(system) = system {
        out.extend(message(
            0x716a,
            0x716b,
            &[MessageParameter::Named { name: &system.name }],
            0x41e,
            RailAudience::Empire,
        ));
    }
    out
}

fn character_parameter(c: &rebellion_core::world::Character) -> MessageParameter<'_> {
    MessageParameter::Character {
        name: &c.name,
        force_ranking: crate::status_window::force_ranking(c.force_level()),
    }
}

/// Force user Detects Enemy (`FUN_0048e1d0`): the side's own character on
/// the side's background (`(side != 1) + 0x412`) with its portrait,
/// TEXTSTRA `0x71b0` / `0x71b1` naming it and the enemy.
///
/// port: the media ids `+0x30` and `+0x32` are not read.
fn detects_enemy_message(
    side: CockpitFaction,
    audience: RailAudience,
    own: &rebellion_core::world::Character,
    enemy: &rebellion_core::world::Character,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    let parameters = [character_parameter(own), character_parameter(enemy)];
    let title = fill_message_text(texts.get(&0x71b0)?, &parameters);
    let body = fill_message_text(texts.get(&0x71b1)?, &parameters);
    Some(
        GameMessage::new(tick, body, MessageCategory::Event)
            .on_rail(STORY_RAIL, audience)
            .with_title(title)
            .with_picture(MessagePicture {
                background: side_background(side),
                foreground: character_portrait(own.dat_id, own.is_major),
            }),
    )
}

/// Character Confronts character (`FUN_0048df20`): `0x71a8` / `0x71a9`
/// naming the Alliance and the Empire character, then for each a line
/// when captive (`+0xac` bit 0: `0x71aa`, `0x71ac`) or injured (`+0x94`:
/// `0x71ab`, `0x71ad`), or `0x71ae` when neither is. The background:
/// `0x41f` (against Vader) or `0x420` when the Alliance character is not
/// Luke; for Luke `0x423` (Vader) or `0x425`, or `0x424` / `0x426` once his
/// `+0xb0` bit 8 (`DagobahCompleted`) is set.
fn confrontation_message(
    a: &rebellion_core::world::Character,
    e: &rebellion_core::world::Character,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    let parameters = [character_parameter(a), character_parameter(e)];
    let title = fill_message_text(texts.get(&0x71a8)?, &parameters);
    let mut body = fill_message_text(texts.get(&0x71a9)?, &parameters);
    let mut told = false;
    for (c, captured, injured) in [(a, 0x71aa, 0x71ab), (e, 0x71ac, 0x71ad)] {
        let line = if c.is_captive {
            Some(captured)
        } else if c.injury != 0 {
            Some(injured)
        } else {
            None
        };
        if let Some(id) = line {
            told = true;
            body.push_str(&fill_message_text(
                texts.get(&id)?,
                &[character_parameter(c)],
            ));
        }
    }
    if !told {
        body.push_str(&fill_message_text(texts.get(&0x71ae)?, &[]));
    }
    let against_vader = e.dat_id.index() == VADER;
    let background = match (a.dat_id.index() == LUKE, a.dagobah_completed, against_vader) {
        (false, _, true) => 0x41f,
        (false, _, false) => 0x420,
        (true, false, true) => 0x423,
        (true, true, true) => 0x424,
        (true, false, false) => 0x425,
        (true, true, false) => 0x426,
    };
    Some(
        GameMessage::new(tick, body, MessageCategory::Event)
            .on_rail(STORY_RAIL, RailAudience::Both)
            .with_title(title)
            .with_picture(MessagePicture {
                background,
                foreground: None,
            }),
    )
}

/// Luke Discovers His Heritage (`FUN_0048dc50`), only for Luke against
/// Vader: `0x71a0` / `0x71a1`, then Luke's line, captured (`0x71a3`),
/// injured (`0x71a4`) or, with Vader free, escaped (`0x71a5`), then
/// Vader's, captured (`0x71a6`) or injured (`0x71a7`), on background
/// `0x422`.
///
/// A captured Luke's line is `0x71a2` (taken to the Emperor) when
/// `taken_to_emperor` holds, else `0x71a3`.
fn heritage_message(
    luke: &rebellion_core::world::Character,
    vader: &rebellion_core::world::Character,
    taken_to_emperor: bool,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    if luke.dat_id.index() != LUKE || vader.dat_id.index() != VADER {
        return None;
    }
    let title = fill_message_text(texts.get(&0x71a0)?, &[]);
    let mut body = fill_message_text(texts.get(&0x71a1)?, &[]);
    let luke_line = if luke.is_captive {
        Some(if taken_to_emperor { 0x71a2 } else { 0x71a3 })
    } else if luke.injury != 0 {
        Some(0x71a4)
    } else if !vader.is_captive {
        Some(0x71a5)
    } else {
        None
    };
    let vader_line = if vader.is_captive {
        Some(0x71a6)
    } else if vader.injury != 0 {
        Some(0x71a7)
    } else {
        None
    };
    for id in luke_line.into_iter().chain(vader_line) {
        body.push_str(&fill_message_text(texts.get(&id)?, &[]));
    }
    Some(
        GameMessage::new(tick, body, MessageCategory::Event)
            .on_rail(STORY_RAIL, RailAudience::Both)
            .with_title(title)
            .with_picture(MessagePicture {
                background: 0x422,
                foreground: None,
            }),
    )
}

/// `FUN_0048dc50`'s test for `0x71a2` (disassembly `0x0048dda0..`):
/// Luke and Vader hold the same mission key (`+0x68`, `FUN_0042d170`), and
/// that mission's target container (`+0x74`) is the Emperor's container
/// (`[Emperor + 0x1c]`, `FUN_004025b0`); the Emperor must exist. The
/// Final Battle's `DarthToEmperor` Move order is such a mission
/// (`final_battle.rs`).
fn taken_to_emperor(
    world: &GameWorld,
    missions: &rebellion_core::missions::MissionState,
    luke: CharacterKey,
    vader: CharacterKey,
) -> bool {
    use rebellion_core::missions::MissionMember;
    let Some(emperor) = world
        .characters
        .values()
        .find(|c| c.dat_id.index() == EMPEROR && !c.is_killed)
    else {
        return false;
    };
    let emperor_system = emperor.current_system.or_else(|| {
        emperor
            .current_fleet
            .and_then(|fleet| world.fleets.get(fleet))
            .map(|fleet| fleet.location)
    });
    missions.missions().iter().any(|mission| {
        let holds = |key| mission.members().any(|m| m == MissionMember::Character(key));
        let container = match (mission.target_abode, emperor.current_abode) {
            (Some(abode), Some(at)) => abode == at,
            (Some(_), None) | (None, Some(_)) => false,
            (None, None) => Some(mission.target_system) == emperor_system,
        };
        holds(luke) && holds(vader) && container
    })
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
    /// Any other named object, such as a system: kind 1 its name.
    Named { name: &'a str },
}

impl MessageParameter<'_> {
    /// The text a substitution of `kind` takes from this parameter.
    #[must_use]
    pub fn text(&self, kind: u8) -> Option<&str> {
        match (self, kind) {
            (Self::Character { name, .. }, 1) => Some(name),
            (Self::Character { force_ranking, .. }, 7) => Some(force_ranking),
            (Self::Named { name }, 1) => Some(name),
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
        // original's ids; the port's 0x363 is gone with jedi.rs (Leia's
        // discovery is the trainer's 0x362); the port's 0x387 is gone (save
        // v77), and the Final Battle
        // (0x220) and the gates have no class.
        let (mut world, character) = (GameWorld::default(), CharacterKey::default());
        let capture = [EventAction::CaptureCharacter {
            character,
            captor_faction: rebellion_core::dat::Faction::Empire,
        }];
        assert_eq!(story_class(0x221), Some(StoryClass::LukeGoesToDagobah));
        assert_eq!(story_class(0x1e1), Some(StoryClass::ForceGrowth));
        for unmapped in [0x220, 0x200, 0x212, 0x362, 0x363, 0x387, 0x399] {
            assert_eq!(story_class(unmapped), None, "{unmapped:#x}");
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
        // The Force Growth class (0x1e1): TEXTSTRA 0x7180 / 0x7181 over the
        // side's background with the subject's portrait, filed under bit
        // 0x10.
        let mut world = GameWorld::default();
        let leia = world.characters.insert(rebellion_core::world::Character {
            dat_id: DatId::new(577),
            name: "Leia Organa".into(),
            is_major: true,
            ..Default::default()
        });
        let texts = HashMap::from([
            (0x7180, "Force Growth".to_owned()),
            (0x7181, "The Force grows stronger.".to_owned()),
        ]);
        let actions = [EventAction::CaptureCharacter {
            character: leia,
            captor_faction: rebellion_core::dat::Faction::Empire,
        }];
        let message = story_message(
            &world,
            0x1e1,
            &actions,
            "port text".into(),
            40,
            CockpitFaction::Empire,
            &texts,
            MessageCategory::Event,
        );
        assert_eq!(message.title.as_deref(), Some("Force Growth"));
        assert_eq!(message.text, "The Force grows stronger.");
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

    /// TEXTSTRA 0x71a0..0x71b1, the encounter class's texts.
    fn encounter_texts() -> HashMap<u16, String> {
        [
            (0x71a0, "Luke Discovers His Heritage"),
            (0x71a1, "Luke has learned that Vader is his father.  "),
            (
                0x71a2,
                "Luke has been captured by Vader and taken to the Emperor.  ",
            ),
            (0x71a3, "Luke has been captured by Vader.  "),
            (0x71a4, "Luke was injured during the battle.  "),
            (0x71a5, "Luke escaped capture after the battle."),
            (0x71a6, "Darth Vader was captured by Luke."),
            (0x71a7, "Vader was injured during the battle."),
            (0x71a8, "{1:1} Confronts {2:1}"),
            (0x71a9, "{1:1} has fought {2:1}.  "),
            (0x71aa, "{1:1} was captured.  "),
            (0x71ab, "{1:1} was injured.  "),
            (0x71ac, "{1:1} was captured."),
            (0x71ad, "{1:1} was injured."),
            (0x71ae, "Both combatants escaped uninjured."),
            (0x71b0, "{1:1} Detects Enemy"),
            (
                0x71b1,
                "{1:1} has detected {2:1} because of a disturbance in the Force.",
            ),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect()
    }

    fn encounter_cast(world: &mut GameWorld) -> [CharacterKey; 5] {
        let mut add = |raw: u32, name: &str, is_alliance: bool, is_major: bool| {
            world.characters.insert(rebellion_core::world::Character {
                dat_id: DatId::new(raw),
                name: name.into(),
                is_alliance,
                is_empire: !is_alliance,
                is_major,
                ..Default::default()
            })
        };
        [
            add(0x242, "Luke Skywalker", true, true),
            add(0x241, "Leia Organa", true, true),
            add(0x281, "Darth Vader", false, true),
            add(0x280, "Emperor Palpatine", false, true),
            add(0x340, "Bria Tharen", true, false),
        ]
    }

    fn met(
        member: CharacterKey,
        defender: CharacterKey,
    ) -> Vec<rebellion_core::missions::MissionEffect> {
        use rebellion_core::missions::MissionEffect;
        vec![
            MissionEffect::EncounterBegan {
                member,
                defender,
                day: 3,
            },
            MissionEffect::EncounterEnded {
                member,
                partner: defender,
            },
        ]
    }

    #[test]
    fn luke_learning_his_heritage_from_vader_posts_the_heritage_message_to_both_sides() {
        // FUN_0048d7f0: bit 0 (0x211) and the Luke-Vader pair build
        // FUN_0048dc50: 0x71a0 / 0x71a1, then Luke escaped (0x71a5) with
        // both free and uninjured, on background 0x422.
        let mut world = GameWorld::default();
        let [luke, _, vader, _, _] = encounter_cast(&mut world);
        let mut effects = met(luke, vader);
        effects.push(rebellion_core::missions::MissionEffect::HeritageLearned { character: luke });

        let messages =
            EncounterNotes::gather(&world, &effects).messages(&world, &rebellion_core::missions::MissionState::new(), 7, &encounter_texts());

        assert_eq!(messages.len(), 1);
        let message = &messages[0];
        assert_eq!(
            message.title.as_deref(),
            Some("Luke Discovers His Heritage")
        );
        assert_eq!(
            message.text,
            "Luke has learned that Vader is his father.  Luke escaped capture after the battle."
        );
        assert_eq!(message.audience, RailAudience::Both);
        assert_eq!(message.picture.expect("a picture").background, 0x422);
        assert_eq!(message.tick, 7);
    }

    #[test]
    fn a_captive_luke_on_vader_s_mission_to_the_emperor_is_taken_to_him() {
        // FUN_0048dc50 (0x0048dda0..0x0048de28): Luke and Vader hold the
        // same mission key (+0x68) whose target container (+0x74) is the
        // Emperor's container ([Emperor + 0x1c]): 0x71a2 for 0x71a3. The
        // Final Battle's DarthToEmperor Move order is one.
        use rebellion_core::missions::{
            MissionFaction, MissionKind, MissionMember, MissionRequest, MissionState,
        };
        let mut world = GameWorld::default();
        let [luke, leia, vader, emperor, _] = encounter_cast(&mut world);
        let mut place = |name: &str| {
            world.systems.insert(rebellion_core::world::System {
                dat_id: DatId::new(0x9000_0001),
                name: name.into(),
                sector: rebellion_core::ids::SectorKey::default(),
                x: 0,
                y: 0,
                exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
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
                control: rebellion_core::world::ControlKind::Uncontrolled,
            })
        };
        let (coruscant, elsewhere) = (place("Coruscant"), place("Yavin"));
        world.characters[emperor].current_system = Some(coruscant);
        world.characters[luke].is_captive = true;
        let mut effects = met(luke, vader);
        effects.push(rebellion_core::missions::MissionEffect::HeritageLearned { character: luke });
        let texts = encounter_texts();
        let line = |missions: &MissionState, world: &GameWorld| {
            EncounterNotes::gather(world, &effects).messages(world, missions, 0, &texts)[0]
                .text
                .clone()
        };
        let mut missions = MissionState::new();
        let order = |missions: &mut MissionState, team: Vec<MissionMember>, to| {
            missions.dispatch(MissionRequest {
                kind: MissionKind::Move,
                faction: MissionFaction::Empire,
                team,
                decoys: Vec::new(),
                target_system: to,
                target_character: None,
                target_object: None,
                tick: 0,
            });
        };
        let both = vec![
            MissionMember::Character(vader),
            MissionMember::Character(luke),
        ];
        // Elsewhere, without Luke or without Vader: captured only.
        order(&mut missions, both.clone(), elsewhere);
        order(&mut missions, vec![MissionMember::Character(vader)], coruscant);
        order(
            &mut missions,
            vec![
                MissionMember::Character(luke),
                MissionMember::Character(leia),
            ],
            coruscant,
        );
        assert!(line(&missions, &world).contains("Luke has been captured by Vader.  "));
        // To the Emperor's place with both aboard: taken to the Emperor.
        order(&mut missions, both, coruscant);
        assert!(line(&missions, &world)
            .contains("Luke has been captured by Vader and taken to the Emperor.  "));
        // An Emperor in an abode: the mission's abode is his container.
        let palace = world.abodes.insert(rebellion_core::world::Abode {
            dat_id: DatId::new(0x481),
            name: "Palace".into(),
            side: rebellion_core::dat::Faction::Neutral,
            special_forces: Vec::new(),
        });
        let lair = world.abodes.insert(rebellion_core::world::Abode {
            dat_id: DatId::new(0x482),
            name: "Lair".into(),
            side: rebellion_core::dat::Faction::Neutral,
            special_forces: Vec::new(),
        });
        world.characters[emperor].current_system = None;
        world.characters[emperor].current_abode = Some(palace);
        let mut abode_missions = MissionState::new();
        order(
            &mut abode_missions,
            vec![
                MissionMember::Character(vader),
                MissionMember::Character(luke),
            ],
            coruscant,
        );
        let id = abode_missions.missions()[0].id;
        abode_missions.aim_at_abode(id, lair);
        assert!(line(&abode_missions, &world).contains("Luke has been captured by Vader.  "));
        abode_missions.aim_at_abode(id, palace);
        assert!(line(&abode_missions, &world)
            .contains("Luke has been captured by Vader and taken to the Emperor.  "));
        // A dead Emperor takes no one.
        world.characters[emperor].is_killed = true;
        assert!(line(&missions, &world).contains("Luke has been captured by Vader.  "));
    }

    #[test]
    fn the_heritage_message_tells_luke_s_capture_and_vader_s_injury() {
        // FUN_0048dc50: Luke captive (+0xac bit 0) -> 0x71a3; Vader injured
        // (+0x94) -> 0x71a7; an injured, free Luke -> 0x71a4 and no escape.
        let mut world = GameWorld::default();
        let [luke, _, vader, _, _] = encounter_cast(&mut world);
        world.characters[luke].is_captive = true;
        world.characters[vader].injury = 20;
        let mut effects = met(luke, vader);
        effects.push(rebellion_core::missions::MissionEffect::HeritageLearned { character: luke });
        let texts = encounter_texts();

        let captured = EncounterNotes::gather(&world, &effects).messages(&world, &rebellion_core::missions::MissionState::new(), 0, &texts);
        assert_eq!(
            captured[0].text,
            "Luke has learned that Vader is his father.  Luke has been captured by Vader.  \
             Vader was injured during the battle."
        );

        world.characters[luke].is_captive = false;
        world.characters[luke].injury = 5;
        world.characters[vader].injury = 0;
        world.characters[vader].is_captive = true;
        let injured = EncounterNotes::gather(&world, &effects).messages(&world, &rebellion_core::missions::MissionState::new(), 0, &texts);
        assert_eq!(
            injured[0].text,
            "Luke has learned that Vader is his father.  Luke was injured during the battle.  \
             Darth Vader was captured by Luke."
        );
    }

    #[test]
    fn a_jedi_pair_without_the_heritage_bit_posts_the_confrontation() {
        // FUN_0048df20: 0x71a8 / 0x71a9 naming the Alliance and the Empire
        // character, a line per captive or injured one, else 0x71ae.
        let mut world = GameWorld::default();
        let [luke, _, vader, _, _] = encounter_cast(&mut world);
        let texts = encounter_texts();

        let calm = EncounterNotes::gather(&world, &met(luke, vader)).messages(&world, &rebellion_core::missions::MissionState::new(), 0, &texts);
        assert_eq!(calm.len(), 1);
        assert_eq!(
            calm[0].title.as_deref(),
            Some("Luke Skywalker Confronts Darth Vader")
        );
        assert_eq!(
            calm[0].text,
            "Luke Skywalker has fought Darth Vader.  Both combatants escaped uninjured."
        );
        assert_eq!(calm[0].audience, RailAudience::Both);

        world.characters[luke].injury = 9;
        world.characters[vader].is_captive = true;
        // The member's order does not matter: each side files its own.
        let hurt = EncounterNotes::gather(&world, &met(vader, luke)).messages(&world, &rebellion_core::missions::MissionState::new(), 0, &texts);
        assert_eq!(
            hurt[0].text,
            "Luke Skywalker has fought Darth Vader.  Luke Skywalker was injured.  \
             Darth Vader was captured."
        );
    }

    #[test]
    fn the_confrontation_background_follows_the_pair_and_luke_s_dagobah() {
        // FUN_0048df20: 0x41f / 0x420 for a non-Luke Alliance character
        // (Vader or not); Luke 0x423 / 0x425, or 0x424 / 0x426 once +0xb0
        // bit 8 (DagobahCompleted) is set.
        let mut world = GameWorld::default();
        let [luke, leia, vader, emperor, _] = encounter_cast(&mut world);
        let texts = encounter_texts();
        let background = |world: &GameWorld, a, e| {
            EncounterNotes::gather(world, &met(a, e)).messages(world, &rebellion_core::missions::MissionState::new(), 0, &texts)[0]
                .picture
                .expect("a picture")
                .background
        };
        assert_eq!(background(&world, leia, vader), 0x41f);
        assert_eq!(background(&world, leia, emperor), 0x420);
        assert_eq!(background(&world, luke, vader), 0x423);
        assert_eq!(background(&world, luke, emperor), 0x425);
        world.characters[luke].dagobah_completed = true;
        assert_eq!(background(&world, luke, vader), 0x424);
        assert_eq!(background(&world, luke, emperor), 0x426);
    }

    #[test]
    fn any_other_pair_posts_force_user_detects_enemy_to_each_side() {
        // FUN_0048e1d0: each side's own character, on (side != 1) + 0x412
        // with its portrait, has detected the other (0x71b0 / 0x71b1).
        let mut world = GameWorld::default();
        let [_, _, vader, _, minor] = encounter_cast(&mut world);

        let messages = EncounterNotes::gather(&world, &met(minor, vader)).messages(
            &world,
            &rebellion_core::missions::MissionState::new(),
            0,
            &encounter_texts(),
        );

        assert_eq!(messages.len(), 2);
        let alliance = &messages[0];
        assert_eq!(alliance.audience, RailAudience::Alliance);
        assert_eq!(alliance.title.as_deref(), Some("Bria Tharen Detects Enemy"));
        assert_eq!(
            alliance.text,
            "Bria Tharen has detected Darth Vader because of a disturbance in the Force."
        );
        let picture = alliance.picture.expect("a picture");
        assert_eq!(picture.background, 0x412);
        assert_eq!(
            picture.foreground,
            character_portrait(DatId::new(0x340), false)
        );
        let empire = &messages[1];
        assert_eq!(empire.audience, RailAudience::Empire);
        assert_eq!(empire.title.as_deref(), Some("Darth Vader Detects Enemy"));
        let picture = empire.picture.expect("a picture");
        assert_eq!(picture.background, 0x413);
        assert_eq!(
            picture.foreground,
            character_portrait(DatId::new(0x281), true)
        );
    }

    #[test]
    fn an_encounter_begun_without_a_confrontation_still_posts() {
        // The keys' setter notifies 0x360 when Force detection sets them
        // (FUN_0055ff60), so a decoy's encounter posts on the day it
        // begins.
        let mut world = GameWorld::default();
        let [luke, _, vader, _, minor] = encounter_cast(&mut world);
        let texts = encounter_texts();
        let begun = |member, defender| {
            [rebellion_core::missions::MissionEffect::EncounterBegan {
                member,
                defender,
                day: 3,
            }]
        };
        let detected =
            EncounterNotes::gather(&world, &begun(minor, vader)).messages(&world, &rebellion_core::missions::MissionState::new(), 0, &texts);
        assert_eq!(detected.len(), 2);
        let confronted =
            EncounterNotes::gather(&world, &begun(luke, vader)).messages(&world, &rebellion_core::missions::MissionState::new(), 0, &texts);
        assert_eq!(
            confronted[0].title.as_deref(),
            Some("Luke Skywalker Confronts Darth Vader")
        );
        // And the confrontation's clearing (FUN_00560200) notifies alone on
        // a later day.
        let ended = [rebellion_core::missions::MissionEffect::EncounterEnded {
            member: luke,
            partner: vader,
        }];
        let later = EncounterNotes::gather(&world, &ended).messages(&world, &rebellion_core::missions::MissionState::new(), 0, &texts);
        assert_eq!(later.len(), 1);
    }

    #[test]
    fn the_final_battle_message_tells_luke_s_fate() {
        // FUN_0048e3c0: 0x71e0, then 0x71e1 with Luke free or 0x71e2 with
        // him captured (+0xac bit 0); background 0x428 once DagobahCompleted
        // (+0xb0 bit 8) is set, else 0x427.
        let mut world = GameWorld::default();
        let [luke, ..] = encounter_cast(&mut world);
        let texts: HashMap<u16, String> = [
            (0x71e0, "The Final Battle"),
            (0x71e1, "Luke confronted and overcame the Emperor."),
            (0x71e2, "The Emperor confronted Luke and captured him."),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();

        let won = final_battle_message(&world, luke, 4, &texts).expect("a message");
        assert_eq!(won.title.as_deref(), Some("The Final Battle"));
        assert_eq!(won.text, "Luke confronted and overcame the Emperor.");
        assert_eq!(won.audience, RailAudience::Both);
        assert_eq!(won.picture.expect("a picture").background, 0x427);

        world.characters[luke].is_captive = true;
        world.characters[luke].dagobah_completed = true;
        let lost = final_battle_message(&world, luke, 4, &texts).expect("a message");
        assert_eq!(lost.text, "The Emperor confronted Luke and captured him.");
        assert_eq!(lost.picture.expect("a picture").background, 0x428);
    }

    #[test]
    fn a_traitor_discovery_names_both_and_the_other_side_for_the_discoverer_s_side() {
        // FUN_0048c9c0: 0x71e8 with the discoverer (1), 0x71e9 with the
        // traitor (2) and the other side (3); the discoverer's side,
        // background (side != 1) + 0x412, the discoverer's portrait.
        let texts: HashMap<u16, String> = [
            (0x71e8, "{1:1} Discovers Traitor"),
            (0x71e9, "Through the use of the Force, I have discovered that {2:1} has betrayed us to the {3:1}."),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let luke = rebellion_core::world::Character {
            name: "Luke Skywalker".into(),
            dat_id: DatId::new(0x242),
            is_major: true,
            is_alliance: true,
            ..Default::default()
        };
        let traitor = rebellion_core::world::Character {
            name: "Bria Tharen".into(),
            is_alliance: true,
            ..Default::default()
        };
        let message = discovers_traitor_message(&luke, &traitor, 6, &texts).expect("message");
        assert_eq!(
            message.title.as_deref(),
            Some("Luke Skywalker Discovers Traitor")
        );
        assert_eq!(
            message.text,
            "Through the use of the Force, I have discovered that Bria Tharen has betrayed us to the Empire."
        );
        assert_eq!(message.audience, RailAudience::Alliance);
        let picture = message.picture.expect("a picture");
        assert_eq!(picture.background, 0x412);
        assert_eq!(picture.foreground, character_portrait(luke.dat_id, true));
        let vader = rebellion_core::world::Character {
            name: "Darth Vader".into(),
            is_empire: true,
            ..Default::default()
        };
        let message = discovers_traitor_message(&vader, &traitor, 6, &texts).expect("message");
        assert_eq!(message.audience, RailAudience::Empire);
        assert!(message.text.ends_with("to the Alliance."));
        assert_eq!(message.picture.map(|p| p.background), Some(0x413));
        assert!(discovers_traitor_message(&luke, &traitor, 6, &HashMap::new()).is_none());
    }

    #[test]
    fn a_rescue_attempt_names_the_rescuer_for_the_alliance_with_his_portrait() {
        // FUN_0048c4c0: 0x71ea / 0x71eb with the rescuer, background 0x412
        // and the portrait (FUN_004c5000), the Alliance only.
        let texts: HashMap<u16, String> = [
            (0x71ea, "{1:1} Rescue Attempt"),
            (0x71eb, "Han Solo has been captured by bounty hunters and taken to Jabba's palace.  I will be departing immediately to attempt a rescue."),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let luke = rebellion_core::world::Character {
            name: "Luke Skywalker".into(),
            dat_id: DatId::new(0x242),
            is_major: true,
            ..Default::default()
        };
        let message = rescue_attempt_message(&luke, 5, &texts).expect("message");
        assert_eq!(
            message.title.as_deref(),
            Some("Luke Skywalker Rescue Attempt")
        );
        assert!(message.text.starts_with("Han Solo has been captured"));
        assert_eq!(message.audience, RailAudience::Alliance);
        let picture = message.picture.expect("a picture");
        assert_eq!(picture.background, 0x412);
        assert_eq!(picture.foreground, character_portrait(luke.dat_id, true));
        assert!(picture.foreground.is_some());
        assert!(rescue_attempt_message(&luke, 5, &HashMap::new()).is_none());
    }

    #[test]
    fn a_victory_conditions_change_posts_the_hq_and_coruscant_messages() {
        // FUN_0048e5a0 on bit 0x1, FUN_0048f790 while Coruscant is the
        // Alliance's (bit 0x100); both sides read them.
        let texts: HashMap<u16, String> = [
            (0x7188, "Alliance Headquarters Destroyed"),
            (
                0x7189,
                "The Empire has destroyed the Alliance Headquarters.",
            ),
            (0x7170, "Coruscant Captured"),
            (
                0x7171,
                "The Alliance has seized the Imperial capital system of Coruscant.",
            ),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let titles = |word| {
            victory_conditions_messages(word, 4, &texts)
                .into_iter()
                .map(|m| (m.title.unwrap_or_default(), m.audience))
                .collect::<Vec<_>>()
        };
        assert!(titles(0x10).is_empty());
        assert_eq!(
            titles(0x1),
            [(
                "Alliance Headquarters Destroyed".to_owned(),
                RailAudience::Both
            )]
        );
        assert_eq!(
            titles(0x100),
            [("Coruscant Captured".to_owned(), RailAudience::Both)]
        );
        assert_eq!(titles(0x101).len(), 2);
        assert!(victory_conditions_messages(0x101, 4, &HashMap::new()).is_empty());
    }

    #[test]
    fn the_emperor_arrives_at_coruscant_for_the_empire_on_its_picture() {
        // FUN_0048ce80: 0x71c8 / 0x71c9, background 0x42c, Empire only.
        let texts: HashMap<u16, String> = [
            (0x71c8, "Emperor Arrives at Coruscant"),
            (0x71c9, "I have returned to the Seat of Power at Coruscant."),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let message = emperor_arrives_message(3, &texts).expect("message");
        assert_eq!(
            message.title.as_deref(),
            Some("Emperor Arrives at Coruscant")
        );
        assert_eq!(
            message.text,
            "I have returned to the Seat of Power at Coruscant."
        );
        assert_eq!(message.audience, RailAudience::Empire);
        assert_eq!(message.picture.map(|p| p.background), Some(0x42c));
        assert!(emperor_arrives_message(3, &HashMap::new()).is_none());
    }

    #[test]
    fn the_assassins_and_death_star_sabotage_messages_name_their_places() {
        // FUN_0048b050: 0x7208 / 0x7209 with the victim (1) and the system
        // (3), background 0x42e, the Alliance only. FUN_0048d470: 0x71bd /
        // 0x71be with the system (1), both sides.
        let texts: HashMap<u16, String> = [
            (0x7208, "{1:1} Killed"),
            (0x7209, "{1:1} was killed by Imperial Assassins at {3:1}."),
            (0x71bd, "Death Star Sabotaged"),
            (
                0x71be,
                "The Rebel Alliance has sabotaged the Death Star at {1:1}.",
            ),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let dodonna = rebellion_core::world::Character {
            name: "Jan Dodonna".into(),
            ..Default::default()
        };
        let killed = assassins_message(&dodonna, "Yavin", 7, &texts).expect("message");
        assert_eq!(killed.title.as_deref(), Some("Jan Dodonna Killed"));
        assert_eq!(
            killed.text,
            "Jan Dodonna was killed by Imperial Assassins at Yavin."
        );
        assert_eq!(killed.audience, RailAudience::Alliance);
        assert_eq!(killed.picture.map(|p| p.background), Some(0x42e));
        let sabotaged = death_star_sabotaged_message("Endor", 8, &texts).expect("message");
        assert_eq!(sabotaged.title.as_deref(), Some("Death Star Sabotaged"));
        assert_eq!(
            sabotaged.text,
            "The Rebel Alliance has sabotaged the Death Star at Endor."
        );
        assert_eq!(sabotaged.audience, RailAudience::Both);
        assert!(assassins_message(&dodonna, "Yavin", 7, &HashMap::new()).is_none());
        assert!(death_star_sabotaged_message("Endor", 8, &HashMap::new()).is_none());
    }

    #[test]
    fn jabba_delivers_prisoners_reads_its_two_texts_on_its_background() {
        // FUN_0048b6e0: TEXTSTRA 0x7200 / 0x7201 with no arguments, on
        // background 0x42a.
        let texts: HashMap<u16, String> = [
            (0x7200, "Jabba Delivers Prisoners"),
            (
                0x7201,
                "Jabba has deliverd Luke, Han, Chewbacca and Leia to Darth Vader.",
            ),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let message = jabba_delivers_message(4, &texts).expect("message");
        assert_eq!(message.title.as_deref(), Some("Jabba Delivers Prisoners"));
        assert_eq!(
            message.text,
            "Jabba has deliverd Luke, Han, Chewbacca and Leia to Darth Vader."
        );
        assert_eq!(message.picture.map(|p| p.background), Some(0x42a));
        assert_eq!(message.audience, RailAudience::Both);
        assert!(jabba_delivers_message(4, &HashMap::new()).is_none());
    }

    #[test]
    fn jabba_captures_names_the_failed_rescuer_on_the_palace_picture() {
        // FUN_00490340 case 5: TEXTSTRA 0x7156 / 0x7157 with the
        // character's name (+0x30), background 0x429.
        let chewbacca = rebellion_core::world::Character {
            name: "Chewbacca".into(),
            ..Default::default()
        };
        let texts: HashMap<u16, String> = [
            (0x7156, "Jabba Captures {1:1}"),
            (
                0x7157,
                "{1:1} was captured by Jabba while attempting to rescue Han Solo.",
            ),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let message = jabba_captures_message(&chewbacca, 9, &texts).expect("message");
        assert_eq!(message.title.as_deref(), Some("Jabba Captures Chewbacca"));
        assert_eq!(
            message.text,
            "Chewbacca was captured by Jabba while attempting to rescue Han Solo."
        );
        assert_eq!(message.picture.map(|p| p.background), Some(0x429));
        assert_eq!(message.tick, 9);
        assert!(jabba_captures_message(&chewbacca, 9, &HashMap::new()).is_none());
    }

    #[test]
    fn the_bounty_attack_messages_follow_han_s_capture() {
        // FUN_0048fa00 on 0x212: captured, both read 0x716c/0x716d on
        // 0x41d; otherwise the Alliance reads 0x7168/0x7169 on 0x41c and the
        // Empire 0x716a/0x716b with Han's system on 0x41e.
        let mut world = GameWorld::default();
        let tatooine = world.systems.insert(rebellion_core::world::System {
            dat_id: DatId::new(0x9000_0001),
            name: "Tatooine".into(),
            sector: rebellion_core::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
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
            control: rebellion_core::world::ControlKind::Uncontrolled,
        });
        let han = world.characters.insert(rebellion_core::world::Character {
            dat_id: DatId::new(0x243),
            name: "Han Solo".into(),
            is_alliance: true,
            current_system: Some(tatooine),
            ..Default::default()
        });
        let texts: HashMap<u16, String> = [
            (0x7168, "Han Solo Attacked by Bounty Hunters"),
            (
                0x7169,
                "Han Solo was attacked by bounty hunters, who failed to capture him.",
            ),
            (0x716a, "Bounty Hunters Locate Solo"),
            (
                0x716b,
                "Bounty hunters have reported that Han Solo is located on {1:1}.",
            ),
            (0x716c, "Han Solo Captured by Bounty Hunters"),
            (
                0x716d,
                "Han Solo was captured by bounty hunters and taken to Jabba's Palace.",
            ),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();

        let attacked = bounty_messages(&world, han, 3, &texts);
        assert_eq!(attacked.len(), 2);
        assert_eq!(attacked[0].audience, RailAudience::Alliance);
        assert_eq!(
            attacked[0].title.as_deref(),
            Some("Han Solo Attacked by Bounty Hunters")
        );
        assert_eq!(attacked[0].picture.expect("a picture").background, 0x41c);
        assert_eq!(attacked[1].audience, RailAudience::Empire);
        assert_eq!(
            attacked[1].text,
            "Bounty hunters have reported that Han Solo is located on Tatooine."
        );
        assert_eq!(attacked[1].picture.expect("a picture").background, 0x41e);

        world.characters[han].is_captive = true;
        let captured = bounty_messages(&world, han, 3, &texts);
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].audience, RailAudience::Both);
        assert_eq!(captured[0].picture.expect("a picture").background, 0x41d);
    }

    #[test]
    fn only_luke_and_vader_have_a_heritage_message() {
        // FUN_0048dc50 shows nothing (+0x40 = 0) unless +0x68 is Luke and
        // +0x6c Vader, even with bit 0 set.
        let mut world = GameWorld::default();
        let [luke, leia, vader, emperor, _] = encounter_cast(&mut world);
        let texts = encounter_texts();
        for (a, e) in [(leia, vader), (luke, emperor)] {
            let mut effects = met(a, e);
            effects
                .push(rebellion_core::missions::MissionEffect::HeritageLearned { character: luke });
            assert!(EncounterNotes::gather(&world, &effects)
                .messages(&world, &rebellion_core::missions::MissionState::new(), 0, &texts)
                .is_empty());
        }
    }

    #[test]
    fn a_neutral_or_missing_side_shows_no_encounter_message() {
        // FUN_0048d9a0 sets +0x64 bit 0x10000000 for a character of neither
        // side; a builder with only one side's character shows nothing.
        let mut world = GameWorld::default();
        let [luke, _, vader, _, _] = encounter_cast(&mut world);
        let texts = encounter_texts();
        assert!(EncounterNotes::gather(&world, &[])
            .messages(&world, &rebellion_core::missions::MissionState::new(), 0, &texts)
            .is_empty());
        world.characters[vader].is_empire = false;
        assert!(EncounterNotes::gather(&world, &met(luke, vader))
            .messages(&world, &rebellion_core::missions::MissionState::new(), 0, &texts)
            .is_empty());
        let lone = [rebellion_core::missions::MissionEffect::HeritageLearned { character: luke }];
        assert!(EncounterNotes::gather(&world, &lone)
            .messages(&world, &rebellion_core::missions::MissionState::new(), 0, &texts)
            .is_empty());
    }
}
