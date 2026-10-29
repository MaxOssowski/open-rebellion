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
    // The classes' +0x28 (advisor-briefing.md, "The events"): Luke's
    // Dagobah 0x16 (FUN_0048d190), the speaker map for Force Growth and
    // Leia (FUN_0048ed80, FUN_0048f1e0), the capture's for Jabba's
    // (FUN_00490340).
    let event = match class {
        StoryClass::LukeGoesToDagobah => 0x16,
        StoryClass::ForceGrowth | StoryClass::LeiaUsesForce => character_speaker_event(subject),
        StoryClass::JabbaCaptures => subject.map_or(0x15, captured_event),
    };
    message
        .with_picture(class.picture(
            side,
            subject.map(|character| (character.dat_id, character.is_major)),
        ))
        .with_advisor_event(event)
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
            .with_picture(StoryClass::LukeGoesToDagobah.picture(CockpitFaction::Alliance, None))
            // FUN_0048d190 writes 0x16.
            .with_advisor_event(0x16),
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
            })
            // FUN_0048d0f0 writes 0x16.
            .with_advisor_event(0x16),
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
            })
            // FUN_0048f1e0's speaker map on the same character.
            .with_advisor_event(character_speaker_event(Some(speaker))),
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
        .with_picture(class.picture(CockpitFaction::Alliance, None))
        .with_advisor_event(captured_event(character)),
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
        })
        // FUN_0048c9c0's speaker map on the discoverer.
        .with_advisor_event(character_speaker_event(Some(discoverer))),
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
        })
        // FUN_0048c4c0's speaker map on the rescuer (+0x68).
        .with_advisor_event(character_speaker_event(Some(rescuer))),
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
            })
            // FUN_0048ce80 writes 0x1a.
            .with_advisor_event(0x1a),
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
        })
        // FUN_0048b050 writes 0x15.
        .with_advisor_event(0x15),
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

/// Smuggling (`FUN_00499130`): notification `0x145`, the system's smuggling
/// percent going from `previous` to `percent`, for the viewer's side. It
/// speaks only when one of them is 0: smuggling begins (`percent` not 0) or
/// ends. The system must be held by a side (`+0x24` bits 6..7): on the
/// viewer's own system it reports losses (TEXTSTRA `0x7060`/`0x7064` on
/// beginning, `0x7061`/`0x7065` on ending), on the other side's benefits
/// (`0x7062`/`0x7066`, `0x7063`/`0x7067`). An Alliance system speaks when
/// `previous` is at least 0, an Empire system when it is below 1. Its
/// category mask is `0x001`, Popular Support (`FUN_00499030`,
/// `FUN_0040f340`); its background is `0x3ec`, the beginning's picture
/// `0x44f`.
///
/// port: the pictures are not drawn.
#[must_use]
pub fn smuggling_message(
    system: &str,
    holder: Option<rebellion_core::dat::Faction>,
    viewer_is_alliance: bool,
    previous: i32,
    percent: i32,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    use rebellion_core::dat::Faction;
    if previous != 0 && percent != 0 {
        return None;
    }
    let (own, speaks) = match holder? {
        Faction::Alliance => (viewer_is_alliance, previous >= 0),
        Faction::Empire => (!viewer_is_alliance, previous < 1),
        Faction::Neutral => return None,
    };
    if !speaks {
        return None;
    }
    let (title, body) = match (percent != 0, own) {
        (true, true) => (0x7060, 0x7064),
        (true, false) => (0x7062, 0x7066),
        (false, true) => (0x7061, 0x7065),
        (false, false) => (0x7063, 0x7067),
    };
    let parameters = [MessageParameter::Named { name: system }];
    Some(
        GameMessage::new(
            tick,
            fill_message_text(texts.get(&body)?, &parameters),
            MessageCategory::Event,
        )
        .on_rail(
            crate::message_log::MessageRail::PopularSupport,
            RailAudience::side(viewer_is_alliance),
        )
        .with_title(fill_message_text(texts.get(&title)?, &parameters)),
    )
}

/// A side as a substitution: the Side's text provider (`FUN_0052d920`)
/// reads TEXTSTRA strings `0x4d50` / `0x4d51` (kind 1), `0x4d60` /
/// `0x4d61` (kind 2) and `0x4d70` / `0x4d71` (kind 4), Alliance then Empire.
#[must_use]
pub fn side_parameter(alliance: bool, texts: &HashMap<u16, String>) -> MessageParameter<'_> {
    let text = |alliance_id: u16| {
        texts
            .get(&if alliance {
                alliance_id
            } else {
                alliance_id + 1
            })
            .map_or("", String::as_str)
    };
    MessageParameter::Side {
        name: text(0x4d50),
        full_name: text(0x4d60),
        adjective: text(0x4d70),
    }
}

/// Uprising Begins / Ends (`FUN_00499460`): notification `0x14b` with the
/// system's uprising bit (`+0x88` bit 2). Both sides' classes speak for a
/// system a side holds (`+0x24` bits 6..7), none for a neutral one. A
/// beginning takes TEXTSTRA `0x7030` / `0x7033` ("An uprising has begun on
/// the {1:4} controlled system {2:1}."), an end `0x7031` / `0x7032`; the
/// body's first substitution is the holder. Returns the title and body.
#[must_use]
pub fn uprising_texts(
    system: &str,
    holder: Option<rebellion_core::dat::Faction>,
    begins: bool,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    use rebellion_core::dat::Faction;
    let alliance = match holder? {
        Faction::Alliance => true,
        Faction::Empire => false,
        Faction::Neutral => return None,
    };
    let (title, body) = if begins {
        (0x7030, 0x7033)
    } else {
        (0x7031, 0x7032)
    };
    let named = MessageParameter::Named { name: system };
    let side = side_parameter(alliance, texts);
    Some((
        fill_message_text(texts.get(&title)?, &[named]),
        fill_message_text(texts.get(&body)?, &[side, named]),
    ))
}

/// Natural Disaster (`FUN_0048e940`): notification `0x154`, then `2`, for
/// the side holding the system only. The title is TEXTSTRA `0x7190`. The
/// body names what was lost: `0x7191` resources only, `0x7192` units only,
/// `0x7193` both; each unit destroyed (a `0x302` of family `0x20`..`0x2f`,
/// `FUN_0048e890`) adds a line `0x719b`. With nothing lost there is no
/// message. Returns the title and body.
///
/// `resources` stands for the class's bits 2 and 4 (`+0x64`): its slot
/// `+0x24` (`FUN_0048e840`) sets bit 4 on `SystemSystemEnergyNotif`
/// (`0x141`) and bit 2 on `SystemSystemRawMaterialNotif` (`0x143`), and bit
/// 1 when the value rose. A disaster only lowers them.
#[must_use]
pub fn disaster_texts(
    system: &str,
    resources: bool,
    units: &[String],
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    let body = match (units.is_empty(), resources) {
        (true, false) => return None,
        (true, true) => 0x7191,
        (false, false) => 0x7192,
        (false, true) => 0x7193,
    };
    let named = [MessageParameter::Named { name: system }];
    let mut text = fill_message_text(texts.get(&body)?, &named);
    let line = texts.get(&0x719b)?;
    for unit in units {
        text.push_str(&fill_message_text(
            line,
            &[MessageParameter::Named { name: unit }],
        ));
    }
    Some((fill_message_text(texts.get(&0x7190)?, &named), text))
}

/// New Resources / Resources Depleted (`FUN_0048e940`): notification
/// `0x155`, then `2`, for the holder's side. `code` is the RESRCTB outcome
/// (1 raw materials depleted, 2 energy depleted, 3 new raw materials, 4 new
/// energy). A gain takes TEXTSTRA `0x7194` with `0x7199` (energy) or
/// `0x719a` (raw materials), a loss `0x7196` with `0x7197` or `0x7198`.
/// Returns the title and body.
#[must_use]
pub fn resource_texts(
    system: &str,
    code: u32,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    let (title, body) = match code {
        1 => (0x7196, 0x7198),
        2 => (0x7196, 0x7197),
        3 => (0x7194, 0x719a),
        4 => (0x7194, 0x7199),
        _ => return None,
    };
    let named = [MessageParameter::Named { name: system }];
    Some((
        fill_message_text(texts.get(&title)?, &named),
        fill_message_text(texts.get(&body)?, &named),
    ))
}

