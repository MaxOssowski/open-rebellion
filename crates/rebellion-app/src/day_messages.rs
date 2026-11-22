//! The day's message builders (Character Health, Force Growth, foiled
//! missions, arrivals, escapes, reports, blockades and uprisings), moved
//! out of `main.rs`. The code reads `main.rs`'s items through `super`.

use super::*;

#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
/// Notification `0x106`, Character Injured or Healed (`FUN_0048fdf0`,
/// TEXTSTRA `0x7160`..`0x7163`): Injured while the injury `+0x94` is not 0,
/// Healed when it is.
///
/// hyp: the builder runs on every change of the injury; the port posts it
/// when an injury is added and when the healing ends, not on each day of
/// healing.
pub(super) fn push_character_health(
    log: &mut MessageLog,
    world: &GameWorld,
    character: rebellion_core::ids::CharacterKey,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) {
    use rebellion_render::story_messages::CharacterHealth;
    let Some(c) = world.characters.get(character) else {
        return;
    };
    // FUN_0048fdf0: a destroyed character (+0x50 bit 3) reads Killed over
    // 0x42e or 0x433 by its side; else Injured or Healed.
    let (health, picture, fallback) = if c.is_killed {
        (
            CharacterHealth::Killed,
            pictures::killed(pictures::character_side(world, character)),
            format!("{} has been killed.", c.name),
        )
    } else if c.injury != 0 {
        (
            CharacterHealth::Injured,
            pictures::injured(),
            format!("{} Injured: {} has been injured.", c.name, c.name),
        )
    } else {
        (
            CharacterHealth::Healed,
            pictures::healed((c.dat_id, c.is_major)),
            format!(
                "{} Healed: {} has been healed of all injuries.",
                c.name, c.name
            ),
        )
    };
    let (title, text) =
        match rebellion_render::story_messages::character_health_texts(&c.name, health, texts) {
            Some((title, body)) => (Some(title), body),
            None => (None, fallback),
        };
    let mut message = GameMessage::new(tick, text, MessageCategory::Event)
        .with_picture_if(Some(picture))
        .with_sounds(MessageSounds::character_health(
            health,
            c.dat_id.index(),
            c.dagobah_completed,
        ))
        // FUN_0048fdf0's speaker map on the character.
        .with_advisor_event(rebellion_render::story_messages::speaker_event(
            world,
            Some(character),
        ));
    if let Some(title) = title {
        message = message.with_title(title);
    }
    log.push(filed(
        message,
        MessageRail::Mission,
        health_audience(world, character),
    ));
}

/// The readers of Character Health: `FUN_0048fdf0` builds it for the
/// reader's side copy of the character (`FUN_004f2d10` on `+0x20`) and
/// drops it without one, so the other side reads it too while it holds a
/// copy (A0 2026-11-18-edited-saves-9 c-01, c-04: the Empire reads "Han
/// Solo Injured").
pub(super) fn health_audience(world: &GameWorld, character: CharacterKey) -> Option<RailAudience> {
    let own = character_audience(world, character)?;
    let c = world.characters.get(character)?;
    let other = if c.is_alliance {
        Faction::Empire
    } else {
        Faction::Alliance
    };
    Some(if world.object_views.character(other, c.dat_id).is_some() {
        RailAudience::Both
    } else {
        own
    })
}

/// A Force change one advance effect makes: the effect's index, the
/// character, its Force (`+0x8c`) before and after.
pub(super) type AdvanceForceChange = (usize, CharacterKey, u32, u32);

/// The Force change of each `ForceExperience` gain in a day's advance, read
/// before [`rebellion_core::missions::apply_advance_effects`] stores them.
/// Each write goes through `FUN_004ee150`: the evade (`FUN_0056f8b0`,
/// `FUN_0056ffa0`), the encounter's lessons (`FUN_00560200`) and Dagobah's
/// end (`FUN_005751d0`). Its hook, slot `+0x320` (`FUN_004f1700` in every
/// character vtable), recomputes the Force at once (`FUN_004eee40` ->
/// `FUN_004ee0d0`), which posts `0x1e1` with that gain's old and new Force.
pub(super) fn advance_force_changes(world: &GameWorld, effects: &[MissionEffect]) -> Vec<AdvanceForceChange> {
    let mut running: std::collections::HashMap<CharacterKey, rebellion_core::world::Character> =
        std::collections::HashMap::new();
    let mut changes = Vec::new();
    for (index, effect) in effects.iter().enumerate() {
        let (character, amount) = match effect {
            MissionEffect::ForceExperienceGained { character, amount } => (*character, *amount),
            MissionEffect::DagobahCompleted {
                character,
                force_gain,
            } => (*character, *force_gain),
            _ => continue,
        };
        let Some(current) = running
            .get(&character)
            .or_else(|| world.characters.get(character))
            .cloned()
        else {
            continue;
        };
        let mut after = current.clone();
        after.add_force_experience(amount);
        changes.push((index, character, current.force_level(), after.force_level()));
        running.insert(character, after);
    }
    changes
}