/// System Destroyed by the Death Star (`FUN_004950d0`): notification
/// `0x182`, then `2`, for both sides. The title is TEXTSTRA `0x70c8` "{2:1}
/// Destroyed!". The body is `0x70c9` "The Death Star has destroyed the
/// {1:4} system {3:1}." for a system a side held (the side, the Death Star,
/// the system), or `0x70ca` for a neutral one. Returns the title and body.
#[must_use]
pub fn death_star_destroyed_texts(
    system: &str,
    holder: Option<rebellion_core::dat::Faction>,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    use rebellion_core::dat::Faction;
    let death_star = MessageParameter::Named { name: "Death Star" };
    let named = MessageParameter::Named { name: system };
    let title = fill_message_text(texts.get(&0x70c8)?, &[death_star, named]);
    let body = match holder {
        Some(Faction::Alliance) | Some(Faction::Empire) => fill_message_text(
            texts.get(&0x70c9)?,
            &[
                side_parameter(holder == Some(Faction::Alliance), texts),
                death_star,
                named,
            ],
        ),
        _ => fill_message_text(texts.get(&0x70ca)?, &[death_star, named]),
    };
    Some((title, body))
}

/// Research Complete (`FUN_00499aa0`): notifications `0x125` (ships),
/// `0x126` (troops) and `0x127` (facilities), a side's research order
/// reaching `level`. `FUN_00567a90` takes the first class of the tree's
/// family whose order equals the level (`FUN_00567880` with only "equal"
/// listed); with none there is no message. Its texts are TEXTSTRA
/// `0x7048` "{1:1} Research Complete" and `0x7049` "R&D Reports that the
/// {1:1} is now available for manufacture.". Returns the title and body.
#[must_use]
pub fn research_complete_texts(
    world: &rebellion_core::world::GameWorld,
    is_alliance: bool,
    tech: rebellion_core::research::TechType,
    level: u32,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    use rebellion_core::manufacturing::ProductionArea;
    use rebellion_core::research::TechType;
    let area = match tech {
        TechType::Ship => ProductionArea::Shipyard,
        TechType::Troop => ProductionArea::TrainingFacility,
        TechType::Facility => ProductionArea::ConstructionYard,
    };
    let class = rebellion_core::build_selection::research_classes(world, area, is_alliance)
        .into_iter()
        .find(|class| class.order == level)?
        .id;
    let name = world
        .capital_ship_classes
        .values()
        .find(|value| value.dat_id == class)
        .map(|value| value.name.clone())
        .or_else(|| {
            world
                .fighter_classes
                .values()
                .find(|value| value.dat_id == class)
                .map(|value| value.name.clone())
        })
        .or_else(|| {
            world
                .buildable_classes
                .get(&class)
                .map(|value| value.name.clone())
        })?;
    let named = [MessageParameter::Named { name: &name }];
    Some((
        fill_message_text(texts.get(&0x7048)?, &named),
        fill_message_text(texts.get(&0x7049)?, &named),
    ))
}

/// Research Exhausted (`FUN_0048abc0`): notifications `0x129` (ships),
/// `0x12a` (troops) and `0x12b` (facilities), raised when a side's tree is
/// marked Done (`FUN_0052f440` and siblings): TEXTSTRA `0x7218` with
/// `0x7219`, `0x721a` or `0x721b`. Returns the title and body.
#[must_use]
pub fn research_exhausted_texts(
    tech: rebellion_core::research::TechType,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    use rebellion_core::research::TechType;
    let body = match tech {
        TechType::Ship => 0x7219,
        TechType::Troop => 0x721a,
        TechType::Facility => 0x721b,
    };
    Some((
        fill_message_text(texts.get(&0x7218)?, &[]),
        fill_message_text(texts.get(&body)?, &[]),
    ))
}

/// Facility Lost (`FUN_00497690`): event `0x303`
/// (`GameObjDestroyedOnArrivalNotif`, message type `0x10` in
/// `FUN_00488de0`) for an object lost on arrival. Only a facility (family
/// `0x20`..`0x2f`) speaks: TEXTSTRA `0x70b0` "Facility Lost" and `0x70b1`
/// "New {1:1} could not be deployed to {2:1}.  The facility has been
/// scrapped.". Returns the title and body; `None` for any other unit.
#[must_use]
pub fn facility_lost_texts(
    world: &rebellion_core::world::GameWorld,
    kind: rebellion_core::manufacturing::BuildableKind,
    destination: &str,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    use rebellion_core::manufacturing::BuildableKind;
    if !matches!(
        kind,
        BuildableKind::DefenseFacility(_)
            | BuildableKind::ManufacturingFacility(_)
            | BuildableKind::ProductionFacility(_)
    ) {
        return None;
    }
    let (name, _) = crate::manufacturing_window::product(world, kind);
    let parameters = [
        MessageParameter::Named { name: &name },
        MessageParameter::Named { name: destination },
    ];
    Some((
        fill_message_text(texts.get(&0x70b0)?, &parameters),
        fill_message_text(texts.get(&0x70b1)?, &parameters),
    ))
}

/// The Message Index rail of a unit's Units Deployed (`FUN_00497940`) or
/// Unit Arrival (`FUN_004981c0`) message: the class's category mask
/// (`+0x34`) by the unit's family. Capital ships and the Death Star take
/// `0x80` (Fleet); a squadron or regiment at a system `0x40` (Defense); a
/// special force `0x10` (Mission); a defense facility `0x40`; a shipyard,
/// training facility or construction yard `0x08` (Manufacturing); a mine
/// or refinery `0x04` (Resource) when deployed, `0x08` when arriving.
///
/// port: a completed or delivered unit stands at its system.
#[must_use]
pub fn unit_message_rail(
    kind: rebellion_core::manufacturing::BuildableKind,
    arriving: bool,
) -> crate::message_log::MessageRail {
    use crate::message_log::MessageRail;
    use rebellion_core::manufacturing::BuildableKind;
    match kind {
        BuildableKind::CapitalShip(_) => MessageRail::Fleet,
        BuildableKind::Fighter(_) | BuildableKind::Troop(_) | BuildableKind::DefenseFacility(_) => {
            MessageRail::Defense
        }
        BuildableKind::SpecialForce(_) => MessageRail::Mission,
        BuildableKind::ManufacturingFacility(_) => MessageRail::Manufacturing,
        BuildableKind::ProductionFacility(_) if arriving => MessageRail::Manufacturing,
        BuildableKind::ProductionFacility(_) => MessageRail::Resource,
    }
}

/// The advisor event a message names its speaker by (`FUN_0048db30`, and
/// the same switch in the mission report `FUN_004927c0` on its `+0x74`):
/// the major characters' own codes, Mon Mothma `0x17`, Leia `0x18`, Luke
/// `0x16`, Han `0x19`, the Emperor `0x1a`, Vader `0x1b`; a speaker that is no
/// character (a special force) `0x15`; any other character none.
#[must_use]
pub fn speaker_event(world: &GameWorld, character: Option<CharacterKey>) -> u32 {
    character_speaker_event(character.and_then(|key| world.characters.get(key)))
}

/// [`speaker_event`] for a character in hand.
#[must_use]
pub fn character_speaker_event(character: Option<&rebellion_core::world::Character>) -> u32 {
    let Some(character) = character else {
        return 0x15;
    };
    if !character.is_major {
        return 0;
    }
    // The loader keeps the record index without the family byte.
    match character.dat_id.index() {
        0x240 => 0x17,
        0x241 => 0x18,
        0x242 => 0x16,
        0x243 => 0x19,
        0x280 => 0x1a,
        0x281 => 0x1b,
        _ => 0,
    }
}

/// The advisor event of a capture (`FUN_00490340`, notification `0x1e0`
/// cases 3..5): Luke `0x1e`, Mon Mothma `0x1f`, Leia `0x20`, Han `0x21`, the
/// Emperor `0x22`, Vader `0x23`, anyone else `0x15`.
///
/// hyp: case 5's Han branch leaves the switch for its default, which may
/// drop the message; the port keeps Han's `0x21`.
#[must_use]
pub fn captured_event(character: &rebellion_core::world::Character) -> u32 {
    if !character.is_major {
        return 0x15;
    }
    match character.dat_id.index() {
        0x242 => 0x1e,
        0x240 => 0x1f,
        0x241 => 0x20,
        0x243 => 0x21,
        0x280 => 0x22,
        0x281 => 0x23,
        _ => 0x15,
    }
}