/// The messages of a day's advance, in effect order, after
/// [`rebellion_core::missions::apply_advance_effects`] stored it: the
/// Character Injured message (`0x106`) for each injury the mission
/// detection dealt, and Force Growth (`0x1e1`, `FUN_0048ed80`) for each
/// Force change [`advance_force_changes`] read.
pub(super) fn push_advance_messages(
    log: &mut MessageLog,
    world: &GameWorld,
    effects: &[MissionEffect],
    force_changes: &[AdvanceForceChange],
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) {
    for (index, effect) in effects.iter().enumerate() {
        if let MissionEffect::CharacterInjured { character, injury } = effect {
            if *injury > 0 {
                push_character_health(log, world, *character, tick, texts);
            }
        }
        for &(_, character, old, new) in force_changes.iter().filter(|change| change.0 == index) {
            if let Some(message) = rebellion_render::story_messages::force_growth_message(
                world, character, old, new, tick, texts,
            ) {
                log.push(message);
            }
        }
    }
}

/// Notification `0x100`, System Joins / Joins Enemy / Declares Neutrality
/// (`FUN_00499760`): `system` changing from `previous` to `holder`, on the
/// viewer's Popular Support rail.
pub(crate) fn system_joins_message(
    world: &GameWorld,
    system: rebellion_core::ids::SystemKey,
    previous: Option<Faction>,
    holder: Option<Faction>,
    viewer_is_alliance: bool,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) -> Option<rebellion_render::message_log::GameMessage> {
    let sys = world.systems.get(system)?;
    let (title, text) = rebellion_render::story_messages::control_change_texts(
        &sys.name,
        previous,
        holder,
        viewer_is_alliance,
        texts,
    )?;
    Some(filed(
        GameMessage::at_system(tick, text, MessageCategory::Diplomacy, system)
            .with_title(title)
            .with_picture_if(Some(pictures::loyalty_change()))
            // FUN_00499760: a side gains it, or it goes neutral.
            .with_sounds(MessageSounds::control_change(matches!(
                holder,
                Some(Faction::Alliance | Faction::Empire)
            )))
            .with_advisor_events(
                joins_event(holder, Faction::Alliance),
                joins_event(holder, Faction::Empire),
            ),
        MessageRail::PopularSupport,
        Some(RailAudience::side(viewer_is_alliance)),
    ))
}

/// Jabba Delivers Prisoners (`FUN_0048b6e0`) as the Pickup posts it: the
/// background and sounds follow Luke's (`0x32000242`) DagobahCompleted.
pub(super) fn jabba_delivers_message(
    world: &GameWorld,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) -> Option<GameMessage> {
    rebellion_render::story_messages::jabba_delivers_message(
        world
            .characters
            .values()
            .any(|c| c.dat_id.index() == 0x242 && c.dagobah_completed),
        tick,
        texts,
    )
}

/// Notification `0x1f1` (`FUN_00491500`): the opponent of a foiled
/// mission reads Enemy Mission Foiled on its Mission rail, and its own
/// side Mission Foiled from its leader for codes 3..5.
pub(super) fn push_foiled_missions(
    log: &mut MessageLog,
    world: &GameWorld,
    ended: &[rebellion_core::missions::MissionEnded],
    texts: &std::collections::HashMap<u16, String>,
) {
    for ended in ended {
        if !rebellion_render::story_messages::foils_the_mission(ended.end_code) {
            continue;
        }
        let Some(target) = world.systems.get(ended.target_system) else {
            continue;
        };
        let mission = rebellion_render::mission_dialog::kind_name(ended.kind);
        let own_is_alliance = ended.faction == MissionFaction::Alliance;
        // Codes 3..5: the mission's own side reads Mission Foiled from its
        // leader; with none it reads nothing. hyp: a killed leader no
        // longer resolves (FUN_004f2d10 on a deleted object).
        let leader = ended
            .character
            .and_then(|character| world.characters.get(character))
            .filter(|leader| !leader.is_killed);
        if let (true, Some(leader)) = (matches!(ended.end_code, 3..=5), leader) {
            let returning = ended
                .returning_to
                .and_then(|place| return_place_name(world, place));
            if let Some((title, text)) = rebellion_render::story_messages::mission_foiled_texts(
                (&leader.name, leader.is_major),
                mission,
                &target.name,
                returning.as_deref(),
                texts,
            ) {
                log.push(filed(
                    GameMessage::at_system(
                        ended.tick,
                        text,
                        MessageCategory::Mission,
                        ended.target_system,
                    )
                    .with_title(title)
                    .with_picture(pictures::mission_foiled(
                        pictures::reader(own_is_alliance),
                        ended
                            .character
                            .and_then(|character| pictures::portrait_key(world, character)),
                    ))
                    .with_sounds(MessageSounds::mission_foiled(
                        leader.dat_id.index(),
                        leader.dagobah_completed,
                    ))
                    .with_advisor_event(rebellion_render::story_messages::speaker_event(
                        world,
                        ended.character,
                    )),
                    MessageRail::Mission,
                    Some(RailAudience::side(own_is_alliance)),
                ));
            }
        }
        // Codes 6..10 and 0xc: the target not found, the system changed
        // sides, no research facilities (FUN_00491500), over the side's
        // background with the leader's portrait.
        if let (true, Some(leader)) = (matches!(ended.end_code, 6..=10 | 0xc), leader) {
            let returning = ended
                .returning_to
                .and_then(|place| return_place_name(world, place));
            // Code 8: joined a side (system +0x24 bits 6..7), else neutral.
            let joined = match target.control.faction() {
                Some(Faction::Alliance) => Some(true),
                Some(Faction::Empire) => Some(false),
                Some(Faction::Neutral) | None => None,
            };
            if let Some((title, text)) = rebellion_render::story_messages::mission_failed_texts(
                ended.end_code,
                ended.kind.research_tech().is_some(),
                (&leader.name, leader.is_major),
                mission,
                &target.name,
                joined,
                returning.as_deref(),
                texts,
            ) {
                log.push(filed(
                    GameMessage::at_system(
                        ended.tick,
                        text,
                        MessageCategory::Mission,
                        ended.target_system,
                    )
                    .with_title(title)
                    .with_picture(pictures::side(
                        pictures::reader(own_is_alliance),
                        ended
                            .character
                            .and_then(|character| pictures::portrait_key(world, character)),
                    ))
                    .with_sounds(MessageSounds::mission_failed(
                        own_is_alliance,
                        leader.dat_id.index(),
                        leader.dagobah_completed,
                    ))
                    .with_advisor_event(rebellion_render::story_messages::speaker_event(
                        world,
                        ended.character,
                    )),
                    MessageRail::Mission,
                    Some(RailAudience::side(own_is_alliance)),
                ));
            }
        }
        let Some((title, text)) =
            rebellion_render::story_messages::enemy_mission_foiled_texts(mission, &target.name, texts)
        else {
            continue;
        };
        let opponent_is_alliance = !own_is_alliance;
        log.push(filed(
            GameMessage::at_system(
                ended.tick,
                text,
                MessageCategory::Mission,
                ended.target_system,
            )
            .with_title(title)
            .with_picture(pictures::enemy_mission_foiled(pictures::reader(
                opponent_is_alliance,
            )))
            .with_sounds(MessageSounds::enemy_mission_foiled(opponent_is_alliance)),
            MessageRail::Mission,
            Some(RailAudience::side(opponent_is_alliance)),
        ));
    }
}

/// Notification `0x200` (`FUN_004955b0`): the side of a Move, Return or
/// Adrift reads Personnel Arrive on its Mission rail, over its own
/// background (`0x412` / `0x413`) with the leader's portrait; a major
/// leader speaks (the speaker map), anyone else is `0x15`.
pub(super) fn push_personnel_arrivals(
    log: &mut MessageLog,
    world: &GameWorld,
    arrivals: &[rebellion_core::missions::PersonnelArrival],
    texts: &std::collections::HashMap<u16, String>,
) {
    use rebellion_core::missions::MissionMember;
    for arrival in arrivals {
        let Some(system) = world.systems.get(arrival.system) else {
            continue;
        };
        let named: Vec<(MissionMember, String, bool)> = arrival
            .members
            .iter()
            .filter_map(|&member| match member {
                MissionMember::Character(key) => world
                    .characters
                    .get(key)
                    .map(|c| (member, c.name.clone(), c.is_major)),
                MissionMember::SpecialForce(key) => world
                    .special_forces
                    .get(key)
                    .and_then(|unit| world.buildable_classes.get(&unit.class_dat_id))
                    .map(|class| (member, class.name.clone(), false)),
            })
            .collect();
        let names: Vec<(&str, bool)> = named
            .iter()
            .map(|(_, name, major)| (name.as_str(), *major))
            .collect();
        let Some((title, text)) =
            rebellion_render::story_messages::personnel_arrive_texts(&names, &system.name, texts)
        else {
            continue;
        };
        let leader = match named.first() {
            Some((MissionMember::Character(key), _, _)) => Some(*key),
            _ => None,
        };
        let major = named.first().is_some_and(|(_, _, major)| *major);
        let is_alliance = arrival.faction == MissionFaction::Alliance;
        log.push(filed(
            GameMessage::at_system(arrival.tick, text, MessageCategory::Mission, arrival.system)
                .with_title(title)
                .with_picture(pictures::side(
                    pictures::reader(is_alliance),
                    leader.and_then(|key| pictures::portrait_key(world, key)),
                ))
                .with_sounds(
                    match leader.and_then(|key| world.characters.get(key)) {
                        Some(c) => MessageSounds::personnel_arrive(
                            is_alliance,
                            c.dat_id.index(),
                            c.dagobah_completed,
                        ),
                        None => MessageSounds::personnel_arrive(is_alliance, 0, false),
                    },
                )
                .with_advisor_event(if major {
                    rebellion_render::story_messages::speaker_event(world, leader)
                } else {
                    0x15
                }),
            MessageRail::Mission,
            Some(RailAudience::side(is_alliance)),
        ));
    }
}

/// A captive escapes to its side: it is freed, and notification `0x1e0`
/// with the escape flag posts Character Escaped (`FUN_00490340` cases 1
/// and 2) to each side, naming the system it held the captive at.
pub(super) fn escape_character(
    world: &mut GameWorld,
    character: CharacterKey,
    escaped_to_alliance: bool,
    tick: u64,
    log: &mut MessageLog,
    texts: &std::collections::HashMap<u16, String>,
) {
    let Some(captive) = world.characters.get(character) else {
        return;
    };
    // The captor: the side that held it, else the other side.
    let captor_is_alliance = captive
        .captured_by
        .map_or(!escaped_to_alliance, |side| side == Faction::Alliance);
    let at = captive
        .current_fleet
        .and_then(|fleet| world.fleets.get(fleet))
        .map(|fleet| fleet.location)
        .or(captive.current_system);
    let place = at
        .and_then(|system| world.systems.get(system))
        .map_or_else(String::new, |system| system.name.clone());
    let before = captive.clone();
    escape_messages_of(&before, captor_is_alliance, at, &place, tick, log, texts);
    if let Some(c) = world.characters.get_mut(character) {
        c.is_alliance = escaped_to_alliance;
        c.is_empire = !escaped_to_alliance;
        c.is_captive = false;
        c.captured_by = None;
        c.capture_tick = None;
    }
}