/// The advisor event a unit's arrival hands on (`FUN_004981c0`): 5 with the
/// ships' texts (`0x7076`), 6 with the units' (`0x707a`).
#[must_use]
pub fn arrival_event(kind: rebellion_core::manufacturing::BuildableKind) -> u32 {
    match kind {
        rebellion_core::manufacturing::BuildableKind::CapitalShip(_) => 5,
        _ => 6,
    }
}

/// Unit Arrival (`FUN_004981c0`): `GameObjEnrouteActiveNotif` (`0x105`)
/// clearing, a unit arriving where it was bound, to the unit's side. A
/// capital ship takes TEXTSTRA `0x7076` "Ships arrive at {2:1}" and
/// `0x7077`; a squadron, regiment or facility `0x707a` "Units Arrive at
/// {2:1}" and `0x707b`; each adds a line `0x707e` for the unit. Characters
/// and special forces (`0x30`..`0x3f`) have no arrival message here.
/// Returns the title and body.
///
/// port: the class also stays silent while the system has a battle
/// pending (`+0x88` bit 12); the port has no such bit.
#[must_use]
pub fn arrival_texts(
    world: &rebellion_core::world::GameWorld,
    kind: rebellion_core::manufacturing::BuildableKind,
    system: &str,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    use rebellion_core::manufacturing::BuildableKind;
    let (title, body) = match kind {
        BuildableKind::CapitalShip(_) => (0x7076, 0x7077),
        BuildableKind::SpecialForce(_) => return None,
        _ => (0x707a, 0x707b),
    };
    let (name, _) = crate::manufacturing_window::product(world, kind);
    let parameters = [
        MessageParameter::Unit {
            name: &name,
            class: &name,
        },
        MessageParameter::Named { name: system },
    ];
    let mut text = fill_message_text(texts.get(&body)?, &parameters);
    text.push_str(&fill_message_text(texts.get(&0x707e)?, &parameters));
    Some((fill_message_text(texts.get(&title)?, &parameters), text))
}

/// Units Deployed (`FUN_00497940`): notification `0x107` for a unit put
/// in place, to the unit's side. The texts follow the unit's family:
/// - a capital ship TEXTSTRA `0x7086` / `0x7087`, the Death Star `0x708e`
///   / `0x708f`;
/// - a squadron `0x708a` / `0x708b`;
/// - a regiment or special force `0x7088` / `0x7089` with a `0x707f` line
///   for the unit (special force `0x3c000009` stays silent);
/// - a facility `0x708c` / `0x708d`.
///
/// The unit is the first substitution and the system the second.
///
/// A product bound elsewhere is Deployed when completed (`FUN_0052bee0`),
/// already a child of its destination, so its message names the
/// destination at departure; its arrival is [`arrival_texts`].
///
/// port: a completed unit stands at its system, so the body's container is
/// the system; the port names no unit, so its name is its class's.
#[must_use]
pub fn deployed_texts(
    world: &rebellion_core::world::GameWorld,
    kind: rebellion_core::manufacturing::BuildableKind,
    system: &str,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    use rebellion_core::manufacturing::BuildableKind;
    let (class, _) = crate::manufacturing_window::product(world, kind);
    let (title, body, line) = match kind {
        BuildableKind::CapitalShip(key) => {
            if world.capital_ship_classes.get(key)?.is_death_star() {
                (0x708e, 0x708f, None)
            } else {
                (0x7086, 0x7087, None)
            }
        }
        BuildableKind::Fighter(_) => (0x708a, 0x708b, None),
        BuildableKind::SpecialForce(class) if class.raw() == 0x3c00_0009 => return None,
        BuildableKind::Troop(_) | BuildableKind::SpecialForce(_) => (0x7088, 0x7089, Some(0x707f)),
        BuildableKind::DefenseFacility(_)
        | BuildableKind::ManufacturingFacility(_)
        | BuildableKind::ProductionFacility(_) => (0x708c, 0x708d, None),
    };
    let parameters = [
        MessageParameter::Unit {
            name: &class,
            class: &class,
        },
        MessageParameter::Named { name: system },
    ];
    let mut text = fill_message_text(texts.get(&body)?, &parameters);
    if let Some(line) = line {
        text.push_str(&fill_message_text(texts.get(&line)?, &parameters));
    }
    Some((fill_message_text(texts.get(&title)?, &parameters), text))
}

/// Facility Idle (`FUN_0048be60`): notification `0x160`, a manager's
/// remaining count reaching 0. The holder's side hears it, when the system
/// still has a facility of the area's family. A construction yard area
/// takes TEXTSTRA `0x71f8` / `0x71f9`, a training area `0x71fa` / `0x71fb`,
/// a shipyard area `0x71fc` / `0x71fd`, the system as `{1:1}`. Returns the
/// title and body.
#[must_use]
pub fn facility_idle_texts(
    system: &str,
    area: rebellion_core::manufacturing::ProductionArea,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    use rebellion_core::manufacturing::ProductionArea;
    let (title, body) = match area {
        ProductionArea::ConstructionYard => (0x71f8, 0x71f9),
        ProductionArea::TrainingFacility => (0x71fa, 0x71fb),
        ProductionArea::Shipyard => (0x71fc, 0x71fd),
    };
    let named = [MessageParameter::Named { name: system }];
    Some((
        fill_message_text(texts.get(&title)?, &named),
        fill_message_text(texts.get(&body)?, &named),
    ))
}

/// System Joins / Joins Enemy / Declares Neutrality (`FUN_00499760`):
/// notification `0x100`, a system's side changing from `previous` to
/// `holder`. The class speaks only while `+0x88` bit `0x2000`
/// (LoyaltyCausedCurrentControlKind, `FUN_00510d70`) is set: the support
/// moved the system, no regiments did. A system joining the viewer's side
/// takes TEXTSTRA `0x7038` / `0x703b`, one joining the other side `0x7039`
/// / `0x703c`, both naming the new side (kind 1). A system left with no
/// side takes `0x703a` / `0x703d`, naming the side it left (kind 4).
/// Returns the title and body; `None` when the side did not change.
#[must_use]
pub fn control_change_texts(
    system: &str,
    previous: Option<rebellion_core::dat::Faction>,
    holder: Option<rebellion_core::dat::Faction>,
    viewer_is_alliance: bool,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    use rebellion_core::dat::Faction;
    let side = |faction: Option<Faction>| match faction {
        Some(Faction::Alliance) => Some(true),
        Some(Faction::Empire) => Some(false),
        _ => None,
    };
    let (before, after) = (side(previous), side(holder));
    if before == after {
        return None;
    }
    let named = MessageParameter::Named { name: system };
    let (title, body, other) = match after {
        Some(alliance) if alliance == viewer_is_alliance => (0x7038, 0x703b, after),
        Some(_) => (0x7039, 0x703c, after),
        None => (0x703a, 0x703d, before),
    };
    let other = other.map_or(MessageParameter::Named { name: "" }, |alliance| {
        side_parameter(alliance, texts)
    });
    Some((
        fill_message_text(texts.get(&title)?, &[named, other]),
        fill_message_text(texts.get(&body)?, &[named, other]),
    ))
}

/// Blockade (`FUN_004960f0`): notification `0x14e` with the blockade bit
/// set; the end of a blockade posts nothing. It needs a held system and a
/// fleet there (`FUN_004ffdd0`, the first). The holder's side reads TEXTSTRA
/// `0x70a2` "{1:1} Under Blockade" and `0x70a3` "{1:4} ships have been
/// detected at {2:1}.  The world is under enemy blockade.", naming the other
/// side (`FUN_004f3dd0(viewer, 3 - holder)`); the other side reads `0x70a0`
/// "Fleet Initiates Blockade of {1:1}" and `0x70a1` "{3:1} has initiated a
/// blockade of {2:1}.", naming the fleet. Returns the title and body.
///
/// port: the class is also silent while the system's `+0x88` bit `0x4000`
/// is set; the port keeps no such bit.
#[must_use]
pub fn blockade_texts(
    system: &str,
    holder: Option<rebellion_core::dat::Faction>,
    fleet: &str,
    viewer_is_alliance: bool,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    use rebellion_core::dat::Faction;
    let holder_is_alliance = match holder? {
        Faction::Alliance => true,
        Faction::Empire => false,
        Faction::Neutral => return None,
    };
    let enemy = side_parameter(!holder_is_alliance, texts);
    let named = MessageParameter::Named { name: system };
    let fleet = MessageParameter::Named { name: fleet };
    let (title, body) = if holder_is_alliance == viewer_is_alliance {
        (0x70a2, 0x70a3)
    } else {
        (0x70a0, 0x70a1)
    };
    let parameters = [enemy, named, fleet];
    Some((
        fill_message_text(texts.get(&title)?, &[named]),
        fill_message_text(texts.get(&body)?, &parameters),
    ))
}