/// The messages of a captive's escape, from the world before it.
pub(super) fn escape_messages(
    world: &GameWorld,
    character: CharacterKey,
    escaped_to_alliance: bool,
    tick: u64,
    log: &mut MessageLog,
    texts: &std::collections::HashMap<u16, String>,
) {
    let Some(captive) = world.characters.get(character) else {
        return;
    };
    let captor_is_alliance = captive
        .captured_by
        .map_or(!escaped_to_alliance, |side| side == Faction::Alliance);
    let at = captive
        .current_fleet
        .and_then(|fleet| world.fleets.get(fleet))
        .map(|fleet| fleet.location)
        .or(captive.current_system);
    let place = at
        .and_then(|system| world.systems.get(system))
        .map_or_else(String::new, |system| system.name.clone());
    escape_messages_of(captive, captor_is_alliance, at, &place, tick, log, texts);
}

/// Notification 0x1e0's escape messages (FUN_00490340), one for each side.
pub(super) fn escape_messages_of(
    before: &rebellion_core::world::Character,
    captor_is_alliance: bool,
    at: Option<SystemKey>,
    place: &str,
    tick: u64,
    log: &mut MessageLog,
    texts: &std::collections::HashMap<u16, String>,
) {
    for reader in [CockpitFaction::Alliance, CockpitFaction::Empire] {
        if let Some(mut message) = rebellion_render::story_messages::escaped_message(
            before,
            captor_is_alliance,
            place,
            reader,
            tick,
            texts,
        ) {
            // FUN_00490340's messages have a target, as Character Captured
            // shows Display lit (A0 messages-day633/capture-palpatine).
            // port: the system the captive was held at.
            message.system = at;
            log.push(message);
        }
    }
}

/// Unit Arrival case 4 (`FUN_004981c0`): the Headquarters reaching its
/// new system, to the Alliance's Defense rail (category `0x40`) over
/// `0x3fe`, advisor event 6.
pub(super) fn headquarters_arrival_message(
    world: &GameWorld,
    transit: &rebellion_core::headquarters_move::HeadquartersTransit,
    texts: &std::collections::HashMap<u16, String>,
) -> Option<GameMessage> {
    let system = world.systems.get(transit.destination)?;
    let (title, text) =
        rebellion_render::story_messages::headquarters_arrival_texts(&system.name, texts)?;
    Some(filed(
        GameMessage::at_system(
            transit.arrival_tick,
            text,
            MessageCategory::Event,
            transit.destination,
        )
        .with_title(title)
        .with_picture_if(pictures::arrival(
            pictures::Arrival::Headquarters,
            CockpitFaction::Alliance,
        ))
        .with_sounds(MessageSounds::arrival(pictures::Arrival::Headquarters, true))
        .with_advisor_event(6),
        MessageRail::Defense,
        Some(RailAudience::Alliance),
    ))
}

/// Notification `0x362` for each Force discovery (timer `0x389`), built as
/// `FUN_0048f1e0` (`story_messages::force_user_discovered`).
pub(super) fn push_force_discoveries(
    log: &mut MessageLog,
    world: &GameWorld,
    discovered: &[rebellion_core::force_discovery::ForceDiscovered],
    texts: &std::collections::HashMap<u16, String>,
) {
    for found in discovered {
        if let Some(message) = rebellion_render::story_messages::force_user_discovered(
            world,
            found.discoverer,
            found.character,
            found.tick,
            texts,
        ) {
            log.push(message);
        }
    }
}

/// The name `0x70e6` gives a return place: a system's, or a fleet's label.
pub(super) fn return_place_name(
    world: &GameWorld,
    place: rebellion_core::missions::ReturnPlace,
) -> Option<String> {
    match place {
        rebellion_core::missions::ReturnPlace::System(system) => {
            world.systems.get(system).map(|system| system.name.clone())
        }
        rebellion_core::missions::ReturnPlace::Fleet(fleet) => {
            rebellion_render::system_window::fleet_label(world, fleet)
        }
    }
}

/// The names a Mission Report gives (`FUN_004927c0`): its leader (`+0x74`)
/// with its major flag, target object or character (`+0x70`) and recruit
/// (`+0xa8`), read
/// before the effects, and whether the effects kill the target.
pub(super) fn report_names(
    world: &GameWorld,
    result: &rebellion_core::missions::MissionResult,
) -> (Option<(String, bool)>, Option<String>, Option<String>, bool) {
    let name_of = |character: Option<CharacterKey>| {
        character
            .and_then(|character| world.characters.get(character))
            .map(|character| (character.name.clone(), character.is_major))
    };
    let recruited = result.effects.iter().find_map(|effect| match effect {
        MissionEffect::CharacterRecruited { character, .. } => Some(*character),
        _ => None,
    });
    let target_killed = result.effects.iter().any(|effect| {
        matches!(effect, MissionEffect::CharacterKilled { character, .. }
            if Some(*character) == result.target_character)
    });
    // FUN_004f2d10 resolves the leader +0x74 whatever its family: a team
    // of special forces alone is led by its first.
    let leader = name_of(result.character).or_else(|| {
        result
            .lead_force
            .and_then(|force| {
                rebellion_render::status_rows::object_name(
                    world,
                    rebellion_core::missions::MissionTarget::SpecialForce(force),
                )
            })
            .map(|name| (name, false))
    });
    (
        leader,
        result
            .target_object
            .and_then(|object| rebellion_render::status_rows::object_name(world, object))
            .or_else(|| name_of(result.target_character).map(|(name, _)| name)),
        name_of(recruited).map(|(name, _)| name),
        target_killed,
    )
}

/// A Mission Report's title and text: the shipped report when there is
/// one, else the port's `fallback` line, then the continue `question`.
/// The shipped body runs straight on into the question (`FUN_004927c0`
/// appends `0x70db` to the same text); the fallback takes a space.
pub(super) fn report_title_and_text(
    shipped: Option<(String, String)>,
    fallback: String,
    question: Option<&str>,
) -> (Option<String>, String) {
    let (title, mut text) = match shipped {
        Some((title, body)) => (Some(title), body),
        None => (None, fallback),
    };
    if let Some(question) = question {
        if title.is_none() {
            text.push(' ');
        }
        text.push_str(question);
    }
    (title, text)
}

/// Notification `0x153`, Informants Provide Information (`FUN_00491060`):
/// TEXTSTRA `0x7150` / `0x7151` naming the system, to the side the
/// informants told, on its Popular Support rail, picture `0x3e8`, advisor
/// event `0x1c`. A0 informants-clean/test-day1457-selonia (an unedited
/// Alliance game) shows it with Display lit.
pub(super) fn informants_message(
    world: &GameWorld,
    system: rebellion_core::ids::SystemKey,
    recipient_is_alliance: bool,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) -> Option<GameMessage> {
    let name = world.systems.get(system).map_or("unknown", |s| s.name.as_str());
    let (title, text) = rebellion_render::story_messages::informants_texts(name, texts)?;
    Some(filed(
        GameMessage::at_system(tick, text, MessageCategory::Event, system)
            .with_title(title)
            .with_picture_if(Some(pictures::informants()))
            .with_sounds(MessageSounds::informants())
            // FUN_00491060 writes 0x1c.
            .with_advisor_event(0x1c),
        MessageRail::PopularSupport,
        Some(RailAudience::side(recipient_is_alliance)),
    ))
}

/// Notification `0x304`, Maintenance Shortfall (`FUN_00498970`), for an
/// autoscrapped unit: TEXTSTRA `0x7080`..`0x7082` naming the system its
/// containers stand at, to the unit's side on the Manufacturing rail,
/// picture `0x3f5` / `0x3f6` by the reader, advisor event `0xc`. A unit at
/// no system (a fleet in hyperspace, a regiment travelling) leaves the
/// system's name empty, and the message still has its target: A0
/// maintenance-shortfall/test-day1224-corvette-no-system reads "destroyed
/// at  due to" with Display lit.
pub(super) fn maintenance_message(
    world: &GameWorld,
    system: Option<rebellion_core::ids::SystemKey>,
    unit: &str,
    alliance: bool,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) -> Option<GameMessage> {
    let name = system.and_then(|system| world.systems.get(system)).map(|s| s.name.as_str());
    let (title, text) = rebellion_render::story_messages::maintenance_texts(name, unit, texts)?;
    let message = match system {
        Some(system) => GameMessage::at_system(tick, text, MessageCategory::Event, system),
        None => GameMessage::new(tick, text, MessageCategory::Event).with_unplaced_target(),
    };
    Some(filed(
        message
            .with_title(title)
            .with_picture_if(Some(pictures::units_destroyed(pictures::reader(alliance))))
            .with_sounds(MessageSounds::units_destroyed())
            // FUN_00498970 writes 0xc.
            .with_advisor_event(0xc),
        MessageRail::Manufacturing,
        Some(RailAudience::side(alliance)),
    ))
}

/// Notification `0x14e`, Blockade (`FUN_004960f0`), as a blockade starts:
/// the holder and the blockader each read their own text, and it needs a
/// fleet at the system (`FUN_004ffdd0`). The class is silent while the
/// system's `+0x88` bit `0x4000` is set, which `FUN_0050a9e0` sets when the
/// blockade bit changes in the pass the battle bit does: a blockade left by
/// a battle (`after_battle`). A0's unedited Alliance game listed no
/// Blockade after the battles at Yavin (day 650) and Umgul (day 718) left
/// those systems blockaded. port: without the texts, the port's line.
/// Notifications `0x125`..`0x127`, Research Complete (`FUN_00499aa0`,
/// class `0x0065bb40`): the class `level` opens on `tech`'s tree, filed on
/// the Manufacturing rail for the side. Kind 4: Display opens the class's
/// Encyclopedia entry (`FUN_00467f10` -> `FUN_0041d6b0`; A0
/// research-complete/02 opens the Nebulon-B Frigate's).
pub(super) fn research_complete_message(
    world: &GameWorld,
    is_alliance: bool,
    tech: rebellion_core::research::TechType,
    level: u32,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) -> Option<GameMessage> {
    let (title, text) =
        match rebellion_render::story_messages::research_complete_texts(world, is_alliance, tech, level, texts) {
            Some((title, body)) => (Some(title), body),
            None if texts.is_empty() => {
                let faction_name = if is_alliance { "Alliance" } else { "Empire" };
                let tree_name = match tech {
                    rebellion_core::research::TechType::Ship => "Ship",
                    rebellion_core::research::TechType::Troop => "Troop",
                    rebellion_core::research::TechType::Facility => "Facility",
                };
                (None, format!("{faction_name} {tree_name} tech advanced to level {level}"))
            }
            None => return None,
        };
    // FUN_00499aa0 writes code 4.
    let mut message = GameMessage::new(tick, text, MessageCategory::Event)
        .with_picture_if(Some(pictures::research()))
        .with_sounds(MessageSounds::research())
        .with_advisor_event(4);
    if let Some(object) =
        rebellion_render::story_messages::research_complete_encyclopedia_object(world, is_alliance, tech, level)
    {
        message = message.with_encyclopedia(object);
    }
    if let Some(title) = title {
        message = message.with_title(title);
    }
    Some(filed(message, MessageRail::Manufacturing, Some(RailAudience::side(is_alliance))))
}