/// Evacuation Losses (`FUN_00495e90`): event `0x340`
/// (`TroopRegDestroyedRunningBlockade`), a regiment lost running a
/// blockade. TEXTSTRA `0x70a8` "Evacuation Losses", `0x70a9` "The following
/// units were lost running an enemy blockade of {2:1}.", and a line `0x70aa`
/// for the regiment. Returns the title and body.
#[must_use]
pub fn evacuation_losses_texts(
    regiment: &str,
    system: &str,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    let parameters = [
        MessageParameter::Named { name: regiment },
        MessageParameter::Named { name: system },
    ];
    let mut body = fill_message_text(texts.get(&0x70a9)?, &parameters);
    body.push_str(&fill_message_text(texts.get(&0x70aa)?, &parameters));
    Some((fill_message_text(texts.get(&0x70a8)?, &parameters), body))
}

/// Informants Provide Information (`FUN_00491060`): notification `0x153`,
/// TEXTSTRA `0x7150` / `0x7151`, the system as `{1:1}`. Returns the title
/// and body.
#[must_use]
pub fn informants_texts(system: &str, texts: &HashMap<u16, String>) -> Option<(String, String)> {
    let named = [MessageParameter::Named { name: system }];
    Some((
        fill_message_text(texts.get(&0x7150)?, &named),
        fill_message_text(texts.get(&0x7151)?, &named),
    ))
}

/// Maintenance Shortfall (`FUN_00498970`): notification `0x304` for each
/// autoscrapped unit, to the unit's side. The title is TEXTSTRA `0x7080`;
/// the body `0x7081` names the system the unit was at (its third
/// substitution, found up the unit's containers, empty when there is
/// none), and each unit adds a line `0x7082`. Returns the title and body.
#[must_use]
pub fn maintenance_texts(
    system: Option<&str>,
    unit: &str,
    texts: &HashMap<u16, String>,
) -> Option<(String, String)> {
    let unit = MessageParameter::Named { name: unit };
    let mut body = fill_message_text(
        texts.get(&0x7081)?,
        &[
            unit,
            MessageParameter::Named { name: "" },
            MessageParameter::Named {
                name: system.unwrap_or_default(),
            },
        ],
    );
    body.push_str(&fill_message_text(texts.get(&0x7082)?, &[unit]));
    Some((fill_message_text(texts.get(&0x7080)?, &[]), body))
}