/// Notification `0xd`, Unit Arrival (`FUN_004981c0`, case 1): TEXTSTRA
/// `0x7078` / `0x7079` name the fleet and the system, filed on the Fleet
/// rail for the fleet's side.
pub(super) fn fleet_arrival_message(
    world: &GameWorld,
    fleet: rebellion_core::ids::FleetKey,
    system: rebellion_core::ids::SystemKey,
    is_alliance: bool,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) -> GameMessage {
    let sys_name = world
        .systems
        .get(system)
        .map_or_else(|| "unknown".into(), |s| s.name.clone());
    let fleet_name = world.fleet_name(fleet).map(str::to_owned).unwrap_or_default();
    let texts = rebellion_render::story_messages::fleet_arrival_texts(&fleet_name, &sys_name, texts);
    let mut message = GameMessage::at_system(
        tick,
        texts
            .as_ref()
            .map_or_else(|| format!("Fleet arrived at {sys_name}"), |(_, body)| body.clone()),
        MessageCategory::Mission,
        system,
    )
    .with_picture_if(pictures::arrival(pictures::Arrival::Fleet, pictures::reader(is_alliance)))
    .with_sounds(MessageSounds::arrival(pictures::Arrival::Fleet, is_alliance))
    // hyp: FUN_004981c0's fleet case (1), code 5.
    .with_advisor_event(5);
    if let Some((title, _)) = texts {
        message = message.with_title(title);
    }
    filed(message, MessageRail::Fleet, Some(RailAudience::side(is_alliance)))
}

pub(super) fn blockade_started_message(
    world: &GameWorld,
    system: rebellion_core::ids::SystemKey,
    after_battle: bool,
    viewer_is_alliance: bool,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) -> Option<GameMessage> {
    let name = world
        .systems
        .get(system)
        .map_or_else(|| "unknown".into(), |s| s.name.clone());
    if after_battle {
        return Some(GameMessage::at_system(
            tick,
            format!("Blockade established at {name}"),
            MessageCategory::Combat,
            system,
        ));
    }
    let holder = world.systems.get(system).and_then(|s| s.control.faction());
    let fleet = world
        .systems
        .get(system)
        .and_then(|s| s.fleets.first())
        .and_then(|&fleet| world.fleet_name(fleet))
        .map(str::to_owned);
    let original = fleet.as_deref().and_then(|fleet| {
        rebellion_render::story_messages::blockade_texts(&name, holder, fleet, viewer_is_alliance, texts)
    });
    let (title, text, audience) = match original {
        Some((title, body)) => (Some(title), body, Some(RailAudience::side(viewer_is_alliance))),
        None if texts.is_empty() => (
            None,
            format!("Blockade established at {name}"),
            Some(RailAudience::Both),
        ),
        None => return None,
    };
    let mut message = GameMessage::at_system(tick, text, MessageCategory::Combat, system)
        .with_picture_if(pictures::blockade(pictures::system_side(world, system)))
        .with_sounds(MessageSounds::blockade())
        // FUN_004960f0: 0xe for the holder, 0xd for the other side.
        .with_advisor_events(
            blockade_event(holder, Faction::Alliance),
            blockade_event(holder, Faction::Empire),
        );
    if let Some(title) = title {
        message = message.with_title(title);
    }
    Some(filed(message, MessageRail::Conflict, audience))
}

/// Notification `0x305`, Saboteurs Strike (`FUN_0048b9a0`), for an object
/// a sabotage destroyed: to the object's side (`+0x40`, `FUN_0048bbf0`),
/// on the Conflict rail (`+0x34` = `0x100`), picture `0x3f5` / `0x3f6` by
/// the reader, advisor event `0xc`. `victim` is the object's side and name,
/// read before it was destroyed; a neutral object posts nothing.
pub(super) fn saboteurs_strike_message(
    world: &GameWorld,
    victim: Option<(Faction, String)>,
    system: rebellion_core::ids::SystemKey,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) -> Option<GameMessage> {
    let (side, name) = victim?;
    let alliance = match side {
        Faction::Alliance => true,
        Faction::Empire => false,
        Faction::Neutral => return None,
    };
    let place = world.systems.get(system).map_or("", |s| s.name.as_str());
    let (title, body) = rebellion_render::story_messages::saboteurs_texts(place, &name, texts)?;
    Some(filed(
        GameMessage::at_system(tick, body, MessageCategory::Combat, system)
            .with_title(title)
            .with_picture_if(Some(pictures::units_destroyed(pictures::reader(alliance))))
            .with_sounds(MessageSounds::units_destroyed())
            .with_advisor_event(0xc),
        MessageRail::Conflict,
        Some(RailAudience::side(alliance)),
    ))
}

/// Whether `fleet`'s ship at `index` is damaged (`+0x50` bit `0x200`,
/// `status_rows::damaged`); None for no such ship.
pub(super) fn ship_damaged(world: &GameWorld, fleet: FleetKey, index: usize) -> Option<bool> {
    let ship = world.fleets.get(fleet)?.capital_ships.get(index)?;
    let class = world.capital_ship_classes.get(ship.class)?;
    Some(rebellion_render::status_rows::damaged(ship, class))
}

/// Notification `0x106` with the damage flag cleared, Capital Ship
/// Repaired (`FUN_00499de0`, class `0x65bb88`): TEXTSTRA `0x7041` /
/// `0x7043` naming the ship and its fleet, to the ship's side on the Fleet
/// rail (`+0x34` = `0x80`), the ship's own picture (`FUN_004c5000`),
/// advisor event 8, sound `0x45e`. Kind 3 (`FUN_005a6730`): Display goes to
/// the fleet's system.
pub(super) fn ship_repaired_message(
    world: &GameWorld,
    fleet: FleetKey,
    index: usize,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) -> Option<GameMessage> {
    let value = world.fleets.get(fleet)?;
    let ship = value.capital_ships.get(index)?;
    let class = world.capital_ship_classes.get(ship.class)?;
    let name = ship.name.clone().unwrap_or_else(|| class.name.clone());
    let container = rebellion_render::system_window::fleet_label(world, fleet).unwrap_or_default();
    let (title, text) = rebellion_render::story_messages::repaired_texts(&name, &container, false, texts)?;
    Some(filed(
        GameMessage::at_system(tick, text, MessageCategory::Combat, value.location)
            .with_title(title)
            .with_picture_if(pictures::deployed(
                world,
                rebellion_core::manufacturing::BuildableKind::CapitalShip(ship.class),
            ))
            .with_sounds(MessageSounds::repaired(false, value.is_alliance))
            .with_advisor_event(8),
        MessageRail::Fleet,
        Some(RailAudience::side(value.is_alliance)),
    ))
}

/// Notification `0x106` with the damage flag cleared for a squadron,
/// Squadron at Full Strength (`FUN_00499de0`, class `0x65bb88`, `+0x28` 9):
/// TEXTSTRA `0x7040` / `0x7042` naming the squadron and its container, to
/// the squadron's side, the squadron's own picture (`FUN_004c5000`), sound
/// `0x457 + (side != 1)`. A squadron aboard a capital ship names the ship's
/// fleet on the Fleet rail (`+0x34` = `0x80`); one on a planet names the
/// system on the Defense rail (`0x40`).
pub(super) fn squadron_full_strength_message(
    world: &GameWorld,
    place: rebellion_core::squadrons::SquadronPlace,
    index: usize,
    tick: u64,
    texts: &std::collections::HashMap<u16, String>,
) -> Option<GameMessage> {
    use rebellion_core::squadrons::SquadronPlace;
    let (entry, system, container, rail) = match place {
        SquadronPlace::Fleet(fleet) => {
            let value = world.fleets.get(fleet)?;
            (
                value.fighters.get(index)?,
                value.location,
                rebellion_render::system_window::fleet_label(world, fleet).unwrap_or_default(),
                MessageRail::Fleet,
            )
        }
        SquadronPlace::Planet(system) => {
            let value = world.systems.get(system)?;
            (
                value.fighters.get(index)?,
                system,
                value.name.clone(),
                MessageRail::Defense,
            )
        }
    };
    let class = world.fighter_classes.get(entry.class)?;
    let alliance = rebellion_core::squadrons::is_alliance(world, entry.class)?;
    let (title, text) =
        rebellion_render::story_messages::repaired_texts(&class.name, &container, true, texts)?;
    Some(filed(
        GameMessage::at_system(tick, text, MessageCategory::Combat, system)
            .with_title(title)
            .with_picture_if(pictures::deployed(
                world,
                rebellion_core::manufacturing::BuildableKind::Fighter(entry.class),
            ))
            .with_sounds(MessageSounds::repaired(true, alliance))
            .with_advisor_event(9),
        rail,
        Some(RailAudience::side(alliance)),
    ))
}

/// A character's Force (`+0x8c`), 0 when it is gone.
pub(super) fn force_level(world: &GameWorld, character: CharacterKey) -> u32 {
    world
        .characters
        .get(character)
        .map_or(0, rebellion_core::world::Character::force_level)
}

/// File Force Growth (notification `0x1e1`, `FUN_0048ed80`) when a change
/// moved the character's Force from `old` (`FUN_004ee0d0` posts it on every
/// change of `+0x8c`).
pub(super) fn push_force_growth(
    log: &mut MessageLog,
    world: &GameWorld,
    character: CharacterKey,
    old: u32,
    tick: u64,
    message_texts: &std::collections::HashMap<u16, String>,
) {
    if let Some(message) = rebellion_render::story_messages::force_growth_message(
        world,
        character,
        old,
        force_level(world, character),
        tick,
        message_texts,
    ) {
        log.push(message);
    }
}

/// FUN_00499760's event code for `reader` when a system joins `new`: 1 when
/// it joins the reader, 2 when it joins the other side, none for neutrality.
pub(super) fn joins_event(new: Option<Faction>, reader: Faction) -> u32 {
    match new {
        Some(Faction::Neutral) | None => 0,
        Some(side) if side == reader => 1,
        Some(_) => 2,
    }
}