/// Near Uprising (`FUN_00498cb0`): notification `0x148`, the system's
/// troop surplus going from `previous` to `surplus` (`FUN_00511fd0`). Only
/// the holder's side hears it, and only for a garrisoned system (`+0x88`
/// bit 8, `FUN_0050ba90`) not in revolt (bit 2) whose surplus has just
/// fallen to 0: TEXTSTRA `0x7070` / `0x7071` ("Unrest has pushed {1:1}
/// close to uprising."). The picture is `0x452` / `0x453` by the holder.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "Mirrors the class's inputs: the system's side, bits and the two values."
)]
pub fn near_uprising_message(
    system: &str,
    holder: Option<rebellion_core::dat::Faction>,
    viewer_is_alliance: bool,
    garrisoned: bool,
    in_uprising: bool,
    previous: i32,
    surplus: i32,
    tick: u64,
    texts: &HashMap<u16, String>,
) -> Option<GameMessage> {
    use rebellion_core::dat::Faction;
    let own = match holder? {
        Faction::Alliance => viewer_is_alliance,
        Faction::Empire => !viewer_is_alliance,
        Faction::Neutral => false,
    };
    if !own || !garrisoned || in_uprising || surplus != 0 || previous == 0 {
        return None;
    }
    let parameters = [MessageParameter::Named { name: system }];
    Some(
        GameMessage::new(
            tick,
            fill_message_text(texts.get(&0x7071)?, &parameters),
            MessageCategory::Event,
        )
        .on_rail(
            crate::message_log::MessageRail::PopularSupport,
            RailAudience::side(viewer_is_alliance),
        )
        .with_title(fill_message_text(texts.get(&0x7070)?, &parameters)),
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
            })
            // FUN_0048b6e0 writes 0x1b.
            .with_advisor_event(0x1b),
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
    // FUN_0048fa00's +0x28: 0x21 for the capture, else 0x19 (the Alliance's
    // attack) and 0x2d (the Empire's location).
    if c.is_captive {
        return message(0x716c, 0x716d, &[], 0x41d, RailAudience::Both)
            .map(|message| message.with_advisor_event(0x21))
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
        .map(|message| message.with_advisor_event(0x19))
        .into_iter()
        .collect();
    if let Some(system) = system {
        out.extend(
            message(
                0x716a,
                0x716b,
                &[MessageParameter::Named { name: &system.name }],
                0x41e,
                RailAudience::Empire,
            )
            .map(|message| message.with_advisor_event(0x2d)),
        );
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
    /// A unit or a system (`FUN_004f6030`): kinds 1, 2 and 4 its own name,
    /// which is its class's when it has none (`FUN_004f62d0`), kind 3 its
    /// class's name.
    Unit { name: &'a str, class: &'a str },
    /// A side (`FUN_0052d920`): kind 1 its name, kind 2 its full name,
    /// kind 4 its adjective ([`side_parameter`]).
    Side {
        name: &'a str,
        full_name: &'a str,
        adjective: &'a str,
    },
}

impl MessageParameter<'_> {
    /// The text a substitution of `kind` takes from this parameter.
    #[must_use]
    pub fn text(&self, kind: u8) -> Option<&str> {
        match (self, kind) {
            (Self::Character { name, .. }, 1) => Some(name),
            (Self::Character { force_ranking, .. }, 7) => Some(force_ranking),
            (Self::Named { name }, 1)
            | (Self::Side { name, .. }, 1)
            | (Self::Unit { name, .. }, 1 | 2 | 4) => Some(name),
            (Self::Unit { class, .. }, 3) => Some(class),
            (Self::Side { full_name, .. }, 2) => Some(full_name),
            (Self::Side { adjective, .. }, 4) => Some(adjective),
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

    #[test]
    fn a_ships_arrival_hands_on_5_and_any_other_units_6() {
        // FUN_004981c0: +0x28 = 5 with texts 0x7076 / 0x7077, 6 with
        // 0x707a / 0x707b.
        use rebellion_core::manufacturing::BuildableKind;
        assert_eq!(
            arrival_event(BuildableKind::CapitalShip(Default::default())),
            5
        );
        assert_eq!(arrival_event(BuildableKind::Fighter(Default::default())), 6);
    }

    use super::*;

    // FUN_00497940 and FUN_004981c0 set their category mask (+0x34) by the
    // unit's family; FUN_0042d8d0 files the message on that rail.
    #[test]
    fn a_unit_s_message_goes_to_the_rail_of_its_family() {
        use crate::message_log::MessageRail;
        use rebellion_core::ids::{CapitalShipKey, DatId, FighterKey};
        use rebellion_core::manufacturing::{BuildableKind, FacilityBuild};
        let build = |id| FacilityBuild {
            class: DatId::new(id),
            is_alliance: true,
        };
        let rail = unit_message_rail;
        assert_eq!(
            rail(BuildableKind::CapitalShip(CapitalShipKey::default()), false),
            MessageRail::Fleet
        );
        assert_eq!(
            rail(BuildableKind::Fighter(FighterKey::default()), false),
            MessageRail::Defense
        );
        assert_eq!(
            rail(BuildableKind::Troop(DatId::new(0x1000_0001)), true),
            MessageRail::Defense
        );
        assert_eq!(
            rail(BuildableKind::SpecialForce(DatId::new(0x3c00_0001)), false),
            MessageRail::Mission
        );
        assert_eq!(
            rail(BuildableKind::DefenseFacility(build(0x2200_0001)), false),
            MessageRail::Defense
        );
        assert_eq!(
            rail(
                BuildableKind::ManufacturingFacility(build(0x2800_0001)),
                false
            ),
            MessageRail::Manufacturing
        );
        assert_eq!(
            rail(BuildableKind::ProductionFacility(build(0x2c00_0001)), false),
            MessageRail::Resource
        );
        assert_eq!(
            rail(BuildableKind::ProductionFacility(build(0x2c00_0001)), true),
            MessageRail::Manufacturing
        );
    }

    // FUN_004981c0 (notification 0x105 clearing): TEXTSTRA 0x7076 / 0x7077,
    // 0x707a / 0x707b and the unit line 0x707e as shipped, by family.
    #[test]
    fn an_arrival_reports_ships_or_units_and_lists_the_unit() {
        use rebellion_core::ids::DatId;
        use rebellion_core::manufacturing::BuildableKind;
        use rebellion_core::world::{CapitalShipClass, GameWorld};
        let texts: HashMap<u16, String> = [
            (0x7076, "Ships arrive at {2:1}"),
            (0x7077, "The following ships have arrived at {2:1}:\n"),
            (0x707a, "Units Arrive at {2:1}"),
            (0x707b, "The following units have arrived at {2:1}:\n"),
            (0x707e, "{1:1}\n"),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let mut world = GameWorld::default();
        let cruiser = world.capital_ship_classes.insert(CapitalShipClass {
            name: "Strike Cruiser".into(),
            ..CapitalShipClass::default()
        });
        assert_eq!(
            arrival_texts(
                &world,
                BuildableKind::CapitalShip(cruiser),
                "F'tral",
                &texts
            ),
            Some((
                "Ships arrive at F'tral".into(),
                "The following ships have arrived at F'tral:\nStrike Cruiser\n".into()
            ))
        );
        let regiment = arrival_texts(
            &world,
            BuildableKind::Troop(DatId::new(0x1000_0003)),
            "F'tral",
            &texts,
        )
        .unwrap();
        assert_eq!(regiment.0, "Units Arrive at F'tral");
        assert!(
            regiment
                .1
                .starts_with("The following units have arrived at F'tral:\n"),
            "{regiment:?}"
        );
        assert_eq!(
            arrival_texts(
                &world,
                BuildableKind::SpecialForce(DatId::new(0x3c00_0001)),
                "F'tral",
                &texts
            ),
            None
        );
    }

    // FUN_00497690 (event 0x303): TEXTSTRA 0x70b0 / 0x70b1 as shipped, for a
    // facility only (family 0x20..0x2f).
    #[test]
    fn only_a_facility_lost_on_arrival_is_reported() {
        use rebellion_core::ids::DatId;
        use rebellion_core::manufacturing::{BuildableKind, FacilityBuild};
        use rebellion_core::world::{CapitalShipClass, GameWorld};
        let texts: HashMap<u16, String> = [
            (0x70b0, "Facility Lost"),
            (
                0x70b1,
                "New {1:1} could not be deployed to {2:1}.  The facility has been scrapped.",
            ),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let mut world = GameWorld::default();
        let mine = DatId::new(0x2c00_0001);
        world.buildable_classes.insert(
            mine,
            rebellion_core::world::BuildableClass {
                name: "Mine".into(),
                ..rebellion_core::world::BuildableClass::default()
            },
        );
        let ship = world.capital_ship_classes.insert(CapitalShipClass {
            name: "Nebulon-B Frigate".into(),
            ..CapitalShipClass::default()
        });
        assert_eq!(
            facility_lost_texts(
                &world,
                BuildableKind::ProductionFacility(FacilityBuild {
                    class: mine,
                    is_alliance: true,
                }),
                "Ghorman",
                &texts
            ),
            Some((
                "Facility Lost".into(),
                "New Mine could not be deployed to Ghorman.  The facility has been scrapped."
                    .into()
            ))
        );
        assert_eq!(
            facility_lost_texts(&world, BuildableKind::CapitalShip(ship), "Ghorman", &texts),
            None
        );
    }

    // FUN_00499aa0 (0x125..0x127) and FUN_0048abc0 (0x129..0x12b): TEXTSTRA
    // 0x7048 / 0x7049 and 0x7218..0x721b as shipped; the class is the first
    // of the tree whose research order equals the new level (FUN_00567a90).
    #[test]
    fn research_names_the_class_it_opened_and_says_when_a_tree_is_done() {
        use rebellion_core::ids::DatId;
        use rebellion_core::research::TechType;
        use rebellion_core::world::{CapitalShipClass, GameWorld};
        let texts: HashMap<u16, String> = [
            (0x7048, "{1:1} Research Complete"),
            (
                0x7049,
                "R&D Reports that the {1:1} is now available for manufacture.",
            ),
            (0x7218, "Research Exhausted"),
            (
                0x7219,
                "There are no further advances expected in ship construction.",
            ),
            (
                0x721a,
                "There are no further advances expected in troop training.",
            ),
            (
                0x721b,
                "There are no further advances expected in facility construction.",
            ),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let mut world = GameWorld::default();
        for (id, name, order, alliance) in [
            (0x1400_0003, "Assault Frigate", 2, true),
            (0x1400_0002, "Nebulon-B Frigate", 1, true),
            (0x1400_0004, "Dreadnaught", 1, false),
        ] {
            world.capital_ship_classes.insert(CapitalShipClass {
                dat_id: DatId::new(id),
                name: name.into(),
                is_alliance: alliance,
                is_empire: !alliance,
                research_order: order,
                refined_material_cost: 10,
                ..CapitalShipClass::default()
            });
        }
        assert_eq!(
            research_complete_texts(&world, true, TechType::Ship, 1, &texts),
            Some((
                "Nebulon-B Frigate Research Complete".into(),
                "R&D Reports that the Nebulon-B Frigate is now available for manufacture.".into()
            ))
        );
        assert_eq!(
            research_complete_texts(&world, false, TechType::Ship, 1, &texts)
                .map(|(title, _)| title),
            Some("Dreadnaught Research Complete".into())
        );
        // A fighter class opens on the same tree.
        for (id, name) in [(0x1c00_0001, "A-wing"), (0x1c00_0002, "B-wing")] {
            world
                .fighter_classes
                .insert(rebellion_core::world::FighterClass {
                    dat_id: DatId::new(id),
                    name: name.into(),
                    is_alliance: true,
                    research_order: if id == 0x1c00_0002 { 3 } else { 4 },
                    refined_material_cost: 5,
                    ..rebellion_core::world::FighterClass::default()
                });
        }
        assert_eq!(
            research_complete_texts(&world, true, TechType::Ship, 3, &texts)
                .map(|(title, _)| title),
            Some("B-wing Research Complete".into())
        );
        // No class at that order: no message.
        assert_eq!(
            research_complete_texts(&world, true, TechType::Ship, 5, &texts),
            None
        );
        assert_eq!(
            research_exhausted_texts(TechType::Troop, &texts),
            Some((
                "Research Exhausted".into(),
                "There are no further advances expected in troop training.".into()
            ))
        );
        assert_eq!(
            research_exhausted_texts(TechType::Ship, &texts).map(|(_, body)| body),
            Some("There are no further advances expected in ship construction.".into())
        );
        assert_eq!(
            research_exhausted_texts(TechType::Facility, &texts).map(|(_, body)| body),
            Some("There are no further advances expected in facility construction.".into())
        );
    }

    // FUN_004950d0 (notification 0x182): TEXTSTRA 0x70c8..0x70ca as shipped;
    // the side is the system's before the blast, by kind 4 (FUN_0052d920).
    #[test]
    fn the_death_star_names_the_side_whose_system_it_destroyed() {
        use rebellion_core::dat::Faction;
        let texts: HashMap<u16, String> = [
            (0x70c8, "{2:1} Destroyed!"),
            (
                0x70c9,
                "The Death Star has destroyed the {1:4} system {3:1}.",
            ),
            (
                0x70ca,
                "The Death Star has destroyed the neutral system {2:1}.",
            ),
            (0x4d70, "Alliance"),
            (0x4d71, "Imperial"),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        assert_eq!(
            death_star_destroyed_texts("Alderaan", Some(Faction::Alliance), &texts),
            Some((
                "Alderaan Destroyed!".into(),
                "The Death Star has destroyed the Alliance system Alderaan.".into()
            ))
        );
        assert_eq!(
            death_star_destroyed_texts("Alderaan", Some(Faction::Empire), &texts)
                .map(|(_, body)| body),
            Some("The Death Star has destroyed the Imperial system Alderaan.".into())
        );
        assert_eq!(
            death_star_destroyed_texts("Alderaan", Some(Faction::Neutral), &texts)
                .map(|(_, body)| body),
            Some("The Death Star has destroyed the neutral system Alderaan.".into())
        );
        assert_eq!(
            death_star_destroyed_texts("Alderaan", None, &texts).map(|(_, body)| body),
            Some("The Death Star has destroyed the neutral system Alderaan.".into())
        );
    }

    // FUN_00497940 (notification 0x107): TEXTSTRA 0x7086..0x708f and 0x707f
    // as shipped, by the unit's family; kind 3 is the class's name
    // (FUN_004f6030).
    #[test]
    fn a_deployed_unit_takes_the_texts_of_its_family() {
        use rebellion_core::ids::DatId;
        use rebellion_core::manufacturing::BuildableKind;
        use rebellion_core::world::{CapitalShipClass, FighterClass, GameWorld};
        let texts: HashMap<u16, String> = [
            (0x707f, "{1:1}\n"),
            (0x7086, "{1:3} Deployed at {2:1}"),
            (0x7087, "The new {1:3}, {1:1}, has been deployed at {2:1}."),
            (0x7088, "{1:1} Deployed to {2:1}"),
            (0x7089, "The following units have been deployed to {2:1}:\n"),
            (0x708a, "{1:1} Squadron Deployed at {2:1}"),
            (0x708b, "A new {1:1} squadron has been deployed to {2:1}."),
            (0x708c, "{1:1} Deployed at {2:1}"),
            (0x708d, "A new {1:1} has been deployed to {2:1}."),
            (0x708e, "New Death Star Deployed at {2:1}"),
            (
                0x708f,
                "The new Death Star, {1:1}, has been deployed at {2:1}.",
            ),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let mut world = GameWorld::default();
        let frigate = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x1400_0002),
            name: "Nebulon-B Frigate".into(),
            ..CapitalShipClass::default()
        });
        let death_star = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(rebellion_core::world::DEATH_STAR_CLASS_ID),
            name: "Death Star".into(),
            ..CapitalShipClass::default()
        });
        let xwing = world.fighter_classes.insert(FighterClass {
            name: "X-wing".into(),
            ..FighterClass::default()
        });
        let say = |kind| deployed_texts(&world, kind, "Fondor", &texts);
        assert_eq!(
            say(BuildableKind::CapitalShip(frigate)),
            Some((
                "Nebulon-B Frigate Deployed at Fondor".into(),
                "The new Nebulon-B Frigate, Nebulon-B Frigate, has been deployed at Fondor.".into()
            ))
        );
        assert_eq!(
            say(BuildableKind::CapitalShip(death_star)),
            Some((
                "New Death Star Deployed at Fondor".into(),
                "The new Death Star, Death Star, has been deployed at Fondor.".into()
            ))
        );
        assert_eq!(
            say(BuildableKind::Fighter(xwing)),
            Some((
                "X-wing Squadron Deployed at Fondor".into(),
                "A new X-wing squadron has been deployed to Fondor.".into()
            ))
        );
        let regiment = say(BuildableKind::Troop(DatId::new(0x1000_0003))).unwrap();
        assert!(regiment.0.ends_with(" Deployed to Fondor"), "{regiment:?}");
        assert!(
            regiment
                .1
                .starts_with("The following units have been deployed to Fondor:\n"),
            "{regiment:?}"
        );
        assert!(regiment.1.ends_with('\n'), "{regiment:?}");
        assert_eq!(
            say(BuildableKind::SpecialForce(DatId::new(0x3c00_0009))),
            None
        );
        // Every other special force speaks with the regiments' texts.
        let commandos = say(BuildableKind::SpecialForce(DatId::new(0x3c00_0001))).unwrap();
        assert!(
            commandos.0.ends_with(" Deployed to Fondor"),
            "{commandos:?}"
        );
    }

    // FUN_0048be60 (notification 0x160): TEXTSTRA 0x71f8..0x71fd as shipped,
    // by the manager's area (slot +0x1ec: 1 construction, 2 training,
    // otherwise shipyards).
    #[test]
    fn an_idle_area_names_its_own_kind_of_facility() {
        use rebellion_core::manufacturing::ProductionArea;
        let texts: HashMap<u16, String> = [
            (0x71f8, "Construction Yard Idle on {1:1}"),
            (
                0x71f9,
                "The Construction Yards at {1:1} have completed their assigned tasks.",
            ),
            (0x71fa, "Training Facility Idle on {1:1}"),
            (
                0x71fb,
                "The Training Facilities at {1:1} are no longer in use.",
            ),
            (0x71fc, "Shipyard Idle on {1:1}"),
            (
                0x71fd,
                "The Shipyards at {1:1} are available for new construction orders.",
            ),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        assert_eq!(
            facility_idle_texts("Fondor", ProductionArea::Shipyard, &texts),
            Some((
                "Shipyard Idle on Fondor".into(),
                "The Shipyards at Fondor are available for new construction orders.".into()
            ))
        );
        assert_eq!(
            facility_idle_texts("Fondor", ProductionArea::TrainingFacility, &texts),
            Some((
                "Training Facility Idle on Fondor".into(),
                "The Training Facilities at Fondor are no longer in use.".into()
            ))
        );
        assert_eq!(
            facility_idle_texts("Fondor", ProductionArea::ConstructionYard, &texts),
            Some((
                "Construction Yard Idle on Fondor".into(),
                "The Construction Yards at Fondor have completed their assigned tasks.".into()
            ))
        );
    }

    // FUN_00499760 (notification 0x100): TEXTSTRA 0x7038..0x703d as
    // shipped; the side is named by FUN_0052d920 (kind 1 0x4d50 / 0x4d51,
    // kind 4 0x4d70 / 0x4d71).
    #[test]
    fn a_system_moved_by_its_support_joins_a_side_or_declares_neutrality() {
        use rebellion_core::dat::Faction;
        let texts: HashMap<u16, String> = [
            (0x7038, "{1:1} Joins"),
            (0x7039, "{1:1} Joins Enemy"),
            (0x703a, "{1:1} Declares Neutrality"),
            (
                0x703b,
                "Popular support on {1:1} has caused that world to join the {2:1}.",
            ),
            (
                0x703c,
                "Popular dissent on {1:1} has caused that world to join the {2:1}.",
            ),
            (
                0x703d,
                "Divided loyalty on {1:1} has caused that world to abandon the {2:4} cause.",
            ),
            (0x4d50, "Alliance"),
            (0x4d51, "Empire"),
            (0x4d70, "Alliance"),
            (0x4d71, "Imperial"),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let say = |previous, holder, viewer_is_alliance| {
            control_change_texts("Bespin", previous, holder, viewer_is_alliance, &texts)
        };
        assert_eq!(
            say(None, Some(Faction::Alliance), true),
            Some((
                "Bespin Joins".into(),
                "Popular support on Bespin has caused that world to join the Alliance.".into()
            ))
        );
        assert_eq!(
            say(Some(Faction::Alliance), Some(Faction::Empire), true),
            Some((
                "Bespin Joins Enemy".into(),
                "Popular dissent on Bespin has caused that world to join the Empire.".into()
            ))
        );
        assert_eq!(
            say(Some(Faction::Empire), Some(Faction::Neutral), true),
            Some((
                "Bespin Declares Neutrality".into(),
                "Divided loyalty on Bespin has caused that world to abandon the Imperial cause."
                    .into()
            ))
        );
        assert_eq!(
            say(Some(Faction::Alliance), None, false).map(|(_, body)| body),
            Some(
                "Divided loyalty on Bespin has caused that world to abandon the Alliance cause."
                    .into()
            )
        );
        // No change of side, no message.
        assert_eq!(
            say(Some(Faction::Empire), Some(Faction::Empire), true),
            None
        );
        assert_eq!(say(Some(Faction::Neutral), None, true), None);
    }

    // FUN_004960f0 (0x14e) and FUN_00495e90 (0x340): TEXTSTRA 0x70a0..0x70a3
    // and 0x70a8..0x70aa as shipped; the holder reads the other side's
    // adjective (FUN_004f3dd0(viewer, 3 - holder) at 0x4961c9).
    #[test]
    fn a_blockade_reads_differently_for_the_holder_and_the_blockader() {
        use rebellion_core::dat::Faction;
        let texts: HashMap<u16, String> = [
            (0x70a0, "Fleet Initiates Blockade of {1:1}"),
            (0x70a1, "{3:1} has initiated a blockade of {2:1}."),
            (0x70a2, "{1:1} Under Blockade"),
            (
                0x70a3,
                "{1:4} ships have been detected at {2:1}.  The world is under enemy blockade.",
            ),
            (0x70a8, "Evacuation Losses"),
            (
                0x70a9,
                "The following units were lost running an enemy blockade of {2:1}.\n",
            ),
            (0x70aa, "\n{1:1}"),
            (0x4d70, "Alliance"),
            (0x4d71, "Imperial"),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        assert_eq!(
            blockade_texts("Bespin", Some(Faction::Alliance), "Fleet 3", true, &texts),
            Some((
                "Bespin Under Blockade".into(),
                "Imperial ships have been detected at Bespin.  The world is under enemy blockade."
                    .into()
            ))
        );
        assert_eq!(
            blockade_texts("Bespin", Some(Faction::Alliance), "Fleet 3", false, &texts),
            Some((
                "Fleet Initiates Blockade of Bespin".into(),
                "Fleet 3 has initiated a blockade of Bespin.".into()
            ))
        );
        assert_eq!(
            blockade_texts("Bespin", Some(Faction::Empire), "Fleet 3", false, &texts)
                .map(|(_, body)| body),
            Some(
                "Alliance ships have been detected at Bespin.  The world is under enemy blockade."
                    .into()
            )
        );
        assert_eq!(
            blockade_texts("Bespin", Some(Faction::Neutral), "Fleet 3", true, &texts),
            None
        );
        assert_eq!(
            evacuation_losses_texts("Alliance Army Regiment", "Bespin", &texts),
            Some((
                "Evacuation Losses".into(),
                "The following units were lost running an enemy blockade of Bespin.\n\nAlliance Army Regiment"
                    .into()
            ))
        );
    }

    // FUN_00491060 (0x153) and FUN_00498970 (0x304): TEXTSTRA 0x7150 /
    // 0x7151 and 0x7080..0x7082 as shipped; 0x7081 names the system as
    // its third substitution.
    #[test]
    fn informants_and_maintenance_shortfall_take_their_texts() {
        let texts: HashMap<u16, String> = [
            (0x7150, "Informants Provide Information"),
            (0x7151, "Informants have provided us with information about {1:1}."),
            (0x7080, "Maintenance Shortfall"),
            (
                0x7081,
                "The following units have been destroyed at {3:1} due to a maintenance shortfall:\n",
            ),
            (0x7082, "{1:1}\n"),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        assert_eq!(
            informants_texts("Mon Calamari", &texts),
            Some((
                "Informants Provide Information".into(),
                "Informants have provided us with information about Mon Calamari.".into()
            ))
        );
        assert_eq!(
            maintenance_texts(Some("Kuat"), "Victory-class Star Destroyer", &texts),
            Some((
                "Maintenance Shortfall".into(),
                "The following units have been destroyed at Kuat due to a maintenance shortfall:\nVictory-class Star Destroyer\n".into()
            ))
        );
        assert_eq!(
            maintenance_texts(None, "Mine", &texts).map(|(_, body)| body),
            Some("The following units have been destroyed at  due to a maintenance shortfall:\nMine\n".into())
        );
    }

    // FUN_0048e940 (notification 0x155): TEXTSTRA 0x7194..0x719a as shipped.
    #[test]
    fn a_resource_change_names_what_was_found_or_used_up() {
        let texts: HashMap<u16, String> = [
            (0x7194, "New Resources on {1:1}"),
            (0x7196, "Resources Depleted on {1:1}"),
            (
                0x7197,
                "Operations on {1:1} have reduced the system energy available.",
            ),
            (
                0x7198,
                "Operations on {1:1} have reduced the number of raw materials on that system.",
            ),
            (
                0x7199,
                "New sources of system energy have been discovered on {1:1}.",
            ),
            (
                0x719a,
                "New sources of raw materials have been discovered on {1:1}.",
            ),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let say = |code| resource_texts("Sullust", code, &texts);
        assert_eq!(
            say(1),
            Some((
                "Resources Depleted on Sullust".into(),
                "Operations on Sullust have reduced the number of raw materials on that system."
                    .into()
            ))
        );
        assert_eq!(
            say(2).map(|(_, body)| body),
            Some("Operations on Sullust have reduced the system energy available.".into())
        );
        assert_eq!(
            say(3),
            Some((
                "New Resources on Sullust".into(),
                "New sources of raw materials have been discovered on Sullust.".into()
            ))
        );
        assert_eq!(
            say(4).map(|(_, body)| body),
            Some("New sources of system energy have been discovered on Sullust.".into())
        );
        assert_eq!(say(0), None);
        assert_eq!(say(5), None);
    }

    // FUN_00498cb0 (notification 0x148): TEXTSTRA 0x7070 / 0x7071 as
    // shipped; the class reads the new surplus as its first value and the
    // old as its second, as System Joins (FUN_00499760) reads the new side.
    #[test]
    fn near_uprising_warns_the_holder_when_a_garrisoned_system_s_surplus_falls_to_zero() {
        use rebellion_core::dat::Faction;
        let texts: HashMap<u16, String> = [
            (0x7070, "{1:1} Near Uprising"),
            (0x7071, "Unrest has pushed {1:1} close to uprising."),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let say = |holder, viewer_is_alliance, garrisoned, in_uprising, previous, surplus| {
            near_uprising_message(
                "Ord Mantell",
                holder,
                viewer_is_alliance,
                garrisoned,
                in_uprising,
                previous,
                surplus,
                3,
                &texts,
            )
            .map(|m| (m.title.unwrap_or_default(), m.text))
        };
        let warned = Some((
            "Ord Mantell Near Uprising".to_string(),
            "Unrest has pushed Ord Mantell close to uprising.".to_string(),
        ));
        assert_eq!(
            say(Some(Faction::Alliance), true, true, false, 2, 0),
            warned
        );
        assert_eq!(
            say(Some(Faction::Empire), false, true, false, -1, 0),
            warned
        );
        // Only the holder hears it.
        assert_eq!(say(Some(Faction::Alliance), false, true, false, 2, 0), None);
        assert_eq!(say(Some(Faction::Empire), true, true, false, 2, 0), None);
        assert_eq!(say(Some(Faction::Neutral), true, true, false, 2, 0), None);
        assert_eq!(say(None, true, true, false, 2, 0), None);
        // An ungarrisoned system, a revolt, a surplus that is not 0, or one
        // that was already 0 say nothing.
        assert_eq!(say(Some(Faction::Alliance), true, false, false, 2, 0), None);
        assert_eq!(say(Some(Faction::Alliance), true, true, true, 2, 0), None);
        assert_eq!(say(Some(Faction::Alliance), true, true, false, 0, 1), None);
        assert_eq!(say(Some(Faction::Alliance), true, true, false, 0, 0), None);
    }

    // FUN_00499460 (notification 0x14b): TEXTSTRA 0x7030..0x7033 as
    // shipped; the body's {1:4} is the holder's adjective from FUN_0052d920
    // (strings 0x4d70 / 0x4d71).
    #[test]
    fn an_uprising_names_the_holder_s_side_when_it_begins_and_ends() {
        use rebellion_core::dat::Faction;
        let texts: HashMap<u16, String> = [
            (0x7030, "Uprising Begins on {1:1}"),
            (0x7031, "Uprising Ends on {1:1}"),
            (0x7032, "The uprising on {2:1} has ended."),
            (
                0x7033,
                "An uprising has begun on the {1:4} controlled system {2:1}.",
            ),
            (0x4d50, "Alliance"),
            (0x4d51, "Empire"),
            (0x4d60, "Rebel Alliance"),
            (0x4d61, "Galactic Empire"),
            (0x4d70, "Alliance"),
            (0x4d71, "Imperial"),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        assert_eq!(
            uprising_texts("Kessel", Some(Faction::Empire), true, &texts),
            Some((
                "Uprising Begins on Kessel".into(),
                "An uprising has begun on the Imperial controlled system Kessel.".into()
            ))
        );
        assert_eq!(
            uprising_texts("Kessel", Some(Faction::Alliance), true, &texts).map(|(_, body)| body),
            Some("An uprising has begun on the Alliance controlled system Kessel.".into())
        );
        assert_eq!(
            uprising_texts("Kessel", Some(Faction::Empire), false, &texts),
            Some((
                "Uprising Ends on Kessel".into(),
                "The uprising on Kessel has ended.".into()
            ))
        );
        // A neutral system has no side to name, and its class stays silent.
        assert_eq!(
            uprising_texts("Kessel", Some(Faction::Neutral), true, &texts),
            None
        );
        assert_eq!(uprising_texts("Kessel", None, true, &texts), None);
        // The provider's other kinds: 1 the name, 2 the full name.
        let side = side_parameter(false, &texts);
        assert_eq!(side.text(1), Some("Empire"));
        assert_eq!(side.text(2), Some("Galactic Empire"));
        assert_eq!(side.text(4), Some("Imperial"));
    }

    // FUN_0048e940 (notification 0x154, then 2): TEXTSTRA 0x7190..0x7193 and
    // the unit line 0x719b as shipped.
    #[test]
    fn a_disaster_lists_what_it_destroyed_and_says_nothing_when_nothing_was_lost() {
        let texts: HashMap<u16, String> = [
            (0x7190, "Natural Disaster on {1:1}"),
            (0x7191, "There has been a natural disaster on {1:1} which has destroyed resources there."),
            (0x7192, "There has been a natural disaster on {1:1} which has destroyed the following units:"),
            (0x7193, "There has been a natural disaster on {1:1} which has destroyed facilities and resources.  The following units were lost:"),
            (0x719b, "\n{1:1}"),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let mine = vec!["Mine".to_string(), "Shipyard".to_string()];
        assert_eq!(
            disaster_texts("Bothawui", true, &[], &texts),
            Some((
                "Natural Disaster on Bothawui".into(),
                "There has been a natural disaster on Bothawui which has destroyed resources there."
                    .into()
            ))
        );
        assert_eq!(
            disaster_texts("Bothawui", false, &mine, &texts).map(|(_, body)| body),
            Some(
                "There has been a natural disaster on Bothawui which has destroyed the following units:\nMine\nShipyard"
                    .into()
            )
        );
        assert_eq!(
            disaster_texts("Bothawui", true, &mine[..1], &texts).map(|(_, body)| body),
            Some(
                "There has been a natural disaster on Bothawui which has destroyed facilities and resources.  The following units were lost:\nMine"
                    .into()
            )
        );
        assert_eq!(disaster_texts("Bothawui", false, &[], &texts), None);
    }

    // FUN_00499130 (notification 0x145): TEXTSTRA 0x7060..0x7067 as
    // shipped; {1:1} is the system's name.
    #[test]
    fn smuggling_speaks_when_it_begins_or_ends_by_whose_system_it_is() {
        use rebellion_core::dat::Faction;
        let texts: HashMap<u16, String> = [
            (0x7060, "Smuggling Losses"),
            (0x7061, "Smuggling Losses End"),
            (0x7062, "Smuggling Benefits"),
            (0x7063, "Smuggling Benefits End"),
            (0x7064, "Smugglers begin on {1:1}."),
            (0x7065, "Smuggling losses end on {1:1}."),
            (0x7066, "Smugglers from {1:1} help us."),
            (0x7067, "Smugglers from {1:1} withdraw."),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let say = |holder, viewer_is_alliance, previous, percent| {
            smuggling_message(
                "Ord Mantell",
                holder,
                viewer_is_alliance,
                previous,
                percent,
                9,
                &texts,
            )
            .map(|m| (m.title.unwrap_or_default(), m.text))
        };
        // Begins on the viewer's own system: losses.
        assert_eq!(
            say(Some(Faction::Alliance), true, 0, 20),
            Some((
                "Smuggling Losses".into(),
                "Smugglers begin on Ord Mantell.".into()
            ))
        );
        // Begins on the other side's: benefits.
        assert_eq!(
            say(Some(Faction::Alliance), false, 0, 20),
            Some((
                "Smuggling Benefits".into(),
                "Smugglers from Ord Mantell help us.".into()
            ))
        );
        // Ends: an Alliance system speaks when it was at least 0.
        assert_eq!(
            say(Some(Faction::Alliance), true, 20, 0).map(|m| m.0),
            Some("Smuggling Losses End".into())
        );
        assert_eq!(say(Some(Faction::Alliance), true, -20, 0), None);
        // An Empire system speaks when it was below 1.
        assert_eq!(
            say(Some(Faction::Empire), true, -20, 0).map(|m| m.0),
            Some("Smuggling Benefits End".into())
        );
        assert_eq!(say(Some(Faction::Empire), false, 20, 0), None);
        // The boundary: an Empire system that was at 1 says nothing.
        assert_eq!(say(Some(Faction::Empire), true, 1, 0), None);
        assert_eq!(
            say(Some(Faction::Empire), false, 0, -20).map(|m| m.0),
            Some("Smuggling Losses".into())
        );
        // A change between two non-zero values, or a neutral system, says
        // nothing.
        assert_eq!(say(Some(Faction::Alliance), true, 10, 20), None);
        assert_eq!(say(Some(Faction::Neutral), true, 0, 20), None);
        assert_eq!(say(None, true, 0, 20), None);
        let message =
            smuggling_message("Ord Mantell", Some(Faction::Empire), true, 0, 5, 9, &texts).unwrap();
        assert_eq!(
            message.rail,
            Some(crate::message_log::MessageRail::PopularSupport)
        );
        assert_eq!(message.audience, RailAudience::Alliance);
    }

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
                last_arrival: None,
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
        // FUN_0048ce80's +0x28.
        assert_eq!(message.advisor_event, [0x1a; 2]);
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
        // FUN_00490340: no major character, 0x15.
        assert_eq!(message.advisor_event, [0x15; 2]);
        assert!(jabba_captures_message(&chewbacca, 9, &HashMap::new()).is_none());
    }

    #[test]
    fn the_bounty_attack_messages_follow_han_s_capture() {
        // FUN_0048fa00 on 0x212: captured, both read 0x716c/0x716d on
        // 0x41d; otherwise the Alliance reads 0x7168/0x7169 on 0x41c and the
        // Empire 0x716a/0x716b with Han's system on 0x41e.
        let mut world = GameWorld::default();
        let tatooine = world.systems.insert(rebellion_core::world::System {
            last_arrival: None,
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
        // FUN_0048fa00's +0x28: 0x19 for the Alliance, 0x2d for the Empire.
        assert_eq!(attacked[0].advisor_event, [0x19; 2]);
        assert_eq!(attacked[1].advisor_event, [0x2d; 2]);

        world.characters[han].is_captive = true;
        let captured = bounty_messages(&world, han, 3, &texts);
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].audience, RailAudience::Both);
        assert_eq!(captured[0].picture.expect("a picture").background, 0x41d);
        assert_eq!(captured[0].advisor_event, [0x21; 2]);
    }

    #[test]
    fn the_majors_speak_by_their_own_codes_minors_by_none_and_others_by_0x15() {
        // FUN_0048db30 (and its copies, e.g. the mission report's on +0x74):
        // 0x240 Mon Mothma 0x17, 0x241 Leia 0x18, 0x242 Luke 0x16, 0x243
        // Han 0x19, 0x280 the Emperor 0x1a, 0x281 Vader 0x1b; no character
        // 0x15; another character none. FUN_00490340's capture codes:
        // Luke 0x1e, Mon Mothma 0x1f, Leia 0x20, Han 0x21, the Emperor 0x22,
        // Vader 0x23, anyone else 0x15.
        let person = |index: u32, is_major: bool| rebellion_core::world::Character {
            dat_id: DatId::new(index),
            is_major,
            ..Default::default()
        };
        for (index, speaker, captured) in [
            (0x240, 0x17, 0x1f),
            (0x241, 0x18, 0x20),
            (0x242, 0x16, 0x1e),
            (0x243, 0x19, 0x21),
            (0x280, 0x1a, 0x22),
            (0x281, 0x1b, 0x23),
        ] {
            let major = person(index, true);
            assert_eq!(character_speaker_event(Some(&major)), speaker, "{index:#x}");
            assert_eq!(captured_event(&major), captured, "{index:#x}");
            // A family byte on the record does not change it.
            let family = person(0x3000_0000 | index, true);
            assert_eq!(character_speaker_event(Some(&family)), speaker);
        }
        let minor = person(0x340, false);
        assert_eq!(character_speaker_event(Some(&minor)), 0);
        assert_eq!(captured_event(&minor), 0x15);
        assert_eq!(character_speaker_event(Some(&person(0x244, true))), 0);
        assert_eq!(captured_event(&person(0x244, true)), 0x15);
        assert_eq!(character_speaker_event(None), 0x15);
        let mut world = GameWorld::default();
        let leia = world.characters.insert(person(0x241, true));
        assert_eq!(speaker_event(&world, Some(leia)), 0x18);
        assert_eq!(speaker_event(&world, None), 0x15);
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