/// The rail an uprising-class message is filed on: Uprising Begins / Ends
/// (`FUN_00499460`) on Popular Support; Natural Disaster's class
/// (`FUN_0048e940`, `0x65b490`), which also files New Resources and
/// Resources Depleted, on the Resource rail (A0 2026-11-15-partial-cells/
/// r2-message-index-natural-disaster-row-d824: the row's icon is 0x2ad6).
pub(super) fn uprising_message_rail(evt: &rebellion_core::uprising::UprisingEvent) -> MessageRail {
    match evt {
        rebellion_core::uprising::UprisingEvent::Disaster { .. } => MessageRail::Resource,
        _ => MessageRail::PopularSupport,
    }
}

/// FUN_00499460's event code for `reader`: 1 when (the reader holds the
/// system) differs from (the uprising begins), else 2; none without a
/// holding side.
pub(super) fn uprising_event(holder: Option<Faction>, reader: Faction, begins: bool) -> u32 {
    match holder {
        Some(Faction::Alliance | Faction::Empire) if (holder == Some(reader)) != begins => 1,
        Some(Faction::Alliance | Faction::Empire) => 2,
        _ => 0,
    }
}

/// FUN_004960f0's event code for `reader`: 0xe when its system is
/// blockaded, 0xd when its side's fleet blockades.
pub(super) fn blockade_event(holder: Option<Faction>, reader: Faction) -> u32 {
    if holder == Some(reader) {
        0xe
    } else {
        0xd
    }
}

/// File a report on the Message Index rail when some side receives it.
pub(super) fn filed(message: GameMessage, rail: MessageRail, audience: Option<RailAudience>) -> GameMessage {
    match audience {
        Some(audience) => message.on_rail(rail, audience),
        None => message,
    }
}

/// The side that controls `system`, whose Message Index receives its reports.
pub(super) fn system_audience(world: &GameWorld, system: SystemKey) -> Option<RailAudience> {
    match world.systems.get(system)?.control.faction()? {
        Faction::Alliance => Some(RailAudience::Alliance),
        Faction::Empire => Some(RailAudience::Empire),
        Faction::Neutral => None,
    }
}

/// The side a character serves, whose Message Index receives its reports.
pub(super) fn character_audience(world: &GameWorld, character: CharacterKey) -> Option<RailAudience> {
    let character = world.characters.get(character)?;
    if character.is_alliance {
        Some(RailAudience::Alliance)
    } else if character.is_empire {
        Some(RailAudience::Empire)
    } else {
        None
    }
}

/// The messages of blockade events not yet applied: a regiment lost
/// running a blockade is named before it goes.
pub(super) fn blockade_messages(
    world: &GameWorld,
    msg_log: &mut MessageLog,
    events: &[rebellion_core::blockade::BlockadeEvent],
    battled: &std::collections::HashSet<rebellion_core::ids::SystemKey>,
    player_is_alliance: bool,
    message_texts: &std::collections::HashMap<u16, String>,
    cockpit_faction: CockpitFaction,
) {
    for evt in events {
        match evt {
            rebellion_core::blockade::BlockadeEvent::BlockadeStarted { system, tick } => {
                if let Some(message) = blockade_started_message(
                    world,
                    *system,
                    battled.contains(system),
                    player_is_alliance,
                    *tick,
                    message_texts,
                ) {
                    msg_log.push(message);
                }
            }
            rebellion_core::blockade::BlockadeEvent::BlockadeEnded { system, tick } => {
                let name = world
                    .systems
                    .get(*system)
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                // Notification 0x14e with the bit cleared: the
                // Blockade class (FUN_004960f0) posts nothing. port:
                // the line stays in the log, off the Message Index.
                msg_log.push(GameMessage::at_system(
                    *tick,
                    format!("Blockade lifted at {name}"),
                    MessageCategory::Combat,
                    *system,
                ));
            }
            rebellion_core::blockade::BlockadeEvent::SupportShifted { .. } => {}
            rebellion_core::blockade::BlockadeEvent::TroopDestroyed {
                system,
                troop,
                tick,
            } => {
                // Event 0x340, Evacuation Losses (FUN_00495e90), to
                // the regiment's side; named before it goes.
                let regiment = world.troops.get(*troop).map(|unit| {
                    let (name, _) = rebellion_render::manufacturing_window::product(
                        world,
                        rebellion_core::manufacturing::BuildableKind::Troop(unit.class_dat_id),
                    );
                    (name, unit.is_alliance)
                });
                let name = world
                    .systems
                    .get(*system)
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                let original = regiment.as_ref().and_then(|(regiment, _)| {
                    rebellion_render::story_messages::evacuation_losses_texts(
                        regiment,
                        &name,
                        message_texts,
                    )
                });
                let audience = regiment
                    .as_ref()
                    .map_or(RailAudience::Both, |&(_, alliance)| {
                        RailAudience::side(alliance)
                    });
                let (title, text) = original.map_or_else(
                    || (None, format!("Troops destroyed by blockade at {name}")),
                    |(title, body)| (Some(title), body),
                );
                let mut message =
                    GameMessage::at_system(*tick, text, MessageCategory::Combat, *system)
                        .with_picture_if(Some(pictures::evacuation_losses(cockpit_faction)))
                        .with_sounds(MessageSounds::evacuation_losses(
                            cockpit_faction == CockpitFaction::Alliance,
                        ))
                        // FUN_00495e90 writes 0x10, which no side's
                        // advisor answers.
                        .with_advisor_event(0x10);
                if let Some(title) = title {
                    message = message.with_title(title);
                }
                msg_log.push(filed(message, MessageRail::Conflict, Some(audience)));
            }
        }
    }
}
